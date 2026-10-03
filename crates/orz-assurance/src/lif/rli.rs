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
//! 锚点（设计 §3）：a1 `u`；a2 `v`（＋预期锚点——0am 四项②原为
//! `10·T̂` 闭式前推，**0be 四项②起分通道 horizon** [`rli_horizon_steps`]，
//! 并把短视档补为**独立锚点** a2s＝`pred1(1·T̂)` 闭式前推
//! [`RliChannel::prediction_short`]——模块头与字段注释的旧「一阶短视
//! `u + v·T̂`」口径系文档漂移，本批收口）；a3 解析
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
//! **0be（2026-09-21 用户令）四项**（来源＝[`RLI_FORECAST_CONTRAST`] §6／§7：
//! ① **会话内在线到达率估计 λ̂**（成功事件间隔 EMA——估计式、非损失式）
//! ＋ **`prog` 前推修正**（自由衰减＋期望注入项：`prog` 是冲量重置型通道，
//! 闭式自由演化只含衰减、不含未来注入 ⇒ 系统性偏低；见
//! [`RliChannel::prediction_at`]）／② **分通道 horizon**（[`rli_horizon_steps`]：
//! Slow／Stall 取短视、Err／Deny 取长视）＋**短视锚点独立化**
//! （[`RliAnchors::prediction_short`]）／③ **自适应参数轨迹进侧车**
//! （[`RliAdaptTraceRow`]，复算可核；新增持久面）／④ **繁杂度**
//! （[`RliComplexity`]：四条复极点通道误差比相对**本会话前段基线**的放大
//! c，三条分位数自校准阈值 θ85/θ95/θ99，**仅用户面机械提醒**——机制照抄
//! 疲劳度 E9、每档一次；**禁用 `prog`**；不设会话桥接）。
//!
//! [`RLI_FORECAST_CONTRAST`]: ../../../../../docs/audits/RLI_FORECAST_CONTRAST_2026-09-21.md

use std::collections::VecDeque;
use std::f64::consts::{PI, TAU};

use serde::{Deserialize, Serialize};

use super::channels::{ChannelKind, STALL_GAP_THRESHOLD_SECS, ToolEvent, ToolOutcome};
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
/// 预测步长默认档（0am 改造四项②，2026-09-20 用户令「预测时间步先定 10」；
/// **0be 四项②起降格**为默认／legacy 档——a2 预期锚点改按
/// [`rli_horizon_steps`] 的**分通道 horizon** 前推，本常数保留两义：
/// ① Err／Prog 通道的分通道值；② [`RliChannel::prediction`] 的兼容口径
/// （`Δt = 10·T̂`，探针／旧读者同源）。
pub const RLI_PREDICTION_STEPS: f64 = 10.0;

/// 分通道 horizon 表（0be 四项②，2026-09-21 用户令「分通道 horizon ＋短视
/// 锚点独立化」；**0bg S2 标定批 2026-09-22**：依 0bf 探针 skill 表
/// （`rli-forecast-contrast-0bf.json`）逐通道峰档重标——Slow h=1→**5**
/// （skill +0.317 vs h1 +0.222）、Deny h=30→**10**（+0.638 vs h30 +0.571）；
/// Err=10、Stall=2、Prog=10 保持（峰档/弱档；探针读数见 0bf 报告 §3）。
/// **语义常数、禁拟合**；`prog` 维持默认档（修正项 ① 后另测）。
///
/// [`RLI_FORECAST_CONTRAST`]: ../../../../../docs/audits/RLI_FORECAST_CONTRAST_2026-09-21.md
pub fn rli_horizon_steps(kind: ChannelKind) -> f64 {
    match kind {
        ChannelKind::Slow => 5.0,
        ChannelKind::Stall => 2.0,
        ChannelKind::Err => RLI_PREDICTION_STEPS,
        ChannelKind::Deny => 10.0,
        ChannelKind::Prog => RLI_PREDICTION_STEPS,
        // 0am P8 新通道（2026-10-03）：默认档（无探针标定读数；语义常数、
        // 禁拟合——标定须按 0bf 探针先例另批）。
        ChannelKind::Verify | ChannelKind::Ctx | ChannelKind::Infra => RLI_PREDICTION_STEPS,
    }
}

/// 会话内在线到达率估计 λ̂（0be 四项①，2026-09-21）：**成功事件到达间隔
/// 的 EMA**（估计式——非损失下降；口径见 [`RLI_FORECAST_CONTRAST`] §6）。
/// 间隔先钳制到 [`RLI_LAMBDA_GAP_MIN_SECS`, `RLI_LAMBDA_GAP_MAX_SECS`]；
/// 不足 [`RLI_LAMBDA_MIN_GAPS`] 个间隔＝**未激活**（退化路径：`prog` 前推
/// 回落纯自由衰减，行为与 0am 版一致）。作用域＝本会话，随侧车延续。
///
/// [`RLI_FORECAST_CONTRAST`]: ../../../../../docs/audits/RLI_FORECAST_CONTRAST_2026-09-21.md
pub const RLI_LAMBDA_EMA_ALPHA: f64 = 0.2;
pub const RLI_LAMBDA_GAP_MIN_SECS: f64 = 0.5;
pub const RLI_LAMBDA_GAP_MAX_SECS: f64 = 3_600.0;
pub const RLI_LAMBDA_MIN_GAPS: u64 = 2;

/// **0cp D2 卡死看门狗单点时间采样阈**（2026-10-02 用户裁定「无动作样且
/// 模型流式传输停滞后3分01秒后触发一个时间采样」，163 批）：**181.0s**＝
/// TER 首报后台化 180s（ADR-0010 §14.55）＋1s 避让——长任务在 180s 转
/// 后台并首报，通常随即引发后续模型轮（流式活动恢复，看门狗自然不触发）；
/// 181s 仍无动作样且无活动＝真卡死（引擎 hang／传输断流／工具挂死且兜底
/// 失效），采样一次留证。语义常数、禁拟合。触发恰产生**一个**时间采样后
/// 休眠，直到下一个动作采样点重武装（`watchdog_armed`）。判定由运行时
/// 轻量定时检查驱动（1–10s 级 interval；仅判定，不产生周期采样）。
pub const RLI_WATCHDOG_SAMPLE_SECS: f64 = 181.0;

/// 0bf ③（2026-09-22，S1 预注册口径②）：持续越线判定——通道 `u ≥ θ`
/// （越线）连续达到本**采样点数**即触发一次模型面提醒；re-arm＝断线归零
/// 后重新累计。「连续 k 次」替代会话长度门（用户令「模型需要获得数据」
/// ——不加长度门）。**0cp D3（2026-10-02 用户裁决，163 批）**：k 5 → 3
/// ——「上一轮的调研里很明确的是5这个阈值过高了」；k＝**连续动作采样点**
/// 的异常累积值（0cp D1 动作化后语义不变，k=3 为动作化口径变严的补偿性
/// 下调）。
pub const RLI_STREAK_K: u64 = 3;

/// 0bf ③：域迁移确认——域切换后新域稳定（无再切换）连续达到本轮数才
/// 「确认迁移」并提醒一次（等震荡结束；震荡期间只更新候选）。
pub const RLI_MIGRATION_SETTLE_ROUNDS: u64 = 3;

/// 0bf ③：模型面提醒队列上限（**0cp D4 起投递改直投**；队列保留为事件
/// 历史，cap 不变）。
pub const RLI_NOTICE_CAP: usize = 16;

/// **0cp D7**：域事件提醒行「近期动作形态」窗（最近 N 个工具事件 outcome；
/// live-only，不随侧车持久）。
pub const RLI_RECENT_OUTCOMES_CAP: usize = 5;

/// 0bf ③：失配概率窗（各通道最近 ≤N 个 ρ 样本合并——每条通道窗上限即
/// [`RLI_CPLX_PER_CHANNEL_CAP`]；ρ > 1.0 计一次失配。ρ 定义见
/// [`RliComplexity`]：预测误差超过「不变化」持久性基线）。
pub const RLI_MISMATCH_WINDOW: usize = 16;

/// 繁杂度（0be 四项④，2026-09-21 用户二次裁定：机制照抄疲劳度 E9、只需
/// 一个指标）——指标＝**四条复极点通道**（err／stall／slow／deny；**禁用
/// `prog`**）前推误差比 ρ = MAE_pred / MAE_persist 的跨通道中位（窗内逐
/// 样本误差比的中位；持久性误差≈0 的样本跳过），相对**本会话前段基线**
/// ρ_base（前 [`RLI_CPLX_BASELINE_SAMPLES`] 个合成样本中位，冻结）的放大
/// c = ρ_cur / ρ_base；三条阈值 θ85/θ95/θ99 用 RLI 现成**分位数自校准**
/// （θ 同法乘性更新，η = [`RLI_ETA_INIT`]）追踪 c 自身 (1−q) 分位；越线
/// 即**锁存**（含中途尖峰）。**不设跨会话桥接**、不设记忆系统延续。
pub const RLI_CPLX_PER_CHANNEL_CAP: usize = 16;
pub const RLI_CPLX_BASELINE_SAMPLES: usize = 16;
pub const RLI_CPLX_MIN_CHANNEL_SAMPLES: usize = 4;
pub const RLI_CPLX_RHO_MAX: f64 = 4.0;
pub const RLI_CPLX_C_MIN: f64 = 0.25;
pub const RLI_CPLX_C_MAX: f64 = 4.0;
/// 档位键（与水位疲劳的 `50`/`70`/`90` **分开命名**；每档一次、用户面）。
pub const RLI_CPLX_TIERS: [(&str, f64); 3] = [("q85", 0.85), ("q95", 0.95), ("q99", 0.99)];
/// 自适应参数轨迹（0be 四项③）：决策轮粒度、cap 128、随侧车持久
/// （复算可核——轨迹与在线自适应同源同序）。
pub const RLI_ADAPT_TRACE_CAP: usize = 128;
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
/// 去冗余（2026-09-20 主会话裁决，用户授权「两名的去留和 `env_prog` 的
/// 去留请你裁决」）：**`env_prog` 撤名**——它在实极点分支上**恒等于**
/// `u_prog`（a3 是紧的上界，见该报告 §1），保留会让一条副本冒充独立读数轴。
/// `env_err` **保留**（复极点分支上 `E > |u|` 泛成立，独立）。
///
/// 0be 四项②（2026-09-21）：**短视锚点独立化**——新增 `pred1_err`／
/// `pred1_prog`（a2s＝`1·T̂` 闭式前推），与原 `pred_*`（a2＝决策通道的
/// 分通道 horizon）分列；旧「一阶短视 `u + v·T̂`」注释口径系文档漂移，
/// 本批收口。
pub const RLI_ANCHOR_FEATURE_NAMES: [&str; 19] = [
    "u_err",
    "v_err",
    "pred_err",
    "pred1_err",
    "env_err",
    "r_err",
    "u_prog",
    "v_prog",
    "pred_prog",
    "pred1_prog",
    "r_prog",
    "slow_prog",
    "fast_prog",
    // 0am P8（2026-10-03，S2 §8「锚点表随实现批定」）：新三通道只挂 u/v
    // 两锚点（水平/变化率；pred/env/r 按需后续批次增补——feature 面保持
    // 最小可读集）。
    "u_verify",
    "v_verify",
    "u_ctx",
    "v_ctx",
    "u_infra",
    "v_infra",
];
/// 自判域近期行窗口（镜像 temporal 的 `RECENT_RECORDS_CAP`）。
pub const RLI_DOMAIN_RECENT_CAP: usize = 20;
/// 侧车快照 schema 标识（随 [`RliShadowSnapshot`] 序列化）。**0am P8 升
/// v2**（2026-10-03）：通道族 5→8，restore 的通道数完备性校验使旧 v1 侧车
/// 快照整体拒续（fresh 重启＝文档化的降级路径；跨会话影子状态一次性重置，
/// 会话内不受影响）。
pub const RLI_SNAPSHOT_SCHEMA: &str = "rli-shadow-v2";
/// 状态存储定点化（3 位小数；同 DynCtx 舍入契约）。
pub const RLI_STATE_DECIMALS: f64 = 1_000.0;

/// 速率型通道周期语义（秒）——`ω = 2π/周期`（设计 §6 表）。
pub const RLI_ERR_PERIOD_SECS: f64 = 180.0;
pub const RLI_STALL_PERIOD_SECS: f64 = 90.0;
pub const RLI_SLOW_PERIOD_SECS: f64 = 600.0;
pub const RLI_DENY_PERIOD_SECS: f64 = 120.0;
/// 新鲜度型通道（轮语义）——`ω = 2π/(k·T̂)`。
pub const RLI_PROG_PERIOD_ROUNDS: f64 = 8.0;
/// 0am P8（2026-10-03，S2 §2 预注册初值；物理依据见
/// `VERIFY_TAU_ROUNDS` 注）。
pub const RLI_VERIFY_PERIOD_ROUNDS: f64 = 8.0;
pub const RLI_CTX_PERIOD_ROUNDS: f64 = 32.0;
pub const RLI_INFRA_PERIOD_ROUNDS: f64 = 64.0;

/// 影子通道族（v1 五条 ＋ **0am P8 三条**＝八通道；stuck 延后——见模块头）。
/// 〔勘误：S2 档 §2 标题「5→7」系算术笔误，通道表实为 5+3=8；随 P8 批勘误。〕
pub const RLI_CHANNELS: [ChannelKind; 8] = [
    ChannelKind::Err,
    ChannelKind::Stall,
    ChannelKind::Slow,
    ChannelKind::Deny,
    ChannelKind::Prog,
    ChannelKind::Verify,
    ChannelKind::Ctx,
    ChannelKind::Infra,
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
        // 0am P8（S2 §2）：Verify（失败累积）／Ctx／Infra（水平/新鲜度类）
        // 走实极点分支——与 prog 同族（持久／适应双模态）。
        ChannelKind::Prog | ChannelKind::Verify | ChannelKind::Ctx | ChannelKind::Infra => {
            RLI_PROG_ZETA
        }
        ChannelKind::Err | ChannelKind::Stall | ChannelKind::Slow | ChannelKind::Deny => RLI_ZETA,
    }
}

/// 实极点分支的两个指数核积分（0be 四项①，闭式解；`prog` 前推修正用）：
///
/// ```text
/// ∫₀^τ e^(−q a)·cosh(μa) da = ½[(1−e^(−(q−μ)τ))/(q−μ) + (1−e^(−(q+μ)τ))/(q+μ)]
/// ∫₀^τ e^(−q a)·sinh(μa) da = ½[(1−e^(−(q−μ)τ))/(q−μ) − (1−e^(−(q+μ)τ))/(q+μ)]
/// ```
///
/// 返回 `(cosh 积分, sinh 积分)`。调用面保证 `q > μ ≥ 0`
/// （`q = ζω + λ̂`，`ζω − μ = ω²/(ζω+μ) > 0`），分母下限 1e−9 仅为数值兜底。
fn overdamped_decay_integrals(q: f64, mu: f64, tau: f64) -> (f64, f64) {
    let minor = (q - mu).max(1e-9);
    let major = q + mu;
    let i_minor = (1.0 - libm::exp(-minor * tau)) / minor;
    let i_major = (1.0 - libm::exp(-major * tau)) / major;
    (0.5 * (i_minor + i_major), 0.5 * (i_minor - i_major))
}

/// 单锚点选取（锚点序列面用；与 [`RliAnchors`] 的 a1–a4 一一对应；θ/hits
/// 是判定面不属锚点面，不入序列）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RliAnchor {
    /// a1 水平。
    U,
    /// a2 变化率。
    V,
    /// a2 预期锚点（分通道 horizon 闭式前推；`pred_*`）。
    Pred,
    /// a2s 短视锚点（0be 四项②：`1·T̂` 闭式前推，独立成档；`pred1_*`）。
    PredShort,
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
            RliAnchor::Pred => ch.prediction_at_horizon(t_hat_secs),
            RliAnchor::PredShort => ch.prediction_short(t_hat_secs),
            RliAnchor::Env => ch.envelope(),
            RliAnchor::Rhythm => ch.rhythm(),
            RliAnchor::ModeSlow => ch.mode_slow(),
            RliAnchor::ModeFast => ch.mode_fast(),
        }
    }
}

/// 锚点序列表（名 → 通道 × 锚点；与 [`RLI_ANCHOR_FEATURE_NAMES`] 同序，
/// 测试钉住两侧不漂移）。
pub const RLI_ANCHOR_FEATURE_TABLE: [(&str, ChannelKind, RliAnchor); 19] = [
    ("u_err", ChannelKind::Err, RliAnchor::U),
    ("v_err", ChannelKind::Err, RliAnchor::V),
    ("pred_err", ChannelKind::Err, RliAnchor::Pred),
    ("pred1_err", ChannelKind::Err, RliAnchor::PredShort),
    ("env_err", ChannelKind::Err, RliAnchor::Env),
    ("r_err", ChannelKind::Err, RliAnchor::Rhythm),
    ("u_prog", ChannelKind::Prog, RliAnchor::U),
    ("v_prog", ChannelKind::Prog, RliAnchor::V),
    ("pred_prog", ChannelKind::Prog, RliAnchor::Pred),
    ("pred1_prog", ChannelKind::Prog, RliAnchor::PredShort),
    ("r_prog", ChannelKind::Prog, RliAnchor::Rhythm),
    ("slow_prog", ChannelKind::Prog, RliAnchor::ModeSlow),
    ("fast_prog", ChannelKind::Prog, RliAnchor::ModeFast),
    ("u_verify", ChannelKind::Verify, RliAnchor::U),
    ("v_verify", ChannelKind::Verify, RliAnchor::V),
    ("u_ctx", ChannelKind::Ctx, RliAnchor::U),
    ("v_ctx", ChannelKind::Ctx, RliAnchor::V),
    ("u_infra", ChannelKind::Infra, RliAnchor::U),
    ("v_infra", ChannelKind::Infra, RliAnchor::V),
];

