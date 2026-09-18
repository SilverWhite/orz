//! Task D S2b (2026-09-06): mechanical rule-family verifiers for the v0.2
//! track — the Rust conformance counterpart of the first six families (seven
//! function entry points) of `assurance/run_event_journal_validation.py`
//! (`_verify_v02_*`), inventoried in
//! `docs/audits/TASK_D_S2A_INVENTORY_2026-09-06.md`.
//!
//! Semantics mirror the Python judge rule for rule (single pass over journal
//! order, per-run / per-activation state machines, canonical-bytes replay
//! comparison via [`super::chain::canonical_json`]). Message TEXT follows the
//! Rust form; the S2b acceptance is verdict parity (same per-family
//! pass/fail), asserted by the crosscheck test against the Python module.
//!
//! Gating (mirrors `validate_journal_text`): families run only on
//! payload-schema-valid journals; on the v0.1 track every `_is_v02` filter
//! makes them no-ops, so the Rust judge skips the stage for V01 journals.

use serde_json::Value;

use super::chain::canonical_json;

/// Payload schema id of the v0.2 track (Python `_is_v02`).
const V02_PAYLOAD_SCHEMA: &str = "run-event-v0.2.schema.json";

pub(crate) fn is_v02(event: &Value) -> bool {
    event.get("payload_schema").and_then(Value::as_str) == Some(V02_PAYLOAD_SCHEMA)
}

/// Python `isinstance(x, int)` for JSON values: integers and booleans count
/// (bool is an int in Python), floats do not.
pub(crate) fn py_int(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::Bool(b)) => Some(*b as i64),
        _ => None,
    }
}

pub(crate) fn str_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

/// Tool-family constants mirroring the Python module tables.
pub(crate) mod toolsets {
    use crate::tool_names::{BLACKBOARD_WRITE_TOOL_NAME, CONTEXT_COMPRESS_TOOL_NAME};
    pub const WORK_TOOLS: &[&str] = &[
        "read_file",
        "list_dir",
        "grep",
        "search_tool",
        "search_replace",
        "run_tests",
        "ask_user_question",
        "blackboard_read",
        // 0aj（2026-09-16）：`blackboard_write`（0ae D0 模型写入面）入工作
        // 工具表——与 orz-loop `tool_probe::WORK_TOOLS` 及 Python
        // `_WORK_TOOLS` 三处同批（探针面与请求面脱同步的修复面）。
        // 名字经 tool_names 单源（0ao）。
        BLACKBOARD_WRITE_TOOL_NAME,
        // 0ap（2026-09-18）：`context_compress` 入工作工具表（压缩交互
        // 第九工具）——三处同批先例（探针/判官/Python 镜像），Python 侧
        // `_WORK_TOOLS` 同批 +1（表格数据同步；判别规则零改动）。
        CONTEXT_COMPRESS_TOOL_NAME,
        "todo_write",
        "update_goal",
        "enter_plan_mode",
        "exit_plan_mode",
        "compaction_whitelist_add",
        "retrieval_disposition",
        "run_terminal_cmd",
        "lsp",
        "memory_get",
        "memory_search",
        "image_gen",
        "image_edit",
        "image_to_video",
        "reference_to_video",
        "use_tool",
    ];
    // 0t S2-R P3 / P1-2b (2026-09-09): `browser_control` 同属 host 路由
    // 检索工具（relay `is_retrieval_mode_gated_host_tool`），启用门族与
    // launch 义务判定面一并纳入（P5 对拍固化）。
    pub const RETRIEVAL_MODE_GATED_TOOLS: &[&str] = &[
        "project_doc_index",
        "browser_read",
        "browser_control",
        "pdf_read",
    ];
    pub const ACAF_TICKETED_TOOLS: &[&str] = &[
        "search_replace",
        "run_tests",
        "run_terminal_cmd",
        "web_fetch",
        "browser_read",
    ];
    pub const RETRIEVAL_TARGETS: &[&str] = &["internal_retrieval", "external_retrieval"];
    pub const HOST_LANE_RETRIEVAL_TOOLS: &[&str] = &["browser_read", "browser_control"];
    pub const CMD_TARGET_TOOLS: &[&str] = &["run_terminal_cmd", "run_tests"];
    pub const ANCHOR_TARGET_TOOLS: &[&str] = &["search_replace"];
    pub const FILE_TARGET_TOOLS: &[&str] = &["search_replace", "read_file", "grep"];
    pub const URL_TARGET_TOOLS: &[&str] = &["web_fetch", "browser_read"];

    pub fn contains(set: &[&str], name: Option<&str>) -> bool {
        match name {
            Some(name) => set.contains(&name),
            None => false,
        }
    }

    /// Python `_is_retrieval_mode_gated_tool`.
    pub fn is_retrieval_mode_gated_tool(name: Option<&str>) -> bool {
        let Some(name) = name else {
            return false;
        };
        RETRIEVAL_MODE_GATED_TOOLS.contains(&name)
            || name.starts_with("retrieve_project_")
            || name == "web_search"
            || name.starts_with("web_search_")
            || name == "web_fetch"
            || name.starts_with("web_fetch_")
    }
}

const LANE_ROLES: &[&str] = &["main", "internal_retrieval", "external_retrieval"];

const SHA256_HEX: &[u8] = b"0123456789abcdef";

fn is_sha256_hex(value: Option<&str>) -> bool {
    match value {
        Some(s) => s.len() == 64 && s.bytes().all(|b| SHA256_HEX.contains(&b)),
        None => false,
    }
}

fn utf8_len(value: &str) -> usize {
    value.len() // Rust str is UTF-8; .len() is the byte length
}

fn canonical_bytes(value: &Value) -> Vec<u8> {
    canonical_json(value).unwrap_or_default()
}

/// Python truthiness for optional payload fields (None/False/0/"" are falsy).
fn py_truthy(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_none_or(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

// ── control_tickets ─────────────────────────────────────────────────────

/// Python `_verify_v02_control_tickets` (ACAF Slice 1): terminal ticket
/// events reference an earlier issued ticket of the same id, agree on
/// `ticket_kind`, and a ticket has at most one terminal event (one-shot).
pub fn verify_control_tickets(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    // ticket_id -> (issue index, kind)
    let mut issued: std::collections::HashMap<String, (usize, String)> =
        std::collections::HashMap::new();
    // ticket_id -> (terminal index, terminal event type)
    let mut terminals: std::collections::HashMap<String, (usize, String)> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event.get("event_type").and_then(Value::as_str);
        if !matches!(
            event_type,
            Some("control_ticket_issued")
                | Some("control_ticket_consumed")
                | Some("control_ticket_rejected")
        ) {
            continue;
        }
        let event_type = event_type.unwrap_or_default();
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let ticket_id = str_of(payload.get("ticket_id"));
        let kind = str_of(payload.get("ticket_kind"));
        let (Some(ticket_id), Some(kind)) = (ticket_id, kind) else {
            // envelope/payload schema violations are reported elsewhere
            continue;
        };
        if event_type == "control_ticket_issued" {
            issued.insert(ticket_id.to_string(), (index, kind.to_string()));
            continue;
        }
        let Some((issue_index, issue_kind)) = issued.get(ticket_id) else {
            errors.push(format!(
                "event {index}: {event_type} references unknown ticket \
                 {ticket_id} (not issued in this journal)"
            ));
            continue;
        };
        if index <= *issue_index {
            errors.push(format!(
                "event {index}: {event_type} for {ticket_id} precedes its \
                 issue at event {issue_index}"
            ));
        }
        if kind != issue_kind {
            errors.push(format!(
                "event {index}: {event_type} ticket_kind {kind:?} disagrees \
                 with issued {issue_kind:?} for {ticket_id}"
            ));
        }
        if let Some((first_index, first_type)) = terminals.get(ticket_id) {
            errors.push(format!(
                "event {index}: {event_type} for {ticket_id} after it was \
                 already {first_type} at event {first_index} (one-shot)"
            ));
        } else {
            terminals.insert(ticket_id.to_string(), (index, event_type.to_string()));
        }
    }
    errors
}

// ── retrieval_mode ──────────────────────────────────────────────────────

/// Python `_verify_v02_retrieval_mode` (ADR-0010 §3.7.1): session-level mode
/// chain, at most one bootstrap transition, and the post-transition
/// obligations for `off` and non-`available` `local_browser` windows.
pub fn verify_retrieval_mode(events: &[Value]) -> Vec<String> {
    use toolsets::{HOST_LANE_RETRIEVAL_TOOLS, RETRIEVAL_TARGETS, contains};

    let mut errors = Vec::new();
    let transitions: Vec<(usize, &Value)> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| is_v02(event) && event["event_type"] == "retrieval_mode_transition")
        .collect();

    let mut bootstrap_count = 0usize;
    let mut current_mode: Option<String> = None;
    for (position, &(index, event)) in transitions.iter().enumerate() {
        let payload = &event["payload"];
        let old_mode = str_of(payload.get("old_mode")).unwrap_or_default();
        let new_mode = str_of(payload.get("new_mode")).unwrap_or_default();
        if payload.get("authority").and_then(Value::as_str) == Some("session_bootstrap") {
            bootstrap_count += 1;
        }
        if let Some(current) = &current_mode
            && old_mode != current.as_str()
        {
            errors.push(format!(
                "event {index}: transition old_mode {old_mode:?} != previous \
                 transition new_mode {current:?}"
            ));
        }
        current_mode = Some(new_mode.to_string());

        let window_end = transitions
            .get(position + 1)
            .map_or(events.len(), |&(next_index, _)| next_index);
        if new_mode == "off" {
            for (offset, ev) in events[index + 1..window_end].iter().enumerate() {
                let j = index + 1 + offset;
                let payload_j = ev.get("payload").cloned().unwrap_or(Value::Null);
                let dispatch = ev.get("event_type").and_then(Value::as_str) == Some("tool_started")
                    && (contains(RETRIEVAL_TARGETS, str_of(payload_j.get("target")))
                        || contains(HOST_LANE_RETRIEVAL_TOOLS, str_of(payload_j.get("tool"))));
                if dispatch {
                    errors.push(format!(
                        "event {j}: retrieval dispatch {} after transition to \
                         off (event {index})",
                        str_of(payload_j.get("tool")).unwrap_or("<missing>")
                    ));
                } else if ev.get("event_type").and_then(Value::as_str)
                    == Some("information_sufficiency_assessment")
                {
                    errors.push(format!(
                        "event {j}: assessment on activation {} after \
                         transition to off (event {index})",
                        str_of(payload_j.get("activation_id")).unwrap_or("<missing>")
                    ));
                } else if ev.get("event_type").and_then(Value::as_str)
                    == Some("retrieval_result_committed")
                {
                    errors.push(format!(
                        "event {j}: committed result after transition to off \
                         (event {index})"
                    ));
                }
            }
        } else if new_mode == "local_browser" {
            let capability_status = str_of(payload.get("capability_status"));
            if !matches!(
                capability_status,
                Some("available") | Some("unsupported") | Some("degraded")
            ) {
                errors.push(format!(
                    "event {index}: capability_status {capability_status:?} \
                     missing/invalid on local_browser transition — the schema \
                     requires one of available/unsupported/degraded"
                ));
                continue;
            }
            if capability_status == Some("available") {
                continue;
            }
            let capability = capability_status.unwrap_or_default();
            for (offset, ev) in events[index + 1..window_end].iter().enumerate() {
                let j = index + 1 + offset;
                let payload_j = ev.get("payload").cloned().unwrap_or(Value::Null);
                if ev.get("event_type").and_then(Value::as_str)
                    == Some("retrieval_result_committed")
                {
                    errors.push(format!(
                        "event {j}: committed result under local_browser mode \
                         with capability_status={capability} (event {index}) — \
                         capability unavailable, no silent fallback allowed"
                    ));
                } else if ev.get("event_type").and_then(Value::as_str) == Some("tool_completed")
                    && (contains(RETRIEVAL_TARGETS, str_of(payload_j.get("target")))
                        || contains(HOST_LANE_RETRIEVAL_TOOLS, str_of(payload_j.get("tool"))))
                    && payload_j.get("status").and_then(Value::as_str) != Some("error")
                {
                    errors.push(format!(
                        "event {j}: retrieval dispatch {} completed non-error \
                         under local_browser mode with \
                         capability_status={capability} (event {index}) — \
                         capability unavailable must fail explicitly",
                        str_of(payload_j.get("tool")).unwrap_or("<missing>")
                    ));
                }
            }
        }
    }
    if bootstrap_count > 1 {
        errors.push(format!(
            "journal has {bootstrap_count} session_bootstrap transitions — \
             at most one allowed"
        ));
    }
    errors
}

// ── retrieval enable gate + browser launch facts (0t, ADR-0010 §14.65) ──

/// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.1/§3.3, v1.3): 检索启用门
/// 不变量——未启用会话无检索工具声明、无检索 ToolStarted。三值检索模式 γ
/// 退役后 enable gate 是唯一授权门。实现口径与 Python
/// `_verify_v02_retrieval_enable_gate` 完全一致：期刊存在
/// request_header_change 且所有 header 都未声明过检索族工具时，任何检索
/// ToolStarted 都是违反；无 header 事件（旧刊/精简夹具）时规则空转。
pub fn verify_retrieval_enable_gate(events: &[Value]) -> Vec<String> {
    use toolsets::{RETRIEVAL_TARGETS, contains, is_retrieval_mode_gated_tool};

    let mut errors = Vec::new();
    let mut header_count = 0usize;
    let mut declared_retrieval = false;
    for event in events {
        if !is_v02(event) || event["event_type"] != "request_header_change" {
            continue;
        }
        header_count += 1;
        let tools = event
            .get("payload")
            .and_then(|p| p.get("tools"))
            .and_then(Value::as_array);
        if let Some(tools) = tools
            && tools
                .iter()
                .filter_map(Value::as_str)
                .any(|t| is_retrieval_mode_gated_tool(Some(t)))
        {
            declared_retrieval = true;
        }
    }
    if header_count == 0 || declared_retrieval {
        return errors;
    }
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) || event["event_type"] != "tool_started" {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let tool = str_of(payload.get("tool"));
        let target = str_of(payload.get("target"));
        if tool.is_some_and(|t| is_retrieval_mode_gated_tool(Some(t)))
            || contains(RETRIEVAL_TARGETS, target)
        {
            errors.push(format!(
                "event {index}: retrieval dispatch {} tool_started in a \
                 journal whose request headers never declared a retrieval \
                 tool (enable gate — 未启用会话无检索 ToolStarted)",
                str_of(payload.get("tool")).unwrap_or("<missing>")
            ));
        }
    }
    errors
}

/// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.3, v1.3): 浏览器启动/探活
/// 事实事件存在性义务——launch 层失败的 browser_read 调用必须在同期刊中
/// 由更早的 `browser_launch_result`(status=failure) 承托。payload shape
/// 由 schema/registry 执法；页面级失败按真实类别正常回传，不在此义务内。
pub fn verify_browser_launch_result(events: &[Value]) -> Vec<String> {
    use toolsets::{HOST_LANE_RETRIEVAL_TOOLS, contains};

    let mut errors = Vec::new();
    let failure_indices: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            is_v02(event)
                && event["event_type"] == "browser_launch_result"
                && event["payload"]["status"] == "failure"
        })
        .map(|(index, _)| index)
        .collect();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) || event["event_type"] != "tool_completed" {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        if contains(HOST_LANE_RETRIEVAL_TOOLS, str_of(payload.get("tool")))
            && payload.get("status").and_then(Value::as_str) == Some("error")
            && payload.get("error").and_then(Value::as_str) == Some("browser_launch_failed")
            && !failure_indices.iter().any(|&fi| fi < index)
        {
            errors.push(format!(
                "event {index}: {} launch failure lacks a preceding \
                 browser_launch_result failure fact event",
                str_of(payload.get("tool")).unwrap_or("<missing>")
            ));
        }
    }
    errors
}

// ── ledger fold families ────────────────────────────────────────────────

/// Python `_verify_v02_ledger_fold_advance` (ADR-0010 §14.26): valid fold
/// points, `view_estimate_after < view_estimate_tokens`, and per-run window
/// invariants (fold_start constant, fold_cut strictly increasing,
/// rounds_folded non-decreasing) reset by `context_compressed`.
pub fn verify_ledger_fold_advance(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    // run_id -> Some((fold_start, fold_cut, rounds_folded)) | None after reset
    let mut window: std::collections::HashMap<String, Option<(i64, i64, i64)>> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
        if event_type == "context_compressed" {
            window.insert(run_id.to_string(), None);
            continue;
        }
        if event_type != "ledger_fold_advance" {
            continue;
        }
        let payload = &event["payload"];
        let fold_start = py_int(payload.get("fold_start"));
        let fold_cut = py_int(payload.get("fold_cut"));
        let rounds_folded = py_int(payload.get("rounds_folded"));
        let estimate = py_int(payload.get("view_estimate_tokens"));
        let estimate_after = py_int(payload.get("view_estimate_after"));
        let agent_role = str_of(payload.get("agent_role"));

        match fold_start {
            None => errors.push(format!(
                "event {index}: ledger_fold_advance fold_start must be a \
                 non-negative integer, got {:?}",
                payload.get("fold_start")
            )),
            Some(v) if v < 0 => errors.push(format!(
                "event {index}: ledger_fold_advance fold_start must be a \
                 non-negative integer, got {v}"
            )),
            _ => {}
        }
        match fold_cut {
            None => errors.push(format!(
                "event {index}: ledger_fold_advance fold_cut must be a \
                 non-negative integer, got {:?}",
                payload.get("fold_cut")
            )),
            Some(v) if v < 0 => errors.push(format!(
                "event {index}: ledger_fold_advance fold_cut must be a \
                 non-negative integer, got {v}"
            )),
            _ => {}
        }
        if let (Some(start), Some(cut)) = (fold_start, fold_cut)
            && start >= cut
        {
            errors.push(format!(
                "event {index}: ledger_fold_advance invariant \
                 fold_start < fold_cut violated ({start} >= {cut})"
            ));
        }
        match rounds_folded {
            None => errors.push(format!(
                "event {index}: ledger_fold_advance rounds_folded must be a \
                 positive integer, got {:?}",
                payload.get("rounds_folded")
            )),
            Some(v) if v < 1 => errors.push(format!(
                "event {index}: ledger_fold_advance rounds_folded must be a \
                 positive integer, got {v}"
            )),
            _ => {}
        }
        match estimate {
            None => errors.push(format!(
                "event {index}: ledger_fold_advance view_estimate_tokens must \
                 be a non-negative integer, got {:?}",
                payload.get("view_estimate_tokens")
            )),
            Some(v) if v < 0 => errors.push(format!(
                "event {index}: ledger_fold_advance view_estimate_tokens must \
                 be a non-negative integer, got {v}"
            )),
            _ => {}
        }
        match estimate_after {
            None => errors.push(format!(
                "event {index}: ledger_fold_advance view_estimate_after must \
                 be a non-negative integer, got {:?}",
                payload.get("view_estimate_after")
            )),
            Some(after) if after < 0 => errors.push(format!(
                "event {index}: ledger_fold_advance view_estimate_after must \
                 be a non-negative integer, got {after}"
            )),
            Some(after) => {
                if let Some(estimate) = estimate
                    && after >= estimate
                {
                    errors.push(format!(
                        "event {index}: ledger_fold_advance \
                         view_estimate_after {after} must be below the \
                         triggering estimate {estimate} (a real advance resets \
                         below the fold trigger)"
                    ));
                }
            }
        }
        if !matches!(agent_role, Some(role) if LANE_ROLES.contains(&role)) {
            errors.push(format!(
                "event {index}: ledger_fold_advance agent_role must be \
                 main/internal_retrieval/external_retrieval, got {agent_role:?}"
            ));
        }

        if let Some(Some((prev_start, prev_cut, prev_rounds))) = window.get(run_id) {
            if let Some(start) = fold_start
                && start != *prev_start
            {
                errors.push(format!(
                    "event {index}: ledger_fold_advance fold_start changed \
                     inside a fold window ({prev_start} -> {start})"
                ));
            }
            if let Some(cut) = fold_cut
                && cut <= *prev_cut
            {
                errors.push(format!(
                    "event {index}: ledger_fold_advance fold_cut must increase \
                     inside a fold window (previous {prev_cut}, got {cut})"
                ));
            }
            if let Some(rounds) = rounds_folded
                && rounds < *prev_rounds
            {
                errors.push(format!(
                    "event {index}: ledger_fold_advance rounds_folded shrank \
                     inside a fold window ({prev_rounds} -> {rounds})"
                ));
            }
        }
        if let (Some(start), Some(cut), Some(rounds)) = (fold_start, fold_cut, rounds_folded) {
            window.insert(run_id.to_string(), Some((start, cut, rounds)));
        }
    }
    errors
}

