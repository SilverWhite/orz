//! F1 typed tool envelope — the shared Result contract for the mechanical
//! layer's tool surface (P2-10 MECHANICAL-LAYER-MATH-CALCULUS §2.1,
//! ADR-0010 §14.47).
//!
//! ```text
//! type Result a = Ok { summary: Text ≤ 200 B, cap: Cap, payload: a, pointer: Pointer? }
//!               | Fail { step, code, message: Text ≤ 200 B, trace_id,
//!                        retryable }   -- P2-11 裁决 3：机械分类位
//! ```
//!
//! - success is always "summary + cap + pointer": the model reads the
//!   summary to decide and pulls the full content through the pointer;
//! - pure-subterm errors (arg validation / no_match / ambiguous) are typed
//!   Fail values with `step=arg_validation`; effect errors (not started /
//!   timeout kill / gate refusal) keep the fail-closed envelope; a terminal
//!   `exit_code ≠ 0` is a structured VALUE, never a Fail (§2.2 ④);
//! - bounds are enforced at construction (UTF-8-safe truncation + marker);
//! - `retryable` is a mechanical classification bit derived from the error
//!   code at construction (P2-11 裁决 3) — deterministic families
//!   (scheme/anchor/sealed/cap) are `false`, transient families
//!   (timeout/network) are `true`, unknown codes fail-closed to `false`;
//!   it is a FACT about the code, never a caller-supplied suggestion, and
//!   the fields are private — the bit is only ever set by the constructor,
//!   so the invariant is type-enforced (2026-09-01 审查处理 O1).

use serde::Serialize;
use serde_json::Value;

pub const SUMMARY_MAX_BYTES: usize = 200;
pub const MESSAGE_MAX_BYTES: usize = 200;

/// The four envelope steps (§2.1) — receipt segments map 1:1 to the event
/// chain (F4 §5.4 trace→event isomorphism).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailStep {
    ArgValidation,
    Gate,
    Execution,
    Delivery,
}

impl FailStep {
    pub fn as_str(self) -> &'static str {
        match self {
            FailStep::ArgValidation => "arg_validation",
            FailStep::Gate => "gate",
            FailStep::Execution => "execution",
            FailStep::Delivery => "delivery",
        }
    }
}

/// Typed pointers (§2.1): the model never receives full bodies it did not
/// ask for — the pointer names where the full content lives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Pointer {
    FilePtr {
        path: String,
        hash: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        offset: Option<u64>,
    },
    BoardPtr {
        partition: String,
        selector: String,
    },
    EvidencePtr {
        canonical_url: String,
        fetch_id: String,
    },
    CmdPtr {
        log_path: String,
        span: String,
    },
}

/// The success envelope: summary (≤ 200 B) + cap + payload + optional pointer.
#[derive(Debug, Clone, Serialize)]
pub struct OkEnvelope {
    pub summary: String,
    pub cap: Value,
    pub payload: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pointer: Option<Pointer>,
}

impl OkEnvelope {
    /// Construct with the summary enforced to `SUMMARY_MAX_BYTES` (UTF-8
    /// boundary, never a mid-codepoint cut; an over-limit summary is
    /// truncated and marked — the model must never see a silently
    /// un-bounded summary).
    pub fn new(
        summary: impl Into<String>,
        cap: Value,
        payload: Value,
        pointer: Option<Pointer>,
    ) -> Self {
        Self {
            summary: enforce_bound(summary.into(), SUMMARY_MAX_BYTES),
            cap,
            payload,
            pointer,
        }
    }

    /// Serialize fail-loud (审查处理 F12): a serialization failure must
    /// surface as an error, never silently degrade to `Null`.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// The typed failure envelope (Fail step / code / bounded message / trace_id /
/// mechanical retryability bit). Fields are private: the envelope is an
/// immutable fact — `retryable` is only ever set by the constructor from the
/// code (P2-11 裁决 3), so a caller can neither supply nor override it
/// (2026-09-01 审查处理 O1).
#[derive(Debug, Clone, Serialize)]
pub struct FailEnvelope {
    step: FailStep,
    code: String,
    message: String,
    trace_id: String,
    /// Mechanical classification bit (P2-11 裁决 3, 2026-08-31 采用):
    /// derived from `code` at construction — deterministic failures
    /// (scheme / anchor / sealed / cap families) are `false`, transient
    /// failures (timeout / network families) are `true`, unknown codes
    /// fail-closed to `false`. Never caller-supplied.
    retryable: bool,
}

impl FailEnvelope {
    pub fn new(
        step: FailStep,
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        let code = code.into();
        Self {
            step,
            retryable: retryable_for_code(&code),
            code,
            message: enforce_bound(message.into(), MESSAGE_MAX_BYTES),
            trace_id: trace_id.into(),
        }
    }

