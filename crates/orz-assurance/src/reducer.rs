//! Minimal typed-term reducer — the compromise tier of the mechanical
//! layer's composition surface (P2-10 F1 §2.3/§2.4, ADR-0010 §14.47).
//!
//! Contract (D1 user ruling, 2026-08-30):
//! - the top level keeps the `tool_calls` shape; tools are contract
//!   functions, and a SINGLE-LAYER `pipe(tool1, tool2)` may reduce in the
//!   same round;
//! - the minimal term set = apply + one-layer pipe — NO let / condition /
//!   nesting;
//! - the reduction bound is ≤ 4 steps per round;
//! - effectful tools reuse the candidate-count/budget hard gates (the
//!   reducer reports the effect count; it never injects partial results).
//!
//! The second tool's arguments are spliced from the first tool's typed
//! result (type-directed: `path`/`file_path` from the View pointer/path,
//! `expected_anchor` from a returned `new_anchor`) — the read → (anchor) →
//! search_replace lens and the grep → read filter→window pipeline.
//!
//! Pipe compatibility is a closed matrix (审查处理 R5 / F6, 2026-08-31):
//! only read → grep, read → search_replace and grep → read are typed
//! lenses; any other pair is rejected with a typed `arg_validation` Fail
//! (`pipe_incompatible`) BEFORE either tool runs — a pipe must never
//! silently degrade to an un-composed call. The grep → read lens selects a
//! match deterministically (default `match_index` = 0, or an explicit
//! non-negative `match_index` consumed from the read arguments) and maps
//! `Match.span {start, end}` to `read`'s `offset`/`length`; an empty match
//! list, out-of-range index or malformed span yields a typed Fail
//! (`no_match` / `match_index_out_of_range` / `invalid_match_index` /
//! `invalid_match_span`).

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::tool_envelope::FailEnvelope;

pub const MAX_STEPS_PER_ROUND: u32 = 4;

/// One tool application inside a term.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ApplySpec {
    pub tool: String,
    pub arguments: Value,
}

/// The minimal term grammar: apply or one-layer pipe (flat applications only).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "term", rename_all = "snake_case")]
pub enum Term {
    Apply(ApplySpec),
    Pipe { first: ApplySpec, second: ApplySpec },
}

impl Term {
    pub fn apply(tool: impl Into<String>, arguments: Value) -> Self {
        Term::Apply(ApplySpec {
            tool: tool.into(),
            arguments,
        })
    }

    pub fn pipe(
        first_tool: impl Into<String>,
        first_args: Value,
        second_tool: impl Into<String>,
        second_args: Value,
    ) -> Self {
        Term::Pipe {
            first: ApplySpec {
                tool: first_tool.into(),
                arguments: first_args,
            },
            second: ApplySpec {
                tool: second_tool.into(),
                arguments: second_args,
            },
        }
    }

    /// Step cost (§2.4): an apply is 1 step; a one-layer pipe is 2.
    pub fn step_cost(&self) -> u32 {
        match self {
            Term::Apply(_) => 1,
            Term::Pipe { .. } => 2,
        }
    }
}

/// The reduction result: the composed typed value + mechanical bookkeeping.
#[derive(Debug, Clone, Serialize)]
pub struct Reduction {
    pub value: Value,
    pub steps: u32,
    /// How many effectful tools ran (the caller feeds this into the
    /// candidate-count / budget hard gates — never bypassed).
    pub effect_count: u32,
}

/// Canonical one-layer pipe matrix (§2.3, 审查处理 R5 / F6): the only typed
/// lenses are read → grep (View path), read → search_replace (View anchor)
/// and grep → read (match selection + span → offset/length).
pub fn pipe_compatible(first_tool: &str, second_tool: &str) -> bool {
    matches!(
        (first_tool, second_tool),
        ("file.read", "file.grep")
            | ("file.read", "file.search_replace")
            | ("file.grep", "file.read")
    )
}

