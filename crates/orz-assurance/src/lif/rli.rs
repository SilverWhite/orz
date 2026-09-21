//! RLI — resonant leaky-integrator shadow channels (0am S2, 2026-09-17).
//!
//! Design: [`RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16`]（orz 自研推导，
//! 暂定基座 v1.0）。本模块＝**旁路影子**：现行一阶（1D）生产基座输出不动，
//! 影子整体由 env 门控（`ORZ_LIF_RLI_SHADOW`，判定在 orz-loop），影子状态
//! 随会话侧车持久化（[`super::temporal::TemporalSessionSnapshot::rli_shadow`]）。
//! 影子无渲染面、无注入面（设计 §3.5 零注入纪律同格）。
//!
//! 数学（设计 §2）：每通道二维状态 `(u, v)`，事件间隔 Δt 内自由演化取
//! 闭式精确解（无步长离散、右端点伪迹豁免）：
//!
//! ```text
//! A = u₀；B = (v₀ + ζω·u₀)/ω_d；ω_d = ω·√(1−ζ²)
//! φ = e^(−ζω·Δt)；C = cos(ω_d·Δt)；S = sin(ω_d·Δt)
//! u₁ = φ·(A·C + B·S)
//! v₁ = φ·((ω_d·B − ζω·A)·C − (ω_d·A + ζω·B)·S)
//! 事件注入：u₁ ← u₁ + w（冲击在 u 上；v 不踢）
//! ```
//!
//! **配极改（0am 改造补充项①，2026-09-20 用户令；构造见设计 §11）**：数学
//! 形式不整体换，改的是**按通道语义分配极点**——速率／压力类（err／stall／
//! slow／deny）保留复极点对（ζ=[`RLI_ZETA`]：预期＋节律语义）；水平／新鲜度
//! 类（prog）走**实极点**（ζ=[`RLI_PROG_ZETA`]：持久／适应语义，两个实模态
//! ＝慢模态持久面＋快模态适应面）。族不变（一阶＝该族的强过阻尼极限，设计
//! §6）。过阻尼分支（ζ > 1）取同一二阶解族的双曲闭式：
//!
//! ```text
//! μ = ω·√(ζ²−1)（下限 1e−12 保 ζ→1 邻域稳定）；φ = e^(−ζω·Δt)
//! u₁ = φ·(u₀·cosh(μΔt) + ((v₀+ζω·u₀)/μ)·sinh(μΔt))
//! v₁ = φ·(v₀·cosh(μΔt) − ((ζω·v₀ + ω²·u₀)/μ)·sinh(μΔt))
//! E₁ = φ·(|u₀|·cosh(μΔt) + |(v₀+ζω·u₀)/μ|·sinh(μΔt))   （双曲包络上界）
//! 注入当刻包络相位零基点：E = φ·|u|（实极点无相位，E 与水平同尺）
//! ```
//!
//! **判断标准改写（0am 改造补充项②，2026-09-20 用户令）**：`stuck` 语义
//! **不补通道**——由 err 通道的 v／E 锚点原生覆盖（设计 §10.3）：压力轴 =
//! err 水平阈 ∨（v > 0＝正在恶化 ∧ E ≥ 包络阈＝未见收敛）。RLI **不再与
//! LIF 现域对照**（LIF 维持现役组件）；域一致性的口径改由离线观测件对
//! 「框架实际动作结果」核读（见 `rli_shadow_replay.rs` 与 0am 结束文档）。
//!
//! 锚点（设计 §3）：a1 `u`；a2 `v`（＋短视预测 û(t+T̂) ≈ u + v·T̂）；a3 解析
//! 包络 `E = φ·√(A²+B²)`；a4 节律计数 `r`（事件间隔近半周期整数倍时 +1，
//! 否则仅衰减）；**a5／a6 模态对**（0am 模态分离批，2026-09-20 用户裁决）：
//! 实极点分支上 `c_slow=(u+b)/2`、`c_fast=(u−b)/2`（`b=(v+ζω·u)/μ`），满足
//! `c_slow+c_fast=u` 且两级各自按自己的时间常数**解耦**衰减——即持久/瞬态
//! 配比，读数层替代相位提供方向性；复极点分支上无定义（返回 `NAN`，如实
//! 登记）。**口径注**：实极点分支上 `E ≡ u`（a3 是紧上界），故 `env_prog`
//! 与 `u_prog` 恒等冗余、**已撤名**（同日裁决；见
//! `docs/audits/0AM_RLI_MODE_SPLIT_2026-09-20.md`）；锚点序列采样为
//! **事件级**（决策轮 ＋ 每个工具事件；判决粒度不变，自判域仍只在决策轮）。
//! 阈值自校准（设计 §4）：`u ≥ θ → θ·e^(+η)`，否则
//! `θ·e^(−η·(1−q)/q)`——平衡点 `P(u≥θ)=1−q`（q=0.95 时过阈率 5%），零梯度、
//! 无速率/方差目标项。无复位（设计 §5）：过阈＝纯观测 + θ 更新，u/v 连续。
//!
//! 确定性契约（设计 §8）：逐事件固定次序（φ、C、S 各算一次）；三角函数与
//! 指数经 **pinned pure-Rust `libm`**（选项 A「固定 libm 版本」：跨平台同
//! crate 版本 ⇒ 同一浮点语义）；状态**存储定点化 3 位小数**（与 DynCtx 舍入
//! 契约同格）；同平台重放＝同一代码路径逐字段 bit-exact。
//!
//! v1 范围（本批）：err / stall / slow / deny / prog 五通道。**stuck 影子
//! 延后**——设计 §6 表格将 stuck 的注入权重 `w` 显式留「影子批语义推导」；
//! w 语义冻结前不落实现（现行 1D stuck 闭式通道维持生产不动，设计 §10.3
//! 「v1 并存」的裁决不受影响）。
//!
//! 初值登记（全部语义推导、禁拟合；影子批离线复核 = 设计 §10.1/§10.4）：
//! `η = 0.05`（对数空间步长；平衡过阈率恰 1−q 与 η 无关）、`θ₀ = 1.0`
//! （单事件权重单位）、`τ_r = 4×半周期`（节律遗忘窗）。
//!
//! [`RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16`]: ../../../../../docs/RLI_RESONANT_CHANNEL_BASE_DESIGN_2026-09-16.md

use std::collections::VecDeque;
use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use super::channels::{
    ChannelKind, SLOW_W_MAX, SLOW_WALL_MS_THRESHOLD, STALL_GAP_THRESHOLD_SECS, ToolEvent,
    ToolOutcome,
};
use super::temporal::Domain;

/// 影子基座公共参数（设计 §6 初值；语义推导、禁拟合）。
pub const RLI_ZETA: f64 = 0.5;
/// 配极改（0am 改造补充项①，2026-09-20）：**水平／新鲜度类通道**（prog）
/// 的实极点配置——ζ = 2.0（过阻尼常数；两个实极点 λ± = −ω(2 ∓ √3)：
/// 慢模态 τ ≈ 3.73/ω＝持久面、快模态 τ ≈ 0.27/ω＝适应面）。速率／压力类
/// （err／stall／slow／deny）沿用复极点对 [`RLI_ZETA`]。族不变（设计 §6：
/// 一阶＝强过阻尼极限），改的是配极与谓词（设计 §11 表）。
pub const RLI_PROG_ZETA: f64 = 2.0;
pub const RLI_Q: f64 = 0.95;
pub const RLI_ETA_INIT: f64 = 0.05;
pub const RLI_THETA_INIT: f64 = 1.0;
/// 节律遗忘窗：`τ_r = 4 × 半周期`（半周期 = π/ω_d）。设计 §10.4 复核项。
pub const RLI_TAU_R_HALF_PERIODS: f64 = 4.0;
/// 节律匹配容差（×半周期）：设计 §7 冻结值 `π/(4ω_d)` = 半周期/2。
/// 诊断切换见 [`RliChannel::set_rhythm_tolerance`]（读数对照用，不作数据选择）。
pub const RLI_RHYTHM_TOLERANCE_HALF_PERIODS: f64 = 0.5;
/// 预测步长（0am 改造四项②，2026-09-20 用户令「预测时间步先定 10」）：
/// a2 预期锚点以**二阶闭式自由演化精确解**对 `Δt = 10·T̂` 前推——不引入
/// 迭代、不引入噪声（见 [`RliChannel::prediction`]）。
pub const RLI_PREDICTION_STEPS: f64 = 10.0;
/// RLI 自判域谓词常数（0am 改造四项①，2026-09-20；0am 改造补充项②
/// 2026-09-20 重推导）——语义常数、禁拟合。压力轴 = err 水平阈 ∨ v／E
/// 原生覆盖项（`stuck` 语义不补通道，设计 §10.3）；见 [`RliDomainMachine`]。
pub const RLI_DOMAIN_PROG_LOW: f64 = 0.5;
pub const RLI_DOMAIN_ERR_PRESSURE: f64 = 2.0;
/// v／E 原生覆盖项的包络阈（0am 改造补充项②）：E ≥ 1.0 ＝ 单次激励单位
/// 仍未消退（θ₀ = w = 1 的语义基准），即「未见收敛」；配 v > 0（设计 §3
/// a2「正在恶化」）构成 stuck 语义的原生读法。
pub const RLI_DOMAIN_ERR_ENVELOPE_FLOOR: f64 = 1.0;
/// 锚点序列面（0am 改造补充项④，2026-09-20 用户令「PULL 面增锚点序列
/// 面」）：**事件级采样**（决策轮 ＋ 每个工具事件）、cap 20、**live-only**
/// （不随侧车持久化——与 LIF temporal 的 feature 序列同格）。
pub const RLI_FEATURE_SERIES_CAP: usize = 20;
/// 锚点序列名集合（与 [`RLI_ANCHOR_FEATURE_TABLE`] 同序、逐项对齐；测试
/// 钉住两侧不漂移）。判决通道（err／prog）锚点 ＋ **模态分离对**
/// （`slow_prog`／`fast_prog`，2026-09-20 用户裁决「拆模态系数＝做」，见
/// `docs/audits/0AM_RLI_MODE_SPLIT_2026-09-20.md`）。
///
/// 去冗余（2026-09-20 主会话裁决，用户授权「两名的去留和 `env_prog` 的
/// 去留请你裁决」）：**`env_prog` 撤名**——它在实极点分支上**恒等于**
/// `u_prog`（a3 是紧的上界，见该报告 §1），保留会让一条副本冒充独立读数轴。
/// `env_err` **保留**（复极点分支上 `E > |u|` 泛成立，独立）。
pub const RLI_ANCHOR_FEATURE_NAMES: [&str; 11] = [
    "u_err",
    "v_err",
    "pred_err",
    "env_err",
    "r_err",
    "u_prog",
    "v_prog",
    "pred_prog",
    "r_prog",
    "slow_prog",
    "fast_prog",
];
/// 自判域近期行窗口（镜像 temporal 的 `RECENT_RECORDS_CAP`）。
pub const RLI_DOMAIN_RECENT_CAP: usize = 20;
/// 侧车快照 schema 标识（随 [`RliShadowSnapshot`] 序列化）。
pub const RLI_SNAPSHOT_SCHEMA: &str = "rli-shadow-v1";
/// 状态存储定点化（3 位小数；同 DynCtx 舍入契约）。
pub const RLI_STATE_DECIMALS: f64 = 1_000.0;

