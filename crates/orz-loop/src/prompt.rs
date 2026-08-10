//! PromptBuilder — assembles system prompt + assurance context blocks.
//!
//! IP2a: TOOL_AVAILABILITY block injection (format aligned with Python
//! `_build_context_block`).
//! IP2c: COUNTEREXAMPLE_GATE + ORIENTATION blocks (ADR-0010 §4.1 six-mechanism
//! split, Phase C 前置序 GAP-INQUIRY-SPLIT). Inquiries are injected as
//! `Role::User` messages by the controller (they are runtime questions to the
//! model, not system context); the constants live here per design §4.6.6.
//! The orientation block text lives in `orz-assurance` (checkpoint.rs) so the
//! journal `message_block` payload and the injected text stay one source;
//! `ORIENTATION_INJECTED_PREFIX` registers it with the stagnation filter.

/// Base system prompt — runtime-neutral orientation.
///
/// D-1 (FIX_PLAN 2026-08-06): the citation rule lives here (the main-agent
/// prompt is the carrier — NOT a project doc / CLAUDE.md / index; those are
/// dev-directory documents unrelated to the binary). One rule + an inline
/// marker, no structured template (the main agent works alone — no helper
/// subagent, so the burden must be minimal). The rule blocks hallucinated
/// attributions ("参考自某处" claims without a locatable source — P7's
/// false "Python 移植" record is the direct precedent). ADR-0010 §3.7.9
/// (V11-IMPL-004): the marker carries a stable `source_id` binding to the
/// source ledger when one exists (observation-time `path:line` remains the
/// local-code fallback); the marker is a WRITER-SIDE binding, NOT a
/// verification claim — a verifier must check source identity / visibility
/// / claim limits, not grep the text.
pub const BASE_SYSTEM_PROMPT: &str = "你是 orz——保证优先的 CLI agent workbench。\
遵循注入的 assurance 上下文块执行任务；工具可用性由运行时声明，不得自行推断。\
\n引用纪律：凡基于外部依据、参考实现或内部文档的引用，必须在引用处附带内联标记 \
`[来源: source_id]`（已有 ledger 记录时）或 `[来源: 路径:行号]`（本地代码 observation-time \
定位）；外部来源引用 URL/document identity + observed scope（如 `[来源: <url> metadata_only]`，\
metadata-only 不得生成全文级归因）；无法定位来源的内容不得引用——不得凭记忆声称『参考自某处』。\
内部文档引用用 文档ID §节/锚点 而非裸行号（行号会漂移）。标记是写入侧绑定，不构成验证；\
来源身份、可见性等级与 claim 上限由 verifier 机械检查。\
\n压缩白名单（A6 §8 C.2）：任务背景、必须获取的信息等客观事实，可在首个工具批次通过 \
compaction_whitelist_add 写入压缩白名单——该内容不被上下文压缩、全程保留；\
写入仅限首轮，存档于 .gsa 记录树（保留 7 天）。白名单只写客观事实，\
不写计划/步骤/推测/临时状态（计划由 plan mode 承载）。";

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

/// GAP-INQUIRY-SPLIT (2026-08-09): prefix of the injected orientation block
/// (ADR-0010 §4.2 — session-level 7-round neutral inquiry). Registered with
/// `is_injected_block_text` so the injected block never enters stagnation
/// inputs. The full block text lives in `orz-assurance` (checkpoint.rs) — the
/// journal `message_block` payload and the injected message share it.
pub const ORIENTATION_INJECTED_PREFIX: &str = "[ORIENTATION";

