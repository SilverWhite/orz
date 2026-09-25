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

use orz_assurance::lif::{DomainSpike, TemporalSessionSnapshot};
use orz_assurance::{EventTrack, EventType, JournalRecorderError, Redaction, RunEvent};
use orz_loop::AgentLoopController;
use orz_loop::acaf::AcafClient;
use orz_loop::blackboard::{Blackboard, ExternalRetSection, InternalRetSection};
use orz_loop::controller::RetrievalMode;
use orz_loop::gateway::model::{Message, Role, ToolCall};
use orz_loop::orientation::OrientationSessionState;
use orz_workspace::permission::PermissionHookTransport;

use flate2::Compression;
use flate2::write::GzEncoder;
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

/// 会话轴原点墙钟（P2-13 B1）：conversation-relative `t` = wall −
/// session_started_at（跨 prompt 单调）。
fn now_epoch_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
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
            // 0p S2 复审 P2 修复（B5 第 5 漏斗，2026-09-07）：grill 会话
            // 记录落卷（`.gsa/grill/*.jsonl`）接 orz-secrets 机械脱敏。
            orz_secrets::redact_json_string_values(&mut record);
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
            let mut record = serde_json::json!({
                "terminal": "finished",
                "episode": episode,
                "summary": summary,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });
            // 0p S2 复审 P2 修复（B5 第 5 漏斗，2026-09-07）：同 grill 记录。
            orz_secrets::redact_json_string_values(&mut record);
            let _ = writeln!(f, "{}", record);
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
        let event = RunEvent::new_v02(
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
        // 0v-C（2026-09-12）：不在漏斗外预封印——记录器先做 payload 脱敏
        // （sk-shape / URL 归一化等确定性改写）再封印，并返回**落盘**哈希。
        // 链只能串这个哈希（调用方自算的是未改写形态，重放报
        // `previous hash mismatch`）。
        // Only advance the chain link after the write is accepted (a refused
        // append must not pollute the caller's bookkeeping — 2026-08-04
        // review P2-7 precedent).
        match self.journal.record_async(event).await {
            Ok(event_hash) => {
                self.prev_hash = Some(event_hash);
                self.seq += 1;
                Ok(())
            }
            Err(JournalRecorderError::DegradedDropped {
                sequence,
                event_type,
            }) => {
                // 0z S2 review F-C-4 (2026-09-13): a degraded-mode refusal is
                // the skeleton contract, not an integrity violation — the run
                // must still reach its enumerable terminal shape. For a
                // terminal event the writer either landed it (skeleton filter
                // admits terminal rows) or wrote the TERMINAL.json sidecar;
                // for any other event the row was intentionally not written.
                // Either way the caller's chain bookkeeping stays untouched
                // and the run proceeds (mirror of EventWriter::record in
                // orz-loop/controller.rs).
                tracing::warn!(
                    sequence,
                    event_type,
                    "RunRecorder: journal degraded mode dropped an event;                      chain bookkeeping untouched"
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
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
    /// GAP-RETRIEVAL-TOOLS (2026-08-10) + 0t (2026-09-09, ADR-0010 §14.65):
    /// the retrieval enable-gate + activation snapshot — persisted across
    /// prompts AND process restarts via
    /// `{cwd}/.gsa/activations/<session8>.json`. Taken out during a run,
    /// written back afterwards (orientation pattern). 三值检索模式已退役，
    /// 快照只承载独立启用门（retrieval_enabled）；per-role activation
    /// state rides the same file (S4).
    activation_snapshot: Option<StoredActivationSnapshot>,
    /// local_browser (2026-08-10): the session's browser lane handle —
    /// survives across runs (the process stays up on its isolated profile;
    /// re-injected into each run's host by the capability probe). Shut down
    /// (process tree kill + profile-dir best-effort delete) on
    /// `close_session`.
    browser: Option<crate::local_browser::SharedBrowser>,
    /// GAP-CONVERSATION-RESTORE (2026-08-10) + P2-13 B1 会话化基础
    /// (2026-09-03): the session continuation — conversation messages +
    /// 黑板 live 视图 + LIF 会话轴（round/域机器/会话起始墙钟）。跨
    /// prompt 与进程重启经 `{cwd}/.gsa/conversations/<session8>.json`
    /// 侧车持久化（in-session copy = authoritative；sidecar best-effort）。
    /// Taken out during a run, written back on success (orientation
    /// pattern——失败 run 不写回，下一 prompt 回退侧车 pre-run 内容)。
    continuation: Option<StoredConversation>,
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10) + 0t (2026-09-09, ADR-0010 §14.65):
/// the persisted retrieval enable-gate + activation snapshot. 三值检索模式
/// γ 退役后只持久化 `retrieval_enabled`（独立启用门）；旧侧车中的
/// `retrieval_mode` 字段经 [`legacy_retrieval_mode`] 兼容读取（非 off 旧值
/// ⇒ 启用），新写不再产出该字段。S4 (activation persistence) fills
/// `next_seq`/`activations`. Same sidecar discipline as the orientation
/// counter: best-effort persist, corrupt → warn, take-out only after every
/// fallible step.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredActivationSnapshot {
    schema_version: String,
    session_id: String,
    /// 0t: 独立检索启用门（默认 false = fail-closed）。旧侧车（无此字段）
    /// 由 [`load_activation_sidecar`] 按 legacy `retrieval_mode` 归一化。
    #[serde(default)]
    retrieval_enabled: bool,
    /// DEPRECATED 兼容字段：只读旧侧车的 `retrieval_mode` 旧值；新侧车
    /// 不写（skip_serializing_if）。不再有 lane 语义，仅用于启用门归一化。
    #[serde(
        default,
        rename = "retrieval_mode",
        skip_serializing_if = "Option::is_none"
    )]
    legacy_retrieval_mode: Option<RetrievalMode>,
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
            retrieval_enabled: false,
            legacy_retrieval_mode: None,
            next_seq: HashMap::new(),
            activations: Vec::new(),
            internal_ret: None,
            external_ret: None,
        }
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
        Ok(text) => {
            let raw: Option<serde_json::Value> = serde_json::from_str(&text).ok();
            let has_explicit_enable = raw
                .as_ref()
                .and_then(|v| v.get("retrieval_enabled"))
                .is_some();
            match serde_json::from_str::<StoredActivationSnapshot>(&text) {
                Ok(mut state) => {
                    // 0t (ADR-0010 §14.65): 旧侧车（无 retrieval_enabled 字段）
                    // 按 legacy retrieval_mode 归一化启用门——非 off 旧值 ⇒
                    // enabled（值不再区分车道）。
                    if !has_explicit_enable {
                        state.retrieval_enabled = state
                            .legacy_retrieval_mode
                            .as_ref()
                            .is_some_and(|m| m.enables_retrieval());
                    }
                    Some(state)
                }
                Err(e) => {
                    tracing::warn!(
                        "activation sidecar corrupt ({}): {e} — starting fresh",
                        path.display()
                    );
                    None
                }
            }
        }
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
/// 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：`.gsa` 侧车持久化的
/// 统一脱敏出口——序列化为 JSON 后对全部字符串值做 orz-secrets 机械脱敏
/// （sk-shape 等结构化形态 + 占位符替换，确定性），再落盘。key 不落卷是
/// 两段门放开的前提不变量；脱敏失败 = 不写盘（fail-closed，不落明文）。
fn scrubbed_json_pretty<T: serde::Serialize>(value: &T) -> Option<String> {
    let mut json = serde_json::to_value(value).ok()?;
    orz_secrets::redact_json_string_values(&mut json);
    serde_json::to_string_pretty(&json).ok()
}

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
    match scrubbed_json_pretty(state) {
        Some(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("activation sidecar write failed ({}): {e}", path.display());
            }
        }
        None => tracing::warn!("activation sidecar scrub/serialize failed — not written"),
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
    /// P2-10 F2 §3.5 (I4, ADR-0010 §14.47): domain-switch spikes archived
    /// with the session sidecar — optional (`serde(default)` keeps old
    /// sidecars parseable, zero migration), cleared with the 7-day
    /// retention sweep, and rebuilt into the LIF engine's domain machine
    /// on cross-prompt restore. Subagent lanes never persist this.
    /// B1（2026-09-03）：新写入同时落 `lif`（精确机器快照）；本字段保留
    /// 为旧读者/旧侧车兼容（内容 = lif.spikes 的冗余投影）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    temporal_spikes: Option<Vec<DomainSpike>>,
    /// B1（2026-09-03，R5 conversation-relative 轴）：会话起始墙钟秒
    /// （epoch seconds）——LIF `t` 轴与会话相对的原点（t = wall −
    /// session_started_at，跨 prompt 单调）。`None` = legacy 侧车无
    /// 原点（沿用 run 起点近似）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_started_at: Option<f64>,
    /// B1：LIF 会话轴快照（决策轮计数 + 域机器 + spike 时间线），下个
    /// prompt 精确续接轮号与域机器。`None` = 尚无决策轮/legacy。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    lif: Option<TemporalSessionSnapshot>,
    /// B1（P2-13 / 设计 §6）：黑板 live 视图随会话延续（每个成功 prompt
    /// 续载而非重建）。`None` = 尚无黑板内容（首 prompt 前/legacy）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blackboard: Option<Blackboard>,
    /// 0bf ③（2026-09-22，用户令「和疲劳度绑在一起做加权，一起算一个
    /// 总值」）：**会话级负担档位**——已投递键（`["50", ...]`）。0bf 起
    /// 单键取代 0be 的 `fatigue_tiers_notified`/`complexity_tiers_notified`
    /// 两键（同梯档位 50/70/90：总值＝水位＋繁杂度增益）；旧侧车无本键 =
    /// 空（legacy 过渡面：可能重发一次，如实记录于 0bf 报告）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    burden_tiers_notified: Vec<String>,
    /// 动态上下文滑块 S1 修订批（v7，2026-09-15，设计 §3.5.1，用户裁定
    /// DP-16）：**会话级提醒水位**——已 fire 的实际上下文刻度键
    /// （`["500k", "900k"]`）。与黑板同族（先例 `fatigue_tiers_notified`）：
    /// 每级每会话一次、跨 prompt 延续、新会话从零开始、恢复不重发。
    /// prompt 起始经 `with_context_scale_notified` 注入 loop，run 成功后由
    /// controller 回写（SUCCESS-ONLY，同疲劳档位语义）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    context_scale_notified: Vec<String>,
}

impl StoredConversation {
    /// 全新会话（首 prompt 前）或 legacy 侧车缺字段时的补全构造：
    /// 黑板/lif/时间线为空，会话轴原点在缺失时取调用方给的当前墙钟。
    fn empty(session_id: &str, session_started_at: Option<f64>) -> Self {
        StoredConversation {
            schema_version: "0.2.0-draft".to_string(),
            session_id: session_id.to_string(),
            messages: Vec::new(),
            temporal_spikes: None,
            session_started_at,
            lif: None,
            blackboard: None,
            burden_tiers_notified: Vec::new(),
            context_scale_notified: Vec::new(),
        }
    }

    /// run 成功后的完整续接包（对话 + LIF 快照 + 黑板 live 快照）。
    fn full(
        session_id: &str,
        messages: Vec<Message>,
        temporal: &TemporalSessionSnapshot,
        session_started_at: Option<f64>,
        blackboard: &Blackboard,
    ) -> Self {
        StoredConversation {
            schema_version: "0.2.0-draft".to_string(),
            session_id: session_id.to_string(),
            messages,
            // legacy 冗余投影（lif.spikes 同源），供旧读者与既有测试。
            temporal_spikes: if temporal.spikes.is_empty() {
                None
            } else {
                Some(temporal.spikes.clone())
            },
            session_started_at,
            lif: Some(temporal.clone()),
            blackboard: Some(blackboard.clone()),
            // 0bf ③：负担档位在 run 成功后由调用方回写（SUCCESS-ONLY）。
            burden_tiers_notified: Vec::new(),
            // v7（S1 修订批）：水位在 run 成功后由调用方按 controller 回写。
            context_scale_notified: Vec::new(),
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
    match scrubbed_json_pretty(state) {
        Some(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("orientation sidecar write failed ({}): {e}", path.display());
            }
        }
        None => tracing::warn!("orientation sidecar scrub/serialize failed — not written"),
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

/// Persist the session continuation envelope（对话 + 黑板 live 视图与 LIF
/// 会话轴）——best-effort：只读工作区绝不因侧车写失败中断 run，in-session
/// 副本为权威写入，侧车为回退（orientation/activation 同纪律）。空对话
/// 不落盘（全新会话在首个成功 prompt 前没有文件，`load` 的 NotFound →
/// `None` 自然覆盖）。
fn persist_conversation_sidecar(base_dir: &Path, continuation: &StoredConversation) {
    if continuation.messages.is_empty() {
        return;
    }
    let path = conversation_sidecar_path(base_dir, &continuation.session_id);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            "conversation sidecar dir create failed ({}): {e}",
            parent.display()
        );
        return;
    }
    match scrubbed_json_pretty(continuation) {
        Some(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!(
                    "conversation sidecar write failed ({}): {e}",
                    path.display()
                );
            }
        }
        None => tracing::warn!("conversation sidecar scrub/serialize failed — not written"),
    }
}

/// 会话存档产物（blocking 打包阶段的结果）。`None` = 无可归档内容（从未
/// 有成功 prompt 的会话）或源 sidecar 损坏（warn 后跳过，不产生事件——
/// B3 复审登记为不可恢复静默边界）。
struct PackagedSessionArchive {
    archive_id: String,
    /// 专用 ARC run id：`ARC-<session8>-<prompt_count>`。
    run_id: String,
    path: PathBuf,
    digest: String,
    status: String,
    attempts: u32,
    fatigue_pct: u64,
    /// 动态上下文滑块 S1（2026-09-15，设计 §3.5 DP-11④）：本次归档时该会话
    /// 的**全量会话 token 估算**（chars/2，与 orz-loop `estimate_messages_tokens`
    /// 同口径）——写成功后落 `.gsa/archives/<session8>.milestones.json`，作为
    /// 「上次归档读数」，使增量归档按里程碑幂等（不重复打包同一刻度）。
    conversation_tokens: u64,
}

/// 动态上下文滑块 S1（2026-09-15，设计 §3.5 / DP-11④）：增量归档里程碑
/// 步长——实际上下文每跨 500K，附加一次增量归档（会话关闭仍照旧打包一次）。
const ARCHIVE_INCREMENT_TOKENS: u64 = 500_000;

/// 归档票：会话关闭时若仍有 run 在进行，先把归档挂起，待该 run 收尾
/// （sidecar 更新落盘后）再补触发——保证存档包含会话最后一段内容
/// （B3 复审 P2-3）。
struct PendingArchiveTicket {
    session_id: String,
    base_dir: PathBuf,
    prompt_count: u64,
    trust_policy: crate::session::TrustPolicy,
    /// 动态上下文滑块 S1（设计 §3.5 DP-11④）：`true` = 里程碑增量归档
    /// （`session_archive` 事件带 `incremental: true`），`false` = 会话关闭
    /// 归档（原有口径）。
    incremental: bool,
}

