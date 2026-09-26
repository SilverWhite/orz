//! Permission bridge — Grok permission manager behind the LoopHost contract.
//!
//! IP6 (hard-gate @ orz-workspace::permission) is the single allowed
//! assurance injection into kept Grok providers. This bridge walks public
//! seams only:
//! - yolo ON by default (用户令 2026-09-26「直接开auto mode就行」：工作台
//!   默认自动审批，手动弹窗退役为非默认路径) — the manager starts in
//!   always-approve; policy denies (ReadOnly/Benchmark) still short-circuit
//!   BEFORE yolo, and the no-bridge `OrzHost::new` default stays fail-closed
//!   `Deny` (review P1-1 不动);
//! - interactive prompting (`PermissionHookTransport`) remains the manual
//!   path; a non-yolo `Ask` still fails closed to `Deny` (timeout/cancel
//!   included);
//! - every decision is journaled here (GateDecision / PermissionDecision
//!   events are written by the caller/controller).
//!
//! P1 scope hardening (2026-08-04 review): the provider's Read decision is
//! unconditional `Allow` (`SAFE_COMMAND`) and `read_file` preserves absolute
//! paths — so this bridge enforces the session-cwd scope itself before the
//! manager sees the request (see `access_in_scope`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use agent_client_protocol::{ToolCallId, ToolCallUpdate, ToolCallUpdateFields};
use orz_assurance::tool_names::{BLACKBOARD_WRITE_TOOL_NAME, CONTEXT_COMPRESS_TOOL_NAME};
use orz_loop::host::{PermitDecision, PermitError, PermitSource, RiskClass};
use orz_workspace::permission::{
    AccessKind, ClientType, Decision, PermissionHandle, PermissionHookTransport,
    spawn_permission_manager_with_hub,
};
use xai_acp_lib::AcpAgentGatewaySender;

/// How long a live gateway may wait for the client to answer a permission
/// prompt before the bridge fails closed (mirrors the Phase 2 TUI bridge's
/// 5-minute permission timeout; a silent client must not stall the loop
/// forever — 2026-08-04 review P2-2).
///
/// `pub` so orz-tui's permission-dialog countdown shares this single source
/// of truth (Phase 3 slice #7): the TUI times out its dialog from its own
/// clock at the same ~300s mark; a few ms of drift between the two is benign
/// (the host's deny wins, the dialog's respond send fails silently).
pub const PERMISSION_PROMPT_TIMEOUT: Duration = Duration::from_secs(300);

/// Per-session permission policy (Phase 3 slice #16 — codex app-server
/// `sandbox` surface).
///
/// `Interactive` is the historical behavior: reads auto-allow, everything
/// else prompts (or fails closed headless). `ReadOnly` is the codex
/// read-only sandbox: read-class tools auto-allow, every other risk class is
/// denied *before* the manager sees the request — no prompt, no approval
/// wire, no `tool_started` ("a write never happens"). `Benchmark` is the
/// headless benchmark/automation policy (2026-08-06 polyglot harness):
/// read-class and local-mutation tools auto-allow — the loop can edit files
/// without a client — while network and shell-escape fail closed like
/// ReadOnly.
///
/// FUS-BENCHMARK-FULL-EXEC (2026-08-18): `Benchmark` is parameterized on two
/// axes — `allow_shell` (shell tools and SandboxEscape auto-allow) and
/// `allow_network` (NetworkCall auto-allow). Default false/false keeps the
/// historical fail-closed semantics exactly; TB2 scoring passes
/// `Benchmark{allow_shell: true, allow_network: true}` via the adapter
/// (`--allow-shell` / `--allow-network`). Opening an axis is a policy-allow
/// surface only — the ACAF ticket gate, audit chain and mode gates are
/// unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionPolicy {
    /// Normal prompt-decides behavior (reads auto-allow; the rest prompts).
    #[default]
    Interactive,
    /// Read-only sandbox — mutation/network/escape requests fail closed
    /// without prompting.
    ReadOnly,
    /// Headless benchmark — read + local file edits auto-allow; shell and
    /// network follow the `allow_shell` / `allow_network` axes (both default
    /// false — fail closed).
    Benchmark {
        /// Shell execution auto-allow (`run_terminal_cmd`/`bash`/`sh`/`cmd`/
        /// `powershell`/`pwsh` + SandboxEscape aliases).
        allow_shell: bool,
        /// Network calls auto-allow (`web_fetch`/`web_search` direct-call
        /// surface).
        allow_network: bool,
    },
}

/// Shell-execution tool names (Benchmark policy shell axis — the controller
/// classifies `run_terminal_cmd` as LocalMutation, so the policy needs an
/// explicit name-level arm; `bash` is SandboxEscape already).
/// 2026-08-18 审查收口：`sh` 入名单——与设计 §3「bash/sh/cmd/pwsh」表述一致；
/// 当前非注册工具，但默认轴下必须对任何 shell 名 fail-closed。
fn is_shell_tool(tool: &str) -> bool {
    matches!(
        tool,
        "bash" | "sh" | "cmd" | "powershell" | "pwsh" | "run_terminal_cmd"
    )
}

/// Grok permission manager wrapped for the LoopHost contract.
pub struct PermissionBridge {    handle: PermissionHandle,
    /// Session working directory.
    ///
    /// REV-083-18g (2026-09-27): the retired read-scope mirror was the last
    /// production reader of this field — the bridge no longer filters on
    /// cwd (single point lives in the orz-tools sandbox). Kept because the
    /// scope tests still construct the bridge over a temp cwd and the field
    /// documents the session identity of the bridge.
    #[allow(dead_code)]
    cwd: orz_paths::AbsPathBuf,
    /// Policy for this bridge (Interactive by default; ReadOnly for codex
    /// read-only sandbox threads).
    policy: PermissionPolicy,
}

/// 测试 seam（crate 内，0p S2 复审 P1-1 全链测试用）：以 allow-all
/// manager 构造桥。生产构造走 `spawn*` 家族；私有字段仅本模块可初始化，
/// 故由本模块提供受限测试构造器（`#[cfg(test)]`，不入生产面）。
#[cfg(test)]
pub(crate) fn bridge_allow_all_for_test(dir: &std::path::Path) -> PermissionBridge {
    PermissionBridge {
        handle: PermissionHandle::allow_all(),
        cwd: orz_paths::AbsPathBuf::new(dir.to_path_buf()).unwrap(),
        policy: PermissionPolicy::Interactive,
    }
}

impl PermissionBridge {
    /// Spawn the Grok permission manager over the ACP outbound gateway.
    ///
    /// `gateway` comes from the stdio server (`AcpServer::gateway()`);
    /// `None` (headless `-p`/`--plan`) substitutes a gateway whose receiver
    /// is dropped — the interactive prompt path fails closed (IP6).
    ///
    /// Note: `spawn_permission_manager_with_hub` spawns the manager actor via
    /// `spawn_local`, so this must be called inside a `tokio::task::LocalSet`.
    pub fn spawn(
        session_id: &str,
        gateway: Option<AcpAgentGatewaySender>,
        cwd: &std::path::Path,
    ) -> Result<Self, String> {
        Self::spawn_with_hub_and_policy(
            session_id,
            gateway,
            None,
            cwd,
            PermissionPolicy::Interactive,
        )
    }

