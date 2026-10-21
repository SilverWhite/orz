//! ACP-over-WebSocket bridge with per-workspace agent keepalive.
//!
//! 329 批（2026-10-21，用户真机问题「一切换会话/刷新就中断，必须刷新后才能
//! 恢复并继续」；审计 [`328`](../../../docs/audits/328_0_9_7_FIVE_ISSUE_REVIEW_AND_54D12BCC_RECOVERY_2026-10-21.md) §6.4）：
//! 旧形态是「一 WS 对一 `orz --stdio` child，任一侧断开即杀子进程」——页面
//! 刷新、会话视图切换、网络抖动都会把正在跑的 run 杀掉；327 只修了「断后
//! 从 journal 恢复」，没有修「不断」。
//!
//! 新形态：**桥持有 child，WS 只是可换的投影面**。
//! - 每个工作区（cwd display 串）一个 `AgentHandle`（child + stdin + 读线程）；
//! - WS 关闭 = detach：不杀 child，继续把 agent stdout 收进有界 backlog
//!   （`MAX_BACKLOG_BYTES`，溢出丢最旧并置 truncated 标记）；
//! - 同工作区重连 = attach：先发 `_orz/bridge_attached` 前置帧（session_id／
//!   prompt_in_flight／grace），再补发 backlog，然后继续双向转发；
//! - 已 attach 时第二个 WS 仍按原语义 409（避免双 sink 抢写）；
//! - 断线超过 grace（`ORZ_WEB_KEEPALIVE_SECS`，默认 1800s；0＝旧行为立即
//!   回收）后由 reaper 杀 child 并移除槽；
//! - child 自己退出时：向已 attach 的 WS 发 `_orz/bridge_child_exited` 并
//!   关闭通道，槽标记 dead，等 reaper 清理；重连会 spawn 新 child（客户端
//!   随后自行 `session/load`）。
//!
//! 帧仍是按行分隔的 ACP JSON-RPC；桥只做转发与两处最小观察（跟踪
//! `session/new|load` 的 sessionId、`session/prompt` 是否在途），不执行、
//! 不改写 agent 语义（ADR-0010 §2.4 条 8 / §2.6 / §14.78 条 5 不变）。

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::{Mutex, mpsc};

/// Cap for a single agent stdout line (one JSON-RPC frame). The agent is
/// our own binary, but the cap exists so a runaway writer can never grow
/// bridge memory without bound — overflow marks the slot dead like any
/// other stream error.
const MAX_ACP_LINE_BYTES: usize = 64 * 1024 * 1024;

/// Detached backlog cap (bytes of JSON-RPC text). While no WS is attached the
/// run keeps emitting `session/update` frames; on overflow the oldest frames
/// are dropped (the truthful history is the journal tail, not this buffer) and
/// the next attach is told so it can reconcile.
const MAX_BACKLOG_BYTES: usize = 8 * 1024 * 1024;

/// Reaper cadence for detached slots.
const REAPER_TICK: Duration = Duration::from_secs(15);

/// Default keepalive grace: a detached workspace child survives this long
/// before the reaper reclaims it. 0 disables keepalive (old kill-on-detach
/// behaviour).
pub const DEFAULT_KEEPALIVE_SECS: u64 = 30 * 60;

/// Keepalive grace override (seconds; `0` = kill on detach). Hard-capped at
/// 6h so a typo cannot pin a child for days.
pub const KEEPALIVE_ENV: &str = "ORZ_WEB_KEEPALIVE_SECS";

/// Read one `\n`-terminated line into `buf` (newline excluded), bounding
/// memory at `cap` bytes. `Ok(None)` = EOF; an unterminated tail is
/// withheld (an incomplete frame is never forwarded).
async fn read_line_bounded<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    cap: usize,
) -> std::io::Result<Option<()>> {
    buf.clear();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            // EOF: withhold any unterminated tail — an incomplete frame is
            // never forwarded.
            buf.clear();
            return Ok(None);
        }
        match available.iter().position(|&b| b == b'\n') {
            Some(pos) => {
                if buf.len() + pos > cap {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "ACP line exceeds cap",
                    ));
                }
                buf.extend_from_slice(&available[..pos]);
                reader.consume(pos + 1);
                return Ok(Some(()));
            }
            None => {
                if buf.len() + available.len() > cap {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "ACP line exceeds cap",
                    ));
                }
                buf.extend_from_slice(available);
                let n = available.len();
                reader.consume(n);
            }
        }
    }
}

