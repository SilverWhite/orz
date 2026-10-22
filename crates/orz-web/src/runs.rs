//! Read-only `.gsa` projection scans (0br S2, survey §5.2): run listing,
//! journal byte-range slices and conversation listing. No arbitrary-path
//! reads — every path is joined onto the workspace `.gsa` root with a
//! validated run id (`security::run_id_ok`).

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// How much of a journal head we read for the started timestamp.
const HEAD_PROBE_BYTES: u64 = 64 * 1024;
/// How much of a journal tail we scan for a terminal event. The terminal
/// event is the last line of a finished run — but only just: keep the
/// window comfortably above the largest plausible trailing event burst so
/// finished runs are not misreported as `running`.
const TERMINAL_PROBE_BYTES: u64 = 256 * 1024;
/// Journals can be megabytes; metadata stays cheap. Beyond the most recent
/// `PROBE_CAP` runs the listing skips content probes entirely (`status:
/// "archived"`, no `started`) — the Explorer shows recent history, not the
/// whole ledger.
const PROBE_CAP: usize = 50;
/// Per-response cap for the events slice endpoint.
pub const EVENTS_SLICE_MAX_BYTES: u64 = 512 * 1024;
/// Listing caps (long sessions accumulate runs; the Explorer shows recent
/// history, not the whole ledger).
const LIST_CAP: usize = 200;

#[derive(Debug, Serialize)]
pub struct RunSummary {
    pub run_id: String,
    pub size_bytes: u64,
    pub modified_ms: u128,
    pub started: Option<String>,
    /// `completed` / `failed` / `cancelled` / `invalidated` / `terminated`
    /// from the latest terminal journal event; `running` when none;
    /// `archived` beyond the content-probe cap (status not determined).
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ConversationSummary {
    pub session8: String,
    /// Full ACP session id recovered from the bounded sidecar head.  The
    /// explorer displays `session8`, but continuation must bind the exact id.
    pub session_id: Option<String>,
    /// 2026-10-21：该会话的归属工作区根（列清单时的 cwd）。切换工作区后
    /// 仍打开着旧会话时，`session/load` 必须用这个根而不是新 cwd——否则
    /// 侧车在新根的 `.gsa/conversations/` 下找不到，界面报「恢复失败」。
    pub root: String,
    pub size_bytes: u64,
    pub modified_ms: u128,
}

fn runs_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("runs")
}

fn conversations_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("conversations")
}

/// Journal file for a validated run id; `None` when the id fails the path
/// safety gate.
pub fn journal_path(cwd: &Path, run_id: &str) -> Option<PathBuf> {
    if !crate::security::run_id_ok(run_id) {
        return None;
    }
    Some(runs_dir(cwd).join(run_id).join("events.jsonl"))
}

fn modified_ms(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Read the UUID-like `session_id` from the bounded JSON head without loading
/// a multi-megabyte conversation every five-second explorer refresh.
///
/// 318 P1-A6: the 64 KiB probe can end in the middle of a multi-byte UTF-8
/// character (long Chinese conversations). Strict `from_utf8` then failed the
/// whole head even when `session_id` was fully inside it, so a valid sidecar
/// randomly lost continuation. Lossy decoding keeps every ASCII byte before
/// the cut intact; only the truncated tail character becomes U+FFFD.
fn conversation_session_id(path: &Path) -> Option<String> {
    let data = read_probe(path, 0, HEAD_PROBE_BYTES)?;
    let text = String::from_utf8_lossy(&data);
    let key = "\"session_id\"";
    let tail = text.get(text.find(key)? + key.len()..)?;
    let tail = tail.get(tail.find(':')? + 1..)?.trim_start();
    let quoted = tail.strip_prefix('"')?;
    let end = quoted.find('"')?;
    let value = quoted.get(..end)?;
    (!value.is_empty()).then(|| value.to_string())
}

/// Pull the first complete line from the head of the file (≤ probe bytes).
fn first_line(path: &Path) -> Option<String> {
    let data = read_probe(path, 0, HEAD_PROBE_BYTES)?;
    let text = String::from_utf8_lossy(&data);
    text.lines().next().map(str::to_string)
}

/// Pull complete lines from the tail probe (last ≤ probe bytes), dropping
/// a possibly-partial first line.
fn tail_lines(path: &Path) -> Vec<String> {
    let Some(meta) = std::fs::metadata(path).ok() else {
        return Vec::new();
    };
    let size = meta.len();
    let start = size.saturating_sub(TERMINAL_PROBE_BYTES);
    let Some(data) = read_probe(path, start, TERMINAL_PROBE_BYTES) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&data);
    let mut lines: Vec<&str> = text.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // likely partial
    }
    lines.into_iter().map(str::to_string).collect()
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

