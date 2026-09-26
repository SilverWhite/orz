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
    /// The ack reports the write result plus the sealed on-disk
    /// `event_sha256` (0v-C — the caller threads *that* hash into the next
    /// event's `previous_event_sha256`, never a pre-funnel seal of its own).
    WriteEvent {
        // Boxed — `RunEvent` is large; the enum travels the writer channel.
        event: Box<RunEvent>,
        ack: oneshot::Sender<Result<String, JournalRecorderError>>,
    },
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
    #[error("journal append refused after terminal event (seq {0})")]
    TerminalAppended(u64),
    #[error("journal io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("journal serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    /// FUS-HOST-RESOURCE-SAFETY §4.3 (2026-09-12, 0z S2): the event was NOT
    /// written — the journal is in degraded (skeleton-only) mode and this
    /// event is regenerable face, or the write itself failed after the
    /// storage-full backoff ladder. The caller must **not** advance its
    /// sequence/chain bookkeeping for this event: the on-disk chain only
    /// stays continuous if dropped events never consume a sequence number
    /// (`validate_chain` requires `sequence == line index`).
    #[error("journal degraded mode dropped event (seq {sequence}, {event_type})")]
    DegradedDropped { sequence: u64, event_type: String },
}

/// FUS-HOST-RESOURCE-SAFETY §4.3 item 4 (2026-09-12, 0z S2): the state-chain
/// volume keeps a reserve so the hard tier can still land terminal events.
/// `ORZ_JOURNAL_RESERVED_BYTES` overrides; the value is a *floor contract*
/// for the reclaim ladder (E) — the pre-dispatch gate (A) refuses heavy
/// actions far above it, so build products cannot eat into the reserve.
pub const DEFAULT_JOURNAL_RESERVED_BYTES: u64 = 64 * 1024 * 1024;

/// The configured state-chain reserve in bytes (design §4.3 item 4).
pub fn journal_reserved_bytes() -> u64 {
    std::env::var("ORZ_JOURNAL_RESERVED_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_JOURNAL_RESERVED_BYTES)
}

/// FUS-HOST-RESOURCE-SAFETY §4.3 item 3 (2026-09-12, 0z S2): a single event
/// in degraded mode is bounded (default ≤ 8 KiB serialized). Oversized string
/// values are mechanically truncated to summary+pointer form — the shape of
/// the payload is preserved so the row stays schema-valid, and the original
/// seal travels inside the `degraded` marker as the pointer.
pub const DEGRADED_EVENT_BOUND_BYTES: usize = 8 * 1024;

/// True when an IO error means "the volume cannot accept more data" — the
/// ENOSPC family the design's backoff ladder targets (Windows
/// `ERROR_DISK_FULL` 112 / `ERROR_HANDLE_DISK_FULL` 39; POSIX `ENOSPC` 28 /
/// `EDQUOT` 122; plus the stable `ErrorKind::StorageFull` mapping). Commit
/// exhaustion and every other IO failure is a different axis and keeps the
/// historical fatal semantics.
fn is_storage_full_error(e: &std::io::Error) -> bool {
    if e.kind() == std::io::ErrorKind::StorageFull {
        return true;
    }
    matches!(
        e.raw_os_error(),
        Some(112) | Some(39) | Some(28) | Some(122)
    )
}

/// 0bc ④（2026-09-21，设计 §11-4）：分配失败——可失败分配路径上
/// `try_reserve` 失败统一映射为 `ErrorKind::OutOfMemory`。它和 ENOSPC
/// **同形**进入降级链（不 abort、留终态），但**不走退避梯**（内存不会因
/// 等待而回来）。
fn is_alloc_failure(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::OutOfMemory
}

/// 可失败分配序列化缓冲（0bc ④）。`Vec` 的 `io::Write` 增长路径在 OOM 时
/// 走 `handle_alloc_error`（abort）；journal 装配是 S1 清单里的巨量分配
/// 路径之一，这里把每次增长改走 `try_reserve`，失败以
/// `ErrorKind::OutOfMemory` 浮出，由写者任务映射进降级链。
struct FallibleBuf {
    bytes: Vec<u8>,
}

impl FallibleBuf {
    fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Fallible capacity growth used by both `write` and `push`.
    fn grow(&mut self, additional: usize) -> Result<(), std::io::Error> {
        self.bytes.try_reserve(additional).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "journal assembly allocation failed (0bc fallible-allocation path)",
            )
        })
    }

    fn try_push(&mut self, byte: u8) -> Result<(), std::io::Error> {
        self.grow(1)?;
        self.bytes.push(byte);
        Ok(())
    }
}

impl std::io::Write for FallibleBuf {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.grow(data.len())?;
        self.bytes.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::ops::Deref for FallibleBuf {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.bytes
    }
}

impl FallibleBuf {
    fn len(&self) -> usize {
        self.bytes.len()
    }
}

/// Degraded-mode state — recorded once when the backoff ladder is exhausted.
#[derive(Debug, Clone)]
struct DegradedState {
    entered_at: String,
    cause: String,
    dropped_events: u64,
    first_dropped_sequence: Option<u64>,
    reserved_bytes: u64,
}

impl DegradedState {
    /// The summary object injected into every event written while degraded
    /// (schema-visible evidence for the `degraded_complete` classification).
    fn to_summary_json(&self) -> serde_json::Value {
        serde_json::json!({
            "entered_at": self.entered_at,
            "dropped_events": self.dropped_events,
            "first_dropped_sequence": self.first_dropped_sequence,
            "reserved_bytes": self.reserved_bytes,
            "cause": self.cause,
        })
    }
}

/// The `.gsa/runs/<run>/TERMINAL.json` sidecar (design §4.3 item 5): when the
/// chain itself cannot accept the terminal event, the run still yields an
/// enumerable terminal shape — a fixed, bounded (< 4 KiB) file on the same
/// volume plus a stderr line. 判据 6: `run_terminated` **or** this file, both
/// enumerable, never a silent death.
fn write_terminal_sidecar(
    journal_dir: &Path,
    run_id: &str,
    state: Option<&DegradedState>,
    sequence: u64,
    event_type: &str,
    cause: &str,
) {
    let payload = serde_json::json!({
        "run_id": run_id,
        "reason": "journal_degraded",
        "planned_event_type": event_type,
        "planned_sequence": sequence,
        "degraded": state.map(DegradedState::to_summary_json),
        "cause": cause,
        "written_at": chrono_now_compact(),
        "note": "terminal event could not be appended; this sidecar is the enumerable terminal shape (design §4.3 item 5)",
    });
    let text = match serde_json::to_string_pretty(&payload) {
        Ok(t) => t,
        Err(_) => return,
    };
    let path = journal_dir.join("TERMINAL.json");
    let result = std::fs::write(&path, &text);
    eprintln!(
        "orz journal: terminal event append failed (degraded); terminal shape written to {} \
         (result: {})",
        path.display(),
        match &result {
            Ok(()) => "ok".to_string(),
            Err(e) => format!("failed: {e}"),
        }
    );
}

