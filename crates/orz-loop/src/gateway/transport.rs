//! SSE streaming transport with retry.
//!
//! Moved from orz-sampler pattern. Handles HTTP connection, SSE parsing,
//! exponential backoff retry, and streaming response assembly.
//!
//! Phase 1: skeleton only. Full implementation in Step 3.

/// Placeholder for the streaming transport.
pub struct SseTransport;

impl SseTransport {
    pub fn new() -> Self {
        SseTransport
    }
}

impl Default for SseTransport {
    fn default() -> Self {
        Self::new()
    }
}
