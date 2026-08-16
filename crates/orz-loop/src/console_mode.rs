//! PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / PLAN_FIRST_BLACKBOARD
//! _DESIGN §7): console/direct dual-mode run-level state machine.
//!
//! - console（每 run 起始）→ direct：唯一入口 = 3 连败助理层故障面后的显式
//!   询问轮，模型选择 switch（§7.2/§7.3）。
//! - direct → console：`console.return_to_console` 单向返回或 run 结束复位。
//! - 故障面计数：receipt ok=false 且 step=verify；或 step=execute 且无业务
//!   exit_code（host 机械故障）。业务非零退出、policy、protocol/registry/
//!   contract、order_stale、step_not_done 不计（§7.2 机械定义）。
//! - direct 只恢复工作工具可见性：权限桥/ACAF/模式门/IPG/预算/探针全部照旧。
//! - 模式为 run 级状态：plan epoch 轮换/黑板旋转不清除；gate_log 中的
//!   transition 记录随黑板保留。

use serde_json::{Value, json};

/// 默认 3 连败阈值（§7.2）；环境变量覆盖。
pub(crate) const DEFAULT_DIRECT_FALLBACK_THRESHOLD: u32 = 3;
pub(crate) const FALLBACK_THRESHOLD_ENV: &str = "ORZ_CONSOLE_DIRECT_FALLBACK_THRESHOLD";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConsoleMode {
    Console,
    Direct,
}

impl ConsoleMode {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ConsoleMode::Console => "console",
            ConsoleMode::Direct => "direct",
        }
    }
}

/// 询问轮模板（§7.3）：`{"decision": "switch"|"stay", "reason": "…"}`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InquiryResponse {
    pub decision: String,
    pub reason: Option<String>,
}

pub(crate) const MAX_INQUIRY_ATTEMPTS: u32 = 2;

/// 解析 + 机械校验询问轮回答（一次重填、仍失败默认 stay）。
pub(crate) fn parse_and_validate_inquiry(text: &str) -> Result<InquiryResponse, Vec<String>> {
    let parsed: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            return Err(vec![
                "invalid JSON — the template must be a JSON object".to_string(),
            ]);
        }
    };
    let mut errors = Vec::new();
    let Some(obj) = parsed.as_object() else {
        return Err(vec![
            "invalid JSON — the template must be a JSON object".to_string(),
        ]);
    };
    let mut decision = None;
    match obj.get("decision") {
        Some(Value::String(s)) if s == "switch" || s == "stay" => {
            decision = Some(s.clone());
        }
        Some(Value::String(_)) => {
            errors.push("decision must be \"switch\" or \"stay\"".to_string());
        }
        Some(_) => errors.push("decision must be a string".to_string()),
        None => errors.push("missing required field: decision".to_string()),
    }
    let mut reason = None;
    match obj.get("reason") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            reason = Some(s.trim().to_string());
        }
        Some(Value::String(_)) => errors.push("reason must be non-empty".to_string()),
        Some(_) => errors.push("reason must be a string".to_string()),
        None => {}
    }
    for key in obj.keys() {
        if key != "decision" && key != "reason" {
            errors.push(format!("unknown field: {key}"));
        }
    }
    if let Some(decision) = decision {
        Ok(InquiryResponse { decision, reason })
    } else {
        Err(errors)
    }
}

/// 询问轮回答的注入提示（§7.3 模板）。
pub(crate) fn inquiry_template_block() -> String {
    "console 助理层连续失败已达阈值——请只回答 JSON 模板 \
     {\"decision\": \"switch\"|\"stay\", \"reason\": \"…\"}：switch=切换 direct \
     受控降级（直接调用工作工具，全链路权限/ACAF/模式门不变）；stay=留在 \
     console（本 run 不再询问）。"
        .to_string()
}

/// 重填反馈块（一次重填；仍失败默认 stay）。
pub(crate) fn inquiry_refill_feedback(errors: &[String]) -> String {
    format!(
        "[CONSOLE_INQUIRY_REFILL v0.1] 回答校验未通过：{}；请只重填询问模板。",
        errors.join("；")
    )
}

