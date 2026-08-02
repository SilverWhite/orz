use crate::events::TelemetryEvent;
pub fn log_event<E: TelemetryEvent>(_event: E) {}
pub fn log_session_event<E: TelemetryEvent>(_event: E) {}
pub fn log_event_dual<E: TelemetryEvent>(_event: E) {}
pub fn with_session_ctx<F, R>(_ctx: TelemetryCtx, f: F) -> R where F: FnOnce() -> R { f() }
pub fn begin_prompt_id() -> Option<String> { None }
pub struct TelemetryCtx;
impl TelemetryCtx { pub fn new() -> Self { Self } }
