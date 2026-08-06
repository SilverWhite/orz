//! Real DeepSeek transport — OpenAI-compatible chat completions.
//!
//! Phase 3 slice #11 (2026-08-06): the stub is wired through the
//! `async-openai` 0.33 fork pinned in `[patch.crates-io]` — the same fork the
//! inherited `orz-tools` / `orz-sampling-types` crates compile against, so
//! the day-1 compile probe from the Phase 2 spike plan is satisfied by
//! construction. SSE framing and `[DONE]` termination are handled inside
//! the fork's `create_stream`; HTTP retries (429/5xx backoff) apply to the
//! non-streaming `create` path only — the stream path does not retry
//! (recorded D3-2, 2026-08-06 design review).
//!
//! IP1 (thinking:disabled): the typed request built here never carries a
//! thinking/reasoning option — `ModelRequest` cannot express one, and the
//! builder surface we touch sets no such field. The `build_request` test
//! locks the serialized surface against the word `thinking`.
//!
//! Live tests are gated behind `ORZ_TEST_LIVE=1` so the offline test suite
//! stays deterministic (FakeProvider is the acceptance path).

use async_openai::{
    Client,
    config::OpenAIConfig,
    error::{ApiError, OpenAIError},
    types::chat::{
        ChatCompletionMessageToolCallChunk, ChatCompletionMessageToolCalls,
        ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
        ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
        ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
        ChatCompletionRequestUserMessageContent, ChatCompletionStreamResponseDelta,
        ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequest,
        CreateChatCompletionResponse, CreateChatCompletionStreamResponse, FinishReason,
        FunctionCallStream, FunctionObject,
    },
};
use async_trait::async_trait;
use futures::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use super::model::{
    FinishReason as OurFinishReason, GatewayError, ModelConfig, ModelGateway, ModelRequest,
    ModelResponse, ToolCall,
};

/// Default DeepSeek API base (OpenAI-compatible).
pub const DEFAULT_DEEPSEEK_API_BASE: &str = "https://api.deepseek.com";

/// Main-agent model id (single source of truth — the live test and the
/// production `--real` gateway share it, so a model change cannot drift
/// between the two).
pub const MAIN_AGENT_MODEL: &str = "deepseek-v4-flash";

