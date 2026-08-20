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
//! Thinking (IP1 → D-6, FIX_PLAN 2026-08-06): the default workpoint is
//! `thinking: {type: "enabled"}` + `reasoning_effort: "high"` + 256K
//! single-round output budget (OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD
//! 2026-08-20, ADR-0010 §14.35 — 输出预算恢复 32K → 256K，回落档 128K，
//! S4 实测校准；2026-08-20 修订: 默认档 max → high（官方 harness 默认），
//! `EnabledMax` 保留为显式可选档、`EnabledLow` 为降级梯中间档——
//! generate_stream 降级梯 high → low → disabled → 失败；OUTPUT-
//! DEGENERATION-GUARD 2026-08-19 的 32K 止损由输出健康哨兵 + 空转预算
//! 兜底取代；previously 160K);
//! `ThinkingMode::Disabled` keeps the P2-era mitigation (all output to
//! `content`) for parity/tests/benchmarks. `ModelRequest` carries no
//! thinking knob — `ModelConfig::thinking` is the single switch.
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
        ChatCompletionRequestUserMessageContent, ChatCompletionStreamOptions,
        ChatCompletionStreamResponseDelta, ChatCompletionTool, ChatCompletionTools,
        CreateChatCompletionRequest, CreateChatCompletionResponse,
        CreateChatCompletionStreamResponse, FinishReason, FunctionCallStream, FunctionObject,
        ReasoningEffort,
    },
};
use async_trait::async_trait;
use backoff::backoff::Backoff;
use futures::StreamExt;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::model::{
    FinishReason as OurFinishReason, GatewayError, ModelConfig, ModelGateway, ModelRequest,
    ModelResponse, ThinkingMode, ToolCall,
};

/// Default DeepSeek API base (OpenAI-compatible).
pub const DEFAULT_DEEPSEEK_API_BASE: &str = "https://api.deepseek.com";

/// Main-agent model id (single source of truth — the live test and the
/// production `--real` gateway share it, so a model change cannot drift
/// between the two).
pub const MAIN_AGENT_MODEL: &str = "deepseek-v4-flash";

/// Main-agent model id for the production gateway — `ORZ_MAIN_AGENT_MODEL`
/// env override for evaluation/model-swap runs (e.g. TB B 组 flash→pro
/// 对照, 2026-08-11); the constant remains the production default.
pub fn main_agent_model() -> String {
    std::env::var("ORZ_MAIN_AGENT_MODEL").unwrap_or_else(|_| MAIN_AGENT_MODEL.to_string())
}

// ───────────────────────────────────────────────────────────────────────────
// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.3)
// ───────────────────────────────────────────────────────────────────────────

/// 连续相同 content delta 触发阈值（N=5；设计 §3.3 检测算法，保守初值）。
pub const DEGENERATION_CONSECUTIVE_DELTAS: usize = 5;

/// 重复率检测的累计 token 门槛——累计输出 ≥ 1K token 后才检查最近 1K token
/// 窗口（设计 §3.3；复读诱因=超长输出，窗口足够小前不误报）。
pub const DEGENERATION_MIN_TOKENS: usize = 1_000;

/// 最近窗口内 3-gram 重复率阈值（>60% 触发；设计 §3.3）。
pub const DEGENERATION_NGRAM_REPEAT_RATIO: f64 = 0.60;

/// 同一会话连续退化中断上限（默认 3；设计 §3.3 防循环——第 3 次起以
/// `degeneration_limit_reached` 标记，run 层据此记 `run_invalidated`，
/// 计入 stagnation 同类终止态）。
pub const DEGENERATION_LIMIT: u32 = 3;

/// 退化中断 detail 前缀（命中但未达上限——run 走 run_failed 同路径）。
pub const DEGENERATION_DETAIL_PREFIX: &str = "degeneration_detected:";

/// 退化中断达上限 detail 前缀（run 层映射为 `run_invalidated`）。
pub const DEGENERATION_LIMIT_PREFIX: &str = "degeneration_limit_reached:";

// ───────────────────────────────────────────────────────────────────────────
// OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35 /
// 设计 §3.2/§3.3)：D-6 空流链官方化收窄 + 退化检测器升级为输出健康哨兵。
// ───────────────────────────────────────────────────────────────────────────

/// 完成型空响应（`empty_content_abnormal`）快速有界重试上限（设计 §3.2：
/// 官方 llm-retry 5 次收窄版——256K+max 下 5 次原样重试成本不可接受；
/// 长烧型空转由 stall 哨兵提前中断、不进入原样重试）。
pub const EMPTY_RESPONSE_MAX_RETRIES: u32 = 2;

/// 空流重试退避初值（官方默认形状，设计 §3.2/§3.5）。
pub const EMPTY_RESPONSE_BACKOFF_INITIAL: Duration = Duration::from_millis(500);

/// 空流重试退避上限（官方默认形状，设计 §3.2/§3.5）。
pub const EMPTY_RESPONSE_BACKOFF_MAX: Duration = Duration::from_secs(10);

/// 空流重试退避抖动（官方默认 ±10%，设计 §3.2/§3.5）。
pub const EMPTY_RESPONSE_BACKOFF_JITTER: f64 = 0.10;

/// reasoning-stall 时间信号：自首 chunk 起无 content/tool_calls 的等待上限
/// （设计 §3.3 预算兜底层；初值 600s，S4 校准 300–900s）。
pub const STALL_FIRST_CONTENT_TIMEOUT: Duration = Duration::from_secs(600);

/// reasoning-stall token 信号：reasoning 估算累计无 content/tool_calls 的
/// 空转预算（设计 §3.3——**与 max_tokens 解耦**，256K 恢复后空转不随预算
/// 放大；初值 64K，S4 校准 32–128K；单次最坏 ≈ ¥0.29 ≈ 现状整条空流链）。
pub const STALL_REASONING_BUDGET_TOKENS: usize = 64_000;

/// reasoning 字符 → token 估算系数（设计 §3.3；沿用折叠桥 8K 实测校准值
/// 2 字符/token，S4 用 usage 真实 reasoning_tokens 复核）。
pub const REASONING_CHARS_PER_TOKEN: usize = 2;

/// content 复读族 detail 前缀（已见输出 → 不重试，ADR-0007）。
pub const CONTENT_REPETITION_DETAIL_PREFIX: &str = "degeneration_detected:content_repetition:";

/// reasoning 复读族 detail 前缀（灵敏层——无可见输出 → 不原样、直接降级）。
pub const REASONING_REPETITION_DETAIL_PREFIX: &str = "degeneration_detected:reasoning_repetition:";

/// reasoning-stall 族 detail 前缀（预算兜底层——无可见输出 → 直接降级）。
pub const REASONING_STALL_DETAIL_PREFIX: &str = "degeneration_detected:reasoning_stall:";

/// 生成期输出健康哨兵（设计 §3.3，第一层治本）：喂入 content delta +
/// reasoning delta + tool_call arguments delta，三族信号（content_repetition
/// / reasoning_repetition / reasoning_stall）命中后持续返回触发原因。纯
/// 机械、零模型调用；token 口径复用 `orz_assurance::orientation::stagnation`
/// （Unicode 词 + CJK 正则），轻量实现（窗口 ≤1K token 的 3-gram 计数）。
#[derive(Debug, Default)]
struct DegenerationDetector {
    /// content 族：最近 N 个 delta（连续相同检测）。
    recent_content_deltas: VecDeque<String>,
    /// content 族：累计 token 数（重复率检测的启用门槛）。
    content_total_tokens: usize,
    /// content 族：最近 1K token 的滑动窗口。
    content_window_tokens: Vec<String>,
    /// reasoning 族：最近 N 个 delta（连续相同检测）。
    recent_reasoning_deltas: VecDeque<String>,
    /// reasoning 族：累计 token 数（重复率检测的启用门槛）。
    reasoning_total_tokens: usize,
    /// reasoning 族：最近 1K token 的滑动窗口。
    reasoning_window_tokens: Vec<String>,
    /// reasoning 累计字符 → 估算 token（字符 ÷ `REASONING_CHARS_PER_TOKEN`；
    /// 空转预算信号，与 max_tokens 解耦；S4 用 usage 真实值复核）。
    reasoning_chars: usize,
    /// 已见可见输出（content 或 tool_calls 出现）——此后 reasoning 族信号
    /// 停用（设计 §3.3：reasoning 复读/stall 仅 content/tool_calls 全空时
    /// 启用；工具轮为合法形态，不误判）。
    saw_visible_output: bool,
    /// 首 chunk 时刻（stall 时间信号的起算点；任意族首个非空 delta 置位）。
    first_chunk_at: Option<std::time::Instant>,
    /// 触发原因（触发后恒定，避免同流重复报错）。
    trip: Option<String>,
}

impl DegenerationDetector {
    /// content 族 feed：内容即可见输出——标记 visible（reasoning 族停用）
    /// 后走复读检测（连续相同 / 1K 窗口 3-gram 重复率）。
    fn feed_content(&mut self, delta: &str) {
        if self.trip.is_some() || delta.is_empty() {
            return;
        }
        self.mark_first_chunk();
        self.saw_visible_output = true;
        feed_repetition(
            &mut self.trip,
            "content",
            CONTENT_REPETITION_DETAIL_PREFIX,
            delta,
            &mut self.recent_content_deltas,
            &mut self.content_total_tokens,
            &mut self.content_window_tokens,
        );
    }

    /// reasoning 族 feed（灵敏层，设计 §3.3）：累计字符 → 估算 token 供
    /// stall 预算；复读检测仅 content/tool_calls 全空时启用。
    fn feed_reasoning(&mut self, delta: &str) {
        if self.trip.is_some() || delta.is_empty() {
            return;
        }
        self.mark_first_chunk();
        self.reasoning_chars = self.reasoning_chars.saturating_add(delta.chars().count());
        if self.saw_visible_output {
            return;
        }
        feed_repetition(
            &mut self.trip,
            "reasoning",
            REASONING_REPETITION_DETAIL_PREFIX,
            delta,
            &mut self.recent_reasoning_deltas,
            &mut self.reasoning_total_tokens,
            &mut self.reasoning_window_tokens,
        );
    }

    /// tool_call arguments 族 feed（观测面，设计 §3.3）：arguments 出现即
    /// tool_calls 非空——可见输出成型，reasoning 族信号停用。复读检测仅
    /// 定义 content/reasoning 两族，tool 参数不参与 n-gram 判定（JSON 参数
    /// 结构重复易误报，设计信号表无此族）。
    fn feed_tool_arguments(&mut self, delta: &str) {
        if self.trip.is_some() || delta.is_empty() {
            return;
        }
        self.mark_first_chunk();
        self.saw_visible_output = true;
    }

    fn mark_first_chunk(&mut self) {
        if self.first_chunk_at.is_none() {
            self.first_chunk_at = Some(std::time::Instant::now());
        }
    }

    /// reasoning 估算 token（字符 ÷ `REASONING_CHARS_PER_TOKEN`）。
    fn reasoning_est_tokens(&self) -> usize {
        self.reasoning_chars / REASONING_CHARS_PER_TOKEN
    }

    /// 预算兜底层（设计 §3.3）：自首 chunk 起无 content/tool_calls 且
    /// reasoning 在流动，超过时间预算（600s）或估算 token ≥64K（OR）→
    /// 触发 reasoning_stall。逐 chunk 调用（chunk 持续到达时在预算点附近
    /// 触发；完全静默由 idle 死线处理——两者互补不重叠）。
    fn check_stall(&mut self, now: std::time::Instant) {
        if self.trip.is_some() {
            return;
        }
        let Some(first) = self.first_chunk_at else {
            return;
        };
        if self.saw_visible_output {
            return;
        }
        let est_tokens = self.reasoning_est_tokens();
        let elapsed = now.saturating_duration_since(first);
        if elapsed >= STALL_FIRST_CONTENT_TIMEOUT || est_tokens >= STALL_REASONING_BUDGET_TOKENS {
            self.trip = Some(format!(
                "{REASONING_STALL_DETAIL_PREFIX} no content/tool_calls for {:.0}s with \
                 ~{est_tokens} estimated reasoning tokens (budget {}s / {} tokens)",
                elapsed.as_secs_f64(),
                STALL_FIRST_CONTENT_TIMEOUT.as_secs(),
                STALL_REASONING_BUDGET_TOKENS,
            ));
        }
    }

    fn trip_reason(&self) -> Option<String> {
        self.trip.clone()
    }
}

