//! Offline run-event journal conformance — the Rust-side schema-level judge.
//!
//! Task D (双实现终局治理, 2026-09-04) batch-1: Rust `orz-assurance` gains a
//! schema-level offline judge that replays a Rust-track journal the way
//! `assurance/run_event_journal_validation.py` does:
//!
//! 1. envelope schema validation (per-event, track selected by the
//!    `schema_version`/`payload_schema` pair),
//! 2. whole-journal track homogeneity (one track per journal, §11.6.2),
//! 3. per-event payload schema validation (schema file resolved through the
//!    machine-readable registry `runtime/run-event-payload-registry-v0.1.json`,
//!    the same table the Python validator registers),
//! 4. full hash-chain recompute + terminal semantics.
//!
//! The chain stage recomputes digests over raw JSON (like the Python
//! validator) instead of the typed [`super::event::RunEvent`] model: the
//! historical v0.1 fixture corpus still contains
//! `runtime_stagnation_guard` events, a type the typed enum retired on
//! 2026-08-22 — schema-level conformance must replay it while the production
//! replay model stays strict.  This is the first measured drift surface the
//! dual-judge cleanup resolves (see the Task D audit doc).
//!
//! The mechanical cross-check families (`_verify_v02_*` in the Python
//! validator) are a separate later slice; this module deliberately mirrors
//! the Python validator's gating: parse errors stop first, then envelope
//! errors, then payload + chain.  Format validation is disabled to match the
//! Python `jsonschema.FormatChecker` no-op behaviour for `date-time`
//! (documented environment fact, reference-spec contract §3).
//!
//! This judge is offline by design: it loads schema files and the registry
//! from the repository root at runtime.  Production replay
//! ([`super::verifier::replay_journal`]) stays hermetic, typed and unchanged.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;

use super::chain::{canonical_json, sha256_hex};
use super::event::EventTrack;

const REGISTRY_REL: &str = "runtime/run-event-payload-registry-v0.1.json";
const ENVELOPE_V01_REL: &str = "runtime/run-event-v0.1.schema.json";
const ENVELOPE_V02_REL: &str = "runtime/run-event-v0.2.schema.json";

/// Result of one journal conformance pass (`errors` empty == valid).
#[derive(Debug, Clone)]
pub struct ConformanceReport {
    pub valid: bool,
    pub journal_path: String,
    pub event_count: u64,
    pub errors: Vec<String>,
    /// FUS-HOST-RESOURCE-SAFETY §4.3 item 6 (2026-09-12, 0z S2): the journal
    /// replayed through a **degraded** (chain-skeleton-only) episode — the
    /// on-disk `degraded` summary markers / `run_terminated
    /// { reason: journal_degraded }` terminal prove it. A status class, never
    /// mixed with validity: a degraded-complete journal is chain-valid
    /// (`valid: true`) while its event set is knowingly incomplete; an
    /// `invalid` journal is invalid regardless of markers. Historical
    /// journals carry no marker and keep their classification untouched
    /// (判据 7).
    pub degraded_complete: bool,
}

/// The degraded-journal evidence scan (see [`ConformanceReport::degraded_complete`]).
fn is_degraded_journal(events: &[Value]) -> bool {
    events.iter().any(|event| {
        let payload = event.get("payload");
        let degraded_marker = payload
            .map(|p| p.get("degraded").is_some_and(Value::is_object))
            .unwrap_or(false);
        let degraded_terminal = event.get("event_type").and_then(Value::as_str)
            == Some("run_terminated")
            && payload
                .map(|p| p.get("reason").and_then(Value::as_str) == Some("journal_degraded"))
                .unwrap_or(false);
        degraded_marker || degraded_terminal
    })
}

/// Registry entry — one `event_type` on one track.
#[derive(Debug, Clone)]
struct RegistryEntry {
    schema_rel: String,
}

/// Payload-schema registry: track tables mirroring the Python validator.
#[derive(Debug, Default)]
struct PayloadRegistry {
    v01: HashMap<String, RegistryEntry>,
    v02: HashMap<String, RegistryEntry>,
}

