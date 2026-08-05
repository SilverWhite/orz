//! Rendering — compose the full retro frame as text lines (Python
//! `widgets.py` render pipeline) and blit into the ratatui buffer.
//!
//! `compose_screen` is a pure function over view-model state, so snapshot
//! tests can pin exact output without a terminal.

use ratatui::backend::Backend;
use ratatui::Terminal;

use crate::app::{Focus, TuiApp};
use crate::theme::{pad_right, str_width, truncate_to_width, EXPLORER_WIDTH, MARKER_WIDTH};
use crate::view_model::{ChatMessage, ContentItem, ContentPane, ToolTraceLine};

/// Compose the full screen as display lines (Python `render(width, height)`).
pub fn compose_screen(app: &TuiApp, width: u16, height: u16) -> Vec<String> {
    if TuiApp::viewport_too_small(width, height) {
        return small_viewport_lines(width, height);
    }
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let w = width as usize;
    let h = height as usize;

    let mut lines: Vec<String> = Vec::with_capacity(h);
    // Row 0 — menu bar.
    lines.push(pad_right(&app.menu_bar.items.join(" "), w as u16));
    // Row 1 — toolbar (right-aligned 命令.../查找... group with a gap).
    lines.push(render_toolbar(&app, w));
    // Body. Each visible side column shares its junction column with the
    // content pane's adjacent border, so the content pane gains one column
    // per visible side column (region width == w by construction).
    let explorer_w = if app.show_explorer { EXPLORER_WIDTH as usize } else { 0 };
    let marker_w = if app.show_marker { MARKER_WIDTH as usize } else { 0 };
    let content_w = w.saturating_sub(explorer_w + marker_w)
        + usize::from(explorer_w > 0)
        + usize::from(marker_w > 0);
    let body_h = h.saturating_sub(4); // menu + toolbar + input + status
    let body = render_body(app, explorer_w, content_w, marker_w, body_h);
    lines.extend(body);
    // Input row.
    lines.push(render_input_line(app, w));
    // Status bar.
    lines.push(render_status_bar(app, w));
    lines.truncate(h);

    // Tabbed-modal overlay (help / find / properties — slice #9), blended
    // before the permission dialog; the two are mutually exclusive by
    // construction (app.rs), so this order is only defensive.
    if let Some(modal) = &app.modal
        && let Some((m_x, m_y, m_w, m_h)) = modal_area(width, height)
    {
        let overlay = compose_modal_overlay(modal, m_w, m_h);
        blend_overlay(&mut lines, overlay, m_x, m_y, m_w);
    }

    // Dialog overlay — replace the base frame's lines inside the centered
    // box (base content stays visible around it: centered modal retaining
    // context, SESSION_PERSISTENCE doc).
    if let Some(dialog) = &app.dialog {
        if let Some((d_x, d_y, d_w, _d_h)) = dialog_area(width, height) {
            // Permission-dialog countdown (Phase 3 slice #7): derived at
            // render time from the pending request's open instant — no state
            // mutation on the 50ms tick.
            let countdown = app.pending_permission.as_ref().and_then(|pp| {
                pp.remaining().map(|r| format!("剩余 {}s", r.as_secs()))
            });
            let overlay = compose_dialog_overlay(dialog, d_w, countdown);
            blend_overlay(&mut lines, overlay, d_x, d_y, d_w);
        }
    }
    lines
}

/// Blend an overlay's lines into the base frame at (x, y) — the shared
/// character-replacement path for dialogs and modals.
fn blend_overlay(
    lines: &mut [String],
    overlay: Vec<String>,
    x: u16,
    y: u16,
    w: u16,
) {
    for (i, oline) in overlay.iter().enumerate() {
        let row = y as usize + i;
        if row >= lines.len() {
            break;
        }
        let base: Vec<char> = lines[row].chars().collect();
        let mut out: Vec<char> = base;
        for (j, c) in oline.chars().take(w as usize).enumerate() {
            let col = x as usize + j;
            if col >= out.len() {
                out.resize(col + 1, ' ');
            }
            out[col] = c;
        }
        lines[row] = out.into_iter().collect();
    }
}

/// Blit the composed screen into the frame buffer with minimal styling.
pub fn render_frame<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut TuiApp,
) -> std::io::Result<()> {
    terminal.draw(|frame| {
        let size = frame.area();
        let lines = compose_screen(app, size.width, size.height);
        let buf = frame.buffer_mut();
        for (y, line) in lines.iter().enumerate() {
            if y >= size.height as usize {
                break;
            }
            buf.set_string(0, y as u16, line, ratatui::style::Style::default());
        }
        // Chat input cursor (only when input owns focus).
        if app.focus == Focus::Chat && size.height >= 2 {
            let input_y = size.height.saturating_sub(2);
            let area = ratatui::layout::Rect::new(0, input_y, size.width, 1);
            if let Some((x, _y)) = app.input.textarea.cursor_pos(area) {
                let x = x.saturating_add(2).min(size.width.saturating_sub(1));
                frame.set_cursor_position((x, input_y));
            }
        }
    })?;
    Ok(())
}

// ── area renderers ──────────────────────────────────────────────────────────

fn render_toolbar(_app: &TuiApp, w: usize) -> String {
    let left = ["后退", "前进", "刷新", "停止", "打开", "验证", "属性"].join(" ");
    let right = ["命令...", "查找..."].join(" ");
    // Grouping gap before the right-aligned group (SESSION_PERSISTENCE doc).
    let left_w = str_width(&left) as usize;
    let right_w = str_width(&right) as usize;
    let gap = w.saturating_sub(left_w + right_w + 4).max(4);
    pad_right(
        &format!("{left}{}{right}", " ".repeat(gap as usize)),
        w as u16,
    )
}

