//! Modals — the tabbed sheets (Help / Properties) and the Find dialog
//! (Phase 3 slice #9). One visible modal at a time, mutually exclusive with
//! the permission dialog (app.rs enforces; permission wins and queues).
//!
//! Python reference: `HelpOverlay` / `PropertiesSheet` / `FindDialog`
//! (widgets.py) — 6 tabs with `▶Tab` markers, footer
//! `Esc 关闭  ←→ 切换标签`, Find's 5 focus rows.

use xai_ratatui_textarea::textarea::TextArea;

/// One tabbed-sheet tab: key-value fields, or free text content.
#[derive(Debug, Clone)]
pub struct Tab {
    pub name: String,
    pub fields: Vec<(String, String)>,
    pub content: Option<String>,
}

/// A tabbed modal sheet (Properties + Help share the form — design doc
/// SUP:233 "HelpOverlay may reuse the PropertiesSheet tab form").
#[derive(Debug, Clone)]
pub struct TabbedSheet {
    pub title: String,
    pub tabs: Vec<Tab>,
    pub active: usize,
}

impl TabbedSheet {
    pub fn new(title: &str, tabs: Vec<Tab>) -> Self {
        Self {
            title: title.to_string(),
            tabs,
            active: 0,
        }
    }

    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
        }
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }
}

/// Help overlay — 6 frozen tabs: 快捷键 / 命令 / 模型 / 审批 / 终端 / 来源.
#[derive(Debug, Clone)]
pub struct HelpOverlay {
    pub sheet: TabbedSheet,
}

