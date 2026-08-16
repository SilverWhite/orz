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
    let Some(Value::Object(plan)) = arguments.get("plan") else {
        return PlanVerdict {
            errors: vec!["missing_required_field: plan".to_string()],
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
pub(crate) fn mark_step_in_progress(steps: &mut [PlanStep], idx: usize) {
    if let Some(step) = steps.get_mut(idx) {
        step.status = StepStatus::InProgress;
    }
}

/// receipt ok → done(receipt_id)；direct 证据门 → done(direct evidence)。
pub(crate) fn mark_step_done(
    steps: &mut [PlanStep],
    idx: usize,
    receipt_id: &str,
    direct: Option<DirectStepEvidence>,
) {
    if let Some(step) = steps.get_mut(idx) {
        step.status = StepStatus::Done(DoneEvidence {
            receipt_id: receipt_id.to_string(),
            direct,
        });
    }
}

/// receipt fail → failed(receipt_id)。
pub(crate) fn mark_step_failed(steps: &mut [PlanStep], idx: usize, receipt_id: &str) {
    if let Some(step) = steps.get_mut(idx) {
        step.status = StepStatus::Failed(FailedEvidence {
            receipt_id: receipt_id.to_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
                        "id": "s2",
                        "goal": "实施修复",
                        "actions": [
                            {"step_id": "s2", "do": "workspace.search_replace", "with": {"path": "src/cache.rs"}}
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
}
