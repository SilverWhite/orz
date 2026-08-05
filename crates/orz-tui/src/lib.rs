// orz-tui — assurance workbench TUI for the orz CLI agent.
//
// Ported from Python assurance/tui/ (spec reference). Displays gate status,
// journal projection, orientation checkpoints, tool traces, permissions and
// plan events as a retro desktop-style workbench (frozen layout from
// SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1).
//
// Phase 3 slice #5 (core workbench v1): in-process ACP client against
// orz-host + live journal tail projection + replay mode.

pub mod acp_client;
pub mod app;
pub mod bridge;
pub mod commands;
pub mod dialogs;
pub mod events;
pub mod explorer;
pub mod input;
pub mod journal_tail;
pub mod modals;
pub mod projection;
pub mod runner;
pub mod snapshot;
pub mod snapshots;
pub mod source;
pub mod theme;
pub mod title;
pub mod view_model;
pub mod widgets;

pub use runner::{run, run_replay, replay_to_screen, TuiConfig, TuiError};

/// Single source of truth for the permission-dialog countdown: the host
/// denies at this timeout, so the TUI's dialog shows the same deadline
/// (Phase 3 slice #7 — Feature B).
pub use orz_host::permission::PERMISSION_PROMPT_TIMEOUT;
