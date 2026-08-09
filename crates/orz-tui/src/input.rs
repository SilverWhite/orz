//! Keyboard handling — global keys (Ctrl+C/Ctrl+Z/Ctrl+F/Alt+H/F1), chat
//! input keys delegated to the workspace textarea widget, dialog keys
//! (Tab/Enter/Esc), explorer navigation (arrows/Enter), modal keys
//! (Tab/←→/Esc/Enter) and the Neutral-focus double-Esc session-list toggle.
//! Mirror of Python `pt_app.py` bindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{Focus, KeyOutcome, TuiApp};
use xai_ratatui_textarea::classify_key_event;

/// Double-Esc (<500 ms) toggles the session list (Python `app.py` P3.4).
pub const DOUBLE_ESC_WINDOW: std::time::Duration = std::time::Duration::from_millis(500);

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
    // Global modal triggers (slice #9): Ctrl+F find; Alt+H / F1 help.
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('f') {
        app.open_find();
        return KeyOutcome::Continue;
    }
    if key.code == KeyCode::F(1)
        || (key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('h'))
    {
        app.open_help();
        return KeyOutcome::Continue;
    }
    // Alt+S opens the snapshot selector (slice #10; parity with Alt+H).
    if key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('s') {
        app.open_snapshots();
        return KeyOutcome::Continue;
    }

    match app.focus {
        Focus::Dialog => handle_dialog_key(app, key),
        Focus::Modal => handle_modal_key(app, key),
        Focus::Chat => handle_chat_key(app, key),
        Focus::Explorer => handle_explorer_key(app, key),
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
            // Nothing was dismissed — stamp for the double-Esc window
            // (slice #9; the next Esc within 500 ms toggles the session
            // list from Neutral).
            app.last_esc = Some(std::time::Instant::now());
            app.focus = Focus::Neutral;
            KeyOutcome::Continue
        }
        KeyCode::F(6) => {
            // Focus cycle (L7-002): chat → explorer → chat; a hidden
            // explorer falls through to Neutral (preserving the v1 escape).
            // Any F6 invalidates the double-Esc window (review D3-4: a
            // focus detour must not let a stale stamp fire later).
            app.last_esc = None;
            app.focus = if app.show_explorer {
                Focus::Explorer
            } else {
                Focus::Neutral
            };
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

fn handle_explorer_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    // Session-list mode: the explorer keys drive the session list, never
    // the hidden tree (review P2-1 — Enter must not fire open_uri on an
    // invisible row).
    if app.explorer.session_mode {
        return handle_session_mode_key(app, key);
    }
    match key.code {
        KeyCode::Esc => {
            app.last_esc = Some(std::time::Instant::now());
            app.focus = Focus::Chat;
        }
        KeyCode::F(6) => {
            app.last_esc = None; // any F6 invalidates the double-Esc window
            app.focus = Focus::Chat;
        }
        KeyCode::Up => app.explorer.move_selection(-1),
        KeyCode::Down => app.explorer.move_selection(1),
        KeyCode::Left => app.explorer.collapse_or_parent(),
        KeyCode::Right => {
            if !app.explorer.expand_or_descend() {
                // Already expanded (or a file row) — descend into the first
                // child / next row.
                app.explorer.move_selection(1);
            }
        }
        KeyCode::Enter => {
            if let Some(uri) = app.explorer.activate() {
                app.open_uri(&uri);
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

/// Session-list navigation shared by the Neutral and Explorer focus
/// (review P2-1): Esc exits to the source tree, ↑/↓ move the selection,
/// Enter reports the replay hint and exits; F6 leaves the pane.
fn handle_session_mode_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    match key.code {
        KeyCode::Esc => exit_session_mode(app),
        KeyCode::F(6) => {
            exit_session_mode(app);
            app.focus = Focus::Chat;
        }
        KeyCode::Up => app.explorer.session_selection_up(),
        KeyCode::Down => app.explorer.session_selection_down(),
        KeyCode::Enter => {
            if let Some(s) = app.explorer.selected_session().cloned() {
                let events = s.run_dir.join("events.jsonl");
                // Read-only view (GRR L4): the TUI never resumes sessions —
                // it reports the replay entry point (Rust's equivalent of
                // Python's `grok session resume` hint). Labeled 运行 (one
                // entry per run journal) — review D2-1 honest labeling.
                app.content.add_system_message(
                    &format!(
                        "运行: {}；重放: orz --replay {}",
                        s.session_id,
                        events.display()
                    ),
                    false,
                );
                exit_session_mode(app);
            }
        }
        _ => {}
    }
    KeyOutcome::Continue
}

/// Leave session mode, restoring the explorer's prior visibility when the
/// double-Esc entry auto-showed it (review D2-3 — a transient gesture must
/// not permanently mutate the user's layout preference).
fn exit_session_mode(app: &mut TuiApp) {
    if app.explorer.session_auto_shown {
        app.show_explorer = false;
    }
    app.explorer.session_auto_shown = false;
    app.explorer.session_mode = false;
    app.last_esc = None;
}

/// Tabbed modals (help / properties): Tab/←→ cycle tabs, Esc closes.
fn handle_tabbed_key(app: &mut TuiApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.close_modal(),
        KeyCode::Tab | KeyCode::Right => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                app.modal_prev_tab();
            } else {
                app.modal_next_tab();
            }
        }
        KeyCode::Left => app.modal_prev_tab(),
        _ => {}
    }
}