impl PayloadRegistry {
    /// Resolve the payload schema for one event (Python `_resolve_payload_schema`
    /// semantics for the Rust track): v0.2 events use their own v0.2 payload
    /// schema when registered, otherwise fall back to the v0.1 table.
    fn resolve(&self, track: EventTrack, event_type: &str) -> Result<&RegistryEntry, String> {
        match track {
            EventTrack::V01 => self.v01.get(event_type).ok_or_else(|| {
                format!("unknown event_type on the Rust v0.1 track: {event_type:?}")
            }),
            EventTrack::V02 => self
                .v02
                .get(event_type)
                .or_else(|| self.v01.get(event_type))
                .ok_or_else(|| {
                    format!("unknown event_type on the Rust v0.2 track: {event_type:?}")
                }),
        }
    }
}

fn load_json(path: &Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON in {}: {e}", path.display()))
}

fn load_registry(repo_root: &Path) -> Result<PayloadRegistry, String> {
    let registry = load_json(&repo_root.join(REGISTRY_REL))?;
    let tracks = registry
        .get("tracks")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{REGISTRY_REL}: missing object field `tracks`"))?;
    let mut out = PayloadRegistry::default();
    for (table, key) in [(&mut out.v01, "v01"), (&mut out.v02, "v02")] {
        let obj = tracks
            .get(key)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{REGISTRY_REL}: missing track table `{key}`"))?;
        for (event_type, entry) in obj {
            let schema_rel = entry
                .get("schema")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{REGISTRY_REL}: {key}/{event_type} missing schema"))?
                .to_string();
            table.insert(event_type.clone(), RegistryEntry { schema_rel });
        }
    }
    Ok(out)
}

/// Compute the canonical-JSON SHA-256 hex of a value (Rust-parity form).
fn canonical_sha256(value: &Value) -> Result<String, String> {
    let bytes = canonical_json(value).map_err(|e| e.to_string())?;
    Ok(sha256_hex(&bytes))
}

fn push_digest_mismatch(
    errors: &mut Vec<String>,
    index: usize,
    what: &str,
    computed: String,
    stored: &str,
) {
    errors.push(format!(
        "{what} mismatch at event {index}: expected {computed}, got {stored}"
    ));
}

