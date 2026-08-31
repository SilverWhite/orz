//! Offline replay harness for the LIF temporal sidecar (P2-10 I1).
//!
//! Walks a job root for `agent/gsa/runs/RUN-*/events.jsonl` (the v0.2 event
//! chain — the 102-run corpus lives at `D:\tb-eval\jobs-official`), replays
//! every run through `orz_assurance::lif::LifEngine`, and emits:
//! - per-run feature/fire/domain statistics;
//! - aggregate anchors from the design's first-pass analysis (§9.6.5):
//!   err fixed-time vs round-semantics fires (22 run/24 fires vs 11 run/8 —
//!   first-pass per-run predicate), stuck θ=1.5·T̂ 0/102 fires + peak θ
//!   ratio, decision-point total (4157);
//!
//! The outcome predicate is the PRODUCTION predicate (R1/R2 rulings,
//! 2026-08-31): structured rejections (policy_denial marker or denial code)
//! are Deny; timeouts and host-level errors (status=error without an
//! exit_code value) are Error; exit_code 0 is Success; a non-zero exit_code
//! is a D2 VALUE and stays neutral (Other) — mirroring `host_exec.rs`'s LIF
//! feed exactly via the shared `classify_event_outcome`.
//! - C1 time-shuffle (10 seeded permutations) on the err channel — fires
//!   change on runs where time structure (not pure counting) is at work.
//!
//! Usage: `cargo run -p orz-assurance --example lif_replay -- [root] [out.json] [err_round_k]`
//! The report is also printed as a compact JSON object to stdout.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::DateTime;
use orz_assurance::lif::{
    DENY_REFRACTORY_SECS, DENY_TAU_SECS, DENY_THETA, ERR_REFRACTORY_SECS, ERR_TAU_SECS, ERR_THETA,
    ErrTauMode, LifEngine, SLOW_REFRACTORY_SECS, SLOW_TAU_SECS, SLOW_THETA, SLOW_W_MAX,
    SLOW_WALL_MS_THRESHOLD, STALL_GAP_THRESHOLD_SECS, STALL_REFRACTORY_SECS, STALL_TAU_SECS,
    STALL_THETA, ToolEvent, ToolOutcome, classify_event_outcome,
};
use serde_json::Value;

/// Tiny deterministic xorshift64 PRNG (C1 permutations only — no new deps).
struct Xorshift64(u64);

