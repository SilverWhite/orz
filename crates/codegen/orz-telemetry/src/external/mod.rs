pub mod config;
pub mod emit;
pub mod providers;
pub mod redact;
pub mod schema;
pub mod truncate;
pub struct ExternalOtelConfig;
impl ExternalOtelConfig {
    pub fn resolve_with(_info: &config::ExternalClientInfo, _file: Option<&ExternalOtelFileConfig>) -> Self { Self }
}
pub struct ExternalOtelFileConfig;
pub struct ExternalOtelRemotePolicy;
pub fn is_active() -> bool { false }
pub fn is_settings_gate_open() -> bool { false }
pub fn suppress_external_otel_until_settings(_snapshot: &crate::orz_auth_stub::CredentialSnapshot) {}
pub fn mark_external_otel_settings_resolved() {}
pub fn apply_remote_policy(_policy: ExternalOtelRemotePolicy) {}
