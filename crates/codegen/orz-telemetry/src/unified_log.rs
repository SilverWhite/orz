use serde_json::Value;
use std::sync::OnceLock;

static VERSION: OnceLock<String> = OnceLock::new();

pub fn set_version(v: &str) {
    let _ = VERSION.set(v.to_string());
}

pub fn info(msg: &str, session_id: Option<&str>, data: Option<Value>) {
    if let Some(sid) = session_id {
        tracing::info!(session_id = %sid, "{msg}");
    } else {
        tracing::info!("{msg}");
    }
}

pub fn warn(msg: &str, session_id: Option<&str>, data: Option<Value>) {
    if let Some(sid) = session_id {
        tracing::warn!(session_id = %sid, "{msg}");
    } else {
        tracing::warn!("{msg}");
    }
}

pub fn error(msg: &str, session_id: Option<&str>, data: Option<Value>) {
    if let Some(sid) = session_id {
        tracing::error!(session_id = %sid, "{msg}");
    } else {
        tracing::error!("{msg}");
    }
}

pub fn debug(msg: &str, session_id: Option<&str>, data: Option<Value>) {
    tracing::debug!("{msg}");
}

pub fn ingest_client_entries(entries: Vec<ClientLogEntry>) {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClientLogEntry {
    pub level: LogLevel,
    pub message: String,
    pub source: LogSource,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LogLevel { Info, Warn, Error, Debug }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum LogSource { Client, Server }

pub struct LogNotificationParams;

pub const LOG_METHOD: &str = "unified_log";
