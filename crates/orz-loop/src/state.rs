//! 0ds M1-S1（2026-10-27，366 章程 / 架构设计 §4.1）：`state` 分区——
//! 会话级「当前事实」薄层，与 plan/notes/findings 同级存放在黑板里。
//!
//! 纪律：
//! - 6 字段固定序 `goal / now / next / open / constraints / updated`；
//! - `updated` 只由系统在写入时盖章（轮次/域/墙钟），模型不写；
//! - 总量 ≤ [`STATE_MAX_TOKENS`]，由 provider tokenizer 硬计（不是 chars/2）；
//! - 写入是**合并**语义（缺省字段保持原值），空串 / `none` / `无` 表示清空；
//! - 只写内存黑板，无文件/网络副作用（ReadOnly 类，权限桥同族登记）。
//!
//! 兼容入口：`blackboard_read/write section=state` 与本模块的
//! `state_read/state_write` 共用同一解析/校验/渲染单源。

use serde::{Deserialize, Serialize};

use crate::tokenizer::{DeepSeekTokenizer, TokenCounter};

/// state 分区 token 上限（架构 §4.1「≤ 200 token」；provider tokenizer 硬计）。
pub const STATE_MAX_TOKENS: u64 = 200;

/// 单条 open/constraints 的字符上限（防止单条无限膨胀；总量仍由
/// [`STATE_MAX_TOKENS`] 兜底）。
pub const STATE_ITEM_MAX_CHARS: usize = 200;

/// 固定字段序（渲染/日志/journ 同源）。
pub const STATE_FIELDS: [&str; 5] = ["goal", "now", "next", "open", "constraints"];

/// 黑板 `state` 分区（v1）。所有字段 serde default：旧板/旧会话快照零迁移。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub now: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub open: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
    /// 系统写：`r<轮>@<域> <ts>`；模型不可写。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
}

/// 一次写入的字段集：`Some` = 本次提供；字符串空值/`none`/`无` = 清空该字段。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateUpdate {
    pub goal: Option<String>,
    pub now: Option<String>,
    pub next: Option<String>,
    pub open: Option<Vec<String>>,
    pub constraints: Option<Vec<String>>,
}

impl StateUpdate {
    /// `state_write` 工具参数 → 更新集（缺省字段不动；空串/`none`/`无` 清空）。
    /// 只做形态解析，预算/条目校验在 [`StateSection::apply`] 统一执行。
    pub fn from_tool_arguments(args: &serde_json::Value) -> Result<Self, StateError> {
        let object = args
            .as_object()
            .ok_or_else(|| StateError::InvalidDocument {
                line: 0,
                detail: "state_write 参数必须是对象".to_string(),
            })?;
        for key in object.keys() {
            if !STATE_FIELDS.contains(&key.as_str()) {
                return Err(StateError::InvalidDocument {
                    line: 0,
                    detail: format!(
                        "未知字段 `{key}`（限 goal/now/next/open/constraints；updated 系统写）"
                    ),
                });
            }
        }
        let text = |key: &str| -> Result<Option<String>, StateError> {
            match args.get(key) {
                None => Ok(None),
                Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
                Some(serde_json::Value::Null) => Ok(Some(String::new())),
                Some(other) => Err(StateError::InvalidDocument {
                    line: 0,
                    detail: format!("`{key}` 必须是字符串（收到 {other}）"),
                }),
            }
        };
        let items = |key: &str| -> Result<Option<Vec<String>>, StateError> {
            match args.get(key) {
                None => Ok(None),
                Some(serde_json::Value::Null) => Ok(Some(Vec::new())),
                Some(serde_json::Value::String(s)) => Ok(Some(split_items(s))),
                Some(serde_json::Value::Array(values)) => {
                    let mut out = Vec::with_capacity(values.len());
                    for (index, value) in values.iter().enumerate() {
                        let Some(s) = value.as_str() else {
                            return Err(StateError::InvalidDocument {
                                line: 0,
                                detail: format!("`{key}` 第 {} 项必须是字符串", index + 1),
                            });
                        };
                        out.push(s.to_string());
                    }
                    Ok(Some(out))
                }
                Some(other) => Err(StateError::InvalidDocument {
                    line: 0,
                    detail: format!("`{key}` 必须是字符串数组（收到 {other}）"),
                }),
            }
        };
        let update = StateUpdate {
            goal: text("goal")?,
            now: text("now")?,
            next: text("next")?,
            open: items("open")?,
            constraints: items("constraints")?,
        };
        if update.is_empty() {
            return Err(StateError::EmptyUpdate);
        }
        Ok(update)
    }

