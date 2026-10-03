//! RLI shadow replay harness（0am 改造四项④，2026-09-20 重写；补充项②③
//! 2026-09-20 按用户令改版）。
//!
//! 沿革：本件原为 0am S3「单组件预测」解码器评估件（标签回路 + Q1′–Q3′
//! 判据，方法论冻结于 2026-09-20）；冻结当轮读数全负，0am 裁决把解码器
//! 形态与标签回路**整体退役**（历史读数保留于
//! `docs/audits/0AM_DECODER_FORM_REPLAY_2026-09-20.md` 与
//! `D:\tb-eval\analysis\0am-rli-decoder-replay-2026-09-20.json`）。本件按
//! RLI 改造四项＋补充项重写为**零标签观测件**：
//!
//! - **域一致性（读数③ 的离线面；补充项②口径）**：RLI 自判动作域 vs
//!   **框架实际动作结果**——同轮动作结果窗 = 该决策轮之后、下一决策轮之前
//!   完成的工具结果集合（＝该轮决策实际引发的动作结果）。两轴核读：
//!   压力面（窗内含 Error|Deny）／进度面（窗内含 Success）。
//!   **不再与 LIF 现域对照**（LIF 维持现役组件，RLI 不与之对照）。
//! - **C3 分离对照（补充项③）**：固定 ω_d 只扫 ζ（**衰减列**）／固定
//!   σ = ζω 只扫 ω_d（**频率列**），把衰减与频率两效应拆开；旧「同 ω 的
//!   ζ=0 对照」降格为 **legacy_non_inertia（参数非惰性检验）**——保留读数、
//!   不再充当阻尼有效性证据。**FR-8（2026-09-20）**：补 **实极点列**——
//!   `prog` 落在过阻尼分支（ζ>1），其上 ω_d／σ 均无定义，故单列「固定 ω
//!   只扫 ζ>1」；三列覆盖两类通道，无通道落在扫描面之外。
//! - **C1 时间打乱**锚点读数（证伪门保留；置换 Δt 多重集、事件序列不变）。
//! - **通道级读数**：末态 hits、锚点均值（|u|、|v|、|pred|、E、节律）。
//! - 改造四项②：a2 锚点 = 闭式自由演化对 `Δt = 10·T̂` 前推（无迭代、无
//!   噪声）。配极改（补充项①）后 prog 走实极点（ζ = `RLI_PROG_ZETA`），
//!   其余通道 ζ = `RLI_ZETA`；C3 三列按分支分派（复极点类走衰减／频率列，
//!   过阻尼类走实极点列）。
//!
//! 读数①消费率（模型是否真的拉取 `blackboard_read section=rli`）与读数②
//! 转向相关需要**活体 run**（journal 采集：`blackboard_read` 工具事件的
//! section 字段 + 其后的模型转向），不在本件范围——口径见 0am 结束文档。
//!
//! Usage: `cargo run -p orz-assurance --example rli_shadow_replay -- [root] [out.json]`
//!        （`--selftest`：域一致性接线自检，不跑语料。）

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::DateTime;
use orz_assurance::lif::router::{ActionClass, DenyClass, StimulusRouting, stimulus_targets};
use orz_assurance::lif::{
    ChannelKind, Domain, LifEngine, RLI_CHANNELS, RLI_DOMAIN_PROG_LOW, RLI_PREDICTION_STEPS,
    RliAnchors, ToolEvent, ToolOutcome, classify_event_outcome, is_denial_code,
};
use serde_json::{Value, json};

/// Tiny deterministic xorshift64 PRNG（C1 置换；无新依赖）。
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

/// C1 置换数（冻结点 2026-09-17；本次重写不新增数据选择面）。
const C1_PERMUTATIONS: u64 = 10;

fn find_journals(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .map(|n| n == "events.jsonl")
                .unwrap_or(false)
            {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn parse_time(s: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0) as f64 / 1e9)
}

fn wall_ms_of(payload: &Value) -> Option<u64> {
    payload.get("wall_ms").and_then(Value::as_u64)
}

/// One typed event in replay order（同 `lif_replay`）。
#[derive(Clone)]
enum Step {
    Decision(f64),
    Tool(f64, ToolEvent),
}

impl Step {
    fn t(&self) -> f64 {
        match *self {
            Step::Decision(t) => t,
            Step::Tool(t, _) => t,
        }
    }
}

fn parse_run(path: &Path) -> (String, Vec<Step>) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut steps = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(ts) = event
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_time)
        else {
            continue;
        };
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "model_output" => {
                let calls = payload.get("tool_calls").and_then(Value::as_array);
                if calls.map(|c| !c.is_empty()).unwrap_or(false) {
                    steps.push(Step::Decision(ts));
                }
            }
            "tool_completed" => {
                let outcome = classify_event_outcome(&payload);
                let wall_ms = wall_ms_of(&payload);
                steps.push(Step::Tool(
                    ts,
                    ToolEvent {
                        outcome,
                        wall_ms,
                        policy_denied: false,
                        routing: None,
                    },
                ));
            }
            _ => {}
        }
    }
    let run_id = text
        .lines()
        .find_map(|l| {
            serde_json::from_str::<Value>(l).ok().and_then(|v| {
                v.get("run_id")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
            })
        })
        .unwrap_or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
    (run_id, steps)
}

/// C3 极点变换（补充项③口径：把衰减与频率两效应拆开，各出一列读数）。
#[derive(Clone, Copy)]
enum PoleTransform {
    /// 基线（生产语义配极：err/stall/slow/deny ζ=0.5 复极点；prog ζ=2.0 实极点）。
    Baseline,
    /// **衰减列**：固定各通道 ω_d、把 ζ 扫到目标值（频率不动、衰减变）。
    ZetaAtFixedWd(f64),
    /// **频率列**：固定各通道 σ = ζω、把 ω_d 乘上倍率（衰减不动、频率变）。
    WdAtFixedSigma(f64),
    /// **实极点列（FR-8）**：固定 ω 只扫 ζ 到目标值（> 1，过阻尼分支）——
    /// `prog` 的语义配极所在分支；复极点类通道不参与（其分离由衰减列承担）。
    RealAtFixedOmega(f64),
    /// 旧口径（同 ω 的 ζ=0）：**参数非惰性检验**——频率与衰减同时变化，
    /// 不构成阻尼有效性证据（补充项③降格标注，读数保留）。
    LegacyZetaZero,
}

/// 每决策轮一行（零标签观测）：RLI 自判域 + 全通道锚点 + 同轮动作结果窗。
struct RoundRow {
    rli_domain: Domain,
    anchors: BTreeMap<ChannelKind, RliAnchors>,
    /// 同轮动作结果窗（该决策轮之后、下一决策轮之前的工具结果）：负面
    /// （Error|Deny）计数。
    window_negative: u64,
    /// 同轮动作结果窗：Success 计数。
    window_success: u64,
    /// 窗内结果总数（0 = 无对照窗——不计入一致性核读）。
    window_outcomes: u64,
}

impl RoundRow {
    fn anchor(&self, kind: ChannelKind) -> Option<&RliAnchors> {
        self.anchors.get(&kind)
    }

    fn value(&self, kind: ChannelKind, pick: fn(&RliAnchors) -> f64) -> Option<f64> {
        self.anchor(kind).map(pick)
    }

    /// RLI 压力面主张（域 ∈ {pressure, stuck}）。
    fn pressure_claim(&self) -> bool {
        matches!(self.rli_domain, Domain::Pressure | Domain::Stuck)
    }

    /// RLI 低进度面主张（域 ∈ {low_progress, stuck}）。
    fn low_progress_claim(&self) -> bool {
        matches!(self.rli_domain, Domain::LowProgress | Domain::Stuck)
    }

    /// **模态分离对照（2026-09-20）**：低进度主张的替代判据——用 prog 的
    /// **持久分量** `c_slow` 替水平 `u`（「持久推进水平低」而非「瞬时水平
    /// 低」）。同一阈 `RLI_DOMAIN_PROG_LOW`，只换判据用的量。
    fn low_progress_claim_slow(&self) -> bool {
        // 与域机同门：首个成功前（Start）不产生低进度主张（否则两侧判据
        // 的差异会混进「起始门」这一无关因素）。
        if matches!(self.rli_domain, Domain::Start) {
            return false;
        }
        self.anchor(ChannelKind::Prog)
            .map(|a| a.mode_slow < RLI_DOMAIN_PROG_LOW)
            .unwrap_or(false)
    }

