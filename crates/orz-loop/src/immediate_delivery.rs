//! 0ac S3①-b (2026-09-14, IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL
//! _DESIGN §5/§9/§10.3①): **即时结果反馈的到达面/交付面写点**。
//!
//! S2 落契（`runtime/retrieval-progress-event-payload-v0.2.schema.json`、
//! `retrieval-result-segment-event-payload-v0.2.schema.json`、
//! `result-delivered-event-payload-v0.2.schema.json` + registry + 法官 fixtures），
//! 本模块是 S3①-b 的**产品码写入侧**：只做三件机械事——
//!
//! 1. 开关（`ORZ_IMMEDIATE_RESULT_DELIVERY`，默认关；§10.3 全部带开关）；
//! 2. 从**本地分段检索**的模型可见渲染里解析到达事实（引擎 / 已等 ms /
//!    逐项逐段）——渲染是工具侧稳定机器格式
//!    （`web_search/local_segmented.rs::render_content`：
//!    `[local_segmented] engine=… hits=… waited=…ms` + `N. title — url` +
//!    `   [§k] segment`）；
//! 3. 生成三个事件的 payload（纯函数，字段与 S2 schema 一一对应，
//!    `additionalProperties:false` ⇒ 不夹带任何多余字段/stamp）。
//!
//! 记账纪律（设计 §5）：本地分段路径的产物**已经在工具结果里被模型直接
//! 读到**，框架不得重复注入 ⇒ 交付面落
//! `result_delivered{suppressed:true, suppressed_reason:"model_read_directly",
//! boundary:"B1_tool_result", latency_ms:0}`。到达面（段事件）与交付面
//! （交付记账）分离，正是"到达"与"已交付"两个事实的可审计化。
//!
//! 路径与边界：① 本地分段检索是**同调用同步返回**，结果到达时刻即
//! B1（tool_result）边界，故无需 pending 队列（队列随 ② 服务端流式的
//! 异步到达面落，见设计 §5 B2/B3）。失败面（`stage:"failed"` + 五稳定码）
//! 只在错误文本自带稳定码时落账（`stable_code_from_error`）——不猜码：
//! 猜出来的失败码是伪事实，比缺事件更坏。

use serde_json::{Value, json};

/// 开关（默认关；§10.3：全部带开关，关时零事件、零行为变化）。
pub const SWITCH_ENV: &str = "ORZ_IMMEDIATE_RESULT_DELIVERY";
/// 单引擎截止（ms；§9.4 不共用钟）。与工具侧同名 env 同源，默认 10s。
pub const DEADLINE_ENV: &str = "ORZ_RETRIEVAL_DEADLINE_MS";
pub const DEFAULT_DEADLINE_MS: u64 = 10_000;

/// 检索路径（① 主路径；服务端流式 = ② 恢复时复用）。
pub const PATH_LOCAL_SEGMENTED: &str = "local_segmented";

pub const STAGE_DISPATCHED: &str = "dispatched";
pub const STAGE_FINISHED: &str = "finished";
pub const STAGE_FAILED: &str = "failed";
/// M3 中途回报阶段（schema `stage` 闭枚举第五位；长工具在途读数）。
pub const STAGE_PROGRESS: &str = "progress";

/// 渲染头（工具侧稳定机器格式；解析漂移由单测钉住）。
const RENDER_HEADER: &str = "[local_segmented]";
const SEGMENT_PREFIX: &str = "[§";

/// 开关判读：真实值 `1/true/on/yes`（大小写不敏感）为开；其余（含未设置）
/// 为关。
pub fn switch_enabled() -> bool {
    match std::env::var(SWITCH_ENV) {
        Ok(v) => switch_truthy(&v),
        Err(_) => false,
    }
}

/// 开关取值判读（纯函数；未设置 = 关，由调用方给 `Err` 分支落 false）。
pub fn switch_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "on" | "yes"
    )
}

/// 本引擎生效截止 ms（env 覆盖，非法/缺省 = 10s）。
pub fn per_engine_deadline_ms() -> u64 {
    std::env::var(DEADLINE_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_DEADLINE_MS)
}

