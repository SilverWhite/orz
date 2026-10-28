//! 0ds M1-S3（2026-10-28）：`context_manage(mode=recenter)` 的纯渲染。
//!
//! 设计：`state + 最近 N 轮摘要 + next`；只读、无副作用、≤1K token、
//! 不写会话、不注入常驻文本。active decisions 属 M1.5，本模块不读
//! `.gsa/decisions`（设计明确 M1.5 再加）。

use crate::action_ledger;
use crate::gateway::model::Message;
use crate::state::StateSection;

/// 与全仓上下文计量同尺（chars/2）。
pub const RECENTER_MAX_TOKENS: usize = 1_024;

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

/// `context_manage(mode=recenter)` 的只读渲染。`state` 原样按固定字段序；
/// `next` 在末尾显式重复一次（验收：调用后下一步与 state.next 一致）。
pub fn render_recenter(state: &StateSection, messages: &[Message], run_id: &str) -> String {
    let max_chars = RECENTER_MAX_TOKENS * 2;
    let next_line = format!("next: {}", state.next.as_deref().unwrap_or("none"));
    let mut lines = vec![format!("[recenter v1] run={run_id}"), "state:".to_string()];
    let state_text = state.render();
    for line in state_text.lines() {
        lines.push(line.to_string());
    }
    lines.push("recent:".to_string());
    let recent = recent_round_lines(messages);
    if recent.is_empty() {
        lines.push("none".to_string());
    } else {
        let mut used: usize = lines.iter().map(|l| l.chars().count() + 1).sum();
        let mut kept = 0usize;
        for line in &recent {
            let line_chars = line.chars().count() + 1;
            // 末尾 next 必须保留；预算不够时宁可少列 recent。
            if used + line_chars + next_line.chars().count() + 2 > max_chars {
                break;
            }
            used += line_chars;
            lines.push(line.clone());
            kept += 1;
        }
        if kept < recent.len() {
            lines.push(format!(
                "... +{} rounds（recenter 1K token 上限）",
                recent.len() - kept
            ));
        }
    }
    lines.push(next_line);
    let mut out = lines.join("\n");
    // state 自身异常膨胀时也不让输出越过硬帽；state 正常 ≤200 token。
    if out.chars().count() / 2 > RECENTER_MAX_TOKENS {
        let cut: String = out.chars().take(max_chars.saturating_sub(1)).collect();
        out = format!("{cut}…");
    }
    out
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

    #[test]
    fn recenter_empty_state_and_no_rounds_is_explicit() {
        let out = render_recenter(&StateSection::default(), &[], "RUN-R");
        assert!(out.starts_with("[recenter v1]"), "{out}");
        assert!(out.contains("state:\n（无）"), "{out}");
        assert!(out.contains("recent:\nnone"), "{out}");
        assert!(out.contains("next: none"), "{out}");
        assert!(out.chars().count() / 2 <= RECENTER_MAX_TOKENS);
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
        let out = render_recenter(&state_full(), &messages, "RUN-R");
        assert!(out.contains("r2 "), "{out}");
        assert!(out.contains("r6 "), "{out}");
        assert!(!out.contains("r1 "), "only last five rounds: {out}");
        assert!(out.contains("next: 跑 loop 测试"), "{out}");
        assert!(out.chars().count() / 2 <= RECENTER_MAX_TOKENS);
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
        let out = render_recenter(&state_full(), &messages, "RUN-R");
        assert!(out.contains("r26 "), "{out}");
        assert!(out.contains("r30 "), "{out}");
        assert!(!out.contains("r25 "), "only last five rounds: {out}");
        assert!(out.contains("next: 跑 loop 测试"), "{out}");
        assert!(out.chars().count() / 2 <= RECENTER_MAX_TOKENS, "{out}");
    }
}
