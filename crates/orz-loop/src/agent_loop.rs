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

use std::sync::{Arc, Mutex};

use orz_assurance::{EventType, GateDecision};

use crate::agents::SubagentRole;
use crate::blackboard::SharedBlackboard;
use crate::controller::{
    AgentLoopController, AgentLoopError, ContextCompactConfig, DenialKey, DenialState, EventWriter,
    PolicyFeedback, TEXT_DELTA_PACING, compact_messages, format_edit_record, DENIAL_BREAKER_CONSECUTIVE,
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
    Retrieval { role: SubagentRole, goal: String },
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
    /// ACAF Slice 2 fail-closed D-13 (2026-08-13): the retrieval lane's
    /// real activation_id — bound on the lane's action tickets (web_fetch /
    /// browser_read). The main lane is `None` (main-lane actions stay
    /// activation-less).
    pub activation_id: Option<String>,
}

impl LoopProfile {
    pub(crate) fn main(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: true,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            dc_enabled: true,
            max_tool_rounds,
            initial_tool_rounds: 0,
            activation_id: None,
        }
    }

    /// Grill-mode turn (2026-08-08): full loop, no final-answer gate
    /// (the grill answer is not a run-semantic; same reasoning as the
    /// counterexample skip comment in `run_turn_inner`).
    pub(crate) fn grill(max_tool_rounds: u32) -> Self {
        Self {
            role: AgentRole::Main,
            counterexample_gate: false,
            orientation_role: Some(AgentRole::Main),
            system_kind: SystemPromptKind::Main,
            tool_filter: ToolFilter::None,
            dc_enabled: false,
            max_tool_rounds,
            initial_tool_rounds: 0,
            activation_id: None,
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
        max_tool_rounds: u32,
        initial_tool_rounds: u32,
        activation_id: &str,
    ) -> Self {
        let agent_role = match role {
            SubagentRole::InternalRetrieval => AgentRole::InternalRetrieval,
            SubagentRole::ExternalRetrieval => AgentRole::ExternalRetrieval,
        };
        Self {
            role: agent_role,
            counterexample_gate: false,
            orientation_role: Some(agent_role),
            system_kind: SystemPromptKind::Retrieval {
                role,
                goal: goal.to_string(),
            },
            tool_filter: ToolFilter::Retrieval,
            dc_enabled: false,
            max_tool_rounds,
            initial_tool_rounds,
            activation_id: Some(activation_id.to_string()),
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
}

/// What the loop produced — the caller maps it to its own terminal
/// semantics (main: run_finished/run_invalidated; subagent: result
/// formation / budget_exhausted / subagent_failed).
pub(crate) struct LoopOutcome {
    pub last_text: Option<String>,
    pub tool_rounds: u32,
    /// Read by the subagent terminal mapping (M3) — the main caller's
    /// terminal event carries `tool_rounds` only.
    #[allow(dead_code)] // consumed by the subagent path (GAP-SUBAGENT-RUNTIME M3)
    pub budget_exhausted: bool,
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
/// `tool_defs` come from the caller's ONE registry-catalog snapshot — the
/// subagent loop does not re-probe (no second `tool_availability_check`
/// event; §3.5.1 per-session catalog — 2026-08-12 语义：目录快照，零可用
/// 性承诺，判定在调用时)。`messages` is in/out: the conversation continues
/// across rounds; the caller owns the seed and the post-loop use (stagnation
/// / grill history writeback).
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
    // A6 (2026-08-08, §8 C.1): explicit context compaction state — the
    // previous round's MEASURED prompt tokens (provider usage; None
    // until the first round reports usage), the rounds since the last
    // compaction (per-turn — the conversation is per-turn too), and
    // whether the previous round was a TOOL round (declaration +
    // execution + pushes) — the rhythm compaction fires only in the
    // gap after the model's LAST tool round (candidate answer round:
    // `!last_round_had_tools` while `counterexample_fired`).
    let mut last_prompt_tokens: Option<u64> = None;
    let mut rounds_since_compact: u32 = 0;
    let mut last_round_had_tools = false;

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

        // A6 (2026-08-08, §8 C.1 用户裁决): explicit context compaction
        // — triggered by the previous round's MEASURED prompt tokens
        // (provider usage prompt_tokens; `None` until the first round
        // reports usage). Two triggers:
        //   - SAFETY (window guard, review D1-1): measured >
        //     `safety_tokens` — fires at any inter-batch gap, ignoring
        //     rhythm conditions; the cost of an extra cache miss is
        //     trivially cheaper than a window-overflow run failure.
        //     Resets the round counter so a later rhythm compaction
        //     judges normally (user decision).
        //   - RHYTHM: measured > `trigger_tokens` with a ≥ min_rounds
        //     cooldown, AND the model just finished its LAST tool batch
        //     (the candidate-answer round — no tool calls — means the
        //     action sequence is complete and the final answer is next
        //     behind the counterexample gate). Mid-task gaps (after
        //     tool rounds) are NEVER compacted — the run's action flow
        //     stays smooth and stable; the final answer round then runs
        //     on a compacted context (~90K) with low look-back pressure.
        //     This gap exists exactly once per run (the gate fires
        //     once), so the rhythm compaction is at most once.
        // Compaction keeps the preamble (original prompt + whitelist)
        // and the newest rounds verbatim; older rounds are dropped
        // whole (declaration + tool replies + injected pushes stay
        // paired); the marker tells the model history was compressed
        // (explicit notice — the model has no metacognition to guess,
        // design §5 A6) and blackboard_read is the look-back window.
        let rhythm_gap = !last_round_had_tools && counterexample_fired;
        let compact_now = match last_prompt_tokens {
            Some(measured) => {
                measured > svc.context_compact.safety_tokens
                    || (rhythm_gap
                        && measured > svc.context_compact.trigger_tokens
                        && rounds_since_compact >= svc.context_compact.min_rounds)
            }
            None => false,
        };
        if compact_now
            && let Some(measured) = last_prompt_tokens
        {
            let stats = compact_messages(messages, svc.context_compact.target_tokens);
            if stats.rounds_dropped > 0 {
                let rounds_since = rounds_since_compact;
                rounds_since_compact = 0;
                messages.insert(
                    stats.marker_index,
                    Message {
                        role: Role::User,
                        content: crate::prompt::context_compressed_marker(
                            stats.rounds_dropped,
                            measured,
                            controller.blackboard_summary_line().as_deref(),
                        ),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    },
                );
                writer
                    .record(
                        EventType::ContextCompressed,
                        serde_json::json!({
                            "trigger_tokens": measured,
                            "target_tokens": svc.context_compact.target_tokens,
                            "rounds_since_last_compaction": rounds_since,
                            "rounds_dropped": stats.rounds_dropped,
                            "messages_dropped": stats.messages_dropped,
                            // The marker message was inserted above —
                            // the final conversation is +1.
                            "messages_kept": stats.messages_kept + 1,
                            "estimated_tokens_after": stats.estimated_tokens_after,
                        }),
                    )
                    .await?;
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
        if let Some(role) = profile.orientation_role {
            controller
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    "loop_top_gap",
                )
                .await?;
        }
        // M5 (2026-08-10): DC checkpoint fire at the same safe gap (the
        // stage threshold was met by signals consumed in a prior tool
        // round — the block is neutral, never a hard gate, §4.6.4).
        if profile.dc_enabled {
            maybe_fire_dc(svc.dc_state, writer, messages).await?;
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
        // A4 (2026-08-08): the resident status line — plan state only
        // (goal + steps + current), rendered from the blackboard plan
        // section. Absent when no plan is set; byte-identical across
        // rounds while the plan is unchanged (same cache discipline as
        // the budget block). Edit counts are deliberately NOT here:
        // per-round edit deltas arrive via `[本轮编辑]` and totals via
        // blackboard_read — a per-round counter in the system prompt
        // would recreate the 17.7%→98% cache regression (2026-08-07).
        let system = match &profile.system_kind {
            SystemPromptKind::Main => {
                let mut system_blocks = budget_block.clone();
                if profile.role == AgentRole::Main
                    && let Some(status_line) = controller.render_status_line()
                {
                    system_blocks.push_str(&format!("\n\n{status_line}"));
                }
                controller
                    .main_agent
                    .prompt_builder
                    .build_system_prompt(Some(&system_blocks))
            }
            SystemPromptKind::Retrieval { role, goal } => {
                // ADR-0010 §3.2 task contract — the subagent's own system
                // (citation rules + [DOC]/[SOURCE] delivery contract), with
                // the shared budget declaration.
                crate::prompt::build_retrieval_system_prompt(
                    role.section_name(),
                    goal,
                    &budget_block,
                )
            }
        };
        // D-6 (FIX_PLAN 2026-08-06): subagents get the full 160K budget
        // too (the main agent's request-level cap — §3.4.2 same defaults).
        // Review F3 (2026-08-10): ONE constant for both lanes — the three
        // agents must never carry their own literals.
        let max_tokens = match &profile.system_kind {
            SystemPromptKind::Main => controller.main_agent_max_tokens(),
            SystemPromptKind::Retrieval { .. } => REQUEST_MAX_TOKENS,
        };

        let mut partial_text: Vec<String> = Vec::new();
        let response = match agent
            .run_round(
                &system,
                messages.clone(),
                tool_defs.to_vec(),
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

        // A6: track the round — measured prompt tokens feed the next
        // loop-top trigger check; the round counter is the compaction
        // cooldown; the tool-round flag gates the rhythm compaction to
        // the gap after the LAST tool round (§8 C.1).
        last_prompt_tokens = response.prompt_tokens;
        rounds_since_compact += 1;
        last_round_had_tools = !response.tool_calls.is_empty();

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
        // 2026-08-08 blackboard partition (A2): snapshot the edit-action
        // length BEFORE this round's tools — the incremental push after
        // the batch reports exactly the records this round added.
        let round_edit_count = svc.blackboard.read().edits.len();
        for tc in &response.tool_calls {
            if cancel.is_some_and(|c| c.is_cancelled()) {
                return Err(AgentLoopError::Cancelled);
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
                            svc.policy_revision.load(std::sync::atomic::Ordering::SeqCst),
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
                                tool_defs,
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
                            svc.policy_revision.load(std::sync::atomic::Ordering::SeqCst),
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
                            svc.policy_revision.load(std::sync::atomic::Ordering::SeqCst),
                        )
                        .await?;
                        round_feedback = Some(f);
                        r
                    } else {
                        let (result, feedback) = controller
                            .run_host_tool(
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
                                // C2-1 (2026-08-11): lane self-execution skips
                                // the per-call permission bridge — the mode
                                // gate is the authorization chain (see the
                                // lane_self_execute comment above).
                                !lane_self_execute,
                            )
                            .await?;
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
                            maybe_consume_dc_signal(svc.dc_state, writer, tc, &result)
                                .await?;
                        }
                        round_feedback = feedback;
                        result
                    }
                }
            };
            match round_feedback {
                Some(PolicyFeedback::Denied(key)) => round_denials.push(key),
                // A successful call resets the breaker; None
                // (timeout / tool error) is neutral — it neither
                // resets nor counts (ADR-0010 §3.5.4 分开记账).
                Some(PolicyFeedback::Succeeded) => round_had_success = true,
                None => {}
            }
            assistant_parts.push(format!("[{}] {}", tc.name, result.output));

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
        // block rides into the SAME turn's next generate. When this is
        // the budget-exhausting round, the final (post-budget) round
        // answers both the orientation block and the exhaustion notice
        // (review D2-1, 2026-08-10: two competing directives on the
        // closing round — accepted; the run is ending anyway).
        if let Some(role) = profile.orientation_role {
            controller
                .maybe_fire_orientation(
                    writer,
                    messages,
                    orientation.as_deref_mut(),
                    role,
                    "post_tool_batch_gap",
                )
                .await?;
        }
        // M5: DC checkpoint fire (same gap semantics as orientation).
        if profile.dc_enabled {
            maybe_fire_dc(svc.dc_state, writer, messages).await?;
        }

        tool_rounds += 1;
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
                content: crate::prompt::tool_round_budget_exhaustion_block(
                    profile.max_tool_rounds,
                ),
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
}
