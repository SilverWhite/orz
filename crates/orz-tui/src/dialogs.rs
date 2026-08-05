//! Modal dialogs — blocking decisions only (CLI_UI_INTERACTION_MODEL_v0.1:
//! permission approval, verifier disagreement, irreversible actions).
//!
//! Pure UI state — the ACP response channel lives in the app layer, not here.

/// What a dialog action does when confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogAction {
    /// Select an ACP permission option (e.g. "allow-once").
    AllowOnce { option_id: String },
    /// Reject / cancel the request.
    Cancel,
    /// Plain acknowledgement.
    Ok,
}

impl DialogAction {
    pub fn label(&self) -> &'static str {
        match self {
            DialogAction::AllowOnce { .. } => "允许一次",
            DialogAction::Cancel => "取消",
            DialogAction::Ok => "确定",
        }
    }
}

/// A centered modal overlay.
#[derive(Debug, Clone)]
pub struct Dialog {
    pub title: String,
    pub message: Vec<String>,
    pub actions: Vec<DialogAction>,
    pub selected: usize,
    pub visible: bool,
}

impl Dialog {
    pub fn new(title: &str, message: Vec<String>, actions: Vec<DialogAction>) -> Self {
        Self {
            title: title.to_string(),
            message,
            actions,
            selected: 0,
            visible: true,
        }
    }

    pub fn select_next(&mut self) {
        if !self.actions.is_empty() {
            self.selected = (self.selected + 1) % self.actions.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.actions.is_empty() {
            self.selected =
                (self.selected + self.actions.len() - 1) % self.actions.len();
        }
    }

    pub fn selected_action(&self) -> Option<&DialogAction> {
        self.actions.get(self.selected)
    }
}

/// Static help overlay builder (v1: shortcut + command summary).
pub fn help_dialog() -> Dialog {
    Dialog::new(
        "帮助",
        vec![
            "快捷键:".into(),
            "  Tab / Shift+Tab   切换焦点".into(),
            "  Esc               关闭对话框 / 取消".into(),
            "  Ctrl+C            退出".into(),
            "  Ctrl+Z            停止当前运行".into(),
            "  F6                循环主面板".into(),
            "".into(),
            "命令:".into(),
            "  /help   本帮助".into(),
            "  /status 运行状态".into(),
            "  /stop   停止请求".into(),
            "  /open   打开文件".into(),
            "  /toggle-markers 显示/隐藏标记栏".into(),
        ],
        vec![DialogAction::Ok],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_selection_cycles() {
        let mut d = Dialog::new(
            "工具权限请求",
            vec!["bash".into()],
            vec![
                DialogAction::AllowOnce { option_id: "allow-once".into() },
                DialogAction::Cancel,
            ],
        );
        assert_eq!(d.selected, 0);
        d.select_next();
        assert_eq!(d.selected, 1);
        d.select_next();
        assert_eq!(d.selected, 0);
        d.select_prev();
        assert_eq!(d.selected, 1);
    }

    #[test]
    fn labels_are_chinese() {
        assert_eq!(DialogAction::AllowOnce { option_id: "x".into() }.label(), "允许一次");
        assert_eq!(DialogAction::Cancel.label(), "取消");
        assert_eq!(DialogAction::Ok.label(), "确定");
    }
}