    pub fn arg_validation(
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(FailStep::ArgValidation, code, message, trace_id)
    }

    pub fn gate(
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(FailStep::Gate, code, message, trace_id)
    }

    pub fn execution(
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(FailStep::Execution, code, message, trace_id)
    }

    pub fn delivery(
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self::new(FailStep::Delivery, code, message, trace_id)
    }

    pub fn step(&self) -> FailStep {
        self.step
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn trace_id(&self) -> &str {
        &self.trace_id
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }

    /// Serialize fail-loud (审查处理 F12): a serialization failure must
    /// surface as an error, never silently degrade to `Null`.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

/// Deterministic failure-code fragments (P2-11 裁决 3): retrying reproduces
/// the same failure by construction — URL scheme / canonicalization facts,
/// GetPut write-guard anchor facts, retired/sealed tool facts, and hard
/// candidate/budget cap facts.
const DETERMINISTIC_CODE_FRAGMENTS: &[&str] = &[
    "scheme",
    "anchor",
    "order_stale",
    "sealed",
    "retired_tool",
    "cap_exceeded",
    "candidate_count",
    "candidate_url",
    "budget",
];

/// Transient failure-code fragments (P2-11 裁决 3): the failure is a
/// temporal or external condition and retry may succeed — timeout and
/// network-level facts.
const TRANSIENT_CODE_FRAGMENTS: &[&str] = &[
    "timeout",
    "network",
    "dns",
    "connection_refused",
    "unreachable",
];

/// Mechanical retryability classification derived from the error-code FACT —
/// never a suggestion (P2-11 裁决 3, 2026-08-31 采用; 2026-09-01 实施).
///
/// - deterministic failures → `false`: scheme / anchor / sealed / cap
///   families (retrying reproduces the same failure by construction);
/// - transient failures → `true`: timeout / network families (retry may
///   succeed);
/// - unknown codes → `false` (fail-closed: the bit is only `true` when a
///   known transient fact is present).
///
/// Transient facts win over deterministic fragments when both match (e.g. a
/// `network_timeout` code is retryable). The classifier is a closed fact
/// table: extending it is a machine-contract change and must add tests.
pub fn retryable_for_code(code: &str) -> bool {
    if TRANSIENT_CODE_FRAGMENTS.iter().any(|f| code.contains(f)) {
        return true;
    }
    if DETERMINISTIC_CODE_FRAGMENTS
        .iter()
        .any(|f| code.contains(f))
    {
        return false;
    }
    false
}

/// Enforce the mechanical retryability invariant on a serialized Fail
/// envelope value (P2-11 裁决 3, 2026-09-01 审查处理 O2): the bit must
/// always equal `retryable_for_code(code)` — missing or inconsistent bits
/// are recomputed from the code fact; a Fail without a usable code string
/// fails closed to `false`. Non-object values are left untouched (callers
/// must discriminate Fail shapes first).
pub fn enforce_retryable_bit(value: &mut Value) {
    let Value::Object(map) = value else { return };
    let bit = map
        .get("code")
        .and_then(Value::as_str)
        .map(retryable_for_code)
        .unwrap_or(false);
    map.insert("retryable".to_string(), Value::Bool(bit));
}

/// Enforce a UTF-8 byte bound: never split a code point; an over-limit text
/// is truncated at the last safe boundary and marked with `…(truncated)`.
pub fn enforce_bound(text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    let marker = "…(truncated)";
    if max_bytes == 0 {
        return String::new();
    }
    // The marker itself may exceed a tiny cap: return the marker's own
    // UTF-8-safe prefix so the result NEVER exceeds max_bytes (F12).
    if max_bytes < marker.len() {
        let mut cut = max_bytes;
        while cut > 0 && !marker.is_char_boundary(cut) {
            cut -= 1;
        }
        return marker[..cut].to_string();
    }
    let mut cut = max_bytes.saturating_sub(marker.len());
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}{}", &text[..cut], marker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_envelope_summary_is_bounded() {
        let long = "x".repeat(500);
        let env = OkEnvelope::new(long.clone(), Value::Null, Value::Null, None);
        assert!(env.summary.len() <= SUMMARY_MAX_BYTES);
        assert!(env.summary.ends_with("(truncated)"));
        let short = OkEnvelope::new("ok".to_string(), Value::Null, Value::Null, None);
        assert_eq!(short.summary, "ok");
    }

