//! Mechanical repair of provider tool-pairing invariants in restored
//! conversations.
//!
//! A session sidecar can lose the tail of a tool round (crash/cancel between
//! the assistant declaration and its `tool` result), and the journal-fallback
//! archive reconstruction never stored tool outputs at all. Providers reject
//! either shape with:
//!
//! > An assistant message with 'tool_calls' must be followed by tool messages
//! > responding to each 'tool_call_id'.
//!
//! This module restores the *shape* without inventing success:
//!
//! - a declaration whose calls are **entirely** unanswered is stripped of its
//!   `tool_calls` (and dropped when it has no text/reasoning body). Journal
//!   reconstruction falls in this branch: the journal has the call record but
//!   not the output, so replaying the call would be a dangling declaration;
//! - a declaration with **some** answered calls gets one truthful synthetic
//!   result per missing call. The text says the result was not recorded and
//!   explicitly tells the model not to treat it as success;
//! - orphan `tool` messages (no owning declaration immediately before) and
//!   duplicate results are removed.
//!
//! The repair is idempotent and preserves every message it does not have to
//! change.

use std::collections::HashSet;

use orz_loop::gateway::{Message, Role, ToolCall};

/// What [`repair_tool_pairing`] changed. All-zero means the conversation was
/// already provider-valid and was left untouched.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct RepairReport {
    /// Assistant declarations whose `tool_calls` were stripped because no
    /// result was recorded for any of their calls.
    pub stripped_declarations: usize,
    /// Individual call ids removed by [`Self::stripped_declarations`].
    pub stripped_calls: usize,
    /// Assistant messages dropped because stripping left them with no body.
    pub dropped_empty_messages: usize,
    /// Synthetic `tool` messages inserted for calls missing from a partially
    /// answered declaration.
    pub synthetic_results: usize,
    /// Orphan or duplicate `tool` messages removed.
    pub dropped_orphan_results: usize,
}

impl RepairReport {
    pub fn changed(&self) -> bool {
        self.stripped_declarations > 0
            || self.stripped_calls > 0
            || self.dropped_empty_messages > 0
            || self.synthetic_results > 0
            || self.dropped_orphan_results > 0
    }
}

/// Repair the assistant/tool pairing invariant in place.
///
/// Scans front-to-back and only treats the contiguous run of `tool` messages
/// immediately after an assistant declaration as its answers (the provider
/// requires that adjacency). Any later `tool` message is an orphan and is
/// removed.
pub fn repair_tool_pairing(messages: &mut Vec<Message>) -> RepairReport {
    let mut report = RepairReport::default();
    let mut repaired: Vec<Message> = Vec::with_capacity(messages.len());
    let mut index = 0usize;

    while index < messages.len() {
        let message = messages[index].clone();
        match message.role {
            Role::Assistant if !message.tool_calls.is_empty() => {
                let start = index + 1;
                let mut end = start;
                while end < messages.len() && messages[end].role == Role::Tool {
                    end += 1;
                }
                let mut results: Vec<Message> = messages[start..end].to_vec();
                report.dropped_orphan_results += drop_unidentified_results(&mut results);

                if results.is_empty() {
                    // No result at all: keep the declaration only when it has
                    // a body the model can still use; otherwise drop it.
                    let mut stripped = message;
                    report.stripped_declarations += 1;
                    report.stripped_calls += stripped.tool_calls.len();
                    stripped.tool_calls.clear();
                    stripped.tool_call_id = None;
                    if assistant_has_body(&stripped) {
                        repaired.push(stripped);
                    } else {
                        report.dropped_empty_messages += 1;
                    }
                    index = end;
                    continue;
                }

                repaired.push(message.clone());
                let mut used = vec![false; results.len()];
                for call in &message.tool_calls {
                    let matched = results.iter().enumerate().find_map(|(pos, result)| {
                        (!used[pos]
                            && result.tool_call_id.as_deref() == Some(call.call_id.as_str()))
                        .then_some(pos)
                    });
                    match matched {
                        Some(pos) => {
                            used[pos] = true;
                            repaired.push(results[pos].clone());
                        }
                        None => {
                            repaired.push(synthetic_result(call));
                            report.synthetic_results += 1;
                        }
                    }
                }
                // Duplicate answers (same id twice) and answers for ids the
                // declaration never issued are both provider-invalid.
                report.dropped_orphan_results += used.iter().filter(|used| !**used).count();
                index = end;
            }
            Role::Tool => {
                // No assistant declaration immediately before this result.
                report.dropped_orphan_results += 1;
                index += 1;
            }
            _ => {
                repaired.push(message);
                index += 1;
            }
        }
    }

    if report.changed() {
        *messages = repaired;
    }
    report
}

fn drop_unidentified_results(results: &mut Vec<Message>) -> usize {
    let before = results.len();
    // Keep the LAST result for a duplicated call_id (the real result can
    // arrive after a cancellation placeholder), drop empty ids, and preserve
    // the original order of the survivors.
    let mut seen: HashSet<String> = HashSet::new();
    let mut keep = vec![false; results.len()];
    for (index, result) in results.iter().enumerate().rev() {
        let Some(id) = result.tool_call_id.as_deref().filter(|id| !id.is_empty()) else {
            continue;
        };
        if seen.insert(id.to_string()) {
            keep[index] = true;
        }
    }
    let mut index = 0usize;
    results.retain(|_| {
        let keep_this = keep[index];
        index += 1;
        keep_this
    });
    before - results.len()
}

