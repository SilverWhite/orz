//! 0ds M1-S3（2026-10-28）：`context_manage(mode=recenter)` 的纯渲染。
//!
//! 设计：`state + 最近 N 轮摘要 + next`；只读、无副作用、≤1K token、
//! 不写会话、不注入常驻文本。active decisions 属 M1.5，本模块不读
//! `.gsa/decisions`（设计明确 M1.5 再加）。
//!
//! 0ds M1 v1（2026-10-28）：预算改用 provider tokenizer；`next` 是最后
//! 一行的硬承诺——先保留 next，再截 recent，绝不做“从尾部 take”把
//! next 截掉的兜底。

use crate::action_ledger;
use crate::gateway::model::Message;
use crate::state::StateSection;
use crate::tokenizer::{DeepSeekTokenizer, TokenCountError, TokenCounter};

/// 与 provider tokenizer 同尺的硬上限。
pub const RECENTER_MAX_TOKENS: u64 = 1_024;

/// 最近完整工具轮数。只取**完整轮**（tool_calls 与 tool 结果配对），
/// 半截轮不进入 recenter；与压缩的 tail 语义区分。
pub const RECENTER_RECENT_ROUNDS: usize = 5;

const RECENT_TARGET_MAX_CHARS: usize = 48;
const RECENT_REPLY_MAX_CHARS: usize = 80;

fn truncate_line(text: &str, max_chars: usize) -> String {
    let one_line = text.replace(['\r', '\n'], " ");
    let mut out: String = one_line.chars().take(max_chars).collect();
    if one_line.chars().count() > max_chars {
        out.push('…');
    }
    out
}

/// 本会话实际进入 recenter 的完整工具轮数（≤[`RECENTER_RECENT_ROUNDS`]）。
pub fn recent_round_count(messages: &[Message]) -> usize {
    let ranges = action_ledger::round_ranges(messages);
    ranges
        .iter()
        .filter(|range| action_ledger::is_round_complete(messages, **range))
        .count()
        .min(RECENTER_RECENT_ROUNDS)
}

/// 最近 N 个**完整**工具轮的压缩行（每轮一行；不含工具大结果，只留工具名/
/// 目标/最终回复短摘要）。
fn recent_round_lines(messages: &[Message]) -> Vec<String> {
    let ranges = action_ledger::round_ranges(messages);
    let complete: Vec<(usize, (usize, usize))> = ranges
        .iter()
        .enumerate()
        .filter(|(_, range)| action_ledger::is_round_complete(messages, **range))
        .map(|(idx, range)| (idx, *range))
        .collect();
    let start = complete.len().saturating_sub(RECENTER_RECENT_ROUNDS);
    let mut out = Vec::new();
    for (round_index, range) in complete[start..].iter().copied() {
        let rows = action_ledger::rows_for_round(messages, range, round_index);
        if rows.is_empty() {
            continue;
        }
        let mut tools: Vec<String> = rows
            .iter()
            .take(2)
            .map(|row| {
                let target = truncate_line(&row.target, RECENT_TARGET_MAX_CHARS);
                if target.is_empty() {
                    row.tool.clone()
                } else {
                    format!("{} {}", row.tool, target)
                }
            })
            .collect();
        if rows.len() > 2 {
            tools.push(format!("+{} tools", rows.len() - 2));
        }
        let reply = rows
            .first()
            .map(|row| truncate_line(&row.final_reply, RECENT_REPLY_MAX_CHARS))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "（无）".to_string());
        out.push(format!(
            "r{} {} -> {}",
            round_index + 1,
            tools.join("; "),
            reply
        ));
    }
    out
}

/// Rendered recenter + actual row counts. `blocks`/`rounds` in the
/// ContextPulled audit must describe **what the model actually received**,
/// not the pre-truncation candidate list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecenterRender {
    pub text: String,
    pub available_rounds: usize,
    pub rendered_rounds: usize,
    pub token_estimate: u64,
}

fn compose_recenter(
    prefix: &[String],
    selected: &[String],
    marker: Option<&str>,
    next_line: &str,
) -> String {
    let mut out = prefix.join("\n");
    if selected.is_empty() && marker.is_none() {
        out.push_str("\nnone");
    } else {
        for line in selected {
            out.push('\n');
            out.push_str(line);
        }
        if let Some(marker) = marker {
            out.push('\n');
            out.push_str(marker);
        }
    }
    out.push('\n');
    out.push_str(next_line);
    out
}

