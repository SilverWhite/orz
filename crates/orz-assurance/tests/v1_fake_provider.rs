//! P2-10 阶段 3 V1 — FakeProvider 测试面验证（无需实机）。
//!
//! 机械层（I5 类型化信封 + I6 一层 pipe 归约）的求值器语义用脚本化
//! FakeProvider 工具面验证：模型面的 `tool_calls` 形状（D1 折中档）经
//! `reduce` 归约，工具契约函数化返回类型化 Result 信封（§2.1/§2.2）。
//! 覆盖：
//! - 信封三件套（summary/cap/payload/pointer）经归约原样保留；
//! - read → grep / read → search_replace（GetPut 锚点）/ grep → read
//!   （match 选择 + span→offset/length）三条类型化透镜；
//! - 不兼容 pipe 在任一工具执行前以 arg_validation Fail（pipe_incompatible）
//!   关闭（R5/F6）；
//! - Fail 短路时 trace_id 原样保留——receipt（trace）与事件链逐段同构的
//!   机械层一侧（§5.4 / F11）；
//! - 效应数复用候选计数/预算硬门（effect_count）。

use orz_assurance::reducer::{pipe_compatible, reduce, validate_term, Term};
use orz_assurance::tool_envelope::{FailEnvelope, OkEnvelope, Pointer};
use serde_json::{Value, json};

/// FakeProvider 工具面：脚本化类型化信封。对已登记工具返回确定性 Ok/Fail
/// 信封值；未登记工具 fail-loud（测试中不应出现）。
fn resolve(tool: &str, args: &Value) -> Value {
    match tool {
        "file.read" => {
            let path = args
                .get("file_path")
                .or_else(|| args.get("path"))
                .and_then(Value::as_str)
                .unwrap_or("a.rs");
            let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0);
            OkEnvelope::new(
                format!("read window at {offset}"),
                json!({ "bytes": 16384 }),
                json!({
                    "path": path,
                    "hash": "ab".repeat(32),
                    "size": 1024,
                    "window": { "offset": offset, "content": "fn main() {}", "truncated": false },
                }),
                Some(Pointer::FilePtr { path: path.into(), hash: "ab".repeat(32), offset: Some(offset) }),
            )
            .to_value()
            .expect("serialize")
        }
        "file.grep" => {
            let pattern = args.get("pattern").and_then(Value::as_str).unwrap_or("");
            if pattern.is_empty() {
                return FailEnvelope::arg_validation(
                    "invalid_pattern",
                    "pattern must be non-empty",
                    "trace-v1",
                )
                .to_value()
                .expect("serialize");
            }
            let path = args.get("file_path").and_then(Value::as_str).unwrap_or("a.rs");
            OkEnvelope::new(
                "1 match",
                json!({ "matches": 64, "bytes": 4096 }),
                json!({
                    "pattern": pattern,
                    "files_searched": 1,
                    "matches": [
                        { "path": path, "line_no": 3, "span": { "start": 120, "end": 132 }, "text": "fn main()" }
                    ],
                    "truncated": false,
                }),
                None,
            )
            .to_value()
            .expect("serialize")
        }
        "file.search_replace" => {
            let anchor = args.get("expected_anchor");
            if anchor.is_some() {
                // GetPut：携带锚点即视为写前核证通过。
                OkEnvelope::new(
                    "replaced",
                    json!({ "bytes": 1024 }),
                    json!({ "ok": true, "diff": { "minus": 1, "plus": 1 }, "new_anchor": anchor }),
                    None,
                )
                .to_value()
                .expect("serialize")
            } else {
                FailEnvelope::gate(
                    "anchor_required",
                    "search_replace needs the read anchor (GetPut)",
                    "trace-v1",
                )
                .to_value()
                .expect("serialize")
            }
        }
        "terminal.run" => OkEnvelope::new(
            "ran",
            json!({ "bytes": 8192 }),
            json!({ "exit_code": 0, "wall_ms": 120, "stdout_tail": "", "stderr_tail": "" }),
            None,
        )
        .to_value()
        .expect("serialize"),
        _ => panic!("fake surface invoked for unknown tool: {tool}"),
    }
}

fn known(tool: &str) -> bool {
    matches!(tool, "file.read" | "file.grep" | "file.search_replace" | "terminal.run")
}

fn effectful(tool: &str) -> bool {
    matches!(tool, "file.search_replace" | "terminal.run")
}

/// V1-1: Apply 归约保留信封三件套 + 指针；步骤 1、效应 0。
#[test]
fn apply_preserves_typed_envelope_shape() {
    let term = Term::apply("file.read", json!({ "path": "a.rs" }));
    assert!(validate_term(&term, &known, &effectful).is_ok());
    let r = reduce(&term, &resolve, &effectful, "trace-v1").unwrap();
    assert_eq!(r.steps, 1);
    assert_eq!(r.effect_count, 0);
    assert_eq!(r.value["summary"], "read window at 0");
    assert!(r.value.get("cap").is_some(), "cap 必在信封上");
    assert_eq!(r.value["payload"]["path"], "a.rs");
    assert_eq!(r.value["pointer"]["kind"], "file_ptr");
}

