//! Terminal window/tab title (Phase 3 slice #9) — `orz — {run state}` via
//! the OSC 0 sequence (VSCODE_SHELL_AND_VISIBILITY_EXTENSIONS_v0.1 §3:
//! app name + run state, restored on exit).
//!
//! `title_for` is a pure state mapping (unit-testable); `set_title` /
//! `restore_title` are the only terminal-touching functions. Anti-rule
//! (VS:170): the title never contains prompt text or other sensitive data —
//! only status labels and the turn counter.

use std::io;

use crate::app::TuiApp;

/// Derive the window title from the app state.
///
/// Priority: gate block > waiting approval > running > terminal outcomes >
/// preflight > idle. Terminal titles keep their ✓/✗ markers (Python parity).
pub fn title_for(app: &TuiApp) -> String {
    if let Some(gate) = &app.gate_block {
        return format!("orz — ⚠ {gate}");
    }
    let state = app.status.items[5].label.as_str();
    let title = match state {
        "等待审批" => "等待审批".to_string(),
        "运行中" => format!("运行中 [turn {}]", app.turn_counter),
        "完成" => "完成 ✓".to_string(),
        "失败" => "失败 ✗".to_string(),
        "已取消" => "已取消 ✗".to_string(),
        "无效" => "无效 ✗".to_string(),
        "预检" => "预检…".to_string(),
        _ => "就绪".to_string(),
    };
    format!("orz — {title}")
}

/// Write the OSC 0 title sequence (`\x1b]0;{title}\x07`) via crossterm's
/// `SetTitle` command (works on Windows and Unix; BEL-terminated on both).
pub fn set_title(title: &str) {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::terminal::SetTitle(title.to_string())
    );
}

/// Restore the previous title — crossterm has no get-title API, so the
/// restore target is empty (Python parity: `_restore_terminal_title`).
pub fn restore_title() {
    set_title("");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::TuiApp;
    use crate::events::TuiEvent;

    #[test]
    fn title_state_mapping() {
        let mut app = TuiApp::new();
        assert_eq!(title_for(&app), "orz — 就绪");

        app.status.set_run_state("运行中", true);
        app.turn_counter = 3;
        assert_eq!(title_for(&app), "orz — 运行中 [turn 3]");

        app.status.set_run_state("等待审批", true);
        assert_eq!(title_for(&app), "orz — 等待审批");

        // Gate block wins over any run state.
        app.gate_block = Some("IPG".into());
        assert_eq!(title_for(&app), "orz — ⚠ IPG");

        // RunStarted clears the gate block.
        app.accept_event(TuiEvent::RunStarted {
            prompt: "x".into(),
            timestamp: String::new(),
        });
        assert!(app.gate_block.is_none());

        for (state, expect) in [
            ("完成", "orz — 完成 ✓"),
            ("失败", "orz — 失败 ✗"),
            ("已取消", "orz — 已取消 ✗"),
            ("无效", "orz — 无效 ✗"),
            ("预检", "orz — 预检…"),
        ] {
            app.status.set_run_state(state, state == "完成" || state == "已取消");
            assert_eq!(title_for(&app), expect, "state {state}");
        }
    }

    #[test]
    fn gate_block_set_on_block_cleared_on_run_started() {
        let mut app = TuiApp::new();
        app.accept_event(TuiEvent::GateDecision {
            gate: "来源".into(),
            decision: "block".into(),
            reason: None,
            tools: vec![],
        });
        assert_eq!(app.gate_block.as_deref(), Some("来源"));
        app.accept_event(TuiEvent::RunStarted {
            prompt: "x".into(),
            timestamp: String::new(),
        });
        assert!(app.gate_block.is_none());

        // Non-block decisions never set the flag.
        app.accept_event(TuiEvent::GateDecision {
            gate: "来源".into(),
            decision: "allow".into(),
            reason: None,
            tools: vec![],
        });
        assert!(app.gate_block.is_none());
    }
}
