//! Projection — dispatch `TuiEvent`s into view-model mutations (Python
//! `projector.py` shape). Pure mutations; the renderer re-reads state on
//! every frame. Returns human status messages (logged to the event log).

use crate::app::TuiApp;
use crate::events::TuiEvent;

/// Apply one event to the app's view model, returning status messages.
pub fn apply_event(app: &mut TuiApp, event: TuiEvent) -> Vec<String> {
    match event {
        // ── lifecycle ──
        TuiEvent::RunPreflight { .. } => {
            app.status.update_item("守护", true);
            app.status.set_run_state("预检", true);
            app.content.add_system_message("预检通过", false);
            vec!["预检通过".into()]
        }
        TuiEvent::RunStarted { prompt, .. } => {
            app.running = true;
            // A new run clears the previous gate block — the OSC title's ⚠
            // state is per-run (slice #9).
            app.gate_block = None;
            app.status.set_run_state("运行中", true);
            if !prompt.is_empty()
                && !app.content.items.iter().any(|item| {
                    matches!(item, crate::view_model::ContentItem::Message(m)
                            if m.role == "用户" && m.content == prompt)
                })
            {
                app.content.add_user_message(&prompt);
            }
            vec!["运行开始".into()]
        }
        TuiEvent::RunFinished { status } => {
            app.running = false;
            // Turn ended — clear the streamed-text append target so the
            // next turn's first text delta starts a fresh card (review
            // P3-1: a turn ending on a tool round leaves the empty-card
            // index set).
            app.content.current_model_index = None;
            app.status.set_run_state("完成", true);
            app.content.collapse_non_warnings();
            app.content
                .add_system_message(&format!("运行完成（{status}）"), false);
            vec![format!("运行完成: {status}")]
        }
        TuiEvent::RunFailed { error } => {
            app.running = false;
            app.content.current_model_index = None;
            app.status.set_run_state("失败", false);
            app.content.collapse_non_warnings();
            app.content
                .add_system_message(&format!("[错误] {error}"), true);
            vec![format!("运行失败: {error}")]
        }
        TuiEvent::RunCancelled { reason } => {
            app.running = false;
            app.content.current_model_index = None;
            app.status.set_run_state("已取消", true);
            app.content.collapse_non_warnings();
            app.content
                .add_system_message(&format!("运行已取消（{reason}）"), false);
            vec!["运行已取消".into()]
        }
        TuiEvent::RunInvalidated { status } => {
            app.running = false;
            app.content.current_model_index = None;
            app.status.set_run_state("无效", false);
            app.content.collapse_non_warnings();
            app.content
                .add_system_message(&format!("运行无效（{status}）"), true);
            vec![format!("运行无效: {status}")]
        }

        // ── prompt / model ──
        TuiEvent::PromptSubmitted { prompt, .. } => {
            app.turn_counter += 1;
            // RunStarted (earlier in sequence) may already have created the
            // user card — never duplicate (keyed by content).
            let existing = app.content.items.iter().position(|item| {
                matches!(item, crate::view_model::ContentItem::Message(m)
                    if m.role == "用户" && m.content == prompt)
            });
            let card_line = match existing {
                Some(idx) => idx + 1, // 1-based content-item line
                None => {
                    app.content.add_user_message(&prompt);
                    app.content.items.len()
                }
            };
            app.marker.add_user_input(card_line);
            vec!["用户消息".into()]
        }
        TuiEvent::ModelRequest { provider, model_id } => {
            app.status.set_model(&model_id);
            app.status.update_item(&model_id, true);
            let _ = provider;
            vec![format!("模型请求: {model_id}")]
        }
        TuiEvent::ModelResponseReceived { .. } => vec!["模型响应已接收".into()],
        TuiEvent::RequestHeaderChange {
            reason,
            tool_count,
            header_sha256,
        } => {
            let short = header_sha256.chars().take(8).collect::<String>();
            let label = if reason == "initial" {
                "初始"
            } else {
                "变化"
            };
            app.content.add_system_message(
                &format!("请求头{label}: {tool_count} 个工具，摘要 {short}…"),
                false,
            );
            vec![format!("请求头{label}: {tool_count} 工具 {short}")]
        }
        TuiEvent::ModelOutput {
            text,
            tool_calls,
            finish_reason,
        } => {
            app.status.set_model("模型");
            app.status.set_run_state("运行中", true);
            let turn = app.turn_counter;
            // Tool calls: one borderless trace line per tool (Python
            // `tool_proposal` handler shape — the controller journals no
            // ToolProposal in the tool loop, so model_output carries them).
            for tc in &tool_calls {
                let target = summarize_arguments(&tc.arguments);
                let detail = if tc.name == "bash" {
                    target.clone()
                } else {
                    "...".into()
                };
                app.content
                    .add_or_update_tool_trace(&tc.name, &target, &detail);
            }
            // Text already streamed via TextDelta → don't duplicate the card
            // (Python `_on_model_output` dedupe: the card already carries the
            // full text). The dedupe requires an EXACT content match — a
            // different text means a new model output (next turn / final
            // answer after a tool round), which must get its own card
            // (review P1-3: matching any non-empty card dropped turn-2+).
            let streamed_match = app.content.current_model_index.is_some_and(|idx| {
                matches!(
                    app.content.items.get(idx),
                    Some(crate::view_model::ContentItem::Message(m))
                        if m.role == "模型" && m.content == text
                )
            });
            if streamed_match {
                app.content.current_model_index = None;
            } else if !text.is_empty() {
                app.content.add_model_message(&text, turn, false);
            } else if !tool_calls.is_empty() {
                app.content.add_model_message("", turn, false);
            }
            let _ = finish_reason;
            vec![format!("模型输出（turn {turn}）")]
        }

        // ── ACP lifecycle ──
        TuiEvent::AcpInitialize { protocol_version } => {
            vec![format!("ACP 初始化（协议 v{protocol_version}）")]
        }
        TuiEvent::AcpSessionCreated { session_id } => {
            app.session_id = Some(session_id.clone());
            app.content
                .add_system_message(&format!("ACP 会话已创建（{session_id}）"), false);
            vec!["ACP 会话已创建".into()]
        }

        // ── tools ──
        TuiEvent::ToolProposal {
            tool_name,
            input_summary,
            ..
        } => {
            let target = if input_summary.is_empty() {
                tool_name.clone()
            } else {
                input_summary
            };
            app.content
                .add_or_update_tool_trace(&tool_name, &target, "...");
            vec![format!("工具提议: {tool_name}")]
        }
        TuiEvent::PermissionRequested { tool, risk, .. } => {
            app.status.set_run_state("等待审批", true);
            let _ = risk;
            vec![format!("权限请求: {tool}")]
        }
        TuiEvent::PermissionDecision { tool, decision } => {
            app.status.set_run_state("运行中", true);
            if decision == "deny" || decision == "defer" {
                app.content
                    .add_system_message(&format!("[权限] {tool}: 拒绝"), true);
            } else {
                app.content
                    .add_system_message(&format!("[权限] {tool}: 允许"), false);
            }
            vec![format!("权限决定: {tool} = {decision}")]
        }
        TuiEvent::ToolStarted { tool, target, .. } => {
            let t = target.unwrap_or_default();
            let detail = if t.is_empty() {
                "运行中".into()
            } else {
                t.clone()
            };
            app.content.add_or_update_tool_trace(&tool, &t, &detail);
            app.toolbar.set_enabled("停止", true);
            vec![format!("工具开始: {tool}")]
        }
        TuiEvent::ToolCompleted {
            tool,
            status,
            error,
            ..
        } => {
            let detail = match error {
                Some(e) => format!("· 错误: {e}"),
                None => "· success".to_string(),
            };
            let _ = status;
            app.content.update_tool_latest(&tool, &detail);
            app.toolbar.set_enabled("停止", false);
            vec![format!("工具完成: {tool}")]
        }
        // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
        // 中间回报——工具仍在运行，系统消息展示运行时长与 PID。
        TuiEvent::ToolRunning {
            tool,
            call_id,
            wall_ms,
            pid,
        } => {
            let pid_note = pid.map(|p| format!("，PID {p}")).unwrap_or_default();
            app.content.add_system_message(
                &format!(
                    "[工具运行中] {tool} {call_id} 已运行 {:.0}s{pid_note}——命令继续运行，终态随后续工具结果返回",
                    wall_ms as f64 / 1000.0
                ),
                false,
            );
            vec![format!("工具运行中: {tool} {call_id}")]
        }

        // ── assurance gates ──
        TuiEvent::OrientationCheckpoint { step_index, .. } => {
            app.content
                .add_system_message(&format!("[检查点] step {step_index}"), false);
            vec![format!("方向检查点: step {step_index}")]
        }
        TuiEvent::ToolAvailabilityCheck {
            complete,
            incomplete,
            ..
        } => {
            let total = complete + incomplete;
            app.status.set_tool_probe(complete, total);
            app.content
                .add_system_message(&format!("工具探针: {complete} 链路完整"), false);
            vec![format!("工具探针: {complete}/{total}")]
        }
        TuiEvent::ToolBeliefStagnation { tool } => {
            app.content
                .add_system_message(&format!("[工具信念停滞] {tool}"), true);
            vec![format!("工具信念停滞: {tool}")]
        }
        TuiEvent::InstructionProvenanceGate { decision, .. } => {
            if decision == "block" {
                app.status.set_run_state("失败", false);
                app.gate_block = Some("IPG".into());
            }
            app.content
                .add_system_message(&format!("[门控] IPG: {decision}"), decision == "block");
            vec![format!("指令溯源门: {decision}")]
        }
        TuiEvent::GateDecision {
            gate,
            decision,
            reason,
            ..
        } => {
            if decision == "block" {
                app.status.set_run_state("失败", false);
                app.gate_block = Some(gate.clone());
            }
            let suffix = reason
                .filter(|r| !r.is_empty())
                .map(|r| format!("（{r}）"))
                .unwrap_or_default();
            app.content.add_system_message(
                &format!("[门控] {gate}: {decision}{suffix}"),
                decision == "block",
            );
            vec![format!("门控决定: {gate} = {decision}")]
        }

        // ── inquiry gates (§4.6) ──
        TuiEvent::NeutralInquiry { trigger_reason } => {
            app.content
                .add_system_message(&format!("[中立问询] {trigger_reason}"), false);
            vec![format!("中立问询: {trigger_reason}")]
        }
        TuiEvent::CounterexampleGate {
            position,
            model_response,
        } => {
            let suffix = model_response
                .filter(|r| !r.is_empty())
                .map(|r| format!("：{r}"))
                .unwrap_or_default();
            app.content
                .add_system_message(&format!("[反例询问] {position}{suffix}"), false);
            vec![format!("反例询问: {position}")]
        }
        TuiEvent::RetrievalCompletionCheck { role, decision } => {
            app.content
                .add_system_message(&format!("[检索完成确认] {role}: {decision}"), false);
            vec![format!("检索完成确认: {role} = {decision}")]
        }

        // ── v0.2 mechanism events (GAP-INQUIRY-SPLIT, 2026-08-09) ──
        TuiEvent::DiagnosticCoverageCheckpoint {
            checkpoint_id,
            threshold_stage,
        } => {
            app.content.add_system_message(
                &format!("[诊断覆盖检查点] {checkpoint_id} 阈值 {threshold_stage}"),
                false,
            );
            vec![format!("诊断覆盖检查点: {checkpoint_id}")]
        }
        // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16):
        // forced-template checkpoint round answer/validation verdict.
        TuiEvent::CheckpointResponse {
            checkpoint_id,
            inquiry_kind,
            attempt,
            outcome,
            validation_valid,
            validation_error_count,
        } => {
            let state = if validation_valid {
                "通过".to_string()
            } else {
                format!("失败({validation_error_count} 项)")
            };
            app.content.add_system_message(
                &format!(
                    "[模板检查点] {inquiry_kind} {checkpoint_id} 第 {attempt} 次: {outcome}（校验{state}）"
                ),
                false,
            );
            vec![format!("模板检查点: {checkpoint_id} → {outcome}")]
        }
        // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan
        // gate result — outcome and validation summary only.
        TuiEvent::PlanWrite {
            plan_id,
            goal,
            step_count,
            outcome,
            validation_valid,
            validation_error_count,
            degrade_reason,
        } => {
            let state = match (
                outcome.as_str(),
                validation_valid,
                degrade_reason.as_deref(),
            ) {
                ("accepted", true, _) => "校验通过".to_string(),
                ("refill_requested", _, _) => {
                    format!("校验失败({validation_error_count} 项)")
                }
                ("degraded", _, Some("plan_not_submitted")) => "未提交计划（机械降级）".to_string(),
                ("degraded", _, Some("plan_rotate_failed")) => "落板失败（机械降级）".to_string(),
                ("degraded", false, _) => {
                    format!("校验失败({validation_error_count} 项，机械降级)")
                }
                ("degraded", _, reason) => {
                    format!("机械降级（{}）", reason.unwrap_or("unknown"))
                }
                (other, _, _) => other.to_string(),
            };
            app.content.add_system_message(
                &format!(
                    "[计划写入] {plan_id} 「{goal}」({step_count} 步): {outcome}（校验{state}）"
                ),
                false,
            );
            vec![format!("计划写入: {plan_id} → {outcome}")]
        }
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console/direct
        // 双模式切换决策——方向 + 触发 + 模型决定。
        TuiEvent::ConsoleModeTransition {
            transition_id,
            from,
            to,
            trigger,
            model_decision,
        } => {
            app.content.add_system_message(
                &format!(
                    "[控制台模式] {from} → {to} ({trigger}, 决定 {model_decision}) {transition_id}"
                ),
                false,
            );
            vec![format!("控制台模式: {from} → {to} · {model_decision}")]
        }
        // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 动作栏订单记录。
        TuiEvent::ConsoleOrderWritten {
            order_id,
            action,
            step_id,
        } => {
            let step = step_id.as_deref().unwrap_or("(无步骤绑定)");
            app.content.add_system_message(
                &format!("[订单写入] {order_id} {action}（step {step}）"),
                false,
            );
            vec![format!("订单写入: {order_id} {action}")]
        }
        // P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): 发放前拒绝。
        TuiEvent::ConsoleOrderRejected {
            order_id,
            step,
            phase,
            code,
        } => {
            app.content.add_system_message(
                &format!("[订单拒绝] {order_id} {phase}/{step}: {code}（结果栏 receipt 保留）"),
                false,
            );
            vec![format!("订单拒绝: {order_id} {code}")]
        }
        // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): 机械折叠推进
        // ——请求视图前缀每窗口重写一次，事件给出折叠点与触发估算。
        TuiEvent::LedgerFoldAdvance {
            fold_start,
            fold_cut,
            rounds_folded,
            view_estimate_tokens,
            agent_role,
        } => {
            app.content.add_system_message(
                &format!(
                    "[台账折叠] {agent_role} fold_start={fold_start} fold_cut={fold_cut} \
                     轮次={rounds_folded} 估算={view_estimate_tokens} tokens（每窗口一次前缀重写）"
                ),
                false,
            );
            vec![format!("台账折叠: {rounds_folded} 轮")]
        }
        // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
        // §14.28 审查修复): 外挂台账写失败——审计告警；预算耗尽后折叠被
        // 禁用（会话继续走全量视图，不空转）。
        TuiEvent::LedgerFoldWriteFailed {
            ledger_path,
            attempt,
            disabled,
            rows,
            ..
        } => {
            app.content.add_system_message(
                &format!(
                    "[折叠台账写失败] 第 {attempt} 次追加失败（{rows} 行）: {ledger_path}{}",
                    if disabled {
                        " — 折叠已禁用，继续全量视图"
                    } else {
                        ""
                    }
                ),
                true,
            );
            vec![format!("折叠台账写失败: {ledger_path}")]
        }
        // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计
        // §2.3): transport 重试计数事件面——恢复/耗尽摘要进入系统消息。
        TuiEvent::TransportRetry {
            outcome,
            kind,
            retries,
            ..
        } => {
            let kind_label = kind.as_deref().unwrap_or("unknown");
            let message = match outcome.as_str() {
                "recovered" => {
                    format!("[模型重试] {kind_label} 重试 {retries} 次后恢复")
                }
                "exhausted" => {
                    format!("[模型重试] {kind_label} 重试 {retries} 次后耗尽")
                }
                _ => format!("[模型重试] {kind_label} × {retries}"),
            };
            app.content.add_system_message(&message, false);
            vec![message]
        }
        TuiEvent::InformationSufficiencyAssessment {
            assessment_id,
            status,
            source_total,
        } => {
            app.content.add_system_message(
                &format!("[信息充分性] {assessment_id}: {status}（来源 {source_total}）"),
                false,
            );
            vec![format!("信息充分性: {status}")]
        }
        TuiEvent::RetrievalParentDisposition {
            disposition_id,
            decision,
        } => {
            app.content
                .add_system_message(&format!("[检索处置] {disposition_id}: {decision}"), false);
            vec![format!("检索处置: {decision}")]
        }
        TuiEvent::RetrievalCloseRecord {
            close_record_id,
            terminal_reason,
        } => {
            app.content.add_system_message(
                &format!("[检索关闭] {close_record_id}: {terminal_reason}"),
                false,
            );
            vec![format!("检索关闭: {terminal_reason}")]
        }
        // GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode / result / restore
        // projections.
        TuiEvent::RetrievalModeTransition {
            transition_id,
            old_mode,
            new_mode,
        } => {
            app.content.add_system_message(
                &format!("[检索模式] {transition_id}: {old_mode} → {new_mode}"),
                false,
            );
            vec![format!("检索模式: {old_mode} → {new_mode}")]
        }
        TuiEvent::RetrievalResultCommitted {
            result_id,
            source_total,
        } => {
            app.content.add_system_message(
                &format!("[检索结果] {result_id}: {source_total} 来源"),
                false,
            );
            vec![format!("检索结果: {source_total} 来源")]
        }
        TuiEvent::RetrievalActivationRestored {
            restore_id,
            activation_id,
            status,
        } => {
            app.content.add_system_message(
                &format!("[激活恢复] {restore_id}: {activation_id} ({status})"),
                false,
            );
            vec![format!("激活恢复: {status}")]
        }
        // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): 机械审查
        // 层轻量事件留痕——只展示键与异常（摘要细节留在 journal）。
        TuiEvent::MechanicalAuditUpdate {
            key,
            round,
            anomaly,
        } => {
            let line = match anomaly {
                Some(a) => format!("[机械审查] {key}（轮 {round}）：{a}"),
                None => format!("[机械审查] {key}（轮 {round}）"),
            };
            app.content.add_system_message(&line, false);
            vec![line]
        }
        // ACAF Slice 1 (ADR-0011 §4.6): control-ticket projections — the
        // ticket lifecycle is a mechanism event; the TUI shows kind + reject
        // code (never the HMAC or binding digests).
        TuiEvent::ControlTicketIssued {
            ticket_id,
            ticket_kind,
            capability_scope,
            ..
        } => {
            app.content.add_system_message(
                &format!("[票据签发] {ticket_id}: {ticket_kind} ({capability_scope})"),
                false,
            );
            vec!["票据签发".into()]
        }
        TuiEvent::ControlTicketConsumed {
            ticket_id,
            ticket_kind,
        } => {
            app.content
                .add_system_message(&format!("[票据消费] {ticket_id}: {ticket_kind}"), false);
            vec!["票据消费".into()]
        }
        TuiEvent::ControlTicketRejected {
            ticket_id,
            ticket_kind,
            reject_code,
        } => {
            app.content.add_system_message(
                &format!("[票据拒绝] {ticket_id}: {ticket_kind} ({reject_code})"),
                false,
            );
            vec![format!("票据拒绝: {reject_code}")]
        }

        // ── A6 explicit context compaction ──
        TuiEvent::ContextCompressed {
            trigger_tokens,
            rounds_dropped,
            estimated_tokens_after,
        } => {
            app.content.add_system_message(
                &format!(
                    "[上下文压缩] 触发于 {}K tokens，压掉 {rounds_dropped} 轮（估 ~{}K）",
                    trigger_tokens / 1000,
                    estimated_tokens_after / 1000,
                ),
                true,
            );
            vec![format!(
                "上下文已压缩: {rounds_dropped} 轮（触发 {t}K）",
                t = trigger_tokens / 1000,
            )]
        }

        // D2-2 (2026-08-14): recovery pre-check truncation.
        TuiEvent::ContextRecoveryTruncated {
            before_estimate_tokens,
            rounds_dropped,
            after_estimate_tokens,
        } => {
            app.content.add_system_message(
                &format!(
                    "[上下文恢复截断] 恢复对话估 {}K，机械截断 {rounds_dropped} 轮（估 ~{}K）",
                    before_estimate_tokens / 1000,
                    after_estimate_tokens / 1000,
                ),
                true,
            );
            vec![format!(
                "恢复对话已截断: {rounds_dropped} 轮（估 {b}K → {a}K）",
                b = before_estimate_tokens / 1000,
                a = after_estimate_tokens / 1000,
            )]
        }

        // F7 (2026-08-15): a blackboard plan-epoch archive write failed —
        // the rotation committed in memory but the durable snapshot is
        // missing (audit trace; shown as a warning).
        TuiEvent::EpochArchiveWriteFailed {
            plan_epoch,
            archive_dir,
            kind,
        } => {
            app.content.add_system_message(
                &format!("[黑板归档失败] epoch {plan_epoch}（{kind}）未落盘: {archive_dir}"),
                true,
            );
            vec![format!("黑板归档失败: epoch {plan_epoch}（{kind}）")]
        }

        // ── IP5 snapshot ──
        TuiEvent::SnapshotCreated {
            tool,
            snapshot_hash,
            snapshot_error,
            ..
        } => {
            if let Some(err) = snapshot_error {
                app.content
                    .add_system_message(&format!("[快照] {tool}: 失败（{err}）"), true);
                vec![format!("快照失败: {tool}")]
            } else if let Some(hash) = snapshot_hash {
                let short: String = hash.chars().take(8).collect();
                app.content
                    .add_system_message(&format!("[快照] {tool} {short}"), false);
                vec![format!("快照已建: {tool} {short}")]
            } else {
                vec![format!("快照: {tool}（无哈希）")]
            }
        }

        TuiEvent::SnapshotRestored {
            snapshot_hash,
            restored,
            snapshot_error,
            ..
        } => {
            if let Some(err) = snapshot_error {
                app.content
                    .add_system_message(&format!("[快照恢复] 失败（{err}）"), true);
                vec![format!("快照恢复失败: {err}")]
            } else {
                let short: String = snapshot_hash
                    .as_deref()
                    .map(|h| h.chars().take(8).collect())
                    .unwrap_or_default();
                app.content.add_system_message(
                    &format!("[快照恢复] {short} 恢复 {} 个文件", restored.len()),
                    false,
                );
                vec![format!("快照恢复: {short}（{} 个文件）", restored.len())]
            }
        }

        // ── artifact ──
        TuiEvent::ArtifactRegistered { artifact_path, .. } => {
            app.content
                .add_system_message(&format!("[制品] {artifact_path}"), false);
            vec![format!("制品已登记: {artifact_path}")]
        }

        // ── plan mode ──
        TuiEvent::PlanProposed {
            plan_id, sections, ..
        } => {
            app.content
                .add_system_message(&format!("[计划] {plan_id}（{sections} 节）"), false);
            vec![format!("计划已提出: {plan_id}")]
        }
        TuiEvent::PlanApproved {
            plan_id,
            plan_epoch,
            authority,
            ..
        } => {
            app.content.add_system_message(
                &format!("[计划] {plan_id}（epoch {plan_epoch}）已批准（{authority}）"),
                false,
            );
            vec![format!("计划已批准: {plan_id}（epoch {plan_epoch}）")]
        }
        TuiEvent::PlanRejected { plan_id } => {
            app.content
                .add_system_message(&format!("[计划] {plan_id} 已拒绝"), true);
            vec![format!("计划已拒绝: {plan_id}")]
        }
        TuiEvent::ActionApproved { action_id } => {
            app.content
                .add_system_message(&format!("[动作] {action_id} 已批准"), false);
            vec![format!("动作已批准: {action_id}")]
        }

        // ── synthetic ──
        TuiEvent::TextDelta { text } => {
            // First delta of a turn creates the card (Python `_on_text_delta`).
            if app.content.current_model_index.is_none() {
                app.content.add_model_message("", app.turn_counter, false);
            }
            app.content.append_text_delta(&text);
            app.status.set_model("模型");
            vec![]
        }
        TuiEvent::StatusUpdate { label, ok } => {
            if label == "验证" {
                // Replay-mode chain validity — visible card + run state.
                app.content.add_system_message(
                    &format!("[验证] 日志链{}", if ok { "有效" } else { "无效" }),
                    !ok,
                );
                app.status
                    .set_run_state(if ok { "完成" } else { "失败" }, ok);
            } else {
                app.status.update_item(&label, ok);
            }
            vec![]
        }
        TuiEvent::Unknown { event_type } => {
            app.content
                .add_system_message(&format!("[未知事件] {event_type}"), true);
            vec![format!("未知事件: {event_type}")]
        }
    }
}

