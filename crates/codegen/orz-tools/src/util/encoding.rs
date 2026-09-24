//! Mechanical text-encoding adaptation (GAP-ENCODING-GATE, OPS-PROTOCOL §8).
//!
//! The contract is encoding-agnostic at the model boundary: every
//! cross-process / cross-environment text surface decodes through the fixed
//! chain — strip BOM → UTF-8 strict → GB18030 → UTF-8 lossy — and records
//! which stage produced the text (`output_encoding`). Writes are always
//! UTF-8 without BOM. Ladder structure and label shapes mirror
//! `assurance/ops_executor.py::decode_text`. A 2026-09-19 full-space
//! differential (1-byte + 2-byte + all 1,587,600 four-byte forms) pinned the
//! GB18030 table boundary between encoding_rs (WHATWG / GB18030-2005+) and
//! the reference's CPython codec (GB18030-2000 PUA mappings): 20 two-byte
//! pairs and one four-byte slot decode to different code points, over-range
//! four-byte forms replace with two U+FFFD there vs one here (which can flip
//! the lossy minimal-replacement choice), and cleanliness agrees everywhere
//! except the bare euro byte 0x80 — the one known label fork. Both sides pin
//! these boundaries: `gb18030_table_boundaries_vs_python_reference_pinned`
//! and `assurance/tests/test_ops_executor_decode_text.py`.
//!
//! 0as (2026-09-19) refines the final lossy stage only: the first three
//! stages keep their whole-buffer order and hit semantics. The lossy stage
//! decodes per line with a minimal-replacement choice (UTF-8 walk with
//! explicit byte placeholders vs GB18030 replacement) and the label carries
//! the degraded fraction — `utf-8-lossy:<p>%`. See [`decode_text`].

use std::io;
use std::path::Path;

/// Extensions whose content is expected to be human-readable text.
///
/// FUS-HOST-RESOURCE-SAFETY §4.4 (2026-09-12): only these extensions take the
/// decode-first gate — a text-family file must not be rejected as "binary"
/// merely because it carries NUL bytes (PowerShell 5's `>` writes UTF-16LE).
/// Everything else keeps the historical order (`is_binary` first), so the
/// `BINARY_EXTENSIONS` contract is untouched for non-text families.
pub const TEXT_FAMILY_EXTENSIONS: &[&str] = &[
    "bat",
    "c",
    "cfg",
    "cmd",
    "conf",
    "cpp",
    "cs",
    "csv",
    "diff",
    "env",
    "err",
    "go",
    "gradle",
    "h",
    "hpp",
    "htm",
    "html",
    "ini",
    "java",
    "js",
    "json",
    "jsonl",
    "jsx",
    "lock",
    "log",
    "md",
    "out",
    "patch",
    "php",
    "properties",
    "ps1",
    "py",
    "rb",
    "rs",
    "rst",
    "sh",
    "sql",
    "toml",
    "ts",
    "tsv",
    "tsx",
    "txt",
    "xml",
    "yaml",
    "yml",
];

/// `true` when `extension` (lowercased, without dot) belongs to the text
/// family above.
pub fn is_text_family_extension(extension: &str) -> bool {
    let ext = extension.trim_start_matches('.').to_ascii_lowercase();
    TEXT_FAMILY_EXTENSIONS.binary_search(&ext.as_str()).is_ok()
}

