//! Render-snapshot helpers — render the full frame through the real
//! `render_frame` path into a `TestBackend` and dump it as text (Python
//! `FullScreenRenderTests` shape). Headless: no crossterm raw mode.

use ratatui::Terminal;
use ratatui::backend::TestBackend;

use crate::app::TuiApp;

/// Render the app through `render_frame` into a TestBackend and return the
/// screen as a single string (rows joined with '\n').
///
/// The conversion skips each wide character's continuation cell (its second
/// buffer cell carries an empty/space symbol), so the output text matches
/// `compose_screen` exactly — wide chars occupy two cells but one glyph.
pub fn render_to_string(app: &mut TuiApp, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("test terminal");
    app.render(&mut terminal).expect("render frame");
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
            let sym = cell.symbol();
            out.push_str(sym);
            if unicode_width::UnicodeWidthStr::width(sym) >= 2 {
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
    use crate::events::TuiEvent;

    #[test]
    fn rendered_frame_100x30_matches_compose_screen() {
        let mut app = TuiApp::new();
        app.accept_event(TuiEvent::PromptSubmitted {
            prompt: "测试问题".into(),
            character_count: 4,
        });
        app.accept_event(TuiEvent::ModelOutput {
            text: "测试回答".into(),
            tool_calls: vec![],
            finish_reason: "stop".into(),
        });
        let rendered = render_to_string(&mut app, 100, 30);
        // The frozen layout markers must survive the real render path.
        assert!(rendered.contains("文件 事件 标记 编辑 模型 来源 运行 验证 帮助"));
        assert!(rendered.contains("后退 前进 刷新 停止 打开 验证 属性"));
        assert!(rendered.contains("命令..."));
        assert!(rendered.contains("守护 | 网络关闭 | 沙箱严格 | 工具 0/0 | 模型 | 运行中"));
        assert!(rendered.contains("┌ [用户]"));
        assert!(rendered.contains("[模型]"));
    }

    #[test]
    fn rendered_frame_dialog_overlay() {
        let mut app = TuiApp::new();
        app.show_permission_dialog(crate::app::PendingPermission::new("bash", "dir", None));
        let rendered = render_to_string(&mut app, 100, 30);
        assert!(rendered.contains("工具权限请求"));
        assert!(rendered.contains("允许一次"));
        assert!(rendered.contains("取消"));
    }

    #[test]
    fn rendered_frame_30x10_shows_viewport_message() {
        let mut app = TuiApp::new();
        let rendered = render_to_string(&mut app, 30, 10);
        assert!(rendered.contains("视口过小"));
    }
}
