//! Temporal partition state — per-decision-round feature records, semantic
//! domain labels and domain spikes (P2-10 F2 §3; ADR-0010 §14.47).
//!
//! Positioning: a pure **observation record surface**. The domain is a
//! semantic predicate over the continuous features (never learned, never a
//! verdict). Only spikes are persisted (session sidecar, I4); the per-round
//! rows are a runtime query surface with a bounded window.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::channels::ToolOutcome;

pub const RECENT_RECORDS_CAP: usize = 20;
pub const MIGRATION_LOG_CAP: usize = 20;
pub const TOOL_WINDOW: usize = 10;
pub const FEATURE_SERIES_CAP: usize = 20;

/// Semantic domain labels (§3.2). Start is the pre-first-success launch state
/// and is never merged into the other domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Start,
    Normal,
    Pressure,
    LowProgress,
    Stuck,
}

impl Domain {
    pub fn as_str(self) -> &'static str {
        match self {
            Domain::Start => "start",
            Domain::Normal => "normal",
            Domain::Pressure => "pressure",
            Domain::LowProgress => "low_progress",
            Domain::Stuck => "stuck",
        }
    }
}

/// One per-decision-round query row (§3.1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TemporalRecord {
    pub t: f64,
    pub domain: Domain,
    pub entry_round: u64,
    pub dwell_rounds: u64,
    pub u_prog: f64,
    pub u_err: f64,
    pub u_stuck: f64,
    pub t_hat: f64,
    pub err10: f64,
    pub succ10: f64,
}

/// Domain-switch point persisted with the session sidecar (§3.5): only the
/// switch times + target domain; dwell rounds are derived from neighbouring
/// spikes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DomainSpike {
    pub t: f64,
    pub domain: Domain,
}

/// B1 会话化基础（2026-09-03，P2-13 / BLACKBOARD_CONVERSATION_SCOPE_FOLD
/// 设计 R5 conversation-relative 轴）：跨 prompt 续接 LIF 会话轴所需的
/// 最小机器状态快照——决策轮计数（会话相对）、域机器状态（has_success /
/// current_domain / entry_round）与域切换时间线（spikes，`t` 为会话相对
/// 秒，同一次会话内跨 prompt 单调）。
///
/// 有界查询面（records / migrations / features）与 run 节奏估计器/通道
/// 不持久化：每次 run 从当前机器状态与 spike 时间线重建（P2-10 阶段 3
/// 已登记「round/entry_round 只能近似」的边界随本快照关闭——round 与
/// 域驻留状态精确续接）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemporalSessionSnapshot {
    /// 会话内已完成决策轮数（跨 prompt 单调；写入黑板各分区的 round 章
    /// 与 temporal 行同刻度）。
    pub round: u64,
    /// 是否出现过成功（域判定前提；Start 仅在首次成功前出现）。
    pub has_success: bool,
    /// 当前驻留域（跨 prompt 续接，不回归 Start）。
    pub current_domain: Domain,
    /// 当前域段的入域轮（会话相对）。
    pub entry_round: u64,
    /// 域切换时间线（t = 会话相对墙钟秒；同 restore_spikes 语义重建
    /// migrations / migration_count）。
    #[serde(default)]
    pub spikes: Vec<DomainSpike>,
}

/// A domain migration (History query). `recovery` marks a
/// Stuck/LowProgress → Normal transition (§3.2 Recovery).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Migration {
    pub from: Domain,
    pub to: Domain,
    pub at_t: f64,
    pub at_round: u64,
    pub dwell_rounds: u64,
    pub recovery: bool,
}

/// Selector vocabulary for `blackboard.read partition="temporal"` (§3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalQuery {
    Now,
    Recent(u64),
    History,
    Feature(&'static str, u64),
}

/// Tool-outcome bucket used by the err10/succ10 window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOutcomeBucket {
    Error,
    Success,
    Other,
}

impl From<ToolOutcome> for ToolOutcomeBucket {
    fn from(v: ToolOutcome) -> Self {
        match v {
            ToolOutcome::Error => ToolOutcomeBucket::Error,
            ToolOutcome::Success => ToolOutcomeBucket::Success,
            ToolOutcome::Deny => ToolOutcomeBucket::Other,
            ToolOutcome::Other => ToolOutcomeBucket::Other,
        }
    }
}

