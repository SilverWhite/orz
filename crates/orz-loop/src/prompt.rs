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
///
/// FUS-RETRIEVAL-MECH P0-B step 6 (2026-08-14): shortened per
/// `RETRIEVAL_MECHANICAL_CONTROLS_DESIGN` §3.3 — the output-level verifier
/// is mechanical now, so the prompt carries only the marker formats + a
/// verifier notice (identity / visibility / claim limits), not the full
/// discipline essay.
pub const BASE_SYSTEM_PROMPT: &str = "你是 orz——保证优先的 CLI agent workbench。\
遵循注入的 assurance 上下文块执行任务；工具列表由运行时按轮声明，不得自行推断。\
\n引用纪律：基于外部依据、参考实现或内部文档的引用，必须附带内联标记 \
`[来源: source_id]`（ledger 记录）或 `[来源: 路径:行号]`（本地代码 observation-time 定位）；\
外部来源引用 URL/document identity + observed scope（如 `[来源: <url> metadata_only]`）；\
内部文档引用用 文档ID §节/锚点；无法定位来源的内容不得引用——不得凭记忆声称『参考自某处』。\
标记格式、来源身份、可见性等级与 claim 上限由 verifier 在交付前机械校验，不通过即阻止交付。\
\n读取纪律（缓存成本，v1.9）：优先用 grep/结构提取定位相关片段，再按需读取；\
只有证据关键文件才读全文，大文件用 offset 分段读取。每轮工具结果注入预算默认 50K \
估计 tokens（ORZ_MAX_INJECT_TOKENS_PER_ROUND 可调）；超限时本轮后续读取会被机械拒绝，\
并显式提示用 offset 续读或改用 grep/结构优先。\
\n压缩白名单（A6 §8 C.2）：任务背景、必须获取的信息等客观事实，可在首个工具批次通过 \
compaction_whitelist_add 写入压缩白名单——该内容不被上下文压缩、全程保留；\
写入仅限首轮，存档于 .gsa 记录树（保留 7 天）。白名单只写客观事实，\
不写计划/步骤/推测/临时状态（计划由 plan mode 承载）。\
\n操作台（P0-C v0.5）：需要执行动作时不要直接调用执行/发送类工具——先读注册板块 \
（blackboard_read section=actions，常驻按需读；内容=动作名+最小参数提示），\
再写动作栏订单（blackboard_action_write：action + arguments）；写订单无副作用，\
订单在轮末由机械层单一出口发放（注册表/契约/目标/ACAF/策略门），\
结果写回结果栏 receipt（含 trace_id；失败含 step/code/upstream）。\
单轮一单：本轮订单未发放完不进入下一轮写单，先看结果栏反馈再调整。";

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

/// FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): prefix of the mechanical
/// degradation block the final-answer citation verifier returns when a
/// `[来源: ...]` marker fails binding/claim validation (ADR-0010 §3.7.9).
/// Registered with `is_injected_block_text` — mechanical injected text,
/// never stagnation input.
pub const CITATION_VALIDATION_FAILED_PREFIX: &str = "[CITATION_VALIDATION_FAILED";

/// Build the explicit degradation block — the delivered final answer when
/// citation validation fails (never a silent downgrade).
pub fn citation_validation_failed_block(reason_codes: &[String]) -> String {
    format!(
        "{CITATION_VALIDATION_FAILED_PREFIX} v0.1]\n\
         最终回答的引用标记未通过机械校验，已阻止交付。\n\
         reason_codes: {}\n\
         [/CITATION_VALIDATION_FAILED]",
        reason_codes.join(", ")
    )
}

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
        // FUS-RETRIEVAL-MECH P0-B step 5 (2026-08-14): the citation
        // validation degradation block is mechanical injected text — never
        // stagnation input.
        || content.starts_with(CITATION_VALIDATION_FAILED_PREFIX)
        // P0-D S2 (2026-08-14): the model-visible action-ledger block
        // (`[动作台账 v0.1] …`) is mechanical injected text — it exists only
        // in per-request collapsed views, never in the persisted
        // conversation; registered so it can never pollute stagnation or
        // restore filters.
        || content.starts_with(crate::action_ledger::ACTION_LEDGER_PREFIX)
        // ORZ-ORIENTATION-FORCED-TEMPLATE (2026-08-15, ADR-0010 §14.16):
        // the one-shot checkpoint re-fill feedback block is mechanical
        // injected text — never stagnation input and never persisted back
        // into the conversation (same rule as the other User-role blocks).
        || content.starts_with(crate::checkpoint::CHECKPOINT_REFILL_PREFIX)
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
         blackboard_read 工具（分区: plan / edits / tool_actions / exec / actions）。\
         {summary}\n\
         [/前文上下文已压缩]",
        trigger_k = trigger_tokens / 1000,
    )
}

