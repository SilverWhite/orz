//! Model gateway contract — request/response types + trait.
//!
//! IP1 (thinking:disabled): `ModelRequest` intentionally has no thinking
//! option — the request surface cannot express it, so no provider backend can
//! enable chain-of-thought by accident.

use async_trait::async_trait;
use serde_json::Value;

use crate::host::ToolDef;

/// Configuration for a single model provider endpoint.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model_id: String,
    pub api_base: String,
    pub api_key: String,
    pub max_tokens: u32,
}

/// Conversation role for a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// One conversation message.
///
/// `tool_call_id` is only set on `Role::Tool` messages — the provider
/// protocol (OpenAI-compatible chat completions) requires each tool result
/// to reference the call it answers, so a transport must be able to emit it.
/// `tool_calls` is only set on `Role::Assistant` messages: the assistant's
/// call declarations must be replayed before their tool results, or the
/// provider rejects the round as an unmatched `tool_call_id`
/// (2026-08-06 design review D2-1).
/// `reasoning_content` is only set on `Role::Assistant` messages — DeepSeek
/// returns it on every completion and expects it replayed on multi-turn
/// conversations (alpha-test 2026-08-06: live probe showed 318 chars even
/// without a thinking option); the transport echoes it back on the
/// declaration message.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub reasoning_content: Option<String>,
}

/// A tool call requested by the model.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
    pub call_id: String,
}

/// Why generation stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
}

/// Request sent to the model. No thinking field — see module doc (IP1).
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: u32,
}

/// Structured model response.
#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    /// Reasoning content (DeepSeek) — echoed on the assistant message the
    /// controller replays before tool results, so the next request carries
    /// the full assistant turn (alpha-test 2026-08-06 closure).
    pub reasoning_content: Option<String>,
}

impl ModelResponse {
    pub fn text_response(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            reasoning_content: None,
        }
    }

    pub fn tool_calls_response(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: None,
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
        }
    }
}

/// Error from the model gateway.
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("model error: {0}")]
    Model(String),
    #[error("response parse error: {0}")]
    Parse(String),
    /// Cooperative cancellation observed mid-stream (Phase 3 slice #11,
    /// P3-7 closure). The loop maps this to `AgentLoopError::Cancelled` so
    /// the run ends with `run_cancelled`, not a spurious failure.
    #[error("generation cancelled")]
    Cancelled,
}

/// The model gateway contract. `generate` takes the full request and returns
/// a structured response; `generate_stream` additionally delivers the text
/// to `on_chunk` as ordered chunks as they are produced (live `text_delta`
/// delivery to the TUI — the chunks are never journaled, Python precedent).
/// Transports that stream over the wire override `generate_stream`; the
/// default buffers the whole response into a single chunk via `generate`,
/// so non-streaming backends stay untouched. Concatenating the chunks must
/// reproduce `response.text` exactly.
#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError>;

    /// `cancel` is a cooperative cancellation check (Phase 3 slice #11):
    /// streaming transports poll it between wire chunks and bail with
    /// `GatewayError::Cancelled` when set. Buffered backends may ignore it —
    /// the loop re-checks after the model round either way.
    async fn generate_stream(
        &self,
        request: ModelRequest,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        let _ = cancel;
        let response = self.generate(request).await?;
        // Clone so the closure's borrow cannot outlive the response move.
        if let Some(t) = response.text.clone().filter(|t| !t.is_empty()) {
            on_chunk(&t);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    #[async_trait]
    impl ModelGateway for TestProvider {
        async fn generate(&self, _request: ModelRequest) -> Result<ModelResponse, GatewayError> {
            Ok(ModelResponse::text_response("buffered"))
        }
    }

    #[tokio::test]
    async fn default_generate_stream_buffers_single_chunk() {
        // Guards the "default buffers via generate" contract for future
        // transports that don't override generate_stream.
        let provider = TestProvider;
        let mut chunks: Vec<String> = Vec::new();
        let response = provider
            .generate_stream(
                ModelRequest {
                    system: String::new(),
                    messages: Vec::new(),
                    tools: Vec::new(),
                    max_tokens: 0,
                },
                None,
                &mut |c| chunks.push(c.to_string()),
            )
            .await
            .unwrap();
        assert_eq!(chunks, vec!["buffered"]);
        assert_eq!(response.text.as_deref(), Some("buffered"));
    }
}
