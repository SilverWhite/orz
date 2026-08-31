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
    // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): terminal command
    // auto-backgrounded at the 300s report point — mid-run status.
    ToolRunning {
        tool: String,
        call_id: String,
        wall_ms: u64,
        pid: Option<u32>,
    },

    // ── assurance gates ──
    OrientationCheckpoint {
        checkpoint_id: String,
        trigger: String,
        step_index: i64,
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
    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): console/direct
    // 双模式切换决策记录（§7.4）。
    ConsoleModeTransition {
        transition_id: String,
        from: String,
        to: String,
        trigger: String,
        model_decision: String,
    },
    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱): 动作栏订单记录
    // （身份/step 绑定/机械盖章；§5-§6）。
    ConsoleOrderWritten {
        order_id: String,
        action: String,
        step_id: Option<String>,
    },
    // P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3): 发放前拒绝记录
    // （订单身份 + 信封 step + phase + 拒绝码）。
    ConsoleOrderRejected {
        order_id: String,
        step: String,
        phase: String,
        code: String,
    },
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): 机械折叠推进
    // ——每窗口一次前缀重写（接受的 miss），事件用于缓存 miss 归因。
    LedgerFoldAdvance {
        fold_start: u64,
        fold_cut: u64,
        rounds_folded: u64,
        view_estimate_tokens: u64,
        agent_role: String,
    },
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): 外挂台账追加失败——折叠回滚 + 失败计数；预算耗尽
    // 后折叠被禁用（审计告警）。
    LedgerFoldWriteFailed {
        ledger_path: String,
        attempt: u64,
        disabled: bool,
        rows: u64,
        view_estimate_tokens: u64,
        agent_role: String,
    },
    // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
    // transport 重试计数事件面——一次逻辑请求内零 chunk/中段截断重试的
    // recovered/exhausted 摘要（TUI 展示次数与类别）。
    TransportRetry {
        agent_role: String,
        outcome: String,
        kind: Option<String>,
        retries: u64,
        reason: Option<String>,
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
    // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): 机械审查层
    // 轻量事件留痕（键/轮/摘要/异常）——TUI 只展示键与异常，摘要细节留在
    // journal payload。
    MechanicalAuditUpdate {
        key: String,
        round: u64,
        anomaly: Option<String>,
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
            TuiEvent::ToolRunning { .. } => "tool_running",
            TuiEvent::OrientationCheckpoint { .. } => "orientation_checkpoint",
            TuiEvent::ToolAvailabilityCheck { .. } => "tool_availability_check",
            TuiEvent::ToolBeliefStagnation { .. } => "tool_belief_stagnation",
            TuiEvent::InstructionProvenanceGate { .. } => "instruction_provenance_gate",
            TuiEvent::GateDecision { .. } => "gate_decision",
            TuiEvent::NeutralInquiry { .. } => "neutral_inquiry",
            TuiEvent::CounterexampleGate { .. } => "counterexample_gate",
            TuiEvent::RetrievalCompletionCheck { .. } => "retrieval_completion_check",
            TuiEvent::PlanWrite { .. } => "plan_write",
            TuiEvent::ConsoleModeTransition { .. } => "console_mode_transition",
            TuiEvent::ConsoleOrderWritten { .. } => "console_order_written",
            TuiEvent::ConsoleOrderRejected { .. } => "console_order_rejected",
            TuiEvent::LedgerFoldAdvance { .. } => "ledger_fold_advance",
            TuiEvent::LedgerFoldWriteFailed { .. } => "ledger_fold_write_failed",
            TuiEvent::TransportRetry { .. } => "transport_retry",
            TuiEvent::InformationSufficiencyAssessment { .. } => {
                "information_sufficiency_assessment"
            }
            TuiEvent::RetrievalParentDisposition { .. } => "retrieval_parent_disposition",
            TuiEvent::RetrievalCloseRecord { .. } => "retrieval_close_record",
            TuiEvent::RetrievalModeTransition { .. } => "retrieval_mode_transition",
            TuiEvent::RetrievalResultCommitted { .. } => "retrieval_result_committed",
            TuiEvent::RetrievalActivationRestored { .. } => "retrieval_activation_restored",
            TuiEvent::MechanicalAuditUpdate { .. } => "mechanical_audit_update",
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
