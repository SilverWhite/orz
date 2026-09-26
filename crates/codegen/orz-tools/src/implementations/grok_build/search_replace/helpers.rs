//! SearchReplace tool implementation.
//!
//! This tool performs exact string replacements in files with support for:
//! - Exact string replacement (find/replace)
//! - New file creation (when `old_string` is empty)
//! - Replace all mode (`replace_all: true`)
//! - Read-before-edit validation (non-concise mode)
//! - External modification detection

use crate::types::output::SearchReplaceEditDetail;

// ============================================================================
// Shared string-edit helpers
// ============================================================================

/// Render a snippet of the file with line numbers around the edit.
pub(crate) fn render_snippet(
    new_text: &str,
    new_string: &str,
    start_pos: usize,
    context_size: usize,
) -> (String, String, String) {
    let LineRange {
        start_line,
        end_line,
    } = compute_line_range(new_text, start_pos, new_string);
    let total_lines_count = new_text.split_inclusive('\n').count();
    let lines = new_text.split_inclusive('\n').collect::<Vec<_>>();

    let snippet_start = start_line.saturating_sub(context_size);
    let snippet_end = (end_line + context_size).min(total_lines_count.saturating_sub(1));

    let before_context = if snippet_start < start_line {
        lines[snippet_start..start_line].join("")
    } else {
        String::new()
    };

    let after_context = if end_line < snippet_end {
        lines[(end_line + 1)..=snippet_end].join("")
    } else {
        String::new()
    };

    let snippet = lines
        .iter()
        .enumerate()
        .map(|(line_num, line)| format!("{}→{}", line_num + 1, line))
        .skip(snippet_start)
        .take(snippet_end - snippet_start + 1)
        .collect::<String>();

    (snippet, before_context, after_context)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
/// 0-based inclusive start and end line indices
pub(crate) struct LineRange {
    pub start_line: usize,
    pub end_line: usize,
}

/// Compute the line range of the inserted text in the text
pub(crate) fn compute_line_range(text: &str, start_pos: usize, inserted_text: &str) -> LineRange {
    let start_line = text[..start_pos].matches('\n').count();
    let lines_in_inserted = inserted_text.split_inclusive('\n').count().max(1);
    let end_line = start_line + lines_in_inserted - 1;
    LineRange {
        start_line,
        end_line,
    }
}

/// f13（0bv，2026-09-26）：歧义失败回执的候选行号渲染——位置为 `text`
/// 中的字节偏移（LF 空间），输出 1 基行号列表（**去重**：同一行上的多处
/// 命中只报一次行号；至多 `CAP` 个行号，超出部分如实计数为 `+N more`，
/// 不虚构、不省略不报）。
pub(crate) fn format_candidate_lines(text: &str, positions: &[usize]) -> String {
    const CAP: usize = 10;
    let mut lines: Vec<usize> = Vec::new();
    for &pos in positions {
        let line = text[..pos.min(text.len())].matches('\n').count() + 1;
        if !lines.contains(&line) {
            lines.push(line);
        }
    }
    let mut out: Vec<String> = lines.iter().take(CAP).map(usize::to_string).collect();
    if lines.len() > CAP {
        out.push(format!("+{} more", lines.len() - CAP));
    }
    out.join(", ")
}

/// f14（0bv，2026-09-26）：替换后版式剧烈变化提示——两条判据与摩擦实证
/// 对偶：(a) 命中区域由多行折为单行（行内换行被吃掉；`old_lines >= 2`
/// 且 `new_lines == 1`）；(b) 单行替换后该行字符数至少翻倍且绝对值越过
/// `LENGTH_BLOWUP_MIN`。只报第一条命中（不喧哗；提示非判决，不改写行为）。
pub(crate) fn format_shift_notice(
    match_text: &str,
    replacements: &[(usize, usize, &str)],
    new_text: &str,
) -> Option<String> {
    const LENGTH_BLOWUP_MIN: usize = 200;
    for &(start, len, new_str) in replacements {
        let end = (start + len).min(match_text.len());
        let old_span = &match_text[start.min(end)..end];
        let old_lines = old_span.matches('\n').count() + 1;
        let new_lines = new_str.matches('\n').count() + 1;
        let old_line_len = line_char_len(match_text, start);
        let new_line_len = line_char_len(new_text, start);
        if old_lines >= 2 && new_lines == 1 {
            return Some(format!(
                "Formatting notice: this edit folded a multi-line match into a single line \
                 ({old_lines} lines -> 1 line; the affected line is now {new_line_len} chars, \
                 was {old_line_len}). Re-read the edited region to verify the formatting \
                 before continuing (especially when the anchor was a whole line)."
            ));
        }
        if old_lines == 1
            && new_lines == 1
            && new_line_len >= LENGTH_BLOWUP_MIN
            && new_line_len >= old_line_len.saturating_mul(2)
        {
            return Some(format!(
                "Formatting notice: this edit changed the affected line length sharply \
                 ({old_line_len} -> {new_line_len} chars; at least doubled). Re-read the \
                 edited region to verify the formatting before continuing."
            ));
        }
    }
    None
}

/// 所在行的字符数（`pos` 落在的行；行界按 `\n` 划分，与全工具口径一致）。
fn line_char_len(text: &str, pos: usize) -> usize {
    let pos = pos.min(text.len());
    let line_start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[pos..]
        .find('\n')
        .map(|i| pos + i)
        .unwrap_or(text.len());
    text[line_start..line_end].chars().count()
}

/// Replace text at specific positions and return new text with new positions.
pub(crate) fn replace_using_positions(
    text: &str,
    match_positions: &[usize],
    old_string: &str,
    new_string: &str,
) -> (String, Vec<usize>) {
    let mut new_text = String::new();
    let mut new_positions: Vec<usize> = Vec::with_capacity(match_positions.len());
    let mut last_end: usize = 0;

    for &pos in match_positions {
        new_text.push_str(&text[last_end..pos]);
        new_positions.push(new_text.len());
        new_text.push_str(new_string);
        last_end = pos + old_string.len();
    }

    new_text.push_str(&text[last_end..]);
    (new_text, new_positions)
}

// ============================================================================
// 行尾逐行保真（0bm ⑤，2026-09-25）
// ============================================================================

/// 行尾形态（CRLF 逐行行尾保真的唯一表征）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineEnding {
    Lf,
    Crlf,
}