fn render_body(
    app: &TuiApp,
    explorer_w: usize,
    content_w: usize,
    marker_w: usize,
    body_h: usize,
) -> Vec<String> {
    if body_h == 0 {
        return Vec::new();
    }
    let mut lines: Vec<String> = Vec::with_capacity(body_h);
    let explorer_inner = explorer_w.saturating_sub(2).max(1);
    let content_inner = content_w.saturating_sub(2).max(1);
    let marker_inner = marker_w.saturating_sub(2).max(1);

    // Each visible side column shares its junction column with the content
    // pane's adjacent border (frozen layout `├───┬───┬───┤`), so the region
    // width is explorer_w + content_w + marker_w - 2 == full width by
    // construction (compose_screen adds one column per visible side column).
    let mut segments: Vec<String> = Vec::new();
    if explorer_w > 0 {
        segments.push("─".repeat(explorer_inner));
    }
    segments.push("─".repeat(content_inner));
    if marker_w > 0 {
        segments.push("─".repeat(marker_inner));
    }
    let top = format!("├{}┤", segments.join("┬"));
    lines.push(top);

    // Content pane rows (Python box_vertical: side borders, bottom-anchored);
    // explorer pane is top-anchored (tree from the top, like the marker).
    let inner_h = body_h.saturating_sub(2); // minus top + bottom borders
    let item_lines = render_content_pane(&app.content, content_inner);
    let marker_lines = render_marker_pane(app, marker_inner);
    let explorer_lines = render_explorer_pane(app, explorer_inner, inner_h);
    let start = item_lines.len().saturating_sub(inner_h);
    let visible: Vec<String> = item_lines
        .iter()
        .skip(start)
        .take(inner_h)
        .cloned()
        .collect();
    for i in 0..inner_h {
        let mut row = String::from("│");
        if explorer_w > 0 {
            let explorer_row = explorer_lines.get(i).cloned().unwrap_or_default();
            row.push_str(&pad_right(&explorer_row, explorer_inner as u16));
            row.push('│');
        }
        let content_row = visible.get(i).cloned().unwrap_or_default();
        row.push_str(&pad_right(&content_row, content_inner as u16));
        row.push('│');
        if marker_w > 0 {
            let marker_row = marker_lines.get(i).cloned().unwrap_or_default();
            row.push_str(&pad_right(&marker_row, marker_inner as u16));
            row.push('│');
        }
        lines.push(row);
    }

    // Bottom border.
    let bottom = format!("└{}┘", segments.join("┴"));
    lines.push(bottom);
    lines.truncate(body_h);
    lines
}

/// Render the conversation stream (message cards + tool traces).
fn render_content_pane(pane: &ContentPane, inner_w: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if pane.items.is_empty() {
        lines.push(pad_right(&pane.empty_hint, inner_w as u16));
        return lines;
    }
    let inner_w = inner_w.max(4) as u16;
    for item in &pane.items {
        match item {
            ContentItem::Message(msg) => lines.extend(render_chat_message(msg, inner_w)),
            ContentItem::ToolTrace(trace) => {
                if trace.expanded {
                    lines.extend(render_expanded_tool(trace, inner_w));
                } else {
                    lines.push(render_tool_collapsed(trace, inner_w));
                }
            }
        }
    }
    lines
}

/// Bordered message card (CONTENT_PANE spec: `┌─┐│└┘`, title
/// `┌ [角色] · … ─┐`). Every row is exactly *width* display columns, so the
/// card is rectangular with all four corners (Python parity: `inner =
/// width - 2`, rows padded to `width`).
fn render_chat_message(msg: &ChatMessage, width: u16) -> Vec<String> {
    let inner = (width as usize).saturating_sub(2);
    let mut lines: Vec<String> = Vec::new();

    if msg.collapsed && !msg.warning {
        let mut preview: String = msg.content.chars().take(60).collect();
        preview = preview.replace('\n', " ");
        if msg.content.chars().count() > 60 {
            preview.push_str("...");
        }
        let mut title = format!(" [{}]", msg.role);
        if msg.turn > 0 {
            title.push_str(&format!(" · turn {}", msg.turn));
        }
        lines.push(format!("  {title}  ▸ {preview}"));
        return lines;
    }

    // Title row — `┌ title ───┐` exactly `width` columns wide.
    let mut title = format!(" [{}]", msg.role);
    if msg.turn > 0 {
        title.push_str(&format!(" · turn {}", msg.turn));
    }
    let fill_w = inner.saturating_sub(str_width(&title) as usize + 2).max(0);
    let title_row = format!("┌{title} {}{}", "─".repeat(fill_w), "┐");
    lines.push(pad_right(&title_row, width));

    // Content rows — plain-text markdown subset.
    let formatted = format_chat_content(&msg.content, inner.saturating_sub(2));
    for fl in formatted {
        if fl.is_empty() {
            lines.push(format!("│{}│", " ".repeat(inner)));
        } else {
            for wl in wrap_line(&fl, inner.saturating_sub(2)) {
                lines.push(format!("│ {} │", pad_right(&wl, inner as u16 - 2)));
            }
        }
    }

    // Bottom border.
    lines.push(format!("└{}┘", "─".repeat(inner)));
    lines
}

/// Plain-text markdown subset (headings, lists, bold strip, code fences).
fn format_chat_content(content: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut in_code_block = false;
    let mut code_lines: Vec<String> = Vec::new();
    let mut code_lang = String::new();

    for raw in content.split('\n') {
        let line = raw.trim_end().to_string();

        if line.starts_with("```") {
            if in_code_block {
                lines.extend(render_code_block(&code_lines, &code_lang, width));
                code_lines.clear();
                code_lang.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
                code_lang = line[3..].trim().to_string();
            }
            continue;
        }
        if in_code_block {
            code_lines.push(line);
            continue;
        }

        // Headings.
        if let Some(stripped) = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))
            .or_else(|| line.strip_prefix("# "))
        {
            let text = stripped.trim().to_string();
            lines.push(text.clone());
            let n = (str_width(&text) as usize).min(width);
            lines.push("─".repeat(n));
            continue;
        }
        // Unordered lists.
        if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            lines.push(format!("  • {}", strip_bold(text.trim())));
            continue;
        }
        // Ordered lists.
        if line.len() > 2 {
            if let Some(dot) = line.find(". ") {
                let (num, rest) = line.split_at(dot);
                if !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()) {
                    lines.push(format!("  {num}. {}", strip_bold(rest[2..].trim())));
                    continue;
                }
            }
        }
        // Regular paragraph.
        lines.push(strip_bold(&line));
    }
    if !code_lines.is_empty() {
        lines.extend(render_code_block(&code_lines, &code_lang, width));
    }
    lines
}

fn strip_bold(text: &str) -> String {
    text.replace("**", "")
}

fn render_code_block(code_lines: &[String], lang: &str, width: usize) -> Vec<String> {
    if code_lines.is_empty() {
        return Vec::new();
    }
    let mut result: Vec<String> = Vec::new();
    let n = code_lines.len();
    let num_w = n.to_string().len();
    if !lang.is_empty() {
        result.push(format!("  [{lang}]"));
    }
    for (i, cl) in code_lines.iter().enumerate() {
        let prefix = format!("  {:>num_w$}  ", i + 1);
        let available = width.saturating_sub(str_width(&prefix) as usize);
        let raw: String = cl.chars().take(available).collect();
        result.push(format!("{prefix}{raw}"));
    }
    result
}

/// Collapsed tool trace row: `  [name]   target · detail   ▸ [展开]`.
fn render_tool_collapsed(trace: &ToolTraceLine, width: u16) -> String {
    let btn = if trace.expanded { "[关闭]" } else { "[展开]" };
    let detail = match trace.entries.last() {
        Some(e) => format!("{} · {}", e.target, e.detail),
        None => String::new(),
    };
    let left = format!("  [{}]   {}", trace.tool_name, detail);
    let right = format!("▸ {btn}");
    let left_w = str_width(&left) as usize;
    let right_w = str_width(&right) as usize;
    let gap = (width as usize).saturating_sub(left_w + right_w).max(1);
    format!("{left}{}{right}", " ".repeat(gap))
}

