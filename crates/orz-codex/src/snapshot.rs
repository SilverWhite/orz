//! Render-snapshot test helper — render through the real `render_frame`
//! path into a `TestBackend` and dump it as text (mirrors orz-tui's
//! `snapshot.rs` pattern, wide-char continuation cells skipped).

use orz_tui::theme::str_width;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::CodexApp;
use crate::widgets::render_frame;

/// Render the current app state to a plain string for assertions. Each wide
/// character's continuation cell is skipped (the second buffer cell carries
/// an empty symbol), so the output matches what a user sees — wide chars
/// occupy two cells but one glyph.
pub fn render_to_string(app: &mut CodexApp, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    render_frame(&mut terminal, app).expect("render");
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    let mut skip_wide_continuation = false;
    for y in 0..height {
        for x in 0..width {
            let cell = buffer.cell((x, y)).expect("cell in bounds");
            if skip_wide_continuation {
                skip_wide_continuation = false;
                continue;
            }
            let symbol = cell.symbol();
            out.push_str(symbol);
            if str_width(symbol) >= 2 {
                skip_wide_continuation = true;
            }
        }
        if y + 1 < height {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_to_string_is_deterministic_and_bounded() {
        let mut app = CodexApp::new();
        app.submit_prompt("测试".to_owned());
        let a = render_to_string(&mut app, 80, 24);
        let b = render_to_string(&mut app, 80, 24);
        assert_eq!(a, b);
        assert_eq!(a.lines().count(), 24);
        // Wide chars render as one glyph (no continuation-cell gaps).
        assert!(a.contains("测试"), "{a}");
        assert!(
            !a.contains("测 试"),
            "no space injected between CJK chars: {a}"
        );
    }
}
