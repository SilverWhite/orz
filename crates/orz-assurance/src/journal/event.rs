//! Journal event types — the data model for append-only hash-chained events.
//!
//! Mirrors `runtime/run-event-v0.1.schema.json` and (for the v0.2 track)
//! `runtime/run-event-v0.2.schema.json` from the Python reference spec.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Terminal event types that close a journal (no further events permitted).
pub const TERMINAL_EVENTS: &[EventType] = &[
    EventType::RunFinished,
    EventType::RunFailed,
    EventType::RunCancelled,
    EventType::RunInvalidated,
    // FUS-HOST-RESOURCE-SAFETY §4.3 (2026-09-12, 0z S2): the explicit
    // terminal shape for the two abnormal endings this design owns —
    // `resource_exhausted` (hard tier) and `journal_degraded` (the audit
    // chain starved). Design 判据 6: under ENOSPC the run still yields an
    // enumerable terminal shape — `run_terminated` on the chain or the
    // `TERMINAL.json` sidecar, never a silent death.
    EventType::RunTerminated,
];

/// All event types in the run lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Lifecycle
    RunPreflight,
    RunStarted,
    RunFinished,
    RunFailed,
    RunCancelled,
    RunInvalidated,
    // FUS-HOST-RESOURCE-SAFETY §4.3/§5 (2026-09-12, 0z S2): explicit terminal
    // shape for resource-exhausted and journal-degraded endings (design §6
    // 判据 6). Terminal — appended to `TERMINAL_EVENTS` above.
    RunTerminated,

    // FUS-HOST-RESOURCE-SAFETY §5 (2026-09-12, 0z S2): the host-resource
    // fact families. Tiers/limits use the snake_case machine keys decided in
    // S1 (`normal` / `watch` / `soft` / `reclaim_direct` / `hard` / `unknown`).
    /// Low-frequency readings face — one row per tier transition (design §4.5,
    /// §5 "低频，跨档才落").
    HostResourceSnapshot,
    /// Pre-dispatch refusal of a heavy action (gate §4.1 / soft 档) — readings
    /// plus the action class; the tool result envelope stays the model face.
    HostResourceDenied,
    /// Hard tier triggered: the pre-kill audit row (§4.8 表 1: planned with
    /// the call_id set about to be killed) and the post-kill row.
    ResourceExhausted,
    /// Orphan reaping audit — planned row first, then the reaped row
    /// (design §4.2 sweep, §4.8 表 2).
    ProcessTreeReaped,
    /// Reclaim audit — written BEFORE any deletion touches disk (design
    /// §4.6 四纪律 1; `outcome ∈ pending_delete / permanent / rejected`).
    ReclaimPerformed,
    /// A tool call hit a kernel-enforced Job ceiling (design §4.7 失败语义:
    /// the failure is labeled, never silent).
    ResourceLimitHit,

    // Prompt/model
    PromptSubmitted,
    ModelRequest,
    ModelResponseReceived,
    ModelOutput,
    // ORZ-CACHE-CONTEXT-COST (2026-08-15, ADR-0010 §3.5 条6/§14.9): the
    // model-request header (system + tools + config) changed or was first
    // observed — journaled so a prefix-cache miss is attributable and probe
    // flips can be cross-checked against the actual request shape.
    RequestHeaderChange,

    // ACP lifecycle
    AcpInitialize,
    AcpSessionCreated,

    // Tool
    ToolProposal,
    PermissionRequested,
    PermissionDecision,
    ToolStarted,
    ToolCompleted,
    // THIN-HARNESS-REDESIGN-V2 §9.7 (2026-08-29 S5-2): a terminal command
    // auto-backgrounded at the 300s report point — the mid-run status
    // (运行时长/进程状态/输出活跃度/落盘指针) journaled between the call's
    // ToolStarted and ToolCompleted (v0.2 track only; at most one per
    // call_id).
    ToolRunning,

    // TER T0.2/T1.9 (2026-09-03/04, TODO2 T0.2 §2 / 设计稿 §3.3): F6 push
    // 档机械注入留痕——剩余评测墙钟跨 <600/<300/<120s 阈值时记一条
    // （payload: remaining_seconds / rounds_used / threshold_seconds；
    // 默认 off，PUSH→PULL 例外）。verifier 每 run ≤4 次、remaining 严格
    // 小于 threshold。
    BudgetCueInjected,

    // Assurance
    OrientationCheckpoint,
    ToolAvailabilityCheck,
    ToolBeliefStagnation,
    InstructionProvenanceGate,
    GateDecision,

    // Inquiry gates (Phase 3, §4.6 wiring — mirrors run-event schema).
    // `NeutralInquiry` and `RetrievalCompletionCheck` are RETIRED on the v0.2
    // track (ADR-0010 §5.1/§11.2 — the mixed-counter inquiry mechanism is
    // gone, GAP-INQUIRY-SPLIT); the variants stay for v0.1 journal replay.
    // The v0.2 producer must never write them (EventWriter asserts).
    NeutralInquiry,
    CounterexampleGate,
    RetrievalCompletionCheck,

    // GAP-INQUIRY-SPLIT (2026-08-09): v0.2 mechanism events (ADR-0010 §5.1).
    // `DiagnosticCoverageCheckpoint` and `CheckpointResponse` are RETIRED on
    // the v0.2 track (P2-11 DC 清理 2026-08-31 — the diagnostic-coverage
    // forced-template mechanism is deleted, MODEL-RESIDUAL-PRESSURE-FOLLOWUP
    // 裁决 2); the variants stay for historical journal replay and the v0.2
    // producer must never write them.
    DiagnosticCoverageCheckpoint,
    CheckpointResponse,
    InformationSufficiencyAssessment,
    RetrievalParentDisposition,
    RetrievalCloseRecord,

    // GAP-RETRIEVAL-TOOLS (2026-08-10): retrieval mode authority (§3.7.1),
    // structured result commit (§3.3.3) and cross-run activation restore
    // (§3.3) — all three are non-terminal v0.2 mechanism events.
    // 0t (2026-09-09, ADR-0010 §14.65 / v1.65): `RetrievalModeTransition`
    // 生产者侧退役——事件类型保留（旧 v0.2 刊只读回放；schema 枚举不收缩），
    // 新 run 不再产出（三值检索模式 γ 整体退役）。
    RetrievalModeTransition,
    RetrievalResultCommitted,
    RetrievalActivationRestored,
    /// 0t (2026-09-09, ADR-0010 §14.65 / v1.65): 浏览器启动/探活尝试事实
    /// 事件——success/failure + 真实原因（三值模式退役后浏览器可用性纯
    /// 事件事实化）。v0.2 非终态机制事件。
    BrowserLaunchResult,
    // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39 / BACKLOG 0g):
    // 机械审查层轻量事件留痕——每次对象键覆盖写（含 step/契约类检查）以
    // `{kind, payload:{key, round, summary, anomaly}}` 记一条，供回放与
    // 验证；报告块本身不进归档（与 counterexample 注入同语义）。
    MechanicalAuditUpdate,

    // ACAF Slice 1 (ADR-0011 §4.2/§4.6): control-ticket lifecycle — issued /
    // consumed / rejected. All three are non-terminal v0.2 mechanism events
    // journaled by the controller's control producers.
    ControlTicketIssued,
    ControlTicketConsumed,
    ControlTicketRejected,

    // A6 explicit context compaction (2026-08-08 — controller-written;
    // mirrors run-event schema)
    ContextCompressed,
    // D2-2 (2026-08-14, ADR-0010 v1.10 — CONTEXT_COMPACTION_DESIGN §6):
    // a restored conversation was mechanically truncated before the first
    // request (recovery pre-check over the conservative window threshold;
    // the full sidecar was copied into the run journal for audit).
    ContextRecoveryTruncated,
    // F7 (2026-08-15, BACKLOG 6e 复查遗留): a blackboard plan-epoch
    // archive write failed after bounded retries. The rotation still
    // committed in memory, but the old epoch's records have no durable
    // snapshot — this event is the audit trace (`kind` distinguishes the
    // rotated-old snapshot from the current-epoch persistence).
    EpochArchiveWriteFailed,
    // P2-13 B3 (2026-09-03, ADR-0010 §14.52 / 设计 §12 R3): a conversation
    // was archived as a single gzip package at session close —
    // archive_id/path/digest/status/attempts/fatigue_pct.
    SessionArchive,

    // IP5 pre-mutation snapshot (Phase 3, slice #4 wiring)
    SnapshotCreated,
    /// IP5 restore/revert (Phase 3, slice #8 — host restore entry; a restore
    /// is its own run: preflight → snapshot_restored → terminal).
    SnapshotRestored,

    // Artifact
    ArtifactRegistered,

    // Plan mode (Phase 2, SLICE-10)
    PlanProposed,
    PlanApproved,
    PlanRejected,
    ActionApproved,
    // PLAN-FIRST 阶段 A (2026-08-16, ADR-0010 §14.17 / PLAN_FIRST_BLACKBOARD
    // _DESIGN §3-§5): the first-round plan gate's plan_write result — plan
    // identity/goal/step count, mechanical validation verdict, one-refill
    // attempt progression and degrade reason. Main lane only.
    PlanWrite,
    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / PLAN_FIRST
    // _BLACKBOARD_DESIGN §7): the console/direct dual-mode transition
    // decision record — console→direct (assistant-failure-streak switch),
    // the stay decision, and direct→console (console_return_to_console).
    // Main lane only; run-level mode state.
    ConsoleModeTransition,
    // PLAN-FIRST 阶段 C (2026-08-16, ADR-0010 §14.17⑱ / PLAN_FIRST
    // _BLACKBOARD_DESIGN §5-§6): the blackboard_action_write order record —
    // order identity, action, step binding (ActionOrder.step_id) and the
    // mechanical round/plan_epoch/run_id stamps. Carries the identity that
    // used to ride the action_write ToolCompleted (S2 payload-shape debt,
    // stage-A audit §7.4); the ToolCompleted converges to the generic shape.
    ConsoleOrderWritten,
    // P0-E 第 4 项 (2026-08-17, ADR-0010 §14.21 项 3 / PLAN_FIRST
    // _BLACKBOARD_DESIGN §5-§6): a written order refused before execution —
    // pre-issuance gates (order_stale / step_not_done /
    // budget_insufficient, phase=pre_issue) and issuance-time gates
    // (registry / contract / target / ACAF / policy / mode, phase=issue).
    // Unifies the structured rejection code into the journal; the
    // result-bar receipt stays the human-readable view. Never emitted for
    // execute/verify outcomes (those already journal tool_started /
    // tool_completed).
    ConsoleOrderRejected,
    // FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): a mechanical
    // action-ledger fold advance — the loop folded the next complete old
    // rounds into the frozen ledger block at the loop-top gap (zero model
    // calls). Emitted ONLY when the fold point actually advanced (not on
    // anti-spin no-ops). The request-view prefix is rewritten once per
    // advance, so the event carries the fold indices, the folded-round
    // count and the triggering view estimate for cache-miss attribution.
    LedgerFoldAdvance,
    // FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
    // §14.28 审查修复): the external-ledger append failed at a fold
    // advance — the fold state was rolled back and the failure counted;
    // after FOLD_WRITE_FAILURE_LIMIT consecutive failures folding is
    // disabled for the loop (`disabled: true`) so a persistent write
    // failure degrades to the pre-fold full view instead of spinning the
    // session on retries. Payload: ledger_path / attempt / disabled /
    // rows / view_estimate_tokens / agent_role.
    LedgerFoldWriteFailed,
    // MIDSTREAM-DECODE-RETRY (2026-08-21, ADR-0010 §14.37 / 设计 §2.3):
    // transport 重试计数事件面——一次逻辑模型请求内零 chunk/中段截断
    // 重试的可观测摘要：`outcome` recovered（重试后成功）/
    // exhausted（重试耗尽显式失败）、`kind` zero_chunk/midstream、
    // `retries` 重发次数、`reason` wire 级原因。事件面计数使重试频率
    // 与耗尽可审计（S4 复验的输入面）。
    TransportRetry,
}