/// Find dialog: 5 focus rows — ↑/↓/Tab move focus (Python parity: plain ↑
/// moves the focus row up; the query row never takes arrows for cursor
/// movement), Enter acts on the row (row 0 inserts a newline — multiline
/// query), chars edit the query.
fn handle_find_key(app: &mut TuiApp, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.close_modal(),
        // Shift+Tab and plain ↑ both move the focus row up (Python parity).
        KeyCode::Up | KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
            app.find_focus_prev();
        }
        KeyCode::Up => app.find_focus_prev(),
        KeyCode::Down | KeyCode::Tab => app.find_focus_next(),
        KeyCode::Enter => match app.find_focus_row() {
            0 => app.find_input(key), // newline into the multiline query
            1 => app.find_cycle_scope(),
            2 => app.find_toggle_case(),
            3 => app.find_toggle_regex(),
            _ => {
                for msg in app.execute_find() {
                    app.content.add_system_message(&msg, false);
                }
            }
        },
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Left | KeyCode::Right
            if app.find_focus_row() == 0 =>
        {
            app.find_input(key);
        }
        _ => {}
    }
}

fn handle_modal_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    // Defensive: Focus::Modal without a modal falls back to chat.
    if app.modal.is_none() {
        app.focus = Focus::Chat;
        return KeyOutcome::Continue;
    }
    match &app.modal {
        Some(crate::modals::Modal::Find(_)) => handle_find_key(app, key),
        Some(crate::modals::Modal::Snapshots(_)) => handle_snapshots_key(app, key),
        _ => handle_tabbed_key(app, key),
    }
    KeyOutcome::Continue
}

/// Snapshot selector: ↑/↓ move the selection, Enter is two-step confirm
/// (first press arms the confirm state on the row, second records the
/// restore intent — slice #10), Esc cascades (confirm → selection, else
/// close).
fn handle_snapshots_key(app: &mut TuiApp, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.snapshots_select_prev(),
        KeyCode::Down => app.snapshots_select_next(),
        KeyCode::Enter => {
            let _ = app.snapshots_confirm();
        }
        KeyCode::Esc => app.snapshots_esc(),
        _ => {}
    }
}

