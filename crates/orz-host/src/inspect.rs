//! 354b（0dr，2026-10-25，SYSTEM_CONSOLE_DESIGN §3.2/§3.3）：系统控制台
//! inspect 族——`session`／`run`／`context`／`events` 四个**只读**诊断面。
//! 数据源全部是落盘工件（`.gsa/` 侧车与 run journal），零审批、零副作用、
//! 不进模型工具面（除非模型主动调——经 CLI `orz inspect` 供人/管道消费）。
//!
//! Envelope v1（§3.3）：本模块的 JSON 输出一律走统一信封
//! `{ok, data, error, meta{chars, lines, encoding, truncated, next_cursor,
//! sha256, took_ms}}`——inspect／capabilities 是 Envelope v1 的首批公民；
//! 模型工具面（read_file/grep/terminal）各自已有等价 meta 形态（ReadHandle
//! 信封／`[output]` 三读数／`[page]` trailer），整面迁移登记为 0dr 剩余
//! 边界（信息等价、纯形状统一，§7.5 预算下不冒险热路径）。
//!
//! 只读纪律：本模块只 `read`＋`read_dir`，绝不写盘；解析失败如实报错
//! （envelope `ok:false`＋`error.code`），不静默截断。

use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{Value, json};

/// Envelope v1 meta（§5 字段表）。`encoding` 对本模块恒 `utf-8`（输出面
/// 即 serde 文本）；`next_cursor` 目前只有 `events --limit` 分页使用。
/// 355 批（2026-10-25，审查条目 3）：`coverage` 携带能力清单的覆盖面
/// 说明（capabilities 专用；inspect 四面不携带，序列化为 null）。
#[derive(Debug)]
pub struct EnvelopeMeta {
    pub chars: usize,
    pub lines: usize,
    pub truncated: bool,
    pub next_cursor: Option<String>,
    pub sha256: Option<String>,
    pub took_ms: u128,
    pub coverage: Option<String>,
}

impl EnvelopeMeta {
    fn to_value(&self) -> Value {
        json!({
            "chars": self.chars,
            "lines": self.lines,
            "encoding": "utf-8",
            "truncated": self.truncated,
            "next_cursor": self.next_cursor,
            "sha256": self.sha256,
            "took_ms": self.took_ms,
            "coverage": self.coverage,
        })
    }
}

fn envelope(ok: bool, data: Value, error: Option<(String, String)>, meta: EnvelopeMeta) -> Value {
    let mut envelope = json!({
        "ok": ok,
        "data": data,
        "error": error.map(|(code, message)| json!({"code": code, "message": message})),
        "meta": meta.to_value(),
    });
    if envelope["error"].is_null() {
        envelope["error"] = Value::Null;
    }
    envelope
}

// Envelope meta 的 sha256 字段：本模块的输出面（serde JSON）自身就是
// 全文交付，无再核证场景；工具面（read_file 锚点）已有独立 sha 通道。
// orz-host 不引 sha2 依赖，字段保留占位 None（Envelope v1 字段表全型）。

fn count_lines(text: &str) -> usize {
    text.lines().count()
}

/// run journal 目录：`.gsa/runs/<run_id>/events.jsonl`。
fn run_journal_path(base_dir: &Path, run_id: &str) -> PathBuf {
    base_dir
        .join(".gsa")
        .join("runs")
        .join(run_id)
        .join("events.jsonl")
}

fn read_events(base_dir: &Path, run_id: &str) -> Result<Vec<Value>, (String, String)> {
    let path = run_journal_path(base_dir, run_id);
    let text = std::fs::read_to_string(&path).map_err(|e| {
        (
            "journal_unreadable".to_string(),
            format!("{}: {e}", path.display()),
        )
    })?;
    let mut events = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event: Value = serde_json::from_str(line).map_err(|e| {
            (
                "journal_corrupt".to_string(),
                format!("line {}: {e}", idx + 1),
            )
        })?;
        events.push(event);
    }
    Ok(events)
}

