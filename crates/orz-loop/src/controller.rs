//! AgentLoopController — the main agent loop.
//!
//! Replaces Grok Sampler's ~10k lines with a clean while loop.
//!
//! Flow (event sequence aligned with Python `orientation_runtime_journal.py`):
//!   1. tool_availability_check (work-tool two-state probe partition —
//!      complete/incomplete with neutral reasons; the retrieval lane and
//!      retrieval lane never appear — must precede run_started per Python
//!      conformance)
//!   2. run_started / prompt_submitted
//!   3. orientation_checkpoint (per-turn, fixed_step_interval — Python parity,
//!      no cooldown; see orz-assurance::orientation)
//!   4. model ↔ tool loop:
//!      - model_output (text + tool_calls)
//!      - retrieval-shaped calls route to subagents (internal/external)
//!      - host calls pass ToolDispatcher gates: IPG (IP3a) → permission →
//!        tool execution
//!   5. run_finished (terminal)
//!
//! Phase 2 (2026-08-04): single main agent + two retrieval subagents.
//! Pro/Flash dual-model dispatch was archived (see
//! INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §4.5).

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use orz_assurance::acaf::TicketKind;
use orz_assurance::{
    EventTrack, EventType, JournalRecorder, JournalRecorderError, Redaction, RunEvent,
    canonical_json, seal_event, sha256_hex,
};

use orz_assurance::session::snapshot::SnapshotStore;

use crate::agent_loop::{
    LoopOutcome, LoopProfile, SharedLoopServices, run_agent_loop, run_template_compact,
};
use crate::agents::MainAgent;
use crate::blackboard::{EditRecord, ExternalRetSection, InternalRetSection, SharedBlackboard};
use crate::console::{ServiceRegistry, TraceStore};
use crate::gateway::fake::FakeProvider;
use crate::gateway::model::{Message, ModelGateway, Role, ToolCall};
use crate::host::{LoopHost, PolicyDenial, PolicyDenialSource, ToolDef, ToolResult};
use crate::orientation::{AgentRole, OrientationFireRecord, OrientationSessionState};
use crate::prompt::{is_injected_block_text, is_restore_retained_block};
use crate::retrieval::activation::{ActivationRegistry, StoredActivation};
use crate::retrieval::evidence::EvidenceRecord;

/// Cap on model↔tool rounds per turn (anti-runaway backstop).
///
/// D-8 (FIX_PLAN 2026-08-06, P7/LOOP-14): 8 → 40, decided by ADR-0008.
/// Budget history: 8 was the Grok ecosystem default (mcp-grok maxTurns=8,
/// recorded inaccurately at first as a Python port — LOOP-14 cross-check:
/// Python uses max_turns=20/max_tool_calls=0); raised to 40 by ADR-0008
/// (2026-08-07); **frozen at 120 by ADR-0010 v1.1 (2026-08-09)** — the main
/// agent and both retrieval subagents each carry a 120-tool-round budget,
/// counted independently per session (FUS-BUDGET). The model is told the
/// budget explicitly (static session block) and can read the live remaining
/// count on demand via `blackboard_read section=session` (PUSH→PULL
/// 2026-08-21 — the per-round mechanical re-declaration is retired; the
/// mechanical hard gates stay fail-closed). Anti-runaway protection is
/// layered: the global round budget is the backstop, the consecutive-denial
/// circuit breaker (IP2a/D-3) is the primary control. ADR-0008's remaining
/// semantics (deny rounds count, session/exhaustion blocks) stay unchanged.
pub const MAX_TOOL_ROUNDS: u32 = 120;

/// Env override for the global round budget (benchmark harnesses — SWE-bench
/// exploration burns 60+ rounds; polyglot stays at the default). Parsed at
/// controller construction; the session-declared budget block follows it, so
/// the model always sees the real cap. Default (absent/invalid) = 120.
pub fn max_tool_rounds_override() -> Option<u32> {
    std::env::var("ORZ_MAX_TOOL_ROUNDS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, BACKLOG 0k / TODO
/// P0-0k 第一批第 2 项)：检索子代理 run 级预算——墙钟默认 600s（10 分钟），
/// `ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` 覆盖，0 禁用（unbounded）。
/// 主车道 wallclock（ORZ_MAX_WALLCLOCK）仍由 orz-bin 层整体兜底；本预算
/// 只约束单次检索派发，防止一个检索会话烧掉主 run 的墙钟。
pub const RETRIEVAL_SUBAGENT_WALLCLOCK_DEFAULT_SECS: u64 = 600;

/// 检索子代理独立工具轮上限（默认 60；主车道仍为 `MAX_TOOL_ROUNDS`=120）。
/// 与 `max_tool_rounds` 取 min 生效（测试用 `with_max_tool_rounds` 缩小
/// 时语义不变）。
pub const RETRIEVAL_SUBAGENT_MAX_TOOL_ROUNDS: u32 = 60;

/// Parse rule for the subagent wallclock env value (tested without env
/// mutation): trimmed u64 seconds; `0` disables; non-numeric → None
/// (invalid ignored, same convention as `max_tool_rounds_override`).
pub(crate) fn parse_retrieval_subagent_wallclock(s: &str) -> Option<Option<std::time::Duration>> {
    match s.trim().parse::<u64>() {
        Ok(0) => Some(None),
        Ok(secs) => Some(Some(std::time::Duration::from_secs(secs))),
        Err(_) => None,
    }
}

pub fn retrieval_subagent_wallclock_override() -> Option<Option<std::time::Duration>> {
    std::env::var("ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS")
        .ok()
        .and_then(|s| parse_retrieval_subagent_wallclock(&s))
}

pub(crate) fn parse_retrieval_subagent_max_tool_rounds(s: &str) -> Option<u32> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

pub fn retrieval_subagent_max_tool_rounds_override() -> Option<u32> {
    std::env::var("ORZ_RETRIEVAL_MAX_TOOL_ROUNDS")
        .ok()
        .and_then(|s| parse_retrieval_subagent_max_tool_rounds(&s))
}

/// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): per-activation web_fetch
/// candidate cap — the mechanical hard gate replacing the prompt's soft
/// "候选 ≤5" rule (design §1.1). User adjudication 2026-08-14: default 8.
/// The count domain is per activation (deduplicated by exact URL string);
/// the same cap applies to every activation and is shared by web_fetch
/// and browser_read (P0-B step 4).
pub const DEFAULT_WEB_FETCH_CANDIDATE_CAP: u32 = 8;

/// Env override for the candidate cap (`ORZ_WEB_FETCH_CANDIDATE_CAP`).
/// Parsed at controller construction; absent/invalid/zero = the default.
pub fn web_fetch_candidate_cap_override() -> Option<u32> {
    std::env::var("ORZ_WEB_FETCH_CANDIDATE_CAP")
        .ok()
        .and_then(|s| parse_web_fetch_candidate_cap(&s))
}

/// Pure parse rule for the candidate cap env value (tested without env
/// mutation): trimmed, positive integer; absent/invalid/zero → None.
pub(crate) fn parse_web_fetch_candidate_cap(s: &str) -> Option<u32> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): the per-model-round
/// tool-result injection budget (estimated tokens, chars/2) — default 50K.
pub const DEFAULT_MAX_INJECT_TOKENS_PER_ROUND: u64 = 50_000;

/// Env override for the per-round injection budget
/// (`ORZ_MAX_INJECT_TOKENS_PER_ROUND`). Parsed at controller construction;
/// absent/invalid/zero = the default.
pub fn max_inject_tokens_per_round_override() -> Option<u64> {
    std::env::var("ORZ_MAX_INJECT_TOKENS_PER_ROUND")
        .ok()
        .and_then(|s| parse_max_inject_tokens_per_round(&s))
}

/// Pure parse rule for the injection-budget env value (tested without env
/// mutation): trimmed, positive integer; absent/invalid/zero → None.
pub(crate) fn parse_max_inject_tokens_per_round(s: &str) -> Option<u64> {
    s.trim().parse().ok().filter(|v| *v > 0)
}

/// THIN-HARNESS-REDESIGN R2a (2026-08-27, §4.4): 检索派发结果回传通道——
/// `blackboard`（默认）= 主代理工具结果只回指针摘要（全文保留在
/// internal_ret / external_ret 黑板分区与 journal，留痕不变），需要时模型
/// 用 `blackboard_read section=internal_ret|external_ret` 按需拉取；
/// `inline` = 保留旧行为（子代理全文回传主对话），作 A/B 与回退通道，两个
/// 施工轮后若零命中则物理删除 parse 路径及其测试（§4.4）。env
/// `ORZ_RETRIEVAL_RESULT_CHANNEL` 可覆盖；缺失/无效值回退 `blackboard`
/// （fail-safe——指针摘要永不比全文更膨胀）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetrievalResultChannel {
    /// 指针摘要回传（默认，PUSH → PULL）。
    Blackboard,
    /// 全文回传（旧行为，A/B 与回退）。
    Inline,
}

/// Environment override for the retrieval result channel
/// (`ORZ_RETRIEVAL_RESULT_CHANNEL=blackboard|inline`).
pub(crate) const RETRIEVAL_RESULT_CHANNEL_ENV: &str = "ORZ_RETRIEVAL_RESULT_CHANNEL";

/// Resolve the retrieval result channel from env (missing/invalid →
/// `Blackboard`). Pure function of process env, following the
/// `ORZ_ORIENTATION_THRESHOLD` / `ORZ_THINKING_MODE` override pattern.
pub(crate) fn retrieval_result_channel_from_env() -> RetrievalResultChannel {
    match std::env::var(RETRIEVAL_RESULT_CHANNEL_ENV)
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("inline") => RetrievalResultChannel::Inline,
        _ => RetrievalResultChannel::Blackboard,
    }
}

/// Streaming pacing (Phase 3 slice #6): a round's `model_output` (journaled,
/// fsync-acked) must be projected by a live client before the next round's
/// first text delta arrives (deltas travel in-memory at arrival rate). The
/// TUI's journal path has TWO 50ms stages — the tail thread polls the file
/// (journal_tail.rs) and the runner drains the channel on its own 50ms tick
/// (runner.rs) — so worst-case projection is ~100ms after fsync. Two ticks
/// (120ms) covers that alignment; a render stall beyond 10ms could still
/// theoretically race (recorded boundary, 2026-08-05 review).
pub const TEXT_DELTA_PACING: std::time::Duration = std::time::Duration::from_millis(120);

pub use crate::compact::ContextCompactConfig;
/// N4 (2026-08-30): 类型/纯函数归位——以下项已在目标模块定义，此处仅保留
/// 兼容重导出（外部调用面 `orz_loop::controller::*` 与文件内测试不变）：
/// 模式面 → `retrieval/mode.rs`；压缩/消息预算 → `compact.rs`；
/// denial 状态机 → `denial.rs`。
pub use crate::compact::{
    DEFAULT_FOLD_TAIL_TOKENS, DEFAULT_FOLD_TRIGGER_TOKENS, DEFAULT_WHITELIST_CAP,
    fold_tail_tokens_override, fold_trigger_tokens_override,
};
pub(crate) use crate::compact::{
    compact_messages, estimate_message_tokens, estimate_messages_tokens,
};
pub use crate::denial::DENIAL_BREAKER_CONSECUTIVE;
pub(crate) use crate::denial::{DenialKey, DenialState, PolicyFeedback};
pub use crate::retrieval::mode::{RetrievalCapability, RetrievalMode};

/// Error during agent loop execution.
#[derive(Debug, thiserror::Error)]
pub enum AgentLoopError {
    #[error("journal error: {0}")]
    Journal(#[from] JournalRecorderError),
    #[error("model error: {0}")]
    Model(String),
    #[error("session error: {0}")]
    Session(String),
    #[error("assurance invariant: {0}")]
    Assurance(String),
    /// The run was cancelled by the client (ACP `session/cancel`). The
    /// controller records a terminal `run_cancelled` event instead of
    /// `run_failed` (Phase 3 slice #7).
    #[error("run cancelled by user")]
    Cancelled,
    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33): 会话级连续
    /// 退化中断达到 `DEGENERATION_LIMIT`——run 层记 `run_invalidated`
    /// （reason=degeneration）而非 run_failed。
    #[error("degeneration limit reached: {0}")]
    Degeneration(String),
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30): 检索子代理单次
    /// 派发墙钟预算耗尽——父 run 继续执行，激活以 `subagent_timeout` 收口
    /// （resumable，无 assessment/digest）。
    #[error("retrieval subagent wallclock exceeded")]
    RetrievalSubagentTimeout,
}

/// ACAF Slice 2 fail-closed (2026-08-13): the ticket lifecycle's decision
/// for one control event or external-effect action. Shadow mode always
/// returns `Proceed` (the ledger IS the journal events); fail-closed
/// returns `Blocked` on every rejection path (D-14/D-15/D-16) — the caller
/// must not execute the event/action, and the refusal has already been
/// journaled as `control_ticket_rejected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TicketGate {
    Proceed,
    Blocked {
        code: orz_assurance::acaf::RejectCode,
        detail: String,
    },
}

/// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the candidate gate's
/// decision — allowed (the URL is committed by the caller at the execution
/// boundary; the post-call count/cap rides the `tool_completed` feedback)
/// or refused (event + tool message already journaled by the gate). Covers
/// the web_fetch family and `browser_read` (same per-activation count
/// domain, design §1.1/§1.3).
pub(crate) enum CandidateGateDecision {
    /// Decision only — the URL is NOT consumed here; the caller commits it
    /// at the execution boundary after the permission/ACAF gates pass
    /// (review fix 2026-08-14 — a later-gate refusal consumes no budget).
    Allowed {
        url: String,
        cap: usize,
    },
    Refused(ToolResult, Option<PolicyFeedback>),
}

/// Stable reason-code prefix for a candidate-counted tool: `web_fetch`
/// family keeps its step-2 codes, `browser_read` gets its own
/// `browser_read_candidate_*` family (P0-B step 4 — same count domain,
/// distinct audit codes).
pub(crate) fn candidate_tool_prefix(tool: &str) -> &'static str {
    if crate::relay::is_web_fetch_tool(tool) {
        "web_fetch"
    } else {
        "browser_read"
    }
}

/// FUS-RETRIEVAL-MECH P0-B step 2/4 review fix (2026-08-14): commit one
/// candidate URL to the shared per-activation counter at the execution
/// boundary — after the permission/ACAF gates passed and immediately
/// before ToolStarted. Exact-string dedup (the same URL re-read consumes
/// nothing); returns the post-commit (count, cap).
pub(crate) fn commit_candidate(
    counter: &Mutex<Vec<String>>,
    url: &str,
    cap: usize,
) -> (usize, usize) {
    let mut seen = counter.lock().unwrap();
    if !seen.iter().any(|u| u == url) {
        seen.push(url.to_string());
    }
    (seen.len(), cap)
}

/// The main agent loop controller.
///
/// Owns the prompt processing lifecycle. Stateless between turns —
/// all persistent state lives in the Blackboard and Journal.
/// Grill-mode turn inputs (2026-08-08 write-placement slice, design §3):
/// the session's accumulated message history, the user's answer to the
/// previous question, and the protocol template (injected once, at session
/// start). Grill turns reuse the full model↔tool loop with a discard
/// `EventWriter`; the conversation is recorded by the host to
/// `{cwd}/.gsa/grill/<session>.jsonl`, never into a run journal.
pub struct GrillTurn<'a> {
    pub history: &'a mut Vec<Message>,
    pub user_input: &'a str,
    pub template: Option<&'a str>,
}

/// 当前文件的实际内容锚点（FUS-READ-ANCHOR-WRITE-GUARD 拒单信封 upstream
/// 用；快筛路径 `sha256` 为 `None`，哈希路径计算后填充）。
pub(crate) struct ActualAnchor {
    pub(crate) size: Option<u64>,
    pub(crate) mtime: Option<u64>,
    pub(crate) sha256: Option<String>,
}

