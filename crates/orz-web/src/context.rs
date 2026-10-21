//! 333（2026-10-21）：上下文占用与最近压缩的只读投影 —— `GET /api/context`。
//!
//! 契约来源：[`S9 上下文与压缩`](../../../architecture/current/surfaces/09-context-compaction.md)
//! 0.9.8 必闭——「Web 端至少显示上下文占用/最近一次压缩的可读读数（不要求
//! 滑块）」。本模块只读两侧事实：
//!
//! - 会话侧车 `.gsa/conversations/{s8}.json` 的 `messages`：按 `chars/2`
//!   估算当前消息面 token（与 orz-loop `estimate_messages_tokens` 同口径的
//!   近似值，明确标注 `estimate_method`，不冒充模型计数器）；
//! - 本会话最近 run journal 尾部的 `context_compressed`/`context_compacted`
//!   事件：`trigger_tokens`/`estimated_tokens_after`/`mode`/`reason`；
//! - 压缩存档计数（`.gsa/compaction/compaction-<run>-*.md` 与
//!   `.gsa/compaction/blocks/<s8>-*`），只数文件，不读正文。
//!
//! 安全/只读：`session8` 必须恰 8 位小写十六进制；所有路径只是
//! `cwd/.gsa/...` 的 join（复用 [`crate::delivery`] 的同一口径）；不写文件、
//! 不触发模型。

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// 扫描最近多少个 run（压缩可能发生在更早的 run；从新到旧找第一条）。
const MAX_RUNS_SCANNED: usize = 8;
/// 单个 journal 尾部探针（与 delivery 同量级）。
const TAIL_PROBE_BYTES: u64 = 256 * 1024;

