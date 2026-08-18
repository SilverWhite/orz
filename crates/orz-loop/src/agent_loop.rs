//! Shared model↔tool loop — the one loop all three agents run.
//!
//! GAP-SUBAGENT-RUNTIME (2026-08-10): ADR-0010 §3.1 isomorphism — the main
//! agent and both retrieval subagents run the SAME agent loop code; the
//! per-role semantics (which lane counts, whether the final-answer
//! counterexample gate applies, the budget) are carried by `LoopProfile`,
//! and the per-role surface (system prompt, tools, blackboard partition)
//! by the callers. The loop body here was extracted verbatim from
//! `AgentLoopController::run_turn_inner` (M1, 2026-08-10) — the main
//! profile must keep byte-identical event sequences with the pre-split
//! code (locked by `EXPECTED_SEQUENCES_V02` + conformance capture).
//!
//! The loop writes no run lifecycle events (`run_started` /
//! `run_finished` / …): those belong to the caller. Subagent loops run
//! inside the parent run's hash chain (M3, 2026-08-10) — the run
//! terminal uniqueness stays with the parent.

use std::path::Path;
use std::sync::{Arc, Mutex};

use orz_assurance::journal::chain::{payload_hash, sha256_hex};
use orz_assurance::{EventType, GateDecision};

use crate::agents::SubagentRole;
use crate::blackboard::SharedBlackboard;
use crate::checkpoint::{self, PendingCheckpoint};
use crate::controller::{
    AgentLoopController, AgentLoopError, ContextCompactConfig, DENIAL_BREAKER_CONSECUTIVE,
    DenialKey, DenialState, EventWriter, PolicyFeedback, TEXT_DELTA_PACING, compact_messages,
    estimate_message_tokens, estimate_messages_tokens, format_edit_record,
};
use crate::diagnostic_coverage::{DebugEpisodeState, maybe_consume_dc_signal, maybe_fire_dc};
use crate::gateway::model::{
    ActivityClock, FinishReason, GatewayError, Message, ModelResponse, Role, ToolCall,
};
use crate::host::{LoopHost, RiskClass, ToolDef, ToolResult};
use crate::orientation::{AgentRole, OrientationSessionState};
use crate::prompt::COUNTEREXAMPLE_GATE_BLOCK;
use crate::relay::{DispatchTarget, route};
use crate::tool::ToolDispatcher;

/// The request-level max-tokens cap for ALL three agents (ADR-0010 §3.4.2:
/// the same default is injected into the main agent and both retrieval
/// subagents — no lane carries its own literal; review F3, 2026-08-10).
/// The transport's `ModelConfig::max_tokens` is the ceiling and this
/// request-level value is min-capped by it — equal today, so the full
/// budget is available (D-6: a thinking subagent with a 1024-token cap
/// would spend everything on reasoning and die before producing output).
pub const REQUEST_MAX_TOKENS: u32 = 160_000;

/// One model generation round — the uniform round entry every agent
/// implements (ADR-0010 §3.4: identical model request shape).
#[allow(clippy::too_many_arguments)] // mirrors MainAgent::run_round's contract
#[async_trait::async_trait]
pub(crate) trait RoundAgent: Send + Sync {
    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the gateway
    /// config digest — part of the model-request header fingerprint.
    fn config_fingerprint(&self) -> String {
        "unknown-config".to_string()
    }

    async fn run_round(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
        max_tokens: u32,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError>;
}

/// Which system prompt the loop assembles — the per-role surface.
#[derive(Debug, Clone)]
pub(crate) enum SystemPromptKind {
    /// The main agent's `BASE_SYSTEM_PROMPT` + availability/budget/status
    /// blocks.
    Main,
    /// A retrieval task contract (ADR-0010 §3.2) — citation rules + the
    /// `[DOC]`/`[SOURCE]` delivery contract, NOT `BASE_SYSTEM_PROMPT`.
    Retrieval {
        role: SubagentRole,
        goal: String,
        /// GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): the explicit retrieval
        /// mode rides the prompt so the subagent's weighting/verification
        /// instructions match the active channel (framework_fallback = layer
        /// 2 web_fetch verification; local_browser = direct page reads).
        mode: crate::controller::RetrievalMode,
    },
}

/// Retrieval-lane tool filtering (ADR-0010 §3.2 — deny-only write domain:
/// the subagent writes only its own blackboard partition, the task's
/// retrieval docs and the retrieval archive; file mutations and shell are
/// structurally refused before the host's permission bridge).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolFilter {
    None,
    Retrieval,
}

impl ToolFilter {
    /// The write-domain gate — `Some(reason)` refuses the tool with a
    /// structured denial (IP2a "失败必显式"; the denial key feeds the
    /// shared 3-round breaker). The ACAF slice will resolve the write
    /// domains to absolute paths/object identities at this seam.
    ///
    /// Three refusal classes (review F6, 2026-08-10): file mutations
    /// (`modifies_files`), shell escape, and — distinctly — `run_tests`:
    /// §3.8.2 treats it as CONTROLLED CODE EXECUTION (the test process may
    /// write files / touch the network), so the lane refuses it with an
    /// execution-class reason, not a file-write one. A retrieval task
    /// contract never carries a test harness.
    fn write_gate(&self, tool: &str) -> Option<&'static str> {
        match self {
            ToolFilter::None => None,
            ToolFilter::Retrieval => {
                if tool == "run_tests" {
                    Some("retrieval_role_execution_denied")
                } else if tool == "blackboard_action_write" {
                    // P0-C S2 (2026-08-15): the console write button is
                    // main-lane only — subagents never write action orders.
                    Some("console_action_write_lane_denied")
                } else if tool == crate::planning::PLAN_WRITE_TOOL {
                    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): the
                    // plan-gate write surface is main-lane only (P2-1
                    // 审查收口 — 投影剥除之外的机械兜底)。
                    Some("plan_write_lane_denied")
                } else if ToolDispatcher::modifies_files(tool) {
                    Some("retrieval_role_write_denied")
                } else if ToolDispatcher::risk_class(tool) == RiskClass::SandboxEscape {
                    Some("retrieval_role_shell_denied")
                } else {
                    None
                }
            }
        }
    }

    /// Nested retrieval dispatch is refused inside a retrieval lane —
    /// one seat per role, no recursion (ADR-0010 §11.3).
    fn denies_nested_dispatch(&self) -> bool {
        matches!(self, ToolFilter::Retrieval)
    }
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the model-request
/// header fingerprint — the provider prefix-cache key components the client
/// can observe (system + tools + config). Messages are deliberately NOT part
/// of the header: they change every round and a header event exists to
/// attribute cache misses caused by the STATIC prefix, not by new dialogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RequestHeader {
    pub header_sha256: String,
    pub system_sha256: String,
    pub tools_sha256: String,
    pub config_sha256: String,
    pub tools: Vec<String>,
}

/// Compute the request-header fingerprint from the assembled system prompt,
/// the projected tool list and the transport config digest.
pub(crate) fn compute_request_header(
    system: &str,
    tools: &[ToolDef],
    config_fingerprint: &str,
) -> RequestHeader {
    let system_sha256 = sha256_hex(system.as_bytes());
    // 2026-08-15 review boundary (audit §5): the JSON fallback strings are
    // theoretical only (serde_json cannot fail on these shapes) — if they
    // ever appeared, real changes would collapse onto a constant digest, so
    // they must be treated as a hard bug, not a silent degrade.
    let mut tool_rows: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect();
    tool_rows.sort_by(|a, b| {
        a.get("name")
            .and_then(serde_json::Value::as_str)
            .cmp(&b.get("name").and_then(serde_json::Value::as_str))
    });
    let tools_sha256 = payload_hash(&tool_rows).unwrap_or_else(|_| "tools-hash-error".to_string());
    let config_sha256 = sha256_hex(config_fingerprint.as_bytes());
    let header_sha256 = payload_hash(&serde_json::json!({
        "system_sha256": system_sha256,
        "tools_sha256": tools_sha256,
        "config_sha256": config_sha256,
    }))
    .unwrap_or_else(|_| "header-hash-error".to_string());
    let tools = tools.iter().map(|t| t.name.clone()).collect();
    RequestHeader {
        header_sha256,
        system_sha256,
        tools_sha256,
        config_sha256,
        tools,
    }
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix, ADR-0010 §3.5 条6): which
/// header component changed between two requests — `system`, `tools`,
/// `config`, or `multiple` when more than one digest differs. This is the
/// mechanical「变化原因」attribution carried by `request_header_change` on
/// `reason=change`; probe-flip triggers are additionally attributable via
/// the flip↔header verifier cross-check (`_verify_v02_probe_accuracy`).
pub(crate) fn header_change_kind(prev: &RequestHeader, cur: &RequestHeader) -> &'static str {
    let mut changed = Vec::with_capacity(3);
    if prev.system_sha256 != cur.system_sha256 {
        changed.push("system");
    }
    if prev.tools_sha256 != cur.tools_sha256 {
        changed.push("tools");
    }
    if prev.config_sha256 != cur.config_sha256 {
        changed.push("config");
    }
    match changed.as_slice() {
        [single] => single,
        _ => "multiple",
    }
}

/// Build the `request_header_change` event payload. `reason` is `initial`
/// for the first request of a loop invocation, `change` when the fingerprint
/// differs from the previous request. `change_kind` is the mechanical
///「变化原因」(system/tools/config/multiple) and is present only on `change`.
pub(crate) fn request_header_payload(
    header: &RequestHeader,
    reason: &str,
    previous_header_sha256: Option<&str>,
    agent_role: &str,
    change_kind: Option<&str>,
) -> serde_json::Value {
    let mut payload = serde_json::json!({
        "reason": reason,
        "header_sha256": header.header_sha256,
        "system_sha256": header.system_sha256,
        "tools_sha256": header.tools_sha256,
        "config_sha256": header.config_sha256,
        "agent_role": agent_role,
        "tools": header.tools,
        "tool_count": header.tools.len(),
    });
    if let Some(prev) = previous_header_sha256 {
        payload["previous_header_sha256"] = serde_json::json!(prev);
    }
    if let Some(kind) = change_kind {
        payload["change_kind"] = serde_json::json!(kind);
    }
    payload
}

/// Role-specific loop semantics (ADR-0010 §3.2 — the ONLY difference surface
/// between the three agents' loop execution).
#[derive(Debug, Clone)]
pub(crate) struct LoopProfile {
    /// Whose loop this is — gates the main-only surfaces (plan status line,
    /// blackboard plan summary in the compaction marker, live text-delta
    /// forwarding).
    pub role: AgentRole,
    /// §4.5: the final-answer counterexample gate fires only in main runs
    /// (a retrieval result is not a run's formal answer). Grill turns fold
    /// this to `false` too (a grill question is not a run-semantic).
    pub counterexample_gate: bool,
    /// FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the final-answer
    /// output-level citation verifier (ADR-0010 §3.7.9) — main runs only,
    /// same delivery boundary as the counterexample gate. Grill answers and
    /// retrieval subagent texts are not run final answers and skip it.
    pub citation_validation: bool,
    /// The orientation lane this loop's completed model rounds count
    /// toward (ADR-0010 §4.2; `None` counts nothing — grill / M3).
    pub orientation_role: Option<AgentRole>,
    /// Which system prompt to assemble per round.
    pub system_kind: SystemPromptKind,
    /// Retrieval-lane tool projection (deny-only write domain + nested
    /// dispatch guard).
    pub tool_filter: ToolFilter,
    /// M5 (2026-08-10): Diagnostic Coverage hard-signal consumption +
    /// checkpoint injection (ADR-0010 §4.6) — main lane only (the
    /// mechanism is debug/problem-solving-specific; §4.6 scope).
    pub dc_enabled: bool,
    /// Independent per-session tool-round budget (§3.4.6 — 120 default).
    pub max_tool_rounds: u32,
    /// Rounds already consumed by this session BEFORE this loop starts
    /// (user adjudication 2026-08-10, review F5): a `continue` re-entry is
    /// the SAME retrieval session — the budget accumulates across
    /// dispatches and resets only with a NEW activation (Closed → next
    /// creation starts at 0). The main agent keeps the inherited per-run
    /// semantic (`main`/`grill` = 0).
    pub initial_tool_rounds: u32,
    /// FUS-TOOL-PROBE P0-A/P0-A-2: whether this loop re-probes the work
    /// tools and re-projects the tool list before every model request.
    /// Main/grill turns yes; retrieval lanes never re-probe (their list is
    /// the caller's final projection and no second
    /// `tool_availability_check` event may fire inside a lane).
    pub probe_work_tools: bool,
    /// ACAF Slice 2 fail-closed D-13 (2026-08-13): the retrieval lane's
    /// real activation_id — bound on the lane's action tickets (web_fetch /
    /// browser_read). The main lane is `None` (main-lane actions stay
    /// activation-less).
    pub activation_id: Option<String>,
    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the current
    /// dispatch's candidate counter (per-activation shared domain — the
    /// subagent loop mutates it on every candidate gate: web_fetch family +
    /// browser_read; the caller writes it back into the activation on every
    /// path). The main/grill lanes pass `None` — candidate-counted tools
    /// never execute there, and a `None` domain fails the gate closed.
    pub fetch_candidates: Option<Arc<Mutex<Vec<String>>>>,
}

impl LoopProfile {
    pub(crate) fn main(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: true,
            citation_validation: true,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            dc_enabled: true,
            max_tool_rounds,
            initial_tool_rounds: 0,
            probe_work_tools: true,
            activation_id: None,
            fetch_candidates: None,
        }
    }

    /// Grill-mode turn (2026-08-08): full loop, no final-answer gate
    /// (the grill answer is not a run-semantic; same reasoning as the
    /// counterexample skip comment in `run_turn_inner`).
    pub(crate) fn grill(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: false,
            citation_validation: false,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            dc_enabled: false,
            max_tool_rounds,
            initial_tool_rounds: 0,
            probe_work_tools: true,
            activation_id: None,
            fetch_candidates: None,
        }
    }

    /// A retrieval subagent's loop (GAP-SUBAGENT-RUNTIME 2026-08-10; M5:
    /// the internal/external lanes are fed — §4.2 counts every agent's
    /// completed logical model rounds; DC stays off (debug-specific, §4.6
    /// is a main-lane mechanism). `initial_tool_rounds` carries the
    /// session's consumed budget across `continue` re-entries (user
    /// adjudication 2026-08-10, review F5 — the caller reads it back from
    /// the activation).
    pub(crate) fn retrieval(
        role: SubagentRole,
        goal: &str,
        mode: crate::controller::RetrievalMode,
        max_tool_rounds: u32,
        initial_tool_rounds: u32,
        activation_id: &str,
        fetch_candidates: Option<Arc<Mutex<Vec<String>>>>,
    ) -> Self {
        let agent_role = match role {
            SubagentRole::InternalRetrieval => AgentRole::InternalRetrieval,
            SubagentRole::ExternalRetrieval => AgentRole::ExternalRetrieval,
        };
        Self {
            role: agent_role,
            counterexample_gate: false,
            citation_validation: false,
            orientation_role: Some(agent_role),
            system_kind: SystemPromptKind::Retrieval {
                role,
                goal: goal.to_string(),
                mode,
            },
            tool_filter: ToolFilter::Retrieval,
            dc_enabled: false,
            max_tool_rounds,
            initial_tool_rounds,
            probe_work_tools: false,
            activation_id: Some(activation_id.to_string()),
            fetch_candidates,
        }
    }
}