/// Bounded temporal state machine (§3.4: O(1)/decision, bounded query window).
#[derive(Debug, Clone)]
pub struct TemporalState {
    round: u64,
    has_success: bool,
    current_domain: Domain,
    entry_round: u64,
    records: VecDeque<TemporalRecord>,
    spikes: Vec<DomainSpike>,
    migrations: VecDeque<Migration>,
    /// PULL 自描述（2026-08-31，P2-11 第 1 项）：域迁移的**单调总计数**
    /// （migrations 是有界队列、pop_front 后长度不可作增量基线）——供
    /// `blackboard_read` 增量头的「域迁移 +n」段使用。restore_spikes 重建时
    /// 同步重建。
    migration_count: u64,
    tool_outcomes: VecDeque<ToolOutcomeBucket>,
    features: HashMap<&'static str, VecDeque<f64>>,
    total_tool_events: u64,
    total_errors: u64,
    total_successes: u64,
}

impl Default for TemporalState {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalState {
    pub fn new() -> Self {
        Self {
            round: 0,
            has_success: false,
            current_domain: Domain::Start,
            entry_round: 0,
            records: VecDeque::with_capacity(RECENT_RECORDS_CAP),
            spikes: Vec::new(),
            migrations: VecDeque::with_capacity(MIGRATION_LOG_CAP),
            migration_count: 0,
            tool_outcomes: VecDeque::with_capacity(TOOL_WINDOW),
            features: HashMap::new(),
            total_tool_events: 0,
            total_errors: 0,
            total_successes: 0,
        }
    }

    /// Feed one completed tool outcome into the err10/succ10 window (last 10
    /// tool events, §3.1).
    pub fn observe_tool_outcome(&mut self, outcome: ToolOutcome) {
        self.total_tool_events += 1;
        match outcome {
            ToolOutcome::Error => self.total_errors += 1,
            ToolOutcome::Success => {
                self.total_successes += 1;
                self.has_success = true;
            }
            // A denial is a completed tool call (counts as an event) but is
            // neither an error nor a success for the err10/succ10 window —
            // the deny channel owns its semantic (§4.3).
            ToolOutcome::Deny => {}
            ToolOutcome::Other => {}
        }
        self.tool_outcomes.push_back(outcome.into());
        if self.tool_outcomes.len() > TOOL_WINDOW {
            self.tool_outcomes.pop_front();
        }
    }

    pub fn total_tool_events(&self) -> u64 {
        self.total_tool_events
    }

    pub fn total_errors(&self) -> u64 {
        self.total_errors
    }

    pub fn total_successes(&self) -> u64 {
        self.total_successes
    }

    fn err10(&self) -> f64 {
        self.tool_outcomes
            .iter()
            .filter(|b| **b == ToolOutcomeBucket::Error)
            .count() as f64
            / TOOL_WINDOW as f64
    }

    fn succ10(&self) -> f64 {
        self.tool_outcomes
            .iter()
            .filter(|b| **b == ToolOutcomeBucket::Success)
            .count() as f64
            / TOOL_WINDOW as f64
    }

    /// Semantic domain predicate (§3.2). `theta_stuck` = 1.5·T̂.
    fn label(&self, u_prog: f64, u_err: f64, u_stuck: f64, theta_stuck: f64) -> Domain {
        if !self.has_success {
            return Domain::Start;
        }
        let low_progress = u_prog < 0.5;
        let pressure = u_err >= 2.0 || u_stuck >= theta_stuck;
        match (low_progress, pressure) {
            (false, false) => Domain::Normal,
            (false, true) => Domain::Pressure,
            (true, false) => Domain::LowProgress,
            (true, true) => Domain::Stuck,
        }
    }