/// Expanded tool trace (pin-to-top list, no border).
fn render_expanded_tool(trace: &ToolTraceLine, width: u16) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let header = format!("  [{}] ▸ [关闭]", trace.tool_name);
    lines.push(truncate_to_width(&header, width, ""));
    for (i, entry) in trace.entries.iter().enumerate() {
        let line = format!("    {:2}  {} · {}", i + 1, entry.target, entry.detail);
        lines.push(truncate_to_width(&line, width, ""));
    }
    lines
}

fn render_marker_pane(app: &TuiApp, inner_w: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(pad_right("Markers", inner_w as u16));
    lines.push("────".to_string());
    if app.marker.is_empty() {
        lines.push(pad_right("(none)", inner_w as u16));
    } else {
        for m in &app.marker.markers {
            lines.push(pad_right(&m.label, inner_w as u16));
        }
    }
    lines
}

/// Explorer pane (slice #9): the session list (double-Esc mode), the cwd
/// tree (F6 focus), or the live event counts — top-anchored like the
/// marker pane. `pane_h` is the body's inner height (tree scroll window).
fn render_explorer_pane(app: &TuiApp, inner_w: usize, pane_h: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if app.explorer.session_mode {
        lines.extend(render_session_list(app, inner_w));
    } else {
        lines.extend(render_tree(app, inner_w, pane_h));
        if app.explorer.show_events {
            lines.push("────".to_string());
            lines.extend(render_event_groups(app, inner_w));
        }
    }
    lines
}

/// Preorder tree rows with ▾/▸ markers and the ▸ selection cursor.
fn render_tree(app: &TuiApp, inner_w: usize, pane_h: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(pad_right("探索器", inner_w as u16));
    lines.push("────".to_string());
    if !app.explorer.loaded {
        lines.push(pad_right("(未加载)", inner_w as u16));
        return lines;
    }
    if app.explorer.tree.is_empty() {
        lines.push(pad_right("(空)", inner_w as u16));
        return lines;
    }
    let visible: Vec<(usize, &crate::explorer::TreeNode)> =
        crate::explorer::flatten(&app.explorer.tree, 0);
    let height = pane_h.saturating_sub(2).max(1);
    let window_top = app.explorer.visible_window(height);
    for (i, (depth, node)) in visible.iter().enumerate().skip(window_top).take(height) {
        let cursor = if i == app.explorer.selected { "▸ " } else { "  " };
        let marker = if !node.children.is_empty() {
            if node.expanded { "▾ " } else { "▸ " }
        } else {
            "  "
        };
        let indent = "  ".repeat(*depth);
        let row = format!("{cursor}{indent}{marker}{}", node.label);
        lines.push(truncate_to_width(&row, inner_w as u16, "…"));
    }
    lines
}

/// Run-history list (double-Esc mode): date groups + one row per run
/// journal. Labeled 运行历史, not 会话列表 — the Rust world has no
/// session.json, so each row is one run, not one session (review D2-1
/// honest labeling; a per-session grouping lands with the marker contract).
fn render_session_list(app: &TuiApp, inner_w: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(pad_right("▾ 运行历史", inner_w as u16));
    lines.push("────".to_string());
    if app.explorer.sessions.is_empty() {
        lines.push(pad_right("  (无历史会话)", inner_w as u16));
        return lines;
    }
    let mut current_date: Option<String> = None;
    for (i, s) in app.explorer.sessions.iter().enumerate() {
        if current_date.as_deref() != Some(s.date.as_str()) {
            current_date = Some(s.date.clone());
            let count = app
                .explorer
                .sessions
                .iter()
                .filter(|x| x.date == s.date)
                .count();
            lines.push(truncate_to_width(
                &format!("▾ {} ({})", s.date, count),
                inner_w as u16,
                "…",
            ));
        }
        let cursor = if i == app.explorer.session_selected { "▸ " } else { "  " };
        let sid8: String = s
            .session_id
            .strip_prefix("RUN-")
            .and_then(|r| r.split('-').next())
            .map(|s| s.chars().take(8).collect())
            .unwrap_or_else(|| s.session_id.clone());
        let preview = crate::theme::truncate_to_width(&s.prompt_preview, 12, "…");
        let row = format!("{cursor}[{sid8}] {}t  {preview}", s.turn_count);
        lines.push(truncate_to_width(&row, inner_w as u16, "…"));
    }
    lines
}

/// Live event-group counts (Python `_render_events` shape; v1 = counts).
fn render_event_groups(app: &TuiApp, inner_w: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    lines.push(pad_right("事件", inner_w as u16));
    if app.explorer.events.is_empty() {
        lines.push(pad_right("  (无)", inner_w as u16));
        return lines;
    }
    for (kind, count) in &app.explorer.events {
        lines.push(truncate_to_width(&format!("  {kind} ({count})"), inner_w as u16, "…"));
    }
    lines
}

fn render_input_line(app: &TuiApp, w: usize) -> String {
    let text = app.input.textarea.text();
    let line = format!("> {text}");
    pad_right(&line, w as u16)
}

fn render_status_bar(app: &TuiApp, w: usize) -> String {
    let joined = app
        .status
        .items
        .iter()
        .map(|i| i.label.as_str())
        .collect::<Vec<_>>()
        .join(" | ");
    pad_right(&joined, w as u16)
}

fn small_viewport_lines(width: u16, height: u16) -> Vec<String> {
    let msg = format!(
        "视口过小（需要 ≥{}×{}，当前 {}×{}）",
        crate::app::ABSOLUTE_MIN_WIDTH,
        crate::app::ABSOLUTE_MIN_HEIGHT,
        width,
        height
    );
    let mut lines = vec![String::new(); (height as usize).saturating_sub(1).max(1)];
    lines.push(msg);
    lines
}

/// Centered dialog area rect (overlay).
fn dialog_area(width: u16, height: u16) -> Option<(u16, u16, u16, u16)> {
    if width < 40 || height < 10 {
        return None;
    }
    let d_w = width.min(64).saturating_sub(4).max(40);
    let d_h = height.min(14).saturating_sub(4).max(8);
    let d_x = (width - d_w) / 2;
    let d_y = (height - d_h) / 2;
    Some((d_x, d_y, d_w, d_h))
}