/// Parse the keepalive grace from a raw env value (pure, testable).
pub fn parse_keepalive(raw: Option<&str>) -> Duration {
    let default = Duration::from_secs(DEFAULT_KEEPALIVE_SECS);
    let Some(raw) = raw else { return default };
    match raw.trim().parse::<u64>() {
        Ok(0) => Duration::ZERO,
        Ok(secs) => Duration::from_secs(secs.min(6 * 3600)),
        Err(_) => default,
    }
}

/// Keepalive grace from the process environment.
pub fn keepalive_grace_from_env() -> Duration {
    parse_keepalive(std::env::var(KEEPALIVE_ENV).ok().as_deref())
}

/// Per-workspace agent child plus its detachable output sink.
struct SlotState {
    /// Current WS output sink; `None` = detached (frames go to `backlog`).
    sink: Option<mpsc::UnboundedSender<String>>,
    /// Frames emitted while detached (bounded by `MAX_BACKLOG_BYTES`).
    backlog: VecDeque<String>,
    backlog_bytes: usize,
    /// Set when a backlog overflow dropped frames (told to the next attach).
    truncated: bool,
    /// When the last attach went away (grace reaper reads this).
    detached_at: Option<Instant>,
    /// Learned from the `session/new` / `session/load` response — lets a
    /// refreshed page re-bind to the still-running session without a reload.
    session_id: Option<String>,
    /// In-flight `session/prompt` request id (reconnect preamble fact).
    prompt_in_flight: Option<u64>,
    /// Child exited / stream broke; the slot is waiting for the reaper.
    dead: bool,
}

impl Default for SlotState {
    fn default() -> Self {
        Self {
            sink: None,
            backlog: VecDeque::new(),
            backlog_bytes: 0,
            truncated: false,
            detached_at: None,
            session_id: None,
            prompt_in_flight: None,
            dead: false,
        }
    }
}

impl SlotState {
    /// Buffer one frame while detached; drop the oldest frames on overflow.
    fn push_backlog(&mut self, line: String) {
        let bytes = line.len();
        if bytes > MAX_BACKLOG_BYTES {
            // A single oversized frame can never fit — drop it, but say so.
            self.truncated = true;
            return;
        }
        while self.backlog_bytes + bytes > MAX_BACKLOG_BYTES {
            let Some(dropped) = self.backlog.pop_front() else {
                break;
            };
            self.backlog_bytes = self.backlog_bytes.saturating_sub(dropped.len());
            self.truncated = true;
        }
        self.backlog_bytes += bytes;
        self.backlog.push_back(line);
    }
}

/// One workspace's agent child. Cloned as `Arc`; the reader task and the
/// bridge both hold it.
struct AgentHandle {
    key: String,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    state: Mutex<SlotState>,
    /// Client→agent request id → method (only `session/*` and `_orz/*`), used
    /// to recognise `session/new`/`session/load` responses and prompt state.
    requests: Mutex<HashMap<u64, String>>,
}

