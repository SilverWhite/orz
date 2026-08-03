//! Journal event types — the data model for append-only hash-chained events.
//!
//! Mirrors `runtime/run-event-v0.1.schema.json` from the Python reference spec.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Terminal event types that close a journal (no further events permitted).
pub const TERMINAL_EVENTS: &[EventType] = &[
    EventType::RunFinished,
    EventType::RunFailed,
    EventType::RunCancelled,
    EventType::RunInvalidated,
];

/// All event types in the run lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Lifecycle
    RunPreflight,
    RunStarted,
    RunFinished,
    RunFailed,
    RunCancelled,
    RunInvalidated,

    // Prompt/model
    PromptSubmitted,
    ModelRequest,
    ModelResponseReceived,
    ModelOutput,

    // ACP lifecycle
    AcpInitialize,
    AcpSessionCreated,

    // Tool
    ToolProposal,
    PermissionRequested,
    PermissionDecision,
    ToolStarted,
    ToolCompleted,

    // Assurance
    OrientationCheckpoint,
    RuntimeStagnationGuard,
    ToolAvailabilityCheck,
    ToolBeliefStagnation,
    InstructionProvenanceGate,
    GateDecision,

    // Artifact
    ArtifactRegistered,
}

impl EventType {
    /// Whether this event type terminates the journal.
    pub fn is_terminal(&self) -> bool {
        TERMINAL_EVENTS.contains(self)
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self)
            .unwrap_or_else(|_| "unknown".into());
        // Strip JSON quotes
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// Redaction level for payload content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redaction {
    None,
    MetadataOnly,
    ContentRedacted,
}

/// A single append-only event in the run journal.
///
/// Each event carries a hash chain link (`previous_event_sha256`) and its own
/// content hash (`event_sha256`). The chain is verified by `JournalVerifier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    /// Schema version — always "0.1.0-draft"
    pub schema_version: String,

    /// Unique run identifier, e.g. "RUN-{uuid}"
    pub run_id: String,

    /// Unique event identifier, e.g. "EVT-{run_id_suffix}-{seq:06}"
    pub event_id: String,

    /// Monotonic sequence number starting at 0.
    pub sequence: u64,

    /// ISO 8601 timestamp (UTC).
    pub timestamp: String,

    /// What happened.
    pub event_type: EventType,

    /// SHA-256 of the canonical run manifest JSON.
    pub run_manifest_sha256: String,

    /// SHA-256 of the previous event, or null for seq=0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_event_sha256: Option<String>,

    /// Schema identifier for the payload.
    pub payload_schema: String,

    /// Arbitrary structured payload.
    pub payload: Value,

    /// SHA-256 of the canonical payload JSON.
    pub payload_sha256: String,

    /// Redaction level for this event.
    pub redaction: Redaction,

    /// SHA-256 of this event's canonical JSON (minus this field).
    /// Computed AFTER all other fields are set.
    pub event_sha256: String,
}

impl RunEvent {
    /// Create a new event with fields filled from context.
    /// The caller MUST call `seal()` or set `event_sha256` before recording.
    pub fn new(
        run_id: String,
        sequence: u64,
        event_type: EventType,
        run_manifest_sha256: String,
        previous_event_sha256: Option<String>,
        payload_schema: String,
        payload: Value,
        redaction: Redaction,
        timestamp: String,
    ) -> Self {
        let event_id = {
            // Strip "RUN-" prefix to get the suffix for the event ID.
            let suffix = run_id.strip_prefix("RUN-").unwrap_or(&run_id);
            format!("EVT-{suffix}-{sequence:06}")
        };

        RunEvent {
            schema_version: "0.1.0-draft".into(),
            run_id,
            event_id,
            sequence,
            timestamp,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            payload_sha256: String::new(), // filled by seal()
            redaction,
            event_sha256: String::new(), // filled by seal()
        }
    }

    /// Returns true if this event terminates the journal.
    pub fn is_terminal(&self) -> bool {
        self.event_type.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_events_block_further_appends() {
        assert!(EventType::RunFinished.is_terminal());
        assert!(EventType::RunFailed.is_terminal());
        assert!(EventType::RunCancelled.is_terminal());
        assert!(!EventType::RunStarted.is_terminal());
        assert!(!EventType::PromptSubmitted.is_terminal());
    }

    #[test]
    fn event_id_format() {
        let event = RunEvent::new(
            "RUN-A1B2C3D4".into(),
            0,
            EventType::RunStarted,
            "abcd1234".into(),
            None,
            "test-schema".into(),
            serde_json::json!({}),
            Redaction::None,
            "2026-08-04T00:00:00Z".into(),
        );
        assert_eq!(event.event_id, "EVT-A1B2C3D4-000000");
    }
}
