//! orz-host — Thin core that bridges Grok providers to the self-built agent loop.
//!
//! Phase 0: Skeleton only — compiles against kept Grok providers.
//! Phase 1+: ACP server, LoopHost implementation, session lifecycle, journal writer.
//!
//! Dependency direction (Codex discipline):
//!   orz-host → {orz-loop, orz-assurance, Grok providers}
//!   No Grok crate depends on orz-host.

pub mod acp_server;
pub mod approval;
pub mod codex_app;
pub mod codex_permission;
pub mod credentials;
pub mod grok_home;
pub mod keystore;
pub mod local_browser;
pub mod pdf_evidence;
pub mod permission;
pub mod project_doc_index;
pub mod retention;
pub mod session;
pub mod stdio;
pub mod tools;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use agent_client_protocol as acp;
use async_trait::async_trait;
use orz_assurance::gates::ipg::WorkspaceTrust;
use orz_assurance::journal::JournalRecorder;
use orz_tools::implementations::web_search::WebSearchConfig;
// NOTE: `PermitError` (orz_loop::host) is the LoopHost contract error; the
// assurance permit error is aliased to keep the two distinct.
use orz_assurance::permit::{
    PermitEnvelope, PermitError as PermitSigningError, PermitSigner, PermitStore,
    SensitiveActionPermit,
};
use orz_loop::host::{
    LoopHost, PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry, ToolResult,
};

use crate::keystore::MemoryInstallationKeyStore;
use crate::permission::{PermissionBridge, PermissionPolicy};
use crate::tools::ToolsetRegistry;
use orz_workspace::permission::PermissionHookTransport;

/// P0-1 (2026-08-08 stall guards): per-tool-call wall-clock budget. A tool
/// that exceeds it is terminated (process tree killed) and the call fails
/// with `ToolError::Timeout` — the run continues with the model notified.
/// Default 5 minutes; configurable via [`OrzHost::with_tool_timeout`].
pub const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(300);

/// Full LoopHost implementation over the Grok providers.
///
/// Wires the journal, the finalized toolset, the workspace-trust observation,
/// and the IP6 permission bridge into the orz-loop contract. Persistence,
/// hooks, MCP, and credentials remain default stubs (chat-state/hooks wiring
/// in Phase 3).
pub struct OrzHost {
    journal: JournalRecorder,
    registry: ToolsetRegistry,
    workspace_trust: WorkspaceTrust,
    permission: Option<PermissionBridge>,
    /// Keystore-backed P1 permit signer (DPAPI install key; test-only
    /// memory store when not injected). Consumed by `issue_permit`.
    permit_signer: Arc<dyn PermitSigner>,
    /// Root for permit artifacts — `{cwd}/.gsa/` (mirrors the Python
    /// namespace layout `<root>/one_shot_permit/`).
    permit_store_root: PathBuf,
    /// Live-client session id (ACP `session_notification` target for
    /// streamed text deltas). `None` = headless — deltas are no-ops.
    session_id: Option<String>,
    /// Live-client outbound gateway (clone; the original goes to the
    /// permission bridge). `None` = headless — deltas are no-ops.
    gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
    /// D-9 (FIX_PLAN 2026-08-06): fixed test-runner command (Aider-model
    /// feedback loop) — `Some` declares the `run_tests` tool to the model;
    /// the command is host-owned and the test files stay hidden.
    test_runner: Option<orz_loop::host::TestRunner>,
    /// P0-1 (2026-08-08 stall guards): per-call wall-clock budget for tool
    /// execution (default `TOOL_CALL_TIMEOUT`). On expiry the tool's
    /// process tree is killed and the call fails with `ToolError::Timeout`.
    tool_timeout: Duration,
    /// Session working directory — the run_tests command's cwd.
    cwd: PathBuf,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10) + GAP-PROJECT-DOC-INDEX-CACHE
    /// (2026-08-11): the project-doc index — the internal retrieval lane's
    /// real discovery/query tool (ADR-0010 §3.7.4/§3.7.5; host-owned,
    /// run_tests precedent). Holds an in-process diff snapshot, persisted
    /// across runs at `{cwd}/.gsa/project-doc-index/cache.json`.
    project_doc_index: crate::project_doc_index::ProjectDocIndex,
    /// ADR-0006 (2026-08-11): the web_search client config — single source
    /// of truth built once from the credential reader (was: env-only bool).
    /// Drives the framework_fallback capability probe (ADR-0010 §3.7.1;
    /// never a silent fallback). The secret may only leave via
    /// [`OrzHost::web_search_config_redacted`].
    web_search_config: WebSearchConfig,
    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the global `web_search`
    /// concurrency gate (ADR-0010 §3.7.7/§11.3 — global web_search
    /// concurrency is fixed at 1; the main agent and the external-retrieval
    /// subagent share this single semaphore, internal retrieval does not
    /// touch it). `call_tool` acquires one permit per `web_search*` call and
    /// drops it when the call future ends (timeout included) — the permit is
    /// owned by the tokio future, so the P0-1 timeout drop releases it
    /// automatically.
    web_search_semaphore: Arc<tokio::sync::Semaphore>,
    /// local_browser (2026-08-10): the session's browser lane handle.
    /// Defaults to a fail-closed [`UnavailableBrowserSession`] (probe failed
    /// or mode ≠ local_browser); the probe injects the real manager.
    browser: crate::local_browser::SharedBrowser,
}

impl OrzHost {
    /// Build a host for a session working directory.
    ///
    /// `workspace_trust` comes from the session bootstrap (see
    /// `session::check_workspace_trust`). No permission bridge — every
    /// `request_permission` fails closed to `Deny` (LoopHost default,
    /// review P1-1: a host that forgets to wire the bridge must not
    /// silently auto-allow `bash`).
    pub fn new(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
    ) -> Result<Self, String> {
        Self::with_permission(journal, cwd, workspace_trust, None)
    }

    /// Build a host with an optional IP6 permission bridge.
    ///
    /// `Some(bridge)` delegates `request_permission` to the Grok permission
    /// manager (Read auto-allow; headless Ask → Deny); `None` keeps the
    /// fail-closed default.
    pub fn with_permission(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        permission: Option<PermissionBridge>,
    ) -> Result<Self, String> {
        Self::with_credential_reader(
            journal,
            cwd,
            workspace_trust,
            permission,
            crate::credentials::web_search_reader(),
        )
    }

    /// ADR-0006 (2026-08-11): build a host with an injected credential
    /// reader — the test seam. A failing reader fixes `web_search` to
    /// Disabled regardless of the platform (so semaphore tests fast-fail
    /// even on a machine that has registered `orz-grok/search`).
    pub(crate) fn with_credential_reader(
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        permission: Option<PermissionBridge>,
        reader: Arc<dyn crate::credentials::CredentialReader>,
    ) -> Result<Self, String> {
        let web_search_config = tools::web_search_config(reader.as_ref());
        let toolset = tools::build_toolset(cwd, &web_search_config)?;
        // ADR-0006 (2026-08-11): the only sanctioned serialization exit for
        // the config — never log the raw `WebSearchConfig` (its `Debug`
        // contains the api_key; `redacted()` is the production surface).
        tracing::info!(
            "web_search client configured: {:?}",
            web_search_config.redacted()
        );
        Ok(Self {
            journal,
            registry: ToolsetRegistry::new(toolset),
            workspace_trust,
            permission,
            // Default: test-only memory signer (no permit is issued unless
            // `issue_permit` is called) — the session bootstrap injects the
            // DPAPI keystore-backed signer via `with_permit_signer`.
            permit_signer: Arc::new(MemoryInstallationKeyStore::new()),
            permit_store_root: cwd.join(".gsa"),
            session_id: None,
            gateway: None,
            test_runner: None,
            tool_timeout: TOOL_CALL_TIMEOUT,
            cwd: cwd.to_path_buf(),
            project_doc_index: crate::project_doc_index::ProjectDocIndex::new(cwd.to_path_buf()),
            web_search_config,
            web_search_semaphore: Arc::new(tokio::sync::Semaphore::new(1)),
            browser: Arc::new(crate::local_browser::UnavailableBrowserSession::new(
                "no browser handle injected".to_string(),
            )),
        })
    }

    /// local_browser (2026-08-10): inject the session's browser lane handle
    /// (the capability probe calls this with a launched manager; tests and
    /// conformance captures inject fakes). Declaration follows readiness —
    /// `browser_read` is only advertised when the lane is actually usable.
    pub fn with_browser_session(mut self, browser: crate::local_browser::SharedBrowser) -> Self {
        self.set_browser_session(browser);
        self
    }

    /// local_browser (2026-08-10): in-place variant for the async probe
    /// (which holds `&mut self`).
    pub fn set_browser_session(&mut self, browser: crate::local_browser::SharedBrowser) {
        let ready = browser.ready();
        self.registry.set_browser_ready(ready);
        self.browser = browser;
    }

    /// Whether the browser lane is ready — drives the `browser_read` tool
    /// declaration (same source of truth as the capability probe).
    pub fn browser_ready(&self) -> bool {
        self.browser.ready()
    }

    /// The browser lane handle (for the probe and shutdown paths).
    pub fn browser_session(&self) -> &crate::local_browser::SharedBrowser {
        &self.browser
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): whether the web_search client is
    /// configured (the framework_fallback capability probe).
    pub fn web_search_configured(&self) -> bool {
        self.web_search_config.is_enabled()
    }

    /// ADR-0006 (2026-08-11): the redacted web_search config — the only
    /// sanctioned serialization exit for the client configuration (the
    /// api_key is replaced with `***REDACTED***`; the raw config's `Debug`
    /// contains the key and must never be logged or journaled).
    pub fn web_search_config_redacted(&self) -> Option<WebSearchConfig> {
        self.web_search_config
            .is_enabled()
            .then(|| self.web_search_config.redacted())
    }

