//! FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): output-level citation
//! verifier (ADR-0010 §3.7.9 / V11-IMPL-004 / RETRIEVAL_MECHANICAL_CONTROLS
//! _DESIGN §3.2).
//!
//! Scans the main agent's final answer for `[来源: ...]` markers, parses them
//! structurally (never grep), binds each marker to THIS run's mechanical
//! evidence — committed retrieval ledgers (`source_id`, URL/document identity)
//! and main-lane read evidence (`path:line`) — and enforces the §3.7.5
//! claim × visibility cap where the marker carries an explicit strength/scope
//! tag. A bare ledger-backed marker is a writer-side binding whose cap is the
//! ledger entry's `highest_allowed_claim` (the marker itself asserts no
//! stronger strength); a bare external-URL marker is a contract violation
//! (the prompt requires URL + observed scope). Any failure degrades/blocks the
//! delivery with an explicit mechanical reason code; a passing final answer
//! journals nothing.

use serde_json::Value;

use crate::controller::EvidenceRecord;

/// The inline citation marker prefix (D-1 / ADR-0010 §3.7.9).
pub const CITATION_MARKER_PREFIX: &str = "[来源:";

/// Review fix (2026-08-14): the full-width colon variant `[来源：...]` is
/// also parsed — a model "citation" that merely switches the colon must not
/// silently escape the verifier (the canonical prompt form stays
/// `[来源: ...]`).
pub const CITATION_MARKER_PREFIX_FULLWIDTH: &str = "[来源：";

// Stable mechanical reason codes — the run-event schema pins the pattern
// `^[a-z0-9_]+$`.
pub const REASON_MARKER_UNPARSABLE: &str = "marker_unparsable";
pub const REASON_EMPTY_MARKER: &str = "empty_marker";
pub const REASON_UNKNOWN_SOURCE_ID: &str = "unknown_source_id";
pub const REASON_SOURCE_UNAVAILABLE: &str = "source_unavailable";
pub const REASON_CLAIM_EXCEEDS_VISIBILITY: &str = "claim_exceeds_visibility";
pub const REASON_URL_NOT_IN_EVIDENCE: &str = "url_not_in_evidence";
pub const REASON_URL_MISSING_OBSERVED_SCOPE: &str = "url_missing_observed_scope";
pub const REASON_PATH_NOT_OBSERVED: &str = "path_not_observed";
pub const REASON_PATH_NOT_FOUND: &str = "path_not_found";
pub const REASON_PATH_LINE_OUT_OF_BOUNDS: &str = "path_line_out_of_bounds";
pub const REASON_UNRESOLVABLE_CITATION: &str = "unresolvable_citation";

/// How a marker binds to evidence (schema `binding` enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    LedgerSourceId,
    PathLine,
    UrlIdentity,
    DocumentIdentity,
    Unresolved,
}

impl BindingKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BindingKind::LedgerSourceId => "ledger_source_id",
            BindingKind::PathLine => "path_line",
            BindingKind::UrlIdentity => "url_identity",
            BindingKind::DocumentIdentity => "document_identity",
            BindingKind::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerStatus {
    Passed,
    Failed,
}

/// One parsed `[来源: ...]` marker and its mechanical verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationMarker {
    pub index: usize,
    pub raw: String,
    pub target: String,
    pub binding: BindingKind,
    pub status: MarkerStatus,
    pub reason_codes: Vec<String>,
}

/// The final-answer verdict — `passed == reason_codes.is_empty()`.
#[derive(Debug, Clone, Default)]
pub struct CitationValidationReport {
    pub markers: Vec<CitationMarker>,
    pub reason_codes: Vec<String>,
    pub passed: bool,
}