    /// 框架实际动作结果：压力面（窗内含 Error|Deny）。
    fn pressure_actual(&self) -> bool {
        self.window_negative > 0
    }

    /// 框架实际动作结果：进度面（窗内含 Success）。
    fn progress_actual(&self) -> bool {
        self.window_success > 0
    }
}

/// 单 run 重放结果（行 + 自判域切换数）。
struct RunReplay {
    rows: Vec<RoundRow>,
    rli_migrations: usize,
    /// 事件级模态间隔（2026-09-20 采样粒度裁决的读数面）：每个工具事件后
    /// `|c_slow_prog − u_prog|`（＝快分量大小）的分布统计。
    event_gap: GapStats,
}

/// 模态间隔分布统计（轮粒度与事件粒度共用）。
#[derive(Clone, Copy, Default)]
struct GapStats {
    n: u64,
    sum: f64,
    max: f64,
    gt_0_01: u64,
    gt_0_05: u64,
}

impl GapStats {
    fn push(&mut self, gap: f64) {
        if !gap.is_finite() {
            return;
        }
        self.n += 1;
        self.sum += gap;
        self.max = self.max.max(gap);
        if gap > 0.01 {
            self.gt_0_01 += 1;
        }
        if gap > 0.05 {
            self.gt_0_05 += 1;
        }
    }

    fn mean(&self) -> f64 {
        if self.n > 0 {
            self.sum / self.n as f64
        } else {
            f64::NAN
        }
    }

    fn merge(&mut self, other: &GapStats) {
        self.n += other.n;
        self.sum += other.sum;
        self.max = self.max.max(other.max);
        self.gt_0_01 += other.gt_0_01;
        self.gt_0_05 += other.gt_0_05;
    }
}

/// 按 C3 极点变换配置影子（只作用于**复极点类**通道；prog 的实极点配置是
/// 语义分配（补充项①），不在 C3 扫描面——如实登记）。
fn apply_pole_transform(engine: &mut LifEngine, transform: PoleTransform) {
    let Some(shadow) = engine.rli_shadow_mut() else {
        return;
    };
    for &kind in &RLI_CHANNELS {
        let ch = shadow.channel_mut(kind);
        let (omega_b, zeta_b) = (ch.omega(), ch.zeta());
        // 分支路由（FR-8）：实极点列只作用于过阻尼类通道，其余三列只作用于
        // 复极点类通道——两分支的「分离不变量」不同，混用会把两个效应重新
        // 叠在一起，正好是补充项③要拆开的东西。
        let overdamped_branch = zeta_b > 1.0;
        if matches!(transform, PoleTransform::RealAtFixedOmega(_)) != overdamped_branch {
            continue;
        }
        match transform {
            PoleTransform::Baseline => {}
            PoleTransform::ZetaAtFixedWd(zeta) => {
                let wd_b = omega_b * (1.0 - zeta_b * zeta_b).sqrt();
                let zeta = zeta.clamp(0.0, 0.95);
                ch.set_poles(wd_b / (1.0 - zeta * zeta).sqrt(), zeta);
            }
            PoleTransform::WdAtFixedSigma(scale) => {
                let wd_b = omega_b * (1.0 - zeta_b * zeta_b).sqrt();
                let sigma_b = zeta_b * omega_b;
                let wd = wd_b * scale.max(1e-6);
                let omega = (sigma_b * sigma_b + wd * wd).sqrt();
                ch.set_poles(omega, sigma_b / omega);
            }
            PoleTransform::RealAtFixedOmega(zeta) => {
                ch.set_poles(omega_b, zeta.clamp(1.000_001, 4.0));
            }
            PoleTransform::LegacyZetaZero => ch.set_zeta(0.0),
        }
    }
}

fn replay_run(steps: &[Step], transform: PoleTransform) -> RunReplay {
    let mut engine = LifEngine::new();
    engine.enable_rli_shadow();
    apply_pole_transform(&mut engine, transform);
    let mut rows: Vec<RoundRow> = Vec::new();
    // 同轮动作结果窗：决策轮开启新窗，工具结果落入当前窗（上一窗在下一
    // 决策轮处封存）。run 起点、首个决策轮之前的工具结果不属任何轮。
    let mut window_negative = 0u64;
    let mut window_success = 0u64;
    let mut window_outcomes = 0u64;
    let mut window_open = false;
    let mut event_gap = GapStats::default();
    for step in steps {
        match *step {
            Step::Decision(t) => {
                if window_open {
                    if let Some(row) = rows.last_mut() {
                        row.window_negative = window_negative;
                        row.window_success = window_success;
                        row.window_outcomes = window_outcomes;
                    }
                }
                window_negative = 0;
                window_success = 0;
                window_outcomes = 0;
                window_open = true;
                engine.on_decision_round(t);
                let t_hat = engine.rli_shadow().map(|s| s.t_hat()).unwrap_or(8.0);
                let mut anchors = BTreeMap::new();
                if let Some(shadow) = engine.rli_shadow() {
                    for &kind in &RLI_CHANNELS {
                        anchors.insert(kind, shadow.channel(kind).anchors(t_hat));
                    }
                }
                let rli_domain = engine
                    .rli_shadow()
                    .and_then(|shadow| shadow.domain().now().copied())
                    .map(|row| row.domain)
                    .unwrap_or(Domain::Start);
                rows.push(RoundRow {
                    rli_domain,
                    anchors,
                    window_negative: 0,
                    window_success: 0,
                    window_outcomes: 0,
                });
            }
            Step::Tool(t, ev) => {
                match ev.outcome {
                    orz_assurance::lif::ToolOutcome::Error
                    | orz_assurance::lif::ToolOutcome::Deny => window_negative += 1,
                    orz_assurance::lif::ToolOutcome::Success => window_success += 1,
                    orz_assurance::lif::ToolOutcome::Other => {}
                }
                window_outcomes += 1;
                engine.on_tool_event(t, ev);
                // 事件级模态间隔（采样粒度读数：注入当刻快分量最大）。
                if let Some(shadow) = engine.rli_shadow() {
                    let ch = shadow.channel(ChannelKind::Prog);
                    event_gap.push((ch.mode_slow() - ch.u()).abs());
                }
            }
        }
    }
    if window_open && let Some(row) = rows.last_mut() {
        row.window_negative = window_negative;
        row.window_success = window_success;
        row.window_outcomes = window_outcomes;
    }
    let rli_migrations = match engine.rli_shadow() {
        Some(shadow) => shadow.domain().spikes().len(),
        None => 0,
    };
    RunReplay {
        rows,
        rli_migrations,
        event_gap,
    }
}

/// 时间打乱（同 `lif_replay` 置换口径：置换 Δt 多重集，事件序列不变）。
fn shuffled_interval_steps(steps: &[Step], perm_seed: u64) -> Vec<Step> {
    let times: Vec<f64> = steps.iter().map(Step::t).collect();
    let intervals: Vec<f64> = times.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();
    let mut rng = Xorshift64::new(0x9E37_79B9_7F4A_7C15 ^ perm_seed.wrapping_mul(0x100_0000_01B3));
    let mut shuffled = intervals.clone();
    for i in (1..shuffled.len()).rev() {
        let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
        shuffled.swap(i, j);
    }
    let mut t = times.first().copied().unwrap_or(0.0);
    let mut out = Vec::with_capacity(steps.len());
    let mut k = 0usize;
    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            t += shuffled[k];
            k += 1;
        }
        out.push(match *step {
            Step::Decision(_) => Step::Decision(t),
            Step::Tool(_, ev) => Step::Tool(t, ev),
        });
    }
    out
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

fn abs_diff(a: Option<f64>, b: Option<f64>) -> f64 {
    match (a, b) {
        (Some(x), Some(y)) => (x - y).abs(),
        _ => 0.0,
    }
}

fn channel_key(kind: ChannelKind) -> String {
    format!("{kind:?}").to_lowercase()
}

