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
//! `thinking: {type: "enabled"}` + `reasoning_effort: "max"` + 256K
//! single-round output budget (OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD
//! 2026-08-20, ADR-0010 §14.35 — 输出预算恢复 32K → 256K；THIN-HARNESS-
//! REDESIGN R1 2026-08-27: 默认档 high → max（官方 82.7% 基线，用户裁决），
//! `ORZ_THINKING_MODE` env 可覆盖；`EnabledLow` 为降级梯中间档——
//! 2026-08-28 THIN-HARNESS-REDESIGN V2 R1：自动降级梯 max/high → low →
//! 明确失败（空响应链，low 封顶、不自动进 disabled）；OUTPUT-
//! DEGENERATION-GUARD 2026-08-19 的 32K 止损由输出健康哨兵取代；
//! THIN-HARNESS-REDESIGN 2026-08-28（用户裁决）：reasoning-stall 预算兜底
//! 物理删除（官方 max 只等待不杀，复读判定已足够；previously 160K);
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
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

use super::model::{
    FinishReason as OurFinishReason, GatewayError, ModelConfig, ModelGateway, ModelRequest,
    ModelResponse, ThinkingMode, ToolCall, TransportRetryInfo, TransportRetryKind,
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

/// 复读判定滚动哈希 L-gram 长度（2026-08-21 设计 §3.3 修订：路径①「连续
/// 相同 delta N=5」由滑动窗口滚动哈希任意偏移检测取代——dna-assembly 复跑
/// 误杀实证：DeepSeek 小 chunk 粒度 + 低熵文本下 5 个相邻相同 delta 天然
/// 命中，与真实复读无关；2026-08-22 再校准：L 48→**200**——DNA 重跑实证
/// 48 字符粒度下 reasoning 正常任务内容重复引用被误杀（缺口 A）；2026-08-23
/// 再校准：**200→400**——G4 冒烟实证（sweep-r1-g4-official）sam-cell-seg
/// reasoning 层 203/204 字符代码引用被误判 2 次，短引用（<400 字符）第一级
/// 即不命中）。流内出现相同 400 字符 L-gram 的匹配即计一次「命中候选」，
/// 经二级「标点块内部重复确认」（或无可切分点）后才计「命中」。
pub const REPETITION_MIN_RUN_CHARS: usize = 400;

/// 复读判定比较窗口（=2L，设计 §3.3 修订 + 2026-08-22 再校准 96→**400**）：
/// 2026-08-23 再校准 **400→800**（=2L，缓冲 600→1200）：新尾部 L-gram 与
/// 最近 800 字符内已出现的 L-gram 做任意偏移比对（起点偏移 ∈ [400, 800]）。
/// 窗口内任意周期可命中（p ≤ 800，修复固定偏移相位对齐缺陷：周期 10 短语
/// 循环在固定偏移下相邻窗口永不相同）；同字符连串由独立触发线
/// `REPETITION_SAME_CHAR_TRIP_CHARS` 接管（2026-08-28 R1，用户裁决保留
/// 802 线——统一门槛 20 下滚动路径对同字符连串要到 819 字符才凑满 20 次
/// 命中，802 独立线更快且语义明确）；DNA 低熵正常序列免疫。
pub const REPETITION_WINDOW_CHARS: usize = 800;

/// 同字符连串独立触发线（2026-08-28 THIN-HARNESS-REDESIGN V2 R1，用户
/// 裁决保留 802）：连续相同字符 ≥ 本值直接显式拦截（病理同字符流，成本
/// 极低、属明确复读，不引入误杀面）。推导=旧「命中门槛 3」派生线：记录
/// 区在 idx ≥ W-1 才非空（800 字符=1 次、801=2 次、802=3 次命中）；统一
/// 门槛 20 后改显式常量，语义不变（设计 §3.3）。
pub const REPETITION_SAME_CHAR_TRIP_CHARS: usize = 2 * REPETITION_MIN_RUN_CHARS + 2;

/// 流内累计命中门槛（2026-08-28 THIN-HARNESS-REDESIGN V2 R1，用户裁决）：
/// 统一门槛 **20**——同一流内累计确认命中 ≥20 次才显式拦截该轮；命中
/// 计数**不因中间未命中内容重置**（间隔不重置）；1–19 次命中仅审计留痕
/// （WARN 输出触发 span + 窗口片段），不中断、不降档。命中计数随流结束
/// 丢弃（不跨请求累积——检测器按 generate_stream 每次新建）；
/// content/reasoning 两族统一；run 级 `DEGENERATION_LIMIT` 语义不变。
/// 2026-08-28：序列内容门（SEQUENCE CONTENT GATE）整套删除——原误杀场景
/// （EGFP/蛋白序列合法引用）被统一门槛 20 自然覆盖，一个旋钮取代整套内容
/// 分类器（设计 §3.2）。
pub const REPETITION_HIT_LIMIT: usize = 20;

/// 二级「标点块内部重复确认」阈值（2026-08-23 用户裁决）：滚动哈希命中
/// 候选的 span 按标点+空白（`_` 除外）切块后，内部重复子块覆盖字符占比
/// ≥ 0.50 才计该次命中（每一对命中都须过二级；占比分母=span 总字符数、
/// 含切分符；无可切分点→二级不起作用→直接判真）。初值，S2 用真实样本
/// 校准。
pub const REPETITION_PUNCT_BLOCK_MIN_RATIO: f64 = 0.50;

/// 二级确认切分符：ASCII 标点（`_` 除外，保持标识符完整）+ 常用中文标点 +
/// 空白（空格/换行等）。
/// 边界（2026-08-23 审查处理 P3-2 登记）：集合为实现常用子集——全角变体
/// （％＃＆〈〉〔〕〖〗等）与生僻中文标点未包含，含此类字符的重复块在该处
/// 不切分、仍按整块计入覆盖；S2 用真实样本校准时可视需要扩集合。
fn is_repetition_block_separator(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '!' | '"'
                | '#'
                | '$'
                | '%'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | '+'
                | ','
                | '-'
                | '.'
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '?'
                | '@'
                | '['
                | '\\'
                | ']'
                | '^'
                | '`'
                | '{'
                | '|'
                | '}'
                | '~'
                | '，'
                | '。'
                | '、'
                | '；'
                | '：'
                | '？'
                | '！'
                | '“'
                | '”'
                | '‘'
                | '’'
                | '（'
                | '）'
                | '《'
                | '》'
                | '【'
                | '】'
                | '「'
                | '」'
                | '『'
                | '』'
                | '·'
                | '…'
                | '—'
                | '～'
        )
}

/// 二级确认的 span 标点块统计（2026-08-23）：按切分符把 span 切成子块，
/// 统计内部重复子块的字符覆盖占比。
#[derive(Debug)]
struct RepetitionBlockStats {
    /// 切分出的子块数（0=整段全为切分符、1=无切分点单块）。
    block_count: usize,
    /// span 总字符数（含切分符；覆盖占比分母）。
    total_chars: usize,
    /// 重复子块覆盖字符占比：Σ(重复块长度×出现次数) / total_chars。
    repeated_coverage: f64,
    /// 重复块直方图（按 长度×次数 降序，至多 8 条；审计留痕用）。
    repeated_blocks: Vec<(String, usize)>,
}

impl RepetitionBlockStats {
    /// 二级判定：无切分点（单块）或整段无可切分块 → 二级不起作用 → 直接
    /// 判真；有子块则重复块覆盖占比 ≥ 阈值才通过。2026-08-25 序列内容门
    /// 后，`feed` 内的三态分派已内联等价逻辑（需区分 block_count > 1 与
    /// 无切分点序列样），本便捷判定仅测试引用（S2 标点块矩阵），豁免
    /// dead_code 保持 clippy 与基线一致。
    #[cfg_attr(not(test), allow(dead_code))]
    fn confirmed(&self) -> bool {
        self.block_count <= 1 || self.repeated_coverage >= REPETITION_PUNCT_BLOCK_MIN_RATIO
    }

    /// 审计摘要（二级不过的命中候选留痕：span + 标点块统计）。
    fn summary(&self) -> String {
        let blocks = self
            .repeated_blocks
            .iter()
            .map(|(b, n)| format!("{:?}x{}", truncate_block(b), n))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "punct-block repeated coverage {:.2} (min {:.2}) over {} span chars; repeated blocks: [{}]",
            self.repeated_coverage, REPETITION_PUNCT_BLOCK_MIN_RATIO, self.total_chars, blocks
        )
    }
}

/// 审计摘要用块文本截断（避免超长块刷屏）。
fn truncate_block(b: &str) -> String {
    let mut it = b.chars();
    let mut head: String = it.by_ref().take(48).collect();
    if it.next().is_some() {
        head.push('…');
    }
    head
}

/// 把 span 按切分符切块并统计内部重复子块覆盖占比（二级确认核心）。
fn repetition_block_stats(span: &[char]) -> RepetitionBlockStats {
    let total_chars = span.len();
    let mut blocks: Vec<String> = Vec::new();
    let mut cur = String::new();
    for &c in span {
        if is_repetition_block_separator(c) {
            if !cur.is_empty() {
                blocks.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        blocks.push(cur);
    }
    let block_count = blocks.len();
    if block_count <= 1 {
        return RepetitionBlockStats {
            block_count,
            total_chars,
            repeated_coverage: 1.0,
            repeated_blocks: Vec::new(),
        };
    }
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for b in &blocks {
        *counts.entry(b.as_str()).or_insert(0) += 1;
    }
    let mut repeated_blocks: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|&(_, n)| n >= 2)
        .map(|(b, n)| (b.to_string(), n))
        .collect();
    let covered: usize = repeated_blocks
        .iter()
        .map(|(b, n)| b.chars().count().saturating_mul(*n))
        .sum();
    repeated_blocks.sort_by_key(|b| std::cmp::Reverse(b.0.chars().count() * b.1));
    repeated_blocks.truncate(8);
    RepetitionBlockStats {
        block_count,
        total_chars,
        repeated_coverage: covered as f64 / total_chars.max(1) as f64,
        repeated_blocks,
    }
}

/// 滚动哈希基数（多项式哈希；取奇数避免与 2^64 非互质的退化）。
const REPETITION_HASH_BASE: u64 = 1_000_003;

/// 滚动哈希窗口容量 = L + W（保留尾部 L-gram + 最近 W 字符比较区）。
const REPETITION_BUFFER_CHARS: usize = REPETITION_MIN_RUN_CHARS + REPETITION_WINDOW_CHARS;

/// B^(L-1) mod 2^64（滑动哈希的移除权重；const fn 编译期计算）。
const fn repetition_hash_pow() -> u64 {
    let mut pow: u64 = 1;
    let mut i: usize = 0;
    while i < REPETITION_MIN_RUN_CHARS - 1 {
        pow = pow.wrapping_mul(REPETITION_HASH_BASE);
        i += 1;
    }
    pow
}

const REPETITION_HASH_POW: u64 = repetition_hash_pow();

/// 重复率检测的累计 token 门槛——累计输出 ≥ 1K token 后才检查最近 1K token
/// 窗口（设计 §3.3；复读诱因=超长输出，窗口足够小前不误报）。
pub const DEGENERATION_MIN_TOKENS: usize = 1_000;

/// 最近窗口内 3-gram 重复率阈值（>70% 触发；设计 §3.3，2026-08-23
/// NGRAM-GUARD-CALIBRATION：0.60→**0.70**，`>` 严格大于保留）。
pub const DEGENERATION_NGRAM_REPEAT_RATIO: f64 = 0.70;

/// 3-gram 路径②流内累计命中门槛（2026-08-28 THIN-HARNESS-REDESIGN V2
/// R1，用户裁决：随复读守卫大幅拉升 3→**15**；阈值 0.70 不变——与复读
/// 守卫统一「只抓明确复读」口径，15 个 1K-token 高重复窗口才 trip）；
/// 1–14 次命中仅审计留痕、间隔不重置、流结束丢弃（与路径①纪律对齐，
/// 设计 §3.3）。
pub const NGRAM_HIT_LIMIT: usize = 15;

/// Token pattern mirroring Python `[\w一-鿿]+` (Unicode word + CJK) — the
/// 3-gram fallback's tokenizer, migrated from the retired runtime stagnation
/// guard (2026-08-22).
fn token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\w\u{4E00}-\u{9FFF}]+").expect("static token regex"))
}

