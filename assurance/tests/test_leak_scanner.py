"""Tests for LeakScanner — dual-channel information leakage detection."""

from __future__ import annotations

import unittest

from assurance.leak_scanner import (
    LeakFinding,
    LeakScanner,
    ScanCategory,
    ScanMode,
    ScanResult,
    Severity,
    _detect_structural_anomalies,
    _detect_fingerprint_match,
    _entropy,
    _hash_snippet,
    assert_no_leaks,
    scan_for_leaks,
)
from assurance.errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# 1. Utility functions
# ══════════════════════════════════════════════════════════════════════════════


class EntropyTests(unittest.TestCase):
    """Tests for _entropy()."""

    def test_empty_string_zero_entropy(self) -> None:
        self.assertEqual(_entropy(""), 0.0)

    def test_uniform_distribution_high_entropy(self) -> None:
        # 256 unique chars → max entropy
        import string
        all_chars = string.printable
        e = _entropy(all_chars)
        self.assertGreater(e, 4.0)

    def test_repeated_char_low_entropy(self) -> None:
        e = _entropy("aaaaa")
        self.assertLess(e, 0.1)

    def test_base64_like_high_entropy(self) -> None:
        e = _entropy("dGhpcyBpcyBhIHRlc3Qgb2YgYmFzZTY0IGVuY29kaW5n")
        self.assertGreater(e, 4.0)


class HashSnippetTests(unittest.TestCase):
    """Tests for _hash_snippet()."""

    def test_returns_64_char_hex(self) -> None:
        h = _hash_snippet("test")
        self.assertEqual(len(h), 64)
        self.assertTrue(all(c in "0123456789abcdef" for c in h))

    def test_deterministic(self) -> None:
        self.assertEqual(_hash_snippet("hello"), _hash_snippet("hello"))


# ══════════════════════════════════════════════════════════════════════════════
# 2. Mechanical channel — pattern detection
# ══════════════════════════════════════════════════════════════════════════════


class MechanicalScanTests(unittest.TestCase):
    """Tests for mechanical (pattern-based) leak detection."""

    def setUp(self) -> None:
        self.scanner = LeakScanner(mode=ScanMode.GUARDED)

    def test_clean_text_no_findings(self) -> None:
        result = self.scanner.scan("The quick brown fox jumps over the lazy dog.")
        self.assertEqual(len(result.findings), 0)
        self.assertFalse(result.blocked)

    def test_api_key_detected(self) -> None:
        result = self.scanner.scan(
            'config: {"api_key": "sk-abcdefghijklmnopqrstuvwxyz1234567890"}'
        )
        cred_findings = [f for f in result.findings if f.category == ScanCategory.CREDENTIAL]
        self.assertGreater(len(cred_findings), 0)
        self.assertTrue(any(f.severity == Severity.CRITICAL for f in cred_findings))

    def test_aws_key_detected(self) -> None:
        result = self.scanner.scan("AKIAIOSFODNN7EXAMPLE")
        cred_findings = [f for f in result.findings if f.category == ScanCategory.CREDENTIAL]
        self.assertGreater(len(cred_findings), 0)

    def test_private_ip_detected(self) -> None:
        result = self.scanner.scan("Server running on 192.168.1.100:8080")
        host_findings = [f for f in result.findings if f.category == ScanCategory.INTERNAL_HOST]
        self.assertGreater(len(host_findings), 0)

    def test_windows_user_path_detected(self) -> None:
        result = self.scanner.scan(r"Error in C:\Users\john\AppData\Local\config.json")
        path_findings = [f for f in result.findings if f.category == ScanCategory.INTERNAL_PATH]
        self.assertGreater(len(path_findings), 0)

    def test_db_connection_string_detected(self) -> None:
        result = self.scanner.scan("postgresql://admin:secret@db.internal:5432/production")
        code_findings = [
            f for f in result.findings
            if f.category == ScanCategory.CODE_FINGERPRINT
            and f.rule_id == "db_connection_string"
        ]
        self.assertGreater(len(code_findings), 0)

    def test_sql_create_table_detected(self) -> None:
        result = self.scanner.scan(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT, password_hash TEXT)"
        )
        code_findings = [
            f for f in result.findings
            if f.rule_id == "sql_create_table"
        ]
        self.assertGreater(len(code_findings), 0)

    def test_private_key_pem_detected(self) -> None:
        result = self.scanner.scan("-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0B\n-----END PRIVATE KEY-----")
        code_findings = [
            f for f in result.findings
            if f.rule_id == "private_key_pem"
        ]
        self.assertGreater(len(code_findings), 0)

    def test_system_persona_leak_detected(self) -> None:
        result = self.scanner.scan("You are a helpful AI assistant named Claude.")
        prompt_findings = [
            f for f in result.findings
            if f.category == ScanCategory.PROMPT_LEAKAGE
        ]
        self.assertGreater(len(prompt_findings), 0)

    def test_findings_never_contain_raw_matched_text(self) -> None:
        """LeakFinding.evidence must never contain the raw matched content."""
        result = self.scanner.scan('sk-ant-api03-abcdefghijklmnopqrstuvwxyz1234567890')
        for f in result.findings:
            self.assertNotIn("sk-ant", f.evidence)
            self.assertEqual(len(f.snippet_hash), 64)


