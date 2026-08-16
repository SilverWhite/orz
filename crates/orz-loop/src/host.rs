//! LoopHost trait — the interface orz-host must implement.
//!
//! Defined in orz-loop per Codex discipline: orz-loop owns the agent loop contract;
//! orz-host bridges Grok providers to fulfill it.
//!
//! See: INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §3.4

use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use orz_assurance::journal::JournalRecorder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Risk classification for permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClass {
    ReadOnly,
    LocalMutation,
    NetworkCall,
    SandboxEscape,
}

/// Session permission policy (IP2a, FIX_PLAN 2026-08-06 D-3) — the policy
/// the host's permission bridge enforces for this session. The loop uses it
/// for the NAME-LEVEL tool-availability projection: tools the policy refuses
/// outright (e.g. network/shell under Benchmark) are filtered from the
/// model-visible tool declarations, so the model never attempts them.
/// Mirrors orz-host's `PermissionPolicy` (defined here to avoid a
/// loop→host dependency; the host maps its policy onto this enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolPolicy {
    /// Normal prompt-decides behavior — all tools are declared; denial is
    /// scope/argument-level at permission time.
    #[default]
    Interactive,
    /// Read-only sandbox — only read-class tools are declared.
    ReadOnly,
    /// Headless benchmark — read + local file edits declared; network and
    /// shell-escape excluded by name.
    Benchmark,
}

/// The refusing gate family behind a structured policy denial
/// (P0-C S3 前置, 2026-08-15, P1-2 定案).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDenialSource {
    /// The per-call permission bridge (user deny / headless defer).
    Permission,
    /// The ACAF ticket gate (Slice 2 fail-closed refusals).
    Acaf,
    /// The retrieval-mode gates (off / framework_fallback / local_browser).
    RetrievalMode,
    /// Reserved: taint action-combination policy (design item, not yet
    /// wired at runtime).
    Taint,
}

impl PolicyDenialSource {
    /// Machine-readable source key (`permission` | `acaf` |
    /// `retrieval_mode` | `taint`) — matches the serde snake_case names.
    pub fn as_str(&self) -> &'static str {
        match self {
            PolicyDenialSource::Permission => "permission",
            PolicyDenialSource::Acaf => "acaf",
            PolicyDenialSource::RetrievalMode => "retrieval_mode",
            PolicyDenialSource::Taint => "taint",
        }
    }
}

/// Structured policy denial (P0-C S3 前置, 2026-08-15, P1-2 定案):
/// `run_host_tool` refusals carry `{source, code, reason}` instead of
/// relying on stable output prefixes; the console adapter maps ONLY this
/// structured signal to `ExecuteError::PolicyDenied`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyDenial {
    pub source: PolicyDenialSource,
    pub code: String,
    pub reason: String,
}

/// Result returned by a tool invocation.
#[derive(Debug, Clone, Default)]
pub struct ToolResult {
    pub output: String,
    pub exit_code: Option<i32>,
    /// P0-C S4 (2026-08-16): true when the host hit a per-call wall-clock
    /// bound and killed the tool's process tree before completion. The
    /// console adapter maps this structured signal to `tool_timeout` /
    /// `script_timeout` (never text prefix parsing).
    pub timed_out: bool,
    /// Decode stage that produced `output` (GAP-ENCODING-GATE,
    /// OPS-PROTOCOL §8): `utf-8` / `utf-8-sig` / `gb18030` / `utf-8-lossy`
    /// (comma-joined for multi-chunk streams). `None` when the tool has no
    /// mechanical decode stage. The controller journals it as
    /// `tool_completed.output_encoding`.
    pub output_encoding: Option<String>,
    /// FUS-RETRIEVAL-MECH B-1 (2026-08-13): optional structured tool
    /// metadata forwarded by the host across the loop seam. Today only
    /// `web_search` fills it — `{"citations": ["https://…", …]}` — the
    /// citation URL candidate pool the mechanical prefilter will consume.
    /// All other tools leave it `None`; it is never derived from the
    /// model-visible output text.
    pub structured: Option<serde_json::Value>,
    /// P0-C S3 前置 (2026-08-15, P1-2 定案): structured policy denial when
    /// the call was refused by a policy gate (permission / ACAF ticket /
    /// retrieval mode; taint reserved). `None` for every executed call and
    /// for non-policy refusals (candidate cap, inject budget, runner
    /// availability). The console adapter classifies policy refusals ONLY
    /// from this field — the old stable-output-prefix judgment is retired.
    pub policy_denial: Option<PolicyDenial>,
}