/// Parse every `[来源: ...]` marker in `text` structurally — the marker
/// prefix followed by the next `]`; the content is never regex/grep-scanned
/// from surrounding prose. An unclosed marker is itself a failed marker
/// (`marker_unparsable`) and stops the scan.
///
/// Review fix (2026-08-14): markers inside fenced code blocks (``` / ~~~)
/// or inline backtick spans are NOT citations — the code mask is computed
/// first and the scanner skips those byte ranges (a literal format example
/// must not block delivery). The full-width colon variant is parsed too.
pub fn parse_markers(text: &str) -> Vec<CitationMarker> {
    let mut markers = Vec::new();
    let mask = code_mask(text);
    let mut search_from = 0usize;
    let mut index = 0usize;
    while search_from < text.len() {
        if let Some(code) = mask.iter().find(|(s, e)| *s <= search_from && search_from < *e) {
            search_from = code.1;
            continue;
        }
        let rel_a = text[search_from..].find(CITATION_MARKER_PREFIX);
        let rel_b = text[search_from..].find(CITATION_MARKER_PREFIX_FULLWIDTH);
        let (rel, prefix_len) = match (rel_a, rel_b) {
            (Some(a), Some(b)) if b < a => (Some(b), CITATION_MARKER_PREFIX_FULLWIDTH.len()),
            (Some(a), _) => (Some(a), CITATION_MARKER_PREFIX.len()),
            (None, Some(b)) => (Some(b), CITATION_MARKER_PREFIX_FULLWIDTH.len()),
            (None, None) => (None, 0),
        };
        let Some(rel) = rel else { break };
        let start = search_from + rel;
        // A marker whose start sits inside a code range (the range may open
        // after `search_from`) is skipped wholesale.
        if let Some(code) = mask.iter().find(|(s, e)| *s <= start && start < *e) {
            search_from = code.1;
            continue;
        }
        let content_start = start + prefix_len;
        let Some(close_rel) = text[content_start..].find(']') else {
            let raw = text[start..].trim().to_string();
            markers.push(CitationMarker {
                index,
                raw,
                target: String::new(),
                binding: BindingKind::Unresolved,
                status: MarkerStatus::Failed,
                reason_codes: vec![REASON_MARKER_UNPARSABLE.to_string()],
            });
            break;
        };
        let content = text[content_start..content_start + close_rel].trim();
        let raw = text[start..=content_start + close_rel].to_string();
        let target = content.to_string();
        markers.push(CitationMarker {
            index,
            raw,
            target,
            binding: BindingKind::Unresolved,
            status: MarkerStatus::Failed,
            reason_codes: vec![REASON_UNRESOLVABLE_CITATION.to_string()],
        });
        search_from = content_start + close_rel + 1;
        index += 1;
    }
    markers
}

/// Byte ranges of `text` that are inside fenced code blocks (``` / ~~~) or
/// inline backtick spans — literal code, never citation sites.
fn code_mask(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut in_fence = false;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let line_end = offset + line.len();
        let trimmed = line.trim_start();
        let is_fence_line = ["```", "~~~"]
            .iter()
            .any(|d| trimmed.starts_with(*d));
        if is_fence_line {
            ranges.push((offset, line_end));
            in_fence = !in_fence;
        } else if in_fence {
            ranges.push((offset, line_end));
        } else {
            let mut in_code = false;
            let mut span_start = 0usize;
            for (i, ch) in line.char_indices() {
                if ch == '`' {
                    if in_code {
                        ranges.push((offset + span_start, offset + i));
                        in_code = false;
                    } else {
                        span_start = i;
                        in_code = true;
                    }
                }
            }
            if in_code {
                ranges.push((offset + span_start, offset + line.len()));
            }
        }
        offset = line_end;
    }
    ranges
}

/// The marker's binding target: the full content minus a trailing
/// strength/scope tag (`observed|derived|synthesized` or a §3.7.5 visibility
/// token). Returns `(target, tag)`.
fn marker_target(content: &str) -> (String, Option<&str>) {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return (String::new(), None);
    }
    let mut tokens = trimmed.split_whitespace();
    let first = tokens.next().unwrap_or_default();
    let rest: Vec<&str> = tokens.collect();
    if rest.is_empty() {
        return (first.to_string(), None);
    }
    let last = rest[rest.len() - 1];
    if is_strength_or_visibility_tag(last) {
        let mut target_parts = vec![first];
        target_parts.extend_from_slice(&rest[..rest.len() - 1]);
        (target_parts.join(" "), Some(last))
    } else {
        (trimmed.to_string(), None)
    }
}

fn is_strength_or_visibility_tag(tag: &str) -> bool {
    matches!(
        tag,
        "observed"
            | "derived"
            | "synthesized"
            | "full_text_observed"
            | "partial_text_observed"
            | "metadata_only"
    )
}

