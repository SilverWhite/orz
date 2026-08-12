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
}