/// 速率型通道周期语义（秒）——`ω = 2π/周期`（设计 §6 表）。
pub const RLI_ERR_PERIOD_SECS: f64 = 180.0;
pub const RLI_STALL_PERIOD_SECS: f64 = 90.0;
pub const RLI_SLOW_PERIOD_SECS: f64 = 600.0;
pub const RLI_DENY_PERIOD_SECS: f64 = 120.0;
/// 新鲜度型通道（轮语义）——`ω = 2π/(k·T̂)`。
pub const RLI_PROG_PERIOD_ROUNDS: f64 = 8.0;

/// 影子通道族（v1 五条；stuck 延后——见模块头）。
pub const RLI_CHANNELS: [ChannelKind; 5] = [
    ChannelKind::Err,
    ChannelKind::Stall,
    ChannelKind::Slow,
    ChannelKind::Deny,
    ChannelKind::Prog,
];

/// 3 位小数定点化（存储边界；`round` 半程远离零，与 DynCtx 契约同格）。
pub fn quantize_state(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    (x * RLI_STATE_DECIMALS).round() / RLI_STATE_DECIMALS
}

/// 通道极点配置查询（0am 配极改，2026-09-20）：速率／压力类 = [`RLI_ZETA`]
/// （复极点对：预期＋节律），水平／新鲜度类（prog）= [`RLI_PROG_ZETA`]
/// （实极点：持久／适应）。配极表本身即语义常数表（设计 §11）。
pub fn rli_zeta_for(kind: ChannelKind) -> f64 {
    match kind {
        ChannelKind::Prog => RLI_PROG_ZETA,
        ChannelKind::Err | ChannelKind::Stall | ChannelKind::Slow | ChannelKind::Deny => RLI_ZETA,
    }
}

/// 单锚点选取（锚点序列面用；与 [`RliAnchors`] 的 a1–a4 一一对应；θ/hits
/// 是判定面不属锚点面，不入序列）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RliAnchor {
    /// a1 水平。
    U,
    /// a2 变化率。
    V,
    /// a2 预期锚点（10·T̂ 闭式前推）。
    Pred,
    /// a3 解析包络。
    Env,
    /// a4 节律计数。
    Rhythm,
    /// a5 慢模态系数（**实极点分支专用**；`c_slow = (u + b)/2`）。
    ModeSlow,
    /// a6 快模态系数（**实极点分支专用**；`c_fast = (u − b)/2`）。
    ModeFast,
}

impl RliAnchor {
    /// 从通道取该锚点当前值（`t_hat_secs` 供预测锚点前推用）。
    pub fn of(self, ch: &RliChannel, t_hat_secs: f64) -> f64 {
        match self {
            RliAnchor::U => ch.u(),
            RliAnchor::V => ch.v(),
            RliAnchor::Pred => ch.prediction(t_hat_secs),
            RliAnchor::Env => ch.envelope(),
            RliAnchor::Rhythm => ch.rhythm(),
            RliAnchor::ModeSlow => ch.mode_slow(),
            RliAnchor::ModeFast => ch.mode_fast(),
        }
    }
}

/// 锚点序列表（名 → 通道 × 锚点；与 [`RLI_ANCHOR_FEATURE_NAMES`] 同序，
/// 测试钉住两侧不漂移）。
pub const RLI_ANCHOR_FEATURE_TABLE: [(&str, ChannelKind, RliAnchor); 11] = [
    ("u_err", ChannelKind::Err, RliAnchor::U),
    ("v_err", ChannelKind::Err, RliAnchor::V),
    ("pred_err", ChannelKind::Err, RliAnchor::Pred),
    ("env_err", ChannelKind::Err, RliAnchor::Env),
    ("r_err", ChannelKind::Err, RliAnchor::Rhythm),
    ("u_prog", ChannelKind::Prog, RliAnchor::U),
    ("v_prog", ChannelKind::Prog, RliAnchor::V),
    ("pred_prog", ChannelKind::Prog, RliAnchor::Pred),
    ("r_prog", ChannelKind::Prog, RliAnchor::Rhythm),
    ("slow_prog", ChannelKind::Prog, RliAnchor::ModeSlow),
    ("fast_prog", ChannelKind::Prog, RliAnchor::ModeFast),
];

/// 单通道锚点读数（S3 回放导出面；只读快照）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliAnchors {
    /// a1 水平。
    pub u: f64,
    /// a2 变化率（1/s）。
    pub v: f64,
    /// a2 短视预测 û(t+T̂) ≈ u + v·T̂。
    pub prediction: f64,
    /// a3 解析包络（自由演化单调衰减；注入处跃升）。
    pub envelope: f64,
    /// a4 节律计数。
    pub rhythm: f64,
    /// a5 慢模态系数（**实极点分支专用**；复极点分支为 `NAN`）。
    ///
    /// `c_slow = (u + b)/2`，`b = (v + ζω·u)/μ`；与 `c_fast` 满足
    /// `c_slow + c_fast = u`，且两者各自按**自己的**时间常数独立衰减
    /// （解耦）——即持久/瞬态配比，读数层替代相位提供方向性。
    pub mode_slow: f64,
    /// a6 快模态系数（**实极点分支专用**；复极点分支为 `NAN`）。
    pub mode_fast: f64,
    /// 自校准阈值（当前值）。
    pub theta: f64,
    /// 过阈累计（纯内部观测；无注入面）。
    pub hits: u64,
}

/// 单通道 RLI 影子状态。
#[derive(Debug, Clone)]
pub struct RliChannel {
    kind: ChannelKind,
    omega: f64,
    zeta: f64,
    q: f64,
    eta: f64,
    rhythm_tolerance: f64,
    u: f64,
    v: f64,
    theta: f64,
    rhythm: f64,
    envelope: f64,
    last_step_t: Option<f64>,
    last_inject_t: Option<f64>,
    hit_count: u64,
    last_hit_t: Option<f64>,
}

impl RliChannel {
    fn new(kind: ChannelKind, omega: f64) -> Self {
        Self {
            kind,
            omega: omega.max(1e-9),
            zeta: rli_zeta_for(kind),
            q: RLI_Q,
            eta: RLI_ETA_INIT,
            rhythm_tolerance: RLI_RHYTHM_TOLERANCE_HALF_PERIODS,
            u: 0.0,
            v: 0.0,
            theta: RLI_THETA_INIT,
            rhythm: 0.0,
            envelope: 0.0,
            last_step_t: None,
            last_inject_t: None,
            hit_count: 0,
            last_hit_t: None,
        }
    }

    pub fn kind(&self) -> ChannelKind {
        self.kind
    }

    pub fn omega(&self) -> f64 {
        self.omega
    }

    pub fn zeta(&self) -> f64 {
        self.zeta
    }

    pub fn u(&self) -> f64 {
        self.u
    }

    pub fn v(&self) -> f64 {
        self.v
    }

    pub fn theta(&self) -> f64 {
        self.theta
    }

    pub fn rhythm(&self) -> f64 {
        self.rhythm
    }

    pub fn envelope(&self) -> f64 {
        self.envelope
    }

    pub fn hit_count(&self) -> u64 {
        self.hit_count
    }

    pub fn last_hit_t(&self) -> Option<f64> {
        self.last_hit_t
    }

    pub fn last_inject_t(&self) -> Option<f64> {
        self.last_inject_t
    }

    /// a2 预期锚点（0am 改造四项②，2026-09-20）：以闭式自由演化精确解对
    /// `Δt = RLI_PREDICTION_STEPS · T̂` 前推（二阶解；退役一阶短视近似
    /// `u + v·T̂`，不引入迭代、不引入噪声）。T̂ 非法（非有限 / ≤ 0）时返回
    /// 当前水平 `u`（保守落点，不虚构前推）。
    pub fn prediction(&self, t_hat_secs: f64) -> f64 {
        if !t_hat_secs.is_finite() || t_hat_secs <= 0.0 {
            return self.u;
        }
        self.free_evolution_at(RLI_PREDICTION_STEPS * t_hat_secs).0
    }

    /// 闭式自由演化单点求值（不改状态）：把当前 `(u, v)` 按设计 §2 精确
    /// 推进 `dt`，返回 `(u₁, v₁, E₁)`，其中复极点分支
    /// `E₁ = φ·√(A² + B²)`、实极点分支 `E₁ = φ·(|A|·cosh + |B'|·sinh)`
    /// 为解析包络。与 [`Self::advance`] 共用同一公式（同一次 φ/C/S 或
    /// φ/cosh/sinh 计算），保证前推读数与后续实际推进逐字段一致（半群性）。
    fn free_evolution_at(&self, dt: f64) -> (f64, f64, f64) {
        let zeta_w = self.zeta * self.omega;
        let phi = libm::exp(-zeta_w * dt);
        if self.is_overdamped() {
            let mu = self.mu();
            let ch = libm::cosh(mu * dt);
            let sh = libm::sinh(mu * dt);
            let a = self.u;
            let b = (self.v + zeta_w * self.u) / mu;
            return (
                phi * (a * ch + b * sh),
                phi * (self.v * ch - (zeta_w * self.v + self.omega * self.omega * a) / mu * sh),
                phi * (a.abs() * ch + b.abs() * sh),
            );
        }
        let wd = self.omega_d();
        let c = libm::cos(wd * dt);
        let s = libm::sin(wd * dt);
        let a = self.u;
        let b = (self.v + zeta_w * self.u) / wd;
        (
            phi * (a * c + b * s),
            phi * ((wd * b - zeta_w * a) * c - (wd * a + zeta_w * b) * s),
            phi * libm::sqrt(a * a + b * b),
        )
    }

    /// 四锚点 + θ/命中读数（S3 导出面）。
    pub fn anchors(&self, t_hat_secs: f64) -> RliAnchors {
        RliAnchors {
            u: self.u,
            v: self.v,
            prediction: self.prediction(t_hat_secs),
            envelope: self.envelope,
            rhythm: self.rhythm,
            mode_slow: self.mode_slow(),
            mode_fast: self.mode_fast(),
            theta: self.theta,
            hits: self.hit_count,
        }
    }