    /// P0-1 (2026-08-08 stall guards): set the per-tool-call wall-clock
    /// budget. Default `TOOL_CALL_TIMEOUT` (5 min) — a tool that exceeds it
    /// is terminated and the call fails with `ToolError::Timeout`; the run
    /// continues and the model is notified (never silently dropped).
    pub fn with_tool_timeout(mut self, timeout: Duration) -> Self {
        self.tool_timeout = timeout;
        self
    }

    /// D-9 (FIX_PLAN 2026-08-06): inject a fixed test-runner command —
    /// declares the `run_tests` tool (Aider-model feedback loop). The
    /// command is host-owned argv; the test files stay hidden from the model.
    pub fn with_test_runner(mut self, runner: Option<orz_loop::host::TestRunner>) -> Self {
        self.test_runner = runner;
        self
    }

    /// Inject the session's keystore-backed permit signer (see
    /// `session::bootstrap_session`). Without injection the host signs with
    /// a fresh in-memory key per run.
    pub fn with_permit_signer(mut self, signer: Arc<dyn PermitSigner>) -> Self {
        self.permit_signer = signer;
        self
    }

    /// The active permit signer (for inspection / verification).
    pub fn permit_signer(&self) -> &Arc<dyn PermitSigner> {
        &self.permit_signer
    }

    /// Issue a P1 one-shot sensitive-action permit signed by the
    /// keystore-backed signer, persisted under `{cwd}/.gsa/one_shot_permit/`.
    ///
    /// `envelope` carries the session's conversation/envelope identity and
    /// expiry clamp (the session security envelope in production).
    ///
    /// KNOWN GAP (2026-08-05 review P2-1): the Python authority verifies the
    /// envelope before issuance (`verify_security_envelope`: schema +
    /// `installation_key_id` match + signature). orz has no security-envelope
    /// module yet, so the envelope is caller-supplied plain data — the
    /// approval path (still a stub) must verify envelope authenticity before
    /// calling this when it lands. Until then, issuing is only sound when the
    /// caller owns the envelope construction.
    #[allow(clippy::too_many_arguments)]
    pub fn issue_permit(
        &self,
        envelope: &PermitEnvelope,
        confirmation_sha256: &str,
        action_sha256: &str,
        target_sha256: &str,
        impact_scope_sha256: &str,
        attempt: u32,
        ttl_seconds: u64,
    ) -> Result<SensitiveActionPermit, PermitSigningError> {
        PermitStore::new(self.permit_store_root.clone()).issue(
            self.permit_signer.as_ref(),
            envelope,
            confirmation_sha256,
            action_sha256,
            target_sha256,
            impact_scope_sha256,
            attempt,
            ttl_seconds,
            None,
        )
    }

    /// Build a host with an IP6 permission bridge in one step.
    ///
    /// `gateway` is the outbound ACP sender when interactive (`--stdio`);
    /// `None` (headless `-p`/`--plan`) substitutes a dead gateway — Read
    /// auto-allows, `Ask` fails closed to `Deny`.
    ///
    /// Note: `PermissionBridge::spawn` runs the manager actor via
    /// `spawn_local`, so callers must be inside a `tokio::task::LocalSet`.
    pub fn with_bridge(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
    ) -> Result<Self, String> {
        Self::with_bridge_and_hub(session_id, journal, cwd, workspace_trust, gateway, None)
    }

    /// `with_bridge` variant that additionally threads an interactive
    /// permission transport (`hub` — Phase 3 slice #12: the codex app-server
    /// approval surface). `None` keeps the ACP-gateway behavior; the rest is
    /// identical.
    pub fn with_bridge_and_hub(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
    ) -> Result<Self, String> {
        Self::with_bridge_and_hub_policy(
            session_id,
            journal,
            cwd,
            workspace_trust,
            gateway,
            hub,
            PermissionPolicy::Interactive,
        )
    }

    /// `with_bridge_and_hub` variant that additionally fixes the per-session
    /// permission policy (slice #16): a codex read-only thread passes
    /// `PermissionPolicy::ReadOnly` — mutations/network are denied without
    /// prompting; `Interactive` keeps the historical behavior.
    pub fn with_bridge_and_hub_policy(
        session_id: &str,
        journal: JournalRecorder,
        cwd: &Path,
        workspace_trust: WorkspaceTrust,
        gateway: Option<xai_acp_lib::AcpAgentGatewaySender>,
        hub: Option<Arc<dyn PermissionHookTransport>>,
        policy: PermissionPolicy,
    ) -> Result<Self, String> {
        // Clone the live-client sender before handing the original to the
        // bridge — the streamed text-delta path uses its own sender clone
        // (the permission manager gets the original).
        let live_gateway = gateway.clone();
        let bridge =
            PermissionBridge::spawn_with_hub_and_policy(session_id, gateway, hub, cwd, policy)?;
        let mut host = Self::with_permission(journal, cwd, workspace_trust, Some(bridge))?;
        host.session_id = Some(session_id.to_string());
        host.gateway = live_gateway;
        Ok(host)
    }

    /// The underlying finalized toolset (for direct dispatch).
    pub fn toolset(&self) -> &Arc<orz_tools::registry::types::FinalizedToolset> {
        self.registry.toolset()
    }
}

/// Stability fix (2026-08-07): terminate the child AND its process tree.
/// `tokio::process::Child::kill` (= TerminateProcess) only kills the direct
/// child — grandchildren that inherited our capture pipes survive as
/// orphans and keep `read_capped` blocked on EOF forever (the 52-minute
/// forth hang: harness killed orz, orz's hung pytest held the pipes).
/// Windows: TaskKill `/T /F` terminates the whole tree (and closes the
/// pipe handles, releasing the readers). ORDER MATTERS: TaskKill `/T`
/// walks ParentProcessId links, so it must run while the tree is intact —
/// killing the parent first reparents the grandchildren (Windows orphans
/// get reparented to the system process) and `/T` then finds nothing.
/// Non-Windows: plain kill remains (recorded limitation — the project
/// runtime is Windows-first).
async fn kill_process_tree(child: &mut tokio::process::Child) {
    #[cfg(windows)]
    {
        if let Some(id) = child.id() {
            let tk = tokio::process::Command::new("taskkill")
                .args(["/PID", &id.to_string(), "/T", "/F"])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if let Ok(mut tk) = tk {
                let _ = tk.wait().await;
            }
        }
    }
    let _ = child.kill().await;
    let _ = child.wait().await;
}

/// F-09 (2026-08-07 review): collect a streamed pipe with a hard cap — the
/// TAIL is kept (test summaries live at the end); leading bytes are dropped
/// on overflow so pathological output cannot blow session memory. A closed
/// or failed pipe simply ends the collection.
async fn read_capped<R: tokio::io::AsyncRead + Unpin>(mut reader: R, buf: &mut Vec<u8>) {
    use tokio::io::AsyncReadExt;
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() > orz_loop::host::RUN_TESTS_OUTPUT_CAP {
                    let excess = buf.len() - orz_loop::host::RUN_TESTS_OUTPUT_CAP;
                    buf.drain(..excess);
                }
            }
        }
    }
}

/// RT-002 (2026-08-11): the fixed test command's minimal environment
/// allowlist — the ONLY host environment the test process inherits (plus the
/// harness's explicit `TestRunner::env` entries). Everything else is cleared:
/// host secrets, `ORZ_*` session variables, arbitrary paths.
#[cfg(windows)]
const TEST_ENV_ALLOWLIST: &[&str] = &[
    // PATH: command lookup (the runner command often names an interpreter
    // by bare name). SystemRoot/PATHEXT/COMSPEC: Windows process bootstrap
    // (DLL search, batch invocation). TEMP/TMP: temp files. USERPROFILE:
    // many tools want a writable home for caches.
    "PATH",
    "SystemRoot",
    "PATHEXT",
    "COMSPEC",
    "TEMP",
    "TMP",
    "USERPROFILE",
];
#[cfg(not(windows))]
const TEST_ENV_ALLOWLIST: &[&str] = &[
    // PATH: command lookup. HOME: cache/config dirs. TMPDIR: temp files.
    // LANG: locale output stability.
    "PATH", "HOME", "TMPDIR", "LANG",
];

/// RT-003 (2026-08-11): directories excluded from the run_tests delta walk —
/// VCS metadata, the host's own `.gsa` tree, and the heavyweight
/// dependency/cache directories a test run would never legitimately write
/// (recording node_modules churn would drown the trace). `__pycache__` /
/// `.pytest_cache` / `.mypy_cache` / `.ruff_cache` / `.tox` are the Python
/// test-runner's own cache surface (the primary harness deployment) —
/// every run rewrites them, so without the exclusion the 200-entry cap is
/// consumed by cache noise and the real side effects get truncated away
/// (review D2-1/P1, 2026-08-11).
const DELTA_EXCLUDED_DIRS: &[&str] = &[
    ".git",
    ".gsa",
    "node_modules",
    ".venv",
    "venv",
    "target",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
];

/// RT-003: cap on the workspace-delta entries recorded per run_tests call.
const RUN_TESTS_DELTA_MAX_ENTRIES: usize = 200;

/// RT-003: worktree metadata walk (zero content reads) — the run_tests
/// delta baseline. Symlinks are not followed (a target outside the worktree
/// is not a delta; a dangling link is not a file change).
fn workspace_delta_walk(cwd: &Path) -> std::collections::HashMap<String, (u64, u64, u32)> {
    let mut map = std::collections::HashMap::new();
    let mut stack = vec![cwd.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            // Review D2-2 (2026-08-11): Windows junctions are DIRECTORY
            // reparse points — `file_type().is_symlink()` is false for them,
            // so without this check a junction pointing at an ancestor (or
            // at a huge tree like C:\Users) would be walked unboundedly
            // (this walk runs OUTSIDE the test-run timeout). Skip every
            // reparse point, symlink or junction — both are linkage, not
            // file content, in the delta semantics. Single-sourced with the
            // ACAF ticket side (2026-08-12): orz-paths
            // `is_reparse_or_symlink` (symlink_metadata — NOT `metadata`,
            // which would follow the junction and hide the 0x400 bit;
            // the old local copy reused the read_dir entry type to skip the
            // extra syscall on Unix — semantically equivalent, registered).
            if orz_paths::resolve::is_reparse_or_symlink(&path) {
                continue;
            }
            if ft.is_dir() {
                if !DELTA_EXCLUDED_DIRS.contains(&name.as_str()) {
                    stack.push(path);
                }
            } else if ft.is_file()
                && let Ok(md) = std::fs::metadata(&path)
            {
                let (mtime_secs, mtime_nanos) = match md.modified() {
                    Ok(t) => {
                        let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
                        (d.as_secs(), d.subsec_nanos())
                    }
                    // No mtime support: (0,0) — every file diffs as
                    // "modified" after a run (conservative; same
                    // registered trade-off as the doc-index walk).
                    Err(_) => (0, 0),
                };
                let rel = path
                    .strip_prefix(cwd)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                map.insert(rel, (md.len(), mtime_secs, mtime_nanos));
            }
        }
    }
    map
}

