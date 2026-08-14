//! Blackboard — structured shared state for multi-agent communication.
//!
//! 5 sections, each with a single writer. All sections are readable by all agents.
//! Write rules enforce single-writer discipline.
//!
//! Single main agent + two retrieval subagents (fusion §4.5): the main agent
//! writes Plan/Exec, each retrieval subagent writes its own section, and the
//! controller writes GateLog.

use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// A single step in the execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

/// Main agent writes: goal, steps, analysis, decisions, auth_grants.
///
/// v1.15 / BLACKBOARD_PLAN_EPOCH_DESIGN (2026-08-14): the plan section is
/// the single-writer plan identity holder — every approved plan carries
/// `plan_id` + `plan_epoch`. Same-plan revisions keep the same epoch and do
/// not rotate the blackboard; a new plan epoch (new plan_id) rotates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanSection {
    /// The currently approved plan's stable identity (`PLAN-…`).
    pub plan_id: Option<String>,
    /// The plan epoch — 0 = no approved plan yet; >0 = current epoch.
    pub plan_epoch: u64,
    pub goal: Option<String>,
    pub steps: Vec<PlanStep>,
    pub analysis: Vec<String>,
    pub decisions: Vec<String>,
    pub auth_grants: Vec<String>,
}

/// Main agent writes: results, observations, errors, auth_requests.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecSection {
    pub results: Vec<String>,
    pub observations: Vec<String>,
    pub errors: Vec<String>,
    pub auth_requests: Vec<String>,
}

/// Internal retrieval subagent writes: project docs, source ledger.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InternalRetSection {
    pub project_docs: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// External retrieval subagent writes: web sources, source ledger.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExternalRetSection {
    pub web_sources: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// Assurance writes: gate decisions, orientation checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateLog {
    pub gate_decisions: Vec<String>,
    pub orientation_checks: Vec<String>,
}

/// 编辑动作区 (blackboard partition, 2026-08-08): one deterministic file-edit
/// record — the line-range delta of a successful file-edit tool call. Written
/// by the controller after execution ("实际变动" 才记); never model-written.
/// Timestamps mirror the journal event's timestamp (events carry one, the
/// in-memory blackboard does not — so the record carries its own).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditRecord {
    pub file: String,
    /// Line count of the tool's `old_string` arg (0 for new-file creation).
    pub old_lines: usize,
    /// Line count of the tool's `new_string` arg.
    pub new_lines: usize,
    /// ISO 8601 timestamp (journal event timestamp of the tool_completed).
    pub timestamp: String,
}

/// 工具动作区 (blackboard partition, 2026-08-08): one classified tool action
/// per executed tool call, folded by category (read / edit / terminal /
/// retrieval) and timestamped — the model looks back via blackboard_read.
/// Denied calls record nothing (the action did not happen).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolActionRecord {
    pub category: String,
    pub tool: String,
    pub timestamp: String,
}

/// The full blackboard with 5 sections + 2 controller-written partitions.
///
/// Read rule: all sections are readable by all agents.
/// Write rule: each section has a single writer (enforced by convention).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Blackboard {
    pub plan: PlanSection,
    pub exec: ExecSection,
    pub internal_ret: InternalRetSection,
    pub external_ret: ExternalRetSection,
    pub gate_log: GateLog,
    /// 编辑动作区 — controller-written after successful file edits.
    pub edits: Vec<EditRecord>,
    /// 工具动作区 — controller-written per executed tool call.
    pub tool_actions: Vec<ToolActionRecord>,
}

impl Blackboard {
    pub fn new() -> Self {
        Blackboard::default()
    }

    /// Capture the current plan-epoch-scoped work state as an archive
    /// snapshot — plan + edits + tool_actions + exec. Gate log, whitelist
    /// and the retrieval partitions are NOT part of an epoch snapshot
    /// (they survive rotation by design).
    pub fn epoch_snapshot(&self, persisted_at: &str) -> EpochSnapshot {
        EpochSnapshot {
            plan_id: self.plan.plan_id.clone(),
            plan_epoch: self.plan.plan_epoch,
            plan: self.plan.clone(),
            edits: self.edits.clone(),
            tool_actions: self.tool_actions.clone(),
            exec: self.exec.clone(),
            persisted_at: persisted_at.to_string(),
        }
    }