/// Compose the dialog overlay lines (called by compose_screen after the
/// base frame — the renderer clears the dialog area first). Every row is
/// exactly *box_width* display columns, so the box is rectangular
/// (review P3-6c: rows used to differ by 2 columns).
///
/// `countdown` renders an extra row between the message and the actions
/// (the permission dialog's 300s timeout countdown — Phase 3 slice #7).
pub fn compose_dialog_overlay(
    dialog: &crate::dialogs::Dialog,
    box_width: u16,
    countdown: Option<String>,
) -> Vec<String> {
    let w = (box_width as usize).max(24);
    let inner = w.saturating_sub(2);
    let mut lines: Vec<String> = Vec::new();
    let title = format!(" {} ", dialog.title);
    // Title: ┌ + title + ─fill + ┐, padded to box_width — rectangular.
    let fill = inner.saturating_sub(str_width(&title) as usize + 2);
    lines.push(pad_right(&format!("┌{title}{}┐", "─".repeat(fill)), box_width));
    for mline in &dialog.message {
        lines.push(format!("│ {} │", pad_right(mline, inner as u16 - 2)));
    }
    if let Some(cd) = countdown {
        lines.push(format!("│ {} │", pad_right(&cd, inner as u16 - 2)));
    }
    // Actions row — selected action styled by the renderer; text keeps
    // bracket markers for snapshot tests.
    let actions: Vec<String> = dialog
        .actions
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if i == dialog.selected {
                format!("<{}>", a.label())
            } else {
                format!("[{}]", a.label())
            }
        })
        .collect();
    let actions_row = format!("│ {} │", pad_right(&actions.join(" "), inner as u16 - 2));
    lines.push(actions_row);
    lines.push(format!("└{}┘", "─".repeat(inner)));
    lines
}

/// Centered tabbed-modal area (slice #9). Width clamps to 36..68 per the
/// design (HelpOverlay width 36-68); height is content-adaptive but never
/// exceeds the viewport.
fn modal_area(width: u16, height: u16) -> Option<(u16, u16, u16, u16)> {
    if width < 40 || height < 12 {
        return None;
    }
    let m_w = (width.saturating_sub(8)).clamp(36, 68);
    // Height computed by the overlay builder; the area only positions it.
    let m_h = height.saturating_sub(4).min(30);
    let m_x = (width - m_w) / 2;
    let m_y = (height - m_h) / 2;
    Some((m_x, m_y, m_w, m_h))
}

/// Compose the tabbed-modal overlay lines (help / find / properties).
pub fn compose_modal_overlay(
    modal: &crate::modals::Modal,
    box_width: u16,
    max_height: u16,
) -> Vec<String> {
    match modal {
        crate::modals::Modal::Help(h) => {
            compose_tabbed_overlay(&h.sheet, box_width, max_height)
        }
        crate::modals::Modal::Properties(p) => {
            compose_tabbed_overlay(&p.sheet, box_width, max_height)
        }
        crate::modals::Modal::Find(f) => compose_find_overlay(f, box_width, max_height),
        crate::modals::Modal::Snapshots(s) => {
            compose_snapshots_overlay(s, box_width, max_height)
        }
    }
}

/// Snapshot selector overlay (slice #10) — title / header / entry rows /
/// footer, every row exactly *box_width* columns (D3-1 corner discipline).
/// The selected row enters a confirm state after the first Enter.
/// Height-adaptive like the tabbed sheet: drop content rows, keep the
/// footer + borders; bottom-anchored jump scroll (ExplorerPane pattern).
fn compose_snapshots_overlay(
    sel: &crate::modals::SnapshotSelector,
    box_width: u16,
    max_height: u16,
) -> Vec<String> {
    let w = (box_width as usize).max(24);
    let inner = w.saturating_sub(2);
    let mut lines: Vec<String> = Vec::new();
    let title = " 快照 ";
    // Corner-flush title row (review D3-1 — same fix as the tabbed sheet).
    let fill = inner.saturating_sub(str_width(title) as usize);
    lines.push(pad_right(&format!("┌{title}{}┐", "─".repeat(fill)), box_width));

    // Column header.
    lines.push(format!(
        "│ {} │",
        pad_right("运行 工具 哈希 文件 日期", inner as u16 - 2)
    ));

    if sel.entries.is_empty() {
        lines.push(format!(
            "│ {} │",
            pad_right("  (无快照 — 尚无变更前快照)", inner as u16 - 2)
        ));
    } else {
        // Content budget: title + header + footer + bottom border = 4.
        let budget = (max_height as usize).saturating_sub(4);
        let top = sel.window_top(budget);
        for (i, e) in sel.entries.iter().enumerate().skip(top).take(budget) {
            let cursor = if i == sel.selected { "▸ " } else { "  " };
            let row = if sel.confirm && i == sel.selected {
                let short: String = e.hash.chars().take(8).collect();
                // Review D2-6: restoring overwrites the current worktree
                // state with no undo — the confirm row says so explicitly.
                format!("{cursor}确认恢复？ {short} 覆盖当前文件（无撤销）— Enter 确认 / Esc 返回")
            } else {
                let sid8: String = e
                    .run_id
                    .strip_prefix("RUN-")
                    .and_then(|r| r.split('-').next())
                    .map(|s| s.chars().take(8).collect())
                    .unwrap_or_default();
                let short: String = e.hash.chars().take(8).collect();
                let files = match e.file_count {
                    Some(n) => format!("{n} 文件"),
                    None => "—".to_string(),
                };
                format!("{cursor}[{sid8}] {} {short} {files} {}", e.tool, e.date)
            };
            let fitted = truncate_to_width(&row, inner as u16 - 2, "…");
            lines.push(format!("│ {} │", pad_right(&fitted, inner as u16 - 2)));
        }
    }

    // Review D2-2: the list is scanned once at open — the footer says so
    // (a run finishing mid-modal leaves the list without its snapshot).
    lines.push(format!(
        "│ {} │",
        pad_right(
            "Esc 关闭  ↑↓ 选择  Enter 确认恢复 · 列表为打开时快照",
            inner as u16 - 2
        )
    ));
    lines.push(format!("└{}┘", "─".repeat(inner)));

    // Defensive height clamp (keeps the footer + bottom border). Reserve a
    // row for the border after truncating (review P3-5: the naive truncate
    // then push yields max_height + 1 rows — unreachable today because
    // modal_area floors the box at height 12, but a trap for future callers).
    if lines.len() > max_height as usize {
        lines.truncate(max_height.saturating_sub(1) as usize);
        lines.push(format!("└{}┘", "─".repeat(inner)));
    }
    lines
}

