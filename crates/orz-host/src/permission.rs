//! Permission bridge — Grok permission manager behind the LoopHost contract.
//!
//! IP6 (hard-gate @ orz-workspace::permission) is the single allowed
//! assurance injection into kept Grok providers. This bridge walks public
//! seams only:
//! - yolo stays off: headless spawn with `initial_yolo = false`, no auto
//!   classifier — no way to sneak past the decision point;
//! - interactive prompting (`PermissionHookTransport`) lands with the TUI
//!   (Phase 3); headless `Ask` fails closed to `Deny`;
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
use orz_loop::host::{PermitDecision, PermitError, RiskClass};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionPolicy {
    /// Normal prompt-decides behavior (reads auto-allow; the rest prompts).
    #[default]
    Interactive,
    /// Read-only sandbox — mutation/network/escape requests fail closed
    /// without prompting.
    ReadOnly,
    /// Headless benchmark — read + local file edits auto-allow; network and
    /// shell-escape fail closed without prompting.
    Benchmark,
}

/// Shell-execution tool names (Benchmark policy exclusion — the controller
/// classifies `run_terminal_cmd` as LocalMutation, so the policy needs an
/// explicit name-level exclusion; `bash` is SandboxEscape already).
fn is_shell_tool(tool: &str) -> bool {
    matches!(
        tool,
        "bash" | "cmd" | "powershell" | "pwsh" | "run_terminal_cmd"
    )
}

