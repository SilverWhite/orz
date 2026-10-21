//! ACP stdio server — agent side over the persistent stdio JSON-RPC stream.
//!
//! Inbound: `AgentSideConnection` frames client requests (initialize /
//! new_session / prompt / cancel) over newline-delimited JSON-RPC and
//! dispatches them to our `MessageHandler` (→ `AcpServer`).
//!
//! Outbound: `acp_gateway::<AgentSide, _>(conn)` produces the
//! `AcpAgentGatewaySender` — exactly the type orz-workspace's permission
//! manager requires as its `GatewaySender` (manager.rs alias). It is parked
//! on the AcpServer for the permission bridge (Phase 2, IP6).
//!
//! Runs inside a `tokio::task::LocalSet` (the connection requires
//! `spawn_local`).

use std::sync::Arc;

use agent_client_protocol as acp;
use agent_client_protocol::{
    AgentCapabilities, AgentResponse, ClientNotification, ClientRequest, ContentBlock,
    InitializeResponse, NewSessionResponse, PromptResponse, SessionId, StopReason,
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use xai_acp_lib::acp_gateway;

use crate::acp_server::AcpServer;

/// Agent-side handler: dispatches inbound client requests to the AcpServer.
pub struct StdioAgentHandler {
    server: Arc<AcpServer>,
    trust_policy: crate::session::TrustPolicy,
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1) + 2026-10-17 用户令:
    /// session-level 检索启用门——三值模式退役后唯一授权状态；**缺省开**
    /// （用户令「正常版本的检索门默认开即可，得常开」），显式关仍可（经
    /// [`crate::retrieval_gate`] 解析）。`false` = 该会话无检索工具。
    retrieval_enabled: bool,
}

impl StdioAgentHandler {
    /// Default handler with the strict `Enforce` trust policy.
    pub fn new(server: Arc<AcpServer>) -> Self {
        Self::with_trust_policy(server, crate::session::TrustPolicy::Enforce)
    }

    /// Handler with an explicit trust policy — the TUI's in-process wiring
    /// and integration tests run against temp workspaces via `Skip`.
    ///
    /// 2026-10-17 用户令：检索启用门缺省开——本构造不带显式门值，走单源解析
    /// [`crate::retrieval_gate::retrieval_enabled_from_env`]（此前硬编码
    /// `false`，使 TUI 进程内面在缺省下无检索工具）。
    pub fn with_trust_policy(
        server: Arc<AcpServer>,
        trust_policy: crate::session::TrustPolicy,
    ) -> Self {
        Self {
            server,
            trust_policy,
            retrieval_enabled: crate::retrieval_gate::retrieval_enabled_from_env(),
        }
    }

    /// 0t (2026-09-09, ADR-0010 §14.65): fix the session-level retrieval
    /// enable gate for sessions created through this handler (the TUI/stdio
    /// surfaces pass their resolved enable flag here; 旧 `--retrieval-mode`
    /// 已在调用方兼容解析为 bool)。
    pub fn with_retrieval_enabled(
        server: Arc<AcpServer>,
        trust_policy: crate::session::TrustPolicy,
        retrieval_enabled: bool,
    ) -> Self {
        Self {
            server,
            trust_policy,
            retrieval_enabled,
        }
    }
}

