//! Action-ledger mechanical collapse (P0-D S2, 2026-08-14).
//!
//! ADR-0010 v1.10 / CONTEXT_COMPACTION_DESIGN §3: after every model tool
//! round, the MODEL-VISIBLE context collapses completed OLD rounds into one
//! deterministic action-ledger block — tool name, target path/URL, result
//! pointer (sha256 of the tool-result content; the full content stays in
//! the journal / conversation sidecar / P4 audit) and the round's non-empty
//! final reply. Zero model calls, no cooldown, whole-round pairing
//! discipline (a round is never split — every surviving tool reply keeps a
//! matching declaration).
//!
//! The persisted `messages` (the conversation) is never mutated here: the
//! collapsed view is built per model request, so the sidecar keeps the full
//! tool records for audit and the model context keeps only ledger rows plus
//! the bounded recent tail.

use crate::gateway::model::{Message, Role, ToolCall};
use orz_assurance::journal::sha256_hex;

/// Prefix of the model-visible action-ledger block (`[动作台账 v0.1] …`).
/// Registered in `is_injected_block_text` (mechanical injected text).
pub const ACTION_LEDGER_PREFIX: &str = "[动作台账";

/// Version marker of the ledger block format.
pub const ACTION_LEDGER_VERSION: &str = "v0.1";

/// Newest tool rounds kept verbatim in the model-visible context (bounded
/// exact tail; older rounds collapse into ledger rows).
pub const DEFAULT_RECENT_TAIL_ROUNDS: usize = 2;

/// FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
/// §14.28): the fixed pointer-message prefix injected into the folded
/// request view (`[U0][preamble][pointer][桥]`). The message is byte-fixed
/// for the session — the external ledger file is the only growing part, so
/// the request prefix stays byte-stable across advances (the per-window
/// rewrite is gone; measured hit rate 81.9% → ~91–93%). FUS-LEDGER-FOLD-
/// BRIDGE (2026-08-19, ADR-0010 §14.32): the bridge is the newest complete
/// rounds within the 8K real-token budget (形态甲: 先定裁剪、再内容截断);
/// the pointer text was updated once for the bridge wording (deployment
/// fingerprint change accepted by §3.4).
pub const LEDGER_FOLD_POINTER_PREFIX: &str = "【历史折叠】";

/// The append-only model-readable ledger projection file (per session;
/// run dirs are naturally isolated). Design: 追加式写入、只增不轮转（压缩
/// 不重置）；序号续点以文件为准——推进时读文件尾行取最大序号 +1。
pub fn ledger_file_path(session_cwd: &std::path::Path) -> std::path::PathBuf {
    session_cwd.join(".gsa").join("ledger").join("current.md")
}

/// The byte-fixed pointer message for the folded request view. 路径/文本
/// 均固定（本会话内不变），不含任何变化 ID/序号——推进不重渲染，前缀稳定。
/// 2026-08-18 审查修复：措辞避免绝对化——压缩会另存「未折叠即被 drain」的
/// 轮次（压缩存档 + marker），外挂文件是折叠轮次的归档投影。
pub fn build_pointer_message(ledger_path: &std::path::Path) -> String {
    format!(
        "{LEDGER_FOLD_POINTER_PREFIX}更早轮次的机械摘要已外挂存档：{}（本会话内固定）。\n\
         需要回顾历史时按行检索该文件，例如 grep \"轮次\" {}、\n\
         grep <工具名> {}。当前会话仅保留最近约 8K 桥接内容，更早轮次已按行归档于 {}。",
        ledger_path.display(),
        ledger_path.display(),
        ledger_path.display(),
        ledger_path.display(),
    )
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.2): 桥预算
/// 字符换算系数——S4 单题复验（path-tracing 2026-08-19 07:08，job
/// 2026-08-19__07-08-57）以折叠后首请求实际重付校准：第二次折叠后桥
/// 12,948 字符（view_estimate_after 6,474 估计 = chars/2）对应首请求重付
/// 6,493 真实 token → 实测 ≈ 2 字符/真实 token（英文/代码负载），初始
/// 保守值 4 高估一倍，按设计 §6 DoD 校准为 2。`estimate_messages_tokens`
/// 估计口径 = chars/2，故桥的估计预算 = `fold_tail_tokens ×
/// FOLD_TAIL_CHARS_PER_TOKEN ÷ 2`（默认 8_000 → 16_000 字符 → 8_000 估计
/// token ≈ 8K 真实 token）。
pub const FOLD_TAIL_CHARS_PER_TOKEN: u64 = 2;

/// 桥预算换算（真实 token 目标 → `estimate_messages_tokens` 估计口径）。
pub fn fold_tail_estimate_budget(fold_tail_tokens: u64) -> u64 {
    fold_tail_tokens.saturating_mul(FOLD_TAIL_CHARS_PER_TOKEN) / 2
}

/// One external-file row: `[<全局序号>] 轮次 <窗口内轮次>: <工具> 目标=…
/// 结果=… 最终回复=…`。`[<全局序号>]` 为按行续点的 per-row 全局序号
/// （跨压缩连续；多结果轮按条各占一行、序号连续递增），文件只增，读尾行
/// 即可续号；`轮次` 沿用现有台账行语义 = `round_index + 1`（窗口内 0 基
/// 声明序号 +1，跨压缩重置，与 `build_ledger_block` 标注一致）——2026-08-18
/// 审查修复：此前两者共用一个 per-row 序号，多工具轮会把轮次标错。
/// 2026-08-18 S4 复验修复：嵌入字段（目标/结果/最终回复）先做换行转义 +
/// 长度上限——否则多行最终回复会把一条逻辑记录拆成多个物理行，`tail_seq`
/// 把续行当损坏尾巴（InvalidData）→ 追加失败 → 连续 3 次后折叠被禁用
/// （实测 `[31] 轮次 27: blackboard_read ...` 记录跨 15 行）。单行契约是
/// 「追加续号」正确性的前提；上限符合设计「摘要行 ~200–350 字符/行」。
pub fn external_row_line(seq: u64, row: &ActionLedgerRow) -> String {
    /// Per-field cap for the single-line row contract（设计 §2.3 摘要行规模）。
    const LEDGER_ROW_FIELD_CAP: usize = 300;
    /// Escape embedded line breaks so a logical row is exactly one physical
    /// line, then bound the field so the file stays a compact grep-able
    /// summary (full content remains in `messages` / journal).
    fn sanitize_field(value: &str) -> String {
        let escaped = value.replace(['\r', '\n'], "\\n");
        if escaped.chars().count() <= LEDGER_ROW_FIELD_CAP {
            return escaped;
        }
        let mut truncated: String = escaped.chars().take(LEDGER_ROW_FIELD_CAP).collect();
        truncated.push('…');
        truncated
    }
    format!(
        "[{seq}] 轮次 {}: {} 目标={} 结果={} 最终回复={}",
        row.round_index + 1,
        row.tool,
        if row.target.is_empty() {
            "（无）".to_string()
        } else {
            sanitize_field(&row.target)
        },
        sanitize_field(&row.pointer),
        if row.final_reply.is_empty() {
            "（无）".to_string()
        } else {
            sanitize_field(&row.final_reply)
        },
    )
}

/// Read the last complete row's leading `[seq]` — the next per-row seq to
/// assign. The file is append-only with monotonically increasing seqs, so
/// the LAST row carries the max; a missing/empty file starts at 1. Reads
/// only the file tail — the window grows until the last row is fully
/// captured (a single row can exceed 4 KiB: long command targets / verbatim
/// final replies; parsing a partial row would silently restart the numbering
/// and break the 跨压缩连续 contract). A non-empty file whose last row has no
/// parseable `[seq]` is corruption — reported as an error so the caller rolls
/// back and degrades instead of renumbering from 1 (2026-08-18 审查修复).
fn tail_seq(path: &std::path::Path) -> std::io::Result<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    const TAIL_BYTES: u64 = 4096;
    let len = file.metadata()?.len();
    if len == 0 {
        return Ok(0);
    }
    let mut window = TAIL_BYTES;
    loop {
        let start = len.saturating_sub(window);
        file.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        (&mut file).take(window).read_to_end(&mut buf)?;
        let text = String::from_utf8_lossy(&buf);
        // Walk newline-separated segments from the end; a segment is a
        // complete row iff it is preceded by a newline inside the window
        // (k > 0) or the window covers the file head (start == 0).
        let mut last_row: Option<&str> = None;
        let segments: Vec<&str> = text.split('\n').collect();
        for (k, segment) in segments.iter().enumerate().rev() {
            if segment.trim().is_empty() {
                continue;
            }
            if k > 0 || start == 0 {
                last_row = Some(*segment);
            }
            break;
        }
        if start == 0 && last_row.is_none() {
            // Empty or blank-only file — nothing written yet.
            return Ok(0);
        }
        if let Some(row) = last_row {
            let seq = row
                .trim_start_matches(|c: char| c.is_whitespace())
                .trim_start_matches('[')
                .split(']')
                .next()
                .and_then(|s| s.trim().parse::<u64>().ok());
            return match seq {
                Some(n) => Ok(n),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("external ledger tail row has no parseable [seq]: {row:?}"),
                )),
            };
        }
        // The last row starts before the window — grow and retry.
        window = window.saturating_mul(2).max(len);
    }
}

/// Append the newly folded rows to the external ledger file (atomic
/// O_APPEND single write + flush). Assigns the per-row global seqs from
/// the file tail (`tail_seq` + 1, +2, …). Failure is reported to the
/// caller — a failed append must NOT advance the fold state (the rows
/// would be lost from both the view and the file; the next trigger retries).
pub fn append_ledger_rows(path: &std::path::Path, rows: &[ActionLedgerRow]) -> std::io::Result<()> {
    use std::io::Write;
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut seq = tail_seq(path)?;
    let mut buf = String::new();
    for row in rows {
        seq += 1;
        buf.push_str(&external_row_line(seq, row));
        buf.push('\n');
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(buf.as_bytes())?;
    file.flush()
}

/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): the stateful fold
/// point of one loop invocation.
///
/// The stateless `build_collapsed_request` recomputes the fold boundary
/// before EVERY model request — as each new complete round shifts the
/// retained tail, the whole request prefix is rewritten and the provider
/// prefix cache misses every round (measured ≈ 67% hit rate; see the
/// 2026-08-18 ledger-fold design). This struct freezes the fold point:
/// between advances the request view is `messages[..fold_start]` +
/// `folded_ledger` + `messages[fold_cut..]` — byte-stable, pure append.
/// FUS-LEDGER-FOLD-STATE external-file design (2026-08-18, ADR-0010
/// §14.28): `folded_ledger` is the byte-FIXED pointer message (not a
/// growing ledger block) — the folded rows are appended to the external
/// ledger file by the caller, so advances never rewrite the view prefix.
///
/// Invariant: `fold_start` and `fold_cut` are Some together (folded) or
/// None together; `folded_ledger` is the pointer message and is set on
/// the first advance, then byte-identical for the rest of the window.
/// The indices are positions into the CURRENT `messages` slice of the
/// loop; they are valid only for that slice (the state is reset after
/// compaction mutates the conversation and at every loop start).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerFoldState {
    /// Fold anchor: the first assistant tool declaration index (the
    /// preamble ends here). Set at the first advance, then immutable.
    pub fold_start: Option<usize>,
    /// Closed-end index of the folded region — the verbatim retention
    /// region of the request view is `[fold_cut..]`.
    pub fold_cut: Option<usize>,
    /// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.1):
    /// closed-end index of the FROZEN bridge — `messages.len()` at the last
    /// advance. The request view renders `[fold_cut..bridge_end)` as the
    /// bridge (reasoning stripped; content truncated to the budget only when
    /// the newest round alone overran it) and `[bridge_end..]` verbatim —
    /// between advances the bridge is fixed, so the view stays a pure
    /// append of the previous request (byte-stable prefix, cache discipline;
    /// the growing tail is never re-truncated per request).
    pub bridge_end: Option<usize>,
    /// The byte-fixed pointer message (external-file design; set once on
    /// the first advance — the folded rows live in the external ledger
    /// file, not in the request view).
    pub folded_ledger: Option<String>,
}

