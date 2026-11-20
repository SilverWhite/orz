//! 0ac S3①-b (2026-09-14, IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL
//! _DESIGN §5/§9/§10.3①): **即时结果反馈的到达面/交付面写点**。
//!
//! S2 落契（`runtime/retrieval-progress-event-payload-v0.2.schema.json`、
//! `retrieval-result-segment-event-payload-v0.2.schema.json`、
//! `result-delivered-event-payload-v0.2.schema.json` + registry + 法官 fixtures），
//! 本模块是 S3①-b 的**产品码写入侧**：只做三件机械事——
//!
//! 1. 开关（`ORZ_IMMEDIATE_RESULT_DELIVERY`；0dv 403 起缺省开，关臂
//!    保留；§10.3 全部带开关）；
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

/// 开关（0dv 403 起缺省开；§10.3 全部带开关的关臂保留＝`0` 整体关闭）。
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

/// 开关判读：真实值 `1/true/on/yes`（大小写不敏感）为开；**缺省开**
/// （0dv S2 裁决 1，2026-11-19——0ac §10.3「默认关」缺省姿态随 402 设计稿
/// 修订：不翻则 idle 唤醒了也无投递通道；`0/false/off/no/未设` 关臂仍
/// 可整体关闭）。历史：0ac S3①-b 起缺省关两年，0dv 402/403 批翻案。
pub fn switch_enabled() -> bool {
    match std::env::var(SWITCH_ENV) {
        Ok(v) => switch_truthy(&v),
        Err(_) => true,
    }
}

/// 开关取值判读（纯函数；仅真实值开，其余一律关——含空串/垃圾值，
/// 缺省开由调用方给 `Err` 分支落 true）。
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

/// M1（收尾注入）开关 env——**0dv 403 批合并退役**：`m1_enabled()` 已
/// 坍缩为主开关单键，本 env 不再门控（兼容读取：设值时启动面一行告知，
/// 见 `m1_env_set`）。常量保留供 orz-bin 告知行与历史 env 识别。
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
/// 0dv S2（403 批，2026-11-19）：idle 唤醒边界（schema 枚举第四位，
/// 就地加位——历史 journal 回放安全）。唤醒 run 开局投递专用。
pub const BOUNDARY_IDLE_WAKE: &str = "idle_wake";
/// 投递方式（schema `delivery_mode` 闭枚举；`sentence_resume` 只随 B3）。
pub const MODE_DIRECT: &str = "direct";
pub const MODE_DIGEST: &str = "digest";
/// 投递分级（schema `delivery_class` 闭枚举）。
pub const CLASS_I1: &str = "I1_immediate_material";
pub const CLASS_I3: &str = "I3_deferred_digest";
/// 来源类型（schema `result_source` 闭枚举第一位）。
pub const SOURCE_BACKGROUND_TASK: &str = "background_task";

// ── 0dv S2（403 批，BG-DONE-IDLE-WAKE 设计 §2.2/§2.4）：idle 唤醒开关面 ──

/// 唤醒 run 的 prompt 机械源标记（era 机械源判定与 loop 开局投递判定
/// 同用此前缀；用户手打同名前缀会被当机械 run 处理——设计上可接受，
/// 机械前缀非用户词汇表成员）。
pub const IDLE_WAKE_TRIGGER: &str = "[idle_wake]";
/// 唤醒 run 开局 prompt 全文（触发器＋说明行；事实本体由 run 开局
/// drain 后注入，prompt 只作机械触发，见 agent_loop 开局投递块）。
pub const IDLE_WAKE_PROMPT: &str = "[idle_wake] 后台任务完成机械唤醒。";
/// 唤醒子开关（设计裁决：`ORZ_IDLE_WAKE` 缺省 on、从属主开关）。
pub const IDLE_WAKE_ENV: &str = "ORZ_IDLE_WAKE";
/// 唤醒去抖合并窗 ms（设计 §2.2：窗内后续完成事实并入一条投递；
/// 窗内用户 prompt 到达 ⇒ 取消唤醒、facts 留账 B1 收走）。
pub const IDLE_WAKE_DEBOUNCE_ENV: &str = "ORZ_IDLE_WAKE_DEBOUNCE_MS";
pub const DEFAULT_IDLE_WAKE_DEBOUNCE_MS: u64 = 5_000;
/// 去抖窗下限（防 0/负窗退化为无限连发）。
pub const MIN_IDLE_WAKE_DEBOUNCE_MS: u64 = 1_000;

