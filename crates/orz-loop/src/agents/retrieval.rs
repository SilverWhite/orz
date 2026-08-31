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
//! in a later slice. GAP-RETRIEVAL-STRUCTURED-RESULT 方向 C
//! (2026-08-30 用户裁决): the `[RESULT_JSON]` organized block is
//! DELETED — the mechanical ledger is the single result-formation track
//! (see `retrieval/evidence.rs`); the line contract is the only
//! model-facing result interface.

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
/// config; D-6: full 256K budget — a thinking subagent with a 1024 token
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

/// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30, TODO P0-0k 第一批
/// 第 3 项)：[DOC]/[SOURCE] 回传预算——声明行机械有界化。
/// - 行数上限 `RETRIEVAL_RESULT_LINE_CAP`（默认 16 条）+ 字节上限
///   `RETRIEVAL_RESULT_BYTES_CAP`（默认 8K，逐条装填）；
/// - 截断时返回文本追加结构化标注（总数/保留数/全文指针），标注行
///   不带 `[DOC]`/`[SOURCE]` 前缀（不会被再解析为声明行）；
/// - 证据以 tool-call evidence ledger 为准，声明行仅元数据级，截断
///   不丢证据；全文由调用方保留（blackboard 分区 response /
///   retrieval-results 存档）。
/// 0k 审查处理 (P3-6) 登记口径：8K 字节预算只约束声明行（逐条装填，
/// 设计语义「声明行字节上限」）；prose 叙述与截断标注行不计入——二者
/// 是子代理自由文本与机械说明，原样保留（blackboard 全文通道不受影响，
/// inline 通道的有界化对象是结构化声明行）。
pub const RETRIEVAL_RESULT_LINE_CAP: usize = 16;
pub const RETRIEVAL_RESULT_BYTES_CAP: usize = 8 * 1024;

#[derive(Debug, Clone)]
pub struct BoundedRetrievalResult {
    /// 有界输出文本：原 prose + 保留的声明行 +（截断时）结构化标注。
    pub output: String,
    pub docs: Vec<String>,
    pub sources: Vec<String>,
    pub truncated: bool,
    pub total_declarations: usize,
}

pub fn bound_retrieval_result(
    output: &str,
    docs: &[String],
    sources: &[String],
    section_name: &str,
) -> BoundedRetrievalResult {
    let mut kept_docs: Vec<String> = Vec::new();
    let mut kept_sources: Vec<String> = Vec::new();
    let mut bytes = 0usize;
    let mut total_declarations = 0usize;
    let mut truncated = false;
    // 保持原声明顺序的近似：先 [DOC] 后 [SOURCE]（parse 已分列，原交错
    // 顺序在分列时即丢失——此处只保分组序）。
    let mut push = |item: &str, is_doc: bool| {
        total_declarations += 1;
        if kept_docs.len() + kept_sources.len() >= RETRIEVAL_RESULT_LINE_CAP
            || bytes + item.len() > RETRIEVAL_RESULT_BYTES_CAP
        {
            truncated = true;
            return;
        }
        bytes += item.len();
        if is_doc {
            kept_docs.push(item.to_string());
        } else {
            kept_sources.push(item.to_string());
        }
    };
    for item in docs {
        push(item, true);
    }
    for item in sources {
        push(item, false);
    }
    let kept = kept_docs.len() + kept_sources.len();
    // 有界输出 = 原 prose（去声明行）+ 保留声明行 +（截断时）标注。
    // 声明行统一收尾排列（原交错在 parse 分列时已丢失，此处只保
    // prose 顺序与保留声明内容）。
    let mut bounded_output: String = output
        .lines()
        .filter(|line| !line.starts_with("[DOC]") && !line.starts_with("[SOURCE]"))
        .collect::<Vec<_>>()
        .join("\n");
    if !bounded_output.is_empty() {
        bounded_output.push('\n');
    }
    for doc in &kept_docs {
        bounded_output.push_str(&format!("[DOC] {doc}\n"));
    }
    for src in &kept_sources {
        bounded_output.push_str(&format!("[SOURCE] {src}\n"));
    }
    if truncated {
        bounded_output.push_str(&format!(
            "[retrieval-result bounded] 声明 {total_declarations} 条，\
             保留 {kept} 条（行数 ≤{}/字节 ≤{}）；\
             全文见 blackboard {} 分区与 retrieval-results 存档",
            RETRIEVAL_RESULT_LINE_CAP, RETRIEVAL_RESULT_BYTES_CAP, section_name,
        ));
    }
    BoundedRetrievalResult {
        output: bounded_output,
        docs: kept_docs,
        sources: kept_sources,
        truncated,
        total_declarations,
    }
}