/// C3 列读数：相对基线的**指定通道**锚点平均绝对差（u 与解析包络 E）。
/// FR-8 起按通道参数化——复极点两列取 `Err`，实极点列取 `Prog`。
fn mean_anchor_deltas(baseline: &RunReplay, alt: &RunReplay, kind: ChannelKind) -> (f64, f64) {
    let mut du = Vec::new();
    let mut de = Vec::new();
    for (orig, other) in baseline.rows.iter().zip(alt.rows.iter()) {
        du.push(abs_diff(
            orig.value(kind, |a| a.u),
            other.value(kind, |a| a.u),
        ));
        de.push(abs_diff(
            orig.value(kind, |a| a.envelope),
            other.value(kind, |a| a.envelope),
        ));
    }
    (mean(&du), mean(&de))
}

/// 自检（`--selftest`）：合成全成功事件流上，同轮动作结果窗逐轮非空且与
/// RLI 自判域两面一致（接线自检；不跑语料）。
fn selftest() {
    let mut steps = Vec::new();
    let mut t = 0.0f64;
    for _ in 0..8 {
        t += 5.0;
        steps.push(Step::Decision(t));
        t += 1.0;
        steps.push(Step::Tool(t, ToolEvent::success(Some(100))));
    }
    let replay = replay_run(&steps, PoleTransform::Baseline);
    assert_eq!(replay.rows.len(), 8, "decision rounds replayed");
    assert!(
        replay.rows.iter().all(|r| r.window_outcomes > 0),
        "every round carries a same-round action window"
    );
    for row in &replay.rows {
        assert!(
            !row.pressure_claim(),
            "all-success stream: no pressure claim"
        );
        assert!(
            !row.low_progress_claim(),
            "all-success stream: no low-progress claim"
        );
        assert_eq!(
            row.pressure_claim(),
            row.pressure_actual(),
            "pressure face agrees with the actual action results"
        );
        assert_eq!(
            row.low_progress_claim(),
            !row.progress_actual(),
            "progress face agrees with the actual action results"
        );
    }
    let last = replay.rows.last().expect("rows");
    assert_eq!(
        last.rli_domain,
        Domain::Normal,
        "all-success stream must settle in the normal domain"
    );
    println!("rli shadow replay selftest ok (action-result consistency wiring)");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s == "--selftest").unwrap_or(false) {
        selftest();
        return;
    }
    if args.get(1).map(|s| s == "--s3-selftest").unwrap_or(false) {
        s3_selftest();
        return;
    }
    if args.get(1).map(|s| s == "--s3").unwrap_or(false) {
        let root = args
            .get(2)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"D:\tb-eval\jobs-official"));
        let out = args
            .get(3)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("s3-routing-replay.json"));
        run_s3(&root, &out);
        return;
    }
    let root = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\tb-eval\jobs-official"));
    let out = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("rli-shadow-replay.json"));

    let journals = find_journals(&root);
    let mut run_reports: Vec<Value> = Vec::new();
    let mut c1_readings: Vec<Value> = Vec::new();
    let mut c3_readings: Vec<Value> = Vec::new();
    let mut runs = 0u64;
    let mut total_rounds = 0u64;
    // 域一致性（补充项②口径）：RLI 自判域 vs 框架实际动作结果（同轮动作
    // 结果窗两轴核读）。
    let mut windows = 0u64;
    let mut pressure_agrees = 0u64;
    let mut progress_agrees = 0u64;
    let mut joint_agrees = 0u64;
    let mut pressure_claims = 0u64;
    let mut pressure_actuals = 0u64;
    let mut low_progress_claims = 0u64;
    let mut no_progress_rounds = 0u64;
    let mut pressure_confusion: BTreeMap<String, u64> = BTreeMap::new();
    let mut progress_confusion: BTreeMap<String, u64> = BTreeMap::new();
    // 模态分离对照（2026-09-20）：低进度判据「水平 u_prog」vs「持久分量
    // c_slow_prog」，同批语料、同一实际对照（同轮动作结果窗）。
    let mut prog_u_agrees = 0u64;
    let mut prog_slow_agrees = 0u64;
    let mut prog_u_claims = 0u64;
    let mut prog_slow_claims = 0u64;
    let mut prog_actuals = 0u64;
    let mut prog_u_confusion: BTreeMap<String, u64> = BTreeMap::new();
    let mut prog_slow_confusion: BTreeMap<String, u64> = BTreeMap::new();
    // 模态间隔统计（解释两判据是否等价：c_slow 与 u 的差即快分量的大小）；
    // 分**轮粒度**与**事件粒度**两栏——后者是采样粒度裁决后的真实分辨力。
    let mut prog_gap_round = GapStats::default();
    let mut prog_gap_event = GapStats::default();
    let mut prog_claim_flips = 0u64;
    let mut rli_domain_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut rli_migrations_total = 0u64;
    let mut channel_hits: BTreeMap<String, u64> = BTreeMap::new();
    let mut channel_u: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut channel_v: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut channel_pred: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut channel_env: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    let mut channel_rhythm: BTreeMap<String, Vec<f64>> = BTreeMap::new();

    for journal in &journals {
        let (run_id, steps) = parse_run(journal);
        if steps.is_empty() {
            continue;
        }
        let replay = replay_run(&steps, PoleTransform::Baseline);
        if replay.rows.is_empty() {
            continue;
        }
        runs += 1;
        total_rounds += replay.rows.len() as u64;
        rli_migrations_total += replay.rli_migrations as u64;

        let mut run_windows = 0u64;
        let mut run_pressure_agrees = 0u64;
        let mut run_progress_agrees = 0u64;
        let mut run_joint_agrees = 0u64;
        let mut run_prog_u_agrees = 0u64;
        let mut run_prog_slow_agrees = 0u64;
        let mut first_pressure_mismatch: Option<u64> = None;
        let mut first_progress_mismatch: Option<u64> = None;
        for (idx, row) in replay.rows.iter().enumerate() {
            *rli_domain_counts
                .entry(row.rli_domain.as_str().to_string())
                .or_default() += 1;
            if row.window_outcomes == 0 {
                // 空窗 = 无对照（该轮决策无完成结果可核读）。
                continue;
            }
            run_windows += 1;
            let pressure_claim = row.pressure_claim();
            let pressure_actual = row.pressure_actual();
            let low_prog_claim = row.low_progress_claim();
            let no_progress_actual = !row.progress_actual();
            if pressure_claim {
                pressure_claims += 1;
            }
            if pressure_actual {
                pressure_actuals += 1;
            }
            if low_prog_claim {
                low_progress_claims += 1;
            }
            if no_progress_actual {
                no_progress_rounds += 1;
            }
            let pressure_ok = pressure_claim == pressure_actual;
            let progress_ok = low_prog_claim == no_progress_actual;
            if pressure_ok {
                run_pressure_agrees += 1;
            } else if first_pressure_mismatch.is_none() {
                first_pressure_mismatch = Some(idx as u64 + 1);
            }
            if progress_ok {
                run_progress_agrees += 1;
            } else if first_progress_mismatch.is_none() {
                first_progress_mismatch = Some(idx as u64 + 1);
            }
            if pressure_ok && progress_ok {
                run_joint_agrees += 1;
            }
            *pressure_confusion
                .entry(format!("{pressure_claim}|{pressure_actual}"))
                .or_default() += 1;
            *progress_confusion
                .entry(format!("{low_prog_claim}|{no_progress_actual}"))
                .or_default() += 1;
            // 模态分离对照（2026-09-20）：同一实际结果下，两个低进度判据
            // 各自与「框架实际动作结果」的核读。
            let slow_claim = row.low_progress_claim_slow();
            if slow_claim {
                prog_slow_claims += 1;
            }
            if low_prog_claim {
                prog_u_claims += 1;
            }
            if no_progress_actual {
                prog_actuals += 1;
            }
            if low_prog_claim == no_progress_actual {
                run_prog_u_agrees += 1;
            }
            if slow_claim == no_progress_actual {
                run_prog_slow_agrees += 1;
            }
            if slow_claim != low_prog_claim {
                prog_claim_flips += 1;
            }
            if let Some(a) = row.anchor(ChannelKind::Prog) {
                prog_gap_round.push((a.mode_slow - a.u).abs());
            }
            *prog_u_confusion
                .entry(format!("{low_prog_claim}|{no_progress_actual}"))
                .or_default() += 1;
            *prog_slow_confusion
                .entry(format!("{slow_claim}|{no_progress_actual}"))
                .or_default() += 1;
        }
        windows += run_windows;
        pressure_agrees += run_pressure_agrees;
        progress_agrees += run_progress_agrees;
        joint_agrees += run_joint_agrees;
        prog_u_agrees += run_prog_u_agrees;
        prog_slow_agrees += run_prog_slow_agrees;
        prog_gap_event.merge(&replay.event_gap);

        // 通道级读数：末态 hits（逐 run 累加）+ 全窗口锚点均值。
        for &kind in &RLI_CHANNELS {
            let key = channel_key(kind);
            if let Some(last) = replay.rows.last().and_then(|r| r.anchor(kind)) {
                *channel_hits.entry(key.clone()).or_default() += last.hits;
            }
            let u_vals: Vec<f64> = replay
                .rows
                .iter()
                .filter_map(|r| r.value(kind, |a| a.u))
                .collect();
            let v_vals: Vec<f64> = replay
                .rows
                .iter()
                .filter_map(|r| r.value(kind, |a| a.v))
                .collect();
            let pred_vals: Vec<f64> = replay
                .rows
                .iter()
                .filter_map(|r| r.value(kind, |a| a.prediction))
                .collect();
            let env_vals: Vec<f64> = replay
                .rows
                .iter()
                .filter_map(|r| r.value(kind, |a| a.envelope))
                .collect();
            let rhythm_vals: Vec<f64> = replay
                .rows
                .iter()
                .filter_map(|r| r.value(kind, |a| a.rhythm))
                .collect();
            channel_u
                .entry(key.clone())
                .or_default()
                .push(mean(&u_vals));
            channel_v
                .entry(key.clone())
                .or_default()
                .push(mean(&v_vals));
            channel_pred
                .entry(key.clone())
                .or_default()
                .push(mean(&pred_vals));
            channel_env
                .entry(key.clone())
                .or_default()
                .push(mean(&env_vals));
            channel_rhythm
                .entry(key.clone())
                .or_default()
                .push(mean(&rhythm_vals));
        }

        // C1：10 个种子置换后的锚点读数（对比同 run 原序列）。
        let mut du_err = Vec::new();
        let mut du_prog = Vec::new();
        let mut drhythm_err = Vec::new();
        let mut anchor_changed = 0u32;
        for perm in 0..C1_PERMUTATIONS {
            let permuted = replay_run(
                &shuffled_interval_steps(&steps, perm),
                PoleTransform::Baseline,
            );
            let mut changed = false;
            for (orig, perm_row) in replay.rows.iter().zip(permuted.rows.iter()) {
                let du = abs_diff(
                    orig.value(ChannelKind::Err, |a| a.u),
                    perm_row.value(ChannelKind::Err, |a| a.u),
                );
                let dprog = abs_diff(
                    orig.value(ChannelKind::Prog, |a| a.u),
                    perm_row.value(ChannelKind::Prog, |a| a.u),
                );
                let dr = abs_diff(
                    orig.value(ChannelKind::Err, |a| a.rhythm),
                    perm_row.value(ChannelKind::Err, |a| a.rhythm),
                );
                if du > 1e-9 || dprog > 1e-9 {
                    changed = true;
                }
                du_err.push(du);
                du_prog.push(dprog);
                drhythm_err.push(dr);
            }
            if changed {
                anchor_changed += 1;
            }
        }
        c1_readings.push(json!({
            "run_id": run_id,
            "rounds": replay.rows.len(),
            "permutations_anchor_changed": anchor_changed,
            "mean_abs_du_err": mean(&du_err),
            "mean_abs_du_prog": mean(&du_prog),
            "mean_abs_d_rhythm_err": mean(&drhythm_err),
        }));

        // C3 分离对照（补充项③）：**衰减列**＝固定 ω_d 只扫 ζ（频率不动、
        // 衰减变）；**频率列**＝固定 σ = ζω 只扫 ω_d（衰减不动、频率变）。
        // 两列各自含一个恒等点（ζ=0.5 / ×1.0）作为变换惰性自检。旧口径
        // 「同 ω 的 ζ=0」降格标注为**参数非惰性检验**（频率与衰减同时变，
        // 不构成阻尼有效性证据）。
        let baseline = &replay;
        let mut decay_column: Vec<Value> = Vec::new();
        for zeta in [0.0, 0.25, 0.5, 0.75, 0.95] {
            let alt = replay_run(&steps, PoleTransform::ZetaAtFixedWd(zeta));
            let (du, de) = mean_anchor_deltas(baseline, &alt, ChannelKind::Err);
            decay_column.push(json!({
                "zeta": zeta,
                "mean_abs_du_err": du,
                "mean_abs_d_env_err": de,
            }));
        }
        let mut frequency_column: Vec<Value> = Vec::new();
        for scale in [0.5, 0.75, 1.0, 1.5, 2.0] {
            let alt = replay_run(&steps, PoleTransform::WdAtFixedSigma(scale));
            let (du, de) = mean_anchor_deltas(baseline, &alt, ChannelKind::Err);
            frequency_column.push(json!({
                "wd_scale": scale,
                "mean_abs_du_err": du,
                "mean_abs_d_env_err": de,
            }));
        }
        // FR-8：**实极点列**（prog）——固定 ω 只扫 ζ>1。复极点两列的分离
        // 不变量（ω_d／σ）在过阻尼分支无定义，故单列；ζ=2.0 即生产配极
        // 恒等点（自检用）。
        let mut real_pole_column: Vec<Value> = Vec::new();
        for zeta in [1.5, 2.0, 3.0, 4.0] {
            let alt = replay_run(&steps, PoleTransform::RealAtFixedOmega(zeta));
            let (du, de) = mean_anchor_deltas(baseline, &alt, ChannelKind::Prog);
            real_pole_column.push(json!({
                "zeta": zeta,
                "mean_abs_du_prog": du,
                "mean_abs_d_env_prog": de,
            }));
        }
        let legacy = replay_run(&steps, PoleTransform::LegacyZetaZero);
        let (legacy_du, legacy_de) = mean_anchor_deltas(baseline, &legacy, ChannelKind::Err);
        c3_readings.push(json!({
            "run_id": run_id,
            "decay_column_fixed_wd": decay_column,
            "frequency_column_fixed_sigma": frequency_column,
            "real_pole_column_fixed_omega_prog": real_pole_column,
            "legacy_non_inertia_zeta0_same_omega": {
                "mean_abs_du_err": legacy_du,
                "mean_abs_d_env_err": legacy_de,
                "label": "参数非惰性检验（补充项③降格标注）——频率与衰减同时变化，不构成阻尼有效性证据",
            },
        }));

        let last = replay.rows.last().expect("non-empty");
        run_reports.push(json!({
            "run_id": run_id,
            "rounds": replay.rows.len(),
            "action_result_consistency": {
                "windows": run_windows,
                "pressure_agrees": run_pressure_agrees,
                "progress_agrees": run_progress_agrees,
                "joint_agrees": run_joint_agrees,
                "joint_rate": if run_windows > 0 {
                    run_joint_agrees as f64 / run_windows as f64
                } else {
                    f64::NAN
                },
                "first_pressure_mismatch_round": first_pressure_mismatch,
                "first_progress_mismatch_round": first_progress_mismatch,
            },
            "prog_predicate_contrast": {
                "windows": run_windows,
                "u_prog_agrees": run_prog_u_agrees,
                "c_slow_prog_agrees": run_prog_slow_agrees,
            },
            "migrations": replay.rli_migrations,
            "final_domain": last.rli_domain.as_str(),
        }));
    }

    let mut channel_summary: BTreeMap<String, Value> = BTreeMap::new();
    for (key, hits) in &channel_hits {
        channel_summary.insert(
            key.clone(),
            json!({
                "final_hits_sum": hits,
                "mean_of_run_mean_u": mean(channel_u.get(key).map(Vec::as_slice).unwrap_or(&[])),
                "mean_of_run_mean_v": mean(channel_v.get(key).map(Vec::as_slice).unwrap_or(&[])),
                "mean_of_run_mean_pred": mean(channel_pred.get(key).map(Vec::as_slice).unwrap_or(&[])),
                "mean_of_run_mean_envelope": mean(channel_env.get(key).map(Vec::as_slice).unwrap_or(&[])),
                "mean_of_run_mean_rhythm": mean(channel_rhythm.get(key).map(Vec::as_slice).unwrap_or(&[])),
            }),
        );
    }

    let rate = |agrees: u64, total: u64| {
        if total > 0 {
            agrees as f64 / total as f64
        } else {
            f64::NAN
        }
    };
    let report = json!({
        "schema": "rli-shadow-replay-v3 (0am 补充项②③重写, 2026-09-20)",
        "root": root.to_string_lossy(),
        "runs": runs,
        "decision_points": total_rounds,
        "prediction_steps": RLI_PREDICTION_STEPS,
        "domain_consistency": {
            "basis": "框架实际动作结果（同轮动作结果窗＝该决策轮之后、下一决策轮之前完成的工具结果；空窗不计）",
            "windows": windows,
            "pressure": {
                "agrees": pressure_agrees,
                "rate": rate(pressure_agrees, windows),
                "claims": pressure_claims,
                "actuals": pressure_actuals,
                "confusion_claim_actual": pressure_confusion,
            },
            "progress": {
                "agrees": progress_agrees,
                "rate": rate(progress_agrees, windows),
                "claims": low_progress_claims,
                "actuals": no_progress_rounds,
                "confusion_claim_actual": progress_confusion,
            },
            "joint_agrees": joint_agrees,
            "joint_rate": rate(joint_agrees, windows),
            "rli_domain_counts": rli_domain_counts,
            "migrations": rli_migrations_total,
            "runs": run_reports,
        },
        "channels": channel_summary,
        "prog_predicate_contrast": {
            "basis": "模态分离对照（2026-09-20 用户裁决）：同一实际对照（同轮动作结果窗内无成功＝进度停滞），只换低进度判据用的量——水平 u_prog（现行）vs 持久分量 c_slow_prog；同阈 RLI_DOMAIN_PROG_LOW；Start 门前两侧同门（不产生主张）。",
            "threshold": RLI_DOMAIN_PROG_LOW,
            "windows": windows,
            "actuals_no_progress": prog_actuals,
            "u_prog": {
                "agrees": prog_u_agrees,
                "rate": rate(prog_u_agrees, windows),
                "claims": prog_u_claims,
                "confusion_claim_actual": prog_u_confusion,
            },
            "c_slow_prog": {
                "agrees": prog_slow_agrees,
                "rate": rate(prog_slow_agrees, windows),
                "claims": prog_slow_claims,
                "confusion_claim_actual": prog_slow_confusion,
            },
            "delta_rate_slow_minus_u": rate(prog_slow_agrees, windows) - rate(prog_u_agrees, windows),
            "gap_c_slow_minus_u_prog": {
                "rounds": prog_gap_round.n,
                "mean_abs": prog_gap_round.mean(),
                "max_abs": prog_gap_round.max,
                "rounds_gap_gt_0_01": prog_gap_round.gt_0_01,
                "rounds_gap_gt_0_05": prog_gap_round.gt_0_05,
                "claim_flips_between_predicates": prog_claim_flips,
                "note": "轮粒度：c_slow − u 即快分量（符号相反）的大小；间隔趋近 0 表示轮粒度宽于快模态消失进程（用户 2026-09-20 判读）。",
            },
            "gap_event_level": {
                "samples": prog_gap_event.n,
                "mean_abs": prog_gap_event.mean(),
                "max_abs": prog_gap_event.max,
                "samples_gap_gt_0_01": prog_gap_event.gt_0_01,
                "samples_gap_gt_0_05": prog_gap_event.gt_0_05,
                "note": "事件粒度（每个工具事件后取一次；2026-09-20 用户裁决「采样尽可能细」）：注入当刻快分量最大，本栏才是模态对的真实分辨力。",
            },
        },
        "c1": { "permutations": C1_PERMUTATIONS, "runs": c1_readings },
        "c3": {
            "sweep_face": "复极点类通道（err/stall/slow/deny）；prog 的实极点配置是语义分配（补充项①）不在扫描面",
            "runs": c3_readings,
        },
        "retired": {
            "decoder_form": "single-layer linear decoder + next-5 label loop + Q1'-Q3' criteria — retired 2026-09-20 by the 0am ruling",
            "historical_readings": "docs/audits/0AM_DECODER_FORM_REPLAY_2026-09-20.md (+ D:\\tb-eval\\analysis\\0am-rli-decoder-replay-2026-09-20.json)",
            "live_readings_note": "consumption rate (①) and turn relevance (②) require live dogfood/batch runs (journal section= collection) — see the 0am closing doc",
        },
    });
    std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).expect("write report");
    println!("{}", serde_json::to_string(&report).unwrap());
    println!("report written to {}", out.display());
}