/// 单通道锚点读数（S3 回放导出面；只读快照）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliAnchors {
    /// a1 水平。
    pub u: f64,
    /// a2 变化率（1/s）。
    pub v: f64,
    /// a2 预期锚点（0be 四项②：**分通道 horizon** 的闭式自由演化前推，
    /// `Δt = rli_horizon_steps(kind)·T̂`；`prog` 通道另含 ① 的期望注入项）。
    pub prediction: f64,
    /// a2s 短视锚点（0be 四项②独立化：`Δt = 1·T̂` 闭式前推；与 `prediction`
    /// 分列——旧注释的「一阶短视 `u + v·T̂`」口径已退役）。
    pub prediction_short: f64,
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
    /// 0bf ③（2026-09-22）：持续越线观察计数——采样点上 `u ≥ θ` 的连续
    /// 次数（**纯观测**：不改 θ/fires 语义；断线归零）。
    streak: u64,
    /// 当前连续段的起点采样时刻（断线 = `None`；FR-7 口径不落假 0）。
    streak_start_t: Option<f64>,
    /// 0be 四项①：成功到达间隔 EMA（`None` ＝尚无间隔样本——退化路径）。
    /// λ̂ 由此派生（[`Self::lambda_hat`]），随快照持久（会话级自适应参数）。
    lambda_gap_ema: Option<f64>,
    /// 已累积的成功间隔样本计数（≥ [`RLI_LAMBDA_MIN_GAPS`] 才激活 λ̂）。
    lambda_gap_samples: u64,
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
            streak: 0,
            streak_start_t: None,
            lambda_gap_ema: None,
            lambda_gap_samples: 0,
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

    /// 持续越线当前连续次数（0bf ③；纯观测面）。
    pub fn streak(&self) -> u64 {
        self.streak
    }

    /// 当前连续段起点采样时刻（0bf ③；`None` ＝未在连续段上）。
    pub fn streak_start_t(&self) -> Option<f64> {
        self.streak_start_t
    }

    /// a2 预期锚点（兼容口径：`Δt = RLI_PREDICTION_STEPS · T̂` 的闭式自由
    /// 演化前推——探针与旧读者同源）。**0be 四项②起**：通道读数面用
    /// [`Self::prediction_at_horizon`]（分通道 horizon），本方法保留
    /// `prediction_at(RLI_PREDICTION_STEPS, ·)` 语义（`prog` 已含 ① 的期望
    /// 注入项；其余通道＝纯自由演化）。T̂ 非法（非有限 / ≤ 0）时返回当前
    /// 水平 `u`（保守落点，不虚构前推）。
    pub fn prediction(&self, t_hat_secs: f64) -> f64 {
        self.prediction_at(RLI_PREDICTION_STEPS, t_hat_secs)
    }

    /// 分通道 horizon 前推（0be 四项②）：`Δt = rli_horizon_steps(kind)·T̂`。
    pub fn prediction_at_horizon(&self, t_hat_secs: f64) -> f64 {
        self.prediction_at(rli_horizon_steps(self.kind), t_hat_secs)
    }

    /// a2s 短视锚点（0be 四项②独立化）：`Δt = 1·T̂` 的闭式前推。
    pub fn prediction_short(&self, t_hat_secs: f64) -> f64 {
        self.prediction_at(1.0, t_hat_secs)
    }

    /// 显式步长前推（0be 四项②；`steps` 以 T̂ 为单位）。`prog`（实极点、
    /// 冲量重置型）走 [`Self::prog_prediction_with_arrivals`]；其余通道＝
    /// 闭式自由演化（二阶解，退役一阶短视近似 `u + v·T̂`——0am 改造）。
    pub fn prediction_at(&self, steps: f64, t_hat_secs: f64) -> f64 {
        if !t_hat_secs.is_finite() || t_hat_secs <= 0.0 || !steps.is_finite() || steps <= 0.0 {
            return self.u;
        }
        let dt = steps * t_hat_secs;
        if self.kind == ChannelKind::Prog && self.is_overdamped() {
            return self.prog_prediction_with_arrivals(dt);
        }
        self.free_evolution_at(dt).0
    }

    /// `prog` 前推修正（0be 四项①，2026-09-21 用户令）：自由衰减 ＋
    /// **期望注入项**。机制＝`prog` 是**冲量重置型**（成功到达把 `u` 置 1），
    /// 闭式自由演化只含衰减、不含未来注入 ⇒ 对拍中系统性偏低（137-run
    /// h=10 均值 0.091 vs 实际 0.592）。
    ///
    /// 模型（近似，如实登记）：成功到达为速率 λ̂ 的泊松过程，各到达把 `u`
    /// 置 1（`v` 不踢）。设窗口年龄 `s`（自当前时刻起算），则
    ///
    /// ```text
    /// E[u(t+τ)] = e^(−λ̂τ)·g_free(τ) + λ̂·∫₀^τ e^(−λ̂s)·g(s) ds
    /// ```
    ///
    /// 其中 `g_free` ＝当前状态 (u₀,v₀) 的自由演化 u 分量；`g(s)` ＝自
    /// 「注入当刻状态 (1, v₀)」自由演化的 u 分量（**同刻注入近似**：未来
    /// 到达时刻的 v 用当前 v 代替——如实登记为近似面）。积分有闭式
    /// （[`overdamped_decay_integrals`]，q = ζω + λ̂ > μ 恒成立）：
    /// O(1)、无迭代、无噪声、无拟合参数。
    ///
    /// λ̂ 未激活（间隔样本不足——退化路径）时**严格**退回自由衰减。
    fn prog_prediction_with_arrivals(&self, dt: f64) -> f64 {
        let free = self.free_evolution_at(dt).0;
        let Some(lambda) = self.lambda_hat() else {
            return free;
        };
        let zeta_w = self.zeta * self.omega;
        let q = zeta_w + lambda;
        let (i_cosh, i_sinh) = overdamped_decay_integrals(q, self.mu(), dt);
        // b̄ = (v₀ + ζω)/μ：注入当刻 (u=1, v=v₀) 的快模偏置。
        let b_c = (self.v + zeta_w) / self.mu();
        let injection = lambda * (i_cosh + b_c * i_sinh);
        libm::exp(-lambda * dt) * free + injection
    }

    /// 会话内在线到达率估计 λ̂（0be 四项①）：成功事件到达间隔 EMA 的倒数。
    /// `None` ＝未激活（间隔样本 < [`RLI_LAMBDA_MIN_GAPS`]——退化路径）。
    pub fn lambda_hat(&self) -> Option<f64> {
        if self.lambda_gap_samples < RLI_LAMBDA_MIN_GAPS {
            return None;
        }
        self.lambda_gap_ema
            .map(|gap| 1.0 / gap.max(RLI_LAMBDA_GAP_MIN_SECS))
    }

    /// 成功到达间隔记账（0be 四项①；`inject_set` 内调用——间隔在更新
    /// `last_inject_t` 前读取）。间隔先钳制到语义上下界，再 EMA（α 见
    /// [`RLI_LAMBDA_EMA_ALPHA`]）。**估计式**：只有矩统计，无损失、无梯度。
    fn note_success_gap(&mut self, t: f64) {
        let Some(prev) = self.last_inject_t else {
            return;
        };
        let dt = t - prev;
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let gap = dt.clamp(RLI_LAMBDA_GAP_MIN_SECS, RLI_LAMBDA_GAP_MAX_SECS);
        self.lambda_gap_ema = Some(match self.lambda_gap_ema {
            None => gap,
            Some(ema) => ema + RLI_LAMBDA_EMA_ALPHA * (gap - ema),
        });
        self.lambda_gap_samples = self.lambda_gap_samples.saturating_add(1);
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
            prediction_short: self.prediction_short(t_hat_secs),
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

    /// 新鲜度型通道（prog）：成功置 1（同现行 1D `prog` 语义）。0be 四项①
    /// 起同时记账**成功到达间隔**（λ̂ 的输入；间隔在 `last_inject_t` 更新前
    /// 读取——见 [`Self::note_success_gap`]）。
    pub fn inject_set(&mut self, t: f64, value: f64) {
        self.note_success_gap(t);
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

    /// 0bf ③（2026-09-22）：**持续越线观察**（纯观测面，不改 θ/fires
    /// 语义）——采样点上 `u ≥ θ` 则连续计数 +1（段首记起点），否则归零。
    /// 返回「恰好达到」[`RLI_STREAK_K`] 的触发沿（re-arm＝归零后重新累计）。
    pub fn observe_streak(&mut self, t: f64) -> bool {
        if self.u >= self.theta {
            if self.streak == 0 {
                self.streak_start_t = Some(t);
            }
            self.streak = self.streak.saturating_add(1);
            self.streak == RLI_STREAK_K
        } else {
            self.streak = 0;
            self.streak_start_t = None;
            false
        }
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
            // 0bf ③（2026-09-22）：持续越线观察（连续计数＋段起点）随侧车
            // 持久——跨 prompt 连续段不因侧车往返断裂（如实续接）。
            streak: self.streak,
            streak_start_t: self.streak_start_t.map(quantize_state),
            // 0be 四项①：λ̂ 估计器状态（间隔 EMA ＋样本计数）随侧车持久
            // ——会话级自适应参数，跨 prompt 精确续接。
            lambda_gap_ema: self.lambda_gap_ema.map(quantize_state),
            lambda_gap_samples: self.lambda_gap_samples,
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
        // 0bf ③：持续越线观察续接（legacy 快照缺字段 ⇒ 0/None——与
        // 「不在连续段上」同义；FR-7 口径不可得与真值 0 不混同）。
        self.streak = snapshot.streak;
        self.streak_start_t = snapshot.streak_start_t.filter(|t| t.is_finite());
        // 0be 四项①：λ̂ 估计器状态（sanitize：非有限/非正 EMA 落回 None＝
        // 未激活；legacy 快照无字段 ⇒ None/0，与「尚无成功间隔」同义可分）。
        self.lambda_gap_ema = snapshot
            .lambda_gap_ema
            .filter(|v| v.is_finite() && *v > 0.0)
            .map(|v| v.clamp(RLI_LAMBDA_GAP_MIN_SECS, RLI_LAMBDA_GAP_MAX_SECS));
        self.lambda_gap_samples = snapshot.lambda_gap_samples;
    }
}

/// 未决算预测（0be 四项④：繁杂度指标的在线配对件——决策轮发出，
/// 「首个 ≥ 到期时刻的决策轮」消费；与对拍件 §1 同口径）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliPendingPrediction {
    /// 发出时刻（会话相对秒）。
    pub issue_t: f64,
    /// 发出时刻的水平 u（持久性基线）。
    pub issue_u: f64,
    /// 前推值（发出时刻按分通道 horizon 计算）。
    pub pred: f64,
    /// 到期时刻（`issue_t + horizon·T̂`）。
    pub due_t: f64,
}

/// 繁杂度状态（0be 四项④，2026-09-21 用户二次裁定「机制照抄疲劳度、只需
/// 一个指标」）——**会话级**（随侧车延续；不设跨会话桥接）。指标链：
///
/// ```text
/// ρ_i   = |pred_i − real_i| / |issue_u_i − real_i|   （逐样本误差比；分母≈0 跳过）
/// ρ_ch  = 窗内 ρ_i 的中位                                  （逐通道）
/// ρ_cur = 各就绪通道 ρ_ch 的中位                            （跨通道；四条复极点通道，禁用 prog）
/// ρ_base= 前 BASELINE_SAMPLES 个 ρ_cur 样本的中位（冻结）  （本会话前段基线）
/// c     = clamp(ρ_cur / ρ_base, C_MIN, C_MAX)              （放大倍数）
/// θ_t   : c 的 (1−q) 分位追踪（θ 同法：c ≥ θ → θ·e^(+η)，否则 θ·e^(−η(1−q)/q)）
/// ```
///
/// 越线（`c > θ_t` 且 `c > 1`）即**锁存**（`latched`，含中途尖峰）；档位键
/// [`RLI_CPLX_TIERS`]（`q85`/`q95`/`q99`）——投递与「每档一次」由宿主面
/// 侧车键簿记（照拷贝疲劳度 E9）。样本不足 = 未就绪（退化路径：无提醒）。
#[derive(Debug, Clone)]
pub struct RliComplexity {
    /// 四条复极点通道的 ρ 窗（顺序 = [`RLI_CPLX_KINDS`]）。
    channel_windows: [VecDeque<f64>; RLI_CPLX_KINDS.len()],
    /// 各通道未决算预测（配对后出窗；cap = PER_CHANNEL_CAP）。
    pending: [VecDeque<RliPendingPrediction>; RLI_CPLX_KINDS.len()],
    /// 合成样本窗（ρ_cur 流；cap = PER_CHANNEL_CAP）。
    combined: VecDeque<f64>,
    /// 基线（前 `BASELINE_SAMPLES` 个合成样本中位；`None` = 未冻结）。
    baseline: Option<f64>,
    /// 已产出的合成样本数。
    samples: u64,
    /// 三条自校准阈值（θ85/θ95/θ99）。
    theta: [f64; RLI_CPLX_TIERS.len()],
    /// 越线锁存（每条阈值一次；投递键由宿主面侧车簿记）。
    latched: [bool; RLI_CPLX_TIERS.len()],
    /// 当前 c（未就绪 = None）。
    current_c: Option<f64>,
}

/// 繁杂度样本通道（**四条复极点通道**；禁用 `prog`——h≥2 结构性负会报假
/// 疲劳，用户裁定见项目索引 `OBS-RLI-SESSION-FATIGUE`）。
pub const RLI_CPLX_KINDS: [ChannelKind; 4] = [
    ChannelKind::Err,
    ChannelKind::Stall,
    ChannelKind::Slow,
    ChannelKind::Deny,
];

impl Default for RliComplexity {
    fn default() -> Self {
        Self::new()
    }
}

impl RliComplexity {
    fn new() -> Self {
        Self {
            channel_windows: std::array::from_fn(|_| VecDeque::new()),
            pending: std::array::from_fn(|_| VecDeque::new()),
            combined: VecDeque::new(),
            baseline: None,
            samples: 0,
            // θ 初值 1.0 ＝「与基线同」的倍数单位（自校准后浮动）。
            theta: [1.0; RLI_CPLX_TIERS.len()],
            latched: [false; RLI_CPLX_TIERS.len()],
            current_c: None,
        }
    }

    /// 是否就绪（基线已冻结）。
    pub fn ready(&self) -> bool {
        self.baseline.is_some()
    }

    /// 已产出的合成样本数。
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// 当前放大倍数 c（未就绪 = `None`）。
    pub fn current_c(&self) -> Option<f64> {
        self.current_c
    }

    /// 本会话前段基线 ρ_base（未冻结 = `None`）。
    pub fn baseline(&self) -> Option<f64> {
        self.baseline
    }

    /// 第 `idx` 条自校准阈值。
    pub fn theta(&self, idx: usize) -> f64 {
        self.theta[idx]
    }

    /// 第 `idx` 档是否曾越线（锁存）。
    pub fn latched(&self, idx: usize) -> bool {
        self.latched[idx]
    }

    /// 繁杂度读数（宿主／渲染面用的只读快照）。
    pub fn reading(&self) -> RliComplexityReading {
        RliComplexityReading {
            ready: self.ready(),
            samples: self.samples,
            c: self.current_c,
            rho_base: self.baseline,
            theta: self.theta,
            latched: self.latched,
        }
    }

    /// 0bf ③（2026-09-22）：**失配概率读数**——各通道最近 ≤
    /// [`RLI_MISMATCH_WINDOW`] 个 ρ 样本合并后「ρ > 1（预测误差超过
    /// 『不变化』持久性基线＝失配）」的占比；无样本 = `None`（未就绪与
    /// 真值 0 不混同——FR-7）。**只读**：不反馈、不改本结构任何状态
    /// （用户令「失配只记录不反馈」）。
    pub fn mismatch_rate(&self) -> Option<f64> {
        let mut total = 0usize;
        let mut mismatched = 0usize;
        for window in &self.channel_windows {
            for rho in window.iter().rev().take(RLI_MISMATCH_WINDOW) {
                total += 1;
                if *rho > 1.0 {
                    mismatched += 1;
                }
            }
        }
        (total > 0).then(|| mismatched as f64 / total as f64)
    }

