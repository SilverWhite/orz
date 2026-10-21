//! 0dm S2（2026-10-21）：交付状态的只读事实投影 —— `GET /api/delivery`。
//!
//! 设计依据 `docs/TERMINAL_DECLARATION_AND_SELF_CHECK_PULL_DESIGN_2026-10-21.md` §4.2：
//! `submit` 已从模型工具面退役，交付由 `[RUN_END]`（intent=conclude /
//! reason=completed）机械完成；`compute_delivery_status` 是 loop 运行时函数
//! （依赖进程内的计划批准基线 `delivery_baseline`），**空闲 UI 不可复算**。
//! 因此本模块只做事实投影，边界如实写进响应的 `notes`：
//!
//! - 会话侧车 `.gsa/conversations/{s8}.json`（`serde_json` 全量解析；形状照
//!   `orz-host/src/acp_server.rs` 的 `StoredConversation` 与黑板 live 快照：
//!   `session_id` / `messages[]` / `blackboard.plan` / `recall_log`）；
//! - `.gsa/runs/` 下本会话最近一个 run 的 journal 尾部（`run_finished` 的
//!   `status` / `terminal_source` / `terminal_nudged` /
//!   `terminal_nudge_count` / `self_check_called`，以及
//!   `tool_completed{tool:"self_check"}` 的完成次数）。
//!
//! 安全：`session8` 必须恰 8 位小写十六进制；所有路径都只是
//! `cwd/.gsa/...` 的 join，调用方无法注入路径片段（路径穿越一律拒绝）。
//! 只读面：不写任何 `.gsa` 文件、不触发模型（ADR-0010 §14.78 条 5：UI 零
//! 执行事实）。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// journal 尾部探针窗口（与 `runs.rs` 的终端探针同量级）：终态事件是最后
/// 一行，窗口只需覆盖末尾的事件突发。
const TAIL_PROBE_BYTES: u64 = 256 * 1024;
/// 侧车全量 JSON 上限。侧车逐字保存整段会话（可达数十 MB）；本投影只在按需
/// 查询时读一次，超限如实降级（`notes` 标注），不阻塞整个面板。
const SIDECAR_MAX_BYTES: u64 = 64 * 1024 * 1024;
/// 终态末步 id —— `orz-loop/src/planning.rs` 的 `LEGACY_TERMINAL_STEP_IDS`
/// 同源副本（`pub(crate)` 常量跨 crate 不可见）。0dm 后新计划只生成
/// `deliver`；旧计划/旧 journal 的 `submit` 末步仍按 terminal 解释（回放兼容）。
const TERMINAL_STEP_IDS: &[&str] = &["deliver", "submit"];

