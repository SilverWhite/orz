//! Codex app-server JSON-RPC surface — the §2.4 fallback protocol door on
//! orz-host, alongside ACP (Phase 3 slice #12).
//!
//! Design (§2.4 of `INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`): the
//! fallback TUI speaks the Codex app-server protocol; the assurance layer
//! (gates/journal/orientation/plan) runs completely and silently underneath.
//! This module is the protocol half — the TUI never sees assurance panels.
//!
//! Wire: bidirectional JSON-RPC 2.0 over a JSONL stream (one message per
//! line), mirroring the OpenAI Codex app-server protocol the repo studied
//! (`assurance/codex_app_server_lifecycle.py`, fixtures under
//! `runtime/fixtures/codex-app-server-lifecycle-v0.1/`). No `jsonrpc`
//! version field, no null padding — the fixture captures carry neither.
//!
//! Implemented surface (all shapes pinned by the fixtures / fake server):
//! - client→server: `initialize` (id 0), `initialized` (notification),
//!   `thread/start {ephemeral, sandbox}` (server-generated `threadId`, or a
//!   client-supplied unknown one), `turn/start {threadId, input}`
//!   (input = `[{type:"text", text}]` blocks, joined), `turn/interrupt
//!   {threadId, turnId}`, `thread/unsubscribe {threadId}`.
//! - server→client: responses (`{"id", "result"}`) plus notifications
//!   `thread/started`, `turn/started`, `item/started`/`item/delta`
//!   `item/completed`, `turn/completed {turn.status ∈ completed|interrupted|
//!   failed, error?}`, `thread/closed`.
//! - extension (documented in the slice audit doc): `approval/request`
//!   server→client request + client `approval/response` — see
//!   [`crate::codex_permission`].
//!
//! Semantics pinned by the repo's protocol research:
//! - the `turn/interrupt` response alone is NOT terminal — the run task
//!   later emits `turn/completed{interrupted}`;
//! - transport EOF is never promoted to a terminal — a spawned turn keeps
//!   running to journal completion (the journal is the source of truth);
//! - one active turn per thread (`-32001` on a second `turn/start`).
//!
//! Decided v1 boundaries (recorded in the slice audit docs):
//! - sandbox (slice #16): `workspace-write` → `PermissionPolicy::Interactive`
//!   (reads auto-allow, mutations prompt); `read-only` →
//!   `PermissionPolicy::ReadOnly` (reads auto-allow, mutations/network are
//!   denied without prompting — "a write never happens"); any other value
//!   (incl. `danger-full-access`, which would need a yolo mode) is a `-32602`
//!   fail-closed reject. Policy is stored per-session (session == thread on
//!   this surface), so Interactive and ReadOnly threads coexist on one server.
//! - thread resume (slice #16 closure): real-protocol "resume" = a second
//!   `turn/start` on the same threadId — already implemented and test-locked
//!   (`interrupt_produces_interrupted_terminal_and_next_turn_succeeds`); a
//!   *second* `thread/start` on a live thread is refused (`-32001`). No
//!   `thread/resume` method exists in the repo's fixtures — adding one would
//!   violate fixture parity, and cross-connection resume would corrupt the
//!   append-only hash-chained journals (P1-2).
//! - `ephemeral` is accepted but informational: journals still write — the
//!   assurance layer runs silently and completely regardless.
//! - turn/item ids are per-thread counters (`turn_1`, `item_1`, …).
//! - thread ids are opaque server-generated strings (`thr-{uuid}` — a random
//!   component keeps separate runs in the same cwd from reusing run dirs,
//!   implementation review P1-2) unless the client supplies one; a *known*
//!   client-supplied id is a `-32001` conflict (a second `thread/start` on a
//!   live thread; `turn/start` is how you resume); a *retired* one is refused
//!   — re-creating it would append onto the old journals.
//! - user input is echoed as an `item/completed {type:"userMessage"}` (only
//!   `agentMessage` items were observed in the repo's captures — the client
//!   is type-tolerant; the TUI also renders its own submissions).
//! - a delta already in the gateway channel when the run returns can land
//!   after `item/completed` — the client drops deltas for sealed item ids
//!   (completion text is authoritative; same spirit as the residual boundary
//!   recorded in controller.rs).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;

use crate::acp_server::{AcpError, AcpServer};
use crate::codex_permission::{CodexPermissionBroker, CodexPermissionTransport};
use crate::permission::PermissionPolicy;
use crate::session::TrustPolicy;

// ── JSON-RPC framing ─────────────────────────────────────────────────────

/// JSON-RPC error codes used by this surface (2.0 reserved + one server
/// conflict code).
pub const PARSE_ERROR: i64 = -32700;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const SERVER_CONFLICT: i64 = -32001;

/// One JSON-LD JSON-RPC message. A message carries exactly one of
/// (request / response / error / notification); `None` fields are omitted
/// from the wire, so serialized frames match the fixture captures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonMessage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