/// `context_manage(mode=recenter)` 的只读渲染。`state` 原样按固定字段序；
/// `next` 在末尾显式重复一次（验收：调用后下一步与 state.next 一致）。
///
/// 预算算法：先保留 header/state/next，再逐条加 recent；若省略标记把预算
/// 顶破，则回退 last recent 直到标记与 next 都能放下。**next 绝不截断**。
pub fn render_recenter(
    state: &StateSection,
    messages: &[Message],
    run_id: &str,
) -> Result<RecenterRender, TokenCountError> {
    let counter = DeepSeekTokenizer::new();
    let next_line = format!("next: {}", state.next.as_deref().unwrap_or("none"));
    let mut prefix = vec![format!("[recenter v1] run={run_id}"), "state:".to_string()];
    for line in state.render().lines() {
        prefix.push(line.to_string());
    }
    prefix.push("recent:".to_string());

    // State should be <=200 provider tokens. If it is not (legacy/corrupt),
    // fail closed rather than emitting a partial state or dropping next.
    let base = compose_recenter(&prefix, &[], None, &next_line);
    if counter.count_tokens(&base)? > RECENTER_MAX_TOKENS {
        return Err(TokenCountError::Encode(
            "recenter base (state + next) exceeds the 1K provider-token budget".to_string(),
        ));
    }

    let recent = recent_round_lines(messages);
    let available_rounds = recent.len();
    let mut selected: Vec<String> = Vec::new();
    for line in &recent {
        let mut candidate = selected.clone();
        candidate.push(line.clone());
        let omitted = available_rounds - candidate.len();
        let marker =
            (omitted > 0).then(|| format!("... +{omitted} rounds（recenter 1K token 上限）"));
        let text = compose_recenter(&prefix, &candidate, marker.as_deref(), &next_line);
        if counter.count_tokens(&text)? <= RECENTER_MAX_TOKENS {
            selected = candidate;
        } else {
            break;
        }
    }

    // The omission marker can itself push the output over budget. Drop the
    // newest selected row (never next) until the marked output fits.
    loop {
        let omitted = available_rounds - selected.len();
        let marker =
            (omitted > 0).then(|| format!("... +{omitted} rounds（recenter 1K token 上限）"));
        let text = compose_recenter(&prefix, &selected, marker.as_deref(), &next_line);
        let token_estimate = counter.count_tokens(&text)?;
        if token_estimate <= RECENTER_MAX_TOKENS {
            return Ok(RecenterRender {
                text,
                available_rounds,
                rendered_rounds: selected.len(),
                token_estimate,
            });
        }
        if selected.pop().is_none() {
            return Err(TokenCountError::Encode(
                "recenter cannot fit next within the 1K provider-token budget".to_string(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gateway::model::{Message, Role, ToolCall};
    use serde_json::json;

    fn state_full() -> StateSection {
        StateSection {
            goal: Some("完成 M1-S3".to_string()),
            now: Some("写 recenter".to_string()),
            next: Some("跑 loop 测试".to_string()),
            open: vec!["预算边界".to_string()],
            constraints: vec!["只读".to_string()],
            updated: Some("r1@normal 2026-10-28T00:00:00Z".to_string()),
        }
    }

    fn msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    fn round(id: &str, target: &str, reply: &str) -> Vec<Message> {
        let mut call = msg(Role::Assistant, "");
        call.tool_calls.push(ToolCall {
            name: "read_file".to_string(),
            arguments: json!({ "path": target }),
            call_id: id.to_string(),
        });
        let mut result = msg(Role::Tool, "tool result");
        result.tool_call_id = Some(id.to_string());
        vec![call, result, msg(Role::Assistant, reply)]
    }

    fn token_count(text: &str) -> u64 {
        DeepSeekTokenizer::new().count_tokens(text).unwrap()
    }

    #[test]
    fn recenter_empty_state_and_no_rounds_is_explicit() {
        let out = render_recenter(&StateSection::default(), &[], "RUN-R").unwrap();
        assert!(out.text.starts_with("[recenter v1]"), "{out:?}");
        assert!(out.text.contains("state:\n（无）"), "{out:?}");
        assert!(out.text.contains("recent:\nnone"), "{out:?}");
        assert!(out.text.contains("next: none"), "{out:?}");
        assert!(token_count(&out.text) <= RECENTER_MAX_TOKENS);
    }

    #[test]
    fn recenter_lists_last_five_complete_rounds_and_next() {
        let mut messages = Vec::new();
        for i in 1..=6 {
            messages.extend(round(
                &format!("c{i}"),
                &format!("f{i}.rs"),
                &format!("reply{i}"),
            ));
        }
        let out = render_recenter(&state_full(), &messages, "RUN-R").unwrap();
        assert!(out.text.contains("r2 "), "{out:?}");
        assert!(out.text.contains("r6 "), "{out:?}");
        assert!(!out.text.contains("r1 "), "only last five rounds: {out:?}");
        assert!(out.text.contains("next: 跑 loop 测试"), "{out:?}");
        assert!(out.rendered_rounds <= RECENTER_RECENT_ROUNDS);
        assert!(token_count(&out.text) <= RECENTER_MAX_TOKENS);
    }

    #[test]
    fn recenter_keeps_last_five_rounds_and_next_within_budget() {
        let mut messages = Vec::new();
        for i in 1..=30 {
            messages.extend(round(
                &format!("c{i}"),
                &format!("{}-{}", "x".repeat(100), i),
                &"y".repeat(200),
            ));
        }
        let out = render_recenter(&state_full(), &messages, "RUN-R").unwrap();
        assert!(out.text.contains("r26 "), "{out:?}");
        assert!(out.text.contains("r30 "), "{out:?}");
        assert!(!out.text.contains("r25 "), "only last five rounds: {out:?}");
        assert!(out.text.contains("next: 跑 loop 测试"), "{out:?}");
        assert!(token_count(&out.text) <= RECENTER_MAX_TOKENS, "{out:?}");
    }

    #[test]
    fn recenter_never_drops_next_when_budget_truncates_recent() {
        // Direct StateSection construction bypasses the write budget on purpose:
        // this is the legacy/corrupt-state boundary that used to hit take(2047).
        let mut state = state_full();
        // ~800 mostly-distinct CJK chars (provider tokens ≈1/char) plus five
        // recent lines forces recenter to drop rows while keeping `next:`.
        state.goal = Some(
            (0..560)
                .map(|i| char::from_u32(0x4e00 + (i % 1000)).unwrap())
                .collect(),
        );
        let mut messages = Vec::new();
        for i in 1..=8 {
            messages.extend(round(
                &format!("c{i}"),
                &format!("{}-{}", "x".repeat(120), i),
                &"y".repeat(220),
            ));
        }
        let out = render_recenter(&state, &messages, "RUN-R").unwrap();
        assert!(out.text.ends_with("next: 跑 loop 测试"), "{out:?}");
        assert!(out.rendered_rounds < out.available_rounds, "{out:?}");
        assert!(token_count(&out.text) <= RECENTER_MAX_TOKENS, "{out:?}");
    }
}