/// Type-directed splice: fill the second tool's missing arguments from the
/// first tool's typed result (§2.3 lens pipelines). Returns a typed
/// `arg_validation` Fail when the composition cannot be materialized
/// (no match / out-of-range index / malformed span / non-object arguments) —
/// fail-closed, never a silent pass-through or a panic.
pub fn splice_result(
    first_tool: &str,
    result: &Value,
    second_tool: &str,
    mut arguments: Value,
    trace_id: &str,
) -> Result<Value, Value> {
    let args = arguments.as_object_mut().ok_or_else(|| {
        fail_value(
            trace_id,
            "invalid_pipe_arguments",
            "pipe second arguments must be an object",
        )
    })?;
    let payload = result.get("payload").unwrap_or(result);
    // grep → read: deterministic match selection + span → offset/length.
    if first_tool == "file.grep" && second_tool == "file.read" {
        splice_grep_read_window(payload, args, trace_id)?;
    }
    // View → path (file.read → file.grep / file.search_replace; the grep
    // splice above already inserted file_path when applicable).
    if !args.contains_key("file_path") && !args.contains_key("path") {
        if let Some(path) = payload.get("path").and_then(Value::as_str) {
            args.insert("file_path".to_string(), json!(path));
        } else if let Some(path) = result
            .get("pointer")
            .and_then(|p| p.get("path"))
            .and_then(Value::as_str)
        {
            args.insert("file_path".to_string(), json!(path));
        }
    }
    // search_replace GetPut: carry the fresh anchor from the read View.
    if !args.contains_key("expected_anchor")
        && let Some(hash) = payload.get("hash").and_then(Value::as_str)
        && let Some(size) = payload.get("size").and_then(Value::as_u64)
    {
        args.insert(
            "expected_anchor".to_string(),
            json!({ "sha256": hash, "size": size }),
        );
    }
    Ok(arguments)
}

/// grep → read filter→window lens: pick the match deterministically and map
/// `Match.span {start, end}` to `read`'s `offset`/`length`. Explicit
/// `file_path`/`offset`/`length` arguments always win; the `match_index`
/// selector is consumed here and never leaks into the read call.
fn splice_grep_read_window(
    payload: &Value,
    args: &mut Map<String, Value>,
    trace_id: &str,
) -> Result<(), Value> {
    let matches = payload
        .get("matches")
        .and_then(Value::as_array)
        .ok_or_else(|| fail_value(trace_id, "no_match", "grep result carries no matches array"))?;
    if matches.is_empty() {
        return Err(fail_value(
            trace_id,
            "no_match",
            "grep returned zero matches; nothing to splice into read",
        ));
    }
    let index = match args.remove("match_index") {
        None => 0usize,
        Some(Value::Number(n)) => match n.as_u64() {
            Some(i) if (i as usize) < matches.len() => i as usize,
            Some(_) => {
                return Err(fail_value(
                    trace_id,
                    "match_index_out_of_range",
                    &format!("match_index {n} exceeds matches length {}", matches.len()),
                ));
            }
            None => {
                return Err(fail_value(
                    trace_id,
                    "invalid_match_index",
                    "match_index must be a non-negative integer",
                ));
            }
        },
        Some(_) => {
            return Err(fail_value(
                trace_id,
                "invalid_match_index",
                "match_index must be a non-negative integer",
            ));
        }
    };
    let m = &matches[index];
    let path = m
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| fail_value(trace_id, "invalid_match_span", "match carries no path"))?;
    let span = m.get("span").and_then(Value::as_object).ok_or_else(|| {
        fail_value(
            trace_id,
            "invalid_match_span",
            "match carries no span object",
        )
    })?;
    let start = span.get("start").and_then(Value::as_u64).ok_or_else(|| {
        fail_value(
            trace_id,
            "invalid_match_span",
            "span start must be a non-negative integer",
        )
    })?;
    let end = span.get("end").and_then(Value::as_u64).ok_or_else(|| {
        fail_value(
            trace_id,
            "invalid_match_span",
            "span end must be a non-negative integer",
        )
    })?;
    if end < start {
        return Err(fail_value(
            trace_id,
            "invalid_match_span",
            "span end precedes start",
        ));
    }
    if end == start {
        return Err(fail_value(
            trace_id,
            "invalid_match_span",
            "zero-length match span; read length must be ≥ 1",
        ));
    }
    if !args.contains_key("file_path") && !args.contains_key("path") {
        args.insert("file_path".to_string(), json!(path));
    }
    if !args.contains_key("offset") {
        args.insert("offset".to_string(), json!(start));
    }
    if !args.contains_key("length") {
        args.insert("length".to_string(), json!(end - start));
    }
    Ok(())
}