    /// 阻尼比切换（**诊断对照专用**：C3 对照——0am 改造补充项③起按
    /// 「固定 ω_d 只扫 ζ」的分离口径使用，衰减与频率两效应分列；生产恒
    /// [`rli_zeta_for`]）。钳制到 `[0, 4.0]`（复极点 ↔ 过阻尼实极点全区间；
    /// ζ = 1 临界点由实极点分支的 μ 下限兜底）。
    pub fn set_zeta(&mut self, zeta: f64) {
        self.zeta = zeta.clamp(0.0, 4.0);
    }

    /// 极点对联合切换（**诊断对照专用**：C3 分离对照需要在保持一个不变量
    /// ——ω_d 或衰减 σ = ζω——的前提下改动另一个）。`omega` 先按语义下限
    /// 钳制，`zeta` 同 [`Self::set_zeta`]。
    pub fn set_poles(&mut self, omega: f64, zeta: f64) {
        self.omega = omega.max(1e-9);
        self.set_zeta(zeta);
    }

    /// 节律匹配容差切换（×半周期；**读数对照专用**，不作数据驱动选择）。
    pub fn set_rhythm_tolerance(&mut self, half_periods: f64) {
        self.rhythm_tolerance = half_periods.max(0.0);
    }

    /// 复极点分支（ζ < 1）的阻尼频率；实极点分支无此量（调用方先判分支）。
    fn omega_d(&self) -> f64 {
        // 欠阻尼（ζ < 1）：ω_d = ω·√(1−ζ²) > 0；ζ ≥ 1 时返回 0（不使用）。
        self.omega * (1.0 - self.zeta * self.zeta).max(0.0).sqrt()
    }

    /// 实极点分支（ζ > 1）的双曲频率 μ = ω·√(ζ²−1)。ζ → 1 邻域由下限
    /// 1e−12 兜底（临界双根的极限式在数值上连续：sinh(μΔt)/μ → Δt）。
    fn mu(&self) -> f64 {
        (self.omega * (self.zeta * self.zeta - 1.0).max(0.0).sqrt()).max(1e-12)
    }

    /// 是否走实极点分支（配极改：prog 恒实极点；诊断 ζ > 1 时一并成立）。
    pub fn is_overdamped(&self) -> bool {
        self.zeta > 1.0
    }

    /// 实极点分支的模态偏置 `b = (v + ζω·u)/μ`（复极点分支无此量）。
    fn mode_skew(&self) -> f64 {
        (self.v + self.zeta * self.omega * self.u) / self.mu()
    }

    /// a5 慢模态系数 `c_slow = (u + b)/2`（**实极点分支专用**，2026-09-20
    /// 用户裁决「拆模态系数＝做」；见 `docs/audits/0AM_RLI_MODE_SPLIT_2026-09-20.md`）。
    ///
    /// 语义：把当前水平拆成「会留下来的持久面」与「马上要消失的适应面」，
    /// 两者满足 `c_slow + c_fast = u`，且各自按**自己的**时间常数独立衰减
    /// （慢 `1/(ω(ζ−√(ζ²−1)))`／快 `1/(ω(ζ+√(ζ²−1)))`；ζ=2 时比 13.9:1，
    /// 以决策轮计约 4.75 轮／0.34 轮，与 T̂ 无关）。复极点分支「慢/快」无
    /// 定义（两模态共轭、衰减率相同），返回 `NAN`——如实登记、不虚构语义。
    pub fn mode_slow(&self) -> f64 {
        if self.is_overdamped() {
            (self.u + self.mode_skew()) / 2.0
        } else {
            f64::NAN
        }
    }

    /// a6 快模态系数 `c_fast = (u − b)/2`（**实极点分支专用**）。
    ///
    /// 注入当刻可为负（ζ=2 时 `b > u`）：负值表示水平最初被该瞬态「扣」掉
    /// 一部分，随后该分量以约 0.34 个决策轮的时间常数消失——属正常 LIF
    /// 动力学（检测粒度宽于该进程），**不设修正**（用户 2026-09-20 口径）。
    pub fn mode_fast(&self) -> f64 {
        if self.is_overdamped() {
            (self.u - self.mode_skew()) / 2.0
        } else {
            f64::NAN
        }
    }

    /// 节律时间尺（半周期类比）：复极点 = π/ω_d（真实半周期）；实极点 =
    /// π/μ（双曲类比——实极点无振荡，r 在该分支退化为纯衰减计数，如实
    /// 登记而非虚构节律语义）。
    fn half_period(&self) -> f64 {
        if self.is_overdamped() {
            PI / self.mu()
        } else {
            PI / self.omega_d()
        }
    }

    /// 自由演化推进到 `t`：闭式精确解（φ、C、S 各算一次）。
    pub fn advance(&mut self, t: f64) {
        let Some(t0) = self.last_step_t else {
            self.last_step_t = Some(t);
            self.refresh_envelope(1.0);
            return;
        };
        let dt = (t - t0).max(0.0);
        self.last_step_t = Some(t);
        if dt <= 0.0 {
            return;
        }
        let (u1, v1, e1) = self.free_evolution_at(dt);
        self.u = u1;
        self.v = v1;
        self.envelope = e1;
    }

    /// 当前包络（无演化时按 φ=1 口径刷新，保证读数始终与 (u,v) 同态）。
    /// 复极点分支 = φ·√(A²+B²)（相位幅值）；实极点分支在相位零基点退化为
    /// φ·|u|（双曲包络 t=0 值；注入只踢 u，E 与水平同尺——登记口径）。
    fn refresh_envelope(&mut self, phi: f64) {
        if self.is_overdamped() {
            self.envelope = phi * self.u.abs();
            return;
        }
        let wd = self.omega_d();
        let a = self.u;
        let b = (self.v + self.zeta * self.omega * self.u) / wd;
        self.envelope = phi * libm::sqrt(a * a + b * b);
    }

    /// 事件注入：`u ← u + w`（v 不踢）；节律计数同步记账（设计 §3 a4）。
    pub fn inject(&mut self, t: f64, w: f64) {
        self.advance(t);
        self.note_arrival(t);
        self.u += w;
        self.refresh_envelope(1.0);
    }

    /// 新鲜度型通道（prog）：成功置 1（同现行 1D `prog` 语义）。
    pub fn inject_set(&mut self, t: f64, value: f64) {
        self.advance(t);
        self.note_arrival(t);
        self.u = value;
        self.refresh_envelope(1.0);
    }

    /// 节律记账：`r ← r·e^(−Δt/τ_r)`；间隔近似半周期整数倍时 `+1`。
    ///
    /// 注（2026-09-17 影子批核读）：设计容差 `π/(4ω_d)`（= 半周期/2）下
    /// k=1,2,… 的匹配窗并集覆盖 `Δt ≥ 半周期/2`（无空隙）——即字面实现等效
    /// 于「非突发间隔计数」。读数与对照见模块测试；容差复核挂设计 §10.4。
    fn note_arrival(&mut self, t: f64) {
        let half = self.half_period();
        let Some(prev) = self.last_inject_t else {
            self.last_inject_t = Some(t);
            return;
        };
        let dt = (t - prev).max(0.0);
        self.last_inject_t = Some(t);
        if dt <= 0.0 {
            return;
        }
        let tau_r = RLI_TAU_R_HALF_PERIODS * half;
        self.rhythm *= libm::exp(-dt / tau_r);
        let k = (dt / half).round().max(1.0);
        if (dt - k * half).abs() <= self.rhythm_tolerance * half {
            self.rhythm += 1.0;
        }
    }

    /// 阈值评估与分位数自校准（设计 §4）：返回是否过阈（hit）。
    pub fn observe_value(&mut self, t: f64, u: f64) -> bool {
        if u >= self.theta {
            self.theta *= libm::exp(self.eta);
            self.hit_count = self.hit_count.saturating_add(1);
            self.last_hit_t = Some(t);
            true
        } else {
            self.theta *= libm::exp(-self.eta * (1.0 - self.q) / self.q);
            false
        }
    }

    /// 步检查：以当前 `u` 做一次阈值评估（调用方先 advance/注入）。
    pub fn check(&mut self, t: f64) -> bool {
        self.observe_value(t, self.u)
    }

    /// 存储快照（3 位小数定点化）。
    ///
    /// 定点化面＝**动力学状态与时间戳**（u/v/θ/r/E + last_*）。参数面
    /// （ω/ζ）不做 3 位小数定点化：ω 的量级 ~1e-2，3 位小数会引入 ~0.3%
    /// 周期漂移（超存储契约收益）；参数随代码常数与 T̂ 派生，重放面本就
    /// 同源。
    pub fn snapshot(&self) -> RliChannelSnapshot {
        RliChannelSnapshot {
            kind: self.kind,
            omega: self.omega,
            zeta: self.zeta,
            u: quantize_state(self.u),
            v: quantize_state(self.v),
            theta: quantize_state(self.theta),
            rhythm: quantize_state(self.rhythm),
            envelope: quantize_state(self.envelope),
            last_step_t: self.last_step_t.map(quantize_state),
            last_inject_t: self.last_inject_t.map(quantize_state),
            hit_count: self.hit_count,
            last_hit_t: self.last_hit_t.map(quantize_state),
        }
    }

    /// 从快照续接。ζ/q/η/容差为代码拥有的常数（不随侧车恢复——契约随代码
    /// 版本走）；ω 与全部动力学状态按快照恢复。
    ///
    /// **0bc FR-6（2026-09-21）**：快照里已带 `zeta`（观测/回放面），续接时
    /// **采纳**它（sanitize 后）——离线回放与在线续接的极点必须一致，否则
    /// 「侧车续接＝同一动力学」的读法在 ζ 改动后失真。钳制口径与
    /// [`Self::set_zeta`] 同（`[0,4]`）；异常/legacy 值（NaN、越界）落回代码
    /// 默认常数（宁缺勿假：不采信一个不可信极点到动力学里）。
    pub fn restore(&mut self, snapshot: &RliChannelSnapshot) {
        self.omega = snapshot.omega.max(1e-9);
        if snapshot.zeta.is_finite() {
            self.zeta = snapshot.zeta.clamp(0.0, 4.0);
        }
        self.u = snapshot.u;
        self.v = snapshot.v;
        self.theta = snapshot.theta.max(1e-9);
        self.rhythm = snapshot.rhythm.max(0.0);
        self.envelope = snapshot.envelope;
        self.last_step_t = snapshot.last_step_t;
        self.last_inject_t = snapshot.last_inject_t;
        self.hit_count = snapshot.hit_count;
        self.last_hit_t = snapshot.last_hit_t;
    }
}