impl AgentHandle {
    /// Observe one agent→client frame: learn session id / prompt completion.
    async fn observe_agent_frame(&self, text: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let id = value.get("id").and_then(serde_json::Value::as_u64);
        // Agent→client requests (permission etc.) also carry id+method; track
        // them like client requests so their responses are consumed quietly.
        if let (Some(id), Some(method)) =
            (id, value.get("method").and_then(serde_json::Value::as_str))
        {
            if tracked_method(method) {
                self.requests.lock().await.insert(id, method.to_string());
                if method == "session/prompt" {
                    self.state.lock().await.prompt_in_flight = Some(id);
                }
            }
            return;
        }
        let Some(id) = id else { return };
        // Responses (id + result/error, no method) close a tracked request.
        if value.get("result").is_none() && value.get("error").is_none() {
            return;
        }
        let method = self.requests.lock().await.remove(&id);
        match method.as_deref() {
            Some("session/new") | Some("session/load") => {
                if let Some(session_id) = value
                    .get("result")
                    .and_then(|r| r.get("sessionId"))
                    .and_then(serde_json::Value::as_str)
                {
                    self.state.lock().await.session_id = Some(session_id.to_string());
                }
            }
            Some("session/prompt") => {
                self.state.lock().await.prompt_in_flight = None;
            }
            // 0dk S2（332）：会话关闭/删除成功后，槽不能再把这个 id 当作
            // 「可重绑的活跃会话」写进 attach preamble——刷新后的页面会拿
            // 已关闭 id 去 session/load。清掉事实即可，桥不做会话语义。
            Some("_orz/session_close") | Some("_orz/session_delete") => {
                let closed_id = value
                    .get("result")
                    .and_then(|r| r.get("sessionId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                let mut state = self.state.lock().await;
                if let Some(closed_id) = closed_id
                    && state.session_id.as_deref() == Some(closed_id.as_str())
                {
                    state.session_id = None;
                    state.prompt_in_flight = None;
                }
            }
            _ => {}
        }
    }

    /// Observe one client→agent frame (request id/method + prompt start).
    async fn observe_client_frame(&self, text: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let (Some(id), Some(method)) = (
            value.get("id").and_then(serde_json::Value::as_u64),
            value.get("method").and_then(serde_json::Value::as_str),
        ) else {
            return;
        };
        if !tracked_method(method) {
            return;
        }
        self.requests.lock().await.insert(id, method.to_string());
        if method == "session/prompt" {
            self.state.lock().await.prompt_in_flight = Some(id);
        }
    }

    /// Deliver one frame to the attached WS, or buffer it while detached.
    async fn deliver(&self, line: String) {
        let mut state = self.state.lock().await;
        if let Some(sink) = state.sink.as_ref() {
            if sink.send(line.clone()).is_ok() {
                return;
            }
            // Receiver gone (WS task vanished between close and detach).
            state.sink = None;
            state.detached_at = Some(Instant::now());
        }
        state.push_backlog(line);
    }

    /// Mark the child gone: notify the attached WS (if any) and close the
    /// sink so the pump exits; the reaper removes the dead slot.
    async fn mark_dead(&self) {
        let mut state = self.state.lock().await;
        state.dead = true;
        if let Some(sink) = state.sink.take() {
            let _ = sink.send(
                r#"{"jsonrpc":"2.0","method":"_orz/bridge_child_exited","params":{}}"#.to_string(),
            );
        }
        state.detached_at = Some(Instant::now());
    }

    async fn write_stdin(&self, text: &str) -> std::io::Result<()> {
        let mut guard = self.stdin.lock().await;
        let Some(stdin) = guard.as_mut() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "agent stdin closed",
            ));
        };
        stdin.write_all(text.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await
    }

    /// Terminate the child and drop its pipes (idempotent).
    async fn kill(&self) {
        {
            let mut state = self.state.lock().await;
            state.dead = true;
            state.sink = None;
            state.backlog.clear();
            state.backlog_bytes = 0;
        }
        self.stdin.lock().await.take();
        let mut child = self.child.lock().await.take();
        if let Some(mut child) = child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
    }
}

/// Only these method families are tracked (keeps the id map tiny).
fn tracked_method(method: &str) -> bool {
    method.starts_with("session/") || method.starts_with("_orz/")
}

/// One attached WS connection handed to [`AgentBridge::run_connection`].
pub struct Attached {
    handle: Arc<AgentHandle>,
    /// First frame on the new WS (binds a refreshed page to the live session).
    preamble: String,
    /// Frames emitted while detached, in order, sent right after the preamble.
    backlog: Vec<String>,
    /// Live output for this connection.
    ingress: mpsc::UnboundedReceiver<String>,
}

/// Why an attach could not be served.
pub enum Attach {
    Ready(Attached),
    /// Another WS is already attached to this workspace.
    Busy,
    /// Spawning `orz --stdio` failed (message is user-visible).
    SpawnFailed(String),
}

