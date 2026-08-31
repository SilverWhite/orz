//! LIF — global time observation sidecar for the mechanical layer
//! (P2-10 MECHANICAL-LAYER-MATH-CALCULUS, F3 §4; ADR-0010 §14.47).
//!
//! This module is a **pure, deterministic, zero-label** computation kernel:
//! - `estimator` — T̂, the online decision-round rhythm estimator (§4.5);
//! - `channels` — first-order leaky-integrate-and-fire channels (err / stall /
//!   slow / deny / prog) and the second-order stuck channel with the exact
//!   event-driven closed form (§4.3/§4.4);
//! - `temporal` — the per-decision-round feature record, semantic domain
//!   labels, domain spikes and the bounded query surface (§3).
//!
//! Invariants (design §1/§4):
//! - the model never writes LIF state; LIF only consumes the event stream,
//!   evolves by its own dynamics and emits internal fire records;
//! - fires stay internal (logs/audit/verification only) — never rendered,
//!   never injected, never in the model surface (user ruling §9.7);
//! - every computation is O(1)/event, deterministic, and free of right-endpoint
//!   discretization for the second-order closed form (§4.4).

pub mod channels;
pub mod estimator;
pub mod temporal;

pub use channels::{
    ChannelKind, DENY_REFRACTORY_SECS, DENY_TAU_SECS, DENY_THETA, ERR_REFRACTORY_SECS,
    ERR_TAU_SECS, ERR_THETA, FirstOrderChannel, PROG_TAU_ROUNDS, SLOW_REFRACTORY_SECS,
    SLOW_TAU_SECS, SLOW_THETA, SLOW_W_MAX, SLOW_WALL_MS_THRESHOLD, STALL_GAP_THRESHOLD_SECS,
    STALL_REFRACTORY_SECS, STALL_TAU_SECS, STALL_THETA, STUCK_REFRACTORY_ROUNDS, STUCK_TAU_ROUNDS,
    STUCK_THETA_ROUNDS, StuckChannel, ToolEvent, ToolOutcome, classify_event_outcome,
    is_denial_code,
};
pub use estimator::{
    INTERVAL_BUF_CAP, INTERVAL_CAP_SECS, RoundIntervalEstimator, T_HAT_ENABLE_SAMPLES,
    T_HAT_INIT_SECS, T_HAT_MAX_SECS, T_HAT_MIN_SECS,
};
pub use temporal::{
    Domain, DomainSpike, Migration, TemporalQuery, TemporalRecord, TemporalState, ToolOutcomeBucket,
};

/// err channel τ semantics (§4.3): fixed wall-clock seconds (spec) or
/// round-semantic `k·T̂` (the counterfactual used by the 102-run comparison:
/// fixed time 22 run/24 fires vs round 11 run/8 fires — round ⊂ time).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ErrTauMode {
    FixedSecs(f64),
    Rounds(f64),
}

impl Default for ErrTauMode {
    fn default() -> Self {
        ErrTauMode::FixedSecs(ERR_TAU_SECS)
    }
}

/// T̂ estimation mode (§4.5 spec = Online; the `Fixed` variant is the
/// first-pass counterfactual — a per-run winsorized scalar — used only by
/// the offline replay diagnostics to reconcile the design's quoted anchors).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum THatMode {
    Online,
    Fixed(f64),
}

impl Default for THatMode {
    fn default() -> Self {
        THatMode::Online
    }
}