impl LineEnding {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::Crlf => "\r\n",
        }
    }
}

/// 逐行收集原始文本的行尾：`ends[i]` = 第 i 行（0 基，按 `\n` 划分）的行尾；
/// `None` = 该行是**文件末行且无终结换行**。空文本返回空表。
///
/// 0bm ⑤（2026-09-25）：只认 `\n` 为行分隔（与全工具口径一致）——单独的
/// `\r`（无 `\n` 跟随时）视作行内容，不构成行尾。CRLF/LF 混排文件据此
/// **逐行**保真，替换不再把整文件统一成一种行尾。
pub(crate) fn line_endings(raw: &str) -> Vec<Option<LineEnding>> {
    let mut ends = Vec::new();
    for piece in raw.split_inclusive('\n') {
        match piece.strip_suffix('\n') {
            Some(body) => ends.push(Some(if body.ends_with('\r') {
                LineEnding::Crlf
            } else {
                LineEnding::Lf
            })),
            None => ends.push(None),
        }
    }
    ends
}

/// 文件主调行尾（首个有行尾的行）；全无行尾（空文件／单行无终结）⇒ LF。
pub(crate) fn prevailing_ending(ends: &[Option<LineEnding>]) -> LineEnding {
    ends.iter()
        .flatten()
        .next()
        .copied()
        .unwrap_or(LineEnding::Lf)
}

/// 追加一段「LF 空间」文本，其中的 `\n` 渲染为 `ending`；串内既有的
/// `\r\n` 先归一为 `\n`（插入内容以本文件局部行尾为准，避免 `\r\r\n`；
/// 单独 `\r` 属内容，原样保留）。
pub(crate) fn push_lf_text_rendered(out: &mut String, text: &str, ending: LineEnding) {
    let normalized;
    let text = if text.contains("\r\n") {
        normalized = text.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        text
    };
    for (index, piece) in text.split('\n').enumerate() {
        if index > 0 {
            out.push_str(ending.as_str());
        }
        out.push_str(piece);
    }
}

