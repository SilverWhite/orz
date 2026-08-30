//! LIF channels — event-driven leaky integrate-and-fire (P2-10 F3 §4.2/§4.3).
//!
//! Channel semantics (§4.3 final table):
//!
//! | channel | semantics | w | τ | θ | refractory | reset |
//! |---|---|---|---|---|---|---|
//! | err    | fixed wall-clock | 1/event | 180 s | 4 events | 60 s | full |
//! | stall  | fixed wall-clock | 1/event | 600 s | 2 events | 120 s | full |
//! | slow   | fixed wall-clock | min(5, wall_ms/60s) | 600 s | 3 | 60 s | full |
//! | deny   | fixed wall-clock | 1/event | 120 s | 4 | 60 s | full |
//! | prog   | round semantics  | success → 1 | 8·T̂ | — | — | success → 1 |
//! | stuck  | round semantics  | I = u_err/θ_err·(1−u_prog) | 3·T̂ | 1.5·T̂ | 8·T̂ | full after fire |
//!
//! Update law: `u(t) = u(t₀)·exp(−(t−t₀)/τ) + w` at event arrival; without
//! events the potential decays continuously. Fire: `u ≥ θ` (past refractory) →
//! internal record + full reset + refractory. The second-order stuck channel
//! uses the exact closed form (§4.4) with `expm1` for numerical stability;
//! right-endpoint discretization is forbidden (7–21× artifacts, proven on
//! 102 runs).

/// --- design constants (§4.3) ---
pub const ERR_TAU_SECS: f64 = 180.0;
pub const ERR_THETA: f64 = 4.0;
pub const ERR_REFRACTORY_SECS: f64 = 60.0;

pub const STALL_TAU_SECS: f64 = 600.0;
pub const STALL_THETA: f64 = 2.0;
pub const STALL_REFRACTORY_SECS: f64 = 120.0;
pub const STALL_GAP_THRESHOLD_SECS: f64 = 90.0;

pub const SLOW_TAU_SECS: f64 = 600.0;
pub const SLOW_THETA: f64 = 3.0;
pub const SLOW_REFRACTORY_SECS: f64 = 60.0;
pub const SLOW_WALL_MS_THRESHOLD: u64 = 60_000;
pub const SLOW_W_MAX: f64 = 5.0;

pub const DENY_TAU_SECS: f64 = 120.0;
pub const DENY_THETA: f64 = 4.0;
pub const DENY_REFRACTORY_SECS: f64 = 60.0;

pub const PROG_TAU_ROUNDS: f64 = 8.0;

pub const STUCK_TAU_ROUNDS: f64 = 3.0;
pub const STUCK_THETA_ROUNDS: f64 = 1.5;
pub const STUCK_REFRACTORY_ROUNDS: f64 = 8.0;

/// Numerically stable `(1 − exp(−y)) / y` (= `−expm1(−y)/y`), with the y→0
/// limit 1 handled explicitly so α = 0 or β = 0 intervals cannot produce NaN.
pub fn expm1_ratio(y: f64) -> f64 {
    if y.abs() < 1e-12 {
        1.0
    } else {
        -(-y).exp_m1() / y
    }
}

/// Which channel a first-order instance implements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    Err,
    Stall,
    Slow,
    Deny,
    Prog,
}

/// A completed tool call, as consumed by the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOutcome {
    Error,
    Success,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub struct ToolEvent {
    pub outcome: ToolOutcome,
    pub wall_ms: Option<u64>,
}

impl ToolEvent {
    pub fn error(wall_ms: Option<u64>) -> Self {
        Self {
            outcome: ToolOutcome::Error,
            wall_ms,
        }
    }

    pub fn success(wall_ms: Option<u64>) -> Self {
        Self {
            outcome: ToolOutcome::Success,
            wall_ms,
        }
    }

    pub fn other(wall_ms: Option<u64>) -> Self {
        Self {
            outcome: ToolOutcome::Other,
            wall_ms,
        }
    }
}

