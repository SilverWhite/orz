//! LoopHost trait — the interface orz-host must implement.
//!
//! Defined in orz-loop per Codex discipline: orz-loop owns the agent loop contract;
//! orz-host bridges Grok providers to fulfill it.
//!
//! See: INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §3.4

use async_trait::async_trait;
use orz_assurance::journal::JournalRecorder;
use serde_json::Value;

/// Risk classification for permission requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskClass {
    ReadOnly,
    LocalMutation,
    NetworkCall,
    SandboxEscape,
}

/// Result returned by a tool invocation.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub output: String,
    pub exit_code: Option<i32>,
}

/// Lightweight error from tool execution.
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("tool execution failed: {0}")]
    ExecutionFailed(String),
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

    /// Execute a tool call.
    async fn call_tool(
        &self,
        _name: &str,
        _args: Value,
        _call_id: &str,
    ) -> Result<ToolResult, ToolError> {
        Err(ToolError::NotFound("not implemented".into()))
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
