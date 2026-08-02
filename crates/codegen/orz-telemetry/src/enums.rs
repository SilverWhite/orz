#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PermissionMode { Default, AcceptEdits, BypassPermissions, Plan }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrCreationSource { Local, Remote }