/// Python `_verify_v02_ledger_fold_write_failed` (ADR-0010 §14.28): full
/// audit shape, consecutive-failure attempt counter (+1 per failure, reset by
/// a successful append), `disabled == (attempt >= 3)` and no failures after
/// the budget is exhausted.
pub fn verify_ledger_fold_write_failed(events: &[Value]) -> Vec<String> {
    const FOLD_WRITE_FAILURE_LIMIT: i64 = 3;
    let mut errors = Vec::new();
    // run_id -> (previous attempt, budget exhausted)
    let mut bursts: std::collections::HashMap<String, (i64, bool)> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
        if event_type == "ledger_fold_advance" {
            bursts.insert(run_id.to_string(), (0, false));
            continue;
        }
        if event_type != "ledger_fold_write_failed" {
            continue;
        }
        let payload = &event["payload"];
        let ledger_path = str_of(payload.get("ledger_path"));
        let attempt = py_int(payload.get("attempt"));
        let disabled = match payload.get("disabled") {
            Some(Value::Bool(b)) => Some(*b),
            _ => None,
        };
        let rows = py_int(payload.get("rows"));
        let estimate = py_int(payload.get("view_estimate_tokens"));
        let agent_role = str_of(payload.get("agent_role"));

        if ledger_path.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: ledger_fold_write_failed ledger_path must be \
                 a non-empty string, got {:?}",
                payload.get("ledger_path")
            ));
        }
        match attempt {
            None => errors.push(format!(
                "event {index}: ledger_fold_write_failed attempt must be a \
                 positive integer, got {:?}",
                payload.get("attempt")
            )),
            Some(v) if v < 1 => errors.push(format!(
                "event {index}: ledger_fold_write_failed attempt must be a \
                 positive integer, got {v}"
            )),
            _ => {}
        }
        if disabled.is_none() {
            errors.push(format!(
                "event {index}: ledger_fold_write_failed disabled must be a \
                 boolean, got {:?}",
                payload.get("disabled")
            ));
        }
        match rows {
            None => errors.push(format!(
                "event {index}: ledger_fold_write_failed rows must be a \
                 positive integer, got {:?}",
                payload.get("rows")
            )),
            Some(v) if v < 1 => errors.push(format!(
                "event {index}: ledger_fold_write_failed rows must be a \
                 positive integer, got {v}"
            )),
            _ => {}
        }
        match estimate {
            None => errors.push(format!(
                "event {index}: ledger_fold_write_failed \
                 view_estimate_tokens must be a non-negative integer, got {:?}",
                payload.get("view_estimate_tokens")
            )),
            Some(v) if v < 0 => errors.push(format!(
                "event {index}: ledger_fold_write_failed \
                 view_estimate_tokens must be a non-negative integer, got {v}"
            )),
            _ => {}
        }
        if !matches!(agent_role, Some(role) if LANE_ROLES.contains(&role)) {
            errors.push(format!(
                "event {index}: ledger_fold_write_failed agent_role must be \
                 main/internal_retrieval/external_retrieval, got {agent_role:?}"
            ));
        }

        let (prev_attempt, exhausted) = bursts.get(run_id).copied().unwrap_or((0, false));
        if exhausted {
            errors.push(format!(
                "event {index}: ledger_fold_write_failed after the failure \
                 budget was exhausted (disabled=true) for run {run_id:?} — \
                 folding is disabled, no further events"
            ));
        }
        if let Some(attempt) = attempt
            && attempt != prev_attempt + 1
        {
            errors.push(format!(
                "event {index}: ledger_fold_write_failed attempt must \
                 increase by 1 across consecutive failures (previous \
                 {prev_attempt}, got {attempt})"
            ));
        }
        if let (Some(attempt), Some(disabled)) = (attempt, disabled) {
            if disabled != (attempt >= FOLD_WRITE_FAILURE_LIMIT) {
                errors.push(format!(
                    "event {index}: ledger_fold_write_failed disabled must \
                     equal (attempt >= {FOLD_WRITE_FAILURE_LIMIT}) per the \
                     producer budget, got attempt={attempt} disabled={disabled}"
                ));
            }
            bursts.insert(run_id.to_string(), (attempt, disabled));
        }
    }
    errors
}

// ── policy_denial ───────────────────────────────────────────────────────

/// Python `_verify_v02_policy_denial` (P0-C S3): structured policy denials on
/// refusal completions — known source, non-empty code, string reason,
/// `status=error` + non-zero `exit_code`, and source→tool-family consistency.
pub fn verify_policy_denial(events: &[Value]) -> Vec<String> {
    use toolsets::{ACAF_TICKETED_TOOLS, WORK_TOOLS, contains, is_retrieval_mode_gated_tool};
    // Python `_PERMISSION_GATED_TOOLS = _WORK_TOOLS | _RETRIEVAL_MODE_GATED_TOOLS`
    // is an EXACT name set, derived from the two tables on both sides
    // (`|WORK_TOOLS| + |RETRIEVAL_MODE_GATED_TOOLS|`; 0aj-review 2026-09-16:
    // 24 + 4 = 28 after `blackboard_write` joined the work table, was 23 + 4 = 27;
    // 0ap 2026-09-18: 25 + 4 = 29 after `context_compress` joined)
    // — NOT the prefix predicate `_is_retrieval_mode_gated_tool` (which
    // additionally admits web_search/web_fetch/retrieve_project_*). Using the
    // predicate here would let source=permission through on web-family tools
    // where Python errors (S2b review P1, 2026-09-06).
    const RETRIEVAL_MODE_GATED_TOOLS: &[&str] = toolsets::RETRIEVAL_MODE_GATED_TOOLS;
    let permission_gated = |name: Option<&str>| {
        contains(WORK_TOOLS, name) || contains(RETRIEVAL_MODE_GATED_TOOLS, name)
    };

    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("tool_completed") {
            continue;
        }
        let payload = &event["payload"];
        let Some(denial) = payload.get("policy_denial") else {
            continue;
        };
        let source = str_of(denial.get("source"));
        let tool = str_of(payload.get("tool"));
        let status = str_of(payload.get("status"));
        // Python bool guard: bool is an int, but True/False must be rejected
        // as exit codes (`isinstance(exit_code, bool)` check).
        let exit_code = match payload.get("exit_code") {
            Some(Value::Number(n)) => n.as_i64(),
            _ => None,
        };
        if status != Some("error") {
            errors.push(format!(
                "event {index}: policy_denial requires status=error (refusal \
                 completion); got {status:?}"
            ));
        }
        if exit_code.is_none_or(|code| code == 0) {
            errors.push(format!(
                "event {index}: policy_denial requires a non-zero exit_code \
                 (refusals are exit_code=Some(1)); got {:?}",
                payload.get("exit_code")
            ));
        }
        let code = str_of(denial.get("code"));
        if code.is_none_or(|c| c.trim().is_empty()) {
            errors.push(format!(
                "event {index}: policy_denial.code must be a non-empty string"
            ));
        }
        if !denial.get("reason").is_some_and(Value::is_string) {
            errors.push(format!(
                "event {index}: policy_denial.reason must be a string"
            ));
        }
        match source {
            Some("acaf") => {
                if !contains(ACAF_TICKETED_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: source=acaf on non-ticketed tool \
                         {tool:?} (ticketed family: \
                         search_replace/run_tests/run_terminal_cmd)"
                    ));
                }
            }
            Some("retrieval_mode") => {
                if !is_retrieval_mode_gated_tool(tool) {
                    errors.push(format!(
                        "event {index}: source=retrieval_mode on non-retrieval \
                         tool {tool:?}"
                    ));
                }
            }
            Some("permission") => {
                if !permission_gated(tool) {
                    errors.push(format!(
                        "event {index}: source=permission on \
                         non-permission-gated tool {tool:?}"
                    ));
                }
            }
            Some("taint") => {
                if !contains(WORK_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: source=taint on non-work tool {tool:?}"
                    ));
                }
            }
            other => {
                errors.push(format!(
                    "event {index}: policy_denial.source {other:?} not in \
                     permission/acaf/retrieval_mode/taint"
                ));
            }
        }
    }
    errors
}

// ── failure_target ──────────────────────────────────────────────────────

/// Python `_verify_v02_failure_target` (MECHANICAL-LAYER-MATH-CALCULUS F4
/// §5.3): failure-target identity only on failure completions, kind enum,
/// 64-char lowercase sha256 id, kind-specific carried fields and
/// kind→tool-family consistency.
pub fn verify_failure_target(events: &[Value]) -> Vec<String> {
    use toolsets::{
        ANCHOR_TARGET_TOOLS, CMD_TARGET_TOOLS, FILE_TARGET_TOOLS, URL_TARGET_TOOLS, contains,
    };

    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("tool_completed") {
            continue;
        }
        let payload = &event["payload"];
        let Some(ft) = payload.get("failure_target") else {
            continue;
        };
        if ft.is_null() {
            continue;
        }
        if payload.get("status").and_then(Value::as_str) != Some("error") {
            errors.push(format!(
                "event {index}: failure_target requires status=error \
                 (failure completion); got {:?}",
                payload.get("status")
            ));
        }
        let Some(ft) = ft.as_object() else {
            errors.push(format!(
                "event {index}: failure_target must be an object; got {:?}",
                ft
            ));
            continue;
        };
        let kind = str_of(ft.get("kind"));
        let Some(kind) = kind else {
            errors.push(format!(
                "event {index}: failure_target.kind {:?} not in \
                 cmd_target/anchor_target/file_target/url_target",
                ft.get("kind")
            ));
            continue;
        };
        if !matches!(
            kind,
            "cmd_target" | "anchor_target" | "file_target" | "url_target"
        ) {
            errors.push(format!(
                "event {index}: failure_target.kind {kind:?} not in \
                 cmd_target/anchor_target/file_target/url_target"
            ));
            continue;
        }
        let ft_id = str_of(ft.get("id"));
        if !is_sha256_hex(ft_id) {
            errors.push(format!(
                "event {index}: failure_target.id must be 64-char lowercase \
                 sha256 hex; got {ft_id:?}"
            ));
        }
        let tool = str_of(payload.get("tool"));
        match kind {
            "cmd_target" => {
                if !contains(CMD_TARGET_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: kind=cmd_target on non-terminal tool \
                         {tool:?} (terminal family: run_terminal_cmd/run_tests)"
                    ));
                }
                let preview_ok = str_of(ft.get("cmd_preview"))
                    .map(|preview| utf8_len(preview) <= 80)
                    .unwrap_or(false);
                if !preview_ok {
                    errors.push(format!(
                        "event {index}: kind=cmd_target requires cmd_preview \
                         (string ≤ 80 UTF-8 bytes)"
                    ));
                }
            }
            "anchor_target" => {
                if !contains(ANCHOR_TARGET_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: kind=anchor_target on non-anchor tool \
                         {tool:?} (search_replace only)"
                    ));
                }
                let path = str_of(ft.get("path"));
                if path.is_none_or(str::is_empty) {
                    errors.push(format!(
                        "event {index}: kind=anchor_target requires non-empty path"
                    ));
                }
                if !is_sha256_hex(str_of(ft.get("anchor_hash"))) {
                    errors.push(format!(
                        "event {index}: kind=anchor_target requires \
                         anchor_hash (64-char sha256 hex)"
                    ));
                }
                // Python: int, not bool, ≥ 0.
                let size_ok = matches!(ft.get("size"), Some(Value::Number(n)) if n.as_i64().is_some_and(|v| v >= 0));
                if !size_ok {
                    errors.push(format!(
                        "event {index}: kind=anchor_target requires size ≥ 0"
                    ));
                }
            }
            "file_target" => {
                if !contains(FILE_TARGET_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: kind=file_target on non-file tool \
                         {tool:?} (search_replace/read_file/grep)"
                    ));
                }
                let path = str_of(ft.get("path"));
                if path.is_none_or(str::is_empty) {
                    errors.push(format!(
                        "event {index}: kind=file_target requires non-empty path"
                    ));
                }
            }
            "url_target" => {
                if !contains(URL_TARGET_TOOLS, tool) {
                    errors.push(format!(
                        "event {index}: kind=url_target on non-web tool \
                         {tool:?} (web_fetch/browser_read)"
                    ));
                }
                let url = str_of(ft.get("canonical_url"));
                if url.is_none_or(str::is_empty) {
                    errors.push(format!(
                        "event {index}: kind=url_target requires non-empty \
                         canonical_url"
                    ));
                }
            }
            _ => unreachable!("kind checked above"),
        }
    }
    errors
}

// ── failure_agg_coverage (0q) ───────────────────────────────────────────

/// Tools whose error-shaped completions the 0q funnel covers — the union of
/// the four identity-family tool tables (mirrors the Rust producer
/// `failure_target::identity_capable` in orz-loop).
fn failure_agg_capable_tool(tool: Option<&str>) -> bool {
    use toolsets::{
        ANCHOR_TARGET_TOOLS, CMD_TARGET_TOOLS, FILE_TARGET_TOOLS, URL_TARGET_TOOLS, contains,
    };
    contains(CMD_TARGET_TOOLS, tool)
        || contains(ANCHOR_TARGET_TOOLS, tool)
        || contains(FILE_TARGET_TOOLS, tool)
        || contains(URL_TARGET_TOOLS, tool)
}

/// Python `_verify_v02_failure_agg_coverage` (0q 统一失败事件管线,
/// 2026-09-08, ADR-0010 §14.63): coverage reconciliation for the
/// write-side failure funnel — "没有漏盖" becomes an executable assertion.
///
/// - Grandfather anchor: the rule fires only on journals whose `run_started`
///   payload declares `failure_pipeline: "funnel-v1"`; older journals are
///   not retroactively enforced (0q 设计 §3.2-4). Any other declared
///   version is an error.
/// - On a post-funnel journal, every error-shaped `tool_completed` of an
///   identity-capable tool carries exactly one of `failure_target`
///   (identity stamped → agg row written by the same funnel pass) or
///   `failure_agg_absent: true` (funnel evaluated, deliberately not
///   aggregated). Neither = the F-C class of bug (missed stamp); both =
///   double-write. A structured `policy_denial` envelope is itself the
///   evaluated-no-stamp evidence (0p S2 裁决) and needs neither field.
/// - `failure_agg_absent` outside an error shape is producer misuse.
pub fn verify_failure_agg_coverage(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut post_funnel = false;
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("run_started") {
            continue;
        }
        match str_of(event["payload"].get("failure_pipeline")) {
            None => {}
            Some("funnel-v1") => post_funnel = true,
            Some(other) => errors.push(format!(
                "event {index}: run_started failure_pipeline {other:?} is not \
                 a known failure-pipeline version (funnel-v1)"
            )),
        }
    }
    if !post_funnel {
        return errors;
    }
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("tool_completed") {
            continue;
        }
        let payload = &event["payload"];
        if !failure_agg_capable_tool(str_of(payload.get("tool"))) {
            continue;
        }
        // 0p S2 信封优先口径：结构化拒绝信封自身即评估证据，两字段都
        // 不要求。
        if payload.get("policy_denial").is_some() {
            continue;
        }
        let status = str_of(payload.get("status"));
        let exit_code = py_int(payload.get("exit_code"));
        let error_shaped = status == Some("error") || exit_code.is_some_and(|code| code != 0);
        let marker = payload.get("failure_agg_absent");
        if !error_shaped {
            if marker.is_some() {
                errors.push(format!(
                    "event {index}: failure_agg_absent on a non-error \
                     completion (misuse); got {marker:?}"
                ));
            }
            continue;
        }
        if let Some(marker) = marker
            && *marker != Value::Bool(true)
        {
            errors.push(format!(
                "event {index}: failure_agg_absent must be the literal true; \
                 got {marker}"
            ));
        }
        let marker_ok = marker == Some(&Value::Bool(true));
        let has_target = matches!(payload.get("failure_target"), Some(Value::Object(_)));
        match (has_target, marker_ok) {
            (true, true) => errors.push(format!(
                "event {index}: failure_target and failure_agg_absent are \
                 mutually exclusive (XOR); funnel double-write"
            )),
            (false, false) => errors.push(format!(
                "event {index}: error-shaped completion of identity-capable \
                 tool {:?} carries neither failure_target nor \
                 failure_agg_absent (missed failure-aggregation stamp)",
                str_of(payload.get("tool")).unwrap_or("<missing>")
            )),
            _ => {}
        }
    }
    errors
}

// ── lifecycle ───────────────────────────────────────────────────────────

struct LifecycleEvent<'a> {
    index: usize,
    payload: &'a Value,
}