/// 把「LF 空间」的替换结果按原文件**逐行行尾**渲染写出（0bm ⑤ 单一实现点）。
///
/// - **未触碰的段**：逐行按其**原行尾**输出（`ends` 由 [`line_endings`] 收集）；
/// - **替换串内部换行**（及与后文的衔接换行）：继承**替换起点行**的原行尾；
///   起点行为文件末无终结行时依次回退 前一行 → 后一行 → 文件主调 → LF；
/// - `replacements` = `(起点字节偏移, 原区段字节长, 新串)`，按起点升序且不重叠
///   （与 [`replace_using_positions`]／[`replace_normalized_matches`] 的调用面
///   同构）；未触碰段的终结换行（含文件末行缺失终结换行）保持原样。
pub(crate) fn splice_with_line_endings(
    text: &str,
    replacements: &[(usize, usize, &str)],
    ends: &[Option<LineEnding>],
) -> String {
    let default = prevailing_ending(ends);
    let mut out = String::with_capacity(text.len() + 16);
    let mut cursor = 0usize;
    // `line` = 已消费的原文中 '\n' 的计数 = 当前写入位置所在的原行索引。
    let mut line = 0usize;
    let emit_unchanged = |from: usize, to: usize, line: &mut usize, out: &mut String| {
        for piece in text[from..to].split_inclusive('\n') {
            match piece.strip_suffix('\n') {
                Some(body) => {
                    out.push_str(body);
                    out.push_str(
                        ends.get(*line)
                            .copied()
                            .flatten()
                            .unwrap_or(default)
                            .as_str(),
                    );
                    *line += 1;
                }
                None => out.push_str(piece),
            }
        }
    };
    for &(pos, old_len, new_text) in replacements {
        emit_unchanged(cursor, pos, &mut line, &mut out);
        let start_line = line;
        let ending = ends
            .get(start_line)
            .copied()
            .flatten()
            .or_else(|| {
                start_line
                    .checked_sub(1)
                    .and_then(|index| ends.get(index).copied().flatten())
            })
            .or_else(|| ends.get(start_line + 1).copied().flatten())
            .unwrap_or(default);
        push_lf_text_rendered(&mut out, new_text, ending);
        let old_end = pos + old_len;
        line += text[pos..old_end].matches('\n').count();
        cursor = old_end;
    }
    emit_unchanged(cursor, text.len(), &mut line, &mut out);
    out
}

/// Build edit details for each replacement.
pub(crate) fn build_edit_details(
    new_text: &str,
    old_string: &str,
    new_string: &str,
    new_positions: &[usize],
    context_lines: usize,
) -> Vec<SearchReplaceEditDetail> {
    let mut details: Vec<SearchReplaceEditDetail> = Vec::with_capacity(new_positions.len());
    for &start_pos in new_positions {
        let (_snippet, context_before, context_after) =
            render_snippet(new_text, new_string, start_pos, context_lines);
        let line_range_new = compute_line_range(new_text, start_pos, new_string);
        // Extract the leading text on the line before the match starts.
        // This is the text between the last '\n' before start_pos and start_pos itself.
        let line_start = new_text[..start_pos]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let line_prefix = new_text[line_start..start_pos].to_owned();

        details.push(SearchReplaceEditDetail {
            old_string: old_string.to_owned(),
            old_line: line_range_new.start_line + 1,
            new_string: new_string.to_owned(),
            new_line: line_range_new.start_line + 1,
            context_before,
            context_after,
            line_prefix,
        });
    }
    details
}

// ============================================================================
// Normalized (confusable-aware) matching helpers
// ============================================================================

/// A single match found via confusable-normalized comparison, expressed in
/// the original text's byte coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedMatch {
    /// Byte offset in the original text where the match starts.
    pub original_start: usize,
    /// Length in bytes of the matched region in the original text.
    /// May differ from the search pattern's byte length because confusable
    /// characters have different UTF-8 widths than their ASCII equivalents.
    pub original_len: usize,
}

/// Result of normalized match search — distinguishes "no match exists" from
/// "matches exist but are unsafe to apply."
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NormalizedMatchResult {
    /// No normalized match exists in the text.
    NoMatch,
    /// One or more valid, non-overlapping matches were found.
    Matches(Vec<NormalizedMatch>),
    /// Normalized matching found candidates but they are ambiguous or unsafe
    /// (overlapping remapped spans, or partial-expansion matches that don't
    /// roundtrip correctly).  The caller should treat this as an explicit
    /// ambiguity error, NOT as "string not found."
    Ambiguous,
}

