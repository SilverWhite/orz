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
use orz_assurance::lif::{
    ChannelKind, Domain, LifEngine, RLI_CHANNELS, RLI_PREDICTION_STEPS, RliAnchors, ToolEvent,
    classify_event_outcome,
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
                steps.push(Step::Tool(ts, ToolEvent { outcome, wall_ms }));
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
        }
        windows += run_windows;
        pressure_agrees += run_pressure_agrees;
        progress_agrees += run_progress_agrees;
        joint_agrees += run_joint_agrees;

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