impl EventType {
    /// Whether this event type terminates the journal.
    pub fn is_terminal(&self) -> bool {
        TERMINAL_EVENTS.contains(self)
    }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_string(self).unwrap_or_else(|_| "unknown".into());
        // Strip JSON quotes
        write!(f, "{}", s.trim_matches('"'))
    }
}

/// Redaction level for payload content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Redaction {
    None,
    MetadataOnly,
    ContentRedacted,
}

/// Journal track — the envelope schema family a producer writes
/// (GAP-INQUIRY-SPLIT, 2026-08-09).
///
/// A journal must be homogeneous on ONE track (§11.6.2 — mixed envelope
/// versions break the hash-chain contract): the track pins both the envelope
/// `schema_version` and the `payload_schema` identifier for every event.
/// `V02` is the production track; `V01` is the historical freeze, used only
/// by replay tests and legacy fixtures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventTrack {
    V01,
    V02,
}

impl EventTrack {
    /// The `payload_schema` value stamped on every event of this track.
    pub fn payload_schema_id(self) -> &'static str {
        match self {
            EventTrack::V01 => "run-event-v0.1.schema.json",
            EventTrack::V02 => "run-event-v0.2.schema.json",
        }
    }

    /// The envelope `schema_version` for this track.
    pub fn schema_version(self) -> &'static str {
        match self {
            EventTrack::V01 => "0.1.0-draft",
            EventTrack::V02 => "0.2.0-draft",
        }
    }
}