/// Find match positions using confusable-normalized comparison and remap
/// them back to the original text's byte coordinates.
///
/// Algorithm:
/// 1. Build `(normalized_text, offset_map)` via [`build_offset_map`].
/// 2. Normalize the search pattern the same way.
/// 3. Find all non-overlapping matches in `normalized_text`.
/// 4. Remap each normalized `[start..end]` span back to original bytes
///    via `offset_map`.
/// 5. **Roundtrip validation:** For each candidate, verify that
///    `normalize_confusables(&text[orig_start..orig_end]) == norm_pattern`.
///    This rejects partial-expansion matches (e.g., pattern `-` matching
///    inside em-dash `—` which normalizes to `--`).
/// 6. Reject overlapping validated spans (fail closed → `Ambiguous`).
///
/// Returns [`NormalizedMatchResult`] to distinguish no-match from ambiguity.
pub(crate) fn find_normalized_match_positions(text: &str, pattern: &str) -> NormalizedMatchResult {
    use crate::util::unicode_confusables::{build_offset_map, normalize_confusables};

    let (norm_text, offset_map) = build_offset_map(text);
    let norm_pattern = normalize_confusables(pattern);

    if norm_pattern.is_empty() {
        return NormalizedMatchResult::NoMatch;
    }

    // Collect all non-overlapping matches in normalized space and validate
    // each candidate via roundtrip check.
    let mut validated = Vec::new();
    let mut had_rejected_candidates = false;

    for (norm_start, _) in norm_text.match_indices(&norm_pattern) {
        let norm_end = norm_start + norm_pattern.len();
        let orig_start = offset_map[norm_start];
        let orig_end = offset_map[norm_end];

        // Reject zero-length or inverted spans.
        if orig_end <= orig_start {
            had_rejected_candidates = true;
            continue;
        }

        let orig_slice = &text[orig_start..orig_end];

        // Roundtrip validation: the normalized original slice must exactly
        // equal the normalized pattern.  This catches partial-expansion
        // matches (e.g., pattern "-" matching inside "—" → "--").
        if normalize_confusables(orig_slice) != norm_pattern {
            had_rejected_candidates = true;
            continue;
        }

        validated.push(NormalizedMatch {
            original_start: orig_start,
            original_len: orig_end - orig_start,
        });
    }

    if validated.is_empty() {
        // If we had candidates but all were rejected, that's ambiguity.
        // If we had zero candidates at all, that's a genuine no-match.
        return if had_rejected_candidates {
            NormalizedMatchResult::Ambiguous
        } else {
            NormalizedMatchResult::NoMatch
        };
    }

    // Reject overlapping remapped spans (fail closed on ambiguity).
    for window in validated.windows(2) {
        let end_of_prev = window[0].original_start + window[0].original_len;
        if end_of_prev > window[1].original_start {
            return NormalizedMatchResult::Ambiguous;
        }
    }

    NormalizedMatchResult::Matches(validated)
}

