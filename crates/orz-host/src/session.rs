//! Session bootstrap — workspace trust, envelope init, journal creation.
//!
//! This is where orz-host wires the Grok providers into the assurance journal.
//! Every run starts here: trust verification → journal creation → run_preflight.
//!
//! See: INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §3.3

use std::path::PathBuf;

use orz_assurance::journal::JournalRecorder;
use orz_assurance::{EventType, Redaction, RunEvent};

/// Error during session bootstrap.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("workspace not trusted: {0}")]
    TrustFailed(String),
    #[error("journal error: {0}")]
    Journal(#[from] orz_assurance::JournalRecorderError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result of bootstrapping a new session.
pub struct SessionHandle {
    pub run_id: String,
    pub run_manifest_sha256: String,
    pub journal: JournalRecorder,
    pub journal_dir: PathBuf,
    /// Sequence number of the next event to append (continues the chain).
    pub next_sequence: u64,
    /// SHA-256 of the last appended event (chain link for the next event).
    pub last_event_sha256: Option<String>,
}

/// Bootstrap a new agent session.
///
/// 1. Determine journal directory (`.gsa/runs/{run_id}/`)
/// 2. Create the JournalRecorder
/// 3. Record run_preflight event
/// 4. Return SessionHandle ready for AgentLoopController
pub async fn bootstrap_session(
    run_id: &str,
    base_dir: Option<PathBuf>,
) -> Result<SessionHandle, SessionError> {
    let base = base_dir.unwrap_or_else(|| PathBuf::from(".gsa"));
    let journal_dir = base.join("runs").join(run_id);

    let journal = JournalRecorder::new(journal_dir.clone());

    // Compute a manifest SHA-256
    let manifest = serde_json::json!({
        "run_id": run_id,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "schema_version": "0.1.0-draft",
    });
    let manifest_sha256 = orz_assurance::sha256_hex(
        &orz_assurance::canonical_json(&manifest).unwrap_or_default(),
    );

    // Record run_preflight as event 0
    let mut preflight = RunEvent::new(
        run_id.into(),
        0,
        EventType::RunPreflight,
        manifest_sha256.clone(),
        None,
        "run-event-v0.1.schema.json".into(),
        manifest,
        Redaction::None,
        chrono::Utc::now().to_rfc3339(),
    );
    orz_assurance::seal_event(&mut preflight)
        .map_err(|e| SessionError::Journal(orz_assurance::JournalRecorderError::Serde(e)))?;

    // The chain continues from the sealed preflight event (read before move).
    let next_sequence = preflight.sequence + 1;
    let last_event_sha256 = Some(preflight.event_sha256.clone());

    journal.record_async(preflight).await?;

    Ok(SessionHandle {
        run_id: run_id.into(),
        run_manifest_sha256: manifest_sha256,
        journal,
        journal_dir,
        next_sequence,
        last_event_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("orz-host-session-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn bootstrap_creates_journal_with_preflight() {
        let base = test_dir();
        let run_id = "RUN-BOOTSTRAP-TEST";

        let handle = bootstrap_session(run_id, Some(base.clone())).await.unwrap();

        assert_eq!(handle.run_id, run_id);
        assert!(!handle.run_manifest_sha256.is_empty());

        // Flush first to ensure events are written (journal writes are async)
        handle.journal.flush_async().await.unwrap();

        // Verify preflight event was recorded
        let replay = orz_assurance::replay_journal(
            &handle.journal_dir.join("events.jsonl"),
            Some(run_id),
            None,
            false, // don't require terminal — session is still open
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.event_count, 1);

        let _ = std::fs::remove_dir_all(&base);
    }
}