/// Decode raw bytes with the fixed chain:
/// strip BOM → UTF-8 strict → GB18030 → UTF-8 lossy.
///
/// Returns `(text, encoding_label)`. Labels:
/// - `utf-8-sig`: UTF-8 with a leading BOM (BOM stripped);
/// - `utf-8`: valid UTF-8 without BOM;
/// - `gb18030`: not UTF-8, but a valid GB18030 sequence;
/// - `utf-8-lossy:<p>%`: neither UTF-8 nor valid GB18030.
///
/// The lossy stage (0as, 2026-09-19) decodes **per line** so a single bad
/// byte no longer degrades the whole block: each line walks the same ladder
/// (valid UTF-8 kept as-is → clean GB18030 kept → minimal-replacement lossy).
/// For a line failing both strict decodes, the UTF-8 lossy walk (invalid
/// subsequences rendered as explicit placeholders, [`utf8_lossy_placeholders`])
/// and the GB18030 replacement decode are compared and the one with fewer
/// degraded units wins; ties keep the ladder order (UTF-8). Degraded units =
/// placeholders + U+FFFD; the label suffix `<p>` = units ÷ post-BOM-strip
/// input bytes × 100, two decimals. Readable text is byte-identical to the
/// input wherever a strict stage can decode it, and the unit count is never
/// higher than the previous whole-buffer `from_utf8_lossy` baseline.
pub fn decode_text(data: &[u8]) -> (String, String) {
    let mut bytes = data;
    let mut label = "utf-8";
    if let Some(stripped) = bytes.strip_prefix(b"\xef\xbb\xbf") {
        bytes = stripped;
        label = "utf-8-sig";
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (text.to_string(), label.to_string());
    }
    // encoding_rs's GB18030 decoder is total; `had_errors` marks byte
    // sequences the reference Python decoder rejects (truncated / illegal
    // four-byte forms). Only a clean decode counts as "gb18030" — anything
    // else falls through to the lossy stage, mirroring `decode_text` in
    // assurance/ops_executor.py.
    let (text, had_errors) = encoding_rs::GB18030.decode_without_bom_handling(bytes);
    if !had_errors {
        return (text.into_owned(), "gb18030".to_string());
    }
    let (text, units) = decode_lossy_segmented(bytes);
    let ratio = units as f64 / bytes.len().max(1) as f64 * 100.0;
    (text, format!("utf-8-lossy:{ratio:.2}%"))
}

/// Refined lossy stage (0as): per-line decode so one bad byte cannot degrade
/// the whole block. Line granularity is safe because neither UTF-8 nor
/// GB18030 encodes `0x0A` as part of a multi-byte sequence, so no multi-byte
/// character can span the split. Returns `(text, degraded_units)`.
fn decode_lossy_segmented(bytes: &[u8]) -> (String, usize) {
    let mut out = String::with_capacity(bytes.len() + 16);
    let mut units = 0usize;
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        let (body, newline) = match line.last() {
            Some(&b'\n') => (&line[..line.len() - 1], "\n"),
            _ => (line, ""),
        };
        let (text, line_units) = decode_lossy_line(body);
        out.push_str(&text);
        out.push_str(newline);
        units += line_units;
    }
    (out, units)
}

/// The fixed ladder applied to one line of the lossy stage: valid UTF-8 kept
/// as-is → clean GB18030 kept → minimal-replacement lossy choice (ties keep
/// the ladder order, UTF-8). Returns `(text, degraded_units)`.
fn decode_lossy_line(line: &[u8]) -> (String, usize) {
    if let Ok(text) = std::str::from_utf8(line) {
        return (text.to_string(), 0);
    }
    let (gb_text, had_errors) = encoding_rs::GB18030.decode_without_bom_handling(line);
    if !had_errors {
        return (gb_text.into_owned(), 0);
    }
    let (utf8_text, utf8_units) = utf8_lossy_placeholders(line);
    let gb_text = gb_text.into_owned();
    let gb_units = gb_text.matches('\u{fffd}').count();
    if gb_units < utf8_units {
        (gb_text, gb_units)
    } else {
        (utf8_text, utf8_units)
    }
}

