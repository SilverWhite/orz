//! Retrieval dispatch — the subagent run entry — batch B5 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use std::sync::{Arc, Mutex};

use orz_assurance::{EventType, sha256_hex};

use crate::agent_loop::{
    LoopOutcome, LoopProfile, SharedLoopServices, run_agent_loop, run_template_compact,
};
use crate::agents::{RetrievalSubagent, SubagentRole};
use crate::blackboard::ToolActionRecord;
use crate::controller::{
    AgentLoopController, AgentLoopError, EventWriter, RetrievalCapability, RetrievalMode,
    RetrievalResultChannel, chrono_utc_now, estimate_messages_tokens,
    retrieval_result_channel_from_env,
};
use crate::gateway::model::{Message, ModelGateway, Role, ToolCall};
use crate::host::{LoopHost, ToolDef, ToolResult};
use crate::orientation::OrientationSessionState;
use crate::relay::DispatchTarget;
use crate::retrieval::activation::{ActivationState, ActivationStatus};
use crate::retrieval::evidence::build_structured_result;

impl AgentLoopController {
    /// Run a retrieval subagent for a retrieval-shaped tool call.
    /// (2026-08-07 review F-03: the run's cancel token is threaded through —
    /// 8 args is the documented cost; a context struct would churn all
    /// call sites for no readability gain.)
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn run_retrieval_subagent(
        &self,
        host: &dyn LoopHost,
        writer: &mut EventWriter<'_>,
        // STALL-DEGENERATION-FAILFAST (2026-08-21, ADR-0010 §14.37 /
        // 设计 §2.2)：本 run 的 per-run 隔离 gateway——检索子代理与
        // 主 agent 共享同一 run 实例，退化计数/档位 run 内一致、跨
        // run 不泄漏。
        run_gateway: &Arc<dyn ModelGateway>,
        target: DispatchTarget,
        tc: &ToolCall,
        messages: &mut Vec<Message>,
        prompt: &str,
        tool_defs: &[ToolDef],
        orientation: Option<&mut OrientationSessionState>,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
    ) -> Result<ToolResult, AgentLoopError> {
        let (role, target_name) = match target {
            DispatchTarget::InternalRetrieval => {
                (SubagentRole::InternalRetrieval, "internal_retrieval")
            }
            DispatchTarget::ExternalRetrieval => {
                (SubagentRole::ExternalRetrieval, "external_retrieval")
            }
            DispatchTarget::ParentDisposition => {
                unreachable!("disposition calls go to handle_parent_disposition")
            }
            DispatchTarget::Host => unreachable!("host calls go to run_host_tool"),
        };
        // GAP-RETRIEVAL-TOOLS (2026-08-10): mode gate (ADR-0010 §3.7.1 —
        // explicit mode, never an implicit fallback).
        //
        // mode=off refuses WITHOUT a ToolStarted: the verifier's mode rule
        // forbids any retrieval dispatch (tool_started with a retrieval
        // target) after a transition to off — the refusal is journaled as
        // the terminal ToolCompleted(error) alone.
        if self.retrieval_mode == RetrievalMode::Off {
            let msg = format!(
                "retrieval '{target_name}' refused — retrieval mode is 'off' \
                 for this session (ADR-0010 §3.7.1); no retrieval tools are \
                 available. Submit retrieval_disposition close for any \
                 already-pending activation.",
            );
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "target": target_name,
                        "status": "error",
                        "error": "retrieval_mode_off",
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }
        // mode=local_browser with an unavailable capability fails EXPLICITLY
        // (browser automation is not implemented in this slice; the probe
        // recorded unsupported — no silent degradation to the framework
        // tools). The refusal follows the standard ToolStarted →
        // ToolCompleted(error) audit shape.
        if self.retrieval_mode == RetrievalMode::LocalBrowser {
            match &self.retrieval_capability {
                RetrievalCapability::Available => {}
                RetrievalCapability::Unsupported(reason)
                | RetrievalCapability::Degraded(reason) => {
                    writer
                        .record(
                            EventType::ToolStarted,
                            serde_json::json!({
                                "tool": tc.name,
                                "call_id": tc.call_id,
                                "target": target_name,
                            }),
                        )
                        .await?;
                    let msg = format!(
                        "retrieval '{target_name}' refused — local_browser \
                         capability is not available ({reason}); no silent \
                         fallback to framework retrieval tools (ADR-0010 \
                         §3.7.1)",
                    );
                    writer
                        .record(
                            EventType::ToolCompleted,
                            serde_json::json!({
                                "tool": tc.name,
                                "call_id": tc.call_id,
                                "target": target_name,
                                "status": "error",
                                "error": "retrieval_capability_unavailable",
                            }),
                        )
                        .await?;
                    messages.push(Message {
                        role: Role::Tool,
                        content: msg.clone(),
                        tool_call_id: Some(tc.call_id.clone()),
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                    return Ok(ToolResult {
                        output: msg,
                        exit_code: Some(1),
                        output_encoding: None,
                        structured: None,
                        ..Default::default()
                    });
                }
            }
        }
        writer
            .record(
                EventType::ToolStarted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "target": target_name,
                }),
            )
            .await?;

        // RETRIEVAL-SUBAGENT-WIRING 审查处理：任务契约 = query（必填，
        // 缺省回退 prompt）+ 可选 scope/max_results 机械并入（ToolDef
        // 声明面承诺的参数必须进契约，否则被静默丢弃）。
        let goal = Self::build_retrieval_task_goal(&tc.arguments, prompt);

        // Activation resolution (ADR-0010 §3.3): the state is REMOVED from
        // the registry so the std::Mutex guard never crosses an await; it is
        // re-inserted after the loop on every path (the conversation is
        // evidence — never deleted on reset, §4.4).
        //
        // Identity (D3-3 upgrade, GAP-SUBAGENT-RUNTIME 2026-08-10): the
        // session-scoped identities replace the temporary call-derived ones.
        let session_id = orientation
            .as_ref()
            .map(|o| o.session_id.clone())
            .unwrap_or_else(|| writer.run_id().to_string());
        let session8: String = session_id.chars().take(8).collect();
        // Refusal check FIRST (no guard held across the await — the std
        // Mutex guard is not Send; M4 makes this branch reachable).
        let awaiting_activation_id = self
            .activations
            .lock()
            .unwrap()
            .states
            .get(&role)
            .filter(|a| a.status == ActivationStatus::AwaitingDisposition)
            .map(|a| a.activation_id.clone());
        if let Some(act_id) = awaiting_activation_id {
            // ADR-0010 §4.4: an unresolved activation refuses new
            // retrieval — the parent must submit a structured disposition
            // first (never guessed from free text; the
            // `retrieval_disposition` control tool is the only path, M4).
            let msg = format!(
                "retrieval '{target_name}' refused — activation {act_id} is awaiting \
                 parent disposition; submit retrieval_disposition close or \
                 continue(requirement_delta) first",
            );
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "target": target_name,
                        "status": "error",
                        "error": "activation_awaiting_disposition",
                    }),
                )
                .await?;
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }
        let (mut act, task_goal) = {
            let mut reg = self.activations.lock().unwrap();
            match reg.states.get(&role) {
                Some(a) if a.status == ActivationStatus::Active => {
                    // Same activation, new task iteration. M4: a `continue`
                    // re-entry's requirement_delta IS the new task goal
                    // (§3.3: the next retrieval loop runs under the new
                    // contract); otherwise the new query is the task.
                    // THIN-HARNESS-REDESIGN R1 (§4.4): with auto-close this
                    // branch is dormant in the production flow (disposition
                    // is gone) — retained for the restore/dormant paths.
                    let mut a = reg.states.remove(&role).unwrap();
                    let task_goal = a.next_goal.take().unwrap_or_else(|| goal.clone());
                    a.conversation.push(Message {
                        role: Role::User,
                        content: task_goal.clone(),
                        tool_call_id: None,
                        tool_calls: Vec::new(),
                        reasoning_content: None,
                    });
                    (a, task_goal)
                }
                // THIN-HARNESS-REDESIGN R1 (§4.4): 每次调用即闭环——已关闭
                // 的激活是终态（verifier：同 activation 首条 close 后不再
                // 允许 assessment/close），后续派发一律新建激活（新预算、
                // 新候选计数、contract_revision 归 0）。`None` 与
                // Closed 共用同一 fresh 分支。
                Some(_) | None => {
                    let seq = reg.next_seq.entry(role).or_insert(0);
                    let activation_id =
                        format!("retrieval-{}-{}-{:02}", role.as_str(), session8, *seq);
                    *seq += 1;
                    (
                        ActivationState {
                            activation_id,
                            parent_session_id: session_id.clone(),
                            subagent_session_id: format!("SUB-{}-{}", role.as_str(), session8),
                            contract_id: format!("retrieval-contract-{}", role.as_str()),
                            contract_revision: 0,
                            status: ActivationStatus::Active,
                            conversation: vec![Message {
                                role: Role::User,
                                content: goal.clone(),
                                tool_call_id: None,
                                tool_calls: Vec::new(),
                                reasoning_content: None,
                            }],
                            pending: None,
                            next_goal: None,
                            result_digest: None,
                            submitted: Vec::new(),
                            // A fresh activation starts a fresh budget
                            // (F5, user adjudication 2026-08-10 — the
                            // budget accumulates only within one
                            // activation's lifetime) and a fresh candidate
                            // count (P0-B step 2/4, 2026-08-14 — design
                            // §1.1: reset only on activation close).
                            tool_rounds_used: 0,
                            candidate_urls: Vec::new(),
                            result_archive_ref: None,
                        },
                        goal.clone(),
                    )
                }
            }
        };
        let goal = task_goal;

        // FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the activation's
        // candidate counter rides the dispatch as a shared mutex — the
        // subagent loop's candidate gate (web_fetch + browser_read)
        // mutates it; it is written back into the activation on EVERY
        // path below, so a `continue` re-entry resumes the same count and
        // a new activation starts empty (design §1.1: per-activation
        // accumulation, reset only on close).
        let fetch_candidates = Arc::new(Mutex::new(std::mem::take(&mut act.candidate_urls)));

        // The subagent's tool projection = the parent's registry minus the
        // main-only control/whitelist tools (the retrieval lane never sees
        // compaction_whitelist_add — a main-run session concept — nor the
        // parent-disposition control tool), plus the host-routed retrieval
        // tools the main lane no longer advertises (2026-08-14 ruling:
        // browser_read is restored from the host registry here).
        let sub_tool_defs = Self::subagent_tool_projection(
            tool_defs,
            host.tools_registry(),
            role,
            self.retrieval_mode,
        );
        // per-run 隔离：子代理用本 run 的 gateway 实例（不再引用
        // controller 上跨 run 共享的 `internal_retrieval` /
        // `external_retrieval`）。
        let subagent = RetrievalSubagent::new(role, run_gateway.clone());

        // GAP-SUBAGENT-RUNTIME (2026-08-10): the subagent runs the SAME
        // shared loop as the main agent — its own budget accounting
        // (`profile.max_tool_rounds` — independent 120), its own journal
        // events in the same hash chain (run terminal uniqueness stays with
        // the parent), the retrieval task contract as IPG input. The lane
        // feeding (orientation) is wired in M5; the session identity above
        // still derives from the session state.
        // F5 (user adjudication 2026-08-10): the activation's consumed
        // rounds carry into this dispatch — a `continue` re-entry is the
        // same retrieval session, so the 120-round budget accumulates.
        let profile = LoopProfile::retrieval(
            role,
            &goal,
            self.retrieval_mode,
            self.max_tool_rounds,
            act.tool_rounds_used,
            &act.activation_id,
            // Only the external lane executes candidate-counted tools
            // (web_fetch lane self-execution / browser_read host route);
            // the internal lane passes `None` — its (never reachable)
            // candidate gate would fail closed.
            match role {
                SubagentRole::ExternalRetrieval => Some(fetch_candidates.clone()),
                SubagentRole::InternalRetrieval => None,
            },
        );
        // Box::pin: the subagent loop is a recursive call through the
        // dispatch edge (main loop → subagent loop; depth is capped at one
        // by the nested-dispatch gate, E0733 requires the box).
        // GAP-RETRIEVAL-TOOLS (2026-08-10): fresh evidence collection per
        // dispatch — the loop fills it from the lane's host calls; result
        // formation consumes it below.
        self.evidence.lock().unwrap().clear();
        let loop_outcome = Box::pin(run_agent_loop(
            &SharedLoopServices {
                blackboard: &self.blackboard,
                denial_state: &self.denial_state,
                pacing_rounds: &self.pacing_rounds,
                context_compact: &self.context_compact,
                dc_state: &self.dc_state,
                // Retrieval lane: collect tool-call evidence (§3.7.4).
                evidence: Some(&self.evidence),
                policy_revision: &self.policy_revision,
                max_inject_tokens_per_round: self.max_inject_tokens_per_round,
                blackboard_archive_dir: self.blackboard_archive_dir(),
            },
            self,
            writer,
            host,
            &subagent,
            &profile,
            &goal, // IPG evaluates the task contract (a query may carry injected content)
            &sub_tool_defs,
            &mut act.conversation,
            // M5: the shared session state is threaded in — the retrieval
            // profile feeds the internal/external lane (§4.2).
            orientation,
            cancel,
            heartbeat,
            run_gateway,
        ))
        .await;

        // F5 (user adjudication 2026-08-10): the session's consumed budget
        // carries into the next dispatch (a continue re-entry) — read back
        // from the loop outcome BEFORE `result` consumes it below. An Err
        // path leaves the count untouched: the activation closes with
        // subagent_failed/subagent_cancelled anyway (a new activation
        // starts a fresh budget). Read back on every path — the close still
        // records the consumed rounds.
        if let Ok(outcome) = &loop_outcome {
            act.tool_rounds_used = outcome.tool_rounds;
        }

        // The subagent's end-of-session compaction (P0-D review fix
        // 2026-08-14): a restored activation resumes from a pinned summary
        // marker instead of a raw restore-time truncation. The subagent's
        // stagnation guard is retired (2026-08-22) — generation-time
        // output-health detection covers the degeneration surface.
        let result: Result<LoopOutcome, AgentLoopError> = match &loop_outcome {
            Ok(outcome_ref) => {
                let estimate = estimate_messages_tokens(&act.conversation);
                if estimate > self.context_compact.session_end_trigger_tokens {
                    let svc = SharedLoopServices {
                        blackboard: &self.blackboard,
                        denial_state: &self.denial_state,
                        pacing_rounds: &self.pacing_rounds,
                        context_compact: &self.context_compact,
                        dc_state: &self.dc_state,
                        evidence: Some(&self.evidence),
                        policy_revision: &self.policy_revision,
                        max_inject_tokens_per_round: self.max_inject_tokens_per_round,
                        blackboard_archive_dir: self.blackboard_archive_dir(),
                    };
                    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010
                    // §14.26): the session-end compaction uses the
                    // loop's final fold state — the summary input is
                    // the same stateful view the main requests saw and
                    // the drain cut is `fold_cut` (never a recomputed
                    // stateless tail).
                    let mut fold_state = outcome_ref.fold_state.clone();
                    let _ = run_template_compact(
                        &svc,
                        writer,
                        host,
                        &mut act.conversation,
                        estimate,
                        "session_end",
                        true,
                        false,
                        outcome_ref.rounds_since_compact,
                        self.context_compact.recent_tail_rounds,
                        // 检索车道不折叠：marker 不携带外挂台账提示。
                        None,
                        &mut fold_state,
                    )
                    .await?;
                }
                loop_outcome
            }
            Err(_) => loop_outcome,
        };

        // Write the candidate counter back into the activation (every
        // path — success, error and cancel keep the count; the close
        // record still observes the consumed candidates).
        act.candidate_urls = std::mem::take(&mut *fetch_candidates.lock().unwrap());

        // Re-insert the activation — the conversation is preserved on every
        // path (§4.4: journal, docs, ledger, receipts never deleted).
        let activation_identity = (
            act.activation_id.clone(),
            act.contract_id.clone(),
            act.contract_revision,
        );
        // GAP-RETRIEVAL-TOOLS: the subagent session id rides the committed
        // result payload — cloned before the move into the registry.
        let subagent_session_id = act.subagent_session_id.clone();
        self.activations.lock().unwrap().states.insert(role, act);

        let tool_result = match result {
            Ok(outcome) => {
                // IP2a (D-3): 失败必显式 — a subagent that returned no text
                // must not leave a blank tool message for the model.
                let output = outcome
                    .last_text
                    .filter(|t| !t.trim().is_empty())
                    .unwrap_or_else(|| format!("retrieval '{target_name}' returned no text"));
                // GAP-SUBAGENT-RUNTIME: result formation — the [DOC]/[SOURCE]
                // line contract writes the subagent's own blackboard section
                // (single-writer discipline; the stable interface real
                // retrieval semantics plug into).
                let (docs, sources) = crate::agents::retrieval::parse_retrieval_text(&output);
                // GAP-RETRIEVAL-TOOLS: the structured result consumes the
                // parsed lines by reference (ledger merge); `write_section`
                // takes them by value afterwards.
                let (activation_id, contract_id, contract_revision) = activation_identity;
                let evidence = self.evidence.lock().unwrap().clone();
                // Run-unique source_id allocation (P0-B step 5 review fix):
                // take the counter, consume it synchronously, write it back —
                // the lock never spans the awaits below.
                let mut source_seq = *self.next_source_seq.lock().unwrap();
                let committed = build_structured_result(
                    &evidence,
                    &self.source_weighting,
                    &self.candidate_prefilter,
                    &mut source_seq,
                    &docs,
                    &sources,
                    match role {
                        SubagentRole::InternalRetrieval => "project_doc",
                        SubagentRole::ExternalRetrieval => "web_page",
                    },
                    &output,
                    &subagent_session_id,
                    &activation_id,
                    &contract_id,
                    contract_revision,
                    &tc.call_id,
                    &goal,
                );
                *self.next_source_seq.lock().unwrap() = source_seq;
                // THIN-HARNESS-REDESIGN R2a 审查处理 (P2-2)：分区 ledger =
                // 结构化 ledger 投影（source_id + 标题/URL）——机械证据以
                // 模型可读形态进分区；internal/external 同口径。P2-3：
                // write_section 全量覆盖，指针摘要计数与分区内容一一对应。
                let ledger_projection: Vec<String> = committed.payload["source_ledger"]
                    .as_array()
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(|e| {
                                let id = e
                                    .get("source_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();
                                let title = e
                                    .get("source_title")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();
                                let url = e
                                    .get("source_url_or_ref")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or_default();
                                let label = if title.is_empty() { url } else { title };
                                if label.is_empty() {
                                    None
                                } else if !url.is_empty() && url != title {
                                    Some(format!("{id} {label} ({url})"))
                                } else {
                                    Some(format!("{id} {label}"))
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                crate::agents::retrieval::write_section(
                    role,
                    &self.blackboard,
                    output.clone(),
                    docs,
                    sources,
                    ledger_projection,
                );
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                            "exit_code": 0,
                        }),
                    )
                    .await?;
                // GAP-INQUIRY-SPLIT (2026-08-09): the free-form
                // RETRIEVAL_COMPLETION_CHECK (model self-reported "是否已获得
                // 所需内容") is deleted — ADR-0010 §4.3 forbids model free-text
                // verdicts.
                //
                // GAP-RETRIEVAL-TOOLS (2026-08-10): structured result
                // formation — ADR-0010 §3.3.3 five sections. The ledger/
                // query_summary/filtering_log/raw_source_refs are built
                // MECHANICALLY from the lane's tool-call evidence (single
                // writer: the controller); the model's `[RESULT_JSON]` block
                // supplies the organized_response and is validated against
                // the ledger (source_ids ⊆ ledger, claim × visibility
                // matrix §3.7.5). Validation failure degrades explicitly —
                // visibility_degraded + reason code, never a silent
                // downgrade. The result is committed as an event, archived
                // to `{journal_dir}/retrieval-results/` (best-effort), and
                // the assessment consumes its mechanical facts.
                let result_digest = committed.result_digest.clone();
                let ledger_digest = committed.ledger_digest.clone();
                let source_counts = committed.source_counts.clone();
                writer
                    .record(
                        EventType::RetrievalResultCommitted,
                        committed.payload.clone(),
                    )
                    .await?;
                // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): keep this
                // run's committed ledgers — the final-answer citation
                // verifier binds `[来源: ...]` markers to them (ADR-0010
                // §3.7.9; per-run cleared at run start).
                if let Some(ledger) = committed.payload.get("source_ledger") {
                    self.run_source_ledgers.lock().unwrap().push(ledger.clone());
                }
                // GAP-RETRIEVAL-TOOLS (2026-08-10): the committed result's
                // source identities count as examined surfaces for the DC
                // signals (key_surface_unexamined; main lane — the
                // subagent dispatch runs under the main profile).
                crate::diagnostic_coverage::maybe_consume_dc_retrieval_evidence(
                    &self.dc_state,
                    &committed,
                )
                .await?;
                let artifact_ref = self.persist_result_artifact(
                    writer,
                    &committed,
                    &activation_id,
                    contract_revision,
                );
                // Review P2-3 (2026-08-10): the id binds the CALL dimension
                // too — two identical outputs on different activations must
                // not collide (the §4.4 verifier rejects a replayed
                // assessment_id carrying a different payload). Both halves
                // are hex digests, matching the `ASSESS-[A-Za-z0-9._-]+`
                // pattern.
                let assessment_id = format!(
                    "ASSESS-{}-{}",
                    &result_digest[..16],
                    &sha256_hex(tc.call_id.as_bytes())[..8],
                );
                {
                    let mut reg = self.activations.lock().unwrap();
                    if let Some(a) = reg.states.get_mut(&role) {
                        a.result_digest = Some(result_digest.clone());
                        a.result_archive_ref = artifact_ref;
                    }
                }
                // source_categories — the mechanical source-type set of the
                // committed ledger.
                let categories: Vec<String> = committed.payload["source_ledger"]
                    .as_array()
                    .map(|ledger| {
                        let mut seen = std::collections::BTreeSet::new();
                        for entry in ledger {
                            if let Some(st) = entry["source_type"].as_str() {
                                seen.insert(st.to_string());
                            }
                        }
                        seen.into_iter().collect()
                    })
                    .unwrap_or_default();
                let mut reason_codes = vec!["no_mechanical_coverage_requirement"];
                if let Some(note) = &committed.validation_note {
                    reason_codes.push(note.as_str());
                }
                writer
                    .record(
                        EventType::InformationSufficiencyAssessment,
                        serde_json::json!({
                            "assessment_id": assessment_id,
                            "activation_id": activation_id,
                            "contract_id": contract_id,
                            "contract_revision": contract_revision,
                            "result_digest": result_digest,
                            "ledger_digest": ledger_digest,
                            "source_counts": source_counts,
                            "source_categories": categories,
                            "missing_categories": [],
                            "filtering_reasons": [],
                            "status": "indeterminate",
                            "reason_codes": reason_codes,
                            "source_visibility_gate": "not_applicable",
                            "assessment_version": "0.1.0",
                        }),
                    )
                    .await?;
                // M4: budget exhaustion on the subagent loop is a TERMINAL
                // authority — a partial result was formed and assessed, then
                // the activation closes with `budget_exhausted` (assessment +
                // digest bound, no disposition; §4.4 terminal close).
                if outcome.budget_exhausted {
                    self.close_activation(
                        writer,
                        role,
                        "budget_exhausted",
                        Some(&assessment_id),
                        Some(&result_digest),
                    )
                    .await?;
                }
                // THIN-HARNESS-REDESIGN R1 (2026-08-27, §4.1/§4.4): 每次
                // 调用即闭环——结果形成并机械评估后立即关闭激活
                // （terminal_reason=auto_close），不再进入 AwaitingDisposition、
                // 不再向主代理回传 [ASSESSMENT] 行、不再要求
                // retrieval_disposition close/continue 往返（父代理需要继续
                // 检索时直接再调 web 工具，新激活即开）。close_activation
                // 幂等——budget_exhausted 已关闭时本调用为空操作。
                self.close_activation(
                    writer,
                    role,
                    "auto_close",
                    Some(&assessment_id),
                    Some(&result_digest),
                )
                .await?;
                // THIN-HARNESS-REDESIGN R2a (2026-08-27, §4.4): 检索派发
                // 结果通道——`blackboard`（默认）：主代理工具结果只回指针
                // 摘要（来源/结论计数 + 分区名 + 条目上限），子代理全文
                // 保留在 internal_ret / external_ret 分区与 journal /
                // retrieval-results 存档（留痕不变），需要时用
                // `blackboard_read section=...` 拉取；`inline`：保留旧
                // 行为（全文回传），供 A/B 与回退。`output` 变量仍被
                // write_section 与 build_structured_result 消费，这里
                // 只决定回传主对话的文本。
                let tool_output = match retrieval_result_channel_from_env() {
                    RetrievalResultChannel::Inline => output,
                    RetrievalResultChannel::Blackboard => {
                        let total_sources = committed.source_counts["total"].as_u64().unwrap_or(0);
                        let conclusion_count = committed.payload["organized_response"]["sections"]
                            .as_array()
                            .map(|a| a.len())
                            .unwrap_or(0);
                        format!(
                            "{}: 已写入 blackboard {}（{} 来源 / {} 结论，条目上限 8K）；\
                             需要详情时用 blackboard_read section={} 读取",
                            tc.name,
                            role.section_name(),
                            total_sources,
                            conclusion_count,
                            role.section_name(),
                        )
                    }
                };
                ToolResult {
                    output: tool_output,
                    exit_code: Some(0),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                }
            }
            Err(e) => {
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "target": target_name,
                            "status": "error",
                            "error": e.to_string(),
                        }),
                    )
                    .await?;
                // M4: terminal authority close — the subagent failed (or was
                // cancelled mid-run); no result was formed, so the close
                // carries no assessment/digest (resumable=true).
                let reason = match &e {
                    AgentLoopError::Cancelled => "subagent_cancelled",
                    _ => "subagent_failed",
                };
                self.close_activation(writer, role, reason, None, None)
                    .await?;
                ToolResult {
                    output: format!("retrieval error: {e}"),
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                }
            }
        };

        // Blackboard: subagent wrote its own section; mirror the result into
        // the exec section for the main agent's visibility. 2026-08-08
        // blackboard partition: fold the retrieval dispatch into the
        // tool-action section (category "retrieval" — subagent calls count
        // as one semantic action each).
        {
            let mut w = self.blackboard.write();
            w.tool_actions.push(ToolActionRecord {
                category: "retrieval".to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
            });
            w.exec
                .results
                .push(format!("[{}] {}", tc.name, tool_result.output));
        }
        messages.push(Message {
            role: Role::Tool,
            content: tool_result.output.clone(),
            // The provider protocol needs the call this result answers; the
            // call_id travels from the model's request through the journal.
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        Ok(tool_result)
    }
}
