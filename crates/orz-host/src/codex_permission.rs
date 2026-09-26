//! Codex app-server permission bridge — interactive approvals over the
//! app-server JSON-RPC channel (Phase 3 slice #12, design §2.4).
//!
//! The permission manager's interactive prompts normally travel over the ACP
//! gateway to the orz-tui dialog. For the codex surface the manager is wired
//! to a [`PermissionHookTransport`] instead (`AcpServer::set_hub_permission`
//! → `OrzHost::with_bridge_and_hub`): the transport turns each prompt into a
//! server→client `approval/request` JSON-RPC request and awaits the client's
//! `approval/response` (correlated by JSON-RPC id).
//!
//! Wire extension (documented in `docs/CODEX_FALLBACK_TUI_SLICE_12`): this
//! approval pair extends the observed codex app-server base surface (the
//! repo's captures covered thread/turn/item lifecycle only). The request
//! params are the hub's `build_permission_payload` shape (`tool_call_id`,
//! `tool_name`, `description`, `scope`, plus `bash_command`/`edit_file_paths`
//! context); the response result is the client's simple decision
//! `{"decision": "allow_once" | "allow" | "deny"}` — the host owns the hub
//! semantics (scope kinds, AllowEditsForSession), not the client.
//!
//! Fail-closed: a timeout or a dropped connection rejects the tool call (the
//! manager turns a transport error into a reject). The hub path has no
//! manager-side timeout (`request_permission_via_hub` awaits without bound),
//! so the transport enforces `PERMISSION_PROMPT_TIMEOUT` itself — and the
//! `PermissionBridge::request` wrapper additionally bounds the whole call.
//!
//! Read-only sandbox (slice #16, `docs/CODEX_FALLBACK_TUI_SLICE_16`): a
//! read-only thread's bridge short-circuits non-read risk classes BEFORE the
//! manager — the hub (and this approval wire) is never reached for those
//! requests; the denial is journaled by the controller as
//! `PermissionDecision{deny}`. Read-class tools keep auto-allowing (no
//! approval either).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::{mpsc, oneshot};

use crate::codex_app::JsonMessage;
use crate::permission::PERMISSION_PROMPT_TIMEOUT;

/// Shared registry of in-flight approval requests + the outbound sink.
///
/// One broker per codex connection. The connection's inbound loop resolves
/// `approval/response` messages here (a JSON-RPC response with `id` + no
/// `method`); [`CodexPermissionTransport`] registers requests here.
pub struct CodexPermissionBroker {
    tx_out: mpsc::UnboundedSender<JsonMessage>,
    pending: Arc<Mutex<HashMap<Value, oneshot::Sender<Value>>>>,
    next_id: AtomicU64,
}

impl CodexPermissionBroker {
    pub fn new(tx_out: mpsc::UnboundedSender<JsonMessage>) -> Self {
        Self {
            tx_out,
            pending: Arc::new(Mutex::new(HashMap::new())),
            next_id: AtomicU64::new(1),
        }
    }

    /// The outbound sink (for the server's own writes).
    pub fn tx_out(&self) -> mpsc::UnboundedSender<JsonMessage> {
        self.tx_out.clone()
    }

    /// Register a pending approval request and return its JSON-RPC id.
    fn register(&self, respond: oneshot::Sender<Value>) -> Value {
        let id = json!(self.next_id.fetch_add(1, Ordering::Relaxed));
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).insert(id.clone(), respond);
        id
    }

    /// Resolve a client `approval/response` by JSON-RPC id. Unknown or stale
    /// ids (a response arriving after the transport timed out) are a silent
    /// no-op.
    pub fn resolve(&self, id: &Value, result: Value) {
        if let Some(respond) = self.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(id) {
            let _ = respond.send(result);
        }
    }

    /// Drop a pending entry without delivering (transport timeout path).
    fn forget(&self, id: &Value) {
        self.pending.lock().unwrap_or_else(|e| e.into_inner()).remove(id);
    }
}

