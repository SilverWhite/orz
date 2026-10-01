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
//! `ORIENTATION_INJECTED_PREFIX` registers it with the injected-block filter.

/// Base system prompt — THIN-HARNESS-REDESIGN-V2 §9.1 (2026-08-29 用户
/// 裁决)：**全空**。模型可见面 = 任务指令（用户消息）+ 工具列表 + 注入
/// 块；契约全部由工具描述 / 结果信封 / 机械门承载：
/// read_file 信封与 offset 续读、search_replace 内容锚点、
/// blackboard_read 分区简定义、submit 两阶段、写锚点校验与拒绝信封。
/// 机械门（预算 / 反例门 / orientation / 审计报告）走消息层，与 system
/// prompt 无关。P6：对模型极简 ≠ 对框架极简——减法只作用于模型可见面，
/// 框架机械层（HA 助理层 / 审计 / 门禁）承重不变。
///
/// 历史（已删除的提示词段落及其去向）："工具按需使用，一次一个"行为行与
/// "完成后用 submit 提交"引导行随 2026-08-29 置空一并消失（submit 两阶段
/// 契约下沉到 submit 工具描述）；更早的 D-1 引用纪律 / 读取纪律 / 会话
/// 数据边界 / 压缩白名单 / plan-first 文本 / 检索分区拉取行见 R1 审计。
pub const BASE_SYSTEM_PROMPT: &str = "";

/// Counterexample gate block — 正式答案输出前, fires once per run and the
/// block explicitly tells the model it appears only once (§4.6.5 verbatim).
pub const COUNTEREXAMPLE_GATE_BLOCK: &str = "[COUNTEREXAMPLE_GATE v0.1]\n\
最终回答即将输出。请对即将输出的结论做最后一次反例自查：\n\
1. 是否存在未验证的前提？\n\
2. 是否存在可推翻结论的已知证据？\n\
3. 结论强度是否超出证据支持？\n\
注意：本反例询问仅出现一次，请在最终回答前完成全部反例自查。\n\
[/COUNTEREXAMPLE_GATE]";

/// GAP-INQUIRY-SPLIT (2026-08-09): prefix of the injected orientation block
/// (ADR-0010 §4.2 — session-level 7-round neutral inquiry). Registered with
/// `is_injected_block_text` so the injected block never enters the persisted
/// conversation. The full block text lives in `orz-assurance` (checkpoint.rs) — the
/// journal `message_block` payload and the injected message share it.
pub const ORIENTATION_INJECTED_PREFIX: &str = "[ORIENTATION";

/// P0-0x S1 (ADR-0010 §14.66): prefix of the injected **initial-round**
/// neutral inquiry block (`[INITIAL_ROUND_INQUIRY v0.1] …`). Same discipline
/// as `ORIENTATION_INJECTED_PREFIX` — the one-shot opening inquiry is
/// mechanical injected text and must never be persisted back into the
/// conversation (or pollute the injected-block / restore filters). The full
/// block text lives in `orz-assurance` (`checkpoint.rs`), so the journal
/// `message_block` payload and the injected message stay one source.
pub const INITIAL_ROUND_INQUIRY_INJECTED_PREFIX: &str = "[INITIAL_ROUND_INQUIRY";