/// D3-1 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6): blocks
/// that MUST survive the conversation restore write-back filter — the
/// compaction marker (rolling single pointer, `[前文上下文已压缩` prefix) and
/// the resident whitelist (`[压缩白名单` prefix). Every other mechanical
/// injected block is still filtered from the persisted conversation.
pub fn is_restore_retained_block(content: &str) -> bool {
    content.starts_with(CONTEXT_COMPRESSED_PREFIX) || content.starts_with(WHITELIST_PREFIX)
}

/// D2-2 (2026-08-14, ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §6): the
/// recovery-truncation marker — tells the model a restored conversation was
/// mechanically truncated before the first request and points at the audit
/// copy in the run journal. Shares the `[前文上下文已压缩` prefix so D3-1
/// retains it across further restores.
pub fn recovery_truncation_marker(
    rounds_dropped: u32,
    before_estimate: u64,
    after_estimate: u64,
    audit_path: &str,
) -> String {
    format!(
        "{CONTEXT_COMPRESSED_PREFIX} v0.1-恢复]\n\
        恢复对话超过窗口，已机械截断 {rounds_dropped} 轮（估算 {before_k}K → {after_k}K \
         tokens）。完整历史保留于审计副本 {audit_path}；如需回看历史，请调用 \
         blackboard_read 工具（分区: plan / edits / tool_actions / exec / actions）。\n\
         [/前文上下文已压缩]",
        before_k = before_estimate / 1000,
        after_k = after_estimate / 1000,
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
            .position(|s| matches!(s.status, StepStatus::InProgress))
            .or_else(|| {
                steps
                    .iter()
                    .position(|s| matches!(s.status, StepStatus::Pending))
            })
            .map(|i| i + 1);
        let done = steps.iter().filter(|s| s.status.is_done()).count();
        let middle = match current {
            Some(i) => format!("当前第 {i} 步「{}」", steps[i - 1].goal,),
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
/// the model has been retrying a tool the permission policy did not let
/// through (polyglot probe P3: `web_search`×4 burned a third of the round
/// budget). It tells the model to switch strategy, names that tool, and is
/// counted as injected text
/// (never stagnation input). The old total-denial ceiling (10/run) is
/// deleted: anti-runaway is the round budget, not a second denial counter.
pub const TOOL_POLICY_BREAKER_PREFIX: &str = "[TOOL_POLICY_BREAKER]";

pub fn tool_policy_breaker_block(tool_name: &str, consecutive: u32) -> String {
    format!(
        "{TOOL_POLICY_BREAKER_PREFIX} v0.1\n\
        连续 {consecutive} 轮工具调用未获权限门禁放行（最后：'{tool_name}'）。\
        请勿继续调用该工具；请切换策略，改用本轮声明列表中的其他工具，\
        或在当前策略下说明任务无法完成。\n\
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
/// The citation-rule text descends from the pre-split one-shot pass
/// (`retrieval.rs`, 2026-08-10) — the stable interface; the delivery
/// contract (the `[DOC]`/`[SOURCE]` line protocol the caller parses
/// mechanically) is an explicit clause instead of an implicit write
/// contract. `blocks` carries the shared availability + budget
/// declarations (the subagent budget is its own — independent per-session
/// accounting).
///
/// FUS-RETRIEVAL-MECH P0-B step 6 (2026-08-14): the obsolete soft
/// "at most 5 candidates" prompt rule is gone — the candidate budget is
/// mechanical (`ORZ_WEB_FETCH_CANDIDATE_CAP`, per-result feedback
/// "候选 N/M，剩余 K"); the source-weighting / citation-rule paragraphs
/// are de-duplicated.
pub fn build_retrieval_system_prompt(
    section_name: &str,
    goal: &str,
    retrieval_mode: &str,
    blocks: &str,
) -> String {
    // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 — the
    // two-channel contract rides the subagent prompt. The mechanical tier
    // judge already labels every web source in the ledger; this text tells
    // the subagent HOW to use the labels (rank + annotate, never hard-block)
    // and WHICH channel may verify what (二存一 — never mix lanes).
    let weighting_contract = match retrieval_mode {
        "framework_fallback" => {
            "Retrieval channel: web_search is the entry; verify only \
             high-value / conclusion-dependent candidates with web_fetch \
             — the candidate budget is mechanical (per-result feedback \
             '候选 N/M，剩余 K'). browser_read is FORBIDDEN in this mode \
             (one channel per task)."
        }
        "local_browser" => {
            "Retrieval channel: browser_read reads pages directly — the \
             read IS the original text (no separate verification layer, no \
             web_fetch/web_search in this mode)."
        }
        _ => "Retrieval channel: off — no web retrieval tools.",
    };
    format!(
        "Retrieval subagent ({section_name}). Goal: {goal}\n\
         Source weighting (ADR-0010 §3.7 条 12): every web source carries a \
         mechanical tier in the ledger (authoritative 1.1 / default 1.0 / \
         low_quality 0.7). Prefer higher-weight sources for conclusions; a \
         low-quality source MAY be used but MUST be explicitly annotated in \
         `source_annotations` with status \"annotated\" (v0: annotate + rank, \
         no hard interception).\n\
         {weighting_contract}\n\
         Citation rule (ADR-0010 §3.7.9): every claim based on external \
         evidence, a reference implementation, or internal docs must carry \
         an inline `[来源: source_id]` marker (ledger-backed) at the citing \
         site. Content without a locatable source must not be cited — never \
         claim '参考自某处' from memory. Internal docs cite as 文档ID \
         §节/锚点; external sources cite as URL/document identity + \
         observed scope (e.g. `[来源: <url> metadata_only]`) — \
         metadata-only material never gets full-text attribution.\n\
         Delivery contract: use your actual tool results. Output \
         `[DOC]`-prefixed lines for project docs and `[SOURCE]`-prefixed \
         lines for sources (internal: source_ledger; external: \
         web_sources); lines without a prefix form the plain response \
         prose. OPTIONALLY, after the prose, emit one \
         `[RESULT_JSON]{{...}}[/RESULT_JSON]` block carrying your \
         organized response: `{{\"sections\": [{{\"section_title\": ..., \
         \"content\": ..., \"source_ids\": [\"SRC-...\"], \"claim_strength\": \
         \"observed|derived|synthesized\"}}], \"claims\": [...], \
         \"source_annotations\": [{{\"source_id\": \"SRC-...\", \"weight\": \
         0.7|1.0|1.1, \"reason\": \"...\", \"status\": \"adopted|annotated\"}}]}}` \
         — every \
         source_id must reference an actual tool result you received; \
         claim_strength must not exceed what the source's visibility \
         supports (observed = you read the full text, derived = partial, \
         synthesized = metadata-level). The mechanical ledger is built \
         from your tool calls, not from this block.\n\n\
         {blocks}"
    )
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
    fn system_prompt_injects_block() {
        // 2026-08-12 裁决：AVAILABLE 块已删除（prompt 不承载工具可用性
        // 声明——工具列表 = API tools 目录，判定在调用时）；build_system_prompt
        // 保留 None/Some 形态以兼容 budget/status 块。
        let builder = PromptBuilder::new();
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
            BASE_SYSTEM_PROMPT.contains("由 verifier 在交付前机械校验"),
            "verifier notice present"
        );
        assert!(
            BASE_SYSTEM_PROMPT.contains("不通过即阻止交付"),
            "verifier blocks delivery on failure"
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
    fn base_system_prompt_avoids_availability_wording() {
        // P0-A 步骤 6：系统提示词只声明"工具列表由运行时按轮声明"，
        // 不承载可用性判定词。
        assert!(
            BASE_SYSTEM_PROMPT.contains("工具列表由运行时按轮声明"),
            "system prompt declares the round-scoped tool list: {BASE_SYSTEM_PROMPT}"
        );
        assert!(
            !BASE_SYSTEM_PROMPT.contains("可用性"),
            "system prompt avoids availability wording: {BASE_SYSTEM_PROMPT}"
        );
    }

    #[test]
    fn tool_policy_breaker_block_is_neutral() {
        // P0-A 步骤 6：熔断块使用 `tool 'X'` 中性与事实陈述，不含
        // 可用/不可用/成功/失败/available/unavailable/success/failure
        // 等判定词，并明确给出"切换策略"指引。
        let block = tool_policy_breaker_block("search_replace", 3);
        for banned in [
            "可用",
            "不可用",
            "成功",
            "失败",
            "available",
            "unavailable",
            "success",
            "failure",
        ] {
            assert!(
                !block.to_lowercase().contains(banned),
                "breaker carries a banned judgment word '{banned}': {block}"
            );
        }
        assert!(block.contains("[TOOL_POLICY_BREAKER]"));
        assert!(block.contains("切换策略"));
        assert!(block.contains("未获权限门禁放行"));
        assert!(block.contains("search_replace"));
        assert!(block.contains('3'));
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
        assert!(is_injected_block_text(
            "[ORIENTATION v0.3] 当前任务、位置与下一目标"
        ));
        // §14.16: the checkpoint re-fill feedback is injected text.
        assert!(is_injected_block_text(
            &crate::checkpoint::refill_feedback_block(&["next_action 越界".to_string()])
        ));
        // The closing tag must never match (starts with `[/`).
        assert!(!is_injected_block_text("[/ORIENTATION]"));
        // The retired blocks must NOT match — nothing injects them anymore.
        assert!(!is_injected_block_text("[INFO_SUFFICIENCY v0.1] 部分拷贝"));
        assert!(!is_injected_block_text("[RETRIEVAL_COMPLETION_CHECK v0.1]"));
        // D-3 breaker + D-8 budget blocks (FIX_PLAN 2026-08-06) — budget
        // blocks use the versioned marker form; the prefix must match it
        // (2026-08-07 review F-04: the constant previously ended in `]` and
        // never matched the `[TOOL_ROUND_BUDGET v0.1]` messages).
        assert!(is_injected_block_text(&tool_policy_breaker_block(
            "web_search",
            3
        )));
        assert!(is_injected_block_text(&tool_round_budget_session_block(
            120
        )));
        assert!(is_injected_block_text(&tool_round_budget_remaining_block(
            38
        )));
        assert!(is_injected_block_text(&tool_round_budget_exhaustion_block(
            120
        )));
        // A6 (2026-08-08): the context-compaction marker is mechanical
        // injected text — never stagnation input.
        assert!(is_injected_block_text(&context_compressed_marker(
            4, 152_000, None
        )));
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
        assert!(is_injected_block_text(
            "  [压缩白名单 v0.1]\n条目\n[/压缩白名单]"
        ));
        assert!(!is_injected_block_text("[/压缩白名单]"));
        // The base prompt carries the whitelist notice (static — cache-safe).
        assert!(BASE_SYSTEM_PROMPT.contains("compaction_whitelist_add"));
        assert!(BASE_SYSTEM_PROMPT.contains("压缩白名单"));
        // Leading/trailing whitespace tolerated.
        assert!(is_injected_block_text(&format!("  {orientation_block}\n")));
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

    /// D3-1 (2026-08-14): only the compaction marker and the whitelist
    /// block are restore-retained; other mechanical injected blocks are not.
    #[test]
    fn restore_retained_blocks_identified() {
        assert!(is_restore_retained_block("[前文上下文已压缩 v0.1]\n内容"));
        assert!(is_restore_retained_block("[压缩白名单 v0.1]\n条目"));
        assert!(!is_restore_retained_block(
            "[ORIENTATION v0.1] 当前任务是什么？"
        ));
        assert!(!is_restore_retained_block("普通对话"));
        // Recovery markers share the compaction prefix and therefore count.
        assert!(is_restore_retained_block(&recovery_truncation_marker(
            2,
            250_000,
            150_000,
            ".gsa/runs/RUN-X/recovery.json"
        )));
    }

    #[test]
    fn status_line_renders_goal_steps_and_current() {
        use crate::blackboard::{PlanStep, StepStatus};
        let steps = vec![
            PlanStep {
                id: "step-1".into(),
                goal: "调查".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                id: "step-2".into(),
                goal: "实施".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
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
                goal: "调查".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::Done(crate::blackboard::DoneEvidence {
                    receipt_id: "ORD-1".into(),
                    direct: None,
                }),
            },
            PlanStep {
                id: "step-2".into(),
                goal: "实施".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::InProgress,
            },
            PlanStep {
                id: "step-3".into(),
                goal: "验证".into(),
                actions: Vec::new(),
                acceptance: String::new(),
                evidence: Vec::new(),
                status: StepStatus::Pending,
            },
        ];
        let line = build_status_line(Some("修复 bug"), &steps);
        assert!(line.contains("已完成 1"));
        assert!(line.contains("当前第 2 步「实施」"));
        assert!(line.contains("待办 2 步"));
    }

    #[test]
    fn retrieval_prompt_carries_mode_specific_weighting_contract() {
        // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13): ADR-0010 §3.7 条 12 —
        // the two-channel contract (二存一) and the layer-3 annotation
        // shape ride the retrieval system prompt.
        let framework =
            build_retrieval_system_prompt("external_ret", "goal", "framework_fallback", "");
        assert!(framework.contains("Source weighting"));
        assert!(framework.contains("source_annotations"));
        assert!(framework.contains("browser_read is FORBIDDEN in this mode"));
        assert!(
            framework.contains("候选 N/M，剩余 K"),
            "mechanical budget feedback contract present"
        );
        assert!(
            !framework.contains("at most 5"),
            "obsolete soft candidate cap removed"
        );
        assert!(framework.contains("\"annotated\""));

        let local = build_retrieval_system_prompt("external_ret", "goal", "local_browser", "");
        assert!(local.contains("no web_fetch/web_search in this mode"));
        assert!(local.contains("browser_read"));

        let off = build_retrieval_system_prompt("external_ret", "goal", "off", "");
        assert!(off.contains("no web retrieval tools"));
    }
}
