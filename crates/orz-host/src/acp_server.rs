//! ACP JSON-RPC stdio server — the entry point for the agent communication protocol.
//!
//! Uses `agent-client-protocol` 0.10.4 + `xai-acp-lib` for gateway/channels.
//! Implements `session/new` and `session/prompt` methods.
//! Delegates agent turns to `orz_loop::AgentLoopController` via the `LoopHost` trait.
//!
//! Phase 1: scaffold — session/new + session/prompt with stub gateway.
//! Full ACP lifecycle (tool calls, permissions, notifications) in Phase 2.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use orz_assurance::{EventType, JournalRecorderError, Redaction, RunEvent, seal_event};
use orz_loop::AgentLoopController;
use orz_workspace::permission::PermissionHookTransport;

use crate::session::{SessionError, bootstrap_session};

/// Errors from the ACP server layer.
#[derive(Debug, thiserror::Error)]
pub enum AcpError {
    #[error("session error: {0}")]
    Session(#[from] SessionError),
    #[error("agent loop error: {0}")]
    AgentLoop(#[from] orz_loop::controller::AgentLoopError),
    #[error("session not found: {0}")]
    SessionNotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("host error: {0}")]
    Host(String),
}

/// Journal-recording error → ACP error (the ACP layer has no journal variant;
/// `SessionError::Journal` wraps the recorder error, mirroring `session.rs`).
fn acp_journal_error(e: JournalRecorderError) -> AcpError {
    AcpError::Session(SessionError::Journal(e))
}

/// Chain-aware event recorder for host-authored runs (restore). Mirrors the
/// loop's `EventWriter` (seal → record → advance) for the few events a
/// host-initiated run writes after bootstrap's `run_preflight`.
struct RunRecorder<'a> {
    journal: &'a JournalRecorder,
    run_id: String,
    manifest_sha256: String,
    seq: u64,
    prev_hash: Option<String>,
}

impl<'a> RunRecorder<'a> {
    fn new(
        journal: &'a JournalRecorder,
        run_id: &str,
        manifest_sha256: &str,
        seq: u64,
        prev_hash: Option<String>,
    ) -> Self {
        Self {
            journal,
            run_id: run_id.to_string(),
            manifest_sha256: manifest_sha256.to_string(),
            seq,
            prev_hash,
        }
    }

