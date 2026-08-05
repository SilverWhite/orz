//! View-model widget state — pure data + mutation methods.
//!
//! No ratatui types appear in mutation paths (Python `widgets.py` precedent:
//! the projector mutates fields in place, the renderer re-reads them), so
//! every mutation is unit-testable without a terminal.

use xai_ratatui_textarea::textarea::TextArea;

/// A conversation message card (bordered).
#[derive(Debug, Clone, PartialEq)]
pub struct ChatMessage {
    /// 用户 | 模型 | 系统
    pub role: String,
    pub content: String,
    pub turn: u32,
    pub collapsible: bool,
    pub warning: bool,
    pub collapsed: bool,
}

/// One entry in a tool trace line.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolEntry {
    pub target: String,
    pub detail: String,
}

/// A tool trace line (borderless, collapsible).
#[derive(Debug, Clone, PartialEq)]
pub struct ToolTraceLine {
    pub tool_name: String,
    pub entries: Vec<ToolEntry>,
    pub expanded: bool,
}

/// Content pane item — message card or tool trace.
#[derive(Debug, Clone, PartialEq)]
pub enum ContentItem {
    Message(ChatMessage),
    ToolTrace(ToolTraceLine),
}

impl ContentItem {
    /// Collapsed for messages; for tool traces "collapsed" = not expanded.
    pub fn collapsed(&self) -> bool {
        match self {
            ContentItem::Message(m) => m.collapsed,
            ContentItem::ToolTrace(t) => !t.expanded,
        }
    }
}

/// The conversation content pane (center column).
#[derive(Debug, Clone)]
pub struct ContentPane {
    pub items: Vec<ContentItem>,
    /// Index of the current model card — text_delta append target.
    pub current_model_index: Option<usize>,
    pub empty_hint: String,
}

impl Default for ContentPane {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            current_model_index: None,
            empty_hint: "输入问题开始对话...".into(),
        }
    }
}

impl ContentPane {
    /// All text entering the view model is sanitized at the boundary —
    /// model/tool output is untrusted and could carry terminal escape
    /// sequences (review P2-1).
    fn sanitize(s: &str) -> String {
        crate::theme::sanitize_text(s)
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.items.push(ContentItem::Message(ChatMessage {
            role: "用户".into(),
            content: Self::sanitize(content),
            turn: 0,
            collapsible: true,
            warning: false,
            collapsed: false,
        }));
    }

    pub fn add_model_message(&mut self, content: &str, turn: u32, warning: bool) {
        self.items.push(ContentItem::Message(ChatMessage {
            role: "模型".into(),
            content: Self::sanitize(content),
            turn,
            collapsible: true,
            warning,
            collapsed: false,
        }));
        self.current_model_index = Some(self.items.len() - 1);
    }

    /// Generic system card (gate status, errors, notices).
    pub fn add_system_message(&mut self, content: &str, warning: bool) {
        self.items.push(ContentItem::Message(ChatMessage {
            role: "系统".into(),
            content: Self::sanitize(content),
            turn: 0,
            collapsible: !warning,
            warning,
            collapsed: false,
        }));
    }

    /// Append streaming text to the current model card (Python
    /// `append_text_delta` — no-op without a current model card).
    pub fn append_text_delta(&mut self, text: &str) {
        let Some(idx) = self.current_model_index else {
            return;
        };
        if let Some(ContentItem::Message(msg)) = self.items.get_mut(idx) {
            if msg.role == "模型" {
                msg.content.push_str(&Self::sanitize(text));
            }
        }
    }

    /// Add a tool trace entry; same tool name reuses the line (scrolls to
    /// newest), different tool gets a new line (Python
    /// `add_or_update_tool_trace`).
    pub fn add_or_update_tool_trace(&mut self, tool_name: &str, target: &str, detail: &str) {
        for item in &mut self.items {
            if let ContentItem::ToolTrace(trace) = item {
                if trace.tool_name == tool_name {
                    trace.entries.push(ToolEntry {
                        target: Self::sanitize(target),
                        detail: Self::sanitize(detail),
                    });
                    return;
                }
            }
        }
        let mut trace = ToolTraceLine {
            tool_name: tool_name.to_string(),
            entries: Vec::new(),
            expanded: false,
        };
        trace.entries.push(ToolEntry {
            target: Self::sanitize(target),
            detail: Self::sanitize(detail),
        });
        self.items.push(ContentItem::ToolTrace(trace));
    }