/// 0dm S2（2026-10-21）：交付状态只读投影。
///
/// 返回 JSON 契约（前端 `delivery.js` 消费；缺数据一律 `null`/空数组，并在
/// `notes` 写明原因，不虚构）：
///
/// ```json
/// { "session8": "…", "session_id": "…", "root": "…", "messages": 12,
///   "plan": { "steps": [{"id": "…", "state": "done"}],
///             "terminal_step": "deliver", "terminal_state": "done",
///             "delivery_status": null },
///   "last_run": { "run_id": "RUN-…", "status": "completed",
///                 "terminal_source": "run_end", "terminal_nudged": false,
///                 "terminal_nudge_count": 0, "self_check_called": true,
///                 "self_check_tool_rounds": 1 },
///   "recall": { "count": 0, "last_ordinal": null },
///   "notes": ["…只读投影边界说明…"] }
/// ```
///
/// `last_run` 在无 run journal 时为 `null`；`messages` 在侧车缺失/不可读时为
/// `null`。唯一的 `Err` 是会话标识不合法（HTTP 层映射 400）。
pub fn delivery_summary(cwd: &Path, session8: &str) -> Result<Value, String> {
    // 复合门：8 位小写十六进制是本面的语义口径；`session_id_ok` 再叠一层
    // 路径安全（无分隔符/点号/保留设备名）——两层都过才允许 join。
    if !session8_ok(session8) || !crate::security::session_id_ok(session8) {
        return Err("无效的会话标识（session8 须为 8 位小写十六进制）".to_string());
    }
    let mut notes: Vec<String> = vec![
        "只读投影：不重算交付变更清单（compute_delivery_status 依赖 loop 运行时的计划批准基线，空闲 UI 不可复算）；delivery_status 原样取自会话侧车已存的机械投影。".to_string(),
    ];

    // ① 会话侧车：全量解析（面板按需查询，不参与探索器的 5 秒轮询）。
    let sidecar = read_sidecar(cwd, session8, &mut notes);
    let session_id = sidecar
        .as_ref()
        .and_then(|s| s.get("session_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let messages = sidecar
        .as_ref()
        .and_then(|s| s.get("messages"))
        .and_then(Value::as_array)
        .map(|msgs| msgs.len() as u64);
    let plan = plan_projection(sidecar.as_ref().and_then(|s| s.get("blackboard")));
    let recall = recall_projection(
        sidecar.as_ref().and_then(|s| s.get("recall_log")),
        &mut notes,
    );

    // ② 最近一个本会话 run：找不到就是 null（不猜、不复用别的会话的 run）。
    let last_run = match latest_run_id(cwd, session8) {
        Some(run_id) => Some(run_facts(cwd, &run_id, &mut notes)),
        None => {
            notes.push(format!(
                "未找到本会话的 run journal（.gsa/runs/RUN-{session8}-*/RUN-CLI-{session8}）：最近 run 事实为 null。"
            ));
            None
        }
    };

    Ok(json!({
        "session8": session8,
        "session_id": session_id,
        "root": cwd.display().to_string(),
        "messages": messages,
        "plan": plan,
        "last_run": last_run,
        "recall": recall,
        "notes": notes,
    }))
}

fn runs_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("runs")
}

fn conversations_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("conversations")
}

/// 会话文件名的安全口径：恰 8 位小写十六进制（`.gsa` 侧车名形）。
fn session8_ok(session8: &str) -> bool {
    session8.len() == 8
        && session8
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// 读会话侧车（全量）。缺失/超限/不可读/坏 JSON 都只是降级：写入 `notes`
/// 并返回 `None`——交付面板照常展示 run 面事实，不因侧车损坏而整体失败。
fn read_sidecar(cwd: &Path, session8: &str, notes: &mut Vec<String>) -> Option<Value> {
    let path = conversations_dir(cwd).join(format!("{session8}.json"));
    let meta = match std::fs::metadata(&path) {
        Ok(meta) => meta,
        Err(_) => {
            notes.push(format!(
                "会话侧车不存在（.gsa/conversations/{session8}.json）：plan/消息数/撤回审计为空——会话可能尚未落盘或已被保留期清扫。"
            ));
            return None;
        }
    };
    if meta.len() > SIDECAR_MAX_BYTES {
        notes.push(format!(
            "会话侧车超出只读投影上限（{} MiB）：plan/消息数/撤回审计不可用。",
            SIDECAR_MAX_BYTES / (1024 * 1024)
        ));
        return None;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            notes.push(format!(
                "会话侧车无法读取（{e}）：plan/消息数/撤回审计不可用。"
            ));
            return None;
        }
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            notes.push(format!(
                "会话侧车无法解析（{e}）：plan/消息数/撤回审计不可用。"
            ));
            None
        }
    }
}

