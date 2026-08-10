//! Diagnostic Coverage Check producer (ADR-0010 §4.6; GAP-SUBAGENT-RUNTIME
//! M5, 2026-08-10).
//!
//! A debug/problem-solving-specific, progressive neutral inquiry on the
//! MAIN lane: the controller consumes only STRUCTURED hard signals from
//! tool results (non-zero test results, repeated failure fingerprints, new
//! error classes, mutation-scope expansion without improvement), dedupes by
//! event/evidence identity (a journal replay never recounts — signals are
//! produced once at the in-process consumption point), and injects exactly
//! one neutral checkpoint per threshold stage (2 → 3 → 4 → 5, capped at 5).
//! A passing `run_tests` (bug resolved, mechanically verifiable) resets the
//! threshold to 2. The checkpoint is not a hard gate — the block is a
//! neutral message and the loop continues.
//!
//! Episode scope: one episode per run (`DC-{run_id}` — the run's journal
//! chain; ACP builds a controller per prompt, so an episode never spans
//! prompts — registered boundary 2026-08-10).

use std::collections::HashSet;
use std::sync::Mutex;

use orz_assurance::{EventType, canonical_json, sha256_hex};

use crate::controller::{AgentLoopError, EventWriter};
use crate::gateway::model::{Message, Role, ToolCall};
use crate::host::ToolResult;
use crate::tool::ToolDispatcher;

/// Neutral checkpoint prefix — registered in `is_injected_block_text`
/// (stagnation guard exclusion; a repeated block must never pollute the
/// ngram stats).
pub const DIAGNOSTIC_COVERAGE_PREFIX: &str = "[DIAGNOSTIC_COVERAGE";

/// §4.6.4 — one fixed minimal diagnostic action (mechanical; the checkpoint
/// lists covered/missing surfaces + this single suggestion).
pub const MINIMAL_NEXT_DIAGNOSTIC_ACTION: &str =
    "运行一次最小复现并读取完整失败输出（或按新 error class 检查首个复现点）";

/// The v0.2 payload `signal_type` enum (schema authority — never free text).
pub const SIGNAL_TYPES: [&str; 6] = [
    "consecutive_same_failure",
    "repeated_pattern",
    "same_module_no_evidence",
    "unabsorbed_new_evidence",
    "large_scope_low_diag",
    "key_surface_unexamined",
];

/// One consumed hard signal (the checkpoint payload's `signals[]` item).
#[derive(Debug, Clone)]
pub(crate) struct DcSignal {
    pub signal_type: &'static str,
    pub evidence_identity: String,
}

/// Per-run debug-episode state (ADR-0010 §4.6; controller-owned — the loop
/// consumes through it; journal replay never recounts because signals are
/// produced exactly once at the in-process consumption point).
#[derive(Debug)]
pub(crate) struct DebugEpisodeState {
    pub episode_id: Option<String>,
    /// 2 → 3 → 4 → 5, capped at 5 (§4.6.1).
    pub threshold: u32,
    /// Hard-signal count for the CURRENT stage (zeroed on fire — the
    /// zeroing IS the once-per-stage guarantee, §4.6.4).
    pub signal_count: u32,
    pub trigger_count: u32,
    /// Dedupe by (signal_type, evidence_identity) — episode-wide; a signal
    /// is bound to its event/evidence identity (§4.6.2).
    pub seen: HashSet<(String, String)>,
    /// Files edited this episode (mutation-scope expansion detection).
    pub edited_files: HashSet<String>,
    /// Fingerprint of the last failed run_tests (consecutive-same-failure
    /// detection; `Some` also means "tests currently failing" for the
    /// scope-expansion signal).
    pub last_fingerprint: Option<String>,
    /// Signals accumulated for the stage — drained into the checkpoint.
    pub pending_signals: Vec<DcSignal>,
    /// GAP-RETRIEVAL-TOOLS (2026-08-10): surfaces EXAMINED this episode —
    /// successfully read files (read_file) and committed retrieval source
    /// identities. `same_module_no_evidence` and `key_surface_unexamined`
    /// consume this set (§4.6.2: an unexamined key surface is a hard
    /// signal).
    pub evidence_ids: HashSet<String>,
    /// Modules (parent dirs) with at least one edit this episode —
    /// same-module repeat edits without new evidence are a hard signal.
    pub edited_modules: HashSet<String>,
    /// Modules that have at least one evidence read (read_file or
    /// retrieval evidence) — an edited module without evidence is the
    /// "same module, no evidence" condition.
    pub module_evidence: HashSet<String>,
}