    /// Variant that threads an interactive permission transport (`hub`).
    ///
    /// `Some(hub)` routes the manager's interactive prompts through
    /// `PermissionHookTransport::request_permission` instead of the ACP
    /// gateway — the codex app-server approval surface (Phase 3 slice #12);
    /// `None` keeps the ACP-gateway behavior. The hub path has no manager
    /// built-in timeout (`request_permission_via_hub` awaits without bound),
    /// so the transport itself must bound the wait and fail closed.
    pub fn spawn_with_hub(
        session_id: &str,
        gateway: Option<AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
        cwd: &std::path::Path,
    ) -> Result<Self, String> {
        Self::spawn_with_hub_and_policy(
            session_id,
            gateway,
            hub,
            cwd,
            PermissionPolicy::Interactive,
        )
    }

    /// Variant that additionally fixes the per-session permission policy
    /// (slice #16): `spawn_with_hub` keeps the historical Interactive
    /// behavior; a codex read-only thread passes `PermissionPolicy::ReadOnly`.
    pub fn spawn_with_hub_and_policy(
        session_id: &str,
        gateway: Option<AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
        cwd: &std::path::Path,
        policy: PermissionPolicy,
    ) -> Result<Self, String> {
        let gateway = gateway.unwrap_or_else(dead_gateway);
        let abs_cwd = orz_paths::AbsPathBuf::new(cwd.to_path_buf())
            .map_err(|e| format!("cwd must be absolute: {e}"))?;
        let (handle, _events) = spawn_permission_manager_with_hub(
            acp::SessionId::new(session_id.to_string()),
            gateway,
            abs_cwd.clone(),
            ClientType::Generic,
            None,       // no managed rules — prompt policy decides
            Vec::new(), // deny_read_globs — the provider carries these for
            // subagent inheritance only; enforcement lives in
            // `access_in_scope` below (P1).
            Vec::new(), // web_fetch_allowed_domains
            true,       // initial_yolo — 默认自动审批（用户令 2026-09-26）；policy 短路仍在 yolo 之前
            None,       // client_identifier
            false,      // remember_tool_approvals
            hub,        // interactive prompter — codex app-server (slice #12)
        );
        Ok(Self {
            handle,
            cwd: abs_cwd,
            policy,
        })
    }

    /// Request permission for a tool call (LoopHost `request_permission`).
    pub async fn request(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        self.request_with_source(risk, tool, args)
            .await
            .map(|(decision, _source)| decision)
    }

