//! MechanicalRelay — pure function.name string-match router.
//!
//! No regex, no text parsing, no LLM involvement.
//! Each tool call's function.name is matched exactly to a dispatch target.
//!
//! Design principle (from D project lessons): structured function calling
//! beats text-pattern matching. Swarm_fetch (function call) was more reliable
//! than NEED regex matching.

/// Which agent should handle this tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchTarget {
    Pro,
    Flash,
    InternalRetrieval,
    ExternalRetrieval,
    Host,
}

/// Route a function name to the appropriate dispatch target.
///
/// Phase 1: route everything to Host (single-agent mode).
/// Phase 2+: distinguish Pro/Flash/Retrieval targets by function name prefix.
pub fn route(function_name: &str) -> DispatchTarget {
    // Phase 2 routing patterns (commented for Phase 1):
    // - "plan_*" / "analyze_*" / "design_*" → Pro
    // - "read_*" / "write_*" / "bash_*" / "search_*" → Flash
    // - "retrieve_project_*" → InternalRetrieval
    // - "web_search_*" / "web_fetch_*" → ExternalRetrieval
    // - everything else → Host

    // Phase 1: single-agent mode — all tool calls go through Host
    let _ = function_name;
    DispatchTarget::Host
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase1_all_routes_to_host() {
        assert_eq!(route("bash"), DispatchTarget::Host);
        assert_eq!(route("read_file"), DispatchTarget::Host);
        assert_eq!(route("web_search"), DispatchTarget::Host);
        assert_eq!(route("unknown_tool"), DispatchTarget::Host);
    }
}