/// 333：上下文读数投影。唯一的 `Err` 是会话标识不合法（HTTP 层映射 400）。
pub fn context_summary(cwd: &Path, session8: &str) -> Result<Value, String> {
    if !session8_ok(session8) || !crate::security::session_id_ok(session8) {
        return Err("无效的会话标识（session8 须为 8 位小写十六进制）".to_string());
    }
    let mut notes: Vec<String> = vec![
        "上下文占用是消息面 chars/2 的近似估算（与 orz-loop 估算口径同源，不是模型 tokenizer 计数）；压缩读数为最近 run journal 的机械事件事实。"
            .to_string(),
    ];

    let sidecar = crate::delivery::read_sidecar(cwd, session8, &mut notes);
    let messages = sidecar
        .as_ref()
        .and_then(|s| s.get("messages"))
        .and_then(Value::as_array);
    let message_count = messages.map(|m| m.len() as u64);
    let chars: u64 = messages
        .map(|m| {
            m.iter()
                .map(|msg| {
                    msg.get("content")
                        .and_then(Value::as_str)
                        .map(|c| c.chars().count() as u64)
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0);
    let estimated_tokens = chars / 2;
    let watermarks = sidecar
        .as_ref()
        .and_then(|s| s.get("context_scale_notified"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let (last_compaction, runs_scanned, journal_truncated) =
        latest_compaction(cwd, session8, &mut notes);
    if last_compaction.is_none() {
        notes.push(
            "未在最近 run 的 journal 尾部找到 context_compressed/context_compacted 事件：最近压缩读数为 null（可能从未压缩，或事件早于扫描窗口）。"
                .to_string(),
        );
    }
    let archives = compaction_archive_count(cwd, session8);
    if archives == 0 {
        notes.push(
            "未找到本会话的压缩存档（.gsa/compaction/*.md / blocks/*）：压缩计数为 0。".to_string(),
        );
    }

    Ok(json!({
        "session8": session8,
        "session_id": sidecar
            .as_ref()
            .and_then(|s| s.get("session_id"))
            .and_then(Value::as_str),
        "root": cwd.display().to_string(),
        "messages": message_count,
        "chars": chars,
        "estimated_tokens": estimated_tokens,
        "estimate_method": "chars/2",
        "watermarks": watermarks,
        "last_compaction": last_compaction,
        "compaction_archives": archives,
        "runs_scanned": runs_scanned,
        "journal_truncated": journal_truncated,
        "notes": notes,
    }))
}

fn session8_ok(session8: &str) -> bool {
    session8.len() == 8
        && session8
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn runs_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("runs")
}

/// 本会话的 run id，按序号从新到旧（`RUN-CLI-<s8>` 无序号，置后）。
fn run_ids_desc(cwd: &Path, session8: &str) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(runs_dir(cwd)) else {
        return Vec::new();
    };
    let numbered = format!("RUN-{session8}-");
    let cli = format!("RUN-CLI-{session8}");
    let mut numbered_ids: Vec<(u64, String)> = Vec::new();
    let mut cli_found = false;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || std::fs::metadata(path.join("events.jsonl")).is_err() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !crate::security::run_id_ok(name) {
            continue;
        }
        if let Some(ordinal) = name.strip_prefix(numbered.as_str()) {
            if let Ok(n) = ordinal.parse::<u64>() {
                numbered_ids.push((n, name.to_string()));
            }
        } else if name == cli {
            cli_found = true;
        }
    }
    numbered_ids.sort_by(|a, b| b.0.cmp(&a.0));
    let mut ids: Vec<String> = numbered_ids.into_iter().map(|(_, id)| id).collect();
    if cli_found {
        ids.push(cli);
    }
    ids
}

/// 最近一次压缩事件（从新 run 到旧 run；同一 run 取尾部最后一条）。
fn latest_compaction(
    cwd: &Path,
    session8: &str,
    notes: &mut Vec<String>,
) -> (Option<Value>, u64, bool) {
    let ids = run_ids_desc(cwd, session8);
    let scanned = ids.len().min(MAX_RUNS_SCANNED) as u64;
    let mut truncated = false;
    for run_id in ids.iter().take(MAX_RUNS_SCANNED) {
        let journal = runs_dir(cwd).join(run_id).join("events.jsonl");
        let (lines, whole) = tail_lines(&journal);
        if !whole {
            truncated = true;
        }
        let mut found: Option<Value> = None;
        for line in &lines {
            let Ok(event) = serde_json::from_str::<Value>(line) else {
                continue; // journal 可能正在追加——坏行不阻塞读数
            };
            let event_type = event
                .get("event_type")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if event_type != "context_compressed" && event_type != "context_compacted" {
                continue;
            }
            let payload = event.get("payload");
            let get_u64 = |key: &str| payload.and_then(|p| p.get(key)).and_then(Value::as_u64);
            found = Some(json!({
                "run_id": run_id,
                "event": event_type,
                "trigger_tokens": get_u64("trigger_tokens"),
                "estimated_tokens_after": get_u64("estimated_tokens_after")
                    .or_else(|| get_u64("target_tokens")),
                "mode": payload.and_then(|p| p.get("mode")).and_then(Value::as_str),
                "reason": payload.and_then(|p| p.get("reason")).and_then(Value::as_str),
                "archive_write_failed": payload
                    .and_then(|p| p.get("archive_write_failed"))
                    .and_then(Value::as_bool),
            }));
        }
        if let Some(found) = found {
            return (Some(found), scanned, truncated);
        }
    }
    if ids.len() > MAX_RUNS_SCANNED {
        notes.push(format!(
            "只扫描最近 {MAX_RUNS_SCANNED} 个 run：更早的压缩事件可能未被本读数覆盖。"
        ));
    }
    (None, scanned, truncated)
}

/// 压缩存档计数（只数件数，不读正文）：`compaction-<run>-*.md` 的 run 属于
/// 本会话，或 `blocks/<s8>-*.md`。
fn compaction_archive_count(cwd: &Path, session8: &str) -> u64 {
    let compaction = cwd.join(".gsa").join("compaction");
    let numbered = format!("RUN-{session8}-");
    let cli = format!("RUN-CLI-{session8}");
    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(&compaction) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(rest) = name.strip_prefix("compaction-") else {
                continue;
            };
            let Some(rest) = rest.strip_suffix(".md") else {
                continue;
            };
            let Some((run_id, _seq)) = rest.rsplit_once('-') else {
                continue;
            };
            if run_id.starts_with(&numbered) || run_id == cli {
                count += 1;
            }
        }
    }
    let blocks = compaction.join("blocks");
    if let Ok(entries) = std::fs::read_dir(&blocks) {
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_string) else {
                continue;
            };
            if (name.starts_with(&format!("{session8}-block-"))
                || name == format!("{session8}-clear.md"))
                && entry.path().is_file()
            {
                count += 1;
            }
        }
    }
    count
}

