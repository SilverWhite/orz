//! Read-only session-archive projection (0br S3 新增面，用户令
//! 2026-09-25「会话的归档和查看归档会话都要做的」): listing of
//! `.gsa/archives/*.json.gz` packages and a bounded in-gzip summary
//! (envelope facts + three-key block + message transcript).
//!
//! The agent side owns archive *writing* (`orz-host/acp_server.rs`:
//! `package_session_archive` → envelope
//! `{"schema":"session-archive-package-v0.2","conversation":<sidecar>,
//!   "archive_keys":{…}}` + `.milestones.json` watermark); this surface
//! only *reads* — no arbitrary-path reads (ids pass
//! `security::session_id_ok`), no content transformation beyond
//! truncation, and the response is read-only projection all the way to
//! the browser (ADR-0010 §14.78 条 5: UI 零执行事实).

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

/// Listing cap — same posture as the run/conversation listings.
const ARCHIVE_LIST_CAP: usize = 200;
/// Refuse to buffer absurd compressed inputs (largest observed package is
/// ≈3 MiB; this cap is two orders above it).
const ARCHIVE_MAX_COMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
/// Decompressed-size cap enforced while streaming (`take`): a gzip bomb
/// must not balloon memory even though the file name is local.
const ARCHIVE_MAX_RAW_BYTES: u64 = 128 * 1024 * 1024;
/// Per-message transcript cap — mirrors the large-file-read contract's
/// bounded-preview posture (ADR-0010 §14.22): truncate with a marker,
/// never return unbounded bodies.
const ARCHIVE_MSG_CAP_CHARS: usize = 4000;
/// Whole-transcript cap; beyond it the response says so
/// (`transcript_truncated`) and full facts stay in the archive itself.
const ARCHIVE_TRANSCRIPT_MAX_CHARS: usize = 1024 * 1024;
/// Tool-argument JSON cap per call (front end summarizes further).
const ARCHIVE_ARGS_CAP_CHARS: usize = 2000;

#[derive(Debug, Serialize)]
pub struct ArchiveSummary {
    pub session8: String,
    pub size_bytes: u64,
    pub modified_ms: u128,
    /// From the sibling `.milestones.json` watermark (best-effort).
    pub archived_at: Option<String>,
    /// Full-conversation token estimate at archive time (watermark file).
    pub archived_tokens: Option<u64>,
}

/// Read failure taxonomy — mapped to HTTP statuses by the server layer.
#[derive(Debug)]
pub enum ArchiveReadError {
    /// Fails `security::session_id_ok` — never touch the filesystem.
    InvalidId,
    /// No `{s8}.json.gz` under `.gsa/archives/`.
    Missing,
    /// Compressed or decompressed size beyond the caps.
    TooLarge,
    /// Not gzip, not JSON, or an unrecognized package shape.
    Corrupt(String),
}

impl ArchiveReadError {
    pub fn message(&self) -> String {
        match self {
            ArchiveReadError::InvalidId => "无效的会话标识".into(),
            ArchiveReadError::Missing => "归档不存在（或已清扫）".into(),
            ArchiveReadError::TooLarge => {
                "归档包超出只读投影上限（64 MiB 压缩 / 128 MiB 解压）".into()
            }
            ArchiveReadError::Corrupt(why) => format!("归档包无法解析: {why}"),
        }
    }
}

fn archives_dir(cwd: &Path) -> PathBuf {
    cwd.join(".gsa").join("archives")
}

