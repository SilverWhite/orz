//! Retrieval mode surface — `RetrievalMode` / `RetrievalCapability` —
//! batch N4 of the controller split second round
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use serde::{Deserialize, Serialize};

/// GAP-RETRIEVAL-TOOLS (2026-08-10) — ADR-0010 §3.7.1: the explicit
/// session/task-contract retrieval mode. `off` is the unauthenticated
/// default; `local_browser` is the preferred enabled mode; `framework_fallback`
/// may only be entered by explicit user / parent-task-contract selection.
/// Mode changes are NEVER implicit — a failure (timeout/login/CAPTCHA) must
/// surface explicitly, not switch modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalMode {
    Off,
    LocalBrowser,
    FrameworkFallback,
}

impl RetrievalMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            RetrievalMode::Off => "off",
            RetrievalMode::LocalBrowser => "local_browser",
            RetrievalMode::FrameworkFallback => "framework_fallback",
        }
    }

    /// Parse the session-level mode from its wire form (ACP session/new).
    pub fn from_wire(value: Option<&str>) -> Option<RetrievalMode> {
        match value {
            Some("local_browser") => Some(RetrievalMode::LocalBrowser),
            Some("framework_fallback") => Some(RetrievalMode::FrameworkFallback),
            Some("off") => Some(RetrievalMode::Off),
            _ => None,
        }
    }
}

/// GAP-RETRIEVAL-TOOLS: the capability probe result for the selected mode.
/// Never a silent fallback — `Unsupported`/`Degraded` record WHY a mode
/// cannot serve (e.g. local_browser automation not implemented in this slice;
/// web client not configured). `Available` is constructed by the web client
/// probe once a client is configured (S5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetrievalCapability {
    #[allow(dead_code)] // constructed by the S5 web-client probe
    Available,
    Unsupported(String),
    #[allow(dead_code)] // reserved for degraded transports (S5)
    Degraded(String),
}

impl RetrievalCapability {
    /// The `capability_status` value for the mode-transition payload.
    pub(crate) fn status_str(&self) -> &'static str {
        match self {
            RetrievalCapability::Available => "available",
            RetrievalCapability::Unsupported(_) => "unsupported",
            RetrievalCapability::Degraded(_) => "degraded",
        }
    }
}