impl Xorshift64 {
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

/// 四对照门参考动力学（V2, 2026-08-31）——对同一事件流用同一更新律重放
/// 三个变体：
/// - `fires`：生产 LIF（复位 + 不应期），与引擎输出做确定性对拍；
/// - `membrane`：C2≡C4 参考——无复位、无不应期的膜电位（单极点下 ≡ 一阶
///   EMA），记录 u ≥ θ 的向上跨阈时刻；
/// - `count_fires`：C3 参考——τ→∞（无泄漏）纯计数，保留复位 + 不应期，
///   记录触发时刻。
struct RefDynamics {
    fires: Vec<f64>,
    membrane: Vec<f64>,
    count_fires: Vec<f64>,
}

/// 单通道四对照门结论。
#[derive(serde::Serialize, Clone)]
struct GateReference {
    /// 参考 LIF 与引擎 fires 完全一致（确定性对拍）。
    ref_matches_engine: bool,
    /// C2/C4：膜电位跨阈次数（无复位/无不应期 = EMA）。
    membrane_crossings: usize,
    /// C3：τ→∞ 纯计数触发次数。
    count_fires: usize,
    /// C2/C4 通过：fires 时机与膜电位跨阈不同（阈值事件结构在起作用）。
    c2c4_fires_differ_from_membrane: bool,
    /// C3 通过：fires 与纯计数触发不同（泄漏/时间窗在起作用）。
    c3_fires_differ_from_counting: bool,
}

/// 逐决策轮特征行（V2 聚类对照，§9.8）。
#[derive(serde::Serialize, Clone)]
struct FeatureRow {
    t: f64,
    domain: String,
    u_prog: f64,
    u_err: f64,
    u_stuck: f64,
    t_hat: f64,
    err10: f64,
    succ10: f64,
}

type WeightFn = dyn Fn(f64, Option<&ToolEvent>, Option<f64>) -> f64;
type FiresFn = dyn Fn(&RunStats) -> usize;
type PermsFn = dyn Fn(&RunStats) -> &[usize];
type GatesFn = dyn Fn(&RunStats) -> &GateReference;

fn err_weight(_t: f64, ev: Option<&ToolEvent>, _prev_tool_t: Option<f64>) -> f64 {
    match ev {
        Some(ev) if ev.outcome == ToolOutcome::Error => 1.0,
        _ => 0.0,
    }
}

fn deny_weight(_t: f64, ev: Option<&ToolEvent>, _prev_tool_t: Option<f64>) -> f64 {
    match ev {
        Some(ev) if ev.outcome == ToolOutcome::Deny => 1.0,
        _ => 0.0,
    }
}

fn stall_weight(t: f64, ev: Option<&ToolEvent>, prev_tool_t: Option<f64>) -> f64 {
    // stall 仅在工具事件到达时探测间隔（§4.3）；决策轮只衰减不喂入。
    if ev.is_none() {
        return 0.0;
    }
    match prev_tool_t {
        Some(prev) if (t - prev) > STALL_GAP_THRESHOLD_SECS => 1.0,
        _ => 0.0,
    }
}

fn slow_weight(_t: f64, ev: Option<&ToolEvent>, _prev_tool_t: Option<f64>) -> f64 {
    match ev {
        Some(ev) => match ev.wall_ms {
            Some(ms) if ms > SLOW_WALL_MS_THRESHOLD => {
                ((ms as f64) / 60_000.0).clamp(1.0, SLOW_W_MAX)
            }
            _ => 0.0,
        },
        None => 0.0,
    }
}

/// 用同一更新律对事件流重放三个参考变体（见 `RefDynamics`）。
fn reference_dynamics(
    steps: &[Step],
    weight: &WeightFn,
    tau: f64,
    theta: f64,
    refractory: f64,
) -> RefDynamics {
    let mut u = 0.0;
    let mut u_mem = 0.0;
    let mut u_cnt = 0.0;
    let mut last_t: Option<f64> = None;
    let mut last_fire_until: Option<f64> = None;
    let mut last_cnt_until: Option<f64> = None;
    let mut prev_mem: f64 = 0.0;
    let mut prev_tool_t: Option<f64> = None;
    let mut fires = Vec::new();
    let mut membrane = Vec::new();
    let mut count_fires = Vec::new();
    // 引擎内部时间以 run 起点归一（`LifEngine::rel`）——参考模拟用同一原点，
    // 保证 fires/跨阈时刻与引擎同基可比（parity 对拍）。
    let origin = steps
        .first()
        .map(|s| match *s {
            Step::Decision(t) => t,
            Step::Tool(t, _) => t,
        })
        .unwrap_or(0.0);
    for step in steps {
        let t = (match *step {
            Step::Decision(t) => t,
            Step::Tool(t, _) => t,
        }) - origin;
        if let Some(t0) = last_t {
            let dt = (t - t0).max(0.0);
            if tau.is_finite() && dt > 0.0 {
                let k = (-dt / tau).exp();
                u *= k;
                u_mem *= k;
            }
        }
        last_t = Some(t);
        let w = match step {
            Step::Decision(_) => weight(t, None, prev_tool_t),
            Step::Tool(_, ev) => weight(t, Some(ev), prev_tool_t),
        };
        if let Step::Tool(_, _) = step {
            prev_tool_t = Some(t);
        }
        if w > 0.0 {
            u += w;
            u_mem += w;
            u_cnt += w;
        }
        let in_ref = matches!(last_fire_until, Some(until) if t < until);
        if u >= theta && !in_ref {
            fires.push(t);
            u = 0.0;
            last_fire_until = Some(t + refractory);
        }
        if u_mem >= theta && prev_mem < theta {
            membrane.push(t);
        }
        prev_mem = u_mem;
        let in_cnt_ref = matches!(last_cnt_until, Some(until) if t < until);
        if u_cnt >= theta && !in_cnt_ref {
            count_fires.push(t);
            u_cnt = 0.0;
            last_cnt_until = Some(t + refractory);
        }
    }
    RefDynamics {
        fires,
        membrane,
        count_fires,
    }
}

/// 单通道四对照门 C2/C3（C2≡C4：单极点下无复位电位 = 一阶 EMA）。
fn gate_reference(
    steps: &[Step],
    engine_fires: &[f64],
    weight: &WeightFn,
    tau: f64,
    theta: f64,
    refractory: f64,
) -> GateReference {
    let r = reference_dynamics(steps, weight, tau, theta, refractory);
    GateReference {
        ref_matches_engine: r.fires == engine_fires,
        membrane_crossings: r.membrane.len(),
        count_fires: r.count_fires.len(),
        c2c4_fires_differ_from_membrane: r.fires != r.membrane,
        c3_fires_differ_from_counting: r.fires != r.count_fires,
    }
}

/// C1 时间打乱：保留事件类型/载荷序列，仅重排事件间隔（10 次种子置换）。
/// 与原 err 通道 C1 种子公式一致，保证既有证据可复现。
fn shuffled_interval_steps(steps: &[Step], perm_seed: u64) -> Vec<Step> {
    let times: Vec<f64> = steps
        .iter()
        .map(|s| match *s {
            Step::Decision(t) => t,
            Step::Tool(t, _) => t,
        })
        .collect();
    let intervals: Vec<f64> = times.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();
    let mut rng = Xorshift64(0x9E3779B97F4A7C15 ^ perm_seed.wrapping_mul(0x100000001B3));
    let mut shuffled = intervals.clone();
    for i in (1..shuffled.len()).rev() {
        let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
        shuffled.swap(i, j);
    }
    let mut t = times[0];
    let mut out = Vec::with_capacity(steps.len());
    let mut idx = 0;
    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            t += shuffled[idx];
            idx += 1;
        }
        match *step {
            Step::Decision(_) => out.push(Step::Decision(t)),
            Step::Tool(_, ev) => out.push(Step::Tool(t, ev)),
        }
    }
    out
}

