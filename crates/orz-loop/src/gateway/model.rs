//! Model gateway contract — request/response types + trait.
//!
//! Thinking policy (IP1 → D-6, FIX_PLAN 2026-08-06): thinking is carried by
//! `ModelConfig::thinking` (a transport-level knob — `ModelRequest` stays
//! thinking-free so a caller cannot enable it by accident). The restored
//! max-config is `enabled` + effort `max` + 160K budget, decided after the
//! 2026-08-07 live probe (reasoning deltas flow ~0.5s after connect; content
//! arrives ~30s later on hard tasks; `usage.reasoning_tokens` is reported
//! per round). The empty-final-content retry chain lives in the transport.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::host::ToolDef;

/// Transport retry/timeout policy (D-7, FIX_PLAN 2026-08-06 — absorbing the
/// Claude Code retry stack; values are the decided defaults, all parameterized
/// so a consumer can tighten/loosen without code change).
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Max retries on the non-streaming `create` path (bounded backoff; the
    /// fork's own loop is additionally window-capped by `request_retry_window`
    /// — the lower bound wins). 10 = Claude Code's maxConsecutive-scale cap.
    pub request_max_retries: u32,
    /// Backoff window cap for the non-streaming path (32s = Claude Code).
    pub request_retry_window: Duration,
    /// Single-request wall-clock timeout for the non-streaming path
    /// (18min level — F-03, 2026-08-07: the 160K max-config thinking budget
    /// can legitimately exceed 10min; 18min covers slow thinking plus
    /// network jitter. Non-streaming paths only — streaming paths are
    /// governed by the idle watchdog + total budget.)
    pub request_timeout: Duration,
    /// Stream idle watchdog: after this much silence, warn (20s, Claude Code).
    pub stream_idle_warn: Duration,
    /// Stream idle watchdog: after this much silence, hard abort (90s, Claude
    /// Code). Slow thinking with progress is NOT a timeout — only a dead wire.
    pub stream_idle_timeout: Duration,
    /// Total stream budget (auxiliary, generous — a relaxed backstop on top
    /// of the idle watchdog; full 160K thinking could exceed 10min).
    pub stream_total_timeout: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            request_max_retries: 10,
            request_retry_window: Duration::from_secs(32),
            request_timeout: Duration::from_secs(18 * 60),
            stream_idle_warn: Duration::from_secs(20),
            stream_idle_timeout: Duration::from_secs(90),
            stream_total_timeout: Duration::from_secs(30 * 60),
        }
    }
}

/// Thinking mode for the provider request (D-6, FIX_PLAN 2026-08-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThinkingMode {
    /// DeepSeek `thinking: {type: "enabled"}` + `reasoning_effort: "max"`
    /// (the restored max-config; default).
    #[default]
    EnabledMax,
    /// DeepSeek `thinking: {type: "disabled"}` — all output routed to
    /// `content` (the P2-era mitigation; kept for parity/tests).
    Disabled,
}

/// Configuration for a single model provider endpoint.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    pub provider: String,
    pub model_id: String,
    pub api_base: String,
    pub api_key: String,
    pub max_tokens: u32,
    /// Retry/timeout policy (D-7). Defaults are the decided values.
    pub retry: RetryPolicy,
    /// Thinking policy (D-6). Defaults to the restored max-config.
    pub thinking: ThinkingMode,
}

/// Conversation role for a message.
///
/// `Serialize/Deserialize` (GAP-CONVERSATION-RESTORE 2026-08-10): the session
/// conversation rides the `.gsa/conversations/<session8>.json` sidecar;
/// `snake_case` names match the provider protocol strings ("system"/"user"/
/// "assistant"/"tool").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
/// `Serialize/Deserialize` (GAP-CONVERSATION-RESTORE 2026-08-10): the full
/// conversation (incl. `reasoning_content` — DeepSeek multi-turn replay
/// requires it back on assistant declaration messages) persists across
/// prompts via the conversation sidecar. Privacy boundary: the sidecar is NOT
/// the journal evidence face — ADR-0010 §5.4.6 restricts reasoning text only
/// from the journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub reasoning_content: Option<String>,
}

/// A tool call requested by the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Request sent to the model. `thinking` is normally transport-level config
/// (IP1: the controller does not decide thinking silently) — the optional
/// override exists for explicitly marked fast-preflight rounds (2026-08-07
/// review F-07: the `-p` plan gate, max_tokens=1024, must not run the
/// thinking-max empty-content chain). `None` = the transport's `ModelConfig`
/// default; call sites that set it override deliberately.
#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolDef>,
    pub max_tokens: u32,
    /// Per-request thinking override; `None` = transport config default.
    pub thinking: Option<ThinkingMode>,
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
    /// Token usage observation (D-6): reasoning tokens for this completion,
    /// when the provider reports them (DeepSeek `usage.completion_tokens_
    /// details.reasoning_tokens`). Journaled for budget/latency calibration —
    /// the 160K budget decision rolls back on the data.
    pub reasoning_tokens: Option<u32>,
    /// Total completion tokens for this response (usage.completion_tokens).
    pub completion_tokens: Option<u32>,
    /// Prompt tokens served from the provider's prefix cache (DeepSeek
    /// `usage.prompt_cache_hit_tokens`) — 2026-08-07 cache-hit observation.
    pub cache_hit_tokens: Option<u64>,
    /// Prompt tokens NOT served from cache (`usage.prompt_cache_miss_tokens`).
    pub cache_miss_tokens: Option<u64>,
    /// Total prompt tokens of the request (`usage.prompt_tokens`) — A6
    /// (2026-08-08): the measured trigger for explicit context compaction
    /// (design §8 C.1: >160K rhythm at the final-answer gap, >250K safety
    /// at any gap — user decisions). `hit + miss` would be the same value
    /// when both are reported, but the total is the direct measure and
    /// works even when the cache breakdown is absent.
    pub prompt_tokens: Option<u64>,
}