/// The §3.7.5 claim rank — mirrored from the controller's structured-result
/// validator (higher = stronger).
pub fn claim_rank(strength: &str) -> u8 {
    match strength {
        "observed" => 3,
        "derived" => 2,
        "synthesized" => 1,
        _ => 0,
    }
}

/// The §3.7.5 visibility rank — mirrored from the controller's structured-result
/// validator (higher = more observed).
pub fn visibility_rank(visibility: &str) -> u8 {
    match visibility {
        "full_text_observed" => 3,
        "partial_text_observed" => 2,
        "metadata_only" => 1,
        _ => 0,
    }
}

/// Validate the final answer against this run's evidence.
///
/// `ledgers` is the list of `source_ledger` arrays committed by retrieval
/// subagents in THIS run; `main_evidence` is the main lane's own tool-call
/// evidence (read_file etc. — the observation-time source for `path:line`).
pub fn validate_final_answer(
    text: &str,
    ledgers: &[Value],
    main_evidence: &[EvidenceRecord],
) -> CitationValidationReport {
    let mut markers = parse_markers(text);
    let mut reasons: Vec<String> = Vec::new();
    for marker in &mut markers {
        if marker.status == MarkerStatus::Failed
            && marker.reason_codes.as_slice() == [REASON_MARKER_UNPARSABLE]
        {
            push_unique(&mut reasons, REASON_MARKER_UNPARSABLE);
            continue;
        }
        if marker.target.is_empty() {
            marker.binding = BindingKind::Unresolved;
            marker.status = MarkerStatus::Failed;
            marker.reason_codes = vec![REASON_EMPTY_MARKER.to_string()];
            push_unique(&mut reasons, REASON_EMPTY_MARKER);
            continue;
        }
        let verdict = validate_target(&marker.target, ledgers, main_evidence);
        marker.binding = verdict.binding;
        marker.status = if verdict.reason_codes.is_empty() {
            MarkerStatus::Passed
        } else {
            MarkerStatus::Failed
        };
        marker.reason_codes = verdict.reason_codes;
        for code in &marker.reason_codes {
            push_unique(&mut reasons, code);
        }
    }
    let passed = markers.iter().all(|m| m.status == MarkerStatus::Passed);
    CitationValidationReport {
        markers,
        reason_codes: reasons,
        passed,
    }
}

struct TargetVerdict {
    binding: BindingKind,
    reason_codes: Vec<String>,
}

fn validate_target(
    target: &str,
    ledgers: &[Value],
    main_evidence: &[EvidenceRecord],
) -> TargetVerdict {
    let (bound_target, tag) = marker_target(target);
    // Ledger source id.
    if bound_target.starts_with("SRC-") {
        return validate_ledger_source_id(&bound_target, tag, ledgers);
    }
    // External URL — URL/document identity + observed scope required.
    if bound_target.to_ascii_lowercase().starts_with("http://")
        || bound_target.to_ascii_lowercase().starts_with("https://")
    {
        return validate_url(&bound_target, tag, ledgers, main_evidence);
    }
    // Local observation-time path:line.
    if let Some((path, line)) = split_path_line(&bound_target) {
        return validate_path_line(path, line, main_evidence);
    }
    // Internal document identity (文档ID §节/锚点 or ledger project_doc path).
    let (doc_id, _anchor) = split_document_identity(&bound_target);
    if let Some(entry) = find_document_entry(doc_id, ledgers) {
        return TargetVerdict {
            binding: BindingKind::DocumentIdentity,
            // Review fix (2026-08-14): the § anchor is a locator, NOT a
            // strength/scope tag — only an explicit trailing tag from
            // `marker_target` participates in the claim × visibility cap.
            reason_codes: check_claim_tag(tag, entry),
        };
    }
    TargetVerdict {
        binding: BindingKind::Unresolved,
        reason_codes: vec![REASON_UNRESOLVABLE_CITATION.to_string()],
    }
}

fn validate_ledger_source_id(target: &str, tag: Option<&str>, ledgers: &[Value]) -> TargetVerdict {
    let Some(entry) = find_ledger_entry(target, ledgers) else {
        return TargetVerdict {
            binding: BindingKind::LedgerSourceId,
            reason_codes: vec![REASON_UNKNOWN_SOURCE_ID.to_string()],
        };
    };
    let mut reasons = Vec::new();
    if entry_visibility(entry) == "unavailable" {
        reasons.push(REASON_SOURCE_UNAVAILABLE.to_string());
    }
    reasons.extend(check_claim_tag(tag, entry));
    TargetVerdict {
        binding: BindingKind::LedgerSourceId,
        reason_codes: reasons,
    }
}

