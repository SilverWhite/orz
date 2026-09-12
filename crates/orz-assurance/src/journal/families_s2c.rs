//! Task D S2c (2026-09-06): the remaining rule-family verifiers of the
//! v0.2 track — the Rust conformance counterparts of the S2a inventory's
//! S2c batches (retrieval ×8, context & compaction ×7, control plane ×9),
//! mirroring `_verify_v02_*` in `assurance/run_event_journal_validation.py`.
//! 25 since 0x S2 (2026-09-11) added `initial_round_inquiry` (ADR-0010
//! §14.66).
//!
//! Semantics mirror the Python judge rule for rule (single pass over journal
//! order, per-run / per-activation state machines, mechanical recomputation
//! of digests via [`super::chain::canonical_json`]). Message TEXT follows the
//! Rust form; acceptance is verdict parity with the Python judge on the same
//! corpus (families.rs crosscheck test).
//!
//! Track gating mirrors Python per family: most families filter `_is_v02`
//! per event (no-ops on a V01 journal), but `candidate_count` and
//! `inject_budget` match `tool_completed` only — like S2b's
//! `policy_denial` / `failure_target`, they run on V01 journals too.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use super::chain::{canonical_json, sha256_hex};
use super::families::{is_v02, py_int, str_of, toolsets};

// ── shared helpers ──────────────────────────────────────────────────────

/// Python truthiness for a payload field (used by `context_compressed` /
/// `probe_accuracy` where the judge relies on truthiness, not `is True`).
fn py_truthy(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_none_or(|f| f != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

/// Python `.get()` view: missing and explicit null both read as None.
fn py_none(value: Option<&Value>) -> Option<&Value> {
    value.filter(|v| !v.is_null())
}

/// Python `==` on JSON-loaded values: numbers compare numerically across
/// int/float, bool coerces to int (True == 1), containers compare
/// recursively / order-insensitively for objects.
fn py_value_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => py_value_eq_inner(x, y),
        _ => false,
    }
}

fn py_value_eq_inner(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Bool(x), Value::Number(y)) | (Value::Number(y), Value::Bool(x)) => {
            Some((*x as i64) as f64) == y.as_f64()
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(u, v)| py_value_eq_inner(u, v))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| py_value_eq_inner(v, w)))
        }
        _ => false,
    }
}

/// Python `_payload_sha256`: canonical-bytes SHA-256 (Rust-parity form).
fn payload_sha256(payload: &Value) -> String {
    let bytes = canonical_json(payload).unwrap_or_default();
    sha256_hex(&bytes)
}

fn digest16(digest: &str) -> &str {
    &digest[..digest.len().min(16)]
}

/// Python value-comparison view of a JSON value for `==` / `<=` contexts:
/// integers pass through, zero-fraction floats normalize to their integer
/// value (`1.0 == 1` holds in Python), bools coerce to 0/1 (`True == 1`).
/// NOT for `isinstance(int)` checks — there [`super::families::py_int`]
/// (None on float) is the faithful mirror (S2c review P2, 2026-09-06).
fn py_int_value(value: Option<&Value>) -> Option<i64> {
    match value {
        Some(Value::Number(n)) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64)),
        Some(Value::Bool(b)) => Some(*b as i64),
        _ => None,
    }
}

// ── S2c-1 retrieval families ────────────────────────────────────────────

