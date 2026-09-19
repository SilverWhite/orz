"""Resident nails for the Python frozen reference ``ops_executor.decode_text``.

Until 2026-09-19 this mirror had no test coverage at all: the Rust side
(orz-tools ``util/encoding.rs``) pins its own behaviour with unit tests, so
the "Rust/Python audits agree" claim could silently drift. These nails pin

* the six cross-implementation golden cases (byte-identical with the Rust
  nails in ``encoding.rs``),
* strict-stage regressions (pure UTF-8 / GB18030 / BOM / empty),
* the structural invariants (lossy ratio recomputable from the label;
  degraded units never exceed the whole-buffer ``from_utf8_lossy`` baseline;
  the lossy stage always degrades at least one unit), and
* — from the 2026-09-19 full-space differential — the characterised GB18030
  table boundaries between encoding_rs 0.8.35 (WHATWG / GB18030-2005+) and
  CPython 3.12's ``gb18030`` codec (GB18030-2000 PUA mappings), so any codec
  or dependency bump that moves them is caught on both sides.

Differential evidence (1-byte + 2-byte + all 1,587,600 four-byte forms,
2026-09-19, recorded in ``docs/audits/0AS_REVIEW_HANDLING_2026-09-19.md``):
cleanliness agrees everywhere except the bare euro byte 0x80 (encoding_rs:
clean ``€``; CPython: error — the one known label fork); 20 two-byte pairs
and one four-byte slot (``8135F437``) map to different code points
(GB18030-2000 PUA vs official characters); over-range four-byte forms
replace with one U+FFFD in encoding_rs vs two in CPython, which can flip the
lossy minimal-replacement choice.
"""

from __future__ import annotations

import unittest

from assurance.ops_executor import decode_text


def _units(text: str) -> int:
    """Degraded units as counted by the lossy stage: placeholders + U+FFFD."""
    return text.count("\ufffd") + text.count("⟨")


class DecodeTextGoldenNails(unittest.TestCase):
    def test_six_cross_implementation_golden_cases(self):
        # Byte-identical with the Rust nails in orz-tools util/encoding.rs
        # (lossy_mixed_sample / prefers_gb18030 / tie_keeps_ladder_order /
        # multibyte_subsequence_placeholder_form / ratio_excludes_stripped_bom
        # / invalid_gb18030_falls_through_to_lossy).
        mixed = (b"readable " + "保持可读".encode() + b"\n\x81\n"
                 + b"\xd6\xd0\xce\xc4" + b" tail\n")
        cases = [
            (mixed, "readable 保持可读\n⟨0x81⟩\n中文 tail\n", "utf-8-lossy:2.94%"),
            (b"\xd6\xd0\xff", "中\ufffd", "utf-8-lossy:33.33%"),
            (b"\xd6\xd0\x8f", "⟨0xD6⟩\u040f", "utf-8-lossy:33.33%"),
            (b"\xf0\x9e\x81", "⟨0xF0 0x9E 0x81⟩", "utf-8-lossy:33.33%"),
            (b"\xef\xbb\xbf\x81", "⟨0x81⟩", "utf-8-lossy:100.00%"),
            (b"\x81\x30\x81", "\ufffd", "utf-8-lossy:33.33%"),
        ]
        for data, want_text, want_label in cases:
            with self.subTest(data=data):
                self.assertEqual(decode_text(data), (want_text, want_label))

    def test_strict_stage_regressions(self):
        # 判据②: pure-encoding behaviour stays byte-identical.
        self.assertEqual(decode_text("hello 世界".encode()), ("hello 世界", "utf-8"))
        self.assertEqual(decode_text(b""), ("", "utf-8"))
        self.assertEqual(decode_text(b"\xef\xbb\xbfhello"), ("hello", "utf-8-sig"))
        self.assertEqual(decode_text(b"\xd6\xd0\xce\xc4"), ("中文", "gb18030"))

    def test_placeholder_form_is_uppercase_hex_space_separated(self):
        text, _ = decode_text(b"\xf0\x9e\x81")
        self.assertEqual(text, "⟨0xF0 0x9E 0x81⟩")


class DecodeTextInvariants(unittest.TestCase):
    def test_lossy_ratio_is_recomputable_from_label(self):
        for data in (b"\xff\xfe\x80\x81", b"\x81\x30\x81", b"\xd6\xd0\xff"):
            with self.subTest(data=data):
                text, label = decode_text(data)
                self.assertTrue(label.startswith("utf-8-lossy:"))
                self.assertTrue(label.endswith("%"))
                pct = float(label[len("utf-8-lossy:"):-1])
                expected = _units(text) / max(len(data), 1) * 100.0
                self.assertAlmostEqual(pct, expected, delta=0.005)

    def test_degraded_units_never_exceed_whole_buffer_baseline(self):
        for data in (b"\xff\xfe\x80\x81", b"\x81\x30\x81", b"\xd6\xd0\xff"):
            with self.subTest(data=data):
                text, _ = decode_text(data)
                baseline = data.decode("utf-8", errors="replace").count("\ufffd")
                self.assertLessEqual(_units(text), baseline)

    def test_lossy_stage_always_degrades_at_least_one_unit(self):
        for data in (b"\x81", b"\x81\x30\x81", b"\xd6\xd0\xff"):
            text, label = decode_text(data)
            if label.startswith("utf-8-lossy:"):
                self.assertGreaterEqual(_units(text), 1)


class DecodeTextTableBoundaryNails(unittest.TestCase):
    """Characterised forks vs the Rust side (encoding_rs / WHATWG) — each has
    a mirrored pin in ``encoding.rs`` so a codec or dependency bump that moves
    the boundary is caught on both sides."""

    def test_bare_euro_byte_falls_to_lossy_in_the_reference(self):
        # Rust (encoding_rs): ("€", "gb18030"). The one known label fork.
        self.assertEqual(decode_text(b"\x80"), ("⟨0x80⟩", "utf-8-lossy:100.00%"))

    def test_a3a0_maps_to_gb18030_2000_pua_in_the_reference(self):
        # Rust (encoding_rs): U+3000. Both sides label "gb18030"; the decoded
        # text differs (GB18030-2000 PUA vs the official character).
        self.assertEqual(decode_text(b"\xa3\xa0"), ("\ue5e5", "gb18030"))

    def test_over_range_four_byte_form_ties_to_utf8_in_the_reference(self):
        # Rust: the GB side counts ONE U+FFFD for the over-range form and wins
        # the minimal choice (("\ufffd", "utf-8-lossy:25.00%")). The reference
        # counts two, so the tie keeps the ladder order (UTF-8).
        self.assertEqual(
            decode_text(b"\x84\x31\xa5\x30"), ("⟨0x84⟩1⟨0xA5⟩0", "utf-8-lossy:50.00%")
        )


if __name__ == "__main__":
    unittest.main()