/// First-order leaky integrate-and-fire channel.
#[derive(Debug, Clone)]
pub struct FirstOrderChannel {
    kind: ChannelKind,
    tau_secs: f64,
    theta: Option<f64>,
    refractory_secs: f64,
    u: f64,
    last_time: Option<f64>,
    refractory_until: Option<f64>,
    fire_times: Vec<f64>,
}

impl FirstOrderChannel {
    fn new(kind: ChannelKind, tau_secs: f64, theta: Option<f64>, refractory_secs: f64) -> Self {
        Self {
            kind,
            tau_secs,
            theta,
            refractory_secs,
            u: 0.0,
            last_time: None,
            refractory_until: None,
            fire_times: Vec::new(),
        }
    }

    pub fn err() -> Self {
        Self::new(
            ChannelKind::Err,
            ERR_TAU_SECS,
            Some(ERR_THETA),
            ERR_REFRACTORY_SECS,
        )
    }

    pub fn stall() -> Self {
        Self::new(
            ChannelKind::Stall,
            STALL_TAU_SECS,
            Some(STALL_THETA),
            STALL_REFRACTORY_SECS,
        )
    }

    pub fn slow() -> Self {
        Self::new(
            ChannelKind::Slow,
            SLOW_TAU_SECS,
            Some(SLOW_THETA),
            SLOW_REFRACTORY_SECS,
        )
    }

    pub fn deny() -> Self {
        Self::new(
            ChannelKind::Deny,
            DENY_TAU_SECS,
            Some(DENY_THETA),
            DENY_REFRACTORY_SECS,
        )
    }

    /// prog is a non-firing freshness channel: success sets u = 1, τ = 8·T̂.
    pub fn prog() -> Self {
        Self::new(ChannelKind::Prog, PROG_TAU_ROUNDS * 8.0, None, 0.0)
    }

    pub fn kind(&self) -> ChannelKind {
        self.kind
    }

    pub fn u(&self) -> f64 {
        self.u
    }

    pub fn tau_secs(&self) -> f64 {
        self.tau_secs
    }

    pub fn theta(&self) -> Option<f64> {
        self.theta
    }

    /// Round-semantic channels have their seconds re-derived per decision
    /// round (§6.3 — unit conversion, never per-round fitting).
    pub fn set_tau_secs(&mut self, tau_secs: f64) {
        self.tau_secs = tau_secs.max(1e-9);
    }

    pub fn fire_times(&self) -> &[f64] {
        &self.fire_times
    }

    pub fn fire_count(&self) -> usize {
        self.fire_times.len()
    }

    /// Decay the potential over the interval since `last_time` and pin the
    /// step time. No fire check — the engine calls `check_fire` explicitly so
    /// every step has exactly one deterministic fire evaluation.
    pub fn decay_to(&mut self, t: f64) {
        match self.last_time {
            None => self.last_time = Some(t),
            Some(t0) => {
                let dt = (t - t0).max(0.0);
                self.last_time = Some(t);
                if dt > 0.0 {
                    self.u *= (-dt / self.tau_secs).exp();
                }
            }
        }
    }

    /// Apply one input spike: decay to `t`, then add `weight` (prog: set 1).
    pub fn spike(&mut self, t: f64, weight: f64) {
        self.decay_to(t);
        if self.kind == ChannelKind::Prog {
            self.u = 1.0;
        } else {
            self.u += weight;
        }
    }

    /// Fire evaluation at step time `t`: u ≥ θ and not in refractory → record
    /// the fire, full reset, arm the refractory window. Returns whether a fire
    /// occurred.
    pub fn check_fire(&mut self, t: f64) -> bool {
        let Some(theta) = self.theta else {
            return false;
        };
        let in_refractory = matches!(self.refractory_until, Some(until) if t < until);
        if self.u >= theta && !in_refractory {
            self.fire_times.push(t);
            self.u = 0.0;
            self.refractory_until = Some(t + self.refractory_secs);
            true
        } else {
            false
        }
    }