/// UTF-8 lossy decode with explicit byte placeholders (0as).
///
/// Same granularity as `String::from_utf8_lossy` (each error spans one
/// maximal subpart, so unit counts match the previous baseline 1:1), but an
/// invalid subsequence renders as `⟨0x8F⟩` (single byte) or `⟨0xE4 0xB8⟩`
/// (multi-byte run, upper-case hex, space-separated) instead of U+FFFD — the
/// model keeps the byte-level fact instead of an uninformative run of
/// replacement characters. Returns `(text, units)`.
fn utf8_lossy_placeholders(bytes: &[u8]) -> (String, usize) {
    let mut out = String::with_capacity(bytes.len() + 16);
    let mut rest = bytes;
    let mut units = 0usize;
    loop {
        match std::str::from_utf8(rest) {
            Ok(text) => {
                out.push_str(text);
                return (out, units);
            }
            Err(error) => {
                let valid_up_to = error.valid_up_to();
                // `valid_up_to` is guaranteed valid by the Utf8Error contract.
                out.push_str(std::str::from_utf8(&rest[..valid_up_to]).unwrap_or(""));
                let start = valid_up_to;
                let len = error.error_len().unwrap_or(rest.len() - start).max(1);
                out.push('\u{27e8}');
                for (index, byte) in rest[start..start + len].iter().enumerate() {
                    if index > 0 {
                        out.push(' ');
                    }
                    out.push_str(&format!("0x{byte:02X}"));
                }
                out.push('\u{27e9}');
                units += 1;
                rest = &rest[start + len..];
            }
        }
    }
}

/// Decode-first gate for text-family files (FUS-HOST-RESOURCE-SAFETY §4.4).
///
/// The gate exists for exactly one class of file: **text the framework is
/// currently wrong about** — wide/legacy-encoded text whose bytes carry NULs,
/// which the extension-less `is_binary` heuristic reads as "binary". Anything
/// the gate declines falls back to the historical path (binary judgement, then
/// [`decode_text`]), so nothing that used to work stops working.
///
/// Chain (2026-09-12 实施注记在 §4.4 基础上收窄，见审计记录):
///
/// 1. UTF-16LE/BE BOM → strict UTF-16 (no U+0000 in the result);
/// 2. UTF-8 BOM / strict UTF-8, **only when the text carries no NUL** (NUL is
///    legal UTF-8 but is never text — it is the signature of a wide encoding or
///    of a blob);
/// 3. no-BOM NUL-alternation heuristic (ASCII-in-UTF-16), strict decode;
/// 4. otherwise `None`.
///
/// The GB18030 stage of the design's chain stays where it already is — inside
/// [`decode_text`], *after* the binary judgement. Pulling it in front of that
/// judgement would make a PNG sitting in a `.txt` decode into mojibake instead
/// of being rejected: the gate would weaken the very check it exists to
/// complement. NUL-free GB18030 text therefore still reads back as
/// `gb18030`, through the unchanged historical path.
///
/// Returns `(text, encoding_label)`. New labels beyond [`decode_text`]:
/// `utf-16le` / `utf-16be`. The label lands on the existing
/// `tool_completed.output_encoding` field — no new field is introduced
/// (design §11 裁决 9).
pub fn sniff_text_bytes(data: &[u8]) -> Option<(String, &'static str)> {
    // UTF-32 BOMs must not be mistaken for UTF-16 (they share the first two
    // bytes) — out of scope for this gate, so decline instead of half-decoding.
    if data.starts_with(&[0xff, 0xfe, 0x00, 0x00]) || data.starts_with(&[0x00, 0x00, 0xfe, 0xff]) {
        return None;
    }
    if let Some(rest) = data.strip_prefix(&[0xff, 0xfe]) {
        return decode_utf16(rest, true, "utf-16le");
    }
    if let Some(rest) = data.strip_prefix(&[0xfe, 0xff]) {
        return decode_utf16(rest, false, "utf-16be");
    }
    if let Some(stripped) = data.strip_prefix(b"\xef\xbb\xbf") {
        return std::str::from_utf8(stripped)
            .ok()
            .filter(|text| !text.contains('\0'))
            .map(|text| (text.to_string(), "utf-8-sig"));
    }
    if let Ok(text) = std::str::from_utf8(data) {
        if !text.contains('\0') {
            return Some((text.to_string(), "utf-8"));
        }
    }
    if let Some(little_endian) = utf16_no_bom_endianness(data) {
        let label = if little_endian {
            "utf-16le"
        } else {
            "utf-16be"
        };
        if let Some(decoded) = decode_utf16(data, little_endian, label) {
            return Some(decoded);
        }
    }
    None
}