/// RT-003: diff two delta walks into the capped change list.
fn workspace_delta_diff(
    before: &std::collections::HashMap<String, (u64, u64, u32)>,
    after: &std::collections::HashMap<String, (u64, u64, u32)>,
) -> (Vec<orz_loop::host::WorkspaceDeltaEntry>, bool) {
    use orz_loop::host::{WorkspaceDeltaEntry, WorkspaceDeltaKind};
    let mut entries: Vec<WorkspaceDeltaEntry> = Vec::new();
    for (path, after_stat) in after {
        match before.get(path) {
            None => entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Added,
            }),
            Some(before_stat) if before_stat != after_stat => entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Modified,
            }),
            _ => {}
        }
    }
    for path in before.keys() {
        if !after.contains_key(path) {
            entries.push(WorkspaceDeltaEntry {
                path: path.clone(),
                kind: WorkspaceDeltaKind::Deleted,
            });
        }
    }
    // Deterministic order (same discipline as the doc-index sorted entries).
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let truncated = entries.len() > RUN_TESTS_DELTA_MAX_ENTRIES;
    entries.truncate(RUN_TESTS_DELTA_MAX_ENTRIES);
    (entries, truncated)
}

impl OrzHost {
    /// P0-C S4 (2026-08-16): shared tool-execution core with an optional
    /// per-call timeout override (script step deadlines). `None` = the
    /// configured host budget; an override is capped at the configured
    /// budget (`min`) so the host ceiling can never be raised by callers.
    async fn call_tool_inner(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
        timeout_override: Option<std::time::Duration>,
    ) -> Result<ToolResult, ToolError> {
        // P0-1 (2026-08-08 stall guards): bounded tool execution. Every
        // tool call runs under a wall-clock cap (default 5 min,
        // configurable via `with_tool_timeout`) — a tool whose
        // implementation awaits forever (a hung bash child, an
        // `ask_user_question` with no user attached, a stuck fs read, …)
        // used to hang the whole run with no bound anywhere (the
        // dna-assembly 16:02 hang attribution). On expiry the tool's
        // process tree is killed via the global process scope (Windows:
        // per-child Job Object `TerminateJobObject` — kills grandchildren
        // too, the 2026-08-07 orphan-holds-pipes mechanism) and the call
        // fails with `ToolError::Timeout`; the controller journals
        // `tool_completed{status:error}` and the model continues the loop.
        //
        // Recorded trade-off: the kill is process-global — any concurrently
        // running tool child (e.g. an earlier background command) is
        // terminated too. A hung tool poisons the session; leaving
        // orphaned processes behind is worse. The bash tool itself carries
        // a foreground timeout (120s default), so this wrapper is the
        // coarse backstop for every tool, bash included.
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the project-doc index is a
        // host-owned tool (run_tests precedent) — routed before the
        // finalize toolset; synchronous and workspace-local.
        if name == "project_doc_index" {
            return self.project_doc_index.query(&args);
        }
        // local_browser (2026-08-10): `browser_read` is host-owned (the
        // browser lane is session state, not a finalized-toolset resource).
        // The mode gate lives in the controller (`is_retrieval_mode_gated_
        // host_tool`); here we only execute when the session carries a
        // browser. Fail-closed: no handle → explicit error, never a stub
        // success (ADR-0010 §3.7.2).
        if name == "browser_read" {
            if !self.browser.ready() {
                return Err(ToolError::ExecutionFailed(
                    "browser_read: browser lane not available (probe failed or \
                     mode ≠ local_browser)"
                        .to_string(),
                ));
            }
            return crate::local_browser::handle_browser_read(self.browser.as_ref(), &args).await;
        }
        // PDF evidence (2026-08-11): `pdf_read` reads the local evidence
        // store — synchronous, workspace-local (project_doc_index pattern).
        if name == "pdf_read" {
            return crate::pdf_evidence::handle_pdf_read(&self.cwd, &args).await;
        }
        // PDF evidence routing (2026-08-11): a `web_fetch` whose URL matches
        // ORZ_PDF_BROWSER_DOMAINS is intercepted BEFORE the toolset — the
        // whitelisted paper-library fetch happens through the browser
        // (operator login). Everything else falls through to the toolset
        // (direct channel, where PDFs are ingested inline). A whitelist hit
        // with an unavailable browser is an explicit failure — never an
        // automatic direct fallback (user ruling; §3.7.2).
        if name == "web_fetch"
            && crate::pdf_evidence::route_for_url(
                args.get("url").and_then(|u| u.as_str()).unwrap_or(""),
            )
        {
            let url = args.get("url").and_then(|u| u.as_str()).unwrap_or_default();
            return crate::pdf_evidence::handle_browser_pdf(
                &self.cwd,
                self.session_id.as_deref(),
                self.browser.as_ref(),
                url,
            )
            .await;
        }
        // GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): global `web_search`
        // concurrency is fixed at 1 (ADR-0010 §3.7.7/§11.3 — the main agent
        // and the external-retrieval subagent share this semaphore; internal
        // retrieval does not touch it, so internal + external sessions still
        // run in parallel). Every `web_search*` call acquires the shared
        // permit BEFORE execution. The P0-1 timeout wraps the whole
        // acquire+call future, so a WAITING call is bounded by the same
        // 300s budget, and the timeout drop releases the permit with the
        // future — a holder can never leak the gate. `web_fetch` is not
        // gated (the contract limits only web_search).
        //
        // P3-1 (review 2026-08-10, upgraded to a fix — the new timeout test
        // reproduced the mis-kill): `started_exec` distinguishes a WAITING
        // timeout from an EXECUTION timeout. A call that times out while
        // waiting for the permit holds no process tree of its own, so the
        // global `kill_active` would only destroy unrelated concurrent
        // processes (reproduced: the parallel `call_tool_timeout_kills_
        // process_tree` test lost its python child to the semaphore test's
        // 100ms waiting timeout). Only an execution timeout kills.
        let started_exec = std::sync::atomic::AtomicBool::new(false);
        let fut = async {
            if crate::tools::is_web_search_tool(name) {
                tracing::debug!(
                    tool = name,
                    "web_search: acquiring the global semaphore (concurrency=1)"
                );
                let _permit = self
                    .web_search_semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|e| {
                        // P3-4 (review 2026-08-10): the map_tool_error
                        // bridge surfaces only the Display text — inline the
                        // code so a model never sees a bare "semaphore
                        // closed" that reads like a tool being switched off.
                        xai_tool_runtime::ToolError::custom(
                            "web_search_semaphore",
                            format!("web_search_semaphore: {e}"),
                        )
                    })?;
                started_exec.store(true, std::sync::atomic::Ordering::SeqCst);
                self.registry
                    .toolset()
                    .call(name, args, call_id, None)
                    .await
            } else {
                started_exec.store(true, std::sync::atomic::Ordering::SeqCst);
                self.registry
                    .toolset()
                    .call(name, args, call_id, None)
                    .await
            }
        };
        let effective_timeout = timeout_override
            .map(|t| t.min(self.tool_timeout))
            .unwrap_or(self.tool_timeout);
        let result = match tokio::time::timeout(effective_timeout, fut).await {
            Ok(result) => result.map_err(|e| crate::tools::map_tool_error(&e))?,
            Err(_) if started_exec.load(std::sync::atomic::Ordering::SeqCst) => {
                tracing::warn!(
                    tool = name,
                    timeout = ?effective_timeout,
                    "tool call TIMED OUT — killing the tool process tree"
                );
                // 2026-08-08 review F1 (P1-1/D1-1): `kill_active` — NOT
                // `kill_all`. The latter latches the global scope closed
                // (its contract is "call only when the process is genuinely
                // exiting"); a mid-session latch would kill every LATER
                // spawn on the spot (terminal.rs ignores `register`'s
                // return), so one tool timeout would poison all subsequent
                // bash calls for the whole session.
                orz_tools::util::global_process_scope().kill_active();
                return Err(ToolError::Timeout(format!(
                    "tool '{name}' TIMED OUT after {timeout:?} wall-clock budget — \
                     process tree killed; the tool did not complete",
                    timeout = effective_timeout,
                )));
            }
            Err(_) => {
                // Waiting timeout: bounded by the same 300s budget (D-3),
                // but nothing was killed — the call never reached a tool.
                tracing::warn!(
                    tool = name,
                    timeout = ?effective_timeout,
                    "web_search call TIMED OUT while waiting for the global permit"
                );
                return Err(ToolError::Timeout(format!(
                    "tool '{name}' TIMED OUT after {timeout:?} wall-clock budget — \
                     still waiting for the global web_search permit (concurrency=1); \
                     nothing was killed, retry when the holder finishes",
                    timeout = effective_timeout,
                )));
            }
        };
        Ok(ToolResult {
            output: result.prompt_text,
            // 2026-08-08 blackboard-partition review closure (conformance
            // agent D1-1): the controller's edit-action gate keys on
            // `exit_code == Some(0)` ("实际变动" 才记). Previously this was
            // hardcoded `None` — the production shape never reached the
            // controller and A1/A2 (edit records + incremental push) were
            // dead in real runs; test hosts fabricating `Some(0)` masked it.
            // Map the structured output: bash carries its real exit code;
            // search_replace reports "applied" only via the EditsApplied
            // variant (NoMatchesFound etc. are Ok outputs that changed
            // nothing → non-zero); every other successful output is 0.
            exit_code: crate::tools::exit_code_from_output(&result.output),
            // GAP-ENCODING-GATE: forward the decode stage observed by the
            // tool implementation (run_terminal_cmd / read_file) to the
            // journal's `tool_completed.output_encoding`.
            output_encoding: result.output_encoding,
            // FUS-RETRIEVAL-MECH B-1 (2026-08-13): web_search citation URLs
            // ride the structured seam into the loop (candidate pool for
            // the mechanical prefilter); every other tool is `None`.
            structured: crate::tools::structured_from_output(&result.output),
            ..Default::default()
        })
    }
}

