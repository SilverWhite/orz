"""Tests for endpoint_canonicalizer — filesystem path and network endpoint
canonicalization used by adapter_preflight, network_permit_gateway, and
other gate modules.
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path
import unittest

from assurance.endpoint_canonicalizer import (
    canonicalize_filesystem_path,
    canonicalize_network_endpoint,
    validate_endpoint_list,
    validate_filesystem_targets,
)
from assurance.errors import AssuranceError


# ── canonicalize_filesystem_path ─────────────────────────────────────────────

class FilesystemCanonicalizationTests(unittest.TestCase):
    """Tests for canonicalize_filesystem_path."""

    def test_absolute_path_normalised(self) -> None:
        # On Windows the result is rooted at the current drive (e.g. D:/foo/baz).
        # On POSIX it is /foo/baz.  We only assert that ``..`` was resolved.
        result = canonicalize_filesystem_path("/foo/bar/../baz")
        self.assertIn("foo", result)
        self.assertIn("baz", result)
        self.assertNotIn("bar", result)
        self.assertNotIn("..", result)

    def test_relative_with_base_root(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            result = canonicalize_filesystem_path("subdir/file.txt", base_root=base)
            expected = (base / "subdir/file.txt").as_posix()
            self.assertEqual(result, expected)

    def test_traversal_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            with self.assertRaises(AssuranceError) as ctx:
                canonicalize_filesystem_path("../outside", base_root=base)
            self.assertIn("traversal", str(ctx.exception))

    def test_traversal_via_absolute_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            escape = (base.parent / "outside").as_posix()
            with self.assertRaises(AssuranceError) as ctx:
                canonicalize_filesystem_path(escape, base_root=base)
            self.assertIn("traversal", str(ctx.exception))

    def test_windows_style_path_handled(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            result = canonicalize_filesystem_path(
                str(base / "sub" / "file.txt"), base_root=base
            )
            self.assertIn("sub/file.txt", result)

    def test_path_object_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            result = canonicalize_filesystem_path(
                Path("subdir/file.txt"), base_root=base
            )
            expected = (base / "subdir/file.txt").as_posix()
            self.assertEqual(result, expected)

    def test_non_existent_path_still_resolved(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            result = canonicalize_filesystem_path(
                "nonexistent/../normalised", base_root=base
            )
            expected = (base / "normalised").as_posix()
            self.assertEqual(result, expected)

    def test_empty_path(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            result = canonicalize_filesystem_path("", base_root=base)
            self.assertEqual(result, base.as_posix())


# ── canonicalize_network_endpoint ─────────────────────────────────────────────

class NetworkEndpointCanonicalizationTests(unittest.TestCase):
    """Tests for canonicalize_network_endpoint."""

    def test_https_default_port_stripped(self) -> None:
        result = canonicalize_network_endpoint("https://api.example.com:443/v1/chat")
        self.assertEqual(result, "https://api.example.com/v1/chat")

    def test_http_default_port_stripped(self) -> None:
        result = canonicalize_network_endpoint("http://localhost:80/path")
        self.assertEqual(result, "http://localhost/path")

    def test_explicit_port_preserved(self) -> None:
        result = canonicalize_network_endpoint("https://api.example.com:8443/v1")
        self.assertEqual(result, "https://api.example.com:8443/v1")

    def test_trailing_slash_stripped(self) -> None:
        result = canonicalize_network_endpoint("https://api.example.com/v1/")
        self.assertEqual(result, "https://api.example.com/v1")

    def test_root_path_preserves_slash(self) -> None:
        result = canonicalize_network_endpoint("https://api.example.com/")
        self.assertEqual(result, "https://api.example.com/")

    def test_fragment_stripped(self) -> None:
        result = canonicalize_network_endpoint("https://api.example.com/path#section")
        self.assertEqual(result, "https://api.example.com/path")

    def test_userinfo_stripped(self) -> None:
        result = canonicalize_network_endpoint("https://user:pass@api.example.com/path")
        self.assertEqual(result, "https://api.example.com/path")

    def test_non_http_rejected(self) -> None:
        with self.assertRaises(AssuranceError) as ctx:
            canonicalize_network_endpoint("ftp://files.example.com/data")
        self.assertIn("http or https", str(ctx.exception))

    def test_no_hostname_rejected(self) -> None:
        with self.assertRaises(AssuranceError) as ctx:
            canonicalize_network_endpoint("https:///no-host")
        self.assertIn("hostname", str(ctx.exception))

    def test_hostname_lowercased(self) -> None:
        result = canonicalize_network_endpoint("https://API.Example.COM/Path")
        self.assertEqual(result, "https://api.example.com/Path")

    def test_deepseek_api_endpoint(self) -> None:
        result = canonicalize_network_endpoint("https://api.deepseek.com/v1/chat/completions")
        self.assertEqual(result, "https://api.deepseek.com/v1/chat/completions")

    def test_anthropic_api_endpoint(self) -> None:
        result = canonicalize_network_endpoint("https://api.anthropic.com/v1/messages")
        self.assertEqual(result, "https://api.anthropic.com/v1/messages")

    def test_malformed_url_rejected(self) -> None:
        with self.assertRaises(AssuranceError):
            canonicalize_network_endpoint("not-a-url-at-all!!!")

    def test_empty_string_rejected(self) -> None:
        with self.assertRaises(AssuranceError):
            canonicalize_network_endpoint("")


# ── validate_endpoint_list ────────────────────────────────────────────────────

class EndpointListValidationTests(unittest.TestCase):
    """Tests for validate_endpoint_list."""

    def test_all_valid_endpoints(self) -> None:
        results = validate_endpoint_list([
            "https://api.example.com/v1",
            "https://api.other.com:8443/path",
        ])
        self.assertEqual(len(results), 2)
        self.assertTrue(all(r["allowed"] for r in results))

    def test_invalid_endpoint_reported(self) -> None:
        results = validate_endpoint_list(["ftp://bad.scheme.com/data"])
        self.assertEqual(len(results), 1)
        self.assertFalse(results[0]["allowed"])
        self.assertIsNone(results[0]["canonical"])

    def test_scheme_allowlist(self) -> None:
        results = validate_endpoint_list(
            ["https://secure.example.com/v1", "http://insecure.example.com/data"],
            allowed_schemes={"https"},
        )
        self.assertTrue(results[0]["allowed"])
        self.assertFalse(results[1]["allowed"])
        self.assertIn("scheme", results[1]["reason"])

    def test_host_allowlist(self) -> None:
        results = validate_endpoint_list(
            [
                "https://api.deepseek.com/v1",
                "https://api.anthropic.com/v1",
            ],
            allowed_hosts={"api.deepseek.com"},
        )
        self.assertTrue(results[0]["allowed"])
        self.assertFalse(results[1]["allowed"])
        self.assertIn("host", results[1]["reason"])

    def test_empty_list(self) -> None:
        results = validate_endpoint_list([])
        self.assertEqual(results, [])

    def test_endpoint_with_port_in_allowlist(self) -> None:
        results = validate_endpoint_list(
            ["https://api.example.com:8443/v1"],
            allowed_hosts={"api.example.com"},
        )
        self.assertTrue(results[0]["allowed"])

    def test_mixed_valid_invalid(self) -> None:
        results = validate_endpoint_list([
            "https://good.example.com/v1",
            "not-a-url",
            "https://blocked.example.com/data",
        ])
        self.assertTrue(results[0]["allowed"])
        self.assertFalse(results[1]["allowed"])
        self.assertTrue(results[2]["allowed"])


# ── validate_filesystem_targets ───────────────────────────────────────────────

class FilesystemTargetsValidationTests(unittest.TestCase):
    """Tests for validate_filesystem_targets."""

    def test_all_valid_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            (base / "allowed").mkdir()
            results = validate_filesystem_targets(
                ["allowed/file.txt", "allowed/other.txt"],
                base_root=base,
            )
            self.assertEqual(len(results), 2)

    def test_path_outside_base_root_reported(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            results = validate_filesystem_targets(
                ["../outside"],
                base_root=base,
            )
            self.assertFalse(results[0]["allowed"])
            self.assertIsNone(results[0]["canonical"])

    def test_forbidden_prefix_detected(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            sub = base / "subdir"
            sub.mkdir()
            target = sub / "file.txt"
            target.write_text("test", encoding="utf-8")
            # Canonical result is absolute POSIX path; prefix must match.
            prefix = sub.as_posix()
            results = validate_filesystem_targets(
                [str(target)],
                base_root=base,
                forbidden_prefixes=[prefix],
            )
            self.assertFalse(results[0]["allowed"])
            self.assertIn("forbidden prefix", results[0]["reason"])

    def test_forbidden_prefix_case_insensitive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            sub = base / "system32_lower"
            sub.mkdir(parents=True, exist_ok=True)
            target = sub / "file.dll"
            target.write_text("test", encoding="utf-8")
            # Use a differently-cased prefix to verify case-insensitive matching.
            prefix = (base / "SYSTEM32_LOWER").as_posix()
            results = validate_filesystem_targets(
                [str(target)],
                base_root=base,
                forbidden_prefixes=[prefix],
            )
            self.assertFalse(results[0]["allowed"])
            self.assertIn("forbidden prefix", results[0]["reason"])

    def test_empty_list(self) -> None:
        results = validate_filesystem_targets([])
        self.assertEqual(results, [])

    def test_mixed_valid_invalid(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp).resolve()
            (base / "ok").mkdir()
            results = validate_filesystem_targets(
                [
                    str(base / "ok" / "file.txt"),
                    str(base.parent / "outside"),
                ],
                base_root=base,
            )
            self.assertEqual(len(results), 2)
            # First path: normalised within base
            self.assertIn("ok/file.txt", results[0]["canonical"] or "")
            self.assertTrue(results[0]["allowed"])
            # Second path: traversal
            self.assertFalse(results[1]["allowed"])


# ── Cross-module integration ──────────────────────────────────────────────────

class EndpointCanonicalizerIntegrationTests(unittest.TestCase):
    """Integration checks: adapters and gates consuming canonicaliser output."""

    def test_canonical_network_endpoint_used_by_preflight(self) -> None:
        # adapter_preflight.run_adapter_preflight internally calls
        # canonicalize_network_endpoint.  Verify it accepts a well-formed endpoint.
        from assurance.adapter_preflight import run_adapter_preflight

        receipt = run_adapter_preflight(
            adapter_id="test-adapter",
            provider="deepseek",
            model_id="deepseek-chat",
            endpoint="https://api.deepseek.com/v1/chat/completions",
            conversation_id="CONV-TEST",
            run_id="RUN-TEST",
        )
        self.assertIn("endpoint_canonicalized", receipt["checks"])

    def test_canonical_network_endpoint_used_by_permit_gateway(self) -> None:
        from assurance.network_permit_gateway import evaluate_network_permit

        receipt = evaluate_network_permit(
            endpoint="https://api.deepseek.com/v1/chat/completions",
            category="llm_provider",
            conversation_id="CONV-TEST",
            attempt=1,
            turn=1,
        )
        self.assertTrue(receipt["checks"]["endpoint_canonicalized"])

    def test_bad_endpoint_blocks_preflight(self) -> None:
        from assurance.adapter_preflight import run_adapter_preflight

        receipt = run_adapter_preflight(
            adapter_id="test-adapter",
            provider="deepseek",
            model_id="deepseek-chat",
            endpoint="ftp://invalid.scheme.com/api",
            conversation_id="CONV-TEST",
            run_id="RUN-TEST",
        )
        self.assertFalse(receipt["checks"]["endpoint_canonicalized"])
        self.assertFalse(receipt["preflight_passed"])

    def test_bad_endpoint_blocks_network_permit(self) -> None:
        from assurance.network_permit_gateway import (
            NetworkPermitBlockedError,
            evaluate_network_permit,
        )

        with self.assertRaises(NetworkPermitBlockedError):
            evaluate_network_permit(
                endpoint="ftp://invalid.scheme.com/api",
                category="llm_provider",
                conversation_id="CONV-TEST",
                attempt=1,
                turn=1,
            )


if __name__ == "__main__":
    unittest.main()