    /// Update the latest entry's detail for *tool_name* (tool completion).
    pub fn update_tool_latest(&mut self, tool_name: &str, detail: &str) {
        for item in &mut self.items {
            if let ContentItem::ToolTrace(trace) = item {
                if trace.tool_name == tool_name {
                    if let Some(last) = trace.entries.last_mut() {
                        last.detail = Self::sanitize(detail);
                    }
                    return;
                }
            }
        }
    }

    pub fn tool_trace_names(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ContentItem::ToolTrace(t) => Some(t.tool_name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Toggle expand for *tool_name*; collapse all other tools.
    pub fn toggle_tool_expand(&mut self, tool_name: &str) {
        for item in &mut self.items {
            if let ContentItem::ToolTrace(trace) = item {
                if trace.tool_name == tool_name {
                    trace.expanded = !trace.expanded;
                } else {
                    trace.expanded = false;
                }
            }
        }
    }

    /// Collapse all collapsible non-warning cards and all tool traces —
    /// called on run end (CONTENT_PANE spec). Blocking errors / unapproved
    /// actions / verifier failures (warnings) never collapse.
    pub fn collapse_non_warnings(&mut self) {
        for item in &mut self.items {
            match item {
                ContentItem::Message(msg) => {
                    if msg.collapsible && !msg.warning {
                        msg.collapsed = true;
                    }
                }
                ContentItem::ToolTrace(trace) => trace.expanded = false,
            }
        }
    }

    /// Search the conversation for *query* (Find dialog, v1 scope = 对话):
    /// message content and tool-trace targets/details. Returns the 0-based
    /// indices of matching content items (marker line = index + 1, mirroring
    /// the projection's card-line numbering).
    ///
    /// `use_regex` compiles *query* with the `regex` crate (multi_line for
    /// multiline queries; invalid patterns surface as `Err`). Without regex,
    /// matching is substring `contains` honoring `case_sensitive`.
    pub fn find(
        &self,
        query: &str,
        case_sensitive: bool,
        use_regex: bool,
    ) -> Result<Vec<usize>, String> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let matcher: Box<dyn Fn(&str) -> bool> = if use_regex {
            let re = regex::RegexBuilder::new(query)
                .case_insensitive(!case_sensitive)
                .multi_line(true)
                .build()
                .map_err(|e| format!("正则表达式错误: {e}"))?;
            Box::new(move |h| re.is_match(h))
        } else if case_sensitive {
            Box::new(move |h| h.contains(query))
        } else {
            let q = query.to_lowercase();
            Box::new(move |h| h.to_lowercase().contains(&q))
        };
        let hits: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| match item {
                ContentItem::Message(m) => matcher(&m.content),
                ContentItem::ToolTrace(t) => t
                    .entries
                    .iter()
                    .any(|e| matcher(&e.target) || matcher(&e.detail)),
            })
            .map(|(i, _)| i)
            .collect();
        Ok(hits)
    }
}

/// Marker column entry (right side).
#[derive(Debug, Clone, PartialEq)]
pub struct MarkerEntry {
    pub kind: MarkerKind,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkerKind {
    UserInput,
    SearchHit,
}

/// Content markers — user-input positions + search hits (Find slice #9).
#[derive(Debug, Clone, Default)]
pub struct ContentMarker {
    pub markers: Vec<MarkerEntry>,
}

impl ContentMarker {
    pub fn add_user_input(&mut self, line: usize) {
        self.markers.push(MarkerEntry {
            kind: MarkerKind::UserInput,
            label: format!("▸ L{line}"),
        });
    }

    /// Add a search-hit marker (`● L{n}`, n = 1-based content-item line).
    pub fn add_search_hit(&mut self, line: usize) {
        self.markers.push(MarkerEntry {
            kind: MarkerKind::SearchHit,
            label: format!("● L{line}"),
        });
    }

    /// Drop all search-hit markers (a new Find replaces the previous hits).
    pub fn clear_search_hits(&mut self) {
        self.markers.retain(|m| m.kind != MarkerKind::SearchHit);
    }

    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
}

/// Status bar — one segment per item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusItem {
    pub label: String,
    /// ok=true renders the label normally; ok=false renders it in error
    /// style (e.g. gate block).
    pub ok: bool,
}

