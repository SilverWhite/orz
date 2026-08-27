//! Forced-template checkpoint round (ADR-0010 §4.2/§14.16;
//! `docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md`).
//!
//! Orientation and Diagnostic Coverage share one mechanism on the MAIN
//! lane: the fire event is journaled at the safe gap, the checkpoint block
//! is injected, and the NEXT model round is a checkpoint round — no tools
//! are offered, the model must answer the JSON template. Mechanical
//! validation gives one error-feedback re-fill; a second failure degrades
//! by the filled-in parts and is journaled (`checkpoint_response` event).
//! Retrieval lanes keep the legacy fire-and-continue behavior (§14.16:
//! 主车道 only; 检索车道不变).
//!
//! Count semantics (§2.4): a checkpoint round counts as one completed
//! logical model round; the Orientation/DC fire state is committed only
//! after the template round completes (accepted or degraded).

use std::collections::HashSet;
use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::diagnostic_coverage::DebugEpisodeState;
use crate::orientation::{AgentRole, OrientationFireRecord, OrientationSessionState};

/// The JSON template's closed field vocabulary.
pub(crate) const TEMPLATE_FIELDS: [&str; 6] = [
    "task_position",
    "progress_evidence",
    "blockers",
    "missing_evidence",
    "next_action",
    "changed_direction",
];

/// `next_action` closed enum (§2.2).
pub(crate) const NEXT_ACTIONS: [&str; 5] = [
    "continue",
    "adjust",
    "gather_evidence",
    "ask_user",
    "handoff",
];

pub(crate) const TASK_POSITION_MAX_CHARS: usize = 400;
pub(crate) const LIST_MAX_ITEMS: usize = 20;
pub(crate) const ITEM_MAX_CHARS: usize = 200;
/// One error-feedback re-fill, then mechanical degrade (§2.3).
pub(crate) const MAX_ATTEMPTS: u32 = 2;

/// Prefix of the injected re-fill feedback block — registered with
/// `is_injected_block_text` (mechanical injected text, never persisted back
/// into the conversation).
pub(crate) const CHECKPOINT_REFILL_PREFIX: &str = "[CHECKPOINT_REFILL";

/// A pending checkpoint — the fire event was journaled and the block
/// injected; the checkpoint round has not completed yet.
#[derive(Debug, Clone)]
pub(crate) enum PendingCheckpoint {
    Orientation {
        record: OrientationFireRecord,
        attempt: u32,
    },
    DiagnosticCoverage {
        payload: Value,
        attempt: u32,
    },
    /// PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / 设计 §7.3):
    /// console 双模式显式询问轮——3 连败助理层故障面触发后的无工具轮；
    /// 模型只回答 `{"decision": "switch"|"stay", "reason": "…"}` 模板。
    /// 优先级低于 orientation/DC（触发点同 gap，orientation > DC > 本项）。
    ConsoleModeInquiry {
        attempt: u32,
        streak: u32,
        order_ids: Vec<String>,
    },
}

impl PendingCheckpoint {
    pub(crate) fn checkpoint_id(&self) -> &str {
        match self {
            PendingCheckpoint::Orientation { record, .. } => &record.checkpoint_id,
            PendingCheckpoint::DiagnosticCoverage { payload, .. } => payload
                .get("checkpoint_id")
                .and_then(Value::as_str)
                .unwrap_or("DIAG-COV-UNKNOWN"),
            PendingCheckpoint::ConsoleModeInquiry { .. } => "CONSOLE-INQUIRY",
        }
    }

    pub(crate) fn inquiry_kind(&self) -> &'static str {
        match self {
            PendingCheckpoint::Orientation { .. } => "orientation_checkpoint",
            PendingCheckpoint::DiagnosticCoverage { .. } => "diagnostic_coverage_checkpoint",
            PendingCheckpoint::ConsoleModeInquiry { .. } => "console_mode_inquiry",
        }
    }

    pub(crate) fn agent_role(&self) -> AgentRole {
        match self {
            PendingCheckpoint::Orientation { record, .. } => record.agent_role,
            // DC is a main-lane mechanism (§4.6 scope).
            PendingCheckpoint::DiagnosticCoverage { .. } => AgentRole::Main,
            // Console dual-mode inquiry is a main-lane mechanism (§7.3).
            PendingCheckpoint::ConsoleModeInquiry { .. } => AgentRole::Main,
        }
    }

    pub(crate) fn attempt(&self) -> u32 {
        match self {
            PendingCheckpoint::Orientation { attempt, .. }
            | PendingCheckpoint::DiagnosticCoverage { attempt, .. }
            | PendingCheckpoint::ConsoleModeInquiry { attempt, .. } => *attempt,
        }
    }

    pub(crate) fn with_attempt(&self, attempt: u32) -> Self {
        match self {
            PendingCheckpoint::Orientation { record, .. } => PendingCheckpoint::Orientation {
                record: record.clone(),
                attempt,
            },
            PendingCheckpoint::DiagnosticCoverage { payload, .. } => {
                PendingCheckpoint::DiagnosticCoverage {
                    payload: payload.clone(),
                    attempt,
                }
            }
            PendingCheckpoint::ConsoleModeInquiry {
                streak, order_ids, ..
            } => PendingCheckpoint::ConsoleModeInquiry {
                attempt,
                streak: *streak,
                order_ids: order_ids.clone(),
            },
        }
    }
}

