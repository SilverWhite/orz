//! App-server JSON-RPC client — the fallback TUI's protocol half (Phase 3
//! slice #12).
//!
//! Speaks the codex app-server wire protocol over an in-process tokio duplex
//! to [`CodexAppServer`] (mirroring how orz-tui's ACP client connects to
//! `AcpServer`, but with the codex framing from
//! `orz_host::codex_app::{JsonMessage, read_json_message, write_json_message}`).
//!
//! Message flow:
//! - outbound requests (`initialize`, `thread/start`, `turn/start`,
//!   `turn/interrupt`, `thread/unsubscribe`) are correlated by JSON-RPC id;
//! - server→client requests (v1: `approval/request`) surface as
//!   [`ClientMsg::ApprovalRequest`] and are answered with
//!   [`CodexClient::answer_approval`] — the reader task stays live while the
//!   dialog awaits the user (same discipline as orz-tui's `TuiClientHandler`);
//! - notifications surface as [`ClientMsg`] variants (streaming items, turn
//!   terminals, thread lifecycle).
//!
//! The client is type-tolerant: unknown notifications and unknown item types
//! are ignored, and a delta arriving for an item the app already sealed is
//! dropped by the app layer (the completion text is authoritative — recorded
//! slice #12 boundary).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use orz_host::codex_app::{
    read_json_message, write_json_message, CodexError, CodexServerParts, JsonMessage,
};
use serde_json::{json, Value};
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot};

/// Upper bound for a request's response (the server answers every request
/// immediately — only the run itself streams over notifications).
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Client-side errors.
#[derive(Debug, thiserror::Error)]
pub enum CodexClientError {
    #[error("request {0} timed out after {REQUEST_TIMEOUT:?}")]
    Timeout(String),
    #[error("connection to the codex server closed")]
    Closed,
    #[error("no active thread — start one first")]
    NoThread,
    #[error("no active turn")]
    NoTurn,
    #[error("server error {code}: {message}")]
    Server { code: i64, message: String },
    #[error("codex error: {0}")]
    Codex(#[from] CodexError),
}

/// Server→client messages surfaced to the app.
#[derive(Debug)]
pub enum ClientMsg {
    /// Permission request (server→client request); answer via
    /// [`CodexClient::answer_approval`].
    ApprovalRequest { id: Value, params: Value },
    /// Streamed agent text (`item/started` with empty text or `item/delta`).
    ItemDelta {
        thread_id: String,
        turn_id: String,
        item_id: String,
        text: String,
    },
    /// A completed item (`kind` = item type, e.g. `agentMessage`).
    ItemCompleted {
        thread_id: String,
        turn_id: String,
        item_id: String,
        text: String,
        kind: String,
    },
    /// A turn reached its terminal state (`completed`/`interrupted`/`failed`).
    TurnCompleted {
        turn_id: String,
        status: String,
        error: Option<String>,
    },
    ThreadStarted { thread_id: String },
    ThreadClosed { thread_id: String },
}

/// In-process app-server JSON-RPC client.
pub struct CodexClient {
    /// Outbound frames (writer task).
    tx: mpsc::UnboundedSender<JsonMessage>,
    /// Request/response correlation (our requests' ids).
    pending: Arc<Mutex<HashMap<Value, oneshot::Sender<Value>>>>,
    pub msg_rx: mpsc::UnboundedReceiver<ClientMsg>,
    next_id: Arc<AtomicU64>,
    /// The active thread id (set by `initialize_and_start_thread`).
    pub thread_id: Option<String>,
    /// The active turn id (set by `start_turn`).
    pub turn_id: Option<String>,
}

impl CodexClient {
    /// Connect over crossed tokio duplexes to a `CodexAppServer` running on
    /// the same LocalSet. Must be called inside a `tokio::task::LocalSet`
    /// (the server and its tasks use `spawn_local`).
    pub fn connect_inprocess(parts: CodexServerParts) -> Self {
        let (client_side, server_side) = tokio::io::duplex(1 << 16);
        tokio::task::spawn_local(async move {
            let _ = orz_host::codex_app::run_codex_server(parts, server_side).await;
        });
        let (read, write) = tokio::io::split(client_side);
        let (tx, mut out_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));

