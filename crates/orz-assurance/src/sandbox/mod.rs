//! Sandbox primitives — Windows Job Object containment.
//!
//! Ported from Python `assurance/job_object_supervisor.py` (spec reference),
//! following the CREATE_SUSPENDED → AssignProcessToJobObject → ResumeThread
//! pattern (JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT 2026-07-31): the child
//! is created suspended, bound into the job, then resumed — closing the
//! post-creation race window in which a child could escape the job.
//!
//! Fail-closed contract: on non-Windows platforms the supervisor refuses to
//! operate (`UnsupportedPlatform`); containment must then come from the
//! platform-native mechanism (e.g. orz-sandbox Landlock/Seatbelt).

pub mod job_object;

pub use job_object::{JobObjectSupervisor, SandboxError};