/// 复读检测共用核心（content 与 reasoning 同一算法，设计 §3.3）：
/// ① 连续相同 delta N=5 → 触发；② 累计 ≥1K token 且最近 1K token 内
/// 3-gram 重复率 >60% → 触发。自由函数（非方法）——调用方以不相交的
/// 字段借用传入，避免方法整体借用与字段借用冲突。
fn feed_repetition(
    trip: &mut Option<String>,
    family: &str,
    detail_prefix: &str,
    delta: &str,
    recent_deltas: &mut VecDeque<String>,
    total_tokens: &mut usize,
    window_tokens: &mut Vec<String>,
) {
    if trip.is_some() || delta.is_empty() {
        return;
    }
    // ① 连续相同块：最近连续 N 个 delta 完全相同 → 触发。
    recent_deltas.push_back(delta.to_string());
    if recent_deltas.len() > DEGENERATION_CONSECUTIVE_DELTAS {
        recent_deltas.pop_front();
    }
    if recent_deltas.len() == DEGENERATION_CONSECUTIVE_DELTAS && {
        let last = recent_deltas.back().expect("len == N");
        recent_deltas.iter().all(|d| d == last)
    } {
        *trip = Some(format!(
            "{detail_prefix} {n} identical {family} deltas in a row",
            n = DEGENERATION_CONSECUTIVE_DELTAS
        ));
        return;
    }
    // ② 重复率：累计 ≥ 1K token 且最近 1K token 内 3-gram 重复率 > 60%
    //    → 触发（复用 stagnation 的 n-gram 思路，轻量实现）。
    let tokens = orz_assurance::orientation::stagnation::tokenize(delta);
    *total_tokens += tokens.len();
    window_tokens.extend(tokens);
    let overflow = window_tokens.len().saturating_sub(DEGENERATION_MIN_TOKENS);
    if overflow > 0 {
        window_tokens.drain(..overflow);
    }
    if *total_tokens >= DEGENERATION_MIN_TOKENS {
        let total_ngrams = window_tokens.len().saturating_sub(2);
        if total_ngrams >= 1 {
            let mut counts: HashMap<&[String], u32> = HashMap::new();
            for index in 0..total_ngrams {
                *counts.entry(&window_tokens[index..index + 3]).or_insert(0) += 1;
            }
            let distinct = counts.len();
            let duplicated = total_ngrams.saturating_sub(distinct);
            let ratio = duplicated as f64 / total_ngrams as f64;
            if ratio > DEGENERATION_NGRAM_REPEAT_RATIO {
                *trip = Some(format!(
                    "{detail_prefix} 3-gram repetition ratio {ratio:.2} \
                     in the recent {} {family} tokens",
                    window_tokens.len()
                ));
            }
        }
    }
}

/// content 族退化中断 detail 判定（语义收窄为 content 族「不重试」判定，
/// 设计 §3.3——已见输出，ADR-0007 纪律；limit 前缀三族共享，达限转
/// run_invalidated）。`stream_once_with_retry` / run 层以
/// `is_degeneration_detail || is_reasoning_guard_detail` 合并使用。
pub(crate) fn is_degeneration_detail(detail: &str) -> bool {
    detail.starts_with(CONTENT_REPETITION_DETAIL_PREFIX)
        || detail.starts_with(DEGENERATION_LIMIT_PREFIX)
}

/// reasoning 族退化中断 detail 判定（设计 §3.3：reasoning_repetition /
/// reasoning_stall——无可见输出，不原样重试、直接降级）。
pub(crate) fn is_reasoning_guard_detail(detail: &str) -> bool {
    detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX)
        || detail.starts_with(REASONING_STALL_DETAIL_PREFIX)
}

/// 审计/日志用触发族标签（detail 可能被 limit 前缀包裹，用 contains 判定
/// 内部族；仅留痕、不改终止语义）。
pub(crate) fn guard_family_label(detail: &str) -> &'static str {
    if detail.contains(CONTENT_REPETITION_DETAIL_PREFIX) {
        "content_repetition"
    } else if detail.contains(REASONING_REPETITION_DETAIL_PREFIX) {
        "reasoning_repetition"
    } else if detail.contains(REASONING_STALL_DETAIL_PREFIX) {
        "reasoning_stall"
    } else if detail.starts_with(DEGENERATION_LIMIT_PREFIX) {
        "degeneration_limit"
    } else {
        "other"
    }
}

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
        main_agent_model(),
    )))
}

/// Real model transport over the OpenAI-compatible chat completions API.
#[derive(Debug, Clone)]
pub struct DeepSeekTransport {
    pub config: ModelConfig,
    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33)：会话级连续
    /// 退化中断计数（Arc 共享——Clone 不复制计数；任何成功请求重置）。
    /// 达到 `DEGENERATION_LIMIT` 后下一次退化中断带
    /// `degeneration_limit_reached` 标记，run 层据此记 run_invalidated
    /// （防会话级循环；设计 §3.3）。
    degeneration_consecutive: Arc<AtomicU32>,
}

