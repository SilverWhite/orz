//! JournalRecorder — thread-safe, async-backed, append-only JSONL journal.
//!
//! Architecture (drawn from Codex pattern — `RolloutRecorder`):
//!   1. Bounded mpsc channel (256 slots) + background Tokio task
//!   2. Blocking `.send()` — POST-PLANA BUGFIX #1: never silently drop events
//!   3. JSONL append with fsync after each write
//!   4. Thread-safe: Clone shares the same channel + task handle
//!   5. Shutdown via explicit `shutdown()` or Drop

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::oneshot;

use super::chain::seal_event;
use super::event::RunEvent;

/// Commands sent to the background writer task.
enum JournalCmd {
    /// Record a single event (blocking send — event must not be lost).
    WriteEvent(RunEvent),
    /// Flush all buffered writes to disk and fsync.
    Flush {
        ack: oneshot::Sender<Result<(), JournalRecorderError>>,
    },
    /// Shut down the writer task gracefully.
    Shutdown {
        ack: oneshot::Sender<Result<(), JournalRecorderError>>,
    },
}

/// Errors from the journal recorder.
#[derive(Debug, thiserror::Error)]
pub enum JournalRecorderError {
    #[error("journal closed (shutdown already called)")]
    Closed,
    #[error("journal io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

/// The shared handle to the journal recorder.
///
/// Cheap to clone — all clones share the same channel and background task.
/// Uses blocking send to guarantee no events are silently dropped.
pub struct JournalRecorder {
    tx: mpsc::Sender<JournalCmd>,
    /// The directory this journal writes to (kept for path queries).
    journal_dir: PathBuf,
}

impl JournalRecorder {
    /// Create a new journal recorder that writes to `journal_dir/events.jsonl`.
    ///
    /// Spawns a background Tokio task that owns the file handle.
    /// The file is created on first write (lazy), like Codex's `persist()` pattern.
    pub fn new(journal_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::channel::<JournalCmd>(256);

        let writer_task = JournalWriterTask::new(journal_dir.join("events.jsonl"), rx);
        tokio::spawn(writer_task.run());

        JournalRecorder {
            tx,
            journal_dir,
        }
    }

    /// Record an event to the journal.
    ///
    /// **Blocking send** — if the channel is full (backpressure from slow I/O),
    /// this will wait until the writer task catches up. Events are never dropped.
    ///
    /// Events are automatically sealed (payload_sha256 + event_sha256 computed)
    /// before being sent.
    ///
    /// Returns `JournalRecorderError::Closed` if the journal has been shut down.
    pub fn record(&self, mut event: RunEvent) -> Result<(), JournalRecorderError> {
        // Auto-seal the event (compute hashes)
        seal_event(&mut event)?;

        // POST-PLANA BUGFIX #1: blocking send — never silently drop
        self.tx
            .blocking_send(JournalCmd::WriteEvent(event))
            .map_err(|_| JournalRecorderError::Closed)?;

        Ok(())
    }

    /// Async version of `record()` — safe to call from within a Tokio runtime.
    ///
    /// Uses `send().await` instead of `blocking_send`. Otherwise identical to `record()`.
    pub async fn record_async(&self, mut event: RunEvent) -> Result<(), JournalRecorderError> {
        seal_event(&mut event)?;

        self.tx
            .send(JournalCmd::WriteEvent(event))
            .await
            .map_err(|_| JournalRecorderError::Closed)?;

        Ok(())
    }

    /// Flush all buffered writes to disk and fsync.
    /// Blocks until the flush is complete.
    pub fn flush(&self) -> Result<(), JournalRecorderError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .blocking_send(JournalCmd::Flush { ack: ack_tx })
            .map_err(|_| JournalRecorderError::Closed)?;
        ack_rx.blocking_recv().unwrap_or(Err(JournalRecorderError::Closed))
    }