/// Build the production real-model gateway from the ADR-0006 credential
/// registry (main-agent target `orz-deepseek/agent`).
///
/// Alpha-test ruling (2026-08-06): the `--real` CLI flag (orz / orz-codex)
/// is the only production consumer; the live test reads the credential
/// directly. Fail-closed — any credential failure is returned, never a
/// silent FakeProvider fallback; the binaries surface the error and exit.
pub fn real_gateway_from_credentials()
-> Result<Arc<dyn ModelGateway>, crate::gateway::credentials::CredentialError> {
    let key = crate::gateway::credentials::read_agent_api_key()?;
    Ok(Arc::new(DeepSeekTransport::deepseek_v4(
        key,
        MAIN_AGENT_MODEL,
    )))
}

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

    fn client(&self) -> Client<OpenAIConfig> {
        Client::with_config(
            OpenAIConfig::new()
                .with_api_key(self.config.api_key.clone())
                .with_api_base(self.config.api_base.clone()),
        )
    }

    /// Model request → typed chat completion request.
    ///
    /// IP1: no thinking/reasoning option is ever set — the builder surface
    /// used here has none, and `ModelRequest` cannot express one.
    /// `stream` is left unset: `create` / `create_stream` validate and
    /// set it themselves.
    ///
    /// `max_tokens` is deprecated in the fork (OpenAI moved to
    /// `max_completion_tokens`), but DeepSeek's OpenAI-compatible surface
    /// still accepts `max_tokens` — the field is the compatible knob.
    #[allow(deprecated)]
    pub fn build_request(&self, request: &ModelRequest) -> CreateChatCompletionRequest {
        // The system prompt (built by the controller — IP2a tool-availability
        // block, orientation context, etc.) must lead the conversation;
        // dropping it would silently strip the assurance context from every
        // real turn (2026-08-06 conformance review D1-1).
        let mut messages = Vec::with_capacity(request.messages.len() + 1);
        if !request.system.is_empty() {
            messages.push(ChatCompletionRequestMessage::System(
                ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(
                        request.system.clone(),
                    ),
                    name: None,
                },
            ));
        }
        messages.extend(request.messages.iter().map(map_message));
        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(
                request
                    .tools
                    .iter()
                    .map(|t| {
                        ChatCompletionTools::Function(ChatCompletionTool {
                            function: FunctionObject {
                                name: t.name.clone(),
                                description: Some(t.description.clone()),
                                parameters: Some(t.parameters.clone()),
                                strict: None,
                            },
                        })
                    })
                    .collect::<Vec<_>>(),
            )
        };
        CreateChatCompletionRequest {
            model: self.config.model_id.clone(),
            messages,
            tools,
            max_tokens: Some(self.config.max_tokens.min(request.max_tokens)),
            ..Default::default()
        }
    }

    /// Non-streamed response → `ModelResponse`.
    pub fn from_response(
        &self,
        body: &CreateChatCompletionResponse,
    ) -> Result<ModelResponse, GatewayError> {
        let choice = body
            .choices
            .first()
            .ok_or_else(|| GatewayError::Parse("missing choices".to_string()))?;
        let message = &choice.message;

        let text = message
            .content
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let mut tool_calls = Vec::new();
        if let Some(calls) = &message.tool_calls {
            for call in calls {
                let (name, arguments, call_id) = match call {
                    ChatCompletionMessageToolCalls::Function(f) => (
                        Some(f.function.name.clone()),
                        Some(f.function.arguments.clone()),
                        f.id.clone(),
                    ),
                    // Custom tools are an OpenAI/xAI surface DeepSeek does not
                    // emit; mapping them anyway keeps the parse total (P3).
                    ChatCompletionMessageToolCalls::Custom(c) => (
                        Some(c.custom_tool.name.clone()),
                        Some(c.custom_tool.input.clone()),
                        c.id.clone(),
                    ),
                };
                tool_calls.push(ToolCall {
                    name: name
                        .ok_or_else(|| GatewayError::Parse("missing tool name".to_string()))?,
                    arguments: arguments
                        .as_deref()
                        .and_then(|a| serde_json::from_str(a).ok())
                        .unwrap_or(Value::Null),
                    call_id,
                });
            }
        }

        Ok(ModelResponse {
            text,
            tool_calls,
            finish_reason: map_finish_reason(choice.finish_reason),
            // DeepSeek returns reasoning_content on every completion (even
            // without a thinking option — live probe 2026-08-06); it is
            // preserved so the controller can replay it on the next request.
            reasoning_content: message.reasoning_content.clone(),
        })
    }

    fn map_error(&self, e: OpenAIError) -> GatewayError {
        match e {
            OpenAIError::Reqwest(e) => GatewayError::Transport(e.to_string()),
            OpenAIError::ApiError(api) => GatewayError::Model(format_api_error(&api)),
            OpenAIError::JSONDeserialize(e, raw) => GatewayError::Parse(format!("{e}: {raw}")),
            OpenAIError::StreamError(e) => GatewayError::Transport(e.to_string()),
            OpenAIError::InvalidArgument(msg) => GatewayError::Model(msg),
            OpenAIError::FileSaveError(msg) => GatewayError::Transport(msg),
            OpenAIError::FileReadError(msg) => GatewayError::Transport(msg),
        }
    }
}

fn format_api_error(api: &ApiError) -> String {
    match (&api.code, &api.param) {
        (Some(code), Some(param)) => format!("{} (code={code}, param={param})", api.message),
        (Some(code), None) => format!("{} (code={code})", api.message),
        (None, Some(param)) => format!("{} (param={param})", api.message),
        (None, None) => api.message.clone(),
    }
}

/// Map a `Message` onto the provider's typed message enum.
///
/// Tool messages must carry the `tool_call_id` the model issued in its call
/// request — the provider rejects a tool result that answers nothing
/// (`controller` sets it from the journaled call id; the protocol requires it).
///
/// `function_call` is deprecated in the fork but still required by the
/// struct — we construct it as `None` (we never emit the legacy
/// function-call surface).
#[allow(deprecated)]
fn map_message(message: &super::model::Message) -> ChatCompletionRequestMessage {
    match message.role {
        super::model::Role::System => {
            ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(message.content.clone()),
                name: None,
            })
        }
        super::model::Role::User => {
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(message.content.clone()),
                name: None,
            })
        }
        super::model::Role::Assistant => {
            // An empty content (a pure tool-call declaration round) maps to
            // `None` — the provider requires content only when tool_calls
            // are absent (D2-1 replay).
            let content = if message.content.is_empty() {
                None
            } else {
                Some(ChatCompletionRequestAssistantMessageContent::Text(
                    message.content.clone(),
                ))
            };
            let tool_calls = if message.tool_calls.is_empty() {
                None
            } else {
                Some(
                    message
                        .tool_calls
                        .iter()
                        .map(|tc| {
                            ChatCompletionMessageToolCalls::Function(
                                async_openai::types::chat::ChatCompletionMessageToolCall {
                                    id: tc.call_id.clone(),
                                    function: async_openai::types::chat::FunctionCall {
                                        name: tc.name.clone(),
                                        arguments: serde_json::to_string(&tc.arguments)
                                            .unwrap_or_else(|_| "{}".to_string()),
                                    },
                                },
                            )
                        })
                        .collect(),
                )
            };
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content,
                // DeepSeek expects the assistant's reasoning content replayed
                // on multi-turn conversations (alpha-test 2026-08-06 closure;
                // fork field our-forks addition).
                reasoning_content: message.reasoning_content.clone(),
                name: None,
                refusal: None,
                audio: None,
                tool_calls,
                function_call: None,
            })
        }
        super::model::Role::Tool => {
            ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                content: ChatCompletionRequestToolMessageContent::Text(message.content.clone()),
                // A tool result without an id cannot be matched to its call;
                // an empty string is the fail-closed fallback (the provider
                // rejects the round rather than guessing).
                tool_call_id: message.tool_call_id.clone().unwrap_or_default(),
            })
        }
    }
}