/// Bottom status bar (frozen pattern from
/// SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1):
/// `守护 | 网络关闭 | 沙箱严格 | 来源 n/m | 模型 | 空闲`.
#[derive(Debug, Clone)]
pub struct StatusBar {
    pub items: Vec<StatusItem>,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            items: vec![
                StatusItem { label: "守护".into(), ok: true },
                StatusItem { label: "网络关闭".into(), ok: true },
                StatusItem { label: "沙箱严格".into(), ok: true },
                StatusItem { label: "来源 0/0".into(), ok: true },
                StatusItem { label: "模型".into(), ok: true },
                StatusItem { label: "空闲".into(), ok: true },
            ],
        }
    }

    /// Update a segment by label prefix (e.g. "来源" or "空闲").
    pub fn update_item(&mut self, label_prefix: &str, ok: bool) {
        for item in &mut self.items {
            if item.label.starts_with(label_prefix) {
                item.ok = ok;
            }
        }
    }

    /// Set the source-visibility count segment (`来源 n/m`).
    pub fn set_sources(&mut self, available: u64, total: u64) {
        for item in &mut self.items {
            if item.label.starts_with("来源") {
                item.label = format!("来源 {available}/{total}");
                item.ok = available == total;
            }
        }
    }

    /// Set the run-state segment (空闲/预检/运行中/等待审批/完成/失败/已取消/
    /// 无效/恢复中 — slice #10: the restore state must remain replaceable).
    pub fn set_run_state(&mut self, state: &str, ok: bool) {
        for item in &mut self.items {
            if matches!(
                item.label.as_str(),
                "空闲" | "预检" | "运行中" | "等待审批" | "完成" | "失败" | "已取消" | "无效" | "恢复中"
            ) {
                item.label = state.to_string();
                item.ok = ok;
            }
        }
    }

    /// Set the model segment label (模型/off).
    pub fn set_model(&mut self, label: &str) {
        for item in &mut self.items {
            if item.label == "模型" {
                item.label = label.to_string();
            }
        }
    }
}

/// Menu bar row — frozen Chinese menu set (v1: display + selection).
#[derive(Debug, Clone)]
pub struct MenuBar {
    pub items: Vec<String>,
    pub selected: Option<usize>,
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            items: vec![
                "文件".into(),
                "事件".into(),
                "标记".into(),
                "编辑".into(),
                "模型".into(),
                "来源".into(),
                "运行".into(),
                "验证".into(),
                "帮助".into(),
            ],
            selected: None,
        }
    }
}

/// Toolbar button.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolbarButton {
    pub label: String,
    pub enabled: bool,
}

/// Toolbar row — frozen layout:
/// `后退 前进 刷新 停止 打开 验证 属性 | 命令... 查找...`
#[derive(Debug, Clone)]
pub struct Toolbar {
    pub items: Vec<ToolbarButton>,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            items: vec![
                ToolbarButton { label: "后退".into(), enabled: false },
                ToolbarButton { label: "前进".into(), enabled: false },
                ToolbarButton { label: "刷新".into(), enabled: false },
                ToolbarButton { label: "停止".into(), enabled: false },
                ToolbarButton { label: "打开".into(), enabled: false },
                ToolbarButton { label: "验证".into(), enabled: true },
                // Slice #9: Properties (Neutral `p`) and Find (Ctrl+F) are
                // live; the toolbar renders static text, so the flags are
                // informational (render_toolbar ignores state).
                ToolbarButton { label: "属性".into(), enabled: true },
                ToolbarButton { label: "命令...".into(), enabled: true },
                ToolbarButton { label: "查找...".into(), enabled: true },
            ],
        }
    }

    pub fn set_enabled(&mut self, label: &str, enabled: bool) {
        for btn in &mut self.items {
            if btn.label == label {
                btn.enabled = enabled;
            }
        }
    }
}

/// Chat input state — wraps the workspace textarea widget plus history.
/// (TextArea is not Clone; this state is owned by TuiApp.)
#[derive(Debug)]
pub struct ChatInputState {
    pub textarea: TextArea,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
}

impl Default for ChatInputState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatInputState {
    pub fn new() -> Self {
        Self {
            textarea: TextArea::new(),
            history: Vec::new(),
            history_index: None,
        }
    }