/// 会话侧车：`.gsa/conversations/<session_id>.json`（StoredConversation
/// 宽松值解析——只读诊断不耦合 host 私有结构）。
fn conversation_sidecar(base_dir: &Path, session_id: &str) -> Result<Value, (String, String)> {
    let path = base_dir
        .join(".gsa")
        .join("conversations")
        .join(format!("{session_id}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| {
        (
            "sidecar_unreadable".to_string(),
            format!("{}: {e}", path.display()),
        )
    })?;
    serde_json::from_str(&text).map_err(|e| {
        (
            "sidecar_corrupt".to_string(),
            format!("{}: {e}", path.display()),
        )
    })
}

/// 悬挂调用扫描（353 P0-2 同口径的只读投影）：assistant 声明的 call_id
/// 中没有对应 `tool` 结果消息的——断 run 后「结果未知」的事实面。
fn unrecovered_tool_calls(messages: &[Value]) -> Vec<Value> {
    let mut answered: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut declared: Vec<(String, String)> = Vec::new();
    for message in messages {
        if message.get("role").and_then(Value::as_str) == Some("tool") {
            if let Some(id) = message.get("tool_call_id").and_then(Value::as_str) {
                answered.insert(id.to_string());
            }
        }
        for call in message
            .get("tool_calls")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = call.get("call_id").and_then(Value::as_str) {
                let name = call.get("name").and_then(Value::as_str).unwrap_or("?");
                declared.push((id.to_string(), name.to_string()));
            }
        }
    }
    declared
        .into_iter()
        .filter(|(id, _)| !answered.contains(id))
        .map(|(id, name)| json!({"call_id": id, "tool": name}))
        .collect()
}

fn message_stats(messages: &[Value]) -> (usize, usize, usize) {
    let mut chars = 0usize;
    let mut tool_calls = 0usize;
    for message in messages {
        chars += message
            .get("content")
            .and_then(Value::as_str)
            .map(str::chars)
            .map(Iterator::count)
            .unwrap_or(0);
        tool_calls += message
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|a| a.len())
            .unwrap_or(0);
    }
    (messages.len(), chars, tool_calls)
}

/// capabilities 等 host 侧面的 Envelope v1 组装（inspect 四面同构——
/// 字段表 §5；took_ms 由调用方自带）。355 批（2026-10-25，审查条目 3）：
/// `coverage` 进 meta——能力清单如实标注覆盖面（inspect 调用方传 None）。
pub fn envelope_for(data: &Value, took_ms: u128, coverage: Option<String>) -> Value {
    let rendered = serde_json::to_string_pretty(data).unwrap_or_default();
    envelope(
        true,
        data.clone(),
        None,
        EnvelopeMeta {
            chars: rendered.chars().count(),
            lines: count_lines(&rendered),
            truncated: false,
            next_cursor: None,
            sha256: None,
            took_ms,
            coverage,
        },
    )
}