fn modified_ms(meta: &std::fs::Metadata) -> u128 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Best-effort milestone facts (`{s8}.milestones.json`; missing/corrupt ⇒
/// both `None` — the watermark is advisory metadata, never load-bearing).
fn milestone_facts(cwd: &Path, session8: &str) -> (Option<String>, Option<u64>) {
    let Ok(raw) =
        std::fs::read_to_string(archives_dir(cwd).join(format!("{session8}.milestones.json")))
    else {
        return (None, None);
    };
    let Ok(v) = serde_json::from_str::<Value>(&raw) else {
        return (None, None);
    };
    (
        v.get("archived_at")
            .and_then(Value::as_str)
            .map(str::to_string),
        v.get("archived_tokens").and_then(Value::as_u64),
    )
}

/// List `.gsa/archives/{s8}.json.gz` packages, newest first, capped.
/// Only `*.json.gz` files whose stem passes the id gate are listed; the
/// `.milestones.json` siblings are never listed standalone.
pub fn list_archives(cwd: &Path) -> Vec<ArchiveSummary> {
    let mut out: Vec<ArchiveSummary> = Vec::new();
    let Ok(entries) = std::fs::read_dir(archives_dir(cwd)) else {
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
        let Some(session8) = name.strip_suffix(".json.gz") else {
            continue;
        };
        if !crate::security::session_id_ok(session8) {
            continue;
        }
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let (archived_at, archived_tokens) = milestone_facts(cwd, session8);
        out.push(ArchiveSummary {
            session8: session8.to_string(),
            size_bytes: meta.len(),
            modified_ms: modified_ms(&meta),
            archived_at,
            archived_tokens,
        });
    }
    out.sort_by_key(|a| std::cmp::Reverse(a.modified_ms));
    out.truncate(ARCHIVE_LIST_CAP);
    out
}

/// Stream-decompress the package under both caps.
fn read_archive_bytes(cwd: &Path, session8: &str) -> Result<Vec<u8>, ArchiveReadError> {
    use std::io::Read;
    if !crate::security::session_id_ok(session8) {
        return Err(ArchiveReadError::InvalidId);
    }
    let path = archives_dir(cwd).join(format!("{session8}.json.gz"));
    let meta = std::fs::metadata(&path).map_err(|_| ArchiveReadError::Missing)?;
    if meta.len() > ARCHIVE_MAX_COMPRESSED_BYTES {
        return Err(ArchiveReadError::TooLarge);
    }
    let file = std::fs::File::open(&path).map_err(|_| ArchiveReadError::Missing)?;
    let mut decoder = flate2::read::GzDecoder::new(file);
    let mut raw = Vec::new();
    // `take(cap + 1)` distinguishes "exactly at the cap" from "over it".
    decoder
        .by_ref()
        .take(ARCHIVE_MAX_RAW_BYTES + 1)
        .read_to_end(&mut raw)
        .map_err(|e| ArchiveReadError::Corrupt(format!("gzip 解压失败: {e}")))?;
    if raw.len() as u64 > ARCHIVE_MAX_RAW_BYTES {
        return Err(ArchiveReadError::TooLarge);
    }
    Ok(raw)
}

/// Envelope discrimination, tolerant exactly like the writer-side
/// `decode_archive_package`: v0.2 envelope (`schema` + `conversation`
/// [+ `archive_keys`]) or legacy bare package (the gzip body directly is
/// the sidecar conversation). Anything else is corrupt.
fn split_envelope(v: Value) -> Result<(Value, Option<Value>), ArchiveReadError> {
    if v.get("schema").and_then(Value::as_str).is_some()
        && let Some(conversation) = v.get("conversation")
    {
        return Ok((conversation.clone(), v.get("archive_keys").cloned()));
    }
    if v.get("messages").is_some() {
        return Ok((v, None));
    }
    Err(ArchiveReadError::Corrupt(
        "既非 session-archive-package-v0.2 信封、亦非旧裸包（无 messages 成员）".into(),
    ))
}

fn cap_chars(s: &str, cap: usize) -> (String, usize, bool) {
    let total = s.chars().count();
    if total <= cap {
        (s.to_string(), total, false)
    } else {
        (s.chars().take(cap).collect(), total, true)
    }
}

fn compact_arguments(args: Option<&Value>) -> String {
    let text = match args {
        Some(v) => serde_json::to_string(v).unwrap_or_default(),
        None => String::new(),
    };
    cap_chars(&text, ARCHIVE_ARGS_CAP_CHARS).0
}

/// Build the detail response: conversation facts + archive_keys passthrough
/// + bounded transcript. Pure projection — every field is read off the
/// package or its watermark, none invented.
fn build_detail(
    session8: &str,
    conversation: &Value,
    archive_keys: Option<Value>,
    archived_at: Option<String>,
    archived_tokens: Option<u64>,
) -> Value {
    let messages = conversation
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut user_count = 0u64;
    let mut assistant_count = 0u64;
    let mut tool_results = 0u64;
    let mut tool_calls_total = 0u64;
    let mut transcript: Vec<Value> = Vec::new();
    let mut transcript_chars = 0usize;
    let mut transcript_truncated = false;

    for m in &messages {
        let role = m.get("role").and_then(Value::as_str).unwrap_or("");
        match role {
            "user" => user_count += 1,
            "assistant" => assistant_count += 1,
            "tool" => tool_results += 1,
            _ => {}
        }
        let content = m.get("content").and_then(Value::as_str).unwrap_or("");
        let (body, total, truncated) = cap_chars(content, ARCHIVE_MSG_CAP_CHARS);
        if transcript_chars + body.chars().count() > ARCHIVE_TRANSCRIPT_MAX_CHARS {
            transcript_truncated = true;
            break;
        }
        transcript_chars += body.chars().count();
        let mut entry = json!({
            "role": role,
            "content": body,
            "content_chars": total,
            "truncated": truncated,
        });
        if let Some(round) = m.get("round").and_then(Value::as_u64) {
            entry["round"] = Value::from(round);
        }
        if let Some(call_id) = m.get("tool_call_id").and_then(Value::as_str) {
            entry["tool_call_id"] = json!(call_id);
        }
        if role == "assistant" {
            let calls = m.get("tool_calls").and_then(Value::as_array);
            let calls = calls.map(Vec::as_slice).unwrap_or(&[]);
            tool_calls_total += calls.len() as u64;
            let rendered: Vec<Value> = calls
                .iter()
                .map(|c| {
                    json!({
                        "name": c.get("name").and_then(Value::as_str).unwrap_or(""),
                        "arguments": compact_arguments(c.get("arguments")),
                        "call_id": c.get("call_id").and_then(Value::as_str).unwrap_or(""),
                    })
                })
                .collect();
            entry["tool_calls"] = Value::from(rendered);
        }
        transcript.push(entry);
    }

    let lif = conversation.get("lif");
    json!({
        "session8": session8,
        "session_id": conversation.get("session_id"),
        "archived_at": archived_at,
        "archived_tokens": archived_tokens,
        "message_count": messages.len(),
        "user_messages": user_count,
        "assistant_messages": assistant_count,
        "tool_results": tool_results,
        "tool_calls": tool_calls_total,
        "session_started_at": conversation.get("session_started_at"),
        "lif": lif,
        "archive_keys": archive_keys,
        "messages_total": messages.len(),
        "messages_shown": transcript.len(),
        "transcript_truncated": transcript_truncated,
        "transcript_chars": transcript_chars,
        "messages": transcript,
    })
}

/// Full detail for one archive package (summary facts + bounded
/// transcript). On-demand — the Explorer polls only the cheap listing.
pub fn archive_detail(cwd: &Path, session8: &str) -> Result<Value, ArchiveReadError> {
    let raw = read_archive_bytes(cwd, session8)?;
    let envelope: Value = serde_json::from_slice(&raw)
        .map_err(|e| ArchiveReadError::Corrupt(format!("归档包不是合法 JSON: {e}")))?;
    let (conversation, keys) = split_envelope(envelope)?;
    let (archived_at, archived_tokens) = milestone_facts(cwd, session8);
    Ok(build_detail(
        session8,
        &conversation,
        keys,
        archived_at,
        archived_tokens,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-web-archives-test-{}-{}-{tag}",
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

    /// Write a package the same way the agent side does: gzip of the
    /// v0.2 envelope JSON (flate2 GzEncoder, default compression).
    fn write_package(cwd: &Path, session8: &str, envelope: Value) {
        use std::io::Write;
        let dir = archives_dir(cwd);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{session8}.json.gz"));
        let file = std::fs::File::create(&path).unwrap();
        let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        enc.write_all(envelope.to_string().as_bytes()).unwrap();
        enc.finish().unwrap();
    }

    fn envelope(conversation: Value) -> Value {
        json!({
            "schema": "session-archive-package-v0.2",
            "conversation": conversation,
            "archive_keys": {"lif": {"round_start": 1, "round_end": 9}, "ledger": {"exists": false}},
        })
    }

    #[test]
    fn listing_reads_only_valid_gz_packages_with_milestones() {
        let root = temp_root("list");
        write_package(&root, "6aab1234", envelope(json!({"messages": []})));
        std::fs::write(
            archives_dir(&root).join("6aab1234.milestones.json"),
            r#"{"archived_tokens":505560,"archived_at":"2026-09-17T16:18:33Z"}"#,
        )
        .unwrap();
        // Junk that must never be listed: non-package files, ids failing
        // the gate, and a milestone file without its package.
        std::fs::write(archives_dir(&root).join("notes.txt"), "x").unwrap();
        std::fs::write(archives_dir(&root).join("bad name.json.gz"), "x").unwrap();
        std::fs::write(
            archives_dir(&root).join("6aab1234.milestones.json.bak"),
            "x",
        )
        .unwrap();

        let list = list_archives(&root);
        assert_eq!(
            list.len(),
            1,
            "only valid {{s8}}.json.gz names pass the gate"
        );
        assert_eq!(list[0].session8, "6aab1234");
        assert_eq!(list[0].archived_tokens, Some(505560));
        assert_eq!(list[0].archived_at.as_deref(), Some("2026-09-17T16:18:33Z"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detail_parses_envelope_counts_and_truncates() {
        let root = temp_root("detail");
        let long = "x".repeat(ARCHIVE_MSG_CAP_CHARS + 100);
        let conversation = json!({
            "schema_version": 2,
            "session_id": "sess-6aab1234-full",
            "session_started_at": 123.5,
            "messages": [
                {"role": "user", "content": "问题一", "round": 1},
                {"role": "assistant", "content": long, "round": 1,
                 "tool_calls": [{"name": "bash", "arguments": {"command": "dir"}, "call_id": "c1"}]},
                {"role": "tool", "content": "结果", "tool_call_id": "c1"},
            ],
        });
        write_package(&root, "6aab1234", envelope(conversation));

        let detail = archive_detail(&root, "6aab1234").unwrap();
        assert_eq!(detail["session8"], "6aab1234");
        assert_eq!(detail["message_count"], 3);
        assert_eq!(detail["user_messages"], 1);
        assert_eq!(detail["assistant_messages"], 1);
        assert_eq!(detail["tool_results"], 1);
        assert_eq!(detail["tool_calls"], 1);
        assert_eq!(detail["messages_total"], 3);
        assert_eq!(detail["messages_shown"], 3);
        assert_eq!(detail["archive_keys"]["lif"]["round_end"], 9);
        let assistant = &detail["messages"][1];
        assert_eq!(assistant["truncated"], true, "超限消息必须截断");
        assert_eq!(
            assistant["content_chars"].as_u64().unwrap() as usize,
            ARCHIVE_MSG_CAP_CHARS + 100,
            "截断消息必须保留原长计数"
        );
        assert_eq!(assistant["tool_calls"][0]["name"], "bash");
        assert_eq!(detail["transcript_truncated"], false);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detail_accepts_legacy_bare_package() {
        let root = temp_root("bare");
        // Old packages: the gzip body directly is the StoredConversation.
        write_package(
            &root,
            "6aacdead",
            serde_json::json!({"schema_version": 1, "messages": [{"role": "user", "content": "hi"}]}),
        );
        let detail = archive_detail(&root, "6aacdead").unwrap();
        assert_eq!(detail["message_count"], 1);
        assert!(detail["archive_keys"].is_null(), "旧裸包无三键段");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn detail_errors_are_typed() {
        let root = temp_root("errors");
        assert!(
            matches!(
                archive_detail(&root, "../evil"),
                Err(ArchiveReadError::InvalidId)
            ),
            "id 门先于文件系统"
        );
        assert!(
            matches!(
                archive_detail(&root, "6aab1234"),
                Err(ArchiveReadError::Missing)
            ),
            "缺失文件＝Missing"
        );
        // Not gzip at all.
        std::fs::create_dir_all(archives_dir(&root)).unwrap();
        std::fs::write(archives_dir(&root).join("6aabcafe.json.gz"), b"plain").unwrap();
        assert!(matches!(
            archive_detail(&root, "6aabcafe"),
            Err(ArchiveReadError::Corrupt(_))
        ));
        // Gzip of non-JSON.
        {
            use std::io::Write;
            let file = std::fs::File::create(archives_dir(&root).join("6aabbeef.json.gz")).unwrap();
            let mut enc = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            enc.write_all(b"not json").unwrap();
            enc.finish().unwrap();
        }
        assert!(matches!(
            archive_detail(&root, "6aabbeef"),
            Err(ArchiveReadError::Corrupt(_))
        ));
        // Unrecognized JSON shape (no schema/conversation/messages).
        write_package(&root, "6aabf00d", json!({"something": "else"}));
        assert!(matches!(
            archive_detail(&root, "6aabf00d"),
            Err(ArchiveReadError::Corrupt(_))
        ));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn transcript_cap_stops_adding_and_flags() {
        let root = temp_root("cap");
        let big = "y".repeat(ARCHIVE_MSG_CAP_CHARS);
        let mut messages = Vec::new();
        for _ in 0..(ARCHIVE_TRANSCRIPT_MAX_CHARS / ARCHIVE_MSG_CAP_CHARS + 10) {
            messages.push(json!({"role": "user", "content": big}));
        }
        write_package(&root, "6aab7777", envelope(json!({"messages": messages})));
        let detail = archive_detail(&root, "6aab7777").unwrap();
        assert_eq!(detail["transcript_truncated"], true);
        assert!(
            detail["messages_shown"].as_u64().unwrap() < detail["messages_total"].as_u64().unwrap(),
            "超限后必须停止装载"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