/// Python `_verify_v02_lifecycle` (ADR-0010 §4.4): disposition/assessment/
/// close-record state machine — CAS revision binding, accepted-on-current-
/// revision, one accepted decision per assessment, revision monotonicity and
/// continue+1, close freeze, and byte-identical idempotent replays.
pub fn verify_lifecycle(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut assessments: Vec<LifecycleEvent> = Vec::new();
    let mut dispositions: Vec<LifecycleEvent> = Vec::new();
    let mut closes: Vec<LifecycleEvent> = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let payload = match event.get("payload") {
            Some(payload) => payload,
            None => continue,
        };
        match event.get("event_type").and_then(Value::as_str) {
            Some("information_sufficiency_assessment") => {
                assessments.push(LifecycleEvent { index, payload })
            }
            Some("retrieval_parent_disposition") => {
                dispositions.push(LifecycleEvent { index, payload })
            }
            Some("retrieval_close_record") => closes.push(LifecycleEvent { index, payload }),
            _ => {}
        }
    }
    if assessments.is_empty() && dispositions.is_empty() && closes.is_empty() {
        return errors;
    }

    fn pstr<'a>(payload: &'a Value, key: &str) -> &'a str {
        payload.get(key).and_then(Value::as_str).unwrap_or("")
    }

    // First-occurrence binding: later replays never rebind references.
    let mut assessment_first: std::collections::HashMap<&str, (usize, &Value)> =
        std::collections::HashMap::new();
    for entry in &assessments {
        let a_id = pstr(entry.payload, "assessment_id");
        assessment_first
            .entry(a_id)
            .or_insert((entry.index, entry.payload));
    }
    let mut disposition_first: std::collections::HashMap<&str, (usize, &Value)> =
        std::collections::HashMap::new();
    for entry in &dispositions {
        let d_id = pstr(entry.payload, "disposition_id");
        disposition_first
            .entry(d_id)
            .or_insert((entry.index, entry.payload));
    }
    // Restore declarations legalize cross-run assessment references (§3.3).
    let mut restore_assessment_declared: std::collections::HashMap<&str, (usize, &Value)> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str)
                != Some("retrieval_activation_restored")
        {
            continue;
        }
        let payload = &event["payload"];
        if payload.get("status").and_then(Value::as_str) == Some("awaiting_disposition") {
            let a_id = pstr(payload, "assessment_id");
            restore_assessment_declared
                .entry(a_id)
                .or_insert((index, payload));
        }
    }

    // FIRST close per activation; a second close is itself a §4.4 violation.
    let mut close_first: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for entry in &closes {
        let payload = entry.payload;
        let activation = pstr(payload, "activation_id");
        let effort = payload.get("effort").and_then(Value::as_str);
        // 0k 第二批 (2026-08-30): optional effort tier whitelist self-check.
        if let Some(effort) = effort
            && !matches!(effort, "standard" | "extended" | "deep")
        {
            errors.push(format!(
                "event {}: close record effort {effort:?} is not a known \
                 delegation tier (standard/extended/deep)",
                entry.index
            ));
        }
        if let Some(&first) = close_first.get(activation) {
            errors.push(format!(
                "event {}: second close record {} on activation {activation} \
                 (first at event {first})",
                entry.index,
                pstr(payload, "close_record_id")
            ));
        } else {
            close_first.insert(activation, entry.index);
        }
    }

    macro_rules! after_close {
        ($index:expr, $what:expr, $activation:expr) => {{
            let closed_at = close_first.get($activation).copied();
            match closed_at {
                Some(closed_at) if closed_at < $index => {
                    errors.push(format!(
                        "event {}: {} on activation {} after its close record \
                         (event {closed_at})",
                        $index, $what, $activation
                    ));
                    true
                }
                _ => false,
            }
        }};
    }

    // Disposition pass (journal order): CAS, accepted-on-current, conflicts,
    // replay identity, close freeze.
    let mut current_revision: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let mut assessment_decision: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut continue_pending: std::collections::HashMap<String, (usize, i64)> =
        std::collections::HashMap::new();
    let mut seen_max: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for entry in &dispositions {
        let index = entry.index;
        let payload = entry.payload;
        let disp_id = pstr(payload, "disposition_id");
        let &(first_index, first_payload) = &disposition_first[disp_id];
        if first_index != index {
            // Idempotent replay: identical canonical payload required.
            if canonical_bytes(first_payload) != canonical_bytes(payload) {
                errors.push(format!(
                    "event {index}: disposition {disp_id} replayed with a \
                     conflicting payload"
                ));
            }
            continue;
        }

        let activation = pstr(payload, "activation_id");
        if after_close!(index, format!("disposition {disp_id}"), activation) {
            continue;
        }

        let a_ref = pstr(payload, "assessment_id");
        let a_revision;
        match assessment_first.get(a_ref) {
            None => {
                // Cross-run reference: only a preceding restore declaration of
                // the same activation makes the assessment known (§3.3).
                match restore_assessment_declared.get(a_ref) {
                    None => {
                        errors.push(format!(
                            "event {index}: disposition {disp_id} references \
                             unknown assessment {a_ref}"
                        ));
                        continue;
                    }
                    Some(&(r_index, r_payload)) => {
                        if r_index >= index {
                            errors.push(format!(
                                "event {index}: disposition {disp_id} \
                                 references assessment {a_ref} restored at \
                                 event {r_index}, which does not precede it"
                            ));
                        }
                        if pstr(r_payload, "activation_id") != activation {
                            errors.push(format!(
                                "event {index}: disposition {disp_id} \
                                 activation_id {activation} != restored \
                                 activation {} declaring assessment {a_ref}",
                                pstr(r_payload, "activation_id")
                            ));
                        }
                        a_revision = py_int(r_payload.get("contract_revision")).unwrap_or(0);
                    }
                }
            }
            Some(&(a_index, a_payload)) => {
                if a_index >= index {
                    errors.push(format!(
                        "event {index}: disposition {disp_id} references \
                         assessment {a_ref} that does not precede it"
                    ));
                }
                if pstr(a_payload, "activation_id") != activation {
                    errors.push(format!(
                        "event {index}: disposition {disp_id} activation_id \
                         {activation} != referenced assessment {a_ref} \
                         activation_id {}",
                        pstr(a_payload, "activation_id")
                    ));
                }
                a_revision = py_int(a_payload.get("contract_revision")).unwrap_or(0);
            }
        }
        let expected_revision = py_int(payload.get("expected_contract_revision")).unwrap_or(0);
        if expected_revision != a_revision {
            errors.push(format!(
                "event {index}: disposition {disp_id} \
                 expected_contract_revision {expected_revision} != referenced \
                 assessment {a_ref} contract_revision {a_revision}"
            ));
        }

        let outcome = pstr(payload, "outcome");
        let decision = pstr(payload, "decision");
        if outcome == "accepted" {
            // Accepted dispositions must act on the current revision (§4.4).
            let cur = current_revision
                .get(activation)
                .copied()
                .unwrap_or(a_revision);
            if expected_revision != cur {
                errors.push(format!(
                    "event {index}: disposition {disp_id} accepted on \
                     activation {activation} at revision {expected_revision}, \
                     current revision is {cur}"
                ));
            }
            // Conflicting decision on the same assessment (§4.4).
            if let Some(prior) = assessment_decision.get(a_ref)
                && prior != decision
            {
                errors.push(format!(
                    "event {index}: disposition {disp_id} decision {decision} \
                     conflicts with prior accepted decision {prior} on \
                     assessment {a_ref}"
                ));
            }
            assessment_decision.insert(a_ref.to_string(), decision.to_string());
            if decision == "continue" {
                current_revision.insert(activation.to_string(), cur + 1);
                continue_pending.insert(activation.to_string(), (index, cur + 1));
            }
            // decision == close: revision frozen; the close record commits it.
        } else if outcome != "rejected_stale" && outcome != "rejected_conflicting" {
            errors.push(format!(
                "event {index}: disposition {disp_id} unknown outcome {outcome:?}"
            ));
        }
    }

    // Close pass: normal_close bindings to the validated disposition and the
    // referenced assessment (or its restore declaration).
    for entry in &closes {
        let index = entry.index;
        let payload = entry.payload;
        let close_id = pstr(payload, "close_record_id");
        let activation = pstr(payload, "activation_id");
        if after_close!(index, format!("close record {close_id}"), activation) {
            continue;
        }
        if pstr(payload, "terminal_reason") != "normal_close" {
            continue;
        }
        let d_ref = pstr(payload, "validated_disposition_id");
        let Some(&(d_index, d_payload)) = disposition_first.get(d_ref) else {
            errors.push(format!(
                "event {index}: close record {close_id} references \
                 disposition {d_ref} that does not precede it"
            ));
            continue;
        };
        if d_index >= index {
            errors.push(format!(
                "event {index}: close record {close_id} references \
                 disposition {d_ref} that does not precede it"
            ));
            continue;
        }
        if pstr(d_payload, "decision") != "close" {
            errors.push(format!(
                "event {index}: close record {close_id} references \
                 disposition {d_ref} with decision != close"
            ));
        }
        let d_outcome = pstr(d_payload, "outcome");
        if d_outcome != "accepted" && d_outcome != "replayed_idempotent" {
            errors.push(format!(
                "event {index}: close record {close_id} references \
                 disposition {d_ref} with outcome {d_outcome} — only a \
                 validated close disposition commits a close record"
            ));
        }
        if py_int(d_payload.get("expected_contract_revision")).unwrap_or(0)
            != py_int(payload.get("contract_revision")).unwrap_or(0)
        {
            errors.push(format!(
                "event {index}: close record {close_id} contract_revision {} \
                 != disposition {d_ref} expected_contract_revision {}",
                py_int(payload.get("contract_revision")).unwrap_or(0),
                py_int(d_payload.get("expected_contract_revision")).unwrap_or(0)
            ));
        }
        if pstr(d_payload, "assessment_id") != pstr(payload, "assessment_id") {
            errors.push(format!(
                "event {index}: close record {close_id} assessment_id {} != \
                 disposition {d_ref} assessment_id {}",
                pstr(payload, "assessment_id"),
                pstr(d_payload, "assessment_id")
            ));
        }
        if pstr(d_payload, "activation_id") != activation {
            errors.push(format!(
                "event {index}: close record {close_id} activation_id \
                 {activation} != disposition {d_ref} activation_id {}",
                pstr(d_payload, "activation_id")
            ));
        }
        // Cross-field binding to the referenced assessment (§4.4 identity
        // chain): contract and result digests must match when carried.
        let a_ref = pstr(payload, "assessment_id");
        let contract_id = payload.get("contract_id");
        let result_digest = payload.get("result_digest");
        match assessment_first.get(a_ref) {
            Some(&(_, a_payload)) => {
                if a_payload.get("contract_id") != contract_id {
                    errors.push(format!(
                        "event {index}: close record {close_id} contract_id \
                         {:?} != referenced assessment {a_ref} contract_id {:?}",
                        contract_id,
                        a_payload.get("contract_id")
                    ));
                }
                if a_payload.get("result_digest") != result_digest {
                    errors.push(format!(
                        "event {index}: close record {close_id} result_digest \
                         {:?} != referenced assessment {a_ref} result_digest {:?}",
                        result_digest,
                        a_payload.get("result_digest")
                    ));
                }
            }
            None => {
                if let Some(&(_, r_payload)) = restore_assessment_declared.get(a_ref) {
                    if r_payload.get("contract_id") != contract_id {
                        errors.push(format!(
                            "event {index}: close record {close_id} \
                             contract_id {:?} != restored assessment {a_ref} \
                             contract_id {:?}",
                            contract_id,
                            r_payload.get("contract_id")
                        ));
                    }
                    let r_digest = r_payload.get("result_digest");
                    if py_truthy(r_digest) && r_digest != result_digest {
                        errors.push(format!(
                            "event {index}: close record {close_id} \
                             result_digest {:?} != restored assessment {a_ref} \
                             result_digest {:?}",
                            result_digest, r_digest
                        ));
                    }
                }
            }
        }
    }

    // Assessment pass: replay identity, continue+1, monotonic revisions.
    for entry in &assessments {
        let index = entry.index;
        let payload = entry.payload;
        let activation = pstr(payload, "activation_id");
        let a_id = pstr(payload, "assessment_id");
        if after_close!(index, format!("assessment {a_id}"), activation) {
            continue;
        }
        let &(first_index, first_payload) = &assessment_first[a_id];
        if first_index != index && canonical_bytes(first_payload) != canonical_bytes(payload) {
            errors.push(format!(
                "event {index}: assessment {a_id} replayed with a conflicting \
                 payload"
            ));
        }
        let revision = py_int(payload.get("contract_revision")).unwrap_or(0);
        if first_index == index {
            // continue+1: the first assessment AFTER an accepted continue
            // must carry exactly the incremented revision.
            if let Some(&(continue_index, expected_revision)) = continue_pending.get(activation)
                && index > continue_index
            {
                if revision != expected_revision {
                    errors.push(format!(
                        "event {index}: assessment {a_id} contract_revision \
                         {revision} != continue revision + 1 \
                         ({expected_revision}) on activation {activation}"
                    ));
                }
                continue_pending.remove(activation);
            }
            // Monotonic revisions never decrease (temporal order).
            if let Some(&prev_max) = seen_max.get(activation)
                && revision < prev_max
            {
                errors.push(format!(
                    "event {index}: assessment {a_id} contract_revision \
                     {revision} < previous max {prev_max} on activation \
                     {activation} (revisions never decrease)"
                ));
            }
            seen_max.insert(activation.to_string(), revision);
            current_revision
                .entry(activation.to_string())
                .or_insert(revision);
        }
    }
    errors
}

// ── dispatch ────────────────────────────────────────────────────────────

/// S2b family ids in Python `validate_journal_text` call order (relative
/// order among these seven; Py 3682-3704).
pub const S2B_FAMILIES: &[&str] = &[
    "ledger_fold_advance",
    "ledger_fold_write_failed",
    "lifecycle",
    "retrieval_mode",
    "policy_denial",
    "failure_target",
    "control_tickets",
];

/// S2c family ids (Task D S2c, 2026-09-06): the remaining rule families,
/// implemented in [`super::families_s2c`]. `console_order_written` was
/// retired 2026-09-06 (S2d 裁决一, ADR-0010 §14.57) — 24 → 23.
pub const S2C_FAMILIES: &[&str] = &[
    "inquiry_kind",
    // P0-0x S2 (2026-09-11, ADR-0010 §14.66): the initial-round inquiry
    // shares the orientation_checkpoint event — its trigger/block/position
    // coupling and its one-shot contract are family-stage rules.
    "initial_round_inquiry",
    "plan_write",
    "console_mode_transition",
    "console_order_rejected",
    "tool_running",
    "output_truncation",
    "budget_cue_injected",
    "result_consistency",
    "reason_codes",
    "source_weighting",
    "search_candidate_pool",
    "candidate_prefilter",
    "candidate_count",
    "inject_budget",
    "receipt_event_isomorphism",
    "dep_graph_events",
    "mechanical_audit",
    "recovery_truncation",
    "context_compressed",
    "activation_restore",
    "tool_availability_probe",
    "request_header",
    "probe_accuracy",
    // 0z S2 (2026-09-12, FUS-HOST-RESOURCE-SAFETY §5): host-resource facts.
    "host_resource_snapshot",
    "host_resource_denied",
    "resource_exhausted",
    "run_terminated",
    "process_tree_reaped",
    "reclaim_performed",
    "resource_limit_hit",
];

/// All rule families in the Python `validate_journal_text` call order
/// (Py order; `console_order_written` retired 2026-09-06, S2d 裁决一 /
/// ADR-0010 §14.57 — the write-order chain rule is gone on both judges and
/// NOT converted into a negative check, so historical journals replay clean)
/// `failure_agg_coverage` added 2026-09-08 (0q / ADR-0010 §14.63);
/// `retrieval_enable_gate` + `browser_launch_result` added 2026-09-09
/// (0t / ADR-0010 §14.65);
/// `initial_round_inquiry` added 2026-09-11 (0x S2 / ADR-0010 §14.66);
/// the five 0ac immediate-feedback families (`retrieval_dedupe`,
/// `result_delivered_accounting`, `retrieval_family_probe`,
/// `failure_cause_shape`, `first_result_deadline`) added 2026-09-13
/// (S3-b, F-007 裁决 (a)) — the S2d full-corpus crosscheck order.
pub const ALL_FAMILIES: &[&str] = &[
    "inquiry_kind",
    "initial_round_inquiry",
    "plan_write",
    "console_mode_transition",
    "console_order_rejected",
    "ledger_fold_advance",
    "ledger_fold_write_failed",
    "lifecycle",
    "tool_running",
    "output_truncation",
    "budget_cue_injected",
    "retrieval_mode",
    "retrieval_enable_gate",
    "browser_launch_result",
    "result_consistency",
    "reason_codes",
    "source_weighting",
    "search_candidate_pool",
    "candidate_prefilter",
    "candidate_count",
    "inject_budget",
    "policy_denial",
    "failure_target",
    "failure_agg_coverage",
    "receipt_event_isomorphism",
    "dep_graph_events",
    "mechanical_audit",
    "recovery_truncation",
    "context_compressed",
    "activation_restore",
    "control_tickets",
    "tool_availability_probe",
    "request_header",
    "probe_accuracy",
    // 0z S2 (2026-09-12, FUS-HOST-RESOURCE-SAFETY §5): host-resource facts.
    "host_resource_snapshot",
    "host_resource_denied",
    "resource_exhausted",
    "run_terminated",
    "process_tree_reaped",
    "reclaim_performed",
    "resource_limit_hit",
    // 0ac S3-b (2026-09-13, F-007 裁决 (a)): immediate-feedback families
    // (`journal/immediate_feedback.rs`); Rust↔Python parity covered.
    "retrieval_dedupe",
    "result_delivered_accounting",
    "retrieval_family_probe",
    "failure_cause_shape",
    "first_result_deadline",
];

// ---------------------------------------------------------------------------
// FUS-HOST-RESOURCE-SAFETY §5 families (2026-09-12, 0z S2) — the seven
// host-resource fact families. Payload shapes are pinned by the v0.2 payload
// schemas; these verifiers pin the cross-field contracts the schemas cannot
// express (tier machine keys, phase coupling, outcome accounting).
// ---------------------------------------------------------------------------

/// snake_case machine key: lowercase letters, digits, underscores.
fn is_snake_case_key(value: &Value) -> bool {
    value
        .as_str()
        .map(|s| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                && s.chars()
                    .next()
                    .map(|c| c.is_ascii_lowercase())
                    .unwrap_or(false)
        })
        .unwrap_or(false)
}

const HOST_RESOURCE_TIERS: &[&str] = &[
    "normal",
    "watch",
    "soft",
    "reclaim_direct",
    "hard",
    "unknown",
];

fn is_host_resource_tier(value: &Value) -> bool {
    value
        .as_str()
        .map(|s| HOST_RESOURCE_TIERS.contains(&s))
        .unwrap_or(false)
}

fn is_non_empty_string(value: &Value) -> bool {
    value.as_str().map(|s| !s.is_empty()).unwrap_or(false)
}

/// `host_resource_snapshot`: readings face — one row per tier transition
/// (design §4.5 低频，跨档才落). Tier + trigger machine keys; readings object.
pub fn verify_host_resource_snapshot(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "host_resource_snapshot")
    {
        let payload = &event["payload"];
        if !is_host_resource_tier(&payload["tier"]) {
            errors.push("host_resource_snapshot: tier is not a machine key".to_string());
        }
        let trigger = payload["trigger"].as_str().unwrap_or_default();
        if !matches!(trigger, "tier_change" | "run_start") {
            errors
                .push("host_resource_snapshot: trigger must be tier_change|run_start".to_string());
        }
        if !payload["readings"].is_object() {
            errors.push("host_resource_snapshot: readings must be an object".to_string());
        }
    }
    errors
}

/// `host_resource_denied`: pre-issue refusal — readings + action class; the
/// refusal happens BEFORE dispatch (判据 1's reviewable fact).
pub fn verify_host_resource_denied(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "host_resource_denied")
    {
        let payload = &event["payload"];
        if payload["phase"].as_str() != Some("pre_issue") {
            errors.push("host_resource_denied: phase must be pre_issue".to_string());
        }
        if !is_snake_case_key(&payload["action_class"]) {
            errors.push("host_resource_denied: action_class must be a snake_case key".to_string());
        }
        if !is_host_resource_tier(&payload["tier"]) {
            errors.push("host_resource_denied: tier is not a machine key".to_string());
        }
        if !is_non_empty_string(&payload["tool"]) || !is_non_empty_string(&payload["call_id"]) {
            errors.push("host_resource_denied: tool/call_id required".to_string());
        }
        if !is_non_empty_string(&payload["reason"]) {
            errors.push("host_resource_denied: reason required".to_string());
        }
        if !payload["readings"].is_object() {
            errors.push("host_resource_denied: readings must be an object".to_string());
        }
    }
    errors
}

/// `resource_exhausted`: hard tier only — planned (audit first) / executed
/// rows carry the call_id set and readings (design §4.8 表 1).
pub fn verify_resource_exhausted(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "resource_exhausted")
    {
        let payload = &event["payload"];
        if payload["tier"].as_str() != Some("hard") {
            errors.push("resource_exhausted: only the hard tier may produce this row".to_string());
        }
        let phase = payload["phase"].as_str().unwrap_or_default();
        if !matches!(phase, "planned" | "executed") {
            errors.push("resource_exhausted: phase must be planned|executed".to_string());
        }
        let Some(call_ids) = payload["call_ids"].as_array() else {
            errors.push("resource_exhausted: call_ids required".to_string());
            continue;
        };
        if call_ids.is_empty() || call_ids.iter().any(|c| !is_non_empty_string(c)) {
            errors.push("resource_exhausted: call_ids must be non-empty id strings".to_string());
        }
        if !payload["readings"].is_object() {
            errors.push("resource_exhausted: readings must be an object".to_string());
        }
    }
    errors
}

/// `run_terminated`: the explicit terminal shape — `journal_degraded` rows
/// must carry the degraded summary (判据 6's chain leg).
pub fn verify_run_terminated(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "run_terminated")
    {
        let payload = &event["payload"];
        match payload["reason"].as_str() {
            Some("resource_exhausted") => {}
            Some("journal_degraded") => {
                if !payload["degraded"].as_object().is_some_and(|d| {
                    d.contains_key("entered_at") && d.contains_key("dropped_events")
                }) {
                    errors.push(
                        "run_terminated: reason journal_degraded requires the degraded summary"
                            .to_string(),
                    );
                }
            }
            _ => errors.push(
                "run_terminated: reason must be resource_exhausted|journal_degraded".to_string(),
            ),
        }
    }
    errors
}

/// `process_tree_reaped`: sweep audit — planned row before any kill
/// (hardening c); pids positive; executed rows carry the reaped set.
pub fn verify_process_tree_reaped(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "process_tree_reaped")
    {
        let payload = &event["payload"];
        let phase = payload["phase"].as_str().unwrap_or_default();
        if !matches!(phase, "planned" | "executed") {
            errors.push("process_tree_reaped: phase must be planned|executed".to_string());
        }
        let reason = payload["reason"].as_str().unwrap_or_default();
        if !matches!(reason, "parent_abort" | "run_shutdown") {
            errors
                .push("process_tree_reaped: reason must be parent_abort|run_shutdown".to_string());
        }
        let Some(pids) = payload["pids"].as_array() else {
            errors.push("process_tree_reaped: pids required".to_string());
            continue;
        };
        let pids_ok = pids
            .iter()
            .all(|p| p.as_u64().map(|n| n >= 1).unwrap_or(false));
        if pids.is_empty() || !pids_ok {
            errors.push("process_tree_reaped: pids must be positive integers".to_string());
        }
    }
    errors
}

/// `reclaim_performed`: reclaim audit — outcome tri-state (no `trash`:
/// the recycle bin is cancelled, §4.6.1); pending_delete rows owe the window;
/// rejected rows freed nothing; evidence/unknown classes can only be rejected.
pub fn verify_reclaim_performed(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "reclaim_performed")
    {
        let payload = &event["payload"];
        let class = payload["class"].as_str().unwrap_or_default();
        if !matches!(class, "cache" | "unknown" | "evidence") {
            errors.push("reclaim_performed: class must be cache|unknown|evidence".to_string());
        }
        let outcome = payload["outcome"].as_str().unwrap_or_default();
        if !matches!(outcome, "pending_delete" | "permanent" | "rejected") {
            errors.push(
                "reclaim_performed: outcome must be pending_delete|permanent|rejected".to_string(),
            );
            continue;
        }
        let tier = payload["tier"].as_str().unwrap_or_default();
        if !matches!(tier, "soft" | "reclaim_direct" | "hard" | "unknown") {
            errors.push(
                "reclaim_performed: tier must be a reclaim tier (soft|reclaim_direct|hard|unknown)"
                    .to_string(),
            );
        }
        let freed = payload["freed_bytes"].as_u64().unwrap_or(0);
        let paths = payload["paths"].as_array();
        match outcome {
            "pending_delete" => {
                let window = payload["window_rounds"].as_u64();
                if !matches!(window, Some(1..=3)) {
                    errors.push(
                        "reclaim_performed: pending_delete requires the window (1..=3)".to_string(),
                    );
                }
                if freed != 0 {
                    errors.push(
                        "reclaim_performed: pending_delete rows freed nothing yet".to_string(),
                    );
                }
                if paths.map(|p| p.is_empty()).unwrap_or(true) {
                    errors.push(
                        "reclaim_performed: pending_delete rows carry the queued paths".to_string(),
                    );
                }
            }
            "rejected" => {
                if freed != 0 {
                    errors.push("reclaim_performed: rejected rows freed nothing".to_string());
                }
            }
            "permanent" if paths.map(|p| p.is_empty()).unwrap_or(true) => {
                errors
                    .push("reclaim_performed: permanent rows carry the deleted paths".to_string());
            }
            _ => {}
        }
        if matches!(class, "evidence" | "unknown") && outcome != "rejected" {
            errors.push(
                "reclaim_performed: evidence/unknown classes must be rejected (fail-closed)"
                    .to_string(),
            );
        }
    }
    errors
}

/// `resource_limit_hit`: a kernel Job ceiling was hit — the failure is labeled
/// with the limit axis and the call (design §4.7 失败语义: never silent).
pub fn verify_resource_limit_hit(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for event in events
        .iter()
        .filter(|e| e["event_type"] == "resource_limit_hit")
    {
        let payload = &event["payload"];
        let limit = payload["limit"].as_str().unwrap_or_default();
        if !matches!(
            limit,
            "commit" | "active_process" | "cpu_rate" | "kill_on_job_close"
        ) {
            errors.push(
                "resource_limit_hit: limit must be commit|active_process|cpu_rate|kill_on_job_close"
                    .to_string(),
            );
        }
        if !is_non_empty_string(&payload["call_id"]) {
            errors.push("resource_limit_hit: call_id required".to_string());
        }
    }
    errors
}

/// All seven host-resource families (corpus + judges).
pub const HOST_RESOURCE_FAMILIES: &[&str] = &[
    "host_resource_snapshot",
    "host_resource_denied",
    "resource_exhausted",
    "run_terminated",
    "process_tree_reaped",
    "reclaim_performed",
    "resource_limit_hit",
];

pub fn verify_host_resource_families(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    errors.extend(verify_host_resource_snapshot(events));
    errors.extend(verify_host_resource_denied(events));
    errors.extend(verify_resource_exhausted(events));
    errors.extend(verify_run_terminated(events));
    errors.extend(verify_process_tree_reaped(events));
    errors.extend(verify_reclaim_performed(events));
    errors.extend(verify_resource_limit_hit(events));
    errors
}

