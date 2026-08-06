//! PromptBuilder — assembles system prompt + assurance context blocks.
//!
//! IP2a: TOOL_AVAILABILITY block injection (format aligned with Python
//! `_build_context_block`).
//! IP2b: ORIENTATION_CHECKPOINT block (injected per-turn by the controller via
//! the orientation monitor; the builder provides the composition helper).
//! IP2c: INFO_SUFFICIENCY + RETRIEVAL_COMPLETION_CHECK + COUNTEREXAMPLE_GATE
//! blocks (§4.6 wiring, Phase 3). Inquiries are injected as `Role::User`
//! messages by the controller (they are runtime questions to the model, not
//! system context); the constants live here per design §4.6.6.

/// Base system prompt — runtime-neutral orientation.
pub const BASE_SYSTEM_PROMPT: &str = "你是 orz——保证优先的 CLI agent workbench。\
遵循注入的 assurance 上下文块执行任务；工具可用性由运行时声明，不得自行推断。";

/// Neutral inquiry block — IP2c, fired after a retrieval round completes when
/// any of the 4 判定点 crosses its threshold (§4.6.5, verbatim).
pub const INFO_SUFFICIENCY_BLOCK: &str = "[INFO_SUFFICIENCY v0.1]\n\
本轮检索已完成。请确认：\n\
是否已获得完成当前主任务所需的内容？\n\
请回答 yes / no / uncertain，并附简短理由。\n\
若为 no 或 uncertain，只列出还需要的内容类型。\n\
[/INFO_SUFFICIENCY]";

/// Subagent close completion check block — one-shot per subagent close, no
/// accumulation (§4.6.1; Python `RETRIEVAL_COMPLETION_CHECK v0.1` verbatim,
/// `assurance/retrieval_subagent.py` L474-482).
pub const RETRIEVAL_COMPLETION_CHECK_BLOCK: &str = "[RETRIEVAL_COMPLETION_CHECK v0.1]\n\
子代理检索任务已完成。关闭前请确认：\n\
\n\
是否已经获得完成当前主任务所需的内容？\n\
\n\
请回答 yes / no / uncertain，并附简短理由。\n\
若为 no 或 uncertain，只列出还需要的内容类型（不自动扩展为新子代理或无限补检索）。\n\
[/RETRIEVAL_COMPLETION_CHECK]";

/// Counterexample gate block — 正式答案输出前, fires once per run and the
/// block explicitly tells the model it appears only once (§4.6.5 verbatim).
pub const COUNTEREXAMPLE_GATE_BLOCK: &str = "[COUNTEREXAMPLE_GATE v0.1]\n\
最终回答即将输出。请对即将输出的结论做最后一次反例自查：\n\
1. 是否存在未验证的前提？\n\
2. 是否存在可推翻结论的已知证据？\n\
3. 结论强度是否超出证据支持？\n\
注意：本反例询问仅出现一次，请在最终回答前完成全部反例自查。\n\
[/COUNTEREXAMPLE_GATE]";

/// Counterexample gate block — plan 写入前 variant: same block minus the
/// "仅出现一次" note line (plan-write is part of the plan approval gate chain
/// and is NOT subject to once-only, §4.6.2 裁决 3).
pub const COUNTEREXAMPLE_GATE_PLAN_BLOCK: &str = "[COUNTEREXAMPLE_GATE v0.1]\n\
最终回答即将输出。请对即将输出的结论做最后一次反例自查：\n\
1. 是否存在未验证的前提？\n\
2. 是否存在可推翻结论的已知证据？\n\
3. 结论强度是否超出证据支持？\n\
[/COUNTEREXAMPLE_GATE]";

/// Whether `content` is exactly one of the runtime-injected inquiry blocks.
/// The controller filters these out of stagnation inputs (D7) — fixed injected
/// text is not model output and repeated blocks would pollute ngram stats.
pub fn is_injected_block_text(content: &str) -> bool {
    let content = content.trim();
    content == INFO_SUFFICIENCY_BLOCK
        || content == RETRIEVAL_COMPLETION_CHECK_BLOCK
        || content == COUNTEREXAMPLE_GATE_BLOCK
        || content == COUNTEREXAMPLE_GATE_PLAN_BLOCK
}

/// Tool availability context block format — aligned with Python
/// `_build_context_block` (`[TOOL_AVAILABILITY v0.1]` ... `[/TOOL_AVAILABILITY]`).
pub fn build_tool_availability_block(
    available: &[String],
    unavailable: &[String],
    degraded: &[String],
    unprobed: &[String],
) -> String {
    // Sorted per category, matching Python's `sorted(...)` joins.
    let mut available = available.to_vec();
    let mut unavailable = unavailable.to_vec();
    let mut degraded = degraded.to_vec();
    let mut unprobed = unprobed.to_vec();
    available.sort();
    unavailable.sort();
    degraded.sort();
    unprobed.sort();

    let mut lines = vec!["[TOOL_AVAILABILITY v0.1]".to_string()];
    if !available.is_empty() {
        lines.push(format!("AVAILABLE: {}", available.join(", ")));
    }
    if !unavailable.is_empty() {
        lines.push(format!("UNAVAILABLE: {}", unavailable.join(", ")));
    }
    if !degraded.is_empty() {
        lines.push(format!("DEGRADED: {}", degraded.join(", ")));
    }
    if !unprobed.is_empty() {
        lines.push(format!("UNPROBED: {}", unprobed.join(", ")));
    }
    lines.push(String::new());
    lines.push("GATE: runtime_probe_authoritative".to_string());
    lines.push("STATUS: descriptive_projection_only".to_string());
    lines.push(
        "ENFORCEMENT: tool calls, provider errors, and capability claims are \
checked by runtime gates and observers outside this text block."
            .to_string(),
    );
    lines.push("[/TOOL_AVAILABILITY]".to_string());
    lines.join("\n")
}

