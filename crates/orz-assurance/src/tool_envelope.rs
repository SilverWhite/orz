//! F1 typed tool envelope — the shared Result contract for the mechanical
//! layer's tool surface (P2-10 MECHANICAL-LAYER-MATH-CALCULUS §2.1,
//! ADR-0010 §14.47).
//!
//! ```text
//! type Result a = Ok { summary: Text ≤ 200 B, cap: Cap, payload: a, pointer: Pointer? }
//!               | Fail { step, code, message: Text ≤ 200 B, trace_id }
//! ```
//!
//! - success is always "summary + cap + pointer": the model reads the
//!   summary to decide and pulls the full content through the pointer;
//! - pure-subterm errors (arg validation / no_match / ambiguous) are typed
//!   Fail values with `step=arg_validation`; effect errors (not started /
//!   timeout kill / gate refusal) keep the fail-closed envelope; a terminal
//!   `exit_code ≠ 0` is a structured VALUE, never a Fail (§2.2 ④);
//! - bounds are enforced at construction (UTF-8-safe truncation + marker).

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
    pub fn new(summary: impl Into<String>, cap: Value, payload: Value, pointer: Option<Pointer>) -> Self {
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

/// The typed failure envelope (Fail step / code / bounded message / trace_id).
#[derive(Debug, Clone, Serialize)]
pub struct FailEnvelope {
    pub step: FailStep,
    pub code: String,
    pub message: String,
    pub trace_id: String,
}

impl FailEnvelope {
    pub fn new(
        step: FailStep,
        code: impl Into<String>,
        message: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            step,
            code: code.into(),
            message: enforce_bound(message.into(), MESSAGE_MAX_BYTES),
            trace_id: trace_id.into(),
        }
    }

    pub fn arg_validation(code: impl Into<String>, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(FailStep::ArgValidation, code, message, trace_id)
    }

    pub fn gate(code: impl Into<String>, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(FailStep::Gate, code, message, trace_id)
    }

    pub fn execution(code: impl Into<String>, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(FailStep::Execution, code, message, trace_id)
    }

    pub fn delivery(code: impl Into<String>, message: impl Into<String>, trace_id: impl Into<String>) -> Self {
        Self::new(FailStep::Delivery, code, message, trace_id)
    }

    /// Serialize fail-loud (审查处理 F12): a serialization failure must
    /// surface as an error, never silently degrade to `Null`.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }
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
        let env = FailEnvelope::new(
            FailStep::ArgValidation,
            "no_match",
            long.clone(),
            "trace-1",
        );
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
