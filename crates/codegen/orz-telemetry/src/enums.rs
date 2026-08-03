//! Telemetry enum stubs.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum McpInitStrategy {
    Lazy,
    Eager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionMode {
    AlwaysApprove,
    Auto,
    Ask,
}
