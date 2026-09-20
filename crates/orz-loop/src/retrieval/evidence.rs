//! Retrieval evidence — tool-call evidence records + the structured result
//! (source ledger / weighting / prefilter consumption) — batch B7 of the
//! controller split (CONTROLLER_SPLIT_DESIGN_2026-08-29).
//!
//! Moved verbatim from `controller.rs`; mechanical extraction only —
//! behavior, events and journal chain unchanged.

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

/// 0at A 面（2026-09-20，S3 摩擦 N1）：query id 的**单一生成源**——
/// `QRY-{sha256(call_id)[..8]}` 与多 query 的 `QRY-{hash}-{i+1}` 后缀，
/// 与契约 `query_entry.query_id` 的形态约束（`^QRY-[A-Za-z0-9._-]+$`）逐
/// 字对应。`query_summary` 与证据谱系（`origin_query_id`）共用本函数。
pub(crate) fn query_ids_for(call_id: &str, query_count: usize) -> Vec<String> {
    let query_hash = &sha256_hex(call_id.as_bytes())[..8];
    (0..query_count)
        .map(|i| {
            if i == 0 {
                format!("QRY-{query_hash}")
            } else {
                format!("QRY-{query_hash}-{}", i + 1)
            }
        })
        .collect()
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): form the structured retrieval result
/// (ADR-0010 §3.3.3/§3.7.4/§3.7.5). GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C
/// (2026-08-30 用户裁决): the `[RESULT_JSON]` organized block is DELETED —
/// query_summary/source_ledger/filtering_log/raw_source_refs are built
/// MECHANICALLY from tool-call evidence + [DOC]/[SOURCE] declaration lines
/// (the mechanical ledger is the single result track; the model's
/// self-description is never mechanical fact). `visibility_degraded` is
/// redefined: true iff the ledger carries NO text-level evidence (zero
/// full/partial — metadata-only declarations or no tool calls), with an
/// explicit `no_fulltext_evidence` reason record — never a silent downgrade.
///
/// 0ar S2-D3（2026-09-19，检索批次回送设计 §5.4）：`queries` 支持多元素
/// ——同轮多检索合并为单激活多 query 时，`query_summary` 每query 一条
/// （契约 `query_entry` 数组本就多元素；契约面同批增可选
/// `usable_source_count` 逐 query 可用计数，宽口径按 search_query 归因，
/// 派生证据多 query 下不归因——边界登记于 `per_query_usable_counts`）。
#[allow(clippy::too_many_arguments)] // the full result-formation contract
pub(crate) fn build_structured_result(
    evidence: &[EvidenceRecord],
    weight_config: &SourceWeightConfig,
    prefilter_config: &orz_assurance::candidate_prefilter::CandidatePrefilterConfig,
    source_seq: &mut u32,
    docs: &[String],
    sources: &[String],
    fallback_source_type: &str,
    subagent_session_id: &str,
    activation_id: &str,
    contract_id: &str,
    contract_revision: u32,
    call_id: &str,
    queries: &[String],
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
            "highest_allowed_claim": "synthesized",
        }));
    }
    for src in sources {
        let source_id = format!("SRC-{:03}", *source_seq + 1);
        *source_seq += 1;
        // GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): [SOURCE]
        // 声明行 URL 规范化——SRC-002 表象修复：URL+标题同写的整行不再
        // 整体落入 source_url_or_ref。URL 形 token 抽取并走 ACAF 网络目标
        // 规范化（scheme/host 小写、去默认端口、去 fragment、userinfo
        // 拒绝；非 http(s) 或解析失败原样保留），剩余文本作 source_title。
        let (source_url_or_ref, source_title) = split_source_declaration(src);
        let mut entry = serde_json::json!({
            "source_id": source_id,
            "source_title": source_title,
            "source_url_or_ref": source_url_or_ref,
            "source_type": fallback_source_type,
            "visibility": "metadata_only",
            "accessed_at": chrono::Utc::now().to_rfc3339(),
            "observed_scope": "declaration only",
            "missing_scope": "content",
            "relevance": "direct",
            "highest_allowed_claim": "synthesized",
        });
        // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): external [SOURCE]
        // declaration lines are metadata-grade web sources — the same
        // mechanical judge applies to the NORMALIZED URL (internal [DOC]
        // lines stay unweighted).
        if fallback_source_type == "web_page" {
            let weighted = weight_config.classify(&source_url_or_ref);
            entry["tier"] = weighted.tier.as_str().into();
            entry["mechanical_weight"] = serde_json::json!(weighted.weight);
            entry["weight_reason"] = weighted.reason.into();
        }
        source_ledger.push(entry);
    }

    // 2. visibility_degraded — GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C
    //    (2026-08-30) 语义重定义：committed result 仅在机械 ledger 无
    //    文本级证据（full_text_observed/partial_text_observed 均为 0——
    //    纯 metadata 声明或无工具调用响应）时 degraded；旧「模型组织块
    //    缺失/非法」含义随块本身一起删除。filtering_log 恒空（annotation
    //    丢弃曾是唯一生产者；prefilter 移除在 prefilter_log 单轨）。
    let filtering_log: Vec<serde_json::Value> = Vec::new();
    let degraded = !source_ledger.iter().any(|e| {
        matches!(
            e["visibility"].as_str(),
            Some("full_text_observed" | "partial_text_observed")
        )
    });
    let validation_note = degraded.then(|| "no_fulltext_evidence".to_string());

    // 3. query_summary — one mechanical entry per query (0ar S2-D3,
    //    2026-09-19 检索批次回送设计 §5.4：同轮多检索合并为单激活多 query
    //    时逐 query 一条；单 query 与旧形态一致——query_text 为任务契约
    //    全文、result_count 为全 ledger 条数，既有 payload 逐字节不变）。
    //    契约面同批增可选 `usable_source_count`（宽口径逐 query 可用计数，
    //    按 search_query 归因；单 query 全额归属——见
    //    `batch_close::per_query_usable_counts`）。
    //    0at（2026-09-20，S3 摩擦 N1）：query id 生成收敛为单源
    //    `query_ids_for`——`query_summary` 与证据谱系（A 面
    //    `origin_query_id`）共用同一 id 域；多 query 批在每条工具证据的
    //    ledger 条目上落派发谱系（规则见
    //    `batch_close::origin_query_assignments`），单 query 批不落（payload
    //    逐字节不变，判据 ③）。批级缺口由可选 `unattributed_usable_count`
    //    披露（＝批级可用 − Σ 逐 query 可用，恒等式判据 ①）；0ax 的
    //    「无 URL 合成答案」单列数由可选 `synthetic_answer_count` 披露
    //    （>0 才落）。
    let query_summary: Vec<serde_json::Value> = {
        let effective_queries: &[String] = if queries.is_empty() {
            &[String::from("(no query)")]
        } else {
            queries
        };
        let per_query_usable =
            crate::retrieval::batch_close::per_query_usable_counts(effective_queries, evidence);
        let source_category = if evidence
            .iter()
            .any(|e| e.source_type == "project_doc" || e.source_type == "local_file")
        {
            "project_docs"
        } else {
            "web"
        };
        let query_ids = query_ids_for(call_id, effective_queries.len());
        effective_queries
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let mut entry = serde_json::json!({
                    "query_id": query_ids[i],
                    "query_text": q,
                    "source_category": source_category,
                    // 单 query：旧口径（全 ledger 条数）；多 query：按
                    // search_query 归因的证据条数（派生证据不归因）。
                    "result_count": if effective_queries.len() <= 1 {
                        source_ledger.len()
                    } else {
                        evidence
                            .iter()
                            .filter(|e| e.search_query.as_deref() == Some(q.as_str()))
                            .count()
                    },
                    "action_taken": "searched",
                    "tool_used": evidence
                        .first()
                        .map(|e| e.tool.as_str())
                        .unwrap_or("retrieval_dispatch"),
                });
                entry["usable_source_count"] =
                    serde_json::json!(per_query_usable.get(i).copied().unwrap_or(0));
                entry
            })
            .collect()
    };
    // 0at A 面：工具证据 ledger 条目的派发谱系（多 query 批限定；条目与
    // 证据同序——source_ledger 前 evidence.len() 条即工具证据）。
    if queries.len() > 1 {
        let assignments = crate::retrieval::batch_close::origin_query_assignments(
            queries,
            evidence,
            &query_ids_for(call_id, queries.len().max(1)),
        );
        for (entry, origin) in source_ledger
            .iter_mut()
            .take(evidence.len())
            .zip(assignments)
        {
            if let Some(id) = origin {
                entry["origin_query_id"] = serde_json::Value::String(id);
            }
        }
    }

    // 4. filtering_log — mechanical filter events. GAP-RETRIEVAL-STRUCTURED-
    //    RESULT 方向 C (2026-08-30): source-annotation drops were the only
    //    producer and are retired with the annotation layer — the field is
    //    kept as an always-empty mechanical slot (prefilter removals live
    //    in prefilter_log on the single-track ledger).

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

    // GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30)：organized_response
    // 随 [RESULT_JSON] 契约删除——committed payload 为机械四段
    // （query_summary/source_ledger/filtering_log/raw_source_refs），
    // 机械 ledger 单轨。
    let four_fields = serde_json::json!({
        "query_summary": query_summary,
        "source_ledger": source_ledger,
        "filtering_log": filtering_log,
        "raw_source_refs": raw_source_refs,
    });
    let result_digest = sha256_hex(&canonical_json(&four_fields).unwrap_or_default());
    let ledger_digest =
        sha256_hex(&canonical_json(&four_fields["source_ledger"]).unwrap_or_default());
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
        "query_summary": four_fields["query_summary"],
        "source_ledger": four_fields["source_ledger"],
        "filtering_log": four_fields["filtering_log"],
        "raw_source_refs": four_fields["raw_source_refs"],
        "prefilter_log": prefilter_log,
        "source_counts": source_counts,
        "visibility_degraded": degraded,
    });
    let mut payload = payload;
    // 0at B 面（多 query 批限定）：批级归因缺口显式披露——恒等式
    // `Σ 逐 query usable ＋ unattributed ＝ 批级 usable`（判据 ①）。单
    // query 批不落（全额归属、零缺口；payload 逐字节不变，判据 ③）。
    // 跨 query 重叠（同一 digest 落两桶）使 Σ 逐 query 超过批级、缺口
    // 饱和到 0——恒等式在该批不成立，留机械告警（可核、不阻断）。
    if queries.len() > 1 {
        let per_query_sum: u64 =
            crate::retrieval::batch_close::per_query_usable_counts(queries, evidence)
                .iter()
                .sum();
        let batch_usable = crate::retrieval::batch_close::usable_source_count(evidence);
        if per_query_sum > batch_usable {
            tracing::warn!(
                batch_usable,
                per_query_sum,
                "per-query usable counts overlap across queries (same content \
                 under multiple declared queries) — unattributed_usable_count \
                 saturates at 0 and the identity sum(per-query) + unattributed \
                 = batch does not hold this batch"
            );
        }
        payload["unattributed_usable_count"] = serde_json::json!(
            crate::retrieval::batch_close::unattributed_usable_count(queries, evidence)
        );
    }
    // 0ax S1：无 URL 合成答案单列数（>0 才落；缺席即「本批无合成答案」，
    // 不扰动单 query 批与既有 payload 形态）。
    let synthetic = crate::retrieval::batch_close::synthetic_answer_count(evidence);
    if synthetic > 0 {
        payload["synthetic_answer_count"] = serde_json::json!(synthetic);
    }
    StructuredCommittedResult {
        payload,
        result_digest,
        ledger_digest,
        source_counts,
        validation_note,
    }
}

/// GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): split a
/// `[SOURCE]` declaration line into `(url_or_ref, title)`. A URL-shaped
/// token (`http://` / `https://`) is extracted and canonicalized with the
/// ACAF network-target rules (`orz_assurance::acaf::target::resolve_network_url`
/// — scheme/host lowercased, explicit default port removed, fragment
/// dropped); userinfo URLs are REJECTED by the resolver and therefore left
/// verbatim (never canonicalized — the declaration line is not dropped).
/// Trailing CJK/Latin punctuation glued to the token is stripped before
/// parsing; a no-whitespace form ("https://example.com/，标题") splits at the
/// first punctuation separator after the scheme. Non-URL lines (internal
/// refs) stay as-is with an empty title.
fn split_source_declaration(line: &str) -> (String, String) {
    let line = line.trim();
    let lower = line.to_ascii_lowercase();
    let mut url_start = None;
    for (i, _) in line.char_indices() {
        if lower[i..].starts_with("http://") || lower[i..].starts_with("https://") {
            url_start = Some(i);
            break;
        }
    }
    let Some(start) = url_start else {
        // S4 实机复验（2026-08-31）：非 URL 声明行（internal ref /
        // 纯描述行）原样保留为 ref；source_title 回退为整行原文——
        // retrieval-result schema 要求 source_title 非空（nonempty
        // minLength 1），空 title 会导致实机 journal 严格校验违例。
        return (line.to_string(), line.trim().to_string());
    };
    let head = line[..start].trim();
    let tail = &line[start..];
    let (url_part, rest) = match tail.find(char::is_whitespace) {
        Some(ws) => {
            // S4 实机复验（2026-08-31）：子代理声明行常写
            // 「URL（full 读取，数据集官方卡片）」——第一个空格在
            // 「full」之后，若按空格切分 URL 会被截在「（full」尾巴上。
            // 在空格候选内再按全角左括号提前截断（URL 不含「（」；
            // 半角 '(' 保留——维基等合法 URL 可含半角括号）。
            let candidate = &tail[..ws];
            let rest0 = tail[ws..].trim();
            let scheme_len = if candidate.starts_with("https://") {
                8
            } else {
                7
            };
            match candidate[scheme_len..]
                .char_indices()
                .find(|(_, c)| *c == '（')
            {
                Some((i, _)) => {
                    let split = scheme_len + i;
                    let glued = &candidate[split..];
                    let title_part = if rest0.is_empty() {
                        glued
                            .trim_start_matches(|c: char| {
                                matches!(
                                    c,
                                    '。' | '，'
                                        | '、'
                                        | '；'
                                        | '：'
                                        | '！'
                                        | '？'
                                        | '（'
                                        | ','
                                        | ';'
                                        | ')'
                                        | ']'
                                        | '）'
                                        | '」'
                                        | ' '
                                )
                            })
                            .to_string()
                    } else {
                        format!(
                            "{} {rest0}",
                            glued
                                .trim_start_matches(|c: char| {
                                    matches!(
                                        c,
                                        '。' | '，'
                                            | '、'
                                            | '；'
                                            | '：'
                                            | '！'
                                            | '？'
                                            | '（'
                                            | ','
                                            | ';'
                                            | ')'
                                            | ']'
                                            | '）'
                                            | '」'
                                            | ' '
                                    )
                                })
                                .trim()
                        )
                    };
                    (&candidate[..split], title_part.trim().to_string())
                }
                None => (candidate, rest0.to_string()),
            }
        }
        None => {
            // No whitespace — the title may still be glued with CJK/Latin
            // punctuation ("https://example.com/，标题"). Split at the first
            // separator AFTER the scheme so the URL token ends cleanly
            // (ASCII ':' is not a separator: schemes and ports carry it).
            let scheme_len = if tail.starts_with("https://") { 8 } else { 7 };
            match tail[scheme_len..].char_indices().find(|(_, c)| {
                matches!(
                    c,
                    '。' | '，'
                        | '、'
                        | '；'
                        | '：'
                        | '！'
                        | '？'
                        | '（'
                        | ','
                        | ';'
                        | ')'
                        | ']'
                        | '）'
                        | '」'
                )
            }) {
                Some((i, _)) => {
                    let split = scheme_len + i;
                    let glued = &tail[split..];
                    let title_part = glued
                        .trim_start_matches(|c: char| {
                            matches!(
                                c,
                                '。' | '，'
                                    | '、'
                                    | '；'
                                    | '：'
                                    | '！'
                                    | '？'
                                    | '（'
                                    | ','
                                    | ';'
                                    | ')'
                                    | ']'
                                    | '）'
                                    | '」'
                                    | ' '
                            )
                        })
                        .trim();
                    (&tail[..split], title_part.to_string())
                }
                None => (tail, String::new()),
            }
        }
    };
    let mut url = url_part;
    while let Some(last) = url.chars().last() {
        if matches!(
            last,
            '。' | '，'
                | '、'
                | '；'
                | '：'
                | '！'
                | '？'
                | '（'
                | ','
                | ';'
                | ')'
                | ']'
                | '）'
                | '」'
        ) {
            url = &url[..url.len() - last.len_utf8()];
        } else {
            break;
        }
    }
    let url = url.trim();
    let canonical =
        orz_assurance::acaf::target::resolve_network_url(url).unwrap_or_else(|_| url.to_string());
    let title = format!("{head} {rest}").trim().to_string();
    let title = if title.is_empty() {
        canonical.clone()
    } else {
        title
    };
    (canonical, title)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::controller::AgentLoopController;
    use crate::controller_test_support::*;
    use crate::gateway::fake::{FakeProvider, ScriptedResponse};
    use crate::gateway::model::ModelGateway;
    use orz_assurance::{EventType, JournalRecorder};
    use std::sync::Arc;

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
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            &["goal".to_string()],
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
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            &["goal".to_string()],
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
            "sub-session",
            "act-1",
            "contract-1",
            0,
            "call-1",
            &["goal".to_string()],
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

    // ── GAP-RETRIEVAL-TOOLS (2026-08-10): structured result (§3.3.3) ──

    /// Evidence collection from the lane's host calls feeds the mechanical
    /// ledger: a read_file round yields a full-text source, and the
    /// committed result carries REAL visibility (full_text_observed > 0).
    #[tokio::test]
    async fn structured_result_uses_tool_evidence_with_real_visibility() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "fn main() {}  // file content".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            ScriptedResponse::text("[DOC] design.md\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查文档", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let commits: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::RetrievalResultCommitted)
            .collect();
        assert_eq!(commits.len(), 1, "{:?}", event_types(&dir));
        let p = &commits[0].payload;
        assert_eq!(p["result_kind"], "retrieval_subagent_result");
        assert_eq!(p["schema_version"], "0.2.0-draft");
        // read_file evidence = full-text source; the [DOC] line is a
        // metadata-only declaration — the merged ledger counts both.
        let ledger = p["source_ledger"].as_array().unwrap();
        assert_eq!(ledger.len(), 2);
        let full = ledger
            .iter()
            .find(|e| e["visibility"] == "full_text_observed")
            .unwrap();
        assert_eq!(full["source_type"], "local_file");
        assert!(full["content_sha256"].as_str().unwrap().len() == 64);
        assert_eq!(full["highest_allowed_claim"], "observed");
        assert_eq!(p["source_counts"]["total"], 2);
        assert_eq!(p["source_counts"]["full_text_observed"], 1);
        assert_eq!(p["source_counts"]["metadata_only"], 1);
        // GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): the
        // organized block is gone — the committed payload carries no
        // organized_response and visibility_degraded is mechanical: false
        // here because the ledger has full-text evidence.
        assert!(p.get("organized_response").is_none());
        assert_eq!(p["visibility_degraded"], false);
        // The assessment consumed the same mechanical counts.
        let assessments: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .collect();
        assert_eq!(assessments.len(), 1);
        assert_eq!(
            assessments[0].payload["source_counts"]["full_text_observed"],
            1
        );
        assert_eq!(assessments[0].payload["result_digest"], p["result_digest"]);
        // Artifact landed under the journal dir.
        let artifact_dir = dir.join("retrieval-results");
        assert!(artifact_dir.is_dir());
        assert_eq!(std::fs::read_dir(&artifact_dir).unwrap().count(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): a stray
    /// `[RESULT_JSON]` block in the subagent text is IGNORED — it is plain
    /// prose, never parsed; the committed result stays mechanical (no
    /// organized_response, no used_in_sections back-fill, no validation
    /// failure, visibility_degraded from the ledger's own evidence).
    #[tokio::test]
    async fn structured_result_ignores_stray_result_json_block() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "fn main() {}".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::tool_calls(vec![tool_call("read_file", "call-r1")]),
            ScriptedResponse::text(concat!(
                "[DOC] design.md\n检索完成\n",
                "[RESULT_JSON]",
                r#"{"sections":[{"section_title":"API","content":"入口函数","source_ids":["SRC-999"],"claim_strength":"observed"}],"claims":[]}"#,
                "[/RESULT_JSON]",
            )),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查文档", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let commit = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalResultCommitted)
            .unwrap();
        assert!(commit.payload.get("organized_response").is_none());
        assert_eq!(commit.payload["visibility_degraded"], false);
        // No back-fill: the stray block never touches the mechanical ledger.
        let ledger = commit.payload["source_ledger"].as_array().unwrap();
        let full = ledger
            .iter()
            .find(|e| e["visibility"] == "full_text_observed")
            .unwrap();
        assert!(full.get("used_in_sections").is_none());
        // No validation failure reason — the block is not a contract.
        let assessment = events
            .iter()
            .find(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .unwrap();
        assert!(
            !assessment.payload["reason_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "structured_result_validation_failed")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): a metadata-only
    /// result (no tool evidence, only a [DOC] declaration) is degraded by
    /// the MECHANICAL definition — visibility_degraded=true with the
    /// `no_fulltext_evidence` reason code (the old
    /// structured_result_validation_failed vocabulary is gone).
    #[tokio::test]
    async fn metadata_only_result_degrades_visibility_explicitly() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: None,
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            ScriptedResponse::tool_calls(vec![tool_call("retrieve_project_docs", "call-1")]),
            ScriptedResponse::text(concat!(
                "[DOC] design.md\n检索完成\n",
                "[RESULT_JSON]",
                r#"{"sections":[{"section_title":"API","content":"c","source_ids":["SRC-999"],"claim_strength":"observed"}],"claims":[]}"#,
                "[/RESULT_JSON]",
            )),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查文档", "RUN-RET", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let commit = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalResultCommitted)
            .unwrap();
        assert!(commit.payload.get("organized_response").is_none());
        assert_eq!(commit.payload["visibility_degraded"], true);
        let assessment = events
            .iter()
            .find(|e| e.event_type == EventType::InformationSufficiencyAssessment)
            .unwrap();
        assert!(
            assessment.payload["reason_codes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "no_fulltext_evidence")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 ──

    /// A fetched web page's evidence carries the MECHANICAL tier/weight in
    /// the committed ledger (authoritative 1.1 / default 1.0 / low_quality
    /// 0.7); external `[SOURCE]` declarations go through the same judge.
    #[tokio::test]
    async fn web_page_evidence_carries_mechanical_tier_and_weight() {
        let dir = test_dir();
        let journal = JournalRecorder::new(dir.clone());
        let host = TestHost {
            journal,
            tool_result: Some(ToolResult {
                output: "page content".to_string(),
                exit_code: Some(0),
                output_encoding: None,
                structured: None,
                ..Default::default()
            }),
        };
        let gateway: Arc<dyn ModelGateway> = Arc::new(FakeProvider::new(vec![
            // Main round: dispatch the external retrieval lane.
            ScriptedResponse::tool_calls(vec![ToolCall {
                name: "web_fetch".to_string(),
                arguments: serde_json::json!({"url": "https://example.com/dispatch"}),
                call_id: "call-d".to_string(),
            }]),
            // Subagent round: fetch a whitelist page and a low-quality page.
            ScriptedResponse::tool_calls(vec![
                ToolCall {
                    name: "web_fetch".to_string(),
                    arguments: serde_json::json!({"url": "https://www.gov.cn/policy/1"}),
                    call_id: "call-f1".to_string(),
                },
                ToolCall {
                    name: "web_fetch".to_string(),
                    arguments: serde_json::json!({"url": "https://blog.csdn.net/foo"}),
                    call_id: "call-f2".to_string(),
                },
            ]),
            ScriptedResponse::text("[SOURCE] https://zhihu.com/p/1\n检索完成"),
            ScriptedResponse::text("完成"),
            ScriptedResponse::text("完成"),
        ]));
        let controller = with_retrieval_enabled(AgentLoopController::with_gateway(gateway));
        controller
            .run_turn(&host, "查政策", "RUN-WT", MANIFEST, 0, None, None, None)
            .await
            .unwrap();

        let events = events(&dir);
        let commit = events
            .iter()
            .find(|e| e.event_type == EventType::RetrievalResultCommitted)
            .unwrap();
        let ledger = commit.payload["source_ledger"].as_array().unwrap();
        let gov = ledger
            .iter()
            .find(|e| e["source_url_or_ref"] == "https://www.gov.cn/policy/1")
            .unwrap();
        assert_eq!(gov["source_type"], "web_page");
        assert_eq!(gov["tier"], "authoritative");
        assert_eq!(gov["mechanical_weight"], 1.1);
        assert_eq!(gov["weight_reason"], "whitelist_suffix:gov.cn");
        let csdn = ledger
            .iter()
            .find(|e| e["source_url_or_ref"] == "https://blog.csdn.net/foo")
            .unwrap();
        assert_eq!(csdn["tier"], "low_quality");
        assert_eq!(csdn["mechanical_weight"], 0.7);
        assert_eq!(csdn["weight_reason"], "low_quality_platform:csdn.net");
        let zhihu = ledger
            .iter()
            .find(|e| e["source_url_or_ref"] == "https://zhihu.com/p/1")
            .unwrap();
        assert_eq!(zhihu["source_type"], "web_page");
        assert_eq!(zhihu["tier"], "low_quality");
        assert_eq!(zhihu["mechanical_weight"], 0.7);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C (2026-08-30): [SOURCE]
    /// 声明行 URL 规范化——URL+标题同写整行拆分为规范化 URL + 标题；
    /// 尾部标点剥离、默认端口/fragment 清理；非 URL 行原样保留。
    /// userinfo URL 被解析器拒绝时原样保留（不规范化、不丢弃声明行）。
    #[test]
    fn source_declaration_url_is_normalized() {
        // URL + 标题同行 → URL 规范化（host 小写、去默认端口、去 fragment、
        // 尾部中文句号剥离），标题为剩余文本。
        let (url, title) =
            split_source_declaration("https://Example.com:443/Paper#sec1 论文标题。");
        assert_eq!(url, "https://example.com/Paper");
        assert_eq!(title, "论文标题。");

        // 标题在前、URL 在后。
        let (url, title) = split_source_declaration("文档：https://zhihu.com/p/1");
        assert_eq!(url, "https://zhihu.com/p/1");
        assert_eq!(title, "文档：");

        // 无空格形态：标题用中文标点粘在 URL 后 → 在标点处拆分。
        let (url, title) = split_source_declaration("https://example.com/，标题");
        assert_eq!(url, "https://example.com/");
        assert_eq!(title, "标题");

        // 无空格形态 + 全角括号（S4 实机复验 2026-08-31：子代理声明行
        // 「URL（说明）」此前整串进 URL 被 percent-encode）→ 在「（」处拆分。
        let (url, title) = split_source_declaration(
            "https://huggingface.co/datasets/open-thoughts/OpenThoughts-114k（full 读取，数据集官方卡片）",
        );
        assert_eq!(
            url,
            "https://huggingface.co/datasets/open-thoughts/OpenThoughts-114k"
        );
        assert_eq!(title, "full 读取，数据集官方卡片）");

        // userinfo URL 被 ACAF 解析器拒绝 → 原样保留、不规范化。
        let (url, title) = split_source_declaration("https://user:pass@example.com/x");
        assert_eq!(url, "https://user:pass@example.com/x");
        assert_eq!(title, "https://user:pass@example.com/x");

        // 纯 URL 行 → 标题回退为规范化 URL。
        let (url, title) = split_source_declaration("https://blog.csdn.net/foo。");
        assert_eq!(url, "https://blog.csdn.net/foo");
        assert_eq!(title, "https://blog.csdn.net/foo");

        // 非 URL 行（内部 ref）原样保留；标题回退为整行原文（schema
        // source_title 非空约束，S4 实机复验 2026-08-31 对齐）。
        let (ref_, title) = split_source_declaration("docs/design.md");
        assert_eq!(ref_, "docs/design.md");
        assert_eq!(title, "docs/design.md");

        // 解析失败（scheme 非 http(s)）原样保留；标题回退为整行原文
        // （source_title 非空约束，S4 实机复验 2026-08-31 对齐）。
        let (url, title) = split_source_declaration("ftp://example.com/x");
        assert_eq!(url, "ftp://example.com/x");
        assert_eq!(title, "ftp://example.com/x");
    }
}