/// Parsed template answer — the structured input to the mechanical
/// validation and the evidence-identity cross-check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemplateResponse {
    pub task_position: String,
    pub progress_evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub missing_evidence: Vec<String>,
    pub next_action: String,
    pub changed_direction: bool,
}

/// Result of parsing + validating one checkpoint round answer.
#[derive(Debug, Clone)]
pub(crate) struct TemplateVerdict {
    /// Cleaned known-field JSON object (unknown fields dropped and listed
    /// in `ignored_fields`); `None` when the text is not a JSON object.
    pub object: Option<Value>,
    pub errors: Vec<String>,
    pub ignored_fields: Vec<String>,
    /// Structured view — `Some` whenever a JSON object was parsed (semantic
    /// errors may still be present).
    pub response: Option<TemplateResponse>,
}

/// Outcome of one checkpoint round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckpointRoundOutcome {
    Accepted,
    RefillRequested,
    Degraded { reason: &'static str },
}

/// Evidence-identity cross-check result (§2.3 缓解必做).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CrossCheck {
    pub evidence_identity_found: Vec<String>,
    pub evidence_identity_missing: Vec<String>,
    pub gather_evidence_missing_surface_provided: bool,
}

/// Extract the first JSON object from a model answer — accepts a fenced
/// ```json code block, a raw JSON object, or text containing an object.
pub(crate) fn extract_json_object(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Fenced code block: strip ```json ... ``` fences first.
    let candidate = if trimmed.starts_with("```") {
        let inner = trimmed
            .trim_start_matches('`')
            .strip_prefix("json")
            .unwrap_or_else(|| trimmed.trim_start_matches('`'));
        inner.trim().trim_end_matches('`').trim()
    } else {
        trimmed
    };
    if let Ok(value @ Value::Object(_)) = serde_json::from_str(candidate) {
        return Some(value);
    }
    // Lenient fallback: first '{' to last '}' (text around the object).
    if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}'))
        && start < end
        && let Ok(value @ Value::Object(_)) = serde_json::from_str(&trimmed[start..=end])
    {
        return Some(value);
    }
    None
}