/// Strict UTF-16 decode: unpaired surrogates or an embedded U+0000 (the
/// signature of a mis-detected binary blob) both fail the gate.
fn decode_utf16(
    bytes: &[u8],
    little_endian: bool,
    label: &'static str,
) -> Option<(String, &'static str)> {
    if bytes.len() % 2 != 0 || bytes.is_empty() {
        return None;
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();
    let text = String::from_utf16(&units).ok()?;
    if text.contains('\0') {
        return None;
    }
    Some((text, label))
}

/// No-BOM UTF-16 detection: ASCII-range text encoded as UTF-16 leaves every
/// other byte at 0x00. Requires a strong asymmetry (≥60% NUL on one parity,
/// ≤5% on the other) so ordinary binary data does not qualify.
fn utf16_no_bom_endianness(bytes: &[u8]) -> Option<bool> {
    const SAMPLE: usize = 8192;
    let sample = &bytes[..bytes.len().min(SAMPLE)];
    let units = sample.len() / 2;
    if units < 2 {
        return None;
    }
    let mut even_zeros = 0usize;
    let mut odd_zeros = 0usize;
    for (index, byte) in sample.iter().take(units * 2).enumerate() {
        if *byte == 0x00 {
            if index % 2 == 0 {
                even_zeros += 1;
            } else {
                odd_zeros += 1;
            }
        }
    }
    let units_f = units as f64;
    let even = even_zeros as f64 / units_f;
    let odd = odd_zeros as f64 / units_f;
    if odd >= 0.6 && even <= 0.05 {
        Some(true)
    } else if even >= 0.6 && odd <= 0.05 {
        Some(false)
    } else {
        None
    }
}

/// Encode text as UTF-8 without a BOM (the write side of the contract).
///
/// Rust strings are already Unicode; this function exists to pin the
/// contract (no BOM, no system code page) at one call site and to make the
/// no-BOM property testable.
pub fn encode_text_no_bom(text: &str) -> Vec<u8> {
    debug_assert!(!text.starts_with('\u{feff}'), "text must not carry a BOM");
    text.as_bytes().to_vec()
}

/// Write `text` to `path` as UTF-8 without a BOM.
pub fn write_text_utf8_no_bom(path: &Path, text: &str) -> io::Result<()> {
    std::fs::write(path, encode_text_no_bom(text))
}

/// 0bi ①（2026-09-23）：解码标签 → 原文件是否带 UTF-8 BOM。
/// `decode_text` 只在输入以 `EF BB BF` 开头时回 `utf-8-sig`；其余标签
/// （`utf-8` / `gb18030` / `utf-8-lossy:<p>%`）都是无 BOM 形态。
pub fn label_had_bom(label: &str) -> bool {
    label == "utf-8-sig"
}

/// 0bi ①（2026-09-23）：写侧 BOM 保真的**单一编码点**——编辑／写入工具
/// 写回时按原文件形态决定是否带 UTF-8 BOM（`had_bom` 来自
/// [`label_had_bom`]）。
///
/// 背景（0bh §5 #1 实锤）：`scripts/build_orz.ps1` 原带 `EF BB BF`，经编辑
/// 工具写回后 BOM 丢失，PS 5.1 按本地代码页（GBK）误读中文注释与字符串，
/// 报「数组索引表达式丢失」类解析错。保持 BOM＝「写回与读入同形」的最小
/// 修复；原无 BOM 的文件维持无 BOM 契约（[`encode_text_no_bom`]）。
pub fn encode_text_preserving_bom(text: &str, had_bom: bool) -> Vec<u8> {
    if !had_bom {
        return text.as_bytes().to_vec();
    }
    let mut bytes = Vec::with_capacity(text.len() + 3);
    bytes.extend_from_slice(b"\xef\xbb\xbf");
    bytes.extend_from_slice(text.as_bytes());
    bytes
}

/// 0bl 审查修复（2026-09-24；0bm 复审补口）：编辑面 UTF-16 fail-closed 门
/// （原为 `search_replace` 私有件；hashline 编辑面接入同一单点后上移 `util`）。
/// 读面 `sniff_text_bytes` 能识别 UTF-16，但编辑面 `decode_text` 不识别——
/// UTF-16 文件可能被 GB18030 分支"干净"解码后以 UTF-8 写回（静默乱码）。
/// 字节呈 UTF-16 形态即判真：`FF FE` / `FE FF` 开头（UTF-32LE/BE BOM 的
/// 前两字节亦然，同样按 UTF-16 形态拒），或前 8KB 内 NUL 字节占比 > 30%
/// （宽编码签名；普通 UTF-8/GB18030 文本几乎不含 NUL）。UTF-8 BOM
/// （`EF BB BF`）路径不受影响。编辑面的**新建文件路径不设此门**：old 内容
/// 整体废弃、不流入写回，无静默乱码风险。残余边界（如实记录）：**无 BOM
/// 且 ASCII 占比低**的 UTF-16（如纯 CJK 内容的 UTF-16LE，码点双字节大多
/// 非 NUL）可逃过本启发式——Windows 生态 UTF-16 几乎必带 BOM，实际暴露面
/// 小；属 fail-closed 门的固有近似，不做二次启发。
pub fn utf16_shaped_input(bytes: &[u8]) -> bool {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        return true;
    }
    let sample = &bytes[..bytes.len().min(8192)];
    if sample.is_empty() {
        return false;
    }
    let nul_count = sample.iter().filter(|&&b| b == 0).count();
    nul_count * 100 > sample.len() * 30
}

