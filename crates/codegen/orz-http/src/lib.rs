//! HTTP client stub for kept Grok providers.

use reqwest_middleware::ClientWithMiddleware;

/// Returns a shared HTTP client.
pub fn shared_client() -> ClientWithMiddleware {
    reqwest_middleware::ClientBuilder::new(reqwest::Client::new()).build()
}

/// Wraps a client with auth retry middleware (no-op in stub).
pub fn with_auth_retry<T>(
    client: ClientWithMiddleware,
    _credentials: T,
) -> ClientWithMiddleware {
    client
}