/// One-line argument summary for a tool trace target (truncated).
fn summarize_arguments(arguments: &str) -> String {
    let compact = arguments.replace(['\n', '\r'], " ");
    let mut out = compact;
    if out.len() > 60 {
        let cut: String = out.chars().take(60).collect();
        out = format!("{cut}…");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::ToolCallInfo;
    use crate::view_model::ContentItem;

    fn app() -> TuiApp {
        TuiApp::new()
    }

    #[test]
    fn prompt_submitted_creates_user_card_and_marker() {
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "你好".into(),
            character_count: 2,
        });
        assert_eq!(a.turn_counter, 1);
        assert_eq!(a.content.items.len(), 1);
        assert_eq!(a.marker.markers.len(), 1);
        assert_eq!(a.marker.markers[0].label, "▸ L1");
    }

    #[test]
    fn user_card_is_not_duplicated_in_real_sequence_order() {
        // Journal order: RunStarted first, then PromptSubmitted.
        let mut a = app();
        a.accept_event(TuiEvent::RunStarted {
            prompt: "你好".into(),
            timestamp: String::new(),
        });
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "你好".into(),
            character_count: 2,
        });
        assert_eq!(a.content.items.len(), 1, "PromptSubmitted must dedupe");
        assert!(a.running);
        assert_eq!(a.status.items[5].label, "运行中");
        assert_eq!(a.marker.markers.len(), 1);

        // Reverse order (defensive): RunStarted must also dedupe.
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "你好".into(),
            character_count: 2,
        });
        a.accept_event(TuiEvent::RunStarted {
            prompt: "你好".into(),
            timestamp: String::new(),
        });
        assert_eq!(a.content.items.len(), 1, "RunStarted must dedupe");
    }

    #[test]
    fn model_output_with_tool_calls_creates_traces_and_card() {
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "q".into(),
            character_count: 1,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: String::new(),
            tool_calls: vec![ToolCallInfo {
                name: "bash".into(),
                arguments: "{\"command\":\"dir\"}".into(),
                call_id: "call-1".into(),
            }],
            finish_reason: "tool_calls".into(),
        });
        assert_eq!(a.content.tool_trace_names(), vec!["bash"]);
        // A model card exists (empty text, but tool round in progress).
        let has_model_card = a
            .content
            .items
            .iter()
            .any(|i| matches!(i, ContentItem::Message(m) if m.role == "模型"));
        assert!(has_model_card);
    }

    #[test]
    fn model_output_plain_text_creates_card() {
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "q".into(),
            character_count: 1,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "最终答案".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let ContentItem::Message(m) = &a.content.items[1] else {
            panic!("expected model card");
        };
        assert_eq!(m.role, "模型");
        assert_eq!(m.content, "最终答案");
    }

    #[test]
    fn text_delta_then_model_output_no_duplicate_card() {
        let mut a = app();
        a.accept_event(TuiEvent::TextDelta {
            text: "流式".into(),
        });
        a.accept_event(TuiEvent::TextDelta {
            text: "文本".into(),
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "流式文本".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let model_cards = a
            .content
            .items
            .iter()
            .filter(|i| matches!(i, ContentItem::Message(m) if m.role == "模型"))
            .count();
        assert_eq!(model_cards, 1, "streamed + full output must not duplicate");
        let ContentItem::Message(m) = &a.content.items[0] else {
            panic!("expected model card");
        };
        assert_eq!(m.content, "流式文本");
    }

    #[test]
    fn terminal_event_resets_stream_append_target() {
        // Review P3-1: a turn ending on a tool round (empty-text model_output
        // with tool_calls) leaves the append target on the empty card — the
        // next turn's first text delta must start a FRESH card, not append
        // to the previous turn's empty one.
        let mut a = app();
        a.accept_event(TuiEvent::ModelOutput {
            text: "".into(),
            tool_calls: vec![crate::events::ToolCallInfo {
                name: "read_file".into(),
                arguments: "…".into(),
                call_id: "call-1".into(),
            }],
            finish_reason: "tool_calls".into(),
        });
        a.accept_event(TuiEvent::RunFinished {
            status: "completed".into(),
        });
        a.accept_event(TuiEvent::TextDelta {
            text: "新轮回答".into(),
        });
        let model_cards: Vec<&str> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                ContentItem::Message(m) if m.role == "模型" => Some(m.content.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            model_cards,
            vec!["", "新轮回答"],
            "next turn's delta must create a fresh card after the terminal event"
        );
    }

    #[test]
    fn tool_lifecycle_updates_same_trace_line() {
        let mut a = app();
        a.accept_event(TuiEvent::ToolStarted {
            tool: "read_file".into(),
            call_id: "call-1".into(),
            target: Some("lib.rs".into()),
        });
        a.accept_event(TuiEvent::ToolCompleted {
            tool: "read_file".into(),
            call_id: "call-1".into(),
            status: "success".into(),
            error: None,
            target: None,
        });
        let ContentItem::ToolTrace(t) = &a.content.items[0] else {
            panic!("expected tool trace");
        };
        assert_eq!(t.entries.len(), 1);
        assert_eq!(t.entries[0].detail, "· success");
    }

    #[test]
    fn permission_deny_adds_warning_card() {
        let mut a = app();
        a.accept_event(TuiEvent::PermissionRequested {
            tool: "bash".into(),
            risk: "SandboxEscape".into(),
            call_id: "c".into(),
        });
        assert_eq!(a.status.items[5].label, "等待审批");
        a.accept_event(TuiEvent::PermissionDecision {
            tool: "bash".into(),
            decision: "deny".into(),
        });
        let ContentItem::Message(m) = &a.content.items[0] else {
            panic!("expected warning card");
        };
        assert!(m.warning);
        assert!(m.content.contains("拒绝"));
        assert_eq!(a.status.items[5].label, "运行中");
    }

    #[test]
    fn snapshot_events_render_cards() {
        let mut a = app();
        a.accept_event(TuiEvent::SnapshotCreated {
            tool: "edit_file".into(),
            targets: vec!["lib.rs".into()],
            snapshot_hash: Some("0123456789abcdef".into()),
            snapshot_error: None,
        });
        a.accept_event(TuiEvent::SnapshotCreated {
            tool: "edit_file".into(),
            targets: vec!["lib.rs".into()],
            snapshot_hash: None,
            snapshot_error: Some("disk full".into()),
        });
        let msgs: Vec<&str> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                ContentItem::Message(m) => Some(m.content.as_str()),
                _ => None,
            })
            .collect();
        assert!(msgs[0].contains("01234567"));
        assert!(msgs[1].contains("失败"));
    }

    /// Slice #8: restore events render as snapshot-restore cards — success
    /// (hash + file count) and failure (warning).
    #[test]
    fn snapshot_restored_events_render_cards() {
        let mut a = app();
        a.accept_event(TuiEvent::SnapshotRestored {
            snapshot_hash: Some("0123456789abcdef".into()),
            scope: None,
            restored: vec!["lib.rs".into(), "Cargo.toml".into()],
            snapshot_error: None,
        });
        a.accept_event(TuiEvent::SnapshotRestored {
            snapshot_hash: None,
            scope: None,
            restored: vec![],
            snapshot_error: Some("snapshot not found".into()),
        });
        let msgs: Vec<&str> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                ContentItem::Message(m) => Some(m.content.as_str()),
                _ => None,
            })
            .collect();
        assert!(msgs[0].contains("01234567"), "hash short form: {}", msgs[0]);
        assert!(msgs[0].contains("2"), "file count: {}", msgs[0]);
        assert!(msgs[1].contains("失败"));
    }

    #[test]
    fn terminal_finished_collapses() {
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "q".into(),
            character_count: 1,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "答".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        assert!(!a.content.items[1].collapsed());
        a.accept_event(TuiEvent::RunFinished {
            status: "completed".into(),
        });
        assert!(!a.running);
        assert!(a.content.items[0].collapsed());
        assert!(a.content.items[1].collapsed());
    }

    #[test]
    fn gate_decision_block_is_warning() {
        let mut a = app();
        a.accept_event(TuiEvent::GateDecision {
            gate: "instruction_provenance_gate".into(),
            decision: "block".into(),
            reason: None,
            tools: vec!["bash".into()],
        });
        let ContentItem::Message(m) = &a.content.items[0] else {
            panic!("expected card");
        };
        assert!(m.warning);
        assert_eq!(a.status.items[5].label, "失败");
    }

    #[test]
    fn plan_events_show_cards() {
        let mut a = app();
        a.accept_event(TuiEvent::PlanProposed {
            plan_id: "PLAN-1".into(),
            task_id: "T".into(),
            sections: 4,
        });
        a.accept_event(TuiEvent::PlanApproved {
            plan_id: "PLAN-1".into(),
            plan_epoch: 1,
            authority: "user".into(),
            decision: "approve".into(),
            execution_policy: "manual".into(),
        });
        a.accept_event(TuiEvent::PlanRejected {
            plan_id: "PLAN-2".into(),
        });
        let msgs: Vec<&str> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                ContentItem::Message(m) => Some(m.content.as_str()),
                _ => None,
            })
            .collect();
        assert!(msgs[0].contains("PLAN-1（4 节）"));
        assert!(msgs[1].contains("已批准"));
        assert!(msgs[2].contains("已拒绝"));
    }

    #[test]
    fn unknown_event_is_warning_card() {
        let mut a = app();
        a.accept_event(TuiEvent::Unknown {
            event_type: "weird".into(),
        });
        let ContentItem::Message(m) = &a.content.items[0] else {
            panic!("expected card");
        };
        assert!(m.warning);
    }

    #[test]
    fn tool_availability_updates_sources_segment() {
        let mut a = app();
        a.accept_event(TuiEvent::ToolAvailabilityCheck {
            complete: 3,
            incomplete: 1,
            gate_decision: "pass".into(),
        });
        assert_eq!(a.status.items[3].label, "工具 3/4");
    }

    /// Per-mapping-row coverage (review P2-4): every event type not already
    /// exercised by a dedicated test must at least produce its expected card
    /// or status effect without panicking.
    /// (event, predicate) — the predicate asserts the expected projection.
    type Case = (TuiEvent, fn(&TuiApp) -> bool);
    #[test]
    fn remaining_mapping_rows_produce_expected_effects() {
        let cases: Vec<Case> =
            vec![
                (
                    TuiEvent::RunPreflight {
                        timestamp: String::new(),
                    },
                    |a| a.status.items[5].label == "预检" && a.content.items.len() == 1,
                ),
                (
                    TuiEvent::RunFailed {
                        error: "崩溃".into(),
                    },
                    |a| {
                        !a.running
                            && a.status.items[5].label == "失败"
                            && a.content.items.iter().any(|i| {
                                matches!(i, ContentItem::Message(m)
                                if m.warning && m.content.contains("崩溃"))
                            })
                    },
                ),
                (
                    TuiEvent::RunCancelled {
                        reason: "user".into(),
                    },
                    |a| !a.running && a.status.items[5].label == "已取消",
                ),
                (
                    TuiEvent::RunInvalidated {
                        status: "degeneration".into(),
                    },
                    |a| !a.running && a.status.items[5].label == "无效",
                ),
                (
                    TuiEvent::ModelRequest {
                        provider: "fake".into(),
                        model_id: "deepseek".into(),
                    },
                    |a| a.status.items[4].label == "deepseek",
                ),
                (
                    // No content card — a status message only.
                    TuiEvent::ModelResponseReceived { tool_calls: vec![] },
                    |a| a.events_log.iter().any(|e| e == "model_response_received"),
                ),
                (
                    TuiEvent::AcpInitialize {
                        protocol_version: 1,
                    },
                    |a| a.events_log.iter().any(|e| e == "acp_initialize"),
                ),
                (
                    TuiEvent::AcpSessionCreated {
                        session_id: "S-1".into(),
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                            matches!(i, ContentItem::Message(m)
                        if m.content.contains("S-1"))
                        })
                    },
                ),
                (
                    TuiEvent::ToolProposal {
                        tool_name: "bash".into(),
                        call_id: "c".into(),
                        input_summary: "dir".into(),
                    },
                    |a| a.content.tool_trace_names() == vec!["bash"],
                ),
                (
                    TuiEvent::OrientationCheckpoint {
                        checkpoint_id: "ORIENT-1".into(),
                        trigger: "pre_handoff".into(),
                        step_index: 3,
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("step 3"))
                })
                    },
                ),
                (
                    TuiEvent::ToolBeliefStagnation {
                        tool: "bash".into(),
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("工具信念停滞"))
                })
                    },
                ),
                (
                    TuiEvent::InstructionProvenanceGate {
                        decision: "block".into(),
                        entries: 1,
                    },
                    |a| {
                        a.status.items[5].label == "失败" && a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("IPG"))
                })
                    },
                ),
                (
                    TuiEvent::NeutralInquiry {
                        trigger_reason: "rounds".into(),
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("中立问询"))
                })
                    },
                ),
                (
                    TuiEvent::CounterexampleGate {
                        position: "final_answer".into(),
                        model_response: None,
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("反例询问"))
                })
                    },
                ),
                (
                    TuiEvent::RetrievalCompletionCheck {
                        role: "internal_retrieval".into(),
                        decision: "yes".into(),
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("检索完成确认"))
                })
                    },
                ),
                (
                    TuiEvent::ArtifactRegistered {
                        artifact_path: "p.json".into(),
                        artifact_sha256: "h".into(),
                    },
                    |a| {
                        a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("p.json"))
                })
                    },
                ),
                (
                    TuiEvent::ActionApproved {
                        action_id: "A-1".into(),
                    },
                    |a| {
                        a.content.items.iter().any(
                            |i| matches!(i, ContentItem::Message(m) if m.content.contains("A-1")),
                        )
                    },
                ),
                (
                    TuiEvent::StatusUpdate {
                        label: "验证".into(),
                        ok: true,
                    },
                    |a| {
                        a.status.items[5].label == "完成" && a.content.items.iter().any(|i| {
                    matches!(i, ContentItem::Message(m) if m.content.contains("日志链有效"))
                })
                    },
                ),
            ];
        for (i, (event, check)) in cases.iter().enumerate() {
            let mut a = app();
            a.accept_event(event.clone());
            assert!(
                check(&a),
                "mapping row #{i} ({}) produced no expected effect",
                event.kind()
            );
        }
    }

    /// P1-3 regression: model outputs of later turns must each get their own
    /// card — the dedupe only applies to an exact text match (streaming).
    #[test]
    fn consecutive_turns_each_get_their_own_model_card() {
        let mut a = app();
        // Turn 1.
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "问题1".into(),
            character_count: 3,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "回答1".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        a.accept_event(TuiEvent::RunFinished {
            status: "completed".into(),
        });
        // Turn 2.
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "问题2".into(),
            character_count: 3,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "回答2".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let model_texts: Vec<&str> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                ContentItem::Message(m) if m.role == "模型" => Some(m.content.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            model_texts,
            vec!["回答1", "回答2"],
            "turn-2 output must not be dropped"
        );

        // Streaming dedupe still works: TextDelta accumulation then the full
        // text must not create a duplicate card.
        let mut a = app();
        a.accept_event(TuiEvent::TextDelta {
            text: "流式".into(),
        });
        a.accept_event(TuiEvent::TextDelta {
            text: "文本".into(),
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "流式文本".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let model_cards = a
            .content
            .items
            .iter()
            .filter(|i| matches!(i, ContentItem::Message(m) if m.role == "模型"))
            .count();
        assert_eq!(model_cards, 1);
    }
}
