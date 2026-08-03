//! OrientationMonitor — event-driven checkpoint injection with cooldown.
//!
//! Neutral inquiry checkpoints are injected into the agent context
//! periodically to prevent stagnation and tunnel vision.
//!
//! Phase 1: skeleton only. Full implementation in Step 3.

/// Monitors agent orientation and injects checkpoint events.
pub struct OrientationMonitor;

impl OrientationMonitor {
    pub fn new() -> Self {
        OrientationMonitor
    }

    /// Whether a checkpoint should fire now (respects cooldown).
    pub fn should_fire(&self) -> bool {
        false // Phase 1: never fire
    }
}

impl Default for OrientationMonitor {
    fn default() -> Self {
        Self::new()
    }
}