/// [`PermissionHookTransport`] over the codex app-server channel.
#[derive(Clone)]
pub struct CodexPermissionTransport {
    broker: Arc<CodexPermissionBroker>,
    timeout: Duration,
}

impl CodexPermissionTransport {
    /// Default timeout: the shared 300s permission-prompt bound.
    pub fn new(broker: Arc<CodexPermissionBroker>) -> Self {
        Self {
            broker,
            timeout: PERMISSION_PROMPT_TIMEOUT,
        }
    }

    /// Explicit wait bound (tests use a short timeout).
    pub fn with_timeout(broker: Arc<CodexPermissionBroker>, timeout: Duration) -> Self {
        Self { broker, timeout }
    }
}

#[async_trait]
impl orz_workspace::permission::PermissionHookTransport for CodexPermissionTransport {
    async fn request_permission(&self, payload: Value) -> Result<Value, String> {
        let (respond, rx) = oneshot::channel();
        let id = self.broker.register(respond);
        let request = JsonMessage::request(id.clone(), "approval/request", payload.clone());
        // A closed connection (dropped receiver) fails closed — the manager
        // turns the error into a reject. Every failure path forgets the
        // pending entry (implementation review P3-2).
        if self.broker.tx_out.send(request).is_err() {
            self.broker.forget(&id);
            return Err("codex approval channel closed".to_owned());
        }
        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(reply)) => Ok(normalize_reply(&payload, &reply)),
            Ok(Err(_)) => {
                self.broker.forget(&id);
                Err("codex approval responder dropped".to_owned())
            }
            Err(_) => {
                self.broker.forget(&id);
                Err(format!(
                    "codex approval prompt timed out after {:?}",
                    self.timeout
                ))
            }
        }
    }
}