/// Claim the single WS sink for `handle`; `None` when another WS already owns
/// it (the 409 path). While detached, buffered frames are handed back so the
/// new connection receives them right after the preamble.
async fn claim_attachment(
    handle: &Arc<AgentHandle>,
    grace: Duration,
    conn: u64,
    reused: bool,
    cwd: &Path,
) -> Option<Attached> {
    let mut state = handle.state.lock().await;
    if state.sink.is_some() {
        return None;
    }
    let (sink, ingress) = mpsc::unbounded_channel();
    state.sink = Some(sink);
    state.detached_at = None;
    let truncated = std::mem::take(&mut state.truncated);
    let mut backlog: Vec<String> = state.backlog.drain(..).collect();
    state.backlog_bytes = 0;
    let preamble = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "_orz/bridge_attached",
        "params": {
            "session_id": state.session_id,
            "reused": reused,
            "prompt_in_flight": state.prompt_in_flight.is_some(),
            "grace_secs": grace.as_secs(),
            "connection": conn,
            // 绑定根的真相面：刷新后的新页面还没拉到 /api/boot 时，
            // 也要能把会话根设对（否则旧工作区会话的 tail 会按新 cwd 找）。
            "cwd": cwd.display().to_string(),
        },
    })
    .to_string();
    drop(state);
    if truncated {
        backlog.insert(
            0,
            r#"{"jsonrpc":"2.0","method":"_orz/bridge_backlog_truncated","params":{"dropped":true}}"#
                .to_string(),
        );
    }
    Some(Attached {
        handle: handle.clone(),
        preamble,
        backlog,
        ingress,
    })
}

/// Per-workspace agent supervisor: owns children, survives WS detach.
#[derive(Clone)]
pub struct AgentBridge {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    slots: Mutex<HashMap<String, Arc<AgentHandle>>>,
    agent_binary: PathBuf,
    grace: Duration,
    next_conn: AtomicU64,
}

impl AgentBridge {
    /// Build the supervisor. Does not spawn anything; call [`Self::start_reaper`]
    /// once the runtime is up.
    pub fn new(agent_binary: PathBuf, grace: Duration) -> Self {
        Self {
            inner: Arc::new(BridgeInner {
                slots: Mutex::new(HashMap::new()),
                agent_binary,
                grace,
                next_conn: AtomicU64::new(0),
            }),
        }
    }

    /// Grace applied to detached slots (also echoed in the attach preamble).
    pub fn grace(&self) -> Duration {
        self.inner.grace
    }

