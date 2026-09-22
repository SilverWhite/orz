//! RLI 前推对拍探针（0bc 后研究件，2026-09-21）。
//!
//! 0bd ⑩ 收编（2026-09-22）：原工程在工作区外 `D:\tb-eval\rli-forecast-probe`
//! （手改需外部脚本；0be 轮非 ASCII 注释曾发生编码粘连）——本文件为其完整
//! 迁入（example 形态；迁入时源文件 sha256 db2ec197c253…）。用法（先把语料
//! 冻结，0bd ⑪）：
//!   cargo run --release --example rli_forecast_probe -- <journal-root>
//!   （可选 `RLI_PROBE_OUT` 指定产物 JSON 路径；冻结副本经
//!   `scripts/freeze_rli_corpus.ps1` 生成，对拍一律对冻结副本做）
//!
//! 问题：`pred(10·T̂)` 与「实际发生」的符合度如何？以及不同 horizon 的表现。
//! 做法：用生产同一套 API 回放 journal（与 `rli_shadow_replay.rs` 同形），
//! 在每个决策轮记下各通道状态与前推到 h·T̂（h ∈ {1,2,5,10,30}）的预测值；
//! 之后用「该 run 后续第一个 ≥ t+h·T̂ 的采样点上的真实 u」作为对照，
//! 并同时给出**持久性基线**（"假设 u 不变"）——预测若打不过它就没有信息量。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::DateTime;
use orz_assurance::lif::{
    ChannelKind, LifEngine, RLI_CHANNELS, ToolEvent,
    classify_event_outcome,
};
use serde_json::{Value, json};

const HORIZONS: [f64; 5] = [1.0, 2.0, 5.0, 10.0, 30.0];

enum Step {
    Decision(f64),
    Tool(f64, ToolEvent),
}

fn parse_time(s: &str) -> Option<f64> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp_millis() as f64 / 1000.0)
}

