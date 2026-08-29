//! Retrieval evidence — tool-call evidence records + the structured result
//! (source ledger / weighting / prefilter consumption) — batch B7 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

use std::collections::HashMap;

use orz_assurance::source_weighting::SourceWeightConfig;
use orz_assurance::{canonical_json, sha256_hex};

use crate::gateway::model::ToolCall;
use crate::host::ToolResult;

/// GAP-RETRIEVAL-TOOLS (2026-08-10): one retrieval-lane tool-call evidence
/// record — identity/source type/access time/visibility/observed-missing
/// scope/digest per ADR-0010 §3.7.4. Built mechanically from the tool call
/// and its result (never from model self-description).
#[derive(Debug, Clone)]
pub(crate) struct EvidenceRecord {
    pub tool: String,
    /// Stable identity — path (project docs) or URL (web).
    pub identity: String,
    /// Display title — basename for local files, the URL for web.
    pub title: String,
    /// `project_doc` | `web_page` | `web_search_result` | `local_file`.
    pub source_type: String,
    /// Four-grade visibility (§3.7.5).
    pub visibility: String,
    pub content_sha256: Option<String>,
    pub observed_scope: String,
    pub missing_scope: String,
    /// FUS-RETRIEVAL-MECH B-1 (2026-08-13): web_search citation URLs from
    /// the host's structured seam — the candidate pool for the mechanical
    /// prefilter. Empty for every other tool; never parsed from text.
    pub candidate_urls: Vec<String>,
    /// FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): the web_search query
    /// that produced this candidate pool — the lexical relevance input for
    /// the mechanical prefilter. `None` for every other tool.
    pub search_query: Option<String>,
    /// RFC 3339 access timestamp (journal format).
    pub accessed_at: String,
}