    /// Background reaper: reclaim detached slots past grace and dead slots.
    pub fn start_reaper(&self) {
        let bridge = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(REAPER_TICK);
            loop {
                tick.tick().await;
                bridge.reap_once().await;
            }
        });
    }

    async fn reap_once(&self) {
        let mut victims: Vec<String> = Vec::new();
        {
            let slots = self.inner.slots.lock().await;
            for (key, handle) in slots.iter() {
                let state = handle.state.lock().await;
                if state.dead {
                    victims.push(key.clone());
                    continue;
                }
                if state.sink.is_none()
                    && let Some(at) = state.detached_at
                    && at.elapsed() >= self.inner.grace
                {
                    victims.push(key.clone());
                }
            }
        }
        for key in victims {
            let handle = self.inner.slots.lock().await.remove(&key);
            if let Some(handle) = handle {
                tracing::info!(key = %key, "ACP keepalive grace expired; reclaiming agent child");
                handle.kill().await;
            }
        }
    }

    /// Attach a new WS to the workspace's child, spawning one when absent.
    ///
    /// `Busy` when another WS is attached; `Ready` also tells whether the
    /// child was reused (keepalive) and which session it holds.
    pub async fn attach(&self, key: &str, cwd: &Path) -> Attach {
        // Single critical section: spawn (fast fork) and sink claim both
        // happen under the slots lock, so two concurrent upgrades cannot both
        // spawn or both attach.
        let mut slots = self.inner.slots.lock().await;
        let existing = slots.get(key).cloned();
        let (handle, reused) = match existing {
            Some(handle) => {
                let dead = handle.state.lock().await.dead;
                if dead {
                    slots.remove(key);
                    match self.spawn_slot(key, cwd).await {
                        Ok(handle) => {
                            slots.insert(key.to_string(), handle.clone());
                            (handle, false)
                        }
                        Err(e) => return Attach::SpawnFailed(e),
                    }
                } else {
                    (handle, true)
                }
            }
            None => match self.spawn_slot(key, cwd).await {
                Ok(handle) => {
                    slots.insert(key.to_string(), handle.clone());
                    (handle, false)
                }
                Err(e) => return Attach::SpawnFailed(e),
            },
        };

        let conn = self.inner.next_conn.fetch_add(1, Ordering::Relaxed) + 1;
        let attached = claim_attachment(&handle, self.inner.grace, conn, reused, cwd).await;
        drop(slots);
        match attached {
            Some(attached) => {
                tracing::info!(key = %key, reused, "ACP WS attached to agent child");
                Attach::Ready(attached)
            }
            None => Attach::Busy,
        }
    }

    async fn spawn_slot(&self, key: &str, cwd: &Path) -> Result<Arc<AgentHandle>, String> {
        let mut child = spawn_agent(&self.inner.agent_binary, cwd)
            .await
            .map_err(|e| {
                format!(
                    "spawning {} --stdio failed: {e}",
                    self.inner.agent_binary.display()
                )
            })?;
        tracing::info!(key = %key, pid = ?child.id(), "ACP agent child spawned");
        let stdin = child.stdin.take().expect("child.stdin piped");
        let stdout = child.stdout.take().expect("child.stdout piped");
        let stderr = child.stderr.take().expect("child.stderr piped");

        // Drain stderr into tracing so the pipe never fills and blocks the agent.
        tokio::spawn(async move {
            let mut stderr = stderr;
            let mut buf = vec![0u8; 8 * 1024];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => tracing::debug!(
                        target: "orz_stdio",
                        "{}",
                        String::from_utf8_lossy(&buf[..n]).trim_end()
                    ),
                }
            }
        });

        let handle = Arc::new(AgentHandle {
            key: key.to_string(),
            child: Mutex::new(Some(child)),
            stdin: Mutex::new(Some(stdin)),
            state: Mutex::new(SlotState::default()),
            requests: Mutex::new(HashMap::new()),
        });
        let reader_handle = handle.clone();
        tokio::spawn(async move { read_agent_loop(reader_handle, stdout).await });
        Ok(handle)
    }

    /// Run one WS connection until it closes (then detach, keep the child).
    pub async fn run_connection(&self, socket: WebSocket, attached: Attached) {
        let handle = attached.handle.clone();
        let key = handle.key.clone();
        let mut socket = socket;
        // Preamble first: a refreshed page binds to the still-running session
        // before backlog/stream frames arrive (and before it decides to
        // session/new — the frontend skips creation when session_id is set).
        if socket.send(Message::text(attached.preamble)).await.is_err() {
            self.detach(&key, &handle).await;
            return;
        }
        for line in attached.backlog {
            if socket.send(Message::text(line)).await.is_err() {
                self.detach(&key, &handle).await;
                return;
            }
        }
        let (mut ws_sink, mut ws_stream) = socket.split();
        let mut ingress = attached.ingress;
        loop {
            tokio::select! {
                // Browser → agent (one JSON-RPC frame per WS text message).
                outbound = ws_stream.next() => match outbound {
                    Some(Ok(Message::Text(t))) => {
                        handle.observe_client_frame(t.as_str()).await;
                        if handle.write_stdin(t.as_str()).await.is_err() {
                            tracing::info!("ACP pump: agent stdin closed");
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    // Ping/Pong are answered at the protocol layer; other
                    // binary frames carry no ACP meaning on this transport.
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        tracing::info!("ACP pump: ws error {e}");
                        break;
                    }
                },
                // Agent → browser (frames from the shared reader).
                inbound = ingress.recv() => match inbound {
                    Some(line) => {
                        if ws_sink.send(Message::text(line)).await.is_err() {
                            break;
                        }
                    }
                    // Reader gone: dead child (frame already announced) or a
                    // replaced sink. Detach; nobody to serve here anymore.
                    None => break,
                },
            }
        }
        let _ = ws_sink.close().await;
        self.detach(&key, &handle).await;
    }

    /// Detach the current WS without killing the child (keepalive).
    async fn detach(&self, key: &str, handle: &Arc<AgentHandle>) {
        {
            let mut state = handle.state.lock().await;
            state.sink = None;
            state.detached_at = Some(Instant::now());
        }
        tracing::info!(key = %key, "ACP WS detached; agent child kept alive");
        if self.inner.grace.is_zero() {
            // Old behaviour explicitly requested (`ORZ_WEB_KEEPALIVE_SECS=0`):
            // reclaim immediately, but never race a reattach that already won
            // the sink.
            let removed = {
                let mut slots = self.inner.slots.lock().await;
                let can_remove = match slots.get(key) {
                    Some(current) => {
                        let state = current.state.lock().await;
                        state.sink.is_none()
                    }
                    None => false,
                };
                if can_remove { slots.remove(key) } else { None }
            };
            if let Some(removed) = removed {
                removed.kill().await;
            }
        }
    }
}