impl LedgerFoldState {
    /// Whether the fold point is active.
    pub fn is_folded(&self) -> bool {
        self.fold_cut.is_some()
    }

    /// Clear the fold state (compaction reset / fresh loop start / restore).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// One deterministic ledger row for a single executed tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionLedgerRow {
    /// 0-based round index among the assistant tool declarations.
    pub round_index: usize,
    /// Tool name (mechanical, from the assistant's tool call).
    pub tool: String,
    /// Best-effort target extracted from the call arguments (path/file/url/
    /// target/document_id/directory/command); empty when none is present.
    pub target: String,
    /// `sha256:<hex>` of the tool-result content — the journal/sidecar
    /// pointer for audit look-back; `no_result` when the result is missing.
    pub pointer: String,
    /// The round's non-empty final assistant reply, verbatim (empty when
    /// the round produced no text).
    pub final_reply: String,
}

/// Index ranges of complete tool rounds: each range starts at an assistant
/// tool-call declaration and ends at the next declaration (or the end).
fn round_ranges(messages: &[Message]) -> Vec<(usize, usize)> {
    let starts: Vec<usize> = messages
        .iter()
        .enumerate()
        .filter(|(_, m)| m.role == Role::Assistant && !m.tool_calls.is_empty())
        .map(|(i, _)| i)
        .collect();
    starts
        .iter()
        .enumerate()
        .map(|(k, &start)| (start, starts.get(k + 1).copied().unwrap_or(messages.len())))
        .collect()
}

/// Every assistant tool call in the round has a matching tool reply — a
/// round missing a result is never collapsed (an orphaned declaration would
/// 400 on replay; D2-1 pairing discipline).
fn is_round_complete(messages: &[Message], range: (usize, usize)) -> bool {
    let (start, end) = range;
    let declared: Vec<&str> = messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.as_str()))
        .collect();
    let replied: Vec<&str> = messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.as_deref())
        .collect();
    declared.iter().all(|id| replied.contains(id))
}

