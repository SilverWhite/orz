//! PLAN-FIRST 阶段 A (ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD_DESIGN §3-§5):
//! first-round plan gate — the `plan_write` tool contract, mechanical plan
//! validation, one-refill progression and degrade accounting.
//!
//! 不验证计划“诚实”（§10）：校验只约束结构与长度，不判断语义真伪。

use serde_json::{Map, Value};

use crate::blackboard::{
    DirectStepEvidence, DoneEvidence, FailedEvidence, PlanAction, PlanStep, StepStatus,
};

/// Canonical PLAN-FIRST execution-style framework block (shared with the
/// external AGENTS.md wrapper; see orz-assurance `plan::framework`).
pub(crate) use orz_assurance::plan::framework::PLAN_FIRST_FRAMEWORK_BLOCK;

pub const PLAN_WRITE_TOOL: &str = "plan_write";
pub const BLACKBOARD_READ_TOOL: &str = "blackboard_read";

/// One error-feedback re-fill, then mechanical degrade (§3/§5; the same
/// `MAX_ATTEMPTS=2` semantics as the forced-template checkpoint round).
pub(crate) const MAX_PLAN_ATTEMPTS: u32 = 2;
/// Plan rounds WITHOUT a plan_write submission before mechanical degrade —
/// the gate must never hang the run (不挂死); after this cap the run
/// proceeds without an approved plan and journals `plan_not_submitted`.
pub(crate) const MAX_PLAN_ROUNDS_WITHOUT_SUBMISSION: u32 = 3;

pub(crate) const PLAN_ID_MAX_CHARS: usize = 128;
pub(crate) const GOAL_MAX_CHARS: usize = 2000;
pub(crate) const STEP_ID_MAX_CHARS: usize = 64;
pub(crate) const STEP_GOAL_MAX_CHARS: usize = 2000;
pub(crate) const ACCEPTANCE_MAX_CHARS: usize = 2000;
pub(crate) const EVIDENCE_ITEM_MAX_CHARS: usize = 500;
pub(crate) const MAX_STEPS: usize = 32;
pub(crate) const MAX_ACTIONS_PER_STEP: usize = 8;
pub(crate) const MAX_EVIDENCE_ITEMS_PER_STEP: usize = 16;
/// Total serialized plan budget — bounds blackboard/event rendering and
/// journal payloads (32K chars; mature plan tools stay well below this as a
/// mechanical ceiling, not a quality target).
pub(crate) const MAX_PLAN_TOTAL_CHARS: usize = 32768;
pub(crate) const ACTION_NAME_MAX_CHARS: usize = 128;
/// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the final plan step must be
/// a fixed terminal step (递交/完成) — it never auto-advances on ordinary
/// order receipts; only the explicit `submit` delivery path marks it done.
pub(crate) const TERMINAL_STEP_IDS: &[&str] = &["deliver", "submit"];

/// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): terminal-step predicate —
/// the LAST plan step whose id ∈ {deliver, submit}. Keyed on the terminal
/// ID (not position alone) so legacy/restored plans whose final step has a
/// plain work id keep the pre-S1 step semantics (auto-advance on ordinary
/// orders, `console_step_done` allowed); only plans that follow the fixed
/// 递交/完成 template shape get the submit-only terminal semantics.
pub(crate) fn is_terminal_step(steps: &[crate::blackboard::PlanStep], idx: usize) -> bool {
    steps
        .get(idx)
        .map(|s| idx + 1 == steps.len() && TERMINAL_STEP_IDS.contains(&s.id.as_str()))
        .unwrap_or(false)
}

/// Per-run plan-gate state (local to `run_agent_loop`, main lane only).
#[derive(Debug, Clone)]
pub(crate) struct PlanGateState {
    /// 1 = first submission; 2 = the single refill attempt.
    pub attempt: u32,
    /// Completed plan-rounds without a plan_write call (cap prevents hangs).
    pub rounds_without_submission: u32,
}

impl PlanGateState {
    pub(crate) fn start() -> Self {
        Self {
            attempt: 1,
            rounds_without_submission: 0,
        }
    }
}

/// Mechanical validation verdict for one plan_write submission.
#[derive(Debug, Clone, Default)]
pub(crate) struct PlanVerdict {
    pub errors: Vec<String>,
    pub ignored_fields: Vec<String>,
    pub plan_id: Option<String>,
    pub goal: Option<String>,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlanWriteOutcome {
    Accepted,
    RefillRequested,
    Degraded { reason: &'static str },
}

/// Parse + mechanically validate a `plan_write` plan object (design §5).
pub(crate) fn parse_and_validate_plan(arguments: &Value) -> PlanVerdict {
    let Some(plan) = arguments.get("plan") else {
        return PlanVerdict {
            // P0-E (2026-08-17): the refill error must state the expected
            // shape so a model that serialized the plan as a string can
            // correct it in the single refill.
            errors: vec![
                "missing required field: plan (expected an object with plan_id / goal / steps[])"
                    .to_string(),
            ],
            ..Default::default()
        };
    };
    let Value::Object(plan) = plan else {
        return PlanVerdict {
            errors: vec![format!(
                "plan must be an object with plan_id / goal / steps[] (got {})",
                json_value_type(plan)
            )],
            ..Default::default()
        };
    };

    // Clean top-level fields: plan_id / goal / steps.
    let mut cleaned = Map::new();
    let mut ignored_fields = Vec::new();
    let mut errors = Vec::new();
    for (key, value) in plan {
        if matches!(key.as_str(), "plan_id" | "goal" | "steps") {
            cleaned.insert(key.clone(), value.clone());
        } else {
            ignored_fields.push(key.clone());
        }
    }

    let mut plan_id = None;
    let mut goal = None;
    match cleaned.get("plan_id") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > PLAN_ID_MAX_CHARS {
                errors.push(format!("plan_id exceeds {PLAN_ID_MAX_CHARS} chars"));
            }
            plan_id = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push("plan_id must be non-empty".to_string()),
        Some(_) => errors.push("plan_id must be a string".to_string()),
        None => errors.push("missing required field: plan_id".to_string()),
    }
    match cleaned.get("goal") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > GOAL_MAX_CHARS {
                errors.push(format!("goal exceeds {GOAL_MAX_CHARS} chars"));
            }
            goal = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push("goal must be non-empty".to_string()),
        Some(_) => errors.push("goal must be a string".to_string()),
        None => errors.push("missing required field: goal".to_string()),
    }

