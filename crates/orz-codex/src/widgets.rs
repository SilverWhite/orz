//! Rendering for the codex-style fallback TUI (Phase 3 slice #12).
//!
//! Three bands only — message list, prompt input, status line — plus the
//! approval dialog overlay. Deliberately no assurance panels (design §2.4):
//! the renderer never touches explorer/properties/find/markers/snapshots.
//! Width/CJK helpers are reused from orz-tui (`theme` module — not copied).

use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::{Frame, Terminal};
use orz_tui::theme::{sanitize_text, truncate_to_width};

use crate::app::{ApprovalDialog, CodexApp, Role, RunState};

/// Minimum viewport for the fallback TUI (a `视口过小` notice replaces the
/// whole frame below it).
pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;

/// Render the app to the terminal (pure over the app state — snapshot tests
/// drive it with a TestBackend).
pub fn render_frame<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &CodexApp,
) -> std::io::Result<()> {
    terminal.draw(|frame| render_into(frame, app))?;
    Ok(())
}

/// Draw one frame into the ratatui buffer.
fn render_into(frame: &mut Frame<'_>, app: &CodexApp) {
    let area = frame.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = format!("视口过小——至少需要 {MIN_WIDTH}x{MIN_HEIGHT}");
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(sanitize_text(&msg)).alignment(Alignment::Center),
            area,
        );
        return;
    }

    let rows = Layout::vertical([
        Constraint::Min(3),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);

    render_messages(frame, rows[0], app);
    render_input(frame, rows[1], app);
    render_status(frame, rows[2], app);

    if let Some(dialog) = &app.dialog {
        render_approval_dialog(frame, area, dialog);
    }
}

/// The message list: user messages right-aligned, agent left, system
/// centered. Streaming agent messages carry a trailing block cursor.
fn render_messages(frame: &mut Frame<'_>, area: Rect, app: &CodexApp) {
    let inner = Block::default().borders(Borders::TOP).title(" 对话 ");
    let content_area = inner.inner(area);
    frame.render_widget(inner, area);

    let width = content_area.width.saturating_sub(1) as usize;
    let mut lines: Vec<Line> = app
        .messages
        .iter()
        .map(|m| message_line(m, width))
        .collect();
    // Tail-follow: show the newest messages that fit; no scrollback in v1
    // (recorded boundary).
    let capacity = content_area.height as usize;
    if lines.len() > capacity {
        lines.drain(0..lines.len() - capacity);
    }
    let paragraph = Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: false });
    frame.render_widget(paragraph, content_area);
}

fn message_line(msg: &crate::app::MessageEntry, width: usize) -> Line<'static> {
    let text = truncate_to_width(&sanitize_text(&msg.text), width as u16, "…");
    match msg.role {
        Role::User => Line::from(vec![
            Span::styled("你: ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(text),
        ])
        .alignment(Alignment::Right),
        Role::Agent => {
            let mut spans = vec![Span::styled(
                if msg.streaming { "▌ " } else { "  " },
                Style::default().fg(Color::Green),
            )];
            let mut t = text;
            if msg.streaming {
                // Streaming marker: a trailing block while deltas are live.
                t = truncate_to_width(&t, width.saturating_sub(1) as u16, "…");
                spans.push(Span::raw(t));
                spans.push(Span::styled("▍", Style::default().fg(Color::Green)));
            } else {
                spans.push(Span::raw(t));
            }
            Line::from(spans).alignment(Alignment::Left)
        }
        Role::System => Line::from(vec![Span::styled(
            text,
            Style::default().fg(Color::Yellow),
        )])
        .alignment(Alignment::Center),
    }
}