    /// Restore a persisted epoch snapshot into the live partitions. The
    /// retrieval partitions / gate log / whitelist are left untouched.
    pub fn restore_epoch_snapshot(&mut self, snapshot: &EpochSnapshot) {
        self.plan = snapshot.plan.clone();
        self.edits = snapshot.edits.clone();
        self.tool_actions = snapshot.tool_actions.clone();
        self.exec = snapshot.exec.clone();
    }

    /// Plan-epoch rotation (ADR-0010 §14.15 / BLACKBOARD_PLAN_EPOCH_DESIGN).
    ///
    /// - Same `plan_id` → same-epoch revision: the plan goal/steps are
    ///   replaced, nothing is cleared, `Ok(None)` is returned (no archive).
    /// - New `plan_id` → new epoch: the old epoch-scoped state is captured
    ///   into an `EpochSnapshot` (`Ok(Some(..))` — the caller archives it),
    ///   the plan section is replaced and edits / tool_actions / exec are
    ///   cleared atomically under one write lock. Gate log, whitelist and
    ///   retrieval partitions survive.
    ///
    /// Identity invariants (v1.15⑧, 2026-08-15 — one-to-one, no misuse):
    /// - `plan_epoch` must be ≥ 1;
    /// - same `plan_id` must reuse the SAME `plan_epoch` (a revision never
    ///   advances the epoch — the caller passes the stored epoch back);
    /// - a new `plan_id` must advance to a STRICTLY GREATER `plan_epoch`
    ///   (timestamp-stamped monotonic numbering, `epoch::next_plan_epoch_
    ///   from_archive`).
    /// Violations return `Err` BEFORE any mutation — the board is untouched.
    pub fn rotate_to_plan(
        &mut self,
        plan_id: String,
        plan_epoch: u64,
        goal: String,
        steps: Vec<String>,
        persisted_at: &str,
    ) -> Result<Option<EpochSnapshot>, PlanEpochError> {
        use crate::blackboard::StepStatus;
        if plan_epoch == 0 {
            return Err(PlanEpochError::ZeroEpoch);
        }
        if let Some(current_id) = self.plan.plan_id.as_deref() {
            if current_id == plan_id.as_str() {
                if plan_epoch != self.plan.plan_epoch {
                    return Err(PlanEpochError::SamePlanEpochMismatch {
                        plan_id,
                        expected: self.plan.plan_epoch,
                        got: plan_epoch,
                    });
                }
                // Same-epoch revision: no rotation, no clearing.
                self.plan.goal = Some(goal);
                self.plan.steps.clear();
                for (i, desc) in steps.into_iter().enumerate() {
                    self.plan.steps.push(PlanStep {
                        id: format!("step-{}", i + 1),
                        description: desc,
                        status: if i == 0 {
                            StepStatus::InProgress
                        } else {
                            StepStatus::Pending
                        },
                    });
                }
                return Ok(None);
            }
            if plan_epoch <= self.plan.plan_epoch {
                return Err(PlanEpochError::NewPlanEpochNotGreater {
                    current_epoch: self.plan.plan_epoch,
                    got: plan_epoch,
                });
            }
        }
        let old = if self.plan.plan_epoch > 0 {
            Some(self.epoch_snapshot(persisted_at))
        } else {
            None
        };
        self.plan = PlanSection {
            plan_id: Some(plan_id),
            plan_epoch,
            goal: Some(goal),
            steps: Vec::new(),
            analysis: Vec::new(),
            decisions: Vec::new(),
            auth_grants: Vec::new(),
        };
        for (i, desc) in steps.into_iter().enumerate() {
            self.plan.steps.push(PlanStep {
                id: format!("step-{}", i + 1),
                description: desc,
                status: if i == 0 {
                    StepStatus::InProgress
                } else {
                    StepStatus::Pending
                },
            });
        }
        self.edits.clear();
        self.tool_actions.clear();
        self.exec = ExecSection::default();
        Ok(old)
    }
}