/// A single append-only event in the run journal.
///
/// Each event carries a hash chain link (`previous_event_sha256`) and its own
/// content hash (`event_sha256`). The chain is verified by `JournalVerifier`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunEvent {
    /// Schema version — "0.1.0-draft" (v0.1 track) or "0.2.0-draft" (v0.2
    /// track; GAP-INQUIRY-SPLIT — producers write the v0.2 track so the
    /// hash chain stays homogeneous, §11.6.2).
    pub schema_version: String,

    /// Unique run identifier, e.g. "RUN-{uuid}"
    pub run_id: String,

    /// Unique event identifier, e.g. "EVT-{run_id_suffix}-{seq:06}"
    pub event_id: String,

    /// Monotonic sequence number starting at 0.
    pub sequence: u64,

    /// ISO 8601 timestamp (UTC).
    pub timestamp: String,

    /// What happened.
    pub event_type: EventType,

    /// SHA-256 of the canonical run manifest JSON.
    pub run_manifest_sha256: String,

    /// SHA-256 of the previous event, or null for seq=0.
    /// Serialized as an explicit `null` for seq=0 — the run-event schema
    /// requires this key on every line (required array + allOf null rule).
    pub previous_event_sha256: Option<String>,

    /// Schema identifier for the payload.
    pub payload_schema: String,

    /// Arbitrary structured payload.
    pub payload: Value,

    /// SHA-256 of the canonical payload JSON.
    pub payload_sha256: String,

    /// Redaction level for this event.
    pub redaction: Redaction,

    /// SHA-256 of this event's canonical JSON (minus this field).
    /// Computed AFTER all other fields are set.
    pub event_sha256: String,
}