/// RLI 影子通道族引擎——与 [`super::LifEngine`] 同一事件流（影子旁路）。
#[derive(Debug, Clone)]
pub struct RliShadow {
    channels: [RliChannel; RLI_CHANNELS.len()],
    t_hat: f64,
    last_tool_t: Option<f64>,
    steps: u64,
    /// 0am 改造四项①（2026-09-20）：自判域状态机（LIF 域状态机职能平移
    /// ——RLI 用自己的锚点自判动作域；0am 改造补充项②起不再与 LIF 现域
    /// 对照，域一致性口径改对「框架实际动作结果」，见离线观测件）。
    domain: RliDomainMachine,
    /// 0am 改造补充项④（2026-09-20）：锚点序列（决策轮粒度、cap
    /// [`RLI_FEATURE_SERIES_CAP`]、live-only 不随侧车持久化）。与
    /// [`RLI_ANCHOR_FEATURE_TABLE`] 同序。
    anchor_series: Vec<VecDeque<f64>>,
}

impl Default for RliShadow {
    fn default() -> Self {
        Self::new()
    }
}

impl RliShadow {
    /// 全新影子（T̂₀ = 8s 预启用节奏，同 1D 引擎口径）。
    pub fn new() -> Self {
        let t_hat0 = 8.0;
        Self {
            channels: [
                RliChannel::new(ChannelKind::Err, TAU / RLI_ERR_PERIOD_SECS),
                RliChannel::new(ChannelKind::Stall, TAU / RLI_STALL_PERIOD_SECS),
                RliChannel::new(ChannelKind::Slow, TAU / RLI_SLOW_PERIOD_SECS),
                RliChannel::new(ChannelKind::Deny, TAU / RLI_DENY_PERIOD_SECS),
                RliChannel::new(ChannelKind::Prog, TAU / (RLI_PROG_PERIOD_ROUNDS * t_hat0)),
            ],
            t_hat: t_hat0,
            last_tool_t: None,
            steps: 0,
            domain: RliDomainMachine::new(),
            anchor_series: (0..RLI_ANCHOR_FEATURE_TABLE.len())
                .map(|_| VecDeque::with_capacity(RLI_FEATURE_SERIES_CAP))
                .collect(),
        }
    }

    fn index_of(kind: ChannelKind) -> usize {
        match kind {
            ChannelKind::Err => 0,
            ChannelKind::Stall => 1,
            ChannelKind::Slow => 2,
            ChannelKind::Deny => 3,
            ChannelKind::Prog => 4,
        }
    }

    pub fn channel(&self, kind: ChannelKind) -> &RliChannel {
        &self.channels[Self::index_of(kind)]
    }

    pub fn channel_mut(&mut self, kind: ChannelKind) -> &mut RliChannel {
        let idx = Self::index_of(kind);
        &mut self.channels[idx]
    }

    pub fn channels(&self) -> &[RliChannel] {
        &self.channels
    }

    pub fn t_hat(&self) -> f64 {
        self.t_hat
    }

    /// 已观测步数（决策轮 + 工具事件）——过阈率读数的分母口径。
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// 自判域状态机（0am 改造四项①；PULL 面与域一致性读数面）。
    pub fn domain(&self) -> &RliDomainMachine {
        &self.domain
    }

    /// 锚点序列（0am 改造补充项④，2026-09-20；**采样粒度＝事件级**，同日
    /// 用户裁决）：最近 k 个采样（决策轮 ＋ 每个工具事件，旧在前）；
    /// 未知名 / 无样本 = 空序列。live-only（不随侧车持久化）。
    pub fn feature(&self, name: &str, k: u64) -> Vec<f64> {
        let Some(idx) = RLI_ANCHOR_FEATURE_TABLE
            .iter()
            .position(|(n, _, _)| *n == name)
        else {
            return Vec::new();
        };
        let series = &self.anchor_series[idx];
        let k = (k as usize).min(series.len());
        series.iter().skip(series.len() - k).copied().collect()
    }

    /// 已知锚点序列名（PULL 面 `feature` 的 name 校验面）。
    pub fn known_feature_names() -> &'static [&'static str] {
        &RLI_ANCHOR_FEATURE_NAMES
    }

    /// 锚点序列采样（**事件级**，2026-09-20 用户裁决「采样尽可能细」）：
    /// **决策轮与每个工具事件各采一次**（旧在前、cap [`RLI_FEATURE_SERIES_CAP`]）。
    ///
    /// 两条理由：① 对话 run 长度不可控——决策轮粒度在短 run 上样本过少，
    /// 细粒度才能在短对话里保留分辨力；② 快模态分量在**注入当刻最大**，
    /// 轮粒度永远取不到它（0am 模态分离批读数：轮粒度下 `|c_slow−u|`
    /// 均值 0.0043，即第二轴被采样粒度抹平）。
    ///
    /// **采样粒度 ≠ 判决粒度**：自判域仍只在决策轮记录（域是决策点语义，
    /// 不随观测面变细而变密）。`t_hat` 取最近一次决策轮的轮语义估计——
    /// 事件间沿用上一轮值，如实口径。
    fn sample_anchor_series(&mut self) {
        let t_hat = self.t_hat;
        let values: Vec<f64> = RLI_ANCHOR_FEATURE_TABLE
            .iter()
            .map(|(_, kind, anchor)| anchor.of(self.channel(*kind), t_hat))
            .collect();
        for (series, value) in self.anchor_series.iter_mut().zip(values) {
            series.push_back(value);
            if series.len() > RLI_FEATURE_SERIES_CAP {
                series.pop_front();
            }
        }
    }

    /// 决策轮：更新轮语义 ω（prog = 2π/(8·T̂)），全通道自由演化 + 逐一阈值
    /// 评估 + 自判域记录（0am 改造四项①）+ 锚点序列采样（补充项④）。
    /// 域判决输入＝RLI 原生锚点（err 的 u/v/E ＋ prog 的 u；见
    /// [`RliDomainMachine::label`]）；**不再携带 LIF 现域**（补充项②：
    /// RLI 不与 LIF 对照，一致性口径改对「框架实际动作结果」）。
    pub fn on_decision_round(&mut self, t: f64, t_hat_secs: f64) {
        self.t_hat = t_hat_secs.max(1e-9);
        self.steps = self.steps.saturating_add(1);
        let omega_prog = TAU / (RLI_PROG_PERIOD_ROUNDS * self.t_hat);
        for ch in &mut self.channels {
            if ch.kind == ChannelKind::Prog {
                ch.omega = omega_prog;
            }
            ch.advance(t);
        }
        for ch in &mut self.channels {
            ch.check(t);
        }
        let u_err = self.channel(ChannelKind::Err).u();
        let v_err = self.channel(ChannelKind::Err).v();
        let env_err = self.channel(ChannelKind::Err).envelope();
        let u_prog = self.channel(ChannelKind::Prog).u();
        self.domain.record_round(t, u_err, u_prog, v_err, env_err);
        // 锚点序列采样（**事件级**：决策轮与每个工具事件各一次；见
        // [`Self::sample_anchor_series`]）。
        self.sample_anchor_series();
    }

    /// 工具事件：间隔看门狗（stall）→ 注入（err/deny/prog/slow）→ 阈值评估
    /// → 锚点采样（事件级，2026-09-20）。与 1D 引擎同序：先推进（自由演化），
    /// 再注入，再检查。
    pub fn on_tool_event(&mut self, t: f64, event: ToolEvent) {
        let long_gap = matches!(
            self.last_tool_t,
            Some(prev) if (t - prev) > STALL_GAP_THRESHOLD_SECS
        );
        self.last_tool_t = Some(t);
        self.steps = self.steps.saturating_add(1);
        for ch in &mut self.channels {
            ch.advance(t);
        }
        match event.outcome {
            ToolOutcome::Error => self.channel_mut(ChannelKind::Err).inject(t, 1.0),
            ToolOutcome::Deny => self.channel_mut(ChannelKind::Deny).inject(t, 1.0),
            ToolOutcome::Success => self.channel_mut(ChannelKind::Prog).inject_set(t, 1.0),
            ToolOutcome::Other => {}
        }
        if let Some(wall_ms) = event.wall_ms
            && wall_ms > SLOW_WALL_MS_THRESHOLD
        {
            let w = ((wall_ms as f64) / 60_000.0).clamp(1.0, SLOW_W_MAX);
            self.channel_mut(ChannelKind::Slow).inject(t, w);
        }
        if long_gap {
            self.channel_mut(ChannelKind::Stall).inject(t, 1.0);
        }
        // 0am 改造四项①：自判域的成功门（与 LIF temporal 的 has_success 同
        // 口径——Start 仅在首次成功前出现）。
        self.domain.observe_tool_outcome(event.outcome);
        for ch in &mut self.channels {
            ch.check(t);
        }
        // 锚点序列采样（事件级；注入后取——快模态分量在注入当刻最大，
        // 决策轮粒度取不到，见 [`Self::sample_anchor_series`]）。
        self.sample_anchor_series();
    }

    /// 快照（3 位小数定点化；随会话侧车持久化）。自判域机器状态一并携带
    /// （0am 改造四项①；字段为兼容扩展——旧侧车无 `domain` 字段照常解析）。
    pub fn snapshot(&self) -> RliShadowSnapshot {
        RliShadowSnapshot {
            schema: RLI_SNAPSHOT_SCHEMA.to_string(),
            t_hat: quantize_state(self.t_hat),
            steps: self.steps,
            channels: self.channels.iter().map(RliChannel::snapshot).collect(),
            domain: Some(self.domain.snapshot()),
        }
    }

    /// 从快照恢复（schema 与通道族完备性校验；失败返回 `false`，状态不动）。
    pub fn restore(&mut self, snapshot: &RliShadowSnapshot) -> bool {
        if snapshot.schema != RLI_SNAPSHOT_SCHEMA {
            return false;
        }
        if snapshot.channels.len() != RLI_CHANNELS.len() {
            return false;
        }
        let mut restored = [false; RLI_CHANNELS.len()];
        for ch_snap in &snapshot.channels {
            let idx = Self::index_of(ch_snap.kind);
            if restored[idx] {
                return false;
            }
            restored[idx] = true;
        }
        for ch_snap in &snapshot.channels {
            let idx = Self::index_of(ch_snap.kind);
            self.channels[idx].restore(ch_snap);
        }
        self.t_hat = snapshot.t_hat.max(1e-9);
        self.steps = snapshot.steps;
        // 0am 改造四项①：自判域机器随快照续接；legacy 快照（无该字段）保持
        // fresh 自判域机器（通道/时间轴照常续接，域机器重新累积）。
        if let Some(domain) = &snapshot.domain {
            self.domain.restore(domain);
        }
        true
    }
}