/// 一个到达的**结果项**（SERP 命中页），带其已抽取段数。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalHit {
    /// 结果项序号（0 基；渲染里印的是 1 基）。
    pub index: u64,
    pub title: String,
    pub url: String,
    pub segments: usize,
    pub has_snippet: bool,
}

impl ArrivalHit {
    /// 观测范围（`observed_scope`）：有段=正文已抽段，仅有 SERP 摘要=
    /// 摘要面，两者皆无=仅元数据。
    pub fn observed_scope(&self) -> String {
        if self.segments > 0 {
            format!("page_segments={}", self.segments)
        } else if self.has_snippet {
            "serp_snippet".to_string()
        } else {
            "metadata_only".to_string()
        }
    }

    /// 可见性：段正文 ⇒ 全文面已观测；仅摘要 ⇒ 部分；皆无 ⇒ 仅元数据。
    pub fn visibility(&self) -> &'static str {
        if self.segments > 0 {
            "full_text_observed"
        } else if self.has_snippet {
            "partial_text_observed"
        } else {
            "metadata_only"
        }
    }
}

/// 一次本地分段检索的到达事实（头 + 逐项）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arrivals {
    pub engine: String,
    pub declared_hits: usize,
    pub waited_ms: u64,
    pub hits: Vec<ArrivalHit>,
}

/// 去重键：同一事实只入账/投递一次（段身份 = call + 项序号）。
pub fn segment_dedupe_key(call_id: &str, index: u64) -> String {
    format!("retrieval_segment:{call_id}:{index}")
}

/// 进度事件去重键（同一 call 的同一阶段只有一条）。
pub fn progress_dedupe_key(call_id: &str, stage: &str) -> String {
    format!("retrieval_progress:{call_id}:{stage}")
}

/// 解析本地分段检索的渲染（非该渲染 ⇒ `None`，调用方**不落**到达面事件）。
pub fn parse_render(output: &str) -> Option<Arrivals> {
    let mut lines = output.lines();
    let header = lines.next()?.trim_end_matches('\r');
    let rest = header.strip_prefix(RENDER_HEADER)?.trim();
    let mut engine = None;
    let mut declared_hits = None;
    let mut waited_ms = None;
    for field in rest.split_whitespace() {
        if let Some(v) = field.strip_prefix("engine=") {
            engine = Some(v.to_string());
        } else if let Some(v) = field.strip_prefix("hits=") {
            declared_hits = v.parse::<usize>().ok();
        } else if let Some(v) = field.strip_prefix("waited=") {
            waited_ms = v.trim_end_matches("ms").parse::<u64>().ok();
        }
    }
    let (engine, declared_hits, waited_ms) = (engine?, declared_hits?, waited_ms?);
    let mut hits: Vec<ArrivalHit> = Vec::new();
    for raw in lines {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if !line.starts_with(char::is_whitespace) && line.chars().next()?.is_ascii_digit() {
            // `N. title — url`（分隔符是排版破折号，容忍半角 ` - `）。
            let (num, tail) = match line.split_once(". ") {
                Some(parts) => parts,
                None => continue,
            };
            let Some(index) = num.trim().parse::<u64>().ok().map(|n| n.saturating_sub(1)) else {
                continue;
            };
            let (title, url) = match split_title_url(tail) {
                Some(parts) => parts,
                None => continue,
            };
            hits.push(ArrivalHit {
                index,
                title,
                url,
                segments: 0,
                has_snippet: false,
            });
        } else if let Some(hit) = hits.last_mut() {
            if line.contains(SEGMENT_PREFIX) {
                hit.segments += 1;
            } else {
                hit.has_snippet = true;
            }
        }
    }
    Some(Arrivals {
        engine,
        declared_hits,
        waited_ms,
        hits,
    })
}

/// 拆 `title — url`（破折号容忍 em/en dash 与半角 ` - `）。
fn split_title_url(tail: &str) -> Option<(String, String)> {
    for sep in ['—', '–'] {
        if let Some((t, u)) = tail.split_once(sep) {
            let (t, u) = (t.trim(), u.trim());
            if !u.is_empty() {
                return Some((t.to_string(), u.to_string()));
            }
        }
    }
    match tail.rsplit_once(" - ") {
        Some((t, u)) if !u.trim().is_empty() => Some((t.trim().to_string(), u.trim().to_string())),
        _ => None,
    }
}

