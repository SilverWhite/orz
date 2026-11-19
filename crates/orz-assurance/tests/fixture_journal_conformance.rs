//! Task D (双实现终局治理, 2026-09-04) batch-1 evidence: the Rust-side
//! schema-level journal judge must accept every real Rust-produced fixture
//! journal committed under `runtime/fixtures/` and reject the same tamper
//! classes the Python validator rejects.
//!
//! Repo-root resolution mirrors the ORZ-BUILD-MOUNT-001 contract:
//! `CARGO_MANIFEST_DIR/../../../runtime` == `<repo root>/runtime`.

use orz_assurance::journal::conformance::validate_journal_file;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    assert!(
        root.join("runtime").is_dir(),
        "fixture conformance must run inside the parent repository \
         (runtime/ missing at {}): expected {root:?}",
        root.display()
    );
    root
}

const V01_JOURNALS: &[&str] = &[
    "plain-run.jsonl",
    "tool-snapshot-run.jsonl",
    "plan-run.jsonl",
    "cancelled-run.jsonl",
    "failed-run.jsonl",
    "restore-run.jsonl",
];

const V02_JOURNALS: &[&str] = &[
    "plain-run.jsonl",
    "tool-snapshot-run.jsonl",
    "plan-run.jsonl",
    "cancelled-run.jsonl",
    "failed-run.jsonl",
    "restore-run.jsonl",
    "orientation-fire-run.jsonl",
    "mode-off-refusal.jsonl",
    "local-browser-capability.jsonl",
    "local-browser-read.jsonl",
    "real-doc-retrieval.jsonl",
    "cross-prompt-restore.jsonl",
    // 0dv S2（403 批，2026-11-19）：idle 唤醒边界（result_delivered
    // boundary 第四位 `idle_wake` 就地加位）正型全链——run_preflight →
    // tool_availability_check → run_started（机械源 prompt）→ 唤醒开局
    // 投递两事件 → model_output → counterexample_gate → model_output →
    // run_finished；链/信封/载荷 schema/族判官全绿。
    "idle-wake-delivery.jsonl",
];

fn fixture_path(track: &str, name: &str) -> PathBuf {
    repo_root()
        .join("runtime/fixtures")
        .join(format!("run-event-{track}"))
        .join("journals")
        .join(name)
}

/// Copy a fixture journal into a temp dir, apply `mutate`, and return the
/// (tempdir, journal path) pair — the tempdir must stay alive for the whole
/// validation call.
fn tampered_journal(
    track: &str,
    name: &str,
    mutate: impl Fn(&mut Vec<Value>),
) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = std::fs::read_to_string(fixture_path(track, name)).expect("fixture journal");
    let mut events: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture line"))
        .collect();
    mutate(&mut events);
    let out = dir.path().join("events.jsonl");
    let mut content = String::new();
    for event in &events {
        content.push_str(&serde_json::to_string(event).expect("serialize"));
        content.push('\n');
    }
    std::fs::write(&out, content).expect("write tampered journal");
    (dir, out)
}

#[test]
fn all_fixture_journals_pass_rust_schema_conformance() {
    let root = repo_root();
    for name in V01_JOURNALS {
        let report = validate_journal_file(&fixture_path("v0.1", name), &root);
        assert!(
            report.valid,
            "v0.1 {name} failed Rust conformance: {:?}",
            report.errors
        );
        assert!(report.event_count > 0, "v0.1 {name}: empty journal");
    }
    for name in V02_JOURNALS {
        let report = validate_journal_file(&fixture_path("v0.2", name), &root);
        assert!(
            report.valid,
            "v0.2 {name} failed Rust conformance: {:?}",
            report.errors
        );
        assert!(report.event_count > 0, "v0.2 {name}: empty journal");
    }
}

#[test]
fn extra_top_level_envelope_field_rejected() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        events[0]["stray_field"] = json!(true);
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("envelope schema violation")),
        "errors: {:?}",
        report.errors
    );
    // Python gating: envelope failures short-circuit before payload/chain.
    assert!(
        report
            .errors
            .iter()
            .all(|e| !e.contains("payload schema violation") && !e.contains("digest mismatch")),
        "envelope-stage-only expected, got: {:?}",
        report.errors
    );
}

