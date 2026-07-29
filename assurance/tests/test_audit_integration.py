"""Tests for audit integration — GAK-EVT-001.

Covers :mod:`assurance.audit_integration`: event-to-audit mapping,
audit seal lifecycle, and verification of real canonical CLI runs
with ``audit=True``.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from assurance.canonical_cli import _build_run_frozen_context
from assurance.errors import AssuranceError


ROOT = Path(__file__).resolve().parents[2]
SOURCE_LEDGER = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"
CREATED_AT = "2026-07-29T12:00:00Z"
ASK = "Check whether the cited source can support a mechanism claim."


def _make_frozen_context(run_id: str) -> dict:
    """Build a valid frozen_context for test namespaces."""
    return _build_run_frozen_context(run_id)


# ══════════════════════════════════════════════════════════════════════════════
# build_audit_writer unit tests
# ══════════════════════════════════════════════════════════════════════════════


class BuildAuditWriterTests(unittest.TestCase):
    """GAK-EVT-001: ``build_audit_writer()`` produces correct callbacks."""

    @classmethod
    def setUpClass(cls) -> None:
        from assurance.conversation import ConversationNamespace
        from assurance.keystore import MemoryInstallationKeyStore
        cls._tmp = tempfile.TemporaryDirectory(dir=ROOT)
        cls._key_store = MemoryInstallationKeyStore()
        cls._namespace = ConversationNamespace.create(
            Path(cls._tmp.name) / "convs",
            key_store=cls._key_store,
            frozen_context=_make_frozen_context("TEST-EVT-001"),
            allowed_capabilities=[
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=["network.unrestricted", "secret.raw_read"],
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls._key_store.close()
        cls._tmp.cleanup()

    def test_build_returns_callable_and_seal(self) -> None:
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-001",
        )
        self.assertTrue(callable(on_event))
        self.assertTrue(callable(seal))

    def test_empty_events_produces_error_on_seal(self) -> None:
        """Sealing an empty event list raises (AuditLedger enforces non-empty)."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-002",
        )
        # No events fed → seal should fail
        with self.assertRaises(AssuranceError) as ctx:
            seal()
        self.assertIn("empty", str(ctx.exception))

    def test_event_mapping_basic(self) -> None:
        """Feed events and verify the seal receipt is valid."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-003",
        )
        # Feed a minimal but complete event sequence
        events = _sample_event_sequence()
        for evt in events:
            on_event(evt)
        receipt = seal()
        self.assertIn("receipt_id", receipt)
        self.assertIn("ledger_id", receipt)
        self.assertEqual(receipt["journal"]["event_count"], len(events))
        self.assertGreater(receipt["journal"]["bytes"], 0)

    def test_source_kind_mapping(self) -> None:
        """Verify each event type maps to the correct audit source_kind."""
        from assurance.audit_integration import _SOURCE_KIND

        self.assertEqual(_SOURCE_KIND["run_preflight"], "kernel")
        self.assertEqual(_SOURCE_KIND["instruction_provenance_gate"], "kernel")
        self.assertEqual(_SOURCE_KIND["tool_availability_check"], "kernel")
        self.assertEqual(_SOURCE_KIND["gate_decision"], "session")
        self.assertEqual(_SOURCE_KIND["model_request"], "provider")
        self.assertEqual(_SOURCE_KIND["model_output"], "provider")
        self.assertEqual(_SOURCE_KIND["artifact_registered"], "runtime")
        self.assertEqual(_SOURCE_KIND["run_finished"], "kernel")
        self.assertEqual(_SOURCE_KIND["run_failed"], "kernel")

    def test_provenance_mapping(self) -> None:
        """Verify model_output is marked derived_unverified."""
        from assurance.audit_integration import _PROVENANCE

        self.assertEqual(_PROVENANCE["model_output"], "derived_unverified")
        self.assertEqual(_PROVENANCE["model_request"], "direct")
        self.assertEqual(_PROVENANCE["run_preflight"], "direct")
        self.assertEqual(_PROVENANCE["gate_decision"], "direct")

    def test_fact_name_safety(self) -> None:
        """Verify fact names produced by _build_facts pass audit validation."""
        from assurance.audit import _normalize_facts
        from assurance.audit_integration import _build_facts

        for evt in _sample_event_sequence():
            facts = _build_facts(evt)
            # Must not raise
            normalized = _normalize_facts(facts)
            self.assertIsInstance(normalized, dict)

    def test_skip_events_with_no_type(self) -> None:
        """Events with empty event_type are silently skipped."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-004",
        )
        # Feed a no-type event (should be skipped)
        on_event({"event_type": "", "payload": {}, "timestamp": ""})
        # Feed a real event
        events = _sample_event_sequence()
        for evt in events:
            on_event(evt)
        receipt = seal()
        # Only the real events should be counted
        self.assertEqual(receipt["journal"]["event_count"], len(events))

    def test_unknown_event_type_defaults_to_kernel(self) -> None:
        """Events with unrecognised event_type fall back to source_kind=kernel."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-005",
        )
        # Unknown event type should not crash
        on_event({
            "event_type": "some_future_event",
            "event_sha256": "a" * 64,
            "payload": {},
            "timestamp": "2026-07-29T00:00:00Z",
        })
        on_event({
            "event_type": "run_finished",
            "event_sha256": "b" * 64,
            "payload": {"status": "completed"},
            "timestamp": "2026-07-29T00:00:01Z",
        })
        receipt = seal()
        self.assertEqual(receipt["journal"]["event_count"], 2)


# ══════════════════════════════════════════════════════════════════════════════
# Integration: real path with audit=True (mock — no real API call)
# ══════════════════════════════════════════════════════════════════════════════


class RealPathAuditIntegrationTests(unittest.TestCase):
    """GAK-EVT-001: ``run_canonical_guarded_cli_real(audit=True)`` wiring.

    These tests mock the DeepSeek API call but exercise the full audit
    integration path (gate setup → event stream → audit seal).
    """

    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.tmp.name) / "audit-real-run"

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_real_path_audit_survives_api_failure(self) -> None:
        """When the real API call fails (no credential), audit still seals."""
        from unittest.mock import patch

        # _read_windows_credential raises → api_error is set → run_failed path
        with patch(
            "assurance.canonical_cli._read_windows_credential",
            side_effect=OSError("mock: credential not available"),
        ):
            from assurance.canonical_cli import run_canonical_guarded_cli_real

            receipt = run_canonical_guarded_cli_real(
                run_root=self.run_root,
                source_ledger_path=SOURCE_LEDGER,
                ask=ASK,
                created_at=CREATED_AT,
                audit=True,
            )
            # The run receipt itself is valid (execution completed),
            # but the terminal event is run_failed
            self.assertTrue(receipt["valid"])
            self.assertIn("run_failed", receipt.get("event_types", []))
            # But the audit seal should still exist
            self.assertIn("audit_seal", receipt)
            seal = receipt["audit_seal"]
            self.assertNotIn("error", seal, f"Audit seal errored: {seal.get('error', '')}")
            self.assertGreater(seal["event_count"], 0)


# ══════════════════════════════════════════════════════════════════════════════
# bypass / missing-source path tests
# ══════════════════════════════════════════════════════════════════════════════


class AuditBypassTests(unittest.TestCase):
    """GAK-EVT-001: verify audit detects missing/incomplete source paths."""

    @classmethod
    def setUpClass(cls) -> None:
        from assurance.conversation import ConversationNamespace
        from assurance.keystore import MemoryInstallationKeyStore
        cls._tmp = tempfile.TemporaryDirectory(dir=ROOT)
        cls._key_store = MemoryInstallationKeyStore()
        cls._namespace = ConversationNamespace.create(
            Path(cls._tmp.name) / "convs",
            key_store=cls._key_store,
            frozen_context=_make_frozen_context("TEST-BYPASS"),
            allowed_capabilities=[
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=["network.unrestricted", "secret.raw_read"],
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls._key_store.close()
        cls._tmp.cleanup()

    def test_missing_provider_source_is_unknown(self) -> None:
        """When no provider events are present, seal declares provider=unknown."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-BYPASS-001",
        )
        # Feed only kernel events — no provider events
        on_event({
            "event_type": "run_preflight",
            "event_sha256": "a" * 64,
            "payload": {"model_id": "test", "real_network_allowed": False},
            "timestamp": "2026-07-29T00:00:00Z",
        })
        on_event({
            "event_type": "run_finished",
            "event_sha256": "b" * 64,
            "payload": {"status": "completed"},
            "timestamp": "2026-07-29T00:00:01Z",
        })
        receipt = seal()
        # Provider source should be declared unknown (no events observed)
        sources = {s["kind"]: s["declared_completeness"] for s in receipt["sources"]}
        self.assertEqual(sources.get("provider"), "unknown")
        self.assertEqual(sources.get("kernel"), "complete")
        self.assertEqual(sources.get("acp"), "unknown")
        self.assertEqual(sources.get("supervisor"), "unknown")

    def test_seal_fails_when_events_are_missing(self) -> None:
        """Sealing requires at least one event."""
        from assurance.audit_integration import build_audit_writer
        on_event, seal = build_audit_writer(
            namespace=self._namespace,
            key_store=self._key_store,
            run_id="RUN-BYPASS-002",
        )
        with self.assertRaises(AssuranceError):
            seal()