/// Lightweight error from tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("tool execution failed: {0}")]
    ExecutionFailed(String),
    /// P0-1 (2026-08-08 stall guards): the tool exceeded the host's
    /// per-call wall-clock budget and its process tree was killed. The
    /// reason carries the budget so the journal (`tool_completed.error`)
    /// and the model-facing message are self-explanatory.
    #[error("tool timeout: {0}")]
    Timeout(String),
}

/// Permission decision returned by the host's approval prompter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermitDecision {
    AllowOnce,
    AllowAlways,
    Deny,
    Defer,
}

#[derive(Debug, thiserror::Error)]
pub enum PermitError {
    #[error("permission denied by user")]
    Denied,
    #[error("permission timeout")]
    Timeout,
}

/// A record of a single turn (prompt + response + tool calls).
#[derive(Debug, Clone)]
pub struct TurnRecord {
    pub turn_id: String,
    pub prompt: String,
    pub response_text: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
}

#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub arguments: Value,
    pub result: Option<ToolResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum PersistError {
    #[error("persist io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error("compaction not supported in this session")]
    NotSupported,
}

/// A hook event from the agent lifecycle.
#[derive(Debug, Clone)]
pub struct HookEvent {
    pub event_type: HookEventType,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub enum HookEventType {
    PreToolUse,
    PostToolUse,
    SessionStart,
    SessionStop,
    TurnStart,
    TurnEnd,
}

/// Result of running a hook.
#[derive(Debug, Clone)]
pub struct HookResult {
    pub hook_name: String,
    pub outcome: HookOutcome,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub enum HookOutcome {
    Allowed,
    Blocked,
    Modified,
}

/// Credentials bundle (e.g. API keys).
#[derive(Debug, Clone)]
pub struct Credentials {
    pub provider: String,
    pub api_key: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("credential not found for provider: {0}")]
    NotFound(String),
}

/// Tool definition (name + description + parameters schema).
#[derive(Debug, Clone)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

/// F-09 (2026-08-07 review): default wall-clock cap for the fixed test
/// command (30min — user decision: context size is the priority, not wall
/// time; the cap only catches true hangs). Hosts may override per
/// `TestRunner::timeout`.
pub const RUN_TESTS_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// F-09: cap on the test output the host collects (tail kept) — bounds
/// session memory against pathological test output.
pub const RUN_TESTS_OUTPUT_CAP: usize = 1024 * 1024;

/// F-09: cap on the test output injected into the conversation (tail kept —
/// test frameworks put their summary at the end). The full (capped) output
/// stays on disk; the model fetches it via read_file when it wants more.
pub const RUN_TESTS_CONTEXT_CAP: usize = 32 * 1024;

/// A fixed test-runner command (D-9, FIX_PLAN 2026-08-06) — the Aider-model
/// feedback loop inside a single run. The host owns the command (the model
/// never sees the test files, only stdout/stderr/exit code via the
/// `run_tests` tool); the harness injects it.
#[derive(Debug, Clone)]
pub struct TestRunner {
    /// argv-style command (e.g. `["python", "-m", "pytest", "<hidden-tests>/x_test.py", "-q"]`).
    pub command: Vec<String>,
    /// Wall-clock cap for one execution (None = `RUN_TESTS_TIMEOUT`).
    pub timeout: Option<Duration>,
    /// RT-002 (2026-08-11): explicit environment allowlist entries — the
    /// harness's required test-environment variables (PYTHONPATH, venv
    /// activation, sitecustomize shim, …). The host runs the command with
    /// `env_clear()` plus a fixed minimal platform allowlist (PATH and the
    /// like) PLUS these entries; the host's full environment (secrets, host
    /// paths) is never inherited.
    pub env: Vec<(String, String)>,
}

/// One workspace change observed across a `run_tests` invocation
/// (RT-003, 2026-08-11) — tests may write files, populate caches, etc.;
/// the change list is the audit trace of that side effect.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WorkspaceDeltaEntry {
    /// Worktree-relative path (forward-slash normalized on Windows).
    pub path: String,
    pub kind: WorkspaceDeltaKind,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceDeltaKind {
    Added,
    Modified,
    Deleted,
}

/// Result of running the fixed test command.
#[derive(Debug, Clone, Default)]
pub struct TestRunResult {
    pub output: String,
    pub exit_code: Option<i32>,
    /// Decoding stage(s) that produced `output` (GAP-ENCODING-GATE,
    /// OPS-PROTOCOL §8): `utf-8` / `utf-8-sig` / `gb18030` / `utf-8-lossy`,
    /// comma-joined when stdout/stderr used different stages.
    pub output_encoding: Option<String>,
    /// Path of the full (capped) output written by the host — readable via
    /// `read_file`; `None` when the host could not write it (or timed out).
    pub full_output_path: Option<String>,
    /// RT-003: workspace changes the test run caused (capped list; the
    /// overall change count is recoverable via `workspace_delta_truncated`
    /// only as a binary signal — the cap is the only bound).
    pub workspace_delta: Vec<WorkspaceDeltaEntry>,
    /// RT-003: true when the delta list was truncated at the host cap.
    pub workspace_delta_truncated: bool,
}

/// Trait for looking up tools by name.
pub trait ToolRegistry: Send + Sync {
    fn get(&self, name: &str) -> Option<ToolDef>;
    fn list(&self) -> Vec<ToolDef>;
}

/// The contract orz-host implements to provide Grok-backed services
/// to the agent loop controller.
///
/// Phase 1: only `journal()` is fully functional; other methods
/// have default stub implementations that return errors.
#[async_trait]
pub trait LoopHost: Send + Sync {
    /// Access the shared journal recorder.
    fn journal(&self) -> &JournalRecorder;