impl DeepSeekTransport {
    pub fn new(config: ModelConfig) -> Self {
        Self {
            config,
            degeneration_consecutive: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Convenience constructor for DeepSeek V4 models.
    pub fn deepseek_v4(api_key: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self::new(ModelConfig {
            provider: "deepseek".to_string(),
            model_id: model_id.into(),
            api_base: DEFAULT_DEEPSEEK_API_BASE.to_string(),
            api_key: api_key.into(),
            // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20,
            // ADR-0010 §14.35): 输出预算恢复 32K → 256K（官方 maxTokens
            // 默认值，与 effort 独立维度；回落档 128K，S4 实测不可接受才
            // 回落，编译期常量）。止损由输出健康哨兵承担——content 复读
            // 检测（P0-0d）+ reasoning 复读灵敏层 + reasoning-stall
            // 600s/64K 预算兜底（与 max_tokens 解耦），空流不原样重试
            // （D-6 快速有界 ≤2 次 + 降级出口）。请求头指纹含 max_tokens
            // 与 retry 参数（idle 5s/30s、重试窗口 50s），部署后首次请求
            // 一次性指纹变化（既有纪律）。2026-08-20 修订: 默认 thinking
            // 档 max → high（官方默认；`EnabledMax` 显式可选），降级梯
            // high → low → disabled（ADR-0010 §14.35 第 5 项 / 设计 §3.6）。
            max_tokens: 256_000,
            retry: Default::default(),
            thinking: Default::default(),
        })
    }

    fn client(&self) -> Client<OpenAIConfig> {
        let policy = &self.config.retry;
        // D-7: bounded backoff — 50s window cap (STREAM-RETRY-RHYTHM
        // 2026-08-20, was 32s) on top of the fork's retry loop, plus a hard
        // retry-count cap. The fork default is a 15-minute window; a stuck
        // upstream must not hold the request for 15 minutes.
        let backoff = backoff::ExponentialBackoff {
            max_elapsed_time: Some(policy.request_retry_window),
            ..Default::default()
        };
        Client::with_config(
            OpenAIConfig::new()
                .with_api_key(self.config.api_key.clone())
                .with_api_base(self.config.api_base.clone()),
        )
        .with_backoff(backoff)
        .with_max_retries(policy.request_max_retries)
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the
    /// transport-level config digest for the request-header fingerprint.
    /// The API key is deliberately excluded — a key rotation must never
    /// surface as a header change (it is not part of the provider's
    /// prefix-cache key). Boundary (2026-08-15 review): per-request
    /// overrides (`request.max_tokens` min-cap and `request.thinking`) are
    /// NOT part of the digest — current in-loop requests pass
    /// config-equivalent values, and any future request-level override
    /// routed through the loop must be registered here.
    fn config_fingerprint_impl(&self) -> String {
        let policy = &self.config.retry;
        orz_assurance::journal::chain::payload_hash(&serde_json::json!({
            "provider": self.config.provider,
            "model_id": self.config.model_id,
            "api_base": self.config.api_base,
            "max_tokens": self.config.max_tokens,
            "thinking": match self.config.thinking {
                ThinkingMode::EnabledHigh => "enabled_high",
                ThinkingMode::EnabledLow => "enabled_low",
                ThinkingMode::EnabledMax => "enabled_max",
                ThinkingMode::Disabled => "disabled",
            },
            "retry": {
                "request_max_retries": policy.request_max_retries,
                "request_retry_window_ms": policy.request_retry_window.as_millis(),
                "request_timeout_ms": policy.request_timeout.as_millis(),
                "stream_idle_warn_ms": policy.stream_idle_warn.as_millis(),
                "stream_idle_timeout_ms": policy.stream_idle_timeout.as_millis(),
                "stream_total_timeout_ms": policy.stream_total_timeout.as_millis(),
            },
        }))
        .unwrap_or_else(|_| "config-fingerprint-error".to_string())
    }

    /// Whether a response is an abnormal empty-content case (D-6): the
    /// final round produced NO text AND NO tool calls — the thinking budget
    /// ate everything (finish=length with zero content) or the model
    /// returned nothing at all. Tool rounds (content='' + tool_calls) are
    /// legal and never retried.
    fn empty_content_abnormal(response: &ModelResponse) -> bool {
        response.text.is_none() && response.tool_calls.is_empty()
    }

    /// F-07 (2026-08-07 review): effective thinking for a request — the
    /// per-request override (explicit fast-preflight rounds) or the
    /// transport config default.
    fn effective_thinking(&self, request: &ModelRequest) -> ThinkingMode {
        request.thinking.unwrap_or(self.config.thinking)
    }

    /// Apply a thinking tier to an already-built request — BOTH the
    /// `thinking` block AND `reasoning_effort` follow the tier (D-6;
    /// `build_request` derives both from `config.thinking`, so a ladder /
    /// override tier must override both knobs — e.g. a high-config
    /// transport degrading to `EnabledLow` must send
    /// `reasoning_effort: "low"`, not the config's high).
    fn apply_thinking(req: &mut CreateChatCompletionRequest, thinking: ThinkingMode) {
        req.thinking = Some(async_openai::types::chat::ThinkingConfig {
            thinking_type: match thinking {
                ThinkingMode::EnabledHigh | ThinkingMode::EnabledLow | ThinkingMode::EnabledMax => {
                    "enabled".to_string()
                }
                ThinkingMode::Disabled => "disabled".to_string(),
            },
        });
        req.reasoning_effort = match thinking {
            ThinkingMode::EnabledHigh => Some(ReasoningEffort::High),
            ThinkingMode::EnabledLow => Some(ReasoningEffort::Low),
            ThinkingMode::EnabledMax => Some(ReasoningEffort::Max),
            ThinkingMode::Disabled => None,
        };
    }

    /// One raw create attempt with the given thinking mode.
    async fn create_once(
        &self,
        request: &ModelRequest,
        thinking: ThinkingMode,
    ) -> Result<ModelResponse, GatewayError> {
        let client = self.client();
        let mut req = self.build_request(request);
        Self::apply_thinking(&mut req, thinking);
        // D-7: single-request wall-clock timeout (10min level) — the fork's
        // create path has no read timeout of its own (P8).
        let response =
            tokio::time::timeout(self.config.retry.request_timeout, client.chat().create(req))
                .await
                .map_err(|_| {
                    GatewayError::Timeout(format!(
                        "non-streaming create exceeded {:?}",
                        self.config.retry.request_timeout
                    ))
                })?
                .map_err(|e| self.map_error(e))?;
        self.from_response(&response)
    }

    /// Model request → typed chat completion request.
    ///
    /// D-6 (FIX_PLAN 2026-08-06 + 2026-08-20 修订): the default sends
    /// `thinking: {type: "enabled"}` + top-level `reasoning_effort: "high"`
    /// (official harness default; wire shape validated by the 2026-08-07
    /// live probe: 559ms to the first reasoning delta, ~30s to content on
    /// a hard task, per-round `usage.reasoning_tokens`). `EnabledLow` maps
    /// to `reasoning_effort: "low"` (ladder middle tier); `EnabledMax`
    /// stays the explicit optional max tier. `Disabled` keeps the P2-era
    /// mitigation (all output to `content`).
    ///
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
        // NOTE (2026-08-20 审查处理 O2): 此映射与 `apply_thinking` 保持
        // 同步——此处只负责 config 默认档（`create_once`/`stream_once`
        // 会再经 `apply_thinking` 按梯级/覆盖档覆盖双旋钮；直调
        // `build_request` 的仅测试场景）。改档位时两处须同时更新。
        let (thinking, reasoning_effort) = match self.config.thinking {
            ThinkingMode::EnabledHigh => (
                Some(async_openai::types::chat::ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                }),
                Some(ReasoningEffort::High),
            ),
            ThinkingMode::EnabledLow => (
                Some(async_openai::types::chat::ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                }),
                Some(ReasoningEffort::Low),
            ),
            ThinkingMode::EnabledMax => (
                Some(async_openai::types::chat::ThinkingConfig {
                    thinking_type: "enabled".to_string(),
                }),
                Some(ReasoningEffort::Max),
            ),
            ThinkingMode::Disabled => (
                Some(async_openai::types::chat::ThinkingConfig {
                    thinking_type: "disabled".to_string(),
                }),
                None,
            ),
        };
        CreateChatCompletionRequest {
            model: self.config.model_id.clone(),
            messages,
            tools,
            max_tokens: Some(self.config.max_tokens.min(request.max_tokens)),
            thinking,
            reasoning_effort,
            // D-6 usage observation: request the aggregate usage in the final
            // stream chunk (DeepSeek honors OpenAI-style
            // stream_options.include_usage) so reasoning_tokens are
            // observable on the streaming path too.
            stream_options: Some(ChatCompletionStreamOptions {
                include_usage: Some(true),
                include_obfuscation: None,
            }),
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
            // D-6 usage observation: reasoning_tokens + completion_tokens
            // feed budget/latency calibration (the single-round budget
            // decision rolls back on the data; S4 用真实值复核
            // REASONING_CHARS_PER_TOKEN 估算系数).
            reasoning_tokens: body
                .usage
                .as_ref()
                .and_then(|u| u.completion_tokens_details.as_ref())
                .and_then(|d| d.reasoning_tokens),
            completion_tokens: body.usage.as_ref().map(|u| u.completion_tokens),
            // Cache-hit observation (2026-08-07 fix): DeepSeek reports
            // usage.prompt_cache_hit/miss_tokens — journaled to verify the
            // prefix-cache fix (system no longer carries per-round state).
            cache_hit_tokens: body.usage.as_ref().and_then(|u| u.prompt_cache_hit_tokens),
            cache_miss_tokens: body.usage.as_ref().and_then(|u| u.prompt_cache_miss_tokens),
            // A6 (2026-08-08): the measured prompt-token total — the trigger
            // for explicit context compaction.
            prompt_tokens: body.usage.as_ref().map(|u| u.prompt_tokens as u64),
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

    /// GAP-STREAM-RETRY (2026-08-12): mark a streaming failure retryable.
    /// A zero-chunk failure — NO SSE chunk decoded (reasoning deltas
    /// included) — means the provider produced no output for this request,
    /// so re-sending the identical body is side-effect-free (idempotent;
    /// ADR-0007 §3 known-boundary premise). Once any chunk landed, output
    /// existed: re-sending could duplicate tool calls, so those failures
    /// stay plain errors (§2.1 已见输出不重试). Only wire-level failures
    /// (transport / timeout) are wrapped: model rejections (permanent, e.g.
    /// 400 — the fork's ApiError carries no HTTP status, so 429/5xx cannot
    /// be classified reliably on this path) and parse errors are never
    /// retried, and a user cancel must never be re-sent.
    fn wrap_zero_chunk(&self, saw_chunk: bool, e: GatewayError) -> GatewayError {
        if saw_chunk {
            return e;
        }
        match e {
            GatewayError::Transport(detail) | GatewayError::Timeout(detail) => {
                GatewayError::StreamInterrupted {
                    attempts: 0,
                    detail,
                }
            }
            other => other,
        }
    }

    /// GAP-STREAM-RETRY (2026-08-12): one logical stream attempt with
    /// zero-chunk interruption retry. `stream_once` is the raw single
    /// connection; this wrapper re-sends the IDENTICAL request body when the
    /// attempt failed before any chunk existed (no model output — idempotent).
    /// Retry discipline mirrors the fork's non-streaming `execute_raw`
    /// (D-7): bounded by BOTH `request_max_retries` AND the backoff's
    /// elapsed-time window (`request_retry_window`) — whichever ends first.
    /// Cancel during a backoff aborts the chain immediately (/stop must not
    /// be held hostage by a retry wait); a user cancel is never re-sent.
    async fn stream_once_with_retry(
        &self,
        request: &ModelRequest,
        thinking: ThinkingMode,
        cancel: Option<&CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        let policy = &self.config.retry;
        // Same backoff shape as the fork's client: default ExponentialBackoff
        // with the window capped at `request_retry_window` (§2.3 parameter
        // surface — no new knobs).
        let mut backoff = backoff::ExponentialBackoff {
            max_elapsed_time: Some(policy.request_retry_window),
            ..Default::default()
        };
        let mut attempts: u32 = 0;
        loop {
            match self
                .stream_once(request, thinking, cancel, heartbeat, on_chunk)
                .await
            {
                Ok(response) => return Ok(response),
                // OUTPUT-DEGENERATION-GUARD (2026-08-19) + OUTPUT-BUDGET-
                // RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35):
                // 输出健康哨兵中断绝不 zero-chunk 重试——content 族已见
                // 输出（重发会重复工具调用，ADR-0007 纪律）；reasoning 族
                // 无可见输出但长烧型原样重试大概率复现且贵（由 D-6 链直接
                // 降级，§3.2）。直接透传，attempts 保持 0。
                Err(GatewayError::StreamInterrupted { attempts, detail })
                    if is_degeneration_detail(&detail) || is_reasoning_guard_detail(&detail) =>
                {
                    tracing::warn!(
                        "stream interrupted by the output-health guard — not zero-chunk \
                         retried: {detail}"
                    );
                    return Err(GatewayError::StreamInterrupted { attempts, detail });
                }
                Err(GatewayError::StreamInterrupted { detail, .. }) => {
                    if attempts >= policy.request_max_retries {
                        tracing::warn!(
                            "stream zero-chunk interruption: retry cap reached ({attempts} retries), giving up: {detail}"
                        );
                        return Err(GatewayError::StreamInterrupted { attempts, detail });
                    }
                    let Some(delay) = backoff.next_backoff() else {
                        tracing::warn!(
                            "stream zero-chunk interruption: retry window exhausted, giving up: {detail}"
                        );
                        return Err(GatewayError::StreamInterrupted { attempts, detail });
                    };
                    tracing::warn!(
                        "stream zero-chunk interruption (attempt {} of {}): retrying in {delay:?}: {detail}",
                        attempts + 1,
                        policy.request_max_retries
                    );
                    match cancel {
                        Some(c) => tokio::select! {
                            biased;
                            _ = c.cancelled() => return Err(GatewayError::Cancelled),
                            _ = tokio::time::sleep(delay) => {}
                        },
                        None => tokio::time::sleep(delay).await,
                    }
                    // P1-1 (2026-08-08 stall guards): the backoff is quiet
                    // time the stall watchdog must not trip over.
                    if let Some(h) = heartbeat {
                        h.stamp();
                    }
                    attempts += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    async fn stream_once(
        &self,
        request: &ModelRequest,
        thinking: ThinkingMode,
        cancel: Option<&CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        // Pre-cancel check (2026-08-06 implementation review P2-2): an
        // already-cancelled run must not open a connection at all.
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(GatewayError::Cancelled);
        }
        let client = self.client();
        let mut req = self.build_request(request);
        Self::apply_thinking(&mut req, thinking);
        // F-08 (2026-08-07 review): the HTTP/TLS handshake sits OUTSIDE the
        // select! watchdog loop — a TCP connection accepted but never
        // answering would hang forever with no timeout and no cancel. Wrap
        // the handshake in the idle-timeout: no response headers within
        // that window is a dead wire (the idle watchdog's 30s semantics —
        // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20, ADR-0010
        // §14.35: 取代 STREAM-RETRY-RHYTHM 未实施的 50s; was 90s; headers
        // arrive long before the first data chunk even under slow thinking).
        let mut stream = tokio::time::timeout(
            self.config.retry.stream_idle_timeout,
            client.chat().create_stream(req),
        )
        .await
        .map_err(|_| {
            self.wrap_zero_chunk(
                false,
                GatewayError::Timeout(
                    "stream handshake hung — no response headers within the idle timeout"
                        .to_string(),
                ),
            )
        })?
        .map_err(|e| self.wrap_zero_chunk(false, self.map_error(e)))?;

        // GAP-STREAM-RETRY (2026-08-12): a stream attempt that produced NO
        // decoded chunk before failing is retryable (see `wrap_zero_chunk`).
        // Any successfully decoded SSE item counts — reasoning-only deltas
        // included — because the provider then owns output for this request.
        let mut saw_chunk = false;
        let mut text_parts: Vec<String> = Vec::new();
        // OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33): 生成期
        // 实时检测——on_chunk content delta 逐块喂入，任一阈值命中即中断
        // 流式请求（StreamInterrupted + 标记），止损于生成期而非事后。
        let mut degeneration = DegenerationDetector::default();
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
        // D-6 usage observation on the streaming path: DeepSeek emits the
        // usage summary in the FINAL stream chunk (choices empty) when
        // `stream_options.include_usage` is set; without it, usage is absent
        // and the response reports `None` (reasoning-token calibration falls
        // back to the non-streaming path / journal).
        let mut stream_usage: Option<async_openai::types::chat::CompletionUsage> = None;

        // D-7 (FIX_PLAN 2026-08-06) — stream idle watchdog + total budget:
        //  - idle_warn (5s): log once per silence stretch that the wire is
        //    quiet (STREAM-RETRY-RHYTHM 2026-08-20, was 20s; slow thinking
        //    with progress is NOT a timeout — only a dead wire is; the
        //    2026-08-07 live probe showed a max-effort stream with reasoning
        //    deltas flowing 0.5s after connect).
        //  - idle_timeout (30s): hard abort on complete silence (= 5s × 6
        //    rounds — OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20,
        //    ADR-0010 §14.35, 取代 STREAM-RETRY-RHYTHM 未实施的 50s; was
        //    90s; TCP timeouts cannot catch a hung keep-alive connection).
        //  - total budget (30min): auxiliary backstop over the whole stream.
        // All parameterized via `ModelConfig::retry`.
        let idle_warn = self.config.retry.stream_idle_warn;
        let idle_timeout = self.config.retry.stream_idle_timeout;
        let total_deadline = tokio::time::Instant::now() + self.config.retry.stream_total_timeout;
        let mut last_activity = tokio::time::Instant::now();
        let mut warned = false;

        loop {
            // Cooperative cancellation checkpoint (Phase 3 slice #11, P3-7;
            // 2026-08-06 design review D1-2): `select!` wakes the moment the
            // token fires — polling would leave `/stop` dead while the wire
            // is stalled (the fork's reqwest client has no read timeout) —
            // and returning drops the stream, whose spawned task sees the rx
            // end and closes the connection (the abort-by-drop path; the
            // provider may keep generating up to one frame — recorded as the
            // inherent boundary of cooperative cancellation). Timeout and
            // cancel stay strictly distinct (D-7): the timeout fires only on
            // wire silence, never on a user cancel.
            let item = match cancel {
                Some(c) => tokio::select! {
                    biased;
                    _ = c.cancelled() => return Err(GatewayError::Cancelled),
                    _ = tokio::time::sleep_until(last_activity + idle_timeout) => {
                        return Err(self.wrap_zero_chunk(
                            saw_chunk,
                            GatewayError::Timeout(format!(
                                "stream idle: no data for {idle_timeout:?}"
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(total_deadline) => {
                        return Err(self.wrap_zero_chunk(
                            saw_chunk,
                            GatewayError::Timeout(format!(
                                "stream total budget {:?} exceeded",
                                self.config.retry.stream_total_timeout
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(last_activity + idle_warn), if !warned => {
                        tracing::warn!(
                            "stream idle {idle_warn:?}: no data since last chunk — waiting (abort at {idle_timeout:?})"
                        );
                        warned = true;
                        continue;
                    }
                    item = stream.next() => item,
                },
                None => tokio::select! {
                    biased;
                    _ = tokio::time::sleep_until(last_activity + idle_timeout) => {
                        return Err(self.wrap_zero_chunk(
                            saw_chunk,
                            GatewayError::Timeout(format!(
                                "stream idle: no data for {idle_timeout:?}"
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(total_deadline) => {
                        return Err(self.wrap_zero_chunk(
                            saw_chunk,
                            GatewayError::Timeout(format!(
                                "stream total budget {:?} exceeded",
                                self.config.retry.stream_total_timeout
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(last_activity + idle_warn), if !warned => {
                        tracing::warn!(
                            "stream idle {idle_warn:?}: no data since last chunk — waiting (abort at {idle_timeout:?})"
                        );
                        warned = true;
                        continue;
                    }
                    item = stream.next() => item,
                },
            };
            let Some(item) = item else {
                break;
            };
            warned = false;
            last_activity = tokio::time::Instant::now();
            // P1-1 (2026-08-08 stall guards): every wire frame is activity —
            // reasoning-only deltas included (the controller never sees
            // them; only the transport observes a long max-effort thinking
            // stream, so only it can keep the heartbeat alive).
            if let Some(h) = heartbeat {
                h.stamp();
            }
            let chunk: CreateChatCompletionStreamResponse =
                item.map_err(|e| self.wrap_zero_chunk(saw_chunk, self.map_error(e)))?;
            saw_chunk = true;
            // D-6: the final chunk carries the aggregate usage (choices
            // empty, usage populated) when include_usage is honored.
            if let Some(u) = &chunk.usage {
                stream_usage = Some(u.clone());
            }
            for choice in &chunk.choices {
                apply_delta(
                    &choice.delta,
                    &mut text_parts,
                    &mut reasoning_parts,
                    &mut tool_calls,
                    &mut degeneration,
                    &mut |text| on_chunk(text),
                );
                if let Some(fr) = choice.finish_reason {
                    saw_finish_reason = true;
                    finish_reason = map_finish_reason(Some(fr));
                }
            }
            // OUTPUT-DEGENERATION-GUARD (2026-08-19) + OUTPUT-BUDGET-
            // RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35): 预算
            // 兜底层逐 chunk 检查（stall 时间/预算 OR 触发）后，命中任一
            // 检测阈值 → 主动中断。防循环计数：会话级连续退化次数达到
            // `DEGENERATION_LIMIT` 后带 `degeneration_limit_reached` 标记
            // （run 层映射 run_invalidated，计入 stagnation 同类终止态）。
            degeneration.check_stall(std::time::Instant::now());
            if let Some(detail) = degeneration.trip_reason() {
                let detail = self.degeneration_interrupt_detail(detail);
                return Err(GatewayError::StreamInterrupted {
                    attempts: 0,
                    detail,
                });
            }
        }

        // D1-1: a stream that ended without a finish_reason was truncated —
        // surfacing it as an error keeps the journal honest (no half-answer
        // recorded as a completed `stop`).
        if !saw_finish_reason {
            return Err(self.wrap_zero_chunk(
                saw_chunk,
                GatewayError::Transport("stream ended without finish_reason".to_string()),
            ));
        }

        // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010
        // §14.35): usage 到达（final chunk）时以真实 reasoning_tokens 复核
        // 字符估算——审计留痕、不改中断决策（中断发生在 usage 前）；S4
        // 用该数据校准 `REASONING_CHARS_PER_TOKEN`。
        if let Some(u) = &stream_usage
            && let Some(actual) = u
                .completion_tokens_details
                .as_ref()
                .and_then(|d| d.reasoning_tokens)
        {
            let estimate = degeneration.reasoning_est_tokens();
            tracing::debug!(
                estimate = estimate,
                actual = actual,
                delta = actual as i64 - estimate as i64,
                "reasoning token estimate vs usage (S4 calibration)"
            );
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

        // D-6 (FIX_PLAN 2026-08-06): tool rounds MUST replay
        // `reasoning_content` — DeepSeek 400s on omission, and empty string
        // is the norm (59% of tool rounds; empty-object/missing breaks the
        // replay). Empty reasoning on a tool round keeps the wire shape
        // identical to the non-streaming path (`Some("")`), which preserves
        // `""` verbatim (2026-08-07 review F-01).
        let reasoning_content = if !reasoning_parts.is_empty() {
            Some(reasoning_parts.concat())
        } else if !tool_calls.is_empty() {
            Some(String::new())
        } else {
            None
        };

        // OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33): 一次成功
        // 完成的请求重置会话级连续退化计数（「连续」= 无成功请求插入）。
        self.degeneration_consecutive.store(0, Ordering::SeqCst);

        Ok(ModelResponse {
            text,
            tool_calls,
            finish_reason,
            reasoning_content,
            // D-6 usage observation from the final stream chunk (None when
            // the provider did not honor include_usage).
            reasoning_tokens: stream_usage
                .as_ref()
                .and_then(|u| u.completion_tokens_details.as_ref())
                .and_then(|d| d.reasoning_tokens),
            completion_tokens: stream_usage.as_ref().map(|u| u.completion_tokens),
            cache_hit_tokens: stream_usage
                .as_ref()
                .and_then(|u| u.prompt_cache_hit_tokens),
            cache_miss_tokens: stream_usage
                .as_ref()
                .and_then(|u| u.prompt_cache_miss_tokens),
            // A6 (2026-08-08): measured prompt-token total from the final
            // usage chunk — the compaction trigger.
            prompt_tokens: stream_usage.as_ref().map(|u| u.prompt_tokens as u64),
        })
    }

    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33)：退化中断
    /// 的防循环计数与 detail 标记——会话级连续计数递增；达到
    /// `DEGENERATION_LIMIT` 后以 `degeneration_limit_reached` 前缀标记
    /// （run 层映射 `run_invalidated`），否则 `degeneration_detected`
    /// （run_failed 同路径）。任何成功请求经 `stream_once` 重置计数。
    fn degeneration_interrupt_detail(&self, detail: String) -> String {
        let consecutive = self.degeneration_consecutive.fetch_add(1, Ordering::SeqCst) + 1;
        if consecutive >= DEGENERATION_LIMIT {
            format!("{DEGENERATION_LIMIT_PREFIX} consecutive={consecutive} ({detail})")
        } else {
            format!("{detail} consecutive={consecutive}")
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

/// OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35):
/// 完成型空响应重试的退避（设计 §3.2/§3.5——官方 llm-retry 默认形状：
/// 500ms 起、指数增长至 10s 上限、±10% jitter；`max_elapsed_time` 只作
/// 兜底，实际由 `EMPTY_RESPONSE_MAX_RETRIES` 次数约束）。
fn empty_response_backoff() -> backoff::ExponentialBackoff {
    backoff::ExponentialBackoff {
        initial_interval: EMPTY_RESPONSE_BACKOFF_INITIAL,
        max_interval: EMPTY_RESPONSE_BACKOFF_MAX,
        randomization_factor: EMPTY_RESPONSE_BACKOFF_JITTER,
        multiplier: 2.0,
        max_elapsed_time: Some(std::time::Duration::from_secs(60)),
        ..Default::default()
    }
}

/// Next lower degradation tier for the D-6 ladder (2026-08-20 修订,
/// ADR-0010 §14.35 第 5 项 / 设计 §3.6): **high → low → disabled → 失败**
/// —— 空响应快速重试与 reasoning 族哨兵跳转共用。`EnabledMax` 是显式可选
/// 档（难题专用）：保留 S4 验证的基线行为——哨兵命中/空流链耗尽直跳
/// `Disabled`（三级梯按默认档 high 起定义；max 不额外多烧 high/low 两轮）。
fn next_degraded_thinking(thinking: ThinkingMode) -> Option<ThinkingMode> {
    match thinking {
        ThinkingMode::EnabledHigh => Some(ThinkingMode::EnabledLow),
        ThinkingMode::EnabledLow => Some(ThinkingMode::Disabled),
        ThinkingMode::EnabledMax => Some(ThinkingMode::Disabled),
        ThinkingMode::Disabled => None,
    }
}

#[async_trait]
impl ModelGateway for DeepSeekTransport {
    fn config_fingerprint(&self) -> String {
        self.config_fingerprint_impl()
    }

    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
        // D-6 empty-content retry chain: an enabled thinking tier can
        // legitimately burn the whole budget on reasoning_content and leave
        // content empty (finish=length, zero output). The chain:
        //   1. normal request (thinking per config);
        //   2. byte-identical retry ONCE (same messages, same config — the
        //      provider's reasoning allocation varies run to run);
        //   3. thinking DISABLED degraded retry once (all output routed to
        //      content);
        //   4. chain end still empty → explicit termination reason, never a
        //      silent blank.
        // Tool rounds (empty content + tool_calls) are legal and skip the
        // chain entirely.
        let first = self
            .create_once(&request, self.effective_thinking(&request))
            .await?;
        if !Self::empty_content_abnormal(&first) {
            return Ok(first);
        }
        let second = self
            .create_once(&request, self.effective_thinking(&request))
            .await?;
        if !Self::empty_content_abnormal(&second) {
            return Ok(second);
        }
        let degraded = self.create_once(&request, ThinkingMode::Disabled).await?;
        if !Self::empty_content_abnormal(&degraded) {
            return Ok(degraded);
        }
        Err(GatewayError::Model(
            "budget exhausted with zero output — thinking retry chain \
             (byte-identical retry + thinking-disabled degrade) all produced \
             empty content"
                .to_string(),
        ))
    }
    async fn generate_stream(
        &self,
        request: ModelRequest,
        cancel: Option<&CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        // D-6 empty-content retry chain on the streaming path
        // (OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20, ADR-0010
        // §14.35 / 设计 §3.2 + §3.6 —— 官方化收窄版 + 三级降级梯):
        //   1. normal request (thinking per config, default high + 256K);
        //   2. completed EMPTY response (`empty_content_abnormal`) → fast
        //      bounded retry within the current tier: same params, backoff
        //      500ms→10s + 10% jitter, ≤ `EMPTY_RESPONSE_MAX_RETRIES`
        //      (official rhythm, narrowed from 5 — 256K 下 5 次原样重试
        //      成本不可接受);
        //   3. still empty → next lower tier: **high → low → disabled**
        //      (2026-08-20 修订, ADR-0010 §14.35 第 5 项; `EnabledMax`
        //      keeps the S4-validated direct jump to disabled);
        //   4. disabled tier still empty → explicit failure, never a
        //      silent blank.
        // Reasoning-family guard interruptions (reasoning_repetition /
        // reasoning_stall) do NOT re-run the identical request
        // (长烧型空转原样重试大概率复现且贵) — they step down ONE ladder
        // tier (high → low → disabled). Content-family interruptions keep
        // the no-retry passthrough (ADR-0007 已见输出不重试). Zero-chunk
        // transport interruptions are handled inside
        // `stream_once_with_retry` and are orthogonal to this chain.
        //
        let mut thinking = self.effective_thinking(&request);
        let mut empty_retries: u32 = 0;
        let mut backoff = empty_response_backoff();
        loop {
            match self
                .stream_once_with_retry(&request, thinking, cancel, heartbeat, on_chunk)
                .await
            {
                Ok(response) if !Self::empty_content_abnormal(&response) => {
                    return Ok(response);
                }
                Ok(_empty) => {
                    // 完成型空响应。降级阶段（thinking 禁用）仍空 → 显式失败。
                    if thinking == ThinkingMode::Disabled {
                        return Err(GatewayError::Model(
                            "budget exhausted with zero output — D-6 chain \
                             (per-tier fast bounded empty retries + high→low→ \
                             disabled degrade) all produced empty content"
                                .to_string(),
                        ));
                    }
                    // 当前档位快速重试耗尽 → 逐级下降（high → low → disabled）；
                    // 换档时重试计数与退避重置（每档独立「快速 ≤2 次」节奏，
                    // 设计 §3.2/§3.6）。
                    if empty_retries >= EMPTY_RESPONSE_MAX_RETRIES {
                        let Some(next) = next_degraded_thinking(thinking) else {
                            unreachable!("enabled tier always has a next tier");
                        };
                        tracing::warn!(
                            "completed empty response after {empty_retries} retries — \
                             degrading to {next:?}"
                        );
                        thinking = next;
                        empty_retries = 0;
                        backoff = empty_response_backoff();
                        continue;
                    }
                    let Some(delay) = backoff.next_backoff() else {
                        let Some(next) = next_degraded_thinking(thinking) else {
                            unreachable!("enabled tier always has a next tier");
                        };
                        tracing::warn!(
                            "completed empty response: backoff exhausted — degrading to {next:?}"
                        );
                        thinking = next;
                        empty_retries = 0;
                        backoff = empty_response_backoff();
                        continue;
                    };
                    tracing::warn!(
                        "completed empty response (retry {} of {}) — retrying in {delay:?}",
                        empty_retries + 1,
                        EMPTY_RESPONSE_MAX_RETRIES
                    );
                    match cancel {
                        Some(c) => tokio::select! {
                            biased;
                            _ = c.cancelled() => return Err(GatewayError::Cancelled),
                            _ = tokio::time::sleep(delay) => {}
                        },
                        None => tokio::time::sleep(delay).await,
                    }
                    // P1-1 (2026-08-08 stall guards): 退避是静默时间，停滞
                    // 看门狗不得在此期间误判。
                    if let Some(h) = heartbeat {
                        h.stamp();
                    }
                    empty_retries += 1;
                }
                Err(GatewayError::StreamInterrupted { detail, .. })
                    if is_reasoning_guard_detail(&detail)
                        && matches!(
                            thinking,
                            ThinkingMode::EnabledHigh
                                | ThinkingMode::EnabledLow
                                | ThinkingMode::EnabledMax
                        ) =>
                {
                    // reasoning 族哨兵（复读/stall）→ 不原样快速重试、逐级
                    // 下降一档（设计 §3.2/§3.6；limit 前缀三族共享、达限转
                    // run_invalidated，由下方透传分支处理）。
                    let next = next_degraded_thinking(thinking)
                        .expect("reasoning guard only fires on enabled tiers");
                    tracing::warn!(
                        "reasoning-family guard interrupted the stream — skipping \
                         identical retries, degrading to {next:?}: {detail}"
                    );
                    thinking = next;
                    empty_retries = 0;
                    backoff = empty_response_backoff();
                }
                Err(e) => return Err(e),
            }
        }
    }
}

fn apply_delta(
    delta: &ChatCompletionStreamResponseDelta,
    text_parts: &mut Vec<String>,
    reasoning_parts: &mut Vec<String>,
    tool_calls: &mut Vec<(u32, StreamToolCall)>,
    degeneration: &mut DegenerationDetector,
    on_chunk: &mut dyn FnMut(&str),
) {
    if let Some(content) = &delta.content
        && !content.is_empty()
    {
        text_parts.push(content.clone());
        degeneration.feed_content(content);
        on_chunk(content);
    }
    if let Some(reasoning) = &delta.reasoning_content
        && !reasoning.is_empty()
    {
        // Reasoning deltas are joined verbatim; they are never delivered as
        // live text deltas (TUI shows answers, not chains of thought).
        reasoning_parts.push(reasoning.clone());
        degeneration.feed_reasoning(reasoning);
    }
    if let Some(chunks) = &delta.tool_calls {
        for tc in chunks {
            apply_tool_call_chunk(tc, tool_calls, degeneration);
        }
    }
}

fn apply_tool_call_chunk(
    chunk: &ChatCompletionMessageToolCallChunk,
    tool_calls: &mut Vec<(u32, StreamToolCall)>,
    degeneration: &mut DegenerationDetector,
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
        if let Some(arguments) = &function.arguments
            && !arguments.is_empty()
        {
            degeneration.feed_tool_arguments(arguments);
        }
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
    use crate::gateway::model::{Message, RetryPolicy, Role};
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
            thinking: None,
        }
    }

    /// ORZ_MAIN_AGENT_MODEL env override for evaluation/model-swap runs
    /// (2026-08-11 TB B 组 flash→pro 对照) — the constant remains the
    /// production default; the env wins only when set. Env writes are
    /// serialized behind a static lock (orz-host TESTS_ENV_LOCK precedent)
    /// so parallel tests cannot race set_var.
    #[test]
    fn main_agent_model_env_override() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();
        let original = std::env::var("ORZ_MAIN_AGENT_MODEL").ok();
        // Rust 2024 edition: env mutation is unsafe — serialized by the
        // static lock above (single-threaded env access in this test).
        unsafe {
            std::env::remove_var("ORZ_MAIN_AGENT_MODEL");
            assert_eq!(main_agent_model(), MAIN_AGENT_MODEL);
            std::env::set_var("ORZ_MAIN_AGENT_MODEL", "deepseek-v4-pro");
            assert_eq!(main_agent_model(), "deepseek-v4-pro");
            match original {
                Some(v) => std::env::set_var("ORZ_MAIN_AGENT_MODEL", v),
                None => std::env::remove_var("ORZ_MAIN_AGENT_MODEL"),
            }
        }
    }

    #[test]
    fn build_request_default_thinking_is_enabled_high() {
        // D-6 (FIX_PLAN 2026-08-06) — 2026-08-20 修订 (ADR-0010 §14.35
        // 第 5 项 / 设计 §3.6): the DEFAULT workpoint is thinking enabled +
        // reasoning_effort "high" (official harness default) + 256K
        // single-round budget. The 256K value is the whole-token
        // single-round cap; the stream requests include_usage so reasoning
        // tokens are observable on the streaming path.
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let mut req = request();
        // The request-level max_tokens is the min-cap over the config's 256K
        // (OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20, ADR-0010
        // §14.35) — a request asking for MORE than the config cap gets the
        // cap, a request asking for less (e.g. gate rounds at 1024) gets
        // less.
        req.max_tokens = 300_000;
        let json = serde_json::to_value(t.build_request(&req)).unwrap();
        let s = json.to_string();
        assert_eq!(json["thinking"]["type"], "enabled", "{s}");
        assert_eq!(json["reasoning_effort"], "high", "{s}");
        assert_eq!(
            json["max_tokens"], 256_000,
            "config 256K caps the request-level budget: {s}"
        );
        assert_eq!(
            json["stream_options"]["include_usage"], true,
            "include_usage requested for reasoning_tokens observation: {s}"
        );
        assert_eq!(json["model"], "deepseek-v4-flash");
    }

    #[test]
    fn build_request_explicit_thinking_tiers_map_reasoning_effort() {
        // 2026-08-20 修订 (ADR-0010 §14.35 第 5 项 / 设计 §3.6):
        // `EnabledMax` 保留为显式可选档（max）、`EnabledLow` 为降级梯中间
        // 档（low）；二者均为 thinking enabled + 对应 effort。
        let transport_for = |thinking: ThinkingMode| DeepSeekTransport {
            config: ModelConfig {
                thinking,
                ..DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash").config
            },
            degeneration_consecutive: Arc::new(AtomicU32::new(0)),
        };
        let max_json =
            serde_json::to_value(transport_for(ThinkingMode::EnabledMax).build_request(&request()))
                .unwrap();
        assert_eq!(max_json["thinking"]["type"], "enabled");
        assert_eq!(max_json["reasoning_effort"], "max", "{max_json}");
        let low_json =
            serde_json::to_value(transport_for(ThinkingMode::EnabledLow).build_request(&request()))
                .unwrap();
        assert_eq!(low_json["thinking"]["type"], "enabled");
        assert_eq!(low_json["reasoning_effort"], "low", "{low_json}");
        let high_json = serde_json::to_value(
            transport_for(ThinkingMode::EnabledHigh).build_request(&request()),
        )
        .unwrap();
        assert_eq!(high_json["reasoning_effort"], "high", "{high_json}");
    }

    #[test]
    fn build_request_thinking_disabled_routes_all_output_to_content() {
        // The P2-era mitigation stays reachable: `ThinkingMode::Disabled`
        // sends the explicit disable (all output to `content`, no reasoning
        // knob) — kept for parity/tests/benchmarks that want the fast path.
        let t = DeepSeekTransport {
            config: ModelConfig {
                thinking: ThinkingMode::Disabled,
                ..DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash").config
            },
            degeneration_consecutive: Arc::new(AtomicU32::new(0)),
        };
        let req = t.build_request(&request());
        let json = serde_json::to_value(&req).unwrap();
        let s = json.to_string();
        assert_eq!(json["thinking"]["type"], "disabled", "{s}");
        assert!(!s.contains("reasoning_effort"), "{s}");
    }

    // ── OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33) ────────────

    #[test]
    fn degeneration_detector_trips_on_five_identical_deltas() {
        let mut d = DegenerationDetector::default();
        for _ in 0..4 {
            d.feed_content("same");
            assert!(d.trip_reason().is_none());
        }
        d.feed_content("same");
        let reason = d.trip_reason().expect("5 identical deltas must trip");
        assert!(reason.starts_with(DEGENERATION_DETAIL_PREFIX), "{reason}");
        assert!(reason.contains("identical content deltas"), "{reason}");
    }

    #[test]
    fn degeneration_detector_no_trip_below_thresholds() {
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_content(&format!("distinct {i} token"));
            assert!(
                d.trip_reason().is_none(),
                "short varied output must not trip"
            );
        }
    }

    #[test]
    fn degeneration_detector_trips_on_high_repetition_ratio() {
        // >1K tokens with a heavily duplicated 3-gram profile; alternating
        // two phrase spellings keeps the consecutive-delta check silent so
        // the repetition-ratio path is the one under test.
        let a = "elided middle see archived log for full content";
        let b = "see archived log for full content elided middle";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_content(if i % 2 == 0 { a } else { b });
        }
        let reason = d.trip_reason().expect("high repetition must trip");
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
    }

    #[test]
    fn degeneration_consecutive_counter_reaches_limit_then_resets() {
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        for i in 1..=2 {
            let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
            assert!(detail.starts_with(DEGENERATION_DETAIL_PREFIX), "{detail}");
            assert!(detail.contains(&format!("consecutive={i}")), "{detail}");
        }
        let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
        assert!(detail.starts_with(DEGENERATION_LIMIT_PREFIX), "{detail}");
        // A successful request resets the consecutive counter (design §3.3:
        // 「连续」= 无成功请求插入).
        t.degeneration_consecutive.store(0, Ordering::SeqCst);
        let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
        assert!(detail.starts_with(DEGENERATION_DETAIL_PREFIX), "{detail}");
        assert!(detail.contains("consecutive=1"), "{detail}");
    }

    /// SSE body with `n` identical content-delta frames (degeneration probe).
    fn degenerate_sse_body(delta: &str, n: usize) -> String {
        let frame = format!(
            r#"{{"id":"x","object":"chat.completion.chunk","created":0,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{delta}"}},"finish_reason":null}}]}}"#
        );
        let mut body = String::new();
        for _ in 0..n {
            body.push_str(&format!("data: {frame}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        body
    }

    #[tokio::test]
    async fn generate_stream_degeneration_interrupts_without_retry() {
        // Design §3.3: the detector trips mid-generation → the stream is
        // interrupted with `degeneration_detected`; the interruption is NOT
        // retried (output already seen — ADR-0007 discipline).
        let body = degenerate_sse_body("hi", 5);
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { attempts: 0, detail } if detail.starts_with(DEGENERATION_DETAIL_PREFIX)),
            "degeneration must surface StreamInterrupted(attempts=0) with the marker, got {err:?}"
        );
        assert_eq!(
            chunks,
            vec!["hi".to_string(); 5],
            "all deltas up to the trip must reach the journal"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 1,
            "degeneration must never be retried (output already seen): {n} connections"
        );
    }

    // ── OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35
    // / 设计 §3.2/§3.3) ─────────────────────────────────────────────────────

    /// SSE body: a COMPLETED stream with zero output (finish=length, no
    /// content/reasoning/tool_calls) — the D-6 empty-response case.
    fn empty_completed_sse_body() -> String {
        "data: {\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"length\"}]}\n\ndata: [DONE]\n\n".to_string()
    }

    /// SSE body with `n` identical reasoning-only delta frames (reasoning
    /// repetition probe — content/tool_calls stay empty).
    fn reasoning_repetition_sse_body(delta: &str, n: usize) -> String {
        let frame = format!(
            r#"{{"id":"x","object":"chat.completion.chunk","created":0,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","reasoning_content":"{delta}"}},"finish_reason":null}}]}}"#
        );
        let mut body = String::new();
        for _ in 0..n {
            body.push_str(&format!("data: {frame}\n\n"));
        }
        body.push_str("data: [DONE]\n\n");
        body
    }

    /// SSE body with one large low-repetition reasoning delta (~≥64K
    /// estimated tokens — the reasoning-stall token-budget probe; distinct
    /// tokens keep the 3-gram repetition detector silent so the stall
    /// budget is the signal under test).
    fn reasoning_stall_sse_body() -> String {
        let mut reasoning = String::new();
        for i in 0..16_000 {
            reasoning.push_str(&format!("token{i} "));
        }
        let frame = format!(
            r#"{{"id":"x","object":"chat.completion.chunk","created":0,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","reasoning_content":"{reasoning}"}},"finish_reason":null}}]}}"#
        );
        format!("data: {frame}\n\ndata: [DONE]\n\n")
    }

    #[test]
    fn detector_reasoning_repetition_trips_on_five_identical_deltas() {
        // 灵敏层（设计 §3.3）：同一算法作用于 reasoning delta——连续相同
        // N=5 → reasoning_repetition；循环型空转可在数 K 内识别，不依赖
        // 大预算兜底。
        let mut d = DegenerationDetector::default();
        for _ in 0..4 {
            d.feed_reasoning("think");
            assert!(d.trip_reason().is_none());
        }
        d.feed_reasoning("think");
        let reason = d
            .trip_reason()
            .expect("5 identical reasoning deltas must trip");
        assert!(
            reason.starts_with(REASONING_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("identical reasoning deltas"), "{reason}");
    }

    #[test]
    fn detector_reasoning_repetition_trips_on_high_ratio() {
        // 与 content 复读同一 1K 窗口 3-gram 重复率算法（>60% 触发）。
        let a = "elided middle see archived log for full content";
        let b = "see archived log for full content elided middle";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_reasoning(if i % 2 == 0 { a } else { b });
        }
        let reason = d
            .trip_reason()
            .expect("high reasoning repetition must trip");
        assert!(
            reason.starts_with(REASONING_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
    }

    #[test]
    fn detector_reasoning_guards_stand_down_after_visible_output() {
        // 设计 §3.3：reasoning 族信号仅 content/tool_calls 全空时启用——
        // content 出现后复读与 stall 均不得再触发。
        let mut d = DegenerationDetector::default();
        d.feed_content("answer");
        for _ in 0..5 {
            d.feed_reasoning("think");
        }
        assert!(
            d.trip_reason().is_none(),
            "reasoning repetition must be disabled once content is seen"
        );
        d.reasoning_chars = STALL_REASONING_BUDGET_TOKENS * REASONING_CHARS_PER_TOKEN;
        d.first_chunk_at = Some(
            std::time::Instant::now()
                - STALL_FIRST_CONTENT_TIMEOUT
                - std::time::Duration::from_secs(1),
        );
        d.check_stall(std::time::Instant::now());
        assert!(
            d.trip_reason().is_none(),
            "stall must be disabled once content is seen"
        );
    }

    #[test]
    fn detector_tool_arguments_stand_down_reasoning_guards() {
        // tool_call arguments 进观测面——arguments 出现即 tool_calls 非空
        // （可见输出成型），reasoning 族信号停用（工具轮为合法形态）。
        let mut d = DegenerationDetector::default();
        d.feed_tool_arguments("{\"path\":\"a.txt\"}");
        for _ in 0..5 {
            d.feed_reasoning("think");
        }
        assert!(
            d.trip_reason().is_none(),
            "tool args are visible output — reasoning guards off"
        );
        d.reasoning_chars = STALL_REASONING_BUDGET_TOKENS * REASONING_CHARS_PER_TOKEN;
        d.first_chunk_at = Some(
            std::time::Instant::now()
                - STALL_FIRST_CONTENT_TIMEOUT
                - std::time::Duration::from_secs(1),
        );
        d.check_stall(std::time::Instant::now());
        assert!(
            d.trip_reason().is_none(),
            "stall must be disabled once tool args are seen"
        );
    }

    #[test]
    fn detector_stall_trips_on_time_budget() {
        // 预算兜底层时间信号（设计 §3.3）：自首 chunk 起 600s 无
        // content/tool_calls 且 reasoning 在流动 → reasoning_stall。
        let mut d = DegenerationDetector::default();
        d.feed_reasoning("some varied reasoning text");
        d.first_chunk_at = Some(
            std::time::Instant::now()
                - STALL_FIRST_CONTENT_TIMEOUT
                - std::time::Duration::from_secs(1),
        );
        d.check_stall(std::time::Instant::now());
        let reason = d.trip_reason().expect("600s time budget must trip");
        assert!(
            reason.starts_with(REASONING_STALL_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("no content/tool_calls"), "{reason}");
    }

    #[test]
    fn detector_stall_trips_on_token_budget() {
        // 预算兜底层 token 信号（设计 §3.3）：reasoning 估算累计 ≥64K 仍无
        // content/tool_calls → reasoning_stall（OR 语义的另一支）。
        let mut d = DegenerationDetector::default();
        d.feed_reasoning("distinct reasoning token sequence here");
        d.reasoning_chars = STALL_REASONING_BUDGET_TOKENS * REASONING_CHARS_PER_TOKEN;
        d.check_stall(std::time::Instant::now());
        let reason = d.trip_reason().expect("64K token budget must trip");
        assert!(
            reason.starts_with(REASONING_STALL_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("estimated reasoning tokens"), "{reason}");
    }

    #[test]
    fn detector_stall_does_not_trip_without_first_chunk() {
        // 完全无 chunk 时 stall 不适用——静默由 idle 死线（30s）处理，
        // stall 与 idle 互补不重叠（设计 §3.4）。
        let mut d = DegenerationDetector::default();
        d.check_stall(std::time::Instant::now());
        assert!(
            d.trip_reason().is_none(),
            "no first chunk → idle handles silence"
        );
    }

    #[test]
    fn detector_reasoning_estimate_uses_chars_per_token() {
        // 估算校准（设计 §3.3）：字符 ÷ REASONING_CHARS_PER_TOKEN（=2，
        // 桥 8K 实测校准值；S4 用 usage 真实值复核）。
        let mut d = DegenerationDetector::default();
        d.feed_reasoning("abc");
        assert_eq!(d.reasoning_est_tokens(), 1);
        d.feed_reasoning("abcde");
        assert_eq!(d.reasoning_est_tokens(), 4); // 8 chars / 2
        assert_eq!(REASONING_CHARS_PER_TOKEN, 2);
    }

    #[test]
    fn stall_budget_decoupled_from_max_tokens() {
        // 设计 §3.3：空转预算与 max_tokens 解耦——256K 输出预算恢复不放大
        // 空转兜底；常量钉死（S4 校准 32–128K）。
        assert_eq!(STALL_REASONING_BUDGET_TOKENS, 64_000);
        assert_eq!(crate::agent_loop::REQUEST_MAX_TOKENS, 256_000);
        assert!(STALL_REASONING_BUDGET_TOKENS < crate::agent_loop::REQUEST_MAX_TOKENS as usize);
    }

    #[test]
    fn empty_response_retry_parameters_match_design() {
        // 设计 §3.2/§3.5：快速有界重试 ≤2 次、官方退避形状 500ms→10s +
        // 10% jitter。
        assert_eq!(EMPTY_RESPONSE_MAX_RETRIES, 2);
        let b = empty_response_backoff();
        assert_eq!(b.initial_interval, EMPTY_RESPONSE_BACKOFF_INITIAL);
        assert_eq!(b.max_interval, EMPTY_RESPONSE_BACKOFF_MAX);
        assert_eq!(b.randomization_factor, EMPTY_RESPONSE_BACKOFF_JITTER);
    }

    #[tokio::test]
    async fn generate_stream_empty_content_retries_twice_then_degrades() {
        // 设计 §3.2：完成型空响应 → 快速有界重试 ≤2 次（同参 max）→
        // thinking 禁用降级；降级轮产出答案。`EnabledMax` 显式档保留 S4
        // 基线：空流链耗尽直跳 disabled（三级梯按默认 high 起定义）。
        let empty = empty_completed_sse_body();
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < 3 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"max\""),
                    "attempts 1-3 use the max config: {body}"
                );
                MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"disabled\"") && !body.contains("reasoning_effort"),
                    "degraded attempt drops reasoning: {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(chunks, vec!["降级答案".to_string()]);
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "1 normal + 2 empty retries + 1 degraded"
        );
    }

    #[tokio::test]
    async fn generate_stream_empty_content_chain_end_errors_explicitly() {
        // 设计 §3.2：链尾（快速重试 2 次 + 降级全空）→ 显式失败，绝不静默
        // 空答案。
        let empty = empty_completed_sse_body();
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Model(m) if m.contains("zero output")),
            "chain end must error explicitly: {err:?}"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_repetition_jumps_to_degrade() {
        // 设计 §3.2/§3.3：max 阶段触发 reasoning 复读哨兵 → 不原样快速
        // 重试（长烧型原样重试大概率复现且贵）、直接跳降级（max 显式档
        // 保留 S4 基线直跳 disabled）。
        let rep = reasoning_repetition_sse_body("think", 5);
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                assert!(
                    body.contains("\"type\":\"enabled\""),
                    "first attempt uses the max config: {body}"
                );
                MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"disabled\"") && !body.contains("reasoning_effort"),
                    "degraded attempt drops reasoning: {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let r = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "reasoning guard must skip identical retries and degrade directly"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_stall_jumps_to_degrade() {
        // 设计 §3.3 预算兜底：reasoning 估算 ≥64K 仍无 content/tool_calls
        // → reasoning_stall → 跳过原样重试、直接降级（max 显式档直跳
        // disabled 基线）。
        let stall = reasoning_stall_sse_body();
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                MockResponse::sse(vec![&stall], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"disabled\"") && !body.contains("reasoning_effort"),
                    "degraded attempt drops reasoning: {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let r = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "stall must skip identical retries and degrade directly"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_guard_during_empty_retry_jumps_to_degrade() {
        // 设计 §3.2：阶段 1 完成型空响应进入快速重试；阶段 2（重试中）触发
        // reasoning 族哨兵 → 跳过剩余原样重试、直接降级（总 3 次而非 4 次）。
        let empty = empty_completed_sse_body();
        let rep = reasoning_repetition_sse_body("think", 5);
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match n {
                0 => MockResponse::sse(vec![&empty], std::time::Duration::ZERO),
                1 => MockResponse::sse(vec![&rep], std::time::Duration::ZERO),
                _ => {
                    assert!(
                        body.contains("\"type\":\"disabled\"")
                            && !body.contains("reasoning_effort"),
                        "degraded attempt drops reasoning: {body}"
                    );
                    MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
                }
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let r = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            3,
            "empty → reasoning guard on retry → degrade (no 3rd identical retry)"
        );
    }

    // ── 2026-08-20 修订: 三级降级梯（high → low → disabled → 失败）─────

    #[tokio::test]
    async fn generate_stream_empty_content_ladder_high_low_disabled_fails() {
        // 设计 §3.6：默认 high 档完成型空响应 → 每档快速重试 ≤2 次 →
        // high → low → disabled → 显式失败（链尾绝不静默空答案）。换档时
        // 重试计数与退避重置——每档独立「快速 ≤2 次」节奏（0.5s→1s×2 档）。
        let empty = empty_completed_sse_body();
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match n {
                0..=2 => {
                    assert!(
                        body.contains("\"type\":\"enabled\"") && body.contains("\"high\""),
                        "attempts 1-3 use the high tier: {body}"
                    );
                    MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
                }
                3..=5 => {
                    assert!(
                        body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                        "attempts 4-6 use the low tier: {body}"
                    );
                    MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
                }
                _ => {
                    assert!(
                        body.contains("\"type\":\"disabled\"")
                            && !body.contains("reasoning_effort"),
                        "degraded attempt drops reasoning: {body}"
                    );
                    MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
                }
            }
        })
        .await;
        let t = mock_transport_with_retry_thinking(
            &base,
            Default::default(),
            ThinkingMode::EnabledHigh,
        );
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Model(m) if m.contains("zero output")),
            "ladder end must error explicitly: {err:?}"
        );
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            7,
            "3 high + 3 low + 1 disabled"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_guard_ladder_high_to_low() {
        // 设计 §3.6：high 档 reasoning 复读哨兵 → 不原样重试、下降一档到
        // low（保留浅思考链）；low 档产出答案。
        let rep = reasoning_repetition_sse_body("think", 5);
        let ok = ok_sse_body("low 答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"high\""),
                    "first attempt uses the high tier: {body}"
                );
                MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                    "second attempt steps down one tier to low: {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t = mock_transport_with_retry_thinking(
            &base,
            Default::default(),
            ThinkingMode::EnabledHigh,
        );
        let r = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("low 答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "reasoning guard must step down exactly one tier (high → low)"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_guard_ladder_low_to_disabled() {
        // 设计 §3.6：low 档 reasoning 复读哨兵 → 下降一档到 disabled；
        // disabled 档产出答案（全部输出走 content，无 reasoning 旋钮）。
        let rep = reasoning_repetition_sse_body("think", 5);
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                    "first attempt uses the low tier: {body}"
                );
                MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"disabled\"") && !body.contains("reasoning_effort"),
                    "degraded attempt drops reasoning: {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledLow);
        let r = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "reasoning guard at the low tier must step down to disabled"
        );
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
            thinking: None,
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
            thinking: None,
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
            thinking: None,
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
            thinking: None,
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
        let mut degeneration = DegenerationDetector::default();

        apply_delta(
            &delta(Some("Hel"), Some("think-"), None),
            &mut text_parts,
            &mut reasoning_parts,
            &mut tool_calls,
            &mut degeneration,
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
            &mut degeneration,
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
            &mut degeneration,
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
            &mut degeneration,
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
        /// Sleep before writing ANY bytes (simulates a stalled upstream —
        /// timeout/watchdog tests). The client's `request_timeout` /
        /// `stream_idle_timeout` must be shorter than this for the test to
        /// observe a timeout.
        pre_delay: std::time::Duration,
        /// GAP-STREAM-RETRY: write the response head claiming the FULL
        /// content-length, deliver only the first half of the body, then
        /// close — the client's read errors with the exact TB job1
        /// signature (`error decoding response body`).
        write_truncated: bool,
    }

    impl MockResponse {
        fn json(status: u16, body: impl Into<String>) -> Self {
            Self {
                status,
                content_type: "application/json",
                body: body.into(),
                frame_delay: std::time::Duration::ZERO,
                pre_delay: std::time::Duration::ZERO,
                write_truncated: false,
            }
        }

        fn json_delayed(
            status: u16,
            body: impl Into<String>,
            pre_delay: std::time::Duration,
        ) -> Self {
            Self {
                pre_delay,
                ..Self::json(status, body)
            }
        }

        fn sse(frames: Vec<&str>, frame_delay: std::time::Duration) -> Self {
            Self {
                status: 200,
                content_type: "text/event-stream",
                body: frames.join(""),
                frame_delay,
                pre_delay: std::time::Duration::ZERO,
                write_truncated: false,
            }
        }

        /// A stalled upstream (handshake-hang tests): headers are withheld
        /// for `pre_delay`, then the response is written as-is.
        fn sse_delayed(frames: Vec<&str>, pre_delay: std::time::Duration) -> Self {
            Self {
                pre_delay,
                ..Self::sse(frames, std::time::Duration::ZERO)
            }
        }

        /// GAP-STREAM-RETRY: deliver only the first half of the body, then
        /// close — the client's read errors with the exact TB job1 signature
        /// (`error decoding response body`) before any chunk lands.
        fn truncated(mut self) -> Self {
            self.write_truncated = true;
            self
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
        // Simulate a stalled upstream: sleep before writing anything, so the
        // client-side timeout/watchdog fires first.
        if !response.pre_delay.is_zero() {
            tokio::time::sleep(response.pre_delay).await;
        }
        let head = format!(
            "HTTP/1.1 {} OK\r\ncontent-type: {}\r\nconnection: close\r\ncontent-length: {}\r\n\r\n",
            response.status,
            response.content_type,
            response.body.len()
        );
        socket.write_all(head.as_bytes()).await?;
        if response.frame_delay.is_zero() {
            if response.write_truncated {
                // Content-length claims the full body; deliver a FIXED 100
                // bytes and close — a mid-frame wire drop (the client sees
                // `error decoding response body`). The cut is decoupled from
                // the frame layout (2026-08-12 review P2-2): the smallest
                // SSE frame is ~150 bytes, so 100 always splits the first
                // frame; a `len()/2` cut would flip to "one full frame
                // delivered" if a test body shrank.
                let cut = 100usize.min(response.body.len());
                socket.write_all(&response.body.as_bytes()[..cut]).await?;
            } else {
                socket.write_all(response.body.as_bytes()).await?;
            }
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
        mock_transport_with_retry(base_url, Default::default())
    }

    /// A transport pointed at the mock server with an explicit retry policy
    /// (timeout/watchdog tests need short budgets).
    fn mock_transport_with_retry(base_url: &str, retry: RetryPolicy) -> DeepSeekTransport {
        mock_transport_with_retry_thinking(base_url, retry, ThinkingMode::Disabled)
    }

    fn mock_transport_with_retry_thinking(
        base_url: &str,
        retry: RetryPolicy,
        thinking: ThinkingMode,
    ) -> DeepSeekTransport {
        DeepSeekTransport {
            config: ModelConfig {
                provider: "deepseek".to_string(),
                model_id: "deepseek-v4-flash".to_string(),
                api_base: base_url.to_string(),
                api_key: "sk-test".to_string(),
                max_tokens: 4096,
                retry,
                thinking,
            },
            degeneration_consecutive: Arc::new(AtomicU32::new(0)),
        }
    }

    #[tokio::test]
    async fn generate_real_http_roundtrip() {
        let base = spawn_mock(|_line, body| {
            assert!(body.contains("\"stream\":false") || !body.contains("\"stream\":true"));
            // D-6 Disabled mode on the wire — the mock transport is built
            // with `ThinkingMode::Disabled` (all output to `content`), so
            // the wire must carry the explicit disable + NO reasoning knob.
            assert!(
                body.contains("\"thinking\":{\"type\":\"disabled\"}"),
                "explicit thinking disable on the wire: {body}"
            );
            assert!(
                !body.contains("reasoning_effort"),
                "Disabled mode must not set reasoning_effort: {body}"
            );
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

    // ── D-6 empty-content retry chain ─────────────────────────────────────

    fn empty_finish_length_response() -> String {
        // finish=length with NO content — the abnormal zero-output case
        // (thinking budget ate everything).
        r#"{"id":"x","object":"chat.completion","created":0,"model":"deepseek-v4-flash","choices":[{"index":0,"message":{"role":"assistant","content":null},"finish_reason":"length"}],"usage":{"prompt_tokens":10,"completion_tokens":400,"completion_tokens_details":{"reasoning_tokens":395},"total_tokens":410}}"#
            .to_string()
    }

    #[tokio::test]
    async fn generate_empty_content_retries_then_degrades() {
        // D-6: abnormal empty content (finish=length, zero output) retries
        // byte-identically, then degrades to thinking-disabled; the chain
        // yields the degraded response instead of erroring.
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < 2 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"max\""),
                    "attempts 1-2 use the max config: {body}"
                );
                MockResponse::json(200, empty_finish_length_response())
            } else {
                assert!(
                    body.contains("\"type\":\"disabled\"") && !body.contains("reasoning_effort"),
                    "degraded attempt drops reasoning: {body}"
                );
                MockResponse::json(
                    200,
                    r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"降级答案"},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#,
                )
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let r = t.generate(request()).await.unwrap();
        assert_eq!(r.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            3,
            "1 normal + 1 byte-identical retry + 1 degraded"
        );
    }

    #[tokio::test]
    async fn generate_empty_content_chain_end_errors_explicitly() {
        // D-6: the chain end (all three attempts empty) surfaces an explicit
        // termination reason — never a silent blank response.
        let base =
            spawn_mock(|_line, _body| MockResponse::json(200, empty_finish_length_response()))
                .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let err = t.generate(request()).await.unwrap_err();
        assert!(
            matches!(&err, GatewayError::Model(m) if m.contains("zero output")),
            "chain end must error explicitly: {err:?}"
        );
    }

    #[tokio::test]
    async fn generate_tool_round_empty_content_is_not_retried() {
        // D-6: a tool round (empty content + tool_calls) is LEGAL — the
        // chain must not treat it as abnormal.
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::json(
                200,
                r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":null,"tool_calls":[{"id":"call-1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"a.txt\"}"}}]},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}}"#,
            )
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let r = t.generate(request()).await.unwrap();
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "read_file");
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "tool rounds are legal — no retry"
        );
    }

    #[tokio::test]
    async fn generate_extracts_reasoning_tokens_from_usage() {
        // D-6 usage observation: reasoning_tokens come from
        // usage.completion_tokens_details.reasoning_tokens on every round.
        let base =
            spawn_mock(|_line, _body| MockResponse::json(200, empty_finish_length_response()))
                .await;
        let t = mock_transport(&base);
        let r = t.generate(request()).await.unwrap_err();
        // The chain retried and errored — but we assert the extraction on a
        // direct from_response instead (deterministic).
        let _ = r;
        let parsed = t
            .from_response(&serde_json::from_str(&empty_finish_length_response()).unwrap())
            .unwrap();
        assert_eq!(parsed.reasoning_tokens, Some(395));
        assert_eq!(parsed.completion_tokens, Some(400));
        // A6 (2026-08-08): the measured prompt-token total — the explicit
        // compaction trigger (usage.prompt_tokens).
        assert_eq!(parsed.prompt_tokens, Some(10));
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
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
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
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
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
    async fn generate_stream_tool_round_preserves_empty_reasoning() {
        // D-6 (2026-08-07 review F-01): the provider emits ONLY empty
        // `reasoning_content` deltas on a tool round (the 59% norm). The
        // replay message must carry `Some("")` — folding it to None would
        // omit the field and DeepSeek 400s on the next request. This locks
        // the streaming path to the non-streaming path's `Some("")` shape.
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
            frame(
                r#"{"role":"assistant","reasoning_content":"","tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"read_file","arguments":"{}"}}]}"#,
                None
            ),
            frame(r#"{"reasoning_content":""}"#, None),
            frame(r#"{}"#, Some("tool_calls")),
        );
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport(&base);
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert!(chunks.is_empty(), "tool-call round: no text chunks");
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].call_id, "call-1");
        assert_eq!(r.reasoning_content, Some(String::new()));
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
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
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
            .generate_stream(request(), Some(&cancel), None, &mut |c| {
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
            .generate_stream(request(), Some(&cancel), None, &mut |c| {
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

    // ── D-7 timeout / retry discipline (FIX_PLAN 2026-08-06, P1/LOOP-16) ───

    fn short_retry_policy() -> RetryPolicy {
        RetryPolicy {
            request_max_retries: 10,
            request_retry_window: std::time::Duration::from_secs(32),
            // NOTE: `request_timeout` wraps the WHOLE create call including
            // its retries — for retry-cap tests it must exceed the backoff
            // accumulation, or the timeout fires first (observed 2026-08-07).
            request_timeout: std::time::Duration::from_secs(60),
            stream_idle_warn: std::time::Duration::from_millis(50),
            stream_idle_timeout: std::time::Duration::from_millis(150),
            stream_total_timeout: std::time::Duration::from_millis(500),
        }
    }

    /// Timeout-focused policy: tiny budgets so the watchdog fires fast.
    fn short_timeout_policy() -> RetryPolicy {
        RetryPolicy {
            request_max_retries: 10,
            request_retry_window: std::time::Duration::from_secs(32),
            request_timeout: std::time::Duration::from_millis(300),
            stream_idle_warn: std::time::Duration::from_millis(50),
            stream_idle_timeout: std::time::Duration::from_millis(150),
            stream_total_timeout: std::time::Duration::from_millis(500),
        }
    }

    /// STREAM-RETRY-RHYTHM (2026-08-20, ADR-0010 §14.34) + OUTPUT-BUDGET-
    /// RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35): retry params
    /// take part in the request-header fingerprint — a value change is a
    /// real header change (cache-miss attribution). The default digest must
    /// equal the digest built from the explicit new values (5s / 30s / 50s)
    /// and differ from the legacy digest (20s / 90s / 32s).
    #[test]
    fn config_fingerprint_reflects_stream_retry_rhythm_defaults() {
        let fingerprint_for = |retry: RetryPolicy| {
            let mut cfg = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash").config;
            cfg.retry = retry;
            DeepSeekTransport::new(cfg).config_fingerprint()
        };

        let new_values = RetryPolicy::default();
        let legacy = RetryPolicy {
            request_retry_window: std::time::Duration::from_secs(32),
            stream_idle_warn: std::time::Duration::from_secs(20),
            stream_idle_timeout: std::time::Duration::from_secs(90),
            ..new_values.clone()
        };

        let default_fp = fingerprint_for(new_values.clone());
        let explicit_new_fp = fingerprint_for(new_values);
        let legacy_fp = fingerprint_for(legacy);

        assert_eq!(
            default_fp, explicit_new_fp,
            "default must carry the new rhythm values into the digest"
        );
        assert_ne!(
            default_fp, legacy_fp,
            "legacy 20s/90s/32s values must produce a different digest"
        );
    }

    #[test]
    fn config_fingerprint_reflects_thinking_tier() {
        // 2026-08-20 审查处理 O4（设计 §3.6）：「请求头指纹含 thinking 档 →
        // 部署后首次请求一次性变化」补断言——默认档（EnabledHigh）指纹与
        // 显式 EnabledHigh 一致，且与 EnabledLow / EnabledMax / Disabled
        // 各档互不相同（指纹为 payload_hash，档位值变化即摘要变化）。
        let fingerprint_for = |thinking: ThinkingMode| {
            let mut cfg = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash").config;
            cfg.thinking = thinking;
            DeepSeekTransport::new(cfg).config_fingerprint()
        };
        let default_fp = fingerprint_for(ThinkingMode::default());
        let high_fp = fingerprint_for(ThinkingMode::EnabledHigh);
        assert_eq!(
            default_fp, high_fp,
            "default must carry enabled_high into the digest"
        );
        for tier in [
            ThinkingMode::EnabledLow,
            ThinkingMode::EnabledMax,
            ThinkingMode::Disabled,
        ] {
            assert_ne!(
                default_fp,
                fingerprint_for(tier),
                "a thinking-tier change must alter the digest"
            );
        }
    }

    #[tokio::test]
    async fn generate_non_streaming_request_timeout() {
        // D-7: the non-streaming path must not block forever on a stalled
        // upstream — `request_timeout` fires and surfaces as `Timeout`
        // (strictly distinct from `Cancelled`).
        let base = spawn_mock(|_line, _body| {
            MockResponse::json_delayed(
                200,
                r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"late"},"finish_reason":"stop"}]}"#,
                std::time::Duration::from_secs(2),
            )
        })
        .await;
        let t = mock_transport_with_retry(&base, short_timeout_policy());
        let err = t.generate(request()).await.unwrap_err();
        assert!(
            matches!(&err, GatewayError::Timeout(m) if m.contains("exceeded")),
            "stalled create must time out, got {err:?}"
        );
    }

    #[tokio::test]
    async fn generate_bounded_retries_on_5xx() {
        // D-7 / LOOP-16: retries are capped by `request_max_retries` — a
        // stuck upstream (persistent 500) must fail after the cap instead of
        // retrying until the backoff window expires. Uses a SMALL cap (3) so
        // the test is fast; the retry-cap semantics are identical at 10.
        let policy = RetryPolicy {
            request_max_retries: 3,
            ..short_retry_policy()
        };
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::json(
                500,
                r#"{"error":{"message":"boom","type":"server_error","code":null}}"#,
            )
        })
        .await;
        let t = mock_transport_with_retry(&base, policy);
        let err = t.generate(request()).await.unwrap_err();
        assert!(
            matches!(err, GatewayError::Model(_)),
            "persistent 5xx must fail after retry cap: {err:?}"
        );
        // max_retries=3 → exactly 4 total attempts (1 + 3 retries).
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 4, "retry cap violated: {n} attempts for max_retries=3");
    }

    #[tokio::test]
    async fn generate_retry_window_caps_before_max_retries() {
        // D-7: the backoff window is an INDEPENDENT bound — a tight window
        // stops retries before `request_max_retries` is reached (a stuck
        // upstream must not hold the request for the full retry sequence).
        let policy = RetryPolicy {
            request_max_retries: 10,
            request_retry_window: std::time::Duration::from_millis(250),
            ..short_retry_policy()
        };
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::json(
                500,
                r#"{"error":{"message":"boom","type":"server_error","code":null}}"#,
            )
        })
        .await;
        let t = mock_transport_with_retry(&base, policy);
        let err = t.generate(request()).await.unwrap_err();
        assert!(
            matches!(err, GatewayError::Model(_)),
            "persistent 5xx must fail once the window expires: {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            n <= 4,
            "window must cap attempts well below max_retries=10: {n}"
        );
    }

    #[tokio::test]
    async fn generate_429_retry_then_success() {
        // D-7: transient 429 retries with backoff, then succeeds — the
        // bounded loop must not give up on the first 429.
        let attempts = std::sync::atomic::AtomicU32::new(0);
        let base = spawn_mock(move |_line, _body| {
            let n = attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < 2 {
                MockResponse::json(
                    429,
                    r#"{"error":{"message":"rate limited","type":"rate_limit_error","code":"rate_limit_exceeded"}}"#,
                )
            } else {
                MockResponse::json(
                    200,
                    r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"ok-after-429"},"finish_reason":"stop"}]}"#,
                )
            }
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let r = t.generate(request()).await.unwrap();
        assert_eq!(r.text.as_deref(), Some("ok-after-429"));
    }

    #[tokio::test]
    async fn generate_stream_idle_watchdog_aborts_silence() {
        // D-7: the idle watchdog aborts a stream that goes completely silent
        // (no data at all past `stream_idle_timeout`) — slow thinking with
        // progress is not a timeout, a dead wire is. GAP-STREAM-RETRY
        // (2026-08-12): a zero-chunk idle is now retried, so a persistently
        // silent upstream surfaces as `StreamInterrupted` after the retry
        // cap — never as a silent `Timeout` with unanswered retries.
        let policy = RetryPolicy {
            request_max_retries: 1,
            ..short_timeout_policy()
        };
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::json_delayed(
                200,
                r#"{"id":"x","object":"chat.completion","created":0,"model":"m","choices":[{"index":0,"message":{"role":"assistant","content":"never-arrives"},"finish_reason":"stop"}]}"#,
                std::time::Duration::from_secs(2),
            )
        })
        .await;
        let t = mock_transport_with_retry(&base, policy);
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { attempts, detail } if *attempts == 1 && detail.contains("idle")),
            "silent stream must retry once then fail via StreamInterrupted, got {err:?}"
        );
        assert!(chunks.is_empty());
        // max_retries=1 → 2 total attempts (1 initial + 1 retry).
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 2,
            "idle retry cap violated: {n} attempts for max_retries=1"
        );
    }

    // ── GAP-STREAM-RETRY (2026-08-12): zero-chunk interruption retry ──────

    /// A well-formed SSE stream body with one text chunk and a finish block.
    fn ok_sse_body(text: &str) -> String {
        let frame = |delta: &str, finish: Option<&str>| {
            let finish = finish
                .map(|f| format!("\"{f}\""))
                .unwrap_or_else(|| "null".to_string());
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":{finish}}}]}}\n\n",
            )
        };
        format!(
            "{}{}data: [DONE]\n\n",
            frame(
                &format!(r#"{{"role":"assistant","content":"{text}"}}"#),
                None
            ),
            frame("{}", Some("stop")),
        )
    }

    #[tokio::test]
    async fn generate_stream_zero_chunk_transport_error_retries_then_succeeds() {
        // GAP-STREAM-RETRY (2026-08-12): the TB job1 signature — the
        // provider's streaming endpoint died with ZERO chunks produced
        // (`transport error: error decoding response body`). The identical
        // request is re-sent; the retried attempt succeeds. Only the
        // retried response's chunks may be projected (no duplicate output).
        let ok = ok_sse_body("ok-after-retry");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                // Mid-frame wire drop — the read dies before any chunk.
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO).truncated()
            } else {
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("ok-after-retry"));
        assert_eq!(chunks, vec!["ok-after-retry"]);
        // NOTE: connection-count assertion is intentionally loose (>=2) —
        // the fork's EventSource ALSO auto-reconnects on read errors
        // (pre-existing, registered in the audit), so the total may exceed
        // orz's own retry count. The precise "exactly one orz retry"
        // contract is asserted by the EOF variant (`..._eof_...`), where the
        // clean end-of-stream does not trigger the fork's reconnect.
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            n >= 2,
            "zero-chunk interruption must be retried (orz or fork reconnect): {n} connections"
        );
    }

    #[tokio::test]
    async fn generate_stream_zero_chunk_eof_retries_then_succeeds() {
        // A clean EOF with zero frames (`data: [DONE]` only) is also a
        // zero-chunk truncation — no finish_reason, no output — and retries.
        let ok = ok_sse_body("ok-after-eof");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                MockResponse::sse(vec!["data: [DONE]\n\n"], std::time::Duration::ZERO)
            } else {
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("ok-after-eof"));
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 2, "zero-chunk EOF must retry exactly once: {n}");
    }

    #[tokio::test]
    async fn generate_stream_after_chunk_interruption_is_not_retried() {
        // The retry boundary (user ruling 2026-08-12): once ANY chunk
        // landed, output existed — re-sending would duplicate tool calls.
        // A mid-stream drop after a chunk must fail immediately (exactly
        // one connection).
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
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Transport(m) if m.contains("finish_reason")),
            "after-chunk truncation must stay a plain transport error: {err:?}"
        );
        assert_eq!(chunks, vec!["half", "-done"]);
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 1,
            "after-chunk interruption must NOT retry: {n} connections"
        );
    }

    #[tokio::test]
    async fn generate_stream_zero_chunk_retry_capped_by_max_retries() {
        // GAP-STREAM-RETRY: retries are capped by `request_max_retries` —
        // a persistently interrupted upstream fails after the cap with the
        // attempt count visible in the error (journal observability).
        let policy = RetryPolicy {
            request_max_retries: 3,
            ..short_retry_policy()
        };
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec!["data: [DONE]\n\n"], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, policy);
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { attempts, detail } if *attempts == 3 && detail.contains("finish_reason")),
            "persistent interruption must surface StreamInterrupted with the retry count: {err:?}"
        );
        assert!(chunks.is_empty());
        // max_retries=3 → exactly 4 total attempts (1 + 3 retries).
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 4, "retry cap violated: {n} attempts for max_retries=3");
    }

    #[tokio::test]
    async fn generate_stream_zero_chunk_retry_window_caps_before_max_retries() {
        // GAP-STREAM-RETRY: the backoff window is an INDEPENDENT bound (same
        // semantics as the fork's non-streaming execute_raw) — a tight
        // window stops retries before `request_max_retries` is reached.
        //
        // Window sizing (2026-08-12 review P2-1): backoff 0.4.0 only grants
        // a delay when `elapsed + randomized_interval <= max_elapsed_time`.
        // At 250ms the FIRST next_backoff already returns None (elapsed≈2ms
        // + min interval 250ms > 250ms), so the test would pass with zero
        // retries — asserting nothing. At 1s the first retry is guaranteed
        // (2ms + ≤750ms ≤ 1s) and the window exhausts well before
        // max_retries=10: retries stack ≥250ms each, so n lands in {2..=5}.
        let policy = RetryPolicy {
            request_max_retries: 10,
            request_retry_window: std::time::Duration::from_millis(1000),
            ..short_retry_policy()
        };
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec!["data: [DONE]\n\n"], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, policy);
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::StreamInterrupted { attempts, .. } if attempts >= 1),
            "window-exhausted interruption must surface StreamInterrupted after ≥1 re-send: {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            (2..=5).contains(&n),
            "window must cap attempts well below max_retries=10 and still allow the first retry: {n}"
        );
    }

    #[tokio::test]
    async fn generate_stream_cancel_during_retry_backoff_aborts_promptly() {
        // GAP-STREAM-RETRY: /stop must not be held hostage by a retry
        // backoff — a cancel during the sleep aborts the chain immediately
        // (no second connection, well under the first backoff delay).
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec!["data: [DONE]\n\n"], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let cancel = tokio_util::sync::CancellationToken::new();
        tokio::spawn({
            let cancel = cancel.clone();
            async move {
                // The first backoff delay is [250, 750]ms (randomization
                // factor 0.5) — cancel long before it.
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                cancel.cancel();
            }
        });
        let mut chunks = Vec::new();
        // 400ms budget: a retry backoff (500ms) would exceed it; the cancel
        // must win long before.
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(400),
            t.generate_stream(request(), Some(&cancel), None, &mut |c| {
                chunks.push(c.to_string())
            }),
        )
        .await;
        let err = result
            .expect("cancel during backoff must return promptly")
            .unwrap_err();
        assert!(
            matches!(err, GatewayError::Cancelled),
            "cancel during backoff must surface as Cancelled: {err:?}"
        );
        assert!(chunks.is_empty());
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 1, "no connection may follow the cancel: {n}");
    }

    #[tokio::test]
    async fn generate_stream_handshake_hang_retried_then_succeeds() {
        // GAP-STREAM-RETRY: a stalled handshake (no response headers within
        // the idle timeout — F-08) is also a zero-chunk failure and retries.
        let ok = ok_sse_body("ok-after-hang");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                // Upstream accepts the connection but withholds headers for
                // 2s — far past the 150ms idle timeout of the test policy.
                MockResponse::sse_delayed(vec![&ok], std::time::Duration::from_secs(2))
            } else {
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t = mock_transport_with_retry(&base, short_timeout_policy());
        let mut chunks = Vec::new();
        let r = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap();
        assert_eq!(r.text.as_deref(), Some("ok-after-hang"));
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 2, "handshake hang must retry exactly once: {n}");
    }

    #[tokio::test]
    async fn generate_stream_total_budget_backstop() {
        // D-7: the total budget is an auxiliary backstop over the whole
        // stream — even with periodic data, a stream that never terminates
        // within the budget errors instead of running forever.
        let frame = |delta: &str| {
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":null}}]}}\n\n",
            )
        };
        // 10 frames × 100ms = 1s of flowing data (100ms > 0, so the wire is
        // alive; 100ms < 150ms idle_timeout, so the idle watchdog never
        // fires) — but the total budget (500ms) caps the whole stream first.
        let mut body = String::new();
        for i in 0..10 {
            body.push_str(&frame(&format!(
                r#"{{"role":"assistant","content":"f{i}"}}"#
            )));
        }
        body.push_str("data: [DONE]\n\n");
        let base = spawn_mock(move |_line, _body| {
            MockResponse::sse(vec![&body], std::time::Duration::from_millis(100))
        })
        .await;
        let t = mock_transport_with_retry(&base, short_timeout_policy());
        let mut chunks = Vec::new();
        let err = t
            .generate_stream(request(), None, None, &mut |c| chunks.push(c.to_string()))
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Timeout(m) if m.contains("total")),
            "non-terminating stream must hit the total budget, got {err:?}"
        );
        // Data did flow before the backstop fired.
        assert!(!chunks.is_empty());
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
                thinking: None,
            })
            .await;
        assert!(r.is_ok(), "{r:?}");
    }
}
