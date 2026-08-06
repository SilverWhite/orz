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
///
/// D-1 (FIX_PLAN 2026-08-06): the citation rule lives here (the main-agent
/// prompt is the carrier — NOT a project doc / CLAUDE.md / index; those are
/// dev-directory documents unrelated to the binary). One rule + an inline
/// marker, no structured template (the main agent works alone — no helper
/// subagent, so the burden must be minimal). The rule blocks hallucinated
/// attributions ("参考自某处" claims without a locatable source — P7's
/// false "Python 移植" record is the direct precedent) and the inline
/// markers are mechanically checkable (grep `[来源:`).
pub const BASE_SYSTEM_PROMPT: &str = "你是 orz——保证优先的 CLI agent workbench。\
遵循注入的 assurance 上下文块执行任务；工具可用性由运行时声明，不得自行推断。\
\n引用纪律：凡基于外部依据、参考实现或内部文档的引用，必须在引用处附带内联标记 \
`[来源: 路径:行号]`；无法定位来源的内容不得引用——不得凭记忆声称『参考自某处』。\
内部文档引用用 文档ID §节/锚点 而非裸行号（行号会漂移）。";

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
        || content.starts_with(TOOL_POLICY_BREAKER_PREFIX)
        || content.starts_with(TOOL_ROUND_BUDGET_PREFIX)
}

/// D-8 (FIX_PLAN 2026-08-06): prefix for the mechanically injected tool-round
/// budget declarations (session budget + per-round remaining + exhaustion).
/// Counted as injected text — never stagnation input.
///
/// Deliberately matches the versioned marker form (`[TOOL_ROUND_BUDGET v0.1]`)
/// as well as the bare form — the previous constant ended in `]` and never
/// matched the versioned messages (2026-08-07 review F-04). The closing tag
/// `[/TOOL_ROUND_BUDGET]` does not match (starts with `[/`).
pub const TOOL_ROUND_BUDGET_PREFIX: &str = "[TOOL_ROUND_BUDGET";

/// IP2a denial-circuit-breaker message (D-3, FIX_PLAN 2026-08-06): injected
/// into the conversation after 3 consecutive policy denials in one run — the
/// model has been retrying a refused tool (polyglot probe P3: `web_search`×4
/// burned a third of the round budget). It tells the model to switch
/// strategy, names the refused tool, and is counted as injected text (never
/// stagnation input). The total-denial ceiling message is a stronger variant.
pub const TOOL_POLICY_BREAKER_PREFIX: &str = "[TOOL_POLICY_BREAKER]";

pub fn tool_policy_breaker_block(tool_name: &str, consecutive: u32) -> String {
    format!(
        "{TOOL_POLICY_BREAKER_PREFIX} v0.1\n\
        Consecutive tool calls have been refused by the session permission policy \
        ({consecutive} in a row, last: '{tool_name}'). The refused tool is NOT \
        available under the current policy — do not retry it. Switch strategy: \
        use only the tools declared as available, or state that the task cannot \
        be completed under the current policy.\n\
        [/TOOL_POLICY_BREAKER]"
    )
}

pub fn tool_policy_ceiling_block(total: u32) -> String {
    format!(
        "{TOOL_POLICY_BREAKER_PREFIX} v0.1 CEILING\n\
        The total number of refused tool calls this run has reached {total} \
        (conservative ceiling). No further refused-tool retries are productive — \
        end the attempt or switch to an available tool immediately.\n\
        [/TOOL_POLICY_BREAKER]"
    )
}

/// D-8 (FIX_PLAN 2026-08-06): session-level budget declaration — injected
/// into the system prompt once per run (BUDGET + REMAINING initial value).
/// The model does not guess or drift the remaining count.
pub fn tool_round_budget_session_block(budget: u32, remaining: u32) -> String {
    format!(
        "{TOOL_ROUND_BUDGET_PREFIX} v0.1]\n\
         BUDGET: {budget} tool rounds per turn\n\
         REMAINING: {remaining}\n\
         After each tool round the controller reports the updated \
         remaining count. Finish your work within the budget; if it \
         is exhausted the run ends with a partial result.\n\
         [/TOOL_ROUND_BUDGET]"
    )
}

/// D-8: per-round mechanical re-declaration of the remaining budget.
pub fn tool_round_budget_remaining_block(remaining: u32) -> String {
    format!(
        "{TOOL_ROUND_BUDGET_PREFIX} v0.1] REMAINING: {remaining} tool rounds left\n\
         [/TOOL_ROUND_BUDGET]"
    )
}

/// D-8: budget-exhaustion notice — the run ends after this round with a
/// partial result; the model must not call more tools.
pub fn tool_round_budget_exhaustion_block(budget: u32) -> String {
    format!(
        "{TOOL_ROUND_BUDGET_PREFIX} v0.1] The {budget}-round budget is \
         exhausted — the run is ending. Report your best partial \
         result now; do NOT call more tools.\n\
         [/TOOL_ROUND_BUDGET]"
    )
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
    fn base_system_prompt_carries_d1_citation_rule() {
        // D-1 (FIX_PLAN 2026-08-06): the citation rule lives in the main
        // agent prompt (the binary's carrier) — inline marker `[来源: 路径:行号]`,
        // no bare line numbers for internal docs (they drift).
        assert!(
            BASE_SYSTEM_PROMPT.contains("[来源: 路径:行号]"),
            "citation marker in the main-agent prompt"
        );
        assert!(
            BASE_SYSTEM_PROMPT.contains("不得凭记忆声称"),
            "no-memory-citation rule present"
        );
        assert!(
            BASE_SYSTEM_PROMPT.contains("文档ID §节/锚点"),
            "internal docs cite by section, not line number"
        );
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
        // D-3 breaker + D-8 budget blocks (FIX_PLAN 2026-08-06) — budget
        // blocks use the versioned marker form; the prefix must match it
        // (2026-08-07 review F-04: the constant previously ended in `]` and
        // never matched the `[TOOL_ROUND_BUDGET v0.1]` messages).
        assert!(is_injected_block_text(&tool_policy_breaker_block("web_search", 3)));
        assert!(is_injected_block_text(&tool_policy_ceiling_block(10)));
        assert!(is_injected_block_text(&tool_round_budget_session_block(40, 40)));
        assert!(is_injected_block_text(&tool_round_budget_remaining_block(38)));
        assert!(is_injected_block_text(&tool_round_budget_exhaustion_block(40)));
        // Leading/trailing whitespace tolerated.
        assert!(is_injected_block_text(&format!(
            "  {INFO_SUFFICIENCY_BLOCK}\n"
        )));
        // Ordinary model/user text must never match.
        assert!(!is_injected_block_text("完成"));
        assert!(!is_injected_block_text("[INFO_SUFFICIENCY v0.1] 部分拷贝"));
        // A closing tag alone must never match (starts with `[/`).
        assert!(!is_injected_block_text("[/TOOL_ROUND_BUDGET]"));
        assert!(!is_injected_block_text(""));
    }
}
