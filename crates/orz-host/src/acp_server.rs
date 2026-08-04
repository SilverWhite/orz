//! ACP JSON-RPC stdio server — the entry point for the agent communication protocol.
//!
//! Uses `agent-client-protocol` 0.10.4 + `xai-acp-lib` for gateway/channels.
//! Implements `session/new` and `session/prompt` methods.
//! Delegates agent turns to `orz_loop::AgentLoopController` via the `LoopHost` trait.
//!
//! Phase 1: scaffold — session/new + session/prompt with stub gateway.
//! Full ACP lifecycle (tool calls, permissions, notifications) in Phase 2.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use std::path::PathBuf;

use orz_loop::AgentLoopController;

use crate::session::{bootstrap_session, SessionError};

/// Errors from the ACP server layer.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("agent loop error: {0}")]
    AgentLoop(#[from] orz_loop::controller::AgentLoopError),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

/// Per-session metadata. Journals are per-**run** (one per prompt), so the
/// session itself holds no journal — each `session/prompt` bootstraps a fresh
/// run with its own hash-chained journal.
struct StoredSession {
    base_dir: PathBuf,
    trust_policy: crate::session::TrustPolicy,
    /// Prompt counter — each prompt gets a unique run id (`RUN-{suffix}-{n}`).
    prompt_count: u64,
}

/// The ACP server — holds active sessions and dispatches requests.
pub struct AcpServer {
    sessions: Arc<Mutex<HashMap<String, StoredSession>>>,
    /// Outbound ACP gateway (agent → client messages: permissions etc.).
    /// `None` in headless mode — permission prompts fail closed.
    gateway: Arc<Mutex<Option<AcpAgentGatewaySender>>>,
    /// Model gateway for agent turns (scripted FakeProvider offline).
    model_gateway: Arc<dyn ModelGateway>,
}

impl AcpServer {
    pub fn new() -> Self {
        Self::with_gateway(Arc::new(FakeProvider::from_texts(vec![
            "(fake) 已收到请求。",
        ])))
    }

    pub fn with_gateway(model_gateway: Arc<dyn ModelGateway>) -> Self {
        AcpServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            gateway: Arc::new(Mutex::new(None)),
            model_gateway,
        }
    }

    /// Set the outbound ACP gateway (wired by the stdio server; consumed by
    /// the permission bridge).
    pub fn set_gateway(&self, sender: AcpAgentGatewaySender) {
        *self.gateway.lock().unwrap() = Some(sender);
    }

    /// Outbound gateway, if interactive mode is active.
    pub fn gateway(&self) -> Option<AcpAgentGatewaySender> {
        self.gateway.lock().unwrap().clone()
    }

    /// Handle a `session/new` request.
    ///
    /// `base_dir` is the directory under which journals are written
    /// (`{base_dir}/runs/{run_id}/events.jsonl`). Tests must pass an isolated
    /// temp dir so no `.gsa/` residue appears in the workspace.
    ///
    /// `trust_policy` controls the workspace-trust gate: production paths pass
    /// `TrustPolicy::Enforce`; tests pass `TrustPolicy::Skip` (trust semantics
    /// are covered by `session::tests`).
    pub async fn handle_session_new(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
        trust_policy: crate::session::TrustPolicy,
    ) -> Result<serde_json::Value, AcpError> {
        // Journals are created per-run at `session/prompt` time — the session
        // itself only records where and under what trust policy runs live.
        self.sessions.lock().unwrap().insert(
            session_id.to_string(),
            StoredSession {
                base_dir: base_dir.unwrap_or_else(|| PathBuf::from(".")),
                trust_policy,
                prompt_count: 0,
            },
        );

        Ok(serde_json::json!({
            "session_id": session_id,
            "status": "created",
        }))
    }

    /// Handle a `session/prompt` request.
    ///
    /// Each prompt is its own **run** with a fresh hash-chained journal. A
    /// journal is a single-run integrity unit — the verifier rejects multiple
    /// terminal events — so multi-prompt sessions get one journal per prompt
    /// instead of sharing one chain (2026-08-04 review P0).
    pub async fn handle_session_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<serde_json::Value, AcpError> {
        let (base_dir, trust_policy, prompt_number) = {
            let sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            (
                session.base_dir.clone(),
                session.trust_policy,
                session.prompt_count,
            )
        };

        let suffix: String = session_id.chars().take(8).collect();
        let run_id = format!("RUN-{suffix}-{prompt_number}");
        let handle = bootstrap_session(&run_id, Some(base_dir), trust_policy).await?;

        let host = JournalOnlyHost::new(handle.journal.clone());
        let controller = AgentLoopController::with_gateway(self.model_gateway.clone());
        let (response, _, _) = controller
            .run_turn(
                &host,
                prompt,
                &handle.run_id,
                &handle.run_manifest_sha256,
                handle.next_sequence,
                handle.last_event_sha256.clone(),
            )
            .await?;

        // The run journal is complete — release the writer task (best effort;
        // the journal content was already flushed inside run_turn).
        let _ = handle.journal.shutdown_async().await;

        // Advance the counter so the next prompt gets a fresh run id.
        {
            let mut sessions = self.sessions.lock().unwrap();
            if let Some(session) = sessions.get_mut(session_id) {
                session.prompt_count += 1;
            }
        }

        Ok(serde_json::json!({
            "session_id": session_id,
            "response": response,
            "status": "completed",
            "run_id": run_id,
        }))
    }

    /// List active session IDs.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().unwrap().keys().cloned().collect()
    }

    /// Close a session and release its stored metadata.
    ///
    /// ACP 0.10.4 has no `session/close` method — the stdio server terminates
    /// on stdin EOF — but explicit closure keeps the session table bounded
    /// for long-lived/embedded hosts (2026-08-04 review P2-4).
    /// Returns `true` if the session existed and was removed.
    pub fn close_session(&self, session_id: &str) -> bool {
        self.sessions.lock().unwrap().remove(session_id).is_some()
    }
}