// ============================================================================
// 0am S3（2026-10-03）：S2 刺激面路由表离线重放（S2 设计档 §6 判据 J1–J5）。
// 只读重放件扩展——零生产面改动；S3/P8 边界＝Verify/Ctx/Infra 三新通道的
// **动力学**不在 S3（P8 实现），S3 只产出其**输入序列账目**；既有五通道的
// 新旧路由对比走真实引擎（[`replay_routed`]）。
// ============================================================================

// 0am P8（2026-10-03）：标签器与分派**同源复用** lib router（S2 §5——
// 生产喂入点与本重放件共用 `orz_assurance::lif::router`，对拍一致性由
// 构造保证）；本地只保留账目键映射。

fn deny_key(dc: DenyClass) -> &'static str {
    match dc {
        DenyClass::PermissionTicket => "permission_ticket",
        DenyClass::PlanLane => "plan_lane",
        DenyClass::GateGuard => "gate_guard",
        DenyClass::RetrievalEnable => "retrieval_enable",
        DenyClass::WriteControl => "write_control",
        DenyClass::PolicyMarker => "policy_marker",
        DenyClass::Other => "other",
    }
}

/// 单事件 S2 路由分派（router 级账目；通道名与注入值）。
#[derive(Default, Clone)]
struct S2Dispatch {
    class: ActionClass,
    /// 认领注入（通道名, 值）；prog 为 set 语义（值 1.0）。
    claims: Vec<(&'static str, f64)>,
    deny_class: Option<&'static str>,
    /// verify 通过（零注入——基线参考语义）。
    verify_pass: bool,
    /// 显式中性（零认领；与 claims 空等价，字段为可读性冗余）。
    neutral: bool,
}

impl S2Dispatch {
    fn has(&self, ch: &str) -> bool {
        self.claims.iter().any(|(c, _)| *c == ch)
    }
}

/// 单事件旧路由分派（生产现状逐字镜像，供 J1/J2 对账）。
#[derive(Default, Clone)]
struct LegacyDispatch {
    claims: Vec<(&'static str, f64)>,
}

impl LegacyDispatch {
    fn has(&self, ch: &str) -> bool {
        self.claims.iter().any(|(c, _)| *c == ch)
    }
}

/// 路由化步骤：时间 ＋ legacy 馈（窗口 ground truth）＋ S2 馈 ＋ 双分派。
/// 决策轮（model_output 带工具调用）以「无双分派且无完成负载」形态在册。
struct RoutedStep {
    t: f64,
    legacy_feed: ToolEvent,
    s2_feed: ToolEvent,
    legacy: LegacyDispatch,
    s2: S2Dispatch,
    decision: bool,
    seq: u64,
}

/// 会话级 Ctx/Infra 认领账目（非工具事件族；逐 run）。
#[derive(Default)]
struct SideClaims {
    ctx_compressed: u64,
    fold_advance: u64,
    fold_write_failed: u64,
    transport_retry: u64,
    availability: u64,
    host_denied: u64,
    limit_hit: u64,
    snapshot_crossing: u64,
    snapshots_seen: u64,
}

impl SideClaims {
    fn ctx(&self) -> u64 {
        self.ctx_compressed + self.fold_advance
    }
    fn infra(&self) -> u64 {
        self.fold_write_failed
            + self.transport_retry
            + self.availability
            + self.host_denied
            + self.limit_hit
            + self.snapshot_crossing
    }
}

fn slow_weight(wall_ms: u64) -> f64 {
    (wall_ms as f64 / 60_000.0).clamp(1.0, 5.0)
}

/// 单事件双分派核心（[`parse_routed`] 与 s3 自检共用，保证同构）。
fn dispatch_event(
    payload: &Value,
    class: ActionClass,
    wcr_block: bool,
    deny_class: Option<DenyClass>,
) -> (LegacyDispatch, S2Dispatch, ToolEvent, ToolEvent) {
    let legacy_outcome = classify_event_outcome(payload);
    let wall_ms = wall_ms_of(payload);
    let exit_code = payload.get("exit_code").and_then(Value::as_i64);
    let policy_denied = payload.get("policy_denial").is_some();
    let non_zero_exit = exit_code.is_some_and(|c| c != 0);
    // —— legacy 分派（生产现状逐字镜像，供 J1/J2 对账；不变）。
    let mut legacy = LegacyDispatch::default();
    match legacy_outcome {
        ToolOutcome::Error => legacy.claims.push(("err", 1.0)),
        ToolOutcome::Deny => legacy.claims.push(("deny", 1.0)),
        ToolOutcome::Success => legacy.claims.push(("prog", 1.0)),
        ToolOutcome::Other => {}
    }
    if let Some(w) = wall_ms
        && w > 60_000
    {
        legacy.claims.push(("slow", slow_weight(w)));
    }
    // —— S2 分派：lib router 同源（`stimulus_targets`；deny 优先/验证
    // 豁免/H2 填平全部在 lib 语义内）。
    let s2_feed = ToolEvent {
        outcome: legacy_outcome,
        wall_ms,
        policy_denied,
        routing: Some(StimulusRouting {
            class,
            deny_class,
            non_zero_exit,
            write_control_block: wcr_block,
        }),
    };
    let t = stimulus_targets(&s2_feed);
    let mut d = S2Dispatch {
        class,
        ..S2Dispatch::default()
    };
    if t.deny {
        d.claims.push(("deny", 1.0));
        d.deny_class = t.deny_class.map(deny_key);
    }
    if t.verify {
        d.claims.push(("verify", 1.0));
    }
    if t.err {
        d.claims.push(("err", 1.0));
    }
    if t.prog_set {
        d.claims.push(("prog", 1.0));
    }
    if let Some(w) = t.slow {
        d.claims.push(("slow", w));
    }
    d.verify_pass = class == ActionClass::Verify && !t.verify && !t.deny;
    let legacy_feed = ToolEvent {
        outcome: legacy_outcome,
        wall_ms,
        policy_denied: false,
        routing: None,
    };
    (legacy, d, legacy_feed, s2_feed)
}

/// 解析单卷 journal 为路由化步骤流（两遍：先取 call_id→命令/写控审查与
/// Ctx/Infra 侧账，再产步骤）。命令串取自 `model_output.tool_calls`（journal
/// 内结构化字段——S2 §5「零新增采集面」的重放兑现）。
fn parse_routed(path: &Path) -> (String, Vec<RoutedStep>, SideClaims) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut commands: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut wcr_block: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut side = SideClaims::default();
    let mut last_tier: Option<String> = None;
    let mut run_id = String::new();
    for line in text.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if run_id.is_empty() {
            run_id = event
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
        }
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "model_output" => {
                if let Some(calls) = payload.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        let Some(id) = call.get("call_id").and_then(Value::as_str) else {
                            continue;
                        };
                        let args = call.get("arguments");
                        let cmd = match args {
                            Some(Value::Object(o)) => {
                                o.get("command").and_then(Value::as_str).map(String::from)
                            }
                            Some(Value::String(s)) => {
                                serde_json::from_str::<Value>(s).ok().and_then(|v| {
                                    v.get("command").and_then(Value::as_str).map(String::from)
                                })
                            }
                            _ => None,
                        };
                        if let Some(c) = cmd {
                            commands.insert(id.to_string(), c);
                        }
                    }
                }
            }
            "write_control_review" => {
                if let Some(id) = payload.get("call_id").and_then(Value::as_str)
                    && payload.get("review").and_then(Value::as_str) == Some("block")
                {
                    wcr_block.insert(id.to_string());
                }
            }
            "context_compressed" => side.ctx_compressed += 1,
            "ledger_fold_advance" => side.fold_advance += 1,
            "ledger_fold_write_failed" => side.fold_write_failed += 1,
            "transport_retry" => side.transport_retry += 1,
            "tool_availability_check" => side.availability += 1,
            "host_resource_denied" => side.host_denied += 1,
            "resource_limit_hit" => side.limit_hit += 1,
            "host_resource_snapshot" => {
                side.snapshots_seen += 1;
                if let Some(t) = payload.get("tier").and_then(Value::as_str) {
                    if let Some(prev) = &last_tier
                        && prev != t
                    {
                        side.snapshot_crossing += 1;
                    }
                    last_tier = Some(t.to_string());
                }
            }
            _ => {}
        }
    }
    if run_id.is_empty() {
        run_id = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
    }
    let mut steps: Vec<RoutedStep> = Vec::new();
    let mut last_tool_t: Option<f64> = None;
    let mut seq_next = 0u64;
    for line in text.lines() {
        let Ok(event) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(ts) = event
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_time)
        else {
            continue;
        };
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "model_output" => {
                let calls = payload.get("tool_calls").and_then(Value::as_array);
                if calls.map(|c| !c.is_empty()).unwrap_or(false) {
                    seq_next += 1;
                    steps.push(RoutedStep {
                        t: ts,
                        legacy_feed: ToolEvent::other(None),
                        s2_feed: ToolEvent::other(None),
                        legacy: LegacyDispatch::default(),
                        s2: S2Dispatch {
                            neutral: true,
                            ..S2Dispatch::default()
                        },
                        decision: true,
                        seq: seq_next,
                    });
                }
            }
            "tool_completed" => {
                let tool = payload.get("tool").and_then(Value::as_str).unwrap_or("");
                let call_id = payload.get("call_id").and_then(Value::as_str);
                let command = call_id.and_then(|id| commands.get(id)).map(|s| s.as_str());
                let class = ActionClass::of_tool(tool, command);
                let blocked = call_id.map(|id| wcr_block.contains(id)).unwrap_or(false);
                // 拒绝类解析（喂入点口径，与生产一致）：信封标记／结构化
                // 拒绝码／写控兜底块。
                let deny_class = if payload.get("policy_denial").is_some() {
                    Some(DenyClass::PolicyMarker)
                } else if let Some(code) = payload.get("error").and_then(Value::as_str)
                    && is_denial_code(code)
                {
                    Some(DenyClass::of_code(code))
                } else if blocked {
                    Some(DenyClass::WriteControl)
                } else {
                    None
                };
                let (mut legacy, mut d, legacy_feed, s2_feed) = dispatch_event(
                    &payload,
                    class,
                    deny_class == Some(DenyClass::WriteControl),
                    deny_class,
                );
                let stall = last_tool_t.map(|prev| ts - prev > 90.0).unwrap_or(false);
                if stall {
                    legacy.claims.push(("stall", 1.0));
                    d.claims.push(("stall", 1.0));
                }
                if d.claims.is_empty() && !d.verify_pass {
                    d.neutral = true;
                }
                last_tool_t = Some(ts);
                seq_next += 1;
                steps.push(RoutedStep {
                    t: ts,
                    legacy_feed,
                    s2_feed,
                    legacy,
                    s2: d,
                    decision: false,
                    seq: seq_next,
                });
            }
            _ => {}
        }
    }
    (run_id, steps, side)
}

