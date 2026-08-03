//! orz-host — Thin core that bridges Grok providers to the self-built agent loop.
//!
//! Phase 0: Skeleton only — compiles against kept Grok providers.
//! Phase 1+: ACP server, LoopHost implementation, session lifecycle, journal writer.
//!
//! Dependency direction (Codex discipline):
//!   orz-host → {orz-loop, orz-assurance, Grok providers}
//!   No Grok crate depends on orz-host.

pub mod acp_server;
pub mod session;
pub mod approval;