/// Build a typed `arg_validation` Fail envelope value (R5/F6): composition
/// failures must be typed Fail values, never a silent pass-through or a
/// panic.
fn fail_value(trace_id: &str, code: &str, message: &str) -> Value {
    FailEnvelope::arg_validation(code, message, trace_id)
        .to_value()
        .expect("FailEnvelope serialization cannot fail")
}

/// Validate the term against the minimal set (§2.4): the pipe must be flat
/// (ApplySpec cannot itself be a pipe — no nesting), both tools must be
/// known, the pipe pair must be in the typed lens matrix (R5/F6), and the
/// step cost must fit the round bound.
pub fn validate_term(
    term: &Term,
    known_tools: &dyn Fn(&str) -> bool,
    _effectful: &dyn Fn(&str) -> bool,
) -> Result<(), String> {
    let specs: Vec<&ApplySpec> = match term {
        Term::Apply(spec) => vec![spec],
        Term::Pipe { first, second } => {
            if !pipe_compatible(&first.tool, &second.tool) {
                return Err(format!(
                    "incompatible pipe: {} -> {} (allowed: file.read→file.grep, \
                     file.read→file.search_replace, file.grep→file.read)",
                    first.tool, second.tool
                ));
            }
            vec![first, second]
        }
    };
    if term.step_cost() > MAX_STEPS_PER_ROUND {
        return Err(format!(
            "term exceeds the round reduction bound: {} steps > {}",
            term.step_cost(),
            MAX_STEPS_PER_ROUND
        ));
    }
    for spec in specs {
        if !known_tools(&spec.tool) {
            return Err(format!("unknown tool in term: {}", spec.tool));
        }
        if !spec.arguments.is_object() {
            return Err(format!(
                "tool arguments must be an object (got {}) for {}",
                spec.tool, spec.arguments
            ));
        }
    }
    Ok(())
}

/// Reduce one term deterministically. `resolver` executes one tool
/// application and returns its typed Result envelope value (Ok envelope or
/// Fail envelope). A Fail short-circuits the pipe with the Fail value. An
/// incompatible pipe pair is rejected with a typed `arg_validation` Fail
/// BEFORE either tool runs (R5/F6).
pub fn reduce(
    term: &Term,
    resolver: &dyn Fn(&str, &Value) -> Value,
    effectful: &dyn Fn(&str) -> bool,
    trace_id: &str,
) -> Result<Reduction, Value> {
    match term {
        Term::Apply(spec) => {
            let value = resolver(&spec.tool, &spec.arguments);
            if is_fail(&value) {
                return Err(value);
            }
            Ok(Reduction {
                value,
                steps: 1,
                effect_count: u32::from(effectful(&spec.tool)),
            })
        }
        Term::Pipe { first, second } => {
            if !pipe_compatible(&first.tool, &second.tool) {
                return Err(fail_value(
                    trace_id,
                    "pipe_incompatible",
                    &format!("incompatible pipe: {} -> {}", first.tool, second.tool),
                ));
            }
            let first_value = resolver(&first.tool, &first.arguments);
            if is_fail(&first_value) {
                return Err(first_value);
            }
            let spliced = splice_result(
                &first.tool,
                &first_value,
                &second.tool,
                second.arguments.clone(),
                trace_id,
            )?;
            let second_value = resolver(&second.tool, &spliced);
            if is_fail(&second_value) {
                return Err(second_value);
            }
            Ok(Reduction {
                value: second_value,
                steps: 2,
                effect_count: u32::from(effectful(&first.tool))
                    + u32::from(effectful(&second.tool)),
            })
        }
    }
}