impl ModelResponse {
    pub fn text_response(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            reasoning_content: None,
            reasoning_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            cache_miss_tokens: None,
            prompt_tokens: None,
        }
    }

    pub fn tool_calls_response(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            text: None,
            tool_calls,
            finish_reason: FinishReason::ToolCalls,
            reasoning_content: None,
            reasoning_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            cache_miss_tokens: None,
            prompt_tokens: None,
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
    /// Wall-clock timeout or idle watchdog fired (D-7, FIX_PLAN 2026-08-06).
    /// Strictly distinct from `Cancelled`: a timeout is a transport failure
    /// (the loop surfaces it as a model error), never a user cancel.
    #[error("generation timed out: {0}")]
    Timeout(String),
    /// GAP-STREAM-RETRY (2026-08-12): a streaming attempt failed BEFORE any
    /// chunk was produced — no model output existed, so re-sending the
    /// identical request body is side-effect-free (idempotent; ADR-0007 §3
    /// known-boundary premise). The transport retries these with bounded
    /// backoff; `attempts` records how many re-sends happened before the
    /// final failure, so the journal shows the retry history. The display
    /// word is "re-sends" — `attempts` counts RETRIES, so the journal text
    /// never reads as total attempts (off-by-one; 2026-08-12 review D2-1).
    #[error("stream interrupted before any chunk (after {attempts} re-sends): {detail}")]
    StreamInterrupted {
        /// Retries already consumed when this error escaped. `0` = the
        /// transport surfaced it without a retry (or surfaced the very
        /// first attempt's failure).
        attempts: u32,
        detail: String,
    },
}

/// P1-1 (2026-08-08 stall guards): process-level heartbeat clock. Any
/// activity source — a journal event (the controller stamps on every
/// record), a model SSE frame (transports stamp per frame), a streamed
/// content chunk (the fake + the default buffered path stamp per chunk) —
/// calls [`ActivityClock::stamp`]. The stall watchdog fires when no stamp
/// has landed for its timeout, i.e. the process went silent anywhere.
///
/// Cheap and lock-free (one relaxed atomic store per stamp); stamping
/// unconditionally is fine.
///
/// MONOTONIC SOURCE (2026-08-08 review P2-2): elapsed time is measured
/// against a construction-time [`Instant`] baseline, never wall clock —
/// a machine suspend or NTP step would otherwise jump `idle()` past the
/// watchdog window and kill a healthy run on wake.
#[derive(Clone, Debug)]
pub struct ActivityClock {
    /// Monotonic baseline taken at construction.
    start: std::time::Instant,
    /// `start.elapsed()` in ms at the last stamp.
    last_elapsed_ms: Arc<std::sync::atomic::AtomicU64>,
}

impl ActivityClock {
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
            last_elapsed_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Record an activity instant (called on every event/frame/chunk).
    pub fn stamp(&self) {
        self.last_elapsed_ms.store(
            self.elapsed_ms(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Duration since the last stamp — the stall watchdog compares this
    /// against its timeout.
    pub fn idle(&self) -> Duration {
        Duration::from_millis(
            self.elapsed_ms()
                .saturating_sub(self.last_elapsed_ms.load(std::sync::atomic::Ordering::Relaxed)),
        )
    }

    fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

impl Default for ActivityClock {
    fn default() -> Self {
        Self::new()
    }
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
    ///
    /// `heartbeat` (P1-1, 2026-08-08 stall guards): stamped on every wire
    /// frame the transport receives — INCLUDING reasoning-only deltas the
    /// controller never sees (a long max-effort thinking stream must count
    /// as activity; only the transport observes its frames). `None` keeps
    /// the pre-guard behavior.
    async fn generate_stream(
        &self,
        request: ModelRequest,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        let _ = cancel;
        let response = self.generate(request).await?;
        // Clone so the closure's borrow cannot outlive the response move.
        if let Some(t) = response.text.clone().filter(|t| !t.is_empty()) {
            if let Some(h) = heartbeat {
                h.stamp();
            }
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
                    thinking: None,
                },
                None,
                None,
                &mut |c| chunks.push(c.to_string()),
            )
            .await
            .unwrap();
        assert_eq!(chunks, vec!["buffered"]);
        assert_eq!(response.text.as_deref(), Some("buffered"));
    }
}