/// `retrieval_progress` payload（S2 v0.2 schema；必填：tool/call_id/
/// retrieval_path/stage/waited_ms/dedupe_key）。
pub fn progress_payload(
    tool: &str,
    call_id: &str,
    stage: &str,
    waited_ms: u64,
    result_count: Option<usize>,
    stable_code: Option<&str>,
) -> Value {
    let mut payload = json!({
        "tool": tool,
        "call_id": call_id,
        "retrieval_path": PATH_LOCAL_SEGMENTED,
        "stage": stage,
        "waited_ms": waited_ms,
        "deadline_ms": per_engine_deadline_ms(),
        "dedupe_key": progress_dedupe_key(call_id, stage),
    });
    if let Some(count) = result_count {
        payload["result_count"] = json!(count);
    }
    if let Some(code) = stable_code {
        payload["stable_code"] = json!(code);
    }
    payload
}

/// `retrieval_result_segment` payload（每个结果项到达即一条）。
pub fn segment_payload(tool: &str, call_id: &str, waited_ms: u64, hit: &ArrivalHit) -> Value {
    json!({
        "tool": tool,
        "call_id": call_id,
        "retrieval_path": PATH_LOCAL_SEGMENTED,
        "segment_index": hit.index,
        "is_partial": false,
        "waited_ms": waited_ms,
        "dedupe_key": segment_dedupe_key(call_id, hit.index),
        "result_summary": {
            "visibility": hit.visibility(),
            "source_url": hit.url,
            "title": hit.title,
            "observed_scope": hit.observed_scope(),
        },
    })
}

/// `result_delivered` payload（交付记账面）。
///
/// 本地分段路径：产物已随工具结果进入对话 ⇒ `suppressed=true` /
/// `model_read_directly` / `boundary="B1_tool_result"` / `latency_ms=0`。
pub fn delivered_payload(call_id: &str, index: u64, latency_ms: u64) -> Value {
    let now = crate::controller::chrono_utc_now();
    json!({
        "result_source": "retrieval_segment",
        "source_id": call_id,
        "boundary": "B1_tool_result",
        "delivery_mode": "direct",
        "delivery_class": "I1_immediate_material",
        "suppressed": true,
        "suppressed_reason": "model_read_directly",
        "latency_ms": latency_ms,
        "dedupe_key": segment_dedupe_key(call_id, index),
        "delivered_at": now,
    })
}

/// 从工具错误文本里读**已有的**稳定码（工具侧 `details.cause`，
/// local_segmented §10.1 闭枚举）。读不到 ⇒ `None`（不落失败事件）。
pub fn stable_code_from_error(text: &str) -> Option<&'static str> {
    [
        "capability_unreachable",
        "network_no_response",
        "network_error",
        "empty_result",
        "no_progress",
    ]
    .into_iter()
    .find(|code| text.contains(*code))
}

// ══════════════════════════════════════════════════════════════════════
// 投递侧（0ac S3①-b M2/M1；设计 §2.1/§2.2/§4.1）
//
// 到达面只管「事实到达」，本段管「框架把事实投给模型」——两者分立记账
// （设计 §4.2「去重与抑制」：框架投递与「模型自己读到」分开）。
//
// M2 = **合法边界投递**：B1（工具结果）/B2（轮结束）都是合法注入点，
//      不再依赖「下一次工具结果」这一特定点；B4（pending 工具窗口）无
//      合法注入点，只能等 B1。队列纪律：同一 `dedupe_key` 只投一次
//      （LRU 防重复）；TTL / 轮数兜底把压过久的事实降级成 digest 一次
//      完整带出（窄窗口失效保证）；run 关闭即删（不跨 run 泄漏）。
// M1 = **收尾注入**（带开关 + A/B，独立于 M2）：本轮已是纯文本终答候选
//      而队列仍有未投递事实时，注入后把这一轮切成一段、仅一次；用尽即
//      按原路径收尾（绝不挂死、绝不重复注入）。
// ══════════════════════════════════════════════════════════════════════

