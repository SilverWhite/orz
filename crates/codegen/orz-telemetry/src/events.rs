//! Telemetry event struct stubs — all no-ops.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolCalled {
    pub tool_name: String,
    pub mcp_server_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerFailed {
    pub error_type: McpErrorType,
    pub error_message: String,
    pub mcp_server_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum McpErrorType {
    SpawnFailed,
    HandshakeFailed,
    Timeout,
    Other,
}