pub struct AgentLoopController {
    /// GAP-SUBAGENT-RUNTIME (2026-08-10): `pub(crate)` — the shared
    /// `run_agent_loop` (agent_loop.rs) drives the model round through it.
    pub(crate) main_agent: MainAgent,
    pub(crate) blackboard: Arc<SharedBlackboard>,
    pub(crate) max_tool_rounds: u32,
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：检索子代理单次
    /// 派发墙钟预算。`None` = 禁用（unbounded）。
    pub(crate) retrieval_subagent_wallclock: Option<std::time::Duration>,
    /// 检索子代理工具轮上限（与 `max_tool_rounds` 取 min 生效）。
    pub(crate) retrieval_max_tool_rounds: u32,
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): per-activation
    /// candidate cap (ORZ_WEB_FETCH_CANDIDATE_CAP, default 8 — user
    /// adjudication 2026-08-14). Shared by web_fetch and browser_read.
    /// Settable for tests.
    pub(crate) candidate_cap: u32,
    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): per-model-round
    /// tool-result injection budget (estimated tokens, chars/2).
    /// `ORZ_MAX_INJECT_TOKENS_PER_ROUND`, default 50K. Settable for tests.
    pub(crate) max_inject_tokens_per_round: u64,
    /// IP5 pre-mutation snapshot store (session-scoped). `None` disables
    /// snapshotting (tests / hosts that opted out).
    pub(crate) snapshot_store: Option<Arc<SnapshotStore>>,
    /// v1.15 (2026-08-14): plan-epoch archive directory
    /// (`<session_cwd>/.gsa/blackboard`). `Some` enables epoch snapshot
    /// persistence on rotation, restore-from-archive on construction and
    /// cross-epoch `blackboard_read`; `None` keeps epochs purely
    /// in-memory (tests / hosts without a session cwd).
    pub(crate) blackboard_archive_dir: Option<PathBuf>,
    /// F7 (2026-08-15, BACKLOG 6e 复查遗留): epoch archive writes that
    /// failed during plan ingest (builder phase — no journal writer yet).
    /// Flushed as `epoch_archive_write_failed` v0.2 events at run start so
    /// the loss of an old epoch's durable snapshot leaves an audit trace.
    epoch_archive_errors: Mutex<Vec<(u64, EpochArchiveWriteKind)>>,
    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): whether the
    /// first-round plan gate is active for main-lane sessions. Production
    /// wiring (ACP server / CLI run) enables it; tests keep the legacy
    /// tool surface by default.
    pub(crate) plan_first_enabled: bool,
    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): whether this
    /// session has already completed its first-round plan gate (session-
    /// level, not run-level — the gate fires once per session; restored
    /// sessions with history are marked done). Runs in the same session
    /// after the first one skip the gate.
    plan_first_session_done: bool,
    /// Monotonic model-round counter across turns (streaming pacing guard).
    /// Kept on the controller (not per-turn) so a turn ≥ 2's FIRST round is
    /// also paced: a programmatic stdio client issuing prompt #2 immediately
    /// after response #1 could otherwise race its first deltas against the
    /// previous turn's final `model_output` through the 50ms tail (review
    /// P3-5, 2026-08-05). User-paced TUIs are naturally safe (turn gaps
    /// ≫ 50ms) — this covers automated clients.
    pub(crate) pacing_rounds: std::sync::atomic::AtomicU32,
    /// IP2a denial circuit breaker (D-3, FIX_PLAN 2026-08-06): consecutive
    /// policy denials in the current run. Mutex since `run_turn_inner` is
    /// `&self` and a turn may run on any thread; reset at turn start.
    pub(crate) denial_state: Mutex<DenialState>,
    /// A6 explicit context compaction parameters (settleable for tests).
    pub(crate) context_compact: ContextCompactConfig,
    /// A6 §8 C.2 compaction whitelist (user decision 2026-08-08): the
    /// model-written list of task facts that survive compaction. Written
    /// only during the FIRST tool batch; resident in the conversation's
    /// preamble zone (the compaction mechanism skips it); archived
    /// best-effort to `{journal_dir}/whitelist.jsonl` (A5 retention
    /// covers it via the run dir).
    pub(crate) whitelist: Mutex<Vec<String>>,
    /// Cumulative character cap for the whitelist (16K default —
    /// user decision; the whitelist must stay a small part of the ~90K
    /// compacted context).
    pub(crate) whitelist_cap: usize,
    /// GAP-SUBAGENT-RUNTIME (2026-08-10): per-role retrieval activation
    /// registry — the subagent lifecycle state (ADR-0010 §3.3). The
    /// controller is the single writer (disposition/close commits are
    /// serialized through it; §4.4). Process/turn-scoped: ACP builds a
    /// controller per prompt, so activations do not survive into the
    /// next prompt (registered boundary — cross-turn persistence is a
    /// later slice).
    pub(crate) activations: Mutex<ActivationRegistry>,
    /// M5 (2026-08-10): Diagnostic Coverage episode state (ADR-0010
    /// §4.6) — main lane; one episode per run (registered boundary:
    /// cross-prompt episodes are not persisted).
    pub(crate) dc_state: Mutex<crate::diagnostic_coverage::DebugEpisodeState>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): ADR-0010 §3.7.1 explicit retrieval
    /// mode — session/task-contract level. `off` is the default; a session
    /// bootstrap transition (session/new with an explicit mode) journals one
    /// `retrieval_mode_transition` on the first run that sees it.
    pub(crate) retrieval_mode: RetrievalMode,
    /// M4 (review 2026-08-10): the mode the session had BEFORE a pending
    /// bootstrap transition — the transition journal uses it as the real
    /// `old_mode` (ADR-0010 §3.7.1 — every transition carries old/new; a
    /// hardcoded "off" would misstate a mode change away from an
    /// already-enabled mode). `None` = the session default (off).
    pub(crate) previous_retrieval_mode: Option<RetrievalMode>,
    /// Capability probe result for `retrieval_mode` — never a silent
    /// fallback: Unsupported/Degraded carry the reason.
    pub(crate) retrieval_capability: RetrievalCapability,
    /// Session bootstrap carried an explicit mode selection — journal the
    /// transition on the next run's startup sequence, then clear. Atomic
    /// because the run path holds only `&self`.
    pub(crate) bootstrap_transition_pending: std::sync::atomic::AtomicBool,
    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：模式 A
    /// 自动降级 transition 的机械元数据（authority, reason_code）——
    /// local_browser probe 失败降级 framework_fallback 时
    /// (mechanical_probe, browser_launch_failed)；显式选择路径为 None
    /// （沿用 session_bootstrap/session_default）。自有字符串以支持从
    /// ACP 侧车跨 run 恢复（审查处理：降级后 run 在 journal 前失败时
    /// 下轮重试不丢失机械元数据）。
    pub(crate) transition_authority: Option<(String, String)>,
    /// The owning session id (for the transition payload; `None` in bare
    /// test controllers).
    pub(crate) session_id: Option<String>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): tool-call evidence collected from
    /// the retrieval lane's host calls — the MECHANICAL source of the
    /// structured result's ledger (ADR-0010 §3.7.4). Cleared at each
    /// dispatch start; consumed at result formation.
    pub(crate) evidence: Mutex<Vec<EvidenceRecord>>,
    /// FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the MAIN lane's own
    /// tool-call evidence (read_file / project_doc_index etc.) — the
    /// observation-time source for final-answer `[来源: 路径:行号]` markers
    /// (ADR-0010 §3.7.9). Per-run: cleared at run_turn_inner start; never
    /// cleared by a retrieval dispatch (separate from `evidence`).
    pub(crate) main_evidence: Mutex<Vec<EvidenceRecord>>,
    /// FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the `source_ledger`
    /// arrays committed by retrieval subagents in THIS run — the binding
    /// authority for final-answer `[来源: source_id]` / URL / document
    /// identity markers. Per-run: cleared at run_turn_inner start.
    pub(crate) run_source_ledgers: Mutex<Vec<serde_json::Value>>,
    /// FUS-RETRIEVAL-MECH P0-B step 5 review fix (2026-08-14): run-unique
    /// source_id allocation — the final-answer verifier binds `SRC-###` to
    /// THIS run's committed ledgers, and per-ledger renumbering would make
    /// `SRC-001` ambiguous across multiple committed results in one run.
    /// The counter is cleared at run start and consumed by
    /// `retrieval::evidence::build_structured_result` at each commit.
    pub(crate) next_source_seq: Mutex<u32>,
    /// GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 —
    /// mechanical source tier judge config (embedded seed lists by default;
    /// `ORZ_SOURCE_WEIGHTING_CONFIG` overrides at runtime). Quality layer
    /// only — never an authorization gate.
    pub(crate) source_weighting: orz_assurance::source_weighting::SourceWeightConfig,
    /// FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): mechanical candidate
    /// prefilter config (embedded seed lists by default;
    /// `ORZ_CANDIDATE_PREFILTER_CONFIG` overrides at runtime). Purifies and
    /// sorts the web_search citation pool — quality layer, never an
    /// authorization gate and never an interception of the model's choice.
    pub(crate) candidate_prefilter: orz_assurance::candidate_prefilter::CandidatePrefilterConfig,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): activations restored from the
    /// sidecar at build time — journaled as `retrieval_activation_restored`
    /// once at the next run's startup (per activation per prompt).
    pub(crate) restored_activations: Mutex<Vec<StoredActivation>>,
    /// ACAF Slice 1 (ADR-0011 §4.4): the host-side signer client for
    /// control-event tickets. `None` = ACAF disabled (zero behaviour change).
    /// Shadow mode: issuance/verification failures are journaled
    /// (`control_ticket_rejected`) but the control event still proceeds.
    pub(crate) acaf: Option<Arc<tokio::sync::Mutex<crate::acaf::AcafClient>>>,
    /// ACAF Slice 2 fail-closed switch (D-9 → full Slice 2 milestone,
    /// 2026-08-13): when true, every ticket failure REFUSES the control
    /// event / external-effect action instead of the shadow-mode
    /// journal-and-proceed. D-14/D-15/D-16 semantics apply only here; the
    /// shadow ledger keeps its registered silent-skip behavior.
    pub(crate) acaf_fail_closed: bool,
    /// ACAF (Slice 1 + goal wiring 2026-08-12): the current task-goal
    /// binding — digest (set at run start from the prompt; check 4) plus the
    /// revision counter. `digest: None` = no goal seen yet (control events
    /// stay unticketed until one exists). An accepted `continue` consumes a
    /// GoalRevisionV1 ticket under the OLD context, then
    /// [`AgentLoopController::update_goal`] swaps the digest and bumps the
    /// version — the next ticket's `ensure_initialized` re-derives
    /// `K_session` (ADR-0011 决策 5: goal change → old tickets die).
    pub(crate) goal_context: Mutex<GoalContext>,
    /// GAP-DENIAL-POLICY-REVISION wiring (2026-08-12): live policy revision
    /// for the ACAF binding (HKDF input + check 4) and the denial breaker
    /// key (ADR-0010 §3.5.4 — a revision change resets the consecutive
    /// count). Per-run reset to 0 alongside `denial_state`. The first
    /// production increment source is Slice 3's ModeChangeTicket
    /// (ADR-0011 决策 9) — the mechanism is wired and test-covered today.
    pub(crate) policy_revision: std::sync::atomic::AtomicU64,
    /// FUS-TOOL-PROBE P0-A/P0-A-2 (2026-08-13): the minimal previous-round
    /// map — work tool `tool → complete/incomplete`, no reasons cached
    /// (design §8) — the flip comparator for `tool_availability_check`
    /// events. Seeded by the pre-run_started probe and reset per run;
    /// never persisted across runs.
    probe_state: Mutex<crate::tool_probe::MinimalProbeMap>,
    /// P0-C orz 内嵌集成 S2 (2026-08-15): 操作台注册表 —— 动作名 → 契约 →
    /// 目标工具的确定性路由（注册时校验并缓存 schema）。生产基础动作集
    /// 由 `console::default_service_registry()` 提供。
    pub(crate) console_registry: ServiceRegistry,
    /// P0-C S2: 操作台执行 trace 存储（有界 50 条；发放收口 commit）。
    pub(crate) console_traces: Mutex<TraceStore>,
    /// P0-C S2: 动作栏订单号机械分配（`ORD-<seq>`，单调）。
    pub(crate) console_order_seq: std::sync::atomic::AtomicU64,
    /// PLAN-FIRST 阶段 B (2026-08-16): 主车道最近一轮探针源（ToolPolicy +
    /// ToolProbeSnapshot）——工具栏投影与注册板块（黑板模型栏）共用的单一
    /// 事实源；`blackboard_read section=actions` 读取时据此派生注册板块。
    /// 仅内存、随轮覆盖、run 起始复位、不持久化（沿用探针快照生命周期
    /// 纪律）。
    pub(crate) console_probe_source: Mutex<
        Option<(
            crate::host::ToolPolicy,
            crate::tool_probe::ToolProbeSnapshot,
        )>,
    >,
    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 双模式（console
    /// 默认 + direct 受控降级）的模型面开关。生产接线（ACP server / CLI
    /// run）随 plan_first 一并开启；测试保持既有工具面。
    pub(crate) console_default_enabled: bool,
    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): run 级双模式
    /// 状态（模式/故障连败/询问标记/transition_id/direct 证据面）。run
    /// 起始复位；plan epoch 轮换/黑板旋转不清除。
    pub(crate) console_mode_state: Mutex<crate::console_mode::ConsoleModeState>,
    /// 2026-08-18 (ADR-0010 §14.25 项 1): 上次追加到消息面的
    /// `[任务状态]` 文本——状态行只在变化时作为尾随用户消息追加
    /// （前缀缓存纪律，system 提示词保持完全静态）；无计划为 None。
    status_line_appended: Mutex<Option<String>>,
    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the worktree metadata
    /// baseline captured at plan approval — the mechanical delivery status
    /// diffs the live snapshot against it at `submit`. `None` = the host
    /// provides no baseline (mock hosts / unsupported) → the status reports
    /// the change list as unavailable rather than fabricating one.
    pub(crate) delivery_baseline: Mutex<Option<std::collections::HashMap<String, (u64, u64, u32)>>>,
    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): two-phase `submit` —
    /// (plan_epoch, pending). Call 1 renders the delivery status into the
    /// plan view and sets pending=true; call 2 (same epoch) confirms and
    /// marks the terminal step done. Keyed by plan epoch so a plan rotation
    /// invalidates any stale pending confirmation.
    pub(crate) delivery_pending: Mutex<(u64, bool)>,
}

/// F7 (2026-08-15, BACKLOG 6e 复查遗留): which epoch snapshot failed to
/// persist — the ROTATED old epoch (lost archive on rotation) or the
/// CURRENT epoch (approval/revision persistence; restore entry weakened).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EpochArchiveWriteKind {
    Rotated,
    Current,
}

impl EpochArchiveWriteKind {
    fn as_str(self) -> &'static str {
        match self {
            EpochArchiveWriteKind::Rotated => "rotated",
            EpochArchiveWriteKind::Current => "current",
        }
    }
}

/// The assessment context a parent disposition must bind (ADR-0010 §4.4:
/// disposition binds activation_id + expected_contract_revision +
/// assessment_id).
#[derive(Debug, Clone)]
pub(crate) struct PendingDisposition {
    pub assessment_id: String,
    /// CAS: the disposition's `expected_contract_revision` must equal the
    /// assessment's `contract_revision`.
    pub expected_contract_revision: u32,
    /// A decision already submitted for this assessment (`close` or
    /// `continue`) — a conflicting second decision is rejected
    /// (`rejected_conflicting`).
    pub decided: Option<String>,
}

/// ACAF goal binding (Slice 1 + goal wiring 2026-08-12): the task-goal
/// digest plus its revision counter, read/written as one lock-protected
/// snapshot so the ticket paths always see a self-consistent pair. A
/// `GoalRevisionV1` consumption bumps `version` — the next ticket
/// `ensure_initialized` sees the mismatch and re-derives `K_session`
/// (ADR-0011 决策 5: goal change → old tickets die).
#[derive(Debug, Clone, Default)]
pub(crate) struct GoalContext {
    pub(crate) digest: Option<String>,
    pub(crate) version: u64,
}

/// F-09 (2026-08-07 review): the mechanical gate over what enters the model
/// context from a `run_tests` call — a fixed completion reminder plus the
/// FINAL output (tail-capped at RUN_TESTS_CONTEXT_CAP; test frameworks put
/// their summary at the end). The full (capped) output was written to disk
/// by the host; the model reads it via read_file when it needs more.
/// 2026-08-08 blackboard partition: compact display form of one edit record
/// — `{file} {old_lines}→{new_lines}行变动`, e.g. `1.py 12→34行变动`
/// (old_lines == 0 = new-file creation: `1.py 新建(5行)`).
pub(crate) fn format_edit_record(record: &EditRecord) -> String {
    if record.old_lines == 0 {
        format!("{} 新建({}行)", record.file, record.new_lines)
    } else {
        format!(
            "{} {}→{}行变动",
            record.file, record.old_lines, record.new_lines
        )
    }
}

pub(crate) fn compose_test_output_message(result: &crate::host::TestRunResult) -> String {
    // THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 审查处理 P2-1):
    // F-09 超时走结构化 timed_out 事实（不再用 "timed out?" 启发式猜测）；
    // 其余 exit_code 缺失维持中性表述。
    let reminder = if result.timed_out {
        "[test-run complete] TIMED OUT — the run did not complete within the wall-clock budget"
            .to_string()
    } else {
        match result.exit_code {
            Some(code) => format!("[test-run complete] exit_code={code}"),
            None => "[test-run complete] exit_code=none (no status)".to_string(),
        }
    };
    // RT-002 (2026-08-11): secret/host-path scrubbing at the CONTEXT
    // boundary (ADR-0010 §3.8.2: "secret、host path 和无关环境信息不得通过
    // 失败输出泄露"). The artifact on disk keeps the raw output; only what
    // enters the conversation is scrubbed — and scrubbing happens BEFORE
    // the tail truncation so a cut-off secret fragment cannot survive as
    // a partial match. Orz-secrets covers known secret shapes plus
    // user-path/home/username segments.
    let secret_scanned = orz_secrets::redact_secrets(&result.output);
    let scrubbed = orz_secrets::redact_user_paths(&secret_scanned);
    let Some(path) = result.full_output_path.as_deref() else {
        // No file on disk (timeout path or write failure): inject the whole
        // (already capped) output rather than lose it.
        return format!("{reminder}\n{}", scrubbed);
    };
    if scrubbed.len() > crate::host::RUN_TESTS_CONTEXT_CAP {
        // Byte-slicing must not split a UTF-8 char — step to the next
        // char boundary.
        let mut start = scrubbed.len() - crate::host::RUN_TESTS_CONTEXT_CAP;
        while start < scrubbed.len() && !scrubbed.is_char_boundary(start) {
            start += 1;
        }
        format!(
            "{reminder}\n[test-run output capped at final {}KB; full output: {path}]\n{}",
            crate::host::RUN_TESTS_CONTEXT_CAP / 1024,
            &scrubbed[start..],
        )
    } else {
        format!("{reminder}\n[full output: {path}]\n{}", scrubbed)
    }
}

