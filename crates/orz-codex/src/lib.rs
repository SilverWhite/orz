//! orz-codex — the Codex-style fallback TUI (design §2.4, Phase 3 slice #12).
//!
//! The fallback is a lightweight Codex-style frontend over the **Codex
//! app-server JSON-RPC** surface on orz-host (the §2.4 dual-TUI strategy: ACP
//! serves orz-tui, app-server JSON-RPC serves the fallback). It shows **no
//! assurance panels** — the assurance layer runs completely and silently in
//! orz-host. One process, one view; the `orz-codex` binary is an explicit
//! fallback entry (default-off at runtime; the `orz` binary is unchanged).
//!
//! Underlying design and functionality are identical to orz: the same host
//! machinery (loop/tools/journal/gates) is driven through this protocol,
//! including the full interactive permission flow (`approval/request` →
//! `approval/response`), streaming text (`item/started`/`item/delta`/
//! `item/completed`) and cancellation (`turn/interrupt` →
//! `turn/completed{interrupted}`).

pub mod app;
pub mod client;
pub mod runner;
pub mod snapshot;
pub mod widgets;

pub use app::{CodexApp, RunState};
pub use client::{ClientMsg, CodexClient};
pub use runner::{TuiConfig, TuiError, run};