/// S3 每轮行：双引擎域＋RLI 锚点（窗口取 legacy 馈——两馈共用同一实际）。
struct RoutedRow {
    rli_domain: Domain,
    lif1d_domain: Domain,
    anchors: BTreeMap<ChannelKind, RliAnchors>,
    window_negative: u64,
    window_success: u64,
    /// S2 语义进度窗：窗内「变更类成功」（durable 工件变更）计数。
    window_mutate: u64,
    window_outcomes: u64,
}

struct RoutedReplay {
    rows: Vec<RoutedRow>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Feed {
    Legacy,
    S2,
}

fn replay_routed(steps: &[RoutedStep], feed: Feed) -> RoutedReplay {
    let mut engine = LifEngine::new();
    engine.enable_rli_shadow();
    let mut rows: Vec<RoutedRow> = Vec::new();
    let (mut w_neg, mut w_suc, mut w_out, mut w_mut) = (0u64, 0u64, 0u64, 0u64);
    let mut window_open = false;
    for step in steps {
        if step.decision {
            if window_open && let Some(row) = rows.last_mut() {
                row.window_negative = w_neg;
                row.window_success = w_suc;
                row.window_mutate = w_mut;
                row.window_outcomes = w_out;
            }
            w_neg = 0;
            w_suc = 0;
            w_out = 0;
            w_mut = 0;
            window_open = true;
            engine.on_decision_round(step.t);
            let t_hat = engine.rli_shadow().map(|s| s.t_hat()).unwrap_or(8.0);
            let mut anchors = BTreeMap::new();
            if let Some(shadow) = engine.rli_shadow() {
                for &kind in &RLI_CHANNELS {
                    anchors.insert(kind, shadow.channel(kind).anchors(t_hat));
                }
            }
            let rli_domain = engine
                .rli_shadow()
                .and_then(|s| s.domain().now().copied())
                .map(|r| r.domain)
                .unwrap_or(Domain::Start);
            let lif1d_domain = engine.temporal().current_domain();
            rows.push(RoutedRow {
                rli_domain,
                lif1d_domain,
                anchors,
                window_negative: 0,
                window_success: 0,
                window_mutate: 0,
                window_outcomes: 0,
            });
        } else {
            // 窗口恒取 legacy 结果（实际动作结果 ground truth，两馈一致）。
            match step.legacy_feed.outcome {
                ToolOutcome::Error | ToolOutcome::Deny => w_neg += 1,
                ToolOutcome::Success => w_suc += 1,
                ToolOutcome::Other => {}
            }
            w_out += 1;
            if step.legacy_feed.outcome == ToolOutcome::Success
                && step.s2.class == ActionClass::Mutate
            {
                w_mut += 1;
            }
            let ev = match feed {
                Feed::Legacy => step.legacy_feed,
                Feed::S2 => step.s2_feed,
            };
            engine.on_tool_event(step.t, ev);
        }
    }
    if window_open && let Some(row) = rows.last_mut() {
        row.window_negative = w_neg;
        row.window_success = w_suc;
        row.window_mutate = w_mut;
        row.window_outcomes = w_out;
    }
    RoutedReplay { rows }
}

fn series_stats(vals: &[f64]) -> Value {
    if vals.is_empty() {
        return json!({ "n": 0 });
    }
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pick = |q: f64| -> f64 {
        let idx = ((sorted.len() as f64 - 1.0) * q).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    };
    let n = sorted.len() as f64;
    let ge = |t: f64| sorted.iter().filter(|v| **v >= t).count();
    json!({
        "n": sorted.len(),
        "mean": mean(vals),
        "median": pick(0.5),
        "p90": pick(0.9),
        "max": sorted[sorted.len() - 1],
        "ge_0_9": ge(0.9),
        "ge_0_975": ge(0.975),
        "ratio_ge_0_975": ge(0.975) as f64 / n,
    })
}

/// `--s3` 入口：J1/J2 路由账目 ＋ J3 死窗 ＋ J4 新旧馈对比 ＋ J5 双引擎同馈。
fn run_s3(root: &Path, out: &Path) {
    let journals = find_journals(root);
    let mut run_reports: Vec<Value> = Vec::new();
    let mut legacy_channel_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut s2_channel_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut side_totals = SideClaims::default();
    let mut reclass_totals: BTreeMap<String, u64> = BTreeMap::new();
    let mut tool_completed_total = 0u64;
    let mut feeds: BTreeMap<String, BTreeMap<String, Vec<f64>>> = BTreeMap::new();
    let mut agree_totals: BTreeMap<&'static str, (u64, u64)> = BTreeMap::new();
    let mut window_agree: BTreeMap<&'static str, (u64, u64, u64, u64, u64, u64, u64, u64)> =
        BTreeMap::new();

    for journal in &journals {
        let (run_id, steps, side) = parse_routed(journal);
        if steps.is_empty() {
            continue;
        }
        side_totals.ctx_compressed += side.ctx_compressed;
        side_totals.fold_advance += side.fold_advance;
        side_totals.fold_write_failed += side.fold_write_failed;
        side_totals.transport_retry += side.transport_retry;
        side_totals.availability += side.availability;
        side_totals.host_denied += side.host_denied;
        side_totals.limit_hit += side.limit_hit;
        side_totals.snapshot_crossing += side.snapshot_crossing;
        side_totals.snapshots_seen += side.snapshots_seen;
        let mut run_legacy: BTreeMap<String, u64> = BTreeMap::new();
        let mut run_s2: BTreeMap<String, u64> = BTreeMap::new();
        let mut verify_pass = 0u64;
        let mut neutral = 0u64;
        let mut reclass: BTreeMap<String, u64> = BTreeMap::new();
        let mut wc_joins: Vec<Value> = Vec::new();
        let mut verify_slow_cut: Vec<Value> = Vec::new();
        for s in &steps {
            if s.decision {
                continue;
            }
            tool_completed_total += 1;
            for (ch, _) in &s.legacy.claims {
                *run_legacy.entry(ch.to_string()).or_default() += 1;
            }
            for (ch, _) in &s.s2.claims {
                *run_s2.entry(ch.to_string()).or_default() += 1;
            }
            if s.s2.verify_pass {
                verify_pass += 1;
            }
            if s.s2.claims.is_empty() && !s.s2.verify_pass {
                neutral += 1;
            }
            let lchs: Vec<&str> = s.legacy.claims.iter().map(|(c, _)| *c).collect();
            let schs: Vec<&str> = s.s2.claims.iter().map(|(c, _)| *c).collect();
            for ch in &lchs {
                if !schs.contains(ch) {
                    *reclass.entry(format!("{ch}:removed")).or_default() += 1;
                }
            }
            for ch in &schs {
                if !lchs.contains(ch) {
                    *reclass.entry(format!("{ch}:added")).or_default() += 1;
                }
            }
            if s.s2.has("deny") && s.legacy.has("err") && s.s2.deny_class == Some("write_control") {
                wc_joins.push(json!({ "run": run_id, "seq": s.seq, "t": s.t }));
            }
            if s.s2.class == ActionClass::Verify && s.legacy.has("slow") {
                verify_slow_cut.push(json!({
                    "run": run_id, "seq": s.seq, "t": s.t,
                    "wall_ms": s.legacy_feed.wall_ms,
                    "legacy_claims": lchs,
                }));
            }
        }
        for (k, v) in run_legacy.iter() {
            *legacy_channel_totals.entry(k.clone()).or_default() += v;
        }
        for (k, v) in run_s2.iter() {
            *s2_channel_totals.entry(k.clone()).or_default() += v;
        }
        for (k, v) in reclass.iter() {
            *reclass_totals.entry(k.clone()).or_default() += v;
        }
        // J4/J5：双馈真实引擎重放（窗口恒 legacy 实际）。
        for (fkey, feed) in [("legacy", Feed::Legacy), ("s2", Feed::S2)] {
            let replay = replay_routed(&steps, feed);
            let (mut p_ok, mut g_ok, mut j_ok, mut wins) = (0u64, 0u64, 0u64, 0u64);
            let (mut g2_ok, mut j2_ok) = (0u64, 0u64);
            let (mut lp_claims, mut lp_hits) = (0u64, 0u64);
            let (mut ag, mut win) = (0u64, 0u64);
            for row in &replay.rows {
                let agree = row.rli_domain.as_str() == row.lif1d_domain.as_str();
                win += 1;
                if agree {
                    ag += 1;
                }
                for &kind in &RLI_CHANNELS {
                    if let Some(a) = row.anchors.get(&kind) {
                        feeds
                            .entry(fkey.to_string())
                            .or_default()
                            .entry(channel_key(kind))
                            .or_default()
                            .push(a.u);
                    }
                }
                if row.window_outcomes == 0 {
                    continue;
                }
                wins += 1;
                let pc = matches!(row.rli_domain, Domain::Pressure | Domain::Stuck);
                let lc = matches!(row.rli_domain, Domain::LowProgress | Domain::Stuck);
                let pa = row.window_negative > 0;
                let la = row.window_success == 0;
                let pok = pc == pa;
                let lok = lc == la;
                if pok {
                    p_ok += 1;
                }
                if lok {
                    g_ok += 1;
                }
                if pok && lok {
                    j_ok += 1;
                }
                // S2 语义口径：进度实际＝窗内变更类成功（durable 工件变更）。
                let la_s2 = row.window_mutate == 0;
                if lc {
                    lp_claims += 1;
                    if la_s2 {
                        lp_hits += 1;
                    }
                }
                if lc == la_s2 {
                    g2_ok += 1;
                }
                if pok && lc == la_s2 {
                    j2_ok += 1;
                }
            }
            let e = agree_totals.entry(fkey).or_default();
            e.0 += ag;
            e.1 += win;
            let prev = window_agree
                .get(fkey)
                .copied()
                .unwrap_or((0, 0, 0, 0, 0, 0, 0, 0));
            *window_agree.entry(fkey).or_default() = (
                prev.0 + p_ok,
                prev.1 + g_ok,
                prev.2 + j_ok,
                prev.3 + wins,
                prev.4 + g2_ok,
                prev.5 + j2_ok,
                prev.6 + lp_claims,
                prev.7 + lp_hits,
            );
        }
        run_reports.push(json!({
            "run_id": run_id,
            "tool_completed": steps.iter().filter(|s| !s.decision).count() as u64,
            "legacy_channel_claims": run_legacy,
            "s2_channel_claims": run_s2,
            "verify_pass_zero": verify_pass,
            "neutral_zero": neutral,
            "side_claims": {
                "ctx": side.ctx(),
                "infra": side.infra(),
                "detail": serde_json::json!({
                    "compressed": side.ctx_compressed,
                    "fold_advance": side.fold_advance,
                    "fold_write_failed": side.fold_write_failed,
                    "transport_retry": side.transport_retry,
                    "availability": side.availability,
                    "host_denied": side.host_denied,
                    "limit_hit": side.limit_hit,
                    "snapshot_crossing": side.snapshot_crossing,
                    "snapshots_seen": side.snapshots_seen,
                }),
            },
            "reclass": reclass,
            "j2a_write_control_deny_joins": wc_joins,
            "j2b_verify_slow_excluded": verify_slow_cut,
        }));
    }

    let report = json!({
        "generated_by": "rli_shadow_replay --s3 (0am S3, 2026-10-03; S2 routing table J1-J5 replay)",
        "root": root.display().to_string(),
        "journals": journals.len(),
        "j1_j2_router_ledger": {
            "tool_completed_total": tool_completed_total,
            "legacy_channel_totals": legacy_channel_totals,
            "s2_tool_channel_totals": s2_channel_totals,
            "s2_ctx_totals": { "compressed": side_totals.ctx_compressed, "fold_advance": side_totals.fold_advance, "total": side_totals.ctx() },
            "s2_infra_totals": { "transport_retry": side_totals.transport_retry, "availability": side_totals.availability, "host_denied": side_totals.host_denied, "limit_hit": side_totals.limit_hit, "snapshot_crossing": side_totals.snapshot_crossing, "snapshots_seen": side_totals.snapshots_seen, "fold_write_failed": side_totals.fold_write_failed, "total": side_totals.infra() },
            "reclass_legacy_to_s2": reclass_totals,
        },
        "j3_dead_windows": {
            "note": "零输入通道＝基线休眠（无自激由引擎构造保证）；此处登记 Ctx/Infra 输入计数备核对",
            "ctx_inputs": side_totals.ctx(),
            "infra_inputs": side_totals.infra(),
        },
        "j2_cases": {
            "write_control_deny_joins": run_reports.iter().map(|r| r["j2a_write_control_deny_joins"].clone()).collect::<Vec<_>>(),
            "verify_slow_excluded": run_reports.iter().map(|r| r["j2b_verify_slow_excluded"].clone()).collect::<Vec<_>>(),
        },
        "j4_j5_feed_contrast": {
            "channel_u_series": feeds.iter().map(|(f, chs)| {
                let per: BTreeMap<String, Value> = chs.iter().map(|(k, v)| (k.clone(), series_stats(v))).collect();
                json!({ "feed": f, "channels": per })
            }).collect::<Vec<_>>(),
            "lif1d_rli_agreement": agree_totals.iter().map(|(k, (a, w))| json!({ "feed": k, "agree": a, "windows": w, "ratio": if *w > 0 { *a as f64 / *w as f64 } else { f64::NAN } })).collect::<Vec<_>>(),
            "window_agreement_vs_legacy_actuals": window_agree.iter().map(|(k, (p, g, j, w, g2, j2, lpc, lph))| json!({ "feed": k, "pressure_ok": p, "progress_ok_legacy_semantics": g, "joint_ok_legacy_semantics": j, "progress_ok_s2_semantics": g2, "joint_ok_s2_semantics": j2, "low_progress_claims": lpc, "low_progress_hits_s2_semantics": lph, "windows": w })).collect::<Vec<_>>(),
        },
        "runs": run_reports,
    });
    std::fs::write(out, serde_json::to_string_pretty(&report).unwrap()).expect("write s3 report");
    println!("s3 routing replay written to {}", out.display());
}

/// `--s3-selftest`：标签器与分派的最小行为钉（合成事件，不跑语料）。
fn s3_selftest() {
    use orz_assurance::lif::ActionClass as AC;
    use orz_assurance::lif::DenyClass as DC;
    use orz_assurance::lif::ToolOutcome as TO;
    // 词表匹配：复合命令命中、词界防误命中。
    assert_eq!(
        AC::of_tool(
            "run_terminal_cmd",
            Some("cd /workspace && .venv/bin/python -m unittest discover -s tests")
        ),
        AC::Verify
    );
    assert_eq!(
        AC::of_tool("run_terminal_cmd", Some("cat Makefile")),
        AC::Neutral
    );
    assert_eq!(
        AC::of_tool("run_terminal_cmd", Some("cargo test --lib")),
        AC::Verify
    );
    assert_eq!(AC::of_tool("search_replace", None), AC::Mutate);
    assert_eq!(AC::of_tool("read_file", None), AC::Retrieve);
    // 验证通过（61s）：legacy prog+slow → S2 零注入（slow 豁免；lib 分派）。
    let p = serde_json::json!({ "tool": "run_terminal_cmd", "exit_code": 0, "wall_ms": 61_000 });
    let (legacy, d, _, _) = dispatch_event(&p, AC::Verify, false, None);
    assert!(legacy.has("prog") && legacy.has("slow"));
    assert!(d.verify_pass && d.claims.is_empty());
    // 验证失败：verify 注入，不串 err。
    let p = serde_json::json!({ "tool": "run_terminal_cmd", "exit_code": 2, "wall_ms": 100 });
    let (_, d, _, _) = dispatch_event(&p, AC::Verify, false, None);
    assert!(d.has("verify") && !d.has("err"));
    // H2 填平：非验证类非零退出（legacy 黑洞）→ err。
    let p = serde_json::json!({ "tool": "run_terminal_cmd", "exit_code": 1, "wall_ms": 10 });
    let (legacy, d, _, _) = dispatch_event(&p, AC::Neutral, false, None);
    assert!(!legacy.has("err"));
    assert!(d.has("err"));
    // 写控块 join：legacy err（无码 status=error）→ S2 deny(write_control)。
    let p = serde_json::json!({ "tool": "run_terminal_cmd", "status": "error", "error": "command blocked by the mechanical write control backstop (rule: raw-device-write)" });
    let (legacy, d, _, _) = dispatch_event(&p, AC::Neutral, true, Some(DC::WriteControl));
    assert!(legacy.has("err"));
    assert!(d.has("deny") && d.deny_class == Some(deny_key(DC::WriteControl)));
    assert!(!d.has("err"));
    // 结构化拒绝码（信封外）：deny＋标签维（TO 未用导入保护）。
    let _ = (TO::Other, DC::PolicyMarker);
    println!("s3 routing selftest ok (lib router + dispatch + J2 cases)");
}
