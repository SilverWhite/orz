//! Face B per-tool mechanical probes (P0-A batch, step 1).
//!
//! Design: TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md §2/§3 — the main
//! agent's working tools split into three surfaces; Face B tools are probed
//! with a two-state neutral verdict:
//!
//! - `Complete` — the tool's whole mechanical chain is present;
//! - `Incomplete(reason)` — any chain component is missing, with a stable
//!   neutral reason that is also the audit key.
//!
//! Invariants:
//! - The model-visible surface only ever carries tool names; reasons are
//!   neutral statements and never availability judgment words
//!   (`可用/不可用/成功/失败/available/...`).
//! - A probe is a snapshot of the mechanical chain at probe time, never a
//!   call-success promise; the call-time permission gate remains the final
//!   backstop (design invariant 2).
//! - Missing probe implementation → `Incomplete("未完成链路检查")`
//!   (fail-closed; never assume complete).
//!
//! Step-1 scope (BACKLOG P0-A #1): the per-tool probe implementations and
//! their unit tests only. Per-action refresh, minimal previous-round
//! mapping, list projection, `tool_availability_check` event upgrade and
//! the run_tests migration are later batch steps.

use std::path::{Path, PathBuf};

use crate::host::ToolPolicy;

/// Face B — locally deterministic tools probed before every action.
/// Order is the canonical projection order (stable across snapshots).
pub const FACE_B_TOOLS: [&str; 7] = [
    "read_file",
    "list_dir",
    "grep",
    "search_tool",
    "search_replace",
    "run_tests",
    "ask_user_question",
];

/// Neutral, stable reasons — machine-readable audit keys, never
/// availability judgment words.
pub const REASON_WORKSPACE_UNREADABLE: &str = "工作区路径不可读";
pub const REASON_WORKSPACE_UNWRITABLE: &str = "工作区路径不可写";
pub const REASON_WRITE_POLICY_NOT_ALLOWED: &str = "写权限策略未放行";
pub const REASON_MISSING_TEST_RUNNER: &str = "缺少测试运行器";
pub const REASON_NO_INTERACTIVE_USER: &str = "无交互式用户会话";
pub const REASON_UNFINISHED_CHAIN_CHECK: &str = "未完成链路检查";

/// Two-state neutral probe verdict (design §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeVerdict {
    /// The tool's mechanical chain is whole (probe-time snapshot only).
    Complete,
    /// A chain component is missing; carries the neutral reason.
    Incomplete(&'static str),
}

impl ProbeVerdict {
    pub fn is_complete(&self) -> bool {
        matches!(self, ProbeVerdict::Complete)
    }

    /// The neutral reason, or `None` for `Complete`.
    pub fn reason(&self) -> Option<&'static str> {
        match self {
            ProbeVerdict::Complete => None,
            ProbeVerdict::Incomplete(reason) => Some(reason),
        }
    }
}

/// One tool's probe result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolProbeResult {
    pub tool: String,
    pub verdict: ProbeVerdict,
}

/// One incomplete Face B tool with its neutral reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFailure {
    pub tool: String,
    pub reason: &'static str,
}

/// Per-action Face B snapshot (design §4: used for the round, then
/// discarded — never persisted, never across runs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolProbeSnapshot {
    pub complete: Vec<String>,
    pub incomplete: Vec<ProbeFailure>,
}

/// Mechanical chain inputs a probe can see at probe time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeContext {
    /// Session working directory — the workspace scope probed for the
    /// read/write chain.
    pub cwd: PathBuf,
    /// Session permission policy — the write-policy half of the
    /// `search_replace` probe (`ReadOnly` refuses writes by policy).
    pub policy: ToolPolicy,
    /// Whether the host carries a fixed test runner
    /// (`host.test_runner().is_some()`) — the `run_tests` probe source.
    pub test_runner_present: bool,
    /// Whether the session is attached to an interactive user who can
    /// answer `ask_user_question` (headless sessions fail closed).
    pub interactive_user: bool,
}

/// Whether `name` belongs to Face B (the only probed surface).
pub fn is_face_b_tool(name: &str) -> bool {
    FACE_B_TOOLS.contains(&name)
}

/// Policy half of the write probe: `ReadOnly` never passes; `Interactive`
/// and `Benchmark` pass at policy level (the call-time permission gate
/// still makes the final decision per arguments).
pub fn policy_allows_write(policy: ToolPolicy) -> bool {
    policy != ToolPolicy::ReadOnly
}

