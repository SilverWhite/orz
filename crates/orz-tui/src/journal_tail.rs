//! Journal tailer — a polling thread that follows an `events.jsonl` file
//! (fsync-per-line append-only, so every poll sees all committed lines).
//!
//! Mirrors the Python `LiveRunEventSource` daemon-thread design (50 ms
//! poll). Missing files are retried (the host creates the run dir moments
//! after the prompt RPC); partial trailing lines are kept until completed;
//! a terminal event or the stop flag ends the tail.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;

use orz_assurance::journal::{EventType, RunEvent};

/// Default poll interval (Python drain loop parity).
pub const POLL_INTERVAL: Duration = Duration::from_millis(50);
/// How long to keep retrying a missing journal before giving up.
pub const MISSING_FILE_RETRY: Duration = Duration::from_secs(60);

/// A running tailer handle.
pub struct JournalTail {
    pub path: PathBuf,
    pub rx: mpsc::UnboundedReceiver<RunEvent>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl JournalTail {
    /// Start tailing *path*; events flow out through the returned receiver.
    pub fn spawn(path: PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let thread_path = path.clone();
        let handle = std::thread::Builder::new()
            .name("orz-tui-journal-tail".into())
            .spawn(move || {
                tail_loop(&thread_path, tx, stop_flag);
            })
            .expect("spawn journal tail thread");
        Self {
            path,
            rx,
            stop,
            handle: Some(handle),
        }
    }

    /// Stop the tail loop (best effort; the loop also ends on terminal).
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }

    /// Drain any buffered events without blocking.
    pub fn poll(&mut self) -> Vec<RunEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            out.push(ev);
        }
        out
    }
}

impl Drop for JournalTail {
    fn drop(&mut self) {
        // Join so the polling thread cannot outlive the tail (review P3 #4 —
        // without the join, a future loop change could leak the thread).
        self.stop();
    }
}

/// Peek the first prompt text of a sealed run journal — the session list's
/// preview column (Phase 3 slice #9). Reads at most the first 10 lines and
/// returns the `payload.prompt` of the first `run_started` /
/// `prompt_submitted` event; `None` when the journal is missing/unreadable
/// or carries no prompt (restore runs, empty journals).
pub fn peek_first_prompt(path: &Path) -> Option<String> {
    use std::io::BufRead;
    let file = File::options().read(true).open(path).ok()?;
    let reader = std::io::BufReader::new(file);
    for line in reader.lines().take(10).flatten() {
        // A corrupt/blank line must not abort the peek — later lines can
        // still carry the prompt (review P3-4).
        let Ok(ev) = serde_json::from_str::<RunEvent>(&line) else {
            continue;
        };
        if matches!(
            ev.event_type,
            EventType::RunStarted | EventType::PromptSubmitted
        ) && let Some(prompt) = ev.payload.get("prompt").and_then(|v| v.as_str())
        {
            return Some(prompt.to_string());
        }
    }
    None
}