/// The full LIF engine: one instance per agent run (or per restored session).
///
/// The engine consumes two kinds of inputs, both with run-relative wall-clock
/// seconds:
/// - `on_decision_round(t)` — a model output with non-empty tool calls
///   (§4.5 decision-round definition);
/// - `on_tool_event(ev)` — a completed tool call with its outcome / wall_ms.
///
/// It owns the rhythm estimator, all channels, and the temporal state machine.
#[derive(Debug, Clone)]
pub struct LifEngine {
    estimator: RoundIntervalEstimator,
    err: FirstOrderChannel,
    stall: FirstOrderChannel,
    slow: FirstOrderChannel,
    deny: FirstOrderChannel,
    prog: FirstOrderChannel,
    stuck: StuckChannel,
    temporal: TemporalState,
    err_tau_mode: ErrTauMode,
    t_hat_mode: THatMode,
    run_t0: Option<f64>,
    last_time: Option<f64>,
    last_decision_t: Option<f64>,
    last_tool_t: Option<f64>,
}

impl Default for LifEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LifEngine {
    /// Fresh engine with all channels at their design constants (§4.3).
    pub fn new() -> Self {
        Self {
            estimator: RoundIntervalEstimator::new(),
            err: FirstOrderChannel::err(),
            stall: FirstOrderChannel::stall(),
            slow: FirstOrderChannel::slow(),
            deny: FirstOrderChannel::deny(),
            prog: FirstOrderChannel::prog(),
            stuck: StuckChannel::new(),
            temporal: TemporalState::new(),
            err_tau_mode: ErrTauMode::default(),
            t_hat_mode: THatMode::default(),
            run_t0: None,
            last_time: None,
            last_decision_t: None,
            last_tool_t: None,
        }
    }

    /// Anchor the run clock at `t0` (the first observed event time). The
    /// engine only ever works in run-relative seconds.
    pub fn set_run_origin(&mut self, t0: f64) {
        self.run_t0 = Some(t0);
        let t0r = 0.0;
        self.last_time = Some(t0r);
    }

    pub fn set_err_tau_mode(&mut self, mode: ErrTauMode) {
        self.err_tau_mode = mode;
    }

    /// Switch the rhythm estimation mode (diagnostic counterfactual only —
    /// production always runs `THatMode::Online`).
    pub fn set_t_hat_mode(&mut self, mode: THatMode) {
        self.t_hat_mode = mode;
    }

    fn current_t_hat(&self) -> f64 {
        match self.t_hat_mode {
            THatMode::Online => self.estimator.estimate(),
            THatMode::Fixed(v) => v.clamp(T_HAT_MIN_SECS, T_HAT_MAX_SECS),
        }
    }

    fn rel(&self, t: f64) -> f64 {
        match self.run_t0 {
            Some(t0) => (t - t0).max(0.0),
            None => t,
        }
    }

    /// A decision round at wall time `t` (§4.5): observe the interval since the
    /// previous decision output, re-derive the round-semantic seconds from the
    /// fresh T̂, decay every channel to `t`, then record the feature row.
    pub fn on_decision_round(&mut self, t: f64) {
        if self.run_t0.is_none() {
            self.set_run_origin(t);
        }
        let t = self.rel(t);
        if matches!(self.t_hat_mode, THatMode::Online)
            && let Some(prev) = self.last_decision_t
        {
            let dt = (t - prev).max(0.0);
            if dt > 0.0 {
                self.estimator.observe_interval(dt);
            }
        }
        self.last_decision_t = Some(t);

        let t_hat = self.current_t_hat();
        self.prog.set_tau_secs(PROG_TAU_ROUNDS * t_hat);
        self.stuck.set_rhythm(t_hat);
        match self.err_tau_mode {
            ErrTauMode::FixedSecs(tau) => self.err.set_tau_secs(tau),
            ErrTauMode::Rounds(k) => self.err.set_tau_secs(k * t_hat),
        }

        self.advance(t);
        self.check_fires(t);
        self.temporal.record_round(
            t,
            self.current_t_hat(),
            self.prog.u(),
            self.err.u(),
            self.stuck.u(),
        );
    }