/// Workspace read chain: the session cwd exists, is a directory and yields
/// a directory listing (mechanical, no reads of file contents).
pub fn workspace_readable(cwd: &Path) -> bool {
    cwd.is_dir() && std::fs::read_dir(cwd).is_ok()
}

/// Workspace write chain: readable plus not flagged read-only by the
/// filesystem metadata. Metadata-grade by design — ACLs and per-target
/// write permission are the call-time gate's job, not the probe's.
pub fn workspace_writable(cwd: &Path) -> bool {
    if !workspace_readable(cwd) {
        return false;
    }
    match std::fs::metadata(cwd) {
        Ok(meta) => !meta.permissions().readonly(),
        Err(_) => false,
    }
}

fn probe_read(ctx: &ProbeContext) -> ProbeVerdict {
    if workspace_readable(&ctx.cwd) {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_WORKSPACE_UNREADABLE)
    }
}

fn probe_write(ctx: &ProbeContext) -> ProbeVerdict {
    if !policy_allows_write(ctx.policy) {
        return ProbeVerdict::Incomplete(REASON_WRITE_POLICY_NOT_ALLOWED);
    }
    if workspace_writable(&ctx.cwd) {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_WORKSPACE_UNWRITABLE)
    }
}

fn probe_test_runner(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.test_runner_present {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_MISSING_TEST_RUNNER)
    }
}

fn probe_interactive_user(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.interactive_user {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_NO_INTERACTIVE_USER)
    }
}

/// Probe one tool name against the mechanical chain. Unknown tools (no
/// probe implementation) fail closed with `未完成链路检查`.
pub fn probe_tool(name: &str, ctx: &ProbeContext) -> ToolProbeResult {
    let verdict = match name {
        "read_file" | "list_dir" | "grep" | "search_tool" => probe_read(ctx),
        "search_replace" => probe_write(ctx),
        "run_tests" => probe_test_runner(ctx),
        "ask_user_question" => probe_interactive_user(ctx),
        _ => ProbeVerdict::Incomplete(REASON_UNFINISHED_CHAIN_CHECK),
    };
    ToolProbeResult {
        tool: name.to_string(),
        verdict,
    }
}

