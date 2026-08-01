"""GAK-05: unified cross-file action kernel verification tests."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path
import unittest

from assurance.action_kernel_result import verify_action_kernel
from assurance.utils import sha256_bytes, sha256_file


def _build_minimal_output_root(root: Path) -> dict[str, object]:
    """Create a minimal valid action kernel output root.

    Returns a dict with keys for each created file so tests can tamper.
    """
    run_id = "RUN-GAK05-TEST-001"
    manifest_sha256 = "0" * 64

    # 1. run-manifest.json — minimal valid dict with run_id
    manifest = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "created_at": "2026-08-02T00:00:00Z",
    }
    manifest_path = root / "run-manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, sort_keys=True) + "\n", encoding="utf-8"
    )
    actual_manifest_sha256 = sha256_file(manifest_path)

    # 2. events.jsonl — 3 events: preflight, started, finished
    events = [
        {
            "schema_version": "0.1.0-draft",
            "run_id": run_id,
            "event_id": f"EVT-TEST-00{i}",
            "sequence": i,
            "timestamp": "2026-08-02T00:00:00Z",
            "event_type": etype,
            "run_manifest_sha256": actual_manifest_sha256,
            "previous_event_sha256": prev if prev else None,
            "payload": {},
            "payload_sha256": sha256_bytes(b"{}"),
            "event_sha256": "temp",  # recomputed below
        }
        for i, (etype, prev) in enumerate([
            ("run_preflight", None),
            ("run_started", "TEMP0"),
            ("run_finished", "TEMP1"),
        ])
    ]
    # Compute real hash chain
    prev_hash: str | None = None
    for event in events:
        event["previous_event_sha256"] = prev_hash
        body = dict(event)
        body.pop("event_sha256", None)
        from assurance.utils import canonical_bytes as _cb
        event["event_sha256"] = sha256_bytes(_cb(body))
        prev_hash = event["event_sha256"]
    journal_path = root / "events.jsonl"
    with journal_path.open("w", encoding="utf-8") as fh:
        for event in events:
            fh.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")

    # 3. windows-process-result.json
    process_result = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "run_manifest_sha256": actual_manifest_sha256,
        "exit_code": 0,
        "terminal_state": "succeeded",
        "executable": "python.exe",
        "argument_count": 2,
        "arguments_sha256": "b" * 64,
        "stdout_bytes": 100,
        "stdout_sha256": "c" * 64,
        "stderr_bytes": 0,
        "stderr_sha256": sha256_bytes(b""),
        "timeout_seconds": 30,
        "wall_clock_ms": 1500,
        "job_object_created": True,
        "job_object_assigned": True,
        "containment_degraded": False,
    }
    proc_path = root / "windows-process-result.json"
    proc_path.write_text(
        json.dumps(process_result, sort_keys=True) + "\n", encoding="utf-8"
    )

    # 4. session-record.json
    session_record = {
        "schema_version": "0.1.0-draft",
        "session_id": f"SESS-{run_id}",
        "run_id": run_id,
        "run_manifest_sha256": actual_manifest_sha256,
        "journal_head": prev_hash,
        "terminal_state": "run_finished",
        "artifact_count": 1,
    }
    session_path = root / "session-record.json"
    session_path.write_text(
        json.dumps(session_record, sort_keys=True) + "\n", encoding="utf-8"
    )

    return {
        "run_id": run_id,
        "manifest_sha256": actual_manifest_sha256,
        "last_event_sha256": prev_hash,
        "manifest_path": manifest_path,
        "journal_path": journal_path,
        "proc_path": proc_path,
        "session_path": session_path,
    }


class ActionKernelResultTests(unittest.TestCase):
    """Tests for unified cross-file verification."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_valid_output_root_passes(self) -> None:
        _build_minimal_output_root(self.root)
        result = verify_action_kernel(self.root)
        self.assertTrue(result["valid"], result["errors"])
        self.assertTrue(result["checks"]["manifest_present"])
        self.assertTrue(result["checks"]["journal_replay_valid"])
        self.assertTrue(result["checks"]["exactly_one_terminal"])
        self.assertTrue(result["checks"]["run_id_process_matches"])
        self.assertTrue(result["checks"]["run_id_session_matches"])
        self.assertTrue(result["checks"]["terminal_state_mapping"])
        self.assertTrue(result["checks"]["no_model_or_tool_events"])

    def test_missing_manifest_fails(self) -> None:
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["manifest_present"])

    def test_corrupt_journal_fails(self) -> None:
        _build_minimal_output_root(self.root)
        # Truncate journal
        journal_path = self.root / "events.jsonl"
        journal_path.write_text("not json\n", encoding="utf-8")
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["journal_replay_valid"])

    def test_no_terminal_event_fails(self) -> None:
        _build_minimal_output_root(self.root)
        # Remove the last event (run_finished)
        journal_path = self.root / "events.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        journal_path.write_text("\n".join(lines[:-1]) + "\n", encoding="utf-8")
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["exactly_one_terminal"])

    def test_run_id_mismatch_process_fails(self) -> None:
        _build_minimal_output_root(self.root)
        proc_path = self.root / "windows-process-result.json"
        proc = json.loads(proc_path.read_text(encoding="utf-8"))
        proc["run_id"] = "RUN-WRONG"
        proc_path.write_text(json.dumps(proc, sort_keys=True) + "\n", encoding="utf-8")
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["run_id_process_matches"])

    def test_run_id_mismatch_session_fails(self) -> None:
        _build_minimal_output_root(self.root)
        session_path = self.root / "session-record.json"
        session = json.loads(session_path.read_text(encoding="utf-8"))
        session["run_id"] = "RUN-WRONG"
        session_path.write_text(
            json.dumps(session, sort_keys=True) + "\n", encoding="utf-8"
        )
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["run_id_session_matches"])

    def test_terminal_state_mapping_fails(self) -> None:
        _build_minimal_output_root(self.root)
        proc_path = self.root / "windows-process-result.json"
        proc = json.loads(proc_path.read_text(encoding="utf-8"))
        proc["terminal_state"] = "failed"
        proc_path.write_text(json.dumps(proc, sort_keys=True) + "\n", encoding="utf-8")
        result = verify_action_kernel(self.root)
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["terminal_state_mapping"])

    def test_missing_process_result_optional(self) -> None:
        _build_minimal_output_root(self.root)
        (self.root / "windows-process-result.json").unlink()
        result = verify_action_kernel(
            self.root, require_process_result=False
        )
        # Should still be valid — process result is optional when flag says so
        self.assertFalse(result["checks"]["process_result_present"])
        # But manifest + journal + session are still consistent
        self.assertTrue(result["checks"]["manifest_present"])
        self.assertTrue(result["checks"]["journal_replay_valid"])

    def test_missing_session_record_optional(self) -> None:
        _build_minimal_output_root(self.root)
        (self.root / "session-record.json").unlink()
        result = verify_action_kernel(
            self.root, require_session_record=False
        )
        self.assertFalse(result["checks"]["session_record_present"])
        self.assertTrue(result["checks"]["manifest_present"])

    def test_forbidden_model_events_detected(self) -> None:
        _build_minimal_output_root(self.root)
        # Rebuild journal with a model_request event inserted (valid hash chain)
        from assurance.utils import canonical_bytes as _cb
        run_id = "RUN-GAK05-TEST-001"
        manifest_sha256 = sha256_file(self.root / "run-manifest.json")
        prev_hash = None
        new_events = [
            ("run_preflight", None),
            ("run_started", None),
            ("model_request", None),       # forbidden event inserted here
            ("run_finished", None),
        ]
        built: list[dict] = []
        for i, (etype, _) in enumerate(new_events):
            event = {
                "schema_version": "0.1.0-draft",
                "run_id": run_id,
                "event_id": f"EVT-TEST-{i:03d}",
                "sequence": i,
                "timestamp": "2026-08-02T00:00:00Z",
                "event_type": etype,
                "run_manifest_sha256": manifest_sha256,
                "previous_event_sha256": prev_hash,
                "payload": {},
                "payload_sha256": sha256_bytes(b"{}"),
                "event_sha256": "",
            }
            event["event_sha256"] = sha256_bytes(_cb(
                {k: v for k, v in event.items() if k != "event_sha256"}
            ))
            prev_hash = event["event_sha256"]
            built.append(event)
        journal_path = self.root / "events.jsonl"
        with journal_path.open("w", encoding="utf-8") as fh:
            for event in built:
                fh.write(json.dumps(event, sort_keys=True, separators=(",", ":")) + "\n")
        result = verify_action_kernel(self.root)
        # The terminal event is not last (model_request is between started and finished),
        # but the hash chain is valid.  The forbidden event should be detected.
        self.assertFalse(result["checks"].get("no_model_or_tool_events", True))
        self.assertTrue(
            any("model" in e.lower() for e in result["errors"]),
            f"expected model event error in {result['errors']}",
        )


if __name__ == "__main__":
    unittest.main()