/// Whether `content` is one of the runtime-injected assurance blocks.
/// The controller filters these out of the persisted conversation (D7) — fixed injected
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
        // GAP-INQUIRY-SPLIT: the injected orientation block must never be
        // persisted — a repeated `[ORIENTATION …]` block would
        // otherwise pollute the persisted conversation (R-8 regression point).
        || content.starts_with(ORIENTATION_INJECTED_PREFIX)
        // P0-0x S1: the one-shot initial-round inquiry block is mechanical
        // injected text too — own prefix, so the two blocks stay
        // distinguishable (neither prefix matches the other's text).
        || content.starts_with(INITIAL_ROUND_INQUIRY_INJECTED_PREFIX)
        || content.starts_with(TOOL_POLICY_BREAKER_PREFIX)
        || content.starts_with(TOOL_ROUND_BUDGET_PREFIX)
        // TER T1.9 (2026-09-04): F6 push 档 cue 是机械注入块（只报中性
        // 事实）——绝不持久化回会话（同 TOOL_ROUND_BUDGET 纪律）。
        || content.starts_with(F6_BUDGET_CUE_PREFIX)
        // 2026-08-08 blackboard partition (review closure, P2-1/D2-1): the
        // incremental-push summary `[本轮编辑] …` is mechanical injected
        // text — same rule as the blocks above. Without registration it
        // entered the persisted conversation, and iterating the
        // same file for ≥11 rounds (a normal edit pattern) tripped
        // the injected-block filter on the message's repeated 3-gram.
        || content.starts_with(EDIT_ROUND_PUSH_PREFIX)
        // A6 (2026-08-08): the context-compaction marker is mechanical
        // injected text (see `context_compressed_marker`) — never
        // persisted back into the conversation.
        || content.starts_with(CONTEXT_COMPRESSED_PREFIX)
        // A6 §8 C.2 (2026-08-08): the resident compaction-whitelist
        // message repeats every round — mechanical injected text, never
        // persisted back (model-written, but a resident framework-
        // managed block, not per-round model output).
        || content.starts_with(WHITELIST_PREFIX)
        // MECHANICAL-AUDIT-LAYER (2026-08-24, ADR-0010 §14.39): the
        // `[MECHANICAL_AUDIT v0.1]` execution-fact report is mechanical
        // injected text — never persisted back into the conversation.
        || content.starts_with(crate::mechanical_audit::MECHANICAL_AUDIT_PREFIX)
        // P0-D S2 (2026-08-14): the model-visible action-ledger block
        // (`[动作台账 v0.1] …`) is mechanical injected text — it exists only
        // in per-request collapsed views, never in the persisted
        // conversation; registered so it can never pollute the injected-block/restore filters or
        // restore filters.
        || content.starts_with(crate::action_ledger::ACTION_LEDGER_PREFIX)
        // 动态上下文滑块 S1（2026-09-15，设计 §3.2/§3.4）：`[CONTEXT_SCALE …]`
        // 提醒块（首次驱逐固化提醒 + 实际上下文 500K/900K）是机械注入文本，
        // 每 run 只出现一次——绝不写回持久化会话（否则长单对话的 sidecar
        // 会累积固定注入文本，且与「固定文本不是模型输出」纪律冲突）。
        || content.starts_with(crate::context_scale::REMINDER_INJECTED_PREFIX)
        // S1 修订批（v7，2026-09-15，设计 §3.4.1）：压缩窗口内的机械提示
        // （「窗口剩余 N 轮」等）此前未注册 ⇒ 会被写回持久化会话（0AE
        // 遗留缺口，审查 P2 记录）。窗口轮提示是机械注入文本，同
        // `[CONTEXT_SCALE` 纪律。
        || content.starts_with(crate::context_scale::WINDOW_NOTICE_PREFIX)
        // 滑块上下文 v8（2026-09-16 勘误批，设计 §2）：`[上下文分块表 …]`
        // 是模型面投影层派生的**分块索引**（每轮现算／按 epoch 冻结），
        // 只存在于请求视图，绝不写回持久化会话。
        || content.starts_with(crate::model_face::BLOCK_TABLE_PREFIX)
}

/// 2026-08-08 blackboard partition (A2): prefix of the incremental-push
/// message the controller injects after a tool round that made file edits
/// (`[本轮编辑] 1.py 2→3行变动；…`). Mechanical injected text — never
/// persisted back (registered in `is_injected_block_text`).
pub const EDIT_ROUND_PUSH_PREFIX: &str = "[本轮编辑";

/// A6 (2026-08-08): prefix of the compaction marker message (`[前文上下文
/// 已压缩 …]`) the controller inserts at the compaction cut point.
/// Mechanical injected text — registered in `is_injected_block_text` (the
/// marker is injected once per compaction and would otherwise pollute the
/// persisted conversation on repeated compactions).
pub const CONTEXT_COMPRESSED_PREFIX: &str = "[前文上下文已压缩";