/// P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §11.3/§12 R3）：会话关闭/
/// 归档 = 对话 + 黑板 live 视图合成**单一 gzip 包**
/// （`.gsa/archives/<session8>.json.gz`——沿用 conversations sidecar 的
/// 8 字符前缀约定，设计 §11.3 的 `<session>` 字面按此口径执行，B3 复审
/// 登记）——纯打包、零内容变换、sha256 digest；tmp+rename 原子写（替换
/// 既有归档时为 remove+rename、非严格原子，B3 复审登记）；原 sidecar 保留
/// 至 retention 清扫。成功后以专用 `ARC-` run journal 记录
/// `session_archive` v0.2 事件（每条会话存档一次）。best-effort：失败只
/// warn、绝不阻断会话关闭。同步文件 IO + gzip 放入 blocking 池执行，
/// 避免阻塞 async worker。
/// 归档主实现（`incremental` = 本次是否为里程碑增量归档）。
///
/// 动态上下文滑块 S1（2026-09-15，设计 §3.5）：包内改为**信封**结构——
/// `{"schema":"session-archive-package-v0.2","conversation":<sidecar 原文>,"archive_keys":{…}}`；
/// `conversation` 成员是把磁盘 sidecar 的**原始字节原样嵌为 JSON 成员值**
/// （不 parse→re-serialize，保证「纯打包零内容变换」不变量仍成立），
/// `archive_keys` 为三键互标段（机械派生，见 `build_archive_keys`）。
/// `explicit_runs`（0ak，GAP-INCREMENTAL-ARCHIVE-HEADLESS）：除按
/// `RUN-{session8}-` 前缀扫描外，显式纳入的 run id——无头 run id
/// （`RUN-CLI-{ts}`）不携带会话段，前缀扫描不中，由调用方注入本次 run；
/// ACP 车道传 `&[]`（行为零变化）。
async fn archive_session_package(
    base_dir: &Path,
    session_id: &str,
    prompt_count: u64,
    trust_policy: crate::session::TrustPolicy,
    incremental: bool,
    explicit_runs: &[String],
) -> Option<PackagedSessionArchive> {
    // 侧车缺失 ⇒ None（无可归档内容，与既有语义一致）；重构路径走
    // `archive_raw_session_package`（0br S3 按需归档）。
    let raw = std::fs::read(conversation_sidecar_path(base_dir, session_id)).ok()?;
    archive_raw_session_package(
        base_dir,
        session_id,
        raw,
        prompt_count,
        trust_policy,
        incremental,
        explicit_runs,
    )
    .await
}

/// 打包任意「原始 sidecar 字节」的共享包装：同步打包 → 里程碑水位 →
/// ARC journal 事件。`archive_session_package`（磁盘侧车）与按需归档的
/// journal 重构路径共用，禁两套实现。
async fn archive_raw_session_package(
    base_dir: &Path,
    session_id: &str,
    raw: Vec<u8>,
    prompt_count: u64,
    trust_policy: crate::session::TrustPolicy,
    incremental: bool,
    explicit_runs: &[String],
) -> Option<PackagedSessionArchive> {
    let base_dir = base_dir.to_path_buf();
    let session_id = session_id.to_string();
    let package_base_dir = base_dir.clone();
    let package_session_id = session_id.clone();
    let explicit = explicit_runs.to_vec();
    let packaged = tokio::task::spawn_blocking(move || {
        package_archive_raw(
            &package_base_dir,
            &package_session_id,
            &raw,
            prompt_count,
            &explicit,
        )
    })
    .await;
    let Some(pkg) = packaged.ok().flatten() else {
        // 任务 panic 或无可归档内容/源损坏：无成品可记事件。
        return None;
    };

    // `session_archive` v0.2 事件 → 专用 ARC run journal（run_preflight 由
    // bootstrap 写入）——每条会话存档一次；journal 失败只 warn（包已落盘，
    // 存档动作本身不因审计面失败回滚）。
    let mut payload = serde_json::json!({
        "archive_id": pkg.archive_id,
        "path": pkg.path.display().to_string(),
        "digest": pkg.digest,
        "status": pkg.status.clone(),
        "attempts": pkg.attempts,
    });
    if pkg.fatigue_pct > 0 {
        payload["fatigue_pct"] = serde_json::Value::from(pkg.fatigue_pct);
    }
    if incremental {
        payload["incremental"] = serde_json::Value::Bool(true);
    }
    // 动态上下文滑块 S1（设计 §3.5 DP-11④）：包落盘成功即更新「上次归档
    // 读数」（单调水位）——增量归档的幂等锚点；失败不动水位（下次 run 尾
    // 重判）。best-effort：写失败只 warn。
    if pkg.status == "completed" {
        record_archived_tokens(&base_dir, &session_id, pkg.conversation_tokens);
    }
    let run_id = pkg.run_id.clone();
    match bootstrap_session(&run_id, Some(base_dir), trust_policy).await {
        Ok(handle) => {
            let mut recorder = RunRecorder::new(
                &handle.journal,
                &handle.run_id,
                &handle.run_manifest_sha256,
                handle.next_sequence,
                handle.last_event_sha256.clone(),
            );
            let mut recorded = recorder.record(EventType::SessionArchive, payload).await;
            if recorded.is_ok() {
                recorded = recorder
                    .record(
                        EventType::RunFinished,
                        serde_json::json!({"status": pkg.status}),
                    )
                    .await;
            }
            let _ = handle.journal.shutdown_async().await;
            if let Err(e) = recorded {
                tracing::warn!("session archive journal failed ({run_id}): {e}");
            }
        }
        Err(e) => {
            tracing::warn!("session archive journal bootstrap failed ({run_id}): {e}");
        }
    }
    Some(pkg)
}

/// 0br S3（2026-09-25 用户令：Web 工作台「归档活跃会话」动作）：对既有会话
/// 侧车执行一次即时归档。与关闭/里程碑归档共用同一原语
/// （`archive_session_package`：gzip 信封＋三键段＋里程碑水位＋ARC
/// journal 事件），禁第二套实现。`session8` 沿会话 id 前 8 字符口径
/// （sidecar／包名同尺，见 `conversation_sidecar_path`）。无侧车 ⇒
/// Err（无可归档内容）；侧车损坏/写盘失败 ⇒ Err（桥如实回传，不静默）。
/// 成功消息＝归档路径＋digest 前缀＋会话 token 读数。
pub async fn archive_session_on_demand(base_dir: &Path, session8: &str) -> Result<String, String> {
    // 批三（2026-09-25 用户令「归档当前全部会话」）：侧车缺失时回退
    // journal 重构——无头 `-p` 会话按 §14.68 未跨里程碑不落侧车，但其
    // 对话事实完整在 run journal（prompt_submitted／model_output）；机械
    // 重构为对话包并注入 reconstructed_from_journal 标记（如实标注，
    // 非零变换侧车拷贝）。黑板/LIF 该类会话本就没有，零损失。
    let (raw, reconstructed_runs) = match std::fs::read(conversation_sidecar_path(
        base_dir, session8,
    )) {
        Ok(bytes) => (bytes, None),
        Err(_) => {
            let (conv, runs) = reconstruct_conversation_from_journal(base_dir, session8)
                    .ok_or_else(|| {
                        format!(
                            "会话 {session8} 无侧车且 journal 无可重构对话事实（无 prompt/输出）——无可归档内容"
                        )
                    })?;
            let mut conv_value =
                serde_json::to_value(&conv).map_err(|e| format!("重构序列化失败: {e}"))?;
            // 如实标注：本包对话源自 journal 重构，非零变换侧车拷贝。
            conv_value["reconstructed_from_journal"] = serde_json::Value::Bool(true);
            let raw = serde_json::to_string(&conv_value)
                .map_err(|e| format!("重构序列化失败: {e}"))?
                .into_bytes();
            (raw, Some(runs))
        }
    };
    // 重构路径把纳入的 run 显式传给三键段（无头 run id `RUN-CLI-{s8}` 不
    // 携带 `RUN-{s8}-` 会话段，前缀扫描不中——与 0ak 显式注入同口径）。
    let explicit: Vec<String> = reconstructed_runs.clone().unwrap_or_default();
    let Some(pkg) = archive_raw_session_package(
        base_dir,
        session8,
        raw,
        1,
        crate::session::TrustPolicy::Enforce,
        false,
        &explicit,
    )
    .await
    else {
        return Err(format!("会话 {session8} 归档失败：内容无法解析"));
    };
    if pkg.status != "completed" {
        return Err(format!(
            "会话 {session8} 归档写盘失败（{attempts} 次尝试）",
            attempts = pkg.attempts
        ));
    }
    let recon_note = match reconstructed_runs {
        Some(runs) => format!("；journal 重构（{} 次运行）", runs.len()),
        None => String::new(),
    };
    Ok(format!(
        "{}（sha256:{}…，tokens={}{}）",
        pkg.path.display(),
        &pkg.digest[..pkg.digest.len().min(16)],
        pkg.conversation_tokens,
        recon_note
    ))
}

