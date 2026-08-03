//! orz-http — minimal HTTP client stub.
//! Provides the shared reqwest client wrapper still referenced by kept providers.

use std::sync::Arc;

/// Shared HTTP client.
pub struct HttpClient {
    pub client: reqwest::Client,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared HTTP client reference type.
pub type SharedClient = Arc<HttpClient>;