    /// Look up available tools.
    fn tools_registry(&self) -> &dyn ToolRegistry;

    /// Workspace trust observation state, fed to the instruction provenance
    /// gate (`trusted_project` downgrades unless `observed_trusted`).
    /// Defaults to `not_observed` (fail-closed); orz-host overrides.
    fn workspace_trust(&self) -> orz_assurance::gates::ipg::WorkspaceTrust {
        orz_assurance::gates::ipg::WorkspaceTrust::NotObserved
    }

    /// Session permission policy (IP2a) — drives the name-level tool
    /// availability projection: tools the policy refuses outright are
    /// filtered from the model-visible declarations. Defaults to
    /// `Interactive` (all tools declared; scope-level denial at permission
    /// time); orz-host maps its `PermissionPolicy` onto this.
    fn tool_policy(&self) -> ToolPolicy {
        ToolPolicy::Interactive
    }

    /// D-9 (FIX_PLAN 2026-08-06): a fixed test-runner command the model can
    /// invoke via the `run_tests` tool (Aider `--test-cmd` semantics) —
    /// stdout/stderr/exit code are fed back inside the same run. The tests
    /// themselves stay hidden (outside the workspace; the model never reads
    /// them). `None` (default) omits the tool entirely.
    fn test_runner(&self) -> Option<TestRunner> {
        None
    }

    /// Session working directory — the workspace scope the work-tool
    /// probes check (read chain / write chain / storage chain). Defaults
    /// to the process cwd; hosts must override with the session cwd (which
    /// may differ).
    fn session_cwd(&self) -> PathBuf {
        std::env::current_dir().unwrap_or_default()
    }