    /// 本次实际提供的字段（固定序；journal 用）。
    pub fn touched_fields(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.goal.is_some() {
            out.push("goal");
        }
        if self.now.is_some() {
            out.push("now");
        }
        if self.next.is_some() {
            out.push("next");
        }
        if self.open.is_some() {
            out.push("open");
        }
        if self.constraints.is_some() {
            out.push("constraints");
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.touched_fields().is_empty()
    }
}

impl StateSection {
    pub fn is_empty(&self) -> bool {
        self.goal.is_none()
            && self.now.is_none()
            && self.next.is_none()
            && self.open.is_empty()
            && self.constraints.is_empty()
            && self.updated.is_none()
    }

    /// 清空（含 updated；`blackboard_write op=clear section=state` 落点）。
    pub fn clear(&mut self) {
        *self = StateSection::default();
    }

    /// 合并一次写入；`updated` 由调用方盖章。校验失败不改变原状态
    /// （先克隆再提交，原子语义）。
    ///
    /// 返回值 = 内容是否真的变化。相同值的重复写、清空本就不存在的字段
    /// 都不刷新 `updated`、不产生 `StateWrite`（与 371 no-op clear 同纪律）。
    pub fn apply(&mut self, update: &StateUpdate, updated: String) -> Result<bool, StateError> {
        if update.is_empty() {
            return Err(StateError::EmptyUpdate);
        }
        let mut next = self.clone();
        if let Some(raw) = &update.goal {
            next.goal = normalized_text("goal", raw)?;
        }
        if let Some(raw) = &update.now {
            next.now = normalized_text("now", raw)?;
        }
        if let Some(raw) = &update.next {
            next.next = normalized_text("next", raw)?;
        }
        if let Some(items) = &update.open {
            next.open = normalized_items("open", items)?;
        }
        if let Some(items) = &update.constraints {
            next.constraints = normalized_items("constraints", items)?;
        }
        // 内容无变化：不盖章、不 bump、不落 StateWrite。
        if next.same_content(self) {
            return Ok(false);
        }
        next.updated = Some(updated);
        // 全部内容字段都为空 ⇒ 归一为空 state（不保留只有 updated 的空壳；
        // 与 `blackboard_write op=clear` 的终态一致）。
        if next.content_is_empty() {
            next.updated = None;
        }
        let estimated = next.estimated_tokens()?;
        if estimated > STATE_MAX_TOKENS {
            return Err(StateError::BudgetExceeded {
                estimated,
                max: STATE_MAX_TOKENS,
            });
        }
        *self = next;
        Ok(true)
    }

    /// 内容字段等价（忽略系统写的 `updated`）。
    pub fn same_content(&self, other: &Self) -> bool {
        self.goal == other.goal
            && self.now == other.now
            && self.next == other.next
            && self.open == other.open
            && self.constraints == other.constraints
    }

    fn content_is_empty(&self) -> bool {
        self.goal.is_none()
            && self.now.is_none()
            && self.next.is_none()
            && self.open.is_empty()
            && self.constraints.is_empty()
    }

