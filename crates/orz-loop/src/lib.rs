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
pub mod action_ledger;
mod agent_loop;
pub mod agents;
pub mod blackboard;
pub(crate) mod citation_validation;
pub mod controller;
mod diagnostic_coverage;
pub mod gateway;
pub mod host;
pub mod orientation;
pub mod prompt;
pub mod relay;
pub mod summary;
pub mod tool;
pub mod tool_probe;

// Re-export core types
pub use controller::AgentLoopController;
pub use host::LoopHost;