impl HelpOverlay {
    /// Build the help sheet. The 命令 tab is derived from the slash-command
    /// registry (single source of truth); 模型 reads the current adapter
    /// label from the status bar.
    pub fn new(model_label: &str) -> Self {
        let commands: Vec<(String, String)> =
            crate::commands::CommandRegistry::with_builtins()
                .search("/") // all builtins, deterministic slash order
                .into_iter()
                .map(|d| (d.slash.to_string(), format!("{} — {}", d.name_zh, d.description_zh)))
                .collect();
        Self {
            sheet: TabbedSheet::new(
                "帮助",
                vec![
                    Tab {
                        name: "快捷键".into(),
                        fields: Vec::new(),
                        content: Some(
                            "Tab / F6      切换焦点（聊天 / 探索器 / 中性）\n\
                             Esc           关闭弹窗 / 对话框 / 级联返回\n\
                             双 Esc        运行历史列表（中立焦点）\n\
                             Enter         提交 / 激活选中项\n\
                             ↑ ↓           输入历史 / 树与列表移动\n\
                             ← →           树折叠/展开 / 弹窗标签切换\n\
                             Ctrl+C        退出\n\
                             Ctrl+Z        取消当前运行\n\
                             Ctrl+F        查找\n\
                             Alt+H / F1    帮助\n\
                             p（中性焦点） 属性\n\
                             /             命令输入"
                                .into(),
                        ),
                    },
                    Tab {
                        name: "命令".into(),
                        fields: commands,
                        content: None,
                    },
                    Tab {
                        name: "模型".into(),
                        fields: vec![
                            ("当前适配器".into(), model_label.to_string()),
                            ("可用模型".into(), "—".into()),
                            ("推理强度".into(), "—".into()),
                            ("Plan 模式".into(), "—".into()),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "审批".into(),
                        fields: vec![
                            ("Plan 审批".into(), "计划提出后批准执行".into()),
                            ("Action 审批".into(), "工具权限对话框（允许一次/取消）".into()),
                            ("审批策略".into(), "手动".into()),
                            ("Ctrl+Z".into(), "取消当前运行".into()),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "终端".into(),
                        fields: vec![
                            ("系统终端".into(), "Windows Terminal / ConPTY".into()),
                            ("VS Code".into(), "集成终端（标题自动刷新）".into()),
                            ("标题格式".into(), "orz — 运行状态（OSC 0 序列）".into()),
                            ("异常提示".into(), "视口过小时显示提示".into()),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "来源".into(),
                        fields: vec![
                            ("可见性等级".into(), "来源 n/m（工具检查）".into()),
                            ("证据等级".into(), "门控决定决定等级".into()),
                            ("Gate 检查".into(), "IPG / 工具可用性 / 停滞守卫".into()),
                            ("检索命令".into(), "（v1 未接线）".into()),
                        ],
                        content: None,
                    },
                ],
            ),
        }
    }
}

/// Find dialog — 5 focus rows: query / scope / case / regex / actions.
/// Multiline query via the textarea (design doc SUP:121: long/multiline
/// searches live in the dialog).
#[derive(Debug)]
pub struct FindDialog {
    pub query: TextArea,
    pub scopes: Vec<&'static str>,
    pub scope: usize,
    pub case_sensitive: bool,
    pub use_regex: bool,
    pub focus_row: usize,
}

/// Default scopes (Python parity — v1 scans the conversation for all of
/// them; the selector is the frozen UI surface).
pub const FIND_SCOPES: [&str; 5] = ["对话", "项目文档", "当前文件", "运行/制品", "索引来源"];

impl Default for FindDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl FindDialog {
    pub fn new() -> Self {
        Self {
            query: TextArea::new(),
            scopes: FIND_SCOPES.to_vec(),
            scope: 0,
            case_sensitive: false,
            use_regex: false,
            focus_row: 0,
        }
    }

    pub fn cycle_scope(&mut self) {
        if !self.scopes.is_empty() {
            self.scope = (self.scope + 1) % self.scopes.len();
        }
    }

    pub fn focus_next(&mut self) {
        self.focus_row = (self.focus_row + 1) % 5;
    }

    pub fn focus_prev(&mut self) {
        self.focus_row = (self.focus_row + 4) % 5;
    }
}

/// Properties sheet — the selected object's metadata (v1: the current
/// run/session, snapshotted at open time).
#[derive(Debug, Clone)]
pub struct PropertiesSheet {
    pub sheet: TabbedSheet,
}

impl PropertiesSheet {
    /// Snapshot of the app's live state at open time (the dialog holds no
    /// borrows; fields it cannot answer show `—`).
    pub fn from_app(app: &crate::app::TuiApp) -> Self {
        let runs_dir = app.cwd.join(".gsa").join("runs");
        Self {
            sheet: TabbedSheet::new(
                "属性",
                vec![
                    Tab {
                        name: "常规".into(),
                        fields: vec![
                            ("会话 ID".into(), app.session_id.clone().unwrap_or_else(|| "—".into())),
                            ("工作目录".into(), app.cwd.display().to_string()),
                            ("运行目录".into(), runs_dir.display().to_string()),
                            ("提示数".into(), app.turn_counter.to_string()),
                            ("模型".into(), app.status.items[4].label.clone()),
                            ("状态".into(), app.status.items[5].label.clone()),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "溯源".into(),
                        fields: vec![
                            ("事件数".into(), app.events_log.len().to_string()),
                            ("消息数".into(), app.content.items.len().to_string()),
                            ("标记数".into(), app.marker.markers.len().to_string()),
                            ("后退栈".into(), app.nav_back.len().to_string()),
                            ("前进栈".into(), app.nav_forward.len().to_string()),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "可见性".into(),
                        fields: vec![
                            (
                                "探索器".into(),
                                if app.show_explorer { "显示".into() } else { "隐藏".into() },
                            ),
                            (
                                "标记栏".into(),
                                if app.show_marker { "显示".into() } else { "隐藏".into() },
                            ),
                            (
                                "事件日志上限".into(),
                                app.max_events_log.to_string(),
                            ),
                        ],
                        content: None,
                    },
                    Tab {
                        name: "依赖".into(),
                        fields: vec![("—".into(), "—".into())],
                        content: None,
                    },
                    Tab {
                        name: "验证".into(),
                        fields: vec![("—".into(), "—".into())],
                        content: None,
                    },
                    Tab {
                        name: "历史".into(),
                        fields: vec![("—".into(), "—".into())],
                        content: None,
                    },
                ],
            ),
        }
    }
}

/// All modal kinds — one visible at a time. The Find dialog carries a
/// textarea (~576 B), so it is boxed to keep the enum small
/// (clippy::large_enum_variant).
#[derive(Debug)]
pub enum Modal {
    Help(HelpOverlay),
    Find(Box<FindDialog>),
    Properties(PropertiesSheet),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sheet_tabs_cycle_wraparound() {
        let mut sheet = TabbedSheet::new("t", vec![Tab { name: "a".into(), fields: vec![], content: None }, Tab { name: "b".into(), fields: vec![], content: None }]);
        assert_eq!(sheet.active, 0);
        sheet.next_tab();
        assert_eq!(sheet.active, 1);
        sheet.next_tab();
        assert_eq!(sheet.active, 0, "next wraps around");
        sheet.prev_tab();
        assert_eq!(sheet.active, 1, "prev wraps around");
        assert_eq!(sheet.active_tab().unwrap().name, "b");
    }

    #[test]
    fn help_has_six_frozen_tabs_and_command_list() {
        let help = HelpOverlay::new("off");
        assert_eq!(
            help.sheet.tabs.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["快捷键", "命令", "模型", "审批", "终端", "来源"]
        );
        // 命令 tab derives from the registry (8 builtins incl. /properties).
        let commands = &help.sheet.tabs[1];
        let slashes: Vec<&str> = commands.fields.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(slashes.len(), 8);
        assert!(slashes.contains(&"/toggle-explorer"));
        assert!(slashes.contains(&"/properties"));
        // 模型 tab carries the live adapter label.
        let model = &help.sheet.tabs[2];
        assert!(model.fields.iter().any(|(_, v)| v == "off"));
        // 快捷键 tab documents the double-Esc entry (review D2-4:
        // discoverability — the session list has no other entry point).
        let shortcuts = help.sheet.tabs[0].content.as_ref().unwrap();
        assert!(shortcuts.contains("双 Esc"), "double-Esc discoverable in Help");
    }

    #[test]
    fn find_dialog_rows_cycle_and_scope_wraps() {
        let mut f = FindDialog::new();
        assert_eq!(f.focus_row, 0);
        f.focus_next();
        f.focus_next();
        assert_eq!(f.focus_row, 2);
        f.focus_prev();
        assert_eq!(f.focus_row, 1);
        // 5 rows — cycling forward from 4 wraps to 0.
        f.focus_row = 4;
        f.focus_next();
        assert_eq!(f.focus_row, 0);
        // Scope cycles through the 5 defaults.
        assert_eq!(f.scope, 0);
        f.cycle_scope();
        assert_eq!(f.scopes[f.scope], "项目文档");
        f.scope = 4;
        f.cycle_scope();
        assert_eq!(f.scope, 0);
    }
}