    let mut steps = Vec::new();
    match cleaned.get("steps") {
        Some(Value::Array(items)) => {
            if items.is_empty() {
                errors.push("steps must be non-empty".to_string());
            }
            if items.len() > MAX_STEPS {
                errors.push(format!("steps exceeds {MAX_STEPS} items"));
            }
            let mut seen_ids: Vec<String> = Vec::new();
            for (idx, item) in items.iter().enumerate() {
                validate_step(
                    idx,
                    item,
                    &mut steps,
                    &mut seen_ids,
                    &mut errors,
                    &mut ignored_fields,
                );
            }
        }
        Some(_) => errors.push("steps must be an array".to_string()),
        None => errors.push("missing required field: steps".to_string()),
    }
    // AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the plan's LAST step must
    // be a fixed terminal step — the model writes the plan in the template
    // shape (…, deliver/submit) and the delivery gate owns the final step.
    // An empty-id last step already carries its own validation error; skip
    // the extra noise.
    if let Some(last) = steps.last()
        && !last.id.is_empty()
        && !TERMINAL_STEP_IDS.contains(&last.id.as_str())
    {
        errors.push(format!(
            "the final step id must be a terminal step ({}); the last step is \
             the fixed 递交/完成 step and advances only via the submit delivery action",
            TERMINAL_STEP_IDS.join(" / ")
        ));
    }

    // Total plan size ceiling (P3-2, 2026-08-16): even with per-field caps,
    // evidence arrays / `with` objects could otherwise inflate the plan
    // without bound. Serialized JSON chars — the same length currency as
    // the per-round injection budget.
    let total_chars = serde_json::to_string(&cleaned)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    if total_chars > MAX_PLAN_TOTAL_CHARS {
        errors.push(format!(
            "plan exceeds {MAX_PLAN_TOTAL_CHARS} chars (total {total_chars})"
        ));
    }

    PlanVerdict {
        errors,
        ignored_fields,
        plan_id,
        goal,
        steps,
    }
}

/// Human-readable JSON type name for validation messages.
fn json_value_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn validate_step(
    idx: usize,
    item: &Value,
    out: &mut Vec<PlanStep>,
    seen_ids: &mut Vec<String>,
    errors: &mut Vec<String>,
    ignored_fields: &mut Vec<String>,
) {
    let Value::Object(step) = item else {
        errors.push(format!("steps[{idx}] must be an object"));
        return;
    };
    let mut cleaned = Map::new();
    let mut step_ignored = Vec::new();
    for (key, value) in step {
        if matches!(
            key.as_str(),
            "id" | "goal" | "actions" | "acceptance" | "evidence" | "status"
        ) {
            cleaned.insert(key.clone(), value.clone());
        } else {
            step_ignored.push(format!("steps[{idx}].{key}"));
        }
    }
    ignored_fields.extend(step_ignored);

    let mut step_id = None;
    match cleaned.get("id") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > STEP_ID_MAX_CHARS {
                errors.push(format!("steps[{idx}].id exceeds {STEP_ID_MAX_CHARS} chars"));
            }
            if seen_ids.contains(&trimmed) {
                errors.push(format!("steps[{idx}].id duplicate: {trimmed}"));
            }
            seen_ids.push(trimmed.clone());
            step_id = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push(format!("steps[{idx}].id must be non-empty")),
        Some(_) => errors.push(format!("steps[{idx}].id must be a string")),
        None => errors.push(format!("missing required field: steps[{idx}].id")),
    }

    let mut step_goal = None;
    match cleaned.get("goal") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > STEP_GOAL_MAX_CHARS {
                errors.push(format!(
                    "steps[{idx}].goal exceeds {STEP_GOAL_MAX_CHARS} chars"
                ));
            }
            step_goal = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push(format!("steps[{idx}].goal must be non-empty")),
        Some(_) => errors.push(format!("steps[{idx}].goal must be a string")),
        None => errors.push(format!("missing required field: steps[{idx}].goal")),
    }

    let mut acceptance = None;
    match cleaned.get("acceptance") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > ACCEPTANCE_MAX_CHARS {
                errors.push(format!(
                    "steps[{idx}].acceptance exceeds {ACCEPTANCE_MAX_CHARS} chars"
                ));
            }
            acceptance = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push(format!("steps[{idx}].acceptance must be non-empty")),
        Some(_) => errors.push(format!("steps[{idx}].acceptance must be a string")),
        None => errors.push(format!("missing required field: steps[{idx}].acceptance")),
    }

    let mut actions = Vec::new();
    match cleaned.get("actions") {
        Some(Value::Array(items)) => {
            if items.is_empty() {
                errors.push(format!("steps[{idx}].actions must be non-empty"));
            }
            if items.len() > MAX_ACTIONS_PER_STEP {
                errors.push(format!(
                    "steps[{idx}].actions exceeds {MAX_ACTIONS_PER_STEP} items"
                ));
            }
            for (aidx, action) in items.iter().enumerate() {
                validate_action(
                    idx,
                    aidx,
                    action,
                    step_id.as_deref(),
                    &mut actions,
                    errors,
                    ignored_fields,
                );
            }
        }
        Some(_) => errors.push(format!("steps[{idx}].actions must be an array")),
        None => errors.push(format!("missing required field: steps[{idx}].actions")),
    }

    let mut evidence = Vec::new();
    match cleaned.get("evidence") {
        Some(Value::Array(items)) => {
            if items.len() > MAX_EVIDENCE_ITEMS_PER_STEP {
                errors.push(format!(
                    "steps[{idx}].evidence exceeds {MAX_EVIDENCE_ITEMS_PER_STEP} items"
                ));
            }
            for item in items {
                match item {
                    Value::String(s) if !s.trim().is_empty() => {
                        let trimmed = s.trim().to_string();
                        if trimmed.chars().count() > EVIDENCE_ITEM_MAX_CHARS {
                            errors.push(format!(
                                "steps[{idx}].evidence item exceeds {EVIDENCE_ITEM_MAX_CHARS} chars"
                            ));
                        }
                        evidence.push(trimmed);
                    }
                    Value::String(_) => {
                        errors.push(format!("steps[{idx}].evidence item must be non-empty"))
                    }
                    _ => errors.push(format!("steps[{idx}].evidence item must be a string")),
                }
            }
        }
        Some(_) => errors.push(format!("steps[{idx}].evidence must be an array")),
        None => errors.push(format!("missing required field: steps[{idx}].evidence")),
    }

    // status is mechanically managed — a submitted plan may only carry
    // `pending` (or omit the field, which defaults to pending).
    match cleaned.get("status") {
        Some(Value::String(s)) if s == "pending" => {}
        Some(Value::String(_)) => errors.push(format!(
            "steps[{idx}].status must be \"pending\" (statuses are mechanically managed)"
        )),
        Some(_) => errors.push(format!("steps[{idx}].status must be a string")),
        None => {}
    }

    out.push(PlanStep {
        id: step_id.unwrap_or_default(),
        goal: step_goal.unwrap_or_default(),
        actions,
        acceptance: acceptance.unwrap_or_default(),
        evidence,
        status: StepStatus::Pending,
    });
}

