//! Journal event types — the data model for append-only hash-chained events.
//!
//! Mirrors `runtime/run-event-v0.1.schema.json` and (for the v0.2 track)
//! `runtime/run-event-v0.2.schema.json` from the Python reference spec.

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

    // Inquiry gates (Phase 3, §4.6 wiring — mirrors run-event schema).
    // `NeutralInquiry` and `RetrievalCompletionCheck` are RETIRED on the v0.2
    // track (ADR-0010 §5.1/§11.2 — the mixed-counter inquiry mechanism is
    // gone, GAP-INQUIRY-SPLIT); the variants stay for v0.1 journal replay.
    // The v0.2 producer must never write them (EventWriter asserts).
    NeutralInquiry,
    CounterexampleGate,
    RetrievalCompletionCheck,

    // GAP-INQUIRY-SPLIT (2026-08-09): v0.2 mechanism events (ADR-0010 §5.1 —
    // five independent event types; two more were already present above).
    DiagnosticCoverageCheckpoint,
    InformationSufficiencyAssessment,
    RetrievalParentDisposition,
    RetrievalCloseRecord,

    // GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode authority (§3.7.1),
    // structured result commit (§3.3.3) and cross-run activation restore
    // (§3.3) — all three are non-terminal v0.2 mechanism events.
    RetrievalModeTransition,
    RetrievalResultCommitted,
    RetrievalActivationRestored,

    // A6 explicit context compaction (2026-08-08 — controller-written;
    // mirrors run-event schema)
    ContextCompressed,

    // IP5 pre-mutation snapshot (Phase 3, slice #4 wiring)
    SnapshotCreated,
    /// IP5 restore/revert (Phase 3, slice #8 — host restore entry; a restore
    /// is its own run: preflight → snapshot_restored → terminal).
    SnapshotRestored,

    // Artifact
    ArtifactRegistered,

    // Plan mode (Phase 2, SLICE-10)
    PlanProposed,
    PlanApproved,
    PlanRejected,
    ActionApproved,
}

impl EventType {
    /// Whether this event type terminates the journal.
    pub fn is_terminal(&self) -> bool {
        TERMINAL_EVENTS.contains(self)
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_else(|_| "unknown".into());
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

/// Journal track — the envelope schema family a producer writes
/// (GAP-INQUIRY-SPLIT, 2026-08-09).
///
/// A journal must be homogeneous on ONE track (§11.6.2 — mixed envelope
/// versions break the hash-chain contract): the track pins both the envelope
/// `schema_version` and the `payload_schema` identifier for every event.
/// `V02` is the production track; `V01` is the historical freeze, used only
/// by replay tests and legacy fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTrack {
    V01,
    V02,
}

impl EventTrack {
    /// The `payload_schema` value stamped on every event of this track.
    pub fn payload_schema_id(self) -> &'static str {
        match self {
            EventTrack::V01 => "run-event-v0.1.schema.json",
            EventTrack::V02 => "run-event-v0.2.schema.json",
        }
    }

    /// The envelope `schema_version` for this track.
    pub fn schema_version(self) -> &'static str {
        match self {
            EventTrack::V01 => "0.1.0-draft",
            EventTrack::V02 => "0.2.0-draft",
        }
    }
}

/// A single append-only event in the run journal.
///
/// Each event carries a hash chain link (`previous_event_sha256`) and its own
/// content hash (`event_sha256`). The chain is verified by `JournalVerifier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    /// Schema version — "0.1.0-draft" (v0.1 track) or "0.2.0-draft" (v0.2
    /// track; GAP-INQUIRY-SPLIT — producers write the v0.2 track so the
    /// hash chain stays homogeneous, §11.6.2).
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
    /// Serialized as an explicit `null` for seq=0 — the run-event schema
    /// requires this key on every line (required array + allOf null rule).
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
    /// The caller MUST call `seal_event()` (or `JournalRecorder::record()`,
    /// which seals automatically) before recording.
    ///
    /// `schema_version` selects the track ("0.1.0-draft" / "0.2.0-draft");
    /// the convenience constructors below pin it.
    fn new(
        schema_version: String,
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
            // Strip the run-kind prefix ("RUN-" prompts, "RST-" restore
            // runs — slice #8) to get the suffix for the event ID. `{:03}`
            // matches the Python authority (canonical_cli.py uses
            // `{sequence:03d}`) so event hashes align across sides.
            let suffix = run_id
                .strip_prefix("RUN-")
                .or_else(|| run_id.strip_prefix("RST-"))
                .unwrap_or(&run_id);
            format!("EVT-{suffix}-{sequence:03}")
        };

        RunEvent {
            schema_version,
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

    /// v0.1 track constructor — legacy producers and replay tests only
    /// (the v0.1 schema is a historical freeze; the v0.2 producer must not
    /// write it).
    pub fn new_v01(
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
        Self::new(
            "0.1.0-draft".into(),
            run_id,
            sequence,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            redaction,
            timestamp,
        )
    }

    /// v0.2 track constructor (GAP-INQUIRY-SPLIT, 2026-08-09): production
    /// producers write this track — `schema_version = "0.2.0-draft"` and
    /// `payload_schema = "run-event-v0.2.schema.json"` (set by the caller /
    /// `EventWriter::V02`).
    pub fn new_v02(
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
        Self::new(
            "0.2.0-draft".into(),
            run_id,
            sequence,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            redaction,
            timestamp,
        )
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
        let event = RunEvent::new_v01(
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
        assert_eq!(event.event_id, "EVT-A1B2C3D4-000");
    }
}
