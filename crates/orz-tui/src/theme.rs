//! Visual tokens and width helpers — the retro workbench theme.
//!
//! Colors are optional in the frozen design (the primary discriminator is
//! border vs borderless, CONTENT_PANE_CONVERSATION_RENDERING_v0.1), so this
//! module is deliberately small. All CJK-aware truncation/padding lives here
//! so snapshot tests can pin exact behavior.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// Minimum viewport from CLI_UI_INTERACTION_MODEL_v0.1.
pub const MIN_WIDTH: u16 = 100;
pub const MIN_HEIGHT: u16 = 30;

/// Marker column width (right side).
pub const MARKER_WIDTH: u16 = 16;
/// Explorer column width (left side) — fixed for v1 (content-only body).
pub const EXPLORER_WIDTH: u16 = 22;

/// Display width of a single character (CJK = 2, ambiguous = 1).
pub fn char_width(c: char) -> u16 {
    u16::try_from(c.width().unwrap_or(0)).unwrap_or(0)
}

/// Display width of a string.
pub fn str_width(s: &str) -> u16 {
    u16::try_from(s.width()).unwrap_or(0)
}

/// Truncate *s* to at most *max_width* display columns, appending the
/// continuation marker when truncation happened. Never splits a wide char.
pub fn truncate_to_width(s: &str, max_width: u16, marker: &str) -> String {
    if str_width(s) <= max_width {
        return s.to_string();
    }
    let marker_width = str_width(marker);
    let budget = max_width.saturating_sub(marker_width);
    let mut out = String::new();
    let mut used: u16 = 0;
    for c in s.chars() {
        let w = char_width(c);
        if used + w > budget {
            break;
        }
        out.push(c);
        used += w;
    }
    format!("{out}{marker}")
}

/// Right-pad (or truncate) *s* to exactly *width* display columns.
pub fn pad_right(s: &str, width: u16) -> String {
    let w = str_width(s);
    if w >= width {
        truncate_to_width(s, width, "")
    } else {
        let mut out = s.to_string();
        out.extend(std::iter::repeat_n(' ', (width - w) as usize));
        out
    }
}

/// Strip control characters (incl. ESC escape sequences) from text that
/// will be rendered — model/tool output is untrusted and could otherwise
/// inject terminal control sequences. Newlines and tabs are preserved
/// (rendering depends on them).
pub fn sanitize_text(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect()
}

/// Left-pad *s* to at least *width* display columns.
pub fn pad_left(s: &str, width: u16) -> String {
    let w = str_width(s);
    if w >= width {
        s.to_string()
    } else {
        format!("{}{}", " ".repeat((width - w) as usize), s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cjk_widths_are_correct() {
        assert_eq!(str_width("中"), 2);
        assert_eq!(str_width("abc"), 3);
        // Box drawing and marker glyphs are width 1 (ambiguous → 1).
        assert_eq!(str_width("┌─┐"), 3);
        assert_eq!(str_width("▸"), 1);
        assert_eq!(str_width("●"), 1);
    }

    #[test]
    fn truncate_never_splits_wide_chars() {
        let s = "你好世界";
        let t = truncate_to_width(s, 4, "…");
        // Budget 2 cols + 1-col marker: exactly one CJK char + marker.
        assert_eq!(t, "你…");
        assert!(str_width(&t) <= 4);

        let full = truncate_to_width(s, 8, "…");
        assert_eq!(full, "你好世界");
    }

    #[test]
    fn sanitize_strips_escape_sequences_keeps_newlines() {
        let evil = "正常\x1b[2J内容\n第二行\t带制表";
        let clean = sanitize_text(evil);
        // ESC is stripped; the printable CSI remnant "[2J" is inert text
        // (a terminal sequence needs its ESC introducer to execute).
        assert!(!clean.contains('\x1b'));
        assert_eq!(clean, "正常[2J内容\n第二行\t带制表");
        // OSC 52 clipboard exfiltration attempt: ESC + BEL both stripped.
        let esc = sanitize_text("\x1b]52;c;YmluYXJ5\x07正文");
        assert!(!esc.contains('\x1b') && !esc.contains('\x07'));
        assert!(esc.contains("正文"));
    }

    #[test]
    fn pad_helpers_round_trip() {
        assert_eq!(pad_right("守", 4), "守  ");
        assert_eq!(pad_right("abcdef", 4), "abcd");
        assert_eq!(pad_left("x", 3), "  x");
        assert_eq!(pad_left("xxxx", 2), "xxxx");
    }
}