/// Raw-JSON hash-chain verification mirroring the Python validator
/// (`_verify_chain`) and `chain.rs`: sequence density, run_id/manifest
/// constancy, previous-event links, payload/event digest recompute and
/// exactly-one-terminal-at-the-end.
fn verify_chain_raw(events: &[Value], errors: &mut Vec<String>) {
    // FUS-HOST-RESOURCE-SAFETY §4.3 (2026-09-12, 0z S2): `run_terminated`
    // joins the terminal set — the explicit terminal shape for
    // resource-exhausted and journal-degraded endings.
    const TERMINAL_TYPES: [&str; 5] = [
        "run_finished",
        "run_failed",
        "run_cancelled",
        "run_invalidated",
        "run_terminated",
    ];
    let terminals: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event
                .get("event_type")
                .and_then(Value::as_str)
                .is_some_and(|t| TERMINAL_TYPES.contains(&t))
        })
        .map(|(index, _)| index)
        .collect();
    for &index in &terminals {
        if index != events.len() - 1 {
            errors.push(format!(
                "terminal event at index {index} is not the last event"
            ));
        }
    }
    if terminals.len() > 1 {
        errors.push(format!("multiple terminal events found"));
    } else if terminals.is_empty() {
        errors.push("expected exactly one terminal event, found 0".into());
    }

    let run_id = events[0]
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let manifest = events[0]
        .get("run_manifest_sha256")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let mut previous: Option<String> = None;

    for (index, event) in events.iter().enumerate() {
        if event.get("sequence").and_then(Value::as_u64) != Some(index as u64) {
            errors.push(format!(
                "sequence mismatch at event {index}: expected {index}, got {}",
                event
                    .get("sequence")
                    .map_or_else(|| "null".into(), |v| v.to_string())
            ));
        }
        if event.get("run_id").and_then(Value::as_str) != Some(run_id.as_str()) {
            errors.push(format!(
                "run_id mismatch at event {index}: expected {run_id}, got {}",
                event
                    .get("run_id")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>")
            ));
        }
        if event.get("run_manifest_sha256").and_then(Value::as_str) != Some(manifest.as_str()) {
            errors.push(format!(
                "manifest digest mismatch at event {index}: expected {manifest}, got {}",
                event
                    .get("run_manifest_sha256")
                    .and_then(Value::as_str)
                    .unwrap_or("<missing>")
            ));
        }
        match index {
            0 => {
                if !event
                    .get("previous_event_sha256")
                    .is_none_or(|v| v.is_null())
                {
                    errors.push(format!(
                        "first event has non-null previous_event_sha256: {}",
                        event
                            .get("previous_event_sha256")
                            .map_or("null".into(), |v| v.to_string())
                    ));
                }
            }
            _ => match event.get("previous_event_sha256").and_then(Value::as_str) {
                None => errors.push(format!(
                    "non-first event {index} has null previous_event_sha256"
                )),
                Some(actual) => {
                    if let Some(expected) = &previous
                        && actual != expected
                    {
                        errors.push(format!(
                            "previous_event_sha256 mismatch at event {index}: \
                             expected {expected}, got {actual}"
                        ));
                    }
                }
            },
        }

        if let Some(payload) = event.get("payload") {
            match canonical_sha256(payload) {
                Ok(computed) => {
                    let stored = event
                        .get("payload_sha256")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if stored != computed {
                        push_digest_mismatch(errors, index, "payload digest", computed, stored);
                    }
                }
                Err(e) => errors.push(format!("failed to hash payload at event {index}: {e}")),
            }
        }

        if let Some(object) = event.as_object() {
            let mut projection = object.clone();
            projection.remove("event_sha256");
            match canonical_sha256(&serde_json::Value::Object(projection)) {
                Ok(computed) => {
                    let stored = event
                        .get("event_sha256")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if stored != computed {
                        push_digest_mismatch(errors, index, "event digest", computed, stored);
                    }
                }
                Err(e) => errors.push(format!("failed to hash event at event {index}: {e}")),
            }
        }

        previous = event
            .get("event_sha256")
            .and_then(Value::as_str)
            .map(String::from);
    }
}

/// Build a compiled envelope validator for a track (format validation off —
/// Python parity).
fn envelope_validator(
    repo_root: &Path,
    track: EventTrack,
) -> Result<jsonschema::Validator, String> {
    let schema_rel = match track {
        EventTrack::V01 => ENVELOPE_V01_REL,
        EventTrack::V02 => ENVELOPE_V02_REL,
    };
    let schema = load_json(&repo_root.join(schema_rel))?;
    jsonschema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .should_validate_formats(false)
        .build(&schema)
        .map_err(|e| format!("schema compile error ({schema_rel}): {e}"))
}

