//! PlanStateMachine — the 8-state planning lifecycle.
//!
//! Ported from Python `plan_mode.PlanStateMachine` (§3.3). Valid transitions:
//!
//! ```text
//! IDLE → PLANNING
//! PLANNING → AWAITING_PLAN_APPROVAL | FAILED
//! AWAITING_PLAN_APPROVAL → EXECUTING | REVISING | REJECTED
//! REVISING → PLANNING
//! EXECUTING → COMPLETED | FAILED
//! ```

use crate::AssuranceError;

use super::artifact::PlanArtifact;

/// Plan lifecycle states (8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanState {
    Idle,
    Planning,
    AwaitingPlanApproval,
    Executing,
    Completed,
    Rejected,
    Failed,
    Revising,
}

/// Approval decision (Python `PlanApprovalDecision`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanApprovalDecision {
    Approve,
    Revise,
    Reject,
}

impl PlanApprovalDecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanApprovalDecision::Approve => "approve",
            PlanApprovalDecision::Revise => "revise",
            PlanApprovalDecision::Reject => "reject",
        }
    }
}

/// One approval record, appended to history and journaled.
#[derive(Debug, Clone)]
pub struct PlanApprovalRecord {
    pub plan_id: String,
    pub decision: PlanApprovalDecision,
    pub authority: String,
    pub timestamp: String,
}

/// The planning lifecycle state machine for a single run.
#[derive(Debug, Clone)]
pub struct PlanStateMachine {
    pub state: PlanState,
    pub planning_policy: String,
    pub approval_policy: String,
    pub plan_versions: Vec<String>,
    pub current_plan: Option<PlanArtifact>,
    pub approval_history: Vec<PlanApprovalRecord>,
}

impl Default for PlanStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl PlanStateMachine {
    pub fn new() -> Self {
        Self {
            state: PlanState::Idle,
            planning_policy: "none".to_string(),
            approval_policy: "manual".to_string(),
            plan_versions: Vec::new(),
            current_plan: None,
            approval_history: Vec::new(),
        }
    }

    pub fn state(&self) -> PlanState {
        self.state
    }

    /// IDLE | REVISING → PLANNING.
    pub fn enter_planning(&mut self, policy: Option<&str>) -> Result<(), AssuranceError> {
        if !matches!(self.state, PlanState::Idle | PlanState::Revising) {
            return Err(self.invalid_transition("enter_planning"));
        }
        if let Some(policy) = policy {
            self.planning_policy = policy.to_string();
        }
        self.state = PlanState::Planning;
        Ok(())
    }

    /// PLANNING → AWAITING_PLAN_APPROVAL.
    pub fn submit_plan(&mut self, artifact: PlanArtifact) -> Result<(), AssuranceError> {
        if self.state != PlanState::Planning {
            return Err(self.invalid_transition("submit_plan"));
        }
        let hash = artifact.compute_sha256();
        self.plan_versions.push(hash);
        self.current_plan = Some(artifact);
        self.state = PlanState::AwaitingPlanApproval;
        Ok(())
    }