/// FUS-RETRIEVAL-MECH B-1 (2026-08-13): mechanically extract the web_search
/// citation URLs from the host's structured payload. Shape-checked (a
/// `citations` array of strings), deduplicated preserving first-seen order,
/// and never derived from model-visible output text.
pub(crate) fn structured_candidate_urls(result: &ToolResult) -> Vec<String> {
    let Some(value) = result.structured.as_ref() else {
        return Vec::new();
    };
    let Some(citations) = value.get("citations").and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::new();
    citations
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .filter(|url| !url.is_empty() && seen.insert(url.clone()))
        .collect()
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): mechanically derive the evidence record
/// for a retrieval-lane host tool call (ADR-0010 §3.7.5 visibility table —
/// a failed call produces NO evidence; read_file success = full text;
/// web_fetch = full/partial by truncation markers; web_search = partial
/// (its snippet is partial text); project_doc_index = full/metadata by the
/// include_content argument). Pure — the structured ledger is built from
/// these records, never from model self-description.
pub(crate) fn build_evidence_record(
    tool: &str,
    tc: &ToolCall,
    result: &ToolResult,
) -> Option<EvidenceRecord> {
    if result.exit_code != Some(0) {
        return None;
    }
    let output = result.output.trim();
    if output.is_empty() {
        return None;
    }
    let arg = |key: &str| tc.arguments.get(key).and_then(|v| v.as_str());
    let identity = arg("path")
        .or_else(|| arg("url"))
        .or_else(|| arg("query"))
        .or_else(|| arg("document_id"))
        .unwrap_or(tool)
        .to_string();
    // FUS-RETRIEVAL-MECH B-1 (2026-08-13): only web_search carries citation
    // URLs across the structured seam; everything else has an empty pool.
    let candidate_urls = if tool == "web_search" {
        structured_candidate_urls(result)
    } else {
        Vec::new()
    };
    // FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): the query rides the
    // evidence record so the prefilter can score lexical relevance at
    // ledger formation time (mechanical keyword overlap, never semantic).
    let search_query = (tool == "web_search")
        .then(|| arg("query").map(str::to_string))
        .flatten();
    let title = match tool {
        "read_file" | "project_doc_index" => identity
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&identity)
            .to_string(),
        _ => identity.clone(),
    };
    // PDF evidence (2026-08-11): when web_fetch output carries the inline
    // evidence marker, the content digest is the PDF's own sha256 (parsed
    // from the marker), not a hash of the preview text. The parsed value is
    // shape-checked (64 hex chars — review P3-8) so page text that merely
    // CONTAINS a `document_id=sha256:` fragment cannot fabricate a
    // pdf_document record.
    let pdf_hex = (tool == "web_fetch")
        .then(|| {
            output
                .split("document_id=sha256:")
                .nth(1)?
                .split(',')
                .next()
                .map(|s| s.trim().to_string())
        })
        .flatten()
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()));
    let (source_type, visibility, observed_scope, missing_scope) = match tool {
        "read_file" => ("local_file", "full_text_observed", "file content", "none"),
        "web_fetch" => {
            // PDF evidence (2026-08-11): the inline marker
            // "PDF evidence: {N} pages, document_id=sha256:{hex},
            // text_layer={yes|no}" drives the visibility table — a
            // text-layer-less document is metadata only (never full-text
            // attribution); truncated previews are partial (§3.7.5).
            if pdf_hex.is_some() {
                let no_text = output.contains("text_layer=no");
                let truncated =
                    output.contains("[web_fetch pdf content truncated") || output.len() > 200_000;
                if no_text {
                    (
                        "pdf_document",
                        "metadata_only",
                        "metadata only (no text layer)",
                        "page text",
                    )
                } else if truncated {
                    (
                        "pdf_document",
                        "partial_text_observed",
                        "first portion",
                        "rest of document",
                    )
                } else {
                    (
                        "pdf_document",
                        "full_text_observed",
                        "extracted text layer",
                        "none",
                    )
                }
            } else if output.contains("PDF downloaded") {
                // Legacy save-to-downloads path (host without an evidence
                // root): the output is a download hint, NOT document text.
                // 2026-08-11 bug fix — this was previously mis-attributed
                // full_text_observed.
                (
                    "web_page",
                    "metadata_only",
                    "download metadata only",
                    "page content",
                )
            } else {
                // H2 (review 2026-08-10): the fetch pipeline's truncation
                // footer is "[web_fetch content truncated: ..." (codegen
                // overflow.rs) — matching that prefix (plus the bounded-budget
                // "[truncated]" fallback marker and a length backstop) is what
                // actually detects a truncated page; the old substrings missed
                // the real footer and granted full-level attribution to
                // truncated text (§3.7.5).
                //
                // PDF evidence (2026-08-11 review D1-1): a whitelisted
                // `web_fetch` intercepted to the browser lane renders an
                // HTML page shaped like browser_read — its truncation footer
                // ("[browser_read content truncated: ...") must count here
                // too, or truncated intercepted pages get full-level
                // attribution.
                let truncated = output.contains("[web_fetch content truncated")
                    || output.contains("[browser_read content truncated")
                    || output.contains("[truncated")
                    || output.len() > 200_000;
                if truncated {
                    (
                        "web_page",
                        "partial_text_observed",
                        "first portion",
                        "rest of page",
                    )
                } else {
                    ("web_page", "full_text_observed", "full document", "none")
                }
            }
        }
        // PDF evidence (2026-08-11): `pdf_read` returns requested pages
        // from the local evidence store — truncated output is partial, else
        // the requested pages were fully observed.
        "pdf_read" => {
            let truncated =
                output.contains("[pdf_read content truncated") || output.len() > 200_000;
            if truncated {
                (
                    "pdf_document",
                    "partial_text_observed",
                    "requested pages",
                    "rest of document",
                )
            } else {
                (
                    "pdf_document",
                    "full_text_observed",
                    "requested pages",
                    "none",
                )
            }
        }
        // A search result's snippet is partial text (never full-text
        // attribution for a snippet — §3.7.5).
        "web_search" => (
            "web_search_result",
            "partial_text_observed",
            "search snippet",
            "full page",
        ),
        // local_browser (2026-08-10 / P0-B step 4): `browser_read` returns
        // rendered page text; `mode` controls the read scope. keywords mode
        // is excerpts by contract → always partial (§3.7.5); preview mode
        // is partial only when the mechanical footer says the page exceeded
        // the preview budget (a short page was returned complete); full
        // mode maps footer or length backstop → partial.
        "browser_read" => {
            let mode = arg("mode").unwrap_or("full");
            let truncated =
                output.contains("[browser_read content truncated") || output.len() > 200_000;
            match mode {
                "keywords" => (
                    "web_page",
                    "partial_text_observed",
                    "keyword excerpts",
                    "rest of page",
                ),
                "preview" if truncated => (
                    "web_page",
                    "partial_text_observed",
                    "first portion (preview)",
                    "rest of page",
                ),
                _ if truncated => (
                    "web_page",
                    "partial_text_observed",
                    "first portion",
                    "rest of page",
                ),
                _ => ("web_page", "full_text_observed", "full document", "none"),
            }
        }
        "project_doc_index" => {
            let include_content = arg("include_content") == Some("true");
            if include_content {
                (
                    "project_doc",
                    "full_text_observed",
                    "document content",
                    "none",
                )
            } else {
                (
                    "project_doc",
                    "metadata_only",
                    "metadata only",
                    "document content",
                )
            }
        }
        _ => return None,
    };
    Some(EvidenceRecord {
        tool: tool.to_string(),
        identity,
        title,
        source_type: source_type.to_string(),
        visibility: visibility.to_string(),
        content_sha256: pdf_hex.or_else(|| Some(sha256_hex(output.as_bytes()))),
        observed_scope: observed_scope.to_string(),
        missing_scope: missing_scope.to_string(),
        candidate_urls,
        search_query,
        accessed_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the committed structured result —
/// the five ADR-0010 §3.3.3 sections as an event payload, plus the digests
/// and mechanical facts the assessment reuses.
#[derive(Debug, Clone)]
pub(crate) struct StructuredCommittedResult {
    /// The full `retrieval_result_committed` payload.
    pub payload: serde_json::Value,
    pub result_digest: String,
    pub ledger_digest: String,
    pub source_counts: serde_json::Value,
    /// `Some` when degraded — the reason code for the assessment.
    pub validation_note: Option<String>,
}

/// The claim × visibility matrix (§3.7.5) — mechanical bounds.
fn claim_rank(strength: &str) -> u8 {
    match strength {
        "observed" => 3,
        "derived" => 2,
        "synthesized" => 1,
        _ => 0,
    }
}

fn visibility_rank(visibility: &str) -> u8 {
    match visibility {
        "full_text_observed" => 3,
        "partial_text_observed" => 2,
        "metadata_only" => 1,
        _ => 0,
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): form the structured retrieval result
/// (ADR-0010 §3.3.3/§3.7.4/§3.7.5). The ledger/query_summary/filtering_log/
/// raw_source_refs are built MECHANICALLY from tool-call evidence; only the
/// organized_response comes from the model's `[RESULT_JSON]` block, and it
/// is validated — source_ids must reference the ledger and claim_strength
/// must respect the visibility matrix. Validation failure (or no block)
/// degrades the organized response to empty with an explicit
/// `visibility_degraded` + reason record — never a silent downgrade.
#[allow(clippy::too_many_arguments)] // the full result-formation contract
pub(crate) fn build_structured_result(
    evidence: &[EvidenceRecord],
    weight_config: &SourceWeightConfig,
    prefilter_config: &orz_assurance::candidate_prefilter::CandidatePrefilterConfig,
    source_seq: &mut u32,
    docs: &[String],
    sources: &[String],
    fallback_source_type: &str,
    output: &str,
    subagent_session_id: &str,
    activation_id: &str,
    contract_id: &str,
    contract_revision: u32,
    call_id: &str,
    task_goal: &str,
) -> StructuredCommittedResult {
    // 1. Mechanical ledger — tool-call evidence first (§3.7.4 identity/type/
    //    access time/visibility/observed-missing scope/digest + derived
    //    claim cap), then the [DOC]/[SOURCE] declaration lines as
    //    metadata-grade entries (a no-tool-call response is legal; the line
    //    contract stays a stable metadata interface).
    let mut source_ledger: Vec<serde_json::Value> = Vec::new();
    // FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): mechanical prefilter
    // removal log — every removal with a stable reason, bound to the
    // web_search_result ledger entry that owned the pool. Always emitted
    // (empty when nothing was removed) so the committed payload is
    // self-describing.
    let mut prefilter_log: Vec<serde_json::Value> = Vec::new();
    for ev in evidence.iter() {
        let source_id = format!("SRC-{:03}", *source_seq + 1);
        *source_seq += 1;
        let highest_allowed_claim = match ev.visibility.as_str() {
            "full_text_observed" => "observed",
            "partial_text_observed" => "derived",
            "metadata_only" => "synthesized",
            _ => "none",
        };
        let limitation = if ev.missing_scope != "none" {
            format!("missing scope: {}", ev.missing_scope)
        } else {
            String::new()
        };
        let mut entry = serde_json::json!({
            "source_id": source_id,
            "source_title": ev.title,
            "source_url_or_ref": ev.identity,
            "source_type": ev.source_type,
            "visibility": ev.visibility,
            "accessed_at": ev.accessed_at,
            "observed_scope": ev.observed_scope,
            "missing_scope": ev.missing_scope,
            "relevance": "direct",
            "used_in_sections": [],
            "highest_allowed_claim": highest_allowed_claim,
        });
        if let Some(digest) = &ev.content_sha256 {
            entry["content_sha256"] = serde_json::Value::String(digest.clone());
        }
        if !limitation.is_empty() {
            entry["limitation"] = serde_json::Value::String(limitation);
        }
        // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 —
        // mechanical tier/weight for page evidence: web_fetch/browser_read
        // pages and URL-shaped PDF documents. Project docs, local files and
        // web_search summary entries carry no tier — the search's citation
        // entries carry the candidate pool instead (FUS-RETRIEVAL-MECH B-1);
        // the fetched/declared pages carry the weight.
        let web_evidence = ev.source_type == "web_page"
            || (ev.source_type == "pdf_document"
                && (ev.identity.starts_with("http://") || ev.identity.starts_with("https://")));
        if web_evidence {
            let weighted = weight_config.classify(&ev.identity);
            entry["tier"] = weighted.tier.as_str().into();
            entry["mechanical_weight"] = serde_json::json!(weighted.weight);
            entry["weight_reason"] = weighted.reason.into();
        }
        // FUS-RETRIEVAL-MECH B-1 + step 3 (2026-08-13/14): web_search
        // citation URLs ride the ledger entry as the PRE-FILTERED candidate
        // pool — canonical/host dedup, known failure forms removed,
        // sorted by tier/weight + lexical relevance. The raw pool is never
        // written; the retained pool mirrors `candidate_pool` metadata and
        // removals land in `prefilter_log` (auditable, never silent).
        if !ev.candidate_urls.is_empty() {
            let report = orz_assurance::candidate_prefilter::prefilter(
                &ev.candidate_urls,
                ev.search_query.as_deref().unwrap_or_default(),
                weight_config,
                prefilter_config,
            );
            entry["candidate_urls"] = serde_json::Value::Array(
                report
                    .retained
                    .iter()
                    .map(|candidate| serde_json::Value::String(candidate.url.clone()))
                    .collect(),
            );
            entry["candidate_pool"] = serde_json::Value::Array(
                report
                    .retained
                    .iter()
                    .map(|candidate| {
                        serde_json::to_value(candidate).expect("prefiltered candidate serializes")
                    })
                    .collect(),
            );
            for removed in report.removed {
                let mut log_entry = serde_json::json!({
                    "source_id": source_id,
                    "url": removed.url,
                    "reason": removed.reason.as_str(),
                    "action": "removed",
                    "filtered_at": chrono::Utc::now().to_rfc3339(),
                });
                if let Some(canonical_url) = &removed.canonical_url {
                    log_entry["canonical_url"] = serde_json::Value::String(canonical_url.clone());
                }
                prefilter_log.push(log_entry);
            }
        }
        source_ledger.push(entry);
    }
    // [DOC]/[SOURCE] declaration lines — metadata-grade (never full-text
    // attribution for a declaration; §3.7.5).
    for doc in docs {
        let source_id = format!("SRC-{:03}", *source_seq + 1);
        *source_seq += 1;
        source_ledger.push(serde_json::json!({
            "source_id": source_id,
            "source_title": doc,
            "source_url_or_ref": doc,
            "source_type": "project_doc",
            "visibility": "metadata_only",
            "accessed_at": chrono::Utc::now().to_rfc3339(),
            "observed_scope": "declaration only",
            "missing_scope": "content",
            "relevance": "direct",
            "used_in_sections": [],
            "highest_allowed_claim": "synthesized",
        }));
    }
    for src in sources {
        let source_id = format!("SRC-{:03}", *source_seq + 1);
        *source_seq += 1;
        let mut entry = serde_json::json!({
            "source_id": source_id,
            "source_title": src,
            "source_url_or_ref": src,
            "source_type": fallback_source_type,
            "visibility": "metadata_only",
            "accessed_at": chrono::Utc::now().to_rfc3339(),
            "observed_scope": "declaration only",
            "missing_scope": "content",
            "relevance": "direct",
            "used_in_sections": [],
            "highest_allowed_claim": "synthesized",
        });
        // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): external [SOURCE]
        // declaration lines are metadata-grade web sources — the same
        // mechanical judge applies (internal [DOC] lines stay unweighted).
        if fallback_source_type == "web_page" {
            let weighted = weight_config.classify(src);
            entry["tier"] = weighted.tier.as_str().into();
            entry["mechanical_weight"] = serde_json::json!(weighted.weight);
            entry["weight_reason"] = weighted.reason.into();
        }
        source_ledger.push(entry);
    }

    // 2. Model organized block — validated against the ledger.
    let model_block = crate::agents::retrieval::parse_retrieval_result_json(output);
    let mut degraded = model_block.is_none();
    let mut validation_note: Option<String> = None;
    let mut sections: Vec<serde_json::Value> = Vec::new();
    let mut claims: Vec<serde_json::Value> = Vec::new();
    let mut used_in_sections: HashMap<String, Vec<String>> = HashMap::new();
    let mut block_accepted = false;
    if let Some(block) = &model_block {
        let ledger_ids: std::collections::HashSet<String> = source_ledger
            .iter()
            .filter_map(|e| e["source_id"].as_str().map(str::to_string))
            .collect();
        let section_list = block
            .get("sections")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let claim_list = block
            .get("claims")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut ok = true;
        for section in &section_list {
            let strength = section.get("claim_strength").and_then(|v| v.as_str());
            let ids: Vec<&str> = section
                .get("source_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            let title = section
                .get("section_title")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            if strength.is_none()
                || ids.is_empty()
                || !ids.iter().all(|id| ledger_ids.contains(*id))
                || !title.is_empty() && section.get("content").and_then(|v| v.as_str()).is_none()
            {
                ok = false;
                break;
            }
            // §3.7.5 matrix: every bound source must satisfy the strength.
            for id in &ids {
                let entry = source_ledger
                    .iter()
                    .find(|e| e["source_id"].as_str() == Some(*id))
                    .unwrap();
                if claim_rank(strength.unwrap())
                    > visibility_rank(entry["visibility"].as_str().unwrap_or_default())
                {
                    ok = false;
                    break;
                }
            }
            for id in &ids {
                used_in_sections
                    .entry((*id).to_string())
                    .or_default()
                    .push(title.to_string());
            }
        }
        for claim in &claim_list {
            let strength = claim.get("claim_strength").and_then(|v| v.as_str());
            let ids: Vec<&str> = claim
                .get("source_ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if strength.is_none()
                || ids.is_empty()
                || !ids.iter().all(|id| ledger_ids.contains(*id))
            {
                ok = false;
                break;
            }
            for id in &ids {
                let entry = source_ledger
                    .iter()
                    .find(|e| e["source_id"].as_str() == Some(*id))
                    .unwrap();
                if claim_rank(strength.unwrap())
                    > visibility_rank(entry["visibility"].as_str().unwrap_or_default())
                {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            sections = section_list;
            claims = claim_list;
            block_accepted = true;
        } else {
            degraded = true;
            validation_note = Some("structured_result_validation_failed".to_string());
        }
    } else {
        validation_note = Some("structured_result_validation_failed".to_string());
    }
    // Back-fill `used_in_sections` from the accepted block.
    for entry in &mut source_ledger {
        if let Some(ids) = used_in_sections.get(entry["source_id"].as_str().unwrap_or_default()) {
            entry["used_in_sections"] = serde_json::Value::Array(
                ids.iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
        }
    }

    // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 layer 3 —
    // the sub-agent's model weighting annotation. v0 semantics: annotate +
    // rank, never intercept. Valid annotations merge into the ledger
    // (model_weight / model_weight_reason / annotation_status) AND are
    // echoed into organized_response.source_annotations; invalid entries
    // are DROPPED and recorded in the filtering_log — explicit, never a
    // silent ignore. The merge rules mirror the Python verifier
    // (assurance/run_event_journal_validation.py `_verify_v02_source_weighting`):
    // annotated MUST carry 0.7, adopted MUST carry >= 1.0, a mechanically
    // low_quality source MUST NOT be adopted, and one source_id may be
    // annotated at most once.
    let mut filtering_log: Vec<serde_json::Value> = Vec::new();
    let mut merged_annotations: Vec<serde_json::Value> = Vec::new();
    if block_accepted && let Some(block) = &model_block {
        if let Some(annotations) = block.get("source_annotations").and_then(|v| v.as_array()) {
            let ledger_ids: std::collections::HashSet<String> = source_ledger
                .iter()
                .filter_map(|e| e["source_id"].as_str().map(str::to_string))
                .collect();
            let mut seen_annotation_ids: std::collections::HashSet<String> =
                std::collections::HashSet::new();
            for annotation in annotations {
                let sid = annotation
                    .get("source_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let weight = annotation.get("weight").and_then(|v| v.as_f64());
                let reason = annotation
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let status = annotation
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let valid_weight = weight.is_some_and(|w| {
                    (w - 0.7).abs() < 1e-9 || (w - 1.0).abs() < 1e-9 || (w - 1.1).abs() < 1e-9
                });
                let valid_status = matches!(status, "adopted" | "annotated");
                let status_weight_consistent = weight.is_some_and(|w| match status {
                    "annotated" => (w - 0.7).abs() < 1e-9,
                    "adopted" => w >= 1.0 - 1e-9,
                    _ => false,
                });
                let low_quality = source_ledger.iter().any(|e| {
                    e["source_id"].as_str() == Some(sid)
                        && e.get("tier").and_then(|t| t.as_str()) == Some("low_quality")
                });
                let low_quality_not_adopted = !(low_quality && status == "adopted");
                // Duplicate detection counts only annotations that will
                // merge: an invalid first annotation must not poison the
                // source_id for a later valid one.
                let duplicate = seen_annotation_ids.contains(sid);
                let known_source = ledger_ids.contains(sid);
                if known_source
                    && valid_weight
                    && valid_status
                    && status_weight_consistent
                    && low_quality_not_adopted
                    && !duplicate
                    && !reason.is_empty()
                {
                    if let Some(entry) = source_ledger
                        .iter_mut()
                        .find(|e| e["source_id"].as_str() == Some(sid))
                    {
                        seen_annotation_ids.insert(sid.to_string());
                        entry["model_weight"] = serde_json::json!(weight.unwrap());
                        entry["model_weight_reason"] =
                            serde_json::Value::String(reason.to_string());
                        entry["annotation_status"] = serde_json::Value::String(status.to_string());
                        merged_annotations.push(annotation.clone());
                    }
                } else {
                    filtering_log.push(serde_json::json!({
                        "source_id": if sid.is_empty() {
                            "SRC-UNKNOWN".to_string()
                        } else {
                            sid.to_string()
                        },
                        "reason": "annotation_invalid",
                        "action": "excluded",
                        "filtered_at": chrono::Utc::now().to_rfc3339(),
                    }));
                }
            }
        }

        // Verifier-aligned rule: a low_quality source that is USED in the
        // organized response MUST carry an "annotated" annotation. If the
        // model did not provide one (or it was dropped above), the whole
        // block is invalid and degrades explicitly — the same treatment as
        // any other structured-result validation failure — so the committed
        // journal always satisfies the Python verifier.
        let used_source_ids: std::collections::HashSet<String> = sections
            .iter()
            .chain(claims.iter())
            .flat_map(|item| {
                item.get("source_ids")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|v| v.as_str().map(str::to_string))
            })
            .collect();
        let missing_low_quality_annotation = source_ledger.iter().any(|e| {
            e.get("tier").and_then(|t| t.as_str()) == Some("low_quality")
                && used_source_ids.contains(e["source_id"].as_str().unwrap_or_default())
                && e.get("annotation_status").and_then(|v| v.as_str()) != Some("annotated")
        });
        if missing_low_quality_annotation {
            degraded = true;
            validation_note = Some("low_quality_source_without_annotation".to_string());
            sections.clear();
            claims.clear();
            merged_annotations.clear();
            for entry in &mut source_ledger {
                entry["used_in_sections"] = serde_json::json!([]);
                if let Some(obj) = entry.as_object_mut() {
                    obj.remove("model_weight");
                    obj.remove("model_weight_reason");
                    obj.remove("annotation_status");
                }
            }
        }
    }

    // 3. query_summary — one mechanical entry for the dispatch.
    let query_summary = vec![serde_json::json!({
        "query_id": format!("QRY-{}", &sha256_hex(call_id.as_bytes())[..8]),
        "query_text": task_goal,
        "source_category": if evidence.iter().any(|e| e.source_type == "project_doc" || e.source_type == "local_file") { "project_docs" } else { "web" },
        "result_count": source_ledger.len(),
        "action_taken": "searched",
        "tool_used": evidence.first().map(|e| e.tool.as_str()).unwrap_or("retrieval_dispatch"),
    })];

    // 4. filtering_log — mechanical filter events; today only invalid
    //    source-annotation drops land here (the real tools' policy
    //    refusals land with the web client wiring).

    // 5. raw_source_refs — mechanical projection of the ledger.
    let raw_source_refs: Vec<serde_json::Value> = source_ledger
        .iter()
        .map(|e| {
            let mut ref_entry = serde_json::json!({
                "source_id": e["source_id"],
                "source_title": e["source_title"],
                "source_url_or_ref": e["source_url_or_ref"],
                "visibility": e["visibility"],
                "content_sha256": e.get("content_sha256").cloned().unwrap_or(serde_json::Value::Null),
            });
            // FUS-RETRIEVAL-MECH B-1 + step 3 (2026-08-13/14): the raw
            // projection mirrors the ledger's prefiltered candidate pool
            // for web_search entries — both the retained URL list and the
            // per-candidate metadata.
            if let Some(candidates) = e.get("candidate_urls") {
                ref_entry["candidate_urls"] = candidates.clone();
            }
            if let Some(pool) = e.get("candidate_pool") {
                ref_entry["candidate_pool"] = pool.clone();
            }
            ref_entry
        })
        .collect();

    // 6. source_counts — mechanical distribution of the ledger.
    let mut counts = [
        ("full_text_observed", 0u64),
        ("partial_text_observed", 0u64),
        ("metadata_only", 0u64),
        ("unavailable", 0u64),
    ];
    for entry in &source_ledger {
        let vis = entry["visibility"].as_str().unwrap_or_default();
        if let Some((_, n)) = counts.iter_mut().find(|(k, _)| *k == vis) {
            *n += 1;
        }
    }
    let source_counts = serde_json::json!({
        "total": source_ledger.len(),
        "full_text_observed": counts[0].1,
        "partial_text_observed": counts[1].1,
        "metadata_only": counts[2].1,
        "unavailable": counts[3].1,
    });

    let mut organized_response = serde_json::json!({ "sections": sections, "claims": claims });
    if !merged_annotations.is_empty() {
        organized_response["source_annotations"] = serde_json::Value::Array(merged_annotations);
    }
    let five_fields = serde_json::json!({
        "query_summary": query_summary,
        "source_ledger": source_ledger,
        "filtering_log": filtering_log,
        "organized_response": organized_response,
        "raw_source_refs": raw_source_refs,
    });
    let result_digest = sha256_hex(&canonical_json(&five_fields).unwrap_or_default());
    let ledger_digest =
        sha256_hex(&canonical_json(&five_fields["source_ledger"]).unwrap_or_default());
    let result_id = format!(
        "RET-RES-{}-{}",
        &result_digest[..16],
        &sha256_hex(call_id.as_bytes())[..8],
    );
    let payload = serde_json::json!({
        "schema_version": "0.2.0-draft",
        "result_kind": "retrieval_subagent_result",
        "result_id": result_id,
        "activation_id": activation_id,
        "subagent_session_id": subagent_session_id,
        "contract_id": contract_id,
        "contract_revision": contract_revision,
        "result_digest": result_digest,
        "ledger_digest": ledger_digest,
        "query_summary": five_fields["query_summary"],
        "source_ledger": five_fields["source_ledger"],
        "filtering_log": five_fields["filtering_log"],
        "organized_response": five_fields["organized_response"],
        "raw_source_refs": five_fields["raw_source_refs"],
        "prefilter_log": prefilter_log,
        "source_counts": source_counts,
        "visibility_degraded": degraded,
    });
    StructuredCommittedResult {
        payload,
        result_digest,
        ledger_digest,
        source_counts,
        validation_note,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evidence visibility table (§3.7.5): read_file=full, web_fetch=full
    /// (or partial when truncated), web_search=partial, project_doc_index by
    /// include_content. Failed calls produce no evidence.
    #[test]
    fn evidence_visibility_table_is_mechanical() {
        let call = |name: &str, args: serde_json::Value| ToolCall {
            name: name.to_string(),
            arguments: args,
            call_id: "c1".to_string(),
        };
        let ok = |output: &str| ToolResult {
            output: output.to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: None,
            ..Default::default()
        };
        let fail = ToolResult {
            output: "boom".to_string(),
            exit_code: Some(1),
            output_encoding: None,
            structured: None,
            ..Default::default()
        };
        // Failed call → no evidence.
        assert!(
            build_evidence_record(
                "read_file",
                &call("read_file", serde_json::json!({"path": "a.rs"})),
                &fail
            )
            .is_none()
        );
        // read_file success → full text.
        let e = build_evidence_record(
            "read_file",
            &call("read_file", serde_json::json!({"path": "src/a.rs"})),
            &ok("x"),
        )
        .unwrap();
        assert_eq!(e.visibility, "full_text_observed");
        assert_eq!(e.source_type, "local_file");
        assert_eq!(e.identity, "src/a.rs");
        // web_fetch full vs truncated. The truncated sample uses the fetch
        // pipeline's REAL footer ("[web_fetch content truncated: showing
        // first N of M bytes...]", codegen overflow.rs) — the pre-review
        // detector matched "[truncated", which that footer does not contain,
        // and granted full-level attribution to truncated text (H2).
        let w = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://x.com"})),
            &ok("page"),
        )
        .unwrap();
        assert_eq!(w.visibility, "full_text_observed");
        let t = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://x.com"})),
            &ok("first portion\n\n[web_fetch content truncated: showing first 1000 of 5000 bytes]"),
        )
        .unwrap();
        assert_eq!(t.visibility, "partial_text_observed");
        // The bounded-budget fallback marker and a long output also count.
        let t2 = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://x.com"})),
            &ok("x\n[truncated]"),
        )
        .unwrap();
        assert_eq!(t2.visibility, "partial_text_observed");
        // web_search → partial (snippet).
        let s = build_evidence_record(
            "web_search",
            &call("web_search", serde_json::json!({"query": "q"})),
            &ok("snippet"),
        )
        .unwrap();
        assert_eq!(s.visibility, "partial_text_observed");
        assert_eq!(s.source_type, "web_search_result");
        // project_doc_index: metadata vs content mode.
        let m = build_evidence_record(
            "project_doc_index",
            &call("project_doc_index", serde_json::json!({"query": "q"})),
            &ok("meta"),
        )
        .unwrap();
        assert_eq!(m.visibility, "metadata_only");
        let f = build_evidence_record(
            "project_doc_index",
            &call(
                "project_doc_index",
                serde_json::json!({"query": "q", "include_content": "true"}),
            ),
            &ok("content"),
        )
        .unwrap();
        assert_eq!(f.visibility, "full_text_observed");
        // local_browser (2026-08-10): browser_read full vs truncated — the
        // host's mechanical footer ("[browser_read content truncated: ...")
        // and the length backstop map to partial (§3.7.5); source_type is
        // web_page (the transport difference lives in evidence.tool).
        let b = build_evidence_record(
            "browser_read",
            &call("browser_read", serde_json::json!({"url": "https://x.com"})),
            &ok("page text"),
        )
        .unwrap();
        assert_eq!(b.visibility, "full_text_observed");
        assert_eq!(b.source_type, "web_page");
        let bt = build_evidence_record(
            "browser_read",
            &call("browser_read", serde_json::json!({"url": "https://x.com"})),
            &ok("first portion\n\n[browser_read content truncated: 100000 chars, page text only]"),
        )
        .unwrap();
        assert_eq!(bt.visibility, "partial_text_observed");
        let bl = build_evidence_record(
            "browser_read",
            &call("browser_read", serde_json::json!({"url": "https://x.com"})),
            &ok(&"x".repeat(200_001)),
        )
        .unwrap();
        assert_eq!(bl.visibility, "partial_text_observed");
        // P0-B step 4 (2026-08-14): mode-aware evidence — preview of a
        // short page (no footer) stays full; truncated preview maps to
        // partial; keywords excerpts are partial by contract.
        let bp_short = build_evidence_record(
            "browser_read",
            &call(
                "browser_read",
                serde_json::json!({"url": "https://x.com", "mode": "preview"}),
            ),
            &ok("page text"),
        )
        .unwrap();
        assert_eq!(bp_short.visibility, "full_text_observed");
        assert_eq!(bp_short.observed_scope, "full document");
        let bp_trunc = build_evidence_record(
            "browser_read",
            &call(
                "browser_read",
                serde_json::json!({"url": "https://x.com", "mode": "preview"}),
            ),
            &ok("first portion\n\n[browser_read content truncated: preview, first 4000 chars, page text only]"),
        )
        .unwrap();
        assert_eq!(bp_trunc.visibility, "partial_text_observed");
        assert_eq!(bp_trunc.observed_scope, "first portion (preview)");
        let bk = build_evidence_record(
            "browser_read",
            &call(
                "browser_read",
                serde_json::json!({
                    "url": "https://x.com",
                    "mode": "keywords",
                    "keywords": ["term"],
                }),
            ),
            &ok("excerpt…\n\n[browser_read content truncated: keyword excerpts (1 terms, 1 excerpts), page text only]"),
        )
        .unwrap();
        assert_eq!(bk.visibility, "partial_text_observed");
        assert_eq!(bk.observed_scope, "keyword excerpts");
        // PDF evidence (2026-08-11): the inline marker
        // "PDF evidence: N pages, document_id=sha256:..., text_layer=..."
        // drives visibility; content_sha256 is the DOCUMENT digest parsed
        // from the marker, not a hash of the preview text.
        let marker = |text_layer: &str, body: &str| {
            format!(
                "PDF evidence: 2 pages, document_id=sha256:ab{}, text_layer={text_layer}\n\n{body}",
                "c".repeat(62)
            )
        };
        let p = build_evidence_record(
            "web_fetch",
            &call(
                "web_fetch",
                serde_json::json!({"url": "https://x.com/paper.pdf"}),
            ),
            &ok(&marker("yes", "page text")),
        )
        .unwrap();
        assert_eq!(p.visibility, "full_text_observed");
        assert_eq!(p.source_type, "pdf_document");
        assert_eq!(
            p.content_sha256.as_deref(),
            Some("abcccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")
        );
        let pt = build_evidence_record(
            "web_fetch",
            &call(
                "web_fetch",
                serde_json::json!({"url": "https://x.com/paper.pdf"}),
            ),
            &ok(&marker(
                "yes",
                "first\n\n[web_fetch pdf content truncated: 50000 chars]",
            )),
        )
        .unwrap();
        assert_eq!(pt.visibility, "partial_text_observed");
        let pn = build_evidence_record(
            "web_fetch",
            &call(
                "web_fetch",
                serde_json::json!({"url": "https://x.com/scan.pdf"}),
            ),
            &ok(&marker("no", "")),
        )
        .unwrap();
        assert_eq!(pn.visibility, "metadata_only");
        assert_eq!(pn.source_type, "pdf_document");
        // Legacy save-to-downloads hint (no evidence root): download
        // metadata only — never full-text attribution (bug fix).
        let legacy = build_evidence_record(
            "web_fetch",
            &call(
                "web_fetch",
                serde_json::json!({"url": "https://x.com/paper.pdf"}),
            ),
            &ok("PDF downloaded (12345 bytes) and saved to /tmp/x.pdf."),
        )
        .unwrap();
        assert_eq!(legacy.visibility, "metadata_only");
        // pdf_read: full when untruncated, partial with the mechanical footer.
        let r = build_evidence_record(
            "pdf_read",
            &call(
                "pdf_read",
                serde_json::json!({"document_id": "sha256:abcd"}),
            ),
            &ok("--- Page 1 ---\ntext"),
        )
        .unwrap();
        assert_eq!(r.visibility, "full_text_observed");
        assert_eq!(r.source_type, "pdf_document");
        assert_eq!(r.identity, "sha256:abcd");
        let rt = build_evidence_record(
            "pdf_read",
            &call(
                "pdf_read",
                serde_json::json!({"document_id": "sha256:abcd"}),
            ),
            &ok("page\n\n[pdf_read content truncated: 100000 chars]"),
        )
        .unwrap();
        assert_eq!(rt.visibility, "partial_text_observed");
        // Intercepted browser-channel page (review D1-1): a whitelisted
        // web_fetch that renders an HTML page outputs browser_read-shaped
        // JSON with the browser_read truncation footer — must be partial.
        let intercepted = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://kns.cnki.net/kcms/detail"})),
            &ok("{\"url\":\"https://kns.cnki.net/kcms/detail\",\"title\":\"x\",\"content\":\"page text\\n\\n[browser_read content truncated: 100000 chars, page text only]\",\"truncated\":true}"),
        )
        .unwrap();
        assert_eq!(intercepted.visibility, "partial_text_observed");
        assert_eq!(intercepted.source_type, "web_page");
        // A fake marker fragment inside ordinary page text must NOT fabricate
        // a pdf_document record (review P3-8 — the hex shape check rejects
        // it), and a non-hex marker stays a plain web page.
        let fake_marker = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://x.com"})),
            &ok("this page mentions document_id=sha256:nothex at the end"),
        )
        .unwrap();
        assert_eq!(fake_marker.source_type, "web_page");
        assert_eq!(fake_marker.visibility, "full_text_observed");
        // Unknown tool → no evidence.
        assert!(
            build_evidence_record("bash", &call("bash", serde_json::json!({})), &ok("x")).is_none()
        );
    }

    /// FUS-RETRIEVAL-MECH B-1 (2026-08-13): web_search citation URLs ride
    /// the host's structured seam into the evidence record and the
    /// committed ledger — shape-checked, deduplicated, and mirrored in
    /// raw_source_refs (the candidate pool for the mechanical prefilter).
    #[test]
    fn web_search_citations_flow_into_candidate_pool() {
        let call = |name: &str, args: serde_json::Value| ToolCall {
            name: name.to_string(),
            arguments: args,
            call_id: "c1".to_string(),
        };
        let result = |citations: serde_json::Value| ToolResult {
            output: "snippet".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: Some(serde_json::json!({ "citations": citations })),
            ..Default::default()
        };

        // Extraction: shape-checked + dedup, first-seen order preserved.
        let urls = structured_candidate_urls(&ToolResult {
            output: "snippet".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: Some(serde_json::json!({
                "citations": ["https://a.example", "https://b.example", "https://a.example", "", 7]
            })),
            ..Default::default()
        });
        assert_eq!(
            urls,
            vec![
                "https://a.example".to_string(),
                "https://b.example".to_string()
            ]
        );
        assert!(
            structured_candidate_urls(&ToolResult {
                output: "x".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            })
            .is_empty()
        );
        assert!(
            structured_candidate_urls(&ToolResult {
                output: "x".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: Some(serde_json::json!({"other": []})),
                ..Default::default()
            })
            .is_empty()
        );

        // Evidence record: only web_search picks the pool up.
        let ev = build_evidence_record(
            "web_search",
            &call("web_search", serde_json::json!({"query": "q"})),
            &result(serde_json::json!([
                "https://a.example",
                "https://b.example"
            ])),
        )
        .unwrap();
        assert_eq!(ev.source_type, "web_search_result");
        assert_eq!(ev.candidate_urls.len(), 2);
        let non_search = build_evidence_record(
            "web_fetch",
            &call("web_fetch", serde_json::json!({"url": "https://x.com"})),
            &result(serde_json::json!(["https://a.example"])),
        )
        .unwrap();
        assert!(non_search.candidate_urls.is_empty());

        // Committed ledger: candidate_urls on the search entry + mirror in
        // raw_source_refs; source counts unchanged (candidates are not
        // observed sources).
        let mut source_seq = 0;
        let committed = build_structured_result(
            &[ev],
            &SourceWeightConfig::default(),
            &orz_assurance::candidate_prefilter::CandidatePrefilterConfig::default(),
            &mut source_seq,
            &[],
            &[],
            "web_page",
            "[RESULT_JSON]{\"sections\":[],\"claims\":[]}[/RESULT_JSON]",
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            "goal",
        );
        let ledger = committed.payload["source_ledger"].as_array().unwrap();
        assert_eq!(ledger.len(), 1);
        assert_eq!(
            ledger[0]["candidate_urls"],
            serde_json::json!(["https://a.example", "https://b.example"])
        );
        assert_eq!(
            ledger[0]["candidate_pool"][0]["url"],
            serde_json::json!("https://a.example")
        );
        assert_eq!(
            ledger[0]["candidate_pool"][1]["url"],
            serde_json::json!("https://b.example")
        );
        assert_eq!(committed.payload["prefilter_log"], serde_json::json!([]));
        let refs = committed.payload["raw_source_refs"].as_array().unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(
            refs[0]["candidate_urls"],
            serde_json::json!(["https://a.example", "https://b.example"])
        );
        assert_eq!(refs[0]["candidate_pool"], ledger[0]["candidate_pool"]);
        assert_eq!(committed.payload["source_counts"]["total"], 1);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 3 (2026-08-14): the mechanical
    /// prefilter shapes the committed candidate pool — canonical/host
    /// dedup, known failure forms removed with stable reasons, tier/weight
    /// and relevance sorting, and the full metadata mirrored in
    /// raw_source_refs (the schema/verifier contract).
    #[test]
    fn mechanical_prefilter_shapes_candidate_pool_and_log() {
        let call = ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "rust policy" }),
            call_id: "c-step3".to_string(),
        };
        let result = ToolResult {
            output: "snippet".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: Some(serde_json::json!({
                "citations": [
                    "https://www.gov.cn/policy/rust",
                    "https://example.com/login?next=/x",
                    "https://example.com/article?utm_source=x",
                    "https://example.com/article?utm_medium=y",
                    "https://www.example.com/",
                    "https://example.com/?ref=z",
                    "https://example.com/unrelated"
                ]
            })),
            ..Default::default()
        };
        let ev = build_evidence_record("web_search", &call, &result).unwrap();
        assert_eq!(ev.candidate_urls.len(), 7);
        assert_eq!(ev.search_query.as_deref(), Some("rust policy"));

        let mut source_seq = 0;
        let committed = build_structured_result(
            &[ev],
            &SourceWeightConfig::default(),
            &orz_assurance::candidate_prefilter::CandidatePrefilterConfig::default(),
            &mut source_seq,
            &[],
            &[],
            "web_page",
            "[RESULT_JSON]{\"sections\":[],\"claims\":[]}[/RESULT_JSON]",
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            "goal",
        );
        let ledger = committed.payload["source_ledger"].as_array().unwrap();
        let entry = &ledger[0];
        assert_eq!(entry["source_type"], serde_json::json!("web_search_result"));
        assert_eq!(
            entry["candidate_urls"],
            serde_json::json!([
                "https://www.gov.cn/policy/rust",
                "https://example.com/article?utm_source=x",
                "https://www.example.com/",
                "https://example.com/unrelated"
            ])
        );
        // Per-candidate metadata: tier/weight/relevance/canonical form.
        let pool = entry["candidate_pool"].as_array().unwrap();
        assert_eq!(pool.len(), 4);
        assert_eq!(pool[0]["tier"], serde_json::json!("authoritative"));
        assert_eq!(pool[0]["mechanical_weight"], serde_json::json!(1.1));
        assert_eq!(pool[0]["relevance"], serde_json::json!("direct"));
        assert_eq!(
            pool[1]["canonical_url"],
            serde_json::json!("https://example.com/article")
        );
        assert_eq!(pool[3]["relevance"], serde_json::json!("tangential"));
        // Removal log: login wall, canonical duplicate, host duplicate.
        let log = committed.payload["prefilter_log"].as_array().unwrap();
        let reasons: Vec<&str> = log.iter().map(|e| e["reason"].as_str().unwrap()).collect();
        assert_eq!(
            reasons,
            vec!["login_wall", "duplicate_canonical", "duplicate_host"]
        );
        assert!(
            log.iter()
                .all(|e| e["source_id"] == serde_json::json!("SRC-001"))
        );
        assert!(
            log.iter()
                .all(|e| e["action"] == serde_json::json!("removed"))
        );
        // raw_source_refs mirrors the prefiltered pool exactly.
        let refs = committed.payload["raw_source_refs"].as_array().unwrap();
        assert_eq!(refs[0]["candidate_urls"], entry["candidate_urls"]);
        assert_eq!(refs[0]["candidate_pool"], entry["candidate_pool"]);
        // Candidates are not observed sources — counts unchanged.
        assert_eq!(committed.payload["source_counts"]["total"], 1);
    }

    /// FUS-RETRIEVAL-MECH P0-B step 3 review fix (2026-08-14): a fully
    /// purified pool is legitimate — all candidates removed by the
    /// prefilter yields an empty retained pool + empty candidate_pool with
    /// every removal recorded in prefilter_log (the verifier must accept
    /// this state; schema already allows empty arrays).
    #[test]
    fn mechanical_prefilter_can_purify_entire_pool() {
        let call = ToolCall {
            name: "web_search".to_string(),
            arguments: serde_json::json!({ "query": "rust" }),
            call_id: "c-step3-empty".to_string(),
        };
        let result = ToolResult {
            output: "snippet".to_string(),
            exit_code: Some(0),
            output_encoding: None,
            structured: Some(serde_json::json!({
                "citations": [
                    "javascript:alert(1)",
                    "https://example.com/login",
                    "https://example.com/?redirect_url=https://other.example"
                ]
            })),
            ..Default::default()
        };
        let ev = build_evidence_record("web_search", &call, &result).unwrap();
        let mut source_seq = 0;
        let committed = build_structured_result(
            &[ev],
            &SourceWeightConfig::default(),
            &orz_assurance::candidate_prefilter::CandidatePrefilterConfig::default(),
            &mut source_seq,
            &[],
            &[],
            "web_page",
            "[RESULT_JSON]{\"sections\":[],\"claims\":[]}[/RESULT_JSON]",
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            "goal",
        );
        let ledger = committed.payload["source_ledger"].as_array().unwrap();
        assert_eq!(ledger[0]["candidate_urls"], serde_json::json!([]));
        assert_eq!(ledger[0]["candidate_pool"], serde_json::json!([]));
        let log = committed.payload["prefilter_log"].as_array().unwrap();
        assert_eq!(log.len(), 3);
        let reasons: Vec<&str> = log.iter().map(|e| e["reason"].as_str().unwrap()).collect();
        assert_eq!(reasons, vec!["bad_url", "login_wall", "redirect_chain"]);
        let refs = committed.payload["raw_source_refs"].as_array().unwrap();
        assert_eq!(refs[0]["candidate_urls"], serde_json::json!([]));
        assert_eq!(refs[0]["candidate_pool"], serde_json::json!([]));
    }
}
