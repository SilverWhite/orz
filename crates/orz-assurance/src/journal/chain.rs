//! Hash chain logic for the append-only journal.
//!
//! Each event's `event_sha256` is computed from the canonical JSON of the event
//! minus the `event_sha256` field itself. The `payload_sha256` is computed separately
//! from the canonical JSON of the payload.
//!
//! Canonical JSON: sorted keys, no whitespace, compact separators (matching Python's
//! `json.dumps(sort_keys=True, separators=(",", ":"))`).

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::event::RunEvent;

/// Produce canonical JSON bytes for a value: sorted keys, compact, no whitespace.
///
/// Matches Python: `json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False)`
pub fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, serde_json::Error> {
    let value = serde_json::to_value(value)?;
    let sorted = sort_json_keys(value);
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::new(&mut buf);
    sorted.serialize(&mut ser)?;
    Ok(buf)
}

/// Recursively sort all object keys in a JSON value.
fn sort_json_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map
                .into_iter()
                .map(|(k, v)| (k, sort_json_keys(v)))
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            serde_json::Value::Object(
                entries.into_iter().collect()
            )
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(
                arr.into_iter().map(sort_json_keys).collect()
            )
        }
        other => other,
    }
}

/// Compute SHA-256 hex digest of bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in hash.iter() {
        use std::fmt::Write;
        write!(&mut hex, "{byte:02x}").unwrap();
    }
    hex
}

/// Compute the SHA-256 of a payload's canonical JSON.
pub fn payload_hash(payload: &impl Serialize) -> Result<String, serde_json::Error> {
    let bytes = canonical_json(payload)?;
    Ok(sha256_hex(&bytes))
}

/// Compute `event_sha256`: canonical JSON of the event MINUS the `event_sha256` field,
/// then SHA-256.
///
/// POST-PLANA BUGFIX #2: This re-computation is what the verifier uses to detect
/// payload tampering — not just chain-link matching.
pub fn compute_event_hash(event: &RunEvent) -> Result<String, serde_json::Error> {
    // Build a projection without event_sha256
    let projection = serde_json::json!({
        "schema_version": event.schema_version,
        "run_id": event.run_id,
        "event_id": event.event_id,
        "sequence": event.sequence,
        "timestamp": event.timestamp,
        "event_type": event.event_type,
        "run_manifest_sha256": event.run_manifest_sha256,
        "previous_event_sha256": event.previous_event_sha256,
        "payload_schema": event.payload_schema,
        "payload": event.payload,
        "payload_sha256": event.payload_sha256,
        "redaction": event.redaction,
    });
    let bytes = canonical_json(&projection)?;
    Ok(sha256_hex(&bytes))
}

/// Seal an event: compute payload_sha256 and event_sha256, mutating the event in place.
pub fn seal_event(event: &mut RunEvent) -> Result<(), serde_json::Error> {
    event.payload_sha256 = payload_hash(&event.payload)?;
    event.event_sha256 = compute_event_hash(event)?;
    Ok(())
}

/// Full chain validation result.
#[derive(Debug)]
pub struct ChainValidation {
    pub valid: bool,
    pub event_count: u64,
    pub last_event_sha256: Option<String>,
    pub terminal_event: Option<String>,
    pub errors: Vec<String>,
}

