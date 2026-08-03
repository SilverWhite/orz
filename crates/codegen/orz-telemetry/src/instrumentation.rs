//! Instrumentation stubs.

/// Tracing target for instrumentation log messages.
pub const TARGET: &str = "orz_telemetry_instrumentation";

/// Returns a no-op timer guard.
pub fn timer(_name: &str) -> NoopTimer {
    NoopTimer
}

pub struct NoopTimer;

impl Drop for NoopTimer {
    fn drop(&mut self) {
        // No-op — telemetry disabled
    }
}