/// 黑板 → plan 投影。形状以 `orz-loop/src/blackboard.rs` 的实际 serde 为准：
/// `plan.steps[].status` 是字符串（`pending`/`in_progress`/`blocked`）或对象
/// （`{"status":"done","receipt_id":…}`/`{"status":"failed",…}`）；旧 epoch
/// 归档的 `Pending`/`InProgress`/`Completed`/`Blocked` 按同一状态机归并。
fn plan_projection(blackboard: Option<&Value>) -> Value {
    let plan = blackboard.and_then(|board| board.get("plan"));
    let steps: Vec<Value> = plan
        .and_then(|p| p.get("steps"))
        .and_then(Value::as_array)
        .map(|raw| {
            raw.iter()
                .filter_map(|step| {
                    // 无 id 的旧/坏步骤无法对应到 plan 身份，不虚造条目。
                    let id = step.get("id").and_then(Value::as_str)?;
                    if id.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "id": id,
                        "state": step_state(step.get("status")),
                    }))
                })
                .collect()
        })
        .unwrap_or_default();
    // 末步只在 id ∈ {deliver, submit} 时按终态步解释（0dm 收窄 + 旧计划兼容）。
    let (terminal_step, terminal_state) = match steps.last() {
        Some(last) => {
            let id = last.get("id").and_then(Value::as_str).unwrap_or_default();
            if TERMINAL_STEP_IDS.contains(&id) {
                (
                    Some(id.to_string()),
                    last.get("state")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                )
            } else {
                (None, None)
            }
        }
        None => (None, None),
    };
    json!({
        "steps": steps,
        "terminal_step": terminal_step,
        "terminal_state": terminal_state,
        // AGENT-DELIVERY-FLOW 的机械投影（最后一次 submit 渲染的面）；
        // submit 退役后多数会话为 null —— 面板如实显示为「未存档」。
        "delivery_status": plan
            .and_then(|p| p.get("delivery_status"))
            .and_then(Value::as_str),
    })
}

/// 单步状态归一：字符串/对象两种 serde 形态 + 旧单位变体。
fn step_state(status: Option<&Value>) -> Option<String> {
    let raw = match status? {
        Value::String(text) => text.as_str(),
        other => other.get("status")?.as_str()?,
    };
    // 旧 epoch 归档（阶段 A 前）的单位变体与现行字符串同义；未知取值原样
    // 投影（不吞、不改写——如实边界）。
    let normalized = match raw {
        "Completed" => "done",
        "Pending" => "pending",
        "InProgress" => "in_progress",
        "Blocked" => "blocked",
        other => other,
    };
    Some(normalized.to_string())
}

/// 撤回审计的紧凑投影（0di：`recall_log` 随会话卷持久化）。
fn recall_projection(log: Option<&Value>, notes: &mut Vec<String>) -> Value {
    let entries = log.and_then(Value::as_array);
    let count = entries.map(|items| items.len()).unwrap_or(0);
    let last_ordinal = entries
        .and_then(|items| items.last())
        .and_then(|record| record.get("ordinal"))
        .and_then(Value::as_u64);
    if count > 0 {
        // 撤回只截断对话消息面；机械面（plan/黑板）不回滚——面板不得让
        // 「撤回」读起来像「计划也撤回」。
        notes.push(format!(
            "会话侧车有 {count} 次撤回（recall_log）：消息面按撤回截断，plan/交付事实不随撤回回滚。"
        ));
    }
    json!({ "count": count, "last_ordinal": last_ordinal })
}

/// 选本会话「最近一个 run」：
/// - `RUN-<s8>-<n>`：按 **数值** 取最大 n（不能按字符串序：`-10` < `-2`）；
/// - 否则 `RUN-CLI-<s8>`（无头 `-p` 会话的无序号 run id；与会话自身 8 位同形）。
///
/// 只认带 `events.jsonl` 的目录（与 `runs.rs::list_runs` 同口径）；文件名
/// 仍过 `security::run_id_ok`，join 前不信任文件名。
fn latest_run_id(cwd: &Path, session8: &str) -> Option<String> {
    let entries = std::fs::read_dir(runs_dir(cwd)).ok()?;
    let numbered = format!("RUN-{session8}-");
    let cli = format!("RUN-CLI-{session8}");
    let mut best: Option<(u64, String)> = None;
    let mut cli_found = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !crate::security::run_id_ok(name) {
            continue;
        }
        if std::fs::metadata(path.join("events.jsonl")).is_err() {
            continue;
        }
        if let Some(ordinal) = name.strip_prefix(numbered.as_str()) {
            if let Ok(n) = ordinal.parse::<u64>()
                && best.as_ref().is_none_or(|(best_n, _)| n > *best_n)
            {
                best = Some((n, name.to_string()));
            }
        } else if name == cli.as_str() {
            cli_found = true;
        }
    }
    match best {
        Some((_, run_id)) => Some(run_id),
        None => cli_found.then_some(cli),
    }
}

