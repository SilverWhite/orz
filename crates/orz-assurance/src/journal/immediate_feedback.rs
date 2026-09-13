//! 0ac S3 (2026-09-13): immediate-feedback rule families — the judge rules
//! that S2's machine contract reserved for S3
//! (`IMMEDIATE_RESULT_DELIVERY_AND_STREAMING_RETRIEVAL_DESIGN §10.3` item 2).
//!
//! The S2 landing (2026-09-13) froze the *shapes*: `retrieval_progress`,
//! `retrieval_result_segment`, `result_delivered`, the `tool_completed.cause`
//! field and the `retrieval_family` probe reading. This module is the
//! Rust-side mechanical judge over those shapes — pure functions over raw
//! journal events, in the style of [`super::families`] / [`super::families_s2c`],
//! gated to the v0.2 track the same way (`_is_v02` parity).
//!
//! Families (each returns a list of neutral violation statements; empty ==
//! pass):
//!
//! 1. [`verify_retrieval_dedupe`] — 每个 `call_id` 的 `dedupe_key` 唯一性
//!    （同一事实只入账一次；progress 与 segment 两个到达面合并记账）。
//! 2. [`verify_result_delivered_accounting`] — `result_delivered` 的
//!    `dedupe_key` 与真实投递一一对应：一个去重键至多一条实际投递
//!    （`suppressed=false`），后续同键行必须如实标 `suppressed=true` +
//!    `duplicate`（不许"投了但被抑制"混记）。
//! 3. [`verify_retrieval_family_probe`] — `retrieval_family` 探针（F-007
//!    口径裁决 (a) 宽松口径，2026-09-13）：absent **不构成违规**，法官只在
//!    探针存在时校验其内容：不许多于一次、须在首个 `model_request` 之前、
//!    三类读数齐全。
//! 4. [`verify_failure_cause_shape`] — 失败两面不得互相矛盾：`cause` 非空、
//!    非壳码、只许出现在失败形状（非零 `exit_code` / `status=error`）；
//!    `failure_target` 同样只许出现在失败形状。两类混合形状（cause 无
//!    target / target 无 cause）经 S2 合法 fixture 证明合法，故都不判。
//! 5. [`verify_first_result_deadline`] — 首个结果 `wall_ms ≤ deadline_ms`
//!    判据族 + 「无到点前零事件路径」回归钉子（索引 0ac 判据 ①/④）。
//!
//! 语料对照测试 [`tests::s2_contract_fixtures_are_judged_as_the_contract_says`]
//! 把五族钉到 S2 机器合约上：`.valid` fixture 零违规、`.constraint.invalid`
//! 对应族必须判违规、既有 17 份 journal 全语料零违规（禁止回溯误判）。
//!
//! Registry note (S3-b, 2026-09-13, 用户裁决 F-007=(a) 宽松口径): the five
//! families are registered in [`super::families::ALL_FAMILIES`] and mirrored
//! 1:1 by the Python judge (`assurance/run_event_journal_validation.py`,
//! `_verify_v02_*` twins) so the Rust↔Python parity crosscheck
//! (`families::tests::s2b_family_verdicts_match_python`) covers the full
//! roster. The S3-a strict switch `REQUIRE_RETRIEVAL_FAMILY_PROBE` was
//! removed by that ruling — no mode fails an absent probe.

use serde_json::Value;

use super::families::{is_v02, py_int, str_of};

// F-007 口径裁决 (a)（2026-09-13）: probe absence is NOT a violation; the
// judge only checks the content of a probe that exists. The S3-a strict
// switch (`REQUIRE_RETRIEVAL_FAMILY_PROBE`) and its enforcing branch were
// removed by this ruling — there is no mode that fails an absent probe.

/// Shell-code set the S2 contract rejects (`not.enum` in
/// `runtime/tool-completed-event-payload-v0.2.schema.json`): a `cause` that
/// only repeats one of these carries no real category.
pub const CAUSE_SHELL_CODES: &[&str] = &[
    "browser_launch_failed",
    "tool_failed",
    "failed",
    "error",
    "unknown_error",
];