impl JsonMessage {
    pub fn request(id: Value, method: &str, params: Value) -> Self {
        Self {
            id: Some(id),
            method: Some(method.to_owned()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn response(id: Value, result: Value) -> Self {
        Self {
            id: Some(id),
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, code: i64, message: &str) -> Self {
        Self {
            id: Some(id),
            method: None,
            params: None,
            result: None,
            error: Some(json!({ "code": code, "message": message })),
        }
    }

    pub fn notify(method: &str, params: Value) -> Self {
        Self {
            id: None,
            method: Some(method.to_owned()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    /// Request: id + method (the server must answer).
    pub fn is_request(&self) -> bool {
        self.id.is_some() && self.method.is_some()
    }

    /// Response: id, no method (an answer to a server-initiated request —
    /// v1: `approval/response`).
    pub fn is_response(&self) -> bool {
        self.id.is_some() && self.method.is_none()
    }

    /// Notification: method, no id (no reply expected).
    pub fn is_notification(&self) -> bool {
        self.id.is_none() && self.method.is_some()
    }
}

/// Errors from the codex surface layer.
#[derive(Debug, thiserror::Error)]
pub enum CodexError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("malformed JSON-RPC message: {0}")]
    Malformed(String),
    #[error("acp error: {0}")]
    Acp(#[from] AcpError),
}

/// Read one JSONL frame. `None` on EOF (never promoted to a terminal).
pub async fn read_json_message<R: AsyncBufRead + Unpin>(
    r: &mut R,
) -> Result<Option<JsonMessage>, CodexError> {
    let mut line = String::new();
    let n = r.read_line(&mut line).await?;
    if n == 0 {
        return Ok(None);
    }
    serde_json::from_str(line.trim())
        .map(Some)
        .map_err(|e| CodexError::Malformed(format!("{line:?}: {e}")))
}

/// Write one JSONL frame.
pub async fn write_json_message<W: AsyncWrite + Unpin>(
    w: &mut W,
    msg: &JsonMessage,
) -> Result<(), CodexError> {
    let mut buf = serde_json::to_vec(msg).map_err(|e| CodexError::Malformed(e.to_string()))?;
    buf.push(b'\n');
    w.write_all(&buf).await?;
    w.flush().await?;
    Ok(())
}

// ── Server state ─────────────────────────────────────────────────────────

/// Per-thread bookkeeping — single active turn per thread, per-thread
/// turn/item id counters.
struct ThreadEntry {
    /// The running turn's id (`None` = idle). Set at `turn/start`, cleared
    /// by the run task on every completion path (incl. errors), so a second
    /// turn can start on the same thread (fixture `interrupted-then-second-
    /// turn` parity).
    active_turn: Option<String>,
    /// The agent item id of the active turn — allocated at `turn/start`,
    /// used by both the streaming path (deltas) and the completion path.
    agent_item: Option<String>,
    next_turn: u64,
    next_item: u64,
}

impl ThreadEntry {
    fn new() -> Self {
        Self {
            active_turn: None,
            agent_item: None,
            next_turn: 1,
            next_item: 1,
        }
    }
}

/// Server-generated thread ids carry a random component (implementation
/// review P1-2): a deterministic per-process counter would make every
/// `orz-codex` run in the same directory reuse `thr-0` → the next run
/// appends its journal onto the previous run's completed chain (the journal
/// opens in append mode), corrupting the hash chain for every later verifier
/// pass. The stdio surface avoids this with UUID session ids — the codex
/// surface does the same.
fn generate_thread_id() -> String {
    format!("thr-{}", uuid::Uuid::new_v4())
}

/// The Codex app-server handler — composes the ACP machinery
/// ([`AcpServer`]: sessions, runs, cancellation, journals) behind the codex
/// wire protocol.
pub struct CodexAppServer {
    acp: Arc<AcpServer>,
    cwd: PathBuf,
    trust_policy: TrustPolicy,
    tx_out: mpsc::UnboundedSender<JsonMessage>,
    broker: Arc<CodexPermissionBroker>,
    threads: Arc<Mutex<HashMap<String, ThreadEntry>>>,
    /// Client-supplied thread ids that were unsubscribed — re-creating them
    /// would restart `prompt_count` at 0 and append new runs onto the old
    /// session's journals (same hash-chain corruption as P1-2). Refused with
    /// `-32001` (implementation review P1-2, path b).
    retired_threads: Arc<Mutex<std::collections::HashSet<String>>>,
}

/// Everything a connection needs — the server plus its two inbound sinks
/// (outbound JSON-RPC frames; ACP client messages for text-delta
/// translation).
pub struct CodexServerParts {
    pub server: Arc<CodexAppServer>,
    pub out_rx: mpsc::UnboundedReceiver<JsonMessage>,
    pub gateway_rx: mpsc::UnboundedReceiver<xai_acp_lib::AcpClientMessage>,
}

impl CodexAppServer {
    /// Build the codex surface over an [`AcpServer`]. Wires the interactive
    /// permission transport (approvals over the codex channel) and the ACP
    /// gateway (its receiver feeds the text-delta translation task) onto the
    /// wrapped server immediately — both must be in place before the first
    /// turn.
    pub fn new_parts(
        acp: Arc<AcpServer>,
        cwd: PathBuf,
        trust_policy: TrustPolicy,
    ) -> CodexServerParts {
        Self::new_parts_with_permission_timeout(
            acp,
            cwd,
            trust_policy,
            crate::permission::PERMISSION_PROMPT_TIMEOUT,
        )
    }

    /// `new_parts` with an explicit approval wait bound (tests inject a
    /// short timeout to exercise the fail-closed path).
    pub fn new_parts_with_permission_timeout(
        acp: Arc<AcpServer>,
        cwd: PathBuf,
        trust_policy: TrustPolicy,
        permission_timeout: std::time::Duration,
    ) -> CodexServerParts {
        let (tx_out, out_rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx_out.clone()));
        let server = Arc::new(Self {
            acp,
            cwd,
            trust_policy,
            tx_out,
            broker,
            threads: Arc::new(Mutex::new(HashMap::new())),
            retired_threads: Arc::new(Mutex::new(std::collections::HashSet::new())),
        });
        // Approvals travel over the codex channel (hub path — no ACP-typed
        // gateway generalization needed; slice #12).
        server
            .acp
            .set_hub_permission(Arc::new(CodexPermissionTransport::with_timeout(
                server.broker.clone(),
                permission_timeout,
            )));
        // Text deltas arrive as ACP client messages; the translation task
        // turns them into `item/started`/`item/delta` notifications.
        let (gateway_tx, gateway_rx) = mpsc::unbounded_channel();
        server
            .acp
            .set_gateway(xai_acp_lib::AcpAgentGatewaySender::new(gateway_tx));
        CodexServerParts {
            server,
            out_rx,
            gateway_rx,
        }
    }

    /// The permission broker (the connection loop resolves `approval/response`
    /// messages here).
    pub fn broker(&self) -> Arc<CodexPermissionBroker> {
        self.broker.clone()
    }

    async fn send_response(&self, id: Value, result: Value) {
        let _ = self.tx_out.send(JsonMessage::response(id, result));
    }

    async fn send_notify(&self, method: &str, params: Value) {
        let _ = self.tx_out.send(JsonMessage::notify(method, params));
    }

    /// The active turn's agent item id — created lazily on the first streamed
    /// chunk, so the completion path reuses the same id.
    fn agent_item_for_delta(&self, thread_id: &str) -> Option<(String, String)> {
        let mut threads = self.threads.lock().unwrap();
        let entry = threads.get_mut(thread_id)?;
        let turn_id = entry.active_turn.clone()?;
        // RS-05 (0aq, 2026-09-19, Top-10 #3): the just-assigned value is
        // used directly — no unwrap after the assignment.
        let item = match entry.agent_item.clone() {
            Some(item) => item,
            None => {
                let n = entry.next_item;
                entry.next_item += 1;
                let item = format!("item_{n}");
                entry.agent_item = Some(item.clone());
                item
            }
        };
        Some((turn_id, item))
    }

    /// Dispatch one inbound message. Errors become JSON-RPC error responses;
    /// responses (v1: `approval/response`) resolve pending approvals.
    async fn handle_inbound(self: &Arc<Self>, msg: JsonMessage) {
        match (msg.id, msg.method) {
            (Some(id), None) => {
                let result = msg.result.unwrap_or(Value::Null);
                self.broker.resolve(&id, result);
            }
            (Some(id), Some(method)) => {
                let params = msg.params.unwrap_or(Value::Null);
                if let Err(err) = self.handle_request(&id, &method, params).await {
                    let _ = self
                        .tx_out
                        .send(JsonMessage::error(id, err.code, &err.message));
                }
            }
            (None, Some(method)) => {
                let params = msg.params.unwrap_or(Value::Null);
                self.handle_notification(&method, params).await;
            }
            (None, None) => {
                // Neither id nor method — not a valid JSON-RPC message. The
                // framing layer already rejected malformed lines; ignore.
            }
        }
    }

    async fn handle_request(
        self: &Arc<Self>,
        id: &Value,
        method: &str,
        params: Value,
    ) -> Result<(), DispatchError> {
        match method {
            "initialize" => {
                self.send_response(
                    id.clone(),
                    json!({
                        "userAgent": format!("orz-codex-host/{}", env!("CARGO_PKG_VERSION")),
                        "platformFamily": platform_family(),
                        "platformOs": platform_family(),
                    }),
                )
                .await;
                Ok(())
            }
            "thread/start" => self.handle_thread_start(id, params).await,
            "turn/start" => self.handle_turn_start(id, params).await,
            "turn/interrupt" => self.handle_turn_interrupt(id, params).await,
            "thread/unsubscribe" => self.handle_thread_unsubscribe(id, params).await,
            _ => Err(DispatchError {
                code: METHOD_NOT_FOUND,
                message: format!("method not found: {method}"),
            }),
        }
    }

    async fn handle_notification(&self, _method: &str, _params: Value) {
        // `initialized` is the only inbound notification in the observed
        // surface; unknown notifications are silently ignored (JSON-RPC 2.0).
        // No state is kept — `initialize` order is lenient (audit §5).
    }

    async fn handle_thread_start(&self, id: &Value, params: Value) -> Result<(), DispatchError> {
        // Sandbox (slice #16): workspace-write → interactive prompts;
        // read-only → the ReadOnly permission policy (mutations/network are
        // silently denied). Anything else — including danger-full-access,
        // which would need a yolo mode — fails closed.
        let policy = match params.get("sandbox").and_then(Value::as_str).unwrap_or("") {
            "workspace-write" => PermissionPolicy::Interactive,
            "read-only" => PermissionPolicy::ReadOnly,
            other => {
                return Err(DispatchError::invalid_params(format!(
                    "unsupported sandbox {other:?} — accepted: \"workspace-write\", \"read-only\""
                )));
            }
        };
        // `ephemeral` is accepted but informational: journals still write —
        // the assurance layer runs silently and completely regardless (§2.4).
        let thread_id = match params.get("threadId").and_then(Value::as_str) {
            Some(tid) => {
                if self.threads.lock().unwrap().contains_key(tid) {
                    // Real-protocol "resume" is a second `turn/start` on this
                    // thread (fixture-pinned); a second `thread/start` on a
                    // live thread is refused (slice #16 closure — see module
                    // doc).
                    return Err(DispatchError::conflict(format!(
                        "thread {tid} already exists — a second thread/start on a live thread is refused; turn/start resumes it"
                    )));
                }
                if self.retired_threads.lock().unwrap().contains(tid) {
                    return Err(DispatchError::conflict(format!(
                        "thread {tid} was closed — re-creating it would corrupt its journals (P1-2)"
                    )));
                }
                tid.to_owned()
            }
            None => generate_thread_id(),
        };
        self.acp
            .handle_session_new_with_policy(
                &thread_id,
                Some(self.cwd.clone()),
                self.trust_policy,
                policy,
            )
            .await
            .map_err(|e| DispatchError::invalid_params(e.to_string()))?;
        self.threads
            .lock()
            .unwrap()
            .insert(thread_id.clone(), ThreadEntry::new());
        // Fixture order: response first, then thread/started.
        self.send_response(id.clone(), json!({ "thread": { "id": thread_id } }))
            .await;
        self.send_notify(
            "thread/started",
            json!({
                "thread": { "id": thread_id, "status": { "type": "idle" }, "turns": [] }
            }),
        )
        .await;
        Ok(())
    }

    async fn handle_turn_start(
        self: &Arc<Self>,
        id: &Value,
        params: Value,
    ) -> Result<(), DispatchError> {
        let thread_id = params
            .get("threadId")
            .and_then(Value::as_str)
            .ok_or_else(|| DispatchError::invalid_params("missing threadId"))?;
        // input = [{type:"text", text}, …] — join all text blocks.
        let prompt = extract_prompt(params.get("input"))
            .ok_or_else(|| DispatchError::invalid_params("missing or empty input text blocks"))?;

        let (turn_id, user_item) = {
            let mut threads = self.threads.lock().unwrap();
            let entry = threads.get_mut(thread_id).ok_or_else(|| {
                DispatchError::invalid_params(format!("unknown thread {thread_id}"))
            })?;
            if entry.active_turn.is_some() {
                return Err(DispatchError::conflict(format!(
                    "a turn is already running on thread {thread_id}"
                )));
            }
            let turn_id = format!("turn_{}", entry.next_turn);
            entry.next_turn += 1;
            entry.active_turn = Some(turn_id.clone());
            // The agent item id is allocated now too, so the streaming and
            // completion paths agree even when no delta ever arrives.
            let user_item = format!("item_{}", entry.next_item);
            entry.next_item += 1;
            let agent_item = format!("item_{}", entry.next_item);
            entry.next_item += 1;
            entry.agent_item = Some(agent_item);
            (turn_id, user_item)
        };

        // Fixture order: response → turn/started.
        let turn = json!({ "id": turn_id, "status": "inProgress", "items": [], "error": null });
        self.send_response(id.clone(), json!({ "turn": turn.clone() }))
            .await;
        self.send_notify(
            "turn/started",
            json!({ "turn": turn, "threadId": thread_id }),
        )
        .await;
        // Echo the user's input as an item (extension beyond the observed
        // base surface — only agentMessage items were captured; the client
        // is type-tolerant and also renders its own submissions).
        self.send_notify(
            "item/completed",
            json!({
                "item": { "id": user_item, "type": "userMessage", "text": prompt },
                "threadId": thread_id,
                "turnId": turn_id,
            }),
        )
        .await;

        // Spawn the run task (spawn_local — the permission manager actor runs
        // on the local executor). The turn keeps running to journal completion
        // even if the connection dies (EOF is never a terminal).
        let this = self.clone();
        let thread = thread_id.to_owned();
        let prompt = prompt.clone();
        tokio::task::spawn_local(async move {
            let outcome = this.acp.handle_session_prompt(&thread, &prompt).await;
            this.finish_turn(&thread, &turn_id, outcome).await;
        });
        Ok(())
    }

    async fn handle_turn_interrupt(&self, id: &Value, params: Value) -> Result<(), DispatchError> {
        let thread_id = params
            .get("threadId")
            .and_then(Value::as_str)
            .ok_or_else(|| DispatchError::invalid_params("missing threadId"))?;
        let turn_id = params.get("turnId").and_then(Value::as_str).unwrap_or("");
        let active = {
            let threads = self.threads.lock().unwrap();
            threads
                .get(thread_id)
                .ok_or_else(|| {
                    DispatchError::invalid_params(format!("unknown thread {thread_id}"))
                })?
                .active_turn
                .clone()
        };
        // Respond immediately — the interrupt response alone is NOT terminal;
        // the run task later emits turn/completed{interrupted} (repo protocol
        // research pins this: `interrupt-without-terminal` fixture).
        if let Some(active_turn) = active {
            if !turn_id.is_empty() && turn_id != active_turn {
                return Err(DispatchError::invalid_params(format!(
                    "turn {turn_id} is not the active turn ({active_turn}) on thread {thread_id}"
                )));
            }
            self.acp.cancel_current_run(thread_id);
        }
        // No running turn → benign no-op (mirrors ACP cancel-when-idle).
        self.send_response(id.clone(), json!({})).await;
        Ok(())
    }

    async fn handle_thread_unsubscribe(
        &self,
        id: &Value,
        params: Value,
    ) -> Result<(), DispatchError> {
        let thread_id = params
            .get("threadId")
            .and_then(Value::as_str)
            .ok_or_else(|| DispatchError::invalid_params("missing threadId"))?;
        {
            let threads = self.threads.lock().unwrap();
            if !threads.contains_key(thread_id) {
                return Err(DispatchError::invalid_params(format!(
                    "unknown thread {thread_id}"
                )));
            }
        }
        self.send_response(id.clone(), json!({ "status": "unsubscribed" }))
            .await;
        self.send_notify("thread/closed", json!({ "threadId": thread_id }))
            .await;
        // A running turn keeps running (journal integrity) — its terminal
        // notifications arrive at a closed thread and are dropped by the
        // client. Closing the session also releases the cancel token, so
        // interrupt after unsubscribe is a no-op; the fallback TUI quits on
        // close anyway. The id is retired: re-creating it would restart
        // prompt_count at 0 and append onto the old journals (P1-2).
        self.threads.lock().unwrap().remove(thread_id);
        self.retired_threads
            .lock()
            .unwrap()
            .insert(thread_id.to_owned());
        self.acp.close_session(thread_id);
        Ok(())
    }

    /// Map a turn's outcome onto the wire terminal and release the thread.
    ///
    /// Journal is the source of truth: the controller already wrote
    /// `run_finished` / `run_cancelled` / `run_failed`; this only translates
    /// the terminal to the codex shape.
    async fn finish_turn(&self, thread_id: &str, turn_id: &str, outcome: Result<Value, AcpError>) {
        let agent_item = {
            let threads = self.threads.lock().unwrap();
            threads
                .get(thread_id)
                .and_then(|e| e.agent_item.clone())
                .unwrap_or_else(|| format!("item-{thread_id}-{turn_id}"))
        };
        match outcome {
            Ok(value) => {
                let mut text = value
                    .get("response")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                // P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §11.2 E9）：
                // 用户侧疲劳提醒——机械追加在模型输出最后、弹给用户；会话
                // 持久化在 ACP 内已完成（不含本附言），不进模型上下文。
                if let Some(notice) = value.get("user_notice").and_then(Value::as_str) {
                    text.push_str("\n\n");
                    text.push_str(notice);
                }
                self.send_notify(
                    "item/completed",
                    json!({
                        "item": { "id": agent_item, "type": "agentMessage", "text": text },
                        "threadId": thread_id,
                        "turnId": turn_id,
                    }),
                )
                .await;
                self.send_notify(
                    "turn/completed",
                    json!({
                        "turn": { "id": turn_id, "status": "completed", "items": [], "error": null }
                    }),
                )
                .await;
            }
            Err(AcpError::AgentLoop(orz_loop::controller::AgentLoopError::Cancelled)) => {
                self.send_notify(
                    "turn/completed",
                    json!({
                        "turn": {
                            "id": turn_id,
                            "status": "interrupted",
                            "items": [],
                            "error": null
                        }
                    }),
                )
                .await;
            }
            Err(e) => {
                self.send_notify(
                    "turn/completed",
                    json!({
                        "turn": {
                            "id": turn_id,
                            "status": "failed",
                            "items": [],
                            "error": { "message": e.to_string() }
                        }
                    }),
                )
                .await;
            }
        }
        // Release the thread for the next turn on every path.
        if let Some(entry) = self.threads.lock().unwrap().get_mut(thread_id) {
            entry.active_turn = None;
            entry.agent_item = None;
        }
    }
}

/// Translate ACP client messages (streamed text deltas) into codex
/// `item/started` + `item/delta` notifications. Non-delta messages (e.g.
/// permission-manager housekeeping on the shared gateway) are ignored.
async fn gateway_translation_task(
    server: Arc<CodexAppServer>,
    mut rx: mpsc::UnboundedReceiver<xai_acp_lib::AcpClientMessage>,
) {
    use xai_acp_lib::AcpClientMessage;
    // (thread, turn) pairs already announced with item/started — keyed by
    // both because turn ids are per-thread counters (`turn_1` on every
    // thread; implementation review P3-1). One entry per turn per
    // connection; unbounded but negligible (design-rationale review note).
    let mut started_turns: std::collections::HashSet<(String, String)> = Default::default();
    while let Some(msg) = rx.recv().await {
        let AcpClientMessage::SessionNotification(args) = msg else {
            continue;
        };
        let notification = args.request;
        let Some(chunk_text) = session_chunk_text(&notification) else {
            continue;
        };
        let thread_id = notification.session_id.0.to_string();
        let Some((turn_id, item_id)) = server.agent_item_for_delta(&thread_id) else {
            // Thread gone (closed mid-turn) — drop the chunk.
            continue;
        };
        if started_turns.insert((thread_id.clone(), turn_id.clone())) {
            server
                .send_notify(
                    "item/started",
                    json!({
                        "item": { "id": item_id, "type": "agentMessage", "text": "" },
                        "threadId": thread_id,
                        "turnId": turn_id,
                    }),
                )
                .await;
        }
        server
            .send_notify(
                "item/delta",
                json!({
                    "item": { "id": item_id, "type": "agentMessage", "text": chunk_text },
                    "threadId": thread_id,
                    "turnId": turn_id,
                }),
            )
            .await;
    }
}

/// Extract the plain text from an ACP session notification's update
/// (`AgentMessageChunk` → `Text` block). Other block kinds yield no text.
fn session_chunk_text(notification: &agent_client_protocol::SessionNotification) -> Option<String> {
    use agent_client_protocol::{ContentBlock, SessionUpdate};
    match &notification.update {
        SessionUpdate::AgentMessageChunk(chunk) => match &chunk.content {
            ContentBlock::Text(t) => Some(t.text.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Errors from request dispatch — always answerable on the wire.
struct DispatchError {
    code: i64,
    message: String,
}

impl DispatchError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: INVALID_PARAMS,
            message: message.into(),
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: SERVER_CONFLICT,
            message: message.into(),
        }
    }
}

/// Extract and join the `{type:"text", text}` blocks of a turn input.
fn extract_prompt(input: Option<&Value>) -> Option<String> {
    let blocks = input?.as_array()?;
    let mut text = String::new();
    for block in blocks {
        if block.get("type").and_then(Value::as_str) == Some("text")
            && let Some(t) = block.get("text").and_then(Value::as_str)
        {
            text.push_str(t);
        }
    }
    (!text.trim().is_empty()).then_some(text)
}

fn platform_family() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

/// Serve the codex surface over a bidirectional transport (in-process tokio
/// duplex for the fallback TUI; stdio for future external clients). Must run
/// inside a `tokio::task::LocalSet` (the permission manager and run tasks
/// use `spawn_local`).
///
/// EOF ends the connection loop but never promotes a terminal — spawned
/// turns keep running to journal completion.
pub async fn run_codex_server(
    parts: CodexServerParts,
    transport: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
) -> Result<(), CodexError> {
    let server = parts.server.clone();
    let (read, write) = tokio::io::split(transport);
    let mut reader = BufReader::new(read);
    // Outbound writer task.
    tokio::task::spawn_local(async move {
        let mut writer = write;
        let mut out_rx = parts.out_rx;
        while let Some(msg) = out_rx.recv().await {
            if let Err(e) = write_json_message(&mut writer, &msg).await {
                tracing::debug!("codex outbound write failed: {e}");
                break;
            }
        }
    });
    // Text-delta translation task.
    tokio::task::spawn_local(gateway_translation_task(server.clone(), parts.gateway_rx));
    // Inbound loop.
    loop {
        match read_json_message(&mut reader).await {
            Ok(Some(msg)) => server.handle_inbound(msg).await,
            Ok(None) => break, // EOF — never promoted to a terminal
            Err(e) => {
                // Malformed line — answer -32700 (id null; the frame was
                // unparseable) and keep the connection alive.
                tracing::debug!("codex inbound parse error: {e}");
                let _ =
                    server
                        .tx_out
                        .send(JsonMessage::error(Value::Null, PARSE_ERROR, "parse error"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_loop::gateway::fake::{FakeProvider, ScriptedResponse};
    use orz_loop::gateway::model::ToolCall;
    use std::path::Path;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Wait bound for in-process stdio turn/completion exchanges. The old
    /// dev-tuned 5s/8s bounds timed out on cold 2-core CI runners under the
    /// serial suite load (RS-01 2026-09-18, run 35334716461: approval test
    /// missed `turn/completed`); 30s keeps a real hang bounded while
    /// tolerating slow machines. Single-sourced across this test module.
    const TURN_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("orz-codex-app-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake(script: Vec<ScriptedResponse>) -> FakeProvider {
        FakeProvider::new(script)
    }

    fn server_with(fake: FakeProvider) -> Arc<AcpServer> {
        // ACAF shadow 默认（signer 存量失败族修复，2026-09-07，同
        // acp_server 测试面约定）：逻辑测试默认 shadow；fail-closed 语义
        // 由专门测试显式开启。生产默认 fail-closed 在下游 crate 测试编译
        // 时生效（orz-loop `#[cfg(test)]` 特例不跨 crate）。
        Arc::new(AcpServer::with_gateway(Arc::new(fake)).with_acaf_fail_closed(false))
    }

    /// MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24): direct 执行面——
    /// `run_terminal_cmd` 直连调用（第 2 轮起模型直接调工作工具；
    /// Interactive 下权限审批在直连调用时触发）。
    fn terminal_call(command: &str, call_id: &str) -> ToolCall {
        ToolCall {
            name: "run_terminal_cmd".to_string(),
            arguments: serde_json::json!({
                "command": command,
                "description": "测试命令",
            }),
            call_id: call_id.to_string(),
        }
    }

    /// 一个完整的 direct 执行回合脚本（THIN-HARNESS-REDESIGN R2a 审查
    /// 处理，2026-08-27：plan 门普适摘除）：
    /// 直连终端调用（权限审批/执行即时触发）→ 开局问询回答（0x S1，软门
    /// 消费）→ 草稿（反例门候选）→ 终答。
    fn console_exec_script(command: &str) -> Vec<ScriptedResponse> {
        vec![
            ScriptedResponse::tool_calls(vec![terminal_call(command, "call-act-1")]),
            ScriptedResponse::text("草稿（等待执行结果）。"),
            ScriptedResponse::text("草稿（等待执行结果）。"),
            ScriptedResponse::text("终答（命令已执行）。"),
        ]
    }

    type TestConn = (
        tokio::io::WriteHalf<tokio::io::DuplexStream>,
        BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    );

    async fn connect(parts: CodexServerParts) -> TestConn {
        let (server_side, client_side) = tokio::io::duplex(1 << 16);
        tokio::task::spawn_local(async move {
            let _ = run_codex_server(parts, server_side).await;
        });
        let (cr, cw) = tokio::io::split(client_side);
        (cw, BufReader::new(cr))
    }

    async fn send<W: AsyncWrite + Unpin>(w: &mut W, msg: &JsonMessage) {
        let mut buf = serde_json::to_vec(msg).unwrap();
        buf.push(b'\n');
        w.write_all(&buf).await.unwrap();
        w.flush().await.unwrap();
    }

    async fn recv<R: AsyncBufRead + Unpin>(r: &mut R) -> JsonMessage {
        let mut line = String::new();
        loop {
            line.clear();
            let n = r.read_line(&mut line).await.expect("read frame line");
            assert!(n > 0, "connection closed unexpectedly");
            if line.trim().is_empty() {
                continue;
            }
            return serde_json::from_str(line.trim()).expect("parse frame");
        }
    }

    /// Skip messages until one with the given method arrives (TURN_WAIT bound).
    async fn recv_until<R: AsyncBufRead + Unpin>(r: &mut R, method: &str) -> JsonMessage {
        tokio::time::timeout(TURN_WAIT, async {
            loop {
                let msg = recv(r).await;
                if msg.method.as_deref() == Some(method) {
                    return msg;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for {method}"))
    }

    /// Skip messages until the response for the given request id arrives.
    async fn recv_until_id<R: AsyncBufRead + Unpin>(r: &mut R, wanted: &Value) -> JsonMessage {
        tokio::time::timeout(TURN_WAIT, async {
            loop {
                let msg = recv(r).await;
                if msg.id.as_ref() == Some(wanted) {
                    return msg;
                }
            }
        })
        .await
        .unwrap_or_else(|_| panic!("timed out waiting for response {wanted}"))
    }

    fn response_of(msg: &JsonMessage) -> Value {
        msg.result.clone().expect("response carries result")
    }

    fn error_code(msg: &JsonMessage) -> i64 {
        msg.error.as_ref().expect("error response carries error")["code"]
            .as_i64()
            .unwrap()
    }

    /// Run directories derive from the session id's first 8 chars
    /// (AcpServer `RUN-{suffix}-{n}`) — the helpers take the full thread id.
    fn run_dir(base: &Path, thread_id: &str, n: u64) -> PathBuf {
        let suffix: String = thread_id.chars().take(8).collect();
        base.join(".gsa")
            .join("runs")
            .join(format!("RUN-{suffix}-{n}"))
            .join("events.jsonl")
    }

    fn journal_terminal(base: &Path, thread_id: &str, n: u64) -> String {
        let path = run_dir(base, thread_id, n);
        let replay = orz_assurance::replay_journal(&path, None, None, true);
        assert!(
            replay.valid,
            "journal {path:?} must be a valid chain: {replay:?}"
        );
        replay.terminal_event.expect("journal has a terminal event")
    }

    fn journal_events(base: &Path, thread_id: &str, n: u64) -> String {
        std::fs::read_to_string(run_dir(base, thread_id, n)).expect("journal file exists")
    }

    async fn initialize_and_start_thread(
        w: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>,
        r: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
        thread_id: &str,
        sandbox: &str,
    ) {
        send(w, &JsonMessage::request(json!(0), "initialize", json!({}))).await;
        let init = recv_until_id(r, &json!(0)).await;
        let init_result = response_of(&init);
        let user_agent = init_result["userAgent"].as_str().unwrap();
        assert!(
            user_agent.starts_with("orz-codex-host/"),
            "fake-server parity userAgent: {user_agent}"
        );
        send(w, &JsonMessage::notify("initialized", json!({}))).await;
        send(
            w,
            &JsonMessage::request(
                json!(1),
                "thread/start",
                json!({ "ephemeral": true, "sandbox": sandbox, "threadId": thread_id }),
            ),
        )
        .await;
        let started = recv_until_id(r, &json!(1)).await;
        assert_eq!(response_of(&started)["thread"]["id"], json!(thread_id));
        // Interleaved streaming from other threads is skipped.
        let notified = recv_until(r, "thread/started").await;
        assert_eq!(
            notified.params.as_ref().unwrap()["thread"]["id"],
            json!(thread_id)
        );
    }

    async fn run_turn(
        w: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>,
        r: &mut BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
        thread_id: &str,
        text: &str,
    ) -> Value {
        send(
            w,
            &JsonMessage::request(
                json!(2),
                "turn/start",
                json!({ "threadId": thread_id, "input": [{ "type": "text", "text": text }] }),
            ),
        )
        .await;
        // Responses and turn/started are skipped through — another thread's
        // streamed traffic may interleave.
        let response = recv_until_id(r, &json!(2)).await;
        let turn = response_of(&response)["turn"].clone();
        assert_eq!(turn["status"], "inProgress");
        recv_until(r, "turn/started").await;
        turn
    }

    #[tokio::test]
    async fn framing_round_trip_and_classification() {
        let request = JsonMessage::request(json!(1), "turn/start", json!({ "a": 1 }));
        let decoded: JsonMessage =
            serde_json::from_slice(&serde_json::to_vec(&request).unwrap()).unwrap();
        assert!(decoded.is_request());
        assert_eq!(decoded.method.as_deref(), Some("turn/start"));
        assert_eq!(decoded.params.as_ref(), Some(&json!({ "a": 1 })));

        let notification = JsonMessage::notify("thread/started", json!({ "t": "x" }));
        let decoded: JsonMessage =
            serde_json::from_slice(&serde_json::to_vec(&notification).unwrap()).unwrap();
        assert!(decoded.is_notification());
        assert!(decoded.id.is_none());

        let response = JsonMessage::response(json!(7), json!({ "ok": true }));
        let decoded: JsonMessage =
            serde_json::from_slice(&serde_json::to_vec(&response).unwrap()).unwrap();
        assert!(decoded.is_response());
        // Fixture parity: no `jsonrpc` version field, no null params padding.
        let raw = serde_json::to_string(&response).unwrap();
        assert!(!raw.contains("jsonrpc"));
        assert!(!raw.contains("params"));
    }

    #[tokio::test]
    async fn malformed_line_gets_parse_error_connection_survives() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts =
                    CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let (mut w, mut r) = connect(parts).await;
                w.write_all(b"{not-json}\n").await.unwrap();
                let msg = recv(&mut r).await;
                // The -32700 error carries `id: null` on the wire; serde
                // deserializes JSON null into `None` (Option short-circuit).
                assert!(msg.id.is_none());
                assert_eq!(error_code(&msg), PARSE_ERROR);
                // The connection survives a malformed line.
                send(
                    &mut w,
                    &JsonMessage::request(json!(0), "initialize", json!({})),
                )
                .await;
                let init = recv(&mut r).await;
                assert!(init.result.is_some());
            })
            .await;
    }

    #[tokio::test]
    async fn unknown_method_and_bad_params_get_error_codes() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let (mut w, mut r) = connect(parts).await;
                send(&mut w, &JsonMessage::request(json!(5), "nope/nope", json!({}))).await;
                let msg = recv(&mut r).await;
                assert_eq!(msg.id, Some(json!(5)));
                assert_eq!(error_code(&msg), METHOD_NOT_FOUND);
                // turn/start on an unknown thread → -32602.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(6),
                        "turn/start",
                        json!({ "threadId": "missing", "input": [{ "type": "text", "text": "x" }] }),
                    ),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(msg.id, Some(json!(6)));
                assert_eq!(error_code(&msg), INVALID_PARAMS);
                // turn/start without input blocks → -32602.
                send(
                    &mut w,
                    &JsonMessage::request(json!(7), "turn/start", json!({ "threadId": "missing" })),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(error_code(&msg), INVALID_PARAMS);
            })
            .await;
    }

    #[tokio::test]
    async fn thread_start_fixture_shapes_and_fail_closed_sandbox() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let (mut w, mut r) = connect(parts).await;

                // danger-full-access → fail-closed -32602 (v1 boundary —
                // would need a yolo mode, still future).
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(1),
                        "thread/start",
                        json!({ "ephemeral": true, "sandbox": "danger-full-access" }),
                    ),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(msg.id, Some(json!(1)));
                assert_eq!(error_code(&msg), INVALID_PARAMS);

                // An unknown sandbox value fails closed too.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(2),
                        "thread/start",
                        json!({ "ephemeral": true, "sandbox": "surprise" }),
                    ),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(error_code(&msg), INVALID_PARAMS);

                // read-only sandbox (slice #16) is now accepted: response then
                // thread/started (fixture order), idle status shape.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(3),
                        "thread/start",
                        json!({ "ephemeral": true, "sandbox": "read-only", "threadId": "thr_ro" }),
                    ),
                )
                .await;
                let response = recv(&mut r).await;
                assert_eq!(
                    response_of(&response),
                    json!({ "thread": { "id": "thr_ro" } })
                );
                let notified = recv(&mut r).await;
                assert_eq!(notified.method.as_deref(), Some("thread/started"));
                assert_eq!(
                    notified.params.as_ref().unwrap()["thread"]["status"]["type"],
                    "idle"
                );

                // workspace-write with a client-supplied id: same shapes.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(4),
                        "thread/start",
                        json!({ "ephemeral": true, "sandbox": "workspace-write", "threadId": "thr_t" }),
                    ),
                )
                .await;
                let response = recv(&mut r).await;
                assert_eq!(response_of(&response), json!({ "thread": { "id": "thr_t" } }));
                let notified = recv(&mut r).await;
                assert_eq!(notified.method.as_deref(), Some("thread/started"));

                // Duplicate client-supplied id → -32001 conflict.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(5),
                        "thread/start",
                        json!({ "ephemeral": true, "sandbox": "workspace-write", "threadId": "thr_t" }),
                    ),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(error_code(&msg), SERVER_CONFLICT);
            })
            .await;
    }

    #[tokio::test]
    async fn turn_happy_path_streams_items_and_completes_with_valid_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let script = vec![
                    // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27)：无
                    // plan 门——直接进入终答文本流。0bi ⑩（2026-09-23）：
                    // 纯文本短答跳过反例门，一轮模型调用即终答。
                    ScriptedResponse::text("终局答案。"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(fake(script)),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_happy", "workspace-write").await;

                let turn = run_turn(&mut w, &mut r, "thr_happy", "你好").await;
                assert_eq!(turn["id"], "turn_1");

                // User input echo (extension type — client tolerant).
                let user_item = recv(&mut r).await;
                assert_eq!(user_item.method.as_deref(), Some("item/completed"));
                assert_eq!(
                    user_item.params.as_ref().unwrap()["item"]["type"],
                    "userMessage"
                );
                assert_eq!(user_item.params.as_ref().unwrap()["item"]["text"], "你好");

                // Streaming: item/started then item/delta chunks, in order.
                let item_started = recv(&mut r).await;
                assert_eq!(item_started.method.as_deref(), Some("item/started"));
                let mut delta_text = String::new();
                loop {
                    let msg = recv(&mut r).await;
                    match msg.method.as_deref() {
                        Some("item/delta") => {
                            let chunk = msg.params.as_ref().unwrap()["item"]["text"]
                                .as_str()
                                .unwrap()
                                .to_owned();
                            assert_eq!(
                                msg.params.as_ref().unwrap()["turnId"],
                                "turn_1",
                                "deltas belong to the active turn"
                            );
                            delta_text.push_str(&chunk);
                        }
                        Some("item/completed") => {
                            let item = msg.params.as_ref().unwrap()["item"].clone();
                            assert_eq!(item["type"], "agentMessage");
                            assert_eq!(item["text"], "终局答案。");
                            break;
                        }
                        other => panic!("unexpected message while streaming: {other:?}"),
                    }
                }
                assert!(
                    delta_text.contains("终局答案。"),
                    "streamed deltas must cover the final answer: {delta_text:?}"
                );

                let completed = recv(&mut r).await;
                assert_eq!(completed.method.as_deref(), Some("turn/completed"));
                assert_eq!(
                    completed.params.as_ref().unwrap()["turn"]["status"],
                    "completed"
                );
                assert_eq!(completed.params.as_ref().unwrap()["turn"]["id"], "turn_1");
                // Fixture parity: error is null in the terminal turn object.
                assert!(completed.params.as_ref().unwrap()["turn"]["error"].is_null());

                assert_eq!(journal_terminal(&base, "thr_happy", 0), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn streamed_chunks_concatenate_to_response_text() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let script = vec![
                    // 0bi ⑩：纯文本短答跳过反例门——单轮文本即终答
                    // （4 字 × chunk_size 3 ⇒ 仍有多段 delta）。
                    ScriptedResponse::text("三字答案"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(fake(script).with_chunk_size(3)),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_chunks", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_chunks", "x").await;
                let mut all = String::new();
                loop {
                    let msg = recv(&mut r).await;
                    match msg.method.as_deref() {
                        Some("item/delta") => {
                            all.push_str(
                                msg.params.as_ref().unwrap()["item"]["text"]
                                    .as_str()
                                    .unwrap(),
                            );
                        }
                        Some("item/completed")
                            if msg.params.as_ref().unwrap()["item"]["type"] == "agentMessage" =>
                        {
                            break;
                        }
                        _ => {}
                    }
                }
                assert!(
                    all.contains("三字答案"),
                    "chunks must cover the answer: {all:?}"
                );
                recv_until(&mut r, "turn/completed").await;
            })
            .await;
    }

    #[tokio::test]
    async fn interrupt_produces_interrupted_terminal_and_next_turn_succeeds() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let script = vec![
                    // 两回合各需 [草稿, 终答]；中断发生在首回合流式期间
                    // （约消费 1-2 项），剩余项须让第二回合仍能走完。
                    ScriptedResponse::text("慢速草稿。"),
                    ScriptedResponse::text("慢速终答。"),
                    ScriptedResponse::text("慢速草稿。"),
                    ScriptedResponse::text("慢速终答。"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(
                        fake(script).with_chunk_delay(std::time::Duration::from_millis(60)),
                    ),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_int", "workspace-write").await;
                let turn = run_turn(&mut w, &mut r, "thr_int", "取消我").await;

                // Interrupt mid-run: immediate result, terminal comes later.
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(9),
                        "turn/interrupt",
                        json!({ "threadId": "thr_int", "turnId": turn["id"] }),
                    ),
                )
                .await;
                // The user item and streamed deltas interleave — skip to the
                // interrupt acknowledgement (id 9).
                let ack = recv_until_id(&mut r, &json!(9)).await;
                assert_eq!(response_of(&ack), json!({}));
                // The interrupt response alone is NOT terminal.
                let completed = recv_until(&mut r, "turn/completed").await;
                assert_eq!(
                    completed.params.as_ref().unwrap()["turn"]["status"],
                    "interrupted"
                );
                assert_eq!(journal_terminal(&base, "thr_int", 0), "run_cancelled");

                // Second turn on the same thread succeeds (fixture
                // interrupted-then-second-turn parity).
                let turn2 = run_turn(&mut w, &mut r, "thr_int", "再来").await;
                assert_eq!(turn2["id"], "turn_2");
                recv_until(&mut r, "turn/completed").await;
                assert_eq!(journal_terminal(&base, "thr_int", 1), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn interrupt_when_idle_is_benign_noop() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts =
                    CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_idle", "workspace-write").await;
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(4),
                        "turn/interrupt",
                        json!({ "threadId": "thr_idle", "turnId": "turn_9" }),
                    ),
                )
                .await;
                let ack = recv(&mut r).await;
                assert_eq!(response_of(&ack), json!({}));
                // No terminal notification follows.
                tokio::time::timeout(std::time::Duration::from_millis(150), async {
                    let msg = recv(&mut r).await;
                    panic!("unexpected message while idle: {msg:?}");
                })
                .await
                .expect_err("idle interrupt must not emit a terminal");
            })
            .await;
    }

    #[tokio::test]
    async fn second_turn_while_running_is_conflict() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(
                    server_with(
                        fake(vec![
                            ScriptedResponse::text("慢草稿"),
                            ScriptedResponse::text("慢终答"),
                        ])
                        .with_chunk_delay(std::time::Duration::from_millis(60)),
                    ),
                    base,
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_busy", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_busy", "占线").await;
                // turn/start while a turn runs → -32001 (skip the running
                // turn's streamed traffic to the conflict response).
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(5),
                        "turn/start",
                        json!({ "threadId": "thr_busy", "input": [{ "type": "text", "text": "x" }] }),
                    ),
                )
                .await;
                let msg = recv_until_id(&mut r, &json!(5)).await;
                assert_eq!(error_code(&msg), SERVER_CONFLICT);
                // Drain to the terminal so the LocalSet has no pending turns.
                recv_until(&mut r, "turn/completed").await;
            })
            .await;
    }

    #[tokio::test]
    async fn failed_turn_maps_to_failed_terminal_with_error() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Empty script: the turn's single model round exhausts it
                // → run fails. (0bi ⑩: a text-only short answer skips the
                // counterexample gate, so the old "gate round consumes the
                // one scripted text, next round exhausts" no longer applies
                // — there is no gate round to burn a script item.)
                let parts = CodexAppServer::new_parts(
                    server_with(fake(vec![])),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_fail", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_fail", "会失败").await;
                let completed = recv_until(&mut r, "turn/completed").await;
                let turn = completed.params.as_ref().unwrap()["turn"].clone();
                assert_eq!(turn["status"], "failed");
                assert!(
                    turn["error"]["message"]
                        .as_str()
                        .unwrap()
                        .contains("script exhausted"),
                    "error message must surface the cause: {turn}"
                );
                assert_eq!(journal_terminal(&base, "thr_fail", 0), "run_failed");
            })
            .await;
    }

    #[tokio::test]
    async fn unsubscribe_closes_thread_and_later_turn_is_rejected() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let server = parts.server.clone();
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_close", "workspace-write").await;

                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(3),
                        "thread/unsubscribe",
                        json!({ "threadId": "thr_close" }),
                    ),
                )
                .await;
                let ack = recv(&mut r).await;
                assert_eq!(response_of(&ack), json!({ "status": "unsubscribed" }));
                let closed = recv(&mut r).await;
                assert_eq!(closed.method.as_deref(), Some("thread/closed"));
                assert_eq!(closed.params.as_ref().unwrap()["threadId"], "thr_close");

                // Session is gone from the ACP server and the thread table.
                assert!(!server.acp.list_sessions().contains(&"thr_close".to_string()));
                send(
                    &mut w,
                    &JsonMessage::request(
                        json!(4),
                        "turn/start",
                        json!({ "threadId": "thr_close", "input": [{ "type": "text", "text": "x" }] }),
                    ),
                )
                .await;
                let msg = recv(&mut r).await;
                assert_eq!(error_code(&msg), INVALID_PARAMS);
            })
            .await;
    }

    #[tokio::test]
    async fn approval_allow_once_executes_tool_and_journals() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(
                    server_with(fake(console_exec_script("dir"))),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_perm", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_perm", "运行 dir").await;

                // The hub payload arrives as a server→client request (skip
                // the user-item echo queued before it).
                let approval = recv_until(&mut r, "approval/request").await;
                assert_eq!(approval.method.as_deref(), Some("approval/request"));
                let id = approval.id.clone().expect("approval carries id");
                let params = approval.params.as_ref().unwrap();
                assert_eq!(params["tool_call_id"], "call-run_terminal_cmd");
                assert_eq!(params["tool_name"], "run_terminal_command");
                assert_eq!(params["bash_command"], "dir");
                assert_eq!(params["scope"], "write");

                // Allow once → the tool executes.
                send(
                    &mut w,
                    &JsonMessage::response(id, json!({ "decision": "allow_once" })),
                )
                .await;
                let completed = recv_until(&mut r, "turn/completed").await;
                assert_eq!(
                    completed.params.as_ref().unwrap()["turn"]["status"],
                    "completed"
                );

                let events = journal_events(&base, "thr_perm", 0);
                assert!(
                    events.contains("\"tool_started\""),
                    "tool executed: {events}"
                );
                assert!(events.contains("\"tool_completed\""));
                assert!(
                    !events.contains("Tool not found"),
                    "the tool must genuinely execute: {events}"
                );
                assert_eq!(journal_terminal(&base, "thr_perm", 0), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn approval_deny_never_starts_tool() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(
                    server_with(fake(console_exec_script("dir"))),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_deny", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_deny", "运行 dir").await;
                let approval = recv_until(&mut r, "approval/request").await;
                send(
                    &mut w,
                    &JsonMessage::response(
                        approval.id.clone().unwrap(),
                        json!({ "decision": "deny" }),
                    ),
                )
                .await;
                recv_until(&mut r, "turn/completed").await;
                let events = journal_events(&base, "thr_deny", 0);
                assert!(
                    !events.contains(
                        "\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-1\",\"exit_code\""
                    ),
                    "denied tool must never start: {events}"
                );
                assert!(events.contains("\"deny\""), "denial is journaled: {events}");
            })
            .await;
    }

    #[tokio::test]
    async fn read_only_sandbox_allows_read_and_denies_mutation_without_prompt() {
        // Slice #16 read-only sandbox: reads auto-allow and execute; the
        // mutation is denied by the policy BEFORE the hub sees it — no
        // approval/request ever reaches the wire ("a write never happens").
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                std::fs::write(base.join("a.txt"), "hello").unwrap();
                let script = vec![
                    ScriptedResponse::tool_calls(vec![
                        ToolCall {
                            name: "read_file".to_string(),
                            arguments: serde_json::json!({ "target_file": "a.txt" }),
                            call_id: "call-1".to_string(),
                        },
                        terminal_call("dir", "call-act-1"),
                    ]),
                    ScriptedResponse::text("完成（只读沙箱）。"),
                    ScriptedResponse::text("完成（只读沙箱）。"),
                    // 0x S1：首个动作批次结束后的开局问询回答轮（软门消费）。
                    ScriptedResponse::text("完成（只读沙箱）。"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(fake(script)),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_ro", "read-only").await;
                run_turn(&mut w, &mut r, "thr_ro", "读文件").await;

                // Drain to the terminal, asserting no approval/request ever
                // appears on the wire. Any terminal (incl. failed) breaks the
                // loop — a run failure asserts below instead of timing out.
                let mut approval_count = 0u32;
                let mut status = String::new();
                tokio::time::timeout(TURN_WAIT, async {
                    loop {
                        let msg = recv(&mut r).await;
                        if msg.method.as_deref() == Some("approval/request") {
                            approval_count += 1;
                        }
                        if msg.method.as_deref() == Some("turn/completed") {
                            status = msg.params.as_ref().unwrap()["turn"]["status"]
                                .as_str()
                                .unwrap()
                                .to_owned();
                            break;
                        }
                    }
                })
                .await
                .expect("turn terminates");
                assert_eq!(
                    approval_count, 0,
                    "read-only sandbox must deny mutations without prompting"
                );
                assert_eq!(status, "completed", "run must complete after the denial");

                let events = journal_events(&base, "thr_ro", 0);
                assert!(
                    !events.contains(
                        "\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-1\",\"exit_code\""
                    ),
                    "the mutation order must never start the tool: {events}"
                );
                assert!(events.contains("\"read_file\""), "read executed: {events}");
                assert!(
                    events.contains("\"tool\":\"run_terminal_cmd\",\"risk\":\"LocalMutation\""),
                    "the mutation was requested: {events}"
                );
                assert!(events.contains("\"deny\""), "denial journaled: {events}");
                assert_eq!(journal_terminal(&base, "thr_ro", 0), "run_finished");
            })
            .await;
    }

    // NOTE (slice #16 test adaptation, final): the NetworkCall deny ruling
    // (web_fetch/web_search denied under read-only) is pinned at the bridge
    // unit level — `permission::tests::read_only_policy_denies_non_read_before_manager`.
    // An app-level E2E is architecturally infeasible: web tools are
    // retrieval-shaped and `route()` (orz-loop/relay.rs) sends them to the
    // ExternalRetrieval subagent BEFORE `run_host_tool` — the permission
    // bridge is never reached for them (verified: a scripted `web_fetch`
    // call fails in the subagent path, never reaching the policy gate; the
    // design review's "permission precedes dispatch" claim holds for
    // Host-class tools only). Recorded in the slice audit doc §4.

    #[tokio::test]
    async fn read_only_and_workspace_write_threads_coexist() {
        // One server, two threads with different sandboxes: the
        // workspace-write thread prompts exactly once (answered allow-once);
        // the read-only thread's same-class tool is denied without prompting.
        // Exactly one approval total, and it belongs to the ww thread.
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Interleave-safe for concurrent pulls from the shared
                // gateway：两线程各需 [直调, 开局问询回答, 草稿, 终答]
                // （0x S1：首个动作批次结束后多一轮开局问询回答）——
                // A、B 交错消费（沿用原 A calls → B calls → 文本轮 的实证
                // 模式；文本轮内容不参与断言，池内轮换取用）。
                let script = vec![
                    ScriptedResponse::tool_calls(vec![terminal_call("echo ro", "call-act-ro")]),
                    ScriptedResponse::tool_calls(vec![terminal_call("echo ww", "call-act-ww")]),
                    ScriptedResponse::text("草稿 A"),
                    ScriptedResponse::text("草稿 B"),
                    ScriptedResponse::text("终答 A"),
                    ScriptedResponse::text("终答 B"),
                    ScriptedResponse::text("补轮 A"),
                    ScriptedResponse::text("补轮 B"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(
                        fake(script).with_chunk_delay(std::time::Duration::from_millis(40)),
                    ),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_ro", "read-only").await;
                initialize_and_start_thread(&mut w, &mut r, "thr_ww", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_ro", "一").await;
                run_turn(&mut w, &mut r, "thr_ww", "二").await;

                // Drain to both terminals; answer the single approval the
                // moment it arrives (a pending approval blocks its turn).
                let mut approvals: Vec<JsonMessage> = Vec::new();
                let mut completed = 0u32;
                tokio::time::timeout(TURN_WAIT, async {
                    while completed < 2 {
                        let msg = recv(&mut r).await;
                        if msg.method.as_deref() == Some("approval/request") {
                            let id = msg.id.clone();
                            approvals.push(msg);
                            send(
                                &mut w,
                                &JsonMessage::response(
                                    id.unwrap(),
                                    json!({ "decision": "allow_once" }),
                                ),
                            )
                            .await;
                        } else if msg.method.as_deref() == Some("turn/completed")
                            && msg.params.as_ref().unwrap()["turn"]["status"] == "completed"
                        {
                            completed += 1;
                        }
                    }
                })
                .await
                .expect("both turns complete");
                assert_eq!(
                    approvals.len(),
                    1,
                    "exactly one approval across the two sandboxes"
                );
                assert_eq!(
                    approvals[0].params.as_ref().unwrap()["bash_command"],
                    "echo ww",
                    "the approval belongs to the workspace-write thread"
                );

                // Journals: the read-only thread never started its tool;
                // the workspace-write thread executed it.
                let ro_events = journal_events(&base, "thr_ro", 0);
                assert!(
                    !ro_events.contains(
                        "\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-1\",\"exit_code\""
                    ),
                    "read-only thread never writes: {ro_events}"
                );
                assert!(
                    ro_events.contains("\"deny\""),
                    "denial journaled: {ro_events}"
                );
                assert_eq!(journal_terminal(&base, "thr_ro", 0), "run_finished");
                let ww_events = journal_events(&base, "thr_ww", 0);
                assert!(
                    ww_events.contains("\"tool_started\""),
                    "workspace-write thread executed: {ww_events}"
                );
                assert_eq!(journal_terminal(&base, "thr_ww", 0), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn approval_allow_persists_for_identical_bash() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Two identical bash calls: the first prompts (always_allow),
                // the second auto-allows via the persisted grant — the second
                // identical bash must NOT prompt again.
                let script = vec![
                    // 两次完全相同的工作区订单（命令参数一致）——发放期
                    // 权限桥按相同 access 命中持久授权。
                    ScriptedResponse::tool_calls(vec![terminal_call("dir", "call-act-1")]),
                    ScriptedResponse::tool_calls(vec![terminal_call("dir", "call-act-2")]),
                    ScriptedResponse::text("两轮工具执行完成。"),
                    ScriptedResponse::text("两轮工具执行完成。"),
                ];
                let parts = CodexAppServer::new_parts(
                    server_with(fake(script)),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_always", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_always", "两次 dir").await;

                let approval = recv_until(&mut r, "approval/request").await;
                send(
                    &mut w,
                    &JsonMessage::response(
                        approval.id.clone().unwrap(),
                        json!({ "decision": "allow" }),
                    ),
                )
                .await;
                // 第二笔相同订单必须经持久授权自动放行——wire 上只应出现
                // 1 次 approval/request（若再提示则无应答，权限超时后订单
                // 拒绝，turn 不会正常 completed）。
                let mut wire_approvals = 1u32;
                let completed = loop {
                    let msg = recv(&mut r).await;
                    if msg.method.as_deref() == Some("approval/request") {
                        wire_approvals += 1;
                    }
                    if msg.method.as_deref() == Some("turn/completed") {
                        break msg;
                    }
                };
                assert_eq!(
                    completed.params.as_ref().unwrap()["turn"]["status"],
                    "completed"
                );
                assert_eq!(
                    wire_approvals, 1,
                    "the second identical order must auto-allow via the persisted grant"
                );
                let events = journal_events(&base, "thr_always", 0);
                // 两笔直连终端调用都真实执行（tool_completed 带 exit_code=0）。
                assert_eq!(
                    events
                        .matches("\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-1\",\"exit_code\":0")
                        .count(),
                    1,
                    "{events}"
                );
                assert_eq!(
                    events
                        .matches("\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-2\",\"exit_code\":0")
                        .count(),
                    1,
                    "{events}"
                );
            })
            .await;
    }

    #[tokio::test]
    async fn approval_timeout_fails_closed_with_denial() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts_with_permission_timeout(
                    server_with(fake(console_exec_script("dir"))),
                    base.clone(),
                    TrustPolicy::Skip,
                    std::time::Duration::from_millis(80),
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_to", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_to", "运行 dir").await;
                let _approval = recv_until(&mut r, "approval/request").await;
                // Never answer — the transport times out and fails closed.
                recv_until(&mut r, "turn/completed").await;
                let events = journal_events(&base, "thr_to", 0);
                assert!(
                    !events.contains(
                        "\"tool\":\"run_terminal_cmd\",\"call_id\":\"call-act-1\",\"exit_code\""
                    ),
                    "{events}"
                );
                assert!(events.contains("\"deny\""), "timeout must deny: {events}");
            })
            .await;
    }

    #[tokio::test]
    async fn eof_mid_turn_keeps_turn_running_to_valid_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(
                    server_with(
                        fake(vec![
                            ScriptedResponse::text("慢草稿"),
                            ScriptedResponse::text("慢终答"),
                        ])
                        .with_chunk_delay(std::time::Duration::from_millis(80)),
                    ),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_eof", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_eof", "别断开").await;
                // Drop the client transport mid-turn (EOF — never a terminal).
                drop(r);
                drop(w);
                // EOF 收尾是异步的：journal 写线程需要时间落盘到 run_finished
                // 终态。固定 900ms 睡眠是开发机调参值，2 核 CI runner 高载下
                // 等不到（run 35341286834 实证，ORZ-DEV-TUNED-BOUND-001 同族
                // 第三例）——改为轮询至终态，TURN_WAIT 封顶。
                let journal_path = base
                    .join(".gsa")
                    .join("runs")
                    .join("RUN-thr_eof-0")
                    .join("events.jsonl");
                let deadline = tokio::time::Instant::now() + TURN_WAIT;
                let replay = loop {
                    let replay = orz_assurance::replay_journal(&journal_path, None, None, true);
                    if replay.valid && replay.terminal_event.as_deref() == Some("run_finished") {
                        break replay;
                    }
                    assert!(
                        tokio::time::Instant::now() < deadline,
                        "journal did not reach run_finished in time: {replay:?}"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                };
                assert!(
                    replay.valid,
                    "journal must be valid despite EOF: {replay:?}"
                );
                assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
            })
            .await;
    }

    #[tokio::test]
    async fn generated_thread_ids_are_unique_across_servers_in_same_cwd() {
        // Implementation review P1-2: a deterministic per-process counter
        // would make two orz-codex runs in the same directory both create
        // `thr-0` — the second appends its journal onto the first's
        // completed chain (append-mode journal), corrupting the hash chain.
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let script = vec![
                    ScriptedResponse::text("第一轮草稿。"),
                    ScriptedResponse::text("第一轮终答。"),
                ];
                let parts_a = CodexAppServer::new_parts(server_with(fake(script.clone())), base.clone(), TrustPolicy::Skip);
                let parts_b = CodexAppServer::new_parts(server_with(fake(script)), base.clone(), TrustPolicy::Skip);
                let (mut wa, mut ra) = connect(parts_a).await;
                let (mut wb, mut rb) = connect(parts_b).await;

                // Both servers generate their own thread id.
                send(&mut wa, &JsonMessage::request(json!(1), "thread/start", json!({ "ephemeral": true, "sandbox": "workspace-write" }))).await;
                let resp_a = recv(&mut ra).await;
                let tid_a = response_of(&resp_a)["thread"]["id"].as_str().unwrap().to_owned();
                send(&mut wb, &JsonMessage::request(json!(1), "thread/start", json!({ "ephemeral": true, "sandbox": "workspace-write" }))).await;
                let resp_b = recv(&mut rb).await;
                let tid_b = response_of(&resp_b)["thread"]["id"].as_str().unwrap().to_owned();
                assert_ne!(tid_a, tid_b, "generated thread ids must be unique");

                // Both runs complete into their own journals — each chain is
                // independently valid (no append-into-old-chain corruption).
                for (w, r, tid) in [(&mut wa, &mut ra, &tid_a), (&mut wb, &mut rb, &tid_b)] {
                    send(
                        w,
                        &JsonMessage::request(
                            json!(2),
                            "turn/start",
                            json!({ "threadId": tid, "input": [{ "type": "text", "text": "你好" }] }),
                        ),
                    )
                    .await;
                    recv(r).await; // response
                    recv(r).await; // turn/started
                    recv_until(r, "turn/completed").await;
                }
                assert_eq!(journal_terminal(&base, &tid_a.chars().take(8).collect::<String>(), 0), "run_finished");
                assert_eq!(journal_terminal(&base, &tid_b.chars().take(8).collect::<String>(), 0), "run_finished");
            })
            .await;
    }

    #[tokio::test]
    async fn retired_thread_id_recreation_is_refused() {
        // Implementation review P1-2 (path b): re-creating a closed thread
        // would restart prompt_count at 0 and append onto the old journals.
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let parts = CodexAppServer::new_parts(server_with(fake(vec![])), base, TrustPolicy::Skip);
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_retire", "workspace-write").await;
                send(&mut w, &JsonMessage::request(json!(3), "thread/unsubscribe", json!({ "threadId": "thr_retire" }))).await;
                recv(&mut r).await; // unsubscribe response
                recv(&mut r).await; // thread/closed
                // Re-creating the same id is a conflict, not a silent restart.
                send(&mut w, &JsonMessage::request(json!(4), "thread/start", json!({ "ephemeral": true, "sandbox": "workspace-write", "threadId": "thr_retire" }))).await;
                let msg = recv(&mut r).await;
                assert_eq!(error_code(&msg), SERVER_CONFLICT);
            })
            .await;
    }

    #[tokio::test]
    async fn concurrent_turns_on_different_threads_have_distinct_journals() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Both threads share ONE model gateway — the script must be
                // interleave-safe：每回合各需 [草稿, 终答] 两轮；
                // [D, D, F, F] 与 A、B 交替消费匹配。终答内容不断言——
                // 只断言完成与 journal 有效性。
                let parts = CodexAppServer::new_parts(
                    server_with(
                        fake(vec![
                            ScriptedResponse::text("共享草稿"),
                            ScriptedResponse::text("共享草稿"),
                            ScriptedResponse::text("共享终答"),
                            ScriptedResponse::text("共享终答"),
                        ])
                        .with_chunk_delay(std::time::Duration::from_millis(40)),
                    ),
                    base.clone(),
                    TrustPolicy::Skip,
                );
                let (mut w, mut r) = connect(parts).await;
                initialize_and_start_thread(&mut w, &mut r, "thr_one", "workspace-write").await;
                initialize_and_start_thread(&mut w, &mut r, "thr_two", "workspace-write").await;
                run_turn(&mut w, &mut r, "thr_one", "一").await;
                run_turn(&mut w, &mut r, "thr_two", "二").await;

                // Both turns complete (order free — drain until two terminal
                // notifications with status completed arrive).
                let mut completed_count = 0u32;
                tokio::time::timeout(TURN_WAIT, async {
                    while completed_count < 2 {
                        let msg = recv_until(&mut r, "turn/completed").await;
                        if msg.params.as_ref().unwrap()["turn"]["status"] == "completed" {
                            completed_count += 1;
                        }
                    }
                })
                .await
                .expect("both concurrent turns complete");
                assert_eq!(journal_terminal(&base, "thr_one", 0), "run_finished");
                assert_eq!(journal_terminal(&base, "thr_two", 0), "run_finished");
            })
            .await;
    }
}