fn validate_url(
    target: &str,
    tag: Option<&str>,
    ledgers: &[Value],
    main_evidence: &[EvidenceRecord],
) -> TargetVerdict {
    let Some(entry) = find_url_entry(target, ledgers, main_evidence) else {
        return TargetVerdict {
            binding: BindingKind::UrlIdentity,
            reason_codes: vec![REASON_URL_NOT_IN_EVIDENCE.to_string()],
        };
    };
    // The prompt contract: external sources cite URL/document identity +
    // observed scope — a bare URL asserts nothing about scope and is rejected.
    let mut reasons = Vec::new();
    if tag.is_none() {
        reasons.push(REASON_URL_MISSING_OBSERVED_SCOPE.to_string());
    }
    if entry_visibility(&entry) == "unavailable" {
        reasons.push(REASON_SOURCE_UNAVAILABLE.to_string());
    }
    reasons.extend(check_claim_tag(tag, &entry));
    TargetVerdict {
        binding: BindingKind::UrlIdentity,
        reason_codes: reasons,
    }
}

fn validate_path_line(path: &str, line: u64, main_evidence: &[EvidenceRecord]) -> TargetVerdict {
    let normalized = normalize_path(path);
    let observed = main_evidence.iter().any(|ev| {
        let identity = normalize_path(&ev.identity);
        identity == normalized
            || identity.ends_with(&format!("/{normalized}"))
            || (normalized.starts_with('/') && identity.ends_with(normalized.as_str()))
    });
    if !observed {
        return TargetVerdict {
            binding: BindingKind::PathLine,
            reason_codes: vec![REASON_PATH_NOT_OBSERVED.to_string()],
        };
    }
    match file_line_count(path) {
        Ok(Some(count)) if count >= line => TargetVerdict {
            binding: BindingKind::PathLine,
            reason_codes: Vec::new(),
        },
        Ok(Some(_)) => TargetVerdict {
            binding: BindingKind::PathLine,
            reason_codes: vec![REASON_PATH_LINE_OUT_OF_BOUNDS.to_string()],
        },
        Ok(None) => TargetVerdict {
            binding: BindingKind::PathLine,
            reason_codes: Vec::new(),
        },
        Err(_) => TargetVerdict {
            binding: BindingKind::PathLine,
            reason_codes: vec![REASON_PATH_NOT_FOUND.to_string()],
        },
    }
}

/// Count lines of `path`, bounded to 32 MiB / 1M lines; `Ok(None)` when the
/// file is too large to bound-check (documented v0 boundary — identity is
/// still evidence-bound).
fn file_line_count(path: &str) -> std::io::Result<Option<u64>> {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Ok(None);
    }
    let mut count: u64 = 0;
    for line in BufReader::new(file).lines() {
        line?;
        count += 1;
        if count > 1_000_000 {
            return Ok(None);
        }
    }
    Ok(Some(count))
}

/// Split a `path:line` target at the LAST colon-digit boundary — handles
/// both `src/main.rs:12` and Windows `C:\x\y.rs:12`.
fn split_path_line(target: &str) -> Option<(&str, u64)> {
    let (path, line_str) = target.rsplit_once(':')?;
    if path.is_empty() || line_str.is_empty() || !line_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let line: u64 = line_str.parse().ok()?;
    if line == 0 {
        return None;
    }
    Some((path, line))
}

/// Split a document identity at the `§` anchor boundary; the anchor is a
/// scope hint, not part of the ledger identity.
fn split_document_identity(target: &str) -> (&str, Option<String>) {
    match target.split_once('§') {
        Some((doc, anchor)) => {
            let anchor = anchor.trim();
            let tag = if is_strength_or_visibility_tag(anchor) {
                None
            } else {
                Some(anchor.to_string())
            };
            (doc.trim(), tag)
        }
        None => (target.trim(), None),
    }
}

fn find_ledger_entry<'a>(source_id: &str, ledgers: &'a [Value]) -> Option<&'a Value> {
    ledgers.iter().find_map(|ledger| {
        ledger.as_array().and_then(|entries| {
            entries
                .iter()
                .find(|e| e["source_id"].as_str() == Some(source_id))
        })
    })
}