    #[test]
    fn fail_message_is_bounded_and_utf8_safe() {
        let long = "你".repeat(300); // 900 bytes
        let env = FailEnvelope::new(FailStep::ArgValidation, "no_match", long.clone(), "trace-1");
        assert!(env.message.len() <= MESSAGE_MAX_BYTES);
        assert!(env.message.is_char_boundary(env.message.len()));
        assert!(env.message.ends_with("(truncated)"));
    }

    #[test]
    fn steps_serialize_snake_case() {
        assert_eq!(FailStep::ArgValidation.as_str(), "arg_validation");
        assert_eq!(FailStep::Gate.as_str(), "gate");
        assert_eq!(FailStep::Execution.as_str(), "execution");
        assert_eq!(FailStep::Delivery.as_str(), "delivery");
    }

    #[test]
    fn envelope_json_shape() {
        let env = OkEnvelope::new(
            "read ok",
            serde_json::json!({"bytes": 4096}),
            serde_json::json!({"window": {"offset": 0, "content": "…", "truncated": false}}),
            Some(Pointer::FilePtr {
                path: "a.rs".to_string(),
                hash: "ab".repeat(32),
                offset: Some(0),
            }),
        );
        let value = env.to_value();
        let value = value.unwrap();
        assert_eq!(value["summary"], "read ok");
        assert_eq!(value["pointer"]["kind"], "file_ptr");
        assert_eq!(value["pointer"]["path"], "a.rs");
        let fail = FailEnvelope::arg_validation("ambiguous", "multiple matches", "trace-9");
        let fv = fail.to_value().unwrap();
        assert_eq!(fv["step"], "arg_validation");
        assert_eq!(fv["code"], "ambiguous");
        assert_eq!(fv["trace_id"], "trace-9");
        // P2-11 裁决 3: the mechanical classification bit is always present.
        assert_eq!(fv["retryable"], serde_json::Value::Bool(false));
    }

    /// P2-11 裁决 3: `retryable` is derived from the error-code FACT —
    /// deterministic families (scheme / anchor / sealed / cap) are false,
    /// transient families (timeout / network) are true, unknown codes
    /// fail-closed to false.
    #[test]
    fn retryable_is_derived_from_code_facts() {
        // Deterministic families → false (retrying reproduces the failure).
        for code in [
            "unsupported_scheme",
            "bad_url_scheme",
            "content_anchor_mismatch",
            "anchor_required",
            "order_stale",
            "sealed_tool_denied",
            "retired_tool_denied",
            "candidate_cap_exceeded",
            "web_fetch_candidate_count_unbound",
            "web_fetch_candidate_url_missing",
            "round_inject_budget_exceeded",
        ] {
            assert!(!retryable_for_code(code), "{code} must be non-retryable");
        }
        // Transient families → true (retry may succeed).
        for code in [
            "timeout",
            "tool_timeout",
            "wall_clock_timeout",
            "network_error",
            "network_unreachable",
            "dns_failed",
            "connection_refused",
        ] {
            assert!(retryable_for_code(code), "{code} must be retryable");
        }
        // Unknown codes fail-closed → false.
        for code in ["no_match", "pipe_incompatible", "invalid_pattern", ""] {
            assert!(!retryable_for_code(code), "{code:?} must default false");
        }
    }

    /// P2-11 裁决 3: transient facts win over deterministic fragments when a
    /// code carries both (e.g. `network_timeout`) — retry may still succeed.
    #[test]
    fn transient_facts_win_over_deterministic_fragments() {
        for code in [
            "network_timeout",
            "timeout_scheme",
            "dns_anchor_error",
            "network_budget_exceeded",
        ] {
            assert!(
                retryable_for_code(code),
                "{code} must be retryable (transient wins)"
            );
        }
    }