#[test]
fn wrong_schema_version_pair_rejected() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        events[0]["schema_version"] = json!("0.1.0-draft");
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("event 0") && e.contains("track pair")),
        "errors: {:?}",
        report.errors
    );
}

#[test]
fn non_object_payload_rejected() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        events[0]["payload"] = json!([]);
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("envelope schema violation")),
        "errors: {:?}",
        report.errors
    );
}

#[test]
fn retired_event_type_rejected_on_v02_track() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        events[0]["event_type"] = json!("neutral_inquiry");
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("envelope schema violation")),
        "errors: {:?}",
        report.errors
    );
}

#[test]
fn mixed_track_journal_rejected_as_not_homogeneous() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        events[1]["schema_version"] = json!("0.1.0-draft");
        events[1]["payload_schema"] = json!("run-event-v0.1.schema.json");
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report.errors.iter().any(|e| e.contains("not homogeneous")),
        "errors: {:?}",
        report.errors
    );
}

#[test]
fn chain_tamper_detected_after_schema_valid_payload() {
    let root = repo_root();
    let (_dir, path) = tampered_journal("v0.2", "plain-run.jsonl", |events| {
        // Keep the value a legal 64-lower-hex digest but break the recompute.
        let digest = events[1]["event_sha256"]
            .as_str()
            .expect("digest")
            .to_string();
        let flipped = if digest.ends_with('0') {
            format!("{}1", &digest[..63])
        } else {
            format!("{}0", &digest[..63])
        };
        events[1]["event_sha256"] = json!(flipped);
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report.errors.iter().any(|e| e.contains("digest mismatch")),
        "errors: {:?}",
        report.errors
    );
}
// ── S2b (2026-09-06): end-to-end family-stage negative evidence ─────────
//
// The conformance stage-5 gating (`payload_valid` → families run) must be
// exercised through the full `validate_journal_file` pipeline: a payload-
// schema-valid tamper of a real fixture journal must surface the family-
// stage error (and ONLY that class of error after re-sealing the chain).

use orz_assurance::journal::{canonical_json, sha256_hex};

/// Copy a fixture journal, apply `mutate`, RE-SEAL the payload/event digests
/// and the previous-event chain, and return the (tempdir, journal path).
fn resealed_journal(
    track: &str,
    name: &str,
    mutate: impl Fn(&mut Vec<Value>),
) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let text = std::fs::read_to_string(fixture_path(track, name)).expect("fixture journal");
    let mut events: Vec<Value> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("fixture line"))
        .collect();
    mutate(&mut events);

    let mut previous: Option<String> = None;
    for event in events.iter_mut() {
        // previous link first: the event digest covers it.
        if let Some(prev) = &previous {
            event["previous_event_sha256"] = json!(prev);
        }
        let payload_sha = sha256_hex(&canonical_json(&event["payload"]).expect("canonical"));
        event["payload_sha256"] = json!(payload_sha);
        if let Some(object) = event.as_object_mut() {
            object.remove("event_sha256");
            let event_sha =
                sha256_hex(&canonical_json(&Value::Object(object.clone())).expect("canonical"));
            object.insert("event_sha256".to_string(), json!(event_sha));
        }
        previous = event
            .get("event_sha256")
            .and_then(Value::as_str)
            .map(String::from);
    }

    let out = dir.path().join("events.jsonl");
    let mut content = String::new();
    for event in &events {
        content.push_str(&serde_json::to_string(event).expect("serialize"));
        content.push('\n');
    }
    std::fs::write(&out, content).expect("write re-sealed journal");
    (dir, out)
}