/// Parse + mechanically validate a checkpoint round answer.
pub(crate) fn parse_and_validate(text: &str) -> TemplateVerdict {
    let Some(object) = extract_json_object(text) else {
        return TemplateVerdict {
            object: None,
            errors: vec!["response_is_not_a_json_object".to_string()],
            ignored_fields: Vec::new(),
            response: None,
        };
    };

    // Clean: drop unknown fields (design §2.2 — 非法字段丢弃并记 journal).
    let mut cleaned = Map::new();
    let mut ignored_fields = Vec::new();
    for (key, value) in object.as_object().expect("object checked above") {
        if TEMPLATE_FIELDS.contains(&key.as_str()) {
            cleaned.insert(key.clone(), value.clone());
        } else {
            ignored_fields.push(key.clone());
        }
    }

    let mut errors = Vec::new();
    let mut response = TemplateResponse {
        task_position: String::new(),
        progress_evidence: Vec::new(),
        blockers: Vec::new(),
        missing_evidence: Vec::new(),
        next_action: String::new(),
        changed_direction: false,
    };

    match cleaned.get("task_position") {
        Some(Value::String(s)) if !s.trim().is_empty() => {
            let trimmed = s.trim().to_string();
            response.task_position = trimmed.clone();
            if trimmed.chars().count() > TASK_POSITION_MAX_CHARS {
                errors.push(format!(
                    "task_position exceeds {TASK_POSITION_MAX_CHARS} chars"
                ));
            }
        }
        Some(Value::String(_)) => errors.push("task_position must be non-empty".to_string()),
        Some(_) => errors.push("task_position must be a string".to_string()),
        None => errors.push("missing required field: task_position".to_string()),
    }

    for (key, out) in [
        ("progress_evidence", &mut response.progress_evidence),
        ("blockers", &mut response.blockers),
        ("missing_evidence", &mut response.missing_evidence),
    ] {
        match cleaned.get(key) {
            Some(Value::Array(items)) => {
                if items.len() > LIST_MAX_ITEMS {
                    errors.push(format!("{key} exceeds {LIST_MAX_ITEMS} items"));
                }
                for item in items {
                    match item {
                        Value::String(s) if !s.trim().is_empty() => {
                            let trimmed = s.trim().to_string();
                            if trimmed.chars().count() > ITEM_MAX_CHARS {
                                errors.push(format!("{key} item exceeds {ITEM_MAX_CHARS} chars"));
                            }
                            out.push(trimmed);
                        }
                        Value::String(_) => {
                            errors.push(format!("{key} item must be non-empty"));
                        }
                        _ => errors.push(format!("{key} item must be a string")),
                    }
                }
            }
            Some(_) => errors.push(format!("{key} must be an array")),
            None => {}
        }
    }

    match cleaned.get("next_action") {
        Some(Value::String(s)) => {
            response.next_action = s.clone();
            if !NEXT_ACTIONS.contains(&s.as_str()) {
                errors.push(format!(
                    "next_action must be one of {}",
                    NEXT_ACTIONS.join("|")
                ));
            }
        }
        Some(_) => errors.push("next_action must be a string".to_string()),
        None => errors.push("missing required field: next_action".to_string()),
    }

    match cleaned.get("changed_direction") {
        Some(Value::Bool(b)) => response.changed_direction = *b,
        Some(_) => errors.push("changed_direction must be a boolean".to_string()),
        None => errors.push("missing required field: changed_direction".to_string()),
    }

    // §2.3 缓解必做: `next_action=gather_evidence` must name the missing
    // evidence surface(s) — recorded as a validation error, never silently
    // accepted.
    if response.next_action == "gather_evidence" && response.missing_evidence.is_empty() {
        errors.push("next_action=gather_evidence requires non-empty missing_evidence".to_string());
    }

    TemplateVerdict {
        object: Some(Value::Object(cleaned)),
        errors,
        ignored_fields,
        response: Some(response),
    }
}

/// §2.3 缓解必做 — `progress_evidence` (and, when present, the gathered-
/// evidence missing surface) cross-checked against the session's journal
/// evidence identities. Non-blocking: 强制表达，不验证诚实 — the found /
/// missing split is audit evidence, not a validation gate.
pub(crate) fn cross_check(
    response: Option<&TemplateResponse>,
    known_identities: &HashSet<String>,
) -> CrossCheck {
    let Some(response) = response else {
        return CrossCheck::default();
    };
    let mut found = Vec::new();
    let mut missing = Vec::new();
    for identity in response
        .progress_evidence
        .iter()
        .chain(response.missing_evidence.iter())
    {
        if known_identities.contains(identity) {
            found.push(identity.clone());
        } else {
            missing.push(identity.clone());
        }
    }
    CrossCheck {
        evidence_identity_found: found,
        evidence_identity_missing: missing,
        gather_evidence_missing_surface_provided: response.next_action != "gather_evidence"
            || !response.missing_evidence.is_empty(),
    }
}

/// §2.3 — one error-feedback re-fill, then mechanical degrade.
pub(crate) fn decide_outcome(attempt: u32, errors: &[String]) -> CheckpointRoundOutcome {
    if errors.is_empty() {
        CheckpointRoundOutcome::Accepted
    } else if attempt < MAX_ATTEMPTS {
        CheckpointRoundOutcome::RefillRequested
    } else {
        CheckpointRoundOutcome::Degraded {
            reason: "validation_failed_after_refill",
        }
    }
}

/// The injected re-fill feedback block (one-shot, §2.3).
pub(crate) fn refill_feedback_block(errors: &[String]) -> String {
    format!(
        "{CHECKPOINT_REFILL_PREFIX} v0.1]\n\
         问询模板校验未通过：{}\n\
         请只输出修正后的 JSON 问询模板，不要调用任何工具。\n\
         [/CHECKPOINT_REFILL]",
        errors.join("；")
    )
}