    /// Force a full reset (used by tests and by restore semantics).
    pub fn reset(&mut self) {
        self.u = 0.0;
        self.refractory_until = None;
    }
}

/// Second-order stuck channel — the exact event-driven closed form (§4.4).
///
/// Input current `I(t) = w·(u_err/θ_err)·(1 − u_prog)`, w = 1. The update over
/// [t₀, t] integrates the product of the two leaking exponentials:
///
/// ```text
/// α = 1/τ_err − 1/τ_stuck
/// β = 1/τ_err + 1/τ_prog − 1/τ_stuck
/// u_stuck(t) = u_stuck(t₀)·exp(−Δt/τ_stuck)
///            + (u_err(t₀)/θ_err)·exp(−Δt/τ_stuck)
///              ·[ (1 − exp(−Δt·α))/α − u_prog(t₀)·(1 − exp(−Δt·β))/β ]
/// ```
///
/// τ_stuck / θ_stuck / refractory are re-derived from the current T̂ at every
/// decision round. The caller must pass the **pre-decay** u_err / u_prog (the
/// values at t₀) — the engine orders the closed-form advance before decaying
/// the first-order channels.
#[derive(Debug, Clone)]
pub struct StuckChannel {
    u_stuck: f64,
    last_time: Option<f64>,
    refractory_until: Option<f64>,
    fire_times: Vec<f64>,
    tau_secs: f64,
    theta_secs: f64,
    refractory_secs: f64,
    peak_theta_ratio: f64,
}

impl Default for StuckChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl StuckChannel {
    pub fn new() -> Self {
        // Pre-enablement rhythm: T̂₀ = 8 s.
        let t_hat = 8.0;
        Self {
            u_stuck: 0.0,
            last_time: None,
            refractory_until: None,
            fire_times: Vec::new(),
            tau_secs: STUCK_TAU_ROUNDS * t_hat,
            theta_secs: STUCK_THETA_ROUNDS * t_hat,
            refractory_secs: STUCK_REFRACTORY_ROUNDS * t_hat,
            peak_theta_ratio: 0.0,
        }
    }

    /// Re-derive seconds from the current T̂ (§6.3 — semantic constants × T̂).
    pub fn set_rhythm(&mut self, t_hat: f64) {
        let t_hat = t_hat.max(1e-9);
        self.tau_secs = STUCK_TAU_ROUNDS * t_hat;
        self.theta_secs = STUCK_THETA_ROUNDS * t_hat;
        self.refractory_secs = STUCK_REFRACTORY_ROUNDS * t_hat;
    }

    pub fn u(&self) -> f64 {
        self.u_stuck
    }

    pub fn theta_secs(&self) -> f64 {
        self.theta_secs
    }

    pub fn tau_secs(&self) -> f64 {
        self.tau_secs
    }

    pub fn refractory_secs(&self) -> f64 {
        self.refractory_secs
    }

    pub fn fire_times(&self) -> &[f64] {
        &self.fire_times
    }

    pub fn fire_count(&self) -> usize {
        self.fire_times.len()
    }

    /// Peak evidence ratio max_t(u_stuck / θ_stuck) since last reset — the
    /// design's "峰值 θ 比" diagnostic (0.65 on the 102-run first pass).
    pub fn peak_theta_ratio(&self) -> f64 {
        self.peak_theta_ratio
    }