/// V1-2: read → grep 透镜（路径 splice）。
#[test]
fn pipe_read_grep_splices_path() {
    let term = Term::pipe("file.read", json!({ "path": "a.rs" }), "file.grep", json!({ "pattern": "fn" }));
    let r = reduce(&term, &resolve, &effectful, "trace-v1").unwrap();
    assert_eq!(r.steps, 2);
    assert_eq!(r.effect_count, 0);
    assert_eq!(r.value["payload"]["matches"][0]["path"], "a.rs");
    assert_eq!(r.value["payload"]["matches"][0]["span"]["start"], 120);
}

/// V1-3: read → search_replace 透镜（GetPut 锚点携带）；效应计数 = 1。
#[test]
fn pipe_read_search_replace_carries_anchor_and_counts_effect() {
    let term = Term::pipe(
        "file.read",
        json!({ "path": "a.rs" }),
        "file.search_replace",
        json!({ "old_string": "x", "new_string": "y" }),
    );
    let r = reduce(&term, &resolve, &effectful, "trace-v1").unwrap();
    assert_eq!(r.effect_count, 1, "search_replace 是效应工具，须计入硬门");
    let anchor = &r.value["payload"]["new_anchor"];
    assert_eq!(anchor["sha256"], "ab".repeat(32));
    assert_eq!(anchor["size"], 1024);
}

/// V1-4: grep → read 过滤→取窗透镜（match 选择 + span→offset/length）。
#[test]
fn pipe_grep_read_selects_match_and_maps_span() {
    let term = Term::pipe("file.grep", json!({ "pattern": "fn" }), "file.read", json!({}));
    let r = reduce(&term, &resolve, &effectful, "trace-v1").unwrap();
    assert_eq!(r.steps, 2);
    assert_eq!(r.value["payload"]["window"]["offset"], 120, "span.start → offset");
    assert_eq!(r.value["payload"]["window"]["content"], "fn main() {}", "length = end−start 窗");
}

/// V1-5: 不兼容 pipe 在任一工具执行前类型化关闭（R5/F6）。
#[test]
fn incompatible_pipe_fails_closed_before_execution() {
    let term = Term::pipe("terminal.run", json!({ "cmd": "ls" }), "file.read", json!({}));
    assert!(validate_term(&term, &known, &effectful).is_err(), "验证层即拒绝");
    let err = reduce(&term, &|_, _| panic!("不兼容 pipe 不得执行任一工具"), &effectful, "trace-v1").unwrap_err();
    assert_eq!(err["step"], "arg_validation");
    assert_eq!(err["code"], "pipe_incompatible");
    assert_eq!(err["trace_id"], "trace-v1");
}

/// V1-6: Fail 短路——第一工具的类型化失败直接成为归约错误，trace_id 原样
/// 保留（receipt ↔ 事件链同构的机械层一侧；§5.4 / F11）。
#[test]
fn fail_short_circuit_preserves_trace_id() {
    let term = Term::pipe("file.grep", json!({ "pattern": "" }), "file.read", json!({}));
    let err = reduce(&term, &resolve, &effectful, "trace-v1").unwrap_err();
    assert_eq!(err["step"], "arg_validation");
    assert_eq!(err["code"], "invalid_pattern");
    assert_eq!(err["trace_id"], "trace-v1");
}

/// V1-7: 兼容矩阵只含三条类型化透镜（R5）。
#[test]
fn pipe_matrix_is_the_documented_lenses() {
    assert!(pipe_compatible("file.read", "file.grep"));
    assert!(pipe_compatible("file.read", "file.search_replace"));
    assert!(pipe_compatible("file.grep", "file.read"));
    assert!(!pipe_compatible("file.read", "file.read"));
    assert!(!pipe_compatible("terminal.run", "file.read"));
    assert!(!pipe_compatible("file.grep", "file.search_replace"));
}

/// V1-8: 效应工具经归约仍计入 effect_count（候选计数/预算硬门不被绕过）。
#[test]
fn effect_count_is_reported_to_the_hard_gate() {
    let read = Term::apply("file.read", json!({ "path": "a.rs" }));
    assert_eq!(reduce(&read, &resolve, &effectful, "trace-v1").unwrap().effect_count, 0);
    let write = Term::pipe(
        "file.read",
        json!({ "path": "a.rs" }),
        "file.search_replace",
        json!({ "old_string": "x", "new_string": "y" }),
    );
    assert_eq!(reduce(&write, &resolve, &effectful, "trace-v1").unwrap().effect_count, 1);
    let term = Term::pipe(
        "file.read",
        json!({ "path": "a.rs" }),
        "terminal.run",
        json!({ "cmd": "echo hi" }),
    );
    // terminal.run → … 不在透镜矩阵内：拒绝发生在效应执行之前。
    let err = reduce(&term, &|_, _| panic!("must not run"), &effectful, "trace-v1").unwrap_err();
    assert_eq!(err["code"], "pipe_incompatible");
}