/// Write the parsed contract into the role's blackboard section
/// (single-writer discipline; never hold the write guard across an await).
///
/// THIN-HARNESS-REDESIGN R2a 审查处理 (2026-08-27, P2-2/P2-3)：每次派发
/// **全量覆盖**（不再 extend 累积）——分区永远只代表最近一次激活的结果，
/// 与「每次调用即闭环」生命周期一致，指针摘要的 N 来源/M 结论计数与
/// 分区内容一一对应；`ledger` 为结构化 ledger 投影（source_id + 标题/URL，
/// 由派发路径从 committed payload 构建），internal 与 external 分区
/// 均写（此前 external.source_ledger 从未被填充，R2a 渲染层会恒显
/// (none)）。
pub fn write_section(
    role: SubagentRole,
    blackboard: &Arc<SharedBlackboard>,
    response: String,
    docs: Vec<String>,
    sources: Vec<String>,
    ledger: Vec<String>,
) {
    let mut w = blackboard.write();
    match role {
        SubagentRole::InternalRetrieval => {
            let section: &mut InternalRetSection = &mut w.internal_ret;
            section.response = Some(response);
            section.project_docs = docs;
            section.source_ledger = ledger;
            w.bump_retrieval("internal_ret");
        }
        SubagentRole::ExternalRetrieval => {
            let section = &mut w.external_ret;
            section.response = Some(response);
            section.web_sources = sources;
            section.source_ledger = ledger;
            w.bump_retrieval("external_ret");
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

    /// RETRIEVAL-ORCHESTRATION-MECHANICAL 0k (2026-08-30)：声明行机械
    /// 有界化——行数上限截断 + 字节上限截断 + 结构化标注 + 不截断时
    /// 原样保留。
    #[test]
    fn bound_retrieval_result_truncates_declarations_with_marker() {
        // 超过行数上限（16）→ 保留前 16 条，标注总数。
        let docs: Vec<String> = (0..20).map(|i| format!("doc-{i}.md")).collect();
        let bounded = bound_retrieval_result("检索完成", &docs, &[], "external_ret");
        assert!(bounded.truncated);
        assert_eq!(bounded.docs.len(), RETRIEVAL_RESULT_LINE_CAP);
        assert_eq!(bounded.total_declarations, 20);
        assert!(bounded.output.contains("检索完成"));
        assert!(bounded.output.contains("[retrieval-result bounded]"));
        assert!(bounded.output.contains("20 条"));
        assert!(bounded.output.contains("external_ret"));
        // 标注行不得带 [DOC]/[SOURCE] 前缀（不会污染再解析）。
        let (redocs, resources) = parse_retrieval_text(&bounded.output);
        assert_eq!(redocs.len(), RETRIEVAL_RESULT_LINE_CAP);
        assert!(resources.is_empty());
    }

    #[test]
    fn bound_retrieval_result_byte_cap_and_untouched_path() {
        // 字节上限：单条超限即截断。
        let huge = "x".repeat(RETRIEVAL_RESULT_BYTES_CAP + 1);
        let bounded = bound_retrieval_result("p", &[huge], &[], "internal_ret");
        assert!(bounded.truncated);
        assert!(bounded.docs.is_empty());
        // 未超限：原样保留、无标注。
        let docs = vec!["a.md".to_string(), "b.md".to_string()];
        let sources = vec!["https://example.com/x".to_string()];
        let bounded = bound_retrieval_result("检索完成", &docs, &sources, "external_ret");
        assert!(!bounded.truncated);
        assert_eq!(bounded.docs, docs);
        assert_eq!(bounded.sources, sources);
        assert_eq!(
            bounded.output,
            "检索完成\n[DOC] a.md\n[DOC] b.md\n[SOURCE] https://example.com/x\n"
        );
        assert!(!bounded.output.contains("[retrieval-result bounded]"));
    }

    #[tokio::test]
    async fn retrieval_subagent_writes_internal_ret_section() {
        let bb = Arc::new(SharedBlackboard::new());
        let subagent = RetrievalSubagent::new(SubagentRole::InternalRetrieval, gateway("x"));
        let (docs, sources) =
            parse_retrieval_text("[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成");
        let _ = subagent;
        let ledger = sources
            .iter()
            .map(|s| format!("SRC-001 {s}"))
            .collect::<Vec<_>>();
        write_section(
            SubagentRole::InternalRetrieval,
            &bb,
            "[DOC] design.md\n[DOC] gate.rs\n[SOURCE] docs/index\n检索完成".to_string(),
            docs,
            sources,
            ledger,
        );

        let r = bb.read();
        assert_eq!(r.internal_ret.project_docs, vec!["design.md", "gate.rs"]);
        assert_eq!(r.internal_ret.source_ledger, vec!["SRC-001 docs/index"]);
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
        let ledger = vec!["SRC-001 https://example.com/paper".to_string()];
        write_section(
            SubagentRole::ExternalRetrieval,
            &bb,
            "[SOURCE] https://example.com/paper\n网页检索完成".to_string(),
            docs,
            sources,
            ledger.clone(),
        );

        let r = bb.read();
        assert_eq!(
            r.external_ret.web_sources,
            vec!["https://example.com/paper"]
        );
        // P2-2: external ledger 由派发路径的结构化投影填充（不再恒空）。
        assert_eq!(r.external_ret.source_ledger, ledger);
        // Internal section untouched.
        assert!(r.internal_ret.project_docs.is_empty());
    }

    /// THIN-HARNESS-REDESIGN R2a 审查处理 (P2-3)：每次派发全量覆盖——
    /// 第二次写入替换（而非累积）上次的 response / entries / ledger。
    #[tokio::test]
    async fn write_section_overwrites_previous_dispatch() {
        let bb = Arc::new(SharedBlackboard::new());
        write_section(
            SubagentRole::ExternalRetrieval,
            &bb,
            "第一轮检索完成".to_string(),
            Vec::new(),
            vec!["https://old.example/a".to_string()],
            vec!["SRC-001 https://old.example/a".to_string()],
        );
        write_section(
            SubagentRole::ExternalRetrieval,
            &bb,
            "第二轮检索完成".to_string(),
            Vec::new(),
            vec!["https://new.example/b".to_string()],
            vec!["SRC-002 https://new.example/b".to_string()],
        );
        let r = bb.read();
        assert_eq!(
            r.external_ret.web_sources,
            vec!["https://new.example/b"],
            "entries must be replaced, not accumulated"
        );
        assert_eq!(
            r.external_ret.source_ledger,
            vec!["SRC-002 https://new.example/b"],
            "ledger must be replaced, not accumulated"
        );
        assert_eq!(r.external_ret.response.as_deref(), Some("第二轮检索完成"));
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
        // same request shape as the main agent (256K budget — D-6,
        // OUTPUT-BUDGET-RESTORE-AND-STALL-GUARD 2026-08-20 §14.35).
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
