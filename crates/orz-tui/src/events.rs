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
    RunPreflight { timestamp: String },
    RunStarted { prompt: String, timestamp: String },
    RunFinished { status: String },
    RunFailed { error: String },
    RunCancelled { reason: String },
    RunInvalidated { status: String },

    // ── prompt / model ──
    PromptSubmitted { prompt: String, character_count: u64 },
    ModelRequest { provider: String, model_id: String },
    ModelResponseReceived { tool_calls: Vec<ToolCallInfo> },
    ModelOutput {
        text: String,
        tool_calls: Vec<ToolCallInfo>,
        finish_reason: String,
    },

    // ── ACP lifecycle ──
    AcpInitialize { protocol_version: i64 },
    AcpSessionCreated { session_id: String },

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
    PermissionDecision { tool: String, decision: String },
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
        available: u64,
        unavailable: u64,
        degraded: u64,
        unprobed: u64,
        gate_decision: String,
    },
    ToolBeliefStagnation { tool: String },
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
    NeutralInquiry { trigger_reason: String },
    CounterexampleGate {
        position: String,
        model_response: Option<String>,
    },
    RetrievalCompletionCheck { role: String, decision: String },

    // ── IP5 snapshot ──
    SnapshotCreated {
        tool: String,
        targets: Vec<String>,
        snapshot_hash: Option<String>,
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
        authority: String,
        decision: String,
        execution_policy: String,
    },
    PlanRejected { plan_id: String },
    ActionApproved { action_id: String },

    // ── synthetic (TUI-owned, not in the run-event schema) ──
    /// Streaming text chunk — accumulates into the current model card.
    TextDelta { text: String },
    /// Generic status-bar label update.
    StatusUpdate { label: String, ok: bool },
    /// An event_type the bridge did not recognize (Python degrade rule).
    Unknown { event_type: String },
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
            TuiEvent::SnapshotCreated { .. } => "snapshot_created",
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
            TuiEvent::RunPreflight { timestamp: String::new() },
            TuiEvent::RunFinished { status: "completed".into() },
            TuiEvent::ToolAvailabilityCheck {
                available: 0,
                unavailable: 0,
                degraded: 0,
                unprobed: 0,
                gate_decision: "allow".into(),
            },
            TuiEvent::SnapshotCreated {
                tool: "edit_file".into(),
                targets: vec![],
                snapshot_hash: None,
                snapshot_error: None,
            },
            TuiEvent::TextDelta { text: String::new() },
            TuiEvent::Unknown { event_type: "weird".into() },
        ] {
            let k = ev.kind();
            assert!(!k.contains(' '), "kind {k} must not contain spaces");
            assert_eq!(k, k.to_lowercase(), "kind {k} must be lowercase");
        }
    }

    #[test]
    fn terminal_states_are_terminal() {
        assert!(TuiEvent::RunFinished { status: "completed".into() }.is_terminal());
        assert!(TuiEvent::RunFailed { error: "x".into() }.is_terminal());
        assert!(TuiEvent::RunCancelled { reason: "x".into() }.is_terminal());
        assert!(TuiEvent::RunInvalidated { status: "x".into() }.is_terminal());
        assert!(!TuiEvent::RunStarted { prompt: "x".into(), timestamp: String::new() }.is_terminal());
        assert!(!TuiEvent::TextDelta { text: "x".into() }.is_terminal());
    }
}
