//! Per-tool mechanical probes — the single probe face (v0.2; ADR-0010
//! §3.5 v1.8, 2026-08-13).
//!
//! Design: TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md §2.0/§3 — every
//! main-agent work tool is governed by the same rule: this round's visible
//! set = mechanically complete ∩ session-declared, names only, no status.
//! The v0.1 faces (A 恒声明 / B 探针过滤 / C 固定列表) are revoked; the
//! former Face B mechanism is upgraded to the full work-tool surface.
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
//! P0-A-2 (2026-08-13): single probe face expansion — the Face A/C
//! constants and fixed-list semantics are gone; `probe_work_tools`
//! partitions ALL 23 work tools. Host capability facts
//! (terminal/lsp/memory/image/video/MCP) come from `LoopHost` fail-closed
//! defaults; session facts (goal context, pending retrieval activation)
//! come from the controller.

use std::path::{Path, PathBuf};

use crate::host::ToolPolicy;

/// Every main-agent work tool in canonical (stable) projection order.
/// Single source of truth for membership, the probe snapshot partition and
/// the Python reference's work-tool set (mirrored in
/// `assurance/run_event_journal_validation.py` — frozen reference since the
/// Task D S3 flip, 2026-09-06). Since S2d 裁决二
/// (ADR-0010 §14.58/§14.59) the journaled availability accounting covers
/// the declared surface only — a subset of this list; the raw probe
/// (`probe_work_tools`) still partitions the full set.
pub const WORK_TOOLS: [&str; 23] = [
    // Locally deterministic tools (former Face B).
    "read_file",
    "list_dir",
    "grep",
    "search_tool",
    "search_replace",
    "run_tests",
    "ask_user_question",
    // Former Face A — control tools with real mechanical chains.
    "blackboard_read",
    "todo_write",
    "update_goal",
    "enter_plan_mode",
    "exit_plan_mode",
    "compaction_whitelist_add",
    "retrieval_disposition",
    // Former Face C — local-process / heavy / network tools; backend
    // presence is now probed, never assumed.
    "run_terminal_cmd",
    "lsp",
    "memory_get",
    "memory_search",
    "image_gen",
    "image_edit",
    "image_to_video",
    "reference_to_video",
    "use_tool",
];

/// Neutral, stable reasons — machine-readable audit keys, never
/// availability judgment words.
pub const REASON_WORKSPACE_UNREADABLE: &str = "工作区路径不可读";
pub const REASON_WORKSPACE_UNWRITABLE: &str = "工作区路径不可写";
pub const REASON_WRITE_POLICY_NOT_ALLOWED: &str = "写权限策略未放行";
pub const REASON_MISSING_TEST_RUNNER: &str = "缺少测试运行器";
pub const REASON_NO_INTERACTIVE_USER: &str = "无交互式用户会话";
pub const REASON_SESSION_STORAGE_UNREADABLE: &str = "会话存储不可读";
pub const REASON_NO_GOAL_CONTEXT: &str = "任务目标上下文不存在";
pub const REASON_PLAN_MODE_UNSUPPORTED: &str = "会话不支持计划模式";
pub const REASON_RETRIEVAL_NOT_ACTIVE: &str = "检索会话未激活";
pub const REASON_TERMINAL_CHAIN_INCOMPLETE: &str = "终端链路不完整";
pub const REASON_LSP_NOT_CONFIGURED: &str = "语言服务未配置";
pub const REASON_MEMORY_NOT_ENABLED: &str = "记忆存储未启用";
pub const REASON_IMAGE_BACKEND_NOT_CONFIGURED: &str = "图像后端未配置";
pub const REASON_VIDEO_BACKEND_NOT_CONFIGURED: &str = "视频后端未配置";
pub const REASON_MCP_NOT_CONFIGURED: &str = "能力注册未配置";
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

/// One incomplete work tool with its neutral reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeFailure {
    pub tool: String,
    pub reason: &'static str,
}

/// Per-round probe snapshot (design §4: used for the round, then
/// discarded — never persisted, never across runs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolProbeSnapshot {
    pub complete: Vec<String>,
    pub incomplete: Vec<ProbeFailure>,
}

/// The minimal previous-round mapping — `tool → complete/incomplete` for
/// every work tool. Reasons are never cached (design §8: 详情与 reason 不
/// 缓存); the map exists solely for the round-to-round flip comparison
/// inside one run. Snapshots themselves are used then discarded — nothing
/// here is persisted or crosses runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinimalProbeMap {
    status: std::collections::BTreeMap<String, bool>,
}

impl MinimalProbeMap {
    /// Canonical map for a probe snapshot — every ACCOUNTED tool carries
    /// exactly one status. Since S2d 裁决二 (ADR-0010 §14.58) the snapshot
    /// is narrowed to the declared surface before it reaches the map, so
    /// sealed/registry-absent work tools are absent from the map too.
    pub fn from_snapshot(snapshot: &ToolProbeSnapshot) -> Self {
        let mut status = std::collections::BTreeMap::new();
        for tool in &snapshot.complete {
            status.insert(tool.clone(), true);
        }
        for failure in &snapshot.incomplete {
            status.insert(failure.tool.clone(), false);
        }
        Self { status }
    }

    /// Whether the fresh snapshot differs from this map — i.e. at least
    /// one work tool flipped (complete↔incomplete).
    pub fn differs_from(&self, snapshot: &ToolProbeSnapshot) -> bool {
        Self::from_snapshot(snapshot) != *self
    }

    /// 调用即探针 (design §5): write a real call failure back as
    /// `incomplete`. Returns whether the map actually changed.
    pub fn mark_incomplete(&mut self, tool: &str) -> bool {
        if !is_main_agent_work_tool(tool) {
            return false;
        }
        if self.status.get(tool) == Some(&false) {
            return false;
        }
        self.status.insert(tool.to_string(), false);
        true
    }
}