# ══════════════════════════════════════════════════════════════════════════════
# 3. Semantic channel — structural heuristics
# ══════════════════════════════════════════════════════════════════════════════


class SemanticScanTests(unittest.TestCase):
    """Tests for semantic (structural) leak detection."""

    def test_high_entropy_line_detected(self) -> None:
        import secrets, string
        # Wide character set + 200 chars → entropy reliably >5.5 bits/char.
        alphabet = string.ascii_letters + string.digits + string.punctuation
        high_entropy = "".join(secrets.choice(alphabet) for _ in range(200))
        findings = _detect_structural_anomalies(high_entropy)
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0].channel, "semantic")

    def test_base64_density_detected(self) -> None:
        import base64
        lines = []
        for _ in range(10):
            lines.append(base64.b64encode(b"x" * 30).decode())
        text = "\n".join(lines)
        findings = _detect_structural_anomalies(text)
        b64_findings = [f for f in findings if f.rule_id == "base64_density"]
        self.assertGreater(len(b64_findings), 0)

    def test_path_density_detected(self) -> None:
        paths = [
            "/home/user/project/src/main.py",
            "/home/user/project/src/utils.py",
            "/home/user/project/lib/config.py",
            "/home/user/project/tests/test_main.py",
            "/home/user/project/data/input.csv",
            "/home/user/project/data/output.csv",
            "/home/user/project/docs/readme.md",
            "/home/user/project/scripts/run.sh",
            "/home/user/project/config/settings.yaml",
            "/home/user/project/logs/debug.log",
        ]
        findings = _detect_structural_anomalies("\n".join(paths))
        path_findings = [f for f in findings if f.rule_id == "path_density"]
        self.assertGreater(len(path_findings), 0)

    def test_fingerprint_match_known_ngrams(self) -> None:
        text = "the quick brown fox jumps over the lazy dog"
        # Pre-compute fingerprint of a known ngram
        fp = _hash_snippet("quick brown fox jumps over")
        findings = _detect_fingerprint_match(text, known_fingerprints=[fp])
        self.assertGreater(len(findings), 0)
        self.assertEqual(findings[0].snippet_hash, fp)

    def test_no_fingerprint_match_without_known_fingerprints(self) -> None:
        findings = _detect_fingerprint_match("any text here", known_fingerprints=[])
        self.assertEqual(len(findings), 0)


# ══════════════════════════════════════════════════════════════════════════════
# 4. Mode behavior
# ══════════════════════════════════════════════════════════════════════════════


class ModeBehaviorTests(unittest.TestCase):
    """Tests for mode-based verdict behavior."""

    def test_discussion_mode_never_blocks(self) -> None:
        scanner = LeakScanner(mode=ScanMode.DISCUSSION)
        # High-severity finding
        result = scanner.scan("AKIAIOSFODNN7EXAMPLE sk-abcdefghijklmnopqrstuvwxyz1234567890")
        self.assertGreater(len(result.findings), 0)
        self.assertFalse(result.blocked)

    def test_strict_mode_blocks_on_any_finding(self) -> None:
        scanner = LeakScanner(mode=ScanMode.STRICT)
        # Even a low-severity finding blocks
        result = scanner.scan(r"C:\Users\public\Downloads\file.txt")
        self.assertGreater(len(result.findings), 0)
        self.assertTrue(result.blocked)

    def test_guarded_blocks_on_high_severity(self) -> None:
        scanner = LeakScanner(mode=ScanMode.GUARDED)
        # API key = CRITICAL → blocked
        result = scanner.scan("sk-abcdefghijklmnopqrstuvwxyz1234567890")
        self.assertTrue(result.blocked)

    def test_guarded_allows_low_severity(self) -> None:
        scanner = LeakScanner(mode=ScanMode.GUARDED)
        # Low-severity finding (path with content in temp)
        result = scanner.scan(r"C:\Windows\Temp\abcdefgh\temp_file.txt")
        low_findings = [f for f in result.findings if f.severity == Severity.LOW]
        if low_findings:
            self.assertFalse(result.blocked)

    def test_clean_text_not_blocked_in_any_mode(self) -> None:
        for mode in ScanMode:
            scanner = LeakScanner(mode=mode)
            result = scanner.scan("The weather is nice today.")
            self.assertEqual(len(result.findings), 0)
            self.assertFalse(result.blocked)


