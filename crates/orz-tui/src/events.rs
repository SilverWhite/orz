//! TUI event protocol — the workbench's own event model.
//!
//! Mirrors `orz_assurance::journal::EventType` (which mirrors
//! `runtime/run-event-v0.1.schema.json`) but is **completely independent**
//! of the assurance core — no imports from `orz_assurance` are permitted
//! here (Python `events.py` rule). The bridge module maps canonical
//! `RunEvent`s into this protocol; the projection layer consumes only this
//! enum.

use std::fmt;

/// A tool call carried by a model output (name + argument summary).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallInfo {
    pub name: String,
    pub arguments: String,
    pub call_id: String,
}

/// A single permission request option id (ACP option id, e.g. "allow-once").
pub type OptionId = String;

/// Everything the workbench can display or react to.
#[derive(Debug, Clone, PartialEq)]
pub enum TuiEvent {
    // ── lifecycle ──
    RunPreflight {
        timestamp: String,
    },
    RunStarted {
        prompt: String,
        timestamp: String,
    },
    RunFinished {
        status: String,
    },
    RunFailed {
        error: String,
    },
    RunCancelled {
        reason: String,
    },
    RunInvalidated {
        status: String,
    },

    // ── prompt / model ──
    PromptSubmitted {
        prompt: String,
        character_count: u64,
    },
    ModelRequest {
        provider: String,
        model_id: String,
    },
    ModelResponseReceived {
        tool_calls: Vec<ToolCallInfo>,
    },
    ModelOutput {
        text: String,
        tool_calls: Vec<ToolCallInfo>,
        finish_reason: String,
    },
    // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6): the model
    // request header (system+tools+config fingerprint) was first observed
    // or changed — shown so a prefix-cache miss / probe flip is explainable
    // at a glance; full digests stay in the journal payload.
    RequestHeaderChange {
        reason: String,
        tool_count: u64,
        header_sha256: String,
    },

    // ── ACP lifecycle ──
    AcpInitialize {
        protocol_version: i64,
    },
    AcpSessionCreated {
        session_id: String,
    },

    // ── tools ──
    ToolProposal {
        tool_name: String,
        call_id: String,
        input_summary: String,
    },
    PermissionRequested {
        tool: String,
        risk: String,
        call_id: String,
    },
    PermissionDecision {
        tool: String,
        decision: String,
    },
    ToolStarted {
        tool: String,
        call_id: String,
        target: Option<String>,
    },
    ToolCompleted {
        tool: String,
        call_id: String,
        status: String,
        error: Option<String>,
        target: Option<String>,
    },

    // ── assurance gates ──
    OrientationCheckpoint {
        checkpoint_id: String,
        trigger: String,
        step_index: i64,
    },
    RuntimeStagnationGuard {
        decision: String,
        reason_codes: Vec<String>,
    },
    ToolAvailabilityCheck {
        complete: u64,
        incomplete: u64,
        gate_decision: String,
    },
    ToolBeliefStagnation {
        tool: String,
    },
    InstructionProvenanceGate {
        decision: String,
        entries: u64,
    },
    GateDecision {
        gate: String,
        decision: String,
        reason: Option<String>,
        tools: Vec<String>,
    },

    // ── inquiry gates (§4.6) ──
    // GAP-INQUIRY-SPLIT (2026-08-09): `NeutralInquiry` /
    // `RetrievalCompletionCheck` remain for v0.1 replay display only — the
    // v0.2 producer never writes them.
    NeutralInquiry {
        trigger_reason: String,
    },
    CounterexampleGate {
        position: String,
        model_response: Option<String>,
    },
    RetrievalCompletionCheck {
        role: String,
        decision: String,
    },