/// `inspect session <id>`：轮次/水位/未回收调用/撤回历史/租约与墓碑态。
pub fn session_overview(base_dir: &Path, session_id: &str) -> Value {
    let started = Instant::now();
    let gsa = base_dir.join(".gsa");
    let lease = gsa.join("sessions").join(format!("{session_id}.lease"));
    let tombstone = gsa.join("sessions").join(format!("{session_id}.deleted"));
    let data = match conversation_sidecar(base_dir, session_id) {
        Ok(sidecar) => {
            let messages = sidecar
                .get("messages")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let (count, chars, tool_calls) = message_stats(&messages);
            let blackboard_chars = sidecar
                .get("blackboard")
                .map(|b| serde_json::to_string(b).map(|s| s.len()).unwrap_or(0))
                .unwrap_or(0);
            json!({
                "session_id": session_id,
                "exists": true,
                "message_count": count,
                "message_chars": chars,
                "tool_call_count": tool_calls,
                "unrecovered_tool_calls": unrecovered_tool_calls(&messages),
                "blackboard_chars": blackboard_chars,
                "lif_decision_rounds": sidecar
                    .get("lif")
                    .and_then(|l| l.get("decision_rounds"))
                    .cloned()
                    .unwrap_or(Value::Null),
                "recall_log_entries": sidecar
                    .get("recall_log")
                    .and_then(Value::as_array)
                    .map(|a| a.len())
                    .unwrap_or(0),
                "reconstructed_from_journal": sidecar
                    .get("reconstructed_from_journal")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                "lease_present": lease.is_file(),
                "tombstoned": tombstone.is_file(),
                "schema_version": sidecar.get("schema_version").cloned().unwrap_or(Value::Null),
            })
        }
        Err((code, message)) => {
            // 会话不存在（无侧车）≠ 诊断失败——如实给缺席事实。
            json!({
                "session_id": session_id,
                "exists": false,
                "reason": format!("{code}: {message}"),
                "lease_present": lease.is_file(),
                "tombstoned": tombstone.is_file(),
            })
        }
    };
    let rendered = serde_json::to_string_pretty(&data).unwrap_or_default();
    envelope(
        true,
        data,
        None,
        EnvelopeMeta {
            chars: rendered.chars().count(),
            lines: count_lines(&rendered),
            truncated: false,
            next_cursor: None,
            sha256: None,
            took_ms: started.elapsed().as_millis(),
            coverage: None,
        },
    )
}