/// Build the v0.2 `checkpoint_response` event payload.
pub(crate) fn checkpoint_response_payload(
    pending: &PendingCheckpoint,
    attempt: u32,
    outcome: &str,
    verdict: &TemplateVerdict,
    cross: &CrossCheck,
    degrade_reason: Option<&str>,
) -> Value {
    serde_json::json!({
        "checkpoint_id": pending.checkpoint_id(),
        "inquiry_family": "neutral",
        "inquiry_kind": pending.inquiry_kind(),
        "agent_role": pending.agent_role().as_str(),
        "attempt": attempt,
        "outcome": outcome,
        "response": verdict.object,
        "validation": {
            "valid": verdict.errors.is_empty(),
            "errors": verdict.errors,
            "ignored_fields": verdict.ignored_fields,
        },
        "cross_check": {
            "evidence_identity_found": cross.evidence_identity_found,
            "evidence_identity_missing": cross.evidence_identity_missing,
            "gather_evidence_missing_surface_provided":
                cross.gather_evidence_missing_surface_provided,
        },
        "degrade_reason": degrade_reason,
    })
}

/// Commit the pending fire AFTER the checkpoint round completed (accepted
/// or degraded — §2.4: only a completed template round resets the count /
/// advances the DC stage). A `None` orientation state (grill/one-shot) is a
/// no-op for the orientation lane.
pub(crate) fn commit_pending(
    pending: PendingCheckpoint,
    orientation: Option<&mut OrientationSessionState>,
    dc_state: &Mutex<DebugEpisodeState>,
) {
    match pending {
        PendingCheckpoint::Orientation { record, .. } => {
            if let Some(state) = orientation {
                state.commit_fire(record.agent_role, &record);
            }
        }
        PendingCheckpoint::DiagnosticCoverage { payload, .. } => {
            crate::diagnostic_coverage::commit_dc_fire(dc_state, &payload);
        }
        // Console inquiry commits nothing to orientation/DC state — the
        // decision is journaled by the transition event (switch/stay) and
        // the mode state is updated by the caller (agent_loop).
        PendingCheckpoint::ConsoleModeInquiry { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> HashSet<String> {
        ["src/cache.rs", "SRC-001", "docs/evidence.md"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    fn valid_json() -> &'static str {
        r#"{
          "task_position": "修复缓存回归",
          "progress_evidence": ["src/cache.rs", "SRC-001"],
          "blockers": [],
          "next_action": "continue",
          "changed_direction": false
        }"#
    }

    #[test]
    fn extracts_fenced_and_raw_json() {
        let fenced = format!("```json\n{}\n```", valid_json());
        assert!(extract_json_object(&fenced).is_some());
        assert!(extract_json_object(valid_json()).is_some());
        assert!(extract_json_object("前文\n{...}\n后文").is_none());
        assert!(extract_json_object("").is_none());
        assert!(extract_json_object("完成").is_none());
    }

    #[test]
    fn valid_template_has_no_errors() {
        let v = parse_and_validate(valid_json());
        assert!(v.errors.is_empty(), "{:?}", v.errors);
        assert!(v.ignored_fields.is_empty());
        let r = v.response.unwrap();
        assert_eq!(r.task_position, "修复缓存回归");
        assert_eq!(r.progress_evidence, vec!["src/cache.rs", "SRC-001"]);
        assert_eq!(r.next_action, "continue");
        assert!(!r.changed_direction);
    }

    #[test]
    fn unknown_fields_dropped_and_recorded() {
        let text = r#"{"task_position":"x","progress_evidence":[],"blockers":[],
            "next_action":"continue","changed_direction":false,"tool_guess":"read_file"}"#;
        let v = parse_and_validate(text);
        assert!(v.errors.is_empty(), "{:?}", v.errors);
        assert_eq!(v.ignored_fields, vec!["tool_guess"]);
        let obj = v.object.unwrap();
        assert!(obj.get("tool_guess").is_none());
    }

    #[test]
    fn missing_required_fields_and_bad_enum_are_errors() {
        let v = parse_and_validate(r#"{"task_position":"x"}"#);
        assert!(
            v.errors
                .iter()
                .any(|e| e.contains("missing required field"))
        );
        let v = parse_and_validate(
            r#"{"task_position":"x","progress_evidence":[],"blockers":[],
                "next_action":"run","changed_direction":false}"#,
        );
        assert!(v.errors.iter().any(|e| e.contains("next_action")));
    }

    #[test]
    fn gather_evidence_requires_missing_surface() {
        let text = r#"{"task_position":"x","progress_evidence":[],"blockers":[],
            "next_action":"gather_evidence","changed_direction":false}"#;
        let v = parse_and_validate(text);
        assert!(
            v.errors.iter().any(|e| e.contains("missing_evidence")),
            "{:?}",
            v.errors
        );
        let ok = r#"{"task_position":"x","progress_evidence":[],"blockers":[],
            "next_action":"gather_evidence","changed_direction":false,
            "missing_evidence":["测试日志"]}"#;
        assert!(parse_and_validate(ok).errors.is_empty());
    }

    #[test]
    fn length_limits_apply_to_trimmed_values() {
        // A 402-char string whose trimmed content is 400 chars passes the
        // length limit (the limit applies to the stored/trimmed value).
        let padded = format!("{}x{}", " ".repeat(401), " ".repeat(401));
        assert_eq!(padded.len(), 803);
        let mut text = format!(
            r#"{{"task_position":"{padded}","progress_evidence":["a"],"blockers":[],"next_action":"continue","changed_direction":false}}"#
        );
        // The padded task_position trims to a single char — under the limit.
        let v = parse_and_validate(&text);
        assert!(v.errors.is_empty(), "{:?}", v.errors);
        assert_eq!(v.response.unwrap().task_position, "x");

        // A list item padded beyond 200 chars but trimmed to 1 is accepted;
        // one whose trimmed content exceeds 200 chars is rejected.
        let item_ok = format!("{}y{}", " ".repeat(201), " ".repeat(201));
        let item_long = "z".repeat(201);
        text = format!(
            r#"{{"task_position":"t","progress_evidence":["{item_ok}","{item_long}"],"blockers":[],"next_action":"continue","changed_direction":false}}"#
        );
        let v = parse_and_validate(&text);
        assert!(
            v.errors.iter().any(|e| e.contains("exceeds 200 chars")),
            "{:?}",
            v.errors
        );
        let r = v.response.unwrap();
        assert_eq!(r.progress_evidence, vec!["y".to_string(), "z".repeat(201)]);
    }

    #[test]
    fn cross_check_splits_found_and_missing() {
        let v = parse_and_validate(valid_json());
        let cross = cross_check(v.response.as_ref(), &known());
        assert_eq!(
            cross.evidence_identity_found,
            vec!["src/cache.rs", "SRC-001"]
        );
        assert!(cross.evidence_identity_missing.is_empty());
        assert!(cross.gather_evidence_missing_surface_provided);
    }

    #[test]
    fn cross_check_records_missing_identities() {
        let text = r#"{"task_position":"x","progress_evidence":["ghost"],
            "blockers":[],"next_action":"continue","changed_direction":false}"#;
        let v = parse_and_validate(text);
        let cross = cross_check(v.response.as_ref(), &known());
        assert_eq!(cross.evidence_identity_missing, vec!["ghost"]);
        assert!(cross.evidence_identity_found.is_empty());
    }

    #[test]
    fn outcome_progression() {
        let errs = vec!["bad".to_string()];
        assert_eq!(
            decide_outcome(1, &errs),
            CheckpointRoundOutcome::RefillRequested
        );
        assert!(matches!(
            decide_outcome(2, &errs),
            CheckpointRoundOutcome::Degraded { reason } if reason == "validation_failed_after_refill"
        ));
        assert_eq!(decide_outcome(2, &[]), CheckpointRoundOutcome::Accepted);
    }

    #[test]
    fn pending_tracks_attempt_and_identity() {
        // R1 (§4.6)：默认阈值 50——测试用显式 7 保持原触发语义。
        let mut orientation = OrientationSessionState::new_with_threshold("sess-1", 7);
        for _ in 0..7 {
            orientation.feed_round(AgentRole::Main);
        }
        let rec = orientation
            .build_fire_record(AgentRole::Main, "RUN-1", "post_tool_batch_gap")
            .unwrap();
        let p = PendingCheckpoint::Orientation {
            record: rec,
            attempt: 1,
        };
        assert_eq!(p.checkpoint_id(), "ORIENT-RUN-1-0000");
        assert_eq!(p.inquiry_kind(), "orientation_checkpoint");
        assert_eq!(p.attempt(), 1);
        assert_eq!(p.with_attempt(2).attempt(), 2);
    }

    #[test]
    fn refill_block_is_registered_prefix() {
        let block = refill_feedback_block(&["task_position missing".to_string()]);
        assert!(block.starts_with(CHECKPOINT_REFILL_PREFIX));
        assert!(block.contains("task_position missing"));
    }
}