/// M1（收尾注入）独立开关——A/B 面：M2 落码即生效，M1 需显式开。
pub const M1_SWITCH_ENV: &str = "ORZ_IMMEDIATE_RESULT_DELIVERY_M1";
/// 投递兜底 TTL（ms；§2.2 降级投递）。
pub const TTL_ENV: &str = "ORZ_IMMEDIATE_DELIVERY_TTL_MS";
/// 投递兜底轮数上限（§2.2 降级投递）。
pub const MAX_ROUNDS_ENV: &str = "ORZ_IMMEDIATE_DELIVERY_MAX_ROUNDS";
/// ⑥ 检索子代理提前收口的连续确定失败阈值（0 = 禁用；缺省 3）。
pub const EARLY_CLOSE_FAILURES_ENV: &str = "ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES";
/// M3 中途回报 tick 间隔（ms；0 = 禁用 tick，缺省 10 000——与
/// `T_first` 同阶，覆盖「到点前零事件」窗口）。
pub const PROGRESS_TICK_MS_ENV: &str = "ORZ_RETRIEVAL_PROGRESS_TICK_MS";
pub const DEFAULT_TTL_MS: u64 = 120_000;
pub const DEFAULT_MAX_ROUNDS: u64 = 3;
pub const DEFAULT_EARLY_CLOSE_FAILURES: u64 = 3;
pub const DEFAULT_PROGRESS_TICK_MS: u64 = 10_000;

/// 投递边界（schema `boundary` 闭枚举；B4 不是边界）。
pub const BOUNDARY_B1: &str = "B1_tool_result";
pub const BOUNDARY_B2: &str = "B2_turn_end";
/// 投递方式（schema `delivery_mode` 闭枚举；`sentence_resume` 只随 B3）。
pub const MODE_DIRECT: &str = "direct";
pub const MODE_DIGEST: &str = "digest";
/// 投递分级（schema `delivery_class` 闭枚举）。
pub const CLASS_I1: &str = "I1_immediate_material";
pub const CLASS_I3: &str = "I3_deferred_digest";
/// 来源类型（schema `result_source` 闭枚举第一位）。
pub const SOURCE_BACKGROUND_TASK: &str = "background_task";

/// M1 开关判读：M2 主开关 **且** M1 子开关为真实值（默认关；A/B 面）。
pub fn m1_enabled() -> bool {
    switch_enabled()
        && std::env::var(M1_SWITCH_ENV)
            .map(|v| switch_truthy(&v))
            .unwrap_or(false)
}

/// 兜底 TTL（env 覆盖，非法/缺省 = 120 s）。
pub fn ttl_ms_from_env() -> u64 {
    std::env::var(TTL_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_TTL_MS)
}

/// 兜底轮数（env 覆盖，非法/缺省 = 3）。
pub fn max_rounds_from_env() -> u64 {
    std::env::var(MAX_ROUNDS_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_MAX_ROUNDS)
}

/// ⑥ 连续确定失败阈值（env 覆盖；`0` = 显式禁用，非法/缺省 = 3）。
pub fn early_close_failure_limit() -> u64 {
    std::env::var(EARLY_CLOSE_FAILURES_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_EARLY_CLOSE_FAILURES)
}

/// M3 tick 间隔（env 覆盖；`0` = 显式禁用，非法/缺省 = 10 000 ms）。
pub fn progress_tick_ms() -> u64 {
    std::env::var(PROGRESS_TICK_MS_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_PROGRESS_TICK_MS)
}

/// 一个待投递事实（来源身份 + 模型可见文本 + 到达时刻/轮次）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingFact {
    pub source: &'static str,
    pub source_id: String,
    /// 模型可见的机械事实文本（由来源侧单一源格式化；loop 不重写）。
    pub text: String,
    pub dedupe_key: String,
    pub class: &'static str,
    pub admitted_ms: u64,
    pub admitted_round: u64,
}

