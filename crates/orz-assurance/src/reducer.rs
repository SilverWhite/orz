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

use serde::Serialize;
use serde_json::{Value, json};

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
    Pipe {
        first: ApplySpec,
        second: ApplySpec,
    },
}

impl Term {
    pub fn apply(tool: impl Into<String>, arguments: Value) -> Self {
        Term::Apply(ApplySpec {
            tool: tool.into(),
            arguments,
        })
    }

    pub fn pipe(first_tool: impl Into<String>, first_args: Value, second_tool: impl Into<String>, second_args: Value) -> Self {
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

/// Type-directed splice: fill the second tool's missing arguments from the
/// first tool's typed result (§2.3 lens pipelines).
pub fn splice_result(result: &Value, mut arguments: Value) -> Value {
    let args = arguments
        .as_object_mut()
        .expect("pipe second arguments must be an object");
    let payload = result.get("payload").unwrap_or(result);
    // View → path (file.read → file.grep / file.search_replace).
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
    arguments
}

/// Validate the term against the minimal set (§2.4): the pipe must be flat
/// (ApplySpec cannot itself be a pipe — no nesting), both tools must be
/// known, and the step cost must fit the round bound.
pub fn validate_term(
    term: &Term,
    known_tools: &dyn Fn(&str) -> bool,
    _effectful: &dyn Fn(&str) -> bool,
) -> Result<(), String> {
    let specs: Vec<&ApplySpec> = match term {
        Term::Apply(spec) => vec![spec],
        Term::Pipe { first, second } => vec![first, second],
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
                spec.tool,
                spec.arguments
            ));
        }
    }
    Ok(())
}

/// Reduce one term deterministically. `resolver` executes one tool
/// application and returns its typed Result envelope value (Ok envelope or
/// Fail envelope). A Fail short-circuits the pipe with the Fail value.
pub fn reduce(
    term: &Term,
    resolver: &dyn Fn(&str, &Value) -> Value,
    effectful: &dyn Fn(&str) -> bool,
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
            let first_value = resolver(&first.tool, &first.arguments);
            if is_fail(&first_value) {
                return Err(first_value);
            }
            let spliced = splice_result(&first_value, second.arguments.clone());
            let second_value = resolver(&second.tool, &spliced);
            if is_fail(&second_value) {
                return Err(second_value);
            }
            Ok(Reduction {
                value: second_value,
                steps: 2,
                effect_count: u32::from(effectful(&first.tool)) + u32::from(effectful(&second.tool)),
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
        )
        .unwrap();
        assert_eq!(r.steps, 1);
        assert_eq!(r.effect_count, 0);
        assert_eq!(r.value["payload"]["path"], "a.rs");
    }

    #[test]
    fn pipe_splices_path_into_second_arguments() {
        let term = Term::pipe("file.read", json!({ "path": "a.rs" }), "file.grep", json!({ "pattern": "fn" }));
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
        )
        .unwrap();
        assert_eq!(r.effect_count, 1, "search_replace is effectful");
    }

    #[test]
    fn fail_short_circuits_the_pipe() {
        let term = Term::pipe("file.read", json!({ "path": "a.rs" }), "file.grep", json!({}));
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
}