/// RLI 自判域单行（决策轮粒度；域一致性读数面）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliDomainRow {
    /// 会话相对决策轮（与 shadow 喂入同轴；逐轮 +1）。
    pub round: u64,
    /// 会话相对墙钟秒（与 LIF temporal 行同轴）。
    pub t: f64,
    /// RLI 自判域。
    pub domain: Domain,
    /// 当前域段入域轮。
    pub entry_round: u64,
    /// 当前域段驻留轮数。
    pub dwell_rounds: u64,
    /// 判决输入：RLI err 通道水平。
    pub u_err: f64,
    /// 判决输入：RLI prog 通道水平。
    pub u_prog: f64,
    /// 判决输入（0am 改造补充项②）：err 通道变化率 v——压力轴第二项的一半
    /// （v > 0 ＝ 正在恶化）。
    ///
    /// **0bc FR-7（2026-09-21）**：`Option<f64>`——legacy 行（字段诞生前）
    /// 是 `None`（**未记录**），与真实读数 `Some(0.0)`（恰好为零）必须可
    /// 分辨（FR7 口径：不可得与真值 0 不混同，同 [`RliDomainSpike::round`]）。
    /// 序列化面 legacy 行不再落 `0.0` 假读数。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub v_err: Option<f64>,
    /// 判决输入（0am 改造补充项②）：err 通道解析包络 E——压力轴第二项的
    /// 另一半（E ≥ 包络阈 ＝ 未见收敛）。0bc FR-7：同 [`Self::v_err`]，
    /// `Option<f64>`（`None` ＝ legacy 未记录）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_err: Option<f64>,
}

/// RLI 自判域状态机（0am 改造四项①，2026-09-20；补充项② 2026-09-20
/// 重推导）：LIF `TemporalState` 域状态机职能平移到 RLI——同一 `Domain`
/// 闭枚举与迁移语义、同一 `has_success` 成功门，判决输入换成 RLI 自身
/// 锚点：
///
/// - 低进度轴 = err 的 u_prog 阈（同 LIF 形状）；
/// - 压力轴 = err 水平阈 **∨** v／E 原生覆盖项（`v > 0 ∧ E ≥ 包络阈`，
///   设计 §10.3：**不补 `stuck` 通道**——stuck 语义由 v／E 原生承担）。
///
/// 每决策轮记录一行 + 域切换 spike（`t` 与 LIF temporal 同轴）。**不再
/// 记录与 LIF 现域的一致性**（补充项②：RLI 不与 LIF 对照；域一致性口径
/// 改对「框架实际动作结果」，由离线观测件核读）。
#[derive(Debug, Clone)]
pub struct RliDomainMachine {
    round: u64,
    has_success: bool,
    current: Domain,
    entry_round: u64,
    rows: VecDeque<RliDomainRow>,
    spikes: Vec<RliDomainSpike>,
}

/// RLI 域切换点位（**RLI 自有类型**，不复用 temporal 的 [`DomainSpike`]）。
///
/// 除时刻与域之外多带**入域轮次**——这是「外挂时间组件」的定位要素：
/// 机械层设计 §3 把 temporal 定位为「时间轴上的特征域事实序列」，§4 是
/// 「LIF 时间外挂计算规格」；模型要拿它做**进度定位与回看**，就必须能把
/// 域事实对齐到轮次，而不只是墙钟秒。
///
/// `round = None` 表示**旧侧车未记录**（FR7 口径：不可得与真值 0 必须可
/// 分辨，故用 `Option` 而非 `0`）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliDomainSpike {
    pub t: f64,
    pub domain: Domain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<u64>,
}

/// 域段（History／回看面的定位要素；由相邻切换点推导，不单独存储——
/// 与 temporal §3.1「驻留轮数由相邻 spike 推导，不单独存储」同口径）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RliDomainSegment {
    /// 前一段的域（首个切换点为 `None`＝run 起点前）。
    pub from: Option<Domain>,
    pub to: Domain,
    pub at_t: f64,
    pub at_round: Option<u64>,
    /// 本段驻留轮数（到下一次切换；末段用当前轮）。
    pub dwell_rounds: Option<u64>,
    /// 恢复标记（设计 §3.2 Recovery）：Stuck／LowProgress → Normal。
    pub recovery: bool,
}

impl Default for RliDomainMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl RliDomainMachine {
    pub fn new() -> Self {
        Self {
            round: 0,
            has_success: false,
            current: Domain::Start,
            entry_round: 0,
            rows: VecDeque::with_capacity(RLI_DOMAIN_RECENT_CAP),
            spikes: Vec::new(),
        }
    }

    /// 成功门（与 LIF temporal `observe_tool_outcome` 的 has_success 同
    /// 口径——Start 仅在首次成功前出现）。
    pub fn observe_tool_outcome(&mut self, outcome: ToolOutcome) {
        if matches!(outcome, ToolOutcome::Success) {
            self.has_success = true;
        }
    }

    /// 语义域谓词（0am 改造补充项②重推导；四象与 LIF temporal 同形而
    /// 压力轴换 RLI 原生读法）：
    ///
    /// ```text
    /// low_progress = u_prog < RLI_DOMAIN_PROG_LOW
    /// pressure     = u_err ≥ RLI_DOMAIN_ERR_PRESSURE
    ///                ∨ (v_err > 0 ∧ env_err ≥ RLI_DOMAIN_ERR_ENVELOPE_FLOOR)
    /// ```
    ///
    /// 第二项即设计 §10.3 的 v／E 原生覆盖（正在恶化 ∧ 未见收敛——stuck
    /// 语义），**不补通道**。
    pub fn label(&self, u_err: f64, u_prog: f64, v_err: f64, env_err: f64) -> Domain {
        if !self.has_success {
            return Domain::Start;
        }
        let low_progress = u_prog < RLI_DOMAIN_PROG_LOW;
        let native_pressure = v_err > 0.0 && env_err >= RLI_DOMAIN_ERR_ENVELOPE_FLOOR;
        let pressure = u_err >= RLI_DOMAIN_ERR_PRESSURE || native_pressure;
        match (low_progress, pressure) {
            (false, false) => Domain::Normal,
            (false, true) => Domain::Pressure,
            (true, false) => Domain::LowProgress,
            (true, true) => Domain::Stuck,
        }
    }

    /// 记录一决策轮：自判 + 域切换 spike；返回本轮行。
    pub fn record_round(
        &mut self,
        t: f64,
        u_err: f64,
        u_prog: f64,
        v_err: f64,
        env_err: f64,
    ) -> RliDomainRow {
        self.round = self.round.saturating_add(1);
        let domain = self.label(u_err, u_prog, v_err, env_err);
        if domain != self.current {
            self.spikes.push(RliDomainSpike {
                t,
                domain,
                round: Some(self.round),
            });
            self.current = domain;
            self.entry_round = self.round;
        }
        let row = RliDomainRow {
            round: self.round,
            t,
            domain,
            entry_round: self.entry_round,
            dwell_rounds: self.round.saturating_sub(self.entry_round) + 1,
            u_err,
            u_prog,
            // 0bc FR-7：在本轮记下的行就是"记录过"的行 → Some。
            v_err: Some(v_err),
            env_err: Some(env_err),
        };
        self.rows.push_back(row);
        if self.rows.len() > RLI_DOMAIN_RECENT_CAP {
            self.rows.pop_front();
        }
        row
    }

    pub fn round(&self) -> u64 {
        self.round
    }

    pub fn current_domain(&self) -> Domain {
        self.current
    }

    pub fn has_success(&self) -> bool {
        self.has_success
    }

    pub fn entry_round(&self) -> u64 {
        self.entry_round
    }

    /// 最新一行（Now）。
    pub fn now(&self) -> Option<&RliDomainRow> {
        self.rows.back()
    }

    /// Recent(k) —— 最近 k 行（旧在前）。
    pub fn recent(&self, k: u64) -> Vec<&RliDomainRow> {
        let k = (k as usize).min(self.rows.len());
        self.rows.iter().skip(self.rows.len() - k).collect()
    }

    /// 域切换时间线（会话内全量；显示与导出面）。
    pub fn spikes(&self) -> &[RliDomainSpike] {
        &self.spikes
    }

    /// 域段（History／回看面）：由相邻切换点推导 `from → to`、入域轮次、
    /// 驻留轮数与恢复标记——**定位要素**（机械层设计 §3「时间轴上的特征域
    /// 事实序列」／§4「LIF 时间外挂」，模型据此做进度定位与回看）。
    /// 末段驻留用当前轮号收口。全 run 无窗口（`rows` 的 20 窗只管运行时行）。
    pub fn segments(&self) -> Vec<RliDomainSegment> {
        let mut out: Vec<RliDomainSegment> = Vec::with_capacity(self.spikes.len());
        for (idx, spike) in self.spikes.iter().enumerate() {
            let from = idx.checked_sub(1).map(|p| self.spikes[p].domain);
            let next_round = self
                .spikes
                .get(idx + 1)
                .map(|s| s.round)
                .unwrap_or(Some(self.round));
            let dwell_rounds = match (spike.round, next_round) {
                (Some(a), Some(b)) => Some(b.saturating_sub(a)),
                _ => None,
            };
            let recovery = matches!(from, Some(Domain::Stuck | Domain::LowProgress))
                && spike.domain == Domain::Normal;
            out.push(RliDomainSegment {
                from,
                to: spike.domain,
                at_t: spike.t,
                at_round: spike.round,
                dwell_rounds,
                recovery,
            });
        }
        out
    }

    /// 快照（随 RLI 影子快照入会话侧车）。
    pub fn snapshot(&self) -> RliDomainSnapshot {
        RliDomainSnapshot {
            round: self.round,
            has_success: self.has_success,
            current: self.current,
            entry_round: self.entry_round,
            rows: self.rows.iter().copied().collect(),
            spikes: self.spikes.clone(),
        }
    }

    /// 从快照续接（窗口按当前 cap 收敛；spike 时间线全量恢复）。
    pub fn restore(&mut self, snapshot: &RliDomainSnapshot) {
        self.round = snapshot.round;
        self.has_success = snapshot.has_success;
        self.current = snapshot.current;
        self.entry_round = snapshot.entry_round;
        self.rows = snapshot.rows.iter().copied().collect();
        while self.rows.len() > RLI_DOMAIN_RECENT_CAP {
            self.rows.pop_front();
        }
        self.spikes = snapshot.spikes.clone();
    }
}

/// 自判域机器快照（0am 改造四项①；随 [`RliShadowSnapshot`] 入会话侧车。
/// 兼容扩展面：旧侧车无 `domain` 字段 = fresh 域机器，通道面不受影响；
/// 旧侧车的 `compares`／`agrees`／行内 `lif_domain`／`consistent` 字段在
/// 补充项②后不再属于本结构，反序列化时按 serde 默认忽略）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RliDomainSnapshot {
    pub round: u64,
    pub has_success: bool,
    pub current: Domain,
    pub entry_round: u64,
    #[serde(default)]
    pub rows: Vec<RliDomainRow>,
    #[serde(default)]
    pub spikes: Vec<RliDomainSpike>,
}

/// 单通道快照（3 位小数定点化存储；serde 兼容面随会话侧车）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RliChannelSnapshot {
    pub kind: ChannelKind,
    pub omega: f64,
    pub zeta: f64,
    pub u: f64,
    pub v: f64,
    pub theta: f64,
    pub rhythm: f64,
    pub envelope: f64,
    #[serde(default)]
    pub last_step_t: Option<f64>,
    #[serde(default)]
    pub last_inject_t: Option<f64>,
    pub hit_count: u64,
    #[serde(default)]
    pub last_hit_t: Option<f64>,
}

