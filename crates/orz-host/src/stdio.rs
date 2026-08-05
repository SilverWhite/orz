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
                    .handle_session_new(&session_id, Some(args.cwd), self.trust_policy)
                    .await
                    .map_err(acp::Error::into_internal_error)?;
                Ok(AgentResponse::NewSessionResponse(NewSessionResponse::new(
                    SessionId::new(session_id),
                )))
            }
            ClientRequest::PromptRequest(args) => {
                let session_id = args.session_id.0.as_ref().to_string();
                let prompt = extract_prompt_text(&args.prompt);
                self.server
                    .handle_session_prompt(&session_id, &prompt)
                    .await
                    .map_err(acp::Error::into_internal_error)?;
                Ok(AgentResponse::PromptResponse(PromptResponse::new(
                    StopReason::EndTurn,
                )))
            }
            _ => Err(acp::Error::method_not_found()),
        }
    }

    async fn handle_notification(&self, notification: ClientNotification) -> acp::Result<()> {
        match notification {
            ClientNotification::CancelNotification(_args) => {
                // Phase 2: the controller runs the turn to completion;
                // cancellation wiring lands with streaming (Phase 3).
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
pub async fn run_stdio_server(server: Arc<AcpServer>) -> acp::Result<()> {
    let handler = StdioAgentHandler::new(server.clone());
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