impl Default for DebugEpisodeState {
    fn default() -> Self {
        Self {
            episode_id: None,
            threshold: 2,
            signal_count: 0,
            trigger_count: 0,
            seen: HashSet::new(),
            edited_files: HashSet::new(),
            last_fingerprint: None,
            pending_signals: Vec::new(),
            evidence_ids: HashSet::new(),
            edited_modules: HashSet::new(),
            module_evidence: HashSet::new(),
        }
    }
}

/// Normalize a file path for evidence identity (forward slashes).
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// The module (parent dir) of a path — "." for root-level files.
fn module_of(path: &str) -> String {
    match path.rfind('/') {
        Some(idx) if idx > 0 => path[..idx].to_string(),
        _ => ".".to_string(),
    }
}

/// Extract file paths referenced by a failing output — `File "..."` quotes,
/// pytest `FAILED tests/...::...` prefixes, and `at ...:line` stack frames.
/// The mechanical input to `key_surface_unexamined` (§4.6.2).
fn extract_file_paths(output: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        // Python traceback / compiler: File "path", line N
        if let Some(start) = t.find("File \"") {
            let rest = &t[start + 6..];
            if let Some(end) = rest.find('"') {
                let p = normalize_path(&rest[..end]);
                if !p.is_empty() && !paths.contains(&p) {
                    paths.push(p);
                }
                continue;
            }
        }
        // pytest: FAILED tests/test_x.py::test_y
        if let Some(rest) = t.strip_prefix("FAILED ") {
            let p = normalize_path(rest.split("::").next().unwrap_or(rest));
            if (p.contains('/') || p.contains('\\')) && !paths.contains(&p) {
                paths.push(p);
            }
        }
    }
    paths
}