/// Compact UTC timestamp for the sidecar.
fn chrono_now_compact() -> String {
    chrono::Utc::now().to_rfc3339()
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
        spawn_writer_thread(writer_task);

        JournalRecorder { tx, journal_dir }
    }

    /// Test seam (design §7-C "故障注入层"): a recorder whose first
    /// `fault_attempts` write attempts fail with a storage-full error — the
    /// ENOSPC injection the degraded-mode ladder is verified against. Hidden
    /// from the documented surface; never used by production code.
    #[doc(hidden)]
    pub fn new_with_write_faults_for_tests(journal_dir: PathBuf, fault_attempts: u32) -> Self {
        let (tx, rx) = mpsc::channel::<JournalCmd>(256);

        let mut writer_task = JournalWriterTask::new(journal_dir.join("events.jsonl"), rx);
        writer_task.fault = Some(WriteFaultScript {
            failures_remaining: fault_attempts,
            partial_bytes: 0,
            alloc_failures_remaining: 0,
        });
        spawn_writer_thread(writer_task);

        JournalRecorder { tx, journal_dir }
    }

    /// Test seam with a realistic torn write (review F-C-1/F-C-2): the first
    /// `fault_attempts` attempts write a `partial_bytes`-byte prefix of the
    /// line to disk, THEN fail with storage-full — the partial-write shape
    /// real ENOSPC produces.
    #[doc(hidden)]
    pub fn new_with_torn_write_faults_for_tests(
        journal_dir: PathBuf,
        fault_attempts: u32,
        partial_bytes: usize,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<JournalCmd>(256);

        let mut writer_task = JournalWriterTask::new(journal_dir.join("events.jsonl"), rx);
        writer_task.fault = Some(WriteFaultScript {
            failures_remaining: fault_attempts,
            partial_bytes,
            alloc_failures_remaining: 0,
        });
        spawn_writer_thread(writer_task);

        JournalRecorder { tx, journal_dir }
    }

    /// Test seam (0bc ④, design §11 item 4, 2026-09-21): allocation failures
    /// on the journal assembly path — the first `fault_attempts` appends fail
    /// with `ErrorKind::OutOfMemory`. **No backoff ladder**: memory does not
    /// come back by waiting, so the failure routes straight into the degraded
    /// chain, the same shape as an exhausted ENOSPC ladder. Hidden from the
    /// documented surface; never used by production code.
    #[doc(hidden)]
    pub fn new_with_alloc_faults_for_tests(journal_dir: PathBuf, fault_attempts: u32) -> Self {
        let (tx, rx) = mpsc::channel::<JournalCmd>(256);

        let mut writer_task = JournalWriterTask::new(journal_dir.join("events.jsonl"), rx);
        writer_task.fault = Some(WriteFaultScript {
            failures_remaining: 0,
            partial_bytes: 0,
            alloc_failures_remaining: fault_attempts,
        });
        spawn_writer_thread(writer_task);

        JournalRecorder { tx, journal_dir }
    }

    /// Record an event to the journal. Returns the **on-disk** `event_sha256`.
    ///
    /// **Blocking send** — if the channel is full (backpressure from slow I/O),
    /// this will wait until the writer task catches up. Events are never dropped.
    ///
    /// Events are sealed (payload_sha256 + event_sha256 computed) inside this
    /// funnel, *after* the payload scrub. The returned hash is therefore the
    /// authoritative preimage for the next event's `previous_event_sha256`:
    /// callers must thread **this** value, never a hash they computed
    /// themselves before calling. (0v-C, 2026-09-12: a caller-side pre-seal ran
    /// on the un-scrubbed payload, so every chained link pointed at a hash that
    /// never reached disk and replay failed with `previous hash mismatch`.)
    ///
    /// Returns an error when the append is refused — `Closed` after shutdown,
    /// `TerminalAppended` after a terminal event. Callers must treat a refused
    /// append as a journal integrity violation (the event was NOT recorded).
    pub fn record(&self, event: RunEvent) -> Result<String, JournalRecorderError> {
        let mut event = event;
        // 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：journal 落
        // `.gsa/runs/`，是会话卷持久化面——写盘前在唯一漏斗对 payload 全部
        // 字符串值做 orz-secrets 机械脱敏（sk-shape 等结构化形态 + 占位符
        // 替换，确定性）；脱敏先于哈希封印，链与落盘内容一致。key 不落卷
        // 是两段门放开的前提不变量。
        orz_secrets::redact_json_string_values(&mut event.payload);
        // 0v-C：封印必须发生在脱敏之后——哈希描述的必须是落盘字节，
        // 而不是调用方提交前的暂存形态（否则链上链接指向不存在的哈希）。
        seal_event(&mut event)?;

        // POST-PLANA BUGFIX #1: blocking send — never silently drop.
        // The ack carries the writer's decision so refusal is observable,
        // and (0v-C) the sealed hash the next event must link to.
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .blocking_send(JournalCmd::WriteEvent {
                event: Box::new(event),
                ack: ack_tx,
            })
            .map_err(|_| JournalRecorderError::Closed)?;
        match ack_rx.blocking_recv() {
            Ok(result) => result,
            // Writer task gone without answering — no hash to thread.
            Err(_) => Err(JournalRecorderError::Closed),
        }
    }

    /// Async version of `record()` — safe to call from within a Tokio runtime.
    ///
    /// Uses `send().await` instead of `blocking_send`. Otherwise identical to
    /// `record()` — including the 0v-C contract: the returned hash is the
    /// on-disk seal (post-scrub) and is what the next event must link to.
    pub async fn record_async(&self, event: RunEvent) -> Result<String, JournalRecorderError> {
        let mut event = event;
        // 0p S2 B5：与 record 同一漏斗纪律（见上）。
        orz_secrets::redact_json_string_values(&mut event.payload);
        // 0v-C：与 record 一致——先脱敏再封印，返回落盘哈希。
        seal_event(&mut event)?;

        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .send(JournalCmd::WriteEvent {
                event: Box::new(event),
                ack: ack_tx,
            })
            .await
            .map_err(|_| JournalRecorderError::Closed)?;
        match ack_rx.await {
            Ok(result) => result,
            Err(_) => Err(JournalRecorderError::Closed),
        }
    }

    /// Flush all buffered writes to disk and fsync.
    /// Blocks until the flush is complete.
    pub fn flush(&self) -> Result<(), JournalRecorderError> {
        let (ack_tx, ack_rx) = oneshot::channel();
        self.tx
            .blocking_send(JournalCmd::Flush { ack: ack_tx })
            .map_err(|_| JournalRecorderError::Closed)?;
        ack_rx
            .blocking_recv()
            .unwrap_or(Err(JournalRecorderError::Closed))
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
        ack_rx
            .blocking_recv()
            .unwrap_or(Err(JournalRecorderError::Closed))
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

/// REV-083-10 (2026-09-27): the journal writer owns blocking file IO
/// (`flush` + `sync_all` per append). It used to run as a `tokio::spawn`ed
/// task, so a slow disk parked a runtime worker thread mid-fsync; it now
/// runs on its own named OS thread (`run_blocking`), leaving the async
/// runtime free. The command channel and oneshot acks are unchanged — a
/// single writer still serializes appends FIFO, so ordering is identical.
fn spawn_writer_thread(writer_task: JournalWriterTask) {
    std::thread::Builder::new()
        .name("orz-journal-writer".to_string())
        .spawn(move || writer_task.run_blocking())
        .expect("spawn orz-journal-writer thread");
}

struct JournalWriterTask {
    path: PathBuf,
    rx: mpsc::Receiver<JournalCmd>,
    file: Option<BufWriter<File>>,
    closed: bool,
    /// Set once a terminal event (run_finished/run_failed/...) has been
    /// written — further appends are refused to preserve chain integrity.
    terminal_seen: bool,
    /// FUS-HOST-RESOURCE-SAFETY §4.3 (0z S2): `Some` once the storage-full
    /// backoff ladder was exhausted — the journal then accepts chain-skeleton
    /// events only and the run_id's terminal shape is guaranteed one way or
    /// the other (`run_terminated`/terminal on the chain, else TERMINAL.json).
    degraded: Option<DegradedState>,
    /// Test seam: scripted write faults (see `new_with_write_faults_for_tests`).
    fault: Option<WriteFaultScript>,
}

/// Test-seam fault script: the first `failures_remaining` write attempts fail
/// with a storage-full error, later attempts succeed. `partial_bytes > 0`
/// additionally writes a torn prefix of the failing line to the file before
/// the error — the partial-write shape real ENOSPC produces (review F-C-1b).
/// `alloc_failures_remaining` (0bc ④, 2026-09-21) injects allocation failures
/// instead — the constructive injection the fallible-allocation degradation
/// contract is verified against.
struct WriteFaultScript {
    failures_remaining: u32,
    partial_bytes: usize,
    alloc_failures_remaining: u32,
}

impl WriteFaultScript {
    fn take_fault(&mut self) -> Option<std::io::Error> {
        if self.failures_remaining > 0 {
            self.failures_remaining -= 1;
            Some(std::io::Error::from_raw_os_error(112)) // ERROR_DISK_FULL
        } else {
            None
        }
    }

    /// 0bc ④: allocation failure — no backoff ladder on purpose (memory does
    /// not come back by waiting); the writer task maps it straight into the
    /// degraded chain, same shape as ENOSPC.
    fn take_alloc_fault(&mut self) -> Option<std::io::Error> {
        if self.alloc_failures_remaining > 0 {
            self.alloc_failures_remaining -= 1;
            Some(std::io::Error::new(
                std::io::ErrorKind::OutOfMemory,
                "simulated allocation failure (0bc S2 ④ test seam)",
            ))
        } else {
            None
        }
    }
}

impl JournalWriterTask {
    fn new(path: PathBuf, rx: mpsc::Receiver<JournalCmd>) -> Self {
        JournalWriterTask {
            path,
            rx,
            file: None,
            closed: false,
            terminal_seen: false,
            degraded: None,
            fault: None,
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

    /// Append one event as a JSONL line and fsync. Returns the serialized line
    /// length on success so the caller can distinguish "nothing written" from
    /// a partial append.
    fn append_event(
        file: &mut BufWriter<File>,
        event: &RunEvent,
    ) -> Result<usize, JournalRecorderError> {
        // Serialize to compact JSON (matching Python: sort_keys + compact separators)
        //
        // 0bc ④（2026-09-21，设计 §11-4）：journal 装配是 S1 清单里的巨量
        // 分配路径之一——序列化缓冲改**可失败增长**（try_reserve），分配
        // 失败返回 `ErrorKind::OutOfMemory` 而不 abort；写者任务把它映射进
        // 降级链（与 ENOSPC 同形：不 abort、留终态）。
        let mut buf = FallibleBuf::new();
        let mut ser = serde_json::Serializer::new(&mut buf);
        if let Err(err) = event.serialize(&mut ser) {
            return Err(match err.io_error_kind() {
                // ByteWriter allocation failures must surface as io errors with
                // the kind preserved so the writer task can route them into the
                // degraded chain exactly like a storage-full error.
                Some(kind) => JournalRecorderError::Io(std::io::Error::new(kind, err.to_string())),
                None => JournalRecorderError::Serde(err),
            });
        }
        buf.try_push(b'\n')?;
        file.write_all(&buf)?;
        file.flush()?;
        file.get_ref().sync_all()?;
        Ok(buf.len())
    }

    /// Repair a partially-appended line after a failed write (0z S2; review
    /// F-C-1/F-C-2, 2026-09-13): the append-only file may hold a torn tail —
    /// everything after the last complete line is dropped so the next append
    /// starts on a line boundary.
    ///
    /// Two review findings fixed here. **F-C-1**: the original 4 KiB tail
    /// window truncated the WHOLE journal to zero whenever the torn line was
    /// longer than the window (no `\n` inside the window ⇒ `keep = 0`) — the
    /// scan now walks backward in 64 KiB chunks until it finds the last
    /// newline, so `keep = 0` happens only when the file truly contains no
    /// complete line at all. **F-C-2**: a failed `flush` can leave the
    /// unwritten suffix inside the BufWriter; repairing only the file while
    /// keeping the buffer would splice the residue into the next line, so the
    /// buffered writer is discarded (self.file = None) before the repair and
    /// the next `ensure_file` opens a fresh one.
    fn repair_partial_tail(&mut self) {
        // F-C-2: drop the BufWriter with any unwritten residue.
        self.file = None;
        let mut f = match OpenOptions::new().read(true).write(true).open(&self.path) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("journal tail repair: reopen failed: {e}");
                return;
            }
        };
        let Ok(len) = f.metadata().map(|m| m.len()) else {
            return;
        };
        use std::io::{Read, Seek, SeekFrom};
        const CHUNK: u64 = 64 * 1024;
        let mut keep = 0u64; // 0 = no complete line anywhere in the file
        let mut scan_from = len;
        while scan_from > 0 {
            let read_from = scan_from.saturating_sub(CHUNK);
            let mut buf = vec![0u8; (scan_from - read_from) as usize];
            if f.seek(SeekFrom::Start(read_from)).is_err() {
                tracing::warn!("journal tail repair: seek failed; file left untouched");
                return;
            }
            if f.read_exact(&mut buf).is_err() {
                tracing::warn!("journal tail repair: tail read failed; file left untouched");
                return;
            }
            if let Some(pos) = buf.iter().rposition(|&b| b == b'\n') {
                keep = read_from + pos as u64 + 1;
                break;
            }
            scan_from = read_from;
        }
        if keep != len {
            if let Err(e) = f.set_len(keep) {
                tracing::warn!("journal tail repair: truncate to {keep} failed: {e}");
                return;
            }
            if let Err(e) = f.sync_all() {
                tracing::warn!("journal tail repair: sync failed: {e}");
            }
        }
    }

    /// The storage-full backoff ladder (design §4.3 item 2): 10 / 40 / 160 ms
    /// retries of the same append; every failed attempt is preceded by a
    /// partial-tail repair so retries always start from a clean line boundary.
    fn append_with_backoff(&mut self, event: &RunEvent) -> Result<usize, std::io::Error> {
        let mut attempt = 0;
        loop {
            // 0bc ④ (design §11 item 4): an allocation failure is returned
            // immediately — no retry ladder (memory does not come back by
            // waiting); the run() match arm routes it into the degraded chain,
            // the same shape as an exhausted ENOSPC ladder.
            if let Some(script) = self.fault.as_mut()
                && let Some(e) = script.take_alloc_fault()
            {
                return Err(e);
            }
            if let Some(script) = self.fault.as_mut()
                && let Some(e) = script.take_fault()
            {
                // Test seam realism (review F-C-1b): a scripted fault with
                // `partial_bytes > 0` first writes a torn prefix of THIS line
                // to the file, then fails — the "torn write longer than any
                // fixed window" shape the repair must survive. Zero partial
                // bytes keeps the historical clean-failure behavior.
                let partial = script.partial_bytes;
                if partial > 0 {
                    let mut buf = Vec::new();
                    let mut ser = serde_json::Serializer::new(&mut buf);
                    let _ = event.serialize(&mut ser);
                    buf.push(b'\n');
                    let cut = partial.clamp(1, buf.len().saturating_sub(1));
                    if let Ok(mut f) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.path)
                    {
                        use std::io::Write;
                        let _ = f.write_all(&buf[..cut]);
                        let _ = f.sync_all();
                    }
                }
                self.repair_partial_tail();
                attempt += 1;
                if attempt > 3 {
                    return Err(e);
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            let write_result = match self.ensure_file() {
                Ok(file) => Self::append_event(file, event).map_err(|e| match e {
                    JournalRecorderError::Io(io) => io,
                    other => std::io::Error::other(other.to_string()),
                }),
                Err(e) => Err(e),
            };
            match write_result {
                Ok(len) => return Ok(len),
                Err(e) if is_storage_full_error(&e) => {
                    self.repair_partial_tail();
                    attempt += 1;
                    if attempt > 3 {
                        return Err(e);
                    }
                    // 10 / 40 / 160 ms — the design's ladder.
                    std::thread::sleep(std::time::Duration::from_millis(
                        10 * 4u64.pow(attempt - 1),
                    ));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Enter degraded mode after the ladder is exhausted (design §4.3 item 3).
    fn enter_degraded(&mut self, dropped_sequence: u64, cause: String) {
        if self.degraded.is_some() {
            return;
        }
        tracing::warn!(
            sequence = dropped_sequence,
            "journal degraded mode entered: storage-full backoff ladder exhausted; \
             chain-skeleton events only (FUS-HOST-RESOURCE-SAFETY §4.3)"
        );
        self.degraded = Some(DegradedState {
            entered_at: chrono_now_compact(),
            cause,
            dropped_events: 1,
            first_dropped_sequence: Some(dropped_sequence),
            reserved_bytes: journal_reserved_bytes(),
        });
    }

    /// Degraded-mode terminal append: inject the degraded summary into the
    /// payload, truncate overlong string values so the serialized line fits
    /// the 8 KiB bound (design §4.3 item 3), re-seal, and append. The summary
    /// marker is the on-disk evidence the `degraded_complete` classification
    /// keys on; the original seal travels inside it as the pointer.
    fn prepare_degraded_terminal(&self, event: &mut RunEvent) {
        let Some(state) = self.degraded.as_ref() else {
            return;
        };
        if event.payload.get("degraded").is_none() {
            event.payload["degraded"] = state.to_summary_json();
        }
        // RS-05 (0aq, 2026-09-19, Top-10 #4)：degraded 最坏时点的再盖章
        // 失败不得 panic 写者线程——留显式错误日志，事件保持未盖章落盘
        //（degraded 分类对未盖章行宽容，收尾链继续）。
        if let Err(e) = seal_event(event) {
            tracing::error!(error = %e, "degraded re-seal failed after summary injection");
        }
        // Bound the line: truncate the longest string values until the
        // serialized event fits. Mechanical, shape-preserving — the row stays
        // schema-valid and the chain stays replayable.
        for _ in 0..16 {
            // 0bc ④: same fallible-growth buffer as the main assembly path.
            let mut buf = FallibleBuf::new();
            let mut ser = serde_json::Serializer::new(&mut buf);
            if event.serialize(&mut ser).is_err() {
                break;
            }
            if buf.len() < DEGRADED_EVENT_BOUND_BYTES {
                break;
            }
            truncate_longest_string(&mut event.payload);
            if let Err(e) = seal_event(event) {
                tracing::error!(error = %e, "degraded re-seal failed after truncation");
            }
        }
    }

    fn run_blocking(mut self) {
        use JournalCmd::*;
        while let Some(cmd) = self.rx.blocking_recv() {
            match cmd {
                WriteEvent { mut event, ack } => {
                    if self.closed {
                        let _ = ack.send(Err(JournalRecorderError::Closed));
                        continue;
                    }
                    // Refuse appends after a terminal event (hash-chain invariant).
                    // The caller must observe the refusal via the ack — a
                    // dropped event with Ok would break the caller's seq/hash
                    // bookkeeping invisibly.
                    if self.terminal_seen {
                        tracing::warn!(
                            "journal append refused after terminal event (seq {})",
                            event.sequence
                        );
                        let _ =
                            ack.send(Err(JournalRecorderError::TerminalAppended(event.sequence)));
                        continue;
                    }
                    // Degraded skeleton filter (design §4.3 item 3): only
                    // chain-skeleton-necessary events land — terminal shapes.
                    // Everything else (mechanical_audit, snapshots, ledger
                    // folds, big payloads…) is regenerable face and is refused
                    // WITHOUT consuming a sequence number (the caller must not
                    // advance its bookkeeping — the `DegradedDropped` ack).
                    if let Some(state) = self.degraded.as_mut()
                        && !event.is_terminal()
                    {
                        state.dropped_events += 1;
                        if state.first_dropped_sequence.is_none() {
                            state.first_dropped_sequence = Some(event.sequence);
                        }
                        let _ = ack.send(Err(JournalRecorderError::DegradedDropped {
                            sequence: event.sequence,
                            event_type: event.event_type.to_string(),
                        }));
                        continue;
                    }
                    if self.degraded.is_some() {
                        self.prepare_degraded_terminal(&mut event);
                    }
                    match self.append_with_backoff(&event) {
                        Ok(_) => {
                            if event.is_terminal() {
                                self.terminal_seen = true;
                            }
                            // 0v-C: report the sealed hash the caller must
                            // thread into the next event's `previous` link.
                            // Degraded rewrites re-seal inside the writer, so
                            // the returned hash is still the on-disk seal.
                            let _ = ack.send(Ok(event.event_sha256.clone()));
                        }
                        Err(e) if is_storage_full_error(&e) || is_alloc_failure(&e) => {
                            // The ladder is exhausted (ENOSPC) or the journal
                            // assembly hit an allocation failure (0bc ④): enter
                            // degraded mode and refuse THIS event (the caller
                            // keeps its chain bookkeeping). The journal stays
                            // open — space may return, and the terminal shape
                            // is still owed.
                            let seq = event.sequence;
                            let was_terminal = event.is_terminal();
                            let event_type = event.event_type.to_string();
                            self.enter_degraded(seq, format!("{e}"));
                            if was_terminal {
                                // 判据 6: a terminal event that cannot land on
                                // the chain still leaves an enumerable terminal
                                // shape — the TERMINAL.json sidecar + stderr.
                                let journal_dir = self
                                    .path
                                    .parent()
                                    .map(Path::to_path_buf)
                                    .unwrap_or_else(|| PathBuf::from("."));
                                write_terminal_sidecar(
                                    &journal_dir,
                                    &event.run_id,
                                    self.degraded.as_ref(),
                                    seq,
                                    &event_type,
                                    &e.to_string(),
                                );
                            }
                            let _ = ack.send(Err(JournalRecorderError::DegradedDropped {
                                sequence: seq,
                                event_type,
                            }));
                        }
                        Err(e) => {
                            tracing::error!("journal write error: {e}");
                            self.closed = true;
                            let _ = ack.send(Err(JournalRecorderError::Io(e)));
                        }
                    }
                }
                Flush { ack } => {
                    let result = match self.file.as_mut() {
                        Some(file) => file
                            .flush()
                            .and_then(|_| file.get_ref().sync_all())
                            .map_err(JournalRecorderError::from),
                        None => Ok(()),
                    };
                    let _ = ack.send(result);
                }
                Shutdown { ack } => {
                    self.closed = true;
                    let result = match self.file.as_mut() {
                        Some(file) => file
                            .flush()
                            .and_then(|_| file.get_ref().sync_all())
                            .map_err(JournalRecorderError::from),
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

/// Truncate the longest string value in the payload (recursively) to a bounded
/// head + pointer marker. Shape-preserving: keys, nesting and value kinds are
/// untouched, so the row stays schema-valid.
fn truncate_longest_string(value: &mut serde_json::Value) {
    use serde_json::Value;
    const HEAD: usize = 256;
    const MARKER: &str = "\u{2026}[degraded-truncated]";
    match value {
        Value::Object(map) => {
            if let Some((_, longest)) = map
                .iter_mut()
                .filter(|(_, v)| {
                    v.as_str()
                        .is_some_and(|s| s.chars().count() > HEAD && !s.ends_with(MARKER))
                })
                .max_by_key(|(_, v)| v.as_str().map(|s| s.len()).unwrap_or(0))
            {
                if let Value::String(s) = longest {
                    let head: String = s.chars().take(HEAD).collect();
                    *s = format!("{head}…[degraded-truncated]");
                }
                return;
            }
            for (_, v) in map.iter_mut() {
                truncate_longest_string(v);
            }
        }
        Value::Array(items) => {
            for item in items {
                truncate_longest_string(item);
            }
        }
        _ => {}
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
        let dir =
            std::env::temp_dir().join(format!("orz-journal-test-{}-{}", std::process::id(), n));
        // Ensure clean start
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_event(
        run_id: &str,
        seq: u64,
        event_type: EventType,
        previous: Option<String>,
    ) -> RunEvent {
        RunEvent::new_v01(
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

    /// 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：journal 落
    /// `.gsa/runs/`（会话卷持久化面）——record 漏斗对 payload 字符串值做
    /// orz-secrets 机械脱敏（sk-shape → 占位符），脱敏先于哈希封印。
    #[test]
    fn record_scrubs_secret_shaped_payload_strings() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        let mut e = make_event("RUN-SCRUB", 0, EventType::ToolCompleted, None);
        let secret = "sk-abcdefghijklmnopqrstuvwxyz012345";
        e.payload = serde_json::json!({
            "tool": "run_terminal_cmd",
            "call_id": "call-scrub-1",
            "exit_code": 0,
            "output": format!("pip install --api-key {secret} done"),
        });
        recorder.record(e).unwrap();
        recorder.shutdown().unwrap();

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert!(
            !content.contains(secret),
            "secret-shaped strings must not reach the journal: {content}"
        );
        assert!(content.contains("[REDACTED_SECRET]"), "{content}");
    }

    #[test]
    fn record_and_verify_chain() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        // Caller-owned chain: seal each event and thread the previous hash
        // through — the controller's EventWriter does the same. The recorder
        // seals payload/event hashes but does NOT maintain the chain.
        let mut e0 = make_event("RUN-CHAIN", 0, EventType::RunStarted, None);
        seal_event(&mut e0).unwrap();
        let prev0 = Some(e0.event_sha256.clone());
        recorder.record(e0).unwrap();

        let mut e1 = make_event("RUN-CHAIN", 1, EventType::PromptSubmitted, prev0);
        seal_event(&mut e1).unwrap();
        let prev1 = Some(e1.event_sha256.clone());
        recorder.record(e1).unwrap();

        let mut e2 = make_event("RUN-CHAIN", 2, EventType::RunFinished, prev1);
        seal_event(&mut e2).unwrap();
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

        // The chain must replay valid — a placeholder previous hash would
        // fail here (see 2026-08-04 review P0-1).
        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-CHAIN"), None, true);
        assert!(replay.valid, "chain broken: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0v-C（2026-09-12）：`record()` 返回的是漏斗内、脱敏**之后**的封印
    /// 哈希。调用方必须在提交后才拿到它——修复前 controller 自己预先封印，
    /// 得到的是未改写 payload 的哈希，与盘上内容永远对不上。
    #[test]
    fn record_returns_post_funnel_sealed_hash() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        // URL 归一化是确定性漏斗改写：url crate 给空路径补 "/"，于是落盘
        // 形态必然不同于调用方提交的形态（等价于线上 0v-C 触发条件）。
        let payload = serde_json::json!({
            "tool": "web_fetch",
            "output": "see https://example.com?q=1 for details",
        });

        // 调用方“自作聪明”的预封印（= 修复前的 controller 行为）。
        let mut prescal = make_event("RUN-HASH", 0, EventType::ToolCompleted, None);
        prescal.payload = payload.clone();
        seal_event(&mut prescal).unwrap();

        let mut e = make_event("RUN-HASH", 0, EventType::ToolCompleted, None);
        e.payload = payload;
        let returned = recorder.record(e).unwrap();
        recorder.shutdown().unwrap();

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let on_disk: RunEvent = serde_json::from_str(content.lines().next().unwrap()).unwrap();

        assert_eq!(
            returned, on_disk.event_sha256,
            "record() must report the on-disk seal"
        );
        assert_ne!(
            returned, prescal.event_sha256,
            "test premise: the funnel must rewrite this payload (URL normalization)"
        );
        assert!(
            !content.contains("https://example.com?q=1"),
            "funnel rewrite must be visible on disk: {content}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0v-C（2026-09-12）：链按 `record()` 返回的哈希串联时，重放必须通过。
    /// 这正是线上 pipeline 上 `previous hash mismatch` 的最小反例。
    #[test]
    fn chain_threads_returned_hash_and_replays_valid_after_funnel_rewrite() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        let mut e0 = make_event("RUN-URLCHAIN", 0, EventType::RunStarted, None);
        e0.payload = serde_json::json!({
            "cwd": "D:/CLI/orz",
            "note": "seed https://example.com?q=1",
        });
        // 修复后：链上链接 = record() 返回值（落盘哈希）。
        let h0 = recorder.record(e0).unwrap();

        let e1 = make_event("RUN-URLCHAIN", 1, EventType::RunFinished, Some(h0));
        recorder.record(e1).unwrap();
        recorder.shutdown().unwrap();

        let events_path = dir.join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let rows: Vec<RunEvent> = content
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            rows[1].previous_event_sha256.as_deref(),
            Some(rows[0].event_sha256.as_str()),
            "link must point at the on-disk hash"
        );

        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-URLCHAIN"), None, true);
        assert!(
            replay.valid,
            "chain broken after funnel rewrite: {:?}",
            replay.errors
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_refused_after_terminal_event() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        let mut terminal = make_event("RUN-TERM", 0, EventType::RunFinished, None);
        seal_event(&mut terminal).unwrap();
        recorder.record(terminal).unwrap();

        // Any further append must be refused with an explicit error — never
        // a silent drop that returns Ok (2026-08-04 review P0-2).
        let mut late = make_event("RUN-TERM", 1, EventType::PromptSubmitted, None);
        seal_event(&mut late).unwrap();
        let err = recorder.record(late).unwrap_err();
        assert!(
            matches!(err, JournalRecorderError::TerminalAppended(1)),
            "expected TerminalAppended, got {err:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn append_refused_after_terminal_event_async() {
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        let mut terminal = make_event("RUN-TERM-A", 0, EventType::RunFinished, None);
        seal_event(&mut terminal).unwrap();
        recorder.record_async(terminal).await.unwrap();

        let mut late = make_event("RUN-TERM-A", 1, EventType::RunStarted, None);
        seal_event(&mut late).unwrap();
        let err = recorder.record_async(late).await.unwrap_err();
        assert!(
            matches!(err, JournalRecorderError::TerminalAppended(1)),
            "expected TerminalAppended, got {err:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn blocking_send_does_not_drop_events() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _guard = rt.enter();
        let dir = temp_dir();
        let recorder = JournalRecorder::new(dir.clone());

        // Rapid-fire 50 events — none should be dropped. Chain links are the
        // caller's job (this test only asserts zero-loss delivery, so `None`
        // previous hashes are fine here).
        for i in 0..50 {
            let event = make_event(
                "RUN-BLOCK",
                i as u64,
                if i == 49 {
                    EventType::RunFinished
                } else {
                    EventType::PromptSubmitted
                },
                None,
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

        r1.record(make_event("RUN-CLONE", 0, EventType::RunStarted, None))
            .unwrap();
        r2.record(make_event("RUN-CLONE", 1, EventType::RunFinished, None))
            .unwrap();

        r2.shutdown().unwrap(); // shutdown via clone

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        assert_eq!(content.lines().count(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 0z S2 §4.3: 状态链抗饿死（ENOSPC 三退 → Degraded → 显式收尾）────

    /// EventWriter's degraded contract, replayed at the recorder level: a
    /// `DegradedDropped` ack means the event was NOT written and the caller
    /// must keep its sequence/hash bookkeeping — the on-disk chain stays
    /// contiguous because dropped events never consume a sequence number.
    struct ChainBookkeeping {
        seq: u64,
        prev: Option<String>,
    }

    impl ChainBookkeeping {
        fn new() -> Self {
            Self { seq: 0, prev: None }
        }

        async fn record(
            &mut self,
            recorder: &JournalRecorder,
            event_type: EventType,
            payload: serde_json::Value,
        ) -> Result<(), JournalRecorderError> {
            let event = RunEvent::new_v01(
                "RUN-DEGRADED".into(),
                self.seq,
                event_type,
                "test-manifest-sha256-64chars-long___________________".into(),
                self.prev.clone(),
                "test-schema".into(),
                payload,
                Redaction::None,
                "2026-09-12T00:00:00Z".into(),
            );
            match recorder.record_async(event).await {
                Ok(hash) => {
                    self.prev = Some(hash);
                    self.seq += 1;
                    Ok(())
                }
                Err(JournalRecorderError::DegradedDropped { .. }) => Ok(()),
                Err(e) => Err(e),
            }
        }
    }

    /// Design §4.3 items 2-3: the ladder (initial + 3 retries) is exhausted →
    /// degraded mode → regenerable events are refused without consuming a
    /// sequence number → the terminal lands WITH the degraded summary marker
    /// → the chain replays valid. 判据 6's `run_terminated`/terminal leg.
    #[tokio::test]
    async fn storage_full_ladder_enters_degraded_and_chain_stays_replayable() {
        let dir = temp_dir();
        // The first event's four attempts (initial + 10/40/160 ms retries)
        // all fail; later attempts succeed (transient ENOSPC that recovered).
        let recorder = JournalRecorder::new_with_write_faults_for_tests(dir.clone(), 4);

        let mut chain = ChainBookkeeping::new();
        chain
            .record(
                &recorder,
                EventType::RunStarted,
                serde_json::json!({"i": 0}),
            )
            .await
            .expect("degraded drop is not an error at the writer level");
        chain
            .record(
                &recorder,
                EventType::ModelOutput,
                serde_json::json!({"i": 1, "big": "regenerable face"}),
            )
            .await
            .expect("dropped regenerable event");
        chain
            .record(
                &recorder,
                EventType::RunFinished,
                serde_json::json!({"status": "completed"}),
            )
            .await
            .expect("terminal lands in degraded mode");
        recorder.shutdown_async().await.unwrap();

        let events_path = dir.join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let rows: Vec<serde_json::Value> = content
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        // run_started (seq 0) and model_output were dropped; run_finished is
        // the journal's FIRST line and carries sequence 0 — the caller's
        // bookkeeping never advanced for the dropped events.
        assert_eq!(rows.len(), 1, "only the terminal landed: {content}");
        assert_eq!(rows[0]["sequence"], serde_json::json!(0));
        assert_eq!(rows[0]["event_type"], serde_json::json!("run_finished"));
        let degraded = rows[0]["payload"]["degraded"]
            .as_object()
            .expect("degraded summary marker injected by the writer");
        assert_eq!(
            degraded["dropped_events"],
            serde_json::json!(2),
            "run_started + model_output were dropped while degraded"
        );
        assert_eq!(degraded["first_dropped_sequence"], serde_json::json!(0));

        // The skeleton replays: contiguous sequences, unbroken hash chain,
        // exactly one terminal.
        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-DEGRADED"), None, true);
        assert!(
            replay.valid,
            "degraded skeleton must replay: {:?}",
            replay.errors
        );

        // No sidecar — the terminal landed on the chain.
        assert!(!dir.join("TERMINAL.json").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0bc ④ (design §11 item 4, 2026-09-21): a **constructive allocation
    /// failure** on the journal assembly path enters the same degraded chain
    /// as an exhausted ENOSPC ladder — no retry ladder, no abort; the failed
    /// event is refused without consuming a sequence number, the terminal
    /// still lands with the degraded summary marker, and the chain replays
    /// valid. 判据 4's constructive-injection leg.
    #[tokio::test]
    async fn alloc_failure_enters_degraded_and_chain_stays_replayable() {
        let dir = temp_dir();
        // The first append attempt fails with OutOfMemory — nothing absorbs
        // it (memory does not come back by waiting).
        let recorder = JournalRecorder::new_with_alloc_faults_for_tests(dir.clone(), 1);

        let mut chain = ChainBookkeeping::new();
        chain
            .record(
                &recorder,
                EventType::RunStarted,
                serde_json::json!({"i": 0}),
            )
            .await
            .expect("degraded drop is not an error at the writer level");
        chain
            .record(
                &recorder,
                EventType::ModelOutput,
                serde_json::json!({"i": 1, "big": "regenerable face"}),
            )
            .await
            .expect("dropped regenerable event");
        chain
            .record(
                &recorder,
                EventType::RunFinished,
                serde_json::json!({"status": "completed"}),
            )
            .await
            .expect("terminal lands in degraded mode");
        recorder.shutdown_async().await.unwrap();

        let events_path = dir.join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let rows: Vec<serde_json::Value> = content
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 1, "only the terminal landed: {content}");
        assert_eq!(rows[0]["sequence"], serde_json::json!(0));
        assert_eq!(rows[0]["event_type"], serde_json::json!("run_finished"));
        let degraded = rows[0]["payload"]["degraded"]
            .as_object()
            .expect("degraded summary marker injected by the writer");
        assert_eq!(
            degraded["dropped_events"],
            serde_json::json!(2),
            "run_started + model_output were dropped while degraded"
        );
        assert_eq!(degraded["first_dropped_sequence"], serde_json::json!(0));
        assert!(
            degraded["cause"]
                .as_str()
                .unwrap_or_default()
                .contains("allocation failure"),
            "the cause names the allocation failure: {}",
            degraded["cause"]
        );

        // The skeleton replays: contiguous sequences, unbroken hash chain,
        // exactly one terminal.
        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-DEGRADED"), None, true);
        assert!(
            replay.valid,
            "degraded skeleton must replay: {:?}",
            replay.errors
        );

        // No sidecar — the terminal landed on the chain.
        assert!(!dir.join("TERMINAL.json").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 判据 6's TERMINAL.json leg: when even the terminal event cannot land
    /// (persistent ENOSPC), the sidecar is the enumerable terminal shape.
    #[tokio::test]
    async fn persistent_enospc_terminal_falls_back_to_sidecar() {
        let dir = temp_dir();
        // Fail far more attempts than any ladder can absorb.
        let recorder = JournalRecorder::new_with_write_faults_for_tests(dir.clone(), 100);

        let mut chain = ChainBookkeeping::new();
        chain
            .record(
                &recorder,
                EventType::RunStarted,
                serde_json::json!({"i": 0}),
            )
            .await
            .expect("degraded drop");
        chain
            .record(
                &recorder,
                EventType::RunTerminated,
                serde_json::json!({"reason": "journal_degraded"}),
            )
            .await
            .expect("terminal refused via the degraded contract, sidecar written");
        recorder.shutdown_async().await.unwrap();

        // Nothing landed on the chain, but the terminal shape exists.
        let content = std::fs::read_to_string(dir.join("events.jsonl"));
        assert!(
            content.as_deref().map(str::is_empty).unwrap_or(true),
            "no event may have landed: {content:?}"
        );
        let sidecar = std::fs::read_to_string(dir.join("TERMINAL.json")).expect("sidecar exists");
        let sidecar: serde_json::Value = serde_json::from_str(&sidecar).unwrap();
        assert_eq!(sidecar["reason"], serde_json::json!("journal_degraded"));
        assert_eq!(
            sidecar["planned_event_type"],
            serde_json::json!("run_terminated")
        );
        assert!(
            sidecar["degraded"].is_object(),
            "sidecar carries the degraded summary"
        );
        let size = std::fs::metadata(dir.join("TERMINAL.json")).unwrap().len();
        assert!(size < 4096, "the sidecar is bounded (< 4 KiB), got {size}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A transient ENOSPC that recovers inside the ladder never degrades the
    /// journal: the event lands on the retry and the chain is untouched.
    #[tokio::test]
    async fn transient_enospc_recovers_within_the_ladder() {
        let dir = temp_dir();
        // One failed attempt: the initial try of the first event; the first
        // retry succeeds.
        let recorder = JournalRecorder::new_with_write_faults_for_tests(dir.clone(), 1);

        let mut chain = ChainBookkeeping::new();
        chain
            .record(
                &recorder,
                EventType::RunStarted,
                serde_json::json!({"i": 0}),
            )
            .await
            .expect("retry succeeds");
        chain
            .record(
                &recorder,
                EventType::RunFinished,
                serde_json::json!({"status": "completed"}),
            )
            .await
            .expect("normal terminal");
        recorder.shutdown_async().await.unwrap();

        let events_path = dir.join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let rows: Vec<serde_json::Value> = content
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 2, "both events landed: {content}");
        assert!(
            rows.iter().all(|r| r["payload"].get("degraded").is_none()),
            "no degraded marker may exist on a non-degraded journal: {content}"
        );
        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-DEGRADED"), None, true);
        assert!(replay.valid, "chain broken: {:?}", replay.errors);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Review F-C-1/F-C-2 end-to-end (2026-09-13): a torn write LONGER than
    /// any fixed window (an 80 KiB event half-written by ENOSPC), followed by
    /// recovery, must never truncate earlier events nor splice the residue
    /// into the next line. The historical 4 KiB-window repair failed both.
    #[tokio::test]
    async fn torn_long_write_is_repaired_without_losing_earlier_events() {
        let dir = temp_dir();
        // Fault 1: clean failure of event A's first attempt (its retry
        // succeeds and lands event A). Faults 2: a TORN 80_000-byte prefix of
        // event B's first attempt, then failure; B's retry succeeds. The torn
        // prefix contains no newline, so the old window scan saw "no line" and
        // truncated the whole file; the residue in the buffer (F-C-2) would
        // have spliced into B's retry line.
        let recorder = JournalRecorder::new_with_torn_write_faults_for_tests(dir.clone(), 1, 0);
        let big = "x".repeat(80_000);
        let e0 = RunEvent::new_v01(
            "RUN-TORN".into(),
            0,
            EventType::RunStarted,
            "test-manifest-sha256-64chars-long___________________".into(),
            None,
            "test-schema".into(),
            serde_json::json!({"i": 0, "pad": big}),
            Redaction::None,
            "2026-09-13T00:00:00Z".into(),
        );
        let h0 = recorder.record_async(e0).await.expect("event A lands");
        recorder.shutdown_async().await.unwrap();
        drop(recorder);

        let recorder =
            JournalRecorder::new_with_torn_write_faults_for_tests(dir.clone(), 1, 80_000);
        let e1 = RunEvent::new_v01(
            "RUN-TORN".into(),
            1,
            EventType::RunFinished,
            "test-manifest-sha256-64chars-long___________________".into(),
            Some(h0),
            "test-schema".into(),
            serde_json::json!({"status": "completed"}),
            Redaction::None,
            "2026-09-13T00:00:01Z".into(),
        );
        recorder
            .record_async(e1)
            .await
            .expect("terminal lands after torn write");
        recorder.shutdown_async().await.unwrap();

        let events_path = dir.join("events.jsonl");
        let content = std::fs::read_to_string(&events_path).unwrap();
        let rows: Vec<serde_json::Value> =
            content
                .lines()
                .map(|line| {
                    serde_json::from_str(line).unwrap_or_else(|e| {
                panic!("every line must be valid JSON (F-C-2 residue check): {e} in {content:?}")
            })
                })
                .collect();
        assert_eq!(
            rows.len(),
            2,
            "F-C-1: the torn write must not have truncated earlier events: {content:?}"
        );
        assert_eq!(rows[0]["event_type"], serde_json::json!("run_started"));
        assert_eq!(rows[1]["event_type"], serde_json::json!("run_finished"));
        assert_eq!(rows[1]["sequence"], serde_json::json!(1));

        let replay =
            super::super::verifier::replay_journal(&events_path, Some("RUN-TORN"), None, true);
        assert!(replay.valid, "chain must replay: {:?}", replay.errors);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The degraded summary injection re-seals the row — the returned hash is
    /// the on-disk seal (the 0v-C contract survives the S2 rewrite path).
    #[tokio::test]
    async fn degraded_terminal_ack_returns_the_ondisk_seal() {
        let dir = temp_dir();
        let recorder = JournalRecorder::new_with_write_faults_for_tests(dir.clone(), 4);

        // Consume the fault script: the non-terminal event exhausts the ladder
        // and enters degraded mode (its ack is a DegradedDropped).
        let dropped = RunEvent::new_v01(
            "RUN-SEAL".into(),
            0,
            EventType::RunStarted,
            "test-manifest-sha256-64chars-long___________________".into(),
            None,
            "test-schema".into(),
            serde_json::json!({"i": 0}),
            Redaction::None,
            "2026-09-12T00:00:00Z".into(),
        );
        assert!(matches!(
            recorder.record_async(dropped).await,
            Err(JournalRecorderError::DegradedDropped { .. })
        ));

        let event = RunEvent::new_v01(
            "RUN-SEAL".into(),
            0,
            EventType::RunFinished,
            "test-manifest-sha256-64chars-long___________________".into(),
            None,
            "test-schema".into(),
            serde_json::json!({"status": "completed"}),
            Redaction::None,
            "2026-09-12T00:00:00Z".into(),
        );
        let acked = recorder.record_async(event).await.expect("terminal lands");
        recorder.shutdown_async().await.unwrap();

        let content = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let on_disk: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(
            acked,
            on_disk["event_sha256"].as_str().unwrap(),
            "the acked hash must be the on-disk seal (post-degraded-rewrite)"
        );
        assert!(
            on_disk["payload"]["degraded"].is_object(),
            "the rewritten row carries the degraded summary"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
