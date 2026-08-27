//! ACP JSON-RPC stdio server — the entry point for the agent communication protocol.
//!
//! Uses `agent-client-protocol` 0.10.4 + `xai-acp-lib` for gateway/channels.
//! Implements `session/new` and `session/prompt` methods.
//! Delegates agent turns to `orz_loop::AgentLoopController` via the `LoopHost` trait.
//!
//! Phase 1: scaffold — session/new + session/prompt with stub gateway.
//! Full ACP lifecycle (tool calls, permissions, notifications) in Phase 2.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use orz_assurance::{EventTrack, EventType, JournalRecorderError, Redaction, RunEvent, seal_event};
use orz_loop::AgentLoopController;
use orz_loop::acaf::AcafClient;
use orz_loop::blackboard::{ExternalRetSection, InternalRetSection};
use orz_loop::controller::RetrievalMode;
use orz_loop::gateway::model::{Message, Role};
use orz_loop::orientation::OrientationSessionState;
use orz_workspace::permission::PermissionHookTransport;

use serde::{Deserialize, Serialize};

use crate::permission::PermissionPolicy;
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

/// Grill protocol template: `{cwd}/.gsa/grill/SKILL.md` when present, else
/// the built-in default (design §3 — 模板可覆盖，改模板不发版).
fn load_grill_template(cwd: &Path) -> String {
    let custom = cwd.join(".gsa").join("grill").join("SKILL.md");
    match std::fs::read_to_string(&custom) {
        Ok(s) if !s.trim().is_empty() => s,
        _ => DEFAULT_GRILL_TEMPLATE.to_string(),
    }
}

/// Append one grill round to the session JSONL (`{episode, turn,
/// user_input, response, error?, timestamp}`). Best-effort sync write —
/// grill turns are user-paced; a failed write must not fail the turn (the
/// conversation itself is the source of truth), but the failure is
/// surfaced via tracing (2026-08-08 review D2-9).
fn append_grill_record(
    log_path: &Path,
    episode: u32,
    turn: u64,
    user_input: &str,
    response: &str,
    error: Option<&str>,
) {
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        Ok(mut f) => {
            let mut record = serde_json::json!({
                "episode": episode,
                "turn": turn,
                "user_input": user_input,
                "response": response,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            if let Some(err) = error {
                record["error"] = serde_json::json!(err);
            }
            let _ = writeln!(f, "{record}");
        }
        Err(e) => tracing::warn!("grill JSONL append failed: {e}"),
    }
}

/// Terminal record — `/grill-finish` archives the session with the
/// "shared understanding reached" summary + locked decision list.
fn append_grill_terminal(log_path: &Path, episode: u32, summary: &str) {
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        Ok(mut f) => {
            let _ = writeln!(
                f,
                "{}",
                serde_json::json!({
                    "terminal": "finished",
                    "episode": episode,
                    "summary": summary,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                })
            );
        }
        Err(e) => tracing::warn!("grill JSONL terminal append failed: {e}"),
    }
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
        let mut event = RunEvent::new_v02(
            self.run_id.clone(),
            self.seq,
            event_type,
            self.manifest_sha256.clone(),
            self.prev_hash.clone(),
            EventTrack::V02.payload_schema_id().into(),
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
    /// Per-session permission policy (slice #16): a codex thread created with
    /// `sandbox: "read-only"` runs every prompt under `PermissionPolicy::ReadOnly`
    /// — session == thread on the codex surface, so the policy rides the
    /// session object and coexists with Interactive threads sharing the server.
    policy: PermissionPolicy,
    /// Prompt counter — each prompt gets a unique run id (`RUN-{suffix}-{n}`).
    prompt_count: u64,
    /// Restore counter — each user-initiated restore is its own run with a
    /// distinct prefix (`RST-{suffix}-{n}`), so it can neither collide with
    /// prompt run ids nor desync the TUI's `run_dir_for_next_prompt` (which
    /// derives the next prompt's dir from the prompt counter; slice #8).
    restore_count: u64,
    /// GAP-INQUIRY-SPLIT (2026-08-09): session-level orientation state
    /// (ADR-0010 §4.2 — 7-round counter, persisted across prompts AND
    /// process restarts via the `{cwd}/.gsa/orientation/<session8>.json`
    /// sidecar; only an actual fire resets it). Taken out during a run,
    /// written back afterwards.
    orientation: Option<OrientationSessionState>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): the retrieval-mode + activation
    /// snapshot (ADR-0010 §3.7.1/§3.3) — persisted across prompts AND
    /// process restarts via `{cwd}/.gsa/activations/<session8>.json`.
    /// Taken out during a run, written back afterwards (orientation
    /// pattern). S2 carries the mode; per-role activation state rides the
    /// same file (S4).
    activation_snapshot: Option<StoredActivationSnapshot>,
    /// local_browser (2026-08-10): the session's browser lane handle —
    /// survives across runs (the process stays up on its isolated profile;
    /// re-injected into each run's host by the capability probe). Shut down
    /// (process tree kill + profile-dir best-effort delete) on
    /// `close_session`.
    browser: Option<crate::local_browser::SharedBrowser>,
    /// GAP-CONVERSATION-RESTORE (2026-08-10): the session conversation —
    /// persists across prompts AND process restarts via the
    /// `{cwd}/.gsa/conversations/<session8>.json` sidecar (the in-session
    /// copy is the authoritative write; the sidecar is best-effort). Taken
    /// out during a run, written back on success (orientation pattern).
    conversation: Option<Vec<Message>>,
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the persisted retrieval-mode +
/// activation snapshot. S2 (mode slice) persists `retrieval_mode` and
/// `bootstrap_transition_pending`; S4 (activation persistence) fills
/// `next_seq`/`activations`. Same sidecar discipline as the orientation
/// counter: best-effort persist, corrupt → warn, take-out only after every
/// fallible step.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredActivationSnapshot {
    schema_version: String,
    session_id: String,
    retrieval_mode: RetrievalMode,
    bootstrap_transition_pending: bool,
    /// M4 (review 2026-08-10): the persisted mode BEFORE a pending explicit
    /// selection — the transition journal reads it as the real `old_mode`.
    /// Cleared once the controller journaled the transition.
    #[serde(default)]
    previous_retrieval_mode: Option<RetrievalMode>,
    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：模式 A 机械降级
    /// 的 pending transition 元数据（authority, reason_code）——降级改写
    /// 快照时落盘，controller journal 成功后清除；防止「降级后本次 run 在
    /// journal 前失败、下轮重试退化为 session_bootstrap」丢失机械元数据。
    #[serde(default)]
    pending_transition_authority: Option<(String, String)>,
    /// S4: per-role activation sequence counters (`activation_id` suffixes).
    #[serde(default)]
    next_seq: HashMap<String, u32>,
    /// S4: live (non-Closed) activation states.
    #[serde(default)]
    activations: Vec<serde_json::Value>,
    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1, 2026-08-27)：检索分区
    /// 随快照持久化——PULL 模式下子代理全文不进主对话，跨 run 后分区是
    /// 模型唯一可追溯视图（journal/.gsa 对模型不可见）；下一 prompt /
    /// 进程重启时由 controller 重建。`None` = 该会话尚未产生该分区。
    #[serde(default)]
    internal_ret: Option<InternalRetSection>,
    #[serde(default)]
    external_ret: Option<ExternalRetSection>,
}

impl StoredActivationSnapshot {
    fn for_session(session_id: &str) -> Self {
        StoredActivationSnapshot {
            schema_version: "0.1.0-draft".to_string(),
            session_id: session_id.to_string(),
            retrieval_mode: RetrievalMode::Off,
            bootstrap_transition_pending: false,
            previous_retrieval_mode: None,
            pending_transition_authority: None,
            next_seq: HashMap::new(),
            activations: Vec::new(),
            internal_ret: None,
            external_ret: None,
        }
    }
}

/// RETRIEVAL-SUBAGENT-WIRING 审查处理：把共享 probe 结果应用到会话快照
/// ——模式 A 降级时改写模式 + 置 bootstrap transition pending + 落盘机械
/// 元数据（authority/reason_code，防「run 在 journal 前失败、下轮重试退化
/// 为 session_bootstrap」丢失元数据）。
fn apply_mode_a_outcome_to_snapshot(
    snapshot: &mut StoredActivationSnapshot,
    outcome: &crate::retrieval_mode::ModeAProbeOutcome,
) {
    if outcome.degraded {
        snapshot.retrieval_mode = outcome.effective_mode;
        snapshot.previous_retrieval_mode = outcome.previous_mode;
        snapshot.bootstrap_transition_pending = true;
        snapshot.pending_transition_authority = Some((
            "mechanical_probe".to_string(),
            "browser_launch_failed".to_string(),
        ));
        tracing::warn!(
            "retrieval mode A auto-degrade: local_browser unavailable \
             (browser_launch_failed) -> framework_fallback; transition \
             journaled at next run start"
        );
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): sidecar path for the session-level
/// activation snapshot — `{cwd}/.gsa/activations/<session8>.json` (mirrors
/// the orientation sidecar; an A-class `.gsa` write point, ADR-0009).
fn activation_sidecar_path(base_dir: &Path, session_id: &str) -> PathBuf {
    let suffix: String = session_id.chars().take(8).collect();
    base_dir
        .join(".gsa")
        .join("activations")
        .join(format!("{suffix}.json"))
}

/// Load the persisted activation snapshot. `None` = no sidecar yet.
/// Corrupt → warn, not silently discarded (orientation sidecar pattern).
fn load_activation_sidecar(base_dir: &Path, session_id: &str) -> Option<StoredActivationSnapshot> {
    let path = activation_sidecar_path(base_dir, session_id);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(state) => Some(state),
            Err(e) => {
                tracing::warn!(
                    "activation sidecar corrupt ({}): {e} — starting fresh",
                    path.display()
                );
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(
                "activation sidecar unreadable ({}): {e} — starting fresh",
                path.display()
            );
            None
        }
    }
}