/// Consume the tool result's hard signals (§4.6.2 — the ONLY production
/// point; a journal replay reads the file and never re-runs this).
/// Main lane only (`profile.dc_enabled`). Called from the shared loop's
/// dispatch after a host tool executes.
pub(crate) async fn maybe_consume_dc_signal(
    dc: &Mutex<DebugEpisodeState>,
    writer: &mut EventWriter<'_>,
    tc: &ToolCall,
    result: &ToolResult,
) -> Result<(), AgentLoopError> {
    let mut s = dc.lock().unwrap();
    // Episode = run: a new run_id starts a fresh episode (registered
    // boundary: cross-prompt episodes are not persisted).
    let run_id = writer.run_id().to_string();
    let expected = format!("DC-{run_id}");
    if s.episode_id.as_deref() != Some(expected.as_str()) {
        *s = DebugEpisodeState::default();
        s.episode_id = Some(expected);
    }

    let error_class = extract_error_class(&result.output);
    let fingerprint = sha256_hex(
        &canonical_json(&serde_json::json!({
            "tool": tc.name,
            "exit_code": result.exit_code,
            "error_class": error_class,
            "output_tail": result.output.chars().take(256).collect::<String>(),
        }))
        .unwrap_or_default(),
    );

    // run_tests — the primary signal source (§4.6.2 first bullet).
    if tc.name == "run_tests" {
        if result.exit_code != Some(0) {
            let signal_type = if s.last_fingerprint.as_deref() == Some(&fingerprint) {
                "consecutive_same_failure"
            } else {
                "repeated_pattern"
            };
            let evidence = format!("{}:{}", tc.call_id, &fingerprint[..8]);
            if s.seen.insert((signal_type.to_string(), evidence.clone())) {
                s.signal_count += 1;
                s.pending_signals.push(DcSignal {
                    signal_type,
                    evidence_identity: evidence,
                });
            }
            s.last_fingerprint = Some(fingerprint.clone());
        } else {
            // §4.6.1: a passing test suite is mechanically-verifiable bug
            // resolution — threshold back to 2, and the stage restarts
            // fresh (residual pre-resolve signals must not re-trigger a
            // checkpoint against the reset threshold).
            s.threshold = 2;
            s.signal_count = 0;
            s.pending_signals.clear();
            s.last_fingerprint = None;
        }
    }

    // New error class / stack location (§4.6.2 third bullet) — any failing
    // output that reveals a class the episode has not yet seen.
    if let Some(cls) = error_class {
        let evidence = format!("{}:{cls}", tc.call_id);
        if s.seen.insert(("unabsorbed_new_evidence".to_string(), evidence.clone())) {
            s.signal_count += 1;
            s.pending_signals.push(DcSignal {
                signal_type: "unabsorbed_new_evidence",
                evidence_identity: evidence,
            });
        }
    }

    // GAP-RETRIEVAL-TOOLS (2026-08-10): read evidence — a successfully read
    // file is an examined surface (consumed by same_module_no_evidence /
    // key_surface_unexamined).
    if tc.name == "read_file"
        && result.exit_code == Some(0)
        && let Some(path) = tc.arguments.get("path").and_then(|v| v.as_str())
    {
        let key = normalize_path(path);
        s.evidence_ids.insert(key.clone());
        s.module_evidence.insert(module_of(&key));
    }

    // §4.6.2: the failing output references files the episode has neither
    // read nor edited — the key surface is unexamined.
    if result.exit_code != Some(0) {
        for path in extract_file_paths(&result.output) {
            if s.evidence_ids.contains(&path) || s.edited_files.contains(&path) {
                continue;
            }
            let evidence = format!("{}:{}", tc.call_id, &sha256_hex(path.as_bytes())[..8]);
            if s.seen.insert(("key_surface_unexamined".to_string(), evidence.clone())) {
                s.signal_count += 1;
                s.pending_signals.push(DcSignal {
                    signal_type: "key_surface_unexamined",
                    evidence_identity: evidence,
                });
            }
            // One signal per failing call.
            break;
        }
    }

    // Mutation-scope expansion without improvement (§4.6.2 fourth bullet):
    // a successful edit on a NEW file while tests are still failing.
    if ToolDispatcher::is_file_edit(&tc.name)
        && result.exit_code == Some(0)
        && s.last_fingerprint.is_some()
    {
        let file = tc
            .arguments
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !file.is_empty() && s.edited_files.insert(file) {
            let evidence = format!("{}:{}", tc.call_id, &fingerprint[..8]);
            if s.seen.insert(("large_scope_low_diag".to_string(), evidence.clone())) {
                s.signal_count += 1;
                s.pending_signals.push(DcSignal {
                    signal_type: "large_scope_low_diag",
                    evidence_identity: evidence,
                });
            }
        }
    }

    // GAP-RETRIEVAL-TOOLS (2026-08-10): same_module_no_evidence (§4.6.2) —
    // a successful edit in a module that ALREADY has edits while that
    // module has no evidence (read/retrieval) — repeated editing without
    // looking is the "locked into one route" signature.
    if ToolDispatcher::is_file_edit(&tc.name) && result.exit_code == Some(0) {
        let file = tc
            .arguments
            .get("file_path")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !file.is_empty() {
            let module = module_of(&normalize_path(&file));
            if s.edited_modules.contains(&module) && !s.module_evidence.contains(&module) {
                let evidence = format!("{}:{}", tc.call_id, &sha256_hex(module.as_bytes())[..8]);
                if s.seen.insert(("same_module_no_evidence".to_string(), evidence.clone())) {
                    s.signal_count += 1;
                    s.pending_signals.push(DcSignal {
                        signal_type: "same_module_no_evidence",
                        evidence_identity: evidence,
                    });
                }
            }
            s.edited_modules.insert(module);
        }
    }
    Ok(())
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): fold a committed retrieval result's
/// source identities into the DC evidence set — the retrieval lane's reads
/// count as examined surfaces (key_surface_unexamined consumes them; the
/// source_ids are the ledger-bound identities). Main lane only — called by
/// the controller after a successful subagent dispatch.
pub(crate) async fn maybe_consume_dc_retrieval_evidence(
    dc: &Mutex<DebugEpisodeState>,
    committed: &crate::controller::StructuredCommittedResult,
) -> Result<(), AgentLoopError> {
    let mut s = dc.lock().unwrap();
    if let Some(ledger) = committed.payload["source_ledger"].as_array() {
        for entry in ledger {
            if let Some(id) = entry["source_id"].as_str() {
                s.evidence_ids.insert(id.to_string());
            }
        }
    }
    Ok(())
}

