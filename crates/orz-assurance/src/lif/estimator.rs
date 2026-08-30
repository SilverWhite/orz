//! T̂ — decision-round rhythm estimator (P2-10 F3 §4.5 / 讨论稿 §9.6.2).
//!
//! Deterministic, online, zero-label:
//! - a decision round = an adjacent pair of decision outputs (model output
//!   with non-empty tool calls); the interval covers thinking + tool execution;
//! - bounded internal storage (last 8–32 intervals — enable ≥ 8 samples,
//!   buffer ≤ 32; 审查处理 F14 措辞统一) + a robust center estimate
//!   (median), O(1)/event;
//! - long intervals are truncated at 300 s so a single long tool call cannot
//!   drag the estimate;
//! - init T̂₀ = 8 s (measured upper bound of a typical round), enabled after
//!   ≥ 8 samples, clamped to [3, 600] s (§4.5 / §6.3 anti self-reference).

use std::cmp::Ordering;
use std::collections::VecDeque;

pub const T_HAT_INIT_SECS: f64 = 8.0;
pub const T_HAT_MIN_SECS: f64 = 3.0;
pub const T_HAT_MAX_SECS: f64 = 600.0;
pub const T_HAT_ENABLE_SAMPLES: usize = 8;
pub const INTERVAL_CAP_SECS: f64 = 300.0;
pub const INTERVAL_BUF_CAP: usize = 32;

/// Online median-of-recent-intervals estimator.
#[derive(Debug, Clone)]
pub struct RoundIntervalEstimator {
    intervals: VecDeque<f64>,
}

impl Default for RoundIntervalEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundIntervalEstimator {
    pub fn new() -> Self {
        Self {
            intervals: VecDeque::with_capacity(INTERVAL_BUF_CAP),
        }
    }

    /// Observe one decision interval (seconds). Long intervals are truncated
    /// at `INTERVAL_CAP_SECS`; non-positive intervals are ignored.
    pub fn observe_interval(&mut self, dt_secs: f64) {
        if !dt_secs.is_finite() || dt_secs <= 0.0 {
            return;
        }
        self.intervals.push_back(dt_secs.min(INTERVAL_CAP_SECS));
        if self.intervals.len() > INTERVAL_BUF_CAP {
            self.intervals.pop_front();
        }
    }

    pub fn sample_count(&self) -> usize {
        self.intervals.len()
    }

    /// Enabled once at least `T_HAT_ENABLE_SAMPLES` intervals were observed.
    pub fn enabled(&self) -> bool {
        self.intervals.len() >= T_HAT_ENABLE_SAMPLES
    }

    /// Current estimate: T̂₀ before enablement; the clamped median afterwards.
    pub fn estimate(&self) -> f64 {
        if !self.enabled() {
            return T_HAT_INIT_SECS;
        }
        let mut v: Vec<f64> = self.intervals.iter().copied().collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        let median = if v.len() % 2 == 1 {
            v[v.len() / 2]
        } else {
            (v[v.len() / 2 - 1] + v[v.len() / 2]) / 2.0
        };
        median.clamp(T_HAT_MIN_SECS, T_HAT_MAX_SECS)
    }

    /// `Some(estimate)` only after enablement.
    pub fn estimate_opt(&self) -> Option<f64> {
        self.enabled().then(|| self.estimate())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_uses_8s_until_enabled() {
        let mut est = RoundIntervalEstimator::new();
        assert_eq!(est.estimate(), T_HAT_INIT_SECS);
        assert!(!est.enabled());
        for i in 0..7 {
            est.observe_interval(5.0 + i as f64);
            assert!(!est.enabled());
        }
        assert_eq!(est.estimate(), T_HAT_INIT_SECS);
        est.observe_interval(5.0);
        assert!(est.enabled());
        // Median of [5,5,6,7,8,9,10,11] is 7.5.
        assert_eq!(est.estimate(), 7.5);
    }

    #[test]
    fn median_of_even_and_odd_windows() {
        let mut est = RoundIntervalEstimator::new();
        for i in 0..8 {
            est.observe_interval((i % 4) as f64 + 10.0);
        }
        // values: 10..13 repeated -> sorted 10,10,11,11,12,12,13,13 -> median 11.5
        assert_eq!(est.estimate(), 11.5);
        est.observe_interval(10.0);
        // sorted 10,10,10,11,11,12,12,13,13 -> median 11
        assert_eq!(est.estimate(), 11.0);
    }

    #[test]
    fn long_intervals_are_truncated() {
        let mut est = RoundIntervalEstimator::new();
        for _ in 0..8 {
            est.observe_interval(1000.0);
        }
        assert_eq!(est.estimate(), INTERVAL_CAP_SECS);
    }

    #[test]
    fn estimate_is_clamped() {
        let mut est = RoundIntervalEstimator::new();
        for _ in 0..8 {
            est.observe_interval(1_000_000.0);
        }
        // Intervals are capped at 300 s on observation, so the median is 300.
        assert_eq!(est.estimate(), INTERVAL_CAP_SECS);
        let mut est2 = RoundIntervalEstimator::new();
        for _ in 0..8 {
            est2.observe_interval(0.001);
        }
        assert_eq!(est2.estimate(), T_HAT_MIN_SECS);
    }

    #[test]
    fn buffer_is_bounded() {
        let mut est = RoundIntervalEstimator::new();
        for i in 0..100 {
            est.observe_interval(i as f64);
        }
        assert_eq!(est.sample_count(), INTERVAL_BUF_CAP);
    }

    #[test]
    fn non_positive_intervals_ignored() {
        let mut est = RoundIntervalEstimator::new();
        est.observe_interval(0.0);
        est.observe_interval(-1.0);
        est.observe_interval(f64::NAN);
        assert_eq!(est.sample_count(), 0);
    }
}