/// Probe every Face B tool in canonical order into one snapshot.
pub fn probe_face_b(ctx: &ProbeContext) -> ToolProbeSnapshot {
    let mut snapshot = ToolProbeSnapshot::default();
    for tool in FACE_B_TOOLS {
        match probe_tool(tool, ctx).verdict {
            ProbeVerdict::Complete => snapshot.complete.push(tool.to_string()),
            ProbeVerdict::Incomplete(reason) => snapshot.incomplete.push(ProbeFailure {
                tool: tool.to_string(),
                reason,
            }),
        }
    }
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test-only temp directory with deterministic cleanup (no new
    /// third-party dependency).
    struct TestDir(PathBuf);

    impl TestDir {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "orz-tool-probe-{tag}-{}-{nanos}",
                std::process::id()
            ));
            std::fs::create_dir_all(&dir).expect("create probe test dir");
            Self(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn full_ctx(dir: &Path) -> ProbeContext {
        ProbeContext {
            cwd: dir.to_path_buf(),
            policy: ToolPolicy::Interactive,
            test_runner_present: true,
            interactive_user: true,
        }
    }

    fn missing_ctx() -> ProbeContext {
        ProbeContext {
            cwd: std::env::temp_dir().join("orz-tool-probe-missing-dir"),
            policy: ToolPolicy::Interactive,
            test_runner_present: true,
            interactive_user: true,
        }
    }

    #[test]
    fn face_b_membership_is_exact() {
        for tool in FACE_B_TOOLS {
            assert!(is_face_b_tool(tool), "{tool} must be Face B");
        }
        for tool in [
            "blackboard_read",
            "run_terminal_cmd",
            "web_search",
            "web_fetch",
            "browser_read",
            "pdf_read",
            "project_doc_index",
            "image_gen",
            "use_tool",
            "compaction_whitelist_add",
            "retrieval_disposition",
        ] {
            assert!(!is_face_b_tool(tool), "{tool} must not be Face B");
        }
    }

    #[test]
    fn full_chain_marks_all_face_b_complete() {
        let dir = TestDir::new("full");
        let snapshot = probe_face_b(&full_ctx(dir.path()));
        assert_eq!(
            snapshot.complete,
            FACE_B_TOOLS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        );
        assert!(snapshot.incomplete.is_empty(), "{:?}", snapshot.incomplete);
    }

    #[test]
    fn missing_workspace_breaks_read_tools() {
        let ctx = missing_ctx();
        for tool in ["read_file", "list_dir", "grep", "search_tool"] {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Incomplete(REASON_WORKSPACE_UNREADABLE),
                "{tool}"
            );
        }
    }

    #[test]
    fn missing_workspace_breaks_write_tool() {
        let ctx = missing_ctx();
        assert_eq!(
            probe_tool("search_replace", &ctx).verdict,
            ProbeVerdict::Incomplete(REASON_WORKSPACE_UNWRITABLE)
        );
    }

    #[test]
    fn readonly_policy_breaks_write_tool_even_with_readable_workspace() {
        let dir = TestDir::new("policy");
        let ctx = ProbeContext {
            policy: ToolPolicy::ReadOnly,
            ..full_ctx(dir.path())
        };
        assert_eq!(
            probe_tool("search_replace", &ctx).verdict,
            ProbeVerdict::Incomplete(REASON_WRITE_POLICY_NOT_ALLOWED)
        );
    }

    #[test]
    fn policy_allows_write_mapping() {
        assert!(policy_allows_write(ToolPolicy::Interactive));
        assert!(policy_allows_write(ToolPolicy::Benchmark));
        assert!(!policy_allows_write(ToolPolicy::ReadOnly));
    }

    #[test]
    fn run_tests_requires_runner() {
        let dir = TestDir::new("runner");
        let without = ProbeContext {
            test_runner_present: false,
            ..full_ctx(dir.path())
        };
        assert_eq!(
            probe_tool("run_tests", &without).verdict,
            ProbeVerdict::Incomplete(REASON_MISSING_TEST_RUNNER)
        );
        assert_eq!(
            probe_tool("run_tests", &full_ctx(dir.path())).verdict,
            ProbeVerdict::Complete
        );
    }

    #[test]
    fn ask_user_question_requires_interactive_user() {
        let dir = TestDir::new("interactive");
        let without = ProbeContext {
            interactive_user: false,
            ..full_ctx(dir.path())
        };
        assert_eq!(
            probe_tool("ask_user_question", &without).verdict,
            ProbeVerdict::Incomplete(REASON_NO_INTERACTIVE_USER)
        );
        assert_eq!(
            probe_tool("ask_user_question", &full_ctx(dir.path())).verdict,
            ProbeVerdict::Complete
        );
    }

    #[test]
    fn unknown_tool_fails_closed() {
        let dir = TestDir::new("unknown");
        let ctx = full_ctx(dir.path());
        for tool in ["image_edit", "image_gen", "use_tool", "lsp", "no_such_tool"] {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Incomplete(REASON_UNFINISHED_CHAIN_CHECK),
                "{tool}"
            );
        }
    }

    #[test]
    fn snapshot_partitions_complete_and_incomplete() {
        let dir = TestDir::new("partition");
        let ctx = ProbeContext {
            policy: ToolPolicy::ReadOnly,
            test_runner_present: false,
            interactive_user: false,
            ..full_ctx(dir.path())
        };
        let snapshot = probe_face_b(&ctx);
        assert_eq!(
            snapshot.complete,
            vec![
                "read_file".to_string(),
                "list_dir".to_string(),
                "grep".to_string(),
                "search_tool".to_string(),
            ]
        );
        assert_eq!(
            snapshot.incomplete,
            vec![
                ProbeFailure {
                    tool: "search_replace".to_string(),
                    reason: REASON_WRITE_POLICY_NOT_ALLOWED,
                },
                ProbeFailure {
                    tool: "run_tests".to_string(),
                    reason: REASON_MISSING_TEST_RUNNER,
                },
                ProbeFailure {
                    tool: "ask_user_question".to_string(),
                    reason: REASON_NO_INTERACTIVE_USER,
                },
            ]
        );
    }

    #[test]
    fn workspace_readable_false_for_missing_path() {
        let missing = std::env::temp_dir().join("orz-tool-probe-definitely-missing");
        assert!(!workspace_readable(&missing));
        assert!(!workspace_writable(&missing));
    }

    #[cfg(windows)]
    #[test]
    fn workspace_writable_detects_readonly_directory() {
        let dir = TestDir::new("readonly-attr");
        let mut perms = std::fs::metadata(dir.path())
            .expect("probe test dir metadata")
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(dir.path(), perms).expect("mark probe dir read-only");
        assert!(!workspace_writable(dir.path()));
    }
}
