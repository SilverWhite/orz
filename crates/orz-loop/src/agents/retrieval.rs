//! Retrieval subagents — internal (project docs) + external (web).
//!
//! Phase 2 scope: structure + scheduling ready. Each subagent runs one
//! scripted model pass and writes its blackboard section. Real retrieval
//! semantics (project doc index, local_browser web retrieval) are deferred —
//! the write contract below is the stable interface future semantics plug into.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::blackboard::{InternalRetSection, SharedBlackboard};
use crate::gateway::model::{GatewayError, Message, ModelGateway, ModelResponse, Role};

/// Which retrieval domain a subagent serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubagentRole {
    InternalRetrieval,
    ExternalRetrieval,
}

/// Specification for a subagent invocation.
#[derive(Debug, Clone)]
pub struct SubagentSpec {
    pub role: SubagentRole,
    pub goal: String,
    pub budget_turns: u32,
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

    /// Blackboard section name for this role.
    pub fn section_name(&self) -> &'static str {
        match self.role {
            SubagentRole::InternalRetrieval => "internal_ret",
            SubagentRole::ExternalRetrieval => "external_ret",
        }
    }

    /// Run one retrieval pass and write results into the blackboard section.
    ///
    /// Scripted in Phase 2 (FakeProvider drives the response). The write
    /// contract: response text → `response`; `[DOC]`-prefixed lines →
    /// `project_docs`; `[SOURCE]`-prefixed lines → `source_ledger` (internal)
    /// or `web_sources` (external). Real retrieval semantics will populate
    /// these same fields later.
    ///
    /// `completion_check_block` (Phase 3 §4.6): the subagent-close completion
    /// check is injected into the request when the caller is about to close
    /// the subagent; the free-form response carries the answer (parsed as
    /// evidence by the caller, never structurally enforced).
    pub async fn run_retrieval(
        &self,
        blackboard: &Arc<SharedBlackboard>,
        spec: &SubagentSpec,
        completion_check_block: Option<&str>,
        cancel: Option<&CancellationToken>,
        heartbeat: Option<&crate::gateway::model::ActivityClock>,
    ) -> Result<ModelResponse, GatewayError> {
        let messages = completion_check_block
            .map(|block| {
                vec![Message {
                    role: Role::User,
                    content: block.to_string(),
                    tool_call_id: None,
                    tool_calls: Vec::new(),
                    reasoning_content: None,
                }]
            })
            .unwrap_or_default();
        let response = self
            .gateway
            .generate_stream(
                crate::gateway::model::ModelRequest {
                    system: format!(
                        "Retrieval subagent ({role}). Goal: {goal}\n\
                         Citation rule (D-1, FIX_PLAN 2026-08-06): any claim based on \
                         external evidence, a reference implementation, or internal \
                         docs must carry an inline `[来源: 路径:行号]` marker at the \
                         citing site; content without a locatable source must not be \
                         cited — never claim '参考自某处' from memory. Internal docs \
                         cite as 文档ID §节/锚点, not bare line numbers (they drift).",
                        role = self.section_name(),
                        goal = spec.goal,
                    ),
                    messages,
                    tools: Vec::new(),
                    // D-6 (FIX_PLAN 2026-08-06): retrieval subagents get the
                    // full 160K budget too — a thinking subagent with a 1024
                    // token cap would spend everything on reasoning and die
                    // before producing output (the Anthropic subagent 8K
                    // hard-cap lesson, FIX_PLAN §3).
                    max_tokens: 160_000,
                    thinking: None,
                },
                cancel,
                heartbeat,
                // F-03 (2026-08-07 review): subagents went streaming — the
                // idle watchdog / total budget / cancel wiring live on the
                // streaming path (a non-streaming 10min wall clock could
                // falsely kill a legitimately slow 160K thinking round, and
                // Ctrl+C mid-retrieval had no effect). Subagent text has no
                // live consumer, so the deltas are discarded.
                &mut |_| {},
            )
            .await?;

        let text = response.text.clone().unwrap_or_default();
        let mut docs: Vec<String> = Vec::new();
        let mut sources: Vec<String> = Vec::new();
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("[DOC]") {
                docs.push(rest.trim().to_string());
            } else if let Some(rest) = line.strip_prefix("[SOURCE]") {
                sources.push(rest.trim().to_string());
            }
        }

        // Scope write to this section only (single-writer discipline);
        // never hold the write guard across an await.
        {
            let mut w = blackboard.write();
            match self.role {
                SubagentRole::InternalRetrieval => {
                    let section: &mut InternalRetSection = &mut w.internal_ret;
                    section.response = Some(text);
                    section.project_docs.extend(docs);
                    section.source_ledger.extend(sources);
                }
                SubagentRole::ExternalRetrieval => {
                    let section = &mut w.external_ret;
                    section.response = Some(text);
                    section.web_sources.extend(sources);
                }
            }
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::fake::FakeProvider;

    fn gateway(text: &str) -> Arc<dyn ModelGateway> {
        Arc::new(FakeProvider::from_texts(vec![text]))
    }

    #[tokio::test]
    async fn retrieval_subagent_writes_internal_ret_section() {
        let bb = Arc::new(SharedBlackboard::new());
        let subagent = RetrievalSubagent::new(
            SubagentRole::InternalRetrieval,
            gateway("[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成"),
        );
        let spec = SubagentSpec {
            role: SubagentRole::InternalRetrieval,
            goal: "找到设计文档".to_string(),
            budget_turns: 1,
        };
        subagent.run_retrieval(&bb, &spec, None, None, None).await.unwrap();

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
        let subagent = RetrievalSubagent::new(
            SubagentRole::ExternalRetrieval,
            gateway("[SOURCE] https://example.com/paper\n网页检索完成"),
        );
        let spec = SubagentSpec {
            role: SubagentRole::ExternalRetrieval,
            goal: "检索论文".to_string(),
            budget_turns: 1,
        };
        subagent.run_retrieval(&bb, &spec, None, None, None).await.unwrap();

        let r = bb.read();
        assert_eq!(
            r.external_ret.web_sources,
            vec!["https://example.com/paper"]
        );
        // Internal section untouched.
        assert!(r.internal_ret.project_docs.is_empty());
    }

    #[tokio::test]
    async fn subagent_section_name() {
        assert_eq!(
            RetrievalSubagent::new(SubagentRole::InternalRetrieval, gateway("x")).section_name(),
            "internal_ret"
        );
        assert_eq!(
            RetrievalSubagent::new(SubagentRole::ExternalRetrieval, gateway("x")).section_name(),
            "external_ret"
        );
    }

    #[tokio::test]
    async fn completion_check_block_is_injected_into_request() {
        // §4.6.1: the close-time completion check rides in the subagent's
        // request messages; the [DOC]/[SOURCE] parse stays on the response
        // side. FakeProvider retains requests, so the injection is assertable.
        let fake = Arc::new(FakeProvider::from_texts(vec![
            "[DOC] a.md\nyes，已获得全部内容",
        ]));
        let gateway: Arc<dyn ModelGateway> = fake.clone();
        let bb = Arc::new(SharedBlackboard::new());
        let subagent = RetrievalSubagent::new(SubagentRole::InternalRetrieval, gateway.clone());
        let spec = SubagentSpec {
            role: SubagentRole::InternalRetrieval,
            goal: "找文档".to_string(),
            budget_turns: 1,
        };
        subagent
            .run_retrieval(
                &bb,
                &spec,
                Some(crate::prompt::RETRIEVAL_COMPLETION_CHECK_BLOCK),
                None,
                None,
            )
            .await
            .unwrap();

        let requests = fake.received_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].messages.len(), 1);
        assert_eq!(
            requests[0].messages[0].content,
            crate::prompt::RETRIEVAL_COMPLETION_CHECK_BLOCK
        );
    }
}
