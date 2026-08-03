//! ToolDispatcher — wraps tool execution with assurance checks.
//!
//! IP3a: Pre-execution IPG (instruction provenance gate)
//! IP3b: Post-execution orientation counters
//! IP3c: Post-retrieval sufficiency trigger
//!
//! Phase 1: skeleton only. Full implementation in Step 3.

/// Dispatches tool calls with pre/post assurance checks.
pub struct ToolDispatcher;

impl ToolDispatcher {
    pub fn new() -> Self {
        ToolDispatcher
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
