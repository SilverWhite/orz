//! Append-only hash-chained JSONL journal.
//!
//! The journal is the foundational assurance primitive: every event in an agent run
//! is recorded with a SHA-256 hash chain that guarantees append-only integrity.
//!
//! # Architecture
//!
//! ```text
//! AgentLoopController
//!       │ record(RunEvent)
//!       ▼
//! JournalRecorder  ──blocking send──▶  JournalWriterTask (background)
//!   (Clone, Send)                          │
//!                                          │ JSONL append + fsync
//!                                          ▼
//!                                     events.jsonl
//! ```
//!
//! # Modules
//!
//! - `event` — RunEvent data model (matching `run-event-v0.1.schema.json`)
//! - `chain` — Hash chain logic (canonical JSON, SHA-256, chain validation)
//! - `recorder` — JournalRecorder (async-backed, blocking send, thread-safe)
//! - `verifier` — JournalVerifier (replay + integrity check)
//! - `conformance` — offline schema-level journal judge (Task D batch-1)

pub mod chain;
pub mod conformance;
pub mod event;
pub mod recorder;
pub mod verifier;

// Re-export commonly used types
pub use chain::{ChainValidation, canonical_json, compute_event_hash, seal_event, sha256_hex};
pub use conformance::{ConformanceReport, validate_journal_file};
pub use event::{EventTrack, EventType, Redaction, RunEvent, TERMINAL_EVENTS};
pub use recorder::{JournalRecorder, JournalRecorderError};
pub use verifier::{ReplayResult, replay_journal};