fn tail_loop(
    path: &Path,
    tx: mpsc::UnboundedSender<RunEvent>,
    stop: Arc<AtomicBool>,
) {
    let mut file: Option<File> = None;
    let mut offset: u64 = 0;
    let mut pending: Vec<u8> = Vec::new();
    let mut missing_since = std::time::Instant::now();
    let mut ended = false;

    while !stop.load(Ordering::SeqCst) && !ended {
        if file.is_none() {
            match File::options().read(true).open(path) {
                Ok(f) => {
                    file = Some(f);
                    missing_since = std::time::Instant::now();
                }
                Err(_) => {
                    if missing_since.elapsed() > MISSING_FILE_RETRY {
                        tracing::warn!(
                            "journal tail: {} never appeared within {:?}",
                            path.display(),
                            MISSING_FILE_RETRY
                        );
                        break;
                    }
                    std::thread::sleep(POLL_INTERVAL);
                    continue;
                }
            }
        }
        let Some(f) = file.as_mut() else {
            continue;
        };

        // Read whatever is past the current offset.
        if f.seek(SeekFrom::Start(offset)).is_err() {
            // File vanished/replaced — reset.
            file = None;
            offset = 0;
            pending.clear();
            continue;
        }
        let mut buf = [0u8; 8192];
        match f.read(&mut buf) {
            Ok(0) => {
                // No new data — but the file may have been replaced.
                std::thread::sleep(POLL_INTERVAL);
            }
            Ok(n) => {
                offset += n as u64;
                pending.extend_from_slice(&buf[..n]);
                // split() keeps the trailing partial as the last element;
                // copy out so the borrow on `pending` ends before we clear.
                let mut lines: Vec<Vec<u8>> = pending
                    .split(|b| *b == b'\n')
                    .map(|l| l.to_vec())
                    .collect();
                let partial = lines.pop();
                pending.clear();
                for line in lines {
                    if line.is_empty() {
                        continue;
                    }
                    match serde_json::from_slice::<RunEvent>(&line) {
                        Ok(ev) => {
                            if ev.event_type.is_terminal() {
                                ended = true;
                            }
                            if tx.send(ev).is_err() {
                                // Receiver gone — stop tailing.
                                return;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("journal tail: unparseable line: {e}");
                        }
                    }
                }
                if let Some(p) = partial
                    && !p.is_empty()
                {
                    pending = p;
                }
            }
            Err(_) => {
                // Read error (e.g. file locked mid-write) — retry next poll.
                std::thread::sleep(POLL_INTERVAL);
            }
        }
    }
    let _ = tx;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;

    fn test_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-tail-{name}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_line(path: &Path, line: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(line.as_bytes()).unwrap();
        f.write_all(b"\n").unwrap();
        f.sync_all().unwrap(); // fsync per line — the tail sees it next poll
    }

    fn sample_line() -> String {
        // A minimal valid RunEvent-shaped JSON line.
        format!(
            "{}",
            serde_json::json!({
                "schema_version": "0.1.0-draft",
                "run_id": "RUN-TAIL01",
                "event_id": "EVT-TAIL01-000",
                "sequence": 0,
                "timestamp": "2026-08-05T00:00:00Z",
                "event_type": "run_started",
                "run_manifest_sha256": "m",
                "previous_event_sha256": null,
                "payload_schema": "run-event-v0.1.schema.json",
                "payload": {"prompt": "hi"},
                "payload_sha256": "p",
                "redaction": "none",
                "event_sha256": "e"
            })
        )
    }

    #[test]
    fn missing_file_retries_then_picks_up() {
        let dir = test_dir("missing");
        let path = dir.join("events.jsonl");
        let _ = std::fs::remove_file(&path);
        let mut tail = JournalTail::spawn(path.clone());
        // Give the thread a couple of polls before the file appears.
        std::thread::sleep(Duration::from_millis(120));
        write_line(&path, &sample_line());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let events = tail.poll();
            if !events.is_empty() {
                assert_eq!(events[0].event_type.to_string(), "run_started");
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for the tailed event"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        tail.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn partial_trailing_line_is_kept_until_completed() {
        let dir = test_dir("partial");
        let path = dir.join("events.jsonl");
        let _ = std::fs::remove_file(&path);
        let mut tail = JournalTail::spawn(path.clone());
        std::thread::sleep(Duration::from_millis(120));

        // Write half a line without newline — must NOT be parsed yet.
        let line = sample_line();
        let split_at = line.len() / 2;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .unwrap();
        f.write_all(&line.as_bytes()[..split_at]).unwrap();
        f.sync_all().unwrap();
        std::thread::sleep(Duration::from_millis(120));
        assert!(tail.poll().is_empty(), "partial line must not be parsed");

        // Complete the line with its newline.
        f.write_all(&line.as_bytes()[split_at..]).unwrap();
        f.write_all(b"\n").unwrap();
        f.sync_all().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let events = tail.poll();
            if events.len() == 1 {
                assert_eq!(events[0].event_type.to_string(), "run_started");
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for the completed line"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        tail.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn malformed_line_is_skipped_without_panic() {
        let dir = test_dir("malformed");
        let path = dir.join("events.jsonl");
        let _ = std::fs::remove_file(&path);
        let mut tail = JournalTail::spawn(path.clone());
        std::thread::sleep(Duration::from_millis(120));
        write_line(&path, "this is not json");
        write_line(&path, &sample_line());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            let events = tail.poll();
            if events.len() == 1 {
                assert_eq!(events[0].event_type.to_string(), "run_started");
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out; got {:?} events",
                events.len()
            );
            std::thread::sleep(Duration::from_millis(30));
        }
        tail.stop();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