    /// Record one decision round row and advance the domain machine.
    pub fn record_round(&mut self, t: f64, t_hat: f64, u_prog: f64, u_err: f64, u_stuck: f64) {
        self.round += 1;
        let theta_stuck = 1.5 * t_hat;
        let domain = self.label(u_prog, u_err, u_stuck, theta_stuck);
        if domain != self.current_domain {
            let dwell = if self.entry_round == 0 {
                0
            } else {
                self.round.saturating_sub(self.entry_round) + 1
            };
            if self.entry_round > 0 {
                let from = self.current_domain;
                let recovery = matches!(
                    (from, domain),
                    (Domain::Stuck | Domain::LowProgress, Domain::Normal)
                );
                self.migrations.push_back(Migration {
                    from,
                    to: domain,
                    at_t: t,
                    at_round: self.round,
                    dwell_rounds: dwell,
                    recovery,
                });
                if self.migrations.len() > MIGRATION_LOG_CAP {
                    self.migrations.pop_front();
                }
                self.migration_count = self.migration_count.saturating_add(1);
            }
            self.spikes.push(DomainSpike { t, domain });
            self.current_domain = domain;
            self.entry_round = self.round;
        }
        let record = TemporalRecord {
            t,
            domain,
            entry_round: self.entry_round,
            dwell_rounds: self.round.saturating_sub(self.entry_round) + 1,
            u_prog,
            u_err,
            u_stuck,
            t_hat,
            err10: self.err10(),
            succ10: self.succ10(),
        };
        self.records.push_back(record);
        if self.records.len() > RECENT_RECORDS_CAP {
            self.records.pop_front();
        }
        for (name, value) in [
            ("u_prog", u_prog),
            ("u_err", u_err),
            ("u_stuck", u_stuck),
            ("t_hat", t_hat),
            ("err10", record.err10),
            ("succ10", record.succ10),
        ] {
            let series = self.features.entry(name).or_default();
            series.push_back(value);
            if series.len() > FEATURE_SERIES_CAP {
                series.pop_front();
            }
        }
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn current_domain(&self) -> Domain {
        self.current_domain
    }

    pub fn has_success(&self) -> bool {
        self.has_success
    }

    /// Latest record (Now).
    pub fn now(&self) -> Option<&TemporalRecord> {
        self.records.back()
    }

    /// Recent(k) — latest k rows, oldest first.
    pub fn recent(&self, k: u64) -> Vec<&TemporalRecord> {
        let k = (k as usize).min(self.records.len());
        self.records.iter().skip(self.records.len() - k).collect()
    }

    /// History — bounded migration log (≤ 20).
    pub fn history(&self) -> Vec<Migration> {
        self.migrations.iter().copied().collect()
    }

    /// 域迁移单调总计数（PULL 自描述增量头基线）。
    pub fn migration_count(&self) -> u64 {
        self.migration_count
    }

    /// Feature(name, k) — compact recent series (oldest first).
    pub fn feature(&self, name: &str, k: u64) -> Vec<f64> {
        let Some(series) = self.features.get(name) else {
            return Vec::new();
        };
        let k = (k as usize).min(series.len());
        series.iter().skip(series.len() - k).copied().collect()
    }

    pub fn known_feature_names() -> &'static [&'static str] {
        &["u_prog", "u_err", "u_stuck", "t_hat", "err10", "succ10"]
    }

    /// All spikes observed so far (archive payload, I4).
    pub fn spikes(&self) -> &[DomainSpike] {
        &self.spikes
    }

    /// B1 会话化基础（2026-09-03）：导出跨 prompt 续接快照——决策轮计数
    /// 与域机器状态精确携带（取代 I4 restore_spikes 的 round/entry_round
    /// 近似口径）。
    pub fn session_snapshot(&self) -> TemporalSessionSnapshot {
        TemporalSessionSnapshot {
            round: self.round,
            has_success: self.has_success,
            current_domain: self.current_domain,
            entry_round: self.entry_round,
            spikes: self.spikes.clone(),
        }
    }