fn terminal_status(event_type: &str, payload: &Value) -> Option<String> {
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match event_type {
        "run_finished" => Some(if status.is_empty() {
            "completed".into()
        } else {
            status.to_string()
        }),
        "run_failed" => Some("failed".into()),
        "run_cancelled" => Some("cancelled".into()),
        "run_invalidated" => Some("invalidated".into()),
        "run_terminated" => Some("terminated".into()),
        _ => None,
    }
}

/// 2026-10-22（338 批）：run 排序——**先按会话分组，组内按序号降序**，组间按
/// 组内最新 mtime 降序。
///
/// `list_runs` 原本只按 journal mtime 排序；但「回档」走
/// `restore_session_from_archive`（`std::fs::write` 逐件重写，归档包只存正文
/// 不存时间戳），会把**同一会话全部 run 的 mtime 压到同一刻**。真机实证
/// c23e0aaa：24 个 journal 的 mtime 落在 20:19:07.700–.744（44 ms 内、16 个
/// 不同值），排序退化成目录名字典序 `0,1,10,11,…,23,3,4,5,6,7,8,9` ⇒ 会话
/// 回放末尾落在 run 8/9 的旧内容上，标记栏按回放顺序乱序，「恢复并继续」
/// 取到的实时 run 也是 run 9。同会话内 mtime 既然如此不可信，就不能再让它
/// 当组内主键：组内一律按 `RUN-/ARC-/RST-<s8>-<n>` 的 n 降序。
///
/// 纯函数，便于单元测试。
fn split_run_id(run_id: &str) -> (String, Option<u64>) {
    match run_id.rsplit_once('-') {
        Some((prefix, tail)) => match tail.parse::<u64>() {
            Ok(seq) => (prefix.to_string(), Some(seq)),
            Err(_) => (run_id.to_string(), None),
        },
        None => (run_id.to_string(), None),
    }
}

/// 排序（新在前）：组间按组内最大 mtime 降序；组内按序号降序；再以完整 id
/// 降序保证确定性（readdir 顺序不是排序依据）。
pub(crate) fn order_run_entries(entries: Vec<(String, u128)>) -> Vec<String> {
    let mut groups: Vec<(String, u128, Vec<(Option<u64>, String)>)> = Vec::new();
    let mut by_key: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (run_id, modified_ms) in entries {
        let (key, seq) = split_run_id(&run_id);
        let slot = match by_key.get(&key) {
            Some(&i) => i,
            None => {
                groups.push((key.clone(), modified_ms, Vec::new()));
                by_key.insert(key.clone(), groups.len() - 1);
                groups.len() - 1
            }
        };
        let group = &mut groups[slot];
        group.1 = group.1.max(modified_ms);
        group.2.push((seq, run_id));
    }
    groups.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    let mut out: Vec<String> = Vec::new();
    for (_, _, mut members) in groups {
        members.sort_by(|(seq_a, id_a), (seq_b, id_b)| {
            seq_b.cmp(seq_a).then_with(|| id_b.cmp(id_a))
        });
        out.extend(members.into_iter().map(|(_, run_id)| run_id));
    }
    out
}

