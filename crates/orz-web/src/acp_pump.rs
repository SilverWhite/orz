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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

/// 334（§4.12）：attached 期 ingress 也有界——8 MiB 过去只约束 detached
/// backlog，慢/半开 WS 时 unbounded channel 可无限增长。满时丢帧并置
/// overflow 标记，由当前连接发 `_orz/bridge_ingress_truncated`（完整事实
/// 仍在 journal，不谎称无丢失）。
const MAX_ATTACHED_INGRESS_FRAMES: usize = 256;

/// 334（§4.6）：未应答 agent→client 请求（permission）的重放上限。超出丢
/// 最旧并计数；permission 超时后 agent 侧 fail-closed。
const MAX_PENDING_PERMISSIONS: usize = 32;

/// 334（§5.8）：`ws_acp` 在升级前就 claim sink；若升级失败回调不运行，
/// sink 会泄漏到下一次 attach 409。claim 后 15 s 内 `run_connection` 未
/// 启动即自动 detach（窄窗口自愈）。
const ATTACH_CLAIM_TIMEOUT: Duration = Duration::from_secs(15);

/// 334（§5.6）：过期事实的保留上限——避免 map 无限累积（旧实现无 TTL）。
const MAX_EXPIRED_AGE: Duration = Duration::from_secs(6 * 3600);

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
    sink: Option<mpsc::Sender<String>>,
    /// Connection generation owning `sink` (334 §5.7：旧 loop 的 detach 不得
    /// 静音新连接）。
    sink_conn: Option<u64>,
    /// Frames emitted while detached (bounded by `MAX_BACKLOG_BYTES`).
    backlog: VecDeque<String>,
    backlog_bytes: usize,
    /// Set when a backlog overflow dropped frames (told to the next attach).
    truncated: bool,
    /// 334：attached ingress 满而丢帧（由当前连接发通知，不只等下次 attach）。
    attached_overflow: bool,
    /// When the last attach went away (grace reaper reads this).
    detached_at: Option<Instant>,
    /// Learned from the `session/new` / `session/load` response — lets a
    /// refreshed page re-bind to the still-running session without a reload.
    session_id: Option<String>,
    /// 334（§4.5）：会话的归属工作区根（load 参数 cwd），不是 child 的启动
    /// cwd。刷新后的页面据此把 tail/撤回锚解析到正确根。
    session_root: Option<String>,
    /// In-flight `session/prompt` request id (reconnect preamble fact).
    prompt_in_flight: Option<u64>,
    /// Child exited / stream broke; the slot is waiting for the reaper.
    dead: bool,
}