    /// B1 会话化基础（2026-09-03）：从会话侧车快照精确续接域机器——
    /// `round` 直接恢复（下一决策轮 = round + 1，会话相对）；域切换
    /// 时间线重建 migrations / migration_count（与 restore_spikes 的
    /// 时间线推导同口径）。records / features / 工具窗口不恢复（新 run
    /// 有界查询面从零开始，与既有 per-run 语义一致）。
    pub fn restore_session_snapshot(&mut self, snapshot: &TemporalSessionSnapshot) {
        self.round = snapshot.round;
        self.has_success = snapshot.has_success;
        self.current_domain = snapshot.current_domain;
        self.entry_round = snapshot.entry_round;
        self.spikes.clear();
        self.migrations.clear();
        self.migration_count = 0;
        // 时间线重放（仅重建 history 查询面；at_round 取重建序号近似，不
        // 参与黑板 round 章——历史查询面恢复的近似性沿用 I4 restore_spikes
        // 已登记边界）。
        let mut replay_domain = Domain::Start;
        let mut replay_entry = 0u64;
        for spike in &snapshot.spikes {
            self.spikes.push(*spike);
            if spike.domain != replay_domain && replay_entry > 0 {
                let recovery = matches!(
                    (replay_domain, spike.domain),
                    (Domain::Stuck | Domain::LowProgress, Domain::Normal)
                );
                self.migrations.push_back(Migration {
                    from: replay_domain,
                    to: spike.domain,
                    at_t: spike.t,
                    at_round: replay_entry,
                    dwell_rounds: 0,
                    recovery,
                });
                if self.migrations.len() > MIGRATION_LOG_CAP {
                    self.migrations.pop_front();
                }
                self.migration_count = self.migration_count.saturating_add(1);
            }
            replay_domain = spike.domain;
            replay_entry = snapshot.round.max(1);
        }
        // 权威机器态已直接恢复（current_domain/entry_round/has_success），
        // 不随时间线重放被覆盖。
    }

    /// Restore spikes from a session sidecar (cross-prompt recovery, I4):
    /// the domain machine is rebuilt from the spike timeline. Only `t` +
    /// `domain` are persisted (§3.5), so `round` / `entry_round` / dwell
    /// metadata stay approximations (0); `has_success` is inferred from the
    /// timeline (any non-Start spike ⇒ at least one success occurred — Start
    /// is only produced before the first success and never re-entered).
    /// (审查处理 R6 / F3.)
    pub fn restore_spikes(&mut self, spikes: Vec<DomainSpike>) {
        self.spikes.clear();
        self.migrations.clear();
        self.current_domain = Domain::Start;
        self.entry_round = 0;
        self.has_success = spikes.iter().any(|s| s.domain != Domain::Start);
        for spike in spikes {
            self.spikes.push(spike);
            if spike.domain != self.current_domain && self.entry_round > 0 {
                let from = self.current_domain;
                let recovery = matches!(
                    (from, spike.domain),
                    (Domain::Stuck | Domain::LowProgress, Domain::Normal)
                );
                self.migrations.push_back(Migration {
                    from,
                    to: spike.domain,
                    at_t: spike.t,
                    at_round: self.round, // round counter not persisted — 0
                    dwell_rounds: 0,      // not derivable from t alone (§3.5)
                    recovery,
                });
                if self.migrations.len() > MIGRATION_LOG_CAP {
                    self.migrations.pop_front();
                }
                self.migration_count = self.migration_count.saturating_add(1);
            }
            self.current_domain = spike.domain;
            self.entry_round = self.round.max(1);
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_domain_until_first_success() {
        let mut st = TemporalState::new();
        st.record_round(1.0, 8.0, 0.0, 0.0, 0.0);
        assert_eq!(st.now().unwrap().domain, Domain::Start);
        st.observe_tool_outcome(ToolOutcome::Error);
        st.record_round(10.0, 8.0, 0.1, 1.0, 0.5);
        assert_eq!(st.now().unwrap().domain, Domain::Start);
        st.observe_tool_outcome(ToolOutcome::Success);
        st.record_round(20.0, 8.0, 1.0, 0.5, 0.3);
        assert_eq!(st.now().unwrap().domain, Domain::Normal);
    }

    /// B1 会话化基础（2026-09-03，R5）：会话快照精确携带决策轮计数与
    /// 域机器状态（round / current_domain / entry_round / has_success /
    /// spikes）——恢复后下一决策轮从快照轮续接、域切换不回归 Start、
    /// 时间线（spikes/migrations）一并重建。
    #[test]
    fn session_snapshot_restores_round_and_domain_machine() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        // r1: Start → Normal（首次成功后的正常域）。
        st.record_round(1.0, 8.0, 0.9, 0.5, 0.1);
        assert_eq!(st.now().unwrap().domain, Domain::Normal);
        // r2: Normal → Stuck。
        st.record_round(2.0, 8.0, 0.2, 2.5, 5.0);
        assert_eq!(st.now().unwrap().domain, Domain::Stuck);

        let snap = st.session_snapshot();
        assert_eq!(snap.round, 2);
        assert_eq!(snap.current_domain, Domain::Stuck);
        assert_eq!(snap.entry_round, 2);
        assert!(snap.has_success);
        assert_eq!(snap.spikes.len(), 2, "Normal@r1 + Stuck@r2 两个切换点");

        let mut restored = TemporalState::new();
        restored.restore_session_snapshot(&snap);
        assert_eq!(restored.round(), 2, "round 精确续接");
        assert_eq!(restored.current_domain(), Domain::Stuck);
        assert!(restored.has_success());
        assert_eq!(restored.entry_round, 2);
        assert_eq!(restored.spikes().len(), 2);

        // 下一决策轮 = 3；Stuck → Normal 恢复迁移正确标注 recovery。
        restored.record_round(3.0, 8.0, 0.9, 0.5, 0.1);
        assert_eq!(restored.round(), 3);
        assert_eq!(restored.now().unwrap().domain, Domain::Normal);
        let history = restored.history();
        let last = history.last().expect("migration rebuilt");
        assert!(last.recovery);
        assert_eq!(last.from, Domain::Stuck);
        assert_eq!(last.to, Domain::Normal);
    }

    #[test]
    fn deny_is_neutral_for_err10_succ10_but_counts_as_tool_event() {
        // R2 (2026-08-31): a denial is a completed tool call (total events
        // increments) but is neither an error nor a success for the
        // err10/succ10 window — the deny channel owns that semantic.
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        st.observe_tool_outcome(ToolOutcome::Deny);
        st.observe_tool_outcome(ToolOutcome::Error);
        st.observe_tool_outcome(ToolOutcome::Deny);
        assert_eq!(st.total_tool_events(), 4);
        assert_eq!(st.total_errors(), 1);
        assert_eq!(st.total_successes(), 1);
        assert!(st.has_success());
        // err10/succ10: 2 events in the last-10 window are Error/Success;
        // the two denies are neutral.
        assert_eq!(st.err10(), 0.1);
        assert_eq!(st.succ10(), 0.1);
    }

    #[test]
    fn semantic_domain_grid() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        // Normal
        st.record_round(1.0, 8.0, 0.9, 0.5, 0.1);
        assert_eq!(st.now().unwrap().domain, Domain::Normal);
        // Pressure: u_err ≥ 2
        st.record_round(2.0, 8.0, 0.9, 2.5, 0.1);
        assert_eq!(st.now().unwrap().domain, Domain::Pressure);
        // Stuck: low progress + pressure
        st.record_round(3.0, 8.0, 0.2, 2.5, 5.0);
        assert_eq!(st.now().unwrap().domain, Domain::Stuck);
        // LowProgress: low progress, no pressure
        st.record_round(4.0, 8.0, 0.2, 0.5, 0.1);
        assert_eq!(st.now().unwrap().domain, Domain::LowProgress);
        // Recovery: LowProgress → Normal
        st.record_round(5.0, 8.0, 0.9, 0.5, 0.1);
        assert_eq!(st.now().unwrap().domain, Domain::Normal);
        let history = st.history();
        assert!(
            history
                .iter()
                .any(|m| m.recovery && m.from == Domain::LowProgress)
        );
    }