/// Mechanical chain inputs a probe can see at probe time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeContext {
    /// Session working directory — the workspace scope probed for the
    /// read/write/storage chains.
    pub cwd: PathBuf,
    /// Session permission policy — the write/exec policy half of the
    /// `search_replace` / `todo_write` / `update_goal` /
    /// `run_terminal_cmd` probes (`ReadOnly` refuses writes; shell-escape
    /// is Interactive-only).
    pub policy: ToolPolicy,
    /// Whether the host carries a fixed test runner
    /// (`host.test_runner().is_some()`) — the `run_tests` probe source.
    pub test_runner_present: bool,
    /// Whether the session is attached to an interactive user who can
    /// answer `ask_user_question` and approve plan mode (headless sessions
    /// fail closed).
    pub interactive_user: bool,
    /// Whether the run carries a goal/todo context (`todo_write` /
    /// `update_goal` chain).
    pub goal_context_present: bool,
    /// Whether a retrieval activation carries an undisposed pending
    /// assessment — the only state in which `retrieval_disposition` can be
    /// submitted (`retrieval_disposition` chain; 审查复核 2026-08-13).
    pub pending_retrieval_activation: bool,
    /// Host terminal backend presence (`run_terminal_cmd` chain).
    pub terminal_available: bool,
    /// Workspace language-service configuration presence (`lsp` chain).
    pub lsp_configured: bool,
    /// Explicit memory opt-in + backend presence (`memory_get` /
    /// `memory_search` chain).
    pub memory_enabled: bool,
    /// Image backend credentials/configuration (`image_gen` / `image_edit`
    /// chain).
    pub image_backend_configured: bool,
    /// Video backend credentials/configuration (`image_to_video` /
    /// `reference_to_video` chain).
    pub video_backend_configured: bool,
    /// MCP / capability registration present in session scope (`use_tool`
    /// chain).
    pub mcp_registry_available: bool,
}

/// Whether `name` is a main-agent work tool. Tools outside the matrix
/// (retrieval lane, bash, host-owned extras) are not part of the list
/// projection and keep their existing declaration rules.
pub fn is_main_agent_work_tool(name: &str) -> bool {
    WORK_TOOLS.contains(&name)
}

/// Policy half of the write probe: `ReadOnly` never passes; `Interactive`
/// `Benchmark` and `BenchmarkFull` pass at policy level (the call-time
/// permission gate still makes the final decision per arguments).
pub fn policy_allows_write(policy: ToolPolicy) -> bool {
    policy != ToolPolicy::ReadOnly
}

/// Policy half of the exec probe: `Interactive` and `BenchmarkFull`
/// (FUS-BENCHMARK-FULL-EXEC 2026-08-18) pass at policy level — ReadOnly
/// refuses non-reads and the default `Benchmark` excludes shell-escape by
/// policy (the call-time permission gate remains the final backstop).
pub fn policy_allows_exec(policy: ToolPolicy) -> bool {
    matches!(policy, ToolPolicy::Interactive | ToolPolicy::BenchmarkFull)
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

fn probe_storage(ctx: &ProbeContext) -> ProbeVerdict {
    // blackboard / compaction-whitelist persistence lives under the
    // session workspace (`.gsa/`); the session-state chain is unreadable
    // when the workspace itself is.
    if workspace_readable(&ctx.cwd) {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_SESSION_STORAGE_UNREADABLE)
    }
}

fn probe_goal_write(ctx: &ProbeContext) -> ProbeVerdict {
    if !ctx.goal_context_present {
        return ProbeVerdict::Incomplete(REASON_NO_GOAL_CONTEXT);
    }
    probe_write(ctx)
}

fn probe_plan_mode(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.interactive_user {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_PLAN_MODE_UNSUPPORTED)
    }
}

fn probe_retrieval_disposition(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.pending_retrieval_activation {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_RETRIEVAL_NOT_ACTIVE)
    }
}

fn probe_terminal(ctx: &ProbeContext) -> ProbeVerdict {
    if policy_allows_exec(ctx.policy) && ctx.terminal_available {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_TERMINAL_CHAIN_INCOMPLETE)
    }
}

fn probe_lsp(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.lsp_configured {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_LSP_NOT_CONFIGURED)
    }
}

fn probe_memory(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.memory_enabled {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_MEMORY_NOT_ENABLED)
    }
}

fn probe_image(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.image_backend_configured {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_IMAGE_BACKEND_NOT_CONFIGURED)
    }
}

fn probe_video(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.video_backend_configured {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_VIDEO_BACKEND_NOT_CONFIGURED)
    }
}

fn probe_mcp(ctx: &ProbeContext) -> ProbeVerdict {
    if ctx.mcp_registry_available {
        ProbeVerdict::Complete
    } else {
        ProbeVerdict::Incomplete(REASON_MCP_NOT_CONFIGURED)
    }
}

