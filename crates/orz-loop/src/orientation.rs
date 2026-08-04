//! OrientationMonitor — per-turn checkpoint injection.
//!
//! Design note (2026-08-04): Python has no cooldown — checkpoints fire on
//! `fixed_step_interval` with step semantics. Rust keeps parity: the monitor
//! emits exactly one checkpoint per turn (interval 1). Cooldown is deferred
//! to Phase 3.

use orz_assurance::orientation::checkpoint::{
    build_checkpoint, Checkpoint, CheckpointTrigger,
};

/// Monitors agent orientation and emits checkpoint events.
#[derive(Debug, Clone)]
pub struct OrientationMonitor {
    step: u64,
    interval: u64,
}

impl OrientationMonitor {
    pub fn new() -> Self {
        Self {
            step: 0,
            interval: 1,
        }
    }

    pub fn with_interval(interval: u64) -> Self {
        Self { step: 0, interval }
    }

    /// Whether a checkpoint should fire at the current step.
    pub fn should_fire(&self) -> bool {
        self.step % self.interval == 0
    }

    /// Build the checkpoint for the current step and advance the counter.
    /// Trigger is `FixedStepInterval` (Python parity; `PreHandoff` is reserved
    /// for the handoff path).
    pub fn next_checkpoint(&mut self, task_id: &str) -> Checkpoint {
        let checkpoint = build_checkpoint(
            task_id,
            CheckpointTrigger::FixedStepInterval {
                step_index: self.step,
            },
            None,
            None,
            None,
        );
        self.step += 1;
        checkpoint
    }

    pub fn step(&self) -> u64 {
        self.step
    }
}

impl Default for OrientationMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_fires_at_step_interval() {
        let mut monitor = OrientationMonitor::with_interval(2);
        // step 0 fires
        assert!(monitor.should_fire());
        // step 1 does not
        let _ = monitor.next_checkpoint("TASK");
        assert!(!monitor.should_fire());
        let _ = monitor.next_checkpoint("TASK");
        // step 2 fires again
        assert!(monitor.should_fire());
    }

    #[test]
    fn next_checkpoint_advances_step() {
        let mut monitor = OrientationMonitor::new();
        let c1 = monitor.next_checkpoint("RUN-1");
        assert_eq!(c1.checkpoint_id(), "ORIENT-RUN-1-0000");
        assert_eq!(c1.trigger.step_index(), 0);
        let c2 = monitor.next_checkpoint("RUN-1");
        assert_eq!(c2.checkpoint_id(), "ORIENT-RUN-1-0001");
        assert_eq!(monitor.step(), 2);
    }

    #[test]
    fn checkpoint_block_is_neutral() {
        let monitor = OrientationMonitor::new();
        let mut m = monitor.clone();
        let c = m.next_checkpoint("RUN-1");
        let block = c.message_block();
        assert!(block.contains("当前正在做什么？"));
        // Forbidden fields must never appear in the injected block.
        for field in orz_assurance::orientation::checkpoint::FORBIDDEN_ORIENTATION_FIELDS {
            assert!(!block.contains(field), "{field}");
        }
    }
}
