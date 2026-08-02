#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TelemetryConfig { pub mode: TelemetryMode }
impl Default for TelemetryConfig { fn default() -> Self { Self { mode: TelemetryMode::Disabled } } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TelemetryMode { Enabled, Disabled, Debug }
impl TelemetryMode {
    pub fn parse(s: &str) -> Self { Self::Disabled }
    pub fn is_disabled(&self) -> bool { true }
}
impl From<bool> for TelemetryMode { fn from(_: bool) -> Self { Self::Disabled } }

pub fn env_telemetry_mode() -> TelemetryMode { TelemetryMode::Disabled }
pub fn deployment_id_from_key(_key: &str) -> Option<String> { None }
