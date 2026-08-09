//! Inquiry trigger counters — IP3b (§4.6.3): the 4 判定点 state machine
//! feeding the neutral INFO_SUFFICIENCY inquiry.
//!
//! Semantics (2026-08-04 定稿, design doc §4.6.3):
//! - 4 counters per agent instance (main agent + retrieval subagents are
//!   configured identically and independently, §4.6.4).
//! - Any counter STRICTLY above its threshold fires the same inquiry; at the
//!   trigger instant ALL 4 counters reset to zero (implicit cooldown — no
//!   re-trigger until thresholds re-accumulate).
//! - Main agent feeds tool-level events (ToolDispatcher wrapper); subagents
//!   feed semantic actions (one `run_retrieval` = 1 action) with tool-level
//!   counting disabled inside subagents.
//! - Thresholds finalized at Phase 3 wiring (ADR-0005): output_repeats > 10,
//!   tool_calls > 10, actions > 10, rounds > 8.

/// Which 判定点 crossed its threshold. Order matters: `any_over` reports the
/// first hit in this fixed order (§4.6.3 列举顺序, reproducible).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerReason {
    OutputRepeats,
    ToolCalls,
    Actions,
    Rounds,
}

impl TriggerReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerReason::OutputRepeats => "output_repeats",
            TriggerReason::ToolCalls => "tool_calls",
            TriggerReason::Actions => "actions",
            TriggerReason::Rounds => "rounds",
        }
    }
}

/// Threshold defaults — finalized at Phase 3 wiring (ADR-0005); output
/// threshold follows the stagnation defaults (strictly greater than 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InquiryThresholds {
    pub output_repeats: u32,
    pub tool_calls: u32,
    pub actions: u32,
    pub rounds: u32,
}

pub const DEFAULT_THRESHOLDS: InquiryThresholds = InquiryThresholds {
    output_repeats: 10,
    tool_calls: 10,
    actions: 10,
    rounds: 8,
};

/// The 4 判定点 counters for one agent instance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InquiryCounters {
    pub output_repeats: u32,
    pub tool_calls: u32,
    pub actions: u32,
    pub rounds: u32,
}

impl InquiryCounters {
    /// Main agent: one model round executed.
    pub fn feed_round(&mut self) {
        self.rounds += 1;
    }

    /// Main agent: one tool call dispatched (tool-level counting).
    pub fn feed_tool_call(&mut self) {
        self.tool_calls += 1;
        self.actions += 1;
    }

    /// Subagent: one semantic action completed (a `run_retrieval` = 1 action;
    /// rounds advance with it while budget_turns == 1). Tool-level counting
    /// stays disabled inside subagents (§4.6.4).
    pub fn feed_semantic_action(&mut self) {
        self.actions += 1;
        self.rounds += 1;
    }

    /// Output threshold: feed the max repeated-content measure observed so
    /// far (the counter accumulates the max, not the sum).
    pub fn feed_output_repeats(&mut self, repeats: u32) {
        self.output_repeats = self.output_repeats.max(repeats);
    }

    /// Strictly-greater-than check across the 4 判定点 in fixed order.
    /// Returns the first reason whose counter exceeds its threshold.
    pub fn any_over(&self, thresholds: &InquiryThresholds) -> Option<TriggerReason> {
        if self.output_repeats > thresholds.output_repeats {
            return Some(TriggerReason::OutputRepeats);
        }
        if self.tool_calls > thresholds.tool_calls {
            return Some(TriggerReason::ToolCalls);
        }
        if self.actions > thresholds.actions {
            return Some(TriggerReason::Actions);
        }
        if self.rounds > thresholds.rounds {
            return Some(TriggerReason::Rounds);
        }
        None
    }

    /// Trigger-instant reset — all 4 counters zeroed so the cooldown is
    /// implicit: re-accumulation is required before any re-trigger.
    pub fn reset_all(&mut self) {
        *self = Self::default();
    }
}

/// Scan a subagent close response for the completion check decision token.
/// Line-wise, case-insensitive; the response is free-form model text so the
/// decision is best-effort evidence — `not_detected` when no token matches.
pub fn parse_completion_decision(text: &str) -> &'static str {
    for line in text.lines() {
        let line = line.trim().to_lowercase();
        if line.starts_with("yes") {
            return "yes";
        }
        if line.starts_with("no") {
            return "no";
        }
        if line.starts_with("uncertain") {
            return "uncertain";
        }
    }
    "not_detected"
}
