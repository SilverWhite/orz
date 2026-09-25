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

/// List runs under `{cwd}/.gsa/runs/`, newest first, capped. Content
/// probes (started timestamp + terminal status) run only for the most
/// recent [`PROBE_CAP`] entries — older runs read as `archived` without
/// touching their journals.
pub fn list_runs(cwd: &Path) -> Vec<RunSummary> {
    // Phase 1: cheap directory metadata only.
    let mut entries: Vec<(String, PathBuf)> = Vec::new();
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
        if std::fs::metadata(&journal).is_err() {
            continue;
        }
        entries.push((run_id.to_string(), journal));
    }
    // Newest first by journal mtime, capped before any content probe.
    entries.sort_by_key(|(_, journal)| {
        std::cmp::Reverse(
            std::fs::metadata(journal)
                .ok()
                .map(|m| modified_ms(&m))
                .unwrap_or(0),
        )
    });
    entries.truncate(LIST_CAP);

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
            size_bytes: meta.len(),
            modified_ms: modified_ms(&meta),
        });
    }
    out.sort_by_key(|r| std::cmp::Reverse(r.modified_ms));
    out.truncate(LIST_CAP);
    out
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
        std::fs::write(dir.join("6ab3dbe5.json"), "{}").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a conversation").unwrap();
        let convs = list_conversations(&root);
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].session8, "6ab3dbe5");
        let _ = std::fs::remove_dir_all(&root);
    }
}