/// UTF-16 形态文件的统一拒绝文案（编辑族共用）：错误信息明确说明文件呈
/// UTF-16 形态、编辑面不支持、请先转换为 UTF-8。
pub fn utf16_rejection_message(file_path: &str) -> String {
    format!(
        "Error: {} appears to be UTF-16 encoded (BOM or NUL-byte pattern detected). \
         The edit face does not support UTF-16 files and refuses to write them back \
         (doing so would silently mojibake the content). Please convert the file to \
         UTF-8 first, then retry the edit.",
        file_path
    )
}

/// Merge encoding labels observed on different chunks of the same stream
/// (truncated output keeps front/back slices, and a process has separate
/// stdout/stderr buffers). Deduplicates while preserving order — the same
/// rule as `assurance/ops_executor.py` (`,`.join(dict.fromkeys(encs))`).
pub fn merge_encoding_labels<'a>(labels: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let mut seen: Vec<&'a str> = Vec::new();
    for label in labels {
        if !label.is_empty() && !seen.contains(&label) {
            seen.push(label);
        }
    }
    if seen.is_empty() {
        None
    } else {
        Some(seen.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_without_bom() {
        let (text, label) = decode_text("hello 世界".as_bytes());
        assert_eq!(text, "hello 世界");
        assert_eq!(label, "utf-8");
    }

    #[test]
    fn utf8_bom_is_stripped_and_labeled() {
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        bytes.extend_from_slice("hello".as_bytes());
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "hello");
        assert_eq!(label, "utf-8-sig");
    }

    #[test]
    fn gb18030_is_decoded() {
        // "中文" in GB18030 (and GBK).
        let bytes = [0xd6, 0xd0, 0xce, 0xc4];
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "中文");
        assert_eq!(label, "gb18030");
    }

    #[test]
    fn invalid_bytes_fall_through_to_lossy() {
        let bytes = [0xff, 0xfe, 0x80, 0x81];
        let (text, label) = decode_text(&bytes);
        assert!(label.starts_with("utf-8-lossy:") && label.ends_with('%'));
        // The refined stage must never degrade more than the old whole-buffer
        // baseline, and the label ratio must be recomputable from the text.
        let baseline = String::from_utf8_lossy(&bytes);
        let units = text.matches('\u{fffd}').count() + text.matches('\u{27e8}').count();
        assert!(units <= baseline.matches('\u{fffd}').count());
        let pct: f64 = label["utf-8-lossy:".len()..label.len() - 1]
            .parse()
            .unwrap();
        let expected = units as f64 / bytes.len() as f64 * 100.0;
        assert!((pct - expected).abs() < 0.005, "{label} vs {expected}");
    }

    #[test]
    fn invalid_gb18030_falls_through_to_lossy() {
        // 0x81 0x30 is an illegal GB18030 sequence (second byte below 0x40;
        // also the prefix of a truncated four-byte form) — Python's
        // `decode("gb18030")` raises here, so the chain must NOT claim
        // "gb18030". The whole three bytes form one truncated 4-byte GB18030
        // sequence, so the GB18030 replacement decode wins the minimal-choice
        // comparison with a single unit (the UTF-8 walk would spend two).
        let bytes = [0x81, 0x30, 0x81];
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "\u{fffd}");
        assert_eq!(label, "utf-8-lossy:33.33%");
    }

    #[test]
    fn empty_input_is_utf8() {
        let (text, label) = decode_text(b"");
        assert_eq!(text, "");
        assert_eq!(label, "utf-8");
    }

    #[test]
    fn encode_is_utf8_without_bom() {
        let bytes = encode_text_no_bom("abc");
        assert_eq!(bytes, b"abc");
        assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    }

    #[test]
    fn write_round_trip_has_no_bom() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.txt");
        write_text_utf8_no_bom(&path, "data").unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes, b"data");
    }

    #[test]
    fn merge_labels_deduplicates_in_order() {
        assert_eq!(
            merge_encoding_labels(["utf-8", "gb18030", "utf-8"]).as_deref(),
            Some("utf-8,gb18030")
        );
        assert_eq!(merge_encoding_labels([""]), None);
        assert_eq!(merge_encoding_labels([]), None);
        // 0as: the refined lossy label merges like any other stage label.
        assert_eq!(
            merge_encoding_labels(["utf-8", "utf-8-lossy:1.23%"]).as_deref(),
            Some("utf-8,utf-8-lossy:1.23%")
        );
    }

    /// 判据① (0as): mixed sample — a valid-UTF-8 line, an isolated illegal
    /// byte, and a GB18030 line. Readable text survives byte-identical; the
    /// old whole-buffer lossy stage turned the GB18030 body into runs of
    /// U+FFFD; the refined stage keeps it readable.
    #[test]
    fn lossy_mixed_sample_keeps_readable_parts_intact() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice("readable 保持可读\n".as_bytes());
        bytes.push(0x81);
        bytes.push(b'\n');
        bytes.extend_from_slice(&[0xd6, 0xd0, 0xce, 0xc4]);
        bytes.extend_from_slice(b" tail\n");
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "readable 保持可读\n⟨0x81⟩\n中文 tail\n");
        // 1 degraded unit over 34 post-BOM-strip input bytes.
        assert_eq!(label, "utf-8-lossy:2.94%");
    }

    /// 判据① headline case (0as): GB18030 text with one hopeless byte — the
    /// GB18030 body stays readable (strictly fewer replacements than the
    /// UTF-8 walk, which would shred every GB pair).
    #[test]
    fn lossy_prefers_gb18030_when_it_has_fewer_replacements() {
        let bytes = [0xd6, 0xd0, 0xff];
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "中\u{fffd}");
        assert_eq!(label, "utf-8-lossy:33.33%");
    }

    /// Tie (equal degraded units) keeps the ladder order: the UTF-8 walk with
    /// explicit byte placeholders wins over the opaque GB18030 replacement.
    #[test]
    fn lossy_tie_keeps_ladder_order_utf8() {
        // [d6 d0 8f]: GB18030 = 中 + truncated lead (1 FFFD); UTF-8 walk =
        // ⟨0xD6⟩ + the valid pair [d0 8f] = U+040F (1 unit).
        let bytes = [0xd6, 0xd0, 0x8f];
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "⟨0xD6⟩\u{040f}");
        assert_eq!(label, "utf-8-lossy:33.33%");
    }

    /// A multi-byte invalid UTF-8 subsequence renders as one placeholder
    /// listing its bytes (tie with the GB18030 replacement count → UTF-8).
    #[test]
    fn lossy_multibyte_subsequence_placeholder_form() {
        // [f0 9e 81]: a truncated 4-byte sequence; GB18030 decodes (f0,9e)
        // cleanly then errors on the trailing lead — 1 unit vs 1 unit tie.
        let bytes = [0xf0, 0x9e, 0x81];
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "⟨0xF0 0x9E 0x81⟩");
        assert_eq!(label, "utf-8-lossy:33.33%");
    }

    /// 判据③ (0as): the BOM is stripped before the ratio denominator, so the
    /// degraded fraction is measured on the actual decode input.
    #[test]
    fn lossy_label_ratio_excludes_stripped_bom() {
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        bytes.push(0x81);
        let (text, label) = decode_text(&bytes);
        assert_eq!(text, "⟨0x81⟩");
        assert_eq!(label, "utf-8-lossy:100.00%");
    }

    #[test]
    fn text_family_lookup_normalizes_case_and_dot() {
        assert!(is_text_family_extension("txt"));
        assert!(is_text_family_extension(".LOG"));
        assert!(is_text_family_extension("ps1"));
        assert!(!is_text_family_extension("exe"));
        assert!(!is_text_family_extension(""));
        assert!(
            TEXT_FAMILY_EXTENSIONS.windows(2).all(|w| w[0] < w[1]),
            "TEXT_FAMILY_EXTENSIONS must stay sorted for the binary search"
        );
    }

    fn utf16le_bytes(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<u8>>()
    }

    fn utf16be_bytes(text: &str) -> Vec<u8> {
        text.encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<u8>>()
    }

    #[test]
    fn sniff_decodes_utf16le_with_bom() {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend_from_slice(&utf16le_bytes("命令输出的中文\r\n"));
        let (text, label) = sniff_text_bytes(&bytes).expect("utf-16le BOM must decode");
        assert_eq!(label, "utf-16le");
        assert_eq!(text, "命令输出的中文\r\n");
    }

    #[test]
    fn sniff_decodes_utf16be_with_bom() {
        let mut bytes = vec![0xfe, 0xff];
        bytes.extend_from_slice(&utf16be_bytes("hi"));
        let (text, label) = sniff_text_bytes(&bytes).expect("utf-16be BOM must decode");
        assert_eq!(label, "utf-16be");
        assert_eq!(text, "hi");
    }

    #[test]
    fn sniff_decodes_bom_less_utf16le_ascii() {
        // What PowerShell 5 `>` produces for pure-ASCII output.
        let bytes = utf16le_bytes("ERROR: build failed\r\n");
        let (text, label) = sniff_text_bytes(&bytes).expect("no-BOM UTF-16LE must decode");
        assert_eq!(label, "utf-16le");
        assert_eq!(text, "ERROR: build failed\r\n");
    }

    #[test]
    fn sniff_decodes_bom_less_utf16be() {
        let bytes = utf16be_bytes("hello world, this is a log line\n");
        let (text, label) = sniff_text_bytes(&bytes).expect("no-BOM UTF-16BE must decode");
        assert_eq!(label, "utf-16be");
        assert_eq!(text, "hello world, this is a log line\n");
    }

    #[test]
    fn sniff_rejects_binary_blobs() {
        // PNG magic + NULs: no BOM, no consistent parity, not UTF-8, not clean
        // GB18030 → the caller must keep the historical rejection.
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&[0x00, 0x00, 0x00, 0x0d]);
        png.extend_from_slice(&[0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x01]);
        assert_eq!(sniff_text_bytes(&png), None);
    }

    #[test]
    fn sniff_declines_utf32_boms() {
        let mut bytes = vec![0xff, 0xfe, 0x00, 0x00];
        bytes.extend_from_slice(&[0x41, 0x00, 0x00, 0x00]);
        assert_eq!(sniff_text_bytes(&bytes), None);
    }

    #[test]
    fn sniff_keeps_the_existing_utf8_labels() {
        assert_eq!(
            sniff_text_bytes(b"plain ascii").map(|(_, l)| l),
            Some("utf-8")
        );
        assert_eq!(
            sniff_text_bytes(b"\xef\xbb\xbfbom").map(|(_, l)| l),
            Some("utf-8-sig")
        );
    }

    /// GB18030 stays on the historical path (`decode_text` after the binary
    /// judgement): the gate must not admit it, or a blob in a text extension
    /// would decode into mojibake instead of being rejected.
    #[test]
    fn sniff_declines_gb18030_to_the_historical_path() {
        assert_eq!(sniff_text_bytes(&[0xd6, 0xd0, 0xce, 0xc4]), None);
        // NUL-free GB18030 still decodes through the documented chain.
        assert_eq!(decode_text(&[0xd6, 0xd0, 0xce, 0xc4]).1, "gb18030");
    }

    /// NUL bytes are never text: a "UTF-8" read that is really UTF-16 must not
    /// short-circuit the wide-encoding heuristic.
    #[test]
    fn sniff_treats_nul_bearing_utf8_as_wide_encoding() {
        let bytes = utf16le_bytes("hi");
        let (text, label) = sniff_text_bytes(&bytes).expect("must decode as UTF-16LE");
        assert_eq!(text, "hi");
        assert_eq!(label, "utf-16le");
    }

    #[test]
    fn sniff_refuses_utf16_decoding_to_embedded_nul() {
        // A BOM followed by a U+0000 payload is not text — decline so the
        // caller can still reject it as binary.
        let bytes = vec![0xff, 0xfe, 0x00, 0x00, 0x00, 0x00];
        assert_eq!(sniff_text_bytes(&bytes), None);
    }

    /// 0as review handling (2026-09-19): pins where the encoding_rs GB18030
    /// table (WHATWG / GB18030-2005+) intentionally differs from the Python
    /// reference's CPython codec (GB18030-2000 PUA mappings) — see the module
    /// header for the full-space differential evidence and the mirrored nails
    /// in `assurance/tests/test_ops_executor_decode_text.py`.
    #[test]
    fn gb18030_table_boundaries_vs_python_reference_pinned() {
        // Bare euro byte: encoding_rs decodes it cleanly (GB18030 euro slot);
        // the CPython reference treats 0x80 as an error and falls to lossy —
        // the one known label fork between the two chains.
        assert_eq!(
            decode_text(&[0x80]),
            ("\u{20ac}".to_string(), "gb18030".to_string())
        );
        // A3A0: official U+3000 here, GB18030-2000 PUA U+E5E5 in the
        // reference (both sides still label "gb18030").
        assert_eq!(
            decode_text(&[0xa3, 0xa0]),
            ("\u{3000}".to_string(), "gb18030".to_string())
        );
        // Over-range/reserved four-byte form: encoding_rs replaces the whole
        // sequence with ONE U+FFFD, so the GB side wins the minimal choice;
        // the reference counts two, ties keep the ladder order (UTF-8) —
        // the lossy-choice fork, pinned on both sides.
        assert_eq!(
            decode_text(&[0x84, 0x31, 0xa5, 0x30]),
            ("\u{fffd}".to_string(), "utf-8-lossy:25.00%".to_string())
        );
    }
}
