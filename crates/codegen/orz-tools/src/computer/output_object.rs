//! TER T1.11 (W-F13b, 2026-09-04)：持久化命令输出检索对象——
//! run_terminal_cmd 的长输出落盘后，按 `output_object_id`（落盘路径）提供
//! pattern / 行区间 / 尾部 N 行三类检索语义；模型侧无需 .gsa 摸黑补读。
//! 读取统一走固定解码链（GAP-ENCODING-GATE / OPS-PROTOCOL §8）。

use std::path::Path;

/// TER 全面审查 M1R-2 (2026-09-04)：单条命中行的最大渲染字符数——minified /
/// base64 / JSON dump 类超长单行（可 MB 级）命中时不整行回传撑爆注入预算；
/// 与 read_file 信封 preview（≤4K）同一量级。截断仍以完整行做匹配判定。
pub const MAX_OUTPUT_OBJECT_HIT_LINE_CHARS: usize = 4 * 1024;

/// 单条 pattern 命中（1-based 行号）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputObjectMatch {
    pub line: usize,
    pub text: String,
}

fn clamp_hit_line(line: &str) -> String {
    let count = line.chars().count();
    if count <= MAX_OUTPUT_OBJECT_HIT_LINE_CHARS {
        return line.to_string();
    }
    let cut: String = line
        .chars()
        .take(MAX_OUTPUT_OBJECT_HIT_LINE_CHARS)
        .collect();
    format!(
        "{cut}…(hit line truncated at {MAX_OUTPUT_OBJECT_HIT_LINE_CHARS} chars; \
             full line is {count} chars — read the object file for the full content)"
    )
}

fn read_lines(path: &Path) -> std::io::Result<Vec<String>> {
    let bytes = std::fs::read(path)?;
    let (text, _label) = crate::util::encoding::decode_text(&bytes);
    Ok(text.lines().map(str::to_owned).collect())
}

/// Pattern 检索（大小写不敏感包含匹配；`max_matches=0` = 不限量）。
pub fn search_output_object(
    path: &Path,
    pattern: &str,
    max_matches: usize,
) -> std::io::Result<Vec<OutputObjectMatch>> {
    let needle = pattern.to_lowercase();
    let mut hits = Vec::new();
    for (idx, line) in read_lines(path)?.into_iter().enumerate() {
        if line.to_lowercase().contains(&needle) {
            hits.push(OutputObjectMatch {
                line: idx + 1,
                text: clamp_hit_line(&line),
            });
            if max_matches > 0 && hits.len() >= max_matches {
                break;
            }
        }
    }
    Ok(hits)
}

/// 行区间检索（1-based 闭区间；越界机械 clamp 到文件实际范围）。
pub fn line_range_output_object(path: &Path, start: usize, end: usize) -> std::io::Result<String> {
    let lines = read_lines(path)?;
    let start = start.max(1);
    let end = end.min(lines.len()).max(start.saturating_sub(1));
    if start > lines.len() || start > end {
        return Ok(String::new());
    }
    Ok(lines[start - 1..end].join("\n"))
}

/// 尾部 N 行检索。
pub fn tail_output_object(path: &Path, n: usize) -> std::io::Result<String> {
    let lines = read_lines(path)?;
    if n == 0 {
        return Ok(String::new());
    }
    let take = n.min(lines.len());
    Ok(lines[lines.len() - take..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_lines(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
        f
    }

    #[test]
    fn pattern_search_is_case_insensitive_and_capped() {
        let f = write_lines(&[
            "build started",
            "Compiling cargo v1",
            "error[E0308]: mismatched types",
            "ERROR: build failed",
            "done",
        ]);
        let hits = search_output_object(f.path(), "ERROR", 0).unwrap();
        assert_eq!(hits.len(), 2, "{hits:?}");
        assert_eq!(hits[0].line, 3);
        assert_eq!(hits[1].line, 4);
        let capped = search_output_object(f.path(), "error", 1).unwrap();
        assert_eq!(capped.len(), 1);
        assert_eq!(capped[0].line, 3);
    }

    #[test]
    fn line_range_is_inclusive_and_clamped() {
        let f = write_lines(&["a", "b", "c", "d"]);
        assert_eq!(line_range_output_object(f.path(), 2, 3).unwrap(), "b\nc");
        assert_eq!(
            line_range_output_object(f.path(), 1, 99).unwrap(),
            "a\nb\nc\nd"
        );
        assert_eq!(line_range_output_object(f.path(), 0, 2).unwrap(), "a\nb");
        assert_eq!(line_range_output_object(f.path(), 9, 9).unwrap(), "");
    }

    #[test]
    fn tail_returns_last_n_lines() {
        let f = write_lines(&["a", "b", "c", "d", "e"]);
        assert_eq!(tail_output_object(f.path(), 2).unwrap(), "d\ne");
        assert_eq!(tail_output_object(f.path(), 0).unwrap(), "");
        assert_eq!(tail_output_object(f.path(), 99).unwrap(), "a\nb\nc\nd\ne");
    }

    #[test]
    fn missing_object_is_an_io_error() {
        assert!(search_output_object(Path::new("nope-missing.log"), "x", 0).is_err());
    }

    #[test]
    fn pattern_hit_long_line_is_capped_not_dropped() {
        let long = "x".repeat(MAX_OUTPUT_OBJECT_HIT_LINE_CHARS + 5000);
        let f = write_lines(&["short ok", &format!("needle-{long}"), "tail"]);
        let hits = search_output_object(f.path(), "needle-", 0).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(hits[0].text.contains("needle-"), "{:?}", hits[0].text);
        assert!(
            hits[0].text.chars().count() < MAX_OUTPUT_OBJECT_HIT_LINE_CHARS + 500,
            "hit line must be capped: {} chars",
            hits[0].text.chars().count()
        );
        assert!(hits[0].text.contains("truncated"), "{:?}", hits[0].text);
    }
}
