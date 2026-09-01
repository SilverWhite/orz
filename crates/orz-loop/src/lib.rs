//! orz-loop — Self-built agent loop engine.
//!
//! Replaces the Grok Sampler (~10k lines) with ~1k lines of clean,
//! single-responsibility components. See AGENT_LOOP_REDESIGN_v0.1.
//!
//! Architecture (Codex discipline: thin core, unidirectional deps):
//!   orz-host → orz-loop → orz-assurance
//!
//! No Grok crate depends on orz-loop.

pub mod acaf;
mod acaf_flow;
pub mod action_ledger;
mod agent_loop;
pub mod agents;
pub mod blackboard;
pub(crate) mod checkpoint;
mod compact;
pub mod console;
mod console_exec;
pub(crate) mod console_mode;
pub mod controller;
mod delivery;
mod denial;
pub mod dep_graph;
pub mod diagnostics;
pub mod entities;
pub mod epoch;
pub(crate) mod failure_target;
pub mod gateway;
pub mod host;
mod host_exec;
pub(crate) mod mechanical_audit;
pub mod orientation;
pub(crate) mod planning;
pub mod prompt;
pub mod relay;
mod retrieval;
pub mod summary;
pub mod tool;
pub mod tool_probe;

#[cfg(test)]
pub(crate) mod controller_test_support;

// Re-export core types
pub use controller::AgentLoopController;
pub use host::LoopHost;