/// `inspect run <id>`：单 run 时间线（模型轮 → 工具 → 审批 → 终态），
/// 机械事实从 events.jsonl 投影——零推断（§4.4 纪律）。
pub fn run_timeline(base_dir: &Path, run_id: &str) -> Value {
    let started = Instant::now();
    match read_events(base_dir, run_id) {
        Ok(events) => {
            let mut tools: std::collections::BTreeMap<String, Value> =
                std::collections::BTreeMap::new();
            let mut model_rounds = 0usize;
            let mut permission_events = Vec::new();
            let mut terminal: Option<Value> = None;
            let mut run_started_at: Option<String> = None;
            // 358c（2026-10-26）：scout 子 run 的父视图——子代理事件与主 run
            // 同 journal；`tool_completed(agent_role=scout)` 携带 child_run_id
            // 与报告 stats，inspect 据此列 children（不伪造独立 journal 目录）。
            let mut children: Vec<Value> = Vec::new();
            // 358 修复（审查 P1）：scout wrapper 的 ToolStarted→ToolCompleted
            // 之间是子代理的工具序列；按 child_run_id 归组。
            let mut current_child: Option<String> = None;
            let mut child_tool_calls: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            for event in &events {
                let event_type = event
                    .get("event_type")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match event_type {
                    "run_started" => {
                        run_started_at = event
                            .get("timestamp")
                            .and_then(Value::as_str)
                            .map(String::from);
                    }
                    "model_output" => model_rounds += 1,
                    "permission_requested" | "permission_decision" => {
                        permission_events.push(json!({
                            "type": event_type,
                            "tool": event["payload"]["tool"].clone(),
                            "decision": event["payload"]["decision"].clone(),
                            "timestamp": event.get("timestamp").cloned().unwrap_or(Value::Null),
                        }))
                    }
                    "tool_started" | "tool_completed" | "tool_heartbeat" | "tool_running" => {
                        let call_id = event["payload"]["call_id"]
                            .as_str()
                            .unwrap_or("?")
                            .to_string();
                        let entry = tools.entry(call_id.clone()).or_insert_with(|| {
                            json!({
                                "call_id": call_id,
                                "tool": event["payload"]["tool"].clone(),
                                "started_at": Value::Null,
                                "completed_at": Value::Null,
                                "exit_code": Value::Null,
                                "status": Value::Null,
                                "heartbeats": [],
                            })
                        });
                        match event_type {
                            "tool_started" => {
                                entry["started_at"] =
                                    event.get("timestamp").cloned().unwrap_or(Value::Null);
                                if event["payload"]["target"] == "scout" {
                                    current_child =
                                        event["payload"]["child_run_id"].as_str().map(String::from);
                                } else if let Some(child) = &current_child {
                                    let list = child_tool_calls.entry(child.clone()).or_default();
                                    if !list.contains(&call_id) {
                                        list.push(call_id.clone());
                                    }
                                }
                            }
                            "tool_completed" => {
                                entry["completed_at"] =
                                    event.get("timestamp").cloned().unwrap_or(Value::Null);
                                entry["exit_code"] = event["payload"]["exit_code"].clone();
                                entry["status"] = event["payload"]["status"].clone();
                                if event["payload"]["target"] == "scout" {
                                    if event["payload"]["agent_role"] == "scout" {
                                        let child_id =
                                            event["payload"]["child_run_id"].as_str().unwrap_or("");
                                        let child_tools = child_tool_calls
                                            .get(child_id)
                                            .map(|ids| {
                                                ids.iter()
                                                    .filter_map(|id| tools.get(id).cloned())
                                                    .collect::<Vec<_>>()
                                            })
                                            .unwrap_or_default();
                                        children.push(json!({
                                            "child_run_id": event["payload"]["child_run_id"].clone(),
                                            "parent_run_id": event["payload"]["parent_run_id"].clone(),
                                            "report_id": event["payload"]["report_id"].clone(),
                                            "status": event["payload"]["status"].clone(),
                                            "exit_code": event["payload"]["exit_code"].clone(),
                                            "files_read": event["payload"]["files_read"].clone(),
                                            "tokens_used": event["payload"]["tokens_used"].clone(),
                                            "wall_s": event["payload"]["wall_s"].clone(),
                                            "tools": child_tools,
                                        }));
                                        current_child = None;
                                    }
                                } else if let Some(child) = &current_child {
                                    let list = child_tool_calls.entry(child.clone()).or_default();
                                    if !list.contains(&call_id) {
                                        list.push(call_id.clone());
                                    }
                                }
                            }
                            "tool_heartbeat" => {
                                if let Some(list) = entry["heartbeats"].as_array_mut() {
                                    list.push(json!({
                                        "elapsed_ms": event["payload"]["elapsed_ms"].clone(),
                                        "timestamp": event.get("timestamp").cloned().unwrap_or(Value::Null),
                                    }));
                                }
                            }
                            _ => {}
                        }
                    }
                    "run_finished" | "run_failed" | "run_cancelled" | "run_invalidated" => {
                        terminal = Some(json!({
                            "event": event_type,
                            "status": event["payload"]["status"].clone(),
                            "timestamp": event.get("timestamp").cloned().unwrap_or(Value::Null),
                        }));
                    }
                    _ => {}
                }
            }
            let tool_list: Vec<Value> = tools.into_values().collect();
            let data = json!({
                "run_id": run_id,
                "event_count": events.len(),
                "run_started_at": run_started_at,
                "model_rounds": model_rounds,
                "tools": tool_list,
                "permissions": permission_events,
                "terminal": terminal,
                "children": children,
            });
            let rendered = serde_json::to_string_pretty(&data).unwrap_or_default();
            envelope(
                true,
                data,
                None,
                EnvelopeMeta {
                    chars: rendered.chars().count(),
                    lines: count_lines(&rendered),
                    truncated: false,
                    next_cursor: None,
                    sha256: None,
                    took_ms: started.elapsed().as_millis(),
                    coverage: None,
                },
            )
        }
        Err((code, message)) => envelope(
            false,
            Value::Null,
            Some((code, message)),
            EnvelopeMeta {
                chars: 0,
                lines: 0,
                truncated: false,
                next_cursor: None,
                sha256: None,
                took_ms: started.elapsed().as_millis(),
                coverage: None,
            },
        ),
    }
}