fn find_document_entry<'a>(doc_id: &str, ledgers: &'a [Value]) -> Option<&'a Value> {
    let normalized = normalize_path(doc_id);
    ledgers.iter().find_map(|ledger| {
        ledger.as_array().and_then(|entries| {
            entries.iter().find(|e| {
                let identity = e["source_url_or_ref"].as_str().unwrap_or_default();
                let source_type = e["source_type"].as_str().unwrap_or_default();
                (source_type == "project_doc" || source_type == "local_file")
                    && (normalize_path(identity) == normalized
                        || normalize_path(identity).ends_with(&format!("/{normalized}")))
            })
        })
    })
}

fn find_url_entry<'a>(
    url: &str,
    ledgers: &'a [Value],
    main_evidence: &'a [EvidenceRecord],
) -> Option<Value> {
    let normalized = normalize_url(url);
    for ledger in ledgers {
        if let Some(entries) = ledger.as_array() {
            for entry in entries {
                let identity = entry["source_url_or_ref"].as_str().unwrap_or_default();
                if normalize_url(identity) == normalized {
                    return Some(entry.clone());
                }
                if let Some(candidates) = entry["candidate_urls"].as_array() {
                    if candidates
                        .iter()
                        .any(|c| normalize_url(c.as_str().unwrap_or_default()) == normalized)
                    {
                        return Some(entry.clone());
                    }
                }
            }
        }
    }
    // Review fix (2026-08-14): the main lane's OWN web evidence (web_fetch /
    // browser_read / pdf_read URL identities) is also this run's evidence —
    // a URL marker may bind to it (ledgers remain authoritative for
    // retrieval-lane sources; main-evidence entries carry the same
    // visibility → highest_allowed_claim projection as the ledger builder).
    for ev in main_evidence {
        if normalize_url(&ev.identity) == normalized {
            return Some(serde_json::json!({
                "visibility": ev.visibility,
                "highest_allowed_claim": match ev.visibility.as_str() {
                    "full_text_observed" => "observed",
                    "partial_text_observed" => "derived",
                    "metadata_only" => "synthesized",
                    _ => "none",
                },
            }));
        }
    }
    None
}

fn entry_visibility(entry: &Value) -> &str {
    entry["visibility"].as_str().unwrap_or_default()
}

fn check_claim_tag(tag: Option<&str>, entry: &Value) -> Vec<String> {
    let Some(tag) = tag else {
        return Vec::new();
    };
    let allowed = entry
        .get("highest_allowed_claim")
        .and_then(Value::as_str)
        .unwrap_or("none");
    let visibility = entry_visibility(entry);
    if claim_rank(tag) > claim_rank(allowed) || visibility_rank(tag) > visibility_rank(visibility) {
        vec![REASON_CLAIM_EXCEEDS_VISIBILITY.to_string()]
    } else {
        Vec::new()
    }
}

