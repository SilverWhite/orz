//! Bridge — the ONLY module allowed to import `orz_assurance` (Python
//! `bridge.py` rule). Maps canonical journal `RunEvent`s into the TUI's
//! independent `TuiEvent` protocol.
//!
//! Extraction is defensive: every payload field is read with a fallback
//! (Python `getattr(..., "")` degrade). Unknown `event_type` values degrade
//! to `TuiEvent::Unknown` instead of failing.

use serde_json::Value;

use orz_assurance::journal::{EventType, RunEvent};

use crate::events::{ToolCallInfo, TuiEvent};

/// Extract a string field from a payload with a default.
fn get_str(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// Extract a u64 field from a payload with a default.
fn get_u64(payload: &Value, key: &str) -> u64 {
    payload.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Extract an optional u64 field.
fn get_opt_u64(payload: &Value, key: &str) -> Option<u64> {
    payload.get(key).and_then(Value::as_u64)
}

/// Extract an i64 field from a payload with a default.
fn get_i64(payload: &Value, key: &str) -> i64 {
    payload.get(key).and_then(Value::as_i64).unwrap_or(0)
}

/// Extract an optional string field.
fn get_opt_str(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_string)
}

/// Extract a string list field (array of strings, or JSON-encoded strings).
fn get_str_list(payload: &Value, key: &str) -> Vec<String> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| match v {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Extract the tool-call list from a `model_output` payload.
fn get_tool_calls(payload: &Value) -> Vec<ToolCallInfo> {
    payload
        .get("tool_calls")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|tc| ToolCallInfo {
                    name: get_str(tc, "name"),
                    arguments: get_str(tc, "arguments"),
                    call_id: get_str(tc, "call_id"),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Map one canonical journal event to the TUI event protocol.
pub fn run_event_to_tui(event: &RunEvent) -> TuiEvent {
    let p = &event.payload;
    match &event.event_type {
        EventType::RunPreflight => TuiEvent::RunPreflight {
            timestamp: event.timestamp.clone(),
        },
        EventType::RunStarted => TuiEvent::RunStarted {
            prompt: get_str(p, "prompt"),
            timestamp: event.timestamp.clone(),
        },
        EventType::RunFinished => TuiEvent::RunFinished {
            status: get_str(p, "status"),
        },
        EventType::RunFailed => TuiEvent::RunFailed {
            error: get_str(p, "error"),
        },
        EventType::RunCancelled => TuiEvent::RunCancelled {
            reason: get_str(p, "reason"),
        },
        EventType::RunInvalidated => TuiEvent::RunInvalidated {
            status: get_str(p, "status"),
        },
        EventType::PromptSubmitted => TuiEvent::PromptSubmitted {
            prompt: get_str(p, "prompt"),
            character_count: get_u64(p, "character_count"),
        },
        EventType::ModelRequest => TuiEvent::ModelRequest {
            provider: get_str(p, "provider"),
            model_id: get_str(p, "model_id"),
        },
        EventType::ModelResponseReceived => TuiEvent::ModelResponseReceived {
            tool_calls: get_tool_calls(p),
        },
        EventType::ModelOutput => TuiEvent::ModelOutput {
            text: get_str(p, "text"),
            tool_calls: get_tool_calls(p),
            finish_reason: get_str(p, "finish_reason"),
        },
        EventType::AcpInitialize => TuiEvent::AcpInitialize {
            protocol_version: get_i64(p, "protocol_version"),
        },
        EventType::AcpSessionCreated => TuiEvent::AcpSessionCreated {
            session_id: get_str(p, "session_id"),
        },
        EventType::ToolProposal => TuiEvent::ToolProposal {
            tool_name: get_str(p, "tool"),
            call_id: get_str(p, "call_id"),
            input_summary: get_str(p, "input_summary"),
        },
        EventType::PermissionRequested => TuiEvent::PermissionRequested {
            tool: get_str(p, "tool"),
            risk: get_str(p, "risk"),
            call_id: get_str(p, "call_id"),
        },
        EventType::PermissionDecision => TuiEvent::PermissionDecision {
            tool: get_str(p, "tool"),
            decision: get_str(p, "decision"),
        },
        EventType::ToolStarted => TuiEvent::ToolStarted {
            tool: get_str(p, "tool"),
            call_id: get_str(p, "call_id"),
            target: get_opt_str(p, "target"),
        },
        EventType::ToolCompleted => TuiEvent::ToolCompleted {
            tool: get_str(p, "tool"),
            call_id: get_str(p, "call_id"),
            status: get_opt_str(p, "status").unwrap_or_else(|| {
                if get_opt_str(p, "error").is_some() {
                    "error".into()
                } else {
                    "success".into()
                }
            }),
            error: get_opt_str(p, "error"),
            target: get_opt_str(p, "target"),
        },
        EventType::OrientationCheckpoint => TuiEvent::OrientationCheckpoint {
            checkpoint_id: get_str(p, "checkpoint_id"),
            trigger: get_str(p, "trigger"),
            step_index: get_i64(p, "step_index"),
        },
        EventType::RuntimeStagnationGuard => TuiEvent::RuntimeStagnationGuard {
            decision: get_str(p, "decision"),
            reason_codes: get_str_list(p, "reason_codes"),
        },
        EventType::ToolAvailabilityCheck => TuiEvent::ToolAvailabilityCheck {
            complete: get_str_list(p, "complete").len() as u64,
            // `incomplete` is an OBJECT array ({tool, reason}) — count the
            // array length directly; get_str_list would drop objects.
            incomplete: p
                .get("incomplete")
                .and_then(Value::as_array)
                .map(|a| a.len() as u64)
                .unwrap_or(0),
            gate_decision: get_str(p, "gate_decision"),
        },
        EventType::ToolBeliefStagnation => TuiEvent::ToolBeliefStagnation {
            tool: get_str(p, "tool"),
        },
        EventType::InstructionProvenanceGate => TuiEvent::InstructionProvenanceGate {
            decision: get_str(p, "decision"),
            entries: get_u64(p, "entries"),
        },
        EventType::GateDecision => TuiEvent::GateDecision {
            gate: get_str(p, "gate"),
            decision: get_str(p, "decision"),
            reason: get_opt_str(p, "reason"),
            tools: get_str_list(p, "tools"),
        },
        EventType::NeutralInquiry => TuiEvent::NeutralInquiry {
            trigger_reason: get_str(p, "trigger_reason"),
        },
        EventType::CounterexampleGate => TuiEvent::CounterexampleGate {
            position: get_str(p, "position"),
            model_response: get_opt_str(p, "model_response"),
        },
        EventType::RetrievalCompletionCheck => TuiEvent::RetrievalCompletionCheck {
            role: get_str(p, "role"),
            decision: get_str(p, "decision"),
        },
        // GAP-INQUIRY-SPLIT (2026-08-09): v0.2 mechanism events. The
        // diagnostic-coverage and disposition variants are projection
        // placeholders — this slice does not produce them yet (the schema
        // and fixtures are in place).
        EventType::DiagnosticCoverageCheckpoint => TuiEvent::DiagnosticCoverageCheckpoint {
            checkpoint_id: get_str(p, "checkpoint_id"),
            threshold_stage: get_i64(p, "threshold_stage"),
        },
        EventType::InformationSufficiencyAssessment => TuiEvent::InformationSufficiencyAssessment {
            assessment_id: get_str(p, "assessment_id"),
            status: get_str(p, "status"),
            source_total: p
                .pointer("/source_counts/total")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        },
        EventType::RetrievalParentDisposition => TuiEvent::RetrievalParentDisposition {
            disposition_id: get_str(p, "disposition_id"),
            decision: get_str(p, "decision"),
        },
        EventType::RetrievalCloseRecord => TuiEvent::RetrievalCloseRecord {
            close_record_id: get_str(p, "close_record_id"),
            terminal_reason: get_str(p, "terminal_reason"),
        },
        // GAP-RETRIEVAL-TOOLS (2026-08-10): mode transition / result commit /
        // activation restore projections.
        EventType::RetrievalModeTransition => TuiEvent::RetrievalModeTransition {
            transition_id: get_str(p, "transition_id"),
            old_mode: get_str(p, "old_mode"),
            new_mode: get_str(p, "new_mode"),
        },
        EventType::RetrievalResultCommitted => TuiEvent::RetrievalResultCommitted {
            result_id: get_str(p, "result_id"),
            source_total: p
                .pointer("/source_counts/total")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        },
        EventType::RetrievalActivationRestored => TuiEvent::RetrievalActivationRestored {
            restore_id: get_str(p, "restore_id"),
            activation_id: get_str(p, "activation_id"),
            status: get_str(p, "status"),
        },
        // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): output-level citation
        // verifier (ADR-0010 §3.7.9) — decision + reason codes only; the
        // marker-level details live in the journal payload.
        EventType::CitationValidation => TuiEvent::CitationValidation {
            decision: get_str(p, "decision"),
            reason_codes: get_str_list(p, "reason_codes"),
        },
        // ACAF Slice 1 (ADR-0011 §4.6): control-ticket lifecycle projections
        // (mechanism events — the HMAC never reaches the TUI; binding fields
        // and reject codes only).
        EventType::ControlTicketIssued => TuiEvent::ControlTicketIssued {
            ticket_id: get_str(p, "ticket_id"),
            ticket_kind: get_str(p, "ticket_kind"),
            capability_scope: get_str(p, "capability_scope"),
            sequence: get_opt_u64(p, "sequence"),
        },
        EventType::ControlTicketConsumed => TuiEvent::ControlTicketConsumed {
            ticket_id: get_str(p, "ticket_id"),
            ticket_kind: get_str(p, "ticket_kind"),
        },
        EventType::ControlTicketRejected => TuiEvent::ControlTicketRejected {
            ticket_id: get_str(p, "ticket_id"),
            ticket_kind: get_str(p, "ticket_kind"),
            reject_code: get_str(p, "reject_code"),
        },
        EventType::ContextCompressed => TuiEvent::ContextCompressed {
            trigger_tokens: get_u64(p, "trigger_tokens"),
            rounds_dropped: get_u64(p, "rounds_dropped"),
            estimated_tokens_after: get_u64(p, "estimated_tokens_after"),
        },
        EventType::SnapshotCreated => TuiEvent::SnapshotCreated {
            tool: get_str(p, "tool"),
            targets: get_str_list(p, "targets"),
            snapshot_hash: get_opt_str(p, "snapshot_hash"),
            snapshot_error: get_opt_str(p, "snapshot_error"),
        },
        EventType::SnapshotRestored => TuiEvent::SnapshotRestored {
            snapshot_hash: get_opt_str(p, "snapshot_hash"),
            scope: if p.get("scope").is_some() {
                Some(get_str_list(p, "scope"))
            } else {
                None
            },
            restored: get_str_list(p, "restored"),
            snapshot_error: get_opt_str(p, "snapshot_error"),
        },
        EventType::ArtifactRegistered => TuiEvent::ArtifactRegistered {
            artifact_path: get_str(p, "artifact_path"),
            artifact_sha256: get_str(p, "artifact_sha256"),
        },
        EventType::PlanProposed => TuiEvent::PlanProposed {
            plan_id: get_str(p, "plan_id"),
            task_id: get_str(p, "task_id"),
            sections: get_u64(p, "sections"),
        },
        EventType::PlanApproved => TuiEvent::PlanApproved {
            plan_id: get_str(p, "plan_id"),
            authority: get_str(p, "authority"),
            decision: get_str(p, "decision"),
            execution_policy: get_str(p, "execution_policy"),
        },
        EventType::PlanRejected => TuiEvent::PlanRejected {
            plan_id: get_str(p, "plan_id"),
        },
        EventType::ActionApproved => TuiEvent::ActionApproved {
            action_id: get_str(p, "action_id"),
        },
    }
}

/// Map an unknown `event_type` string (from a journal line the schema does
/// not recognize) to the degrade variant.
pub fn unknown_event_type(kind: &str) -> TuiEvent {
    TuiEvent::Unknown {
        event_type: kind.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orz_assurance::journal::{Redaction, RunEvent};
    use orz_assurance::seal_event;
    use serde_json::json;

    fn make_event(event_type: EventType, payload: Value) -> RunEvent {
        let mut ev = RunEvent::new_v01(
            "RUN-TEST01".into(),
            0,
            event_type,
            "manifest".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            payload,
            Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        seal_event(&mut ev).expect("sealing a test event must succeed");
        ev
    }

    #[test]
    fn maps_all_event_types_without_panic() {
        let cases = [
            // A6 (2026-08-08): explicit context compaction.
            (
                EventType::ContextCompressed,
                json!({
                    "trigger_tokens": 152000,
                    "target_tokens": 100000,
                    "rounds_since_last_compaction": 22,
                    "rounds_dropped": 4,
                    "messages_dropped": 12,
                    "messages_kept": 8,
                    "estimated_tokens_after": 95000,
                }),
            ),
            (EventType::RunPreflight, json!({})),
            (EventType::RunStarted, json!({"prompt": "hi"})),
            (EventType::RunFinished, json!({"status": "completed"})),
            (EventType::RunFailed, json!({"error": "boom"})),
            (EventType::RunCancelled, json!({"reason": "user"})),
            (
                EventType::RunInvalidated,
                json!({"status": "restart_requested"}),
            ),
            (
                EventType::PromptSubmitted,
                json!({"prompt": "hi", "character_count": 2}),
            ),
            (EventType::ModelRequest, json!({})),
            (EventType::ModelResponseReceived, json!({"tool_calls": []})),
            (
                EventType::ModelOutput,
                json!({"text": "ok", "tool_calls": [], "finish_reason": "stop"}),
            ),
            (EventType::AcpInitialize, json!({"protocol_version": 1})),
            (EventType::AcpSessionCreated, json!({"session_id": "s"})),
            (
                EventType::ToolProposal,
                json!({"tool": "bash", "call_id": "c", "input_summary": "ls"}),
            ),
            (
                EventType::PermissionRequested,
                json!({"tool": "bash", "risk": "SandboxEscape", "call_id": "c"}),
            ),
            (
                EventType::PermissionDecision,
                json!({"tool": "bash", "decision": "deny"}),
            ),
            (
                EventType::ToolStarted,
                json!({"tool": "read_file", "call_id": "c"}),
            ),
            (
                EventType::ToolCompleted,
                json!({"tool": "read_file", "call_id": "c", "exit_code": 0}),
            ),
            (
                EventType::OrientationCheckpoint,
                json!({"checkpoint_id": "ORIENT-1", "trigger": "pre_handoff", "step_index": 0}),
            ),
            (
                EventType::RuntimeStagnationGuard,
                json!({"decision": "continue", "reason_codes": ["REPEATED_CONTENT"]}),
            ),
            (
                EventType::ToolAvailabilityCheck,
                json!({
                    "probe_scope": "main_agent_work_tools",
                    "complete": ["read_file", "grep"],
                    "incomplete": [{"tool": "ask_user_question", "reason": "无交互式用户会话"}],
                    "gate_decision": "pass"
                }),
            ),
            (EventType::ToolBeliefStagnation, json!({"tool": "bash"})),
            (
                EventType::InstructionProvenanceGate,
                json!({"decision": "block", "entries": 1}),
            ),
            (
                EventType::GateDecision,
                json!({"gate": "instruction_provenance_gate", "decision": "block", "tools": ["bash"]}),
            ),
            (
                EventType::NeutralInquiry,
                json!({"trigger_reason": "rounds"}),
            ),
            (
                EventType::CounterexampleGate,
                json!({"position": "final_answer", "once_only": true}),
            ),
            (
                EventType::RetrievalCompletionCheck,
                json!({"role": "internal_retrieval", "decision": "yes"}),
            ),
            (
                EventType::SnapshotCreated,
                json!({"tool": "edit_file", "targets": ["lib.rs"], "snapshot_hash": "abc"}),
            ),
            (
                EventType::SnapshotRestored,
                json!({"snapshot_hash": "abc", "restored": ["lib.rs"]}),
            ),
            (
                EventType::ArtifactRegistered,
                json!({"artifact_path": "p", "artifact_sha256": "h"}),
            ),
            (
                EventType::PlanProposed,
                json!({"plan_id": "PLAN-1", "task_id": "T", "sections": 4}),
            ),
            (
                EventType::PlanApproved,
                json!({"plan_id": "PLAN-1", "authority": "user", "decision": "approve", "execution_policy": "manual"}),
            ),
            (EventType::PlanRejected, json!({"plan_id": "PLAN-1"})),
            (EventType::ActionApproved, json!({"action_id": "A"})),
            (
                EventType::CitationValidation,
                json!({
                    "decision": "block",
                    "reason_codes": ["unknown_source_id"],
                }),
            ),
        ];
        for (ty, payload) in cases {
            let tui = run_event_to_tui(&make_event(ty, payload));
            // Every canonical kind must land on its own variant (never Unknown).
            let kind = tui.kind();
            assert_ne!(kind, "unknown", "event_type must not degrade: {kind}");
        }
    }

    #[test]
    fn tool_availability_counts_object_arrays() {
        let ev = make_event(
            EventType::ToolAvailabilityCheck,
            json!({
                "probe_scope": "main_agent_work_tools",
                "complete": ["read_file", "list_dir", "grep"],
                "incomplete": [
                    {"tool": "run_tests", "reason": "缺少测试运行器"},
                    {"tool": "ask_user_question", "reason": "无交互式用户会话"},
                ],
                "gate_decision": "pass",
            }),
        );
        let TuiEvent::ToolAvailabilityCheck {
            complete,
            incomplete,
            gate_decision,
        } = run_event_to_tui(&ev)
        else {
            panic!("expected ToolAvailabilityCheck");
        };
        assert_eq!(complete, 3);
        assert_eq!(incomplete, 2, "object-array incomplete must be counted");
        assert_eq!(gate_decision, "pass");
    }

    #[test]
    fn tool_output_extracts_calls() {
        let ev = make_event(
            EventType::ModelOutput,
            json!({
                "text": "using tools",
                "tool_calls": [
                    {"name": "bash", "arguments": "{\"command\":\"dir\"}", "call_id": "call-1"}
                ],
                "finish_reason": "tool_calls",
            }),
        );
        let TuiEvent::ModelOutput {
            text,
            tool_calls,
            finish_reason,
        } = run_event_to_tui(&ev)
        else {
            panic!("expected ModelOutput");
        };
        assert_eq!(text, "using tools");
        assert_eq!(finish_reason, "tool_calls");
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].name, "bash");
        assert_eq!(tool_calls[0].call_id, "call-1");
    }

    #[test]
    fn missing_payload_fields_degrade_gracefully() {
        // Empty payload — every getter must fall back, never panic.
        let ev = make_event(EventType::ModelOutput, json!({}));
        let TuiEvent::ModelOutput {
            text, tool_calls, ..
        } = run_event_to_tui(&ev)
        else {
            panic!("expected ModelOutput");
        };
        assert_eq!(text, "");
        assert!(tool_calls.is_empty());

        // ToolCompleted without status but with error → "error".
        let ev = make_event(
            EventType::ToolCompleted,
            json!({"tool": "bash", "call_id": "c", "error": "kaboom"}),
        );
        let TuiEvent::ToolCompleted { status, error, .. } = run_event_to_tui(&ev) else {
            panic!("expected ToolCompleted");
        };
        assert_eq!(status, "error");
        assert_eq!(error.as_deref(), Some("kaboom"));
    }

    #[test]
    fn unknown_event_type_degrades() {
        let ev = unknown_event_type("future_event_v99");
        assert!(matches!(ev, TuiEvent::Unknown { .. }));
        assert_eq!(ev.kind(), "unknown");
        // The projection renders it as a warning card without panicking.
        let mut app = crate::app::TuiApp::new();
        app.accept_event(ev);
        assert!(!app.content.items[0].collapsed());
    }

    #[test]
    fn snapshot_hash_or_error_are_mutually_exclusive() {
        let ok = make_event(
            EventType::SnapshotCreated,
            json!({"tool": "edit_file", "targets": ["a"], "snapshot_hash": "h"}),
        );
        let TuiEvent::SnapshotCreated {
            snapshot_hash,
            snapshot_error,
            ..
        } = run_event_to_tui(&ok)
        else {
            panic!("expected SnapshotCreated");
        };
        assert_eq!(snapshot_hash.as_deref(), Some("h"));
        assert_eq!(snapshot_error, None);

        let err = make_event(
            EventType::SnapshotCreated,
            json!({"tool": "edit_file", "targets": ["a"], "snapshot_error": "disk full"}),
        );
        let TuiEvent::SnapshotCreated {
            snapshot_hash,
            snapshot_error,
            ..
        } = run_event_to_tui(&err)
        else {
            panic!("expected SnapshotCreated");
        };
        assert_eq!(snapshot_error.as_deref(), Some("disk full"));
        assert_eq!(snapshot_hash, None);
    }

    /// Restore maps scope (revert) vs no scope (full restore) and the
    /// error branch (no hash/restored).
    #[test]
    fn snapshot_restored_maps_scope_and_error() {
        let full = make_event(
            EventType::SnapshotRestored,
            json!({"snapshot_hash": "h", "restored": ["a.txt"]}),
        );
        let TuiEvent::SnapshotRestored {
            snapshot_hash,
            scope,
            restored,
            snapshot_error,
        } = run_event_to_tui(&full)
        else {
            panic!("expected SnapshotRestored");
        };
        assert_eq!(snapshot_hash.as_deref(), Some("h"));
        assert_eq!(scope, None, "full restore has no scope key");
        assert_eq!(restored, ["a.txt"]);
        assert_eq!(snapshot_error, None);

        let revert = make_event(
            EventType::SnapshotRestored,
            json!({"snapshot_hash": "h", "scope": ["a.txt"], "restored": ["a.txt"]}),
        );
        let TuiEvent::SnapshotRestored { scope, .. } = run_event_to_tui(&revert) else {
            panic!("expected SnapshotRestored");
        };
        assert_eq!(scope, Some(vec!["a.txt".to_string()]));

        let err = make_event(
            EventType::SnapshotRestored,
            json!({"snapshot_error": "snapshot not found"}),
        );
        let TuiEvent::SnapshotRestored {
            snapshot_hash,
            restored,
            snapshot_error,
            ..
        } = run_event_to_tui(&err)
        else {
            panic!("expected SnapshotRestored");
        };
        assert_eq!(snapshot_error.as_deref(), Some("snapshot not found"));
        assert_eq!(snapshot_hash, None);
        assert!(restored.is_empty());
    }
}
