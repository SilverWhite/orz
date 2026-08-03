//! Mixpanel stub - analytics disabled in orz.
//! All methods are no-ops that return Ok(()).

use std::collections::HashMap;

/// Mixpanel client (stub - analytics disabled).
#[derive(Clone)]
pub struct Mixpanel {
    token: String,
    client: reqwest::Client,
}

/// Error type for Mixpanel operations.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
}

impl Mixpanel {
    pub fn new(token: impl Into<String>) -> Self {
        Self { token: token.into(), client: reqwest::Client::new() }
    }

    pub fn with_client(token: impl Into<String>, client: reqwest::Client) -> Self {
        Self { token: token.into(), client }
    }

    pub async fn track(
        &self,
        _event: &str,
        _properties: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<(), Error> {
        Ok(())
    }

    pub async fn engage(
        &self,
        _distinct_id: &str,
        _set: HashMap<String, serde_json::Value>,
    ) -> Result<(), Error> {
        Ok(())
    }
}