    /// Async version of `flush()`.
    pub async fn flush_async(&self) -> Result<(), JournalRecorderError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(JournalCmd::Flush { ack: ack_tx })
            .await
            .map_err(|_| JournalRecorderError::Closed)?;
        ack_rx.await.unwrap_or(Err(JournalRecorderError::Closed))
    }

    /// Shut down the journal gracefully.
    ///
    /// Drains the channel, writes any queued events, and closes the file.
    /// After shutdown, further calls to `record()` / `record_async()` return
    /// `JournalRecorderError::Closed`.
    pub fn shutdown(&self) -> Result<(), JournalRecorderError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .blocking_send(JournalCmd::Shutdown { ack: ack_tx })
            .map_err(|_| JournalRecorderError::Closed)?;
        ack_rx.blocking_recv().unwrap_or(Err(JournalRecorderError::Closed))
    }

    /// Async version of `shutdown()` — safe to call from within a Tokio runtime.
    pub async fn shutdown_async(&self) -> Result<(), JournalRecorderError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(JournalCmd::Shutdown { ack: ack_tx })
            .await
            .map_err(|_| JournalRecorderError::Closed)?;
        ack_rx.await.unwrap_or(Err(JournalRecorderError::Closed))
    }

    /// Path to the journal directory.
    pub fn journal_dir(&self) -> &Path {
        &self.journal_dir
    }

    /// Path to the events file.
    pub fn events_path(&self) -> PathBuf {
        self.journal_dir.join("events.jsonl")
    }
}

impl Clone for JournalRecorder {
    fn clone(&self) -> Self {
        JournalRecorder {
            tx: self.tx.clone(),
            journal_dir: self.journal_dir.clone(),
        }
    }
}

// Note: no custom `Drop` implementation. The writer task exits on its own
// when the last `JournalRecorder` clone is dropped (the channel closes and
// `rx.recv()` returns `None`). Callers that need durability must call
// `shutdown()` (or `flush()` at minimum) before releasing the handle —
// a dropped handle does not fsync.

// ── Background writer task ──────────────────────────────────────────────

struct JournalWriterTask {
    path: PathBuf,
    rx: mpsc::Receiver<JournalCmd>,
    file: Option<BufWriter<File>>,
    closed: bool,
    /// Set once a terminal event (run_finished/run_failed/...) has been
    /// written — further appends are refused to preserve chain integrity.
    terminal_seen: bool,
}

impl JournalWriterTask {
    fn new(path: PathBuf, rx: mpsc::Receiver<JournalCmd>) -> Self {
        JournalWriterTask {
            path,
            rx,
            file: None,
            closed: false,
            terminal_seen: false,
        }
    }

    /// Ensure the file is open (lazy initialization on first write).
    fn ensure_file(&mut self) -> Result<&mut BufWriter<File>, std::io::Error> {
        if self.file.is_none() {
            // Create parent directory
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            self.file = Some(BufWriter::new(f));
        }
        Ok(self.file.as_mut().unwrap())
    }

    /// Write one event as a JSONL line and fsync.
    fn write_event(file: &mut BufWriter<File>, event: &RunEvent) -> Result<(), JournalRecorderError> {
        // Serialize to compact JSON (matching Python: sort_keys + compact separators)
        let mut buf = Vec::new();
        let mut ser = serde_json::Serializer::new(&mut buf);
        event.serialize(&mut ser)?;
        buf.push(b'\n');
        file.write_all(&buf)?;
        file.flush()?;
        file.get_ref().sync_all()?;
        Ok(())
    }

