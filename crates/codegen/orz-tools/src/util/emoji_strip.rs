//! Write-face emoji stripping (0bi ⑪, 2026-09-23).
//!
//! Single implementation point for the "no emoji in written files" policy:
//! the edit family's input decode point (`run_search_replace`) applies
//! [`strip_emoji`] to `new_string` before dispatch. The read face and the
//! conversation face are untouched. Governing design:
//! `docs/WRITE_FACE_EMOJI_STRIPPING_DESIGN_2026-09-23.md` (v1.0).
//!
//! Mechanic semantics:
//! - Strip set: `Emoji_Presentation ∪ Extended_Pictographic ∪ sequence
//!   residues` (ZWJ, VS15/16, keycap, skin tones, regional indicators, tag
//!   chars) — see [`super::emoji_strip_ranges::WRITE_FACE_STRIP_RANGES`].
//! - Maximal runs are purged whole, so a ZWJ sequence leaves no residue.
//! - Keycap bases (`0-9` `#` `*` immediately before `FE0F? 20E3`) go with
//!   their sequence.
//! - Whitespace: only when both sides of a run are horizontal whitespace
//!   (space / tab) is the run folded to a single space; otherwise the run is
//!   deleted as-is (no new whitespace is introduced; never across lines).
//! - 0bs ⑦（2026-09-25，F13 用户令）：**状态符号白名单窗口**——报告/交接
//!   文档表头与状态列常用记号（对勾/叉/警示/时间/红黄绿灯/旗标等，集合见
//!   [`super::emoji_strip_ranges::STATUS_SYMBOL_WINDOW`]）整体放行（含
//!   VS15/16 变体、不拆序列），不拦截、不计数告知；其余 emoji 照旧剥离。
//! - Escape hatch: `ORZ_WRITE_KEEP_EMOJI=1|true` disables stripping so that
//!   emoji fixtures keep byte-identical writes.

use super::emoji_strip_ranges::{STATUS_SYMBOL_WINDOW, WRITE_FACE_STRIP_RANGES};

/// Env var that disables write-face emoji stripping (`1` / `true`).
pub(crate) const KEEP_EMOJI_ENV: &str = "ORZ_WRITE_KEEP_EMOJI";

/// Serializes tests that toggle the emoji escape-hatch env var (0bi ⑪).
/// 0bl 审查修复（2026-09-24）：从 search_replace 的测试模块上移到此处，
/// 让 search_replace 与 hashline_edit 的 emoji 测试共用同一把锁——两处
/// 工具都读 `keep_emoji_requested()`，必须与设开关的测试互斥。
#[cfg(test)]
pub(crate) static EMOJI_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Maximum number of distinct line numbers rendered in the notice.
const NOTICE_LINE_CAP: usize = 20;

/// Result of one strip pass over write-face text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StripNotice {
    /// Number of stripped runs (strip points).
    pub(crate) count: usize,
    /// Distinct 1-based line numbers carrying stripped runs, in order.
    pub(crate) lines: Vec<usize>,
}

impl StripNotice {
    /// One-line mechanical notice, e.g.
    /// `[emoji 剥离] 3 处（写入内容 L12/L45/L46 行）`.
    ///
    /// 0bl 审查修复（2026-09-24）：行号语义——这些是**写入内容内部**的
    /// 1 基行号（交给写面的那段文本），不是真实文件中的行号：写入文本
    /// 会被编辑工具拼进更大的文件，直接当文件坐标读会错位。文案显式
    /// 点明作用域（`写入内容 … 行`），避免误读。
    pub(crate) fn render(&self) -> String {
        let mut shown: Vec<String> = self
            .lines
            .iter()
            .take(NOTICE_LINE_CAP)
            .map(|line| format!("L{line}"))
            .collect();
        if self.lines.len() > NOTICE_LINE_CAP {
            shown.push("…".to_string());
        }
        format!(
            "[emoji 剥离] {} 处（写入内容 {} 行）",
            self.count,
            shown.join("/")
        )
    }
}

/// Whether the escape hatch is enabled (`ORZ_WRITE_KEEP_EMOJI=1|true`).
pub(crate) fn keep_emoji_requested() -> bool {
    keep_emoji_requested_value(std::env::var(KEEP_EMOJI_ENV).ok().as_deref())
}

fn keep_emoji_requested_value(value: Option<&str>) -> bool {
    match value.map(|value| value.trim().to_ascii_lowercase()) {
        Some(value) => value == "1" || value == "true",
        None => false,
    }
}

fn is_strip_char(c: char) -> bool {
    let cp = c as u32;
    WRITE_FACE_STRIP_RANGES
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                std::cmp::Ordering::Greater
            } else if cp > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// 0bs ⑦（F13 用户令定案）：**状态符号白名单窗口**判定——窗口内字符