        // Writer task: frames → wire.
        tokio::task::spawn_local(async move {
            let mut w = write;
            while let Some(msg) = out_rx.recv().await {
                if write_json_message(&mut w, &msg).await.is_err() {
                    break;
                }
            }
        });
        // Reader task: wire → responses (pending) + ClientMsg. Stays live
        // while an approval dialog awaits the user.
        let pending_r = pending.clone();
        let msg_r = msg_tx.clone();
        tokio::task::spawn_local(async move {
            let mut r = BufReader::new(read);
            loop {
                match read_json_message(&mut r).await {
                    Ok(Some(msg)) => dispatch(&msg, &pending_r, &msg_r),
                    Ok(None) => break,
                    // A malformed server frame is skipped (tolerant); IO
                    // errors end the connection.
                    Err(CodexError::Malformed(e)) => {
                        tracing::debug!("codex client: skipping malformed frame: {e}");
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            tx,
            pending,
            msg_rx,
            next_id,
            thread_id: None,
            turn_id: None,
        }
    }

    /// `initialize` (id 0) → `initialized` → `thread/start` and wait for the
    /// server-generated thread id. Returns the thread id.
    pub async fn initialize_and_start_thread(&mut self) -> Result<String, CodexClientError> {
        let _ = self.call("initialize", json!({})).await?;
        self.tx
            .send(JsonMessage::notify("initialized", json!({})))
            .map_err(|_| CodexClientError::Closed)?;
        let result = self
            .call("thread/start", json!({ "ephemeral": true, "sandbox": "workspace-write" }))
            .await?;
        let thread_id = result
            .get("thread")
            .and_then(|t| t.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| CodexClientError::Server {
                code: -32602,
                message: "thread/start response lacks thread.id".to_owned(),
            })?
            .to_owned();
        self.thread_id = Some(thread_id.clone());
        Ok(thread_id)
    }

    /// Start a turn on the active thread. Returns the turn id.
    pub async fn start_turn(&mut self, text: &str) -> Result<String, CodexClientError> {
        let thread_id = self
            .thread_id
            .as_ref()
            .ok_or(CodexClientError::NoThread)?;
        let result = self
            .call(
                "turn/start",
                json!({
                    "threadId": thread_id,
                    "input": [{ "type": "text", "text": text }]
                }),
            )
            .await?;
        let turn_id = result
            .get("turn")
            .and_then(|t| t.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| CodexClientError::Server {
                code: -32602,
                message: "turn/start response lacks turn.id".to_owned(),
            })?
            .to_owned();
        self.turn_id = Some(turn_id.clone());
        Ok(turn_id)
    }

    /// Interrupt the active turn (the terminal `turn/completed{interrupted}`
    /// notification arrives asynchronously).
    pub async fn interrupt_turn(&self) -> Result<(), CodexClientError> {
        let thread_id = self.thread_id.as_ref().ok_or(CodexClientError::NoThread)?;
        let turn_id = self.turn_id.as_ref().ok_or(CodexClientError::NoTurn)?;
        self.call(
            "turn/interrupt",
            json!({ "threadId": thread_id, "turnId": turn_id }),
        )
        .await?;
        Ok(())
    }

    /// Unsubscribe the thread (server responds, then emits `thread/closed`).
    pub async fn close_thread(&mut self) -> Result<(), CodexClientError> {
        let thread_id = self.thread_id.as_ref().ok_or(CodexClientError::NoThread)?;
        self.call("thread/unsubscribe", json!({ "threadId": thread_id }))
            .await?;
        self.thread_id = None;
        self.turn_id = None;
        Ok(())
    }

    /// Answer an `approval/request` with the simple decision
    /// (`allow_once` / `allow` / `deny`).
    pub fn answer_approval(&self, id: &Value, decision: &str) {
        let _ = self
            .tx
            .send(JsonMessage::response(id.clone(), json!({ "decision": decision })));
    }

    /// Take the next server→client message.
    pub async fn recv(&mut self) -> Option<ClientMsg> {
        self.msg_rx.recv().await
    }

    /// Send a request and await its response (or error response).
    async fn call(&self, method: &str, params: Value) -> Result<Value, CodexClientError> {
        let id = json!(self.next_id.fetch_add(1, Ordering::Relaxed));
        let (respond, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), respond);
        if self
            .tx
            .send(JsonMessage::request(id.clone(), method, params))
            .is_err()
        {
            self.pending.lock().unwrap().remove(&id);
            return Err(CodexClientError::Closed);
        }
        let value = match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(value)) => value,
            Ok(Err(_)) => {
                self.pending.lock().unwrap().remove(&id);
                return Err(CodexClientError::Closed);
            }
            Err(_) => {
                // Drop the pending entry so a late response is a no-op
                // (implementation review P3-3).
                self.pending.lock().unwrap().remove(&id);
                return Err(CodexClientError::Timeout(method.to_owned()));
            }
        };
        // Error responses are wrapped by dispatch; surface them distinctly.
        if let Some(error) = value.get("__error") {
            return Err(CodexClientError::Server {
                code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown server error")
                    .to_owned(),
            });
        }
        Ok(value)
    }
}

/// Route one inbound frame: response → pending oneshot (error responses are
/// wrapped so the caller can surface them); server request → `ApprovalRequest`;
/// notification → typed [`ClientMsg`].
fn dispatch(
    msg: &JsonMessage,
    pending: &Arc<Mutex<HashMap<Value, oneshot::Sender<Value>>>>,
    msg_tx: &mpsc::UnboundedSender<ClientMsg>,
) {
    match (msg.id.clone(), msg.method.as_deref()) {
        (Some(id), None) => {
            if let Some(respond) = pending.lock().unwrap().remove(&id) {
                let payload = match &msg.error {
                    Some(error) => json!({ "__error": error }),
                    None => msg.result.clone().unwrap_or(Value::Null),
                };
                let _ = respond.send(payload);
            }
        }
        (Some(id), Some("approval/request")) => {
            let _ = msg_tx.send(ClientMsg::ApprovalRequest {
                id,
                params: msg.params.clone().unwrap_or(Value::Null),
            });
        }
        (None, Some(method)) => {
            let params = msg.params.clone().unwrap_or(Value::Null);
            let message = match method {
                "thread/started" => params
                    .get("thread")
                    .and_then(|t| t.get("id"))
                    .and_then(Value::as_str)
                    .map(|s| ClientMsg::ThreadStarted {
                        thread_id: s.to_owned(),
                    }),
                "thread/closed" => params
                    .get("threadId")
                    .and_then(Value::as_str)
                    .map(|s| ClientMsg::ThreadClosed {
                        thread_id: s.to_owned(),
                    }),
                "item/started" | "item/delta" => item_parts(&params).map(
                    |(thread_id, turn_id, item_id, text)| ClientMsg::ItemDelta {
                        thread_id,
                        turn_id,
                        item_id,
                        text,
                    },
                ),
                "item/completed" => item_parts(&params).map(
                    |(thread_id, turn_id, item_id, text)| ClientMsg::ItemCompleted {
                        thread_id,
                        turn_id,
                        item_id,
                        text,
                        kind: params
                            .get("item")
                            .and_then(|i| i.get("type"))
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_owned(),
                    },
                ),
                "turn/completed" => turn_completed_msg(&params),
                _ => None,
            };
            if let Some(message) = message {
                let _ = msg_tx.send(message);
            }
        }
        // Unknown server requests / malformed shapes are ignored (tolerant).
        _ => {}
    }
}

/// Extract a turn-terminal message (returns `None` on malformed shapes).
fn turn_completed_msg(params: &Value) -> Option<ClientMsg> {
    let turn = params.get("turn")?;
    Some(ClientMsg::TurnCompleted {
        turn_id: turn.get("id")?.as_str()?.to_owned(),
        status: turn.get("status")?.as_str()?.to_owned(),
        error: turn
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

/// Extract `(threadId, turnId, item.id, item.text)` from an item frame.
fn item_parts(params: &Value) -> Option<(String, String, String, String)> {
    Some((
        params.get("threadId")?.as_str()?.to_owned(),
        params.get("turnId")?.as_str()?.to_owned(),
        params.get("item")?.get("id")?.as_str()?.to_owned(),
        params
            .get("item")?
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_host::acp_server::AcpServer;
    use orz_host::codex_app::CodexAppServer;
    use orz_host::session::TrustPolicy;
    use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
    use orz_loop::gateway::model::ToolCall;
    use std::path::Path;
    use std::sync::atomic::AtomicU32;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn test_dir() -> std::path::PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "orz-codex-client-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn parts(script: Vec<ScriptedResponse>, base: &Path) -> CodexServerParts {
        let acp = Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::new(script))));
        CodexAppServer::new_parts(acp, base.to_path_buf(), TrustPolicy::Skip)
    }

    fn bash_script() -> Vec<ScriptedResponse> {
        vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "run_terminal_cmd".to_string(),
                arguments: serde_json::json!({ "command": "dir" }),
                call_id: "call-1".to_string(),
            }]),
            ScriptedResponse::text("完成（bash 已执行）。"),
            ScriptedResponse::text("完成（bash 已执行）。"),
        ]
    }

    async fn recv_until(client: &mut CodexClient, kind: &str) -> ClientMsg {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                match client.recv().await.expect("client connection alive") {
                    msg if matches_client_msg(&msg, kind) => return msg,
                    _ => {}
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {kind}"))
    }

    /// Whether a ClientMsg matches a short kind label (for drain helpers).
    fn matches_client_msg(msg: &ClientMsg, kind: &str) -> bool {
        matches!(
            (kind, msg),
            ("turn/completed", ClientMsg::TurnCompleted { .. })
                | ("item/completed", ClientMsg::ItemCompleted { .. })
                | ("approval/request", ClientMsg::ApprovalRequest { .. })
                | ("thread/started", ClientMsg::ThreadStarted { .. })
        )
    }

    fn journal_terminal(base: &Path, thread8: &str) -> String {
        let path = base
            .join(".gsa")
            .join("runs")
            .join(format!("RUN-{thread8}-0"))
            .join("events.jsonl");
        let replay = orz_assurance::replay_journal(&path, None, None, true);
        assert!(replay.valid, "journal {path:?} must be valid: {replay:?}");
        replay.terminal_event.expect("terminal event")
    }

    #[tokio::test]
    async fn initialize_starts_thread_and_streams_a_turn_to_completion() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let mut client = CodexClient::connect_inprocess(parts(
                    vec![
                        ScriptedResponse::text("第一轮草稿。"),
                        ScriptedResponse::text("终局答案。"),
                    ],
                    &base,
                ));
                let thread_id = client.initialize_and_start_thread().await.unwrap();
                assert!(thread_id.starts_with("thr-"));
                let started = recv_until(&mut client, "thread/started").await;
                let ClientMsg::ThreadStarted { thread_id: tid } = started else {
                    unreachable!()
                };
                assert_eq!(tid, thread_id);

                let turn_id = client.start_turn("你好").await.unwrap();
                assert_eq!(turn_id, "turn_1");

                // Streamed deltas accumulate; the terminal item seals.
                let mut delta_text = String::new();
                let (text, tid) = loop {
                    match client.recv().await.expect("connection alive") {
                        ClientMsg::ItemDelta { text, .. } => delta_text.push_str(&text),
                        ClientMsg::ItemCompleted { kind, text, turn_id: tid, .. }
                            if kind == "agentMessage" =>
                        {
                            break (text, tid);
                        }
                        _ => {}
                    }
                };
                assert_eq!(text, "终局答案。");
                assert_eq!(tid, turn_id);
                assert!(delta_text.contains("终局答案。"), "{delta_text:?}");

                let terminal = recv_until(&mut client, "turn/completed").await;
                let ClientMsg::TurnCompleted { status, error, .. } = terminal else {
                    unreachable!()
                };
                assert_eq!(status, "completed");
                assert!(error.is_none());
                assert_eq!(journal_terminal(&base, &thread_id.chars().take(8).collect::<String>()), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn approval_request_is_answered_and_tool_executes() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let mut client = CodexClient::connect_inprocess(parts(bash_script(), &base));
                let thread_id = client.initialize_and_start_thread().await.unwrap();
                let turn_id = client.start_turn("运行 dir").await.unwrap();

                let approval = recv_until(&mut client, "approval/request").await;
                let ClientMsg::ApprovalRequest { id, params } = approval else {
                    unreachable!()
                };
                assert_eq!(params["tool_name"], "run_terminal_command");
                assert_eq!(params["bash_command"], "dir");
                client.answer_approval(&id, "allow_once");

                let terminal = recv_until(&mut client, "turn/completed").await;
                let ClientMsg::TurnCompleted { status, .. } = terminal else {
                    unreachable!()
                };
                assert_eq!(status, "completed");
                assert_eq!(turn_id, "turn_1");
                // The tool genuinely executed (journal evidence) — the run
                // dir derives from the first 8 chars of the thread id.
                let suffix: String = thread_id.chars().take(8).collect();
                let events = std::fs::read_to_string(
                    base.join(".gsa")
                        .join("runs")
                        .join(format!("RUN-{suffix}-0"))
                        .join("events.jsonl"),
                )
                .expect("journal must exist");
                assert!(events.contains("\"tool_completed\""));
            })
            .await;
    }

    #[tokio::test]
    async fn interrupt_produces_interrupted_terminal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let mut client = CodexClient::connect_inprocess(parts(
                    vec![
                        ScriptedResponse::text("慢速草稿。"),
                        ScriptedResponse::text("慢速终答。"),
                    ],
                    &base,
                ));
                client.initialize_and_start_thread().await.unwrap();
                client.start_turn("取消我").await.unwrap();
                // Let the run actually start (first chunk), then interrupt.
                let mut saw_delta = false;
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    while !saw_delta {
                        if let ClientMsg::ItemDelta { .. } = client.recv().await.unwrap() {
                            saw_delta = true;
                        }
                    }
                })
                .await
                .expect("streaming begins before interrupt");
                client.interrupt_turn().await.unwrap();
                let terminal = recv_until(&mut client, "turn/completed").await;
                let ClientMsg::TurnCompleted { status, error, .. } = terminal else {
                    unreachable!()
                };
                assert_eq!(status, "interrupted");
                assert!(error.is_none());
            })
            .await;
    }

    #[tokio::test]
    async fn server_errors_surface_as_client_errors() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let mut client = CodexClient::connect_inprocess(parts(vec![], &base));
                // turn/start without a thread → server -32602.
                let err = client.start_turn("x").await.unwrap_err();
                assert!(matches!(err, CodexClientError::NoThread), "{err:?}");
                client.initialize_and_start_thread().await.unwrap();
                // An unknown method → -32601 surfaces as a Server error.
                let err = client
                    .call("nope/nope", json!({}))
                    .await
                    .unwrap_err();
                assert!(
                    matches!(err, CodexClientError::Server { code: -32601, .. }),
                    "{err:?}"
                );
            })
            .await;
    }

    #[tokio::test]
    async fn close_thread_emits_thread_closed() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let mut client = CodexClient::connect_inprocess(parts(vec![], &base));
                client.initialize_and_start_thread().await.unwrap();
                client.close_thread().await.unwrap();
                let closed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    loop {
                        if let msg @ ClientMsg::ThreadClosed { .. } =
                            client.recv().await.expect("connection alive")
                        {
                            return msg;
                        }
                    }
                })
                .await
                .expect("thread/closed arrives");
                assert!(matches!(closed, ClientMsg::ThreadClosed { .. }));
                assert!(client.thread_id.is_none());
            })
            .await;
    }
}