/// List runs under `{cwd}/.gsa/runs/`, newest first, capped. Content
/// probes (started timestamp + terminal status) run only for the most
/// recent [`PROBE_CAP`] entries — older runs read as `archived` without
/// touching their journals.
pub fn list_runs(cwd: &Path) -> Vec<RunSummary> {
    // Phase 1: cheap directory metadata only.
    let mut entries: Vec<(String, PathBuf, u128)> = Vec::new();
    let Ok(dir_entries) = std::fs::read_dir(runs_dir(cwd)) else {
        return Vec::new();
    };
    for entry in dir_entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(run_id) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !crate::security::run_id_ok(run_id) {
            continue;
        }
        let journal = path.join("events.jsonl");
        let Ok(meta) = std::fs::metadata(&journal) else {
            continue;
        };
        entries.push((run_id.to_string(), journal, modified_ms(&meta)));
    }
    // Newest first（338：mtime 降序；平局按会话内序号降序），capped before
    // any content probe.
    let order = order_run_entries(
        entries
            .iter()
            .map(|(run_id, _, ms)| (run_id.clone(), *ms))
            .collect(),
    );
    let index: std::collections::HashMap<&str, &PathBuf> = entries
        .iter()
        .map(|(run_id, journal, _)| (run_id.as_str(), journal))
        .collect();
    let entries: Vec<(String, PathBuf)> = order
        .into_iter()
        .take(LIST_CAP)
        .filter_map(|run_id| {
            let journal = index.get(run_id.as_str())?;
            Some((run_id, (*journal).clone()))
        })
        .collect();

    let mut out: Vec<RunSummary> = Vec::with_capacity(entries.len());
    for (i, (run_id, journal)) in entries.iter().enumerate() {
        let meta = std::fs::metadata(journal).ok();
        let (started, status) = if i < PROBE_CAP {
            let started = first_line(journal).and_then(|line| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .filter(|v| v.get("event_type").and_then(Value::as_str) == Some("run_started"))
                    .and_then(|v| {
                        v.get("timestamp")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    })
            });
            let mut status = "running".to_string();
            for line in tail_lines(journal) {
                if let Ok(v) = serde_json::from_str::<Value>(&line) {
                    let et = v.get("event_type").and_then(Value::as_str).unwrap_or("");
                    if let Some(s) = terminal_status(et, &v) {
                        status = s;
                    }
                }
            }
            (started, status)
        } else {
            (None, "archived".to_string())
        };
        out.push(RunSummary {
            run_id: run_id.clone(),
            size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified_ms: meta.as_ref().map(modified_ms).unwrap_or(0),
            started,
            status,
        });
    }
    out
}

/// Read a byte-range of complete JSONL lines. Returns the parsed values
/// plus the byte offset to resume from (a trailing partial line is
/// withheld — append-only journals make byte offsets stable).
pub fn slice_events(
    cwd: &Path,
    run_id: &str,
    from: u64,
    max_bytes: u64,
) -> Option<(Vec<Value>, u64)> {
    use std::io::{Read, Seek, SeekFrom};
    let path = journal_path(cwd, run_id)?;
    let mut f = std::fs::File::open(&path).ok()?;
    let size = f.metadata().ok()?.len();
    let from = from.min(size);
    f.seek(SeekFrom::Start(from)).ok()?;
    let cap = max_bytes.min(EVENTS_SLICE_MAX_BYTES) as usize;
    let mut buf = vec![0u8; cap];
    let mut read = 0usize;
    while read < buf.len() {
        match f.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(_) => break,
        }
    }
    buf.truncate(read);
    let mut lines = Vec::new();
    let mut consumed = 0usize;
    let mut start = 0usize;
    for (i, b) in buf.iter().enumerate() {
        if *b == b'\n' {
            consumed = i + 1;
            let line = &buf[start..i];
            start = i + 1;
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_slice::<Value>(line) {
                lines.push(v);
            }
        }
    }
    let next = from + consumed as u64;
    Some((lines, if consumed == 0 { from } else { next }))
}

/// List conversations under `{cwd}/.gsa/conversations/`, newest first.
pub fn list_conversations(cwd: &Path) -> Vec<ConversationSummary> {
    let mut out: Vec<ConversationSummary> = Vec::new();
    let Ok(entries) = std::fs::read_dir(conversations_dir(cwd)) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(session8) = name.strip_suffix(".json") else {
            continue;
        };
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        out.push(ConversationSummary {
            session8: session8.to_string(),
            session_id: conversation_session_id(&path),
            root: cwd.display().to_string(),
            size_bytes: meta.len(),
            modified_ms: modified_ms(&meta),
        });
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.modified_ms));
    out.truncate(LIST_CAP);
    out
}