/// Probe one tool name against the mechanical chain. Unknown tools (no
/// probe implementation) fail closed with `未完成链路检查`.
pub fn probe_tool(name: &str, ctx: &ProbeContext) -> ToolProbeResult {
    let verdict = match name {
        "read_file" | "list_dir" | "grep" | "search_tool" => probe_read(ctx),
        "search_replace" => probe_write(ctx),
        "run_tests" => {
            if ctx.test_runner_present {
                ProbeVerdict::Complete
            } else {
                ProbeVerdict::Incomplete(REASON_MISSING_TEST_RUNNER)
            }
        }
        "ask_user_question" => {
            if ctx.interactive_user {
                ProbeVerdict::Complete
            } else {
                ProbeVerdict::Incomplete(REASON_NO_INTERACTIVE_USER)
            }
        }
        "blackboard_read" | "compaction_whitelist_add" => probe_storage(ctx),
        "todo_write" | "update_goal" => probe_goal_write(ctx),
        "enter_plan_mode" | "exit_plan_mode" => probe_plan_mode(ctx),
        "retrieval_disposition" => probe_retrieval_disposition(ctx),
        "run_terminal_cmd" => probe_terminal(ctx),
        "lsp" => probe_lsp(ctx),
        "memory_get" | "memory_search" => probe_memory(ctx),
        "image_gen" | "image_edit" => probe_image(ctx),
        "image_to_video" | "reference_to_video" => probe_video(ctx),
        "use_tool" => probe_mcp(ctx),
        _ => ProbeVerdict::Incomplete(REASON_UNFINISHED_CHAIN_CHECK),
    };
    ToolProbeResult {
        tool: name.to_string(),
        verdict,
    }
}

