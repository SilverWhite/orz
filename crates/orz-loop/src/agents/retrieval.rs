//! Retrieval subagents — internal (project docs) + external (web).
//!
//! GAP-SUBAGENT-RUNTIME (2026-08-10): the one-shot scripted model pass is
//! GONE — ADR-0010 §3.1 forbids the zero-tool, no-session, no-journal
//! special runtime. The subagent now implements `RoundAgent` and runs the
//! SAME shared loop as the main agent (`crate::agent_loop`), with its own
//! tool-round budget, journal events, write-domain gate and (later) its
//! orientation lane. The `[DOC]`/`[SOURCE]` line contract below is the
//! stable result-formation interface — real retrieval semantics (project
//! doc index, local_browser web retrieval) plug into these same fields
//! in a later slice (structured result schema is deferred by user
//! decision 2026-08-10).

use std::sync::Arc;

use crate::agent_loop::RoundAgent;
use crate::blackboard::{InternalRetSection, SharedBlackboard};
use crate::gateway::model::{
    ActivityClock, GatewayError, Message, ModelGateway, ModelRequest, ModelResponse,
};
use crate::host::ToolDef;

/// Which retrieval domain a subagent serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubagentRole {
    InternalRetrieval,
    ExternalRetrieval,
}

impl SubagentRole {
    /// v0.2 payload `target` / `agent_role` value (schema enum — never free text).
    pub fn as_str(&self) -> &'static str {
        match self {
            SubagentRole::InternalRetrieval => "internal_retrieval",
            SubagentRole::ExternalRetrieval => "external_retrieval",
        }
    }

    /// Blackboard section name for this role.
    pub fn section_name(&self) -> &'static str {
        match self {
            SubagentRole::InternalRetrieval => "internal_ret",
            SubagentRole::ExternalRetrieval => "external_ret",
        }
    }
}

/// A retrieval subagent with its own model gateway (scripted in tests).
#[derive(Clone)]
pub struct RetrievalSubagent {
    pub role: SubagentRole,
    pub gateway: Arc<dyn ModelGateway>,
}

impl RetrievalSubagent {
    pub fn new(role: SubagentRole, gateway: Arc<dyn ModelGateway>) -> Self {
        Self { role, gateway }
    }
}

/// GAP-SUBAGENT-RUNTIME (2026-08-10): the subagent's model round — same
/// shape as `MainAgent::run_round` (ADR-0010 §3.4.2: identical model
/// config; D-6: full 160K budget — a thinking subagent with a 1024 token
/// cap would spend everything on reasoning and die before producing
/// output, the Anthropic subagent 8K hard-cap lesson).
#[async_trait::async_trait]
impl RoundAgent for RetrievalSubagent {
    fn config_fingerprint(&self) -> String {
        self.gateway.config_fingerprint()
    }

    async fn run_round(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Vec<ToolDef>,
        max_tokens: u32,
        cancel: Option<&tokio_util::sync::CancellationToken>,
        heartbeat: Option<&ActivityClock>,
        on_chunk: &mut (dyn for<'a> FnMut(&'a str) + Send),
    ) -> Result<ModelResponse, GatewayError> {
        self.gateway
            .generate_stream(
                ModelRequest {
                    system: system.to_string(),
                    messages,
                    tools,
                    max_tokens,
                    thinking: None,
                },
                cancel,
                heartbeat,
                on_chunk,
            )
            .await
    }
}

/// GAP-RETRIEVAL-TOOLS (2026-08-10): the model-organized structured result
/// block — `[RESULT_JSON]{...}[/RESULT_JSON]`. Carries ONLY the
/// organized_response shape (sections/claims bound to ledger source_ids);
/// query_summary/source_ledger/filtering_log/raw_source_refs are built
/// MECHANICALLY by the controller from tool-call evidence (§3.3.3 —
/// provider-neutral: the model's self-description is never mechanical
/// fact). Malformed JSON or a non-object block returns `None` — the caller
/// falls back to the [DOC]/[SOURCE] prose contract with an explicit
/// visibility_degraded record (never a silent downgrade).
pub fn parse_retrieval_result_json(text: &str) -> Option<serde_json::Value> {
    const OPEN: &str = "[RESULT_JSON]";
    const CLOSE: &str = "[/RESULT_JSON]";
    let start = text.find(OPEN)?;
    let after = &text[start + OPEN.len()..];
    let end = after.find(CLOSE)?;
    let block = after[..end].trim();
    if !block.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(block).ok()?;
    if !value.is_object() {
        return None;
    }
    Some(value)
}