# ══════════════════════════════════════════════════════════════════════════════
# helpers
# ══════════════════════════════════════════════════════════════════════════════


def _sample_event_sequence() -> list[dict]:
    """Return a minimal valid event sequence matching the canonical CLI shape."""
    return [
        {
            "event_type": "run_preflight",
            "event_sha256": "a1b2c3d4e5f6" * 10 + "abcd",
            "payload": {
                "model_id": "deepseek-v4-pro",
                "adapter_id": "fake",
                "real_network_allowed": False,
            },
            "timestamp": "2026-07-29T00:00:00Z",
        },
        {
            "event_type": "instruction_provenance_gate",
            "event_sha256": "b2c3d4e5f6a1" * 10 + "bcde",
            "payload": {"receipt_sha256": "c" * 64, "context_sha256": "d" * 64},
            "timestamp": "2026-07-29T00:00:01Z",
        },
        {
            "event_type": "tool_availability_check",
            "event_sha256": "c3d4e5f6a1b2" * 10 + "cdef",
            "payload": {"available_count": 3, "unavailable_count": 0},
            "timestamp": "2026-07-29T00:00:02Z",
        },
        {
            "event_type": "run_started",
            "event_sha256": "d4e5f6a1b2c3" * 10 + "defa",
            "payload": {"task_id": "TASK-001", "run_root": "/tmp/test"},
            "timestamp": "2026-07-29T00:00:03Z",
        },
        {
            "event_type": "gate_decision",
            "event_sha256": "e5f6a1b2c3d4" * 10 + "efab",
            "payload": {"decision": "allow", "reference_count": 2},
            "timestamp": "2026-07-29T00:00:04Z",
        },
        {
            "event_type": "model_request",
            "event_sha256": "f6a1b2c3d4e5" * 10 + "fabc",
            "payload": {
                "provider": "deepseek",
                "model_id": "deepseek-v4-pro",
                "real_network_used": False,
            },
            "timestamp": "2026-07-29T00:00:05Z",
        },
        {
            "event_type": "model_output",
            "event_sha256": "a1b2c3d4e5f6" * 10 + "a1b2",
            "payload": {
                "answer_packet_sha256": "e" * 64,
                "structured_output_valid": True,
                "real_network_used": False,
            },
            "timestamp": "2026-07-29T00:00:06Z",
        },
        {
            "event_type": "artifact_registered",
            "event_sha256": "b2c3d4e5f6a1" * 10 + "b2c3",
            "payload": {"artifact_path": "answer-packet.json", "artifact_sha256": "e" * 64},
            "timestamp": "2026-07-29T00:00:07Z",
        },
        {
            "event_type": "run_finished",
            "event_sha256": "c3d4e5f6a1b2" * 10 + "c3d4",
            "payload": {"status": "completed"},
            "timestamp": "2026-07-29T00:00:08Z",
        },
    ]


if __name__ == "__main__":
    unittest.main()
