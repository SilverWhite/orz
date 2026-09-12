//! 0v-C forensics harness (scratch, 2026-09-12): replay the journal redaction
//! funnel offline over one recorded event and report
//!   (a) stored vs recomputed `event_sha256`,
//!   (b) every payload string the real `orz-secrets` funnel would still change
//!       (proves whether the on-disk payload is a funnel fixed point),
//!   (c) the post-redaction `event_sha256` (the value the chain link must
//!       carry once the funnel is wired correctly).
//!
//! Usage: `cargo run -p orz-assurance --example redact_forensics -- <events.jsonl> <sequence> [ghost_sha]`
//!
//! Scratch tooling for the 0v-C root-cause investigation — not part of the
//! production surface.

use orz_assurance::{RunEvent, compute_event_hash};
use serde_json::Value;

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .expect("usage: redact_forensics <events.jsonl> <sequence> [ghost_sha]");
    let seq: u64 = args.next().expect("sequence").parse().expect("sequence");
    let ghost = args.next();

    let text = std::fs::read_to_string(&path).expect("read journal");
    let line = text
        .lines()
        .find(|l| {
            !l.trim().is_empty()
                && serde_json::from_str::<Value>(l)
                    .map(|v| v["sequence"].as_u64() == Some(seq))
                    .unwrap_or(false)
        })
        .expect("sequence not found in journal");

    let mut event: RunEvent = serde_json::from_str(line).expect("parse RunEvent");
    let stored = event.event_sha256.clone();
    let recomputed = compute_event_hash(&event).expect("recompute");
    println!("stored                    = {stored}");
    println!("recomputed                = {recomputed}");
    println!("in-line self-consistent   = {}", stored == recomputed);
    if let Some(g) = &ghost {
        println!("stored == ghost           = {}", stored == *g);
    }

    let before = event.payload.clone();
    orz_secrets::redact_json_string_values(&mut event.payload);
    let mut diffs: Vec<(String, String, String)> = Vec::new();
    collect_diffs(&before, &event.payload, "$".into(), &mut diffs);
    println!("funnel changes on payload = {}", diffs.len());
    for (p, b, a) in diffs.iter().take(24) {
        println!("  at {p}");
        println!("    before: {}", preview(b));
        println!("    after : {}", preview(a));
    }

    orz_assurance::seal_event(&mut event).expect("seal");
    println!("post-redaction sha        = {}", event.event_sha256);
    if let Some(g) = &ghost {
        println!("post-redaction == ghost   = {}", event.event_sha256 == *g);
    }
}

fn preview(s: &str) -> String {
    let mut out: String = s.chars().take(240).collect();
    if s.chars().count() > 240 {
        out.push('…');
    }
    out.replace('\n', "\\n")
}

fn collect_diffs(
    before: &Value,
    after: &Value,
    path: String,
    out: &mut Vec<(String, String, String)>,
) {
    match (before, after) {
        (Value::String(b), Value::String(a)) => {
            if b != a {
                out.push((path, b.clone(), a.clone()));
            }
        }
        (Value::Array(b), Value::Array(a)) => {
            for (i, (bv, av)) in b.iter().zip(a.iter()).enumerate() {
                collect_diffs(bv, av, format!("{path}[{i}]"), out);
            }
        }
        (Value::Object(b), Value::Object(a)) => {
            for (k, bv) in b.iter() {
                if let Some(av) = a.get(k) {
                    collect_diffs(bv, av, format!("{path}.{k}"), out);
                }
            }
        }
        _ => {}
    }
}