/// Probe every work tool in canonical order into one snapshot.
pub fn probe_work_tools(ctx: &ProbeContext) -> ToolProbeSnapshot {
    let mut snapshot = ToolProbeSnapshot::default();
    for tool in WORK_TOOLS {
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

/// S2d 裁决二 (2026-09-06, ADR-0010 §14.58): narrow the probe partition to
/// the currently-visible declared surface — a tool that can never re-enter
/// the model-visible face (R1-sealed from the main face, or absent from the
/// registry) is dropped from `complete`/`incomplete`. Its mechanical verdict
/// can still flip internally, but that flip can never change the projected
/// tool list, so journaling it as an availability flip would trip the
/// probe↔header accuracy invariant (ADR-0010 §3.5 条 7) for a phantom
/// change. Legitimate declaration-surface flips (e.g. retrieval mode A
/// browser-capability degradation) keep their declared tools in the
/// partition and remain governed.
pub fn narrow_to_declared(
    snapshot: ToolProbeSnapshot,
    is_declared: impl Fn(&str) -> bool,
) -> ToolProbeSnapshot {
    ToolProbeSnapshot {
        complete: snapshot
            .complete
            .into_iter()
            .filter(|tool| is_declared(tool))
            .collect(),
        incomplete: snapshot
            .incomplete
            .into_iter()
            .filter(|failure| is_declared(&failure.tool))
            .collect(),
    }
}

/// Declared-surface narrowing as wired at BOTH production snapshot sites
/// (run-start + per-round): registry-present AND not R1-sealed from the
/// main face — single source so the two call sites cannot drift (S2d 批 2
/// 复审处理采纳). The base tool-defs list must be the same mode-projected
/// base the visible-face projection consumes.
pub fn narrow_to_declared_face(
    snapshot: ToolProbeSnapshot,
    base_tool_defs: &[crate::host::ToolDef],
) -> ToolProbeSnapshot {
    narrow_to_declared(snapshot, |tool| {
        base_tool_defs.iter().any(|d| d.name == tool)
            && !crate::controller::AgentLoopController::R1_SEALED_MAIN_TOOLS.contains(&tool)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{GatewayError, ModelGateway, ModelRequest, ModelResponse};
    use crate::host::{
        LoopHost, PermitDecision, PermitError, RiskClass, ToolDef, ToolError, ToolRegistry,
        ToolResult,
    };
    use async_trait::async_trait;
    use orz_assurance::{EventType, JournalRecorder, RunEvent};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

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
            goal_context_present: true,
            pending_retrieval_activation: true,
            terminal_available: true,
            lsp_configured: true,
            memory_enabled: true,
            image_backend_configured: true,
            video_backend_configured: true,
            mcp_registry_available: true,
        }
    }

    fn missing_ctx() -> ProbeContext {
        ProbeContext {
            cwd: std::env::temp_dir().join("orz-tool-probe-missing-dir"),
            ..full_ctx(Path::new(env!("CARGO_MANIFEST_DIR")))
        }
    }

    #[test]
    fn work_tool_membership_is_exact() {
        assert_eq!(WORK_TOOLS.len(), 23, "single probe face = 23 tools");
        for tool in WORK_TOOLS {
            assert!(is_main_agent_work_tool(tool), "{tool} must be a work tool");
        }
        for tool in [
            "web_search",
            "web_fetch",
            "browser_read",
            "pdf_read",
            "project_doc_index",
            "retrieve_project_docs",
            "bash",
            "no_such_tool",
        ] {
            assert!(
                !is_main_agent_work_tool(tool),
                "{tool} must not be a work tool"
            );
        }
    }

    // ── S2d 裁决二 (2026-09-06, ADR-0010 §14.58): declared-surface
    //    narrowing of the availability accounting ────────────────────────

    /// The declared-surface predicate as wired at both snapshot call sites:
    /// registry-present AND not R1-sealed. A sealed tool that is registry-
    /// present (e.g. `todo_write`) and a registry-absent tool (e.g. an
    /// unwired backend) both drop out.
    fn declared_face(tool: &str) -> bool {
        tool != "todo_write" && tool != "lsp"
    }

    #[test]
    fn narrow_to_declared_face_helper_contract() {
        // The shared production predicate (批 2 复审处理采纳): base declares
        // a declared tool (read_file) and a sealed one (todo_write); lsp is
        // registry-absent. Helper keeps the declared tool, drops the sealed
        // one from incomplete and the absent one from complete.
        let base: Vec<ToolDef> = ["read_file", "todo_write"]
            .iter()
            .map(|n| ToolDef {
                name: n.to_string(),
                description: format!("tool {n}"),
                parameters: serde_json::json!({}),
            })
            .collect();
        let snapshot = ToolProbeSnapshot {
            complete: vec!["read_file".into(), "lsp".into()],
            incomplete: vec![ProbeFailure {
                tool: "todo_write".into(),
                reason: REASON_NO_GOAL_CONTEXT,
            }],
        };
        let narrowed = narrow_to_declared_face(snapshot, &base);
        assert_eq!(narrowed.complete, vec!["read_file".to_string()]);
        assert!(narrowed.incomplete.is_empty());
    }

    #[test]
    fn narrow_to_declared_drops_sealed_and_registry_absent() {
        let snapshot = ToolProbeSnapshot {
            complete: vec!["read_file".into(), "todo_write".into(), "grep".into()],
            incomplete: vec![
                ProbeFailure {
                    tool: "lsp".into(),
                    reason: REASON_LSP_NOT_CONFIGURED,
                },
                ProbeFailure {
                    tool: "run_terminal_cmd".into(),
                    reason: REASON_TERMINAL_CHAIN_INCOMPLETE,
                },
            ],
        };
        let narrowed = narrow_to_declared(snapshot, declared_face);
        // Sealed (todo_write) and registry-absent (lsp) are gone from BOTH
        // sets; declared tools keep canonical order and reasons.
        assert_eq!(
            narrowed.complete,
            vec!["read_file".to_string(), "grep".to_string()]
        );
        assert_eq!(
            narrowed.incomplete,
            vec![ProbeFailure {
                tool: "run_terminal_cmd".into(),
                reason: REASON_TERMINAL_CHAIN_INCOMPLETE,
            }]
        );
    }

    /// 0ac S3①（2026-09-13，设计稿 §9/§10.2）：`tool_availability_check`
    /// 事件面现在承载**两个探针**——工作面 `main_agent_work_tools` 与检索族
    /// `retrieval_family`（各自成事件）。本模块的工作面钉子只数工作面事件。
    fn is_work_tool_probe(e: &RunEvent) -> bool {
        e.payload
            .get("probe_scope")
            .and_then(|s| s.as_str())
            != Some("retrieval_family")
    }

    #[test]
    fn narrow_to_declared_sealed_flip_is_not_an_availability_flip() {
        // A sealed tool flipping complete↔incomplete must NOT produce a
        // flip after narrowing (the projected face cannot change) — this is
        // the phantom-flip leak the 裁决二 accounting narrowing seals.
        let sealed_complete = ToolProbeSnapshot {
            complete: vec!["read_file".into(), "todo_write".into()],
            incomplete: vec![],
        };
        let sealed_flipped = ToolProbeSnapshot {
            complete: vec!["read_file".into()],
            incomplete: vec![ProbeFailure {
                tool: "todo_write".into(),
                reason: REASON_NO_GOAL_CONTEXT,
            }],
        };
        let before =
            MinimalProbeMap::from_snapshot(&narrow_to_declared(sealed_complete, declared_face));
        let after =
            MinimalProbeMap::from_snapshot(&narrow_to_declared(sealed_flipped, declared_face));
        assert_eq!(before, after, "sealed-tool flip must be invisible");

        // A declared tool flipping still flips after narrowing (legitimate
        // declaration-surface change → header change stays governed).
        let declared_flipped = ToolProbeSnapshot {
            complete: vec![],
            incomplete: vec![ProbeFailure {
                tool: "read_file".into(),
                reason: REASON_WORKSPACE_UNREADABLE,
            }],
        };
        let after_declared =
            MinimalProbeMap::from_snapshot(&narrow_to_declared(declared_flipped, declared_face));
        assert_ne!(
            before, after_declared,
            "declared-tool flip must remain a flip"
        );
    }

    #[test]
    fn narrowed_full_chain_partition_excludes_r1_sealed_work_tools() {
        // Full-chain context: every work tool probes complete; after
        // narrowing, the R1-sealed work tools (todo_write / update_goal /
        // compaction_whitelist_add / list_dir / search_tool / run_tests /
        // retrieval_disposition) must leave the partition while the
        // unsealed declared face stays intact.
        let dir = std::env::temp_dir().join("orz-tool-probe-narrow-full");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("probe test dir");
        let snapshot = probe_work_tools(&full_ctx(&dir));
        assert!(snapshot.incomplete.is_empty(), "{:?}", snapshot.incomplete);
        // Registry = every work tool present (the projection can only be
        // blocked by the R1 seal in this configuration).
        let narrowed = narrow_to_declared(snapshot, |tool| {
            !AgentLoopController::R1_SEALED_MAIN_TOOLS.contains(&tool)
        });
        for tool in AgentLoopController::R1_SEALED_MAIN_TOOLS {
            assert!(
                !narrowed.complete.iter().any(|c| c == tool),
                "sealed {tool} must not ride the narrowed partition"
            );
        }
        let sealed_work_tools = AgentLoopController::R1_SEALED_MAIN_TOOLS
            .iter()
            .filter(|tool| WORK_TOOLS.contains(tool))
            .count();
        assert_eq!(
            narrowed.complete.len(),
            WORK_TOOLS.len() - sealed_work_tools,
            "declared face = probe matrix minus the R1 seal"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn full_chain_marks_all_work_tools_complete() {
        let dir = TestDir::new("full");
        let snapshot = probe_work_tools(&full_ctx(dir.path()));
        assert_eq!(
            snapshot.complete,
            WORK_TOOLS.iter().map(|s| s.to_string()).collect::<Vec<_>>()
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
    fn missing_workspace_breaks_storage_tools() {
        let ctx = missing_ctx();
        for tool in ["blackboard_read", "compaction_whitelist_add"] {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Incomplete(REASON_SESSION_STORAGE_UNREADABLE),
                "{tool}"
            );
        }
    }

    #[test]
    fn readonly_policy_breaks_write_tools_even_with_readable_workspace() {
        let dir = TestDir::new("policy");
        let ctx = ProbeContext {
            policy: ToolPolicy::ReadOnly,
            ..full_ctx(dir.path())
        };
        for tool in ["search_replace", "todo_write", "update_goal"] {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Incomplete(REASON_WRITE_POLICY_NOT_ALLOWED),
                "{tool}"
            );
        }
    }

    #[test]
    fn goal_write_requires_goal_context() {
        let dir = TestDir::new("goal");
        let ctx = ProbeContext {
            goal_context_present: false,
            ..full_ctx(dir.path())
        };
        for tool in ["todo_write", "update_goal"] {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Incomplete(REASON_NO_GOAL_CONTEXT),
                "{tool}"
            );
        }
    }

    #[test]
    fn plan_mode_requires_interactive_user() {
        let dir = TestDir::new("plan");
        let without = ProbeContext {
            interactive_user: false,
            ..full_ctx(dir.path())
        };
        for tool in ["enter_plan_mode", "exit_plan_mode"] {
            assert_eq!(
                probe_tool(tool, &without).verdict,
                ProbeVerdict::Incomplete(REASON_PLAN_MODE_UNSUPPORTED),
                "{tool}"
            );
            assert_eq!(
                probe_tool(tool, &full_ctx(dir.path())).verdict,
                ProbeVerdict::Complete,
                "{tool}"
            );
        }
    }

    #[test]
    fn retrieval_disposition_requires_live_activation() {
        let dir = TestDir::new("activation");
        let without = ProbeContext {
            pending_retrieval_activation: false,
            ..full_ctx(dir.path())
        };
        assert_eq!(
            probe_tool("retrieval_disposition", &without).verdict,
            ProbeVerdict::Incomplete(REASON_RETRIEVAL_NOT_ACTIVE)
        );
        assert_eq!(
            probe_tool("retrieval_disposition", &full_ctx(dir.path())).verdict,
            ProbeVerdict::Complete
        );
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
    fn terminal_requires_interactive_policy_and_host_terminal() {
        let dir = TestDir::new("terminal");
        let ctx = full_ctx(dir.path());
        assert_eq!(
            probe_tool("run_terminal_cmd", &ctx).verdict,
            ProbeVerdict::Complete
        );
        let no_backend = ProbeContext {
            terminal_available: false,
            ..ctx.clone()
        };
        assert_eq!(
            probe_tool("run_terminal_cmd", &no_backend).verdict,
            ProbeVerdict::Incomplete(REASON_TERMINAL_CHAIN_INCOMPLETE)
        );
        for policy in [ToolPolicy::ReadOnly, ToolPolicy::Benchmark] {
            let blocked = ProbeContext {
                policy,
                ..ctx.clone()
            };
            assert_eq!(
                probe_tool("run_terminal_cmd", &blocked).verdict,
                ProbeVerdict::Incomplete(REASON_TERMINAL_CHAIN_INCOMPLETE),
                "{policy:?}"
            );
        }
        // FUS-BENCHMARK-FULL-EXEC (2026-08-18): the full benchmark policy
        // (shell axis open) passes at policy level — with a terminal
        // backend the exec chain is complete, so the action-bar button is
        // rendered (the console still routes execution via orders, never a
        // direct model tool call).
        let full = ProbeContext {
            policy: ToolPolicy::BenchmarkFull,
            ..ctx.clone()
        };
        assert_eq!(
            probe_tool("run_terminal_cmd", &full).verdict,
            ProbeVerdict::Complete
        );
    }

    #[test]
    fn backend_tools_require_host_configuration() {
        let dir = TestDir::new("backend");
        let ctx = full_ctx(dir.path());
        let cases: &[(&str, bool, &str)] = &[
            ("lsp", false, REASON_LSP_NOT_CONFIGURED),
            ("memory_get", false, REASON_MEMORY_NOT_ENABLED),
            ("memory_search", false, REASON_MEMORY_NOT_ENABLED),
            ("image_gen", false, REASON_IMAGE_BACKEND_NOT_CONFIGURED),
            ("image_edit", false, REASON_IMAGE_BACKEND_NOT_CONFIGURED),
            ("image_to_video", false, REASON_VIDEO_BACKEND_NOT_CONFIGURED),
            (
                "reference_to_video",
                false,
                REASON_VIDEO_BACKEND_NOT_CONFIGURED,
            ),
            ("use_tool", false, REASON_MCP_NOT_CONFIGURED),
        ];
        for (tool, configured, reason) in cases {
            assert_eq!(
                probe_tool(tool, &ctx).verdict,
                ProbeVerdict::Complete,
                "{tool}"
            );
            let flag = |name: &str, value: bool| -> ProbeContext {
                let mut c = ctx.clone();
                match name {
                    "lsp" => c.lsp_configured = value,
                    "memory" => c.memory_enabled = value,
                    "image" => c.image_backend_configured = value,
                    "video" => c.video_backend_configured = value,
                    "mcp" => c.mcp_registry_available = value,
                    _ => unreachable!(),
                }
                c
            };
            let group = match *tool {
                "lsp" => "lsp",
                "memory_get" | "memory_search" => "memory",
                "image_gen" | "image_edit" => "image",
                "image_to_video" | "reference_to_video" => "video",
                "use_tool" => "mcp",
                _ => unreachable!(),
            };
            assert_eq!(
                probe_tool(tool, &flag(group, *configured)).verdict,
                ProbeVerdict::Incomplete(reason),
                "{tool}"
            );
        }
    }

    #[test]
    fn policy_allows_write_mapping() {
        assert!(policy_allows_write(ToolPolicy::Interactive));
        assert!(policy_allows_write(ToolPolicy::Benchmark));
        assert!(policy_allows_write(ToolPolicy::BenchmarkFull));
        assert!(!policy_allows_write(ToolPolicy::ReadOnly));
    }

    #[test]
    fn policy_allows_exec_mapping() {
        assert!(policy_allows_exec(ToolPolicy::Interactive));
        assert!(policy_allows_exec(ToolPolicy::BenchmarkFull));
        assert!(!policy_allows_exec(ToolPolicy::ReadOnly));
        assert!(!policy_allows_exec(ToolPolicy::Benchmark));
    }

    #[test]
    fn unknown_tool_fails_closed() {
        let dir = TestDir::new("unknown");
        let ctx = full_ctx(dir.path());
        for tool in ["no_such_tool", "web_search", "bash"] {
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
            pending_retrieval_activation: false,
            terminal_available: false,
            lsp_configured: false,
            memory_enabled: false,
            image_backend_configured: false,
            video_backend_configured: false,
            mcp_registry_available: false,
            ..full_ctx(dir.path())
        };
        let snapshot = probe_work_tools(&ctx);
        assert_eq!(
            snapshot.complete,
            vec![
                "read_file".to_string(),
                "list_dir".to_string(),
                "grep".to_string(),
                "search_tool".to_string(),
                "blackboard_read".to_string(),
                "compaction_whitelist_add".to_string(),
            ]
        );
        assert_eq!(snapshot.incomplete.len(), 17, "{:?}", snapshot.incomplete);
        let tools: Vec<&str> = snapshot
            .incomplete
            .iter()
            .map(|f| f.tool.as_str())
            .collect();
        for tool in [
            "search_replace",
            "run_tests",
            "ask_user_question",
            "todo_write",
            "update_goal",
            "enter_plan_mode",
            "exit_plan_mode",
            "retrieval_disposition",
            "run_terminal_cmd",
            "lsp",
            "memory_get",
            "memory_search",
            "image_gen",
            "image_edit",
            "image_to_video",
            "reference_to_video",
            "use_tool",
        ] {
            assert!(tools.contains(&tool), "missing incomplete {tool}");
        }
    }

    #[test]
    fn minimal_map_covers_all_work_tools_and_detects_flips() {
        let dir = TestDir::new("minimal-map");
        let full = probe_work_tools(&full_ctx(dir.path()));
        assert_eq!(
            MinimalProbeMap::from_snapshot(&full).status.len(),
            WORK_TOOLS.len(),
            "minimal map must cover every work tool exactly once"
        );
        let map = MinimalProbeMap::from_snapshot(&full);
        assert!(!map.differs_from(&full), "same partition is not a flip");

        // One tool flips complete → incomplete: the map must differ.
        let mut changed = full.clone();
        changed.complete.retain(|t| t != "run_tests");
        changed.incomplete.push(ProbeFailure {
            tool: "run_tests".to_string(),
            reason: REASON_MISSING_TEST_RUNNER,
        });
        assert!(map.differs_from(&changed), "flip must be detected");

        // Order of the incomplete list is irrelevant — only the partition.
        let mut reordered = changed.clone();
        reordered.incomplete.reverse();
        let changed_map = MinimalProbeMap::from_snapshot(&changed);
        assert!(
            !changed_map.differs_from(&reordered),
            "same partition in another order is not a flip"
        );
    }

    #[test]
    fn minimal_map_mark_incomplete_writes_back_work_tools_only() {
        let dir = TestDir::new("minimal-writeback");
        let full = probe_work_tools(&full_ctx(dir.path()));
        let mut map = MinimalProbeMap::from_snapshot(&full);

        assert!(
            map.mark_incomplete("run_tests"),
            "complete → incomplete must change the map"
        );
        assert!(
            !map.mark_incomplete("run_tests"),
            "already incomplete must not change the map"
        );
        assert!(!map.mark_incomplete("bash"), "non-work tools are ignored");
        assert!(map.differs_from(&full));
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

    // ── P0-A step 5: minimal previous-round map + flip-only events ──────

    /// Registry declaring a SEALED work tool (`run_tests`) plus a DECLARED
    /// flip vehicle (`run_terminal_cmd`) — S2d 裁决二 (ADR-0010 §14.58):
    /// the sealed tool must stay silent in the availability accounting while
    /// the declared one remains flip-observable.
    struct SealedAndDeclaredRegistry;
    impl ToolRegistry for SealedAndDeclaredRegistry {
        fn get(&self, name: &str) -> Option<ToolDef> {
            self.list().into_iter().find(|t| t.name == name)
        }
        fn list(&self) -> Vec<ToolDef> {
            ["run_tests", "run_terminal_cmd"]
                .iter()
                .map(|n| ToolDef {
                    name: n.to_string(),
                    description: format!("tool {n}"),
                    parameters: serde_json::json!({}),
                })
                .collect()
        }
    }

    /// A host whose runner/terminal presence can flip mid-run — the
    /// per-round probe must observe the DECLARED change and emit a second
    /// `tool_availability_check`; the sealed tool's flip stays silent.
    struct FlipRunnerHost {
        journal: JournalRecorder,
        runner: std::sync::Arc<AtomicBool>,
        terminal: std::sync::Arc<AtomicBool>,
    }
    #[async_trait]
    impl LoopHost for FlipRunnerHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &SealedAndDeclaredRegistry
        }
        fn tool_policy(&self) -> crate::host::ToolPolicy {
            // BenchmarkFull: the exec policy gate must pass so the terminal
            // chain probe can flip with `terminal_available` (裁决二 flip
            // vehicle run_terminal_cmd is a declared tool).
            crate::host::ToolPolicy::BenchmarkFull
        }
        fn test_runner(&self) -> Option<crate::host::TestRunner> {
            self.runner
                .load(Ordering::SeqCst)
                .then(|| crate::host::TestRunner {
                    command: vec!["pytest-stub".to_string()],
                    timeout: None,
                    env: Vec::new(),
                })
        }
        fn terminal_available(&self) -> bool {
            self.terminal.load(Ordering::SeqCst)
        }
        async fn request_permission(
            &self,
            _risk: RiskClass,
            _tool: &str,
            _args: &serde_json::Value,
        ) -> Result<PermitDecision, PermitError> {
            Ok(PermitDecision::AllowOnce)
        }
        async fn call_tool(
            &self,
            _name: &str,
            _args: serde_json::Value,
            _call_id: &str,
        ) -> Result<ToolResult, ToolError> {
            unreachable!("text-only flip test never calls tools");
        }
    }

    /// Flips the host terminal AND runner flags during the FIRST model
    /// round — the next loop-top probe sees the new state. Only the
    /// declared flip (run_terminal_cmd) may journal an event.
    struct FlipRunnerAfterFirstRound {
        inner: Arc<FakeProvider>,
        runner: std::sync::Arc<AtomicBool>,
        terminal: std::sync::Arc<AtomicBool>,
        flipped: std::sync::Arc<AtomicBool>,
    }
    #[async_trait]
    impl ModelGateway for FlipRunnerAfterFirstRound {
        fn for_new_run(&self) -> std::sync::Arc<dyn ModelGateway> {
            // per-run 隔离语义：新 run 从「未翻转」开始（共享底层
            // fake/runner/terminal/flipped ——测试断言语义不变）。
            std::sync::Arc::new(Self {
                inner: self.inner.clone(),
                runner: self.runner.clone(),
                terminal: self.terminal.clone(),
                flipped: std::sync::Arc::new(AtomicBool::new(false)),
            })
        }

        async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
            if self
                .flipped
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                self.runner.store(false, Ordering::SeqCst);
                self.terminal.store(false, Ordering::SeqCst);
            }
            self.inner.generate(request).await
        }
    }

    /// P0-A step 5 + S2d 裁决二 (ADR-0010 §14.58): a mid-run probe flip on
    /// a DECLARED tool (run_terminal_cmd complete → incomplete) emits a
    /// SECOND `tool_availability_check` event and re-projects the next
    /// model request — no event fires while the partition is stable. The
    /// SEALED tool's simultaneous flip (run_tests) stays silent: it can
    /// never change the projected face, so it must not ride the
    /// availability accounting.
    #[tokio::test]
    async fn probe_flip_emits_second_availability_event_and_reprojects() {
        let dir = test_dir();
        let runner = std::sync::Arc::new(AtomicBool::new(true));
        let terminal = std::sync::Arc::new(AtomicBool::new(true));
        let host = FlipRunnerHost {
            journal: JournalRecorder::new(dir.clone()),
            runner: runner.clone(),
            terminal: terminal.clone(),
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = Arc::new(FlipRunnerAfterFirstRound {
            inner: fake.clone(),
            runner,
            terminal,
            flipped: std::sync::Arc::new(AtomicBool::new(false)),
        });
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-FLIP", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let all = events(&dir);
        let checks: Vec<&RunEvent> = all
            .iter()
            .filter(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .collect();
        assert_eq!(
            checks.len(),
            2,
            "one initial + one flip event — the sealed run_tests flip emits nothing"
        );
        let first_idx = all
            .iter()
            .position(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .unwrap();
        let run_started_idx = all
            .iter()
            .position(|e| e.event_type == EventType::RunStarted)
            .unwrap();
        assert!(
            first_idx < run_started_idx,
            "initial event must precede run_started"
        );
        // Both events carry the DECLARED surface only — the sealed
        // run_tests never rides complete or incomplete (裁决二).
        for check in &checks {
            let partition: Vec<String> = check.payload["complete"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .chain(
                    check.payload["incomplete"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|v| v["tool"].as_str().map(str::to_string)),
                )
                .collect();
            assert!(
                !partition.iter().any(|t| t == "run_tests"),
                "sealed run_tests must not ride the narrowed partition: {partition:?}"
            );
        }
        // Initial snapshot: terminal present → run_terminal_cmd complete.
        assert!(
            checks[0].payload["complete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("run_terminal_cmd")),
            "initial payload: {:?}",
            checks[0].payload
        );
        // Flip snapshot: run_terminal_cmd incomplete with the neutral reason.
        let p = &checks[1].payload;
        assert!(
            !p["complete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("run_terminal_cmd")),
            "flip payload complete: {:?}",
            p["complete"]
        );
        assert!(
            p["incomplete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["tool"] == "run_terminal_cmd" && v["reason"] == "终端链路不完整"),
            "flip payload incomplete: {:?}",
            p["incomplete"]
        );
        assert_eq!(p["gate_decision"], "pass", "probe never blocks");
        // R1 封存：run_tests 无论探针状态都不进声明面——两个请求均不出现；
        // 声明面翻转（run_terminal_cmd）随探针重投影。
        let received = fake.received_requests();
        assert!(
            received
                .iter()
                .all(|r| !r.tools.iter().any(|t| t.name == "run_tests")),
            "run_tests sealed from the declaration surface: {:?}",
            received
                .iter()
                .map(|r| r.tools.iter().map(|t| &t.name).collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );
        assert!(
            received[0]
                .tools
                .iter()
                .any(|t| t.name == "run_terminal_cmd"),
            "declared face initially projects run_terminal_cmd"
        );
        assert!(
            !received[1]
                .tools
                .iter()
                .any(|t| t.name == "run_terminal_cmd"),
            "flip re-projects the next model request"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A host whose `read_file` mechanically fails at call time while the
    /// probe (readable workspace) still says complete — 调用即探针 must
    /// correct the minimal map so the next probe reports a recovery flip.
    struct FailingReadHost {
        journal: JournalRecorder,
    }
    #[async_trait]
    impl LoopHost for FailingReadHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &MixedProjectionRegistry
        }
        async fn request_permission(
            &self,
            _risk: RiskClass,
            _tool: &str,
            _args: &serde_json::Value,
        ) -> Result<PermitDecision, PermitError> {
            Ok(PermitDecision::AllowOnce)
        }
        async fn call_tool(
            &self,
            name: &str,
            _args: serde_json::Value,
            _call_id: &str,
        ) -> Result<ToolResult, ToolError> {
            if name == "read_file" {
                return Err(ToolError::ExecutionFailed("stub read failure".into()));
            }
            unreachable!("only read_file is called");
        }
    }

    /// P0-A step 5 (design §5): a real work-tool call failure writes back into
    /// the minimal map (调用即探针) — the next per-round probe sees the
    /// corrected state and emits a recovery-flip event.
    #[tokio::test]
    async fn probe_call_failure_writes_back_and_emits_recovery_flip() {
        let dir = test_dir();
        let host = FailingReadHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "读", "RUN-WB", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let all = events(&dir);
        let checks: Vec<&RunEvent> = all
            .iter()
            .filter(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .collect();
        assert_eq!(checks.len(), 2, "initial + recovery flip");
        // The failure itself is audited as ToolCompleted(error).
        let failed_idx = all
            .iter()
            .position(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("read_file ToolCompleted(error) must be journaled");
        // The recovery flip event follows the failed call.
        let flip_idx = all
            .iter()
            .enumerate()
            .filter(|(_, e)| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .map(|(i, _)| i)
            .nth(1)
            .expect("second availability event");
        assert!(
            flip_idx > failed_idx,
            "recovery flip must follow the failed call"
        );
        // The flip snapshot reports the probe truth: read_file complete again.
        assert!(
            checks[1].payload["complete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("read_file")),
            "recovery flip payload: {:?}",
            checks[1].payload
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-A step 5 review fix (2026-08-13): the retrieval lane never
    /// writes back into the main probe map — a failed work-tool read call
    /// inside the lane must NOT flip `read_file` in the main map.
    /// THIN-HARNESS-REDESIGN R1 (§4.4): auto-close 下激活不再进入
    /// awaiting-disposition，`retrieval_disposition` 探针恒 incomplete——
    /// 无激活生命周期翻转，journal 只含初始那一条 availability 事件
    /// （车道内失败也不产生主面事件）。
    #[tokio::test]
    async fn retrieval_lane_failure_does_not_pollute_main_probe_map() {
        let dir = test_dir();
        let host = FailingReadHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        // Main declares retrieval → subagent round 1 calls read_file
        // (work-tool host tool, allowed in the lane) and FAILS → subagent
        // round 2 forms the result → main concludes (gate + final).
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-2")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-LANE-WB",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let types = event_types(&dir);
        let availability_count = types
            .iter()
            .filter(|t| **t == EventType::ToolAvailabilityCheck)
            .count();
        assert_eq!(
            availability_count, 2,
            "0ac S3① probe face = work-tool + retrieval-family events, both \
             run-start (no activation flip and no lane-local pollution): {types:?}"
        );
        // The lane failure itself is still audited via ToolCompleted(error).
        assert!(
            types.contains(&EventType::ToolCompleted),
            "lane failure must stay in the audit chain: {types:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-A step 5: the minimal previous-round map is per-run — a second
    /// run on the same controller starts fresh. Run 1 has a terminal
    /// (run_terminal_cmd complete); run 2 loses it. If run 1's map leaked,
    /// run 2's first loop-top probe would differ and emit a SECOND
    /// availability event — asserting exactly one per run locks the reset.
    #[tokio::test]
    async fn probe_state_resets_across_runs() {
        let dir1 = test_dir();
        let dir2 = test_dir();
        let runner = std::sync::Arc::new(AtomicBool::new(true));
        let terminal = std::sync::Arc::new(AtomicBool::new(true));
        let host1 = FlipRunnerHost {
            journal: JournalRecorder::new(dir1.clone()),
            runner: runner.clone(),
            terminal: terminal.clone(),
        };
        // Four texts: two per run (counterexample gate + final answer).
        let fake = Arc::new(FakeProvider::from_texts(vec![
            "完成", "完成", "完成", "完成",
        ]));
        let controller = AgentLoopController::with_gateway(fake);
        controller
            .run_turn(&host1, "hi", "RUN-R1", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let run1 = events(&dir1);
        let r1_count = run1
            .iter()
            .filter(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .count();
        assert_eq!(r1_count, 1, "run 1 must emit exactly one event");
        assert!(
            run1.iter()
                .find(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
                .unwrap()
                .payload["complete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str() == Some("run_terminal_cmd")),
            "run 1: run_terminal_cmd complete"
        );

        terminal.store(false, Ordering::SeqCst);
        let host2 = FlipRunnerHost {
            journal: JournalRecorder::new(dir2.clone()),
            runner,
            terminal,
        };
        controller
            .run_turn(&host2, "hi", "RUN-R2", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let run2 = events(&dir2);
        let r2_count = run2
            .iter()
            .filter(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
            .count();
        assert_eq!(
            r2_count,
            1,
            "run 2 must not inherit run 1's map: {:?}",
            event_types(&dir2)
        );
        assert!(
            run2.iter()
                .find(|e| e.event_type == EventType::ToolAvailabilityCheck && is_work_tool_probe(e))
                .unwrap()
                .payload["incomplete"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v["tool"] == "run_terminal_cmd" && v["reason"] == "终端链路不完整"),
            "run 2: run_terminal_cmd incomplete"
        );
        let _ = std::fs::remove_dir_all(&dir1);
        let _ = std::fs::remove_dir_all(&dir2);
    }
}