fn push_unique(codes: &mut Vec<String>, code: &str) {
    if !codes.iter().any(|c| c == code) {
        codes.push(code.to_string());
    }
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim().trim_matches('"').trim_matches('`');
    let replaced = trimmed.replace('\\', "/");
    let stripped = replaced.strip_prefix("./").unwrap_or(&replaced);
    if cfg!(windows) {
        stripped.to_lowercase()
    } else {
        stripped.to_string()
    }
}

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim().trim_matches('"').trim_matches('`');
    // Review fix (2026-08-14): reuse the shared network-target
    // canonicalisation (scheme/host lowercase, default ports removed,
    // fragment dropped, userinfo rejected — orz-assurance `acaf::target`).
    // Malformed / non-http inputs fall back to the lenient v0 matcher.
    if let Ok(canonical) = orz_assurance::acaf::target::resolve_network_url(trimmed) {
        return canonical;
    }
    trimmed.trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn evidence(path: &str) -> EvidenceRecord {
        EvidenceRecord {
            tool: "read_file".to_string(),
            identity: path.to_string(),
            title: path.rsplit(['/', '\\']).next().unwrap_or(path).to_string(),
            source_type: "local_file".to_string(),
            visibility: "full_text_observed".to_string(),
            content_sha256: None,
            observed_scope: "file content".to_string(),
            missing_scope: "none".to_string(),
            candidate_urls: Vec::new(),
            search_query: None,
            accessed_at: "2026-08-14T00:00:00Z".to_string(),
        }
    }

    fn ledger_entry(source_id: &str, url: &str, visibility: &str) -> Value {
        json!({
            "source_id": source_id,
            "source_title": "t",
            "source_url_or_ref": url,
            "source_type": "web_page",
            "visibility": visibility,
            "observed_scope": "full document",
            "missing_scope": "none",
            "highest_allowed_claim": match visibility {
                "full_text_observed" => "observed",
                "partial_text_observed" => "derived",
                "metadata_only" => "synthesized",
                _ => "none",
            },
        })
    }

    #[test]
    fn parse_finds_and_binds_nothing_before_validation() {
        let markers = parse_markers("结论[来源: SRC-001] 结束");
        assert_eq!(markers.len(), 1);
        assert_eq!(markers[0].raw, "[来源: SRC-001]");
        assert_eq!(markers[0].target, "SRC-001");
    }

    #[test]
    fn unclosed_marker_is_unparsable() {
        let report = validate_final_answer("内容[来源: SRC-001", &[], &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_MARKER_UNPARSABLE.to_string())
        );
    }

    #[test]
    fn unknown_source_id_fails() {
        let report = validate_final_answer("[来源: SRC-999]", &[], &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_UNKNOWN_SOURCE_ID.to_string())
        );
        assert_eq!(report.markers[0].binding, BindingKind::LedgerSourceId);
    }

    #[test]
    fn known_full_text_source_passes_bare_marker() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "full_text_observed"
        )])];
        let report = validate_final_answer("[来源: SRC-001]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
    }

    #[test]
    fn tag_exceeding_visibility_fails() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "partial_text_observed"
        )])];
        let report = validate_final_answer("[来源: SRC-001 observed]", &ledgers, &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_CLAIM_EXCEEDS_VISIBILITY.to_string())
        );
    }

    #[test]
    fn tag_within_visibility_passes() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "partial_text_observed"
        )])];
        let report = validate_final_answer("[来源: SRC-001 derived]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
    }

    #[test]
    fn unavailable_source_fails() {
        let ledgers = vec![json!([ledger_entry("SRC-001", "https://x", "unavailable")])];
        let report = validate_final_answer("[来源: SRC-001]", &ledgers, &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_SOURCE_UNAVAILABLE.to_string())
        );
    }

    #[test]
    fn bare_url_requires_observed_scope() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "metadata_only"
        )])];
        let report = validate_final_answer("[来源: https://x]", &ledgers, &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_URL_MISSING_OBSERVED_SCOPE.to_string())
        );
    }

    #[test]
    fn url_with_scope_passes() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "metadata_only"
        )])];
        let report = validate_final_answer("[来源: https://x metadata_only]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
    }

    #[test]
    fn url_with_strong_scope_fails() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "metadata_only"
        )])];
        let report = validate_final_answer("[来源: https://x full_text_observed]", &ledgers, &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_CLAIM_EXCEEDS_VISIBILITY.to_string())
        );
    }

    #[test]
    fn path_line_not_observed_fails() {
        let report = validate_final_answer("[来源: src/main.rs:12]", &[], &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_PATH_NOT_OBSERVED.to_string())
        );
    }

    #[test]
    fn path_line_observed_and_in_bounds_passes() {
        let evs = vec![evidence("src/lib.rs")];
        // The test file is this source file itself; line 1 always exists.
        let report = validate_final_answer("[来源: src/lib.rs:1]", &[], &evs);
        assert!(report.passed, "{:?}", report.reason_codes);
    }

    #[test]
    fn path_line_out_of_bounds_fails() {
        let evs = vec![evidence("src/lib.rs")];
        let report = validate_final_answer("[来源: src/lib.rs:99999999]", &[], &evs);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_PATH_LINE_OUT_OF_BOUNDS.to_string())
        );
    }

    #[test]
    fn document_identity_binds_to_project_doc() {
        let ledger = json!([{
            "source_id": "SRC-001",
            "source_title": "d",
            "source_url_or_ref": "docs/guide.md",
            "source_type": "project_doc",
            "visibility": "metadata_only",
            "observed_scope": "declaration only",
            "missing_scope": "content",
            "highest_allowed_claim": "synthesized",
        }]);
        let report = validate_final_answer("[来源: docs/guide.md §2]", &[ledger], &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers[0].binding, BindingKind::DocumentIdentity);
    }

    #[test]
    fn unresolvable_target_fails() {
        let report = validate_final_answer("[来源: 某记忆中的文档]", &[], &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_UNRESOLVABLE_CITATION.to_string())
        );
    }

    #[test]
    fn multiple_markers_union_reasons() {
        let report = validate_final_answer(
            "甲[来源: SRC-001] 乙[来源: SRC-999]",
            &[json!([ledger_entry(
                "SRC-001",
                "https://x",
                "full_text_observed"
            )])],
            &[],
        );
        assert!(!report.passed);
        assert_eq!(report.markers.len(), 2);
        assert!(
            report
                .reason_codes
                .contains(&REASON_UNKNOWN_SOURCE_ID.to_string())
        );
    }

    // ── P0-B step 5 review fixes (2026-08-14) ─────────────────────────────

    #[test]
    fn fullwidth_colon_marker_is_parsed() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "full_text_observed"
        )])];
        let report = validate_final_answer("结论[来源：SRC-001] 结束", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers.len(), 1);
        assert_eq!(report.markers[0].target, "SRC-001");
    }

    #[test]
    fn fenced_code_block_markers_are_skipped() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "full_text_observed"
        )])];
        // Unclosed AND unknown markers inside fences are literal examples,
        // not citations; the real marker outside still validates.
        let text = "```\n[来源: SRC-999\n```\n结论 [来源: SRC-001]";
        let report = validate_final_answer(text, &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers.len(), 1);
        assert_eq!(report.markers[0].target, "SRC-001");
    }

    #[test]
    fn fenced_code_block_with_language_tag_is_skipped() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "full_text_observed"
        )])];
        let text = "~~~markdown\n[来源: SRC-999]\n~~~\n[来源: SRC-001]";
        let report = validate_final_answer(text, &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers.len(), 1);
    }

    #[test]
    fn inline_code_markers_are_skipped() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x",
            "full_text_observed"
        )])];
        let text = "格式为 `[来源: SRC-999]`，实际引用 [来源: SRC-001]";
        let report = validate_final_answer(text, &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers.len(), 1);
    }

    #[test]
    fn run_unique_source_ids_bind_across_ledgers() {
        // A run may commit several ledgers; the review fix allocates
        // source_ids run-uniquely (SRC-001, SRC-002, ...), so a marker can
        // never first-match a different ledger's same-numbered entry.
        let ledgers = vec![
            json!([ledger_entry("SRC-001", "https://a", "full_text_observed")]),
            json!([ledger_entry("SRC-002", "https://b", "metadata_only")]),
        ];
        let report = validate_final_answer("[来源: SRC-002 metadata_only]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        // The claim cap consults the SECOND ledger's visibility — a
        // full-text tag must fail even though SRC-001 allows it.
        let report = validate_final_answer("[来源: SRC-002 observed]", &ledgers, &[]);
        assert!(!report.passed);
        assert!(
            report
                .reason_codes
                .contains(&REASON_CLAIM_EXCEEDS_VISIBILITY.to_string())
        );
    }

    #[test]
    fn url_canonical_variants_bind() {
        let ledgers = vec![json!([ledger_entry(
            "SRC-001",
            "https://x.example/doc",
            "full_text_observed"
        )])];
        // Explicit default port + uppercase host canonicalise to the same
        // identity as the ledger URL.
        let report =
            validate_final_answer("[来源: https://X.EXAMPLE:443/doc full_text_observed]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
        // A fragment is dropped by the shared canonicalisation.
        let report =
            validate_final_answer("[来源: https://x.example/doc#sec-2 full_text_observed]", &ledgers, &[]);
        assert!(report.passed, "{:?}", report.reason_codes);
    }

    #[test]
    fn url_binds_to_main_lane_web_evidence() {
        let mut ev = evidence("https://x.example/doc");
        ev.tool = "web_fetch".to_string();
        ev.source_type = "web_page".to_string();
        ev.visibility = "full_text_observed".to_string();
        let report = validate_final_answer(
            "[来源: https://X.EXAMPLE/doc full_text_observed]",
            &[],
            &[ev],
        );
        assert!(report.passed, "{:?}", report.reason_codes);
        assert_eq!(report.markers[0].binding, BindingKind::UrlIdentity);
    }
}
