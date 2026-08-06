//! Journal verifier — replay and verify the integrity of a journal.
//!
//! Reads JSONL line by line, validates each event against the schema invariants,
//! and checks hash chain continuity. Designed for both in-process verification
//! (e.g. on session close) and offline auditing.

use std::path::Path;

use crate::journal::chain::validate_chain;
use crate::journal::event::RunEvent;

/// Result of a journal replay + verification pass.
#[derive(Debug)]
pub struct ReplayResult {
    pub valid: bool,
    pub run_id: String,
    pub run_manifest_sha256: String,
    pub journal_path: String,
    pub event_count: u64,
    pub last_event_sha256: Option<String>,
    pub terminal_event: Option<String>,
    pub errors: Vec<String>,
    pub limitations: Vec<String>,
}

/// Replay a journal from a JSONL file and verify its integrity.
///
/// Parameters:
/// - `journal_path`: Path to the `events.jsonl` file
/// - `expected_run_id`: If provided, every event must match this run_id
/// - `expected_manifest_sha256`: If provided, every event must match
/// - `require_terminal`: If true, require exactly one terminal event at the end
pub fn replay_journal(
    journal_path: &Path,
    expected_run_id: Option<&str>,
    expected_manifest_sha256: Option<&str>,
    require_terminal: bool,
) -> ReplayResult {
    let mut errors: Vec<String> = Vec::new();

    if !journal_path.is_file() {
        errors.push(format!(
            "journal file not found: {}",
            journal_path.display()
        ));
        return ReplayResult {
            valid: false,
            run_id: String::new(),
            run_manifest_sha256: String::new(),
            journal_path: journal_path.display().to_string(),
            event_count: 0,
            last_event_sha256: None,
            terminal_event: None,
            errors,
            limitations: vec![
                "Replay verifies mechanics and does not score model correctness.".into(),
            ],
        };
    }

    // Read and parse all lines
    let raw = match std::fs::read_to_string(journal_path) {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!("failed to read journal: {e}"));
            return ReplayResult {
                valid: false,
                run_id: String::new(),
                run_manifest_sha256: String::new(),
                journal_path: journal_path.display().to_string(),
                event_count: 0,
                last_event_sha256: None,
                terminal_event: None,
                errors,
                limitations: vec![
                    "Replay verifies mechanics and does not score model correctness.".into(),
                ],
            };
        }
    };

    let mut events: Vec<RunEvent> = Vec::new();
    for (line_number, line) in raw.lines().enumerate() {
        let line_num = line_number + 1; // 1-indexed

        if line.trim().is_empty() {
            errors.push(format!("blank journal line {line_num}"));
            continue;
        }

        match serde_json::from_str::<RunEvent>(line) {
            Ok(event) => events.push(event),
            Err(e) => {
                errors.push(format!("invalid JSON at line {line_num}: {e}"));
                continue;
            }
        }
    }

    if events.is_empty() {
        errors.push("journal contains no valid events".into());
        return ReplayResult {
            valid: false,
            run_id: String::new(),
            run_manifest_sha256: String::new(),
            journal_path: journal_path.display().to_string(),
            event_count: 0,
            last_event_sha256: None,
            terminal_event: None,
            errors,
            limitations: vec![
                "Replay verifies mechanics and does not score model correctness.".into(),
            ],
        };
    }

    // Determine expected identifiers from the first event if not provided
    let run_id = expected_run_id
        .map(String::from)
        .unwrap_or_else(|| events[0].run_id.clone());
    let manifest_sha256 = expected_manifest_sha256
        .map(String::from)
        .unwrap_or_else(|| events[0].run_manifest_sha256.clone());

    let chain_result = validate_chain(&events, &run_id, &manifest_sha256, require_terminal);
    errors.extend(chain_result.errors);

    ReplayResult {
        valid: errors.is_empty(),
        run_id,
        run_manifest_sha256: manifest_sha256,
        journal_path: journal_path.display().to_string(),
        event_count: chain_result.event_count,
        last_event_sha256: chain_result.last_event_sha256,
        terminal_event: chain_result.terminal_event,
        errors,
        limitations: vec!["Replay verifies mechanics and does not score model correctness.".into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::chain::seal_event;
    use crate::journal::event::{EventType, Redaction};
    use std::io::Write;

    fn make_event(run_id: &str, seq: u64, et: EventType, prev: Option<String>) -> RunEvent {
        let mut e = RunEvent::new(
            run_id.into(),
            seq,
            et,
            "abcd-manifest-sha-64chars-long_____________________".into(),
            prev,
            "test-schema".into(),
            serde_json::json!({"seq": seq}),
            Redaction::None,
            "2026-08-04T00:00:00Z".into(),
        );
        seal_event(&mut e).unwrap();
        e
    }

    fn write_chain(dir: &Path, events: &[RunEvent]) {
        let path = dir.join("events.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        for e in events {
            let line = serde_json::to_string(e).unwrap();
            writeln!(f, "{line}").unwrap();
        }
    }

    #[test]
    fn valid_journal_replays_successfully() {
        let dir = std::env::temp_dir().join(format!("orz-verify-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut prev: Option<String> = None;
        let mut events = Vec::new();
        for (seq, et) in [
            EventType::RunStarted,
            EventType::PromptSubmitted,
            EventType::RunFinished,
        ]
        .iter()
        .enumerate()
        {
            let mut e = make_event("RUN-REPLAY", seq as u64, et.clone(), prev);
            seal_event(&mut e).unwrap();
            prev = Some(e.event_sha256.clone());
            events.push(e);
        }
        write_chain(&dir, &events);

        let result = replay_journal(&dir.join("events.jsonl"), None, None, true);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert_eq!(result.event_count, 3);
        assert_eq!(result.terminal_event.unwrap(), "run_finished");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tampered_journal_detected() {
        let dir = std::env::temp_dir().join(format!("orz-tamper-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let mut prev: Option<String> = None;
        let mut events: Vec<RunEvent> = Vec::new();
        for (seq, et) in [
            EventType::RunStarted,
            EventType::PromptSubmitted,
            EventType::RunFinished,
        ]
        .iter()
        .enumerate()
        {
            let mut e = make_event("RUN-TAMP", seq as u64, et.clone(), prev);
            seal_event(&mut e).unwrap();
            prev = Some(e.event_sha256.clone());
            events.push(e);
        }

        // Tamper: change payload without re-hashing
        events[1].payload = serde_json::json!({"injected": true});
        write_chain(&dir, &events);

        let result = replay_journal(&dir.join("events.jsonl"), None, None, true);
        assert!(!result.valid, "should detect tampering");
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("payload digest") || e.contains("event digest")),
            "errors: {:?}",
            result.errors
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_journal_is_error() {
        let result = replay_journal(
            Path::new("/nonexistent/journal/events.jsonl"),
            None,
            None,
            true,
        );
        assert!(!result.valid);
    }
}