/// 尾部窗口的完整行（丢掉可能被截断的首行）＋窗口是否覆盖整个文件。
fn tail_lines(path: &Path) -> (Vec<String>, bool) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(meta) = std::fs::metadata(path) else {
        return (Vec::new(), true);
    };
    let size = meta.len();
    let start = size.saturating_sub(TAIL_PROBE_BYTES);
    let Ok(mut f) = std::fs::File::open(path) else {
        return (Vec::new(), true);
    };
    if f.seek(SeekFrom::Start(start)).is_err() {
        return (Vec::new(), true);
    }
    let mut buf = vec![0u8; TAIL_PROBE_BYTES as usize];
    let mut read = 0usize;
    while read < buf.len() {
        match f.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
    buf.truncate(read);
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // 可能是被截断的半行
    }
    (lines.into_iter().map(str::to_string).collect(), start == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-context-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn context_summary_reports_estimate_and_last_compaction() {
        let cwd = temp_dir("readout");
        let s8 = "cafe1234";
        std::fs::create_dir_all(cwd.join(".gsa").join("conversations")).unwrap();
        std::fs::write(
            cwd.join(".gsa")
                .join("conversations")
                .join(format!("{s8}.json")),
            serde_json::to_vec(&json!({
                "session_id": "cafe1234-aaaa-bbbb-cccc-ddddeeeeffff",
                "messages": [
                    {"role": "user", "content": "12345678"},
                    {"role": "assistant", "content": "1234"}
                ],
                "context_scale_notified": ["500K", "900K"]
            }))
            .unwrap(),
        )
        .unwrap();
        let run = cwd.join(".gsa").join("runs").join(format!("RUN-{s8}-3"));
        std::fs::create_dir_all(&run).unwrap();
        let events = [
            json!({"event_type": "context_compressed", "payload": {
                "trigger_tokens": 900000, "estimated_tokens_after": 420000,
                "mode": "mechanical", "reason": "context_scale_window"
            }}),
            json!({"event_type": "run_finished", "payload": {"status": "completed"}}),
        ];
        std::fs::write(
            run.join("events.jsonl"),
            events
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        std::fs::create_dir_all(cwd.join(".gsa").join("compaction").join("blocks")).unwrap();
        std::fs::write(
            cwd.join(".gsa")
                .join("compaction")
                .join("blocks")
                .join(format!("{s8}-block-0001.md")),
            "block",
        )
        .unwrap();

        let summary = context_summary(&cwd, s8).expect("summary");
        assert_eq!(summary["messages"], 2);
        assert_eq!(summary["chars"], 12);
        assert_eq!(summary["estimated_tokens"], 6);
        assert_eq!(summary["estimate_method"], "chars/2");
        assert_eq!(summary["watermarks"].as_array().unwrap().len(), 2);
        assert_eq!(summary["last_compaction"]["trigger_tokens"], 900000);
        assert_eq!(summary["last_compaction"]["estimated_tokens_after"], 420000);
        assert_eq!(summary["last_compaction"]["mode"], "mechanical");
        assert_eq!(summary["compaction_archives"], 1);
        assert_eq!(summary["runs_scanned"], 1);
        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn context_summary_empty_session_has_null_readout() {
        let cwd = temp_dir("empty");
        let summary = context_summary(&cwd, "deadbeef").expect("summary");
        assert!(summary["messages"].is_null());
        assert_eq!(summary["estimated_tokens"], 0);
        assert!(summary["last_compaction"].is_null());
        assert_eq!(summary["compaction_archives"], 0);
        assert!(!summary["notes"].as_array().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&cwd);
    }

    #[test]
    fn context_summary_rejects_bad_session_id() {
        let cwd = temp_dir("bad");
        assert!(context_summary(&cwd, "../evil").is_err());
        assert!(context_summary(&cwd, "CAFE1234").is_err());
        let _ = std::fs::remove_dir_all(&cwd);
    }
}