impl AgentLoopController {
    /// Default pipeline over a bare FakeProvider (empty script — callers
    /// inject a scripted provider for deterministic runs).
    pub fn new() -> Self {
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(Vec::new()));
        Self::with_gateway(gateway)
    }

    /// Full pipeline over a single shared gateway (main + both subagents).
    pub fn with_gateway(gateway: Arc<dyn ModelGateway>) -> Self {
        Self {
            main_agent: MainAgent::new(gateway.clone()),
            blackboard: Arc::new(SharedBlackboard::new()),
            max_tool_rounds: max_tool_rounds_override().unwrap_or(MAX_TOOL_ROUNDS),
            retrieval_subagent_wallclock: retrieval_subagent_wallclock_override().unwrap_or(Some(
                std::time::Duration::from_secs(RETRIEVAL_SUBAGENT_WALLCLOCK_DEFAULT_SECS),
            )),
            retrieval_max_tool_rounds: retrieval_subagent_max_tool_rounds_override()
                .unwrap_or(RETRIEVAL_SUBAGENT_MAX_TOOL_ROUNDS),
            candidate_cap: web_fetch_candidate_cap_override()
                .unwrap_or(DEFAULT_WEB_FETCH_CANDIDATE_CAP),
            max_inject_tokens_per_round: max_inject_tokens_per_round_override()
                .unwrap_or(DEFAULT_MAX_INJECT_TOKENS_PER_ROUND),
            snapshot_store: None,
            blackboard_archive_dir: None,
            epoch_archive_errors: Mutex::new(Vec::new()),
            pacing_rounds: std::sync::atomic::AtomicU32::new(0),
            denial_state: Mutex::new(DenialState::default()),
            // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26):
            // production reads `ORZ_FOLD_TRIGGER_TOKENS` at construction;
            // tests pin tiny thresholds via
            // `ContextCompactConfig::with_fold_trigger_tokens` (compact.rs).
            context_compact: ContextCompactConfig {
                fold_trigger_tokens: fold_trigger_tokens_override()
                    .unwrap_or(DEFAULT_FOLD_TRIGGER_TOKENS),
                fold_tail_tokens: fold_tail_tokens_override().unwrap_or(DEFAULT_FOLD_TAIL_TOKENS),
                ..ContextCompactConfig::default()
            },
            whitelist: Mutex::new(Vec::new()),
            whitelist_cap: DEFAULT_WHITELIST_CAP,
            activations: Mutex::new(ActivationRegistry::default()),
            dc_state: Mutex::new(crate::diagnostic_coverage::DebugEpisodeState::default()),
            retrieval_mode: RetrievalMode::Off,
            previous_retrieval_mode: None,
            retrieval_capability: RetrievalCapability::Unsupported(
                "retrieval_mode_not_selected".to_string(),
            ),
            bootstrap_transition_pending: std::sync::atomic::AtomicBool::new(false),
            transition_authority: None,
            session_id: None,
            evidence: Mutex::new(Vec::new()),
            main_evidence: Mutex::new(Vec::new()),
            run_source_ledgers: Mutex::new(Vec::new()),
            next_source_seq: Mutex::new(0),
            source_weighting:
                orz_assurance::source_weighting::SourceWeightConfig::from_env_or_default(),
            candidate_prefilter:
                orz_assurance::candidate_prefilter::CandidatePrefilterConfig::from_env_or_default(),
            restored_activations: Mutex::new(Vec::new()),
            acaf: None,
            acaf_fail_closed: false,
            goal_context: Mutex::new(GoalContext::default()),
            policy_revision: std::sync::atomic::AtomicU64::new(0),
            probe_state: Mutex::new(crate::tool_probe::MinimalProbeMap::default()),
            console_registry: crate::console::default_service_registry(),
            console_traces: Mutex::new(TraceStore::new()),
            console_order_seq: std::sync::atomic::AtomicU64::new(0),
            console_probe_source: Mutex::new(None),
            console_default_enabled: false,
            console_mode_state: Mutex::new(crate::console_mode::ConsoleModeState::start(
                crate::console_mode::DEFAULT_DIRECT_FALLBACK_THRESHOLD,
            )),
            status_line_appended: Mutex::new(None),
            delivery_baseline: Mutex::new(None),
            delivery_pending: Mutex::new((0, false)),
            plan_first_enabled: false,
            plan_first_session_done: false,
        }
    }

    /// ACAF Slice 1 (ADR-0011 §4.4): attach the host-side signer client.
    /// `None` (default) keeps the control events unticketed.
    pub fn with_acaf(
        mut self,
        acaf: Option<Arc<tokio::sync::Mutex<crate::acaf::AcafClient>>>,
    ) -> Self {
        self.acaf = acaf;
        self
    }

    /// ACAF Slice 2 (2026-08-13): flip the fail-closed switch. Shadow mode
    /// (default) journals rejections and proceeds; fail-closed refuses the
    /// ticketed event/action on every rejection path (D-14/D-15/D-16).
    pub fn with_acaf_fail_closed(mut self, fail_closed: bool) -> Self {
        self.acaf_fail_closed = fail_closed;
        self
    }

    /// ACAF Slice 1: pin the run's task goal (the prompt) as the ticket goal
    /// binding (check 4). Called at run start by the loop entry; resets the
    /// revision counter to 0 (a fresh run starts a fresh goal epoch).
    pub(crate) fn set_goal_digest(&self, goal: &str) {
        *self.goal_context.lock().unwrap() = GoalContext {
            digest: Some(Self::goal_digest_of(goal)),
            version: 0,
        };
    }

    /// The canonical goal-binding digest: sha256(canonical_json({"goal": …})).
    pub(crate) fn goal_digest_of(goal: &str) -> String {
        sha256_hex(
            &canonical_json(&serde_json::json!({ "goal": goal }))
                .unwrap_or_else(|_| goal.as_bytes().to_vec()),
        )
    }

    /// ACAF goal wiring (Slice 2, 2026-08-12): after a GoalRevisionV1
    /// ticket is consumed under the OLD goal context, swap the run-level
    /// goal binding to the continue's new goal and bump the version. The
    /// caller guarantees the update happens strictly AFTER the ticket's
    /// verify-and-consume (which reads the old cached session) — the next
    /// ticket call re-derives `K_session` and old unconsumed tickets die
    /// (ADR-0011 决策 5). Harmless no-op when ACAF is not configured.
    pub(crate) fn update_goal(&self, new_goal: &str) {
        let mut g = self.goal_context.lock().unwrap();
        g.digest = Some(Self::goal_digest_of(new_goal));
        g.version += 1;
    }

    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): increment the live policy
    /// revision — a policy-identity change. Wired and test-covered; the
    /// first production caller is Slice 3's ModeChangeTicket (ADR-0011
    /// 决策 9 — mode switch increments policy_revision).
    #[allow(dead_code)] // Slice 3 ModeChangeTicket is the first production caller (registered)
    pub(crate) fn bump_policy_revision(&self) {
        self.policy_revision
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Current policy revision (ACAF binding + DenialKey input).
    pub(crate) fn policy_revision(&self) -> u64 {
        self.policy_revision
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    // ── FUS-TOOL-PROBE (P0-A steps 2/5 + P0-A-2 single probe face) ─────

    /// The `tool_availability_check` payload for a work-tool snapshot —
    /// neutral reasons only. `gate_decision` stays the internal mechanical
    /// "pass" (the probe never blocks — design invariant 2; the §7 draft
    /// example keeps pass even with incomplete tools); non-pass mapping
    /// remains reserved for future gate semantics.
    pub(crate) fn tool_availability_payload(
        snapshot: &crate::tool_probe::ToolProbeSnapshot,
    ) -> serde_json::Value {
        let incomplete: Vec<serde_json::Value> = snapshot
            .incomplete
            .iter()
            .map(|f| serde_json::json!({ "tool": f.tool, "reason": f.reason }))
            .collect();
        serde_json::json!({
            "probe_scope": "main_agent_work_tools",
            "probe_timestamp": chrono_utc_now(),
            "complete": snapshot.complete,
            "incomplete": incomplete,
            "gate_decision": "pass",
        })
    }

    /// P0-A step 5: seed the minimal previous-round map from the
    /// pre-run_started snapshot — the loop's first per-round probe then
    /// emits only on an actual flip.
    pub(crate) fn probe_state_seed(&self, snapshot: &crate::tool_probe::ToolProbeSnapshot) {
        *self.probe_state.lock().unwrap() =
            crate::tool_probe::MinimalProbeMap::from_snapshot(snapshot);
    }

    /// P0-A step 5: compare a fresh snapshot against the minimal
    /// previous-round map; on a flip, replace the map and return `true`
    /// (the caller journals the event with the fresh snapshot).
    pub(crate) fn probe_flip(&self, snapshot: &crate::tool_probe::ToolProbeSnapshot) -> bool {
        let mut map = self.probe_state.lock().unwrap();
        if map.differs_from(snapshot) {
            *map = crate::tool_probe::MinimalProbeMap::from_snapshot(snapshot);
            true
        } else {
            false
        }
    }

    /// P0-A step 5 (design §5): 调用即探针 — a real work-tool call failure
    /// (ToolCompleted status=error) writes back into the minimal map as
    /// incomplete so the next probe compares against the corrected state.
    /// Non-work tools are ignored (their declaration rules are unchanged).
    pub(crate) fn note_probe_call_failure(&self, tool: &str) {
        self.probe_state.lock().unwrap().mark_incomplete(tool);
    }

    /// P0-A step 5 review fix (2026-08-13): 调用即探针, lane-gated — only
    /// lanes that own the probe map (main/grill, `probe_writeback`
    /// = `profile.probe_work_tools`) write call failures back. Retrieval lanes
    /// never re-probe and must not pollute the main map with lane-local
    /// failures (would surface as spurious recovery-flip events in the main
    /// audit stream).
    pub(crate) fn maybe_note_probe_call_failure(&self, probe_writeback: bool, tool: &str) {
        if probe_writeback {
            self.note_probe_call_failure(tool);
        }
    }

    /// FUS-TOOL-PROBE P0-A-2: whether the run carries a goal context
    /// (`todo_write` / `update_goal` probe chain). Pinned at run start by
    /// `set_goal_digest`.
    pub(crate) fn goal_context_present(&self) -> bool {
        self.goal_context.lock().unwrap().digest.is_some()
    }

    /// FUS-TOOL-PROBE P0-A-2 (审查复核 2026-08-13): whether an activation
    /// carries an undisposed pending assessment — the only state in which
    /// `retrieval_disposition` can be submitted.
    pub(crate) fn has_live_activation(&self) -> bool {
        self.activations.lock().unwrap().has_live()
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): attach the session-level retrieval
    /// mode (ADR-0010 §3.7.1). `bootstrap_transition_pending` journals one
    /// `retrieval_mode_transition` (off → mode) at the next run's startup.
    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：可选
    /// transition 元数据（authority/reason）——模式 A 自动降级
    /// （local_browser probe 失败 → framework_fallback）以
    /// `mechanical_probe` / `browser_launch_failed` 落盘；缺省保持
    /// `session_bootstrap` / `session_default`（既有显式选择语义）。
    pub fn with_retrieval_mode(
        mut self,
        mode: RetrievalMode,
        capability: RetrievalCapability,
        bootstrap_transition_pending: bool,
        session_id: Option<String>,
        previous_mode: Option<RetrievalMode>,
        transition_authority: Option<(String, String)>,
    ) -> Self {
        self.retrieval_mode = mode;
        self.retrieval_capability = capability;
        self.bootstrap_transition_pending =
            std::sync::atomic::AtomicBool::new(bootstrap_transition_pending);
        self.session_id = session_id;
        self.previous_retrieval_mode = previous_mode;
        self.transition_authority = transition_authority;
        self
    }

    /// GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): override the mechanical
    /// source tier judge config (ADR-0010 §3.7 条 12). Defaults to the
    /// embedded seed lists (runtime override: `ORZ_SOURCE_WEIGHTING_CONFIG`).
    pub fn with_source_weighting_config(
        mut self,
        config: orz_assurance::source_weighting::SourceWeightConfig,
    ) -> Self {
        self.source_weighting = config;
        self
    }

    /// FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): override the mechanical
    /// candidate prefilter config (defaults to the embedded seed lists;
    /// runtime override: `ORZ_CANDIDATE_PREFILTER_CONFIG`).
    pub fn with_candidate_prefilter_config(
        mut self,
        config: orz_assurance::candidate_prefilter::CandidatePrefilterConfig,
    ) -> Self {
        self.candidate_prefilter = config;
        self
    }

    /// GAP-RETRIEVAL-TOOLS: whether the bootstrap mode transition was
    /// journaled (the pending flag cleared) — the acp_server uses it to
    /// clear the sidecar flag after a successful run.
    pub fn bootstrap_transition_journaled(&self) -> bool {
        !self
            .bootstrap_transition_pending
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): seed the activation registry from a
    /// cross-prompt sidecar snapshot. Restored activations are journaled as
    /// `retrieval_activation_restored` at the next run's startup (each
    /// prompt's controller build declares the handover once).
    pub fn with_activation_snapshot(self, snapshot: Option<&serde_json::Value>) -> Self {
        if let Some(value) = snapshot {
            let restored = {
                let mut reg = self.activations.lock().unwrap();
                reg.seed_from_json(value)
            };
            *self.restored_activations.lock().unwrap() = restored;
        }
        self
    }

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1, 2026-08-27)：恢复时重建
    /// 检索分区——PULL 模式下子代理全文不进主对话，跨 run 后分区是模型
    /// 唯一可追溯视图；ACP 会话重启后黑板为全新内存态，由 sidecar 快照
    /// 携带的 internal_ret / external_ret 经本 builder 灌回。`None` =
    /// 该会话尚无对应分区（保持空分区）。单写者纪律：灌回发生在 run 外
    /// （会话恢复边界），run 内仍由派发路径独占写入。
    pub fn with_retrieval_partitions(
        self,
        internal: Option<InternalRetSection>,
        external: Option<ExternalRetSection>,
    ) -> Self {
        if internal.is_some() || external.is_some() {
            let mut w = self.blackboard.write();
            if let Some(section) = internal {
                w.internal_ret = section;
            }
            if let Some(section) = external {
                w.external_ret = section;
            }
        }
        self
    }

    /// IP5: attach the session's pre-mutation snapshot store (see
    /// `orz-assurance::session::snapshot`). Mutation-class tools with
    /// knowable targets get tracked before execution.
    pub fn with_snapshot_store(mut self, store: Option<Arc<SnapshotStore>>) -> Self {
        self.snapshot_store = store;
        self
    }

    /// v1.15 (2026-08-14): attach the plan-epoch archive directory
    /// (`<session_cwd>/.gsa/blackboard`). When `Some`, the latest archived
    /// epoch snapshot is restored into the blackboard (plan/edits/
    /// tool_actions/exec — the epoch restore entry), later rotations
    /// persist their snapshots there, and `blackboard_read` can query
    /// archived epochs.
    pub fn with_blackboard_archive_dir(mut self, dir: Option<PathBuf>) -> Self {
        if let Some(dir) = &dir
            && let Some(snapshot) = crate::epoch::latest_epoch_snapshot(dir)
        {
            self.blackboard.write().restore_epoch_snapshot(&snapshot);
        }
        self.blackboard_archive_dir = dir;
        self
    }

    /// F5 (2026-08-15, BACKLOG 6e 复查遗留): the single source of truth for
    /// the epoch archive directory. Consumers (the path-slot overflow
    /// pointer, cross-epoch `blackboard_read`, archive writes/restores)
    /// must derive from this value — never re-derive a
    /// `session_cwd/.gsa/blackboard` path, which would distort a custom
    /// archive directory.
    pub(crate) fn blackboard_archive_dir(&self) -> Option<&Path> {
        self.blackboard_archive_dir.as_deref()
    }

    /// GAP-SUBAGENT-RUNTIME (M4/M5 tests): an independent small budget —
    /// the main loop AND the subagent loops share the configured cap
    /// (env override mirrors the main; ADR-0010 §3.4.6 independent
    /// accounting means each loop instance counts separately).
    pub fn with_max_tool_rounds(mut self, rounds: u32) -> Self {
        self.max_tool_rounds = rounds;
        self
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): pin the candidate
    /// cap (test seam; production reads ORZ_WEB_FETCH_CANDIDATE_CAP at
    /// construction — shared by web_fetch and browser_read).
    pub fn with_candidate_cap(mut self, cap: u32) -> Self {
        self.candidate_cap = cap.max(1);
        self
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): test seam for
    /// the per-round injection budget (production reads
    /// `ORZ_MAX_INJECT_TOKENS_PER_ROUND` at construction).
    pub fn with_max_inject_tokens_per_round(mut self, budget: u64) -> Self {
        self.max_inject_tokens_per_round = budget.max(1);
        self
    }

    /// 2026-08-08 blackboard partition (A4) + v1.15 (2026-08-14): ingest an
    /// approved plan — `plan_id` + `plan_epoch` identity, goal + step
    /// descriptions — into the blackboard plan section (the plan-mode
    /// state-machine mapping; §4.1: goal、步骤 + 状态). The first step is
    /// marked in-progress, the rest pending; step transitions are a later
    /// refinement (steps stay pending until then — the status line is
    /// byte-stable across rounds, which is exactly what the prefix-cache
    /// discipline wants). Non-plan runs simply never call this: the status
    /// line is absent and zero dilution.
    ///
    /// Same `plan_id` = same-epoch revision (plan text replaced, blackboard
    /// untouched). New `plan_id` = new plan epoch: the old epoch's
    /// plan/edits/tool_actions/exec are captured and archived (when an
    /// archive dir is configured), then the epoch-scoped partitions are
    /// cleared and the new plan written — one atomic rotation. The current
    /// epoch's snapshot is always persisted after ingest (approval or
    /// revision) so a process restart can restore the live epoch — the
    /// rotation archive preserves the replaced epochs.
    ///
    /// Identity invariants (v1.15⑧, 2026-08-15): the `plan_id ↔ plan_epoch`
    /// mapping is one-to-one and enforced — same `plan_id` must reuse the
    /// SAME `plan_epoch`; a new `plan_id` must advance to a STRICTLY GREATER
    /// `plan_epoch` (timestamp-stamped monotonic numbering). Violations
    /// fail fast via `try_with_plan`/`with_plan` and never mutate the board.
    ///
    /// The plan section feeds both the resident status line (`[任务状态]`
    /// block in the system prompt) and the `blackboard_read` plan partition.
    pub fn with_plan(
        self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<String>,
    ) -> Self {
        self.try_with_plan(plan_id, plan_epoch, goal, steps).expect(
            "plan epoch identity invariant violated (same plan_id must reuse \
             its plan_epoch; a new plan_id must advance to a strictly greater \
             plan_epoch)",
        )
    }

    /// Fallible form of [`Self::with_plan`]: returns the identity violation
    /// instead of panicking. On `Err` the blackboard is left untouched.
    pub fn try_with_plan(
        self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<String>,
    ) -> Result<Self, crate::blackboard::PlanEpochError> {
        let rotated = {
            let mut w = self.blackboard.write();
            w.rotate_to_plan(plan_id, plan_epoch, goal, steps, &chrono_utc_now())?
        };
        if let Some(dir) = &self.blackboard_archive_dir {
            if let Some(snapshot) = rotated {
                if !crate::epoch::write_epoch_archive_retry(dir, &snapshot) {
                    tracing::warn!(
                        "epoch archive write failed ({}): epoch {} — rotation still committed",
                        dir.display(),
                        snapshot.plan_epoch,
                    );
                    // F7 (2026-08-15): queue the audit event — flushed at
                    // run start when the journal writer exists.
                    self.epoch_archive_errors
                        .lock()
                        .unwrap()
                        .push((snapshot.plan_epoch, EpochArchiveWriteKind::Rotated));
                }
            }
            let current = {
                let bb = self.blackboard.read();
                bb.epoch_snapshot(&chrono_utc_now())
            };
            if !crate::epoch::write_epoch_archive_retry(dir, &current) {
                tracing::warn!(
                    "epoch archive write failed ({}): epoch {} — live board still committed",
                    dir.display(),
                    current.plan_epoch,
                );
                self.epoch_archive_errors
                    .lock()
                    .unwrap()
                    .push((current.plan_epoch, EpochArchiveWriteKind::Current));
            }
        }
        Ok(self)
    }

    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): land a validated
    /// structured plan (design §5) on the blackboard plan epoch — the
    /// `&self` twin of [`Self::try_with_plan`] for the `plan_write` tool
    /// handler (the blackboard is shared). Same identity invariants and
    /// same archive discipline (F7 failures queued for the run-start flush).
    pub(crate) fn apply_structured_plan(
        &self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<crate::blackboard::PlanStep>,
    ) -> Result<(), crate::blackboard::PlanEpochError> {
        let rotated = {
            let mut w = self.blackboard.write();
            w.rotate_to_structured_plan(plan_id, plan_epoch, goal, steps, &chrono_utc_now())?
        };
        if let Some(dir) = &self.blackboard_archive_dir {
            if let Some(snapshot) = rotated {
                if !crate::epoch::write_epoch_archive_retry(dir, &snapshot) {
                    tracing::warn!(
                        "epoch archive write failed ({}): epoch {} — rotation still committed",
                        dir.display(),
                        snapshot.plan_epoch,
                    );
                    self.epoch_archive_errors
                        .lock()
                        .unwrap()
                        .push((snapshot.plan_epoch, EpochArchiveWriteKind::Rotated));
                }
            }
            let current = {
                let bb = self.blackboard.read();
                bb.epoch_snapshot(&chrono_utc_now())
            };
            if !crate::epoch::write_epoch_archive_retry(dir, &current) {
                tracing::warn!(
                    "epoch archive write failed ({}): epoch {} — live board still committed",
                    dir.display(),
                    current.plan_epoch,
                );
                self.epoch_archive_errors
                    .lock()
                    .unwrap()
                    .push((current.plan_epoch, EpochArchiveWriteKind::Current));
            }
        }
        Ok(())
    }

    /// F7 (2026-08-15, BACKLOG 6e 复查遗留): flush plan-epoch archive
    /// write failures queued during the builder phase into the journal as
    /// `epoch_archive_write_failed` events. The rotation/approval already
    /// committed in memory — this event is the durable audit trace that
    /// the old epoch's snapshot is NOT on disk. A refused append fails the
    /// run (journal integrity violation, same discipline as every event).
    async fn flush_epoch_archive_write_failures(
        &self,
        writer: &mut EventWriter<'_>,
    ) -> Result<(), AgentLoopError> {
        let pending = std::mem::take(&mut *self.epoch_archive_errors.lock().unwrap());
        for (plan_epoch, kind) in pending {
            let archive_dir = self
                .blackboard_archive_dir
                .as_deref()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            writer
                .record(
                    EventType::EpochArchiveWriteFailed,
                    serde_json::json!({
                        "archive_dir": archive_dir,
                        "plan_epoch": plan_epoch,
                        "kind": kind.as_str(),
                        "attempts": crate::epoch::EPOCH_ARCHIVE_MAX_ATTEMPTS,
                    }),
                )
                .await?;
        }
        Ok(())
    }

    /// A6 review D2-2 (2026-08-08): one-line MECHANICAL digest of the
    /// blackboard — total edit records + tool-action counts by category —
    /// the deterministic「摘要」for the compaction marker (zero model calls;
    /// dropped rounds dominate the totals, and the exact per-round
    /// A4 (2026-08-08): render the resident 极简状态行 from the blackboard
    /// plan section — `None` when no plan is set (zero injection). The block
    /// is a pure function of plan state, so it is byte-identical across
    /// rounds while the plan is unchanged (prefix-cache discipline).
    pub(crate) fn render_status_line(&self) -> Option<String> {
        let bb = self.blackboard.read();
        if bb.plan.goal.is_none() && bb.plan.steps.is_empty() {
            return None;
        }
        Some(crate::prompt::build_status_line(
            bb.plan.goal.as_deref(),
            &bb.plan.steps,
        ))
    }

    /// 2026-08-18 (ADR-0010 §14.25 项 1): 前缀缓存纪律——`[任务状态]`
    /// 常驻状态行移出系统提示词，改为尾随用户消息、仅在变化时追加
    /// （尾随消息纪律；退役前的 `[TOOL_ROUND_BUDGET] REMAINING` 同此
    /// 纪律）。系统提示词保持完全静态，步骤推进不再打断提供方前缀缓存；
    /// 变化轮仅小段状态行作为新尾随消息计费。无计划（None）不追加。
    pub(crate) fn sync_status_line_message(&self, messages: &mut Vec<Message>) {
        let line = self.render_status_line();
        let mut last = self.status_line_appended.lock().unwrap();
        if line != *last {
            if let Some(text) = line.clone() {
                messages.push(Message {
                    role: Role::User,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
            }
            *last = line;
        }
    }

    /// Component injection for tests (independent scripted providers).
    /// STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37)：检索
    /// 子代理不再作为 controller 常驻字段——每 run 由 `run_turn_inner`
    /// 的 per-run gateway 在 `retrieval/dispatch.rs` 的
    /// `run_retrieval_subagent` 内局部构造，本签名不再接收 retrieval
    /// 组件。
    pub fn with_components(main_agent: MainAgent) -> Self {
        Self {
            main_agent,
            blackboard: Arc::new(SharedBlackboard::new()),
            max_tool_rounds: MAX_TOOL_ROUNDS,
            retrieval_subagent_wallclock: Some(std::time::Duration::from_secs(
                RETRIEVAL_SUBAGENT_WALLCLOCK_DEFAULT_SECS,
            )),
            retrieval_max_tool_rounds: RETRIEVAL_SUBAGENT_MAX_TOOL_ROUNDS,
            candidate_cap: DEFAULT_WEB_FETCH_CANDIDATE_CAP,
            max_inject_tokens_per_round: DEFAULT_MAX_INJECT_TOKENS_PER_ROUND,
            snapshot_store: None,
            blackboard_archive_dir: None,
            epoch_archive_errors: Mutex::new(Vec::new()),
            pacing_rounds: std::sync::atomic::AtomicU32::new(0),
            denial_state: Mutex::new(DenialState::default()),
            context_compact: ContextCompactConfig::default(),
            whitelist: Mutex::new(Vec::new()),
            whitelist_cap: DEFAULT_WHITELIST_CAP,
            activations: Mutex::new(ActivationRegistry::default()),
            dc_state: Mutex::new(crate::diagnostic_coverage::DebugEpisodeState::default()),
            retrieval_mode: RetrievalMode::Off,
            previous_retrieval_mode: None,
            retrieval_capability: RetrievalCapability::Unsupported(
                "retrieval_mode_not_selected".to_string(),
            ),
            bootstrap_transition_pending: std::sync::atomic::AtomicBool::new(false),
            transition_authority: None,
            session_id: None,
            evidence: Mutex::new(Vec::new()),
            main_evidence: Mutex::new(Vec::new()),
            run_source_ledgers: Mutex::new(Vec::new()),
            next_source_seq: Mutex::new(0),
            source_weighting:
                orz_assurance::source_weighting::SourceWeightConfig::from_env_or_default(),
            candidate_prefilter:
                orz_assurance::candidate_prefilter::CandidatePrefilterConfig::from_env_or_default(),
            restored_activations: Mutex::new(Vec::new()),
            acaf: None,
            acaf_fail_closed: false,
            goal_context: Mutex::new(GoalContext::default()),
            policy_revision: std::sync::atomic::AtomicU64::new(0),
            probe_state: Mutex::new(crate::tool_probe::MinimalProbeMap::default()),
            console_registry: crate::console::default_service_registry(),
            console_traces: Mutex::new(TraceStore::new()),
            console_order_seq: std::sync::atomic::AtomicU64::new(0),
            console_probe_source: Mutex::new(None),
            console_default_enabled: false,
            console_mode_state: Mutex::new(crate::console_mode::ConsoleModeState::start(
                crate::console_mode::DEFAULT_DIRECT_FALLBACK_THRESHOLD,
            )),
            status_line_appended: Mutex::new(None),
            delivery_baseline: Mutex::new(None),
            delivery_pending: Mutex::new((0, false)),
            plan_first_enabled: false,
            plan_first_session_done: false,
        }
    }

    pub fn blackboard(&self) -> &Arc<SharedBlackboard> {
        &self.blackboard
    }

    /// PLAN-FIRST 阶段 B (2026-08-16): 记录主车道本轮探针源——工具栏投影
    /// 与注册板块（黑板模型栏）共用的单一事实源。仅内存、随轮覆盖。
    pub(crate) fn set_console_probe_source(
        &self,
        policy: crate::host::ToolPolicy,
        snapshot: crate::tool_probe::ToolProbeSnapshot,
    ) {
        *self.console_probe_source.lock().unwrap() = Some((policy, snapshot));
    }

    /// PLAN-FIRST 阶段 B 审查收口 (2026-08-16): 探针源随 run 复位——与
    /// `probe_state` 同纪律（run 起始清空，本 run 首个有探针轮重新记录；
    /// 不跨 run 沿用）。复位后无探针源时，注册板块沿用既有内容、不派生。
    pub(crate) fn reset_console_probe_source(&self) {
        *self.console_probe_source.lock().unwrap() = None;
    }

    /// PLAN-FIRST 阶段 B (2026-08-16): 注册板块派生并持久化——由最近探针源
    /// 派生 Profile/Bundle ∩ 探针完整集投影并写入黑板 actions 板块；无探针
    /// 源时不改写（checkpoint 轮/无探针轮次沿用既有内容，替代 bundle-only
    /// 静态刷新中间态）。
    pub(crate) fn sync_console_registrations(&self) {
        let Some((policy, probe)) = self.console_probe_source.lock().unwrap().clone() else {
            return;
        };
        let registrations = self.console_registrations(policy, Some(&probe));
        self.blackboard
            .write()
            .actions
            .set_registration(registrations);
    }

    /// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): enable the
    /// first-round plan gate for main-lane sessions. Production wiring
    /// (ACP server / CLI run) calls this; tests keep the legacy surface.
    pub fn with_plan_first_enabled(mut self, enabled: bool) -> Self {
        self.plan_first_enabled = enabled;
        self
    }

    /// PLAN-FIRST 阶段 A (2026-08-16): mark the session's first-round plan
    /// gate as already completed (or restored with history) — later runs in
    /// the same session skip the gate.
    pub fn with_plan_first_session_done(mut self, done: bool) -> Self {
        self.plan_first_session_done = done;
        self
    }

    pub(crate) fn plan_first_enabled(&self) -> bool {
        self.plan_first_enabled
    }

    pub(crate) fn plan_first_session_done(&self) -> bool {
        self.plan_first_session_done
    }

    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): enable the
    /// console-default model surface (dual-mode: console default + direct
    /// audited fallback). Production wiring (ACP server / CLI run) calls
    /// this together with `with_plan_first_enabled`; tests keep the legacy
    /// surface by default.
    pub fn with_console_default_enabled(mut self, enabled: bool) -> Self {
        self.console_default_enabled = enabled;
        self
    }

    pub(crate) fn console_default_enabled(&self) -> bool {
        self.console_default_enabled
    }

    /// PLAN-FIRST 阶段 C: 故障面阈值（§7.2）——`ORZ_CONSOLE_DIRECT_
    /// FALLBACK_THRESHOLD` 环境变量覆盖，默认 3。
    pub(crate) fn console_fallback_threshold() -> u32 {
        std::env::var(crate::console_mode::FALLBACK_THRESHOLD_ENV)
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|n| *n >= 1)
            .unwrap_or(crate::console_mode::DEFAULT_DIRECT_FALLBACK_THRESHOLD)
    }

    pub(crate) fn console_mode(&self) -> crate::console_mode::ConsoleMode {
        self.console_mode_state.lock().unwrap().mode
    }

    /// run 起始复位（与 `reset_console_probe_source` 同一点调用）：
    /// 模式回 console、连败清零、询问标记清空、direct 证据面清空。
    pub(crate) fn reset_console_mode(&self) {
        *self.console_mode_state.lock().unwrap() =
            crate::console_mode::ConsoleModeState::start(Self::console_fallback_threshold());
    }

    /// 询问触发检查（§7.2/§7.3）：console 态 + 连续故障 ≥ 阈值 + 未问。
    pub(crate) fn console_inquiry_due(&self) -> bool {
        self.console_default_enabled && self.console_mode_state.lock().unwrap().inquiry_due()
    }

    /// 询问轮触发的快照：当前连败计数 + 构成连败的订单 id。
    pub(crate) fn console_streak_snapshot(&self) -> (u32, Vec<String>) {
        let state = self.console_mode_state.lock().unwrap();
        (state.streak, state.streak_order_ids.clone())
    }

    /// direct 模式直接动作开始：创建 console trace（trace_id 供事件盖章
    /// 与 `console_step_done` 证据），返回盖章 + 工作 trace。非 direct
    /// 态返回 None。
    pub(crate) fn console_direct_begin(
        &self,
        call_id: &str,
    ) -> Option<(crate::console_mode::DirectStamp, crate::console::Trace)> {
        let state = self.console_mode_state.lock().unwrap();
        if !state.is_direct() {
            return None;
        }
        let transition_id = state.transition_id.clone()?;
        let trace = self
            .console_traces
            .lock()
            .unwrap()
            .new_trace(Some(call_id.to_string()));
        let stamp = crate::console_mode::DirectStamp {
            transition_id,
            trace_id: trace.trace_id.clone(),
        };
        Some((stamp, trace))
    }

    /// direct 模式直接动作收口：trace 事件提交 + 证据面登记（§7.5
    /// step_done 的 trace_id ↔ ToolCompleted 对应关系由此建立）。
    pub(crate) fn console_direct_end(
        &self,
        stamp: &crate::console_mode::DirectStamp,
        mut trace: crate::console::Trace,
        tool: &str,
        ok: bool,
        detail: Option<&str>,
    ) {
        trace.add(
            "execute",
            Some(tool),
            ok,
            if ok { None } else { Some("error") },
            Some(detail.unwrap_or_default().to_string()),
            None,
        );
        self.console_traces.lock().unwrap().commit(&trace);
        self.console_mode_state
            .lock()
            .unwrap()
            .record_direct_trace(&stamp.trace_id);
    }

    /// 2026-08-08 blackboard partition (A3) + v1.15 (2026-08-14): render one
    /// blackboard section for the `blackboard_read` tool. `since` (ISO 8601 /
    /// RFC 3339 — the journal timestamp format) filters timestamped entries;
    /// plan has current state only (no per-entry timestamps) and exec entries
    /// carry none — `since` applies to edits and tool_actions.
    ///
    /// `epoch = Some(n)` reads the ARCHIVED epoch-n snapshot instead of the
    /// live view (cross-epoch look-back). Missing archive or unconfigured
    /// archive dir returns an explicit message — never a silent fallback to
    /// the live board.
    ///
    /// `receipt_id` (方案 B，2026-08-19，ADR-0010 §14.31 / 设计 §4.5)：结果栏
    /// 单条 receipt 点读——仅与 section=actions 组合有效；与 epoch 组合 =
    /// 归档快照点读；无 receipt_id 时整段输出与 S1 逐字节一致。
    ///
    /// Review closure (P2-2, 2026-08-08): `since` is parsed as RFC 3339 —
    /// a bare string compare silently dropped same-instant records when the
    /// model passed 'Z' or truncated precision. Unparseable values fall
    /// back to no filtering (read everything), never nothing.
    pub(crate) fn render_blackboard_section(
        &self,
        section: &str,
        since: Option<&str>,
        epoch: Option<u64>,
        receipt_id: Option<&str>,
    ) -> String {
        // THIN-HARNESS-REDESIGN R2a (2026-08-27, §4.4): 检索分区按需拉取
        // （PUSH → PULL）。live-only——检索结果从不进 epoch 快照（全文
        // 走 journal + retrieval-results 存档），带 epoch 读取 = 显式报错
        // （fail loud，同 session 面纪律）；receipt_id 仅 actions 点读语义，
        // 组合 = 显式报错。`since` 同 plan/exec/actions 处理：检索分区无
        // 条目级时间戳，忽略（不缩小读取范围）。
        if section == "internal_ret" || section == "external_ret" {
            if epoch.is_some() {
                return format!(
                    "blackboard_read {section} with epoch is not supported — \
                     retrieval partitions are live-only (results ride the \
                     journal + retrieval-results archive); omit epoch to \
                     read the live section"
                );
            }
            if receipt_id.is_some() {
                return format!(
                    "receipt_id 仅与 section=actions 组合有效（点读结果栏单条 \
                     receipt）；当前 section={section} 不支持 receipt_id"
                );
            }
            let bb = self.blackboard.read();
            return crate::epoch::render_retrieval_section(
                section,
                &bb.internal_ret,
                &bb.external_ret,
            );
        }
        // R2 半助理层（§4.5）：实体状态分区——process/file/environment
        // 稳定视图（摘要清单 + 总上限），live-only（不进 epoch 快照），
        // 单实体详情（含最近失败诊断）经 `diagnostics.diagnose` 点读。
        if section == "entities" {
            if epoch.is_some() {
                return format!(
                    "blackboard_read {section} with epoch is not supported — \
                     entities is a live-only partition (half-assistant entity \
                     state rides the run lifecycle); omit epoch to read the \
                     live section"
                );
            }
            if receipt_id.is_some() {
                return format!(
                    "receipt_id 仅与 section=actions 组合有效（点读结果栏单条 \
                     receipt）；当前 section={section} 不支持 receipt_id"
                );
            }
            let bb = self.blackboard.read();
            return bb
                .entities
                .render_text(crate::entities::ENTITIES_SUMMARY_MAX);
        }
        if let Some(epoch) = epoch {
            let Some(dir) = &self.blackboard_archive_dir else {
                return format!(
                    "epoch snapshot {epoch} unavailable: blackboard archive dir not configured"
                );
            };
            return match crate::epoch::load_epoch_snapshot(dir, epoch) {
                Some(snapshot) => crate::epoch::render_section(
                    &snapshot.plan,
                    &snapshot.edits,
                    &snapshot.tool_actions,
                    &snapshot.exec,
                    &snapshot.actions,
                    section,
                    since,
                    receipt_id,
                ),
                None => format!(
                    "epoch snapshot {epoch} not found (archive: {})",
                    dir.display()
                ),
            };
        }
        // PLAN-FIRST 阶段 B (2026-08-16): 注册板块绑定黑板模型栏——读取
        // actions 分区时由最近探针源派生（并持久化回板块），替代仅依赖
        // loop-top 静态刷新的陈旧内容；无探针源时沿用既有内容。归档
        // epoch 读保持快照原样，不派生。
        if section == "actions" {
            self.sync_console_registrations();
        }
        let bb = self.blackboard.read();
        crate::epoch::render_section(
            &bb.plan,
            &bb.edits,
            &bb.tool_actions,
            &bb.exec,
            &bb.actions,
            section,
            since,
            receipt_id,
        )
    }

    /// PUSH→PULL (2026-08-21, CONTEXT_SCAFFOLDING_PULL_REDESIGN §4 方案 A):
    /// the `blackboard_read section=session` face — live session state only
    /// (never archived): tool-round budget used/remaining + the resident
    /// status line (`render_status_line`). `tool_rounds` is the in-run
    /// consumed count (completed rounds; the in-flight round counts against
    /// the budget when it completes — the same accounting as the retired
    /// per-round REMAINING block). `epoch` with this section = explicit
    /// error (the face is live-only, nothing is archived under "session");
    /// `receipt_id` with this section = explicit error (point-read is an
    /// actions-board addressing concept). `since` is ignored — the face is
    /// a single live snapshot (same lenient treatment as plan/exec/actions).
    /// 2026-08-21 全面审查处理（O4）：组合错误以 `Err` 返回——调用方按
    /// 参数级显式报错处理（ToolCompleted exit_code 1 + error 字段），与
    /// 非法 epoch/receipt_id 同纪律（区别于纯渲染层的 receipt_id+非
    /// actions 文本错误先例，后者保持 exit_code 0，差异登记于设计 §8）。
    pub(crate) fn render_session_section(
        &self,
        epoch: Option<u64>,
        receipt_id: Option<&str>,
        tool_rounds: u32,
    ) -> Result<String, String> {
        if let Some(epoch) = epoch {
            return Err(format!(
                "invalid blackboard_read session read: session 面是 live 会话状态，\
                 不进 epoch 归档；省略 epoch 参数读取实时状态（epoch={epoch}）"
            ));
        }
        if receipt_id.is_some() {
            return Err(
                "receipt_id 仅与 section=actions 组合有效（点读结果栏单条 receipt）；\
                 当前 section=session 不支持 receipt_id"
                    .to_string(),
            );
        }
        Ok(crate::prompt::session_face_block(
            tool_rounds,
            self.max_tool_rounds,
            self.render_status_line().as_deref(),
        ))
    }

    /// Run a single turn of the agent loop for a given user prompt.
    ///
    /// This is the entry point called by orz-host's ACP `session/prompt` handler.
    /// It writes all events to the host's journal and returns when the turn finishes.
    ///
    /// `next_sequence` and `previous_event_sha256` continue the journal's hash
    /// chain from where the session bootstrap (or a prior turn) left off.
    ///
    /// Returns `(response, next_sequence, last_event_sha256)` so the caller can
    /// continue the chain on the next turn (multi-prompt session support).
    /// On error a terminal `run_failed` event is written (best effort) so the
    /// journal never ends on a mid-sequence orphan.
    /// `orientation`: session-level orientation state (ADR-0010 §4.2,
    /// GAP-INQUIRY-SPLIT) — `None` for grill turns and one-shot CLI runs.
    /// `conversation` (GAP-CONVERSATION-RESTORE, 2026-08-10): session-level
    /// multi-prompt conversation — `Some` seeds the model context with the
    /// prior prompts' history (clone) and writes the full conversation back
    /// on success; `None` (one-shot CLI / grill) keeps the single-prompt
    /// seed.
    #[allow(clippy::too_many_arguments)] // mirrors run_turn_with_guards
    pub async fn run_turn(
        &self,
        host: &dyn LoopHost,
        prompt: &str,
        run_id: &str,
        run_manifest_sha256: &str,
        next_sequence: u64,
        previous_event_sha256: Option<String>,
        orientation: Option<&mut OrientationSessionState>,
        conversation: Option<&mut Vec<Message>>,
    ) -> Result<(String, u64, Option<String>), AgentLoopError> {
        self.run_turn_with_cancel(
            host,
            prompt,
            run_id,
            run_manifest_sha256,
            next_sequence,
            previous_event_sha256,
            None,
            orientation,
            conversation,
        )
        .await
    }

    /// `run_turn` with cooperative cancellation (Phase 3 slice #7).
    ///
    /// `cancel` is polled at fixed checkpoints — loop top, after each model
    /// round, and before tool dispatch — never inside the permission await
    /// (the permission manager is session-scoped and shared; a dropped
    /// future could leave its prompt unresolved until the dialog answers or
    /// the host's 300s timeout). `None` behaves exactly like `run_turn`.
    #[allow(clippy::too_many_arguments)] // mirrors run_turn + the cancel token
    pub async fn run_turn_with_cancel(
        &self,
        host: &dyn LoopHost,
        prompt: &str,
        run_id: &str,
        run_manifest_sha256: &str,
        next_sequence: u64,
        previous_event_sha256: Option<String>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        orientation: Option<&mut OrientationSessionState>,
        conversation: Option<&mut Vec<Message>>,
    ) -> Result<(String, u64, Option<String>), AgentLoopError> {
        self.run_turn_with_guards(
            host,
            prompt,
            run_id,
            run_manifest_sha256,
            next_sequence,
            previous_event_sha256,
            cancel,
            None,
            orientation,
            conversation,
        )
        .await
    }

    /// `run_turn_with_cancel` + a P1-1 (2026-08-08 stall guards) activity
    /// heartbeat. `heartbeat` is stamped on every journal event (inside
    /// `EventWriter`) and forwarded to the gateway so the transport stamps
    /// it on every wire frame — the stall watchdog measures silence since
    /// the last stamp. `None` behaves exactly like `run_turn_with_cancel`.
    #[allow(clippy::too_many_arguments)] // mirrors run_turn_with_cancel + the heartbeat
    pub async fn run_turn_with_guards(
        &self,
        host: &dyn LoopHost,
        prompt: &str,
        run_id: &str,
        run_manifest_sha256: &str,
        next_sequence: u64,
        previous_event_sha256: Option<String>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        orientation: Option<&mut OrientationSessionState>,
        conversation: Option<&mut Vec<Message>>,
    ) -> Result<(String, u64, Option<String>), AgentLoopError> {
        // ACAF Slice 2 fail-closed D-15 (2026-08-13): fail-closed with an
        // unconfigured fabric is a STARTUP error — there is no unticketed
        // channel and no silent downgrade (ADR-0011 §2 fail-closed
        // principle; the explicit-downgrade alternative is a future
        // explicit switch).
        if self.acaf_fail_closed && self.acaf.is_none() {
            return Err(AgentLoopError::Assurance(
                "ACAF fail-closed is enabled but no signer client is \
                 configured (ORZ_ACAF_MANIFEST + ORZ_ACAF_KEYSTORE); \
                 refusing to start the run"
                    .to_string(),
            ));
        }
        let journal = host.journal();
        let mut writer = EventWriter::new(
            Some(journal),
            EventTrack::V02,
            run_id,
            run_manifest_sha256,
            next_sequence,
            previous_event_sha256,
            heartbeat.cloned(),
        );
        // F7 (2026-08-15): plan-epoch archive failures queued during the
        // builder phase get their audit events at run start — the first
        // moment a journal writer exists.
        self.flush_epoch_archive_write_failures(&mut writer).await?;
        let result = self
            .run_turn_inner(
                &mut writer,
                host,
                prompt,
                run_id,
                run_manifest_sha256,
                cancel,
                heartbeat,
                None,
                orientation,
                conversation,
            )
            .await;
        match result {
            Ok(response) => {
                journal.flush_async().await?;
                Ok((response, writer.seq(), writer.prev_hash()))
            }
            Err(e) => {
                // M4 (GAP-SUBAGENT-RUNTIME 2026-08-10): a cancelled run
                // closes every pending activation with a terminal close
                // record BEFORE the run terminal (ADR-0010 §4.4: cancel is a
                // terminal authority; the close records reference the same
                // run journal). `session_cancelled` is indistinguishable
                // from a user cancel on the current ACP path — mapped to
                // `user_cancelled` (registered decision).
                if matches!(e, AgentLoopError::Cancelled) {
                    self.close_all_activations(&mut writer, "user_cancelled")
                        .await;
                }
                // Terminal event — best effort; the journal must end on a
                // terminal event, never a mid-sequence orphan. A user cancel
                // records `run_cancelled` (already terminal, schema-valid);
                // everything else records `run_failed`. If the journal itself
                // is dead this also fails, and the original error is returned
                // regardless.
                let payload = match &e {
                    AgentLoopError::Cancelled => {
                        serde_json::json!({"reason": "user_cancelled"})
                    }
                    AgentLoopError::Degeneration(detail) => {
                        // OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010
                        // §14.33) + STALL-DEGENERATION-FAILFAST (2026-08-21,
                        // ADR-0010 §14.37 / 设计 §2.2.4): run_invalidated{
                        // status: degeneration}——detail 携带终止原因（族 +
                        // consecutive + round），显式标明、不吞错误。
                        serde_json::json!({
                            "status": "degeneration",
                            "detail": detail,
                        })
                    }
                    _ => serde_json::json!({"error": e.to_string()}),
                };
                let event = match &e {
                    AgentLoopError::Cancelled => EventType::RunCancelled,
                    AgentLoopError::Degeneration(_) => EventType::RunInvalidated,
                    _ => EventType::RunFailed,
                };
                let _ = writer.record(event, payload).await;
                let _ = journal.flush_async().await;
                Err(e)
            }
        }
    }

    /// Grill-mode turn (2026-08-08 write-placement slice, design §3): a
    /// full model↔tool round with a discard `EventWriter` — the session's
    /// history persists across turns via `history` (in/out: the complete
    /// conversation including tool rounds is written back), the host records
    /// the session to its grill JSONL, and the run journal is never touched.
    /// Read-only tool policy is enforced by the host's `PermissionBridge`
    /// (ReadOnly — "先探索代码库" is the protocol's core).
    pub async fn run_grill_turn(
        &self,
        host: &dyn LoopHost,
        history: &mut Vec<Message>,
        user_input: &str,
        template: Option<&str>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> Result<String, AgentLoopError> {
        // ACAF fail-closed D-15 (2026-08-16 review fix P2-I1): grill turns
        // are a full model↔tool round — fail-closed with an unconfigured
        // fabric refuses loudly here too (the ACP server's comment promises
        // exactly this). Read-only policy is defense-in-depth, not the
        // authorization gate; a future policy widening must not open an
        // unticketed channel.
        if self.acaf_fail_closed && self.acaf.is_none() {
            return Err(AgentLoopError::Assurance(
                "ACAF fail-closed is enabled but no signer client is \
                 configured (ORZ_ACAF_MANIFEST + ORZ_ACAF_KEYSTORE); \
                 refusing to start the grill turn"
                    .to_string(),
            ));
        }
        let mut writer = EventWriter::new(None, EventTrack::V02, "grill", "", 0, None, None);
        let mut turn = GrillTurn {
            history,
            user_input,
            template,
        };
        self.run_turn_inner(
            &mut writer,
            host,
            user_input,
            "grill",
            "",
            cancel,
            None,
            Some(&mut turn),
            // Grill turns never fire orientation (D9 — a grill question is
            // not a run-semantic; same reasoning as the counterexample skip).
            None,
            // Grill keeps its own history path (`GrillTurn.history`).
            None,
        )
        .await
    }

    /// The turn body — writes all events except the failure terminal.
    /// The caller (`run_turn`) owns the `EventWriter` and finalizes the chain.
    /// `cancel` is polled at cooperative checkpoints (Phase 3 slice #7);
    /// `heartbeat` (P1-1) forwards to the gateway for per-frame stamping.
    /// `grill` (2026-08-08): `Some` runs a grill-mode turn — the message
    /// history starts from the session's accumulated conversation (plus the
    /// once-injected template and the user input), the final-answer
    /// counterexample gate is skipped, and the full conversation is written
    /// back into `GrillTurn.history` on success.
    ///
    /// `orientation` (GAP-INQUIRY-SPLIT, 2026-08-09): session-level orientation
    /// state — `None` for grill turns and one-shot CLI runs (ADR-0010 §4.2:
    /// session-level 7-round counter; the state rides the host's session, not
    /// the per-prompt controller).
    /// `conversation` (GAP-CONVERSATION-RESTORE, 2026-08-10): session-level
    /// multi-prompt conversation — seeds the model context with the prior
    /// prompts' history (cloned; the caller keeps its copy so error paths
    /// preserve it) and receives the full conversation back on success.
    /// `None` (one-shot CLI, grill) keeps the single-prompt seed.
    #[allow(clippy::too_many_arguments)] // mirrors run_turn_with_guards + the grill turn
    async fn run_turn_inner(
        &self,
        writer: &mut EventWriter<'_>,
        host: &dyn LoopHost,
        prompt: &str,
        _run_id: &str,
        _run_manifest_sha256: &str,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
        mut grill: Option<&mut GrillTurn<'_>>,
        orientation: Option<&mut OrientationSessionState>,
        mut conversation: Option<&mut Vec<Message>>,
    ) -> Result<String, AgentLoopError> {
        // Review P3-5 (three-agent 2026-08-10): grill and conversation are
        // mutually exclusive by construction (grill keeps its own history
        // path; callers pass `None` for the other) — asserted in debug
        // builds so a future caller cannot silently lose one of them.
        debug_assert!(
            grill.is_none() || conversation.is_none(),
            "grill and conversation are mutually exclusive"
        );
        // IP2a: the denial circuit breaker is per-run — a fresh turn starts
        // clean (D-3: "连续拒绝 3 次/轮" — the window is one run).
        *self.denial_state.lock().unwrap() = DenialState::default();
        // GAP-DENIAL-POLICY-REVISION (2026-08-12): policy revision is also
        // per-run — a fresh run starts at 0 (same window as the breaker).
        self.policy_revision
            .store(0, std::sync::atomic::Ordering::SeqCst);
        // FUS-TOOL-PROBE P0-A step 5: the minimal previous-round map is
        // per-run too — a fresh run starts with no previous state (the
        // pre-run_started probe below seeds it; never persisted across
        // runs, design §8).
        *self.probe_state.lock().unwrap() = crate::tool_probe::MinimalProbeMap::default();
        // PLAN-FIRST 阶段 B 审查收口 (2026-08-16): 注册板块探针源同样随
        // run 复位——与 `probe_state` 同纪律，本 run 首个有探针轮重新记录，
        // 不跨 run 沿用（此前依赖「首个有探针轮必先覆盖」才能保证无泄漏）。
        self.reset_console_probe_source();
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 双模式状态随
        // run 复位——每 run 起始 console 默认、连败清零、询问标记清空、
        // direct 证据面清空（模式为 run 级状态，§7.1）。
        self.reset_console_mode();
        // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the final-answer
        // citation verifier's evidence is per-run — the main lane's read
        // evidence and the committed retrieval ledgers start empty.
        *self.main_evidence.lock().unwrap() = Vec::new();
        *self.run_source_ledgers.lock().unwrap() = Vec::new();
        *self.next_source_seq.lock().unwrap() = 0;
        // ACAF Slice 1 (ADR-0011 §4.2): the run's task goal is the ticket
        // goal binding (check 4) — pinned before any control event can fire.
        self.set_goal_digest(prompt);
        // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.2)：per-run 模型健康状态隔离——长驻进程（ACP server）跨
        // run 共享同一 transport，退化计数与会话 thinking 档位若挂在
        // transport 上会跨任务泄漏（新任务继承上个任务的档位/计数）。
        // 本 run 用 `for_new_run` 换新实例（设计原话「仅 run 边界重置=
        // 新 transport」的正式路径实现）；主 agent 与两个检索子代理共享
        // 同一 run 实例（同一 run 内计数/档位一致）。
        let run_gateway = self.main_agent.gateway.for_new_run();
        let run_main_agent = MainAgent::new(run_gateway.clone());

        // 1. tool_availability_check — the v0.2 single probe face snapshot
        // BEFORE run_started (Python conformance: the probe must precede
        // run_started).
        //
        // FUS-TOOL-PROBE P0-A-2 (2026-08-13, design §3/§7 v0.2): the event
        // reports the ALL-work-tool mechanical probe partition —
        // complete / incomplete(reason) — with neutral reasons only; the
        // retrieval lane never appears. P0-A step 5: this run-start event
        // is the initial journal; the loop re-probes before EVERY model
        // request and emits `tool_availability_check` again ONLY on a flip
        // (minimal previous-round map). The snapshot seeds that map; the
        // list projection (design §4) runs per-round inside the loop
        // (`project_main_agent_tool_defs`) — the `tool_defs` built below is
        // the BASE list (registry + main-only additions + mode projection),
        // not yet probe-filtered. The call-time permission gate remains
        // the final backstop (design invariant 2).
        let mut tool_defs: Vec<ToolDef> = host.tools_registry().list().into_iter().collect();
        // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24, ADR-0010 §14.39 /
        // FUS-READ-ANCHOR-WRITE-GUARD)：direct 面 read-anchor 写前核证——
        // `search_replace` 声明补可选 `expected_anchor`（read_file 返回的
        // {size, mtime, sha256}），运行时执行前机械核证（见
        // `run_host_tool_with_timeout`）；目标不存在（新建）时无需携带。
        if let Some(def) = tool_defs.iter_mut().find(|t| t.name == "search_replace")
            && let Some(props) = def
                .parameters
                .as_object_mut()
                .and_then(|p| p.get_mut("properties"))
                .and_then(serde_json::Value::as_object_mut)
        {
            props.insert(
                "expected_anchor".to_string(),
                serde_json::json!({
                    "type": "object",
                    "description": "read_file 返回的内容锚点（sha256/size/mtime）——写前机械核证期望值；不匹配时拒绝并要求重读后重试。新建文件（目标不存在）时无需携带。",
                    "properties": {
                        "size": { "type": "integer", "minimum": 0 },
                        "mtime": {
                            "anyOf": [
                                { "type": "integer", "minimum": 0 },
                                { "type": "null" },
                            ]
                        },
                        "sha256": { "type": "string", "pattern": "^[0-9a-f]{64}$" },
                    },
                }),
            );
        }
        // P0-A steps 3-5 / P0-A-2: `run_tests` is a work tool whose declaration is
        // decided by the per-round probe snapshot (runner existence), not
        // by a direct host call here; the per-round list projection runs
        // inside the loop. No conditional declaration remains in this block.
        // 2026-08-08 blackboard partition (A3): `blackboard_read` is the
        // model's ON-DEMAND window into the blackboard — declared whenever
        // the loop runs (the blackboard is always live). The model pulls a
        // partition (plan / edits / tool_actions / exec / actions / session /
        // internal_ret / external_ret / entities) when it needs to look
        // back; no full
        // render is ever injected uninvited (zero dilution when not called).
        // ReadOnly risk class → auto-allows under every policy
        // (Interactive/ReadOnly/Benchmark).
        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1 边界项):
        // `compaction_whitelist_add` 接口封存——不再向模型声明；调用由
        // `run_host_tool_with_timeout` 的 sealed-tool 窄门结构化拒绝
        // （代码与写盘链路保留为独立模块，A/B 观察后可经配置恢复）。
        if !tool_defs.iter().any(|t| t.name == "blackboard_read") {
            tool_defs.push(ToolDef {
                name: "blackboard_read".to_string(),
                description: "Read a blackboard partition. `section` is one \
                     of: entities (R2 half-assistant entity states — \
                     process/file/environment stable views with anchors and \
                     availability; 黑板=框架状态区，分区保存实体状态/检索结果/\
                     审计留痕), plan (current goal + step statuses; each step line \
                     starts with its id: `- [状态] <step_id>: <目标> ...` — \
                     use that id for the step_id binding when writing console \
                     orders), edits (file-edit records: file, line-range \
                     delta, timestamp), tool_actions (executed tool calls \
                     folded by category read/edit/terminal/retrieval with \
                     timestamps), exec (tool results — the full accumulated \
                     log; read_file still works for files), actions (P0-C \
                     console: current registration board buttons, the pending \
                     action-bar order and recent result receipts), session \
                     (live tool-round budget — used/remaining — plus the \
                     resident 状态行; read it on demand to gauge how many \
                     tool rounds are left; the controller enforces the cap \
                     mechanically either way), internal_ret / external_ret \
                     (live retrieval partitions — the subagent's full result \
                     text, parsed entries and source ledger; read them when a \
                     web_search / web_fetch / retrieve_project_docs dispatch \
                     returns a pointer summary instead of inline text). \
                     Optional `since_timestamp` (RFC 3339, e.g. the timestamp \
                     this tool returned earlier) filters the edits / tool_actions \
                     entries to those at or after that time. Optional `epoch` \
                     (integer) reads that plan-epoch ARCHIVE instead of the \
                     live view — use it to recall a previous task's plan/edits \
                     after a new plan epoch rotated the blackboard (retrieval \
                     partitions are live-only and reject `epoch`). Optional \
                     `receipt_id` (an order_id from the actions results board, \
                     e.g. ORD-000012) point-reads ONE result receipt's full \
                     response/error content (bounded ≤8K chars) that the slim \
                     board hides — only valid with `section=actions`; combine \
                     with `epoch` to read archived receipts; \
                     `since_timestamp` is ignored when `receipt_id` is \
                     present. Call this when you need to recall what changed \
                     or what you did earlier — it costs nothing when you do \
                     not call it."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "section": {
                            "type": "string",
                            "enum": [
                                "plan",
                                "edits",
                                "tool_actions",
                                "exec",
                                "actions",
                                "session",
                                "internal_ret",
                                "external_ret",
                                "entities",
                            ],
                        },
                        "since_timestamp": {"type": "string"},
                        "epoch": {
                            "type": "integer",
                            "minimum": 1,
                            "description": "Optional plan-epoch archive to read (cross-epoch look-back).",
                        },
                        "receipt_id": {
                            "type": "string",
                            "minLength": 1,
                            "description": "Optional single-receipt point-read: an order_id from the actions results board (e.g. ORD-000012). Returns that receipt's full response/error content, bounded at 8000 chars. Only valid with section=actions; combine with epoch to point-read an archived receipt; since_timestamp is ignored when present.",
                        },
                    },
                    "required": ["section"],
                }),
            });
        }
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD
        // _DESIGN §3-§5): `plan_write` — 首轮计划轮的唯一写面（结构化分步
        // 计划落黑板 plan epoch）；首轮之外保留用于计划修订。ReadOnly 类
        // （只写内存黑板计划槽，无外部副作用）→ 所有策略自动放行。
        // 2026-08-16 审查收口（P3-4）：声明面随 plan_first_enabled 收敛——
        // 关闭态/grill 不声明，调用时另由 handler 拒绝（不旋转黑板）。
        if self.plan_first_enabled && !tool_defs.iter().any(|t| t.name == "plan_write") {
            tool_defs.push(ToolDef {
                name: "plan_write".to_string(),
                description: "Write the structured step-by-step plan (PLAN-FIRST). \
                     `plan` is an object with plan_id, goal and an ordered steps \
                     array; every step carries id, goal, actions (each \
                     {step_id, do, with}), acceptance and evidence. The plan \
                     lands in the blackboard plan section (plan epoch). Steps \
                     are EXECUTION ORDER markers: they only constrain issuing \
                     orders in order — a step marked done means its orders \
                     executed, not that its goal is achieved (the delivery \
                     gate arbitrates the goal). The LAST step must be the fixed \
                     terminal step (递交/完成) with id `deliver` or `submit`; \
                     it never auto-advances on ordinary orders and is advanced \
                     only via the `submit` delivery action. On the FIRST round \
                     of a run this is the only write tool available; an invalid \
                     plan is rejected with mechanical validation errors (one \
                     refill opportunity, then degrade)."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "plan": {
                            "type": "object",
                            "description": "Structured plan: plan_id, goal, ordered steps[].",
                            "properties": {
                                "plan_id": {
                                    "type": "string",
                                    "minLength": 1,
                                    "maxLength": 128,
                                },
                                "goal": {
                                    "type": "string",
                                    "minLength": 1,
                                    "maxLength": 2000,
                                },
                                "steps": {
                                    "type": "array",
                                    "minItems": 1,
                                    "maxItems": 32,
                                    "items": {
                                        "type": "object",
                                        "required": [
                                            "id",
                                            "goal",
                                            "actions",
                                            "acceptance",
                                            "evidence",
                                        ],
                                        "properties": {
                                            "id": {
                                                "type": "string",
                                                "minLength": 1,
                                                "maxLength": 64,
                                            },
                                            "goal": {
                                                "type": "string",
                                                "minLength": 1,
                                                "maxLength": 2000,
                                            },
                                            "actions": {
                                                "type": "array",
                                                "minItems": 1,
                                                "maxItems": 8,
                                                "items": {
                                                    "type": "object",
                                                    "required": ["step_id", "do", "with"],
                                                    "properties": {
                                                        "step_id": {
                                                            "type": "string",
                                                            "minLength": 1,
                                                            "maxLength": 64,
                                                        },
                                                        "do": {
                                                            "type": "string",
                                                            "minLength": 1,
                                                            "maxLength": 128,
                                                        },
                                                        "with": { "type": "object" },
                                                    },
                                                },
                                            },
                                            "acceptance": {
                                                "type": "string",
                                                "minLength": 1,
                                                "maxLength": 2000,
                                            },
                                            "evidence": {
                                                "type": "array",
                                                "maxItems": 16,
                                                "items": {
                                                    "type": "string",
                                                    "minLength": 1,
                                                    "maxLength": 500,
                                                },
                                            },
                                            "status": { "const": "pending" },
                                        },
                                    },
                                },
                            },
                            "required": ["plan_id", "goal", "steps"],
                        },
                    },
                    "required": ["plan"],
                }),
            });
        }
        // AGENT-DELIVERY-FLOW (2026-08-23, ADR-0010 §14.35 第 19 项 / 设计
        // §2.2) + THIN-HARNESS-REDESIGN-V2 §9.3 (2026-08-29): `submit` ——
        // 显式递交/交付状态展示路径（无参，console 默认态主车道；无 plan
        // 会话同样放行，降级为纯状态展示，非硬门）。第一次调用机械计算
        // 交付状态（工作区变更清单，过滤 .gsa/缓存目录，上限 20 + 计数行）
        // 渲染进黑板 plan 视图；模型核查后同动作再触发一次确认。终答仍
        // 只由模型自发（反例门 + 机械审计报告在终答前承接核对）。
        if self.console_default_enabled && !tool_defs.iter().any(|t| t.name == "submit") {
            tool_defs.push(ToolDef {
                name: "submit".to_string(),
                description: "Request/confirm delivery (AGENT-DELIVERY-FLOW). \
                     Call `submit` (no arguments) once to have the harness \
                     mechanically compute and render the delivery status \
                     (workspace changes, filtered, ≤20 entries) into the \
                     blackboard plan view; review it, then call `submit` \
                     again to confirm and advance into the final-answer \
                     flow. Informational, not a hard gate — submission \
                     requires no plan and no step state."
                    .to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                }),
            });
        }
        // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1): `retrieval_disposition`
        // 与 `retrieve_project_docs` 从主代理声明面删除——检索子代理结果
        // 改为每次调用即闭环（§4.4；见 retrieval/dispatch.rs 的
        // run_retrieval_subagent），内部检索子代理随主面下线（主代理对
        // 项目文档直接用 grep/read/search_replace）。
        // 底层 handler/relay 路由保留为休眠模块（审查 P2-2 收口：正常主面
        // 不声明，但并非「不可达」——跨 run 恢复的 AwaitingDisposition
        // 激活仍经 retrieval_disposition/retrieve_project_docs 延续，见
        // retrieval/dispatch.rs 的 run_retrieval_subagent 与
        // retrieval/disposition.rs 的 handle_parent_disposition；幻觉调用
        // 内部 lane 会产生一次子代理运行（成本已评估），R3 裁决彻底封死/
        // 物理删除）。
        // GAP-RETRIEVAL-TOOLS (2026-08-10): mode=off removes the retrieval
        // dispatch family from the model-visible declarations (ADR-0010
        // §3.7.1 — unauthenticated retrieval starts from off; §3.5.2
        // name-level refusal: the model never sees tools that cannot run).
        // `retrieval_disposition` STAYS — disposing an already-pending
        // (possibly cross-run restored) activation is a legal off-mode
        // action; the relay still refuses the family at dispatch time
        // (belt and braces).
        if self.retrieval_mode == RetrievalMode::Off {
            // H1 (review 2026-08-10): `project_doc_index` is a retrieval
            // tool routed through the host lane — the off projection hides
            // it too (the dispatch gate in run_host_tool is belt and
            // braces).
            tool_defs.retain(|t| {
                !crate::relay::is_retrieval_dispatch_name(&t.name)
                    && !crate::relay::is_retrieval_mode_gated_host_tool(&t.name)
            });
        }
        // RETRIEVAL-SUBAGENT-WIRING 审查处理 (2026-08-25)：工具面跟随
        // 模式 A 定档——local_browser 隐藏 web 族、framework_fallback
        // 隐藏 browser_read（主面 base 投影；子代理父面继承同一规则，
        // 外部 lane 的 browser_read 恢复另行按模式门控）。
        Self::apply_retrieval_surface_projection(&mut tool_defs, self.retrieval_mode);
        // GAP-RETRIEVAL-TOOLS (2026-08-10): journal the session bootstrap
        // mode transition (ADR-0010 §3.7.1 — every transition carries
        // old/new/authority/reason; never implicit). Written BEFORE
        // tool_availability_check so the availability gate reflects the
        // mode's tool projection. Cleared on journal success (the
        // acp_server sidecar write-back flips the session flag).
        // M4 (review 2026-08-10): any explicit mode change journals —
        // including a change TO off (that is a transition like any other);
        // old_mode is the real persisted value, never a hardcoded "off".
        if self
            .bootstrap_transition_pending
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            let suffix = writer
                .run_id()
                .strip_prefix("RUN-")
                .unwrap_or(writer.run_id());
            let capability_status = if self.retrieval_mode == RetrievalMode::Off {
                // Schema allOf: new_mode=off ⇒ capability_status must be null.
                serde_json::Value::Null
            } else {
                serde_json::json!(self.retrieval_capability.status_str())
            };
            writer
                .record(
                    EventType::RetrievalModeTransition,
                    serde_json::json!({
                        "transition_id": format!("MODETRANS-{}-{:04}", suffix, writer.seq()),
                        "session_id": self.session_id.clone().unwrap_or_else(|| writer.run_id().to_string()),
                        "old_mode": self
                            .previous_retrieval_mode
                            .as_ref()
                            .map_or("off", |m| m.as_str()),
                        "new_mode": self.retrieval_mode.as_str(),
                        "authority": self
                            .transition_authority
                            .as_ref()
                            .map_or("session_bootstrap", |(a, _)| a.as_str()),
                        "reason_code": self
                            .transition_authority
                            .as_ref()
                            .map_or("session_default", |(_, r)| r.as_str()),
                        "capability_status": capability_status,
                    }),
                )
                .await?;
            self.bootstrap_transition_pending
                .store(false, std::sync::atomic::Ordering::Relaxed);
        }
        // GAP-RETRIEVAL-TOOLS (2026-08-10): journal the sidecar-restored
        // activations (ADR-0010 §3.3 — the assessment is declared known so a
        // parent disposition may close/continue across runs). After the mode
        // transition, before the availability gate.
        self.journal_activation_restores(writer).await?;
        let probe_context = crate::tool_probe::ProbeContext {
            cwd: host.session_cwd(),
            policy: host.tool_policy(),
            test_runner_present: host.test_runner().is_some(),
            interactive_user: host.interactive_user(),
            goal_context_present: self.goal_context_present(),
            pending_retrieval_activation: self.has_live_activation(),
            terminal_available: host.terminal_available(),
            lsp_configured: host.lsp_configured(),
            memory_enabled: host.memory_enabled(),
            image_backend_configured: host.image_backend_configured(),
            video_backend_configured: host.video_backend_configured(),
            mcp_registry_available: host.mcp_registry_available(),
        };
        let probe_snapshot = crate::tool_probe::probe_work_tools(&probe_context);
        // P0-A-2 (design §4/§8 v0.2): the initial snapshot journals the
        // pre-run_started event and seeds the minimal previous-round map.
        // The per-round list projection (探针完整集 ∩ 会话声明集 + 非工作工具)
        // runs inside the loop before every model request — `tool_defs`
        // passed below is the unfiltered BASE list.
        writer
            .record(
                EventType::ToolAvailabilityCheck,
                Self::tool_availability_payload(&probe_snapshot),
            )
            .await?;
        self.probe_state_seed(&probe_snapshot);

        // 2. run_started + prompt_submitted
        writer
            .record(EventType::RunStarted, serde_json::json!({"prompt": prompt}))
            .await?;
        writer
            .record(
                EventType::PromptSubmitted,
                serde_json::json!({
                    "prompt": prompt,
                    "character_count": prompt.chars().count(),
                }),
            )
            .await?;

        // 3. (GAP-INQUIRY-SPLIT, 2026-08-09): the per-turn orientation event
        // is GONE — the orientation producer is now the session-level 7-round
        // state machine (§4.2); the `orientation_checkpoint` event fires only
        // when an orientation is actually injected (see `maybe_fire_orientation`
        // in the model↔tool loop below).

        // 4. model ↔ tool loop. Grill mode (2026-08-08) starts from the
        // session's accumulated conversation: history + once-injected
        // template + the user's answer to the previous question.
        let mut messages: Vec<Message> = match &mut grill {
            Some(g) => {
                let mut m = g.history.clone();
                if let Some(tpl) = g.template {
                    m.push(Message {
                        role: Role::User,
                        content: tpl.to_string(),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                }
                m.push(Message {
                    role: Role::User,
                    content: g.user_input.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                m
            }
            // GAP-CONVERSATION-RESTORE (2026-08-10): the session's prior
            // conversation seeds the model context (cloned — the caller keeps
            // its copy so an error path leaves the pre-run history intact);
            // `None` reproduces the historical single-prompt seed byte-for-byte.
            None => {
                let mut m = conversation
                    .as_deref_mut()
                    .map(|conv| conv.clone())
                    .unwrap_or_default();
                m.push(Message {
                    role: Role::User,
                    content: prompt.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                m
            }
        };
        // D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6):
        // a restored conversation may exceed the provider window before the
        // first request. Estimate it and, when over the conservative
        // recovery threshold (200K), mechanically drop whole OLD rounds
        // toward the recovery target — preamble/whitelist and the newest
        // rounds stay verbatim. The FULL sidecar is copied into the run
        // journal as the audit copy (the sidecar file itself is not modified
        // here), a recovery marker is inserted, and the truncation is
        // journaled. Grill turns are excluded (grill keeps its own history
        // path; conversation and grill are mutually exclusive).
        if conversation.is_some() {
            let before_estimate = estimate_messages_tokens(&messages);
            let cfg = self.context_compact;
            if before_estimate > cfg.recovery_trigger_tokens {
                let restored_full = conversation.as_deref().cloned().unwrap_or_default();
                let stats = compact_messages(&mut messages, cfg.recovery_target_tokens);
                if stats.rounds_dropped > 0 {
                    let audit_path = host
                        .journal()
                        .journal_dir()
                        .join("recovery-conversation-full.json");
                    if let Ok(payload) = serde_json::to_string_pretty(&restored_full) {
                        if let Some(parent) = audit_path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(&audit_path, payload);
                    }
                    let marker = crate::prompt::recovery_truncation_marker(
                        stats.rounds_dropped,
                        before_estimate,
                        stats.estimated_tokens_after,
                        &audit_path.display().to_string(),
                    );
                    messages.insert(
                        stats.marker_index,
                        Message {
                            role: Role::User,
                            content: marker,
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        },
                    );
                    writer
                        .record(
                            EventType::ContextRecoveryTruncated,
                            serde_json::json!({
                                "before_estimate_tokens": before_estimate,
                                "target_tokens": cfg.recovery_target_tokens,
                                "after_estimate_tokens": stats.estimated_tokens_after,
                                "rounds_dropped": stats.rounds_dropped,
                                "messages_dropped": stats.messages_dropped,
                                "messages_kept": stats.messages_kept + 1,
                                "audit_path": audit_path.display().to_string(),
                            }),
                        )
                        .await?;
                }
            }
        }
        // GAP-SUBAGENT-RUNTIME (2026-08-10): the model↔tool loop body is
        // the shared `run_agent_loop` (agent_loop.rs) — the main agent and
        // both retrieval subagents run the SAME loop; the main profile
        // preserves the pre-split event sequences byte-for-byte (locked by
        // EXPECTED_SEQUENCES_V02 + conformance capture).
        let profile = if grill.is_some() {
            LoopProfile::grill(self.max_tool_rounds)
        } else {
            LoopProfile::main(self.max_tool_rounds)
        };
        let outcome = run_agent_loop(
            &SharedLoopServices {
                blackboard: &self.blackboard,
                denial_state: &self.denial_state,
                pacing_rounds: &self.pacing_rounds,
                context_compact: &self.context_compact,
                dc_state: &self.dc_state,
                // Main lane: collect the lane's own read evidence — the
                // observation-time source for final-answer path:line
                // citation binding (P0-B step 5, ADR-0010 §3.7.9).
                evidence: Some(&self.main_evidence),
                policy_revision: &self.policy_revision,
                max_inject_tokens_per_round: self.max_inject_tokens_per_round,
                blackboard_archive_dir: self.blackboard_archive_dir(),
            },
            self,
            writer,
            host,
            &run_main_agent,
            &profile,
            prompt,
            &tool_defs,
            &mut messages,
            orientation,
            cancel,
            heartbeat,
            &run_gateway,
        )
        .await?;
        let LoopOutcome {
            last_text,
            tool_rounds,
            rounds_since_compact,
            mut fold_state,
            ..
        } = outcome;

        // 5. terminal — the runtime stagnation guard is RETIRED (2026-08-22,
        // user adjudication): generation-time output-health detection
        // (rolling-hash repetition + stall budget + fail-fast) covers the
        // actual degeneration surface; a completed loop turn finishes
        // "completed" unless a hard error path (degeneration / wallclock)
        // invalidated it earlier.
        let (terminal_event, status) = (EventType::RunFinished, "completed");
        // P0-D review fix (2026-08-14, ADR-0010 v1.14): end-of-session
        // compaction — 治本 for restore. A successful run compacts its
        // conversation BEFORE the terminal event and the sidecar write-back,
        // pinning the summary marker into the persisted conversation (D3-1
        // retains it on restore); the D2-2 recovery pre-check remains the
        // fallback for older sidecars that never ran this path. Grill turns
        // are excluded (they keep their own one-shot history). The summary
        // call is forced (terminal housekeeping, not a mid-task cost gate).
        if conversation.is_some() {
            let estimate = estimate_messages_tokens(&messages);
            if estimate > self.context_compact.session_end_trigger_tokens {
                let svc = SharedLoopServices {
                    blackboard: &self.blackboard,
                    denial_state: &self.denial_state,
                    pacing_rounds: &self.pacing_rounds,
                    context_compact: &self.context_compact,
                    dc_state: &self.dc_state,
                    evidence: Some(&self.main_evidence),
                    policy_revision: &self.policy_revision,
                    max_inject_tokens_per_round: self.max_inject_tokens_per_round,
                    blackboard_archive_dir: self.blackboard_archive_dir(),
                };
                let _ = run_template_compact(
                    &svc,
                    writer,
                    host,
                    &mut messages,
                    estimate,
                    "session_end",
                    true,
                    false,
                    rounds_since_compact,
                    self.context_compact.recent_tail_rounds,
                    // FUS-LEDGER-FOLD-STATE external-file design
                    // (2026-08-18, ADR-0010 §14.28 审查修复): 主车道收尾
                    // 压缩的 marker 携带外挂台账路径提示（内部再按
                    // 文件存在/已折叠过滤）。
                    Some(&crate::action_ledger::ledger_file_path(&host.session_cwd())),
                    &mut fold_state,
                )
                .await?;
            }
        }
        writer
            .record(
                terminal_event,
                serde_json::json!({
                    "status": status,
                    "turn_count": 1,
                    "tool_rounds": tool_rounds,
                }),
            )
            .await?;

        // Grill mode (2026-08-08): persist the full conversation (incl. tool
        // rounds) as the next turn's history — the session is multi-turn.
        // GAP-CONVERSATION-RESTORE (2026-08-10): the main-lane conversation is
        // written back the same way (success-only — a failed run's partial
        // messages never enter the conversation; the journal is the failure
        // evidence). Error paths skip this: a failed turn keeps the previous
        // history.
        if let Some(g) = &mut grill {
            *g.history = messages;
        } else if let Some(conv) = conversation {
            // GAP-CONVERSATION-RESTORE: mechanical injection blocks
            // (counterexample gate / budget / breaker / edit push /
            // orientation) are runtime scaffolding, not dialogue — filtered
            // so the persisted conversation stays clean dialogue (a restored
            // prompt would otherwise replay stale "once-only" gates).
            //
            // Review P2-1 (three-agent 2026-08-10): the filter only drops
            // USER-role injection blocks (every injection point injects
            // `Role::User` — verified across agent_loop.rs) — a Tool message
            // whose CONTENT happens to match a prefix is kept (deleting it
            // would orphan the assistant's `tool_calls` declaration; the
            // replayed conversation would 400 forever). Index 0 (the seed's
            // first dialogue message) is never dropped.
            //
            // Review D2-4 (three-agent 2026-08-10): only a COMPLETED loop
            // reaches this write-back — hard error paths (degeneration /
            // wallclock) invalidate the run before the sidecar write, so a
            // failed run's messages never enter the conversation. The
            // stagnation guard that previously gated this is retired
            // (2026-08-22).
            *conv = messages
                .into_iter()
                .enumerate()
                .filter(|(i, m)| {
                    // D3-1 (2026-08-14, ADR-0010 v1.10): the compaction
                    // marker and the whitelist block are restore-retained
                    // (a restored prompt must see the compression notice
                    // and the task facts); every other mechanical
                    // injected block stays filtered.
                    *i == 0
                        || !(m.role == Role::User
                            && is_injected_block_text(&m.content)
                            && !is_restore_retained_block(&m.content))
                })
                .map(|(_, m)| m)
                .collect();
        }

        Ok(last_text.unwrap_or_default())
    }

    /// ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): the
    /// journal evidence identities available to the forced-template
    /// `progress_evidence` cross-check — committed retrieval ledger ids /
    /// refs plus the main lane's own evidence identities. Non-blocking
    /// mitigation (强制表达，不验证诚实): the checkpoint response event
    /// records which identities matched and which did not.
    pub(crate) fn checkpoint_source_identities(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for ledger in self.run_source_ledgers.lock().unwrap().iter() {
            let Some(entries) = ledger.as_array() else {
                continue;
            };
            for entry in entries {
                for key in ["source_id", "source_url_or_ref", "source_title"] {
                    if let Some(value) = entry.get(key).and_then(serde_json::Value::as_str) {
                        ids.push(value.to_string());
                    }
                }
            }
        }
        ids
    }

    /// Default max tokens for the main agent (configurable later).
    /// D-6 (FIX_PLAN 2026-08-06): single-round output budget
    /// (OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20, ADR-0010 §14.35:
    /// 32K → 256K，回落档 128K；the transport's `ModelConfig::max_tokens`
    /// is the cap; this request-level value is min-capped by it — equal
    /// here so the full budget is available).
    /// Review F3 (2026-08-10): the retrieval lane reads the SAME constant
    /// (`agent_loop::REQUEST_MAX_TOKENS`) — one source for all three agents.
    pub(crate) fn main_agent_max_tokens(&self) -> u32 {
        crate::agent_loop::REQUEST_MAX_TOKENS
    }

    /// GAP-INQUIRY-SPLIT (2026-08-09) — the ORIENTATION producer (ADR-0010
    /// §4.2): when the lane's session-level count reached the 7-round
    /// threshold, journal the v0.2 `orientation_checkpoint` event (10-field
    /// payload — `inquiry_family=neutral` + `inquiry_kind=orientation_checkpoint`
    /// consts cross-checked by the verifier) and inject the orientation block
    /// as a User message so the next generate answers it. `None` orientation
    /// state is a no-op (grill / one-shot CLI).
    /// THIN-HARNESS-REDESIGN-V2 §9.2 (2026-08-29 用户裁决): orientation
    /// 软门——主车道（main/grill）fire **延迟 commit**：pending 轮消费时
    /// 提交（纯文本回答被消费、loop 明确续跑；工具调用照常执行，不设
    /// 模板校验与工具禁令），终答仍只由模型自发。检索车道保持旧
    /// fire-and-continue（fire 即 commit、无 pending 轮——检索结果文本
    /// 不被消费截留）。`force_template_round` 为 2026-08-14 强制模板轮
    /// 的休眠参数（恒 false、不启用；若未来恢复硬门，需同时在 pending
    /// 消费路径恢复模板校验与工具禁令——2026-08-29 软门消费路径已将其
    /// 移除）。DC 的强制模板轮由 `maybe_fire_dc` 独立承载。
    /// Fires at most once per call — commit-then-reset guarantees the two
    /// injection points (post-tool-batch gap + loop-top) never double-fire.
    /// Review P2-2 (2026-08-10): build → journal → inject → COMMIT — a
    /// journal-write failure propagates before the counter is reset, so a
    /// failed run never persists a reset-but-never-fired counter.
    pub(crate) async fn maybe_fire_orientation(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        orientation: Option<&mut OrientationSessionState>,
        role: AgentRole,
        injection_position: &str,
        force_template_round: bool,
    ) -> Result<Option<OrientationFireRecord>, AgentLoopError> {
        let Some(state) = orientation else {
            return Ok(None);
        };
        let run_id = writer.run_id().to_string();
        let Some(rec) = state.build_fire_record(role, &run_id, injection_position) else {
            return Ok(None);
        };
        // ACAF Slice 1 (ADR-0011 §4.2/§4.6): an orientation fire is a control
        // event — ticket it first (shadow mode: rejected tickets journal
        // `control_ticket_rejected` but the orientation still fires;
        // fail-closed 2026-08-13: a Blocked gate skips the fire — no
        // injection, no checkpoint record, no commit).
        let gate = self
            .acaf_control_event(
                writer,
                TicketKind::OrientationV1,
                None,
                &serde_json::json!({ "agent_role": role.as_str() }),
            )
            .await?;
        if let TicketGate::Blocked { .. } = &gate {
            return Ok(None);
        }
        writer
            .record(
                EventType::OrientationCheckpoint,
                serde_json::json!({
                    "checkpoint_id": rec.checkpoint_id,
                    "inquiry_family": rec.inquiry_family,
                    "inquiry_kind": rec.inquiry_kind,
                    "agent_role": rec.agent_role.as_str(),
                    "session_id": rec.session_id,
                    "trigger": rec.trigger,
                    "completed_turns_since_orientation": rec.completed_turns_since_orientation,
                    "step_index": rec.step_index,
                    "message_block": rec.message_block,
                    "injection_position": rec.injection_position,
                }),
            )
            .await?;
        // 软门/强制模板模式：延迟 commit，pending 轮消费时提交；检索车道
        // 维持 fire-and-continue（fire 即 commit）。reset 不得在失败写入
        // 后残留（review P2-2）——两种模式的 commit 都只发生在事件已
        // journaled 之后。
        // 固有边界（2026-08-29 审查收口）：延迟 commit 下，fire 与
        // pending 消费之间若硬中断（传输错误/取消/panic），journal 留有
        // 一条已 fire 未消费的 orientation_checkpoint 事件且计数不提交
        // ——会话计数仍 ≥ 阈值，下次 run 在 loop-top 首轮重触发（到期
        // 方向检查不丢失，符合 recovery-resumes-counting 语义）；孤儿
        // 事件由运行终止事件在审计链中解释，接受现状、不补机制。
        let deferred = force_template_round || matches!(role, AgentRole::Main);
        if !deferred {
            state.commit_fire(role, &rec);
        }
        messages.push(Message {
            role: Role::User,
            content: rec.message_block.clone(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        Ok(deferred.then_some(rec))
    }

    /// ACAF Slice 2 fail-closed (2026-08-13): surface a ticket refusal as a
    /// no-ToolStarted tool error (the same shape as the retrieval-mode
    /// refusals) — the `control_ticket_rejected` security event is already
    /// in the journal; the tool itself never starts.
    pub(crate) async fn refuse_ticketed_tool(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tc: &ToolCall,
        gate: &TicketGate,
        probe_writeback: bool,
    ) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
        let TicketGate::Blocked { code, detail } = gate else {
            unreachable!("refuse_ticketed_tool called with Proceed");
        };
        let msg = format!(
            "ACAF ticket refused for '{}' — {} ({}); the action was not executed.",
            tc.name,
            detail,
            code.as_str(),
        );
        writer
            .record(
                EventType::ToolCompleted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "exit_code": 1,
                    "status": "error",
                    "error": format!("control_ticket_rejected:{}", code.as_str()),
                    "policy_denial": {
                        "source": "acaf",
                        "code": format!("control_ticket_rejected:{}", code.as_str()),
                        "reason": detail,
                    },
                }),
            )
            .await?;
        // P0-A step 5 (design §5): 调用即探针 — the refused work-tool call
        // writes back into the minimal previous-round map (search_replace /
        // run_tests carry action tickets under fail-closed). Main lane only
        // — retrieval lanes never write the main probe map (review fix
        // 2026-08-13).
        self.maybe_note_probe_call_failure(probe_writeback, &tc.name);
        messages.push(Message {
            role: Role::Tool,
            content: msg.clone(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        Ok((
            ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                policy_denial: Some(PolicyDenial {
                    source: PolicyDenialSource::Acaf,
                    code: format!("control_ticket_rejected:{}", code.as_str()),
                    reason: detail.clone(),
                }),
                timed_out: false,
                ..Default::default()
            },
            None,
        ))
    }
}

impl Default for AgentLoopController {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash-chained event writer — owns the journal sequence state within a turn.
/// `pub(crate)` (GAP-SUBAGENT-RUNTIME 2026-08-10): the shared loop in
/// `agent_loop.rs` records through it.
pub(crate) struct EventWriter<'a> {
    /// `None` = grill mode (2026-08-08): the turn runs the full model↔tool
    /// loop but records nothing — the grill session's conversation is
    /// persisted by the host to `{cwd}/.gsa/grill/<session>.jsonl`, never
    /// into a run journal (run = single-run integrity unit).
    journal: Option<&'a JournalRecorder>,
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：buffered 模式——
    /// `record` 追加到此（不写链、不打心跳），调用方按序 drain 后经真实
    /// writer 重放（同轮读类并行批次的提交阶段）。
    buffer: Option<Vec<(EventType, serde_json::Value)>>,
    /// GAP-INQUIRY-SPLIT (2026-08-09): the journal track — production writes
    /// `V02` (homogeneous chain, §11.6.2); `V01` is replay-only.
    track: EventTrack,
    run_id: String,
    manifest_sha256: String,
    seq: u64,
    prev_hash: Option<String>,
    /// P1-1 (2026-08-08 stall guards): stamped on every recorded event —
    /// journaled activity keeps the stall watchdog armed.
    heartbeat: Option<crate::gateway::model::ActivityClock>,
}

impl EventWriter<'_> {
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): the run's journal directory
    /// (`None` in grill discard mode) — structured-result artifacts land
    /// under `{dir}/retrieval-results/`.
    pub(crate) fn journal_dir(&self) -> Option<std::path::PathBuf> {
        self.journal.map(|j| j.journal_dir().to_path_buf())
    }
}

/// GAP-SUBAGENT-RUNTIME M5 (2026-08-10): a discard-mode writer (grill
/// semantics — `journal: None`) for unit tests that exercise producer
/// logic without a journal.
#[cfg(test)]
pub(crate) fn discard_event_writer(run_id: &str) -> EventWriter<'static> {
    EventWriter::new(None, EventTrack::V02, run_id, "", 0, None, None)
}

/// Test-only journal-backed writer for unit-level event assertions.
#[cfg(test)]
pub(crate) fn journal_event_writer<'a>(
    journal: &'a JournalRecorder,
    run_id: &str,
) -> EventWriter<'a> {
    EventWriter::new(
        Some(journal),
        EventTrack::V02,
        run_id,
        &"0".repeat(64),
        0,
        None,
        None,
    )
}

impl<'a> EventWriter<'a> {
    pub(crate) fn new(
        journal: Option<&'a JournalRecorder>,
        track: EventTrack,
        run_id: &str,
        manifest_sha256: &str,
        seq: u64,
        prev_hash: Option<String>,
        heartbeat: Option<crate::gateway::model::ActivityClock>,
    ) -> Self {
        Self {
            journal,
            buffer: None,
            track,
            run_id: run_id.to_string(),
            manifest_sha256: manifest_sha256.to_string(),
            seq,
            prev_hash,
            heartbeat,
        }
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：per-call 缓冲
    /// writer——并行读批次内每个调用一个，事件先收集、提交阶段按声明序
    /// 经真实 writer 重放（保持 hash 链与消息序）。
    pub(crate) fn buffered(run_id: &str) -> Self {
        let mut w = Self::new(None, EventTrack::V02, run_id, "", 0, None, None);
        w.buffer = Some(Vec::new());
        w
    }

    /// Take the buffered events (parallel read batch commit replay).
    pub(crate) fn drain(&mut self) -> Vec<(EventType, serde_json::Value)> {
        self.buffer.take().unwrap_or_default()
    }

    pub(crate) async fn record(
        &mut self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Result<(), AgentLoopError> {
        // Grill mode: nothing to record — no chain, no heartbeats.
        // Buffered mode (0k parallel read batch): collect for ordered replay.
        let Some(journal) = self.journal else {
            if let Some(buf) = &mut self.buffer {
                buf.push((event_type, payload));
            }
            return Ok(());
        };
        // GAP-INQUIRY-SPLIT: `neutral_inquiry` / `retrieval_completion_check`
        // are retired on the v0.2 track (ADR-0010 §5.1) — a v0.2 producer
        // writing one is a producer bug (mirrors the verifier's negative
        // fixtures). `V01` keeps them for historical replay only.
        // Review P3-6 (2026-08-10): an error return, not a panic — a future
        // producer slip must fail the run, not the whole process.
        if self.track == EventTrack::V02
            && matches!(
                event_type,
                EventType::NeutralInquiry | EventType::RetrievalCompletionCheck
            )
        {
            return Err(AgentLoopError::Assurance(format!(
                "v0.2 track must not write retired event type {event_type}"
            )));
        }
        // P1-1: a journaled event is activity (rounds, gates, tool events).
        if let Some(h) = &self.heartbeat {
            h.stamp();
        }
        let mut event = match self.track {
            EventTrack::V02 => RunEvent::new_v02(
                self.run_id.clone(),
                self.seq,
                event_type,
                self.manifest_sha256.clone(),
                self.prev_hash.clone(),
                self.track.payload_schema_id().into(),
                payload,
                Redaction::None,
                chrono_utc_now(),
            ),
            EventTrack::V01 => RunEvent::new_v01(
                self.run_id.clone(),
                self.seq,
                event_type,
                self.manifest_sha256.clone(),
                self.prev_hash.clone(),
                self.track.payload_schema_id().into(),
                payload,
                Redaction::None,
                chrono_utc_now(),
            ),
        };
        seal_event(&mut event).map_err(|e| AgentLoopError::Assurance(e.to_string()))?;
        let event_hash = event.event_sha256.clone();
        // Only advance the chain link after the write is accepted — a
        // refused append (Closed/TerminalAppended) must not pollute the
        // caller's bookkeeping (2026-08-04 review P2-7).
        journal.record_async(event).await?;
        self.prev_hash = Some(event_hash);
        self.seq += 1;
        Ok(())
    }

    pub(crate) fn seq(&self) -> u64 {
        self.seq
    }

    /// The run this writer appends to (checkpoint_ids embed the run id).
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Hash of the last recorded event — the next event's chain link.
    pub fn prev_hash(&self) -> Option<String> {
        self.prev_hash.clone()
    }
}

/// Get current UTC timestamp in ISO 8601 format.
pub(crate) fn chrono_utc_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::SubagentRole;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{GatewayError, ModelGateway, ModelRequest, ModelResponse};
    use crate::host::{LoopHost, PermitDecision, PermitError, RiskClass, ToolError, ToolRegistry};
    use async_trait::async_trait;
    use orz_assurance::JournalRecorder;

    #[tokio::test]
    async fn run_turn_full_gate_sequence() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        // Two scripted texts — the first is intercepted by the counterexample
        // gate (§4.6: the final-answer gate fires once before the conclusion);
        // the second is the post-gate final answer.
        let gateway: Arc<dyn ModelGateway> =
            Arc::new(FakeProvider::from_texts(vec!["结果：完成", "结果：完成"]));
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(
                &host,
                "列出当前目录",
                "RUN-SEQ",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(result.unwrap().0, "结果：完成");

        let replay =
            orz_assurance::replay_journal(&dir.join("events.jsonl"), Some("RUN-SEQ"), None, true);
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_finished"));

        // Full gate sequence (text-only path) — the availability probe
        // precedes run_started (Python conformance); the counterexample gate
        // separates the two model outputs. GAP-INQUIRY-SPLIT: the per-turn
        // orientation event is gone (the orientation producer fires only on
        // the session-level 7-round trigger — this two-round run fires none;
        // the no-orientation-state path skips it entirely).
        let types = event_types(&dir);
        assert_eq!(
            types,
            vec![
                EventType::ToolAvailabilityCheck,
                EventType::RunStarted,
                EventType::PromptSubmitted,
                // ORZ-CACHE-CONTEXT-COST (2026-08-15): the first model
                // request of the loop journals its header fingerprint.
                EventType::RequestHeaderChange,
                EventType::ModelOutput,
                EventType::CounterexampleGate,
                EventType::ModelOutput,
                EventType::RunFinished,
            ],
            "{types:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A4: without a plan the status line is absent — zero dilution for
    /// non-plan runs.
    #[tokio::test]
    async fn no_plan_means_no_status_line() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("ok"),
            ScriptedResponse::text("ok"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-NOPLAN", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        for request in fake.received_requests() {
            assert!(
                !request.system.contains("[任务状态"),
                "no status line without a plan: {}",
                request.system
            );
            assert!(
                !request
                    .messages
                    .iter()
                    .any(|m| m.content.contains("[任务状态")),
                "no trailing status message without a plan"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A4 cache discipline: while the plan state is unchanged, the system
    /// prompt is byte-identical across rounds — the provider prefix cache
    /// keeps hitting (2026-08-07 regression discipline).
    #[tokio::test]
    async fn status_line_is_byte_stable_across_rounds() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "ok".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"target_file": "a.txt"}),
                call_id: "call-c1".to_string(),
            }]),
            ScriptedResponse::text("第一轮回答"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-STABLE".to_string(),
            1,
            "任务".to_string(),
            vec!["步骤一".to_string()],
        );
        controller
            .run_turn(&host, "开始", "RUN-STABLE", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let received = fake.received_requests();
        assert!(received.len() >= 2, "{received:?}");
        assert_eq!(
            received[0].system, received[1].system,
            "system prompt must be byte-identical across rounds"
        );
        assert!(!received[0].system.contains("[任务状态 v0.1]"));
        // 2026-08-18 (ADR-0010 §14.25 项 1): 状态行作为尾随用户消息、
        // 仅在变化时追加——计划不变 → 每轮请求恰好一条状态消息（去重）。
        assert_eq!(
            received[0]
                .messages
                .iter()
                .filter(|m| m.content.contains("[任务状态"))
                .count(),
            1,
            "status line appended once on the first request"
        );
        assert_eq!(
            received[1]
                .messages
                .iter()
                .filter(|m| m.content.contains("[任务状态"))
                .count(),
            1,
            "unchanged plan state must not append a second status line"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn error_path_writes_terminal_run_failed() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        // Empty script — the provider errors on the first round (exhausted).
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(Vec::new()));
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(&host, "hello", "RUN-FAIL", MANIFEST, 0, None, None, None)
            .await;
        assert!(result.is_err(), "expected model error, got {result:?}");

        // The journal must end on a terminal run_failed event — never a
        // mid-sequence orphan (2026-08-04 review P1-1).
        let replay =
            orz_assurance::replay_journal(&dir.join("events.jsonl"), Some("RUN-FAIL"), None, true);
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F-06 (2026-08-07 review): a gateway that streams partial content and
    /// then aborts mid-stream — the partial output must reach the journal as
    /// an incomplete `model_output` BEFORE the terminal `run_failed` (D-7
    /// "保留输出 + incomplete 标记"; previously the partial text went only
    /// to live deltas and the journal lost it).
    #[tokio::test]
    async fn stream_abort_preserves_partial_output_in_journal() {
        struct PartialThenAbort;
        #[async_trait]
        impl ModelGateway for PartialThenAbort {
            fn for_new_run(&self) -> std::sync::Arc<dyn ModelGateway> {
                std::sync::Arc::new(PartialThenAbort)
            }

            async fn generate(&self, _req: ModelRequest) -> Result<ModelResponse, GatewayError> {
                Err(GatewayError::Transport("stream aborted mid-way".into()))
            }
            async fn generate_stream(
                &self,
                _req: ModelRequest,
                _cancel: Option<&tokio_util::sync::CancellationToken>,
                _heartbeat: Option<&crate::gateway::model::ActivityClock>,
                on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
            ) -> Result<ModelResponse, GatewayError> {
                on_chunk("partial answer...");
                Err(GatewayError::Transport("stream aborted mid-way".into()))
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(PartialThenAbort);
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(&host, "hello", "RUN-PART", MANIFEST, 0, None, None, None)
            .await;
        assert!(result.is_err(), "expected model error, got {result:?}");

        let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let lines: Vec<serde_json::Value> = events
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        let model_outputs: Vec<_> = lines
            .iter()
            .filter(|l| l["event_type"] == "model_output")
            .collect();
        assert_eq!(
            model_outputs.len(),
            1,
            "exactly one (incomplete) model_output in: {events}"
        );
        assert_eq!(model_outputs[0]["payload"]["text"], "partial answer...");
        assert_eq!(model_outputs[0]["payload"]["incomplete"], true);
        assert_eq!(lines.last().unwrap()["event_type"], "run_failed");

        // The chain must stay valid — the incomplete record precedes the
        // terminal event.
        let replay =
            orz_assurance::replay_journal(&dir.join("events.jsonl"), Some("RUN-PART"), None, true);
        assert!(replay.valid, "journal errors: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_failed"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3)：
    /// 成功路径 transport 重试计数入事件面——重试后恢复
    /// （`transport_retry{outcome: recovered}`），journal 可见重试次数
    /// 与类别。
    #[tokio::test]
    async fn transport_retry_recovered_event_is_journaled() {
        struct RetryRecoveredGateway {
            inner: Arc<FakeProvider>,
            fired: std::sync::Arc<std::sync::atomic::AtomicU32>,
        }
        #[async_trait]
        impl ModelGateway for RetryRecoveredGateway {
            fn for_new_run(&self) -> std::sync::Arc<dyn ModelGateway> {
                std::sync::Arc::new(Self {
                    inner: self.inner.clone(),
                    fired: self.fired.clone(),
                })
            }

            async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
                self.inner.generate(request).await
            }

            async fn generate_stream(
                &self,
                request: ModelRequest,
                cancel: Option<&tokio_util::sync::CancellationToken>,
                heartbeat: Option<&crate::gateway::model::ActivityClock>,
                on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
            ) -> Result<ModelResponse, GatewayError> {
                let mut response = self
                    .inner
                    .generate_stream(request, cancel, heartbeat, on_chunk)
                    .await?;
                // 仅第一次模型请求携带重试摘要（counterexample gate 轮
                // 保持无重试，断言事件恰好一条）。
                if self.fired.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                    response.transport_retry = crate::gateway::model::TransportRetryInfo {
                        retries: 2,
                        kind: Some(crate::gateway::model::TransportRetryKind::ZeroChunk),
                        reason: Some("error sending request: connection reset".to_string()),
                    };
                }
                Ok(response)
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        // 主模型轮 + 最终答案 counterexample gate 轮（§4.6）各一次。
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let gateway: Arc<dyn ModelGateway> = Arc::new(RetryRecoveredGateway {
            inner: fake,
            fired: std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0)),
        });
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(&host, "hi", "RUN-RETRY-REC", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let lines: Vec<serde_json::Value> = events
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        let retries: Vec<_> = lines
            .iter()
            .filter(|l| l["event_type"] == "transport_retry")
            .collect();
        assert_eq!(retries.len(), 1, "one transport_retry event in: {events}");
        assert_eq!(retries[0]["payload"]["outcome"], "recovered");
        assert_eq!(retries[0]["payload"]["retries"], 2);
        assert_eq!(retries[0]["payload"]["kind"], "zero_chunk");
        assert_eq!(
            retries[0]["payload"]["reason"],
            "error sending request: connection reset"
        );
        // 事件位于 model_output 之后、terminal 之前（链序）。
        let retry_idx = lines
            .iter()
            .position(|l| l["event_type"] == "transport_retry")
            .unwrap();
        assert_eq!(lines.last().unwrap()["event_type"], "run_finished");
        assert!(retry_idx < lines.len() - 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3)：
    /// 失败路径 transport 重试耗尽事件（`transport_retry{outcome:
    /// exhausted}`）——记录在 run_failed 之前，journal 链序完整。
    #[tokio::test]
    async fn transport_retry_exhausted_event_is_journaled() {
        struct RetryExhaustedGateway;
        #[async_trait]
        impl ModelGateway for RetryExhaustedGateway {
            fn for_new_run(&self) -> std::sync::Arc<dyn ModelGateway> {
                std::sync::Arc::new(RetryExhaustedGateway)
            }

            async fn generate(&self, _req: ModelRequest) -> Result<ModelResponse, GatewayError> {
                Err(GatewayError::StreamInterrupted {
                    attempts: 3,
                    saw_chunk: false,
                    detail: "stream zero-chunk interruption: retry cap reached".to_string(),
                })
            }

            async fn generate_stream(
                &self,
                _req: ModelRequest,
                _cancel: Option<&tokio_util::sync::CancellationToken>,
                _heartbeat: Option<&crate::gateway::model::ActivityClock>,
                _on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
            ) -> Result<ModelResponse, GatewayError> {
                Err(GatewayError::StreamInterrupted {
                    attempts: 3,
                    saw_chunk: false,
                    detail: "stream zero-chunk interruption: retry cap reached".to_string(),
                })
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(RetryExhaustedGateway);
        let controller = AgentLoopController::with_gateway(gateway);
        let result = controller
            .run_turn(&host, "hi", "RUN-RETRY-EXH", MANIFEST, 0, None, None, None)
            .await;
        assert!(result.is_err(), "expected model error, got {result:?}");

        let events = std::fs::read_to_string(dir.join("events.jsonl")).unwrap();
        let lines: Vec<serde_json::Value> = events
            .lines()
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        let retries: Vec<_> = lines
            .iter()
            .filter(|l| l["event_type"] == "transport_retry")
            .collect();
        assert_eq!(retries.len(), 1, "one transport_retry event in: {events}");
        assert_eq!(retries[0]["payload"]["outcome"], "exhausted");
        assert_eq!(retries[0]["payload"]["retries"], 3);
        assert_eq!(retries[0]["payload"]["kind"], "zero_chunk");
        // 耗尽事件必须位于 terminal run_failed 之前。
        let retry_idx = lines
            .iter()
            .position(|l| l["event_type"] == "transport_retry")
            .unwrap();
        let failed_idx = lines
            .iter()
            .position(|l| l["event_type"] == "run_failed")
            .unwrap();
        assert!(retry_idx < failed_idx);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn counterexample_gate_fires_once_before_final_answer() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        // First text-only round is the final-answer candidate → the gate
        // fires ONCE and the post-gate round produces the actual final answer.
        let fake = Arc::new(FakeProvider::from_texts(vec!["草稿", "终答"]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway);
        let (response, _, _) = controller
            .run_turn(&host, "hello", "RUN-GATE", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        // The intercepted draft is never returned — the post-gate answer is.
        assert_eq!(response, "终答");

        let types = event_types(&dir);
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == EventType::CounterexampleGate)
                .count(),
            1,
            "gate must fire exactly once: {types:?}"
        );
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == EventType::ModelOutput)
                .count(),
            2,
            "{types:?}"
        );
        let events = events(&dir);
        let gate_event = events
            .iter()
            .find(|e| e.event_type == EventType::CounterexampleGate)
            .unwrap();
        assert_eq!(
            gate_event.payload.get("position").and_then(|p| p.as_str()),
            Some("final_answer")
        );
        assert_eq!(
            gate_event
                .payload
                .get("once_only")
                .and_then(|o| o.as_bool()),
            Some(true)
        );
        assert!(
            gate_event
                .payload
                .get("message_block")
                .and_then(|b| b.as_str())
                .is_some_and(|b| b.starts_with("[COUNTEREXAMPLE_GATE v0.1]"))
        );

        // The block was injected into the post-gate round's request.
        let requests = fake.received_requests();
        assert_eq!(requests.len(), 2);
        assert!(
            requests[1]
                .messages
                .iter()
                .any(|m| m.role == Role::User && m.content.contains("[COUNTEREXAMPLE_GATE v0.1]")),
            "{:?}",
            requests[1].messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── Phase 3 slice #7: cooperative cancellation ─────────────────────────

    /// A pre-cancelled token aborts at the loop-top checkpoint (before any
    /// model round): the journal ends with a valid `run_cancelled` terminal
    /// and never sees a `model_output`.
    #[tokio::test]
    async fn pre_cancelled_token_aborts_run_with_run_cancelled_terminal() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> =
            Arc::new(FakeProvider::from_texts(vec!["结果：完成", "结果：完成"]));
        let controller = AgentLoopController::with_gateway(gateway);
        let token = tokio_util::sync::CancellationToken::new();
        token.cancel();

        let result = controller
            .run_turn_with_cancel(
                &host,
                "hello",
                "RUN-CANCEL",
                MANIFEST,
                0,
                None,
                Some(&token),
                None,
                None,
            )
            .await;

        assert!(matches!(result, Err(AgentLoopError::Cancelled)));
        let types = event_types(&dir);
        assert!(
            !types.contains(&EventType::ModelOutput),
            "cancel at loop top precedes any model round: {types:?}"
        );

        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-CANCEL"),
            None,
            true,
        );
        assert!(replay.valid, "journal invalid: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));
        let cancelled = events(&dir)
            .into_iter()
            .find(|e| e.event_type == EventType::RunCancelled)
            .expect("run_cancelled event");
        assert_eq!(
            cancelled.payload.get("reason").and_then(|r| r.as_str()),
            Some("user_cancelled")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A cancel landing mid-model-round terminates at the after-round
    /// checkpoint — BEFORE tool dispatch, so no tool ever starts.
    #[tokio::test]
    async fn cancel_mid_round_skips_tool_dispatch() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        // First round is a text (slow-chunked so the cancel lands inside the
        // model round); the tool round would come after it — never reached.
        let gateway: Arc<dyn ModelGateway> = Arc::new(
            FakeProvider::new(vec![
                ScriptedResponse::text("准备执行命令。"),
                ScriptedResponse::tool_calls(vec![tool_call("bash", "call-1")]),
                ScriptedResponse::text("结果：完成"),
                ScriptedResponse::text("结果：完成"),
            ])
            .with_chunk_delay(std::time::Duration::from_millis(100)),
        );
        let controller = Arc::new(AgentLoopController::with_gateway(gateway));
        let token = tokio_util::sync::CancellationToken::new();

        let c = controller.clone();
        let t = token.clone();
        let run = tokio::task::spawn(async move {
            c.run_turn_with_cancel(
                &host,
                "执行命令",
                "RUN-CANCEL-2",
                MANIFEST,
                0,
                None,
                Some(&t),
                None,
                None,
            )
            .await
        });
        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        token.cancel();
        let result = run.await.unwrap();

        assert!(matches!(result, Err(AgentLoopError::Cancelled)));
        let types = event_types(&dir);
        assert!(
            !types.contains(&EventType::ToolStarted),
            "cancel precedes tool dispatch: {types:?}"
        );
        assert!(types.contains(&EventType::RunCancelled), "{types:?}");

        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-CANCEL-2"),
            None,
            true,
        );
        assert!(replay.valid, "journal invalid: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A multi-tool round: a cancel landing while the FIRST tool is still
    /// running must not start the remaining tools — the per-tool checkpoint
    /// (2026-08-05 review P2-2) gates every dispatch, not just the round.
    #[tokio::test]
    async fn cancel_mid_multi_tool_round_skips_unstarted_tools() {
        struct SlowFirstToolHost {
            journal: JournalRecorder,
            release: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<()>>>>,
        }

        #[async_trait]
        impl LoopHost for SlowFirstToolHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _args: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                Ok(PermitDecision::AllowOnce)
            }
            async fn call_tool(
                &self,
                name: &str,
                _args: serde_json::Value,
                _call_id: &str,
            ) -> Result<ToolResult, ToolError> {
                if name == "bash" {
                    // Block until the test releases — the cancel lands while
                    // tool #1 is in flight.
                    let mut rx = self.release.lock().unwrap().take().unwrap();
                    let _ = rx.recv().await;
                }
                Ok(ToolResult {
                    output: "ok".to_string(),
                    exit_code: None,
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                })
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let (release_tx, release_rx) = tokio::sync::mpsc::channel::<()>(1);
        let host = SlowFirstToolHost {
            journal,
            release: Arc::new(Mutex::new(Some(release_rx))),
        };
        // One round with TWO tool calls: bash (slow) + read_file.
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                tool_call("bash", "call-1"),
                tool_call("read_file", "call-2"),
            ]),
            ScriptedResponse::text("结果：完成"),
            ScriptedResponse::text("结果：完成"),
        ]));
        let controller = Arc::new(AgentLoopController::with_gateway(gateway));
        let token = tokio_util::sync::CancellationToken::new();

        let c = controller.clone();
        let t = token.clone();
        let mut run = tokio::task::spawn(async move {
            c.run_turn_with_cancel(
                &host,
                "执行命令",
                "RUN-CANCEL-4",
                MANIFEST,
                0,
                None,
                Some(&t),
                None,
                None,
            )
            .await
        });

        // The run parks inside bash (tool #1 in flight); cancel lands there.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        token.cancel();
        // Cooperative: the in-flight tool finishes first.
        tokio::select! {
            _ = &mut run => panic!("cancel must not abort the in-flight tool"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
        let _ = release_tx.send(()).await;
        let result = run.await.unwrap();
        assert!(matches!(result, Err(AgentLoopError::Cancelled)));

        let all_events = events(&dir);
        let types: Vec<EventType> = all_events.iter().map(|e| e.event_type.clone()).collect();
        let started: Vec<&str> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::ToolStarted)
            .filter_map(|e| e.payload.get("tool").and_then(|t| t.as_str()))
            .collect();
        assert_eq!(
            started,
            vec!["bash"],
            "only the in-flight tool starts — read_file must never dispatch: {types:?}"
        );
        assert!(types.contains(&EventType::RunCancelled), "{types:?}");

        let replay = orz_assurance::replay_journal(
            &dir.join("events.jsonl"),
            Some("RUN-CANCEL-4"),
            None,
            true,
        );
        assert!(replay.valid, "journal invalid: {:?}", replay.errors);
        assert_eq!(replay.terminal_event.as_deref(), Some("run_cancelled"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A host whose permission await is gated on an external oneshot: a
    /// cancel during the permission prompt does NOT interrupt it (cooperative
    /// semantics — the permission manager is session-scoped and shared; a
    /// dropped future could orphan its prompt). The run terminates at the
    /// next checkpoint AFTER the permission resolves.
    #[tokio::test]
    async fn cancel_during_permission_await_resolves_then_terminates() {
        struct GatedHost {
            journal: JournalRecorder,
            release: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<()>>>>,
        }

        #[async_trait]
        impl LoopHost for GatedHost {
            fn journal(&self) -> &JournalRecorder {
                &self.journal
            }
            fn tools_registry(&self) -> &dyn ToolRegistry {
                &EmptyRegistry
            }
            async fn request_permission(
                &self,
                _risk: RiskClass,
                _tool: &str,
                _args: &serde_json::Value,
            ) -> Result<PermitDecision, PermitError> {
                // Block until the test sends the release, then deny.
                let mut rx = self.release.lock().unwrap().take().unwrap();
                let _ = rx.recv().await;
                Ok(PermitDecision::Deny)
            }
        }

        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let (release_tx, release_rx) = tokio::sync::mpsc::channel::<()>(1);
        let host = GatedHost {
            journal,
            release: Arc::new(Mutex::new(Some(release_rx))),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("bash", "call-1")]),
            ScriptedResponse::text("结果：完成"),
            ScriptedResponse::text("结果：完成"),
        ]));
        let controller = Arc::new(AgentLoopController::with_gateway(gateway));
        let token = tokio_util::sync::CancellationToken::new();

        let c = controller.clone();
        let t = token.clone();
        let mut run = tokio::task::spawn(async move {
            c.run_turn_with_cancel(
                &host,
                "执行命令",
                "RUN-CANCEL-3",
                MANIFEST,
                0,
                None,
                Some(&t),
                None,
                None,
            )
            .await
        });

        // The run parks at the permission await (tool round reached).
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        token.cancel();
        // Cooperative: cancelling does NOT abort the pending permission.
        tokio::select! {
            _ = &mut run => panic!("cancel must not interrupt the permission await"),
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
        // Release the gate (deny) — the loop's next checkpoint terminates.
        let _ = release_tx.send(()).await;
        let result = run.await.unwrap();
        assert!(matches!(result, Err(AgentLoopError::Cancelled)));

        let types = event_types(&dir);
        assert!(
            !types.contains(&EventType::ToolStarted),
            "denied tool must not start: {types:?}"
        );
        assert!(types.contains(&EventType::PermissionRequested), "{types:?}");
        assert!(types.contains(&EventType::RunCancelled), "{types:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Grill mode (2026-08-08 write-placement slice, design §3): a grill
    /// turn runs the full model↔tool loop with a discard `EventWriter` — no
    /// journal events land on disk, the conversation history is written back
    /// for the next turn, the template is injected once, and the
    /// final-answer counterexample gate is skipped (a grill question is not
    /// a final answer — the gate would force an extra provider round).
    #[tokio::test]
    async fn grill_turn_no_journal_history_and_skips_gate() {
        // Two scripts: if the counterexample gate fired, the model would get
        // a second round and the final response would be the second script.
        let provider = FakeProvider::new(vec![
            ScriptedResponse::text("问题一: 是否考虑……?"),
            ScriptedResponse::text("问题二: 不应到达"),
        ]);
        let controller = AgentLoopController::with_gateway(Arc::new(provider));
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };

        // Turn 1: template injected once + user input + assistant question.
        let mut history = Vec::new();
        let response = controller
            .run_grill_turn(&host, &mut history, "用户回答一", Some("模板"), None)
            .await
            .unwrap();
        assert_eq!(response, "问题一: 是否考虑……?");
        assert_eq!(history.len(), 3, "template + user input + assistant reply");
        assert_eq!(history[0].content, "模板");
        assert_eq!(history[1].content, "用户回答一");
        assert_eq!(history[2].content, "问题一: 是否考虑……?");

        // Turn 2: no template (session start only), history continues; the
        // provider's second script is consumed normally.
        let response2 = controller
            .run_grill_turn(&host, &mut history, "用户回答二", None, None)
            .await
            .unwrap();
        assert_eq!(response2, "问题二: 不应到达");
        assert_eq!(history.len(), 5);

        // No run journal was touched (bootstrap dirs belong to the host; the
        // controller wrote zero events).
        assert!(
            !dir.join("events.jsonl").exists(),
            "grill turns must not write run-journal events"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Grill turns are a model↔tool round — ACAF fail-closed with an
    /// unconfigured fabric must refuse loudly here too (2026-08-16 review
    /// fix P2-I1), matching the `run_turn_with_guards` D-15 startup check
    /// and the ACP server's documented promise.
    #[tokio::test]
    async fn grill_turn_fail_closed_without_fabric_refuses() {
        let provider = FakeProvider::new(vec![ScriptedResponse::text("不应到达")]);
        let controller =
            AgentLoopController::with_gateway(Arc::new(provider)).with_acaf_fail_closed(true);
        let dir = test_dir();
        let host = TestHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: None,
        };
        let mut history = Vec::new();
        let err = controller
            .run_grill_turn(&host, &mut history, "问题", None, None)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("fail-closed"),
            "unexpected error: {err}"
        );
        assert_eq!(history.len(), 0, "no turn may start without a fabric");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A valid five-section summary response (mechanical slots are filled
    /// by the controller; only the two model slots matter here).
    /// The conversation seeds the model context (prior history + new prompt)
    /// and the full conversation comes back on success — reasoning content
    /// included (the DeepSeek multi-turn replay requirement).
    #[tokio::test]
    async fn run_turn_conversation_seeds_and_writes_back() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("收到"),
            ScriptedResponse::text("收到"),
        ]));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut conversation = vec![
            conv_message(Role::User, "第一问"),
            Message {
                role: Role::Assistant,
                content: "第一答".to_string(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: Some("思考过程".to_string()),
            },
        ];
        let response = controller
            .run_turn(
                &host,
                "第二问",
                "RUN-CONV",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        assert_eq!(response.0, "收到");
        // The model request carried the full history + the new prompt.
        let reqs = fake.received_requests();
        assert_eq!(reqs.len(), 2, "model call + final answer round");
        let msgs = &reqs[0].messages;
        assert_eq!(msgs[0].content, "第一问", "{msgs:?}");
        assert_eq!(msgs[1].content, "第一答");
        assert_eq!(
            msgs[1].reasoning_content.as_deref(),
            Some("思考过程"),
            "reasoning must be replayed"
        );
        assert_eq!(msgs[2].content, "第二问");
        // The conversation came back complete (history + new turn + the
        // model's reply; mechanical injection blocks filtered out).
        assert_eq!(conversation.len(), 4, "{conversation:?}");
        assert_eq!(conversation[0].content, "第一问");
        assert_eq!(conversation[2].content, "第二问");
        assert_eq!(conversation[3].content, "收到");
        assert!(
            conversation
                .iter()
                .all(|m| !is_injected_block_text(&m.content)),
            "injection blocks filtered: {conversation:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6): a
    /// restored conversation over the recovery window is mechanically
    /// truncated before the first request — whole old rounds dropped, the
    /// recovery marker inserted, the full sidecar copied into the run
    /// journal as the audit copy, and `context_recovery_truncated` journaled.
    #[tokio::test]
    async fn recovery_conversation_over_window_truncates_before_first_request() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("收到"),
            ScriptedResponse::text("收到"),
        ]));
        let controller =
            AgentLoopController::with_gateway(fake.clone()).with_recovery_compact(10, 2);
        let mut conversation = vec![conv_message(Role::User, "第一问")];
        conversation.extend(tool_round("call-r1", "第一轮工具结果"));
        conversation.extend(tool_round("call-r2", "第二轮工具结果"));
        let before = conversation.clone();
        let _ = controller
            .run_turn(
                &host,
                "第二问",
                "RUN-REC",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await
            .unwrap();
        let evs = events(&dir);
        let trunc: Vec<&RunEvent> = evs
            .iter()
            .filter(|e| e.event_type == EventType::ContextRecoveryTruncated)
            .collect();
        assert_eq!(trunc.len(), 1, "{evs:?}");
        assert!(
            trunc[0].payload["rounds_dropped"].as_u64().unwrap() >= 1,
            "must drop whole rounds: {:?}",
            trunc[0].payload
        );
        let audit = dir.join("recovery-conversation-full.json");
        assert!(audit.exists(), "full sidecar audit copy must exist");
        let audit_payload: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&audit).unwrap()).unwrap();
        assert_eq!(audit_payload, serde_json::to_value(&before).unwrap());
        // The recovery marker reaches the first model request and survives
        // the restore write-back (D3-1).
        let reqs = fake.received_requests();
        let first = &reqs[0].messages;
        assert!(
            first.iter().any(|m| m
                .content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)),
            "recovery marker must reach the first request: {first:?}"
        );
        assert!(
            conversation.iter().any(|m| m
                .content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)),
            "marker must survive write-back: {conversation:?}"
        );
        // No orphaned tool results after truncation: every Tool message's
        // call id must be declared by a surviving assistant message.
        let declared: Vec<&str> = conversation
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.as_str()))
            .collect();
        for m in conversation.iter().filter(|m| m.role == Role::Tool) {
            assert!(
                m.tool_call_id
                    .as_deref()
                    .is_some_and(|id| declared.contains(&id)),
                "orphan tool result after truncation: {m:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A failed run keeps the caller's conversation byte-identical (clone
    /// seed + success-only write-back; the journal is the failure evidence).
    #[tokio::test]
    async fn run_turn_conversation_error_preserves_history() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        // Empty script — the model call fails with "script exhausted".
        let fake = Arc::new(FakeProvider::new(vec![]));
        let controller = AgentLoopController::with_gateway(fake.clone());
        let mut conversation = vec![conv_message(Role::User, "第一问")];
        let before = conversation.clone();
        let result = controller
            .run_turn(
                &host,
                "第二问",
                "RUN-CONV-FAIL",
                MANIFEST,
                0,
                None,
                None,
                Some(&mut conversation),
            )
            .await;
        assert!(result.is_err(), "empty script must fail the run");
        assert_eq!(conversation, before, "failed run must not touch history");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `None` conversation keeps the historical single-prompt seed exactly.
    #[tokio::test]
    async fn run_turn_none_conversation_unchanged() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::from_texts(vec!["完成", "完成"]));
        let controller = AgentLoopController::with_gateway(fake.clone());
        controller
            .run_turn(&host, "hi", "RUN-NONE", MANIFEST, 0, None, None, None)
            .await
            .unwrap();
        let reqs = fake.received_requests();
        assert_eq!(reqs.len(), 2, "model call + final answer round");
        let first = &reqs[0].messages;
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first[0].content, "hi");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// seed_from_json restores the subagent conversation (incl. reasoning +
    /// tool messages) — and `submitted` stays empty (D-6 update: the replay
    /// ledger still does not ride the sidecar).
    #[tokio::test]
    async fn seed_from_json_restores_conversation() {
        let snapshot = serde_json::json!({
            "next_seq": {"internal_retrieval": 1},
            "activations": [{
                "activation_id": "retrieval-internal_retrieval-sess-abc-00",
                "parent_session_id": "sess-abcdef123456",
                "subagent_session_id": "SUB-internal_retrieval-sess-abc",
                "contract_id": "retrieval-contract-internal_retrieval",
                "contract_revision": 0,
                "status": "awaiting_disposition",
                "tool_rounds_used": 2,
                "result_digest": "b".repeat(64),
                "pending_assessment_id": "ASSESS-PREV-1",
                "pending_expected_contract_revision": 0,
                "origin_run_id": "RUN-PREV-0001",
                "conversation": [
                    {"role": "user", "content": "第一问", "tool_call_id": null,
                     "tool_calls": [], "reasoning_content": null},
                    {"role": "assistant", "content": "第一答", "tool_call_id": null,
                     "tool_calls": [], "reasoning_content": "思考"},
                    {"role": "tool", "content": "结果", "tool_call_id": "call-1",
                     "tool_calls": [], "reasoning_content": null},
                ]
            }]
        });
        let controller =
            AgentLoopController::with_gateway(Arc::new(FakeProvider::from_texts(vec!["x"])));
        let mut registry = controller.activations.lock().unwrap();
        let restored = registry.seed_from_json(&snapshot);
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].conversation.len(), 3);
        assert_eq!(
            restored[0].conversation[1].reasoning_content.as_deref(),
            Some("思考")
        );
        assert_eq!(
            restored[0].conversation[2].tool_call_id.as_deref(),
            Some("call-1")
        );
        // ActivationState restored; submitted stays in-process empty.
        let state = registry
            .states
            .get(&SubagentRole::InternalRetrieval)
            .expect("activation restored");
        assert_eq!(state.conversation.len(), 3);
        assert!(state.submitted.is_empty(), "submitted stays in-process");
        drop(registry);
    }
}
