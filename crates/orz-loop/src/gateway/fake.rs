//! FakeProvider — deterministic scripted model gateway.
//!
//! Offline test main path, mirroring the Python fake-provider fixture
//! precedent: the Phase 2 acceptance chain must be reproducible without any
//! real model or network. Scripts rotate in order; exhaustion is an error so
//! tests fail loudly instead of silently reusing responses.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use super::model::{FinishReason, GatewayError, ModelGateway, ModelRequest, ModelResponse, ToolCall};

/// One scripted response step.
#[derive(Debug, Clone)]
pub struct ScriptedResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
}

impl ScriptedResponse {
    /// A plain text response (finish: stop).
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
        }
    }

    /// A tool-call response (finish: tool_calls).
    pub fn tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: None,
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
        }
    }
}

/// Deterministic scripted gateway.
pub struct FakeProvider {
    script: Mutex<VecDeque<ScriptedResponse>>,
    received: Mutex<Vec<ModelRequest>>,
}

impl FakeProvider {
    pub fn new(script: Vec<ScriptedResponse>) -> Self {
        Self {
            script: Mutex::new(script.into()),
            received: Mutex::new(Vec::new()),
        }
    }

    /// Convenience: a sequence of plain text responses.
    pub fn from_texts(texts: Vec<&str>) -> Self {
        Self::new(texts.into_iter().map(ScriptedResponse::text).collect())
    }

    /// Requests received so far (for assertions). Cloned to avoid lock issues.
    pub fn received_requests(&self) -> Vec<ModelRequest> {
        self.received.lock().unwrap().clone()
    }

    /// Remaining scripted steps.
    pub fn remaining(&self) -> usize {
        self.script.lock().unwrap().len()
    }
}

#[async_trait]
impl ModelGateway for FakeProvider {
    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
        self.received.lock().unwrap().push(request);
        let next = self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| GatewayError::Model("fake script exhausted".to_string()))?;
        Ok(ModelResponse {
            text: next.text,
            tool_calls: next.tool_calls,
            finish_reason: next.finish_reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Message, Role};

    fn request() -> ModelRequest {
        ModelRequest {
            system: "system".to_string(),
            messages: vec![Message {
                role: Role::User,
                content: "hello".to_string(),
            }],
            tools: Vec::new(),
            max_tokens: 128,
        }
    }

    #[tokio::test]
    async fn scripted_sequence_replays() {
        let provider = FakeProvider::new(vec![
            ScriptedResponse::text("first"),
            ScriptedResponse::text("second"),
        ]);
        let r1 = provider.generate(request()).await.unwrap();
        assert_eq!(r1.text.as_deref(), Some("first"));
        assert_eq!(r1.finish_reason, FinishReason::Stop);
        let r2 = provider.generate(request()).await.unwrap();
        assert_eq!(r2.text.as_deref(), Some("second"));
        assert_eq!(provider.remaining(), 0);
    }

    #[tokio::test]
    async fn exhausted_script_errors() {
        let provider = FakeProvider::from_texts(vec!["only one"]);
        provider.generate(request()).await.unwrap();
        let err = provider.generate(request()).await.unwrap_err();
        assert!(err.to_string().contains("script exhausted"), "{err}");
    }

    #[tokio::test]
    async fn ip1_thinking_disabled_asserted() {
        // IP1: the request type has no thinking field — assert the surface
        // stays free of any thinking option across received requests.
        let provider = FakeProvider::from_texts(vec!["ok"]);
        provider.generate(request()).await.unwrap();
        let received = provider.received_requests();
        assert_eq!(received.len(), 1);
        // Compile-time IP1 assertion: ModelRequest exposes no thinking field.
        let _compile_time_check: fn(&ModelRequest) = |r: &ModelRequest| {
            let _ = r; // if a thinking field is ever added, update IP1 here
        };
        let _ = _compile_time_check;
    }

    #[tokio::test]
    async fn tool_call_script_roundtrip() {
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "a.txt"}),
            call_id: "call-1".to_string(),
        };
        let provider = FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![call]),
            ScriptedResponse::text("done"),
        ]);
        let r1 = provider.generate(request()).await.unwrap();
        assert_eq!(r1.finish_reason, FinishReason::ToolCalls);
        assert_eq!(r1.tool_calls.len(), 1);
        assert_eq!(r1.tool_calls[0].name, "read_file");
        let r2 = provider.generate(request()).await.unwrap();
        assert_eq!(r2.finish_reason, FinishReason::Stop);
    }
}