/// Validate one journal against the Rust-track schema-level conformance rules.
///
/// - `journal_path`: the `events.jsonl` to validate
/// - `repo_root`: repository root that contains `runtime/` (schema + registry)
pub fn validate_journal_file(journal_path: &Path, repo_root: &Path) -> ConformanceReport {
    let mut errors: Vec<String> = Vec::new();

    let text = match std::fs::read_to_string(journal_path) {
        Ok(s) => s,
        Err(e) => {
            errors.push(format!(
                "journal file not found: {} ({e})",
                journal_path.display()
            ));
            return ConformanceReport {
                valid: false,
                journal_path: journal_path.display().to_string(),
                event_count: 0,
                errors,
                degraded_complete: false,
            };
        }
    };

    // 1. Parse every line as JSON.  Parse/blank-line errors stop first (the
    //    Python validator's gating).
    let mut events: Vec<Value> = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        let line_num = line_number + 1;
        if line.trim().is_empty() {
            errors.push(format!("blank journal line {line_num}"));
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(event) => events.push(event),
            Err(e) => errors.push(format!("invalid JSON at line {line_num}: {e}")),
        }
    }
    if !errors.is_empty() {
        return ConformanceReport {
            valid: false,
            journal_path: journal_path.display().to_string(),
            event_count: 0,
            errors,
            degraded_complete: false,
        };
    }
    if events.is_empty() {
        errors.push("journal contains no valid events".into());
        return ConformanceReport {
            valid: false,
            journal_path: journal_path.display().to_string(),
            event_count: 0,
            errors,
            degraded_complete: false,
        };
    }
    let event_count = events.len() as u64;

    // 2. Envelope validation + track homogeneity.
    let first_track = match track_of(&events[0]) {
        Ok(track) => track,
        Err(e) => {
            errors.push(format!("event 0: {e}"));
            return ConformanceReport {
                valid: false,
                journal_path: journal_path.display().to_string(),
                event_count,
                errors,
                degraded_complete: false,
            };
        }
    };
    let v01_validator = match envelope_validator(repo_root, EventTrack::V01) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            return ConformanceReport {
                valid: false,
                journal_path: journal_path.display().to_string(),
                event_count,
                errors,
                degraded_complete: false,
            };
        }
    };
    let v02_validator = match envelope_validator(repo_root, EventTrack::V02) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            return ConformanceReport {
                valid: false,
                journal_path: journal_path.display().to_string(),
                event_count,
                errors,
                degraded_complete: false,
            };
        }
    };

    for (index, event) in events.iter().enumerate() {
        let track = match track_of(event) {
            Ok(track) => track,
            Err(e) => {
                errors.push(format!("event {index}: {e}"));
                continue;
            }
        };
        if track != first_track {
            errors.push(format!(
                "event {index}: journal track is not homogeneous \
                 (first event is {}, this event is {})",
                first_track.payload_schema_id(),
                track.payload_schema_id()
            ));
        }
        let validator = match track {
            EventTrack::V01 => &v01_validator,
            EventTrack::V02 => &v02_validator,
        };
        if let Err(e) = validator.validate(event) {
            errors.push(format!("envelope schema violation at event {index}: {e}"));
        }
    }
    if !errors.is_empty() {
        // Python semantics: envelope failures can leave fields missing that
        // hashing needs — report envelope stage only.
        return ConformanceReport {
            valid: false,
            journal_path: journal_path.display().to_string(),
            event_count,
            errors,
            degraded_complete: false,
        };
    }

    // 3. Payload schema validation through the shared registry.
    let registry = match load_registry(repo_root) {
        Ok(reg) => reg,
        Err(e) => {
            errors.push(e);
            return ConformanceReport {
                valid: false,
                journal_path: journal_path.display().to_string(),
                event_count,
                errors,
                degraded_complete: false,
            };
        }
    };
    let mut payload_validators: HashMap<String, jsonschema::Validator> = HashMap::new();
    let mut payload_errors: Vec<String> = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let track = match track_of(event) {
            Ok(track) => track,
            Err(_) => continue, // already reported above
        };
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let entry = match registry.resolve(track, event_type) {
            Ok(entry) => entry,
            Err(e) => {
                payload_errors.push(format!("event {index}: {e}"));
                continue;
            }
        };
        let payload = match event.get("payload") {
            Some(payload) => payload,
            None => {
                payload_errors.push(format!("event {index}: payload missing"));
                continue;
            }
        };
        let validator = if let Some(validator) = payload_validators.get(&entry.schema_rel) {
            validator
        } else {
            let schema = match load_json(&repo_root.join(&entry.schema_rel)) {
                Ok(schema) => schema,
                Err(e) => {
                    payload_errors.push(format!("event {index}: {e}"));
                    continue;
                }
            };
            let validator = match jsonschema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .should_validate_formats(false)
                .build(&schema)
            {
                Ok(validator) => validator,
                Err(e) => {
                    payload_errors.push(format!(
                        "event {index}: schema compile error ({}): {e}",
                        entry.schema_rel
                    ));
                    continue;
                }
            };
            payload_validators.insert(entry.schema_rel.clone(), validator);
            payload_validators.get(&entry.schema_rel).unwrap()
        };
        if let Err(e) = validator.validate(payload) {
            payload_errors.push(format!("payload schema violation at event {index}: {e}"));
        }
    }
    let payload_valid = payload_errors.is_empty();
    errors.extend(payload_errors);

    // 4. Full raw-JSON hash-chain recompute + terminal semantics.
    verify_chain_raw(&events, &mut errors);

    // 5. Rule-family verifiers (Task D S2b/S2c, 2026-09-06). Python gating:
    // the cross-layer families run only on schema-valid input (`not
    // payload_errors`) — on BOTH tracks. Most families filter `_is_v02` per
    // event (no-ops on a V01 journal), but FOUR do NOT (they only match
    // `event_type`): `policy_denial` / `failure_target` (S2b) and
    // `candidate_count` / `inject_budget` (S2c) — the v0.1 tool-completed
    // payload schema defines those fields, so the stage must run on V01
    // journals too (S2b review P1, 2026-09-06).
    if payload_valid {
        errors.extend(super::families::verify_all_families(&events));
    }

    let degraded_complete = is_degraded_journal(&events);
    ConformanceReport {
        valid: errors.is_empty(),
        journal_path: journal_path.display().to_string(),
        event_count,
        errors,
        degraded_complete,
    }
}