fn wall_ms_of(payload: &Value) -> Option<u64> {
    payload.get("wall_ms").and_then(Value::as_u64)
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

fn parse_run(path: &Path) -> Vec<Step> {
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
    steps
}

/// 一个决策轮的观测量。
struct Row {
    t: f64,
    t_hat: f64,
    /// kind → (真实 u, θ, 各 horizon 前推值)
    ch: BTreeMap<ChannelKind, (f64, f64, Vec<f64>)>,
    /// RLI 自判域（原始枚举调试名）。
    domain: String,
}

/// 0be session-level final readings (lambda_hat / complexity), ASCII note.
struct SessionReadings {
    lambda_hat: Option<f64>,
    cplx_ready: bool,
    cplx_c: Option<f64>,
    cplx_samples: u64,
    cplx_theta: [f64; 3],
    cplx_latched: [bool; 3],
}

fn replay(steps: &[Step]) -> (Vec<Row>, SessionReadings) {
    let mut engine = LifEngine::new();
    engine.enable_rli_shadow();
    let mut rows: Vec<Row> = Vec::new();
    for step in steps {
        match *step {
            Step::Decision(t) => {
                engine.on_decision_round(t);
                let Some(shadow) = engine.rli_shadow() else {
                    continue;
                };
                let t_hat = shadow.t_hat();
                let mut ch = BTreeMap::new();
                for &kind in &RLI_CHANNELS {
                    let channel = shadow.channel(kind);
                    let snap = channel.snapshot();
                    let preds: Vec<f64> = HORIZONS
                        .iter()
                        .map(|m| {
                            // `prediction(x)` = 自由演化 10·x ⇒ x = m·T̂/10 即 m 步前推。
                            channel.prediction_at(*m, t_hat)
                        })
                        .collect();
                    ch.insert(kind, (snap.u, snap.theta, preds));
                }
                let domain = shadow
                    .domain()
                    .now()
                    .map(|r| format!("{:?}", r.domain))
                    .unwrap_or_else(|| "-".to_string());
                rows.push(Row {
                    t,
                    t_hat,
                    ch,
                    domain,
                });
            }
            Step::Tool(t, ev) => engine.on_tool_event(t, ev),
        }
    }
    let shadow = engine.rli_shadow().expect("shadow enabled");
    let reading = shadow.complexity_reading();
    let readings = SessionReadings {
        lambda_hat: shadow.channel(ChannelKind::Prog).lambda_hat(),
        cplx_ready: reading.ready,
        cplx_c: reading.c,
        cplx_samples: reading.samples,
        cplx_theta: reading.theta,
        cplx_latched: reading.latched,
    };
    (rows, readings)
}

#[derive(Default, Clone)]
struct Acc {
    n: u64,
    abs_pred: f64,
    abs_persist: f64,
    sum_pred: f64,
    sum_real: f64,
    /// 阈值符号命中（预测是否与实现在 θ 同侧），仅对进度轴报告。
    th_n: u64,
    th_hit: u64,
    /// 退化检查：两侧各自"在阈上"的频次（若一侧恒真，则该命中率无意义）。
    th_pred_ge: u64,
    th_real_ge: u64,
}

impl Acc {
    fn push(&mut self, pred: f64, persist: f64, real: f64) {
        self.n += 1;
        self.abs_pred += (pred - real).abs();
        self.abs_persist += (persist - real).abs();
        self.sum_pred += pred;
        self.sum_real += real;
    }
    fn json(&self) -> Value {
        if self.n == 0 {
            return json!({ "n": 0 });
        }
        let mae_pred = self.abs_pred / self.n as f64;
        let mae_persist = self.abs_persist / self.n as f64;
        let skill = if mae_persist > 1e-12 {
            1.0 - mae_pred / mae_persist
        } else {
            f64::NAN
        };
        json!({
            "n": self.n,
            "mae_pred": round4(mae_pred),
            "mae_persist": round4(mae_persist),
            "skill_vs_persist": round4(skill),
            "mean_pred": round4(self.sum_pred / self.n as f64),
            "mean_real": round4(self.sum_real / self.n as f64),
            "theta_side_hit_rate": if self.th_n == 0 { Value::Null }
                else { json!(round4(self.th_hit as f64 / self.th_n as f64)) },
            "theta_side_n": self.th_n,
            "theta_pred_ge_count": self.th_pred_ge,
            "theta_real_ge_count": self.th_real_ge,
        })
    }
}

fn round4(x: f64) -> f64 {
    if !x.is_finite() {
        return x;
    }
    (x * 10_000.0).round() / 10_000.0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let roots: Vec<PathBuf> = if args.is_empty() {
        vec![PathBuf::from(r"D:\CLI\.gsa\runs")]
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    let out_path = std::env::var("RLI_PROBE_OUT")
        .unwrap_or_else(|_| r"D:\tb-eval\analysis\rli-forecast-contrast-2026-09-21.json".into());

    // kind → horizon → acc
    let mut table: BTreeMap<String, Vec<Acc>> = BTreeMap::new();
    // (kind, horizon) → 逐 run 的 skill（用户口径：**预测只认 run 内**；跨 run 仅
    // 用于报告分布，不做任何拟合）
    let mut run_skills: BTreeMap<(String, usize), Vec<f64>> = BTreeMap::new();
    let mut runs_used = 0u64;
    let mut rows_total = 0u64;
    let mut t_hat_sum = 0.0f64;
    let mut t_hat_n = 0u64;
    let mut domain_hist: BTreeMap<String, u64> = BTreeMap::new();
    let mut horizon_secs: Vec<f64> = Vec::new();
    let mut sessions: Vec<Value> = Vec::new();
    let mut cplx_ready_runs = 0u64;
    let mut cplx_latched_runs = [0u64; 3];
    let mut c_val_sum = 0.0f64;
    let mut c_val_n = 0u64;
    let mut lambda_samples = 0u64;

    for root in &roots {
        for journal in find_journals(root) {
            let steps = parse_run(&journal);
            if steps.is_empty() {
                continue;
            }
            let (rows, readings) = replay(&steps);
            if rows.is_empty() {
                continue;
            }
            runs_used += 1;
            if readings.cplx_ready {
                cplx_ready_runs += 1;
            }
            for (i, l) in readings.cplx_latched.iter().enumerate() {
                if *l {
                    cplx_latched_runs[i] += 1;
                }
            }
            if let Some(c) = readings.cplx_c {
                c_val_sum += c;
                c_val_n += 1;
            }
            if readings.lambda_hat.is_some() {
                lambda_samples += 1;
            }
            sessions.push(json!({
                "run": journal
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                "rows": rows.len(),
                "t_hat_mean_secs": round4(rows.iter().map(|r| r.t_hat).sum::<f64>() / rows.len() as f64),
                "lambda_hat": readings.lambda_hat.map(round4),
                "complexity": {
                    "ready": readings.cplx_ready,
                    "samples": readings.cplx_samples,
                    "c": readings.cplx_c.map(round4),
                    "theta": readings.cplx_theta.map(round4),
                    "latched": readings.cplx_latched,
                },
            }));
            rows_total += rows.len() as u64;
            for r in &rows {
                if r.t_hat.is_finite() && r.t_hat > 0.0 {
                    t_hat_sum += r.t_hat;
                    t_hat_n += 1;
                    horizon_secs.push(10.0 * r.t_hat);
                }
                *domain_hist.entry(r.domain.clone()).or_insert(0) += 1;
            }
            // 逐行对拍（同一 run 内前向匹配）。
            let mut run_acc: BTreeMap<(String, usize), (f64, f64, u64)> = BTreeMap::new();
            for (i, row) in rows.iter().enumerate() {
                for (hi, m) in HORIZONS.iter().enumerate() {
                    let target = row.t + m * row.t_hat;
                    let Some(fut) = rows[i..].iter().find(|r| r.t >= target) else {
                        continue;
                    };
                    for (&kind, (u, theta, preds)) in &row.ch {
                        let Some((real, real_theta, _)) = fut.ch.get(&kind) else {
                            continue;
                        };
                        let key = format!("{:?}", kind);
                        let accs = table
                            .entry(key.clone())
                            .or_insert_with(|| vec![Acc::default(); HORIZONS.len()]);
                        accs[hi].push(preds[hi], *u, *real);
                        let slot = run_acc.entry((key.clone(), hi)).or_insert((0.0, 0.0, 0));
                        slot.0 += (preds[hi] - *real).abs();
                        slot.1 += (*u - *real).abs();
                        slot.2 += 1;
                        if kind == ChannelKind::Prog {
                            let predicted_side = preds[hi] >= *theta;
                            let real_side = *real >= *real_theta;
                            accs[hi].th_n += 1;
                            if predicted_side {
                                accs[hi].th_pred_ge += 1;
                            }
                            if real_side {
                                accs[hi].th_real_ge += 1;
                            }
                            if predicted_side == real_side {
                                accs[hi].th_hit += 1;
                            }
                        }
                    }
                }
            }
            for ((kind, hi), (abs_pred, abs_persist, n)) in run_acc {
                if n == 0 || abs_persist <= 1e-12 {
                    continue;
                }
                run_skills
                    .entry((kind, hi))
                    .or_default()
                    .push(1.0 - abs_pred / abs_persist);
            }
        }
    }

    let mut channels = serde_json::Map::new();
    for (kind, accs) in &table {
        let mut per_h = serde_json::Map::new();
        for (i, m) in HORIZONS.iter().enumerate() {
            per_h.insert(format!("h_{}", *m as u64), accs[i].json());
            if let Some(skills) = run_skills.get(&(kind.clone(), i)) {
                let mut v = skills.clone();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = v.len();
                let share_pos = v.iter().filter(|s| **s > 0.0).count() as f64 / n as f64;
                per_h.insert(
                    format!("h_{}_per_run", *m as u64),
                    json!({
                        "runs": n,
                        "median_skill": round4(v[n / 2]),
                        "p10_skill": round4(v[n / 10]),
                        "p90_skill": round4(v[(n * 9 / 10).min(n - 1)]),
                        "share_runs_positive": round4(share_pos),
                        "min_skill": round4(v[0]),
                        "max_skill": round4(v[n - 1]),
                    }),
                );
            }
        }
        channels.insert(kind.clone(), Value::Object(per_h));
    }
    let t_hat_mean = if t_hat_n == 0 {
        f64::NAN
    } else {
        t_hat_sum / t_hat_n as f64
    };
    horizon_secs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = |v: &Vec<f64>| {
        if v.is_empty() {
            f64::NAN
        } else {
            v[v.len() / 2]
        }
    };
    let doc = json!({
        "probe": "rli-forecast-contrast",
        "date": "2026-09-21",
        "note": "预测=闭式自由演化前推到 h·T̂；对照=该 run 内第一个 ≥ t+h·T̂ 的决策轮采样上的真实 u；基线=持久性（u 不变）。skill_vs_persist>0 才说明预测有信息量。",
        "roots": roots.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "runs_used": runs_used,
        "decision_rows": rows_total,
        "t_hat_mean_secs": round4(t_hat_mean),
        "horizon_10_secs_median": round4(median(&horizon_secs)),
        "domain_rows_hist": domain_hist,
        "complexity_summary": {
            "runs_ready": cplx_ready_runs,
            "runs_with_lambda": lambda_samples,
            "latched_runs": cplx_latched_runs,
            "c_runs": c_val_n,
            "c_mean": if c_val_n == 0 { Value::Null } else { json!(round4(c_val_sum / c_val_n as f64)) },
        },
        "sessions": sessions,
        "channels": channels,
    });
    if let Some(parent) = Path::new(&out_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&out_path, serde_json::to_string_pretty(&doc).unwrap());
    println!(
        "runs={} rows={} t_hat_mean={:.2}s horizon10_median={:.1}s -> {}",
        runs_used,
        rows_total,
        t_hat_mean,
        median(&horizon_secs),
        out_path
    );
    println!("\n== 逐 run skill（用户口径：预测只认 run 内；跨 run 只看分布）==");
    for (kind, _) in &table {
        for (i, m) in HORIZONS.iter().enumerate() {
            if let Some(skills) = run_skills.get(&(kind.clone(), i)) {
                let mut v = skills.clone();
                v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let n = v.len();
                let share = v.iter().filter(|s| **s > 0.0).count() as f64 / n as f64;
                println!(
                    "{:<6} h={:<2} runs={:<5} median={:+.3} p10={:+.3} p90={:+.3} 正比例={:.2}",
                    kind,
                    *m as u64,
                    n,
                    v[n / 2],
                    v[n / 10],
                    v[(n * 9 / 10).min(n - 1)],
                    share
                );
            }
        }
    }
    for (kind, accs) in &table {
        for (i, m) in HORIZONS.iter().enumerate() {
            let a = &accs[i];
            if a.n == 0 {
                continue;
            }
            println!(
                "{:<6} h={:<2} n={:<6} MAE_pred={:.4} MAE_persist={:.4} skill={:+.3}{}",
                kind,
                *m as u64,
                a.n,
                a.abs_pred / a.n as f64,
                a.abs_persist / a.n as f64,
                1.0 - (a.abs_pred / a.abs_persist),
                if kind == "Prog" && a.th_n > 0 {
                    format!(
                        "  θ侧命中 {:.3} (n={})",
                        a.th_hit as f64 / a.th_n as f64,
                        a.th_n
                    )
                } else {
                    String::new()
                }
            );
        }
    }
}