/// `inspect context <id>`：当前窗口构成投影——消息面计数/水位、注入块、
/// LIF 轮次、未回收调用、最近一次 face_fingerprint（从该会话最新 run
/// journal 回读）。窗口投影是**侧车近似**（live 视图属 loop 内存态），
/// 如实标注 `source: sidecar`。
pub fn context_view(base_dir: &Path, session_id: &str) -> Value {
    let started = Instant::now();
    let sidecar = match conversation_sidecar(base_dir, session_id) {
        Ok(s) => s,
        Err((code, message)) => {
            return envelope(
                false,
                Value::Null,
                Some((code, message)),
                EnvelopeMeta {
                    chars: 0,
                    lines: 0,
                    truncated: false,
                    next_cursor: None,
                    sha256: None,
                    took_ms: started.elapsed().as_millis(),
                    coverage: None,
                },
            );
        }
    };
    let messages = sidecar
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let (count, chars, tool_calls) = message_stats(&messages);
    let injected_blocks = messages
        .iter()
        .filter(|m| {
            m.get("role").and_then(Value::as_str) == Some("user")
                && m.get("content")
                    .and_then(Value::as_str)
                    .is_some_and(|c| c.starts_with('['))
        })
        .count();
    // 最近 run 的 face_fingerprint（窗口读数的机械事实源）。run 目录序
    // `RUN-<s8>-<n>`（acp_server `format!("RUN-{key}-{n}")`，序数不补零）。
    // 355 批（2026-10-25，审查条目 2）：字典序取最大在会话 ≥10 个 run 后
    // 失真（RUN-…-9 排在 RUN-…-10 之后）——按「前缀剥离后的尾段 u64 数字
    // 序」取最大；任一侧尾段解析失败（异形目录名）回退整名字典序。
    let runs_dir = base_dir.join(".gsa").join("runs");
    let mut latest_fingerprint: Value = Value::Null;
    let mut latest_run: Option<String> = None;
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        let run_ordinal = |name: &str| -> Option<u64> {
            name.rsplit('-').next().and_then(|t| t.parse::<u64>().ok())
        };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(&format!("RUN-{session_id}-")))
            .collect();
        names.sort_by(|a, b| match (run_ordinal(a), run_ordinal(b)) {
            (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.cmp(b)),
            _ => a.cmp(b),
        });
        if let Some(latest) = names.last().cloned() {
            latest_run = Some(latest.clone());
            if let Ok(events) = read_events(base_dir, &latest) {
                if let Some(fingerprint) = events
                    .iter()
                    .filter(|e| {
                        e.get("event_type").and_then(Value::as_str) == Some("face_fingerprint")
                    })
                    .last()
                {
                    latest_fingerprint = fingerprint.get("payload").cloned().unwrap_or(Value::Null);
                }
            }
        }
    }
    let data = json!({
        "session_id": session_id,
        "source": "sidecar",
        "message_count": count,
        "message_chars": chars,
        "tool_call_count": tool_calls,
        "injected_block_user_messages": injected_blocks,
        "unrecovered_tool_calls": unrecovered_tool_calls(&messages),
        "lif_decision_rounds": sidecar.get("lif").and_then(|l| l.get("decision_rounds")).cloned().unwrap_or(Value::Null),
        "latest_run": latest_run,
        "face_fingerprint": latest_fingerprint,
    });
    let rendered = serde_json::to_string_pretty(&data).unwrap_or_default();
    envelope(
        true,
        data,
        None,
        EnvelopeMeta {
            chars: rendered.chars().count(),
            lines: count_lines(&rendered),
            truncated: false,
            next_cursor: None,
            sha256: None,
            took_ms: started.elapsed().as_millis(),
            coverage: None,
        },
    )
}