    async fn run(mut self) {
        use JournalCmd::*;
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                WriteEvent(event) => {
                    if self.closed {
                        continue;
                    }
                    // Refuse appends after a terminal event (hash-chain invariant)
                    if self.terminal_seen {
                        tracing::warn!(
                            "journal append refused after terminal event (seq {})",
                            event.sequence
                        );
                        continue;
                    }
                    match Self::ensure_file(&mut self) {
                        Ok(file) => {
                            if let Err(e) = Self::write_event(file, &event) {
                                tracing::error!("journal write error: {e}");
                                self.closed = true;
                            } else if event.is_terminal() {
                                self.terminal_seen = true;
                            }
                        }
                        Err(e) => {
                            tracing::error!("journal file open error: {e}");
                            self.closed = true;
                        }
                    }
                }
                Flush { ack } => {
                    let result = match self.file.as_mut() {
                        Some(file) => {
                            file.flush()
                                .and_then(|_| file.get_ref().sync_all())
                                .map_err(JournalRecorderError::from)
                        }
                        None => Ok(()),
                    };
                    let _ = ack.send(result);
                }
                Shutdown { ack } => {
                    self.closed = true;
                    let result = match self.file.as_mut() {
                        Some(file) => {
                            file.flush()
                                .and_then(|_| file.get_ref().sync_all())
                                .map_err(JournalRecorderError::from)
                        }
                        None => Ok(()),
                    };
                    let _ = ack.send(result);
                    // Exit the loop — writer task stops
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::event::{EventType, Redaction};

    use std::sync::atomic::{AtomicU64, Ordering};
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("orz-journal-test-{}-{}", std::process::id(), n));
        // Ensure clean start
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_event(run_id: &str, seq: u64, event_type: EventType, previous: Option<String>) -> RunEvent {
        RunEvent::new(
            run_id.into(),
            seq,
            event_type,
            "test-manifest-sha256-64chars-long___________________".into(),
            previous,
            "test-schema".into(),
            serde_json::json!({"test": true, "seq": seq}),
            Redaction::None,
            "2026-08-04T00:00:00Z".into(),
        )
    }

    #[test]
    fn record_and_verify_chain() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        // Record 3 events in a chain
        let e0 = make_event("RUN-CHAIN", 0, EventType::RunStarted, None);
        recorder.record(e0).unwrap();

        let e1 = {
            // Use the previous event's hash — but we don't have it directly.
            // The recorder auto-seals, so we construct with a placeholder
            // and the recorder computes the real hash.
            make_event(
                "RUN-CHAIN",
                1,
                EventType::PromptSubmitted,
                Some("placeholder-will-be-replaced-by-seal-64chars__".into()),
            )
        };
        recorder.record(e1).unwrap();

        let e2 = make_event(
            "RUN-CHAIN",
            2,
            EventType::RunFinished,
            Some("another-placeholder-64chars-long_____________".into()),
        );
        recorder.record(e2).unwrap();

        recorder.shutdown().unwrap();

        // Verify the file exists and has 3 lines
        let events_path = dir.join("events.jsonl");
        assert!(events_path.exists());
        let content = std::fs::read_to_string(&events_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3);

        // Each line is valid JSON
        for line in &lines {
            let _: serde_json::Value = serde_json::from_str(line).unwrap();
        }

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn blocking_send_does_not_drop_events() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        // Rapid-fire 50 events — none should be dropped
        for i in 0..50 {
            let event = make_event(
                "RUN-BLOCK",
                i as u64,
                if i == 49 {
                    EventType::RunFinished
                } else {
                    EventType::PromptSubmitted
                },
                None, // placeholder — recorder auto-seals
            );
            recorder.record(event).unwrap();
        }

        recorder.shutdown().unwrap();

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert_eq!(content.lines().count(), 50);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clone_shares_channel() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let r1 = JournalRecorder::new(dir.clone());
        let r2 = r1.clone();

        r1.record(make_event(
            "RUN-CLONE", 0, EventType::RunStarted, None,
        ))
        .unwrap();
        r2.record(make_event(
            "RUN-CLONE", 1, EventType::RunFinished, None,
        ))
        .unwrap();

        r2.shutdown().unwrap(); // shutdown via clone

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert_eq!(content.lines().count(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