    /// 0bt④（2026-09-26；原 0bu 并件）：同 [`request`](Self::request)，另回传
    /// 判定**来源**（封闭集 [`PermitSource`]）——只加观测面：判定结果与
    /// 拒绝序逐字不变，来源仅用于 journal 复核。
    pub async fn request_with_source(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<(PermitDecision, PermitSource), PermitError> {
        // Read-only sandbox: only read-class accesses may proceed — and they
        // auto-allow below. Anything else (mutation, network, escape, MCP)
        // is denied before the manager sees the request: no prompt, no
        // approval wire, no `tool_started` (slice #16; the denial is
        // journaled as a PermissionDecision by the caller).
        //
        // MCP names (`{server}__{tool}`) are always denied: their risk class
        // derives from a prefix match on the FULL name, so a server named
        // `read_*`/`list_*`/`grep*` would classify as ReadOnly and slip past
        // the policy gate (design review D2-1 — currently unreachable: no MCP
        // config in the toolset, but the gate must not depend on that).
        if self.policy == PermissionPolicy::ReadOnly
            && (risk != RiskClass::ReadOnly || tool.contains("__"))
        {
            return Ok((PermitDecision::Deny, PermitSource::Policy));
        }
        // Benchmark (2026-08-06): local edits auto-allow (the harness has no
        // client to answer prompts); network and shell-escape fail closed
        // before the manager sees the request — same shape as ReadOnly.
        // MCP names are always denied (same prefix-spoof rationale as
        // ReadOnly above). Shell tools classify LocalMutation in the
        // controller (`run_terminal_cmd` is the GrokBuild bash name), so
        // they need an explicit exclusion here — Benchmark grants file
        // edits, never shell execution.
        //
        // FUS-BENCHMARK-FULL-EXEC (2026-08-18): the `allow_shell` /
        // `allow_network` axes open the policy's allow surface only —
        // shell tools (incl. SandboxEscape aliases) auto-allow under
        // `allow_shell`, NetworkCall auto-allows under `allow_network`, and
        // the default false/false preserves the original deny semantics.
        // The audit chain and the ACAF ticket gate remain the final
        // authorization backstop.
        if let PermissionPolicy::Benchmark {
            allow_shell,
            allow_network,
        } = self.policy
        {
            match risk {
                RiskClass::ReadOnly => {}
                RiskClass::LocalMutation => {
                    if tool.contains("__") || (is_shell_tool(tool) && !allow_shell) {
                        return Ok((PermitDecision::Deny, PermitSource::Policy));
                    }
                    return Ok((PermitDecision::AllowOnce, PermitSource::Policy));
                }
                RiskClass::SandboxEscape => {
                    if allow_shell && is_shell_tool(tool) {
                        return Ok((PermitDecision::AllowOnce, PermitSource::Policy));
                    }
                    return Ok((PermitDecision::Deny, PermitSource::Policy));
                }
                RiskClass::NetworkCall => {
                    if allow_network {
                        return Ok((PermitDecision::AllowOnce, PermitSource::Policy));
                    }
                    return Ok((PermitDecision::Deny, PermitSource::Policy));
                }
            }
        }
        let access = access_kind(tool, args);
        // P1: the provider auto-allows Read regardless of path — confine it
        // to the session cwd (and away from the runtime's own `.gsa` tree)
        // before the manager sees the request.
        if !self.access_in_scope(&access) {
            return Ok((PermitDecision::Deny, PermitSource::Scope));
        }
        let update = ToolCallUpdate::new(
            ToolCallId::new(format!("call-{tool}")),
            ToolCallUpdateFields::new(),
        );
        // 0bs ⑪（2026-09-26，用户令）：唯二门禁——`browser_control` 的
        // `download` / `script` 动作须**逐次**经用户明确批准（ApprovalAlways）。
        // 请求带 `force_prompt` 进管理器：yolo／会话授予／auto 分类器／
        // policy-allow／sandbox 一切自动放行短路对它失效，直落交互提示；
        // 无客户端应答 fail-closed（提示路径既有语义）。其余动作面全面放开
        // （navigate/click/type/scroll/… 照旧自动审批）。
        let force_prompt = tool == "browser_control" && browser_action_requires_approval(args);
        // A live gateway awaits the client's answer; a silent client must not
        // stall the loop forever — bounded wait, then fail closed (P2-2).
        let request_fut = async {
            if force_prompt {
                self.handle
                    .request_force_prompt(access, update, None, None, None)
                    .await
            } else {
                self.handle.request(access, update, None, None, None).await
            }
        };
        let decision = match tokio::time::timeout(PERMISSION_PROMPT_TIMEOUT, request_fut).await
        {
            Ok(decision) => decision,
            Err(_) => {
                tracing::warn!(
                    tool,
                    "permission prompt timed out after {PERMISSION_PROMPT_TIMEOUT:?} — denying"
                );
                return Ok((PermitDecision::Deny, PermitSource::Timeout));
            }
        };
        // 0bt④: provenance is observation-only — the decision mapping below is
        // byte-for-byte the historical one (AllowOnce / Deny), only labeled.
        // 0bs ⑪: a forced-approval request is answered by the user even under
        // yolo — an allow is a user answer, never a mode artifact.
        let yolo = self.handle.is_yolo_mode() && !force_prompt;
        Ok(match decision {
            // Yolo auto-approve fast path; without yolo an allow is either the
            // manager's read auto-allow (ReadOnly class) or a client answer.
            Decision::Allow => (
                PermitDecision::AllowOnce,
                if yolo {
                    PermitSource::Yolo
                } else if force_prompt || risk != RiskClass::ReadOnly {
                    PermitSource::User
                } else {
                    PermitSource::Policy
                },
            ),
            // Headless: an interactive prompt has no client to answer it —
            // fail closed rather than stall or guess.
            Decision::Ask => (PermitDecision::Deny, PermitSource::FailClosed),
            // A client answered (reject-with-message / plain reject).
            Decision::FollowupMessage(_) | Decision::Reject(_) => {
                (PermitDecision::Deny, PermitSource::User)
            }
            Decision::PolicyDeny(_) => (PermitDecision::Deny, PermitSource::Policy),
            // Interaction terminated without an affirmative answer.
            Decision::Cancelled => (PermitDecision::Deny, PermitSource::FailClosed),
        })
    }

    /// The session's permission policy (IP2a, FIX_PLAN 2026-08-06 D-3) —
    /// consumed by the loop's name-level tool availability projection.
    pub fn policy(&self) -> PermissionPolicy {
        self.policy
    }

    /// REV-083-18g (2026-09-27)：读范围镜像段删除。
    ///
    /// Read-scope semantics were single-sourced to the orz-tools read-tool
    /// sandbox (`resources::is_session_volume_window_path` / two-stage
    /// gate) and the cwd-containment requirement was deleted earlier
    /// (MECHANICAL-AUDIT-LAYER 2026-08-24: cwd 外的读由权限策略轴与 ACAF
    /// 承担)。What remained here — the RETIRED-IN-PLACE `.gsa` mirror kept
    /// as a "double deny" for ghost whitelist forms — only duplicated the
    /// tool layer's verdict and risked drift; it is gone. The bridge now
    /// performs no read-scope filtering: everything not short-circuited by
    /// the policy arms above reaches the manager / tool layer, which holds
    /// the single-point verdict (含幽灵窗口形态恒拒)。
    fn access_in_scope(&self, _access: &AccessKind) -> bool {
        true
    }
}

/// Lexically resolve `.` / `..` components so an escaped relative path
/// compares as what it would actually touch (a bare `..` component would
/// otherwise pass `path_under` since it is still a normal component).
///
/// REV-083-18g (2026-09-27): the read-scope mirror that consumed this
/// helper is gone; it survives for the path-comparison tests only.
#[cfg(test)]
fn normalize_lexical(p: &Path) -> PathBuf {
    let mut out = std::path::PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// A gateway whose receiver is dropped immediately: the interactive prompt
/// path fails closed when it is ever reached (permission manager's
/// SendFailed → Ask → Deny; under the default yolo spawn the manager
/// auto-approves before prompting, so this backstop only binds non-yolo
/// paths).
pub(crate) fn dead_gateway() -> AcpAgentGatewaySender {
    AcpAgentGatewaySender::new(tokio::sync::mpsc::unbounded_channel().0)
}

/// Component-wise `path`-starts-with-`base`, case-insensitive on Windows
/// (canonicalized paths may differ in case from the session cwd).
///
/// REV-083-18g (2026-09-27): kept for the path-comparison tests (the
/// read-scope mirror that used it in production is gone).
#[cfg(test)]
fn path_under(base: &Path, path: &Path) -> bool {
    let base_parts = components_lower(&strip_verbatim_prefix(base));
    let path_parts = components_lower(&strip_verbatim_prefix(path));
    path_parts.len() >= base_parts.len() && base_parts.iter().zip(&path_parts).all(|(a, b)| a == b)
}

/// `dunce::canonicalize` keeps the `\\?\`-prefixed (verbatim) extended form
/// for paths it cannot safely simplify (>260 chars, reserved device names) —
/// strip that prefix so such a canonicalized target compares against the
/// plain session cwd. `\\?\UNC\server\share` maps back to `\\server\share`.
///
/// REV-083-18g (2026-09-27): test-only helper now (see `path_under`).
#[cfg(test)]
fn strip_verbatim_prefix(p: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = p.to_string_lossy();
        let stripped = s
            .strip_prefix(r"\\?\UNC\")
            .map(|rest| format!(r"\\{rest}"))
            .or_else(|| s.strip_prefix(r"\\?\").map(str::to_string))
            .unwrap_or_else(|| s.to_string());
        PathBuf::from(stripped)
    }
    #[cfg(not(windows))]
    {
        p.to_path_buf()
    }
}

/// REV-083-18g (2026-09-27): test-only helper (see `path_under`).
#[cfg(test)]
fn components_lower(p: &Path) -> Vec<String> {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect()
}

/// Mechanical tool-name → AccessKind mapping for the permission manager.
/// MCP names contain the `__` server separator; everything else maps by
/// GrokBuild client name (run_terminal_cmd / read_file / grep / web_*).
fn access_kind(tool: &str, args: &serde_json::Value) -> AccessKind {
    if tool.contains("__") {
        AccessKind::MCPTool {
            name: tool.to_string(),
            input: args.clone(),
        }
    } else if matches!(
        tool,
        "run_terminal_cmd" | "bash" | "sh" | "cmd" | "powershell" | "pwsh"
    ) {
        // GrokBuild's terminal tool is `run_terminal_cmd`; the controller's
        // risk classifier also recognizes the `bash` alias (P2-1 fix).
        AccessKind::Bash(
            args.get("command")
                .and_then(|c| c.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else if tool == "read_file" {
        AccessKind::Read(
            args.get("target_file")
                .or_else(|| args.get("path"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
        )
    } else if tool == "list_dir" {
        // Grok Build list_dir parameter is `target_directory` (not
        // `target_file`) — without it the mapped path is always None and
        // deny_read_globs matching is lost.
        AccessKind::Read(
            args.get("target_directory")
                .or_else(|| args.get("target_file"))
                .or_else(|| args.get("path"))
                .and_then(|f| f.as_str())
                .map(str::to_string),
        )
    } else if tool == "grep" {
        AccessKind::Grep {
            path: args
                .get("path")
                .and_then(|p| p.as_str())
                .map(str::to_string),
            glob: None,
        }
    } else if tool == "web_search" {
        AccessKind::WebSearch(
            args.get("query")
                .and_then(|q| q.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else if tool == "web_fetch" {
        AccessKind::WebFetch(
            args.get("url")
                .and_then(|u| u.as_str())
                .unwrap_or_default()
                .to_string(),
        )
    } else if tool == "project_doc_index" {
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the host-owned project-doc
        // index — a workspace-local read (discovery + query), Read class
        // (auto-allowed under ReadOnly/Interactive like read_file).
        AccessKind::Read(None)
    } else if tool == "browser_read" {
        // local_browser (2026-08-10): the host-owned headless-browser read —
        // pure read with no worktree side effect, same Read(None) shape as
        // project_doc_index (auto-allowed; no path restriction — the URL
        // gate lives in the browser lane itself, ADR-0010 §3.7.3, and the
        // controller mode-gates the tool; permission's job is only "read,
        // not edit"). Without this mapping the fallthrough Edit branch
        // denied every call (project_doc_index precedent, review P1-1).
        AccessKind::Read(None)
    } else if tool == "browser_control" {
        // P0-0v S4 修复 (2026-09-11)：browser_control 的 `risk_class` 已在
        // 0v S1 归入 ReadOnly（orz-loop `tool.rs`，与 browser_read 同族：导航
        // 受 URL gate 约束、读取有界，无工作区副作用），但**本表这一半当时
        // 没跟上** —— 整工具落进 else 的 `AccessKind::Edit`，于是无头部署
        // （gateway=None，fail-closed）在权限门就把每一次调用确定性拒掉
        // （0x/0v S4 实测 3/3 次 `search` 被拒；同车道同 `risk: ReadOnly` 的
        // browser_read 5/5 放行，差异只在这张表）。这与 review P1-1 记录的
        // project_doc_index / browser_read 是同形缺陷。
        //
        // 判定按**动作**分档（fail-closed 默认）：现行七种动作
        // （navigate/back/forward/refresh/wait_load/snapshot/search）全部是
        // 纯读或受 URL gate 约束的导航读 → Read(None)；未知动作与后续
        // Phase 2 交互动作（click/type/eval 一类，见 `tool.rs` 注释）仍落
        // Edit，需在放行时另行复核。
        match args.get("action").and_then(|a| a.as_str()) {
            Some(
                "navigate" | "back" | "forward" | "refresh" | "wait_load" | "snapshot" | "search",
            ) => AccessKind::Read(None),
            _ => AccessKind::Edit(format!("{tool}: {args}")),
        }
    } else if tool == "compaction_whitelist_add"        || tool == "blackboard_read"
        // P0-C orz 内嵌集成 S2 (2026-08-15): `blackboard_action_write`
        // writes ONLY the in-memory action-bar slot — no external side
        // effect (side effects happen at the mechanical issuance exit) —
        // so the permission gate auto-allows it like the whitelist write.
        || tool == "blackboard_action_write"
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): `plan_write`
        // writes ONLY the in-memory blackboard plan section — the same
        // controller-owned in-memory class as the action-bar write.
        || tool == "plan_write"
        // 0aj（2026-09-16，狗粮 run RUN-CLI-6aa999d6 摩擦 F5）：`blackboard_write`
        // （0ae D0，设计稿 docs/CONTEXT_SOFT_GATE_MODEL_PARTICIPATED_COMPRESSION_DESIGN_2026-09-15.md
        // §3 用户裁决 DP-6）写 ONLY 内存黑板的
        // plan/notes 两域——无文件、无网络、无外部副作用，与 `plan_write`
        // 同族。本 arm 缺失时它落进下方 Edit else 分支 ⇒ 无头 `-p`／死网关
        // 部署在权限门确定性 deny（journal `risk: ReadOnly` → `deny`，
        // 模型计划/笔记面恒空、`plan_write` 事件 0 条）。与 review P1-1
        // （`blackboard_read`）、0x/0v S4（`browser_control`）同形第三例：
        // 控制器 `risk_class` 与权限桥 `access_kind` 两表必须同时改。
        || tool == BLACKBOARD_WRITE_TOOL_NAME
        // 0ap（2026-09-18，设计 §4 用户裁决）：`context_compress` 纯内存
        // 压缩状态操作（知情发起 D3 模型参与压缩窗口＋滑块读数），无文件/
        // 网络/黑板外部副作用——与 `blackboard_write` 同族。沿 0aj 教训
        // 两表同批：controller `risk_class`（READ_ONLY_EXEMPT_TOOLS）与本
        // 桥 arm 必须同时登记，遍历式护栏（本文件测试）看护漏网。
        // 0am FR4（2026-09-20）：可选 `whitelist` 条目并入驻同一调用——写的
        // 仍是**内存白名单 + .gsa 侧的机械 best-effort 存档 append**（与
        // A6 §8 C.2 封存的 `compaction_whitelist_add` 同类，run 目录内、A5
        // 保留期覆盖），**不新增任何访问类型**：本 arm 的 ReadOnly 判定与
        // 两表登记纪律不变。
        || tool == CONTEXT_COMPRESS_TOOL_NAME
    {
        // Controller-owned in-memory tools (A3 blackboard_read / A6 §8 C.2
        // compaction_whitelist_add): NO external side effect — no file, no
        // network, no worktree mutation (the whitelist's .gsa archive is a
        // mechanical best-effort audit append inside the run dir). The
        // risk_class on the controller side already classes them ReadOnly
        // (declared under every policy); THIS mapping is the permission
        // gate's half of the same decision — without it `access_kind` fell
        // into the Edit else-branch and headless/dead-gateway deployments
        // denied every call deterministically (review P1-1, 2026-08-08:
        // blackboard_read had the identical latent defect since A3).
        // `Read(None)` = auto-allowed (read-class), no path restriction.
        AccessKind::Read(None)
    } else {
        AccessKind::Edit(format!("{tool}: {args}"))
    }
}

/// 0bs ⑪（2026-09-26，用户令）：`browser_control` 的唯二门禁动作——下载与
/// 脚本执行。二者是整条浏览器车道里仅存的、能改变系统状态或执行模型提供
/// 的代码的动作面，须**逐次**经用户明确批准（ApprovalAlways：桥把请求带
/// `force_prompt` 送进管理器，一切自动放行短路对其失效；无客户端应答
/// fail-closed）。其余动作面（navigate/type/key/click/scroll/…）照旧自动
/// 审批。判定只按动作名，不看参数——参数校验在 local_browser 工具层。
fn browser_action_requires_approval(args: &serde_json::Value) -> bool {
    matches!(
        args.get("action").and_then(|a| a.as_str()),
        Some("download") | Some("script")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-permission-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// The permission manager actor spawns local tasks — everything runs
    /// inside one LocalSet per test. `None` gateway = dropped receiver
    /// (fail-closed), exactly the headless `-p`/`--plan` configuration.
    async fn with_bridge<F, Fut, T>(f: F) -> T
    where
        F: FnOnce(PermissionBridge) -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let bridge = PermissionBridge::spawn("sess-test", None, &dir).unwrap();
                f(bridge).await
            })
            .await
    }

    /// RS-07（0aq，2026-09-19）：**跨真实声明面的 deny 路径遍历**——
    /// 历史缺陷族＝控制器 `risk_class` 表与宿主 `access_kind` 表脱同步
    /// （project_doc_index／browser_read／browser_control／blackboard_write
    /// 四例先例，0aj 复盘）。本测试对探针声明面
    /// [`orz_loop::tool_probe::WORK_TOOLS`] 的**每一个**工具取控制器侧
    /// 单源分类（`ToolDispatcher::risk_class`）后驱动权限桥，断言：
    /// ① 判定**全总**（每工具都有 allow/deny，不悬挂不 panic 不漏分类）；
    /// ② ReadOnly 类工具在 ReadOnly 策略下自动放行（0aj 缺陷形态的
    /// 反向钉——漏 arm 即报红）；③ 其余各类在无客户端 fail-closed 桥上
    /// 确定性；LocalMutation 的 manager 自动放行（0b 决策表现行行为）与
    /// 其余 fail-closed 拒绝均须**两遍一致**——分类与桥臂脱同步或判定
    /// 不确定即报红。
    #[tokio::test]
    async fn permission_bridge_decides_every_declared_work_tool() {
        async fn traverse(dir: &std::path::Path, tag: &str) -> Vec<(String, String)> {
            tokio::task::LocalSet::new()
                .run_until(async move {
                    let bridge =
                        PermissionBridge::spawn(&format!("sess-rs07-{tag}"), None, dir).unwrap();
                    let mut results = Vec::new();
                    for name in orz_loop::tool_probe::WORK_TOOLS {
                        let risk = orz_loop::tool::ToolDispatcher::risk_class(name);
                        let decision = bridge
                            .request(risk, name, &serde_json::json!({}))
                            .await
                            .unwrap_or_else(|e| {
                                panic!("permission bridge error for {name}: {e:?}")
                            });
                        results.push((name.to_string(), format!("{decision:?}")));
                    }
                    results
                })
                .await
        }
        let dir = test_dir();
        let first = traverse(&dir, "a").await;
        let second = traverse(&dir, "b").await;
        assert_eq!(
            first.len(),
            orz_loop::tool_probe::WORK_TOOLS.len(),
            "every declared tool must get a total decision"
        );
        assert_eq!(first, second, "decisions must be deterministic");
        for (name, decision) in &first {
            let risk = orz_loop::tool::ToolDispatcher::risk_class(name);
            if risk == RiskClass::ReadOnly {
                assert_eq!(
                    decision, "AllowOnce",
                    "{name} is ReadOnly-classed but the bridge did not auto-allow (0aj desync family)"
                );
            }
        }
    }

    #[tokio::test]
    async fn ask_auto_approves_under_default_yolo() {
        // 用户令 2026-09-26「直接开auto mode就行」：生产 spawn 初始即 yolo，
        // Bash（SandboxEscape）等 Ask 类不再等弹窗/超时——manager 在 yolo
        // 快路径自动放行（policy 短路与无桥 fail-closed 默认不受影响）。
        // 0bt④：来源观测面同址钉——yolo 快路必须自报 `yolo`。
        let (decision, source) = with_bridge(|b| async move {
            b.request_with_source(
                RiskClass::SandboxEscape,
                "run_terminal_cmd",
                &serde_json::json!({"command": "dir"}),
            )
            .await
            .unwrap()
        })
        .await;
        assert_eq!(
            decision,
            PermitDecision::AllowOnce,
            "default-yolo spawn must auto-approve Ask-class requests"
        );
        assert_eq!(
            source,
            PermitSource::Yolo,
            "0bt④: the auto-approve fast path must report source=yolo"
        );
    }

    #[tokio::test]
    async fn read_request_auto_allows() {
        // Read-only access is auto-approved by the permission manager
        // (low-risk reads never prompt); Bash without an interactive client
        // fails closed. Risk layering is exactly the point of IP6.
        let decision = with_bridge(|b| async move {
            b.request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": "a.txt"}),
            )
            .await
            .unwrap()
        })
        .await;
        assert_eq!(decision, PermitDecision::AllowOnce);
    }

    #[test]
    fn access_kind_mapping() {
        assert!(matches!(
            access_kind("read_file", &serde_json::json!({"target_file": "a"})),
            AccessKind::Read(Some(_))
        ));
        assert!(matches!(
            access_kind("run_terminal_cmd", &serde_json::json!({"command": "ls"})),
            AccessKind::Bash(_)
        ));
        // P2-1: the `bash` alias (and other shell names) must classify as
        // Bash — not fall through to the generic `Edit` bucket.
        assert!(matches!(
            access_kind("bash", &serde_json::json!({"command": "dir"})),
            AccessKind::Bash(_)
        ));
        assert!(matches!(
            access_kind("powershell", &serde_json::json!({"command": "ls"})),
            AccessKind::Bash(_)
        ));
        assert!(matches!(
            access_kind("web_search", &serde_json::json!({"query": "x"})),
            AccessKind::WebSearch(_)
        ));
        assert!(matches!(
            access_kind("server__tool", &serde_json::json!({})),
            AccessKind::MCPTool { .. }
        ));
        // Review P1-1 (2026-08-08): controller-owned in-memory tools map to
        // Read(None) — auto-allowed, no path restriction — NOT the Edit
        // else-branch (headless deployments denied them deterministically).
        assert!(matches!(
            access_kind(
                "compaction_whitelist_add",
                &serde_json::json!({"content": "x"})
            ),
            AccessKind::Read(None)
        ));
        assert!(matches!(
            access_kind("blackboard_read", &serde_json::json!({"section": "plan"})),
            AccessKind::Read(None)
        ));
        assert!(matches!(
            access_kind(
                "plan_write",
                &serde_json::json!({"plan": {"plan_id": "p", "goal": "g", "steps": []}})
            ),
            AccessKind::Read(None)
        ));
        // 0aj（2026-09-16，狗粮 run RUN-CLI-6aa999d6 摩擦 F5 根因钉）：
        // `blackboard_write` 写的是**内存黑板**（plan/notes），无文件／网络／
        // 外部副作用，控制器侧 `risk_class` 已定 ReadOnly（0ae D0）。本表
        // 缺 arm 时它会落进 Edit else 分支 ⇒ 无头 `-p`/死网关部署在权限门
        // 确定性拒掉每一次调用（journal：`risk: ReadOnly` → `deny`；模型
        // 计划/笔记面恒空）。与 review P1-1（`blackboard_read`）、0x/0v S4
        // （`browser_control`）**同形第三例**——两表必须同时改。
        assert!(matches!(
            access_kind(
                "blackboard_write",
                &serde_json::json!({"section": "notes", "content": "笔记"})
            ),
            AccessKind::Read(None)
        ));
        // P0-0v S4 修复 (2026-09-11): browser_control must NOT fall into the
        // Edit else-branch — headless deployments deny Edit deterministically
        // (0x/0v S4: 3/3 `search` calls denied). Every现行 action is a read or
        // a URL-gated navigation read.
        for action in [
            "navigate",
            "back",
            "forward",
            "refresh",
            "wait_load",
            "snapshot",
            "search",
        ] {
            assert!(
                matches!(
                    access_kind("browser_control", &serde_json::json!({"action": action})),
                    AccessKind::Read(None)
                ),
                "browser_control action {action} must map to Read(None)"
            );
        }
        // Fail-closed: unknown / future Phase 2 interactive actions stay Edit.
        assert!(matches!(
            access_kind("browser_control", &serde_json::json!({"action": "click"})),
            AccessKind::Edit(_)
        ));
        assert!(matches!(
            access_kind("browser_control", &serde_json::json!({})),
            AccessKind::Edit(_)
        ));
    }

    /// 跨表护栏（0v S4 修复同刀；0aj-review 2026-09-16 改**表驱动**）：
    /// 控制器侧 `risk_class`(orz-loop `tool.rs`) 判为 ReadOnly 的工具，宿主侧
    /// 这里**不得**落进 `Edit` 兜底——否则无头部署会在权限门确定性拒绝
    /// （`project_doc_index` / `browser_read` / `browser_control` / 第四例
    /// `blackboard_write` 都是这一形态）。
    ///
    /// 0aj 的漏网直接原因就是本护栏当时是**手写样本表**、样本漏列该工具。
    /// 现在遍历控制器侧单一源
    /// [`ToolDispatcher::READ_ONLY_EXEMPT_TOOLS`]，并断言「代表参数表恰好覆盖
    /// 该表」——新增显式 ReadOnly 工具只在那里登记一处，本护栏自动覆盖：
    /// 缺 `access_kind` arm ⇒ 落 `Edit` ⇒ 此处报红；漏补代表参数 ⇒ 覆盖断言报红。
    #[test]
    fn read_only_tools_never_fall_into_the_edit_bucket() {
        use orz_loop::tool::ToolDispatcher;

        // 单一源表每一项的代表调用参数（工具 → 一次合法代表调用）。
        let samples: &[(&str, serde_json::Value)] = &[
            ("blackboard_read", serde_json::json!({"section": "plan"})),
            ("blackboard_action_write", serde_json::json!({"order": "x"})),
            ("plan_write", serde_json::json!({"plan": {}})),
            (
                BLACKBOARD_WRITE_TOOL_NAME,
                serde_json::json!({"section": "plan", "content": "x"}),
            ),
            // 0ap：压缩窗口请求无参数；空对象即代表调用形态。
            (CONTEXT_COMPRESS_TOOL_NAME, serde_json::json!({})),
            (
                "compaction_whitelist_add",
                serde_json::json!({"content": "x"}),
            ),
            ("project_doc_index", serde_json::json!({"query": "x"})),
            (
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
            ),
            (
                "browser_control",
                serde_json::json!({"action": "search", "query": "x"}),
            ),
            (
                "browser_control",
                serde_json::json!({"action": "navigate", "url": "https://example.com"}),
            ),
        ];
        // 覆盖率断言（0aj-review）：样本表必须恰好覆盖单一源表——多一个
        // 名字说明单一源已改名/删除，少一个说明漏补代表参数。
        let sampled: std::collections::BTreeSet<&str> =
            samples.iter().map(|(tool, _)| *tool).collect();
        let single_source: std::collections::BTreeSet<&str> =
            ToolDispatcher::READ_ONLY_EXEMPT_TOOLS
                .iter()
                .copied()
                .collect();
        assert_eq!(
            sampled, single_source,
            "guardrail samples must cover ToolDispatcher::READ_ONLY_EXEMPT_TOOLS exactly \
             (add representative args for any new entry)"
        );
        for (tool, args) in samples {
            assert_eq!(
                ToolDispatcher::risk_class(tool),
                orz_loop::host::RiskClass::ReadOnly,
                "sample {tool} is expected to be ReadOnly on the controller side"
            );
            assert!(
                !matches!(access_kind(tool, args), AccessKind::Edit(_)),
                "{tool} is ReadOnly on the controller side but maps to the Edit bucket here \
                 (headless deployments would deny it deterministically)"
            );
        }
        // 前缀族（`read_` / `list_` / `grep` / `search`）不在单一源表内：它们是
        // 规则式（开放集）。引入新的前缀族工具名时必须同样在此补宿主映射；
        // 未知名字保持 `Edit` fail-closed（见本文件 `access_kind` 末尾 else）。
        for (tool, args) in [
            ("read_file", serde_json::json!({"target_file": "a"})),
            ("list_dir", serde_json::json!({"target_directory": "/tmp"})),
            ("grep", serde_json::json!({"pattern": "x"})),
        ]
        .iter()
        {
            assert_eq!(
                ToolDispatcher::risk_class(tool),
                orz_loop::host::RiskClass::ReadOnly,
                "prefix-family sample {tool} is expected to be ReadOnly"
            );
            assert!(
                !matches!(access_kind(tool, args), AccessKind::Edit(_)),
                "prefix-family {tool} is ReadOnly on the controller side but maps to the \
                 Edit bucket here (headless deployments would deny it deterministically)"
            );
        }
        // 反空转控制（0aj-review）：未映射的工具名必须**仍然落 `Edit`**
        // （fail-closed 是这张表的设计），否则本护栏在「access_kind 无条件
        // 放行」的实现下会退化成空转——四条同形缺陷的保护会静默消失。
        assert!(
            matches!(
                access_kind("blackboard_write_unknown_probe", &serde_json::json!({})),
                AccessKind::Edit(_)
            ),
            "an unmapped tool name must stay in the Edit bucket (fail-closed); \
             otherwise this guardrail cannot discriminate"
        );
    }

    // ── P1 scope enforcement ─────────────────────────────────────────────

    /// Windows 盘符路径语义（Linux 上 `D:\` 是相对路径，行为不同）。
    #[cfg(windows)]
    #[test]
    fn path_under_component_wise_and_case_insensitive() {
        assert!(path_under(
            Path::new("D:\\CLI\\orz"),
            Path::new("D:\\CLI\\orz\\a\\b")
        ));
        assert!(path_under(
            Path::new("D:\\CLI\\orz"),
            Path::new("d:\\cli\\ORZ\\x")
        ));
        // Windows canonicalize returns `\\?\`-prefixed paths — must compare
        // equal to the plain cwd (regression: real files were all denied).
        assert!(path_under(
            Path::new("D:\\CLI\\orz"),
            Path::new(r"\\?\D:\CLI\orz\rust-toolchain.toml")
        ));
        assert!(path_under(
            Path::new(r"\\?\D:\CLI\orz"),
            Path::new(r"\\?\D:\CLI\orz\a")
        ));
        // Boundary: a sibling with a shared prefix must NOT match.
        assert!(!path_under(
            Path::new("D:\\CLI\\orz"),
            Path::new("D:\\CLI\\orz2\\x")
        ));
        assert!(!path_under(
            Path::new("D:\\CLI\\orz"),
            Path::new("C:\\CLI\\orz\\x")
        ));
    }

    /// Bridge-scope unit test: build the bridge over a real temp cwd and
    /// check `access_in_scope` directly (no manager actor involved).
    fn bridge_over(dir: &std::path::Path) -> PermissionBridge {
        PermissionBridge {
            handle: PermissionHandle::allow_all(),
            cwd: orz_paths::AbsPathBuf::new(dir.to_path_buf()).unwrap(),
            policy: PermissionPolicy::Interactive,
        }
    }

    #[tokio::test]
    async fn read_scope_enforced_before_manager() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::create_dir_all(dir.join(".gsa").join("runs")).unwrap();
        std::fs::create_dir_all(dir.join(".gsa").join("session").join("terminal")).unwrap();
        // Existing file → `canonicalize` succeeds → the Windows `\\?\` prefix
        // path is exercised (regression: prefixed paths were all denied).
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        std::fs::write(
            dir.join(".gsa")
                .join("session")
                .join("terminal")
                .join("ord-000001.log"),
            "full output",
        )
        .unwrap();
        let outside = test_dir(); // sibling temp dir — outside cwd
        let bridge = bridge_over(&dir);

        async fn read_req(bridge: &PermissionBridge, path: &str) -> PermitDecision {
            bridge
                .request(
                    RiskClass::ReadOnly,
                    "read_file",
                    &serde_json::json!({"target_file": path}),
                )
                .await
                .unwrap()
        }

        // Relative path inside cwd → allowed (auto).
        assert_eq!(
            read_req(&bridge, "inside/a.txt").await,
            PermitDecision::AllowOnce
        );
        // Absolute path inside cwd → allowed.
        let abs_in = dir.join("inside").join("a.txt");
        assert_eq!(
            read_req(&bridge, &abs_in.to_string_lossy()).await,
            PermitDecision::AllowOnce
        );
        // MECHANICAL-AUDIT-LAYER (2026-08-24): `..` escaping cwd → allowed
        // (cwd 包含性已删除；仅 `.gsa` 证据面仍不可见)。
        assert_eq!(
            read_req(&bridge, "../outside-escape.txt").await,
            PermitDecision::AllowOnce
        );
        // Absolute path outside cwd → allowed (the P1 hole is closed in the
        // opposite direction — 越权读由权限策略轴与 ACAF 承担).
        assert_eq!(
            read_req(&bridge, &outside.join("secret.txt").to_string_lossy()).await,
            PermitDecision::AllowOnce
        );
        // The runtime's own `.gsa` tree → ALLOWED at the bridge（0p S2
        // P1-1 修复，2026-09-07，ADR-0010 §14.61 设计 B）：内部区读判定权
        // 单点归 orz-tools 两段门（首读通知→二读放行；凭据区恒拒），桥
        // 镜像让路——若镜像先拒，两段门在所有带桥生产路径不可达且拒绝
        // 走 event-less 路径（W2 D-3 原形）。旧断言 `Deny` 随镜像退役。
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("runs")
                    .join("events.jsonl")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::AllowOnce,
            "internal-region reads yield to the tool-layer two-stage gate"
        );
        // GAP-RUN-TESTS (2026-08-11): the run_tests output artifact is the
        // model's permission-gated window into the full test output — the
        // EXACT filename under `.gsa` is allowed...
        std::fs::write(dir.join(".gsa").join("run_tests_output.txt"), "1 passed").unwrap();
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("run_tests_output.txt")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::AllowOnce,
            "run_tests output artifact readable per ADR §3.8.3"
        );
        // ...and near-miss filenames are not the window（0p S2 P1-1 后桥
        // 镜像不再一刀切拒 `.gsa`，但窗口守卫臂仍按精确文件名放行——
        // `.bak` 形态不命中窗口臂，落入让路臂交由工具层判决；桥面无
        // 特权放行，tool 层对其拒/门照旧）。
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("run_tests_output.txt.bak")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::AllowOnce,
            "non-window .gsa reads yield to the tool-layer gate (no bridge privilege)"
        );
        // OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计
        // §3.2): terminal output logs under `session/terminal/` are the
        // model's permission-gated window into the FULL terminal output —
        // the truncation receipt points at them and instructs read_file.
        let term_log = dir
            .join(".gsa")
            .join("session")
            .join("terminal")
            .join("ord-000001.log");
        assert_eq!(
            read_req(&bridge, &term_log.to_string_lossy()).await,
            PermitDecision::AllowOnce,
            "terminal output log readable per OUTPUT-DEGENERATION-GUARD 补读闭环"
        );
        assert_eq!(
            read_req(&bridge, ".gsa/session/terminal/ord-000001.log").await,
            PermitDecision::AllowOnce,
            "relative terminal output log readable"
        );
        // grep shares the same scope rule for the terminal log path.
        assert_eq!(
            bridge
                .request(
                    RiskClass::ReadOnly,
                    "grep",
                    &serde_json::json!({"path": term_log.to_string_lossy(), "pattern": "x"}),
                )
                .await
                .unwrap(),
            PermitDecision::AllowOnce
        );
        // `..` escaping out of `session/terminal/` into other `.gsa`
        // internals（0p S2 P1-1 后桥面让路：词法折叠后非窗口形态 →
        // AllowOnce；工具层两段门对该路径恒拒——keystore 属 B1 凭据区
        // CredentialsDenied，通知亦不放开。安全锁在 orz-tools 判决单点
        // 与 GAP-GSA-SYMLINK-STALE-TEST 回归，桥不再持第二判定权威）。
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("session")
                    .join("terminal")
                    .join("..")
                    .join("keystore")
                    .join("installation-key.bin")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::AllowOnce,
            "bridge yields; tool-layer gate permanently denies the credentials region"
        );
        // Non-`.log` siblings under `session/terminal/` are not whitelisted
        // at the bridge（0p S2 P1-1 后交由工具层两段门：内部区首读通知/
        // 二读放行，桥面不再预判）。
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("session")
                    .join("terminal")
                    .join("ord-000001.log.bak")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::AllowOnce,
            "bridge yields non-window shapes to the tool-layer gate"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }

    #[tokio::test]
    async fn non_read_accesses_skip_scope_check() {
        let dir = test_dir();
        let bridge = bridge_over(&dir);
        // Bash and web accesses pass through to the manager regardless of
        // scope (they fail closed via Ask → Deny anyway, and interactive
        // prompts land in Phase 3).
        assert!(bridge.access_in_scope(&AccessKind::Bash("dir".to_string())));
        assert!(bridge.access_in_scope(&AccessKind::WebFetch("https://x".to_string())));
        assert!(bridge.access_in_scope(&AccessKind::Edit("write: x".to_string())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── ReadOnly policy (slice #16) ──────────────────────────────────────

    /// Build a bridge with an allow-all manager under the given policy — the
    /// manager would auto-allow *everything*, so any Deny proves the
    /// policy/short-circuit decided it before the manager saw the request.
    fn bridge_with_policy(dir: &std::path::Path, policy: PermissionPolicy) -> PermissionBridge {
        PermissionBridge {
            handle: PermissionHandle::allow_all(),
            cwd: orz_paths::AbsPathBuf::new(dir.to_path_buf()).unwrap(),
            policy,
        }
    }

    /// Review P1-1 (2026-08-08): controller-owned in-memory tools
    /// (`blackboard_read` / `compaction_whitelist_add`) auto-allow under
    /// EVERY policy — ReadOnly/Benchmark policy short-circuits pass them
    /// (ReadOnly risk class), the bridge maps them to `Read(None)` (no path
    /// restriction → `access_in_scope` passes), and the read-class manager
    /// auto-allows (the provider's SAFE_COMMAND path — the same inherited
    /// behavior `read_file` exercises in production). Without the `Read(None)`
    /// mapping they fell into the Edit else-branch and headless deployments
    /// denied them deterministically.
    #[tokio::test]
    async fn controller_owned_tools_auto_allow_under_every_policy() {
        let dir = test_dir();
        for policy in [
            PermissionPolicy::Interactive,
            PermissionPolicy::ReadOnly,
            PermissionPolicy::Benchmark {
                allow_shell: false,
                allow_network: false,
            },
        ] {
            let bridge = bridge_with_policy(&dir, policy);
            for (tool, args) in [
                ("blackboard_read", serde_json::json!({"section": "plan"})),
                (
                    "compaction_whitelist_add",
                    serde_json::json!({"content": "任务背景"}),
                ),
                // 0aj（2026-09-16，狗粮 run RUN-CLI-6aa999d6 摩擦 F5）：同族
                // 第三例——`risk_class` 侧已把 `blackboard_write` 定为
                // ReadOnly（0ae D0），但本表的 arm 缺失 ⇒ 无头 `-p` 车道
                // 每次调用确定性 deny（journal `risk: ReadOnly` →
                // `decision: deny`），模型计划/笔记面恒空。
                (
                    BLACKBOARD_WRITE_TOOL_NAME,
                    serde_json::json!({"section": "notes", "content": "笔记"}),
                ),
                // 0ap：纯内存压缩状态操作——ReadOnly 类全策略自动放行
                //（权限桥放行链钉，S1 判据④）。
                (CONTEXT_COMPRESS_TOOL_NAME, serde_json::json!({})),
            ] {
                let decision = bridge
                    .request(RiskClass::ReadOnly, tool, &args)
                    .await
                    .unwrap();
                assert_eq!(
                    decision,
                    PermitDecision::AllowOnce,
                    "{tool} under {policy:?}"
                );
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn read_only_policy_denies_non_read_before_manager() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        let outside = test_dir(); // sibling temp dir — outside cwd
        let bridge = bridge_with_policy(&dir, PermissionPolicy::ReadOnly);

        // Non-read risk classes → denied without touching the manager
        // (allow_all would have said yes to every one of these). NOTE: the
        // controller classifies `run_terminal_cmd` as LocalMutation (only the
        // `bash` alias is SandboxEscape — orz-loop/tool.rs); either label
        // exercises the same short-circuit, and the real-link risk labels are
        // pinned by the codex_app E2E tests (review D2-4).
        for (risk, tool, args) in [
            (
                RiskClass::SandboxEscape,
                "bash",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::NetworkCall,
                "web_fetch",
                serde_json::json!({"url": "https://x"}),
            ),
            (
                RiskClass::LocalMutation,
                "search_replace",
                serde_json::json!({"path": "a.txt"}),
            ),
        ] {
            let (decision, source) = bridge.request_with_source(risk, tool, &args).await.unwrap();
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "{tool} must be denied under ReadOnly policy"
            );
            // 0bt④：策略门短路必须自报来源 `policy`（观测面）。
            assert_eq!(
                source,
                PermitSource::Policy,
                "{tool}: ReadOnly short-circuit must report source=policy"
            );
        }

        // MCP tools are always denied under ReadOnly — even when the server
        // prefix would classify as read-class (the risk classifier prefix-
        // matches the FULL name, so a server named `read_*`/`list_*`/`grep*`
        // would otherwise slip past the policy gate; review D2-1). The `__`
        // separator marks MCP names (access_kind below).
        let (decision, source) = bridge
            .request_with_source(
                RiskClass::ReadOnly,
                "read_server__extract",
                &serde_json::json!({}),
            )
            .await
            .unwrap();
        assert_eq!(
            decision,
            PermitDecision::Deny,
            "MCP names must never bypass the read-only gate"
        );
        assert_eq!(source, PermitSource::Policy);

        // Read within the cwd → auto-allowed (the manager's allow_all path).
        let decision = bridge
            .request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": "inside/a.txt"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);

        // blackboard_read — the controller classifies it ReadOnly via an
        // explicit name registration (the `blackboard_` prefix misses the
        // read_/list_/grep/search prefixes; 2026-08-08 review closure,
        // orz-loop/tool.rs). It must auto-allow exactly like read_file —
        // ReadOnly sandboxes declared it AND must be able to call it.
        let decision = bridge
            .request(
                RiskClass::ReadOnly,
                "blackboard_read",
                &serde_json::json!({"section": "plan"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);

        // MECHANICAL-AUDIT-LAYER (2026-08-24): read outside the cwd is now
        // auto-allowed (cwd 包含性删除；.gsa 证据面仍由 scope check 拒绝).
        let decision = bridge
            .request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": outside.join("x.txt").to_string_lossy()}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }

    // ── Benchmark policy (2026-08-06 polyglot harness) ──────────────────

    #[tokio::test]
    async fn benchmark_default_policy_allows_local_mutation_denies_network_and_shell() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        // Default fail-closed axes: `Benchmark{allow_shell:false,
        // allow_network:false}` must behave exactly like the pre-parameter
        // unit variant (FUS-BENCHMARK-FULL-EXEC old-semantics guard).
        let bridge = bridge_with_policy(
            &dir,
            PermissionPolicy::Benchmark {
                allow_shell: false,
                allow_network: false,
            },
        );

        // Local edits auto-allow — the loop must be able to modify files
        // with no client to answer prompts.
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "search_replace",
                &serde_json::json!({"path": "a.txt"}),
            )
            .await
            .unwrap();
        assert_eq!(
            decision,
            PermitDecision::AllowOnce,
            "search_replace must auto-allow under Benchmark policy"
        );

        // Shell execution stays fail-closed under the default axes —
        // `run_terminal_cmd` classifies LocalMutation in the controller but
        // is the GrokBuild bash name; Benchmark grants file edits, never
        // shell, unless `allow_shell` is set.
        for (risk, tool, args) in [
            (
                RiskClass::SandboxEscape,
                "bash",
                serde_json::json!({"command": "dir"}),
            ),
            (
                // 2026-08-18 审查收口：`sh` 与其它 shell 名在默认轴一律 deny
                // （设计 §3 名单一致性；当前非注册工具，fail-closed 兜底）。
                RiskClass::LocalMutation,
                "sh",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::LocalMutation,
                "run_terminal_cmd",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::NetworkCall,
                "web_fetch",
                serde_json::json!({"url": "https://x"}),
            ),
        ] {
            let decision = bridge.request(risk, tool, &args).await.unwrap();
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "{tool} must be denied under Benchmark policy"
            );
        }

        // MCP names never auto-allow mutation.
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "write_server__write",
                &serde_json::json!({}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::Deny);

        // Reads still auto-allow (manager path).
        let decision = bridge
            .request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": "inside/a.txt"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── FUS-BENCHMARK-FULL-EXEC (2026-08-18): shell/network axes ────────

    #[tokio::test]
    async fn benchmark_shell_axis_allows_shell_keeps_network_fail_closed() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        let bridge = bridge_with_policy(
            &dir,
            PermissionPolicy::Benchmark {
                allow_shell: true,
                allow_network: false,
            },
        );

        // Shell tools auto-allow under `allow_shell` — `run_terminal_cmd`
        // classifies LocalMutation in the controller; `bash` is
        // SandboxEscape. Both open (the console action bar routes the
        // former; the latter is the direct-call alias).
        for (risk, tool, args) in [
            (
                RiskClass::LocalMutation,
                "run_terminal_cmd",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::LocalMutation,
                "powershell",
                serde_json::json!({"command": "ls"}),
            ),
            (
                // 2026-08-18 审查收口：`sh` 在 allow_shell 下 AllowOnce
                // （与设计 §3 名单一致；access_kind 早已按 Bash 映射）。
                RiskClass::LocalMutation,
                "sh",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::SandboxEscape,
                "bash",
                serde_json::json!({"command": "dir"}),
            ),
        ] {
            let decision = bridge.request(risk, tool, &args).await.unwrap();
            assert_eq!(
                decision,
                PermitDecision::AllowOnce,
                "{tool} must auto-allow under Benchmark{{allow_shell:true}}"
            );
        }

        // Network stays fail-closed without `allow_network`.
        let decision = bridge
            .request(
                RiskClass::NetworkCall,
                "web_fetch",
                &serde_json::json!({"url": "https://x"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::Deny);

        // Local file edits still auto-allow; MCP names never do.
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "search_replace",
                &serde_json::json!({"path": "a.txt"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "write_server__write",
                &serde_json::json!({}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::Deny);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn benchmark_network_axis_allows_network_keeps_shell_fail_closed() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        let bridge = bridge_with_policy(
            &dir,
            PermissionPolicy::Benchmark {
                allow_shell: false,
                allow_network: true,
            },
        );

        // Network calls auto-allow under `allow_network` (the direct-call
        // web surface; the console default face has no web actions).
        for (tool, args) in [
            ("web_fetch", serde_json::json!({"url": "https://x"})),
            ("web_search", serde_json::json!({"query": "x"})),
        ] {
            let decision = bridge
                .request(RiskClass::NetworkCall, tool, &args)
                .await
                .unwrap();
            assert_eq!(
                decision,
                PermitDecision::AllowOnce,
                "{tool} must auto-allow under Benchmark{{allow_network:true}}"
            );
        }

        // Shell stays fail-closed without `allow_shell`.
        for (risk, tool, args) in [
            (
                RiskClass::LocalMutation,
                "run_terminal_cmd",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::LocalMutation,
                "sh",
                serde_json::json!({"command": "dir"}),
            ),
            (
                RiskClass::SandboxEscape,
                "bash",
                serde_json::json!({"command": "dir"}),
            ),
        ] {
            let decision = bridge.request(risk, tool, &args).await.unwrap();
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "{tool} must be denied without allow_shell"
            );
        }

        // Local file edits still auto-allow; MCP names never do.
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "search_replace",
                &serde_json::json!({"path": "a.txt"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);
        let decision = bridge
            .request(
                RiskClass::LocalMutation,
                "read_server__extract",
                &serde_json::json!({}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::Deny);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn interactive_policy_is_default_and_unaffected() {
        // Default policy is Interactive (the zero-arg constructor path) —
        // `spawn` and `spawn_with_hub` must not change historical behavior.
        assert_eq!(PermissionPolicy::default(), PermissionPolicy::Interactive);

        let dir = test_dir();
        let bridge = bridge_with_policy(&dir, PermissionPolicy::Interactive);

        // Under Interactive, a bash request reaches the allow-all manager →
        // AllowOnce (proof: the policy short-circuit is policy-gated, not
        // tool-gated).
        let decision = bridge
            .request(
                RiskClass::SandboxEscape,
                "run_terminal_cmd",
                &serde_json::json!({"command": "dir"}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::AllowOnce);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Windows 盘符路径语义（Linux 上 `D:\` 是相对路径，行为不同）。
    #[cfg(windows)]
    #[test]
    fn normalize_lexical_resolves_escapes() {
        let base = Path::new("D:\\CLI\\orz");
        assert_eq!(
            normalize_lexical(&base.join("..").join("x")),
            PathBuf::from("D:\\CLI\\x")
        );
        assert_eq!(
            normalize_lexical(&base.join("a").join("..").join("b")),
            PathBuf::from("D:\\CLI\\orz\\b")
        );
    }
}