/// The shared `&self`-field subset the loop reads. The controller reference
/// rides separately — its methods (`maybe_fire_orientation`, tool dispatch,
/// status line) stay on the type.
pub(crate) struct SharedLoopServices<'a> {
    pub blackboard: &'a Arc<SharedBlackboard>,
    pub denial_state: &'a Mutex<DenialState>,
    pub pacing_rounds: &'a std::sync::atomic::AtomicU32,
    pub context_compact: &'a ContextCompactConfig,
    /// M5 (2026-08-10): the Diagnostic Coverage episode state (ADR-0010
    /// §4.6) — consumed by the main lane only (`profile.dc_enabled`).
    pub dc_state: &'a Mutex<DebugEpisodeState>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval-lane tool-call evidence
    /// (ADR-0010 §3.7.4 — the mechanical source of the structured ledger).
    /// `Some` only on retrieval profiles; the main lane passes `None`.
    pub evidence: Option<&'a Mutex<Vec<crate::controller::EvidenceRecord>>>,
    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): the live policy revision —
    /// feeds the role-gate denial key (a bump is a key change → the breaker
    /// resets, ADR-0010 §3.5.4).
    pub policy_revision: &'a std::sync::atomic::AtomicU64,
    /// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): the per-model-round
    /// tool-result injection budget in estimated tokens
    /// (`ORZ_MAX_INJECT_TOKENS_PER_ROUND`, default 50K). Read once at
    /// controller construction; the loop refuses later calls of a batch once
    /// the accumulated estimated tokens reach it.
    pub max_inject_tokens_per_round: u64,
    /// F5 (2026-08-15, BACKLOG 6e 复查遗留): the controller-configured
    /// epoch archive directory — the single source for the path-slot
    /// overflow pointer (never re-derived from the session cwd).
    pub blackboard_archive_dir: Option<&'a Path>,
}

/// What the loop produced — the caller maps it to its own terminal
/// semantics (main: run_finished/run_invalidated; subagent: result
/// formation / budget_exhausted / subagent_failed).
pub(crate) struct LoopOutcome {
    pub last_text: Option<String>,
    pub tool_rounds: u32,
    /// Rounds since the last compaction at loop exit — the session-end
    /// compaction reports it honestly (P0-D review fix 2026-08-14).
    pub rounds_since_compact: u32,
    /// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the loop's
    /// final fold state — the caller's session-end compaction uses the
    /// same stateful view for its summary input and drains up to
    /// `fold_cut`. Per-loop local (each lane has its own conversation;
    /// a controller-level field would be clobbered by the nested
    /// subagent dispatch), so it is reset at every loop start.
    pub fold_state: crate::action_ledger::LedgerFoldState,
    /// Read by the subagent terminal mapping (M3) — the main caller's
    /// terminal event carries `tool_rounds` only.
    #[allow(dead_code)] // consumed by the subagent path (GAP-SUBAGENT-RUNTIME M3)
    pub budget_exhausted: bool,
}

/// P0-D review fix (2026-08-14, ADR-0010 v1.14): the reduction-guard retry
/// budget — a guard that cannot be satisfied retries on the next trigger
/// rounds (without interrupting content) and only after three consecutive
/// failures forces one compaction and reports `guard_failed`.
pub(crate) const GUARD_RETRY_LIMIT: u32 = 3;

/// FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
/// §14.28 审查修复): consecutive external-ledger append failures tolerated
/// before folding is disabled for the rest of the loop invocation — a
/// persistent write failure (disk full / permission / `.gsa/ledger` path
/// conflict) must degrade to the pre-fold full view, never spin the loop
/// on retries (设计 §8「推进失败不阻塞会话」).
pub(crate) const FOLD_WRITE_FAILURE_LIMIT: u32 = 3;

/// Outcome of one template-summary attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactDecision {
    /// Nothing collapsible (or the trigger did not reach this path).
    NoOp,
    /// Collapsible content exists but the reduction guard is not
    /// satisfiable and the compaction was not forced.
    GuardBlocked,
    /// A compaction executed (summary complete or `summary_incomplete`
    /// termination state).
    Executed { incomplete: bool },
}

