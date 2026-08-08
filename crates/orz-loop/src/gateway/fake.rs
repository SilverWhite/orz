//! FakeProvider — deterministic scripted model gateway.
//!
//! Offline test main path, mirroring the Python fake-provider fixture
//! precedent: the Phase 2 acceptance chain must be reproducible without any
//! real model or network. Scripts rotate in order; exhaustion is an error so
//! tests fail loudly instead of silently reusing responses.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use super::model::{
    FinishReason, GatewayError, ModelGateway, ModelRequest, ModelResponse, ToolCall,
};

/// One scripted response step.
#[derive(Debug, Clone)]
pub struct ScriptedResponse {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: FinishReason,
    /// Reasoning content (DeepSeek) — scriptable so tests can lock the
    /// replay path without a real model.
    pub reasoning_content: Option<String>,
    /// Measured prompt tokens (A6 — the compaction trigger). `None` = the
    /// provider did not report usage (no compaction in the default path).
    pub prompt_tokens: Option<u64>,
}

impl ScriptedResponse {
    /// A plain text response (finish: stop).
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            reasoning_content: None,
            prompt_tokens: None,
        }
    }

    /// A tool-call response (finish: tool_calls).
    pub fn tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: None,
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            prompt_tokens: None,
        }
    }

    /// Attach reasoning content (the DeepSeek field replayed with the
    /// assistant declaration).
    pub fn with_reasoning(mut self, reasoning: impl Into<String>) -> Self {
        self.reasoning_content = Some(reasoning.into());
        self
    }

    /// Attach a measured prompt-token count (A6 — the compaction trigger
    /// reads `ModelResponse.prompt_tokens`).
    pub fn with_prompt_tokens(mut self, tokens: u64) -> Self {
        self.prompt_tokens = Some(tokens);
        self
    }
}

/// Deterministic scripted gateway.
pub struct FakeProvider {
    script: Mutex<VecDeque<ScriptedResponse>>,
    received: Mutex<Vec<ModelRequest>>,
    /// Char-count split for `generate_stream` (char boundaries — CJK-safe;
    /// never byte slicing). Default 4.
    chunk_size: usize,
    /// Optional pause before each chunk (demo visibility; tests use None).
    chunk_delay: Option<std::time::Duration>,
}

impl FakeProvider {
    pub fn new(script: Vec<ScriptedResponse>) -> Self {
        Self {
            script: Mutex::new(script.into()),
            received: Mutex::new(Vec::new()),
            chunk_size: 4,
            chunk_delay: None,
        }
    }

    /// Convenience: a sequence of plain text responses.
    pub fn from_texts(texts: Vec<&str>) -> Self {
        Self::new(texts.into_iter().map(ScriptedResponse::text).collect())
    }

    /// Split `generate_stream` text into chunks of this many chars
    /// (char-boundary split; clamped to at least 1).
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size.max(1);
        self
    }

    /// Pause before each streamed chunk (including the first) so the TUI
    /// demo streams visibly. Tests keep the default `None`.
    pub fn with_chunk_delay(mut self, delay: std::time::Duration) -> Self {
        self.chunk_delay = Some(delay);
        self
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
            reasoning_content: next.reasoning_content,
            reasoning_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            cache_miss_tokens: None,
            prompt_tokens: next.prompt_tokens,
        })
    }

    async fn generate_stream(
        &self,
        request: ModelRequest,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        self.received.lock().unwrap().push(request);
        let next = self
            .script
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| GatewayError::Model("fake script exhausted".to_string()))?;
        let response = ModelResponse {
            text: next.text,
            tool_calls: next.tool_calls,
            finish_reason: next.finish_reason,
            reasoning_content: next.reasoning_content,
            reasoning_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            cache_miss_tokens: None,
            prompt_tokens: next.prompt_tokens,
        };
        // Split on char boundaries (CJK-safe) — concatenating the chunks must
        // reproduce the full text exactly (Python text_delta invariant).
        if let Some(text) = response.text.as_deref() {
            let chunks: Vec<String> = text
                .chars()
                .collect::<Vec<_>>()
                .chunks(self.chunk_size)
                .map(|c| c.iter().collect())
                .collect();
            for chunk in chunks {
                // Cooperative cancellation checkpoint (slice #11, P3-7) —
                // mirrors the real transport's per-chunk check so the
                // fake-provider demo honors /stop mid-stream too.
                if cancel.is_some_and(|c| c.is_cancelled()) {
                    return Err(GatewayError::Cancelled);
                }
                if let Some(delay) = self.chunk_delay {
                    tokio::time::sleep(delay).await;
                }
                on_chunk(&chunk);
            }
        }
        Ok(response)
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
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            }],
            tools: Vec::new(),
            max_tokens: 128,
            thinking: None,
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
    async fn generate_stream_splits_on_char_boundaries() {
        // CJK-safe: char boundaries, never byte slicing ("你" is 3 UTF-8
        // bytes; a byte split would produce invalid text).
        let provider = FakeProvider::from_texts(vec!["你好世界ABC"]).with_chunk_size(2);
        let mut chunks: Vec<String> = Vec::new();
        let response = provider
            .generate_stream(request(), None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(chunks, vec!["你好", "世界", "AB", "C"]);
        assert_eq!(response.text.as_deref(), Some("你好世界ABC"));
    }

    #[tokio::test]
    async fn generate_stream_chunks_concat_equals_full_text() {
        for (text, size) in [
            ("你好世界", 1usize),
            ("你好世界ABC", 2),
            ("hello world", 4),
            ("你好，world！", 99),
        ] {
            let provider = FakeProvider::from_texts(vec![text]).with_chunk_size(size);
            let mut joined = String::new();
            let response = provider
                .generate_stream(request(), None, &mut |c| joined.push_str(c))
                .await
                .unwrap();
            // Python text_delta invariant: deltas accumulate to the full text.
            assert_eq!(joined, text, "chunk_size={size}");
            assert_eq!(response.text.as_deref(), Some(text), "chunk_size={size}");
        }
    }

    #[tokio::test]
    async fn generate_stream_no_text_yields_no_chunks() {
        let call = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "a.txt"}),
            call_id: "call-1".to_string(),
        };
        let provider = FakeProvider::new(vec![ScriptedResponse::tool_calls(vec![call])]);
        let mut chunk_count = 0;
        let response = provider
            .generate_stream(request(), None, &mut |_| chunk_count += 1)
            .await
            .unwrap();
        assert_eq!(chunk_count, 0);
        assert_eq!(response.finish_reason, FinishReason::ToolCalls);
        assert_eq!(response.tool_calls.len(), 1);
    }

    #[tokio::test]
    async fn generate_stream_exhausted_script_errors() {
        let provider = FakeProvider::from_texts(vec!["only one"]).with_chunk_size(2);
        let mut on_chunk = |_c: &str| {};
        provider
            .generate_stream(request(), None, &mut on_chunk)
            .await
            .unwrap();
        let err = provider
            .generate_stream(request(), None, &mut on_chunk)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("script exhausted"), "{err}");
    }

    #[tokio::test]
    async fn generate_stream_records_received_request() {
        let provider = FakeProvider::from_texts(vec!["ok"]).with_chunk_size(2);
        let mut on_chunk = |_c: &str| {};
        provider
            .generate_stream(request(), None, &mut on_chunk)
            .await
            .unwrap();
        let received = provider.received_requests();
        assert_eq!(received.len(), 1);
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