/// A6 §8 C.2 (2026-08-08): prefix of the resident compaction-whitelist
/// message — the model-written list of task facts that survive compaction
/// (task background, must-know constraints). The message lives in the
/// conversation's preamble zone (after the original prompt, before the
/// first tool declaration), so the compaction mechanism skips it as part
/// of the always-kept preamble — never re-injected, never strengthened.
/// Registered in `is_injected_block_text` (it repeats every round and must
/// not pollute the persisted conversation).
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
         blackboard_read 工具（分区: plan / edits / tool_actions / exec / actions / \
         internal_ret / external_ret）。\
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
         blackboard_read 工具（分区: plan / edits / tool_actions / exec / actions / \
         internal_ret / external_ret）。\n\
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
/// section — goal + plan summary (total steps, current step id + goal,
/// remaining) — 2-3 lines. P0-E 计划视图补渲染步骤 ID (2026-08-17): the
/// current step line carries its `id` so the console step gate's exact
/// `step_id` binding is visible without a blackboard_read. Rendered from
/// STABLE plan state only: ids never vary while the plan is unchanged, so
/// the controller keeps the block byte-identical across rounds and the
/// provider prefix-cache hit survives (2026-08-07 discipline — per-round-
/// varying content lives in trailing messages, e.g. `[本轮编辑]` /
/// `[TOOL_ROUND_BUDGET] REMAINING`).
///
/// Edit counts are deliberately EXCLUDED from the resident block: they
/// change per edit round, and a per-round-varying system prompt recreates
/// the 17.7%→98% cache regression (2026-08-07 fix). Per-round edit deltas
/// already arrive via the `[本轮编辑]` push; totals are one blackboard_read
/// (edits partition) away.
///
/// 0cg（2026-10-01）墙钟可见性拆除：原 0am S1 Part A 的「轮次预算:」
/// 投影行参数退役——本块只渲染目标/步骤推进事实。
pub fn build_status_line(goal: Option<&str>, steps: &[crate::blackboard::PlanStep]) -> String {
    let goal = goal.unwrap_or("（无）");
    let mut lines = vec![format!("{STATUS_LINE_PREFIX} v0.1]")];
    if steps.is_empty() {
        lines.push(format!("目标: {goal}（无计划步骤）"));
    } else {
        // 2026-08-18 (ADR-0010 §14.25 项 2): 当前步 = 第一个非 done（与
        // `planning::current_step_index` 步骤门一致）；failed 步骤保持
        // 当前、可重试（旧逻辑跳过 failed 会与门禁显示不一致）。
        let current = steps
            .iter()
            .position(|s| !s.status.is_done())
            .map(|i| i + 1);
        let done = steps.iter().filter(|s| s.status.is_done()).count();
        let middle = match current {
            Some(i) => format!(
                "当前第 {i} 步 [{}]「{}」",
                steps[i - 1].id,
                steps[i - 1].goal,
            ),
            None => "当前步骤: （无）".to_string(),
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
/// budget declarations (session budget + exhaustion). PUSH→PULL
/// (2026-08-21): the per-round remaining declaration is retired — the live
/// count is read on demand via `blackboard_read section=session` (a tool
/// result, not an injected block). Counted as injected text — never
/// persisted back.
///
/// Deliberately matches the versioned marker form (`[TOOL_ROUND_BUDGET v0.1]`)
/// as well as the bare form — the previous constant ended in `]` and never
/// matched the versioned messages (2026-08-07 review F-04). The closing tag
/// `[/TOOL_ROUND_BUDGET]` does not match (starts with `[/`).
pub const TOOL_ROUND_BUDGET_PREFIX: &str = "[TOOL_ROUND_BUDGET";

/// TER T1.9 (2026-09-04)：F6 push 档机械注入块前缀——剩余评测墙钟跨阈值
/// 时注入一次中性事实（只报剩余/上限/已用轮数，不附建议）；默认 off
/// （PUSH→PULL 纪律的显式例外）。与其它机械注入块同注册——绝不持久化
/// 回会话。
pub const F6_BUDGET_CUE_PREFIX: &str = "[F6_BUDGET_CUE";

/// F6 push 档注入文本（中性事实，无建议）。
pub fn f6_budget_cue_block(remaining_secs: u64, limit_secs: u64, rounds_used: u32) -> String {
    format!(
        "{F6_BUDGET_CUE_PREFIX} v0.1] 评测墙钟剩余约 {remaining_secs}s（上限 {limit_secs}s）；\
         已用工具轮 {rounds_used}。[/F6_BUDGET_CUE]"
    )
}

/// TER T1.9：跨阈值判定（纯函数）——`remaining < 600/300/120` 且该档未
/// 注入过时返回该档（每 run 每档至多一次 → ≤3 次/run；T0.2 verifier
/// 上限 4 兼容）。同轮只取第一个未注入的更高档（避免一跳多档刷屏）。
pub fn f6_push_cue_for_remaining(remaining_secs: u64, crossed: &mut [bool; 3]) -> Option<u64> {
    const THRESHOLDS: [u64; 3] = [600, 300, 120];
    for (i, threshold) in THRESHOLDS.iter().enumerate() {
        if remaining_secs < *threshold && !crossed[i] {
            crossed[i] = true;
            return Some(*threshold);
        }
    }
    None
}

/// 0cg（2026-10-01）墙钟可见性拆除：原 0am S1 Part A 的 T̂→墙钟↔轮次
/// 换算渲染件（`WALLCLOCK_REMAINING_ROUNDS` 行、resident `[任务状态]`
/// 「轮次预算:」行、1-2-5 阶梯投影纯函数族）已整体退役——设计权威见
/// BACKLOG 0cg（138 批用户裁决「直接撤掉这一设计」）；`ORZ_MAX_WALLCLOCK`
/// 硬超时、轮预算硬门、0z 资源门、orientation 阈值语义不变。
///
/// IP2a denial-circuit-breaker message (D-3, FIX_PLAN 2026-08-06; ADR-0010
/// §3.5.4 / V11-IMPL-012): injected after 3 CONSECUTIVE TOOL ROUNDS whose
/// denials share one normalized key (tool, reason_code, policy_revision) —
/// the model has been retrying a tool the permission policy did not let
/// through (polyglot probe P3: `web_search`×4 burned a third of the round
/// budget). It tells the model to switch strategy, names that tool, and is
/// counted as injected text
/// (never persisted back). The old total-denial ceiling (10/run) is
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

/// PUSH→PULL (2026-08-21, CONTEXT_SCAFFOLDING_PULL_REDESIGN §4 方案 A):
/// the `blackboard_read section=session` face — the on-demand replacement
/// for the retired per-round `[TOOL_ROUND_BUDGET] REMAINING` trailing
/// injection. `used` = tool rounds consumed so far (completed rounds; the
/// round currently in flight counts against the budget when it completes);
/// `remaining` = budget − used. `status_line` = the resident `[任务状态]`
/// block (`None` when no plan is set). The mechanical hard gates
/// (`budget_insufficient` precheck, exhaustion block, `run_invalidated`)
/// stay untouched — this face is advisory, never a correctness premise.
///
/// 0cg（2026-10-01）墙钟可见性拆除：原 TER T1.8 的
/// `session_face_block_with_wallclock`（`WALLCLOCK_ELAPSED` /
/// `WALLCLOCK_LIMIT` / `WALLCLOCK_REMAINING` 三行）与 0am S1 Part A 的
/// `WALLCLOCK_REMAINING_ROUNDS` 换算行同源退役；`TOOL_ROUND_*` 行与
/// status 行保留。
pub fn session_face_block(used: u32, budget: u32, status_line: Option<&str>) -> String {
    // TER T1.7 (2026-09-04)：默认 `budget == 0` = 无硬限——模型面不再宣示
    // 一个并不存在的 120 轮静态上限，改为 unlimited（显式配置非零上限时
    // 才渲染数字档与 remaining）。
    let budget_line = if budget == 0 {
        "TOOL_ROUND_BUDGET: unlimited (配置 ORZ_MAX_TOOL_ROUNDS / 构造上限后显示档位)".to_string()
    } else {
        format!("TOOL_ROUND_BUDGET: {budget} tool rounds per turn")
    };
    let remaining = if budget == 0 {
        "unlimited".to_string()
    } else {
        budget.saturating_sub(used).to_string()
    };
    let mut out = format!(
        "[SESSION v0.1]\n\
         {budget_line}\n\
         TOOL_ROUNDS_USED: {used} (completed so far; the round in flight \
         counts when it completes)\n\
         TOOL_ROUNDS_REMAINING: {remaining}\n",
    );
    if let Some(line) = status_line {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str("[/SESSION]");
    out
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
/// 0t (2026-09-09, ADR-0010 §14.65 / 设计 §3.2): 三值检索模式退役——通道
/// 契约不再按 mode 分支；外部 lane 双族恒在并携带 ≤1 句静态推荐序
/// （本地浏览器优先、失败按普通错误回传可自由换道），内部 lane 仅文档
/// 读族。标注只出现在一次性子代理提示与工具描述，无新增常驻 token。
pub fn build_retrieval_system_prompt(
    role: crate::agents::SubagentRole,
    goal: &str,
    blocks: &str,
) -> String {
    let section_name = role.section_name();
    let lanes_contract = match role {
        crate::agents::SubagentRole::ExternalRetrieval => {
            "Retrieval lanes (0t dual-lane): both lanes are available in \
             this session — browser_read — [车道:本地浏览器检索|推荐首选] \
             reads pages directly (the read IS the original text); \
             web_search — [车道:原生检索] is the native lane (verify only \
             high-value / conclusion-dependent candidates with web_fetch; \
             the candidate budget is mechanical '候选 N/M，剩余 K'). Prefer \
             the local-browser lane first; lane failures are returned as \
             ordinary errors with their real cause — switch lanes freely."
        }
        crate::agents::SubagentRole::InternalRetrieval => {
            "Retrieval scope: internal project documentation only — use the \
             read family / project_doc_index; external web tools are not \
             part of this lane."
        }
    };
    format!(
        "Retrieval subagent ({section_name}). Goal: {goal}\n\
         Source weighting (ADR-0010 §3.7 条 12): every web source carries a \
         mechanical tier in the ledger (authoritative 1.1 / default 1.0 / \
         low_quality 0.7). Prefer higher-weight sources for conclusions; a \
         low-quality source MAY be used — the mechanical tier is the only \
         weighting signal (model annotations retired, GAP-RETRIEVAL-\
         STRUCTURED-RESULT 方向 C).\n\
         {lanes_contract}\n\
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
         prose. The mechanical ledger is built from your tool calls and \
         `[DOC]`/`[SOURCE]` declaration lines — there is no separate \
         structured block to emit.\n\n\
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

    /// IP2a: assemble the system prompt — the base is empty (2026-08-29
    /// §9.1), so the result is exactly the optional tool availability /
    /// probe block (if any) and nothing else.
    pub fn build_system_prompt(&self, tool_availability_block: Option<&str>) -> String {
        tool_availability_block.unwrap_or_default().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_injects_block() {
        // THIN-HARNESS-REDESIGN-V2 §9.1 (2026-08-29)：base 全空——
        // 无探针块时系统提示为空串；有块时返回块本身（无 base 拼接）。
        let builder = PromptBuilder::new();
        let bare = builder.build_system_prompt(None);
        assert_eq!(bare, "");
        let with_block = builder.build_system_prompt(Some("[PROBE] tools"));
        assert_eq!(with_block, "[PROBE] tools");
    }

    #[test]
    fn session_face_reports_unlimited_when_no_cap() {
        // TER T1.7 (2026-09-04)：默认轮预算无硬限（0）——session 面宣示
        // unlimited，不再渲染一个并不存在的静态档位。
        let unlimited = session_face_block(7, 0, None);
        assert!(
            unlimited.contains("TOOL_ROUND_BUDGET: unlimited"),
            "{unlimited}"
        );
        assert!(
            unlimited.contains("TOOL_ROUNDS_REMAINING: unlimited"),
            "{unlimited}"
        );
        assert!(
            !unlimited.contains("tool rounds per turn"),
            "unlimited face must not quote a fake static cap: {unlimited}"
        );

        // 显式配置非零上限 → 原静态档位宣示保留（逃生阀语义）。
        let capped = session_face_block(7, 120, None);
        assert!(
            capped.contains("TOOL_ROUND_BUDGET: 120 tool rounds per turn"),
            "{capped}"
        );
        assert!(capped.contains("TOOL_ROUNDS_REMAINING: 113"), "{capped}");
    }

    #[test]
    fn session_face_carries_no_wallclock_after_0cg() {
        // 0cg（2026-10-01）墙钟可见性拆除钉：session 面任何形态都不得再
        // 出现 WALLCLOCK_* 行（原 TER T1.8 三行与 0am S1 Part A 换算行
        // 已退役；评测墙钟回归纯机械面——orz-bin 硬门 + F6 push 默认 off）。
        let capped = session_face_block(3, 120, Some("[任务状态 v0.1] x\n[/任务状态]"));
        let unlimited = session_face_block(7, 0, None);
        for face in [capped, unlimited] {
            assert!(!face.contains("WALLCLOCK"), "{face}");
        }
    }

    #[test]
    fn f6_push_cue_is_neutral_and_crosses_each_threshold_once() {
        // TER T1.9 (2026-09-04)：注入文本只含中性事实，无建议。
        let block = f6_budget_cue_block(123, 900, 7);
        assert!(block.contains("剩余约 123s"), "{block}");
        assert!(block.contains("上限 900s"), "{block}");
        assert!(block.contains("已用工具轮 7"), "{block}");
        assert!(!block.contains("建议"), "{block}");
        assert!(!block.contains("请"), "{block}");

        // 每档至多一次 → 最多 3 次/run；同轮一跳多档只取第一档（更高档）。
        let mut crossed = [false; 3];
        assert_eq!(f6_push_cue_for_remaining(590, &mut crossed), Some(600));
        assert_eq!(f6_push_cue_for_remaining(290, &mut crossed), Some(300));
        assert_eq!(f6_push_cue_for_remaining(110, &mut crossed), Some(120));
        assert_eq!(f6_push_cue_for_remaining(50, &mut crossed), None);
        assert_eq!(crossed, [true, true, true]);

        // 一跳多档：后续轮次逐档补注入，总次数仍 ≤3。
        let mut jumped = [false; 3];
        assert_eq!(f6_push_cue_for_remaining(50, &mut jumped), Some(600));
        assert_eq!(f6_push_cue_for_remaining(50, &mut jumped), Some(300));
        assert_eq!(f6_push_cue_for_remaining(50, &mut jumped), Some(120));
        assert_eq!(f6_push_cue_for_remaining(50, &mut jumped), None);
    }

    #[test]
    fn base_system_prompt_is_empty() {
        // THIN-HARNESS-REDESIGN-V2 §9.1 (2026-08-29 用户裁决)：prompt
        // 全空——模型不接收任何系统级行为/契约文本；契约全部由工具描述、
        // 结果信封与机械门承载（P6：减法只作用于模型可见面）。
        assert!(
            BASE_SYSTEM_PROMPT.is_empty(),
            "base system prompt must be empty, got: {BASE_SYSTEM_PROMPT:?}"
        );
        assert_eq!(BASE_SYSTEM_PROMPT.lines().count(), 0);
    }

    #[test]
    fn base_system_prompt_avoids_availability_wording() {
        // P0-A 步骤 6 沿用：系统提示词不承载可用性判定词。
        assert!(
            !BASE_SYSTEM_PROMPT.contains("不可用"),
            "system prompt avoids unavailable wording: {BASE_SYSTEM_PROMPT}"
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
    }

    #[test]
    fn is_injected_block_text_detects_blocks() {
        assert!(is_injected_block_text(COUNTEREXAMPLE_GATE_BLOCK));
        // GAP-INQUIRY-SPLIT (2026-08-09): the old mixed-counter inquiry
        // blocks are deleted; the orientation block is registered by prefix
        // (its v0.2 text carries a version marker — equality would miss it).
        let orientation_block = orz_assurance::orientation::checkpoint::ORIENTATION_BLOCK;
        assert!(is_injected_block_text(orientation_block));
        assert!(is_injected_block_text("[ORIENTATION v0.4] 当前正在做什么"));
        // The closing tag must never match (starts with `[/`).
        assert!(!is_injected_block_text("[/ORIENTATION]"));
        // P0-0x S1: the one-shot initial-round inquiry block is registered by
        // its own prefix — the two injected blocks must not shadow each other
        // (a leak here would persist the block back into the conversation).
        let initial_block = orz_assurance::orientation::checkpoint::INITIAL_ROUND_INQUIRY_BLOCK;
        assert!(is_injected_block_text(initial_block));
        assert!(is_injected_block_text(
            "[INITIAL_ROUND_INQUIRY v0.1] 本任务实际要交付什么"
        ));
        assert!(is_injected_block_text(&format!("  {initial_block}\n")));
        assert!(!is_injected_block_text("[/INITIAL_ROUND_INQUIRY]"));
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
        assert!(is_injected_block_text(&tool_round_budget_exhaustion_block(
            120
        )));
        // A6 (2026-08-08): the context-compaction marker is mechanical
        // injected text — never persisted back into the conversation.
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
        // mechanical injected text and never persisted back into the conversation.
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
        // THIN-HARNESS-REDESIGN R1 (§4.2): the base prompt is near-zero —
        // the whitelist notice and every retired tool name are gone from it.
        assert!(!BASE_SYSTEM_PROMPT.contains("compaction_whitelist_add"));
        assert!(!BASE_SYSTEM_PROMPT.contains("压缩白名单"));
        assert!(!BASE_SYSTEM_PROMPT.contains("plan_write"));
        // Leading/trailing whitespace tolerated.
        assert!(is_injected_block_text(&format!("  {orientation_block}\n")));
        // Ordinary model/user text must never match.
        assert!(!is_injected_block_text("完成"));
        // The prefix match is deliberately conservative: the legacy v0.1
        // text `[ORIENTATION_CHECKPOINT …]` also hits — harmless (that text
        // was never injected into a conversation; over-matching only makes
        // the injected-block filter stricter).
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
        assert!(line.contains("当前第 1 步 [step-1]「调查」"));
        assert!(line.contains("待办 2 步"));

        // No plan section → fallback goal text.
        let bare = build_status_line(None, &[]);
        assert!(bare.contains("目标: （无）"));
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
        assert!(line.contains("当前第 2 步 [step-2]「实施」"));
        assert!(line.contains("待办 2 步"));
    }

    #[test]
    fn retrieval_prompt_carries_dual_lane_contract() {
        // GAP-SOURCE-WEIGHTING-IMPL (2026-08-13) / GAP-RETRIEVAL-STRUCTURED-
        // RESULT 方向 C (2026-08-30) + 0t (2026-09-09, ADR-0010 §14.65):
        // 外部 lane 双族契约 rides the retrieval system prompt; 模式分支
        // 文本（framework_fallback/local_browser/off）退役；[RESULT_JSON]
        // 与 layer-3 标注已删。
        let external = build_retrieval_system_prompt(
            crate::agents::SubagentRole::ExternalRetrieval,
            "goal",
            "",
        );
        assert!(external.contains("Source weighting"));
        assert!(
            !external.contains("source_annotations"),
            "layer-3 model annotation contract retired"
        );
        assert!(
            !external.contains("[RESULT_JSON]"),
            "organized block contract removed"
        );
        assert!(
            external.contains("[车道:本地浏览器检索|推荐首选]")
                && external.contains("[车道:原生检索]"),
            "dual-lane static labels ride the prompt"
        );
        assert!(
            external.contains("候选 N/M，剩余 K"),
            "mechanical budget feedback contract present"
        );
        assert!(
            !external.contains("at most 5"),
            "obsolete soft candidate cap removed"
        );
        assert!(
            !external.contains("browser_read is FORBIDDEN"),
            "二存一禁令退役"
        );
        assert!(
            !external.contains("\"annotated\""),
            "annotation status vocabulary retired"
        );

        // 内部 lane：仅项目文档读族描述，无外部 web 工具文本。
        let internal = build_retrieval_system_prompt(
            crate::agents::SubagentRole::InternalRetrieval,
            "goal",
            "",
        );
        assert!(internal.contains("internal project documentation only"));
        assert!(!internal.contains("[车道:原生检索]"));
        assert!(!internal.contains("[RESULT_JSON]"));
    }
}
