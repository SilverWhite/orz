//! Blackboard — structured shared state for multi-agent communication.
//!
//! 5 sections, each with a single writer. All sections are readable by all agents.
//! Write rules enforce single-writer discipline.
//!
//! Single main agent + two retrieval subagents (fusion §4.5): the main agent
//! writes Plan/Exec, each retrieval subagent writes its own section, and the
//! controller writes GateLog.

use std::sync::RwLock;

/// A single step in the execution plan.
#[derive(Debug, Clone)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub status: StepStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

/// Main agent writes: goal, steps, analysis, decisions, auth_grants.
#[derive(Debug, Clone, Default)]
pub struct PlanSection {
    pub goal: Option<String>,
    pub steps: Vec<PlanStep>,
    pub analysis: Vec<String>,
    pub decisions: Vec<String>,
    pub auth_grants: Vec<String>,
}

/// Main agent writes: results, observations, errors, auth_requests.
#[derive(Debug, Clone, Default)]
pub struct ExecSection {
    pub results: Vec<String>,
    pub observations: Vec<String>,
    pub errors: Vec<String>,
    pub auth_requests: Vec<String>,
}

/// Internal retrieval subagent writes: project docs, source ledger.
#[derive(Debug, Clone, Default)]
pub struct InternalRetSection {
    pub project_docs: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// External retrieval subagent writes: web sources, source ledger.
#[derive(Debug, Clone, Default)]
pub struct ExternalRetSection {
    pub web_sources: Vec<String>,
    pub source_ledger: Vec<String>,
    pub response: Option<String>,
}

/// Assurance writes: gate decisions, orientation checks.
#[derive(Debug, Clone, Default)]
pub struct GateLog {
    pub gate_decisions: Vec<String>,
    pub orientation_checks: Vec<String>,
}

/// The full blackboard with all 5 sections.
///
/// Read rule: all sections are readable by all agents.
/// Write rule: each section has a single writer (enforced by convention).
#[derive(Debug, Default)]
pub struct Blackboard {
    pub plan: PlanSection,
    pub exec: ExecSection,
    pub internal_ret: InternalRetSection,
    pub external_ret: ExternalRetSection,
    pub gate_log: GateLog,
}

impl Blackboard {
    pub fn new() -> Self {
        Blackboard::default()
    }
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