#[async_trait]
impl LoopHost for OrzHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }

    fn tools_registry(&self) -> &dyn ToolRegistry {
        &self.registry
    }

    fn workspace_trust(&self) -> WorkspaceTrust {
        self.workspace_trust
    }

    /// IP2a (FIX_PLAN 2026-08-06 D-3): map the bridge's session permission
    /// policy onto the loop's `ToolPolicy` — the loop filters policy-refused
    /// tools out of the model-visible declarations. A missing bridge (no
    /// policy known) maps to `Interactive` for the DECLARATION projection
    /// only; execution still fails closed through `request_permission`
    /// (LoopHost default → Deny), so nothing is auto-allowed by this.
    fn tool_policy(&self) -> orz_loop::host::ToolPolicy {
        use orz_loop::host::ToolPolicy;
        match self.permission.as_ref().map(|b| b.policy()) {
            Some(PermissionPolicy::ReadOnly) => ToolPolicy::ReadOnly,
            Some(PermissionPolicy::Benchmark) => ToolPolicy::Benchmark,
            _ => ToolPolicy::Interactive,
        }
    }

    /// D-9 (FIX_PLAN 2026-08-06): the injected fixed test-runner command.
    fn test_runner(&self) -> Option<orz_loop::host::TestRunner> {
        self.test_runner.clone()
    }

    fn session_cwd(&self) -> std::path::PathBuf {
        self.cwd.clone()
    }

    /// The ACP outbound gateway is the only live interactive-user channel
    /// (`ask_user_question` delivers through it); headless sessions
    /// (`gateway == None`, including the codex hub-only approval surface)
    /// fail closed.
    fn interactive_user(&self) -> bool {
        self.gateway.is_some()
    }

    /// FUS-TOOL-PROBE P0-A-2 (v0.2 single probe face): the ORZ host always
    /// wires a local terminal backend (`tools::build_toolset` constructs
    /// `LocalTerminalBackend`), so `run_terminal_cmd`'s mechanical chain is
    /// complete at the host level (policy still gates exec per session).
    fn terminal_available(&self) -> bool {
        true
    }

    /// FUS-TOOL-PROBE P0-A-2: the ORZ host currently builds its toolset
    /// with every optional backend disabled (`build_toolset`:
    /// `memory_backend: None`, `lsp: None`, image/video configs
    /// `Default::default()` = Disabled, no MCP registration). The probes
    /// therefore fail closed — these tools are declared in the GrokBuild
    /// registry but cannot complete a call, so the single probe face
    /// removes them from the model-visible list. When a backend is wired
    /// in `tools::build_toolset`, flip the matching accessor here.
    fn lsp_configured(&self) -> bool {
        false
    }

    fn memory_enabled(&self) -> bool {
        false
    }

    fn image_backend_configured(&self) -> bool {
        false
    }

    fn video_backend_configured(&self) -> bool {
        false
    }

    fn mcp_registry_available(&self) -> bool {
        false
    }

    /// D-9: run the FIXED test command in the session cwd. The model never
    /// supplies argv — the command is host-owned; stdout/stderr/exit code
    /// feed back to the model (Aider-model loop).
    async fn run_tests(&self) -> Result<orz_loop::host::TestRunResult, ToolError> {
        let Some(runner) = self.test_runner.clone() else {
            return Err(ToolError::NotFound("no test runner configured".into()));
        };
        if runner.command.is_empty() {
            return Err(ToolError::ExecutionFailed("empty test command".into()));
        }
        let timeout = runner.timeout.unwrap_or(orz_loop::host::RUN_TESTS_TIMEOUT);
        // RT-003 (2026-08-11): workspace delta — snapshot the worktree
        // metadata before the run; after the run the diff (added/modified/
        // deleted, capped) is the audit trace of the test's file side
        // effects (ADR §3.8.2 requires workspace-delta/journal recording).
        let before = workspace_delta_walk(&self.cwd);
        // F-09 (2026-08-07 review): bounded execution + context gating. The
        // command runs under a wall-clock cap (default 30min — the cap only
        // catches true hangs; context size, not wall time, is the priority);
        // a hung child is killed. Output is collected with a TAIL cap (1MB)
        // so pathological output cannot blow session memory; the full
        // (capped) output is written to a file the model can read
        // (read_file), and the controller injects only the final 32KB into
        // the conversation.
        let mut cmd = tokio::process::Command::new(&runner.command[0]);
        cmd.args(&runner.command[1..])
            .current_dir(&self.cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        // RT-002 (2026-08-11): the test process must NOT inherit the host's
        // environment wholesale — it would see host secrets and paths
        // (ADV §3.8.2: secret/host-path leakage through test output). Clear
        // everything, then restore a fixed minimal platform allowlist plus
        // the harness's explicit `TestRunner::env` entries.
        cmd.env_clear();
        for var in TEST_ENV_ALLOWLIST {
            if let Ok(value) = std::env::var(var) {
                cmd.env(var, value);
            }
        }
        for (k, v) in &runner.env {
            cmd.env(k, v);
        }
        let mut child = cmd
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("test runner spawn: {e}")))?;
        // Stability fix (2026-08-07, design review D2 #3/#4): bind the child
        // into a kill-on-close Job Object. This closes the original hang
        // mechanism end-to-end: when orz ITSELF is killed (the harness's
        // timeout path), the job handle closes with the process and the
        // kernel terminates the whole contained tree — the orphaned pytest
        // that held our capture pipes and blocked the harness forever (the
        // 52-minute forth hang) cannot survive orz. Assigning after spawn
        // (running) is a millisecond window vs CREATE_SUSPENDED, accepted
        // and recorded; descendants spawned after assignment inherit the job.
        // Non-Windows (or job creation failure): Option::None falls back to
        // the TaskKill tree-kill path below.
        let supervisor: Option<orz_assurance::sandbox::job_object::JobObjectSupervisor> =
            orz_assurance::sandbox::job_object::JobObjectSupervisor::new().ok();
        if let (Some(sup), Some(pid)) = (supervisor.as_ref(), child.id())
            && let Err(e) = sup.assign_process(pid)
        {
            tracing::warn!(
                "run_tests: job-object assignment failed ({e}); \
                            falling back to TaskKill on timeout"
            );
            // Note: `supervisor` is deliberately not rebound here — the
            // assignment failure leaves a live job with no members, which is
            // harmless to drop; the TaskKill path covers the timeout case.
        }
        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stdout pipe unreadable".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stderr pipe unreadable".into())
        })?;
        let mut out_buf = Vec::<u8>::new();
        let mut err_buf = Vec::<u8>::new();
        let collect = async {
            tokio::join!(
                read_capped(stdout, &mut out_buf),
                read_capped(stderr, &mut err_buf),
            );
            child.wait().await
        };
        let status = match tokio::time::timeout(timeout, collect).await {
            Ok(status) => {
                status.map_err(|e| ToolError::ExecutionFailed(format!("test runner wait: {e}")))?
            }
            Err(_) => {
                // Stability fix (2026-08-07): `Child::kill` terminates only
                // the direct child — grandchildren that inherited our capture
                // pipes (e.g. a pytest spawned by the runner) survive as
                // orphans and keep `read_capped` blocked on EOF forever (the
                // 52-minute forth hang). Kill the whole tree on Windows via
                // TaskKill; unix keeps the plain kill (recorded limitation).
                kill_process_tree(&mut child).await;
                // RT-003: the timed-out run may still have written files —
                // record what it changed before returning.
                let (workspace_delta, workspace_delta_truncated) =
                    workspace_delta_diff(&before, &workspace_delta_walk(&self.cwd));
                let (partial_output, output_encoding) =
                    orz_tools::util::encoding::decode_text(&out_buf);
                return Ok(orz_loop::host::TestRunResult {
                    output: format!(
                        "[test runner TIMED OUT after {timeout:?} — process tree killed; \
                         partial output follows]\n{}",
                        partial_output,
                    ),
                    exit_code: None,
                    full_output_path: None,
                    output_encoding: Some(output_encoding.to_string()),
                    workspace_delta,
                    workspace_delta_truncated,
                });
            }
        };
        // GAP-ENCODING-GATE (OPS-PROTOCOL §8): stdout/stderr each decode
        // through the fixed chain; both labels are recorded (comma-joined).
        let (mut text, stdout_encoding) = orz_tools::util::encoding::decode_text(&out_buf);
        let mut encodings = vec![stdout_encoding];
        if !err_buf.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            let (err_text, err_encoding) = orz_tools::util::encoding::decode_text(&err_buf);
            encodings.push(err_encoding);
            text.push_str(&err_text);
        }
        let output_encoding =
            orz_tools::util::encoding::merge_encoding_labels(encodings.iter().copied());
        // F-09: write the full (capped) output to a file the model can read
        // (read_file) — the conversation only carries the final 32KB.
        let gsa_dir = self.cwd.join(".gsa");
        let full_output_path = match std::fs::create_dir_all(&gsa_dir)
            .and_then(|()| std::fs::write(gsa_dir.join("run_tests_output.txt"), &text))
        {
            Ok(()) => Some(
                gsa_dir
                    .join("run_tests_output.txt")
                    .to_string_lossy()
                    .into_owned(),
            ),
            Err(e) => {
                tracing::warn!("run_tests: full output file write failed: {e}");
                None
            }
        };
        // RT-003: workspace delta — diff the post-run walk against the
        // pre-run baseline (`.gsa` is excluded, so the output file written
        // above does not pollute the trace).
        let (workspace_delta, workspace_delta_truncated) =
            workspace_delta_diff(&before, &workspace_delta_walk(&self.cwd));
        Ok(orz_loop::host::TestRunResult {
            output: text,
            exit_code: status.code(),
            full_output_path,
            output_encoding,
            workspace_delta,
            workspace_delta_truncated,
        })
    }

    /// Forward a streamed model text chunk to the live ACP client as an
    /// `agent_message_chunk` notification — the TUI renders it incrementally
    /// into the current model card (streaming slice). Deltas are live-only,
    /// never journaled (Python `text_delta` precedent); headless hosts
    /// (no gateway) drop the chunk silently.
    fn on_text_delta(&self, text: &str) {
        let (Some(gateway), Some(session_id)) = (&self.gateway, &self.session_id) else {
            return;
        };
        let notification = acp::SessionNotification::new(
            acp::SessionId::new(session_id.clone()),
            acp::SessionUpdate::AgentMessageChunk(acp::ContentChunk::new(acp::ContentBlock::Text(
                acp::TextContent::new(text.to_string()),
            ))),
        );
        // Fire-and-forget: the `acp::Agent::session_notification` trait method
        // is `#[async_trait(?Send)]` (its future is not Send), so it cannot be
        // awaited inside this Send-bound LoopHost impl — `forward_fire_and_forget`
        // is the documented bypass. A dropped receiver discards the chunk.
        if !gateway.forward_fire_and_forget(notification) {
            tracing::debug!("on_text_delta: gateway receiver dropped, chunk discarded");
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool_inner(name, args, call_id, None).await
    }

    /// P0-C S4 (2026-08-16): script step deadlines — a per-call override
    /// bounded by the configured host budget (`min`). The host still owns
    /// process-tree reclamation on expiry (`kill_active`), exactly like the
    /// default path.
    async fn call_tool_with_timeout(
        &self,
        name: &str,
        args: serde_json::Value,
        call_id: &str,
        timeout: Option<std::time::Duration>,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool_inner(name, args, call_id, timeout).await
    }

    async fn request_permission(
        &self,
        risk: RiskClass,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<PermitDecision, PermitError> {
        match &self.permission {
            Some(bridge) => bridge.request(risk, tool, args).await,
            // No bridge wired → fail closed, never auto-allow (review P1-1).
            None => Ok(PermitDecision::Deny),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_assurance::journal::{EventType, RunEvent};
    use orz_loop::AgentLoopController;
    use orz_loop::gateway::fake::FakeProvider;
    use orz_loop::gateway::model::{ModelGateway, ToolCall};
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// P2-1 (review 2026-08-10): every test that mutates the web_search env
    /// vars takes this lock — `web_search_config_follows_credential_source`
    /// (tools.rs) and the semaphore tests here share one test binary and
    /// would otherwise race on the `ORZ_WEB_SEARCH_BASE_URL`/`_MODEL`
    /// overrides (a cross-test flake). (2026-08-11: `ORZ_WEB_SEARCH_API_KEY`
    /// was removed with the direction correction — the key now comes from
    /// the credential reader; the lock guards the remaining non-secret env
    /// overrides.)
    /// tokio mutex: the async tests hold it across awaits (clippy
    /// await_holding_lock would flag a std guard); OnceLock because tokio's
    /// `Mutex::new` is not const.
    pub(crate) static TESTS_ENV_LOCK: std::sync::OnceLock<tokio::sync::Mutex<()>> =
        std::sync::OnceLock::new();

    /// P2-1: accessor for the shared env-test lock.
    pub(crate) fn tests_env_lock() -> &'static tokio::sync::Mutex<()> {
        TESTS_ENV_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
    }

    /// P2-1 (review 2026-08-10): restores an env var on drop — tests that
    /// mutate shared env state must not leak into later tests.
    pub(crate) struct EnvVarGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvVarGuard {
        pub(crate) fn new(key: &'static str) -> Self {
            let old = std::env::var(key).ok();
            Self { key, old }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.old {
                Some(v) => unsafe { std::env::set_var(self.key, v) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-host-lib-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 2026-08-11: a credential reader that always fails — fixes
    /// `web_search` to Disabled regardless of the platform (semaphore and
    /// routing tests must fast-fail even on a machine whose DeepSeek
    /// credential is registered; without this seam they would attempt real
    /// API calls).
    fn failing_reader() -> Arc<dyn crate::credentials::CredentialReader> {
        struct Fail;
        impl crate::credentials::CredentialReader for Fail {
            fn read(&self) -> Result<String, crate::credentials::CredentialError> {
                Err(crate::credentials::CredentialError {
                    message: "test: no credential registered".into(),
                })
            }
        }
        Arc::new(Fail)
    }

    /// Host with the failing reader injected (see [`failing_reader`]).
    fn host_with_failing_reader(journal: JournalRecorder, dir: &std::path::Path) -> OrzHost {
        OrzHost::with_credential_reader(
            journal,
            dir,
            WorkspaceTrust::ObservedTrusted,
            None,
            failing_reader(),
        )
        .expect("host build")
    }

    /// Toolset construction is expensive (builder finalize) — share one.
    static SHARED_TOOLSET: OnceLock<Arc<orz_tools::registry::types::FinalizedToolset>> =
        OnceLock::new();

    fn shared_toolset() -> &'static Arc<orz_tools::registry::types::FinalizedToolset> {
        SHARED_TOOLSET.get_or_init(|| {
            tools::build_toolset(&std::env::temp_dir(), &WebSearchConfig::Disabled)
                .expect("shared toolset")
        })
    }

    #[tokio::test]
    async fn registry_lists_builtin_tools() {
        let toolset = shared_toolset();
        let names: Vec<String> = toolset
            .tool_definitions()
            .iter()
            .map(|d| d.function.name.clone())
            .collect();
        assert!(
            names.iter().any(|n| n == "read_file"),
            "expected read_file in builtin tools, got {names:?}"
        );
        assert!(
            names.iter().any(|n| n == "run_terminal_cmd"),
            "expected run_terminal_cmd (GrokBuild bash), got {names:?}"
        );
    }
    /// GAP-ENCODING-GATE e2e: `read_file` through the host decodes a
    /// GB18030 file with the fixed chain and forwards the observed stage to
    /// the loop's `ToolResult.output_encoding` (journaled upstream as
    /// `tool_completed.output_encoding`).
    #[tokio::test]
    async fn read_file_gb18030_forwards_output_encoding() {
        let dir = test_dir();
        let mut bytes = vec![0xd6, 0xd0, 0xce, 0xc4];
        bytes.push(b'\n');
        std::fs::write(dir.join("gb.txt"), &bytes).unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let result = host
            .call_tool(
                "read_file",
                serde_json::json!({"target_file": "gb.txt"}),
                "call-gb",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains('中'), "output: {}", result.output);
        assert_eq!(result.output_encoding.as_deref(), Some("gb18030"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// local_browser (2026-08-10): the registry declares `browser_read`
    /// ONLY when the browser lane is ready (fail-closed default), and the
    /// host routes it to the injected browser session.
    #[tokio::test]
    async fn browser_read_declaration_follows_browser_ready() {
        // Default registry: not ready → not declared.
        let mut registry = ToolsetRegistry::new(shared_toolset().clone());
        assert!(registry.get("browser_read").is_none());
        assert!(!registry.list().iter().any(|d| d.name == "browser_read"));
        // Flip ready → declared (declaration and probe are one source).
        registry.set_browser_ready(true);
        assert!(registry.get("browser_read").is_some());
        assert!(registry.list().iter().any(|d| d.name == "browser_read"));
        // Flip back → gone again.
        registry.set_browser_ready(false);
        assert!(registry.get("browser_read").is_none());
    }

    /// local_browser (2026-08-10): `call_tool("browser_read")` routes to the
    /// injected browser session; without a ready lane it fails explicitly
    /// (never a silent stub success — ADR-0010 §3.7.2).
    #[tokio::test]
    async fn call_browser_read_routes_to_session_and_fails_closed() {
        let dir = test_dir();

        // Fail-closed default: no handle injected → explicit error.
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let err = host
            .call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c1",
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("browser lane not available"),
            "{err}"
        );

        // Ready session → success path with the real wrapper.
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        assert!(host.browser_ready());
        let result = host
            .call_tool(
                "browser_read",
                serde_json::json!({"url": "https://example.com"}),
                "c2",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("hello page"), "{}", result.output);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// PDF evidence (2026-08-11): `call_tool("web_fetch", whitelisted url)`
    /// is intercepted at the chokepoint and routed through the browser lane;
    /// without a ready lane it fails explicitly with the whitelist failure
    /// code (user ruling — no automatic fallback to direct fetch).
    #[tokio::test]
    async fn call_web_fetch_whitelist_routes_through_browser() {
        let _env_lock = crate::tests::tests_env_lock().lock().await;
        let dir = test_dir();
        crate::pdf_evidence::set_domains_override(Some("*.cnki.net".to_string()));

        // No browser handle → explicit whitelist failure, no fallback.
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap();
        let err = host
            .call_tool(
                "web_fetch",
                serde_json::json!({"url": "https://kns.cnki.net/paper.pdf"}),
                "p1",
            )
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("web_fetch_pdf_browser_unavailable"),
            "{err}"
        );

        // Ready lane → intercepted: the stub's download path runs (returns
        // a Page outcome — shaped like browser_read, no document_id).
        let stub = crate::local_browser::tests::ready_stub_browser();
        let host = OrzHost::new(
            JournalRecorder::new(dir.clone()),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .unwrap()
        .with_browser_session(stub);
        let result = host
            .call_tool(
                "web_fetch",
                serde_json::json!({"url": "https://kns.cnki.net/kcms/detail"}),
                "p2",
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("hello page"), "{}", result.output);
        assert!(!result.output.contains("document_id="), "{}", result.output);

        crate::pdf_evidence::set_domains_override(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): a `web_search` call waits on
    /// the global permit (concurrency=1, ADR-0010 §3.7.7/§11.3). The test
    /// holds the permit, spawns the call, asserts it is still pending while
    /// held, then releases — the call completes (fast-fails: the tool is
    /// unregistered without an env key, but the gate ordering is what is
    /// under test).
    #[tokio::test]
    async fn web_search_call_waits_for_global_permit() {
        let dir = test_dir();
        // ADR-0006 (2026-08-11): the injected failing reader fixes
        // web_search to Disabled on every platform (was: env removal — a
        // machine with a registered `orz-grok/search` credential would
        // otherwise enable the client and hit the real API).
        let host = Arc::new(host_with_failing_reader(
            JournalRecorder::new(dir.clone()),
            &dir,
        ));

        // Hold the single permit.
        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool(
                    "web_search",
                    serde_json::json!({"query": "test"}),
                    "ws-gate-1",
                )
                .await
        });

        // While the permit is held the call must still be pending.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert!(
            !handle.is_finished(),
            "web_search call must wait on the global permit"
        );

        // Release → the call proceeds (and fast-fails: no key = unregistered).
        drop(permit);
        let result = handle.await.unwrap();
        assert!(
            result.is_err(),
            "unregistered web_search must fail: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10, review F7): a P0-1 timeout on a
    /// WAITING call drops the future → the permit releases with it. A holder
    /// can never leak the gate: after the timeout the next call acquires
    /// immediately (fast-fails again — unregistered without an env key).
    #[tokio::test]
    async fn web_search_timeout_releases_the_permit() {
        let dir = test_dir();
        // ADR-0006 (2026-08-11): injected failing reader (see the waiting
        // test above) — platform-independent Disabled, no real API risk.
        let host = Arc::new(
            host_with_failing_reader(JournalRecorder::new(dir.clone()), &dir)
                .with_tool_timeout(std::time::Duration::from_millis(100)),
        );

        // Hold the single permit so the call can only wait.
        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool("web_search", serde_json::json!({"query": "x"}), "ws-tout-1")
                .await
        });
        let err = handle.await.unwrap().unwrap_err();
        assert!(
            err.to_string().contains("TIMED OUT"),
            "waiting call must hit the P0-1 budget: {err}"
        );

        // The timeout dropped the future → the permit is back.
        drop(permit);
        let host3 = host.clone();
        let again = tokio::spawn(async move {
            host3
                .call_tool("web_search", serde_json::json!({"query": "y"}), "ws-tout-2")
                .await
        });
        let res = again.await.unwrap();
        assert!(
            res.is_err(),
            "second call must acquire the released permit and fast-fail: {res:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-WEB-SEARCH-SEMAPHORE (2026-08-10): the gate covers ONLY the
    /// `web_search` family — a non-search tool (read_file) completes while
    /// the permit is held.
    #[tokio::test]
    async fn non_web_search_tools_ignore_the_gate() {
        let dir = test_dir();
        let path = dir.join("gate.txt");
        std::fs::write(&path, "gate").unwrap();
        let host = Arc::new(host_with_failing_reader(
            JournalRecorder::new(dir.clone()),
            &dir,
        ));

        let permit = host
            .web_search_semaphore
            .clone()
            .acquire_owned()
            .await
            .unwrap();

        let host2 = host.clone();
        let handle = tokio::spawn(async move {
            host2
                .call_tool(
                    "read_file",
                    serde_json::json!({"target_file": path.to_string_lossy()}),
                    "ws-gate-2",
                )
                .await
        });

        // read_file is not gated — it completes while the permit is held.
        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(result.output.contains("gate"), "{}", result.output);
        drop(permit);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-11 (direction correction): live DeepSeek backend check —
    /// the web_search executor against the real `/v1/responses` endpoint
    /// with the real key. Evidence that the request shape (async-openai
    /// `CreateResponseArgs`) is accepted and the raw-JSON parse extracts
    /// content + citations (the typed Responses parse does not match the
    /// DeepSeek backend). Double-gated: `--ignored` + `ORZ_TEST_LIVE=1`.
    #[tokio::test]
    #[ignore = "live: requires a real DeepSeek key (ORZ_TEST_LIVE=1)"]
    async fn live_deepseek_web_search_roundtrip() {
        if std::env::var("ORZ_TEST_LIVE").as_deref() != Ok("1") {
            eprintln!("skipping live web search test (ORZ_TEST_LIVE != 1)");
            return;
        }
        let key = orz_loop::gateway::credentials::read_agent_api_key()
            .expect("DeepSeek key (orz-deepseek/agent)");
        let config = WebSearchConfig::Enabled {
            api_key: key,
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-flash".to_string(),
            extra_headers: Default::default(),
            alpha_test_key: None,
        };
        let client =
            orz_tools::implementations::web_search::client::WebSearchClient::new(&config, None)
                .expect("client build");
        let (content, citations) = client
            .search("2026 年 xAI 最新发布的技术", None)
            .await
            .expect("live search");
        assert!(!content.is_empty(), "search content must not be empty");
        assert!(
            !citations.is_empty(),
            "DeepSeek open_page citations expected; content head: {}",
            &content[..content.len().min(80)]
        );
    }

    /// ADR-0006 (2026-08-11): the host's redacted-config accessor — the
    /// only sanctioned serialization exit (the raw config's Debug contains
    /// the api_key and must never be logged or journaled).
    #[tokio::test]
    async fn host_web_search_config_redacted_never_leaks_key() {
        struct OkReader;
        impl crate::credentials::CredentialReader for OkReader {
            fn read(&self) -> Result<String, crate::credentials::CredentialError> {
                Ok("sk-test-9f8e7d6c5b4a".to_string())
            }
        }
        let dir = test_dir();
        let host = OrzHost::with_credential_reader(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
            None,
            Arc::new(OkReader),
        )
        .expect("host build");
        assert!(host.web_search_configured());
        let redacted = host
            .web_search_config_redacted()
            .expect("enabled config redacted");
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(json.contains("***REDACTED***"), "{json}");
        assert!(!json.contains("sk-test-9f8e7d6c5b4a"), "{json}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C2-1 (2026-08-11, ADR-0006 web-search slice): end-to-end through the
    /// shared agent loop with a REAL host — the main lane delegates
    /// web_search to the external retrieval subagent, and the subagent
    /// SELF-EXECUTES the web tool through the host (no
    /// `nested_subagent_dispatch_refused`). With the failing credential
    /// reader the client is not injected, so the call fast-fails at the
    /// toolset sink ("missing required resource") — proving the routing
    /// change without any real API call.
    #[tokio::test]
    async fn retrieval_lane_web_search_routes_to_host_and_fast_fails() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = host_with_failing_reader(journal, &dir);

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            orz_loop::gateway::fake::ScriptedResponse::tool_calls(vec![ToolCall {
                name: "web_search".to_string(),
                arguments: serde_json::json!({"query": "t"}),
                call_id: "call-1".to_string(),
            }]),
            orz_loop::gateway::fake::ScriptedResponse::tool_calls(vec![ToolCall {
                name: "web_search".to_string(),
                arguments: serde_json::json!({"query": "t"}),
                call_id: "call-2".to_string(),
            }]),
            orz_loop::gateway::fake::ScriptedResponse::text("检索完成"),
            orz_loop::gateway::fake::ScriptedResponse::text("完成"),
            orz_loop::gateway::fake::ScriptedResponse::text("完成"),
        ]));
        let controller = orz_loop::AgentLoopController::with_gateway(gateway).with_retrieval_mode(
            orz_loop::controller::RetrievalMode::FrameworkFallback,
            orz_loop::controller::RetrievalCapability::Available,
            false,
            None,
            None,
        );
        controller
            .run_turn(
                &host,
                "查一下",
                "RUN-R2H",
                "manifest-sha",
                0,
                None,
                None,
                None,
            )
            .await
            .expect("run turn");

        let content = std::fs::read_to_string(dir.join("j").join("events.jsonl")).unwrap();
        let events: Vec<RunEvent> = content
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        // The web tool was never refused as a nested dispatch.
        assert!(
            !events.iter().any(|e| {
                e.payload.get("error").and_then(|v| v.as_str())
                    == Some("nested_subagent_dispatch_refused")
            }),
            "the lane-internal web call must self-execute, not be nested-refused"
        );
        // The lane-internal call (call-2) reached the REAL toolset and
        // fast-failed: client not injected (failing reader) →
        // "missing required resource" at the call_tool sink.
        let failure = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-2")
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("web_search host failure journaled");
        let err = failure
            .payload
            .get("error")
            .and_then(|v| v.as_str())
            .expect("error field");
        assert!(
            err.contains("missing required resource"),
            "web_search must reach the toolset sink: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn call_read_file_returns_content() {
        let dir = test_dir();
        let path = dir.join("test.txt");
        std::fs::write(&path, "hello orz tools").unwrap();

        let toolset = shared_toolset();
        let result = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-1",
                None,
            )
            .await
            .expect("read_file call");
        assert!(result.prompt_text.contains("hello orz tools"), "{result:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// ORZ-LARGE-FILE-READ-CONTRACT (ADR-0010 §14.22): a file above the
    /// coarse gate returns the bounded read-handle envelope through the host
    /// toolset — never full content.
    #[tokio::test]
    async fn call_read_file_large_file_returns_handle_envelope() {
        let dir = test_dir();
        let path = dir.join("big.txt");
        let content = format!("{}\n", "x".repeat(200)).repeat(200);
        assert!(content.len() > 16 * 1024);
        std::fs::write(&path, &content).unwrap();

        let toolset = shared_toolset();
        let result = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": path}),
                "call-big",
                None,
            )
            .await
            .expect("read_file call");
        assert!(
            result.prompt_text.contains("[read handle]"),
            "large file must return the read-handle envelope, got: {result:?}"
        );
        assert!(
            result.prompt_text.len() <= 8 * 1024,
            "envelope must stay bounded, got {} bytes",
            result.prompt_text.len()
        );
        assert!(
            result.prompt_text.contains("truncated=true"),
            "large multi-line file must report truncated, got: {result:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-08 blackboard-partition review closure (conformance D1-1):
    /// the host derives the ToolResult exit_code from the structured output
    /// — search_replace reports "applied" only via EditsApplied (Ok outputs
    /// like FileNotFound/NoMatchesFound changed nothing → non-zero), every
    /// other successful output is 0. Previously hardcoded `None` made the
    /// controller's edit-action gate dead in production.
    #[tokio::test]
    async fn call_tool_maps_real_exit_codes() {
        let dir = test_dir();
        let toolset = shared_toolset();

        // read_file (generic success) → Some(0).
        let read = dir.join("readme.txt");
        std::fs::write(&read, "x").unwrap();
        let ok = toolset
            .call(
                "read_file",
                serde_json::json!({"target_file": read}),
                "c-r",
                None,
            )
            .await
            .expect("read_file");
        assert_eq!(crate::tools::exit_code_from_output(&ok.output), Some(0));

        // search_replace on a missing file — Ok(FileNotFound), changed
        // nothing → non-zero ("实际变动" gate stays closed).
        let missing = toolset
            .call(
                "search_replace",
                serde_json::json!({
                    "file_path": dir.join("nope.txt"),
                    "old_string": "a",
                    "new_string": "b",
                }),
                "c-m",
                None,
            )
            .await
            .expect("search_replace on missing file");
        assert_eq!(
            crate::tools::exit_code_from_output(&missing.output),
            Some(1),
            "a search_replace that applied nothing must be non-zero"
        );

        // A real applied edit → EditsApplied → Some(0).
        let target = dir.join("edit.txt");
        std::fs::write(&target, "before").unwrap();
        let applied = toolset
            .call(
                "search_replace",
                serde_json::json!({
                    "file_path": target,
                    "old_string": "before",
                    "new_string": "after",
                }),
                "c-a",
                None,
            )
            .await
            .expect("search_replace applied");
        assert_eq!(
            crate::tools::exit_code_from_output(&applied.output),
            Some(0),
            "an applied edit must be zero"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn orz_host_full_loophost_chain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join(".gsa").join("runs").join("RUN-T"));
        let host =
            OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host build");

        // Two texts — the counterexample gate (§4.6) intercepts the first.
        let gateway: Arc<dyn ModelGateway> =
            Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let controller = AgentLoopController::with_gateway(gateway);
        let (response, _, _) = controller
            .run_turn(&host, "hi", "RUN-T", "manifest-sha", 0, None, None, None)
            .await
            .expect("run turn");
        assert_eq!(response, "完成");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: a host WITHOUT a permission bridge must fail closed —
    /// never auto-allow (review P1-1).
    #[tokio::test]
    async fn host_without_bridge_fails_closed_on_permission() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).expect("host");

        for (risk, tool) in [
            (RiskClass::ReadOnly, "read_file"),
            (RiskClass::SandboxEscape, "bash"),
        ] {
            let decision = host
                .request_permission(risk, tool, &serde_json::json!({}))
                .await
                .expect("request_permission");
            assert_eq!(
                decision,
                PermitDecision::Deny,
                "no bridge → fail closed for {tool}"
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P1 permit keystore injection: a permit issued through the host signs
    /// with the injected keystore signer and self-verifies; the artifact
    /// lands under `{cwd}/.gsa/one_shot_permit/` (Python namespace layout).
    #[tokio::test]
    async fn issue_permit_uses_injected_keystore_signer() {
        use orz_assurance::permit::verify_sensitive_action_permit;

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.join("j"));
        let host = OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted)
            .expect("host build")
            .with_permit_signer(Arc::new(
                MemoryInstallationKeyStore::from_secret(&[0x42u8; 32]).unwrap(),
            ));

        let envelope = PermitEnvelope {
            conversation_id: "conv-1".into(),
            envelope_id: "env-1".into(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        };
        let digest = |s: &str| orz_assurance::sha256_hex(s.as_bytes());
        let permit = host
            .issue_permit(
                &envelope,
                &digest("confirmation"),
                &digest("action"),
                &digest("target"),
                &digest("impact scope"),
                1,
                300,
            )
            .expect("issue permit");

        assert!(
            permit.integrity.key_id.starts_with("KEY-"),
            "key id: {}",
            permit.integrity.key_id
        );
        let verification =
            verify_sensitive_action_permit(host.permit_signer().as_ref(), &envelope, &permit, None);
        assert!(verification.valid, "{:?}", verification.errors);
        let artifact = dir
            .join(".gsa")
            .join("one_shot_permit")
            .join(format!("{}.issued.json", permit.permit_id));
        assert!(artifact.is_file(), "missing permit artifact");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Phase 3 wiring: host WITH the bridge — Read auto-allows through the
    /// permission manager, Bash (no interactive client) Ask → Deny (IP6).
    #[tokio::test]
    async fn host_with_bridge_auto_allows_read_denies_bash() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let journal = JournalRecorder::new(dir.join("j"));
                let host = OrzHost::with_bridge(
                    "sess-1",
                    journal,
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    None, // headless dead gateway
                )
                .expect("host with bridge");

                let read = host
                    .request_permission(
                        RiskClass::ReadOnly,
                        "read_file",
                        &serde_json::json!({"target_file": "a.txt"}),
                    )
                    .await
                    .expect("read request");
                assert_eq!(read, PermitDecision::AllowOnce);

                let bash = host
                    .request_permission(
                        RiskClass::SandboxEscape,
                        "run_terminal_cmd",
                        &serde_json::json!({"command": "dir"}),
                    )
                    .await
                    .expect("bash request");
                assert_eq!(bash, PermitDecision::Deny, "headless Ask → Deny");

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }

    /// Streaming slice: `on_text_delta` forwards the chunk to the live
    /// client as an `agent_message_chunk` session notification, in order.
    #[tokio::test]
    async fn on_text_delta_forwards_session_notification_to_gateway() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let dir = test_dir();
                let (tx, mut rx) =
                    tokio::sync::mpsc::unbounded_channel::<xai_acp_lib::AcpClientMessage>();
                let sender = xai_acp_lib::AcpAgentGatewaySender::new(tx);
                let host = OrzHost::with_bridge(
                    "sess-1",
                    JournalRecorder::new(dir.join("j")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                    Some(sender),
                )
                .expect("host with bridge");

                host.on_text_delta("你好");
                host.on_text_delta("世界");

                for expected in ["你好", "世界"] {
                    let msg = rx.recv().await.expect("notification");
                    match msg {
                        xai_acp_lib::AcpClientMessage::SessionNotification(args) => {
                            assert_eq!(args.request.session_id.0.as_ref(), "sess-1");
                            match args.request.update {
                                acp::SessionUpdate::AgentMessageChunk(chunk) => {
                                    match chunk.content {
                                        acp::ContentBlock::Text(t) => {
                                            assert_eq!(t.text, expected);
                                        }
                                        other => panic!("unexpected content block: {other:?}"),
                                    }
                                }
                                other => panic!("unexpected update: {other:?}"),
                            }
                        }
                        other => panic!("unexpected message: {other:?}"),
                    }
                }

                // Headless host (no gateway): on_text_delta is a silent no-op.
                let headless = OrzHost::new(
                    JournalRecorder::new(dir.join("j2")),
                    &dir,
                    WorkspaceTrust::ObservedTrusted,
                )
                .expect("headless host");
                headless.on_text_delta("丢弃");

                let _ = std::fs::remove_dir_all(&dir);
            })
            .await
    }

    /// Stability fix (2026-08-07): a hung test child plus its grandchildren
    /// must ALL be terminated on timeout — killing only the direct child
    /// left an orphan holding the capture pipes (the 52-minute forth hang:
    /// harness killed orz, orz's hung pytest kept the pipes open, the
    /// harness's communicate() blocked on EOF forever).
    #[cfg(windows)]
    #[tokio::test]
    async fn run_tests_timeout_kills_process_tree() {
        let dir = test_dir();
        let pidfile = dir.join("gc.pid");
        let script = format!(
            // Child spawns a grandchild that inherits our stdout/stderr
            // pipes and writes its pid to the pidfile; both then sleep
            // forever (deadlock-style hang).
            "import subprocess, sys, time, pathlib, os; \
             g = subprocess.Popen([sys.executable, '-c', \
             'import time, pathlib, os; pathlib.Path(r\"{pf}\").write_text(str(os.getpid())); \
             [time.sleep(1) for _ in range(999999)]'], \
             stdout=sys.stdout, stderr=sys.stderr); \
             [time.sleep(1) for _ in range(999999)]",
            pf = pidfile.display().to_string().replace('\\', "/"),
        );
        let runner = orz_loop::host::TestRunner {
            command: vec!["python".to_string(), "-c".to_string(), script.clone()],
            timeout: Some(std::time::Duration::from_secs(2)),
            env: Vec::new(),
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        assert!(
            result.output.contains("TIMED OUT"),
            "expected TIMED OUT marker: {}",
            result.output
        );
        assert_eq!(result.exit_code, None);
        // Give TaskKill a moment to reap the tree, then verify the
        // grandchild (the pipe holder) is gone.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let gcid: i64 = std::fs::read_to_string(&pidfile)
            .expect("grandchild pid written")
            .trim()
            .parse()
            .expect("pid parses");
        let listing = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {gcid}"), "/NH"])
            .output()
            .expect("tasklist runs");
        let text = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !text.contains(&gcid.to_string()),
            "grandchild {gcid} survived the tree kill: {text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-002 (2026-08-11): the fixed test command runs env_clear()ed — the
    /// test process sees ONLY the platform allowlist (TEST_ENV_ALLOWLIST)
    /// plus the harness's explicit `TestRunner::env` entries. Host
    /// environment (session variables, arbitrary paths, secrets) is never
    /// inherited.
    #[tokio::test]
    async fn run_tests_env_is_isolated_from_host() {
        let dir = test_dir();
        let runner = orz_loop::host::TestRunner {
            command: vec![
                "python".to_string(),
                "-c".to_string(),
                "import os; print('KEYS:' + ','.join(sorted(os.environ.keys()))); \
                 print('INJECTED:' + os.environ.get('ORZ_TEST_RUNNER_INJECTED', '<absent>'))"
                    .to_string(),
            ],
            timeout: Some(std::time::Duration::from_secs(30)),
            env: vec![(
                "ORZ_TEST_RUNNER_INJECTED".to_string(),
                "visible".to_string(),
            )],
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        let keys_line = result
            .output
            .lines()
            .find(|l| l.starts_with("KEYS:"))
            .expect("KEYS line in output");
        let keys: Vec<&str> = keys_line["KEYS:".len()..].split(',').collect();
        // Windows env vars are case-insensitive — compare uppercase.
        let mut allowed: std::collections::HashSet<String> = TEST_ENV_ALLOWLIST
            .iter()
            .map(|k| k.to_uppercase())
            .collect();
        allowed.insert("ORZ_TEST_RUNNER_INJECTED".to_string());
        for key in &keys {
            assert!(
                allowed.contains(&key.to_uppercase()),
                "test process saw host env var '{key}' that is not in the allowlist"
            );
        }
        assert!(
            keys.iter().any(|k| k.eq_ignore_ascii_case("PATH")),
            "PATH (command lookup) is always in the allowlist"
        );
        assert!(
            result.output.contains("INJECTED:visible"),
            "harness-injected env entry must be visible: {}",
            result.output
        );
        assert!(
            !result.output.contains("ORZ_TEST_RUNNER="),
            "the runner's own env var must not leak into the test process"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-003 (2026-08-11): the workspace delta records added/modified/
    /// deleted files across a run_tests call (ADR-0010 §3.8.2
    /// workspace-delta recording — tests may write files, populate caches,
    /// etc.; the change list is the audit trace).
    #[tokio::test]
    async fn run_tests_records_workspace_delta() {
        let dir = test_dir();
        std::fs::write(dir.join("existing.txt"), "v1").unwrap();
        std::fs::write(dir.join("todelete.txt"), "x").unwrap();
        let runner = orz_loop::host::TestRunner {
            command: vec![
                "python".to_string(),
                "-c".to_string(),
                "from pathlib import Path; \
                 Path('new.txt').write_text('n'); \
                 Path('existing.txt').write_text('version two - longer'); \
                 Path('todelete.txt').unlink()"
                    .to_string(),
            ],
            timeout: Some(std::time::Duration::from_secs(30)),
            env: Vec::new(),
        };
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_test_runner(Some(runner));
        let result = host.run_tests().await.expect("run_tests returns a result");
        use orz_loop::host::WorkspaceDeltaKind;
        let kinds: Vec<(String, WorkspaceDeltaKind)> = result
            .workspace_delta
            .iter()
            .map(|e| (e.path.clone(), e.kind.clone()))
            .collect();
        assert!(
            kinds.contains(&("new.txt".to_string(), WorkspaceDeltaKind::Added)),
            "added file in delta: {kinds:?}"
        );
        assert!(
            kinds.contains(&("existing.txt".to_string(), WorkspaceDeltaKind::Modified)),
            "modified file in delta: {kinds:?}"
        );
        assert!(
            kinds.contains(&("todelete.txt".to_string(), WorkspaceDeltaKind::Deleted)),
            "deleted file in delta: {kinds:?}"
        );
        assert!(
            !result.workspace_delta_truncated,
            "small delta must not be flagged truncated"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RT-003: the delta diff caps the entry list (deterministic order,
    /// truncation flag) so a test that churns thousands of files cannot
    /// drown the journal event.
    #[test]
    fn workspace_delta_diff_caps_and_sorts() {
        let mut before = std::collections::HashMap::new();
        let mut after = std::collections::HashMap::new();
        // Seed one untouched file (not in the diff).
        before.insert("stable.txt".to_string(), (1, 1, 0));
        after.insert("stable.txt".to_string(), (1, 1, 0));
        // More changes than the cap — all "added".
        for i in 0..(RUN_TESTS_DELTA_MAX_ENTRIES + 50) {
            after.insert(format!("gen/{i}.txt"), (1, 2, 0));
        }
        let (entries, truncated) = workspace_delta_diff(&before, &after);
        assert!(truncated, "over-cap diff must be flagged truncated");
        assert_eq!(entries.len(), RUN_TESTS_DELTA_MAX_ENTRIES);
        // Deterministic order.
        let mut sorted = entries.clone();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(entries, sorted);
        // Deleted detection — a separate small diff (an over-cap diff
        // truncates away later-sorted entries, which is the cap working).
        let mut before2 = std::collections::HashMap::new();
        before2.insert("gone.txt".to_string(), (1, 1, 0));
        before2.insert("stable.txt".to_string(), (1, 1, 0));
        let after2 = std::collections::HashMap::from([("stable.txt".to_string(), (1, 1, 0))]);
        let (entries, truncated) = workspace_delta_diff(&before2, &after2);
        assert!(!truncated);
        assert!(
            entries
                .iter()
                .any(|e| e.path == "gone.txt"
                    && e.kind == orz_loop::host::WorkspaceDeltaKind::Deleted),
            "deleted file detected"
        );
    }

    /// P0-1 (2026-08-08 stall guards): a tool call that exceeds the host's
    /// per-call wall-clock budget must fail with `ToolError::Timeout` (the
    /// reason carried for the journal / model), kill the tool's process
    /// tree INCLUDING grandchildren (the orphan-holds-pipes hang
    /// mechanism), and leave the session usable for the next call.
    #[cfg(windows)]
    #[tokio::test]
    async fn call_tool_timeout_kills_process_tree() {
        let dir = test_dir();
        // A python script that spawns a grandchild (which writes its pid
        // and sleeps forever), then sleeps forever itself — a deadlock-style
        // hang inside run_terminal_cmd.
        let pidfile = dir.join("gc.pid");
        let script = dir.join("hang.py");
        let script_body = format!(
            "import subprocess, sys, time, pathlib, os; \
             g = subprocess.Popen([sys.executable, '-c', \
             'import time, pathlib, os; pathlib.Path(r\"{pf}\").write_text(str(os.getpid())); \
             [time.sleep(1) for _ in range(999999)]'], \
             stdout=sys.stdout, stderr=sys.stderr); \
             [time.sleep(1) for _ in range(999999)]",
            pf = pidfile.display().to_string().replace('\\', "/"),
        );
        std::fs::write(&script, &script_body).unwrap();

        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        // 2s (not 500ms — 2026-08-08 review P3-6b): under parallel-test CPU
        // contention the 500ms budget could expire before python even
        // spawned the grandchild, making the pidfile assertion below panic
        // spuriously. 2s is comfortably past python's cold start while far
        // below any real tool window.
        .expect("host")
        .with_tool_timeout(std::time::Duration::from_millis(2000));
        let result = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": format!("python {}", script.display()),
                    "description": "stall-guard timeout test",
                }),
                "call-t1",
            )
            .await;
        let err = result.expect_err("hung tool must time out");
        assert!(
            err.to_string().contains("TIMED OUT"),
            "expected TIMED OUT marker: {err}"
        );
        // The session survives — including for PROCESS-type tools (2026-08-08
        // review F1: `kill_active` must not latch the scope, or every later
        // spawn would die on arrival). The follow-up is a real
        // `run_terminal_cmd` — the original review flagged that a `read_file`
        // follow-up (no process) could not catch the latch.
        let follow_up = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo orz-alive",
                    "description": "post-timeout liveness",
                }),
                "call-t2",
            )
            .await
            .expect("process-type tool works after the timeout");
        assert!(
            follow_up.output.contains("orz-alive"),
            "follow-up bash returned the expected output: {follow_up:?}"
        );
        // Give the kill a moment to reap the tree, then verify the
        // grandchild (the pipe holder) is gone.
        tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
        let gcid: i64 = std::fs::read_to_string(&pidfile)
            .expect("grandchild pid written")
            .trim()
            .parse()
            .expect("pid parses");
        let listing = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {gcid}"), "/NH"])
            .output()
            .expect("tasklist runs");
        let text = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !text.contains(&gcid.to_string()),
            "grandchild {gcid} survived the tool-timeout tree kill: {text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P0-C S4 (2026-08-16): a per-call timeout override is honored — the
    /// host is configured with a LONG budget (10s), but
    /// `call_tool_with_timeout` with a 1.2s override must kill the process
    /// tree at the override bound, fail with `ToolError::Timeout` carrying
    /// the override budget, and leave the session usable.
    #[cfg(windows)]
    #[tokio::test]
    async fn call_tool_with_timeout_override_is_honored() {
        let dir = test_dir();
        let script = dir.join("hang.py");
        std::fs::write(
            &script,
            "import time\n[time.sleep(1) for _ in range(999999)]\n",
        )
        .unwrap();
        let host = OrzHost::new(
            JournalRecorder::new(dir.join("j")),
            &dir,
            WorkspaceTrust::ObservedTrusted,
        )
        .expect("host")
        .with_tool_timeout(std::time::Duration::from_secs(10));
        let started = std::time::Instant::now();
        let result = host
            .call_tool_with_timeout(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": format!("python {}", script.display()),
                    "description": "per-call override timeout test",
                }),
                "call-t3",
                Some(std::time::Duration::from_millis(1200)),
            )
            .await;
        let err = result.expect_err("override bound must time out");
        assert!(
            err.to_string().contains("TIMED OUT after 1.2s"),
            "error carries the override budget: {err}"
        );
        assert!(
            started.elapsed() < std::time::Duration::from_secs(5),
            "override bound respected: {:?}",
            started.elapsed()
        );
        // The session survives the override kill (kill_active not latched).
        let follow_up = host
            .call_tool(
                "run_terminal_cmd",
                serde_json::json!({
                    "command": "echo orz-alive",
                    "description": "post-override-timeout liveness",
                }),
                "call-t4",
            )
            .await
            .expect("process-type tool works after the override timeout");
        assert!(follow_up.output.contains("orz-alive"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