fn assistant_has_body(message: &Message) -> bool {
    !message.content.trim().is_empty()
        || message
            .reasoning_content
            .as_deref()
            .is_some_and(|reasoning| !reasoning.trim().is_empty())
}

fn synthetic_result(call: &ToolCall) -> Message {
    Message {
        role: Role::Tool,
        content: format!(
            "工具调用 `{}`（call_id={}）的结果在会话恢复时缺失：journal/侧车未保留工具输出。\
             该调用没有返回结果，请勿把它当作成功；如仍需要该事实，请重新调用。",
            call.name, call.call_id
        ),
        tool_call_id: Some(call.call_id.clone()),
        tool_calls: Vec::new(),
        reasoning_content: None,
        round: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant(content: &str, calls: &[(&str, &str)]) -> Message {
        Message {
            role: Role::Assistant,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: calls
                .iter()
                .map(|(name, id)| ToolCall {
                    name: (*name).to_string(),
                    arguments: serde_json::json!({}),
                    call_id: (*id).to_string(),
                })
                .collect(),
            reasoning_content: None,
            round: None,
        }
    }

    fn tool(id: &str, content: &str) -> Message {
        Message {
            role: Role::Tool,
            content: content.to_string(),
            tool_call_id: Some(id.to_string()),
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    fn user(content: &str) -> Message {
        Message {
            role: Role::User,
            content: content.to_string(),
            tool_call_id: None,
            tool_calls: Vec::new(),
            reasoning_content: None,
            round: None,
        }
    }

    #[test]
    fn complete_pairing_is_untouched() {
        let mut messages = vec![
            user("问题"),
            assistant("", &[("read_file", "c1")]),
            tool("c1", "结果"),
            assistant("完成", &[]),
        ];
        let before = messages.clone();
        let report = repair_tool_pairing(&mut messages);
        assert!(!report.changed(), "{report:?}");
        assert_eq!(messages, before);
    }

    #[test]
    fn fully_unanswered_declaration_is_stripped_and_empty_message_dropped() {
        let mut messages = vec![
            user("问题"),
            assistant("", &[("read_file", "c1"), ("bash", "c2")]),
            assistant("完成报告", &[]),
        ];
        let report = repair_tool_pairing(&mut messages);
        assert_eq!(report.stripped_declarations, 1);
        assert_eq!(report.stripped_calls, 2);
        assert_eq!(report.dropped_empty_messages, 1);
        assert_eq!(messages.len(), 2);
        assert!(messages.iter().all(|m| m.tool_calls.is_empty()));
        assert_eq!(messages[1].content, "完成报告");
    }

    #[test]
    fn unanswered_declaration_with_text_keeps_its_body() {
        let mut messages = vec![
            user("问题"),
            assistant("我先读文件。", &[("read_file", "c1")]),
            user("继续"),
        ];
        let report = repair_tool_pairing(&mut messages);
        assert_eq!(report.stripped_declarations, 1);
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[1].content, "我先读文件。");
        assert!(messages[1].tool_calls.is_empty());
    }

    #[test]
    fn partial_pairing_gets_one_truthful_synthetic_result_per_missing_call() {
        let mut messages = vec![
            user("问题"),
            assistant("", &[("read_file", "c1"), ("bash", "c2")]),
            tool("c1", "真实结果"),
            assistant("完成", &[]),
        ];
        let report = repair_tool_pairing(&mut messages);
        assert_eq!(report.synthetic_results, 1);
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[2].tool_call_id.as_deref(), Some("c1"));
        assert_eq!(messages[3].tool_call_id.as_deref(), Some("c2"));
        assert!(messages[3].content.contains("结果在会话恢复时缺失"));
        assert!(messages[3].content.contains("请勿把它当作成功"));
    }

    #[test]
    fn orphan_and_duplicate_results_are_removed() {
        let mut messages = vec![
            user("问题"),
            tool("lost", "孤儿结果"),
            assistant("", &[("read_file", "c1")]),
            tool("c1", "第一次"),
            tool("c1", "重复"),
            tool("other", "别人的"),
            assistant("完成", &[]),
        ];
        let report = repair_tool_pairing(&mut messages);
        assert_eq!(report.dropped_orphan_results, 3);
        assert_eq!(
            messages
                .iter()
                .filter(|m| m.role == Role::Tool)
                .map(|m| m.tool_call_id.as_deref())
                .collect::<Vec<_>>(),
            vec![Some("c1")]
        );
    }

    #[test]
    fn repair_is_idempotent() {
        let mut messages = vec![
            user("问题"),
            assistant("", &[("read_file", "c1"), ("bash", "c2")]),
            tool("c1", "真实结果"),
            assistant("完成", &[]),
        ];
        let first = repair_tool_pairing(&mut messages);
        assert!(first.changed());
        let snapshot = messages.clone();
        let second = repair_tool_pairing(&mut messages);
        assert!(!second.changed(), "{second:?}");
        assert_eq!(messages, snapshot);
    }
}
