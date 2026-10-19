//! Live journal tail — a tokio task that follows `{cwd}/.gsa/runs/{run_id}/
//! events.jsonl` (fsync-per-line append-only, so a byte offset never
//! changes meaning) and forwards complete lines as WS text messages.
//!
//! Port of the `orz-tui/journal_tail.rs` polling design (50 ms), moved to
//! the bridge so the browser projection consumes the same canonical facts
//! (`orz-tui` tails in-process; the stdio-lane bridge cannot).

use std::path::{Path, PathBuf};
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket};
use futures::SinkExt;
use tokio::io::{AsyncReadExt, AsyncSeekExt, SeekFrom};

/// Python drain-loop / orz-tui parity poll interval.
const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// How long to keep retrying a missing journal before giving up (the host
/// creates the run dir moments after the prompt RPC).
const MISSING_FILE_RETRY: Duration = Duration::from_secs(60);
/// Read chunk size. A single legitimate journal line can be far larger than
/// this (`supplement_delivered` carries up to 256 KiB of text), so chunks are
/// only a read granularity — never a line boundary.
const READ_CHUNK: usize = 64 * 1024;

/// Pending-buffer line splitter (318 P1-B6, ported from the TUI tail).
///
/// Previously `run_tail` advanced its byte offset only when a read chunk
/// contained a newline. A >64 KiB line therefore split into chunks that were
/// all withheld *and* never retried at a stable offset: the event was dropped
/// or duplicated. Here every byte read advances `offset` immediately; bytes
/// without a newline accumulate in `pending` until the line completes.
struct PendingLines {
    offset: u64,
    pending: Vec<u8>,
}

impl PendingLines {
    fn new(start_offset: u64) -> Self {
        Self {
            offset: start_offset,
            pending: Vec::new(),
        }
    }

    /// Restart after a truncation/rewrite: the pending bytes belong to the
    /// old file contents and are not valid lines of the new one.
    fn reset(&mut self) {
        self.offset = 0;
        self.pending.clear();
    }

    /// Consume a read chunk, advancing the byte offset by the full chunk and
    /// returning only the now-complete, non-empty lines (UTF-8 lossy — the
    /// journal contract is valid UTF-8 JSON, but a torn tail must not panic).
    fn push(&mut self, chunk: &[u8]) -> Vec<String> {
        self.offset += chunk.len() as u64;
        self.pending.extend_from_slice(chunk);
        let Some(last_newline) = self.pending.iter().rposition(|b| *b == b'\n') else {
            return Vec::new();
        };
        let drained: Vec<u8> = self.pending.drain(..=last_newline).collect();
        let mut lines = Vec::new();
        for line in drained.split(|b| *b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
                continue;
            }
            lines.push(String::from_utf8_lossy(line).into_owned());
        }
        lines
    }
}