/// 329（2026-10-21）：会话侧车 `recall_log` 的只读投影（/rewind 撤回水位）。
///
/// 侧车只在「打开会话」时读一次全量 JSON——不参与 5 秒一次的探索器轮询。
/// 无 recall_log（旧侧车/未撤回）返回空数组；坏 id / 缺侧车 / 坏 JSON 都是
/// 显式错误（调用方按无水位继续，但不得静默伪造）。
pub fn recall_log(cwd: &Path, session8: &str) -> Result<Vec<Value>, String> {
    if !session8_is_safe(session8) {
        return Err(format!("invalid session id: {session8}"));
    }
    let path = conversations_dir(cwd).join(format!("{session8}.json"));
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("conversation not found: {e}"))?;
    let value: Value =
        serde_json::from_str(&text).map_err(|e| format!("invalid conversation sidecar: {e}"))?;
    Ok(value
        .get("recall_log")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

/// 8 lowercase hex chars (the `.gsa` session-file stem shape).
fn session8_is_safe(session8: &str) -> bool {
    session8.len() == 8
        && session8
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-web-runs-test-{}-{}-{tag}",
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

    #[test]
    fn list_runs_reads_light_metadata_and_terminal_status() {
        let root = temp_root("list");
        let run = root.join(".gsa/runs/RUN-ab12cd34-1");
        std::fs::create_dir_all(&run).unwrap();
        let journal = run.join("events.jsonl");
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_started","timestamp":"2026-09-24T00:00:00Z","payload":{}}),
        );
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_finished","timestamp":"2026-09-24T00:01:00Z","payload":{"status":"completed"}}),
        );
        let runs = list_runs(&root);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_id, "RUN-ab12cd34-1");
        assert_eq!(runs[0].status, "completed");
        assert_eq!(runs[0].started.as_deref(), Some("2026-09-24T00:00:00Z"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn run_without_terminal_event_reads_as_running() {
        let root = temp_root("running");
        let run = root.join(".gsa/runs/RUN-ab12cd34-2");
        std::fs::create_dir_all(&run).unwrap();
        write_line(
            &run.join("events.jsonl"),
            serde_json::json!({"event_type":"run_started","timestamp":"t","payload":{}}),
        );
        let runs = list_runs(&root);
        assert_eq!(runs[0].status, "running");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn runs_beyond_probe_cap_read_as_archived() {
        let root = temp_root("archive");
        // PROBE_CAP + 5 run dirs. mtimes tie within the test (std cannot
        // set them), so the newest-first order is whatever the tie-break
        // yields — assert on counts, which hold under any ordering.
        for n in 0..(PROBE_CAP + 5) {
            let dir = root.join(format!(".gsa/runs/RUN-aaaa{n:04}-{n}"));
            std::fs::create_dir_all(&dir).unwrap();
            write_line(
                &dir.join("events.jsonl"),
                serde_json::json!({"event_type":"run_finished","timestamp":"t","payload":{"status":"completed"}}),
            );
        }
        let runs = list_runs(&root);
        assert_eq!(runs.len(), PROBE_CAP + 5);
        assert_eq!(
            runs.iter().filter(|r| r.status == "completed").count(),
            PROBE_CAP,
            "exactly the probed entries carry terminal status"
        );
        assert_eq!(
            runs.iter().filter(|r| r.status == "archived").count(),
            5,
            "entries beyond the probe cap must skip journal reads"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn slice_events_withholds_partial_tail_and_resumes() {
        let root = temp_root("slice");
        let run = root.join(".gsa/runs/RUN-ab12cd34-3");
        std::fs::create_dir_all(&run).unwrap();
        let journal = run.join("events.jsonl");
        write_line(
            &journal,
            serde_json::json!({"event_type":"run_started","n":1}),
        );
        write_line(
            &journal,
            serde_json::json!({"event_type":"model_output","n":2}),
        );
        // Partial trailing line (no newline — the writer has not flushed the
        // line yet) — must be withheld from the slice.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&journal)
                .unwrap();
            write!(f, "{{\"event_type\":\"tool_started\",\"call\":\"c1\"").unwrap();
        }
        let (lines, next) = slice_events(&root, "RUN-ab12cd34-3", 0, 1024 * 1024).unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["event_type"], "run_started");
        assert_eq!(lines[1]["event_type"], "model_output");
        // The writer completes the SAME line (append-only journals only ever
        // extend the tail); resuming from the withheld offset yields it.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&journal)
                .unwrap();
            writeln!(f, "}}").unwrap();
        }
        let (lines2, _) = slice_events(&root, "RUN-ab12cd34-3", next, 1024 * 1024).unwrap();
        assert_eq!(lines2.len(), 1);
        assert_eq!(lines2[0]["event_type"], "tool_started");
        assert_eq!(lines2[0]["call"], "c1");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn slice_rejects_bad_run_ids() {
        let root = temp_root("badid");
        assert!(slice_events(&root, "../x", 0, 1024).is_none());
        assert!(slice_events(&root, "", 0, 1024).is_none());
        assert!(journal_path(&root, "a/b").is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn list_conversations_reads_json_files_only() {
        let root = temp_root("conv");
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("6ab3dbe5.json"),
            r#"{"schema_version":"0.4","session_id":"6ab3dbe5-1111-2222-3333-444455556666","messages":[]}"#,
        )
        .unwrap();
        std::fs::write(dir.join("notes.txt"), "not a conversation").unwrap();
        let convs = list_conversations(&root);
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].session8, "6ab3dbe5");
        assert_eq!(
            convs[0].session_id.as_deref(),
            Some("6ab3dbe5-1111-2222-3333-444455556666")
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 318 P1-A6 钉：第 65536 字节落在多字节字符中间时，`session_id` 仍须
    /// 可解析（严格 UTF-8 解码会把整段 head 判死，长中文会话随机失去续接）。
    #[test]
    fn conversation_session_id_survives_utf8_boundary_at_probe_cap() {
        let root = temp_root("conv-utf8-boundary");
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        let prefix = r#"{"schema_version":"0.4","session_id":"6ab3dbe5-1111-2222-3333-444455556666","messages":[{"role":"user","content":""#;
        let cap = HEAD_PROBE_BYTES as usize;
        // 让一个 3 字节汉字从 cap-2 开始：探针读 [0, cap) 时正好截在它中间。
        // 先补 ASCII 到 cap-2，再放汉字（以及足够多的后续内容）。
        let prefix_len = prefix.len();
        assert!(prefix_len < cap - 2);
        let mut body = String::with_capacity(cap + 64);
        body.push_str(prefix);
        body.push_str(&"x".repeat(cap - 2 - prefix_len));
        body.push('中'); // bytes: cap-2, cap-1, cap
        body.push_str(&"文".repeat(64));
        body.push_str("\"}]}");
        assert!(
            !body.is_char_boundary(cap),
            "test setup: probe cap must split a multi-byte char"
        );
        std::fs::write(dir.join("6ab3dbe5.json"), &body).unwrap();
        let convs = list_conversations(&root);
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].session8, "6ab3dbe5");
        assert_eq!(
            convs[0].session_id.as_deref(),
            Some("6ab3dbe5-1111-2222-3333-444455556666"),
            "head cut inside a multi-byte char must not drop session_id"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 329 钉：recall_log 只读投影（撤回水位）——有则原样返回，无则空数组；
    /// 坏 id / 缺侧车是显式错误（调用方按无水位继续，但不得静默伪装）。
    #[test]
    fn recall_log_projects_sidecar_watermark_and_rejects_bad_ids() {
        let root = temp_root("recall-log");
        let dir = root.join(".gsa/conversations");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("0a1b2c3d.json"),
            r#"{"schema_version":"0.2.0-draft","session_id":"0a1b2c3d-1111-2222-3333-444455556666","messages":[],"recall_log":[{"recalled_at":1760000000.5,"ordinal":2,"removed_messages":4,"remaining_messages":3,"dropped_supplements":0}]}"#,
        )
        .unwrap();
        let log = recall_log(&root, "0a1b2c3d").unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0]["ordinal"], 2);
        assert_eq!(log[0]["recalled_at"].as_f64(), Some(1760000000.5));

        // 旧侧车（无 recall_log）＝空水位，不是错误。
        std::fs::write(
            dir.join("0a1b2c3e.json"),
            r#"{"schema_version":"0.2.0-draft","session_id":"x","messages":[]}"#,
        )
        .unwrap();
        assert!(recall_log(&root, "0a1b2c3e").unwrap().is_empty());

        // 坏 id（长度/大写/穿越）一律拒绝。
        assert!(recall_log(&root, "../etc").is_err());
        assert!(recall_log(&root, "0A1B2C3D").is_err());
        assert!(recall_log(&root, "0a1b2c3d0").is_err());
        // 缺侧车是显式错误。
        assert!(recall_log(&root, "deadbeef").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 338 钉（真机 c23e0aaa）：回档把同会话全部 journal 的 mtime 压到同一刻
    /// （实测 24 件落在 44 ms 内、16 个不同值），只按 mtime 排序会退化成目录名
    /// 字典序 `0,1,10,…,23,3,4,5,6,7,8,9`——会话回放末尾落在旧 run、标记栏乱
    /// 序、「恢复并继续」取到 run 9。同会话内必须以序号为平局判据。
    #[test]
    fn order_run_entries_breaks_mtime_ties_by_session_sequence() {
        // 平局组（回档同刻写入）：字典序输入必须还原为序号降序（新在前）。
        let same_ms = 1_792_671_547_700u128;
        let ids: Vec<String> = [
            "RUN-c23e0aaa-0",
            "RUN-c23e0aaa-1",
            "RUN-c23e0aaa-10",
            "RUN-c23e0aaa-11",
            "RUN-c23e0aaa-2",
            "RUN-c23e0aaa-23",
            "RUN-c23e0aaa-3",
            "RUN-c23e0aaa-9",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let mut entries: Vec<(String, u128)> = ids.into_iter().map(|id| (id, same_ms)).collect();
        // 打乱一次，确认顺序不依赖输入次序（readdir 顺序不是排序依据）。
        entries.reverse();
        let ordered = order_run_entries(entries);
        assert_eq!(
            ordered,
            vec![
                "RUN-c23e0aaa-23",
                "RUN-c23e0aaa-11",
                "RUN-c23e0aaa-10",
                "RUN-c23e0aaa-9",
                "RUN-c23e0aaa-3",
                "RUN-c23e0aaa-2",
                "RUN-c23e0aaa-1",
                "RUN-c23e0aaa-0",
            ],
            "同 mtime 的同会话 run 必须按序号降序（新在前），不得落回字典序"
        );

        // 跨会话仍以 mtime 为主（新会话的 run 排在前），平局才看序号。
        let ordered = order_run_entries(vec![
            ("RUN-aaaaaaaa-0".to_string(), same_ms),
            ("RUN-bbbbbbbb-5".to_string(), same_ms + 1),
            ("RUN-aaaaaaaa-2".to_string(), same_ms),
        ]);
        assert_eq!(ordered[0], "RUN-bbbbbbbb-5", "较新 mtime 必须排最前");
        assert_eq!(ordered[1], "RUN-aaaaaaaa-2", "同 mtime 组内仍按序号降序");
        assert_eq!(ordered[2], "RUN-aaaaaaaa-0");

        // 非序号形态（RUN-CLI-<s8> 无尾序号）不参与序号比较，仍确定性排序。
        let ordered = order_run_entries(vec![
            ("RUN-CLI-cccccccc".to_string(), same_ms),
            ("RUN-cccccccc-3".to_string(), same_ms),
        ]);
        assert_eq!(ordered.len(), 2, "两条都必须保留");
        assert_eq!(
            ordered[0], "RUN-cccccccc-3",
            "有序号者优先（视作同刻内更具体）；无序号形态不 panic"
        );
    }
}
