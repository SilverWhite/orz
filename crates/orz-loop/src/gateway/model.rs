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
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
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
}

impl ModelResponse {
    pub fn text_response(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
        }
    }

    pub fn tool_calls_response(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: None,
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
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
}

/// The model gateway contract. `generate` takes the full request and returns
/// a structured response; streaming is assembled inside the implementation.
#[async_trait]
pub trait ModelGateway: Send + Sync {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError>;
}