    /// 决策轮推进：先消费到期预测（逐通道出窗），再发出新预测；然后更新
    /// 合成样本、基线与三条自校准阈值（含锁存）。返回当前 c（未就绪
    /// = `None`）——副作用面仅本结构（影子侧无注入）。
    fn on_decision_round(
        &mut self,
        t: f64,
        t_hat: f64,
        channels: &[RliChannel; RLI_CHANNELS.len()],
    ) -> Option<f64> {
        let mut produced = false;
        for (slot, &kind) in RLI_CPLX_KINDS.iter().enumerate() {
            let ch = &channels[RliShadow::index_of(kind)];
            // 消费到期预测：首个 ≥ due_t 的决策轮（当前轮）出窗。
            while let Some(front) = self.pending[slot].front() {
                if front.due_t > t {
                    break;
                }
                let pending = self.pending[slot].pop_front().expect("front exists");
                let denom = (pending.issue_u - ch.u()).abs();
                if denom > 1e-9 {
                    let rho = ((pending.pred - ch.u()).abs() / denom).clamp(0.0, RLI_CPLX_RHO_MAX);
                    push_capped(
                        &mut self.channel_windows[slot],
                        rho,
                        RLI_CPLX_PER_CHANNEL_CAP,
                    );
                    produced = true;
                }
            }
            // 发出新预测（分通道 horizon；`prog` 不参与——本表只含复极点通道）。
            let horizon = rli_horizon_steps(kind) * t_hat;
            push_capped(
                &mut self.pending[slot],
                RliPendingPrediction {
                    issue_t: t,
                    issue_u: ch.u(),
                    pred: ch.prediction_at_horizon(t_hat),
                    due_t: t + horizon,
                },
                RLI_CPLX_PER_CHANNEL_CAP,
            );
        }
        if produced {
            // 合成样本：各就绪通道窗中位 → 跨通道中位（≥2 通道就绪才产出）。
            let mut per_channel: Vec<f64> = Vec::with_capacity(RLI_CPLX_KINDS.len());
            for window in &self.channel_windows {
                if window.len() >= RLI_CPLX_MIN_CHANNEL_SAMPLES {
                    let mut v: Vec<f64> = window.iter().copied().collect();
                    per_channel.push(median_of(&mut v));
                }
            }
            if per_channel.len() >= 2 {
                let rho_cur = median_of(&mut per_channel);
                push_capped(&mut self.combined, rho_cur, RLI_CPLX_PER_CHANNEL_CAP);
                self.samples = self.samples.saturating_add(1);
                if self.baseline.is_none() && self.samples >= RLI_CPLX_BASELINE_SAMPLES as u64 {
                    let mut v: Vec<f64> = self.combined.iter().copied().collect();
                    self.baseline = Some(median_of(&mut v));
                }
            }
        }
        let Some(base) = self.baseline else {
            return None;
        };
        let mut window: Vec<f64> = self.combined.iter().copied().collect();
        let rho_cur = median_of(&mut window);
        let c = if base > 1e-9 {
            (rho_cur / base).clamp(RLI_CPLX_C_MIN, RLI_CPLX_C_MAX)
        } else {
            1.0
        };
        self.current_c = Some(c);
        // 三条阈值（θ 同法：log 空间乘性更新，平衡点 P(c ≥ θ) = 1−q）。
        for (idx, (_, q)) in RLI_CPLX_TIERS.iter().enumerate() {
            if c >= self.theta[idx] {
                self.theta[idx] *= libm::exp(RLI_ETA_INIT);
                if c > 1.0 {
                    // 越线锁存（含中途尖峰；「每档一次」的簿记在宿主面）。
                    self.latched[idx] = true;
                }
            } else {
                self.theta[idx] *= libm::exp(-RLI_ETA_INIT * (1.0 - q) / q);
            }
        }
        Some(c)
    }

    fn snapshot(&self) -> RliComplexitySnapshot {
        RliComplexitySnapshot {
            channel_windows: self
                .channel_windows
                .iter()
                .map(|w| w.iter().map(|v| quantize_state(*v)).collect())
                .collect(),
            pending: self
                .pending
                .iter()
                .map(|p| {
                    p.iter()
                        .map(|e| RliPendingPrediction {
                            issue_t: quantize_state(e.issue_t),
                            issue_u: quantize_state(e.issue_u),
                            pred: quantize_state(e.pred),
                            due_t: quantize_state(e.due_t),
                        })
                        .collect()
                })
                .collect(),
            combined: self.combined.iter().map(|v| quantize_state(*v)).collect(),
            baseline: self.baseline.map(quantize_state),
            samples: self.samples,
            theta: self.theta.iter().map(|v| quantize_state(*v)).collect(),
            latched: self.latched.to_vec(),
        }
    }

    /// 从快照续接；尺寸不符（legacy／损坏）＝保持 fresh 状态并返回 false
    /// （调用方口径：繁杂度是可选读数面，坏结构不影响通道续接）。
    fn restore(&mut self, snapshot: &RliComplexitySnapshot) -> bool {
        if snapshot.channel_windows.len() != RLI_CPLX_KINDS.len()
            || snapshot.pending.len() != RLI_CPLX_KINDS.len()
            || snapshot.theta.len() != RLI_CPLX_TIERS.len()
            || snapshot.latched.len() != RLI_CPLX_TIERS.len()
        {
            return false;
        }
        for (slot, values) in snapshot.channel_windows.iter().enumerate() {
            self.channel_windows[slot] = values
                .iter()
                .rev()
                .take(RLI_CPLX_PER_CHANNEL_CAP)
                .rev()
                .copied()
                .collect();
        }
        for (slot, entries) in snapshot.pending.iter().enumerate() {
            self.pending[slot] = entries
                .iter()
                .rev()
                .take(RLI_CPLX_PER_CHANNEL_CAP)
                .rev()
                .copied()
                .collect();
        }
        self.combined = snapshot
            .combined
            .iter()
            .rev()
            .take(RLI_CPLX_PER_CHANNEL_CAP)
            .rev()
            .copied()
            .collect();
        self.baseline = snapshot.baseline.filter(|v| v.is_finite() && *v > 0.0);
        self.samples = snapshot.samples;
        for (idx, theta) in snapshot.theta.iter().enumerate() {
            if theta.is_finite() && *theta > 1e-9 {
                self.theta[idx] = *theta;
            }
        }
        for (idx, latched) in snapshot.latched.iter().enumerate() {
            self.latched[idx] = *latched;
        }
        self.current_c = None; // 由下一次决策轮重算（读数不虚构）。
        true
    }
}

/// 繁杂度读数（只读快照；宿主面投递判定与渲染面共用）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RliComplexityReading {
    /// 基线是否已冻结（未就绪 = 无提醒——退化路径）。
    pub ready: bool,
    /// 已产出合成样本数。
    pub samples: u64,
    /// 当前放大倍数 c（未就绪 = `None`）。
    pub c: Option<f64>,
    /// 本会话前段基线 ρ_base。
    pub rho_base: Option<f64>,
    /// 三条自校准阈值（θ85/θ95/θ99，顺序 = [`RLI_CPLX_TIERS`]）。
    pub theta: [f64; RLI_CPLX_TIERS.len()],
    /// 越线锁存（顺序同上）。
    pub latched: [bool; RLI_CPLX_TIERS.len()],
}

fn push_capped<T>(deque: &mut VecDeque<T>, item: T, cap: usize) {
    deque.push_back(item);
    if deque.len() > cap {
        deque.pop_front();
    }
}

fn median_of(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    if n == 0 {
        return f64::NAN;
    }
    if n % 2 == 1 {
        values[n / 2]
    } else {
        0.5 * (values[n / 2 - 1] + values[n / 2])
    }
}

/// 模型面提醒种类（0bf ③，2026-09-22）——用户令「模型面提醒只留两个触发」：
/// 域迁移完成（震荡结束后确认一次）与持续性越线（连续 k 采样点）。
/// **0cp D7（167 批）新增域事件提醒族第三触发源**：spike 进入端即提醒
/// （域瞬态偏移发生当刻；回归端不提醒——不能假设现实任务能被分出稳定域
/// 特征，稳定门会把提醒面在现实任务中滤成事实死亡）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RliNoticeKind {
    /// 域迁移确认（等震荡结束后报一次；域事件提醒族触发源①——稳定门）。
    MigrationConfirmed,
    /// 持续越线（通道 `u ≥ θ` 连续 [`RLI_STREAK_K`] 个动作采样点）。
    StreakCrossed,
    /// 掩盖缺口（0bg S1，2026-09-22 用户裁决「直接做掩盖缺口吧」）：**域
    /// 覆盖缺口**事实——就绪（已完成段 ≥ [`RLI_COVERAGE_MIN_SEGMENTS`]）∧
    /// 存在未访问域（分母＝四值域枚举，`Start` 不计）∧ 当前段驻留 ≥ 该域
    /// 已完段驻留中位（不足退全域中位）时，在**触发沿**报一次；域切换后
    /// 重武装。**每轮最多一条域类提醒**（与域迁移确认同轮时让位，沿不吞——
    /// 条件持续成立即下一轮报）。
    CoverageGap,
    /// **0cp D7**（2026-10-03 用户裁定）：域 spike 进入端即时提醒——自判域
    /// 切换当刻报一次（无需等待稳定）；回归端（**凡异常域→Normal**；169 批
    /// 勘误：原枚举 Stuck/LowProgress→Normal 不完备，Pressure→Normal 同属
    /// 恢复）与 bootstrap（Start→首域）不提醒（回归只计数）。域机器本身
    /// 不动（spike 判定、域状态机归 0am）——改的只是「哪些 LIF 事件产生
    /// 模型面提醒」。
    DomainSpikeEntry,
}

/// **0cp D6**（166 批，2026-10-03 用户裁定）：触发提醒行自带的 **T̂ 参数
/// 含义注解**（单一源——四类提醒行的趋势段共用本前缀；模型免回查即可
/// 理解读数含义）。趋势读数形如「{本前缀}：8.94→31.89 ↑」（前值→现值＋
/// 方向；无趋势＝仅现值）；**不含趋势成因归因**（用户裁定：只给趋势，
/// 不给成因）。
pub const RLI_T_HAT_ANNOTATION: &str = "T̂(单轮工具周期会话自适应估计，秒)";

/// 0bg S2（2026-09-22，用户裁决「该加的注解都加上」⇒ **随报**全采纳）：
/// 每条模型面提醒的**固定模板注解**（术语释义／基准／非阻断声明；**自含**
/// ——不引用历史消息，压缩后仍可解读）。**单一源（FR-5）**：提醒文案、面头
/// 符号表与工具描述都引用本处；钉子保同步（`notice_texts_stay_within_budget`
/// 与 `symbol_legend_is_single_source`）。预算：本体＋注解
/// ≤ [`RLI_NOTICE_TEXT_BUDGET`] B/条（0cp D6 直投行同预算纪律）。
pub fn rli_notice_annotation(kind: RliNoticeKind) -> &'static str {
    match kind {
        RliNoticeKind::CoverageGap => {
            // 0cp D6 压缩：行内已带 T̂ 注解，尾注解只保留判据键释义
            // （「域失配」字面自明；预算钉见 notice_texts_stay_within_budget）。
            "〔g=未访域占比；r=驻留÷中位；仅读数非阻断〕"
        }
        RliNoticeKind::MigrationConfirmed | RliNoticeKind::DomainSpikeEntry => {
            // 0cp D7：域事件提醒族二触发源共用同一行格式与注解（行内自带
            // T̂ 参数含义注解＋动作特征＋域趋势——参数含义已由行内 T̂ 注解
            // 承载，尾注解只保留非阻断声明以守住 240B 预算）。
            "〔仅读数非阻断〕"
        }
        RliNoticeKind::StreakCrossed => "〔连续k个动作采样点u≥θ；失配=ρ>1占比；仅读数非阻断〕",
    }
}

/// 0bg S2：提醒**本体＋注解**的字节预算（B/条）——注解随报的成本上界
/// （用户裁决「随报的成本不大…关键是要方便模型理解」的口径门）。
/// **0am P8-b 重订（2026-10-03，S2 §4.3/§8）**：成因段（标签维「源：…」）
/// 入行后 240B 必越限（S2 档 §4.4 预见）——新预算 **320B**（最坏 D6 形态
/// ≈270B＋余量；0cp D6「无成因」子句由 S2 P8 取代，行宽子句随本批修订）。
pub const RLI_NOTICE_TEXT_BUDGET: usize = 320;

/// 0bg S2：覆盖缺口就绪门——已完成段数下限（用户令「冷启动久些不是坏事，
/// 域建模与预测本就需要基础数据量」；域图需基础数据量）。
pub const RLI_COVERAGE_MIN_SEGMENTS: u64 = 3;

/// 0bg S2：RLI 面**面头固定符号表**（一行；不逐行注解、避免挤掉读数——
/// 用户裁决 (c)）。与 [`rli_notice_annotation`] 同为单一源素材：工具描述
/// 由本常量拼入（钉子保同步）。
pub const RLI_SYMBOL_LEGEND: &str = "符号: u=水平 v=速率 pred=闭式前推 p1=短视 E=包络 r=节律 θ=阈 λ̂=到达率 ρ=失配率 c=繁杂度 T̂=轮语义秒";

/// 一次模型面提醒（0bf ③）。**0cp D4（163 批）投递语义改造**：提醒在
/// **触发当刻**转附注式直投——宿主在下一工具批回传装配点取走（
/// [`RliShadow::take_pending_for_push`]）并附 `mechanical_audit_update`
/// 注入行，同刻 journal 留痕（盲区闭环）；`delivered` ＝ 附注行已装配。
/// 队列保留为**事件历史**（cap 16 不变，随侧车持久——不跨会话补投）。
/// `text` 为机械事实文案（只含读数与特征，**不含动作建议**——0bf ② 纪律；
/// 0cp D6 起自带 T̂ 参数含义注解＋趋势）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RliNotice {
    pub kind: RliNoticeKind,
    /// 触发时刻（会话相对秒）。
    pub t: f64,
    /// 触发时决策轮号（动作触发点 = 当前域轮号；FR-7：不可得用 `None`）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub round: Option<u64>,
    /// 机械事实文案（含失配概率；由触发时读数生成，事后不可变）。
    pub text: String,
    /// 已投递（附注行已装配）；未投递 = 待下一工具批装配点取走。
    #[serde(default)]
    pub delivered: bool,
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
    /// 0be 四项④（2026-09-21）：会话级繁杂度读数（指标链见
    /// [`RliComplexity`]；随侧车延续，不设跨会话桥接）。
    cplx: RliComplexity,
    /// 0be 四项③（2026-09-21）：自适应参数轨迹（决策轮粒度、cap
    /// [`RLI_ADAPT_TRACE_CAP`]、随侧车持久——复算可核）。
    adapt_trace: VecDeque<RliAdaptTraceRow>,
    /// **0cp D2**（2026-10-03）：上一**动作采样点**时刻（决策轮／工具事件
    /// ——看门狗 ① 窗起点；原 0bf ④ 网格补点锚职责随网格退役转为看门狗
    /// 窗口）。
    last_sample_t: Option<f64>,
    /// 0bf ③：模型面提醒队列（旧在前、cap [`RLI_NOTICE_CAP`]；随侧车持久；
    /// **0cp D4 起＝事件历史**，投递改直投）。
    notices: VecDeque<RliNotice>,
    /// **0cp D2**：看门狗武装标志——每个动作采样点置 `true`；看门狗触发
    /// 采样后置 `false`（休眠，恰一样）。随侧车持久（legacy 快照缺字段 =
    /// `true`，宁可多采一次也不吞）。
    watchdog_armed: bool,
    /// **0cp D6**：T̂ 前值（最近一次变化前的轮语义估计）——触发提醒行趋势
    /// 段「前值→现值 ↑」的数据面；无变化＝与现值同（不渲染趋势）。
    t_hat_prev: Option<f64>,
    /// **0cp D7**：域 spike 进入端累计次数（回归与 bootstrap 不计）——
    /// 域事件提醒行「域趋势」段；随侧车持久（legacy = 0）。
    spike_entries: u64,
    /// **0cp D7**：域 spike 回归端累计次数（Stuck/LowProgress→Normal）。
    spike_returns: u64,
    /// **0cp D7**：近期动作形态窗（最近 5 个工具事件 outcome；live-only
    /// 不随侧车持久——域事件提醒行「近期动作」段的数据面）。
    recent_outcomes: VecDeque<ToolOutcome>,
    /// 0bf ④：采样点总数（决策轮＋工具事件＋看门狗单点；开销读数面）。
    sample_points: u64,
    /// 0bg S2（2026-09-22）：掩盖缺口（[`RliNoticeKind::CoverageGap`]）**触发
    /// 沿重武装**标志——触发一次后置 `false`，域切换重武装（`true`）。随
    /// 侧车持久（legacy 快照缺字段 = `true`，宁可多报一次也不吞）。
    coverage_gap_armed: bool,
    /// 0bh ①（2026-09-22）：提醒**投递总数**（pull-delta 头实际携带过的条数；
    /// 投递率分子；随侧车持久）。
    notice_delivered_total: u64,
    /// 0bh ①：**预算受阻暂存数**（本轮装不下、留待下次读取的条数；投递率
    /// 分母的另一半；不重不漏——`delivered + deferred` 对照提醒队列总数可核）。
    notice_deferred_total: u64,
    /// 0bh ①：最近一次装配后的**头段余量**（字节；独立提醒预算减已用——
    /// 「头段余量可核」钉子；随侧车持久）。
    notice_headroom_bytes: u64,
    /// **0am P8-b**（2026-10-03，S2 §4.3 标签成因）：各通道最近注入的机械
    /// 标签环（cap [`RLI_CHANNEL_LABEL_CAP`]；**live-only** 不随侧车持久；
    /// streak 成因段「驱动标签名＋计数」数据面——闭集词表见
    /// [`RliShadow::stamp_label`] 调用点）。
    channel_labels: [VecDeque<&'static str>; RLI_CHANNELS.len()],
}

/// 标签环容量（P8 成因段读数窗；触发沿渲染按环内计数）。
pub const RLI_CHANNEL_LABEL_CAP: usize = 8;

impl Default for RliShadow {
    fn default() -> Self {
        Self::new()
    }
}