/// The five stable codes of the retrieval contract (`retrieval_progress.stable_code`).
pub const RETRIEVAL_STABLE_CODES: &[&str] = &[
    "capability_unreachable",
    "network_no_response",
    "network_error",
    "empty_result",
    "no_progress",
];

fn payload(event: &Value) -> Option<&Value> {
    event.get("payload")
}

fn event_type(event: &Value) -> Option<&str> {
    event.get("event_type").and_then(Value::as_str)
}

fn payload_str<'a>(event: &'a Value, key: &str) -> Option<&'a str> {
    payload(event).and_then(|p| str_of(p.get(key)))
}

/// Every v0.2 event of one of the given types, in journal order, with the
/// payload attached and the event_type available.
fn rows<'a>(events: &'a [Value], types: &[&str]) -> Vec<(&'a str, &'a Value)> {
    events
        .iter()
        .filter(|e| is_v02(e))
        .filter_map(|e| {
            let t = event_type(e)?;
            if types.contains(&t) {
                payload(e).map(|p| (t, p))
            } else {
                None
            }
        })
        .collect()
}

/// Family 1: `dedupe_key` uniqueness per `call_id` across the two retrieval
/// arrival faces (`retrieval_progress` / `retrieval_result_segment`).
pub fn verify_retrieval_dedupe(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen: Vec<(String, String)> = Vec::new();
    for (event_type, p) in rows(events, &["retrieval_progress", "retrieval_result_segment"]) {
        let call_id = p.get("call_id").and_then(Value::as_str).unwrap_or("");
        let dedupe_key = p.get("dedupe_key").and_then(Value::as_str).unwrap_or("");
        if call_id.is_empty() || dedupe_key.is_empty() {
            // Presence is the schema's job; a structurally valid journal never
            // reaches here. Keep the family silent instead of double-reporting.
            continue;
        }
        let key = (call_id.to_string(), dedupe_key.to_string());
        if seen.contains(&key) {
            errors.push(format!(
                "duplicate dedupe_key {dedupe_key:?} for call_id {call_id:?} ({event_type})"
            ));
        } else {
            seen.push(key);
        }
    }
    errors
}

/// Family 2: `result_delivered` accounting — one real delivery per
/// `dedupe_key`; anything after it must be journalled as suppressed.
pub fn verify_result_delivered_accounting(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut delivered: Vec<(String, String)> = Vec::new(); // (dedupe_key, source_id)
    for (_, p) in rows(events, &["result_delivered"]) {
        let dedupe_key = p.get("dedupe_key").and_then(Value::as_str).unwrap_or("");
        let source_id = p.get("source_id").and_then(Value::as_str).unwrap_or("");
        let suppressed = p.get("suppressed").and_then(Value::as_bool);
        if dedupe_key.is_empty() {
            continue;
        }
        match suppressed {
            Some(false) => {
                if let Some((_, first_source)) = delivered.iter().find(|(key, _)| key == dedupe_key)
                {
                    errors.push(format!(
                        "dedupe_key {dedupe_key:?} delivered twice ({first_source:?} and {source_id:?})"
                    ));
                } else {
                    delivered.push((dedupe_key.to_string(), source_id.to_string()));
                }
            }
            Some(true) => {
                if str_of(p.get("suppressed_reason")).is_none() {
                    errors.push(format!(
                        "suppressed result_delivered {dedupe_key:?} carries no suppressed_reason"
                    ));
                }
            }
            None => {}
        }
    }
    errors
}