/// Tabbed sheet: title / tab row (`▶名`) / separator / fields or content /
/// footer / bottom border — every row exactly *box_width* display columns.
pub fn compose_tabbed_overlay(
    sheet: &crate::modals::TabbedSheet,
    box_width: u16,
    max_height: u16,
) -> Vec<String> {
    let w = (box_width as usize).max(24);
    let inner = w.saturating_sub(2);
    let mut lines: Vec<String> = Vec::new();
    let title = format!(" {} ", sheet.title);
    // `┌ {title} ───┐` spans exactly box_width: fill = inner - title width
    // (review D3-1: the corner must sit flush at w-1, not w-3).
    let fill = inner.saturating_sub(str_width(&title) as usize);
    lines.push(pad_right(&format!("┌{title}{}┐", "─".repeat(fill)), box_width));

    // Tab row — active tab carries the ▶ marker (Python PropertiesSheet).
    let mut tab_content = String::new();
    for (i, tab) in sheet.tabs.iter().enumerate() {
        if i > 0 {
            tab_content.push(' ');
        }
        if i == sheet.active {
            tab_content.push_str(&format!("▶{}", tab.name));
        } else {
            tab_content.push_str(&tab.name);
        }
    }
    lines.push(format!(
        "│ {} │",
        pad_right(&tab_content, inner as u16 - 2)
    ));

    // Separator under the tabs.
    lines.push(format!("├{}┤", "─".repeat(inner)));

    // Fields / content rows.
    if let Some(tab) = sheet.active_tab() {
        let rows: Vec<String> = if let Some(content) = &tab.content {
            content.lines().map(|l| l.to_string()).collect()
        } else {
            tab.fields
                .iter()
                .map(|(k, v)| format!("  {k}: {v}"))
                .collect()
        };
        for row in rows {
            let fitted = truncate_to_width(&row, inner as u16 - 2, "…");
            lines.push(format!("│ {} │", pad_right(&fitted, inner as u16 - 2)));
        }
    }

    // Footer hint.
    lines.push(format!(
        "│ {} │",
        pad_right("Esc 关闭  ←→ 切换标签", inner as u16 - 2)
    ));
    lines.push(format!("└{}┘", "─".repeat(inner)));

    // Height-adaptive: drop content rows from the bottom if the box would
    // exceed the viewport, keeping the footer + borders.
    if lines.len() > max_height as usize {
        let overflow = lines.len() - max_height as usize;
        // Remove the oldest content rows (after the separator, before the
        // footer — index 3 + overflow).
        if lines.len() > 4 + overflow {
            lines.drain(3..3 + overflow);
        }
    }
    lines
}

/// Find overlay — 5 focus rows: query / scope / case+regex / actions.
fn compose_find_overlay(
    find: &crate::modals::FindDialog,
    box_width: u16,
    max_height: u16,
) -> Vec<String> {
    let w = (box_width as usize).max(24);
    let inner = w.saturating_sub(2);
    let mut lines: Vec<String> = Vec::new();
    let title = " 查找 ";
    // Corner-flush title row (review D3-1 — same fix as the tabbed sheet).
    let fill = inner.saturating_sub(str_width(title) as usize);
    lines.push(pad_right(&format!("┌{title}{}┐", "─".repeat(fill)), box_width));

    let focus_mark = |row: usize| -> &'static str {
        if find.focus_row == row { "▸ " } else { "  " }
    };
    let row_line = |mark: &str, content: String| -> String {
        format!(
            "│ {} │",
            pad_right(&format!("{mark}{content}"), inner as u16 - 2)
        )
    };

    // Row 0 — query input.
    let query_text = find.query.text().replace('\n', "\\n");
    lines.push(row_line(focus_mark(0), format!("查找: {query_text}")));

    // Row 1 — scope.
    let scope = find.scopes.get(find.scope).copied().unwrap_or("对话");
    lines.push(row_line(focus_mark(1), format!("范围: {scope}")));

    // Row 2 — case; Row 3 — regex.
    let case = if find.case_sensitive { "是" } else { "否" };
    let regex = if find.use_regex { "是" } else { "否" };
    lines.push(row_line(focus_mark(2), format!("大小写: {case}")));
    lines.push(row_line(focus_mark(3), format!("正则: {regex}")));

    // Row 4 — actions.
    lines.push(row_line(focus_mark(4), "[ 查找 ] [ 取消 ]".to_string()));

    lines.push(format!("└{}┘", "─".repeat(inner)));
    // Find content is fixed-height (6 rows) — truncate defensively.
    if lines.len() > max_height as usize {
        lines.truncate(max_height as usize);
        lines.push(format!("└{}┘", "─".repeat(inner)));
    }
    lines
}