/// Python `_verify_v02_result_consistency` (ADR-0010 §3.3.3/§3.7.5): source
/// counts = mechanical ledger distribution, `visibility_degraded` == "no
/// text-level evidence", digest/`result_id`/assessment binding facts.
pub fn verify_result_consistency(events: &[Value]) -> Vec<String> {
    const VISIBILITIES: &[&str] = &[
        "full_text_observed",
        "partial_text_observed",
        "metadata_only",
        "unavailable",
    ];
    const CLAIM_BY_VISIBILITY: &[(&str, &str)] = &[
        ("full_text_observed", "observed"),
        ("partial_text_observed", "derived"),
        ("metadata_only", "synthesized"),
        ("unavailable", "none"),
    ];

    let mut errors = Vec::new();
    let mut commits: Vec<(usize, &Value)> = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if is_v02(event)
            && event.get("event_type").and_then(Value::as_str) == Some("retrieval_result_committed")
        {
            commits.push((index, event));
        }
    }

    // (activation_id, contract_revision) -> [(index, event)]
    let mut assessments_by_key: std::collections::HashMap<(String, i64), Vec<(usize, &Value)>> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str)
                != Some("information_sufficiency_assessment")
        {
            continue;
        }
        let payload = &event["payload"];
        let activation = str_of(payload.get("activation_id"))
            .unwrap_or_default()
            .to_string();
        let revision = py_int_value(payload.get("contract_revision")).unwrap_or(i64::MIN);
        assessments_by_key
            .entry((activation, revision))
            .or_default()
            .push((index, event));
    }

    for (index, event) in commits {
        let payload = &event["payload"];
        let ledger = match payload.get("source_ledger").and_then(Value::as_array) {
            Some(l) => l,
            None => continue, // payload schema violations are reported elsewhere
        };
        let mut counted: std::collections::BTreeMap<&str, i64> =
            VISIBILITIES.iter().map(|&vis| (vis, 0)).collect();
        for entry in ledger {
            let vis = str_of(entry.get("visibility")).unwrap_or_default();
            if let Some(slot) = counted.get_mut(vis) {
                *slot += 1;
            } else {
                errors.push(format!(
                    "event {index}: source {} has unknown visibility {vis:?}",
                    str_of(entry.get("source_id")).unwrap_or("<missing>")
                ));
            }
        }
        let mut expected_counts = serde_json::Map::new();
        for (vis, count) in &counted {
            expected_counts.insert((*vis).to_string(), Value::from(*count));
        }
        expected_counts.insert("total".into(), Value::from(ledger.len()));
        let expected_counts = Value::Object(expected_counts);
        if !py_value_eq(
            Some(&expected_counts),
            py_none(payload.get("source_counts")),
        ) {
            errors.push(format!(
                "event {index}: source_counts {} != mechanical distribution \
                 {expected_counts} of source_ledger",
                payload
                    .get("source_counts")
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            ));
        }

        // D1 (review 2026-08-10): highest_allowed_claim is a mechanical
        // projection of visibility (§3.7.5).
        for entry in ledger {
            let Some(declared) = py_none(entry.get("highest_allowed_claim")) else {
                continue;
            };
            let vis = str_of(entry.get("visibility")).unwrap_or_default();
            let Some((_, expected)) = CLAIM_BY_VISIBILITY.iter().find(|(v, _)| *v == vis) else {
                continue; // unknown visibility already reported above
            };
            if str_of(Some(declared)) != Some(*expected) {
                errors.push(format!(
                    "event {index}: source {} highest_allowed_claim {declared} \
                     != mechanical cap {expected:?} for visibility {vis:?}",
                    str_of(entry.get("source_id")).unwrap_or("<missing>")
                ));
            }
        }

        // 方向 C (2026-08-30): organized_response retired; visibility_degraded
        // must equal "no text-level evidence".
        if payload.get("organized_response").is_some() {
            errors.push(format!(
                "event {index}: organized_response is retired \
                 (GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
            ));
        }
        let full = py_int_value(
            payload
                .get("source_counts")
                .and_then(|c| c.get("full_text_observed")),
        );
        let partial = py_int_value(
            payload
                .get("source_counts")
                .and_then(|c| c.get("partial_text_observed")),
        );
        let no_text_evidence = full == Some(0) && partial == Some(0);
        if py_truthy(payload.get("visibility_degraded")) != no_text_evidence {
            errors.push(format!(
                "event {index}: visibility_degraded {} != no-text-evidence flag \
                 {no_text_evidence} (full={full:?}, partial={partial:?})",
                payload
                    .get("visibility_degraded")
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            ));
        }

        // 方向 C review round (2026-08-30): result_digest = canonical SHA-256
        // of the four mechanical segments; ledger_digest of source_ledger;
        // result_id carries the digest prefix.
        let mut segments = serde_json::Map::new();
        for field in [
            "query_summary",
            "source_ledger",
            "filtering_log",
            "raw_source_refs",
        ] {
            segments.insert(
                field.to_string(),
                payload.get(field).cloned().unwrap_or(Value::Null),
            );
        }
        let expected_digest = payload_sha256(&Value::Object(segments));
        let result_digest = str_of(payload.get("result_digest")).unwrap_or_default();
        if result_digest != expected_digest {
            errors.push(format!(
                "event {index}: result_digest does not match the canonical \
                 four-segment digest (expected {}…)",
                digest16(&expected_digest)
            ));
        }
        let expected_ledger = payload_sha256(payload.get("source_ledger").unwrap_or(&Value::Null));
        let ledger_digest = str_of(payload.get("ledger_digest")).unwrap_or_default();
        if ledger_digest != expected_ledger {
            errors.push(format!(
                "event {index}: ledger_digest does not match the canonical \
                 source_ledger digest (expected {}…)",
                digest16(&expected_ledger)
            ));
        }
        let result_id = str_of(payload.get("result_id")).unwrap_or_default();
        let prefix = format!("RET-RES-{}-", digest16(result_digest));
        if !result_id.starts_with(&prefix) {
            errors.push(format!(
                "event {index}: result_id {result_id:?} does not carry the \
                 result_digest prefix {}",
                digest16(result_digest)
            ));
        }

        let key = (
            str_of(payload.get("activation_id"))
                .unwrap_or_default()
                .to_string(),
            py_int_value(payload.get("contract_revision")).unwrap_or(i64::MIN),
        );
        for &(a_index, a_event) in assessments_by_key.get(&key).into_iter().flatten() {
            if a_index < index {
                continue;
            }
            let a_payload = &a_event["payload"];
            let a_id = str_of(a_payload.get("assessment_id")).unwrap_or_default();
            if str_of(a_payload.get("result_digest")) != Some(result_digest) {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} result_digest != committed \
                     result {result_id} (event {index})"
                ));
            }
            if str_of(a_payload.get("ledger_digest")) != Some(ledger_digest) {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} ledger_digest != committed \
                     result {result_id} (event {index})"
                ));
            }
            let assess_prefix = format!("ASSESS-{}-", digest16(result_digest));
            if !a_id.starts_with(&assess_prefix) {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} id does not carry the \
                     committed result_digest prefix {}",
                    digest16(result_digest)
                ));
            }
            let codes: Vec<&str> = a_payload
                .get("reason_codes")
                .and_then(Value::as_array)
                .map(|list| list.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let has_no_fulltext = codes.contains(&"no_fulltext_evidence");
            // Python keys these checks on the DECLARED visibility_degraded
            // truthiness, not the derived no-text-evidence flag.
            let declared_degraded = py_truthy(payload.get("visibility_degraded"));
            if declared_degraded && !has_no_fulltext {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} for a degraded committed \
                     result must carry reason code 'no_fulltext_evidence'"
                ));
            }
            if !declared_degraded && has_no_fulltext {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} claims 'no_fulltext_evidence' \
                     for a committed result with text-level evidence"
                ));
            }
            if !py_value_eq(
                py_none(a_payload.get("source_counts")),
                py_none(payload.get("source_counts")),
            ) {
                errors.push(format!(
                    "event {a_index}: assessment {a_id} source_counts {} != \
                     committed result {result_id} (event {index})",
                    a_payload
                        .get("source_counts")
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_reason_codes` (方向 C review round, 2026-08-30): the
/// v0.2 retrieval assessment reason_codes vocabulary is mechanical — only the
/// producer's two codes are legal; the block-era codes are retired.
pub fn verify_reason_codes(events: &[Value]) -> Vec<String> {
    const ALLOWED: &[&str] = &["no_mechanical_coverage_requirement", "no_fulltext_evidence"];
    const RETIRED: &[&str] = &[
        "structured_result_validation_failed",
        "low_quality_source_without_annotation",
    ];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str)
                != Some("information_sufficiency_assessment")
        {
            continue;
        }
        let codes = event["payload"]
            .get("reason_codes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for code in &codes {
            let code = str_of(Some(code)).unwrap_or_default();
            if RETIRED.contains(&code) {
                errors.push(format!(
                    "event {index}: reason code {code:?} is retired \
                     (GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
                ));
            } else if !ALLOWED.contains(&code) {
                let mut sorted_allowed = ALLOWED.to_vec();
                sorted_allowed.sort_unstable();
                errors.push(format!(
                    "event {index}: unknown retrieval assessment reason code \
                     {code:?} (allowed: {sorted_allowed:?})"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_source_weighting` (FUS-SOURCE-WEIGHTING): mechanical
/// tier/weight facts on committed retrieval results — web_page entries carry
/// a tier, mechanical fields travel together against the fixed tier/weight
/// table, and the retired model-annotation vocabulary is absent.
pub fn verify_source_weighting(events: &[Value]) -> Vec<String> {
    const WEIGHT_BY_TIER: &[(&str, f64)] = &[
        ("authoritative", 1.1),
        ("default", 1.0),
        ("low_quality", 0.7),
    ];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("retrieval_result_committed")
        {
            continue;
        }
        let payload = &event["payload"];
        let Some(ledger) = payload.get("source_ledger").and_then(Value::as_array) else {
            continue;
        };
        for entry in ledger {
            let sid = str_of(entry.get("source_id")).unwrap_or("<missing>");
            let tier = py_none(entry.get("tier"));
            let mechanical_weight = py_none(entry.get("mechanical_weight"));
            let weight_reason = py_none(entry.get("weight_reason"));
            let has_any_mechanical =
                tier.is_some() || mechanical_weight.is_some() || weight_reason.is_some();
            if entry.get("source_type").and_then(Value::as_str) == Some("web_page")
                && tier.is_none()
            {
                errors.push(format!(
                    "event {index}: source {sid} is web_page but carries no mechanical tier"
                ));
            } else if has_any_mechanical {
                if tier.is_none() || mechanical_weight.is_none() || weight_reason.is_none() {
                    errors.push(format!(
                        "event {index}: source {sid} has partial mechanical weighting fields"
                    ));
                } else {
                    let tier_str = tier.and_then(Value::as_str);
                    let expected = tier_str
                        .and_then(|t| WEIGHT_BY_TIER.iter().find(|(name, _)| *name == t))
                        .map(|(_, w)| *w);
                    let weight = mechanical_weight.and_then(Value::as_f64);
                    // Python: _WEIGHT_BY_TIER.get(tier) != mechanical_weight —
                    // an unknown tier yields None != weight → violation; an
                    // int weight equals its float table value (1 == 1.0).
                    if expected.is_none() || weight.is_none() || weight != expected {
                        errors.push(format!(
                            "event {index}: source {sid} tier {tier:?} weight \
                             {mechanical_weight:?} violates the fixed tier/weight table"
                        ));
                    }
                }
            }
            let retired: Vec<&str> = ["model_weight", "model_weight_reason", "annotation_status"]
                .into_iter()
                .filter(|field| entry.get(field).is_some())
                .collect();
            if !retired.is_empty() {
                errors.push(format!(
                    "event {index}: source {sid} carries retired model annotation \
                     fields {retired:?} (GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C)"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_search_candidate_pool` (FUS-RETRIEVAL-MECH B-1 +
/// step 3): the citation candidate pool on committed retrieval results —
/// web_search_result-only, non-empty unique strings (empty only via
/// mechanical purification), per-candidate metadata against the fixed
/// tier/weight table, and raw_source_refs mirroring.
pub fn verify_search_candidate_pool(events: &[Value]) -> Vec<String> {
    const TIER_WEIGHT: &[(&str, f64)] = &[
        ("authoritative", 1.1),
        ("default", 1.0),
        ("low_quality", 0.7),
    ];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("retrieval_result_committed")
        {
            continue;
        }
        let payload = &event["payload"];
        let Some(ledger) = payload.get("source_ledger").and_then(Value::as_array) else {
            continue;
        };
        let refs = payload.get("raw_source_refs").and_then(Value::as_array);
        let ledger_by_id: std::collections::HashMap<&str, &Value> = ledger
            .iter()
            .filter_map(|entry| str_of(entry.get("source_id")).map(|sid| (sid, entry)))
            .collect();
        let ref_by_id: std::collections::HashMap<&str, &Value> = refs
            .into_iter()
            .flatten()
            .filter_map(|reference| str_of(reference.get("source_id")).map(|sid| (sid, reference)))
            .collect();

        for entry in ledger {
            let sid = str_of(entry.get("source_id")).unwrap_or("<missing>");
            let candidates = py_none(entry.get("candidate_urls"));
            let pool = py_none(entry.get("candidate_pool"));
            if candidates.is_none() && pool.is_some() {
                errors.push(format!(
                    "event {index}: source {sid} carries candidate_pool without \
                     candidate_urls (the fields travel together)"
                ));
                continue;
            }
            let Some(candidates_value) = candidates else {
                continue; // absent candidate_urls: silent on both judges
            };
            let Some(candidates) = candidates_value.as_array() else {
                // Python 1571-1578: a present, non-array candidate_urls is the
                // same "list of non-empty strings" violation — never silent.
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls must be a list of \
                     non-empty strings"
                ));
                continue;
            };
            if entry.get("source_type").and_then(Value::as_str) != Some("web_search_result") {
                errors.push(format!(
                    "event {index}: source {sid} carries candidate_urls but \
                     source_type is {:?} (only web_search_result may carry a \
                     candidate pool)",
                    entry
                        .get("source_type")
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                ));
            }
            let url_list: Option<Vec<&str>> = candidates.iter().map(|u| str_of(Some(u))).collect();
            let Some(url_list) = url_list else {
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls must be a list of \
                     non-empty strings"
                ));
                continue;
            };
            if url_list.iter().any(|url| url.is_empty()) {
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls must be a list of \
                     non-empty strings"
                ));
                continue;
            }
            if candidates.is_empty() {
                let removals_for_source = payload
                    .get("prefilter_log")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter(|item| py_value_eq(item.get("source_id"), Some(&entry["source_id"])))
                    .count();
                if removals_for_source == 0 {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_urls is empty with no \
                         prefilter_log removal — an empty retained pool must be the \
                         result of mechanical purification"
                    ));
                }
            }
            let unique: BTreeSet<&str> = url_list.iter().copied().collect();
            if unique.len() != url_list.len() {
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls contains duplicates"
                ));
            }
            let Some(pool) = pool.and_then(Value::as_array) else {
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls without candidate_pool \
                     (the prefiltered pool must carry per-candidate metadata)"
                ));
                continue;
            };
            if pool.len() != candidates.len() {
                errors.push(format!(
                    "event {index}: source {sid} candidate_pool must mirror \
                     candidate_urls exactly (same length, same order)"
                ));
                continue;
            }
            for (i, item) in pool.iter().enumerate() {
                if !item.is_object() {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] is not an object"
                    ));
                    continue;
                }
                let url = str_of(item.get("url"));
                if url != Some(url_list[i]) {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] url {url:?} \
                         does not match candidate_urls[{i}] {:?}",
                        url_list[i]
                    ));
                }
                let canonical = str_of(item.get("canonical_url"));
                if canonical.is_none_or(str::is_empty) {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] canonical_url \
                         must be a non-empty string"
                    ));
                }
                let tier = str_of(item.get("tier"));
                let expected_weight = tier
                    .and_then(|t| TIER_WEIGHT.iter().find(|(name, _)| *name == t))
                    .map(|(_, w)| *w);
                let weight = item.get("mechanical_weight").and_then(Value::as_f64);
                match (tier, expected_weight) {
                    (None, _) | (Some(_), None) => {
                        errors.push(format!(
                            "event {index}: source {sid} candidate_pool[{i}] invalid \
                             tier {tier:?}"
                        ));
                    }
                    (Some(_), Some(expected)) => {
                        if weight != Some(expected) {
                            errors.push(format!(
                                "event {index}: source {sid} candidate_pool[{i}] \
                                 mechanical_weight {:?} does not match tier {tier:?}",
                                item.get("mechanical_weight")
                            ));
                        }
                    }
                }
                let weight_reason = str_of(item.get("weight_reason"));
                if weight_reason.is_none_or(str::is_empty) {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] weight_reason \
                         must be a non-empty string"
                    ));
                }
                if !matches!(
                    str_of(item.get("relevance")),
                    Some("direct") | Some("partial") | Some("tangential")
                ) {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] invalid \
                         relevance {:?}",
                        item.get("relevance")
                            .map(|v| v.to_string())
                            .unwrap_or_default()
                    ));
                }
                let form_reasons_ok = item
                    .get("form_reasons")
                    .and_then(Value::as_array)
                    .is_some_and(|list| {
                        list.iter()
                            .all(|r| str_of(Some(r)).is_some_and(|s| !s.is_empty()))
                    });
                if !form_reasons_ok {
                    errors.push(format!(
                        "event {index}: source {sid} candidate_pool[{i}] form_reasons \
                         must be a list of non-empty strings"
                    ));
                }
            }
            let reference = ref_by_id.get(sid);
            let mirrors = reference.is_some_and(|reference| {
                py_value_eq(
                    py_none(reference.get("candidate_urls")),
                    Some(&entry["candidate_urls"]),
                ) && py_value_eq(
                    py_none(reference.get("candidate_pool")),
                    Some(&entry["candidate_pool"]),
                )
            });
            if !mirrors {
                errors.push(format!(
                    "event {index}: source {sid} candidate_urls/candidate_pool not \
                     mirrored in raw_source_refs"
                ));
            }
        }

        for reference in refs.into_iter().flatten() {
            let sid = str_of(reference.get("source_id")).unwrap_or("<missing>");
            let entry = ledger_by_id.get(sid);
            for field in ["candidate_urls", "candidate_pool"] {
                // Python `if field in ref` — explicit null counts as present.
                if reference.get(field).is_some() {
                    let matches = entry.is_some_and(|entry| {
                        py_value_eq(py_none(entry.get(field)), py_none(reference.get(field)))
                    });
                    if !matches {
                        errors.push(format!(
                            "event {index}: raw_source_refs {sid} {field} does not \
                             match the ledger"
                        ));
                    }
                }
            }
        }
    }
    errors
}

/// Python `_verify_v02_candidate_prefilter` (FUS-RETRIEVAL-MECH P0-B step 3):
/// the mechanical prefilter removal log — required whenever a pool is
/// present, closed reason vocabulary, (source_id, url, reason) recorded at
/// most once, and non-duplicate removals must not stay retained.
pub fn verify_candidate_prefilter(events: &[Value]) -> Vec<String> {
    const REMOVAL_REASONS: &[&str] = &[
        "bad_url",
        "login_wall",
        "redirect_chain",
        "duplicate_canonical",
        "duplicate_host",
    ];
    const DUPLICATE_REASONS: &[&str] = &["duplicate_canonical", "duplicate_host"];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("retrieval_result_committed")
        {
            continue;
        }
        let payload = &event["payload"];
        let Some(ledger) = payload.get("source_ledger").and_then(Value::as_array) else {
            continue;
        };
        let ledger_by_id: std::collections::HashMap<&str, &Value> = ledger
            .iter()
            .filter_map(|entry| str_of(entry.get("source_id")).map(|sid| (sid, entry)))
            .collect();
        let has_pool = ledger
            .iter()
            .any(|entry| py_none(entry.get("candidate_urls")).is_some());
        let log = py_none(payload.get("prefilter_log"));
        let Some(log) = log else {
            if has_pool {
                errors.push(format!(
                    "event {index}: prefilter_log missing while a candidate pool is present"
                ));
            }
            continue;
        };
        let Some(log) = log.as_array() else {
            errors.push(format!("event {index}: prefilter_log must be an array"));
            continue;
        };
        let mut retained_by_source: std::collections::HashMap<&str, Vec<&str>> =
            std::collections::HashMap::new();
        for entry in ledger {
            if let Some(candidates) = py_none(entry.get("candidate_urls")).and_then(Value::as_array)
            {
                let urls: Vec<&str> = candidates.iter().filter_map(Value::as_str).collect();
                if let Some(sid) = str_of(entry.get("source_id")) {
                    retained_by_source.insert(sid, urls);
                }
            }
        }
        let mut seen: BTreeSet<(String, String, String)> = BTreeSet::new();
        for item in log {
            let sid = str_of(item.get("source_id"));
            let url = str_of(item.get("url"));
            let reason = str_of(item.get("reason"));
            if sid.is_none() || url.is_none_or(str::is_empty) {
                errors.push(format!(
                    "event {index}: prefilter_log entry needs source_id and a \
                     non-empty url"
                ));
                continue;
            }
            let (sid, url) = (sid.unwrap_or_default(), url.unwrap_or_default());
            let Some(reason) = reason else {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} invalid removal reason {:?}",
                    item.get("reason")
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                ));
                continue;
            };
            if !REMOVAL_REASONS.contains(&reason) {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} invalid removal reason {reason:?}"
                ));
                continue;
            }
            if item.get("action").and_then(Value::as_str) != Some("removed") {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} action must be 'removed', \
                     got {:?}",
                    item.get("action")
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                ));
            }
            if let Some(canonical) = py_none(item.get("canonical_url"))
                && str_of(Some(canonical)).is_none_or(str::is_empty)
            {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} canonical_url must be a \
                     non-empty string when present"
                ));
            }
            let key = (sid.to_string(), url.to_string(), reason.to_string());
            if !seen.insert(key) {
                errors.push(format!(
                    "event {index}: duplicate prefilter_log removal {sid} {url} {reason}"
                ));
            }
            let entry = ledger_by_id.get(sid).copied();
            let entry_ok = entry.is_some_and(|entry| {
                entry.get("source_type").and_then(Value::as_str) == Some("web_search_result")
                    && py_none(entry.get("candidate_urls")).is_some()
            });
            if !entry_ok {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} does not reference a \
                     web_search_result entry carrying a candidate pool"
                ));
                continue;
            }
            if !DUPLICATE_REASONS.contains(&reason)
                && retained_by_source
                    .get(sid)
                    .is_some_and(|retained| retained.contains(&url))
            {
                errors.push(format!(
                    "event {index}: prefilter_log {sid} removed {url} ({reason}) but \
                     the URL is still retained"
                ));
            }
        }
    }
    errors
}

const TERMINAL_TYPES: &[&str] = &[
    "run_finished",
    "run_failed",
    "run_cancelled",
    "run_invalidated",
    // FUS-HOST-RESOURCE-SAFETY §4.3 (2026-09-12, 0z S2): the explicit
    // terminal shape for resource-exhausted / journal-degraded endings
    // (mirror of conformance.rs TERMINAL_TYPES and the Python frozen
    // reference's _TERMINAL_TYPES).
    "run_terminated",
];

/// Python `_verify_v02_receipt_event_isomorphism` (F11 §5.4, B 族): receipt ↔
/// event-chain pairing — failure completions must carry gate evidence
/// (policy_denial marker or a structured denial code, start optional but
/// never after) or a preceding tool_started; a completed execution is
/// journaled at most once per call; and a run that reached a non-
/// run_invalidated terminal closes every tool_started (the wall-clock-kill
/// in-flight exemption). The `in_flight_tools` orphan-repair mechanism of
/// agent_loop.rs is the producer side of that exemption (S2a #1) — this
/// verifier is the independent journal-side check.
pub fn verify_receipt_event_isomorphism(events: &[Value]) -> Vec<String> {
    use crate::lif::channels::is_denial_code;

    type Key = (String, String, String);
    let mut errors = Vec::new();
    let mut started: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
    let mut completed: std::collections::HashMap<Key, Vec<usize>> =
        std::collections::HashMap::new();
    let mut terminal_type: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event
            .get("event_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let run = event
            .get("run_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if TERMINAL_TYPES.contains(&event_type) {
            terminal_type.insert(run, event_type.to_string());
        } else if event_type == "tool_started" {
            let key: Key = (
                run,
                str_of(payload.get("tool")).unwrap_or("").to_string(),
                str_of(payload.get("call_id")).unwrap_or("").to_string(),
            );
            started.entry(key).or_insert(index);
        } else if event_type == "tool_completed" {
            let key: Key = (
                run,
                str_of(payload.get("tool")).unwrap_or("").to_string(),
                str_of(payload.get("call_id")).unwrap_or("").to_string(),
            );
            completed.entry(key).or_default().push(index);
        }
    }

    let is_gate = |payload: &Value| {
        py_none(payload.get("policy_denial")).is_some()
            || str_of(payload.get("error")).is_some_and(is_denial_code)
    };
    for (key, endings) in &completed {
        if endings.len() > 1 {
            errors.push(format!(
                "event {}: duplicate tool_completed for {}/{} — a receipt segment \
                 is journaled at most once per call",
                endings[0], key.1, key.2
            ));
            continue;
        }
        let index = endings[0];
        let payload = events[index].get("payload").cloned().unwrap_or(Value::Null);
        // §5.4 的 receipt 段（arg_validation/gate/execution/delivery）都是失败
        // 段——配对检查只对失败完成生效；成功完成由运行时结构保证。
        if payload.get("status").and_then(Value::as_str) != Some("error") {
            continue;
        }
        let start_index = started.get(key).copied();
        if is_gate(&payload) {
            if let Some(start) = start_index
                && start > index
            {
                errors.push(format!(
                    "event {index}: gate refusal completion precedes its tool_started \
                     (receipt order broken) for {}/{}",
                    key.1, key.2
                ));
            }
            continue;
        }
        match start_index {
            None => {
                errors.push(format!(
                    "event {index}: non-gate completion without a preceding \
                     tool_started (execution segment missing) for {}/{}",
                    key.1, key.2
                ));
            }
            Some(start) if start > index => {
                errors.push(format!(
                    "event {index}: tool_completed precedes its tool_started (receipt \
                     order broken) for {}/{}",
                    key.1, key.2
                ));
            }
            Some(_) => {}
        }
    }

    for (key, start_index) in &started {
        // 0z S2 review F-C-3 (2026-09-13): `run_terminated` joins the
        // exemption set — a degraded journal drops tool_completed rows by
        // design (skeleton-only), and a hard-tier tree kill legitimately
        // leaves in-flight tools without completions (§4.8 表 1). Both are
        // the same wall-clock-kill family of exemptions.
        let exempt = match terminal_type.get(&key.0) {
            None => true, // interrupted run — no completion obligation
            Some(term) => term == "run_invalidated" || term == "run_terminated", // 墙钟超时豁免 (S4) / 0z 降级与树杀豁免
        };
        if exempt {
            continue;
        }
        if !completed.contains_key(key) {
            errors.push(format!(
                "event {start_index}: tool_started {}/{} has no matching \
                 tool_completed in run {} that reached {} (execution receipt \
                 incomplete)",
                key.1,
                key.2,
                key.0,
                terminal_type.get(&key.0).map(String::as_str).unwrap_or("?")
            ));
        }
    }
    errors
}

/// Python `_is_candidate_counted_tool` (P0-B step 2/4): web_fetch variants
/// plus browser_read share the per-activation count domain.
fn is_candidate_counted_tool(name: Option<&str>) -> bool {
    matches!(name, Some(name) if name == "web_fetch" || name.starts_with("web_fetch_") || name == "browser_read")
}

const CANDIDATE_CAP_EXCEEDED_CODES: &[&str] = &[
    "web_fetch_candidate_cap_exceeded",
    "browser_read_candidate_cap_exceeded",
];

/// Python `_verify_v02_candidate_count` (P0-B step 2/4): candidate count
/// fields travel together, only on the candidate-counted family, within
/// 0 <= count <= cap (cap >= 1); non-error lane completions must carry them
/// and cap-exceeded refusals sit exactly at the cap boundary. NOTE: like the
/// Python judge, this family does NOT filter `_is_v02` — it runs on V01
/// journals too (S2b review P1 lineage).
pub fn verify_candidate_count(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("tool_completed") {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let tool = str_of(payload.get("tool"));
        let has_count = payload.get("candidate_count").is_some();
        let has_cap = payload.get("candidate_cap").is_some();
        if has_count != has_cap {
            errors.push(format!(
                "event {index}: tool_completed carries candidate_count but not \
                 candidate_cap (or vice versa) — the fields travel together"
            ));
            continue;
        }
        if !has_count {
            if payload.get("status").and_then(Value::as_str) != Some("error")
                && is_candidate_counted_tool(tool)
                && py_none(payload.get("target")).is_none()
            {
                errors.push(format!(
                    "event {index}: non-error lane candidate-counted completion for \
                     {tool:?} must carry candidate_count/candidate_cap"
                ));
            } else if matches!(str_of(payload.get("error")), Some(code)
                if CANDIDATE_CAP_EXCEEDED_CODES.contains(&code))
            {
                errors.push(format!(
                    "event {index}: cap-exceeded refusal must carry \
                     candidate_count/candidate_cap"
                ));
            }
            continue;
        }
        if !is_candidate_counted_tool(tool) {
            errors.push(format!(
                "event {index}: tool {tool:?} carries candidate count fields \
                 (web_fetch/browser_read family only)"
            ));
            continue;
        }
        let count = payload
            .get("candidate_count")
            .and_then(|v| if v.is_boolean() { None } else { v.as_i64() });
        let cap = payload
            .get("candidate_cap")
            .and_then(|v| if v.is_boolean() { None } else { v.as_i64() });
        if count.is_none_or(|c| c < 0) || cap.is_none_or(|c| c < 1) || count > cap {
            errors.push(format!(
                "event {index}: candidate_count {:?} / candidate_cap {:?} out of \
                 range — need 0 <= count <= cap, cap >= 1",
                payload.get("candidate_count"),
                payload.get("candidate_cap")
            ));
        }
        if matches!(str_of(payload.get("error")), Some(code)
            if CANDIDATE_CAP_EXCEEDED_CODES.contains(&code))
        {
            if payload.get("status").and_then(Value::as_str) != Some("error") {
                errors.push(format!(
                    "event {index}: cap-exceeded refusal must be status=error"
                ));
            }
            if count != cap {
                errors.push(format!(
                    "event {index}: cap-exceeded refusal candidate_count {count:?} \
                     must equal candidate_cap {cap:?} (refusal happens at the cap \
                     boundary)"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_inject_budget` (ORZ-CACHE-CONTEXT-COST, 2026-08-15
/// review fix): the injection-budget fields travel together, carry used >= 0
/// and budget >= 1 on `round_inject_budget_exceeded`, and are only legal with
/// that error code. NOTE: like the Python judge, this family does NOT filter
/// `_is_v02` — it runs on V01 journals too.
pub fn verify_inject_budget(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if event.get("event_type").and_then(Value::as_str) != Some("tool_completed") {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let error = str_of(payload.get("error")).unwrap_or("");
        let used = payload.get("inject_tokens_used");
        let budget = payload.get("inject_tokens_budget");
        let has_used = used.is_some();
        let has_budget = budget.is_some();
        if error == "round_inject_budget_exceeded" {
            if !has_used || !has_budget {
                errors.push(format!(
                    "event {index}: round_inject_budget_exceeded must carry \
                     inject_tokens_used and inject_tokens_budget"
                ));
                continue;
            }
            // Python: isinstance(int) and not isinstance(bool) and range.
            let used_int = used.and_then(|v| if v.is_boolean() { None } else { v.as_i64() });
            let budget_int = budget.and_then(|v| if v.is_boolean() { None } else { v.as_i64() });
            if used_int.is_none_or(|v| v < 0) || budget_int.is_none_or(|v| v < 1) {
                errors.push(format!(
                    "event {index}: inject_tokens_used {:?} / inject_tokens_budget \
                     {:?} out of range — need used >= 0, budget >= 1",
                    used, budget
                ));
            }
            continue;
        }
        if has_used != has_budget {
            errors.push(format!(
                "event {index}: tool_completed carries inject_tokens_used but not \
                 inject_tokens_budget (or vice versa) — the fields travel together"
            ));
        } else if has_used {
            errors.push(format!(
                "event {index}: inject_tokens_used/inject_tokens_budget are only \
                 legal with error=round_inject_budget_exceeded (got {error:?})"
            ));
        }
    }
    errors
}

// ── S2c-2 context & compaction families ─────────────────────────────────

/// Python `_verify_v02_recovery_truncation` (D2-2): the recovery pre-check
/// event precedes the first model_request and drops at least one whole round.
pub fn verify_recovery_truncation(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_model_request = false;
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event.get("event_type").and_then(Value::as_str);
        if event_type == Some("model_request") {
            seen_model_request = true;
            continue;
        }
        if event_type != Some("context_recovery_truncated") {
            continue;
        }
        if seen_model_request {
            errors.push(format!(
                "event {index}: context_recovery_truncated must precede the first \
                 model_request"
            ));
        }
        match py_int_value(
            event
                .get("payload")
                .unwrap_or(&Value::Null)
                .get("rounds_dropped"),
        ) {
            Some(rounds) if rounds > 0 => {}
            other => {
                errors.push(format!(
                    "event {index}: context_recovery_truncated must drop at least \
                     one whole round (got {other:?})"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_context_compressed` (P0-D S3 + review fix): mode and
/// reason vocabularies, guard_failed only on rhythm/fallback, mechanical
/// never incomplete, incomplete ⇔ null archive fields, archive_write_failed
/// only on a complete summary.
pub fn verify_context_compressed(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("context_compressed")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let mode = str_of(payload.get("mode"));
        if !matches!(mode, Some("template_summary") | Some("mechanical")) {
            errors.push(format!(
                "event {index}: context_compressed mode must be \
                 template_summary/mechanical"
            ));
        }
        let reason = str_of(payload.get("reason"));
        if !matches!(
            reason,
            Some("rhythm") | Some("fallback") | Some("session_end")
        ) {
            errors.push(format!(
                "event {index}: context_compressed reason must be \
                 rhythm/fallback/session_end"
            ));
        }
        let guard_failed = py_truthy(payload.get("guard_failed"));
        if guard_failed && reason == Some("session_end") {
            errors.push(format!(
                "event {index}: guard_failed may only ride rhythm/fallback \
                 triggers, never session_end"
            ));
        }
        let incomplete = py_truthy(payload.get("summary_incomplete"));
        if mode == Some("mechanical") && incomplete {
            errors.push(format!(
                "event {index}: mechanical compaction must never be \
                 summary_incomplete (no model slots to fail)"
            ));
        }
        let archive_fields = [
            py_none(payload.get("summary_id")),
            py_none(payload.get("summary_digest")),
            py_none(payload.get("summary_path")),
        ];
        if incomplete && archive_fields.iter().any(Option::is_some) {
            errors.push(format!(
                "event {index}: incomplete summary must carry null archive fields"
            ));
        }
        if !incomplete && archive_fields.iter().any(Option::is_none) {
            errors.push(format!(
                "event {index}: complete summary must carry archive id/digest/path"
            ));
        }
        let archive_write_failed = py_truthy(payload.get("archive_write_failed"));
        if archive_write_failed && incomplete {
            errors.push(format!(
                "event {index}: archive_write_failed may only ride a complete \
                 summary (a failed summary never attempts the write)"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_activation_restore` (ADR-0010 §3.3/§4.4): the same
/// activation is restored at most once per journal.
pub fn verify_activation_restore(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str)
                != Some("retrieval_activation_restored")
        {
            continue;
        }
        let Some(activation) = str_of(
            event
                .get("payload")
                .unwrap_or(&Value::Null)
                .get("activation_id"),
        ) else {
            continue;
        };
        if let Some(first) = seen.get(activation) {
            errors.push(format!(
                "event {index}: second restore of activation {activation} (first \
                 at event {first})"
            ));
        } else {
            seen.insert(activation.to_string(), index);
        }
    }
    errors
}

/// Python `_verify_v02_dep_graph_events` (P2-11 依赖图主线, 2026-09-01):
/// journaled read/write facts are self-consistent and chain-consistent —
/// kind→tool family + exit 0, and a write's `consumed_read` anchor edge must
/// point at an earlier same-path same-run read with a matching anchor
/// (sha256 authoritative, size+mtime fallback).
pub fn verify_dep_graph_events(events: &[Value]) -> Vec<String> {
    fn anchor_matches(a: Option<&Value>, b: Option<&Value>) -> bool {
        let (Some(a), Some(b)) = (a, b) else {
            return false;
        };
        if !a.is_object() || !b.is_object() {
            return false;
        }
        let a_sha = str_of(a.get("sha256"));
        let b_sha = str_of(b.get("sha256"));
        if let (Some(a_sha), Some(b_sha)) = (a_sha, b_sha) {
            return a_sha == b_sha;
        }
        py_value_eq(py_none(a.get("size")), py_none(b.get("size")))
            && py_value_eq(py_none(a.get("mtime")), py_none(b.get("mtime")))
    }

    let mut errors = Vec::new();
    // (run, call_id) -> read fact
    let mut reads: std::collections::HashMap<(String, String), Value> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("tool_completed")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let Some(fact) = py_none(payload.get("dep_graph")) else {
            continue;
        };
        if !fact.is_object() {
            errors.push(format!("event {index}: dep_graph must be an object"));
            continue;
        }
        let run = event.get("run_id").and_then(Value::as_str).unwrap_or("");
        let tool = str_of(payload.get("tool")).unwrap_or("");
        let call_id = str_of(payload.get("call_id")).unwrap_or("");
        let kind = str_of(fact.get("kind"));
        let path = str_of(fact.get("path"));
        if !matches!(kind, Some("read") | Some("write")) {
            errors.push(format!(
                "event {index}: dep_graph.kind must be read|write (got {:?})",
                fact.get("kind").map(|v| v.to_string()).unwrap_or_default()
            ));
            continue;
        }
        if path.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: dep_graph.path must be a non-empty string"
            ));
        }
        if !py_value_eq_inner(
            payload.get("exit_code").unwrap_or(&Value::Null),
            &Value::from(0),
        ) {
            errors.push(format!(
                "event {index}: dep_graph present on non-success completion \
                 ({tool}/{call_id}, exit_code={:?})",
                payload
                    .get("exit_code")
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            ));
        }
        if kind == Some("read") {
            if tool != "read_file" {
                errors.push(format!(
                    "event {index}: dep_graph.kind=read on tool {tool:?} (must be \
                     read_file)"
                ));
            }
            let anchor = py_none(fact.get("anchor"));
            if anchor.is_some_and(|a| !a.is_object()) {
                errors.push(format!(
                    "event {index}: dep_graph.anchor must be an object or null"
                ));
            }
            reads.insert((run.to_string(), call_id.to_string()), fact.clone());
        } else {
            if tool != "search_replace" {
                errors.push(format!(
                    "event {index}: dep_graph.kind=write on tool {tool:?} (must be \
                     search_replace)"
                ));
            }
            for key in ["consumed_anchor", "new_anchor"] {
                if py_none(fact.get(key)).is_some_and(|v| !v.is_object()) {
                    errors.push(format!(
                        "event {index}: dep_graph.{key} must be an object or null"
                    ));
                }
            }
            let consumed = py_none(fact.get("consumed_read"));
            // Python 2891-2897: a present-but-empty string takes the
            // "must be a non-empty string or null" branch and never reaches
            // the lookup (S2c review P1, 2026-09-06).
            let Some(consumed) = str_of(consumed).filter(|c| !c.is_empty()) else {
                if consumed.is_some() {
                    errors.push(format!(
                        "event {index}: dep_graph.consumed_read must be a non-empty \
                         string or null"
                    ));
                }
                continue;
            };
            let Some(read) = reads.get(&(run.to_string(), consumed.to_string())) else {
                errors.push(format!(
                    "event {index}: dep_graph.consumed_read={consumed:?} has no \
                     earlier read fact in the same run (anchor edge dangling)"
                ));
                continue;
            };
            if str_of(read.get("path")) != path {
                errors.push(format!(
                    "event {index}: dep_graph anchor edge points at read {consumed} \
                     on a different path ({:?} != {:?})",
                    read.get("path").map(|v| v.to_string()).unwrap_or_default(),
                    path.unwrap_or_default()
                ));
            } else if !anchor_matches(fact.get("consumed_anchor"), read.get("anchor")) {
                errors.push(format!(
                    "event {index}: dep_graph anchor edge {consumed} anchors do not \
                     match (sha256 authoritative; fallback size+mtime)"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_tool_running` (S5-2 + TER T0.2): mid-run reports sit
/// between their call's start and completion (running:true + exit_code null
/// on that completion, exactly one completion), and idle-kill reports
/// reference an auto-backgrounded call and post-date its running completion.
pub fn verify_tool_running(events: &[Value]) -> Vec<String> {
    type Key = (String, String, String);
    let mut errors = Vec::new();
    let mut running_mid: Vec<(usize, &Value)> = Vec::new();
    let mut running_killed: Vec<(usize, &Value)> = Vec::new();
    let mut started: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
    let mut completed: std::collections::HashMap<Key, Vec<usize>> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event.get("event_type").and_then(Value::as_str);
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let key: Key = (
            event
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            str_of(payload.get("tool")).unwrap_or("").to_string(),
            str_of(payload.get("call_id")).unwrap_or("").to_string(),
        );
        if event_type == Some("tool_running") {
            if payload.get("status").and_then(Value::as_str) == Some("idle_killed") {
                running_killed.push((index, event));
            } else {
                running_mid.push((index, event));
            }
        } else if event_type == Some("tool_started") {
            started.entry(key).or_insert(index);
        } else if event_type == Some("tool_completed") {
            completed.entry(key).or_default().push(index);
        }
    }

    let mut seen_mid: BTreeSet<Key> = BTreeSet::new();
    let mut mid_index: std::collections::HashMap<Key, usize> = std::collections::HashMap::new();
    for (index, event) in running_mid {
        let payload = &event["payload"];
        let key: Key = (
            event
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            str_of(payload.get("tool")).unwrap_or("").to_string(),
            str_of(payload.get("call_id")).unwrap_or("").to_string(),
        );
        let call_id = str_of(payload.get("call_id")).unwrap_or("");
        if !seen_mid.insert(key.clone()) {
            errors.push(format!(
                "event {index}: duplicate tool_running for call_id {call_id:?} — at \
                 most one mid-run report per call"
            ));
            continue;
        }
        mid_index.insert(key.clone(), index);
        let start_index = started.get(&key).copied();
        match start_index {
            Some(start) if start <= index => {}
            _ => {
                errors.push(format!(
                    "event {index}: tool_running for call_id {call_id:?} has no \
                     preceding tool_started of the same tool/call_id in its run"
                ));
                continue;
            }
        }
        let all_end = completed.get(&key).cloned().unwrap_or_default();
        if all_end.len() > 1 {
            errors.push(format!(
                "event {index}: call_id {call_id:?} has {} tool_completed events — \
                 exactly one completion per mid-run call (the background terminal \
                 state rides the completion reminder, not a second ToolCompleted)",
                all_end.len()
            ));
        }
        let Some(end_index) = all_end.iter().copied().find(|i| *i > index) else {
            errors.push(format!(
                "event {index}: tool_running for call_id {call_id:?} has no following \
                 tool_completed of the same tool/call_id in its run"
            ));
            continue;
        };
        let cpayload = events[end_index]
            .get("payload")
            .cloned()
            .unwrap_or(Value::Null);
        if cpayload.get("running") != Some(&Value::Bool(true)) {
            errors.push(format!(
                "event {end_index}: tool_completed for call_id {call_id:?} must carry \
                 running:true after a tool_running mid-run report"
            ));
        } else if py_none(cpayload.get("exit_code")).is_some() {
            errors.push(format!(
                "event {end_index}: tool_completed for call_id {call_id:?} carries \
                 running:true but exit_code {:?} — a still-running command must stay \
                 exit_code=null",
                cpayload.get("exit_code")
            ));
        }
    }

    let mut seen_killed: BTreeSet<Key> = BTreeSet::new();
    for (index, event) in running_killed {
        let payload = &event["payload"];
        let key: Key = (
            event
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            str_of(payload.get("tool")).unwrap_or("").to_string(),
            str_of(payload.get("call_id")).unwrap_or("").to_string(),
        );
        let call_id = str_of(payload.get("call_id")).unwrap_or("");
        if !seen_killed.insert(key.clone()) {
            errors.push(format!(
                "event {index}: duplicate idle-kill tool_running for call_id \
                 {call_id:?} — at most one idle-kill per call"
            ));
            continue;
        }
        let reason = str_of(payload.get("reason"));
        if reason.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: idle-killed tool_running for call_id {call_id:?} must \
                 carry a non-empty reason"
            ));
        }
        let Some(mid_run_index) = mid_index.get(&key).copied() else {
            errors.push(format!(
                "event {index}: idle-killed tool_running for call_id {call_id:?} \
                 requires a preceding auto-background mid-run tool_running of the \
                 same tool/call_id in its run"
            ));
            continue;
        };
        let all_end = completed.get(&key).cloned().unwrap_or_default();
        let Some(completion_index) = all_end.iter().copied().find(|i| *i > mid_run_index) else {
            errors.push(format!(
                "event {index}: idle-killed tool_running for call_id {call_id:?} has \
                 no completed auto-bg call to kill"
            ));
            continue;
        };
        if index <= completion_index {
            errors.push(format!(
                "event {index}: idle-killed tool_running for call_id {call_id:?} must \
                 post-date the call's running:true tool_completed at event \
                 {completion_index}"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_output_truncation` (TER W-F13b): the truncation marker
/// carries the true byte count and the retrieval-object pointer requires the
/// explicit truncation fact.
pub fn verify_output_truncation(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("tool_completed")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let truncated = payload.get("output_truncated") == Some(&Value::Bool(true));
        let has_bytes = payload.get("total_bytes").is_some();
        let has_object = payload.get("output_object_id").is_some();
        if truncated && !has_bytes {
            errors.push(format!(
                "event {index}: tool_completed carries output_truncated but no \
                 total_bytes — the true byte count must accompany the truncation \
                 marker"
            ));
        }
        if has_object && !(truncated && has_bytes) {
            errors.push(format!(
                "event {index}: tool_completed carries output_object_id but no \
                 output_truncated+total_bytes pair — the retrieval-object pointer \
                 requires an explicit truncation fact"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_budget_cue_injected` (TER F6 push): at most 4 cues per
/// run and a cue fires only strictly below its tier threshold.
pub fn verify_budget_cue_injected(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("budget_cue_injected")
        {
            continue;
        }
        let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
        *counts.entry(run_id.to_string()).or_default() += 1;
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let remaining = py_int(payload.get("remaining_seconds"));
        let threshold = py_int(payload.get("threshold_seconds"));
        if let (Some(remaining), Some(threshold)) = (remaining, threshold)
            && remaining >= threshold
        {
            errors.push(format!(
                "event {index}: budget_cue_injected remaining_seconds {remaining} is \
                 not below its threshold_seconds {threshold} — cues fire only after \
                 the tier boundary is crossed"
            ));
        }
    }
    for (run_id, count) in &counts {
        if *count > 4 {
            errors.push(format!(
                "run {run_id}: {count} budget_cue_injected events exceed the \
                 ≤4-per-run push cap"
            ));
        }
    }
    errors
}

// ── S2c-3 control-plane families ────────────────────────────────────────

/// Python `_verify_v02_inquiry_kind` (§5.1): the neutral-inquiry payload's
/// const `inquiry_kind` equals the envelope `event_type`.
pub fn verify_inquiry_kind(events: &[Value]) -> Vec<String> {
    const NEUTRAL_INQUIRY_EVENTS: &[&str] = &["orientation_checkpoint"];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let Some(event_type) = event.get("event_type").and_then(Value::as_str) else {
            continue;
        };
        if !NEUTRAL_INQUIRY_EVENTS.contains(&event_type) {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let inquiry_kind = str_of(payload.get("inquiry_kind"));
        if inquiry_kind != Some(event_type) {
            errors.push(format!(
                "event {index}: v0.2 neutral inquiry payload inquiry_kind {:?} != \
                 envelope event_type {event_type:?}",
                payload
                    .get("inquiry_kind")
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            ));
        }
    }
    errors
}

/// P0-0x S2 (ADR-0010 §14.66, 2026-09-11) — `initial_round_inquiry`: the
/// one-shot initial-round inquiry shares the `orientation_checkpoint` event
/// with the periodic threshold inquiry, so `trigger` is the dispatcher and
/// the payload must be self-consistent:
///
/// - `trigger == "initial_round"` ⇒ `message_block` starts with
///   `[INITIAL_ROUND_INQUIRY` AND `injection_position == "post_tool_batch_gap"`
///   (设计 §3.2: 触发时机固定为首个含工具调用的动作批次结束);
/// - any OTHER trigger must NOT carry the initial-round block (the two
///   injected blocks are distinct; a mismatch means the producer wired the
///   wrong text to the wrong trigger);
/// - 会话内恰好一次: at most ONE initial-round fire per
///   (`session_id`, `agent_role`) inside one journal.
///
/// Not expressible in the payload schema (cross-field / cross-event), hence
/// a family-stage rule. Python twin: `_verify_v02_initial_round_inquiry`.
pub fn verify_initial_round_inquiry(events: &[Value]) -> Vec<String> {
    const INITIAL_TRIGGER: &str = "initial_round";
    const INITIAL_BLOCK_PREFIX: &str = "[INITIAL_ROUND_INQUIRY";
    const POST_TOOL_BATCH_GAP: &str = "post_tool_batch_gap";

    let mut errors = Vec::new();
    let mut fires: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("orientation_checkpoint")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let trigger = str_of(payload.get("trigger")).unwrap_or_default();
        let block = str_of(payload.get("message_block")).unwrap_or_default();
        let position = str_of(payload.get("injection_position")).unwrap_or_default();

        if trigger == INITIAL_TRIGGER {
            if !block.starts_with(INITIAL_BLOCK_PREFIX) {
                errors.push(format!(
                    "event {index}: orientation_checkpoint trigger \"initial_round\" \
                     must carry the [INITIAL_ROUND_INQUIRY block (got {block:?})"
                ));
            }
            if position != POST_TOOL_BATCH_GAP {
                errors.push(format!(
                    "event {index}: orientation_checkpoint trigger \"initial_round\" \
                     must be injected at {POST_TOOL_BATCH_GAP} (got {position:?})"
                ));
            }
            let session = str_of(payload.get("session_id"))
                .unwrap_or_default()
                .to_string();
            let role = str_of(payload.get("agent_role"))
                .unwrap_or_default()
                .to_string();
            *fires.entry((session, role)).or_insert(0) += 1;
        } else if block.starts_with(INITIAL_BLOCK_PREFIX) {
            errors.push(format!(
                "event {index}: orientation_checkpoint trigger {trigger:?} carries the \
                 initial-round block — trigger and message_block must agree"
            ));
        }
    }
    for ((session, role), count) in &fires {
        if *count > 1 {
            errors.push(format!(
                "session {session} / agent {role}: {count} initial_round inquiries — \
                 the initial-round inquiry is one-shot per session"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_plan_write` (PLAN-FIRST 阶段 C review closure): the
/// gate attempt state machine — refill only on attempt 1 followed by a
/// second write, validation_failed_after_refill only on attempt 2, accepted
/// requires valid=true and attempt 1|2, mechanical degrades keep
/// validation.valid=true while validation degrades carry false.
pub fn verify_plan_write(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let writes: Vec<(usize, &Value)> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            is_v02(event) && event.get("event_type").and_then(Value::as_str) == Some("plan_write")
        })
        .collect();
    for (position, (index, event)) in writes.iter().enumerate() {
        let payload = &event["payload"];
        let attempt = py_int_value(payload.get("attempt")).unwrap_or_default();
        let outcome = str_of(payload.get("outcome")).unwrap_or_default();
        let degrade_reason = str_of(payload.get("degrade_reason"));
        let valid = payload.get("validation").and_then(|v| v.get("valid"));
        if outcome == "refill_requested" {
            if attempt != 1 {
                errors.push(format!(
                    "event {index}: plan_write refill_requested on attempt {attempt} \
                     (only attempt 1 may request a refill)"
                ));
            }
            if position + 1 >= writes.len() {
                errors.push(format!(
                    "event {index}: plan_write refill_requested without a following \
                     refill attempt"
                ));
            }
            if valid != Some(&Value::Bool(false)) {
                errors.push(format!(
                    "event {index}: plan_write refill_requested but validation.valid \
                     is not false"
                ));
            }
        }
        if outcome == "degraded" {
            match degrade_reason {
                Some("validation_failed_after_refill") => {
                    if attempt != 2 {
                        errors.push(format!(
                            "event {index}: plan_write \
                             validation_failed_after_refill degrade on attempt \
                             {attempt} (degrade only after the refill attempt)"
                        ));
                    }
                    if valid != Some(&Value::Bool(false)) {
                        errors.push(format!(
                            "event {index}: plan_write \
                             validation_failed_after_refill but validation.valid is \
                             not false"
                        ));
                    }
                }
                Some(reason @ ("plan_rotate_failed" | "plan_not_submitted")) => {
                    if valid != Some(&Value::Bool(true)) {
                        errors.push(format!(
                            "event {index}: plan_write {reason} carries structural \
                             validation errors but the failure is mechanical \
                             (validation.valid must be true)"
                        ));
                    }
                }
                Some("validation_failed") if valid != Some(&Value::Bool(false)) => {
                    errors.push(format!(
                        "event {index}: plan_write revision validation_failed but \
                         validation.valid is not false"
                    ));
                }
                _ => {}
            }
        }
        if outcome == "accepted" {
            if valid != Some(&Value::Bool(true)) {
                errors.push(format!(
                    "event {index}: plan_write accepted but validation.valid is not true"
                ));
            }
            if attempt != 1 && attempt != 2 {
                errors.push(format!(
                    "event {index}: plan_write accepted on attempt {attempt} \
                     (expected 1 or 2)"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_console_mode_transition` (PLAN-FIRST 阶段 C): fixed
/// field shapes per direction, related_transition_id chaining, one streak
/// decision per run, and direct-mode tool events stamped with the current
/// direct transition id.
pub fn verify_console_mode_transition(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    let transitions: Vec<(usize, &Value)> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            is_v02(event)
                && event.get("event_type").and_then(Value::as_str)
                    == Some("console_mode_transition")
        })
        .collect();
    let mut per_run_streak_decisions: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    // run_id -> {transition_id -> index}
    let mut direct_transitions_by_run: std::collections::HashMap<
        String,
        std::collections::HashMap<String, usize>,
    > = std::collections::HashMap::new();
    let mut seen_transition_ids: std::collections::HashMap<String, BTreeSet<String>> =
        std::collections::HashMap::new();
    for (index, event) in &transitions {
        let payload = &event["payload"];
        let run_id = str_of(payload.get("run_id"))
            .unwrap_or_default()
            .to_string();
        let t_from = str_of(payload.get("from")).unwrap_or_default();
        let t_to = str_of(payload.get("to")).unwrap_or_default();
        let trigger = str_of(payload.get("trigger")).unwrap_or_default();
        let decision = str_of(payload.get("model_decision")).unwrap_or_default();
        let transition_id = str_of(payload.get("transition_id")).unwrap_or_default();
        let seen = seen_transition_ids.entry(run_id.clone()).or_default();
        if !seen.insert(transition_id.to_string()) {
            errors.push(format!(
                "event {index}: duplicate console_mode_transition transition_id \
                 {transition_id:?} in run {run_id}"
            ));
        }
        if t_from == "console" && t_to == "direct" {
            if trigger != "assistant_failure_streak" {
                errors.push(format!(
                    "event {index}: console→direct transition requires \
                     trigger=assistant_failure_streak, got {trigger:?}"
                ));
            }
            if decision != "switch" {
                errors.push(format!(
                    "event {index}: console→direct transition requires \
                     model_decision=switch, got {decision:?}"
                ));
            }
            let streak = py_int(payload.get("streak"));
            if streak.is_none_or(|s| s < 1) {
                errors.push(format!(
                    "event {index}: console→direct transition requires streak ≥ 1"
                ));
            }
            let order_ids_ok = payload
                .get("order_ids")
                .and_then(Value::as_array)
                .is_some_and(|list| !list.is_empty());
            if !order_ids_ok {
                errors.push(format!(
                    "event {index}: console→direct transition requires non-empty \
                     order_ids"
                ));
            }
            if py_none(payload.get("related_transition_id")).is_some() {
                errors.push(format!(
                    "event {index}: console→direct transition requires \
                     related_transition_id=null"
                ));
            }
            direct_transitions_by_run
                .entry(run_id.clone())
                .or_default()
                .insert(transition_id.to_string(), *index);
        } else if t_from == "direct" && t_to == "console" {
            if trigger != "model_return" {
                errors.push(format!(
                    "event {index}: direct→console transition requires \
                     trigger=model_return, got {trigger:?}"
                ));
            }
            if decision != "return_to_console" {
                errors.push(format!(
                    "event {index}: direct→console transition requires \
                     model_decision=return_to_console, got {decision:?}"
                ));
            }
            let related = py_none(payload.get("related_transition_id"));
            match str_of(related) {
                None => {
                    errors.push(format!(
                        "event {index}: direct→console transition requires \
                         related_transition_id"
                    ));
                }
                Some(related) => {
                    let known = direct_transitions_by_run
                        .get(&run_id)
                        .is_some_and(|ids| ids.contains_key(related));
                    if !known {
                        errors.push(format!(
                            "event {index}: direct→console related_transition_id \
                             {related:?} does not match an earlier console→direct \
                             transition of the same run"
                        ));
                    }
                }
            }
        } else if t_from == "console" && t_to == "console" {
            if trigger != "assistant_failure_streak" {
                errors.push(format!(
                    "event {index}: stay decision requires \
                     trigger=assistant_failure_streak, got {trigger:?}"
                ));
            }
            if decision != "stay" {
                errors.push(format!(
                    "event {index}: stay decision requires model_decision=stay, got \
                     {decision:?}"
                ));
            }
            if py_none(payload.get("related_transition_id")).is_some() {
                errors.push(format!(
                    "event {index}: stay decision requires related_transition_id=null"
                ));
            }
        } else {
            errors.push(format!(
                "event {index}: invalid console mode direction {t_from:?} → {t_to:?}"
            ));
        }
        if trigger == "assistant_failure_streak" {
            *per_run_streak_decisions.entry(run_id).or_default() += 1;
        }
    }
    for (run_id, count) in &per_run_streak_decisions {
        if *count > 1 {
            errors.push(format!(
                "run {run_id}: {count} assistant_failure_streak console mode \
                 decisions (at most one inquiry per run)"
            ));
        }
    }

    // Direct-mode tool events must carry the current direct transition id
    // (tool_running included — S5-2 review P2-3, 2026-08-29).
    let mut current_direct: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event.get("event_type").and_then(Value::as_str);
        if event_type == Some("console_mode_transition") {
            let payload = &event["payload"];
            let run_id = str_of(payload.get("run_id"))
                .unwrap_or_default()
                .to_string();
            if payload.get("from").and_then(Value::as_str) == Some("console")
                && payload.get("to").and_then(Value::as_str) == Some("direct")
            {
                current_direct.insert(
                    run_id,
                    str_of(payload.get("transition_id"))
                        .unwrap_or_default()
                        .to_string(),
                );
            } else if payload.get("from").and_then(Value::as_str) == Some("direct")
                && payload.get("to").and_then(Value::as_str) == Some("console")
            {
                current_direct.remove(&run_id);
            }
            continue;
        }
        if !matches!(
            event_type,
            Some("tool_started") | Some("tool_completed") | Some("tool_running")
        ) {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        if payload.get("console_mode").and_then(Value::as_str) == Some("direct") {
            let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
            let tid = str_of(payload.get("transition_id"));
            match current_direct.get(run_id) {
                None => {
                    errors.push(format!(
                        "event {index}: direct-mode tool event without a preceding \
                         console→direct transition in its run"
                    ));
                }
                Some(expected) => {
                    if tid != Some(expected.as_str()) {
                        errors.push(format!(
                            "event {index}: direct-mode tool event transition_id \
                             {tid:?} != current direct transition {expected:?}"
                        ));
                    }
                }
            }
        }
    }
    errors
}

/// Python `_verify_v02_console_order_rejected` (P0-E 第 4 项, B 族 — 收窄于
/// 2026-09-06 任务 D S2d 裁决一 / ADR-0010 §14.57: 写单链规则退役后收窄为
/// 形状不变量 — closed phase/step/code triples, non-empty reason, one
/// rejection per order per run; the prior-same-run written requirement and
/// the stamp-consistency friction sub-rules are retired with the §14.39
/// write-order chain, and `console_order_written` is retired outright
/// without a negative check — historical v0.2 journals legally carry
/// written chains).
pub fn verify_console_order_rejected(events: &[Value]) -> Vec<String> {
    const PRE_ISSUE_CODES: &[&str] = &["order_stale", "step_not_done", "budget_insufficient"];
    const ISSUE_STEPS: &[&str] = &["registry", "contract", "target", "policy"];
    let mut errors = Vec::new();
    let mut rejected_per_run: std::collections::HashMap<String, BTreeSet<String>> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("console_order_rejected")
        {
            continue;
        }
        let payload = &event["payload"];
        let run_id = event.get("run_id").and_then(Value::as_str).unwrap_or("");
        let order_id = str_of(payload.get("order_id")).unwrap_or_default();
        if !rejected_per_run
            .entry(run_id.to_string())
            .or_default()
            .insert(order_id.to_string())
        {
            errors.push(format!(
                "event {index}: duplicate console_order_rejected order_id \
                 {order_id:?} in run {run_id}"
            ));
        }
        let phase = str_of(payload.get("phase"));
        let step = str_of(payload.get("step"));
        let code = str_of(payload.get("code"));
        if !matches!(phase, Some("pre_issue") | Some("issue")) {
            errors.push(format!(
                "event {index}: console_order_rejected phase must be pre_issue or \
                 issue, got {phase:?}"
            ));
        }
        if phase == Some("pre_issue") {
            if step != Some("protocol") {
                errors.push(format!(
                    "event {index}: console_order_rejected pre_issue phase requires \
                     step=protocol, got {step:?}"
                ));
            }
            if !code.is_some_and(|c| PRE_ISSUE_CODES.contains(&c)) {
                errors.push(format!(
                    "event {index}: console_order_rejected pre_issue code must be \
                     order_stale/step_not_done/budget_insufficient, got {code:?}"
                ));
            }
        } else if phase == Some("issue") && !step.is_some_and(|s| ISSUE_STEPS.contains(&s)) {
            errors.push(format!(
                "event {index}: console_order_rejected issue phase step must be \
                 registry/contract/target/policy, got {step:?}"
            ));
        }
        let reason = str_of(payload.get("reason"));
        if reason.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: console_order_rejected reason must be a non-empty string"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_mechanical_audit` (MECHANICAL-AUDIT-LAYER): closed
/// kind vocabulary, full payload shape (key / non-negative round / summary /
/// nullable string anomaly).
pub fn verify_mechanical_audit(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("mechanical_audit_update")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let kind = str_of(payload.get("kind"));
        if !matches!(
            kind,
            Some("tool_result") | Some("plan_gate") | Some("budget")
        ) {
            errors.push(format!(
                "event {index}: mechanical_audit_update kind {:?} must be \
                 tool_result / plan_gate / budget",
                payload
                    .get("kind")
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            ));
        }
        let Some(entry) = py_none(payload.get("payload")).filter(|v| v.is_object()) else {
            errors.push(format!(
                "event {index}: mechanical_audit_update needs a payload object"
            ));
            continue;
        };
        let key = str_of(entry.get("key"));
        if key.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: mechanical_audit_update key must be a non-empty string"
            ));
        }
        let round = py_int(entry.get("round"));
        if round.is_none_or(|r| r < 0) {
            errors.push(format!(
                "event {index}: mechanical_audit_update round must be a non-negative \
                 integer"
            ));
        }
        let summary = str_of(entry.get("summary"));
        if summary.is_none_or(str::is_empty) {
            errors.push(format!(
                "event {index}: mechanical_audit_update summary must be a non-empty \
                 string"
            ));
        }
        if let Some(anomaly) = py_none(entry.get("anomaly"))
            && !anomaly.is_string()
        {
            errors.push(format!(
                "event {index}: mechanical_audit_update anomaly must be a string or null"
            ));
        }
    }
    errors
}

/// Python `_verify_v02_tool_availability_probe` (FUS-TOOL-PROBE; partition
/// clause narrowed 2026-09-06, ADR-0010 §14.59 — the complete/incomplete
/// sets must be disjoint and cover work tools only, a subset rather than
/// the exact full set since the producer accounts the declared surface,
/// §14.58), and incomplete reasons stay neutral (no availability judgment
/// words).
pub fn verify_tool_availability_probe(events: &[Value]) -> Vec<String> {
    const JUDGMENT_WORD_TOKENS: &[&str] = &[
        "可用",
        "不可用",
        "成功",
        "失败",
        "available",
        "unavailable",
        "success",
        "failure",
    ];
    let mut errors = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("tool_availability_check")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let complete: BTreeSet<String> = payload
            .get("complete")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect();
        let incomplete: BTreeMap<String, String> = payload
            .get("incomplete")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| {
                let tool = str_of(item.get("tool"))?.to_string();
                let reason = str_of(item.get("reason")).unwrap_or("").to_string();
                Some((tool, reason))
            })
            .collect();
        let incomplete_tools: Vec<&String> = complete
            .iter()
            .filter(|tool| incomplete.contains_key(*tool))
            .collect();
        if !incomplete_tools.is_empty() {
            errors.push(format!(
                "event {index}: tool(s) in both complete and incomplete: \
                 {incomplete_tools:?}"
            ));
        }
        let mut union = complete.clone();
        union.extend(incomplete.keys().cloned());
        let work: BTreeSet<String> = toolsets::WORK_TOOLS.iter().map(|s| s.to_string()).collect();
        // Partition clause narrowed 2026-09-06 (ADR-0010 §14.59, 任务 D S2d
        // 裁决二补裁决): the producer's availability accounting covers the
        // declared surface only (§14.58), so the partition must be a subset
        // of the work tools rather than the exact full set — historical
        // full-partition journals keep replaying clean as a subset.
        let extra: Vec<&String> = union.difference(&work).collect();
        if !extra.is_empty() {
            errors.push(format!(
                "event {index}: probe partition must be a subset of the work tools; \
                 extra={extra:?}"
            ));
        }
        for (tool, reason) in &incomplete {
            let lowered = reason.to_lowercase();
            if JUDGMENT_WORD_TOKENS
                .iter()
                .any(|token| lowered.contains(token))
            {
                errors.push(format!(
                    "event {index}: incomplete reason for {tool:?} uses an \
                     availability judgment word: {reason:?}"
                ));
            }
        }
    }
    errors
}

/// Python `_verify_v02_request_header` (ORZ-CACHE-CONTEXT-COST): per-lane
/// header chains — initial opens (no previous/change_kind), change follows
/// the lane's last header with matching previous, differs, and change_kind
/// names exactly the components whose digests changed; tools unique with
/// matching tool_count.
pub fn verify_request_header(events: &[Value]) -> Vec<String> {
    fn header_change_kind(
        prev_system: Option<&Value>,
        prev_tools: Option<&Value>,
        prev_config: Option<&Value>,
        system: Option<&Value>,
        tools: Option<&Value>,
        config: Option<&Value>,
    ) -> &'static str {
        let mut changed = 0usize;
        let mut only: &str = "";
        for (name, prev, cur) in [
            ("system", prev_system, system),
            ("tools", prev_tools, tools),
            ("config", prev_config, config),
        ] {
            if !py_value_eq(prev, cur) {
                changed += 1;
                only = name;
            }
        }
        if changed == 1 { only } else { "multiple" }
    }

    let mut errors = Vec::new();
    // role -> (index, header, system_sha, tools_sha, config_sha)
    let mut last_by_role: std::collections::HashMap<String, (usize, Value, Value, Value, Value)> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event)
            || event.get("event_type").and_then(Value::as_str) != Some("request_header_change")
        {
            continue;
        }
        let payload = event.get("payload").cloned().unwrap_or(Value::Null);
        let role = str_of(payload.get("agent_role")).unwrap_or("").to_string();
        let reason = str_of(payload.get("reason"));
        let header = py_none(payload.get("header_sha256"));
        let previous = py_none(payload.get("previous_header_sha256"));
        let change_kind = py_none(payload.get("change_kind"));
        let tools = payload
            .get("tools")
            .cloned()
            .unwrap_or(Value::Array(vec![]));
        let raw_len = tools.as_array().map(|l| l.len()).unwrap_or(0);
        let tool_strs: Vec<&str> = tools
            .as_array()
            .map(|l| l.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let mut unique: BTreeSet<&str> = BTreeSet::new();
        unique.extend(tool_strs.iter().copied());
        if unique.len() != tool_strs.len() {
            errors.push(format!(
                "event {index}: request_header_change tools must be unique"
            ));
        }
        if py_int_value(payload.get("tool_count")) != Some(raw_len as i64) {
            errors.push(format!(
                "event {index}: request_header_change tool_count {:?} != tools \
                 length {raw_len}",
                payload.get("tool_count")
            ));
        }
        let last = last_by_role.get(&role);
        match reason {
            Some("initial") => {
                if previous.is_some() {
                    errors.push(format!(
                        "event {index}: initial request_header_change must not carry \
                         previous_header_sha256"
                    ));
                }
                if change_kind.is_some() {
                    errors.push(format!(
                        "event {index}: initial request_header_change must not carry \
                         change_kind"
                    ));
                }
            }
            Some("change") => {
                // Clone the lane's last component digests up front — the map is
                // re-borrowed mutably by the unconditional insert below.
                let last = last.map(|(_, h, s, t, c)| (h.clone(), s.clone(), t.clone(), c.clone()));
                let Some((last_header, last_system, last_tools, last_config)) = last else {
                    errors.push(format!(
                        "event {index}: change request_header_change without a prior \
                         initial for role {role:?}"
                    ));
                    last_by_role.insert(
                        role,
                        (
                            index,
                            header.cloned().unwrap_or(Value::Null),
                            payload.get("system_sha256").cloned().unwrap_or(Value::Null),
                            payload.get("tools_sha256").cloned().unwrap_or(Value::Null),
                            payload.get("config_sha256").cloned().unwrap_or(Value::Null),
                        ),
                    );
                    continue;
                };
                if !py_value_eq(previous, py_none(Some(&last_header))) {
                    errors.push(format!(
                        "event {index}: change previous_header_sha256 {previous:?} != \
                         last header {last_header} for role {role:?}"
                    ));
                }
                if py_value_eq(previous, header) {
                    errors.push(format!(
                        "event {index}: change request_header_change must differ from \
                         the previous header"
                    ));
                }
                match str_of(change_kind) {
                    None => {
                        errors.push(format!(
                            "event {index}: change request_header_change must carry \
                             change_kind"
                        ));
                    }
                    Some(kind) => {
                        let expected = header_change_kind(
                            py_none(Some(&last_system)),
                            py_none(Some(&last_tools)),
                            py_none(Some(&last_config)),
                            py_none(payload.get("system_sha256")),
                            py_none(payload.get("tools_sha256")),
                            py_none(payload.get("config_sha256")),
                        );
                        if kind != expected {
                            errors.push(format!(
                                "event {index}: change_kind {kind:?} != actual changed \
                                 components {expected:?}"
                            ));
                        }
                    }
                }
            }
            other => {
                errors.push(format!(
                    "event {index}: request_header_change reason {other:?} not in \
                     initial/change"
                ));
            }
        }
        last_by_role.insert(
            role,
            (
                index,
                header.cloned().unwrap_or(Value::Null),
                payload.get("system_sha256").cloned().unwrap_or(Value::Null),
                payload.get("tools_sha256").cloned().unwrap_or(Value::Null),
                payload.get("config_sha256").cloned().unwrap_or(Value::Null),
            ),
        );
    }
    errors
}

/// Python `_verify_v02_probe_accuracy` (ORZ-CACHE-CONTEXT-COST): a complete-
/// set flip must be followed by a main-lane header `change` before the next
/// model_output; the check applies only to journals that carry
/// request_header_change events (2026-08-15 compatibility boundary).
pub fn verify_probe_accuracy(events: &[Value]) -> Vec<String> {
    let mut errors = Vec::new();
    if !events.iter().any(|event| {
        is_v02(event)
            && event.get("event_type").and_then(Value::as_str) == Some("request_header_change")
    }) {
        return errors;
    }
    let mut prev_complete: Option<BTreeSet<String>> = None;
    let mut pending_flip: Option<(usize, Vec<String>, Vec<String>)> = None;
    for (index, event) in events.iter().enumerate() {
        if !is_v02(event) {
            continue;
        }
        let event_type = event.get("event_type").and_then(Value::as_str);
        if event_type == Some("tool_availability_check") {
            let complete: BTreeSet<String> = event["payload"]
                .get("complete")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect();
            if let Some(prev) = &prev_complete
                && &complete != prev
            {
                pending_flip = Some((
                    index,
                    prev.iter().cloned().collect(),
                    complete.iter().cloned().collect(),
                ));
            }
            prev_complete = Some(complete);
            continue;
        }
        if event_type == Some("request_header_change")
            && event["payload"].get("agent_role").and_then(Value::as_str) == Some("main")
        {
            // A main-lane `initial` deliberately does NOT clear a pending flip
            // (2026-08-15 review note — currently unreachable).
            if pending_flip.is_some()
                && event["payload"].get("reason").and_then(Value::as_str) == Some("change")
            {
                pending_flip = None;
            }
            continue;
        }
        if event_type == Some("model_output")
            && let Some((flip_index, before, after)) = pending_flip.take()
        {
            errors.push(format!(
                "event {index}: tool_availability_check flip at event \
                 {flip_index} (complete {before:?} -> {after:?}) was not followed \
                 by a request_header_change(reason=change) before the next \
                 model_output"
            ));
        }
    }
    errors
}

// ── dispatch ────────────────────────────────────────────────────────────

/// Run one named S2c family over a parsed journal; unknown names yield an
/// empty list (the caller validates the name).
pub fn verify_s2c_family(family: &str, events: &[Value]) -> Vec<String> {
    match family {
        "inquiry_kind" => verify_inquiry_kind(events),
        "initial_round_inquiry" => verify_initial_round_inquiry(events),
        "plan_write" => verify_plan_write(events),
        "console_mode_transition" => verify_console_mode_transition(events),
        // "console_order_written" retired 2026-09-06 (任务 D S2d 裁决一,
        // ADR-0010 §14.57): no Rust rule either, and no negative check —
        // historical v0.2 journals legally carry written chains.
        "console_order_rejected" => verify_console_order_rejected(events),
        "tool_running" => verify_tool_running(events),
        "output_truncation" => verify_output_truncation(events),
        "budget_cue_injected" => verify_budget_cue_injected(events),
        "result_consistency" => verify_result_consistency(events),
        "reason_codes" => verify_reason_codes(events),
        "source_weighting" => verify_source_weighting(events),
        "search_candidate_pool" => verify_search_candidate_pool(events),
        "candidate_prefilter" => verify_candidate_prefilter(events),
        "candidate_count" => verify_candidate_count(events),
        "inject_budget" => verify_inject_budget(events),
        "receipt_event_isomorphism" => verify_receipt_event_isomorphism(events),
        "dep_graph_events" => verify_dep_graph_events(events),
        "mechanical_audit" => verify_mechanical_audit(events),
        "recovery_truncation" => verify_recovery_truncation(events),
        "context_compressed" => verify_context_compressed(events),
        "activation_restore" => verify_activation_restore(events),
        "tool_availability_probe" => verify_tool_availability_probe(events),
        "request_header" => verify_request_header(events),
        "probe_accuracy" => verify_probe_accuracy(events),
        _ => Vec::new(),
    }
}
