"""Tests for :mod:`assurance.finding_registry` — D3.22."""

from __future__ import annotations

import json
import unittest

from assurance.finding_registry import (
    Finding,
    FindingRegistry,
    PermissionRecord,
    record_permission_decision,
)
from assurance.utils import sha256_bytes


class FindingTests(unittest.TestCase):
    """Finding dataclass — schema, immutability, no permission fields."""

    def test_create_produces_valid_finding(self) -> None:
        finding = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"test-snippet"),
            evidence="API key pattern matched at position 42",
            location="line 42",
            rule_id="openai_api_key_format",
            channel="mechanical",
        )
        self.assertTrue(finding.finding_id.startswith("FIND-"))
        self.assertEqual(len(finding.finding_id), 21)  # FIND- + 16 hex
        self.assertEqual(finding.source_scanner, "leak_scanner")
        self.assertEqual(finding.severity, "critical")
        self.assertEqual(len(finding.compute_sha256()), 64)

    def test_finding_sha256_is_stable(self) -> None:
        finding = Finding.create(
            source_scanner="credential_scrub",
            category="credential",
            severity="high",
            snippet_hash=sha256_bytes(b"secret-token"),
            evidence="Bearer token found",
        )
        sha1 = finding.compute_sha256()
        sha2 = finding.compute_sha256()
        self.assertEqual(sha1, sha2)

    def test_finding_has_no_permission_fields(self) -> None:
        """Invariant: Finding dataclass has no decision/recommendation fields."""
        finding = Finding.create(
            source_scanner="leak_scanner",
            category="internal_path",
            severity="medium",
            snippet_hash=sha256_bytes(b"C:\\Users\\test"),
            evidence="Windows user profile path detected",
        )
        d = finding.to_dict()
        # These fields must NOT exist
        for forbidden in ["recommended_action", "should_block", "decision", "permission"]:
            self.assertNotIn(forbidden, d, f"Finding must not contain '{forbidden}'")

    def test_finding_is_immutable(self) -> None:
        finding = Finding.create(
            source_scanner="validator_bridge",
            category="schema_violation",
            severity="high",
            snippet_hash=sha256_bytes(b"bad-schema"),
            evidence="Schema validation failed",
        )
        with self.assertRaises(Exception):
            finding.severity = "low"  # type: ignore[misc]

    def test_finding_idempotent_registration(self) -> None:
        """Registering the same finding twice is a no-op."""
        registry = FindingRegistry()
        finding = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"key"),
            evidence="test",
        )
        registry.register(finding)
        registry.register(finding)  # no-op
        self.assertEqual(len(registry), 1)

    def test_finding_id_collision_with_different_content_raises(self) -> None:
        """Same finding_id with different content raises ValueError."""
        registry = FindingRegistry()
        f1 = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"key1"),
            evidence="test 1",
        )
        registry.register(f1)
        # Manually create a Finding with same ID but different content
        f2 = Finding(
            finding_id=f1.finding_id,
            source_scanner="leak_scanner",
            category="different",
            severity="low",
            snippet_hash=sha256_bytes(b"key2"),
            evidence="test 2",
            created_at=f1.created_at,
        )
        with self.assertRaises(ValueError):
            registry.register(f2)


class PermissionRecordTests(unittest.TestCase):
    """PermissionRecord — dissociation from findings."""

    def test_create_produces_valid_record(self) -> None:
        record = PermissionRecord.create(
            session_run_id="RUN-TEST-001",
            decision="allow_once",
            authority="user",
            tool_name="read_file",
            referenced_finding_ids=["a" * 64, "b" * 64],
            basis="user approved via TUI dialog",
        )
        self.assertTrue(record.record_id.startswith("PERM-"))
        self.assertEqual(len(record.record_id), 21)
        self.assertEqual(record.decision, "allow_once")
        self.assertEqual(record.authority, "user")
        self.assertEqual(len(record.referenced_finding_ids), 2)
        self.assertEqual(len(record.compute_sha256()), 64)

    def test_record_has_no_finding_content(self) -> None:
        """Invariant: PermissionRecord references findings by ID only."""
        record = PermissionRecord.create(
            session_run_id="RUN-TEST-002",
            decision="cancelled",
            authority="adapter",
            tool_name="shell",
            referenced_finding_ids=["c" * 64],
        )
        d = record.to_dict()
        # Must reference by ID
        self.assertIn("referenced_finding_ids", d)
        # Must NOT embed finding content
        for forbidden in ["severity", "evidence", "snippet_hash", "category"]:
            self.assertNotIn(forbidden, d, f"PermissionRecord must not contain '{forbidden}'")
        # referenced_finding_ids must be SHA-256 hex strings
        for ref_id in d["referenced_finding_ids"]:
            self.assertEqual(len(ref_id), 64)
            int(ref_id, 16)  # valid hex

    def test_authority_validation(self) -> None:
        """User, adapter, and policy are valid authorities."""
        for authority in ("user", "adapter", "policy"):
            record = PermissionRecord.create(
                session_run_id="RUN-TEST",
                decision="allow_once",
                authority=authority,
                tool_name="test",
            )
            self.assertEqual(record.authority, authority)

    def test_default_empty_references(self) -> None:
        record = PermissionRecord.create(
            session_run_id="RUN-TEST",
            decision="allow_once",
            authority="adapter",
            tool_name="test",
        )
        self.assertEqual(record.referenced_finding_ids, [])