impl RliShadow {
    /// 全新影子（T̂₀ ＝ 估计器初值单源〔RS-06 双源消解，2026-10-03〕——
    /// 预启用节奏与 1D 引擎同源）。
    pub fn new() -> Self {
        let t_hat0 = crate::lif::estimator::T_HAT_INIT_SECS;
        Self {
            channels: [
                RliChannel::new(ChannelKind::Err, TAU / RLI_ERR_PERIOD_SECS),
                RliChannel::new(ChannelKind::Stall, TAU / RLI_STALL_PERIOD_SECS),
                RliChannel::new(ChannelKind::Slow, TAU / RLI_SLOW_PERIOD_SECS),
                RliChannel::new(ChannelKind::Deny, TAU / RLI_DENY_PERIOD_SECS),
                RliChannel::new(ChannelKind::Prog, TAU / (RLI_PROG_PERIOD_ROUNDS * t_hat0)),
                // 0am P8（S2 §2 预注册初值）：轮语义周期 k·T̂，随 T̂ 重导出。
                RliChannel::new(
                    ChannelKind::Verify,
                    TAU / (RLI_VERIFY_PERIOD_ROUNDS * t_hat0),
                ),
                RliChannel::new(ChannelKind::Ctx, TAU / (RLI_CTX_PERIOD_ROUNDS * t_hat0)),
                RliChannel::new(ChannelKind::Infra, TAU / (RLI_INFRA_PERIOD_ROUNDS * t_hat0)),
            ],
            t_hat: t_hat0,
            last_tool_t: None,
            steps: 0,
            domain: RliDomainMachine::new(),
            anchor_series: (0..RLI_ANCHOR_FEATURE_TABLE.len())
                .map(|_| VecDeque::with_capacity(RLI_FEATURE_SERIES_CAP))
                .collect(),
            cplx: RliComplexity::new(),
            adapt_trace: VecDeque::with_capacity(RLI_ADAPT_TRACE_CAP),
            last_sample_t: None,
            notices: VecDeque::with_capacity(RLI_NOTICE_CAP),
            // 0cp D2：看门狗武装从待触发起（首个动作采样点后维持武装）。
            watchdog_armed: true,
            // 0cp D6：T̂ 趋势数据面（首个决策轮记下初值变迁）。
            t_hat_prev: None,
            // 0cp D7：域事件计数从零起。
            spike_entries: 0,
            spike_returns: 0,
            recent_outcomes: VecDeque::with_capacity(RLI_RECENT_OUTCOMES_CAP),
            // 0bf ④：采样点总数从零起（决策轮＋工具事件＋看门狗单点）。
            sample_points: 0,
            // 0bg S2：掩盖缺口触发沿重武装——新建即待触发。
            coverage_gap_armed: true,
            // 0bh ①：投递面会计从零起（投递率可核）。
            notice_delivered_total: 0,
            notice_deferred_total: 0,
            notice_headroom_bytes: 0,
            // 0am P8-b：标签环从空起（live-only）。
            channel_labels: (0..RLI_CHANNELS.len())
                .map(|_| VecDeque::with_capacity(RLI_CHANNEL_LABEL_CAP))
                .collect::<Vec<_>>()
                .try_into()
                .expect("label rings sized to channel bank"),
        }
    }

    /// **0am P8-b**（S2 §4.3 标签成因）：注入时盖机械标签（闭集词表——
    /// 调用点逐一具名；`&'static str` 直存，零格式化开销）。
    fn stamp_label(&mut self, kind: ChannelKind, label: &'static str) {
        let ring = &mut self.channel_labels[Self::index_of(kind)];
        ring.push_back(label);
        while ring.len() > RLI_CHANNEL_LABEL_CAP {
            ring.pop_front();
        }
    }

    /// 成因段标签维：环内按标签计数（旧在前），如「执行失败×3＋验证失败×1」；
    /// 空环＝None（无注入史——不虚构成因）。
    fn channel_label_cause(&self, kind: ChannelKind) -> Option<String> {
        let ring = &self.channel_labels[Self::index_of(kind)];
        if ring.is_empty() {
            return None;
        }
        let mut counts: Vec<(&'static str, usize)> = Vec::new();
        for label in ring {
            match counts.iter_mut().find(|(l, _)| l == label) {
                Some((_, c)) => *c += 1,
                None => counts.push((label, 1)),
            }
        }
        Some(
            counts
                .iter()
                .map(|(l, c)| format!("{l}×{c}"))
                .collect::<Vec<_>>()
                .join("＋"),
        )
    }

    /// 拒绝类 → 机械标签（闭集；Deny 通道成因段）。
    fn deny_label(dc: crate::lif::router::DenyClass) -> &'static str {
        use crate::lif::router::DenyClass as DC;
        match dc {
            DC::PermissionTicket => "权限拒绝",
            DC::PlanLane => "计划/车道拒绝",
            DC::GateGuard => "门/护栏拒绝",
            DC::RetrievalEnable => "检索启用拒绝",
            DC::WriteControl => "写控拦截",
            DC::PolicyMarker => "策略拒绝",
            DC::Other => "其他拒绝",
        }
    }