/// Run one named rule family (S2b ∪ S2c) over a parsed journal; unknown names
/// yield an empty list (the caller validates the name).
pub fn verify_family(family: &str, events: &[Value]) -> Vec<String> {
    match family {
        "ledger_fold_advance" => verify_ledger_fold_advance(events),
        "ledger_fold_write_failed" => verify_ledger_fold_write_failed(events),
        "retrieval_mode" => verify_retrieval_mode(events),
        "retrieval_enable_gate" => verify_retrieval_enable_gate(events),
        "browser_launch_result" => verify_browser_launch_result(events),
        "policy_denial" => verify_policy_denial(events),
        "failure_target" => verify_failure_target(events),
        "failure_agg_coverage" => verify_failure_agg_coverage(events),
        "control_tickets" => verify_control_tickets(events),
        "lifecycle" => verify_lifecycle(events),
        // 0z S2 (2026-09-12): the host-resource fact families.
        "host_resource_snapshot" => verify_host_resource_snapshot(events),
        "host_resource_denied" => verify_host_resource_denied(events),
        "resource_exhausted" => verify_resource_exhausted(events),
        "run_terminated" => verify_run_terminated(events),
        "process_tree_reaped" => verify_process_tree_reaped(events),
        "reclaim_performed" => verify_reclaim_performed(events),
        "resource_limit_hit" => verify_resource_limit_hit(events),
        // 0ac S3-b (2026-09-13, F-007 裁决 (a)): immediate-feedback families.
        "retrieval_dedupe"
        | "result_delivered_accounting"
        | "retrieval_family_probe"
        | "failure_cause_shape"
        | "first_result_deadline" => {
            super::immediate_feedback::verify_immediate_feedback_family(family, events)
        }
        family => super::families_s2c::verify_s2c_family(family, events),
    }
}

