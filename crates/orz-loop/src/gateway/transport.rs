//! Real DeepSeek transport — OpenAI-compatible chat completions.
//!
//! Phase 2 spike plan (2026-08-04): day-1 compile probe against the
//! `async-openai` 0.33 fork pinned in the workspace (`[patch.crates-io]`);
//! if the fork fails to compile, fall back to `reqwest 0.12` + hand-written
//! SSE parsing. `DeepSeekTransport` is the only type affected — the
//! `ModelGateway` interface stays identical under either backend.
//!
//! Live tests are gated behind `ORZ_TEST_LIVE=1` so the offline test suite
//! stays deterministic (FakeProvider is the acceptance path).

use async_trait::async_trait;
use serde_json::Value;

use super::model::{GatewayError, ModelConfig, ModelGateway, ModelRequest, ModelResponse};

/// Default DeepSeek API base (OpenAI-compatible).
pub const DEFAULT_DEEPSEEK_API_BASE: &str = "https://api.deepseek.com";

/// Real model transport over the OpenAI-compatible chat completions API.
#[derive(Debug, Clone)]
pub struct DeepSeekTransport {
    pub config: ModelConfig,
}

impl DeepSeekTransport {
    pub fn new(config: ModelConfig) -> Self {
        Self { config }
    }

    /// Convenience constructor for DeepSeek V4 models.
    pub fn deepseek_v4(api_key: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            config: ModelConfig {
                provider: "deepseek".to_string(),
                model_id: model_id.into(),
                api_base: DEFAULT_DEEPSEEK_API_BASE.to_string(),
                api_key: api_key.into(),
                max_tokens: 4096,
            },
        }
    }

    /// Model request → wire payload (OpenAI chat completions shape).
    ///
    /// IP1: no `thinking`/reasoning option is ever emitted.
    pub fn build_payload(&self, request: &ModelRequest) -> Value {
        let mut messages: Vec<Value> = Vec::new();
        if !request.system.is_empty() {
            messages.push(serde_json::json!({
                "role": "system",
                "content": request.system,
            }));
        }
        for message in &request.messages {
            messages.push(serde_json::json!({
                "role": match message.role {
                    crate::gateway::model::Role::System => "system",
                    crate::gateway::model::Role::User => "user",
                    crate::gateway::model::Role::Assistant => "assistant",
                    crate::gateway::model::Role::Tool => "tool",
                },
                "content": message.content,
            }));
        }
        let mut payload = serde_json::json!({
            "model": self.config.model_id,
            "messages": messages,
            "max_tokens": self.config.max_tokens.min(request.max_tokens),
            "stream": false,
        });
        if !request.tools.is_empty() {
            payload["tools"] = serde_json::json!(
                request
                    .tools
                    .iter()
                    .map(|t| serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    }))
                    .collect::<Vec<_>>()
            );
        }
        payload
    }

    /// Parse a chat completions response into `ModelResponse`.
    pub fn parse_response(&self, body: &Value) -> Result<ModelResponse, GatewayError> {
        let choice = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|c| c.first())
            .ok_or_else(|| GatewayError::Parse("missing choices".to_string()))?;
        let message = choice
            .get("message")
            .ok_or_else(|| GatewayError::Parse("missing message".to_string()))?;

        let text = message
            .get("content")
            .and_then(|c| c.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let mut tool_calls = Vec::new();
        if let Some(calls) = message.get("tool_calls").and_then(|c| c.as_array()) {
            for call in calls {
                let function = call
                    .get("function")
                    .ok_or_else(|| GatewayError::Parse("missing function".to_string()))?;
                tool_calls.push(crate::gateway::model::ToolCall {
                    name: function
                        .get("name")
                        .and_then(|n| n.as_str())
                        .ok_or_else(|| GatewayError::Parse("missing tool name".to_string()))?
                        .to_string(),
                    arguments: function
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(Value::Null),
                    call_id: call
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or_default()
                        .to_string(),
                });
            }
        }

        let finish_reason = match choice
            .get("finish_reason")
            .and_then(|f| f.as_str())
        {
            Some("tool_calls") => crate::gateway::model::FinishReason::ToolCalls,
            Some("length") => crate::gateway::model::FinishReason::Length,
            _ => crate::gateway::model::FinishReason::Stop,
        };

        Ok(ModelResponse {
            text,
            tool_calls,
            finish_reason,
        })
    }
}

#[async_trait]
impl ModelGateway for DeepSeekTransport {
    async fn generate(&self, _request: ModelRequest) -> Result<ModelResponse, GatewayError> {
        // Phase 2 spike: async-openai fork compile probe, or reqwest + SSE
        // fallback. Live network tests are gated behind ORZ_TEST_LIVE=1.
        if std::env::var("ORZ_TEST_LIVE").as_deref() == Ok("1") {
            Err(GatewayError::Transport(
                "live transport not yet wired — spike pending".to_string(),
            ))
        } else {
            Err(GatewayError::Transport(
                "live transport disabled (set ORZ_TEST_LIVE=1 after spike)".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Message, Role};

    #[test]
    fn build_payload_omits_thinking_ip1() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let payload = t.build_payload(&ModelRequest {
            system: "sys".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: "hi".to_string(),
            }],
            tools: Vec::new(),
            max_tokens: 512,
        });
        let json = payload.to_string();
        // IP1: the payload surface must never contain a thinking option.
        assert!(!json.contains("thinking"));
        assert_eq!(payload["model"], "deepseek-v4-flash");
        assert_eq!(payload["stream"], false);
    }

    #[test]
    fn build_payload_includes_tool_defs() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let payload = t.build_payload(&ModelRequest {
            system: String::new(),
            messages: Vec::new(),
            tools: vec![crate::host::ToolDef {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            }],
            max_tokens: 512,
        });
        assert_eq!(payload["tools"][0]["function"]["name"], "read_file");
    }

    #[test]
    fn parse_response_extracts_text_and_tool_calls() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "content": "done",
                    "tool_calls": [{
                        "id": "call-1",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"a.txt\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let r = t.parse_response(&body).unwrap();
        assert_eq!(r.text.as_deref(), Some("done"));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "read_file");
        assert_eq!(
            r.tool_calls[0].arguments,
            serde_json::json!({"path": "a.txt"})
        );
        assert_eq!(r.finish_reason, crate::gateway::model::FinishReason::ToolCalls);
    }

    #[test]
    fn parse_response_missing_choices_errors() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let err = t.parse_response(&serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("choices"), "{err}");
    }

    #[tokio::test]
    #[ignore = "live transport spike — set ORZ_TEST_LIVE=1 after wiring"]
    async fn live_chat_completion_roundtrip() {
        let key = std::env::var("ORZ_TEST_API_KEY").unwrap_or_default();
        if key.is_empty() {
            return;
        }
        let t = DeepSeekTransport::deepseek_v4(key, "deepseek-v4-flash");
        let r = t
            .generate(ModelRequest {
                system: "You are a test assistant. Reply with exactly OK".to_string(),
                messages: vec![Message {
                    role: Role::User,
                    content: "ping".to_string(),
                }],
                tools: Vec::new(),
                max_tokens: 32,
            })
            .await;
        assert!(r.is_ok(), "{r:?}");
    }
}