/// 影子族快照（随 [`super::temporal::TemporalSessionSnapshot::rli_shadow`]
/// 入会话侧车；旧侧车无此字段 = 零迁移）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RliShadowSnapshot {
    pub schema: String,
    pub t_hat: f64,
    pub steps: u64,
    pub channels: Vec<RliChannelSnapshot>,
    /// 0am 改造四项①（2026-09-20）：自判域机器状态（`None` / 缺字段 =
    /// legacy 快照——恢复时保持 fresh 域机器，通道与时间轴照常续接）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<RliDomainSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 确定性 xorshift64（合成事件流；无新依赖，镜像 `lif_replay` 的 PRNG）。
    struct Xorshift64(u64);

    impl Xorshift64 {
        fn new(seed: u64) -> Self {
            Self(seed.max(1))
        }

        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        fn next_f64(&mut self) -> f64 {
            (self.next() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn err_omega() -> f64 {
        TAU / RLI_ERR_PERIOD_SECS
    }

    fn half_period(omega: f64) -> f64 {
        PI / (omega * (1.0 - RLI_ZETA * RLI_ZETA).sqrt())
    }

    /// RK4 参考积分（`u' = v；v' = −2ζω·v − ω²·u`）——闭式解的对拍标尺。
    fn rk4_step(u: f64, v: f64, h: f64, two_zeta_w: f64, w2: f64) -> (f64, f64) {
        let f = |u: f64, v: f64| (v, -two_zeta_w * v - w2 * u);
        let (k1u, k1v) = f(u, v);
        let (k2u, k2v) = f(u + 0.5 * h * k1u, v + 0.5 * h * k1v);
        let (k3u, k3v) = f(u + 0.5 * h * k2u, v + 0.5 * h * k2v);
        let (k4u, k4v) = f(u + h * k3u, v + h * k3v);
        (
            u + h / 6.0 * (k1u + 2.0 * k2u + 2.0 * k3u + k4u),
            v + h / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v),
        )
    }

    fn synthetic_events(seed: u64, n: usize) -> Vec<(f64, ToolEvent)> {
        let mut rng = Xorshift64::new(seed);
        let mut t = 0.0;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            t += 1.0 + 90.0 * rng.next_f64();
            let ev = if i % 7 == 0 {
                ToolEvent::error(Some(1_500))
            } else if i % 11 == 0 {
                ToolEvent::deny(Some(20))
            } else if i % 13 == 0 {
                ToolEvent::success(Some(120_000))
            } else {
                ToolEvent::success(Some(500))
            };
            out.push((t, ev));
        }
        out
    }

    /// 交错流：每个工具事件前 0.25s 一次决策轮（同 LifEngine 的真实喂入形态）。
    fn feed(shadow: &mut RliShadow, events: &[(f64, ToolEvent)]) {
        for (t, ev) in events {
            shadow.on_decision_round(t - 0.25, 8.0);
            shadow.on_tool_event(*t, *ev);
        }
    }

    #[test]
    fn closed_form_matches_fine_ode_reference() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        ch.inject(0.0, 1.0);
        let (mut u, mut v) = (1.0_f64, 0.0_f64);
        let h = 0.005_f64;
        let omega = err_omega();
        let mut t = 0.0_f64;
        while t < 60.0 - 1e-9 {
            let (nu, nv) = rk4_step(u, v, h, 2.0 * RLI_ZETA * omega, omega * omega);
            u = nu;
            v = nv;
            t += h;
        }
        ch.advance(60.0);
        assert!((ch.u() - u).abs() < 1e-6, "u {} vs {}", ch.u(), u);
        assert!((ch.v() - v).abs() < 1e-6, "v {} vs {}", ch.v(), v);
    }

    /// 闭式解无步长离散：分段推进 == 一步推进（半群性精确成立）。
    #[test]
    fn advance_is_composable_without_discretization_error() {
        let mut a = RliChannel::new(ChannelKind::Err, err_omega());
        let mut b = RliChannel::new(ChannelKind::Err, err_omega());
        a.inject(0.0, 3.0);
        b.inject(0.0, 3.0);
        a.advance(50.0);
        a.advance(120.0);
        b.advance(120.0);
        assert!((a.u() - b.u()).abs() < 1e-12, "u {} vs {}", a.u(), b.u());
        assert!((a.v() - b.v()).abs() < 1e-12, "v {} vs {}", a.v(), b.v());
    }

    #[test]
    fn injection_updates_u_only_and_refreshes_envelope() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        ch.advance(0.0);
        ch.advance(30.0);
        let v_before = ch.v();
        ch.inject(30.0, 2.0);
        assert!((ch.v() - v_before).abs() < 1e-12, "v must not be kicked");
        assert!((ch.u() - 2.0).abs() < 1e-12);
        assert!(
            ch.envelope() > 1.0,
            "envelope re-energized: {}",
            ch.envelope()
        );
    }

    /// 分位数自校准平衡性质：恒定输入下过阈率 → 1−q = 5%（与 η 无关）。
    #[test]
    fn theta_balance_rate_is_one_minus_q() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        let steps = 4_000u32;
        let mut hits = 0u32;
        for i in 0..steps {
            if ch.observe_value(i as f64, 1.0) {
                hits += 1;
            }
        }
        let rate = f64::from(hits) / f64::from(steps);
        assert!(
            (0.03..=0.08).contains(&rate),
            "exceedance rate {rate} (hits {hits}) off the 1−q target"
        );
    }

    /// 水平迁移：θ 跟随输入抬升，第二段回到 ~5% 过阈率（regime 自归一）。
    #[test]
    fn theta_adapts_across_level_shift() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        for i in 0..2_000u32 {
            ch.observe_value(i as f64, 1.0);
        }
        let mut hits = 0u32;
        for i in 2_000..4_000u32 {
            if ch.observe_value(i as f64, 3.0) {
                hits += 1;
            }
        }
        let rate = f64::from(hits) / 2_000.0;
        assert!(
            (0.03..=0.12).contains(&rate),
            "post-shift rate {rate} (hits {hits}) must re-anchor near 5-10%"
        );
    }

    /// a3 包络：阻尼下自由演化单调衰减；ζ=0（C3 对照）不衰减。
    #[test]
    fn envelope_decays_and_zeta_zero_preserves_amplitude() {
        let mut damped = RliChannel::new(ChannelKind::Err, err_omega());
        damped.inject(0.0, 1.0);
        let e0 = damped.envelope();
        damped.advance(90.0);
        assert!(
            damped.envelope() < 0.5 * e0,
            "E {} vs {e0}",
            damped.envelope()
        );

        let mut undamped = RliChannel::new(ChannelKind::Err, err_omega());
        undamped.set_zeta(0.0);
        undamped.inject(0.0, 1.0);
        let e_undamped = undamped.envelope();
        undamped.advance(90.0);
        assert!(
            (undamped.envelope() - e_undamped).abs() < 1e-9,
            "ζ=0 envelope must not decay: {} vs {e_undamped}",
            undamped.envelope()
        );
    }

    /// C5（a4 节律特异性）：周期间隔放大节律计数，随机同均值序列不放大。
    #[test]
    fn rhythm_is_amplified_by_periodic_spacing_only() {
        let half = half_period(err_omega());
        let mut periodic = RliChannel::new(ChannelKind::Err, err_omega());
        let mut t = 0.0;
        for _ in 0..200 {
            t += half;
            periodic.inject(t, 1.0);
        }
        let mut rng = Xorshift64::new(0x9E37_79B9_7F4A_7C15);
        let mut random = RliChannel::new(ChannelKind::Err, err_omega());
        let mut t = 0.0;
        for _ in 0..200 {
            let u = rng.next_f64().clamp(1e-9, 1.0 - 1e-9);
            t += -half * u.ln();
            random.inject(t, 1.0);
        }
        assert!(
            periodic.rhythm() > random.rhythm() + 0.8,
            "r_periodic {} vs r_random {}",
            periodic.rhythm(),
            random.rhythm()
        );
    }

    /// 容差并集核读（2026-09-17，供设计 §10.4 复核）：默认容差
    /// `π/(4ω_d)` 下 k=1,2,… 匹配窗并集覆盖 `Δt ≥ 半周期/2`——0.6×半周期
    /// 匹配、0.4×半周期 不匹配；收紧到 0.125×半周期 后 0.6× 不再匹配、
    /// 1.1× 匹配（整数倍语义恢复可分辨）。
    #[test]
    fn rhythm_tolerance_union_note() {
        let half = half_period(err_omega());
        let mut spaced = RliChannel::new(ChannelKind::Err, err_omega());
        spaced.inject(0.0, 1.0);
        spaced.inject(0.6 * half, 1.0);
        assert_eq!(spaced.rhythm(), 1.0, "default tolerance matches 0.6×half");

        let mut burst = RliChannel::new(ChannelKind::Err, err_omega());
        burst.inject(0.0, 1.0);
        burst.inject(0.4 * half, 1.0);
        assert_eq!(burst.rhythm(), 0.0, "sub-half/2 bursts never match");

        let mut tight = RliChannel::new(ChannelKind::Err, err_omega());
        tight.set_rhythm_tolerance(0.125);
        tight.inject(0.0, 1.0);
        tight.inject(0.6 * half, 1.0);
        assert_eq!(tight.rhythm(), 0.0, "tightened tolerance rejects 0.6×half");
        let mut tight_multiple = RliChannel::new(ChannelKind::Err, err_omega());
        tight_multiple.set_rhythm_tolerance(0.125);
        tight_multiple.inject(0.0, 1.0);
        tight_multiple.inject(1.1 * half, 1.0);
        assert_eq!(
            tight_multiple.rhythm(),
            1.0,
            "tightened tolerance keeps 1.1×half"
        );
    }

    /// 轮语义：prog 的 ω = 2π/(8·T̂) 随决策轮刷新（T̂ 变大 → ω 变小）。
    #[test]
    fn prog_period_follows_t_hat_on_decision_rounds() {
        let mut shadow = RliShadow::new();
        shadow.on_decision_round(0.0, 8.0);
        let omega_at_8 = shadow.channel(ChannelKind::Prog).omega();
        shadow.on_decision_round(10.0, 20.0);
        let omega_at_20 = shadow.channel(ChannelKind::Prog).omega();
        assert!((omega_at_20 - TAU / (RLI_PROG_PERIOD_ROUNDS * 20.0)).abs() < 1e-12);
        assert!(omega_at_20 < omega_at_8);
    }

    #[test]
    fn snapshot_restore_rejects_foreign_schema_and_short_families() {
        let shadow = RliShadow::new();
        let mut snap = shadow.snapshot();
        snap.schema = "rli-shadow-v0".to_string();
        let mut target = RliShadow::new();
        assert!(!target.restore(&snap));
        let mut short = shadow.snapshot();
        short.channels.pop();
        assert!(!target.restore(&short));
    }

    /// 快照续接：3 位小数定点化后继续推进，锚点差异保持在定点量化界内。
    #[test]
    fn snapshot_roundtrip_continuation_stays_close() {
        let events = synthetic_events(0x00AB_CDEF, 60);
        let mut a = RliShadow::new();
        feed(&mut a, &events);
        let snap = a.snapshot();

        let mut b = RliShadow::new();
        assert!(b.restore(&snap));

        let more = synthetic_events(0x0012_3456, 60);
        feed(&mut a, &more);
        feed(&mut b, &more);
        for &kind in &RLI_CHANNELS {
            let (ca, cb) = (a.channel(kind), b.channel(kind));
            assert!(
                (ca.u() - cb.u()).abs() < 5e-3,
                "{kind:?} u {} vs {}",
                ca.u(),
                cb.u()
            );
            assert!(
                (ca.v() - cb.v()).abs() < 5e-3,
                "{kind:?} v {} vs {}",
                ca.v(),
                cb.v()
            );
            // θ/节律面是离散判定驱动（近阈值平局对 1e-3 时间量化敏感）：
            // 续接契约按相对界断言，翻转不改变锚点方向语义。
            let theta_rel = (ca.theta() - cb.theta()).abs() / ca.theta().max(1e-9);
            assert!(theta_rel < 0.15, "{kind:?} theta rel drift {theta_rel}");
            assert!(
                (ca.rhythm() - cb.rhythm()).abs() <= 1.0,
                "{kind:?} rhythm {} vs {}",
                ca.rhythm(),
                cb.rhythm()
            );
        }
    }

    /// 重放＝同一代码路径逐字段 bit-exact（确定性契约 §8 的同平台半）。
    #[test]
    fn replay_is_bit_exact() {
        let events = synthetic_events(0x0005_EED0, 200);
        let mut a = RliShadow::new();
        let mut b = RliShadow::new();
        feed(&mut a, &events);
        feed(&mut b, &events);
        for &kind in &RLI_CHANNELS {
            let (ca, cb) = (a.channel(kind), b.channel(kind));
            assert_eq!(ca.u().to_bits(), cb.u().to_bits(), "{kind:?} u");
            assert_eq!(ca.v().to_bits(), cb.v().to_bits(), "{kind:?} v");
            assert_eq!(ca.theta().to_bits(), cb.theta().to_bits(), "{kind:?} theta");
            assert_eq!(
                ca.rhythm().to_bits(),
                cb.rhythm().to_bits(),
                "{kind:?} rhythm"
            );
            assert_eq!(ca.hit_count(), cb.hit_count(), "{kind:?} hits");
        }
    }

    /// 存储定点化：3 位小数（与 DynCtx 舍入契约同格）。
    #[test]
    fn snapshot_quantizes_to_three_decimals() {
        assert_eq!(quantize_state(1.234_567), 1.235);
        assert_eq!(quantize_state(-0.000_4), -0.0);
        assert_eq!(quantize_state(2.0), 2.0);
        let shadow = RliShadow::new();
        let snap = shadow.snapshot();
        for ch in &snap.channels {
            let scaled = ch.u * 1000.0;
            assert!((scaled - scaled.round()).abs() < 1e-9, "u not quantized");
        }
    }

    /// 0am 改造四项②（2026-09-20）：a2 预测锚点 = 闭式自由演化对
    /// Δt = 10·T̂ 的精确前推——与 `advance(10·T̂)` 逐字段一致（半群性），
    /// 且不再等于同点一阶短视近似；T̂ 非法时保守落回当前水平。
    #[test]
    fn prediction_is_closed_form_at_ten_t_hat() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        ch.inject(0.0, 1.0);
        let t_hat = 8.0;
        let predicted = ch.prediction(t_hat);

        let mut probe = RliChannel::new(ChannelKind::Err, err_omega());
        probe.inject(0.0, 1.0);
        probe.advance(RLI_PREDICTION_STEPS * t_hat);
        assert!(
            (predicted - probe.u()).abs() < 1e-12,
            "prediction {predicted} vs advance-at-horizon {}",
            probe.u()
        );

        // 同点一阶短视近似（退役形态 u + v·10T̂）与闭式解不同——预测步长
        // 确已换轨（10·T̂ 量级上二阶项不可忽略）。
        let first_order = ch.u() + ch.v() * RLI_PREDICTION_STEPS * t_hat;
        assert!(
            (predicted - first_order).abs() > 1e-9,
            "closed-form must replace the first-order sight ({predicted} vs {first_order})"
        );

        for bad in [f64::NAN, 0.0, -1.0, f64::INFINITY] {
            assert_eq!(ch.prediction(bad), ch.u(), "bad T̂ {bad}");
        }
    }

    /// 0am 改造补充项②（2026-09-20）：自判域谓词 + v／E 原生覆盖项
    /// （stuck 语义不补通道——设计 §10.3；压力轴 = err 水平阈 ∨
    /// 「正在恶化 ∧ 未见收敛」）。
    #[test]
    fn domain_predicate_uses_native_v_and_envelope_for_stuck() {
        let mut m = RliDomainMachine::new();
        // 首次成功前恒 Start（无论读数）。
        assert_eq!(
            m.record_round(1.0, 3.0, 0.1, 5.0, 9.0).domain,
            Domain::Start
        );
        m.observe_tool_outcome(ToolOutcome::Success);
        // 四象：水平轴 × 进度轴。
        assert_eq!(m.label(1.0, 0.9, 0.0, 0.0), Domain::Normal);
        assert_eq!(m.label(2.5, 0.9, 0.0, 0.0), Domain::Pressure);
        assert_eq!(m.label(1.0, 0.2, 0.0, 0.0), Domain::LowProgress);
        assert_eq!(m.label(2.5, 0.2, 0.0, 0.0), Domain::Stuck);
        // v／E 原生覆盖：水平阈以下，但恶化中且未见收敛 ⇒ 压力轴成立。
        assert_eq!(m.label(0.2, 0.9, 0.01, 1.2), Domain::Pressure);
        assert_eq!(m.label(0.2, 0.2, 0.01, 1.2), Domain::Stuck);
        // 任一半不成立 ⇒ 回落水平单轴语义（v ≤ 0 或包络已环降）。
        assert_eq!(m.label(0.2, 0.9, -0.01, 1.2), Domain::Normal);
        assert_eq!(m.label(0.2, 0.9, 0.01, 0.5), Domain::Normal);
        assert_eq!(m.label(0.2, 0.2, -0.01, 1.2), Domain::LowProgress);
    }

    /// 锚点序列表双侧不漂移（名集合 ↔ (通道 × 锚点) 表，同序逐项）。
    #[test]
    fn anchor_feature_table_matches_name_list() {
        assert_eq!(
            RLI_ANCHOR_FEATURE_TABLE.len(),
            RLI_ANCHOR_FEATURE_NAMES.len()
        );
        for ((name, _, _), listed) in RLI_ANCHOR_FEATURE_TABLE
            .iter()
            .zip(RLI_ANCHOR_FEATURE_NAMES.iter())
        {
            assert_eq!(name, listed, "anchor feature table drifted");
        }
    }

    /// 0am 改造补充项④（2026-09-20）：锚点序列 = **事件级采样**（决策轮 ＋
    /// 每个工具事件）、cap 20、live-only（快照恢复后序列为空——序列是观测面
    /// 而非持久状态）。
    #[test]
    fn anchor_series_samples_per_decision_round_and_caps() {
        let mut shadow = RliShadow::new();
        for i in 0..25 {
            shadow.on_decision_round(i as f64, 8.0);
        }
        assert_eq!(
            shadow.feature("u_err", 20).len(),
            20,
            "cap 20, oldest dropped"
        );
        assert_eq!(shadow.feature("u_err", 100).len(), 20);
        assert_eq!(shadow.feature("u_err", 3).len(), 3);
        assert!(
            shadow.feature("u_err", 20).iter().all(|v| v.is_finite()),
            "series values finite"
        );
        assert_eq!(shadow.feature("nope", 5), Vec::<f64>::new());
        assert_eq!(RliShadow::known_feature_names().len(), 11);
        // 序列不随快照持久化（live-only）。
        let snap = shadow.snapshot();
        let mut restored = RliShadow::new();
        assert!(restored.restore(&snap));
        assert!(restored.feature("u_err", 20).is_empty());
    }

    /// 0am 采样粒度（2026-09-20 用户裁决「采样尽可能细」）：**事件级**——
    /// 每个工具事件也入列（决策轮 ＋ 事件各一次）；且注入当刻就能取到快模态
    /// 分量（轮粒度取不到它——0am 模态分离批读数：轮粒度下 `|c_slow−u|`
    /// 均值 0.0043，第二轴被采样粒度抹平）。同时钉住撤名后 `env_prog`
    /// 不再属于序列面。
    #[test]
    fn anchor_series_is_event_level_and_captures_fast_mode_at_injection() {
        let mut shadow = RliShadow::new();
        shadow.on_decision_round(0.0, 8.0);
        assert_eq!(shadow.feature("slow_prog", 5).len(), 1, "决策轮采样");
        shadow.on_tool_event(1.0, ToolEvent::success(None));
        shadow.on_tool_event(2.0, ToolEvent::success(None));
        assert_eq!(shadow.feature("slow_prog", 10).len(), 3, "每事件各采一次");
        // 注入当刻：快模态分量可见（|c_slow − u| = |c_fast| > 0）。
        let ch = shadow.channel(ChannelKind::Prog);
        assert!(ch.u() > 0.9, "inject_set 置 1：u={}", ch.u());
        assert!(
            (ch.mode_slow() - ch.u()).abs() > 0.01,
            "注入当刻快分量非零: slow={} u={}",
            ch.mode_slow(),
            ch.u()
        );
        // 撤名（主会话裁决）：`env_prog` 不在序列面 ⇒ 查询返回空序列。
        assert_eq!(shadow.feature("env_prog", 5), Vec::<f64>::new());
        assert_eq!(shadow.feature("env_err", 5).len(), 3, "env_err 保留");
    }

    /// 0am 改造补充项①（2026-09-20）：配极改——prog 走实极点（ζ = 2.0，
    /// 两个实模态），闭式解与 RK4 参考积分对拍、包络单调环降；
    /// 速率／压力类通道保持复极点 ζ = 0.5。
    #[test]
    fn overdamped_branch_matches_ode_reference_and_pole_table() {
        assert_eq!(rli_zeta_for(ChannelKind::Prog), RLI_PROG_ZETA);
        assert!(
            rli_zeta_for(ChannelKind::Prog) > 1.0,
            "prog walks real poles"
        );
        for kind in [
            ChannelKind::Err,
            ChannelKind::Stall,
            ChannelKind::Slow,
            ChannelKind::Deny,
        ] {
            assert_eq!(rli_zeta_for(kind), RLI_ZETA, "rate/pressure keeps ζ=0.5");
        }

        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let mut ch = RliChannel::new(ChannelKind::Prog, omega);
        assert!(ch.is_overdamped());
        ch.inject_set(0.0, 1.0);
        let (mut u, mut v) = (1.0_f64, 0.0_f64);
        let h = 0.005_f64;
        let mut t = 0.0_f64;
        while t < 40.0 - 1e-9 {
            let (nu, nv) = rk4_step(u, v, h, 2.0 * RLI_PROG_ZETA * omega, omega * omega);
            u = nu;
            v = nv;
            t += h;
        }
        ch.advance(40.0);
        assert!((ch.u() - u).abs() < 1e-6, "u {} vs {}", ch.u(), u);
        assert!((ch.v() - v).abs() < 1e-6, "v {} vs {}", ch.v(), v);

        // 包络单调衰减（环降语义在实极点分支同构成立）。
        let mut env = RliChannel::new(ChannelKind::Prog, omega);
        env.inject_set(0.0, 1.0);
        let mut prev = env.envelope();
        let mut t = 0.0;
        while t < 200.0 {
            t += 5.0;
            env.advance(t);
            assert!(
                env.envelope() <= prev + 1e-12,
                "envelope must not grow under free evolution: {} → {}",
                prev,
                env.envelope()
            );
            prev = env.envelope();
        }
        assert!(prev < 0.05, "slow mode rings down: {prev}");
    }

    /// C3 分离对照的极点控制面（0am 改造补充项③）：`set_poles` 在保持一个
    /// 不变量（ω_d 或 σ = ζω）的前提下改动另一个；钳制区间覆盖复／实两分支。
    #[test]
    fn set_poles_drives_both_branches() {
        let mut ch = RliChannel::new(ChannelKind::Err, err_omega());
        let wd_target = ch.omega_d();
        // 固定 ω_d 扫 ζ：ω = ω_d/√(1−ζ²)。
        let zeta = 0.75_f64;
        ch.set_poles(wd_target / (1.0 - zeta * zeta).sqrt(), zeta);
        assert!((ch.omega_d() - wd_target).abs() < 1e-12);
        // ζ > 1 走实极点分支，且钳制上界为 4.0。
        ch.set_poles(err_omega(), 9.9);
        assert!(ch.is_overdamped());
        assert_eq!(ch.zeta(), 4.0);
        ch.set_zeta(-1.0);
        assert_eq!(ch.zeta(), 0.0);
    }

    /// 0am 模态分离（2026-09-20 用户裁决「拆模态系数＝做」；见
    /// `docs/audits/0AM_RLI_MODE_SPLIT_2026-09-20.md`）：
    /// ① 两级和恒等于水平；② 两级各自解耦衰减（比例恒为各自己的
    /// `exp(−λΔt)`）；③ 实极点分支上包络恒等于水平（a3 是紧上界，
    /// 本批观察项的钉子）；④ 复极点分支上两级无定义（NAN，如实登记）。
    #[test]
    fn mode_split_decomposes_level_and_decouples_decay() {
        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let zeta = RLI_PROG_ZETA;
        let root = (zeta * zeta - 1.0).sqrt();
        let lambda_slow = omega * (zeta - root);
        let lambda_fast = omega * (zeta + root);

        let mut ch = RliChannel::new(ChannelKind::Prog, omega);
        assert!(ch.is_overdamped());
        ch.inject_set(0.0, 1.0);
        let (s0, f0) = (ch.mode_slow(), ch.mode_fast());
        assert!(s0.is_finite() && f0.is_finite());
        assert!((s0 + f0 - ch.u()).abs() < 1e-12, "c_slow + c_fast = u");
        // ζ=2 注入当刻 b = ζ/√(ζ²−1) ≈ 1.155 > u ⇒ 快分量为负（正常 LIF
        // 动力学：水平最初被瞬态扣掉一部分）。
        assert!(s0 > ch.u() && f0 < 0.0, "s0={s0} f0={f0} u={}", ch.u());

        // 自由演化：两级各自按自己的时间常数衰减（解耦），水平＝两级和。
        let dt = 30.0_f64;
        ch.advance(dt);
        let (s1, f1) = (ch.mode_slow(), ch.mode_fast());
        assert!((s1 + f1 - ch.u()).abs() < 1e-12, "sum stays u");
        assert!(
            (s1 / s0 - (-lambda_slow * dt).exp()).abs() < 1e-9,
            "slow mode decays at λ_slow"
        );
        assert!(
            (f1 / f0 - (-lambda_fast * dt).exp()).abs() < 1e-9,
            "fast mode decays at λ_fast"
        );
        // 双时间尺度比（ζ=2）：λ_fast/λ_slow = (2+√3)/(2−√3) ≈ 13.93。
        assert!(
            (lambda_fast / lambda_slow - 13.93).abs() < 0.01,
            "two time scales"
        );
        // a3 紧上界：实极点分支上 E ≡ u（本批观察项的钉子）。
        assert!(
            (ch.envelope() - ch.u()).abs() < 1e-12,
            "E ≡ u on real poles"
        );

        // 复极点分支：慢/快无定义（不虚构语义）。
        let complex = RliChannel::new(ChannelKind::Err, err_omega());
        assert!(!complex.is_overdamped());
        assert!(complex.mode_slow().is_nan());
        assert!(complex.mode_fast().is_nan());
    }

    /// 0am 改造补充项②：逐轮判决记录（v／E 原生覆盖参与判决）+ 域切换
    /// spike；本机器不再有「对照」概念（RLI 不与 LIF 对照）。
    #[test]
    fn domain_machine_records_predicate_and_spikes() {
        let mut m = RliDomainMachine::new();
        m.observe_tool_outcome(ToolOutcome::Success);
        let r1 = m.record_round(1.0, 1.0, 0.9, 0.0, 0.0);
        assert_eq!(r1.domain, Domain::Normal);
        // v > 0 ∧ E ≥ 1.0 ⇒ 压力轴由原生覆盖项支撑（水平阈以下同样成立）。
        let r2 = m.record_round(2.0, 1.0, 0.1, 0.01, 1.2);
        assert_eq!(r2.domain, Domain::Stuck);
        assert_eq!(r2.v_err, Some(0.01));
        assert_eq!(r2.env_err, Some(1.2));
        // 域切换 spike：Start→Normal→Stuck（两次）。
        assert_eq!(m.spikes().len(), 2);
        assert_eq!(m.spikes()[1].domain, Domain::Stuck);
        // 包络环降后（E < 1.0）压力轴回落。
        let r3 = m.record_round(3.0, 1.0, 0.9, 0.01, 0.5);
        assert_eq!(r3.domain, Domain::Normal);
    }

    /// 域级定位面（2026-09-20 用户令「RLI 得配上域级判断部分来方便模型进行
    /// 进度定位和回看」）：切换点带**入域轮次**（FR7 口径用 `Option`，旧侧车
    /// 未记录 = `None` 而不是真值 0），段由相邻点推导 `from → to`／驻留轮数／
    /// 恢复标记（temporal §3.2 Recovery 同口径；末段用当前轮收口）。
    #[test]
    fn domain_spikes_carry_rounds_and_segments_derive_localization() {
        let mut m = RliDomainMachine::new();
        m.observe_tool_outcome(ToolOutcome::Success);
        m.record_round(1.0, 1.0, 0.9, 0.0, 0.0); // r1 Normal
        m.record_round(2.0, 1.0, 0.1, 0.01, 1.2); // r2 Stuck
        m.record_round(3.0, 1.0, 0.1, 0.01, 1.2); // r3 仍 Stuck（无切换点）
        m.record_round(4.0, 1.0, 0.9, 0.0, 0.0); // r4 Normal（恢复）
        let spikes = m.spikes();
        assert_eq!(spikes.len(), 3, "Normal → Stuck → Normal");
        assert_eq!(spikes[0].round, Some(1));
        assert_eq!(spikes[1].round, Some(2));
        assert_eq!(spikes[2].round, Some(4));
        let segs = m.segments();
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].from, None);
        assert_eq!(segs[0].to, Domain::Normal);
        assert_eq!(segs[0].at_round, Some(1));
        assert_eq!(segs[0].dwell_rounds, Some(1), "r1 → r2");
        assert!(!segs[0].recovery);
        assert_eq!(segs[1].from, Some(Domain::Normal));
        assert_eq!(segs[1].to, Domain::Stuck);
        assert_eq!(segs[1].dwell_rounds, Some(2), "r2 → r4");
        assert!(!segs[1].recovery);
        assert_eq!(segs[2].from, Some(Domain::Stuck));
        assert_eq!(segs[2].to, Domain::Normal);
        assert_eq!(segs[2].dwell_rounds, Some(0), "末段用当前轮 r4 收口");
        assert!(segs[2].recovery, "Stuck → Normal = recovery");
    }

    /// 0am 改造四项①：自判域机器随影子快照续接（跨 prompt 面）；legacy
    /// 快照（无 `domain` 字段）＝通道照常续接、域机器 fresh。
    #[test]
    fn domain_machine_rides_shadow_snapshot() {
        let mut shadow = RliShadow::new();
        shadow.on_tool_event(1.0, ToolEvent::success(Some(100)));
        shadow.on_decision_round(2.0, 8.0);
        shadow.on_tool_event(3.0, ToolEvent::error(Some(100)));
        shadow.on_decision_round(10.0, 8.0);
        let before_domain = shadow.domain().current_domain();
        let before_rows = shadow.domain().recent(20).len();
        let snap = shadow.snapshot();
        assert!(snap.domain.is_some(), "domain rides the shadow snapshot");

        let mut restored = RliShadow::new();
        assert!(restored.restore(&snap));
        assert_eq!(restored.domain().current_domain(), before_domain);
        assert_eq!(restored.domain().round(), shadow.domain().round());
        assert_eq!(restored.domain().recent(20).len(), before_rows);

        let mut legacy = snap.clone();
        legacy.domain = None;
        let mut legacy_target = RliShadow::new();
        assert!(legacy_target.restore(&legacy));
        assert_eq!(legacy_target.domain().round(), 0, "fresh machine on legacy");
        assert_eq!(legacy_target.steps(), snap.steps);
        // 通道面照常续接（3 位小数存储契约 ⇒ 相对界断言）。
        let live_u = shadow.channel(ChannelKind::Err).u();
        let restored_u = legacy_target.channel(ChannelKind::Err).u();
        assert!(
            (live_u - restored_u).abs() < 1e-3,
            "legacy restore keeps the channel face ({restored_u} vs {live_u})"
        );
    }
}