/// Persist the activation snapshot — best-effort (a read-only workspace must
/// never fail the run); failures are WARNED (orientation sidecar pattern).
fn persist_activation_sidecar(base_dir: &Path, session_id: &str, state: &StoredActivationSnapshot) {
    let path = activation_sidecar_path(base_dir, session_id);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            "activation sidecar dir create failed ({}): {e}",
            parent.display()
        );
        return;
    }
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("activation sidecar write failed ({}): {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("activation sidecar serialize failed: {e}"),
    }
}

/// GAP-CONVERSATION-RESTORE (2026-08-10): the session conversation sidecar —
/// `{cwd}/.gsa/conversations/<session8>.json` (same sidecar discipline as the
/// orientation/activation sidecars; an A-class `.gsa` write point, ADR-0009).
///
/// Privacy boundary: the file holds the FULL conversation in clear text,
/// including `reasoning_content` (DeepSeek multi-turn replay requires the
/// reasoning back on assistant declaration messages — missing it 400s, the
/// F-01 lesson). The sidecar is NOT the journal evidence face: ADR-0010
/// §5.4.6 restricts reasoning/credential/private-transcript text only from
/// the journal; this file is a local session artifact outside the run
/// evidence chain, retained by the 7-day retention sweep.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredConversation {
    schema_version: String,
    session_id: String,
    messages: Vec<Message>,
}

impl StoredConversation {
    fn new(session_id: &str, messages: Vec<Message>) -> Self {
        StoredConversation {
            schema_version: "0.1.0-draft".to_string(),
            session_id: session_id.to_string(),
            messages,
        }
    }
}

/// GAP-INQUIRY-SPLIT (2026-08-09): sidecar path for the session-level
/// orientation counter — `{cwd}/.gsa/orientation/<session8>.json` (mirrors
/// the grill log pattern; an A-class `.gsa` write point, ADR-0009).
fn orientation_sidecar_path(base_dir: &Path, session_id: &str) -> PathBuf {
    let suffix: String = session_id.chars().take(8).collect();
    base_dir
        .join(".gsa")
        .join("orientation")
        .join(format!("{suffix}.json"))
}

/// Load the persisted orientation counter (ADR-0010 §4.2 — recovery resumes
/// counting; only an actual fire resets). `None` = no sidecar yet.
/// Review P2-1 (2026-08-10): a corrupt sidecar is WARNED, not silently
/// discarded — silent failure would look like "restart resets the counter"
/// to an operator who never sees a crash.
fn load_orientation_sidecar(base_dir: &Path, session_id: &str) -> Option<OrientationSessionState> {
    let path = orientation_sidecar_path(base_dir, session_id);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(state) => Some(state),
            Err(e) => {
                tracing::warn!(
                    "orientation sidecar corrupt ({}): {e} — starting the counter at 0",
                    path.display()
                );
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(
                "orientation sidecar unreadable ({}): {e} — starting the counter at 0",
                path.display()
            );
            None
        }
    }
}

/// Persist the orientation counter — best-effort (a read-only workspace must
/// never fail the run; the in-session write is the authoritative path).
/// Review P2-1: failures are WARNED (same pattern as the grill JSONL).
fn persist_orientation_sidecar(base_dir: &Path, session_id: &str, state: &OrientationSessionState) {
    let path = orientation_sidecar_path(base_dir, session_id);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            "orientation sidecar dir create failed ({}): {e}",
            parent.display()
        );
        return;
    }
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("orientation sidecar write failed ({}): {e}", path.display());
            }
        }
        Err(e) => tracing::warn!("orientation sidecar serialize failed: {e}"),
    }
}

/// GAP-CONVERSATION-RESTORE (2026-08-10): sidecar path for the session
/// conversation — `{cwd}/.gsa/conversations/<session8>.json`.
fn conversation_sidecar_path(base_dir: &Path, session_id: &str) -> PathBuf {
    let suffix: String = session_id.chars().take(8).collect();
    base_dir
        .join(".gsa")
        .join("conversations")
        .join(format!("{suffix}.json"))
}

/// Load the persisted conversation (cross-prompt AND cross-process recovery —
/// a process restart that re-creates the same `session_id` resumes the
/// conversation). `None` = no sidecar yet (brand-new session); a corrupt
/// sidecar is WARNED and discarded (same discipline as the orientation
/// counter — silent failure would look like "restart loses the history").
fn load_conversation_sidecar(base_dir: &Path, session_id: &str) -> Option<StoredConversation> {
    let path = conversation_sidecar_path(base_dir, session_id);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<StoredConversation>(&text) {
            // Review P3-3 (three-agent 2026-08-10): the envelope's
            // `session_id` is checked against the requester — an 8-char
            // prefix collision (or a hand-moved file) must not restore
            // another session's conversation.
            Ok(stored) if stored.session_id == session_id => Some(stored),
            Ok(stored) => {
                tracing::warn!(
                    "conversation sidecar session mismatch ({}): expected {session_id}, found {} — starting a fresh conversation",
                    path.display(),
                    stored.session_id
                );
                None
            }
            Err(e) => {
                tracing::warn!(
                    "conversation sidecar corrupt ({}): {e} — starting a fresh conversation",
                    path.display()
                );
                None
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            tracing::warn!(
                "conversation sidecar unreadable ({}): {e} — starting a fresh conversation",
                path.display()
            );
            None
        }
    }
}

/// Persist the session conversation — best-effort (a read-only workspace must
/// never fail the run; the in-session write is the authoritative path), same
/// discipline as the orientation/activation sidecars. An EMPTY conversation
/// is not persisted (a brand-new session has no file until its first
/// successful prompt — `load`'s NotFound → `None` naturally covers it).
fn persist_conversation_sidecar(base_dir: &Path, session_id: &str, messages: &[Message]) {
    if messages.is_empty() {
        return;
    }
    let path = conversation_sidecar_path(base_dir, session_id);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            "conversation sidecar dir create failed ({}): {e}",
            parent.display()
        );
        return;
    }
    match serde_json::to_string_pretty(&StoredConversation::new(session_id, messages.to_vec())) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!(
                    "conversation sidecar write failed ({}): {e}",
                    path.display()
                );
            }
        }
        Err(e) => tracing::warn!("conversation sidecar serialize failed: {e}"),
    }
}

/// Grill-mode session state (2026-08-08 write-placement slice, design §3):
/// the accumulated conversation (persists across turns), the turn counter,
/// and the audit JSONL path. Independent of run journals — a grill session
/// is recorded to `{cwd}/.gsa/grill/<session8>.jsonl`, never into `.gsa/runs/`
/// (run = single-run integrity unit; zero run-event schema involvement).
struct GrillSession {
    session_id: String,
    messages: Vec<Message>,
    turn: u64,
    /// Monotonic episode number (2026-08-08 review D2-7): a finished grill
    /// session followed by a new `/grill` reuses the same session JSONL —
    /// the episode field disambiguates turn numbering across episodes.
    episode: u32,
    log_path: PathBuf,
}

/// Built-in default grill protocol template (design §3): Socratic
/// questioning — one question at a time, every question carries a
/// recommended answer, decision tree depth-first, explore the codebase
/// first. Overridable via `{cwd}/.gsa/grill/SKILL.md` (改模板不发版).
const DEFAULT_GRILL_TEMPLATE: &str = "\
[GRILL 模式] 你是设计拷问者（grill-me 协议，Matt Pocock/MIT）。本次对话是设计讨论：\
一次只问一个问题；每个问题必须附带你推荐的答案（\"推荐: ...\"）；按决策树深度优先\
推进；提问前先用只读工具探索代码库（read_file/grep/list_dir）。用户输入即对上一问\
的回答。[/GRILL 模式]";

/// `/grill-finish` summary instruction — the final turn asks for the
/// "shared understanding reached" summary + the locked decision list, then
/// the session is archived.
const GRILL_FINISH_PROMPT: &str = "\
[GRILL 结束] 请输出「共享理解达成」总结：本次讨论达成的结论、锁定的决策清单（逐条）、\
以及仍待定的问题（如有）。[/GRILL 结束]";