/// 唤醒开关判读：主开关 **且** 唤醒子开关为真实值（子开关缺省开）。
pub fn idle_wake_enabled() -> bool {
    switch_enabled()
        && match std::env::var(IDLE_WAKE_ENV) {
            Ok(v) => switch_truthy(&v),
            Err(_) => true,
        }
}

/// 唤醒去抖窗 ms（env 覆盖；非法/低于下限 = 缺省 5s）。
pub fn idle_wake_debounce_ms() -> u64 {
    std::env::var(IDLE_WAKE_DEBOUNCE_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v >= MIN_IDLE_WAKE_DEBOUNCE_MS)
        .unwrap_or(DEFAULT_IDLE_WAKE_DEBOUNCE_MS)
}

/// 唤醒 run 开局投递的机械核（0dv 403）：drain 事实 → 队列（round 0
/// admit）→ 边界内交付列表。journal 落账与消息注入由 loop 开局块执行
/// （`delivered_fact_payload`＋`BOUNDARY_IDLE_WAKE`＋`render_idle_wake_
/// message`）；抽纯函数供单测钉测（`(task_id, report)` 解耦 host 类型）。
pub fn wake_open_deliveries(
    facts: Vec<(String, String)>,
    queue: &mut DeliveryQueue,
    now_ms: u64,
) -> Vec<Delivery> {
    for (task_id, report) in facts {
        queue.admit(PendingFact::background_task(&task_id, report, now_ms, 0));
    }
    queue.due(0, now_ms)
}

/// 唤醒 run 开局投递消息头（机械事实＋一句引导语，设计裁决 3；多任务
/// 合并一条——`render_delivery_message` 同构拼接）。头常量单独导出供
/// 写回过滤生成形态判据同用（404 处置：唤醒投递消息与 B1/B2 投递消息
/// 同族待遇——机械注入块不写回会话侧车，见
/// `prompt::is_generated_injected_block`）。
pub const IDLE_WAKE_DELIVERY_HEADER: &str = "[结果投递] 后台任务已完成，结果如下：\n\n";

pub fn render_idle_wake_message(deliveries: &[Delivery]) -> String {
    format!(
        "{}{}",
        IDLE_WAKE_DELIVERY_HEADER,
        render_delivery_message(deliveries)
    )
}

/// M1 判读（**0dv 403 批退役**）：双门坍缩为主开关单键——本函数与
/// [`switch_enabled`] 等价，仅为既有调用面保留名号。设过 M1 env 的部署
/// 由启动面（orz-bin）一行告知，不再产生行为差。
pub fn m1_enabled() -> bool {
    switch_enabled()
}