/// [DOC]/[SOURCE] line-contract parse (stable interface — the subagent's
/// result formation: `[DOC]`-prefixed lines → docs, `[SOURCE]`-prefixed
/// lines → sources; other lines are plain response prose).
pub fn parse_retrieval_text(text: &str) -> (Vec<String>, Vec<String>) {
    let mut docs: Vec<String> = Vec::new();
    let mut sources: Vec<String> = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("[DOC]") {
            docs.push(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("[SOURCE]") {
            sources.push(rest.trim().to_string());
        }
    }
    (docs, sources)
}

/// Write the parsed contract into the role's blackboard section
/// (single-writer discipline; never hold the write guard across an await).
pub fn write_section(
    role: SubagentRole,
    blackboard: &Arc<SharedBlackboard>,
    response: String,
    docs: Vec<String>,
    sources: Vec<String>,
) {
    let mut w = blackboard.write();
    match role {
        SubagentRole::InternalRetrieval => {
            let section: &mut InternalRetSection = &mut w.internal_ret;
            section.response = Some(response);
            section.project_docs.extend(docs);
            section.source_ledger.extend(sources);
        }
        SubagentRole::ExternalRetrieval => {
            let section = &mut w.external_ret;
            section.response = Some(response);
            section.web_sources.extend(sources);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::fake::FakeProvider;

    fn gateway(text: &str) -> Arc<dyn ModelGateway> {
        Arc::new(FakeProvider::from_texts(vec![text]))
    }

    #[test]
    fn parse_retrieval_result_json_extracts_organized_block() {
        let text = concat!(
            "检索完成\n",
            "[RESULT_JSON]",
            r#"{"sections":[{"section_title":"t","content":"c","source_ids":["SRC-1"],"claim_strength":"observed"}],"claims":[]}"#,
            "[/RESULT_JSON]\n",
        );
        let value = parse_retrieval_result_json(text).unwrap();
        assert_eq!(value["sections"][0]["source_ids"][0], "SRC-1");
    }

    #[test]
    fn parse_retrieval_result_json_rejects_malformed_blocks() {
        // No open marker.
        assert!(parse_retrieval_result_json("无块").is_none());
        // Open without close.
        assert!(parse_retrieval_result_json("[RESULT_JSON]{").is_none());
        // Not an object.
        assert!(parse_retrieval_result_json("[RESULT_JSON][1,2][/RESULT_JSON]").is_none());
        // Malformed JSON inside.
        assert!(parse_retrieval_result_json("[RESULT_JSON]{bad[/RESULT_JSON]").is_none());
    }

    #[test]
    fn parse_retrieval_text_splits_doc_and_source_lines() {
        let (docs, sources) =
            parse_retrieval_text("[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成");
        assert_eq!(docs, vec!["design.md", "gate.rs"]);
        assert_eq!(sources, vec!["docs/index"]);
    }

    #[test]
    fn parse_retrieval_text_handles_empty_and_unprefixed_lines() {
        let (docs, sources) = parse_retrieval_text("仅文字\n[DOC] a.md\n无前缀行\n[SOURCE] b");
        assert_eq!(docs, vec!["a.md"]);
        assert_eq!(sources, vec!["b"]);
        let (empty_docs, empty_sources) = parse_retrieval_text("");
        assert!(empty_docs.is_empty());
        assert!(empty_sources.is_empty());
    }

    #[tokio::test]
    async fn retrieval_subagent_writes_internal_ret_section() {
        let bb = Arc::new(SharedBlackboard::new());
        let subagent = RetrievalSubagent::new(SubagentRole::InternalRetrieval, gateway("x"));
        let (docs, sources) =
            parse_retrieval_text("[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成");
        let _ = subagent;
        write_section(
            SubagentRole::InternalRetrieval,
            &bb,
            "[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成".to_string(),
            docs,
            sources,
        );

        let r = bb.read();
        assert_eq!(r.internal_ret.project_docs, vec!["design.md", "gate.rs"]);
        assert_eq!(r.internal_ret.source_ledger, vec!["docs/index"]);
        assert!(
            r.internal_ret
                .response
                .as_deref()
                .unwrap()
                .contains("检索完成")
        );
        // External section untouched.
        assert!(r.external_ret.web_sources.is_empty());
    }

    #[tokio::test]
    async fn retrieval_subagent_writes_external_ret_section() {
        let bb = Arc::new(SharedBlackboard::new());
        let (docs, sources) =
            parse_retrieval_text("[SOURCE] https://example.com/paper\n网页检索完成");
        write_section(
            SubagentRole::ExternalRetrieval,
            &bb,
            "[SOURCE] https://example.com/paper\n网页检索完成".to_string(),
            docs,
            sources,
        );

        let r = bb.read();
        assert_eq!(
            r.external_ret.web_sources,
            vec!["https://example.com/paper"]
        );
        // Internal section untouched.
        assert!(r.internal_ret.project_docs.is_empty());
    }

    #[test]
    fn subagent_role_names() {
        assert_eq!(
            SubagentRole::InternalRetrieval.as_str(),
            "internal_retrieval"
        );
        assert_eq!(
            SubagentRole::ExternalRetrieval.as_str(),
            "external_retrieval"
        );
        assert_eq!(
            SubagentRole::InternalRetrieval.section_name(),
            "internal_ret"
        );
        assert_eq!(
            SubagentRole::ExternalRetrieval.section_name(),
            "external_ret"
        );
    }

    #[tokio::test]
    async fn subagent_run_round_uses_gateway_and_full_budget() {
        // The RoundAgent implementation forwards to the gateway with the
        // same request shape as the main agent (160K budget — D-6).
        let subagent = RetrievalSubagent::new(
            SubagentRole::ExternalRetrieval,
            Arc::new(FakeProvider::from_texts(vec!["网页检索完成"])),
        );
        let response = subagent
            .run_round(
                "system",
                Vec::new(),
                Vec::new(),
                crate::agent_loop::REQUEST_MAX_TOKENS,
                None,
                None,
                &mut |_| {},
            )
            .await
            .unwrap();
        assert!(response.text.as_deref().unwrap().contains("检索完成"));
    }
}
