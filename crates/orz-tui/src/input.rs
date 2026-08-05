//! Keyboard handling — global keys (Tab/Esc cascade/Ctrl+C/Ctrl+Z/F6),
//! chat-input keys delegated to the workspace textarea widget, and dialog
//! keys (Tab/Enter/Esc). Mirror of Python `pt_app.py` bindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Focus, KeyOutcome, TuiApp};
use xai_ratatui_textarea::classify_key_event;

/// Handle one key event against the app state.
pub fn handle_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    // Ctrl+C always quits (Python `q`/Ctrl+C).
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return KeyOutcome::Quit;
    }
    // Ctrl+Z cancels the running session from any focus.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('z') {
        return KeyOutcome::CancelRun;
    }

    match app.focus {
        Focus::Dialog => handle_dialog_key(app, key),
        Focus::Chat => handle_chat_key(app, key),
        Focus::Neutral => handle_neutral_key(app, key),
    }
}

fn handle_dialog_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                if let Some(d) = app.dialog.as_mut() {
                    d.select_prev();
                }
            } else if let Some(d) = app.dialog.as_mut() {
                d.select_next();
            }
        }
        KeyCode::Enter => {
            app.confirm_dialog();
        }
        KeyCode::Esc => {
            app.cancel_dialog();
        }
        _ => {}
    }
    KeyOutcome::Continue
}

fn handle_chat_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => {
            app.focus = Focus::Neutral;
            KeyOutcome::Continue
        }
        KeyCode::F(6) => {
            app.focus = Focus::Neutral;
            KeyOutcome::Continue
        }
        KeyCode::Enter => {
            let line = app.input.take_line();
            if line.is_empty() {
                return KeyOutcome::Continue;
            }
            if line.starts_with('/') {
                for msg in app.dispatch_command(&line) {
                    app.content.add_system_message(&msg, false);
                }
                KeyOutcome::Continue
            } else {
                KeyOutcome::Prompt(line)
            }
        }
        KeyCode::Up => {
            app.input.history_up();
            KeyOutcome::Continue
        }
        KeyCode::Down => {
            app.input.history_down();
            KeyOutcome::Continue
        }
        // Delegate editing keys to the textarea widget; printable chars the
        // classifier does not recognize (IME/composed input) insert directly.
        _ => {
            if classify_key_event(&key).is_none()
                && let KeyCode::Char(c) = key.code
                && !c.is_control()
            {
                app.input.textarea.insert_str(&c.to_string());
                return KeyOutcome::Continue;
            }
            app.input.textarea.input(key);
            KeyOutcome::Continue
        }
    }
}

fn handle_neutral_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Esc | KeyCode::F(6) => {
            app.focus = Focus::Chat;
            KeyOutcome::Continue
        }
        KeyCode::Char('q') => KeyOutcome::Quit,
        KeyCode::Char('/') => {
            app.focus = Focus::Chat;
            app.input.textarea.set_text("/");
            KeyOutcome::Continue
        }
        _ => KeyOutcome::Continue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{PendingPermission, PermissionOutcome};
    use crossterm::event::KeyModifiers as M;
    use std::cell::Cell;
    use std::rc::Rc;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn ctrl_c_quits_from_any_focus() {
        let mut a = TuiApp::new();
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Char('c'), M::CONTROL)),
            KeyOutcome::Quit
        );
        a.focus = Focus::Dialog;
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Char('c'), M::CONTROL)),
            KeyOutcome::Quit
        );
    }

    #[test]
    fn ctrl_z_requests_cancel() {
        let mut a = TuiApp::new();
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Char('z'), M::CONTROL)),
            KeyOutcome::CancelRun
        );
    }

    #[test]
    fn enter_submits_prompt() {
        let mut a = TuiApp::new();
        a.input.textarea.set_text("你好世界");
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Enter, M::NONE)),
            KeyOutcome::Prompt("你好世界".into())
        );
        assert!(a.input.textarea.text().is_empty());
    }

    #[test]
    fn slash_commands_dispatch_inline() {
        let mut a = TuiApp::new();
        a.input.textarea.set_text("/help");
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Enter, M::NONE)),
            KeyOutcome::Continue
        );
        assert!(a.dialog.is_some());
        assert_eq!(a.focus, Focus::Dialog);
    }

    #[test]
    fn unknown_slash_shows_message() {
        let mut a = TuiApp::new();
        a.input.textarea.set_text("/nope");
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let last = a.content.items.last().unwrap();
        assert!(format!("{last:?}").contains("未知命令"));
    }

    #[test]
    fn dialog_tab_cycles_enter_confirms_esc_cancels() {
        fn cell() -> Rc<Cell<Option<PermissionOutcome>>> {
            Rc::new(Cell::new(None))
        }
        fn respond(
            cell: &Rc<Cell<Option<PermissionOutcome>>>,
        ) -> Box<dyn FnOnce(PermissionOutcome)> {
            let cell = cell.clone();
            Box::new(move |o| cell.set(Some(o)))
        }

        let mut a = TuiApp::new();
        // Esc → Cancelled.
        let fired = cell();
        a.show_permission_dialog(PendingPermission {
            tool: "bash".into(),
            args_summary: "dir".into(),
            respond: Some(respond(&fired)),
        });
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::Cancelled));
        assert_eq!(a.focus, Focus::Chat);

        // Tab to Cancel, Enter → Cancelled.
        let fired = cell();
        a.show_permission_dialog(PendingPermission {
            tool: "bash".into(),
            args_summary: "dir".into(),
            respond: Some(respond(&fired)),
        });
        handle_key(&mut a, key(KeyCode::Tab, M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::Cancelled));

        // Default Enter → AllowOnce.
        let fired = cell();
        a.show_permission_dialog(PendingPermission {
            tool: "bash".into(),
            args_summary: "dir".into(),
            respond: Some(respond(&fired)),
        });
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::AllowOnce));
    }

    #[test]
    fn esc_cascade_chat_to_neutral_then_back() {
        let mut a = TuiApp::new();
        assert_eq!(a.focus, Focus::Chat);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Neutral);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Chat);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        handle_key(&mut a, key(KeyCode::Char('q'), M::NONE));
        assert_eq!(handle_key(&mut a, key(KeyCode::Char('q'), M::NONE)), KeyOutcome::Quit);
    }

    #[test]
    fn plain_chars_insert_into_textarea() {
        let mut a = TuiApp::new();
        handle_key(&mut a, key(KeyCode::Char('中'), M::NONE));
        assert_eq!(a.input.textarea.text(), "中");
        handle_key(&mut a, key(KeyCode::Char('a'), M::NONE));
        assert_eq!(a.input.textarea.text(), "中a");
    }

    #[test]
    fn history_arrows() {
        let mut a = TuiApp::new();
        a.input.textarea.set_text("第一个");
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        a.input.textarea.set_text("第二个");
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        a.input.textarea.set_text("第三个");
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        handle_key(&mut a, key(KeyCode::Up, M::NONE));
        assert_eq!(a.input.textarea.text(), "第三个");
        handle_key(&mut a, key(KeyCode::Up, M::NONE));
        assert_eq!(a.input.textarea.text(), "第二个");
        handle_key(&mut a, key(KeyCode::Down, M::NONE));
        assert_eq!(a.input.textarea.text(), "第三个");
    }
}