/// `inspect events <id> [--limit N] [--kind TYPE]`：结构化事件流（稳定
/// schema passthrough）＋`next_cursor` 分页。
///
/// 355 批（2026-10-25，审查条目 1）：游标语义＝**绝对事件行号**（0 基，
/// 下一页读取起点）。此前实现按「本页已消费的匹配数」回写游标，而
/// `skip(cursor)` 按绝对行跳过——带 `--kind` 过滤时两把尺错位，第 2 页会
/// 重发第 1 页的匹配（构造 [Y,X,Y,X]、kind=X、limit=1：两页都发 X@1）。
/// 现改为按索引遍历：发满 `limit` 条后取「最后发出事件的下一索引」为
/// `consumed_cursor`；`has_more`＝其后是否仍有匹配（文件有界，扫到首个
/// 后续匹配即判定）。
pub fn events_stream(
    base_dir: &Path,
    run_id: &str,
    limit: Option<usize>,
    kind_filter: Option<&str>,
    cursor: usize,
) -> Value {
    let started = Instant::now();
    match read_events(base_dir, run_id) {
        Ok(events) => {
            let take = limit.unwrap_or(events.len());
            let mut page: Vec<Value> = Vec::new();
            let mut consumed_cursor = cursor;
            let mut has_more = false;
            for (idx, event) in events.iter().enumerate() {
                if idx < cursor {
                    continue;
                }
                let kind_matches = kind_filter.is_none_or(|kind| {
                    event.get("event_type").and_then(Value::as_str) == Some(kind)
                });
                if !kind_matches {
                    continue;
                }
                if page.len() < take {
                    page.push(event.clone());
                    consumed_cursor = idx + 1;
                } else {
                    has_more = true;
                    break;
                }
            }
            let next_cursor = has_more.then(|| consumed_cursor.to_string());
            let data = json!({
                "run_id": run_id,
                "total_events": events.len(),
                "events": page,
            });
            let rendered = serde_json::to_string_pretty(&data).unwrap_or_default();
            envelope(
                true,
                data,
                None,
                EnvelopeMeta {
                    chars: rendered.chars().count(),
                    lines: count_lines(&rendered),
                    truncated: has_more,
                    next_cursor,
                    sha256: None,
                    took_ms: started.elapsed().as_millis(),
                    coverage: None,
                },
            )
        }
        Err((code, message)) => envelope(
            false,
            Value::Null,
            Some((code, message)),
            EnvelopeMeta {
                chars: 0,
                lines: 0,
                truncated: false,
                next_cursor: None,
                sha256: None,
                took_ms: started.elapsed().as_millis(),
                coverage: None,
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-inspect-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn seed_run(base: &Path, run_id: &str, events: &[Value]) {
        let dir = base.join(".gsa").join("runs").join(run_id);
        std::fs::create_dir_all(&dir).unwrap();
        let text = events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(dir.join("events.jsonl"), text).unwrap();
    }

    fn event(kind: &str, payload: Value) -> Value {
        json!({
            "event_type": kind,
            "timestamp": "2026-10-25T12:00:00+00:00",
            "payload": payload,
        })
    }

    #[test]
    fn run_timeline_aggregates_tools_permissions_and_heartbeat() {
        let base = tempdir();
        seed_run(
            &base,
            "RUN-s1-1",
            &[
                event("run_started", json!({})),
                event(
                    "tool_started",
                    json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                ),
                event(
                    "tool_heartbeat",
                    json!({"tool": "run_terminal_cmd", "call_id": "c1", "elapsed_ms": 20_000}),
                ),
                event(
                    "tool_completed",
                    json!({"tool": "run_terminal_cmd", "call_id": "c1", "exit_code": 0}),
                ),
                event("run_finished", json!({"status": "completed"})),
            ],
        );
        let out = run_timeline(&base, "RUN-s1-1");
        assert_eq!(out["ok"], true);
        assert_eq!(out["data"]["model_rounds"], 0);
        assert_eq!(out["data"]["tools"].as_array().unwrap().len(), 1);
        let tool = &out["data"]["tools"][0];
        assert_eq!(tool["call_id"], "c1");
        assert_eq!(tool["exit_code"], 0);
        assert_eq!(tool["heartbeats"].as_array().unwrap().len(), 1);
        assert_eq!(out["data"]["terminal"]["status"], "completed");
        assert!(out["meta"]["took_ms"].is_u64());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 358c（2026-10-26）：inspect run 的父子视图——scout 子 run 的
    /// `tool_completed(agent_role=scout)` 携带 child_run_id/parent_run_id
    /// 与报告 stats，`run_timeline` 据此列 children。
    #[test]
    fn run_timeline_lists_scout_children() {
        let base = tempdir();
        seed_run(
            &base,
            "RUN-s1-2",
            &[
                event("run_started", json!({})),
                event(
                    "tool_started",
                    json!({
                        "tool": "spawn_scout",
                        "call_id": "c1",
                        "target": "scout",
                        "agent_role": "scout",
                        "child_run_id": "scout-s1-0001",
                        "parent_run_id": "RUN-s1-2"
                    }),
                ),
                event(
                    "tool_started",
                    json!({"tool": "read_file", "call_id": "c2"}),
                ),
                event(
                    "tool_completed",
                    json!({"tool": "read_file", "call_id": "c2", "exit_code": 0}),
                ),
                event(
                    "tool_completed",
                    json!({
                        "tool": "spawn_scout",
                        "call_id": "c1",
                        "target": "scout",
                        "agent_role": "scout",
                        "child_run_id": "scout-s1-0001",
                        "parent_run_id": "RUN-s1-2",
                        "report_id": "SC-0001",
                        "status": "ok",
                        "exit_code": 0,
                        "files_read": 3,
                        "tokens_used": 1200,
                        "wall_s": 7
                    }),
                ),
                event("run_finished", json!({"status": "completed"})),
            ],
        );
        let out = run_timeline(&base, "RUN-s1-2");
        let children = out["data"]["children"].as_array().expect("children array");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0]["child_run_id"], "scout-s1-0001");
        assert_eq!(children[0]["parent_run_id"], "RUN-s1-2");
        assert_eq!(children[0]["report_id"], "SC-0001");
        assert_eq!(children[0]["files_read"], 3);
        assert_eq!(children[0]["status"], "ok");
        let child_tools = children[0]["tools"].as_array().expect("child tools");
        assert_eq!(child_tools.len(), 1);
        assert_eq!(child_tools[0]["tool"], "read_file");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn session_overview_flags_unrecovered_calls_and_tombstone() {
        let base = tempdir();
        let gsa = base.join(".gsa");
        std::fs::create_dir_all(gsa.join("sessions")).unwrap();
        std::fs::write(gsa.join("sessions").join("s1.deleted"), "").unwrap();
        let sidecar = json!({
            "schema_version": "0.2.0-draft",
            "messages": [
                {"role": "user", "content": "q"},
                {"role": "assistant", "content": "", "tool_calls": [
                    {"name": "bash", "call_id": "cx"}
                ]},
            ],
        });
        let conv_dir = gsa.join("conversations");
        std::fs::create_dir_all(&conv_dir).unwrap();
        std::fs::write(
            conv_dir.join("s1.json"),
            serde_json::to_string_pretty(&sidecar).unwrap(),
        )
        .unwrap();

        let out = session_overview(&base, "s1");
        assert_eq!(out["data"]["exists"], true);
        assert_eq!(out["data"]["message_count"], 2);
        let dangling = out["data"]["unrecovered_tool_calls"].as_array().unwrap();
        assert_eq!(dangling.len(), 1);
        assert_eq!(dangling[0]["call_id"], "cx");
        assert_eq!(out["data"]["tombstoned"], true);
        assert_eq!(out["meta"]["encoding"], "utf-8");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn events_stream_paginates_with_next_cursor_and_kind_filter() {
        let base = tempdir();
        seed_run(
            &base,
            "RUN-s2-1",
            &[
                event("tool_started", json!({"call_id": "a"})),
                event("tool_heartbeat", json!({"call_id": "a", "elapsed_ms": 1})),
                event("tool_heartbeat", json!({"call_id": "a", "elapsed_ms": 2})),
                event("tool_completed", json!({"call_id": "a"})),
            ],
        );
        let page0 = events_stream(&base, "RUN-s2-1", Some(2), None, 0);
        assert_eq!(page0["data"]["events"].as_array().unwrap().len(), 2);
        assert_eq!(page0["meta"]["next_cursor"], "2");
        assert_eq!(page0["meta"]["truncated"], true);
        // 续页不重不漏。
        let page1 = events_stream(&base, "RUN-s2-1", Some(2), None, 2);
        assert_eq!(page1["data"]["events"].as_array().unwrap().len(), 2);
        assert_eq!(page1["meta"]["next_cursor"], Value::Null);
        // kind 过滤。
        let beats = events_stream(&base, "RUN-s2-1", None, Some("tool_heartbeat"), 0);
        assert_eq!(beats["data"]["events"].as_array().unwrap().len(), 2);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 355 批（2026-10-25，审查条目 1）交错过滤钉：events=[Y,X,Y,X]、
    /// kind=X、limit=1 → 两页各发一条、不重不漏、cursor 2/4。旧实现按
    /// 匹配数回写游标，两页会重发 X@1。
    #[test]
    fn events_stream_kind_filter_pages_do_not_overlap() {
        let base = tempdir();
        seed_run(
            &base,
            "RUN-s3-1",
            &[
                event("other_y", json!({"n": 1})),
                event("target_x", json!({"n": 1})),
                event("other_y", json!({"n": 2})),
                event("target_x", json!({"n": 2})),
            ],
        );
        let page0 = events_stream(&base, "RUN-s3-1", Some(1), Some("target_x"), 0);
        let page0_events = page0["data"]["events"].as_array().unwrap();
        assert_eq!(page0_events.len(), 1);
        assert_eq!(page0_events[0]["payload"]["n"], 1, "首页发 X@1");
        assert_eq!(page0["meta"]["next_cursor"], "2", "游标＝X@1 的下一索引");
        assert_eq!(page0["meta"]["truncated"], true);

        let page1 = events_stream(&base, "RUN-s3-1", Some(1), Some("target_x"), 2);
        let page1_events = page1["data"]["events"].as_array().unwrap();
        assert_eq!(page1_events.len(), 1);
        assert_eq!(page1_events[0]["payload"]["n"], 2, "续页发 X@3，不重发");
        assert_eq!(page1["meta"]["next_cursor"], Value::Null, "无更多匹配");
        assert_eq!(page1["meta"]["truncated"], false);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// 355 批（2026-10-25，审查条目 2）：run 序数不补零——RUN-s9-9 与
    /// RUN-s9-10 并存时最新 run 必须选 -10（数字序），face_fingerprint
    /// 从 -10 的 journal 回读。
    #[test]
    fn context_view_latest_run_follows_numeric_ordinal() {
        let base = tempdir();
        let gsa = base.join(".gsa");
        let conv_dir = gsa.join("conversations");
        std::fs::create_dir_all(&conv_dir).unwrap();
        std::fs::write(
            conv_dir.join("s9.json"),
            serde_json::to_string_pretty(&json!({
                "schema_version": "0.2.0-draft",
                "messages": [],
            }))
            .unwrap(),
        )
        .unwrap();
        let fingerprint_new = json!({"model": "test", "round": 10});
        seed_run(
            &base,
            "RUN-s9-9",
            &[event(
                "face_fingerprint",
                json!({"model": "test", "round": 9}),
            )],
        );
        seed_run(
            &base,
            "RUN-s9-10",
            &[event("face_fingerprint", fingerprint_new.clone())],
        );
        let out = context_view(&base, "s9");
        assert_eq!(out["ok"], true, "error: {}", out["error"]);
        assert_eq!(
            out["data"]["latest_run"], "RUN-s9-10",
            "数字序取最大：-10 > -9"
        );
        assert_eq!(out["data"]["face_fingerprint"], fingerprint_new);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn missing_journal_reports_structured_error() {
        let base = tempdir();
        let out = run_timeline(&base, "RUN-none-9");
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"]["code"], "journal_unreadable");
        let _ = std::fs::remove_dir_all(&base);
    }
}