#[test]
fn family_stage_tamper_detected_end_to_end() {
    let root = repo_root();
    // P0-0x S2 (§14.66) re-capture note: the current orientation-fire-run
    // capture is auto_close-only (no dispositions: R1 §4.4 closes every
    // retrieval dispatch — the pre-0t tamper targeted a
    // `retrieval_parent_disposition` that no longer exists, so it mutated
    // nothing and the assertion failed on a clean journal).
    //
    // The tamper now targets the ONE-SHOT initial-round inquiry: moving its
    // `injection_position` off `post_tool_batch_gap` breaks the
    // trigger↔block↔position coupling — a rule ONLY the family stage checks
    // (the payload schema accepts any non-empty string).
    let (_dir, path) = resealed_journal("v0.2", "orientation-fire-run.jsonl", |events| {
        for event in events.iter_mut() {
            if event["event_type"] == "orientation_checkpoint"
                && event["payload"]["trigger"] == "initial_round"
            {
                event["payload"]["injection_position"] = json!("loop_top_gap");
                break;
            }
        }
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("initial_round") && e.contains("post_tool_batch_gap")),
        "family-stage (initial_round_inquiry) error expected, got: {:?}",
        report.errors
    );
    // The re-sealed journal must be clean everywhere BEFORE the family
    // stage — this locks the stage-5 gating (a flipped gate condition would
    // leave the suite green without this test).
    assert!(
        report.errors.iter().all(|e| {
            !e.contains("envelope schema violation")
                && !e.contains("payload schema violation")
                && !e.contains("digest mismatch")
        }),
        "re-sealed journal must only fail in the family stage, got: {:?}",
        report.errors
    );
}

#[test]
fn family_stage_s2c_tamper_detected_end_to_end() {
    let root = repo_root();
    // real-doc-retrieval carries a retrieval_result_committed: inflating the
    // declared source_counts.total breaks the mechanical ledger distribution —
    // a rule ONLY the S2c result_consistency family checks (the payload
    // schema accepts any non-negative integer).
    let (_dir, path) = resealed_journal("v0.2", "real-doc-retrieval.jsonl", |events| {
        for event in events.iter_mut() {
            if event["event_type"] == "retrieval_result_committed" {
                event["payload"]["source_counts"]["total"] = json!(99);
                break;
            }
        }
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report.errors.iter().any(|e| e.contains("source_counts")),
        "family-stage (result_consistency) error expected, got: {:?}",
        report.errors
    );
    assert!(
        report.errors.iter().all(|e| {
            !e.contains("envelope schema violation")
                && !e.contains("payload schema violation")
                && !e.contains("digest mismatch")
        }),
        "re-sealed journal must only fail in the family stage, got: {:?}",
        report.errors
    );
}

// ── 0dw S1 整改批（2026-11-18，398 批审查 P1）──────────────────────────
//
// 载体身份章两键（carrier_version ＋ binary_sha256）的 payload schema 面。
// 398 批以「registry/judge 两侧均无此族、扩键安全」落码——审查证伪（registry
// v01 侧有严格 schema 且回落解析必撞 additionalProperties）；整改立 v0.2
// 自有 schema（两键 optional＝章前历史 grandfather）。本组两钉锁死修复：
// 五键重封正型全链绿；越第六键恰在 payload 阶段拒。

#[test]
fn preflight_carrier_stamp_sealed_journal_passes_conformance() {
    let root = repo_root();
    let (_dir, path) = resealed_journal("v0.2", "plain-run.jsonl", |events| {
        let payload = events[0]["payload"]
            .as_object_mut()
            .expect("preflight payload");
        payload.insert("carrier_version".to_string(), json!("0.9.15"));
        payload.insert(
            "binary_sha256".to_string(),
            json!("3f79bb7b435b05321651daefd374cdc681dc06faa65e374e38337b88ca044dea"),
        );
    });
    let report = validate_journal_file(&path, &root);
    assert!(
        report.valid,
        "5-key stamped preflight must pass the v0.2 payload schema: {:?}",
        report.errors
    );
}

#[test]
fn preflight_extra_key_rejected_at_payload_stage() {
    let root = repo_root();
    let (_dir, path) = resealed_journal("v0.2", "plain-run.jsonl", |events| {
        let payload = events[0]["payload"]
            .as_object_mut()
            .expect("preflight payload");
        payload.insert("stray_key".to_string(), json!(true));
    });
    let report = validate_journal_file(&path, &root);
    assert!(!report.valid);
    assert!(
        report
            .errors
            .iter()
            .any(|e| e.contains("payload schema violation")),
        "payload-stage violation expected, got: {:?}",
        report.errors
    );
    // 重封后链/信封干净 ⇒ 唯一违例类＝payload schema（恰触本事件的越键）。
    assert!(
        report.errors.iter().all(|e| {
            !e.contains("envelope schema violation") && !e.contains("digest mismatch")
        }),
        "re-sealed journal must only fail in the payload stage, got: {:?}",
        report.errors
    );
}
