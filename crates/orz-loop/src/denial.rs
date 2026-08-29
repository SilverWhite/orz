//! Denial state machine — IP2a circuit-breaker types — batch N4 of the
//! controller split second round (CONTROLLER_SPLIT_DESIGN_2026-08-29 §3.5).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

/// IP2a denial counter state (D-3; ADR-0010 §3.5.4 / V11-IMPL-012): the
/// circuit breaker counts CONSECUTIVE TOOL-CALL ROUNDS whose denials share
/// one normalized key — not individual tool calls. A round with any
/// successful tool, a denial key change, or a permission policy revision
/// change resets the count. The total-denial ceiling (old 10) is deleted:
/// anti-runaway is owned by the 120-round budget (FUS-BUDGET), the breaker
/// only corrects tool-belief/availability.
#[derive(Debug, Default)]
pub(crate) struct DenialState {
    pub(crate) consecutive_rounds: u32,
    /// Normalized key of the last counted denial round; used to reset on
    /// key change.
    pub(crate) last_key: Option<DenialKey>,
}

/// Normalized denial key (ADR-0010 §3.5.4): same key across rounds is what
/// accumulates; tool, reason code or policy revision changes reset it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DenialKey {
    pub(crate) tool_name: String,
    pub(crate) reason_code: String,
    /// GAP-DENIAL-POLICY-REVISION (2026-08-12): the controller's live
    /// `policy_revision` (u64, aligned with the ACAF binding) — a bump is a
    /// key change, so the breaker resets. First production increment source
    /// = Slice 3 ModeChangeTicket.
    pub(crate) policy_revision: u64,
}

/// Feedback from a host tool call for the round-level denial aggregator.
/// `None` is neutral — timeout, tool error and whitelist-refused calls are
/// neither successes nor denials: they must not reset the streak AND must
/// not count as a deny round (ADR-0010 §3.5.4: "用户取消、timeout、tool
/// error 与 permission deny 分开记账").
#[derive(Debug)]
pub(crate) enum PolicyFeedback {
    /// The call was refused by the permission gate, with its normalized key.
    Denied(DenialKey),
    /// The call executed successfully — resets the consecutive streak.
    Succeeded,
}

/// IP2a circuit-breaker threshold (D-3 — the 3-consecutive value shared by
/// Claude Code's maxConsecutive; the 10-total ceiling is deleted per
/// ADR-0010 §3.5.4).
pub const DENIAL_BREAKER_CONSECUTIVE: u32 = 3;

#[cfg(test)]
mod tests {
    /// P0-C S3 前置审查修复 (F4): `PolicyDenialSource::as_str` and the
    /// serde snake_case wire names must never drift — both feed the same
    /// journal/console contract (`permission|acaf|retrieval_mode|taint`).
    #[test]
    fn policy_denial_source_as_str_matches_serde_snake_case() {
        use crate::host::PolicyDenialSource;
        for source in [
            PolicyDenialSource::Permission,
            PolicyDenialSource::Acaf,
            PolicyDenialSource::RetrievalMode,
            PolicyDenialSource::Taint,
        ] {
            let serialized = serde_json::to_string(&source).expect("serde round-trip");
            assert_eq!(
                serialized,
                format!("\"{}\"", source.as_str()),
                "as_str must agree with the serde snake_case wire name"
            );
        }
    }
}