fn perm_counts(steps: &[Step], extract: &dyn Fn(&LifEngine) -> usize) -> Vec<usize> {
    (0_u64..10)
        .map(|perm| {
            let p = replay(
                &shuffled_interval_steps(steps, perm),
                ErrTauMode::FixedSecs(180.0),
            );
            extract(&p)
        })
        .collect()
}

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

/// One typed event in replay order.
enum Step {
    Decision(f64),
    Tool(f64, ToolEvent),
}

fn parse_run(path: &Path) -> (String, Vec<Step>) {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut steps = Vec::new();
    let mut first_t = None;
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
        if first_t.is_none() {
            first_t = Some(ts);
        }
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

struct RunStats {
    run_id: String,
    rounds: u64,
    tool_events: u64,
    errors: u64,
    successes: u64,
    err_fires: usize,
    err_first_fire: Option<f64>,
    stall_fires: usize,
    slow_fires: usize,
    deny_fires: usize,
    stuck_fires: usize,
    stuck_peak_ratio: f64,
    t_hat_last: f64,
    t_hat_median: f64,
    domain_counts: BTreeMap<String, u64>,
    spikes: usize,
    migrations: usize,
    err_fires_round: usize,
    err_fires_permutation: Vec<usize>,
    deny_fires_permutation: Vec<usize>,
    stall_fires_permutation: Vec<usize>,
    slow_fires_permutation: Vec<usize>,
    err_gates: GateReference,
    deny_gates: GateReference,
    stall_gates: GateReference,
    slow_gates: GateReference,
    stuck_membrane_crossed: bool,
    features: Vec<FeatureRow>,
}

fn replay(steps: &[Step], err_mode: ErrTauMode) -> LifEngine {
    let mut engine = LifEngine::new();
    engine.set_err_tau_mode(err_mode);
    for step in steps {
        match *step {
            Step::Decision(t) => engine.on_decision_round(t),
            Step::Tool(t, ev) => engine.on_tool_event(t, ev),
        }
    }
    engine
}

fn run_stats(run_id: String, steps: &[Step], err_round_k: f64) -> RunStats {
    let mut engine = LifEngine::new();
    let mut t_hats: Vec<f64> = Vec::new();
    let mut domain_counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut features: Vec<FeatureRow> = Vec::new();
    for step in steps {
        match *step {
            Step::Decision(t) => {
                engine.on_decision_round(t);
                t_hats.push(engine.estimator().estimate());
                let rec = engine.temporal().now().copied();
                if let Some(rec) = rec {
                    *domain_counts
                        .entry(rec.domain.as_str().to_string())
                        .or_insert(0) += 1;
                    features.push(FeatureRow {
                        t: rec.t,
                        domain: rec.domain.as_str().to_string(),
                        u_prog: rec.u_prog,
                        u_err: rec.u_err,
                        u_stuck: rec.u_stuck,
                        t_hat: rec.t_hat,
                        err10: rec.err10,
                        succ10: rec.succ10,
                    });
                }
            }
            Step::Tool(t, ev) => engine.on_tool_event(t, ev),
        }
    }
    let mut t_hat_sorted = t_hats.clone();
    t_hat_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let t_hat_median = if t_hat_sorted.is_empty() {
        0.0
    } else if t_hat_sorted.len() % 2 == 1 {
        t_hat_sorted[t_hat_sorted.len() / 2]
    } else {
        (t_hat_sorted[t_hat_sorted.len() / 2 - 1] + t_hat_sorted[t_hat_sorted.len() / 2]) / 2.0
    };

    // Round-semantics err counterfactual (design §9.6.5.6 anchor).
    let round_engine = replay(steps, ErrTauMode::Rounds(err_round_k));

    // C1: 10 seeded permutations of event intervals, per channel with fires.
    let err_perm = if engine.err().fire_count() > 0 {
        perm_counts(steps, &|e| e.err().fire_count())
    } else {
        Vec::new()
    };
    let deny_perm = if engine.deny().fire_count() > 0 {
        perm_counts(steps, &|e| e.deny().fire_count())
    } else {
        Vec::new()
    };
    let stall_perm = if engine.stall().fire_count() > 0 {
        perm_counts(steps, &|e| e.stall().fire_count())
    } else {
        Vec::new()
    };
    let slow_perm = if engine.slow().fire_count() > 0 {
        perm_counts(steps, &|e| e.slow().fire_count())
    } else {
        Vec::new()
    };

    // C2/C4（膜电位 ≡ EMA，无复位/无不应期）+ C3（τ→∞ 纯计数）参考重放。
    let err_gates = gate_reference(
        steps,
        engine.err().fire_times(),
        &err_weight,
        ERR_TAU_SECS,
        ERR_THETA,
        ERR_REFRACTORY_SECS,
    );
    let deny_gates = gate_reference(
        steps,
        engine.deny().fire_times(),
        &deny_weight,
        DENY_TAU_SECS,
        DENY_THETA,
        DENY_REFRACTORY_SECS,
    );
    let stall_gates = gate_reference(
        steps,
        engine.stall().fire_times(),
        &stall_weight,
        STALL_TAU_SECS,
        STALL_THETA,
        STALL_REFRACTORY_SECS,
    );
    let slow_gates = gate_reference(
        steps,
        engine.slow().fire_times(),
        &slow_weight,
        SLOW_TAU_SECS,
        SLOW_THETA,
        SLOW_REFRACTORY_SECS,
    );
    let stuck_peak_ratio = engine.stuck().peak_theta_ratio();

    RunStats {
        run_id,
        rounds: engine.temporal().round(),
        tool_events: engine.temporal().total_tool_events(),
        errors: engine.temporal().total_errors(),
        successes: engine.temporal().total_successes(),
        err_fires: engine.err().fire_count(),
        err_first_fire: engine.err().fire_times().first().copied(),
        stall_fires: engine.stall().fire_count(),
        slow_fires: engine.slow().fire_count(),
        deny_fires: engine.deny().fire_count(),
        stuck_fires: engine.stuck().fire_count(),
        stuck_peak_ratio: engine.stuck().peak_theta_ratio(),
        t_hat_last: t_hats.last().copied().unwrap_or(0.0),
        t_hat_median,
        domain_counts,
        spikes: engine.temporal().spikes().len(),
        migrations: engine.temporal().history().len(),
        err_fires_round: round_engine.err().fire_count(),
        err_fires_permutation: err_perm,
        deny_fires_permutation: deny_perm,
        stall_fires_permutation: stall_perm,
        slow_fires_permutation: slow_perm,
        err_gates,
        deny_gates,
        stall_gates,
        slow_gates,
        stuck_membrane_crossed: stuck_peak_ratio >= 1.0,
        features,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"D:\tb-eval\jobs-official"));
    let out_path = args.get(2).map(PathBuf::from);
    let err_round_k: f64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3.0);

    let journals = find_journals(&root);
    let mut runs = Vec::new();
    let mut total_decision_points = 0_u64;
    for path in &journals {
        let (run_id, steps) = parse_run(path);
        let stats = run_stats(run_id, &steps, err_round_k);
        total_decision_points += stats.rounds;
        runs.push(stats);
    }
    runs.sort_by(|a, b| a.run_id.cmp(&b.run_id));

    let runs_with_err = runs.iter().filter(|r| r.err_fires > 0).count();
    let total_err_fires: usize = runs.iter().map(|r| r.err_fires).sum();
    let runs_with_err_round = runs.iter().filter(|r| r.err_fires_round > 0).count();
    let total_err_fires_round: usize = runs.iter().map(|r| r.err_fires_round).sum();
    let runs_with_stuck = runs.iter().filter(|r| r.stuck_fires > 0).count();
    let peak_ratio = runs
        .iter()
        .map(|r| r.stuck_peak_ratio)
        .fold(0.0_f64, f64::max);
    let runs_with_slow = runs.iter().filter(|r| r.slow_fires > 0).count();
    let runs_with_stall = runs.iter().filter(|r| r.stall_fires > 0).count();
    let runs_with_deny = runs.iter().filter(|r| r.deny_fires > 0).count();

    // 四对照门聚合：对每个有 fires 的一阶通道统计
    // C1（10 置换下 fires 数/时机改变）、C2/C4（fires ≠ 膜电位跨阈）、
    // C3（fires ≠ τ→∞ 纯计数触发）以及参考对拍（确定性 parity）。
    let c1_stats = |fires: &FiresFn, perms: &PermsFn| -> (usize, usize, usize) {
        let with = runs.iter().filter(|r| fires(r) > 0).count();
        let changed = runs
            .iter()
            .filter(|r| fires(r) > 0)
            .filter(|r| perms(r).iter().any(|c| *c != fires(r)))
            .count();
        (with, changed, with.saturating_sub(changed))
    };
    let gate_stats = |fires: &FiresFn, g: &GatesFn| -> (usize, usize, usize, usize) {
        let with = runs.iter().filter(|r| fires(r) > 0).count();
        let c2c4_diff = runs
            .iter()
            .filter(|r| fires(r) > 0)
            .filter(|r| g(r).c2c4_fires_differ_from_membrane)
            .count();
        let c3_diff = runs
            .iter()
            .filter(|r| fires(r) > 0)
            .filter(|r| g(r).c3_fires_differ_from_counting)
            .count();
        let parity_fail = runs.iter().filter(|r| !g(r).ref_matches_engine).count();
        (with, c2c4_diff, c3_diff, parity_fail)
    };
    let (err_c1_with, err_c1_changed, err_c1_unchanged) =
        c1_stats(&|r| r.err_fires, &|r| &r.err_fires_permutation);
    let (deny_c1_with, deny_c1_changed, deny_c1_unchanged) =
        c1_stats(&|r| r.deny_fires, &|r| &r.deny_fires_permutation);
    let (stall_c1_with, stall_c1_changed, stall_c1_unchanged) =
        c1_stats(&|r| r.stall_fires, &|r| &r.stall_fires_permutation);
    let (slow_c1_with, slow_c1_changed, slow_c1_unchanged) =
        c1_stats(&|r| r.slow_fires, &|r| &r.slow_fires_permutation);
    let (_err_g_with, err_g_c2, err_g_c3, err_g_parity) =
        gate_stats(&|r| r.err_fires, &|r| &r.err_gates);
    let (_deny_g_with, deny_g_c2, deny_g_c3, deny_g_parity) =
        gate_stats(&|r| r.deny_fires, &|r| &r.deny_gates);
    let (_stall_g_with, stall_g_c2, stall_g_c3, stall_g_parity) =
        gate_stats(&|r| r.stall_fires, &|r| &r.stall_gates);
    let (_slow_g_with, slow_g_c2, slow_g_c3, slow_g_parity) =
        gate_stats(&|r| r.slow_fires, &|r| &r.slow_gates);
    let stuck_membrane_crossed_runs = runs.iter().filter(|r| r.stuck_membrane_crossed).count();
    let stuck_parity = runs
        .iter()
        .filter(|r| !r.stuck_peak_ratio.is_finite())
        .count();

    let report = serde_json::json!({
        "replay": {
            "harness": "orz_assurance::lif::LifEngine",
            "date": "2026-08-30",
            "root": root.display().to_string(),
            "runs": runs.len(),
            "total_decision_points": total_decision_points,
            "err_round_k": err_round_k,
        },
        "aggregate": {
            "err_fixed_runs": runs_with_err,
            "err_fixed_fires": total_err_fires,
            "err_round_runs": runs_with_err_round,
            "err_round_fires": total_err_fires_round,
            "round_subset_of_time": runs_with_err_round <= runs_with_err,
            "stuck_runs": runs_with_stuck,
            "stuck_peak_theta_ratio": peak_ratio,
            "slow_runs": runs_with_slow,
            "stall_runs": runs_with_stall,
            "deny_runs": runs_with_deny,
            "c1_changed": err_c1_changed,
            "c1_unchanged": err_c1_unchanged,
        },
        "controls": {
            "err": {
                "runs_with_fires": err_c1_with,
                "c1_changed": err_c1_changed,
                "c1_unchanged": err_c1_unchanged,
                "c2c4_fires_differ_from_membrane": err_g_c2,
                "c3_fires_differ_from_counting": err_g_c3,
                "reference_parity_fail_runs": err_g_parity,
            },
            "deny": {
                "runs_with_fires": deny_c1_with,
                "c1_changed": deny_c1_changed,
                "c1_unchanged": deny_c1_unchanged,
                "c2c4_fires_differ_from_membrane": deny_g_c2,
                "c3_fires_differ_from_counting": deny_g_c3,
                "reference_parity_fail_runs": deny_g_parity,
            },
            "stall": {
                "runs_with_fires": stall_c1_with,
                "c1_changed": stall_c1_changed,
                "c1_unchanged": stall_c1_unchanged,
                "c2c4_fires_differ_from_membrane": stall_g_c2,
                "c3_fires_differ_from_counting": stall_g_c3,
                "reference_parity_fail_runs": stall_g_parity,
            },
            "slow": {
                "runs_with_fires": slow_c1_with,
                "c1_changed": slow_c1_changed,
                "c1_unchanged": slow_c1_unchanged,
                "c2c4_fires_differ_from_membrane": slow_g_c2,
                "c3_fires_differ_from_counting": slow_g_c3,
                "reference_parity_fail_runs": slow_g_parity,
            },
            "stuck": {
                "runs_with_fires": runs_with_stuck,
                "peak_theta_ratio": peak_ratio,
                "membrane_crossed_runs": stuck_membrane_crossed_runs,
                "nonfinite_ratio_runs": stuck_parity,
            },
        },
        "runs": runs
            .iter()
            .map(|r| {
                serde_json::json!({
                    "run_id": r.run_id,
                    "rounds": r.rounds,
                    "tool_events": r.tool_events,
                    "errors": r.errors,
                    "successes": r.successes,
                    "err_fires": r.err_fires,
                    "err_first_fire": r.err_first_fire,
                    "err_fires_round": r.err_fires_round,
                    "stall_fires": r.stall_fires,
                    "slow_fires": r.slow_fires,
                    "deny_fires": r.deny_fires,
                    "stuck_fires": r.stuck_fires,
                    "stuck_peak_ratio": r.stuck_peak_ratio,
                    "t_hat_last": r.t_hat_last,
                    "t_hat_median": r.t_hat_median,
                    "domains": r.domain_counts,
                    "spikes": r.spikes,
                    "migrations": r.migrations,
                    "c1_permutations": r.err_fires_permutation,
                    "deny_c1_permutations": r.deny_fires_permutation,
                    "stall_c1_permutations": r.stall_fires_permutation,
                    "slow_c1_permutations": r.slow_fires_permutation,
                    "err_gates": r.err_gates,
                    "deny_gates": r.deny_gates,
                    "stall_gates": r.stall_gates,
                    "slow_gates": r.slow_gates,
                    "stuck_membrane_crossed": r.stuck_membrane_crossed,
                    "features": r.features,
                })
            })
            .collect::<Vec<_>>(),
    });

    if let Some(out) = out_path {
        std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).expect("write report");
        eprintln!("report written to {}", out.display());
    }
    println!("{}", serde_json::to_string(&report).unwrap());
}