impl PendingFact {
    /// 后台任务完成（I1：确定完成 / 失败稳定码）——`dedupe_key` = 来源+身份，
    /// 同一任务只投一次。
    pub fn background_task(task_id: &str, text: String, now_ms: u64, round: u64) -> PendingFact {
        PendingFact {
            source: SOURCE_BACKGROUND_TASK,
            source_id: task_id.to_string(),
            text,
            dedupe_key: fact_dedupe_key(SOURCE_BACKGROUND_TASK, task_id),
            class: CLASS_I1,
            admitted_ms: now_ms,
            admitted_round: round,
        }
    }
}

/// 事实去重键（设计 §2.3：去重键 =（来源类型, 任务/调用 id, 结果摘要））。
pub fn fact_dedupe_key(source: &str, source_id: &str) -> String {
    format!("{source}:{source_id}")
}

/// 一次投递（事实 + 方式 + 到达→投递延迟）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub fact: PendingFact,
    pub mode: &'static str,
    pub latency_ms: u64,
}

/// 待投递队列（run 作用域；关闭即删）。
#[derive(Debug)]
pub struct DeliveryQueue {
    pending: Vec<PendingFact>,
    seen: std::collections::HashSet<String>,
    ttl_ms: u64,
    max_rounds: u64,
}

impl DeliveryQueue {
    pub fn new(ttl_ms: u64, max_rounds: u64) -> DeliveryQueue {
        DeliveryQueue {
            pending: Vec::new(),
            seen: std::collections::HashSet::new(),
            ttl_ms,
            max_rounds,
        }
    }

    /// env 口径（ORZ_IMMEDIATE_DELIVERY_TTL_MS / _MAX_ROUNDS）。
    pub fn from_env() -> DeliveryQueue {
        DeliveryQueue::new(ttl_ms_from_env(), max_rounds_from_env())
    }