class FindingRegistryTests(unittest.TestCase):
    """FindingRegistry — thread-safe, session-scoped."""

    def test_empty_registry(self) -> None:
        registry = FindingRegistry()
        self.assertEqual(len(registry), 0)
        self.assertEqual(registry.list_ids(), [])
        self.assertEqual(registry.list_active(), [])

    def test_register_and_list(self) -> None:
        registry = FindingRegistry()
        f1 = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"k1"),
            evidence="e1",
        )
        f2 = Finding.create(
            source_scanner="credential_scrub",
            category="internal_path",
            severity="medium",
            snippet_hash=sha256_bytes(b"k2"),
            evidence="e2",
        )
        registry.register(f1)
        registry.register(f2)
        self.assertEqual(len(registry), 2)
        self.assertIn(f1.finding_id, registry)
        self.assertIn(f2.finding_id, registry)
        self.assertEqual(len(registry.list_ids()), 2)

    def test_registry_isolation(self) -> None:
        """Two registries don't share state."""
        r1 = FindingRegistry()
        r2 = FindingRegistry()
        f = Finding.create(
            source_scanner="leak_scanner",
            category="test",
            severity="low",
            snippet_hash=sha256_bytes(b"iso"),
            evidence="isolation test",
        )
        r1.register(f)
        self.assertEqual(len(r1), 1)
        self.assertEqual(len(r2), 0)
        self.assertNotIn(f.finding_id, r2)

    def test_list_ids_is_sorted(self) -> None:
        registry = FindingRegistry()
        ids = []
        for i in range(5):
            f = Finding.create(
                source_scanner="leak_scanner",
                category="test",
                severity="low",
                snippet_hash=sha256_bytes(f"id-{i}".encode()),
                evidence=f"finding {i}",
            )
            ids.append(f.finding_id)
            registry.register(f)
        self.assertEqual(registry.list_ids(), sorted(ids))


class RecordPermissionDecisionTests(unittest.TestCase):
    """Integration: record_permission_decision() convenience function."""

    def test_records_with_registry_snapshot(self) -> None:
        registry = FindingRegistry()
        f = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"snap"),
            evidence="snapshot test",
        )
        registry.register(f)
        record = record_permission_decision(
            registry=registry,
            session_run_id="RUN-SNAP-001",
            decision="allow_once",
            authority="user",
            tool_name="bash",
            basis="user approved with awareness of 1 finding",
        )
        self.assertEqual(record.decision, "allow_once")
        self.assertEqual(record.authority, "user")
        self.assertEqual(len(record.referenced_finding_ids), 1)
        self.assertIn(f.compute_sha256(), record.referenced_finding_ids)

    def test_records_with_none_registry(self) -> None:
        """None registry → empty references."""
        record = record_permission_decision(
            registry=None,
            session_run_id="RUN-NONE-001",
            decision="cancelled",
            authority="adapter",
            tool_name="shell",
        )
        self.assertEqual(record.referenced_finding_ids, [])
        self.assertEqual(record.authority, "adapter")

    def test_record_sha256_is_stable(self) -> None:
        record = record_permission_decision(
            registry=None,
            session_run_id="RUN-STABLE-001",
            decision="allow_once",
            authority="adapter",
            tool_name="test",
        )
        sha1 = record.compute_sha256()
        sha2 = record.compute_sha256()
        self.assertEqual(sha1, sha2)


class SchemaValidationTests(unittest.TestCase):
    """End-to-end: to_dict() output validates against JSON Schema."""

    def test_finding_to_dict_validates(self) -> None:
        finding = Finding.create(
            source_scanner="leak_scanner",
            category="credential",
            severity="critical",
            snippet_hash=sha256_bytes(b"val"),
            evidence="Schema validation test",
            location="test.py:10",
            rule_id="test_rule",
            channel="mechanical",
        )
        d = finding.to_dict()
        d["finding_sha256"] = finding.compute_sha256()
        # Round-trip through JSON to verify schema compatibility
        raw = json.dumps(d, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        reparsed = json.loads(raw)
        self.assertEqual(reparsed["finding_kind"], "assurance_finding")
        self.assertEqual(len(reparsed["finding_sha256"]), 64)

    def test_permission_record_to_dict_validates(self) -> None:
        record = PermissionRecord.create(
            session_run_id="RUN-VAL-001",
            decision="allow_once",
            authority="user",
            tool_name="read_file",
            referenced_finding_ids=["a" * 64],
            basis="validation test",
        )
        d = record.to_dict()
        d["record_sha256"] = record.compute_sha256()
        raw = json.dumps(d, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        reparsed = json.loads(raw)
        self.assertEqual(reparsed["record_kind"], "assurance_permission_record")
        self.assertEqual(len(reparsed["referenced_finding_ids"]), 1)