/// 判定 receipt 是否属于「助理层故障面」（§7.2 机械定义）。
///
/// 递增：step=verify；step=execute 且无业务 exit_code（host 机械故障、
/// 超时、无结果）。
/// 不递增：step=execute 且 exit_code=Some(非零)（业务失败）；policy；
/// protocol/registry/contract；order_stale；step_not_done。
pub(crate) fn counts_as_assistant_fault(step: &str, upstream: Option<&Value>) -> bool {
    match step {
        "verify" => true,
        "execute" => {
            // 业务失败（exit_code=Some(非零)）→ 不计。
            // 机械口径（2026-08-16 审查收口）：只有整数 exit_code 才算
            // 业务结果；exit_code 缺失/null/非整数 = 无业务结果（host
            // 机械故障面）→ 计为故障。
            if let Some(_exit) = upstream
                .and_then(|u| u.get("exit_code"))
                .and_then(Value::as_i64)
            {
                return false;
            }
            // 脚本内层业务失败（inner upstream 带非零 exit_code）→ 不计。
            if let Some(inner) = upstream.and_then(|u| u.get("upstream"))
                && let Some(_exit) = inner.get("exit_code").and_then(Value::as_i64)
            {
                return false;
            }
            true
        }
        _ => false,
    }
}

/// 每 run 的双模式状态（run 级；run 起始复位、run 结束即弃）。
#[derive(Debug, Clone)]
pub(crate) struct ConsoleModeState {
    pub mode: ConsoleMode,
    /// 连续助理层故障面计数（§7.2；ok=true / 模式切换 / run 开始重置）。
    pub streak: u32,
    /// 本轮询问是否已发起/回答（每 run 至多一次）。
    pub asked: bool,
    /// 当前 direct 切换的 transition_id（console 态为 None）。
    pub transition_id: Option<String>,
    /// 构成故障面连败的订单 id（随询问轮/切换事件承载）。
    pub streak_order_ids: Vec<String>,
    /// direct 态已发生的直接动作 trace_id（`console.step_done` 证据面）。
    pub direct_trace_ids: Vec<String>,
    pub threshold: u32,
}

impl ConsoleModeState {
    pub(crate) fn start(threshold: u32) -> Self {
        Self {
            mode: ConsoleMode::Console,
            streak: 0,
            asked: false,
            transition_id: None,
            streak_order_ids: Vec::new(),
            direct_trace_ids: Vec::new(),
            threshold,
        }
    }

    pub(crate) fn is_direct(&self) -> bool {
        self.mode == ConsoleMode::Direct
    }

    /// 询问触发条件（§7.2/§7.3）：console 态、连续故障 ≥ 阈值、本 run 未问。
    pub(crate) fn inquiry_due(&self) -> bool {
        self.mode == ConsoleMode::Console
            && !self.asked
            && self.streak >= self.threshold
            && !self.streak_order_ids.is_empty()
    }

    /// 订单 receipt 记账：ok=true 重置连败；ok=false 且属故障面则递增。
    pub(crate) fn record_receipt(&mut self, ok: bool, step: &str, upstream: Option<&Value>) {
        if ok {
            self.streak = 0;
            self.streak_order_ids.clear();
            return;
        }
        if counts_as_assistant_fault(step, upstream) {
            self.streak = self.streak.saturating_add(1);
        }
        // 非故障面失败（业务/policy/protocol 等）不清零也不递增——保持
        // 连续语义：只有 ok=true 或模式切换才重置（§7.2）。
    }

    pub(crate) fn push_streak_order(&mut self, order_id: &str) {
        if self.streak_order_ids.len() >= self.threshold.max(1) as usize {
            self.streak_order_ids.remove(0);
        }
        self.streak_order_ids.push(order_id.to_string());
    }

    /// switch：进入 direct，记录 transition_id，连败清零。
    pub(crate) fn switch_to_direct(&mut self, transition_id: String) {
        self.mode = ConsoleMode::Direct;
        self.transition_id = Some(transition_id);
        self.streak = 0;
        self.asked = true;
        self.direct_trace_ids.clear();
    }

    /// stay：留在 console，连败清零，本 run 不再询问。
    pub(crate) fn stay_in_console(&mut self) {
        self.mode = ConsoleMode::Console;
        self.streak = 0;
        self.streak_order_ids.clear();
        self.asked = true;
    }

    /// return_to_console：单向返回 console，连败清零，本 run 不再询问。
    pub(crate) fn return_to_console(&mut self) {
        self.mode = ConsoleMode::Console;
        self.transition_id = None;
        self.streak = 0;
        self.streak_order_ids.clear();
        self.asked = true;
        self.direct_trace_ids.clear();
    }

    /// 登记 direct 直接动作的 trace_id（step_done 证据面）。
    /// 边界（2026-08-16 审查登记）：证据面有界保留最近 100 条 trace；
    /// 超出后最旧 trace 不再可作 `console.step_done` 证据（防证据面
    /// 无界增长；step_done 应紧跟 direct 动作提交）。
    pub(crate) fn record_direct_trace(&mut self, trace_id: &str) {
        if self.direct_trace_ids.len() >= 100 {
            self.direct_trace_ids.remove(0);
        }
        self.direct_trace_ids.push(trace_id.to_string());
    }

    pub(crate) fn has_direct_trace(&self, trace_id: &str) -> bool {
        self.direct_trace_ids.iter().any(|t| t == trace_id)
    }
}