/// Word wrap with hard-break fallback (Python `_wrap_line`).
pub fn wrap_line(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_string()];
    }
    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_w = 0usize;
    for word in text.split(' ') {
        let word_w = str_width(word) as usize;
        let sep_w = if current.is_empty() { 0 } else { 1 };
        if current_w + sep_w + word_w <= max_width {
            if !current.is_empty() {
                current.push(' ');
                current_w += 1;
            }
            current.push_str(word);
            current_w += word_w;
        } else {
            if !current.is_empty() {
                result.push(std::mem::take(&mut current));
                current_w = 0;
            }
            if word_w <= max_width {
                current.push_str(word);
                current_w = word_w;
            } else {
                // Hard break the long token.
                let mut rest = word.to_string();
                while str_width(&rest) as usize > max_width {
                    let mut cut = String::new();
                    let mut cut_w = 0usize;
                    for c in rest.chars() {
                        let w = crate::theme::char_width(c) as usize;
                        if cut_w + w > max_width {
                            break;
                        }
                        cut.push(c);
                        cut_w += w;
                    }
                    let cut_len = cut.len();
                    result.push(cut);
                    rest = rest[cut_len..].to_string();
                }
                if !rest.is_empty() {
                    current_w = str_width(&rest) as usize;
                    current = rest;
                }
            }
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{PermissionOutcome, PendingPermission};
    use crate::dialogs::DialogAction;
    use crate::events::TuiEvent;

    fn app() -> TuiApp {
        TuiApp::new()
    }

    #[test]
    fn wrap_line_basic_and_hard_break() {
        assert_eq!(wrap_line("hello world", 5), vec!["hello", "world"]);
        // 6 CJK chars = 12 cols; hard breaks into 2-char (4-col) chunks.
        assert_eq!(wrap_line("一二三四五六", 4), vec!["一二", "三四", "五六"]);
        let long = wrap_line("aaaaaaaa", 4);
        assert_eq!(long, vec!["aaaa", "aaaa"]);
    }

    #[test]
    fn chat_message_bordered_card_is_rectangular() {
        let msg = ChatMessage {
            role: "用户".into(),
            content: "你好".into(),
            turn: 1,
            collapsible: true,
            warning: false,
            collapsed: false,
        };
        let lines = render_chat_message(&msg, 20);
        // Title = " [用户] · turn 1" (16 cols), inner = 18 → fill 0;
        // every row must be exactly 20 cols with all four corners intact.
        assert_eq!(lines[0], "┌ [用户] · turn 1 ┐ ");
        assert_eq!(lines[1], "│ 你好             │");
        assert_eq!(lines[2], "└──────────────────┘");
        for line in &lines {
            assert_eq!(str_width(line), 20, "card rows must be rectangular");
        }
        assert!(lines[0].starts_with('┌') && lines[0].contains('┐'));
        assert!(lines.last().unwrap().starts_with('└') && lines.last().unwrap().contains('┘'));
    }

    #[test]
    fn chat_message_card_renders_inside_pane_width() {
        // The card must fit the pane's inner width without losing corners
        // (P1-1 regression: rows were inner+1/+2 and got truncated).
        let msg = ChatMessage {
            role: "模型".into(),
            content: "多行\n内容".into(),
            turn: 1,
            collapsible: true,
            warning: false,
            collapsed: false,
        };
        let lines = render_content_pane(
            &ContentPane {
                items: vec![ContentItem::Message(msg)],
                current_model_index: Some(0),
                empty_hint: String::new(),
            },
            83,
        );
        for line in &lines {
            assert_eq!(str_width(line), 83, "card must exactly fill inner width");
        }
        let joined = lines.join("\n");
        assert!(joined.contains("┌ [模型]"), "top-left corner present");
        assert!(joined.contains("┐"), "top-right corner present");
        assert!(joined.contains("┘"), "bottom-right corner present");
    }

    #[test]
    fn chat_message_collapsed_summary() {
        let msg = ChatMessage {
            role: "模型".into(),
            content: "长内容".repeat(30),
            turn: 0,
            collapsible: true,
            warning: false,
            collapsed: true,
        };
        let lines = render_chat_message(&msg, 40);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("▸"));
        assert!(lines[0].contains("..."));
    }

    #[test]
    fn warning_collapsed_renders_full_card() {
        let msg = ChatMessage {
            role: "系统".into(),
            content: "block".into(),
            turn: 0,
            collapsible: false,
            warning: true,
            collapsed: true,
        };
        let lines = render_chat_message(&msg, 20);
        assert!(lines[0].starts_with("┌"), "warning must never collapse");
    }

    #[test]
    fn tool_collapsed_format() {
        let mut trace = ToolTraceLine {
            tool_name: "read_file".into(),
            entries: Vec::new(),
            expanded: false,
        };
        trace.entries.push(crate::view_model::ToolEntry {
            target: "lib.rs".into(),
            detail: "运行中".into(),
        });
        let line = render_tool_collapsed(&trace, 40);
        assert!(line.starts_with("  [read_file]"));
        assert!(line.contains("lib.rs · 运行中"));
        assert!(line.ends_with("▸ [展开]"));
    }

    #[test]
    fn tool_expanded_format() {
        let mut trace = ToolTraceLine {
            tool_name: "bash".into(),
            entries: Vec::new(),
            expanded: true,
        };
        trace.entries.push(crate::view_model::ToolEntry {
            target: "dir".into(),
            detail: "成功".into(),
        });
        let lines = render_expanded_tool(&trace, 40);
        assert_eq!(lines[0], "  [bash] ▸ [关闭]");
        assert_eq!(lines[1], "     1  dir · 成功");
    }

    #[test]
    fn full_screen_100x30_frozen_layout() {
        let mut a = app();
        // Frozen-layout state with the marker column shown (sidebars default
        // hidden since the slice #9 user ruling — see
        // layout_no_sidebars_big_main_window).
        a.show_marker = true;
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "你好".into(),
            character_count: 2,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "回答".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        a.accept_event(TuiEvent::ToolAvailabilityCheck {
            available: 3,
            unavailable: 1,
            degraded: 0,
            unprobed: 0,
            gate_decision: "allow".into(),
        });
        let lines = compose_screen(&a, 100, 30);
        assert_eq!(lines.len(), 30);
        // Menu row (right-padded to the viewport width).
        assert_eq!(lines[0].trim_end(), "文件 事件 标记 编辑 模型 来源 运行 验证 帮助");
        // Toolbar contains the frozen buttons with the grouping gap.
        assert!(lines[1].contains("后退 前进 刷新 停止 打开 验证 属性"));
        assert!(lines[1].contains("命令..."));
        assert!(lines[1].contains("查找..."));
        // Body top border with marker T-junction.
        assert!(lines[2].contains("├"));
        assert!(lines[2].contains("┬"));
        // Status bar frozen pattern.
        let last = lines.last().unwrap();
        assert!(last.starts_with("守护 | 网络关闭 | 沙箱严格 | 来源 3/4 | 模型 | 运行中"));
    }

    #[test]
    fn full_screen_140x40_marker_none() {
        let mut a = app();
        a.show_marker = true;
        let lines = compose_screen(&a, 140, 40);
        assert_eq!(lines.len(), 40);
        let marker_row = &lines[2];
        // Marker column shown; empty state shows (none) inside.
        assert!(marker_row.contains("┬"));
        let body = lines[3..39].join("\n");
        assert!(body.contains("(none)"));
    }

    #[test]
    fn body_row_alignment_with_marker_junction() {
        let mut a = app();
        a.show_marker = true;
        let lines = compose_screen(&a, 100, 30);
        // Top border, body rows and bottom border must all fill the viewport
        // in DISPLAY columns (CJK counts 2 — chars().count() differs).
        let top = &lines[2];
        let row = &lines[3];
        let bottom = &lines[lines.len() - 3];
        assert_eq!(str_width(top), 100);
        assert_eq!(str_width(row), 100);
        assert_eq!(str_width(bottom), 100);
        // The junction sits at the content/marker boundary (display col 84:
        // 1 border + 83 content-inner columns).
        assert_eq!(top.chars().nth(84), Some('┬'));
        assert_eq!(bottom.chars().nth(84), Some('┴'));
    }

    #[test]
    fn marker_toggle_hides_column() {
        let mut a = app();
        a.show_marker = false;
        let lines = compose_screen(&a, 100, 30);
        let top = &lines[2];
        assert!(!top.contains("┬"), "marker column must be gone");
    }

    // ── Phase 3 slice #9: sidebar layouts ──────────────────────────────────

    /// Default (user ruling): both sidebars hidden — full-width 大主窗.
    #[test]
    fn layout_no_sidebars_big_main_window() {
        let a = app();
        assert!(!a.show_explorer && !a.show_marker, "sidebars default hidden");
        let lines = compose_screen(&a, 100, 30);
        let top = &lines[2];
        assert!(!top.contains('┬'), "no junctions with all sidebars hidden");
        assert!(top.starts_with('├') && top.ends_with('┤'));
        assert_eq!(str_width(top), 100);
        // Content spans the full width (row 3 is a body row).
        assert_eq!(str_width(&lines[3]), 100);
        let body = lines[3..lines.len() - 3].join("\n");
        assert!(body.contains("输入问题开始对话..."), "empty-hint centered");
    }

    /// Both sidebars shown — junctions at cols 21 (explorer) and 84 (marker).
    #[test]
    fn layout_100x30_explorer_marker() {
        let mut a = app();
        a.show_explorer = true;
        a.show_marker = true;
        a.explorer.loaded = true;
        let lines = compose_screen(&a, 100, 30);
        let top = &lines[2];
        assert_eq!(str_width(top), 100);
        // `├` + 20 explorer-inner + `┬` + 62 content-inner + `┬` + 14
        // marker-inner + `┤`.
        assert_eq!(top.chars().next(), Some('├'));
        assert_eq!(top.chars().nth(21), Some('┬'));
        assert_eq!(top.chars().nth(84), Some('┬'));
        assert_eq!(top.chars().nth(99), Some('┤'));
        // Rows carry three columns.
        let row = &lines[3];
        assert_eq!(str_width(row), 100);
        assert!(row.contains("探索器"), "explorer header visible");
        let bottom = &lines[lines.len() - 3];
        assert_eq!(bottom.chars().nth(21), Some('┴'));
        assert_eq!(bottom.chars().nth(84), Some('┴'));
    }

    #[test]
    fn layout_explorer_only() {
        let mut a = app();
        a.show_explorer = true;
        let lines = compose_screen(&a, 100, 30);
        let top = &lines[2];
        // `├` + 20 + `┬` + 77 content-inner + `┤` (junction at col 21 only).
        assert_eq!(top.chars().nth(21), Some('┬'));
        assert!(
            !top.chars().skip(22).any(|c| c == '┬'),
            "no marker junction"
        );
        let row = &lines[3];
        assert!(row.contains("探索器"));
        assert_eq!(str_width(row), 100);
    }

    #[test]
    fn layout_marker_only() {
        let mut a = app();
        a.show_marker = true;
        let lines = compose_screen(&a, 100, 30);
        let top = &lines[2];
        assert_eq!(top.chars().nth(84), Some('┬'));
        assert_eq!(str_width(top), 100);
    }

    #[test]
    fn explorer_renders_tree_events_and_session_list() {
        let mut a = app();
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
            expanded: true,
            uri: None,
        }];
        a.accept_event(TuiEvent::StatusUpdate { label: "x".into(), ok: true });
        a.explorer.refresh_events(&a.events_log);
        let lines = compose_screen(&a, 100, 30);
        let body = lines[3..27].join("\n");
        assert!(body.contains("探索器"));
        assert!(body.contains("▾ src"), "expanded dir marker");
        assert!(body.contains("lib.rs"));
        assert!(body.contains("事件"), "events section header");
        assert!(body.contains("status_update (1)"), "event group count");

        // Session mode replaces the tree + events.
        a.explorer.session_mode = true;
        a.explorer.sessions = vec![crate::explorer::SessionEntry {
            run_dir: std::path::PathBuf::from("x"),
            session_id: "RUN-a1b2c3d4-0".into(),
            prompt_preview: "审查".into(),
            turn_count: 1,
            date: "2026-08-05".into(),
        }];
        let lines = compose_screen(&a, 100, 30);
        let body = lines[3..27].join("\n");
        assert!(body.contains("运行历史"), "honest per-run labeling (D2-1)");
        assert!(body.contains("2026-08-05 (1)"));
        assert!(body.contains("[a1b2c3d4] 1t"));
        assert!(!body.contains("探索器"), "tree hidden in session mode");
    }

    #[test]
    fn tabbed_overlay_rows_are_rectangular() {
        let sheet = crate::modals::HelpOverlay::new("off").sheet;
        let lines = compose_tabbed_overlay(&sheet, 60, 30);
        for line in &lines {
            assert_eq!(str_width(line), 60, "tabbed rows must be rectangular: {line:?}");
        }
        assert!(lines[0].starts_with('┌'));
        assert!(lines[0].ends_with('┐'), "title corner flush at w-1 (D3-1)");
        assert!(lines[1].contains("▶快捷键"), "active tab marker");
        assert!(lines.last().unwrap().starts_with('└'));
        assert!(lines.iter().any(|l| l.contains("Esc 关闭")));
        // 命令 tab content derives from the registry.
        let mut sheet2 = sheet.clone();
        sheet2.next_tab();
        let lines = compose_tabbed_overlay(&sheet2, 60, 30);
        assert!(lines.iter().any(|l| l.contains("/toggle-explorer")));
    }

    #[test]
    fn tabbed_overlay_height_adaptive() {
        let sheet = crate::modals::HelpOverlay::new("off").sheet;
        // A tiny box drops content rows but keeps footer + borders.
        let lines = compose_tabbed_overlay(&sheet, 60, 5);
        assert!(lines.len() <= 5);
        assert!(lines.last().unwrap().starts_with('└'));
        assert!(lines.iter().any(|l| l.contains("Esc 关闭")), "footer survives");
    }

    #[test]
    fn snapshots_overlay_rows_are_rectangular() {
        use crate::snapshots::SnapshotEntry;
        let entry = |h: &str, files: Option<usize>| SnapshotEntry {
            hash: h.into(),
            tool: "search_replace".into(),
            targets: vec!["a.txt".into()],
            run_id: "RUN-a1b2c3d4-3".into(),
            date: "2026-08-05".into(),
            file_count: files,
        };
        let mut sel = crate::modals::SnapshotSelector::new(vec![
            entry(&"a".repeat(64), Some(2)),
            entry(&"b".repeat(64), None),
        ]);
        sel.selected = 1;
        let lines = compose_snapshots_overlay(&sel, 60, 20);
        for line in &lines {
            assert_eq!(str_width(line), 60, "snapshot rows must be rectangular: {line:?}");
        }
        assert!(lines[0].starts_with('┌'));
        assert!(lines[0].ends_with('┐'), "title corner flush at w-1 (D3-1)");
        assert!(lines[0].contains("快照"));
        assert!(lines.iter().any(|l| l.contains("▸ [a1b2c3d4]")), "selection cursor + sid8");
        assert!(lines.iter().any(|l| l.contains("2 文件")), "file count from manifest");
        assert!(lines.iter().any(|l| l.contains("—")), "missing manifest renders —");
        assert!(lines.last().unwrap().starts_with('└'));
        assert!(lines.iter().any(|l| l.contains("Enter 确认恢复")));

        // Confirm state on the selected row.
        sel.confirm = true;
        let lines = compose_snapshots_overlay(&sel, 60, 20);
        assert!(
            lines.iter().any(|l| l.contains("确认恢复？")),
            "confirm prompt on the selected row"
        );
    }

    #[test]
    fn snapshots_overlay_empty_and_height_adaptive() {
        use crate::snapshots::SnapshotEntry;
        // Empty list → a single content row.
        let empty = crate::modals::SnapshotSelector::new(vec![]);
        let lines = compose_snapshots_overlay(&empty, 60, 20);
        assert!(lines.iter().any(|l| l.contains("(无快照")));
        for line in &lines {
            assert_eq!(str_width(line), 60);
        }
        // Tiny box with many entries: footer + bottom border survive.
        let entries: Vec<SnapshotEntry> = (0..20)
            .map(|i| SnapshotEntry {
                hash: format!("h{i}"),
                tool: "t".into(),
                targets: vec![],
                run_id: format!("RUN-a1b2c3d4-{i}"),
                date: "2026-08-05".into(),
                file_count: None,
            })
            .collect();
        let sel = crate::modals::SnapshotSelector::new(entries);
        let lines = compose_snapshots_overlay(&sel, 60, 6);
        assert!(lines.len() <= 6);
        assert!(lines.last().unwrap().starts_with('└'));
        assert!(lines.iter().any(|l| l.contains("Esc 关闭")), "footer survives");
    }

    #[test]
    fn snapshots_overlay_renders_in_full_screen() {
        let dir = std::env::temp_dir().join(format!(
            "orz-tui-widgets-snapshots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let runs = dir.join(".gsa").join("runs").join("RUN-a1b2c3d4-0");
        std::fs::create_dir_all(&runs).unwrap();
        let mut ev = orz_assurance::journal::RunEvent::new(
            "RUN-a1b2c3d4-0".into(),
            0,
            orz_assurance::journal::EventType::SnapshotCreated,
            "m".into(),
            None,
            "run-event-v0.1.schema.json".into(),
            serde_json::json!({"tool": "search_replace", "targets": ["a.txt"], "snapshot_hash": "a".repeat(64)}),
            orz_assurance::journal::Redaction::None,
            "2026-08-05T00:00:00Z".into(),
        );
        orz_assurance::seal_event(&mut ev).unwrap();
        std::fs::write(
            runs.join("events.jsonl"),
            format!("{}\n", serde_json::to_string(&ev).unwrap()),
        )
        .unwrap();

        let mut app = crate::app::TuiApp::new();
        app.cwd = dir.clone();
        app.open_snapshots();
        let screen = compose_screen(&app, 100, 40);
        let joined = screen.join("\n");
        assert!(joined.contains("快照"), "overlay blended into the frame");
        assert!(joined.contains("[a1b2c3d4]"), "entry row rendered");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn find_overlay_rows_are_rectangular_with_focus_marks() {
        let mut f = crate::modals::FindDialog::new();
        f.query.set_text("审查");
        f.focus_row = 2;
        let lines = compose_find_overlay(&f, 60, 30);
        for line in &lines {
            assert_eq!(str_width(line), 60, "find rows must be rectangular");
        }
        assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
        assert!(lines[1].contains("查找: 审查"));
        assert!(lines[3].contains("▸ 大小写: 否"), "focus row 2 marked");
        assert!(lines[4].contains("正则: 否"));
        assert!(lines[5].contains("[ 查找 ] [ 取消 ]"));
    }

    #[test]
    fn small_viewport_message() {
        let lines = compose_screen(&app(), 30, 10);
        assert!(lines.last().unwrap().contains("视口过小"));
    }

    #[test]
    fn minimum_80x24_renders() {
        // Review P3-3: the plan requires 80x24 to render (design minimum is
        // 100x30; the absolute floor is 60x15).
        let mut a = app();
        a.accept_event(TuiEvent::PromptSubmitted {
            prompt: "你好".into(),
            character_count: 2,
        });
        a.accept_event(TuiEvent::ModelOutput {
            text: "回答".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let lines = compose_screen(&a, 80, 24);
        assert_eq!(lines.len(), 24);
        assert!(lines[0].contains("文件"));
        assert!(lines[lines.len() - 1].contains("守护"));
    }

    #[test]
    fn dialog_overlay_composition() {
        let d = crate::dialogs::Dialog::new(
            "工具权限请求",
            vec!["工具: bash".into()],
            vec![
                DialogAction::AllowOnce { option_id: "allow-once".into() },
                DialogAction::Cancel,
            ],
        );
        let lines = compose_dialog_overlay(&d, 60, None);
        assert!(lines[0].starts_with("┌ 工具权限请求"));
        assert!(lines[1].contains("工具: bash"));
        assert!(lines[2].contains("<允许一次>"));
        assert!(lines[2].contains("[取消]"));
        // Rectangular: every row exactly 60 columns (review P3-6c).
        for line in &lines {
            assert_eq!(str_width(line), 60, "dialog rows must be rectangular");
        }

        // Phase 3 slice #7: the countdown renders between message and
        // actions, keeping the box rectangular.
        let lines = compose_dialog_overlay(&d, 60, Some("剩余 50s".into()));
        assert!(lines[1].contains("工具: bash"));
        assert!(lines[2].contains("剩余 50s"), "{lines:?}");
        assert!(lines[3].contains("<允许一次>"));
        for line in &lines {
            assert_eq!(str_width(line), 60, "countdown rows must be rectangular");
        }
    }

    /// Phase 3 slice #7: the permission dialog's countdown line is derived
    /// at render time from the pending request's open instant.
    #[test]
    fn permission_dialog_renders_remaining_seconds_line() {
        let mut a = app();
        a.show_permission_dialog(PendingPermission::new("bash", "dir", None));
        // Backdate AFTER presentation — the countdown starts at show time.
        a.pending_permission.as_mut().unwrap().opened_at =
            std::time::Instant::now() - std::time::Duration::from_secs(250);
        let lines = compose_screen(&a, 100, 30);
        let joined = lines.join("\n");
        assert!(
            joined.contains("剩余 "),
            "countdown line must render: {joined}"
        );
        // 50s remaining (tolerance: ±1s tick granularity).
        let remaining = a.pending_permission.as_ref().unwrap().remaining().unwrap();
        let secs = remaining.as_secs();
        assert!(
            (49..=50).contains(&secs),
            "250s elapsed of 300s leaves ~50s, got {secs}s"
        );
    }

    #[test]
    fn permission_dialog_renders_in_screen() {
        let mut a = app();
        a.show_permission_dialog(PendingPermission::new(
            "bash",
            "dir",
            Some(Box::new(|_: PermissionOutcome| {})),
        ));
        let lines = compose_screen(&a, 100, 30);
        let joined = lines.join("\n");
        assert!(joined.contains("工具权限请求"));
        assert!(joined.contains("允许一次"));
        assert!(joined.contains("取消"));
    }

    #[test]
    fn input_row_and_status_bar() {
        let mut a = app();
        a.input.textarea.set_text("测试输入");
        let lines = compose_screen(&a, 100, 30);
        assert!(lines[28].starts_with("> 测试输入"));
        assert_eq!(
            lines[29].trim_end(),
            "守护 | 网络关闭 | 沙箱严格 | 来源 0/0 | 模型 | 空闲"
        );
    }
}