/// Python `_tokenize`: casefolded token list.
fn tokenize(value: &str) -> Vec<String> {
    token_regex()
        .find_iter(&value.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}

/// 同一会话连续退化中断上限（默认 3；设计 §3.3 防循环——第 3 次起以
/// `degeneration_limit_reached` 标记，run 层据此记 `run_invalidated`）。
pub const DEGENERATION_LIMIT: u32 = 3;

/// 退化中断 detail 前缀（命中但未达上限——run 走 run_failed 同路径）。
pub const DEGENERATION_DETAIL_PREFIX: &str = "degeneration_detected:";

/// 退化中断达上限 detail 前缀（run 层映射为 `run_invalidated`）。
pub const DEGENERATION_LIMIT_PREFIX: &str = "degeneration_limit_reached:";

/// MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.2)：已见
/// chunk 但未解码出完整 tool_calls 的中段截断重试上限。编译期常量（设计
/// §3：不新增运行期旋钮）——有界 1 次，防「已烧数十 K 再原样重试烧一轮」
/// 病态放大；耗尽后显式失败。
pub const CHUNKED_MIDSTREAM_MAX_RETRIES: u32 = 1;

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

/// content 复读族 detail 前缀（已见输出 → 不重试，ADR-0007）。
pub const CONTENT_REPETITION_DETAIL_PREFIX: &str = "degeneration_detected:content_repetition:";

/// reasoning 复读族 detail 前缀（灵敏层——无可见输出 → 不原样重试、显式
/// 拦截；2026-08-28 R1 起不再降档）。
pub const REASONING_REPETITION_DETAIL_PREFIX: &str = "degeneration_detected:reasoning_repetition:";

/// 一次滚动哈希字符级命中候选（2026-08-23 二级确认后）：字符级匹配但
/// 二级不过的候选仅审计留痕（span + 标点块统计），不计数。2026-08-28 R1
/// 序列内容门删除后收敛为二态（confirmed bool），不再有序列族分派。
#[derive(Debug)]
struct RepetitionCandidateAudit {
    /// 二级确认结果：true=通过（计命中）；false=二级不过（仅留痕）。
    confirmed: bool,
    /// 审计上下文：重复 span 文本 + 两个匹配偏移 + 窗口尾部（仅确认命中
    /// 构造——二级不过的候选不构建 ~1.3KB 完整上下文，由 `span` +
    /// `block_stats` 聚合留痕）。
    ctx: Option<String>,
    /// 重复 span 文本（审计留痕用；代表本条候选的 400 字符内容）。
    span: String,
    /// 二级不过时的标点块统计摘要（通过时为 None）。
    block_stats: Option<String>,
}

/// 滑动窗口滚动哈希任意偏移复读检测（2026-08-21 设计 §3.3 修订 +
/// 2026-08-22/23 再校准）：维护最近 L+W=1200 字符缓冲，记录区内（起点
/// 偏移 ∈ [400, 800]）全部 400 字符 L-gram 的滚动哈希；新尾部 L-gram 的
/// 哈希若在记录区已出现即计一次「命中候选」（哈希命中后字符级比对防
/// 碰撞），候选再经二级「标点块内部重复确认」（或无切分点）才计命中。
/// 窗口内任意周期可命中（p ≤ 800；修复固定偏移相位对齐缺陷）；DNA 低熵
/// 正常序列免疫；O(1)/字符（摊销）。命中后窗口**继续喂入**（不早停）——
/// 流内命中累计、间隔不重置，由检测器按 `REPETITION_HIT_LIMIT` 判定
/// 触发；调用方以「剩余门槛」封顶单次喂入（达门槛即不再消费，见
/// `feed_chars_capped`，2026-08-22 审查处理 P3）。
#[derive(Debug, Default)]
struct RollingRepetitionWindow {
    /// 最近 ≤L+W 字符（`base` 为 `chars[0]` 的全局序号）。
    chars: VecDeque<char>,
    /// `chars[0]` 的全局序号（从未喂入时为 0，无意义）。
    base: usize,
    /// 记录区 L-gram：(滚动哈希, 全局起点)，按起点升序、最旧在前。
    recorded: VecDeque<(u64, usize)>,
    /// 记录区哈希 → 出现次数（O(1) 命中判定；字符级比对在命中后扫描
    /// `recorded` 完成）。
    counts: HashMap<u64, usize>,
    /// 尾部 ≤L 字符的滚动哈希（当前 L-gram）。
    trailing_hash: u64,
    /// 即将进入记录区的 L-gram 滚动哈希（长度达标前未使用）。
    enter_hash: u64,
    /// 最近一次命中时的两个匹配 span 全局起点（旧起点 + 新起点）——审计
    /// 留痕用（缺口 A，2026-08-22：退化触发需落盘触发内容供事后判定真
    /// 复读 vs 误杀；命中后窗口继续喂入，每次命中覆盖为最新一对）。
    last_match: Option<(usize, usize)>,
    /// 本 delta 内命中候选审计记录（`feed_chars_capped` 每次调用前清空；
    /// 2026-08-23 二级确认后：字符级匹配但二级不过的候选也留痕（按 delta
    /// 聚合为一条摘要 + 计数，见 `feed_repetition`），不计数）。
    candidates: Vec<RepetitionCandidateAudit>,
    /// 下一个待喂入字符的全局序号。
    next: usize,
}

impl RollingRepetitionWindow {
    /// 喂入一个 delta（逐字符）；返回本 delta 内新产生的**通过二级确认的**
    /// 命中次数
    /// （2026-08-22 再校准：命中后窗口继续喂入——命中计数流内累计、间隔
    /// 不重置，调用方在累计 ≥`REPETITION_HIT_LIMIT` 时判定触发；旧语义
    /// 「命中即终止」退役）。等价于不设上限的 `feed_chars_capped`
    /// （`usize::MAX`），供测试与独立窗口使用（2026-08-22 审查处理：
    /// 生产路径已改走 `feed_chars_capped`，lib 目标下该便捷方法仅测试
    /// 引用，豁免 dead_code 保持 clippy 与基线一致）。
    #[cfg_attr(not(test), allow(dead_code))]
    fn feed_chars(&mut self, delta: &str) -> usize {
        self.feed_chars_capped(delta, usize::MAX)
    }

    /// 带命中上限的喂入：本 delta 内新**确认**命中数达 `hit_cap` 即停止
    /// 消费（后续字符不再喂入）。2026-08-22 审查处理（P3）：调用方以
    /// 「剩余门槛=门槛-已累计」封顶——同一 delta 内累计确认命中达到
    /// `REPETITION_HIT_LIMIT` 后该流必触发，继续喂入超大退化帧只会白做
    /// 滚动哈希与字符级比对；子门槛 delta 全量消费，窗口状态与旧语义
    /// 完全一致。2026-08-23：每次调用前清空 `candidates`（本 delta 命中
    /// 候选留痕，供调用方逐条审计；二级不过的候选不计数）。返回本 delta
    /// 确认命中数。
    fn feed_chars_capped(&mut self, delta: &str, hit_cap: usize) -> usize {
        self.candidates.clear();
        let mut hits = 0;
        for c in delta.chars() {
            if hits >= hit_cap {
                break;
            }
            if self.feed(c) {
                hits += 1;
            }
        }
        hits
    }

    /// 取走本 delta 的命中候选审计记录（调用方逐条审计；二级不过的候选
    /// 不计数、仅留痕）。
    fn take_candidates(&mut self) -> Vec<RepetitionCandidateAudit> {
        std::mem::take(&mut self.candidates)
    }

    /// 喂入一个字符；返回是否计一次确认命中（true=通过二级确认）。
    fn feed(&mut self, c: char) -> bool {
        let idx = self.next;
        self.next += 1;
        self.chars.push_back(c);

        // 尾部 L-gram 滚动哈希：<L 字符时增长；≥L 后滑动（丢最旧、加新）。
        if idx < REPETITION_MIN_RUN_CHARS {
            self.trailing_hash = self
                .trailing_hash
                .wrapping_mul(REPETITION_HASH_BASE)
                .wrapping_add(c as u64);
        } else {
            let drop = self.char_at(idx - REPETITION_MIN_RUN_CHARS);
            self.trailing_hash = slide_repetition_hash(self.trailing_hash, drop, c);
        }

        // 进入记录区的 L-gram 滚动哈希：起点 p = idx-(W-1)（span [p, p+L)），
        // 从 idx=W-1（p=0）起每步右移 1。
        if idx == REPETITION_WINDOW_CHARS - 1 {
            self.enter_hash = self.hash_span(idx - (REPETITION_WINDOW_CHARS - 1));
        } else if idx > REPETITION_WINDOW_CHARS - 1 {
            let drop = self.char_at(idx - REPETITION_WINDOW_CHARS);
            let add = self.char_at(idx - REPETITION_MIN_RUN_CHARS);
            self.enter_hash = slide_repetition_hash(self.enter_hash, drop, add);
        }

        // 记录区增删（新尾部 L-gram 的任意偏移比较集：起点 ∈
        // [idx-(L+W-1), idx-(W-1)]；与尾部起点 idx-(L-1) 的距离恒 ∈
        // [L, W]，结构性保证「起点距离 ≥L」）。
        if idx >= REPETITION_WINDOW_CHARS - 1 {
            let p = idx - (REPETITION_WINDOW_CHARS - 1);
            self.recorded.push_back((self.enter_hash, p));
            *self.counts.entry(self.enter_hash).or_insert(0) += 1;
        }
        if idx >= REPETITION_BUFFER_CHARS {
            let (h, p) = self.recorded.pop_front().expect("recorded is non-empty");
            debug_assert_eq!(p, idx - REPETITION_BUFFER_CHARS);
            if let Some(n) = self.counts.get_mut(&h) {
                *n -= 1;
                if *n == 0 {
                    self.counts.remove(&h);
                }
            }
        }

        // 任意偏移命中候选：尾部 L-gram 完整（idx ≥ L-1）且与记录区已
        // 出现（起点偏移 ≥L）字符级一致（防哈希碰撞误报）。记录区到
        // idx ≥ W-1 才非空，此前实际不可能命中。命中后**不早退**——窗口
        // 继续滑动，供流内命中计数累计（2026-08-22 再校准）；2026-08-23
        // 起字符级匹配仅产生「候选」，经二级「标点块内部重复确认」才计
        // 命中（不过者留痕、不计数；`feed_repetition` 按 delta 聚合）。
        let mut confirmed = false;
        if idx >= REPETITION_MIN_RUN_CHARS - 1
            && self.counts.get(&self.trailing_hash).is_some_and(|n| *n > 0)
        {
            let s = idx - (REPETITION_MIN_RUN_CHARS - 1);
            if let Some(p) = self
                .recorded
                .iter()
                .find(|&&(h, p)| h == self.trailing_hash && self.spans_equal(p, s))
                .map(|&(_, p)| p)
            {
                self.last_match = Some((p, s));
                // 二级确认（2026-08-23 用户裁决）：被判定重复的大块须「内部
                // 由重复的标点块构成」（或无可切分点）才计该次命中——字符
                // 级匹配但二级不过的候选仅审计留痕、不计数。`spans_equal`
                // 已保证两 span 字符级相等，检查新 span 内部结构即代表该对。
                let span_chars: Vec<char> = self
                    .chars
                    .iter()
                    .skip(s - self.base)
                    .take(REPETITION_MIN_RUN_CHARS)
                    .copied()
                    .collect();
                let span: String = span_chars.iter().collect();
                let stats = repetition_block_stats(&span_chars);
                confirmed = stats.block_count <= 1
                    || stats.repeated_coverage >= REPETITION_PUNCT_BLOCK_MIN_RATIO;
                // 完整 ctx（偏移 + 窗口尾部）仅确认命中需要（触发/审计上下
                // 文）；二级不过的候选按 span 聚合，不构造大文本。
                let ctx = confirmed.then(|| {
                    self.match_context()
                        .unwrap_or_else(|| format!("repeated span at global offset {s}"))
                });
                self.candidates.push(RepetitionCandidateAudit {
                    confirmed,
                    ctx,
                    span,
                    block_stats: (!confirmed).then(|| stats.summary()),
                });
            }
        }

        // 回收超过缓冲容量的字符。
        if self.chars.len() > REPETITION_BUFFER_CHARS {
            self.chars.pop_front();
            self.base += 1;
        }
        confirmed
    }

    /// 缓冲内全局序号 `g` 的字符。
    fn char_at(&self, g: usize) -> char {
        self.chars[g - self.base]
    }

    /// 缓冲内起点 `start` 的 L 字符 span 的滚动哈希（多项式、首字符最高位）。
    fn hash_span(&self, start: usize) -> u64 {
        let mut h = 0u64;
        let pos = start - self.base;
        for i in 0..REPETITION_MIN_RUN_CHARS {
            h = h
                .wrapping_mul(REPETITION_HASH_BASE)
                .wrapping_add(self.chars[pos + i] as u64);
        }
        h
    }

    /// 字符级比对两个 L 字符 span（全局起点 a/b，均在缓冲内）。
    fn spans_equal(&self, a: usize, b: usize) -> bool {
        let pa = a - self.base;
        let pb = b - self.base;
        (0..REPETITION_MIN_RUN_CHARS).all(|k| self.chars[pa + k] == self.chars[pb + k])
    }

    /// 触发审计上下文（缺口 A）：重复 span 文本 + 两个匹配起点在最近
    /// 窗口内的偏移 + 窗口尾部片段。仅在 `last_match` 置位后调用。
    fn match_context(&self) -> Option<String> {
        let (old, new) = self.last_match?;
        let span: String = self
            .chars
            .iter()
            .skip(new - self.base)
            .take(REPETITION_MIN_RUN_CHARS)
            .collect();
        let tail_len = self.chars.len().min(REPETITION_WINDOW_CHARS);
        let tail_start = self.chars.len() - tail_len;
        let tail: String = self.chars.iter().skip(tail_start).collect();
        Some(format!(
            "repeated {L}-char span {span:?} (matched offsets {old_off} and {new_off} \
             in the recent {W}-char window); window tail: {tail:?}",
            L = REPETITION_MIN_RUN_CHARS,
            old_off = old.saturating_sub(self.base),
            new_off = new.saturating_sub(self.base),
            W = REPETITION_WINDOW_CHARS,
        ))
    }
}

/// 滚动哈希滑动一步：丢 `drop_c`（权重 B^(L-1)）、追加 `add_c`。
fn slide_repetition_hash(h: u64, drop_c: char, add_c: char) -> u64 {
    h.wrapping_sub((drop_c as u64).wrapping_mul(REPETITION_HASH_POW))
        .wrapping_mul(REPETITION_HASH_BASE)
        .wrapping_add(add_c as u64)
}

/// 复读检测单族状态（content / reasoning 各一份）：滚动哈希窗口 + 3-gram
/// 窗口 + 流内命中计数。集中为单参数传入 `feed_repetition`，保持自由函数
/// 参数在 clippy 阈值内（2026-08-22：命中计数随流结束丢弃——检测器按
/// generate_stream 每次新建，天然不跨请求累积）。
#[derive(Debug, Default)]
struct RepetitionFamilyState {
    /// 滑动窗口滚动哈希复读检测。
    rolling: RollingRepetitionWindow,
    /// 累计 token 数（3-gram 重复率检测的启用门槛）。
    total_tokens: usize,
    /// 最近 1K token 的滑动窗口。
    window_tokens: Vec<String>,
    /// 路径①滚动哈希流内累计确认命中（≥`REPETITION_HIT_LIMIT` 触发；
    /// 1–19 次仅审计留痕、间隔不重置）。2026-08-28 R1：序列门删除后无
    /// 单独序列计数。
    hits: usize,
    /// 同字符连串运行长度（跨 delta 连续；字符变化即重置为 1）。802 独立
    /// 触发线（2026-08-28 R1，用户裁决保留）——与滚动哈希/3-gram 路径
    /// 并列，命中即显式拦截。
    same_char_run: usize,
    /// 上一个喂入字符（跨 delta 判定连串）。
    last_char: Option<char>,
    /// 路径②3-gram 流内累计命中（每次 feed 超阈值计 1 次；≥`NGRAM_HIT_LIMIT`
    /// 才 trip；1–14 次仅审计留痕、间隔不重置、流结束丢弃——2026-08-23
    /// NGRAM-GUARD-CALIBRATION + 2026-08-28 R1 门槛 15）。
    ngram_hits: usize,
}

/// 生成期输出健康哨兵（设计 §3.3，第一层治本）：喂入 content delta +
/// reasoning delta + tool_call arguments delta，两族信号（content_repetition
/// / reasoning_repetition）命中后持续返回触发原因。纯
/// 机械、零模型调用；token 口径=Unicode 词 + CJK 正则（迁移自退役的停滞
/// 守卫）；复读判定=滑动窗口滚动哈希任意偏移（设计 §3.3 修订）+ 1K token
/// 窗口 3-gram 重复率兜底。
#[derive(Debug, Default)]
struct DegenerationDetector {
    /// content 族复读检测状态（滚动哈希 + 3-gram + 流内命中计数）。
    content_repetition: RepetitionFamilyState,
    /// reasoning 族复读检测状态（同上）。
    reasoning_repetition: RepetitionFamilyState,
    /// 已见可见输出（content 或 tool_calls 出现）——此后 reasoning 族信号
    /// 停用（设计 §3.3：reasoning 复读仅 content/tool_calls 全空时
    /// 启用；工具轮为合法形态，不误判）。
    saw_visible_output: bool,
    /// 触发原因（触发后恒定，避免同流重复报错）。
    trip: Option<String>,
    /// 触发审计上下文（缺口 A，2026-08-22）：触发 span + 窗口片段，供
    /// 事后判定真复读 vs 误杀（WARN 留痕，不改终止语义）。
    trigger_context: Option<String>,
    /// 流内非触发命中审计留痕（缺口 A 扩展）：1–19 次命中逐条登记触发
    /// 上下文（重复 span + 偏移 + 窗口尾部），调用方逐条 WARN——不中断、
    /// 不降档；达到门槛后由 `trigger_context` 走 trip 分支。
    audit_hits: Vec<String>,
}

impl DegenerationDetector {
    /// content 族 feed：内容即可见输出——标记 visible（reasoning 族停用）
    /// 后走复读检测（连续相同 / 1K 窗口 3-gram 重复率）。
    fn feed_content(&mut self, delta: &str) {
        if self.trip.is_some() || delta.is_empty() {
            return;
        }
        self.saw_visible_output = true;
        feed_repetition(
            &mut self.trip,
            &mut self.trigger_context,
            &mut self.audit_hits,
            "content",
            CONTENT_REPETITION_DETAIL_PREFIX,
            delta,
            &mut self.content_repetition,
        );
    }

    /// reasoning 族 feed（灵敏层，设计 §3.3）：复读检测仅 content/tool_calls
    /// 全空时启用。
    fn feed_reasoning(&mut self, delta: &str) {
        if self.trip.is_some() || delta.is_empty() {
            return;
        }
        if self.saw_visible_output {
            return;
        }
        feed_repetition(
            &mut self.trip,
            &mut self.trigger_context,
            &mut self.audit_hits,
            "reasoning",
            REASONING_REPETITION_DETAIL_PREFIX,
            delta,
            &mut self.reasoning_repetition,
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
        self.saw_visible_output = true;
    }

    fn trip_reason(&self) -> Option<String> {
        self.trip.clone()
    }

    fn trigger_context(&self) -> Option<String> {
        self.trigger_context.clone()
    }

    /// 取走流内非触发命中的审计留痕（2026-08-22 再校准 + 2026-08-28 R1：
    /// 1–19 次命中仅审计留痕、不中断不降档）——调用方逐条 WARN 输出触发
    /// span + 窗口
    /// 片段（缺口 A）。
    fn take_audit_hits(&mut self) -> Vec<String> {
        std::mem::take(&mut self.audit_hits)
    }
}

/// 复读检测共用核心（content 与 reasoning 同一算法，设计 §3.3 +
/// 2026-08-28 THIN-HARNESS-REDESIGN V2 R1 后置化）：
/// ① 滑动窗口滚动哈希任意偏移——流内出现相同 L 字符 span 的匹配先过二级
/// 「标点块内部重复确认」（或无切分点直接判真）才算一次命中；**流内累计
/// 命中 ≥20 次（间隔不重置）才显式拦截**（2026-08-28 R1 用户裁决：观察
/// 留流内、控制移结果侧——1–19 次仅审计留痕，不中断、不降档）；二级不过
/// 的命中候选也留痕（span + 标点块统计，按 delta 聚合为一条摘要 + 计数）、
/// 不计数；
/// ② 同字符连串独立触发线（802，用户裁决保留）——连续相同字符
/// ≥ `REPETITION_SAME_CHAR_TRIP_CHARS` 直接显式拦截（病理同字符流）；
/// ③ 累计 ≥1K token 且最近 1K token 内 3-gram 重复率 >70% → 每次 feed
/// 计 1 次流内命中，累计 ≥`NGRAM_HIT_LIMIT`（15）才触发（保留兜底，不经
/// 二级——重复率判定自带 1K token 滑动窗口粒度；2026-08-28 R1 门槛
/// 3→15，1–14 次命中仅审计留痕）。
/// 自由函数（非方法）——调用方以不相交的字段借用传入，避免方法整体
/// 借用与字段借用冲突。
fn feed_repetition(
    trip: &mut Option<String>,
    trigger_context: &mut Option<String>,
    audit_hits: &mut Vec<String>,
    family: &str,
    detail_prefix: &str,
    delta: &str,
    state: &mut RepetitionFamilyState,
) {
    if trip.is_some() || delta.is_empty() {
        return;
    }
    // ② 同字符连串独立触发线（2026-08-28 R1，用户裁决保留 802）：跨
    // delta 累计连续相同字符；达线即显式拦截（detail 带明确原因）。此
    // 检查先于滚动喂入——达线即无需再消费窗口。
    for c in delta.chars() {
        state.same_char_run = if state.last_char == Some(c) {
            state.same_char_run.saturating_add(1)
        } else {
            1
        };
        state.last_char = Some(c);
        if state.same_char_run >= REPETITION_SAME_CHAR_TRIP_CHARS {
            *trip = Some(format!(
                "{detail_prefix} {run}/{line} same-character run (pathological \
                 identical character stream; explicit line kept by \
                 THIN-HARNESS-REDESIGN V2 R1)",
                run = state.same_char_run,
                line = REPETITION_SAME_CHAR_TRIP_CHARS,
            ));
            return;
        }
    }
    // ① 滚动哈希任意偏移（设计 §3.3 修订 + 2026-08-22/23 再校准 +
    //    2026-08-28 R1 序列门删除）：与 delta 切块粒度无关的字符流级
    //    判定——每次命中候选（相同 L 字符 span 与记录区匹配）先过二级
    //    「标点块内部重复确认」；确认命中统一累计（间隔不重置）。
    // 2026-08-22 审查处理（P3）：以剩余门槛封顶单次喂入——同一 delta
    // 内新**确认**命中达「门槛-已累计」即停止消费（该流必触发，避免超
    // 大退化帧在命中门槛后仍全量喂入的浪费）；子门槛 delta 全量消费，
    // 窗口状态与旧语义一致。
    let remaining = REPETITION_HIT_LIMIT.saturating_sub(state.hits);
    let new_hits = state.rolling.feed_chars_capped(delta, remaining);
    let candidates = state.rolling.take_candidates();
    // 二级不过的命中候选：按 delta 聚合（2026-08-23 审查处理 P2-2，用户
    // 裁决：一条摘要 + 命中计数）——滑动窗口「每对」语义下同一底层重复
    // 内容在每个移位窗口各成一候选且 span 文本互异（1200 字符流内可达
    // 401 对），按精确 span 文本或重复块签名都无法稳定折叠（移位会改变
    // 边缘切块）。聚合后每个 delta 至多一条 rejected 审计（代表 span +
    // 标点块统计 + 本 delta 候选对数），不计数、不影响命中门槛。边界：
    // 同一 delta 内多个不同重复内容合并为一条（代表取最后一条候选；SSE
    // delta 通常数百字符，多重复内容同 delta 罕见），登记为已接受。
    let rejected_count = candidates.iter().filter(|c| !c.confirmed).count();
    if rejected_count > 0 {
        let rep = candidates
            .iter()
            .rev()
            .find(|c| !c.confirmed)
            .expect("rejected_count > 0 implies a rejected candidate");
        audit_hits.push(format!(
            "repeated {L}-char span {span:?} [second-stage rejected: {stats}] \
             ({rejected_count} candidate pair(s) in this delta, aggregated by delta)",
            L = REPETITION_MIN_RUN_CHARS,
            span = rep.span,
            stats = rep.block_stats.as_deref().unwrap_or("no block stats"),
        ));
    }
    if new_hits > 0 {
        // 触发/审计上下文取本 delta 最后一条确认命中（同一重复 span 语义；
        // 2026-08-23：候选已带各自 ctx，不依赖命中后 match_context）。
        let ctx = candidates
            .iter()
            .rev()
            .find(|c| c.confirmed)
            .and_then(|c| c.ctx.clone())
            .or_else(|| state.rolling.match_context());
        *trigger_context = ctx.clone();
        state.hits = state.hits.saturating_add(new_hits);
        if state.hits >= REPETITION_HIT_LIMIT {
            *trip = Some(format!(
                "{detail_prefix} {REPETITION_MIN_RUN_CHARS}-char repeated span in the \
                 recent {REPETITION_WINDOW_CHARS}-char window, {}/{} stream hits (rolling \
                 hash, arbitrary offset)",
                state.hits, REPETITION_HIT_LIMIT
            ));
            return;
        }
        // 1–19 次命中：仅审计留痕（不中断、不降档）。同一 delta 内多次
        // 命中逐条登记（上下文取该 delta 最后一次确认命中——同一重复
        // span 语义）。
        if let Some(ctx) = ctx {
            for _ in 0..new_hits {
                audit_hits.push(ctx.clone());
            }
        }
    }
    // ③ 重复率：累计 ≥ 1K token 且最近 1K token 内 3-gram 重复率 > 70%
    //    → 每次 feed 计 1 次流内命中，累计 ≥ `NGRAM_HIT_LIMIT`（15）才
    //    触发（n-gram 兜底；tokenizer 迁移自退役的停滞守卫；2026-08-23
    //    NGRAM-GUARD-CALIBRATION：阈值 0.60→0.70 + 流内累计命中；
    //    2026-08-28 R1：门槛 3→15，1–14 次仅审计、间隔不重置、流结束
    //    丢弃）。
    let tokens = tokenize(delta);
    state.total_tokens += tokens.len();
    state.window_tokens.extend(tokens);
    let overflow = state
        .window_tokens
        .len()
        .saturating_sub(DEGENERATION_MIN_TOKENS);
    if overflow > 0 {
        state.window_tokens.drain(..overflow);
    }
    if state.total_tokens >= DEGENERATION_MIN_TOKENS {
        let total_ngrams = state.window_tokens.len().saturating_sub(2);
        if total_ngrams >= 1 {
            let mut counts: HashMap<&[String], u32> = HashMap::new();
            for index in 0..total_ngrams {
                *counts
                    .entry(&state.window_tokens[index..index + 3])
                    .or_insert(0) += 1;
            }
            let distinct = counts.len();
            let duplicated = total_ngrams.saturating_sub(distinct);
            let ratio = duplicated as f64 / total_ngrams as f64;
            if ratio > DEGENERATION_NGRAM_REPEAT_RATIO {
                state.ngram_hits = state.ngram_hits.saturating_add(1);
                if state.ngram_hits >= NGRAM_HIT_LIMIT {
                    // 2026-08-22 审查处理（P3）+ 2026-08-23 校准：3-gram
                    // 触发无「重复 span」语义——清掉本 delta 内滚动路径
                    // 可能留下的子门槛命中上下文，避免 trip WARN 把子
                    // 门槛 span 误标为该触发的审计上下文（3-gram 的审计
                    // 口径是重复率本身，见 detail）。
                    *trigger_context = None;
                    *trip = Some(format!(
                        "{detail_prefix} 3-gram repetition ratio {ratio:.3} \
                         in the recent {} {family} tokens, {}/{} stream hits",
                        state.window_tokens.len(),
                        state.ngram_hits,
                        NGRAM_HIT_LIMIT
                    ));
                } else {
                    // 1–14 次命中：仅审计留痕——审计内容 = ratio + 窗口
                    // token 数 + 族（3-gram 无「重复 span」语义，不复用
                    // span 上下文；设计 §2.2）。
                    audit_hits.push(format!(
                        "3-gram repetition ratio {ratio:.3} in the recent {} \
                         {family} tokens (stream hit {}/{}; audit only, not \
                         tripping)",
                        state.window_tokens.len(),
                        state.ngram_hits,
                        NGRAM_HIT_LIMIT
                    ));
                }
            }
        }
    }
}

/// content 族退化中断 detail 判定（语义收窄为 content 族「不重试」判定，
/// 设计 §3.3——已见输出，ADR-0007 纪律；limit 前缀复读两族共享，达限转
/// run_invalidated）。`stream_once_with_retry` / run 层以
/// `is_degeneration_detail || is_reasoning_guard_detail` 合并使用。
pub(crate) fn is_degeneration_detail(detail: &str) -> bool {
    detail.starts_with(CONTENT_REPETITION_DETAIL_PREFIX)
        || detail.starts_with(DEGENERATION_LIMIT_PREFIX)
}

/// reasoning 族退化中断 detail 判定（设计 §3.3 + 2026-08-28 R1：
/// reasoning_repetition——无可见输出，不原样重试、显式拦截该轮；不再
/// 降档）。
pub(crate) fn is_reasoning_guard_detail(detail: &str) -> bool {
    detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX)
}

/// 审计/日志用触发族标签（detail 可能被 limit 前缀包裹，用 contains 判定
/// 内部族；仅留痕、不改终止语义）。
pub(crate) fn guard_family_label(detail: &str) -> &'static str {
    if detail.contains(CONTENT_REPETITION_DETAIL_PREFIX) {
        "content_repetition"
    } else if detail.contains(REASONING_REPETITION_DETAIL_PREFIX) {
        "reasoning_repetition"
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
    /// 退化中断计数（Arc 共享——Clone 不复制计数）。STALL-DEGENERATION-
    /// FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计 §2.2)：**run 内单调
    /// 递增、成功请求不再清零**（仅 run 边界重置=新 transport）。达到
    /// `DEGENERATION_LIMIT` 后下一次退化中断带 `degeneration_limit_reached`
    /// 标记，run 层据此记 run_invalidated（防会话级循环；设计 §3.3）。
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
        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.6): 默认档 EnabledMax
        // （用户裁决——官方 82.7% 基线即 max effort）；`ORZ_THINKING_MODE`
        // （max|high|low|disabled）env 覆盖以便 A/B 与回退。无效值静默
        // 回退默认档（fail-safe，不会误解成 disabled）。
        let thinking = Self::thinking_mode_from_env().unwrap_or(ThinkingMode::EnabledMax);
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
            // 一次性指纹变化（既有纪律）。THIN-HARNESS-REDESIGN R1
            // (2026-08-27, §4.6): 默认 thinking 档 max（官方 82.7% 基线，
            // 用户裁决）；`ORZ_THINKING_MODE` env 可覆盖（A/B 与回退）；
            // 2026-08-28 R1：复读哨兵命中不再改 thinking 档位（显式拦截
            // 该轮），thinking 只由显式配置与空响应链（max/high → low 封
            // 顶）决定。
            max_tokens: 256_000,
            retry: Default::default(),
            thinking,
        })
    }

    /// THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.6): resolve
    /// `ORZ_THINKING_MODE` (max|high|low|disabled). Missing/invalid →
    /// `None` (caller falls back to the configured default).
    pub fn thinking_mode_from_env() -> Option<ThinkingMode> {
        match std::env::var("ORZ_THINKING_MODE").ok().as_deref() {
            Some("max") => Some(ThinkingMode::EnabledMax),
            Some("high") => Some(ThinkingMode::EnabledHigh),
            Some("low") => Some(ThinkingMode::EnabledLow),
            Some("disabled") => Some(ThinkingMode::Disabled),
            _ => None,
        }
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
            // 2026-08-25 (initial-package trial): v4-flash sometimes omits
            // the field entirely on text-only completions — normalize to
            // Some("") so the assistant replay never drops the field
            // (D-6: omission 400s on the next request).
            reasoning_content: Some(message.reasoning_content.clone().unwrap_or_default()),
            // D-6 usage observation: reasoning_tokens + completion_tokens
            // feed budget/latency calibration (the single-round budget
            // decision rolls back on the data).
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
            // 非流式 `generate` 无 transport 重试摘要（重试仅存在于流式
            // 路径）。
            transport_retry: TransportRetryInfo::default(),
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

    /// GAP-STREAM-RETRY (2026-08-12) + MIDSTREAM-DECODE-RETRY (2026-08-21,
    /// ADR-0010 §14.37 / 设计 §2.1)：mark a streaming failure retryable.
    /// The retry criterion is "no complete tool call decoded" — NOT "no
    /// chunk": once a tool call has a full identity + valid JSON arguments,
    /// output existed and re-sending could duplicate it (ADR-0007 §3
    /// known-boundary premise; double-insurance even though the error path
    /// never executes tool calls). Failures without a complete tool call
    /// are idempotent to re-send — zero-chunk AND midstream (already-seen
    /// content/reasoning chunks) are both wrapped, with `saw_chunk`
    /// distinguishing the retry budget (zero-chunk 180s/10 vs midstream 1).
    /// Only wire-level failures (transport / timeout) are wrapped: model
    /// rejections (permanent, e.g. 400 — the fork's ApiError carries no
    /// HTTP status, so 429/5xx cannot be classified reliably on this path)
    /// and parse errors are never retried, and a user cancel must never be
    /// re-sent.
    fn wrap_no_tool_side_effects(
        &self,
        saw_chunk: bool,
        saw_complete_tool_calls: bool,
        e: GatewayError,
    ) -> GatewayError {
        if saw_complete_tool_calls {
            return e;
        }
        match e {
            GatewayError::Transport(detail) | GatewayError::Timeout(detail) => {
                GatewayError::StreamInterrupted {
                    attempts: 0,
                    saw_chunk,
                    detail,
                }
            }
            other => other,
        }
    }

    /// Whether the accumulated stream state holds at least one COMPLETE
    /// tool call — identity (call_id + name) established AND arguments
    /// parse as valid JSON. Used only on ERROR paths to decide retryability
    /// (MIDSTREAM-DECODE-RETRY 设计 §2.1 双保险边界).
    ///
    /// 边界取「成功路径可分发的最小保守上界」：成功路径对非 JSON
    /// arguments 容错为 `Value::Null` 后仍会分发工具（参数在工具校验处
    /// 失败、无副作用）；本判定把非 JSON arguments 视为「未完整」→
    /// 可重试。方向安全（Null 参数分发无副作用），且比成功路径更保守。
    fn has_complete_tool_call(tool_calls: &[(u32, StreamToolCall)]) -> bool {
        tool_calls.iter().any(|(_, t)| {
            !t.call_id.is_empty()
                && !t.name.is_empty()
                && serde_json::from_str::<Value>(&t.arguments).is_ok()
        })
    }

    /// GAP-STREAM-RETRY (2026-08-12) + MIDSTREAM-DECODE-RETRY (2026-08-21,
    /// ADR-0010 §14.37): one logical stream attempt with bounded
    /// interruption retry. `stream_once` is the raw single connection; this
    /// wrapper re-sends the IDENTICAL request body when the attempt failed
    /// before any complete tool call existed (no executable model output —
    /// idempotent). Two budgets by interruption class:
    ///   - zero-chunk (no decoded chunk): `request_max_retries` (10) AND
    ///     the backoff window (`request_retry_window`, 180s) — either bound
    ///     ends the chain first (ADR-0010 §14.36).
    ///   - midstream (chunks decoded, no complete tool call): at most
    ///     `CHUNKED_MIDSTREAM_MAX_RETRIES` (1) extra re-send — bounded to
    ///     avoid re-burning a partially-consumed budget (设计 §2.2).
    ///   - mixed sequences: a midstream retry whose retried attempt then
    ///     fails ZERO-chunk enters the zero-chunk budget (fresh class —
    ///     the retried request produced no output at all, so re-sending
    ///     stays side-effect-free; the shared 180s backoff window still
    ///     bounds the whole chain). 设计 §2.2 的「中段至多 1 次」按中断
    ///     类别计，混合序列的总重发可超 1，属于有界延伸。
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
        let mut chunked_retries: u32 = 0;
        // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.3)：本次逻辑请求的重试可观测摘要——成功路径随 response
        // 返回，run 层记录 `transport_retry{outcome: recovered}`。
        let mut retry_info = TransportRetryInfo::default();
        loop {
            match self
                .stream_once(request, thinking, cancel, heartbeat, on_chunk)
                .await
            {
                Ok(mut response) => {
                    response.transport_retry = retry_info;
                    return Ok(response);
                }
                // OUTPUT-DEGENERATION-GUARD (2026-08-19) + OUTPUT-BUDGET-
                // RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35):
                // 输出健康哨兵中断绝不 zero-chunk 重试——content 族已见
                // 输出（重发会重复工具调用，ADR-0007 纪律）；reasoning 族
                // 无可见输出但长烧型原样重试大概率复现且贵（由 D-6 链直接
                // 降级，§3.2）。直接透传，attempts 保持 0。
                Err(GatewayError::StreamInterrupted {
                    attempts, detail, ..
                }) if is_degeneration_detail(&detail) || is_reasoning_guard_detail(&detail) => {
                    tracing::warn!(
                        "stream interrupted by the output-health guard — not zero-chunk \
                         retried: {detail}"
                    );
                    return Err(GatewayError::StreamInterrupted {
                        attempts,
                        saw_chunk: false,
                        detail,
                    });
                }
                Err(GatewayError::StreamInterrupted {
                    detail, saw_chunk, ..
                }) => {
                    if saw_chunk {
                        if chunked_retries >= CHUNKED_MIDSTREAM_MAX_RETRIES {
                            tracing::warn!(
                                "stream midstream interruption: chunked retry cap reached \
                                 ({chunked_retries} retries), giving up: {detail}"
                            );
                            return Err(GatewayError::StreamInterrupted {
                                attempts,
                                saw_chunk,
                                detail,
                            });
                        }
                        chunked_retries += 1;
                    } else if attempts >= policy.request_max_retries {
                        tracing::warn!(
                            "stream zero-chunk interruption: retry cap reached ({attempts} retries), giving up: {detail}"
                        );
                        return Err(GatewayError::StreamInterrupted {
                            attempts,
                            saw_chunk,
                            detail,
                        });
                    }
                    let Some(delay) = backoff.next_backoff() else {
                        tracing::warn!(
                            "stream {} interruption: retry window exhausted, giving up: {detail}",
                            if saw_chunk { "midstream" } else { "zero-chunk" }
                        );
                        return Err(GatewayError::StreamInterrupted {
                            attempts,
                            saw_chunk,
                            detail,
                        });
                    };
                    retry_info.retries += 1;
                    retry_info.kind = Some(if saw_chunk {
                        TransportRetryKind::Midstream
                    } else {
                        TransportRetryKind::ZeroChunk
                    });
                    retry_info.reason = Some(detail.clone());
                    let (budget_label, budget_total) = if saw_chunk {
                        ("midstream (chunked budget)", CHUNKED_MIDSTREAM_MAX_RETRIES)
                    } else {
                        ("zero-chunk", policy.request_max_retries)
                    };
                    tracing::warn!(
                        "stream {budget_label} interruption (retry {} of {budget_total}): retrying in {delay:?}: {detail}",
                        retry_info.retries,
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
            self.wrap_no_tool_side_effects(
                false,
                false,
                GatewayError::Timeout(
                    "stream handshake hung — no response headers within the idle timeout"
                        .to_string(),
                ),
            )
        })?
        .map_err(|e| self.wrap_no_tool_side_effects(false, false, self.map_error(e)))?;

        // GAP-STREAM-RETRY (2026-08-12): a stream attempt that produced NO
        // complete tool call before failing is retryable (see
        // `wrap_no_tool_side_effects`). `saw_chunk` (any successfully
        // decoded SSE item — reasoning-only deltas included) distinguishes
        // the zero-chunk vs midstream retry budget; a COMPLETE tool call is
        // the double-insurance non-retry boundary (MIDSTREAM-DECODE-RETRY
        // 设计 §2.1).
        let mut saw_chunk = false;
        let mut saw_complete_tool_calls = false;
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
                        return Err(self.wrap_no_tool_side_effects(
                            saw_chunk,
                            saw_complete_tool_calls,
                            GatewayError::Timeout(format!(
                                "stream idle: no data for {idle_timeout:?}"
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(total_deadline) => {
                        return Err(self.wrap_no_tool_side_effects(
                            saw_chunk,
                            saw_complete_tool_calls,
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
                        return Err(self.wrap_no_tool_side_effects(
                            saw_chunk,
                            saw_complete_tool_calls,
                            GatewayError::Timeout(format!(
                                "stream idle: no data for {idle_timeout:?}"
                            )),
                        ));
                    }
                    _ = tokio::time::sleep_until(total_deadline) => {
                        return Err(self.wrap_no_tool_side_effects(
                            saw_chunk,
                            saw_complete_tool_calls,
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
            let chunk: CreateChatCompletionStreamResponse = item.map_err(|e| {
                self.wrap_no_tool_side_effects(
                    saw_chunk,
                    saw_complete_tool_calls,
                    self.map_error(e),
                )
            })?;
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
            // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37)：累积
            // 状态一旦出现完整 tool call（call_id + name + 合法 JSON
            // arguments ——成功路径可分发的最小保守上界，见
            // `has_complete_tool_call`），后续任何失败都不得重发
            // （双保险边界，设计 §2.1）。
            if Self::has_complete_tool_call(&tool_calls) {
                saw_complete_tool_calls = true;
            }
            // OUTPUT-DEGENERATION-GUARD (2026-08-19) + OUTPUT-BUDGET-
            // RESTORE-AND-STALL-GUARD (2026-08-20, ADR-0010 §14.35): 预算
            // 兜底层逐 chunk 检查（stall 时间/预算 OR 触发）后，命中任一
            // 检测阈值 → 主动中断。防循环计数：会话级连续退化次数达到
            // `DEGENERATION_LIMIT` 后带 `degeneration_limit_reached` 标记
            // （run 层映射 run_invalidated）。
            // 2026-08-22 审查处理（P3）：先取走非触发命中审计条目、再做
            // 触发判定——若本 chunk 触发（滚动路径达门槛、或 3-gram/stall
            // 路径随后触发），审计条目并入下方 trip WARN 的
            // trigger_context，不再单独输出「audit only, not tripping」
            // （避免同 chunk 先标不触发、随即又触发的误导文案）；未触发
            // 时逐条 WARN 留痕（缺口 A，2026-08-22 再校准 + 2026-08-28
            // R1：1–19 次命中仅审计、不中断不降档）。
            let audit_hits = degeneration.take_audit_hits();
            if let Some(detail) = degeneration.trip_reason() {
                let detail = self.degeneration_interrupt_detail(detail);
                // 缺口 A (2026-08-22)：退化触发内容留痕——WARN 输出触发
                // span + 窗口片段（content/reasoning 两族共用），供事后
                // 判定真复读 vs 误杀；不改终止语义（detail 仍为稳定前缀）。
                if let Some(ctx) = degeneration.trigger_context() {
                    tracing::warn!(
                        detail = %detail,
                        trigger_context = %ctx,
                        "output-health guard trip (audit context)"
                    );
                } else {
                    tracing::warn!(detail = %detail, "output-health guard trip");
                }
                // P3-1（2026-08-23 审查处理）：触发 chunk 内已取走的审计
                // 条目同样留痕——此前 trip 分支先 return，同 chunk 的
                // 二级不过候选/子门槛确认命中被丢弃，与「二级不过的候选
                // 也留痕（span + 标点块统计）供事后判定」不符。逐条 WARN
                // 输出（rejected 条目自带 `[second-stage rejected: ...]`
                // 与计数标记；子门槛确认条目即 span 上下文）；文案不复用
                // 「not tripping」，避免同 chunk 先标不触发再触发的误导。
                for ctx in audit_hits {
                    tracing::warn!(
                        rolling_hit_limit = REPETITION_HIT_LIMIT,
                        ngram_hit_limit = NGRAM_HIT_LIMIT,
                        trigger_context = %ctx,
                        "output-health guard repetition audit (same chunk as trip)"
                    );
                }
                // 哨兵中断的 `saw_chunk` 恒为 false：哨兵错误一律不重试
                // （`stream_once_with_retry` 的 guard 透传分支），该字段
                // 只用于选择零 chunk/中段预算，哨兵路径不消费——语义
                // 上「已见 chunk 但被哨兵截停」不进入任何重试预算。
                return Err(GatewayError::StreamInterrupted {
                    attempts: 0,
                    saw_chunk: false,
                    detail,
                });
            }
            // 未触发：流内 1–19 次命中仅审计留痕——逐条 WARN 输出触发
            // span + 窗口片段（缺口 A），不中断、不降档；第 20 次命中才
            // 走上方 trip 分支（2026-08-22 再校准 + 2026-08-23 二级确认 +
            // 2026-08-28 R1 统一门槛 20；二级不过的候选同样在此留痕）。
            // 2026-08-23 审查处理（I1）：审计条目可能来自路径①滚动哈希或
            // 3-gram 路径②——hit 门槛字段按路径分别标注（rolling/ngram），
            // 避免 NGRAM_HIT_LIMIT 日后独立调整时 3-gram 条目被误标。
            for ctx in audit_hits {
                tracing::warn!(
                    rolling_hit_limit = REPETITION_HIT_LIMIT,
                    ngram_hit_limit = NGRAM_HIT_LIMIT,
                    trigger_context = %ctx,
                    "output-health guard repetition hit (audit only, not tripping)"
                );
            }
        }

        // D1-1: a stream that ended without a finish_reason was truncated —
        // surfacing it as an error keeps the journal honest (no half-answer
        // recorded as a completed `stop`).
        if !saw_finish_reason {
            return Err(self.wrap_no_tool_side_effects(
                saw_chunk,
                saw_complete_tool_calls,
                GatewayError::Transport("stream ended without finish_reason".to_string()),
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
            // 2026-08-25 (initial-package trial): v4-flash text-only
            // completions can omit reasoning_content deltas entirely; keep
            // the replay wire shape stable with Some("") (D-6: omission
            // 400s on the next request).
            Some(String::new())
        };

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
            // transport 重试摘要由 `stream_once_with_retry` 在成功返回前
            // 填装；`stream_once` 本身恒为无重试（默认）。
            transport_retry: TransportRetryInfo::default(),
        })
    }

    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33) +
    /// STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37)：退化
    /// 中断的防循环计数与 detail 标记——run 内单调递增（成功请求不再
    /// 清零，设计 §2.2.2；仅 run 边界重置=新 transport）。达到
    /// `DEGENERATION_LIMIT` 后以 `degeneration_limit_reached` 前缀标记
    /// （run 层映射 `run_invalidated`），否则 `degeneration_detected`
    /// （run_failed 同路径）。
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

/// Next lower tier for the D-6 empty-response ladder (2026-08-28
/// THIN-HARNESS-REDESIGN V2 R1 用户裁决): **max/high → low → 失败**——
/// 空响应链最多降到 low、**不关闭 thinking**；low 档仍空即明确失败。
/// `ThinkingMode::Disabled` 只保留为显式手动 A/B 配置（`ORZ_THINKING_MODE`
/// env），自动链永不进入 disabled。
fn next_degraded_thinking(thinking: ThinkingMode) -> Option<ThinkingMode> {
    match thinking {
        ThinkingMode::EnabledHigh => Some(ThinkingMode::EnabledLow),
        ThinkingMode::EnabledLow => None,
        ThinkingMode::EnabledMax => Some(ThinkingMode::EnabledLow),
        ThinkingMode::Disabled => None,
    }
}

#[async_trait]
impl ModelGateway for DeepSeekTransport {
    fn config_fingerprint(&self) -> String {
        self.config_fingerprint_impl()
    }

    /// STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计
    /// §2.2)：per-run 隔离——退化计数全新（配置复制、健康状态归零；
    /// 2026-08-28 R1 起无会话 thinking 档位）。长驻进程（ACP server）
    /// 跨 run 共享原 transport，run 边界由控制器 `for_new_run` 显式换新。
    fn for_new_run(&self) -> Arc<dyn ModelGateway> {
        Arc::new(DeepSeekTransport::new(self.config.clone()))
    }

    async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
        // D-6 empty-content retry chain: an enabled thinking tier can
        // legitimately burn the whole budget on reasoning_content and leave
        // content empty (finish=length, zero output). The chain (2026-08-28
        // THIN-HARNESS-REDESIGN V2 R1 用户裁决):
        //   1. current tier (config default) with fast bounded retries
        //      ≤ `EMPTY_RESPONSE_MAX_RETRIES`;
        //   2. still empty → next lower tier (max/high → low; low → None);
        //   3. low tier still empty → explicit termination reason, never a
        //      silent blank, never an automatic disabled tier.
        // Tool rounds (empty content + tool_calls) are legal and skip the
        // chain entirely.
        let mut thinking = self.effective_thinking(&request);
        loop {
            for _ in 0..=EMPTY_RESPONSE_MAX_RETRIES {
                let response = self.create_once(&request, thinking).await?;
                if !Self::empty_content_abnormal(&response) {
                    return Ok(response);
                }
            }
            let Some(next) = next_degraded_thinking(thinking) else {
                return Err(GatewayError::Model(
                    "budget exhausted with zero output — D-6 chain reached the \
                     low tier (never disabled automatically) and still produced \
                     empty content"
                        .to_string(),
                ));
            };
            thinking = next;
        }
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
        // §14.35 / 设计 §3.2 + §3.6 —— 官方化收窄版 + 2026-08-28 R1
        // low 封顶降级梯):
        //   1. normal request (thinking per config, default max + 256K);
        //   2. completed EMPTY response (`empty_content_abnormal`) → fast
        //      bounded retry within the current tier: same params, backoff
        //      500ms→10s + 10% jitter, ≤ `EMPTY_RESPONSE_MAX_RETRIES`
        //      (official rhythm, narrowed from 5 — 256K 下 5 次原样重试
        //      成本不可接受);
        //   3. still empty → next lower tier: **max/high → low → 失败**
        //      （2026-08-28 R1 用户裁决：最多降到 low、不关闭；low 档
        //      仍空即明确失败，自动链永不进入 disabled）；
        //   4. `ThinkingMode::Disabled`（仅手动 `ORZ_THINKING_MODE` 配置）
        //      空响应立即显式失败（无更低档）。
        // Reasoning-family guard interruptions (reasoning_repetition) do
        // NOT re-run the identical request and NO LONGER step down the
        // ladder (2026-08-28 R1：触发改显式拦截该轮、不重试、不降档)；
        // content-family interruptions keep the no-retry passthrough
        // (ADR-0007 已见输出不重试)。Zero-chunk transport interruptions
        // are handled inside `stream_once_with_retry` and are orthogonal
        // to this chain.
        let mut thinking = self.effective_thinking(&request);
        let mut empty_retries: u32 = 0;
        let mut backoff = empty_response_backoff();
        // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.3)：D-6 空响应链多次调用 `stream_once_with_retry`——transport
        // 重试摘要跨调用累积，最终成功响应携带全量计数。
        let mut total_retry_info = TransportRetryInfo::default();
        loop {
            match self
                .stream_once_with_retry(&request, thinking, cancel, heartbeat, on_chunk)
                .await
            {
                Ok(response) => {
                    let is_empty = Self::empty_content_abnormal(&response);
                    // 累加本次调用的 transport 重试摘要（跨 D-6 迭代）；
                    // kind/reason 取最后一次实际重试的类别/原因。
                    total_retry_info.retries += response.transport_retry.retries;
                    if let Some(kind) = response.transport_retry.kind {
                        total_retry_info.kind = Some(kind);
                        total_retry_info.reason = response.transport_retry.reason.clone();
                    }
                    if is_empty {
                        // 完成型空响应 → 落入下方 D-6 链（快速有界重试/
                        // 降级），本次重试摘要已并入 total_retry_info。
                        // 手动 disabled 配置（仅 `ORZ_THINKING_MODE`）无更低
                        // 档——空响应立即显式失败。
                        if thinking == ThinkingMode::Disabled {
                            return Err(GatewayError::Model(
                                "budget exhausted with zero output — D-6 chain \
                                 (manual disabled tier, no lower tier) produced \
                                 empty content"
                                    .to_string(),
                            ));
                        }
                        // 当前档位快速重试耗尽 → 逐级下降（max/high → low；
                        // low 已无更低档 → 显式失败）；换档时重试计数与退避
                        // 重置（每档独立「快速 ≤2 次」节奏，设计 §3.2/§3.6；
                        // 2026-08-28 R1：不降到 disabled）。
                        if empty_retries >= EMPTY_RESPONSE_MAX_RETRIES {
                            let Some(next) = next_degraded_thinking(thinking) else {
                                return Err(GatewayError::Model(
                                    "budget exhausted with zero output — D-6 chain \
                                     reached the low tier (never disabled \
                                     automatically) and still produced empty content"
                                        .to_string(),
                                ));
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
                                return Err(GatewayError::Model(
                                    "budget exhausted with zero output — D-6 chain \
                                     reached the low tier (never disabled \
                                     automatically) and still produced empty content"
                                        .to_string(),
                                ));
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
                        continue;
                    }
                    let mut response = response;
                    response.transport_retry = total_retry_info.clone();
                    return Ok(response);
                }
                Err(GatewayError::StreamInterrupted { detail, .. })
                    if detail.starts_with(DEGENERATION_LIMIT_PREFIX) =>
                {
                    // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010
                    // §14.37 / 设计 §2.2.2)：退化计数已达
                    // `DEGENERATION_LIMIT` —— **不论当前 thinking 档位**
                    // 立即显式终止（detail 已带
                    // `degeneration_limit_reached` 前缀，run 层映射
                    // run_invalidated）。独立分支把「达限即终止」从
                    // 档位判定与分支顺序中解耦，避免未来梯级改动时
                    // 静默破坏。
                    tracing::warn!(
                        "degeneration limit reached — terminating the run explicitly: {detail}"
                    );
                    return Err(GatewayError::StreamInterrupted {
                        attempts: 0,
                        saw_chunk: false,
                        detail,
                    });
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
            // RS-05 (0aq, 2026-09-19, Top-10 #7)：刚 push 的条目必然在尾
            // 部（不变量显式化）。
            tool_calls
                .last_mut()
                .expect("tool call entry was just pushed")
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
                round: None,
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
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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
    fn build_request_default_thinking_is_enabled_max() {
        // D-6 (FIX_PLAN 2026-08-06) + THIN-HARNESS-REDESIGN R1
        // (2026-08-27, §4.6, 用户裁决): the DEFAULT workpoint is thinking
        // enabled + reasoning_effort "max" (the official 82.7% baseline) +
        // 256K single-round budget. The 256K value is the whole-token
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
        assert_eq!(json["reasoning_effort"], "max", "{s}");
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

    /// R1 (§4.6): `ORZ_THINKING_MODE` overrides the default max tier;
    /// invalid values fall back to the default (never disabled).
    #[test]
    fn env_thinking_mode_override() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let original = std::env::var("ORZ_THINKING_MODE").ok();
        unsafe {
            std::env::set_var("ORZ_THINKING_MODE", "high");
        }
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let json = serde_json::to_value(t.build_request(&request())).unwrap();
        assert_eq!(json["reasoning_effort"], "high", "{json}");
        unsafe {
            std::env::set_var("ORZ_THINKING_MODE", "disabled");
        }
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let json = serde_json::to_value(t.build_request(&request())).unwrap();
        assert_eq!(json["thinking"]["type"], "disabled", "{json}");
        assert!(!json.to_string().contains("reasoning_effort"), "{json}");
        unsafe {
            std::env::set_var("ORZ_THINKING_MODE", "bogus");
        }
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        let json = serde_json::to_value(t.build_request(&request())).unwrap();
        assert_eq!(json["reasoning_effort"], "max", "{json}");
        // Restore the original env (parallel-test safety).
        match original {
            Some(v) => unsafe { std::env::set_var("ORZ_THINKING_MODE", v) },
            None => unsafe { std::env::remove_var("ORZ_THINKING_MODE") },
        }
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
    fn degeneration_detector_trips_on_repeated_span() {
        // 2026-08-21 设计 §3.3 修订：路径①（连续相同 delta N=5）由滚动
        // 哈希任意偏移取代；2026-08-22 再校准：L=200 + 流内累计命中
        // ——5×"same"（20 字符）与 24×"same"（96 字符 < L）均不再触发；
        // 2026-08-23 再校准：L=400 + 二级确认——400 字符 span 出现 3 次
        // 命中即触发；2026-08-28 R1（THIN-HARNESS-REDESIGN V2）：统一门槛
        // 20，同字符连串改由独立 802 线直接触发（不再由 3 次滚动命中派生）。
        let mut d = DegenerationDetector::default();
        for i in 0..23 {
            d.feed_content("same");
            assert!(
                d.trip_reason().is_none(),
                "short identical deltas must NOT trip (low-entropy false-positive fix), feed {i}"
            );
        }
        d.feed_content("same"); // 96 字符 < L=400，无命中
        assert!(
            d.trip_reason().is_none(),
            "96 chars below L=400 must NOT trip"
        );
        d.feed_content(&"x".repeat(802));
        let reason = d
            .trip_reason()
            .expect("802-char same-character run must trip");
        assert!(
            reason.starts_with(CONTENT_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("same-character run"), "{reason}");
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

    // ── S2（2026-08-21 设计 §4.8/§4.9：滚动哈希任意偏移测试批次）───────

    /// 确定性伪随机互异内容（宽字母表；单一大 chunk 用例）。
    fn distinct_random_text(len: usize, seed: u64) -> String {
        const ALPHABET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+[]{};:,.<>?/";
        let mut state = seed;
        let mut out = String::with_capacity(len);
        while out.len() < len {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            out.push(ALPHABET[((state >> 33) as usize) % ALPHABET.len()] as char);
        }
        out
    }

    /// 确定性伪随机无标点互异内容（字母+数字，无空格/标点——二级「无切分
    /// 点直接判真」路径的精确复读用例）。
    fn distinct_alpha_text(len: usize, seed: u64) -> String {
        const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut state = seed;
        let mut out = String::with_capacity(len);
        while out.len() < len {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            out.push(ALPHABET[((state >> 33) as usize) % ALPHABET.len()] as char);
        }
        out
    }

    /// 确定性伪随机 DNA 低熵样本（ACGT + 散点低熵短特征；6439 字符与
    /// dna-assembly 误杀样本同量级，设计 §3.3/§4.8）。
    fn dna_like_text(len: usize, seed: u64) -> String {
        const BASES: &[u8] = b"ACGT";
        let mut state = seed;
        let mut out = String::with_capacity(len);
        let mut i = 0usize;
        while i < len {
            let feature = match i {
                100..=104 => Some("ttttt"),
                500..=504 => Some("aaaaa"),
                900..=904 => Some("ggggg"),
                1_300..=1_308 => Some("N N N N N"),
                1_700..=1_705 => Some("GGTCTC"),
                _ => None,
            };
            if let Some(f) = feature {
                let take = f.len().min(len - i);
                out.push_str(&f[..take]);
                i += take;
                continue;
            }
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            out.push(BASES[((state >> 33) as usize) % BASES.len()] as char);
            i += 1;
        }
        out
    }

    /// final-smoke-2026-08-25 dna-assembly 误杀实证捕获的真实 EGFP 编码
    /// 区 400 字符 span（orz.txt WARN trigger_context，匹配偏移 18/801；
    /// 全小写 ACGT。2026-08-25 为序列内容门目标样本；2026-08-28 R1 序列
    /// 门删除后作为统一门槛 20 的真实回放回归样本）。源证据=
    /// `D:\tb-eval\jobs-official\final-smoke-2026-08-25\dna-assembly__sHQCjg3\agent\orz.txt`。
    const EGFP_SPAN_400: &str = "tgagcaagggcgaggagctgttcaccggggtggtgcccatcctggtcgagctggacggcgacgtaaacggccacaagttcagcgtgtccggcgagggtgagggcgatgccacctacggcaagctgaccctgaagttcatctgcaccacgggcaagctgcccgtgccctggcccaccctcgtgaccaccctgacctacggcgtgcagtgcttcagccgctaccccgaccacatgaagcagcacgacttcttcaagtccgccatgcccgaaggctacgtccaggagcgcaccatcttcttcaaggacgacggcaactacaagacccgcgccgaggtgaagttcgagggcgacaccctggtgaaccgcatcgagctgaagggcatcgacttcaaggaggacgg";

    /// 真实蛋白序列 400 字符 span（2026-08-26 全面审查处理：蛋白/氨基酸
    /// 序列覆盖扩展目标样本）。源证据=UniProt P35579（人类 myosin-9，
    /// 1960 aa）第 1–400 位。2026-08-28 R1 序列门删除后作为统一门槛 20
    /// 的真实回放回归样本。
    const PROTEIN_SPAN_400: &str = "MAQQAADKYLYVDKNFINNPLAQADWAAKKLVWVPSDKSGFEPASLKEEVGEEAIVELVENGKKVKVNKDDIQKMNPPKFSKVEDMAELTCLNEASVLHNLKERYYSGLIYTYSGLFCVVINPYKNLPIYSEEIVEMYKGKKRHEMPPHIYAITDTAYRSMMQDREDQSILCTGESGAGKTENTKKVIQYLAYVASSHKSKKDQGELERQLLQANPILEAFGNAKTVKNDNSSRFGKFIRINFDVNGYIVGANIETYLLEKSRAIRQAKEERTFHIFYYLLSGAGEHLKTDLLLEPYNKYRFLSNGHVTIPGQQDKDMFQETMEAMRIMGIPEEEQMGLLRVISGVLQLGNIVFKKERNTDQASMPDNTAAQKVSHLLGINVTDFTRGILTPRIKVGRDY";

    #[test]
    fn degeneration_detector_short_low_entropy_deltas_do_not_trip() {
        // S2（设计 §4.8）：短低熵块不触发——5×"a"（5 字符）与 5×"same"
        // （20 字符）在旧路径①（连续 5 相同 delta）下命中，滚动哈希粒度
        // 下天然免疫（远低于 48 字符 L-gram）。
        let mut d = DegenerationDetector::default();
        for _ in 0..5 {
            d.feed_content("a");
        }
        for _ in 0..5 {
            d.feed_content("same");
        }
        assert!(
            d.trip_reason().is_none(),
            "short low-entropy identical deltas must NOT trip"
        );
    }

    #[test]
    fn degeneration_detector_poly_a_threshold() {
        // S2（设计 §3.3）+ 2026-08-22/23 再校准：poly-A 精确阈值——799 同
        // 字符不触发（无命中）；800 = 1 次命中（仅审计）；801 = 2 次命中
        // （仅审计）。2026-08-28 R1（THIN-HARNESS-REDESIGN V2）：统一门槛
        // 20 + 802 独立同字符线——802 由独立线直接触发（无需再凑 20 次
        // 滚动命中，也无需序列门）。
        let mut d = DegenerationDetector::default();
        d.feed_content(&"a".repeat(799));
        assert!(
            d.trip_reason().is_none(),
            "799 identical chars must NOT trip"
        );
        assert!(d.take_audit_hits().is_empty(), "799 chars produce no hit");
        let mut d = DegenerationDetector::default();
        d.feed_content(&"a".repeat(800));
        assert!(
            d.trip_reason().is_none(),
            "800 identical chars = 1 hit, must NOT trip"
        );
        assert_eq!(d.take_audit_hits().len(), 1, "800 chars = 1 audit hit");
        let mut d = DegenerationDetector::default();
        d.feed_content(&"a".repeat(801));
        assert!(
            d.trip_reason().is_none(),
            "801 identical chars = 2 hits, must NOT trip"
        );
        assert_eq!(d.take_audit_hits().len(), 2, "801 chars = 2 audit hits");
        let mut d = DegenerationDetector::default();
        d.feed_content(&"a".repeat(802));
        let reason = d
            .trip_reason()
            .expect("802 identical chars = independent same-char line, must trip");
        assert!(reason.contains("same-character run"), "{reason}");
    }

    #[test]
    fn degeneration_detector_egfp_real_span_three_hits_audit_only() {
        // 2026-08-28 R1（THIN-HARNESS-REDESIGN V2 §3.4）：final-smoke
        // dna-assembly EGFP 400 字符真实 span 原样回放——引用 4 段、中间
        // 插入互异推理文本（离线模拟验证命中 idx 999/1599/2199 = 3 次，
        // 间隔不重置累计）→ 统一门槛 20 下 0 trip（序列门删除后由门槛
        // 20 自然覆盖）；3 条审计（无 `sequence_gated` 标注）。
        assert_eq!(
            EGFP_SPAN_400.chars().count(),
            REPETITION_MIN_RUN_CHARS,
            "EGFP span must be exactly L=400 chars"
        );
        let span = EGFP_SPAN_400;
        let gap1 = distinct_random_text(200, 101);
        let gap2 = distinct_random_text(200, 102);
        let gap3 = distinct_random_text(200, 103);
        let mut d = DegenerationDetector::default();
        d.feed_reasoning(span);
        d.feed_reasoning(&gap1);
        d.feed_reasoning(span);
        d.feed_reasoning(&gap2);
        d.feed_reasoning(span);
        d.feed_reasoning(&gap3);
        d.feed_reasoning(span);
        assert!(
            d.trip_reason().is_none(),
            "3 EGFP replays must NOT trip (unified limit 20)"
        );
        let audits = d.take_audit_hits();
        assert_eq!(audits.len(), 3, "three EGFP replays must be audited");
        assert!(
            audits.iter().all(|a| !a.contains("sequence_gated")),
            "sequence gate markers must be gone: {audits:?}"
        );
    }

    #[test]
    fn degeneration_detector_egfp_real_span_twenty_hits_trips() {
        // 2026-08-28 R1（设计 §3.4）：同一 EGFP span 原样回放 19 次 →
        // 0 trip（仅审计）；第 20 次 → 显式拦截（统一门槛 20，detail 带
        // 20/20；序列门删除后无需 `sequence_gated`）。
        let span = EGFP_SPAN_400;
        let mut d = DegenerationDetector::default();
        d.feed_reasoning(span); // 首次出现，不计命中
        for i in 0..19 {
            d.feed_reasoning(&distinct_random_text(200, 1000 + i));
            d.feed_reasoning(span); // 回放 → 1 次命中
            assert!(
                d.trip_reason().is_none(),
                "{} EGFP replays must NOT trip (unified limit 20)",
                i + 1
            );
        }
        let audits = d.take_audit_hits();
        assert_eq!(audits.len(), 19, "19 replays = 19 audit hits");
        d.feed_reasoning(&distinct_random_text(200, 9999));
        d.feed_reasoning(span); // 第 20 次回放 → 20/20 → trip
        let reason = d
            .trip_reason()
            .expect("20th EGFP replay must trip (unified limit 20)");
        assert!(reason.contains("20/20"), "{reason}");
    }

    #[test]
    fn degeneration_detector_protein_span_three_hits_audit_only() {
        // 2026-08-28 R1：真实蛋白 span（UniProt P35579）合法回显 4 段
        // （3 次命中、间隔不重置）→ 统一门槛 20 下 0 trip（序列门删除后
        // 由门槛 20 覆盖）；3 条审计（无 `sequence_gated`）。
        assert_eq!(
            PROTEIN_SPAN_400.chars().count(),
            REPETITION_MIN_RUN_CHARS,
            "protein span must be exactly L=400 chars"
        );
        let span = PROTEIN_SPAN_400;
        let gap1 = distinct_random_text(200, 201);
        let gap2 = distinct_random_text(200, 202);
        let gap3 = distinct_random_text(200, 203);
        let mut d = DegenerationDetector::default();
        d.feed_reasoning(span);
        d.feed_reasoning(&gap1);
        d.feed_reasoning(span);
        d.feed_reasoning(&gap2);
        d.feed_reasoning(span);
        d.feed_reasoning(&gap3);
        d.feed_reasoning(span);
        assert!(
            d.trip_reason().is_none(),
            "3 protein replays must NOT trip (unified limit 20)"
        );
        let audits = d.take_audit_hits();
        assert_eq!(audits.len(), 3, "three protein replays must be audited");
        assert!(
            audits.iter().all(|a| !a.contains("sequence_gated")),
            "sequence gate markers must be gone: {audits:?}"
        );
    }

    #[test]
    fn degeneration_detector_poly_a_below_run_chars_silent() {
        // 2026-08-25 SEQUENCE CONTENT GATE §3 矩阵项 3：poly-A / 低熵
        // 399/400/401 形态维持不触发——第一级滚动窗口即不命中（记录区
        // 在 idx ≥ W-1 才非空），0 审计、0 trip。
        for n in [399usize, 400, 401] {
            let mut d = DegenerationDetector::default();
            d.feed_reasoning(&"a".repeat(n));
            assert!(
                d.trip_reason().is_none(),
                "{n} identical chars must NOT trip"
            );
            assert!(
                d.take_audit_hits().is_empty(),
                "{n} identical chars must produce no hit"
            );
        }
    }

    #[test]
    fn degeneration_detector_rolling_hits_discarded_at_stream_end() {
        // 2026-08-28 R1（流结束丢弃，序列门删除后同语义）：检测器按
        // generate_stream 每次新建——第一个流 3 次命中仅审计后结束；新建
        // 流重新从 0 计数（非 6 次累计），同一流内第 20 次命中才触发。
        let span = EGFP_SPAN_400;
        let gaps: Vec<String> = (101..=130).map(|s| distinct_random_text(200, s)).collect();
        let mut first = DegenerationDetector::default();
        for i in 0..4 {
            first.feed_reasoning(span);
            if let Some(g) = gaps.get(i) {
                first.feed_reasoning(g);
            }
        }
        assert!(
            first.trip_reason().is_none(),
            "3 hits in stream 1 must NOT trip"
        );
        assert_eq!(first.take_audit_hits().len(), 3);
        // 流结束：丢弃 first；新建流计数从 0 开始。
        let mut second = DegenerationDetector::default();
        for i in 0..4 {
            second.feed_reasoning(span);
            if let Some(g) = gaps.get(i) {
                second.feed_reasoning(g);
            }
        }
        assert!(
            second.trip_reason().is_none(),
            "fresh stream must restart the counter (3/20, not 6 cumulative)"
        );
        assert_eq!(second.take_audit_hits().len(), 3);
        // 同一流内继续回放到第 20 次命中 → 触发。
        for i in 4..22 {
            second.feed_reasoning(span);
            if let Some(g) = gaps.get(i) {
                second.feed_reasoning(g);
            }
            if second.trip_reason().is_some() {
                break;
            }
        }
        let reason = second
            .trip_reason()
            .expect("20th hit in the same stream must trip");
        assert!(reason.contains("20/20"), "{reason}");
    }

    #[test]
    fn degeneration_trigger_context_records_repeated_span() {
        // 缺口 A（2026-08-22）+ 再校准（2026-08-23 L=400）：退化触发时
        // 落盘触发上下文——重复 400 字符 span 文本、两个匹配偏移与窗口
        // 尾部（2026-08-28 R1 统一门槛 20：第 20 次命中时），供事后判定
        // 真复读 vs 误杀。
        let mut d = DegenerationDetector::default();
        d.feed_content(&"abcdefghij".repeat(82)); // 820 字符：idx 799 起 21 次命中 → 20/20 触发
        let ctx = d
            .trigger_context()
            .expect("trigger context must be recorded on trip");
        // 重复 span 是周期 10 循环中的一个 400 字符段（"abcdefghij" 40
        // 个周期；从窗口内某偏移截取），窗口尾部是循环内容本身。
        assert!(
            ctx.contains(&format!("repeated {}-char span", REPETITION_MIN_RUN_CHARS)),
            "{ctx}"
        );
        assert!(ctx.contains("window tail"), "{ctx}");
        // 两个匹配偏移都落在记录区 [400, 800] 内（语义即起点距离 ≥L）。
        assert!(ctx.contains("matched offsets"), "{ctx}");
        // content 族触发详情仍稳定（detail 前缀不变，审计上下文不污染）。
        let reason = d.trip_reason().expect("must trip");
        assert!(
            reason.starts_with("degeneration_detected:content_repetition:"),
            "{reason}"
        );
    }

    #[test]
    fn degeneration_detector_periodic_phrase_trips_any_phase() {
        // S2（设计 §3.3，对齐缺陷回归）+ 2026-08-22 再校准：周期 10 短语
        // 循环——固定偏移下相邻窗口永不相同、系统性漏检；滚动哈希任意
        // 偏移在 idx 799 起连续命中（repeat(82)=820 字符 → 21 次命中，
        // 2026-08-28 R1 统一门槛 20 下触发；无切分点 → 二级直接判真）。
        let mut d = DegenerationDetector::default();
        d.feed_content(&"abcdefghij".repeat(80)); // 800 字符 = 1 次命中（仅审计）
        assert!(d.trip_reason().is_none(), "1 hit must NOT trip");
        assert_eq!(
            d.take_audit_hits().len(),
            1,
            "800 chars of period-10 = 1 audit hit"
        );
        let mut d = DegenerationDetector::default();
        d.feed_content(&"abcdefghij".repeat(82)); // 820 字符 = 21 次命中 ≥ 20
        let reason = d.trip_reason().expect("period-10 cycle must trip");
        assert!(reason.contains("repeated span"), "{reason}");
    }

    #[test]
    fn degeneration_detector_single_large_distinct_chunk_does_not_trip() {
        // S2（设计 §4.8）+ 2026-08-22/23 L=400：单一大 chunk 不触发——一次
        // 喂入 2000 字符互异内容（确定性伪随机，无 400 字符重复 span）。
        let mut d = DegenerationDetector::default();
        d.feed_content(&distinct_random_text(2000, 12345));
        assert!(
            d.trip_reason().is_none(),
            "single large distinct chunk must NOT trip"
        );
        assert!(
            d.take_audit_hits().is_empty(),
            "distinct content must not produce any hit"
        );
    }

    #[test]
    fn degeneration_detector_dna_low_entropy_output_does_not_trip() {
        // S2（设计 §3.3/§4.8）+ 2026-08-22/23 L=400：dna-assembly 误杀回归
        // ——6439 字符低熵 DNA 分析文本（含 ttttt/aaaaa/ggggg/N N N N N/
        // GGTCTC 短特征）不触发；800 字符窗口内出现两个完全相同 400 字符
        // 子串的概率 ≈ 4⁻⁴⁰⁰ 量级。
        let text = dna_like_text(6439, 20_260_821);
        assert_eq!(text.len(), 6439, "sample must match the evidence size");
        let mut d = DegenerationDetector::default();
        d.feed_content(&text);
        assert!(
            d.trip_reason().is_none(),
            "low-entropy DNA-like analysis must NOT trip"
        );
        assert!(
            d.take_audit_hits().is_empty(),
            "DNA low-entropy text must not repeat 400-char spans"
        );
    }

    #[test]
    fn rolling_window_spans_equal_char_level_verification() {
        // S2（设计 §4.8 哈希碰撞比对）+ 2026-08-22/23 L=400：哈希命中后的
        // 字符级比对路径直接验证——相同 span 判等、单字符差异判不等。
        // 真实 u64 多项式哈希碰撞构造不可行（B=1_000_003 奇数、400 位置、
        // 字符字母表，2-adic 差异上界远小于 2^64），登记为已接受边界；
        // `spans_equal` 即本路径的可测部分。
        let span = distinct_random_text(REPETITION_MIN_RUN_CHARS, 42);
        assert_eq!(span.chars().count(), REPETITION_MIN_RUN_CHARS);
        let mut w = RollingRepetitionWindow::default();
        w.feed_chars(&span);
        w.feed_chars(&span);
        assert!(
            w.spans_equal(0, REPETITION_MIN_RUN_CHARS),
            "identical L-char spans must compare equal"
        );
        let mut mutated = span.clone();
        mutated.replace_range(REPETITION_MIN_RUN_CHARS - 1..REPETITION_MIN_RUN_CHARS, "X");
        let mut w = RollingRepetitionWindow::default();
        w.feed_chars(&span);
        w.feed_chars(&mutated);
        assert!(
            !w.spans_equal(0, REPETITION_MIN_RUN_CHARS),
            "single-char difference must compare unequal"
        );
    }

    #[test]
    fn degeneration_detector_near_repeat_does_not_trip_then_exact_repeat_trips() {
        // S2 补充 + 2026-08-22/23 L=400：400 字符 span 与邻近内容仅差一个
        // 字符（近重复，非精确复读）不触发；精确复读按流内累计命中计——
        // 1 次命中仅审计留痕，第 20 次命中才触发（2026-08-28 R1 门槛
        // 20；相邻第三份复制在窗口内产生大量匹配即达 20）。用无标点互异
        // 文本（二级无切分点直接判真，确认路径被激活）。
        let span = distinct_alpha_text(REPETITION_MIN_RUN_CHARS, 7);
        let mut near = span.clone();
        // 突变字符取与 span 首/末字符均不同的字母（避免移位窗口意外相等）。
        let first = span.chars().next().expect("span is non-empty");
        let last = span.chars().last().expect("span is non-empty");
        let mut mutation = 'X';
        for cand in b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789" {
            let c = *cand as char;
            if c != first && c != last {
                mutation = c;
                break;
            }
        }
        near.replace_range(
            REPETITION_MIN_RUN_CHARS - 1..REPETITION_MIN_RUN_CHARS,
            &mutation.to_string(),
        );
        let mut d = DegenerationDetector::default();
        d.feed_content(&span); // 首次出现
        d.feed_content(&near); // 近重复：无精确 L 字符 span 复现
        assert!(
            d.trip_reason().is_none(),
            "near-repeat (1-char diff) must NOT trip"
        );
        assert!(
            d.take_audit_hits().is_empty(),
            "near-repeat must not count as a hit"
        );
        d.feed_content(&span); // 精确复读 → 命中 1（仅审计）
        assert!(d.trip_reason().is_none(), "1 stream hit must NOT trip");
        assert_eq!(
            d.take_audit_hits().len(),
            1,
            "first exact repeat must be audit-only"
        );
        d.feed_content(&span); // 相邻第三份 → 窗口内 400 个匹配位置 → 第 20 次命中触发
        let reason = d.trip_reason().expect("20th stream hit must trip");
        assert!(reason.contains("repeated span"), "{reason}");
    }

    #[test]
    fn degeneration_detector_trips_on_high_repetition_ratio() {
        // >1K tokens with a heavily duplicated 3-gram profile; 2026-08-21
        // 修订：共享核心压到 L 以下 + 互异尾部——任何 L 字符窗口必然包含
        // 变体特有标记，滚动哈希任意偏移路径保持静默（仅 3-gram 重复率
        // 路径是本次被测对象）；2026-08-22 L=200、2026-08-23 L=400 后再
        // 校准：每 feed 追加唯一 4 位标记，杜绝 400 字符 span 精确复现；
        // 2026-08-23 NGRAM-GUARD-CALIBRATION：单份 core 的 3-gram 重复率
        // 约 0.694（恰为 0.69x 边界样本，S2 保留为「不触发」用例）——
        // 本用例改双份 core（ratio ≈ 0.825 > 0.70）+ 流内累计命中 ≥15
        // （2026-08-28 R1 门槛 3→15；窗口填满后前 14 次超阈值 feed 仅
        // 审计留痕，第 15 次才触发）。
        let core = "the quick brown fox jumps over lazy dog the quick brown \
                    fox jumps over lazy dog";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_content(&format!("{core} marker {i:04}"));
        }
        let audits = d.take_audit_hits();
        assert_eq!(
            audits.len(),
            14,
            "fourteen sub-threshold 3-gram hits must be audited, got {audits:?}"
        );
        assert!(
            audits.iter().all(|a| {
                a.contains("3-gram repetition ratio")
                    && a.contains("content tokens")
                    && a.contains("audit only")
            }),
            "audit entries must carry ratio + window + family, got {audits:?}"
        );
        let reason = d.trip_reason().expect("high repetition must trip");
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
        assert!(reason.contains("15/15"), "{reason}");
        assert!(
            d.take_audit_hits().is_empty(),
            "rolling path must stay silent (no 400-char span repeats)"
        );
    }

    #[test]
    fn degeneration_detector_ngram_boundary_below_070_never_hits() {
        // S2（NGRAM-GUARD-CALIBRATION，设计 §2.3）：0.69x 边界样本——
        // 单份 8 词 core 的 3-gram 重复率约 0.694（< 0.70，`>` 严格大于）
        // ——任意 feed 均不计数、永不触发（旧 0.60 阈值下同 profile 会
        // 单发即 trip；0.70 阈值下被边界吸收）。
        let core = "the quick brown fox jumps over lazy dog";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_content(&format!("{core} marker {i:04}"));
        }
        assert!(d.trip_reason().is_none(), "0.694 < 0.70 must never trip");
        assert!(
            d.take_audit_hits().is_empty(),
            "0.694 < 0.70 must never count a hit"
        );
    }

    #[test]
    fn degeneration_detector_ngram_boundary_above_070_trips_after_fifteen_hits() {
        // S2（设计 §2.3）：0.70x 边界样本——9 词 core 的 3-gram 重复率约
        // 0.720（> 0.70）——窗口填满后每次超阈值 feed 计 1 次命中：前
        // 14 次仅审计（1/15…14/15），第 15 次才 trip（15/15）；`>` 严格
        // 大于语义由 0.694（不计数）/ 0.720（计数）两侧明确断言。
        // （2026-08-28 R1：门槛 3→15。）
        let core = "the quick brown fox jumps over lazy dog alpha";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_content(&format!("{core} marker {i:04}"));
        }
        let audits = d.take_audit_hits();
        assert_eq!(
            audits.len(),
            14,
            "fourteen sub-threshold 3-gram hits must be audited, got {audits:?}"
        );
        assert!(audits[0].contains("stream hit 1/15"), "{}", audits[0]);
        assert!(audits[13].contains("stream hit 14/15"), "{}", audits[13]);
        let reason = d
            .trip_reason()
            .expect("0.720 > 0.70 must trip after 15 hits");
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
        assert!(reason.contains("15/15"), "{reason}");
    }

    #[test]
    fn degeneration_detector_ngram_single_feed_counts_one_hit() {
        // S2（设计 §2.2）：命中按 feed 粒度计 1 次——单个超大高重复 feed
        // （约 1200 token、ratio≈0.825）也只计 1 次（审计）、不 trip；
        // 第 2…15 个同类 feed 依次计 2…15 次，第 15 个才触发（2026-08-28
        // R1：门槛 3→15）。滚动路径
        // 保持静默（同字符 span 间隔 6120 字符 > 800 窗口缓冲，无精确
        // 复现）。
        let chunk = {
            let core = "the quick brown fox jumps over lazy dog the quick \
                        brown fox jumps over lazy dog";
            let mut s = String::new();
            for i in 0..60 {
                s.push_str(&format!("{core} marker {i:04} "));
            }
            s
        };
        let mut d = DegenerationDetector::default();
        d.feed_content(&chunk);
        assert!(
            d.trip_reason().is_none(),
            "one huge feed = 1 hit, must not trip"
        );
        let audits = d.take_audit_hits();
        assert_eq!(audits.len(), 1, "one hit audited, got {audits:?}");
        assert!(audits[0].contains("stream hit 1/15"), "{}", audits[0]);
        for feed_no in 2..=14 {
            d.feed_content(&chunk);
            assert!(
                d.trip_reason().is_none(),
                "feed {feed_no} = {feed_no} hits, must not trip"
            );
            let audits = d.take_audit_hits();
            assert_eq!(audits.len(), 1, "one hit per feed, got {audits:?}");
            assert!(
                audits[0].contains(&format!("stream hit {feed_no}/15")),
                "{}",
                audits[0]
            );
        }
        d.feed_content(&chunk);
        let reason = d.trip_reason().expect("15th feed = hit 15, must trip");
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
        assert!(reason.contains("15/15"), "{reason}");
    }

    #[test]
    fn degeneration_detector_ngram_hit_counter_survives_gaps() {
        // S2（设计 §2.2：间隔不重置）：命中 1-2（feed 90/91）后插入大
        // 间隔（单 feed 300 互异 token，窗口重复率压到 0.70 以下、间隔
        // 自身不计命中）——计数不清零（ngram_hits 仍 2）；重新灌入高
        // 重复内容恢复超阈值后继续累计到第 15 次并 trip（15/15；
        // 2026-08-28 R1 门槛 3→15）——若间隔重置，需再累计 15 次才触发
        // 且中间先出 13 条新审计。
        let core = "the quick brown fox jumps over lazy dog alpha";
        let mut d = DegenerationDetector::default();
        for i in 0..92 {
            d.feed_content(&format!("{core} marker {i:04}"));
        }
        assert_eq!(
            d.content_repetition.ngram_hits, 2,
            "fill phase must land exactly 2 hits (feeds 90/91)"
        );
        let audits = d.take_audit_hits();
        assert_eq!(
            audits.len(),
            2,
            "two sub-threshold hits audited, got {audits:?}"
        );
        assert!(d.trip_reason().is_none(), "2 hits must not trip");
        let gap = (0..100)
            .map(|j| format!("g{j:03} w{j:03} t{j:03}"))
            .collect::<Vec<_>>()
            .join(" ");
        d.feed_content(&gap);
        assert_eq!(
            d.content_repetition.ngram_hits, 2,
            "gap feed below threshold must neither hit nor reset the counter"
        );
        assert!(d.trip_reason().is_none(), "gap must not trip");
        assert!(
            d.take_audit_hits().is_empty(),
            "gap feed below threshold must not audit"
        );
        for i in 0..200 {
            d.feed_content(&format!("{core} marker R{i:04}"));
            if d.trip_reason().is_some() {
                break;
            }
        }
        let reason = d.trip_reason().expect("15th hit after gap must trip");
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
        assert!(reason.contains("15/15"), "{reason}");
    }

    #[test]
    fn degeneration_detector_ngram_hits_discarded_at_stream_end() {
        // S2（设计 §2.2：流结束丢弃）：命中计数不跨请求累积——新流首
        // feed 即使与旧流累计第 15 次同源，也只计 1 次（审计）、不 trip。
        // （2026-08-28 R1：门槛 3→15。）
        let core = "the quick brown fox jumps over lazy dog alpha";
        let mut first = DegenerationDetector::default();
        for i in 0..92 {
            first.feed_content(&format!("{core} marker {i:04}"));
        }
        assert_eq!(first.content_repetition.ngram_hits, 2);
        assert!(first.trip_reason().is_none(), "2 hits in first stream");
        assert_eq!(first.take_audit_hits().len(), 2);
        let mut second = DegenerationDetector::default();
        for i in 0..91 {
            second.feed_content(&format!("{core} marker {i:04}"));
        }
        assert_eq!(
            second.content_repetition.ngram_hits, 1,
            "fresh stream starts with zero hits"
        );
        assert!(
            second.trip_reason().is_none(),
            "fresh stream must not trip on its first hit"
        );
        let audits = second.take_audit_hits();
        assert_eq!(audits.len(), 1);
        assert!(audits[0].contains("stream hit 1/15"), "{}", audits[0]);
    }

    #[test]
    fn degeneration_detector_trips_after_twenty_stream_hits() {
        // 2026-08-28 R1（用户裁决：统一门槛 20）——同一 400 字符 span
        // 原样回放 19 次 → 0 trip（仅审计）；第 20 次 → 显式拦截（设计
        // §3.4 构造）。1–19 次命中仅审计留痕（take_audit_hits），不中断
        // 不降档。
        let span = "x".repeat(REPETITION_MIN_RUN_CHARS);
        let mut d = DegenerationDetector::default();
        d.feed_content(&span); // 首次出现，不计命中
        for i in 0..19 {
            d.feed_content(&distinct_random_text(200, 500 + i));
            d.feed_content(&span); // 回放 → 1 次命中
            assert!(d.trip_reason().is_none(), "{} replays must NOT trip", i + 1);
        }
        assert_eq!(
            d.take_audit_hits().len(),
            19,
            "19 sub-threshold hits must be audited"
        );
        d.feed_content(&distinct_random_text(200, 9000));
        d.feed_content(&span); // 第 20 次回放 → 20/20 → trip
        let reason = d.trip_reason().expect("20th stream hit must trip");
        assert!(reason.contains("repeated span"), "{reason}");
        assert!(reason.contains("20/20"), "{reason}");
    }

    #[test]
    fn degeneration_detector_hit_counter_survives_gaps() {
        // 2026-08-22 再校准（用户裁决：间隔不重置）+ 2026-08-23 L=400 +
        // 2026-08-28 R1 门槛 20——命中之间插入互异字符（不产生命中），
        // 流内计数不清零：累计到第 20 次仍触发（若间隔重置则停在 19 次、
        // 永不触发）。
        let span = distinct_alpha_text(REPETITION_MIN_RUN_CHARS, 7);
        let mut d = DegenerationDetector::default();
        d.feed_content(&span); // 首次出现
        d.feed_content(&distinct_random_text(200, 41)); // 间隔 1：无命中
        d.feed_content(&span); // 命中 1（仅审计）
        assert!(d.trip_reason().is_none(), "1 hit must NOT trip");
        for i in 0..18 {
            d.feed_content(&distinct_random_text(200, 700 + i)); // 间隔：无命中
            d.feed_content(&span); // 命中 2…19
            assert!(d.trip_reason().is_none(), "{} hits must NOT trip", i + 2);
        }
        assert_eq!(
            d.take_audit_hits().len(),
            19,
            "19 sub-threshold hits must be audited"
        );
        d.feed_content(&distinct_random_text(200, 7777)); // 间隔：无命中
        d.feed_content(&span); // 命中 20 → trip（间隔不重置）
        let reason = d.trip_reason().expect("20th hit after gaps must trip");
        assert!(reason.contains("repeated span"), "{reason}");
    }

    #[test]
    fn repetition_block_stats_confirms_structural_repeats() {
        // 二级确认核心（2026-08-23）：标点+空白切块后的内部重复覆盖占比。
        // `the the the`（空白切块）与 `aaa, aaa, aaa`（标点+空白切块）内部
        // 块重复 → 覆盖占比 ≥0.50 → confirmed；多样引用（每个块互异）→
        // 覆盖占比 <0.50 → 拒绝；无切分点单块 → 直接判真。
        let stats = repetition_block_stats(&"the the the".chars().collect::<Vec<_>>());
        assert_eq!(stats.block_count, 3);
        assert!(stats.confirmed(), "the the the must be confirmed");
        assert!(stats.repeated_coverage >= 0.5, "{stats:?}");
        let stats = repetition_block_stats(&"aaa, aaa, aaa".chars().collect::<Vec<_>>());
        assert!(stats.confirmed(), "aaa, aaa, aaa must be confirmed");
        let stats = repetition_block_stats(
            &"the quick brown fox jumps over the lazy dog"
                .chars()
                .collect::<Vec<_>>(),
        );
        assert!(!stats.confirmed(), "diverse citation must be rejected");
        assert!(stats.repeated_coverage < 0.5, "{stats:?}");
        let stats = repetition_block_stats(
            &"a".repeat(REPETITION_MIN_RUN_CHARS)
                .chars()
                .collect::<Vec<_>>(),
        );
        assert_eq!(stats.block_count, 1, "no split points → single block");
        assert!(stats.confirmed(), "no split points must confirm directly");
        // 全切分符 span（无有效块）同样无法切出多样结构 → 直接判真。
        let stats = repetition_block_stats(&", , , , ".chars().collect::<Vec<_>>());
        assert_eq!(stats.block_count, 0);
        assert!(
            stats.confirmed(),
            "separator-only span must confirm directly"
        );
    }

    #[test]
    fn repetition_second_stage_filters_diverse_span_candidates() {
        // 2026-08-23 二级确认接线 + S1 审查处理（P2-2 聚合）：字符级精确
        // 重复但内部标点块多样的大块（G4 sam-cell-seg 代码引用型误杀形
        // 态）→ 候选不计数、仅审计留痕；同 delta 候选聚合为一条摘要 +
        // 计数——1200 字符流内（自 idx 799 起每字符一对，共 401 对）只
        // 出两条聚合审计（计数 1 / 400），不逐条刷屏；构造的 `aaa, `
        // 周期（内部块重复）→ 二级过 → 触发。
        let mut quote = String::new();
        let mut i = 0usize;
        while quote.chars().count() < REPETITION_MIN_RUN_CHARS {
            quote.push_str(&format!("word{i} "));
            i += 1;
        }
        let quote: String = quote.chars().take(REPETITION_MIN_RUN_CHARS).collect();
        assert_eq!(quote.chars().count(), REPETITION_MIN_RUN_CHARS);
        let mut d = DegenerationDetector::default();
        for _ in 0..3 {
            d.feed_content(&quote);
        }
        assert!(
            d.trip_reason().is_none(),
            "diverse citation must NOT trip (second-stage rejects)"
        );
        let audits = d.take_audit_hits();
        // 聚合后：第二遍引用出 1 对（idx 799）、第三遍出 400 对（idx
        // 800–1199）——每 delta 一条摘要 + 计数，共 2 条。
        assert!(
            audits.len() == 2,
            "rejected candidates must be aggregated per delta, got {}",
            audits.len()
        );
        assert!(
            audits.iter().all(|a| a.contains("second-stage rejected")),
            "rejected audits must carry the second-stage marker: {audits:?}"
        );
        assert!(
            audits
                .iter()
                .all(|a| a.contains("punct-block repeated coverage")),
            "rejected audits must carry block stats: {audits:?}"
        );
        assert!(
            audits.iter().all(|a| a.contains("aggregated by delta")),
            "rejected audits must carry the aggregation marker: {audits:?}"
        );
        assert!(
            audits.iter().any(|a| a.contains("(1 candidate pair(s)")),
            "first-repeat delta must report count 1: {audits:?}"
        );
        assert!(
            audits.iter().any(|a| a.contains("(400 candidate pair(s)")),
            "third-repeat delta must report count 400: {audits:?}"
        );
        // 真复读：`aaa, ` 周期（80 周期 = 400 字符；内部块 "aaa" 重复 80
        // 次，覆盖占比 240/400=0.60 ≥ 0.50）→ 二级过 → 相邻第三份复制在
        // 窗口内产生大量确认命中 → 20/20 触发（2026-08-28 R1 门槛 20）。
        let cycle = "aaa, ".repeat(REPETITION_MIN_RUN_CHARS / 5);
        assert_eq!(cycle.chars().count(), REPETITION_MIN_RUN_CHARS);
        let mut d = DegenerationDetector::default();
        for _ in 0..3 {
            d.feed_content(&cycle);
        }
        let reason = d.trip_reason().expect("`aaa, ` cycle must trip");
        assert!(reason.contains("repeated span"), "{reason}");
    }

    #[test]
    fn repetition_second_stage_coverage_ratio_boundary() {
        // S2 设计项 + 2026-08-23 S1 审查处理（P3-3）：覆盖占比阈值边界
        // ——`abc` 重复块 + 互异单字符块的构造样本（逗号分隔）：
        // 39 遍 → 117/236 = 49.6% < 0.50 拒绝；40 遍 → 120/240 = 50.0%
        // 恰在阈值（≥0.50）通过；41 遍 → 123/242 = 50.8% 通过。
        let below = boundary_span(39, 40);
        let stats = repetition_block_stats(&below.chars().collect::<Vec<_>>());
        assert!(!stats.confirmed(), "49.6% must be rejected: {stats:?}");
        assert!(stats.repeated_coverage < 0.5, "{stats:?}");
        let at = boundary_span(40, 40);
        let stats = repetition_block_stats(&at.chars().collect::<Vec<_>>());
        assert!(
            stats.confirmed(),
            "50.0% must be confirmed (>= threshold): {stats:?}"
        );
        assert!((stats.repeated_coverage - 0.5).abs() < 1e-9, "{stats:?}");
        let above = boundary_span(41, 39);
        let stats = repetition_block_stats(&above.chars().collect::<Vec<_>>());
        assert!(stats.confirmed(), "50.8% must be confirmed: {stats:?}");
        assert!(stats.repeated_coverage > 0.5, "{stats:?}");
    }

    #[test]
    fn repetition_second_stage_pairwise_confirmation_accumulates() {
        // S2 设计项 + 2026-08-23 S1 审查处理（P3-3）：每对命中独立二级
        // 确认——第一对（多样引用 Q）字符级匹配但二级不过 → 不计数、仅
        // 聚合留痕；后续 `aaa, ` 结构对（R）的确认命中才累计：1 次仅
        // 审计、20 次触发（2026-08-28 R1 门槛 20）。
        let mut quote = String::new();
        let mut i = 0usize;
        while quote.chars().count() < REPETITION_MIN_RUN_CHARS {
            quote.push_str(&format!("word{i} "));
            i += 1;
        }
        let quote: String = quote.chars().take(REPETITION_MIN_RUN_CHARS).collect();
        let cycle = "aaa, ".repeat(REPETITION_MIN_RUN_CHARS / 5);
        let mut d = DegenerationDetector::default();
        // Q 两遍：idx 799 出现第一对（Q==Q）→ 二级不过 → 不计数。
        d.feed_content(&quote);
        d.feed_content(&quote);
        assert!(d.trip_reason().is_none(), "rejected pair must NOT count");
        let audits = d.take_audit_hits();
        assert_eq!(
            audits.len(),
            1,
            "one aggregated rejected entry, got {audits:?}"
        );
        assert!(audits[0].contains("second-stage rejected"), "{audits:?}");
        // R 第一遍：无候选（Q 区无 R 内容）；R 第二遍：idx 1599 出现
        // 确认对（R==R）→ 累计 1 次（仅审计，不触发）。
        d.feed_content(&cycle);
        d.feed_content(&cycle);
        assert!(
            d.trip_reason().is_none(),
            "1 confirmed hit must NOT trip (rejected pair did not count)"
        );
        let audits = d.take_audit_hits();
        assert_eq!(audits.len(), 1, "1 confirmed hit audited, got {audits:?}");
        assert!(!audits[0].contains("second-stage rejected"), "{audits:?}");
        // R 第三遍（相邻复制）：窗口内大量确认命中 → 累计 20 → 触发。
        d.feed_content(&cycle);
        let reason = d.trip_reason().expect("20th confirmed hit must trip");
        assert!(reason.contains("20/20"), "{reason}");
    }

    #[test]
    fn repetition_second_stage_short_citation_replay_stays_silent() {
        // 2026-08-23 S1 审查处理（P3-3）：G4 真实样本（sam-cell-seg
        // reasoning 层 203/204 字符代码引用）字节级夹具不在仓库，以形态
        // 等价构造离线回放——~203 字符代码引用（多样标点块）在同一流内
        // 多次原样引用（引用-再确认循环形态，间隔互异填充防跨引用窗口
        // 误成 400 字符周期）→ 第一级 L=400 即不命中：无候选、无审计、
        // 无触发（G4 误杀根因消除的离线验证形态）。
        // （2026-08-23 S2 正式轮起，字节级真实回放见
        // `repetition_second_stage_g4_real_span_replay_stays_silent`。）
        let snippet = "def _merge_collinear(segments):\n    out = []\n    for s in sorted(segments):\n        if out and out[-1][1] == s[0]:\n            out[-1] = (out[-1][0], s[1])\n        else:\n            out.append(s)\n    return out\n";
        let cite203: String = snippet.chars().take(203).collect();
        assert_eq!(cite203.chars().count(), 203);
        assert!(
            cite203.contains('(') && cite203.contains('[') && cite203.contains('\n'),
            "citation shape must include diverse punctuation blocks"
        );
        let mut d = DegenerationDetector::default();
        for seed in 1..=3u64 {
            d.feed_content(&cite203);
            d.feed_content(&distinct_random_text(50, seed));
        }
        assert!(d.trip_reason().is_none(), "short citations must NOT trip");
        assert!(
            d.take_audit_hits().is_empty(),
            "short citations must not reach first-stage candidates"
        );
    }

    #[test]
    fn repetition_second_stage_g4_real_span_replay_stays_silent() {
        // 2026-08-23 S2 正式轮：G4 官方轮真实字节离线回放。源证据=
        // D:\tb-eval\jobs-sweep\sweep-r1-g4-official\sam-cell-seg__CT5JrD3
        // \agent\orz.txt WARN 记录（2026-08-21T21:35:29 / 21:36:00）——
        // 旧 L=200 检测器在同一 reasoning 流内 3/3 命中并两次触发
        // （consecutive=1/2，降级梯 EnabledMax→EnabledLow→Disabled）。
        // 日志捕获的匹配 span 各 200 字符（检测器打印口径；设计早前
        // 203/204 为完整重复引用区域口径）——均 < L=400，第一级滚动
        // 窗口即不命中。回放形态=引用-再确认循环：真实 span 原样引用
        // 3 遍，中间插入与观察一致的互异再确认文本。
        let span_a = "\n```\n478→            return None\n479→        pts = _merge_collinear(pts)\n480→        if len(pts) < 4:\n481→            return None\n482→        glob = [(px + x0 - pad, py + y0 - pad) for px, py in pts]\n";
        let span_b = "   # 3) absolute last resort: the region's own pixels, duplicated if needed\n666-    base = sorted(own)\n667-    loop = (base * 4)[:4] if base else [(ax, ay)] * 4\n668-    loop = _dedupe_consecutive(loop";
        assert_eq!(
            span_a.chars().count(),
            200,
            "G4 span A must be the log-captured 200 chars"
        );
        assert_eq!(
            span_b.chars().count(),
            200,
            "G4 span B must be the log-captured 200 chars"
        );
        assert!(span_a.chars().count() < REPETITION_MIN_RUN_CHARS);
        assert!(span_b.chars().count() < REPETITION_MIN_RUN_CHARS);

        let interleave_a = "```\nActually the output showed lines: `478→            return None` — hmm no, let me look again at what was returned:\n\nFrom the offset=438 read:\n";
        let interleave_b = "Hmm wait the grep earlier showed lines 663-668 — let me re-read the file to confirm.\n";
        for span in [span_a, span_b] {
            let mut d = DegenerationDetector::default();
            d.feed_reasoning(span);
            d.feed_reasoning(interleave_a);
            d.feed_reasoning(span);
            d.feed_reasoning(interleave_b);
            d.feed_reasoning(span);
            assert!(
                d.trip_reason().is_none(),
                "real G4 reasoning span replay must NOT trip at L=400"
            );
            assert!(
                d.take_audit_hits().is_empty(),
                "real G4 spans (< L) must not reach first-stage candidates"
            );
        }
    }

    /// 二级覆盖占比边界样本：`abc` 块重复 `repeat_n` 遍 + `distinct_n`
    /// 个互异单字符块，全部逗号分隔（每块后跟一个逗号）。覆盖占比
    /// = 3·repeat_n / (4·repeat_n + 2·distinct_n)（含分隔符分母）。
    fn boundary_span(repeat_n: usize, distinct_n: usize) -> String {
        const SINGLES: &str = "defghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut out = String::new();
        for _ in 0..repeat_n {
            out.push_str("abc,");
        }
        for c in SINGLES.chars().take(distinct_n) {
            out.push(c);
            out.push(',');
        }
        out
    }

    #[test]
    fn degeneration_consecutive_counter_reaches_limit_and_never_resets() {
        // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.2.2)：哨兵计数 run 内单调递增——成功请求不再清零（仅 run
        // 边界重置=新 transport），封死「stall→成功→stall」跨请求反复烧
        // 预算模式。
        let t = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash");
        for i in 1..=2 {
            let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
            assert!(detail.starts_with(DEGENERATION_DETAIL_PREFIX), "{detail}");
            assert!(detail.contains(&format!("consecutive={i}")), "{detail}");
        }
        let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
        assert!(detail.starts_with(DEGENERATION_LIMIT_PREFIX), "{detail}");
        // 「成功」不再重置计数——第 4 次触发继续带 limit 标记且 consecutive
        // 单调递增（旧语义为重置回 1）。
        let detail = t.degeneration_interrupt_detail("degeneration_detected: x".into());
        assert!(detail.starts_with(DEGENERATION_LIMIT_PREFIX), "{detail}");
        assert!(detail.contains("consecutive=4"), "{detail}");
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
        // 2026-08-21 修订：滚动哈希粒度下 5×"hi" 不再触发；2026-08-22
        // 再校准：L=200 + 流内累计命中；2026-08-23 再校准：L=400 + 二级
        // 确认；2026-08-28 R1：统一门槛 20 + 802 独立同字符线（此处 802
        // 同字符由独立线直接触发）。
        let repeated = "x".repeat(802);
        let body = degenerate_sse_body(&repeated, 1);
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
            matches!(&err, GatewayError::StreamInterrupted { attempts: 0, detail, .. } if detail.starts_with(DEGENERATION_DETAIL_PREFIX)),
            "degeneration must surface StreamInterrupted(attempts=0) with the marker, got {err:?}"
        );
        assert_eq!(
            chunks,
            vec![repeated.clone()],
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

    /// SSE body with one reasoning-only delta + one content delta + a finish
    /// block (sub-threshold hit probe — the stream must complete normally
    /// when hits < limit; content keeps the D-6 empty-response chain out).
    fn reasoning_completed_sse_body(reasoning: &str, content: &str) -> String {
        let reasoning_frame = format!(
            r#"{{"id":"x","object":"chat.completion.chunk","created":0,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","reasoning_content":"{reasoning}"}},"finish_reason":null}}]}}"#
        );
        let content_frame = format!(
            r#"{{"id":"x","object":"chat.completion.chunk","created":0,"model":"m","choices":[{{"index":0,"delta":{{"role":"assistant","content":"{content}"}},"finish_reason":null}}]}}"#
        );
        format!(
            "data: {reasoning_frame}\n\ndata: {content_frame}\n\ndata: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{{}},\"finish_reason\":\"stop\"}}]}}\n\ndata: [DONE]\n\n"
        )
    }

    #[test]
    fn detector_reasoning_repetition_trips_on_repeated_span() {
        // 灵敏层（设计 §3.3 修订 + 2026-08-22/23 再校准）：同一滚动哈希
        // 算法作用于 reasoning delta——5×"same"（20 字符）与 24×"same"
        // （96 字符 < L=400）均不再触发；2026-08-28 R1 起 802 同字符由
        // 独立同字符线直接触发 → reasoning_repetition；循环型空转仍可
        // 在滚动路径 20 次命中时识别。
        let mut d = DegenerationDetector::default();
        for i in 0..23 {
            d.feed_reasoning("same");
            assert!(
                d.trip_reason().is_none(),
                "short identical reasoning deltas must NOT trip, feed {i}"
            );
        }
        d.feed_reasoning("same"); // 96 字符 < L=400，无命中
        assert!(
            d.trip_reason().is_none(),
            "96 chars below L=400 must NOT trip"
        );
        d.feed_reasoning(&"x".repeat(802));
        let reason = d
            .trip_reason()
            .expect("802-char same-character reasoning run must trip");
        assert!(
            reason.starts_with(REASONING_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("same-character run"), "{reason}");
    }

    #[test]
    fn detector_reasoning_repetition_periodic_cycle_trips() {
        // S2（设计 §3.3）+ 2026-08-22/23 再校准：同一滚动哈希算法作用于
        // reasoning——周期 10 循环（循环型空转特征）在灵敏层同样触发
        // （对齐缺陷回归；repeat(82)=820 字符 → 21 次命中 ≥ 20，
        // 2026-08-28 R1 门槛 20）。
        let mut d = DegenerationDetector::default();
        d.feed_reasoning(&"abcdefghij".repeat(82));
        let reason = d.trip_reason().expect("periodic reasoning cycle must trip");
        assert!(
            reason.starts_with(REASONING_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("repeated span"), "{reason}");
    }

    #[test]
    fn detector_reasoning_repetition_trips_on_high_ratio() {
        // 与 content 复读同一 1K 窗口 3-gram 重复率算法（>70% 触发 +
        // 流内累计命中 ≥15，2026-08-23 NGRAM-GUARD-CALIBRATION +
        // 2026-08-28 R1 门槛 3→15）；2026-08-21
        // 修订 + 2026-08-22/23 L=400：滚动哈希任意偏移保持静默（每 feed
        // 追加唯一 4 位标记，同 content 用例；双份 core 使 ratio ≈ 0.825）。
        let core = "the quick brown fox jumps over lazy dog the quick brown \
                    fox jumps over lazy dog";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_reasoning(&format!("{core} marker {i:04}"));
        }
        let audits = d.take_audit_hits();
        assert_eq!(
            audits.len(),
            14,
            "fourteen sub-threshold 3-gram hits must be audited, got {audits:?}"
        );
        assert!(
            audits
                .iter()
                .all(|a| a.contains("3-gram repetition ratio") && a.contains("reasoning tokens")),
            "{audits:?}"
        );
        let reason = d
            .trip_reason()
            .expect("high reasoning repetition must trip");
        assert!(
            reason.starts_with(REASONING_REPETITION_DETAIL_PREFIX),
            "{reason}"
        );
        assert!(reason.contains("3-gram repetition ratio"), "{reason}");
        assert!(reason.contains("15/15"), "{reason}");
        assert!(
            d.take_audit_hits().is_empty(),
            "rolling path must stay silent (no 400-char span repeats)"
        );
    }

    #[test]
    fn detector_reasoning_ngram_boundary_below_070_never_hits() {
        // S2（设计 §2.3）：reasoning 族与 content 同 profile 同语义——
        // 0.69x 边界样本（ratio≈0.694 < 0.70）永不计数、永不触发。
        let core = "the quick brown fox jumps over lazy dog";
        let mut d = DegenerationDetector::default();
        for i in 0..200 {
            d.feed_reasoning(&format!("{core} marker {i:04}"));
        }
        assert!(d.trip_reason().is_none(), "0.694 < 0.70 must never trip");
        assert!(
            d.take_audit_hits().is_empty(),
            "0.694 < 0.70 must never count a hit"
        );
    }

    #[test]
    fn detector_reasoning_guards_stand_down_after_visible_output() {
        // 设计 §3.3：reasoning 族信号仅 content/tool_calls 全空时启用——
        // content 出现后复读不得再触发。
        let mut d = DegenerationDetector::default();
        d.feed_content("answer");
        for _ in 0..5 {
            d.feed_reasoning("think");
        }
        assert!(
            d.trip_reason().is_none(),
            "reasoning repetition must be disabled once content is seen"
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
        // 设计 §3.2 + 2026-08-28 R1：完成型空响应 → 快速有界重试 ≤2 次
        // （同参 max）→ 降到 low（最多降到 low、不关闭）；low 轮产出答案。
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
                    body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                    "degraded attempt uses the low tier: {body}"
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
            "1 normal + 2 empty retries + 1 low-tier attempt"
        );
    }

    #[tokio::test]
    async fn generate_stream_empty_content_chain_end_errors_explicitly() {
        // 设计 §3.2 + 2026-08-28 R1：链尾（max/high 快速重试 2 次 → low →
        // low 快速重试 2 次 → 仍空）→ 显式失败，绝不静默空答案、绝不自动
        // 进入 disabled。
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
    async fn generate_stream_reasoning_repetition_interrupts_explicitly() {
        // 2026-08-28 R1（用户裁决：触发改显式拦截、不重试、不降档）：
        // max 阶段触发 reasoning 复读哨兵 → 不原样快速重试、不改 thinking
        // 档位、显式拦截该轮（detail 带 reasoning_repetition）；下一请求
        // 仍从 config 默认档 max 起始（无会话降级）。
        // 2026-08-21 修订：5×"think" 不再触发；2026-08-22/23 再校准：
        // 802 同字符 reasoning span 由独立同字符线触发（2026-08-28 R1）。
        let rep = reasoning_repetition_sse_body(&"x".repeat(802), 1);
        let ok = ok_sse_body("降级答案");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"max\""),
                    "first attempt uses the max config: {body}"
                );
                MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
            } else {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"max\""),
                    "subsequent request must still start at the config tier (max): {body}"
                );
                MockResponse::sse(vec![&ok], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX)),
            "reasoning trip must surface an explicit StreamInterrupted without degrade: {err:?}"
        );
        // 第二请求：同一 transport 仍从 max 起始（档位不被哨兵改动）。
        let r2 = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap();
        assert_eq!(r2.text.as_deref(), Some("降级答案"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "trip (1) + subsequent request at the same tier (2): no degrade, no retry"
        );
    }

    #[tokio::test]
    async fn generate_stream_reasoning_subthreshold_hits_do_not_interrupt() {
        // 2026-08-22 再校准 + 2026-08-23 L=400 + 2026-08-28 R1 门槛 20：
        // 1–19 次命中仅审计留痕——801 个同字符 reasoning（2 次命中）流
        // 正常完成（无 StreamInterrupted、无降档重试、单连接）。
        let body = reasoning_completed_sse_body(&"t".repeat(801), "ok");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
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
        let expected_reasoning = "t".repeat(801);
        assert_eq!(
            r.reasoning_content.as_deref(),
            Some(expected_reasoning.as_str()),
            "reasoning deltas must be joined verbatim",
        );
        assert_eq!(r.text.as_deref(), Some("ok"), "content must be delivered");
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 1,
            "sub-threshold hits must not trigger retries or degrade: {n} connections"
        );
    }

    // ── STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计
    // §2.2) + THIN-HARNESS-REDESIGN V2 R1（2026-08-28）：单调计数保留、
    // 会话档位移除、trip 显式拦截不降档 ─────────────────────────────────

    #[tokio::test]
    async fn generate_stream_reasoning_guard_chain_terminates_at_limit() {
        // 设计 §2.2.2 + 2026-08-28 R1：哨兵计数 run 内单调、档位不动——
        // 同一 thinking 档（high）连续 3 次复读 trip，第 3 次 detail 带
        // `degeneration_limit_reached` 标记（run_invalidated 语义），显式
        // 终止不再重试。
        let rep = reasoning_repetition_sse_body(&"x".repeat(802), 1);
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert!(
                body.contains("\"type\":\"enabled\"") && body.contains("\"high\""),
                "all attempts stay at the config tier (high), no degrade: {body}"
            );
            MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry_thinking(
            &base,
            Default::default(),
            ThinkingMode::EnabledHigh,
        );
        for _ in 0..2 {
            let err = t
                .generate_stream(request(), None, None, &mut |_| {})
                .await
                .unwrap_err();
            assert!(
                matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX) && !detail.starts_with(DEGENERATION_LIMIT_PREFIX)),
                "early trips must carry the reasoning marker without the limit prefix: {err:?}"
            );
        }
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(DEGENERATION_LIMIT_PREFIX)),
            "3rd guard must carry the run_invalidated marker: {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 3,
            "three trips at the same tier → explicit termination: {n} connections"
        );
    }

    #[tokio::test]
    async fn generate_stream_disabled_tier_guard_interrupts_explicitly() {
        // 2026-08-28 R1：`ORZ_THINKING_MODE=disabled` 仅保留为显式手动
        // A/B 配置——手动 disabled 档触发复读哨兵 → 显式拦截该轮
        // （StreamInterrupted 带 reasoning 前缀 + consecutive=1），不再
        // 强制补 `degeneration_limit_reached`（run 级达限由
        // DEGENERATION_LIMIT 单调计数统一判定）。
        let rep = reasoning_repetition_sse_body(&"x".repeat(802), 1);
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::Disabled);
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX) && detail.contains("consecutive=1")),
            "manual-disabled guard must surface the explicit interruption: {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 1,
            "one trip, no retry, no forced limit marker: {n} connections"
        );
    }

    #[tokio::test]
    async fn for_new_run_resets_degeneration_counter_across_logical_runs() {
        // STALL-DEGENERATION-FAILFAST + 2026-08-28 R1：per-run 隔离——
        // run 1 三次复读 trip 达 DEGENERATION_LIMIT（第 3 次带 limit
        // 前缀）；`for_new_run` 换新实例后计数归零，run 2 首次 trip 回到
        // consecutive=1（无 limit 前缀）。档位在两次 run 中保持 config
        // 默认（high）不变（无会话降级）。
        let rep = reasoning_repetition_sse_body(&"x".repeat(802), 1);
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            assert!(
                body.contains("\"type\":\"enabled\"") && body.contains("\"high\""),
                "all attempts stay at the config tier (high): {body}"
            );
            MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry_thinking(
            &base,
            Default::default(),
            ThinkingMode::EnabledHigh,
        );
        // run 1：连续 3 次 trip，第 3 次带 run_invalidated 标记。
        for _ in 0..2 {
            let err = t
                .generate_stream(request(), None, None, &mut |_| {})
                .await
                .unwrap_err();
            assert!(
                matches!(&err, GatewayError::StreamInterrupted { detail, .. } if !detail.starts_with(DEGENERATION_LIMIT_PREFIX)),
                "early trips must not carry the limit marker: {err:?}"
            );
        }
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(DEGENERATION_LIMIT_PREFIX)),
            "3rd trip in run 1 must carry the limit marker: {err:?}"
        );
        // run 边界：换新实例（等价于 controller.run_turn_inner 的
        // `for_new_run`）。
        let fresh = t.for_new_run();
        let err = fresh
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX) && detail.contains("consecutive=1")),
            "fresh run must restart the degeneration counter: {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(n, 4, "run 1 ×3 trips + run 2 ×1 trip: {n} connections");
    }

    #[tokio::test]
    async fn generate_stream_reasoning_guard_during_empty_retry_interrupts_explicitly() {
        // 设计 §3.2 + 2026-08-28 R1：阶段 1 完成型空响应进入快速重试；
        // 阶段 2（重试中）触发 reasoning 族哨兵 → 显式拦截（不重试、不
        // 降档、不继续空响应链）。
        let empty = empty_completed_sse_body();
        let rep = reasoning_repetition_sse_body(&"x".repeat(802), 1);
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                MockResponse::sse(vec![&empty], std::time::Duration::ZERO)
            } else {
                MockResponse::sse(vec![&rep], std::time::Duration::ZERO)
            }
        })
        .await;
        let t =
            mock_transport_with_retry_thinking(&base, Default::default(), ThinkingMode::EnabledMax);
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::StreamInterrupted { detail, .. } if detail.starts_with(REASONING_REPETITION_DETAIL_PREFIX)),
            "reasoning guard during the empty retry must interrupt explicitly: {err:?}"
        );
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            2,
            "empty → reasoning guard (no further identical retry, no degrade)"
        );
    }

    // ── 2026-08-28 修订: 空响应降级梯（max/high → low → 失败）──────────

    #[tokio::test]
    async fn generate_stream_empty_content_ladder_high_low_fails() {
        // 设计 §3.6 + 2026-08-28 R1：默认 high 档完成型空响应 → 每档快速
        // 重试 ≤2 次 → high → low → 显式失败（low 后不再降 disabled；
        // 链尾绝不静默空答案）。换档时重试计数与退避重置——每档独立
        // 「快速 ≤2 次」节奏（0.5s→1s×2 档）。
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
                _ => unreachable!("ladder must fail explicitly after the low tier"),
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
            6,
            "3 high + 3 low, then explicit failure (never disabled)"
        );
    }

    #[tokio::test]
    async fn generate_stream_empty_content_ladder_recovers_on_low_tier_success() {
        // 设计 §3.6 + 2026-08-28 R1：high 档快速重试耗尽 → 降 low；low 档
        // 成功 → 正常返回（恢复）。档位按请求新建，不跨请求残留。
        let empty = empty_completed_sse_body();
        let ok_body = reasoning_completed_sse_body("", "ok");
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
                3 => {
                    assert!(
                        body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                        "attempt 4 uses the low tier: {body}"
                    );
                    MockResponse::sse(vec![&ok_body], std::time::Duration::ZERO)
                }
                _ => unreachable!("recovery must stop the ladder"),
            }
        })
        .await;
        let t = mock_transport_with_retry_thinking(
            &base,
            Default::default(),
            ThinkingMode::EnabledHigh,
        );
        let response = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .expect("low-tier success must recover");
        assert!(response.text.as_deref().unwrap_or("").contains("ok"));
        assert_eq!(
            attempts.load(std::sync::atomic::Ordering::SeqCst),
            4,
            "3 high attempts + 1 low success"
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
                round: None,
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
                    round: None,
                },
                Message {
                    role: Role::Tool,
                    content: "ok".to_string(),
                    tool_call_id: Some("call-9".to_string()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
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
                    round: None,
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
                    round: None,
                },
                Message {
                    role: Role::Tool,
                    content: "ok".to_string(),
                    tool_call_id: Some("call-1".to_string()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
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
        // reasoning_content absent → Some("") (2026-08-25: normalized empty
        // string — the assistant replay wire shape must keep the field;
        // D-6 omission 400s on the next request).
        assert_eq!(r.reasoning_content, Some(String::new()));
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
        // D-6 + 2026-08-28 R1: abnormal empty content (finish=length, zero
        // output) retries ≤2 per tier, then degrades to low (never disabled);
        // the chain yields the degraded response instead of erroring.
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n < 3 {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"max\""),
                    "attempts 1-3 use the max config: {body}"
                );
                MockResponse::json(200, empty_finish_length_response())
            } else {
                assert!(
                    body.contains("\"type\":\"enabled\"") && body.contains("\"low\""),
                    "degraded attempt uses the low tier: {body}"
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
            4,
            "3 max + 1 low-tier attempt"
        );
    }

    #[tokio::test]
    async fn generate_empty_content_chain_end_errors_explicitly() {
        // D-6 + 2026-08-28 R1: the chain end (max ×3 + low ×3 all empty)
        // surfaces an explicit termination reason — never a silent blank
        // response, never an automatic disabled tier.
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
    async fn generate_stream_text_round_preserves_empty_reasoning() {
        // 2026-08-25 (initial-package trial, 3/6 flake): v4-flash text-only
        // completions can omit `reasoning_content` deltas entirely. The
        // replay must still carry `Some("")` — folding to None drops the
        // field and DeepSeek 400s on the next request ("reasoning_content
        // in the thinking mode must be passed back to the API").
        let frame = |delta: &str, finish: Option<&str>| {
            let finish = finish
                .map(|f| format!("\"{f}\""))
                .unwrap_or_else(|| "null".to_string());
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":{finish}}}]}}\n\n",
            )
        };
        let body = format!(
            "{}{}data: [DONE]\n\n",
            frame(r#"{"role":"assistant","content":"done"}"#, None),
            frame(r#"{"content":"-now"}"#, Some("stop")),
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
        assert_eq!(chunks, vec!["done", "-now"]);
        assert_eq!(r.text.as_deref(), Some("done-now"));
        assert!(r.tool_calls.is_empty());
        assert_eq!(r.reasoning_content, Some(String::new()));
        assert_eq!(r.finish_reason, OurFinishReason::Stop);
    }

    #[tokio::test]
    async fn generate_stream_truncated_stream_retries_then_errors_explicitly() {
        // A stream that ends without a finish_reason block is truncated
        // (proxy drop / deploy switch / overload) — it must surface as an
        // error, never as a completed `stop` answer (D1-1). MIDSTREAM-
        // DECODE-RETRY (2026-08-21, ADR-0010 §14.37): chunks decoded with
        // no complete tool call → bounded single retry → explicit failure.
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
            matches!(&err, GatewayError::StreamInterrupted { attempts: 1, saw_chunk: true, detail } if detail.contains("finish_reason")),
            "truncated stream must retry once then error explicitly, got {err:?}"
        );
        // Chunks delivered from BOTH attempts are still honest deltas — but
        // the response must never be treated as complete.
        assert_eq!(chunks, vec!["half", "-done", "half", "-done"]);
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
        // THIN-HARNESS-REDESIGN R1 (§4.6, 2026-08-27)：默认档改为
        // EnabledMax（用户裁决）——默认指纹与显式 EnabledMax 一致，且与
        // EnabledHigh / EnabledLow / Disabled 各档互不相同。
        let fingerprint_for = |thinking: ThinkingMode| {
            let mut cfg = DeepSeekTransport::deepseek_v4("sk-test", "deepseek-v4-flash").config;
            cfg.thinking = thinking;
            DeepSeekTransport::new(cfg).config_fingerprint()
        };
        let default_fp = fingerprint_for(ThinkingMode::default());
        let max_fp = fingerprint_for(ThinkingMode::EnabledMax);
        assert_eq!(
            default_fp, max_fp,
            "default must carry enabled_max into the digest"
        );
        for tier in [
            ThinkingMode::EnabledHigh,
            ThinkingMode::EnabledLow,
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
            matches!(&err, GatewayError::StreamInterrupted { attempts, detail, .. } if *attempts == 1 && detail.contains("idle")),
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

    /// SSE body: content chunks followed by `[DONE]` with NO finish_reason
    /// — a midstream truncation (chunks decoded, no complete tool call).
    fn midstream_truncation_sse_body() -> String {
        let frame = |delta: &str| {
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":null}}]}}\n\n",
            )
        };
        format!(
            "{}{}data: [DONE]\n\n",
            frame(r#"{"role":"assistant","content":"half"}"#),
            frame(r#"{"content":"-done"}"#),
        )
    }

    #[tokio::test]
    async fn generate_stream_midstream_truncation_retries_then_succeeds() {
        // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.1)：已见 chunk 但无完整 tool_calls 的截断（dna-assembly 的
        // `error decoding response body` 类签名）→ 有界重试 1 次（幂等——
        // 错误路径工具从未执行）。旧纪律「已见输出不重试」废止。
        let body = midstream_truncation_sse_body();
        let ok = ok_sse_body("ok-after-retry");
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            let n = attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                MockResponse::sse(vec![&body], std::time::Duration::ZERO)
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
        assert_eq!(
            chunks,
            vec!["half", "-done", "ok-after-retry"],
            "partial chunks from the failed attempt are delivered, then the retry's output"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 2,
            "midstream truncation must retry exactly once: {n} connections"
        );
    }

    #[tokio::test]
    async fn generate_stream_midstream_truncation_retry_exhausts_explicitly() {
        // 设计 §2.2：chunked 有界 1 次——持续中段截断在第 2 次连接后显式
        // 失败（attempts=1、saw_chunk=true），不再无限烧预算。
        let body = midstream_truncation_sse_body();
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
            matches!(&err, GatewayError::StreamInterrupted { attempts: 1, saw_chunk: true, detail } if detail.contains("finish_reason")),
            "midstream retry exhaustion must surface StreamInterrupted(attempts=1, saw_chunk=true): {err:?}"
        );
        assert_eq!(chunks, vec!["half", "-done", "half", "-done"]);
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 2,
            "chunked retry budget must cap at one extra re-send: {n} connections"
        );
    }

    #[tokio::test]
    async fn generate_stream_complete_tool_call_truncation_is_not_retried() {
        // 设计 §2.1 双保险边界：失败流已解码出完整 tool_calls（id + name +
        // 合法 JSON arguments）→ 不重试，保持普通 Transport 错误（即便
        // 执行路径不会运行它）。
        let frame = |delta: &str| {
            format!(
                "data: {{\"id\":\"x\",\"object\":\"chat.completion.chunk\",\"created\":0,\"model\":\"m\",\"choices\":[{{\"index\":0,\"delta\":{delta},\"finish_reason\":null}}]}}\n\n",
            )
        };
        let body = format!(
            "{}data: [DONE]\n\n",
            frame(
                r#"{"role":"assistant","tool_calls":[{"index":0,"id":"call-1","type":"function","function":{"name":"read_file","arguments":"{\"target_file\":\"a.txt\"}"}}]}"#
            ),
        );
        let attempts = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let attempts_handle = attempts.clone();
        let base = spawn_mock(move |_line, _body| {
            attempts_handle.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            MockResponse::sse(vec![&body], std::time::Duration::ZERO)
        })
        .await;
        let t = mock_transport_with_retry(&base, short_retry_policy());
        let err = t
            .generate_stream(request(), None, None, &mut |_| {})
            .await
            .unwrap_err();
        assert!(
            matches!(&err, GatewayError::Transport(m) if m.contains("finish_reason")),
            "complete tool call must keep the plain transport error (no retry): {err:?}"
        );
        let n = attempts.load(std::sync::atomic::Ordering::SeqCst);
        assert_eq!(
            n, 1,
            "complete tool calls must never be re-sent: {n} connections"
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
            matches!(&err, GatewayError::StreamInterrupted { attempts, detail, .. } if *attempts == 3 && detail.contains("finish_reason")),
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
        // within the budget errors instead of running forever. MIDSTREAM-
        // DECODE-RETRY (2026-08-21): the budget timeout after decoded
        // chunks (no complete tool call) is retried once, then fails
        // explicitly.
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
            matches!(&err, GatewayError::StreamInterrupted { attempts: 1, saw_chunk: true, detail } if detail.contains("total")),
            "non-terminating stream must hit the total budget then fail explicitly, got {err:?}"
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
                    round: None,
                }],
                tools: Vec::new(),
                max_tokens: 32,
                thinking: None,
            })
            .await;
        assert!(r.is_ok(), "{r:?}");
    }
}
