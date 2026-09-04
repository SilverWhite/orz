//! Retrieval disposition judgment + close chain — batch B4 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::acaf::TicketKind;
use orz_assurance::{EventType, canonical_json, sha256_hex};

use crate::agents::SubagentRole;
use crate::blackboard::ToolActionRecord;
use crate::controller::{
    AgentLoopController, AgentLoopError, EventWriter, PendingDisposition, TicketGate,
    chrono_utc_now,
};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::ToolResult;
use crate::retrieval::activation::{ActivationState, ActivationStatus};

/// §4.4 judgment outcome — the `outcome` field of the disposition event
/// (replay idempotency is handled before the judgment — the original
/// payload is re-journaled and the handler returns early).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DispositionVerdict {
    AcceptedClose,
    AcceptedContinue,
    RejectedStale,
    RejectedConflicting,
}

/// M4 (GAP-SUBAGENT-RUNTIME 2026-08-10): the `retrieval_parent_disposition`
/// event payload (ADR-0010 §5.2 — disposition/parent/subagent/activation/
/// assessment identity + expected revision + decision + delta +
/// capability-gate + mechanical outcome). `capability_gate` is
/// `not_applicable` until the capability-receipt slice lands (scope
/// expansion re-runs the task-contract gate there).
fn disposition_payload(
    disposition_id: &str,
    act: &ActivationState,
    pending: &PendingDisposition,
    decision: &str,
    requirement_delta: &Option<String>,
    outcome: &str,
) -> serde_json::Value {
    serde_json::json!({
        "disposition_id": disposition_id,
        "parent_session_id": act.parent_session_id,
        "subagent_session_id": act.subagent_session_id,
        "activation_id": act.activation_id,
        "assessment_id": pending.assessment_id,
        "expected_contract_revision": pending.expected_contract_revision,
        "decision": decision,
        "requirement_delta": if decision == "continue" {
            requirement_delta.clone()
        } else {
            None
        },
        "capability_gate": "not_applicable",
        "outcome": outcome,
    })
}