/// Read agent stdout lines, observe facts, and fan them out to the slot.
async fn read_agent_loop(handle: Arc<AgentHandle>, stdout: ChildStdout) {
    let mut stdout = BufReader::new(stdout);
    let mut line_buf: Vec<u8> = Vec::new();
    loop {
        match read_line_bounded(&mut stdout, &mut line_buf, MAX_ACP_LINE_BYTES).await {
            Ok(Some(())) => {
                let text = String::from_utf8_lossy(&line_buf);
                let text = text.strip_suffix('\r').unwrap_or(&text).to_string();
                handle.observe_agent_frame(&text).await;
                handle.deliver(text).await;
            }
            Ok(None) => {
                tracing::info!("ACP agent exited");
                handle.mark_dead().await;
                break;
            }
            Err(e) => {
                tracing::info!("ACP agent stdout error: {e}");
                handle.mark_dead().await;
                break;
            }
        }
    }
}

async fn spawn_agent(agent_binary: &PathBuf, cwd: &Path) -> std::io::Result<Child> {
    tokio::process::Command::new(agent_binary)
        .arg("--stdio")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_line_reader_holds_newline_and_eof() {
        use std::io::Cursor;
        let mut reader = BufReader::new(Cursor::new(b"alpha\nbeta\ngamma".to_vec()));
        let mut buf = Vec::new();
        assert!(
            read_line_bounded(&mut reader, &mut buf, 1024)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(buf, b"alpha");
        assert!(
            read_line_bounded(&mut reader, &mut buf, 1024)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(buf, b"beta");
        // Unterminated tail withholds — an incomplete frame never forwards.
        assert!(
            read_line_bounded(&mut reader, &mut buf, 1024)
                .await
                .unwrap()
                .is_none()
        );
        assert!(buf.is_empty());
    }

    #[tokio::test]
    async fn bounded_line_reader_enforces_cap() {
        use std::io::Cursor;
        let data = vec![b'x'; 128];
        let mut reader = BufReader::new(Cursor::new(data));
        let mut buf = Vec::new();
        let err = read_line_bounded(&mut reader, &mut buf, 64)
            .await
            .expect_err("over-cap line must error");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn keepalive_grace_parsing_is_bounded() {
        assert_eq!(
            parse_keepalive(None),
            Duration::from_secs(DEFAULT_KEEPALIVE_SECS)
        );
        assert_eq!(parse_keepalive(Some("0")), Duration::ZERO);
        assert_eq!(parse_keepalive(Some(" 90 ")), Duration::from_secs(90));
        assert_eq!(
            parse_keepalive(Some("nope")),
            Duration::from_secs(DEFAULT_KEEPALIVE_SECS)
        );
        // Hard cap: a typo cannot pin a child for days.
        assert_eq!(
            parse_keepalive(Some("999999")),
            Duration::from_secs(6 * 3600)
        );
    }

    #[test]
    fn detached_backlog_is_bounded_and_flags_truncation() {
        let mut state = SlotState::default();
        let frame = "x".repeat(1024);
        for _ in 0..(MAX_BACKLOG_BYTES / 1024 + 8) {
            state.push_backlog(frame.clone());
        }
        assert!(state.backlog_bytes <= MAX_BACKLOG_BYTES);
        assert!(
            state.truncated,
            "overflow must be flagged for the next attach"
        );
        // Oldest frames were dropped, newest kept.
        assert_eq!(state.backlog.len(), MAX_BACKLOG_BYTES / 1024);
    }

    #[test]
    fn oversized_single_frame_is_dropped_not_buffered() {
        let mut state = SlotState::default();
        state.push_backlog("y".repeat(MAX_BACKLOG_BYTES + 1));
        assert!(state.backlog.is_empty());
        assert!(state.truncated);
    }

    /// 329 钉：一个工作区同一时刻只允许一个 WS sink；WS 消失但 child 保活，
    /// 期间帧进 backlog，重连时先发 preamble 再补发 backlog。
    #[tokio::test]
    async fn attach_is_exclusive_and_detached_frames_replay_on_reattach() {
        let handle = fake_handle();
        handle.state.lock().await.session_id = Some("sess-1".into());
        let cwd = Path::new("C:/workspace");
        let grace = Duration::from_secs(60);

        let first = claim_attachment(&handle, grace, 1, true, cwd)
            .await
            .expect("first attach must win the sink");
        assert!(first.preamble.contains("\"_orz/bridge_attached\""));
        assert!(first.preamble.contains("sess-1"));
        assert!(first.preamble.contains("\"reused\":true"));

        assert!(
            claim_attachment(&handle, grace, 2, true, cwd)
                .await
                .is_none(),
            "a second WS must be refused while one is attached (409)"
        );

        // WS vanishes (receiver dropped) without an explicit detach: the next
        // delivered frame must fall back to the detached backlog.
        drop(first);
        handle
            .deliver(r#"{"jsonrpc":"2.0","method":"session/update"}"#.to_string())
            .await;
        {
            let state = handle.state.lock().await;
            assert!(state.sink.is_none(), "lost receiver must detach the sink");
            assert_eq!(state.backlog.len(), 1);
        }

        let second = claim_attachment(&handle, grace, 3, true, cwd)
            .await
            .expect("same workspace must reattach (keepalive)");
        assert_eq!(second.backlog.len(), 1);
        assert!(second.backlog[0].contains("session/update"));
    }

    /// 329 钉：桥只做最小观察——session/new|load 的 sessionId 与
    /// session/prompt 在途事实供重连 preamble 使用。
    #[tokio::test]
    async fn session_id_and_prompt_state_are_learned_from_frames() {
        let handle = fake_handle();
        handle
            .observe_client_frame(r#"{"jsonrpc":"2.0","id":7,"method":"session/new","params":{}}"#)
            .await;
        handle
            .observe_agent_frame(r#"{"jsonrpc":"2.0","id":7,"result":{"sessionId":"sess-x"}}"#)
            .await;
        assert_eq!(
            handle.state.lock().await.session_id.as_deref(),
            Some("sess-x")
        );

        handle
            .observe_client_frame(
                r#"{"jsonrpc":"2.0","id":9,"method":"session/prompt","params":{}}"#,
            )
            .await;
        assert_eq!(handle.state.lock().await.prompt_in_flight, Some(9));
        handle
            .observe_agent_frame(r#"{"jsonrpc":"2.0","id":9,"result":{"stopReason":"end_turn"}}"#)
            .await;
        assert_eq!(handle.state.lock().await.prompt_in_flight, None);
    }

    fn fake_handle() -> Arc<AgentHandle> {
        Arc::new(AgentHandle {
            key: "C:/workspace".to_string(),
            child: Mutex::new(None),
            stdin: Mutex::new(None),
            state: Mutex::new(SlotState::default()),
            requests: Mutex::new(HashMap::new()),
        })
    }
}
