//! Retrieval subsystem modules — split from `controller.rs` per
//! CONTROLLER_SPLIT_DESIGN_2026-08-29 (batch B1). Mechanical extraction
//! only — behavior, events and journal chain unchanged.

pub(crate) mod activation;
pub(crate) mod dispatch;
pub(crate) mod disposition;
pub(crate) mod effort;
pub(crate) mod evidence;
pub(crate) mod mode;
pub(crate) mod projection;