/// Grok permission manager wrapped for the LoopHost contract.
pub struct PermissionBridge {
    handle: PermissionHandle,
    /// Session working directory — the scope Read auto-allow is confined to.
    cwd: orz_paths::AbsPathBuf,
    /// Policy for this bridge (Interactive by default; ReadOnly for codex
    /// read-only sandbox threads).
    policy: PermissionPolicy,
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
            None,       // no managed rules — prompt policy decides; headless Ask → Deny
            Vec::new(), // deny_read_globs — the provider carries these for
            // subagent inheritance only; enforcement lives in
            // `access_in_scope` below (P1).
            Vec::new(), // web_fetch_allowed_domains
            false,      // initial_yolo — headless: yolo never on (IP6)
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
            return Ok(PermitDecision::Deny);
        }
        // Benchmark (2026-08-06): local edits auto-allow (the harness has no
        // client to answer prompts); network and shell-escape fail closed
        // before the manager sees the request — same shape as ReadOnly.
        // MCP names are always denied (same prefix-spoof rationale as
        // ReadOnly above). Shell tools classify LocalMutation in the
        // controller (`run_terminal_cmd` is the GrokBuild bash name), so
        // they need an explicit exclusion here — Benchmark grants file
        // edits, never shell execution.
        if self.policy == PermissionPolicy::Benchmark {
            match risk {
                RiskClass::ReadOnly => {}
                RiskClass::LocalMutation => {
                    if tool.contains("__") || is_shell_tool(tool) {
                        return Ok(PermitDecision::Deny);
                    }
                    return Ok(PermitDecision::AllowOnce);
                }
                RiskClass::NetworkCall | RiskClass::SandboxEscape => {
                    return Ok(PermitDecision::Deny);
                }
            }
        }
        let access = access_kind(tool, args);
        // P1: the provider auto-allows Read regardless of path — confine it
        // to the session cwd (and away from the runtime's own `.gsa` tree)
        // before the manager sees the request.
        if !self.access_in_scope(&access) {
            return Ok(PermitDecision::Deny);
        }
        let update = ToolCallUpdate::new(
            ToolCallId::new(format!("call-{tool}")),
            ToolCallUpdateFields::new(),
        );
        // A live gateway awaits the client's answer; a silent client must not
        // stall the loop forever — bounded wait, then fail closed (P2-2).
        let decision = tokio::time::timeout(
            PERMISSION_PROMPT_TIMEOUT,
            self.handle.request(access, update, None, None, None),
        )
        .await
        .unwrap_or_else(|_| {
            tracing::warn!(
                tool,
                "permission prompt timed out after {PERMISSION_PROMPT_TIMEOUT:?} — denying"
            );
            Decision::Cancelled
        });
        Ok(match decision {
            Decision::Allow => PermitDecision::AllowOnce,
            // Headless: an interactive prompt has no client to answer it —
            // fail closed rather than stall or guess.
            Decision::Ask | Decision::FollowupMessage(_) => PermitDecision::Deny,
            Decision::Reject(_) | Decision::PolicyDeny(_) | Decision::Cancelled => {
                PermitDecision::Deny
            }
        })
    }

    /// The session's permission policy (IP2a, FIX_PLAN 2026-08-06 D-3) —
    /// consumed by the loop's name-level tool availability projection.
    pub fn policy(&self) -> PermissionPolicy {
        self.policy
    }

    /// P1 scope check for read-class accesses.
    ///
    /// The permission manager auto-allows `Read`/`Grep` unconditionally
    /// (SAFE_COMMAND) and GrokBuild's `read_file` preserves absolute paths —
    /// so without this, a headless agent could auto-read any absolute path on
    /// the machine. Rule: the resolved target must live under the session
    /// cwd and outside the runtime's own `.gsa` tree (journals/session state
    /// are agent-invisible — keeps the evidence chain out of the model's
    /// feedback loop). Non-read accesses pass through untouched.
    fn access_in_scope(&self, access: &AccessKind) -> bool {
        let path = match access {
            AccessKind::Read(p) | AccessKind::Grep { path: p, .. } => p.as_deref(),
            _ => return true,
        };
        let Some(path) = path else { return true };
        let p = Path::new(path);
        // Resolve relative paths against the toolset cwd — `..` can escape
        // the cwd, so lexical normalization happens before the comparison.
        let resolved = if p.is_absolute() {
            normalize_lexical(p)
        } else {
            normalize_lexical(&self.cwd.join(path).to_path_buf())
        };
        let canonical = dunce::canonicalize(&resolved).unwrap_or(resolved);
        let gsa_root = self.cwd.join(".gsa");
        path_under(self.cwd.as_path(), &canonical)
            && (!path_under(gsa_root.as_path(), &canonical)
                // GAP-RUN-TESTS (2026-08-11): the run_tests output artifact
                // (`{cwd}/.gsa/run_tests_output.txt`) is the model's
                // permission-gated window into the FULL test output —
                // ADR-0010 §3.8.3 / F-09: "完整输出保存在受控任务 artifact
                // 中，可按 permission 读取"; the conversation only carries
                // the 32KB tail. The whitelist is an exact fixed filename —
                // everything else under `.gsa` (journals, session state,
                // keystore, snapshots) stays agent-invisible.
                || canonical == normalize_lexical(gsa_root.join("run_tests_output.txt").as_path()))
    }
}

/// Lexically resolve `.` / `..` components so an escaped relative path
/// compares as what it would actually touch (a bare `..` component would
/// otherwise pass `path_under` since it is still a normal component).
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
/// path fails closed (permission manager's SendFailed → Ask → Deny).
pub(crate) fn dead_gateway() -> AcpAgentGatewaySender {
    AcpAgentGatewaySender::new(tokio::sync::mpsc::unbounded_channel().0)
}

/// Component-wise `path`-starts-with-`base`, case-insensitive on Windows
/// (canonicalized paths may differ in case from the session cwd).
fn path_under(base: &Path, path: &Path) -> bool {
    let base_parts = components_lower(&strip_verbatim_prefix(base));
    let path_parts = components_lower(&strip_verbatim_prefix(path));
    path_parts.len() >= base_parts.len() && base_parts.iter().zip(&path_parts).all(|(a, b)| a == b)
}

