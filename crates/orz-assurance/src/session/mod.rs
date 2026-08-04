//! Session state management — pre-mutation snapshots (IP5).
//!
//! Ported from the fusion architecture design (INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN
//! v0.2 §4 IP5 + FORK_IMPLEMENTATION_DESIGN v0.1 §4.6): before any file-mutating
//! tool executes, the loop records a snapshot of the target files so the session
//! can be rolled back to the pre-mutation state on user request.
//!
//! Implementation note (2026-08-05): the v0.1 design specified a git-based store
//! (`git init` + alternates + `write-tree`). This slice ships a **content-addressed
//! store** instead (objects keyed by SHA-256, manifest per snapshot, no git
//! subprocess) — same contract (track → hash handle → restore/revert/verify),
//! deterministic on Windows, no external process in the assurance hot path.
//! A git-backed store remains an option if diff/merge semantics are ever needed.

pub mod snapshot;

pub use snapshot::{SnapshotError, SnapshotRecord, SnapshotRestoreOutcome, SnapshotStore, SnapshotVerifyOutcome};