impl Default for AcpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Phase 1: Minimal LoopHost implementation ─────────────────────────

use orz_assurance::JournalRecorder;
use orz_loop::gateway::fake::FakeProvider;
use orz_loop::gateway::model::ModelGateway;
use orz_loop::host::{LoopHost, ToolDef, ToolRegistry};
use xai_acp_lib::AcpAgentGatewaySender;
use async_trait::async_trait;

/// Phase 1 minimal host — only provides journal access.
/// Tool registry, permissions, etc. are stubbed.
pub struct JournalOnlyHost {
    journal: JournalRecorder,
}

impl JournalOnlyHost {
    pub fn new(journal: JournalRecorder) -> Self {
        JournalOnlyHost { journal }
    }
}

struct EmptyRegistry;
impl ToolRegistry for EmptyRegistry {
    fn get(&self, _name: &str) -> Option<ToolDef> {
        None
    }
    fn list(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

#[async_trait]
impl LoopHost for JournalOnlyHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }
    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyRegistry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::path::PathBuf;

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(
            format!("orz-acp-test-{}-{}", std::process::id(), n)
        );
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn session_new_registers_session() {
        let base = test_dir();

        let server = AcpServer::new();
        let result = server
            .handle_session_new(
                "test-session-1234",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        assert_eq!(result["session_id"], "test-session-1234");
        assert_eq!(result["status"], "created");

        let sessions = server.list_sessions();
        assert!(sessions.contains(&"test-session-1234".to_string()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn session_prompt_produces_valid_journal_chain() {
        let base = test_dir();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "test-session-prompt",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        let result = server
            .handle_session_prompt("test-session-prompt", "hello world")
            .await
            .unwrap();

        assert_eq!(result["session_id"], "test-session-prompt");
        assert_eq!(result["status"], "completed");
        assert!(result["response"].as_str().unwrap().contains("已收到请求"));

        // The full ACP path (session/new → session/prompt) must produce a
        // continuous hash chain: preflight → started → prompt → output → finished.
        let runs_dir = base.join(".gsa").join("runs");
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(run_dirs.len(), 1, "expected exactly one run journal");
        let events_path = run_dirs[0].join("events.jsonl");

        let replay = orz_assurance::replay_journal(&events_path, None, None, true);
        assert!(
            replay.valid,
            "ACP path journal invalid: {:?}",
            replay.errors
        );
        // Full Phase 2 gate chain: preflight + started + prompt_submitted +
        // orientation + tool_availability + model_output + stagnation + finished.
        assert_eq!(replay.event_count, 8);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn unknown_session_returns_error() {
        let server = AcpServer::new();
        let result = server.handle_session_prompt("nonexistent", "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn second_prompt_continues_hash_chain() {
        let base = test_dir();

        // Two scripted responses — one per prompt turn (the default gateway
        // has a single response and would exhaust on the second prompt).
        let server = AcpServer::with_gateway(Arc::new(FakeProvider::from_texts(vec![
            "(fake) 第一轮。",
            "(fake) 第二轮。",
        ])));
        server
            .handle_session_new(
                "test-session-two-prompts",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        let first = server
            .handle_session_prompt("test-session-two-prompts", "hello")
            .await
            .unwrap();
        assert_eq!(first["status"], "completed");

        let second = server
            .handle_session_prompt("test-session-two-prompts", "world")
            .await
            .unwrap();
        assert_eq!(second["status"], "completed");

        // Each prompt is its own run journal — a journal is a single-run
        // hash chain with exactly one terminal event (2026-08-04 review P0).
        let runs_dir = base.join(".gsa").join("runs");
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(run_dirs.len(), 2, "one run journal per prompt");

        for dir in &run_dirs {
            let replay = orz_assurance::replay_journal(&dir.join("events.jsonl"), None, None, true);
            assert!(
                replay.valid,
                "run journal invalid: {:?}",
                replay.errors
            );
            assert_eq!(replay.event_count, 8, "preflight + 7 turn events");
            assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
        }

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn close_session_releases_metadata() {
        let base = test_dir();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "test-session-close",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        assert!(server.list_sessions().contains(&"test-session-close".to_string()));

        assert!(server.close_session("test-session-close"));
        assert!(!server.list_sessions().contains(&"test-session-close".to_string()));
        // Closing a nonexistent session reports false.
        assert!(!server.close_session("test-session-close"));

        // A closed session rejects new prompts.
        let result = server.handle_session_prompt("test-session-close", "hi").await;
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&base);
    }
}