impl Default for SlotState {
    fn default() -> Self {
        Self {
            sink: None,
            sink_conn: None,
            backlog: VecDeque::new(),
            backlog_bytes: 0,
            truncated: false,
            attached_overflow: false,
            detached_at: None,
            session_id: None,
            session_root: None,
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

/// 333（2026-10-21）：保活窗口到期被回收的会话事实。桥在 grace 回收时记下
/// 它，下一次同区 attach 的前置帧据此发「未恢复」的显式提示——否则刷新后
/// 只会看到一个新 child，用户无从知道上一会话已过窗口。
#[derive(Debug, Clone)]
struct ExpiredSession {
    session_id: String,
    at: Instant,
    /// `grace` = 保活窗口到期；`child_exited` = detached 期 child 自己死了。
    /// 334（§5.6）：dead 分支也必须留下过期事实，不能只给通用 boot 文案。
    reason: &'static str,
}

/// One workspace's agent child. Cloned as `Arc`; the reader task and the
/// bridge both hold it.
/// 334（§4.1）：client→agent 请求事实。**只**记录这个方向；agent→client
/// 的 permission 请求有独立的 id 空间（agent rpc 从 0 起、Web 从 1 起），
/// 混在一张表里会让同值 id 互相覆盖，prompt 响应永远清不掉 in-flight。
struct ClientRequestFact {
    method: String,
    /// `session/new|load` 的归属根（params.cwd），供刷新 preamble 回传。
    root: Option<String>,
}

/// 334（§4.6）：未应答的 agent→client permission 请求（原帧），attach 时
/// 重放给新页面；客户端响应（同 id、无 method）到达后移除。
struct PendingPermission {
    id: u64,
    frame: String,
}

struct AgentHandle {
    key: String,
    child: Mutex<Option<Child>>,
    stdin: Mutex<Option<ChildStdin>>,
    state: Mutex<SlotState>,
    /// Client→agent request id → fact (only `session/*` and `_orz/*`), used
    /// to recognise `session/new`/`session/load` responses and prompt state.
    client_requests: Mutex<HashMap<u64, ClientRequestFact>>,
    /// Agent→client requests awaiting a response (bounded).
    pending_permissions: Mutex<VecDeque<PendingPermission>>,
    /// session id → workspace key（刷新页按 session 找回原 child，而不是按
    /// 当前 cwd 另起一个）。
    session_slots: Arc<Mutex<HashMap<String, String>>>,
}

impl AgentHandle {
    /// Observe one agent→client frame: learn session id / prompt completion,
    /// and queue unanswered permission requests for the next attach.
    async fn observe_agent_frame(&self, text: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let id = value.get("id").and_then(serde_json::Value::as_u64);
        if let (Some(id), Some(method)) =
            (id, value.get("method").and_then(serde_json::Value::as_str))
        {
            // 334：agent→client 请求单独排队；**绝不**写 client_requests。
            // permission 是唯一需要重放给新页面的请求种类。
            if method == "session/request_permission" {
                let mut pending = self.pending_permissions.lock().await;
                if !pending.iter().any(|p| p.id == id) {
                    pending.push_back(PendingPermission {
                        id,
                        frame: text.to_string(),
                    });
                    while pending.len() > MAX_PENDING_PERMISSIONS {
                        pending.pop_front();
                    }
                }
            }
            return;
        }
        let Some(id) = id else { return };
        // Responses (id + result/error, no method) close a tracked request.
        if value.get("result").is_none() && value.get("error").is_none() {
            return;
        }
        let fact = self.client_requests.lock().await.remove(&id);
        let Some(fact) = fact else { return };
        match fact.method.as_str() {
            "session/new" | "session/load" => {
                if let Some(session_id) = value
                    .get("result")
                    .and_then(|r| r.get("sessionId"))
                    .and_then(serde_json::Value::as_str)
                {
                    let mut state = self.state.lock().await;
                    state.session_id = Some(session_id.to_string());
                    state.session_root = fact.root.clone();
                    drop(state);
                    // 334（§4.5）：记 session→slot，刷新页带 session 查询时
                    // 直接 attach 到真正持有它的 child（跨区 load 也不丢）。
                    self.session_slots
                        .lock()
                        .await
                        .insert(session_id.to_string(), self.key.clone());
                }
            }
            "session/prompt" => {
                let mut state = self.state.lock().await;
                // 只清自己那一条：同 id 的新 prompt（client id 单调，不应
                // 发生）或别的请求不能误清新 run。
                if state.prompt_in_flight == Some(id) {
                    state.prompt_in_flight = None;
                }
            }
            // 0dk S2（332）：会话关闭/删除成功后，槽不能再把这个 id 当作
            // 「可重绑的活跃会话」写进 attach preamble——刷新后的页面会拿
            // 已关闭 id 去 session/load。清掉事实即可，桥不做会话语义。
            "_orz/session_close" | "_orz/session_delete" => {
                let closed_id = value
                    .get("result")
                    .and_then(|r| r.get("sessionId"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string);
                if let Some(closed_id) = closed_id {
                    self.session_slots.lock().await.remove(&closed_id);
                    let mut state = self.state.lock().await;
                    if state.session_id.as_deref() == Some(closed_id.as_str()) {
                        state.session_id = None;
                        state.session_root = None;
                        state.prompt_in_flight = None;
                    }
                }
            }
            _ => {}
        }
    }

    /// Observe one client→agent frame (request id/method + prompt start) or a
    /// client response to a pending agent request (permission answer).
    async fn observe_client_frame(&self, text: &str) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
            return;
        };
        let id = value.get("id").and_then(serde_json::Value::as_u64);
        let method = value.get("method").and_then(serde_json::Value::as_str);
        let Some(id) = id else { return };
        let Some(method) = method else {
            // 334（§4.6）：permission 响应（同 id、无 method）到达——移出
            // 未应答队列，后续 attach 不再重放。
            if value.get("result").is_some() || value.get("error").is_some() {
                self.pending_permissions.lock().await.retain(|p| p.id != id);
            }
            return;
        };
        if !tracked_method(method) {
            return;
        }
        // 334（§4.5）：记请求的 cwd（session/new|load 的归属根）。
        let root = value
            .get("params")
            .and_then(|p| p.get("cwd"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        self.client_requests.lock().await.insert(
            id,
            ClientRequestFact {
                method: method.to_string(),
                root,
            },
        );
        if method == "session/prompt" {
            self.state.lock().await.prompt_in_flight = Some(id);
        }
    }

    /// 334（§4.12）：当前连接取走 ingress 溢出标记（取走即清）。
    async fn take_attached_overflow(&self) -> bool {
        let mut state = self.state.lock().await;
        std::mem::take(&mut state.attached_overflow)
    }

    async fn pending_permission_frames(&self) -> Vec<String> {
        self.pending_permissions
            .lock()
            .await
            .iter()
            .map(|p| p.frame.clone())
            .collect()
    }

    /// Deliver one frame to the attached WS, or buffer it while detached.
    async fn deliver(&self, line: String) {
        let mut state = self.state.lock().await;
        if let Some(sink) = state.sink.as_ref() {
            match sink.try_send(line.clone()) {
                Ok(()) => return,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // 334（§4.12）：attached ingress 有界——满则丢本帧并置
                    // 标记；当前连接发 `_orz/bridge_ingress_truncated`。
                    // 完整事实仍在 journal，通知不谎称「无丢失」。
                    state.attached_overflow = true;
                    return;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    // Receiver gone (WS task vanished between close and detach).
                    state.sink = None;
                    state.sink_conn = None;
                    state.detached_at = Some(Instant::now());
                }
            }
        }
        state.push_backlog(line);
    }

    /// Mark the child gone: notify the attached WS (if any) and close the
    /// sink so the pump exits; the reaper removes the dead slot.
    async fn mark_dead(&self) {
        let mut state = self.state.lock().await;
        state.dead = true;
        state.sink_conn = None;
        if let Some(sink) = state.sink.take() {
            let _ = sink.try_send(
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
            state.sink_conn = None;
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
    /// 334（§4.6）：未应答的 permission 请求，随 attach 重放（等客户端响应
    /// 后才移除；若新页再次断开，下次 attach 仍会重放）。
    permissions: Vec<String>,
    /// Live output for this connection (bounded, 334 §4.12).
    ingress: mpsc::Receiver<String>,
    /// Connection generation (334 §5.7：旧 loop 不得 detach 新连接).
    conn: u64,
    /// Set by `run_connection`; the attach-claim watchdog uses it to release a
    /// sink whose WS upgrade never completed.
    started: Arc<AtomicBool>,
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
    expired: Option<&ExpiredSession>,
) -> Option<Attached> {
    let pending_permissions = handle.pending_permission_frames().await;
    let pending_permission_count = pending_permissions.len();
    let mut state = handle.state.lock().await;
    if state.sink.is_some() {
        return None;
    }
    let (sink, ingress) = mpsc::channel(MAX_ATTACHED_INGRESS_FRAMES);
    state.sink = Some(sink);
    state.sink_conn = Some(conn);
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
            // 333：保活窗口到期回收过（且尚未被新会话覆盖）——前端必须显式
            // 说明「上一会话未恢复」，不得让用户以为重连成功。
            "keepalive_expired": expired.is_some(),
            "expired_session_id": expired.map(|e| e.session_id.clone()),
            "expired_secs_ago": expired.map(|e| e.at.elapsed().as_secs()),
            // 334（§5.6）：dead child 也留过期事实，并带原因。
            "expired_reason": expired.map(|e| e.reason),
            // 绑定根的真相面：刷新后的新页面还没拉到 /api/boot 时，
            // 也要能把会话根设对（否则旧工作区会话的 tail 会按新 cwd 找）。
            // 334（§4.5）：会话根（load 参数）优先于 child 启动 cwd。
            "cwd": state
                .session_root
                .clone()
                .unwrap_or_else(|| cwd.display().to_string()),
            "session_root": state.session_root.clone(),
            "pending_permission_count": pending_permission_count,
            "attached_ingress_capacity": MAX_ATTACHED_INGRESS_FRAMES,
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
        permissions: pending_permissions,
        ingress,
        conn,
        started: Arc::new(AtomicBool::new(false)),
    })
}

/// Per-workspace agent supervisor: owns children, survives WS detach.
#[derive(Clone)]
pub struct AgentBridge {
    inner: Arc<BridgeInner>,
}

struct BridgeInner {
    slots: Mutex<HashMap<String, Arc<AgentHandle>>>,
    /// 333：key → 最近一次因保活到期被回收的会话（下一次 attach 消费）。
    expired: Mutex<HashMap<String, ExpiredSession>>,
    /// 334（§4.5）：session id → workspace key。刷新页带 session 查询时
    /// 直接 attach 到真正持有它的 child（跨区 load / 切区后刷新都不丢）。
    session_slots: Arc<Mutex<HashMap<String, String>>>,
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
                expired: Mutex::new(HashMap::new()),
                session_slots: Arc::new(Mutex::new(HashMap::new())),
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
        let handles: Vec<(String, Arc<AgentHandle>)> = self
            .inner
            .slots
            .lock()
            .await
            .iter()
            .map(|(key, handle)| (key.clone(), handle.clone()))
            .collect();
        // 334（§5.6）：dead child 与 grace 到期都留过期事实（带 reason）。
        for (key, handle) in handles {
            let (reason, session_id) = {
                let state = handle.state.lock().await;
                if state.dead {
                    ("child_exited", state.session_id.clone())
                } else if state.sink.is_none()
                    && let Some(at) = state.detached_at
                    && at.elapsed() >= self.inner.grace
                {
                    ("grace", state.session_id.clone())
                } else {
                    continue;
                }
            };
            // 334（§5.7）：按 Arc 身份 + 复核条件移除，避免误删同 key 的
            // 新槽（reap 收集 victim 后、remove 前发生重连/重建）。
            let removed = {
                let mut slots = self.inner.slots.lock().await;
                let still = match slots.get(&key) {
                    Some(current) if Arc::ptr_eq(current, &handle) => {
                        let state = current.state.lock().await;
                        state.dead
                            || (state.sink.is_none()
                                && state
                                    .detached_at
                                    .is_some_and(|at| at.elapsed() >= self.inner.grace))
                    }
                    _ => false,
                };
                if still { slots.remove(&key) } else { None }
            };
            let Some(removed) = removed else { continue };
            if let Some(session_id) = session_id {
                self.inner.expired.lock().await.insert(
                    key.clone(),
                    ExpiredSession {
                        session_id,
                        at: Instant::now(),
                        reason,
                    },
                );
            }
            tracing::info!(
                key = %key,
                reason,
                "ACP agent child reclaimed (grace/dead); session expiry recorded"
            );
            removed.kill().await;
        }
        self.inner
            .expired
            .lock()
            .await
            .retain(|_, expired| expired.at.elapsed() < MAX_EXPIRED_AGE);
    }

    /// Attach a new WS to the workspace's child, spawning one when absent.
    ///
    /// `Busy` when another WS is attached; `Ready` also tells whether the
    /// child was reused (keepalive) and which session it holds.
    pub async fn attach(&self, key: &str, cwd: &Path, requested_session: Option<&str>) -> Attach {
        // 334（§4.5）：刷新页可带 session id——优先 attach 到真正持有该
        // 会话的槽（跨工作区 load 后，会话根 ≠ 当前 cwd；按 cwd 找会失联）。
        let owner_key = match requested_session {
            Some(session_id) => self
                .inner
                .session_slots
                .lock()
                .await
                .get(session_id)
                .cloned(),
            None => None,
        };
        let key: &str = owner_key.as_deref().unwrap_or(key);
        let cwd: &Path = if owner_key.is_some() {
            Path::new(key)
        } else {
            cwd
        };
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
        // 333：过期事实只在「新建 child」的 attach 上消费；复用保活 child
        // 时它仍是同一个会话，不得谎报上一会话已回收。
        let expired = if reused {
            None
        } else {
            self.inner.expired.lock().await.remove(key)
        };
        let attached = claim_attachment(
            &handle,
            self.inner.grace,
            conn,
            reused,
            cwd,
            expired.as_ref(),
        )
        .await;
        drop(slots);
        match attached {
            Some(attached) => {
                // 334（§5.8）：升级失败回调不运行时，claim 的 sink 不能永久
                // 占位（下一次 attach 会 409）。15 s 内 run_connection 未启动
                // 就按连接代次安全释放。
                let handle = attached.handle.clone();
                let started = attached.started.clone();
                let conn = attached.conn;
                tokio::spawn(async move {
                    tokio::time::sleep(ATTACH_CLAIM_TIMEOUT).await;
                    if started.load(Ordering::Acquire) {
                        return;
                    }
                    let mut state = handle.state.lock().await;
                    if state.sink_conn == Some(conn) {
                        state.sink = None;
                        state.sink_conn = None;
                        state.detached_at = Some(Instant::now());
                        tracing::warn!(
                            "ACP WS upgrade did not start within 15s; released the attach claim"
                        );
                    }
                });
                tracing::info!(key = %key, reused, "ACP WS attached to agent child");
                Attach::Ready(attached)
            }
            None => Attach::Busy,
        }
    }

    /// 334（§3.2/S1）：服务端「有 run 在跑」读数——工作区切换的拒绝依据。
    /// 旧实现只靠前端 `state.running`；假运行态/多页时门形同虚设。
    pub async fn any_prompt_in_flight(&self) -> bool {
        let slots: Vec<Arc<AgentHandle>> =
            self.inner.slots.lock().await.values().cloned().collect();
        for handle in slots {
            if handle.state.lock().await.prompt_in_flight.is_some() {
                return true;
            }
        }
        false
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
            client_requests: Mutex::new(HashMap::new()),
            pending_permissions: Mutex::new(VecDeque::new()),
            session_slots: self.inner.session_slots.clone(),
        });
        let reader_handle = handle.clone();
        tokio::spawn(async move { read_agent_loop(reader_handle, stdout).await });
        Ok(handle)
    }

    /// Run one WS connection until it closes (then detach, keep the child).
    pub async fn run_connection(&self, socket: WebSocket, attached: Attached) {
        let handle = attached.handle.clone();
        let key = handle.key.clone();
        let conn = attached.conn;
        attached.started.store(true, Ordering::Release);
        let mut socket = socket;
        // Preamble first: a refreshed page binds to the still-running session
        // before backlog/stream frames arrive (and before it decides to
        // session/new — the frontend skips creation when session_id is set).
        if socket.send(Message::text(attached.preamble)).await.is_err() {
            self.detach(&key, &handle, conn).await;
            return;
        }
        for line in attached.backlog {
            if socket.send(Message::text(line)).await.is_err() {
                self.detach(&key, &handle, conn).await;
                return;
            }
        }
        // 334（§4.6）：断线前未应答的 permission 请求重放给新页（等响应才
        // 移除；若新页再次断开，下次 attach 仍会重放）。
        for frame in attached.permissions {
            if socket.send(Message::text(frame)).await.is_err() {
                self.detach(&key, &handle, conn).await;
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
                        // 334（§4.12）：attached ingress 曾满而丢帧——当前
                        // 连接立即收到通知，不用等到下次 attach。
                        if handle.take_attached_overflow().await {
                            let notice = r#"{"jsonrpc":"2.0","method":"_orz/bridge_ingress_truncated","params":{"dropped":"attached_ingress_full","detail":"浏览器接收过慢，部分流式帧已丢弃；完整事实以 journal 为准"}}"#;
                            if ws_sink.send(Message::text(notice)).await.is_err() {
                                break;
                            }
                        }
                    }
                    // Reader gone: dead child (frame already announced) or a
                    // replaced sink. Detach; nobody to serve here anymore.
                    None => break,
                },
            }
        }
        let _ = ws_sink.close().await;
        self.detach(&key, &handle, conn).await;
    }

    /// Detach the current WS without killing the child (keepalive).
    ///
    /// 334（§5.7）：带连接代次——旧 loop 退出时若新连接已 attach（同一
    /// handle、sink 已换），旧 detach 不得把新 sink 置空（否则新连接被静音）。
    async fn detach(&self, key: &str, handle: &Arc<AgentHandle>, conn: u64) {
        {
            let mut state = handle.state.lock().await;
            if state.sink_conn != Some(conn) {
                tracing::debug!(
                    key = %key,
                    conn,
                    current = ?state.sink_conn,
                    "stale ACP WS detach ignored (a newer connection owns the sink)"
                );
                return;
            }
            state.sink = None;
            state.sink_conn = None;
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

        let first = claim_attachment(&handle, grace, 1, true, cwd, None)
            .await
            .expect("first attach must win the sink");
        assert!(first.preamble.contains("\"_orz/bridge_attached\""));
        assert!(first.preamble.contains("sess-1"));
        assert!(first.preamble.contains("\"reused\":true"));

        assert!(
            claim_attachment(&handle, grace, 2, true, cwd, None)
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

        let second = claim_attachment(&handle, grace, 3, true, cwd, None)
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

    /// 333 钉：保活窗口到期回收后，下一次 attach 的前置帧必须显式报告
    /// 「上一会话未恢复」——不能只给一个干净的新 child 让用户误以为重连成功。
    #[tokio::test]
    async fn keepalive_expiry_is_reported_on_next_attach() {
        let bridge = AgentBridge::new(PathBuf::from("orz"), Duration::from_secs(1));
        let handle = fake_handle();
        {
            let mut state = handle.state.lock().await;
            state.session_id = Some("sess-expired".into());
            state.detached_at = Some(Instant::now() - Duration::from_secs(2));
        }
        bridge
            .inner
            .slots
            .lock()
            .await
            .insert("C:/workspace".into(), handle);
        bridge.reap_once().await;
        assert!(
            bridge.inner.slots.lock().await.is_empty(),
            "grace-expired slot must be reclaimed"
        );
        let expired = bridge
            .inner
            .expired
            .lock()
            .await
            .remove("C:/workspace")
            .expect("expiry fact retained for the next attach");
        assert_eq!(expired.session_id, "sess-expired");

        let fresh = fake_handle();
        let attached = claim_attachment(
            &fresh,
            bridge.grace(),
            9,
            false,
            Path::new("C:/workspace"),
            Some(&expired),
        )
        .await
        .expect("fresh attach");
        assert!(
            attached.preamble.contains("\"keepalive_expired\":true"),
            "{}",
            attached.preamble
        );
        assert!(
            attached.preamble.contains("sess-expired"),
            "{}",
            attached.preamble
        );
        assert!(
            attached.preamble.contains("\"expired_secs_ago\""),
            "{}",
            attached.preamble
        );
    }

    /// 334（§4.1）：client 与 agent 的 id 空间独立但值域重叠。同值 id 的
    /// permission 不得覆盖在途 prompt；prompt 响应必须仍能清 in-flight。
    #[tokio::test]
    async fn bridge_request_ids_do_not_collide() {
        let handle = fake_handle();
        handle
            .observe_client_frame(
                r#"{"jsonrpc":"2.0","id":3,"method":"session/prompt","params":{"sessionId":"s","prompt":[]}}"#,
            )
            .await;
        handle
            .observe_agent_frame(
                r#"{"jsonrpc":"2.0","id":3,"method":"session/request_permission","params":{}}"#,
            )
            .await;
        // prompt 事实仍在 client 表；permission 单独排队。
        assert_eq!(
            handle
                .client_requests
                .lock()
                .await
                .get(&3)
                .map(|f| f.method.as_str()),
            Some("session/prompt")
        );
        assert_eq!(handle.pending_permissions.lock().await.len(), 1);
        assert_eq!(handle.state.lock().await.prompt_in_flight, Some(3));

        // agent 对 prompt 的响应按 client id 清 in-flight；permission 等
        // 客户端响应（无 method）才移出队列。
        handle
            .observe_agent_frame(r#"{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}"#)
            .await;
        assert_eq!(handle.state.lock().await.prompt_in_flight, None);
        assert_eq!(handle.pending_permissions.lock().await.len(), 1);
        handle
            .observe_client_frame(
                r#"{"jsonrpc":"2.0","id":3,"result":{"outcome":{"outcome":"cancelled"}}}"#,
            )
            .await;
        assert!(handle.pending_permissions.lock().await.is_empty());
    }

    /// 334（§4.6）：permission 已投递 → 页面消失 → 新 attach 必须重放到
    /// 新连接（不是留在旧 WS 的已消费队列里）。
    #[tokio::test]
    async fn unanswered_permission_is_replayed_on_reattach() {
        let handle = fake_handle();
        handle
            .observe_agent_frame(
                r#"{"jsonrpc":"2.0","id":7,"method":"session/request_permission","params":{"toolCall":{"title":"bash"}}}"#,
            )
            .await;
        let attached = claim_attachment(
            &handle,
            Duration::from_secs(60),
            1,
            true,
            Path::new("C:/workspace"),
            None,
        )
        .await
        .expect("attach");
        assert_eq!(attached.permissions.len(), 1);
        assert!(attached.permissions[0].contains("session/request_permission"));
        assert!(attached.preamble.contains("\"pending_permission_count\":1"));
        drop(attached);
        // 模拟 WS 消失：下一帧发现 receiver 已关，清 sink 后进 backlog。
        handle
            .deliver(r#"{"jsonrpc":"2.0","method":"session/update"}"#.to_string())
            .await;
        // 未应答：再次 attach 仍重放。
        let again = claim_attachment(
            &handle,
            Duration::from_secs(60),
            2,
            true,
            Path::new("C:/workspace"),
            None,
        )
        .await
        .expect("reattach");
        assert_eq!(again.permissions.len(), 1);
    }

    /// 334（§4.5）：session/new|load 的归属根随响应记入槽，attach preamble
    /// 回传 session_root（不是 child 启动 cwd）。
    #[tokio::test]
    async fn session_root_is_learned_from_load_params() {
        let handle = fake_handle();
        handle
            .observe_client_frame(
                r#"{"jsonrpc":"2.0","id":4,"method":"session/load","params":{"cwd":"C:/root-a","sessionId":"sess-a"}}"#,
            )
            .await;
        handle
            .observe_agent_frame(r#"{"jsonrpc":"2.0","id":4,"result":{"sessionId":"sess-a"}}"#)
            .await;
        assert_eq!(
            handle.state.lock().await.session_root.as_deref(),
            Some("C:/root-a")
        );
        assert_eq!(
            handle
                .session_slots
                .lock()
                .await
                .get("sess-a")
                .map(String::as_str),
            Some("C:/workspace")
        );
        let attached = claim_attachment(
            &handle,
            Duration::from_secs(60),
            1,
            false,
            Path::new("C:/workspace"),
            None,
        )
        .await
        .expect("attach");
        assert!(attached.preamble.contains("\"session_root\":\"C:/root-a\""));
        assert!(attached.preamble.contains("\"cwd\":\"C:/root-a\""));
    }

    /// 334（§4.12）：attached ingress 有界——第 257 帧触发 overflow 标记而
    /// 不是无限缓冲。
    #[tokio::test]
    async fn attached_ingress_is_bounded() {
        let handle = fake_handle();
        let attached = claim_attachment(
            &handle,
            Duration::from_secs(60),
            1,
            true,
            Path::new("C:/workspace"),
            None,
        )
        .await
        .expect("attach");
        let frame = r#"{"jsonrpc":"2.0","method":"session/update"}"#.to_string();
        for _ in 0..MAX_ATTACHED_INGRESS_FRAMES {
            handle.deliver(frame.clone()).await;
        }
        assert!(
            !handle.take_attached_overflow().await,
            "capacity not exceeded"
        );
        handle.deliver(frame).await;
        assert!(handle.take_attached_overflow().await);
        drop(attached);
    }

    /// 334（§5.6）：detached 期 child 死亡也留过期事实（reason=child_exited），
    /// 新 attach 的 preamble 必须报「未恢复」。
    #[tokio::test]
    async fn dead_child_is_reported_as_expired_on_next_attach() {
        let bridge = AgentBridge::new(PathBuf::from("orz"), Duration::from_secs(60));
        let handle = fake_handle();
        {
            let mut state = handle.state.lock().await;
            state.session_id = Some("sess-dead".into());
        }
        bridge
            .inner
            .slots
            .lock()
            .await
            .insert("C:/workspace".into(), handle.clone());
        handle.mark_dead().await;
        bridge.reap_once().await;
        let expired = bridge
            .inner
            .expired
            .lock()
            .await
            .remove("C:/workspace")
            .expect("dead child expiry fact");
        assert_eq!(expired.session_id, "sess-dead");
        assert_eq!(expired.reason, "child_exited");
    }

    fn fake_handle() -> Arc<AgentHandle> {
        Arc::new(AgentHandle {
            key: "C:/workspace".to_string(),
            child: Mutex::new(None),
            stdin: Mutex::new(None),
            state: Mutex::new(SlotState::default()),
            client_requests: Mutex::new(HashMap::new()),
            pending_permissions: Mutex::new(VecDeque::new()),
            session_slots: Arc::new(Mutex::new(HashMap::new())),
        })
    }
}