    fn index_of(kind: ChannelKind) -> usize {
        match kind {
            ChannelKind::Err => 0,
            ChannelKind::Stall => 1,
            ChannelKind::Slow => 2,
            ChannelKind::Deny => 3,
            ChannelKind::Prog => 4,
            // 0am P8：新通道索引恒追加（既有五通道索引稳定＝旧侧车快照兼容）。
            ChannelKind::Verify => 5,
            ChannelKind::Ctx => 6,
            ChannelKind::Infra => 7,
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

    /// 繁杂度状态（0be 四项④；会话级读数——指标链见 [`RliComplexity`]）。
    pub fn complexity(&self) -> &RliComplexity {
        &self.cplx
    }

    /// 繁杂度读数（只读快照；宿主面投递判定与渲染共用）。
    pub fn complexity_reading(&self) -> RliComplexityReading {
        self.cplx.reading()
    }

    /// 自适应参数轨迹（0be 四项③；决策轮粒度、旧在前、cap
    /// [`RLI_ADAPT_TRACE_CAP`]）。
    pub fn adapt_trace(&self) -> Vec<RliAdaptTraceRow> {
        self.adapt_trace.iter().copied().collect()
    }

    /// 模型面提醒队列（0bf ③；旧在前、含已投递项——事件追溯面）。
    pub fn notices(&self) -> Vec<&RliNotice> {
        self.notices.iter().collect()
    }

    /// 未投递提醒（**0cp D4 直投装配点的输入**；旧在前）。
    pub fn pending_notices(&self) -> Vec<&RliNotice> {
        self.notices.iter().filter(|n| !n.delivered).collect()
    }

    /// **0cp D4（2026-10-03）**：取走全部未投递提醒并即刻置位 `delivered`
    /// （附注行已装配——投递语义，同刻 journal 留痕由宿主装配点写入）；
    /// 返回克隆（触发时定格文案；返回态与队列一致＝已装配）。
    /// 无后续工具批（会话即终止）＝不跨会话补投（如实弃置于队列历史）。
    ///
    /// 169 批审查修正：取走与置位改为**按未投递谓词直取**（克隆先于置位、
    /// 返回态随置位改写）——不再依赖「已投递前缀／未投递后缀」队列序不变
    /// 量去反推「末尾 N 条」，消除未来任何部分置位调用形态下的取错集风险。
    pub fn take_pending_for_push(&mut self) -> Vec<RliNotice> {
        let mut taken: Vec<RliNotice> = self
            .notices
            .iter()
            .filter(|n| !n.delivered)
            .cloned()
            .collect();
        if taken.is_empty() {
            return taken;
        }
        let mut marked = 0usize;
        for notice in &mut self.notices {
            if !notice.delivered {
                notice.delivered = true;
                marked += 1;
            }
        }
        for notice in &mut taken {
            notice.delivered = true;
        }
        self.notice_delivered_total += marked as u64;
        taken
    }

    /// 标记前 `n` 条未投递提醒为已投递（头携带即投递；返回实际条数）。
    pub fn mark_notices_delivered(&mut self, n: usize) -> usize {
        let mut marked = 0usize;
        for notice in self.notices.iter_mut() {
            if marked >= n {
                break;
            }
            if !notice.delivered {
                notice.delivered = true;
                marked += 1;
            }
        }
        self.notice_delivered_total += marked as u64;
        marked
    }

    /// 0bh ①（2026-09-22）：按**未投递列表索引**标记投递——域类提醒的让位
    /// （每次至多 1 条）会让「前 n 条」口径错位（被让位者从未到达却可能被
    /// 标成已投递）。装配循环持有索引，此接口与循环严格同口径。
    /// 返回实际标记条数，并累计 `notice_delivered_total`。
    ///
    /// **0cp D4 起＝冻结面**（169 批注记）：唯一调用方 pull-delta 头装配
    /// 循环已随直投改造整体退役，生产零调用；保留为 pub 库面（历史侧车/
    /// 复算工具可用），不删除、不扩张。
    pub fn mark_notices_delivered_at(&mut self, indices: &[usize]) -> usize {
        let pending: Vec<usize> = self
            .notices
            .iter()
            .enumerate()
            .filter(|(_, n)| !n.delivered)
            .map(|(i, _)| i)
            .collect();
        let mut marked = 0usize;
        for &pending_pos in indices {
            if let Some(&idx) = pending.get(pending_pos) {
                self.notices[idx].delivered = true;
                marked += 1;
            }
        }
        self.notice_delivered_total += marked as u64;
        marked
    }

    /// 0bh ①（2026-09-22）：投递面会计——预算受阻的暂存条数与**头段余量**
    /// （字节）落影子（随侧车持久；「头段余量可核＋投递率计数」钉子）。
    ///
    /// **0cp D4 起＝冻结面**（169 批注记）：唯一调用方 pull-delta 头装配
    /// 循环已随直投改造整体退役，生产零调用（两计数面生产不再写入）；保留
    /// 为 pub 库面（快照兼容与历史侧车可读），不删除、不扩张。
    pub fn record_notice_delivery_accounting(&mut self, deferred: usize, headroom_bytes: usize) {
        self.notice_deferred_total += deferred as u64;
        self.notice_headroom_bytes = headroom_bytes as u64;
    }

    /// 0bh ①：投递率读数 `(已投递, 预算受阻, 最近头段余量字节)`。
    ///
    /// **0cp D4（2026-10-03）投递语义改造**：`delivered` ＝ 附注行已装配
    /// （[`Self::take_pending_for_push`] 直投取走即置位）；「预算受阻／头段
    /// 余量」两计数面随 pull-delta 投递退役冻结（字段保留＝快照兼容与历史
    /// 侧车可读，生产不再写入）——投递率＝附注装配数／fire 数（可核）。
    pub fn notice_delivery_stats(&self) -> (u64, u64, u64) {
        (
            self.notice_delivered_total,
            self.notice_deferred_total,
            self.notice_headroom_bytes,
        )
    }

    /// **0cp D6**：T̂ 趋势段文案（触发提醒行共用；D6 格式「前值→现值 ↑，
    /// 无成因」）。无前值或无实质变化（<0.05s）＝仅现值（不虚构趋势）。
    pub(crate) fn t_hat_trend_text(&self) -> String {
        let cur = self.t_hat;
        match self.t_hat_prev {
            Some(prev) if (prev - cur).abs() >= 0.05 => {
                let arrow = if cur > prev { "↑" } else { "↓" };
                format!("{RLI_T_HAT_ANNOTATION}：{prev:.2}→{cur:.2} {arrow}")
            }
            _ => format!("{RLI_T_HAT_ANNOTATION}：{cur:.2}"),
        }
    }

    /// **0cp D7**：近期动作形态段文案（「；近N: 成x误y拒z」；空窗＝空串，
    /// 不渲染）。只数特征、无建议。
    pub(crate) fn recent_actions_text(&self) -> String {
        if self.recent_outcomes.is_empty() {
            return String::new();
        }
        let (mut ok, mut err, mut deny) = (0u32, 0u32, 0u32);
        for o in &self.recent_outcomes {
            match o {
                ToolOutcome::Success => ok += 1,
                ToolOutcome::Error => err += 1,
                ToolOutcome::Deny => deny += 1,
                ToolOutcome::Other => {}
            }
        }
        format!("；近{}: 成{ok}误{err}拒{deny}", self.recent_outcomes.len())
    }

    /// **0cp D7**：域事件计数读数 `(进入端累计, 回归端累计)`（测试与读数面）。
    pub fn spike_counts(&self) -> (u64, u64) {
        (self.spike_entries, self.spike_returns)
    }

    /// 0bg S2（2026-09-22）：域覆盖读数（掩盖缺口判据与随报字段的单一
    /// 来源）——就绪/越线判定由 [`Self::coverage_gap_text`] 消费，测试与
    /// 渲染可直接读。
    pub fn coverage_stats(&self) -> RliCoverageStats {
        self.domain.coverage_stats()
    }

    /// 采样点总数（决策轮＋工具事件＋看门狗单点；0bf ④ 开销读数面，
    /// 0cp D1 起无网格补点）。
    pub fn sample_points(&self) -> u64 {
        self.sample_points
    }

    /// 失配概率读数（0bf ③；ρ > 1 占比，未就绪 = `None`——只记录不反馈）。
    pub fn mismatch_rate(&self) -> Option<f64> {
        self.cplx.mismatch_rate()
    }

    /// 域转移倾向读数（0bf ②；本会话纯经验统计——由自判域机器推导）。
    pub fn transition_tendency(&self) -> RliTransitionTendency {
        self.domain.transition_tendency()
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

    /// **0cp D1（2026-10-03）动作采样点结算**（决策轮／工具事件两源共用）
    /// ——锚点序列采样 ＋ 持续越线观察 ＋ 采样计数 ＋ 看门狗重武装；末尾
    /// 盖**动作采样锚**（看门狗 ① 窗起点）。持续越线只观察四条压力通道
    /// （err／stall／slow／deny；`prog` 不参与——新鲜度高不是异常）。观察
    /// 为纯读数：不改 θ、不改 fires、不反馈。
    fn note_sample_point(&mut self, t: f64) {
        self.sample_points = self.sample_points.saturating_add(1);
        self.sample_anchor_series();
        self.observe_streaks_and_fire(t);
        self.last_sample_t = Some(t);
        // 0cp D2：动作采样点重武装看门狗（每个动作样都重置单点窗）。
        self.watchdog_armed = true;
    }

    /// **0cp D1**：持续越线观察＋触发沿成提醒（动作采样点与看门狗单点两
    /// 源共用）。0cp D6：行内自带 T̂ 参数含义注解＋趋势（无成因）。
    fn observe_streaks_and_fire(&mut self, t: f64) {
        let mut fired: Vec<(ChannelKind, u64, f64, f64)> = Vec::new();
        for ch in &mut self.channels {
            if ch.kind == ChannelKind::Prog {
                continue;
            }
            if ch.observe_streak(t) {
                fired.push((ch.kind, ch.streak(), ch.u(), ch.theta()));
            }
        }
        if fired.is_empty() {
            return;
        }
        let p = self.mismatch_probability_text();
        let listed = fired
            .iter()
            .map(|(kind, streak, _, _)| format!("{}×{}", channel_label(*kind), streak))
            .collect::<Vec<_>>()
            .join(" ");
        let detail = if fired.len() == 1 {
            let (_, _, u, theta) = fired[0];
            format!("（u={u:.2}≥θ={theta:.2}；失配概率 {p}）")
        } else {
            format!("（失配概率 {p}）")
        };
        let t_hat_segment = self.t_hat_trend_text();
        // **0am P8-b**（S2 §4.3 成因段·标签维）：单 fire 时附「源：驱动标签
        // ×环内计数」——机械标签仅述已发生注入，不构成触发条件（P9）；
        // 多 fire 行保持机制成因（宽度纪律）。
        let label_cause = if fired.len() == 1 {
            let (kind, ..) = fired[0];
            self.channel_label_cause(kind)
                .map(|c| format!("；源：{c}"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let round = (self.domain.round() > 0).then(|| self.domain.round());
        self.push_notice(RliNotice {
            kind: RliNoticeKind::StreakCrossed,
            t,
            round,
            text: format!("持续越线: {listed}{detail}{label_cause}；{t_hat_segment}"),
            delivered: false,
        });
    }

    /// **0cp D2 卡死看门狗**（2026-10-03 用户裁定）：无动作样且无活动的
    /// 停滞期，恰产生**一个**时间采样后休眠。三同时触发：①已武装且自上一
    /// 动作采样点起 Δt ≥ [`RLI_WATCHDOG_SAMPLE_SECS`]；②该窗内无活动
    /// （`activity_idle_secs` ≥ 同阈——运行时以心跳钟 idle 度量：流式
    /// chunk／SSE 帧／journal 记录皆盖章，生成活动本身即「活」的证据）；
    /// ③run 仍活跃（由宿主任务随 run 终止退出保证）。采样＝推进动力学 →
    /// θ 评估 → streak/锚点采样 → 若成触发沿则按 D4 直投；**不**盖动作
    /// 采样锚、**不**计入 steps（过阈率分母口径不变）。
    ///
    /// 窗锚缺失（`last_sample_t` = `None`，尚无任何动作样）＝不触发
    /// （保守：不虚构窗起点；进程级静默由既有 360s stall 看门狗兜底）。
    pub fn on_watchdog_tick(&mut self, now: f64, activity_idle_secs: f64) -> bool {
        if !self.watchdog_due(now, activity_idle_secs) {
            return false;
        }
        self.watchdog_armed = false;
        for ch in &mut self.channels {
            ch.advance(now);
        }
        for ch in &mut self.channels {
            ch.check(now);
        }
        self.sample_points = self.sample_points.saturating_add(1);
        self.sample_anchor_series();
        self.observe_streaks_and_fire(now);
        true
    }

    /// 看门狗触发判定（与 [`Self::on_watchdog_tick`] 同口径，单测可纯调）。
    pub fn watchdog_due(&self, now: f64, activity_idle_secs: f64) -> bool {
        self.watchdog_armed
            && self
                .last_sample_t
                .is_some_and(|t0| now.is_finite() && now - t0 >= RLI_WATCHDOG_SAMPLE_SECS)
            && activity_idle_secs.is_finite()
            && activity_idle_secs >= RLI_WATCHDOG_SAMPLE_SECS
    }

    /// 0bg S2（2026-09-22）：掩盖缺口文案（只给读数与特征、无动作建议）。
    /// 返回 `None` ＝ 未达触发条件（就绪门／缺口 `g>0`／越线门——判据读自
    /// [`RliCoverageStats`] 单一来源）。**0cp D6**：行内自带 T̂ 参数含义
    /// 注解＋趋势；文案＋注解 ≤ [`RLI_NOTICE_TEXT_BUDGET`] B/条（由
    /// `push_notice` 统一追加注解）。
    fn coverage_gap_text(&self) -> Option<String> {
        let stats = self.domain.coverage_stats();
        if stats.completed_segments < RLI_COVERAGE_MIN_SEGMENTS || stats.uncovered.is_empty() {
            return None;
        }
        // 越线门：当前段驻留 ≥ 该域已完段中位；不足退会话内全域中位。
        let median = stats.current_median.or(stats.fallback_median)?;
        if stats.current_dwell < median {
            return None;
        }
        let listed = stats
            .uncovered
            .iter()
            .take(2)
            .map(|d| d.as_str())
            .collect::<Vec<_>>()
            .join("/");
        let more = if stats.uncovered.len() > 2 { "…" } else { "" };
        Some(format!(
            "掩盖缺口: 未访 {listed}{more}；g={:.2}；驻留{}/中位{median}；\
             域失配{}轮；{}",
            stats.g,
            stats.current_dwell,
            stats.overdue_rounds,
            self.t_hat_trend_text(),
        ))
    }

    /// 0bg S2：掩盖缺口**触发沿**评估——就绪 ∧ g>0 ∧ 越线 ⇒ 报一次并关闭
    /// 沿（域切换／`restore` 重武装）；已关闭或条件不满足 ⇒ `false`。
    /// 抽为独立方法：触发逻辑可单测、不经决策轮的域机副作用。
    pub(crate) fn maybe_push_coverage_gap(&mut self, t: f64) -> bool {
        if !self.coverage_gap_armed {
            return false;
        }
        let Some(text) = self.coverage_gap_text() else {
            return false;
        };
        let round = (self.domain.round() > 0).then(|| self.domain.round());
        self.push_notice(RliNotice {
            kind: RliNoticeKind::CoverageGap,
            t,
            round,
            text,
            delivered: false,
        });
        self.coverage_gap_armed = false;
        true
    }

    /// 失配概率文案（0bf ③）：最近 ρ 样本中失配（ρ > 1）占比；未就绪 =
    /// 「—」（FR-7：不可得与真值 0 不混同）。
    fn mismatch_probability_text(&self) -> String {
        self.cplx
            .mismatch_rate()
            .map_or_else(|| "—".to_string(), |p| format!("{p:.2}"))
    }

    /// 0bf ③：压入模型面提醒（cap [`RLI_NOTICE_CAP`]，超限丢最旧）。
    ///
    /// 0bg S2（2026-09-22）：**注解随报**——文案在本方法统一追加
    /// [`rli_notice_annotation`]（固定模板；单一源，不重复散落各构造点），
    /// 本体＋注解 ≤ [`RLI_NOTICE_TEXT_BUDGET`] B/条（钉子
    /// `notice_texts_stay_within_budget` 逐 kind 断言）。
    fn push_notice(&mut self, mut notice: RliNotice) {
        notice.text.push_str(rli_notice_annotation(notice.kind));
        self.notices.push_back(notice);
        while self.notices.len() > RLI_NOTICE_CAP {
            self.notices.pop_front();
        }
    }

    /// 决策轮：更新轮语义 ω（prog = 2π/(8·T̂)），全通道自由演化 + 逐一阈值
    /// 评估 + 自判域记录（0am 改造四项①）+ 锚点序列采样（补充项④）。
    /// 域判决输入＝RLI 原生锚点（err 的 u/v/E ＋ prog 的 u；见
    /// [`RliDomainMachine::label`]）；**不再携带 LIF 现域**（补充项②：
    /// RLI 不与 LIF 对照，一致性口径改对「框架实际动作结果」）。
    /// **0cp D1**：动作采样点（网格补点退役——闭式精确解按 Δt 连续演化，
    /// 补点对状态零贡献，唯一作用曾是制造假连续采样污染 streak 判定）。
    pub fn on_decision_round(&mut self, t: f64, t_hat_secs: f64) {
        let t_hat_new = t_hat_secs.max(1e-9);
        // 0cp D6：T̂ 前值追踪（最近一次变化前的值——触发提醒行趋势段）。
        if (t_hat_new - self.t_hat).abs() > f64::EPSILON {
            self.t_hat_prev = Some(self.t_hat);
        }
        self.t_hat = t_hat_new;
        self.steps = self.steps.saturating_add(1);
        // 轮语义通道族（prog＋0am P8 的 verify/ctx/infra）：周期 k·T̂ 随
        // T̂ 每轮重导出（**prog-ω 热更新建模注记〔RS-06 P3，2026-10-03〕**：
        // 热更新＝单位换算臂——周期物理定义 k·T̂ 的秒化，非参数拟合/热调参；
        // P3 物理性三条件之「由构造设定」臂，永不对结局拟合）。
        let round_semantic_omega = |k: f64| TAU / (k * self.t_hat);
        for ch in &mut self.channels {
            ch.omega = match ch.kind {
                ChannelKind::Prog => round_semantic_omega(RLI_PROG_PERIOD_ROUNDS),
                ChannelKind::Verify => round_semantic_omega(RLI_VERIFY_PERIOD_ROUNDS),
                ChannelKind::Ctx => round_semantic_omega(RLI_CTX_PERIOD_ROUNDS),
                ChannelKind::Infra => round_semantic_omega(RLI_INFRA_PERIOD_ROUNDS),
                _ => ch.omega,
            };
            ch.advance(t);
        }
        for ch in &mut self.channels {
            ch.check(t);
        }
        let u_err = self.channel(ChannelKind::Err).u();
        let v_err = self.channel(ChannelKind::Err).v();
        let env_err = self.channel(ChannelKind::Err).envelope();
        let u_prog = self.channel(ChannelKind::Prog).u();
        let domain_before = self.domain.current_domain();
        self.domain.record_round(t, u_err, u_prog, v_err, env_err);
        let switched = self.domain.current_domain() != domain_before;
        // 0be 四项④：繁杂度推进（消费到期预测 → 发出新预测 → 合成样本／
        // 基线／三条自校准阈值＋锁存）。
        let c = self.cplx.on_decision_round(t, self.t_hat, &self.channels);
        // 0bg S2：域切换 ⇒ 掩盖缺口（CoverageGap）触发沿**重武装**（用户
        // 裁决「触发沿一次、域切换重武装」）。
        if switched {
            self.coverage_gap_armed = true;
        }
        // 0bf ③（2026-09-22）：域迁移确认（等震荡结束后一次）→ 模型面提醒
        // （随报失配概率；只报一次由 take 消费语义保证）。
        let mut domain_notice_pushed = false;
        if let Some(conf) = self.domain.take_confirmed_migration() {
            domain_notice_pushed = true;
            let round = (conf.at_round > 0).then_some(conf.at_round);
            self.push_notice(RliNotice {
                kind: RliNoticeKind::MigrationConfirmed,
                t: conf.at_t,
                round,
                text: format!(
                    "域事件: 稳定确认 {}→{}@r{}（稳定 {} 轮；进{}回{}）；{}；\
                     err={:.1} slow={:.1} stall={:.1}{}",
                    conf.from.as_str(),
                    conf.to.as_str(),
                    conf.at_round,
                    conf.settle_rounds,
                    self.spike_entries,
                    self.spike_returns,
                    self.t_hat_trend_text(),
                    self.channel(ChannelKind::Err).u(),
                    self.channel(ChannelKind::Slow).u(),
                    self.channel(ChannelKind::Stall).u(),
                    self.recent_actions_text(),
                ),
                delivered: false,
            });
        }
        // **0cp D7（167 批用户裁定）**：spike 进入端即提醒——域瞬态偏移发生
        // 当刻报一次（与稳定确认合并为统一「域事件提醒」族，同一行格式）；
        // 回归端与 bootstrap 不提醒（回归只计数）。域机器不动；recli 实测
        // spike 进入 13 次／5,166s 已足稀疏，无阻尼常数（禁拟合）。切换轮与
        // 稳定确认轮互斥（确认要求连续 ≥3 轮无切换），此处顺序无冲突。
        // **169 批审查勘误**：回归端＝**凡异常域→Normal**——原枚举
        // 「Stuck/LowProgress→Normal」不完备：`label()` 四值两两可达，
        // Pressure→Normal（错误压力直接衰减回正常）属恢复而非进入，旧判定
        // 会把它计成进入并向模型面发「spike进入 pressure→normal」。
        if switched {
            let to = self.domain.current_domain();
            let bootstrap = domain_before == Domain::Start;
            let recovery = !bootstrap && to == Domain::Normal;
            if recovery {
                self.spike_returns = self.spike_returns.saturating_add(1);
            } else if !bootstrap {
                self.spike_entries = self.spike_entries.saturating_add(1);
                domain_notice_pushed = true;
                let round_now = self.domain.round();
                let round = (round_now > 0).then_some(round_now);
                self.push_notice(RliNotice {
                    kind: RliNoticeKind::DomainSpikeEntry,
                    t,
                    round,
                    text: format!(
                        "域事件: spike进入 {}→{}@r{}（进{}回{}）；{}；\
                         err={:.1} slow={:.1} stall={:.1}{}",
                        domain_before.as_str(),
                        to.as_str(),
                        round_now,
                        self.spike_entries,
                        self.spike_returns,
                        self.t_hat_trend_text(),
                        self.channel(ChannelKind::Err).u(),
                        self.channel(ChannelKind::Slow).u(),
                        self.channel(ChannelKind::Stall).u(),
                        self.recent_actions_text(),
                    ),
                    delivered: false,
                });
            }
        }
        // 0bg S2（2026-09-22，用户裁决「直接做掩盖缺口吧」）：掩盖缺口
        // （CoverageGap）——就绪 ∧ g>0 ∧ 越线 ⇒ **触发沿**报一次；**每轮
        // 最多一条域类提醒**（本轮已有域迁移确认／spike 提醒时让位——沿
        // 不被吞：条件持续成立即下一轮报）。
        if !domain_notice_pushed {
            self.maybe_push_coverage_gap(t);
        }
        // 0be 四项③：自适应参数轨迹（决策轮粒度；复算可核）。λ̂ 取 prog
        // 通道估计（未激活 = None——与真值 0 不混同）。
        let lambda_hat = self.channel(ChannelKind::Prog).lambda_hat();
        push_capped(
            &mut self.adapt_trace,
            RliAdaptTraceRow {
                round: self.domain.round(),
                t: quantize_state(t),
                lambda_hat: lambda_hat.map(quantize_state),
                c: c.map(quantize_state),
                rho_base: self.cplx.baseline().map(quantize_state),
                theta: self.cplx.theta.map(quantize_state),
            },
            RLI_ADAPT_TRACE_CAP,
        );
        // 采样点结算（0bf 起含持续越线观察；见 [`Self::note_sample_point`]）。
        self.note_sample_point(t);
    }

    /// 工具事件：间隔看门狗（stall）→ 注入（err/deny/prog/slow）→ 阈值评估
    /// → 锚点采样（事件级，2026-09-20）。与 1D 引擎同序：先推进（自由演化），
    /// 再注入，再检查。**0cp D1**：动作采样点（网格补点退役）。
    pub fn on_tool_event(&mut self, t: f64, event: ToolEvent) {
        let long_gap = matches!(
            self.last_tool_t,
            Some(prev) if (t - prev) > STALL_GAP_THRESHOLD_SECS
        );
        self.last_tool_t = Some(t);
        self.steps = self.steps.saturating_add(1);
        // 0cp D7：近期动作形态窗（域事件提醒行「近期动作」段；live-only）。
        self.recent_outcomes.push_back(event.outcome);
        while self.recent_outcomes.len() > RLI_RECENT_OUTCOMES_CAP {
            self.recent_outcomes.pop_front();
        }
        for ch in &mut self.channels {
            ch.advance(t);
        }
        // 0am P8（2026-10-03，S2 路由表）：分派统一走 router 纯函数
        // （`routing: None`＝旧四值语义，行为不变；生产喂入点恒带路由键）。
        // 零触发语义（P9）。
        let targets = crate::lif::router::stimulus_targets(&event);
        // 0am P8-b：注入同刻盖机械标签（成因段数据面；闭集词表）。
        if targets.err {
            self.channel_mut(ChannelKind::Err).inject(t, 1.0);
            self.stamp_label(ChannelKind::Err, "执行失败");
        }
        if targets.deny {
            self.channel_mut(ChannelKind::Deny).inject(t, 1.0);
            let dc = targets
                .deny_class
                .map(Self::deny_label)
                .unwrap_or("其他拒绝");
            self.stamp_label(ChannelKind::Deny, dc);
        }
        if targets.prog_set {
            self.channel_mut(ChannelKind::Prog).inject_set(t, 1.0);
            self.stamp_label(ChannelKind::Prog, "变更成功");
        }
        if targets.verify {
            self.channel_mut(ChannelKind::Verify).inject(t, 1.0);
            self.stamp_label(ChannelKind::Verify, "验证失败");
        }
        if let Some(w) = targets.slow {
            self.channel_mut(ChannelKind::Slow).inject(t, w);
            self.stamp_label(ChannelKind::Slow, "长时(非验证)");
        }
        if long_gap {
            self.channel_mut(ChannelKind::Stall).inject(t, 1.0);
            self.stamp_label(ChannelKind::Stall, "间隔失节律");
        }
        // 0am 改造四项①：自判域的成功门（与 LIF temporal 的 has_success 同
        // 口径——Start 仅在首次成功前出现）。
        self.domain.observe_tool_outcome(event.outcome);
        for ch in &mut self.channels {
            ch.check(t);
        }
        // 锚点序列采样（事件级；注入后取——快模态分量在注入当刻最大，
        // 决策轮粒度取不到，见 [`Self::sample_anchor_series`]）；0bf 起同点
        // 结算持续越线观察（[`Self::note_sample_point`]）。
        self.note_sample_point(t);
    }

    /// 0am P8（2026-10-03，S2 §3）：总线事件注入（Ctx/Infra 认领通道；
    /// 跨档过滤在 [`crate::lif::LifEngine::on_bus_event`] 已完成——本层
    /// 只注入）。总线事件**非动作采样点**（0cp D1 动作化口径：streak 窗
    /// 只认决策轮/工具事件/看门狗样），不调 [`Self::note_sample_point`]。
    pub fn on_bus_event(&mut self, t: f64, kind: ChannelKind, label: &'static str) {
        for ch in &mut self.channels {
            ch.advance(t);
        }
        self.channel_mut(kind).inject(t, 1.0);
        self.stamp_label(kind, label);
        for ch in &mut self.channels {
            ch.check(t);
        }
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
            // 0be 四项④：繁杂度状态（legacy 快照缺字段 = None ⇒ fresh）。
            complexity: Some(self.cplx.snapshot()),
            // 0be 四项③：自适应参数轨迹（已有行逐字段量化；legacy = 空）。
            adapt_trace: self.adapt_trace.iter().copied().collect(),
            // 0cp D2：动作采样锚（看门狗 ① 窗起点；legacy 缺字段 = None）。
            last_sample_t: self.last_sample_t.map(quantize_state),
            // 0bf ③：提醒队列（**0cp D4 起＝事件历史**；legacy = 空）。
            notices: self.notices.iter().cloned().collect(),
            sample_points: self.sample_points,
            coverage_gap_armed: self.coverage_gap_armed,
            // 0cp D2：看门狗武装标志（legacy 缺字段 = true——宁可多采一次）。
            watchdog_armed: self.watchdog_armed,
            // 0cp D6：T̂ 前值（趋势段；legacy = None＝无趋势）。
            t_hat_prev: self.t_hat_prev.map(quantize_state),
            // 0cp D7：域事件计数（legacy = 0）。
            spike_entries: self.spike_entries,
            spike_returns: self.spike_returns,
            // 0bh ①：投递面会计（legacy 缺字段 = 0）。
            notice_delivered_total: self.notice_delivered_total,
            notice_deferred_total: self.notice_deferred_total,
            notice_headroom_bytes: self.notice_headroom_bytes,
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
        // 0be 四项④：繁杂度状态续接；legacy／结构不符 = fresh（可选读数面
        // ——坏结构不拖垮通道与时间轴续接）。
        match &snapshot.complexity {
            Some(complexity) if self.cplx.restore(complexity) => {}
            _ => self.cplx = RliComplexity::new(),
        }
        // 0be 四项③：自适应参数轨迹续接（cap 截断；legacy = 空）。
        self.adapt_trace = snapshot
            .adapt_trace
            .iter()
            .rev()
            .take(RLI_ADAPT_TRACE_CAP)
            .rev()
            .copied()
            .collect();
        // 0bf ③④：采样锚、提醒队列与计数续接（legacy 缺字段 = None/空/0；
        // 队列 cap 收敛、未投递标志原样保留——跨 prompt 投递不重不漏）。
        self.last_sample_t = snapshot.last_sample_t.filter(|t| t.is_finite());
        self.notices = snapshot
            .notices
            .iter()
            .rev()
            .take(RLI_NOTICE_CAP)
            .rev()
            .cloned()
            .collect();
        self.sample_points = snapshot.sample_points;
        // 0bg S2：掩盖缺口重武装标志续接（legacy 缺字段 = true，见快照定义）。
        self.coverage_gap_armed = snapshot.coverage_gap_armed;
        // 0cp D2/D6/D7：看门狗武装、T̂ 前值与域事件计数续接（legacy = true/
        // None/0——宁可多采一次、趋势从下轮起算、计数从本会话已知部分起算）。
        self.watchdog_armed = snapshot.watchdog_armed;
        self.t_hat_prev = snapshot.t_hat_prev.filter(|v| v.is_finite());
        self.spike_entries = snapshot.spike_entries;
        self.spike_returns = snapshot.spike_returns;
        // 0bh ①：投递面会计续接（legacy 缺字段 = 0——投递率从本会话已知部分起算）。
        self.notice_delivered_total = snapshot.notice_delivered_total;
        self.notice_deferred_total = snapshot.notice_deferred_total;
        self.notice_headroom_bytes = snapshot.notice_headroom_bytes;
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
    /// 0bf ③（2026-09-22）：**未确认迁移候选**——域切换后新域稳定不足
    /// [`RLI_MIGRATION_SETTLE_ROUNDS`] 轮时为 `Some`（震荡期只更新候选，
    /// 不提醒）；稳定达成转 [`Self::confirmed_migration`]。
    pending_migration: Option<RliPendingMigration>,
    /// 0bf ③：已确认待消费的迁移（决策轮产出，由影子取出转模型面提醒；
    /// `take_confirmed_migration` 消费后清空）。
    confirmed_migration: Option<RliMigrationConfirmed>,
}

/// 0bf ③：未确认的域迁移候选（域切换即产生；稳定达成＝确认）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliPendingMigration {
    pub from: Domain,
    pub to: Domain,
    pub at_round: u64,
    pub at_t: f64,
}

/// 0bf ③：已确认的域迁移（等震荡结束——新域连续稳定
/// [`RLI_MIGRATION_SETTLE_ROUNDS`] 轮——后的定案事实；只报一次）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliMigrationConfirmed {
    pub from: Domain,
    pub to: Domain,
    pub at_round: u64,
    pub at_t: f64,
    /// 确认时的稳定轮数（= 自 `at_round` 起新域连续驻留轮数）。
    pub settle_rounds: u64,
}

/// 0bg S2（2026-09-22）：域覆盖读数（掩盖缺口判据＋随报字段的单一来源；
/// 由 [`RliDomainMachine::coverage_stats`] 推导——不单独存储，与域段/
/// 倾向同口径"由迁移点推导"）。
#[derive(Debug, Clone, PartialEq)]
pub struct RliCoverageStats {
    /// 未访问域（按域序 `normal < pressure < low_progress < stuck`）。
    pub uncovered: Vec<Domain>,
    /// 未访问域占比（分母＝四值域枚举；`Start` 不计）。
    pub g: f64,
    /// 已完成段数（相邻迁移点之间；末段＝当前段不计）。
    pub completed_segments: u64,
    /// 已完段累计驻留轮数（域失配/离开率的分母面）。
    pub completed_rounds: u64,
    /// 当前自判域。
    pub current_domain: Domain,
    /// 当前段驻留轮数（含入域轮）。
    pub current_dwell: u64,
    /// 该域（当前域）已完段驻留中位（无 = `None` ⇒ 退全域中位）。
    pub current_median: Option<u64>,
    /// 会话内全域已完段驻留中位（退档面）。
    pub fallback_median: Option<u64>,
    /// **域失配**（域模型的累计超期）：Σ max(0, 段驻留 − 该域已完段中位)
    /// （轮）——随触发一并报、不独立触发。
    pub overdue_rounds: u64,
    /// 超期段数（分子面读数）。
    pub overdue_segments: u64,
}

/// 中位（整数域；偶数取中间两值平均向下取整——与倾向的 f64 中位同序，
/// 整数面避免浮点噪声进读数）。
fn median_u64(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let n = values.len();
    Some(if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2
    })
}

/// 域转移倾向读数（0bf ②，2026-09-22）——**本会话纯经验统计**（零拟合；
/// 只给特征与域状态、不含动作建议）。由迁移 spike 历史推导：当前域已
/// 驻留轮数、本会话该域**完成段**的计数与累计驻留（段驻留＝相邻切换点
/// 轮差；legacy 缺轮号的段不可得、如实跳过），去向分布与逐轮离开率
/// （＝离开次数／完成段累计轮；样本不足 = `None`——不虚构率）。
#[derive(Debug, Clone, PartialEq)]
pub struct RliTransitionTendency {
    /// 当前自判域。
    pub domain: Domain,
    /// 当前段已驻留轮数（含入域轮）。
    pub current_dwell: u64,
    /// 本会话当前域的完成段数（≥0；样本少时读数即计数本身）。
    pub completed_episodes: u64,
    /// 完成段累计驻留轮数。
    pub completed_rounds: u64,
    /// 去向分布（次数降序、同数按域名稳定序）。
    pub directions: Vec<(Domain, u64)>,
    /// 逐轮离开率（离开次数／完成段累计轮；无完成段 = `None`）。
    pub per_round_rate: Option<f64>,
    /// 完成段驻留轮数中位（无 = `None`）。
    pub median_dwell: Option<u64>,
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

/// 域枚举稳定序（转移倾向的去向分布排序用；同数按此序，保证可复算）。
fn domain_rank(domain: Domain) -> u8 {
    match domain {
        Domain::Start => 0,
        Domain::Normal => 1,
        Domain::Pressure => 2,
        Domain::LowProgress => 3,
        Domain::Stuck => 4,
    }
}

/// 通道短名（0bf ③：模型面提醒与渲染文案用；与 [`ChannelKind`] 一一对应）。
pub fn channel_label(kind: ChannelKind) -> &'static str {
    match kind {
        ChannelKind::Err => "err",
        ChannelKind::Stall => "stall",
        ChannelKind::Slow => "slow",
        ChannelKind::Deny => "deny",
        ChannelKind::Prog => "prog",
        ChannelKind::Verify => "verify",
        ChannelKind::Ctx => "ctx",
        ChannelKind::Infra => "infra",
    }
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
            pending_migration: None,
            confirmed_migration: None,
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
            // 0bf ③（2026-09-22）：切换即置/替换**未确认候选**——震荡期
            // （再次切换）只更新候选、不提醒；稳定达成在下方确认。
            self.pending_migration = Some(RliPendingMigration {
                from: self.current,
                to: domain,
                at_round: self.round,
                at_t: t,
            });
            self.current = domain;
            self.entry_round = self.round;
        } else if let Some(pending) = self.pending_migration
            && pending.to == domain
            && self.round.saturating_sub(pending.at_round) + 1 >= RLI_MIGRATION_SETTLE_ROUNDS
        {
            // 0bf ③：等震荡结束——新域连续驻留 SETTLE 轮（含切换轮）后
            // 确认迁移（只产出一次；由影子取出转模型面提醒）。
            self.confirmed_migration = Some(RliMigrationConfirmed {
                from: pending.from,
                to: pending.to,
                at_round: pending.at_round,
                at_t: pending.at_t,
                settle_rounds: self.round.saturating_sub(pending.at_round) + 1,
            });
            self.pending_migration = None;
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

    /// 0bg S2（2026-09-22）：域覆盖读数——掩盖缺口（`CoverageGap`）判据与
    /// 随报字段的**单一来源**（触发点与测试共用，避免两处口径漂移）。
    ///
    /// 口径（S1 勘定稿）：
    /// - 分母＝四值域枚举（`normal/pressure/low_progress/stuck`；`Start`
    ///   仅首采样前哨兵、不计）；`g = 未访问域占比`；
    /// - 已完成段＝相邻迁移点（含轮号）之间的段（驻留＝轮差）；末段＝当前段
    ///   （不并入"已完段"）；
    /// - 「该域已完段中位」不足（该域无已完段）⇒ 退会话内全域中位；
    /// - **域失配**＝Σ max(0, 段驻留 − 该域已完段中位)（轮），并给超期段数
    ///   ——域模型的累计超期（随触发一并报；不独立触发）。
    pub fn coverage_stats(&self) -> RliCoverageStats {
        let mut visited: Vec<Domain> = Vec::new();
        let mut completed: Vec<(Domain, u64)> = Vec::new();
        for w in self.spikes.windows(2) {
            let (a, b) = (w[0], w[1]);
            if !visited.contains(&a.domain) {
                visited.push(a.domain);
            }
            if let (Some(r1), Some(r2)) = (a.round, b.round)
                && r2 >= r1
            {
                completed.push((a.domain, r2 - r1));
            }
        }
        if let Some(last) = self.spikes.last()
            && !visited.contains(&last.domain)
        {
            visited.push(last.domain);
        }
        if self.round > 0 && !visited.contains(&self.current) {
            visited.push(self.current);
        }
        visited.retain(|d| *d != Domain::Start);
        let uncovered: Vec<Domain> = [
            Domain::Normal,
            Domain::Pressure,
            Domain::LowProgress,
            Domain::Stuck,
        ]
        .into_iter()
        .filter(|d| !visited.contains(d))
        .collect();
        let g = uncovered.len() as f64 / 4.0;
        let completed_rounds: u64 = completed.iter().map(|(_, d)| d).sum();
        let mut fallback_values: Vec<u64> = completed.iter().map(|(_, d)| *d).collect();
        let fallback_median = median_u64(&mut fallback_values);
        let mut current_values: Vec<u64> = completed
            .iter()
            .filter(|(d, _)| *d == self.current)
            .map(|(_, d)| *d)
            .collect();
        let current_median = median_u64(&mut current_values);
        let mut overdue_rounds: u64 = 0;
        let mut overdue_segments: u64 = 0;
        for (domain, dwell) in &completed {
            let mut values: Vec<u64> = completed
                .iter()
                .filter(|(d, _)| d == domain)
                .map(|(_, d)| *d)
                .collect();
            if let Some(median) = median_u64(&mut values)
                && *dwell > median
            {
                overdue_rounds = overdue_rounds.saturating_add(dwell - median);
                overdue_segments = overdue_segments.saturating_add(1);
            }
        }
        RliCoverageStats {
            uncovered,
            g,
            completed_segments: completed.len() as u64,
            completed_rounds,
            current_domain: self.current,
            current_dwell: self.round.saturating_sub(self.entry_round) + 1,
            current_median,
            fallback_median,
            overdue_rounds,
            overdue_segments,
        }
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

    /// 0bf ③（2026-09-22）：取出已确认的域迁移（决策轮产出一次；消费即
    /// 清空——模型面「域迁移完成」提醒的只报一次语义由本方法保证）。
    pub fn take_confirmed_migration(&mut self) -> Option<RliMigrationConfirmed> {
        self.confirmed_migration.take()
    }

    /// 未确认迁移候选（读数/回放面；`None` = 无候选）。
    pub fn pending_migration(&self) -> Option<RliPendingMigration> {
        self.pending_migration
    }

    /// 0bf ②（2026-09-22）：**域转移倾向**读数——本会话纯经验统计
    /// （零拟合；只给特征与域状态，不含动作建议）。口径见
    /// [`RliTransitionTendency`]。
    pub fn transition_tendency(&self) -> RliTransitionTendency {
        let domain = self.current;
        let current_dwell = self.round.saturating_sub(self.entry_round) + 1;
        let mut completed_episodes: u64 = 0;
        let mut completed_rounds: u64 = 0;
        let mut dwells: Vec<u64> = Vec::new();
        let mut directions: Vec<(Domain, u64)> = Vec::new();
        for pair in self.spikes.windows(2) {
            let (from, to) = (pair[0], pair[1]);
            if from.domain != domain {
                continue;
            }
            // 段驻留 = 相邻切换点轮差；legacy 缺轮号 = 不可得（如实跳过，
            // 不落假 0——FR-7 口径）。
            let (Some(a), Some(b)) = (from.round, to.round) else {
                continue;
            };
            completed_episodes = completed_episodes.saturating_add(1);
            completed_rounds = completed_rounds.saturating_add(b.saturating_sub(a));
            dwells.push(b.saturating_sub(a));
            if let Some(entry) = directions.iter_mut().find(|(d, _)| *d == to.domain) {
                entry.1 = entry.1.saturating_add(1);
            } else {
                directions.push((to.domain, 1));
            }
        }
        // 去向分布：次数降序；同数按域枚举序（稳定、可复算）。
        directions.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| domain_rank(a.0).cmp(&domain_rank(b.0)))
        });
        let per_round_rate =
            (completed_rounds > 0).then(|| completed_episodes as f64 / completed_rounds as f64);
        let median_dwell = if dwells.is_empty() {
            None
        } else {
            dwells.sort_unstable();
            Some(dwells[dwells.len() / 2])
        };
        RliTransitionTendency {
            domain,
            current_dwell,
            completed_episodes,
            completed_rounds,
            directions,
            per_round_rate,
            median_dwell,
        }
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
            pending_migration: self.pending_migration,
            confirmed_migration: self.confirmed_migration,
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
        // 0bf ③：未确认/待消费迁移随快照续接（legacy 缺字段 = 无）。
        self.pending_migration = snapshot.pending_migration;
        self.confirmed_migration = snapshot.confirmed_migration;
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
    /// 0bf ③（2026-09-22）：未确认迁移候选（legacy 缺字段 = 无候选）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_migration: Option<RliPendingMigration>,
    /// 0bf ③：已确认待消费的迁移（同上）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirmed_migration: Option<RliMigrationConfirmed>,
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
    /// 0be 四项①：成功到达间隔 EMA（`None` ＝未记录/未激活——legacy
    /// 快照缺字段落 `None`，与「尚无间隔样本」同义；FR-7 口径下
    /// 不可得与估计值不混同）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lambda_gap_ema: Option<f64>,
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub lambda_gap_samples: u64,
    /// 0bf ③（2026-09-22）：持续越线观察（连续计数＋段起点；legacy 缺
    /// 字段 ⇒ 0/None，与「不在连续段上」同义）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub streak: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streak_start_t: Option<f64>,
}

fn is_zero_u64(v: &u64) -> bool {
    *v == 0
}

/// serde 缺省（0bg S2）：`coverage_gap_armed` legacy 缺字段 = `true`
/// （宁可多报一次也不吞；域切换会重武装）。
fn is_true_default() -> bool {
    true
}

/// `skip_serializing_if`（serde 传字段引用）：`true` 不落盘（缺省即真）。
fn is_true(v: &bool) -> bool {
    *v
}

/// 繁杂度快照（0be 四项④；随 [`RliShadowSnapshot::complexity`] 入会话侧车。
/// 缺字段 = legacy ⇒ fresh 状态；数组尺寸校验失败 = fresh）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RliComplexitySnapshot {
    /// 四条复极点通道的 ρ 窗（顺序 = [`RLI_CPLX_KINDS`]）。
    pub channel_windows: Vec<Vec<f64>>,
    /// 各通道未决算预测（配对件；跨 prompt 同刻近似不重置）。
    pub pending: Vec<Vec<RliPendingPrediction>>,
    /// 合成样本窗（ρ_cur 流）。
    pub combined: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline: Option<f64>,
    pub samples: u64,
    /// 三条自校准阈值（顺序 = [`RLI_CPLX_TIERS`]）。
    pub theta: Vec<f64>,
    /// 越线锁存（顺序同上）。
    pub latched: Vec<bool>,
}

/// 自适应参数轨迹单行（0be 四项③，2026-09-21）——决策轮粒度、随侧车
/// 持久（**复算可核**：轨迹与在线自适应同源同序，可对照重放复算）。
/// `lambda_hat`／`c`／`rho_base` 为 `Option`（不可得与真值 0 不混同——
/// FR-7 口径：λ̂ 未激活、繁杂度未就绪都不落假 0）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RliAdaptTraceRow {
    /// 会话相对决策轮（与域行同轴）。
    pub round: u64,
    /// 会话相对墙钟秒。
    pub t: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lambda_hat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub c: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rho_base: Option<f64>,
    /// 三条自校准阈值（顺序 = [`RLI_CPLX_TIERS`]）。
    pub theta: [f64; RLI_CPLX_TIERS.len()],
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
    /// 0be 四项④（2026-09-21）：繁杂度状态（缺字段 = legacy ⇒ fresh）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complexity: Option<RliComplexitySnapshot>,
    /// 0be 四项③（2026-09-21）：自适应参数轨迹（缺字段 = legacy ⇒ 空）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapt_trace: Vec<RliAdaptTraceRow>,
    /// 0cp D2：动作采样锚（看门狗 ① 窗起点；legacy = None）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sample_t: Option<f64>,
    /// 0bf ③：模型面提醒队列（含已投递标志；**0cp D4 起＝事件历史**；legacy = 空）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<RliNotice>,
    /// 0bf ④：采样点总数（决策轮＋工具事件＋看门狗单点；legacy = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub sample_points: u64,
    /// 0bg S2（2026-09-22）：掩盖缺口触发沿重武装标志（legacy 缺字段 =
    /// `true`——宁可多报一次也不吞；域切换重武装）。
    #[serde(default = "is_true_default", skip_serializing_if = "is_true")]
    pub coverage_gap_armed: bool,
    /// 0cp D2：看门狗武装标志（legacy 缺字段 = `true`——宁可多采一次）。
    #[serde(default = "is_true_default", skip_serializing_if = "is_true")]
    pub watchdog_armed: bool,
    /// 0cp D6：T̂ 前值（趋势段数据面；legacy = None＝无趋势）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub t_hat_prev: Option<f64>,
    /// 0cp D7：域 spike 进入端累计（legacy = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub spike_entries: u64,
    /// 0cp D7：域 spike 回归端累计（legacy = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub spike_returns: u64,
    /// 0bh ①（2026-09-22）：提醒投递总数（投递率分子；legacy 缺字段 = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub notice_delivered_total: u64,
    /// 0bh ①：预算受阻暂存数（legacy 缺字段 = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub notice_deferred_total: u64,
    /// 0bh ①：最近头段余量（字节；legacy 缺字段 = 0）。
    #[serde(default, skip_serializing_if = "is_zero_u64")]
    pub notice_headroom_bytes: u64,
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

    /// 0bg S2（2026-09-22，用户裁决「直接做掩盖缺口吧」）：掩盖缺口触发——
    /// 就绪（已完段 ≥3）∧ 未访域 ∧ 越线（当前段驻留 ≥ 该域已完段中位）⇒
    /// 触发沿报一次；关闭后不重复；重武装（域切换语义）后可再报。
    #[test]
    fn coverage_gap_fires_on_edge_and_respects_budget() {
        let mut shadow = RliShadow::new();
        shadow.domain.has_success = true;
        shadow.domain.round = 12;
        shadow.domain.current = Domain::Normal;
        shadow.domain.entry_round = 7;
        shadow.domain.spikes = vec![
            RliDomainSpike {
                t: 0.0,
                domain: Domain::Normal,
                round: Some(1),
            },
            RliDomainSpike {
                t: 30.0,
                domain: Domain::Stuck,
                round: Some(4),
            },
            RliDomainSpike {
                t: 50.0,
                domain: Domain::Pressure,
                round: Some(6),
            },
            RliDomainSpike {
                t: 60.0,
                domain: Domain::Normal,
                round: Some(7),
            },
        ];
        let stats = shadow.coverage_stats();
        assert_eq!(stats.completed_segments, 3, "{stats:?}");
        assert_eq!(stats.uncovered, vec![Domain::LowProgress]);
        assert!((stats.g - 0.25).abs() < 1e-9, "g={}", stats.g);
        assert_eq!(stats.current_dwell, 6, "r7..=r12");
        assert_eq!(stats.current_median, Some(3), "Normal 段 [r1,r4)=3");
        assert_eq!(stats.overdue_rounds, 0, "{stats:?}");

        assert!(shadow.maybe_push_coverage_gap(70.0), "沿上应触发");
        assert!(!shadow.coverage_gap_armed, "触发后沿关闭（域切换重武装）");
        assert!(!shadow.maybe_push_coverage_gap(71.0), "沿关闭后不重复");
        let notice = shadow.notices().last().copied().expect("notice pushed");
        assert_eq!(notice.kind, RliNoticeKind::CoverageGap);
        assert!(notice.text.starts_with("掩盖缺口: "), "{}", notice.text);
        assert!(notice.text.contains("未访 low_progress"), "{}", notice.text);
        assert!(notice.text.contains("g=0.25"), "{}", notice.text);
        assert!(notice.text.contains("驻留6/中位3"), "{}", notice.text);
        assert!(notice.text.contains("域失配0轮"), "{}", notice.text);
        // 0cp D6：行内自带 T̂ 参数含义注解＋趋势。
        assert!(
            notice.text.contains(RLI_T_HAT_ANNOTATION),
            "{}",
            notice.text
        );
        assert!(
            notice.text.len() <= RLI_NOTICE_TEXT_BUDGET,
            "本体＋注解 {}B 超预算: {}",
            notice.text.len(),
            notice.text
        );

        // 域切换重武装语义（on_decision_round 中 switched 分支直接置位）。
        shadow.coverage_gap_armed = true;
        assert!(shadow.maybe_push_coverage_gap(80.0), "重武装后可再报");
    }

    /// 0bg S2：提醒**本体＋注解** ≤ [`RLI_NOTICE_TEXT_BUDGET`]（最坏形态；
    /// 注解为固定模板、单一源 [`rli_notice_annotation`]）。0cp D6/D7：
    /// 行内 T̂ 注解＋趋势／域事件统一行格式一并入钉（四类触发源全覆盖）。
    #[test]
    fn notice_texts_stay_within_budget() {
        let t_hat = format!("{}：8.94→31.89 ↑", RLI_T_HAT_ANNOTATION);
        let worst = [
            (
                RliNoticeKind::StreakCrossed,
                format!("持续越线: err×3 stall×3 slow×3 deny×3（失配概率 0.12）；{t_hat}"),
            ),
            (
                RliNoticeKind::MigrationConfirmed,
                format!(
                    "域事件: 稳定确认 low_progress→normal@r123（稳定 12 轮；进99回99）；\
                     {t_hat}；err=9.9 slow=9.9 stall=9.9；近5: 成5误0拒0"
                ),
            ),
            (
                RliNoticeKind::DomainSpikeEntry,
                format!(
                    "域事件: spike进入 normal→low_progress@r123（进99回99）；\
                     {t_hat}；err=9.9 slow=9.9 stall=9.9；近5: 成5误0拒0"
                ),
            ),
            (
                RliNoticeKind::CoverageGap,
                format!(
                    "掩盖缺口: 未访 pressure/low_progress…；g=0.50；驻留99/中位99；\
                     域失配99轮；{t_hat}"
                ),
            ),
        ];
        for (kind, body) in &worst {
            let total = body.len() + rli_notice_annotation(*kind).len();
            assert!(
                total <= RLI_NOTICE_TEXT_BUDGET,
                "{kind:?}: 本体＋注解 {total}B 超预算: {body}"
            );
        }
        // D6：四类行均自带 T̂ 参数含义注解（模型免回查）；无成因段。
        for (kind, body) in &worst {
            assert!(
                body.contains(RLI_T_HAT_ANNOTATION),
                "{kind:?}: 缺 T̂ 注解: {body}"
            );
        }
    }

    /// 0bg S2：面头符号表——缩写族单行给全（渲染面与工具描述引用同一常量；
    /// 用户裁决 (c) 的「面头固定符号表一行」）。
    #[test]
    fn symbol_legend_names_the_abbreviation_family() {
        for needle in [
            "u=", "v=", "pred=", "p1=", "E=", "r=", "θ=", "λ̂=", "ρ=", "c=", "T̂",
        ] {
            assert!(
                RLI_SYMBOL_LEGEND.contains(needle),
                "符号表缺 {needle}: {RLI_SYMBOL_LEGEND}"
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
        // 0be 四项②：短视锚点独立化后名集合 11 → 13（pred1_err/pred1_prog）；
        // 0am P8：新三通道 u/v 锚点 13 → 19（feature 面最小可读集）。
        assert_eq!(RliShadow::known_feature_names().len(), 19);
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

    /// 0be 四项①：λ̂ 需要至少 2 个成功间隔才激活；间隔先钳制到语义上下界
    /// 再进 EMA（估计式——只有矩统计，无损失、无梯度）。
    #[test]
    fn lambda_hat_requires_min_gaps_and_clamps() {
        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let mut ch = RliChannel::new(ChannelKind::Prog, omega);
        ch.inject_set(10.0, 1.0);
        assert_eq!(ch.lambda_hat(), None, "no gap yet");
        ch.inject_set(20.0, 1.0);
        assert_eq!(ch.lambda_hat(), None, "one gap < MIN_GAPS");
        ch.inject_set(30.0, 1.0);
        let lambda = ch.lambda_hat().expect("two gaps activate");
        assert!((lambda - 0.1).abs() < 1e-9, "λ̂ = 1/gap (got {lambda})");
        // 0.1 s 的间隔按 0.5 s 下界记账：λ̂ 上移但不越上界。
        ch.inject_set(30.1, 1.0);
        let clamped = ch.lambda_hat().expect("still active");
        assert!(
            clamped > lambda,
            "tight gap raises the rate toward the clamp"
        );
        assert!(clamped <= 1.0 / RLI_LAMBDA_GAP_MIN_SECS + 1e-9);
    }

    /// 0be 四项①退化路径：λ̂ 未激活时 `prog` 前推**严格**等于自由衰减；
    /// 激活后预测上抬（期望注入项为正）。
    #[test]
    fn prog_prediction_degenerates_to_free_decay_without_lambda() {
        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let mut ch = RliChannel::new(ChannelKind::Prog, omega);
        ch.inject_set(0.0, 1.0);
        let t_hat = 8.0;
        let free = ch.free_evolution_at(RLI_PREDICTION_STEPS * t_hat).0;
        assert_eq!(
            ch.prediction(10.0 * t_hat / RLI_PREDICTION_STEPS),
            free,
            "without λ̂ the closed form is the free decay (strict)"
        );
        ch.inject_set(20.0, 1.0);
        ch.inject_set(40.0, 1.0);
        assert!(ch.lambda_hat().is_some());
        let uplifted = ch.prediction(10.0 * t_hat / RLI_PREDICTION_STEPS);
        assert!(
            uplifted > ch.free_evolution_at(RLI_PREDICTION_STEPS * t_hat).0,
            "expected-injection term lifts the forecast ({uplifted})"
        );
    }

    /// 0be 四项①：预测对 λ̂ 单调不减（同状态、同 horizon）。
    #[test]
    fn prog_prediction_is_monotone_in_arrival_rate() {
        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let mut sparse = RliChannel::new(ChannelKind::Prog, omega);
        let mut dense = RliChannel::new(ChannelKind::Prog, omega);
        for ch in [&mut sparse, &mut dense] {
            ch.inject_set(0.0, 1.0);
            ch.advance(3.0);
        }
        sparse.lambda_gap_ema = Some(600.0);
        sparse.lambda_gap_samples = 2;
        dense.lambda_gap_ema = Some(1.0);
        dense.lambda_gap_samples = 2;
        let p_sparse = sparse.prediction_at(RLI_PREDICTION_STEPS, 8.0);
        let p_dense = dense.prediction_at(RLI_PREDICTION_STEPS, 8.0);
        let free = sparse.free_evolution_at(RLI_PREDICTION_STEPS * 8.0).0;
        assert!(p_sparse >= free - 1e-12, "sparse ≥ free");
        assert!(p_dense > p_sparse, "denser arrivals ⇒ higher forecast");
    }

    /// 0be 四项①：闭式期望注入项与数值积分对拍（同一模型、同一状态）。
    #[test]
    fn prog_prediction_matches_numeric_injection_integral() {
        let omega = TAU / (RLI_PROG_PERIOD_ROUNDS * 8.0);
        let mut ch = RliChannel::new(ChannelKind::Prog, omega);
        ch.inject_set(0.0, 1.0);
        ch.inject_set(12.0, 1.0);
        ch.lambda_gap_ema = Some(12.0);
        ch.lambda_gap_samples = 2;
        let t_hat = 8.0;
        let tau = RLI_PREDICTION_STEPS * t_hat;
        let lambda = 1.0 / 12.0;
        let free = ch.free_evolution_at(tau).0;
        let mut probe = ch.clone();
        probe.u = 1.0;
        let steps = 4_000usize;
        let h = tau / steps as f64;
        let mut acc = 0.0;
        for i in 0..=steps {
            let s = i as f64 * h;
            let w = if i == 0 || i == steps { 0.5 } else { 1.0 };
            acc += w * libm::exp(-lambda * s) * probe.free_evolution_at(s).0;
        }
        let numeric = libm::exp(-lambda * tau) * free + lambda * acc * h;
        let closed = ch.prediction_at(RLI_PREDICTION_STEPS, t_hat);
        assert!(
            (closed - numeric).abs() < 1e-4,
            "closed {closed} vs numeric {numeric}"
        );
    }

    /// 0be 四项②：分通道 horizon 表钉住；短视锚点＝`1·T̂` 独立档。
    #[test]
    fn horizon_table_and_short_anchor_pin() {
        // 0bg S2 标定批（2026-09-22）：Slow 1→5、Deny 30→10（0bf 探针 skill
        // 表峰档：Slow h5 +0.317 vs h1 +0.222；Deny h10 +0.638 vs h30 +0.571）；
        // Err/Stall/Prog 保持（峰档或弱档）。
        assert_eq!(rli_horizon_steps(ChannelKind::Slow), 5.0);
        assert_eq!(rli_horizon_steps(ChannelKind::Stall), 2.0);
        assert_eq!(rli_horizon_steps(ChannelKind::Err), RLI_PREDICTION_STEPS);
        assert_eq!(rli_horizon_steps(ChannelKind::Deny), 10.0);
        assert_eq!(rli_horizon_steps(ChannelKind::Prog), RLI_PREDICTION_STEPS);

        let mut err = RliChannel::new(ChannelKind::Err, err_omega());
        err.inject(0.0, 1.0);
        let t_hat = 8.0;
        assert_eq!(
            err.prediction_at_horizon(t_hat),
            err.prediction_at(RLI_PREDICTION_STEPS, t_hat)
        );
        assert_eq!(err.prediction_short(t_hat), err.prediction_at(1.0, t_hat));
        assert!(
            (err.prediction_at_horizon(t_hat) - err.prediction_short(t_hat)).abs() > 1e-9,
            "Err keeps the short anchor independent of the channel horizon"
        );

        let mut slow = RliChannel::new(ChannelKind::Slow, TAU / RLI_SLOW_PERIOD_SECS);
        slow.inject(0.0, 1.0);
        // 0bg S2 标定批：Slow 升到 h=5 后与短视锚点（h=1）**分列**（不再恒等）。
        assert_eq!(
            slow.prediction_at_horizon(t_hat),
            slow.prediction_at(5.0, t_hat),
            "Slow h=5（标定批）"
        );
        assert!(
            (slow.prediction_at_horizon(t_hat) - slow.prediction_short(t_hat)).abs() > 1e-9,
            "Slow 分通道 horizon 与短视锚点分列（标定批）"
        );

        // 非法 horizon / T̂ ⇒ 保守落回当前水平（不虚构前推）。
        assert_eq!(err.prediction_at(0.0, t_hat), err.u());
        assert_eq!(err.prediction_at(f64::NAN, t_hat), err.u());
        assert_eq!(err.prediction_at(RLI_PREDICTION_STEPS, -1.0), err.u());
    }

    /// 0be 四项④：繁杂度就绪门槛、c 计算、锁存与读数面（同模块测试可触
    /// 内部状态——直置基线与窗）。
    #[test]
    fn complexity_requires_baseline_then_latches_and_reads() {
        let mut shadow = RliShadow::new();
        assert!(!shadow.complexity().ready(), "no baseline initially");
        {
            let cplx = &mut shadow.cplx;
            cplx.combined = VecDeque::from(vec![2.0; RLI_CPLX_BASELINE_SAMPLES]);
            cplx.baseline = Some(1.0);
            cplx.samples = RLI_CPLX_BASELINE_SAMPLES as u64;
            cplx.theta = [1.5, 2.5, 3.5];
        }
        shadow.on_decision_round(10.0, 8.0);
        assert!(shadow.complexity().ready());
        assert_eq!(shadow.complexity().current_c(), Some(2.0));
        assert!(shadow.complexity().latched(0), "c=2.0 ≥ θ85=1.5 ∧ c>1");
        assert!(!shadow.complexity().latched(1), "c < θ95");
        assert!(!shadow.complexity().latched(2), "c < θ99");
        // θ 同法：越线档上移、未越线档下移。
        assert!(shadow.complexity().theta(0) > 1.5);
        assert!(shadow.complexity().theta(1) < 2.5);
        let reading = shadow.complexity_reading();
        assert_eq!(reading.c, Some(2.0));
        assert_eq!(reading.latched, [true, false, false]);
        assert!(reading.ready);
    }

    /// 0be 四项③④：繁杂度与 λ̂ 估计器状态随影子快照持久；legacy 快照
    /// （无字段）＝可选面 fresh、通道与 λ̂ 照常续接。
    #[test]
    fn complexity_and_lambda_state_ride_the_shadow_snapshot() {
        let events = synthetic_events(0x0BEE_0001, 80);
        let mut a = RliShadow::new();
        feed(&mut a, &events);
        let snap = a.snapshot();
        assert!(snap.complexity.is_some(), "complexity rides the snapshot");
        assert!(
            !snap.adapt_trace.is_empty(),
            "adapt trace rides the snapshot"
        );
        let lambda = a.channel(ChannelKind::Prog).lambda_hat();
        assert!(lambda.is_some(), "synthetic successes activate λ̂");

        let mut b = RliShadow::new();
        assert!(b.restore(&snap));
        assert_eq!(b.complexity().samples(), a.complexity().samples());
        assert_eq!(b.complexity().ready(), a.complexity().ready());
        assert_eq!(b.adapt_trace().len(), a.adapt_trace().len());
        let lambda_b = b.channel(ChannelKind::Prog).lambda_hat();
        assert!(
            (lambda_b.unwrap() - lambda.unwrap()).abs() < 1e-3,
            "λ̂ estimate survives the (3-decimal) snapshot"
        );

        let mut legacy = snap.clone();
        legacy.complexity = None;
        legacy.adapt_trace.clear();
        let mut c = RliShadow::new();
        assert!(c.restore(&legacy));
        assert!(!c.complexity().ready(), "legacy ⇒ fresh complexity face");
        assert_eq!(c.complexity().samples(), 0);
        assert!(c.adapt_trace().is_empty());
        let lambda_c = c.channel(ChannelKind::Prog).lambda_hat();
        assert!(
            (lambda_c.unwrap() - lambda.unwrap()).abs() < 1e-3,
            "λ̂ estimator restores even on legacy complexity"
        );
    }

    /// 0be 四项③：轨迹 cap 128、保留最近行、未激活/未就绪落 `None`
    /// （FR-7：不可得与真值 0 不混同）。
    #[test]
    fn adapt_trace_caps_and_marks_inactive_states() {
        let mut shadow = RliShadow::new();
        for i in 0..200u64 {
            shadow.on_decision_round(i as f64 * 10.0, 8.0);
        }
        let trace = shadow.adapt_trace();
        assert_eq!(trace.len(), RLI_ADAPT_TRACE_CAP, "cap 128, oldest dropped");
        assert_eq!(trace.first().map(|r| r.round), Some(200 - 128 + 1));
        assert_eq!(trace.last().map(|r| r.round), Some(200));
        assert!(trace.iter().all(|r| r.lambda_hat.is_none()), "no successes");
        assert!(trace.iter().all(|r| r.c.is_none()), "no samples yet");
        assert!(trace.iter().all(|r| r.rho_base.is_none()));
    }

    /// 0be 四项②：短视锚点进入锚点序列（新名 `pred1_*` 事件级采样）。
    #[test]
    fn short_anchor_series_samples_under_new_names() {
        let mut shadow = RliShadow::new();
        shadow.on_decision_round(1.0, 8.0);
        assert_eq!(shadow.feature("pred1_err", 20).len(), 1);
        assert_eq!(shadow.feature("pred1_prog", 20).len(), 1);
        assert!(
            shadow.feature("env_prog", 20).is_empty(),
            "withdrawn name keeps sampling nothing"
        );
    }

    /// 0cp D1＋D2（2026-10-03）：动作采样点纪律——决策轮／工具事件各自
    /// 恰产一个采样（无网格补点：单条长命令执行段内零中间采样，同一命令
    /// 不再可能独自凑满 k）；看门狗单点——窗内无动作样且无活动 ≥181s 触发
    /// **恰好 1** 个时间采样后休眠，动作样重武装；有活动的等长窗不触发；
    /// 看门狗样不盖动作锚、不推进 steps。
    #[test]
    fn action_samples_only_and_watchdog_single_shot() {
        let mut shadow = RliShadow::new();
        shadow.on_decision_round(1.0, 8.0);
        assert_eq!(shadow.sample_points(), 1);
        // D1：100s 空档内零补点（原网格批在此产出 9 个补点样）。
        shadow.on_decision_round(101.0, 8.0);
        assert_eq!(shadow.sample_points(), 2, "网格退役：空档零补点");
        // 窗锚：决策轮 101.0 盖动作锚；181s 内（≤180.9）不触发。
        assert!(!shadow.watchdog_due(101.0 + 180.0, 181.0), "①未到窗不触发");
        assert!(
            !shadow.watchdog_due(101.0 + 200.0, 60.0),
            "②有活动（idle < 181s）不触发"
        );
        // 三同时：Δt ≥ 181 ∧ idle ≥ 181 ⇒ 恰一个时间采样。
        let now = 101.0 + RLI_WATCHDOG_SAMPLE_SECS;
        assert!(shadow.watchdog_due(now, RLI_WATCHDOG_SAMPLE_SECS));
        assert!(shadow.on_watchdog_tick(now, RLI_WATCHDOG_SAMPLE_SECS));
        assert_eq!(shadow.sample_points(), 3, "看门狗恰产一个采样");
        // 休眠：同刻续窗（动作样未增）不再触发——一次停滞至多一个样。
        assert!(!shadow.watchdog_due(now + 300.0, 300.0), "触发后休眠");
        assert!(!shadow.on_watchdog_tick(now + 300.0, 300.0));
        // 动作采样点重武装（工具事件），随后看门狗可再触发。
        shadow.on_tool_event(now + 310.0, ToolEvent::success(Some(10)));
        assert_eq!(shadow.sample_points(), 4, "动作样本身照常结算");
        assert!(
            shadow.watchdog_due(now + 310.0 + RLI_WATCHDOG_SAMPLE_SECS, 400.0),
            "动作样重武装看门狗"
        );
        // 窗锚缺失（无任何动作样的 fresh 影子）＝保守不触发。
        let fresh = RliShadow::new();
        assert!(
            !fresh.watchdog_due(RLI_WATCHDOG_SAMPLE_SECS, RLI_WATCHDOG_SAMPLE_SECS),
            "无动作锚不虚构窗起点"
        );
        // steps 分母口径不变：看门狗样不计步（决策轮 2 ＋ 工具事件 1）。
        assert_eq!(shadow.steps(), 3);
    }

    /// 0bf ③（2026-09-22）＋0cp D3（k 5→3）：持续越线（压力四通道、连续
    /// 动作样计数）→ 恰在第 3 个动作采样点产**一条**提醒（行内自带 T̂ 注解
    /// ＋趋势——D6；随报失配概率；未就绪 = "—"）；投递标记可消费
    /// （**0cp D4** `take_pending_for_push` 排空，装配即置位）。
    #[test]
    fn streak_crossing_pushes_single_notice_and_marks_delivered() {
        let mut shadow = RliShadow::new();
        // 连续错误工具事件：err 通道每次 +1.0（θ 随命中自适应上抬，首样后
        // θ>u 归零一段，故多给一个事件余量）——u≥θ 连续 3 个动作采样点触发
        // **一条**提醒（触发后继续计数不再重复触发）。
        for i in 0..5u32 {
            shadow.on_tool_event(10.0 + i as f64, ToolEvent::error(Some(900)));
        }
        let notices = shadow.notices();
        assert_eq!(notices.len(), 1, "exactly one notice at k=3");
        assert_eq!(notices[0].kind, RliNoticeKind::StreakCrossed);
        assert!(
            notices[0].text.contains("持续越线: err×3"),
            "{}",
            notices[0].text
        );
        assert!(notices[0].text.contains("失配概率"), "{}", notices[0].text);
        // 0cp D6：行内自带 T̂ 参数含义注解（无成因）。
        assert!(
            notices[0].text.contains(RLI_T_HAT_ANNOTATION),
            "{}",
            notices[0].text
        );
        assert!(
            notices[0].text.len() <= RLI_NOTICE_TEXT_BUDGET,
            "{}B 超预算: {}",
            notices[0].text.len(),
            notices[0].text
        );
        // 0cp D4：直投取走即置位（队列保留为事件历史）。
        assert_eq!(shadow.pending_notices().len(), 1);
        let taken = shadow.take_pending_for_push();
        assert_eq!(taken.len(), 1);
        assert!(shadow.pending_notices().is_empty());
        assert!(shadow.notices().len() == 1, "队列留事件历史");
        assert!(shadow.notices()[0].delivered);
        assert_eq!(shadow.take_pending_for_push(), Vec::<RliNotice>::new());
        let (delivered, _deferred, _headroom) = shadow.notice_delivery_stats();
        assert_eq!(delivered, 1, "投递率分子＝附注装配数");
    }

    /// 0bf ②③（2026-09-22）：域迁移**确认**（新域稳定 3 轮后一次）＋转移
    /// 倾向读数（完成段数／累计轮／去向分布；只给特征）。
    #[test]
    fn migration_confirmation_and_transition_tendency() {
        let mut shadow = RliShadow::new();
        // 首个成功（离开 start）→ 错误风暴推 err 通道过 2.0（pressure）。
        shadow.on_tool_event(1.0, ToolEvent::success(Some(120)));
        let mut t = 2.0;
        for _ in 0..3u32 {
            shadow.on_tool_event(t, ToolEvent::error(Some(900)));
            t += 1.0;
        }
        // 3 个决策轮：pressure 连续驻留（含切换轮）→ 确认迁移（一次）。
        for i in 0..3u32 {
            shadow.on_decision_round(t + f64::from(i) * 10.0, 8.0);
        }
        let confirmed: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::MigrationConfirmed)
            .collect();
        assert_eq!(confirmed.len(), 1, "one confirmation per migration");
        assert!(
            confirmed[0].text.contains("域事件: 稳定确认"),
            "{}",
            confirmed[0].text
        );
        assert!(
            confirmed[0].text.contains("稳定 3 轮"),
            "{}",
            confirmed[0].text
        );
        // 0cp D6/D7：域事件统一行格式（T̂ 注解＋动作特征；无成因）。
        assert!(
            confirmed[0].text.contains(RLI_T_HAT_ANNOTATION),
            "{}",
            confirmed[0].text
        );
        assert!(
            confirmed[0].text.contains("err=") && confirmed[0].text.contains("近"),
            "{}",
            confirmed[0].text
        );
        // 第一段 pressure 尚未「完成」（无后继迁移）——倾向统计恰为空
        // （条件口径：只统计**本域已完段**）。
        let first = shadow.transition_tendency();
        assert_eq!(first.domain, Domain::Pressure);
        assert_eq!(first.completed_episodes, 0, "{first:?}");
        // 错误退散＋成功保温 → normal 段；再错误风暴 → pressure 第二段。
        // （成功 = `inject_set` 置 1.0，prog 通道 ζ=2 过阻尼衰减较快——
        // 决策轮紧跟成功之后采，保证 u_prog ≥ 0.5。）
        t += 10.0;
        for _ in 0..3u32 {
            t += 40.0;
            t += 1.0;
            shadow.on_tool_event(t, ToolEvent::success(Some(120)));
            t += 1.0;
            shadow.on_decision_round(t, 8.0);
        }
        assert_eq!(shadow.domain().current_domain(), Domain::Normal);
        for _ in 0..3u32 {
            t += 1.0;
            shadow.on_tool_event(t, ToolEvent::error(Some(900)));
        }
        t += 9.0;
        shadow.on_decision_round(t, 8.0);
        assert_eq!(shadow.domain().current_domain(), Domain::Pressure);
        // 倾向：当前段 pressure 的**已完段**（第一段）统计——去向 normal。
        let tendency = shadow.transition_tendency();
        assert_eq!(tendency.domain, Domain::Pressure);
        assert!(tendency.completed_episodes >= 1, "{tendency:?}");
        assert!(
            tendency
                .directions
                .iter()
                .any(|(d, n)| *d == Domain::Normal && *n >= 1),
            "{tendency:?}"
        );
        assert!(tendency.median_dwell.is_some(), "{tendency:?}");
        assert!(tendency.per_round_rate.is_some(), "{tendency:?}");
    }

    /// 0bf ③④（2026-09-22）＋0cp D2/D6/D7（2026-10-03）：提醒队列／动作
    /// 采样锚／看门狗武装／T̂ 前值／域事件计数随快照往返（未投递标志原样
    /// 保留）；legacy 快照（新字段清空/缺省）= 空/None/0（FR-7）。
    #[test]
    fn notices_grid_and_anchor_ride_the_shadow_snapshot() {
        let mut shadow = RliShadow::new();
        for i in 0..5u32 {
            shadow.on_tool_event(10.0 + i as f64, ToolEvent::error(Some(900)));
        }
        shadow.on_decision_round(100.0, 18.0);
        let snap = shadow.snapshot();
        assert!(!snap.notices.is_empty());
        assert!(snap.last_sample_t.is_some());
        assert!(
            snap.t_hat_prev.is_some(),
            "T̂ 变迁随快照（趋势跨 prompt 续接）"
        );
        let mut restored = RliShadow::new();
        assert!(restored.restore(&snap));
        assert_eq!(restored.sample_points(), shadow.sample_points());
        assert_eq!(restored.notices().len(), shadow.notices().len());
        assert_eq!(
            restored.pending_notices().len(),
            shadow.pending_notices().len()
        );
        assert_eq!(restored.watchdog_armed, shadow.watchdog_armed);
        assert_eq!(restored.t_hat_prev, shadow.t_hat_prev);
        assert_eq!(restored.spike_counts(), shadow.spike_counts());
        // legacy 快照（新字段清空/缺省）= 空/None/0。
        let mut legacy = snap.clone();
        legacy.notices.clear();
        legacy.last_sample_t = None;
        legacy.sample_points = 0;
        legacy.watchdog_armed = true;
        legacy.t_hat_prev = None;
        legacy.spike_entries = 0;
        legacy.spike_returns = 0;
        let mut target = RliShadow::new();
        assert!(target.restore(&legacy));
        assert!(target.notices().is_empty());
        assert_eq!(target.sample_points(), 0);
        assert!(target.watchdog_armed, "legacy = 武装（宁可多采一次）");
        assert_eq!(target.t_hat_prev, None);
        assert_eq!(target.spike_counts(), (0, 0));
    }

    /// 0cp D7（167 批用户裁定）：spike 进入端即提醒——自判域切换当刻报一次
    /// （统一「域事件提醒」行格式：T̂ 注解＋err/slow/stall 现值＋近期动作
    /// 形态＋域趋势进/回累计）；回归端（low_progress→normal）不提醒只计数；
    /// bootstrap（Start→首域）不提醒；稳定确认触发条件与域机器不变（钉）。
    #[test]
    fn domain_spike_entry_notifies_immediately_return_is_silent() {
        let mut shadow = RliShadow::new();
        // bootstrap：首个成功离开 Start——不提醒（无 spike 语义）。
        shadow.on_tool_event(1.0, ToolEvent::success(Some(120)));
        shadow.on_decision_round(2.0, 8.0);
        assert_eq!(shadow.spike_counts(), (0, 0));
        assert!(shadow.notices().is_empty(), "bootstrap 不提醒");
        // 错误×2＋prog 慢模态衰减（39s 龄）→ u_prog<0.5、u_err<2 ⇒
        // normal→low_progress（进入端）：当刻提醒（无需等稳定；与 recli
        // 瞬态偏移同形）。
        shadow.on_tool_event(3.0, ToolEvent::error(Some(900)));
        shadow.on_tool_event(4.0, ToolEvent::error(Some(900)));
        shadow.on_decision_round(40.0, 8.0);
        let entries: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::DomainSpikeEntry)
            .collect();
        assert_eq!(entries.len(), 1, "spike 进入端当刻提醒");
        let text = &entries[0].text;
        assert!(text.contains("spike进入 normal→low_progress"), "{text}");
        assert!(text.contains("进1回0"), "{text}");
        assert!(text.contains(RLI_T_HAT_ANNOTATION), "{text}");
        assert!(
            text.contains("err=") && text.contains("slow=") && text.contains("stall="),
            "{text}"
        );
        assert!(text.contains("近"), "近期动作形态段: {text}");
        assert!(text.len() <= RLI_NOTICE_TEXT_BUDGET, "{text}");
        // 回归端：成功保温 → low_progress→normal——不提醒，只计数。
        shadow.on_tool_event(60.0, ToolEvent::success(Some(120)));
        shadow.on_decision_round(61.0, 8.0);
        let entries_after: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::DomainSpikeEntry)
            .collect();
        assert_eq!(entries_after.len(), 1, "回归端不提醒");
        assert_eq!(shadow.spike_counts(), (1, 1), "进入/回归计数各 +1");
        // 域机器稳定门不动：错误风暴推回 pressure 并驻留 3 轮（65/72/79）
        // ⇒ 稳定确认照常一次（触发源①；本轮 spike 提醒与确认互不吞）。
        for t in [62.0, 63.0, 64.0] {
            shadow.on_tool_event(t, ToolEvent::error(Some(900)));
        }
        for (i, t) in [65.0, 72.0, 79.0].into_iter().enumerate() {
            shadow.on_decision_round(t, 8.0);
            if i == 2 {
                assert!(
                    shadow
                        .notices()
                        .iter()
                        .any(|n| n.kind == RliNoticeKind::MigrationConfirmed),
                    "稳定确认触发源不变"
                );
            }
        }
    }

    /// 0cp D7 ＋ 169 批审查勘误：**Pressure→Normal 属回归端**——不提醒、
    /// 只计回归（旧判定只枚举 Stuck/LowProgress→Normal，会把「错误压力直接
    /// 衰减回正常」误计成进入并向模型面发「spike进入 pressure→normal」；
    /// `label()` 四值两两可达，该迁移真实可达）。bootstrap（Start→Normal）
    /// 仍不计数不提醒；稳定确认触发源与「进N回M」域趋势读数不变。
    #[test]
    fn domain_pressure_to_normal_counts_return_not_entry() {
        let mut shadow = RliShadow::new();
        // bootstrap：Start→Normal（u_prog 新鲜、u_err=0）——不计数不提醒。
        shadow.on_tool_event(1.0, ToolEvent::success(Some(120)));
        shadow.on_decision_round(2.0, 8.0);
        assert_eq!(shadow.spike_counts(), (0, 0));
        assert!(shadow.notices().is_empty(), "bootstrap 不提醒");
        // 错误风暴 ×3 → u_err≥2 ∧ u_prog≥0.5 ⇒ normal→pressure（进入端，
        // 当刻提醒）；确认门不受影响（切换轮重置稳定候选）。
        for t in [3.0, 4.0, 5.0] {
            shadow.on_tool_event(t, ToolEvent::error(Some(900)));
        }
        shadow.on_decision_round(6.0, 8.0);
        let entries: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::DomainSpikeEntry)
            .collect();
        assert_eq!(entries.len(), 1, "normal→pressure 进入端当刻提醒");
        assert!(
            entries[0].text.contains("spike进入 normal→pressure"),
            "{text}",
            text = entries[0].text
        );
        assert_eq!(shadow.spike_counts(), (1, 0));
        // 压力衰减（err 慢模态 ~107s）＋成功保温（prog 慢模态 ~38s）：
        // t=60 时 u_err<2 ∧ u_prog≥0.5 ⇒ pressure→normal——**回归端**：
        // 不提醒、只计回归（169 批勘误断言核心）。
        shadow.on_tool_event(40.0, ToolEvent::success(Some(120)));
        shadow.on_decision_round(60.0, 8.0);
        let entries_after: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::DomainSpikeEntry)
            .collect();
        assert_eq!(entries_after.len(), 1, "回归端不提醒");
        assert_eq!(shadow.spike_counts(), (1, 1), "进入/回归计数各 +1");
        assert!(
            shadow
                .notices()
                .iter()
                .all(|n| !n.text.contains("pressure→normal")),
            "恢复不得以 spike进入 面目出现: {all:?}",
            all = shadow.notices().iter().map(|n| &n.text).collect::<Vec<_>>()
        );
        // 稳定确认触发源不动：连续驻留 ≥3 轮后确认一次，行内域趋势
        // 「进1回1」与计数面同源。（机器轮：t=2→r1、t=6→r2 切换、
        // t=60→r3 回归切换、t=70/78→r4/r5 驻留——r5 达 settle=3 门。）
        shadow.on_tool_event(65.0, ToolEvent::success(Some(120)));
        shadow.on_decision_round(70.0, 8.0);
        shadow.on_decision_round(78.0, 8.0);
        let confirmed: Vec<&RliNotice> = shadow
            .notices()
            .into_iter()
            .filter(|n| n.kind == RliNoticeKind::MigrationConfirmed)
            .collect();
        assert_eq!(confirmed.len(), 1, "稳定确认恰一次");
        assert!(
            confirmed[0].text.contains("稳定确认 pressure→normal"),
            "{text}",
            text = confirmed[0].text
        );
        assert!(
            confirmed[0].text.contains("进1回1"),
            "{text}",
            text = confirmed[0].text
        );
    }
}
