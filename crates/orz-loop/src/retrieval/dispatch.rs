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
use crate::blackboard::{DispatchStamp, ToolActionRecord};
use crate::controller::{
    AgentLoopController, AgentLoopError, EventWriter, RetrievalCapability, RetrievalMode,
    RetrievalResultChannel, chrono_utc_now, estimate_messages_tokens,
    retrieval_result_channel_from_env, retrieval_subagent_max_tool_rounds_override,
    retrieval_subagent_wallclock_override,
};
use crate::gateway::model::{Message, ModelGateway, Role, ToolCall};
use crate::host::{LoopHost, ToolDef, ToolResult};
use crate::orientation::OrientationSessionState;
use crate::relay::DispatchTarget;
use crate::retrieval::activation::{ActivationState, ActivationStatus};
use crate::retrieval::effort::{
    classify_retrieval_effort, effort_inputs_from_args, retrieval_effort_override,
};
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
        // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
        // 委托契约复杂度分档——先机械分档（env 强制覆盖优先），档位默认
        // max_results 并入 goal；档位只调执行预算，模型面零改动。
        let (effort, goal) = {
            let (query, scope, max_results) = effort_inputs_from_args(&tc.arguments);
            let tier = retrieval_effort_override().unwrap_or_else(|| {
                classify_retrieval_effort(
                    &query,
                    scope.as_deref(),
                    max_results,
                    role == SubagentRole::ExternalRetrieval,
                )
            });
            let goal = Self::build_retrieval_task_goal(&tc.arguments, prompt, Some(tier));
            (tier, goal)
        };

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
                    // 第二批分档：`continue` 重入覆盖为最新档（close record
                    // 登记最后一次派发的 effort）。
                    a.effort = Some(effort);
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
                            effort: Some(effort),
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
        // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
        // 委托契约复杂度分档——预算解析顺序：显式 env（含 0=禁用） >
        // 测试/配置 seam（controller 字段） > 档位默认（240/600/900s、
        // 30/60/90 轮）；与主车道仍取 min。
        let retrieval_wallclock: Option<std::time::Duration> =
            match retrieval_subagent_wallclock_override() {
                Some(v) => v,
                None => self
                    .retrieval_subagent_wallclock
                    .or_else(|| Some(effort.wallclock_default())),
            };
        let retrieval_max_rounds: Option<u32> = match retrieval_subagent_max_tool_rounds_override()
        {
            Some(v) => v,
            None => self
                .retrieval_max_tool_rounds
                .or_else(|| Some(effort.max_tool_rounds_default())),
        };
        let profile = LoopProfile::retrieval(
            role,
            &goal,
            self.retrieval_mode,
            // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30): 检索
            // 子代理独立轮数上限（默认 60），与主车道全局轮数取 min——
            // 测试用 with_max_tool_rounds 缩小时语义不变；None（env 0）
            // = 禁用独立上限，仅用主车道（P3-5 对齐墙钟 0=禁用语义）。
            retrieval_max_rounds
                .map(|r| self.max_tool_rounds.min(r))
                .unwrap_or(self.max_tool_rounds),
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
            // 第二批分档：同轮 browser_read 并行上限（standard 2 /
            // extended 4 / deep 不限——host tab 池为最终物理上限）。
            effort.browser_read_concurrency(),
        );
        // Box::pin: the subagent loop is a recursive call through the
        // dispatch edge (main loop → subagent loop; depth is capped at one
        // by the nested-dispatch gate, E0733 requires the box).
        // GAP-RETRIEVAL-TOOLS (2026-08-10): fresh evidence collection per
        // dispatch — the loop fills it from the lane's host calls; result
        // formation consumes it below.
        self.evidence.lock().unwrap().clear();
        // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30): 单次检索派发
        // 墙钟预算——包裹整个子代理 loop（轮数预算在其内）；超时即丢弃
        // loop future，父 run 继续，激活以 subagent_timeout 收口（Err 分支）。
        // 事件链：子代理事件已随同一 writer 入链，丢弃后父代理从同一
        // seq/prev_hash 续写，ToolCompleted(error) 补全本次派发的审计形态。
        // 0k 审查处理 (P3-4)：in-flight 工具槽——run_agent_loop 串行工具
        // 执行处入/出槽；墙钟超时 drop loop future 后，据此为链上孤儿
        // ToolStarted 补 ToolCompleted(error)（审计形态完整）。
        let in_flight_tools: std::sync::Mutex<Vec<(String, String)>> =
            std::sync::Mutex::new(Vec::new());
        let svc = SharedLoopServices {
            blackboard: &self.blackboard,
            denial_state: &self.denial_state,
            pacing_rounds: &self.pacing_rounds,
            context_compact: &self.context_compact,
            // Retrieval lane: collect tool-call evidence (§3.7.4).
            evidence: Some(&self.evidence),
            policy_revision: &self.policy_revision,
            max_inject_tokens_per_round: self.max_inject_tokens_per_round,
            blackboard_archive_dir: self.blackboard_archive_dir(),
            in_flight_tools: Some(&in_flight_tools),
        };
        let loop_future = Box::pin(run_agent_loop(
            &svc,
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
        ));
        let loop_outcome = match retrieval_wallclock {
            Some(budget) => match tokio::time::timeout(budget, loop_future).await {
                Ok(outcome) => outcome,
                Err(_) => {
                    tracing::warn!(
                        ?budget,
                        target = %target_name,
                        "retrieval subagent wallclock exceeded — closing activation"
                    );
                    // 0k 审查处理 (P3-4)：为 in-flight 孤儿 ToolStarted 补
                    // ToolCompleted(error)——子代理 loop 被 drop 时正在
                    // 执行的串行工具已记录 ToolStarted、未记录
                    // ToolCompleted；合成收口让审计形态完整（tool/call_id
                    // 与孤儿 ToolStarted 配对，验证器 started→completed
                    // 形态闭合）。
                    let interrupted: Vec<(String, String)> =
                        in_flight_tools.lock().unwrap().clone();
                    for (tool, call_id) in interrupted {
                        writer
                            .record(
                                EventType::ToolCompleted,
                                serde_json::json!({
                                    "tool": tool,
                                    "call_id": call_id,
                                    "status": "error",
                                    "error": "subagent_wallclock_timeout_mid_tool",
                                }),
                            )
                            .await?;
                    }
                    Err(AgentLoopError::RetrievalSubagentTimeout)
                }
            },
            None => loop_future.await,
        };

        // F5 (user adjudication 2026-08-10): the session's consumed budget
        // carries into the next dispatch (a continue re-entry) — read back
        // from the loop outcome BEFORE `result` consumes it below. An Err
        // path leaves the count untouched: the activation closes with
        // subagent_failed/subagent_cancelled anyway (a new activation
        // starts a fresh budget). Read back on every path — the close still
        // records the consumed rounds.
        // 0k 审查处理 (P3)：Err 路径（含 subagent_timeout）不更新
        // `act.tool_rounds_used`——超时后激活以 subagent_timeout 关闭
        // （closed），continue 重入不可能，陈旧轮数值不产生语义影响；
        // close record 不携带轮数，审计以事件链为准。
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
                        evidence: Some(&self.evidence),
                        policy_revision: &self.policy_revision,
                        max_inject_tokens_per_round: self.max_inject_tokens_per_round,
                        blackboard_archive_dir: self.blackboard_archive_dir(),
                        in_flight_tools: Some(&in_flight_tools),
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
                // RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO
                // P0-0k 第一批第 3 项)：[DOC]/[SOURCE] 声明行机械有界化——
                // ledger/事件 payload 与 inline 回传用有界视图（行数+字节
                // 上限 + 结构化标注 + 全文指针）；blackboard 分区保留全文
                // （留痕不变，主代理可 blackboard_read 拉取）。
                let bounded = crate::agents::retrieval::bound_retrieval_result(
                    &output,
                    &docs,
                    &sources,
                    role.section_name(),
                );
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
                    &bounded.docs,
                    &bounded.sources,
                    match role {
                        SubagentRole::InternalRetrieval => "project_doc",
                        SubagentRole::ExternalRetrieval => "web_page",
                    },
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
                // B1 会话化基础（2026-09-03，R1）：派发完成时取写时章——
                // round/domain 来自本派发所属主决策轮的 LIF 当前态（写时
                // 单一来源，与 failure_agg/exec 同刻度）。锁不跨 write_section。
                let (round, domain) = self.blackboard_stamp();
                crate::agents::retrieval::write_section(
                    role,
                    &self.blackboard,
                    output.clone(),
                    docs,
                    sources,
                    ledger_projection,
                    DispatchStamp {
                        round,
                        domain,
                        timestamp: chrono_utc_now(),
                    },
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
                // GAP-RETRIEVAL-TOOLS (2026-08-10) / GAP-RETRIEVAL-
                // STRUCTURED-RESULT 方向 C (2026-08-30): structured result
                // formation — ADR-0010 §3.3.3 mechanical four sections
                // (query_summary/source_ledger/filtering_log/
                // raw_source_refs), built MECHANICALLY from the lane's
                // tool-call evidence + [DOC]/[SOURCE] declaration lines
                // (single writer: the controller; the `[RESULT_JSON]`
                // organized block is deleted). `visibility_degraded` =
                // no text-level evidence (reason code `no_fulltext_evidence`),
                // never a silent downgrade. The result is committed as an
                // event, archived to `{journal_dir}/retrieval-results/`
                // (best-effort), and the assessment consumes its mechanical
                // facts.
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
                    RetrievalResultChannel::Inline => bounded.output,
                    RetrievalResultChannel::Blackboard => {
                        let total_sources = committed.source_counts["total"].as_u64().unwrap_or(0);
                        // GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C
                        // (2026-08-30)：organized_response 已删除——摘要
                        // 口径改为机械文本级证据计数（full + partial）。
                        let text_evidence = committed.payload["source_counts"]["full_text_observed"]
                            .as_u64()
                            .unwrap_or(0)
                            + committed.payload["source_counts"]["partial_text_observed"]
                                .as_u64()
                                .unwrap_or(0);
                        format!(
                            "{}: 已写入 blackboard {}（{} 来源 / {} 文本证据，条目上限 8K）；\
                             需要详情时用 blackboard_read section={} 读取",
                            tc.name,
                            role.section_name(),
                            total_sources,
                            text_evidence,
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
                    AgentLoopError::RetrievalSubagentTimeout => "subagent_timeout",
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
        // B1：写时盖 (round, domain) 章（与主决策轮同刻度）。
        let (round, domain) = self.blackboard_stamp();
        {
            let mut w = self.blackboard.write();
            w.push_tool_action(ToolActionRecord {
                category: "retrieval".to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
                round,
                domain: Some(domain),
            });
            w.push_exec_result(crate::blackboard::ExecEntry::stamped(
                format!("[{}] {}", tc.name, tool_result.output),
                round,
                domain,
                chrono_utc_now(),
            ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{ExternalRetSection, InternalRetSection};
    use crate::controller::{
        CandidateGateDecision, RETRIEVAL_RESULT_CHANNEL_ENV, commit_candidate,
        parse_max_inject_tokens_per_round, parse_retrieval_subagent_max_tool_rounds,
        parse_retrieval_subagent_wallclock, parse_web_fetch_candidate_cap, rollback_candidate,
    };
    use crate::controller_test_support::*;
    use crate::denial::PolicyFeedback;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{GatewayError, ModelRequest, ModelResponse};
    use async_trait::async_trait;
    use orz_assurance::{EventTrack, JournalRecorder, RunEvent};

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P2-1, 2026-08-27)：env 突变
    /// 测试共享同一把锁——此前 `retrieval_dispatch_inline_channel_keeps_
    /// full_text` 与 `retrieval_result_channel_env_parse_rules` 各自声明
    /// 函数级 ENV_LOCK（互不排斥），且默认指针摘要测试无锁读取同一 env；
    /// inline 测试持 env=inline 跨整个 async run_turn 期间，并行测试可能
    /// 读到 inline 导致断言 flake。三个测试统一持本锁（读方也持锁排除
    /// 突变窗口；std::sync::MutexGuard 跨 await 仅对 current_thread
    /// 测试运行时成立，本文件 tokio::test 默认即此）。
    static RETRIEVAL_CHANNEL_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P3-1): 恢复时重建检索分区——
    /// sidecar 快照携带的 internal_ret / external_ret 经
    /// `with_retrieval_partitions` 灌回全新黑板；`None` 保持空分区。
    #[test]
    fn with_retrieval_partitions_seeds_blackboard_sections() {
        let internal = InternalRetSection {
            project_docs: vec!["design.md".to_string()],
            source_ledger: vec!["SRC-001 design.md".to_string()],
            response: Some("恢复的检索完成".to_string()),
            stamp: None,
        };
        let external = ExternalRetSection {
            web_sources: vec!["https://example.com/paper".to_string()],
            source_ledger: vec!["SRC-002 https://example.com/paper".to_string()],
            response: Some("恢复的网页检索完成".to_string()),
            stamp: None,
        };
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_retrieval_partitions(Some(internal.clone()), Some(external.clone()));
        let bb = controller.blackboard().read();
        assert_eq!(bb.internal_ret.project_docs, internal.project_docs);
        assert_eq!(bb.internal_ret.source_ledger, internal.source_ledger);
        assert_eq!(bb.internal_ret.response, internal.response);
        assert_eq!(bb.external_ret.web_sources, external.web_sources);
        assert_eq!(bb.external_ret.source_ledger, external.source_ledger);
        assert_eq!(bb.external_ret.response, external.response);

        // None 不动分区（默认空）。
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_retrieval_partitions(None, None);
        let bb = controller.blackboard().read();
        assert!(bb.internal_ret.project_docs.is_empty());
        assert!(bb.external_ret.web_sources.is_empty());
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): 检索派发默认返回指针摘要——主对话
    /// 只收到「已写入 blackboard 分区（N 来源 / M 结论）」指针，子代理全文
    /// 只留在分区与 journal（留痕不变）。审查处理 (P2-1)：读方也持共享
    /// env 锁，排除并行 env 突变测试的干扰窗口。
    #[tokio::test]
    async fn retrieval_dispatch_returns_pointer_summary_by_default() {
        let _guard = RETRIEVAL_CHANNEL_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-PTR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-1"))
            })
            .expect("call-1 round");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-1"))
            .expect("retrieval dispatch tool result message");
        assert!(
            reply.content.contains("已写入 blackboard internal_ret")
                && reply.content.contains("来源")
                && reply
                    .content
                    .contains("blackboard_read section=internal_ret"),
            "pointer summary reply: {:?}",
            round.messages
        );
        assert!(
            !reply.content.contains("检索完成"),
            "full subagent text must not reach the main conversation: {:?}",
            round.messages
        );
        // 分区仍保留全文（留痕不变）。
        let r = controller.blackboard().read();
        assert!(
            r.internal_ret
                .response
                .as_deref()
                .unwrap()
                .contains("检索完成"),
            "section keeps the full text"
        );
        assert_eq!(r.internal_ret.project_docs, vec!["design.md"]);
        // P2-2: 分区 ledger = 结构化投影（SRC 编号 + 标题），非空。
        assert_eq!(
            r.internal_ret.source_ledger,
            vec!["SRC-001 design.md"],
            "partition ledger is the structured projection"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P2-3)：每次派发全量覆盖分区
    /// ——第二次派发替换（而非累积）response / entries / ledger，指针
    /// 摘要计数与分区内容一一对应。
    #[tokio::test]
    async fn retrieval_dispatch_overwrites_partition_on_each_dispatch() {
        let _guard = RETRIEVAL_CHANNEL_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] old.md\n第一轮检索完成"),
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::text("[DOC] new.md\n第二轮检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "两次查找项目文档",
                "RUN-PTW",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let r = controller.blackboard().read();
        assert_eq!(
            r.internal_ret.project_docs,
            vec!["new.md"],
            "second dispatch replaces entries"
        );
        assert_eq!(
            r.internal_ret.source_ledger,
            vec!["SRC-002 new.md"],
            "second dispatch replaces the ledger"
        );
        assert_eq!(
            r.internal_ret.response.as_deref(),
            Some("[DOC] new.md\n第二轮检索完成"),
            "second dispatch replaces the response (全文保留，含 [DOC] 行)"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): `ORZ_RETRIEVAL_RESULT_CHANNEL=inline`
    /// 保留旧行为——子代理全文回传主对话（A/B 与回退通道）。
    #[tokio::test]
    async fn retrieval_dispatch_inline_channel_keeps_full_text() {
        // P2-1: 共享锁（模块级 RETRIEVAL_CHANNEL_ENV_LOCK），与解析规则
        // 测试及默认指针摘要测试互斥；还原放在断言前避免 panic 泄漏。
        let _guard = RETRIEVAL_CHANNEL_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let original = std::env::var(RETRIEVAL_RESULT_CHANNEL_ENV).ok();
        unsafe {
            std::env::set_var(RETRIEVAL_RESULT_CHANNEL_ENV, "inline");
        }
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-INL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        // 还原 env（无论断言结果——放在断言前，避免 panic 泄漏）。
        match original {
            Some(v) => unsafe { std::env::set_var(RETRIEVAL_RESULT_CHANNEL_ENV, v) },
            None => unsafe { std::env::remove_var(RETRIEVAL_RESULT_CHANNEL_ENV) },
        }

        let received = fake.received_requests();
        let round = received
            .iter()
            .find(|r| {
                r.messages
                    .iter()
                    .any(|m| m.tool_call_id.as_deref() == Some("call-1"))
            })
            .expect("call-1 round");
        let reply = round
            .messages
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-1"))
            .expect("retrieval dispatch tool result message");
        assert!(
            reply.content.contains("检索完成"),
            "inline channel keeps the full text: {:?}",
            round.messages
        );
        assert!(
            !reply.content.contains("已写入 blackboard"),
            "inline channel must not emit the pointer summary: {:?}",
            round.messages
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R2a (§4.4): `ORZ_RETRIEVAL_RESULT_CHANNEL`
    /// 解析规则——缺失/无效回退 `blackboard`，`inline` 命中旧行为。
    #[test]
    fn retrieval_result_channel_env_parse_rules() {
        // P2-1: 共享锁，与 inline 测试 / 默认指针摘要测试互斥。
        let _guard = RETRIEVAL_CHANNEL_ENV_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let original = std::env::var(RETRIEVAL_RESULT_CHANNEL_ENV).ok();
        unsafe {
            std::env::remove_var(RETRIEVAL_RESULT_CHANNEL_ENV);
        }
        assert_eq!(
            retrieval_result_channel_from_env(),
            RetrievalResultChannel::Blackboard,
            "missing env → blackboard default"
        );
        unsafe {
            std::env::set_var(RETRIEVAL_RESULT_CHANNEL_ENV, "inline");
        }
        assert_eq!(
            retrieval_result_channel_from_env(),
            RetrievalResultChannel::Inline,
            "inline env honored"
        );
        unsafe {
            std::env::set_var(RETRIEVAL_RESULT_CHANNEL_ENV, "bogus");
        }
        assert_eq!(
            retrieval_result_channel_from_env(),
            RetrievalResultChannel::Blackboard,
            "invalid env falls back to blackboard"
        );
        match original {
            Some(v) => unsafe { std::env::set_var(RETRIEVAL_RESULT_CHANNEL_ENV, v) },
            None => unsafe { std::env::remove_var(RETRIEVAL_RESULT_CHANNEL_ENV) },
        }
    }

    #[tokio::test]
    async fn retrieval_call_dispatches_to_subagent() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        // Script: main agent requests internal retrieval → subagent returns
        // doc-tagged text → main agent concludes (first text is gate-
        // intercepted, the second is the post-gate final answer).
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-RET",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let types = event_types(&dir);
        assert!(types.contains(&EventType::ToolStarted), "{types:?}");
        // Subagent wrote its blackboard section.
        let bb = controller.blackboard();
        let r = bb.read();
        assert_eq!(r.internal_ret.project_docs, vec!["design.md"]);
        assert!(
            r.internal_ret
                .response
                .as_deref()
                .unwrap()
                .contains("检索完成")
        );
        // External section untouched.
        assert!(r.external_ret.web_sources.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GAP-SUBAGENT-RUNTIME (2026-08-10): shared-loop subagent semantics ──

    /// A retrieval dispatch whose subagent runs MULTIPLE rounds, including a
    /// host-tool round — the shared loop's journal events (subagent
    /// model_output ×2, the host ToolCompleted) land in the same chain
    /// inside the parent's wrapper.
    #[tokio::test]
    async fn subagent_loop_runs_multi_round_with_host_tool() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "file contents".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };

        // main declares retrieval → subagent round 1 calls read_file (host
        // tool, allowed in the lane) → subagent round 2 forms the result →
        // main concludes (gate + final).
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-2")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-MR",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let types = event_types(&dir);
        // Subagent: 2 model rounds + 1 host tool round — all inside the
        // parent's tool_started/tool_completed pair.
        let sub_outputs = types
            .iter()
            .filter(|t| **t == EventType::ModelOutput)
            .count();
        assert_eq!(sub_outputs, 5, "{types:?}"); // main 3 (decl/gate-answer/final) + subagent 2
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == EventType::ToolCompleted)
                .count(),
            2, // read_file + the retrieval wrapper
            "{types:?}"
        );
        // The read_file host tool executed inside the lane.
        let events = events(&dir);
        let read_completed = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")
            })
            .expect("read_file ToolCompleted");
        assert_eq!(
            read_completed.payload.get("exit_code"),
            Some(&serde_json::json!(0))
        );
        // Result formed into the section.
        let bb = controller.blackboard();
        let r = bb.read();
        assert_eq!(r.internal_ret.project_docs, vec!["design.md"]);
        assert!(
            r.internal_ret
                .response
                .as_deref()
                .unwrap()
                .contains("检索完成")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The ADR-0010 §3.2 deny-only write domain: a mutation-class tool call
    /// inside the retrieval lane is structurally refused (never reaching the
    /// host permission bridge) and journaled as ToolCompleted(status=error).
    #[tokio::test]
    async fn subagent_denies_mutation_tools_with_structured_reason() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "edited".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("search_replace", "call-2")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-WG",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let denied = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("role-gate denial journaled");
        assert_eq!(
            denied.payload.get("error").and_then(|v| v.as_str()),
            Some("retrieval_role_write_denied")
        );
        // The host tool never executed — no second success completion.
        assert_eq!(
            events
                .iter()
                .filter(|e| {
                    e.event_type == EventType::ToolCompleted
                        && e.payload.get("exit_code") == Some(&serde_json::json!(0))
                })
                .count(),
            1, // only the retrieval wrapper succeeded
        );
        // The model saw the refusal in its tool reply.
        let conversation_denial = events
            .iter()
            .find(|e| e.event_type == EventType::ToolCompleted && e.payload.get("error").is_some());
        assert!(conversation_denial.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// One seat per role: a retrieval lane never dispatches another
    /// retrieval (ADR-0010 §11.3) — refused with a structured denial.
    /// C2-1 (2026-08-11): the anti-recursion guard now covers ONLY the
    /// retrieval-dispatch family (`retrieve_project_*`); the web family
    /// self-executes (see `retrieval_lane_web_search_routes_to_host`).
    #[tokio::test]
    async fn subagent_refuses_nested_subagent_dispatch() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            // Inside the retrieval lane: dispatching ANOTHER retrieval is
            // still refused (recursion guard intact).
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-NS",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let denied = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("nested-dispatch denial journaled");
        assert_eq!(
            denied.payload.get("error").and_then(|v| v.as_str()),
            Some("nested_subagent_dispatch_refused")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C2-1 (2026-08-11, ADR-0006 web-search slice): inside a retrieval
    /// lane, a web tool self-executes through the HOST instead of being
    /// refused as a nested dispatch — the lane IS the web lane (ADR-0010
    /// §3.7.8). With TestHost the call reaches `call_tool` (the NotFound
    /// proves it) and no `nested_subagent_dispatch_refused` is journaled;
    /// the lane also skips the permission bridge (no PermissionRequested
    /// for the web call — the mode gate is the authorization chain).
    #[tokio::test]
    async fn retrieval_lane_web_search_routes_to_host() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-2")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-R2H",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        assert!(
            !events.iter().any(|e| {
                e.payload.get("error").and_then(|v| v.as_str())
                    == Some("nested_subagent_dispatch_refused")
            }),
            "the web tool must not be refused as a nested dispatch"
        );
        // The web call reached the host toolset (TestHost has no tool
        // result — NotFound proves the call arrived at call_tool).
        let host_failure = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_search")
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .expect("web_search host failure journaled");
        let err = host_failure
            .payload
            .get("error")
            .and_then(|v| v.as_str())
            .expect("error field");
        assert!(
            err.contains("test host has no tool result"),
            "web_search must reach the host call_tool: {err}"
        );
        // Lane self-execution skips the permission bridge (no
        // PermissionRequested for the web call).
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::PermissionRequested
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_search")
            }),
            "lane self-execution must not consult the permission bridge"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C2-1 (2026-08-11): the mode=off gate covers the web family at the
    /// host path too (belt and braces — "off means no retrieval tools",
    /// ADR-0010 §3.7.1). Topologically the lane self-execution path can
    /// only carry web tools under framework_fallback, so this exercises
    /// run_host_tool directly (in-flight mode transitions are the edge it
    /// defends).
    #[tokio::test]
    async fn mode_off_gate_covers_lane_web_search() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        // Default controller — mode=off.
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("x"),
        ])));
        let tc = ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({"query": "t"}),
            call_id: "call-woff".to_string(),
        };
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(None, EventTrack::V02, "RUN-WOFF", "", 0, None, None);
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &tc,
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                None,
                None, // main-lane direct call: no web_fetch count domain
                true,
                true, // main-lane semantics: probe write-back enabled
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(
            result.output.contains("refused") && result.output.contains("off"),
            "{}",
            result.output
        );
        // P0-C S3 前置 (2026-08-15, P1-2 定案): the refusal carries the
        // structured signal at the run_host_tool boundary.
        let denial = result.policy_denial.as_ref().expect("structured denial");
        assert_eq!(
            denial.source,
            crate::host::PolicyDenialSource::RetrievalMode
        );
        assert_eq!(denial.code, "retrieval_mode_off");
        assert!(feedback.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// C2-1 (2026-08-11): the mirror lane gate — web tools are only
    /// reachable under framework_fallback; under local_browser the
    /// subagent's self-executed web call is refused with an explicit
    /// `retrieval_mode_requires_framework_fallback` (no silent cross-lane
    /// fallback, ADR-0010 §3.7.1). No ToolStarted for the refused call.
    #[tokio::test]
    async fn local_browser_lane_refuses_web_tools() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-2")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_retrieval_mode(
            RetrievalMode::LocalBrowser,
            RetrievalCapability::Available,
            false,
            None,
            None,
            None,
        );
        controller
            .run_turn(&host, "查网页", "RUN-LBW", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        // The lane-internal web call is refused by the framework_fallback
        // gate (the ONLY refusal — the dispatch-level off gate does not
        // fire under local_browser).
        let refused: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
            })
            .collect();
        assert!(
            refused.iter().any(|e| e.payload["error"]
                == serde_json::json!("retrieval_mode_requires_framework_fallback")),
            "{refused:?}"
        );
        assert!(
            refused.iter().any(|e| {
                e.payload["error"]
                    == serde_json::json!("retrieval_mode_requires_framework_fallback")
                    && e.payload["policy_denial"]["source"] == serde_json::json!("retrieval_mode")
                    && e.payload["policy_denial"]["code"]
                        == serde_json::json!("retrieval_mode_requires_framework_fallback")
            }),
            "framework_fallback refusal must carry the structured denial: {refused:?}"
        );
        // P0-C S3 前置审查修复 (F1): every refusal completion carries the
        // non-zero exit_code required by the Python cross-check.
        for event in &refused {
            assert_eq!(
                event.payload["exit_code"],
                serde_json::json!(1),
                "refusal completion must carry exit_code=1: {event:?}"
            );
            assert_eq!(event.payload["status"], serde_json::json!("error"));
        }
        // The refused lane-internal web call (call-2) has NO ToolStarted —
        // same refusal shape as the other mode gates. (The main-lane
        // dispatch wrapper for call-1 does start — that is the subagent
        // dispatch, not the web execution.)
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-2")
            }),
            "refused web tool must not start"
        );
        assert!(
            !events.iter().any(|e| {
                e.payload.get("error").and_then(|v| v.as_str())
                    == Some("nested_subagent_dispatch_refused")
            }),
            "the web tool must self-execute (and be mode-refused), not be nested-refused"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): the web_fetch
    /// candidate count gate — per-activation accumulation, exact-string
    /// URL dedup (a duplicate consumes no new candidate), mechanical
    /// count feedback on every allowed fetch, and a no-ToolStarted cap
    /// refusal that feeds the denial breaker (no retry space).
    #[tokio::test]
    async fn web_fetch_candidate_gate_counts_dedups_and_rejects_at_cap() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "page content".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-WFC",
            "",
            0,
            None,
            None,
        );
        let call = |i: usize| ToolCall {
            name: "web_fetch".to_string(),
            arguments: serde_json::json!({ "url": format!("https://example.com/{i}") }),
            call_id: format!("call-{i}"),
        };
        // 8 distinct URLs are allowed, each carrying the count feedback.
        for i in 0..8 {
            let (result, feedback) = controller
                .run_host_tool(
                    &host,
                    &mut writer,
                    &call(i),
                    "",
                    orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                    &mut messages,
                    0,
                    None,
                    Some("act-1"),
                    Some(&counter),
                    false,
                    false,
                )
                .await
                .unwrap();
            assert_eq!(result.exit_code, Some(0), "call {i}");
            assert!(
                result
                    .output
                    .contains(&format!("候选 {}/8，剩余 {}", i + 1, 7 - i)),
                "call {i}: {}",
                result.output
            );
            assert!(matches!(feedback, Some(PolicyFeedback::Succeeded)));
        }
        // A duplicate URL consumes no new candidate (still allowed at 8/8).
        let (result, _) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(0),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.output.contains("候选 8/8，剩余 0"),
            "{}",
            result.output
        );
        assert_eq!(counter.lock().unwrap().len(), 8);
        // A NEW URL at the cap is refused — no ToolStarted, Denied key.
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(8),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(
            result.output.contains("候选核验数量已达上限 8"),
            "{}",
            result.output
        );
        assert!(
            matches!(feedback, Some(PolicyFeedback::Denied(_))),
            "cap refusal must feed the denial breaker"
        );
        assert_eq!(counter.lock().unwrap().len(), 8, "refused URL not counted");

        // Journal facts: allowed completions carry count/cap; the refusal
        // has no ToolStarted and the cap-exceeded error at the boundary.
        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_fetch")
            })
            .collect();
        assert_eq!(completed.len(), 10, "8 allowed + duplicate + refusal");
        for (i, e) in completed.iter().enumerate().take(8) {
            assert_eq!(e.payload["candidate_count"], serde_json::json!(i + 1));
            assert_eq!(e.payload["candidate_cap"], serde_json::json!(8));
        }
        let refused_ev = completed
            .iter()
            .find(|e| e.payload["error"] == serde_json::json!("web_fetch_candidate_cap_exceeded"))
            .expect("cap refusal journaled");
        assert_eq!(refused_ev.payload["candidate_count"], serde_json::json!(8));
        assert_eq!(refused_ev.payload["candidate_cap"], serde_json::json!(8));
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-8")
            }),
            "refused web_fetch must not start"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): fail-closed arms of
    /// the web_fetch count gate — a missing `url` argument (no count
    /// identity) and a missing count domain (no per-activation counter)
    /// both refuse without ToolStarted.
    #[tokio::test]
    async fn web_fetch_candidate_gate_fails_closed_on_missing_url_and_domain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: None,
        };
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-WFU",
            "",
            0,
            None,
            None,
        );
        let call = |url: Option<&str>, call_id: &str| ToolCall {
            name: "web_fetch".to_string(),
            arguments: match url {
                Some(u) => serde_json::json!({ "url": u }),
                None => serde_json::json!({ "query": "no url" }),
            },
            call_id: call_id.to_string(),
        };
        // Missing url argument.
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(None, "call-u1"),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(result.output.contains("缺少 url 参数"), "{}", result.output);
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        // Missing count domain (main-lane style direct call).
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(Some("https://example.com/0"), "call-u2"),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                None,
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(
            result.output.contains("候选核验计数域不可用"),
            "{}",
            result.output
        );
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        // Neither refusal started the tool.
        let events = events(&dir);
        for refused_id in ["call-u1", "call-u2"] {
            assert!(
                !events.iter().any(|e| {
                    e.event_type == EventType::ToolStarted
                        && e.payload.get("call_id").and_then(|v| v.as_str()) == Some(refused_id)
                }),
                "{refused_id} must not start"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2/4 (2026-08-14): the builder/override
    /// seam — a shorter cap refuses earlier (production reads
    /// ORZ_WEB_FETCH_CANDIDATE_CAP at construction).
    #[tokio::test]
    async fn web_fetch_candidate_cap_override_shortens_budget() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "page".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_retrieval_enabled(
            AgentLoopController::with_gateway(Arc::new(FakeProvider::new(vec![
                ScriptedResponse::text("x"),
            ])))
            .with_candidate_cap(2),
        );
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-WFO",
            "",
            0,
            None,
            None,
        );
        let call = |i: usize| ToolCall {
            name: "web_fetch".to_string(),
            arguments: serde_json::json!({ "url": format!("https://example.com/{i}") }),
            call_id: format!("call-o{i}"),
        };
        for i in 0..2 {
            let (result, _) = controller
                .run_host_tool(
                    &host,
                    &mut writer,
                    &call(i),
                    "",
                    orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                    &mut messages,
                    0,
                    None,
                    Some("act-1"),
                    Some(&counter),
                    false,
                    false,
                )
                .await
                .unwrap();
            assert_eq!(result.exit_code, Some(0));
            assert!(
                result
                    .output
                    .contains(&format!("候选 {}/2，剩余 {}", i + 1, 1 - i)),
                "{}",
                result.output
            );
        }
        let (result, _) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(2),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(result.output.contains("上限 2"), "{}", result.output);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): the env parse rule —
    /// trimmed positive integer; zero/invalid/absent fall back to the
    /// default (pure function, no env mutation in tests).
    #[test]
    fn web_fetch_candidate_cap_parse_rules() {
        assert_eq!(parse_web_fetch_candidate_cap("8"), Some(8));
        assert_eq!(parse_web_fetch_candidate_cap(" 4 "), Some(4));
        assert_eq!(parse_web_fetch_candidate_cap("0"), None);
        assert_eq!(parse_web_fetch_candidate_cap("-1"), None);
        assert_eq!(parse_web_fetch_candidate_cap("abc"), None);
        assert_eq!(parse_web_fetch_candidate_cap(""), None);
    }

    /// ORZ-CACHE-CONTEXT-COST (2026-08-15): `ORZ_MAX_INJECT_TOKENS_PER_ROUND`
    /// parse rules — trimmed positive integer; invalid/zero → default.
    #[test]
    fn max_inject_tokens_per_round_parse_rules() {
        assert_eq!(parse_max_inject_tokens_per_round("50000"), Some(50_000));
        assert_eq!(parse_max_inject_tokens_per_round(" 1024 "), Some(1024));
        assert_eq!(parse_max_inject_tokens_per_round("0"), None);
        assert_eq!(parse_max_inject_tokens_per_round("-1"), None);
        assert_eq!(parse_max_inject_tokens_per_round("abc"), None);
        assert_eq!(parse_max_inject_tokens_per_round(""), None);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): browser_read joins the
    /// SAME candidate count domain — per-activation accumulation,
    /// exact-string URL dedup (a duplicate consumes no new candidate),
    /// mechanical count feedback on every allowed read, and a no-ToolStarted
    /// cap refusal (`browser_read_candidate_cap_exceeded`) that feeds the
    /// denial breaker.
    #[tokio::test]
    async fn browser_read_candidate_gate_counts_dedups_and_rejects_at_cap() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal: journal.clone(),
            tool_result: Some(ToolResult {
                output: "page content".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let controller = with_local_browser_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&journal),
            EventTrack::V02,
            "RUN-BRC",
            "",
            0,
            None,
            None,
        );
        let call = |i: usize| ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": format!("https://example.com/{i}") }),
            call_id: format!("call-{i}"),
        };
        // 8 distinct URLs are allowed, each carrying the count feedback.
        for i in 0..8 {
            let (result, feedback) = controller
                .run_host_tool(
                    &host,
                    &mut writer,
                    &call(i),
                    "",
                    orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                    &mut messages,
                    0,
                    None,
                    Some("act-1"),
                    Some(&counter),
                    false,
                    false,
                )
                .await
                .unwrap();
            assert_eq!(result.exit_code, Some(0), "call {i}");
            assert!(
                result
                    .output
                    .contains(&format!("候选 {}/8，剩余 {}", i + 1, 7 - i)),
                "call {i}: {}",
                result.output
            );
            assert!(matches!(feedback, Some(PolicyFeedback::Succeeded)));
        }
        // A duplicate URL consumes no new candidate (still allowed at 8/8).
        let (result, _) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(0),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(0));
        assert!(
            result.output.contains("候选 8/8，剩余 0"),
            "{}",
            result.output
        );
        assert_eq!(counter.lock().unwrap().len(), 8);
        // A NEW URL at the cap is refused — no ToolStarted, Denied key.
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(8),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(result.output.contains("上限 8"), "{}", result.output);
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        // Journal: the cap refusal carries count/cap; the refused call has
        // no ToolStarted.
        let events = events(&dir);
        let refused = events
            .iter()
            .find(|e| {
                e.payload["error"] == serde_json::json!("browser_read_candidate_cap_exceeded")
            })
            .expect("cap refusal journaled");
        assert_eq!(refused.payload["candidate_count"], serde_json::json!(8));
        assert_eq!(refused.payload["candidate_cap"], serde_json::json!(8));
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("call_id").and_then(|v| v.as_str()) == Some("call-8")
            }),
            "refused browser_read must not start"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): browser_read's count
    /// gate fails closed on a missing `url` argument (no count identity)
    /// and a missing count domain (no per-activation counter) — both
    /// no-ToolStarted refusals with `browser_read_candidate_*` codes.
    #[tokio::test]
    async fn browser_read_candidate_gate_fails_closed_on_missing_url_and_domain() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let controller = with_local_browser_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&host.journal),
            EventTrack::V02,
            "RUN-BRCF",
            "",
            0,
            None,
            None,
        );
        let call = |args: serde_json::Value, call_id: &str| ToolCall {
            name: "browser_read".to_string(),
            arguments: args,
            call_id: call_id.to_string(),
        };
        // Missing url argument.
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(serde_json::json!({}), "call-u1"),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                Some(&counter),
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(result.output.contains("缺少 url 参数"), "{}", result.output);
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        // Missing count domain (main-lane style direct call).
        let (result, feedback) = controller
            .run_host_tool(
                &host,
                &mut writer,
                &call(
                    serde_json::json!({"url": "https://example.com/0"}),
                    "call-u2",
                ),
                "",
                orz_assurance::gates::ipg::WorkspaceTrust::ObservedTrusted,
                &mut messages,
                0,
                None,
                Some("act-1"),
                None,
                false,
                false,
            )
            .await
            .unwrap();
        assert_eq!(result.exit_code, Some(1));
        assert!(
            result.output.contains("候选核验计数域不可用"),
            "{}",
            result.output
        );
        assert!(matches!(feedback, Some(PolicyFeedback::Denied(_))));
        // Neither refusal started the tool; both carried stable codes.
        let events = events(&dir);
        for (call_id, code) in [
            ("call-u1", "browser_read_candidate_url_missing"),
            ("call-u2", "browser_read_candidate_count_unbound"),
        ] {
            assert!(
                events.iter().any(|e| {
                    e.event_type == EventType::ToolCompleted
                        && e.payload.get("call_id").and_then(|v| v.as_str()) == Some(call_id)
                        && e.payload["error"] == serde_json::json!(code)
                }),
                "{call_id} must journal {code}"
            );
            assert!(
                !events.iter().any(|e| {
                    e.event_type == EventType::ToolStarted
                        && e.payload.get("call_id").and_then(|v| v.as_str()) == Some(call_id)
                }),
                "{call_id} must not start"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 0k 审查处理 (P2-2, 2026-08-30)：candidate_gate 决策+预留原子化——
    /// 锁内检查 cap 并立即占位（消除并行批次下「决策/提交分离」的硬 cap
    /// 竞态：两个调用基于同一旧计数同时通过 → 超限最多 +批次大小）；
    /// `commit_candidate` 对已预留 url 去重幂等；后续 permission/ACAF 门
    /// 拒绝由 `rollback_candidate` 释放占位，保持「被权限/票据拒绝的调用
    /// 不消耗候选」语义（review fix 2026-08-14 不变）。
    #[tokio::test]
    async fn candidate_gate_reserves_atomically_and_rollback_releases() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let controller = with_local_browser_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(vec![ScriptedResponse::text("x")]),
        )));
        let counter: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let mut messages: Vec<Message> = Vec::new();
        let mut writer = EventWriter::new(
            Some(&host.journal),
            EventTrack::V02,
            "RUN-BRGATE",
            "",
            0,
            None,
            None,
        );
        let call = |url: &str| ToolCall {
            name: "browser_read".to_string(),
            arguments: serde_json::json!({ "url": url }),
            call_id: "call-g1".to_string(),
        };
        // Gate 决策即原子预留。
        let decision = controller
            .candidate_gate(
                &mut writer,
                &mut messages,
                &call("https://a.example"),
                Some(&counter),
            )
            .await
            .unwrap();
        assert!(
            matches!(decision, CandidateGateDecision::Allowed { .. }),
            "fresh URL under the cap must be allowed"
        );
        assert_eq!(
            counter.lock().unwrap().len(),
            1,
            "the gate must reserve atomically (check + push under one lock)"
        );
        // Commit consumes with exact-string dedup.
        assert_eq!(commit_candidate(&counter, "https://a.example", 8), (1, 8));
        assert_eq!(
            commit_candidate(&counter, "https://a.example", 8),
            (1, 8),
            "duplicate URL consumes nothing"
        );
        assert_eq!(commit_candidate(&counter, "https://b.example", 8), (2, 8));
        // 后续门拒绝 → 回滚释放占位（不消耗候选）。
        rollback_candidate(&counter, "https://b.example");
        assert_eq!(commit_candidate(&counter, "https://b.example", 8), (2, 8));
        rollback_candidate(&counter, "https://b.example");
        assert_eq!(
            counter.lock().unwrap().len(),
            1,
            "rollback releases the slot"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 4 (2026-08-14): the per-activation
    /// count domain flows through the external retrieval lane for
    /// browser_read — the counter writes back into the activation, and the
    /// journal carries the count/cap on each lane completion.
    #[tokio::test]
    async fn browser_read_candidate_count_accumulates_in_external_lane() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b1", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b2", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b3", "https://b.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_local_browser_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查网页", "RUN-BRL", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // The external activation's counter holds the lane's read URLs
        // (exact-string dedup, first-seen order) — written back after the
        // loop; the main-lane dispatch (call-1) never counted.
        let registry = controller.activations.lock().unwrap();
        let act = registry
            .states
            .get(&SubagentRole::ExternalRetrieval)
            .unwrap();
        assert_eq!(
            act.candidate_urls,
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );

        // Journal: three lane browser_read completions with counts 1, 1, 2
        // (the duplicate consumed no new candidate).
        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("browser_read")
                    && e.payload.get("candidate_count").is_some()
            })
            .collect();
        assert_eq!(completed.len(), 3, "{completed:?}");
        let counts: Vec<i64> = completed
            .iter()
            .map(|e| e.payload["candidate_count"].as_i64().unwrap())
            .collect();
        assert_eq!(counts, vec![1, 1, 2]);
        assert_eq!(completed[0].payload["candidate_cap"], serde_json::json!(8));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4, 审查 P2 收口): auto-close 后每次
    /// 派发都是新激活——browser_read 候选计数按激活隔离（设计 §1.1：只有
    /// activation close 才重置；auto-close 每次派发即重置），第二次派发从
    /// 1/8 重新起算，不再跨派发累积。
    #[tokio::test]
    async fn browser_read_candidate_count_resets_per_auto_closed_dispatch() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-1")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b1", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b2", "https://a.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-2")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b3", "https://b.example")]),
            ScriptedResponse::tool_calls(vec![browser_read_call("call-b4", "https://b.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_local_browser_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查网页", "RUN-BRLC", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // auto-close：第二次派发是新激活（fresh id、revision 0），候选
        // 计数从 1/8 重新起算——registry 里是最新激活（b.example）。
        let registry = controller.activations.lock().unwrap();
        let act = registry
            .states
            .get(&SubagentRole::ExternalRetrieval)
            .unwrap();
        assert_eq!(
            act.activation_id, "retrieval-external_retrieval-RUN-BRLC-01",
            "auto-close 后新派发新建激活"
        );
        assert_eq!(act.contract_revision, 0);
        assert_eq!(act.candidate_urls, vec!["https://b.example".to_string()]);

        // Journal: 4 次 lane browser_read 完成，计数按激活分别 1,1,1,1
        // （不再跨派发累积为 1,1,2,2）。
        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("browser_read")
                    && e.payload.get("candidate_count").is_some()
            })
            .collect();
        assert_eq!(completed.len(), 4, "{completed:?}");
        let counts: Vec<i64> = completed
            .iter()
            .map(|e| e.payload["candidate_count"].as_i64().unwrap())
            .collect();
        assert_eq!(counts, vec![1, 1, 1, 1]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 2 (2026-08-14): the per-activation
    /// count domain flows through the external retrieval lane — only the
    /// lane's SELF-EXECUTED web_fetch calls count (the main-lane dispatch
    /// wrapper is not a fetch), the counter writes back into the activation,
    /// and the journal carries the count/cap on each completion.
    #[tokio::test]
    async fn web_fetch_candidate_count_accumulates_in_external_lane() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-1", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-2", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-3", "https://b.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查网页", "RUN-WF2", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // The external activation's counter holds the lane's fetched URLs
        // (exact-string dedup, first-seen order) — written back after the
        // loop; the main-lane dispatch (call-1) never counted.
        let registry = controller.activations.lock().unwrap();
        let act = registry
            .states
            .get(&SubagentRole::ExternalRetrieval)
            .unwrap();
        assert_eq!(
            act.candidate_urls,
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );

        // Journal: two lane web_fetch completions with counts 1 and 2.
        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_fetch")
                    // The main-lane dispatch wrapper's completion (call-1)
                    // carries no count — it is not a fetch; only the
                    // lane's self-executed fetches annotate candidate_count.
                    && e.payload.get("candidate_count").is_some()
            })
            .collect();
        assert_eq!(completed.len(), 2, "{completed:?}");
        assert_eq!(
            completed[0].payload["candidate_count"],
            serde_json::json!(1)
        );
        assert_eq!(
            completed[1].payload["candidate_count"],
            serde_json::json!(2)
        );
        assert_eq!(completed[1].payload["candidate_cap"], serde_json::json!(8));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4, 审查 P2 收口): auto-close 后每次
    /// 派发都是新激活——web_fetch 候选计数按激活隔离，第二次派发从 1/8
    /// 重新起算，不再跨派发累积。
    #[tokio::test]
    async fn web_fetch_candidate_count_resets_per_auto_closed_dispatch() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-1", "https://a.example")]),
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-2", "https://a.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-3", "https://b.example")]),
            ScriptedResponse::tool_calls(vec![web_fetch_call("call-4", "https://b.example")]),
            ScriptedResponse::text("检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查网页", "RUN-WFC2", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        // auto-close：第二次派发是新激活（fresh id、revision 0），候选
        // 计数从 1/8 重新起算——registry 里是最新激活（b.example）。
        let registry = controller.activations.lock().unwrap();
        let act = registry
            .states
            .get(&SubagentRole::ExternalRetrieval)
            .unwrap();
        assert_eq!(
            act.activation_id, "retrieval-external_retrieval-RUN-WFC2-01",
            "auto-close 后新派发新建激活"
        );
        assert_eq!(act.contract_revision, 0);
        assert_eq!(act.candidate_urls, vec!["https://b.example".to_string()]);

        // Journal: 两条 lane web_fetch 完成，计数按激活分别为 1,1
        // （dispatch 包装 call-1/call-3 不计数；不再跨派发累积为 1,2）。
        let events = events(&dir);
        let completed: Vec<&RunEvent> = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("web_fetch")
                    && e.payload.get("candidate_count").is_some()
            })
            .collect();
        assert_eq!(completed.len(), 2, "{completed:?}");
        assert_eq!(
            completed[0].payload["candidate_count"],
            serde_json::json!(1)
        );
        assert_eq!(
            completed[1].payload["candidate_count"],
            serde_json::json!(1)
        );
        assert_eq!(completed[1].payload["candidate_cap"], serde_json::json!(8));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4): 每次调用即闭环——每个派发新建
    /// 激活（fresh activation_id / contract_revision 0 / 新预算与新候选
    /// 计数），auto_close 后不再复用；黑板分区按 P2-3 用户裁决全量覆盖
    /// （response / entries / ledger 替换，分区代表最近一次派发）。
    #[tokio::test]
    async fn subagent_dispatch_creates_fresh_activation_per_call() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] a.md\n第一批"),
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::text("[DOC] b.md\n第二批"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        // R1 (§4.6): 生产默认阈值 50；这些场景固定 7 保持原触发语义。
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-abcdef123456", 7);
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-ID",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let assessments: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .collect();
        assert_eq!(assessments.len(), 2);
        let p0 = &assessments[0].payload;
        let p1 = &assessments[1].payload;
        // session8 = "sess-abc" — the first 8 chars of the session id.
        assert_eq!(
            p0.get("activation_id").and_then(|v| v.as_str()),
            Some("retrieval-internal_retrieval-sess-abc-00")
        );
        assert_eq!(
            p1.get("activation_id").and_then(|v| v.as_str()),
            Some("retrieval-internal_retrieval-sess-abc-01"),
            "auto-close 后新派发新建激活"
        );
        assert_eq!(
            p0.get("contract_id").and_then(|v| v.as_str()),
            Some("retrieval-contract-internal_retrieval")
        );
        // auto-close：两次派发均 revision 0（无 continue 递增）。
        assert_eq!(p0.get("contract_revision"), Some(&serde_json::json!(0)));
        assert_eq!(p1.get("contract_revision"), Some(&serde_json::json!(0)));
        // 每条派发一条 auto_close close record（每激活一条，verifier
        // 同 activation 单 close 规则满足）。
        let closes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
            .collect();
        assert_eq!(closes.len(), 2, "{:?}", event_types(&dir));
        assert!(closes.iter().all(|c| {
            c.payload.get("terminal_reason").and_then(|v| v.as_str()) == Some("auto_close")
        }));
        // 无 disposition 事件（close/continue 往返已退役）。
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::RetrievalParentDisposition)
                .count(),
            0
        );
        // 黑板分区覆盖语义（P2-3 用户裁决选 a）——最近一次派发替换。
        let r = controller.blackboard().read();
        assert_eq!(r.internal_ret.project_docs, vec!["b.md"]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：委托
    /// 契约复杂度分档——close record 登记 effort（短内部查询 → standard；
    /// 长聚合词查询 → deep）；预算解析在 controller 字段未显式设置时走
    /// 档位默认。
    #[tokio::test]
    async fn close_record_carries_effort_tier() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let long_query = format!("调研{}", "x".repeat(900));
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "retrieve_project_docs".to_string(),
                arguments: serde_json::json!({ "query": "查找项目文档" }),
                call_id: "call-1".to_string(),
            }]),
            ScriptedResponse::text("[DOC] a.md\n检索完成"),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "retrieve_project_docs".to_string(),
                arguments: serde_json::json!({ "query": long_query }),
                call_id: "call-2".to_string(),
            }]),
            ScriptedResponse::text("[DOC] b.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "分档测试", "RUN-EFF", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let closes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
            .collect();
        assert_eq!(closes.len(), 2, "{:?}", event_types(&dir));
        assert_eq!(
            closes[0].payload.get("effort").and_then(|v| v.as_str()),
            Some("standard"),
            "{:?}",
            closes[0].payload
        );
        assert_eq!(
            closes[1].payload.get("effort").and_then(|v| v.as_str()),
            Some("deep"),
            "{:?}",
            closes[1].payload
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn disposition_call(
        role: &str,
        decision: &str,
        delta: Option<&str>,
        call_id: &str,
    ) -> ToolCall {
        let mut arguments = serde_json::json!({
            "role": role,
            "decision": decision,
        });
        if let Some(d) = delta {
            arguments["requirement_delta"] = serde_json::Value::String(d.to_string());
        }
        ToolCall {
            name: "retrieval_disposition".to_string(),
            arguments,
            call_id: call_id.to_string(),
        }
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4): 派发成功即 auto_close close
    /// record（assessment + result_digest 绑定，terminal_reason
    /// auto_close）；对已关闭激活的迟到 retrieval_disposition 调用一律
    /// 机械拒绝（no_pending_assessment，零 disposition 事件、零二次
    /// close）。
    #[tokio::test]
    async fn dispatch_auto_closes_and_late_disposition_refused() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] a.md\n检索完成"),
            ScriptedResponse::tool_calls(vec![disposition_call(
                "internal_retrieval",
                "close",
                None,
                "call-d1",
            )]),
            ScriptedResponse::tool_calls(vec![disposition_call(
                "internal_retrieval",
                "close",
                None,
                "call-d2",
            )]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-abcdef123456", 7);
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-CLOSE",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        // 无 disposition 事件（close/continue 往返已退役）。
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::RetrievalParentDisposition)
                .count(),
            0
        );
        // 一条 auto_close close record（assessment + digest 绑定）。
        let closes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
            .collect();
        assert_eq!(closes.len(), 1);
        let c = &closes[0].payload;
        assert_eq!(
            c.get("terminal_reason").and_then(|v| v.as_str()),
            Some("auto_close")
        );
        assert_eq!(
            c.get("validated_disposition_id"),
            Some(&serde_json::Value::Null),
            "auto_close 不引用 disposition"
        );
        assert!(c.get("assessment_id").is_some());
        let digest = c.get("result_digest").and_then(|v| v.as_str()).unwrap();
        assert_eq!(digest.len(), 64, "64-hex sha256 for auto_close");
        assert_eq!(c.get("contract_revision"), Some(&serde_json::json!(0)));
        // 迟到的两次 disposition 调用被拒（no_pending_assessment）。
        assert_eq!(
            events
                .iter()
                .filter(|e| {
                    e.event_type == EventType::ToolCompleted
                        && e.payload.get("error").and_then(|v| v.as_str())
                            == Some("no_pending_assessment")
                })
                .count(),
            2
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4): auto-close 后再次派发 = 新激活
    /// （fresh activation_id、revision 0）；夹在中间的 retrieval_disposition
    /// 调用被机械拒绝（no_pending_assessment），不产生 disposition 事件、
    /// 不干扰两次 auto_close。
    #[tokio::test]
    async fn second_dispatch_after_auto_close_starts_fresh() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] a.md\n检索完成"),
            ScriptedResponse::tool_calls(vec![disposition_call(
                "internal_retrieval",
                "close",
                None,
                "call-d1",
            )]),
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::text("[DOC] b.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-REPLAY",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        // 零 disposition 事件；一次 no_pending_assessment 拒绝。
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::RetrievalParentDisposition)
                .count(),
            0
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| {
                    e.event_type == EventType::ToolCompleted
                        && e.payload.get("error").and_then(|v| v.as_str())
                            == Some("no_pending_assessment")
                })
                .count(),
            1
        );
        // 两次派发 = 两个新激活 + 两条 auto_close close record。
        let assessments: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .collect();
        assert_eq!(assessments.len(), 2);
        assert_eq!(
            assessments[0]
                .payload
                .get("activation_id")
                .and_then(|v| v.as_str()),
            Some("retrieval-internal_retrieval-RUN-REPL-00"),
            "无 orientation 时激活身份取 run_id 前 8 字符"
        );
        assert_eq!(
            assessments[1]
                .payload
                .get("activation_id")
                .and_then(|v| v.as_str()),
            Some("retrieval-internal_retrieval-RUN-REPL-01")
        );
        assert_eq!(
            assessments[0].payload.get("contract_revision"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            assessments[1].payload.get("contract_revision"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
                .count(),
            2
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4): auto-close 后连续两次检索派发
    /// 都成功——第二次不再被"awaiting disposition"拒（该状态已被每次调用
    /// 即闭环取代），产生两条 assessment 与两条 auto_close close record。
    #[tokio::test]
    async fn consecutive_dispatches_both_auto_close() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text("[DOC] a.md\n检索完成"),
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::text("[DOC] b.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-AWAIT",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::InformationSufficiencyAssessment)
                .count(),
            2,
            "both dispatches formed results: {:?}",
            event_types(&dir)
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
                .count(),
            2,
            "both dispatches auto-closed: {:?}",
            event_types(&dir)
        );
        // 无 awaiting-disposition 拒绝（该拒绝路径已不可达）。
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("activation_awaiting_disposition")
            }),
            "awaiting-disposition refusal must not fire under auto-close"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// M4: a user cancel closes every pending activation with a terminal
    /// close record BEFORE the run_cancelled terminal event.
    #[tokio::test]
    async fn user_cancel_closes_pending_activations_before_run_cancelled() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };

        // Slow-chunked subagent response — the cancel lands inside the
        // subagent stream (→ subagent_cancelled close), then propagates to
        // the run level (→ run_cancelled). The subagent_cancelled close
        // covers the activation; the user-cancel pass skips it (idempotent).
        let gateway: Arc<dyn ModelGateway> = Arc::new(
            FakeProvider::new(vec![
                ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
                ScriptedResponse::text("[DOC] a.md\n检索完成"),
            ])
            .with_chunk_delay(std::time::Duration::from_millis(60)),
        );
        let controller = Arc::new(with_retrieval_enabled(AgentLoopController::with_gateway(
            gateway,
        )));
        let token = tokio_util::sync::CancellationToken::new();

        let c = controller.clone();
        let t = token.clone();
        let run = tokio::task::spawn(async move {
            c.run_turn_with_cancel(
                &host,
                "查找项目文档",
                "RUN-UC",
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

        let events = events(&dir);
        let close_reasons: Vec<Option<String>> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
            .map(|e| {
                e.payload
                    .get("terminal_reason")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .collect();
        // The cancel may land before or during the subagent round — either
        // way exactly ONE close per activation, with a cancel-family reason.
        assert_eq!(close_reasons.len(), 1, "{close_reasons:?}");
        assert!(
            close_reasons[0].as_deref() == Some("user_cancelled")
                || close_reasons[0].as_deref() == Some("subagent_cancelled"),
            "{close_reasons:?}"
        );
        // The close precedes the run terminal.
        let close_idx = events
            .iter()
            .position(|e| e.event_type == EventType::RetrievalCloseRecord)
            .unwrap();
        let cancel_idx = events
            .iter()
            .position(|e| e.event_type == EventType::RunCancelled)
            .unwrap();
        assert!(close_idx < cancel_idx, "close before run_cancelled");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// M4: budget exhaustion on the subagent loop is a terminal authority —
    /// a partial result is assessed, then the activation closes with
    /// `budget_exhausted` (assessment + digest bound, no disposition).
    #[tokio::test]
    async fn subagent_budget_exhaustion_closes_activation() {
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

        // Subagent budget 2: three subagent tool rounds exhaust it, then a
        // final no-tool round reports the partial result.
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-2")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-3")]),
            ScriptedResponse::text("[DOC] a.md\n部分结果"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway))
            .with_max_tool_rounds(2);
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-BUD",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        // Assessment precedes the close (verifier ordering).
        let assessment_idx = events
            .iter()
            .position(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .unwrap();
        let close = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalCloseRecord)
            .expect("budget close record");
        assert_eq!(
            close
                .payload
                .get("terminal_reason")
                .and_then(|v| v.as_str()),
            Some("budget_exhausted")
        );
        assert!(close.payload.get("assessment_id").is_some());
        assert_eq!(
            close.payload.get("validated_disposition_id"),
            Some(&serde_json::Value::Null)
        );
        let close_idx = events
            .iter()
            .position(|e| e.event_type == EventType::RetrievalCloseRecord)
            .unwrap();
        assert!(assessment_idx < close_idx, "assessment before close");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN R1 (§4.4): auto-close 后每次派发 = 新激活 =
    /// 新预算（F5 的 continue 累积语义随 disposition 往返一并退役）。
    /// 两次派发各耗 2 个子代理工具轮（< 4），都不触顶、都形成结果并
    /// auto_close——验证"新激活从 0 起算"（若沿用旧累积语义，第二次会
    /// 在累计 4 轮时 budget_exhausted）。
    #[tokio::test]
    async fn subagent_budget_is_fresh_per_dispatch() {
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

        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s2")]),
            ScriptedResponse::text("[DOC] a.md\n第一批"),
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-2")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s3")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s4")]),
            ScriptedResponse::text("[DOC] b.md\n第二批"),
            ScriptedResponse::text("完成"),
            // The main's final answer crosses the counterexample gate —
            // one extra model round answers it (run-semantic; the
            // subagent lanes never fire the gate).
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway))
            .with_max_tool_rounds(4);
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-BUDACC",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        // 新预算语义：两次派发均 < 4，无 tool_rounds_limit 门触发。
        assert!(
            !events.iter().any(|e| {
                e.event_type == EventType::GateDecision
                    && e.payload.get("gate").and_then(|v| v.as_str()) == Some("tool_rounds_limit")
            }),
            "fresh per-dispatch budget must not trip the limit: {:?}",
            event_types(&dir)
        );
        // 两次派发均形成结果——revision 0（无 continue 递增）。
        let assessments: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .collect();
        assert_eq!(assessments.len(), 2, "{:?}", event_types(&dir));
        assert_eq!(
            assessments[0].payload.get("contract_revision"),
            Some(&serde_json::json!(0))
        );
        assert_eq!(
            assessments[1].payload.get("contract_revision"),
            Some(&serde_json::json!(0))
        );
        // 两条 auto_close close record。
        let closes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalCloseRecord)
            .collect();
        assert_eq!(closes.len(), 2, "{:?}", event_types(&dir));
        assert!(closes.iter().all(|c| {
            c.payload.get("terminal_reason").and_then(|v| v.as_str()) == Some("auto_close")
        }));
        // 两次派发各执行 2 个 read_file（共 4 个，均成功）。
        assert_eq!(
            events
                .iter()
                .filter(|e| {
                    e.event_type == EventType::ToolCompleted
                        && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")
                        && e.payload.get("exit_code") == Some(&serde_json::json!(0))
                })
                .count(),
            4
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：子代理墙钟
    /// 解析规则——`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS`（0 禁用）、
    /// `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS`（0 禁用独立上限、非法忽略）。
    #[test]
    fn retrieval_subagent_budget_parse_rules() {
        assert_eq!(
            parse_retrieval_subagent_wallclock("600"),
            Some(Some(std::time::Duration::from_secs(600)))
        );
        assert_eq!(
            parse_retrieval_subagent_wallclock(" 30 "),
            Some(Some(std::time::Duration::from_secs(30)))
        );
        assert_eq!(parse_retrieval_subagent_wallclock("0"), Some(None));
        assert_eq!(parse_retrieval_subagent_wallclock("abc"), None);
        assert_eq!(
            parse_retrieval_subagent_max_tool_rounds("60"),
            Some(Some(60))
        );
        assert_eq!(parse_retrieval_subagent_max_tool_rounds("0"), Some(None));
        assert_eq!(parse_retrieval_subagent_max_tool_rounds("x"), None);
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：测试网关——
    /// 脚本弹尽即挂起（`std::future::pending`），用于子代理墙钟超时收口。
    struct ScriptThenHangGateway {
        script: std::sync::Arc<Mutex<std::collections::VecDeque<ScriptedResponse>>>,
    }

    #[async_trait]
    impl ModelGateway for ScriptThenHangGateway {
        async fn generate(&self, request: ModelRequest) -> Result<ModelResponse, GatewayError> {
            let next = {
                let mut guard = self.script.lock().unwrap();
                guard.pop_front()
            };
            match next {
                Some(response) => FakeProvider::new(vec![response]).generate(request).await,
                None => std::future::pending().await,
            }
        }

        fn for_new_run(&self) -> Arc<dyn ModelGateway> {
            Arc::new(ScriptThenHangGateway {
                script: self.script.clone(),
            })
        }
    }

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：单次检索派发
    /// 墙钟预算耗尽 → 子代理 loop 被丢弃、激活以 `subagent_timeout` 收口、
    /// 主 run 继续正常结束。
    #[tokio::test]
    async fn subagent_wallclock_timeout_closes_activation_and_main_continues() {
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
        // 主 #1 派发；子代理 #2 一个 read_file 工具轮；子代理 #3 挂起
        // （脚本弹尽）→ 墙钟（150ms）触发；主 #4/#5 完成（counterexample
        // 门需一次额外模型轮）。
        let script: std::collections::VecDeque<ScriptedResponse> = vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-s1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]
        .into();
        let gateway: Arc<dyn ModelGateway> = Arc::new(ScriptThenHangGateway {
            script: std::sync::Arc::new(Mutex::new(script)),
        });
        let mut controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller.retrieval_subagent_wallclock = Some(std::time::Duration::from_millis(150));
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-RTO",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let close = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalCloseRecord)
            .expect("timeout close record");
        assert_eq!(
            close
                .payload
                .get("terminal_reason")
                .and_then(|v| v.as_str()),
            Some("subagent_timeout"),
            "{:?}",
            event_types(&dir)
        );
        // 主车道派发错误恰 1 次（error 精确匹配——旧断言 contains("wallclock")
        // 会把主车道错误与合成收口混计，2026-08-31 审查处理 N3 修正）。
        let main_failed = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("retrieval subagent wallclock exceeded")
            })
            .count();
        assert_eq!(main_failed, 1, "{:?}", event_types(&dir));
        // 子代理侧合成收口（在途工具中断）至多 1 次：是否命中取决于超时
        // 瞬间子代理是否仍有在途工具，属时序相关合法形态，不作硬断言
        // （0 或 1 均正确——read_file 在 150 ms 内完成则无在途工具）。
        let mid_tool = events
            .iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("status").and_then(|v| v.as_str()) == Some("error")
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("subagent_wallclock_timeout_mid_tool")
            })
            .count();
        assert!(mid_tool <= 1, "{:?}", event_types(&dir));
        // 主 run 正常终止（run_finished，非 run_failed）。
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EventType::RunFinished),
            "{:?}",
            event_types(&dir)
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── M5 (2026-08-10): orientation lane feeding + Diagnostic Coverage ────

    /// The subagent's completed model rounds feed the internal lane — 7
    /// rounds cross the threshold and fire an `orientation_checkpoint` with
    /// `agent_role=internal_retrieval`, injected into the SUBAGENT
    /// conversation (ADR-0010 §4.2).
    #[tokio::test]
    async fn subagent_lane_feeds_and_fires_orientation() {
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

        // The subagent runs 7 read_file tool rounds (7 feeds), then a text
        // round forms the result; the main wraps it with 3 rounds total.
        let mut script = vec![ScriptedResponse::tool_calls(vec![tool_call(
            "retrieve_project_docs",
            "call-1",
        )])];
        for i in 0..7 {
            script.push(ScriptedResponse::tool_calls(vec![tool_call(
                "read_file",
                &format!("call-s{i}"),
            )]));
        }
        script.push(ScriptedResponse::text("[DOC] doc.md\n检索完成"));
        script.push(ScriptedResponse::text("完成"));
        script.push(ScriptedResponse::text("完成"));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(Arc::new(
            FakeProvider::new(script),
        )));
        let mut orientation =
            crate::orientation::OrientationSessionState::new_with_threshold("sess-lane1234567", 7);
        controller
            .run_turn(
                &host,
                "查找项目文档",
                "RUN-LANE",
                MANIFEST,
                0,
                None,
                Some(&mut orientation),
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let fires: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::OrientationCheckpoint)
            .collect();
        assert_eq!(fires.len(), 1, "{:?}", event_types(&dir));
        let p = &fires[0].payload;
        assert_eq!(
            p.get("agent_role").and_then(|v| v.as_str()),
            Some("internal_retrieval")
        );
        assert_eq!(
            p.get("trigger").and_then(|v| v.as_str()),
            Some("completed_turns_interval")
        );
        assert_eq!(
            p.get("completed_turns_since_orientation"),
            Some(&serde_json::json!(7))
        );
        assert_eq!(
            p.get("injection_position").and_then(|v| v.as_str()),
            Some("post_tool_batch_gap")
        );
        assert!(
            p.get("message_block")
                .and_then(|v| v.as_str())
                .unwrap()
                .starts_with("[ORIENTATION")
        );
        // Lanes count independently: the main's 3 rounds never fed the
        // subagent lane and vice versa. The fire COMMIT reset the internal
        // lane (7 → 0); the 8th (result-forming) round re-fed it to 1.
        assert_eq!(orientation.internal.completed_rounds, 1);
        assert_eq!(orientation.main.completed_rounds, 3);
        assert_eq!(orientation.external.completed_rounds, 0);
        // §14.16 检索车道不变: the subagent lane keeps the legacy
        // fire-and-continue behavior — no forced template round, no
        // `checkpoint_response` event.
        assert!(
            events
                .iter()
                .all(|e| e.event_type != EventType::CheckpointResponse),
            "retrieval lanes must not produce checkpoint_response events"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 设计 §2.3/§2.7/§5 验收 3（派发面）：主车道调用 web_search → relay
    /// 派发检索子代理（ToolStarted target=external_retrieval），子代理车道
    /// 自行执行检索工具（TestHost NotFound 证明到达 host 工具集）。
    #[tokio::test]
    async fn main_lane_web_search_dispatches_to_retrieval_subagent() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            // 主车道第 1 轮：直接调 web_search。
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-w1")]),
            // 子代理车道第 1 轮：自行执行 web_search（lane self-execution）。
            ScriptedResponse::tool_calls(vec![tool_call("web_search", "call-w2")]),
            // 子代理车道第 2 轮：形成检索结果并关闭。
            ScriptedResponse::text("检索完成"),
            // 主车道：终答前反例自查轮 + 终答。
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        let (response, _, _) = controller
            .run_turn(
                &host,
                "查一下",
                "RUN-RETRIEVAL-MAIN",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response, "终答");

        let events = events(&dir);
        // 主车道 web_search 派发到 external_retrieval（relay 路由）。
        let dispatch = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str)
                        == Some("web_search")
                    && e.payload.get("target").and_then(serde_json::Value::as_str)
                        == Some("external_retrieval")
            })
            .expect("web_search dispatched to the external retrieval subagent");
        assert_eq!(dispatch.payload["call_id"], serde_json::json!("call-w1"));
        // 子代理车道实际执行了 web_search（TestHost NotFound 证明到达
        // host 工具集；车道自执行事件不带 target 字段）。
        let subagent_call = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str)
                        == Some("web_search")
                    && e.payload.get("call_id").and_then(serde_json::Value::as_str)
                        == Some("call-w2")
            })
            .expect("subagent lane self-executes web_search through the host");
        let err = subagent_call
            .payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .expect("host failure");
        assert!(
            err.contains("test host has no tool result"),
            "web_search reached the host toolset: {err}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// RETRIEVAL-SUBAGENT-WIRING (2026-08-25, ADR-0010 §14.40)：主车道
    /// 调 `retrieve_project_docs` → relay 路由到 internal_retrieval
    /// 子代理（内部 lane 工具面仅读族）；子代理多轮检索后返回
    /// `[DOC]` 结构化结果。
    #[tokio::test]
    async fn main_lane_retrieve_project_docs_dispatches_to_internal_subagent() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            // 主车道第 1 轮：直接调 retrieve_project_docs。
            // 审查处理：query 必填 + 可选 scope/max_results 一并进入契约。
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "retrieve_project_docs".to_string(),
                arguments: serde_json::json!({
                    "query": "调研缓存层",
                    "scope": "src/cache",
                    "max_results": 3,
                }),
                call_id: "call-rpd-1".to_string(),
            }]),
            // 子代理车道第 1 轮：调用读族工具（read_file）做检索。
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-rpd-2")]),
            // 子代理车道第 2 轮：形成结构化结果并关闭。
            ScriptedResponse::text("[DOC] src/cache.rs 缓存回归定位"),
            // 主车道：终答前反例自查轮 + 终答。
            ScriptedResponse::text("草稿"),
            ScriptedResponse::text("终答"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        let (response, _, _) = controller
            .run_turn(
                &host,
                "调研项目文档",
                "RUN-RETRIEVAL-INTERNAL",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response, "终答");

        // 审查处理：scope/max_results 机械并入子代理任务契约（子代理只收
        // goal 文本，声明面承诺的参数必须进契约）。
        let received = fake.received_requests();
        let subagent_req = received
            .iter()
            .find(|r| r.system.contains("调研缓存层"))
            .expect("internal subagent receives the folded retrieval task goal");
        assert!(
            subagent_req.system.contains("scope: src/cache"),
            "scope folded into the subagent contract: {subagent_req:?}"
        );
        assert!(
            subagent_req.system.contains("max_results: 3"),
            "max_results folded into the subagent contract: {subagent_req:?}"
        );

        let events = events(&dir);
        // 主车道 retrieve_project_docs 派发到 internal_retrieval（relay 路由）。
        let dispatch = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str)
                        == Some("retrieve_project_docs")
                    && e.payload.get("target").and_then(serde_json::Value::as_str)
                        == Some("internal_retrieval")
            })
            .expect("retrieve_project_docs dispatched to the internal retrieval subagent");
        assert_eq!(dispatch.payload["call_id"], serde_json::json!("call-rpd-1"));
        // 子代理车道实际执行了读族工具（TestHost NotFound 证明到达 host
        // 工具集；内部 lane 不含 web 族/browser_read）。
        let subagent_call = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str)
                        == Some("read_file")
                    && e.payload.get("call_id").and_then(serde_json::Value::as_str)
                        == Some("call-rpd-2")
            })
            .expect("internal lane executes read-family tool through the host");
        let err = subagent_call
            .payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .expect("host failure");
        assert!(
            err.contains("test host has no tool result"),
            "read_file reached the host toolset: {err}"
        );
        // 内部 lane 事件面：无 web_search / web_fetch / browser_read 调用。
        assert!(
            !events.iter().any(|e| {
                (e.event_type == EventType::ToolStarted || e.event_type == EventType::ToolCompleted)
                    && e.payload
                        .get("tool")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|t| {
                            crate::relay::is_web_retrieval_tool(t) || t == "browser_read"
                        })
            }),
            "internal lane must not touch web/browser tools: {events:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
