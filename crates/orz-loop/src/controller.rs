//! AgentLoopController — the main agent loop.
//!
//! Replaces Grok Sampler's ~10k lines with a clean while loop (~300 lines).
//!
//! Flow:
//!   1. PromptBuilder assembles system prompt + tool defs + injection points
//!   2. ModelGateway sends prompt to LLM, streams response
//!   3. For each tool_call in response: MechanicalRelay routes to target agent
//!   4. ToolDispatcher wraps tool execution with assurance checks
//!   5. JournalWriter records every event
//!   6. OrientationMonitor injects checkpoints periodically
//!   7. Loop until text-only response or terminal condition
//!
//! Phase 1: single-agent mode with stub gateway. Produces valid journal chain.

use orz_assurance::{
    seal_event, EventType, JournalRecorder, JournalRecorderError, Redaction, RunEvent,
};

use crate::host::LoopHost;

/// Error during agent loop execution.
#[derive(Debug, thiserror::Error)]
pub enum AgentLoopError {
    #[error("journal error: {0}")]
    Journal(#[from] JournalRecorderError),
    #[error("model error: {0}")]
    Model(String),
    #[error("session error: {0}")]
    Session(String),
}

/// The main agent loop controller.
///
/// Owns the prompt processing lifecycle. Stateless between turns —
/// all persistent state lives in the Blackboard and Journal.
pub struct AgentLoopController;

impl AgentLoopController {
    pub fn new() -> Self {
        AgentLoopController
    }

    /// Run a single turn of the agent loop for a given user prompt.
    ///
    /// This is the entry point called by orz-host's ACP `session/prompt` handler.
    /// It writes all events to the host's journal and returns when the turn finishes.
    ///
    /// `next_sequence` and `previous_event_sha256` continue the journal's hash
    /// chain from where the session bootstrap (or a prior turn) left off.
    ///
    /// Phase 1: stub model gateway returns a text-only response, which
    /// triggers the terminal condition (no tool calls → run_finished).
    pub async fn run_turn(
        &self,
        host: &dyn LoopHost,
        prompt: &str,
        run_id: &str,
        run_manifest_sha256: &str,
        next_sequence: u64,
        previous_event_sha256: Option<String>,
    ) -> Result<String, AgentLoopError> {
        let journal = host.journal();
        let mut prev_hash = previous_event_sha256;
        let manifest = run_manifest_sha256.to_string();
        let rid = run_id.to_string();
        let mut seq = next_sequence;

        // Helper to build, seal, and record an event, tracking the hash chain
        async fn record_event(
            journal: &JournalRecorder,
            run_id: &str,
            manifest_sha256: &str,
            prev_hash: &mut Option<String>,
            seq: u64,
            event_type: EventType,
            payload: serde_json::Value,
        ) -> Result<(), AgentLoopError> {
            let mut event = RunEvent::new(
                run_id.into(),
                seq,
                event_type,
                manifest_sha256.into(),
                prev_hash.clone(),
                "run-event-v0.1.schema.json".into(),
                payload,
                Redaction::None,
                chrono_utc_now(),
            );
            seal_event(&mut event)
                .map_err(|e| AgentLoopError::Model(e.to_string()))?;
            *prev_hash = Some(event.event_sha256.clone());
            journal.record_async(event).await?;
            Ok(())
        }

        // 1. run_started
        record_event(journal, &rid, &manifest, &mut prev_hash, seq,
            EventType::RunStarted, serde_json::json!({"prompt": prompt})).await?;
        seq += 1;

        // 2. prompt_submitted
        record_event(journal, &rid, &manifest, &mut prev_hash, seq,
            EventType::PromptSubmitted,
            serde_json::json!({"prompt": prompt, "character_count": prompt.len()})).await?;
        seq += 1;

        // 3. Phase 1 stub response
        let response_text = format!(
            "[Phase 1 stub] Received prompt ({} chars). Model gateway not yet integrated.",
            prompt.len()
        );

        // 4. model_output
        record_event(journal, &rid, &manifest, &mut prev_hash, seq,
            EventType::ModelOutput,
            serde_json::json!({"text": response_text, "tool_calls": [], "finish_reason": "stop"})).await?;
        seq += 1;

        // 5. run_finished (terminal)
        record_event(journal, &rid, &manifest, &mut prev_hash, seq,
            EventType::RunFinished,
            serde_json::json!({"status": "completed", "turn_count": 1})).await?;

        // 6. Flush to disk
        journal.flush_async().await?;

        Ok(response_text)
    }
}

impl Default for AgentLoopController {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current UTC timestamp in ISO 8601 format.
fn chrono_utc_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{LoopHost, ToolDef, ToolRegistry};
    use async_trait::async_trait;
    use orz_assurance::JournalRecorder;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Minimal LoopHost for testing the controller.
    struct TestHost {
        journal: JournalRecorder,
    }

    struct EmptyRegistry;
    impl ToolRegistry for EmptyRegistry {
        fn get(&self, _name: &str) -> Option<ToolDef> { None }
        fn list(&self) -> Vec<ToolDef> { Vec::new() }
    }

    #[async_trait]
    impl LoopHost for TestHost {
        fn journal(&self) -> &JournalRecorder { &self.journal }
        fn tools_registry(&self) -> &dyn ToolRegistry { &EmptyRegistry }
    }

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("orz-controller-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn run_turn_produces_valid_journal_chain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost { journal };

        let controller = AgentLoopController::new();
        let result = controller
            .run_turn(
                &host,
                "hello world",
                "RUN-TEST-TURN",
                "abcd-manifest-sha-64chars-long_____________________",
                0,
                None,
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("Phase 1 stub"));

        // Verify journal
        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-TEST-TURN"),
            None,
            true,
        );
        assert!(replay.valid, "journal validation errors: {:?}", replay.errors);
        assert_eq!(replay.event_count, 4);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn journal_contains_expected_event_types() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost { journal };

        let controller = AgentLoopController::new();
        controller
            .run_turn(&host, "test", "RUN-TEST-EVENTS", "abcd-manifest-sha-64chars-long_____________________", 0, None)
            .await
            .unwrap();

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let events: Vec<RunEvent> = content
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].event_type, EventType::RunStarted);
        assert_eq!(events[1].event_type, EventType::PromptSubmitted);
        assert_eq!(events[2].event_type, EventType::ModelOutput);
        assert_eq!(events[3].event_type, EventType::RunFinished);

        assert!(events[0].previous_event_sha256.is_none());
        assert_eq!(
            events[1].previous_event_sha256.as_deref(),
            Some(events[0].event_sha256.as_str())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
