//! ModelGateway — sends prompts to LLMs and returns structured responses.
//!
//! IP1: thinking:disabled enforced at the request level — `ModelRequest` has
//! no thinking option field; providers must not send one.
//!
//! - `model` — gateway trait + request/response types
//! - `fake` — deterministic scripted provider (offline test main path)
//! - `transport` — real DeepSeek (OpenAI-compatible) transport
//!
//! Phase 2 decision (2026-08-04): FakeProvider is the primary path for the
//! Phase 2 acceptance chain; the real transport is an in-slice spike with a
//! reqwest + hand-written SSE fallback. Transport interface must stay
//! identical under either backend.

pub mod credentials;
pub mod fake;
pub mod model;
/// Live probe (FIX_PLAN 2026-08-06 item ①) — raw-JSON measurement of the
/// thinking-enabled surface; test-only, gated on ORZ_TEST_LIVE=1.
#[cfg(test)]
pub mod probe_thinking_max;
pub mod transport;

pub use fake::{FakeProvider, ScriptedResponse};
pub use model::{
    FinishReason, GatewayError, Message, ModelConfig, ModelGateway, ModelRequest, ModelResponse,
    Role, ToolCall,
};
