pub const ENV_MASTER_SWITCH: &str = "GROK_OTEL_ENABLED";
#[derive(Debug, Clone, Default)]
pub struct ExternalClientInfo {
    pub service_version: String,
    pub client_version: String,
    pub app_entrypoint: String,
}
