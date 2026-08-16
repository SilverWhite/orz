//! Plan mode — planning/execution separation for the main agent.
//!
//! Ported from Python `assurance/plan_mode.py` (R15, spec reference).
//! The plan state machine carries planning/execution separation — the design
//! decision that replaced the archived Pro/Flash dual-model idea
//! (INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2 §4.5).
//!
//! Scope: `artifact` (4-section structured plan + sha256) and `state_machine`
//! (8-state lifecycle, plan/action two-level approval). ProcessUsageSampler /
//! VSCodeTitleUpdater / terminal status line stay on the TUI/host side
//! (Phase 3).

pub mod artifact;
pub mod framework;
pub mod state_machine;

pub use artifact::{PlanArtifact, PlanSection, verify_plan_artifact};
pub use framework::PLAN_FIRST_FRAMEWORK_BLOCK;
pub use state_machine::{PlanApprovalDecision, PlanApprovalRecord, PlanState, PlanStateMachine};