    // ── v0.2 mechanism events (GAP-INQUIRY-SPLIT, 2026-08-09) ──
    DiagnosticCoverageCheckpoint {
        checkpoint_id: String,
        threshold_stage: i64,
    },
    // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16): the
    // forced-template checkpoint round's answer/validation verdict — the
    // TUI shows the outcome and error summary; the full parsed template and
    // cross-check stay in the journal payload.
    CheckpointResponse {
        checkpoint_id: String,
        inquiry_kind: String,
        attempt: i64,
        outcome: String,
        validation_valid: bool,
        validation_error_count: i64,
    },
    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17): first-round plan gate
    // result — outcome and validation summary only; the full cleaned plan
    // stays in the journal payload.
    PlanWrite {
        plan_id: String,
        goal: String,
        step_count: i64,
        outcome: String,
        validation_valid: bool,
        validation_error_count: i64,
        degrade_reason: Option<String>,
    },
    InformationSufficiencyAssessment {
        assessment_id: String,
        status: String,
        source_total: u64,
    },
    RetrievalParentDisposition {
        disposition_id: String,
        decision: String,
    },
    RetrievalCloseRecord {
        close_record_id: String,
        terminal_reason: String,
    },
    // GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode authority (§3.7.1),
    // structured result commit (§3.3.3) and cross-run activation restore
    // (§3.3) — projection placeholders; the payload details render through
    // the generic fallback in projection.rs.
    RetrievalModeTransition {
        transition_id: String,
        old_mode: String,
        new_mode: String,
    },
    RetrievalResultCommitted {
        result_id: String,
        source_total: u64,
    },
    RetrievalActivationRestored {
        restore_id: String,
        activation_id: String,
        status: String,
    },
    // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): output-level citation
    // verifier block (ADR-0010 §3.7.9) — the TUI shows decision + reason
    // codes only; the full marker details stay in the journal payload.
    CitationValidation {
        decision: String,
        reason_codes: Vec<String>,
    },
    // ACAF Slice 1 (ADR-0011 §4.6): control-ticket lifecycle projections.
    ControlTicketIssued {
        ticket_id: String,
        ticket_kind: String,
        capability_scope: String,
        sequence: Option<u64>,
    },
    ControlTicketConsumed {
        ticket_id: String,
        ticket_kind: String,
    },
    ControlTicketRejected {
        ticket_id: String,
        ticket_kind: String,
        reject_code: String,
    },

    // ── A6 explicit context compaction ──
    ContextCompressed {
        trigger_tokens: u64,
        rounds_dropped: u64,
        estimated_tokens_after: u64,
    },
    // D2-2 (2026-08-14, ADR-0010 v1.10): recovery pre-check truncation.
    ContextRecoveryTruncated {
        before_estimate_tokens: u64,
        rounds_dropped: u64,
        after_estimate_tokens: u64,
    },
    // F7 (2026-08-15, BACKLOG 6e 复查遗留): a blackboard plan-epoch
    // snapshot failed to persist — the rotation still committed, but the
    // archive is missing (audit trace; the journal payload is the record).
    EpochArchiveWriteFailed {
        plan_epoch: u64,
        archive_dir: String,
        kind: String,
    },

    // ── IP5 snapshot ──
    SnapshotCreated {
        tool: String,
        targets: Vec<String>,
        snapshot_hash: Option<String>,
        snapshot_error: Option<String>,
    },
    /// IP5 restore/revert (slice #8 — host restore runs are their own
    /// journals: preflight → snapshot_restored → terminal).
    SnapshotRestored {
        snapshot_hash: Option<String>,
        scope: Option<Vec<String>>,
        restored: Vec<String>,
        snapshot_error: Option<String>,
    },

    // ── artifact ──
    ArtifactRegistered {
        artifact_path: String,
        artifact_sha256: String,
    },

    // ── plan mode ──
    PlanProposed {
        plan_id: String,
        task_id: String,
        sections: u64,
    },
    PlanApproved {
        plan_id: String,
        plan_epoch: u64,
        authority: String,
        decision: String,
        execution_policy: String,
    },
    PlanRejected {
        plan_id: String,
    },
    ActionApproved {
        action_id: String,
    },

    // ── synthetic (TUI-owned, not in the run-event schema) ──
    /// Streaming text chunk — accumulates into the current model card.
    TextDelta {
        text: String,
    },
    /// Generic status-bar label update.
    StatusUpdate {
        label: String,
        ok: bool,
    },
    /// An event_type the bridge did not recognize (Python degrade rule).
    Unknown {
        event_type: String,
    },
}

