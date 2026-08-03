//! ModelGateway — sends prompts to LLMs and streams responses back.
//!
//! IP1: thinking:disabled enforced at the request level.
//! Uses SSE streaming transport with retry.
//!
//! Phase 1: skeleton only. Full implementation in Step 3.

pub mod model;
pub mod transport;