    /// `blackboard_write section=state` 的文档形态解析：`key: value` 行，
    /// 允许 `state v1` 头、空行与 `#` 注释；未知 key / 缺冒号报错。
    /// 只解析，不提交（调用方再走 [`Self::apply`] 的预算/原子校验）。
    pub fn parse_document(doc: &str) -> Result<StateUpdate, StateError> {
        let mut update = StateUpdate::default();
        let mut seen = false;
        for (index, raw) in doc.lines().enumerate() {
            let line = raw.trim();
            // 371 批（2026-10-28，370 P3 收口）：版本头只认**逐字**
            // `state v1`；其余 `state …` 前缀行不再静默跳过（fail-open 例外
            // 收口）——走下方 key/value 或未知字段路径如实报错。
            if line.is_empty() || line.starts_with('#') || line == "state v1" {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else {
                return Err(StateError::InvalidDocument {
                    line: index + 1,
                    detail: "缺少 `key: value` 冒号".to_string(),
                });
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "goal" => update.goal = Some(value.to_string()),
                "now" => update.now = Some(value.to_string()),
                "next" => update.next = Some(value.to_string()),
                "open" => update.open = Some(split_items(value)),
                "constraints" => update.constraints = Some(split_items(value)),
                // `updated` 系统写：文档里出现时忽略（不报错，便于整段回填）。
                // 它不算“提供了内容字段”——只有 updated 的文档仍是空更新。
                "updated" => continue,
                other => {
                    return Err(StateError::InvalidDocument {
                        line: index + 1,
                        detail: format!(
                            "未知字段 `{other}`（限 goal/now/next/open/constraints/updated）"
                        ),
                    });
                }
            }
            seen = true;
        }
        if !seen {
            return Err(StateError::EmptyUpdate);
        }
        Ok(update)
    }

    /// 模型面渲染（固定序、空值 `none`；空分区 = `（无）`）。
    pub fn render(&self) -> String {
        if self.is_empty() {
            return "（无）".to_string();
        }
        let mut out = String::from("state v1");
        out.push_str(&format!(
            "\ngoal: {}",
            self.goal.as_deref().unwrap_or("none")
        ));
        out.push_str(&format!("\nnow: {}", self.now.as_deref().unwrap_or("none")));
        out.push_str(&format!(
            "\nnext: {}",
            self.next.as_deref().unwrap_or("none")
        ));
        out.push_str(&format!("\nopen: {}", render_items(&self.open)));
        out.push_str(&format!(
            "\nconstraints: {}",
            render_items(&self.constraints)
        ));
        out.push_str(&format!(
            "\nupdated: {}",
            self.updated.as_deref().unwrap_or("none")
        ));
        out
    }

    /// Provider tokenizer hard count (no chars/2 fallback on this path).
    pub fn estimated_tokens(&self) -> Result<u64, StateError> {
        DeepSeekTokenizer::new()
            .count_tokens(&self.render())
            .map_err(|err| StateError::TokenCount(err.to_string()))
    }
}

fn render_items(items: &[String]) -> String {
    if items.is_empty() {
        "none".to_string()
    } else {
        items.join("; ")
    }
}

fn normalized_text(field: &'static str, raw: &str) -> Result<Option<String>, StateError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_none_word(trimmed) {
        return Ok(None);
    }
    reject_multiline(field, trimmed)?;
    Ok(Some(trimmed.to_string()))
}

/// State is a line-oriented model-face document: a newline inside a value
/// would forge additional `next:` / `recent:` sections. Reject it at the
/// single normalization point instead of escaping downstream.
fn reject_multiline(field: &'static str, value: &str) -> Result<(), StateError> {
    if value.chars().any(|c| c == '\n' || c == '\r') {
        return Err(StateError::InvalidDocument {
            line: 0,
            detail: format!("`{field}` 不得包含换行/回车（state 是行式文档）"),
        });
    }
    Ok(())
}

fn normalized_items(field: &'static str, items: &[String]) -> Result<Vec<String>, StateError> {
    // 371 批（2026-10-28，370 P3 收口）：none 词项与文本字段同纪律——
    // `none`/`无` 项先剔除再计数（全 none ⇒ 清空整字段，不限原始条数；
    // 混合时保留实质项）。
    let effective: Vec<(usize, &str)> = items
        .iter()
        .enumerate()
        .map(|(index, item)| (index, item.trim()))
        .filter(|(_, trimmed)| !is_none_word(trimmed))
        .collect();
    if effective.len() > 3 {
        return Err(StateError::TooManyItems {
            field,
            max: 3,
            got: effective.len(),
        });
    }
    let mut out = Vec::with_capacity(effective.len());
    for (index, trimmed) in effective {
        if trimmed.is_empty() {
            return Err(StateError::EmptyItem { field, index });
        }
        reject_multiline(field, trimmed)?;
        if trimmed.contains([';', '；']) {
            return Err(StateError::InvalidDocument {
                line: 0,
                detail: format!(
                    "`{field}` 第 {} 条不得包含分号（分号是 state 条目分隔符）",
                    index + 1
                ),
            });
        }
        if trimmed.chars().count() > STATE_ITEM_MAX_CHARS {
            return Err(StateError::ItemTooLong {
                field,
                index,
                max: STATE_ITEM_MAX_CHARS,
            });
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

fn split_items(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_none_word(trimmed) {
        return Vec::new();
    }
    trimmed
        .split([';', '；', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_none_word(raw: &str) -> bool {
    raw.eq_ignore_ascii_case("none") || raw == "无"
}

/// state 写入/解析错误（模型面窄文案；fail-closed，不写半条）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateError {
    EmptyUpdate,
    TooManyItems {
        field: &'static str,
        max: usize,
        got: usize,
    },
    EmptyItem {
        field: &'static str,
        index: usize,
    },
    ItemTooLong {
        field: &'static str,
        index: usize,
        max: usize,
    },
    InvalidDocument {
        line: usize,
        detail: String,
    },
    BudgetExceeded {
        estimated: u64,
        max: u64,
    },
    TokenCount(String),
}

impl std::fmt::Display for StateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StateError::EmptyUpdate => write!(
                f,
                "state 写入为空：至少提供 goal/now/next/open/constraints 之一"
            ),
            StateError::TooManyItems { field, max, got } => {
                write!(f, "state {field} 条目过多（{got} > {max}）")
            }
            StateError::EmptyItem { field, index } => {
                write!(f, "state {field} 第 {} 条为空", index + 1)
            }
            StateError::ItemTooLong { field, index, max } => {
                write!(f, "state {field} 第 {} 条超出 {max} 字符", index + 1)
            }
            StateError::InvalidDocument { line, detail } => {
                write!(f, "state 文档第 {line} 行非法：{detail}")
            }
            StateError::BudgetExceeded { estimated, max } => write!(
                f,
                "state 超出预算（provider tokenizer 计数 {estimated}/{max} token）：请裁剪 open/constraints 或缩短字段"
            ),
            StateError::TokenCount(detail) => {
                write!(f, "state 预算计数失败（provider tokenizer）：{detail}")
            }
        }
    }
}

impl std::error::Error for StateError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn update_goal(goal: &str) -> StateUpdate {
        StateUpdate {
            goal: Some(goal.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn empty_state_renders_placeholder() {
        let state = StateSection::default();
        assert!(state.is_empty());
        assert_eq!(state.render(), "（无）");
    }

    #[test]
    fn write_merge_and_updated_stamp() {
        let mut state = StateSection::default();
        state
            .apply(
                &update_goal("落地 0ds M1-S1"),
                "r3@normal 2026-10-27T20:00:00Z".into(),
            )
            .unwrap();
        assert_eq!(state.goal.as_deref(), Some("落地 0ds M1-S1"));
        assert_eq!(
            state.updated.as_deref(),
            Some("r3@normal 2026-10-27T20:00:00Z")
        );
        state
            .apply(
                &StateUpdate {
                    next: Some("写 state_read/state_write".into()),
                    ..Default::default()
                },
                "r4@normal 2026-10-27T20:05:00Z".into(),
            )
            .unwrap();
        assert_eq!(
            state.goal.as_deref(),
            Some("落地 0ds M1-S1"),
            "缺省字段保持"
        );
        assert_eq!(state.next.as_deref(), Some("写 state_read/state_write"));
        assert_eq!(
            state.updated.as_deref(),
            Some("r4@normal 2026-10-27T20:05:00Z")
        );
    }

    #[test]
    fn none_word_clears_field() {
        let mut state = StateSection::default();
        state
            .apply(&update_goal("x"), "r1@normal t".into())
            .unwrap();
        state
            .apply(
                &StateUpdate {
                    goal: Some("none".into()),
                    ..Default::default()
                },
                "r2@normal t".into(),
            )
            .unwrap();
        assert!(state.goal.is_none());
        assert!(state.is_empty(), "清空最后一个内容字段 ⇒ 归一为空 state");
        state.clear();
        assert!(state.is_empty());
    }

    /// 371 批（2026-10-28，370 P3 收口）：none 词**数组项**与文本字段同
    /// 纪律——`["none"]` 清空整字段；混合项剔除 none 词保留实质项。
    #[test]
    fn none_word_array_items_are_filtered_and_clear() {
        let mut state = StateSection::default();
        state
            .apply(
                &StateUpdate {
                    open: Some(vec!["a".into(), "NONE".into(), "无".into(), "b".into()]),
                    ..Default::default()
                },
                "r1@normal t".into(),
            )
            .unwrap();
        assert_eq!(state.open, vec!["a", "b"], "混合项剔除 none 词");
        state
            .apply(
                &StateUpdate {
                    open: Some(vec!["none".into()]),
                    ..Default::default()
                },
                "r2@normal t".into(),
            )
            .unwrap();
        assert!(
            state.open.is_empty(),
            "全 none 词项 ⇒ 字段清空: {:?}",
            state.open
        );
        // 文档形态同语义（parse → split_items → apply 统一归一口径）。
        let update = StateSection::parse_document("open: 问题A；none；问题B").unwrap();
        let mut doc_state = StateSection::default();
        doc_state.apply(&update, "r3@normal t".into()).unwrap();
        assert_eq!(doc_state.open, vec!["问题A", "问题B"]);
    }

    /// 371 批（2026-10-28，370 P3 收口）：版本头只认逐字 `state v1`；
    /// 其余 `state …` 前缀行不再静默跳过——fail-open 例外收口为如实报错。
    #[test]
    fn document_state_prefixed_lines_are_rejected_not_skipped() {
        assert!(matches!(
            StateSection::parse_document("state v2\ngoal: x"),
            Err(StateError::InvalidDocument { .. })
        ));
        assert!(matches!(
            StateSection::parse_document("state goal: 误写"),
            Err(StateError::InvalidDocument { .. })
        ));
        // 逐字 v1 头仍被接受（回环不变）。
        let update = StateSection::parse_document("state v1\ngoal: 目标").unwrap();
        assert_eq!(update.touched_fields(), vec!["goal"]);
    }

    #[test]
    fn budget_rejects_without_mutation() {
        let mut state = StateSection::default();
        state
            .apply(&update_goal("ok"), "r1@normal t".into())
            .unwrap();
        let before = state.clone();
        let long = "长".repeat(STATE_MAX_TOKENS as usize * 2 + 80);
        let err = state
            .apply(
                &StateUpdate {
                    now: Some(long),
                    ..Default::default()
                },
                "r2@normal t".into(),
            )
            .unwrap_err();
        assert!(matches!(err, StateError::BudgetExceeded { .. }));
        assert_eq!(state, before, "预算拒绝必须原子（不改原状态）");
        assert!(state.estimated_tokens().unwrap() <= STATE_MAX_TOKENS);
    }

    /// 0ds M1 v1 (2026-10-28): hard budget is provider-tokenizer counted.
    /// 401 repeats of a common CJK char are >200 provider tokens (chars/2
    /// would have said 200 and accepted the old bug).
    #[test]
    fn token_budget_uses_provider_tokenizer_and_rejects_cjk_overrun() {
        assert_eq!(STATE_MAX_TOKENS, 200);
        let mut state = StateSection::default();
        let err = state
            .apply(&update_goal(&"一".repeat(401)), "r1@normal t".into())
            .unwrap_err();
        assert!(matches!(err, StateError::BudgetExceeded { .. }), "{err:?}");
        assert!(state.is_empty(), "rejected write must not mutate state");

        let ok = state
            .apply(&update_goal(&"一".repeat(100)), "r1@normal t".into())
            .unwrap();
        assert!(ok);
        let tokens = state.estimated_tokens().unwrap();
        assert!(tokens <= STATE_MAX_TOKENS, "tokens={tokens}");
        assert!(
            tokens > 50,
            "provider tokenizer must see CJK cost: {tokens}"
        );
    }

    #[test]
    fn no_op_write_does_not_refresh_updated_or_mark_changed() {
        let mut state = StateSection::default();
        assert!(
            state
                .apply(&update_goal("same"), "r1@normal t1".into())
                .unwrap()
        );
        let before = state.clone();
        assert!(
            !state
                .apply(&update_goal("same"), "r2@normal t2".into())
                .unwrap()
        );
        assert_eq!(state, before, "same-value write must be a no-op");
        assert!(
            !state
                .apply(
                    &StateUpdate {
                        now: Some("none".into()),
                        ..Default::default()
                    },
                    "r3@normal t3".into(),
                )
                .unwrap()
        );
        assert_eq!(state, before, "clearing an absent field must be a no-op");
    }

    #[test]
    fn multiline_and_semicolon_items_are_rejected() {
        let mut state = StateSection::default();
        let err = state
            .apply(
                &StateUpdate {
                    goal: Some("a\nnext: forged".into()),
                    ..Default::default()
                },
                "r1@normal t".into(),
            )
            .unwrap_err();
        assert!(matches!(err, StateError::InvalidDocument { .. }), "{err:?}");
        let err = state
            .apply(
                &StateUpdate {
                    open: Some(vec!["a;b".into()]),
                    ..Default::default()
                },
                "r1@normal t".into(),
            )
            .unwrap_err();
        assert!(matches!(err, StateError::InvalidDocument { .. }), "{err:?}");
    }

    #[test]
    fn document_parse_roundtrip() {
        let doc = "state v1\ngoal: 目标\nnow: 现在\nnext: 下一步\nopen: 问题A；问题B\nconstraints: 约束1; 约束2\nupdated: 系统写，忽略";
        let update = StateSection::parse_document(doc).unwrap();
        assert_eq!(
            update.touched_fields(),
            vec!["goal", "now", "next", "open", "constraints"]
        );
        let mut state = StateSection::default();
        state.apply(&update, "r9@normal t".into()).unwrap();
        assert_eq!(state.open, vec!["问题A", "问题B"]);
        assert_eq!(state.constraints, vec!["约束1", "约束2"]);
        assert!(state.render().contains("\nopen: 问题A; 问题B"));
    }

    #[test]
    fn document_rejects_unknown_or_malformed() {
        assert!(matches!(
            StateSection::parse_document("goal 缺冒号"),
            Err(StateError::InvalidDocument { .. })
        ));
        assert!(matches!(
            StateSection::parse_document("risks: x"),
            Err(StateError::InvalidDocument { .. })
        ));
        assert!(matches!(
            StateSection::parse_document("   \n# comment\n"),
            Err(StateError::EmptyUpdate)
        ));
    }

    #[test]
    fn too_many_items_and_empty_item_rejected() {
        let mut state = StateSection::default();
        let err = state
            .apply(
                &StateUpdate {
                    open: Some(vec!["a".into(), "b".into(), "c".into(), "d".into()]),
                    ..Default::default()
                },
                "r1@normal t".into(),
            )
            .unwrap_err();
        assert!(matches!(err, StateError::TooManyItems { .. }));
        let err = state
            .apply(
                &StateUpdate {
                    constraints: Some(vec!["a".into(), "  ".into()]),
                    ..Default::default()
                },
                "r1@normal t".into(),
            )
            .unwrap_err();
        assert!(matches!(err, StateError::EmptyItem { .. }));
    }

    #[test]
    fn tool_arguments_reject_unknown_fields_and_empty_update() {
        let err = StateUpdate::from_tool_arguments(&serde_json::json!({
            "goal": "x",
            "updated": "模型不得写",
        }))
        .unwrap_err();
        assert!(matches!(err, StateError::InvalidDocument { .. }), "{err:?}");
        let err = StateUpdate::from_tool_arguments(&serde_json::json!({})).unwrap_err();
        assert!(matches!(err, StateError::EmptyUpdate));
        let update = StateUpdate::from_tool_arguments(&serde_json::json!({
            "open": "a；b",
            "constraints": ["c", "d"],
        }))
        .unwrap();
        assert_eq!(update.open, Some(vec!["a".into(), "b".into()]));
        assert_eq!(update.constraints, Some(vec!["c".into(), "d".into()]));
    }
}
