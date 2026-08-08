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
pub mod keystore;
pub mod permission;
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
        let toolset = tools::build_toolset(cwd)?;
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
        })
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
        // F-09 (2026-08-07 review): bounded execution + context gating. The
        // command runs under a wall-clock cap (default 30min — the cap only
        // catches true hangs; context size, not wall time, is the priority);
        // a hung child is killed. Output is collected with a TAIL cap (1MB)
        // so pathological output cannot blow session memory; the full
        // (capped) output is written to a file the model can read
        // (read_file), and the controller injects only the final 32KB into
        // the conversation.
        let mut child = tokio::process::Command::new(&runner.command[0])
            .args(&runner.command[1..])
            .current_dir(&self.cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
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
            tracing::warn!("run_tests: job-object assignment failed ({e}); \
                            falling back to TaskKill on timeout");
            // Note: `supervisor` is deliberately not rebound here — the
            // assignment failure leaves a live job with no members, which is
            // harmless to drop; the TaskKill path covers the timeout case.
        }
        let stdout = child.stdout.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stdout pipe unavailable".into())
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ToolError::ExecutionFailed("test runner stderr pipe unavailable".into())
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
            Ok(status) => status
                .map_err(|e| ToolError::ExecutionFailed(format!("test runner wait: {e}")))?,
            Err(_) => {
                // Stability fix (2026-08-07): `Child::kill` terminates only
                // the direct child — grandchildren that inherited our capture
                // pipes (e.g. a pytest spawned by the runner) survive as
                // orphans and keep `read_capped` blocked on EOF forever (the
                // 52-minute forth hang). Kill the whole tree on Windows via
                // TaskKill; unix keeps the plain kill (recorded limitation).
                kill_process_tree(&mut child).await;
                return Ok(orz_loop::host::TestRunResult {
                    output: format!(
                        "[test runner TIMED OUT after {timeout:?} — process tree killed; \
                         partial output follows]\n{}",
                        String::from_utf8_lossy(&out_buf),
                    ),
                    exit_code: None,
                    full_output_path: None,
                });
            }
        };
        let mut text = String::from_utf8_lossy(&out_buf).into_owned();
        if !err_buf.is_empty() {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&String::from_utf8_lossy(&err_buf));
        }
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
        Ok(orz_loop::host::TestRunResult {
            output: text,
            exit_code: status.code(),
            full_output_path,
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
        let fut = self.registry.toolset().call(name, args, call_id, None);
        let result = match tokio::time::timeout(self.tool_timeout, fut).await {
            Ok(result) => result.map_err(|e| crate::tools::map_tool_error(&e))?,
            Err(_) => {
                tracing::warn!(
                    tool = name,
                    timeout = ?self.tool_timeout,
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
                    timeout = self.tool_timeout,
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
        })
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
    use orz_loop::AgentLoopController;
    use orz_loop::gateway::fake::FakeProvider;
    use orz_loop::gateway::model::ModelGateway;
    use std::sync::OnceLock;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-host-lib-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Toolset construction is expensive (builder finalize) — share one.
    static SHARED_TOOLSET: OnceLock<Arc<orz_tools::registry::types::FinalizedToolset>> =
        OnceLock::new();

    fn shared_toolset() -> &'static Arc<orz_tools::registry::types::FinalizedToolset> {
        SHARED_TOOLSET
            .get_or_init(|| tools::build_toolset(&std::env::temp_dir()).expect("shared toolset"))
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
            .run_turn(&host, "hi", "RUN-T", "manifest-sha", 0, None)
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
}
