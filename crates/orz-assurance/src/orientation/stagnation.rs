//! Runtime stagnation guard — mechanical repetition detection.
//!
//! Ported from Python `assurance/orientation_runtime_guard.py`:
//! `evaluate_runtime_stagnation_guard`. No model invocation, no network:
//! pure token statistics over public outputs.
//!
//! Triggers (strictly greater than threshold):
//! - consecutive identical normalized outputs > `repeated_content_threshold`
//!   → `STAGNATION-CONSECUTIVE-REPEAT`
//! - max n-gram (3..=8) token repeat > `ngram_repeat_threshold`
//!   → `STAGNATION-NGRAM-REPEAT`

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::AssuranceError;

/// Token pattern mirroring Python `[\w一-鿿]+` (Unicode word + CJK).
fn token_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\w\u{4E00}-\u{9FFF}]+").expect("static token regex"))
}

/// Python `_normalize_text`: space-joined tokens, casefolded.
pub fn normalize_text(value: &str) -> String {
    token_regex()
        .find_iter(&value.to_lowercase())
        .map(|m| m.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Python `_tokenize`: token list, casefolded.
pub fn tokenize(value: &str) -> Vec<String> {
    token_regex()
        .find_iter(&value.to_lowercase())
        .map(|m| m.as_str().to_string())
        .collect()
}

/// Python `_max_consecutive_repeated`: max run of identical non-empty values.
pub fn max_consecutive_repeated(values: &[String]) -> u32 {
    let mut best = 0u32;
    let mut previous: Option<&str> = None;
    let mut current = 0u32;
    for value in values {
        if value.is_empty() {
            continue;
        }
        if Some(value.as_str()) == previous {
            current += 1;
        } else {
            previous = Some(value.as_str());
            current = 1;
        }
        best = best.max(current);
    }
    best
}

/// Python `_max_ngram_repeat`: max count of any token n-gram for n in 3..=8.
pub fn max_ngram_repeat(tokens: &[String], min_n: usize, max_n: usize) -> u32 {
    let mut best = 0u32;
    for ngram_size in min_n..=max_n {
        if tokens.len() < ngram_size {
            continue;
        }
        let mut counts: HashMap<&[String], u32> = HashMap::new();
        for index in 0..=(tokens.len() - ngram_size) {
            *counts.entry(&tokens[index..index + ngram_size]).or_insert(0) += 1;
        }
        if let Some(max_count) = counts.values().max() {
            best = best.max(*max_count);
        }
    }
    best
}

/// Terminal-safe restart packet (Python `terminal_safe_restart_packet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartPacket {
    pub packet_kind: &'static str,
    pub included_state: Vec<&'static str>,
    pub runaway_suffix_retained: bool,
}

impl RestartPacket {
    pub fn new() -> Self {
        Self {
            packet_kind: "terminal_safe_restart_packet",
            included_state: vec![
                "task_contract",
                "verified_artifact_ledger",
                "unresolved_questions",
                "last_valid_checkpoint_digest",
            ],
            runaway_suffix_retained: false,
        }
    }
}

impl Default for RestartPacket {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard decision (Python `decision` / `action` pair).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StagnationDecision {
    Continue,
    RestartRequested { packet: RestartPacket },
    HandoffRequired,
}

/// Inputs to the stagnation guard (Python keyword args).
#[derive(Debug, Clone)]
pub struct StagnationInput {
    pub public_outputs: Vec<String>,
    pub retry_count: u32,
    pub retry_budget: u32,
    pub repeated_content_threshold: u32,
    pub ngram_repeat_threshold: u32,
    pub progress_markers: Vec<String>,
}

impl Default for StagnationInput {
    fn default() -> Self {
        Self {
            public_outputs: Vec::new(),
            retry_count: 0,
            retry_budget: 1,
            repeated_content_threshold: 10,
            ngram_repeat_threshold: 10,
            progress_markers: Vec::new(),
        }
    }
}

/// Metrics + reason codes (Python receipt metrics/reason_codes).
#[derive(Debug, Clone, Default)]
pub struct StagnationMetrics {
    pub public_output_count: usize,
    pub token_count: usize,
    pub max_consecutive_repeated_content: u32,
    pub max_ngram_repeat: u32,
    pub progress_marker_count: usize,
    pub reason_codes: Vec<String>,
}

pub const REASON_CONSECUTIVE_REPEAT: &str = "STAGNATION-CONSECUTIVE-REPEAT";
pub const REASON_NGRAM_REPEAT: &str = "STAGNATION-NGRAM-REPEAT";

/// Evaluate the stagnation guard (Python `evaluate_runtime_stagnation_guard`).
///
/// Errors on thresholds below 1 (Python `AssuranceError`; retry values are
/// `u32` so negative inputs are unrepresentable).
pub fn evaluate_runtime_stagnation_guard(
    input: &StagnationInput,
) -> Result<(StagnationDecision, StagnationMetrics), AssuranceError> {
    if input.repeated_content_threshold < 1 || input.ngram_repeat_threshold < 1 {
        return Err(AssuranceError::InvariantViolation(
            "stagnation thresholds must be positive".to_string(),
        ));
    }

    let normalized_outputs: Vec<String> = input
        .public_outputs
        .iter()
        .map(|o| normalize_text(o))
        .collect();
    let tokens = tokenize(&input.public_outputs.join("\n"));
    let max_consecutive = max_consecutive_repeated(&normalized_outputs);
    let max_ngram = max_ngram_repeat(&tokens, 3, 8);
    let marker_count = {
        let mut seen = std::collections::HashSet::new();
        for m in &input.progress_markers {
            seen.insert(m.clone());
        }
        seen.len()
    };

    let mut reason_codes: Vec<String> = Vec::new();
    if max_consecutive > input.repeated_content_threshold {
        reason_codes.push(REASON_CONSECUTIVE_REPEAT.to_string());
    }
    if max_ngram > input.ngram_repeat_threshold {
        reason_codes.push(REASON_NGRAM_REPEAT.to_string());
    }

    let decision = if reason_codes.is_empty() {
        StagnationDecision::Continue
    } else if input.retry_count < input.retry_budget {
        StagnationDecision::RestartRequested {
            packet: RestartPacket::new(),
        }
    } else {
        StagnationDecision::HandoffRequired
    };

    let metrics = StagnationMetrics {
        public_output_count: input.public_outputs.len(),
        token_count: tokens.len(),
        max_consecutive_repeated_content: max_consecutive,
        max_ngram_repeat: max_ngram,
        progress_marker_count: marker_count,
        reason_codes,
    };

    Ok((decision, metrics))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs(outputs: Vec<&str>) -> StagnationInput {
        StagnationInput {
            public_outputs: outputs.into_iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn consecutive_repeat_over_threshold_triggers() {
        // 11 identical outputs > threshold 10.
        let outputs = vec!["同样的输出内容"; 11];
        let (decision, metrics) =
            evaluate_runtime_stagnation_guard(&inputs(outputs)).unwrap();
        assert_eq!(decision, StagnationDecision::RestartRequested { packet: RestartPacket::new() });
        assert_eq!(
            metrics.reason_codes,
            vec![REASON_CONSECUTIVE_REPEAT.to_string()]
        );
    }

    #[test]
    fn ngram_repeat_triggers() {
        // 11 repetitions of a 3-token pattern within one output.
        let pattern = "重复 的 片段 ";
        let output = pattern.repeat(11);
        let (decision, metrics) =
            evaluate_runtime_stagnation_guard(&inputs(vec![output.as_str()])).unwrap();
        assert!(metrics.reason_codes.contains(&REASON_NGRAM_REPEAT.to_string()));
        assert_ne!(decision, StagnationDecision::Continue);
    }

    #[test]
    fn below_threshold_continues() {
        let outputs = vec!["第一轮输出", "第二轮输出", "第三轮输出"];
        let (decision, metrics) =
            evaluate_runtime_stagnation_guard(&inputs(outputs)).unwrap();
        assert_eq!(decision, StagnationDecision::Continue);
        assert!(metrics.reason_codes.is_empty());
    }

    #[test]
    fn restart_within_budget_emits_packet() {
        let outputs = vec!["相同"; 12];
        let mut input = inputs(outputs);
        input.retry_count = 0;
        input.retry_budget = 1;
        let (decision, _) = evaluate_runtime_stagnation_guard(&input).unwrap();
        match decision {
            StagnationDecision::RestartRequested { packet } => {
                assert_eq!(packet.packet_kind, "terminal_safe_restart_packet");
                assert!(packet.included_state.contains(&"task_contract"));
                assert!(!packet.runaway_suffix_retained);
            }
            other => panic!("expected RestartRequested, got {other:?}"),
        }
    }

    #[test]
    fn budget_exhausted_hands_off() {
        let outputs = vec!["相同"; 12];
        let mut input = inputs(outputs);
        input.retry_count = 1;
        input.retry_budget = 1;
        let (decision, _) = evaluate_runtime_stagnation_guard(&input).unwrap();
        assert_eq!(decision, StagnationDecision::HandoffRequired);
    }

    #[test]
    fn threshold_boundary_is_strict_greater_than() {
        // Exactly 10 repeats (== threshold) does not trigger.
        let outputs = vec!["相同内容"; 10];
        let (decision, metrics) =
            evaluate_runtime_stagnation_guard(&inputs(outputs)).unwrap();
        assert_eq!(decision, StagnationDecision::Continue);
        assert_eq!(metrics.max_consecutive_repeated_content, 10);
        assert!(metrics.reason_codes.is_empty());
    }

    #[test]
    fn zero_threshold_rejected() {
        let mut input = inputs(vec!["a"]);
        input.repeated_content_threshold = 0;
        assert!(evaluate_runtime_stagnation_guard(&input).is_err());
    }

    #[test]
    fn chinese_and_ascii_tokens_normalize() {
        assert_eq!(
            normalize_text("Hello世界! 你好"),
            "hello世界 你好"
        );
        let tokens = tokenize("ABC def");
        assert_eq!(tokens, vec!["abc".to_string(), "def".to_string()]);
    }
}
