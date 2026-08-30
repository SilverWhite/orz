//! Delivery flow helpers — batch B2 of the controller split
//! (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use orz_assurance::EventType;

use crate::controller::{AgentLoopController, AgentLoopError, EventWriter};
use crate::gateway::model::{Message, Role};
use crate::host::{LoopHost, ToolResult};

impl AgentLoopController {
    /// THIN-HARNESS-REDESIGN-V2 §9.3 (2026-08-29 审查处理)：submit 确认
    /// 消息——有 plan 时注明末步完成并入最终回答流程；无 plan（降级纯
    /// 状态展示）不虚构机械"最终回答流程"（终答只由模型自发），明确仅
    /// 状态展示、未推进计划步骤。
    pub(crate) fn submit_confirm_message(terminal_id: Option<&str>, status: &str) -> String {
        match terminal_id {
            Some(id) => format!("submit: 递交已确认，末步 {id} 完成，进入最终回答流程。\n{status}"),
            None => format!(
                "submit: 递交已确认（无计划基线，仅状态展示，未推进计划步骤；终答请自行给出）。\n{status}"
            ),
        }
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the mechanical delivery
    /// status — diff the live worktree snapshot against the plan-approval
    /// baseline (`.gsa`/缓存目录已由 walk 排除), capped at 20 entries +
    /// count line + truncation marker. Tool output — rendered into the
    /// blackboard plan view by `submit`, never a model-authored statement.
    /// `None` baseline (host without snapshot support) reports the change
    /// list as unavailable rather than fabricating one.
    pub(crate) fn compute_delivery_status(&self, host: &dyn LoopHost) -> String {
        let baseline = self.delivery_baseline.lock().unwrap().clone();
        let Some(before) = baseline else {
            // THIN-HARNESS-REDESIGN-V2 §9.3 (2026-08-29)：无计划批准基线
            // ——无 plan 会话（submit 降级为纯状态展示）或计划尚未批准；
            // 活快照存在时才会走到这里，故归因于基线缺失而非 host 能力。
            return "[delivery] 状态: 变更清单不可用（无计划批准基线；submit 保持信息展示）"
                .to_string();
        };
        let Some(after) = host.workspace_snapshot() else {
            return "[delivery] 状态: 变更清单不可用（host 未提供工作区快照）".to_string();
        };
        let (all, _) = crate::host::workspace_delta_diff(&before, &after, usize::MAX);
        let total = all.len();
        let cap = crate::host::DELIVERY_DELTA_MAX_ENTRIES;
        let truncated = total > cap;
        let shown = &all[..total.min(cap)];
        let kinds = shown
            .iter()
            .map(|e| {
                let kind = match e.kind {
                    crate::host::WorkspaceDeltaKind::Added => "A",
                    crate::host::WorkspaceDeltaKind::Modified => "M",
                    crate::host::WorkspaceDeltaKind::Deleted => "D",
                };
                format!("{} {}", e.path, kind)
            })
            .collect::<Vec<_>>()
            .join(", ");
        let mut line = format!("[delivery] 状态: {total} 个变更 ({kinds})");
        if truncated {
            line.push_str(&format!(" — 仅显示前 {cap} 条（截断）"));
        }
        line
    }

    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.4):
    /// `console_mode_transition` 事件 + 黑板 gate_log 同步（模型可读）。
    pub(crate) async fn refuse_console_tool(
        &self,
        writer: &mut EventWriter<'_>,
        messages: &mut Vec<Message>,
        tool: &str,
        call_id: &str,
        code: &str,
        msg: &str,
    ) -> Result<ToolResult, AgentLoopError> {
        writer
            .record(
                EventType::ToolCompleted,
                serde_json::json!({
                    "tool": tool,
                    "call_id": call_id,
                    "exit_code": 1,
                    "status": "error",
                    "error": code,
                }),
            )
            .await?;
        messages.push(Message {
            role: Role::Tool,
            content: msg.to_string(),
            tool_call_id: Some(call_id.to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        // P2-10 R2 (2026-08-31): console-tool refusal (submit / step_done /
        // return) = deny event.
        self.feed_lif_deny(None);
        Ok(ToolResult {
            output: msg.to_string(),
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{ModelGateway, ToolCall};
    use orz_assurance::{EventType, JournalRecorder};
    use std::collections::VecDeque;
    use std::sync::Arc;

    /// 设计 §2.8/§5 验收 5（行为侧零残留）：终答携带未绑定本 run 证据的
    /// `[来源: SRC-999]` 标记也不再机械拦截——原样交付（旧引用校验器会
    /// block 并替换为降级块）。
    #[tokio::test]
    async fn final_answer_with_unbound_citation_delivered_as_is() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::from_texts(vec![
            "草稿",
            "结论 [来源: SRC-999]",
        ]));
        let controller = AgentLoopController::with_gateway(gateway);
        let (response, _, _) = controller
            .run_turn(
                &host,
                "hello",
                "RUN-CITE-GONE",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(response, "结论 [来源: SRC-999]");
        // 运行正常终止（无引用校验事件类型——编译期已删除该变体）。
        let types = event_types(&dir);
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == EventType::RunFinished)
                .count(),
            1,
            "{types:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── AGENT-DELIVERY-FLOW (2026-08-23, ADR-0010 §14.35 第 19 项) ─────

    /// 设计 §2.2：`submit` 两阶段——第一次调用机械计算交付状态渲染进黑板
    /// plan 视图（末步不置 done）；第二次调用确认置末步 done（进入最终回答
    /// 流程）。普通订单绑定末步不产生 done（末步仅显式递交推进）。
    #[tokio::test]
    async fn submit_two_phase_renders_status_then_confirms() {
        let dir = test_dir();
        let baseline =
            std::collections::HashMap::from([("src/cache.rs".to_string(), (10u64, 1u64, 0u32))]);
        let after = std::collections::HashMap::from([
            ("src/cache.rs".to_string(), (10u64, 1u64, 0u32)),
            ("output.txt".to_string(), (5u64, 2u64, 0u32)),
        ]);
        let host = DeliveryHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: ok_result(),
            snapshots: std::sync::Mutex::new(VecDeque::from([
                baseline.clone(),
                after.clone(),
                after.clone(),
            ])),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            // MECHANICAL-AUDIT-LAYER 审查处理 (2026-08-24): direct 面——
            // 工作模拟用直接只读调用（订单层已退役）。
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({ "target_file": "a.txt" }),
                call_id: "call-w-s1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({ "target_file": "b.txt" }),
                call_id: "call-w-deliver".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "submit".to_string(),
                arguments: serde_json::json!({}),
                call_id: "call-sub-1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "submit".to_string(),
                arguments: serde_json::json!({}),
                call_id: "call-sub-2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = stage_c_controller(gateway);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-SUB",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let submits: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str) == Some("submit")
            })
            .collect();
        assert_eq!(submits.len(), 2, "two submit calls (request + confirm)");
        assert_eq!(
            submits[0].payload["delivery_phase"],
            serde_json::json!("requested")
        );
        assert_eq!(
            submits[1].payload["delivery_phase"],
            serde_json::json!("confirmed")
        );
        {
            let r = controller.blackboard().read();
            let status = r
                .plan
                .delivery_status
                .clone()
                .expect("delivery status rendered into the plan section");
            assert!(
                status.contains("[delivery] 状态: 1 个变更 (output.txt A)"),
                "{status}"
            );
            assert!(
                r.plan.steps[1].status.is_done(),
                "terminal step done after the confirm call: {:?}",
                r.plan.steps[1].status
            );
            let plan_text = crate::epoch::render_section(
                &r.plan,
                &r.edits,
                &r.tool_actions,
                &r.exec,
                &r.actions,
                "plan",
                None,
                None,
            );
            assert!(plan_text.contains("[delivery] 状态:"), "{plan_text}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.3 (2026-08-29)：无 plan 会话 submit
    /// 放行 / 降级为纯状态展示——不再 `no plan in force` 拒绝；两阶段仍
    /// 工作（requested → confirmed），确认轮不推进任何计划步骤。
    #[tokio::test]
    async fn submit_no_plan_renders_status_and_confirms() {
        let dir = test_dir();
        let baseline =
            std::collections::HashMap::from([("src/cache.rs".to_string(), (10u64, 1u64, 0u32))]);
        let after = std::collections::HashMap::from([
            ("src/cache.rs".to_string(), (10u64, 1u64, 0u32)),
            ("output.txt".to_string(), (5u64, 2u64, 0u32)),
        ]);
        let host = DeliveryHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: ok_result(),
            snapshots: std::sync::Mutex::new(VecDeque::from([
                baseline.clone(),
                after.clone(),
                after.clone(),
            ])),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "submit".to_string(),
                arguments: serde_json::json!({}),
                call_id: "call-sub-np-1".to_string(),
            }]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "submit".to_string(),
                arguments: serde_json::json!({}),
                call_id: "call-sub-np-2".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        // 生产路径形态：plan_first 关闭（默认）+ console_default 启用——
        // 会话无 plan 在册。
        let controller =
            AgentLoopController::with_gateway(gateway).with_console_default_enabled(true);
        controller
            .run_turn(
                &host,
                "完成任务",
                "RUN-SUB-NP",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let submits: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str) == Some("submit")
            })
            .collect();
        assert_eq!(
            submits.len(),
            2,
            "two submit calls (request + confirm), no refusal: {:?}",
            event_types(&dir)
        );
        assert!(
            submits
                .iter()
                .all(|e| e.payload["exit_code"] == serde_json::json!(0)),
            "submit must not be refused on a no-plan session"
        );
        assert_eq!(
            submits[0].payload["delivery_phase"],
            serde_json::json!("requested")
        );
        assert_eq!(
            submits[1].payload["delivery_phase"],
            serde_json::json!("confirmed")
        );
        // 无 plan：不推进任何步骤（steps 保持空）；交付状态仍渲染——
        // 无计划批准基线 → 状态降级为"变更清单不可用"（纯状态展示、
        // 非拒绝）。
        {
            let r = controller.blackboard().read();
            assert!(r.plan.steps.is_empty(), "no plan steps may be fabricated");
            let status = r
                .plan
                .delivery_status
                .clone()
                .expect("delivery status rendered");
            assert!(
                status.contains("[delivery] 状态: 变更清单不可用"),
                "{status}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.3（2026-08-29 审查处理）：submit 确认
    /// 消息——有 plan 时保留"末步完成 + 进入最终回答流程"；无 plan（降级
    /// 纯状态展示）不虚构机械流程，明确仅状态展示、未推进计划步骤。
    #[test]
    fn submit_confirm_message_differentiates_plan_and_no_plan() {
        let status = "[delivery] 状态: 变更清单可用";
        let with_plan = AgentLoopController::submit_confirm_message(Some("step-7"), status);
        assert!(with_plan.contains("末步 step-7 完成"), "{with_plan}");
        assert!(with_plan.contains("进入最终回答流程"), "{with_plan}");
        assert!(with_plan.ends_with(status), "{with_plan}");

        let no_plan = AgentLoopController::submit_confirm_message(None, status);
        assert!(no_plan.contains("无计划基线"), "{no_plan}");
        assert!(no_plan.contains("仅状态展示"), "{no_plan}");
        assert!(no_plan.contains("未推进计划步骤"), "{no_plan}");
        assert!(
            !no_plan.contains("进入最终回答流程"),
            "no-plan confirm must not fabricate a mechanical final-answer flow: {no_plan}"
        );
        assert!(!no_plan.contains("末步"), "{no_plan}");
        assert!(no_plan.ends_with(status), "{no_plan}");
    }

    /// MECHANICAL-AUDIT-LAYER (2026-08-24, 设计 §2.1/§3)：submit 为信息
    /// 展示、非硬门——前序步骤未 done 不再拒绝（订单层退役后无机械步骤
    /// 推进机制；step 绑定/顺序转事件留痕，终答前反例自查轮 + 审计报告
    /// 承接「计划完成声明」核对）。首次调用渲染交付状态（phase=requested）。
    #[tokio::test]
    async fn submit_renders_delivery_status_even_when_earlier_steps_pending() {
        let dir = test_dir();
        let host = DeliveryHost {
            journal: JournalRecorder::new(dir.clone()),
            tool_result: ok_result(),
            snapshots: std::sync::Mutex::new(VecDeque::from([std::collections::HashMap::new()])),
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "submit".to_string(),
                arguments: serde_json::json!({}),
                call_id: "call-sub-early".to_string(),
            }]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = stage_c_controller(gateway);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-SUB-EARLY",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let submits = events(&dir)
            .into_iter()
            .filter(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(serde_json::Value::as_str) == Some("submit")
            })
            .collect::<Vec<_>>();
        assert_eq!(
            submits.len(),
            1,
            "submit called once (early, pre-completion)"
        );
        assert_eq!(submits[0].payload["exit_code"].as_u64(), Some(0));
        assert_eq!(
            submits[0].payload["delivery_phase"].as_str(),
            Some("requested")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