/// Plan-epoch identity violation (v1.15⑧, 2026-08-15). The one-to-one
/// `plan_id ↔ plan_epoch` mapping is enforced at the rotation boundary;
/// callers that compute epochs via `epoch::next_plan_epoch_from_archive`
/// can only violate it through a programming error, so the API rejects it
/// instead of silently reinterpreting the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanEpochError {
    /// `plan_epoch == 0` — no approved plan has epoch 0.
    ZeroEpoch,
    /// Same `plan_id` passed with a different `plan_epoch`: revisions must
    /// reuse the epoch recorded with the plan_id.
    SamePlanEpochMismatch {
        plan_id: String,
        expected: u64,
        got: u64,
    },
    /// New `plan_id` passed without a strictly greater `plan_epoch`
    /// (timestamp-stamped monotonic numbering forbids reuse/regression).
    NewPlanEpochNotGreater { current_epoch: u64, got: u64 },
}

impl std::fmt::Display for PlanEpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanEpochError::ZeroEpoch => write!(f, "plan_epoch must be ≥ 1"),
            PlanEpochError::SamePlanEpochMismatch {
                plan_id,
                expected,
                got,
            } => write!(
                f,
                "plan_id {plan_id} already owns plan_epoch {expected}; revision passed {got} \
                 (same plan_id must reuse the same epoch)"
            ),
            PlanEpochError::NewPlanEpochNotGreater { current_epoch, got } => write!(
                f,
                "new plan_id must advance beyond current plan_epoch {current_epoch}; got {got}"
            ),
        }
    }
}

impl std::error::Error for PlanEpochError {}

/// Deterministic archive of one plan epoch (v1.15, 2026-08-14): the full
/// path/action records of an epoch at its rotation moment. Written to
/// `.gsa/blackboard/epoch-<plan_epoch>.json`; read back for cross-epoch
/// `blackboard_read` and restore.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochSnapshot {
    pub plan_id: Option<String>,
    pub plan_epoch: u64,
    pub plan: PlanSection,
    pub edits: Vec<EditRecord>,
    pub tool_actions: Vec<ToolActionRecord>,
    pub exec: ExecSection,
    /// When this snapshot was PERSISTED (approval/revision refresh or
    /// rotation). F9 (2026-08-15, BACKLOG 6e 复查遗留): the old name
    /// `rotated_at` misleadingly implied rotation-only — the current-epoch
    /// persistence happens at every approval/revision. `alias` keeps old
    /// archives loadable (the JSON key is `persisted_at` for new writes).
    #[serde(alias = "rotated_at")]
    pub persisted_at: String,
}

/// Thread-safe wrapper around the Blackboard.
///
/// All agents share the same `SharedBlackboard` via `Arc<SharedBlackboard>`.
pub struct SharedBlackboard {
    inner: RwLock<Blackboard>,
}

impl SharedBlackboard {
    pub fn new() -> Self {
        SharedBlackboard {
            inner: RwLock::new(Blackboard::new()),
        }
    }

    /// Read the entire blackboard (shared access).
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Blackboard> {
        self.inner.read().unwrap()
    }

    /// Mutate the blackboard (exclusive access).
    pub fn write(&self) -> std::sync::RwLockWriteGuard<'_, Blackboard> {
        self.inner.write().unwrap()
    }
}

impl Default for SharedBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_blackboard_read_write() {
        let bb = SharedBlackboard::new();

        // Write
        {
            let mut w = bb.write();
            w.plan.goal = Some("test goal".into());
            w.plan.steps.push(PlanStep {
                id: "step-1".into(),
                description: "do something".into(),
                status: StepStatus::Pending,
            });
        }

        // Read
        let r = bb.read();
        assert_eq!(r.plan.goal.as_deref(), Some("test goal"));
        assert_eq!(r.plan.steps.len(), 1);
    }

    #[test]
    fn gate_log_append() {
        let bb = SharedBlackboard::new();
        {
            let mut w = bb.write();
            w.gate_log.gate_decisions.push("IPG: pass".into());
            w.gate_log
                .orientation_checks
                .push("checkpoint: no stagnation".into());
        }
        let r = bb.read();
        assert_eq!(r.gate_log.gate_decisions.len(), 1);
        assert_eq!(r.gate_log.orientation_checks.len(), 1);
    }
}