/// Family 3: when a run carries the `retrieval_family` probe, it runs exactly
/// once and before the first model request, with the three readings present.
///
/// Absence is never a violation — F-007 口径裁决 (a) 宽松口径 (2026-09-13):
/// the judge only checks the content of a probe that exists.
pub fn verify_retrieval_family_probe(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut probe_indices: Vec<usize> = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) || event_type(event) != Some("tool_availability_check") {
            continue;
        }
        if payload_str(event, "probe_scope") == Some("retrieval_family") {
            probe_indices.push(index);
        }
    }
    match probe_indices.len() {
        // F-007 裁决 (a) 宽松口径: absent ⇒ 不判（探针缺失不构成违规）。
        0 => {}
        1 => {
            let index = probe_indices[0];
            let family = payload(&events[index]).and_then(|p| p.get("retrieval_family"));
            let complete = family
                .map(|f| {
                    ["browser", "search_engine", "web_channel"]
                        .iter()
                        .all(|key| f.get(*key).is_some())
                })
                .unwrap_or(false);
            if !complete {
                errors.push(
                    "retrieval_family probe reading is incomplete (browser/search_engine/web_channel)"
                        .to_string(),
                );
            }
            let first_model_request = events
                .iter()
                .position(|e| is_v02(e) && event_type(e) == Some("model_request"));
            if let Some(first_request) = first_model_request
                && index > first_request
            {
                errors.push(
                    "retrieval_family probe appears after the first model_request (not run-start)"
                        .to_string(),
                );
            }
        }
        count => {
            errors.push(format!(
                "retrieval_family probe emitted {count} times (run must carry exactly one)"
            ));
        }
    }
    errors
}

/// Family 4: the failure faces of `tool_completed` must not contradict each
/// other or the failure shape.
///
/// * `cause`, when present, must be a real category — non-empty and outside
///   the shell-code set the S2 schema rejects by `not.enum`
///   (`tool-completed.cause-shellcode.constraint.invalid`) — and may not ride
///   a success-shaped completion.
/// * `failure_target`, when present, marks the completion as a failure; the
///   same shape rule applies to that identity face.
///
/// Deliberately **not** judged here (evidence-driven, 2026-09-13): the two
/// mixed shapes the S2 contract already accepts as valid — `cause` without
/// `failure_target` (`payloads/tool-completed.cause.valid.json`) and
/// `failure_target` without `cause`
/// (`journals/local-browser-capability.jsonl`) — so neither direction is a
/// violation. The "every attributed failure carries a real cause" nail belongs
/// to the producer side (TODO 0ac ③ / design §10.3 item 1) and its switch —
/// a judge enforcing it today would retro-fail journals written before that
/// writer existed.
pub fn verify_failure_cause_shape(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (_, p) in rows(events, &["tool_completed"]) {
        let exit_code = py_int(p.get("exit_code"));
        let is_failure_shape =
            exit_code.is_some_and(|code| code != 0) || str_of(p.get("status")) == Some("error");
        // Python `.get()` view: explicit null reads as absent.
        let cause = p
            .get("cause")
            .filter(|value| !value.is_null())
            .and_then(Value::as_str);
        if let Some(cause) = cause {
            if cause.trim().is_empty() {
                errors.push("tool_completed cause is empty".to_string());
            } else if CAUSE_SHELL_CODES.contains(&cause) {
                errors.push(format!(
                    "tool_completed cause {cause:?} is a shell code (no real category)"
                ));
            }
            if !is_failure_shape {
                errors.push(format!(
                    "tool_completed cause {cause:?} on a non-failure shape"
                ));
            }
        }
        if p.get("failure_target")
            .filter(|value| !value.is_null())
            .is_some()
            && !is_failure_shape
        {
            errors.push("tool_completed failure_target on a non-failure shape".to_string());
        }
    }
    errors
}

