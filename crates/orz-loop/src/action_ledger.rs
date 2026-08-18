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
/// request view (`[U0][preamble][pointer][最近 1 轮原文]`). The message is
/// byte-fixed for the session — the external ledger file is the only
/// growing part, so the request prefix stays byte-stable across advances
/// (the per-window rewrite is gone; measured hit rate 81.9% → ~91–93%).
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
         grep <工具名> {}。当前会话仅保留最近 1 轮原文，更早内容按行归档在该文件中。",
        ledger_path.display(),
        ledger_path.display(),
        ledger_path.display(),
    )
}

/// One external-file row: `[<全局序号>] 轮次 <窗口内轮次>: <工具> 目标=…
/// 结果=… 最终回复=…`。`[<全局序号>]` 为按行续点的 per-row 全局序号
/// （跨压缩连续；多结果轮按条各占一行、序号连续递增），文件只增，读尾行
/// 即可续号；`轮次` 沿用现有台账行语义 = `round_index + 1`（窗口内 0 基
/// 声明序号 +1，跨压缩重置，与 `build_ledger_block` 标注一致）——2026-08-18
/// 审查修复：此前两者共用一个 per-row 序号，多工具轮会把轮次标错。
pub fn external_row_line(seq: u64, row: &ActionLedgerRow) -> String {
    format!(
        "[{seq}] 轮次 {}: {} 目标={} 结果={} 最终回复={}",
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
///   file) + `messages[fold_cut..]` (recent tail verbatim).
///
/// The source `messages` slice is never modified — the journal/sidecar
/// keeps the complete tool records (audit dual-track unchanged).
pub fn build_request_view(messages: &[Message], fold: &LedgerFoldState) -> Vec<Message> {
    let (Some(fold_start), Some(fold_cut)) = (fold.fold_start, fold.fold_cut) else {
        return messages.to_vec();
    };
    let Some(ledger) = fold.folded_ledger.as_deref() else {
        return messages.to_vec();
    };
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
    view.extend_from_slice(&messages[cut..]);
    view
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
/// external-file design): advance the fold point — fold the NEW complete
/// old rounds before the `keep_recent_rounds` tail and move `fold_cut`
/// forward.
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
/// fold — i.e. fewer than `keep_recent_rounds + 1` complete rounds, the
/// recomputed cut does not move past the current `fold_cut` (nothing new
/// outside the tail since the last advance), or the same preamble/cut
/// safety checks that make `build_request_view` abandon the fold fire
/// (stale fold indices / unbalanced rounds — rows must never be appended
/// for rounds the view would keep verbatim).
pub fn advance_fold(
    messages: &[Message],
    fold: &mut LedgerFoldState,
    keep_recent_rounds: usize,
    ledger_path: &std::path::Path,
) -> Option<Vec<ActionLedgerRow>> {
    let kept_start = collapsed_cut(messages, keep_recent_rounds)?;
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
        };
        let view = build_request_view(&messages, &fold);
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
        };
        let view = build_request_view(&messages, &fold);
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
        };
        let view = build_request_view(&messages, &fold);
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
        assert_eq!(build_request_view(&messages, &fold), messages);
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
        let rows = advance_fold(&messages, &mut fold, 1, ledger_path)
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

        let view = build_request_view(&messages, &fold);
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
        assert!(advance_fold(&messages, &mut fold, 2, ledger_path).is_some());
        let before = fold.clone();
        // No new complete round outside the tail — no-op.
        assert!(advance_fold(&messages, &mut fold, 2, ledger_path).is_none());
        assert_eq!(fold, before);
        // Fewer than tail+1 complete rounds — no-op.
        let mut sparse = vec![msg(Role::User, "任务")];
        sparse.extend(round("c1", "read_file", "a.py", "内容A"));
        let mut sparse_fold = LedgerFoldState::default();
        assert!(advance_fold(&sparse, &mut sparse_fold, 2, ledger_path).is_none());
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
        let first = advance_fold(&messages, &mut fold, 2, ledger_path).unwrap();
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
        let second = advance_fold(&messages, &mut fold, 2, ledger_path).unwrap();
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
        assert!(advance_fold(&messages, &mut fold, 1, ledger_path).is_some());
        let view1 = build_request_view(&messages, &fold);
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
            let advanced = advance_fold(&messages, &mut fold, 1, ledger_path).is_some();
            let view = build_request_view(&messages, &fold);
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
        let rows = advance_fold(&messages, &mut fold, 1, &path).unwrap();
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
        let rows2 = advance_fold(&fresh, &mut fresh_fold, 1, &path).unwrap();
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
        let long_reply = "很长的最终回复".repeat(3_000); // ~36KB > 4KiB 尾窗
        let batch1 = vec![
            ActionLedgerRow {
                round_index: 0,
                tool: "read_file".to_string(),
                target: "a.py".to_string(),
                pointer: "sha256:1".to_string(),
                final_reply: "短回复".to_string(),
            },
            ActionLedgerRow {
                round_index: 1,
                tool: "run_tests".to_string(),
                target: "make-doom".to_string(),
                pointer: "sha256:2".to_string(),
                final_reply: long_reply,
            },
        ];
        append_ledger_rows(&path, &batch1).unwrap();
        let batch2 = vec![ActionLedgerRow {
            round_index: 2,
            tool: "read_file".to_string(),
            target: "c.py".to_string(),
            pointer: "sha256:3".to_string(),
            final_reply: "短回复".to_string(),
        }];
        append_ledger_rows(&path, &batch2).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(
            lines[2].starts_with("[3] 轮次 3: read_file 目标=c.py"),
            "续号必须从文件尾行继续（而非从 1 重置）: {}",
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
        let rows = advance_fold(&messages, &mut fold, 1, ledger_path).unwrap();
        assert_eq!(rows.len(), 1, "只有 c1 被折叠");
        assert_eq!(fold.fold_cut, Some(3), "cut 回退到 c2 轮起点");
        let view = build_request_view(&messages, &fold);
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
        };
        let ledger_path = std::path::Path::new(".gsa/ledger/current.md");
        assert!(
            advance_fold(&messages, &mut fold, 1, ledger_path).is_none(),
            "preamble 边界不合格必须放弃推进"
        );
        assert_eq!(fold.fold_start, Some(2), "状态保持不动");
        assert_eq!(fold.fold_cut, None, "状态保持不动");
        assert!(fold.folded_ledger.is_none(), "状态保持不动");
    }
}