/// M1 兼容告知面（0dv 403）：env 设过即 true——启动面据此打印一行
/// 「设值不再门控」告知，不改变任何行为。
pub fn m1_env_set() -> bool {
    std::env::var(M1_SWITCH_ENV).is_ok()
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
    fn switch_truthy_contract_and_default_on() {
        // 0dv 403 纪律：开关缺省开（env 未设置 ⇒ 开；仅真实值开、其余关）。
        assert!(!switch_truthy("0"));
        assert!(!switch_truthy(""));
        assert!(!switch_truthy("off"));
        assert!(switch_truthy("on"));
        assert!(switch_truthy("TRUE"));
        assert!(switch_truthy(" 1 "));
        if std::env::var_os(SWITCH_ENV).is_none() {
            assert!(switch_enabled(), "未设置开关时必须默认开（0dv 403 翻案）");
        }
    }

    /// 0dv 403：M1 双门坍缩——`m1_enabled` 与主开关恒等（M1 env 设值不再
    /// 门控，仅启动面告知）。
    #[test]
    fn m1_gate_collapsed_onto_main_switch() {
        assert_eq!(m1_enabled(), switch_enabled());
        assert_eq!(m1_env_set(), std::env::var_os(M1_SWITCH_ENV).is_some());
    }

    /// 0dv 403：唤醒开关面——子开关关 ⇒ 唤醒必关（对主开关任意姿态成立，
    /// 避免并行测试互踩主开关 env）；去抖窗 env 覆盖与下限钳制。
    #[test]
    fn idle_wake_switch_and_debounce_contract() {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        // 子开关关臂（从属关系：无论主开关姿态，子开关关 ⇒ 唤醒关）。
        unsafe { std::env::set_var(IDLE_WAKE_ENV, "0") };
        assert!(!idle_wake_enabled());
        unsafe { std::env::set_var(IDLE_WAKE_ENV, "off") };
        assert!(!idle_wake_enabled());
        unsafe { std::env::remove_var(IDLE_WAKE_ENV) };
        // 去抖窗：非法/低于下限 = 缺省 5s；合法值直通。
        unsafe { std::env::set_var(IDLE_WAKE_DEBOUNCE_ENV, "12000") };
        assert_eq!(idle_wake_debounce_ms(), 12_000);
        unsafe { std::env::set_var(IDLE_WAKE_DEBOUNCE_ENV, "500") };
        assert_eq!(
            idle_wake_debounce_ms(),
            DEFAULT_IDLE_WAKE_DEBOUNCE_MS,
            "低于下限钳回缺省"
        );
        unsafe { std::env::set_var(IDLE_WAKE_DEBOUNCE_ENV, "garbage") };
        assert_eq!(idle_wake_debounce_ms(), DEFAULT_IDLE_WAKE_DEBOUNCE_MS);
        unsafe { std::env::remove_var(IDLE_WAKE_DEBOUNCE_ENV) };
        assert_eq!(idle_wake_debounce_ms(), DEFAULT_IDLE_WAKE_DEBOUNCE_MS);
    }

    /// 0dv 403：唤醒开局机械核——facts → 队列 → direct 交付列表；
    /// 空 facts 恒空列表（竞态极小窗：唤醒 run 开局账本已被清）。
    #[test]
    fn wake_open_deliveries_maps_facts_to_direct_deliveries() {
        let mut q = DeliveryQueue::from_env();
        let empty = wake_open_deliveries(Vec::new(), &mut q, 1_000);
        assert!(empty.is_empty());
        let deliveries = wake_open_deliveries(
            vec![
                ("task-a".into(), "report a".into()),
                ("task-b".into(), "report b".into()),
            ],
            &mut q,
            1_000,
        );
        assert_eq!(deliveries.len(), 2);
        assert_eq!(deliveries[0].fact.source_id, "task-a");
        assert_eq!(deliveries[0].mode, MODE_DIRECT);
        assert_eq!(deliveries[0].fact.class, CLASS_I1);
        assert_eq!(deliveries[0].latency_ms, 0, "开局投递延迟为零");
        let keys: Vec<&str> = deliveries
            .iter()
            .map(|d| d.fact.dedupe_key.as_str())
            .collect();
        assert_eq!(
            keys,
            vec!["background_task:task-a", "background_task:task-b"]
        );
    }

    /// 0dv 403：唤醒开局投递消息形态（机械事实＋引导语＋合并拼接）。
    #[test]
    fn idle_wake_message_leads_with_mechanical_fact_and_guide() {
        let mut q = DeliveryQueue::new(DEFAULT_TTL_MS, DEFAULT_MAX_ROUNDS);
        assert!(q.admit(PendingFact::background_task("t1", "report a".into(), 0, 0)));
        assert!(q.admit(PendingFact::background_task("t2", "report b".into(), 0, 0)));
        let deliveries = q.due(0, 10);
        assert_eq!(deliveries.len(), 2);
        let text = render_idle_wake_message(&deliveries);
        assert!(text.starts_with("[结果投递] 后台任务已完成，结果如下："));
        assert!(text.contains("report a"));
        assert!(text.contains("report b"));
        assert!(
            text.contains("report a\n\nreport b"),
            "多任务合并一条按序拼接"
        );
        // payload 边界位：idle_wake 扩位经由同一 delivered_fact_payload。
        let p = delivered_fact_payload(&deliveries[0], BOUNDARY_IDLE_WAKE);
        assert_eq!(p["boundary"], "idle_wake");
        assert_eq!(p["suppressed"], false);
        assert!(p.get("suppressed_reason").is_none());
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

    /// M1 子开关从属钉（0dv 403 改判重写）：双门坍缩为单键后，原
    /// 「主开关关 ⇒ M1 恒关」前提消失——`m1_enabled` 与主开关恒等，
    /// 见 `m1_gate_collapsed_onto_main_switch`；原钉名保留改判，
    /// M1 env 设值不再门控，仅启动面告知。
    #[test]
    fn m1_switch_is_subordinate_to_master_switch() {
        assert_eq!(
            m1_enabled(),
            switch_enabled(),
            "403 合并退役：M1 门与主开关恒等"
        );
    }

    // ── 0ag 契约面机械对账（2026-09-15；立项时初名 0af，与邻线资源门文案项撞号后按先占改名 0ag）───────────────────────
    //
    // 摩擦 C 治本（交接件 [`0AC_S3B_HANDOVER_2026-09-15`] §7-C / §5-A）：
    // 「实现侧先写常量、契约枚举不参与编译期校验」是事件面静默失配的
    // 温床。本钉子把 §5-A 人工核对点升级为**机械对账**——S2 契约 schema
    // 闭枚举与实现常量在测试期逐字互证：实现常量 ⊆ schema 枚举（写出去
    // 的事件必被契约接受）+ 稳定码闭集全等（不猜码纪律的契约侧镜像）。

    fn runtime_schema(name: &str) -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../runtime")
            .join(name);
        serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("runtime schema readable: {}: {e}", path.display())),
        )
        .expect("schema json")
    }

    fn schema_enum(schema: &serde_json::Value, pointer: &str) -> Vec<String> {
        schema
            .pointer(pointer)
            .and_then(|v| v.get("enum"))
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .map(|x| x.as_str().expect("enum string").to_string())
                    .collect()
            })
            .unwrap_or_else(|| panic!("enum at {pointer}"))
    }

    #[test]
    fn schema_closed_enums_verbatim_match_implementation_constants() {
        let delivered = runtime_schema("result-delivered-event-payload-v0.2.schema.json");
        // 实现常量 ⊆ schema 闭枚举（B3/sentence_resume/I2 是契约预留形态，
        // 实现未用——单向包含而非全等）。
        let boundary = schema_enum(&delivered, "/properties/boundary");
        for constant in [BOUNDARY_B1, BOUNDARY_B2] {
            assert!(
                boundary.contains(&constant.to_string()),
                "{constant} ∉ {boundary:?}"
            );
        }
        let mode = schema_enum(&delivered, "/properties/delivery_mode");
        for constant in [MODE_DIRECT, MODE_DIGEST] {
            assert!(
                mode.contains(&constant.to_string()),
                "{constant} ∉ {mode:?}"
            );
        }
        let class = schema_enum(&delivered, "/properties/delivery_class");
        for constant in [CLASS_I1, CLASS_I3] {
            assert!(
                class.contains(&constant.to_string()),
                "{constant} ∉ {class:?}"
            );
        }
        let source = schema_enum(&delivered, "/properties/result_source");
        assert!(
            source.contains(&SOURCE_BACKGROUND_TASK.to_string()),
            "{SOURCE_BACKGROUND_TASK} ∉ {source:?}"
        );
        // 抑制码全等（「投了但被抑制」的记账面闭集）。
        let suppressed = schema_enum(&delivered, "/properties/suppressed_reason");
        assert_eq!(
            suppressed,
            vec![
                "duplicate".to_string(),
                "model_read_directly".to_string(),
                "class_capped".to_string()
            ]
        );

        let progress = runtime_schema("retrieval-progress-event-payload-v0.2.schema.json");
        let stage = schema_enum(&progress, "/properties/stage");
        for constant in [
            STAGE_DISPATCHED,
            STAGE_PROGRESS,
            STAGE_FAILED,
            STAGE_FINISHED,
        ] {
            assert!(
                stage.contains(&constant.to_string()),
                "{constant} ∉ {stage:?}"
            );
        }
        let path_enum = schema_enum(&progress, "/properties/retrieval_path");
        assert!(path_enum.contains(&PATH_LOCAL_SEGMENTED.to_string()));
        // 稳定码闭集全等（schema 侧镜像 = 不猜码纪律）。
        let codes = schema_enum(&progress, "/properties/stable_code");
        assert_eq!(
            codes,
            vec![
                "capability_unreachable".to_string(),
                "network_no_response".to_string(),
                "network_error".to_string(),
                "empty_result".to_string(),
                "no_progress".to_string()
            ]
        );

        // 0ae D0：blackboard_write 的 section 扩展枚举与 ModelNoteSection
        // 同源（复用 plan_write 事件族的契约面）；0cu（2026-10-07）+findings。
        let plan_write = runtime_schema("plan-write-event-payload-v0.2.schema.json");
        let section = schema_enum(&plan_write, "/properties/section");
        assert_eq!(
            section,
            vec![
                "plan".to_string(),
                "notes".to_string(),
                "findings".to_string()
            ]
        );
    }
}