fn map_finish_reason(reason: Option<FinishReason>) -> OurFinishReason {
    match reason {
        Some(FinishReason::ToolCalls) => OurFinishReason::ToolCalls,
        Some(FinishReason::Length) => OurFinishReason::Length,
        // ContentFilter / FunctionCall are OpenAI-specific surfaces DeepSeek
        // does not emit; collapsing them onto Stop keeps the journal enum
        // stable (recorded P3).
        _ => OurFinishReason::Stop,
    }
}

/// Accumulated stream state for one tool call (by chunk `index`).
#[derive(Default)]
struct StreamToolCall {
    name: String,
    arguments: String,
    call_id: String,
}

#[async_trait]
impl ModelGateway for DeepSeekTransport {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
        let client = self.client();
        let response = client
            .chat()
            .create(self.build_request(&request))
            .await
            .map_err(|e| self.map_error(e))?;
        self.from_response(&response)
    }

    async fn generate_stream(
        &self,
        request: ModelRequest,
        cancel: Option<&CancellationToken>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        // Pre-cancel check (2026-08-06 implementation review P2-2): an
        // already-cancelled run must not open a connection at all.
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(GatewayError::Cancelled);
        }
        let client = self.client();
        let mut stream = client
            .chat()
            .create_stream(self.build_request(&request))
            .await
            .map_err(|e| self.map_error(e))?;

        let mut text_parts: Vec<String> = Vec::new();
        // DeepSeek interleaves reasoning_content deltas with content deltas;
        // accumulated separately, then joined verbatim onto the response
        // (alpha-test 2026-08-06 closure).
        let mut reasoning_parts: Vec<String> = Vec::new();
        // Ordered by first-seen chunk index; the provider sends each call's
        // deltas in order, so Vec append + final sort keeps call order stable.
        let mut tool_calls: Vec<(u32, StreamToolCall)> = Vec::new();
        let mut finish_reason = OurFinishReason::Stop;
        // The provider always terminates a well-formed stream with a
        // finish_reason block; an EOF without one is a truncated stream
        // (proxy drop, deploy switch, overload) and must not be journaled
        // as a completed answer (2026-08-06 design review D1-1).
        let mut saw_finish_reason = false;

        loop {
            // Cooperative cancellation checkpoint (Phase 3 slice #11, P3-7;
            // 2026-08-06 design review D1-2): `select!` wakes the moment the
            // token fires — polling would leave `/stop` dead while the wire
            // is stalled (the fork's reqwest client has no read timeout) —
            // and returning drops the stream, whose spawned task sees the rx
            // end and closes the connection (the abort-by-drop path; the
            // provider may keep generating up to one frame — recorded as the
            // inherent boundary of cooperative cancellation).
            let item = match cancel {
                Some(c) => tokio::select! {
                    biased;
                    _ = c.cancelled() => return Err(GatewayError::Cancelled),
                    item = stream.next() => item,
                },
                None => stream.next().await,
            };
            let Some(item) = item else {
                break;
            };
            let chunk: CreateChatCompletionStreamResponse = item.map_err(|e| self.map_error(e))?;
            for choice in &chunk.choices {
                apply_delta(
                    &choice.delta,
                    &mut text_parts,
                    &mut reasoning_parts,
                    &mut tool_calls,
                    &mut |text| on_chunk(text),
                );
                if let Some(fr) = choice.finish_reason {
                    saw_finish_reason = true;
                    finish_reason = map_finish_reason(Some(fr));
                }
            }
        }

        // D1-1: a stream that ended without a finish_reason was truncated —
        // surfacing it as an error keeps the journal honest (no half-answer
        // recorded as a completed `stop`).
        if !saw_finish_reason {
            return Err(GatewayError::Transport(
                "stream ended without finish_reason".to_string(),
            ));
        }

        let text = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.concat())
        };

        tool_calls.sort_by_key(|(index, _)| *index);
        let tool_calls = tool_calls
            .into_iter()
            .map(|(_, t)| ToolCall {
                name: t.name,
                arguments: serde_json::from_str(&t.arguments).unwrap_or(Value::Null),
                call_id: t.call_id,
            })
            .collect::<Vec<_>>();

        let reasoning_content = if reasoning_parts.is_empty() {
            None
        } else {
            Some(reasoning_parts.concat())
        };

        Ok(ModelResponse {
            text,
            tool_calls,
            finish_reason,
            reasoning_content,
        })
    }
}

