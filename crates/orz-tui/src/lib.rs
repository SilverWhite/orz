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
pub mod input;
pub mod journal_tail;
pub mod projection;
pub mod runner;
pub mod snapshot;
pub mod source;
pub mod theme;
pub mod view_model;
pub mod widgets;

pub use runner::{run, run_replay, replay_to_screen, TuiConfig, TuiError};