/// Extract plain text from a prompt's content blocks (baseline: Text blocks).
fn extract_prompt_text(blocks: &[ContentBlock]) -> String {
    blocks
        .iter()
        .filter_map(|b| match b {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 0di: wrap a JSON value as an ACP ext-method response (`ExtResponse` is a
/// transparent `Arc<RawValue>`; `to_raw_value` yields the boxed form).
fn ext_response_value(v: serde_json::Value) -> AgentResponse {
    let raw: std::sync::Arc<serde_json::value::RawValue> = std::sync::Arc::from(
        serde_json::value::to_raw_value(&v).expect("ext response serializable"),
    );
    AgentResponse::ExtMethodResponse(acp::ExtResponse::new(raw))
}

/// 0dk S2（332）：`_orz/*` 参数兼容 camelCase（设计线格式）与既有
/// snake_case（0di 面），两者都收。
fn ext_session_id(params: &serde_json::Value) -> String {
    params
        .get("sessionId")
        .or_else(|| params.get("session_id"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// 0dk S2（332）：会话生命周期错误 → JSON-RPC 错误（-32010..-32014 +
/// `data` 四要素）。传输适配器只做这一层映射，状态机在 `session_ops`。
fn session_error_to_acp(err: crate::session_ops::SessionError) -> acp::Error {
    acp::Error::new(err.code(), err.to_string()).data(err.data())
}

impl acp::MessageHandler<acp::AgentSide> for StdioAgentHandler {
    async fn handle_request(&self, request: ClientRequest) -> acp::Result<AgentResponse> {
        match request {
            ClientRequest::InitializeRequest(args) => {
                let response = InitializeResponse::new(args.protocol_version)
                    .agent_capabilities(AgentCapabilities::default().load_session(true));
                Ok(AgentResponse::InitializeResponse(response))
            }
            ClientRequest::NewSessionRequest(args) => {
                // The client asks for a new session; the agent generates the ID.
                let session_id = uuid::Uuid::new_v4().to_string();
                self.server
                    // 329（2026-10-21）：交互式 session/new 不做 0cy 收卷——
                    // 用户切换/新建会话不得收卷其它会话足迹（328 §6.5）。
                    .handle_session_new_with_options_no_sweep(
                        &session_id,
                        Some(args.cwd),
                        self.trust_policy,
                        Default::default(),
                        self.retrieval_enabled,
                    )
                    .await
                    .map_err(acp::Error::into_internal_error)?;
                Ok(AgentResponse::NewSessionResponse(NewSessionResponse::new(
                    SessionId::new(session_id),
                )))
            }
            ClientRequest::LoadSessionRequest(args) => {
                let session_id = args.session_id.0.as_ref().to_string();
                self.server
                    .handle_session_load_with_options(
                        &session_id,
                        args.cwd,
                        self.trust_policy,
                        Default::default(),
                        self.retrieval_enabled,
                    )
                    .await
                    .map_err(|e| match e {
                        // Busy/validation are normal protocol conditions, not
                        // internal faults (318 P3): keep the JSON-RPC code
                        // meaningful so clients can retry instead of treating
                        // it as an agent crash.
                        crate::acp_server::AcpError::InvalidRequest(message) => {
                            acp::Error::new(acp::ErrorCode::InvalidRequest.into(), message)
                        }
                        crate::acp_server::AcpError::SessionNotFound(message) => acp::Error::new(
                            acp::ErrorCode::InvalidRequest.into(),
                            format!("session not found: {message}"),
                        ),
                        other => acp::Error::into_internal_error(other),
                    })?;
                Ok(AgentResponse::LoadSessionResponse(
                    acp::LoadSessionResponse::new(),
                ))
            }
            ClientRequest::PromptRequest(args) => {
                let session_id = args.session_id.0.as_ref().to_string();
                let prompt = extract_prompt_text(&args.prompt);
                match self
                    .server
                    .handle_session_prompt(&session_id, &prompt)
                    .await
                {
                    Ok(_) => Ok(AgentResponse::PromptResponse(PromptResponse::new(
                        StopReason::EndTurn,
                    ))),
                    // A user cancel is NOT an error — the protocol contract
                    // (agent-client-protocol-schema, session/cancel) requires
                    // replying to the original prompt request with
                    // `StopReason::Cancelled` (Phase 3 slice #7).
                    Err(crate::acp_server::AcpError::AgentLoop(
                        orz_loop::controller::AgentLoopError::Cancelled,
                    )) => Ok(AgentResponse::PromptResponse(PromptResponse::new(
                        StopReason::Cancelled,
                    ))),
                    Err(e) => Err(acp::Error::into_internal_error(e)),
                }
            }
            ClientRequest::ExtMethodRequest(args) => {
                // 0di (2026-10-18, ADR-0010 §14.88): the client→agent
                // extension surface — queued supplements (`_orz/queue_append`)
                // and recall (`_orz/recall_preview` / `_orz/recall`). The acp
                // crate strips the `_` prefix before routing here
                // (`ExtRequest.method` = "orz/…"); unknown ext methods keep
                // the protocol's method-not-found semantics. Zero model-face
                // tool surface touched (ext methods are a client↔agent
                // protocol face, not model tools).
                let params: serde_json::Value =
                    serde_json::from_str(args.params.get()).unwrap_or(serde_json::Value::Null);
                match args.method.as_ref() {
                    "orz/queue_append" => {
                        let session_id = params
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let text = params
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default();
                        if session_id.is_empty() {
                            return Err(acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "queue_append requires session_id",
                            ));
                        }
                        self.server
                            .handle_queue_append(&session_id, text)
                            .map(ext_response_value)
                            .map_err(acp::Error::into_internal_error)
                    }
                    "orz/recall_preview" => {
                        let session_id = params
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        self.server
                            .handle_recall_preview(&session_id)
                            .map(ext_response_value)
                            .map_err(acp::Error::into_internal_error)
                    }
                    "orz/recall" => {
                        let session_id = params
                            .get("session_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let ordinal = params.get("ordinal").and_then(|v| v.as_u64());
                        let Some(ordinal) = ordinal else {
                            return Err(acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "recall requires ordinal (u32)",
                            ));
                        };
                        let ordinal = u32::try_from(ordinal).map_err(|_| {
                            acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "recall ordinal out of range",
                            )
                        })?;
                        self.server
                            .handle_recall(&session_id, ordinal)
                            .map(ext_response_value)
                            .map_err(acp::Error::into_internal_error)
                    }
                    // 0dk S2（332）：会话生命周期私有扩展（设计 §5.2）。
                    // `_orz/session_close` 先 cancel 再释放（10s 上限），
                    // `_orz/session_state` 只查不移，`_orz/session_delete`
                    // 先 close 再删数据并写墓碑。
                    "orz/session_close" => {
                        let session_id = ext_session_id(&params);
                        if session_id.is_empty() {
                            return Err(acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "session_close requires sessionId",
                            ));
                        }
                        let reason = crate::session_ops::CloseReason::parse(
                            params.get("reason").and_then(|v| v.as_str()),
                        );
                        let outcome = self
                            .server
                            .handle_session_close(&session_id, reason)
                            .await
                            .map_err(session_error_to_acp)?;
                        Ok(ext_response_value(
                            serde_json::to_value(outcome).unwrap_or(serde_json::Value::Null),
                        ))
                    }
                    "orz/session_state" => {
                        let session_id = ext_session_id(&params);
                        if session_id.is_empty() {
                            return Err(acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "session_state requires sessionId",
                            ));
                        }
                        let outcome = self
                            .server
                            .handle_session_state(&session_id)
                            .map_err(session_error_to_acp)?;
                        Ok(ext_response_value(
                            serde_json::to_value(outcome).unwrap_or(serde_json::Value::Null),
                        ))
                    }
                    "orz/session_delete" => {
                        let session_id = ext_session_id(&params);
                        if session_id.is_empty() {
                            return Err(acp::Error::new(
                                acp::ErrorCode::InvalidRequest.into(),
                                "session_delete requires sessionId",
                            ));
                        }
                        let value = self
                            .server
                            .handle_session_delete(&session_id)
                            .await
                            .map_err(session_error_to_acp)?;
                        Ok(ext_response_value(value))
                    }
                    _ => Err(acp::Error::method_not_found()),
                }
            }
            _ => Err(acp::Error::method_not_found()),
        }
    }

    async fn handle_notification(&self, notification: ClientNotification) -> acp::Result<()> {
        match notification {
            ClientNotification::CancelNotification(args) => {
                // Phase 3 slice #7: route the cancel to the in-flight run's
                // token. The loop polls it cooperatively at its checkpoints;
                // a cancel when idle or already finished is a benign no-op.
                let session_id = args.session_id.0.as_ref().to_string();
                self.server.cancel_current_run(&session_id);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// Serve the ACP protocol over stdio until the connection closes.
///
/// Wires the outbound gateway into the server (permission bridge reads it),
/// then awaits the connection I/O future — which completes on stdin EOF.
///
/// `retrieval_enabled` (0t, ADR-0010 §14.65): the session-level 检索启用门
/// applied to every session created over this connection; `false` =
/// fail-closed（无检索工具）。
pub async fn run_stdio_server(server: Arc<AcpServer>, retrieval_enabled: bool) -> acp::Result<()> {
    let handler = StdioAgentHandler::with_retrieval_enabled(
        server.clone(),
        crate::session::TrustPolicy::Enforce,
        retrieval_enabled,
    );
    let (conn, io_future) = acp::AgentSideConnection::new(
        handler,
        tokio::io::stdout().compat_write(),
        tokio::io::stdin().compat(),
        |fut| {
            tokio::task::spawn_local(fut);
        },
    );
    let (sender, receiver) = acp_gateway::<acp::AgentSide, _>(conn);
    server.set_gateway(sender);
    tokio::task::spawn_local(receiver.run());
    io_future.await
}
