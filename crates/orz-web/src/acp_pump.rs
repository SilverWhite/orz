//! ACP-over-WebSocket pump (0br S2, survey §5.1): one WS connection
//! corresponds to one `orz --stdio` child process. Frames are newline-
//! delimited ACP JSON-RPC; the pump is a transparent 1:1 relay in both
//! directions. The bridge owns the child lifecycle — socket close or child
//! exit tears the pair down (the bridge is the direct parent, so no orphan
//! is left behind; the `stdin EOF ⇒ exit` contract stays the backstop).

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::Child;
use tokio::sync::Mutex;

/// Cap for a single agent stdout line (one JSON-RPC frame). The agent is
/// our own binary, but the cap exists so a runaway writer can never grow
/// bridge memory without bound — overflow tears the pair down like any
/// other stream error.
const MAX_ACP_LINE_BYTES: usize = 64 * 1024 * 1024;

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

/// Run one pumped connection to completion. `release` must be invoked on
/// every exit path to free the single-session slot.
pub async fn pump(
    socket: WebSocket,
    agent_binary: PathBuf,
    cwd: PathBuf,
    release: Arc<dyn Fn() + Send + Sync>,
) {
    let mut child = match spawn_agent(&agent_binary, &cwd).await {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("spawning {} --stdio failed: {e}", agent_binary.display());
            let mut socket = socket;
            let _ = socket
                .send(Message::text(
                    r#"{"error":{"code":-32603,"message":"failed to spawn orz --stdio"}}"#,
                ))
                .await;
            let _ = socket.close().await;
            release();
            return;
        }
    };
    tracing::info!("ACP agent child spawned (pid {:?})", child.id());

    let mut stdin = child.stdin.take().expect("child.stdin piped");
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

    let (mut ws_sink, mut ws_stream) = socket.split();
    let mut stdout = BufReader::new(stdout);
    let mut line_buf: Vec<u8> = Vec::new();
    let mut close_reason: Option<String> = None;

    while close_reason.is_none() {
        tokio::select! {
            // Browser → agent (one JSON-RPC frame per WS text message).
            outbound = ws_stream.next() => match outbound {
                Some(Ok(Message::Text(t))) => {
                    if stdin.write_all(t.as_bytes()).await.is_err()
                        || stdin.write_all(b"\n").await.is_err()
                        || stdin.flush().await.is_err()
                    {
                        close_reason = Some("agent stdin closed".into());
                    }
                }
                Some(Ok(Message::Close(_))) | None => {
                    close_reason = Some("client closed".into());
                }
                // Ping/Pong are answered at the protocol layer; other
                // binary frames carry no ACP meaning on this transport.
                Some(Ok(_)) => {}
                Some(Err(e)) => close_reason = Some(format!("ws error: {e}")),
            },
            // Agent → browser (one stdio line per WS text message).
            inbound = read_line_bounded(&mut stdout, &mut line_buf, MAX_ACP_LINE_BYTES) => match inbound {
                Ok(Some(())) => {
                    let text = String::from_utf8_lossy(&line_buf);
                    let text = text.strip_suffix('\r').unwrap_or(&text);
                    if ws_sink.send(Message::text(text.to_string())).await.is_err() {
                        close_reason = Some("client closed".into());
                    }
                }
                Ok(None) => close_reason = Some("agent exited".into()),
                Err(e) => close_reason = Some(format!("agent stdout error: {e}")),
            },
        }
    }

    if let Some(reason) = &close_reason {
        tracing::info!("ACP pump ending: {reason}");
    }
    // One side is gone — tear the pair down. `start_kill` prompts the exit;
    // dropping the child would also kill it (kill_on_drop).
    let _ = child.start_kill();
    let _ = child.wait().await;
    let _ = ws_sink.close().await;
    release();
}

async fn spawn_agent(agent_binary: &PathBuf, cwd: &PathBuf) -> std::io::Result<Child> {
    tokio::process::Command::new(agent_binary)
        .arg("--stdio")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
}

/// Per-workspace session guard（0bv ②，2026-09-26，B 形态）：每个工作区各自
/// 一条活动会话槽——切换工作区不清场、不杀旧对话子进程；新区可在其自身 cwd
/// 下开新会话。旧形态为全局单槽（跨工作区互斥），与「切换留下旧对话」语义
/// 冲突，故按区隔离（键＝连接时点 cwd 的 canonical display 串）。
#[derive(Clone, Default)]
pub struct SessionSlot(Arc<Mutex<std::collections::HashSet<String>>>);

impl SessionSlot {
    /// Try to claim the session for `key`; `false` when already held.
    pub async fn try_acquire(&self, key: &str) -> bool {
        let mut guard = self.0.lock().await;
        guard.insert(key.to_string())
    }

    /// Peek without claiming (pre-upgrade 409 answer).
    pub async fn is_held(&self, key: &str) -> bool {
        self.0.lock().await.contains(key)
    }

    /// 任一工作区持有活动会话（观测读数；不作门禁）。
    pub async fn any_held(&self) -> bool {
        !self.0.lock().await.is_empty()
    }

    pub async fn release(&self, key: &str) {
        self.0.lock().await.remove(key);
    }
}

/// RAII companion to a held [`SessionSlot`]: dropping the guard frees the
/// slot (the release is spawned onto the runtime), so a panic unwinding
/// through the pump future cannot strand the bridge in a permanent 409.
/// The explicit [`SlotGuard::releaser`] hook stays idempotent with `Drop`.
pub struct SlotGuard {
    slot: SessionSlot,
    key: String,
}

impl SlotGuard {
    pub fn new(slot: SessionSlot, key: String) -> Self {
        Self { slot, key }
    }

    /// Early-release hook handed to the pump (idempotent with `Drop`).
    pub fn releaser(&self) -> Arc<dyn Fn() + Send + Sync> {
        let slot = self.slot.clone();
        let key = self.key.clone();
        Arc::new(move || {
            let slot = slot.clone();
            let key = key.clone();
            tokio::spawn(async move { slot.release(&key).await });
        })
    }
}

impl Drop for SlotGuard {
    fn drop(&mut self) {
        let slot = self.slot.clone();
        let key = self.key.clone();
        tokio::spawn(async move { slot.release(&key).await });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn session_slot_is_per_workspace() {
        let slot = SessionSlot::default();
        assert!(slot.try_acquire("w1").await, "first acquire must win");
        assert!(
            !slot.try_acquire("w1").await,
            "second acquire on the same workspace must be refused"
        );
        assert!(
            slot.try_acquire("w2").await,
            "a different workspace may hold its own session (0bv ② B 形态)"
        );
        slot.release("w1").await;
        assert!(slot.try_acquire("w1").await, "release must reopen the slot");
        assert!(slot.is_held("w2").await, "other workspace untouched");
        assert!(slot.any_held().await, "observability: some workspace is held");
    }

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

    #[tokio::test]
    async fn slot_guard_releases_on_drop() {
        let slot = SessionSlot::default();
        let guard = SlotGuard::new(slot.clone(), "w1".to_string());
        assert!(slot.try_acquire("w1").await, "guard must hold the slot first");
        drop(guard);
        // Drop spawns the async release; yield until it lands.
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            if !slot.is_held("w1").await {
                return;
            }
        }
        panic!("slot must be released after guard drop");
    }
}