/// Mechanical best-effort target extraction from tool-call arguments.
fn target_of(tool_call: &ToolCall) -> String {
    const TARGET_FIELDS: &[&str] = &[
        "path",
        "file",
        "url",
        "target",
        "document_id",
        "directory",
        "command",
    ];
    let Some(args) = tool_call.arguments.as_object() else {
        return String::new();
    };
    for field in TARGET_FIELDS {
        if let Some(s) = args.get(*field).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

/// `sha256:<hex>` pointer of the tool-result content (or `no_result`).
fn pointer_of(tool_result: Option<&Message>) -> String {
    match tool_result {
        Some(m) => format!("sha256:{}", sha256_hex(m.content.as_bytes())),
        None => "no_result".to_string(),
    }
}

/// The round's last non-empty assistant text (verbatim), if any.
fn final_reply_of(messages: &[Message], range: (usize, usize)) -> String {
    messages[range.0..range.1]
        .iter()
        .filter(|m| m.role == Role::Assistant && !m.content.is_empty())
        .map(|m| m.content.clone())
        .last()
        .unwrap_or_default()
}

/// One row per tool call in the round ("多结果轮按条登记").
fn rows_for_round(
    messages: &[Message],
    range: (usize, usize),
    round_index: usize,
) -> Vec<ActionLedgerRow> {
    let final_reply = final_reply_of(messages, range);
    let mut rows = Vec::new();
    for m in &messages[range.0..range.1] {
        if m.role != Role::Assistant {
            continue;
        }
        for tc in &m.tool_calls {
            let tool_result = messages[range.0..range.1].iter().find(|r| {
                r.role == Role::Tool && r.tool_call_id.as_deref() == Some(tc.call_id.as_str())
            });
            rows.push(ActionLedgerRow {
                round_index,
                tool: tc.name.clone(),
                target: target_of(tc),
                pointer: pointer_of(tool_result),
                final_reply: final_reply.clone(),
            });
        }
    }
    rows
}

/// Render the deterministic ledger block (one line per row).
pub fn build_ledger_block(rows: &[ActionLedgerRow]) -> String {
    let mut lines = vec![format!("{ACTION_LEDGER_PREFIX} {ACTION_LEDGER_VERSION}]")];
    for row in rows {
        lines.push(format!(
            "轮次 {}: {} 目标={} 结果={} 最终回复={}",
            row.round_index + 1,
            row.tool,
            if row.target.is_empty() {
                "（无）"
            } else {
                &row.target
            },
            row.pointer,
            if row.final_reply.is_empty() {
                "（无）"
            } else {
                &row.final_reply
            },
        ));
    }
    lines.push("[/动作台账]".to_string());
    lines.join("\n")
}

/// Build the model-visible request view: preamble + one action-ledger block
/// for the collapsed OLD rounds + the newest `keep_recent_rounds` rounds
/// verbatim. The source `messages` slice is never modified.
///
/// Collapse stops before the first incomplete round — a round missing a
/// tool result is kept verbatim (it becomes part of the exact tail).
pub fn collapsed_cut(messages: &[Message], keep_recent_rounds: usize) -> Option<usize> {
    let ranges = round_ranges(messages);
    if ranges.is_empty() {
        return None;
    }
    let tail = keep_recent_rounds.max(1);
    let mut collapse_count = ranges.len().saturating_sub(tail);
    // FUS-LEDGER-FOLD-STATE review fix (2026-08-18): the completeness check
    // covers EVERY round to be collapsed — not just the boundary round — so
    // a mid-history incomplete round (declaration without a matching tool
    // reply) is never folded into ledger rows (`no_result` rows would lose
    // the in-flight declaration from the model view). Matches the doc
    // contract "a round missing a tool result is kept verbatim".
    while collapse_count > 0
        && !ranges[..collapse_count]
            .iter()
            .all(|&r| is_round_complete(messages, r))
    {
        collapse_count -= 1;
    }
    if collapse_count == 0 {
        return None;
    }
    Some(ranges[collapse_count].0)
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.2): 折叠
/// 推进的保留起点——桥预算版（取代外挂文件设计 §2.2「最近 1 轮」）。
/// 从最新完整轮往回累加整轮估计（`estimate_messages_tokens`，chars/2
/// 口径），直至再纳入更早一轮会超出桥预算；**最新一轮恒保留**（单独超预算
/// 时由视图内容截断 §3.3 处理），预算剩余时按时间从新到旧容纳更早完整轮。
/// `fold_tail_rounds` 语义退役（桥恒含 ≥1 轮结构，无需轮数下限）。
///
/// 完整性纪律不变：被折叠区（cut 之前）必须全为完整轮——第一个不完整轮
/// 及其后全部保留原文（绝不折叠，同 `collapsed_cut` 的 completeness
/// back-off；不完整轮进桥造成的预算超出由视图截断兜底，声明内容通常很小）。
pub fn bridge_cut(messages: &[Message], budget_estimate: u64) -> Option<usize> {
    let ranges = round_ranges(messages);
    if ranges.is_empty() {
        return None;
    }
    let mut kept_total: u64 = 0;
    let mut kept_from = ranges.len();
    for k in (0..ranges.len()).rev() {
        let estimate =
            crate::controller::estimate_messages_tokens(&messages[ranges[k].0..ranges[k].1]);
        // `kept_from == ranges.len()` 表示尚未计入任何轮——最新一轮恒入桥。
        if kept_from != ranges.len() && kept_total.saturating_add(estimate) > budget_estimate {
            break;
        }
        kept_total += estimate;
        kept_from = k;
    }
    if let Some(first_incomplete) = ranges.iter().position(|&r| !is_round_complete(messages, r))
        && kept_from > first_incomplete
    {
        kept_from = first_incomplete;
    }
    if kept_from == 0 {
        return None;
    }
    Some(ranges[kept_from].0)
}

/// Number of complete round ranges whose start index is strictly before
/// `cut` — the rounds a compaction draining `[first_declaration..cut)`
/// would drop (FUS-LEDGER-FOLD-STATE: the cut may be the frozen
/// `fold_cut` rather than a recomputed stateless tail).
pub fn rounds_before(messages: &[Message], cut: usize) -> usize {
    round_ranges(messages)
        .iter()
        .take_while(|&&(start, _)| start < cut)
        .count()
}

/// Build the model-visible request view: preamble + one action-ledger block
/// for the collapsed OLD rounds + the newest `keep_recent_rounds` rounds
/// verbatim. The source `messages` slice is never modified.
pub fn build_collapsed_request(messages: &[Message], keep_recent_rounds: usize) -> Vec<Message> {
    let Some(kept_start) = collapsed_cut(messages, keep_recent_rounds) else {
        return messages.to_vec();
    };
    let ranges = round_ranges(messages);
    let collapse_count = ranges
        .iter()
        .position(|&(start, _)| start == kept_start)
        .unwrap_or(ranges.len());
    let mut rows = Vec::new();
    for (idx, &range) in ranges.iter().take(collapse_count).enumerate() {
        rows.extend(rows_for_round(messages, range, idx));
    }
    let mut collapsed = messages[..ranges[0].0].to_vec();
    collapsed.push(Message {
        role: Role::User,
        content: build_ledger_block(&rows),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    collapsed.extend_from_slice(&messages[kept_start..]);
    collapsed
}

/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): build the
/// model-visible request view under the stateful fold point.
///
/// - Not folded: `messages` verbatim (no per-request stateless collapse —
///   the prefix stays byte-stable from the first round; the single fold
///   advance happens at the mechanical trigger instead).
/// - Folded: `messages[..fold_start]` (preamble) + the byte-fixed pointer
///   message (user message; the folded rows live in the external ledger
///   file) + the bridge (`messages[fold_cut..]` transformed).
///
/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.1/§3.3):
/// the bridge is the newest complete rounds within the `budget_estimate`
/// (`estimate_messages_tokens` 口径) — `reasoning_content` is stripped from
/// bridge messages (思维链不进桥; 审计仅保留计数) except for **plain-text**
/// assistant messages (no `tool_calls`), whose `reasoning_content` is kept
/// (THIN-HARNESS-REDESIGN-V2 §9.6, 2026-08-29 S5-1 修复 A — DeepSeek
/// /responses requires non-empty reasoning on assistant messages to be
/// passed back verbatim); when the whole bridge still exceeds the budget,
/// content is truncated oldest→newest (tool replies keep their TAIL +
/// 「…（前略）」+ 指针行, assistant final reply text follows the existing
/// `truncate_chars` head-199+… discipline, declarations keep visible
/// content + tool_calls complete). Messages are never removed and rounds
/// are never split, so the pairing invariant holds.
///
/// The source `messages` slice is never modified — the journal/sidecar
/// keeps the complete tool records (audit dual-track unchanged).
pub fn build_request_view(
    messages: &[Message],
    fold: &LedgerFoldState,
    budget_estimate: u64,
) -> Vec<Message> {
    let (Some(fold_start), Some(fold_cut)) = (fold.fold_start, fold.fold_cut) else {
        return messages.to_vec();
    };
    let Some(ledger) = fold.folded_ledger.as_deref() else {
        return messages.to_vec();
    };
    // FUS-LEDGER-FOLD-BRIDGE 审查处理 (2026-08-19, O2)：防御性索引越界
    // 守卫——压缩/恢复等路径在极端陈旧状态下可能让冻结索引超过当前
    // `messages` 长度（正常路径由推进与视图两侧的 preamble/safe_fold_cut
    // 防线覆盖，压缩路径会 reset 状态）；此时 `messages[..fold_start]` 或
    // `messages[cut..]` 会越界 panic，直接放弃折叠回原文（与 stale fold
    // 回退一致，最坏代价=每窗一次全量视图，低频接受）。
    if fold_start >= messages.len() || fold_cut > messages.len() {
        tracing::warn!(
            fold_start,
            fold_cut,
            len = messages.len(),
            "ledger fold indices out of bounds — returning the full view"
        );
        return messages.to_vec();
    }
    // FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27 / 处理
    // 文档 §2.2)：preamble 边界校验——`messages[..fold_start]` 末条若为
    // assistant 声明（tool_calls 非空），其 tool 回复必在折叠区（索引 ≥
    // fold_start），折叠视图会裸露声明 → provider 400 `insufficient tool
    // messages`。该形态发生在压缩触发（未执行）删除 marker 后冻结折叠
    // 索引未失效（索引左移）；此时放弃折叠回原文。
    if messages[..fold_start]
        .last()
        .is_some_and(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
    {
        tracing::warn!(
            fold_start,
            "ledger fold preamble ends with an assistant declaration — stale fold indices, returning the full view"
        );
        return messages.to_vec();
    }
    // FUS-LEDGER-FOLD-STATE 复验修复 (2026-08-18, make-doom-for-mips 单题
    // 复验 400 根因)：`fold_cut` 必须落在完整轮起点——尾部 `[cut..]` 不得
    // 以孤儿 tool 消息开头（其轮起点在 cut 之前）或含未配对 assistant
    // 声明，否则 provider 报 `invalid_request_error`（assistant tool_calls
    // 后 tool 消息不足）。压缩（marker 插入）与状态行尾随消息会移动消息
    // 索引，防御性回退到最近安全轮起点，并留痕供审计。
    let Some(cut) = safe_fold_cut(messages, fold_cut) else {
        tracing::warn!(
            fold_cut,
            "ledger fold cut has no safe complete round start — returning the full view"
        );
        return messages.to_vec();
    };
    if cut != fold_cut {
        tracing::warn!(
            fold_cut,
            cut,
            window = ?messages[cut.saturating_sub(2)..cut.saturating_add(10)]
                .iter()
                .map(|m| match m.role {
                    Role::Assistant => format!(
                        "A[{}]",
                        m.tool_calls
                            .iter()
                            .map(|t| t.call_id.as_str())
                            .collect::<Vec<_>>()
                            .join(",")
                    ),
                    Role::Tool => format!("T[{}]", m.tool_call_id.as_deref().unwrap_or("?")),
                    _ => "U".to_string(),
                })
                .collect::<Vec<_>>(),
            "ledger fold cut adjusted to a complete round start (provider pairing guard)"
        );
    }
    let mut view = messages[..fold_start].to_vec();
    view.push(Message {
        role: Role::User,
        content: ledger.to_string(),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    // FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.1/§3.2):
    // 桥 = 最近一次推进冻结的 `[cut..bridge_end)`（推进时 messages 末尾）——
    // 该区间内容级截断到预算（仅最新一轮单独超预算的例外情形）；`bridge_end`
    // 之后的轮次（推进后追加）原样保留、纯追加——折叠之间前缀字节稳定，
    // 推进后的增长尾部绝不被逐请求重截（缓存纪律与模型工作窗口）。
    let bridge_end = fold
        .bridge_end
        .unwrap_or(messages.len())
        .min(messages.len());
    let bridge_end = bridge_end.max(cut);
    view.extend(build_bridge(&messages[cut..bridge_end], budget_estimate));
    view.extend_from_slice(&messages[bridge_end..]);
    view
}

/// FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32 / 设计 §3.1/§3.5):
/// 桥视图变换——剔除 `reasoning_content`（不修改 `messages` 本身），仅
/// **纯文本** assistant 消息（无 `tool_calls`）保留其 `reasoning_content`
/// （THIN-HARNESS-REDESIGN-V2 §9.6, 2026-08-29 S5-1 修复 A——DeepSeek
/// /responses 对非空 reasoning 的 assistant 消息要求原样回传，orientation
/// 纯文本回答被 fold 落入桥内末条剥除 → 400 的根因）；整桥估计超预算时
/// 从旧到新逐消息内容截断直至 ≤ 预算。不删消息、不拆轮：任何声明的
/// call_id 都保留其工具回复在场（配对不变量）。
fn build_bridge(bridge: &[Message], budget_estimate: u64) -> Vec<Message> {
    let mut out: Vec<Message> = bridge
        .iter()
        .cloned()
        .map(|mut m| {
            if !(m.role == Role::Assistant && m.tool_calls.is_empty()) {
                m.reasoning_content = None;
            }
            m
        })
        .collect();
    if crate::controller::estimate_messages_tokens(&out) > budget_estimate {
        truncate_bridge_to_budget(&mut out, bridge, budget_estimate);
    }
    out
}

/// 工具回复截断形态行首前缀（「…（前略）」）。
const BRIDGE_TOOL_REPLY_OMIT_PREFIX: &str = "…（前略）\n";

/// 桥工具回复尾部保留的初始档位（字符）；全部压到当前档位仍超预算则减半。
const BRIDGE_TOOL_REPLY_INITIAL_TAIL: usize = 4 * 1024;

/// 桥工具回复尾部保留的最小档位——指针行恒可见（同 `render_notes_capped`
/// 的指针可见纪律）；低于该档位仍超预算属物理下限（消息不可删、轮不可拆），
/// 接受并留痕。
const BRIDGE_TOOL_REPLY_MIN_TAIL: usize = 256;

/// assistant 无 tool_calls 文本按既有 `truncate_chars` 口径（头部 199
/// 字符接「…」；设计 §3.3——轮内中间说明文本与最终回复统一按此口径，
/// 审查处理 2026-08-19 O1）。
const BRIDGE_FINAL_REPLY_MAX_CHARS: usize = 200;

/// 截断分配（设计 §3.3）：从旧到新逐消息压缩，直至整桥估计 ≤ 预算。
/// 声明消息（assistant + tool_calls）可见 content 与 tool_calls 完整保留；
/// 工具回复超档位时截断**保留尾部**（最新状态/错误/退出码最相关）+
/// 「…（前略）」+ 指针行（sha256 与台账行同口径，完整内容在日志/对话存档）；
/// assistant 无 tool_calls 文本（含轮内中间说明与最终回复）按
/// `truncate_chars`（头部 199 字符 + …）。
/// 指针行计入预算但必须可见。
///
/// `source` 为桥的源切片（与 `bridge` 一一对应）——每次截断都从**原文**
/// 截取，同一条消息在同一档位下幂等（指针行不会因重复截断而叠入尾部，
/// 「保留尾部」= 原文尾部），档位因此在无变更时可靠下降并终止。
fn truncate_bridge_to_budget(bridge: &mut [Message], source: &[Message], budget_estimate: u64) {
    debug_assert_eq!(
        bridge.len(),
        source.len(),
        "bridge is a 1:1 clone of the source tail"
    );
    let mut cap = BRIDGE_TOOL_REPLY_INITIAL_TAIL;
    loop {
        if crate::controller::estimate_messages_tokens(bridge) <= budget_estimate {
            return;
        }
        let mut changed = false;
        for (m, src) in bridge.iter_mut().zip(source.iter()) {
            match m.role {
                Role::Tool => {
                    if m.content.chars().count() > cap {
                        let truncated = truncate_tool_reply_tail(&src.content, cap);
                        // 截断形态（前缀 + 尾 + 指针）的长度可能不低于原文
                        // ——仅在**确实更短**时采纳，否则计为无变更、档位
                        // 减半，保证档位单调下降并终止。
                        if truncated.chars().count() < m.content.chars().count() {
                            m.content = truncated;
                            changed = true;
                        }
                    }
                }
                Role::Assistant
                    if m.tool_calls.is_empty()
                        && m.content.chars().count() > BRIDGE_FINAL_REPLY_MAX_CHARS =>
                {
                    let truncated =
                        crate::summary::truncate_chars(&src.content, BRIDGE_FINAL_REPLY_MAX_CHARS);
                    if truncated.chars().count() < m.content.chars().count() {
                        m.content = truncated;
                        changed = true;
                    }
                }
                _ => {}
            }
        }
        if !changed {
            if cap <= BRIDGE_TOOL_REPLY_MIN_TAIL {
                tracing::debug!(
                    budget_estimate,
                    bridge_messages = bridge.len(),
                    "ledger bridge at minimal tool-reply form still over budget — physical floor (messages are never removed)"
                );
                return;
            }
            cap /= 2;
        }
    }
}

/// 工具回复截断形态：行首「…（前略）」+ 保留尾部（`tail_chars` 字符）+ 指针
/// 行。OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2)：
/// 源 content 已含工具结果自身落盘指针（终端截断指针行「完整内容见
/// <路径>，请使用 read_file」）时，保留尾部即天然留存该行（桥 8K 信息损失
/// 可恢复），不再追加兜底；仅当源 content 无落盘指针（未截断过的工具结果）
/// 才生成「完整内容见 日志/对话存档 sha256:<hex>」兜底（与台账行
/// `结果=sha256:` 同口径；messages/日志/存档全量保留，审计双轨不变）。
fn truncate_tool_reply_tail(content: &str, tail_chars: usize) -> String {
    let tail: String = content
        .chars()
        .rev()
        .take(tail_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if has_tool_result_read_back_pointer(content) {
        format!("{BRIDGE_TOOL_REPLY_OMIT_PREFIX}{tail}")
    } else {
        format!(
            "{BRIDGE_TOOL_REPLY_OMIT_PREFIX}{tail}\n完整内容见 日志/对话存档 sha256:{}",
            sha256_hex(content.as_bytes())
        )
    }
}

/// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2):
/// 工具结果自身落盘指针行的机械判定——终端截断统一格式为
/// 「完整内容见 <路径>，请使用 read_file 读取（大文件用 offset/limit 分页）」
/// （types::output::truncation_read_back_pointer）。两个标志同时出现才算
/// 命中，避免与存档/TraceStore 指针（「完整内容见存档（epoch-…）」）混淆。
fn has_tool_result_read_back_pointer(content: &str) -> bool {
    content.contains("完整内容见") && content.contains("请使用 read_file")
}

/// 折叠态下的安全保留起点：`cut` 若落在某个工具轮的中间（尾部以孤儿
/// tool 消息开头）或起点轮不平衡，向前回退到最近的平衡轮起点——保证
/// 视图尾部任何 assistant 工具声明都带足 tool 回复、且任何 tool 消息
/// 都有前导声明（provider 协议配对不变量，双向——2026-08-18 复验补强：
/// console 订单发放的 ord-xxx 额外 tool 消息属"孤儿"，单向检查漏网）。
/// `round_ranges` 的起点恒为 assistant 声明；正常情况下 `advance_fold`
/// 的 `kept_start` 已是轮起点，此函数为压缩/状态行等索引漂移场景的
/// 防御兜底。FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27)：
/// 返回 `Option<usize>`——首轮（fold 区起点）即不完整时无安全轮起点可
/// 回退，返回 `None`（调用方放弃折叠回原文），不再原样返回旧 cut。
fn safe_fold_cut(messages: &[Message], cut: usize) -> Option<usize> {
    let ranges = round_ranges(messages);
    let mut c = cut;
    loop {
        // 最后一个起点 <= c 的轮（c 落在其内部或其后缘）。
        let Some(idx) = ranges.iter().rposition(|&(s, _)| s <= c) else {
            return Some(c);
        };
        let (s, e) = ranges[idx];
        if c == s {
            // 起点恰为轮起点：若该轮双向平衡则安全；否则（防御场景）前移
            // 到上一轮起点继续检查，直至 fold 区起点（preamble 边界）。
            if is_round_balanced(messages, (s, e)) {
                return Some(c);
            }
            if idx == 0 {
                // 最前轮仍不完整——无安全轮起点可回退，放弃折叠（调用方
                // 回原文；loop-top 不变量保证正常路径不会到达）。
                return None;
            }
            c = ranges[idx - 1].0;
            continue;
        }
        // cut 落在轮中间（孤儿 tool 消息风险）——回退到该轮起点。
        c = s;
    }
}

/// 双向配对检查：轮内每条声明的 call_id 都有 tool 回复，且每条 tool
/// 消息的 call_id 都来自轮内声明（孤儿 tool 消息——如 console 订单发放
/// 的 ord-xxx 额外回复——会被判不平衡，从而整轮回退保留，不折叠）。
fn is_round_balanced(messages: &[Message], range: (usize, usize)) -> bool {
    if !is_round_complete(messages, range) {
        return false;
    }
    let (start, end) = range;
    let declared: Vec<String> = messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Assistant)
        .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.clone()))
        .collect();
    messages[start..end]
        .iter()
        .filter(|m| m.role == Role::Tool)
        .filter_map(|m| m.tool_call_id.as_deref())
        .all(|id| declared.iter().any(|d| d == id))
}

/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26 + §14.28
/// external-file design) × FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010
/// §14.32): advance the fold point — fold the NEW complete old rounds
/// before the bridge (newest complete rounds within `budget_estimate`,
/// `estimate_messages_tokens` 口径) and move `fold_cut` forward.
///
/// Returns `Some(rows)` — ONLY the rows newly folded since the last
/// advance — and the caller appends them to the external ledger file
/// (IO is deliberately outside this pure function). `folded_ledger` is
/// set to the byte-fixed pointer message on the FIRST advance and never
/// rewritten (the request-view prefix stays byte-stable across advances:
/// preamble + pointer + `[fold_cut..]`). `fold_start` is set on the first
/// advance (first tool declaration) and never changes. `messages` itself
/// is never mutated.
///
/// Returns `None` (no-op, anti-spin) when there is no complete round to
/// fold — i.e. every complete round fits in the bridge (cut stays at 0),
/// the recomputed cut does not move past the current `fold_cut` (nothing
/// new outside the bridge since the last advance), or the same preamble/cut
/// safety checks that make `build_request_view` abandon the fold fire
/// (stale fold indices / unbalanced rounds — rows must never be appended
/// for rounds the view would keep verbatim).
pub fn advance_fold(
    messages: &[Message],
    fold: &mut LedgerFoldState,
    budget_estimate: u64,
    ledger_path: &std::path::Path,
) -> Option<Vec<ActionLedgerRow>> {
    let kept_start = bridge_cut(messages, budget_estimate)?;
    let old_cut = fold.fold_cut;
    if old_cut.is_some_and(|cut| kept_start <= cut) {
        return None;
    }
    let ranges = round_ranges(messages);
    let fold_start = fold.fold_start.unwrap_or(ranges[0].0);
    // FUS-LEDGER-FOLD-STATE 400 修复 (ADR-0010 §14.27) × external-file
    // design (2026-08-18 审查修复): the same preamble/cut safety checks
    // that make `build_request_view` abandon the fold must gate the
    // ADVANCE itself — rows must never be appended to the external file
    // for rounds the view would refuse to fold (defensive stale-index /
    // unbalanced-round paths). Preamble 末条为 assistant 声明 → 放弃;
    // `safe_fold_cut` 回退到最近平衡轮起点（`cut` 可能早于 `kept_start`——
    // 该区间留作原文，行不入文件，与视图一致）。
    if messages[..fold_start]
        .last()
        .is_some_and(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
    {
        return None;
    }
    let cut = safe_fold_cut(messages, kept_start)?;
    if old_cut.is_some_and(|c| cut <= c) {
        return None;
    }
    let collapse_count = ranges
        .iter()
        .position(|&(start, _)| start == cut)
        .unwrap_or(ranges.len());
    // Only the rounds newly folded since the last advance produce rows:
    // `[old_cut .. cut)` — previously folded rounds are already in
    // the external file (and were never part of the view).
    let first_new = old_cut
        .map(|c| {
            ranges
                .iter()
                .position(|&(start, _)| start >= c)
                .unwrap_or(collapse_count)
        })
        .unwrap_or(0);
    let mut rows = Vec::new();
    for (idx, &range) in ranges
        .iter()
        .enumerate()
        .take(collapse_count)
        .skip(first_new)
    {
        rows.extend(rows_for_round(messages, range, idx));
    }
    if rows.is_empty() {
        return None;
    }
    fold.fold_start = Some(fold_start);
    fold.fold_cut = Some(cut);
    // 冻结桥末端 = 推进时的消息末尾；此后追加的轮次不属于桥（视图纯追加，
    // 直到下一次推进才把新桥冻结）。
    fold.bridge_end = Some(messages.len());
    if fold.folded_ledger.is_none() {
        fold.folded_ledger = Some(build_pointer_message(ledger_path));
    }
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        }
    }

    fn round(call_id: &str, tool: &str, target: &str, result: &str) -> Vec<Message> {
        vec![
            Message {
                role: Role::Assistant,
                content: String::new(),
                tool_call_id: None,
                tool_calls: vec![ToolCall {
                    name: tool.to_string(),
                    arguments: serde_json::json!({"path": target}),
                    call_id: call_id.to_string(),
                }],
                reasoning_content: None,
            },
            Message {
                role: Role::Tool,
                content: result.to_string(),
                tool_call_id: Some(call_id.to_string()),
                tool_calls: Vec::new(),
                reasoning_content: None,
            },
        ]
    }

    fn declared_ids(messages: &[Message]) -> Vec<String> {
        messages
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.clone()))
            .collect()
    }

    /// 桥预算测试助手：直接给出 `estimate_messages_tokens` 估计口径的整段
    /// 预算（与生产换算 `fold_tail_estimate_budget` 解耦，便于按轮精确控制
    /// 桥内轮数）。
    fn est_budget(messages: &[Message], from: usize) -> u64 {
        crate::controller::estimate_messages_tokens(&messages[from..])
    }

    /// FUS-LEDGER-FOLD-STATE 复验修复 (2026-08-18)：`fold_cut` 落在轮中间
    /// （孤儿 tool 消息风险）时回退到最近完整轮起点。
    #[test]
    fn safe_fold_cut_mid_round_falls_back_to_round_start() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "grep", "b.rs", "B"));
        // 消息结构：0=user, 1=assistant(c1), 2=tool(c1), 3=assistant(c2),
        // 4=tool(c2)。
        assert_eq!(
            safe_fold_cut(&messages, 2),
            Some(1),
            "cut at tool msg of round 1"
        );
        assert_eq!(
            safe_fold_cut(&messages, 4),
            Some(3),
            "cut at tool msg of round 2"
        );
        assert_eq!(safe_fold_cut(&messages, 3), Some(3), "complete round start");
        assert_eq!(safe_fold_cut(&messages, 5), Some(3), "cut past the end");
    }

    /// FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27)：
    /// `safe_fold_cut` 无安全轮起点可回退（idx==0 首轮不完整）→ `None`
    /// （放弃折叠），不再原样返回旧 cut。
    #[test]
    fn safe_fold_cut_first_round_unbalanced_returns_none() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.pop(); // 删除 tool 回复 → 首轮不完整（声明裸露）
        assert_eq!(
            safe_fold_cut(&messages, 1),
            None,
            "cut at the unbalanced first round start"
        );
        assert_eq!(
            safe_fold_cut(&messages, 2),
            None,
            "cut mid-round also backtracks to the unbalanced first round"
        );
    }

    /// FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27)：折叠
    /// 态 preamble 末条为 assistant 声明（其 tool 回复被折叠）时放弃折叠
    /// 回原文——补齐「safe_fold_cut 只防 cut 不防 fold_start」的缺口。
    #[test]
    fn build_request_view_abandons_fold_when_preamble_ends_with_declaration() {
        // 模拟压缩触发（未执行）删除 marker 后冻结折叠索引失效：fold_start=2
        // 使 preamble = [U0, A[c1]]——声明无回复，折叠视图必 400。
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "grep", "b.rs", "B"));
        let fold = LedgerFoldState {
            fold_start: Some(2),
            fold_cut: Some(4),
            folded_ledger: Some("ledger".to_string()),
            ..Default::default()
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        assert_eq!(view, messages, "stale fold falls back to the full view");
    }

    /// FUS-LEDGER-FOLD-STATE 400 修复 (2026-08-18, ADR-0010 §14.27)：旧 bug
    /// 场景全链——marker 在索引 1、fold 三态冻结；压缩触发 retain 删除
    /// marker（未 reset）后索引左移。build_request_view 必须回原文，且
    /// 视图中每个声明的 call_id 都有 tool 回复（无 400 形态）。
    #[test]
    fn marker_removal_without_fold_reset_builds_full_view() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(msg(Role::User, "[前文上下文已压缩 v0.2] 摘要"));
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "grep", "b.rs", "B"));
        // 旧代码：压缩触发首行无条件 retain 删除 marker，折叠三态未失效。
        messages.retain(|m| {
            !m.content
                .starts_with(crate::prompt::CONTEXT_COMPRESSED_PREFIX)
        });
        let fold = LedgerFoldState {
            fold_start: Some(2),
            fold_cut: Some(4),
            folded_ledger: Some("ledger".to_string()),
            ..Default::default()
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        assert_eq!(view, messages, "stale indices fall back to the full view");
        let declared: Vec<String> = view
            .iter()
            .filter(|m| m.role == Role::Assistant)
            .flat_map(|m| m.tool_calls.iter().map(|tc| tc.call_id.clone()))
            .collect();
        let replied: Vec<String> = view
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.clone())
            .collect();
        assert!(
            declared.iter().all(|d| replied.iter().any(|r| r == d)),
            "every declared call has a tool reply: {view:?}"
        );
    }

    /// FUS-LEDGER-FOLD-BRIDGE 审查处理 (2026-08-19, O2)：极端陈旧状态下
    /// 冻结折叠索引可能超出当前 `messages` 长度——必须放弃折叠回原文，
    /// 不得 `messages[..fold_start]` / `messages[cut..]` 越界 panic。
    #[test]
    fn build_request_view_defends_out_of_bounds_fold_indices() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        // fold_start / fold_cut 均超出当前消息长度（压缩删除后索引未失效
        // 的极端场景）；folded_ledger 已设置——守卫必须优先于切片访问。
        let fold = LedgerFoldState {
            fold_start: Some(5),
            fold_cut: Some(9),
            folded_ledger: Some("ledger".to_string()),
            ..Default::default()
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        assert_eq!(
            view, messages,
            "out-of-bounds fold indices fall back to the full view"
        );

        // fold_cut 单独越界（fold_start 仍有效）同样回原文。
        let fold_cut_oob = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(99),
            folded_ledger: Some("ledger".to_string()),
            ..Default::default()
        };
        let view2 = build_request_view(&messages, &fold_cut_oob, 1_000_000);
        assert_eq!(
            view2, messages,
            "out-of-bounds fold_cut falls back to the full view"
        );
    }

    /// 折叠态下视图尾部不得含孤儿 tool 消息——cut 回退后第一条必须是
    /// assistant 声明，且所有声明都有 tool 回复。
    #[test]
    fn build_request_view_adjusts_orphan_tool_tail() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "grep", "b.rs", "B"));
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(2),
            folded_ledger: Some("ledger".to_string()),
            ..Default::default()
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        // 视图 = [user(0), ledger, 尾部[1..]（c1 轮 + c2 轮，4 条）] = 6 条。
        assert_eq!(view.len(), 6);
        assert_eq!(view[0], messages[0]);
        assert_eq!(view[1].role, Role::User);
        assert!(view[1].content.contains("ledger"));
        assert_eq!(view[2], messages[1], "tail starts at assistant declaration");
        let declared = declared_ids(&view[2..]);
        let replied: Vec<&str> = view[2..]
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert!(
            declared.iter().all(|d| replied.contains(&d.as_str())),
            "every declared call has a tool reply"
        );
    }

    #[test]
    fn zero_rounds_is_identity() {
        let messages = vec![msg(Role::User, "你好")];
        let collapsed = build_collapsed_request(&messages, DEFAULT_RECENT_TAIL_ROUNDS);
        assert_eq!(collapsed, messages);
    }

    #[test]
    fn old_rounds_collapse_into_ledger_rows() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "search_replace", "b.rs", "已编辑"));
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        let collapsed = build_collapsed_request(&messages, 2);
        // Preamble + ledger block + newest 2 rounds (c2/c3 are NOT collapsed
        // when only the oldest is older than the tail — tail=2 keeps c2+c3).
        let ledger = collapsed
            .iter()
            .find(|m| m.content.starts_with(ACTION_LEDGER_PREFIX))
            .expect("ledger block present");
        assert!(ledger.content.contains("read_file"), "{ledger:?}");
        assert!(ledger.content.contains("目标=a.py"), "{ledger:?}");
        assert!(ledger.content.contains("sha256:"), "{ledger:?}");
        assert!(ledger.content.contains("轮次 1:"), "{ledger:?}");
        assert!(
            collapsed.iter().all(|m| {
                if m.role == Role::Tool {
                    let id = m.tool_call_id.as_deref().unwrap();
                    declared_ids(&collapsed).iter().any(|d| d == id)
                } else {
                    true
                }
            }),
            "no orphan tool results: {collapsed:?}"
        );
        // c3's result stays verbatim in the newest tail.
        assert!(
            collapsed.iter().any(|m| m.content == "页面"),
            "{collapsed:?}"
        );
    }

    #[test]
    fn multi_result_round_registers_one_row_per_call() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![
                ToolCall {
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "a.py"}),
                    call_id: "c1".into(),
                },
                ToolCall {
                    name: "read_file".into(),
                    arguments: serde_json::json!({"path": "b.py"}),
                    call_id: "c2".into(),
                },
            ],
            reasoning_content: None,
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".into(),
            tool_call_id: Some("c1".into()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果B".into(),
            tool_call_id: Some("c2".into()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        messages.extend(round("c3", "read_file", "c.py", "结果C"));
        let collapsed = build_collapsed_request(&messages, 1);
        let ledger = collapsed
            .iter()
            .find(|m| m.content.starts_with(ACTION_LEDGER_PREFIX))
            .expect("ledger block present");
        assert_eq!(ledger.content.matches("轮次 1:").count(), 2, "{ledger:?}");
    }

    #[test]
    fn incomplete_round_is_kept_verbatim() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        // Round 2 declares c2 but the tool result is missing.
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
        });
        let collapsed = build_collapsed_request(&messages, 1);
        assert!(
            collapsed
                .iter()
                .any(|m| m.content.starts_with(ACTION_LEDGER_PREFIX)),
            "the complete older round still collapses: {collapsed:?}"
        );
        assert!(
            collapsed.iter().all(|m| m.content != "结果A"),
            "the complete round's raw result is collapsed into the ledger: {collapsed:?}"
        );
        assert!(
            collapsed
                .iter()
                .any(|m| m.tool_calls.iter().any(|tc| tc.call_id == "c2")),
            "incomplete declaration stays: {collapsed:?}"
        );
        let ledger = collapsed
            .iter()
            .find(|m| m.content.starts_with(ACTION_LEDGER_PREFIX))
            .unwrap();
        assert!(
            !ledger.content.contains("c2"),
            "the incomplete round must not enter the ledger: {ledger:?}"
        );
    }

    #[test]
    fn mid_history_incomplete_round_never_folds() {
        // FUS-LEDGER-FOLD-STATE review fix (2026-08-18): the completeness
        // back-off must cover EVERY collapsed round, not only the boundary
        // round — a mid-history incomplete declaration must stay verbatim
        // (folding it would drop the in-flight call from the model view
        // and register a `no_result` row).
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        // Round 2 (mid-history) declares c2 without a tool result.
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
        });
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        messages.extend(round("c4", "read_file", "d.py", "结果D"));
        let collapsed = build_collapsed_request(&messages, 1);
        let ledger = collapsed
            .iter()
            .find(|m| m.content.starts_with(ACTION_LEDGER_PREFIX))
            .expect("the complete older round still collapses");
        // Only the complete round before the incomplete one may fold; the
        // incomplete declaration and everything after stays verbatim.
        assert!(ledger.content.contains("a.py"), "{ledger:?}");
        assert!(!ledger.content.contains("b.py"), "{ledger:?}");
        assert!(!ledger.content.contains("c3"), "{ledger:?}");
        assert!(
            collapsed
                .iter()
                .any(|m| m.tool_calls.iter().any(|tc| tc.call_id == "c2")),
            "the mid-history incomplete declaration stays: {collapsed:?}"
        );
        assert!(
            collapsed.iter().any(|m| m.content == "页面"),
            "the later complete rounds stay verbatim: {collapsed:?}"
        );
    }

    #[test]
    fn deterministic_output() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "结果A"));
        messages.extend(round("c2", "web_fetch", "https://x.dev", "页面"));
        let a = build_collapsed_request(&messages, 1);
        let b = build_collapsed_request(&messages, 1);
        assert_eq!(a, b);
    }

    // ---- FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26) ----

    #[test]
    fn unfolded_view_is_verbatim() {
        // Before the first fold advance the request view is `messages`
        // unchanged — no per-request stateless collapse (byte-stable
        // prefix from the first round; the old stateless collapse only
        // kicks in once the mechanical trigger advances the fold).
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "search_replace", "b.rs", "已编辑"));
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        let fold = LedgerFoldState::default();
        assert_eq!(build_request_view(&messages, &fold, 1_000_000), messages);
        assert!(!fold.is_folded());
    }

    // ---- FUS-LEDGER-FOLD-STATE external-file design (2026-08-18,
    // ADR-0010 §14.28) ----

    #[test]
    fn first_advance_folds_old_rounds_behind_fixed_pointer() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "search_replace", "b.rs", "已编辑"));
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        messages.extend(round("c4", "read_file", "c.py", "内容C"));
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        let mut fold = LedgerFoldState::default();
        // 预算恰容纳最新一轮（c4）：c1–c3 折进台账，桥 = c4。
        let rows = advance_fold(&messages, &mut fold, est_budget(&messages, 7), ledger_path)
            .expect("first advance folds old rounds");
        assert!(fold.is_folded());
        assert_eq!(fold.fold_start, Some(1), "first tool declaration index");
        // kept_start = the start of the newest tail=1 round (c4 at idx 7).
        assert_eq!(fold.fold_cut, Some(7));
        assert_eq!(rows.len(), 3, "rows for rounds c1/c2/c3");
        assert!(
            rows.iter()
                .any(|r| r.tool == "read_file" && r.target == "a.py")
        );
        assert!(rows.iter().any(|r| r.tool == "search_replace"));
        assert!(
            rows.iter()
                .any(|r| r.tool == "web_fetch" && r.target == "https://x.dev")
        );
        // folded_ledger = the byte-fixed pointer message, NOT the rows.
        let pointer = fold.folded_ledger.as_deref().unwrap();
        assert!(pointer.starts_with(LEDGER_FOLD_POINTER_PREFIX), "{pointer}");
        assert!(pointer.contains(".gsa/ledger/current.md"), "{pointer}");
        assert!(
            !pointer.contains("read_file"),
            "rows never enter the view: {pointer}"
        );

        let view = build_request_view(&messages, &fold, est_budget(&messages, 7));
        assert_eq!(view[0].content, "任务");
        assert_eq!(
            view[1].content, pointer,
            "the view carries the fixed pointer message"
        );
        // The retained region starts at fold_cut — c4 stays verbatim; the
        // folded rounds' raw results are gone from the view.
        assert!(view.iter().any(|m| m.content == "内容C"), "{view:?}");
        assert!(view.iter().all(|m| m.content != "内容A"), "{view:?}");
        assert!(view.iter().all(|m| m.content != "页面"), "{view:?}");
    }

    #[test]
    fn advance_is_anti_spin_without_new_rounds() {
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "read_file", "b.py", "内容B"));
        messages.extend(round("c3", "read_file", "c.py", "内容C"));
        let mut fold = LedgerFoldState::default();
        // 预算恰容纳最新两轮（c2+c3）：c1 折进台账。
        assert!(
            advance_fold(&messages, &mut fold, est_budget(&messages, 3), ledger_path).is_some()
        );
        let before = fold.clone();
        // No new complete round outside the tail — no-op.
        assert!(
            advance_fold(&messages, &mut fold, est_budget(&messages, 3), ledger_path).is_none()
        );
        assert_eq!(fold, before);
        // 仅 1 个完整轮：桥恒含 ≥1 轮结构、无可折叠轮 — no-op。
        let mut sparse = vec![msg(Role::User, "任务")];
        sparse.extend(round("c1", "read_file", "a.py", "内容A"));
        let mut sparse_fold = LedgerFoldState::default();
        assert!(advance_fold(&sparse, &mut sparse_fold, 1_000_000, ledger_path).is_none());
        assert!(!sparse_fold.is_folded());
    }

    #[test]
    fn advance_returns_only_newly_folded_rows_and_keeps_pointer() {
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "read_file", "b.py", "B"));
        messages.extend(round("c3", "read_file", "c.py", "C"));
        messages.extend(round("c4", "read_file", "d.py", "D"));
        let mut fold = LedgerFoldState::default();
        // 预算恰容纳 c3+c4：首次推进折 c1/c2；预算先取一次、跨追加复用
        // （第二次推进前 messages 已含 c5/c6，重算会把预算放大到整段）。
        let budget = est_budget(&messages, 5);
        let first = advance_fold(&messages, &mut fold, budget, ledger_path).unwrap();
        assert_eq!(first.len(), 2, "rounds c1/c2");
        assert!(
            first
                .iter()
                .all(|r| r.target == "a.py" || r.target == "b.py")
        );
        let pointer = fold.folded_ledger.clone().unwrap();

        // Two more complete rounds: the next advance returns ONLY the
        // newly folded c3/c4 rows — the pointer is never rewritten.
        messages.extend(round("c5", "read_file", "e.py", "E"));
        messages.extend(round("c6", "read_file", "f.py", "F"));
        let second = advance_fold(&messages, &mut fold, budget, ledger_path).unwrap();
        assert_eq!(second.len(), 2, "rounds c3/c4 only");
        assert!(
            second
                .iter()
                .all(|r| r.target == "c.py" || r.target == "d.py")
        );
        assert!(
            second
                .iter()
                .all(|r| r.target != "a.py" && r.target != "b.py"),
            "already-written rows are never returned again"
        );
        assert_eq!(
            fold.folded_ledger.as_deref(),
            Some(pointer.as_str()),
            "pointer message byte-identical across advances"
        );
        assert!(fold.fold_cut.is_some_and(|cut| cut > 7));
    }

    #[test]
    fn folded_view_prefix_is_byte_stable_across_advances() {
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        let mut messages = vec![msg(Role::User, "任务")];
        for k in 1..=4 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &format!("内容{k}"),
            ));
        }
        let mut fold = LedgerFoldState::default();
        let budget = est_budget(&messages, 7);
        assert!(advance_fold(&messages, &mut fold, budget, ledger_path).is_some());
        let view1 = build_request_view(&messages, &fold, budget);
        // Preamble + fixed pointer: byte-stable across ALL advances.
        let prefix_len = fold.fold_start.unwrap() + 1;
        let prefix1: Vec<Message> = view1.iter().take(prefix_len).cloned().collect();
        let mut prev_view = view1;
        for k in 5..=10 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &format!("内容{k}"),
            ));
            let advanced = advance_fold(&messages, &mut fold, budget, ledger_path).is_some();
            let view = build_request_view(&messages, &fold, budget);
            let prefix: Vec<Message> = view.iter().take(prefix_len).cloned().collect();
            assert_eq!(prefix, prefix1, "folded prefix rewritten at round {k}");
            if advanced {
                // A mechanical advance trims the tail to the new cut (the
                // round that just crossed the line) — the only accepted
                // view rewrite, beyond the never-changing pointer.
                assert!(
                    view.len() <= prev_view.len(),
                    "advance must not grow the tail: round {k}"
                );
            } else {
                // Between advances the view is pure append of the previous.
                assert_eq!(
                    &view[..prev_view.len()],
                    prev_view.as_slice(),
                    "view rewritten outside an advance at round {k}"
                );
            }
            prev_view = view;
        }
    }

    #[test]
    fn external_file_append_continues_sequence_across_windows() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        let rows = |targets: &[&str]| {
            targets
                .iter()
                .enumerate()
                .map(|(i, t)| ActionLedgerRow {
                    round_index: i,
                    tool: "read_file".to_string(),
                    target: (*t).to_string(),
                    pointer: format!("sha256:{i:064x}"),
                    final_reply: format!("已读 {t}"),
                })
                .collect::<Vec<_>>()
        };
        append_ledger_rows(&path, &rows(&["a.py", "b.py", "c.py"])).unwrap();
        append_ledger_rows(&path, &rows(&["d.py", "e.py"])).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(
            lines[0].starts_with("[1] 轮次 1: read_file 目标=a.py"),
            "{}",
            lines[0]
        );
        assert!(
            lines[1].starts_with("[2] 轮次 2: read_file 目标=b.py"),
            "{}",
            lines[1]
        );
        assert!(
            lines[3].starts_with("[4] 轮次 1: read_file 目标=d.py"),
            "{}",
            lines[3]
        );
        assert!(
            lines[4].starts_with("[5] 轮次 2: read_file 目标=e.py"),
            "{}",
            lines[4]
        );
        assert!(lines[0].contains("结果=sha256:") && lines[0].contains("最终回复=已读 a.py"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fold_reset_continues_external_file_sequence() {
        // Compaction semantics: fold_state.reset() clears the in-memory
        // fold, but the external file is append-only and its seqs continue
        // — a later advance (fresh window after the marker) numbers from
        // the file tail, NOT from 1 again (跨压缩连续).
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-reset-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        let mut messages = vec![msg(Role::User, "任务")];
        for k in 1..=3 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &format!("内容{k}"),
            ));
        }
        let mut fold = LedgerFoldState::default();
        let rows = advance_fold(&messages, &mut fold, est_budget(&messages, 5), &path).unwrap();
        append_ledger_rows(&path, &rows).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 2);

        // Compaction: messages truncated (marker replaces the folded
        // region) and the fold state resets — a brand-new window.
        fold.reset();
        let mut fresh = vec![
            msg(Role::User, "任务"),
            msg(Role::User, "[前文上下文已压缩 v0.2] 摘要"),
        ];
        for k in 10..=12 {
            fresh.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &format!("内容{k}"),
            ));
        }
        let mut fresh_fold = LedgerFoldState::default();
        let rows2 = advance_fold(&fresh, &mut fresh_fold, est_budget(&fresh, 5), &path).unwrap();
        append_ledger_rows(&path, &rows2).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "file is append-only across compaction");
        assert!(lines[2].starts_with("[3]"), "{}", lines[2]);
        assert!(lines[2].contains("目标=10.py"), "{}", lines[2]);
        assert!(
            lines[3].starts_with("[4] 轮次 2: read_file 目标=11.py"),
            "{}",
            lines[3]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：`[seq]`（per-row 全局序号）与 `轮次`（窗口内
    /// round_index+1）解耦——多工具轮各行轮次标注一致，跨窗口时全局序号
    /// 继续而轮次从 1 重新计数。
    #[test]
    fn external_row_round_label_uses_round_index_not_global_seq() {
        let row = ActionLedgerRow {
            round_index: 2,
            tool: "read_file".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:ab".to_string(),
            final_reply: "已读".to_string(),
        };
        let line = external_row_line(7, &row);
        assert!(
            line.starts_with("[7] 轮次 3: read_file 目标=a.py"),
            "{line}"
        );
        // 同一轮的第二条（同 round_index）全局序号 +1、轮次不变。
        let line2 = external_row_line(8, &row);
        assert!(
            line2.starts_with("[8] 轮次 3: read_file 目标=a.py"),
            "{line2}"
        );
    }

    /// 2026-08-18 审查修复：尾行超过 4KiB 时 `tail_seq` 仍取到完整末行，
    /// 续号不因截断而重置（此前 4KiB 尾窗会静默从 1 重新编号）。
    /// 2026-08-18 S4 复验修复后：`external_row_line` 对嵌入字段做换行转义 +
    /// 300 字符上限，正常追加路径不再产生超长行；本测试改为直接写盘一条
    /// >4KiB 单行记录，保留对 `tail_seq` 尾窗逐级倍增路径的防御覆盖。
    #[test]
    fn tail_seq_survives_long_last_row() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-long-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // 单行 >4KiB：尾窗必须逐级倍增取到完整末行，不得截断续号。
        let long_row = format!(
            "[41] 轮次 7: run_terminal 目标=make 结果=sha256:1 最终回复={}",
            "x".repeat(36_000)
        );
        std::fs::write(&path, long_row).unwrap();
        assert_eq!(
            tail_seq(&path).unwrap(),
            41,
            "long single-line row keeps its seq"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 S4 复验修复：嵌入字段（目标/结果/最终回复）中的换行必须
    /// 转义——多行最终回复会把一条逻辑记录拆成多个物理行，`tail_seq` 将续行
    /// 当损坏尾巴（InvalidData）导致追加失败、连续 3 次后折叠被禁用。单行
    /// 契约是「追加续号」正确性的前提；字段上限符合设计「摘要行 ~200–350
    /// 字符/行」。
    #[test]
    fn external_row_line_escapes_embedded_newlines_and_caps() {
        let row = ActionLedgerRow {
            round_index: 3,
            tool: "blackboard_read".to_string(),
            target: "多行\n目标".to_string(),
            pointer: "sha256:ab".to_string(),
            final_reply: "[blackboard_read] == registration ==\nstep1: build\nstep2: verify\n"
                .to_string(),
        };
        let line = external_row_line(31, &row);
        assert_eq!(
            line.lines().count(),
            1,
            "row must be a single physical line: {line}"
        );
        assert!(
            line.starts_with("[31] 轮次 4: blackboard_read 目标=多行\\n目标"),
            "{line}"
        );
        assert!(
            line.contains(
                "最终回复=[blackboard_read] == registration ==\\nstep1: build\\nstep2: verify\\n"
            ),
            "{line}"
        );
        // 字段上限：>300 字符截断并以 … 标记。
        let row2 = ActionLedgerRow {
            round_index: 4,
            tool: "grep".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:cd".to_string(),
            final_reply: "很长".repeat(500),
        };
        let line2 = external_row_line(32, &row2);
        assert_eq!(line2.lines().count(), 1);
        assert!(
            line2.ends_with('…'),
            "truncated field ends with marker: {line2}"
        );
        assert!(
            line2.chars().count() < 400,
            "row bounded by field cap + prefix: {}",
            line2.chars().count()
        );
    }

    /// 2026-08-18 S4 复验回归：多行最终回复的批次写入后，下一次追加必须
    /// 继续续号（`tail_seq` 不被续行误导），文件保持一行一条逻辑记录。
    #[test]
    fn append_ledger_rows_continues_after_multiline_reply() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-multiline-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        let batch1 = vec![
            ActionLedgerRow {
                round_index: 26,
                tool: "blackboard_read".to_string(),
                target: "（无）".to_string(),
                pointer: "sha256:1".to_string(),
                final_reply: "[blackboard_read] == registration ==\nstep1: build\nstep2: verify"
                    .to_string(),
            },
            ActionLedgerRow {
                round_index: 27,
                tool: "run_terminal".to_string(),
                target: "make".to_string(),
                pointer: "sha256:2".to_string(),
                final_reply: "exit: 0\ncompiled ok".to_string(),
            },
        ];
        append_ledger_rows(&path, &batch1).unwrap();
        let batch2 = vec![ActionLedgerRow {
            round_index: 28,
            tool: "read_file".to_string(),
            target: "vm.js".to_string(),
            pointer: "sha256:3".to_string(),
            final_reply: "ok".to_string(),
        }];
        append_ledger_rows(&path, &batch2).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3, "one physical line per logical row:\n{text}");
        assert!(
            lines[2].starts_with("[3] 轮次 29: read_file 目标=vm.js"),
            "续号必须从文件尾行继续: {}",
            lines[2]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：非空文件尾行无 `[seq]` 视为损坏——返回错误让
    /// 调用方回滚并降级，而不是静默从 1 重新编号造成重复序号。
    #[test]
    fn tail_seq_rejects_unparseable_tail() {
        let dir = std::env::temp_dir().join(format!(
            "orz-ledger-ext-corrupt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let path = ledger_file_path(&dir);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "不是台账行的垃圾内容\n").unwrap();
        let row = ActionLedgerRow {
            round_index: 0,
            tool: "read_file".to_string(),
            target: "a.py".to_string(),
            pointer: "sha256:1".to_string(),
            final_reply: "已读".to_string(),
        };
        assert!(
            append_ledger_rows(&path, std::slice::from_ref(&row)).is_err(),
            "corrupt tail must fail loudly, not restart numbering at 1"
        );
        // 空文件（或仅空行）仍从 1 开始。
        let empty = ledger_file_path(&dir.join("empty"));
        append_ledger_rows(&empty, std::slice::from_ref(&row)).unwrap();
        let text = std::fs::read_to_string(&empty).unwrap();
        assert!(
            text.starts_with("[1] 轮次 1: read_file 目标=a.py"),
            "{text}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 2026-08-18 审查修复：`safe_fold_cut` 防御在推进侧生效——最新保留轮
    /// 不完整（声明无回复）时 cut 回退到最近平衡轮起点，未折叠轮次留在
    /// 原文视图、不入外挂文件，行与视图一致。
    #[test]
    fn advance_fold_defers_cut_to_last_balanced_round() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "read_file", "b.py", "B"));
        // c3 轮不完整：assistant 声明无对应 tool 回复。
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".to_string(),
                arguments: serde_json::json!({"path": "c.py"}),
                call_id: "c3".to_string(),
            }],
            reasoning_content: None,
        });
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        let mut fold = LedgerFoldState::default();
        // 预算恰容纳 c2+c3（c3 为不完整声明）：行走会停在 c1 之后。
        let rows =
            advance_fold(&messages, &mut fold, est_budget(&messages, 3), ledger_path).unwrap();
        assert_eq!(rows.len(), 1, "只有 c1 被折叠");
        assert_eq!(fold.fold_cut, Some(3), "cut 回退到 c2 轮起点");
        let view = build_request_view(&messages, &fold, est_budget(&messages, 3));
        assert!(
            view.iter().any(|m| m.content == "B"),
            "c2 原文保留: {view:?}"
        );
        assert!(
            view.iter()
                .any(|m| m.tool_calls.iter().any(|t| t.call_id == "c3")),
            "不完整的 c3 声明保留在视图: {view:?}"
        );
        assert!(
            view.iter().all(|m| m.content != "A"),
            "c1 内容折叠出视图: {view:?}"
        );
    }

    /// 2026-08-18 审查修复：preamble 边界校验在推进侧生效——`fold_start`
    /// 过期（末条为 assistant 声明）时放弃推进，折叠状态保持不动，不落行。
    #[test]
    fn advance_fold_abandons_when_preamble_ends_with_declaration() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "read_file", "b.py", "B"));
        messages.extend(round("c3", "read_file", "c.py", "C"));
        // 过期 fold_start=2：messages[..2] 末条为 c1 声明（其回复在折叠区）。
        let mut fold = LedgerFoldState {
            fold_start: Some(2),
            fold_cut: None,
            folded_ledger: None,
            ..Default::default()
        };
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        assert!(
            advance_fold(&messages, &mut fold, 1_000_000, ledger_path).is_none(),
            "preamble 边界不合格必须放弃推进"
        );
        assert_eq!(fold.fold_start, Some(2), "状态保持不动");
        assert_eq!(fold.fold_cut, None, "状态保持不动");
        assert!(fold.folded_ledger.is_none(), "状态保持不动");
    }

    // ---- FUS-LEDGER-FOLD-BRIDGE (2026-08-19, ADR-0010 §14.32) ----

    #[test]
    fn bridge_cut_walks_newest_first_until_budget() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "run_terminal", "make", &"out".repeat(200)));
        messages.extend(round("c3", "read_file", "c.py", "C"));
        // 预算恰容纳 c2+c3：从最新往回累加，c1 必须折进台账。
        let cut = bridge_cut(&messages, est_budget(&messages, 3)).unwrap();
        assert_eq!(cut, 3, "cut = c2 轮起点（c2+c3 入桥）");
    }

    #[test]
    fn bridge_cut_keeps_newest_round_when_alone_exceeds_budget() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "run_terminal", "make", &"x".repeat(20_000)));
        // 极小预算：最新一轮单独超预算仍恒入桥，更早轮照常折叠。
        let cut = bridge_cut(&messages, 10).unwrap();
        assert_eq!(cut, 3, "c2 单独入桥、c1 折叠");
    }

    #[test]
    fn bridge_cut_defers_to_first_incomplete_round() {
        // c1 巨大（单独占满预算）→ 预算行走会折掉 c1 保留 c2/c3；
        // c2 不完整 → cut 必须停在 c2 轮起点，不得把不完整轮折进台账。
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "run_terminal", "make", &"x".repeat(20_000)));
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "b.py"}),
                call_id: "c2".into(),
            }],
            reasoning_content: None,
        });
        messages.extend(round("c3", "read_file", "c.py", "C"));
        let cut = bridge_cut(&messages, 10).unwrap();
        assert_eq!(cut, 3, "cut = c2 轮起点（不完整轮永不折叠）");
    }

    #[test]
    fn fold_tail_estimate_budget_converts_tokens_to_chars_half() {
        // 默认 8K 真实 token → 16K 字符 → 8K 估计口径（chars/2）——
        // S4 实测校准（2026-08-19）：≈ 2 字符/真实 token，估计口径 ≈ 真实
        // token（path-tracing 07:08 运行，桥 12,948 字符 → 重付 6,493）。
        assert_eq!(fold_tail_estimate_budget(8_000), 8_000);
        assert_eq!(fold_tail_estimate_budget(1), 1);
        assert_eq!(fold_tail_estimate_budget(0), 0);
        assert_eq!(
            fold_tail_estimate_budget(u64::MAX),
            u64::MAX / 2,
            "饱和乘法后 ÷2"
        );
    }

    #[test]
    fn pointer_message_mentions_8k_bridge() {
        let pointer = build_pointer_message(std::path::Path::new(".gsa/ledger/current.md"));
        assert!(pointer.starts_with(LEDGER_FOLD_POINTER_PREFIX), "{pointer}");
        assert!(pointer.contains(".gsa/ledger/current.md"), "{pointer}");
        assert!(pointer.contains("约 8K 桥接内容"), "{pointer}");
        assert!(
            pointer.contains("更早轮次已按行归档于 .gsa/ledger/current.md"),
            "指针文案与设计 §3.4 定稿措辞一致（审查处理 N1）: {pointer}"
        );
        assert!(!pointer.contains("归档在该文件中"), "{pointer}");
        assert!(!pointer.contains("最近 1 轮原文"), "{pointer}");
    }

    #[test]
    fn bridge_view_strips_reasoning_and_keeps_structure() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: "进行中".to_string(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "a.py"}),
                call_id: "c1".into(),
            }],
            reasoning_content: Some("思考链内容".to_string()),
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".to_string(),
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        assert_eq!(view.len(), 4, "U0 + pointer + A + T");
        let decl = &view[2];
        assert_eq!(decl.role, Role::Assistant);
        assert!(
            decl.reasoning_content.is_none(),
            "思维链必须从桥剔除: {decl:?}"
        );
        assert_eq!(decl.content, "进行中", "声明可见 content 完整保留");
        assert_eq!(decl.tool_calls.len(), 1);
        assert_eq!(decl.tool_calls[0].call_id, "c1", "tool_calls 完整保留");
        assert_eq!(view[3].content, "结果A", "预算内工具回复原文保留");
        assert_eq!(
            messages[1].reasoning_content.as_deref(),
            Some("思考链内容"),
            "视图变换不得修改 messages 本身"
        );
        let declared = declared_ids(&view[2..]);
        let replied: Vec<&str> = view[2..]
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert!(declared.iter().all(|d| replied.contains(&d.as_str())));
    }

    #[test]
    fn bridge_within_budget_is_verbatim_except_reasoning() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "a.py"}),
                call_id: "c1".into(),
            }],
            reasoning_content: Some("思考链".to_string()),
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".to_string(),
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        let expected: Vec<Message> = messages[1..]
            .iter()
            .cloned()
            .map(|mut m| {
                m.reasoning_content = None;
                m
            })
            .collect();
        assert_eq!(
            &view[2..],
            &expected[..],
            "预算内桥 = 原文（仅剔除声明消息思维链）"
        );
    }

    /// THIN-HARNESS-REDESIGN-V2 §9.6 (2026-08-29 S5-1 修复 A 回归):
    /// 桥内**纯文本** assistant 消息（无 tool_calls）保留
    /// `reasoning_content`——DeepSeek /responses 要求非空 reasoning 原样
    /// 回传，orientation 纯文本回答被 fold 落入桥内末条剥除 → 400；
    /// 声明消息（带 tool_calls）仍剔除（思维链不进桥）。
    #[test]
    fn bridge_keeps_plain_text_assistant_reasoning_but_strips_declarations() {
        let mut messages = vec![msg(Role::User, "任务")];
        // orientation 纯文本回答形态：reasoning 必须保留。
        messages.push(Message {
            role: Role::Assistant,
            content: "当前在编译 make-doom，继续".to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: Some("方向检查思考链".to_string()),
        });
        // 声明消息（带 tool_calls）：reasoning 仍剔除。
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "a.py"}),
                call_id: "c1".into(),
            }],
            reasoning_content: Some("声明思考链".to_string()),
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".to_string(),
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 1_000_000);
        let plain = view
            .iter()
            .find(|m| m.role == Role::Assistant && m.tool_calls.is_empty())
            .expect("plain-text assistant message survives");
        assert_eq!(
            plain.reasoning_content.as_deref(),
            Some("方向检查思考链"),
            "桥内纯文本 assistant 消息必须保留 reasoning_content: {plain:?}"
        );
        let decl = view
            .iter()
            .find(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
            .expect("declaration survives");
        assert!(
            decl.reasoning_content.is_none(),
            "声明消息思维链仍必须从桥剔除: {decl:?}"
        );
        assert_eq!(
            messages[1].reasoning_content.as_deref(),
            Some("方向检查思考链"),
            "视图变换不得修改 messages 本身"
        );
    }

    #[test]
    fn bridge_view_truncates_over_budget_tool_reply_tail_with_pointer() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "a.py"}),
                call_id: "c1".into(),
            }],
            reasoning_content: Some("思考链".to_string()),
        });
        messages.push(Message {
            role: Role::Tool,
            content: "原始输出".repeat(5_000), // 2 万字符 ≈ 1 万估计 token
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        // 预算 200 估计 token：整桥必然超限，工具回复必须截断到预算内。
        let budget = 200;
        let view = build_request_view(&messages, &fold, budget);
        let tool = view.iter().find(|m| m.role == Role::Tool).unwrap();
        assert!(tool.content.starts_with("…（前略）\n"), "{tool:?}");
        assert!(
            tool.content.contains("完整内容见 日志/对话存档 sha256:"),
            "{tool:?}"
        );
        assert!(
            tool.content.contains("输出"),
            "保留尾部（最新内容）: {tool:?}"
        );
        let bridge_estimate = crate::controller::estimate_messages_tokens(&view[2..]);
        assert!(
            bridge_estimate <= budget,
            "桥估计必须落入预算: {bridge_estimate}"
        );
        // 配对不变量不因截断破坏；源 messages 不动。
        let declared = declared_ids(&view[2..]);
        let replied: Vec<&str> = view[2..]
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert!(declared.iter().all(|d| replied.contains(&d.as_str())));
        assert_eq!(messages[2].content, "原始输出".repeat(5_000));
    }

    /// OUTPUT-DEGENERATION-GUARD (2026-08-19, ADR-0010 §14.33 / 设计 §3.2):
    /// 源工具回复已含终端截断的落盘指针（「完整内容见 <路径>，请使用
    /// read_file」）时，桥截断保留尾部即天然留存该行——不再追加 sha256
    /// 兜底（信息损失可经 read_file 恢复）。
    #[test]
    fn bridge_truncation_keeps_tool_result_read_back_pointer_without_sha256() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "run_terminal_cmd".into(),
                arguments: serde_json::json!({"command": "cat big"}),
                call_id: "c1".into(),
            }],
            reasoning_content: None,
        });
        // 模拟终端 8K 截断后的工具结果：长输出 + 末尾机械指针行。
        messages.push(Message {
            role: Role::Tool,
            content: format!(
                "{}完整内容见 .gsa/session/terminal/ord-1.log，请使用 read_file 读取（大文件用 offset/limit 分页）",
                "大段输出".repeat(3_000)
            ),
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 200);
        let tool = view.iter().find(|m| m.role == Role::Tool).unwrap();
        assert!(tool.content.starts_with("…（前略）\n"), "{tool:?}");
        assert!(
            tool.content.contains(".gsa/session/terminal/ord-1.log"),
            "尾部必须保留工具结果自身落盘指针: {tool:?}"
        );
        assert!(
            tool.content
                .ends_with("请使用 read_file 读取（大文件用 offset/limit 分页）"),
            "read_file 指引必须留存: {tool:?}"
        );
        assert!(
            !tool.content.contains("sha256:"),
            "已有落盘指针时不得追加 sha256 兜底: {tool:?}"
        );
        let bridge_estimate = crate::controller::estimate_messages_tokens(&view[2..]);
        assert!(
            bridge_estimate <= 200,
            "桥估计必须落入预算: {bridge_estimate}"
        );
    }

    #[test]
    fn bridge_view_truncates_assistant_final_reply_head_style() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.push(Message {
            role: Role::Assistant,
            content: String::new(),
            tool_call_id: None,
            tool_calls: vec![ToolCall {
                name: "read_file".into(),
                arguments: serde_json::json!({"path": "a.py"}),
                call_id: "c1".into(),
            }],
            reasoning_content: None,
        });
        messages.push(Message {
            role: Role::Tool,
            content: "结果A".to_string(),
            tool_call_id: Some("c1".to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        // 轮内 assistant 最终回复文本（无 tool_calls）超限。
        messages.push(Message {
            role: Role::Assistant,
            content: "最终回复".repeat(200), // 600 字符
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
        });
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 200);
        let final_text = view
            .iter()
            .find(|m| m.role == Role::Assistant && m.tool_calls.is_empty())
            .expect("final reply message survives");
        assert_eq!(final_text.content.chars().count(), 200, "{final_text:?}");
        assert!(final_text.content.starts_with("最终回复"), "{final_text:?}");
        assert!(final_text.content.ends_with('…'), "{final_text:?}");
        assert!(
            crate::controller::estimate_messages_tokens(&view[2..]) <= 200,
            "桥估计必须落入预算"
        );
    }

    #[test]
    fn bridge_truncation_preserves_role_order_and_pairing() {
        let mut messages = vec![msg(Role::User, "任务")];
        for k in 1..=3 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &"大".repeat(2_000),
            ));
        }
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let budget = 600;
        let view = build_request_view(&messages, &fold, budget);
        let roles: Vec<Role> = view.iter().map(|m| m.role).collect();
        assert_eq!(
            roles,
            vec![
                Role::User,
                Role::User,
                Role::Assistant,
                Role::Tool,
                Role::Assistant,
                Role::Tool,
                Role::Assistant,
                Role::Tool,
            ],
            "消息不删、轮不拆、角色顺序保持: {view:?}"
        );
        let declared = declared_ids(&view[2..]);
        let replied: Vec<&str> = view[2..]
            .iter()
            .filter(|m| m.role == Role::Tool)
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert!(declared.iter().all(|d| replied.contains(&d.as_str())));
        for m in view.iter().filter(|m| m.role == Role::Tool) {
            assert!(m.content.starts_with("…（前略）\n"), "{m:?}");
            assert!(m.content.contains("sha256:"), "{m:?}");
        }
        for m in view
            .iter()
            .filter(|m| m.role == Role::Assistant && !m.tool_calls.is_empty())
        {
            assert_eq!(m.content, "", "声明可见 content 保留原样: {m:?}");
            assert!(!m.tool_calls.is_empty());
        }
        let bridge_estimate = crate::controller::estimate_messages_tokens(&view[2..]);
        assert!(
            bridge_estimate <= budget,
            "桥估计必须落入预算: {bridge_estimate}"
        );
    }

    #[test]
    fn bridge_truncation_stops_at_physical_floor_without_spinning() {
        // 预算低于最小形态的物理下限（消息不可删/轮不可拆）：循环必须终止
        // （不得同档位无限重截），桥停在最小形态、估计略超预算为已接受边界。
        let mut messages = vec![msg(Role::User, "任务")];
        for k in 1..=3 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &"大".repeat(2_000),
            ));
        }
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(messages.len()),
        };
        let view = build_request_view(&messages, &fold, 100);
        assert_eq!(
            view.iter().filter(|m| m.role == Role::Tool).count(),
            3,
            "消息不删、轮不拆"
        );
        for m in view.iter().filter(|m| m.role == Role::Tool) {
            assert!(m.content.starts_with("…（前略）\n"), "{m:?}");
            assert!(m.content.contains("sha256:"), "{m:?}");
        }
    }

    #[test]
    fn bridge_truncation_only_touches_frozen_bridge_and_appended_rounds_stay_verbatim() {
        // 推进时冻结桥（`[fold_cut..bridge_end)`）；此后追加的轮次不属于桥
        // ——即使整段（桥 + 追加）超过预算，桥也绝不因追加内容被逐请求重截
        // （前缀字节稳定、折叠之间纯追加——缓存纪律），追加轮原样保留。
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "run_terminal", "make", &"大".repeat(5_000)));
        let bridge_end = messages.len(); // 推进时冻结点
        messages.extend(round("c2", "read_file", "b.py", &"追加".repeat(3_000)));
        let fold = LedgerFoldState {
            fold_start: Some(1),
            fold_cut: Some(1),
            folded_ledger: Some("ledger".to_string()),
            bridge_end: Some(bridge_end),
        };
        let budget = 200;
        let view = build_request_view(&messages, &fold, budget);
        let tool_c1 = view
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("c1"))
            .expect("c1 tool reply survives");
        assert!(
            tool_c1.content.starts_with("…（前略）\n"),
            "桥内工具回复被截断到预算: {tool_c1:?}"
        );
        let tool_c2 = view
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("c2"))
            .expect("c2 tool reply survives");
        assert_eq!(
            tool_c2.content,
            "追加".repeat(3_000),
            "桥外追加轮原样保留（不被重截）: {tool_c2:?}"
        );
        // 桥（视图索引 2..2+(bridge_end-cut)）的估计 ≤ 预算；追加轮不计入。
        let bridge_estimate =
            crate::controller::estimate_messages_tokens(&view[2..2 + bridge_end - 1]);
        assert!(
            bridge_estimate <= budget,
            "桥估计必须落入预算: {bridge_estimate}"
        );
    }
}