/// Whether `content` is one of the runtime-injected assurance blocks.
/// The controller filters these out of stagnation inputs (D7) — fixed injected
/// text is not model output and repeated blocks would pollute ngram stats.
///
/// GAP-INQUIRY-SPLIT (2026-08-09): the old INFO_SUFFICIENCY and
/// RETRIEVAL_COMPLETION_CHECK blocks are deleted (the mixed-counter inquiry
/// mechanism is gone); the orientation block is registered by prefix — the
/// v0.2 block text carries a version marker (`[ORIENTATION v0.2]`), so the
/// registration must be a prefix match, not equality.
pub fn is_injected_block_text(content: &str) -> bool {
    let content = content.trim();
    content == COUNTEREXAMPLE_GATE_BLOCK
        || content == COUNTEREXAMPLE_GATE_PLAN_BLOCK
        // GAP-INQUIRY-SPLIT: the injected orientation block must never be
        // stagnation input — a repeated `[ORIENTATION …]` block would
        // otherwise pollute the guard's ngram stats (R-8 regression point).
        || content.starts_with(ORIENTATION_INJECTED_PREFIX)
        || content.starts_with(TOOL_POLICY_BREAKER_PREFIX)
        || content.starts_with(TOOL_ROUND_BUDGET_PREFIX)
        // 2026-08-08 blackboard partition (review closure, P2-1/D2-1): the
        // incremental-push summary `[本轮编辑] …` is mechanical injected
        // text — same rule as the blocks above. Without registration it
        // entered the stagnation guard's public_outputs, and iterating the
        // same file for ≥11 rounds (a normal edit pattern) tripped
        // STAGNATION-NGRAM-REPEAT on the message's repeated 3-gram.
        || content.starts_with(EDIT_ROUND_PUSH_PREFIX)
        // A6 (2026-08-08): the context-compaction marker is mechanical
        // injected text (see `context_compressed_marker`) — never
        // stagnation input.
        || content.starts_with(CONTEXT_COMPRESSED_PREFIX)
        // A6 §8 C.2 (2026-08-08): the resident compaction-whitelist
        // message repeats every round — mechanical injected text, never
        // stagnation input (model-written, but a resident framework-
        // managed block, not per-round model output).
        || content.starts_with(WHITELIST_PREFIX)
        // GAP-SUBAGENT-RUNTIME M5 (2026-08-10): the Diagnostic Coverage
        // checkpoint block (ADR-0010 §4.6.4) is mechanical injected text —
        // never stagnation input.
        || content.starts_with(crate::diagnostic_coverage::DIAGNOSTIC_COVERAGE_PREFIX)
}

/// 2026-08-08 blackboard partition (A2): prefix of the incremental-push
/// message the controller injects after a tool round that made file edits
/// (`[本轮编辑] 1.py 2→3行变动；…`). Mechanical injected text — never
/// stagnation input (registered in `is_injected_block_text`).
pub const EDIT_ROUND_PUSH_PREFIX: &str = "[本轮编辑";

/// A6 (2026-08-08): prefix of the compaction marker message (`[前文上下文
/// 已压缩 …]`) the controller inserts at the compaction cut point.
/// Mechanical injected text — registered in `is_injected_block_text` (the
/// marker is injected once per compaction and would otherwise pollute the
/// stagnation guard's ngram stats on repeated compactions).
pub const CONTEXT_COMPRESSED_PREFIX: &str = "[前文上下文已压缩";

/// A6 §8 C.2 (2026-08-08): prefix of the resident compaction-whitelist
/// message — the model-written list of task facts that survive compaction
/// (task background, must-know constraints). The message lives in the
/// conversation's preamble zone (after the original prompt, before the
/// first tool declaration), so the compaction mechanism skips it as part
/// of the always-kept preamble — never re-injected, never strengthened.
/// Registered in `is_injected_block_text` (it repeats every round and must
/// not pollute the stagnation guard's ngram stats).
pub const WHITELIST_PREFIX: &str = "[压缩白名单";

/// A6 §8 C.2: build the resident whitelist message content from the
/// entry list.
pub fn build_whitelist_block(entries: &[String]) -> String {
    let mut lines = vec![format!("{WHITELIST_PREFIX} v0.1]")];
    for entry in entries {
        lines.push(entry.clone());
    }
    lines.push("[/压缩白名单]".to_string());
    lines.join("\n")
}

/// A6: the marker message — tells the model the earlier conversation was
/// compacted and points it at `blackboard_read` for look-backs (design §5
/// A6: 压缩段打标「前文上下文已压缩」——同时是回查提示；隐式逼迫不可靠，模型
/// 无元认知，必须显式告知).
///
/// Review D2-2 (2026-08-08): `summary_line` is a one-line MECHANICAL digest
/// of what the dropped rounds did (edit count + tool-action counts by
/// category, read from the blackboard — zero model calls, deterministic) —
/// this is the 「工具结果全文 → 摘要」 of design §5 in its deterministic
/// form. `None` when the blackboard has nothing to summarize.
pub fn context_compressed_marker(
    rounds_dropped: u32,
    trigger_tokens: u64,
    summary_line: Option<&str>,
) -> String {
    let summary = summary_line
        .map(|s| format!(" 累计: {s}"))
        .unwrap_or_default();
    format!(
        "{CONTEXT_COMPRESSED_PREFIX} v0.1]\n\
         前文 {rounds_dropped} 轮已压缩（触发于 {trigger_k}K tokens）。\
         之前的工具结果全文不再在本对话中；如需回看历史，请调用 \
         blackboard_read 工具（分区: plan / edits / tool_actions / exec）。\
         {summary}\n\
         [/前文上下文已压缩]",
        trigger_k = trigger_tokens / 1000,
    )
}