/// Validate a full chain of events.
///
/// Checks:
/// 1. Sequence numbers are dense and start at 0
/// 2. run_id matches on every event
/// 3. run_manifest_sha256 matches on every event
/// 4. previous_event_sha256 chain links are correct
/// 5. payload_sha256 matches on every event
/// 6. event_sha256 matches on every event (recomputed)
/// 7. Only one terminal event, at the end
///
/// POST-PLANA BUGFIX #2: event_sha256 is recomputed from canonical JSON for every event,
/// not just chain-link checked. This detects payload tampering.
pub fn validate_chain(
    events: &[RunEvent],
    expected_run_id: &str,
    expected_manifest_sha256: &str,
    require_terminal: bool,
) -> ChainValidation {
    let mut errors: Vec<String> = Vec::new();
    let mut previous: Option<&str> = None;
    let mut terminal_events: Vec<String> = Vec::new();

    for (idx, event) in events.iter().enumerate() {
        let seq = idx as u64;

        // 1. Sequence
        if event.sequence != seq {
            errors.push(format!(
                "sequence mismatch at event {idx}: expected {seq}, got {}",
                event.sequence
            ));
        }

        // 2. run_id
        if event.run_id != expected_run_id {
            errors.push(format!(
                "run_id mismatch at event {idx}: expected {expected_run_id}, got {}",
                event.run_id
            ));
        }

        // 3. manifest digest
        if event.run_manifest_sha256 != expected_manifest_sha256 {
            errors.push(format!(
                "manifest digest mismatch at event {idx}: expected {expected_manifest_sha256}, got {}",
                event.run_manifest_sha256
            ));
        }

        // 4. previous_event_sha256 chain
        match (seq, &event.previous_event_sha256) {
            (0, None) => {} // first event — correct
            (0, Some(v)) => errors.push(format!(
                "first event has non-null previous_event_sha256: {v}"
            )),
            (_, None) => errors.push(format!(
                "non-first event {idx} has null previous_event_sha256"
            )),
            (_, Some(actual)) => {
                if let Some(expected) = previous {
                    if actual != expected {
                        errors.push(format!(
                            "previous_event_sha256 mismatch at event {idx}: expected {expected}, got {actual}"
                        ));
                    }
                }
            }
        }

        // 5. payload_sha256
        match payload_hash(&event.payload) {
            Ok(computed) => {
                if event.payload_sha256 != computed {
                    errors.push(format!(
                        "payload digest mismatch at event {idx}: expected {computed}, got {}",
                        event.payload_sha256
                    ));
                }
            }
            Err(e) => {
                errors.push(format!("failed to hash payload at event {idx}: {e}"));
            }
        }

        // 6. event_sha256 (recomputed — POST-PLANA BUGFIX #2)
        match compute_event_hash(event) {
            Ok(computed) => {
                if event.event_sha256 != computed {
                    errors.push(format!(
                        "event digest mismatch at event {idx}: expected {computed}, got {}",
                        event.event_sha256
                    ));
                }
            }
            Err(e) => {
                errors.push(format!("failed to hash event at {idx}: {e}"));
            }
        }

        // Track terminal events
        if event.is_terminal() {
            terminal_events.push(event.event_type.to_string());
            if idx != events.len() - 1 {
                errors.push(format!(
                    "terminal event at index {idx} is not the last event"
                ));
            }
        }

        previous = Some(&event.event_sha256);
    }

    if require_terminal && terminal_events.len() != 1 {
        errors.push(format!(
            "expected exactly one terminal event, found {}",
            terminal_events.len()
        ));
    }
    if terminal_events.len() > 1 {
        errors.push("multiple terminal events found".into());
    }

    ChainValidation {
        valid: errors.is_empty(),
        event_count: events.len() as u64,
        last_event_sha256: previous.map(String::from),
        terminal_event: terminal_events.last().cloned(),
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::event::{EventType, Redaction};

    fn make_event(
        run_id: &str,
        seq: u64,
        event_type: EventType,
        manifest_sha256: &str,
        previous: Option<&str>,
        payload: serde_json::Value,
    ) -> RunEvent {
        let mut event = RunEvent::new(
            run_id.into(),
            seq,
            event_type,
            manifest_sha256.into(),
            previous.map(String::from),
            "test-schema".into(),
            payload,
            Redaction::None,
            "2026-08-04T00:00:00Z".into(),
        );
        seal_event(&mut event).unwrap();
        event
    }

    #[test]
    fn canonical_json_is_deterministic() {
        let a = serde_json::json!({"b": 1, "a": 2});
        let b = serde_json::json!({"a": 2, "b": 1});
        assert_eq!(canonical_json(&a).unwrap(), canonical_json(&b).unwrap());
    }

    #[test]
    fn valid_chain_passes_validation() {
        let events = vec![
            make_event(
                "RUN-TEST", 0, EventType::RunStarted,
                "abcd1234", None,
                serde_json::json!({"step": "start"}),
            ),
            make_event(
                "RUN-TEST", 1, EventType::PromptSubmitted,
                "abcd1234", Some(&"REPLACED_BY_SEAL"),
                serde_json::json!({"prompt": "hello"}),
            ),
            make_event(
                "RUN-TEST", 2, EventType::RunFinished,
                "abcd1234", Some(&"REPLACED_BY_SEAL"),
                serde_json::json!({"status": "ok"}),
            ),
        ];

        // Fix up chain links
        let mut chain: Vec<RunEvent> = Vec::new();
        let mut prev: Option<String> = None;
        for mut e in events {
            e.previous_event_sha256 = prev;
            seal_event(&mut e).unwrap();
            prev = Some(e.event_sha256.clone());
            chain.push(e);
        }

        let result = validate_chain(&chain, "RUN-TEST", "abcd1234", true);
        assert!(result.valid, "errors: {:?}", result.errors);
        assert_eq!(result.event_count, 3);
        assert!(!result.last_event_sha256.unwrap().is_empty());
    }

    #[test]
    fn tampered_payload_detected() {
        let mut events = vec![
            make_event(
                "RUN-TEST", 0, EventType::RunStarted,
                "abcd1234", None,
                serde_json::json!({"step": "start"}),
            ),
            make_event(
                "RUN-TEST", 1, EventType::RunFinished,
                "abcd1234", Some(&"PLACEHOLDER"),
                serde_json::json!({"status": "ok"}),
            ),
        ];

        // Build proper chain
        let mut prev: Option<String> = None;
        for e in &mut events {
            e.previous_event_sha256 = prev.take();
            seal_event(e).unwrap();
            prev = Some(e.event_sha256.clone());
        }

        // Tamper with payload AFTER sealing
        events[1].payload = serde_json::json!({"status": "tampered"});
        // payload_sha256 and event_sha256 are now stale

        let result = validate_chain(&events, "RUN-TEST", "abcd1234", true);
        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.contains("payload digest")),
            "should detect payload digest mismatch: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.contains("event digest")),
            "should detect event digest mismatch: {:?}",
            result.errors
        );
    }

    #[test]
    fn broken_chain_detected() {
        let events = vec![
            make_event(
                "RUN-TEST", 0, EventType::RunStarted,
                "abcd1234", None,
                serde_json::json!({"step": "start"}),
            ),
            make_event(
                "RUN-TEST", 1, EventType::RunFinished,
                "abcd1234", Some("wrong_previous_hash_64_chars___________________________"),
                serde_json::json!({"status": "ok"}),
            ),
        ];

        let result = validate_chain(&events, "RUN-TEST", "abcd1234", true);
        assert!(!result.valid);
        assert!(
            result.errors.iter().any(|e| e.contains("previous_event_sha256")),
            "should detect chain break: {:?}",
            result.errors
        );
    }

    #[test]
    fn missing_terminal_event_is_error() {
        let events = vec![
            make_event(
                "RUN-TEST", 0, EventType::RunStarted,
                "abcd1234", None,
                serde_json::json!({"step": "start"}),
            ),
        ];

        let result = validate_chain(&events, "RUN-TEST", "abcd1234", true);
        assert!(!result.valid);
    }
}
