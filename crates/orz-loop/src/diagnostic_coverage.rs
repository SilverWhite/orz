//! Diagnostic Coverage (ADR-0010 §4.6): progressive 2→3→4→5 hard-signal
//! thresholds for a debug episode, deduplicated by evidence identity.
//!
//! Consumption is the ONLY production mutation point (§4.6.2 — a journal
//! replay never recounts). `maybe_fire_dc` builds the mechanical
//! checkpoint payload and journals the fire; the forced-template round's
//! completion commits the stage via `commit_dc_fire` (§2.4 — only a
//! completed template round advances the stage / resets the count).

use std::collections::HashSet;
use std::sync::Mutex;

use orz_assurance::EventType;
use orz_assurance::journal::sha256_hex;

use crate::controller::{AgentLoopError, EventWriter};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::ToolResult;

/// Mechanical injected-text prefix of the DC checkpoint block (registered
/// with the injected-block/restore filters — never model output).
pub const DIAGNOSTIC_COVERAGE_PREFIX: &str = "[DIAGNOSTIC_COVERAGE";

/// One pending hard signal, bound to its evidence identity (§4.6.2).
#[derive(Debug, Clone)]
pub struct SignalRecord {
    pub signal_type: &'static str,
    pub evidence_identity: String,
}

/// Session-level debug-episode state (main lane only; `profile.dc_enabled`).
#[derive(Debug)]
pub struct DebugEpisodeState {
    /// Pending hard signals, deduplicated by (type, evidence identity).
    pub pending_signals: Vec<SignalRecord>,
    /// Total pending signal count (== `pending_signals.len()`).
    pub signal_count: usize,
    /// Examined surfaces — read/edited file paths and committed retrieval
    /// source ids (fed into the forced-template evidence cross-check and
    /// the key-surface signal).
    pub evidence_ids: HashSet<String>,
    /// Current threshold stage: 2 → 3 → 4 → 5 (ADR-0010 §4.6).
    pub threshold_stage: u32,
    /// Number of completed fires (0-based value carried by the next fire's
    /// `trigger_count`).
    pub trigger_count: u32,
    /// Stable debug-episode identity for checkpoint ids.
    pub debug_episode_id: String,
    /// A fired-but-not-yet-committed checkpoint payload — blocks a second
    /// fire until the forced-template round commits (§2.4).
    pub pending_fire: Option<serde_json::Value>,
    /// Failure fingerprints seen in the current episode — the
    /// `consecutive_same_failure` repeat basis (cleared on a passing suite,
    /// a mechanically-verifiable resolution).
    pub seen_fingerprints: HashSet<String>,
    /// Last edited module (parent directory), for same_module_no_evidence.
    last_edited_module: Option<String>,
    /// Whether a read covered the last edited module since the edit.
    read_since_last_edit: bool,
}

impl Default for DebugEpisodeState {
    fn default() -> Self {
        Self {
            pending_signals: Vec::new(),
            signal_count: 0,
            evidence_ids: HashSet::new(),
            threshold_stage: 2,
            trigger_count: 0,
            debug_episode_id: "BUG-MAIN".to_string(),
            pending_fire: None,
            seen_fingerprints: HashSet::new(),
            last_edited_module: None,
            read_since_last_edit: false,
        }
    }
}

fn push_signal(state: &mut DebugEpisodeState, signal_type: &'static str, identity: String) {
    if state
        .pending_signals
        .iter()
        .any(|s| s.signal_type == signal_type && s.evidence_identity == identity)
    {
        return;
    }
    state.pending_signals.push(SignalRecord {
        signal_type,
        evidence_identity: identity,
    });
    state.signal_count = state.pending_signals.len();
}

/// First-line failure class: `Traceback ...` or `FAILED ...` (mechanical,
/// stable — see `extract_file_paths`).
fn extract_error_class(output: &str) -> Option<String> {
    let first = output.lines().next().unwrap_or("").trim();
    if first.starts_with("Traceback") || first.starts_with("FAILED") {
        Some(first.to_string())
    } else {
        None
    }
}