/// v0.2 `console_mode_transition` 事件 payload（§7.4；Schema 先行）。
#[allow(clippy::too_many_arguments)] // mirrors the v0.2 payload's closed field set
pub(crate) fn transition_payload(
    transition_id: &str,
    from: ConsoleMode,
    to: ConsoleMode,
    trigger: &str,
    streak: Option<u32>,
    order_ids: &[String],
    model_decision: &str,
    model_reason: Option<&str>,
    run_id: &str,
    round: u32,
    plan_epoch: u64,
    related_transition_id: Option<&str>,
) -> Value {
    json!({
        "transition_id": transition_id,
        "from": from.as_str(),
        "to": to.as_str(),
        "trigger": trigger,
        "streak": streak,
        "order_ids": order_ids,
        "model_decision": model_decision,
        "model_reason": model_reason,
        "run_id": run_id,
        "round": round,
        "plan_epoch": plan_epoch,
        "related_transition_id": related_transition_id,
    })
}

/// direct 动作的事件盖章（ToolStarted/ToolCompleted 携带 console_mode +
/// transition_id + trace_id；§7.4 事件链关联）。
#[derive(Debug, Clone)]
pub(crate) struct DirectStamp {
    pub transition_id: String,
    pub trace_id: String,
}

impl DirectStamp {
    pub(crate) fn apply(&self, payload: &mut Value) {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("console_mode".to_string(), json!("direct"));
            obj.insert("transition_id".to_string(), json!(self.transition_id));
            obj.insert("trace_id".to_string(), json!(self.trace_id));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(step: &str, exit_code: Option<i64>) -> (String, Option<Value>) {
        let upstream = match exit_code {
            Some(code) => Some(json!({ "exit_code": code })),
            None => None,
        };
        (step.to_string(), upstream)
    }

    #[test]
    fn fault_surface_classifies_mechanically() {
        assert!(counts_as_assistant_fault("verify", None));
        assert!(counts_as_assistant_fault("execute", None));
        assert!(!counts_as_assistant_fault(
            "execute",
            Some(&json!({ "exit_code": 1 }))
        ));
        assert!(!counts_as_assistant_fault("policy", None));
        assert!(!counts_as_assistant_fault("protocol", None));
        assert!(!counts_as_assistant_fault("registry", None));
        assert!(!counts_as_assistant_fault("contract", None));
        assert!(!counts_as_assistant_fault("order_stale", None));
        assert!(!counts_as_assistant_fault("step_not_done", None));
    }

    #[test]
    fn streak_increments_only_on_fault_and_resets_on_ok() {
        let mut state = ConsoleModeState::start(3);
        for i in 1..=3 {
            state.record_receipt(false, "execute", err("execute", None).1.as_ref());
            state.push_streak_order(&format!("ORD-{i}"));
        }
        assert!(state.inquiry_due());
        state.record_receipt(true, "execute", None);
        assert!(!state.inquiry_due());
        assert_eq!(state.streak, 0);
    }

    #[test]
    fn business_failure_does_not_increment() {
        let mut state = ConsoleModeState::start(3);
        state.record_receipt(false, "execute", Some(&json!({ "exit_code": 1 })));
        state.record_receipt(false, "policy", None);
        assert_eq!(state.streak, 0);
        assert!(!state.inquiry_due());
    }

    #[test]
    fn switch_and_return_reset_and_mark_asked() {
        let mut state = ConsoleModeState::start(3);
        for i in 1..=3 {
            state.record_receipt(false, "verify", None);
            state.push_streak_order(&format!("ORD-{i}"));
        }
        assert!(state.inquiry_due());
        state.switch_to_direct("TRANS-1".to_string());
        assert!(state.is_direct());
        assert_eq!(state.transition_id.as_deref(), Some("TRANS-1"));
        assert!(state.asked);
        state.return_to_console();
        assert!(!state.is_direct());
        assert!(state.asked);
    }

    #[test]
    fn inquiry_parse_and_validate() {
        assert_eq!(
            parse_and_validate_inquiry(r#"{"decision":"switch","reason":"r"}"#).unwrap(),
            InquiryResponse {
                decision: "switch".to_string(),
                reason: Some("r".to_string()),
            }
        );
        assert!(parse_and_validate_inquiry(r#"{"decision":"exit"}"#).is_err());
        assert!(parse_and_validate_inquiry("not json").is_err());
    }

    #[test]
    fn direct_trace_evidence_registered() {
        let mut state = ConsoleModeState::start(3);
        state.switch_to_direct("TRANS-1".to_string());
        state.record_direct_trace("call-1");
        assert!(state.has_direct_trace("call-1"));
        assert!(!state.has_direct_trace("call-2"));
    }
}