/// Map the client's simple decision onto the hub reply contract
/// (`permission_answer_to_json` shape: `{outcome, scope?}`).
///
/// - `allow_once` → `approve` — the current tool call executes once.
/// - `allow` → `always_approve` — with a `bash_command` scope when the
///   payload carries one (the manager persists `allowed_bash_commands`); for
///   Edit accesses the manager itself derives `AllowEditsForSession`; other
///   accesses get a plain session-scoped always-grant.
/// - anything else (`deny`, missing, garbage) → `reject` (fail closed).
///
/// NOTE (2026-08-06 slice #12): `PermissionBridge::request` maps every
/// `Decision::Allow` → `PermitDecision::AllowOnce` at the LoopHost boundary
/// (permission.rs) — observably "always" still holds because the grant is
/// persisted in the manager (allowed_bash_commands / AllowEditsForSession):
/// the current call runs and future identical calls auto-allow without a
/// prompt. The bridge itself never emits `AllowAlways`; differentiating the
/// two is a future slice.
fn normalize_reply(payload: &Value, reply: &Value) -> Value {
    match reply.get("decision").and_then(Value::as_str) {
        Some("allow_once") => json!({ "outcome": "approve" }),
        Some("allow") => {
            let mut outcome = json!({ "outcome": "always_approve" });
            // Non-empty command only — an empty `bash_command` (args without
            // `command`) must not persist an empty grant (implementation
            // review P3-7).
            if let Some(command) = payload
                .get("bash_command")
                .and_then(Value::as_str)
                .filter(|c| !c.is_empty())
            {
                outcome["scope"] = json!({ "kind": "bash_command", "value": command });
            }
            outcome
        }
        _ => json!({ "outcome": "reject" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_workspace::permission::PermissionHookTransport as _;

    #[test]
    fn decisions_map_to_hub_outcomes() {
        let payload = json!({ "bash_command": "ls -la" });
        assert_eq!(
            normalize_reply(&payload, &json!({ "decision": "allow_once" })),
            json!({ "outcome": "approve" })
        );
        assert_eq!(
            normalize_reply(&payload, &json!({ "decision": "allow" })),
            json!({
                "outcome": "always_approve",
                "scope": { "kind": "bash_command", "value": "ls -la" }
            })
        );
        // Edit payloads carry no bash_command → plain always_approve (the
        // manager derives AllowEditsForSession for Edit accesses itself).
        assert_eq!(
            normalize_reply(
                &json!({ "edit_file_paths": ["a.rs"] }),
                &json!({ "decision": "allow" })
            ),
            json!({ "outcome": "always_approve" })
        );
        assert_eq!(
            normalize_reply(&payload, &json!({ "decision": "deny" })),
            json!({ "outcome": "reject" })
        );
        // Unknown / missing / garbage decisions fail closed to reject.
        assert_eq!(
            normalize_reply(&payload, &json!({ "decision": "maybe" })),
            json!({ "outcome": "reject" })
        );
        assert_eq!(
            normalize_reply(&payload, &json!({})),
            json!({ "outcome": "reject" })
        );
    }

    #[test]
    fn broker_resolves_registered_ids_only() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx));
        let (respond, _hold) = oneshot::channel();
        let id = broker.register(respond);
        assert_eq!(id, json!(1));
        // Unknown id: no-op, nothing panics.
        broker.resolve(&json!(99), json!({}));
        // Registered id resolves.
        broker.resolve(&id, json!({ "decision": "deny" }));
        // Second resolve for the same id is a no-op (entry consumed).
        broker.resolve(&id, json!({ "decision": "deny" }));
    }

    #[tokio::test]
    async fn transport_sends_approval_request_and_awaits_reply() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx.clone()));
        let transport = CodexPermissionTransport::new(broker.clone());
        let expected_params = json!({
            "tool_call_id": "tc-1",
            "tool_name": "run_terminal_command",
            "description": "Run a terminal command",
            "scope": "write",
            "bash_command": "ls -la",
        });

        let payload_for_task = expected_params.clone();
        let t = transport.clone();
        let task = tokio::spawn(async move { t.request_permission(payload_for_task).await });

        // The outbound request carries the hub payload and a fresh id.
        let msg = rx.recv().await.expect("approval request sent");
        assert_eq!(msg.method.as_deref(), Some("approval/request"));
        let id = msg.id.clone().expect("request has id");
        assert_eq!(msg.params, Some(expected_params));

        // Client answers allow_once → tool executes once.
        broker.resolve(&id, json!({ "decision": "allow_once" }));
        assert_eq!(
            task.await.expect("transport resolves"),
            Ok(json!({ "outcome": "approve" }))
        );
    }

    #[tokio::test]
    async fn transport_maps_always_allow_for_bash() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx));
        let transport = CodexPermissionTransport::new(broker.clone());
        let payload = json!({ "bash_command": "rm -rf /tmp/x" });

        let t = transport.clone();
        let task = tokio::spawn(async move { t.request_permission(payload).await });
        let msg = rx.recv().await.unwrap();
        broker.resolve(&msg.id.unwrap(), json!({ "decision": "allow" }));
        assert_eq!(
            task.await.unwrap(),
            Ok(json!({
                "outcome": "always_approve",
                "scope": { "kind": "bash_command", "value": "rm -rf /tmp/x" }
            }))
        );
    }

    #[tokio::test]
    async fn transport_times_out_and_fails_closed() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx));
        let transport =
            CodexPermissionTransport::with_timeout(broker.clone(), Duration::from_millis(50));

        let started = std::time::Instant::now();
        let result = transport
            .request_permission(json!({ "bash_command": "ls" }))
            .await;
        assert!(result.is_err(), "timeout must fail closed: {result:?}");
        assert!(
            started.elapsed() >= Duration::from_millis(45),
            "returned before the injected timeout window"
        );
        // The pending entry was cleaned — a late response is a silent no-op.
        assert!(broker.pending.lock().unwrap_or_else(|e| e.into_inner()).is_empty());
    }

    #[tokio::test]
    async fn transport_fails_closed_when_channel_closed() {
        let (tx, rx) = mpsc::unbounded_channel();
        let broker = Arc::new(CodexPermissionBroker::new(tx));
        let transport = CodexPermissionTransport::new(broker);
        drop(rx); // connection gone before the request
        let result = transport.request_permission(json!({})).await;
        assert!(result.is_err());
    }
}