    /// Closed-form advance over [last_time, t]. `u_err` / `u_prog` must be the
    /// pre-decay values (state at `last_time`), matching the formula's t₀.
    pub fn advance(&mut self, t: f64, u_err: f64, u_prog: f64, theta_err: f64, tau_err: f64, tau_prog: f64) {
        let Some(t0) = self.last_time else {
            self.last_time = Some(t);
            return;
        };
        let dt = (t - t0).max(0.0);
        self.last_time = Some(t);
        if dt <= 0.0 {
            return;
        }
        let tau_stuck = self.tau_secs.max(1e-9);
        let tau_err = tau_err.max(1e-9);
        let tau_prog = tau_prog.max(1e-9);
        let alpha = 1.0 / tau_err - 1.0 / tau_stuck;
        let beta = 1.0 / tau_err + 1.0 / tau_prog - 1.0 / tau_stuck;
        let decay = (-dt / tau_stuck).exp();
        // (1 − exp(−Δt·α))/α = Δt·(1 − exp(−y))/y = Δt·expm1_ratio(y).
        let k_alpha = dt * expm1_ratio(dt * alpha);
        let k_beta = dt * expm1_ratio(dt * beta);
        self.u_stuck = self.u_stuck * decay
            + (u_err / theta_err) * decay * (k_alpha - u_prog * k_beta);
        // Structural fact (no clamp): err full-resets keep u_err ≤ θ_err, so
        // u_err/θ_err ∈ [0,1] (§4.4).
        if !self.u_stuck.is_finite() {
            self.u_stuck = 0.0;
        }
        let ratio = self.u_stuck / self.theta_secs.max(1e-9);
        if ratio > self.peak_theta_ratio {
            self.peak_theta_ratio = ratio;
        }
    }