    /// AWAITING_PLAN_APPROVAL → EXECUTING (plan approval, level 1).
    pub fn approve(
        &mut self,
        authority: &str,
        execution_policy: Option<&str>,
    ) -> Result<PlanApprovalRecord, AssuranceError> {
        self.require_awaiting()?;
        let plan_id = self.current_plan.as_ref().unwrap().plan_id.clone();
        let record = PlanApprovalRecord {
            plan_id,
            decision: PlanApprovalDecision::Approve,
            authority: authority.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Some(plan) = self.current_plan.as_mut() {
            plan.approval_decision = "approve".to_string();
            plan.approval_timestamp = record.timestamp.clone();
            plan.approval_authority = authority.to_string();
            plan.selected_approval_policy = execution_policy
                .unwrap_or("manual")
                .to_string();
        }
        if let Some(policy) = execution_policy {
            self.approval_policy = policy.to_string();
        }
        self.approval_history.push(record.clone());
        self.state = PlanState::Executing;
        Ok(record)
    }

    /// AWAITING_PLAN_APPROVAL → REVISING → PLANNING (immediate, §3.3).
    pub fn revise(&mut self, authority: &str) -> Result<PlanApprovalRecord, AssuranceError> {
        self.require_awaiting()?;
        let record = PlanApprovalRecord {
            plan_id: self.current_plan.as_ref().unwrap().plan_id.clone(),
            decision: PlanApprovalDecision::Revise,
            authority: authority.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        // Mirror approve(): write the decision back to the artifact so the
        // artifact state matches the approval history (Python parity).
        if let Some(plan) = self.current_plan.as_mut() {
            plan.approval_decision = "revise".to_string();
            plan.approval_timestamp = record.timestamp.clone();
            plan.approval_authority = authority.to_string();
        }
        self.approval_history.push(record.clone());
        self.state = PlanState::Revising;
        self.state = PlanState::Planning; // immediate transition per §3.3
        Ok(record)
    }

    /// AWAITING_PLAN_APPROVAL → REJECTED (terminal).
    pub fn reject(&mut self, authority: &str) -> Result<PlanApprovalRecord, AssuranceError> {
        self.require_awaiting()?;
        let record = PlanApprovalRecord {
            plan_id: self.current_plan.as_ref().unwrap().plan_id.clone(),
            decision: PlanApprovalDecision::Reject,
            authority: authority.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        // Mirror approve(): write the decision back to the artifact (Python
        // parity — the artifact hash includes the approval fields).
        if let Some(plan) = self.current_plan.as_mut() {
            plan.approval_decision = "reject".to_string();
            plan.approval_timestamp = record.timestamp.clone();
            plan.approval_authority = authority.to_string();
        }
        self.approval_history.push(record.clone());
        self.state = PlanState::Rejected;
        Ok(record)
    }

    /// EXECUTING → COMPLETED.
    pub fn complete(&mut self) -> Result<(), AssuranceError> {
        if self.state != PlanState::Executing {
            return Err(self.invalid_transition("complete"));
        }
        self.state = PlanState::Completed;
        Ok(())
    }

    /// PLANNING | EXECUTING → FAILED (terminal).
    pub fn fail(&mut self) -> Result<(), AssuranceError> {
        if !matches!(self.state, PlanState::Planning | PlanState::Executing) {
            return Err(self.invalid_transition("fail"));
        }
        self.state = PlanState::Failed;
        Ok(())
    }

    fn require_awaiting(&self) -> Result<(), AssuranceError> {
        if self.state != PlanState::AwaitingPlanApproval {
            return Err(self.invalid_transition("approval"));
        }
        if self.current_plan.is_none() {
            return Err(AssuranceError::InvariantViolation(
                "no plan to approve".to_string(),
            ));
        }
        Ok(())
    }

    fn invalid_transition(&self, action: &str) -> AssuranceError {
        AssuranceError::InvariantViolation(format!(
            "cannot {action} from state {state:?}",
            state = self.state
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::artifact::{PlanSection, PlanArtifact};

    fn section(title: &str) -> PlanSection {
        PlanSection {
            title: title.to_string(),
            content_md: format!("{title} 内容"),
            evidence_status: "observed".to_string(),
            source_refs: Vec::new(),
        }
    }

    fn artifact() -> PlanArtifact {
        PlanArtifact::new(
            "PLAN-1",
            "TASK-1",
            "RUN-1",
            "/workspace",
            "manual",
            vec![
                section("前期调查"),
                section("具体计划"),
                section("具体设计"),
                section("实施方案"),
            ],
        )
    }

    #[test]
    fn full_approve_lifecycle() {
        let mut sm = PlanStateMachine::new();
        assert_eq!(sm.state(), PlanState::Idle);
        sm.enter_planning(None).unwrap();
        sm.submit_plan(artifact()).unwrap();
        assert_eq!(sm.state(), PlanState::AwaitingPlanApproval);
        let record = sm.approve("user", Some("manual")).unwrap();
        assert_eq!(record.decision, PlanApprovalDecision::Approve);
        assert_eq!(sm.state(), PlanState::Executing);
        sm.complete().unwrap();
        assert_eq!(sm.state(), PlanState::Completed);
        assert_eq!(sm.plan_versions.len(), 1);
        assert_eq!(sm.approval_history.len(), 1);
    }

    #[test]
    fn revise_returns_to_planning() {
        let mut sm = PlanStateMachine::new();
        sm.enter_planning(None).unwrap();
        sm.submit_plan(artifact()).unwrap();
        let record = sm.revise("user").unwrap();
        assert_eq!(record.decision, PlanApprovalDecision::Revise);
        assert_eq!(sm.state(), PlanState::Planning);
    }

    #[test]
    fn reject_is_terminal() {
        let mut sm = PlanStateMachine::new();
        sm.enter_planning(None).unwrap();
        sm.submit_plan(artifact()).unwrap();
        let record = sm.reject("user").unwrap();
        assert_eq!(record.decision, PlanApprovalDecision::Reject);
        assert_eq!(sm.state(), PlanState::Rejected);
    }

    #[test]
    fn invalid_transitions_rejected() {
        let mut sm = PlanStateMachine::new();
        assert!(sm.submit_plan(artifact()).is_err()); // IDLE → submit
        sm.enter_planning(None).unwrap();
        assert!(sm.approve("user", None).is_err()); // PLANNING → approve
        assert!(sm.fail().is_ok());
        assert!(sm.complete().is_err()); // FAILED → complete
        assert!(sm.enter_planning(None).is_err()); // FAILED → planning
    }
}
