use std::sync::OnceLock;
static AGENT_ID: OnceLock<String> = OnceLock::new();
pub fn agent_id() -> Option<String> { None }
pub fn agent_instance_id() -> String { "local".to_string() }
