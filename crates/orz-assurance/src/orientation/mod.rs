//! Orientation runtime guard — checkpoint + stagnation detection.
//!
//! Ported from Python `assurance/orientation_runtime_guard.py` (spec reference).
//! - `checkpoint` — neutral orientation checkpoint construction + response
//!   verification (trigger types: fixed_step_interval / pre_handoff).
//! - `stagnation` — runtime stagnation guard: consecutive-repeat and ngram-repeat
//!   detection with retry budget and terminal-safe restart packet.
//!
//! Design note (2026-08-04): Python has no cooldown concept; Rust keeps parity —
//! the controller calls the checkpoint once per turn. Cooldown is deferred to
//! Phase 3.

pub mod checkpoint;
pub mod stagnation;