fn handle_neutral_key(app: &mut TuiApp, key: KeyEvent) -> KeyOutcome {
    // Session-list mode: Esc/↑/↓/Enter route to the shared session handler
    // (review P2-1); q, /, p and F6 keep their Neutral meanings.
    if app.explorer.session_mode {
        match key.code {
            KeyCode::Esc | KeyCode::Up | KeyCode::Down | KeyCode::Enter => {
                return handle_session_mode_key(app, key);
            }
            _ => {}
        }
    }
    match key.code {
        KeyCode::Esc => {
            // The stamp's lifetime is scoped to "currently in Neutral" — a
            // double-Esc within 500 ms toggles the session list (Python
            // app.py P3.4); a stale stamp (F6 detour) must not fire
            // (review P3-1 — F6 below clears it).
            if app
                .last_esc
                .is_some_and(|t| t.elapsed() < DOUBLE_ESC_WINDOW)
            {
                // Double-Esc toggles the run-history list; a hidden
                // explorer auto-shows so the list is visible (the exit
                // path reverts the auto-show — review D2-3).
                let was_hidden = !app.show_explorer;
                app.explorer.reload_sessions(&app.cwd);
                app.explorer.session_mode = true;
                app.explorer.session_auto_shown = was_hidden;
                app.show_explorer = true;
                app.content
                    .add_system_message("运行历史（Esc 返回来源树）", false);
                app.last_esc = None;
            } else {
                app.last_esc = Some(std::time::Instant::now());
                app.focus = Focus::Chat;
            }
            KeyOutcome::Continue
        }
        KeyCode::F(6) => {
            // Leaving Neutral invalidates the double-Esc stamp (P3-1).
            app.last_esc = None;
            app.focus = Focus::Chat;
            KeyOutcome::Continue
        }
        KeyCode::Char('q') => KeyOutcome::Quit,
        KeyCode::Char('p') => {
            // Properties (Python `p` binding — only meaningful in the
            // neutral focus; chat typing never reaches here).
            app.open_properties();
            KeyOutcome::Continue
        }
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
        assert!(a.modal.is_some(), "/help opens the tabbed help modal");
        assert_eq!(a.focus, Focus::Modal);
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
        a.show_permission_dialog(PendingPermission::new("bash", "dir", Some(respond(&fired))));
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::Cancelled));
        assert_eq!(a.focus, Focus::Chat);

        // Tab to Cancel, Enter → Cancelled.
        let fired = cell();
        a.show_permission_dialog(PendingPermission::new("bash", "dir", Some(respond(&fired))));
        handle_key(&mut a, key(KeyCode::Tab, M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::Cancelled));

        // Default Enter → AllowOnce.
        let fired = cell();
        a.show_permission_dialog(PendingPermission::new("bash", "dir", Some(respond(&fired))));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(fired.get(), Some(PermissionOutcome::AllowOnce));
    }

    /// Phase 3 slice #7: Enter on `/stop` while running marks the runner's
    /// cancel intent (key handling stays synchronous; the runner owns the
    /// async cancel).
    #[test]
    fn stop_slash_dispatch_sets_pending_when_running() {
        let mut a = TuiApp::new();
        a.running = true;
        a.input.textarea.set_text("/stop");
        let outcome = handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(outcome, KeyOutcome::Continue);
        assert!(a.stop_pending, "/stop Enter while running marks intent");
    }

    // ── Phase 3 slice #10: snapshot selector ────────────────────────────

    /// Hermetic cwd with two sealed `snapshot_created` journals (two runs
    /// of the same session — navigation needs >1 row).
    fn snapshot_fixture_cwd() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-input-snapshots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        for (n, h) in [(0u32, "a".repeat(64)), (1, "b".repeat(64))] {
            let run_id = format!("RUN-a1b2c3d4-{n}");
            let runs = dir.join(".gsa").join("runs").join(&run_id);
            std::fs::create_dir_all(&runs).unwrap();
            let mut ev = orz_assurance::journal::RunEvent::new_v01(
                run_id.clone(),
                0,
                orz_assurance::journal::EventType::SnapshotCreated,
                "m".into(),
                None,
                "run-event-v0.1.schema.json".into(),
                serde_json::json!({"tool": "search_replace", "targets": ["a.txt"], "snapshot_hash": h}),
                orz_assurance::journal::Redaction::None,
                "2026-08-05T00:00:00Z".into(),
            );
            orz_assurance::seal_event(&mut ev).unwrap();
            std::fs::write(
                runs.join("events.jsonl"),
                format!("{}\n", serde_json::to_string(&ev).unwrap()),
            )
            .unwrap();
        }
        dir
    }

    #[test]
    fn alt_s_opens_snapshot_selector() {
        let dir = snapshot_fixture_cwd();
        let mut a = TuiApp::new();
        a.cwd = dir.clone();
        handle_key(&mut a, key(KeyCode::Char('s'), M::ALT));
        assert!(matches!(a.modal, Some(crate::modals::Modal::Snapshots(_))));
        assert_eq!(a.focus, Focus::Modal);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn alt_s_refused_while_permission_showing() {
        let mut a = TuiApp::new();
        a.show_permission_dialog(crate::app::PendingPermission::new("bash", "dir", None));
        handle_key(&mut a, key(KeyCode::Char('s'), M::ALT));
        assert!(a.modal.is_none(), "permission flow is blocking");
    }

    #[test]
    fn snapshots_slash_dispatch_opens_selector() {
        let dir = snapshot_fixture_cwd();
        let mut a = TuiApp::new();
        a.cwd = dir.clone();
        a.input.textarea.set_text("/snapshots");
        let outcome = handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(outcome, KeyOutcome::Continue);
        assert!(matches!(a.modal, Some(crate::modals::Modal::Snapshots(_))));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshot_selector_keys_navigate_confirm_and_esc() {
        let dir = snapshot_fixture_cwd();
        let mut a = TuiApp::new();
        a.cwd = dir.clone();
        handle_key(&mut a, key(KeyCode::Char('s'), M::ALT)); // open
        // Runs sort newest first — index 0 is -1 (hash b…), index 1 is -0
        // (hash a…). Down moves to the older row.
        let crate::modals::Modal::Snapshots(sel) = a.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert_eq!(sel.entries[0].run_id, "RUN-a1b2c3d4-1");
        handle_key(&mut a, key(KeyCode::Down, M::NONE));
        let crate::modals::Modal::Snapshots(sel) = a.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert_eq!(sel.selected, 1);
        assert_eq!(sel.entries[sel.selected].run_id, "RUN-a1b2c3d4-0");
        // First Enter arms confirm; second records the intent and closes.
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let crate::modals::Modal::Snapshots(sel) = a.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert!(sel.confirm, "first Enter arms the confirm state");
        assert!(a.pending_restore.is_none());
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(a.pending_restore.as_deref(), Some("a".repeat(64).as_str()));
        assert!(a.modal.is_none(), "second Enter closes the selector");

        // Reopen; Esc from the confirm state cascades back, then closes.
        handle_key(&mut a, key(KeyCode::Char('s'), M::ALT));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE)); // confirm
        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // cascade back
        let crate::modals::Modal::Snapshots(sel) = a.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert!(!sel.confirm);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // close
        assert!(a.modal.is_none());
        assert_eq!(a.focus, Focus::Chat);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tab_is_a_noop_in_snapshot_selector() {
        let dir = snapshot_fixture_cwd();
        let mut a = TuiApp::new();
        a.cwd = dir.clone();
        handle_key(&mut a, key(KeyCode::Char('s'), M::ALT));
        handle_key(&mut a, key(KeyCode::Tab, M::NONE));
        let crate::modals::Modal::Snapshots(sel) = a.modal.as_ref().unwrap() else {
            panic!("snapshots modal");
        };
        assert_eq!(sel.selected, 0, "Tab must not move the selection");
        assert!(!sel.confirm);
        assert!(a.modal.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The Esc cascade (Chat→Neutral→Chat) with a stale double-Esc stamp —
    /// a fresh stamp (within 500 ms) instead toggles the session list (see
    /// double_esc_toggles_session_mode).
    #[test]
    fn esc_cascade_chat_to_neutral_then_back() {
        let mut a = TuiApp::new();
        assert_eq!(a.focus, Focus::Chat);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Neutral);
        // A stamp outside the double-Esc window falls through to Chat.
        a.last_esc = Some(std::time::Instant::now() - crate::input::DOUBLE_ESC_WINDOW * 2);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Chat);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Neutral);
        handle_key(&mut a, key(KeyCode::Char('q'), M::NONE));
        assert_eq!(
            handle_key(&mut a, key(KeyCode::Char('q'), M::NONE)),
            KeyOutcome::Quit
        );
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

    // ── Phase 3 slice #9: modal triggers / explorer / session list ────────

    #[test]
    fn ctrl_f_alt_h_f1_open_modals() {
        let mut a = TuiApp::new();
        handle_key(&mut a, key(KeyCode::Char('f'), M::CONTROL));
        assert!(matches!(a.modal, Some(crate::modals::Modal::Find(_))));
        assert_eq!(a.focus, Focus::Modal);
        a.close_modal();

        handle_key(&mut a, key(KeyCode::Char('h'), M::ALT));
        assert!(matches!(a.modal, Some(crate::modals::Modal::Help(_))));
        a.close_modal();

        handle_key(&mut a, key(KeyCode::F(1), M::NONE));
        assert!(matches!(a.modal, Some(crate::modals::Modal::Help(_))));
    }

    #[test]
    fn f6_cycles_chat_explorer_neutral() {
        let mut a = TuiApp::new();
        a.show_explorer = true;
        handle_key(&mut a, key(KeyCode::F(6), M::NONE));
        assert_eq!(a.focus, Focus::Explorer);
        handle_key(&mut a, key(KeyCode::F(6), M::NONE));
        assert_eq!(a.focus, Focus::Chat);

        // Explorer hidden → F6 falls through to Neutral (v1 escape).
        a.show_explorer = false;
        handle_key(&mut a, key(KeyCode::F(6), M::NONE));
        assert_eq!(a.focus, Focus::Neutral);
    }

    #[test]
    fn explorer_keys_navigate_and_open() {
        let mut a = TuiApp::new();
        a.show_explorer = true;
        a.explorer.loaded = true;
        a.explorer.tree = vec![crate::explorer::TreeNode {
            label: "src".into(),
            children: vec![crate::explorer::TreeNode {
                label: "lib.rs".into(),
                children: vec![],
                expanded: false,
                uri: Some("file:///x/lib.rs".into()),
            }],
            expanded: false,
            uri: None,
        }];
        a.focus = Focus::Explorer;

        // → expands the dir; ↓ moves onto the child; Enter opens it.
        handle_key(&mut a, key(KeyCode::Right, M::NONE));
        assert!(a.explorer.tree[0].expanded);
        handle_key(&mut a, key(KeyCode::Down, M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert_eq!(
            a.nav_back.last().map(String::as_str),
            Some("file:///x/lib.rs")
        );
        assert!(
            a.content.items.iter().any(|i| {
                matches!(
                    i,
                    crate::view_model::ContentItem::Message(m)
                        if m.content.contains("[打开] file:///x/lib.rs")
                )
            }),
            "open message surfaced"
        );

        // ← collapses the dir; Esc returns to chat.
        a.explorer.selected = 0;
        handle_key(&mut a, key(KeyCode::Left, M::NONE));
        assert!(!a.explorer.tree[0].expanded);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert_eq!(a.focus, Focus::Chat);
    }

    #[test]
    fn double_esc_toggles_session_mode_and_autoshows_explorer() {
        let mut a = TuiApp::new();
        // Hermetic cwd — no real .gsa reads.
        let base = std::env::temp_dir().join(format!("orz-tui-esc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        a.cwd = base.clone();

        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // Chat → Neutral (stamp)
        assert_eq!(a.focus, Focus::Neutral);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // double → session mode
        assert!(a.explorer.session_mode);
        assert!(a.show_explorer, "hidden explorer auto-shows");
        assert_eq!(a.focus, Focus::Neutral);

        // First Esc in session mode returns to the source tree — and the
        // auto-show is REVERTED (review D2-3: a transient gesture must not
        // permanently mutate the layout preference).
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert!(!a.explorer.session_mode);
        assert!(!a.show_explorer, "auto-show reverted on exit");
        assert!(!a.explorer.session_auto_shown);
        assert_eq!(a.focus, Focus::Neutral);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn stale_esc_stamp_does_not_toggle() {
        let mut a = TuiApp::new();
        let base = std::env::temp_dir().join(format!("orz-tui-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();
        a.cwd = base.clone();

        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // Chat → Neutral (stamp)
        a.last_esc = Some(std::time::Instant::now() - DOUBLE_ESC_WINDOW * 2);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE)); // stale → back to Chat
        assert!(!a.explorer.session_mode);
        assert_eq!(a.focus, Focus::Chat);
        let _ = std::fs::remove_dir_all(&base);
    }

    /// Review P2-1: explorer-focus keys must drive the session list (not
    /// the hidden tree) while session mode is active — Enter reports the
    /// replay hint and never fires open_uri on an invisible tree row.
    #[test]
    fn explorer_focus_routes_keys_to_session_list_when_session_mode() {
        let mut a = TuiApp::new();
        let base = std::env::temp_dir().join(format!("orz-tui-sessfx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let run_dir = base.join(".gsa").join("runs").join("RUN-a1b2c3d4-0");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::write(run_dir.join("events.jsonl"), "").unwrap();
        a.cwd = base.clone();
        a.show_explorer = true;
        a.explorer.loaded = true;
        a.explorer.tree = vec![crate::explorer::TreeNode {
            label: "文件.txt".into(),
            children: vec![],
            expanded: false,
            uri: Some("file:///x/文件.txt".into()),
        }];
        a.explorer.session_mode = true;
        a.explorer.reload_sessions(&a.cwd);
        assert_eq!(a.explorer.sessions.len(), 1);
        a.focus = Focus::Explorer;

        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert!(!a.explorer.session_mode, "Enter exits session mode");
        assert!(
            a.nav_back.is_empty(),
            "session Enter must not open the hidden tree row"
        );
        let system: Vec<String> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                crate::view_model::ContentItem::Message(m) if m.role == "系统" => {
                    Some(m.content.clone())
                }
                _ => None,
            })
            .collect();
        assert!(
            system.iter().any(|m| m.contains("orz --replay")),
            "replay hint surfaced: {system:?}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn session_enter_shows_replay_message_only() {
        let mut a = TuiApp::new();
        let base = std::env::temp_dir().join(format!("orz-tui-sess-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let run_dir = base.join(".gsa").join("runs").join("RUN-a1b2c3d4-0");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::write(run_dir.join("events.jsonl"), "").unwrap();
        a.cwd = base.clone();
        a.explorer.session_mode = true;
        a.explorer.reload_sessions(&a.cwd);
        assert_eq!(a.explorer.sessions.len(), 1);
        a.focus = Focus::Neutral;

        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        assert!(!a.explorer.session_mode, "Enter exits session mode");
        let system: Vec<String> = a
            .content
            .items
            .iter()
            .filter_map(|i| match i {
                crate::view_model::ContentItem::Message(m) if m.role == "系统" => {
                    Some(m.content.clone())
                }
                _ => None,
            })
            .collect();
        assert!(
            system.iter().any(|m| m.contains("orz --replay")),
            "replay hint surfaced: {system:?}"
        );
        // Read-only view: no session store was created (GRR L4).
        assert!(!base.join(".gsa").join("sessions").exists());
        assert!(run_dir.join("events.jsonl").is_file());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn find_dialog_keys_edit_query_and_cycle_rows() {
        let mut a = TuiApp::new();
        handle_key(&mut a, key(KeyCode::Char('f'), M::CONTROL));
        // Row 0: chars edit the query; Enter inserts a newline (multiline).
        handle_key(&mut a, key(KeyCode::Char('审'), M::NONE));
        handle_key(&mut a, key(KeyCode::Char('查'), M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let crate::modals::Modal::Find(f) = a.modal.as_ref().unwrap() else {
            panic!("find modal");
        };
        assert_eq!(f.query.text(), "审查\n");
        // Tab to row 1: Enter cycles the scope.
        handle_key(&mut a, key(KeyCode::Tab, M::NONE));
        assert_eq!(a.find_focus_row(), 1);
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let crate::modals::Modal::Find(f) = a.modal.as_ref().unwrap() else {
            panic!("find modal");
        };
        assert_eq!(f.scope, 1);
        // Row 2/3 toggles.
        handle_key(&mut a, key(KeyCode::Down, M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let crate::modals::Modal::Find(f) = a.modal.as_ref().unwrap() else {
            panic!("find modal");
        };
        assert!(f.case_sensitive);
        handle_key(&mut a, key(KeyCode::Down, M::NONE));
        handle_key(&mut a, key(KeyCode::Enter, M::NONE));
        let crate::modals::Modal::Find(f) = a.modal.as_ref().unwrap() else {
            panic!("find modal");
        };
        assert!(f.use_regex);
        // Plain ↑ moves the focus row up (Python parity — review D3-2).
        handle_key(&mut a, key(KeyCode::Up, M::NONE));
        assert_eq!(a.find_focus_row(), 2);
        // Esc closes.
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert!(a.modal.is_none());
        assert_eq!(a.focus, Focus::Chat);
    }

    #[test]
    fn tabbed_modal_keys_cycle_tabs_and_esc_closes() {
        let mut a = TuiApp::new();
        handle_key(&mut a, key(KeyCode::F(1), M::NONE));
        let crate::modals::Modal::Help(h) = a.modal.as_ref().unwrap() else {
            panic!("help modal");
        };
        assert_eq!(h.sheet.active, 0);
        handle_key(&mut a, key(KeyCode::Tab, M::NONE));
        handle_key(&mut a, key(KeyCode::Right, M::NONE));
        let crate::modals::Modal::Help(h) = a.modal.as_ref().unwrap() else {
            panic!("help modal");
        };
        assert_eq!(h.sheet.active, 2);
        handle_key(&mut a, key(KeyCode::Left, M::NONE));
        let crate::modals::Modal::Help(h) = a.modal.as_ref().unwrap() else {
            panic!("help modal");
        };
        assert_eq!(h.sheet.active, 1);
        handle_key(&mut a, key(KeyCode::Esc, M::NONE));
        assert!(a.modal.is_none());
    }

    #[test]
    fn p_key_opens_properties_from_neutral() {
        let mut a = TuiApp::new();
        a.focus = Focus::Neutral;
        handle_key(&mut a, key(KeyCode::Char('p'), M::NONE));
        assert!(matches!(a.modal, Some(crate::modals::Modal::Properties(_))));
        assert_eq!(a.focus, Focus::Modal);
        // p in chat types normally (never reaches here — chat handles chars).
        let mut b = TuiApp::new();
        handle_key(&mut b, key(KeyCode::Char('p'), M::NONE));
        assert_eq!(b.input.textarea.text(), "p");
    }
}