/// 从 run journal 机械重构会话对话（0br S3 批三，仅按需归档的无侧车
/// 回退路径）：`prompt_submitted` → 用户消息、`model_output`（有正文或
/// 工具调用）→ 助手消息。run 面＝`RUN-{s8}-*` 前缀 ∪ `RUN-CLI-{s8}` 精确
/// （与三键段 journal 键同一口径），按 run id 排序、run 内 sequence 序。
/// 零对话事实 ⇒ `None`。产出即「重构事实」，非零变换侧车拷贝——调用方
/// 在包内保留 `reconstructed_from_journal` 标记。
fn reconstruct_conversation_from_journal(
    base_dir: &Path,
    session8: &str,
) -> Option<(StoredConversation, Vec<String>)> {
    let runs_dir = base_dir.join(".gsa").join("runs");
    let mut ids: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&runs_dir) {
        ids.extend(
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| {
                    name.starts_with(&format!("RUN-{session8}-"))
                        || name == &format!("RUN-CLI-{session8}")
                }),
        );
    }
    ids.sort();
    if ids.is_empty() {
        return None;
    }

    let mut messages: Vec<Message> = Vec::new();
    let mut assistant_rounds = 0u64;
    let mut any_success = false;
    let mut started_at: Option<f64> = None;
    for run_id in &ids {
        let Ok(text) = std::fs::read_to_string(runs_dir.join(run_id).join("events.jsonl")) else {
            continue;
        };
        for line in text.lines() {
            let Ok(ev) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if started_at.is_none() {
                started_at = ev
                    .get("timestamp")
                    .and_then(serde_json::Value::as_str)
                    .and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok())
                    .map(|t| t.timestamp() as f64);
            }
            match ev.get("event_type").and_then(serde_json::Value::as_str) {
                Some("run_finished") => any_success = true,
                Some("prompt_submitted") => {
                    let prompt = ev
                        .get("payload")
                        .and_then(|p| p.get("prompt"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    if !prompt.trim().is_empty() {
                        messages.push(Message {
                            role: Role::User,
                            content: prompt.to_string(),
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                            round: None,
                        });
                    }
                }
                Some("model_output") => {
                    let payload = ev.get("payload");
                    let content = payload
                        .and_then(|p| p.get("text"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let tool_calls: Vec<ToolCall> = payload
                        .and_then(|p| p.get("tool_calls"))
                        .and_then(serde_json::Value::as_array)
                        .map(|calls| {
                            calls
                                .iter()
                                .filter_map(|c| {
                                    Some(ToolCall {
                                        name: c.get("name")?.as_str()?.to_string(),
                                        arguments: c.get("arguments").cloned().unwrap_or_default(),
                                        call_id: c
                                            .get("call_id")
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or_default()
                                            .to_string(),
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if !content.trim().is_empty() || !tool_calls.is_empty() {
                        assistant_rounds += 1;
                        messages.push(Message {
                            role: Role::Assistant,
                            content,
                            tool_call_id: None,
                            tool_calls,
                            reasoning_content: None,
                            round: None,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    if messages.is_empty() {
        return None;
    }
    let snapshot = TemporalSessionSnapshot {
        round: assistant_rounds,
        has_success: any_success,
        current_domain: orz_assurance::lif::Domain::Normal,
        entry_round: 0,
        spikes: Vec::new(),
        rli_shadow: None,
    };
    let mut conversation = StoredConversation::full(
        session8,
        messages,
        &snapshot,
        started_at,
        &Blackboard::default(),
    );
    conversation.blackboard = None; // 重构面无黑板事实，不落默认空黑板
    Some((conversation, ids))
}

/// 同步打包阶段：校验原始 sidecar 字节 → gzip → digest（tmp + rename）。
/// 0br S3 拆分：读文件由调用方负责（磁盘侧车或 journal 重构字节同一入口）。
fn package_archive_raw(
    base_dir: &Path,
    session_id: &str,
    raw_sidecar: &[u8],
    prompt_count: u64,
    explicit_runs: &[String],
) -> Option<PackagedSessionArchive> {
    // 纯打包 = 对既有内容原样压缩；空内容 → 无可存档。
    let parsed = match serde_json::from_slice::<StoredConversation>(raw_sidecar) {
        Ok(parsed) => parsed,
        Err(_) => {
            tracing::warn!(
                "session archive skipped: corrupt sidecar {}",
                conversation_sidecar_path(base_dir, session_id).display()
            );
            return None;
        }
    };
    let suffix: String = session_id.chars().take(8).collect();
    let archive_dir = base_dir.join(".gsa").join("archives");
    let final_path = archive_dir.join(format!("{suffix}.json.gz"));
    let archive_id = format!(
        "{session_id}-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    );
    // 动态上下文滑块 S1（2026-09-15，设计 §3.5）：包内容改为信封——
    // `conversation` 成员＝磁盘 sidecar 的原始字节（零内容变换），
    // `archive_keys`＝三键互标段（LIF 会话轮跨度／台账 `[seq]` 跨度／
    // journal run+sequence，外加窗口轮跨度临时键与 A 类压缩存档清单）。
    let archive_keys = build_archive_keys(base_dir, session_id, &parsed, explicit_runs);
    let conversation_tokens = estimate_conversation_tokens(&parsed);
    let Some(package_bytes) = build_archive_envelope(raw_sidecar, &archive_keys) else {
        tracing::warn!(
            "session archive skipped: sidecar is not valid UTF-8 ({})",
            conversation_sidecar_path(base_dir, session_id).display()
        );
        return None;
    };
    // 终点疲劳水位（百分比，0–100）：黑板 live 紧凑 JSON 字节 / W。
    let fatigue_pct = parsed
        .blackboard
        .as_ref()
        .map(|bb| {
            let bytes = serde_json::to_vec(bb).map(|v| v.len()).unwrap_or(0);
            orz_loop::fatigue::fatigue_percent(bytes, orz_loop::fatigue::live_budget_bytes())
        })
        .unwrap_or(0);

    let mut attempts = 0u32;
    let mut status = "failed";
    let mut digest = String::new();
    const MAX_ATTEMPTS: u32 = 3;
    while attempts < MAX_ATTEMPTS {
        attempts += 1;
        if std::fs::create_dir_all(&archive_dir).is_err() {
            continue;
        }
        let tmp_path = archive_dir.join(format!(".{suffix}.{archive_id}.tmp"));
        let write_ok = (|| -> std::io::Result<()> {
            let file = std::fs::File::create(&tmp_path)?;
            let mut enc = GzEncoder::new(file, Compression::default());
            enc.write_all(&package_bytes)?;
            enc.finish()?;
            Ok(())
        })();
        if write_ok.is_err() {
            let _ = std::fs::remove_file(&tmp_path);
            continue;
        }
        let Ok(bytes) = std::fs::read(&tmp_path) else {
            let _ = std::fs::remove_file(&tmp_path);
            continue;
        };
        digest = orz_assurance::journal::sha256_hex(&bytes);
        // 替换既有归档：Windows rename 不能覆盖目标，先 remove 再 rename
        // （非严格原子——B3 复审登记；首次写入为原子 tmp+rename）。
        if final_path.exists() {
            let _ = std::fs::remove_file(&final_path);
        }
        match std::fs::rename(&tmp_path, &final_path) {
            Ok(_) => {
                status = "completed";
                break;
            }
            Err(_) => {
                let _ = std::fs::remove_file(&tmp_path);
            }
        }
    }
    if status == "failed" {
        tracing::warn!(
            "session archive failed after {attempts} attempts: {}",
            final_path.display()
        );
    }
    Some(PackagedSessionArchive {
        archive_id,
        run_id: format!("ARC-{suffix}-{prompt_count}"),
        path: final_path,
        digest,
        status: status.to_string(),
        attempts,
        fatigue_pct,
        conversation_tokens,
    })
}

/// 动态上下文滑块 S1（2026-09-15，设计 §3.5）：归档包信封装配——
/// `conversation` 成员直接嵌入 sidecar 的**原始 JSON 文本**（不 parse→
/// re-serialize ⇒ 纯打包零内容变换不变量保持；旧读者仍可由 `decode_archive_package`
/// 兼容读取）。`raw_sidecar` 非 UTF-8 ⇒ `None`（调用方 warn 后跳过，与
/// 损坏 sidecar 同语义）。`archive_keys` 序列化失败同样回落 `None`（结构由
/// 本模块机械构造，正常不可达；fail-closed 优于写半个包）。
fn build_archive_envelope(raw_sidecar: &[u8], archive_keys: &serde_json::Value) -> Option<Vec<u8>> {
    let conversation = std::str::from_utf8(raw_sidecar).ok()?;
    let keys = serde_json::to_string(archive_keys).ok()?;
    Some(format!(
        "{{\"schema\":\"{ARCHIVE_PACKAGE_SCHEMA}\",\"conversation\":{conversation},\"archive_keys\":{keys}}}"
    )
    .into_bytes())
}

/// 归档包信封的 schema 标识（S1 起；`decode_archive_package` 用它区分
/// 新信封与旧裸包）。
const ARCHIVE_PACKAGE_SCHEMA: &str = "session-archive-package-v0.2";

/// 动态上下文滑块 S1（2026-09-15，设计 §3.5）：归档包 tolerant 解码——
/// 新信封（`conversation` ＋ `archive_keys`）与**旧裸包**（gzip 内容直接是
/// `StoredConversation`）都接受，返回 (对话, 可选的 archive_keys)。
/// 回读路径与测试共用同一入口，避免两处各自解析漂移。
/// 当前生产侧无包回读点（会话续接走 conversation sidecar，journal 走逐事件
/// 链）——本入口先服务归档一致性测试与将来的离线回读/核验工具，故显式
/// `allow(dead_code)` 而非删除（删掉就得让每个读者各自实现信封判别）。
#[allow(dead_code)]
fn decode_archive_package(bytes: &[u8]) -> Option<(StoredConversation, Option<serde_json::Value>)> {
    if let Ok(stored) = serde_json::from_slice::<StoredConversation>(bytes) {
        return Some((stored, None));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let conversation = value.get("conversation")?;
    let stored: StoredConversation = serde_json::from_value(conversation.clone()).ok()?;
    Some((stored, value.get("archive_keys").cloned()))
}

/// 动态上下文滑块 S1（设计 §3.5 判据「存档一致性（三键）」）：归档包
/// `archive_keys` 段——**机械派生、零模型调用、best-effort**（数据源缺失给
/// 确定结构 `exists=false`/空数组，绝不 panic，也绝不整段缺失）。
///
/// 三键口径（v6 更正后的键名，尾批 §7 落地前给会话级退化）：
/// 1. **会话相对 LIF 轮**：S1 给会话级跨度（`entry_round`→`round`，
///    来自 sidecar 的 `lif` 快照）＋ `window_rounds` 临时键（窗口内声明轮
///    跨度，`axis="window"`，尾批换成逐段 LIF 轮区间）；
/// 2. **台账 `[seq]`**：`.gsa/ledger/current.md` 的全局行序号跨度与行数
///    （行格式见 `orz-loop/action_ledger.rs::external_row_line`：`[<seq>] …`，
///    一条逻辑记录＝一个物理行）；
/// 3. **journal run+sequence**：本会话 run 目录（`.gsa/runs/RUN-<session8>-*`，
///    run id 口径见本文件 `format!("RUN-{suffix}-{prompt_number}")`）的
///    事件数与该文件首/末 `sequence`（首末两行各解析一次，避免整刊反序列化）。
///
/// 另附 A 类（机械压缩部分）清单：`.gsa/compaction/compaction-*.md` 的
/// 路径与字节数（B 类以 journal 为准，包内只留 run id/路径指针，避免整包膨胀）。
fn build_archive_keys(
    base_dir: &Path,
    session_id: &str,
    parsed: &StoredConversation,
    explicit_runs: &[String],
) -> serde_json::Value {
    let suffix: String = session_id.chars().take(8).collect();
    let lif = parsed.lif.as_ref();
    let tool_rounds = parsed
        .messages
        .iter()
        .filter(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .count() as u64;
    let ledger = ledger_key_facts(&base_dir.join(".gsa").join("ledger").join("current.md"));
    let journal = journal_key_facts(&base_dir.join(".gsa").join("runs"), &suffix, explicit_runs);
    let compaction = compaction_archive_facts(&base_dir.join(".gsa").join("compaction"));
    serde_json::json!({
        "axis_note": "LIF 逐段映射随尾批（CONTEXT_DYNAMIC_SLIDER_DESIGN §7）落地；\
                      S1 提供会话级 LIF 轮跨度（entry_round→round）与窗口轮跨度\
                      （window_rounds.axis=\"window\"，临时代用键，非全局键）。",
        "lif": {
            "round_start": lif.map(|l| l.entry_round),
            "round_end": lif.map(|l| l.round),
            "domain": lif.map(|l| l.current_domain.as_str()),
            "session_started_at": parsed.session_started_at,
        },
        "window_rounds": {
            "axis": "window",
            "provisional": true,
            "start": 1,
            "end": tool_rounds,
        },
        "ledger": ledger,
        "journal": journal,
        "compaction": compaction,
    })
}

/// 台账文件的 `[seq]` 跨度（不存在 ⇒ `exists=false` 且其余为 null/0）。
fn ledger_key_facts(path: &Path) -> serde_json::Value {
    let mut rows = 0u64;
    let mut first_seq: Option<u64> = None;
    let mut last_seq: Option<u64> = None;
    if let Ok(text) = std::fs::read_to_string(path) {
        for line in text.lines() {
            let seq = line
                .trim_start()
                .strip_prefix('[')
                .and_then(|rest| rest.split(']').next())
                .and_then(|s| s.trim().parse::<u64>().ok());
            if let Some(seq) = seq {
                rows += 1;
                first_seq.get_or_insert(seq);
                last_seq = Some(seq);
            }
        }
        return serde_json::json!({
            "path": ".gsa/ledger/current.md",
            "exists": true,
            "first_seq": first_seq,
            "last_seq": last_seq,
            "rows": rows,
        });
    }
    serde_json::json!({
        "path": ".gsa/ledger/current.md",
        "exists": false,
        "first_seq": serde_json::Value::Null,
        "last_seq": serde_json::Value::Null,
        "rows": 0,
    })
}

/// run journal 键事实：按 `RUN-{suffix}-*` 前缀扫描 run 目录，外加
/// `explicit_runs` 显式注入（0ak——无头 run id `RUN-CLI-{ts}` 不携带会话段，
/// 前缀扫描不中，由收尾归档注入本次 run；重复注入去重，合并后排序保证
/// 键序确定）。逐 run 读取 `events.jsonl` 的首末 sequence 与事件数。
fn journal_key_facts(runs_dir: &Path, suffix: &str, explicit_runs: &[String]) -> serde_json::Value {
    let mut ids: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(runs_dir) {
        ids.extend(
            entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().into_string().ok())
                .filter(|name| name.starts_with(&format!("RUN-{suffix}-"))),
        );
    }
    for run_id in explicit_runs {
        if !ids.contains(run_id) {
            ids.push(run_id.clone());
        }
    }
    ids.sort();
    let mut runs: Vec<serde_json::Value> = Vec::new();
    for run_id in ids {
        let events_path = runs_dir.join(&run_id).join("events.jsonl");
        let Ok(text) = std::fs::read_to_string(&events_path) else {
            continue;
        };
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let seq_of = |line: &str| -> Option<u64> {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()?
                .get("sequence")?
                .as_u64()
        };
        let first_sequence = lines.first().and_then(|l| seq_of(l)).unwrap_or(0);
        let last_sequence = lines
            .last()
            .and_then(|l| seq_of(l))
            .unwrap_or(first_sequence);
        runs.push(serde_json::json!({
            "run_id": run_id,
            "first_sequence": first_sequence,
            "last_sequence": last_sequence,
            "events": lines.len() as u64,
        }));
    }
    serde_json::json!({ "runs": runs })
}

/// A 类压缩摘要存档清单（`.gsa/compaction/compaction-*.md`）——只列路径与
/// 字节数，内容留在原文件（包内不复制，避免整包膨胀）。
fn compaction_archive_facts(dir: &Path) -> serde_json::Value {
    let mut items: Vec<serde_json::Value> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.starts_with("compaction-") && n.ends_with(".md"))
            })
            .collect();
        files.sort();
        for path in files {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            items.push(serde_json::json!({
                "id": id,
                "path": format!(".gsa/compaction/{}", path.file_name().and_then(|n| n.to_str()).unwrap_or_default()),
                "bytes": bytes,
            }));
        }
    }
    serde_json::Value::Array(items)
}

/// 全量会话 token 估算——与 orz-loop `compact::estimate_messages_tokens`
/// **同口径**（每消息 content ＋ reasoning ＋ tool_calls 名/参数 字符数 ÷2
/// 求和；该函数为 `pub(crate)`，跨 crate 无法复用，故此处镜像并在此注明
/// 单一来源）。用于增量归档里程碑（实际上下文刻度，非视图刻度）。
///
/// 审查修正批（2026-09-15，审查 P2④）：**「同尺」指口径同源，不等于读数等值**
/// ——本函数量的是**写回 sidecar 的会话**（机械注入块已被 `is_injected_block_text`
/// 过滤剔除），而 loop 侧 `estimate_messages_tokens(messages)` 量的是**含注入块的
/// 运行期会话**（`[CONTEXT_SCALE …]`／窗口提示／orientation／预算块…）⇒ 同一时刻
/// 本读数系统性偏低，差值＝会话内注入块累积量（长单对话下不可忽略）。里程碑只需
/// 单调＋幂等，故不要求与提醒读数逐 token 对齐（设计 §3.5.1「同尺」按此口径读）。
fn estimate_conversation_tokens(conversation: &StoredConversation) -> u64 {
    conversation
        .messages
        .iter()
        .map(|m| {
            let mut chars = m.content.chars().count() as u64;
            if let Some(reasoning) = &m.reasoning_content {
                chars += reasoning.chars().count() as u64;
            }
            for call in &m.tool_calls {
                chars += call.name.chars().count() as u64;
                chars += serde_json::to_string(&call.arguments)
                    .map(|s| s.chars().count() as u64)
                    .unwrap_or(0);
            }
            chars / 2
        })
        .sum()
}

/// 「上次归档读数」文件（会话级；单调水位）。
fn archive_milestone_path(base_dir: &Path, session_id: &str) -> PathBuf {
    let suffix: String = session_id.chars().take(8).collect();
    base_dir
        .join(".gsa")
        .join("archives")
        .join(format!("{suffix}.milestones.json"))
}

/// 读「上次归档读数」（缺失/损坏 ⇒ None ⇒ 视为尚无归档）。
fn last_archived_tokens(base_dir: &Path, session_id: &str) -> Option<u64> {
    let raw = std::fs::read_to_string(archive_milestone_path(base_dir, session_id)).ok()?;
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()?
        .get("archived_tokens")?
        .as_u64()
}

/// 写「上次归档读数」（best-effort：失败只 warn，绝不影响归档结论）。
fn record_archived_tokens(base_dir: &Path, session_id: &str, tokens: u64) {
    let path = archive_milestone_path(base_dir, session_id);
    if let Some(parent) = path.parent()
        && let Err(e) = std::fs::create_dir_all(parent)
    {
        tracing::warn!(
            "archive milestone dir create failed ({}): {e}",
            parent.display()
        );
        return;
    }
    let payload = serde_json::json!({
        "archived_tokens": tokens,
        "archived_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    });
    if let Err(e) = std::fs::write(&path, payload.to_string()) {
        tracing::warn!("archive milestone write failed ({}): {e}", path.display());
    }
}

/// 动态上下文滑块 S1（设计 §3.5 DP-11④）：增量归档**是否到期**——
/// 实际上下文估算 ≥500K 且（尚无归档 或 ≥ 上次归档读数 ＋500K）。纯机械、
/// 单调（水位只在包写成功后推进）⇒ 同一里程碑不会重复打包。
fn incremental_archive_due(
    base_dir: &Path,
    session_id: &str,
    conversation: &StoredConversation,
) -> bool {
    let tokens = estimate_conversation_tokens(conversation);
    if tokens < ARCHIVE_INCREMENT_TOKENS {
        return false;
    }
    match last_archived_tokens(base_dir, session_id) {
        Some(previous) => tokens >= previous.saturating_add(ARCHIVE_INCREMENT_TOKENS),
        None => true,
    }
}

/// 无头一次性 run 收尾归档的结果（0ak）。`archived = false` 时其余字段
/// 均为初始值——未跨里程碑 ⇒ 零产物（不落侧车、不打包、无 ARC journal）。
#[derive(Debug, Clone)]
pub struct HeadlessRunArchive {
    pub session_id: String,
    /// 对话侧车是否落盘。无头口径：仅在归档到期时落（侧车的唯一消费者是
    /// 归档打包；未到期不留 `.gsa/conversations/` 产物——与 ACP 车道
    /// 「每个成功 prompt 都落侧车」的差异属设计边界，ADR-0010 §14.68）。
    pub sidecar_written: bool,
    /// 里程碑增量归档包是否写盘成功（含 ARC `session_archive` 事件）。
    pub archived: bool,
    pub archive_path: Option<PathBuf>,
    /// 归档发生时的全量会话 token 估算（chars/2，与增量归档判据同尺）。
    pub conversation_tokens: Option<u64>,
}

/// 0ak（2026-09-16 用户裁决采 B，`GAP-INCREMENTAL-ARCHIVE-HEADLESS`）：无头
/// `-p` 一次性 run 的会话持久化与里程碑增量归档。一次性 run 现在携带会话
/// 对话（调用方以空 `Vec` 起点随 run 线程携带、成功后取回传入），本函数按
/// `StoredConversation` 既有形态装配侧车，并在跨 500K 里程碑时打包
/// `.gsa/archives/<session8>.json.gz` 三键包——判定（`incremental_archive_due`）、
/// 打包（`package_session_archive`）与 ARC 审计 journal 全部复用 ACP 车道
/// 同一套原语，禁第二套实现。
///
/// 与 ACP 车道的口径差异（设计边界，须与 ADR-0010 §14.68 保持一致）：
/// ① 一次性 run 无 close 语义 ⇒ 只做里程碑增量归档，会话关闭归档不适用；
/// ② 侧车仅在归档到期时落盘（消费者只有打包）；③ 跨调用对话恢复不开启
/// ——每次 `-p` 生成全新会话身份（GAP-CONVERSATION-RESTORE 边界不变）。
/// best-effort：源损坏/打包失败只 warn，绝不影响 run 结局。
#[allow(clippy::too_many_arguments)]
pub async fn headless_session_archive(
    base_dir: &Path,
    session_id: &str,
    run_id: &str,
    messages: Vec<Message>,
    temporal: &TemporalSessionSnapshot,
    session_started_at: Option<f64>,
    blackboard: &Blackboard,
    context_scale_notified: Vec<String>,
    trust_policy: crate::session::TrustPolicy,
) -> HeadlessRunArchive {
    let mut result = HeadlessRunArchive {
        session_id: session_id.to_string(),
        sidecar_written: false,
        archived: false,
        archive_path: None,
        conversation_tokens: None,
    };
    let mut full = StoredConversation::full(
        session_id,
        messages,
        temporal,
        session_started_at,
        blackboard,
    );
    // v7（DP-16）水位随侧车同源落盘（一次性 run 恢复不开启，纯审计保真）。
    full.context_scale_notified = context_scale_notified;
    if !incremental_archive_due(base_dir, session_id, &full) {
        return result;
    }
    persist_conversation_sidecar(base_dir, &full);
    result.sidecar_written = !full.messages.is_empty();
    // 三键 journal 键按 `RUN-{session8}-` 前缀扫描；无头 run id
    // （`RUN-CLI-{ts}`）不携带会话段 ⇒ 显式注入本次 run（去重合并）。
    let explicit = [run_id.to_string()];
    let Some(pkg) = archive_session_package(
        base_dir,
        session_id,
        1, // 一次性 run = 单 prompt（ARC 审计 run id 的计数段）
        trust_policy,
        true,
        &explicit,
    )
    .await
    else {
        return result;
    };
    if pkg.status == "completed" {
        result.archived = true;
        result.archive_path = Some(pkg.path);
        result.conversation_tokens = Some(pkg.conversation_tokens);
    }
    result
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
    /// B3 复审 P2-3：close 时仍有 run 在进行 → 归档推迟到该 run 收尾
    /// （run 完成路径在 persist 后消费并补触发，保证存档包含会话最后一段
    /// 内容）。key = session id；每次 close 至多挂一张票，消费后即删。
    pending_archives: Arc<Mutex<HashMap<String, PendingArchiveTicket>>>,
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
        let mut guard = runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(&self.session);
    }
}

impl AcpServer {
    pub fn new() -> Self {
        // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, 实际使用裁决)：
        // plan 门已普适摘除——canned provider 不再应答 plan_write，首轮
        // 直接产出正文；反例自查门（§4.6/§14.76，0bi ⑩ 收窄后）仅在
        // 「有执行事实或未完成 plan」时于终答前加一轮——纯文本短答不再
        // 加轮（0bm 复审更正注释；下方第二条脚本为带工具轮场景备用）。
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
            acaf_fail_closed: orz_loop::controller::default_acaf_fail_closed(),
            runs: Arc::new(Mutex::new(HashMap::new())),
            pending_cancels: Arc::new(Mutex::new(HashMap::new())),
            pending_archives: Arc::new(Mutex::new(HashMap::new())),
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
        *self
            .hub_permission
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(hub);
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
            let map = self
                .runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(RunInFlight::Prompt(token)) = map.get(session_id) {
                token.cancel();
                drop(map);
                // The live-token path supersedes any remembered cancel.
                self.pending_cancels
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(session_id);
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
        *self
            .gateway
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sender);
    }

    /// Outbound gateway, if interactive mode is active.
    pub fn gateway(&self) -> Option<AcpAgentGatewaySender> {
        self.gateway
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
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
        self.handle_session_new_with_options(session_id, base_dir, trust_policy, policy, false)
            .await
    }

    /// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): `retrieval_enabled` 是
    /// 会话级独立启用门（三值检索模式退役后唯一授权状态；`false` 默认 =
    /// fail-closed）。orz-bin env/CLI 与 stdio/TUI 入口把旧 `retrieval_mode`
    /// 兼容解析映射为 bool 后传入（旧值不再有 lane 语义）。The activation
    /// sidecar (enable gate + per-role activations) resumes across process
    /// restarts.
    pub async fn handle_session_new_with_options(
        &self,
        session_id: &str,
        base_dir: Option<PathBuf>,
        trust_policy: crate::session::TrustPolicy,
        policy: PermissionPolicy,
        retrieval_enabled: bool,
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
        // GAP-RETRIEVAL-TOOLS + 0t: resume the activation sidecar; an
        // explicit enable selection overrides the persisted gate (no
        // transition journaling — 三值模式退役，mode transition 事件族不再
        // 产出)。
        let mut activation_snapshot = load_activation_sidecar(&base, session_id)
            .unwrap_or_else(|| StoredActivationSnapshot::for_session(session_id));
        activation_snapshot.retrieval_enabled = retrieval_enabled;
        // GAP-CONVERSATION-RESTORE + P2-13 B1: resume the full continuation
        // envelope (conversation + 黑板 live 视图 + LIF 会话轴) when a
        // previous process left one (cross-process continuation); a
        // brand-new session starts an empty envelope with the会话轴原点 =
        // 本会话创建时刻（首 prompt 前无消息，不落盘——第一个成功 prompt
        // 才写侧车）。Legacy 侧车（无 session_started_at）补当前墙钟。
        let mut continuation = load_conversation_sidecar(&base, session_id)
            .unwrap_or_else(|| StoredConversation::empty(session_id, None));
        if continuation.session_started_at.is_none() {
            continuation.session_started_at = Some(now_epoch_secs());
        }
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
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
                    continuation: Some(continuation),
                },
            );

        Ok(serde_json::json!({
            "session_id": session_id,
            "status": "created",
        }))
    }

    /// P1-1（2026-09-09, S2-R P2）：run 收尾把 host 内**存活**的浏览器句柄
    /// 回写会话，供下个 prompt 复用。只在 `ready()` 时覆盖——未启动/已死
    /// 句柄不得替换先前 ready 句柄（浏览器进程由会话 Arc 持有，host drop
    /// 只释放本 run 引用，不杀进程）。独立成函数以便确定性单测 ready 判定
    /// 与覆盖语义（真实「run 中懒启动换入 → 回写」依赖检索子代理链路 +
    /// 真实浏览器，属 S4 宿主机实测载体，P9）。
    fn fold_back_browser(
        sessions: &Mutex<HashMap<String, StoredSession>>,
        session_id: &str,
        live: crate::local_browser::SharedBrowser,
    ) {
        if live.ready()
            && let Some(session) = sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_mut(session_id)
        {
            session.browser = Some(live);
        }
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
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            let mut map = self
                .runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if matches!(map.get(session_id), Some(RunInFlight::Restore)) {
                return Err(AcpError::InvalidRequest(format!(
                    "prompt rejected: a snapshot restore is in flight for session {session_id}"
                )));
            }
            map.insert(session_id.to_string(), RunInFlight::Prompt(cancel.clone()));
            if let Some(stamped) = self
                .pending_cancels
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(session_id)
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
            self.runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(session_id);
            // B3 复审 P2-3：close 若已在 run 注册后发生（归档票已挂），
            // bootstrap 失败中止 run 时补触发存档。
            self.take_deferred_archive(session_id);
        }
        let handle = bootstrap?;

        // Phase 3 wiring: the real host — finalized GrokBuild toolset +
        // workspace trust + IP6 permission bridge. The bridge consumes the
        // outbound ACP gateway when interactive (`--stdio`); headless
        // (`None` gateway) fails closed: Read auto-allows, Bash → Deny.
        // (PermissionBridge spawns the manager actor via spawn_local, so
        // this path must run inside a LocalSet — the stdio server does.)
        let mut host = match self.build_host(&handle, session_id, &base_dir, policy) {
            Ok(host) => host,
            Err(e) => {
                // B3 复审 P2-3：build_host 失败中止 run 时同样补触发
                // deferred 存档（若 close 已发生）。
                self.take_deferred_archive(session_id);
                return Err(e);
            }
        };
        // GAP-INQUIRY-SPLIT (review P1-1, 2026-08-10): take the orientation
        // counter out of the session ONLY after every fallible step above
        // (restore-in-flight check, bootstrap, build_host) has succeeded —
        // an early `?` return must never leave the session with a taken-out
        // counter (the next prompt would silently restart at 0 and the
        // sidecar would be overwritten — violating "only an actual fire
        // resets", ADR-0010 §4.2). The sidecar is the fallback for a session
        // created before this slice (`None`): resume from it, else start 0.
        let mut orientation = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(session) = sessions.get_mut(session_id) else {
                // B3 复审 P2-3：close 发生在 run 注册后、取态前——run 中止，
                // 补触发 deferred 存档（读最近一次成功持久化的 sidecar）。
                self.take_deferred_archive(session_id);
                return Err(AcpError::SessionNotFound(session_id.to_string()));
            };
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
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(session) = sessions.get_mut(session_id) else {
                self.take_deferred_archive(session_id);
                return Err(AcpError::SessionNotFound(session_id.to_string()));
            };
            session.activation_snapshot.take().unwrap_or_else(|| {
                load_activation_sidecar(&base_dir, session_id)
                    .unwrap_or_else(|| StoredActivationSnapshot::for_session(session_id))
            })
        };
        // GAP-CONVERSATION-RESTORE + P2-13 B1: take out the full session
        // continuation (conversation + 黑板 live 视图 + LIF 会话轴) with
        // the orientation/activation state — same discipline: only after
        // every fallible step above, so an early `?` never leaves the
        // session with a taken-out continuation (the next prompt would
        // silently restart from zero and the sidecar would be overwritten).
        let mut continuation = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let Some(session) = sessions.get_mut(session_id) else {
                self.take_deferred_archive(session_id);
                return Err(AcpError::SessionNotFound(session_id.to_string()));
            };
            match session.continuation.take() {
                Some(cont) => cont,
                None => load_conversation_sidecar(&base_dir, session_id)
                    .unwrap_or_else(|| StoredConversation::empty(session_id, None)),
            }
        };
        if continuation.session_started_at.is_none() {
            continuation.session_started_at = Some(now_epoch_secs());
        }
        let session_started_at = continuation.session_started_at;
        let restored_lif = continuation.lif.clone();
        let restored_blackboard = continuation.blackboard.take();
        let mut conversation = std::mem::take(&mut continuation.messages);
        let restored_legacy_spikes = continuation.temporal_spikes.clone().unwrap_or_default();
        // 0bf ③（2026-09-22）：会话级负担档位元数据随续接包跨 prompt 延续
        // （SUCCESS-ONLY 纪律：失败 run 不更新；总值＝水位＋繁杂度增益，
        // 档位 50/70/90）。单键取代 0be 的两套档位簿记。
        let mut burden_tiers_notified = continuation.burden_tiers_notified.clone();
        // v7（S1 修订批，设计 §3.5.1，DP-16）：会话级刻度水位随续接包跨
        // prompt 延续（新会话为空；恢复侧车不重发）。
        let restored_context_scale_notified = continuation.context_scale_notified.clone();
        drop(continuation);
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
        // 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1): 检索启用会话把
        // browser_read 声明为常驻（静态双族工具面）——无持久句柄时也
        // 声明；调用期按需懒启动（§3.3/§3.5）。
        if activation_snapshot.retrieval_enabled {
            host.declare_browser_declared();
        }
        // 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3): 模式 A 机械 probe/
        // 降级整体退役——不再在 run 启动期拉起浏览器或做 capability 定档；
        // 浏览器可用性是纯事件事实（browser_launch_result），browser_read
        // 调用期按普通失败回传（§3.4），换道由模型自主。
        // GAP-RETRIEVAL-TOOLS (S4): seed the activation registry from the
        // sidecar — a cross-run AwaitingDisposition activation is restored
        // and journaled (the parent may dispose it in this run).
        let activation_snapshot_json =
            serde_json::to_value(&activation_snapshot).unwrap_or(serde_json::Value::Null);
        let controller = AgentLoopController::with_gateway(self.model_gateway.clone())
            // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, 实际使用
            // 裁决)：plan 门为普适删减——ACP 交互会话面与 CLI 一致不再
            // 启用首轮计划仪式（安全职责在机械审计层与提交门，不靠计划
            // 结构）；`plan_first` 仅作休眠开关保留（测试/回退），生产
            // 不再启用。console 默认面随生产路径保留（与 CLI 一致）。
            .with_console_default_enabled(true)
            // 0p S1 / W2 D-2 (2026-09-07)：真实会话轮计数——本 run 是会话
            // 内第 N 个用户 prompt（prompt_number 0 起算，事件面 1 起算）。
            .with_session_turn(prompt_number + 1)
            .with_snapshot_store(Some(handle.snapshot_store.clone()))
            // ACAF production flip (2026-08-16): the ACP session path shares
            // the signer-process client + fail-closed posture of the CLI run
            // path (previously the ACP path ran unticketed).
            .with_acaf(self.acaf.clone())
            .with_acaf_fail_closed(self.acaf_fail_closed)
            .with_retrieval_enabled(activation_snapshot.retrieval_enabled)
            .with_session_id(Some(session_id.to_string()))
            // v7（S1 修订批，设计 §3.5.1，DP-16）：会话级刻度水位注入 loop
            // ——每级每会话一次（跨 prompt 不重发；fire 时 controller 回写）。
            .with_context_scale_notified(restored_context_scale_notified.clone())
            .with_activation_snapshot(Some(&activation_snapshot_json))
            // P2-13 B1 (2026-09-03)：会话级 live 黑板续载——上个成功
            // prompt 的整板快照灌回（含 exec/edits/tool_actions/actions
            // receipts/failure_agg/检索分区/entities；动作栏注册与订单槽、
            // 依赖图在快照生成时已剥离）。先恢复整板（快照携带的检索
            // 分区副本在 activation 无分区时作为回退）。
            .with_live_blackboard(restored_blackboard)
            // THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1, 2026-08-27) + B1
            // 复审（2026-09-03）：检索分区归属 activation 侧车生命周期
            // （派发全量覆盖、run 结束无条件下沉、跨失败 run 延续既有
            // 语义），因此在会话黑板恢复后以其为准覆盖——compare-and-set
            // 保证与快照同内容时保持 restore 的 +1 徽章、不额外计变化；
            // `None` 保持快照内容（回退）。
            .with_retrieval_partitions(
                activation_snapshot.internal_ret.clone(),
                activation_snapshot.external_ret.clone(),
            );
        // P2-10 F2 §3.5 (I4) + P2-13 B1 (R5 conversation-relative 轴)：
        // 续接 LIF 会话轴——优先用精确快照（轮号 + 域机器 + spike 时间
        // 线）；legacy 侧车只有 temporal_spikes 时退回既有近似恢复。
        // 会话轴原点（session_started_at）对首个 prompt 亦预置，使
        // temporal `t` 与失败聚合首末时间跨 prompt 以会话为原点单调。
        match restored_lif {
            Some(snapshot) => controller.restore_lif_session(&snapshot, session_started_at),
            None => {
                if !restored_legacy_spikes.is_empty() {
                    controller.restore_temporal_spikes(restored_legacy_spikes);
                }
                if let Some(started_at) = session_started_at {
                    controller.preset_lif_session_axis(started_at);
                }
            }
        }

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

        // Every path: shut the journal writer task down (previously only the
        // success path released it). The run token is released AFTER the
        // sidecar persists below — close 落在收尾窗口内仍视为 run 进行中，
        // 归档按 B3 复审 P2-3 推迟，避免包缺最后一段内容。
        let _ = handle.journal.shutdown_async().await;

        // GAP-INQUIRY-SPLIT: persist the orientation counter — back into the
        // session and (best-effort, a read-only workspace must never fail a
        // run) to the sidecar, so the next prompt / process restart resumes
        // counting (§4.2).
        persist_orientation_sidecar(&base_dir, session_id, &orientation);
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(session_id)
        {
            session.orientation = Some(orientation);
        }
        // GAP-RETRIEVAL-TOOLS + 0t: persist the activation snapshot
        // (enable gate + activations + retrieval partitions)。三值模式退役
        // 后无 bootstrap/mode-transition 状态待清。
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
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(session_id)
        {
            session.activation_snapshot = Some(activation_snapshot);
        }
        // GAP-CONVERSATION-RESTORE + P2-13 B1: persist the full continuation
        // (conversation + LIF 会话轴快照 + 黑板 live 视图) — back into the
        // session and (best-effort, a read-only workspace must never fail a
        // run) to the sidecar, so the next prompt / process restart resumes
        // the history, the round/domain machine and the blackboard.
        // SUCCESS-ONLY: a failed run keeps the pre-run state (the run's
        // partial messages / board mutations never enter the continuation —
        // the journal is the failure evidence).
        //
        // Review P3-1 (three-agent 2026-08-10): the in-session write sits in
        // the SAME `is_ok` branch as the sidecar — a failure (incl. the
        // `journal.flush_async` error surfaced from `run_turn_with_guards`)
        // leaves `session.continuation` at `None`, so the next prompt's
        // take-out falls back to the sidecar (the pre-run state) instead
        // of diverging from it.
        // 0bf ③（2026-09-22，用户令「和疲劳度绑在一起做加权，一起算一个
        // 总值」）：用户侧**会话负担**提醒——机械附言、不进模型上下文；
        // 无新档 = None。总值＝水位（原疲劳分量，语义不变）＋繁杂度增益
        // （至多 +30）；会话关闭后的提醒去重随侧车持久化（SUCCESS-ONLY）。
        let mut burden_notice_text: Option<String> = None;
        if run_result.is_ok() {
            let lif = controller.lif_session_snapshot();
            let blackboard = controller.blackboard_conversation_snapshot();
            let mut full = StoredConversation::full(
                session_id,
                conversation,
                &lif,
                session_started_at,
                &blackboard,
            );
            // 水位分量：黑板 live 字节水位（原疲劳判定的输入，无压缩轮数
            // 门槛——B3 复审裁决）。
            let threshold = orz_loop::fatigue::live_budget_bytes();
            let board_bytes = serde_json::to_vec(&blackboard)
                .map(|v| v.len())
                .unwrap_or(0);
            let fatigue_percent = orz_loop::fatigue::fatigue_percent(board_bytes, threshold);
            // 繁杂度分量：RLI 影子读数（未启用/未就绪 = `None`——增益记 0，
            // 机械如实；FR-7 不落假值）。
            let cplx_reading = controller.rli_complexity_reading();
            // 复合判定：单次只投最高未投递档，跳过的低档一并落档（不刷屏、
            // 不补发）。
            if let Some(decision) = orz_loop::complexity::pending_burden_notice(
                fatigue_percent,
                cplx_reading.as_ref(),
                &burden_tiers_notified,
            ) {
                for tier in decision.tiers_to_mark {
                    if !burden_tiers_notified.iter().any(|t| t == tier) {
                        burden_tiers_notified.push(tier.to_string());
                    }
                }
                burden_notice_text = Some(decision.notice.text);
            }
            full.burden_tiers_notified = burden_tiers_notified.clone();
            // v7（S1 修订批，DP-16）：刻度水位随侧车落盘（跨 prompt 延续；
            // 失败 run 不更新——SUCCESS-ONLY 同疲劳档位）。
            full.context_scale_notified = controller.context_scale_notified_keys();
            persist_conversation_sidecar(&base_dir, &full);
            if let Some(session) = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get_mut(session_id)
            {
                session.continuation = Some(full);
            }
        }

        // Every path: release the run token here (stale cancels become
        // no-ops) — after persist, so the close-with-active-run detection in
        // `close_session` stays accurate through the whole tail window.
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
        // B3 复审 P2-3：close 在 run 进行中发生时，归档票在此消费（run 已
        // 收尾、成功路径的 sidecar 已同步落盘；失败/取消路径不更新
        // sidecar，存档最近一次成功内容）。会话未被 close（无票）= 无操作。
        self.take_deferred_archive(session_id);
        // 动态上下文滑块 S1（2026-09-15，设计 §3.5 DP-11④）：**增量归档
        // 时点**＝会话关闭一次（上方既有路径）＋ **实际上下文跨 500K 里程碑
        // 追加一次**——长单对话「不结束就没有存档」的缺口由此闭合。判据机械
        // （全量会话估算 chars/2，非视图刻度）、单调水位幂等；best-effort：
        // 水位只在包写成功后推进，失败留给下次 run 尾重判。
        if let Some(session) = self
            .sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(session_id)
            && let Some(continuation) = session.continuation.as_ref()
            && incremental_archive_due(&session.base_dir, session_id, continuation)
        {
            Self::spawn_archive_task(PendingArchiveTicket {
                session_id: session_id.to_string(),
                base_dir: session.base_dir.clone(),
                prompt_count: session.prompt_count,
                trust_policy: session.trust_policy,
                incremental: true,
            });
        }
        // P1-1（2026-09-09, S2-R P2）：浏览器生命周期跨 prompt 复用——run
        // 内懒启动换入的真实句柄在 host drop 前回写会话（host 每次 prompt
        // 新建；run 前不再预存默认 unavailable 句柄，见审查处理 §2 P1-1）。
        // 只在 ready 时回写：未启动/已死句柄不得覆盖先前 ready 句柄。浏览器
        // 进程由会话持有（Arc），host drop 只释放本 run 的引用、不杀进程；
        // 下个 prompt 经上方 re-inject 复用同一进程（§3.5 宿主机日常可用）。
        Self::fold_back_browser(&self.sessions, session_id, host.browser_session());

        match run_result {
            Ok((response, _, _)) => {
                let mut payload = serde_json::json!({
                "session_id": session_id,
                "response": response,
                "status": "completed",
                "run_id": run_id,
                });
                // 用户侧机械附言（不进模型上下文）：0bf ③ 起为**单条**负担
                // 提醒（复合总值口径——水位＋繁杂度增益；至多一条）。
                if let Some(notice) = burden_notice_text {
                    payload["user_notice"] = serde_json::Value::String(notice);
                }
                Ok(payload)
            }
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
            let sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions
                .get(session_id)
                .ok_or_else(|| AcpError::SessionNotFound(session_id.to_string()))?;
            (session.base_dir.clone(), session.trust_policy)
        };
        if self
            .runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(session_id)
        {
            return Err(AcpError::InvalidRequest(
                "grill turn rejected: a run is in flight for this session".into(),
            ));
        }
        // Session init + template: `Some` on the first turn (template
        // injected once — design §3, "会话开始"). A session switch rebinds
        // the grill session to the new session_id (at most one active).
        let (run_id, template) = {
            let mut grill = self
                .grill
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
            let mut grill = self
                .grill
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
                let mut grill = self
                    .grill
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
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
                let mut grill = self
                    .grill
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                let entry = grill.as_mut().expect("grill session still active");
                messages.push(Message {
                    role: Role::User,
                    content: user_input.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
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
        if let Some(g) = self
            .grill
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
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
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        self.sessions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .cloned()
            .collect()
    }

    /// Spawn the archive task for a ticket（后台 best-effort，失败只 warn、
    /// 不阻塞调用方）。
    fn spawn_archive_task(ticket: PendingArchiveTicket) {
        tokio::spawn(async move {
            archive_session_package(
                &ticket.base_dir,
                &ticket.session_id,
                ticket.prompt_count,
                ticket.trust_policy,
                ticket.incremental,
                // ACP 车道 run id 自带 `RUN-{session8}-` 会话段，前缀扫描
                // 足够；显式注入是无头车道（0ak）专用。
                &[],
            )
            .await;
        });
    }

    /// 消费 deferred 归档票并触发存档——run 收尾路径（以及注册后的早期
    /// 失败路径）调用；无票 = 无操作。
    fn take_deferred_archive(&self, session_id: &str) {
        if let Some(ticket) = self
            .pending_archives
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id)
        {
            Self::spawn_archive_task(ticket);
        }
    }

    /// Close a session and release its stored metadata.
    ///
    /// ACP 0.10.4 has no `session/close` method — the stdio server terminates
    /// on stdin EOF — but explicit closure keeps the session table bounded
    /// for long-lived/embedded hosts (2026-08-04 review P2-4).
    /// Returns `true` if the session existed and was removed.
    pub fn close_session(&self, session_id: &str) -> bool {
        let removed = {
            let mut sessions = self
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sessions.remove(session_id)
        };
        // 判定 close 瞬间是否仍有 run 在进行（Prompt）——必须在下面移除
        // runs token 之前检查。
        let run_in_flight = {
            let runs = self
                .runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            matches!(runs.get(session_id), Some(RunInFlight::Prompt(_)))
        };
        // P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §11.3/§12 R3）：
        // 会话关闭/归档 = 对话 + 黑板 live 视图合成单一 gzip 包 + 专用
        // ARC run journal 记 `session_archive` 事件。后台 best-effort——
        // close_session 保持同步、不等待压缩/审计完成（失败只 warn）。
        // B3 复审 P2-3：若仍有 run 在进行，归档推迟到该 run 收尾（persist
        // 之后 session 已不在表时补触发），保证包包含会话最后一段内容。
        if let Some(session) = removed.as_ref() {
            let ticket = PendingArchiveTicket {
                session_id: session_id.to_string(),
                base_dir: session.base_dir.clone(),
                prompt_count: session.prompt_count,
                trust_policy: session.trust_policy,
                incremental: false,
            };
            if run_in_flight {
                self.pending_archives
                    .lock()
                    .unwrap()
                    .insert(session_id.to_string(), ticket);
            } else {
                Self::spawn_archive_task(ticket);
            }
        }
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
        self.runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
        self.pending_cancels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(session_id);
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
            self.hub_permission
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone(),
            policy,
        )
        .map_err(AcpError::Host)?
        // FUS-HOST-RESOURCE-SAFETY (2026-09-12, 0z S1): 真机 ACP/stdio 路径的
        // 生产装配点——run 级 Job 硬上限 + 派发前资源预检门。
        .with_host_resource_safety()
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

    /// 0p S2 B5（2026-09-07，ADR-0010 §14.61 设计 B5）：`.gsa` 会话侧车
    /// 持久化写入路径接 orz-secrets 脱敏——conversations sidecar（对话 +
    /// 黑板 live 快照）落盘前对全部字符串值机械脱敏；archive 包直接压缩
    /// sidecar 字节，随本漏斗同链覆盖。
    #[test]
    fn conversation_sidecar_scrubs_secret_shaped_strings() {
        let base = std::env::temp_dir().join(format!(
            "orz-sidecar-scrub-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        let mut continuation = StoredConversation::empty("sess-scrub-01", None);
        let secret = "sk-abcdefghijklmnopqrstuvwxyz012345";
        continuation
            .messages
            .push(orz_loop::gateway::model::Message {
                role: orz_loop::gateway::model::Role::Tool,
                content: format!("pip install --api-key {secret} done"),
                tool_call_id: Some("call-scrub-1".to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
        persist_conversation_sidecar(&base, &continuation);
        let sidecar = base
            .join(".gsa")
            .join("conversations")
            .join("sess-scr.json");
        let content = std::fs::read_to_string(sidecar).unwrap();
        assert!(
            !content.contains(secret),
            "secret-shaped strings must not reach the sidecar: {content}"
        );
        assert!(content.contains("[REDACTED_SECRET]"), "{content}");
        let _ = std::fs::remove_dir_all(&base);
    }

    /// ACAF shadow 默认（ACAF signer 存量失败族修复，2026-09-07）：本文件
    /// 测试面沿用 orz-loop 单元测试约定——逻辑测试默认 shadow，ACAF 语义
    /// 由专门测试经 `with_acaf_fail_closed(true)` 显式开启（见
    /// `acaf_fail_closed_without_fabric_refuses_prompt`）。生产默认
    /// fail-closed 在下游 crate 测试编译时生效（orz-loop 的 `#[cfg(test)]`
    /// 特例不跨 crate），Task C（2026-09-04）翻转默认后未跟进的测试面
    /// 由此修复。
    fn shadow_server() -> AcpServer {
        AcpServer::new().with_acaf_fail_closed(false)
    }

    fn shadow_server_with_gateway(
        gateway: Arc<dyn orz_loop::gateway::model::ModelGateway>,
    ) -> AcpServer {
        AcpServer::with_gateway(gateway).with_acaf_fail_closed(false)
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

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1)：检索分区随快照 serde
    /// 往返（sidecar 持久化/恢复不丢全文与条目）；旧版快照（无该字段）
    /// 反序列化为 None——恢复时保持空分区而非报错。
    #[test]
    fn activation_snapshot_roundtrips_retrieval_partitions() {
        let internal = InternalRetSection {
            project_docs: vec!["design.md".to_string()],
            source_ledger: vec!["SRC-001 design.md".to_string()],
            response: Some("跨 run 的检索完成".to_string()),
            stamp: None,
        };
        let external = ExternalRetSection {
            web_sources: vec!["https://example.com/paper".to_string()],
            source_ledger: vec!["SRC-002 https://example.com/paper".to_string()],
            response: Some("跨 run 的网页检索完成".to_string()),
            stamp: None,
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

    #[tokio::test]
    async fn session_new_registers_session() {
        let base = test_dir();

        let server = shadow_server();
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

                let server = shadow_server();
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
                // finished.
                // GAP-INQUIRY-SPLIT: no per-turn orientation event (fires
                // only on the session-level 7-round trigger).
                // THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27): plan 门
                // 普适摘除——不再有 plan 轮（原 18 → 9，与无门轮次一致）。
                // 0ac S3①-b 收尾批（2026-09-15）：9 → 11——已提交批带入的
                // 三件合法事件（检索族探针 / `request_header_change` 请求头
                // 指纹 / `host_resource_snapshot` 资源档位快照）。
                // 0bi ⑩（ADR-0010 §14.76，2026-09-23）：answer 反例门收窄
                // ——纯文本短答（无工具轮、无未完成 plan）跳过门省一轮模型
                // 调用，`counterexample_gate` + 第二个 `model_output` 消失：
                // 11 → 9。实测序列：preflight / availability×2 / started /
                // prompt_submitted / header_change / model_output /
                // snapshot / finished。
                assert_eq!(replay.event_count, 9);
                assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    #[tokio::test]
    async fn unknown_session_returns_error() {
        let server = shadow_server();
        let result = server.handle_session_prompt("nonexistent", "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn second_prompt_continues_hash_chain() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();

                // Two scripted responses — one per prompt turn. 0bi ⑩
                // (2026-09-23): the counterexample gate is skipped for
                // text-only short answers (no tool rounds, no open plan),
                // so each prompt costs a single model round.
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("(fake) 第一轮终答。"),
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
                    // 0ac S3①-b 收尾批（2026-09-15）：9 → 11，同上三件
                    // 已提交批合法事件（检索族探针 / 请求头指纹 /
                    // 资源档位快照）。0bi ⑩（2026-09-23）：纯文本短答跳过
                    // 反例门省一轮——`counterexample_gate` + 第二个
                    // `model_output` 消失：11 → 9。
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

        let server = shadow_server();
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

    /// P1-1（2026-09-09, S2-R P2）：run 前不再把默认 unavailable 句柄预存
    /// 进会话（审查发现：run 前存入 unavailable、run 收尾不回写真实句柄 →
    /// host drop 杀浏览器，下个 prompt 只能整体冷启动）。未启动浏览器的
    /// 两轮 prompt 后，会话 browser 保持 None（修复前 = Some(unavailable)）。
    #[tokio::test]
    async fn prompt_does_not_seed_unavailable_browser_handle() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("(fake) 第一轮。"),
                    ScriptedResponse::text("(fake) 第一轮终答。"),
                    ScriptedResponse::text("(fake) 第二轮。"),
                    ScriptedResponse::text("(fake) 第二轮终答。"),
                ])));
                server
                    .handle_session_new(
                        "sess-browser-none",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server
                    .handle_session_prompt("sess-browser-none", "第一问")
                    .await
                    .unwrap();
                {
                    let sessions = server
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let session = sessions.get("sess-browser-none").expect("session");
                    assert!(
                        session.browser.is_none(),
                        "no launch happened → no seeded default handle expected"
                    );
                }
                server
                    .handle_session_prompt("sess-browser-none", "第二问")
                    .await
                    .unwrap();
                {
                    let sessions = server
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let session = sessions.get("sess-browser-none").expect("session");
                    assert!(
                        session.browser.is_none(),
                        "still none after the second prompt"
                    );
                }
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// P1-1（2026-09-09, S2-R P2）：`fold_back_browser` 的 ready 判定与覆盖
    /// 语义——ready 句柄覆盖；未启动/已死句柄不得覆盖先前 ready 句柄；无
    /// 句柄时保持 None。
    #[tokio::test]
    async fn fold_back_browser_only_overwrites_with_ready_handle() {
        let base = test_dir();
        let server = shadow_server();
        server
            .handle_session_new(
                "sess-fold-back",
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();

        // not-ready live（未启动）+ 会话无句柄 → 保持 None（不预存 unavailable）。
        let unavailable: crate::local_browser::SharedBrowser = Arc::new(
            crate::local_browser::UnavailableBrowserSession::new("no browser".to_string()),
        );
        assert!(!unavailable.ready());
        AcpServer::fold_back_browser(&server.sessions, "sess-fold-back", unavailable);
        {
            let sessions = server
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get("sess-fold-back").expect("session");
            assert!(session.browser.is_none(), "unready handle must not seed");
        }

        // 先注入 ready 句柄 S。
        let ready: crate::local_browser::SharedBrowser =
            crate::local_browser::tests::ready_stub_browser();
        {
            let mut sessions = server
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            sessions.get_mut("sess-fold-back").expect("session").browser = Some(ready.clone());
        }
        // not-ready live 不得覆盖先前 ready 句柄。
        let unavailable2: crate::local_browser::SharedBrowser = Arc::new(
            crate::local_browser::UnavailableBrowserSession::new("dead".to_string()),
        );
        AcpServer::fold_back_browser(&server.sessions, "sess-fold-back", unavailable2);
        {
            let sessions = server
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get("sess-fold-back").expect("session");
            assert!(
                Arc::ptr_eq(&ready, session.browser.as_ref().expect("ready handle")),
                "unready handle must not replace a ready one"
            );
        }
        // ready live 覆盖（换入新句柄后回写）。
        let live: crate::local_browser::SharedBrowser =
            crate::local_browser::tests::ready_stub_browser();
        AcpServer::fold_back_browser(&server.sessions, "sess-fold-back", live.clone());
        {
            let sessions = server
                .sessions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let session = sessions.get("sess-fold-back").expect("session");
            assert!(
                Arc::ptr_eq(&live, session.browser.as_ref().expect("live handle")),
                "ready live handle must be folded back"
            );
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    /// P1-1（2026-09-09, S2-R P2）：会话级浏览器句柄的机械生命周期——
    /// ready 句柄注入后跨两个 prompt 保持同一 Arc（run 收尾只在 ready 时
    /// 回写，host drop 只释放本 run 引用、不触发底层 drop）；close_session
    /// 恰好 shutdown 一次并释放最后一个 Arc。
    #[tokio::test]
    async fn browser_handle_survives_prompt_boundary_and_close_shuts_down_once() {
        use std::sync::atomic::AtomicUsize;

        struct CountingBrowser {
            shutdowns: Arc<AtomicUsize>,
            drops: Arc<AtomicUsize>,
        }

        impl Drop for CountingBrowser {
            fn drop(&mut self) {
                self.drops.fetch_add(1, Ordering::SeqCst);
            }
        }

        #[async_trait::async_trait]
        impl crate::local_browser::BrowserSession for CountingBrowser {
            async fn read_page(
                &self,
                _url: &str,
                _mode: crate::local_browser::ReadMode,
            ) -> Result<crate::local_browser::PageReadOutcome, crate::local_browser::CdpError>
            {
                Err(crate::local_browser::CdpError::Io(
                    "test stub: no read".to_string(),
                ))
            }

            async fn control(
                &self,
                _action: crate::local_browser::BrowserControlAction,
                _timeout: std::time::Duration,
            ) -> Result<crate::local_browser::BrowserControlOutcome, crate::local_browser::CdpError>
            {
                Err(crate::local_browser::CdpError::Io(
                    "test stub: no control".to_string(),
                ))
            }

            async fn download_or_read(
                &self,
                _url: &str,
                _download_dir: &std::path::Path,
            ) -> Result<crate::local_browser::BrowserDownloadOutcome, crate::local_browser::CdpError>
            {
                Err(crate::local_browser::CdpError::Io(
                    "test stub: no download".to_string(),
                ))
            }

            fn ready(&self) -> bool {
                true
            }

            async fn shutdown(&self) {
                self.shutdowns.fetch_add(1, Ordering::SeqCst);
            }
        }

        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let shutdowns = Arc::new(AtomicUsize::new(0));
                let drops = Arc::new(AtomicUsize::new(0));
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
                    ScriptedResponse::text("(fake) 第一轮。"),
                    ScriptedResponse::text("(fake) 第一轮终答。"),
                    ScriptedResponse::text("(fake) 第二轮。"),
                    ScriptedResponse::text("(fake) 第二轮终答。"),
                ])));
                server
                    .handle_session_new(
                        "sess-browser-lifecycle",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                let browser: crate::local_browser::SharedBrowser = Arc::new(CountingBrowser {
                    shutdowns: shutdowns.clone(),
                    drops: drops.clone(),
                });
                {
                    let mut sessions = server
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    sessions
                        .get_mut("sess-browser-lifecycle")
                        .expect("session")
                        .browser = Some(browser.clone());
                }

                server
                    .handle_session_prompt("sess-browser-lifecycle", "第一问")
                    .await
                    .unwrap();
                server
                    .handle_session_prompt("sess-browser-lifecycle", "第二问")
                    .await
                    .unwrap();

                // host 两轮 drop 后底层对象不得被释放（会话仍持同一 Arc）。
                assert_eq!(drops.load(Ordering::SeqCst), 0, "host drop must not kill");
                let same_handle = {
                    let sessions = server
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let session = sessions.get("sess-browser-lifecycle").expect("session");
                    session.browser.clone().expect("browser handle")
                };
                assert!(
                    Arc::ptr_eq(&browser, &same_handle),
                    "同一 ready 句柄跨 prompt 保持"
                );
                assert_eq!(shutdowns.load(Ordering::SeqCst), 0);

                // 释放测试自身持有的 Arc——close 后会话表移除应是最后一个引用。
                drop(same_handle);
                drop(browser);
                assert!(server.close_session("sess-browser-lifecycle"));
                // close_session 触发 detached shutdown——短暂等待任务落地。
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                assert_eq!(
                    shutdowns.load(Ordering::SeqCst),
                    1,
                    "close_session must shut down the lane exactly once"
                );
                assert_eq!(
                    drops.load(Ordering::SeqCst),
                    1,
                    "session removal drops the last Arc"
                );
                let _ = std::fs::remove_dir_all(&base);
            })
            .await
    }

    /// P2-13 B3（2026-09-03，ADR-0010 §14.52 / 设计 §11.3/§12 R3）：会话
    /// 存档 = 单一 gzip 包（对话 + 黑板 live 视图 + 元数据，纯打包零内容
    /// 变换）+ 专用 ARC run journal 的 `session_archive` v0.2 事件。
    #[tokio::test]
    async fn session_archive_single_gzip_package_and_event() {
        use std::io::Read;

        let base = test_dir();
        let session_id = "sess-arch-test-0001";
        let full = StoredConversation::full(
            session_id,
            vec![Message {
                role: Role::User,
                content: "任务".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            }],
            &TemporalSessionSnapshot {
                round: 1,
                has_success: true,
                current_domain: orz_assurance::lif::Domain::Normal,
                entry_round: 1,
                spikes: Vec::new(),
                rli_shadow: None,
            },
            Some(1_700_000_000.0),
            &Blackboard::default(),
        );
        persist_conversation_sidecar(&base, &full);

        archive_session_package(
            &base,
            session_id,
            3,
            crate::session::TrustPolicy::Skip,
            false,
            &[],
        )
        .await;

        let suffix: String = session_id.chars().take(8).collect();
        let gz_path = base
            .join(".gsa")
            .join("archives")
            .join(format!("{suffix}.json.gz"));
        assert!(
            gz_path.is_file(),
            "archive package missing: {}",
            gz_path.display()
        );
        let mut decoder = flate2::read::GzDecoder::new(
            std::fs::File::open(&gz_path).expect("archive package opens"),
        );
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .expect("archive package decodes");
        // 动态上下文滑块 S1（2026-09-15，设计 §3.5）：包内为信封结构，读取
        // 一律走 tolerant 解码（新信封 + 旧裸包都接受）。
        let (stored, keys) =
            decode_archive_package(&decoded).expect("archive package decodes (envelope)");
        assert_eq!(stored.session_id, session_id);
        assert_eq!(stored.messages.len(), 1);
        assert_eq!(stored.messages[0].content, "任务");
        // 三键互标段齐备（判据「存档一致性」：缺一即判存档不完整）。
        let keys = keys.expect("archive_keys present");
        assert_eq!(keys["window_rounds"]["axis"], "window");
        assert_eq!(
            keys["window_rounds"]["end"], 0,
            "本测试对话无工具声明轮 ⇒ 窗口轮跨度 end=0（不给伪值）"
        );
        assert_eq!(keys["lif"]["round_end"], 1, "LIF 轮跨度取 sidecar 快照");
        assert_eq!(keys["lif"]["domain"], "normal");
        assert_eq!(keys["lif"]["session_started_at"], 1_700_000_000.0);
        assert_eq!(
            keys["ledger"]["exists"], false,
            "本测试无外挂台账 ⇒ 确定性结构（不整段缺失）"
        );
        assert!(keys["ledger"]["first_seq"].is_null());
        assert!(keys["journal"]["runs"].is_array());
        assert!(keys["compaction"].is_array());
        assert!(
            keys["axis_note"].as_str().unwrap().contains("尾批"),
            "临时键必须显式标注尾批前的退化口径: {keys}"
        );

        // 「上次归档读数」水位（增量归档幂等锚点）在包写成功后落地。
        assert_eq!(
            last_archived_tokens(&base, session_id),
            Some(estimate_conversation_tokens(&stored)),
            "里程碑水位 = 本次归档的会话估算读数"
        );
        assert!(
            !incremental_archive_due(&base, session_id, &stored),
            "刚归档过 ⇒ 同一里程碑不得重复打包"
        );
        assert!(
            !incremental_archive_due(&base, "sess-arch-fresh-x", &stored),
            "小会话（<500K）永不触发增量归档"
        );

        // 专用 ARC run journal：run_preflight（bootstrap）→ session_archive
        // → run_finished。
        let journal_dir = base
            .join(".gsa")
            .join("runs")
            .join(format!("ARC-{suffix}-3"));
        let events = std::fs::read_to_string(journal_dir.join("events.jsonl"))
            .expect("ARC archive journal written");
        assert!(
            events.contains("\"event_type\":\"session_archive\""),
            "{events}"
        );
        assert!(
            events.contains("\"event_type\":\"run_finished\""),
            "{events}"
        );
        assert!(events.contains("\"status\":\"completed\""), "{events}");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 动态上下文滑块 S1（2026-09-15，设计 §3.5 / DP-11）：① 旧裸包仍可
    /// tolerant 解码；② `archive_keys` 的三键由**真实文件**机械派生（台账
    /// `[seq]` 跨度、journal run+sequence、A 类压缩清单）；③ 增量归档里程碑
    /// 挂**实际上下文**刻度（≥500K 且比上次归档读数多 500K），单调幂等。
    #[tokio::test]
    async fn archive_keys_and_incremental_milestones_follow_the_design() {
        let base = test_dir();
        let session_id = "sess-keys-test-0002";
        let suffix: String = session_id.chars().take(8).collect();
        let snapshot = TemporalSessionSnapshot {
            round: 7,
            has_success: true,
            current_domain: orz_assurance::lif::Domain::Pressure,
            entry_round: 3,
            spikes: Vec::new(),
            rli_shadow: None,
        };
        let conversation = |chars: usize| {
            StoredConversation::full(
                session_id,
                vec![Message {
                    role: Role::User,
                    content: "z".repeat(chars),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                }],
                &snapshot,
                Some(1_700_000_000.0),
                &Blackboard::default(),
            )
        };
        let small = conversation(10);

        // ① 旧包（裸 StoredConversation）兼容：tautan 解码返回无 keys。
        let legacy_bytes = serde_json::to_vec(&small).unwrap();
        let (decoded, keys) =
            decode_archive_package(&legacy_bytes).expect("legacy package decodes");
        assert_eq!(decoded.session_id, session_id);
        assert!(keys.is_none(), "旧包没有 archive_keys 段");

        // ② 真实文件派生（台账行 / run journal / 压缩存档清单）。
        let ledger_dir = base.join(".gsa").join("ledger");
        std::fs::create_dir_all(&ledger_dir).unwrap();
        std::fs::write(
            ledger_dir.join("current.md"),
            "[1] 轮次 1: read_file 目标=a.py 结果=sha256:x 最终回复=（无）\n\
             [2] 轮次 2: grep 目标=b.rs 结果=sha256:y 最终回复=ok\n",
        )
        .unwrap();
        let session_run = base
            .join(".gsa")
            .join("runs")
            .join(format!("RUN-{suffix}-0"));
        std::fs::create_dir_all(&session_run).unwrap();
        std::fs::write(
            session_run.join("events.jsonl"),
            "{\"sequence\":0,\"event_type\":\"run_started\"}\n\
             {\"sequence\":1,\"event_type\":\"run_finished\"}\n",
        )
        .unwrap();
        // 非本会话的 run 不得混入（run id 前缀是唯一会话键）。
        let other_run = base.join(".gsa").join("runs").join("RUN-othe0000-0");
        std::fs::create_dir_all(&other_run).unwrap();
        std::fs::write(other_run.join("events.jsonl"), "{\"sequence\":0}\n").unwrap();
        let compaction_dir = base.join(".gsa").join("compaction");
        std::fs::create_dir_all(&compaction_dir).unwrap();
        std::fs::write(compaction_dir.join("compaction-RUN-x-0001.md"), "# 摘要").unwrap();

        let keys = build_archive_keys(&base, session_id, &small, &[]);
        assert_eq!(keys["ledger"]["exists"], true);
        assert_eq!(keys["ledger"]["first_seq"], 1);
        assert_eq!(keys["ledger"]["last_seq"], 2);
        assert_eq!(keys["ledger"]["rows"], 2);
        assert_eq!(
            keys["journal"]["runs"].as_array().unwrap().len(),
            1,
            "只纳入本会话 RUN-<session8>-* 的 run: {keys}"
        );
        assert_eq!(
            keys["journal"]["runs"][0]["run_id"],
            format!("RUN-{suffix}-0")
        );
        assert_eq!(keys["journal"]["runs"][0]["events"], 2);
        assert_eq!(keys["journal"]["runs"][0]["last_sequence"], 1);
        assert_eq!(keys["compaction"][0]["id"], "compaction-RUN-x-0001");
        assert_eq!(keys["lif"]["round_start"], 3);
        assert_eq!(keys["lif"]["round_end"], 7);
        assert_eq!(keys["lif"]["domain"], "pressure");

        // ③ 增量归档里程碑（实际上下文刻度，单调幂等）。
        let unarchived = "sess-milestone-x";
        assert!(
            !incremental_archive_due(&base, unarchived, &conversation(900_000)),
            "≈450K < 500K ⇒ 不触发"
        );
        let first = conversation(1_100_000); // ≈550K
        assert!(incremental_archive_due(&base, unarchived, &first));
        record_archived_tokens(&base, unarchived, estimate_conversation_tokens(&first));
        assert!(
            !incremental_archive_due(&base, unarchived, &first),
            "同一里程碑（水位未再跨 500K）不得重复打包"
        );
        assert!(
            incremental_archive_due(&base, unarchived, &conversation(2_200_000)),
            "跨下一个 500K 里程碑再归档一次"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0ak（GAP-INCREMENTAL-ARCHIVE-HEADLESS，2026-09-16 用户裁决采 B）钉子：
    /// 无头一次性 run 收尾归档端到端——跨 500K 里程碑产出「对话侧车 ＋
    /// 三键包 ＋ ARC 审计 journal（incremental 标记）＋ 里程碑水位」，且
    /// 三键 journal 键**显式纳入无头 run**（`RUN-CLI-{ts}` 不匹配
    /// `RUN-{session8}-` 前缀，靠 `explicit_runs` 注入；缺失即三键不齐）。
    #[tokio::test]
    async fn headless_run_archive_produces_three_key_package_with_explicit_run() {
        use std::io::Read;

        let base = test_dir();
        let ts = "6aa999d6";
        // 会话身份口径：`{ts}-cli` ⇒ session8 = ts，与 `RUN-CLI-{ts}` 一眼互认。
        let session_id = format!("{ts}-cli");
        let run_id = format!("RUN-CLI-{ts}");
        let full = StoredConversation::full(
            &session_id,
            // 1.1M chars ⇒ 估算 ≈550K ≥ 500K（判据同尺）。
            vec![Message {
                role: Role::User,
                content: "z".repeat(1_100_000),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            }],
            &TemporalSessionSnapshot {
                round: 9,
                has_success: true,
                current_domain: orz_assurance::lif::Domain::Normal,
                entry_round: 1,
                spikes: Vec::new(),
                rli_shadow: None,
            },
            Some(1_700_000_000.0),
            &Blackboard::default(),
        );

        // 无头 run 的 journal（生产中归档时点必已 shutdown 落盘；缺失的
        // run 与前缀扫描同纪律——静默跳过，不编造键值）。
        let run_dir = base.join(".gsa").join("runs").join(&run_id);
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::write(
            run_dir.join("events.jsonl"),
            "{\"sequence\":0,\"event_type\":\"run_started\"}\n\
             {\"sequence\":1,\"event_type\":\"run_finished\"}\n",
        )
        .unwrap();

        let result = headless_session_archive(
            &base,
            &session_id,
            &run_id,
            full.messages.clone(),
            full.lif.as_ref().expect("lif snapshot"),
            full.session_started_at,
            full.blackboard.as_ref().expect("blackboard"),
            vec!["500k".to_string()],
            crate::session::TrustPolicy::Skip,
        )
        .await;

        assert!(result.archived, "跨里程碑 ⇒ 归档落盘: {result:?}");
        assert!(result.sidecar_written);
        let tokens = result.conversation_tokens.expect("token estimate");
        assert!(tokens >= 500_000, "估算读数须与判据同尺: {tokens}");

        // 侧车（归档源）与包（三键信封）都在。
        let sidecar = base
            .join(".gsa")
            .join("conversations")
            .join(format!("{ts}.json"));
        assert!(sidecar.is_file(), "sidecar missing: {}", sidecar.display());
        let gz_path = result.archive_path.expect("archive path");
        assert!(gz_path.is_file(), "archive missing: {}", gz_path.display());
        let mut decoder = flate2::read::GzDecoder::new(std::fs::File::open(&gz_path).unwrap());
        let mut decoded = Vec::new();
        decoder.read_to_end(&mut decoded).unwrap();
        let (stored, keys) = decode_archive_package(&decoded).expect("envelope decodes");
        assert_eq!(stored.session_id, session_id);
        assert_eq!(
            stored.context_scale_notified,
            vec!["500k".to_string()],
            "v7 水位随侧车同源落盘"
        );
        let keys = keys.expect("archive_keys present");
        let runs = keys["journal"]["runs"].as_array().expect("runs array");
        assert!(
            runs.iter().any(|r| r["run_id"] == run_id),
            "三键 journal 键必须显式纳入无头 run（前缀扫描不中）: {keys}"
        );
        assert_eq!(keys["lif"]["round_end"], 9);

        // ARC 审计 journal：incremental 里程碑归档（一次性 run = 单 prompt）。
        let arc_journal = base
            .join(".gsa")
            .join("runs")
            .join(format!("ARC-{ts}-1"))
            .join("events.jsonl");
        let events = std::fs::read_to_string(&arc_journal).expect("ARC journal written");
        assert!(
            events.contains("\"event_type\":\"session_archive\""),
            "{events}"
        );
        assert!(events.contains("\"incremental\":true"), "{events}");

        // 里程碑水位落地（幂等锚点）。
        assert!(last_archived_tokens(&base, &session_id).is_some());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0ak 钉子：阈值下（<500K）无头收尾**零产物**——不落侧车、不打包、
    /// 无 ARC journal、无水位文件。
    #[tokio::test]
    async fn headless_run_archive_below_threshold_writes_nothing() {
        let base = test_dir();
        let session_id = "0000abcd-cli";
        let result = headless_session_archive(
            &base,
            session_id,
            "RUN-CLI-0000abcd",
            vec![Message {
                role: Role::User,
                content: "短任务".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            }],
            &TemporalSessionSnapshot {
                round: 1,
                has_success: true,
                current_domain: orz_assurance::lif::Domain::Normal,
                entry_round: 1,
                spikes: Vec::new(),
                rli_shadow: None,
            },
            Some(1_700_000_000.0),
            &Blackboard::default(),
            Vec::new(),
            crate::session::TrustPolicy::Skip,
        )
        .await;

        assert!(!result.archived);
        assert!(!result.sidecar_written);
        assert!(result.archive_path.is_none());
        assert!(result.conversation_tokens.is_none());
        assert!(
            !base.join(".gsa").join("archives").exists(),
            "阈值下不得产生 archives 产物"
        );

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0br S3 钉子（Web「归档活跃会话」动作）：按需归档复用同一打包原语
    /// ——包落盘＋ARC journal 事件＋里程碑水位一次到位；无侧车/空会话
    /// 如实报错不静默。
    #[tokio::test]
    async fn archive_on_demand_reuses_the_close_archive_primitive() {
        let base = test_dir();
        let session_id = "6ab7dem0-cli";
        let snapshot = TemporalSessionSnapshot {
            round: 4,
            has_success: true,
            current_domain: orz_assurance::lif::Domain::Normal,
            entry_round: 1,
            spikes: Vec::new(),
            rli_shadow: None,
        };
        let conversation = StoredConversation::full(
            session_id,
            vec![Message {
                role: Role::User,
                content: "问题正文".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            }],
            &snapshot,
            Some(1_700_000_000.0),
            &Blackboard::default(),
        );
        let sidecar_dir = base.join(".gsa").join("conversations");
        std::fs::create_dir_all(&sidecar_dir).unwrap();
        std::fs::write(
            sidecar_dir.join("6ab7dem0.json"),
            serde_json::to_vec(&conversation).unwrap(),
        )
        .unwrap();

        let report = archive_session_on_demand(&base, "6ab7dem0")
            .await
            .expect("archives");
        assert!(report.contains("6ab7dem0.json.gz"), "{report}");
        assert!(report.contains("tokens="), "{report}");
        let gz = base.join(".gsa").join("archives").join("6ab7dem0.json.gz");
        assert!(gz.is_file());
        // ARC 审计 journal（增量标记不得出现——按需归档是关闭同口径）。
        let events = std::fs::read_to_string(base.join(".gsa/runs/ARC-6ab7dem0-1/events.jsonl"))
            .expect("ARC journal");
        assert!(
            events.contains("\"event_type\":\"session_archive\""),
            "{events}"
        );
        assert!(!events.contains("incremental"), "{events}");
        // 水位随包落盘（后续里程碑判定的锚点）。
        assert!(last_archived_tokens(&base, session_id).is_some());

        // 幂等：第二次按需归档再次打包成功（覆盖写，水位只进不退）。
        archive_session_on_demand(&base, "6ab7dem0")
            .await
            .expect("re-archives");

        // 无侧车会话 ⇒ 显式错误。
        let err = archive_session_on_demand(&base, "0000dead")
            .await
            .unwrap_err();
        assert!(err.contains("无侧车"), "{err}");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0br S3 批三钉子：无侧车会话（§14.68 未跨里程碑的无头 `-p` run）
    /// 经 journal 重构归档——prompt/输出事实成包、`reconstructed_from_journal`
    /// 标记在位、三键段显式纳入无头 run。
    #[tokio::test]
    async fn archive_on_demand_reconstructs_sidecarless_sessions_from_journal() {
        use std::io::Write as _;
        let base = test_dir();
        let session8 = "6aabf5eb";
        let run_dir = base
            .join(".gsa")
            .join("runs")
            .join(format!("RUN-CLI-{session8}"));
        std::fs::create_dir_all(&run_dir).unwrap();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(run_dir.join("events.jsonl"))
            .unwrap();
        let event = |seq: u64, ty: &str, payload: serde_json::Value| {
            serde_json::json!({
                "schema_version": "0.2.0-draft",
                "run_id": format!("RUN-CLI-{session8}"),
                "sequence": seq,
                "timestamp": "2026-09-17T14:15:08.303424100+00:00",
                "event_type": ty,
                "payload": payload,
            })
        };
        for ev in [
            event(0, "run_preflight", serde_json::json!({})),
            event(
                1,
                "prompt_submitted",
                serde_json::json!({"prompt": "处理摩擦项", "character_count": 6}),
            ),
            event(
                2,
                "model_output",
                serde_json::json!({"text": null, "tool_calls": [
                    {"name": "read_file", "arguments": {"target_file": "a.md"}, "call_id": "c1"}
                ]}),
            ),
            event(
                3,
                "model_output",
                serde_json::json!({"text": "完成报告", "tool_calls": []}),
            ),
            event(
                4,
                "run_finished",
                serde_json::json!({"status": "completed"}),
            ),
        ] {
            writeln!(f, "{ev}").unwrap();
        }
        drop(f);

        let report = archive_session_on_demand(&base, session8)
            .await
            .expect("reconstructed archive");
        assert!(report.contains("journal 重构"), "{report}");
        let gz = base
            .join(".gsa")
            .join("archives")
            .join(format!("{session8}.json.gz"));
        assert!(gz.is_file());
        let mut decoder = flate2::read::GzDecoder::new(std::fs::File::open(&gz).unwrap());
        let mut decoded = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut decoded).unwrap();
        let envelope: serde_json::Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(envelope["schema"], "session-archive-package-v0.2");
        let conversation = envelope["conversation"].as_object().unwrap();
        assert_eq!(
            conversation
                .get("reconstructed_from_journal")
                .and_then(|v| v.as_bool()),
            Some(true),
            "重构标记必须在包内"
        );
        let messages = conversation["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3, "用户＋工具轮助手＋正文助手");
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "处理摩擦项");
        assert_eq!(messages[1]["role"], "assistant");
        assert_eq!(messages[1]["tool_calls"][0]["name"], "read_file");
        assert_eq!(messages[2]["content"], "完成报告");
        // 三键段 journal 键显式纳入无头 run（前缀扫描不中）。
        let runs = envelope["archive_keys"]["journal"]["runs"]
            .as_array()
            .unwrap();
        assert!(
            runs.iter()
                .any(|r| r["run_id"] == format!("RUN-CLI-{session8}")),
            "{runs:?}"
        );

        // 零对话事实 ⇒ 如实报错。
        let err = archive_session_on_demand(&base, "ffffffff")
            .await
            .unwrap_err();
        assert!(err.contains("无可重构"), "{err}");

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 0ak 钉子：幂等——同一会话同一里程碑内第二次收尾不重复打包
    /// （水位单调锚点；未到期连侧车都不重落）。
    #[tokio::test]
    async fn headless_run_archive_is_idempotent_per_milestone() {
        let base = test_dir();
        let session_id = "1111beef-cli";
        let args = |content: String| {
            (
                vec![Message {
                    role: Role::User,
                    content,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                }],
                TemporalSessionSnapshot {
                    round: 2,
                    has_success: true,
                    current_domain: orz_assurance::lif::Domain::Normal,
                    entry_round: 1,
                    spikes: Vec::new(),
                    rli_shadow: None,
                },
            )
        };
        let (messages, temporal) = args("z".repeat(1_100_000));
        let first = headless_session_archive(
            &base,
            session_id,
            "RUN-CLI-1111beef",
            messages,
            &temporal,
            Some(1_700_000_000.0),
            &Blackboard::default(),
            Vec::new(),
            crate::session::TrustPolicy::Skip,
        )
        .await;
        assert!(first.archived, "首归档成立");

        // 同一里程碑内（水位 +500K 未跨）第二次收尾：零重复产物。
        let (messages, temporal) = args("z".repeat(1_200_000));
        let second = headless_session_archive(
            &base,
            session_id,
            "RUN-CLI-1111beef",
            messages,
            &temporal,
            Some(1_700_000_000.0),
            &Blackboard::default(),
            Vec::new(),
            crate::session::TrustPolicy::Skip,
        )
        .await;
        assert!(!second.archived, "同一里程碑不得重复打包: {second:?}");
        assert!(!second.sidecar_written);
        assert!(second.archive_path.is_none());

        let _ = std::fs::remove_dir_all(&base);
    }

    /// B3 复审 P2-3：close 时仍有 run 在进行 → 归档推迟到该 run 收尾
    /// （run 完成路径消费票后补触发），包包含最新 sidecar；不立即落包。
    #[tokio::test]
    async fn close_with_active_run_defers_archive_until_run_completion() {
        let base = test_dir();
        let session_id = "sess-defer-test-0001";
        let full = StoredConversation::full(
            session_id,
            vec![Message {
                role: Role::User,
                content: "任务".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            }],
            &TemporalSessionSnapshot {
                round: 1,
                has_success: true,
                current_domain: orz_assurance::lif::Domain::Normal,
                entry_round: 1,
                spikes: Vec::new(),
                rli_shadow: None,
            },
            Some(1_700_000_000.0),
            &Blackboard::default(),
        );
        persist_conversation_sidecar(&base, &full);

        let server = shadow_server();
        server
            .handle_session_new(
                session_id,
                Some(base.clone()),
                crate::session::TrustPolicy::Skip,
            )
            .await
            .unwrap();
        // 模拟 run 进行中（真实路径在 bootstrap 前注册 token）。
        server
            .runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
                session_id.to_string(),
                RunInFlight::Prompt(tokio_util::sync::CancellationToken::new()),
            );

        assert!(server.close_session(session_id));
        let suffix: String = session_id.chars().take(8).collect();
        let gz_path = base
            .join(".gsa")
            .join("archives")
            .join(format!("{suffix}.json.gz"));
        assert!(
            !gz_path.exists(),
            "archive must be deferred while a run is in flight"
        );

        // run 收尾路径（handle_session_prompt 尾部同款调用）消费票并补触发。
        server.take_deferred_archive(session_id);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !gz_path.exists() && std::time::Instant::now() < deadline {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        assert!(
            gz_path.is_file(),
            "deferred archive written after run completion"
        );

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
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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

                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                    // 0x S1：首个动作批次结束后多一轮开局问询回答（软门消费）。
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

                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                    // 0x S1：开局问询回答轮（软门消费）。
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
                // run pulls [直调工具, 开局问询回答, 草稿, 终答]（0x S1：
                // 首个动作批次结束后多一轮开局问询回答，软门消费）。
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
                    ScriptedResponse::text("完成。"),
                ];
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(script)));
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

                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                    // 0x S1：开局问询回答轮（软门消费）。
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
                let server = Arc::new(shadow_server_with_gateway(Arc::new(
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
                let server = Arc::new(shadow_server());
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
                let server = Arc::new(shadow_server_with_gateway(Arc::new(
                    FakeProvider::from_texts(vec!["结果：完成", "结果：完成"]),
                )));
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
                let server = Arc::new(shadow_server_with_gateway(Arc::new(
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
                let server = Arc::new(shadow_server_with_gateway(Arc::new(
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

        let server = shadow_server();
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

        let server = shadow_server();
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
        let server = shadow_server();
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
        let server = shadow_server();
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

        let server = shadow_server();
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
        let server = shadow_server();
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
        server
            .runs
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(
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
        let server = shadow_server();
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
                let server = shadow_server();
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
                        server
                            .runs
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .get("sess-restore-prompt"),
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

        let server = shadow_server();
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
            !server
                .runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains_key("sess-release"),
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
            !server
                .runs
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .contains_key("sess-release"),
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
                let server = shadow_server();
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
                    !server
                        .runs
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .contains_key("sess-token"),
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

                let server = shadow_server();
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
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                    let mut runs = server
                        .runs
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
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
                server
                    .runs
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove("sess-grill");

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
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                round: None,
            },
            Message {
                role: Role::Assistant,
                content: "回答".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: Some("推理过程".to_string()),
                round: None,
            },
            Message {
                role: Role::Tool,
                content: "工具结果".to_string(),
                tool_call_id: Some("call-1".to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            },
        ];
        // P2-10 F2 §3.5 (I4) + P2-13 B1: temporal spikes / LIF 会话轴快照
        // 与黑板 live 视图同信封往返——round 与域机器精确续接（不再只有
        // spike 时间线近似）。
        let spikes = vec![
            orz_assurance::lif::DomainSpike {
                t: 12.5,
                domain: orz_assurance::lif::Domain::Normal,
            },
            orz_assurance::lif::DomainSpike {
                t: 340.0,
                domain: orz_assurance::lif::Domain::Stuck,
            },
        ];
        let lif = orz_assurance::lif::TemporalSessionSnapshot {
            round: 41,
            has_success: true,
            current_domain: orz_assurance::lif::Domain::Stuck,
            entry_round: 33,
            spikes: spikes.clone(),
            rli_shadow: None,
        };
        let blackboard = Blackboard::default();
        let full = StoredConversation::full(
            "sess-roundtrip",
            messages,
            &lif,
            Some(1_700_000_000.0),
            &blackboard,
        );
        // 0bf ③：负担档位元数据随侧车往返（已投递档位去重）。
        let mut full = full;
        full.burden_tiers_notified = vec![orz_loop::complexity::BURDEN_TIER_ORDER[0].to_string()];
        // v7（S1 修订批，设计 §3.5.1，DP-16）：会话级刻度水位同信封往返
        // （每级每会话一次；跨 prompt 不重发、新会话从零开始）。
        full.context_scale_notified = vec!["500k".to_string(), "900k".to_string()];
        persist_conversation_sidecar(&base, &full);
        let stored = load_conversation_sidecar(&base, "sess-roundtrip").expect("sidecar loads");
        assert_eq!(stored.session_id, "sess-roundtrip");
        assert_eq!(stored.messages.len(), 3);
        let restored = stored.temporal_spikes.expect("spikes roundtrip");
        assert_eq!(restored.len(), 2);
        assert_eq!(restored[1].t, 340.0);
        assert_eq!(restored[1].domain, orz_assurance::lif::Domain::Stuck);
        assert_eq!(stored.session_started_at, Some(1_700_000_000.0));
        let restored_lif = stored.lif.expect("lif snapshot roundtrip");
        assert_eq!(restored_lif.round, 41);
        assert_eq!(
            restored_lif.current_domain,
            orz_assurance::lif::Domain::Stuck
        );
        assert_eq!(restored_lif.entry_round, 33);
        assert!(stored.blackboard.is_some(), "blackboard rides the envelope");
        assert_eq!(stored.messages[0].content, "中文问题");
        assert_eq!(
            stored.messages[1].reasoning_content.as_deref(),
            Some("推理过程")
        );
        assert_eq!(stored.messages[2].tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(
            stored.burden_tiers_notified,
            vec![orz_loop::complexity::BURDEN_TIER_ORDER[0].to_string()]
        );
        assert_eq!(
            stored.context_scale_notified,
            vec!["500k".to_string(), "900k".to_string()],
            "会话级刻度水位随侧车往返"
        );
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

    /// P2-10 F2 §3.5 (I4): a legacy sidecar without `temporal_spikes` still
    /// parses (`serde(default)`) — zero migration, old sessions restore
    /// their conversation and simply start with an empty spike timeline.
    #[test]
    fn conversation_sidecar_legacy_without_spikes_parses() {
        let base = test_dir();
        let path = conv_sidecar_path(&base, "sess-legacy");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"{
                "schema_version": "0.1.0-draft",
                "session_id": "sess-legacy",
                "messages": [{
                    "role": "user",
                    "content": "hi",
                    "tool_call_id": null,
                    "tool_calls": [],
                    "reasoning_content": null
                }]
            }"#,
        )
        .unwrap();
        let stored = load_conversation_sidecar(&base, "sess-legacy").expect("legacy parses");
        assert_eq!(stored.messages.len(), 1);
        assert!(stored.temporal_spikes.is_none());
        // v7（S1 修订批）：缺字段的 legacy 侧车按空水位解析（serde default）
        // ——新字段不破坏既有侧车。
        assert!(stored.context_scale_notified.is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_conversation_not_persisted() {
        let base = test_dir();
        let empty = StoredConversation::empty("sess-empty", Some(1_700_000_000.0));
        persist_conversation_sidecar(&base, &empty);
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
                    // plan 门。0bi ⑩（2026-09-23）：纯文本短答跳过反例门
                    // ——每个 prompt 一轮模型调用，本轮文本即终答。
                    ScriptedResponse::text("第一答"),
                    ScriptedResponse::text("第二答"),
                ]));
                let server = shadow_server_with_gateway(fake.clone());
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
                // v7（S1 修订批，设计 §3.5.1，DP-16）：把会话级刻度水位预置进
                // **内存续接包**（prompt 2 从这里取态，等同侧车恢复路径）——
                // prompt 2 起始注入 loop、成功后由 controller 回写；下方断言
                // 证明「注入 → 回写」整条接线（任一环断掉都会把水位丢成空）。
                {
                    let mut sessions = server
                        .sessions
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    let session = sessions.get_mut("sess-conv").expect("session");
                    let continuation = session.continuation.as_mut().expect("continuation");
                    continuation.context_scale_notified = vec!["500k".to_string()];
                }
                let r2 = server
                    .handle_session_prompt("sess-conv", "第二问")
                    .await
                    .unwrap();
                assert_eq!(r2["response"], "第二答");

                // The second prompt's first model request opened with the
                // first turn (one model call per prompt — 0bi ⑩ skips the
                // counterexample gate for text-only short answers; the plan
                // gate is removed universally).
                let reqs = fake.received_requests();
                assert_eq!(reqs.len(), 2, "one model call per prompt");
                let msgs = &reqs[1].messages;
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
                // v7（S1 修订批，设计 §3.5.1，DP-16）：会话级刻度水位在 ACP
                // 路径上「prompt 起始注入 loop → run 成功后回写侧车」——
                // 预置值原样往返即证明注入/回写接线成立（本会话未越刻度，
                // 故不新增水位键）。
                assert_eq!(
                    stored.context_scale_notified,
                    vec!["500k".to_string()],
                    "v7 DP-16：预置会话级水位必须原样往返（注入 loop → 成功回写）"
                );

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

    /// P2-13 B1 会话化基础 (2026-09-03)：会话级黑板与 LIF 会话轴跨
    /// prompt 续接——两个 prompt 各执行一次 `read_file`（各 = 一个决策
    /// 轮）。断言：第二 prompt 的黑板 exec 行 round 从 1 续到 2（不回归
    /// 1）、行带 LIF 域章、LIF 快照 round=2；成功 run 把黑板 + 快照随
    /// 对话一起写回侧车（依赖图 live-only 不落侧车）。
    #[tokio::test]
    async fn cross_prompt_blackboard_and_lif_axis_continue() {
        tokio::task::LocalSet::new()
            .run_until(async {
                let base = test_dir();
                let target = base.join("sample.txt");
                std::fs::write(&target, "wired file content").unwrap();

                let mut script = Vec::new();
                for marker in ["p1", "p2"] {
                    script.push(ScriptedResponse::tool_calls(vec![ToolCall {
                        name: "read_file".to_string(),
                        arguments: serde_json::json!({ "target_file": target }),
                        call_id: format!("call-{marker}"),
                    }]));
                    script.push(ScriptedResponse::text(format!("{marker} 完成")));
                    script.push(ScriptedResponse::text(format!("{marker} 完成")));
                    // 0x S1：首个动作批次结束后的开局问询回答轮（软门消费）。
                    script.push(ScriptedResponse::text(format!("{marker} 完成")));
                }
                let fake = Arc::new(FakeProvider::new(script));
                let server = shadow_server_with_gateway(fake.clone());
                // Interactive shape — a dead receiver still lets the
                // low-risk read auto-allow (same as the bridge test).
                server.set_gateway(dead_gateway());
                server
                    .handle_session_new(
                        "sess-bb",
                        Some(base.clone()),
                        crate::session::TrustPolicy::Skip,
                    )
                    .await
                    .unwrap();
                server
                    .handle_session_prompt("sess-bb", "第一问")
                    .await
                    .unwrap();
                server
                    .handle_session_prompt("sess-bb", "第二问")
                    .await
                    .unwrap();

                let stored = load_conversation_sidecar(&base, "sess-bb").expect("sidecar");
                let bb = stored.blackboard.expect("blackboard rides the envelope");
                assert_eq!(
                    bb.exec.results.len(),
                    2,
                    "two stamped exec rows across prompts"
                );
                assert_eq!(bb.exec.results[0].round, 1, "prompt-1 decision round");
                assert_eq!(bb.exec.results[1].round, 2, "round continues, not resets");
                assert!(
                    bb.exec.results[0].domain.is_some() && bb.exec.results[1].domain.is_some(),
                    "exec rows carry the LIF domain stamp"
                );
                assert_eq!(
                    bb.tool_actions.len(),
                    2,
                    "tool-action partition also persists across prompts"
                );
                assert!(
                    bb.tool_actions
                        .iter()
                        .all(|ta| ta.round > 0 && ta.domain.is_some()),
                    "tool-action rows stamped"
                );

                // LIF 会话轴快照：决策轮计数精确续接（不再只有 spike 近似）。
                let lif = stored.lif.expect("lif session snapshot rides the envelope");
                assert_eq!(lif.round, 2);
                assert!(lif.has_success, "read_file success observed");

                // 依赖图 live-only：侧车 JSON 不携带 dep_graph。
                let raw = std::fs::read_to_string(conv_sidecar_path(&base, "sess-bb")).unwrap();
                assert!(!raw.contains("\"dep_graph\""), "dep graph not persisted");

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
                // Process 1: one successful prompt lands the sidecar
                // (one model round per prompt — 0bi ⑩ skips the
                // counterexample gate for text-only short answers).
                let fake1 = Arc::new(FakeProvider::new(vec![ScriptedResponse::text("第一答")]));
                let server1 = shadow_server_with_gateway(fake1.clone());
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
                let fake2 = Arc::new(FakeProvider::new(vec![ScriptedResponse::text("第二答")]));
                let server2 = shadow_server_with_gateway(fake2.clone());
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
                assert_eq!(reqs.len(), 1, "one model call per prompt");
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
                // One successful prompt consumes the single scripted reply
                // (one model round per prompt — 0bi ⑩ skips the
                // counterexample gate for text-only short answers); the
                // second prompt hits an empty script → model failure.
                let server = shadow_server_with_gateway(Arc::new(FakeProvider::new(vec![
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
                let server = shadow_server_with_gateway(fake.clone());
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