    /// Take the current buffer, push it to history, and clear.
    ///
    /// History dedup (Python `AddressBar._push_history` rule): adjacent
    /// duplicates are suppressed — re-submitting the same prompt must not
    /// create a repeated history entry. Empty lines never enter history.
    pub fn take_line(&mut self) -> String {
        let line = self.textarea.text().to_string();
        if !line.trim().is_empty() && self.history.last().map(String::as_str) != Some(line.as_str())
        {
            self.history.push(line.clone());
        }
        self.textarea.set_text("");
        self.history_index = None;
        line
    }

    pub fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_index {
            Some(i) if i > 0 => i - 1,
            _ => self.history.len() - 1,
        };
        self.history_index = Some(idx);
        self.textarea.set_text(&self.history[idx]);
    }

    pub fn history_down(&mut self) {
        let Some(i) = self.history_index else {
            return;
        };
        if i + 1 < self.history.len() {
            self.history_index = Some(i + 1);
            self.textarea.set_text(&self.history[i + 1]);
        } else {
            self.history_index = None;
            self.textarea.set_text("");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_pane_message_lifecycle() {
        let mut pane = ContentPane::default();
        pane.add_user_message("你好");
        pane.add_model_message("回答", 1, false);
        assert_eq!(pane.items.len(), 2);
        assert_eq!(pane.current_model_index, Some(1));

        pane.append_text_delta("，补充");
        let ContentItem::Message(msg) = &pane.items[1] else {
            panic!("expected message");
        };
        assert_eq!(msg.content, "回答，补充");
    }

    #[test]
    fn text_delta_without_model_card_is_noop() {
        let mut pane = ContentPane::default();
        pane.add_user_message("hi");
        pane.append_text_delta("x");
        let ContentItem::Message(msg) = &pane.items[0] else {
            panic!("expected message");
        };
        assert_eq!(msg.content, "hi");
    }

    #[test]
    fn tool_trace_merges_same_tool() {
        let mut pane = ContentPane::default();
        pane.add_or_update_tool_trace("read_file", "cli.py", "运行中");
        pane.add_or_update_tool_trace("read_file", "cli.py", "成功");
        pane.add_or_update_tool_trace("bash", "dir", "运行中");
        assert_eq!(pane.items.len(), 2);
        assert_eq!(pane.tool_trace_names(), vec!["read_file", "bash"]);
        let ContentItem::ToolTrace(t) = &pane.items[0] else {
            panic!("expected tool trace");
        };
        assert_eq!(t.entries.len(), 2);
        assert_eq!(t.entries[1].detail, "成功");
    }

    #[test]
    fn toggle_expand_collapses_others() {
        let mut pane = ContentPane::default();
        pane.add_or_update_tool_trace("read_file", "a", "d");
        pane.add_or_update_tool_trace("bash", "b", "d");
        pane.toggle_tool_expand("bash");
        let ContentItem::ToolTrace(a) = &pane.items[0] else {
            panic!("expected tool trace");
        };
        assert!(!a.expanded);
        let b_expanded = matches!(
            &pane.items[1],
            ContentItem::ToolTrace(t) if t.expanded
        );
        assert!(b_expanded);
        pane.toggle_tool_expand("bash");
        let b_expanded = matches!(
            &pane.items[1],
            ContentItem::ToolTrace(t) if t.expanded
        );
        assert!(!b_expanded);
    }

    #[test]
    fn collapse_non_warnings_keeps_warnings() {
        let mut pane = ContentPane::default();
        pane.add_user_message("q");
        pane.add_model_message("a", 1, false);
        pane.add_system_message("[门控] IPG: block", true);
        pane.add_or_update_tool_trace("bash", "x", "运行中");
        pane.collapse_non_warnings();
        match &pane.items[..] {
            [
                ContentItem::Message(user),
                ContentItem::Message(model),
                ContentItem::Message(warn),
                ContentItem::ToolTrace(trace),
            ] => {
                assert!(user.collapsed);
                assert!(model.collapsed);
                assert!(!warn.collapsed, "warning card must never collapse");
                assert!(!trace.expanded);
            }
            _ => panic!("unexpected items: {:?}", pane.items),
        }
    }

    #[test]
    fn status_bar_frozen_layout_and_updates() {
        let bar = StatusBar::new();
        let labels: Vec<&str> = bar.items.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(
            labels,
            ["守护", "网络关闭", "沙箱严格", "来源 0/0", "模型", "空闲"]
        );

        let mut bar = StatusBar::new();
        bar.set_sources(3, 4);
        assert_eq!(bar.items[3].label, "来源 3/4");
        assert!(!bar.items[3].ok);
        bar.set_run_state("运行中", true);
        assert_eq!(bar.items[5].label, "运行中");
        bar.set_model("off");
        assert_eq!(bar.items[4].label, "off");
    }

    #[test]
    fn chat_input_history() {
        let mut input = ChatInputState::new();
        input.textarea.set_text("第一个问题");
        assert_eq!(input.take_line(), "第一个问题");
        assert!(input.textarea.text().is_empty());
        input.textarea.set_text("第二个问题");
        assert_eq!(input.take_line(), "第二个问题");

        input.history_up();
        assert_eq!(input.textarea.text(), "第二个问题");
        input.history_up();
        assert_eq!(input.textarea.text(), "第一个问题");
        input.history_down();
        assert_eq!(input.textarea.text(), "第二个问题");
        input.history_down();
        assert!(input.textarea.text().is_empty());
    }

    // ── Phase 3 slice #9: history dedup + find + search-hit markers ───────

    #[test]
    fn take_line_dedup_only_vs_last() {
        let mut input = ChatInputState::new();
        // First submit enters history.
        input.textarea.set_text("重复问题");
        input.take_line();
        // Immediate re-submit of the SAME text is suppressed (adjacent dup).
        input.textarea.set_text("重复问题");
        input.take_line();
        assert_eq!(input.history, vec!["重复问题"]);

        // A different text in between allows the same text again
        // (non-adjacent repeats are permitted — Python parity).
        input.textarea.set_text("别的");
        input.take_line();
        input.textarea.set_text("重复问题");
        input.take_line();
        assert_eq!(input.history, vec!["重复问题", "别的", "重复问题"]);

        // Empty/whitespace lines never enter history.
        input.textarea.set_text("   ");
        assert_eq!(input.take_line(), "   ");
        assert_eq!(input.history.len(), 3);
    }

    fn pane_with_items() -> ContentPane {
        let mut pane = ContentPane::default();
        pane.add_user_message("审查 session 列表设计");
        pane.add_model_message("来源可见性需要检查", 1, false);
        pane.add_or_update_tool_trace("read_file", "session.json", "运行中");
        pane
    }

    #[test]
    fn find_matches_message_and_tool_entries() {
        let pane = pane_with_items();
        // Message content match.
        let hits = pane.find("列表设计", true, false).unwrap();
        assert_eq!(hits, vec![0]);
        // Tool trace target match (item 2).
        let hits = pane.find("session.json", true, false).unwrap();
        assert_eq!(hits, vec![2]);
        // No match → empty.
        assert!(pane.find("不存在的词", true, false).unwrap().is_empty());
    }

    #[test]
    fn find_case_sensitivity() {
        let pane = pane_with_items();
        // Case-sensitive: "Session" does not match "session".
        assert!(pane.find("Session", true, false).unwrap().is_empty());
        // Case-insensitive default matches.
        assert_eq!(pane.find("SESSION.JSON", false, false).unwrap(), vec![2]);
    }

    #[test]
    fn find_regex_and_multiline_query() {
        let pane = pane_with_items();
        // Regex alternation.
        let hits = pane.find("列表|可见性", false, true).unwrap();
        assert_eq!(hits, vec![0, 1]);
        // Regex honors case_insensitive=false.
        assert!(pane.find("SESSION", true, true).unwrap().is_empty());
        // Multiline query with (?s) spanning content lines.
        let pane = ContentPane::default();
        let mut pane = pane;
        pane.add_model_message("第一行\n第二行", 1, false);
        let hits = pane.find("第一行(?s:.)*第二行", false, true).unwrap();
        assert_eq!(hits, vec![0]);
        // Invalid regex surfaces as Err.
        let err = pane.find("([", false, true).unwrap_err();
        assert!(err.contains("正则表达式错误"), "{err}");
    }

    #[test]
    fn marker_search_hits_add_and_clear() {
        let mut marker = ContentMarker::default();
        marker.add_user_input(1);
        marker.add_search_hit(3);
        marker.add_search_hit(5);
        assert_eq!(marker.markers.len(), 3);
        assert!(marker.markers[1].label.starts_with("● L3"));

        // A new search replaces previous hits only — user inputs survive.
        marker.clear_search_hits();
        assert_eq!(marker.markers.len(), 1);
        assert_eq!(marker.markers[0].kind, MarkerKind::UserInput);
    }
}