    /// Whether the session is attached to an interactive user who can
    /// answer `ask_user_question` and approve plan mode (work-tool probe
    /// source). Defaults to `false` (fail-closed); hosts with a live
    /// client override.
    fn interactive_user(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2 (v0.2 single probe face, ADR-0010 §3.5 v1.8):
    /// whether the host carries a usable terminal backend
    /// (`run_terminal_cmd` probe source). Fail-closed default; hosts wire
    /// their real backend.
    fn terminal_available(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether a workspace language-service backend
    /// is configured (`lsp` probe source). Fail-closed default.
    fn lsp_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether memory is explicitly opted in and a
    /// backend is present (`memory_get` / `memory_search` probe source).
    /// Fail-closed default.
    fn memory_enabled(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether an image-generation backend is
    /// configured (`image_gen` / `image_edit` probe source). Fail-closed
    /// default.
    fn image_backend_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether a video-generation backend is
    /// configured (`image_to_video` / `reference_to_video` probe source).
    /// Fail-closed default.
    fn video_backend_configured(&self) -> bool {
        false
    }

    /// FUS-TOOL-PROBE P0-A-2: whether an MCP / capability registry is
    /// present in session scope (`use_tool` probe source). Fail-closed
    /// default.
    fn mcp_registry_available(&self) -> bool {
        false
    }

    /// D-9: execute the host's fixed test command. Only called when
    /// `test_runner()` returned `Some`. The host runs the command in the
    /// session cwd with the injected argv — never a model-supplied command.
    async fn run_tests(&self) -> Result<TestRunResult, ToolError> {
        Err(ToolError::NotFound("no test runner configured".into()))
    }

    /// Execute a tool call.
    async fn call_tool(
        &self,
        _name: &str,
        _args: Value,
        _call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::NotFound("not implemented".into()))
    }

    /// Execute a tool call under an explicit per-call wall-clock bound
    /// (P0-C S4, 2026-08-16: script step deadlines). `None` = host default
    /// (`call_tool` semantics). Hosts that honor the override MUST bound
    /// the whole call with the shorter of the override and their configured
    /// budget, and on expiry kill the tool's process tree and fail with
    /// `ToolError::Timeout` — a script layer timeout alone must never
    /// substitute for host-side process-tree reclamation. The default
    /// implementation ignores the override: hosts that do NOT override it
    /// do not provide per-step host deadlines (script steps then rely on
    /// step-boundary checks only, and the script runner enforces the total
    /// wall clock ex-post after each step). Production `OrzHost` honors
    /// the override.
    async fn call_tool_with_timeout(
        &self,
        name: &str,
        args: Value,
        call_id: &str,
        _timeout: Option<Duration>,
    ) -> Result<ToolResult, ToolError> {
        self.call_tool(name, args, call_id).await
    }

    /// Request user permission for a risky action.
    ///
    /// Default is fail-closed `Deny`. Hosts must implement their own
    /// permission bridge (e.g. `PermissionBridge` for ACP) to allow tools —
    /// an `AllowOnce` default would silently auto-allow `bash` whenever a
    /// host forgets to wire the bridge (review P1-1, 2026-08-04).
    async fn request_permission(
        &self,
        _risk: RiskClass,
        _tool: &str,
        _args: &Value,
    ) -> Result<PermitDecision, PermitError> {
        Ok(PermitDecision::Deny)
    }

    /// Live text-delta hook (streaming slice): the controller forwards each
    /// model text chunk in order, as the gateway produces it, before the
    /// round's `model_output` is recorded. Live-only — never journaled
    /// (Python `text_delta` precedent). Default no-op; hosts with a live
    /// client forward fire-and-forget. Synchronous because every delivery
    /// path is fire-and-forget (a dropped client must not stall the turn).
    fn on_text_delta(&self, _text: &str) {}

    /// Persist a turn record.
    async fn persist_turn(&self, _turn: &TurnRecord) -> Result<(), PersistError> {
        Ok(())
    }

    /// Trigger context compaction.
    async fn trigger_compaction(&self) -> Result<(), CompactionError> {
        Err(CompactionError::NotSupported)
    }

    /// Run lifecycle hooks.
    async fn run_hooks(&self, _event: &HookEvent) -> Vec<HookResult> {
        Vec::new()
    }

    /// Retrieve stored credentials.
    fn auth_credentials(&self) -> Result<Credentials, AuthError> {
        Err(AuthError::NotFound("no credentials configured".into()))
    }

    /// List MCP-provided tools.
    fn mcp_tools(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}
