//! Assurance gates — decision hot path ported from Python `assurance/`.
//!
//! Phase 2 modules:
//! - `ipg` — instruction provenance gate (source classification + injection scan)
//! - `tool_availability` — mechanical tool availability probe + gate decision
//!
//! Python side retains receipt/context construction and jsonschema validation
//! as conformance suite + schema authority. This crate implements the pure
//! decision logic only.

pub mod ipg;
pub mod tool_availability;