/// Discriminated Fail detection (审查处理 R7 / F5): a Fail envelope has
/// `step`/`code`/`message`/`trace_id` and NEVER a `summary`; a success Ok
/// envelope always carries `summary` (construction guarantees it). Checking
/// `step` alone would misclassify a success payload that happens to carry
/// `step`/`code` keys.
pub fn is_fail(value: &Value) -> bool {
    value.get("step").is_some() && value.get("summary").is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(tool: &str) -> bool {
        matches!(
            tool,
            "file.read" | "file.grep" | "file.search_replace" | "terminal.run" | "blackboard.read"
        )
    }

    fn effectful(tool: &str) -> bool {
        matches!(tool, "file.search_replace" | "terminal.run")
    }

    #[test]
    fn apply_reduces_in_one_step() {
        let term = Term::apply("file.read", json!({ "path": "a.rs" }));
        let r = reduce(
            &term,
            &|tool, args| {
                assert_eq!(tool, "file.read");
                json!({ "summary": "read ok", "payload": { "path": args["path"] } })
            },
            &effectful,
            "t-1",
        )
        .unwrap();
        assert_eq!(r.steps, 1);
        assert_eq!(r.effect_count, 0);
        assert_eq!(r.value["payload"]["path"], "a.rs");
    }

    #[test]
    fn pipe_splices_path_into_second_arguments() {
        let term = Term::pipe(
            "file.read",
            json!({ "path": "a.rs" }),
            "file.grep",
            json!({ "pattern": "fn" }),
        );
        let r = reduce(
            &term,
            &|tool, args| match tool {
                "file.read" => json!({ "summary": "read", "payload": { "path": "a.rs", "hash": "0".repeat(64), "size": 10 } }),
                "file.grep" => {
                    assert_eq!(args["file_path"], "a.rs", "path must splice into grep args");
                    json!({ "summary": "1 match", "payload": { "matches": [] } })
                }
                _ => unreachable!(),
            },
            &effectful,
            "t-1",
        )
        .unwrap();
        assert_eq!(r.steps, 2);
        assert_eq!(r.effect_count, 0);
    }

    #[test]
    fn read_search_replace_lens_carries_anchor() {
        let term = Term::pipe(
            "file.read",
            json!({ "path": "a.rs" }),
            "file.search_replace",
            json!({ "old_string": "x", "new_string": "y" }),
        );
        let r = reduce(
            &term,
            &|tool, args| match tool {
                "file.read" => json!({ "summary": "read", "payload": { "path": "a.rs", "hash": "ab".repeat(32), "size": 42 } }),
                "file.search_replace" => {
                    assert_eq!(args["file_path"], "a.rs");
                    assert_eq!(args["expected_anchor"]["sha256"], "ab".repeat(32));
                    assert_eq!(args["expected_anchor"]["size"], 42);
                    json!({ "summary": "replaced", "payload": { "ok": true } })
                }
                _ => unreachable!(),
            },
            &effectful,
            "t-1",
        )
        .unwrap();
        assert_eq!(r.effect_count, 1, "search_replace is effectful");
    }

    #[test]
    fn fail_short_circuits_the_pipe() {
        let term = Term::pipe(
            "file.read",
            json!({ "path": "a.rs" }),
            "file.grep",
            json!({}),
        );
        let err = reduce(
            &term,
            &|tool, _| {
                if tool == "file.read" {
                    json!({ "step": "arg_validation", "code": "no_match", "message": "no file", "trace_id": "t-1" })
                } else {
                    json!({})
                }
            },
            &effectful,
            "t-1",
        )
        .unwrap_err();
        assert_eq!(err["code"], "no_match");
    }

    /// 审查处理 R7 (F5): a success Ok envelope that happens to carry
    /// `step`/`code` keys must NOT be misclassified as a Fail — the
    /// discriminator is the envelope's `summary` field, not field sniffing.
    #[test]
    fn success_with_step_and_code_keys_is_not_fail() {
        let ok_payload = json!({
            "summary": "ok",
            "cap": { "bytes": 10 },
            "payload": { "step": "arg_validation", "code": "whatever" },
        });
        assert!(!is_fail(&ok_payload), "Ok envelope must not look like Fail");
        let fail = json!({
            "step": "arg_validation",
            "code": "no_match",
            "message": "no match",
            "trace_id": "t-1",
        });
        assert!(is_fail(&fail), "Fail envelope must be detected");
        // A non-envelope value with a `step` marker and no `summary` is
        // treated as Fail (fail-closed: without an Ok-envelope `summary`
        // the value cannot be a success).
        assert!(is_fail(&json!({ "step": "x", "code": "y" })));
    }

    #[test]
    fn validation_rejects_unknown_tools_and_bad_args() {
        let term = Term::apply("nope", json!({}));
        assert!(validate_term(&term, &known, &effectful).is_err());
        let bad_args = Term::apply("file.read", json!("string"));
        assert!(validate_term(&bad_args, &known, &effectful).is_err());
        let ok = Term::pipe("file.read", json!({}), "file.grep", json!({}));
        assert!(validate_term(&ok, &known, &effectful).is_ok());
    }

    #[test]
    fn step_cost_is_bounded() {
        assert_eq!(Term::apply("file.read", json!({})).step_cost(), 1);
        assert_eq!(
            Term::pipe("file.read", json!({}), "file.grep", json!({})).step_cost(),
            2
        );
        assert!(2 <= MAX_STEPS_PER_ROUND);
    }

    #[test]
    fn pipe_compatibility_matrix_is_the_documented_lenses() {
        assert!(pipe_compatible("file.read", "file.grep"));
        assert!(pipe_compatible("file.read", "file.search_replace"));
        assert!(pipe_compatible("file.grep", "file.read"));
        assert!(!pipe_compatible("file.grep", "file.search_replace"));
        assert!(!pipe_compatible("file.read", "file.read"));
        assert!(!pipe_compatible("terminal.run", "file.read"));
        assert!(!pipe_compatible("file.read", "terminal.run"));
        assert!(!pipe_compatible("blackboard.read", "file.read"));
    }

    /// R5/F6: grep → read must splice the selected match's span into read's
    /// offset/length (filter→window lens), and the `match_index` selector
    /// must be consumed, never forwarded to the read call.
    #[test]
    fn grep_read_pipe_splices_match_span_into_read_args() {
        let term = Term::pipe(
            "file.grep",
            json!({ "pattern": "fn", "path": "src" }),
            "file.read",
            json!({}),
        );
        let r = reduce(
            &term,
            &|tool, args| match tool {
                "file.grep" => json!({
                    "summary": "1 match",
                    "payload": { "matches": [
                        { "path": "a.rs", "line_no": 3, "span": { "start": 120, "end": 132 }, "text": "fn main()" }
                    ] }
                }),
                "file.read" => {
                    assert_eq!(args["file_path"], "a.rs", "match path must splice");
                    assert_eq!(args["offset"], 120, "span start → offset");
                    assert_eq!(args["length"], 12, "span end - start → length");
                    assert!(
                        args.get("match_index").is_none(),
                        "match_index must be consumed, never leaked to read"
                    );
                    json!({ "summary": "window", "payload": { "path": "a.rs", "window": { "offset": 120, "content": "fn main()", "truncated": false } } })
                }
                _ => unreachable!(),
            },
            &effectful,
            "t-1",
        )
        .unwrap();
        assert_eq!(r.steps, 2);
        assert_eq!(r.value["payload"]["window"]["offset"], 120);
    }

    /// R5/F6: explicit `match_index` selects the match; explicit read args
    /// (offset/length/file_path) always win over the spliced span.
    #[test]
    fn grep_read_pipe_selects_index_and_keeps_explicit_args() {
        let term = Term::pipe(
            "file.grep",
            json!({ "pattern": "x" }),
            "file.read",
            json!({ "match_index": 1, "length": 5 }),
        );
        let r = reduce(
            &term,
            &|tool, args| match tool {
                "file.grep" => json!({
                    "summary": "2 matches",
                    "payload": { "matches": [
                        { "path": "a.rs", "line_no": 1, "span": { "start": 0, "end": 10 }, "text": "aaa" },
                        { "path": "b.rs", "line_no": 2, "span": { "start": 40, "end": 60 }, "text": "bbb" }
                    ] }
                }),
                "file.read" => {
                    assert_eq!(args["file_path"], "b.rs", "match_index 1 selects b.rs");
                    assert_eq!(args["offset"], 40, "span start of selected match");
                    assert_eq!(args["length"], 5, "explicit length wins over span width");
                    assert!(args.get("match_index").is_none());
                    json!({ "summary": "window", "payload": {} })
                }
                _ => unreachable!(),
            },
            &effectful,
            "t-1",
        )
        .unwrap();
        assert_eq!(r.effect_count, 0);
    }

    /// R5/F6: an empty match list is a typed arg_validation Fail (no_match),
    /// and the read tool must never run.
    #[test]
    fn grep_read_pipe_empty_matches_returns_typed_arg_validation() {
        let term = Term::pipe(
            "file.grep",
            json!({ "pattern": "x" }),
            "file.read",
            json!({}),
        );
        let err = reduce(
            &term,
            &|tool, _| match tool {
                "file.grep" => json!({ "summary": "no match", "payload": { "matches": [] } }),
                _ => unreachable!("read must not run when grep has no matches"),
            },
            &effectful,
            "t-1",
        )
        .unwrap_err();
        assert_eq!(err["step"], "arg_validation");
        assert_eq!(err["code"], "no_match");
        assert_eq!(err["trace_id"], "t-1");
    }

    /// R5/F6: malformed selection (out-of-range / non-numeric match_index,
    /// reversed or zero-length span) must fail closed with typed codes.
    #[test]
    fn grep_read_pipe_rejects_bad_index_and_span() {
        let one_match = |_: &str, _: &Value| {
            json!({
                "summary": "m",
                "payload": { "matches": [
                    { "path": "a.rs", "span": { "start": 0, "end": 1 }, "text": "x" }
                ] }
            })
        };
        let out_of_range = Term::pipe(
            "file.grep",
            json!({}),
            "file.read",
            json!({ "match_index": 5 }),
        );
        let err = reduce(&out_of_range, &one_match, &effectful, "t-1").unwrap_err();
        assert_eq!(err["code"], "match_index_out_of_range");

        let bad_index = Term::pipe(
            "file.grep",
            json!({}),
            "file.read",
            json!({ "match_index": "first" }),
        );
        let err = reduce(&bad_index, &one_match, &effectful, "t-1").unwrap_err();
        assert_eq!(err["code"], "invalid_match_index");

        let reversed = Term::pipe("file.grep", json!({}), "file.read", json!({}));
        let err = reduce(
            &reversed,
            &|_, _| {
                json!({ "summary": "m", "payload": { "matches": [
                { "path": "a.rs", "span": { "start": 9, "end": 3 }, "text": "x" }
            ] } })
            },
            &effectful,
            "t-1",
        )
        .unwrap_err();
        assert_eq!(err["code"], "invalid_match_span");

        let zero = Term::pipe("file.grep", json!({}), "file.read", json!({}));
        let err = reduce(
            &zero,
            &|_, _| {
                json!({ "summary": "m", "payload": { "matches": [
                { "path": "a.rs", "span": { "start": 3, "end": 3 }, "text": "" }
            ] } })
            },
            &effectful,
            "t-1",
        )
        .unwrap_err();
        assert_eq!(err["code"], "invalid_match_span");
    }

    /// R5/F6: an incompatible pipe pair (terminal.run → file.read) is
    /// rejected with a typed arg_validation Fail BEFORE either tool runs.
    #[test]
    fn incompatible_pipe_rejected_with_typed_fail_before_execution() {
        let term = Term::pipe(
            "terminal.run",
            json!({ "cmd": "ls" }),
            "file.read",
            json!({}),
        );
        let err = reduce(
            &term,
            &|_, _| panic!("incompatible pipe must be rejected before any tool runs"),
            &effectful,
            "t-1",
        )
        .unwrap_err();
        assert_eq!(err["step"], "arg_validation");
        assert_eq!(err["code"], "pipe_incompatible");
        assert_eq!(err["trace_id"], "t-1");
    }

    /// R5/F6: validation rejects incompatible pairs too (fail-closed at the
    /// term-validation stage, not only at reduction time).
    #[test]
    fn validate_term_rejects_incompatible_pipe_and_accepts_lenses() {
        let bad = Term::pipe("terminal.run", json!({}), "file.read", json!({}));
        assert!(validate_term(&bad, &known, &effectful).is_err());
        let bad_grep_read_write =
            Term::pipe("file.grep", json!({}), "file.search_replace", json!({}));
        assert!(validate_term(&bad_grep_read_write, &known, &effectful).is_err());
        let good = Term::pipe("file.grep", json!({}), "file.read", json!({}));
        assert!(validate_term(&good, &known, &effectful).is_ok());
    }
}
