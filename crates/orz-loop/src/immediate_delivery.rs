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

use serde_json::{json, Value};

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
            s["dedupe_key"],
            "retrieval_segment:call-1:0",
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
}