/// Family 5: first-result deadline family + the "no zero-event dispatch"
/// regression nail.
pub fn verify_first_result_deadline(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut deadlines: Vec<(String, i64)> = Vec::new();
    let mut first_segments: Vec<(String, i64)> = Vec::new();
    let mut progress_seen: Vec<(String, String)> = Vec::new(); // (call_id, stage)
    let mut no_response_calls: Vec<String> = Vec::new();

    for (event_type, p) in rows(events, &["retrieval_progress", "retrieval_result_segment"]) {
        let call_id = p.get("call_id").and_then(Value::as_str).unwrap_or("");
        if call_id.is_empty() {
            continue;
        }
        if event_type == "retrieval_progress" {
            let stage = str_of(p.get("stage")).unwrap_or("");
            progress_seen.push((call_id.to_string(), stage.to_string()));
            if str_of(p.get("stable_code")) == Some("network_no_response") {
                no_response_calls.push(call_id.to_string());
            }
            if let Some(deadline) = payload_int_from(p, "deadline_ms")
                && !deadlines.iter().any(|(id, _)| id == call_id)
            {
                deadlines.push((call_id.to_string(), deadline));
            }
            if stage == "channel_alive" {
                if let Some(deadline) = payload_int_from(p, "deadline_ms") {
                    let waited = payload_int_from(p, "waited_ms");
                    if waited.is_some_and(|w| w > deadline) {
                        errors.push(format!(
                            "call_id {call_id:?} channel_alive at {}ms beyond deadline_ms={deadline}",
                            waited.unwrap_or_default()
                        ));
                    }
                }
            }
        } else {
            let index = payload_int_from(p, "segment_index").unwrap_or(0);
            let waited = payload_int_from(p, "waited_ms").unwrap_or(0);
            match first_segments.iter_mut().find(|(id, _)| id == call_id) {
                Some((_, best_waited)) => {
                    if index == 0 {
                        *best_waited = waited;
                    }
                }
                None => {
                    if index == 0 {
                        first_segments.push((call_id.to_string(), waited));
                    }
                }
            }
        }
    }

    for (call_id, waited) in &first_segments {
        let Some((_, deadline)) = deadlines.iter().find(|(id, _)| id == call_id) else {
            continue;
        };
        if waited > deadline && !no_response_calls.iter().any(|id| id == call_id) {
            errors.push(format!(
                "call_id {call_id:?} first result waited_ms={waited} > deadline_ms={deadline} \
                 without a network_no_response fact"
            ));
        }
    }

    // Regression nail (index 0ac judgment ④): a dispatched call must produce
    // at least one observable follow-up fact on either arrival face — no
    // "zero events until the deadline" waiting path.
    for (call_id, _) in progress_seen
        .iter()
        .filter(|(_, stage)| stage == "dispatched")
    {
        let observed = progress_seen
            .iter()
            .any(|(id, stage)| id == call_id && stage != "dispatched")
            || first_segments.iter().any(|(id, _)| id == call_id);
        if !observed {
            errors.push(format!(
                "call_id {call_id:?} dispatched with no follow-up event (zero-event waiting path)"
            ));
        }
    }

    errors
}

/// `payload_int` for an already-extracted payload object.
fn payload_int_from(p: &Value, key: &str) -> Option<i64> {
    py_int(p.get(key))
}

/// Family registry for the immediate-feedback families (name, verifier).
pub const IMMEDIATE_FEEDBACK_FAMILIES: &[(&str, fn(&[Value]) -> Vec<String>)] = &[
    ("retrieval_dedupe", verify_retrieval_dedupe),
    (
        "result_delivered_accounting",
        verify_result_delivered_accounting,
    ),
    ("retrieval_family_probe", verify_retrieval_family_probe),
    ("failure_cause_shape", verify_failure_cause_shape),
    ("first_result_deadline", verify_first_result_deadline),
];

/// Run one immediate-feedback family by name (`unknown` family → one
/// statement, mirroring the s2c dispatcher's fail-closed shape).
pub fn verify_immediate_feedback_family(family: &str, events: &[Value]) -> Vec<String> {
    match IMMEDIATE_FEEDBACK_FAMILIES
        .iter()
        .find(|(name, _)| *name == family)
    {
        Some((_, verifier)) => verifier(events),
        None => vec![format!("unknown immediate-feedback family {family:?}")],
    }
}