impl AgentLoopController {
    /// M4 (GAP-SUBAGENT-RUNTIME 2026-08-10): the parent's structured
    /// disposition control tool handler (ADR-0010 §4.4). The proposal is
    /// judged MECHANICALLY — mirroring the §4.4 verifier rules in priority
    /// order (replay idempotency → stale revision → conflicting decision →
    /// accepted) — and journaled as `retrieval_parent_disposition` with the
    /// mechanical `outcome`. Accepted close commits the close record in the
    /// same handler (controller single writer — close commit and state
    /// switch are one commit, §4.4); accepted continue increments the
    /// contract revision and keeps the activation active.
    pub(crate) async fn handle_parent_disposition(
        &self,
        writer: &mut EventWriter<'_>,
        tc: &ToolCall,
        messages: &mut Vec<Message>,
    ) -> Result<ToolResult, AgentLoopError> {
        let role = match tc.arguments.get("role").and_then(|v| v.as_str()) {
            Some("internal_retrieval") => SubagentRole::InternalRetrieval,
            Some("external_retrieval") => SubagentRole::ExternalRetrieval,
            _ => {
                let msg = "retrieval_disposition: unknown `role` — use \
                           internal_retrieval or external_retrieval"
                    .to_string();
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                return Ok(ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                });
            }
        };
        let decision = match tc.arguments.get("decision").and_then(|v| v.as_str()) {
            Some("close") | Some("continue") => tc
                .arguments
                .get("decision")
                .and_then(|v| v.as_str())
                .unwrap()
                .to_string(),
            _ => {
                let msg = "retrieval_disposition: `decision` must be close or continue".to_string();
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                return Ok(ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                });
            }
        };
        let requirement_delta = tc
            .arguments
            .get("requirement_delta")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        if decision == "continue"
            && requirement_delta
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            let msg = "retrieval_disposition: continue requires a non-empty \
                       requirement_delta"
                .to_string();
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }
        if decision == "close" && requirement_delta.is_some() {
            let msg = "retrieval_disposition: close cannot carry requirement_delta".to_string();
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }

        // The control call is an executed tool — journal it like any other
        // (audit discipline: a declared tool's round is visible in the
        // chain).
        writer
            .record(
                EventType::ToolStarted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                }),
            )
            .await?;

        // Take the activation — the temporary guard drops at the end of the
        // let statement, so no guard ever crosses an await (the state is
        // re-inserted on every path below).
        let act = self.activations.lock().unwrap().states.remove(&role);
        let mut act = match act {
            Some(a) => a,
            None => {
                let msg = format!(
                    "retrieval_disposition: no activation for role '{}'",
                    role.as_str()
                );
                writer
                    .record(
                        EventType::ToolCompleted,
                        serde_json::json!({
                            "tool": tc.name,
                            "call_id": tc.call_id,
                            "status": "error",
                            "error": "no_activation",
                        }),
                    )
                    .await?;
                messages.push(Message {
                    role: Role::Tool,
                    content: msg.clone(),
                    tool_call_id: Some(tc.call_id.clone()),
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                    round: None,
                });
                return Ok(ToolResult {
                    output: msg,
                    exit_code: Some(1),
                    output_encoding: None,
                    structured: None,
                    ..Default::default()
                });
            }
        };

        // ── replay idempotency FIRST (the id derives from the PRE-OUTCOME
        // proposal ONLY — a retried call yields the same id regardless of
        // activation state; the outcome is the judgment's product, never
        // part of the id). A replayed call re-journals the ORIGINAL payload
        // byte-identically and moves no state — recognized even after the
        // activation closed (pending cleared), which is exactly when a
        // retry would arrive.
        let proposal_canonical = canonical_json(&serde_json::json!({
            "role": role.as_str(),
            "decision": decision,
            "requirement_delta": requirement_delta,
        }))
        .unwrap_or_default();
        let disposition_id = format!(
            "DISP-{}-{}",
            &sha256_hex(&proposal_canonical)[..16],
            &sha256_hex(tc.call_id.as_bytes())[..8],
        );
        if let Some((_, original_canonical)) =
            act.submitted.iter().find(|(id, _)| id == &disposition_id)
        {
            let original: serde_json::Value =
                serde_json::from_slice(original_canonical).unwrap_or(serde_json::Value::Null);
            writer
                .record(EventType::RetrievalParentDisposition, original)
                .await?;
            let output = format!("[{disposition_id}] replayed_idempotent — already recorded");
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "exit_code": 0,
                    }),
                )
                .await?;
            self.activations.lock().unwrap().states.insert(role, act);
            messages.push(Message {
                role: Role::Tool,
                content: output.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            return Ok(ToolResult {
                output,
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }

        // §4.4: the disposition binds an assessment — an activation without
        // a pending one (Active mid-task / Closed) refuses.
        if act.pending.is_none() {
            let msg = format!(
                "retrieval_disposition: activation {} has no pending \
                 assessment to dispose",
                act.activation_id,
            );
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "status": "error",
                        "error": "no_pending_assessment",
                    }),
                )
                .await?;
            self.activations.lock().unwrap().states.insert(role, act);
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }
        let pending = act.pending.as_ref().unwrap().clone();

        // ── judgment (mirrors the §4.4 verifier; priority order — replay
        // was handled above, before the pending check) ─────────────────────
        let (verdict, payload): (DispositionVerdict, serde_json::Value) = {
            if pending.expected_contract_revision != act.contract_revision {
                // 2. Stale — a late close after a continue, or a disposition
                //    on a superseded revision (§4.4 rejected_stale).
                let payload = disposition_payload(
                    &disposition_id,
                    &act,
                    &pending,
                    &decision,
                    &requirement_delta,
                    "rejected_stale",
                );
                (DispositionVerdict::RejectedStale, payload)
            } else if pending.decided.is_some() {
                // 3. Conflicting — the same assessment already decided:
                //    ONE accepted decision per assessment (§4.4 同 assessment
                //    单 decision; a second accepted disposition would fail
                //    the verifier regardless of direction). Defense-in-depth:
                //    in the current flow a continue bumps the revision first,
                //    so the stale branch usually catches it — registered.
                let payload = disposition_payload(
                    &disposition_id,
                    &act,
                    &pending,
                    &decision,
                    &requirement_delta,
                    "rejected_conflicting",
                );
                (DispositionVerdict::RejectedConflicting, payload)
            } else if decision == "close" {
                // 4. Accepted close.
                let payload = disposition_payload(
                    &disposition_id,
                    &act,
                    &pending,
                    &decision,
                    &requirement_delta,
                    "accepted",
                );
                (DispositionVerdict::AcceptedClose, payload)
            } else {
                // 5. Accepted continue.
                let payload = disposition_payload(
                    &disposition_id,
                    &act,
                    &pending,
                    &decision,
                    &requirement_delta,
                    "accepted",
                );
                (DispositionVerdict::AcceptedContinue, payload)
            }
        };

        // ── commit (the verdict's state switch; §4.4 single writer) ────────
        // The disposition event + the close record (for an accepted close)
        // + the state switch are ONE commit; the ToolCompleted closes the
        // control call after it. Review F7 (2026-08-10): every error path
        // re-inserts the activation BEFORE leaving — a journal write
        // failure is run-fatal (it propagates to run_failed), but the
        // registry must never silently drop a live activation.
        //
        // ACAF Slice 1 (ADR-0011 §4.2): an ACCEPTED disposition is a control
        // event — ticket it before the commit (shadow mode). Rejected
        // verdicts and replay idempotency are mechanical records, not
        // state-moving control events — they stay unticketed (registered
        // boundary). Slice 2 fail-closed (2026-08-13): a Blocked gate
        // refuses the disposition — no commit, no state movement; the
        // parent sees an unauthorized tool result.
        let disposition_gate = if matches!(
            verdict,
            DispositionVerdict::AcceptedClose | DispositionVerdict::AcceptedContinue
        ) {
            Some(
                self.acaf_control_event(
                    writer,
                    TicketKind::DispositionV1,
                    Some(act.activation_id.clone()),
                    &serde_json::json!({
                        "role": role.as_str(),
                        "decision": decision,
                        "requirement_delta": requirement_delta,
                    }),
                )
                .await?,
            )
        } else {
            None
        };
        if let Some(TicketGate::Blocked { code, detail }) = &disposition_gate {
            let msg = format!(
                "[{disposition_id}] disposition unauthorized — DispositionV1 \
                 ticket rejected ({}: {}); activation {} unchanged",
                code.as_str(),
                detail,
                act.activation_id,
            );
            writer
                .record(
                    EventType::ToolCompleted,
                    serde_json::json!({
                        "tool": tc.name,
                        "call_id": tc.call_id,
                        "status": "error",
                        "error": format!("control_ticket_rejected:{}", code.as_str()),
                    }),
                )
                .await?;
            self.activations.lock().unwrap().states.insert(role, act);
            messages.push(Message {
                role: Role::Tool,
                content: msg.clone(),
                tool_call_id: Some(tc.call_id.clone()),
                tool_calls: Vec::new(),
                reasoning_content: None,
                round: None,
            });
            return Ok(ToolResult {
                output: msg,
                exit_code: Some(1),
                output_encoding: None,
                structured: None,
                ..Default::default()
            });
        }
        // D-16 (2026-08-13): a rejected GoalRevisionV1 under fail-closed
        // blocks the continue's state migration — captured for the message.
        let mut continue_unauthorized: Option<(String, String)> = None;
        let mut close_unauthorized: Option<(String, String)> = None;
        let commit: Result<(), AgentLoopError> = async {
            // Journal the disposition event — every verdict is journaled
            // (the refusal records document the rejection, §4.4).
            writer
                .record(EventType::RetrievalParentDisposition, payload.clone())
                .await?;
            let payload_canonical = canonical_json(&payload).unwrap_or_default();
            match verdict {
                DispositionVerdict::AcceptedClose => {
                    let p = act.pending.as_mut().unwrap();
                    p.decided = Some(decision.clone());
                    act.submitted
                        .push((disposition_id.clone(), payload_canonical));
                    let assessment_id = p.assessment_id.clone();
                    let result_digest = act.result_digest.clone();
                    // Close commit: the close record references the
                    // validated disposition (normal_close) and the state
                    // switch follows in the same handler — no window for a
                    // late disposition.
                    let close_gate = self
                        .write_close_record(
                            writer,
                            &act.activation_id,
                            &act.parent_session_id,
                            &act.subagent_session_id,
                            &act.contract_id,
                            act.contract_revision,
                            "normal_close",
                            Some(&disposition_id),
                            Some(&assessment_id),
                            result_digest.as_deref(),
                            act.effort.map(|e| e.as_str()),
                        )
                        .await?;
                    if let TicketGate::Blocked { code, detail } = close_gate {
                        close_unauthorized = Some((code.as_str().to_string(), detail.clone()));
                    } else {
                        act.status = ActivationStatus::Closed;
                        act.pending = None;
                        act.next_goal = None;
                    }
                }
                DispositionVerdict::AcceptedContinue => {
                    let p = act.pending.as_mut().unwrap();
                    p.decided = Some(decision.clone());
                    act.submitted
                        .push((disposition_id.clone(), payload_canonical));
                    // ACAF (Slice 1 + goal wiring 2026-08-12): a continue's
                    // requirement delta REVISES the activation's task goal —
                    // a goal-revision control event, ticketed. The
                    // GoalRevisionV1 ticket is signed and consumed under the
                    // OLD goal context (ensure_initialized → sign →
                    // verify_and_consume all read the old cached session —
                    // the ticket authorizes the transition itself); ONLY
                    // NOW does the run-level goal binding switch, so the
                    // next ticket call re-derives K_session and old
                    // unconsumed tickets die (ADR-0011 决策 5 — goal change
                    // → new key; Slice 1 audit D5 closed).
                    let goal_gate = self
                        .acaf_control_event(
                            writer,
                            TicketKind::GoalRevisionV1,
                            Some(act.activation_id.clone()),
                            &serde_json::json!({
                                "new_goal": requirement_delta.clone().unwrap_or_default(),
                            }),
                        )
                        .await?;
                    if let TicketGate::Blocked { code, detail } = &goal_gate {
                        // D-16: consumed-only — the state migration does NOT
                        // run; the rejection was already journaled.
                        continue_unauthorized = Some((code.as_str().to_string(), detail.clone()));
                    } else {
                        // §4.4: revision + 1, activation stays ACTIVE, the
                        // current assessment is marked consumed/superseded
                        // (the pending is KEPT as the consumed record).
                        act.contract_revision += 1;
                        let new_goal = requirement_delta.clone().unwrap_or_default();
                        self.update_goal(&new_goal);
                        act.next_goal = Some(new_goal);
                        act.status = ActivationStatus::Active;
                    }
                }
                DispositionVerdict::RejectedStale | DispositionVerdict::RejectedConflicting => {
                    // Pure record — no state movement (§4.4: refusal
                    // records document the rejection).
                    act.submitted
                        .push((disposition_id.clone(), payload_canonical));
                }
            }
            Ok(())
        }
        .await;

        // The verdict's message — read AFTER the commit so the continue
        // message shows the bumped revision (`pending` is a clone taken
        // before the commit; the rejection messages read its fields).
        let (output, exit_code) = match verdict {
            DispositionVerdict::AcceptedClose => (
                match &close_unauthorized {
                    Some((code, detail)) => format!(
                        "[{disposition_id}] close unauthorized — CloseV1 \
                         ticket rejected ({code}: {detail}); activation {} \
                         unchanged",
                        act.activation_id,
                    ),
                    None => format!(
                        "[{disposition_id}] close accepted — activation {} \
                         closed (normal_close)",
                        act.activation_id,
                    ),
                },
                if close_unauthorized.is_some() { 1 } else { 0 },
            ),
            DispositionVerdict::AcceptedContinue => match &continue_unauthorized {
                Some((code, detail)) => (
                    format!(
                        "[{disposition_id}] continue unauthorized — \
                         GoalRevisionV1 ticket rejected ({code}: {detail}); \
                         activation {} unchanged",
                        act.activation_id,
                    ),
                    1,
                ),
                None => (
                    format!(
                        "[{disposition_id}] continue accepted — contract \
                         revision {} -> {}; activation {} stays active",
                        act.contract_revision - 1,
                        act.contract_revision,
                        act.activation_id,
                    ),
                    0,
                ),
            },
            DispositionVerdict::RejectedStale => (
                format!(
                    "[{disposition_id}] rejected_stale — expected contract \
                     revision {} does not match the current {}",
                    pending.expected_contract_revision, act.contract_revision,
                ),
                1,
            ),
            DispositionVerdict::RejectedConflicting => (
                format!(
                    "[{disposition_id}] rejected_conflicting — assessment {} \
                     already decided {}",
                    pending.assessment_id,
                    pending.decided.as_deref().unwrap_or("?"),
                ),
                1,
            ),
        };

        // Re-insert the activation on EVERY path (review F7) — before any
        // error leaves, so a failed commit never drops a live activation.
        self.activations.lock().unwrap().states.insert(role, act);
        commit?;

        writer
            .record(
                EventType::ToolCompleted,
                serde_json::json!({
                    "tool": tc.name,
                    "call_id": tc.call_id,
                    "exit_code": exit_code,
                }),
            )
            .await?;
        messages.push(Message {
            role: Role::Tool,
            content: output.clone(),
            tool_call_id: Some(tc.call_id.clone()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        });
        // Audit mirror — the control call is one semantic action.
        // B1：写时盖 (round, domain) 章。
        // P2-14 S1（2026-09-04，ADR-0010 §14.54 复审处理）：父派发控制工具
        // 在检索子车道内执行，但写的是共享折叠分区（exec/tool_actions）——
        // 行章必须取执行窗主轮章（effective_blackboard_stamp，主 pin 优先），
        // 与同窗其它共享折叠写面同轴（此前漏改 live 章 = 双轴错配，登记
        // 于复审处理）。
        let (round, domain) = self.effective_blackboard_stamp();
        {
            let mut w = self.blackboard.write();
            w.push_tool_action(ToolActionRecord {
                category: "other".to_string(),
                tool: tc.name.clone(),
                timestamp: chrono_utc_now(),
                round,
                domain: Some(domain),
            });
            w.push_exec_result(crate::blackboard::ExecEntry::stamped(
                format!("[{}] {}", tc.name, output),
                round,
                domain,
                chrono_utc_now(),
            ));
        }
        Ok(ToolResult {
            output,
            exit_code: Some(exit_code),
            output_encoding: None,
            structured: None,
            ..Default::default()
        })
    }

    /// Journal a `retrieval_close_record` (ADR-0010 §4.4) for the given
    /// activation identity. `validated_disposition_id` is non-null only for
    /// `normal_close` (a committed close disposition); terminal authorities
    /// carry assessment/digest when a result was formed. `resumable=true`
    /// uniformly — the conversation/journal/ledger are preserved (§4.4;
    /// registered decision 2026-08-10).
    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k 第二批 (2026-08-30)：
    /// `effort` = 本激活最近一次派发的委托契约复杂度档（可选——旧 journal
    /// 与恢复激活不携带，schema optional）。
    #[allow(clippy::too_many_arguments)] // the full close-record identity
    async fn write_close_record(
        &self,
        writer: &mut EventWriter<'_>,
        activation_id: &str,
        parent_session_id: &str,
        subagent_session_id: &str,
        contract_id: &str,
        contract_revision: u32,
        terminal_reason: &str,
        validated_disposition_id: Option<&str>,
        assessment_id: Option<&str>,
        result_digest: Option<&str>,
        effort: Option<&str>,
    ) -> Result<TicketGate, AgentLoopError> {
        // ACAF Slice 1 (ADR-0011 §4.2): a close record is a control event —
        // ticket it before the record (shadow mode). Covers every terminal
        // reason (normal_close / subagent_failed / budget_exhausted / …).
        // Slice 2 fail-closed (2026-08-13): a Blocked gate refuses the
        // close record — no `retrieval_close_record` event, no state
        // switch; the rejection is already journaled.
        let gate = self
            .acaf_control_event(
                writer,
                TicketKind::CloseV1,
                Some(activation_id.to_string()),
                &serde_json::json!({
                    "activation_id": activation_id,
                    "terminal_reason": terminal_reason,
                    "validated_disposition_id": validated_disposition_id,
                }),
            )
            .await?;
        if let TicketGate::Blocked { .. } = &gate {
            return Ok(gate);
        }
        let close_record_id = format!(
            "CLOSE-{}-{:04}",
            &sha256_hex(
                &canonical_json(&serde_json::json!({
                    "activation_id": activation_id,
                    "contract_revision": contract_revision,
                    "terminal_reason": terminal_reason,
                    "assessment_id": assessment_id,
                }))
                .unwrap_or_default()
            )[..16],
            contract_revision,
        );
        // GAP-RETRIEVAL-TOOLS (2026-08-10): the close record cites the REAL
        // structured-result artifact when one was committed (normal_close /
        // budget_exhausted with a result); terminal closes without a result
        // keep the journal reference.
        let archive_ref = match (
            &result_digest,
            self.retrieval_capability_archive_ref(activation_id),
        ) {
            (Some(_), Some(artifact)) => artifact,
            _ => format!("run-journal:{}", writer.run_id()),
        };
        let mut close_payload = serde_json::json!({
            "close_record_id": close_record_id,
            "parent_session_id": parent_session_id,
            "subagent_session_id": subagent_session_id,
            "activation_id": activation_id,
            "contract_id": contract_id,
            "contract_revision": contract_revision,
            "result_digest": result_digest,
            "assessment_id": assessment_id,
            "validated_disposition_id": validated_disposition_id,
            "terminal_reason": terminal_reason,
            "resumable": true,
            "live_state_reset": true,
            "archive_ref": archive_ref,
        });
        if let Some(effort) = effort {
            close_payload["effort"] = serde_json::json!(effort);
        }
        writer
            .record(EventType::RetrievalCloseRecord, close_payload)
            .await?;
        Ok(gate)
    }

    /// GAP-RETRIEVAL-TOOLS (2026-08-10): the activation's committed-result
    /// artifact path (set at result formation; `None` before any commit).
    fn retrieval_capability_archive_ref(&self, activation_id: &str) -> Option<String> {
        self.activations
            .lock()
            .unwrap()
            .states
            .values()
            .find(|a| a.activation_id == activation_id)
            .and_then(|a| a.result_archive_ref.clone())
    }

    /// Close the role's activation with a TERMINAL authority reason (user
    /// cancel / subagent failed/cancelled / budget exhausted): journal the
    /// close record and commit the state switch. Idempotent — already-closed
    /// activations are skipped (the verifier rejects a second close record
    /// on an activation).
    pub(crate) async fn close_activation(
        &self,
        writer: &mut EventWriter<'_>,
        role: SubagentRole,
        terminal_reason: &str,
        assessment_id: Option<&str>,
        result_digest: Option<&str>,
    ) -> Result<(), AgentLoopError> {
        // Capture the identity (short critical section — the guard never
        // crosses an await).
        let snapshot = {
            let reg = self.activations.lock().unwrap();
            match reg.states.get(&role) {
                Some(a) if a.status != ActivationStatus::Closed => Some((
                    a.activation_id.clone(),
                    a.parent_session_id.clone(),
                    a.subagent_session_id.clone(),
                    a.contract_id.clone(),
                    a.contract_revision,
                    a.effort,
                )),
                _ => None,
            }
        };
        let Some((
            activation_id,
            parent_session_id,
            subagent_session_id,
            contract_id,
            contract_revision,
            act_effort,
        )) = snapshot
        else {
            return Ok(());
        };
        let gate = self
            .write_close_record(
                writer,
                &activation_id,
                &parent_session_id,
                &subagent_session_id,
                &contract_id,
                contract_revision,
                terminal_reason,
                None,
                assessment_id,
                result_digest,
                act_effort.map(|e| e.as_str()),
            )
            .await?;
        if let TicketGate::Blocked { .. } = &gate {
            // Slice 2 fail-closed (2026-08-13): the close record ticket was
            // rejected — the close control event is REFUSED, so the
            // activation stays open (no unticketed state switch). The
            // rejection is in the journal; terminal paths treat this as
            // best-effort (the run itself is already ending).
            return Ok(());
        }
        let mut reg = self.activations.lock().unwrap();
        if let Some(act) = reg.states.get_mut(&role) {
            act.status = ActivationStatus::Closed;
            act.pending = None;
            act.next_goal = None;
        }
        Ok(())
    }

    /// Close every pending activation with a terminal authority reason —
    /// used by the run-level cancellation path (a user cancel closes the
    /// activations BEFORE the run_cancelled terminal event; best-effort).
    pub(crate) async fn close_all_activations(
        &self,
        writer: &mut EventWriter<'_>,
        terminal_reason: &str,
    ) {
        let roles: Vec<SubagentRole> = {
            let reg = self.activations.lock().unwrap();
            reg.states.keys().copied().collect()
        };
        for role in roles {
            let _ = self
                .close_activation(writer, role, terminal_reason, None, None)
                .await;
        }
    }
}
