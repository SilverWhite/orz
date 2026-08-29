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
        Ok(ToolResult {
            output: msg.to_string(),
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        })
    }
}
