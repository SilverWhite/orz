// orz-assurance — assurance kernel for the orz CLI agent workbench.
//
// Ported from Python assurance/ (spec reference).
// Modules: journal (Phase 1), gates, orientation, plan (Phase 2),
// session/snapshot, sandbox/job_object, credential, permit (Phase 3),
// acaf (ACAF Slice 1 — ADR-0011 control tickets).

pub mod acaf;
pub mod candidate_prefilter;
pub mod credential;
pub mod gates;
pub mod journal;
pub mod lif;
pub mod orientation;
pub mod permit;
pub mod plan;
pub mod reducer;
pub mod sandbox;
pub mod session;
pub mod source_weighting;
pub mod tool_envelope;

// Re-export commonly used types from journal
pub use journal::{
    ChainValidation, EventTrack, EventType, JournalRecorder, JournalRecorderError, Redaction,
    ReplayResult, RunEvent, TERMINAL_EVENTS, canonical_json, compute_event_hash, replay_journal,
    seal_event, sha256_hex,
};

/// Gate decision — modelled on PROTOCOL_DRAFT §9.
/// Five-valued: Pass, Warn, Block, Defer, NotApplicable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    /// Evidence sufficient, no concerns.
    Pass,
    /// Pass with reservations (e.g. incomplete coverage, not blocking).
    Warn { reason_codes: Vec<String> },
    /// Hard stop — must not proceed.
    Block { reason_codes: Vec<String> },
    /// Evidence insufficient — need more information.
    Defer { missing: Vec<String> },
    /// Gate does not apply to the current action.
    NotApplicable,
}

impl GateDecision {
    /// Canonical short name for journal payloads (GAK-02 vocabulary:
    /// pass / warn / block / defer / not_applicable).
    pub fn decision_str(&self) -> &'static str {
        match self {
            GateDecision::Pass => "pass",
            GateDecision::Warn { .. } => "warn",
            GateDecision::Block { .. } => "block",
            GateDecision::Defer { .. } => "defer",
            GateDecision::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AssuranceError {
    #[error("gate blocked: {0:?}")]
    GateBlocked(GateDecision),

    #[error("journal error: {0}")]
    Journal(#[from] std::io::Error),

    #[error("journal recorder error: {0}")]
    JournalRecorder(#[from] JournalRecorderError),

    #[error("hash chain broken at event {sequence}: expected {expected}, got {actual}")]
    HashChainBroken {
        sequence: u64,
        expected: String,
        actual: String,
    },

    #[error("invariant violation: {0}")]
    InvariantViolation(String),

    #[error("credential error: {0}")]
    Credential(String),
}
