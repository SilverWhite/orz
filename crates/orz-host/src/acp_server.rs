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

use crate::session::{bootstrap_session, SessionHandle, SessionError};

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

/// The ACP server — holds active sessions and dispatches requests.
pub struct AcpServer {
    sessions: Arc<Mutex<HashMap<String, SessionHandle>>>,
}

impl AcpServer {
    pub fn new() -> Self {
        AcpServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Handle a `session/new` request.
    ///
    /// `base_dir` is the directory under which journals are written
    /// (`{base_dir}/runs/{run_id}/events.jsonl`). Tests must pass an isolated
    /// temp dir so no `.gsa/` residue appears in the workspace.
    pub async fn handle_session_new(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
    ) -> Result<serde_json::Value, AcpError> {
        // Byte-slice defensively: take at most 8 bytes, but only on a char boundary
        let suffix: String = session_id.chars().take(8).collect();
        let run_id = format!("RUN-{suffix}");
        let handle = bootstrap_session(&run_id, base_dir).await?;

        let result = serde_json::json!({
            "session_id": session_id,
            "run_id": handle.run_id,
            "status": "created",
        });

        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.to_string(), handle);

        Ok(result)
    }

    /// Handle a `session/prompt` request.
    pub async fn handle_session_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<serde_json::Value, AcpError> {
        let sessions = self.sessions.lock().unwrap();
        let handle = sessions
            .get(session_id)
            .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;

        let run_id = handle.run_id.clone();
        let manifest_sha256 = handle.run_manifest_sha256.clone();
        let next_sequence = handle.next_sequence;
        let last_event_sha256 = handle.last_event_sha256.clone();

        let host = JournalOnlyHost {
            journal: handle.journal.clone(),
        };

        let controller = AgentLoopController::new();
        let response = controller
            .run_turn(&host, prompt, &run_id, &manifest_sha256, next_sequence, last_event_sha256)
            .await?;

        Ok(serde_json::json!({
            "session_id": session_id,
            "response": response,
            "status": "completed",
        }))
    }

    /// List active session IDs.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().unwrap().keys().cloned().collect()
    }
}

impl Default for AcpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Phase 1: Minimal LoopHost implementation ─────────────────────────

use orz_assurance::JournalRecorder;
use orz_loop::host::{LoopHost, ToolDef, ToolRegistry};
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
    async fn session_new_creates_journal() {
        let base = test_dir();

        let server = AcpServer::new();
        let result = server
            .handle_session_new("test-session-1234", Some(base.clone()))
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
            .handle_session_new("test-session-prompt", Some(base.clone()))
            .await
            .unwrap();

        let result = server
            .handle_session_prompt("test-session-prompt", "hello world")
            .await
            .unwrap();

        assert_eq!(result["session_id"], "test-session-prompt");
        assert_eq!(result["status"], "completed");
        assert!(result["response"].as_str().unwrap().contains("Phase 1 stub"));

        // The full ACP path (session/new → session/prompt) must produce a
        // continuous hash chain: preflight → started → prompt → output → finished.
        let runs_dir = base.join("runs");
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
        assert_eq!(replay.event_count, 5);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn unknown_session_returns_error() {
        let server = AcpServer::new();
        let result = server.handle_session_prompt("nonexistent", "test").await;
        assert!(result.is_err());
    }
}
