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
///
/// Invariant: `fold_start` and `fold_cut` are Some together (folded) or
/// None together; `folded_ledger` is non-empty iff the state is folded.
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
    /// The frozen ledger block text (re-rendered once per advance, then
    /// byte-identical across requests).
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
    while collapse_count > 0 && !is_round_complete(messages, ranges[collapse_count - 1]) {
        collapse_count -= 1;
    }
    if collapse_count == 0 {
        return None;
    }
    Some(ranges[collapse_count].0)
}

/// Number of complete old rounds that would collapse for the given tail.
pub fn collapsed_round_count(messages: &[Message], keep_recent_rounds: usize) -> usize {
    let Some(kept_start) = collapsed_cut(messages, keep_recent_rounds) else {
        return 0;
    };
    rounds_before(messages, kept_start)
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
/// - Folded: `messages[..fold_start]` (preamble) + the frozen ledger block
///   (user message) + `messages[fold_cut..]` (recent tail verbatim).
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
    let mut view = messages[..fold_start].to_vec();
    view.push(Message {
        role: Role::User,
        content: ledger.to_string(),
        tool_call_id: None,
        tool_calls: Vec::new(),
        reasoning_content: None,
    });
    view.extend_from_slice(&messages[fold_cut..]);
    view
}

/// FUS-LEDGER-FOLD-STATE (2026-08-18, ADR-0010 §14.26): advance the fold
/// point — fold every complete old round before the `keep_recent_rounds`
/// tail into a fresh frozen ledger block and move `fold_cut` forward.
///
/// The ledger is re-rendered from the FULL history before the new cut
/// (global round indices keep the numbering continuous with the previous
/// frozen block; rows for already-folded rounds are byte-identical), so
/// `folded_ledger` = 旧台账行 + 新增行. `fold_start` is set on the first
/// advance (first tool declaration) and never changes. `messages` itself
/// is never mutated.
///
/// Returns `false` (no-op, anti-spin) when there is no complete round to
/// fold — i.e. fewer than `keep_recent_rounds + 1` complete rounds, or the
/// recomputed cut equals the current `fold_cut` (nothing new outside the
/// tail since the last advance).
pub fn advance_fold(
    messages: &[Message],
    fold: &mut LedgerFoldState,
    keep_recent_rounds: usize,
) -> bool {
    let Some(kept_start) = collapsed_cut(messages, keep_recent_rounds) else {
        return false;
    };
    if fold.fold_cut.is_some_and(|cut| kept_start <= cut) {
        return false;
    }
    let ranges = round_ranges(messages);
    let collapse_count = ranges
        .iter()
        .position(|&(start, _)| start == kept_start)
        .unwrap_or(ranges.len());
    let mut rows = Vec::new();
    for (idx, &range) in ranges.iter().take(collapse_count).enumerate() {
        rows.extend(rows_for_round(messages, range, idx));
    }
    let fold_start = fold.fold_start.unwrap_or(ranges[0].0);
    fold.fold_start = Some(fold_start);
    fold.fold_cut = Some(kept_start);
    fold.folded_ledger = Some(build_ledger_block(&rows));
    true
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

    #[test]
    fn first_advance_folds_old_rounds_into_frozen_ledger() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "search_replace", "b.rs", "已编辑"));
        messages.extend(round("c3", "web_fetch", "https://x.dev", "页面"));
        messages.extend(round("c4", "read_file", "c.py", "内容C"));
        let mut fold = LedgerFoldState::default();
        assert!(advance_fold(&messages, &mut fold, 2));
        assert!(fold.is_folded());
        assert_eq!(fold.fold_start, Some(1), "first tool declaration index");
        // kept_start = the first of the newest tail=2 rounds (c3 at idx 5).
        assert_eq!(fold.fold_cut, Some(5));
        let ledger = fold.folded_ledger.as_deref().unwrap();
        assert!(ledger.starts_with(ACTION_LEDGER_PREFIX));
        assert!(ledger.contains("read_file 目标=a.py"), "{ledger}");
        assert!(ledger.contains("search_replace 目标=b.rs"), "{ledger}");
        assert!(
            !ledger.contains("c3"),
            "tail rounds stay verbatim: {ledger}"
        );

        let view = build_request_view(&messages, &fold);
        assert!(view[0].content == "任务", "{view:?}");
        assert!(
            view[1].content.starts_with(ACTION_LEDGER_PREFIX),
            "{view:?}"
        );
        // The retained region starts at fold_cut — c3/c4 stay verbatim.
        assert!(view.iter().any(|m| m.content == "页面"), "{view:?}");
        assert!(view.iter().any(|m| m.content == "内容C"), "{view:?}");
        assert!(view.iter().all(|m| m.content != "内容A"), "{view:?}");
    }

    #[test]
    fn advance_is_anti_spin_without_new_rounds() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "内容A"));
        messages.extend(round("c2", "read_file", "b.py", "内容B"));
        messages.extend(round("c3", "read_file", "c.py", "内容C"));
        let mut fold = LedgerFoldState::default();
        assert!(advance_fold(&messages, &mut fold, 2));
        let before = fold.clone();
        // No new complete round outside the tail — no-op.
        assert!(!advance_fold(&messages, &mut fold, 2));
        assert_eq!(fold, before);
        // Fewer than tail+1 complete rounds — no-op.
        let mut sparse = vec![msg(Role::User, "任务")];
        sparse.extend(round("c1", "read_file", "a.py", "内容A"));
        let mut sparse_fold = LedgerFoldState::default();
        assert!(!advance_fold(&sparse, &mut sparse_fold, 2));
        assert!(!sparse_fold.is_folded());
    }

    #[test]
    fn advance_extends_ledger_with_continuous_round_numbers() {
        let mut messages = vec![msg(Role::User, "任务")];
        messages.extend(round("c1", "read_file", "a.py", "A"));
        messages.extend(round("c2", "read_file", "b.py", "B"));
        messages.extend(round("c3", "read_file", "c.py", "C"));
        messages.extend(round("c4", "read_file", "d.py", "D"));
        let mut fold = LedgerFoldState::default();
        assert!(advance_fold(&messages, &mut fold, 2));
        let first = fold.folded_ledger.clone().unwrap();
        assert!(
            first.contains("轮次 1:") && first.contains("轮次 2:"),
            "{first}"
        );

        // Two more complete rounds: the next advance folds c3/c4 as well
        // (tail=2 now keeps c5/c6) — rows 1..4, numbering continuous.
        messages.extend(round("c5", "read_file", "e.py", "E"));
        messages.extend(round("c6", "read_file", "f.py", "F"));
        assert!(advance_fold(&messages, &mut fold, 2));
        let second = fold.folded_ledger.clone().unwrap();
        assert!(
            second.contains("轮次 1:") && second.contains("轮次 4:"),
            "{second}"
        );
        // The old rows are re-rendered byte-identically (the closing tag
        // naturally moves to the end of the extended block).
        let first_rows: Vec<&str> = first.lines().filter(|l| *l != "[/动作台账]").collect();
        let second_lines: Vec<&str> = second.lines().collect();
        assert!(
            second_lines.starts_with(&first_rows),
            "old rows stay byte-identical: {second}"
        );
        assert!(
            second.contains("read_file 目标=c.py") && second.contains("read_file 目标=d.py"),
            "newly folded rows appended: {second}"
        );
        assert!(
            !second.contains("e.py") && !second.contains("f.py"),
            "the newest tail=2 rounds stay verbatim: {second}"
        );
    }

    #[test]
    fn folded_view_prefix_is_byte_stable_across_appends() {
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
        assert!(advance_fold(&messages, &mut fold, 2));
        let view1 = build_request_view(&messages, &fold);
        // The folded prefix (preamble + frozen ledger) is byte-stable.
        let prefix_len = fold.fold_start.unwrap() + 1;
        let prefix1: Vec<&Message> = view1.iter().take(prefix_len).collect();
        for k in 5..=10 {
            messages.extend(round(
                &format!("c{k}"),
                "read_file",
                &format!("{k}.py"),
                &format!("内容{k}"),
            ));
            let view = build_request_view(&messages, &fold);
            let prefix: Vec<&Message> = view.iter().take(prefix_len).collect();
            assert_eq!(prefix, prefix1, "folded prefix rewritten at round {k}");
            // And the whole request view is pure append of the previous.
            assert_eq!(&view[..view1.len()], view1.as_slice(), "round {k}");
        }
    }
}
