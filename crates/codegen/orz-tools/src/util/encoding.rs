//! Mechanical text-encoding adaptation (GAP-ENCODING-GATE, OPS-PROTOCOL §8).
//!
//! The contract is encoding-agnostic at the model boundary: every
//! cross-process / cross-environment text surface decodes through the fixed
//! chain — strip BOM → UTF-8 strict → GB18030 → UTF-8 lossy — and records
//! which stage produced the text (`output_encoding`). Writes are always
//! UTF-8 without BOM. The reference labels mirror
//! `assurance/ops_executor.py::decode_text` so Rust and Python audits agree.

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
/// - `utf-8-lossy`: neither UTF-8 nor valid GB18030 — replacement chars.
pub fn decode_text(data: &[u8]) -> (String, &'static str) {
    let mut bytes = data;
    let mut label = "utf-8";
    if let Some(stripped) = bytes.strip_prefix(b"\xef\xbb\xbf") {
        bytes = stripped;
        label = "utf-8-sig";
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (text.to_string(), label);
    }
    // encoding_rs's GB18030 decoder is total; `had_errors` marks byte
    // sequences the reference Python decoder rejects (truncated / illegal
    // four-byte forms). Only a clean decode counts as "gb18030" — anything
    // else falls through to the lossy stage, mirroring `decode_text` in
    // assurance/ops_executor.py.
    let (text, had_errors) = encoding_rs::GB18030.decode_without_bom_handling(bytes);
    if !had_errors {
        return (text.into_owned(), "gb18030");
    }
    (String::from_utf8_lossy(bytes).into_owned(), "utf-8-lossy")
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
        assert_eq!(label, "utf-8-lossy");
        assert!(text.contains('\u{fffd}'));
    }

    #[test]
    fn invalid_gb18030_falls_through_to_lossy() {
        // 0x81 0x30 is an illegal GB18030 sequence (second byte below 0x40;
        // also a truncated four-byte form) — Python's `decode("gb18030")`
        // raises here, so the chain must NOT claim "gb18030".
        let bytes = [0x81, 0x30, 0x81];
        let (_, label) = decode_text(&bytes);
        assert_eq!(label, "utf-8-lossy");
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
}