fn apply_delta(
    delta: &ChatCompletionStreamResponseDelta,
    text_parts: &mut Vec<String>,
    reasoning_parts: &mut Vec<String>,
    tool_calls: &mut Vec<(u32, StreamToolCall)>,
    on_chunk: &mut dyn FnMut(&str),
) {
    if let Some(content) = &delta.content
        && !content.is_empty()
    {
        text_parts.push(content.clone());
        on_chunk(content);
    }
    if let Some(reasoning) = &delta.reasoning_content
        && !reasoning.is_empty()
    {
        // Reasoning deltas are joined verbatim; they are never delivered as
        // live text deltas (TUI shows answers, not chains of thought).
        reasoning_parts.push(reasoning.clone());
    }
    if let Some(chunks) = &delta.tool_calls {
        for tc in chunks {
            apply_tool_call_chunk(tc, tool_calls);
        }
    }
}

fn apply_tool_call_chunk(
    chunk: &ChatCompletionMessageToolCallChunk,
    tool_calls: &mut Vec<(u32, StreamToolCall)>,
) {
    let entry = match tool_calls
        .iter_mut()
        .find(|(index, _)| *index == chunk.index)
    {
        Some(e) => e,
        None => {
            tool_calls.push((chunk.index, StreamToolCall::default()));
            tool_calls.last_mut().unwrap()
        }
    };
    if let Some(id) = &chunk.id {
        entry.1.call_id.clone_from(id);
    }
    if let Some(function) = &chunk.function {
        apply_function_chunk(function, &mut entry.1);
    }
}