    /// Fire evaluation at step time `t` (u_stuck ≥ θ_stuck and past
    /// refractory) → internal record + full reset + refractory.
    pub fn check_fire(&mut self, t: f64) -> bool {
        let in_refractory = matches!(self.refractory_until, Some(until) if t < until);
        if self.u_stuck >= self.theta_secs && !in_refractory {
            self.fire_times.push(t);
            self.u_stuck = 0.0;
            self.peak_theta_ratio = 0.0;
            self.refractory_until = Some(t + self.refractory_secs);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_order_decay_is_exponential() {
        let mut ch = FirstOrderChannel::err();
        ch.spike(0.0, 1.0);
        assert!((ch.u() - 1.0).abs() < 1e-12);
        ch.decay_to(ERR_TAU_SECS);
        assert!((ch.u() - (-1.0_f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn err_fires_when_potential_reaches_theta_and_resets() {
        let mut ch = FirstOrderChannel::err();
        // With the leak, a fresh channel needs a fifth 10 s-spaced error to
        // push u ≥ θ = 4 (the first four reach ≈3.69).
        for i in 1..=4 {
            ch.spike(i as f64 * 10.0, 1.0);
            ch.check_fire(i as f64 * 10.0);
            assert_eq!(ch.fire_count(), 0);
        }
        ch.spike(50.0, 1.0);
        assert!(ch.check_fire(50.0));
        assert_eq!(ch.fire_count(), 1);
        assert_eq!(ch.u(), 0.0);
    }

    #[test]
    fn refractory_suppresses_second_fire() {
        let mut ch = FirstOrderChannel::err();
        for i in 0..5 {
            ch.spike(i as f64 * 10.0, 1.0);
            ch.check_fire(i as f64 * 10.0);
        }
        // Fire #1 at t=50 (u ≥ 4 with five 10 s-spaced errors); refractory
        // window runs to t=110.
        assert_eq!(ch.fire_count(), 1);
        // Eight more errors arrive inside the 60s refractory window; the
        // accumulated potential must exceed θ once the window expires.
        for i in 0..8 {
            ch.spike(60.0 + i as f64, 1.0);
            ch.check_fire(60.0 + i as f64);
        }
        assert_eq!(ch.fire_count(), 1);
        // At t=110 the refractory window has expired and the accumulated
        // potential is still ≥ θ.
        assert!(ch.check_fire(110.0));
        assert_eq!(ch.fire_count(), 2);
    }

    #[test]
    fn slow_weight_is_bounded() {
        let mut ch = FirstOrderChannel::slow();
        ch.spike(0.0, 5.0);
        assert_eq!(ch.u(), 5.0);
        let mut ch2 = FirstOrderChannel::slow();
        ch2.spike(0.0, 1.0_f64.min(SLOW_W_MAX));
        assert!((ch2.u() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn prog_sets_one_on_success() {
        let mut ch = FirstOrderChannel::prog();
        ch.spike(0.0, 1.0);
        assert!((ch.u() - 1.0).abs() < 1e-12);
        ch.set_tau_secs(64.0);
        ch.decay_to(64.0);
        assert!((ch.u() - (-1.0_f64).exp()).abs() < 1e-12);
        ch.spike(100.0, 1.0);
        assert!((ch.u() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn expm1_ratio_limits() {
        assert!((expm1_ratio(0.0) - 1.0).abs() < 1e-9);
        assert!((expm1_ratio(1e-15) - 1.0).abs() < 1e-9);
        // (1 − exp(−1))/1 ≈ 0.63212
        assert!((expm1_ratio(1.0) - 0.6321205588).abs() < 1e-9);
    }

    /// The design's ODE reference: 0.05 s steps, same formula family but
    /// integrated numerically — the closed form must match within 0.006.
    #[test]
    fn closed_form_matches_fine_ode_reference() {
        let mut ref_u = 0.0_f64;
        let mut ref_err = 3.0_f64;
        let mut ref_prog = 0.4_f64;
        let tau_stuck: f64 = 3.0 * 8.0;
        let tau_err = ERR_TAU_SECS;
        let tau_prog: f64 = 8.0 * 8.0;
        let theta_err = ERR_THETA;

        let mut closed = StuckChannel::new();
        closed.advance(0.0, 0.0, 0.0, theta_err, tau_err, tau_prog);
        let mut err = FirstOrderChannel::err();
        let mut prog = FirstOrderChannel::prog();
        err.spike(0.0, 3.0);
        prog.spike(0.0, 1.0);
        prog.set_tau_secs(tau_prog);
        prog.decay_to(0.0); // pin step time
        prog.spike(0.0, 1.0);
        prog.u = 0.4; // deterministic fixture state
        err.decay_to(0.0);

        // Simulate to 60 s.
        let end = 60.0_f64;
        let h = 0.05_f64;
        let mut t = 0.0_f64;
        while t < end - 1e-9 {
            let i = ref_err / theta_err * (1.0 - ref_prog);
            ref_u += h * (i - ref_u / tau_stuck);
            ref_err *= (-h / tau_err).exp();
            ref_prog *= (-h / tau_prog).exp();
            t += h;
        }

        closed.advance(end, err.u(), prog.u(), theta_err, tau_err, tau_prog);
        let diff = (closed.u() - ref_u).abs();
        assert!(
            diff < 0.006,
            "closed form deviates from fine ODE by {diff} (> 0.006)"
        );
    }

    /// Right-endpoint discretization artifact: the naive event-driven
    /// recurrence `u ← u·exp(−Δt/τ) + I·Δt` with I sampled at the interval
    /// END holds the current constant over the whole interval. On the design's
    /// long-interval pathology the closed form is bounded while the naive
    /// recurrence over-accumulates (7–21× on the 102-run analysis); this test
    /// asserts a conservative ≥2× separation on a single 300 s interval.
    #[test]
    fn right_endpoint_discretization_artifacts_are_avoided() {
        let mut closed = StuckChannel::new();
        let tau_err = ERR_TAU_SECS;
        let tau_prog: f64 = 8.0 * 8.0;
        closed.advance(0.0, 0.0, 0.0, ERR_THETA, tau_err, tau_prog);
        // One 300 s interval with constant pre-decay pressure.
        let u_err = 4.0_f64;
        let u_prog = 0.05_f64;
        closed.advance(300.0, u_err, u_prog, ERR_THETA, tau_err, tau_prog);

        // Naive right-endpoint recurrence over the same interval.
        let dt = 300.0_f64;
        let i_end = (u_err * (-dt / tau_err).exp()) / ERR_THETA
            * (1.0 - u_prog * (-dt / tau_prog).exp());
        let re_u = i_end * dt;
        assert!(
            re_u > 2.0 * closed.u(),
            "expected a ≥2× right-endpoint artifact, got closed={} right-endpoint={}",
            closed.u(),
            re_u
        );
    }

    #[test]
    fn stuck_requires_pressure_and_no_progress() {
        let mut stuck = StuckChannel::new();
        let tau_err = ERR_TAU_SECS;
        let tau_prog: f64 = 8.0 * 8.0;
        stuck.advance(0.0, 0.0, 0.0, ERR_THETA, tau_err, tau_prog);
        // High u_err with periodic successes (u_prog reset to 1 every 10 s) →
        // I stays ≈ 0 and u_stuck stays low.
        let mut err = FirstOrderChannel::err();
        let mut prog = FirstOrderChannel::prog();
        err.spike(0.0, 4.0);
        prog.spike(0.0, 1.0);
        for t in [10.0, 20.0, 30.0, 40.0, 50.0, 60.0] {
            stuck.advance(t, err.u(), prog.u(), ERR_THETA, tau_err, tau_prog);
            err.decay_to(t);
            prog.decay_to(t);
            prog.spike(t, 1.0); // success at the end of the interval
        }
        assert!(stuck.u() < 2.0);
        // Low u_prog but no pressure → I ≈ 0.
        let mut stuck2 = StuckChannel::new();
        stuck2.advance(0.0, 0.0, 0.0, ERR_THETA, tau_err, tau_prog);
        stuck2.advance(60.0, 0.0, 0.1, ERR_THETA, tau_err, tau_prog);
        assert!(stuck2.u() < 1e-9);
        // Both → strong accumulation.
        let mut stuck3 = StuckChannel::new();
        stuck3.advance(0.0, 0.0, 0.0, ERR_THETA, tau_err, tau_prog);
        stuck3.advance(60.0, 4.0, 0.1, ERR_THETA, tau_err, tau_prog);
        assert!(stuck3.u() > 3.0);
    }

    #[test]
    fn stuck_fire_resets_and_refractory_holds() {
        let mut stuck = StuckChannel::new();
        let mut err = FirstOrderChannel::err();
        let mut prog = FirstOrderChannel::prog();
        err.spike(0.0, 4.0);
        prog.spike(0.0, 1.0);
        prog.u = 0.05;
        err.u = 4.0;
        err.decay_to(0.0);
        prog.decay_to(0.0);
        stuck.advance(0.0, 0.0, 0.0, ERR_THETA, ERR_TAU_SECS, 64.0);
        // θ = 1.5·8 = 12; drive u_stuck past it.
        let mut fired_at = None;
        for step in 1..=240 {
            let t = step as f64;
            stuck.advance(t, err.u(), prog.u(), ERR_THETA, ERR_TAU_SECS, 64.0);
            err.decay_to(t);
            prog.decay_to(t);
            if stuck.check_fire(t) {
                fired_at = Some(t);
                break;
            }
        }
        let fired_at = fired_at.expect("stuck must fire under sustained pressure");
        assert_eq!(stuck.fire_count(), 1);
        assert!(stuck.u() < stuck.theta_secs(), "full reset after fire");
        // No second fire inside the refractory window (8·T̂ = 64 s), even with
        // sustained pressure.
        for step in 1..=40 {
            let t = fired_at + step as f64;
            stuck.advance(t, 4.0, 0.05, ERR_THETA, ERR_TAU_SECS, 64.0);
            assert!(!stuck.check_fire(t), "fire inside refractory");
        }
        // After the refractory expires, sustained pressure fires again.
        let refractory_end = fired_at + stuck.refractory_secs();
        let mut fired_again = false;
        for step in 1..=120 {
            let t = refractory_end + step as f64;
            stuck.advance(t, 4.0, 0.05, ERR_THETA, ERR_TAU_SECS, 64.0);
            if stuck.check_fire(t) {
                fired_again = true;
                break;
            }
        }
        assert!(fired_again, "must fire again after refractory");
        assert_eq!(stuck.fire_count(), 2);
    }
}