/// Run all five families, prefixed with the family name (empty == pass).
pub fn verify_all_immediate_feedback(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, verifier) in IMMEDIATE_FEEDBACK_FAMILIES {
        for violation in verifier(events) {
            errors.push(format!("{name}: {violation}"));
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn event(event_type: &str, payload: Value) -> Value {
        json!({
            "event_type": event_type,
            "payload_schema": "run-event-v0.2.schema.json",
            "payload": payload,
        })
    }

    fn progress(call_id: &str, stage: &str, dedupe_key: &str) -> Value {
        event(
            "retrieval_progress",
            json!({
                "tool": "web_search",
                "call_id": call_id,
                "retrieval_path": "local_segmented",
                "stage": stage,
                "waited_ms": 120,
                "deadline_ms": 10000,
                "dedupe_key": dedupe_key,
            }),
        )
    }

    fn segment(call_id: &str, segment_index: u64, waited_ms: u64, dedupe_key: &str) -> Value {
        event(
            "retrieval_result_segment",
            json!({
                "tool": "web_search",
                "call_id": call_id,
                "retrieval_path": "local_segmented",
                "segment_index": segment_index,
                "is_partial": false,
                "waited_ms": waited_ms,
                "dedupe_key": dedupe_key,
                "result_summary": { "visibility": "metadata_only", "source_url": "https://example.com/a" },
            }),
        )
    }

    fn family_probe(timestamp: &str) -> Value {
        event(
            "tool_availability_check",
            json!({
                "probe_scope": "retrieval_family",
                "probe_timestamp": timestamp,
                "complete": [],
                "incomplete": [],
                "gate_decision": "pass",
                "retrieval_family": {
                    "browser": { "present": false, "reason": "no browser binary configured" },
                    "search_engine": { "present": true, "detail": "cn.bing.com" },
                    "web_channel": { "present": true, "detail": "http" },
                },
            }),
        )
    }

    #[test]
    fn dedupe_key_must_be_unique_per_call() {
        let events = vec![
            progress("CALL-1", "dispatched", "ret:CALL-1:dispatch"),
            segment("CALL-1", 0, 800, "ret:CALL-1:seg0"),
        ];
        assert!(verify_retrieval_dedupe(&events).is_empty());

        let duplicated = vec![
            progress("CALL-1", "dispatched", "ret:CALL-1:dispatch"),
            progress("CALL-1", "channel_alive", "ret:CALL-1:dispatch"),
        ];
        let errors = verify_retrieval_dedupe(&duplicated);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("duplicate dedupe_key"));
    }

    #[test]
    fn delivery_accounting_flags_second_real_delivery_and_silent_suppression() {
        let delivered = |suppressed: bool, reason: Option<&str>| {
            let mut p = json!({
                "result_source": "retrieval_segment",
                "source_id": "CALL-1",
                "boundary": "B1_tool_result",
                "delivery_mode": "direct",
                "suppressed": suppressed,
                "dedupe_key": "ret:CALL-1:seg0",
                "delivered_at": "2026-09-13T00:00:00Z",
            });
            if let Some(reason) = reason {
                p["suppressed_reason"] = json!(reason);
            }
            event("result_delivered", p)
        };

        assert!(verify_result_delivered_accounting(&[delivered(false, None)]).is_empty());

        let twice =
            verify_result_delivered_accounting(&[delivered(false, None), delivered(false, None)]);
        assert_eq!(twice.len(), 1);
        assert!(twice[0].contains("delivered twice"));

        let duplicate_ok = verify_result_delivered_accounting(&[
            delivered(false, None),
            delivered(true, Some("duplicate")),
        ]);
        assert!(duplicate_ok.is_empty());

        let silent = verify_result_delivered_accounting(&[delivered(true, None)]);
        assert_eq!(silent.len(), 1);
        assert!(silent[0].contains("no suppressed_reason"));
    }

    #[test]
    fn retrieval_family_probe_is_exactly_once_at_run_start() {
        let request = event("model_request", json!({ "model": "m" }));

        // F-007 口径裁决 (a) 宽松口径（2026-09-13）：absent ⇒ 不判违规
        // （法官只在探针存在时校验内容）。
        assert!(verify_retrieval_family_probe(&[request.clone()]).is_empty());

        let ok = vec![family_probe("2026-09-13T00:00:00Z"), request.clone()];
        assert!(verify_retrieval_family_probe(&ok).is_empty());

        let late = vec![request.clone(), family_probe("2026-09-13T00:00:01Z")];
        let errors = verify_retrieval_family_probe(&late);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("after the first model_request"));

        let twice = vec![
            family_probe("2026-09-13T00:00:00Z"),
            family_probe("2026-09-13T00:00:01Z"),
            request,
        ];
        assert_eq!(verify_retrieval_family_probe(&twice).len(), 1);
    }

    #[test]
    fn failure_cause_shape_rejects_shell_codes_and_orphan_targets() {
        let with_cause = event(
            "tool_completed",
            json!({
                "tool": "web_search",
                "call_id": "CALL-1",
                "exit_code": 1,
                "cause": "channel_deadline_exceeded",
                "failure_target": { "kind": "url", "id": "https://example.com" },
            }),
        );
        assert!(verify_failure_cause_shape(&[with_cause]).is_empty());

        let shell = event(
            "tool_completed",
            json!({
                "tool": "browser_read",
                "call_id": "CALL-2",
                "exit_code": 1,
                "cause": "browser_launch_failed",
                "failure_target": { "kind": "url", "id": "https://example.com" },
            }),
        );
        let shell_errors = verify_failure_cause_shape(&[shell]);
        assert_eq!(shell_errors.len(), 1);
        assert!(shell_errors[0].contains("shell code"));

        // Contract-valid shape (`payloads/tool-completed.cause.valid.json`):
        // a real cause with no identity face must NOT be flagged.
        let cause_without_target = event(
            "tool_completed",
            json!({
                "tool": "web_search",
                "call_id": "CALL-RET-0009",
                "exit_code": 1,
                "cause": "channel_deadline_exceeded",
            }),
        );
        assert!(verify_failure_cause_shape(&[cause_without_target]).is_empty());

        // Contract-valid shape (`journals/local-browser-capability.jsonl`):
        // an identity face with no cause must NOT be flagged either.
        let target_without_cause = event(
            "tool_completed",
            json!({
                "tool": "browser_read",
                "call_id": "call-b1",
                "status": "error",
                "error": "browser_launch_failed",
                "wall_ms": 3,
                "failure_target": {
                    "kind": "url_target",
                    "id": "0f115db062b7c0dd030b16878c99dea5c354b49dc37b38eb8846179c7783e9d7",
                },
            }),
        );
        assert!(verify_failure_cause_shape(&[target_without_cause]).is_empty());

        // The shape rule still bites: no failure face may ride a success.
        let non_failure = event(
            "tool_completed",
            json!({
                "tool": "web_search",
                "call_id": "CALL-4",
                "exit_code": 0,
                "cause": "channel_deadline_exceeded",
            }),
        );
        let non_failure_errors = verify_failure_cause_shape(&[non_failure]);
        assert_eq!(non_failure_errors.len(), 1);
        assert!(non_failure_errors[0].contains("non-failure shape"));

        let target_on_success = event(
            "tool_completed",
            json!({
                "tool": "web_search",
                "call_id": "CALL-5",
                "exit_code": 0,
                "failure_target": { "kind": "url", "id": "https://example.com" },
            }),
        );
        let target_errors = verify_failure_cause_shape(&[target_on_success]);
        assert_eq!(target_errors.len(), 1);
        assert!(target_errors[0].contains("failure_target on a non-failure shape"));
    }

    #[test]
    fn first_result_deadline_family_and_zero_event_nail() {
        let healthy = vec![
            progress("CALL-1", "dispatched", "ret:CALL-1:dispatched"),
            progress("CALL-1", "channel_alive", "ret:CALL-1:alive"),
            segment("CALL-1", 0, 800, "ret:CALL-1:seg0"),
        ];
        assert!(verify_first_result_deadline(&healthy).is_empty());

        let late_first_result = vec![
            progress("CALL-2", "dispatched", "ret:CALL-2:dispatched"),
            segment("CALL-2", 0, 12000, "ret:CALL-2:seg0"),
        ];
        let late_errors = verify_first_result_deadline(&late_first_result);
        assert_eq!(late_errors.len(), 1);
        assert!(late_errors[0].contains("without a network_no_response fact"));

        let mut declared = progress("CALL-3", "failed", "ret:CALL-3:failed");
        declared["payload"]["stable_code"] = json!("network_no_response");
        let late_but_declared = vec![declared, segment("CALL-3", 0, 12000, "ret:CALL-3:seg0")];
        assert!(verify_first_result_deadline(&late_but_declared).is_empty());

        let zero_event = vec![progress("CALL-4", "dispatched", "ret:CALL-4:dispatched")];
        let zero_errors = verify_first_result_deadline(&zero_event);
        assert_eq!(zero_errors.len(), 1);
        assert!(zero_errors[0].contains("zero-event waiting path"));
    }

    #[test]
    fn family_dispatcher_is_fail_closed_on_unknown_names() {
        let errors = verify_immediate_feedback_family("no_such_family", &[]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("unknown immediate-feedback family"));
        assert_eq!(IMMEDIATE_FEEDBACK_FAMILIES.len(), 5);
    }

    /// Mechanical tie to the S2 machine contract (design §10.1/§10.3 item 2):
    /// the fixture shapes the S2 gate accepted must not be flagged by the S3
    /// judge rules, and the shapes it rejects must be.
    #[test]
    fn s2_contract_fixtures_are_judged_as_the_contract_says() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let fixtures = repo_root.join("runtime/fixtures/run-event-v0.2");
        assert!(
            fixtures.is_dir(),
            "the S3 fixture check must run inside the parent repository \
             (runtime/fixtures missing at {})",
            fixtures.display()
        );

        let payload = |name: &str| -> Value {
            let path = fixtures.join("payloads").join(name);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("fixture {} unreadable: {e}", path.display()));
            serde_json::from_str(&text).expect("fixture JSON")
        };

        // S2 `.valid` shapes: zero violations from every family.
        let valid: &[(&str, &str)] = &[
            (
                "retrieval_progress",
                "retrieval-progress.no-progress.valid.json",
            ),
            (
                "retrieval_result_segment",
                "retrieval-result-segment.partial.valid.json",
            ),
            ("result_delivered", "result-delivered.suppressed.valid.json"),
            (
                "tool_availability_check",
                "tool-availability-check.retrieval-family.valid.json",
            ),
            ("tool_completed", "tool-completed.cause.valid.json"),
        ];
        for (event_type, name) in valid {
            let events = vec![event(event_type, payload(name))];
            let violations = verify_all_immediate_feedback(&events);
            assert!(
                violations.is_empty(),
                "S2-valid fixture {name} flagged by S3 judge rules: {violations:?}"
            );
        }

        // S2 `.constraint.invalid` shapes: the family that owns the contract
        // must reject them.
        let shell_code = vec![event(
            "tool_completed",
            payload("tool-completed.cause-shellcode.constraint.invalid.json"),
        )];
        let shell_errors = verify_failure_cause_shape(&shell_code);
        assert!(
            shell_errors.iter().any(|e| e.contains("shell code")),
            "shell-code cause must be rejected: {shell_errors:?}"
        );

        let probe_missing = vec![event(
            "tool_availability_check",
            payload("tool-availability-check.retrieval-family-missing.constraint.invalid.json"),
        )];
        let probe_errors = verify_retrieval_family_probe(&probe_missing);
        assert!(
            probe_errors.iter().any(|e| e.contains("incomplete")),
            "an incomplete probe reading must be rejected: {probe_errors:?}"
        );

        // Corpus sweep: no existing schema-valid journal may be retro-flagged
        // (the producers of the three new events are not landed yet —
        // design §10.3 item 1 — so the rules must stay no-ops on real runs).
        for track in ["run-event-v0.2", "run-event-v0.1"] {
            let dir = repo_root
                .join("runtime/fixtures")
                .join(track)
                .join("journals");
            let mut names: Vec<String> = std::fs::read_dir(&dir)
                .unwrap_or_else(|e| panic!("{} unreadable: {e}", dir.display()))
                .map(|entry| {
                    entry
                        .expect("fixture entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .filter(|name| name.ends_with(".jsonl"))
                .collect();
            names.sort();
            assert!(!names.is_empty(), "{} is empty", dir.display());
            for name in names {
                let text = std::fs::read_to_string(dir.join(&name)).expect("fixture journal");
                let events: Vec<Value> = text
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| serde_json::from_str(line).expect("fixture line"))
                    .collect();
                let violations = verify_all_immediate_feedback(&events);
                assert!(
                    violations.is_empty(),
                    "{track}/{name} retro-flagged by the S3 judge rules: {violations:?}"
                );
            }
        }
    }
}