fn validate_action(
    step_idx: usize,
    action_idx: usize,
    action: &Value,
    step_id: Option<&str>,
    out: &mut Vec<PlanAction>,
    errors: &mut Vec<String>,
    ignored_fields: &mut Vec<String>,
) {
    let Value::Object(a) = action else {
        errors.push(format!(
            "steps[{step_idx}].actions[{action_idx}] must be an object"
        ));
        return;
    };
    let mut cleaned = Map::new();
    let mut ignored = Vec::new();
    for (key, value) in a {
        if matches!(key.as_str(), "step_id" | "do" | "with") {
            cleaned.insert(key.clone(), value.clone());
        } else {
            ignored.push(format!("steps[{step_idx}].actions[{action_idx}].{key}"));
        }
    }
    ignored_fields.extend(ignored.into_iter().map(|f| format!("{f}")));
    let prefix = format!("steps[{step_idx}].actions[{action_idx}]");

    let mut a_step_id = None;
    match cleaned.get("step_id") {
        Some(Value::String(s)) if !s.trim().is_empty() => a_step_id = Some(s.trim().to_string()),
        Some(Value::String(_)) => errors.push(format!("{prefix}.step_id must be non-empty")),
        Some(_) => errors.push(format!("{prefix}.step_id must be a string")),
        None => errors.push(format!("missing required field: {prefix}.step_id")),
    }
    if let (Some(expected), Some(got)) = (step_id, a_step_id.as_deref())
        && expected != got
    {
        errors.push(format!(
            "{prefix}.step_id must match the owning step id ({expected}); got {got}"
        ));
    }

    let mut do_action = None;
    match cleaned.get("do") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            if trimmed.chars().count() > ACTION_NAME_MAX_CHARS {
                errors.push(format!("{prefix}.do exceeds {ACTION_NAME_MAX_CHARS} chars"));
            }
            do_action = Some(trimmed);
        }
        Some(Value::String(_)) => errors.push(format!("{prefix}.do must be non-empty")),
        Some(_) => errors.push(format!("{prefix}.do must be a string")),
        None => errors.push(format!("missing required field: {prefix}.do")),
    }

    let with = match cleaned.get("with") {
        Some(v @ Value::Object(_)) => v.clone(),
        Some(_) => {
            errors.push(format!("{prefix}.with must be an object"));
            Value::Object(Map::new())
        }
        None => {
            errors.push(format!("missing required field: {prefix}.with"));
            Value::Object(Map::new())
        }
    };

    out.push(PlanAction {
        step_id: a_step_id.unwrap_or_default(),
        do_action: do_action.unwrap_or_default(),
        with,
    });
}

/// §3 — one error-feedback re-fill, then mechanical degrade.
pub(crate) fn decide_outcome(attempt: u32, errors: &[String]) -> PlanWriteOutcome {
    if errors.is_empty() {
        PlanWriteOutcome::Accepted
    } else if attempt < MAX_PLAN_ATTEMPTS {
        PlanWriteOutcome::RefillRequested
    } else {
        PlanWriteOutcome::Degraded {
            reason: "validation_failed_after_refill",
        }
    }
}

/// Build the v0.2 `plan_write` event payload (main lane only).
pub(crate) fn plan_write_payload(
    plan_id: &str,
    goal: &str,
    step_count: usize,
    outcome: &str,
    attempt: u32,
    verdict: &PlanVerdict,
    degrade_reason: Option<&str>,
) -> Value {
    serde_json::json!({
        "plan_id": plan_id,
        "goal": goal,
        "step_count": step_count,
        "outcome": outcome,
        "attempt": attempt,
        "validation": {
            "valid": verdict.errors.is_empty(),
            "errors": verdict.errors,
            "ignored_fields": verdict.ignored_fields,
        },
        "degrade_reason": degrade_reason,
    })
}

/// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §6): 当前可执行
/// 步骤 = 第一个非 done 步骤（pending / in_progress / failed 均可被订单
/// 重新寻址——失败步骤可重试）。全部 done 时返回 None。
pub(crate) fn current_step_index(steps: &[PlanStep]) -> Option<usize> {
    steps.iter().position(|s| !s.status.is_done())
}

/// 步骤门拒绝原因（§6 `step_not_done` 家族）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepGateError {
    /// 无计划步骤（订单不绑定步骤）。
    NoPlanSteps,
    /// 计划在案但订单未带 step_id。
    MissingStepId,
    /// 订单 step_id 不是当前可执行步骤（上一步未 done / 未知 id）。
    StepNotDone { expected: String, got: String },
}

impl StepGateError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            StepGateError::NoPlanSteps => "step_not_done",
            StepGateError::MissingStepId => "step_not_done",
            StepGateError::StepNotDone { .. } => "step_not_done",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            StepGateError::NoPlanSteps => {
                "order refused — no plan steps in force; write a plan first".to_string()
            }
            StepGateError::MissingStepId => {
                "order refused — the order must bind a plan step (step_id); \
                 the step gate requires the current step to be done first"
                    .to_string()
            }
            StepGateError::StepNotDone { expected, got } => format!(
                "order refused — step_not_done: the current step is {expected:?}, \
                 not {got:?}; the previous step must be done (receipt) before \
                 the next order"
            ),
        }
    }
}

/// §6 步骤门：console 订单发放前机械校验——订单必须绑定当前可执行步骤
/// （第一个非 done 步骤），返回该步骤索引。
pub(crate) fn order_step_gate(
    steps: &[PlanStep],
    order_step_id: Option<&str>,
) -> Result<usize, StepGateError> {
    let Some(idx) = current_step_index(steps) else {
        return Err(StepGateError::NoPlanSteps);
    };
    let Some(step_id) = order_step_id else {
        return Err(StepGateError::MissingStepId);
    };
    let current = &steps[idx];
    if current.id != step_id {
        return Err(StepGateError::StepNotDone {
            expected: current.id.clone(),
            got: step_id.to_string(),
        });
    }
    // 上一步必须 done（current 是第一个非 done，故 0..idx 全 done；
    // 显式断言防御索引漂移）。
    if idx > 0 && !steps[idx - 1].status.is_done() {
        return Err(StepGateError::StepNotDone {
            expected: steps[idx - 1].id.clone(),
            got: step_id.to_string(),
        });
    }
    Ok(idx)
}

/// 发放时迁移：pending/failed → in_progress（§6）。
/// 返回是否发生了可见状态变更（PULL 自描述 2026-08-31：调用方据此推进
/// plan 分区版本计数——幂等重标不计数）。
pub(crate) fn mark_step_in_progress(steps: &mut [PlanStep], idx: usize) -> bool {
    if let Some(step) = steps.get_mut(idx) {
        let changed = step.status != StepStatus::InProgress;
        step.status = StepStatus::InProgress;
        return changed;
    }
    false
}

/// receipt ok → done(receipt_id)；direct 证据门 → done(direct evidence)。
pub(crate) fn mark_step_done(
    steps: &mut [PlanStep],
    idx: usize,
    receipt_id: &str,
    direct: Option<DirectStepEvidence>,
) -> bool {
    if let Some(step) = steps.get_mut(idx) {
        let changed = !matches!(&step.status, StepStatus::Done(DoneEvidence { .. }));
        step.status = StepStatus::Done(DoneEvidence {
            receipt_id: receipt_id.to_string(),
            direct,
        });
        return changed;
    }
    false
}