/// Run every rule family (Python call order) over a parsed journal.
pub fn verify_all_families(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for family in ALL_FAMILIES {
        errors.extend(verify_family(family, events));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    fn ev(event_type: &str, payload: Value) -> Value {
        json!({
            "schema_version": "0.2.0-draft",
            "payload_schema": V02_PAYLOAD_SCHEMA,
            "event_type": event_type,
            "run_id": "run-1",
            "payload": payload,
        })
    }

    fn ev01(event_type: &str, payload: Value) -> Value {
        json!({
            "schema_version": "0.1.0-draft",
            "payload_schema": "run-event-v0.1.schema.json",
            "event_type": event_type,
            "run_id": "run-1",
            "payload": payload,
        })
    }

    fn hex64(seed: u8) -> String {
        let mut s = String::new();
        for i in 0..64 {
            s.push(char::from_digit(((seed + i) % 16) as u32, 16).unwrap());
        }
        s
    }

    fn denial(tool: &str, source: &str, status: &str, exit_code: i64) -> Value {
        ev(
            "tool_completed",
            json!({
                "tool": tool,
                "status": status,
                "exit_code": exit_code,
                "policy_denial": {"source": source, "code": "denied", "reason": "mechanical refusal"},
            }),
        )
    }

    fn assessment(
        id: &str,
        activation: &str,
        revision: i64,
        contract: &str,
        digest: &str,
    ) -> Value {
        ev(
            "information_sufficiency_assessment",
            json!({
                "assessment_id": id,
                "activation_id": activation,
                "contract_revision": revision,
                "contract_id": contract,
                "result_digest": digest,
            }),
        )
    }

    fn disposition(
        id: &str,
        activation: &str,
        assessment: &str,
        expected: i64,
        decision: &str,
        outcome: &str,
    ) -> Value {
        ev(
            "retrieval_parent_disposition",
            json!({
                "disposition_id": id,
                "activation_id": activation,
                "assessment_id": assessment,
                "expected_contract_revision": expected,
                "decision": decision,
                "outcome": outcome,
            }),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn close_record(
        id: &str,
        activation: &str,
        disposition_id: &str,
        assessment_id: &str,
        revision: i64,
        contract: &str,
        digest: &str,
    ) -> Value {
        ev(
            "retrieval_close_record",
            json!({
                "close_record_id": id,
                "activation_id": activation,
                "terminal_reason": "normal_close",
                "validated_disposition_id": disposition_id,
                "assessment_id": assessment_id,
                "contract_revision": revision,
                "contract_id": contract,
                "result_digest": digest,
            }),
        )
    }

    fn fold_advance(start: i64, cut: i64, rounds: i64, estimate: i64, after: i64) -> Value {
        ev(
            "ledger_fold_advance",
            json!({
                "fold_start": start, "fold_cut": cut, "rounds_folded": rounds,
                "view_estimate_tokens": estimate, "view_estimate_after": after,
                "agent_role": "main",
            }),
        )
    }

    fn restore(activation: &str, assessment: &str, revision: i64, status: &str) -> Value {
        ev(
            "retrieval_activation_restored",
            json!({
                "activation_id": activation,
                "assessment_id": assessment,
                "contract_revision": revision,
                "status": status,
            }),
        )
    }

    fn fold_ok() -> Value {
        fold_advance(0, 5, 1, 10, 5)
    }

    fn fold_write_failed(attempt: i64, disabled: bool) -> Value {
        ev(
            "ledger_fold_write_failed",
            json!({
                "ledger_path": "ledger.jsonl", "attempt": attempt, "disabled": disabled,
                "rows": 4, "view_estimate_tokens": 1000, "agent_role": "main",
            }),
        )
    }

    // ── S2c corpus helpers (Task D S2c, 2026-09-06) ─────────────────────

    use crate::journal::chain::sha256_hex;

    fn payload_digest(payload: &Value) -> String {
        sha256_hex(&canonical_json(payload).unwrap_or_default())
    }

    /// Build a payload-valid `retrieval_result_committed` whose mechanical
    /// facts (source_counts, degraded flag, digests, result_id) are computed
    /// from its own four segments — callers mutate the payload to introduce
    /// specific violations. `prefilter_log` sits outside the digest segments,
    /// so it can be added afterwards without invalidating the digests.
    fn finish_commit(activation: &str, revision: i64, ledger: Value, refs: Value) -> Value {
        let entries = ledger.as_array().cloned().unwrap_or_default();
        let count_vis = |vis: &str| {
            entries
                .iter()
                .filter(|e| e.get("visibility").and_then(Value::as_str) == Some(vis))
                .count()
        };
        let degraded =
            count_vis("full_text_observed") == 0 && count_vis("partial_text_observed") == 0;
        let mut counts = serde_json::Map::new();
        for vis in [
            "full_text_observed",
            "partial_text_observed",
            "metadata_only",
            "unavailable",
        ] {
            counts.insert(vis.to_string(), json!(count_vis(vis)));
        }
        counts.insert("total".to_string(), json!(entries.len()));
        let four = json!({
            "query_summary": "q",
            "source_ledger": ledger.clone(),
            "filtering_log": [],
            "raw_source_refs": refs.clone(),
        });
        let result_digest = payload_digest(&four);
        let ledger_digest = payload_digest(&ledger);
        ev(
            "retrieval_result_committed",
            json!({
                "activation_id": activation,
                "contract_revision": revision,
                "query_summary": "q",
                "source_ledger": ledger,
                "filtering_log": [],
                "raw_source_refs": refs,
                "source_counts": Value::Object(counts),
                "visibility_degraded": degraded,
                "result_digest": result_digest,
                "ledger_digest": ledger_digest,
                "result_id": format!("RET-RES-{}-1", &result_digest[..16]),
            }),
        )
    }

    fn committed(activation: &str, revision: i64, ledger: Value) -> Value {
        finish_commit(activation, revision, ledger, json!([]))
    }

    fn mutate_payload(event: Value, key: &str, value: Value) -> Value {
        let mut event = event;
        event["payload"][key] = value;
        event
    }

    /// The assessment bound to a committed result (matching digests/counts,
    /// `no_fulltext_evidence` exactly when the commit is degraded).
    fn bound_assessment(commit: &Value) -> Value {
        let p = &commit["payload"];
        let digest = p["result_digest"].as_str().unwrap_or_default().to_string();
        let codes = if p["visibility_degraded"].as_bool().unwrap_or(false) {
            vec!["no_fulltext_evidence"]
        } else {
            vec![]
        };
        ev(
            "information_sufficiency_assessment",
            json!({
                "assessment_id": format!("ASSESS-{}-1", &digest[..16]),
                "activation_id": p["activation_id"],
                "contract_revision": p["contract_revision"],
                "contract_id": "C1",
                "result_digest": p["result_digest"],
                "ledger_digest": p["ledger_digest"],
                "source_counts": p["source_counts"],
                "reason_codes": codes,
            }),
        )
    }

    fn ledger_entry(sid: &str, source_type: &str, visibility: &str) -> Value {
        json!({"source_id": sid, "source_type": source_type, "visibility": visibility})
    }

    fn pool_entry(sid: &str, candidates: Value, pool: Value) -> Value {
        json!({
            "source_id": sid, "source_type": "web_search_result",
            "visibility": "metadata_only",
            "candidate_urls": candidates, "candidate_pool": pool,
        })
    }

    fn pool_item(url: &str) -> Value {
        json!({
            "url": url, "canonical_url": url, "tier": "default",
            "mechanical_weight": 1.0, "weight_reason": "default tier",
            "relevance": "direct", "form_reasons": [],
        })
    }

    fn pool_refs(entries: &[Value]) -> Value {
        Value::Array(
            entries
                .iter()
                .filter_map(|entry| {
                    let urls = entry.get("candidate_urls")?.clone();
                    let pool = entry.get("candidate_pool")?.clone();
                    Some(json!({
                        "source_id": entry["source_id"],
                        "candidate_urls": urls,
                        "candidate_pool": pool,
                    }))
                })
                .collect(),
        )
    }

    fn tstart(tool: &str, call: &str) -> Value {
        ev("tool_started", json!({"tool": tool, "call_id": call}))
    }

    fn probe_event(complete: Vec<&str>, incomplete: Value) -> Value {
        ev(
            "tool_availability_check",
            json!({"complete": complete, "incomplete": incomplete}),
        )
    }

    fn work_tools_minus(name: &str) -> Vec<&'static str> {
        toolsets::WORK_TOOLS
            .iter()
            .copied()
            .filter(|t| *t != name)
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn header_event(
        reason: &str,
        header: &str,
        previous: Option<&str>,
        system: &str,
        tools_sha: &str,
        config: &str,
        tools: Vec<&str>,
        change_kind: Option<&str>,
    ) -> Value {
        let mut payload = json!({
            "agent_role": "main",
            "reason": reason,
            "header_sha256": header,
            "system_sha256": system,
            "tools_sha256": tools_sha,
            "config_sha256": config,
            "tools": tools,
            "tool_count": tools.len(),
        });
        if let Some(prev) = previous {
            payload["previous_header_sha256"] = json!(prev);
        }
        if let Some(kind) = change_kind {
            payload["change_kind"] = json!(kind);
        }
        ev("request_header_change", payload)
    }

    /// Shared scenario corpus: (name, events). Used by the table-driven unit
    /// test below and by the Python crosscheck (same corpus, both judges).
    fn scenarios() -> Vec<(&'static str, Vec<Value>)> {
        vec![
            (
                "host_resource_families_ok",
                vec![
                    ev(
                        "host_resource_snapshot",
                        json!({
                            "tier": "watch", "trigger": "tier_change",
                            "readings": {"volume_free_bytes": 9000000000i64}
                        }),
                    ),
                    ev(
                        "host_resource_denied",
                        json!({
                            "tool": "run_terminal_cmd", "call_id": "call-1",
                            "phase": "pre_issue", "action_class": "heavy", "tier": "hard",
                            "reason": "below the heavy floor",
                            "readings": {"volume_free_bytes": 1000000i64}
                        }),
                    ),
                    ev(
                        "resource_exhausted",
                        json!({
                            "phase": "planned", "tier": "hard",
                            "call_ids": ["call-1"], "readings": {"volume_free_bytes": 1000i64}
                        }),
                    ),
                    ev(
                        "resource_exhausted",
                        json!({
                            "phase": "executed", "tier": "hard",
                            "call_ids": ["call-1"], "readings": {"volume_free_bytes": 1000i64}
                        }),
                    ),
                    ev(
                        "run_terminated",
                        json!({
                            "reason": "resource_exhausted", "detail": "hard tier"
                        }),
                    ),
                    ev(
                        "process_tree_reaped",
                        json!({
                            "phase": "planned", "reason": "run_shutdown", "pids": [42],
                            "call_ids": ["call-1"]
                        }),
                    ),
                    ev(
                        "process_tree_reaped",
                        json!({
                            "phase": "executed", "reason": "parent_abort", "pids": [42]
                        }),
                    ),
                    ev(
                        "reclaim_performed",
                        json!({
                            "class": "cache", "outcome": "pending_delete", "tier": "soft",
                            "paths": ["D:/w/target/debug/incremental"], "freed_bytes": 0,
                            "window_rounds": 2
                        }),
                    ),
                    ev(
                        "reclaim_performed",
                        json!({
                            "class": "evidence", "outcome": "rejected", "tier": "soft",
                            "paths": ["D:/w/.gsa/runs/r"], "freed_bytes": 0
                        }),
                    ),
                    ev(
                        "resource_limit_hit",
                        json!({
                            "limit": "commit", "call_id": "call-1"
                        }),
                    ),
                ],
            ),
            (
                "host_resource_families_violations",
                vec![
                    // snapshot: tier not a machine key + bad trigger.
                    ev(
                        "host_resource_snapshot",
                        json!({
                            "tier": "full", "trigger": "every_round", "readings": {}
                        }),
                    ),
                    // denied: wrong phase + non-snake_case class.
                    ev(
                        "host_resource_denied",
                        json!({
                            "tool": "t", "call_id": "c", "phase": "post_issue",
                            "action_class": "Heavy", "tier": "hard", "reason": "r"
                        }),
                    ),
                    // exhausted: not the hard tier.
                    ev(
                        "resource_exhausted",
                        json!({
                            "phase": "planned", "tier": "soft", "call_ids": ["c"],
                            "readings": {}
                        }),
                    ),
                    // run_terminated: journal_degraded without the summary.
                    ev("run_terminated", json!({"reason": "journal_degraded"})),
                    // reaped: pid 0 + bad reason.
                    ev(
                        "process_tree_reaped",
                        json!({
                            "phase": "executed", "reason": "random", "pids": [0]
                        }),
                    ),
                    // reclaim: pending_delete without window + freed bytes;
                    // evidence class must be rejected.
                    ev(
                        "reclaim_performed",
                        json!({
                            "class": "cache", "outcome": "pending_delete", "tier": "soft",
                            "paths": ["D:/w/target"], "freed_bytes": 5
                        }),
                    ),
                    ev(
                        "reclaim_performed",
                        json!({
                            "class": "unknown", "outcome": "permanent", "tier": "hard",
                            "paths": ["D:/w/x"], "freed_bytes": 1
                        }),
                    ),
                    // limit hit: unknown axis.
                    ev(
                        "resource_limit_hit",
                        json!({"limit": "bandwidth", "call_id": "c"}),
                    ),
                ],
            ),
            (
                "control_tickets_ok",
                vec![
                    ev(
                        "control_ticket_issued",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                    ev(
                        "control_ticket_consumed",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                ],
            ),
            (
                "control_tickets_unknown",
                vec![ev(
                    "control_ticket_consumed",
                    json!({"ticket_id": "tX", "ticket_kind": "file_write_v1"}),
                )],
            ),
            (
                "control_tickets_kind_conflict",
                vec![
                    ev(
                        "control_ticket_issued",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                    ev(
                        "control_ticket_consumed",
                        json!({"ticket_id": "t1", "ticket_kind": "network_v1"}),
                    ),
                ],
            ),
            (
                "control_tickets_one_shot",
                vec![
                    ev(
                        "control_ticket_issued",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                    ev(
                        "control_ticket_consumed",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                    ev(
                        "control_ticket_rejected",
                        json!({"ticket_id": "t1", "ticket_kind": "file_write_v1"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_ok",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "web_search", "new_mode": "local_browser", "authority": "mechanical_probe", "capability_status": "available"}),
                    ),
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "local_browser", "new_mode": "off", "authority": "session_bootstrap"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_double_bootstrap",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "off", "new_mode": "web_search", "authority": "session_bootstrap"}),
                    ),
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "web_search", "new_mode": "off", "authority": "session_bootstrap"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_chain_break",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "off", "new_mode": "web_search", "authority": "session_bootstrap"}),
                    ),
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "local_browser", "new_mode": "off", "authority": "mechanical_probe"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_off_dispatch",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "web_search", "new_mode": "off", "authority": "session_bootstrap"}),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "browser_read", "target": "internal_retrieval"}),
                    ),
                    // The Python families index schema-required payload fields
                    // directly (they run post payload-validation) — synthetic
                    // assessments must therefore be schema-shaped.
                    ev(
                        "information_sufficiency_assessment",
                        json!({"assessment_id": "A9", "activation_id": "a1", "contract_revision": 1, "contract_id": "C", "result_digest": "D"}),
                    ),
                    committed("a1", 1, json!([])),
                ],
            ),
            (
                "retrieval_mode_lb_unavailable",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "off", "new_mode": "local_browser", "authority": "mechanical_probe", "capability_status": "degraded"}),
                    ),
                    committed("a1", 1, json!([])),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "target": "external_retrieval", "status": "ok"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_lb_missing_capability",
                vec![ev(
                    "retrieval_mode_transition",
                    json!({"old_mode": "off", "new_mode": "local_browser", "authority": "mechanical_probe"}),
                )],
            ),
            (
                "ledger_advance_ok",
                vec![
                    fold_advance(0, 10, 3, 2000, 1000),
                    fold_advance(0, 20, 4, 2000, 500),
                ],
            ),
            (
                "ledger_advance_after_not_below",
                vec![fold_advance(0, 10, 3, 1000, 1000)],
            ),
            (
                "ledger_advance_cut_not_increasing",
                vec![
                    fold_advance(0, 10, 3, 2000, 1000),
                    fold_advance(0, 10, 4, 2000, 500),
                ],
            ),
            (
                "ledger_advance_start_changed",
                vec![
                    fold_advance(0, 10, 3, 2000, 1000),
                    fold_advance(5, 20, 4, 2000, 500),
                ],
            ),
            (
                "ledger_advance_window_reset_ok",
                vec![
                    fold_advance(0, 10, 3, 2000, 1000),
                    ev("context_compressed", json!({"marker": "m"})),
                    fold_advance(7, 30, 2, 2000, 500),
                ],
            ),
            (
                "ledger_write_failed_ok",
                vec![
                    fold_write_failed(1, false),
                    fold_ok(),
                    fold_write_failed(1, false),
                ],
            ),
            (
                "ledger_write_failed_budget",
                vec![
                    fold_write_failed(1, false),
                    fold_write_failed(2, false),
                    fold_write_failed(3, true),
                ],
            ),
            (
                "ledger_write_failed_bad_disabled",
                vec![fold_write_failed(1, true)],
            ),
            (
                "ledger_write_failed_attempt_jump",
                vec![fold_write_failed(1, false), fold_write_failed(3, true)],
            ),
            (
                "ledger_write_failed_after_exhausted",
                vec![
                    fold_write_failed(1, false),
                    fold_write_failed(2, false),
                    fold_write_failed(3, true),
                    fold_write_failed(4, true),
                ],
            ),
            (
                "policy_denial_ok",
                vec![denial("search_replace", "acaf", "error", 1)],
            ),
            (
                "policy_denial_bad_status",
                vec![denial("search_replace", "acaf", "ok", 1)],
            ),
            (
                "policy_denial_zero_exit",
                vec![denial("search_replace", "acaf", "error", 0)],
            ),
            (
                "policy_denial_wrong_family",
                vec![denial("read_file", "acaf", "error", 1)],
            ),
            (
                "policy_denial_unknown_source",
                vec![denial("read_file", "bogus", "error", 1)],
            ),
            (
                "failure_target_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "cmd_target", "id": hex64(1), "cmd_preview": "make doom"},
                    }),
                )],
            ),
            (
                "failure_target_bad_id",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "cmd_target", "id": "abc", "cmd_preview": "make"},
                    }),
                )],
            ),
            (
                "failure_target_wrong_tool",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "anchor_target", "id": hex64(2), "path": "a.rs", "anchor_hash": hex64(3), "size": 10},
                    }),
                )],
            ),
            (
                "failure_target_preview_too_long",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "cmd_target", "id": hex64(1), "cmd_preview": "x".repeat(81)},
                    }),
                )],
            ),
            (
                "lifecycle_ok",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                    assessment("A2", "act-1", 2, "C1", "R1"),
                    disposition("P2", "act-1", "A2", 2, "close", "accepted"),
                    close_record("L1", "act-1", "P2", "A2", 2, "C1", "R1"),
                ],
            ),
            (
                "lifecycle_unknown_assessment",
                vec![disposition(
                    "P1",
                    "act-1",
                    "A-missing",
                    1,
                    "close",
                    "accepted",
                )],
            ),
            (
                "lifecycle_cas_mismatch",
                vec![
                    assessment("A1", "act-1", 2, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                ],
            ),
            (
                "lifecycle_revision_decrease",
                vec![
                    assessment("A1", "act-1", 2, "C1", "D1"),
                    assessment("A2", "act-1", 1, "C1", "D2"),
                ],
            ),
            (
                "lifecycle_stale_accepted",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                    assessment("A2", "act-1", 2, "C1", "D2"),
                    disposition("P2", "act-1", "A1", 1, "close", "accepted"),
                ],
            ),
            (
                "lifecycle_double_close",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                    close_record("L1", "act-1", "P1", "A1", 1, "C1", "D1"),
                    close_record("L2", "act-1", "P1", "A1", 1, "C1", "D1"),
                ],
            ),
            (
                "lifecycle_after_close",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                    close_record("L1", "act-1", "P1", "A1", 1, "C1", "D1"),
                    assessment("A2", "act-1", 2, "C1", "D2"),
                ],
            ),
            (
                "lifecycle_conflicting_replay",
                vec![
                    disposition("P1", "act-1", "A-missing", 1, "close", "accepted"),
                    disposition("P1", "act-1", "A-missing", 2, "close", "accepted"),
                ],
            ),
            (
                "lifecycle_replay_ok",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                ],
            ),
            (
                "retrieval_mode_off_target_only",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "web_search", "new_mode": "off", "authority": "session_bootstrap"}),
                    ),
                    // target-clause-only hit (no host-lane tool) — the clause
                    // the browser_read scenarios mask.
                    ev(
                        "tool_started",
                        json!({"tool": "web_search", "target": "external_retrieval"}),
                    ),
                ],
            ),
            (
                "retrieval_mode_lb_target_only",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "off", "new_mode": "local_browser", "authority": "mechanical_probe", "capability_status": "unsupported"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "web_fetch", "target": "external_retrieval", "status": "ok"}),
                    ),
                ],
            ),
            (
                "policy_denial_permission_ok",
                vec![denial("read_file", "permission", "error", 1)],
            ),
            (
                // Locks the exact-set permission mapping (S2b review P1):
                // web-family tools are NOT permission-gated in Python.
                "policy_denial_permission_web_fetch",
                vec![denial("web_fetch", "permission", "error", 1)],
            ),
            (
                "policy_denial_taint_ok",
                vec![denial("read_file", "taint", "error", 1)],
            ),
            (
                "policy_denial_retrieval_mode_ok",
                vec![denial("browser_read", "retrieval_mode", "error", 1)],
            ),
            (
                "failure_target_file_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "file_target", "id": hex64(4), "path": "a.rs"},
                    }),
                )],
            ),
            (
                "failure_target_file_wrong_tool",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "file_target", "id": hex64(4), "path": "a.rs"},
                    }),
                )],
            ),
            (
                "failure_target_url_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "web_fetch", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "url_target", "id": hex64(5), "canonical_url": "https://example.com/a"},
                    }),
                )],
            ),
            (
                "failure_target_url_wrong_tool",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "status": "error", "exit_code": 1,
                        "failure_target": {"kind": "url_target", "id": hex64(5), "canonical_url": "https://example.com/a"},
                    }),
                )],
            ),
            (
                "lifecycle_close_bad_decision",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                    close_record("L1", "act-1", "P1", "A1", 1, "C1", "D1"),
                ],
            ),
            (
                "lifecycle_close_bad_outcome",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "rejected_stale"),
                    close_record("L1", "act-1", "P1", "A1", 1, "C1", "D1"),
                ],
            ),
            (
                "lifecycle_close_revision_mismatch",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                    close_record("L1", "act-1", "P1", "A1", 2, "C1", "D1"),
                ],
            ),
            (
                "lifecycle_close_digest_mismatch",
                vec![
                    assessment("A1", "act-1", 1, "C1", "R1"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                    close_record("L1", "act-1", "P1", "A1", 1, "C1", "RX"),
                ],
            ),
            (
                // rejected_stale / rejected_conflicting outcomes pass through
                // the disposition state machine without advancing it.
                "lifecycle_passthrough_outcomes",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P2", "act-1", "A1", 1, "continue", "rejected_stale"),
                    disposition("P3", "act-1", "A1", 1, "close", "rejected_conflicting"),
                ],
            ),
            (
                // A first-occurrence disposition with outcome
                // replayed_idempotent is an unknown outcome on BOTH judges
                // (the close pass accepts that outcome value, but the
                // disposition pass rejects it first) — verdict parity pinned.
                "lifecycle_close_replayed_outcome_errors",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "close", "replayed_idempotent"),
                ],
            ),
            (
                "lifecycle_restore_not_preceding",
                vec![
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                    restore("act-1", "A1", 1, "awaiting_disposition"),
                ],
            ),
            (
                "lifecycle_restore_activation_mismatch",
                vec![
                    restore("act-2", "A1", 1, "awaiting_disposition"),
                    disposition("P1", "act-1", "A1", 1, "close", "accepted"),
                ],
            ),
            (
                "lifecycle_continue_plus_one_wrong",
                vec![
                    assessment("A1", "act-1", 1, "C1", "D1"),
                    disposition("P1", "act-1", "A1", 1, "continue", "accepted"),
                    assessment("A2", "act-1", 3, "C1", "D2"),
                ],
            ),
            (
                "ledger_advance_start_ge_cut",
                vec![fold_advance(10, 5, 1, 100, 50)],
            ),
            (
                "ledger_advance_rounds_shrink",
                vec![
                    fold_advance(0, 10, 5, 2000, 1000),
                    fold_advance(0, 20, 4, 2000, 500),
                ],
            ),
            (
                "v01_noop",
                vec![
                    ev01(
                        "control_ticket_consumed",
                        json!({"ticket_id": "tX", "ticket_kind": "file_write_v1"}),
                    ),
                    ev01(
                        "retrieval_parent_disposition",
                        json!({"disposition_id": "P", "activation_id": "a", "assessment_id": "A-missing", "expected_contract_revision": 1, "decision": "close", "outcome": "accepted"}),
                    ),
                ],
            ),
            // ── S2c scenarios (Task D S2c, 2026-09-06) ──────────────────
            (
                "inquiry_kind_ok",
                vec![ev(
                    "orientation_checkpoint",
                    json!({"inquiry_kind": "orientation_checkpoint", "prompt": "continue?"}),
                )],
            ),
            (
                "inquiry_kind_mismatch",
                vec![ev(
                    "orientation_checkpoint",
                    json!({"inquiry_kind": "orientation", "prompt": "continue?"}),
                )],
            ),
            // P0-0x S2 (2026-09-11, ADR-0010 §14.66): initial-round inquiry
            // scenarios — one legal shape + four rule violations (wrong
            // block, wrong injection position, initial block under another
            // trigger, and the one-shot contract).
            (
                "initial_round_ok",
                vec![ev(
                    "orientation_checkpoint",
                    json!({
                        "inquiry_kind": "orientation_checkpoint",
                        "agent_role": "main",
                        "session_id": "sess-0x",
                        "trigger": "initial_round",
                        "message_block": "[INITIAL_ROUND_INQUIRY v0.1]\n开局问询\n[/INITIAL_ROUND_INQUIRY]",
                        "injection_position": "post_tool_batch_gap",
                    }),
                )],
            ),
            (
                "initial_round_wrong_block",
                vec![ev(
                    "orientation_checkpoint",
                    json!({
                        "inquiry_kind": "orientation_checkpoint",
                        "agent_role": "main",
                        "session_id": "sess-0x",
                        "trigger": "initial_round",
                        "message_block": "[ORIENTATION v0.4]\n方向检查\n[/ORIENTATION]",
                        "injection_position": "post_tool_batch_gap",
                    }),
                )],
            ),
            (
                "initial_round_wrong_position",
                vec![ev(
                    "orientation_checkpoint",
                    json!({
                        "inquiry_kind": "orientation_checkpoint",
                        "agent_role": "main",
                        "session_id": "sess-0x",
                        "trigger": "initial_round",
                        "message_block": "[INITIAL_ROUND_INQUIRY v0.1]\n开局问询\n[/INITIAL_ROUND_INQUIRY]",
                        "injection_position": "loop_top_gap",
                    }),
                )],
            ),
            (
                "initial_round_under_periodic_trigger",
                vec![ev(
                    "orientation_checkpoint",
                    json!({
                        "inquiry_kind": "orientation_checkpoint",
                        "agent_role": "main",
                        "session_id": "sess-0x",
                        "trigger": "completed_turns_interval",
                        "message_block": "[INITIAL_ROUND_INQUIRY v0.1]\n开局问询\n[/INITIAL_ROUND_INQUIRY]",
                        "injection_position": "post_tool_batch_gap",
                    }),
                )],
            ),
            (
                "initial_round_twice",
                vec![
                    ev(
                        "orientation_checkpoint",
                        json!({
                            "inquiry_kind": "orientation_checkpoint",
                            "agent_role": "main",
                            "session_id": "sess-0x",
                            "trigger": "initial_round",
                            "message_block": "[INITIAL_ROUND_INQUIRY v0.1]\n开局问询\n[/INITIAL_ROUND_INQUIRY]",
                            "injection_position": "post_tool_batch_gap",
                        }),
                    ),
                    ev(
                        "orientation_checkpoint",
                        json!({
                            "inquiry_kind": "orientation_checkpoint",
                            "agent_role": "main",
                            "session_id": "sess-0x",
                            "trigger": "initial_round",
                            "message_block": "[INITIAL_ROUND_INQUIRY v0.1]\n开局问询\n[/INITIAL_ROUND_INQUIRY]",
                            "injection_position": "post_tool_batch_gap",
                        }),
                    ),
                ],
            ),
            (
                "plan_write_ok",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "accepted", "validation": {"valid": true}}),
                )],
            ),
            (
                "plan_write_refill_attempt_2",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 2, "outcome": "refill_requested", "validation": {"valid": false}}),
                )],
            ),
            (
                "plan_write_refill_unfollowed",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "refill_requested", "validation": {"valid": false}}),
                )],
            ),
            (
                "plan_write_refill_valid_true",
                vec![
                    ev(
                        "plan_write",
                        json!({"attempt": 1, "outcome": "refill_requested", "validation": {"valid": true}}),
                    ),
                    ev(
                        "plan_write",
                        json!({"attempt": 2, "outcome": "accepted", "validation": {"valid": true}}),
                    ),
                ],
            ),
            (
                "plan_write_after_refill_attempt_1",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "degraded", "degrade_reason": "validation_failed_after_refill", "validation": {"valid": false}}),
                )],
            ),
            (
                "plan_write_rotate_degrade_valid_false",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "degraded", "degrade_reason": "plan_rotate_failed", "validation": {"valid": false}}),
                )],
            ),
            (
                "plan_write_accepted_attempt_3",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 3, "outcome": "accepted", "validation": {"valid": true}}),
                )],
            ),
            (
                "console_mode_ok",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "console_mode": "direct", "transition_id": "t1"}),
                    ),
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "direct", "to": "console",
                            "trigger": "model_return", "model_decision": "return_to_console",
                            "transition_id": "t2", "related_transition_id": "t1",
                        }),
                    ),
                ],
            ),
            (
                "console_mode_stay_ok",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "console",
                        "trigger": "assistant_failure_streak", "model_decision": "stay",
                        "transition_id": "t1", "related_transition_id": null,
                    }),
                )],
            ),
            (
                "console_mode_c2d_bad_trigger",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "direct",
                        "trigger": "model_return", "model_decision": "switch",
                        "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                        "related_transition_id": null,
                    }),
                )],
            ),
            (
                "console_mode_d2c_bad_related",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "direct", "to": "console",
                            "trigger": "model_return", "model_decision": "return_to_console",
                            "transition_id": "t2", "related_transition_id": "tX",
                        }),
                    ),
                ],
            ),
            (
                "console_mode_double_streak",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "console",
                            "trigger": "assistant_failure_streak", "model_decision": "stay",
                            "transition_id": "t2", "related_transition_id": null,
                        }),
                    ),
                ],
            ),
            (
                "console_mode_direct_tool_unstamped",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "console_mode": "direct"}),
                    ),
                ],
            ),
            (
                "console_mode_direct_tool_wrong_tid",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "console_mode": "direct", "transition_id": "t9"}),
                    ),
                ],
            ),
            // console_order_written retired 2026-09-06 (S2d 裁决一, ADR-0010
            // §14.57): these three scenarios are the historical-replay-legal
            // guards — written chains (backed / duplicate / unbacked) must
            // produce ZERO errors from every family (no negative check).
            (
                "console_order_written_ok",
                vec![
                    ev(
                        "tool_completed",
                        json!({"tool": "blackboard_action_write", "call_id": "w1", "status": "success", "exit_code": 0}),
                    ),
                    ev(
                        "console_order_written",
                        json!({
                            "order_id": "ORD-1", "action": "workspace.run_terminal",
                            "step_id": null, "write_call_id": "w1",
                            "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                ],
            ),
            (
                "console_order_written_duplicate",
                vec![
                    ev(
                        "tool_completed",
                        json!({"tool": "blackboard_action_write", "call_id": "w1", "status": "success", "exit_code": 0}),
                    ),
                    ev(
                        "console_order_written",
                        json!({
                            "order_id": "ORD-1", "action": "workspace.run_terminal",
                            "write_call_id": "w1", "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                    ev(
                        "console_order_written",
                        json!({
                            "order_id": "ORD-1", "action": "workspace.run_terminal",
                            "write_call_id": "w1", "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                ],
            ),
            (
                "console_order_written_unbacked",
                vec![ev(
                    "console_order_written",
                    json!({
                        "order_id": "ORD-1", "action": "workspace.run_terminal",
                        "write_call_id": "wX", "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            // console_order_rejected narrowed to shape invariants (S2d 裁决一,
            // ADR-0010 §14.57): no prior-written requirement, no stamp
            // consistency — the ok scenario carries NO written order and the
            // stamp-drift scenario proves the retired friction stays retired.
            (
                "console_order_rejected_ok",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-1", "phase": "issue", "step": "policy",
                        "code": "policy_denied", "reason": "mechanical refusal",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_stamp_drift_legal",
                vec![
                    ev(
                        "console_order_written",
                        json!({
                            "order_id": "ORD-4", "action": "workspace.run_terminal",
                            "write_call_id": "w1", "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                    ev(
                        "console_order_rejected",
                        json!({
                            "order_id": "ORD-4", "phase": "issue", "step": "policy",
                            "code": "policy_denied", "reason": "mechanical refusal",
                            "round": 4, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                ],
            ),
            (
                "console_order_rejected_duplicate",
                vec![
                    ev(
                        "console_order_rejected",
                        json!({
                            "order_id": "ORD-1", "phase": "issue", "step": "policy",
                            "code": "policy_denied", "reason": "mechanical refusal",
                            "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                    ev(
                        "console_order_rejected",
                        json!({
                            "order_id": "ORD-1", "phase": "issue", "step": "policy",
                            "code": "policy_denied", "reason": "mechanical refusal",
                            "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                ],
            ),
            (
                "console_order_rejected_empty_reason",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-1", "phase": "issue", "step": "policy",
                        "code": "policy_denied", "reason": "",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_reason_null",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-1", "phase": "issue", "step": "policy",
                        "code": "policy_denied", "reason": null,
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_pre_issue_bad_step",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-1", "phase": "pre_issue", "step": "registry",
                        "code": "step_not_done", "reason": "mechanical refusal",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_cross_run_duplicate_legal",
                // Same order_id in two DIFFERENT runs is a fresh rejection
                // budget each — the per-run isolation must not be tripped by
                // the shared id (envelope run_id differs, built manually
                // because `ev` pins run-1).
                vec![
                    ev(
                        "console_order_rejected",
                        json!({
                            "order_id": "ORD-1", "phase": "issue", "step": "policy",
                            "code": "policy_denied", "reason": "mechanical refusal",
                            "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                        }),
                    ),
                    json!({
                        "schema_version": "0.2.0-draft",
                        "payload_schema": V02_PAYLOAD_SCHEMA,
                        "event_type": "console_order_rejected",
                        "run_id": "run-2",
                        "payload": {
                            "order_id": "ORD-1", "phase": "issue", "step": "policy",
                            "code": "policy_denied", "reason": "mechanical refusal",
                            "round": 1, "plan_epoch": "e2", "run_id": "run-2",
                        },
                    }),
                ],
            ),
            (
                "console_order_rejected_issue_bad_step",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-1", "phase": "issue", "step": "execute",
                        "code": "policy_denied", "reason": "mechanical refusal",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_bad_phase",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-2", "phase": "execute", "step": "execute",
                        "code": "boom", "reason": "mechanical refusal",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "console_order_rejected_pre_issue_bad_code",
                vec![ev(
                    "console_order_rejected",
                    json!({
                        "order_id": "ORD-3", "phase": "pre_issue", "step": "protocol",
                        "code": "boom", "reason": "mechanical refusal",
                        "round": 3, "plan_epoch": "e1", "run_id": "run-1",
                    }),
                )],
            ),
            (
                "tool_running_ok",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "note": "still building"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                ],
            ),
            (
                "tool_running_duplicate",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                ],
            ),
            (
                "tool_running_no_start",
                vec![ev(
                    "tool_running",
                    json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                )],
            ),
            (
                "tool_running_completion_not_running",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "exit_code": 0}),
                    ),
                ],
            ),
            (
                "tool_running_idle_kill_ok",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "idle_killed", "reason": "no output activity for 300s"}),
                    ),
                ],
            ),
            (
                "tool_running_idle_kill_premature",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "idle_killed", "reason": "no output activity for 300s"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                ],
            ),
            (
                "output_truncation_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "success", "exit_code": 0,
                        "output_truncated": true, "total_bytes": 90000,
                        "output_object_id": "OBJ-1",
                    }),
                )],
            ),
            (
                "output_truncation_missing_bytes",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "success", "exit_code": 0,
                        "output_truncated": true,
                    }),
                )],
            ),
            (
                "output_truncation_object_unmarked",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "run_terminal_cmd", "status": "success", "exit_code": 0,
                        "output_object_id": "OBJ-1",
                    }),
                )],
            ),
            (
                "budget_cue_ok",
                vec![
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 599, "threshold_seconds": 600}),
                    ),
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 299, "threshold_seconds": 300}),
                    ),
                ],
            ),
            (
                "budget_cue_cap_exceeded",
                vec![
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 5, "threshold_seconds": 120}),
                    ),
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 4, "threshold_seconds": 120}),
                    ),
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 3, "threshold_seconds": 120}),
                    ),
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 2, "threshold_seconds": 120}),
                    ),
                    ev(
                        "budget_cue_injected",
                        json!({"remaining_seconds": 1, "threshold_seconds": 120}),
                    ),
                ],
            ),
            (
                "budget_cue_not_below_threshold",
                vec![ev(
                    "budget_cue_injected",
                    json!({"remaining_seconds": 600, "threshold_seconds": 600}),
                )],
            ),
            (
                "result_consistency_ok",
                vec![committed(
                    "act-1",
                    1,
                    json!([
                        ledger_entry("s1", "doc_page", "full_text_observed"),
                        ledger_entry("s2", "doc_page", "metadata_only"),
                    ]),
                )],
            ),
            ("result_consistency_with_binding_ok", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([
                        ledger_entry("s1", "doc_page", "full_text_observed"),
                        ledger_entry("s2", "doc_page", "metadata_only"),
                    ]),
                );
                let assess = bound_assessment(&commit);
                vec![commit, assess]
            }),
            (
                "result_consistency_counts_mismatch",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                    ),
                    "source_counts",
                    json!({"full_text_observed": 0, "partial_text_observed": 0, "metadata_only": 0, "unavailable": 0, "total": 1}),
                )],
            ),
            (
                "result_consistency_degraded_mismatch",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                    ),
                    "visibility_degraded",
                    json!(true),
                )],
            ),
            (
                "result_consistency_bad_digest",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                    ),
                    "result_digest",
                    json!(hex64(9)),
                )],
            ),
            (
                "result_consistency_retired_organized",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                    ),
                    "organized_response",
                    json!({"blocks": []}),
                )],
            ),
            ("result_consistency_assessment_drift", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                );
                let assess = mutate_payload(
                    bound_assessment(&commit),
                    "source_counts",
                    json!({"full_text_observed": 9, "partial_text_observed": 0, "metadata_only": 0, "unavailable": 0, "total": 9}),
                );
                vec![commit, assess]
            }),
            (
                "reason_codes_ok",
                vec![ev(
                    "information_sufficiency_assessment",
                    json!({
                        "assessment_id": "ASSESS-1", "activation_id": "act-1",
                        "contract_revision": 1, "contract_id": "C1",
                        "reason_codes": ["no_mechanical_coverage_requirement", "no_fulltext_evidence"],
                    }),
                )],
            ),
            (
                "reason_codes_retired",
                vec![ev(
                    "information_sufficiency_assessment",
                    json!({
                        "assessment_id": "ASSESS-1", "activation_id": "act-1",
                        "contract_revision": 1, "contract_id": "C1",
                        "reason_codes": ["structured_result_validation_failed"],
                    }),
                )],
            ),
            (
                "reason_codes_unknown",
                vec![ev(
                    "information_sufficiency_assessment",
                    json!({
                        "assessment_id": "ASSESS-1", "activation_id": "act-1",
                        "contract_revision": 1, "contract_id": "C1",
                        "reason_codes": ["vibes_based"],
                    }),
                )],
            ),
            (
                "source_weighting_ok",
                vec![committed(
                    "act-1",
                    1,
                    json!([json!({
                        "source_id": "s1", "source_type": "web_page",
                        "visibility": "metadata_only", "tier": "authoritative",
                        "mechanical_weight": 1.1, "weight_reason": "seed whitelist",
                    })]),
                )],
            ),
            (
                "source_weighting_web_page_no_tier",
                vec![committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "web_page", "metadata_only")]),
                )],
            ),
            (
                "source_weighting_partial_fields",
                vec![committed(
                    "act-1",
                    1,
                    json!([json!({
                        "source_id": "s1", "source_type": "doc_page",
                        "visibility": "metadata_only", "tier": "default",
                    })]),
                )],
            ),
            (
                "source_weighting_wrong_weight",
                vec![committed(
                    "act-1",
                    1,
                    json!([json!({
                        "source_id": "s1", "source_type": "doc_page",
                        "visibility": "metadata_only", "tier": "low_quality",
                        "mechanical_weight": 1.0, "weight_reason": "wrong table entry",
                    })]),
                )],
            ),
            (
                "source_weighting_retired_annotation",
                vec![committed(
                    "act-1",
                    1,
                    json!([json!({
                        "source_id": "s1", "source_type": "doc_page",
                        "visibility": "metadata_only", "model_weight": 0.5,
                    })]),
                )],
            ),
            (
                "candidate_pool_ok",
                vec![{
                    let entries = vec![pool_entry(
                        "s1",
                        json!(["u1", "u2"]),
                        json!([pool_item("u1"), pool_item("u2")]),
                    )];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u3", "reason": "bad_url", "action": "removed", "canonical_url": null},
                    ]);
                    commit
                }],
            ),
            (
                "candidate_pool_non_web_search",
                vec![{
                    let entry = json!({
                        "source_id": "s1", "source_type": "doc_page",
                        "visibility": "metadata_only",
                        "candidate_urls": ["u1"], "candidate_pool": [pool_item("u1")],
                    });
                    let entries = vec![entry];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "candidate_pool_empty_no_removal",
                vec![{
                    let entries = vec![pool_entry("s1", json!([]), json!([]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "candidate_pool_url_mismatch",
                vec![{
                    let entries = vec![pool_entry(
                        "s1",
                        json!(["u1"]),
                        json!([{
                            "url": "uX", "canonical_url": "uX", "tier": "default",
                            "mechanical_weight": 1.0, "weight_reason": "default tier",
                            "relevance": "direct", "form_reasons": [],
                        }]),
                    )];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "candidate_pool_refs_not_mirrored",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit = finish_commit("act-1", 1, json!(entries), json!([]));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "candidate_prefilter_ok",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "removed", "canonical_url": null},
                    ]);
                    commit
                }],
            ),
            (
                "candidate_prefilter_missing_log",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    finish_commit("act-1", 1, json!(entries), pool_refs(&entries))
                }],
            ),
            (
                "candidate_prefilter_bad_reason",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "vibes", "action": "removed"},
                    ]);
                    commit
                }],
            ),
            (
                "candidate_prefilter_removed_retained",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u1", "reason": "bad_url", "action": "removed"},
                    ]);
                    commit
                }],
            ),
            (
                "candidate_prefilter_duplicate_entry",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "removed"},
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "removed"},
                    ]);
                    commit
                }],
            ),
            (
                "candidate_count_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "browser_read", "call_id": "c1", "status": "success",
                        "candidate_count": 3, "candidate_cap": 8,
                    }),
                )],
            ),
            (
                "candidate_count_fields_alone",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "web_fetch", "call_id": "c1", "status": "success",
                        "candidate_count": 3,
                    }),
                )],
            ),
            (
                "candidate_count_wrong_family",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "c1", "status": "success",
                        "candidate_count": 3, "candidate_cap": 8,
                    }),
                )],
            ),
            (
                "candidate_count_missing_on_lane",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "browser_read", "call_id": "c1", "status": "success",
                    }),
                )],
            ),
            (
                "candidate_count_cap_mismatch",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "web_fetch", "call_id": "c1", "status": "error", "exit_code": 1,
                        "error": "web_fetch_candidate_cap_exceeded",
                        "candidate_count": 5, "candidate_cap": 8,
                    }),
                )],
            ),
            (
                "candidate_count_wrapper_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "web_fetch", "call_id": "c1", "status": "success",
                        "target": "external_retrieval",
                    }),
                )],
            ),
            (
                "inject_budget_ok",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "c1", "status": "error", "exit_code": 1,
                        "error": "round_inject_budget_exceeded",
                        "inject_tokens_used": 60000, "inject_tokens_budget": 50000,
                    }),
                )],
            ),
            (
                "inject_budget_missing_fields",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "c1", "status": "error", "exit_code": 1,
                        "error": "round_inject_budget_exceeded",
                    }),
                )],
            ),
            (
                "inject_budget_fields_other_code",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "c1", "status": "success", "error": "boom",
                        "inject_tokens_used": 5, "inject_tokens_budget": 10,
                    }),
                )],
            ),
            (
                "inject_budget_zero_budget",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "c1", "status": "error", "exit_code": 1,
                        "error": "round_inject_budget_exceeded",
                        "inject_tokens_used": 5, "inject_tokens_budget": 0,
                    }),
                )],
            ),
            (
                "receipt_gate_without_start_ok",
                vec![denial("search_replace", "acaf", "error", 1)],
            ),
            (
                "receipt_execution_pair_ok",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "error", "exit_code": 1, "error": "timeout"}),
                    ),
                ],
            ),
            (
                "receipt_completed_before_start",
                vec![
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "error", "exit_code": 1, "error": "timeout"}),
                    ),
                    tstart("run_terminal_cmd", "c1"),
                ],
            ),
            (
                "receipt_non_gate_no_start",
                vec![ev(
                    "tool_completed",
                    json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "error", "exit_code": 1, "error": "timeout"}),
                )],
            ),
            (
                "receipt_duplicate_completed",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "error", "exit_code": 1, "error": "timeout"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "error", "exit_code": 1, "error": "timeout"}),
                    ),
                ],
            ),
            (
                "receipt_open_start_at_terminal",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev("run_finished", json!({})),
                ],
            ),
            (
                "receipt_open_start_invalidated_ok",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev("run_invalidated", json!({})),
                ],
            ),
            (
                "dep_graph_ok",
                vec![
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "read", "path": "a.rs", "anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}},
                        }),
                    ),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "write", "path": "a.rs", "consumed_read": "r1",
                                "consumed_anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}, "new_anchor": null},
                        }),
                    ),
                ],
            ),
            (
                "dep_graph_bad_kind",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "mutate", "path": "a.rs", "anchor": null},
                    }),
                )],
            ),
            (
                "dep_graph_wrong_tool",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "grep", "call_id": "r1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "read", "path": "a.rs", "anchor": null},
                    }),
                )],
            ),
            (
                "dep_graph_non_success",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 1,
                        "dep_graph": {"kind": "read", "path": "a.rs", "anchor": null},
                    }),
                )],
            ),
            (
                "dep_graph_dangling_consumed",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "write", "path": "a.rs", "consumed_read": "rX",
                            "consumed_anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}, "new_anchor": null},
                    }),
                )],
            ),
            (
                "dep_graph_anchor_mismatch",
                vec![
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "read", "path": "a.rs", "anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}},
                        }),
                    ),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "write", "path": "a.rs", "consumed_read": "r1",
                                "consumed_anchor": {"sha256": hex64(2), "size": 10, "mtime": 1}, "new_anchor": null},
                        }),
                    ),
                ],
            ),
            (
                "mechanical_audit_ok",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": {"key": "file:a.rs", "round": 3, "summary": "read a.rs", "anomaly": null},
                    }),
                )],
            ),
            (
                "mechanical_audit_bad_kind",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "vibes",
                        "payload": {"key": "file:a.rs", "round": 3, "summary": "read a.rs", "anomaly": null},
                    }),
                )],
            ),
            (
                "mechanical_audit_negative_round",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": {"key": "file:a.rs", "round": -1, "summary": "read a.rs", "anomaly": null},
                    }),
                )],
            ),
            (
                "mechanical_audit_anomaly_type",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": {"key": "file:a.rs", "round": 3, "summary": "read a.rs", "anomaly": 5},
                    }),
                )],
            ),
            (
                "recovery_truncation_ok",
                vec![
                    ev("context_recovery_truncated", json!({"rounds_dropped": 5})),
                    ev("model_request", json!({})),
                ],
            ),
            (
                "recovery_truncation_after_model_request",
                vec![
                    ev("model_request", json!({})),
                    ev("context_recovery_truncated", json!({"rounds_dropped": 5})),
                ],
            ),
            (
                "recovery_truncation_zero_rounds",
                vec![
                    ev("context_recovery_truncated", json!({"rounds_dropped": 0})),
                    ev("model_request", json!({})),
                ],
            ),
            (
                "context_compressed_ok",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "mechanical", "reason": "rhythm", "summary_incomplete": false,
                        "summary_id": "S1", "summary_digest": "d1", "summary_path": "p1",
                    }),
                )],
            ),
            (
                "context_compressed_bad_reason",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "mechanical", "reason": "bored", "summary_incomplete": false,
                        "summary_id": "S1", "summary_digest": "d1", "summary_path": "p1",
                    }),
                )],
            ),
            (
                "context_compressed_mechanical_incomplete",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "mechanical", "reason": "fallback", "summary_incomplete": true,
                        "summary_id": null, "summary_digest": null, "summary_path": null,
                    }),
                )],
            ),
            (
                "context_compressed_incomplete_with_archive",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "template_summary", "reason": "rhythm", "summary_incomplete": true,
                        "summary_id": "S1", "summary_digest": "d1", "summary_path": "p1",
                    }),
                )],
            ),
            (
                "context_compressed_write_failed_on_incomplete",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "template_summary", "reason": "fallback", "summary_incomplete": true,
                        "summary_id": null, "summary_digest": null, "summary_path": null,
                        "archive_write_failed": true,
                    }),
                )],
            ),
            (
                "activation_restore_ok",
                vec![ev(
                    "retrieval_activation_restored",
                    json!({"activation_id": "act-9"}),
                )],
            ),
            (
                "activation_restore_duplicate",
                vec![
                    ev(
                        "retrieval_activation_restored",
                        json!({"activation_id": "act-9"}),
                    ),
                    ev(
                        "retrieval_activation_restored",
                        json!({"activation_id": "act-9"}),
                    ),
                ],
            ),
            (
                "probe_partition_ok",
                vec![probe_event(
                    work_tools_minus("grep"),
                    json!([{"tool": "grep", "reason": "not registered in this session"}]),
                )],
            ),
            (
                "probe_overlap",
                vec![probe_event(
                    toolsets::WORK_TOOLS.to_vec(),
                    json!([{"tool": "read_file", "reason": "not registered in this session"}]),
                )],
            ),
            (
                "probe_judgment_word",
                vec![probe_event(
                    work_tools_minus("grep"),
                    json!([{"tool": "grep", "reason": "unavailable in this session"}]),
                )],
            ),
            (
                // Partition clause narrowed 2026-09-06 (ADR-0010 §14.59): a
                // missing work tool is now legal (declared-surface
                // accounting, §14.58) — the violation is a NON-work tool in
                // the partition.
                "probe_partition_extra",
                vec![probe_event(
                    work_tools_minus("grep"),
                    json!([{"tool": "ghost_tool", "reason": "not registered in this session"}]),
                )],
            ),
            (
                // Narrowed declared-surface partition (subset of the work
                // tools, sealed/absent tools legitimately out) replays
                // clean — the 裁决二补裁决 positive example.
                "probe_partition_narrowed_legal",
                vec![probe_event(work_tools_minus("grep"), json!([]))],
            ),
            (
                "request_header_ok",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        Some("tools"),
                    ),
                ],
            ),
            (
                "request_header_initial_with_previous",
                vec![header_event(
                    "initial",
                    "H1",
                    Some("H0"),
                    "S1",
                    "T1",
                    "C1",
                    vec!["read_file"],
                    None,
                )],
            ),
            (
                "request_header_change_prev_mismatch",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("HX"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        Some("tools"),
                    ),
                ],
            ),
            (
                "request_header_change_kind_wrong",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        Some("config"),
                    ),
                ],
            ),
            (
                "request_header_tools_dup",
                vec![header_event(
                    "initial",
                    "H1",
                    None,
                    "S1",
                    "T1",
                    "C1",
                    vec!["read_file", "read_file"],
                    None,
                )],
            ),
            (
                "probe_accuracy_ok",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    probe_event(toolsets::WORK_TOOLS.to_vec(), json!([])),
                    probe_event(
                        work_tools_minus("grep"),
                        json!([{"tool": "grep", "reason": "not registered in this session"}]),
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        Some("tools"),
                    ),
                    ev("model_output", json!({})),
                ],
            ),
            (
                "probe_accuracy_unreported_flip",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    probe_event(toolsets::WORK_TOOLS.to_vec(), json!([])),
                    probe_event(
                        work_tools_minus("grep"),
                        json!([{"tool": "grep", "reason": "not registered in this session"}]),
                    ),
                    ev("model_output", json!({})),
                ],
            ),
            (
                "probe_accuracy_no_header_events_ok",
                vec![
                    probe_event(toolsets::WORK_TOOLS.to_vec(), json!([])),
                    probe_event(
                        work_tools_minus("grep"),
                        json!([{"tool": "grep", "reason": "not registered in this session"}]),
                    ),
                    ev("model_output", json!({})),
                ],
            ),
            (
                "v01_tool_budget_fields",
                vec![
                    ev01(
                        "tool_completed",
                        json!({"tool": "web_fetch", "status": "success", "candidate_count": 3}),
                    ),
                    ev01(
                        "tool_completed",
                        json!({"tool": "read_file", "status": "success", "error": "boom", "inject_tokens_used": 5, "inject_tokens_budget": 10}),
                    ),
                ],
            ),
            // ── S2c review-handling coverage batch (2026-09-06) ─────────
            (
                "rc_unknown_visibility",
                vec![committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "vibes")]),
                )],
            ),
            (
                "rc_claim_mismatch",
                vec![committed(
                    "act-1",
                    1,
                    json!([json!({
                        "source_id": "s1", "source_type": "doc_page",
                        "visibility": "metadata_only", "highest_allowed_claim": "observed",
                    })]),
                )],
            ),
            (
                "rc_ledger_digest_mismatch",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                    ),
                    "ledger_digest",
                    json!(hex64(8)),
                )],
            ),
            (
                "rc_result_id_prefix",
                vec![mutate_payload(
                    committed(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                    ),
                    "result_id",
                    json!("RET-RES-deadbeef-1"),
                )],
            ),
            ("rc_assessment_digest_drift", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                );
                let assess =
                    mutate_payload(bound_assessment(&commit), "result_digest", json!(hex64(7)));
                vec![commit, assess]
            }),
            ("rc_assessment_id_prefix", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                );
                let assess = mutate_payload(
                    bound_assessment(&commit),
                    "assessment_id",
                    json!("ASSESS-aaaa-1"),
                );
                vec![commit, assess]
            }),
            ("rc_no_fulltext_missing", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                );
                let assess = mutate_payload(bound_assessment(&commit), "reason_codes", json!([]));
                vec![commit, assess]
            }),
            ("rc_no_fulltext_spurious", {
                let commit = committed(
                    "act-1",
                    1,
                    json!([ledger_entry("s1", "doc_page", "full_text_observed")]),
                );
                let assess = mutate_payload(
                    bound_assessment(&commit),
                    "reason_codes",
                    json!(["no_fulltext_evidence"]),
                );
                vec![commit, assess]
            }),
            (
                "pool_without_urls",
                vec![{
                    let mut commit = finish_commit(
                        "act-1",
                        1,
                        json!([{"source_id": "s1", "source_type": "web_search_result", "visibility": "metadata_only", "candidate_pool": [pool_item("u1")]}]),
                        json!([]),
                    );
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_urls_non_array",
                vec![{
                    let mut commit = finish_commit(
                        "act-1",
                        1,
                        json!([{"source_id": "s1", "source_type": "web_search_result", "visibility": "metadata_only", "candidate_urls": "u1"}]),
                        json!([]),
                    );
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_urls_without_pool",
                vec![{
                    let mut commit = finish_commit(
                        "act-1",
                        1,
                        json!([{"source_id": "s1", "source_type": "web_search_result", "visibility": "metadata_only", "candidate_urls": ["u1"]}]),
                        json!([]),
                    );
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_length_mismatch",
                vec![{
                    let entries = vec![pool_entry(
                        "s1",
                        json!(["u1", "u2"]),
                        json!([pool_item("u1")]),
                    )];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_bad_canonical",
                vec![{
                    let mut item = pool_item("u1");
                    item["canonical_url"] = json!("");
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([item]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_bad_relevance",
                vec![{
                    let mut item = pool_item("u1");
                    item["relevance"] = json!("vibes");
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([item]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "pool_bad_form_reasons",
                vec![{
                    let mut item = pool_item("u1");
                    item["form_reasons"] = json!([5]);
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([item]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([]);
                    commit
                }],
            ),
            (
                "prefilter_missing_sid",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] =
                        json!([{"url": "u2", "reason": "bad_url", "action": "removed"}]);
                    commit
                }],
            ),
            (
                "prefilter_action_not_removed",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "kept"},
                    ]);
                    commit
                }],
            ),
            (
                "prefilter_canonical_empty",
                vec![{
                    let entries = vec![pool_entry("s1", json!(["u1"]), json!([pool_item("u1")]))];
                    let mut commit =
                        finish_commit("act-1", 1, json!(entries.clone()), pool_refs(&entries));
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "removed", "canonical_url": ""},
                    ]);
                    commit
                }],
            ),
            (
                "prefilter_entry_not_pool_source",
                vec![{
                    let mut commit = finish_commit(
                        "act-1",
                        1,
                        json!([ledger_entry("s1", "doc_page", "metadata_only")]),
                        json!([]),
                    );
                    commit["payload"]["prefilter_log"] = json!([
                        {"source_id": "s1", "url": "u2", "reason": "bad_url", "action": "removed"},
                    ]);
                    commit
                }],
            ),
            (
                "cm_c2d_no_streak",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "direct",
                        "trigger": "assistant_failure_streak", "model_decision": "switch",
                        "order_ids": ["ORD-1"], "transition_id": "t1",
                        "related_transition_id": null,
                    }),
                )],
            ),
            (
                "cm_c2d_empty_order_ids",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "direct",
                        "trigger": "assistant_failure_streak", "model_decision": "switch",
                        "streak": 3, "order_ids": [], "transition_id": "t1",
                        "related_transition_id": null,
                    }),
                )],
            ),
            (
                "cm_c2d_related_set",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "direct",
                        "trigger": "assistant_failure_streak", "model_decision": "switch",
                        "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                        "related_transition_id": "t0",
                    }),
                )],
            ),
            (
                "cm_stay_bad_decision",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "console", "to": "console",
                        "trigger": "assistant_failure_streak", "model_decision": "switch",
                        "transition_id": "t1", "related_transition_id": null,
                    }),
                )],
            ),
            (
                "cm_invalid_direction",
                vec![ev(
                    "console_mode_transition",
                    json!({
                        "run_id": "run-1", "from": "direct", "to": "direct",
                        "trigger": "model_return", "model_decision": "return_to_console",
                        "transition_id": "t1", "related_transition_id": "t1",
                    }),
                )],
            ),
            (
                "cm_duplicate_transition_id",
                vec![
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 3, "order_ids": ["ORD-1"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                    ev(
                        "console_mode_transition",
                        json!({
                            "run_id": "run-1", "from": "console", "to": "direct",
                            "trigger": "assistant_failure_streak", "model_decision": "switch",
                            "streak": 4, "order_ids": ["ORD-2"], "transition_id": "t1",
                            "related_transition_id": null,
                        }),
                    ),
                ],
            ),
            (
                "tr_multi_completion",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                ],
            ),
            (
                "tr_no_following_completion",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                ],
            ),
            (
                "tr_running_with_exit",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": 0}),
                    ),
                ],
            ),
            (
                "tr_idle_kill_no_mid",
                vec![ev(
                    "tool_running",
                    json!({
                        "tool": "run_terminal_cmd", "call_id": "c1",
                        "status": "idle_killed", "reason": "no output activity for 300s",
                    }),
                )],
            ),
            (
                "tr_idle_kill_no_completion",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_running",
                        json!({
                            "tool": "run_terminal_cmd", "call_id": "c1",
                            "status": "idle_killed", "reason": "no output activity for 300s",
                        }),
                    ),
                ],
            ),
            (
                "tr_idle_kill_empty_reason",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                    ev(
                        "tool_running",
                        json!({
                            "tool": "run_terminal_cmd", "call_id": "c1",
                            "status": "idle_killed", "reason": "",
                        }),
                    ),
                ],
            ),
            (
                "tr_idle_kill_duplicate",
                vec![
                    tstart("run_terminal_cmd", "c1"),
                    ev(
                        "tool_running",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "run_terminal_cmd", "call_id": "c1", "status": "success", "running": true, "exit_code": null}),
                    ),
                    ev(
                        "tool_running",
                        json!({
                            "tool": "run_terminal_cmd", "call_id": "c1",
                            "status": "idle_killed", "reason": "no output activity for 300s",
                        }),
                    ),
                    ev(
                        "tool_running",
                        json!({
                            "tool": "run_terminal_cmd", "call_id": "c1",
                            "status": "idle_killed", "reason": "no output activity for 300s",
                        }),
                    ),
                ],
            ),
            (
                "rh_initial_with_change_kind",
                vec![header_event(
                    "initial",
                    "H1",
                    None,
                    "S1",
                    "T1",
                    "C1",
                    vec!["read_file"],
                    Some("tools"),
                )],
            ),
            (
                "rh_change_no_initial",
                vec![header_event(
                    "change",
                    "H2",
                    Some("H1"),
                    "S1",
                    "T2",
                    "C1",
                    vec!["read_file", "grep"],
                    Some("tools"),
                )],
            ),
            (
                "rh_same_header",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H2"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        Some("tools"),
                    ),
                ],
            ),
            (
                "rh_missing_change_kind",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S1",
                        "T2",
                        "C1",
                        vec!["read_file", "grep"],
                        None,
                    ),
                ],
            ),
            (
                "rh_tool_count_mismatch",
                vec![mutate_payload(
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file", "grep"],
                        None,
                    ),
                    "tool_count",
                    json!(5),
                )],
            ),
            (
                "rh_bad_reason",
                vec![header_event(
                    "bored",
                    "H1",
                    None,
                    "S1",
                    "T1",
                    "C1",
                    vec!["read_file"],
                    None,
                )],
            ),
            (
                "rh_change_kind_multiple",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S2",
                        "T2",
                        "C2",
                        vec!["read_file"],
                        Some("system"),
                    ),
                ],
            ),
            (
                "rh_change_kind_config_ok",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    header_event(
                        "change",
                        "H2",
                        Some("H1"),
                        "S1",
                        "T1",
                        "C2",
                        vec!["read_file"],
                        Some("config"),
                    ),
                ],
            ),
            (
                "dg_empty_path",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "read", "path": "", "anchor": null},
                    }),
                )],
            ),
            (
                "dg_anchor_non_object",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "read", "path": "a.rs", "anchor": 5},
                    }),
                )],
            ),
            (
                "dg_consumed_non_string",
                vec![ev(
                    "tool_completed",
                    json!({
                        "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                        "dep_graph": {"kind": "write", "path": "a.rs", "consumed_read": 5,
                            "consumed_anchor": null, "new_anchor": null},
                    }),
                )],
            ),
            (
                "dg_consumed_empty_string",
                vec![
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file", "call_id": "", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "read", "path": "a.rs", "anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}},
                        }),
                    ),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "write", "path": "a.rs", "consumed_read": "",
                                "consumed_anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}, "new_anchor": null},
                        }),
                    ),
                ],
            ),
            (
                "dg_path_mismatch",
                vec![
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file", "call_id": "r1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "read", "path": "a.rs", "anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}},
                        }),
                    ),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "search_replace", "call_id": "w1", "status": "success", "exit_code": 0,
                            "dep_graph": {"kind": "write", "path": "b.rs", "consumed_read": "r1",
                                "consumed_anchor": {"sha256": hex64(1), "size": 10, "mtime": 1}, "new_anchor": null},
                        }),
                    ),
                ],
            ),
            (
                "ma_payload_not_object",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": "flat text",
                    }),
                )],
            ),
            (
                "ma_key_empty",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": {"key": "", "round": 3, "summary": "read a.rs", "anomaly": null},
                    }),
                )],
            ),
            (
                "ma_summary_empty",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "tool_result",
                        "payload": {"key": "file:a.rs", "round": 3, "summary": "", "anomaly": null},
                    }),
                )],
            ),
            (
                "ma_plan_gate_ok",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "plan_gate",
                        "payload": {"key": "plan", "round": 1, "summary": "plan gate closed", "anomaly": null},
                    }),
                )],
            ),
            (
                "ma_budget_ok",
                vec![ev(
                    "mechanical_audit_update",
                    json!({
                        "kind": "budget",
                        "payload": {"key": "budget", "round": 2, "summary": "budget cue", "anomaly": "overflow"},
                    }),
                )],
            ),
            (
                "cc_guard_failed_session_end",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "mechanical", "reason": "session_end", "summary_incomplete": false,
                        "guard_failed": true,
                        "summary_id": "S1", "summary_digest": "d1", "summary_path": "p1",
                    }),
                )],
            ),
            (
                "cc_complete_missing_archive",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "mechanical", "reason": "rhythm", "summary_incomplete": false,
                        "summary_id": "S1", "summary_digest": null, "summary_path": "p1",
                    }),
                )],
            ),
            (
                "cc_mode_invalid",
                vec![ev(
                    "context_compressed",
                    json!({
                        "mode": "vibes", "reason": "rhythm", "summary_incomplete": false,
                        "summary_id": "S1", "summary_digest": "d1", "summary_path": "p1",
                    }),
                )],
            ),
            (
                "pw_validation_failed_degrade",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "degraded", "degrade_reason": "validation_failed", "validation": {"valid": true}}),
                )],
            ),
            (
                "pw_accepted_valid_false",
                vec![ev(
                    "plan_write",
                    json!({"attempt": 1, "outcome": "accepted", "validation": {"valid": false}}),
                )],
            ),
            (
                "rcpt_gate_before_start",
                vec![
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "search_replace", "call_id": "c1",
                            "status": "error", "exit_code": 1,
                            "policy_denial": {"source": "acaf", "code": "denied", "reason": "mechanical refusal"},
                        }),
                    ),
                    tstart("search_replace", "c1"),
                ],
            ),
            // ── 0t S2-R P5（2026-09-09）：retrieval_enable_gate /
            // browser_launch_result 两族正反例场景 ─────────────────────
            (
                "enable_gate_declared_dual_lane_ok",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec![
                            "read_file",
                            "web_search",
                            "web_fetch",
                            "browser_read",
                            "browser_control",
                        ],
                        None,
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "web_search", "call_id": "w1", "target": "external_retrieval"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "web_search", "call_id": "w1", "target": "external_retrieval", "status": "ok"}),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "browser_read", "call_id": "b1"}),
                    ),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0001", "status": "success", "cause": null}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "ok", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "browser_control", "call_id": "bc1"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_control", "call_id": "bc1", "status": "ok"}),
                    ),
                ],
            ),
            (
                "enable_gate_disabled_host_dispatch",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file", "grep"],
                        None,
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "browser_control", "call_id": "bc1"}),
                    ),
                ],
            ),
            (
                "enable_gate_disabled_web_dispatch",
                vec![
                    header_event(
                        "initial",
                        "H1",
                        None,
                        "S1",
                        "T1",
                        "C1",
                        vec!["read_file"],
                        None,
                    ),
                    ev(
                        "tool_started",
                        json!({"tool": "web_search", "call_id": "w1", "target": "external_retrieval"}),
                    ),
                ],
            ),
            (
                "browser_launch_s1_failure_fact_ok",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0002", "status": "failure", "cause": "browser_not_found: ORZ_BROWSER_PATH unset"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "error", "error": "browser_launch_failed", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                ],
            ),
            (
                "browser_launch_s1_missing_fact",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "error", "error": "browser_launch_failed", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                ],
            ),
            (
                "browser_launch_s1_late_fact",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "error", "error": "browser_launch_failed", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0003", "status": "failure", "cause": "browser_not_found: ORZ_BROWSER_PATH unset"}),
                    ),
                ],
            ),
            (
                "browser_launch_s1_control_failure_fact_ok",
                vec![
                    tstart("browser_control", "bc1"),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0004", "status": "failure", "cause": "browser_not_found: ORZ_BROWSER_PATH unset"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_control", "call_id": "bc1", "status": "error", "error": "browser_launch_failed"}),
                    ),
                ],
            ),
            (
                "browser_launch_s1_control_missing_fact",
                vec![
                    tstart("browser_control", "bc1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_control", "call_id": "bc1", "status": "error", "error": "browser_launch_failed"}),
                    ),
                ],
            ),
            (
                "browser_launch_s2_success_ok",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0005", "status": "success", "cause": null}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "ok", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                ],
            ),
            (
                "browser_launch_s3_step_failed_ok",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "browser_launch_result",
                        json!({"attempt_id": "BLAUNCH-RUN-1-0006", "status": "success", "cause": null}),
                    ),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "error", "error": "browser_read failed [browser_read_empty_content]: page produced no readable text", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                ],
            ),
            (
                "browser_launch_s4_ready_no_fact_ok",
                vec![
                    tstart("browser_read", "b1"),
                    ev(
                        "tool_completed",
                        json!({"tool": "browser_read", "call_id": "b1", "status": "ok", "candidate_count": 1, "candidate_cap": 8}),
                    ),
                ],
            ),
        ]
    }

    /// Expected verdicts per scenario per family (the S2b/S2c spec table).
    /// `true` = the family must report at least one error.
    fn expected_violations() -> Vec<(&'static str, Vec<(&'static str, bool)>)> {
        let ok: Vec<(&'static str, bool)> =
            ALL_FAMILIES.iter().map(|family| (*family, false)).collect();
        let mut rows: Vec<(&'static str, Vec<(&'static str, bool)>)> = scenarios()
            .iter()
            .map(|(name, _)| (*name, ok.clone()))
            .collect();
        let mut expect = |scenario: &'static str, family: &'static str| {
            let row = rows
                .iter_mut()
                .find(|(name, _)| *name == scenario)
                .expect("known scenario");
            let cell = row
                .1
                .iter_mut()
                .find(|(f, _)| *f == family)
                .expect("known family");
            cell.1 = true;
        };
        // 0z S2 (2026-09-12): host-resource family violations — the
        // violations scenario trips exactly its seven families.
        for family in HOST_RESOURCE_FAMILIES {
            expect("host_resource_families_violations", family);
        }
        expect("control_tickets_unknown", "control_tickets");
        expect("control_tickets_kind_conflict", "control_tickets");
        expect("control_tickets_one_shot", "control_tickets");
        expect("retrieval_mode_double_bootstrap", "retrieval_mode");
        expect("retrieval_mode_chain_break", "retrieval_mode");
        expect("retrieval_mode_off_dispatch", "retrieval_mode");
        expect("retrieval_mode_lb_unavailable", "retrieval_mode");
        expect("retrieval_mode_lb_missing_capability", "retrieval_mode");
        expect("retrieval_mode_off_target_only", "retrieval_mode");
        expect("retrieval_mode_lb_target_only", "retrieval_mode");
        expect("ledger_advance_after_not_below", "ledger_fold_advance");
        expect("ledger_advance_cut_not_increasing", "ledger_fold_advance");
        expect("ledger_advance_start_changed", "ledger_fold_advance");
        expect("ledger_advance_start_ge_cut", "ledger_fold_advance");
        expect("ledger_advance_rounds_shrink", "ledger_fold_advance");
        expect(
            "ledger_write_failed_bad_disabled",
            "ledger_fold_write_failed",
        );
        expect(
            "ledger_write_failed_attempt_jump",
            "ledger_fold_write_failed",
        );
        expect(
            "ledger_write_failed_after_exhausted",
            "ledger_fold_write_failed",
        );
        expect("policy_denial_bad_status", "policy_denial");
        expect("policy_denial_zero_exit", "policy_denial");
        expect("policy_denial_wrong_family", "policy_denial");
        expect("policy_denial_unknown_source", "policy_denial");
        expect("policy_denial_permission_web_fetch", "policy_denial");
        expect("failure_target_bad_id", "failure_target");
        expect("failure_target_wrong_tool", "failure_target");
        expect("failure_target_preview_too_long", "failure_target");
        expect("failure_target_file_wrong_tool", "failure_target");
        expect("failure_target_url_wrong_tool", "failure_target");
        expect("lifecycle_unknown_assessment", "lifecycle");
        expect("lifecycle_cas_mismatch", "lifecycle");
        expect("lifecycle_revision_decrease", "lifecycle");
        expect("lifecycle_stale_accepted", "lifecycle");
        expect("lifecycle_double_close", "lifecycle");
        expect("lifecycle_after_close", "lifecycle");
        expect("lifecycle_conflicting_replay", "lifecycle");
        expect("lifecycle_close_bad_decision", "lifecycle");
        expect("lifecycle_close_bad_outcome", "lifecycle");
        expect("lifecycle_close_revision_mismatch", "lifecycle");
        expect("lifecycle_close_digest_mismatch", "lifecycle");
        expect("lifecycle_restore_not_preceding", "lifecycle");
        expect("lifecycle_restore_activation_mismatch", "lifecycle");
        expect("lifecycle_continue_plus_one_wrong", "lifecycle");
        expect("failure_target_url_wrong_tool", "failure_target");
        expect("lifecycle_close_replayed_outcome_errors", "lifecycle");
        // S2c cross-family cell: the ledger window-reset scenario journals a
        // bare context_compressed marker, which the S2c context_compressed
        // family (correctly) flags on BOTH judges.
        expect("ledger_advance_window_reset_ok", "context_compressed");
        // S2c cross-family cells: these failure_target scenarios carry a
        // non-gate error completion without a preceding tool_started, so the
        // receipt_event_isomorphism family (S2c) reports them as well.
        for name in [
            "failure_target_ok",
            "failure_target_bad_id",
            "failure_target_wrong_tool",
            "failure_target_preview_too_long",
            "failure_target_file_ok",
            "failure_target_file_wrong_tool",
            "failure_target_url_ok",
            "failure_target_url_wrong_tool",
        ] {
            expect(name, "receipt_event_isomorphism");
        }
        // ── S2c violation rows ─────────────────────────────────────────
        expect("inquiry_kind_mismatch", "inquiry_kind");
        // P0-0x S2 (ADR-0010 §14.66): trigger ↔ block ↔ position coupling
        // and the one-shot contract (`initial_round_ok` expects NO violation).
        for name in [
            "initial_round_wrong_block",
            "initial_round_wrong_position",
            "initial_round_under_periodic_trigger",
            "initial_round_twice",
        ] {
            expect(name, "initial_round_inquiry");
        }
        for name in [
            "plan_write_refill_attempt_2",
            "plan_write_refill_unfollowed",
            "plan_write_refill_valid_true",
            "plan_write_after_refill_attempt_1",
            "plan_write_rotate_degrade_valid_false",
            "plan_write_accepted_attempt_3",
        ] {
            expect(name, "plan_write");
        }
        for name in [
            "console_mode_c2d_bad_trigger",
            "console_mode_d2c_bad_related",
            "console_mode_double_streak",
            "console_mode_direct_tool_unstamped",
            "console_mode_direct_tool_wrong_tid",
        ] {
            expect(name, "console_mode_transition");
        }
        // console_order_written family retired 2026-09-06 (S2d 裁决一,
        // ADR-0010 §14.57): the written scenarios are historical-replay-legal
        // guards and expect NO violations from any family (absent here).
        for name in [
            "console_order_rejected_duplicate",
            "console_order_rejected_empty_reason",
            "console_order_rejected_reason_null",
            "console_order_rejected_issue_bad_step",
            "console_order_rejected_pre_issue_bad_step",
            "console_order_rejected_bad_phase",
            "console_order_rejected_pre_issue_bad_code",
        ] {
            expect(name, "console_order_rejected");
        }
        for name in [
            "tool_running_duplicate",
            "tool_running_no_start",
            "tool_running_completion_not_running",
            "tool_running_idle_kill_premature",
        ] {
            expect(name, "tool_running");
        }
        for name in [
            "output_truncation_missing_bytes",
            "output_truncation_object_unmarked",
        ] {
            expect(name, "output_truncation");
        }
        for name in ["budget_cue_cap_exceeded", "budget_cue_not_below_threshold"] {
            expect(name, "budget_cue_injected");
        }
        for name in [
            "result_consistency_counts_mismatch",
            "result_consistency_degraded_mismatch",
            "result_consistency_bad_digest",
            "result_consistency_retired_organized",
            "result_consistency_assessment_drift",
        ] {
            expect(name, "result_consistency");
        }
        for name in ["reason_codes_retired", "reason_codes_unknown"] {
            expect(name, "reason_codes");
        }
        for name in [
            "source_weighting_web_page_no_tier",
            "source_weighting_partial_fields",
            "source_weighting_wrong_weight",
            "source_weighting_retired_annotation",
        ] {
            expect(name, "source_weighting");
        }
        for name in [
            "candidate_pool_non_web_search",
            "candidate_pool_empty_no_removal",
            "candidate_pool_url_mismatch",
            "candidate_pool_refs_not_mirrored",
        ] {
            expect(name, "search_candidate_pool");
        }
        for name in [
            "candidate_prefilter_missing_log",
            "candidate_prefilter_bad_reason",
            "candidate_prefilter_removed_retained",
            "candidate_prefilter_duplicate_entry",
        ] {
            expect(name, "candidate_prefilter");
        }
        for name in [
            "candidate_count_fields_alone",
            "candidate_count_wrong_family",
            "candidate_count_missing_on_lane",
            "candidate_count_cap_mismatch",
            "v01_tool_budget_fields",
        ] {
            expect(name, "candidate_count");
        }
        for name in [
            "inject_budget_missing_fields",
            "inject_budget_fields_other_code",
            "inject_budget_zero_budget",
            "v01_tool_budget_fields",
        ] {
            expect(name, "inject_budget");
        }
        for name in [
            "receipt_completed_before_start",
            "receipt_non_gate_no_start",
            "receipt_duplicate_completed",
            "receipt_open_start_at_terminal",
        ] {
            expect(name, "receipt_event_isomorphism");
        }
        for name in [
            "dep_graph_bad_kind",
            "dep_graph_wrong_tool",
            "dep_graph_non_success",
            "dep_graph_dangling_consumed",
            "dep_graph_anchor_mismatch",
        ] {
            expect(name, "dep_graph_events");
        }
        for name in [
            "mechanical_audit_bad_kind",
            "mechanical_audit_negative_round",
            "mechanical_audit_anomaly_type",
        ] {
            expect(name, "mechanical_audit");
        }
        for name in [
            "recovery_truncation_after_model_request",
            "recovery_truncation_zero_rounds",
        ] {
            expect(name, "recovery_truncation");
        }
        for name in [
            "context_compressed_bad_reason",
            "context_compressed_mechanical_incomplete",
            "context_compressed_incomplete_with_archive",
            "context_compressed_write_failed_on_incomplete",
        ] {
            expect(name, "context_compressed");
        }
        expect("activation_restore_duplicate", "activation_restore");
        for name in [
            "probe_overlap",
            "probe_judgment_word",
            "probe_partition_extra",
        ] {
            expect(name, "tool_availability_probe");
        }
        for name in [
            "request_header_initial_with_previous",
            "request_header_change_prev_mismatch",
            "request_header_change_kind_wrong",
            "request_header_tools_dup",
        ] {
            expect(name, "request_header");
        }
        expect("probe_accuracy_unreported_flip", "probe_accuracy");
        // ── S2c review-handling coverage rows (2026-09-06) ──────────────
        for name in [
            "rc_unknown_visibility",
            "rc_claim_mismatch",
            "rc_ledger_digest_mismatch",
            "rc_result_id_prefix",
            "rc_assessment_digest_drift",
            "rc_assessment_id_prefix",
            "rc_no_fulltext_missing",
            "rc_no_fulltext_spurious",
        ] {
            expect(name, "result_consistency");
        }
        for name in [
            "pool_without_urls",
            "pool_urls_non_array",
            "pool_urls_without_pool",
            "pool_length_mismatch",
            "pool_bad_canonical",
            "pool_bad_relevance",
            "pool_bad_form_reasons",
        ] {
            expect(name, "search_candidate_pool");
        }
        for name in [
            "prefilter_missing_sid",
            "prefilter_action_not_removed",
            "prefilter_canonical_empty",
            "prefilter_entry_not_pool_source",
        ] {
            expect(name, "candidate_prefilter");
        }
        for name in [
            "cm_c2d_no_streak",
            "cm_c2d_empty_order_ids",
            "cm_c2d_related_set",
            "cm_stay_bad_decision",
            "cm_invalid_direction",
            "cm_duplicate_transition_id",
        ] {
            expect(name, "console_mode_transition");
        }
        for name in [
            "tr_no_following_completion",
            "tr_running_with_exit",
            "tr_idle_kill_no_mid",
            "tr_idle_kill_no_completion",
            "tr_idle_kill_empty_reason",
            "tr_idle_kill_duplicate",
        ] {
            expect(name, "tool_running");
        }
        // Multi-completion also breaks the receipt run-level pairing (S2c
        // receipt_event_isomorphism duplicate-completed branch).
        expect("tr_multi_completion", "tool_running");
        expect("tr_multi_completion", "receipt_event_isomorphism");
        for name in [
            "rh_initial_with_change_kind",
            "rh_change_no_initial",
            "rh_same_header",
            "rh_missing_change_kind",
            "rh_tool_count_mismatch",
            "rh_bad_reason",
            "rh_change_kind_multiple",
        ] {
            expect(name, "request_header");
        }
        for name in [
            "dg_empty_path",
            "dg_anchor_non_object",
            "dg_consumed_non_string",
            "dg_consumed_empty_string",
            "dg_path_mismatch",
        ] {
            expect(name, "dep_graph_events");
        }
        for name in ["ma_payload_not_object", "ma_key_empty", "ma_summary_empty"] {
            expect(name, "mechanical_audit");
        }
        for name in [
            "cc_guard_failed_session_end",
            "cc_complete_missing_archive",
            "cc_mode_invalid",
        ] {
            expect(name, "context_compressed");
        }
        for name in ["pw_validation_failed_degrade", "pw_accepted_valid_false"] {
            expect(name, "plan_write");
        }
        expect("rcpt_gate_before_start", "receipt_event_isomorphism");
        // 0t S2-R P5（2026-09-09）：retrieval_enable_gate /
        // browser_launch_result 两族正反例期望表。
        expect(
            "enable_gate_disabled_host_dispatch",
            "retrieval_enable_gate",
        );
        expect("enable_gate_disabled_web_dispatch", "retrieval_enable_gate");
        expect("browser_launch_s1_missing_fact", "browser_launch_result");
        expect("browser_launch_s1_late_fact", "browser_launch_result");
        expect(
            "browser_launch_s1_control_missing_fact",
            "browser_launch_result",
        );
        rows
    }

    #[test]
    fn family_verdicts_match_spec_table() {
        // Written-rule retirement guard (S2d 裁决一, ADR-0010 §14.57): the
        // three historical-replay scenarios must produce ZERO errors from
        // every family — the retirement must never drift into a negative
        // check on one side only (the crosscheck below could hide a shared
        // both-sides drift).
        for name in [
            "console_order_written_ok",
            "console_order_written_duplicate",
            "console_order_written_unbacked",
        ] {
            let events = scenarios()
                .into_iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("scenario {name} missing"))
                .1;
            assert!(
                verify_all_families(&events).is_empty(),
                "written-chain historical scenario {name} must replay legal"
            );
        }
        for (name, expected) in expected_violations() {
            let events = scenarios()
                .into_iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("scenario {name} missing"))
                .1;
            for (family, expect_violation) in expected {
                let errors = verify_family(family, &events);
                assert_eq!(
                    !errors.is_empty(),
                    expect_violation,
                    "scenario {name} family {family}: {errors:?}"
                );
            }
        }
    }

    /// Task D acceptance: per-family verdict parity with the Python judge on
    /// the same corpus (the full `ALL_FAMILIES` roster since the S2d 裁决一
    /// written-rule retirement, ADR-0010 §14.57; +0q failure pipeline +0t
    /// retrieval gate / browser_launch_result 两族 2026-09-09; +0z
    /// host-resource 七族 2026-09-12; +0ac immediate-feedback 五族 2026-09-13)
    /// — the synthetic scenarios
    /// above PLUS every real v0.2 fixture journal. Verdict parity =
    /// (errors empty) agrees on both sides; message text is deliberately
    /// Rust-form.
    #[test]
    fn s2b_family_verdicts_match_python() {
        // Mount-contract guard (ORZ-BUILD-MOUNT-001): the judge reads the
        // Python module and the fixture corpus from the parent repository.
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let assurance_dir = repo_root.join("assurance");
        assert!(
            assurance_dir
                .join("run_event_journal_validation.py")
                .is_file(),
            "S2b crosscheck must run inside the parent repository \
             (assurance/ missing at {})",
            assurance_dir.display()
        );

        // Corpus = synthetic scenarios + real fixture journals (both tracks —
        // policy_denial / failure_target run on v0.1 journals too).
        let scenario_count = scenarios().len();
        let mut corpus: Vec<(String, Vec<Value>)> = scenarios()
            .into_iter()
            .map(|(name, events)| (name.to_string(), events))
            .collect();
        let mut fixture_names: Vec<String> = Vec::new();
        for track in ["run-event-v0.2", "run-event-v0.1"] {
            let fixtures_dir = repo_root
                .join("runtime/fixtures")
                .join(track)
                .join("journals");
            let names: Vec<String> = std::fs::read_dir(&fixtures_dir)
                .unwrap_or_else(|e| {
                    panic!(
                        "fixture journals dir {} unreadable: {e}",
                        fixtures_dir.display()
                    )
                })
                .map(|entry| {
                    entry
                        .expect("fixture entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .filter(|name| name.ends_with(".jsonl"))
                .collect();
            assert!(
                !names.is_empty(),
                "fixture journals dir {} is empty — the crosscheck would \
                 silently lose its real-journal signal",
                fixtures_dir.display()
            );
            for name in &names {
                let text =
                    std::fs::read_to_string(fixtures_dir.join(name)).expect("fixture journal");
                let events: Vec<Value> = text
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| serde_json::from_str(line).expect("fixture line"))
                    .collect();
                corpus.push((format!("{track}/{name}"), events));
                fixture_names.push(format!("{track}/{name}"));
            }
        }
        fixture_names.sort();

        // Python side: run every `_verify_v02_*` function per corpus item
        // (console_order_written retired 2026-09-06, ADR-0010 §14.57);
        // the five 0ac immediate-feedback twins joined 2026-09-13 (S3-b).
        let script = r#"
import sys, json
sys.path.insert(0, sys.argv[1])
import run_event_journal_validation as v
fams = {
    "inquiry_kind": v._verify_v02_inquiry_kind,
    "initial_round_inquiry": v._verify_v02_initial_round_inquiry,
    "plan_write": v._verify_v02_plan_write,
    "console_mode_transition": v._verify_v02_console_mode_transition,
    "console_order_rejected": v._verify_v02_console_order_rejected,
    "ledger_fold_advance": v._verify_v02_ledger_fold_advance,
    "ledger_fold_write_failed": v._verify_v02_ledger_fold_write_failed,
    "lifecycle": v._verify_v02_lifecycle,
    "tool_running": v._verify_v02_tool_running,
    "output_truncation": v._verify_v02_output_truncation,
    "budget_cue_injected": v._verify_v02_budget_cue_injected,
    "retrieval_mode": v._verify_v02_retrieval_mode,
    "retrieval_enable_gate": v._verify_v02_retrieval_enable_gate,
    "browser_launch_result": v._verify_v02_browser_launch_result,
    "result_consistency": v._verify_v02_result_consistency,
    "reason_codes": v._verify_v02_reason_codes,
    "source_weighting": v._verify_v02_source_weighting,
    "search_candidate_pool": v._verify_v02_search_candidate_pool,
    "candidate_prefilter": v._verify_v02_candidate_prefilter,
    "candidate_count": v._verify_v02_candidate_count,
    "inject_budget": v._verify_v02_inject_budget,
    "policy_denial": v._verify_v02_policy_denial,
    "failure_target": v._verify_v02_failure_target,
    "failure_agg_coverage": v._verify_v02_failure_agg_coverage,
    "receipt_event_isomorphism": v._verify_v02_receipt_event_isomorphism,
    "dep_graph_events": v._verify_v02_dep_graph_events,
    "mechanical_audit": v._verify_v02_mechanical_audit,
    "recovery_truncation": v._verify_v02_recovery_truncation,
    "context_compressed": v._verify_v02_context_compressed,
    "activation_restore": v._verify_v02_activation_restore,
    "control_tickets": v._verify_v02_control_tickets,
    "tool_availability_probe": v._verify_v02_tool_availability_probe,
    "request_header": v._verify_v02_request_header,
    "probe_accuracy": v._verify_v02_probe_accuracy,
    "host_resource_snapshot": v._verify_v02_host_resource_snapshot,
    "host_resource_denied": v._verify_v02_host_resource_denied,
    "resource_exhausted": v._verify_v02_resource_exhausted,
    "run_terminated": v._verify_v02_run_terminated,
    "process_tree_reaped": v._verify_v02_process_tree_reaped,
    "reclaim_performed": v._verify_v02_reclaim_performed,
    "resource_limit_hit": v._verify_v02_resource_limit_hit,
    "retrieval_dedupe": v._verify_v02_retrieval_dedupe,
    "result_delivered_accounting": v._verify_v02_result_delivered_accounting,
    "retrieval_family_probe": v._verify_v02_retrieval_family_probe,
    "failure_cause_shape": v._verify_v02_failure_cause_shape,
    "first_result_deadline": v._verify_v02_first_result_deadline,
}
data = json.load(sys.stdin)
out = {}
for sc in data:
    out[sc["name"]] = {n: bool(fn(sc["events"])) for n, fn in fams.items()}
json.dump(out, sys.stdout)
"#;
        let corpus_json = serde_json::to_string(
            &corpus
                .iter()
                .map(|(name, events)| json!({"name": name, "events": events}))
                .collect::<Vec<_>>(),
        )
        .expect("serialize corpus");

        use std::io::Write;
        use std::process::{Command, Stdio};
        let python = std::env::var("ORZ_PYTHON").unwrap_or_else(|_| "python".into());
        // -X utf8: the corpus carries non-ASCII fixture text — never decode
        // stdin through the platform ANSI code page (S2b review P2).
        let spawned = Command::new(&python)
            .arg("-X")
            .arg("utf8")
            .arg("-c")
            .arg(script)
            .arg(assurance_dir.display().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => panic!(
                "spawn `{python}` for the S2b family crosscheck failed: {e} — \
                 install Python (with the jsonschema package) or point \
                 ORZ_PYTHON at an interpreter"
            ),
        };
        let write = child
            .stdin
            .take()
            .expect("python stdin")
            .write_all(corpus_json.as_bytes());
        if let Err(e) = write {
            let _ = child.wait();
            panic!("write corpus to python failed (interpreter exited early?): {e}");
        }
        let output = child.wait_with_output().expect("python crosscheck");
        assert!(
            output.status.success(),
            "python crosscheck failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let py_verdicts: BTreeMap<String, BTreeMap<String, bool>> =
            serde_json::from_slice(&output.stdout).expect("python verdict JSON");

        // Rust side + verdict comparison (0 差 = every cell agrees).
        let mut checked = 0usize;
        for (name, events) in &corpus {
            let py_row = py_verdicts
                .get(name)
                .unwrap_or_else(|| panic!("python missing scenario {name}"));
            for family in ALL_FAMILIES {
                let rust_violation = !verify_family(family, events).is_empty();
                let py_violation = *py_row
                    .get(*family)
                    .unwrap_or_else(|| panic!("python missing family {family}"));
                assert_eq!(
                    rust_violation, py_violation,
                    "verdict mismatch on {name}/{family} \
                     (rust={rust_violation}, python={py_violation})"
                );
                checked += 1;
            }
        }
        // Exact cell accounting (a tautological floor would let fixture or
        // scenario loss pass silently — S2b review P2).
        assert_eq!(
            checked,
            ALL_FAMILIES.len() * corpus.len(),
            "crosscheck cell accounting drifted"
        );
        assert_eq!(
            scenario_count, 251,
            "synthetic scenario corpus count drifted from its registered size              ({scenario_count})"
        );
        assert!(
            fixture_names.len() >= 18,
            "real fixture journals shrunk below their registered floor ({})",
            fixture_names.len()
        );
    }

    // ==== 0p S2 复审补测（2026-09-07）：两段门/读沙箱事件面载荷直调
    // 法官族 + v0.2 载荷 schema 校验——修复前 `session_volume_*` 载荷
    // 零 assurance 侧覆盖（复审 P3-6）。独立测试不进 Rust↔Python 对拍
    // corpus（对拍场景数钉死，Python 侧为冻结 reference）。

    /// 0p S2 设计 C：三形态完成事件全部过 `verify_policy_denial` 族——
    /// 首读通知（session_volume_notice）、逃逸恒拒
    /// （session_volume_agent_invisible）、通知后放行（session_volume_
    /// opened，非 deny 形态）。
    #[test]
    fn policy_denial_session_volume_payloads_pass_family() {
        let notice = vec![ev(
            "tool_completed",
            json!({
                "tool": "read_file", "status": "error", "exit_code": 1,
                "policy_denial": {"source": "permission", "code": "session_volume_notice",
                    "reason": "session volume first access: duties and structure preview \
                               provided; read again to open (open_after_notice)"},
            }),
        )];
        assert!(
            verify_policy_denial(&notice).is_empty(),
            "notice envelope must pass: {:?}",
            verify_policy_denial(&notice)
        );

        let invisible = vec![ev(
            "tool_completed",
            json!({
                "tool": "grep", "status": "error", "exit_code": 1,
                "policy_denial": {"source": "permission",
                    "code": "session_volume_agent_invisible",
                    "reason": "escaping canonical target"},
            }),
        )];
        assert!(
            verify_policy_denial(&invisible).is_empty(),
            "escape denial must pass: {:?}",
            verify_policy_denial(&invisible)
        );

        // 通知后放行是成功完成（status=ok + opened 标记），不是 deny。
        let opened = vec![ev(
            "tool_completed",
            json!({
                "tool": "read_file", "status": "ok", "exit_code": 0,
                "session_volume_opened": true,
            }),
        )];
        assert!(
            verify_policy_denial(&opened).is_empty(),
            "opened completion is not a denial: {:?}",
            verify_policy_denial(&opened)
        );

        // 反例锁定：denial 载荷缺非零 exit / status=ok → 族仍拦截。
        let broken = vec![ev(
            "tool_completed",
            json!({
                "tool": "read_file", "status": "ok", "exit_code": 0,
                "policy_denial": {"source": "permission", "code": "session_volume_notice",
                    "reason": "self-describing refusal required"},
            }),
        )];
        assert!(
            !verify_policy_denial(&broken).is_empty(),
            "status=ok + exit=0 denial must be rejected by the family"
        );
    }

    /// v0.2 载荷 schema（additionalProperties=false + const true）接受
    /// `session_volume_opened` 载荷并拒绝假值——schema 层的机械锁定。
    #[test]
    fn tool_completed_payload_schema_accepts_session_volume_opened() {
        // 从 crate 目录向上探测含 runtime/ 的仓库根（容器挂载形态安全）。
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut dir = manifest.clone();
        let schema_path = loop {
            let candidate = dir.join("runtime/tool-completed-event-payload-v0.2.schema.json");
            if candidate.exists() {
                break candidate;
            }
            dir = dir
                .parent()
                .expect("repo root with runtime/ schema not found")
                .to_path_buf();
        };
        let text = std::fs::read_to_string(&schema_path).expect("schema file reads");
        let schema: Value = serde_json::from_str(&text).expect("schema parses");
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .should_validate_formats(false)
            .build(&schema)
            .expect("schema compiles");
        // 生产形状：成功完成不写 status 字段（schema 合约「status 存在即
        // 必须为 error」——status 只在拒绝完成上由生产者写入）。
        let payload = json!({
            "tool": "read_file", "call_id": "call-1",
            "exit_code": 0,
            "session_volume_opened": true,
        });
        validator
            .validate(&payload)
            .expect("opened payload must validate against v0.2 schema");
        let bad = json!({
            "tool": "read_file", "call_id": "call-1",
            "exit_code": 0,
            "session_volume_opened": false,
        });
        assert!(
            validator.validate(&bad).is_err(),
            "const-true must reject false"
        );
        // 通知信封完成事件（status=error + 结构化 denial）同过 schema。
        let notice = json!({
            "tool": "read_file", "call_id": "call-2",
            "status": "error", "exit_code": 1,
            "policy_denial": {"source": "permission", "code": "session_volume_notice",
                "reason": "first access notice"},
        });
        validator
            .validate(&notice)
            .expect("notice envelope completion must validate");
    }

    // ==== 0q 统一失败事件管线（2026-09-08，ADR-0010 §14.63）：法官对账
    // 族 `failure_agg_coverage` 正反两测 + 专属 Rust↔Python 对拍。独立
    // 测试不进 pinned 对拍 corpus（场景数钉死，Python 侧为冻结 reference）。

    /// Post-funnel journal scaffold: run_started carries the
    /// `failure_pipeline: "funnel-v1"` grandfather anchor.
    fn funnel_run_started() -> Value {
        ev(
            "run_started",
            json!({"prompt": "p", "failure_pipeline": "funnel-v1"}),
        )
    }

    fn stamped_error(tool: &str, ft: Value) -> Value {
        ev(
            "tool_completed",
            json!({
                "tool": tool,
                "call_id": "call-1",
                "exit_code": 1,
                "status": "error",
                "error": "boom",
                "failure_target": ft,
            }),
        )
    }

    #[test]
    fn failure_agg_coverage_post_funnel_positive_and_grandfather() {
        // 正测：post-funnel 刊，error 形状完成带 failure_target（盖章）或
        // failure_agg_absent（标记）→ 0 错。
        let ok = vec![
            funnel_run_started(),
            stamped_error(
                "read_file",
                json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
            ),
            ev(
                "tool_completed",
                json!({
                    "tool": "grep",
                    "call_id": "call-2",
                    "exit_code": 1,
                    "status": "error",
                    "error": "boom",
                    "failure_agg_absent": true,
                }),
            ),
        ];
        assert!(
            verify_failure_agg_coverage(&ok).is_empty(),
            "{:?}",
            verify_failure_agg_coverage(&ok)
        );

        // grandfather：同一形状去掉 run_started 版本锚（漏斗前旧刊）→
        // 不回溯执法，连「漏盖」负测形状也放行。
        let legacy = vec![stamped_error(
            "read_file",
            json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
        )];
        assert!(verify_failure_agg_coverage(&legacy).is_empty());

        // 拒绝信封完成（policy_denial）自身即评估证据 → 无需两字段。
        let denial = vec![
            funnel_run_started(),
            ev(
                "tool_completed",
                json!({
                    "tool": "grep",
                    "call_id": "call-3",
                    "exit_code": 1,
                    "status": "error",
                    "error": "session_volume_notice",
                    "policy_denial": {"source": "permission", "code": "session_volume_notice", "reason": "r"},
                }),
            ),
        ];
        assert!(verify_failure_agg_coverage(&denial).is_empty());
    }

    #[test]
    fn failure_agg_coverage_tamper_and_xor_negatives() {
        // 篡改负测（S3 正反两测的「删聚合行」journal 面）：post-funnel 刊
        // 上剥掉 failure_target（模拟漏盖）→ 法官报错。
        let mut tampered = stamped_error(
            "read_file",
            json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
        );
        tampered["payload"]
            .as_object_mut()
            .unwrap()
            .remove("failure_target");
        let journal = vec![funnel_run_started(), tampered];
        let errors = verify_failure_agg_coverage(&journal);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missed failure-aggregation stamp")),
            "{errors:?}"
        );

        // XOR 反测：两字段并存 = 漏斗双写。
        let both = vec![
            funnel_run_started(),
            ev(
                "tool_completed",
                json!({
                    "tool": "read_file",
                    "call_id": "call-4",
                    "exit_code": 1,
                    "status": "error",
                    "error": "boom",
                    "failure_target": json!({"kind": "file_target", "id": hex64(1), "path": "a.py"}),
                    "failure_agg_absent": true,
                }),
            ),
        ];
        let errors = verify_failure_agg_coverage(&both);
        assert!(
            errors.iter().any(|e| e.contains("mutually exclusive")),
            "{errors:?}"
        );

        // 误用反测：marker 出现在非 error 形状（成功完成）。
        let misuse = vec![
            funnel_run_started(),
            ev(
                "tool_completed",
                json!({"tool": "read_file", "call_id": "call-5", "exit_code": 0, "failure_agg_absent": true}),
            ),
        ];
        let errors = verify_failure_agg_coverage(&misuse);
        assert!(
            errors.iter().any(|e| e.contains("non-error completion")),
            "{errors:?}"
        );

        // 未知管线版本 → 报错（版本锚封闭词表执法）。
        let unknown = vec![
            ev(
                "run_started",
                json!({"prompt": "p", "failure_pipeline": "funnel-v9"}),
            ),
            stamped_error(
                "read_file",
                json!({"kind": "file_target", "id": hex64(2), "path": "a.py"}),
            ),
        ];
        let errors = verify_failure_agg_coverage(&unknown);
        assert!(errors.iter().any(|e| e.contains("funnel-v9")), "{errors:?}");

        // Ok 臂命令级失败（exit≠0、无 status）同样是 error 形状：剥掉
        // 身份字段后必须报错——F-C 漏盖类漏洞的机械证明。
        let mut ok_arm = ev(
            "tool_completed",
            json!({
                "tool": "run_terminal_cmd",
                "call_id": "call-6",
                "exit_code": 7,
                "failure_target": json!({"kind": "cmd_target", "id": hex64(3), "cmd_preview": "make"}),
            }),
        );
        ok_arm["payload"]
            .as_object_mut()
            .unwrap()
            .remove("failure_target");
        let journal = vec![funnel_run_started(), ok_arm];
        let errors = verify_failure_agg_coverage(&journal);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("missed failure-aggregation stamp")),
            "{errors:?}"
        );
    }

    /// 0q 新族专属 Rust↔Python 对拍（0p 补测同款：不进 pinned corpus）——
    /// 正测 / grandfather / 篡改 / XOR / 误用 / 未知版本六场景 verdict
    /// parity。
    #[test]
    fn failure_agg_coverage_verdicts_match_python() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let assurance_dir = repo_root.join("assurance");
        assert!(
            assurance_dir
                .join("run_event_journal_validation.py")
                .is_file(),
            "0q crosscheck must run inside the parent repository"
        );

        let mut tampered = stamped_error(
            "read_file",
            json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
        );
        tampered["payload"]
            .as_object_mut()
            .unwrap()
            .remove("failure_target");
        let scenarios: Vec<(String, Vec<Value>)> = vec![
            (
                "positive".into(),
                vec![
                    funnel_run_started(),
                    stamped_error(
                        "read_file",
                        json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
                    ),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "grep",
                            "call_id": "call-2",
                            "exit_code": 1,
                            "status": "error",
                            "error": "boom",
                            "failure_agg_absent": true,
                        }),
                    ),
                ],
            ),
            (
                "grandfather".into(),
                vec![stamped_error(
                    "read_file",
                    json!({"kind": "file_target", "id": hex64(0), "path": "a.py"}),
                )],
            ),
            ("missed_stamp".into(), vec![funnel_run_started(), tampered]),
            (
                "double_write".into(),
                vec![
                    funnel_run_started(),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file",
                            "call_id": "call-3",
                            "exit_code": 1,
                            "status": "error",
                            "error": "boom",
                            "failure_target": json!({"kind": "file_target", "id": hex64(1), "path": "a.py"}),
                            "failure_agg_absent": true,
                        }),
                    ),
                ],
            ),
            (
                "marker_misuse".into(),
                vec![
                    funnel_run_started(),
                    ev(
                        "tool_completed",
                        json!({
                            "tool": "read_file", "call_id": "call-4", "exit_code": 0,
                            "failure_agg_absent": true,
                        }),
                    ),
                ],
            ),
            (
                "unknown_version".into(),
                vec![
                    ev(
                        "run_started",
                        json!({"prompt": "p", "failure_pipeline": "funnel-v9"}),
                    ),
                    stamped_error(
                        "read_file",
                        json!({"kind": "file_target", "id": hex64(2), "path": "a.py"}),
                    ),
                ],
            ),
        ];
        let corpus_json = serde_json::to_string(
            &scenarios
                .iter()
                .map(|(name, events)| json!({"name": name, "events": events}))
                .collect::<Vec<_>>(),
        )
        .unwrap();

        let script = r#"