/// `dunce::canonicalize` keeps the `\\?\`-prefixed (verbatim) extended form
/// for paths it cannot safely simplify (>260 chars, reserved device names) —
/// strip that prefix so such a canonicalized target compares against the
/// plain session cwd. `\\?\UNC\server\share` maps back to `\\server\share`.
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
    } else if tool == "compaction_whitelist_add" || tool == "blackboard_read" {
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

    #[tokio::test]
    async fn ask_with_no_client_fails_closed() {
        // Bash (SandboxEscape) has no allow rule and no interactive client →
        // headless Ask → Deny (fail-closed).
        let decision = with_bridge(|b| async move {
            b.request(
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
            PermitDecision::Deny,
            "headless Ask must fail closed"
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
            access_kind("compaction_whitelist_add", &serde_json::json!({"content": "x"})),
            AccessKind::Read(None)
        ));
        assert!(matches!(
            access_kind("blackboard_read", &serde_json::json!({"section": "plan"})),
            AccessKind::Read(None)
        ));
    }

    // ── P1 scope enforcement ─────────────────────────────────────────────

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
        // Existing file → `canonicalize` succeeds → the Windows `\\?\` prefix
        // path is exercised (regression: prefixed paths were all denied).
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
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
        // `..` escaping cwd → denied.
        assert_eq!(
            read_req(&bridge, "../outside-escape.txt").await,
            PermitDecision::Deny
        );
        // Absolute path outside cwd → denied (the P1 hole).
        assert_eq!(
            read_req(&bridge, &outside.join("secret.txt").to_string_lossy()).await,
            PermitDecision::Deny
        );
        // The runtime's own `.gsa` tree → denied.
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("runs")
                    .join("events.jsonl")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::Deny
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
        // ...and everything else under `.gsa` stays denied (no wildcard).
        assert_eq!(
            read_req(
                &bridge,
                &dir.join(".gsa")
                    .join("run_tests_output.txt.bak")
                    .to_string_lossy()
            )
            .await,
            PermitDecision::Deny
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
            PermissionPolicy::Benchmark,
        ] {
            let bridge = bridge_with_policy(&dir, policy);
            for (tool, args) in [
                ("blackboard_read", serde_json::json!({"section": "plan"})),
                (
                    "compaction_whitelist_add",
                    serde_json::json!({"content": "任务背景"}),
                ),
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
            let decision = bridge.request(risk, tool, &args).await.unwrap();
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "{tool} must be denied under ReadOnly policy"
            );
        }

        // MCP tools are always denied under ReadOnly — even when the server
        // prefix would classify as read-class (the risk classifier prefix-
        // matches the FULL name, so a server named `read_*`/`list_*`/`grep*`
        // would otherwise slip past the policy gate; review D2-1). The `__`
        // separator marks MCP names (access_kind below).
        let decision = bridge
            .request(
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

        // Read outside the cwd → still denied by the P1 scope check.
        let decision = bridge
            .request(
                RiskClass::ReadOnly,
                "read_file",
                &serde_json::json!({"target_file": outside.join("x.txt").to_string_lossy()}),
            )
            .await
            .unwrap();
        assert_eq!(decision, PermitDecision::Deny);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
    }

    // ── Benchmark policy (2026-08-06 polyglot harness) ──────────────────

    #[tokio::test]
    async fn benchmark_policy_allows_local_mutation_denies_network_and_shell() {
        let dir = test_dir();
        std::fs::create_dir_all(dir.join("inside")).unwrap();
        std::fs::write(dir.join("inside").join("a.txt"), "x").unwrap();
        let bridge = bridge_with_policy(&dir, PermissionPolicy::Benchmark);

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

        // Shell execution stays fail-closed — `run_terminal_cmd` classifies
        // LocalMutation in the controller but is the GrokBuild bash name;
        // Benchmark grants file edits, never shell.
        for (risk, tool, args) in [
            (
                RiskClass::SandboxEscape,
                "bash",
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