    async fn record(
        &mut self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<(), JournalRecorderError> {
        let mut event = RunEvent::new(
            self.run_id.clone(),
            self.seq,
            event_type,
            self.manifest_sha256.clone(),
            self.prev_hash.clone(),
            "run-event-v0.1.schema.json".into(),
            payload,
            Redaction::None,
            chrono::Utc::now().to_rfc3339(),
        );
        seal_event(&mut event).map_err(JournalRecorderError::Serde)?;
        let event_hash = event.event_sha256.clone();
        // Only advance the chain link after the write is accepted (a refused
        // append must not pollute the caller's bookkeeping — 2026-08-04
        // review P2-7 precedent).
        self.journal.record_async(event).await?;
        self.prev_hash = Some(event_hash);
        self.seq += 1;
        Ok(())
    }
}

/// Payload-friendly scope strings: worktree-relative, `/`-separated (the
/// store's own manifest format). Only relative paths reach this on a
/// success path — the store rejects absolute paths and `..` escapes
/// fail-closed before any write — so the caller's paths are joined
/// verbatim. (2026-08-05 review P3-4: future protocol entries must filter
/// empty/absolute scope items before calling.)
fn scope_strings(scope: &[PathBuf]) -> Vec<String> {
    scope
        .iter()
        .map(|p| {
            p.components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

/// Per-session metadata. Journals are per-**run** (one per prompt), so the
/// session itself holds no journal — each `session/prompt` bootstraps a fresh
/// run with its own hash-chained journal.
struct StoredSession {
    base_dir: PathBuf,
    trust_policy: crate::session::TrustPolicy,
    /// Prompt counter — each prompt gets a unique run id (`RUN-{suffix}-{n}`).
    prompt_count: u64,
    /// Restore counter — each user-initiated restore is its own run with a
    /// distinct prefix (`RST-{suffix}-{n}`), so it can neither collide with
    /// prompt run ids nor desync the TUI's `run_dir_for_next_prompt` (which
    /// derives the next prompt's dir from the prompt counter; slice #8).
    restore_count: u64,
}

/// The ACP server — holds active sessions and dispatches requests.
pub struct AcpServer {
    sessions: Arc<Mutex<HashMap<String, StoredSession>>>,
    /// Outbound ACP gateway (agent → client messages: permissions etc.).
    /// `None` in headless mode — permission prompts fail closed.
    gateway: Arc<Mutex<Option<AcpAgentGatewaySender>>>,
    /// Model gateway for agent turns (scripted FakeProvider offline).
    model_gateway: Arc<dyn ModelGateway>,
    /// Per-session in-flight run state (Phase 3 slice #7 token map, extended
    /// slice #11 P2-2 to cover restores). One lock, one map: check + insert
    /// happen in a single critical section, so prompt-vs-restore and
    /// restore-vs-restore exclusions are atomic — no check-then-act window
    /// survives concurrent dispatch (the SSE entry). A `Prompt` token is
    /// inserted when a prompt starts and removed on every completion path —
    /// a cancel arriving after completion is a benign no-op.
    runs: Arc<Mutex<HashMap<String, RunInFlight>>>,
    /// Sessions that cancelled while no run was registered (Phase 3 slice
    /// #7, cancel-before-bootstrap race): a client pressing Ctrl+Z
    /// immediately after submitting can beat the prompt's bootstrap. The
    /// flag is consumed by the next prompt's token registration — but only
    /// within a short window, so a stale/idle cancel never poisons an
    /// unrelated later prompt (2026-08-05 review P2-1).
    pending_cancels: Arc<Mutex<HashMap<String, std::time::Instant>>>,
    /// Optional interactive permission transport (Phase 3 slice #12): when
    /// set, the permission manager routes interactive prompts through
    /// `PermissionHookTransport::request_permission` (the codex app-server
    /// approval surface) instead of the ACP gateway. `None` keeps the
    /// gateway path (stdio/TUI).
    hub_permission: Arc<Mutex<Option<Arc<dyn PermissionHookTransport>>>>,
}

/// What is in flight for a session under `AcpServer::runs`.
enum RunInFlight {
    /// A prompt is running; the token is the slice #7 cancellation handle.
    Prompt(tokio_util::sync::CancellationToken),
    /// A snapshot restore is executing (no cancellation token — P3-7
    /// record: restores are short host-side operations, not agent runs).
    Restore,
}

/// How long a remembered cancel stays valid. A cancel racing the prompt's
/// registration (task-spawn → token-registration gap) is honored; anything
/// older is an idle/stale cancel and must not cancel an unrelated later
/// prompt (2026-08-05 review P2-1).
const PENDING_CANCEL_WINDOW: std::time::Duration = std::time::Duration::from_secs(2);

/// RAII release of a session's restore-in-flight marker (Phase 3 slice #11,
/// P2-2). Registering happens inside `new` under the same lock as the
/// check — the shared `runs` map — so neither a concurrent restore nor a
/// running prompt can slip between check and insert; dropping the guard
/// removes the marker on every completion path — including errors and
/// journal failures — so a later restore/prompt is never falsely rejected
/// by a stale marker.
struct RestoreInflightGuard {
    runs: Arc<Mutex<HashMap<String, RunInFlight>>>,
    session: String,
}

impl RestoreInflightGuard {
    /// Register the marker; returns `Err(session_id)` when the session
    /// already has anything in flight (prompt or restore).
    fn new(
        runs: &Arc<Mutex<HashMap<String, RunInFlight>>>,
        session_id: &str,
    ) -> Result<Self, String> {
        let mut guard = runs.lock().unwrap();
        if guard.contains_key(session_id) {
            return Err(session_id.to_string());
        }
        guard.insert(session_id.to_string(), RunInFlight::Restore);
        drop(guard);
        Ok(Self {
            runs: runs.clone(),
            session: session_id.to_string(),
        })
    }
}

impl Drop for RestoreInflightGuard {
    fn drop(&mut self) {
        self.runs.lock().unwrap().remove(&self.session);
    }
}

impl AcpServer {
    pub fn new() -> Self {
        // Two scripted texts per turn — the counterexample gate (§4.6) adds
        // one model round before the final answer.
        Self::with_gateway(Arc::new(FakeProvider::from_texts(vec![
            "(fake) 已收到请求。",
            "(fake) 已收到请求。",
        ])))
    }

    pub fn with_gateway(model_gateway: Arc<dyn ModelGateway>) -> Self {
        AcpServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            gateway: Arc::new(Mutex::new(None)),
            model_gateway,
            runs: Arc::new(Mutex::new(HashMap::new())),
            pending_cancels: Arc::new(Mutex::new(HashMap::new())),
            hub_permission: Arc::new(Mutex::new(None)),
        }
    }

    /// Route interactive permission prompts through `hub` (the codex
    /// app-server approval surface) instead of the ACP gateway. Call once
    /// after construction, before any turn; the transport must bound its own
    /// wait and fail closed (the hub path has no manager-side timeout).
    pub fn set_hub_permission(&self, hub: Arc<dyn PermissionHookTransport>) {
        *self.hub_permission.lock().unwrap() = Some(hub);
    }

    /// Cancel the run currently in flight for a session (ACP `session/cancel`
    /// — Phase 3 slice #7). Returns whether a run was tracked: `false` means
    /// the session is idle or its run already finished. A cancel arriving
    /// BEFORE the next prompt's bootstrap registers its token is remembered
    /// as pending — the prompt starts already-cancelled (the
    /// cancel-before-bootstrap race; a client pressing Ctrl+Z immediately
    /// after submitting). `CancellationToken::cancel()` is idempotent, so
    /// double cancels are safe; after the map entry is removed stale cancels
    /// return `false`.
    pub fn cancel_current_run(&self, session_id: &str) -> bool {
        {
            let map = self.runs.lock().unwrap();
            if let Some(RunInFlight::Prompt(token)) = map.get(session_id) {
                token.cancel();
                drop(map);
                // The live-token path supersedes any remembered cancel.
                self.pending_cancels.lock().unwrap().remove(session_id);
                return true;
            }
        }
        // No live run — remember the cancel for the bootstrap window only
        // (it expires, so a stale/idle cancel cannot cancel an unrelated
        // later prompt).
        self.pending_cancels
            .lock()
            .unwrap()
            .insert(session_id.to_string(), std::time::Instant::now());
        false
    }

    /// Set the outbound ACP gateway (wired by the stdio server; consumed by
    /// the permission bridge).
    pub fn set_gateway(&self, sender: AcpAgentGatewaySender) {
        *self.gateway.lock().unwrap() = Some(sender);
    }

    /// Outbound gateway, if interactive mode is active.
    pub fn gateway(&self) -> Option<AcpAgentGatewaySender> {
        self.gateway.lock().unwrap().clone()
    }

    /// Handle a `session/new` request.
    ///
    /// `base_dir` is the directory under which journals are written
    /// (`{base_dir}/runs/{run_id}/events.jsonl`). Tests must pass an isolated
    /// temp dir so no `.gsa/` residue appears in the workspace.
    ///
    /// `trust_policy` controls the workspace-trust gate: production paths pass
    /// `TrustPolicy::Enforce`; tests pass `TrustPolicy::Skip` (trust semantics
    /// are covered by `session::tests`).
    pub async fn handle_session_new(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
        trust_policy: crate::session::TrustPolicy,
    ) -> Result<serde_json::Value, AcpError> {
        // Journals are created per-run at `session/prompt` time — the session
        // itself only records where and under what trust policy runs live.
        self.sessions.lock().unwrap().insert(
            session_id.to_string(),
            StoredSession {
                base_dir: base_dir.unwrap_or_else(|| PathBuf::from(".")),
                trust_policy,
                prompt_count: 0,
                restore_count: 0,
            },
        );

        Ok(serde_json::json!({
            "session_id": session_id,
            "status": "created",
        }))
    }

    /// Handle a `session/prompt` request.
    ///
    /// Each prompt is its own **run** with a fresh hash-chained journal. A
    /// journal is a single-run integrity unit — the verifier rejects multiple
    /// terminal events — so multi-prompt sessions get one journal per prompt
    /// instead of sharing one chain (2026-08-04 review P0).
    pub async fn handle_session_prompt(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<serde_json::Value, AcpError> {
        // Reserve the run id BEFORE bootstrap: the counter advances even when
        // the run fails (untrusted cwd, model error), so a retried prompt gets
        // a fresh run dir — reusing a failed run's dir would append to its
        // journal and corrupt the chain (2026-08-05 orz-tui review P2-2).
        let (base_dir, trust_policy, prompt_number) = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            let n = session.prompt_count;
            session.prompt_count += 1;
            (session.base_dir.clone(), session.trust_policy, n)
        };

        let suffix: String = session_id.chars().take(8).collect();
        let run_id = format!("RUN-{suffix}-{prompt_number}");

        // Phase 3 slice #7: register the run's cancellation token BEFORE the
        // bootstrap awaits — a `session/cancel` landing anywhere in this
        // function's body (including during the trust scan) hits the token
        // directly instead of the pending fallback. A cancel arriving in the
        // tiny gap between the connection spawning this task and this line is
        // consumed from the pending set — the run starts already-cancelled
        // (the cancel-before-bootstrap race). NOTE the protocol-order
        // semantic: a cancel that beats the NEXT prompt's registration (e.g.
        // sent immediately after a completed run) cancels that next prompt —
        // ACP `session/cancel` cancels the session's next operation; the TUI
        // never hits this (it only cancels while `running`).
        //
        // Phase 3 slice #11 (P2-2): the restore exclusion lives in the same
        // critical section — check + insert under one lock, so a concurrent
        // restore cannot slip between the check and our registration
        // (prompt-vs-restore atomicity; the restore side registers under the
        // same map). Prompt-vs-prompt keeps the recorded overwrite semantics
        // (slice #7 P3 — the TUI guards `running`, stdio is sequential).
        let cancel = tokio_util::sync::CancellationToken::new();
        {
            let mut map = self.runs.lock().unwrap();
            if matches!(map.get(session_id), Some(RunInFlight::Restore)) {
                return Err(AcpError::InvalidRequest(format!(
                    "prompt rejected: a snapshot restore is in flight for session {session_id}"
                )));
            }
            map.insert(session_id.to_string(), RunInFlight::Prompt(cancel.clone()));
            if let Some(stamped) = self.pending_cancels.lock().unwrap().remove(session_id)
                && stamped.elapsed() < PENDING_CANCEL_WINDOW
            {
                // A cancel within the bootstrap window — the run starts
                // already-cancelled. Stale/idle cancels expire silently.
                cancel.cancel();
            }
        }

        // Slice #10 review D2-4: a bootstrap failure must release the token
        // too — a leaked token would make restore_snapshot's in-flight check
        // reject every restore with a false "a run is in flight" until the
        // next successful prompt.
        let bootstrap = bootstrap_session(&run_id, Some(base_dir.clone()), trust_policy).await;
        if bootstrap.is_err() {
            self.runs.lock().unwrap().remove(session_id);
        }
        let handle = bootstrap?;

        // Phase 3 wiring: the real host — finalized GrokBuild toolset +
        // workspace trust + IP6 permission bridge. The bridge consumes the
        // outbound ACP gateway when interactive (`--stdio`); headless
        // (`None` gateway) fails closed: Read auto-allows, Bash → Deny.
        // (PermissionBridge spawns the manager actor via spawn_local, so
        // this path must run inside a LocalSet — the stdio server does.)
        let host = self.build_host(&handle, session_id, &base_dir)?;
        // IP5: attach the session's pre-mutation snapshot store — mutation
        // tools with knowable targets get tracked before execution.
        let controller = AgentLoopController::with_gateway(self.model_gateway.clone())
            .with_snapshot_store(Some(handle.snapshot_store.clone()));

        let run_result = controller
            .run_turn_with_cancel(
                &host,
                prompt,
                &handle.run_id,
                &handle.run_manifest_sha256,
                handle.next_sequence,
                handle.last_event_sha256.clone(),
                Some(&cancel),
            )
            .await;

        // Every path: release the run token (stale cancels become no-ops)
        // and the journal writer task (previously only the success path
        // released it).
        self.runs.lock().unwrap().remove(session_id);
        let _ = handle.journal.shutdown_async().await;

        match run_result {
            Ok((response, _, _)) => Ok(serde_json::json!({
                "session_id": session_id,
                "response": response,
                "status": "completed",
                "run_id": run_id,
            })),
            // A user cancel propagates distinctly — the stdio layer maps it
            // to `StopReason::Cancelled` (the ACP-correct reply to a
            // cancelled session/prompt), not an internal error.
            Err(e) => Err(AcpError::AgentLoop(e)),
        }
    }

    /// IP5 restore entry (Phase 3, slice #8 — P2-3 closure): restore the
    /// session worktree from a snapshot recorded earlier in this session's
    /// runs.
    ///
    /// A restore is **its own run** (`RST-{suffix}-{n}`) with a full
    /// hash-chained journal (`run_preflight → snapshot_restored → terminal`):
    /// journals are single-run integrity units (2026-08-04 review P0), so a
    /// user-initiated restore between prompts cannot append to a finished
    /// run's journal. The `RST-` prefix (vs `RUN-`) keeps the TUI journal
    /// tail — which scans for the newest `RUN-{session8}-{n}` dir — from
    /// tailing a restore journal.
    ///
    /// - `scope = None` → full `restore`; `Some(paths)` → selective `revert`
    ///   (paths must be worktree-relative — the store rejects `..` escapes
    ///   and absolute paths, fail-closed).
    /// - No permission flow: the restore does not touch the permission
    ///   system (design v0.1 §3.5 — "恢复成功不改变原 permission 决策");
    ///   the approval/TUI layer decides when to call this.
    /// - Fail-closed: unknown session → `SessionNotFound`; a prompt in
    ///   flight → `InvalidRequest` (a restore mid-run would mutate the
    ///   worktree under the running agent — the approval/TUI layer must
    ///   sequence it between runs); a failed restore records
    ///   `snapshot_restored{snapshot_error}` + `run_failed` and returns the
    ///   error — the journal is the evidence record either way.
    ///
    /// Known records (2026-08-05 review):
    /// - P2-2 (CLOSED, slice #11): prompt and restore in-flight state share
    ///   one map (`runs`) with check + register in a single critical
    ///   section — prompt-vs-restore and restore-vs-restore exclusions are
    ///   atomic under concurrent dispatch (SSE entry); an RAII guard
    ///   releases the restore marker on every completion path.
    /// - P3-1: a rejected restore still consumes a restore sequence number
    ///   (holes in `RST-…-n` numbering) — harmless while nothing derives
    ///   restore dirs from the counter.
    /// - P3-7: a restore registers no cancellation token; a cancel during a
    ///   future protocol-level restore would fall into `pending_cancels`
    ///   and pre-cancel the next prompt within the 2s window — handle when
    ///   wiring the protocol entry.
    pub async fn restore_snapshot(
        &self,
        session_id: &str,
        snapshot_hash: &str,
        scope: Option<Vec<PathBuf>>,
    ) -> Result<serde_json::Value, AcpError> {
        let (base_dir, trust_policy, restore_number) = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            let n = session.restore_count;
            session.restore_count += 1;
            (session.base_dir.clone(), session.trust_policy, n)
        };
        // Fail-closed (P2-2, slice #11): register the session's restore
        // marker under the same lock as the check — the shared `runs` map —
        // so neither a running prompt (a live token marks it) nor a
        // concurrent restore can slip between check and insert. The RAII
        // guard releases the marker on every path below.
        let _inflight = RestoreInflightGuard::new(&self.runs, session_id).map_err(|session_id| {
            AcpError::InvalidRequest(format!(
                "restore rejected: a run or restore is already in flight for session {session_id}"
            ))
        })?;

        let suffix: String = session_id.chars().take(8).collect();
        let run_id = format!("RST-{suffix}-{restore_number}");

        // The restore run bootstraps like any other run (trust + journal +
        // preflight + session snapshot store) — no agent loop involved.
        let handle = bootstrap_session(&run_id, Some(base_dir), trust_policy).await?;
        let mut recorder = RunRecorder::new(
            &handle.journal,
            &handle.run_id,
            &handle.run_manifest_sha256,
            handle.next_sequence,
            handle.last_event_sha256.clone(),
        );

        let outcome = match &scope {
            None => handle.snapshot_store.restore(snapshot_hash).await,
            Some(paths) => handle.snapshot_store.revert(snapshot_hash, paths).await,
        };

        let result = match outcome {
            Ok(outcome) => {
                let mut payload = serde_json::json!({
                    "snapshot_hash": snapshot_hash,
                    "restored": outcome.restored,
                });
                if let Some(paths) = &scope {
                    payload["scope"] = serde_json::json!(scope_strings(paths));
                }
                // Journal both events; the second only if the first landed —
                // the chain must never skip a link. NOTE: a journal write
                // failure here DOES surface as an error (the worktree is
                // already restored, but the evidence record is mandatory —
                // evidence-layer, not gate, semantics apply to the *track*
                // side; a failed restore journal is an integrity failure).
                // The journal task still shuts down on every path (slice #7
                // "shutdown 全路径").
                let mut recorded = recorder.record(EventType::SnapshotRestored, payload).await;
                if recorded.is_ok() {
                    recorded = recorder
                        .record(
                            EventType::RunFinished,
                            serde_json::json!({"status": "completed"}),
                        )
                        .await;
                }
                recorded
                    .map(|_| {
                        serde_json::json!({
                            "session_id": session_id,
                            "run_id": run_id,
                            "snapshot_hash": snapshot_hash,
                            "restored": outcome.restored,
                            "status": "restored",
                        })
                    })
                    .map_err(acp_journal_error)
            }
            Err(e) => {
                // Best effort journaling; the ORIGINAL error is returned
                // regardless (loop precedent — "the original error is
                // returned even if the journal is dead").
                let mut recorded = recorder
                    .record(
                        EventType::SnapshotRestored,
                        serde_json::json!({"snapshot_error": e.to_string()}),
                    )
                    .await;
                if recorded.is_ok() {
                    recorded = recorder
                        .record(
                            EventType::RunFailed,
                            serde_json::json!({"error": e.to_string()}),
                        )
                        .await;
                }
                let _ = recorded; // journal failure does not shadow the restore failure
                Err(AcpError::Session(SessionError::Snapshot(e)))
            }
        };
        // Every path: release the journal writer task (slice #7 discipline).
        let _ = handle.journal.shutdown_async().await;
        result
    }

    /// List active session IDs.
    pub fn list_sessions(&self) -> Vec<String> {
        self.sessions.lock().unwrap().keys().cloned().collect()
    }

    /// Close a session and release its stored metadata.
    ///
    /// ACP 0.10.4 has no `session/close` method — the stdio server terminates
    /// on stdin EOF — but explicit closure keeps the session table bounded
    /// for long-lived/embedded hosts (2026-08-04 review P2-4).
    /// Returns `true` if the session existed and was removed.
    pub fn close_session(&self, session_id: &str) -> bool {
        let existed = self.sessions.lock().unwrap().remove(session_id).is_some();
        // Release cancellation state too — a remembered cancel must not
        // outlive its session (2026-08-05 review P2-1). A stray run/restore
        // marker is dropped the same way (slice #11, P2-2).
        self.runs.lock().unwrap().remove(session_id);
        self.pending_cancels.lock().unwrap().remove(session_id);
        existed
    }

    /// Build the real host for a run: finalized GrokBuild toolset +
    /// workspace-trust observation + IP6 permission bridge.
    ///
    /// The bridge consumes the outbound ACP gateway when one is wired
    /// (`--stdio`); otherwise a fail-closed dead gateway — Read auto-allows,
    /// Bash `Ask` → `Deny` (IP6 headless semantics).
    fn build_host(
        &self,
        handle: &crate::session::SessionHandle,
        session_id: &str,
        base_dir: &std::path::Path,
    ) -> Result<crate::OrzHost, AcpError> {
        // The permission manager requires an absolute cwd (AbsPathBuf) —
        // canonicalize, falling back to the raw path on failure. dunce strips
        // the `\\?\` verbatim prefix on Windows (clippy.toml ban on std).
        let cwd = dunce::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        // P1 permit keystore: the session's DPAPI-backed signer (or the
        // test-only memory store under TrustPolicy::Skip).
        Ok(crate::OrzHost::with_bridge_and_hub(
            session_id,
            handle.journal.clone(),
            &cwd,
            handle.workspace_trust,
            self.gateway(),
            self.hub_permission.lock().unwrap().clone(),
        )
        .map_err(AcpError::Host)?
        .with_permit_signer(handle.permit_signer.clone()))
    }
}

impl Default for AcpServer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Phase 1: Minimal LoopHost implementation ─────────────────────────

use async_trait::async_trait;
use orz_assurance::JournalRecorder;
use orz_loop::gateway::fake::FakeProvider;
use orz_loop::gateway::model::ModelGateway;
use orz_loop::host::{LoopHost, ToolDef, ToolRegistry};
use xai_acp_lib::AcpAgentGatewaySender;

/// Phase 1 minimal host — only provides journal access.
/// Tool registry, permissions, etc. are stubbed.
pub struct JournalOnlyHost {
    journal: JournalRecorder,
}

impl JournalOnlyHost {
    pub fn new(journal: JournalRecorder) -> Self {
        JournalOnlyHost { journal }
    }
}

struct EmptyRegistry;
impl ToolRegistry for EmptyRegistry {
    fn get(&self, _name: &str) -> Option<ToolDef> {
        None
    }
    fn list(&self) -> Vec<ToolDef> {
        Vec::new()
    }
}

#[async_trait]
impl LoopHost for JournalOnlyHost {
    fn journal(&self) -> &JournalRecorder {
        &self.journal
    }
    fn tools_registry(&self) -> &dyn ToolRegistry {
        &EmptyRegistry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::dead_gateway;
    use crate::stdio::StdioAgentHandler;
    use agent_client_protocol as acp;
    use agent_client_protocol::MessageHandler;
    use orz_assurance::EventType;
    use orz_loop::controller::AgentLoopError;
    use orz_loop::gateway::fake::ScriptedResponse;
    use orz_loop::gateway::model::ToolCall;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Paths of every run journal under `base`, in creation order.
    fn all_run_events_paths(base: &Path) -> Vec<PathBuf> {
        let runs_dir = base.join(".gsa").join("runs");
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        dirs.sort();
        dirs.iter().map(|d| d.join("events.jsonl")).collect()
    }

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Typed events of the single run journal under `base`.
    fn run_events(base: &Path) -> Vec<orz_assurance::RunEvent> {
        let runs_dir = base.join(".gsa").join("runs");
        let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(run_dirs.len(), 1, "expected one run journal");
        let content = std::fs::read_to_string(run_dirs[0].join("events.jsonl")).unwrap();
        content
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    fn test_dir() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("orz-acp-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn session_new_registers_session() {
        let base = test_dir();

        let server = AcpServer::new();
        let result = server
            .handle_session_new(
                "test-session-1234",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        assert_eq!(result["session_id"], "test-session-1234");
        assert_eq!(result["status"], "created");

        let sessions = server.list_sessions();
        assert!(sessions.contains(&"test-session-1234".to_string()));

        let _ = std::fs::remove_dir_all(&base);
    }

    #[tokio::test]
    async fn session_prompt_produces_valid_journal_chain() {
        // `handle_session_prompt` builds the OrzHost + IP6 permission bridge
        // (manager actor runs via `spawn_local`) — everything inside a
        // LocalSet, matching the stdio server shape.
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();

                let server = AcpServer::new();
                server
                    .handle_session_new(
                        "test-session-prompt",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let result = server
                    .handle_session_prompt("test-session-prompt", "hello world")
                    .await
                    .unwrap();

                assert_eq!(result["session_id"], "test-session-prompt");
                assert_eq!(result["status"], "completed");
                assert!(result["response"].as_str().unwrap().contains("已收到请求"));

                // The full ACP path (session/new → session/prompt) must produce a
                // continuous hash chain: preflight → started → prompt → output → finished.
                let runs_dir = base.join(".gsa").join("runs");
                let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                assert_eq!(run_dirs.len(), 1, "expected exactly one run journal");
                let events_path = run_dirs[0].join("events.jsonl");

                let replay = orz_assurance::replay_journal(&events_path, None, None, true);
                assert!(
                    replay.valid,
                    "ACP path journal invalid: {:?}",
                    replay.errors
                );
                // Full Phase 2 gate chain + §4.6: preflight + started +
                // prompt_submitted + orientation + tool_availability +
                // model_output + counterexample_gate + model_output +
                // stagnation + finished.
                assert_eq!(replay.event_count, 10);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn unknown_session_returns_error() {
        let server = AcpServer::new();
        let result = server.handle_session_prompt("nonexistent", "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn second_prompt_continues_hash_chain() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();

                // Four scripted responses — two per prompt turn (each turn's
                // first round is gate-intercepted; the default gateway's two
                // entries would exhaust on the second prompt).
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::from_texts(vec![
                    "(fake) 第一轮。",
                    "(fake) 第一轮终答。",
                    "(fake) 第二轮。",
                    "(fake) 第二轮终答。",
                ])));
                server
                    .handle_session_new(
                        "test-session-two-prompts",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let first = server
                    .handle_session_prompt("test-session-two-prompts", "hello")
                    .await
                    .unwrap();
                assert_eq!(first["status"], "completed");

                let second = server
                    .handle_session_prompt("test-session-two-prompts", "world")
                    .await
                    .unwrap();
                assert_eq!(second["status"], "completed");

                // Each prompt is its own run journal — a journal is a single-run
                // hash chain with exactly one terminal event (2026-08-04 review P0).
                let runs_dir = base.join(".gsa").join("runs");
                let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                assert_eq!(run_dirs.len(), 2, "one run journal per prompt");

                for dir in &run_dirs {
                    let replay =
                        orz_assurance::replay_journal(&dir.join("events.jsonl"), None, None, true);
                    assert!(replay.valid, "run journal invalid: {:?}", replay.errors);
                    assert_eq!(replay.event_count, 10, "preflight + 9 turn events");
                    assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
                }

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn close_session_releases_metadata() {
        let base = test_dir();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "test-session-close",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        assert!(
            server
                .list_sessions()
                .contains(&"test-session-close".to_string())
        );

        assert!(server.close_session("test-session-close"));
        assert!(
            !server
                .list_sessions()
                .contains(&"test-session-close".to_string())
        );
        // Closing a nonexistent session reports false.
        assert!(!server.close_session("test-session-close"));

        // A closed session rejects new prompts.
        let result = server
            .handle_session_prompt("test-session-close", "hi")
            .await;
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Phase 3 wiring: the ACP path must drive the REAL OrzHost toolset
    /// through the IP6 permission bridge — a low-risk `read_file` call
    /// auto-allows and actually executes (ToolStarted/ToolCompleted).
    #[tokio::test]
    async fn session_prompt_read_tool_executes_through_bridge() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let target = base.join("sample.txt");
                std::fs::write(&target, "wired file content").unwrap();

                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({"target_file": target}),
                        call_id: "call-1".to_string(),
                    }]),
                    ScriptedResponse::text("完成（读取成功）。"),
                    ScriptedResponse::text("完成（读取成功）。"),
                ])));
                // Interactive-gateway shape (--stdio wires the same way); a
                // dead receiver still lets low-risk reads auto-allow.
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-read",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let result = server
                    .handle_session_prompt("sess-read", "读取 sample.txt")
                    .await
                    .unwrap();
                assert_eq!(result["status"], "completed");
                assert!(
                    result["response"].as_str().unwrap().contains("完成"),
                    "{result}"
                );

                let events = run_events(&base);
                let types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();
                assert!(types.contains(&EventType::ToolStarted), "{types:?}");
                assert!(types.contains(&EventType::ToolCompleted), "{types:?}");
                let pd = events
                    .iter()
                    .find(|e| e.event_type == EventType::PermissionDecision)
                    .expect("permission decision");
                assert_eq!(
                    pd.payload.get("decision").and_then(|d| d.as_str()),
                    Some("allow_once")
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 wiring: `bash` (SandboxEscape) with no interactive client
    /// fails closed — PermissionDecision deny, tool never starts (IP6).
    #[tokio::test]
    async fn session_prompt_bash_denied_without_interactive_client() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();

                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "bash".to_string(),
                        arguments: serde_json::json!({"command": "dir"}),
                        call_id: "call-1".to_string(),
                    }]),
                    ScriptedResponse::text("完成（bash 被拒）。"),
                    ScriptedResponse::text("完成（bash 被拒）。"),
                ])));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-bash",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let result = server
                    .handle_session_prompt("sess-bash", "执行命令")
                    .await
                    .unwrap();
                assert_eq!(result["status"], "completed");

                let events = run_events(&base);
                let types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();
                assert!(
                    !types.contains(&EventType::ToolStarted),
                    "bash must not start headless: {types:?}"
                );
                let pd = events
                    .iter()
                    .find(|e| e.event_type == EventType::PermissionDecision)
                    .expect("permission decision");
                assert_eq!(
                    pd.payload.get("decision").and_then(|d| d.as_str()),
                    Some("deny")
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// IP5 wiring E2E (headless fail-closed): a mutation tool
    /// (`search_replace`) is denied by the dead gateway before it starts —
    /// the pre-mutation snapshot is NOT taken for denied tools (the snapshot
    /// fires only after the permission gate allows, preserving the
    /// fail-closed ordering).
    #[tokio::test]
    async fn session_prompt_denied_mutation_records_no_snapshot() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                std::fs::write(base.join("lib.rs"), "fn main() {}").unwrap();

                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "lib.rs",
                            "old_string": "fn main",
                            "new_string": "fn renamed",
                        }),
                        call_id: "call-1".to_string(),
                    }]),
                    ScriptedResponse::text("完成（被拒）。"),
                    ScriptedResponse::text("完成（被拒）。"),
                ])));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-deny-snap",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let result = server
                    .handle_session_prompt("sess-deny-snap", "修改 lib.rs")
                    .await
                    .unwrap();
                assert_eq!(result["status"], "completed");

                let types: Vec<EventType> = run_events(&base)
                    .iter()
                    .map(|e| e.event_type.clone())
                    .collect();
                assert!(types.contains(&EventType::PermissionDecision), "{types:?}");
                assert!(
                    !types.contains(&EventType::SnapshotCreated),
                    "denied mutation must not snapshot: {types:?}"
                );
                assert!(
                    !types.contains(&EventType::ToolStarted),
                    "denied tool must not start: {types:?}"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 slice #7: `cancel_current_run` aborts the in-flight prompt
    /// cooperatively — the loop's next checkpoint terminates with a
    /// `run_cancelled` journal terminal, and the token is removed so a
    /// follow-up cancel is a no-op.
    #[tokio::test]
    async fn cancel_current_run_aborts_pending_prompt() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                )));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-cancel",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let srv = server.clone();
                let prompt = tokio::task::spawn_local(async move {
                    srv.handle_session_prompt("sess-cancel", "hello").await
                });
                // Let the first model round get underway, then cancel — the
                // run terminates at the after-round checkpoint.
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert!(server.cancel_current_run("sess-cancel"));

                let result = prompt.await.unwrap();
                assert!(matches!(
                    result,
                    Err(AcpError::AgentLoop(AgentLoopError::Cancelled))
                ));

                // Token removed on completion — a stale cancel is a no-op.
                assert!(!server.cancel_current_run("sess-cancel"));

                let events = run_events(&base);
                let types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();
                assert!(types.contains(&EventType::RunCancelled), "{types:?}");
                let runs_dir = base.join(".gsa").join("runs");
                let run_dirs: Vec<PathBuf> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .collect();
                let replay = orz_assurance::replay_journal(
                    &run_dirs[0].join("events.jsonl"),
                    None,
                    None,
                    true,
                );
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 slice #7: cancelling when idle (fresh session) or after a run
    /// already finished is a benign no-op returning `false` — no panic, no
    /// effect on the (already finished) run.
    #[tokio::test]
    async fn cancel_when_idle_is_noop() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = Arc::new(AcpServer::new());
                server
                    .handle_session_new(
                        "sess-idle",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                // A completed run removed its token synchronously.
                server
                    .handle_session_prompt("sess-idle", "hello")
                    .await
                    .unwrap();
                assert!(!server.cancel_current_run("sess-idle"));

                // Fresh session, never ran: also false (no in-flight run).
                server
                    .handle_session_new(
                        "sess-idle2",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                assert!(!server.cancel_current_run("sess-idle2"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 slice #7 — cancel-before-bootstrap race: a cancel arriving
    /// before the prompt's token registration is remembered and the run
    /// starts already-cancelled (a client pressing Ctrl+Z immediately after
    /// submitting must not run to completion).
    #[tokio::test]
    async fn cancel_before_bootstrap_cancels_next_prompt() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = Arc::new(AcpServer::with_gateway(Arc::new(FakeProvider::from_texts(
                    vec!["结果：完成", "结果：完成"],
                ))));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-race",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                // Cancel BEFORE the prompt is even issued.
                assert!(!server.cancel_current_run("sess-race"));

                let result = server.handle_session_prompt("sess-race", "hello").await;
                assert!(matches!(
                    result,
                    Err(AcpError::AgentLoop(AgentLoopError::Cancelled))
                ));

                let replay = orz_assurance::replay_journal(
                    &all_run_events_paths(&base)[0],
                    None,
                    None,
                    true,
                );
                assert!(replay.valid, "journal invalid: {:?}", replay.errors);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 slice #7: a cancelled run's journal is `run_cancelled`-terminal
    /// and valid; the next prompt still gets a FRESH run dir with a valid
    /// journal (the run-id counter advances on the cancelled path too — the
    /// reserved-counter invariant from the 2026-08-05 orz-tui review P2-2).
    #[tokio::test]
    async fn cancel_then_next_prompt_gets_fresh_valid_journal() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // run #1 consumes 1–2 texts (cancelled mid-round); run #2
                // needs 2 (counterexample gate + final answer).
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第二轮回答。"),
                        ScriptedResponse::text("第二轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                )));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-seq",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let srv = server.clone();
                let run1 = tokio::task::spawn_local(async move {
                    srv.handle_session_prompt("sess-seq", "first").await
                });
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert!(server.cancel_current_run("sess-seq"));
                let result = run1.await.unwrap();
                assert!(matches!(
                    result,
                    Err(AcpError::AgentLoop(AgentLoopError::Cancelled))
                ));

                let result2 = server
                    .handle_session_prompt("sess-seq", "second")
                    .await
                    .unwrap();
                assert_eq!(result2["status"], "completed");

                let journals = all_run_events_paths(&base);
                assert_eq!(journals.len(), 2, "two runs, two journals");
                for (path, terminal) in [
                    (&journals[0], "run_cancelled"),
                    (&journals[1], "run_finished"),
                ] {
                    let replay = orz_assurance::replay_journal(path, None, None, true);
                    assert!(replay.valid, "{path:?} invalid: {:?}", replay.errors);
                    assert_eq!(replay.terminal_event.as_deref(), Some(terminal), "{path:?}");
                }

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Phase 3 slice #7: an ACP `session/cancel` notification over the stdio
    /// handler aborts the in-flight prompt and the prompt request resolves
    /// with `StopReason::Cancelled` (the protocol contract for cancellation —
    /// a success response, not an error).
    #[tokio::test]
    async fn stdio_cancel_notification_yields_cancelled_stop_reason() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = Arc::new(AcpServer::with_gateway(Arc::new(
                    FakeProvider::new(vec![
                        ScriptedResponse::text("第一轮回答。"),
                        ScriptedResponse::text("第一轮回答。"),
                    ])
                    .with_chunk_delay(std::time::Duration::from_millis(100)),
                )));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-stdio-cancel",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let handler = Arc::new(StdioAgentHandler::with_trust_policy(
                    server.clone(),
                    crate::session::TrustPolicy::Skip,
                ));

                let h = handler.clone();
                let prompt_task = tokio::task::spawn_local(async move {
                    h.handle_request(acp::ClientRequest::PromptRequest(acp::PromptRequest::new(
                        "sess-stdio-cancel",
                        vec![acp::ContentBlock::Text(acp::TextContent::new("hello"))],
                    )))
                    .await
                });
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                handler
                    .handle_notification(acp::ClientNotification::CancelNotification(
                        acp::CancelNotification::new(acp::SessionId::new(
                            "sess-stdio-cancel".to_string(),
                        )),
                    ))
                    .await
                    .unwrap();

                match prompt_task.await.unwrap().unwrap() {
                    acp::AgentResponse::PromptResponse(p) => assert_eq!(
                        p.stop_reason,
                        acp::StopReason::Cancelled,
                        "cancel must resolve the prompt with StopReason::Cancelled"
                    ),
                    other => panic!("unexpected response: {other:?}"),
                }

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    // ── Slice #8: IP5 restore entry (P2-3 closure) ───────────────────────

    /// The restore journal path for `session_id` restore number `n`.
    fn restore_journal(base: &Path, session_id: &str, n: u64) -> PathBuf {
        let session8: String = session_id.chars().take(8).collect();
        base.join(".gsa")
            .join("runs")
            .join(format!("RST-{session8}-{n}"))
            .join("events.jsonl")
    }

    fn read_journal(path: &PathBuf) -> Vec<orz_assurance::RunEvent> {
        std::fs::read_to_string(path)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect()
    }

    /// Full restore: the worktree is restored from the snapshot, and the
    /// restore is its own run journal (`RST-{suffix}-{n}`) with a complete
    /// valid chain — run_preflight → snapshot_restored → run_finished.
    #[tokio::test]
    async fn restore_snapshot_full_restore_records_valid_chain() {
        let base = test_dir();
        let target = base.join("a.txt");
        std::fs::write(&target, "v1").unwrap();

        let store = orz_assurance::session::snapshot::SnapshotStore::new(
            base.join(".gsa").join("snapshots"),
            base.clone(),
        )
        .unwrap();
        let record = store.track(&[PathBuf::from("a.txt")]).await.unwrap();
        std::fs::write(&target, "v2").unwrap();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-restore-1",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        let report = server
            .restore_snapshot("sess-restore-1", &record.snapshot_hash, None)
            .await
            .expect("full restore");
        assert_eq!(report["status"], "restored");
        assert_eq!(
            std::fs::read_to_string(&target).unwrap(),
            "v1",
            "worktree must be restored"
        );

        let journal_path = restore_journal(&base, "sess-restore-1", 0);
        let replay = orz_assurance::replay_journal(&journal_path, None, None, true);
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));
        let events = read_journal(&journal_path);
        let types: Vec<String> = events.iter().map(|e| e.event_type.to_string()).collect();
        assert_eq!(
            types,
            ["run_preflight", "snapshot_restored", "run_finished"]
        );
        let payload = &events[1].payload;
        assert_eq!(payload["snapshot_hash"], record.snapshot_hash);
        assert_eq!(payload["restored"], serde_json::json!(["a.txt"]));
        assert!(
            payload.get("scope").is_none(),
            "full restore has no scope key"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Selective revert: only the scoped paths come back; the scope and the
    /// actually-restored paths are both recorded in the payload.
    #[tokio::test]
    async fn restore_snapshot_selective_revert_only_restores_scoped_paths() {
        let base = test_dir();
        let a = base.join("a.txt");
        let b = base.join("b.txt");
        std::fs::write(&a, "a-v1").unwrap();
        std::fs::write(&b, "b-v1").unwrap();

        let store = orz_assurance::session::snapshot::SnapshotStore::new(
            base.join(".gsa").join("snapshots"),
            base.clone(),
        )
        .unwrap();
        let record = store
            .track(&[PathBuf::from("a.txt"), PathBuf::from("b.txt")])
            .await
            .unwrap();
        std::fs::write(&a, "a-v2").unwrap();
        std::fs::write(&b, "b-v2").unwrap();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-revert",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        let report = server
            .restore_snapshot(
                "sess-revert",
                &record.snapshot_hash,
                Some(vec![PathBuf::from("a.txt")]),
            )
            .await
            .expect("selective revert");
        assert_eq!(report["restored"], serde_json::json!(["a.txt"]));
        assert_eq!(
            std::fs::read_to_string(&a).unwrap(),
            "a-v1",
            "scoped file restored"
        );
        assert_eq!(
            std::fs::read_to_string(&b).unwrap(),
            "b-v2",
            "unscoped file untouched"
        );

        let events = read_journal(&restore_journal(&base, "sess-revert", 0));
        let payload = &events[1].payload;
        assert_eq!(payload["snapshot_hash"], record.snapshot_hash);
        assert_eq!(payload["scope"], serde_json::json!(["a.txt"]));
        assert_eq!(payload["restored"], serde_json::json!(["a.txt"]));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Fail-closed: an unknown snapshot hash records
    /// `snapshot_restored{snapshot_error}` + `run_failed` (valid chain) and
    /// returns the error — the journal is the evidence record either way.
    #[tokio::test]
    async fn restore_snapshot_unknown_hash_records_error_and_run_failed() {
        let base = test_dir();
        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-bad-hash",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        let unknown = "b".repeat(64);
        let err = server
            .restore_snapshot("sess-bad-hash", &unknown, None)
            .await
            .expect_err("unknown snapshot must fail");
        assert!(
            matches!(err, AcpError::Session(SessionError::Snapshot(_))),
            "unexpected error: {err:?}"
        );

        let journal_path = restore_journal(&base, "sess-bad-hash", 0);
        let replay = orz_assurance::replay_journal(&journal_path, None, None, true);
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));
        let events = read_journal(&journal_path);
        assert_eq!(events.len(), 3);
        assert_eq!(events[1].event_type, EventType::SnapshotRestored);
        assert!(
            events[1].payload["snapshot_error"]
                .as_str()
                .unwrap()
                .contains("b".repeat(64).as_str()),
            "error payload: {:?}",
            events[1].payload
        );
        assert_eq!(events[2].event_type, EventType::RunFailed);

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Fail-closed: an unknown session is rejected without touching disk.
    #[tokio::test]
    async fn restore_snapshot_unknown_session_rejected() {
        let base = test_dir();
        let server = AcpServer::new();
        let err = server
            .restore_snapshot("sess-nope", &"c".repeat(64), None)
            .await
            .expect_err("unknown session must fail");
        assert!(
            matches!(err, AcpError::SessionNotFound(_)),
            "unexpected error: {err:?}"
        );
        assert!(
            !base.join(".gsa").exists(),
            "no session → no journal side effects"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Fail-closed: a `..`-escaping revert scope is rejected by the store
    /// before any write, and the journal still ends on `run_failed`.
    #[tokio::test]
    async fn restore_snapshot_escape_scope_rejected_fail_closed() {
        let base = test_dir();
        let target = base.join("a.txt");
        std::fs::write(&target, "v1").unwrap();

        let store = orz_assurance::session::snapshot::SnapshotStore::new(
            base.join(".gsa").join("snapshots"),
            base.clone(),
        )
        .unwrap();
        let record = store.track(&[PathBuf::from("a.txt")]).await.unwrap();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-escape",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        let err = server
            .restore_snapshot(
                "sess-escape",
                &record.snapshot_hash,
                Some(vec![PathBuf::from("../evil.txt")]),
            )
            .await
            .expect_err("escape scope must fail");
        assert!(
            matches!(err, AcpError::Session(SessionError::Snapshot(_))),
            "unexpected error: {err:?}"
        );
        assert!(!base.parent().unwrap().join("evil.txt").exists());

        let replay = orz_assurance::replay_journal(
            &restore_journal(&base, "sess-escape", 0),
            None,
            None,
            true,
        );
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Fail-closed: a restore is rejected while the session's prompt is
    /// still running (a live cancellation token marks it) — restoring would
    /// mutate the worktree under the running agent's tools.
    #[tokio::test]
    async fn restore_snapshot_rejected_while_run_in_flight() {
        let base = test_dir();
        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-busy",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        // Simulate an in-flight prompt: a live cancellation token for the
        // session (registered at prompt start, slice #7).
        server.runs.lock().unwrap().insert(
            "sess-busy".into(),
            RunInFlight::Prompt(tokio_util::sync::CancellationToken::new()),
        );

        let err = server
            .restore_snapshot("sess-busy", &"d".repeat(64), None)
            .await
            .expect_err("in-flight restore must be rejected");
        assert!(
            matches!(err, AcpError::InvalidRequest(_)),
            "unexpected error: {err:?}"
        );
        assert!(
            !base.join(".gsa").exists(),
            "rejected before bootstrap — no journal side effects"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Slice #11 P2-2: restore-vs-restore exclusion — the per-session
    /// in-flight marker is registered atomically with the check, so a second
    /// restore for the same session is rejected instead of running
    /// concurrently over the worktree.
    #[tokio::test]
    async fn restore_snapshot_rejected_while_restore_in_flight() {
        let base = test_dir();
        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-restore-busy",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        // Simulate an in-flight restore: the marker a restore registers
        // before executing.
        server
            .runs
            .lock()
            .unwrap()
            .insert("sess-restore-busy".into(), RunInFlight::Restore);

        let err = server
            .restore_snapshot("sess-restore-busy", &"d".repeat(64), None)
            .await
            .expect_err("concurrent restore must be rejected");
        assert!(
            matches!(err, AcpError::InvalidRequest(_)),
            "unexpected error: {err:?}"
        );
        assert!(
            !base.join(".gsa").exists(),
            "rejected before bootstrap — no journal side effects"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Slice #11 P2-2: prompt-vs-restore exclusion, the prompt side — a
    /// prompt landing while a restore is in flight is rejected rather than
    /// interleaving with the worktree mutation.
    #[tokio::test]
    async fn prompt_rejected_while_restore_in_flight() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = AcpServer::new();
                server
                    .handle_session_new(
                        "sess-restore-prompt",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server
                    .runs
                    .lock()
                    .unwrap()
                    .insert("sess-restore-prompt".into(), RunInFlight::Restore);

                let result = server
                    .handle_session_prompt("sess-restore-prompt", "hello")
                    .await;
                let err = result.expect_err("prompt during restore must be rejected");
                assert!(
                    matches!(err, AcpError::InvalidRequest(_)),
                    "unexpected error: {err:?}"
                );
                assert!(
                    !matches!(
                        server.runs.lock().unwrap().get("sess-restore-prompt"),
                        Some(RunInFlight::Prompt(_))
                    ),
                    "no run token registered on the rejected path"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Slice #11 P2-2: the in-flight marker is released on every completion
    /// path — success and failure — so a later restore/prompt is never
    /// falsely rejected by a stale marker.
    #[tokio::test]
    async fn restore_releases_inflight_marker_on_success_and_failure() {
        let base = test_dir();
        let target = base.join("a.txt");
        std::fs::write(&target, "v1").unwrap();
        let store = orz_assurance::session::snapshot::SnapshotStore::new(
            base.join(".gsa").join("snapshots"),
            base.clone(),
        )
        .unwrap();
        let record = store.track(&[PathBuf::from("a.txt")]).await.unwrap();

        let server = AcpServer::new();
        server
            .handle_session_new(
                "sess-release",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        // Success path: a restore completes and releases the marker.
        server
            .restore_snapshot("sess-release", &record.snapshot_hash, None)
            .await
            .unwrap();
        assert!(
            !server.runs.lock().unwrap().contains_key("sess-release"),
            "marker released after a successful restore"
        );

        // Failure path: an unknown hash also releases the marker — the next
        // restore passes the in-flight check and fails on the hash instead
        // of being falsely rejected as concurrent.
        let err = server
            .restore_snapshot("sess-release", &"0".repeat(64), None)
            .await
            .expect_err("unknown hash still fails on the snapshot");
        assert!(
            !matches!(err, AcpError::InvalidRequest(_)),
            "no false in-flight rejection after failure: {err:?}"
        );
        assert!(
            !server.runs.lock().unwrap().contains_key("sess-release"),
            "marker released after a failed restore"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// Slice #10 review D2-4: a bootstrap failure releases the cancel token
    /// — a leaked token would make restore_snapshot's in-flight check reject
    /// every restore with a false "a run is in flight" until the next
    /// successful prompt.
    #[tokio::test]
    async fn prompt_bootstrap_failure_releases_cancel_token() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = AcpServer::new();
                server
                    .handle_session_new(
                        "sess-token",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                // Block the prompt's run dir with a FILE → bootstrap_session
                // fails after the token was registered (slice #7 inserts it
                // before bootstrap).
                let session8: String = "sess-token".chars().take(8).collect();
                std::fs::create_dir_all(base.join(".gsa").join("runs")).unwrap();
                std::fs::write(
                    base.join(".gsa")
                        .join("runs")
                        .join(format!("RUN-{session8}-0")),
                    "blocker",
                )
                .unwrap();

                let result = server.handle_session_prompt("sess-token", "x").await;
                assert!(result.is_err(), "bootstrap must fail: {result:?}");
                assert!(
                    !server.runs.lock().unwrap().contains_key("sess-token"),
                    "token released on the bootstrap-failure path"
                );
                // A restore afterwards is NOT falsely rejected as in-flight
                // (it fails on the unknown hash instead).
                let err = server
                    .restore_snapshot("sess-token", &"0".repeat(64), None)
                    .await
                    .expect_err("restore proceeds past the in-flight check");
                assert!(
                    !matches!(err, AcpError::InvalidRequest(_)),
                    "no false in-flight rejection: {err:?}"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// RST- and RUN- run ids are independent: a second restore gets
    /// `RST-…-1`, and a prompt after restores still gets `RUN-…-0` (the
    /// prompt counter is untouched — the TUI derives the next run dir from
    /// it, review P2-2 drift precedent).
    #[tokio::test]
    async fn restore_and_prompt_run_ids_are_independent() {
        // The prompt path spawns the permission-manager actor via
        // `spawn_local` — the whole body runs inside a LocalSet.
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                std::fs::write(base.join("a.txt"), "v1").unwrap();
                let store = orz_assurance::session::snapshot::SnapshotStore::new(
                    base.join(".gsa").join("snapshots"),
                    base.clone(),
                )
                .unwrap();
                let record = store.track(&[PathBuf::from("a.txt")]).await.unwrap();

                let server = AcpServer::new();
                server
                    .handle_session_new(
                        "sess-indep",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server
                    .restore_snapshot("sess-indep", &record.snapshot_hash, None)
                    .await
                    .unwrap();
                server
                    .restore_snapshot("sess-indep", &record.snapshot_hash, None)
                    .await
                    .unwrap();
                assert!(restore_journal(&base, "sess-indep", 0).is_file());
                assert!(restore_journal(&base, "sess-indep", 1).is_file());

                // A prompt after two restores still uses the prompt counter (0) —
                // the client's `run_dir_for_next_prompt` guess stays correct.
                server
                    .handle_session_prompt("sess-indep", "hi")
                    .await
                    .expect("prompt after restores");
                let session8: String = "sess-indep".chars().take(8).collect();
                let prompt_journal = base
                    .join(".gsa")
                    .join("runs")
                    .join(format!("RUN-{session8}-0"))
                    .join("events.jsonl");
                assert!(
                    prompt_journal.is_file(),
                    "prompt run id must not be displaced by restores"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }
}
