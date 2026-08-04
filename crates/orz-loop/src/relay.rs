//! MechanicalRelay — pure function.name string-match router.
//!
//! No regex, no text parsing, no LLM involvement.
//! Each tool call's function.name is matched exactly to a dispatch target.
//!
//! Design principle (from D project lessons): structured function calling
//! beats text-pattern matching. Swarm_fetch (function call) was more reliable
//! than NEED regex matching.
//!
//! Phase 2 (2026-08-04): dual-model distribution (plan_* → Pro, read_* →
//! Flash) is NOT implemented — the Pro/Flash dual-model design was archived
//! (see INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §4.5). Only retrieval
//! routing remains: retrieval-shaped calls go to the subagents, everything
//! else stays on the main agent (Host).

/// Which agent should handle this tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchTarget {
    InternalRetrieval,
    ExternalRetrieval,
    Host,
}

/// Match the external retrieval tool family: bare `web_search`/`web_fetch`
/// (Grok tool names) or any `web_search_*` / `web_fetch_*` variant.
fn is_web_retrieval_tool(name: &str) -> bool {
    (name == "web_search" || name.starts_with("web_search_"))
        || (name == "web_fetch" || name.starts_with("web_fetch_"))
}

/// Route a function name to the appropriate dispatch target.
///
/// - `retrieve_project_*` → InternalRetrieval
/// - `web_search` / `web_fetch` (and `_*` variants) → ExternalRetrieval
/// - everything else → Host (main agent)
pub fn route(function_name: &str) -> DispatchTarget {
    if function_name.starts_with("retrieve_project_") {
        DispatchTarget::InternalRetrieval
    } else if is_web_retrieval_tool(function_name) {
        DispatchTarget::ExternalRetrieval
    } else {
        DispatchTarget::Host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_retrieval_routes() {
        assert_eq!(route("retrieve_project_docs"), DispatchTarget::InternalRetrieval);
        assert_eq!(
            route("retrieve_project_source_ledger"),
            DispatchTarget::InternalRetrieval
        );
    }

    #[test]
    fn external_retrieval_routes() {
        assert_eq!(route("web_search"), DispatchTarget::ExternalRetrieval);
        assert_eq!(route("web_fetch"), DispatchTarget::ExternalRetrieval);
        assert_eq!(
            route("web_search_arxiv_paper"),
            DispatchTarget::ExternalRetrieval
        );
    }

    #[test]
    fn unknown_routes_to_host() {
        assert_eq!(route("bash"), DispatchTarget::Host);
        assert_eq!(route("read_file"), DispatchTarget::Host);
        assert_eq!(route("unknown_tool"), DispatchTarget::Host);
        // Prefix boundary: bare "retrieve_project" without underscore → Host.
        assert_eq!(route("retrieve_project"), DispatchTarget::Host);
    }

    #[test]
    fn non_retrieval_matches_stay_on_host() {
        // Phase 1's all-to-host behavior holds for every non-retrieval name.
        for name in ["plan_task", "analyze_results", "read_file", "write_file", "bash"] {
            assert_eq!(route(name), DispatchTarget::Host, "{name}");
        }
    }
}