    /// 入队；同一 `dedupe_key` 已入队/已投 ⇒ 拒绝（LRU 防重复，返回 false）。
    pub fn admit(&mut self, fact: PendingFact) -> bool {
        if !self.seen.insert(fact.dedupe_key.clone()) {
            return false;
        }
        self.pending.push(fact);
        true
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    /// 当前边界应收的全部事实（M2：到达即投，不按轮攒）。
    ///
    /// 投递方式：I1 = `direct`；逾期事实（超 TTL 或超轮数上限）降级为
    /// `digest`，`delivery_class` 如实改记 I3_deferred_digest（交付形态
    /// = 延迟摘要）——兜底保证一次完整带出。
    pub fn due(&mut self, round: u64, now_ms: u64) -> Vec<Delivery> {
        let mut out = Vec::new();
        for mut fact in std::mem::take(&mut self.pending) {
            let overdue = now_ms.saturating_sub(fact.admitted_ms) >= self.ttl_ms
                || round.saturating_sub(fact.admitted_round) >= self.max_rounds;
            let mode = if fact.class == CLASS_I1 && !overdue {
                MODE_DIRECT
            } else {
                fact.class = CLASS_I3;
                MODE_DIGEST
            };
            let latency_ms = now_ms.saturating_sub(fact.admitted_ms);
            out.push(Delivery {
                fact,
                mode,
                latency_ms,
            });
        }
        out
    }

    /// 关闭即删（设计 §4.2）：run 关闭路径立即清空队列并返回被丢弃事实
    /// ——调用方据此落 journal 留痕；队列**不跨 run 存活**。
    pub fn close_drop(&mut self) -> Vec<PendingFact> {
        std::mem::take(&mut self.pending)
    }
}

/// 投递消息文本（中性事实；来源侧已格式化，此处只做机械拼接，不重写）。
pub fn render_delivery_message(deliveries: &[Delivery]) -> String {
    deliveries
        .iter()
        .map(|d| d.fact.text.trim_end().to_string())
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// `result_delivered` payload（**真投递**：suppressed=false；schema 要求
/// 此形态不得携带 `suppressed_reason`）。
pub fn delivered_fact_payload(delivery: &Delivery, boundary: &str) -> Value {
    json!({
        "result_source": delivery.fact.source,
        "source_id": delivery.fact.source_id,
        "boundary": boundary,
        "delivery_mode": delivery.mode,
        "delivery_class": delivery.fact.class,
        "suppressed": false,
        "latency_ms": delivery.latency_ms,
        "dedupe_key": delivery.fact.dedupe_key,
        "delivered_at": crate::controller::chrono_utc_now(),
    })
}

/// M3 中途回报（长工具边界）：检索调用仍在途的机械读数——`stage:"progress"`，
/// 只带已等待 ms（`deadline_ms` 由 [`progress_payload`] 同源补），
/// 让「到点前零事件」路径不存在。`tick` 区分同一 call 的多次读数。
pub fn progress_tick_payload(tool: &str, call_id: &str, waited_ms: u64, tick: u64) -> Value {
    let mut payload = progress_payload(
        tool,
        call_id,
        crate::immediate_delivery::STAGE_PROGRESS,
        waited_ms,
        None,
        None,
    );
    payload["dedupe_key"] = json!(format!("retrieval_progress:{call_id}:progress:{tick}"));
    payload
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 夹具逐字取自工具侧渲染格式（`render_content`），单测即漂移门。
    const RENDER: &str = "[local_segmented] engine=bing_cn hits=2 waited=1840ms\n\n1. Rust 官网 — https://www.rust-lang.org/\n   Rust is a systems language.\n   [§1] Rust 是一门系统级语言。\n   [§2] Cargo 是构建工具。\n\n2. The Rust Book — https://doc.rust-lang.org/book/\n   Learn Rust with the official book.\n";

    #[test]
    fn parse_render_reads_header_and_per_hit_segments() {
        let a = parse_render(RENDER).expect("local_segmented render parses");
        assert_eq!(a.engine, "bing_cn");
        assert_eq!(a.declared_hits, 2);
        assert_eq!(a.waited_ms, 1840);
        assert_eq!(a.hits.len(), 2);
        assert_eq!(a.hits[0].index, 0);
        assert_eq!(a.hits[0].url, "https://www.rust-lang.org/");
        assert_eq!(a.hits[0].segments, 2);
        assert_eq!(a.hits[0].visibility(), "full_text_observed");
        assert_eq!(a.hits[1].index, 1);
        assert_eq!(a.hits[1].segments, 0);
        assert_eq!(a.hits[1].has_snippet, true);
        assert_eq!(a.hits[1].visibility(), "partial_text_observed");
        assert_eq!(a.hits[1].observed_scope(), "serp_snippet");
    }

    #[test]
    fn parse_render_rejects_non_local_renders() {
        assert!(parse_render("some server-side web_search text").is_none());
        assert!(parse_render("").is_none());
    }

    #[test]
    fn payloads_match_the_s2_schemas_fieldwise() {
        let a = parse_render(RENDER).unwrap();
        let p = progress_payload("web_search", "call-1", STAGE_DISPATCHED, 0, None, None);
        assert_eq!(p["stage"], "dispatched");
        assert_eq!(p["retrieval_path"], "local_segmented");
        assert_eq!(p["dedupe_key"], "retrieval_progress:call-1:dispatched");
        assert!(p.get("stable_code").is_none());
        let s = segment_payload("web_search", "call-1", a.waited_ms, &a.hits[0]);
        assert_eq!(s["segment_index"], 0);
        assert_eq!(s["is_partial"], false);
        assert_eq!(s["result_summary"]["visibility"], "full_text_observed");
        assert_eq!(
            s["dedupe_key"], "retrieval_segment:call-1:0",
            "段身份键 = call + 项序号"
        );
        let d = delivered_payload("call-1", 0, 0);
        assert_eq!(d["suppressed"], true);
        assert_eq!(d["suppressed_reason"], "model_read_directly");
        assert_eq!(d["boundary"], "B1_tool_result");
        assert_eq!(d["delivery_mode"], "direct");
        assert_eq!(d["source_id"], "call-1");
    }

    #[test]
    fn stable_code_never_guessed() {
        assert_eq!(
            stable_code_from_error("network error: cause=network_no_response"),
            Some("network_no_response")
        );
        assert_eq!(stable_code_from_error("channel dead"), None);
        assert_eq!(stable_code_from_error(""), None);
    }

    #[test]
    fn switch_truthy_contract_and_default_off() {
        // §10.3 纪律：开关默认关（env 未设置 ⇒ 关；仅真实值开）。
        assert!(!switch_truthy("0"));
        assert!(!switch_truthy(""));
        assert!(!switch_truthy("off"));
        assert!(switch_truthy("on"));
        assert!(switch_truthy("TRUE"));
        assert!(switch_truthy(" 1 "));
        if std::env::var_os(SWITCH_ENV).is_none() {
            assert!(!switch_enabled(), "未设置开关时必须默认关");
        }
    }

    /// M2 队列纪律：同一事实身份（来源+id）只投一次——重复入队被拒。
    #[test]
    fn delivery_queue_dedupes_by_fact_identity() {
        let mut q = DeliveryQueue::new(DEFAULT_TTL_MS, DEFAULT_MAX_ROUNDS);
        let a = PendingFact::background_task("task-1", "fact a".into(), 1_000, 1);
        let dup = PendingFact::background_task("task-1", "fact a".into(), 1_100, 1);
        assert!(q.admit(a));
        assert!(!q.admit(dup), "同一 dedupe_key 不得二次入队");
        let out = q.due(1, 1_200);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].mode, MODE_DIRECT);
        assert_eq!(out[0].latency_ms, 200);
        assert_eq!(out[0].fact.dedupe_key, "background_task:task-1");
        assert!(q.is_empty(), "投出即出队");
    }

