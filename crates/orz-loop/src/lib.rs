//! orz-loop — Self-built agent loop engine.
//!
//! Replaces the Grok Sampler (~10k lines) with ~1k lines of clean,
//! single-responsibility components. See AGENT_LOOP_REDESIGN_v0.1.
//!
//! Architecture (Codex discipline: thin core, unidirectional deps):
//!   orz-host → orz-loop → orz-assurance
//!
//! No Grok crate depends on orz-loop.

pub mod agents;
pub mod blackboard;
pub mod controller;
pub mod gateway;
pub mod host;
pub mod inquiry;
pub mod orientation;
pub mod prompt;
pub mod relay;
pub mod tool;

// Re-export core types
pub use controller::AgentLoopController;
pub use host::LoopHost;