fn render_input(frame: &mut Frame<'_>, area: Rect, app: &CodexApp) {
    let title = match &app.turn_id {
        Some(turn) => format!(" 输入 — 当前回合 {turn} "),
        None => " 输入 ".to_string(),
    };
    let inner = Block::default().borders(Borders::ALL).title(title);
    let content = inner.inner(area);
    frame.render_widget(inner, area);
    let text = truncate_to_width(&sanitize_text(app.input.text()), content.width, "…");
    let paragraph = Paragraph::new(text).style(if app.status == RunState::Running {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default()
    });
    frame.render_widget(paragraph, content);
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &CodexApp) {
    let mut spans = vec![
        Span::styled(
            "状态: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(app.status.label(), status_style(app.status)),
    ];
    if let Some(turn) = &app.turn_id {
        spans.push(Span::raw(format!(" [回合 {turn}]")));
    }
    if let Some(detail) = &app.status_detail {
        spans.push(Span::styled(
            format!(" — {detail}"),
            Style::default().fg(Color::Red),
        ));
    }
    spans.push(Span::raw("    Enter 发送 · Ctrl+Z 中断 · Ctrl+C 退出"));
    let line = Line::from(spans);
    let mut text = Vec::new();
    for span in line.spans {
        text.push(span.content.to_string());
    }
    let joined = text.join("");
    let joined = truncate_to_width(&sanitize_text(&joined), area.width, "…");
    frame.render_widget(Paragraph::new(joined), area);
}

fn status_style(status: RunState) -> Style {
    match status {
        RunState::Running => Style::default().fg(Color::Green),
        RunState::WaitingApproval => Style::default().fg(Color::Yellow),
        RunState::Interrupted => Style::default().fg(Color::Blue),
        RunState::Failed => Style::default().fg(Color::Red),
        RunState::Completed => Style::default().fg(Color::Green),
        RunState::Idle => Style::default(),
    }
}

/// Centered approval dialog — the fallback's only overlay (the permission
/// surface; the host's assurance layers stay invisible).
fn render_approval_dialog(frame: &mut Frame<'_>, area: Rect, dialog: &ApprovalDialog) {
    let width = area.width.min(64).saturating_sub(4);
    let params = &dialog.params;

    let mut body = vec![Line::from(format!(
        "工具: {}",
        params.get("tool_name").and_then(|v| v.as_str()).unwrap_or("?")
    ))];
    let description = params
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !description.is_empty() {
        body.push(Line::from(format!("描述: {description}")));
    }
    if let Some(cmd) = params.get("bash_command").and_then(|v| v.as_str()) {
        body.push(Line::from(format!("命令: {cmd}")));
    }
    if let Some(paths) = params.get("edit_file_paths").and_then(|v| v.as_array()) {
        let joined = paths
            .iter()
            .filter_map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        body.push(Line::from(format!("文件: {joined}")));
    }

    // Option row with the selection highlighted.
    let mut option_spans: Vec<Span> = Vec::new();
    for (i, option) in ApprovalDialog::OPTIONS.iter().enumerate() {
        if i > 0 {
            option_spans.push(Span::raw("  "));
        }
        let style = if i == dialog.selected {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        option_spans.push(Span::styled(format!("[{option}]"), style));
    }
    body.push(Line::from(""));
    body.push(Line::from(option_spans));
    body.push(Line::from(format!(
        "（{remaining} 秒后自动拒绝）  Esc 拒绝 · ←→ 选择 · Enter 确认",
        remaining = dialog.remaining().as_secs()
    )));

    let height = (body.len() as u16 + 2).min(area.height.saturating_sub(2));
    let overlay = Rect {
        x: (area.width - width).saturating_div(2),
        y: (area.height - height).saturating_div(2),
        width,
        height,
    };
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(" 工具权限请求 ")
            .border_style(Style::default().fg(Color::Yellow)),
        overlay,
    );
    let inner = Rect {
        x: overlay.x + 1,
        y: overlay.y + 1,
        width: overlay.width.saturating_sub(2),
        height: overlay.height.saturating_sub(2),
    };
    frame.render_widget(Paragraph::new(body).wrap(ratatui::widgets::Wrap { trim: false }), inner);
}

#[cfg(test)]
mod tests {
    use crate::app::CodexApp;
    use crate::snapshot::render_to_string;

    /// The assurance-workbench strings that must never appear in the
    /// fallback TUI (design §2.4 — no assurance panels).
    const ASSURANCE_SURFACE: [&str; 6] = ["文件", "事件", "标记", "来源", "验证", "帮助"];

    fn assert_no_assurance_panels(rendered: &str) {
        for s in ASSURANCE_SURFACE {
            assert!(
                !rendered.contains(s),
                "fallback TUI must not surface the assurance panel string {s:?}:\n{rendered}"
            );
        }
    }

    #[test]
    fn idle_frame_has_no_assurance_panels() {
        let mut app = CodexApp::new();
        let rendered = render_to_string(&mut app, 100, 30);
        assert!(rendered.contains("状态: 空闲"), "{rendered}");
        assert!(rendered.contains("Enter 发送"), "{rendered}");
        assert_no_assurance_panels(&rendered);
    }

    #[test]
    fn messages_render_with_roles_and_streaming_marker() {
        let mut app = CodexApp::new();
        app.submit_prompt("你好".to_owned());
        app.on_item_delta("turn_1".into(), "item_1".into(), "正在".into());
        app.on_item_delta("turn_1".into(), "item_1".into(), "回答".into());
        let rendered = render_to_string(&mut app, 100, 30);
        assert!(rendered.contains("你好"), "{rendered}");
        assert!(rendered.contains("正在回答"), "{rendered}");
        assert!(rendered.contains("▍"), "streaming marker present: {rendered}");
        assert_no_assurance_panels(&rendered);
    }

    #[test]
    fn completed_turn_shows_status_and_agent_message() {
        let mut app = CodexApp::new();
        app.submit_prompt("你好".to_owned());
        app.on_item_completed("turn_1".into(), "item_1".into(), "终局答案。".into(), "agentMessage".into());
        app.on_turn_completed("turn_1".into(), "completed".into(), None);
        let rendered = render_to_string(&mut app, 100, 30);
        assert!(rendered.contains("终局答案。"), "{rendered}");
        assert!(rendered.contains("状态: 完成"), "{rendered}");
        assert_no_assurance_panels(&rendered);
    }

    #[test]
    fn approval_dialog_overlay_renders_with_countdown() {
        let mut app = CodexApp::new();
        app.on_approval_request(
            serde_json::json!(1),
            serde_json::json!({
                "tool_name": "run_terminal_command",
                "description": "Run a terminal command",
                "bash_command": "dir",
            }),
        );
        let rendered = render_to_string(&mut app, 100, 30);
        assert!(rendered.contains("工具权限请求"), "{rendered}");
        assert!(rendered.contains("run_terminal_command"), "{rendered}");
        assert!(rendered.contains("dir"), "{rendered}");
        assert!(rendered.contains("[允许一次]"), "{rendered}");
        assert!(rendered.contains("自动拒绝"), "{rendered}");
        assert!(rendered.contains("状态: 等待审批"), "{rendered}");
        assert_no_assurance_panels(&rendered);
    }

    #[test]
    fn tiny_viewport_shows_notice() {
        let mut app = CodexApp::new();
        let rendered = render_to_string(&mut app, 60, 15);
        assert!(rendered.contains("视口过小"), "{rendered}");
    }

    #[test]
    fn user_messages_are_right_aligned_agent_left() {
        let mut app = CodexApp::new();
        app.submit_prompt("右对齐的我".to_owned());
        app.on_item_completed("turn_1".into(), "item_1".into(), "左对齐的它".into(), "agentMessage".into());
        let rendered = render_to_string(&mut app, 100, 30);
        // The user line is padded to the right edge (indent of the last
        // line's prefix + text near the right side).
        let lines: Vec<&str> = rendered.lines().collect();
        let user_line = lines.iter().find(|l| l.contains("右对齐的我")).expect("user message");
        assert!(
            user_line.trim_end().ends_with('\u{6211}'),
            "user line flush right: {user_line:?}"
        );
        let agent_line = lines.iter().find(|l| l.contains("左对齐的它")).expect("agent message");
        // The agent line carries the 2-space prefix and hugs the left edge
        // (no right padding — the user line carries it instead).
        assert!(
            agent_line.trim_start().starts_with("左对齐的它"),
            "agent line flush left: {agent_line:?}"
        );
    }
}
