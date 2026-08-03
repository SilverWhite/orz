//! Session context — no-op log_event.

use crate::events;

/// Log a telemetry event. No-op — never transmits data.
pub fn log_event<E: serde::Serialize>(_event: E) {
    // Telemetry disabled — intentional no-op
}