fn apply_function_chunk(function: &FunctionCallStream, state: &mut StreamToolCall) {
    if let Some(name) = &function.name {
        state.name.push_str(name);
    }
    if let Some(arguments) = &function.arguments {
        state.arguments.push_str(arguments);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Message, Role};
    use std::sync::Arc;

    fn request() -> ModelRequest {
        ModelRequest {
            system: "sys".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: "hi".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            }],
            tools: Vec::new(),
            max_tokens: 512,
        }
    }

    #[test]
    fn build_request_omits_thinking_ip1() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&request());
        let json = serde_json::to_value(&req).unwrap();
        // IP1: the serialized request surface must never contain a thinking
        // option nor any reasoning knob the fork exposes (reasoning_effort
        // stays None via `..Default::default()` — asserted so a future
        // mistake cannot silently enable it). `reasoning_content` is a
        // CONTENT field (replay of what the model already produced), not a
        // knob — it stays legal; request() above carries None and must
        // therefore be absent from the wire.
        let s = json.to_string();
        assert!(!s.contains("thinking"), "{s}");
        assert!(!s.contains("reasoning_effort"), "{s}");
        assert!(!s.contains("reasoning_content"), "{s}");
        assert_eq!(json["model"], "deepseek-v4-flash");
        assert!(!s.contains("\"stream\":"));
    }

    #[test]
    fn build_request_replays_assistant_reasoning_content() {
        // DeepSeek returns reasoning_content on every completion and expects
        // it replayed with the assistant turn (alpha-test 2026-08-06 closure);
        // the declaration message carries it verbatim on the next request.
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&ModelRequest {
            system: String::new(),
            messages: vec![Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: Some("thinking-about-the-tool".to_string()),
            }],
            tools: Vec::new(),
            max_tokens: 512,
        });
        let json = serde_json::to_value(&req).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "assistant");
        assert_eq!(messages[0]["reasoning_content"], "thinking-about-the-tool");
    }

    #[test]
    fn build_request_prepends_system_message() {
        // The system prompt (IP2a tool-availability block etc.) must lead
        // the conversation — dropping it would strip the assurance context
        // (2026-08-06 conformance review D1-1).
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&request()); // request() has system "sys"
        let json = serde_json::to_value(&req).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "sys");
        assert_eq!(messages[1]["role"], "user");
    }

    #[test]
    fn build_request_maps_tool_message_with_call_id() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&ModelRequest {
            system: String::new(),
            messages: vec![
                Message {
                    role: Role::User,
                    content: "read it".to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                },
                Message {
                    role: Role::Tool,
                    content: "ok".to_string(),
                    tool_call_id: Some("call-9".to_string()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                },
            ],
            tools: Vec::new(),
            max_tokens: 512,
        });
        let json = serde_json::to_value(&req).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[1]["role"], "tool");
        assert_eq!(messages[1]["tool_call_id"], "call-9");
        assert_eq!(messages[1]["content"], "ok");
    }

    #[test]
    fn build_request_maps_assistant_tool_calls() {
        // The assistant's call declarations must be replayed before their
        // results — a tool message whose tool_call_id has no matching
        // declaration in the history is rejected by the provider (D2-1).
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&ModelRequest {
            system: String::new(),
            messages: vec![
                Message {
                    role: Role::User,
                    content: "list files".to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                },
                Message {
                    role: Role::Assistant,
                    content: String::new(), // pure declaration round
                    tool_call_id: None,
                    tool_calls: vec![ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({"path": "a.txt"}),
                        call_id: "call-1".to_string(),
                    }],
                    reasoning_content: None,
                },
                Message {
                    role: Role::Tool,
                    content: "ok".to_string(),
                    tool_call_id: Some("call-1".to_string()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                },
            ],
            tools: Vec::new(),
            max_tokens: 512,
        });
        let json = serde_json::to_value(&req).unwrap();
        let messages = json["messages"].as_array().unwrap();
        assert_eq!(messages[0]["role"], "user");
        let assistant = &messages[1];
        assert_eq!(assistant["role"], "assistant");
        assert!(assistant.get("content").is_none(), "empty content omitted");
        let calls = assistant["tool_calls"].as_array().unwrap();
        assert_eq!(calls[0]["id"], "call-1");
        assert_eq!(calls[0]["type"], "function");
        assert_eq!(calls[0]["function"]["name"], "read_file");
        assert_eq!(calls[0]["function"]["arguments"], r#"{"path":"a.txt"}"#);
        // The tool result pairs with the declaration.
        assert_eq!(messages[2]["role"], "tool");
        assert_eq!(messages[2]["tool_call_id"], "call-1");
    }

    #[test]
    fn build_request_includes_tool_defs() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let req = t.build_request(&ModelRequest {
            system: String::new(),
            messages: Vec::new(),
            tools: vec![crate::host::ToolDef {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            }],
            max_tokens: 512,
        });
        let json = serde_json::to_value(&req).unwrap();
        let tools = json["tools"].as_array().unwrap();
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "read_file");
        assert_eq!(tools[0]["function"]["description"], "Read a file");
    }

    #[test]
    fn from_response_extracts_text_and_tool_calls() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let body: CreateChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "x",
            "object": "chat.completion",
            "created": 0,
            "model": "deepseek-v4-flash",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "done",
                    "tool_calls": [{
                        "id": "call-1",
                        "type": "function",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"a.txt\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }))
        .unwrap();
        let r = t.from_response(&body).unwrap();
        assert_eq!(r.text.as_deref(), Some("done"));
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "read_file");
        assert_eq!(
            r.tool_calls[0].arguments,
            serde_json::json!({"path": "a.txt"})
        );
        assert_eq!(r.finish_reason, OurFinishReason::ToolCalls);
        // reasoning_content absent → None (absent-optional, not empty string).
        assert_eq!(r.reasoning_content, None);
    }

    #[test]
    fn from_response_extracts_reasoning_content() {
        // DeepSeek returns reasoning_content on every completion even without
        // a thinking option (live probe 2026-08-06 — 318 chars on a plain
        // prompt); the transport must preserve it for replay.
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let body: CreateChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "x",
            "object": "chat.completion",
            "created": 0,
            "model": "deepseek-v4-flash",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "done",
                    "reasoning_content": "need to read the file first"
                },
                "finish_reason": "stop"
            }]
        }))
        .unwrap();
        let r = t.from_response(&body).unwrap();
        assert_eq!(
            r.reasoning_content.as_deref(),
            Some("need to read the file first")
        );
        assert_eq!(r.text.as_deref(), Some("done"));
    }

    #[test]
    fn from_response_missing_choices_errors() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let body: CreateChatCompletionResponse = serde_json::from_value(serde_json::json!({
            "id": "x",
            "object": "chat.completion",
            "created": 0,
            "model": "m",
            "choices": []
        }))
        .unwrap();
        let err = t.from_response(&body).unwrap_err();
        assert!(err.to_string().contains("choices"), "{err}");
    }

    #[test]
    #[allow(deprecated)] // full struct construction: `function_call` is deprecated-but-required
    fn stream_aggregation_concatenates_chunks_and_tool_calls() {
        // Two text chunks + a two-chunk tool call, out of order by index
        // (chunk index 1 arrives before chunk index 0's continuation), plus
        // reasoning deltas interleaved with content (DeepSeek shape).
        let delta = |content: Option<&str>, reasoning: Option<&str>, tool_calls: Option<Value>| {
            ChatCompletionStreamResponseDelta {
                content: content.map(|s| s.to_string()),
                reasoning_content: reasoning.map(|s| s.to_string()),
                function_call: None,
                tool_calls: tool_calls.map(|v| serde_json::from_value(v).unwrap()),
                role: None,
                refusal: None,
            }
        };
        let mut text_parts = Vec::new();
        let mut reasoning_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<(u32, StreamToolCall)> = Vec::new();
        let mut chunks = Vec::new();
        let mut emit = |c: &str| chunks.push(c.to_string());

        apply_delta(
            &delta(Some("Hel"), Some("think-"), None),
            &mut text_parts,
            &mut reasoning_parts,
            &mut tool_calls,
            &mut emit,
        );
        apply_delta(
            &delta(
                None,
                None,
                Some(serde_json::json!([{"index": 1, "id": "call-2",
                    "function": {"name": "grep", "arguments": "{\"path\":"}}])),
            ),
            &mut text_parts,
            &mut reasoning_parts,
            &mut tool_calls,
            &mut emit,
        );
        apply_delta(
            &delta(
                Some("lo!"),
                Some("ing-about-the-tool"),
                Some(serde_json::json!([{"index": 0, "id": "call-1",
                    "function": {"name": "read_file", "arguments": "{\"path\":\"a.txt\"}"}}])),
            ),
            &mut text_parts,
            &mut reasoning_parts,
            &mut tool_calls,
            &mut emit,
        );
        apply_delta(
            &delta(
                None,
                None,
                Some(serde_json::json!([{"index": 1, "function": {"arguments": "\"b.txt\"}"}}])),
            ),
            &mut text_parts,
            &mut reasoning_parts,
            &mut tool_calls,
            &mut emit,
        );

        assert_eq!(text_parts, vec!["Hel", "lo!"]);
        assert_eq!(chunks, vec!["Hel", "lo!"]);
        // Reasoning deltas accumulate verbatim — never delivered as live text
        // deltas (TUI shows answers, not chains of thought).
        assert_eq!(reasoning_parts, vec!["think-", "ing-about-the-tool"]);
        tool_calls.sort_by_key(|(index, _)| *index);
        assert_eq!(tool_calls.len(), 2);
        assert_eq!(tool_calls[0].1.name, "read_file");
        assert_eq!(tool_calls[0].1.arguments, "{\"path\":\"a.txt\"}");
        assert_eq!(tool_calls[0].1.call_id, "call-1");
        assert_eq!(tool_calls[1].1.name, "grep");
        assert_eq!(tool_calls[1].1.arguments, "{\"path\":\"b.txt\"}");
        assert_eq!(tool_calls[1].1.call_id, "call-2");
    }

    // ── Offline E2E against a local mock of the OpenAI-compatible API ──────
    //
    // The real transport is exercised over an actual TCP connection (no
    // network): a scripted HTTP server plays the DeepSeek endpoint, so the
    // wire path — request serialization, SSE framing, [DONE] termination,
    // error mapping — is tested deterministically without an API key.

    /// A scripted mock response.
    struct MockResponse {
        status: u16,
        content_type: &'static str,
        /// Body sent in one write; for SSE the frames are written with
        /// `frame_delay` between them (streaming visibility / cancel tests).
        body: String,
        frame_delay: std::time::Duration,
    }

    impl MockResponse {
        fn json(status: u16, body: impl Into<String>) -> Self {
            Self {
                status,
                content_type: "application/json",
                body: body.into(),
                frame_delay: std::time::Duration::ZERO,
            }
        }

        fn sse(frames: Vec<&str>, frame_delay: std::time::Duration) -> Self {
            Self {
                status: 200,
                content_type: "text/event-stream",
                body: frames.join(""),
                frame_delay,
            }
        }
    }

    /// Serve one request (read headers + content-length body), answer with
    /// `handler`, then close. Returns the base URL to point the client at.
    async fn spawn_mock(
        handler: impl Fn(&str, &str) -> MockResponse + Send + Sync + 'static,
    ) -> String {
        let handler = Arc::new(handler);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let handler = handler.clone();
                tokio::spawn(async move {
                    let _ = handle_mock_conn(&mut socket, handler.as_ref()).await;
                });
            }
        });
        format!("http://127.0.0.1:{}/v1", addr.port())
    }

    async fn handle_mock_conn(
        socket: &mut tokio::net::TcpStream,
        handler: &(dyn Fn(&str, &str) -> MockResponse + Send + Sync + 'static),
    ) -> std::io::Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = Vec::new();
        let mut tmp = [0u8; 4096];
        // Read until the header terminator, then the content-length body.
        let header_end = loop {
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
            let n = socket.read(&mut tmp).await?;
            if n == 0 {
                return Ok(());
            }
            buf.extend_from_slice(&tmp[..n]);
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]).to_string();
        let content_length = headers
            .lines()
            .find_map(|l| {
                let (k, v) = l.split_once(':')?;
                k.trim()
                    .eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        while buf.len() < header_end + content_length {
            let n = socket.read(&mut tmp).await?;
            if n == 0 {
                return Ok(());
            }
            buf.extend_from_slice(&tmp[..n]);
        }
        let body =
            String::from_utf8_lossy(&buf[header_end..header_end + content_length]).to_string();
        let first_line = headers.lines().next().unwrap_or("").to_string();

        let response = handler(&first_line, &body);
        let head = format!(
            "HTTP/1.1 {} OK\r\ncontent-type: {}\r\nconnection: close\r\ncontent-length: {}\r\n\r\n",
            response.status,
            response.content_type,
            response.body.len()
        );
        socket.write_all(head.as_bytes()).await?;
        if response.frame_delay.is_zero() {
            socket.write_all(response.body.as_bytes()).await?;
        } else {
            // SSE: write frame by frame so a mid-stream cancel is observable.
            for frame in response.body.split("\n\n") {
                if frame.is_empty() {
                    continue;
                }
                socket.write_all(format!("{frame}\n\n").as_bytes()).await?;
                socket.flush().await?;
                tokio::time::sleep(response.frame_delay).await;
            }
        }
        Ok(())
    }

    /// A transport pointed at the mock server.
    fn mock_transport(base_url: &str) -> DeepSeekTransport {
        DeepSeekTransport {
            config: ModelConfig {
                provider: "deepseek".to_string(),
                model_id: "deepseek-v4-flash".to_string(),
                api_base: base_url.to_string(),
                api_key: "sk-test".to_string(),
                max_tokens: 4096,
            },
        }
    }

    #[tokio::test]
    async fn generate_real_http_roundtrip() {
        let base = spawn_mock(|_line, body| {
            assert!(body.contains("\"stream\":false") || !body.contains("\"stream\":true"));
            assert!(!body.contains("thinking"), "IP1 on the wire");
            // The system prompt leads the conversation on the wire (D1-1).
            assert!(body.contains("\"role\":\"system\""), "{body}");
            assert!(body.contains("\"content\":\"sys\""), "{body}");
            MockResponse::json(
                200,
                r#"{"id":"x","object":"chat.completion","created":0,"model":"deepseek-v4-flash","choices":[{"index":0,"message":{"role":"assistant","content":"你好"},"finish_reason":"stop"}]}"#,
            )
        })
        .await;
        let t = mock_transport(&base);
        let r = t.generate(request()).await.unwrap();
        assert_eq!(r.text.as_deref(), Some("你好"));
        assert_eq!(r.finish_reason, OurFinishReason::Stop);
    }

    #[tokio::test]
    async fn generate_maps_api_error() {
        let base = spawn_mock(|_line, _body| {
            MockResponse::json(
                400,
                r#"{"error":{"message":"model does not exist","type":"invalid_request_error","code":"model_not_found"}}"#,
            )
        })
        .await;
        let t = mock_transport(&base);
        let err = t.generate(request()).await.unwrap_err();
        assert!(
            matches!(&err, GatewayError::Model(m) if m.contains("model does not exist") && m.contains("model_not_found")),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn generate_stream_real_sse_chunks() {
        let frame = |delta: &str, finish: Option<&str>| {
            let finish = finish
                .map(|f| format!("\"{f}\""))
                .unwrap_or_else(|| "null".to_string());
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":{finish}}}]}}\n\n",
            )
        };
        let body = format!(
            "{}{}{}data: [DONE]\n\n",
            frame(r#"{"role":"assistant","content":"Hel"}"#, None),
            frame(r#"{"content":"lo!"}"#, None),
            frame("{}", Some("stop")),
        );
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport(&base);
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(chunks, vec!["Hel", "lo!"]);
        assert_eq!(r.text.as_deref(), Some("Hello!"));
        assert_eq!(r.finish_reason, OurFinishReason::Stop);
    }

    #[tokio::test]
    async fn generate_stream_real_sse_aggregates_tool_calls() {
        let frame = |delta: &str, finish: Option<&str>| {
            let finish = finish
                .map(|f| format!("\"{f}\""))
                .unwrap_or_else(|| "null".to_string());
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":{finish}}}]}}\n\n",
            )
        };
        let body = format!(
            "{}{}{}{}data: [DONE]\n\n",
            frame(
                r#"{"role":"assistant","tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"a.txt\"}"}}]}"#,
                None
            ),
            frame(
                r#"{"tool_calls":[{"index":1,"id":"call-2","type":"function","function":{"name":"grep","arguments":"{\"path\":"}}]}"#,
                None
            ),
            frame(
                r#"{"tool_calls":[{"index":1,"function":{"arguments":"\"b.txt\"}"}}]}"#,
                None
            ),
            // A real provider always terminates the stream with a
            // finish_reason block (D1-1).
            frame(r#"{}"#, Some("tool_calls")),
        );
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport(&base);
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert!(chunks.is_empty(), "tool-call round: no text chunks");
        assert_eq!(r.tool_calls.len(), 2);
        assert_eq!(r.tool_calls[0].name, "read_file");
        assert_eq!(
            r.tool_calls[0].arguments,
            serde_json::json!({"path": "a.txt"})
        );
        assert_eq!(r.tool_calls[0].call_id, "call-1");
        assert_eq!(r.tool_calls[1].name, "grep");
        assert_eq!(
            r.tool_calls[1].arguments,
            serde_json::json!({"path": "b.txt"})
        );
        assert_eq!(r.tool_calls[1].call_id, "call-2");
        assert_eq!(r.finish_reason, OurFinishReason::ToolCalls);
    }

    #[tokio::test]
    async fn generate_stream_truncated_stream_is_an_error() {
        // A stream that ends without a finish_reason block is truncated
        // (proxy drop / deploy switch / overload) — it must surface as an
        // error, never as a completed `stop` answer (D1-1).
        let frame = |delta: &str| {
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":null}}]}}\n\n",
            )
        };
        let body = format!(
            "{}{}data: [DONE]\n\n",
            frame(r#"{"role":"assistant","content":"half"}"#),
            frame(r#"{"content":"-done"}"#),
        );
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport(&base);
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Transport(m) if m.contains("finish_reason")),
            "truncated stream must error, got {err:?}"
        );
        // Chunks delivered so far are still honest deltas — but the response
        // must never be treated as complete.
        assert_eq!(chunks, vec!["half", "-done"]);
    }

    #[tokio::test]
    async fn generate_stream_cancelled_mid_stream() {
        let frame = |delta: &str| {
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":null}}]}}\n\n",
            )
        };
        let body = format!(
            "{}{}{}data: [DONE]\n\n",
            frame(r#"{"role":"assistant","content":"slow"}"#),
            frame(r#"{"content":"..."}"#),
            frame(r#"{"content":"done"}"#),
        );
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::from_millis(60))
        })
        .await;
        let t = mock_transport(&base);
        let cancel = tokio_util::sync::CancellationToken::new();
        // Cancel mid-stream: the first chunk lands, then the token fires.
        tokio::spawn({
            let cancel = cancel.clone();
            async move {
                tokio::time::sleep(std::time::Duration::from_millis(90)).await;
                cancel.cancel();
            }
        });
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), Some(&cancel), &mut |c| {
                chunks.push(c.to_string())
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::Cancelled),
            "mid-stream cancel must surface as Cancelled: {err:?}"
        );
        // At least the first chunk made it; the tail must not be projected
        // (P3-3: the stream is dropped, so no frame after the cancel lands).
        assert!(!chunks.is_empty(), "some chunks arrived before cancel");
        assert!(
            !chunks.iter().any(|c| c.contains("done")),
            "frames after the cancel must not be projected: {chunks:?}"
        );
    }

    #[tokio::test]
    async fn generate_stream_precancelled_opens_no_connection() {
        // P2-2: an already-cancelled run must not reach the wire at all —
        // the mock's handler panics if a request arrives.
        let base = spawn_mock(|_line, _body| {
            panic!("no request may be sent for a pre-cancelled run");
        })
        .await;
        let t = mock_transport(&base);
        let cancel = tokio_util::sync::CancellationToken::new();
        cancel.cancel();
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), Some(&cancel), &mut |c| {
                chunks.push(c.to_string())
            })
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::Cancelled),
            "pre-cancel must surface as Cancelled: {err:?}"
        );
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    #[ignore = "live transport — set ORZ_TEST_LIVE=1 (API key from Windows Credential Manager, ADR-0006)"]
    async fn live_chat_completion_roundtrip() {
        // Double gate: `#[ignore]` keeps the offline suite deterministic AND
        // the body bails unless the marker is set — so `--ignored` without
        // an explicit opt-in cannot silently pass (2026-08-06 conformance
        // review D2-1). The API key comes from the Windows Credential
        // Manager (`orz-deepseek/agent`, ADR-0006) — no env-var injection.
        if std::env::var("ORZ_TEST_LIVE").as_deref() != Ok("1") {
            return;
        }
        let key = match crate::gateway::credentials::read_agent_api_key() {
            Ok(key) => key,
            Err(e) => {
                eprintln!("live test skipped: {e}");
                return;
            }
        };
        let t = DeepSeekTransport::deepseek_v4(key, MAIN_AGENT_MODEL);
        let r = t
            .generate(ModelRequest {
                system: "You are a test assistant. Reply with exactly OK".to_string(),
                messages: vec![Message {
                    role: Role::User,
                    content: "ping".to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                }],
                tools: Vec::new(),
                max_tokens: 32,
            })
            .await;
        assert!(r.is_ok(), "{r:?}");
    }
}
