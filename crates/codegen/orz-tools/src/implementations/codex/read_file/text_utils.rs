//! Shared text utilities for the codex read_file tool.

use super::slice::MAX_LINE_LENGTH;

/// Truncate a string at a char boundary, returning at most `max_bytes`
/// bytes. Port of codex `take_bytes_at_char_boundary`.
pub(crate) fn take_at_char_boundary(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut last_ok = 0;
    for (i, ch) in s.char_indices() {
        let nb = i + ch.len_utf8();
        if nb > max_bytes {
            break;
        }
        last_ok = nb;
    }
    &s[..last_ok]
}

/// Fixed-chain decode (GAP-ENCODING-GATE, OPS-PROTOCOL §8) + truncate at
/// MAX_LINE_LENGTH at a char boundary.
pub(crate) fn format_display(raw: &[u8]) -> String {
    let decoded = crate::util::encoding::decode_text(raw).0;
    if decoded.len() > MAX_LINE_LENGTH {
        take_at_char_boundary(&decoded, MAX_LINE_LENGTH).to_string()
    } else {
        decoded
    }
}