/// 最近 run 的事实投影。journal 只扫尾部窗口（终态事件在末尾）；旧 journal
/// 缺字段时安全降级为 null，并给出 `notes`。
fn run_facts(cwd: &Path, run_id: &str, notes: &mut Vec<String>) -> Value {
    let journal = runs_dir(cwd).join(run_id).join("events.jsonl");
    let (lines, whole_file) = tail_lines(&journal);
    if !whole_file {
        notes.push(format!(
            "最近 run 的 journal 大于 {} KiB，仅扫描末尾窗口：self_check 完成次数可能低估（self_check_called 以 run_finished 字段为准）。",
            TAIL_PROBE_BYTES / 1024
        ));
    }
    let mut status: Option<String> = None;
    let mut terminal_source: Option<String> = None;
    let mut terminal_nudged: Option<bool> = None;
    let mut terminal_nudge_count: Option<u64> = None;
    let mut self_check_called: Option<bool> = None;
    let mut self_check_tool_rounds: u64 = 0;
    let mut finished = false;
    for line in &lines {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue; // 坏行不阻塞投影（journal 可能正在追加）
        };
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let payload = event.get("payload");
        match event_type {
            "run_finished" => {
                finished = true;
                status = Some(
                    payload
                        .and_then(|p| p.get("status"))
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .unwrap_or("completed")
                        .to_string(),
                );
                // 0dm（2026-10-21）：终态来源与自查事实（旧 journal 缺字段 ⇒ null）。
                terminal_source = payload
                    .and_then(|p| p.get("terminal_source"))
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string);
                terminal_nudged = payload
                    .and_then(|p| p.get("terminal_nudged"))
                    .and_then(Value::as_bool);
                terminal_nudge_count = payload
                    .and_then(|p| p.get("terminal_nudge_count"))
                    .and_then(Value::as_u64);
                self_check_called = payload
                    .and_then(|p| p.get("self_check_called"))
                    .and_then(Value::as_bool);
            }
            // 非正常收尾的终态事件（与 `runs.rs::terminal_status` 同一口径）。
            "run_failed" => status = Some("failed".to_string()),
            "run_cancelled" => status = Some("cancelled".to_string()),
            "run_invalidated" => status = Some("invalidated".to_string()),
            "run_terminated" => status = Some("terminated".to_string()),
            "tool_completed" => {
                let tool = payload.and_then(|p| p.get("tool")).and_then(Value::as_str);
                // 0dm：self_check_called 由**成功完成**的调用派生（被拒绝/
                // 被拦截的 tool_completed 不算）。
                if tool == Some("self_check") && tool_completed_ok(payload) {
                    self_check_tool_rounds += 1;
                }
            }
            _ => {}
        }
    }
    if finished {
        if self_check_called.is_none() {
            self_check_called = Some(self_check_tool_rounds > 0);
            notes.push(
                "该 run 的 run_finished 无 self_check_called 字段（0dm 前的旧 journal 形状）：按成功完成的 tool_completed{tool:\"self_check\"} 计数派生。"
                    .to_string(),
            );
        }
        if terminal_nudged.is_none()
            && let Some(count) = terminal_nudge_count
        {
            // 旧形状缺 terminal_nudged 但带计数时按计数派生（nudge 次数即事实）。
            terminal_nudged = Some(count > 0);
        }
        if terminal_source.is_none() {
            notes.push(
                "该 run 的 run_finished 无 terminal_source（0dm 前的旧 journal 形状）：终态来源为空。"
                    .to_string(),
            );
        }
    } else {
        notes.push(format!(
            "最近 run {run_id} 尚未收尾（尾部窗口内无终态事件）：status=running，终态字段为空。"
        ));
    }
    json!({
        "run_id": run_id,
        "status": status.as_deref().unwrap_or("running"),
        "terminal_source": terminal_source,
        "terminal_nudged": terminal_nudged,
        "terminal_nudge_count": terminal_nudge_count,
        "self_check_called": self_check_called,
        "self_check_tool_rounds": self_check_tool_rounds,
    })
}