/// File paths referenced by a failure output: `File "path"` lines and the
/// `FAILED <path>::...` pytest line.
fn extract_file_paths(output: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in output.lines() {
        if let Some(start) = line.find("File \"") {
            let rest = &line[start + 6..];
            if let Some(end) = rest.find('"') {
                paths.push(rest[..end].to_string());
            }
        } else if let Some(rest) = line.strip_prefix("FAILED ") {
            // Only pytest-style `FAILED <path>::...` (or a path with
            // separators) is a file reference — a bare error line like
            // "FAILED test_a" is an error class, not a surface.
            if rest.contains("::") || rest.contains('/') || rest.contains('\\') {
                let path = rest.split("::").next().unwrap_or(rest).trim();
                if !path.is_empty() {
                    paths.push(path.to_string());
                }
            }
        }
    }
    paths
}

fn module_of(path: &str) -> String {
    std::path::Path::new(path)
        .parent()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

/// M5 (2026-08-10): the ONLY production consumption point (§4.6.2).
/// Consumes one completed tool call into the debug-episode state:
///
/// - failing `run_tests`: failure-fingerprint signal (per call),
///   error-class signal (deduped), `key_surface_unexamined` per referenced
///   path not yet examined, and `consecutive_same_failure` when the same
///   fingerprint repeats;
/// - passing `run_tests`: mechanically-verifiable resolution — stage
///   resets to 2 and pending signals clear;
/// - `read_file`: marks the path examined (and clears the same-module
///   condition);
/// - `search_replace`: tracks the edited module — a second edit in the same
///   module with no intervening read raises `same_module_no_evidence`.
pub(crate) async fn maybe_consume_dc_signal(
    state: &Mutex<DebugEpisodeState>,
    _writer: &mut EventWriter<'_>,
    tc: &ToolCall,
    result: &ToolResult,
) -> Result<(), AgentLoopError> {
    let mut s = state.lock().unwrap();
    match tc.name.as_str() {
        "read_file" => {
            if let Some(path) = tc.arguments.get("path").and_then(|v| v.as_str()) {
                s.evidence_ids.insert(path.to_string());
                if s.last_edited_module.as_deref() == Some(module_of(path).as_str()) {
                    s.read_since_last_edit = true;
                }
            }
        }
        "search_replace" => {
            if let Some(path) = tc.arguments.get("file_path").and_then(|v| v.as_str()) {
                s.evidence_ids.insert(path.to_string());
                let module = module_of(path);
                if s.last_edited_module.as_deref() == Some(module.as_str())
                    && !s.read_since_last_edit
                {
                    push_signal(
                        &mut s,
                        "same_module_no_evidence",
                        format!("module:{module}"),
                    );
                }
                s.last_edited_module = Some(module);
                s.read_since_last_edit = false;
            }
        }
        "run_tests" => {
            if result.exit_code == Some(0) {
                // Passing suite = mechanically-verifiable resolution.
                s.threshold_stage = 2;
                s.pending_signals.clear();
                s.signal_count = 0;
                s.pending_fire = None;
                s.seen_fingerprints.clear();
            } else {
                let fingerprint = sha256_hex(result.output.as_bytes());
                let repeated = format!("call:{}:{fingerprint}", tc.call_id);
                push_signal(&mut s, "repeated_pattern", repeated);
                if !s.seen_fingerprints.insert(fingerprint.clone()) {
                    push_signal(
                        &mut s,
                        "consecutive_same_failure",
                        format!("consecutive:{fingerprint}"),
                    );
                }
                if let Some(class) = extract_error_class(&result.output) {
                    push_signal(
                        &mut s,
                        "unabsorbed_new_evidence",
                        format!("error_class:{class}"),
                    );
                }
                for path in extract_file_paths(&result.output) {
                    if !s.evidence_ids.contains(&path) {
                        push_signal(&mut s, "key_surface_unexamined", format!("surface:{path}"));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): committed retrieval sources count as
/// examined surfaces for the key-surface signal.
pub(crate) async fn maybe_consume_dc_retrieval_evidence(
    state: &Mutex<DebugEpisodeState>,
    committed: &crate::controller::StructuredCommittedResult,
) -> Result<(), AgentLoopError> {
    let mut s = state.lock().unwrap();
    if let Some(ledger) = committed
        .payload
        .get("source_ledger")
        .and_then(|v| v.as_array())
    {
        for entry in ledger {
            if let Some(id) = entry.get("source_id").and_then(|v| v.as_str()) {
                s.evidence_ids.insert(id.to_string());
            }
        }
    }
    Ok(())
}

/// M5 (2026-08-10): fire a mechanical diagnostic-coverage checkpoint when
/// the pending signal count reaches the current threshold stage. The fire
/// journals the event, injects the coverage block and marks the fire
/// pending; `commit_dc_fire` advances the stage after the forced-template
/// round completes (§2.4). Returns `None` when not due or a fire is already
/// pending (one checkpoint round at a time).
pub(crate) async fn maybe_fire_dc(
    state: &Mutex<DebugEpisodeState>,
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
) -> Result<Option<serde_json::Value>, AgentLoopError> {
    let payload = {
        let s = state.lock().unwrap();
        if s.pending_fire.is_some() {
            return Ok(None);
        }
        let count = s.pending_signals.len() as u64;
        if (count as u32) < s.threshold_stage {
            return Ok(None);
        }
        let stage = s.threshold_stage;
        let trigger = s.trigger_count;
        let episode = s.debug_episode_id.clone();
        let signals: Vec<serde_json::Value> = s
            .pending_signals
            .iter()
            .enumerate()
            .map(|(i, sig)| {
                serde_json::json!({
                    "signal_id": format!("SIG-{:04}", i + 1),
                    "signal_type": sig.signal_type,
                    "evidence_identity": sig.evidence_identity,
                })
            })
            .collect();
        let mut covered: Vec<String> = s.evidence_ids.iter().cloned().collect();
        covered.sort();
        let mut missing: Vec<String> = s
            .pending_signals
            .iter()
            .filter(|sig| sig.signal_type == "key_surface_unexamined")
            .map(|sig| {
                sig.evidence_identity
                    .trim_start_matches("surface:")
                    .to_string()
            })
            .collect();
        missing.sort();
        missing.dedup();
        let action = if missing.is_empty() {
            "运行最小复现并采集 trace".to_string()
        } else {
            format!("读取缺失表面：{}", missing.join("、"))
        };
        let covered_text = if covered.is_empty() {
            "无".to_string()
        } else {
            covered.join("、")
        };
        let missing_text = if missing.is_empty() {
            "无".to_string()
        } else {
            missing.join("、")
        };
        let message_block = format!(
            "[DIAGNOSTIC_COVERAGE v0.2] 已覆盖：{}；缺失：{}；最小补诊断动作：{}",
            covered_text, missing_text, action
        );
        serde_json::json!({
            "checkpoint_id": format!("DIAG-COV-{}-{}", episode, trigger + 1),
            "inquiry_family": "neutral",
            "inquiry_kind": "diagnostic_coverage_checkpoint",
            "agent_role": "main",
            "debug_episode_id": episode,
            "threshold_stage": stage,
            "hard_signal_count": count,
            "trigger_count": trigger,
            "signals": signals,
            "covered_surfaces": covered,
            "missing_surfaces": missing,
            "message_block": message_block,
            "minimal_next_diagnostic_action": action,
        })
    };
    writer
        .record(EventType::DiagnosticCoverageCheckpoint, payload.clone())
        .await?;
    messages.push(Message {
        role: Role::User,
        content: payload["message_block"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    state.lock().unwrap().pending_fire = Some(payload.clone());
    Ok(Some(payload))
}

/// Commit a completed DC fire (accepted or degraded template round — §2.4):
/// clear the pending fire, advance the threshold stage (2→3→4→5), clear the
/// signal batch and increment the trigger count.
pub(crate) fn commit_dc_fire(state: &Mutex<DebugEpisodeState>, _payload: &serde_json::Value) {
    let mut s = state.lock().unwrap();
    if s.pending_fire.is_none() {
        return;
    }
    s.pending_fire = None;
    s.threshold_stage = (s.threshold_stage + 1).min(5);
    s.trigger_count += 1;
    s.pending_signals.clear();
    s.signal_count = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(output: &str, exit_code: Option<i32>) -> ToolResult {
        ToolResult {
            output: output.to_string(),
            exit_code,
            output_encoding: None,
            structured: None,
            ..Default::default()
        }
    }

    fn tc(name: &str, call_id: &str) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            arguments: serde_json::json!({}),
            call_id: call_id.to_string(),
        }
    }

    #[test]
    fn error_class_extraction() {
        assert_eq!(
            extract_error_class("Traceback (most recent call last):\n  File x"),
            Some("Traceback (most recent call last):".to_string())
        );
        assert_eq!(
            extract_error_class("FAILED tests/test_x.py::test_y"),
            Some("FAILED tests/test_x.py::test_y".to_string())
        );
        assert_eq!(extract_error_class("all green"), None);
        assert_eq!(extract_error_class(""), None);
    }

    #[tokio::test]
    async fn consume_dedupes_by_evidence_identity() {
        // The same call (same call_id + same failure) consumed twice: the
        // repeated_pattern evidence dedupes (counted once); the second
        // consumption adds only the consecutive_same_failure signal — a
        // DIFFERENT type with its own identity (§4.6.2 binds signals to
        // event/evidence identity).
        let dc = Mutex::new(DebugEpisodeState::default());
        let tc = tc("run_tests", "call-1");
        let res = result("FAILED test_a", Some(1));
        // No journal in a unit test — a discard writer.
        let mut writer = crate::controller::discard_event_writer("RUN-DC-UNIT");
        maybe_consume_dc_signal(&dc, &mut writer, &tc, &res)
            .await
            .unwrap();
        maybe_consume_dc_signal(&dc, &mut writer, &tc, &res)
            .await
            .unwrap();
        let s = dc.lock().unwrap();
        let types: Vec<&str> = s.pending_signals.iter().map(|x| x.signal_type).collect();
        assert_eq!(
            types.iter().filter(|t| **t == "repeated_pattern").count(),
            1,
            "repeated_pattern deduped: {types:?}"
        );
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == "unabsorbed_new_evidence")
                .count(),
            1,
            "error-class signal deduped: {types:?}"
        );
        assert_eq!(
            types
                .iter()
                .filter(|t| **t == "consecutive_same_failure")
                .count(),
            1,
            "{types:?}"
        );
        assert_eq!(s.signal_count, 3); // 2 first + 1 consecutive on re-consume
    }

    #[tokio::test]
    async fn consecutive_same_failure_detected() {
        // Two identical failures → the second is consecutive_same_failure.
        let dc = Mutex::new(DebugEpisodeState::default());
        let mut writer = crate::controller::discard_event_writer("RUN-DC-UNIT");
        let res = result("FAILED test_a", Some(1));
        maybe_consume_dc_signal(&dc, &mut writer, &tc("run_tests", "call-1"), &res)
            .await
            .unwrap();
        maybe_consume_dc_signal(&dc, &mut writer, &tc("run_tests", "call-2"), &res)
            .await
            .unwrap();
        let s = dc.lock().unwrap();
        let types: Vec<&str> = s.pending_signals.iter().map(|x| x.signal_type).collect();
        assert!(types.contains(&"repeated_pattern"));
        assert!(types.contains(&"consecutive_same_failure"));
        // Per call: the failure signal + its error-class signal.
        assert_eq!(s.signal_count, 4);
    }

    // ── GAP-RETRIEVAL-TOOLS (2026-08-10): same_module_no_evidence +
    //    key_surface_unexamined ──

    #[test]
    fn file_path_extraction() {
        assert_eq!(
            extract_file_paths("Traceback:\n  File \"src/a.rs\", line 3"),
            vec!["src/a.rs"]
        );
        assert_eq!(
            extract_file_paths("FAILED tests/test_x.py::test_y"),
            vec!["tests/test_x.py"]
        );
        assert_eq!(extract_file_paths("all green"), Vec::<String>::new());
    }

    #[tokio::test]
    async fn key_surface_unexamined_fires_on_unread_failure_reference() {
        let dc = Mutex::new(DebugEpisodeState::default());
        let mut writer = crate::controller::discard_event_writer("RUN-DC-UNIT");
        // A failing run referencing a file never read nor edited.
        let mut run = tc("run_tests", "call-1");
        maybe_consume_dc_signal(
            &dc,
            &mut writer,
            &run,
            &result("FAILED tests/test_x.py::test_y", Some(1)),
        )
        .await
        .unwrap();
        {
            let s = dc.lock().unwrap();
            assert!(
                s.pending_signals
                    .iter()
                    .any(|x| x.signal_type == "key_surface_unexamined")
            );
        }

        // After a read_file of the referenced path, the signal is NOT
        // produced again (examined surface).
        let read = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "tests/test_x.py"}),
            call_id: "call-r".to_string(),
        };
        maybe_consume_dc_signal(&dc, &mut writer, &read, &result("content", Some(0)))
            .await
            .unwrap();
        run.call_id = "call-2".to_string();
        maybe_consume_dc_signal(
            &dc,
            &mut writer,
            &run,
            &result("FAILED tests/test_x.py::test_y", Some(1)),
        )
        .await
        .unwrap();
        let s = dc.lock().unwrap();
        // The examined surface suppresses the signal — still exactly the
        // call-1 signal in the pending list, no new one.
        assert_eq!(
            s.pending_signals
                .iter()
                .filter(|x| x.signal_type == "key_surface_unexamined")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn same_module_no_evidence_fires_on_repeat_edit_without_read() {
        let dc = Mutex::new(DebugEpisodeState::default());
        let mut writer = crate::controller::discard_event_writer("RUN-DC-UNIT");
        let edit = |call_id: &str| ToolCall {
            name: "search_replace".to_string(),
            arguments: serde_json::json!({"file_path": "src/impl.rs"}),
            call_id: call_id.to_string(),
        };
        // First edit in src/ — new module, no signal.
        maybe_consume_dc_signal(&dc, &mut writer, &edit("call-e1"), &result("ok", Some(0)))
            .await
            .unwrap();
        // Second edit in the SAME module with no read evidence → signal.
        maybe_consume_dc_signal(&dc, &mut writer, &edit("call-e2"), &result("ok", Some(0)))
            .await
            .unwrap();
        {
            let s = dc.lock().unwrap();
            assert!(
                s.pending_signals
                    .iter()
                    .any(|x| x.signal_type == "same_module_no_evidence"),
                "{:?}",
                s.pending_signals
            );
        }

        // A read in the module clears the condition for the next edit.
        let read = ToolCall {
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "src/impl.rs"}),
            call_id: "call-r".to_string(),
        };
        maybe_consume_dc_signal(&dc, &mut writer, &read, &result("content", Some(0)))
            .await
            .unwrap();
        maybe_consume_dc_signal(&dc, &mut writer, &edit("call-e3"), &result("ok", Some(0)))
            .await
            .unwrap();
        let s = dc.lock().unwrap();
        // The module read clears the condition — still exactly the e2
        // signal in the pending list, no new one.
        assert_eq!(
            s.pending_signals
                .iter()
                .filter(|x| x.signal_type == "same_module_no_evidence")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn fire_commit_advances_stage_and_allows_next_fire() {
        let dc = Mutex::new(DebugEpisodeState::default());
        let mut writer = crate::controller::discard_event_writer("RUN-DC-UNIT");
        let mut messages: Vec<Message> = Vec::new();
        let res = result("FAILED tests/test_x.py::test_y", Some(1));
        maybe_consume_dc_signal(&dc, &mut writer, &tc("run_tests", "call-1"), &res)
            .await
            .unwrap();
        let p1 = maybe_fire_dc(&dc, &mut writer, &mut messages)
            .await
            .unwrap()
            .expect("first fire");
        assert_eq!(p1["threshold_stage"], serde_json::json!(2));
        assert_eq!(p1["trigger_count"], serde_json::json!(0));
        // A pending fire blocks re-firing until the template round commits.
        assert!(
            maybe_fire_dc(&dc, &mut writer, &mut messages)
                .await
                .unwrap()
                .is_none(),
            "pending fire must block a second fire"
        );
        commit_dc_fire(&dc, &p1);
        assert_eq!(dc.lock().unwrap().threshold_stage, 3);
        assert_eq!(dc.lock().unwrap().trigger_count, 1);
        maybe_consume_dc_signal(&dc, &mut writer, &tc("run_tests", "call-2"), &res)
            .await
            .unwrap();
        let p2 = maybe_fire_dc(&dc, &mut writer, &mut messages)
            .await
            .unwrap()
            .expect("second fire");
        assert_eq!(p2["threshold_stage"], serde_json::json!(3));
        assert_eq!(p2["trigger_count"], serde_json::json!(1));
    }
}