/// Fire the checkpoint when the stage threshold is met (ADR-0010 §4.6.4:
/// exactly once per stage; a neutral message block + the v0.2
/// `diagnostic_coverage_checkpoint` event). Called at the loop's two
/// injection gaps (loop-top / post-tool-batch — the same safe gaps as
/// orientation). Not a hard gate — the loop continues after the injection.
pub(crate) async fn maybe_fire_dc(
    dc: &Mutex<DebugEpisodeState>,
    writer: &mut EventWriter<'_>,
    messages: &mut Vec<Message>,
) -> Result<(), AgentLoopError> {
    // Build the payload + commit the stage transition UNDER the guard (no
    // await inside — the std Mutex guard must never cross one); the
    // journal write and injection happen after.
    let (payload, message_block) = {
        let mut s = dc.lock().unwrap();
        if s.episode_id.is_none() || s.signal_count < s.threshold {
            return Ok(());
        }
        let run_id = writer.run_id().to_string();
        let episode_id = s.episode_id.clone().unwrap_or_else(|| format!("DC-{run_id}"));
        let threshold = s.threshold;
        let signal_count = s.signal_count;
        let trigger_count = s.trigger_count;
        let signals: Vec<serde_json::Value> = s
            .pending_signals
            .drain(..)
            .map(|sig| {
                serde_json::json!({
                    "signal_id": format!("SIG-{}-{}", sig.signal_type, &sha256_hex(sig.evidence_identity.as_bytes())[..8]),
                    "signal_type": sig.signal_type,
                    "evidence_identity": sig.evidence_identity,
                })
            })
            .collect();
        let covered: HashSet<&str> = s.seen.iter().map(|(t, _)| t.as_str()).collect();
        let covered_surfaces: Vec<&str> = SIGNAL_TYPES
            .iter()
            .copied()
            .filter(|t| covered.contains(t))
            .collect();
        let missing_surfaces: Vec<&str> = SIGNAL_TYPES
            .iter()
            .copied()
            .filter(|t| !covered.contains(t))
            .collect();
        let message_block = format!(
            "{DIAGNOSTIC_COVERAGE_PREFIX} v0.2] 已覆盖: {}；缺失: {}。\
             建议下一步: {MINIMAL_NEXT_DIAGNOSTIC_ACTION}。\
             这不是硬门禁——请基于证据继续当前方向。[/DIAGNOSTIC_COVERAGE]",
            if covered_surfaces.is_empty() {
                "无".to_string()
            } else {
                covered_surfaces.join(", ")
            },
            if missing_surfaces.is_empty() {
                "无".to_string()
            } else {
                missing_surfaces.join(", ")
            },
        );
        let payload = serde_json::json!({
            "checkpoint_id": format!("DIAG-COV-{run_id}-{trigger_count}"),
            "inquiry_family": "neutral",
            "inquiry_kind": "diagnostic_coverage_checkpoint",
            "debug_episode_id": episode_id,
            "threshold_stage": threshold,
            "hard_signal_count": signal_count,
            "trigger_count": trigger_count,
            "signals": signals,
            "covered_surfaces": covered_surfaces,
            "missing_surfaces": missing_surfaces,
            "message_block": message_block,
            "minimal_next_diagnostic_action": MINIMAL_NEXT_DIAGNOSTIC_ACTION,
        });
        // §4.6.1: count cleared, next threshold +1 (capped 5) — the count
        // zeroing is the once-per-stage guarantee; the next fire needs the
        // next threshold's signals.
        s.signal_count = 0;
        s.threshold = (s.threshold + 1).min(5);
        s.trigger_count += 1;
        (payload, message_block)
    };
    writer
        .record(EventType::DiagnosticCoverageCheckpoint, payload)
        .await?;
    messages.push(Message {
        role: Role::User,
        content: message_block,
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    Ok(())
}

/// Extract the first error class from a tool output's leading lines
/// (`Error` / `panic` / `Traceback` / `FAILED` normalized) — `None` when the
/// output shows no failure signature.
fn extract_error_class(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| {
            let t = line.trim();
            let failed = t.contains("Error") || t.contains("panic") || t.contains("Traceback");
            if failed || t.contains("FAILED") {
                Some(t.chars().take(80).collect::<String>())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tc(name: &str, call_id: &str) -> ToolCall {
        ToolCall {
            name: name.to_string(),
            arguments: serde_json::json!({}),
            call_id: call_id.to_string(),
        }
    }

    fn result(output: &str, exit_code: Option<i32>) -> ToolResult {
        ToolResult {
            output: output.to_string(),
            exit_code,
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
            types.iter().filter(|t| **t == "unabsorbed_new_evidence").count(),
            1,
            "error-class signal deduped: {types:?}"
        );
        assert_eq!(
            types.iter().filter(|t| **t == "consecutive_same_failure").count(),
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
}
