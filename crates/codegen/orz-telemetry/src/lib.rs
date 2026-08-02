// orz-telemetry — Phase 2 compatibility shim.
// Provides no-op stubs for all xai-grok-telemetry symbols referenced by orz-shell.
#![allow(unused_imports, unused_variables, dead_code)]

pub mod unified_log;
pub mod events;
pub mod session_ctx;
pub mod id;
pub mod enums;
pub mod config;
pub mod memory_log;
pub mod external;
pub mod client;
pub mod debug_log;
pub mod sentry;
pub mod otel_layer;
pub mod instrumentation;
pub mod sampling_log;
pub mod hooks_log;
pub mod session_metrics;
pub mod memory_telemetry;
pub mod context;
pub mod prompt_timing;
pub mod appender;
pub mod redact_common;
pub mod orz_auth_stub;