    /// 降级投递（§2.2）：逾期事实（超轮数 / 超 TTL）一次完整带出为 digest。
    #[test]
    fn overdue_facts_degrade_to_digest() {
        let mut q = DeliveryQueue::new(1_000, 2);
        assert!(q.admit(PendingFact::background_task("task-t", "t".into(), 0, 1)));
        // 同一轮到达 ⇒ 不逾期 ⇒ direct（class 保持 I1）。
        let first = &q.due(1, 10)[0];
        assert_eq!(first.mode, MODE_DIRECT);
        assert_eq!(first.fact.class, CLASS_I1);

        assert!(q.admit(PendingFact::background_task("task-r", "r".into(), 0, 1)));
        assert_eq!(q.due(3, 10)[0].mode, MODE_DIGEST, "超轮数上限即降级");
        assert!(q.admit(PendingFact::background_task("task-ttl", "x".into(), 0, 1)));
        let degraded = &q.due(1, 5_000)[0];
        assert_eq!(degraded.mode, MODE_DIGEST, "超 TTL 即降级");
        assert_eq!(degraded.fact.class, CLASS_I3, "降级交付如实改记 I3");
    }

    /// ③ 跨 run 时序钉子（§4.2「不跨 run 存活」的另一面）：队列状态是
    /// per-run 的——新 run（新队列）可再次接纳同一事实身份；旧 run 的
    /// close_drop 之后 due 恒空（见 close_drop_clears_the_queue）。
    #[test]
    fn fresh_queue_per_run_readmits_the_same_fact_identity() {
        let mut run_one = DeliveryQueue::new(DEFAULT_TTL_MS, DEFAULT_MAX_ROUNDS);
        assert!(run_one.admit(PendingFact::background_task("task-x", "x".into(), 0, 1)));
        assert_eq!(run_one.due(1, 10).len(), 1);
        let mut run_two = DeliveryQueue::from_env();
        assert!(
            run_two.admit(PendingFact::background_task("task-x", "x".into(), 100, 2)),
            "新 run 的队列不得继承上一 run 的去重状态"
        );
    }

