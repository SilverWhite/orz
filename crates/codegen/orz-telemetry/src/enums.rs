#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionMode { Default, AcceptEdits, BypassPermissions, Plan, AlwaysApprove, Auto, Ask }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrCreationSource { Local, Remote }
#[derive(Debug, Clone, Copy)] pub enum McpInitStrategy { Lazy, Eager }