/// 2026-08-08 blackboard partition (A4): prefix of the resident status line
/// block (`[任务状态 v0.1]`). Plan-state-derived — injected into the system
/// prompt ONLY when a plan is set (zero dilution without one).
pub const STATUS_LINE_PREFIX: &str = "[任务状态";

/// A4 (2026-08-08): build the resident 极简状态行 over the blackboard plan
/// section — goal + plan summary (total steps, current step, remaining) —
/// 2-3 lines. Rendered from STABLE plan state only: the controller keeps
/// the block byte-identical across rounds while the plan is unchanged, so
/// the rebuilt system prompt keeps its provider prefix-cache hit (2026-08-07
/// discipline — per-round-varying content lives in trailing messages, e.g.
/// `[本轮编辑]` / `[TOOL_ROUND_BUDGET] REMAINING`).
///
/// Edit counts are deliberately EXCLUDED from the resident block: they
/// change per edit round, and a per-round-varying system prompt recreates
/// the 17.7%→98% cache regression (2026-08-07 fix). Per-round edit deltas
/// already arrive via the `[本轮编辑]` push; totals are one blackboard_read
/// (edits partition) away.
pub fn build_status_line(goal: Option<&str>, steps: &[crate::blackboard::PlanStep]) -> String {
    use crate::blackboard::StepStatus;
    let goal = goal.unwrap_or("(未设置)");
    let mut lines = vec![format!("{STATUS_LINE_PREFIX} v0.1]")];
    if steps.is_empty() {
        lines.push(format!("目标: {goal}（无计划步骤）"));
    } else {
        // Current step = the first in-progress step, falling back to the
        // first pending one when nothing is marked in-progress yet.
        let current = steps
            .iter()
            .position(|s| s.status == StepStatus::InProgress)
            .or_else(|| steps.iter().position(|s| s.status == StepStatus::Pending))
            .map(|i| i + 1);
        let done = steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .count();
        let middle = match current {
            Some(i) => format!(
                "当前第 {i} 步「{}」",
                steps[i - 1].description,
            ),
            None => "当前步骤: (无)".to_string(),
        };
        lines.push(format!("目标: {goal}"));
        lines.push(format!(
            "计划: 共 {} 步，已完成 {}，{middle}，待办 {} 步",
            steps.len(),
            done,
            steps.len() - done,
        ));
    }
    lines.push("[/任务状态]".to_string());
    lines.join("\n")
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

/// IP2a denial-circuit-breaker message (D-3, FIX_PLAN 2026-08-06; ADR-0010
/// §3.5.4 / V11-IMPL-012): injected after 3 CONSECUTIVE TOOL ROUNDS whose
/// denials share one normalized key (tool, reason_code, policy_revision) —
/// the model has been retrying a refused tool (polyglot probe P3:
/// `web_search`×4 burned a third of the round budget). It tells the model to
/// switch strategy, names the refused tool, and is counted as injected text
/// (never stagnation input). The old total-denial ceiling (10/run) is
/// deleted: anti-runaway is the round budget, not a second denial counter.
pub const TOOL_POLICY_BREAKER_PREFIX: &str = "[TOOL_POLICY_BREAKER]";

pub fn tool_policy_breaker_block(tool_name: &str, consecutive: u32) -> String {
    format!(
        "{TOOL_POLICY_BREAKER_PREFIX} v0.1\n\
        Consecutive tool rounds have been refused by the session permission policy \
        ({consecutive} rounds in a row, last: '{tool_name}'). The refused tool is NOT \
        available under the current policy — do not retry it. Switch strategy: \
        use only the tools declared as available, or state that the task cannot \
        be completed under the current policy.\n\
        [/TOOL_POLICY_BREAKER]"
    )
}

/// D-8 (FIX_PLAN 2026-08-06): session-level budget declaration — injected
/// into the system prompt once per run. The remaining count is deliberately
/// NOT part of the system prompt: the system is rebuilt each round, so any
/// per-round state inside it breaks the provider's prefix cache on every
/// round (2026-08-07 fix — hit rate was ~17%; the count is declared by the
/// trailing `tool_round_budget_remaining_block` messages instead).
pub fn tool_round_budget_session_block(budget: u32) -> String {
    format!(
        "{TOOL_ROUND_BUDGET_PREFIX} v0.1]\n\
         BUDGET: {budget} tool rounds per turn\n\
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
/// ADR-0010 §3.2 retrieval task contract + §3.7.9 citation rule — the
/// subagent's system prompt (NOT `BASE_SYSTEM_PROMPT`: a retrieval task
/// contract is not a run-semantic; GAP-SUBAGENT-RUNTIME 2026-08-10).
///
/// The citation-rule text is preserved verbatim from the pre-split
/// one-shot pass (`retrieval.rs`, 2026-08-10) — the stable interface;
/// the delivery contract (the `[DOC]`/`[SOURCE]` line protocol the
/// caller parses mechanically) is now an explicit clause instead of an
/// implicit write contract. `blocks` carries the shared availability +
/// budget declarations (the subagent budget is its own — independent
/// per-session accounting).
pub fn build_retrieval_system_prompt(section_name: &str, goal: &str, blocks: &str) -> String {
    format!(
        "Retrieval subagent ({section_name}). Goal: {goal}\n\
         Citation rule (D-1, FIX_PLAN 2026-08-06; ADR-0010 §3.7.9): \
         any claim based on external evidence, a reference \
         implementation, or internal docs must carry an inline \
         `[来源: source_id]` marker (ledger-backed) at the citing site \
         — the ledger is the single binding authority; observation-time \
         path references are notes only, never a verification claim. \
         Content without a locatable source must not be cited — never \
         claim '参考自某处' from memory. Internal docs cite as 文档ID \
         §节/锚点; EXTERNAL sources cite as URL/document identity + \
         observed scope (e.g. `[来源: <url> metadata_only]`) — never \
         full-text attribution for metadata-only material.\n\
         Delivery contract: use your actual tool results. Output \
         `[DOC]`-prefixed lines for project docs and `[SOURCE]`-prefixed \
         lines for sources (internal: source_ledger; external: \
         web_sources); lines without a prefix form the plain response \
         prose. OPTIONALLY, after the prose, emit one \
         `[RESULT_JSON]{{...}}[/RESULT_JSON]` block carrying your \
         organized response: `{{\"sections\": [{{\"section_title\": ..., \
         \"content\": ..., \"source_ids\": [\"SRC-...\"], \"claim_strength\": \
         \"observed|derived|synthesized\"}}], \"claims\": [...]}}` — every \
         source_id must reference an actual tool result you received; \
         claim_strength must not exceed what the source's visibility \
         supports (observed = you read the full text, derived = partial, \
         synthesized = metadata-level). The mechanical ledger is built \
         from your tool calls, not from this block.\n\n\
         {blocks}"
    )
}

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
        // D-1 (FIX_PLAN 2026-08-06) + ADR-0010 §3.7.9 (V11-IMPL-004): the
        // citation rule lives in the main-agent prompt (the binary's carrier)
        // — inline marker `[来源: source_id]` (ledger-backed) or
        // `[来源: 路径:行号]` (local observation-time); no bare line numbers
        // for internal docs (they drift); the marker is a writer-side
        // binding, not a verification claim.
        assert!(
            BASE_SYSTEM_PROMPT.contains("[来源: source_id]"),
            "ledger-backed citation marker in the main-agent prompt"
        );
        assert!(
            BASE_SYSTEM_PROMPT.contains("[来源: 路径:行号]"),
            "observation-time path:line fallback still present"
        );
        assert!(
            BASE_SYSTEM_PROMPT.contains("不构成验证"),
            "marker is a binding, not a verification claim"
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
    fn counterexample_blocks_match_adr0010_v1_1_s4_5() {
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
        assert!(is_injected_block_text(COUNTEREXAMPLE_GATE_BLOCK));
        assert!(is_injected_block_text(COUNTEREXAMPLE_GATE_PLAN_BLOCK));
        // GAP-INQUIRY-SPLIT (2026-08-09): the old mixed-counter inquiry
        // blocks are deleted; the orientation block is registered by prefix
        // (its v0.2 text carries a version marker — equality would miss it).
        let orientation_block = orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK;
        assert!(is_injected_block_text(orientation_block));
        assert!(is_injected_block_text("[ORIENTATION v0.2] 当前任务、位置与下一目标"));
        // The closing tag must never match (starts with `[/`).
        assert!(!is_injected_block_text("[/ORIENTATION]"));
        // The retired blocks must NOT match — nothing injects them anymore.
        assert!(!is_injected_block_text("[INFO_SUFFICIENCY v0.1] 部分拷贝"));
        assert!(!is_injected_block_text("[RETRIEVAL_COMPLETION_CHECK v0.1]"));
        // D-3 breaker + D-8 budget blocks (FIX_PLAN 2026-08-06) — budget
        // blocks use the versioned marker form; the prefix must match it
        // (2026-08-07 review F-04: the constant previously ended in `]` and
        // never matched the `[TOOL_ROUND_BUDGET v0.1]` messages).
        assert!(is_injected_block_text(&tool_policy_breaker_block("web_search", 3)));
        assert!(is_injected_block_text(&tool_round_budget_session_block(120)));
        assert!(is_injected_block_text(&tool_round_budget_remaining_block(38)));
        assert!(is_injected_block_text(&tool_round_budget_exhaustion_block(120)));
        // A6 (2026-08-08): the context-compaction marker is mechanical
        // injected text — never stagnation input.
        assert!(is_injected_block_text(&context_compressed_marker(4, 152_000, None)));
        assert!(context_compressed_marker(4, 152_000, None).contains("152K"));
        assert!(context_compressed_marker(4, 152_000, None).contains("4 轮已压缩"));
        assert!(context_compressed_marker(4, 152_000, None).contains("blackboard_read"));
        assert!(context_compressed_marker(4, 152_000, None).starts_with("[前文上下文已压缩 v0.1]"));
        // D2-2 (2026-08-08): the optional mechanical digest line rides the
        // same marker (累计 label — session blackboard totals).
        let with_summary = context_compressed_marker(4, 152_000, Some("编辑 3 处；读 5"));
        assert!(with_summary.contains("累计: 编辑 3 处；读 5"));
        // A6 §8 C.2 (2026-08-08): the resident whitelist block is
        // mechanical injected text and never stagnation input.
        let whitelist = build_whitelist_block(&[
            "任务背景：修复 orz 的缓存回归".to_string(),
            "约束：不改动 schema".to_string(),
        ]);
        assert!(whitelist.starts_with("[压缩白名单 v0.1]"));
        assert!(whitelist.ends_with("[/压缩白名单]"));
        assert!(whitelist.contains("任务背景：修复 orz 的缓存回归"));
        assert!(is_injected_block_text(&whitelist));
        assert!(is_injected_block_text("  [压缩白名单 v0.1]\n条目\n[/压缩白名单]"));
        assert!(!is_injected_block_text("[/压缩白名单]"));
        // The base prompt carries the whitelist notice (static — cache-safe).
        assert!(BASE_SYSTEM_PROMPT.contains("compaction_whitelist_add"));
        assert!(BASE_SYSTEM_PROMPT.contains("压缩白名单"));
        // Leading/trailing whitespace tolerated.
        assert!(is_injected_block_text(&format!(
            "  {orientation_block}\n"
        )));
        // Ordinary model/user text must never match.
        assert!(!is_injected_block_text("完成"));
        // The prefix match is deliberately conservative: the legacy v0.1
        // text `[ORIENTATION_CHECKPOINT …]` also hits — harmless (that text
        // was never injected into a conversation; over-matching only makes
        // the stagnation filter stricter).
        assert!(is_injected_block_text("[ORIENTATION_CHECKPOINT v0.1] 手抄"));
        // A closing tag alone must never match (starts with `[/`).
        assert!(!is_injected_block_text("[/TOOL_ROUND_BUDGET]"));
        assert!(!is_injected_block_text(""));
    }

    #[test]
    fn status_line_renders_goal_steps_and_current() {
        use crate::blackboard::{PlanStep, StepStatus};
        let steps = vec![
            PlanStep {
                id: "step-1".into(),
                description: "调查".into(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                id: "step-2".into(),
                description: "实施".into(),
                status: StepStatus::Pending,
            },
        ];
        let line = build_status_line(Some("修复 bug"), &steps);
        assert!(line.starts_with("[任务状态 v0.1]"));
        assert!(line.ends_with("[/任务状态]"));
        assert!(line.contains("目标: 修复 bug"));
        assert!(line.contains("共 2 步，已完成 0"));
        assert!(line.contains("当前第 1 步「调查」"));
        assert!(line.contains("待办 2 步"));

        // No plan section → fallback goal text.
        let bare = build_status_line(None, &[]);
        assert!(bare.contains("目标: (未设置)"));
        assert!(bare.contains("无计划步骤"));
    }

    #[test]
    fn status_line_tracks_completed_steps() {
        use crate::blackboard::{PlanStep, StepStatus};
        let steps = vec![
            PlanStep {
                id: "step-1".into(),
                description: "调查".into(),
                status: StepStatus::Completed,
            },
            PlanStep {
                id: "step-2".into(),
                description: "实施".into(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                id: "step-3".into(),
                description: "验证".into(),
                status: StepStatus::Pending,
            },
        ];
        let line = build_status_line(Some("修复 bug"), &steps);
        assert!(line.contains("已完成 1"));
        assert!(line.contains("当前第 2 步「实施」"));
        assert!(line.contains("待办 2 步"));
    }
}