/// Builds prompts with injected assurance context.
#[derive(Debug, Clone, Default)]
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        PromptBuilder
    }

    /// IP2a: assemble the system prompt with the tool availability block
    /// injected (when present) above the base instructions.
    pub fn build_system_prompt(&self, tool_availability_block: Option<&str>) -> String {
        match tool_availability_block {
            Some(block) => format!("{block}\n\n{BASE_SYSTEM_PROMPT}"),
            None => BASE_SYSTEM_PROMPT.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_availability_block_format_matches_python() {
        let block = build_tool_availability_block(
            &["read_file".to_string(), "bash".to_string()],
            &[],
            &["web_search".to_string()],
            &[],
        );
        assert!(block.starts_with("[TOOL_AVAILABILITY v0.1]"));
        assert!(block.ends_with("[/TOOL_AVAILABILITY]"));
        assert!(block.contains("AVAILABLE: bash, read_file"));
        assert!(block.contains("DEGRADED: web_search"));
        assert!(block.contains("GATE: runtime_probe_authoritative"));
        assert!(!block.contains("UNAVAILABLE:"));
    }

    #[test]
    fn system_prompt_injects_block() {
        let builder = PromptBuilder::new();
        let block = build_tool_availability_block(&[], &[], &[], &["grep".to_string()]);
        let prompt = builder.build_system_prompt(Some(&block));
        assert!(prompt.contains("[TOOL_AVAILABILITY v0.1]"));
        assert!(prompt.contains(BASE_SYSTEM_PROMPT));
        let bare = builder.build_system_prompt(None);
        assert_eq!(bare, BASE_SYSTEM_PROMPT);
    }

    #[test]
    fn inquiry_blocks_match_design_doc_verbatim() {
        // INFO_SUFFICIENCY — design doc §4.6.5 verbatim.
        assert!(INFO_SUFFICIENCY_BLOCK.starts_with("[INFO_SUFFICIENCY v0.1]"));
        assert!(INFO_SUFFICIENCY_BLOCK.contains("本轮检索已完成。请确认："));
        assert!(INFO_SUFFICIENCY_BLOCK.contains("是否已获得完成当前主任务所需的内容？"));
        assert!(INFO_SUFFICIENCY_BLOCK.contains("请回答 yes / no / uncertain，并附简短理由。"));
        assert!(INFO_SUFFICIENCY_BLOCK.ends_with("[/INFO_SUFFICIENCY]"));

        // RETRIEVAL_COMPLETION_CHECK — Python precedent verbatim.
        assert!(RETRIEVAL_COMPLETION_CHECK_BLOCK.starts_with("[RETRIEVAL_COMPLETION_CHECK v0.1]"));
        assert!(RETRIEVAL_COMPLETION_CHECK_BLOCK.contains("子代理检索任务已完成。关闭前请确认："));
        assert!(
            RETRIEVAL_COMPLETION_CHECK_BLOCK.contains("是否已经获得完成当前主任务所需的内容？")
        );
        assert!(RETRIEVAL_COMPLETION_CHECK_BLOCK.contains("不自动扩展为新子代理或无限补检索"));
        assert!(RETRIEVAL_COMPLETION_CHECK_BLOCK.ends_with("[/RETRIEVAL_COMPLETION_CHECK]"));

        // COUNTEREXAMPLE_GATE — final-answer variant carries the once-only note.
        assert!(COUNTEREXAMPLE_GATE_BLOCK.starts_with("[COUNTEREXAMPLE_GATE v0.1]"));
        assert!(
            COUNTEREXAMPLE_GATE_BLOCK
                .contains("最终回答即将输出。请对即将输出的结论做最后一次反例自查：")
        );
        assert!(COUNTEREXAMPLE_GATE_BLOCK.contains("1. 是否存在未验证的前提？"));
        assert!(COUNTEREXAMPLE_GATE_BLOCK.contains("2. 是否存在可推翻结论的已知证据？"));
        assert!(COUNTEREXAMPLE_GATE_BLOCK.contains("3. 结论强度是否超出证据支持？"));
        assert!(COUNTEREXAMPLE_GATE_BLOCK.contains("本反例询问仅出现一次"));
        assert!(COUNTEREXAMPLE_GATE_BLOCK.ends_with("[/COUNTEREXAMPLE_GATE]"));

        // Plan variant: identical to the final-answer variant except the
        // once-only note line (plan-write is not subject to once-only).
        assert_eq!(
            COUNTEREXAMPLE_GATE_PLAN_BLOCK,
            COUNTEREXAMPLE_GATE_BLOCK.replace(
                "注意：本反例询问仅出现一次，请在最终回答前完成全部反例自查。\n",
                ""
            )
        );
    }

    #[test]
    fn is_injected_block_text_detects_blocks() {
        assert!(is_injected_block_text(INFO_SUFFICIENCY_BLOCK));
        assert!(is_injected_block_text(RETRIEVAL_COMPLETION_CHECK_BLOCK));
        assert!(is_injected_block_text(COUNTEREXAMPLE_GATE_BLOCK));
        assert!(is_injected_block_text(COUNTEREXAMPLE_GATE_PLAN_BLOCK));
        // Leading/trailing whitespace tolerated.
        assert!(is_injected_block_text(&format!(
            "  {INFO_SUFFICIENCY_BLOCK}\n"
        )));
        // Ordinary model/user text must never match.
        assert!(!is_injected_block_text("完成"));
        assert!(!is_injected_block_text("[INFO_SUFFICIENCY v0.1] 部分拷贝"));
        assert!(!is_injected_block_text(""));
    }
}
