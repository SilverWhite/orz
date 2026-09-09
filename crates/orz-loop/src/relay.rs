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
    /// GAP-SUBAGENT-RUNTIME (2026-08-10, M4): the parent's structured
    /// disposition control tool (`retrieval_disposition`) — main-lane only
    /// (the subagent's tool projection strips it).
    ParentDisposition,
    Host,
}

/// Match the external retrieval tool family: bare `web_search`/`web_fetch`
/// (Grok tool names) or any `web_search_*` / `web_fetch_*` variant.
pub(crate) fn is_web_retrieval_tool(name: &str) -> bool {
    (name == "web_search" || name.starts_with("web_search_"))
        || (name == "web_fetch" || name.starts_with("web_fetch_"))
}

/// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): the web_fetch family —
/// bare `web_fetch` and every `web_fetch_*` variant. The candidate count
/// gate keys on this family (web_search produces candidates; web_fetch
/// consumes them).
pub(crate) fn is_web_fetch_tool(name: &str) -> bool {
    name == "web_fetch" || name.starts_with("web_fetch_")
}

/// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): the candidate-counted tool
/// family — web_fetch consumes a candidate in framework_fallback, and
/// `browser_read` (local_browser second segment) consumes a candidate from
/// the SAME per-activation count domain (design §1.1/§1.3 — one budget for
/// candidate verification regardless of the channel). `pdf_read` is the PDF
/// evidence store, not candidate verification, and stays outside.
pub(crate) fn is_candidate_counted_tool(name: &str) -> bool {
    is_web_fetch_tool(name) || name == "browser_read"
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the retrieval dispatch family —
/// internal (`retrieve_project_*`) and external (`web_search*`/`web_fetch*`)
/// tool names. 0t (2026-09-09, ADR-0010 §14.65): 独立检索启用门用它把
/// 未启用会话的检索族从声明面剔除（fail-closed）；relay 在派发时拒绝同名
/// 调用（belt and braces）。
pub fn is_retrieval_dispatch_name(name: &str) -> bool {
    name.starts_with("retrieve_project_") || is_web_retrieval_tool(name)
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10, H1 review): the host-routed retrieval
/// tool family. These are retrieval tools even though they do not go
/// through the subagent dispatch lanes, so the 0t enable-gate projection and
/// the dispatch gate cover them too (ADR-0010 §14.65 — 未启用会话无检索
/// 工具；模型不得经这些名字绕过门)。local_browser (2026-08-10):
/// `browser_read` joins the family — 0t 后声明由启用门置位、浏览器缺席由
/// 调用期懒启动兜底（不再是 mode 互斥）。PDF evidence (2026-08-11):
/// `pdf_read` reads externally-sourced evidence — gated like the other
/// retrieval tools. S2-R P3 / P1-2b (2026-09-09): `browser_control` 同属
/// 本地浏览器车道（导航级动作），检索启用门族一并覆盖。
pub fn is_retrieval_mode_gated_host_tool(name: &str) -> bool {
    name == "project_doc_index"
        || name == "browser_read"
        || name == "browser_control"
        || name == "pdf_read"
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
    } else if function_name == "retrieval_disposition" {
        DispatchTarget::ParentDisposition
    } else {
        DispatchTarget::Host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_retrieval_routes() {
        assert_eq!(
            route("retrieve_project_docs"),
            DispatchTarget::InternalRetrieval
        );
        assert_eq!(
            route("retrieve_project_source_ledger"),
            DispatchTarget::InternalRetrieval
        );
    }

    #[test]
    fn external_retrieval_routes() {
        assert_eq!(route("web_search"), DispatchTarget::ExternalRetrieval);
        assert_eq!(route("web_fetch"), DispatchTarget::ExternalRetrieval);
        assert_eq!(route("web_fetch_page"), DispatchTarget::ExternalRetrieval);
        assert_eq!(
            route("web_search_arxiv_paper"),
            DispatchTarget::ExternalRetrieval
        );
    }

    #[test]
    fn web_fetch_family_boundary() {
        for name in [
            "web_fetch",
            "web_fetch_page",
            "web_fetch_pdf",
            "web_fetch_anything",
        ] {
            assert!(is_web_fetch_tool(name), "{name}");
        }
        // Prefix boundary: bare "web_fetch" without underscore is the only
        // exact match; web_search and unrelated names never count.
        for name in [
            "web_search",
            "web_search_arxiv_paper",
            "web_fetching",
            "read_file",
        ] {
            assert!(!is_web_fetch_tool(name), "{name}");
        }
    }

    #[test]
    fn candidate_counted_family_covers_web_fetch_and_browser_read() {
        for name in [
            "web_fetch",
            "web_fetch_page",
            "web_fetch_pdf",
            "browser_read",
        ] {
            assert!(is_candidate_counted_tool(name), "{name}");
        }
        for name in [
            "web_search",
            "web_search_arxiv_paper",
            "pdf_read",
            "project_doc_index",
            "read_file",
        ] {
            assert!(!is_candidate_counted_tool(name), "{name}");
        }
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
    fn parent_disposition_routes() {
        assert_eq!(
            route("retrieval_disposition"),
            DispatchTarget::ParentDisposition
        );
    }

    /// local_browser (2026-08-10): the host-routed retrieval family covers
    /// `project_doc_index` AND `browser_read` (mode=off projection + the
    /// dispatch gate both key on it — a model must not bypass the gate by
    /// switching names). PDF evidence (2026-08-11): `pdf_read` joins the
    /// family — it reads externally-sourced local evidence. These tools
    /// route to Host (not a subagent dispatch lane).
    #[test]
    fn gated_host_tools_include_browser_read() {
        assert!(is_retrieval_mode_gated_host_tool("project_doc_index"));
        assert!(is_retrieval_mode_gated_host_tool("browser_read"));
        assert!(is_retrieval_mode_gated_host_tool("pdf_read"));
        for name in [
            "web_search",
            "web_fetch",
            "retrieve_project_docs",
            "read_file",
            "bash",
        ] {
            assert!(!is_retrieval_mode_gated_host_tool(name), "{name}");
        }
        // browser_read is a Host lane tool (not a subagent dispatch lane).
        assert_eq!(route("browser_read"), DispatchTarget::Host);
        assert_eq!(route("pdf_read"), DispatchTarget::Host);
    }

    #[test]
    fn non_retrieval_matches_stay_on_host() {
        // Phase 1's all-to-host behavior holds for every non-retrieval name.
        for name in [
            "plan_task",
            "analyze_results",
            "read_file",
            "write_file",
            "bash",
        ] {
            assert_eq!(route(name), DispatchTarget::Host, "{name}");
        }
    }
}