    #[test]
    fn pressure_uses_stuck_theta_bound() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        // u_stuck ≥ 1.5·T̂ = 12 with T̂=8 → pressure even with u_err < 2.
        st.record_round(1.0, 8.0, 0.9, 0.5, 13.0);
        assert_eq!(st.now().unwrap().domain, Domain::Pressure);
        st.record_round(2.0, 8.0, 0.2, 0.5, 13.0);
        assert_eq!(st.now().unwrap().domain, Domain::Stuck);
    }

    #[test]
    fn err10_succ10_window_is_bounded() {
        let mut st = TemporalState::new();
        for _ in 0..6 {
            st.observe_tool_outcome(ToolOutcome::Error);
        }
        for _ in 0..6 {
            st.observe_tool_outcome(ToolOutcome::Success);
        }
        st.record_round(1.0, 8.0, 1.0, 0.0, 0.0);
        let rec = st.now().unwrap();
        assert!((rec.err10 - 0.4).abs() < 1e-12); // 4 errors in the last 10
        assert!((rec.succ10 - 0.6).abs() < 1e-12); // 6 successes in the last 10
    }

    #[test]
    fn spikes_and_migrations_are_recorded() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        st.record_round(1.0, 8.0, 0.9, 0.5, 0.1); // Normal
        st.record_round(2.0, 8.0, 0.2, 2.5, 5.0); // Stuck
        st.record_round(3.0, 8.0, 0.9, 0.5, 0.1); // Normal (recovery)
        assert_eq!(st.spikes().len(), 3); // Normal, Stuck, Normal
        assert_eq!(st.history().len(), 2);
        assert_eq!(st.now().unwrap().dwell_rounds, 1);
        assert_eq!(st.now().unwrap().entry_round, 3);
    }

    #[test]
    fn recent_and_feature_queries_respect_bounds() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        for i in 0..30 {
            st.record_round(i as f64, 8.0, 1.0, 0.0, 0.0);
        }
        assert_eq!(st.recent(5).len(), 5);
        assert_eq!(st.recent(100).len(), RECENT_RECORDS_CAP);
        let u = st.feature("u_prog", 5);
        assert_eq!(u.len(), 5);
        let all = st.feature("u_prog", 100);
        assert_eq!(all.len(), FEATURE_SERIES_CAP);
        assert!(st.feature("nope", 3).is_empty());
    }

    #[test]
    fn restore_spikes_rebuilds_domain_machine() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        st.record_round(1.0, 8.0, 0.9, 0.5, 0.1); // Normal
        st.record_round(2.0, 8.0, 0.2, 2.5, 5.0); // Stuck
        let spikes = st.spikes().to_vec();
        assert_eq!(spikes.len(), 2);

        // No success pre-seeding: restore must infer has_success from the
        // spike timeline itself (审查处理 R6 / F3 — the pre-fix test masked
        // the defect by seeding a Success before restore).
        let mut restored = TemporalState::new();
        restored.restore_spikes(spikes);
        assert_eq!(restored.current_domain(), Domain::Stuck);
        assert!(restored.has_success(), "restore must rebuild has_success");
        assert_eq!(restored.spikes().len(), 2);

        // First round after restore with the same features must stay Stuck —
        // no fake Start regression, migration or extra spike.
        let hist_len = restored.history().len();
        let spikes_len = restored.spikes().len();
        restored.record_round(3.0, 8.0, 0.2, 2.5, 5.0);
        assert_eq!(restored.current_domain(), Domain::Stuck);
        assert_eq!(restored.history().len(), hist_len);
        assert_eq!(restored.spikes().len(), spikes_len);

        // Recovery flag must be recomputed (not hard-coded false): a
        // Stuck → Normal timeline restores a recovery-marked migration.
        let mut st2 = TemporalState::new();
        st2.observe_tool_outcome(ToolOutcome::Success);
        st2.record_round(1.0, 8.0, 0.2, 2.5, 5.0); // Stuck
        st2.record_round(2.0, 8.0, 0.9, 0.5, 0.1); // Normal (recovery)
        let mut restored2 = TemporalState::new();
        restored2.restore_spikes(st2.spikes().to_vec());
        assert!(
            restored2.history().iter().any(|m| m.recovery),
            "Stuck→Normal migration must be marked recovery after restore"
        );
    }

    /// PULL 自描述（2026-08-31，P2-11 第 1 项）：migration_count 是单调
    /// 总计数（有界队列 pop_front 后不可用长度作增量基线）；restore_spikes
    /// 从 spike 时间线重建计数。
    #[test]
    fn migration_count_is_monotonic_across_bounded_log() {
        let mut st = TemporalState::new();
        st.observe_tool_outcome(ToolOutcome::Success);
        st.record_round(1.0, 8.0, 0.9, 0.5, 0.1); // Normal
        st.record_round(2.0, 8.0, 0.2, 2.5, 5.0); // Stuck → 迁移 1
        st.record_round(3.0, 8.0, 0.2, 2.5, 5.0); // 仍 Stuck（无迁移）
        st.record_round(4.0, 8.0, 0.9, 0.5, 0.1); // Normal（recovery）→ 迁移 2
        assert_eq!(st.migration_count(), 2);
        assert_eq!(st.history().len(), 2);
        assert!(st.history()[1].recovery);

        let spikes = st.spikes().to_vec();
        let mut restored = TemporalState::new();
        restored.restore_spikes(spikes);
        assert_eq!(restored.migration_count(), 2);
        assert_eq!(restored.history().len(), 2);
    }
}