import sys, json
sys.path.insert(0, sys.argv[1])
import run_event_journal_validation as v
data = json.load(sys.stdin)
out = {}
for sc in data:
    out[sc["name"]] = bool(v._verify_v02_failure_agg_coverage(sc["events"]))
json.dump(out, sys.stdout)
"#;
        use std::io::Write as _;
        use std::process::{Command, Stdio};
        let python = std::env::var("ORZ_PYTHON").unwrap_or_else(|_| "python".into());
        let mut child = Command::new(&python)
            .arg("-X")
            .arg("utf8")
            .arg("-c")
            .arg(script)
            .arg(assurance_dir.display().to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn python for the 0q failure_agg_coverage crosscheck");
        child
            .stdin
            .take()
            .expect("python stdin")
            .write_all(corpus_json.as_bytes())
            .expect("write corpus");
        let output = child.wait_with_output().expect("python crosscheck");
        assert!(
            output.status.success(),
            "python crosscheck failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let py_verdicts: BTreeMap<String, bool> =
            serde_json::from_slice(&output.stdout).expect("python verdict JSON");
        for (name, events) in &scenarios {
            let rust_violation = !verify_failure_agg_coverage(events).is_empty();
            let py_violation = *py_verdicts
                .get(name)
                .unwrap_or_else(|| panic!("python missing scenario {name}"));
            assert_eq!(
                rust_violation, py_violation,
                "verdict mismatch on {name} (rust={rust_violation}, python={py_violation})"
            );
        }
    }
}