    /// A completed tool event at wall time `t`.
    pub fn on_tool_event(&mut self, t: f64, event: ToolEvent) {
        if self.run_t0.is_none() {
            self.set_run_origin(t);
        }
        let t = self.rel(t);
        // stall gap detection: interval since the previous tool event > 90 s
        // (§4.3 stall channel, fixed wall-clock interval probe).
        let long_gap = match self.last_tool_t {
            Some(prev) => (t - prev) > STALL_GAP_THRESHOLD_SECS,
            None => false,
        };
        self.last_tool_t = Some(t);

        self.advance(t);
        match event.outcome {
            ToolOutcome::Error => {
                self.err.spike(t, 1.0);
            }
            ToolOutcome::Deny => {
                self.deny.spike(t, 1.0);
            }
            ToolOutcome::Success => {
                self.prog.spike(t, 1.0);
            }
            ToolOutcome::Other => {}
        }
        if let Some(wall_ms) = event.wall_ms
            && wall_ms > SLOW_WALL_MS_THRESHOLD
        {
            let w = ((wall_ms as f64) / 60_000.0).clamp(1.0, SLOW_W_MAX);
            self.slow.spike(t, w);
        }
        if long_gap {
            self.stall.spike(t, 1.0);
        }
        self.check_fires(t);
        self.temporal.observe_tool_outcome(event.outcome);
    }

    /// Decay every channel over the interval since the last step, with the
    /// second-order closed form applied first (using pre-decay u_err / u_prog,
    /// design §4.4).
    fn advance(&mut self, t: f64) {
        let Some(t0) = self.last_time else {
            self.last_time = Some(t);
            return;
        };
        let dt = (t - t0).max(0.0);
        self.last_time = Some(t);
        if dt <= 0.0 {
            return;
        }
        let u_err = self.err.u();
        let u_prog = self.prog.u();
        let tau_err = self.err.tau_secs();
        let tau_prog = self.prog.tau_secs();
        self.stuck
            .advance(t, u_err, u_prog, ERR_THETA, tau_err, tau_prog);
        self.err.decay_to(t);
        self.prog.decay_to(t);
        self.slow.decay_to(t);
        self.stall.decay_to(t);
        self.deny.decay_to(t);
    }

    /// Fire checks for all threshold channels at the current step time.
    fn check_fires(&mut self, t: f64) {
        self.err.check_fire(t);
        self.stall.check_fire(t);
        self.slow.check_fire(t);
        self.deny.check_fire(t);
        self.stuck.check_fire(t);
    }

    pub fn estimator(&self) -> &RoundIntervalEstimator {
        &self.estimator
    }

    pub fn err(&self) -> &FirstOrderChannel {
        &self.err
    }

    pub fn stall(&self) -> &FirstOrderChannel {
        &self.stall
    }

    pub fn slow(&self) -> &FirstOrderChannel {
        &self.slow
    }

    pub fn deny(&self) -> &FirstOrderChannel {
        &self.deny
    }

    pub fn prog(&self) -> &FirstOrderChannel {
        &self.prog
    }

    pub fn stuck(&self) -> &StuckChannel {
        &self.stuck
    }

    pub fn temporal(&self) -> &TemporalState {
        &self.temporal
    }

    pub fn temporal_mut(&mut self) -> &mut TemporalState {
        &mut self.temporal
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_events_feed_deny_channel_and_not_err() {
        // R2 (2026-08-31): five 10 s-spaced deny events fire the deny
        // channel (θ=4, τ=120s) while leaving the err channel untouched —
        // a rejection is not an execution error.
        let mut engine = LifEngine::new();
        for i in 1..=4 {
            engine.on_tool_event(i as f64 * 10.0, ToolEvent::deny(Some(0)));
            assert_eq!(engine.deny().fire_count(), 0);
            assert_eq!(engine.err().fire_count(), 0);
        }
        engine.on_tool_event(50.0, ToolEvent::deny(Some(0)));
        assert_eq!(engine.deny().fire_count(), 1);
        assert_eq!(engine.err().fire_count(), 0);
        assert_eq!(engine.temporal().total_tool_events(), 5);
        assert_eq!(engine.temporal().total_errors(), 0);
    }
}
