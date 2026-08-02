// No-op event structs — all analytics removed in Phase 2.
use serde::{Deserialize, Serialize};

macro_rules! event_struct {
    ($name:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct $name;
    };
    ($name:ident, $($field:ident : $ty:ty),*) => {
        #[derive(Debug, Clone, Serialize, Deserialize, Default)]
        pub struct $name { $(pub $field: $ty),* }
    };
}

event_struct!(Login);
event_struct!(Logout);
event_struct!(SessionNew);
event_struct!(SessionStarted, session_id: String);
event_struct!(SessionEnded, session_id: String);
event_struct!(ModelSwitched, model_id: String, reason: String);
event_struct!(ToolCallStarted);
event_struct!(ToolCallCompleted, outcome: String);
event_struct!(PromptSent);
event_struct!(ResponseReceived);
event_struct!(CompactionStarted);
event_struct!(CompactionCompleted);
event_struct!(MemoryDreamStarted);
event_struct!(MemoryDreamCompleted);
event_struct!(SubagentSpawned);
event_struct!(SubagentCompleted);
event_struct!(ErrorOccurred);
event_struct!(PermissionRequested);
event_struct!(PermissionGranted);
event_struct!(PermissionDenied);
event_struct!(TraceUploadStarted);
event_struct!(TraceUploadCompleted);
event_struct!(TraceUploadSkipped);
event_struct!(SessionHarness);
event_struct!(MemoryReindex);
event_struct!(MemorySessionInit);
event_struct!(BackgroundLoopStarted);
event_struct!(BackgroundLoopCompleted);
event_struct!(SearchIndexUpdated);
event_struct!(WorktreeCreated);
event_struct!(WorktreeDeleted);
event_struct!(FileSystemEvent);
event_struct!(NetworkRequest);
event_struct!(NetworkResponse);
event_struct!(MCPToolCalled);
event_struct!(PluginLoaded);
event_struct!(HookExecuted);
event_struct!(ConfigReloaded);
event_struct!(AuthRefreshed);
event_struct!(RateLimitHit);
event_struct!(RetryAttempt);
event_struct!(CheckpointCreated);
event_struct!(CheckpointRestored);
event_struct!(UserFeedbackSubmitted);
event_struct!(FeatureFlagEvaluated);
event_struct!(SandboxProfileApplied);
event_struct!(ProcessTreeExited);

pub trait TelemetryEvent {
    fn event_name(&self) -> &'static str;
}
pub struct McpServerFailed { pub server_name: String, pub error_type: McpErrorType, pub duration_ms: u64, pub timeout_sec: u64 }
pub struct McpToolCalled { pub server_name: String, pub tool_name: String, pub qualified_name: String, pub success: bool, pub duration_ms: u64 }
#[derive(Debug, Clone)] pub enum McpErrorType { SpawnFailed, Timeout, ConnectionLost, ProtocolError, Unknown }
impl TelemetryEvent for McpServerFailed { fn event_name(&self) -> &'static str { "mcp_server_failed" } }
impl TelemetryEvent for McpToolCalled { fn event_name(&self) -> &'static str { "mcp_tool_called" } }
