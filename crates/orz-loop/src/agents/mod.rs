//! Agents — main agent + retrieval subagents.
//!
//! Phase 2 (2026-08-04): single main agent + two retrieval subagents
//! (internal project-doc retrieval, external web retrieval). The Pro/Flash
//! dual-model design was archived (see INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN
//! v0.2 §4.5). Subagents are structure + scheduling ready; real retrieval
//! semantics (project doc index, browser/web retrieval) are deferred.
//!
//! Blackboard write ownership:
//! - main agent → plan + exec sections (self-consistent, no cross-agent format contract)
//! - internal retrieval subagent → internal_ret section
//! - external retrieval subagent → external_ret section
//! - assurance → gate_log section

pub mod main;
pub mod retrieval;

pub use main::MainAgent;
pub use retrieval::{RetrievalSubagent, SubagentRole, SubagentSpec};
