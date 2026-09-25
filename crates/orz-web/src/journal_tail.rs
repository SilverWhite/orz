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

/// Run the tail loop until the socket closes or the file goes away for
/// `MISSING_FILE_RETRY`. `start_offset` lets the client resume a replay
/// (`/api/run/{id}/events` `next_offset` semantics).
pub async fn run_tail(mut socket: WebSocket, path: PathBuf, start_offset: u64) {
    let mut offset = start_offset;
    let mut missing_since: Option<tokio::time::Instant> = None;
    let mut buf = vec![0u8; 64 * 1024];
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
        if size < offset {
            // Truncated/rewritten journal — restart from zero rather than
            // seeking past EOF forever.
            offset = 0;
        }
        if file.seek(SeekFrom::Start(offset)).await.is_err() {
            tokio::time::sleep(POLL_INTERVAL).await;
            continue;
        }
        loop {
            match file.read(&mut buf).await {
                Ok(0) => break,
                Ok(n) => {
                    // Only forward COMPLETE lines; withhold the partial
                    // tail for the next poll (byte offset stays stable).
                    let chunk = &buf[..n];
                    let upto = match chunk.iter().rposition(|b| *b == b'\n') {
                        Some(pos) => pos + 1,
                        None => 0,
                    };
                    if upto > 0 {
                        let text = String::from_utf8_lossy(&chunk[..upto]);
                        for line in text.lines() {
                            if line.is_empty() {
                                continue;
                            }
                            if socket.send(Message::text(line.to_string())).await.is_err() {
                                return; // client went away
                            }
                        }
                        offset += upto as u64;
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