impl RunEvent {
    /// Create a new event with fields filled from context.
    /// The caller MUST call `seal_event()` (or `JournalRecorder::record()`,
    /// which seals automatically) before recording.
    ///
    /// `schema_version` selects the track ("0.1.0-draft" / "0.2.0-draft");
    /// the convenience constructors below pin it.
    fn new(
        schema_version: String,
        run_id: String,
        sequence: u64,
        event_type: EventType,
        run_manifest_sha256: String,
        previous_event_sha256: Option<String>,
        payload_schema: String,
        payload: Value,
        redaction: Redaction,
        timestamp: String,
    ) -> Self {
        let event_id = {
            // Strip the run-kind prefix ("RUN-" prompts, "RST-" restore
            // runs — slice #8) to get the suffix for the event ID. `{:03}`
            // matches the Python authority (canonical_cli.py uses
            // `{sequence:03d}`) so event hashes align across sides.
            let suffix = run_id
                .strip_prefix("RUN-")
                .or_else(|| run_id.strip_prefix("RST-"))
                .unwrap_or(&run_id);
            format!("EVT-{suffix}-{sequence:03}")
        };

        RunEvent {
            schema_version,
            run_id,
            event_id,
            sequence,
            timestamp,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            payload_sha256: String::new(), // filled by seal()
            redaction,
            event_sha256: String::new(), // filled by seal()
        }
    }

    /// v0.1 track constructor — legacy producers and replay tests only
    /// (the v0.1 schema is a historical freeze; the v0.2 producer must not
    /// write it).
    pub fn new_v01(
        run_id: String,
        sequence: u64,
        event_type: EventType,
        run_manifest_sha256: String,
        previous_event_sha256: Option<String>,
        payload_schema: String,
        payload: Value,
        redaction: Redaction,
        timestamp: String,
    ) -> Self {
        Self::new(
            "0.1.0-draft".into(),
            run_id,
            sequence,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            redaction,
            timestamp,
        )
    }

    /// v0.2 track constructor (GAP-INQUIRY-SPLIT, 2026-08-09): production
    /// producers write this track — `schema_version = "0.2.0-draft"` and
    /// `payload_schema = "run-event-v0.2.schema.json"` (set by the caller /
    /// `EventWriter::V02`).
    pub fn new_v02(
        run_id: String,
        sequence: u64,
        event_type: EventType,
        run_manifest_sha256: String,
        previous_event_sha256: Option<String>,
        payload_schema: String,
        payload: Value,
        redaction: Redaction,
        timestamp: String,
    ) -> Self {
        Self::new(
            "0.2.0-draft".into(),
            run_id,
            sequence,
            event_type,
            run_manifest_sha256,
            previous_event_sha256,
            payload_schema,
            payload,
            redaction,
            timestamp,
        )
    }

    /// Returns true if this event terminates the journal.
    pub fn is_terminal(&self) -> bool {
        self.event_type.is_terminal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_events_block_further_appends() {
        assert!(EventType::RunFinished.is_terminal());
        assert!(EventType::RunFailed.is_terminal());
        assert!(EventType::RunCancelled.is_terminal());
        assert!(!EventType::RunStarted.is_terminal());
        assert!(!EventType::PromptSubmitted.is_terminal());
    }

    #[test]
    fn event_id_format() {
        let event = RunEvent::new_v01(
            "RUN-A1B2C3D4".into(),
            0,
            EventType::RunStarted,
            "abcd1234".into(),
            None,
            "test-schema".into(),
            serde_json::json!({}),
            Redaction::None,
            "2026-08-04T00:00:00Z".into(),
        );
        assert_eq!(event.event_id, "EVT-A1B2C3D4-000");
    }
}
