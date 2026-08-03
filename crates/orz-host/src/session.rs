//! Session bootstrap and lifecycle.
//! - Workspace trust verification (before any project config is loaded)
//! - Security envelope initialization (HMAC)
//! - Journal recorder creation + run_preflight event
//! - Session close → run_finished + journal seal + receipt

// TODO: Implement session bootstrap (Phase 1)