/// receipt fail → failed(receipt_id)。
pub(crate) fn mark_step_failed(steps: &mut [PlanStep], idx: usize, receipt_id: &str) -> bool {
    if let Some(step) = steps.get_mut(idx) {
        let changed = !matches!(&step.status, StepStatus::Failed(FailedEvidence { .. }));
        step.status = StepStatus::Failed(FailedEvidence {
            receipt_id: receipt_id.to_string(),
        });
        return changed;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::{EditRecord, ToolActionRecord};
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::{ModelGateway, Role};
    use crate::host::ToolResult;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

    fn valid_plan() -> serde_json::Value {
        serde_json::json!({
            "plan": {
                "plan_id": "plan-1",
                "goal": "修复缓存回归",
                "steps": [
                    {
                        "id": "s1",
                        "goal": "复现问题",
                        "actions": [
                            {"step_id": "s1", "do": "workspace.read_file", "with": {"path": "src/cache.rs"}}
                        ],
                        "acceptance": "已定位回归点",
                        "evidence": ["src/cache.rs"],
                        "status": "pending"
                    },
                    {
                        "id": "deliver",
                        "goal": "实施修复",
                        "actions": [
                            {"step_id": "deliver", "do": "workspace.search_replace", "with": {"path": "src/cache.rs"}}
                        ],
                        "acceptance": "修复已落地",
                        "evidence": ["src/cache.rs"]
                    }
                ]
            }
        })
    }

    #[test]
    fn valid_plan_has_no_errors() {
        let v = parse_and_validate_plan(&valid_plan());
        assert!(v.errors.is_empty(), "{:?}", v.errors);
        assert!(v.ignored_fields.is_empty());
        assert_eq!(v.plan_id.as_deref(), Some("plan-1"));
        assert_eq!(v.steps.len(), 2);
        assert_eq!(v.steps[0].id, "s1");
        assert_eq!(v.steps[0].actions.len(), 1);
        assert_eq!(v.steps[0].actions[0].do_action, "workspace.read_file");
        assert_eq!(v.steps[1].status, StepStatus::Pending);
    }

    #[test]
    fn missing_plan_is_an_error() {
        let v = parse_and_validate_plan(&serde_json::json!({}));
        assert!(v.errors.iter().any(|e| e.contains("plan")));
    }

    /// P0-E (2026-08-17): a plan serialized as a JSON string must be rejected
    /// with a message that states the expected shape (the single-refill
    /// opportunity depends on it).
    #[test]
    fn plan_as_string_error_states_shape() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": "{\"plan_id\":\"p\",\"goal\":\"g\",\"steps\":[]}"
        }));
        assert_eq!(v.errors.len(), 1, "{:?}", v.errors);
        assert!(
            v.errors[0].contains("plan must be an object with plan_id / goal / steps[]"),
            "{:?}",
            v.errors
        );
        assert!(v.errors[0].contains("got string"), "{:?}", v.errors);
    }

    /// P0-E (2026-08-17): a missing plan error also states the expected shape.
    #[test]
    fn missing_plan_error_states_shape() {
        let v = parse_and_validate_plan(&serde_json::json!({}));
        assert!(
            v.errors[0].contains("missing required field: plan"),
            "{:?}",
            v.errors
        );
        assert!(
            v.errors[0].contains("plan_id / goal / steps[]"),
            "{:?}",
            v.errors
        );
    }

    /// P0-E probe audit (2026-08-17): loose/empty action shapes must all be
    /// mechanically rejected — locks the actions shape against regressions.
    #[test]
    fn action_shape_probe_rejects_loose_shapes() {
        let cases: Vec<(serde_json::Value, &str)> = vec![
            (serde_json::json!([]), "empty array"),
            (serde_json::json!([""]), "empty string item"),
            (
                serde_json::json!(["workspace.read_file"]),
                "bare string item",
            ),
            (serde_json::json!([null]), "null item"),
            (serde_json::json!([{}]), "empty object item"),
            (
                serde_json::json!([{"step_id": "s1", "do": "x"}]),
                "missing with",
            ),
            (
                serde_json::json!([{"step_id": "s1", "do": "x", "with": ""}]),
                "string with",
            ),
            (
                serde_json::json!([{"step_id": "s1", "do": "x", "with": []}]),
                "array with",
            ),
            (
                serde_json::json!([{"step_id": "s1", "do": "", "with": {}}]),
                "empty do",
            ),
            (
                serde_json::json!([{"step_id": "s1", "do": "   ", "with": {}}]),
                "whitespace do",
            ),
            (
                serde_json::json!([{"step_id": "", "do": "x", "with": {}}]),
                "empty step_id",
            ),
            (
                serde_json::json!([{"step_id": "s2", "do": "x", "with": {}}]),
                "step_id mismatch",
            ),
            (serde_json::json!("not-an-array"), "string actions"),
            (serde_json::json!(null), "null actions"),
        ];
        for (actions, label) in cases {
            let plan = serde_json::json!({
                "plan": {
                    "plan_id": "p",
                    "goal": "g",
                    "steps": [
                        {
                            "id": "s1",
                            "goal": "g",
                            "actions": actions,
                            "acceptance": "a",
                            "evidence": [],
                        }
                    ],
                }
            });
            let v = parse_and_validate_plan(&plan);
            assert!(
                !v.errors.is_empty(),
                "{label}: loose shape passed: {actions}"
            );
        }
    }

    /// P0-E probe audit: the minimal well-formed action shape must pass.
    #[test]
    fn action_shape_probe_accepts_minimal_valid() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {
                        "id": "deliver",
                        "goal": "g",
                        "actions": [
                            {"step_id": "deliver", "do": "workspace.read_file", "with": {}}
                        ],
                        "acceptance": "a",
                        "evidence": [],
                    }
                ],
            }
        }));
        assert!(v.errors.is_empty(), "{:?}", v.errors);
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): the plan's final step
    /// must be a fixed terminal step (递交/完成) — mechanically rejected
    /// otherwise (plan_write validation).
    #[test]
    fn final_step_must_be_terminal() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {"id": "s1", "goal": "a", "actions": [{"step_id": "s1", "do": "x", "with": {}}], "acceptance": "a", "evidence": []},
                    {"id": "s2", "goal": "b", "actions": [{"step_id": "s2", "do": "y", "with": {}}], "acceptance": "b", "evidence": []}
                ]
            }
        }));
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("final step id must be a terminal step")),
            "{:?}",
            v.errors
        );
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2): both allowlisted
    /// terminal ids (`deliver` / `submit`) pass the final-step rule.
    #[test]
    fn final_step_submit_is_accepted() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {"id": "s1", "goal": "a", "actions": [{"step_id": "s1", "do": "x", "with": {}}], "acceptance": "a", "evidence": []},
                    {"id": "submit", "goal": "递交", "actions": [{"step_id": "submit", "do": "x", "with": {}}], "acceptance": "a", "evidence": []}
                ]
            }
        }));
        assert!(v.errors.is_empty(), "{:?}", v.errors);
    }

    /// AGENT-DELIVERY-FLOW (2026-08-23, 设计 §2.2 + 审查处理 O2): the
    /// terminal predicate is ID-keyed — the LAST step whose id is
    /// `deliver`/`submit` is terminal; a legacy/restored plan whose final
    /// step has a plain work id keeps the pre-S1 step semantics.
    #[test]
    fn terminal_step_predicate_is_id_keyed() {
        let step = |id: &str| crate::blackboard::PlanStep {
            id: id.to_string(),
            goal: String::new(),
            actions: Vec::new(),
            acceptance: String::new(),
            evidence: Vec::new(),
            status: crate::blackboard::StepStatus::Pending,
        };
        // New-shape plan: last step deliver → terminal; earlier step not.
        let steps = vec![step("s1"), step("deliver")];
        assert!(super::is_terminal_step(&steps, 1));
        assert!(!super::is_terminal_step(&steps, 0));
        // New-shape plan: last step submit → terminal.
        let steps = vec![step("s1"), step("submit")];
        assert!(super::is_terminal_step(&steps, 1));
        // Legacy-shape plan: last step is a plain work id → NOT terminal.
        let steps = vec![step("s1"), step("s2")];
        assert!(!super::is_terminal_step(&steps, 1));
        assert!(!super::is_terminal_step(&steps, 0));
        // Terminal id on a NON-last position is not the fixed terminal step.
        let steps = vec![step("deliver"), step("s2")];
        assert!(!super::is_terminal_step(&steps, 0));
        // Out-of-range index.
        assert!(!super::is_terminal_step(&steps, 2));
    }

    #[test]
    fn bad_structure_and_unknown_fields_are_reported() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "",
                "goal": "x",
                "steps": [
                    {
                        "id": "s1",
                        "goal": "g",
                        "actions": [
                            {"step_id": "s2", "do": "", "with": "not-an-object"}
                        ],
                        "acceptance": "a",
                        "evidence": [],
                        "status": "done",
                        "extra": 1
                    }
                ],
                "extra_top": true
            }
        }));
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("plan_id must be non-empty"))
        );
        assert!(v.errors.iter().any(|e| e.contains("step_id must match")));
        assert!(v.errors.iter().any(|e| e.contains("do must be non-empty")));
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("with must be an object"))
        );
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("status must be \"pending\""))
        );
        assert!(v.ignored_fields.contains(&"extra_top".to_string()));
    }

    #[test]
    fn duplicate_and_empty_steps_are_errors() {
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": []
            }
        }));
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("steps must be non-empty"))
        );

        let dup = serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {"id": "s1", "goal": "a", "actions": [{"step_id": "s1", "do": "x", "with": {}}], "acceptance": "a", "evidence": []},
                    {"id": "s1", "goal": "b", "actions": [{"step_id": "s1", "do": "y", "with": {}}], "acceptance": "b", "evidence": []}
                ]
            }
        });
        let v = parse_and_validate_plan(&dup);
        assert!(v.errors.iter().any(|e| e.contains("duplicate")));
    }

    #[test]
    fn evidence_item_cap_is_enforced() {
        let evidence: Vec<serde_json::Value> = (0..=MAX_EVIDENCE_ITEMS_PER_STEP)
            .map(|i| serde_json::json!(format!("e{i}")))
            .collect();
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {
                        "id": "s1",
                        "goal": "g1",
                        "actions": [{"step_id": "s1", "do": "x", "with": {}}],
                        "acceptance": "a",
                        "evidence": evidence,
                    }
                ]
            }
        }));
        assert!(
            v.errors.iter().any(|e| e.contains("evidence exceeds")),
            "{:?}",
            v.errors
        );
    }

    #[test]
    fn action_count_cap_is_enforced() {
        let actions: Vec<serde_json::Value> = (0..=MAX_ACTIONS_PER_STEP)
            .map(|_| serde_json::json!({"step_id": "s1", "do": "x", "with": {}}))
            .collect();
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": "g",
                "steps": [
                    {
                        "id": "s1",
                        "goal": "g1",
                        "actions": actions,
                        "acceptance": "a",
                        "evidence": [],
                    }
                ]
            }
        }));
        assert!(
            v.errors.iter().any(|e| e.contains("actions exceeds")),
            "{:?}",
            v.errors
        );
    }

    #[test]
    fn total_plan_size_cap_is_enforced() {
        let big_goal = "x".repeat(MAX_PLAN_TOTAL_CHARS + 1);
        let v = parse_and_validate_plan(&serde_json::json!({
            "plan": {
                "plan_id": "p",
                "goal": big_goal,
                "steps": [
                    {
                        "id": "s1",
                        "goal": "g1",
                        "actions": [{"step_id": "s1", "do": "x", "with": {}}],
                        "acceptance": "a",
                        "evidence": [],
                    }
                ]
            }
        }));
        assert!(
            v.errors.iter().any(|e| e.contains("plan exceeds")),
            "{:?}",
            v.errors
        );
    }

    #[test]
    fn outcome_progression() {
        let errs = vec!["bad".to_string()];
        assert_eq!(decide_outcome(1, &errs), PlanWriteOutcome::RefillRequested);
        assert!(matches!(
            decide_outcome(2, &errs),
            PlanWriteOutcome::Degraded { reason } if reason == "validation_failed_after_refill"
        ));
        assert_eq!(decide_outcome(2, &[]), PlanWriteOutcome::Accepted);
    }

    #[test]
    fn payload_matches_schema_shape() {
        let v = parse_and_validate_plan(&valid_plan());
        let payload = plan_write_payload(
            v.plan_id.as_deref().unwrap_or(""),
            v.goal.as_deref().unwrap_or(""),
            v.steps.len(),
            "accepted",
            1,
            &v,
            None,
        );
        assert_eq!(payload["outcome"], "accepted");
        assert_eq!(payload["step_count"], 2);
        assert_eq!(payload["validation"]["valid"], true);
        assert!(payload.get("degrade_reason").unwrap().is_null());
    }

    /// A4 (2026-08-08) + 2026-08-18 (ADR-0010 §14.25 项 1): an ingested
    /// plan populates the blackboard plan section (goal + steps, first step
    /// in-progress); the resident status line NO LONGER lives in the system
    /// prompt — it is appended as a trailing user message (cache discipline).
    #[tokio::test]
    async fn with_plan_status_line_is_trailing_user_message() {
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
            ScriptedResponse::text("调查完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let controller = AgentLoopController::with_gateway(gateway).with_plan(
            "PLAN-TEST-A".to_string(),
            1,
            "修复 bug".to_string(),
            vec!["调查".to_string(), "实施".to_string()],
        );

        // Plan section written (feeds blackboard_read plan partition too).
        let bb = controller.blackboard();
        {
            let r = bb.read();
            assert_eq!(r.plan.goal.as_deref(), Some("修复 bug"));
            assert_eq!(r.plan.steps.len(), 2);
            assert_eq!(
                r.plan.steps[0].status,
                crate::blackboard::StepStatus::InProgress
            );
            assert_eq!(
                r.plan.steps[1].status,
                crate::blackboard::StepStatus::Pending
            );
        }

        controller
            .run_turn(
                &host,
                "请调查",
                "RUN-PLAN-ST",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let received = fake.received_requests();
        assert!(!received.is_empty(), "at least one request");
        for request in &received {
            assert!(
                !request.system.contains("[任务状态"),
                "status line must not live in the system prompt: {}",
                request.system
            );
        }
        let status_texts: Vec<String> = received
            .iter()
            .flat_map(|r| r.messages.iter())
            .filter(|m| matches!(m.role, Role::User) && m.content.contains("[任务状态 v0.1]"))
            .map(|m| m.content.clone())
            .collect();
        assert!(
            !status_texts.is_empty(),
            "status line missing from messages"
        );
        assert!(status_texts[0].contains("目标: 修复 bug"));
        assert!(status_texts[0].contains("当前第 1 步 [step-1]「调查」"));
        assert!(status_texts[0].contains("[/任务状态]"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// v1.15 (2026-08-14): a new plan_id rotates the blackboard — the old
    /// epoch's plan/edits/tool_actions/exec are archived to the configured
    /// archive dir, the epoch-scoped partitions are cleared, and the new
    /// plan carries plan_id + plan_epoch. A same-plan_id approval is a
    /// revision: no rotation, no clearing, same epoch.
    #[test]
    fn plan_epoch_rotation_archives_and_revision_keeps_board() {
        let dir = test_dir().join("gsa").join("blackboard");
        let mut controller =
            AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
                .with_blackboard_archive_dir(Some(dir.clone()));
        controller = controller.with_plan(
            "PLAN-ROT-A".to_string(),
            1,
            "任务A".to_string(),
            vec!["步骤A".to_string()],
        );
        {
            let mut bb = controller.blackboard().write();
            bb.edits.push(EditRecord {
                file: "a.py".into(),
                old_lines: 1,
                new_lines: 2,
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
            bb.tool_actions.push(ToolActionRecord {
                category: "read".to_string(),
                tool: "read_file".into(),
                timestamp: "2026-08-14T00:00:00Z".into(),
            });
        }
        // The current epoch snapshot is persisted at approval (the restore
        // entry), even before any rotation.
        assert!(dir.join("epoch-1.json").exists(), "current epoch persisted");
        let first: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("epoch-1.json")).unwrap())
                .unwrap();
        assert_eq!(first["plan_id"], "PLAN-ROT-A");

        // New plan_id → epoch 2, old epoch-1 archived, work partitions cleared.
        controller = controller.with_plan(
            "PLAN-ROT-B".to_string(),
            2,
            "任务B".to_string(),
            vec!["步骤B".to_string()],
        );
        let bb = controller.blackboard();
        {
            let r = bb.read();
            assert_eq!(r.plan.plan_id.as_deref(), Some("PLAN-ROT-B"));
            assert_eq!(r.plan.plan_epoch, 2);
            assert!(r.edits.is_empty());
            assert!(r.tool_actions.is_empty());
        }
        let archived = dir.join("epoch-1.json");
        assert!(archived.exists(), "old epoch must be archived");
        let snapshot: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&archived).unwrap()).unwrap();
        assert_eq!(snapshot["plan_id"], "PLAN-ROT-A");
        assert_eq!(snapshot["plan_epoch"], 1);
        assert_eq!(snapshot["edits"][0]["file"], "a.py");

        // Same plan_id → revision: no rotation, no archive, same epoch.
        controller = controller.with_plan(
            "PLAN-ROT-B".to_string(),
            2,
            "任务B（修订）".to_string(),
            vec!["步骤B".to_string(), "步骤B2".to_string()],
        );
        let r = controller.blackboard().read();
        assert_eq!(r.plan.plan_epoch, 2);
        assert_eq!(r.plan.goal.as_deref(), Some("任务B（修订）"));
        assert_eq!(r.plan.steps.len(), 2);
        // The revision refreshes the CURRENT epoch snapshot (restore sees
        // the latest approved plan text) without creating a new epoch.
        let current: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("epoch-2.json")).unwrap())
                .unwrap();
        assert_eq!(current["plan_epoch"], 2);
        assert_eq!(current["plan"]["goal"], "任务B（修订）");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// F7 (2026-08-15, BACKLOG 6e 复查遗留): an epoch archive write failure
    /// (rotated old snapshot or current-epoch persistence) is journaled as
    /// `epoch_archive_write_failed` at run start — never only a warn.
    #[tokio::test]
    async fn epoch_archive_write_failure_is_journaled() {
        let dir = test_dir();
        let blocked = test_dir();
        // `.gsa` exists as a FILE → create_dir_all(.gsa/blackboard) fails.
        std::fs::write(blocked.join(".gsa"), "occupied").unwrap();
        let archive_dir = blocked.join(".gsa").join("blackboard");
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let mut controller = AgentLoopController::with_gateway(gateway)
            .with_blackboard_archive_dir(Some(archive_dir.clone()));
        // PLAN-ARCH-A: the current snapshot write fails (queued current).
        controller = controller.with_plan(
            "PLAN-ARCH-A".to_string(),
            1,
            "任务A".to_string(),
            vec!["步骤A".to_string()],
        );
        // PLAN-ARCH-B: the rotated old snapshot AND the new current
        // snapshot writes fail (queued rotated + current).
        controller = controller.with_plan(
            "PLAN-ARCH-B".to_string(),
            2,
            "任务B".to_string(),
            vec!["步骤B".to_string()],
        );
        controller
            .run_turn(&host, "归档失败", "RUN-BAF", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let all_events = events(&dir);
        let failed: Vec<&serde_json::Value> = all_events
            .iter()
            .filter(|e| e.event_type == EventType::EpochArchiveWriteFailed)
            .map(|e| &e.payload)
            .collect();
        assert_eq!(failed.len(), 3, "{all_events:?}");
        // Deterministic order: current(1) → rotated(1) → current(2).
        assert_eq!(failed[0]["kind"], "current");
        assert_eq!(failed[0]["plan_epoch"], 1);
        assert_eq!(failed[1]["kind"], "rotated");
        assert_eq!(failed[1]["plan_epoch"], 1);
        assert_eq!(failed[2]["kind"], "current");
        assert_eq!(failed[2]["plan_epoch"], 2);
        let archive_dir_text = archive_dir.display().to_string();
        for payload in failed {
            assert_eq!(payload["attempts"], 3);
            assert_eq!(
                payload["archive_dir"].as_str(),
                Some(archive_dir_text.as_str())
            );
        }

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&blocked);
    }

    /// v1.15⑧ (2026-08-15): the plan_id ↔ plan_epoch mapping is one-to-one —
    /// same plan_id with a different epoch, or a new plan_id without a
    /// strictly greater epoch, is rejected before any mutation.
    #[test]
    fn try_with_plan_enforces_epoch_identity_invariants() {
        let controller = AgentLoopController::with_gateway(Arc::new(FakeProvider::new(Vec::new())))
            .with_plan(
                "PLAN-INV-A".to_string(),
                1,
                "任务A".to_string(),
                vec!["步骤A".to_string()],
            );
        // Same plan_id, different epoch → rejected.
        let err = match controller.try_with_plan(
            "PLAN-INV-A".to_string(),
            2,
            "任务A（错误修订）".to_string(),
            vec!["步骤A".to_string()],
        ) {
            Err(e) => e,
            Ok(_) => panic!("same plan_id with a different epoch must be rejected"),
        };
        assert!(
            matches!(
                err,
                crate::blackboard::PlanEpochError::SamePlanEpochMismatch {
                    expected: 1,
                    got: 2,
                    ..
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn rotate_to_plan_rejects_zero_or_non_advancing_epochs() {
        let mut bb = crate::blackboard::Blackboard::new();
        bb.plan.plan_id = Some("PLAN-INV-A".into());
        bb.plan.plan_epoch = 1;
        // New plan_id must strictly advance.
        let err = bb
            .rotate_to_plan(
                "PLAN-INV-B".into(),
                1,
                "B".into(),
                vec!["b".into()],
                "2026-08-15T00:00:00Z",
            )
            .unwrap_err();
        assert_eq!(
            err,
            crate::blackboard::PlanEpochError::NewPlanEpochNotGreater {
                current_epoch: 1,
                got: 1,
            }
        );
        // Zero epoch is never valid.
        let err = bb
            .rotate_to_plan(
                "PLAN-INV-C".into(),
                0,
                "C".into(),
                vec!["c".into()],
                "2026-08-15T00:00:00Z",
            )
            .unwrap_err();
        assert_eq!(err, crate::blackboard::PlanEpochError::ZeroEpoch);
        // Violations leave the board untouched.
        assert_eq!(bb.plan.plan_id.as_deref(), Some("PLAN-INV-A"));
        assert_eq!(bb.plan.plan_epoch, 1);
    }

    // ── PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17) ─────────────

    /// 首轮计划轮硬门：首轮请求 header 工具面 = blackboard_read +
    /// plan_write；有效计划落黑板 plan epoch（`plan_write` 事件 accepted）
    /// 并放行后续轮次。
    #[tokio::test]
    async fn plan_first_round_surface_is_plan_only_and_valid_plan_lands() {
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
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF1",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let initial = events
            .iter()
            .find(|e| {
                e.event_type == EventType::RequestHeaderChange
                    && e.payload.get("reason").and_then(|v| v.as_str()) == Some("initial")
            })
            .expect("initial request header");
        let tools: Vec<String> = initial.payload["tools"]
            .as_array()
            .expect("tools list")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(tools, vec!["blackboard_read", "plan_write"], "{tools:?}");

        let writes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::PlanWrite)
            .collect();
        assert_eq!(writes.len(), 1, "{:?}", event_types(&dir));
        assert_eq!(writes[0].payload["outcome"].as_str(), Some("accepted"));
        assert_eq!(writes[0].payload["step_count"].as_u64(), Some(2));
        assert_eq!(writes[0].payload["attempt"].as_u64(), Some(1));
        assert_eq!(
            controller.blackboard().read().plan.plan_id.as_deref(),
            Some("plan-1")
        );
        assert!(controller.blackboard().read().plan.plan_epoch >= 1);
        assert_eq!(controller.blackboard().read().plan.steps.len(), 2);
        let completed = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("plan_write")
            })
            .expect("plan_write ToolCompleted");
        assert_eq!(completed.payload["exit_code"].as_u64(), Some(0));
        assert!(
            completed.payload.get("status").is_none()
                && completed.payload.get("plan_id").is_none()
                && completed.payload.get("plan_epoch").is_none()
                && completed.payload.get("outcome").is_none(),
            "plan_write ToolCompleted must stay in the generic contract shape: {:?}",
            completed.payload
        );
        assert_eq!(
            events.last().unwrap().event_type,
            EventType::RunFinished,
            "run continues after the accepted plan round"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 一次错误反馈重填：首次无效计划 → refill_requested；第二次有效计划
    /// → accepted；两次提交都留痕。
    #[tokio::test]
    async fn plan_first_round_refills_once_then_accepts() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-bad-1", invalid_plan_json())]),
            ScriptedResponse::tool_calls(vec![plan_write_call("call-ok-2", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF2",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let writes: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::PlanWrite)
            .collect();
        let outcomes: Vec<Option<&str>> = writes
            .iter()
            .map(|e| e.payload["outcome"].as_str())
            .collect();
        assert_eq!(outcomes, vec![Some("refill_requested"), Some("accepted")]);
        let attempts: Vec<u64> = writes
            .iter()
            .map(|e| e.payload["attempt"].as_u64().unwrap())
            .collect();
        assert_eq!(attempts, vec![1, 2]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 两次无效提交 → 机械降级（validation_failed_after_refill 留痕），
    /// 门解除、运行继续，不挂死。
    #[tokio::test]
    async fn plan_first_round_invalid_twice_degrades_and_continues() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-bad-1", invalid_plan_json())]),
            ScriptedResponse::tool_calls(vec![plan_write_call("call-bad-2", invalid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF3",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let writes: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::PlanWrite)
            .collect();
        let outcomes: Vec<Option<&str>> = writes
            .iter()
            .map(|e| e.payload["outcome"].as_str())
            .collect();
        assert_eq!(outcomes, vec![Some("refill_requested"), Some("degraded")]);
        assert_eq!(
            writes[1].payload["degrade_reason"].as_str(),
            Some("validation_failed_after_refill")
        );
        assert!(controller.blackboard().read().plan.plan_id.is_none());
        assert_eq!(
            events(&dir).last().unwrap().event_type,
            EventType::RunFinished
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 计划轮内不允许任何其他工具：read_file 在首轮被机械拒绝
    /// （plan_round_tool_denied），随后有效计划仍可落板。
    #[tokio::test]
    async fn plan_first_round_refuses_non_plan_tools() {
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
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF4",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let refused = events(&dir)
            .into_iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("plan_round_tool_denied")
            })
            .expect("plan-round refusal journaled");
        assert_eq!(
            refused.payload.get("tool").and_then(|v| v.as_str()),
            Some("read_file")
        );
        assert_eq!(refused.payload["exit_code"].as_u64(), Some(1));
        let refused_started = events(&dir)
            .into_iter()
            .find(|e| {
                e.event_type == EventType::ToolStarted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")
            })
            .expect("plan-round refusal ToolStarted journaled");
        assert!(
            refused_started.payload.get("target").is_none(),
            "main-lane ToolStarted must not carry a retrieval target: {:?}",
            refused_started.payload
        );
        assert_eq!(
            controller.blackboard().read().plan.plan_id.as_deref(),
            Some("plan-1")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 连续三轮不提交计划 → plan_not_submitted 机械降级并放行（不挂死）。
    #[tokio::test]
    async fn plan_first_round_no_submission_degrades_after_cap() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("还在思考"),
            ScriptedResponse::text("还在思考"),
            ScriptedResponse::text("还在思考"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF5",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let writes: Vec<_> = events(&dir)
            .into_iter()
            .filter(|e| e.event_type == EventType::PlanWrite)
            .collect();
        assert_eq!(writes.len(), 1, "{:?}", event_types(&dir));
        assert_eq!(writes[0].payload["outcome"].as_str(), Some("degraded"));
        assert_eq!(
            writes[0].payload["degrade_reason"].as_str(),
            Some("plan_not_submitted")
        );
        assert_eq!(
            events(&dir).last().unwrap().event_type,
            EventType::RunFinished
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 已有已批准计划的会话（恢复/后续 run）不重复触发计划门——首轮工具
    /// 面即正常探针面。
    #[tokio::test]
    async fn planned_session_skips_plan_gate() {
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
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway)
            .with_plan_first_enabled(true)
            .with_plan("PLAN-1".into(), 1, "旧目标".into(), vec!["旧步骤".into()]);
        controller
            .run_turn(&host, "继续", "RUN-PF6", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        assert!(
            !events.iter().any(|e| e.event_type == EventType::PlanWrite),
            "no plan gate for a session that already has an approved plan"
        );
        let initial = events
            .iter()
            .find(|e| {
                e.event_type == EventType::RequestHeaderChange
                    && e.payload.get("reason").and_then(|v| v.as_str()) == Some("initial")
            })
            .expect("initial request header");
        let tools: Vec<String> = initial.payload["tools"]
            .as_array()
            .expect("tools list")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        // 计划已存在 → 不触发计划门：read_file 直接执行（无 plan_write
        // 事件）。工具面本身在 plan_first 会话恒含 plan_write（声明面随
        // plan_first_enabled 收敛），故用执行证据而非工具列表断言。
        assert!(
            events
                .iter()
                .any(|e| e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")),
            "pre-approved plan skips the gate — the work tool executes directly"
        );
        // THIN-HARNESS-REDESIGN R1 (§4.1)：compaction_whitelist_add 封存——
        // 正常探针面也不含边界工具（TestHost 注册面为空，blackboard_read
        // 由控制器常驻声明）。
        assert!(
            !tools.contains(&"compaction_whitelist_add".to_string()),
            "{tools:?}"
        );
        assert!(tools.contains(&"blackboard_read".to_string()), "{tools:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── 2026-08-16 阶段 A 审查收口（用户裁决 + P1/P2/P3 修复）────────

    /// 用户裁决：计划轮不消耗 tool-round 预算。max=1 时计划落板后仍有一个
    /// 完整工具轮可执行 read_file（若计划轮计 1，第二轮会预算耗尽被拒）。
    #[tokio::test]
    async fn plan_first_round_does_not_consume_tool_round_budget() {
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
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway)
            .with_plan_first_enabled(true)
            .with_max_tool_rounds(1);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF-BUDGET",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let read_completed = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("tool").and_then(|v| v.as_str()) == Some("read_file")
            })
            .expect("read_file must execute after the plan round (budget preserved)");
        assert_eq!(read_completed.payload["exit_code"].as_u64(), Some(0));
        assert_eq!(events.last().unwrap().event_type, EventType::RunFinished);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P3-5：同一计划轮最多一次 plan_write——同轮第二次提交机械拒绝
    /// （plan_write_already_submitted），不落板、不留 PlanWrite 事件。
    #[tokio::test]
    async fn plan_first_round_rejects_second_plan_write_in_same_round() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![
                plan_write_call("call-plan-1", valid_plan_json()),
                plan_write_call("call-plan-2", valid_plan_json()),
            ]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway).with_plan_first_enabled(true);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF-DUP",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        let events = events(&dir);
        let writes: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::PlanWrite)
            .collect();
        assert_eq!(writes.len(), 1, "only one PlanWrite event per plan round");
        assert_eq!(writes[0].payload["outcome"].as_str(), Some("accepted"));
        let denied = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("plan_write_already_submitted")
            })
            .expect("second plan_write in the same round must be refused");
        assert_eq!(denied.payload["exit_code"].as_u64(), Some(1));
        assert_eq!(
            controller.blackboard().read().plan.plan_id.as_deref(),
            Some("plan-1")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// P3-4：plan_first 关闭时 plan_write 不声明且调用被拒
    /// （plan_write_disabled），不产生 PlanWrite 事件、不落板。
    #[tokio::test]
    async fn plan_write_disabled_refuses_when_gate_off() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-1", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = AgentLoopController::with_gateway(gateway);
        controller
            .run_turn(
                &host,
                "修复缓存回归",
                "RUN-PF-OFF",
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
            !events.iter().any(|e| e.event_type == EventType::PlanWrite),
            "no PlanWrite event when the gate is disabled"
        );
        let denied = events
            .iter()
            .find(|e| {
                e.event_type == EventType::ToolCompleted
                    && e.payload.get("error").and_then(|v| v.as_str())
                        == Some("plan_write_disabled")
            })
            .expect("plan_write must be refused when the gate is disabled");
        assert_eq!(denied.payload["exit_code"].as_u64(), Some(1));
        assert!(controller.blackboard().read().plan.plan_id.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// D2 全覆盖（2026-08-16 审查收口）：plan_first 会话的系统提示词注入
    /// <plan_first_framework>（不依赖 AGENTS.md）；关闭态不注入。
    #[tokio::test]
    async fn plan_first_framework_block_follows_gate_switch() {
        let enabled_dir = test_dir();
        let enabled_fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![plan_write_call("call-plan-fw", valid_plan_json())]),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let enabled_gateway: Arc<dyn ModelGateway> = enabled_fake.clone();
        let enabled_controller = AgentLoopController::with_gateway(enabled_gateway)
            .with_plan_first_enabled(true)
            .with_max_tool_rounds(1);
        enabled_controller
            .run_turn(
                &TestHost {
                    journal: JournalRecorder::new(enabled_dir.clone()),
                    tool_result: None,
                },
                "hi",
                "RUN-PF-FW-ON",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let received = enabled_fake.received_requests();
        assert!(
            received[0].system.contains("<plan_first_framework>"),
            "plan-first system prompt must carry the framework block"
        );
        let _ = std::fs::remove_dir_all(&enabled_dir);

        let off_dir = test_dir();
        let off_fake = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let off_gateway: Arc<dyn ModelGateway> = off_fake.clone();
        let off_controller = AgentLoopController::with_gateway(off_gateway).with_max_tool_rounds(1);
        off_controller
            .run_turn(
                &TestHost {
                    journal: JournalRecorder::new(off_dir.clone()),
                    tool_result: None,
                },
                "hi",
                "RUN-PF-FW-OFF",
                MANIFEST,
                0,
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let received = off_fake.received_requests();
        assert!(
            !received[0].system.contains("<plan_first_framework>"),
            "legacy (gate-off) system prompt must not carry the framework block"
        );
        let _ = std::fs::remove_dir_all(&off_dir);
    }
}