/// 整体放行（不拦截、不计数），见
/// [`super::emoji_strip_ranges::STATUS_SYMBOL_WINDOW`]。
fn is_status_symbol(c: char) -> bool {
    let cp = c as u32;
    STATUS_SYMBOL_WINDOW
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                std::cmp::Ordering::Greater
            } else if cp > hi {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn is_keycap_base(c: char) -> bool {
    c.is_ascii_digit() || c == '#' || c == '*'
}

/// Strip write-face emoji from `text`.
///
/// Returns the text to write plus a notice when anything was stripped;
/// `(text.to_string(), None)` when the text is already clean.
pub(crate) fn strip_emoji(text: &str) -> (String, Option<StripNotice>) {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut count = 0usize;
    let mut lines: Vec<usize> = Vec::new();
    let mut line = 1usize;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        // 0bs ⑦（F13 用户令定案）：状态符号白名单窗口——整体放行（VS15/16
        // 变体随基座，不拆序列）、不拦截、不计数告知。
        if is_status_symbol(c) {
            out.push(c);
            i += 1;
            while i < chars.len() && matches!(chars[i], '\u{FE0E}' | '\u{FE0F}') {
                out.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if !is_strip_char(c) {
            if c == '\n' {
                line += 1;
            }
            out.push(c);
            i += 1;
            continue;
        }
        // Maximal run of strip-set characters.
        let run_start = i;
        let mut run_end = i;
        // 0bs ⑦：白名单窗口是运行边界——状态符号不被并入 emoji 运行。
        while run_end < chars.len()
            && is_strip_char(chars[run_end])
            && !is_status_symbol(chars[run_end])
        {
            run_end += 1;
        }
        // Keycap base absorption: `0-9#*` + optional VS16 + U+20E3.
        let mut removed_base = false;
        if run_start > 0 && is_keycap_base(chars[run_start - 1]) {
            let begins_keycap = chars[run_start] == '\u{20E3}'
                || (chars[run_start] == '\u{FE0F}'
                    && run_start + 1 < chars.len()
                    && chars[run_start + 1] == '\u{20E3}');
            if begins_keycap {
                // The base char was already emitted; drop it again.
                out.pop();
                removed_base = true;
            }
        }
        count += 1;
        if lines.last() != Some(&line) {
            lines.push(line);
        }
        let left_idx = if removed_base {
            run_start - 1
        } else {
            run_start
        };
        let left = left_idx.checked_sub(1).map(|idx| chars[idx]);
        let right = chars.get(run_end).copied();
        let left_hws = matches!(left, Some(' ') | Some('\t'));
        let right_hws = matches!(right, Some(' ') | Some('\t'));
        if left_hws && right_hws {
            // Fold the two horizontal whitespace neighbours into one space.
            while out.ends_with(' ') || out.ends_with('\t') {
                out.pop();
            }
            out.push(' ');
            let mut next = run_end;
            while next < chars.len() && matches!(chars[next], ' ' | '\t') {
                next += 1;
            }
            i = next;
        } else {
            i = run_end;
        }
    }
    if count == 0 {
        (text.to_string(), None)
    } else {
        (out, Some(StripNotice { count, lines }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stripped(text: &str) -> String {
        strip_emoji(text).0
    }

    #[test]
    fn strip_ranges_sorted_and_disjoint() {
        let mut prev_hi: Option<u32> = None;
        for &(lo, hi) in WRITE_FACE_STRIP_RANGES {
            assert!(lo <= hi, "empty/descending range {lo:#X}..={hi:#X}");
            if let Some(prev) = prev_hi {
                assert!(lo > prev, "overlap at {lo:#X} (previous high {prev:#X})");
            }
            prev_hi = Some(hi);
        }
    }

    #[test]
    fn zwj_sequence_purged_whole() {
        let text = "x \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467} y";
        let (out, notice) = strip_emoji(text);
        assert_eq!(out, "x y");
        assert!(!out.contains('\u{200D}'));
        assert!(!out.contains('\u{FE0F}'));
        let notice = notice.expect("notice");
        assert_eq!(notice.count, 1);
        assert_eq!(notice.render(), "[emoji 剥离] 1 处（写入内容 L1 行）");
    }

    #[test]
    fn vs16_skin_flag_and_keycap_cleared() {
        // VS16 / VS15
        assert_eq!(stripped("x \u{2764}\u{FE0F} y"), "x y");
        assert_eq!(stripped("x \u{2764}\u{FE0E} y"), "x y");
        // Skin tone modifier
        assert_eq!(stripped("x \u{1F44D}\u{1F3FD} y"), "x y");
        // Regional-indicator flag pair
        assert_eq!(stripped("x \u{1F1E8}\u{1F1F3} y"), "x y");
        // Keycap with and without VS16 (base char goes with the sequence)
        assert_eq!(stripped("x 1\u{FE0F}\u{20E3} y"), "x y");
        assert_eq!(stripped("x 7\u{20E3} y"), "x y");
        assert_eq!(stripped("x #\u{20E3} y"), "x y");
    }

    #[test]
    fn non_emoji_symbols_kept() {
        let text = "★ → ⇒ § ± × ÷ ∑ ≠ ∞";
        assert!(!text.chars().any(is_strip_char));
        assert_eq!(strip_emoji(text), (text.to_string(), None));
    }

    #[test]
    fn plain_text_untouched() {
        let text = "hello world\nsecond line\n";
        assert_eq!(strip_emoji(text), (text.to_string(), None));
    }

    #[test]
    fn whitespace_folding_rules() {
        // Both sides horizontal whitespace → single space.
        assert_eq!(stripped("a \u{1F600} b"), "a b");
        assert_eq!(stripped("a  \u{1F600}  b"), "a b");
        assert_eq!(stripped("a\t\u{1F600}\tb"), "a b");
        // Otherwise the run is deleted as-is (no new whitespace).
        assert_eq!(stripped("a\u{1F600}b"), "ab");
        assert_eq!(stripped("a \u{1F600}b"), "a b");
        assert_eq!(stripped("a\u{1F600} b"), "a b");
        // Never across lines.
        assert_eq!(stripped("a\n\u{1F600}\nb"), "a\n\nb");
    }

    #[test]
    fn notice_counts_and_lines() {
        let text = "ok\nx \u{1F389} y\nno\nz \u{1F680} w\n";
        let (out, notice) = strip_emoji(text);
        assert_eq!(out, "ok\nx y\nno\nz w\n");
        let notice = notice.expect("notice");
        assert_eq!(notice.count, 2);
        assert_eq!(notice.lines, vec![2, 4]);
        assert_eq!(notice.render(), "[emoji 剥离] 2 处（写入内容 L2/L4 行）");
    }

    #[test]
    fn emoji_only_line_becomes_empty() {
        let (out, notice) = strip_emoji("a\n\u{1F600}\nb\n");
        assert_eq!(out, "a\n\nb\n");
        assert_eq!(notice.expect("notice").lines, vec![2]);
    }

    #[test]
    fn extended_pictographic_symbols_are_stripped() {
        assert_eq!(stripped("a \u{00A9} b"), "a b"); // ©
        assert_eq!(stripped("a \u{2122} b"), "a b"); // ™
        assert_eq!(stripped("a \u{2194} b"), "a b"); // ↔
    }

    #[test]
    fn escape_hatch_value_parsing() {
        assert!(!keep_emoji_requested_value(None));
        assert!(!keep_emoji_requested_value(Some("")));
        assert!(!keep_emoji_requested_value(Some("0")));
        assert!(!keep_emoji_requested_value(Some("no")));
        assert!(keep_emoji_requested_value(Some("1")));
        assert!(keep_emoji_requested_value(Some("true")));
        assert!(keep_emoji_requested_value(Some(" TRUE ")));
    }

    /// 0bs ⑦ 钉（2026-09-25，F13 用户令）：状态符号白名单窗口——整体放行
    /// （含 VS16 变体）、不拆序列、不拦截、不计数告知。
    #[test]
    fn status_symbol_window_passes_with_vs16_and_never_counts() {
        // 夹具以 `\u{}` 转义承载（免疫写面剥离；0bs ⑦ 实现轮在 0.6.14 载体上
        // 亲历字面符号被旧剥离面移除——S4 复验：同内容进新载体不再剥离）。
        let text = "x \u{2705} \u{23F3} \u{26A0}\u{FE0F} \u{26D4} \u{2757} \
                    \u{274C} \u{2714} \u{2717} \u{1F534} \u{1F7E1} \u{1F7E2} \
                    \u{1F6A9} \u{1F4CC} y";
        assert_eq!(strip_emoji(text), (text.to_string(), None));
        // 与 emoji 混排：只有 emoji 被剥离、状态符号原样留存、计数只算 emoji。
        let (out, notice) = strip_emoji("a \u{1F389}\u{2705}\u{1F6AB}\u{1F389} b");
        assert_eq!(out, "a \u{2705}\u{1F6AB} b");
        let notice = notice.expect("notice");
        assert_eq!(notice.count, 2, "状态符号不计数");
        assert_eq!(notice.lines, vec![1]);
    }

    #[test]
    fn status_symbol_window_ranges_sorted_and_disjoint() {
        let mut prev_hi: Option<u32> = None;
        for &(lo, hi) in STATUS_SYMBOL_WINDOW {
            assert!(lo <= hi, "empty/descending range {lo:#X}..={hi:#X}");
            if let Some(prev) = prev_hi {
                assert!(lo > prev, "overlap at {lo:#X} (previous high {prev:#X})");
            }
            prev_hi = Some(hi);
        }
    }
}
