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
//! The err predicate is the PRODUCTION predicate (R1 ruling, 2026-08-31):
//! timeouts and host-level errors (status=error without an exit_code value)
//! are Error; exit_code 0 is Success; a non-zero exit_code is a D2 VALUE
//! and stays neutral (Other) — mirroring `host_exec.rs`'s LIF feed exactly.
//! - C1 time-shuffle (10 seeded permutations) on the err channel — fires
//!   change on runs where time structure (not pure counting) is at work.
//!
//! Usage: `cargo run -p orz-assurance --example lif_replay -- [root] [out.json] [err_round_k]`
//! The report is also printed as a compact JSON object to stdout.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::DateTime;
use orz_assurance::lif::{
    Domain, ErrTauMode, LifEngine, ToolEvent, ToolOutcome,
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
            } else if path.file_name().map(|n| n == "events.jsonl").unwrap_or(false) {
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

fn outcome_of(payload: &Value) -> ToolOutcome {
    // Production predicate (R1, 2026-08-31): timeout and host-level error
    // (status=error with no exit_code value) → Error; exit 0 → Success;
    // exit≠0 is a D2 structured VALUE → Other (neutral).
    if payload.get("timed_out").and_then(Value::as_bool) == Some(true) {
        return ToolOutcome::Error;
    }
    if payload.get("status").and_then(Value::as_str) == Some("error")
        && payload.get("exit_code").is_none()
    {
        return ToolOutcome::Error;
    }
    match payload.get("exit_code").and_then(Value::as_i64) {
        Some(0) => ToolOutcome::Success,
        Some(_) => ToolOutcome::Other,
        None => ToolOutcome::Other,
    }
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
        let Some(ts) = event.get("timestamp").and_then(Value::as_str).and_then(parse_time) else {
            continue;
        };
        if first_t.is_none() {
            first_t = Some(ts);
        }
        let event_type = event.get("event_type").and_then(Value::as_str).unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        match event_type {
            "model_output" => {
                let calls = payload.get("tool_calls").and_then(Value::as_array);
                if calls.map(|c| !c.is_empty()).unwrap_or(false) {
                    steps.push(Step::Decision(ts));
                }
            }
            "tool_completed" => {
                let outcome = outcome_of(&payload);
                let wall_ms = wall_ms_of(&payload);
                steps.push(Step::Tool(ts, ToolEvent { outcome, wall_ms }));
            }
            _ => {}
        }
    }
    let run_id = text
        .lines()
        .find_map(|l| {
            serde_json::from_str::<Value>(l)
                .ok()
                .and_then(|v| v.get("run_id").and_then(Value::as_str).map(|s| s.to_string()))
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
    for step in steps {
        match *step {
            Step::Decision(t) => {
                engine.on_decision_round(t);
                t_hats.push(engine.estimator().estimate());
                let domain = engine.temporal().now().map(|r| r.domain).unwrap_or(Domain::Start);
                *domain_counts.entry(domain.as_str().to_string()).or_insert(0) += 1;
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

    // C1: 10 seeded permutations of event intervals on the err channel.
    let mut perm_counts = Vec::new();
    let base_err = engine.err().fire_count();
    if base_err > 0 {
        let times: Vec<f64> = steps
            .iter()
            .map(|s| match *s {
                Step::Decision(t) => t,
                Step::Tool(t, _) => t,
            })
            .collect();
        let intervals: Vec<f64> = times.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect();
        for perm in 0_u64..10 {
            let mut rng = Xorshift64(0x9E3779B97F4A7C15 ^ perm.wrapping_mul(0x100000001B3));
            let mut shuffled = intervals.clone();
            // Fisher–Yates over the interval list.
            for i in (1..shuffled.len()).rev() {
                let j = (rng.next_f64() * (i + 1) as f64).floor() as usize;
                shuffled.swap(i, j);
            }
            let mut t = times[0];
            let mut perm_steps = Vec::with_capacity(steps.len());
            let mut idx = 0;
            for (i, step) in steps.iter().enumerate() {
                if i > 0 {
                    t += shuffled[idx];
                    idx += 1;
                }
                match *step {
                    Step::Decision(_) => perm_steps.push(Step::Decision(t)),
                    Step::Tool(_, ev) => perm_steps.push(Step::Tool(t, ev)),
                }
            }
            let p = replay(&perm_steps, ErrTauMode::FixedSecs(180.0));
            perm_counts.push(p.err().fire_count());
        }
    }

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
        err_fires_permutation: perm_counts,
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

    // C1 aggregate: of the runs with ≥1 fixed-time err fire, how many changed
    // their fire count under ≥1 of the 10 permutations.
    let c1_changed = runs
        .iter()
        .filter(|r| r.err_fires > 0)
        .filter(|r| r.err_fires_permutation.iter().any(|c| *c != r.err_fires))
        .count();
    let c1_unchanged = runs_with_err.saturating_sub(c1_changed);

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
            "c1_changed": c1_changed,
            "c1_unchanged": c1_unchanged,
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