/// `tool_completed` 是否为成功完成：显式错误/`status=error`/非 0 退出码都不是。
fn tool_completed_ok(payload: Option<&Value>) -> bool {
    let Some(payload) = payload else {
        return true;
    };
    if payload.get("error").is_some() {
        return false;
    }
    if payload.get("status").and_then(Value::as_str) == Some("error") {
        return false;
    }
    match payload.get("exit_code").and_then(Value::as_i64) {
        Some(code) => code == 0,
        None => true,
    }
}

/// 尾部探针的完整行（丢掉可能被截断的首行）＋「窗口是否覆盖整个文件」。
/// 返回 false 表示文件大于窗口，早期事件未被本投影看到（`notes` 如实标注）。
fn tail_lines(path: &Path) -> (Vec<String>, bool) {
    let Some(meta) = std::fs::metadata(path).ok() else {
        return (Vec::new(), true);
    };
    let size = meta.len();
    let start = size.saturating_sub(TAIL_PROBE_BYTES);
    let Some(data) = read_probe(path, start, TAIL_PROBE_BYTES) else {
        return (Vec::new(), true);
    };
    let text = String::from_utf8_lossy(&data);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // likely partial
    }
    (lines.into_iter().map(str::to_string).collect(), start == 0)
}

fn read_probe(path: &Path, start: u64, len: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = vec![0u8; len as usize];
    let mut read = 0usize;
    while read < buf.len() {
        match f.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
    buf.truncate(read);
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-web-delivery-test-{}-{}-{tag}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_line(path: &Path, v: serde_json::Value) {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(f, "{v}").unwrap();
    }

    /// 标准侧车：3 条消息、plan 两步（末步 `deliver` done、带存档的交付状态）、
    /// 黑板里没有 recall_log（旧侧车形状）。
    fn write_sidecar(root: &Path, session8: &str) {
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{session8}.json")),
            r#"{"schema_version":"0.2.0-draft","session_id":"0a1b2c3d-1111-2222-3333-444455556666","messages":[{"role":"user","content":"hi"},{"role":"assistant","content":"ok"},{"role":"user","content":"go"}],"blackboard":{"plan":{"plan_id":"PLAN-1","plan_epoch":1,"goal":"g","steps":[{"id":"s1","goal":"","actions":[],"acceptance":"","evidence":[],"status":"done"},{"id":"deliver","goal":"","actions":[],"acceptance":"","evidence":[],"status":{"status":"done","receipt_id":"RCP-1"}}],"delivery_status":"[delivery] 状态: 1 个变更 (src/a.rs M)","analysis":[],"decisions":[],"auth_grants":[],"model_notes":[]}}}"#,
        )
        .unwrap();
    }

    /// 0dm 正常投影：侧车 plan/消息数/会话 id ＋ 编号最大 run 的终态字段与
    /// self_check 完成次数。
    #[test]
    fn delivery_summary_projects_sidecar_and_last_run() {
        let root = temp_root("normal");
        let s8 = "0a1b2c3d";
        write_sidecar(&root, s8);
        let run = root.join(format!(".gsa/runs/RUN-{s8}-3"));
        std::fs::create_dir_all(&run).unwrap();
        let journal = run.join("events.jsonl");
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_started","timestamp":"t","payload":{}}),
        );
        write_line(
            &journal,
            serde_json::json!({"event_type":"tool_completed","payload":{"tool":"self_check","call_id":"c1","exit_code":0}}),
        );
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_finished","payload":{"status":"completed","terminal_source":"run_end","terminal_nudged":false,"terminal_nudge_count":0,"self_check_called":true}}),
        );
        // 同会话的另一个（更早的）run 目录必须被忽略。
        let older = root.join(format!(".gsa/runs/RUN-{s8}-2"));
        std::fs::create_dir_all(&older).unwrap();
        write_line(
            &older.join("events.jsonl"),
            serde_json::json!({"event_type":"run_cancelled","payload":{}}),
        );

        let v = delivery_summary(&root, s8).unwrap();
        assert_eq!(v["session8"], s8);
        assert_eq!(v["session_id"], "0a1b2c3d-1111-2222-3333-444455556666");
        assert_eq!(v["root"], root.display().to_string());
        assert_eq!(v["messages"], 3);
        assert_eq!(v["plan"]["steps"][0]["id"], "s1");
        assert_eq!(v["plan"]["steps"][0]["state"], "done");
        assert_eq!(v["plan"]["terminal_step"], "deliver");
        assert_eq!(v["plan"]["terminal_state"], "done");
        assert_eq!(
            v["plan"]["delivery_status"],
            "[delivery] 状态: 1 个变更 (src/a.rs M)"
        );
        assert_eq!(v["last_run"]["run_id"], format!("RUN-{s8}-3"));
        assert_eq!(v["last_run"]["status"], "completed");
        assert_eq!(v["last_run"]["terminal_source"], "run_end");
        assert_eq!(v["last_run"]["terminal_nudged"], false);
        assert_eq!(v["last_run"]["terminal_nudge_count"], 0);
        assert_eq!(v["last_run"]["self_check_called"], true);
        assert_eq!(v["last_run"]["self_check_tool_rounds"], 1);
        assert_eq!(v["recall"]["count"], 0);
        assert!(
            v["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n.as_str().unwrap_or("").contains("只读投影")),
            "notes 必须如实标注只读投影边界"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 安全门：session8 只接受恰 8 位小写十六进制；穿越/大写/长度错一律 Err。
    #[test]
    fn delivery_summary_rejects_bad_session8() {
        let root = temp_root("badid");
        for bad in [
            "",
            "../etc",
            "..\\etc",
            "a/b",
            "0A1B2C3D",
            "0a1b2c3",
            "0a1b2c3d0",
            "0a1b2c3g",
            "中文中文中文",
        ] {
            assert!(
                delivery_summary(&root, bad).is_err(),
                "session8 {bad:?} 必须被拒绝"
            );
        }
        // 合法 id 但无数据：不是错误，是「空投影」。
        assert!(delivery_summary(&root, "0a1b2c3d").is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 无 run：`last_run` 如实 null，并在 notes 说明（侧车事实仍可展示）。
    #[test]
    fn delivery_summary_without_run_reports_null() {
        let root = temp_root("no-run");
        let s8 = "0a1b2c3d";
        write_sidecar(&root, s8);
        let v = delivery_summary(&root, s8).unwrap();
        assert!(v["last_run"].is_null());
        assert_eq!(v["messages"], 3);
        assert_eq!(v["plan"]["terminal_step"], "deliver");
        assert!(
            v["notes"].as_array().unwrap().iter().any(|n| n
                .as_str()
                .unwrap_or("")
                .contains("未找到本会话的 run journal")),
            "notes 必须说明 run 缺失"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 旧 journal 降级：`run_finished` 只有 status 时不 panic、终态字段 null；
    /// `self_check_called` 按成功完成的 tool_completed 派生（被拒绝的不算）。
    #[test]
    fn delivery_summary_degrades_when_run_finished_fields_missing() {
        let root = temp_root("degrade");
        let s8 = "0a1b2c3d";
        write_sidecar(&root, s8);
        let run = root.join(format!(".gsa/runs/RUN-{s8}-1"));
        std::fs::create_dir_all(&run).unwrap();
        let journal = run.join("events.jsonl");
        write_line(
            &journal,
            serde_json::json!({"event_type":"tool_completed","payload":{"tool":"self_check","call_id":"c1","exit_code":0}}),
        );
        // 被拒绝的 self_check（0dm 拒绝面：exit_code 1 + status error）不计入。
        write_line(
            &journal,
            serde_json::json!({"event_type":"tool_completed","payload":{"tool":"self_check","call_id":"c2","exit_code":1,"status":"error","error":"self_check_disabled"}}),
        );
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_finished","timestamp":"t","payload":{"status":"completed"}}),
        );

        let v = delivery_summary(&root, s8).unwrap();
        assert_eq!(v["last_run"]["status"], "completed");
        assert!(v["last_run"]["terminal_source"].is_null());
        assert!(v["last_run"]["terminal_nudged"].is_null());
        assert!(v["last_run"]["terminal_nudge_count"].is_null());
        assert_eq!(v["last_run"]["self_check_called"], true, "按完成次数派生");
        assert_eq!(v["last_run"]["self_check_tool_rounds"], 1);
        let notes: Vec<String> = v["notes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect();
        assert!(
            notes.iter().any(|n| n.contains("self_check_called")),
            "缺字段降级必须写进 notes"
        );
        assert!(
            notes.iter().any(|n| n.contains("terminal_source")),
            "缺 terminal_source 必须写进 notes"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// run 选择按序号数值最大（`-10` 必须胜过 `-2`，字符串序会选错），
    /// 且旧计划的 `submit` 末步仍按 terminal 解释（0dm 回放兼容）。
    #[test]
    fn delivery_summary_picks_numeric_latest_run_and_accepts_legacy_submit_step() {
        let root = temp_root("latest");
        let s8 = "0a1b2c3d";
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{s8}.json")),
            r#"{"session_id":"0a1b2c3d-1111-2222-3333-444455556666","messages":[],"blackboard":{"plan":{"steps":[{"id":"s1","status":"done"},{"id":"submit","status":{"status":"done","receipt_id":"RCP-9"}}]}}}"#,
        )
        .unwrap();
        for (ordinal, source) in [(2u64, "text_without_terminal"), (10u64, "run_end")] {
            let run = root.join(format!(".gsa/runs/RUN-{s8}-{ordinal}"));
            std::fs::create_dir_all(&run).unwrap();
            write_line(
                &run.join("events.jsonl"),
                serde_json::json!({"event_type":"run_finished","payload":{"status":"completed","terminal_source":source}}),
            );
        }
        let v = delivery_summary(&root, s8).unwrap();
        assert_eq!(v["last_run"]["run_id"], format!("RUN-{s8}-10"));
        assert_eq!(v["last_run"]["terminal_source"], "run_end");
        assert_eq!(v["plan"]["terminal_step"], "submit");
        assert_eq!(v["plan"]["terminal_state"], "done");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 无头 `-p` 会话：没有编号 run 时回落到 `RUN-CLI-<s8>`。
    #[test]
    fn delivery_summary_falls_back_to_run_cli() {
        let root = temp_root("cli-run");
        let s8 = "0a1b2c3d";
        write_sidecar(&root, s8);
        let run = root.join(format!(".gsa/runs/RUN-CLI-{s8}"));
        std::fs::create_dir_all(&run).unwrap();
        write_line(
            &run.join("events.jsonl"),
            serde_json::json!({"event_type":"run_finished","payload":{"status":"completed","terminal_source":"short_text","terminal_nudged":false,"terminal_nudge_count":0,"self_check_called":false}}),
        );
        let v = delivery_summary(&root, s8).unwrap();
        assert_eq!(v["last_run"]["run_id"], format!("RUN-CLI-{s8}"));
        assert_eq!(v["last_run"]["terminal_source"], "short_text");
        assert_eq!(v["last_run"]["self_check_called"], false);
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 撤回审计只截断消息面：`recall_log` 计入 recall 投影并在 notes 说明
    /// plan/交付事实不回滚。
    #[test]
    fn delivery_summary_projects_recall_log_without_rolling_back_plan() {
        let root = temp_root("recall");
        let s8 = "0a1b2c3d";
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{s8}.json")),
            r#"{"session_id":"0a1b2c3d-1111-2222-3333-444455556666","messages":[{"role":"user","content":"x"}],"blackboard":{"plan":{"steps":[{"id":"deliver","status":"done"}]}},"recall_log":[{"recalled_at":1760000000.5,"ordinal":2,"removed_messages":4,"remaining_messages":1,"dropped_supplements":0}]}"#,
        )
        .unwrap();
        let v = delivery_summary(&root, s8).unwrap();
        assert_eq!(v["recall"]["count"], 1);
        assert_eq!(v["recall"]["last_ordinal"], 2);
        assert_eq!(v["messages"], 1);
        assert_eq!(v["plan"]["terminal_state"], "done");
        assert!(
            v["notes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n.as_str().unwrap_or("").contains("不随撤回回滚")),
            "撤回边界必须写进 notes"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
