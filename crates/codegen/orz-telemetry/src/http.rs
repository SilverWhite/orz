//! Origin/client identification stub.

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OriginClientInfo {
    pub product: String,
    pub version: Option<String>,
}

/// Stub — always returns None.
pub fn origin_client_info_from_env() -> Option<OriginClientInfo> {
    None
}