    /// ③ 时序钉子（§10.3-2②）：`result_delivered` 与真实投递**一一对应**
    /// ——每条 Delivery 恰好产生一个 unsuppressed payload，dedupe_key
    /// 逐字一致；「投了但被抑制」不得混记（真投递恒 suppressed=false
    /// 且不带 suppressed_reason）。
    #[test]
    fn each_delivery_maps_to_exactly_one_unsuppressed_payload() {
        let mut q = DeliveryQueue::new(DEFAULT_TTL_MS, DEFAULT_MAX_ROUNDS);
        assert!(q.admit(PendingFact::background_task("a", "a".into(), 0, 1)));
        assert!(q.admit(PendingFact::background_task("b", "b".into(), 0, 1)));
        let deliveries = q.due(9, 999_999);
        assert_eq!(deliveries.len(), 2);
        for d in &deliveries {
            let payload = delivered_fact_payload(d, BOUNDARY_B1);
            assert_eq!(payload["suppressed"], false);
            assert!(payload.get("suppressed_reason").is_none());
            assert_eq!(payload["dedupe_key"], json!(d.fact.dedupe_key));
            assert_eq!(payload["delivery_mode"], json!(d.mode));
            assert_eq!(payload["delivery_class"], json!(d.fact.class));
            assert_eq!(payload["latency_ms"], json!(d.latency_ms));
        }
        let keys: Vec<&str> = deliveries
            .iter()
            .map(|d| d.fact.dedupe_key.as_str())
            .collect();
        assert_ne!(keys[0], keys[1], "一次投递一个键，不合并");
    }

    /// 关闭即删（§4.2）：run 关闭清空队列并交还被丢弃事实（不跨 run 存活）。
    #[test]
    fn close_drop_clears_the_queue() {
        let mut q = DeliveryQueue::from_env();
        assert!(q.admit(PendingFact::background_task("task-c", "c".into(), 0, 1)));
        let dropped = q.close_drop();
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].source_id, "task-c");
        assert!(q.is_empty());
        assert!(q.due(9, 9_999).is_empty(), "关闭后不得再投");
    }

    /// 真投递 payload 形态：suppressed=false 且**不得**携带 suppressed_reason
    /// （schema allOf 的 else 分支）。
    #[test]
    fn delivered_fact_payload_is_an_unsuppressed_delivery() {
        let mut q = DeliveryQueue::new(DEFAULT_TTL_MS, DEFAULT_MAX_ROUNDS);
        q.admit(PendingFact::background_task(
            "task-9",
            "text".into(),
            500,
            2,
        ));
        let d = &q.due(2, 900)[0];
        let p = delivered_fact_payload(d, BOUNDARY_B2);
        assert_eq!(p["result_source"], SOURCE_BACKGROUND_TASK);
        assert_eq!(p["source_id"], "task-9");
        assert_eq!(p["boundary"], BOUNDARY_B2);
        assert_eq!(p["delivery_mode"], MODE_DIRECT);
        assert_eq!(p["delivery_class"], CLASS_I1);
        assert_eq!(p["suppressed"], false);
        assert!(p.get("suppressed_reason").is_none());
        assert_eq!(p["latency_ms"], 400);
        assert!(p["delivered_at"].is_string());
        assert_eq!(
            render_delivery_message(std::slice::from_ref(d)),
            "text",
            "投递文本由来源侧单一源格式化，loop 只做拼接"
        );
    }

    /// M3：中途读数用 stage=progress + 逐次唯一去重键（不猜稳定码）。
    #[test]
    fn progress_tick_payload_shape() {
        let p = progress_tick_payload("web_search", "call-7", 61_000, 1);
        assert_eq!(p["stage"], "progress");
        assert_eq!(p["retrieval_path"], PATH_LOCAL_SEGMENTED);
        assert_eq!(p["waited_ms"], 61_000);
        assert_eq!(p["dedupe_key"], "retrieval_progress:call-7:progress:1");
        assert_eq!(p["deadline_ms"], DEFAULT_DEADLINE_MS);
        assert!(p.get("stable_code").is_none());
        let p2 = progress_tick_payload("web_search", "call-7", 121_000, 2);
        assert_ne!(p["dedupe_key"], p2["dedupe_key"]);
    }

    /// M1 子开关从属于主开关（A/B 面：主开关关 ⇒ M1 恒定关）。
    #[test]
    fn m1_switch_is_subordinate_to_master_switch() {
        if std::env::var_os(SWITCH_ENV).is_none() && std::env::var_os(M1_SWITCH_ENV).is_none() {
            assert!(!m1_enabled(), "主开关未开时 M1 必须关");
        }
    }
}
