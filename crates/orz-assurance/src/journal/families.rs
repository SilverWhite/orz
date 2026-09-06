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

fn is_v02(event: &Value) -> bool {
    event.get("payload_schema").and_then(Value::as_str) == Some(V02_PAYLOAD_SCHEMA)
}

/// Python `isinstance(x, int)` for JSON values: integers and booleans count
/// (bool is an int in Python), floats do not.
fn py_int(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n.as_i64(),
        Some(Value::Bool(b)) => Some(*b as i64),
        _ => None,
    }
}

fn str_of(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str)
}

/// Tool-family constants mirroring the Python module tables.
mod toolsets {
    pub const WORK_TOOLS: &[&str] = &[
        "read_file",
        "list_dir",
        "grep",
        "search_tool",
        "search_replace",
        "run_tests",
        "ask_user_question",
        "blackboard_read",
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
    pub const RETRIEVAL_MODE_GATED_TOOLS: &[&str] =
        &["project_doc_index", "browser_read", "pdf_read"];
    pub const ACAF_TICKETED_TOOLS: &[&str] = &[
        "search_replace",
        "run_tests",
        "run_terminal_cmd",
        "web_fetch",
        "browser_read",
    ];
    pub const RETRIEVAL_TARGETS: &[&str] = &["internal_retrieval", "external_retrieval"];
    pub const HOST_LANE_RETRIEVAL_TOOLS: &[&str] = &["browser_read"];
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
    // is an EXACT name set (26 entries, Py 2419) — NOT the prefix predicate
    // `_is_retrieval_mode_gated_tool` (which additionally admits web_search/
    // web_fetch/retrieve_project_*). Using the predicate here would let
    // source=permission through on web-family tools where Python errors
    // (S2b review P1, 2026-09-06).
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

/// Run one named S2b family over a parsed journal; unknown names yield an
/// empty list (the caller validates the name).
pub fn verify_family(family: &str, events: &[Value]) -> Vec<String> {
    match family {
        "ledger_fold_advance" => verify_ledger_fold_advance(events),
        "ledger_fold_write_failed" => verify_ledger_fold_write_failed(events),
        "retrieval_mode" => verify_retrieval_mode(events),
        "policy_denial" => verify_policy_denial(events),
        "failure_target" => verify_failure_target(events),
        "control_tickets" => verify_control_tickets(events),
        "lifecycle" => verify_lifecycle(events),
        _ => Vec::new(),
    }
}

/// Run every S2b family (Python call order) over a parsed journal.
pub fn verify_all_families(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for family in S2B_FAMILIES {
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

    /// Shared scenario corpus: (name, events). Used by the table-driven unit
    /// test below and by the Python crosscheck (same corpus, both judges).
    fn scenarios() -> Vec<(&'static str, Vec<Value>)> {
        vec![
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
                    ev("retrieval_result_committed", json!({"activation_id": "a1"})),
                ],
            ),
            (
                "retrieval_mode_lb_unavailable",
                vec![
                    ev(
                        "retrieval_mode_transition",
                        json!({"old_mode": "off", "new_mode": "local_browser", "authority": "mechanical_probe", "capability_status": "degraded"}),
                    ),
                    ev("retrieval_result_committed", json!({"activation_id": "a1"})),
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
        ]
    }

    /// Expected verdicts per scenario per family (the S2b spec table).
    /// `true` = the family must report at least one error.
    fn expected_violations() -> Vec<(&'static str, Vec<(&'static str, bool)>)> {
        let ok: Vec<(&'static str, bool)> =
            S2B_FAMILIES.iter().map(|family| (*family, false)).collect();
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
        expect("lifecycle_close_replayed_outcome_errors", "lifecycle");
        rows
    }

    #[test]
    fn family_verdicts_match_spec_table() {
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

    /// S2b acceptance: per-family verdict parity with the Python judge on the
    /// same corpus — the synthetic scenarios above PLUS every real v0.2
    /// fixture journal. Verdict parity = (errors empty) agrees on both sides;
    /// message text is deliberately Rust-form.
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

        // Python side: run the seven `_verify_v02_*` functions per corpus item.
        let script = r#"
import sys, json
sys.path.insert(0, sys.argv[1])
import run_event_journal_validation as v
fams = {
    "control_tickets": v._verify_v02_control_tickets,
    "retrieval_mode": v._verify_v02_retrieval_mode,
    "ledger_fold_advance": v._verify_v02_ledger_fold_advance,
    "ledger_fold_write_failed": v._verify_v02_ledger_fold_write_failed,
    "policy_denial": v._verify_v02_policy_denial,
    "failure_target": v._verify_v02_failure_target,
    "lifecycle": v._verify_v02_lifecycle,
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
            for family in S2B_FAMILIES {
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
            S2B_FAMILIES.len() * corpus.len(),
            "crosscheck cell accounting drifted"
        );
        assert!(
            scenario_count >= 58,
            "synthetic scenario corpus shrunk below its registered floor \
             ({scenario_count})"
        );
        assert!(
            fixture_names.len() >= 18,
            "real fixture journals shrunk below their registered floor ({})",
            fixture_names.len()
        );
    }
}
