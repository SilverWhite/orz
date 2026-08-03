// orz-assurance — assurance kernel for the orz CLI agent workbench.
//
// Ported from Python assurance/ (spec reference).
// Modules: journal (Phase 1), gates, orientation, session, sandbox, credential, permit (Phase 2+).

pub mod journal;

// Phase 2+ module declarations:
// pub mod gates;
// pub mod orientation;
// pub mod session;
// pub mod sandbox;
// pub mod credential;
// pub mod permit;

// Re-export commonly used types from journal
pub use journal::{
    canonical_json, compute_event_hash, replay_journal, seal_event, sha256_hex,
    ChainValidation, EventType, JournalRecorder, JournalRecorderError, ReplayResult, Redaction,
    RunEvent, TERMINAL_EVENTS,
};

/// Gate decision — modelled on PROTOCOL_DRAFT §9.
/// Five-valued: Pass, Warn, Block, Defer, NotApplicable.
#[derive(Debug, Clone)]
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
