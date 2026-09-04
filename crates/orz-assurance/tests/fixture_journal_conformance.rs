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