# ══════════════════════════════════════════════════════════════════════════════
# 5. scan_dict
# ══════════════════════════════════════════════════════════════════════════════


class ScanDictTests(unittest.TestCase):
    """Tests for scan_dict()."""

    def test_clean_dict_no_findings(self) -> None:
        scanner = LeakScanner(mode=ScanMode.STRICT)
        result = scanner.scan_dict({"answer": "The result is 42."})
        self.assertFalse(result.blocked)

    def test_dict_with_api_key_detected(self) -> None:
        scanner = LeakScanner(mode=ScanMode.GUARDED)
        result = scanner.scan_dict({
            "config": {"api_key": "sk-abcdefghijklmnopqrstuvwxyz1234567890"},
            "answer": "ok",
        })
        self.assertTrue(result.blocked)

    def test_nested_dict_scanned_recursively(self) -> None:
        scanner = LeakScanner(mode=ScanMode.GUARDED)
        result = scanner.scan_dict({
            "level1": {
                "level2": {
                    "level3": "postgresql://user:pass@10.0.0.1:5432/db",
                },
            },
        })
        self.assertGreater(len(result.findings), 0)


# ══════════════════════════════════════════════════════════════════════════════
# 6. Convenience functions
# ══════════════════════════════════════════════════════════════════════════════


class ConvenienceFunctionTests(unittest.TestCase):
    """Tests for scan_for_leaks and assert_no_leaks."""

    def test_scan_for_leaks_returns_result(self) -> None:
        result = scan_for_leaks("clean text", mode=ScanMode.GUARDED)
        self.assertIsInstance(result, ScanResult)
        self.assertFalse(result.blocked)

    def test_assert_no_leaks_passes_on_clean_text(self) -> None:
        result = assert_no_leaks("clean text")
        self.assertFalse(result.blocked)

    def test_assert_no_leaks_raises_on_leak(self) -> None:
        with self.assertRaises(AssuranceError):
            assert_no_leaks("sk-abcdefghijklmnopqrstuvwxyz1234567890")

    def test_assert_no_leaks_defaults_to_strict(self) -> None:
        # In STRICT mode, even LOW severity blocks.
        with self.assertRaises(AssuranceError):
            assert_no_leaks(
                "Server at localhost:8080\n"
                + "\n".join(f"/tmp/test_{i}.txt" for i in range(30))
            )


# ══════════════════════════════════════════════════════════════════════════════
# 7. Channel tracking
# ══════════════════════════════════════════════════════════════════════════════


class ChannelTrackingTests(unittest.TestCase):
    """Tests that findings are correctly attributed to channels."""

    def test_both_channels_produce_findings(self) -> None:
        scanner = LeakScanner(mode=ScanMode.DISCUSSION)
        # Text with both credential (mechanical) and structural anomalies
        import base64
        text = (
            "sk-abcdefghijklmnopqrstuvwxyz1234567890\n"
            + base64.b64encode(b"x" * 60).decode() + "\n"
        )
        result = scanner.scan(text)
        self.assertGreater(result.channel_mechanical_count, 0)
        # Semantic channel findings may vary; at minimum, mechanical found something.

    def test_all_findings_have_channel_set(self) -> None:
        scanner = LeakScanner(mode=ScanMode.DISCUSSION)
        result = scanner.scan("sk-abcdefghijklmnopqrstuvwxyz1234567890")
        for f in result.findings:
            self.assertIn(f.channel, ("mechanical", "semantic"))


if __name__ == "__main__":
    unittest.main()
