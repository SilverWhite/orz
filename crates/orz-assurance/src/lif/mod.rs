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
pub mod rli;
pub mod router;
pub mod temporal;

pub use router::{
    ActionClass, BusStimulus, DenyClass, ResourceTier, StimulusRouting, stimulus_targets,
};

pub use channels::{
    CTX_TAU_ROUNDS, ChannelKind, DENY_REFRACTORY_SECS, DENY_TAU_SECS, DENY_THETA,
    ERR_REFRACTORY_SECS, ERR_TAU_SECS, ERR_THETA, FirstOrderChannel, INFRA_TAU_ROUNDS,
    PROG_TAU_ROUNDS, SLOW_REFRACTORY_SECS, SLOW_TAU_SECS, SLOW_THETA, SLOW_W_MAX,
    SLOW_WALL_MS_THRESHOLD, STALL_GAP_THRESHOLD_SECS, STALL_REFRACTORY_SECS, STALL_TAU_SECS,
    STALL_THETA, STUCK_REFRACTORY_ROUNDS, STUCK_TAU_ROUNDS, STUCK_THETA_ROUNDS, StuckChannel,
    ToolEvent, ToolOutcome, VERIFY_TAU_ROUNDS, classify_event_outcome, is_denial_code,
};
pub use estimator::{
    INTERVAL_BUF_CAP, INTERVAL_CAP_SECS, RoundIntervalEstimator, T_HAT_ENABLE_SAMPLES,
    T_HAT_INIT_SECS, T_HAT_MAX_SECS, T_HAT_MIN_SECS,
};
// 0am S2（2026-09-17）：RLI 影子族公开面。
// 0am 改造四项①②（2026-09-20）＋补充项①②④（2026-09-20）：预测步长常数 +
// 自判域机器族 + 配极表 + 锚点序列表公开面。
// 0be 四项（2026-09-21）：λ̂／分通道 horizon／短视锚点／自适应轨迹／繁杂度、
// 公开面（宿主投递判定与渲染面共用）。
pub use rli::{
    RLI_ADAPT_TRACE_CAP, RLI_ANCHOR_FEATURE_NAMES, RLI_ANCHOR_FEATURE_TABLE, RLI_CHANNELS,
    RLI_COVERAGE_MIN_SEGMENTS, RLI_CPLX_BASELINE_SAMPLES, RLI_CPLX_KINDS,
    RLI_CPLX_MIN_CHANNEL_SAMPLES, RLI_CPLX_PER_CHANNEL_CAP, RLI_CPLX_TIERS,
    RLI_DOMAIN_ERR_ENVELOPE_FLOOR, RLI_DOMAIN_ERR_PRESSURE, RLI_DOMAIN_PROG_LOW,
    RLI_DOMAIN_RECENT_CAP, RLI_ETA_INIT, RLI_FEATURE_SERIES_CAP, RLI_LAMBDA_EMA_ALPHA,
    RLI_LAMBDA_GAP_MAX_SECS, RLI_LAMBDA_GAP_MIN_SECS, RLI_LAMBDA_MIN_GAPS, RLI_NOTICE_CAP,
    RLI_NOTICE_TEXT_BUDGET, RLI_PREDICTION_STEPS, RLI_PROG_ZETA, RLI_Q, RLI_RECENT_OUTCOMES_CAP,
    RLI_SNAPSHOT_SCHEMA, RLI_STREAK_K, RLI_SYMBOL_LEGEND, RLI_T_HAT_ANNOTATION,
    RLI_TAU_R_HALF_PERIODS, RLI_THETA_INIT, RLI_WATCHDOG_SAMPLE_SECS, RLI_ZETA, RliAdaptTraceRow,
    RliAnchor, RliAnchors, RliChannel, RliChannelSnapshot, RliComplexity, RliComplexityReading,
    RliComplexitySnapshot, RliDomainMachine, RliDomainRow, RliDomainSegment, RliDomainSnapshot,
    RliDomainSpike, RliNotice, RliNoticeKind, RliPendingPrediction, RliShadow, RliShadowSnapshot,
    rli_horizon_steps, rli_notice_annotation, rli_zeta_for,
};
pub use temporal::{
    Domain, DomainSpike, Migration, TemporalQuery, TemporalRecord, TemporalSessionSnapshot,
    TemporalState, ToolOutcomeBucket,
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
    /// 0am P8（2026-10-03，S2 路由表）：验证摩擦／认知负载／供给摩擦
    /// 三通道（非 firing 水平通道，τ 随决策轮 k·T̂ 重导出）。
    verify: FirstOrderChannel,
    ctx: FirstOrderChannel,
    infra: FirstOrderChannel,
    stuck: StuckChannel,
    temporal: TemporalState,
    /// 0am S2（2026-09-17）：RLI 影子族（旁路并行）。`None` = 未启用
    /// （默认；零成本——影子不构造、不喂入、侧车不携带）。启用判定在
    /// orz-loop（env `ORZ_LIF_RLI_SHADOW`），本层不做 env 读取（保持
    /// 纯确定性内核纪律）。
    rli_shadow: Option<RliShadow>,
    err_tau_mode: ErrTauMode,
    t_hat_mode: THatMode,
    run_t0: Option<f64>,
    last_time: Option<f64>,
    last_decision_t: Option<f64>,
    last_tool_t: Option<f64>,
    /// 0am P8：最近一次资源快照档位（跨档过滤记忆；live-only 不随任何
    /// 持久化面携带——run 级新鲜态）。
    last_snapshot_tier: Option<ResourceTier>,
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
            verify: FirstOrderChannel::verify(),
            ctx: FirstOrderChannel::ctx(),
            infra: FirstOrderChannel::infra(),
            stuck: StuckChannel::new(),
            temporal: TemporalState::new(),
            rli_shadow: None,
            err_tau_mode: ErrTauMode::default(),
            t_hat_mode: THatMode::default(),
            run_t0: None,
            last_time: None,
            last_decision_t: None,
            last_tool_t: None,
            last_snapshot_tier: None,
        }
    }

    /// Anchor the run clock at `t0` (the first observed event time). The
    /// engine only ever works in run-relative seconds.
    pub fn set_run_origin(&mut self, t0: f64) {
        self.run_t0 = Some(t0);
        let t0r = 0.0;
        self.last_time = Some(t0r);
    }

    /// Anchor the run clock at `t0` only when the engine has not observed
    /// anything yet (P2-12 write-time stamping must share ONE run-relative
    /// axis with the temporal rows even if the first stamped failure event
    /// would otherwise race the engine's own first observation).
    pub fn ensure_run_origin(&mut self, t0: f64) {
        if self.run_t0.is_none() {
            self.set_run_origin(t0);
        }
    }

    /// B1 会话化基础（2026-09-03，P2-13 / R5 conversation-relative 轴）：
    /// 把时间轴原点预置为会话起始墙钟秒（仅在引擎尚未观测任何事件时
    /// 生效），使 temporal 行与 failure_agg 的 `t` 跨 prompt 单调（会话
    /// 相对）。与 [`Self::set_run_origin`] 不同：**不**把 `last_time`
    /// 预置到 0——通道/估计器保持 run 级新鲜态，跨 prompt 间隙不产生
    /// 衰减/驻留伪差（last_time 在首次观测时初始化）。
    pub fn set_axis_origin(&mut self, wall_t0: f64) {
        if self.run_t0.is_none() {
            self.run_t0 = Some(wall_t0);
        }
    }

    /// Run-relative seconds on the engine axis for a wall-clock epoch time
    /// `t`. When the engine has no origin yet this mirrors [`Self::rel`]
    /// (returns `t` untouched) — callers that need a guaranteed shared axis
    /// should call [`Self::ensure_run_origin`] first.
    pub fn run_relative_secs(&self, t: f64) -> f64 {
        self.rel(t)
    }

    /// TER T1.8 (2026-09-04): run 时间轴原点（wall-clock epoch 秒）；
    /// `None` = 引擎尚未锚定（读取侧按 elapsed 0 呈现，不 side-effect
    /// 锚定）。
    pub fn run_origin_secs(&self) -> Option<f64> {
        self.run_t0
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
        // 0am P8：轮语义通道族 τ 重导出（k·T̂；单位换算）。
        self.verify.set_tau_secs(VERIFY_TAU_ROUNDS * t_hat);
        self.ctx.set_tau_secs(CTX_TAU_ROUNDS * t_hat);
        self.infra.set_tau_secs(INFRA_TAU_ROUNDS * t_hat);
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
        // 0am S2：影子旁路喂入（同一事件流、同一 run 相对轴；不影响 1D）。
        // 0am 改造补充项②（2026-09-20）：**不再携带 LIF 现域**——RLI 不与
        // LIF 对照（域一致性口径改对「框架实际动作结果」，见离线观测件）。
        if let Some(shadow) = &mut self.rli_shadow {
            shadow.on_decision_round(t, t_hat);
        }
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
        // 0am P8（2026-10-03，S2 路由表）：分派统一走 router 纯函数——
        // `routing: None`（合成事件/旧调用面）＝旧四值语义，行为不变；
        // 生产喂入点恒带路由键＝S2 语义（Prog 收窄/验证通道/Slow 豁免/
        // H2 填平）。**零触发语义**（P9）——本函数只决定「通道看见什么」。
        let targets = router::stimulus_targets(&event);
        if targets.err {
            self.err.spike(t, 1.0);
        }
        if targets.deny {
            self.deny.spike(t, 1.0);
        }
        if targets.prog_set {
            self.prog.spike(t, 1.0);
        }
        if targets.verify {
            self.verify.spike(t, 1.0);
        }
        if let Some(w) = targets.slow {
            self.slow.spike(t, w);
        }
        if long_gap {
            self.stall.spike(t, 1.0);
        }
        self.check_fires(t);
        self.temporal.observe_tool_outcome(event.outcome);
        // 0am S2：影子旁路喂入（同一事件流；不影响 1D 任何输出）。
        if let Some(shadow) = &mut self.rli_shadow {
            shadow.on_tool_event(t, event);
        }
    }

    /// 0am P8（2026-10-03，S2 §3 Ctx/Infra 认领）：非工具事件总线注入。
    /// `host_resource_snapshot` 在此做**跨档过滤**（v1.1：run 首测＝基线
    /// 不计；引擎记忆上一档位）。零触发语义（P9）。
    pub fn on_bus_event(&mut self, t: f64, stimulus: BusStimulus) {
        if self.run_t0.is_none() {
            self.set_run_origin(t);
        }
        let t = self.rel(t);
        let stimulus = match stimulus {
            BusStimulus::HostResourceSnapshotTier(tier) => {
                let crossing = matches!(self.last_snapshot_tier, Some(prev) if prev != tier);
                self.last_snapshot_tier = Some(tier);
                if !crossing {
                    return;
                }
                BusStimulus::HostResourceSnapshotTier(tier)
            }
            other => other,
        };
        let Some(kind) = stimulus.channel() else {
            return;
        };
        // 0am P8-b：总线标签（闭集；成因段数据面）。
        let label = match stimulus {
            BusStimulus::ContextCompressed => "压缩",
            BusStimulus::LedgerFoldAdvance => "折叠推进",
            BusStimulus::LedgerFoldWriteFailed => "折叠写失败",
            BusStimulus::TransportRetry => "传输重试",
            BusStimulus::AvailabilityFlip => "探针翻转",
            BusStimulus::HostResourceDenied => "资源拒绝",
            BusStimulus::ResourceLimitHit => "限额命中",
            BusStimulus::HostResourceSnapshotTier(_) => "资源跨档",
        };
        self.advance(t);
        match kind {
            ChannelKind::Ctx => self.ctx.spike(t, 1.0),
            ChannelKind::Infra => self.infra.spike(t, 1.0),
            _ => return,
        }
        if let Some(shadow) = &mut self.rli_shadow {
            shadow.on_bus_event(t, kind, label);
        }
    }

    /// 0cp S2 修正批（169 批，2026-10-03）：RLI 卡死看门狗的宿主 tick 唯一
    /// 入口——与两动作源（[`Self::on_decision_round`]／[`Self::on_tool_event`]
    /// ）同轴纪律：`t` 为墙钟 epoch 秒，经 [`Self::rel`] 转 **run 相对轴**后
    /// 才喂影子（影子内部轴＝run 相对秒；直传 epoch 会令看门狗窗口①恒真、
    /// 并以 epoch 量级 Δt 推进通道动力学＝状态湮灭＋影子冻结——168 批审查
    /// P1 实锤，轴转换收敛于本包装，宿主不得再绕行）。
    ///
    /// 影子未启用（kill switch）＝恒 `false` 零采样。判定与采样在调用方持有
    /// 引擎锁的临界区内完成（同步方法、不跨 await）。
    pub fn on_rli_watchdog_tick(&mut self, t: f64, activity_idle_secs: f64) -> bool {
        let t = self.rel(t);
        match &mut self.rli_shadow {
            Some(shadow) => shadow.on_watchdog_tick(t, activity_idle_secs),
            None => false,
        }
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
        self.verify.decay_to(t);
        self.ctx.decay_to(t);
        self.infra.decay_to(t);
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

    /// 0am P8：新三通道只读面（锚点/诊断）。
    pub fn verify(&self) -> &FirstOrderChannel {
        &self.verify
    }

    pub fn ctx(&self) -> &FirstOrderChannel {
        &self.ctx
    }

    pub fn infra(&self) -> &FirstOrderChannel {
        &self.infra
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

    /// 0am S2（2026-09-17）：启用 RLI 影子族（幂等）。env 门控判定在
    /// orz-loop（`ORZ_LIF_RLI_SHADOW`）；未启用时零成本（影子不构造、
    /// 不喂入、侧车不携带字段）。
    pub fn enable_rli_shadow(&mut self) {
        if self.rli_shadow.is_none() {
            self.rli_shadow = Some(RliShadow::new());
        }
    }

    /// 0bf ①（2026-09-22）：停用 RLI 影子族（kill switch 的引擎侧等效
    /// 动作）——影子整体丢弃（含未投递提醒与采样锚；下次启用 = fresh，
    /// 通道面与时间轴不受影响）。loop 层由 env
    /// `ORZ_LIF_RLI_SHADOW=0/off/false/no` 判定（缺省常开）。
    pub fn disable_rli_shadow(&mut self) {
        self.rli_shadow = None;
    }

    pub fn rli_shadow(&self) -> Option<&RliShadow> {
        self.rli_shadow.as_ref()
    }

    pub fn rli_shadow_mut(&mut self) -> Option<&mut RliShadow> {
        self.rli_shadow.as_mut()
    }

    pub fn rli_shadow_enabled(&self) -> bool {
        self.rli_shadow.is_some()
    }

    /// B1 会话化基础（2026-09-03）：跨 prompt 续接快照（round / 域机器
    /// / spike 时间线）。0am S2：影子启用时同时携带 RLI 影子族状态
    /// （`rli_shadow`；未启用 = `None`，侧车无字段）。
    pub fn temporal_session_snapshot(&self) -> TemporalSessionSnapshot {
        let mut snapshot = self.temporal.session_snapshot();
        snapshot.rli_shadow = self.rli_shadow.as_ref().map(RliShadow::snapshot);
        snapshot
    }

    /// B1 会话化基础（2026-09-03）：从会话侧车快照续接轮号与域机器；
    /// `axis_origin_wall = Some(session_start)` 时同时把 `t` 轴预置为
    /// 会话相对（跨 prompt 单调）。`None` = 无会话轴（legacy 侧车 /
    /// 单 run），沿用 run 起点原点语义。
    /// 0am S2：快照携带影子状态且本引擎已启用影子时一并续接（未启用 =
    /// 无影子可续，快照字段静默忽略——门控纪律：关 = 零成本）。
    pub fn restore_temporal_session(
        &mut self,
        snapshot: &TemporalSessionSnapshot,
        axis_origin_wall: Option<f64>,
    ) {
        self.temporal.restore_session_snapshot(snapshot);
        if let Some(t0) = axis_origin_wall {
            self.set_axis_origin(t0);
        }
        if let Some(shadow_snapshot) = &snapshot.rli_shadow
            && let Some(shadow) = &mut self.rli_shadow
            && !shadow.restore(shadow_snapshot)
        {
            tracing::warn!(
                "RLI shadow snapshot rejected (schema or channel-family mismatch) — \
                 fresh shadow state kept"
            );
        }
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

    /// B1 会话化基础（2026-09-03，R5 conversation-relative 轴）：轴原点
    /// 预置为会话起始墙钟后，temporal `t` = wall − session_start（会话
    /// 相对、跨 prompt 单调）；`restore_temporal_session` 精确续接轮号
    /// 且保持同一轴。
    #[test]
    fn axis_preset_and_session_restore_keep_conversation_relative_t() {
        let session_start = 1_700_000_000.0;
        let mut engine = LifEngine::new();
        engine.set_axis_origin(session_start);
        engine.on_decision_round(session_start + 10.0);
        assert_eq!(engine.temporal().round(), 1);
        let row = engine.temporal().now().expect("decision row");
        assert!(
            (row.t - 10.0).abs() < 1e-6,
            "t is session-relative (got {})",
            row.t
        );

        // 跨 prompt：新引擎恢复快照 + 同轴原点 → 决策轮从 1 续到 2、
        // t 从 60s 续接（不回归 0）。
        let snap = engine.temporal_session_snapshot();
        let mut engine2 = LifEngine::new();
        engine2.restore_temporal_session(&snap, Some(session_start));
        engine2.on_decision_round(session_start + 60.0);
        assert_eq!(engine2.temporal().round(), 2, "round continues");
        let row2 = engine2.temporal().now().expect("second row");
        assert!(
            (row2.t - 60.0).abs() < 1e-6,
            "t continues on the same axis (got {})",
            row2.t
        );
    }

    /// 0am S2 钉子：影子关闭 = 零成本默认；启用影子不改变 1D 生产面
    /// （同一事件流下 1D 读数逐字段全等——生产 1D 不动的机械证据）。
    #[test]
    fn rli_shadow_off_by_default_and_leaves_production_1d_untouched() {
        let mut events = Vec::new();
        let mut t = 0.0f64;
        for i in 0..40u32 {
            t += 3.0 + f64::from(i % 7);
            let ev = match i % 5 {
                0 => ToolEvent::deny(Some(10)),
                1 => ToolEvent::error(Some(900)),
                _ => ToolEvent::success(Some(400)),
            };
            events.push((t, ev));
        }

        let mut plain = LifEngine::new();
        let mut shadowed = LifEngine::new();
        assert!(!plain.rli_shadow_enabled(), "shadow off by default");
        shadowed.enable_rli_shadow();
        assert!(shadowed.rli_shadow_enabled());

        let mut td = 0.0f64;
        for (tt, ev) in &events {
            td = td.max(*tt - 1.0);
            plain.on_decision_round(td);
            shadowed.on_decision_round(td);
            plain.on_tool_event(*tt, *ev);
            shadowed.on_tool_event(*tt, *ev);
        }

        assert_eq!(plain.err().u().to_bits(), shadowed.err().u().to_bits());
        assert_eq!(plain.prog().u().to_bits(), shadowed.prog().u().to_bits());
        assert_eq!(plain.slow().u().to_bits(), shadowed.slow().u().to_bits());
        assert_eq!(plain.stall().u().to_bits(), shadowed.stall().u().to_bits());
        assert_eq!(plain.deny().u().to_bits(), shadowed.deny().u().to_bits());
        assert_eq!(plain.stuck().u().to_bits(), shadowed.stuck().u().to_bits());
        assert_eq!(
            plain.current_t_hat().to_bits(),
            shadowed.current_t_hat().to_bits()
        );
        assert_eq!(plain.temporal().round(), shadowed.temporal().round());
        assert_eq!(
            plain.temporal().total_tool_events(),
            shadowed.temporal().total_tool_events()
        );

        // 旁路非空转：err 影子通道确已注入。
        let shadow = shadowed.rli_shadow().expect("shadow enabled");
        assert!(shadow.channel(ChannelKind::Err).u() > 0.0);
        assert!(shadow.steps() > 0);
    }

    /// 0am S2：影子随会话快照进入侧车面并可精确续接；未启用引擎的
    /// 快照无影子字段（零迁移 / 门控纪律）。
    #[test]
    fn rli_shadow_rides_session_snapshot_and_restores() {
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        let mut t = 0.0f64;
        for i in 0..12u32 {
            t += 10.0;
            engine.on_decision_round(t);
            let ev = if i % 3 == 0 {
                ToolEvent::error(Some(1_200))
            } else {
                ToolEvent::success(Some(300))
            };
            engine.on_tool_event(t + 1.0, ev);
        }
        let snapshot = engine.temporal_session_snapshot();
        let shadow_snapshot = snapshot.rli_shadow.as_ref().expect("shadow in snapshot");
        assert_eq!(shadow_snapshot.schema, RLI_SNAPSHOT_SCHEMA);
        assert_eq!(shadow_snapshot.channels.len(), RLI_CHANNELS.len());
        let err_u = engine.rli_shadow().unwrap().channel(ChannelKind::Err).u();
        assert!(err_u > 0.0);

        let mut restored = LifEngine::new();
        restored.enable_rli_shadow();
        restored.restore_temporal_session(&snapshot, None);
        let restored_u = restored.rli_shadow().unwrap().channel(ChannelKind::Err).u();
        assert!(
            (restored_u - err_u).abs() < 1e-3,
            "restored {restored_u} vs {err_u} (3-decimal storage contract)"
        );

        // 未启用引擎：快照无字段；恢复时静默忽略（门控关 = 零成本）。
        let mut plain = LifEngine::new();
        plain.on_decision_round(1.0);
        assert!(plain.temporal_session_snapshot().rli_shadow.is_none());
        plain.restore_temporal_session(&snapshot, None);
        assert!(!plain.rli_shadow_enabled());
    }

    /// 0cp S2 修正批（169 批，2026-10-03）：看门狗**宿主接缝的轴判别钉**——
    /// `on_rli_watchdog_tick` 接收墙钟 epoch 秒，窗口①必须在 run 相对轴上
    /// 判「自上一动作样 Δt ≥ 181s」。168 批实现曾把 epoch 直传影子：窗口①
    /// 对 epoch 恒真（首个满 idle 的 tick 即误触发），并以 epoch 量级 Δt 推
    /// 进通道动力学＝通道湮灭、锚点序列记入 ~1.7e9 垃圾点、后续动作样
    /// dt=0 ⇒ 影子永久冻结。本钉在 epoch 时间基下判别：窗未满不触发、窗满
    /// 恰产一样、动作锚不被看门狗盖（影子内 `last_sample_t` 保持相对轴值）。
    #[test]
    fn rli_watchdog_tick_converts_wall_epoch_to_run_relative_axis() {
        let t0 = 1_759_000_000.0_f64;
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        // 首个决策轮锚定轴原点（run_t0 = t0+100）并盖首个动作样锚（rel 0）。
        engine.on_decision_round(t0 + 100.0);
        let samples_before = engine.rli_shadow().unwrap().sample_points();
        assert_eq!(samples_before, 1);
        // 窗未满（rel 180.5 < 181）∧ idle 已满 181 ⇒ 不触发——旧实现
        // （epoch 直传）此处恒真必误触发，本断言即轴判别。
        assert!(
            !engine.on_rli_watchdog_tick(t0 + 100.0 + 180.5, 181.0),
            "window ① is judged on the run-relative axis, not wall epoch"
        );
        assert_eq!(
            engine.rli_shadow().unwrap().sample_points(),
            samples_before,
            "no sample before the window is due"
        );
        // 窗满（rel 281.5 ≥ 181）∧ idle 满 ⇒ 恰产一个时间采样。
        assert!(engine.on_rli_watchdog_tick(t0 + 100.0 + 181.5, 181.0));
        assert_eq!(
            engine.rli_shadow().unwrap().sample_points(),
            samples_before + 1
        );
        // 动作锚不被看门狗盖（设计 D2：不盖锚）——影子内锚仍在相对轴原值
        // 0.0（若被 epoch 污染将 ≈1.759e9）。
        assert_eq!(
            engine.rli_shadow().unwrap().snapshot().last_sample_t,
            Some(0.0),
            "anchor stays on the run-relative axis, untouched by the watchdog"
        );
        // 触发后动作样照常结算（影子未被冻结：采样数随动作样继续前进）。
        engine.on_tool_event(t0 + 300.0, ToolEvent::success(Some(10)));
        assert_eq!(
            engine.rli_shadow().unwrap().sample_points(),
            samples_before + 2
        );
    }

    /// 0am 改造四项①（2026-09-20）＋补充项②④（2026-09-20）：引擎逐决策轮
    /// 喂入影子；自判域行携带 RLI 原生锚点（u/v/E + u_prog）、两轴同轮对齐
    /// （自判域行 round / t 与 temporal 行同刻度）；**不再携带 LIF 现域对照**
    /// （RLI 不与 LIF 对照——域一致性口径改对「框架实际动作结果」）。
    #[test]
    fn rli_domain_rows_record_native_anchors_per_decision_round() {
        let mut engine = LifEngine::new();
        engine.enable_rli_shadow();
        let mut t = 0.0f64;
        for i in 0..12u32 {
            t += 10.0;
            engine.on_decision_round(t);
            let ev = if i % 4 == 3 {
                ToolEvent::error(Some(500))
            } else {
                ToolEvent::success(Some(200))
            };
            engine.on_tool_event(t + 0.5, ev);
        }
        let shadow = engine.rli_shadow().expect("shadow enabled");
        let domain = shadow.domain();
        assert_eq!(
            domain.round(),
            engine.temporal().round(),
            "same-round axes (decision rounds)"
        );
        let row = domain.now().expect("domain row");
        let lif_row = engine.temporal().now().expect("temporal row");
        assert!((row.t - lif_row.t).abs() < 1e-9, "same t axis");
        assert!(
            matches!(row.v_err, Some(v) if v.is_finite()),
            "native v anchor recorded (0bc FR-7: Some ＝ 记录过): {row:?}"
        );
        assert!(
            matches!(row.env_err, Some(e) if e.is_finite()),
            "native E anchor recorded (0bc FR-7: Some ＝ 记录过): {row:?}"
        );
        assert!(
            row.env_err.is_some_and(|e| e >= 0.0),
            "envelope is a magnitude: {:?}",
            row.env_err
        );
        assert!(
            ["start", "normal", "pressure", "low_progress", "stuck"].contains(&row.domain.as_str()),
            "closed domain vocabulary: {}",
            row.domain.as_str()
        );
        // 补充项④：锚点序列随决策轮采样（与域行同轮刻度）。
        assert_eq!(shadow.feature("u_err", 5).len(), 5);
        assert_eq!(shadow.feature("env_err", 5).len(), 5);
    }
}