/// The ACP server — holds active sessions and dispatches requests.
pub struct AcpServer {
    sessions: Arc<Mutex<HashMap<String, StoredSession>>>,
    /// Outbound ACP gateway (agent → client messages: permissions etc.).
    /// `None` in headless mode — permission prompts fail closed.
    gateway: Arc<Mutex<Option<AcpAgentGatewaySender>>>,
    /// Model gateway for agent turns (scripted FakeProvider offline).
    model_gateway: Arc<dyn ModelGateway>,
    /// ACAF (ADR-0011): optional signer-process client threaded into every
    /// session controller. `None` = unticketed; combined with
    /// `acaf_fail_closed` the controller refuses runs without a fabric
    /// (D-15 fail-fast).
    acaf: Option<Arc<tokio::sync::Mutex<AcafClient>>>,
    /// ACAF fail-closed enforcement for sessions started by this server
    /// (production flip 2026-08-16: the binary entrypoint sets this from
    /// `ORZ_ACAF_FAIL_CLOSED`; unset = enforced).
    acaf_fail_closed: bool,
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
    /// Grill-mode session (2026-08-08 write-placement slice, design §3):
    /// `Some` while a `/grill` session is active — at most one at a time,
    /// bound to the session it started under. Turns run the full model↔tool
    /// loop under a ReadOnly permission policy and record to the session's
    /// grill JSONL (never a run journal).
    grill: Mutex<Option<GrillSession>>,
    /// Monotonic grill-episode counter (review D2-7): never reused, so each
    /// `/grill` episode is distinguishable in the shared session JSONL even
    /// after a finish/clear (turn numbers restart per episode).
    grill_episode: AtomicU32,
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
        // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, 实际使用裁决)：
        // plan 门已普适摘除——canned provider 不再应答 plan_write，首轮
        // 直接产出正文；反例自查门（§4.6）仍在终答前加一轮。
        Self::with_gateway(Arc::new(FakeProvider::new(vec![
            orz_loop::gateway::fake::ScriptedResponse::text("(fake) 已收到请求。"),
            orz_loop::gateway::fake::ScriptedResponse::text("(fake) 已收到请求。"),
        ])))
    }

    pub fn with_gateway(model_gateway: Arc<dyn ModelGateway>) -> Self {
        AcpServer {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            gateway: Arc::new(Mutex::new(None)),
            model_gateway,
            acaf: None,
            acaf_fail_closed: false,
            runs: Arc::new(Mutex::new(HashMap::new())),
            pending_cancels: Arc::new(Mutex::new(HashMap::new())),
            hub_permission: Arc::new(Mutex::new(None)),
            grill: Mutex::new(None),
            grill_episode: AtomicU32::new(0),
        }
    }

    /// ACAF (ADR-0011): optional signer-process client threaded into every
    /// session controller (same shape as the CLI run path). `None` keeps the
    /// zero-behaviour-change unticketed default.
    pub fn with_acaf(mut self, acaf: Option<Arc<tokio::sync::Mutex<AcafClient>>>) -> Self {
        self.acaf = acaf;
        self
    }

    /// ACAF fail-closed enforcement for sessions started by this server
    /// (D-14/D-15/D-16 semantics — an unconfigured fabric under fail-closed
    /// refuses runs at the controller boundary).
    pub fn with_acaf_fail_closed(mut self, fail_closed: bool) -> Self {
        self.acaf_fail_closed = fail_closed;
        self
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
        self.handle_session_new_with_policy(session_id, base_dir, trust_policy, Default::default())
            .await
    }

    /// `handle_session_new` variant that additionally fixes the session's
    /// permission policy (slice #16): the codex app-server passes
    /// `PermissionPolicy::ReadOnly` for `sandbox: "read-only"` threads and the
    /// default `Interactive` otherwise; every other caller keeps the
    /// no-policy behavior.
    pub async fn handle_session_new_with_policy(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
        trust_policy: crate::session::TrustPolicy,
        policy: PermissionPolicy,
    ) -> Result<serde_json::Value, AcpError> {
        self.handle_session_new_with_options(session_id, base_dir, trust_policy, policy, None)
            .await
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): `retrieval_mode` is the
    /// session-level explicit mode (ADR-0010 §3.7.1 — `Some` = explicit user
    /// / parent-task-contract selection; `None` = the `off` default). An
    /// explicit selection different from the persisted mode marks a bootstrap
    /// transition pending (journaled on the next run's startup). The
    /// activation sidecar (mode + per-role activations) resumes across
    /// process restarts.
    pub async fn handle_session_new_with_options(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
        trust_policy: crate::session::TrustPolicy,
        policy: PermissionPolicy,
        retrieval_mode: Option<RetrievalMode>,
    ) -> Result<serde_json::Value, AcpError> {
        // Journals are created per-run at `session/prompt` time — the session
        // itself only records where, under what trust policy, and under what
        // permission policy runs live.
        let base = base_dir.unwrap_or_else(|| PathBuf::from("."));
        // GAP-INQUIRY-SPLIT: resume the orientation counter from the sidecar
        // when a previous process left one (ADR-0010 §4.2 — recovery resumes
        // counting; only an actual fire resets). A brand-new session starts
        // at 0.
        let orientation = load_orientation_sidecar(&base, session_id)
            .unwrap_or_else(|| OrientationSessionState::new(session_id));
        // GAP-RETRIEVAL-TOOLS: resume the activation sidecar; an explicit
        // mode selection that differs from the persisted mode marks a
        // bootstrap transition pending (off → mode, journaled once).
        let mut activation_snapshot = load_activation_sidecar(&base, session_id)
            .unwrap_or_else(|| StoredActivationSnapshot::for_session(session_id));
        if let Some(mode) = retrieval_mode
            && mode != activation_snapshot.retrieval_mode
        {
            // M4 (review 2026-08-10): every explicit mode change journals —
            // including a change TO off (a transition like any other,
            // §3.7.1 "never implicit"); the previous persisted mode is
            // carried as the transition's real old_mode.
            activation_snapshot.previous_retrieval_mode = Some(activation_snapshot.retrieval_mode);
            activation_snapshot.retrieval_mode = mode;
            activation_snapshot.bootstrap_transition_pending = true;
        }
        // GAP-CONVERSATION-RESTORE: resume the conversation sidecar when a
        // previous process left one (cross-process continuation); a brand-new
        // session has no conversation (`None` — the first prompt seeds a
        // single-message context).
        let conversation = load_conversation_sidecar(&base, session_id).map(|s| s.messages);
        self.sessions.lock().unwrap().insert(
            session_id.to_string(),
            StoredSession {
                base_dir: base,
                trust_policy,
                policy,
                prompt_count: 0,
                restore_count: 0,
                orientation: Some(orientation),
                activation_snapshot: Some(activation_snapshot),
                browser: None,
                conversation: conversation.clone(),
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
        let (base_dir, trust_policy, policy, prompt_number) = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            let n = session.prompt_count;
            session.prompt_count += 1;
            (
                session.base_dir.clone(),
                session.trust_policy,
                session.policy,
                n,
            )
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
        let mut host = self.build_host(&handle, session_id, &base_dir, policy)?;
        // GAP-INQUIRY-SPLIT (review P1-1, 2026-08-10): take the orientation
        // counter out of the session ONLY after every fallible step above
        // (restore-in-flight check, bootstrap, build_host) has succeeded —
        // an early `?` return must never leave the session with a taken-out
        // counter (the next prompt would silently restart at 0 and the
        // sidecar would be overwritten — violating "only an actual fire
        // resets", ADR-0010 §4.2). The sidecar is the fallback for a session
        // created before this slice (`None`): resume from it, else start 0.
        let mut orientation = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            session.orientation.take().unwrap_or_else(|| {
                load_orientation_sidecar(&base_dir, session_id)
                    .unwrap_or_else(|| OrientationSessionState::new(session_id))
            })
        };
        // GAP-RETRIEVAL-TOOLS: take out the activation snapshot (mode +
        // pending + activations) with the orientation counter — same
        // discipline: only after every fallible step, so an early `?` never
        // leaves the session with a taken-out snapshot.
        let mut activation_snapshot = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            session.activation_snapshot.take().unwrap_or_else(|| {
                load_activation_sidecar(&base_dir, session_id)
                    .unwrap_or_else(|| StoredActivationSnapshot::for_session(session_id))
            })
        };
        // GAP-CONVERSATION-RESTORE: take out the session conversation with
        // the orientation/activation state — same discipline: only after
        // every fallible step above, so an early `?` never leaves the
        // session with a taken-out conversation (the next prompt would
        // silently restart from zero and the sidecar would be overwritten).
        let mut conversation = {
            let mut sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get_mut(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            session.conversation.take().unwrap_or_else(|| {
                load_conversation_sidecar(&base_dir, session_id)
                    .map(|s| s.messages)
                    .unwrap_or_default()
            })
        };
        // IP5: attach the session's pre-mutation snapshot store — mutation
        // tools with knowable targets get tracked before execution.
        // local_browser (2026-08-10): re-inject the session's browser lane
        // (launched on a previous prompt — the process stays up across runs
        // on its isolated profile) BEFORE the probe runs.
        if let Some(browser) = self
            .sessions
            .lock()
            .unwrap()
            .get(session_id)
            .and_then(|s| s.browser.clone())
        {
            host.set_browser_session(browser);
        }
        // RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：外部
        // 子代理=模式 A 自动定档（共享入口 retrieval_mode::probe_retrieval_
        // with_mode_a）——local_browser probe 失败（浏览器启动失败）机械
        // 降级 framework_fallback：改写快照模式 + 置 bootstrap transition
        // pending + 落盘机械元数据（authority/reason，防跨 run 重试丢失）。
        // 降级后 capability 已由共享入口以 framework_fallback 的真实能力
        // 重探（transition 的 capability_status 不残留浏览器失败原因）。
        let mode_a_outcome = crate::retrieval_mode::probe_retrieval_with_mode_a(
            activation_snapshot.retrieval_mode,
            host.web_search_configured(),
            &mut host,
            &base_dir,
            session_id,
        )
        .await;
        apply_mode_a_outcome_to_snapshot(&mut activation_snapshot, &mode_a_outcome);
        // GAP-RETRIEVAL-TOOLS (S4): seed the activation registry from the
        // sidecar — a cross-run AwaitingDisposition activation is restored
        // and journaled (the parent may dispose it in this run). 序列化在
        // 模式 A 降级之后——controller 拿到的是降级后的快照（retrieval_mode
        // = framework_fallback + bootstrap_transition_pending=true）。
        let activation_snapshot_json =
            serde_json::to_value(&activation_snapshot).unwrap_or(serde_json::Value::Null);
        // Freshly-launched browser (the probe injected it): persist the
        // handle back onto the session so it survives across runs. An
        // existing handle is never replaced.
        if let Some(session) = self.sessions.lock().unwrap().get_mut(session_id)
            && session.browser.is_none()
        {
            session.browser = Some(host.browser_session().clone());
        }
        let controller = AgentLoopController::with_gateway(self.model_gateway.clone())
            // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, 实际使用
            // 裁决)：plan 门为普适删减——ACP 交互会话面与 CLI 一致不再
            // 启用首轮计划仪式（安全职责在机械审计层与提交门，不靠计划
            // 结构）；`plan_first` 仅作休眠开关保留（测试/回退），生产
            // 不再启用。console 默认面随生产路径保留（与 CLI 一致）。
            .with_console_default_enabled(true)
            .with_snapshot_store(Some(handle.snapshot_store.clone()))
            // ACAF production flip (2026-08-16): the ACP session path shares
            // the signer-process client + fail-closed posture of the CLI run
            // path (previously the ACP path ran unticketed).
            .with_acaf(self.acaf.clone())
            .with_acaf_fail_closed(self.acaf_fail_closed)
            .with_retrieval_mode(
                activation_snapshot.retrieval_mode,
                mode_a_outcome.capability,
                activation_snapshot.bootstrap_transition_pending,
                Some(session_id.to_string()),
                activation_snapshot.previous_retrieval_mode.clone(),
                activation_snapshot.pending_transition_authority.clone(),
            )
            .with_activation_snapshot(Some(&activation_snapshot_json))
            // THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1, 2026-08-27)：恢复
            // 时重建检索分区——PULL 模式下全文不进主对话，重启后分区是
            // 模型唯一可追溯视图；sidecar 快照携带的 internal_ret /
            // external_ret 灌回全新黑板的对应分区（None = 无历史）。
            .with_retrieval_partitions(
                activation_snapshot.internal_ret.clone(),
                activation_snapshot.external_ret.clone(),
            );

        let run_result = controller
            .run_turn_with_cancel(
                &host,
                prompt,
                &handle.run_id,
                &handle.run_manifest_sha256,
                handle.next_sequence,
                handle.last_event_sha256.clone(),
                Some(&cancel),
                Some(&mut orientation),
                Some(&mut conversation),
            )
            .await;

        // Every path: release the run token (stale cancels become no-ops)
        // and the journal writer task (previously only the success path
        // released it).
        self.runs.lock().unwrap().remove(session_id);
        let _ = handle.journal.shutdown_async().await;

        // GAP-INQUIRY-SPLIT: persist the orientation counter — back into the
        // session and (best-effort, a read-only workspace must never fail a
        // run) to the sidecar, so the next prompt / process restart resumes
        // counting (§4.2).
        persist_orientation_sidecar(&base_dir, session_id, &orientation);
        if let Some(session) = self.sessions.lock().unwrap().get_mut(session_id) {
            session.orientation = Some(orientation);
        }
        // GAP-RETRIEVAL-TOOLS: persist the activation snapshot (mode +
        // pending + activations). The bootstrap transition flag clears once
        // the controller journaled it (the transition is written exactly
        // once; a failed run before it keeps the flag for the next prompt).
        if controller.bootstrap_transition_journaled() {
            activation_snapshot.bootstrap_transition_pending = false;
            // M4: the transition's old_mode is consumed — the next change
            // records its own previous value.
            activation_snapshot.previous_retrieval_mode = None;
            // RETRIEVAL-SUBAGENT-WIRING 审查处理：pending 机械元数据随
            // transition 消费清除（已落盘的 authority/reason 不再保留）。
            activation_snapshot.pending_transition_authority = None;
        }
        // S4: fold the controller's live registry back into the snapshot
        // (next_seq + non-Closed activations; Closed excluded).
        let live = controller.activation_snapshot_json(&handle.run_id);
        if let (Some(next_seq), Some(activations)) = (
            live.get("next_seq").cloned(),
            live.get("activations").cloned(),
        ) {
            activation_snapshot.next_seq = serde_json::from_value(next_seq).unwrap_or_default();
            activation_snapshot.activations =
                serde_json::from_value(activations).unwrap_or_default();
        }
        // THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1, 2026-08-27): fold the
        // controller's retrieval partitions back into the snapshot. 全量
        // 覆盖语义下，取回值 = 本次 run 最后完成的一次派发结果；派发级
        // 失败（子代理 loop Err）不写分区，故保留灌回的历史值——不存在
        // 半成品覆盖。下个 prompt / 进程重启经 with_retrieval_partitions
        // 重建。
        {
            let bb = controller.blackboard().read();
            activation_snapshot.internal_ret = Some(bb.internal_ret.clone());
            activation_snapshot.external_ret = Some(bb.external_ret.clone());
        }
        persist_activation_sidecar(&base_dir, session_id, &activation_snapshot);
        if let Some(session) = self.sessions.lock().unwrap().get_mut(session_id) {
            session.activation_snapshot = Some(activation_snapshot);
        }
        // GAP-CONVERSATION-RESTORE: persist the conversation — back into the
        // session and (best-effort, a read-only workspace must never fail a
        // run) to the sidecar, so the next prompt / process restart resumes
        // the history. SUCCESS-ONLY: a failed run keeps the pre-run history
        // (the run's partial messages never enter the conversation — the
        // journal is the failure evidence). The seed was a clone, so a failed
        // run leaves the caller's copy byte-identical.
        //
        // Review P3-1 (three-agent 2026-08-10): the in-session write sits in
        // the SAME `is_ok` branch as the sidecar — a failure (incl. the
        // `journal.flush_async` error surfaced from `run_turn_with_guards`)
        // leaves `session.conversation` at `None`, so the next prompt's
        // take-out falls back to the sidecar (the pre-run history) instead
        // of diverging from it.
        if run_result.is_ok() {
            persist_conversation_sidecar(&base_dir, session_id, &conversation);
            if let Some(session) = self.sessions.lock().unwrap().get_mut(session_id) {
                session.conversation = Some(conversation);
            }
        }

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

    /// Grill-mode turn (2026-08-08 write-placement slice, design §3): run
    /// the user's answer through the full model↔tool loop under a ReadOnly
    /// permission policy ("先探索代码库" is the protocol's core), persisting
    /// the session's conversation to `{cwd}/.gsa/grill/<session8>.jsonl`.
    ///
    /// Independent of run integrity units: no run journal is touched and no
    /// run is registered in the in-flight map — a grill turn is not
    /// cancellable via `session/cancel`, and the TUI sequences it between
    /// runs (a run in flight → `InvalidRequest`).
    ///
    /// The first turn injects the protocol template: `{cwd}/.gsa/grill/
    /// SKILL.md` when present, else the built-in default (design §3 —
    /// 改模板不发版).
    pub async fn run_grill_turn(
        &self,
        session_id: &str,
        user_input: &str,
    ) -> Result<String, AcpError> {
        let (base_dir, trust_policy) = {
            let sessions = self.sessions.lock().unwrap();
            let session = sessions
                .get(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            (session.base_dir.clone(), session.trust_policy)
        };
        if self.runs.lock().unwrap().contains_key(session_id) {
            return Err(AcpError::InvalidRequest(
                "grill turn rejected: a run is in flight for this session".into(),
            ));
        }
        // Session init + template: `Some` on the first turn (template
        // injected once — design §3, "会话开始"). A session switch rebinds
        // the grill session to the new session_id (at most one active).
        let (run_id, template) = {
            let mut grill = self.grill.lock().unwrap();
            let suffix: String = session_id.chars().take(8).collect();
            let entry = grill.get_or_insert_with(|| GrillSession {
                session_id: session_id.to_string(),
                messages: Vec::new(),
                turn: 0,
                episode: self.grill_episode.fetch_add(1, Ordering::SeqCst) + 1,
                log_path: base_dir
                    .join(".gsa")
                    .join("grill")
                    .join(format!("{suffix}.jsonl")),
            });
            if entry.session_id != session_id {
                *entry = GrillSession {
                    session_id: session_id.to_string(),
                    messages: Vec::new(),
                    turn: 0,
                    episode: self.grill_episode.fetch_add(1, Ordering::SeqCst) + 1,
                    log_path: base_dir
                        .join(".gsa")
                        .join("grill")
                        .join(format!("{suffix}.jsonl")),
                };
            }
            let tpl = (entry.turn == 0).then(|| load_grill_template(&base_dir));
            (format!("GRILL-{suffix}-{}", entry.turn), tpl)
        };

        // Same host shape as a prompt run, but policy is ALWAYS ReadOnly
        // (write tools Deny before any permission wire — no dialogs).
        let bootstrap = bootstrap_session(&run_id, Some(base_dir.clone()), trust_policy).await?;
        let host = self.build_host(
            &bootstrap,
            session_id,
            &base_dir,
            PermissionPolicy::ReadOnly,
        )?;
        let controller = AgentLoopController::with_gateway(self.model_gateway.clone())
            .with_snapshot_store(Some(bootstrap.snapshot_store.clone()))
            // ACAF production flip (2026-08-16): grill turns run read-only
            // (no action tickets), but the controller shares the server's
            // fail-closed posture so an unconfigured fabric refuses loudly
            // instead of silently degrading.
            .with_acaf(self.acaf.clone())
            .with_acaf_fail_closed(self.acaf_fail_closed);

        // 2026-08-08 review P2-2: the turn runs OUTSIDE the `grill` lock —
        // no std MutexGuard lives across an await (a concurrent caller
        // would deadlock the single-threaded LocalSet). The messages are
        // taken out and written back after the turn.
        let mut messages = {
            let mut grill = self.grill.lock().unwrap();
            let entry = grill
                .as_mut()
                .expect("grill session initialized above (single-threaded TUI)");
            std::mem::take(&mut entry.messages)
        };
        let result = controller
            .run_grill_turn(&host, &mut messages, user_input, template.as_deref(), None)
            .await;
        let _ = bootstrap.journal.shutdown_async().await;

        match result {
            Ok(response) => {
                // Audit record (best-effort; the JSONL is append-only
                // Q/A/recommendation log — zero run-event schema involvement,
                // design §3).
                let mut grill = self.grill.lock().unwrap();
                let entry = grill.as_mut().expect("grill session still active");
                entry.messages = messages;
                append_grill_record(
                    &entry.log_path,
                    entry.episode,
                    entry.turn,
                    user_input,
                    &response,
                    None,
                );
                entry.turn += 1;
                Ok(response)
            }
            Err(e) => {
                // 2026-08-08 review D2-5: a failed turn must not silently
                // eat the user's answer — the conversation stays continuous
                // (the answer is appended to the history) and the failure is
                // audited in the JSONL. The turn counter advances so a retry
                // gets a fresh GRILL-* dir (no duplicated seq-0 preflight).
                let mut grill = self.grill.lock().unwrap();
                let entry = grill.as_mut().expect("grill session still active");
                messages.push(Message {
                    role: Role::User,
                    content: user_input.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                entry.messages = messages;
                append_grill_record(
                    &entry.log_path,
                    entry.episode,
                    entry.turn,
                    user_input,
                    "",
                    Some(&e.to_string()),
                );
                entry.turn += 1;
                Err(AcpError::AgentLoop(e))
            }
        }
    }

    /// Grill-mode finish (design §3): one final turn asking for the
    /// "shared understanding reached" summary + locked decision list, then
    /// the session is archived (terminal record in the grill JSONL) and
    /// cleared. No active grill session → `InvalidRequest`.
    pub async fn finish_grill(&self, session_id: &str) -> Result<String, AcpError> {
        let active = self
            .grill
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|g| g.session_id == session_id);
        if !active {
            return Err(AcpError::InvalidRequest(
                "no active grill session for this session id".into(),
            ));
        }
        let response = self.run_grill_turn(session_id, GRILL_FINISH_PROMPT).await?;
        if let Some(g) = self.grill.lock().unwrap().take() {
            append_grill_terminal(&g.log_path, g.episode, &response);
        }
        Ok(response)
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
        let removed = {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.remove(session_id)
        };
        // local_browser (2026-08-10): tear the browser down (process-tree
        // kill + best-effort profile-dir delete). `close_session` is sync —
        // the shutdown runs detached (best-effort; an orphaned profile dir
        // is covered by the A5 retention sweep on `chrome-profile-*`).
        if let Some(browser) = removed.as_ref().and_then(|s| s.browser.clone()) {
            tokio::spawn(async move {
                browser.shutdown().await;
            });
        }
        // Release cancellation state too — a remembered cancel must not
        // outlive its session (2026-08-05 review P2-1). A stray run/restore
        // marker is dropped the same way (slice #11, P2-2).
        self.runs.lock().unwrap().remove(session_id);
        self.pending_cancels.lock().unwrap().remove(session_id);
        removed.is_some()
    }

    /// Build the real host for a run: finalized GrokBuild toolset +
    /// workspace-trust observation + IP6 permission bridge.
    ///
    /// The bridge consumes the outbound ACP gateway when one is wired
    /// (`--stdio`); otherwise a fail-closed dead gateway — Read auto-allows,
    /// Bash `Ask` → `Deny` (IP6 headless semantics). `policy` fixes the
    /// bridge's per-session behavior (slice #16): a read-only session denies
    /// mutation/network without prompting.
    fn build_host(
        &self,
        handle: &crate::session::SessionHandle,
        session_id: &str,
        base_dir: &std::path::Path,
        policy: PermissionPolicy,
    ) -> Result<crate::OrzHost, AcpError> {
        // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, 实际使用裁决)：
        // ACP 交互路径工具超时逃生舱——此前仅 CLI/-p 读
        // ORZ_TOOL_TIMEOUT_SECS，TUI/stdio 走 host 默认 300s，实际使用中
        // 长命令（10 分钟级构建/测试）被机械超时误杀。按实际使用对齐：
        // 缺失保持默认 300s；0 = 不限制；非法值显式报错（同 main.rs
        // 纪律）。桌面进程未设置时行为与之前完全一致。
        let tool_timeout = std::env::var("ORZ_TOOL_TIMEOUT_SECS")
            .ok()
            .map(|s| s.trim().parse::<u64>())
            .transpose()
            .map_err(|_| {
                AcpError::Host("ORZ_TOOL_TIMEOUT_SECS must be a number of seconds".to_string())
            })?
            .filter(|&s| s > 0)
            .map(std::time::Duration::from_secs);
        // The permission manager requires an absolute cwd (AbsPathBuf) —
        // canonicalize, falling back to the raw path on failure. dunce strips
        // the `\\?\` verbatim prefix on Windows (clippy.toml ban on std).
        let cwd = dunce::canonicalize(base_dir).unwrap_or_else(|_| base_dir.to_path_buf());
        // P1 permit keystore: the session's DPAPI-backed signer (or the
        // test-only memory store under TrustPolicy::Skip).
        let mut host = crate::OrzHost::with_bridge_and_hub_policy(
            session_id,
            handle.journal.clone(),
            &cwd,
            handle.workspace_trust,
            self.gateway(),
            self.hub_permission.lock().unwrap().clone(),
            policy,
        )
        .map_err(AcpError::Host)?
        .with_permit_signer(handle.permit_signer.clone());
        if let Some(timeout) = tool_timeout {
            host = host.with_tool_timeout(timeout);
        }
        Ok(host)
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
    use crate::local_browser::ORZ_BROWSER_PATH_ENV;
    use crate::permission::dead_gateway;
    use crate::retrieval_mode::probe_retrieval_capability;
    use crate::stdio::StdioAgentHandler;
    use agent_client_protocol as acp;
    use agent_client_protocol::MessageHandler;
    use orz_assurance::EventType;
    use orz_assurance::gates::ipg::WorkspaceTrust;
    use orz_loop::controller::AgentLoopError;
    use orz_loop::controller::RetrievalCapability;
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

    /// RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：模式 A 降级结果
    /// 应用到会话快照——改写模式 + transition pending + 机械元数据落盘；
    /// 未降级时快照不动。降级决策规则本体在
    /// `retrieval_mode::mode_a_degrade_rules`（共享入口，ACP/CLI 共用）。
    #[test]
    fn mode_a_outcome_applies_to_snapshot() {
        // 降级 → 快照改写 + pending + 机械元数据。
        let mut snap = StoredActivationSnapshot::for_session("sess-a");
        snap.retrieval_mode = RetrievalMode::LocalBrowser;
        apply_mode_a_outcome_to_snapshot(
            &mut snap,
            &crate::retrieval_mode::ModeAProbeOutcome {
                effective_mode: RetrievalMode::FrameworkFallback,
                capability: RetrievalCapability::Available,
                previous_mode: Some(RetrievalMode::LocalBrowser),
                degraded: true,
            },
        );
        assert_eq!(snap.retrieval_mode, RetrievalMode::FrameworkFallback);
        assert_eq!(
            snap.previous_retrieval_mode,
            Some(RetrievalMode::LocalBrowser)
        );
        assert!(snap.bootstrap_transition_pending);
        assert_eq!(
            snap.pending_transition_authority,
            Some((
                "mechanical_probe".to_string(),
                "browser_launch_failed".to_string()
            ))
        );

        // 未降级 → 快照不动（显式选择路径语义不变）。
        let mut snap = StoredActivationSnapshot::for_session("sess-b");
        snap.retrieval_mode = RetrievalMode::LocalBrowser;
        apply_mode_a_outcome_to_snapshot(
            &mut snap,
            &crate::retrieval_mode::ModeAProbeOutcome {
                effective_mode: RetrievalMode::LocalBrowser,
                capability: RetrievalCapability::Available,
                previous_mode: None,
                degraded: false,
            },
        );
        assert_eq!(snap.retrieval_mode, RetrievalMode::LocalBrowser);
        assert!(!snap.bootstrap_transition_pending);
        assert_eq!(snap.pending_transition_authority, None);
    }

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1)：检索分区随快照 serde
    /// 往返（sidecar 持久化/恢复不丢全文与条目）；旧版快照（无该字段）
    /// 反序列化为 None——恢复时保持空分区而非报错。
    #[test]
    fn activation_snapshot_roundtrips_retrieval_partitions() {
        let internal = InternalRetSection {
            project_docs: vec!["design.md".to_string()],
            source_ledger: vec!["SRC-001 design.md".to_string()],
            response: Some("跨 run 的检索完成".to_string()),
        };
        let external = ExternalRetSection {
            web_sources: vec!["https://example.com/paper".to_string()],
            source_ledger: vec!["SRC-002 https://example.com/paper".to_string()],
            response: Some("跨 run 的网页检索完成".to_string()),
        };
        let mut snap = StoredActivationSnapshot::for_session("sess-p31");
        snap.internal_ret = Some(internal.clone());
        snap.external_ret = Some(external.clone());

        let json = serde_json::to_value(&snap).unwrap();
        let restored: StoredActivationSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(restored.internal_ret, Some(internal));
        assert_eq!(restored.external_ret, Some(external));

        // 旧版 sidecar（无分区字段）→ None，向后兼容。
        let legacy = serde_json::json!({
            "schema_version": "0.1.0-draft",
            "session_id": "sess-legacy",
            "retrieval_mode": "off",
            "bootstrap_transition_pending": false,
            "next_seq": {},
            "activations": [],
        });
        let legacy_snap: StoredActivationSnapshot = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy_snap.internal_ret, None);
        assert_eq!(legacy_snap.external_ret, None);
    }

    /// local_browser (2026-08-10): the probe reuses an already-ready lane
    /// (Available without re-launching), fails Degraded with a subdivided
    /// cause when the browser cannot start, and keeps the framework_fallback
    /// semantics untouched (off → Unsupported; missing key → Degraded).
    #[tokio::test]
    async fn probe_retrieval_capability_three_states() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());

        // LocalBrowser + ready lane → Available (no re-launch).
        let stub = crate::local_browser::tests::ready_stub_browser();
        let mut host = crate::OrzHost::new(journal.clone(), &dir, WorkspaceTrust::ObservedTrusted)
            .unwrap()
            .with_browser_session(stub);
        let cap = probe_retrieval_capability(
            RetrievalMode::LocalBrowser,
            true,
            &mut host,
            &dir,
            "RUN-TEST01",
        )
        .await;
        assert_eq!(cap, RetrievalCapability::Available);

        // LocalBrowser + no lane + ORZ_BROWSER_PATH pointing nowhere →
        // Degraded("browser_launch_failed: browser_not_found: ...").
        let missing = dir.join("no-such-browser.exe");
        // SAFETY: test-only; parallel tests read the env through
        // find_browser which tolerates concurrent set/remove (worst case a
        // degraded reason names a missing path).
        unsafe { std::env::set_var(ORZ_BROWSER_PATH_ENV, &missing) };
        let mut host =
            crate::OrzHost::new(journal.clone(), &dir, WorkspaceTrust::ObservedTrusted).unwrap();
        let cap = probe_retrieval_capability(
            RetrievalMode::LocalBrowser,
            true,
            &mut host,
            &dir,
            "RUN-TEST02",
        )
        .await;
        unsafe { std::env::remove_var(ORZ_BROWSER_PATH_ENV) };
        match cap {
            RetrievalCapability::Degraded(reason) => {
                assert!(reason.starts_with("browser_launch_failed: "), "{reason}");
                assert!(reason.contains("browser_not_found"), "{reason}");
            }
            other => panic!("expected Degraded, got {other:?}"),
        }

        // Off → Unsupported; framework_fallback without key → Degraded.
        let mut host = crate::OrzHost::new(journal, &dir, WorkspaceTrust::ObservedTrusted).unwrap();
        assert_eq!(
            probe_retrieval_capability(RetrievalMode::Off, true, &mut host, &dir, "RUN-TEST03")
                .await,
            RetrievalCapability::Unsupported("retrieval_mode_not_selected".to_string())
        );
        assert_eq!(
            probe_retrieval_capability(
                RetrievalMode::FrameworkFallback,
                false,
                &mut host,
                &dir,
                "RUN-TEST04"
            )
            .await,
            RetrievalCapability::Degraded("web_search_not_configured".to_string())
        );
        let _ = std::fs::remove_dir_all(&dir);
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
                // Full phase chain + §4.6: preflight + started +
                // prompt_submitted + tool_availability + model_output +
                // counterexample_gate + model_output + finished.
                // GAP-INQUIRY-SPLIT: no per-turn orientation event (fires
                // only on the session-level 7-round trigger).
                // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): plan 门
                // 普适摘除——不再有 plan 轮（原 18 → 9，与无门轮次一致）。
                assert_eq!(replay.event_count, 9);
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

                // Four scripted responses — two per prompt turn (draft +
                // final; the counterexample gate adds one round).
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("(fake) 第一轮。"),
                    ScriptedResponse::text("(fake) 第一轮终答。"),
                    ScriptedResponse::text("(fake) 第二轮。"),
                    ScriptedResponse::text("(fake) 第二轮终答。"),
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

                for dir in run_dirs.iter() {
                    let replay =
                        orz_assurance::replay_journal(&dir.join("events.jsonl"), None, None, true);
                    assert!(replay.valid, "run journal invalid: {:?}", replay.errors);
                    // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): plan
                    // 门普适摘除——两个 prompt 均为无门轮次（9 事件）。
                    assert_eq!(replay.event_count, 9, "preflight + turn events");
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

    /// ACAF production flip (2026-08-16): the ACP session path now carries
    /// the fail-closed posture — a server with no signer fabric refuses
    /// prompts with the D-15 startup error instead of running unticketed.
    #[tokio::test]
    async fn acaf_fail_closed_without_fabric_refuses_prompt() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("完成"),
                ])))
                .with_acaf_fail_closed(true);
                server
                    .handle_session_new(
                        "sess-fc",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let err = server
                    .handle_session_prompt("sess-fc", "任何任务")
                    .await
                    .expect_err("fail-closed + no fabric must refuse the prompt");
                assert!(
                    err.to_string().contains("fail-closed"),
                    "error must name fail-closed: {err}"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
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
                    // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): 无
                    // plan 门——首轮直接调 read_file（direct 面，不再经
                    // 订单层，2026-08-24 起）。
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({ "target_file": target }),
                        call_id: "call-read-1".to_string(),
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
                    // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24): direct
                    // 面——模型直接调 search_replace（写权限在直连调用时
                    // 到达 permission bridge，dead gateway → 拒绝）。
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "a.txt",
                            "old_string": "v1",
                            "new_string": "v2",
                        }),
                        call_id: "call-edit-1".to_string(),
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
                    !events.iter().any(|e| {
                        e.event_type == EventType::ToolStarted
                            && e.payload.get("tool").and_then(|t| t.as_str())
                                == Some("search_replace")
                    }),
                    "the denied order target must not start headless: {types:?}"
                );
                let pd = events
                    .iter()
                    .filter(|e| e.event_type == EventType::PermissionDecision)
                    .find(|e| {
                        e.payload.get("tool").and_then(|t| t.as_str()) == Some("search_replace")
                    })
                    .expect("permission decision");
                assert_eq!(
                    pd.payload.get("decision").and_then(|d| d.as_str()),
                    Some("deny")
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// Slice #16: the policy stored on `handle_session_new_with_policy`
    /// flows into the prompt path. An always-approving hub makes this
    /// decisive: under the default Interactive policy the bash executes;
    /// under ReadOnly it is denied BEFORE the hub sees it (no ToolStarted).
    #[tokio::test]
    async fn session_prompt_respects_session_policy() {
        /// Test-only transport that approves everything (the Interactive
        /// control's manager decision must be `Allow`).
        struct AlwaysAllowTransport;
        #[async_trait::async_trait]
        impl PermissionHookTransport for AlwaysAllowTransport {
            async fn request_permission(
                &self,
                _payload: serde_json::Value,
            ) -> Result<serde_json::Value, String> {
                Ok(serde_json::json!({ "outcome": "approve" }))
            }
        }

        async fn run_bash_prompt(server: &AcpServer, session: &str, base: &Path) -> Vec<EventType> {
            let result = server
                .handle_session_prompt(session, "执行命令")
                .await
                .unwrap();
            assert_eq!(result["status"], "completed");
            run_events(base)
                .iter()
                .map(|e| e.event_type.clone())
                .collect()
        }

        tokio::task::LocalSet::new()
            .run_until(async {
                let base_ro = test_dir();
                let base_ww = test_dir();
                // Two sequential prompts share the one FakeProvider — each
                // run pulls [直调工具, 草稿, 终答]。
                let script = vec![
                    // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24): direct
                    // 面——模型直接调 search_replace。
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "a.txt",
                            "old_string": "v1",
                            "new_string": "v2",
                        }),
                        call_id: "call-edit-ro".to_string(),
                    }]),
                    ScriptedResponse::text("完成。"),
                    ScriptedResponse::text("完成。"),
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "a.txt",
                            "old_string": "v1",
                            "new_string": "v2",
                        }),
                        call_id: "call-edit-ww".to_string(),
                    }]),
                    ScriptedResponse::text("完成。"),
                    ScriptedResponse::text("完成。"),
                ];
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(script)));
                server.set_gateway(dead_gateway());
                server.set_hub_permission(Arc::new(AlwaysAllowTransport));

                // ReadOnly session: the policy short-circuits the bash
                // BEFORE the hub — denied, never started.
                server
                    .handle_session_new_with_policy(
                        "sess-ro",
                        Some(base_ro.clone()),
                        crate::session::TrustPolicy::Skip,
                        PermissionPolicy::ReadOnly,
                    )
                    .await
                    .unwrap();
                let ro_types = run_bash_prompt(&server, "sess-ro", &base_ro).await;
                let ro_events = run_events(&base_ro);
                assert!(
                    !ro_events.iter().any(|e| {
                        e.event_type == EventType::ToolStarted
                            && e.payload.get("tool").and_then(|t| t.as_str())
                                == Some("search_replace")
                    }),
                    "read-only session must deny the mutation: {ro_types:?}"
                );

                // Interactive session (default): the always-allow hub means
                // the tool executes — proving the policy, not the hub,
                // decided the read-only case.
                server
                    .handle_session_new(
                        "sess-ww",
                        Some(base_ww.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                let ww_types = run_bash_prompt(&server, "sess-ww", &base_ww).await;
                let ww_events = run_events(&base_ww);
                assert!(
                    ww_events.iter().any(|e| {
                        e.event_type == EventType::ToolStarted
                            && e.payload.get("tool").and_then(|t| t.as_str())
                                == Some("search_replace")
                    }),
                    "interactive session with allow-all hub must execute: {ww_types:?}"
                );

                let _ = std::fs::remove_dir_all(&base_ro);
                let _ = std::fs::remove_dir_all(&base_ww);
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
                    // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24): direct
                    // 面——模型直接调 search_replace（dead gateway 下写权限
                    // 拒绝 → 无快照记录）。
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({
                            "file_path": "lib.rs",
                            "old_string": "fn main",
                            "new_string": "fn renamed",
                        }),
                        call_id: "call-edit-snap".to_string(),
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

                let events = run_events(&base);
                let types: Vec<EventType> = events.iter().map(|e| e.event_type.clone()).collect();
                assert!(types.contains(&EventType::PermissionDecision), "{types:?}");
                assert!(
                    !types.contains(&EventType::SnapshotCreated),
                    "denied mutation must not snapshot: {types:?}"
                );
                assert!(
                    !events.iter().any(|e| {
                        e.event_type == EventType::ToolStarted
                            && e.payload.get("tool").and_then(|t| t.as_str())
                                == Some("search_replace")
                    }),
                    "denied order target must not start: {types:?}"
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
                // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): 无 plan
                // 门——run #1 中断于第一段草稿的流式期间（消费 0–1 项）；
                // run #2 需要 [草稿, 终答]。四项文本覆盖消费 0–2 项。
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

    #[tokio::test]
    async fn grill_turn_records_jsonl_archives_and_rejects_while_in_flight() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Scripts: turn1 → question (gate skipped → exactly one
                // provider round), turn2 → question, finish → summary.
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("问题一: 是否考虑……?"),
                    ScriptedResponse::text("问题二: 方案取舍……?"),
                    ScriptedResponse::text("总结: 决策清单……"),
                ])));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-grill",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let r1 = server.run_grill_turn("sess-grill", "回答一").await.unwrap();
                assert_eq!(r1, "问题一: 是否考虑……?");
                let r2 = server.run_grill_turn("sess-grill", "回答二").await.unwrap();
                assert_eq!(r2, "问题二: 方案取舍……?");

                // Audit JSONL: one record per turn (Q/A), nothing else.
                let log = base.join(".gsa").join("grill").join("sess-gri.jsonl");
                let content = std::fs::read_to_string(&log).unwrap();
                let lines: Vec<&str> = content.lines().collect();
                assert_eq!(lines.len(), 2, "{content}");
                assert!(content.contains("\"turn\":0") || content.contains("\"turn\": 0"));
                assert!(content.contains("回答一"));
                assert!(content.contains("回答二"));

                // No run-journal events: grill bootstraps a GRILL-* run dir
                // (host shape) but the controller's discard writer never
                // writes events.jsonl.
                let runs_dir = base.join(".gsa").join("runs");
                let grill_dirs: Vec<String> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with("GRILL-"))
                    .collect();
                // Two turns → two GRILL-* bootstrap dirs. The host bootstrap
                // mechanically records run_preflight as event 0 (session
                // lifecycle); the controller's discard writer adds NOTHING —
                // no model/tool/gate/terminal events ever land here (the
                // audit trail is the grill JSONL). An open chain without a
                // terminal is expected: a grill turn is not a run.
                assert_eq!(grill_dirs.len(), 2, "{grill_dirs:?}");
                for dir in &grill_dirs {
                    let content =
                        std::fs::read_to_string(runs_dir.join(dir).join("events.jsonl")).unwrap();
                    let evts: Vec<orz_assurance::RunEvent> = content
                        .lines()
                        .map(|l| serde_json::from_str(l).unwrap())
                        .collect();
                    assert_eq!(
                        evts.len(),
                        1,
                        "GRILL journal must hold only run_preflight: {content}"
                    );
                    assert_eq!(evts[0].event_type, EventType::RunPreflight);
                }

                // A run in flight rejects a grill turn (sequencing guard).
                {
                    let mut runs = server.runs.lock().unwrap();
                    runs.insert(
                        "sess-grill".to_string(),
                        RunInFlight::Prompt(tokio_util::sync::CancellationToken::new()),
                    );
                }
                let err = server
                    .run_grill_turn("sess-grill", "回答三")
                    .await
                    .unwrap_err();
                assert!(matches!(err, AcpError::InvalidRequest(_)), "{err}");
                server.runs.lock().unwrap().remove("sess-grill");

                // finish: summary turn + terminal record + session cleared.
                let summary = server.finish_grill("sess-grill").await.unwrap();
                assert_eq!(summary, "总结: 决策清单……");
                let content2 = std::fs::read_to_string(&log).unwrap();
                assert!(
                    content2.contains("\"terminal\":\"finished\"")
                        || content2.contains("\"terminal\": \"finished\""),
                    "{content2}"
                );
                // Cleared → a second finish is an InvalidRequest.
                assert!(server.finish_grill("sess-grill").await.is_err());

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    #[tokio::test]
    async fn grill_turn_denies_write_tools_under_read_only() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "search_replace".to_string(),
                        arguments: serde_json::json!({"path": "x.py"}),
                        call_id: "call-write-1".to_string(),
                    }]),
                    ScriptedResponse::text("已检查只读边界，不写入。"),
                ])));
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-ro",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                let response = server.run_grill_turn("sess-ro", "背景").await.unwrap();
                assert_eq!(response, "已检查只读边界，不写入。");

                // The write tool was denied — no journal (discard) and no
                // file mutation; the response shows the model acknowledged.
                assert!(
                    !base.join("x.py").exists(),
                    "grill (ReadOnly) must never execute write tools"
                );

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    // ── GAP-CONVERSATION-RESTORE (2026-08-10): conversation sidecar ──

    fn conv_sidecar_path(base: &Path, session_id: &str) -> PathBuf {
        let suffix: String = session_id.chars().take(8).collect();
        base.join(".gsa")
            .join("conversations")
            .join(format!("{suffix}.json"))
    }

    #[test]
    fn conversation_sidecar_roundtrip() {
        let base = test_dir();
        let messages = vec![
            Message {
                role: Role::User,
                content: "中文问题".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
            Message {
                role: Role::Assistant,
                content: "回答".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: Some("推理过程".to_string()),
            },
            Message {
                role: Role::Tool,
                content: "工具结果".to_string(),
                tool_call_id: Some("call-1".to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
        ];
        persist_conversation_sidecar(&base, "sess-roundtrip", &messages);
        let stored = load_conversation_sidecar(&base, "sess-roundtrip").expect("sidecar loads");
        assert_eq!(stored.session_id, "sess-roundtrip");
        assert_eq!(stored.messages.len(), 3);
        assert_eq!(stored.messages[0].content, "中文问题");
        assert_eq!(
            stored.messages[1].reasoning_content.as_deref(),
            Some("推理过程")
        );
        assert_eq!(stored.messages[2].tool_call_id.as_deref(), Some("call-1"));
        // Exact path shape.
        assert!(conv_sidecar_path(&base, "sess-roundtrip").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn conversation_sidecar_corrupt_warns_and_none() {
        let base = test_dir();
        let path = conv_sidecar_path(&base, "sess-corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{not json").unwrap();
        assert!(load_conversation_sidecar(&base, "sess-corrupt").is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn conversation_sidecar_missing_returns_none() {
        let base = test_dir();
        assert!(load_conversation_sidecar(&base, "sess-missing").is_none());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_conversation_not_persisted() {
        let base = test_dir();
        persist_conversation_sidecar(&base, "sess-empty", &[]);
        assert!(!conv_sidecar_path(&base, "sess-empty").exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Core regression: two sequential prompts on one session — the second
    /// model request carries the first turn's history (conversation seeds
    /// across prompts); the sidecar lands; orientation/activation sidecars
    /// are unaffected.
    #[tokio::test]
    async fn cross_prompt_conversation_continues() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let fake = Arc::new(FakeProvider::new(vec![
                    // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): 无
                    // plan 门——每个 prompt 两轮（草稿 + 终答）。
                    ScriptedResponse::text("第一答"),
                    ScriptedResponse::text("第一答"),
                    ScriptedResponse::text("第二答"),
                    ScriptedResponse::text("第二答"),
                ]));
                let server = AcpServer::with_gateway(fake.clone());
                server
                    .handle_session_new(
                        "sess-conv",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                let r1 = server
                    .handle_session_prompt("sess-conv", "第一问")
                    .await
                    .unwrap();
                assert_eq!(r1["response"], "第一答");
                let r2 = server
                    .handle_session_prompt("sess-conv", "第二问")
                    .await
                    .unwrap();
                assert_eq!(r2["response"], "第二答");

                // The second prompt's first model request opened with the
                // first turn (two model calls per prompt — the
                // counterexample gate adds one round; the plan gate is
                // removed universally).
                let reqs = fake.received_requests();
                assert_eq!(reqs.len(), 4, "two model calls per prompt");
                let msgs = &reqs[2].messages;
                assert!(
                    msgs.iter().any(|m| m.content == "第一问"),
                    "first prompt in history: {msgs:?}"
                );
                assert!(
                    msgs.iter().any(|m| m.content == "第一答"),
                    "first reply in history: {msgs:?}"
                );
                assert!(msgs.iter().any(|m| m.content == "第二问"));

                // The sidecar landed (only after a successful run).
                let stored = load_conversation_sidecar(&base, "sess-conv").expect("sidecar");
                assert!(stored.messages.iter().any(|m| m.content == "第一问"));
                assert!(stored.messages.iter().any(|m| m.content == "第二答"));

                // Two run journals exist (one per prompt) — the conversation
                // path adds no extra runs.
                let runs_dir = base.join(".gsa").join("runs");
                let run_dirs: Vec<String> = std::fs::read_dir(&runs_dir)
                    .unwrap()
                    .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .filter(|n| n.starts_with("RUN-"))
                    .collect();
                assert_eq!(run_dirs.len(), 2, "{run_dirs:?}");

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// Process-restart recovery (Review P3-8): a new server re-creating the
    /// same `session_id` loads the conversation sidecar — the first prompt
    /// seeds from the previous process's history.
    #[tokio::test]
    async fn new_server_resumes_conversation_from_sidecar() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Process 1: one successful prompt lands the sidecar.
                let fake1 = Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("第一答"),
                    ScriptedResponse::text("第一答"),
                ]));
                let server1 = AcpServer::with_gateway(fake1.clone());
                server1
                    .handle_session_new(
                        "sess-restart",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server1
                    .handle_session_prompt("sess-restart", "重启前的问题")
                    .await
                    .unwrap();
                // (server1 dropped — process restart.)

                // Process 2: a NEW server re-creates the session.
                let fake2 = Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("第二答"),
                    ScriptedResponse::text("第二答"),
                ]));
                let server2 = AcpServer::with_gateway(fake2.clone());
                server2
                    .handle_session_new(
                        "sess-restart",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server2
                    .handle_session_prompt("sess-restart", "重启后的问题")
                    .await
                    .unwrap();

                // The new process's first request carried the old history.
                let reqs = fake2.received_requests();
                assert_eq!(reqs.len(), 2, "two model calls per prompt");
                let msgs = &reqs[0].messages;
                assert!(
                    msgs.iter().any(|m| m.content == "重启前的问题"),
                    "pre-restart prompt in history: {msgs:?}"
                );
                assert!(
                    msgs.iter().any(|m| m.content == "第一答"),
                    "pre-restart reply in history: {msgs:?}"
                );
                assert!(msgs.iter().any(|m| m.content == "重启后的问题"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// A failed prompt keeps the pre-run conversation — the sidecar is not
    /// overwritten by a failed run's partial messages.
    #[tokio::test]
    async fn failed_prompt_keeps_pre_run_conversation() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // One successful prompt consumes the scripted replies; the
                // second prompt hits an empty script → model failure.
                let server = AcpServer::with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("第一答"),
                    ScriptedResponse::text("第一答"),
                ])));
                server
                    .handle_session_new(
                        "sess-fail",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();

                server
                    .handle_session_prompt("sess-fail", "第一问")
                    .await
                    .unwrap();
                let err = server
                    .handle_session_prompt("sess-fail", "第二问")
                    .await
                    .unwrap_err();
                assert!(matches!(err, AcpError::AgentLoop(_)), "{err}");

                // The sidecar still holds only the FIRST run's history.
                let stored = load_conversation_sidecar(&base, "sess-fail").expect("sidecar");
                assert!(stored.messages.iter().any(|m| m.content == "第一问"));
                assert!(stored.messages.iter().any(|m| m.content == "第一答"));
                assert!(stored.messages.iter().all(|m| m.content != "第二问"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }

    /// The activation sidecar's conversation survives a cross-run round trip
    /// through the host: sidecar → session → controller seed → fold → persist.
    #[tokio::test]
    async fn restored_activation_conversation_across_runs() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                // Pre-seed the activation sidecar with a restored
                // AwaitingDisposition activation carrying its conversation.
                let activation = serde_json::json!({
                    "schema_version": "0.1.0-draft",
                    "session_id": "sess-act",
                    "retrieval_mode": "framework_fallback",
                    "bootstrap_transition_pending": false,
                    "next_seq": {"internal_retrieval": 1},
                    "activations": [{
                        "activation_id": "retrieval-internal_retrieval-sess-act-00",
                        "parent_session_id": "sess-act",
                        "subagent_session_id": "SUB-internal_retrieval-sess-act",
                        "contract_id": "retrieval-contract-internal_retrieval",
                        "contract_revision": 0,
                        "status": "awaiting_disposition",
                        "tool_rounds_used": 2,
                        "result_digest": "b".repeat(64),
                        "pending_assessment_id": "ASSESS-PREV-1",
                        "pending_expected_contract_revision": 0,
                        "origin_run_id": "RUN-PREV-0001",
                        "conversation": [
                            {"role": "user", "content": "跨 run 的历史上下文",
                             "tool_call_id": null, "tool_calls": [],
                             "reasoning_content": null},
                            {"role": "assistant", "content": "跨 run 的结论",
                             "tool_call_id": null, "tool_calls": [],
                             "reasoning_content": "跨 run 推理"},
                        ]
                    }]
                });
                let act_dir = base.join(".gsa").join("activations");
                std::fs::create_dir_all(&act_dir).unwrap();
                std::fs::write(
                    act_dir.join("sess-act.json"),
                    serde_json::to_string_pretty(&activation).unwrap(),
                )
                .unwrap();

                let fake = Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("收到"),
                    ScriptedResponse::text("收到"),
                ]));
                let server = AcpServer::with_gateway(fake.clone());
                server
                    .handle_session_new(
                        "sess-act",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server
                    .handle_session_prompt("sess-act", "继续检索")
                    .await
                    .unwrap();

                // The restored activation was journaled (restore event) and
                // the persisted sidecar still carries the conversation.
                let events = run_events(&base);
                assert!(
                    events
                        .iter()
                        .any(|e| e.event_type == EventType::RetrievalActivationRestored),
                    "restore event journaled"
                );
                let persisted = std::fs::read_to_string(act_dir.join("sess-act.json")).unwrap();
                let json: serde_json::Value = serde_json::from_str(&persisted).unwrap();
                let conv = &json["activations"][0]["conversation"];
                assert_eq!(conv[0]["content"], "跨 run 的历史上下文");
                assert_eq!(conv[1]["reasoning_content"], "跨 run 推理");
                // The restore event does not carry the conversation (journal
                // stays reasoning-free — zero-event-change discipline).
                let restore = events
                    .iter()
                    .find(|e| e.event_type == EventType::RetrievalActivationRestored)
                    .unwrap();
                assert!(restore.payload.get("conversation").is_none());

                let _ = std::fs::remove_dir_all(&base);
            })
            .await;
    }
}