/// The `schema_version`/`payload_schema` pair pins the Rust journal track.
fn track_of(event: &Value) -> Result<EventTrack, String> {
    let schema_version = event.get("schema_version").and_then(Value::as_str);
    let payload_schema = event.get("payload_schema").and_then(Value::as_str);
    match (schema_version, payload_schema) {
        (Some(version), Some(payload))
            if version == EventTrack::V01.schema_version()
                && payload == EventTrack::V01.payload_schema_id() =>
        {
            Ok(EventTrack::V01)
        }
        (Some(version), Some(payload))
            if version == EventTrack::V02.schema_version()
                && payload == EventTrack::V02.payload_schema_id() =>
        {
            Ok(EventTrack::V02)
        }
        _ => Err(format!(
            "unrecognized Rust-track pair schema_version={schema_version:?} \
             payload_schema={payload_schema:?}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0bt④（2026-09-26）：`permission_decision` payload schema 的 `source`
    /// 封闭集钉——旧形状（无 source）与七个合法来源放行；未知值必须拒绝
    /// （未知值负例）。
    #[test]
    fn permission_decision_source_schema_rejects_unknown_values() {
        let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let schema_path =
            repo_root.join("runtime/permission-decision-event-payload-v0.1.schema.json");
        let schema: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&schema_path)
                .unwrap_or_else(|e| panic!("read {}: {e}", schema_path.display())),
        )
        .expect("schema JSON");
        let validator = jsonschema::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .should_validate_formats(false)
            .build(&schema)
            .expect("schema compiles");

        // 旧形状（无 source）——旧 journal 回放零新增报错。
        validator
            .validate(&serde_json::json!({"tool": "read_file", "decision": "allow_once"}))
            .expect("old shape without source stays valid");
        // 封闭集正例逐值放行。
        for source in [
            "policy",
            "scope",
            "yolo",
            "user",
            "timeout",
            "fail_closed",
            "classifier",
        ] {
            validator
                .validate(&serde_json::json!({
                    "tool": "bash",
                    "decision": "deny",
                    "source": source,
                }))
                .unwrap_or_else(|e| panic!("source={source} must be valid: {e}"));
        }
        // 未知值负例：必须拒绝。
        assert!(
            validator
                .validate(&serde_json::json!({
                    "tool": "bash",
                    "decision": "deny",
                    "source": "grok_said_no",
                }))
                .is_err(),
            "unknown source values must be rejected by the closed-set schema"
        );
    }

    /// 0z S2 §4.3 item 6 / 判据 7（2026-09-12）：降级卷判 `degraded_complete`
    /// ——链骨架完整 + 盘上降级证据；不得与 `invalid` 混判；历史卷（无标记）
    /// 分类不变（`degraded_complete: false`）。
    #[test]
    fn degraded_marker_classifies_degraded_complete_without_mixing_invalid() {
        fn repo_root() -> std::path::PathBuf {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
            assert!(root.join("runtime").is_dir());
            root
        }

        let dir = tempfile::tempdir().unwrap();
        let run_id = "RUN-DEGRADED-CONF";
        let manifest = "0".repeat(64);
        // 一条降级形态的 v0.2 卷：降级期间写入者只落骨架事件——终端行
        // run_terminated(journal_degraded) 携带 degraded 摘要；链用
        // seal_event 真实封印（与 recorder 的盘上形态同构）。
        use crate::journal::chain::seal_event;
        use crate::journal::event::Redaction;
        let mut e0 = crate::journal::event::RunEvent::new_v02(
            run_id.into(),
            0,
            crate::journal::event::EventType::RunStarted,
            manifest.clone(),
            None,
            "run-event-v0.2.schema.json".into(),
            serde_json::json!({"prompt": "p"}),
            Redaction::None,
            "2026-09-12T00:00:00Z".into(),
        );
        seal_event(&mut e0).unwrap();
        let h0 = e0.event_sha256.clone();
        let mut e1 = crate::journal::event::RunEvent::new_v02(
            run_id.into(),
            1,
            crate::journal::event::EventType::RunTerminated,
            manifest,
            Some(h0),
            "run-event-v0.2.schema.json".into(),
            serde_json::json!({
                "reason": "journal_degraded",
                "detail": "original terminal run_finished: status=completed",
                "degraded": {
                    "entered_at": "2026-09-12T00:00:00Z",
                    "dropped_events": 7,
                    "first_dropped_sequence": 0,
                    "cause": "storage full",
                },
            }),
            Redaction::None,
            "2026-09-12T00:00:01Z".into(),
        );
        seal_event(&mut e1).unwrap();
        let text = format!(
            "{}
{}
",
            serde_json::to_string(&e0).unwrap(),
            serde_json::to_string(&e1).unwrap()
        );
        let journal = dir.path().join("events.jsonl");
        std::fs::write(&journal, &text).unwrap();

        let report = validate_journal_file(&journal, &repo_root());
        assert!(
            report.valid,
            "a degraded journal is chain-valid, not invalid: {:?}",
            report.errors
        );
        assert!(
            report.degraded_complete,
            "the degraded evidence must classify degraded_complete"
        );

        // 历史卷（无降级标记）分类不变：chain-valid → degraded_complete: false。
        let plain = dir.path().join("plain.jsonl");
        std::fs::copy(
            repo_root().join("runtime/fixtures/run-event-v0.2/journals/plain-run.jsonl"),
            &plain,
        )
        .unwrap();
        let plain_report = validate_journal_file(&plain, &repo_root());
        assert!(plain_report.valid, "{:?}", plain_report.errors);
        assert!(!plain_report.degraded_complete);
    }

    #[test]
    fn track_pair_classification() {
        let v01 = serde_json::json!({
            "schema_version": "0.1.0-draft",
            "payload_schema": "run-event-v0.1.schema.json"
        });
        let v02 = serde_json::json!({
            "schema_version": "0.2.0-draft",
            "payload_schema": "run-event-v0.2.schema.json"
        });
        assert_eq!(track_of(&v01).unwrap(), EventTrack::V01);
        assert_eq!(track_of(&v02).unwrap(), EventTrack::V02);
        let mixed = serde_json::json!({
            "schema_version": "0.2.0-draft",
            "payload_schema": "run-event-v0.1.schema.json"
        });
        assert!(track_of(&mixed).is_err());
    }
}
