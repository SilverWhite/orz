//! Telemetry engine for Grok Build sessions: product events + Mixpanel emission +
//! Sentry error reporting + OpenTelemetry tracing + structured unified log.
//!
//! Extracted from `xai-file-utils` per review feedback so telemetry has
//! its own ownership boundary (see CODEOWNERS) and so downstream consumers
//! that only want event tracking + inference metrics no longer pull in
//! Mixpanel/HTTP/identity dependencies.

//! Dead modules removed in fusion cleanup (Slice #13): `appender`,
//! `context`, `debug_log`, `hooks_log`, `prompt_timing`, `sampling_log`,
//! `sentry`, `unified_log` — none reachable from live code (enums/events/
//! external/instrumentation/memory_log/memory_telemetry/session_ctx/
//! session_metrics/client/config/http/id/otel_layer/otlp_http/redact_common).

pub mod client;
pub mod config;
pub mod enums;
pub mod events;
pub mod external;
pub mod http;
pub mod id;
pub mod instrumentation;
pub mod memory_log;
pub mod memory_telemetry;
pub mod otel_layer;
pub(crate) mod otlp_http;
pub(crate) mod redact_common;
pub mod session_ctx;
pub mod session_metrics;

pub use client::{
    Metadata, TelemetryClient, UserContext, init, init_if_needed, is_enabled,
    is_session_metrics_enabled,
};
pub use events::TelemetryEvent;
pub use session_ctx::{
    EmitterOrigin, TelemetryCtx, emit_event, emit_event_with_origin, log_event, log_session_event,
    log_session_event_with_origin, with_session_ctx,
};
