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
    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): session-level 检索启用门
    /// ——三值模式退役后唯一授权状态；`false` = fail-closed（无检索工具）。
    retrieval_enabled: bool,
}

impl StdioAgentHandler {
    /// Default handler with the strict `Enforce` trust policy.
    pub fn new(server: Arc<AcpServer>) -> Self {
        Self::with_trust_policy(server, crate::session::TrustPolicy::Enforce)
    }

    /// Handler with an explicit trust policy — the TUI's in-process wiring
    /// and integration tests run against temp workspaces via `Skip`.
    pub fn with_trust_policy(
        server: Arc<AcpServer>,
        trust_policy: crate::session::TrustPolicy,
    ) -> Self {
        Self {
            server,
            trust_policy,
            retrieval_enabled: false,
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

impl acp::MessageHandler<acp::AgentSide> for StdioAgentHandler {
    async fn handle_request(&self, request: ClientRequest) -> acp::Result<AgentResponse> {
        match request {
            ClientRequest::InitializeRequest(args) => {
                let response = InitializeResponse::new(args.protocol_version)
                    .agent_capabilities(AgentCapabilities::default());
                Ok(AgentResponse::InitializeResponse(response))
            }
            ClientRequest::NewSessionRequest(args) => {
                // The client asks for a new session; the agent generates the ID.
                let session_id = uuid::Uuid::new_v4().to_string();
                self.server
                    .handle_session_new_with_options(
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
