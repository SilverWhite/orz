"""Tests for :mod:`assurance.grok_session_verifier` — D1.12."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from assurance.grok_session_verifier import verify_acp_session
from assurance.utils import sha256_bytes, canonical_bytes


def _make_acp_receipt(
    *,
    run_id: str = "RUN-GROK-ACP-TEST-001",
    session_id: str = "test-session-id-12345",
    tool_call_count: int = 2,
    permission_requests_count: int = 1,
    permission_outcomes: list[str] | None = None,
) -> dict:
    session_id_hash = sha256_bytes(session_id.encode("utf-8"))
    return {
        "schema_version": "0.1.0",
        "receipt_kind": "grok_runtime_adapter_receipt",
        "run_id": run_id,
        "acp": {
            "protocol_version": 1,
            "session_id_hash": session_id_hash,
            "tool_call_count": tool_call_count,
            "permission_requests_count": permission_requests_count,
            "permission_outcomes": permission_outcomes if permission_outcomes is not None else ["allow_once"],
            "turn_count": 2,
            "stop_reason": "end_turn",
            "event_count": 7,
        },
        "execution": {"stdout_path": ""},
        "artifacts": {"events_path": ""},
    }


def _make_events_jsonl(
    *,
    session_id_hash: str = "",
    tool_count: int = 2,
    perm_outcomes: list[str] | None = None,
) -> str:
    """Build a valid events.jsonl with hash chain for a mock ACP session."""
    import secrets
    if not session_id_hash:
        session_id_hash = sha256_bytes(b"test-session")
    if perm_outcomes is None:
        perm_outcomes = ["allow_once"]

    events: list[dict] = []
    prev = None
    seq = 0

    def _add(et: str, payload: dict) -> None:
        nonlocal seq, prev
        event = {
            "schema_version": "0.1.0-draft",
            "run_id": "RUN-GROK-ACP-TEST-001",
            "event_id": f"EVT-TEST-{seq:03d}",
            "sequence": seq,
            "timestamp": "2026-07-31T00:00:00Z",
            "event_type": et,
            "run_manifest_sha256": sha256_bytes(b"test-manifest"),
            "previous_event_sha256": prev,
            "payload_schema": "grok-runtime-normalized-v0.1",
            "payload": payload,
            "payload_sha256": sha256_bytes(canonical_bytes(payload)),
            "redaction": "metadata_only",
            "event_sha256": "",
        }
        event["event_sha256"] = sha256_bytes(
            canonical_bytes({k: v for k, v in event.items() if k != "event_sha256"})
        )
        prev = event["event_sha256"]
        seq += 1
        events.append(event)

    _add("run_preflight", {"adapter_id": "test", "provider": "grok"})
    _add("run_started", {"task_id": "TASK-TEST", "run_root": "/tmp/test"})
    _add("acp_initialize", {"protocol_version": 1})
    _add("acp_session_created", {"session_id_hash": session_id_hash})
    for i in range(tool_count):
        _add("tool_proposal", {"tool_name": f"tool_{i}", "tool_call_id": f"tc_{i}"})
    for outcome in perm_outcomes:
        _add("permission_requested", {"permission": "test_tool", "options": ["allow_once"]})
        _add("permission_decision", {"permission": "test_tool", "decision": outcome})
    _add("artifact_registered", {"artifact_path": "/tmp/test/receipt.json", "artifact_kind": "receipt"})
    _add("run_finished", {"status": "completed"})

    return "\n".join(json.dumps(e, ensure_ascii=False, sort_keys=True, separators=(",", ":")) for e in events) + "\n"


def _make_transcript_jsonl(
    *,
    messages: int = 8,
) -> str:
    """Build a minimal ACP transcript with valid message_sha256 entries."""
    lines = []
    for i in range(messages):
        direction = "client_to_agent" if i % 2 == 0 else "agent_to_client"
        # Use a deterministic-but-unique digest per message.
        digest = sha256_bytes(f"acp-message-{i}".encode("utf-8"))
        lines.append(json.dumps(
            {"direction": direction, "message_sha256": digest},
            ensure_ascii=False, sort_keys=True, separators=(",", ":"),
        ))
    return "\n".join(lines) + "\n"


class GrokSessionVerifierTests(unittest.TestCase):
    """Post-run cross-verification tests — D1.12."""

    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory(prefix="gsa-verify-test-")
        self.tmp = Path(self._tmpdir.name)

    def tearDown(self) -> None:
        self._tmpdir.cleanup()

    def _write_files(
        self,
        receipt: dict,
        *,
        session_id: str = "test-session-id-12345",
        tool_count: int = 2,
        perm_outcomes: list[str] | None = None,
        transcript_messages: int = 8,
    ) -> dict:
        """Write receipt + events.jsonl + transcript.jsonl to tmpdir and return receipt."""
        session_id_hash = sha256_bytes(session_id.encode("utf-8"))
        events_raw = _make_events_jsonl(
            session_id_hash=session_id_hash,
            tool_count=tool_count,
            perm_outcomes=perm_outcomes,
        )
        transcript_raw = _make_transcript_jsonl(messages=transcript_messages)

        events_path = self.tmp / "events.jsonl"
        transcript_path = self.tmp / "acp_transcript.jsonl"
        events_path.write_text(events_raw, encoding="utf-8")
        transcript_path.write_text(transcript_raw, encoding="utf-8")

        receipt["execution"]["stdout_path"] = str(transcript_path.resolve())
        receipt["artifacts"]["events_path"] = str(events_path.resolve())
        return receipt

    # ── Happy path ─────────────────────────────────────────────────────────

    def test_all_checks_pass_with_consistent_data(self) -> None:
        """Consistent receipt + events + transcript → all checks pass."""
        receipt = self._write_files(_make_acp_receipt())
        result = verify_acp_session(receipt)
        self.assertTrue(result["valid"], f"errors: {result['errors']}")
        self.assertTrue(result["checks"]["acp_lifecycle_complete"])
        self.assertTrue(result["checks"]["session_id_consistent"])
        self.assertTrue(result["checks"]["tool_call_count_match"])
        self.assertTrue(result["checks"]["permission_count_match"])
        self.assertTrue(result["checks"]["permission_outcomes_match"])
        self.assertTrue(result["checks"]["transcript_has_expected_lifecycle"])
        self.assertTrue(result["checks"]["event_hash_chain_valid"])
        self.assertEqual(result["event_count"], 10)  # preflight+start+init+session+2tools+2perm+artifact+finish
        self.assertEqual(len(result["errors"]), 0)

    def test_receipt_kind_and_version_populated(self) -> None:
        """Receipt metadata is well-formed."""
        receipt = self._write_files(_make_acp_receipt())
        result = verify_acp_session(receipt)
        self.assertEqual(result["receipt_kind"], "grok_session_verification_receipt")
        self.assertEqual(result["schema_version"], "0.1.0-draft")
        self.assertTrue(result["verification_id"].startswith("GSV-"))

    # ── Tool count mismatch ────────────────────────────────────────────────

    def test_tool_count_mismatch_detected(self) -> None:
        """Receipt says 3 tool calls but events have only 2."""
        receipt = self._write_files(_make_acp_receipt(tool_call_count=3), tool_count=2)
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["tool_call_count_match"])
        self.assertIn("tool_call count mismatch", result["errors"][0])

    # ── Permission count mismatch ──────────────────────────────────────────

    def test_permission_count_mismatch_detected(self) -> None:
        """Receipt says 2 permission requests but events have only 1."""
        receipt = self._write_files(
            _make_acp_receipt(permission_requests_count=2),
            perm_outcomes=["allow_once"],
        )
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["permission_count_match"])

    # ── Permission outcomes mismatch ───────────────────────────────────────

    def test_permission_outcomes_mismatch_detected(self) -> None:
        """Receipt outcomes differ from event decisions."""
        receipt = self._write_files(
            _make_acp_receipt(permission_outcomes=["allow_once"]),
            perm_outcomes=["cancelled"],
        )
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["permission_outcomes_match"])

    # ── Session ID mismatch ────────────────────────────────────────────────

    def test_session_id_mismatch_detected(self) -> None:
        """Receipt session_id_hash differs from events."""
        receipt = self._write_files(_make_acp_receipt(), session_id="different-session-id")
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["session_id_consistent"])

    # ── ACP lifecycle incomplete ───────────────────────────────────────────

    def test_acp_lifecycle_incomplete_detected(self) -> None:
        """Events missing acp_initialize event."""
        receipt = self._write_files(_make_acp_receipt())
        # Manually remove acp_initialize + acp_session_created from events
        events_path = Path(str(receipt["artifacts"]["events_path"]))
        events_raw = events_path.read_text(encoding="utf-8")
        filtered = [
            line for line in events_raw.splitlines()
            if line.strip() and '"acp_initialize"' not in line and '"acp_session_created"' not in line
        ]
        events_path.write_text("\n".join(filtered) + "\n", encoding="utf-8")
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["acp_lifecycle_complete"])

    # ── Transcript lifecycle incomplete ────────────────────────────────────

    def test_transcript_too_short_detected(self) -> None:
        """Transcript has fewer than 6 messages."""
        receipt = self._write_files(_make_acp_receipt(), transcript_messages=4)
        result = verify_acp_session(receipt)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["transcript_has_expected_lifecycle"])

    # ── Event hash chain ───────────────────────────────────────────────────

    def test_hash_chain_valid(self) -> None:
        """Valid hash chain passes verification."""
        receipt = self._write_files(_make_acp_receipt())
        result = verify_acp_session(receipt)
        self.assertTrue(result["checks"]["event_hash_chain_valid"])

    def test_hash_chain_broken_detected(self) -> None:
        """Broken hash chain is detected."""
        receipt = self._write_files(_make_acp_receipt())
        events_path = Path(str(receipt["artifacts"]["events_path"]))
        events_raw = events_path.read_text(encoding="utf-8")
        # Corrupt the previous_event_sha256 in the second event
        lines = events_raw.splitlines()
        if len(lines) >= 2:
            evt = json.loads(lines[1])
            evt["previous_event_sha256"] = "0" * 64
            lines[1] = json.dumps(evt, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        events_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        result = verify_acp_session(receipt)
        self.assertFalse(result["checks"]["event_hash_chain_valid"])

    # ── Empty / missing files ──────────────────────────────────────────────

    def test_missing_events_file_produces_empty_counts(self) -> None:
        """Missing events.jsonl → empty events, all checks fail or produce 0 counts."""
        receipt = _make_acp_receipt()
        receipt["artifacts"]["events_path"] = str(self.tmp / "nonexistent.jsonl")
        receipt["execution"]["stdout_path"] = str(self.tmp / "nonexistent_transcript.jsonl")
        result = verify_acp_session(receipt)
        self.assertEqual(result["event_count"], 0)
        self.assertEqual(result["transcript_message_count"], 0)
        # Without events, lifecycle is incomplete and counts won't match
        self.assertFalse(result["checks"]["acp_lifecycle_complete"])

    # ── SHA-256 fields ─────────────────────────────────────────────────────

    def test_sha256_fields_populated(self) -> None:
        """All SHA-256 reference fields are 64-char hex strings."""
        receipt = self._write_files(_make_acp_receipt())
        result = verify_acp_session(receipt)
        for field in ["receipt_acp_block_sha256", "events_sha256", "transcript_sha256", "receipt_sha256"]:
            val = result[field]
            self.assertEqual(len(val), 64, f"{field} should be 64 chars, got {len(val)}")
            int(val, 16)  # valid hex

    def test_multiple_permission_outcomes(self) -> None:
        """Multiple permission requests with interleaved outcomes."""
        receipt = self._write_files(
            _make_acp_receipt(
                permission_requests_count=2,
                permission_outcomes=["allow_once", "cancelled"],
            ),
            perm_outcomes=["allow_once", "cancelled"],
        )
        result = verify_acp_session(receipt)
        self.assertTrue(result["valid"], f"errors: {result['errors']}")
        self.assertTrue(result["checks"]["permission_outcomes_match"])

    def test_zero_tool_calls(self) -> None:
        """Session with zero tool calls — all counts match at 0."""
        receipt = self._write_files(
            _make_acp_receipt(
                tool_call_count=0,
                permission_requests_count=0,
                permission_outcomes=[],
            ),
            tool_count=0,
            perm_outcomes=[],
        )
        result = verify_acp_session(receipt)
        self.assertTrue(result["valid"], f"errors: {result['errors']}")
        self.assertTrue(result["checks"]["tool_call_count_match"])
        self.assertTrue(result["checks"]["permission_count_match"])