/// Replace text at normalized-match positions and return the new text with
/// new byte offsets of each replacement.
///
/// Each `NormalizedMatch` specifies a region in the original text (which may
/// contain Unicode confusables) to be replaced with `new_string`.
pub(crate) fn replace_normalized_matches(
    text: &str,
    matches: &[NormalizedMatch],
    new_string: &str,
) -> (String, Vec<usize>) {
    let mut result = String::new();
    let mut new_positions: Vec<usize> = Vec::with_capacity(matches.len());
    let mut last_end: usize = 0;

    for m in matches {
        result.push_str(&text[last_end..m.original_start]);
        new_positions.push(result.len());
        result.push_str(new_string);
        last_end = m.original_start + m.original_len;
    }

    result.push_str(&text[last_end..]);
    (result, new_positions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_snippet_middle_of_file_contexts() {
        let new_text = "one\ntwo NEW here\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n";
        let inserted = "NEW here";
        let start_pos = new_text.find(inserted).unwrap();

        let (snippet, before_context, after_context) =
            render_snippet(new_text, inserted, start_pos, 3);

        let expected_snippet = "1→one\n2→two NEW here\n3→three\n4→four\n5→five\n";
        assert_eq!(snippet, expected_snippet);
        assert_eq!(before_context, "one\n");
        assert_eq!(after_context, "three\nfour\nfive\n");

        let line_range = compute_line_range(new_text, start_pos, inserted);
        assert_eq!(
            line_range,
            LineRange {
                start_line: 1,
                end_line: 1
            }
        );
    }

    #[test]
    fn render_snippet_start_of_file_contexts() {
        let new_text = "AA1\nAA2\nb\nc\nd\ne\nf\n";
        let inserted = "AA1\nAA2\n";
        let start_pos = 0;

        let (snippet, before_context, after_context) =
            render_snippet(new_text, inserted, start_pos, 3);

        let expected_snippet = "1→AA1\n2→AA2\n3→b\n4→c\n5→d\n";
        assert_eq!(snippet, expected_snippet);
        assert_eq!(before_context, "");
        assert_eq!(after_context, "b\nc\nd\n");

        let line_range = compute_line_range(new_text, start_pos, inserted);
        assert_eq!(
            line_range,
            LineRange {
                start_line: 0,
                end_line: 1
            }
        );
    }

    #[test]
    fn render_snippet_end_of_file_contexts() {
        let new_text = "a\nb\nc\nd\ne\nNEW\n";
        let inserted = "NEW\n";
        let start_pos = new_text.find(inserted).unwrap();

        let (snippet, before_context, after_context) =
            render_snippet(new_text, inserted, start_pos, 3);

        let expected_snippet = "3→c\n4→d\n5→e\n6→NEW\n";
        assert_eq!(snippet, expected_snippet);
        assert_eq!(before_context, "c\nd\ne\n");
        assert_eq!(after_context, "");

        let line_range = compute_line_range(new_text, start_pos, inserted);
        assert_eq!(
            line_range,
            LineRange {
                start_line: 5,
                end_line: 5
            }
        );
    }

    #[test]
    fn render_snippet_multiline_insertion_middle() {
        let new_text = "1\n2\nAAA\nBBB\nCCC\n3\n4\n5\n6\n7\n";
        let inserted = "AAA\nBBB\nCCC\n";
        let start_pos = new_text.find(inserted).unwrap();

        let (snippet, before_context, after_context) =
            render_snippet(new_text, inserted, start_pos, 3);

        let expected_snippet = "1→1\n2→2\n3→AAA\n4→BBB\n5→CCC\n6→3\n7→4\n8→5\n";
        assert_eq!(snippet, expected_snippet);
        assert_eq!(before_context, "1\n2\n");
        assert_eq!(after_context, "3\n4\n5\n");

        let line_range = compute_line_range(new_text, start_pos, inserted);
        assert_eq!(
            line_range,
            LineRange {
                start_line: 2,
                end_line: 4
            }
        );
    }

    #[test]
    fn test_replace_using_positions() {
        let text = "hello world, hello again";
        let positions = vec![0, 13];
        let (new_text, new_positions) = replace_using_positions(text, &positions, "hello", "hi");
        assert_eq!(new_text, "hi world, hi again");
        assert_eq!(new_positions, vec![0, 10]);
    }

    // ── Normalized matching helpers ─────────────────────────────────────

    fn unwrap_matches(result: NormalizedMatchResult) -> Vec<NormalizedMatch> {
        match result {
            NormalizedMatchResult::Matches(m) => m,
            other => panic!("Expected Matches, got {:?}", other),
        }
    }

    #[test]
    fn normalized_match_smart_quotes() {
        let text = "say \u{201C}hello\u{201D} world";
        let matches = unwrap_matches(find_normalized_match_positions(text, "\"hello\""));
        assert_eq!(matches.len(), 1);
        let m = &matches[0];
        assert_eq!(
            &text[m.original_start..m.original_start + m.original_len],
            "\u{201C}hello\u{201D}"
        );
    }

    #[test]
    fn normalized_match_em_dash() {
        let text = "foo\u{2014}bar";
        let matches = unwrap_matches(find_normalized_match_positions(text, "foo--bar"));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn normalized_match_nbsp() {
        let text = "hello\u{00A0}world";
        let matches = unwrap_matches(find_normalized_match_positions(text, "hello world"));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn normalized_match_ellipsis() {
        let text = "wait\u{2026}";
        let matches = unwrap_matches(find_normalized_match_positions(text, "wait..."));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn normalized_match_returns_no_match_for_unrelated() {
        assert_eq!(
            find_normalized_match_positions("hello world", "xyz"),
            NormalizedMatchResult::NoMatch
        );
    }

    #[test]
    fn normalized_match_pure_ascii_still_works() {
        let matches = unwrap_matches(find_normalized_match_positions("hello world", "hello"));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].original_start, 0);
        assert_eq!(matches[0].original_len, 5);
    }

    #[test]
    fn normalized_match_multiple_occurrences() {
        let text = "\u{201C}a\u{201D} and \u{201C}b\u{201D}";
        let matches = unwrap_matches(find_normalized_match_positions(text, "\"a\""));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn normalized_match_empty_pattern_returns_no_match() {
        assert_eq!(
            find_normalized_match_positions("hello", ""),
            NormalizedMatchResult::NoMatch
        );
    }

    // ── Partial-expansion rejection ─────────────────────────────────────

    #[test]
    fn partial_expansion_dash_inside_em_dash_rejected() {
        let text = "\u{2014}";
        assert_eq!(
            find_normalized_match_positions(text, "-"),
            NormalizedMatchResult::Ambiguous,
            "partial match inside em-dash must be Ambiguous"
        );
    }

    #[test]
    fn partial_expansion_dot_inside_ellipsis_rejected() {
        let text = "\u{2026}";
        assert_eq!(
            find_normalized_match_positions(text, "."),
            NormalizedMatchResult::Ambiguous,
            "partial match inside ellipsis must be Ambiguous"
        );
    }

    #[test]
    fn partial_expansion_double_dot_inside_ellipsis_rejected() {
        let text = "\u{2026}";
        assert_eq!(
            find_normalized_match_positions(text, ".."),
            NormalizedMatchResult::Ambiguous,
        );
    }

    #[test]
    fn full_expansion_em_dash_accepted() {
        let text = "a\u{2014}b";
        let matches = unwrap_matches(find_normalized_match_positions(text, "--"));
        assert_eq!(matches.len(), 1);
        assert_eq!(
            &text[matches[0].original_start..matches[0].original_start + matches[0].original_len],
            "\u{2014}"
        );
    }

    #[test]
    fn full_expansion_ellipsis_accepted() {
        let text = "a\u{2026}b";
        let matches = unwrap_matches(find_normalized_match_positions(text, "..."));
        assert_eq!(matches.len(), 1);
    }

    // ── Replace with new return type ────────────────────────────────────

    #[test]
    fn replace_normalized_matches_basic() {
        let text = "say \u{201C}hello\u{201D} world";
        let matches = unwrap_matches(find_normalized_match_positions(text, "\"hello\""));
        let (new_text, new_positions) = replace_normalized_matches(text, &matches, "\"goodbye\"");
        assert_eq!(new_text, "say \"goodbye\" world");
        assert_eq!(new_positions.len(), 1);
    }

    #[test]
    fn replace_normalized_matches_preserves_surrounding() {
        let text = "before \u{201C}target\u{201D} after";
        let matches = unwrap_matches(find_normalized_match_positions(text, "\"target\""));
        let (new_text, _) = replace_normalized_matches(text, &matches, "\"replaced\"");
        assert!(new_text.starts_with("before "));
        assert!(new_text.ends_with(" after"));
        assert!(new_text.contains("\"replaced\""));
    }

    #[test]
    fn replace_normalized_matches_at_end_of_string() {
        let text = "prefix\u{2026}";
        let matches = unwrap_matches(find_normalized_match_positions(text, "prefix..."));
        let (new_text, _) = replace_normalized_matches(text, &matches, "done");
        assert_eq!(new_text, "done");
    }

    #[test]
    fn replace_normalized_result_is_valid_utf8() {
        let text = "a\u{201C}b\u{2014}c\u{00A0}d";
        let matches = unwrap_matches(find_normalized_match_positions(text, "\"b--c d"));
        let (new_text, _) = replace_normalized_matches(text, &matches, "replaced");
        assert_eq!(new_text, "areplaced");
    }
}
