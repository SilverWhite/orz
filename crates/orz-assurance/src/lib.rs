// orz-assurance — assurance kernel for the orz CLI agent workbench.
//
// Ported from Python assurance/ (spec reference).
// Modules: gates, orientation, journal, session, sandbox, credential, permit.
//
// Phase 1: empty skeleton.

// Future module declarations:
// pub mod gates;
// pub mod orientation;
// pub mod journal;
// pub mod session;
// pub mod sandbox;
// pub mod credential;
// pub mod permit;

#[derive(Debug, thiserror::Error)]
pub enum AssuranceError {
    #[error("gate blocked")]
    GateBlocked,
    #[error("journal error: {0}")]
    Journal(#[from] std::io::Error),
}