/// Run the tail loop until the socket closes or the file goes away for
/// `MISSING_FILE_RETRY`. `start_offset` lets the client resume a replay
/// (`/api/run/{id}/events` `next_offset` semantics).
pub async fn run_tail(mut socket: WebSocket, path: PathBuf, start_offset: u64) {
    let mut tail = PendingLines::new(start_offset);
    let mut missing_since: Option<tokio::time::Instant> = None;
    let mut buf = vec![0u8; READ_CHUNK];
    loop {
        let mut file = match tokio::fs::File::open(&path).await {
            Ok(f) => {
                missing_since = None;
                f
            }
            Err(_) => {
                let first = missing_since.is_none();
                let at = missing_since.get_or_insert(tokio::time::Instant::now());
                if first {
                    tracing::info!("journal tail: {} not created yet, waiting", path.display());
                }
                if at.elapsed() > MISSING_FILE_RETRY {
                    tracing::info!(
                        "journal tail giving up: {} missing too long",
                        path.display()
                    );
                    let _ = socket
                        .send(Message::text(
                            r#"{"tail":"closed","reason":"journal_missing"}"#,
                        ))
                        .await;
                    let _ = socket.close().await;
                    return;
                }
                tokio::time::sleep(POLL_INTERVAL).await;
                continue;
            }
        };
        let size = file.metadata().await.map(|m| m.len()).unwrap_or(0);
        if size < tail.offset {
            // Truncated/rewritten journal — restart from zero rather than
            // seeking past EOF forever. Pending bytes are from the old file.
            tail.reset();
        }
        if file.seek(SeekFrom::Start(tail.offset)).await.is_err() {
            tokio::time::sleep(POLL_INTERVAL).await;
            continue;
        }
        loop {
            match file.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    // Every byte is consumed from the file (offset += n);
                    // only COMPLETE lines are forwarded. A partial tail stays
                    // in `pending` and is retried after the next poll.
                    let lines = tail.push(&buf[..n]);
                    for line in lines {
                        if socket.send(Message::text(line)).await.is_err() {
                            return; // client went away
                        }
                    }
                }
                Err(_) => break,
            }
        }
        drop(file);
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Validate + resolve the journal path for a run id (path-safety gate).
pub fn resolve(cwd: &Path, run_id: &str) -> Option<PathBuf> {
    crate::runs::journal_path(cwd, run_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_lines_offsets_every_byte_and_only_emits_complete_lines() {
        let mut tail = PendingLines::new(7);
        assert_eq!(
            tail.push(b"{\"a\":1}\n{\"b\""),
            vec!["{\"a\":1}".to_string()]
        );
        assert_eq!(tail.offset, 7 + 12);
        assert_eq!(tail.push(b":2}\n"), vec!["{\"b\":2}".to_string()]);
        assert_eq!(tail.offset, 7 + 12 + 4);
        // Empty lines are noise, CRLF is normalized by stripping '\r'.
        assert_eq!(tail.push(b"\r\n{\"c\":3}\n"), vec!["{\"c\":3}".to_string()]);
    }

    /// 318 P1-B6 钉：合法 `supplement_delivered` 单行 >256 KiB。修复前
    /// 64 KiB 分块不含换行 ⇒ offset 不前进且事件丢失/重复；现在逐块累计、
    /// 完整换行到达时整行一次发送。
    #[test]
    fn single_supplement_line_over_256k_is_reassembled_once() {
        let text = "补".repeat(100_000); // ~300 KiB UTF-8
        let line = serde_json::json!({
            "event_type": "supplement_delivered",
            "payload": { "supplement_seq": 1, "character_count": 100_000, "text": text }
        })
        .to_string();
        let bytes = line.as_bytes();
        assert!(bytes.len() > 256 * 1024, "fixture must exceed 256 KiB");

        let mut tail = PendingLines::new(0);
        let mut emitted: Vec<String> = Vec::new();
        for chunk in bytes.chunks(READ_CHUNK) {
            emitted.extend(tail.push(chunk));
        }
        assert!(emitted.is_empty(), "no newline yet: nothing may be emitted");
        assert_eq!(
            tail.offset,
            bytes.len() as u64,
            "every read byte advances offset"
        );

        emitted.extend(tail.push(b"\n"));
        assert_eq!(emitted.len(), 1, "exactly one event line, no duplication");
        let parsed: serde_json::Value = serde_json::from_str(&emitted[0]).unwrap();
        assert_eq!(parsed["event_type"], "supplement_delivered");
        assert_eq!(
            parsed["payload"]["text"].as_str().unwrap().chars().count(),
            100_000,
            "the full supplement text must survive the tail"
        );
        assert_eq!(tail.offset, bytes.len() as u64 + 1);
    }

    #[test]
    fn reset_drops_pending_bytes_on_truncation() {
        let mut tail = PendingLines::new(10);
        assert!(tail.push(b"{\"partial\":").is_empty());
        tail.reset();
        assert_eq!(tail.offset, 0);
        assert_eq!(
            tail.push(b"{\"fresh\":1}\n"),
            vec!["{\"fresh\":1}".to_string()]
        );
    }
}