/// The shared five-section template-summary flow (P0-D S3 + review fixes).
///
/// Used by the loop-top rhythm/fallback trigger and by the end-of-session
/// compaction (reason = "session_end", forced). Runs the summary ≤3 times,
/// persists the archive with bounded retries (an archive failure is
/// explicitly reported in the marker and the event), truncates the
/// conversation, inserts the rolling single marker and journals
/// `context_compressed` v0.2. v1.15 (2026-08-14): compaction never touches
/// the blackboard — the blackboard lifecycle is the plan epoch, not the
/// context window.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_template_compact(
    svc: &SharedLoopServices<'_>,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    agent: &dyn RoundAgent,
    messages: &mut Vec<Message>,
    measured: u64,
    reason: &str,
    force: bool,
    guard_failed: bool,
    rounds_since: u32,
    tail: usize,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): the external-ledger path hint for the marker —
    // main lane only (检索车道不折叠); the hint is further filtered to
    // files that exist or a fold that happened, so a restored
    // conversation never gets a dangling pointer.
    ledger_path: Option<&std::path::Path>,
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the loop's
    // stateful fold point — the summary input uses the same folded view
    // as the main requests (同源), the drain cut is `fold_cut` when
    // folded, and the state is reset once the conversation is mutated.
    fold_state: &mut crate::action_ledger::LedgerFoldState,
    cancel: Option<&tokio_util::sync::CancellationToken>,
    heartbeat: Option<&ActivityClock>,
) -> Result<CompactDecision, AgentLoopError> {
    // FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27 / 处理文档
    // LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18 §2.1): the rolling
    // single marker is NOT removed on the guard path — the frozen fold
    // indices (`fold_start`/`fold_cut`) are positions into the CURRENT
    // `messages` slice, and any in-place mutation before the fold state is
    // reset invalidates them. The previous unconditional `retain()` deleted
    // the marker even when the reduction guard then returned
    // `GuardBlocked` (fold untouched) → stale preamble `[U0, A[...]]`
    // (declaration without its tool reply in the fold region) → provider
    // 400 `insufficient tool messages`. The marker is removed only once
    // execution is confirmed; the indices are recomputed below.
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the drain cut
    // comes from the frozen fold state when folded (保留起点基于 fold_cut
    // 而非重算 tail); otherwise the stateless tail cut as before.
    let Some(kept_start) = fold_state
        .fold_cut
        .or_else(|| crate::action_ledger::collapsed_cut(messages, tail))
    else {
        return Ok(CompactDecision::NoOp);
    };
    let cfg = svc.context_compact;
    // Guard 口径：含旧 marker 的数组 + 冻结 kept_start（触发时不动数组）。
    let after = estimate_messages_tokens(&messages[..kept_start])
        + estimate_messages_tokens(&messages[kept_start..])
        + crate::summary::SUMMARY_MARKER_ESTIMATE_TOKENS;
    let removable = measured.saturating_sub(after);
    let reduction_ok = after as f64 <= measured as f64 * cfg.max_reduction_ratio;
    if !force && (removable < cfg.min_compactable || !reduction_ok) {
        return Ok(CompactDecision::GuardBlocked);
    }

    // 执行已确认：删除旧 marker 并重算 kept_start——marker 删除使冻结索引
    // 整体左移一位（折叠态 `fold_cut - had_marker`；marker 恒在 fold_cut
    // 之前：marker 插入点为首个声明，折叠 cut 恒在其后）。未折叠态按无
    // marker 数组重算 `collapsed_cut`。`after` 同步按无 marker 口径重算，
    // 事件估计与执行后数组一致。
    let had_marker = messages.iter().any(|m| {
        m.content
            .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
    });
    if had_marker {
        messages.retain(|m| {
            !m.content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
        });
    }
    let kept_start = fold_state
        .fold_cut
        .map(|cut| cut.saturating_sub(usize::from(had_marker)))
        .or_else(|| crate::action_ledger::collapsed_cut(messages, tail))
        .expect("guard path already resolved a kept_start for the same messages");
    let after = estimate_messages_tokens(&messages[..kept_start])
        + estimate_messages_tokens(&messages[kept_start..])
        + crate::summary::SUMMARY_MARKER_ESTIMATE_TOKENS;

    let rounds_dropped = crate::action_ledger::rounds_before(messages, kept_start) as u32;
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): the marker carries the fixed external-ledger path
    // hint so a restored conversation points the model at the surviving
    // history (the file is append-only and NOT reset by compaction). The
    // hint is written only when the file exists or the current window is
    // folded — never a dangling pointer for a conversation that never
    // folded.
    let ledger_hint = ledger_path.filter(|p| p.exists() || fold_state.is_folded());
    // The archive id rides the writer's CURRENT seq — no event is recorded
    // between here and the `context_compressed` journal, so the id is
    // stable and unique within the run.
    let id = format!("compaction-{}-{:04}", writer.run_id(), writer.seq());
    let archive_dir = host.session_cwd().join(".gsa").join("compaction");
    let archive_path = archive_dir.join(format!("{id}.md"));
    // v1.15 (2026-08-14): the path slot's overflow pointer targets the
    // current plan-epoch snapshot (the epoch archive is the permanent
    // holder of the full path/action records). F5 (2026-08-15): the path
    // comes from the controller's configured archive dir — the single
    // source — so a custom archive dir stays the real pointer target.
    let plan_epoch = svc.blackboard.read().plan.plan_epoch;
    let epoch_archive = (plan_epoch > 0).then(|| {
        svc.blackboard_archive_dir
            .map(|dir| dir.join(format!("epoch-{plan_epoch}.json")))
    });
    let epoch_archive = epoch_archive.flatten();
    let mechanical = {
        let bb = svc.blackboard.read();
        let (purpose, plan, paths) =
            crate::summary::mechanical_slots(&bb, &archive_path, epoch_archive.as_deref());
        crate::summary::SummarySlots {
            purpose,
            plan,
            paths,
            notes: String::new(),
            continuation: String::new(),
        }
    };
    // Summary input = mechanical slots + the collapsed history prefix
    // (tool records already mechanically handled by the first layer).
    let summary_input = {
        let mut input = vec![Message {
            role: Role::User,
            content: crate::summary::summary_user_prompt(&mechanical),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        }];
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the
        // summary input is the SAME stateful folded view as the main
        // requests — 摘要输入与主请求同源, never a separate stateless
        // recomputation (avoids a second measurement口径).
        input.extend(crate::action_ledger::build_request_view(
            messages, fold_state,
        ));
        input
    };
    let mut outcome: Option<crate::summary::SummarySlots> = None;
    for _attempt in 0..crate::summary::SUMMARY_MAX_ATTEMPTS {
        // P0-D review fix (2026-08-14): the summary call has its own 120s
        // wall-clock budget — a hung summarizer must fall through to the
        // retry/termination path instead of holding the run at the session
        // transport's far longer timeouts.
        let timed = tokio::time::timeout(
            crate::summary::SUMMARY_CALL_TIMEOUT,
            agent.run_round(
                &crate::summary::summary_system_prompt(),
                summary_input.clone(),
                Vec::new(),
                crate::summary::SUMMARY_MAX_TOKENS,
                cancel,
                heartbeat,
                &mut |_| {},
            ),
        )
        .await;
        match timed {
            Ok(Ok(resp)) => {
                match crate::summary::parse_model_output(
                    resp.text.as_deref().unwrap_or(""),
                    &mechanical,
                ) {
                    Ok(slots) => {
                        outcome = Some(slots);
                        break;
                    }
                    Err(_) => continue,
                }
            }
            Ok(Err(_)) | Err(_) => continue,
        }
    }

    let first_round_start = messages
        .iter()
        .position(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .unwrap_or(messages.len());
    if let Some(slots) = outcome {
        let markdown = crate::summary::summary_archive_markdown(
            &id,
            &slots,
            rounds_dropped,
            false,
            guard_failed,
            // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 + §14.28
            // external-file design): the archive preserves the folded view
            // the model saw — the byte-fixed pointer message. The folded
            // ROWS survive in the append-only external ledger file (never
            // drained by compaction), which the marker points a restored
            // conversation at. The fold state is still folded here (reset
            // happens after the drain).
            fold_state.folded_ledger.as_deref(),
        );
        let digest = crate::summary::archive_digest(&markdown);
        let archive_write_failed =
            !crate::summary::write_archive_retry(&archive_dir, &archive_path, &markdown);
        let marker = crate::summary::build_summary_marker(
            &id,
            &digest,
            &archive_path,
            &slots,
            rounds_dropped,
            false,
            guard_failed,
            archive_write_failed,
            plan_epoch,
            ledger_hint,
        );
        let messages_dropped = messages.drain(first_round_start..kept_start).count();
        messages.insert(
            first_round_start,
            Message {
                role: Role::User,
                content: marker,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
        );
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the
        // conversation was mutated (drain + marker) — the frozen fold
        // indices are stale. Reset; the view re-accumulates from the
        // marker (折叠重置为 None，marker 之后重新累积).
        fold_state.reset();
        writer
            .record(
                EventType::ContextCompressed,
                serde_json::json!({
                    "trigger_tokens": measured,
                    "target_tokens": after,
                    "rounds_since_last_compaction": rounds_since,
                    "rounds_dropped": rounds_dropped,
                    "messages_dropped": messages_dropped,
                    "messages_kept": messages.len(),
                    "estimated_tokens_after": after,
                    "mode": "template_summary",
                    "reason": reason,
                    "summary_id": id,
                    "summary_digest": digest,
                    "summary_path": archive_path.display().to_string(),
                    "summary_incomplete": false,
                    "retained_rounds": tail,
                    "guard_failed": guard_failed,
                    "archive_write_failed": archive_write_failed,
                }),
            )
            .await?;
        Ok(CompactDecision::Executed { incomplete: false })
    } else {
        // Termination state: mechanical slots only, marker flagged
        // `summary_incomplete`, wider recent tail; on the FALLBACK trigger
        // the conversation is still mechanically truncated so the run never
        // stays over the window (D2-2 emergency — the guard-failure path
        // itself never uses raw truncation, per the review fix).
        let mechanical_slots = crate::summary::SummarySlots {
            notes: String::new(),
            continuation: String::new(),
            ..mechanical
        };
        let marker = crate::summary::build_summary_marker(
            "summary-incomplete",
            "",
            &archive_dir,
            &mechanical_slots,
            rounds_dropped,
            true,
            guard_failed,
            false,
            plan_epoch,
            ledger_hint,
        );
        let mut dropped = rounds_dropped;
        let mut messages_dropped = 0usize;
        let mut final_after = after;
        if reason == "fallback" {
            let stats = compact_messages(messages, cfg.recovery_target_tokens);
            if stats.rounds_dropped > 0 {
                dropped = stats.rounds_dropped;
                messages_dropped = stats.messages_dropped;
                final_after = stats.estimated_tokens_after;
            }
        }
        let insert_at = messages
            .iter()
            .position(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
            .unwrap_or(messages.len());
        messages.insert(
            insert_at,
            Message {
                role: Role::User,
                content: marker,
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
        );
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): same reset
        // on the termination path — the conversation changed (marker and
        // possibly mechanical truncation), stale fold indices are dropped.
        fold_state.reset();
        if final_after == after {
            final_after = estimate_messages_tokens(messages);
        }
        writer
            .record(
                EventType::ContextCompressed,
                serde_json::json!({
                    "trigger_tokens": measured,
                    "target_tokens": final_after,
                    "rounds_since_last_compaction": rounds_since,
                    "rounds_dropped": dropped,
                    "messages_dropped": messages_dropped,
                    "messages_kept": messages.len(),
                    "estimated_tokens_after": final_after,
                    "mode": "template_summary",
                    "reason": reason,
                    "summary_id": null,
                    "summary_digest": null,
                    "summary_path": null,
                    "summary_incomplete": true,
                    "retained_rounds": tail + 1,
                    "guard_failed": guard_failed,
                    "archive_write_failed": false,
                }),
            )
            .await?;
        Ok(CompactDecision::Executed { incomplete: true })
    }
}

/// The shared model↔tool loop (M1 extraction, 2026-08-10).
///
/// Semantics preserved verbatim from `run_turn_inner`'s loop body:
/// pacing → compaction → orientation loop-top gap → system assembly →
/// model round → model_output journal → orientation feed → budget
/// exhaustion / final answer / counterexample gate / IPG → tool dispatch
/// (role-gated) → batch injections (denial breaker / edit push /
/// orientation post-tool-batch gap) → budget re-declaration.
///
/// `tool_defs` is the BASE list for main/grill turns (registry + main-only
/// additions + mode projection) — the loop re-probes the work tools before
/// EVERY model request and re-projects it (探针完整集 ∩ 会话声明集 + 非工作
/// 工具), emitting `tool_availability_check` only on flips against
/// the minimal previous-round map (P0-A step 5 / P0-A-2). Retrieval lanes pass
/// their FINAL list and never re-probe (no second
/// `tool_availability_check` event inside a lane). The call-time permission
/// gate remains the final backstop. `messages` is in/out: the conversation
/// continues across rounds; the caller owns the seed and the post-loop use
/// (stagnation / grill history writeback).
#[allow(clippy::too_many_arguments)] // the shared loop's full contract
pub(crate) async fn run_agent_loop(
    svc: &SharedLoopServices<'_>,
    controller: &AgentLoopController,
    writer: &mut EventWriter<'_>,
    host: &dyn LoopHost,
    agent: &dyn RoundAgent,
    profile: &LoopProfile,
    prompt: &str,
    tool_defs: &[ToolDef],
    messages: &mut Vec<Message>,
    mut orientation: Option<&mut OrientationSessionState>,
    cancel: Option<&tokio_util::sync::CancellationToken>,
    heartbeat: Option<&ActivityClock>,
) -> Result<LoopOutcome, AgentLoopError> {
    let workspace_trust = host.workspace_trust();
    // The session's budget counter (user adjudication 2026-08-10, review
    // F5): a `continue` re-entry is the same retrieval session — the
    // counter starts from the activation's consumed rounds and resets only
    // with a NEW activation (Closed → next creation starts at 0). The main
    // agent keeps the inherited per-run semantic (`initial_tool_rounds` 0).
    let mut tool_rounds = profile.initial_tool_rounds;
    // The `None` seed is required by Rust's initialization rules (the
    // value is overwritten on every break path before the read at the
    // end — clippy's unused_assignments is a false positive here).
    #[allow(unused_assignments)]
    let mut last_text: Option<String> = None;
    // D-8: after the budget is exhausted the model gets ONE final
    // no-tools round to report a partial result; if it still requests
    // tools, the run ends there (no execution of post-budget calls).
    let mut budget_exhausted = false;
    // §4.6 wiring state: the final-answer counterexample gate fires once
    // per run. The old mixed inquiry counters are gone (GAP-INQUIRY-SPLIT
    // 2026-08-09) — orientation counts live in the session-level
    // `OrientationSessionState` threaded through the turn chain; output
    // repetition belongs to the runtime stagnation guard only.
    let mut counterexample_fired = false;
    // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): a
    // checkpoint fire whose block was injected but whose forced-template
    // round has not completed yet (main lane only — retrieval lanes commit
    // at fire time and never set this). While pending, the loop skips
    // compaction and further fires, offers NO tools, and runs the template
    // round at the next loop-top.
    let mut pending_checkpoint: Option<PendingCheckpoint> = None;
    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 首轮计划轮硬门 —
    // 主车道且黑板尚无已批准计划（新会话/新 plan epoch）时触发；已有计划
    // （恢复会话或本会话后续 run）不重复触发；检索车道不触发。计划轮计入
    // 已完成逻辑模型轮（feed_round 照常），通过后计划落黑板 plan epoch。
    let mut plan_gate: Option<crate::planning::PlanGateState> = None;
    if controller.plan_first_enabled()
        && !controller.plan_first_session_done()
        && profile.role == AgentRole::Main
        && svc.blackboard.read().plan.plan_id.is_none()
    {
        plan_gate = Some(crate::planning::PlanGateState::start());
    }
    // P0-D (2026-08-14, ADR-0010 v1.10 / v1.14): template-summary state —
    // the previous round's MEASURED prompt tokens (provider usage; None
    // until the first round reports usage), the rounds since the last
    // summary (the 2-model-round cooldown; the 200K fallback bypasses it)
    // and the reduction-guard retry chain (GUARD_RETRY_LIMIT consecutive
    // failures force one compaction and report `guard_failed`).
    let mut last_prompt_tokens: Option<u64> = None;
    let mut rounds_since_compact: u32 = 0;
    let mut guard_failures: u32 = 0;
    // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the previous
    // model-request header fingerprint of THIS loop invocation — `None`
    // until the first request, so the first request journals `initial` and
    // only real prefix changes journal `change`.
    let mut last_request_header: Option<RequestHeader> = None;
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the loop's own
    // stateful fold point — per-loop local (see `LoopOutcome.fold_state`):
    // the same controller drives the main lane AND the nested retrieval
    // subagent lanes (each with its own conversation), so a controller-
    // shared field would be clobbered by the nested dispatch. Every loop
    // invocation starts unfolded and re-accumulates (restore semantics —
    // ADR-0010 §14.26 恢复路径: 恢复后均为 None，首次推进重写一次前缀，低频接受).
    let mut fold_state = crate::action_ledger::LedgerFoldState::default();
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): consecutive append-failure budget + degrade flag.
    // 失败时不再 `continue` 跳过本轮请求（持久失败 = 会话空转）；连续
    // FOLD_WRITE_FAILURE_LIMIT 次失败后本循环禁用折叠，视图退回全量原文，
    // 压缩仍兜底窗口。
    let mut fold_write_failures: u32 = 0;
    let mut fold_disabled = false;

    loop {
        // Cooperative cancellation checkpoint (Phase 3 slice #7): polled
        // BEFORE the pacing sleep so a cancel never waits on
        // TEXT_DELTA_PACING. Returning here terminates the run with a
        // `run_cancelled` journal event (recorded by `run_turn`).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }

        // Streaming ordering guard (Phase 3 slice #6): this round's text
        // deltas reach a live client at arrival rate, but the previous
        // round's `model_output` lands via the TUI journal tail — the
        // tail thread polls the file every 50ms AND the runner drains
        // its channel on a separate 50ms tick, so worst-case projection
        // is ~100ms after the fsync ack; TEXT_DELTA_PACING (2 ticks)
        // covers that alignment. Starting a new round's deltas before
        // that journal event is projected would append them to the
        // previous round's card (the dedup only clears
        // `current_model_index` on a matching model_output). The counter
        // lives on the controller (not per-turn) so a turn ≥ 2's FIRST
        // round is paced too — a programmatic client may chain prompts
        // faster than the tail projects the previous turn's terminal
        // events (review P3-5); user-paced TUIs are naturally safe.
        // Residual boundary (2026-08-05 review): a render stall beyond
        // ~10ms could still race — accepted. Note the sleep applies per
        // extra round even headless (deltas go nowhere): ~120ms × rounds
        // is the recorded cost of keeping the guard universal.
        if svc
            .pacing_rounds
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            > 0
        {
            tokio::time::sleep(TEXT_DELTA_PACING).await;
        }

        // P0-D (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §2-§4):
        // template-summary trigger at any safe loop-top gap (never inside a
        // batch). Measured = the previous round's provider prompt tokens on
        // the FOLDED request view. Two triggers:
        //   - RHYTHM: measured > trigger_tokens (192K, 2026-08-18
        //     adjudication ADR-0010 §14.26) with a ≥ min_rounds
        //     (2) model-round cooldown (v1.14 review fix).
        //   - FALLBACK (window guard): measured > safety_tokens (256K,
        //     2026-08-18 adjudication ADR-0010 §14.26), bypassing the
        //     cooldown — the emergency path must not stay over the line
        //     even when the summary fails (mechanical truncation
        //     fallback, D2-2 semantics).
        // Reduction guards: removable content ≥ min_compactable and kept ≤
        // max_reduction_ratio of before. P0-D review fix (2026-08-14): a
        // guard that cannot be satisfied is NOT skipped — it retries on the
        // next trigger rounds (no content interruption, no raw truncation)
        // and after GUARD_RETRY_LIMIT consecutive failures forces one
        // compaction and reports `guard_failed` for explicit handling.
        // A pending checkpoint round has priority over compaction — the
        // injected template block must reach the model before any window
        // collapse (§14.16: 触发点下一安全动作间隙暂停).
        let fallback_now = pending_checkpoint.is_none()
            && last_prompt_tokens.is_some_and(|m| m > svc.context_compact.safety_tokens);
        let rhythm_now = pending_checkpoint.is_none()
            && last_prompt_tokens.is_some_and(|m| m > svc.context_compact.trigger_tokens)
            && rounds_since_compact >= svc.context_compact.min_rounds;
        let summary_now = fallback_now || rhythm_now;
        let mut failure_widened_tail = false;
        if summary_now && let Some(measured) = last_prompt_tokens {
            let reason = if fallback_now { "fallback" } else { "rhythm" };
            let tail = svc.context_compact.recent_tail_rounds;
            // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18,
            // ADR-0010 §14.28 审查修复): the external ledger is the MAIN
            // lane's conversation projection — retrieval lanes never fold,
            // so their compaction marker carries no ledger hint.
            let ledger_hint = (profile.role == AgentRole::Main)
                .then(|| crate::action_ledger::ledger_file_path(&host.session_cwd()));
            match run_template_compact(
                svc,
                writer,
                host,
                agent,
                messages,
                measured,
                reason,
                false,
                false,
                rounds_since_compact,
                tail,
                ledger_hint.as_deref(),
                &mut fold_state,
                cancel,
                heartbeat,
            )
            .await?
            {
                CompactDecision::Executed { incomplete } => {
                    // A compaction consumed the window (complete or
                    // termination state): the cooldown restarts and the
                    // guard retry chain resets.
                    guard_failures = 0;
                    rounds_since_compact = 0;
                    failure_widened_tail = incomplete;
                }
                CompactDecision::NoOp => {
                    guard_failures = 0;
                }
                CompactDecision::GuardBlocked => {
                    // The guard cannot be satisfied THIS round — retry on
                    // the next trigger rounds without interrupting content;
                    // after the retry budget is exhausted, force one
                    // compaction and report the mechanism failure.
                    guard_failures += 1;
                    if guard_failures >= GUARD_RETRY_LIMIT {
                        match run_template_compact(
                            svc,
                            writer,
                            host,
                            agent,
                            messages,
                            measured,
                            reason,
                            true,
                            true,
                            rounds_since_compact,
                            tail,
                            ledger_hint.as_deref(),
                            &mut fold_state,
                            cancel,
                            heartbeat,
                        )
                        .await?
                        {
                            CompactDecision::Executed { incomplete } => {
                                rounds_since_compact = 0;
                                failure_widened_tail = incomplete;
                            }
                            _ => {}
                        }
                        guard_failures = 0;
                    }
                }
            }
        }

        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 + §14.28
        // external-file design): fold-advance trigger — the same safe
        // loop-top gap as compaction (never inside a tool batch; a
        // pending checkpoint round has priority: `pending_checkpoint` is
        // None here). When the ESTIMATED request view (chars/2) reaches
        // `fold_trigger_tokens` (default 128K) and complete old rounds
        // exist outside the tail, fold the NEW rounds into the external
        // ledger file ONCE and move `fold_cut` forward. The request view
        // becomes preamble + byte-fixed pointer + `[fold_cut..]` — the
        // prefix stays byte-stable across ALL advances (no per-window
        // rewrite), which is the v1.9 prefix-cache discipline the old
        // per-request stateless recomputation broke (measured hit rate
        // 81.9% → ~91–93%; 2026-08-18 design §1.3/§5). The advance never
        // mutates `messages` (journal/sidecar keep the full tool records
        // — audit dual-track unchanged; the external file is a
        // deterministically re-derivable projection).
        if pending_checkpoint.is_none() && profile.role == AgentRole::Main && !fold_disabled {
            // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18,
            // ADR-0010 §14.28 审查修复): 仅主车道折叠——检索子车道共享
            // session_cwd，外挂文件是主会话的模型可读投影；子车道窗口小，
            // 压缩机制已覆盖，避免多车道行混入同一文件（指针消息「本会话
            // 内固定」契约）。`fold_disabled` = 连续写失败后的降级开关。
            let fold_tail =
                svc.context_compact.fold_tail_rounds + if failure_widened_tail { 1 } else { 0 };
            let view_estimate = {
                let view = crate::action_ledger::build_request_view(messages, &fold_state);
                estimate_messages_tokens(&view)
            };
            if view_estimate >= svc.context_compact.fold_trigger_tokens {
                let ledger_path = crate::action_ledger::ledger_file_path(&host.session_cwd());
                let prev_fold = fold_state.clone();
                if let Some(rows) = crate::action_ledger::advance_fold(
                    messages,
                    &mut fold_state,
                    fold_tail,
                    &ledger_path,
                ) {
                    if crate::action_ledger::append_ledger_rows(&ledger_path, &rows).is_err() {
                        // External-file write failure must NOT advance the
                        // fold — the rows would be lost from both the view
                        // and the file. Roll back, count the failure and
                        // journal it; after the budget folding is disabled
                        // for this loop. Deliberately NOT `continue`: the
                        // session keeps making progress with the
                        // rolled-back (unfolded) view and the next loop-top
                        // retries — a persistent failure must never spin
                        // the loop without a model call (设计 §8「推进失败
                        // 不阻塞会话」, 2026-08-18 审查修复).
                        fold_state = prev_fold;
                        fold_write_failures += 1;
                        let disabled = fold_write_failures >= FOLD_WRITE_FAILURE_LIMIT;
                        if disabled {
                            fold_disabled = true;
                        }
                        tracing::warn!(
                            ledger_path = %ledger_path.display(),
                            rows = rows.len(),
                            attempts = fold_write_failures,
                            disabled,
                            "external ledger append failed — fold state rolled back; folding disabled after the budget"
                        );
                        writer
                            .record(
                                EventType::LedgerFoldWriteFailed,
                                serde_json::json!({
                                    "ledger_path": ledger_path.display().to_string(),
                                    "attempt": fold_write_failures,
                                    "disabled": disabled,
                                    "rows": rows.len(),
                                    "view_estimate_tokens": view_estimate,
                                    "agent_role": profile.role.as_str(),
                                }),
                            )
                            .await?;
                    } else {
                        fold_write_failures = 0;
                        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010
                        // §14.26): one mechanical fold advance — the
                        // request-view prefix is stable (pointer never
                        // rewritten), but `fold_cut` moves and the view
                        // grows by pure append; the event carries the new
                        // fold point, the triggering estimate and the
                        // POST-advance estimate (触发复位证明：推进后视图
                        // < 阈值——2026-08-18 审查修复补足 S2 测试 #3 的直接
                        // 断言口径). Emitted only on a real advance.
                        let view_after = {
                            let view =
                                crate::action_ledger::build_request_view(messages, &fold_state);
                            estimate_messages_tokens(&view)
                        };
                        writer
                            .record(
                                EventType::LedgerFoldAdvance,
                                serde_json::json!({
                                    "fold_start": fold_state.fold_start,
                                    "fold_cut": fold_state.fold_cut,
                                    "rounds_folded": fold_state
                                        .fold_cut
                                        .map(|cut| crate::action_ledger::rounds_before(messages, cut))
                                        .unwrap_or(0),
                                    "view_estimate_tokens": view_estimate,
                                    "view_estimate_after": view_after,
                                    "agent_role": profile.role.as_str(),
                                }),
                            )
                            .await?;
                    }
                } else {
                    // Anti-spin no-op (no complete round outside the
                    // tail): emits nothing — the repeated O(view)
                    // estimation until a new round arrives is accepted.
                }
            }
        }

        // GAP-INQUIRY-SPLIT (2026-08-09) — FALLBACK orientation injection
        // point (loop-top): a post-tool-batch gap exists only on tool
        // rounds. When the 7th completed round is a FINAL round (no tool
        // batch — the turn breaks right after), the crossing is carried
        // here on the next loop-top: pacing + compaction done, before
        // the system prompt is built. (A deny round DOES have a tool
        // batch — its crossing fires in the post-tool-batch gap like any
        // tool round; review D3-1.) This is also the recovery resume
        // point (a restored session's persisted count crosses here).
        // Fires at most once per loop iteration (commit resets the
        // lane), so the two injection points never double-fire.
        // One pending checkpoint at a time — while a forced-template round
        // is pending, neither family may fire again (the counts are not
        // committed yet, so the gate is the only thing preventing a
        // double-fire at the next loop-top).
        if pending_checkpoint.is_none()
            && let Some(role) = profile.orientation_role
            && let Some(record) = controller
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    "loop_top_gap",
                    // Main lane: the checkpoint round is forced (no tools,
                    // template answer); retrieval lanes keep the legacy
                    // fire-and-continue behavior (§14.16 检索车道不变).
                    profile.role == AgentRole::Main,
                )
                .await?
        {
            pending_checkpoint = Some(PendingCheckpoint::Orientation { record, attempt: 1 });
        }
        // M5 (2026-08-10): DC checkpoint fire at the same safe gap (the
        // stage threshold was met by signals consumed in a prior tool
        // round). §14.16: one checkpoint round at a time — when Orientation
        // and DC are due in the same gap, Orientation wins and DC fires at
        // the next safe gap after its template round completes.
        if pending_checkpoint.is_none()
            && profile.dc_enabled
            && let Some(payload) = maybe_fire_dc(svc.dc_state, writer, messages).await?
        {
            pending_checkpoint = Some(PendingCheckpoint::DiagnosticCoverage {
                payload,
                attempt: 1,
            });
        }

        // FUS-TOOL-PROBE P0-A-2 (design §4/§5 v0.2): per-round work-tool
        // refresh before EVERY model request. The minimal previous-round
        // map (seeded by the pre-run_started event in `run_turn_inner`)
        // decides flip-only events: an unchanged partition emits nothing;
        // a flip journals the fresh snapshot and re-projects the visible
        // list (探针完整集 ∩ 会话声明集 + 非工作工具, names only). Retrieval
        // lanes never re-probe — their list is the caller's final
        // projection (no second `tool_availability_check` inside a lane).
        // The call-time permission gate remains the final backstop (design
        // invariant 2).
        // P0-C S3 (2026-08-15): 探针快照提升到循环顶部作用域——同一快照
        // 同时驱动模型可见工具投影与注册板块投影（Profile/Bundle ∩ 探针
        // 完整集），避免两处各探一次导致投影不一致。
        let probe_snapshot: Option<crate::tool_probe::ToolProbeSnapshot> =
            if pending_checkpoint.is_some() {
                // §14.16: a checkpoint round is a tool-free pause — no
                // registry projection and no probe.
                None
            } else if profile.probe_work_tools {
                let probe_context = crate::tool_probe::ProbeContext {
                    cwd: host.session_cwd(),
                    policy: host.tool_policy(),
                    test_runner_present: host.test_runner().is_some(),
                    interactive_user: host.interactive_user(),
                    goal_context_present: controller.goal_context_present(),
                    pending_retrieval_activation: controller.has_live_activation(),
                    terminal_available: host.terminal_available(),
                    lsp_configured: host.lsp_configured(),
                    memory_enabled: host.memory_enabled(),
                    image_backend_configured: host.image_backend_configured(),
                    video_backend_configured: host.video_backend_configured(),
                    mcp_registry_available: host.mcp_registry_available(),
                };
                let snapshot = crate::tool_probe::probe_work_tools(&probe_context);
                if controller.probe_flip(&snapshot) {
                    writer
                        .record(
                            EventType::ToolAvailabilityCheck,
                            AgentLoopController::tool_availability_payload(&snapshot),
                        )
                        .await?;
                }
                // PLAN-FIRST 阶段 B (2026-08-16): 记录主车道本轮探针源——
                // 工具栏投影与注册板块（黑板模型栏）共用的单一事实源；
                // 检索车道无操作台，不记录。
                if profile.role == AgentRole::Main {
                    controller.set_console_probe_source(host.tool_policy(), snapshot.clone());
                }
                Some(snapshot)
            } else {
                None
            };
        let current_tool_defs: Vec<ToolDef> = if pending_checkpoint.is_some() {
            Vec::new()
        } else if plan_gate.is_some() {
            // 首轮计划轮面：只暴露黑板读取 + plan_write（设计 §2.3/§3）。
            tool_defs
                .iter()
                .filter(|t| {
                    t.name == crate::planning::PLAN_WRITE_TOOL
                        || t.name == crate::planning::BLACKBOARD_READ_TOOL
                })
                .cloned()
                .collect()
        } else if let Some(snapshot) = probe_snapshot.as_ref() {
            let projected = AgentLoopController::project_main_agent_tool_defs(tool_defs, snapshot);
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §4):
            // console 默认面收敛——主车道无执行工具（读面=黑板+只读核查，
            // 写面=plan_write+action_write；执行/变更/shell/子代理/检索
            // 全部隐藏，经订单下发）。direct 模式恢复探针过滤后的工作
            // 工具投影（§7 双模式）。
            if controller.console_default_enabled()
                && controller.console_mode() == crate::console_mode::ConsoleMode::Console
            {
                AgentLoopController::project_console_default_tool_defs(&projected, snapshot)
            } else {
                projected
            }
        } else {
            tool_defs.to_vec()
        };
        // P0-C orz 内嵌集成 S2 (2026-08-15): 注册板块每轮机械刷新（主车道；
        // 检索车道无操作台）。内容 = 动作名 + 最小参数提示（最小提示由
        // `console::ServiceRegistry` 生成，不复制完整 schema）；板块常驻、
        // 内容按需读（模型用 blackboard_read section=actions 取回）。
        // PLAN-FIRST 阶段 B (2026-08-16): 刷新收敛为「记录探针源 → 派生
        // 注册板块」单一路径——与模型可见工具投影共用同一探针源（工具栏
        // 绑定黑板模型栏）；checkpoint 轮/无探针轮次不改写板块，保留上
        // 一轮探针过滤后的内容（替代 bundle-only 静态刷新中间态）。
        if profile.role == AgentRole::Main && pending_checkpoint.is_none() {
            controller.sync_console_registrations();
        }

        // 2026-08-12 裁决（ADR-0010 §3.5 v1.x）：AVAILABLE 块不再注入——
        // 模型可见工具列表 = API tools 参数中的 registry 能力目录（全量，
        // 零可用性承诺）；可用性判定完全发生在调用时。prompt 不再承载
        // 任何"可用性声明"（不固定在 prompt 中）。
        // D-8 (FIX_PLAN 2026-08-06): the round budget is declared to the
        // model up front — it does not guess or drift. The remaining
        // count is re-declared mechanically after every tool round.
        // Cache-prefix fix (2026-08-07): the session block is static
        // (BUDGET only) so the rebuilt system prompt is byte-identical
        // across rounds — the provider's prefix cache keeps hitting.
        let budget_block = crate::prompt::tool_round_budget_session_block(profile.max_tool_rounds);
        let system = match &profile.system_kind {
            SystemPromptKind::Main => {
                let system_blocks = budget_block.clone();
                controller
                    .main_agent
                    .prompt_builder
                    .build_system_prompt(Some(&system_blocks))
            }
            SystemPromptKind::Retrieval { role, goal, mode } => {
                // ADR-0010 §3.2 task contract — the subagent's own system
                // (citation rules + [DOC]/[SOURCE] delivery contract), with
                // the shared budget declaration.
                crate::prompt::build_retrieval_system_prompt(
                    role.section_name(),
                    goal,
                    mode.as_str(),
                    &budget_block,
                )
            }
        };
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划型执行框架
        // 在系统提示词层无条件注入（plan_first 会话；主/检索子代理同一
        // 入口）——不依赖 AGENTS.md 是否存在（D2 全覆盖，审查收口
        // 2026-08-16）。常量跨轮稳定，不影响前缀缓存。
        let system = if controller.plan_first_enabled() {
            format!(
                "{}\n\n{}",
                crate::planning::PLAN_FIRST_FRAMEWORK_BLOCK,
                system
            )
        } else {
            system
        };
        // D-6 (FIX_PLAN 2026-08-06): subagents get the full 160K budget
        // too (the main agent's request-level cap — §3.4.2 same defaults).
        // Review F3 (2026-08-10): ONE constant for both lanes — the three
        // agents must never carry their own literals.
        let max_tokens = match &profile.system_kind {
            SystemPromptKind::Main => controller.main_agent_max_tokens(),
            SystemPromptKind::Retrieval { .. } => REQUEST_MAX_TOKENS,
        };

        // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9):
        // request-header留痕 — journal the fingerprint (system + tools +
        // config digests) only when it changes from the previous request of
        // this loop (first request = `initial`). The event precedes the
        // model round so a prefix-cache miss is attributable and probe
        // flips (which re-projected `current_tool_defs` above) are
        // cross-checkable against the actual request shape.
        // Boundary (2026-08-15 review, ADR-0010 §14.9/审计 §5): auxiliary
        // model requests OUTSIDE this loop — compaction summary calls
        // (`run_template_compact`) and fast-preflight gates — do not emit
        // header events by design: their header (fixed system prompt +
        // empty tools + config) is constant and never interacts with probe
        // flips, so they are deliberately excluded from the留痕 chain.
        let current_header =
            compute_request_header(&system, &current_tool_defs, &agent.config_fingerprint());
        if last_request_header
            .as_ref()
            .is_none_or(|prev| prev.header_sha256 != current_header.header_sha256)
        {
            let reason = if last_request_header.is_none() {
                "initial"
            } else {
                "change"
            };
            let previous_sha = last_request_header
                .as_ref()
                .map(|prev| prev.header_sha256.clone());
            let change_kind = last_request_header
                .as_ref()
                .map(|prev| header_change_kind(prev, &current_header));
            writer
                .record(
                    EventType::RequestHeaderChange,
                    request_header_payload(
                        &current_header,
                        reason,
                        previous_sha.as_deref(),
                        profile.role.as_str(),
                        change_kind,
                    ),
                )
                .await?;
            last_request_header = Some(current_header);
        }

        let mut partial_text: Vec<String> = Vec::new();
        // P0-D S2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN
        // §3) + FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the
        // MODEL-VISIBLE view folds completed old tool rounds into
        // deterministic action-ledger rows (bounded recent tail kept
        // verbatim) — statefully: the fold point advances only at the
        // mechanical trigger (loop-top, view estimate ≥ 128K), and
        // between advances the view prefix is byte-stable (pure append,
        // restoring the v1.9 prefix-cache discipline). `messages` itself
        // stays full for journal/sidecar audit, so the persisted
        // conversation keeps the complete records.
        // 2026-08-18 (ADR-0010 §14.25 项 1): 常驻状态行移出系统提示词——
        // 每轮请求前把 `[任务状态]` 作为尾随用户消息、仅在变化时追加
        // （与 `[TOOL_ROUND_BUDGET] REMAINING` 同纪律），system 提示词
        // 保持完全静态，前缀缓存不被步骤推进打断。
        controller.sync_status_line_message(messages);
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the request
        // view comes from the stateful fold point — `messages` verbatim
        // until the first mechanical advance, then preamble + frozen
        // ledger + `[fold_cut..]` (byte-stable prefix, pure append).
        let request_messages = crate::action_ledger::build_request_view(messages, &fold_state);
        // FUS-LEDGER-FOLD-STATE 复验取证 (2026-08-18)：`ORZ_DEBUG_VIEW=1`
        // 时在请求失败路径 dump 实际发送的视图角色序列（定位折叠/压缩
        // 交互下的消息配对破坏点；正常路径零成本）。
        let view_debug = std::env::var("ORZ_DEBUG_VIEW").is_ok().then(|| {
            request_messages
                .iter()
                .map(|m| match m.role {
                    Role::Assistant => format!(
                        "A[{}]",
                        m.tool_calls
                            .iter()
                            .map(|t| t.call_id.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    Role::Tool => {
                        format!("T[{}]", m.tool_call_id.as_deref().unwrap_or("?"))
                    }
                    _ => "U".to_string(),
                })
                .collect::<Vec<_>>()
        });
        let response = match agent
            .run_round(
                &system,
                request_messages,
                current_tool_defs.clone(),
                max_tokens,
                cancel,
                heartbeat,
                &mut |chunk| {
                    // F-06 (2026-08-07 review): accumulate the streamed
                    // content deltas — on an abort (watchdog/timeout)
                    // the partial output must still reach the journal.
                    partial_text.push(chunk.to_string());
                    // Subagent text has no live consumer — deltas are
                    // dropped (F-03, matching the pre-split one-shot pass).
                    if profile.role == AgentRole::Main {
                        host.on_text_delta(chunk);
                    }
                },
            )
            .await
        {
            Ok(r) => r,
            // Phase 3 slice #11 (P3-7): a cancellation observed mid-stream
            // is a cancel, not a model failure — it must end the run with
            // `run_cancelled`, not a spurious `run_failed`.
            Err(GatewayError::Cancelled) => {
                return Err(AgentLoopError::Cancelled);
            }
            Err(other) => {
                if let Some(roles) = view_debug.as_ref() {
                    tracing::error!(
                        "model request failed ({other}); view roles: {}",
                        roles.join(" ")
                    );
                }
                // F-06 (D-7 "保留输出 + incomplete 标记 + 明确终止原因"): a
                // stream that aborted after producing partial content
                // must not lose it from the audit trail — journal it as
                // an incomplete model output BEFORE the terminal event
                // records the failure. Previously the partial text went
                // only to live deltas; the journal had a run_failed with
                // no trace of what was produced (2026-08-07 review).
                if !partial_text.is_empty() {
                    writer
                        .record(
                            EventType::ModelOutput,
                            serde_json::json!({
                                "text": partial_text.concat(),
                                "tool_calls": [],
                                // No natural finish reached — closest
                                // enum value; the terminal run_failed
                                // carries the real abort reason.
                                "finish_reason": "length",
                                "reasoning_tokens": null,
                                "completion_tokens": null,
                                "cache_hit_tokens": null,
                                "cache_miss_tokens": null,
                                "incomplete": true,
                            }),
                        )
                        .await?;
                }
                return Err(AgentLoopError::Model(other.to_string()));
            }
        };

        // Cooperative cancellation checkpoint (Phase 3 slice #7): a
        // cancelled run may omit this round's `model_output` — the chain
        // stays valid (the terminal event follows).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }

        writer
            .record(
                EventType::ModelOutput,
                serde_json::json!({
                    "text": response.text,
                    "tool_calls": response.tool_calls.iter().map(|tc| {
                        serde_json::json!({
                            "name": tc.name,
                            "arguments": tc.arguments,
                            "call_id": tc.call_id,
                        })
                    }).collect::<Vec<_>>(),
                    "finish_reason": match response.finish_reason {
                        FinishReason::Stop => "stop",
                        FinishReason::ToolCalls => "tool_calls",
                        FinishReason::Length => "length",
                    },
                    // D-6 usage observation — reasoning tokens per round
                    // calibrate the 160K budget decision (data → whether
                    // the budget rolls back).
                    "reasoning_tokens": response.reasoning_tokens,
                    "completion_tokens": response.completion_tokens,
                    // Cache-hit observation (2026-08-07 fix): per-round
                    // hit/miss tokens verify the prefix-cache fix — the
                    // hit rate jumped from ~17% (per-round REMAINING in
                    // the rebuilt system prompt) to 98%+ steady-state /
                    // ~80% incl. cold start (live-verified 2026-08-07).
                    "cache_hit_tokens": response.cache_hit_tokens,
                    "cache_miss_tokens": response.cache_miss_tokens,
                }),
            )
            .await?;

        // P0-D: track the round — measured prompt tokens feed the next
        // loop-top trigger check; the round counter is the summary
        // cooldown.
        last_prompt_tokens = response.prompt_tokens;
        rounds_since_compact += 1;

        // GAP-INQUIRY-SPLIT (2026-08-09): the model round just COMPLETED —
        // count it against the session-level orientation counter (ADR-0010
        // §4.2: completed logical model rounds; this point is the only
        // completion point that every round passes — tool rounds, deny
        // rounds and gate-answer rounds alike — while transport retries
        // never reach it, so they never count). Output repetition is
        // consumed ONLY by the runtime stagnation guard at end of turn —
        // the old per-round double-consumption is deleted.
        if let (Some(o), Some(role)) = (orientation.as_deref_mut(), profile.orientation_role) {
            o.feed_round(role);
        }

        // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16):
        // a pending checkpoint's model round is the FORCED TEMPLATE round —
        // no tools were offered, so nothing may execute. Validate the JSON
        // answer, journal the `checkpoint_response` event (parsed fields +
        // mechanical validation + evidence-identity cross-check + degrade
        // reason), and either ask once for a re-fill or commit the fire
        // (accepted/degraded, §2.4). The checkpoint answer itself counts as
        // one completed logical model round (feed above) and stays in the
        // conversation; the loop always continues after the branch.
        if let Some(pending) = pending_checkpoint.take() {
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
            // console 双模式询问轮——模板不同（decision/reason），无工具轮
            // 语义同 checkpoint；一次重填、仍失败默认 stay；每 run 至多一次。
            if let PendingCheckpoint::ConsoleModeInquiry {
                attempt,
                streak: _,
                order_ids: _,
            } = &pending
            {
                let text = response.text.as_deref().unwrap_or_default();
                let parsed = crate::console_mode::parse_and_validate_inquiry(text);
                let answer = match parsed {
                    Ok(answer) => Some(answer),
                    Err(errors) if *attempt < crate::console_mode::MAX_INQUIRY_ATTEMPTS => {
                        messages.push(Message {
                            role: Role::User,
                            content: crate::console_mode::inquiry_refill_feedback(&errors),
                            tool_call_id: None,
                            tool_calls: Vec::new(),
                            reasoning_content: None,
                        });
                        pending_checkpoint = Some(pending.with_attempt(*attempt + 1));
                        continue;
                    }
                    Err(_) => None,
                };
                match answer {
                    Some(answer) if answer.decision == "switch" => {
                        controller
                            .switch_console_to_direct(writer, tool_rounds, answer.reason.as_deref())
                            .await?;
                    }
                    _ => {
                        // stay（含降级默认 stay）：写 transition 事件 + gate_log。
                        controller
                            .record_console_stay(
                                writer,
                                tool_rounds,
                                answer
                                    .as_ref()
                                    .and_then(|a| a.reason.as_deref())
                                    .or(Some("degraded: invalid template after refill")),
                            )
                            .await?;
                    }
                }
                // 询问轮的回答是模型输出——保留在会话中。
                if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                }
                continue;
            }
            let attempt = pending.attempt();
            let mut verdict =
                checkpoint::parse_and_validate(response.text.as_deref().unwrap_or_default());
            // A checkpoint round may never dispatch tools — a tool_calls
            // response is a template violation (the calls are not executed).
            if !response.tool_calls.is_empty() {
                verdict.errors.push("tool_calls_not_allowed".to_string());
            }
            // §2.3 缓解必做: `progress_evidence` (and the gathered-evidence
            // missing surface) cross-checked against journal evidence
            // identities — the main lane's own evidence + committed
            // retrieval ledger ids/refs + DC examined-surface ids.
            let mut identities: std::collections::HashSet<String> = controller
                .checkpoint_source_identities()
                .into_iter()
                .collect();
            if let Some(evidence) = svc.evidence {
                identities.extend(
                    evidence
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|record| record.identity.clone()),
                );
            }
            {
                let dc = svc.dc_state.lock().unwrap();
                identities.extend(dc.evidence_ids.iter().cloned());
            }
            let cross = checkpoint::cross_check(verdict.response.as_ref(), &identities);
            let outcome = checkpoint::decide_outcome(attempt, &verdict.errors);
            let (outcome_str, degrade_reason) = match outcome {
                checkpoint::CheckpointRoundOutcome::Accepted => ("accepted", None),
                checkpoint::CheckpointRoundOutcome::RefillRequested => ("refill_requested", None),
                checkpoint::CheckpointRoundOutcome::Degraded { reason } => {
                    ("degraded", Some(reason))
                }
            };
            writer
                .record(
                    EventType::CheckpointResponse,
                    checkpoint::checkpoint_response_payload(
                        &pending,
                        attempt,
                        outcome_str,
                        &verdict,
                        &cross,
                        degrade_reason,
                    ),
                )
                .await?;
            // The template answer is model output — keep it in the
            // conversation (the re-fill feedback below is injected text).
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
            }
            match outcome {
                checkpoint::CheckpointRoundOutcome::Accepted
                | checkpoint::CheckpointRoundOutcome::Degraded { .. } => {
                    // §2.4: only a completed template round (accepted or
                    // degraded) commits the fire / advances the DC stage.
                    checkpoint::commit_pending(pending, orientation.as_deref_mut(), svc.dc_state);
                }
                checkpoint::CheckpointRoundOutcome::RefillRequested => {
                    messages.push(Message {
                        role: Role::User,
                        content: checkpoint::refill_feedback_block(&verdict.errors),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                    pending_checkpoint = Some(pending.with_attempt(attempt + 1));
                }
            }
            continue;
        }

        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 首轮计划轮——
        // 无工具回复（未写 plan_write）不算最终答案：继续计划面；连续
        // MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION 轮未提交则机械降级
        // （plan_not_submitted 留痕）并放行，绝不挂死。
        if plan_gate.is_some() && response.tool_calls.is_empty() {
            let gate = plan_gate.as_mut().expect("checked above");
            gate.rounds_without_submission += 1;
            if gate.rounds_without_submission >= crate::planning::MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION
            {
                let g = plan_gate.take().expect("checked above");
                writer
                    .record(
                        EventType::PlanWrite,
                        crate::planning::plan_write_payload(
                            "",
                            "",
                            0,
                            "degraded",
                            g.attempt,
                            &crate::planning::PlanVerdict::default(),
                            Some("plan_not_submitted"),
                        ),
                    )
                    .await?;
                // 降级后按普通最终答案路径继续（下面的 final-answer 分支处理）。
            } else {
                if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                    messages.push(Message {
                        role: Role::Assistant,
                        content: text,
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                }
                continue;
            }
        }

        // D-8: the post-exhaustion final round may only produce TEXT — a
        // tool request there is refused (no execution after the budget is
        // gone) and the run ends with the partial result. Checked BEFORE
        // the final-answer path so the exhaustion round never triggers
        // the counterexample gate's extra model round.
        if budget_exhausted {
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
            }
            last_text = response.text;
            break;
        }

        if response.tool_calls.is_empty() {
            // §4.6.1/4.6.2: the first no-tool-call response is a
            // final-answer candidate — before committing it, the
            // counterexample gate fires ONCE (the block explicitly tells
            // the model it appears only once). The candidate is journaled
            // as model_output (evidence) but not committed to the
            // conversation; the post-gate response is the final answer
            // (D6). A post-gate round that returns tool calls continues
            // the loop normally — the gate never fires again this run.
            // Grill mode (2026-08-08): the counterexample gate is a
            // run-semantic (final answers); a grill question is not one
            // — skipped.
            if !counterexample_fired && profile.counterexample_gate {
                writer
                    .record(
                        EventType::CounterexampleGate,
                        serde_json::json!({
                            "position": "final_answer",
                            "message_block": COUNTEREXAMPLE_GATE_BLOCK,
                            "once_only": true,
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::User,
                    content: COUNTEREXAMPLE_GATE_BLOCK.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
                counterexample_fired = true;
                continue;
            }
            // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): output-level
            // citation verifier (ADR-0010 §3.7.9 / RETRIEVAL_MECHANICAL
            // _CONTROLS_DESIGN §3.2) — the final answer is formed; validate
            // it at the same delivery boundary as the counterexample gate,
            // BEFORE it is committed to the conversation. A failure replaces
            // the delivered text with a mechanical degradation block (with
            // reason codes), journals `citation_validation`, and never
            // commits the model text as the final answer.
            if profile.citation_validation {
                let report = controller
                    .validate_final_answer_citations(response.text.as_deref().unwrap_or_default());
                if !report.passed {
                    let block =
                        crate::prompt::citation_validation_failed_block(&report.reason_codes);
                    writer
                        .record(
                            EventType::CitationValidation,
                            serde_json::json!({
                                "schema_version": "0.2.0-draft",
                                "position": "final_answer",
                                "decision": "block",
                                "marker_count": report.markers.len(),
                                "reason_codes": report.reason_codes,
                                "degraded": true,
                                "message_block": block,
                                "markers": report.markers.iter().map(|m| {
                                    serde_json::json!({
                                        "index": m.index,
                                        "raw": m.raw,
                                        "target": m.target,
                                        "binding": m.binding.as_str(),
                                        "status": if m.status
                                            == crate::citation_validation::MarkerStatus::Passed
                                        {
                                            "passed"
                                        } else {
                                            "failed"
                                        },
                                        "reason_codes": m.reason_codes,
                                    })
                                }).collect::<Vec<_>>(),
                            }),
                        )
                        .await?;
                    last_text = Some(block);
                    break;
                }
            }
            // Include the final assistant message in the conversation so
            // stagnation sees the model's actual output and the rebuilt
            // dialogue matches what a real transport would have received
            // (2026-08-04 review P2-2).
            if let Some(text) = response.text.clone().filter(|t| !t.is_empty()) {
                messages.push(Message {
                    role: Role::Assistant,
                    content: text,
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
            }
            last_text = response.text;
            break;
        }

        // IP3a: instruction provenance gate — evaluated once per tool
        // phase. A block stops every tool and ends the phase without
        // further model calls (the instruction stream is poisoned).
        let ipg = ToolDispatcher::evaluate_ipg(prompt, workspace_trust);
        if matches!(ipg, GateDecision::Block { .. }) {
            writer
                .record(
                    EventType::InstructionProvenanceGate,
                    serde_json::json!({
                        "decision": ipg.decision_str(),
                        "entries": 1,
                    }),
                )
                .await?;
            writer
                .record(
                    EventType::GateDecision,
                    serde_json::json!({
                        "gate": "instruction_provenance_gate",
                        "decision": "block",
                        "tools": response.tool_calls.iter().map(|tc| tc.name.clone()).collect::<Vec<_>>(),
                    }),
                )
                .await?;
            {
                let mut w = svc.blackboard.write();
                w.gate_log
                    .gate_decisions
                    .push("IPG: block (tool phase)".to_string());
            }
            last_text = response.text;
            break;
        }

        // Execute tool calls, feeding results back into the conversation.
        // Cooperative cancellation checkpoints (Phase 3 slice #7): the
        // loop-top check before dispatch plus a per-tool re-check inside
        // — a cancel landing while tool #k runs must not start tools
        // #k+1..N of the same round (2026-08-05 review P2-2; the comment
        // "never starts a new tool" holds per tool, not per round).
        if cancel.is_some_and(|c| c.is_cancelled()) {
            return Err(AgentLoopError::Cancelled);
        }
        // Replay the assistant's call declarations BEFORE their results:
        // the provider protocol requires each tool message's
        // `tool_call_id` to match a declaration in the history, and
        // DeepSeek rejects unmatched ids (2026-08-06 design review D2-1).
        // The reasoning content rides the same declaration message —
        // DeepSeek expects it replayed with the assistant turn
        // (alpha-test 2026-08-06 closure).
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: response.tool_calls.clone(),
            reasoning_content: response.reasoning_content.clone(),
        });
        let mut assistant_parts: Vec<String> = Vec::new();
        // Pending policy messages (denial breaker) — appended AFTER the
        // tool batch completes so no user message lands between the
        // assistant declaration and its tool replies (provider protocol;
        // 2026-08-07 wordy 400 + review P1).
        let mut pending_policy: Vec<Message> = Vec::new();
        // ADR-0010 §3.5.4 round-level denial aggregation: the breaker
        // counts ROUNDS (a round with N denied calls and no success
        // counts 1), keyed by (tool, reason_code, policy_revision).
        let mut round_denials: Vec<crate::controller::DenialKey> = Vec::new();
        let mut round_had_success = false;
        // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): the accumulated
        // estimated tokens of THIS model round's injected tool results
        // (chars/2 — the same estimator as compaction). Once at/over the
        // budget, the remaining calls of the batch are refused without
        // execution and told to continue with offset/grep-first.
        let mut round_inject_tokens: u64 = 0;
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 本轮是否提交过
        // plan_write —— 无提交的工具轮计入 plan_not_submitted 降级计数。
        let mut plan_write_called = false;
        // 2026-08-16 审查收口：计划轮不消耗 tool-round 预算（用户裁决）——
        // 在批处理前固定本轮的“计划轮身份”，即使 plan_write 中途 accepted
        // 解除门，本轮仍按计划轮处理（不 +1）；whitelist 的 tool_rounds==0
        // 窗口因此顺延到计划落板后的首个工具轮（P2-2 顺延裁决）。
        let plan_round_active = plan_gate.is_some();
        // 2026-08-08 blackboard partition (A2): snapshot the edit-action
        // length BEFORE this round's tools — the incremental push after
        // the batch reports exactly the records this round added.
        let round_edit_count = svc.blackboard.read().edits.len();
        for tc in &response.tool_calls {
            if cancel.is_some_and(|c| c.is_cancelled()) {
                return Err(AgentLoopError::Cancelled);
            }
            if round_inject_tokens >= svc.max_inject_tokens_per_round {
                let (result, feedback) = refuse_inject_budget(
                    writer,
                    messages,
                    tc,
                    round_inject_tokens,
                    svc.max_inject_tokens_per_round,
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                match feedback {
                    Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                    Some(PolicyFeedback::Succeeded) => round_had_success = true,
                    None => {}
                }
                assistant_parts.push(format!("[{}] {}", tc.name, result.output));
                continue;
            }
            // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划轮只允许
            // blackboard_read + plan_write；其余工具即使被声明也机械拒绝
            // （首轮禁止 执行/变更/shell/子代理/检索/action_write）。
            if plan_round_active && tc.name == crate::planning::PLAN_WRITE_TOOL && plan_write_called
            {
                // P3-5 (2026-08-16): 同一计划轮最多一次 plan_write ——
                // 重填反馈在下一轮注入，同轮第二次提交无意义且会绕过
                // “错误反馈后重填”的交互语义。
                let (r, f) = plan_round_denied(
                    writer,
                    messages,
                    tc,
                    "plan_write_already_submitted",
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                match f {
                    PolicyFeedback::Denied(key) => round_denials.push(key),
                    PolicyFeedback::Succeeded => round_had_success = true,
                }
                assistant_parts.push(format!("[{}] {}", tc.name, r.output));
                continue;
            }
            if plan_gate.is_some()
                && tc.name != crate::planning::PLAN_WRITE_TOOL
                && tc.name != crate::planning::BLACKBOARD_READ_TOOL
            {
                let (r, f) = plan_round_denied(
                    writer,
                    messages,
                    tc,
                    "plan_round_tool_denied",
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                match f {
                    PolicyFeedback::Denied(key) => round_denials.push(key),
                    PolicyFeedback::Succeeded => round_had_success = true,
                }
                assistant_parts.push(format!("[{}] {}", tc.name, r.output));
                continue;
            }
            // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §4):
            // console 默认态调用面门禁——声明面已收敛（投影无执行工具），
            // 调用面 belt-and-braces 拒绝隐藏工具（防幻觉直接调用执行/
            // 变更/shell/子代理/检索面）。direct 模式不适用（恢复工作工具）。
            if controller.console_default_enabled()
                && controller.console_mode() == crate::console_mode::ConsoleMode::Console
                && profile.role == AgentRole::Main
                && !AgentLoopController::is_console_surface_tool(&tc.name)
            {
                let (r, f) = plan_round_denied(
                    writer,
                    messages,
                    tc,
                    "console_mode_tool_denied",
                    svc.policy_revision
                        .load(std::sync::atomic::Ordering::SeqCst),
                )
                .await?;
                match f {
                    PolicyFeedback::Denied(key) => round_denials.push(key),
                    PolicyFeedback::Succeeded => round_had_success = true,
                }
                assistant_parts.push(format!("[{}] {}", tc.name, r.output));
                continue;
            }
            let target = route(&tc.name);
            // C2-1 (2026-08-11, ADR-0006 web-search slice): lane
            // self-execution — inside a retrieval lane, web tools (routed
            // ExternalRetrieval by relay) execute through the host instead
            // of dispatching a nested retrieval: the retrieval lane IS the
            // web lane (ADR-0010 §3.7.8 — the delegated search must
            // actually run). The nested-dispatch refusal keeps its
            // anti-recursion meaning for `retrieve_project_*`. The host
            // path preserves the write gate, the semaphore, the evidence
            // ledger and the mode gates; the permission bridge is skipped
            // — the explicit retrieval-mode gate (ADR-0010 §3.7.1) is the
            // authorization chain (2026-08-11 user adjudication; the main
            // lane's delegated path has no per-call gate either).
            let lane_self_execute = target == DispatchTarget::ExternalRetrieval
                && profile.tool_filter.denies_nested_dispatch();
            let effective = if lane_self_execute {
                DispatchTarget::Host
            } else {
                target.clone()
            };
            let mut round_feedback: Option<PolicyFeedback> = None;
            let result = match effective {
                DispatchTarget::InternalRetrieval | DispatchTarget::ExternalRetrieval => {
                    if profile.tool_filter.denies_nested_dispatch() {
                        // One seat per role — a retrieval lane never
                        // dispatches another retrieval (ADR-0010 §11.3).
                        let (r, f) = role_gate_denied(
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            "nested_subagent_dispatch_refused",
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        round_feedback = Some(f);
                        r
                    } else {
                        controller
                            .run_retrieval_subagent(
                                host,
                                writer,
                                target.clone(),
                                tc,
                                messages,
                                prompt,
                                &current_tool_defs,
                                orientation.as_deref_mut(),
                                cancel,
                                heartbeat,
                            )
                            .await?
                    }
                }
                DispatchTarget::ParentDisposition => {
                    // M4: the parent's structured disposition control tool.
                    // The subagent's projection strips it; the gate below is
                    // belt-and-braces for scripted lanes.
                    if profile.tool_filter.denies_nested_dispatch() {
                        let (r, f) = role_gate_denied(
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            "control_tool_lane_denied",
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        round_feedback = Some(f);
                        r
                    } else {
                        controller
                            .handle_parent_disposition(writer, tc, messages)
                            .await?
                    }
                }
                DispatchTarget::Host => {
                    if let Some(reason) = profile.tool_filter.write_gate(&tc.name) {
                        // ADR-0010 §3.2 deny-only write domain: refused
                        // BEFORE the host's permission bridge (a policy
                        // refusal, not a user choice — the permission
                        // dialog is never consulted).
                        let (r, f) = role_gate_denied(
                            writer,
                            messages,
                            tc,
                            profile.role.as_str(),
                            reason,
                            svc.policy_revision
                                .load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        round_feedback = Some(f);
                        r
                    } else {
                        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱):
                        // direct 模式直接动作——创建 console trace（trace_id
                        // 供事件盖章与 step_done 证据），盖章传入事件链。
                        let direct_ctx = if controller.console_default_enabled()
                            && profile.role == AgentRole::Main
                        {
                            controller.console_direct_begin(&tc.call_id)
                        } else {
                            None
                        };
                        let (result, feedback) = controller
                            .run_host_tool_with_plan_gate(
                                host,
                                writer,
                                tc,
                                prompt,
                                workspace_trust,
                                messages,
                                tool_rounds,
                                heartbeat,
                                // ACAF Slice 2 D-13 (2026-08-13): the lane's
                                // real activation — bound on web_fetch etc.
                                profile.activation_id.as_deref(),
                                // FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14):
                                // the dispatch's web_fetch candidate counter
                                // (None on main/grill — fails the gate closed).
                                profile.fetch_candidates.as_deref(),
                                // C2-1 (2026-08-11): lane self-execution skips
                                // the per-call permission bridge — the mode
                                // gate is the authorization chain (see the
                                // lane_self_execute comment above).
                                !lane_self_execute,
                                // P0-A step 5 review fix: only lanes that
                                // probe work tools (main/grill) write call
                                // failures back into the probe map.
                                profile.probe_work_tools,
                                // PLAN-FIRST 阶段 A (2026-08-16): the plan
                                // gate's current submission attempt — the
                                // plan_write handler decides refill vs
                                // degrade from it.
                                plan_gate.as_ref().map(|g| g.attempt),
                                direct_ctx.as_ref().map(|(stamp, _)| stamp),
                            )
                            .await?;
                        // direct 动作 trace 收口（commit + 证据面登记）。
                        if let Some((stamp, trace)) = direct_ctx {
                            let detail = if result.exit_code == Some(0) {
                                None
                            } else {
                                Some(result.output.as_str())
                            };
                            controller.console_direct_end(
                                &stamp,
                                trace,
                                &tc.name,
                                result.exit_code == Some(0),
                                detail,
                            );
                        }
                        // GAP-RETRIEVAL-TOOLS (2026-08-10): evidence
                        // collection for the retrieval lanes — the
                        // mechanical source of the structured result's
                        // ledger (§3.7.4). Main lane: `None`.
                        if let Some(evidence) = svc.evidence
                            && let Some(record) =
                                crate::controller::build_evidence_record(&tc.name, tc, &result)
                        {
                            evidence.lock().unwrap().push(record);
                        }
                        // M5 (2026-08-10): DC hard-signal consumption — the
                        // ONLY production point (§4.6.2; a journal replay
                        // never recounts). Main lane only.
                        if profile.dc_enabled {
                            maybe_consume_dc_signal(svc.dc_state, writer, tc, &result).await?;
                        }
                        round_feedback = feedback;
                        result
                    }
                }
            };
            // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 更新计划门
            // 状态——accepted/degraded 解除门，refill_requested 进入第二次
            // （最后一次）提交机会。
            if tc.name == crate::planning::PLAN_WRITE_TOOL {
                plan_write_called = true;
                if let Some(outcome) = result
                    .structured
                    .as_ref()
                    .and_then(|s| s.get("outcome"))
                    .and_then(serde_json::Value::as_str)
                {
                    match outcome {
                        "accepted" | "degraded" => plan_gate = None,
                        "refill_requested" => {
                            if let Some(g) = plan_gate.as_mut() {
                                g.attempt += 1;
                                g.rounds_without_submission = 0;
                            }
                        }
                        _ => {}
                    }
                }
            }
            match round_feedback {
                Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                // A successful call resets the breaker; None
                // (timeout / tool error) is neutral — it neither
                // resets nor counts (ADR-0010 §3.5.4 分开记账).
                Some(PolicyFeedback::Succeeded) => round_had_success = true,
                None => {}
            }
            assistant_parts.push(format!("[{}] {}", tc.name, result.output));
            // Count this result against the per-round injection budget (the
            // same `[tool] output` text the model receives).
            round_inject_tokens =
                round_inject_tokens.saturating_add(estimate_message_tokens(&Message {
                    role: Role::Tool,
                    content: format!("[{}] {}", tc.name, result.output),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                }));

            // GAP-INQUIRY-SPLIT (2026-08-09): the old per-tool-call
            // counter feeds are deleted — `tool_calls` / `tool_variety`
            // are not orientation 判定点 (§4.2) and subagent output
            // repetition belongs to the stagnation guard only. The main
            // lane's orientation round count happens at the model-round
            // completion point (one completed logical model round counts
            // 1 regardless of tool-call count).
        }
        if !assistant_parts.is_empty() {
            messages.push(Message {
                role: Role::Assistant,
                content: assistant_parts.join("\n"),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
        }
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 一轮工具轮未写
        // plan_write（仅 blackboard_read 等）→ 无提交计数；达到上限机械
        // 降级放行（plan_not_submitted 留痕，不挂死）。
        if let Some(gate) = plan_gate.as_mut()
            && !plan_write_called
        {
            gate.rounds_without_submission += 1;
            if gate.rounds_without_submission >= crate::planning::MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION
            {
                let g = plan_gate.take().expect("checked above");
                writer
                    .record(
                        EventType::PlanWrite,
                        crate::planning::plan_write_payload(
                            "",
                            "",
                            0,
                            "degraded",
                            g.attempt,
                            &crate::planning::PlanVerdict::default(),
                            Some("plan_not_submitted"),
                        ),
                    )
                    .await?;
            }
        }
        // Post-tool-batch injections — AFTER every tool reply of this
        // round, so no user message breaks the assistant-declaration →
        // tool-replies sequence (provider protocol; 2026-08-07 review
        // P1/P2). Semantics are unchanged: the neutral inquiry fires at
        // most once per round (counters reset on trigger), so hoisting
        // it out of the per-tool loop is equivalent.
        // ADR-0010 §3.5.4 round-level denial aggregation — extracted to
        // [`aggregate_denial_round`] (2026-08-12, behaviour unchanged): a
        // round with any success or with denials that do NOT all share one
        // normalized key resets the count; otherwise the round counts, and
        // at 3 consecutive same-key rounds the breaker message fires once
        // (count restarts).
        {
            let mut denial = svc.denial_state.lock().unwrap();
            if let Some(tool_name) =
                aggregate_denial_round(&mut denial, &round_denials, round_had_success)
            {
                pending_policy.push(Message {
                    role: Role::User,
                    content: crate::prompt::tool_policy_breaker_block(
                        &tool_name,
                        DENIAL_BREAKER_CONSECUTIVE,
                    ),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                });
            }
        }
        for pm in pending_policy {
            messages.push(pm);
        }
        // 2026-08-08 blackboard partition (A2): incremental push — the
        // edits this round actually made, replayed as ONE compact user
        // message right after the tool batch (mechanical, deterministic;
        // the model sees "本轮发生了什么" — the delta, never the whole
        // blackboard). Entries before this round were pushed on their
        // own rounds and are already in history; the blackboard keeps
        // the full list for blackboard_read look-backs.
        let round_edits = svc.blackboard.read().edits.clone();
        // `round_edit_count` was the section length BEFORE this round's
        // tools — it IS the start index of this round's records.
        let edits_start = round_edit_count.min(round_edits.len());
        let round_summary: Vec<String> = round_edits[edits_start..]
            .iter()
            .map(format_edit_record)
            .collect();
        if !round_summary.is_empty() {
            messages.push(Message {
                role: Role::User,
                content: format!("[本轮编辑] {}", round_summary.join("；")),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
        }
        // GAP-INQUIRY-SPLIT (2026-08-09) — MAIN orientation injection
        // point: the post-tool-batch gap (a safe action gap: the tool
        // results are in, the next generate has not started). The fired
        // block rides into the next generate — which, on the main lane, is
        // now a FORCED TEMPLATE round (no tools; the model answers the JSON
        // template before actions resume). When this is the budget-exhausting
        // round, the checkpoint round runs first and the post-budget final
        // round reports the partial result after it (§14.16).
        if pending_checkpoint.is_none()
            && let Some(role) = profile.orientation_role
            && let Some(record) = controller
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    "post_tool_batch_gap",
                    profile.role == AgentRole::Main,
                )
                .await?
        {
            pending_checkpoint = Some(PendingCheckpoint::Orientation { record, attempt: 1 });
        }
        // M5: DC checkpoint fire (same gap semantics as orientation; one
        // pending checkpoint at a time — orientation wins a same-gap tie).
        if pending_checkpoint.is_none()
            && profile.dc_enabled
            && let Some(payload) = maybe_fire_dc(svc.dc_state, writer, messages).await?
        {
            pending_checkpoint = Some(PendingCheckpoint::DiagnosticCoverage {
                payload,
                attempt: 1,
            });
        }
        // P0-C orz 内嵌集成 S2 (2026-08-15): 轮末机械发放——动作栏有未消费
        // 订单时在 post-tool-batch 安全间隙发放（副作用只发生在单一出口；
        // 模型面只有读板块 + 写订单）。pending checkpoint 优先级：本间隙
        // 已有 checkpoint 待轮时跳过发放，订单留在槽中；下一工具轮发放时
        // round 不匹配会按 `order_stale` 显式拒绝（防重放/过期，fail-closed）。
        let console_consumed = if pending_checkpoint.is_none() && profile.role == AgentRole::Main {
            controller
                .issue_pending_console_order(
                    host,
                    writer,
                    prompt,
                    workspace_trust,
                    tool_rounds,
                    heartbeat,
                )
                .await?
        } else {
            0
        };
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
        // 双模式显式询问轮触发——发放后若 console 态连续故障 ≥ 阈值且本
        // run 未问过，设置询问 checkpoint（无工具轮；优先级低于
        // orientation/DC——上方 fire 先占位）。询问回答（switch/stay）在
        // 下一轮 checkpoint 分支处理。
        if pending_checkpoint.is_none()
            && profile.role == AgentRole::Main
            && controller.console_inquiry_due()
        {
            let (streak, order_ids) = controller.console_streak_snapshot();
            pending_checkpoint = Some(PendingCheckpoint::ConsoleModeInquiry {
                attempt: 1,
                streak,
                order_ids,
            });
            // 询问轮为无工具轮——注入模板块（模型只回答 decision/reason）。
            messages.push(Message {
                role: Role::User,
                content: crate::console_mode::inquiry_template_block(),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
        }

        // P0-C S4 (2026-08-16): 发放的 console 动作按实际执行单位计入同一
        // tool-round 预算（直接订单 1、脚本每步 1）——下一轮 remaining 块
        // 机械反映；耗尽后同样进入最后无工具轮并结束。
        tool_rounds = tool_rounds.saturating_add(console_consumed);
        if !plan_round_active {
            tool_rounds += 1;
        }
        // D-8: mechanically re-declare the remaining budget after each
        // tool round — the model does not guess or drift (the previous
        // round's `[TOOL_ROUND_BUDGET]` text is already in history).
        messages.push(Message {
            role: Role::User,
            content: crate::prompt::tool_round_budget_remaining_block(
                profile.max_tool_rounds.saturating_sub(tool_rounds),
            ),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        if tool_rounds >= profile.max_tool_rounds {
            // Anti-runaway backstop — mark the truncation so the journal
            // records why pending tool calls were dropped. D-8: the cap
            // is a backstop, not a target — the run gets ONE final
            // no-tools round to report its partial result, then ends.
            writer
                .record(
                    EventType::GateDecision,
                    serde_json::json!({
                        "gate": "tool_rounds_limit",
                        "decision": "stop",
                        "reason": "max_tool_rounds_reached",
                        "tool_rounds": tool_rounds,
                        "max_tool_rounds": profile.max_tool_rounds,
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::User,
                content: crate::prompt::tool_round_budget_exhaustion_block(profile.max_tool_rounds),
                tool_call_id: None,
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            budget_exhausted = true;
        }
    }

    Ok(LoopOutcome {
        last_text,
        tool_rounds,
        rounds_since_compact,
        fold_state,
        budget_exhausted,
    })
}

/// ADR-0010 §3.5.4 round-level denial aggregation (extracted from the
/// post-tool-batch injection point, 2026-08-12 — behaviour unchanged): a
/// round with any success, or whose denials do NOT all share one normalized
/// key, resets the count; otherwise the round counts against `last_key`,
/// and at `DENIAL_BREAKER_CONSECUTIVE` consecutive same-key rounds returns
/// the offending tool name (the caller injects the strategy-switch message
/// once) with the count restarted. The key includes `policy_revision` — a
/// policy bump is a key change, so the reset path is structurally
/// reachable (GAP-DENIAL-POLICY-REVISION, 2026-08-12).
pub(crate) fn aggregate_denial_round(
    denial: &mut DenialState,
    round_denials: &[DenialKey],
    round_had_success: bool,
) -> Option<String> {
    let all_same_key = round_denials
        .first()
        .is_some_and(|k0| round_denials.iter().all(|k| k == k0));
    if round_had_success || !all_same_key {
        denial.consecutive_rounds = 0;
        denial.last_key = None;
        None
    } else if let Some(key) = round_denials.first() {
        if denial.last_key.as_ref() == Some(key) {
            denial.consecutive_rounds += 1;
        } else {
            denial.consecutive_rounds = 1;
            denial.last_key = Some(key.clone());
        }
        if denial.consecutive_rounds >= DENIAL_BREAKER_CONSECUTIVE {
            denial.consecutive_rounds = 0; // injected once per burst
            Some(key.tool_name.clone())
        } else {
            None
        }
    } else {
        None
    }
}

/// Structured refusal for a role-gated tool (ADR-0010 §3.2 deny-only write
/// domain / §11.3 nested dispatch guard): the call is journaled as
/// ToolStarted → ToolCompleted(status=error) — a refused call is visible in
/// the audit chain — and replayed as a Tool message (provider protocol:
/// every declared call is answered; 2026-08-06 polyglot probe). The denial
/// key feeds the shared 3-round breaker via the round-level aggregation.
async fn role_gate_denied(
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    target: &str,
    reason: &str,
    policy_revision: u64,
) -> Result<(ToolResult, PolicyFeedback), AgentLoopError> {
    writer
        .record(
            EventType::ToolStarted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "target": target,
            }),
        )
        .await?;
    let output = format!(
        "tool '{}' denied — {}; this retrieval lane refuses this tool \
         (write-domain deny-only gate, GAP-SUBAGENT-RUNTIME).",
        tc.name, reason,
    );
    writer
        .record(
            EventType::ToolCompleted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "target": target,
                "exit_code": 1,
                "status": "error",
                "error": reason,
            }),
        )
        .await?;
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: reason.to_string(),
            // GAP-DENIAL-POLICY-REVISION (2026-08-12): live value — a bump is
            // a key change, resetting the breaker (ADR-0010 §3.5.4).
            policy_revision,
        }),
    ))
}

/// PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): 计划轮的机械拒绝 —
/// 首轮只允许 `blackboard_read` + `plan_write`；任何其他工具调用（即使被
/// 声明）都在派发前拒绝并留痕（ToolStarted → ToolCompleted(status=error)），
/// 与角色门的审计形状一致。
async fn plan_round_denied(
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    reason: &'static str,
    policy_revision: u64,
) -> Result<(ToolResult, PolicyFeedback), AgentLoopError> {
    writer
        .record(
            EventType::ToolStarted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
            }),
        )
        .await?;
    let output = match reason {
        "plan_round_tool_denied" => format!(
            "tool '{}' denied — 首轮计划轮只允许 blackboard_read 与 plan_write；\
             执行/变更/检索/子代理等工具在计划落板前不可用（PLAN-FIRST 阶段 A）。",
            tc.name,
        ),
        "plan_write_already_submitted" => {
            "plan_write denied — 本轮已提交过 plan_write；请等待校验反馈\
             （若需重填，反馈将在下一轮注入）。"
                .to_string()
        }
        _ => format!("tool '{}' denied — {reason}", tc.name),
    };
    writer
        .record(
            EventType::ToolCompleted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "status": "error",
                "error": reason,
            }),
        )
        .await?;
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: reason.to_string(),
            policy_revision,
        }),
    ))
}

/// ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.6): per-round tool-result
/// injection budget refusal — the batch is at/over
/// `ORZ_MAX_INJECT_TOKENS_PER_ROUND` (default 50K), so this call is refused
/// WITHOUT ToolStarted (nothing executed). The journal carries the used
/// budget and the model receives an explicit offset/grep-first hint. The
/// refusal feeds the consecutive-denial breaker (same normalized key →
/// after 3 rounds the strategy-switch message fires, ADR-0010 §3.5.4).
async fn refuse_inject_budget(
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
    tc: &ToolCall,
    used: u64,
    budget: u64,
    policy_revision: u64,
) -> Result<(ToolResult, Option<PolicyFeedback>), AgentLoopError> {
    let code = "round_inject_budget_exceeded";
    let output = format!(
        "tool '{}' — 本轮工具结果注入预算已满（已用 {} 估计 tokens / 上限 {}）；\
         请改用 grep/结构提取优先，或对 read_file 使用 offset 分段续读。",
        tc.name, used, budget,
    );
    writer
        .record(
            EventType::ToolCompleted,
            serde_json::json!({
                "tool": tc.name,
                "call_id": tc.call_id,
                "exit_code": 1,
                "status": "error",
                "error": code,
                "inject_tokens_used": used,
                "inject_tokens_budget": budget,
            }),
        )
        .await?;
    messages.push(Message {
        role: Role::Tool,
        content: output.clone(),
        tool_call_id: Some(tc.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    Ok((
        ToolResult {
            output,
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        },
        Some(PolicyFeedback::Denied(DenialKey {
            tool_name: tc.name.clone(),
            reason_code: code.to_string(),
            policy_revision,
        })),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The retrieval-lane write-domain gate classifies the three refusal
    /// classes distinctly (ADR-0010 §3.2 deny-only domain; review F6,
    /// 2026-08-10 — `run_tests` is §3.8.2 controlled code execution, not a
    /// file write, and gets its own reason).
    #[test]
    fn retrieval_write_gate_classifies_denial_classes() {
        // File mutations → write-domain denial.
        assert_eq!(
            ToolFilter::Retrieval.write_gate("search_replace"),
            Some("retrieval_role_write_denied")
        );
        // Controlled code execution → execution denial (the test process
        // may write files / touch the network; a retrieval task contract
        // never carries a test harness).
        assert_eq!(
            ToolFilter::Retrieval.write_gate("run_tests"),
            Some("retrieval_role_execution_denied")
        );
        // Shell escape → shell denial.
        assert_eq!(
            ToolFilter::Retrieval.write_gate("bash"),
            Some("retrieval_role_shell_denied")
        );
        // Reads pass the gate — the same registry stays visible, refusal
        // is scope-level at call time (§3.5.2).
        assert_eq!(ToolFilter::Retrieval.write_gate("read_file"), None);
        assert_eq!(ToolFilter::Retrieval.write_gate("web_search"), None);
        // The main lane's gate refuses nothing.
        assert_eq!(ToolFilter::None.write_gate("search_replace"), None);
        assert_eq!(ToolFilter::None.write_gate("bash"), None);
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15): the request-header fingerprint
    /// is stable for identical inputs and changes when ANY header component
    /// (system / tools / config) changes.
    #[test]
    fn request_header_fingerprint_is_stable_and_component_sensitive() {
        let tools = vec![
            ToolDef {
                name: "read_file".to_string(),
                description: "reads a file".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
            ToolDef {
                name: "grep".to_string(),
                description: "searches text".to_string(),
                parameters: serde_json::json!({"type": "object"}),
            },
        ];
        let a = compute_request_header("system v1", &tools, "config-v1");
        let b = compute_request_header("system v1", &tools, "config-v1");
        assert_eq!(a, b);
        assert_eq!(a.header_sha256.len(), 64);
        assert_eq!(a.tools, vec!["read_file", "grep"]);

        // Tool order must not change the digest (the canonical rows are
        // sorted by name before hashing).
        let swapped = vec![tools[1].clone(), tools[0].clone()];
        let c = compute_request_header("system v1", &swapped, "config-v1");
        assert_eq!(a.tools_sha256, c.tools_sha256);
        assert_eq!(a.header_sha256, c.header_sha256);

        // Each component change breaks the header digest.
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v2", &tools, "config-v1").header_sha256
        );
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v1", &tools[..1], "config-v1").header_sha256
        );
        assert_ne!(
            a.header_sha256,
            compute_request_header("system v1", &tools, "config-v2").header_sha256
        );
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15): the payload carries the reason,
    /// the three component digests, the tool list and the change-kind
    /// attribution; `change` includes the previous header digest.
    #[test]
    fn request_header_payload_shapes_initial_and_change() {
        let header = compute_request_header("system", &[], "config");
        let initial = request_header_payload(&header, "initial", None, "main", None);
        assert_eq!(initial["reason"], "initial");
        assert_eq!(initial["header_sha256"], header.header_sha256);
        assert_eq!(initial["tool_count"], 0);
        assert_eq!(initial["agent_role"], "main");
        assert!(initial.get("previous_header_sha256").is_none());
        assert!(initial.get("change_kind").is_none());

        let changed =
            request_header_payload(&header, "change", Some("prev"), "main", Some("system"));
        assert_eq!(changed["reason"], "change");
        assert_eq!(changed["previous_header_sha256"], "prev");
        assert_eq!(changed["change_kind"], "system");
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15 review fix): the change-kind
    /// attribution is exact — one component change names that component,
    /// two or three name `multiple`.
    #[test]
    fn header_change_kind_attributes_component_changes() {
        let tool = vec![ToolDef {
            name: "read_file".to_string(),
            description: "reads a file".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }];
        let base = compute_request_header("system v1", &[], "config-v1");
        let system = compute_request_header("system v2", &[], "config-v1");
        let tools = compute_request_header("system v1", &tool, "config-v1");
        let config = compute_request_header("system v1", &[], "config-v2");
        let all = compute_request_header("system v2", &tool, "config-v2");
        assert_eq!(header_change_kind(&base, &system), "system");
        assert_eq!(header_change_kind(&base, &tools), "tools");
        assert_eq!(header_change_kind(&base, &config), "config");
        assert_eq!(header_change_kind(&base, &all), "multiple");
    }

    fn denial_key(tool: &str, reason: &str, policy_revision: u64) -> DenialKey {
        DenialKey {
            tool_name: tool.to_string(),
            reason_code: reason.to_string(),
            policy_revision,
        }
    }

    /// Same normalized key across three consecutive rounds fires the breaker
    /// exactly once (count restarts — injected once per burst).
    #[test]
    fn aggregate_denial_round_same_key_fires_at_three() {
        let mut denial = DenialState::default();
        let key = denial_key("read_file", "permission_denied", 0);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 2);
        // Third round → fires, count restarts (burst-once semantics).
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            Some("read_file".to_string())
        );
        assert_eq!(denial.consecutive_rounds, 0);
    }

    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): a policy revision change is
    /// a DenialKey change — the consecutive count resets, so a
    /// `0,0,1`-sequence never fires (ADR-0010 §3.5.4 reset path now
    /// structurally reachable).
    #[test]
    fn aggregate_denial_round_policy_revision_change_resets() {
        let mut denial = DenialState::default();
        let rev0 = denial_key("read_file", "permission_denied", 0);
        let rev1 = denial_key("read_file", "permission_denied", 1);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev0), false),
            None
        );
        // Two rounds at revision 0…
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev0), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 2);
        // …then the policy bumps: the third round's key differs → reset.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&rev1), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        // 0,0,1 then 1,1 — a fresh 3-round run at the new revision would
        // still fire; the bump itself never does.
        assert_eq!(denial.last_key, Some(rev1));
    }

    /// A round with any success resets the consecutive count.
    #[test]
    fn aggregate_denial_round_success_resets() {
        let mut denial = DenialState::default();
        let key = denial_key("read_file", "permission_denied", 0);
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), false),
            None
        );
        assert_eq!(denial.consecutive_rounds, 1);
        // A mixed round (denial + success) is a key change → reset.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), true),
            None
        );
        assert_eq!(denial.consecutive_rounds, 0);
        // A pure-success round also resets.
        assert_eq!(
            aggregate_denial_round(&mut denial, std::slice::from_ref(&key), true),
            None
        );
        assert_eq!(denial.consecutive_rounds, 0);
        // Empty round (no denials) resets too (conservative baseline).
        assert_eq!(aggregate_denial_round(&mut denial, &[], false), None);
        assert_eq!(denial.consecutive_rounds, 0);
    }

    // ---- FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27) ----

    use crate::action_ledger::LedgerFoldState;
    use crate::host::ToolRegistry;
    use orz_assurance::journal::JournalRecorder;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, AtomicU64};

    struct CompactTestHost {
        journal: JournalRecorder,
    }

    struct CompactEmptyRegistry;

    impl ToolRegistry for CompactEmptyRegistry {
        fn get(&self, _name: &str) -> Option<ToolDef> {
            None
        }
        fn list(&self) -> Vec<ToolDef> {
            Vec::new()
        }
    }

    #[async_trait::async_trait]
    impl LoopHost for CompactTestHost {
        fn journal(&self) -> &JournalRecorder {
            &self.journal
        }
        fn tools_registry(&self) -> &dyn ToolRegistry {
            &CompactEmptyRegistry
        }
        // Summary archives land under session_cwd/.gsa/compaction — point
        // the mock at the per-test journal dir (same pattern as the
        // controller test hosts).
        fn session_cwd(&self) -> PathBuf {
            self.journal.journal_dir().to_path_buf()
        }
    }

    struct CompactSummaryAgent {
        response: ModelResponse,
    }

    #[async_trait::async_trait]
    impl RoundAgent for CompactSummaryAgent {
        async fn run_round(
            &self,
            _system: &str,
            _messages: Vec<Message>,
            _tools: Vec<ToolDef>,
            _max_tokens: u32,
            _cancel: Option<&tokio_util::sync::CancellationToken>,
            _heartbeat: Option<&ActivityClock>,
            _on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
        ) -> Result<ModelResponse, GatewayError> {
            Ok(self.response.clone())
        }
    }

    fn compact_test_svc<'a>(
        cfg: &'a ContextCompactConfig,
        blackboard: &'a Arc<SharedBlackboard>,
        denial_state: &'a Mutex<DenialState>,
        pacing: &'a AtomicU32,
        dc: &'a Mutex<DebugEpisodeState>,
        policy: &'a AtomicU64,
    ) -> SharedLoopServices<'a> {
        SharedLoopServices {
            blackboard,
            denial_state,
            pacing_rounds: pacing,
            context_compact: cfg,
            dc_state: dc,
            evidence: None,
            policy_revision: policy,
            max_inject_tokens_per_round: 50_000,
            blackboard_archive_dir: None,
        }
    }

    /// 取证形态：0=user, 1=marker, 2=assistant(c1), 3=tool(c1),
    /// 4=assistant(c2), 5=tool(c2)——折叠冻结态 fold_start=2/fold_cut=4
    /// （与 make-doom-for-mips 复验 400 的 marker 在索引 1 场景同构）。
    fn compact_test_messages() -> Vec<Message> {
        let mut messages = vec![Message {
            role: Role::User,
            content: "任务".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        }];
        messages.push(Message {
            role: Role::User,
            content: format!("{} v0.2] 摘要", crate::prompt::CONTEXT_COMPRESSED_PREFIX),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        for (id, target, result) in [("c1", "a.py", "A"), ("c2", "b.rs", "B")] {
            messages.push(Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    name: "read_file".to_string(),
                    arguments: serde_json::json!({"path": target}),
                    call_id: id.to_string(),
                }],
                reasoning_content: None,
            });
            messages.push(Message {
                role: Role::Tool,
                content: result.to_string(),
                tool_call_id: Some(id.to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
        }
        messages
    }

    fn compact_folded_state() -> LedgerFoldState {
        LedgerFoldState {
            fold_start: Some(2),
            fold_cut: Some(4),
            folded_ledger: Some("ledger".to_string()),
        }
    }

    fn compact_dummy_response() -> ModelResponse {
        ModelResponse {
            text: None,
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            reasoning_content: None,
            reasoning_tokens: None,
            completion_tokens: None,
            cache_hit_tokens: None,
            cache_miss_tokens: None,
            prompt_tokens: None,
        }
    }

    /// 压缩触发但缩减守卫不满足 → `GuardBlocked`：messages 与折叠三态均
    /// 不变（旧代码在此路径删除了 marker、折叠索引未失效 → 后续视图
    /// preamble 裸露声明 → provider 400）。
    #[tokio::test]
    async fn guard_blocked_leaves_messages_and_fold_untouched() {
        let dir = std::env::temp_dir().join(format!("orz-compact-guard-{}", std::process::id()));
        let host = CompactTestHost {
            journal: JournalRecorder::new(dir),
        };
        let cfg = ContextCompactConfig::default();
        let blackboard = Arc::new(SharedBlackboard::new());
        let denial_state = Mutex::new(DenialState::default());
        let pacing = AtomicU32::new(0);
        let dc = Mutex::new(DebugEpisodeState::default());
        let policy = AtomicU64::new(0);
        let svc = compact_test_svc(&cfg, &blackboard, &denial_state, &pacing, &dc, &policy);
        let mut writer = crate::controller::discard_event_writer("test-run");
        let agent = CompactSummaryAgent {
            response: compact_dummy_response(),
        };
        let mut messages = compact_test_messages();
        let mut fold = compact_folded_state();
        let before_messages = messages.clone();
        let before_fold = fold.clone();
        let decision = run_template_compact(
            &svc,
            &mut writer,
            &host,
            &agent,
            &mut messages,
            0, // measured=0 → removable=0 < min_compactable → 守卫必不满足
            "rhythm",
            false,
            false,
            0,
            2,
            None,
            &mut fold,
            None,
            None,
        )
        .await
        .expect("guard path returns a decision");
        assert_eq!(decision, CompactDecision::GuardBlocked);
        assert_eq!(
            messages, before_messages,
            "guard path must not mutate messages (marker stays)"
        );
        assert_eq!(
            fold, before_fold,
            "guard path must not touch the fold state"
        );
    }

    /// 执行路径：确认执行后删除旧 marker 并重算 kept_start（折叠态
    /// `fold_cut - had_marker`）——drain 数量与事件口径一致，折叠状态
    /// 在 drain 后重置。
    #[tokio::test]
    async fn compaction_execution_recomputes_kept_start_after_marker_removal() {
        let dir = std::env::temp_dir().join(format!("orz-compact-exec-{}", std::process::id()));
        let host = CompactTestHost {
            journal: JournalRecorder::new(dir.clone()),
        };
        let cfg = ContextCompactConfig::default();
        let blackboard = Arc::new(SharedBlackboard::new());
        let denial_state = Mutex::new(DenialState::default());
        let pacing = AtomicU32::new(0);
        let dc = Mutex::new(DebugEpisodeState::default());
        let policy = AtomicU64::new(0);
        let svc = compact_test_svc(&cfg, &blackboard, &denial_state, &pacing, &dc, &policy);
        let mut writer = crate::controller::discard_event_writer("test-run");
        // 可解析的五段模板输出：两个模型生成槽（≥300 有效字符）。
        let notes = format!("[注意事项]\n{}\n[/注意事项]", "n".repeat(160));
        let continuation = format!("[后续衔接]\n{}\n[/后续衔接]", "c".repeat(160));
        let agent = CompactSummaryAgent {
            response: ModelResponse::text_response(format!("{notes}\n{continuation}")),
        };
        let mut messages = compact_test_messages();
        let mut fold = compact_folded_state();
        let decision = run_template_compact(
            &svc,
            &mut writer,
            &host,
            &agent,
            &mut messages,
            100_000, // force=true 跳过缩减守卫，measured 仅用于事件口径
            "rhythm",
            true,
            false,
            0,
            2,
            None,
            &mut fold,
            None,
            None,
        )
        .await
        .expect("execution path returns a decision");
        assert_eq!(decision, CompactDecision::Executed { incomplete: false });
        // marker 删除使 fold_cut 4→3；drain [first_round_start=1..3) 丢弃
        // c1 轮（声明+回复 2 条）；新 marker 插入索引 1 → U0+marker+c2 轮。
        assert_eq!(messages.len(), 4, "U0 + marker + c2 轮");
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::User);
        assert!(
            messages[1]
                .content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX),
            "new rolling marker inserted at index 1"
        );
        assert_eq!(messages[2].tool_calls.len(), 1, "c2 声明保留");
        assert_eq!(messages[2].tool_calls[0].call_id, "c2");
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("c2"));
        assert!(!fold.is_folded(), "执行后折叠状态重置");
        let archive = dir
            .join(".gsa")
            .join("compaction")
            .join("compaction-test-run-0000.md");
        assert!(
            archive.exists(),
            "summary archive written under session_cwd"
        );
    }
}