impl TuiEvent {
    /// Snake_case kind string — the run-event `event_type` value this event
    /// mirrors (synthetic kinds use their own names).
    pub fn kind(&self) -> &'static str {
        match self {
            TuiEvent::RunPreflight { .. } => "run_preflight",
            TuiEvent::RunStarted { .. } => "run_started",
            TuiEvent::RunFinished { .. } => "run_finished",
            TuiEvent::RunFailed { .. } => "run_failed",
            TuiEvent::RunCancelled { .. } => "run_cancelled",
            TuiEvent::RunInvalidated { .. } => "run_invalidated",
            TuiEvent::PromptSubmitted { .. } => "prompt_submitted",
            TuiEvent::ModelRequest { .. } => "model_request",
            TuiEvent::ModelResponseReceived { .. } => "model_response_received",
            TuiEvent::ModelOutput { .. } => "model_output",
            TuiEvent::RequestHeaderChange { .. } => "request_header_change",
            TuiEvent::AcpInitialize { .. } => "acp_initialize",
            TuiEvent::AcpSessionCreated { .. } => "acp_session_created",
            TuiEvent::ToolProposal { .. } => "tool_proposal",
            TuiEvent::PermissionRequested { .. } => "permission_requested",
            TuiEvent::PermissionDecision { .. } => "permission_decision",
            TuiEvent::ToolStarted { .. } => "tool_started",
            TuiEvent::ToolCompleted { .. } => "tool_completed",
            TuiEvent::OrientationCheckpoint { .. } => "orientation_checkpoint",
            TuiEvent::RuntimeStagnationGuard { .. } => "runtime_stagnation_guard",
            TuiEvent::ToolAvailabilityCheck { .. } => "tool_availability_check",
            TuiEvent::ToolBeliefStagnation { .. } => "tool_belief_stagnation",
            TuiEvent::InstructionProvenanceGate { .. } => "instruction_provenance_gate",
            TuiEvent::GateDecision { .. } => "gate_decision",
            TuiEvent::NeutralInquiry { .. } => "neutral_inquiry",
            TuiEvent::CounterexampleGate { .. } => "counterexample_gate",
            TuiEvent::RetrievalCompletionCheck { .. } => "retrieval_completion_check",
            TuiEvent::DiagnosticCoverageCheckpoint { .. } => "diagnostic_coverage_checkpoint",
            TuiEvent::CheckpointResponse { .. } => "checkpoint_response",
            TuiEvent::PlanWrite { .. } => "plan_write",
            TuiEvent::InformationSufficiencyAssessment { .. } => {
                "information_sufficiency_assessment"
            }
            TuiEvent::RetrievalParentDisposition { .. } => "retrieval_parent_disposition",
            TuiEvent::RetrievalCloseRecord { .. } => "retrieval_close_record",
            TuiEvent::RetrievalModeTransition { .. } => "retrieval_mode_transition",
            TuiEvent::RetrievalResultCommitted { .. } => "retrieval_result_committed",
            TuiEvent::RetrievalActivationRestored { .. } => "retrieval_activation_restored",
            TuiEvent::CitationValidation { .. } => "citation_validation",
            TuiEvent::ControlTicketIssued { .. } => "control_ticket_issued",
            TuiEvent::ControlTicketConsumed { .. } => "control_ticket_consumed",
            TuiEvent::ControlTicketRejected { .. } => "control_ticket_rejected",
            TuiEvent::ContextCompressed { .. } => "context_compressed",
            TuiEvent::ContextRecoveryTruncated { .. } => "context_recovery_truncated",
            TuiEvent::EpochArchiveWriteFailed { .. } => "epoch_archive_write_failed",
            TuiEvent::SnapshotCreated { .. } => "snapshot_created",
            TuiEvent::SnapshotRestored { .. } => "snapshot_restored",
            TuiEvent::ArtifactRegistered { .. } => "artifact_registered",
            TuiEvent::PlanProposed { .. } => "plan_proposed",
            TuiEvent::PlanApproved { .. } => "plan_approved",
            TuiEvent::PlanRejected { .. } => "plan_rejected",
            TuiEvent::ActionApproved { .. } => "action_approved",
            TuiEvent::TextDelta { .. } => "text_delta",
            TuiEvent::StatusUpdate { .. } => "status_update",
            TuiEvent::Unknown { .. } => "unknown",
        }
    }

    /// Terminal run states (finished/failed/cancelled/invalidated).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TuiEvent::RunFinished { .. }
                | TuiEvent::RunFailed { .. }
                | TuiEvent::RunCancelled { .. }
                | TuiEvent::RunInvalidated { .. }
        )
    }
}

impl fmt::Display for TuiEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_strings_are_snake_case() {
        for ev in [
            TuiEvent::RunPreflight {
                timestamp: String::new(),
            },
            TuiEvent::RunFinished {
                status: "completed".into(),
            },
            TuiEvent::ToolAvailabilityCheck {
                complete: 0,
                incomplete: 0,
                gate_decision: "pass".into(),
            },
            TuiEvent::SnapshotCreated {
                tool: "edit_file".into(),
                targets: vec![],
                snapshot_hash: None,
                snapshot_error: None,
            },
            TuiEvent::TextDelta {
                text: String::new(),
            },
            TuiEvent::Unknown {
                event_type: "weird".into(),
            },
        ] {
            let k = ev.kind();
            assert!(!k.contains(' '), "kind {k} must not contain spaces");
            assert_eq!(k, k.to_lowercase(), "kind {k} must be lowercase");
        }
    }

    #[test]
    fn terminal_states_are_terminal() {
        assert!(
            TuiEvent::RunFinished {
                status: "completed".into()
            }
            .is_terminal()
        );
        assert!(TuiEvent::RunFailed { error: "x".into() }.is_terminal());
        assert!(TuiEvent::RunCancelled { reason: "x".into() }.is_terminal());
        assert!(TuiEvent::RunInvalidated { status: "x".into() }.is_terminal());
        assert!(
            !TuiEvent::RunStarted {
                prompt: "x".into(),
                timestamp: String::new()
            }
            .is_terminal()
        );
        assert!(!TuiEvent::TextDelta { text: "x".into() }.is_terminal());
    }
}