    /// P2-11 裁决 3 (审查处理 O4): the bit is Fail-only — an Ok envelope
    /// must never carry it — and the fragment match is exact lowercase
    /// snake_case vocabulary, not a case-insensitive guess.
    #[test]
    fn ok_envelope_never_carries_retryable_and_match_is_case_sensitive() {
        let ok = OkEnvelope::new("ok", serde_json::Value::Null, serde_json::Value::Null, None);
        let v = ok.to_value().unwrap();
        assert!(v.get("retryable").is_none(), "Ok must not carry retryable");
        assert!(!retryable_for_code("TIMEOUT"));
        assert!(!retryable_for_code("Network_Error"));
        assert!(!retryable_for_code("DNS_FAILED"));
    }

    /// P2-11 裁决 3 (审查处理 O2): at a Fail boundary the bit is recomputed
    /// from the code — missing or inconsistent serialized bits are corrected,
    /// no-code Fails fail closed, non-object values are untouched.
    #[test]
    fn enforce_retryable_bit_normalizes_fail_values() {
        let mut missing = serde_json::json!({
            "step": "execution", "code": "tool_timeout", "message": "x", "trace_id": "t"
        });
        enforce_retryable_bit(&mut missing);
        assert_eq!(missing["retryable"], serde_json::Value::Bool(true));

        let mut inconsistent = serde_json::json!({
            "step": "gate", "code": "content_anchor_mismatch", "message": "x",
            "trace_id": "t", "retryable": true,
        });
        enforce_retryable_bit(&mut inconsistent);
        assert_eq!(inconsistent["retryable"], serde_json::Value::Bool(false));

        let mut no_code = serde_json::json!({
            "step": "execution", "message": "x", "trace_id": "t"
        });
        enforce_retryable_bit(&mut no_code);
        assert_eq!(no_code["retryable"], serde_json::Value::Bool(false));

        let mut not_object = serde_json::json!("boom");
        enforce_retryable_bit(&mut not_object);
        assert_eq!(not_object, serde_json::json!("boom"));
    }

    /// P2-11 裁决 3: the bit rides on the serialized envelope and is computed
    /// at construction — the caller cannot supply or override it.
    #[test]
    fn fail_envelope_carries_mechanical_retryable() {
        let deterministic = FailEnvelope::execution("sealed_tool_denied", "sealed", "t1");
        assert!(!deterministic.retryable());
        let dv = deterministic.to_value().unwrap();
        assert_eq!(dv["retryable"], serde_json::Value::Bool(false));

        let transient = FailEnvelope::execution("network_error", "net down", "t2");
        assert!(transient.retryable());
        let tv = transient.to_value().unwrap();
        assert_eq!(tv["retryable"], serde_json::Value::Bool(true));

        let gate = FailEnvelope::gate("anchor_required", "GetPut anchor needed", "t3");
        assert!(!gate.retryable());

        let unknown = FailEnvelope::arg_validation("no_match", "no match", "t4");
        assert!(!unknown.retryable());
        // O1: getters expose the facts; the fields are not writable.
        assert_eq!(deterministic.code(), "sealed_tool_denied");
        assert_eq!(deterministic.step(), FailStep::Execution);
        assert_eq!(transient.trace_id(), "t2");
        assert_eq!(transient.message(), "net down");
    }

    /// F12: `enforce_bound` must never return a string longer than the cap,
    /// even when the cap is smaller than the marker itself.
    #[test]
    fn enforce_bound_never_exceeds_tiny_caps() {
        assert_eq!(enforce_bound("hello".to_string(), 0), "");
        let tiny = enforce_bound("hello".to_string(), 4); // marker is 14 B
        assert!(tiny.len() <= 4, "got {} bytes: {tiny:?}", tiny.len());
        assert!(tiny.is_char_boundary(tiny.len()));
        let bounded = enforce_bound("你".repeat(50), 8); // marker > cap
        assert!(bounded.len() <= 8);
    }

    #[test]
    fn pointer_kinds_serialize() {
        let pointers = vec![
            Pointer::FilePtr {
                path: "a".into(),
                hash: "0".repeat(64),
                offset: None,
            },
            Pointer::BoardPtr {
                partition: "temporal".into(),
                selector: "now".into(),
            },
            Pointer::EvidencePtr {
                canonical_url: "https://example.com".into(),
                fetch_id: "f-1".into(),
            },
            Pointer::CmdPtr {
                log_path: "/tmp/x.log".into(),
                span: "1:20".into(),
            },
        ];
        for p in pointers {
            let v = serde_json::to_value(&p).unwrap();
            assert!(v.get("kind").is_some(), "{v}");
        }
    }
}
