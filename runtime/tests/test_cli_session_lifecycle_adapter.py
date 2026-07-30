from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from assurance.journal import append_event, replay_journal


ROOT = Path(__file__).resolve().parents[2]
ADAPTER = ROOT / "scripts" / "append_cli_session_lifecycle_event.py"
VERIFIER = ROOT / "scripts" / "verify_cli_session_lifecycle_receipt.py"


def _canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")


class CliSessionLifecycleAdapterTests(unittest.TestCase):
    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *args],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
            check=False,
        )

    def _observation(
        self,
        sequence: int,
        kind: str,
        *,
        turn_id: str | None = None,
        turn_status: str | None = None,
        outcome: str | None = None,
        error_sha256: str | None = None,
        observation_id: str | None = None,
        adapter_id: str = "CLIADAPTER-FIXTURE-001",
        session_id: str = "SESSION-FIXTURE-001",
        source_stream_id: str = "CLISTREAM-FIXTURE-001",
        source_record_sequence: int | None = None,
    ) -> dict:
        return {
            "schema_version": "0.2.0",
            "artifact_kind": "cli-session-lifecycle-observation",
            "observation_id": observation_id or f"CLIOBS-FIXTURE-{sequence:03d}",
            "adapter_id": adapter_id,
            "runtime_family": "generic-cli",
            "runtime_version": "1.0.0-fixture",
            "session_id": session_id,
            "run_id": "RUN-CLI-LIFECYCLE-001",
            "run_manifest_sha256": "a" * 64,
            "source_stream_id": source_stream_id,
            "source_sequence": sequence,
            "source_record_sequence": (
                sequence
                if source_record_sequence is None
                else source_record_sequence
            ),
            "timestamp": f"2026-07-25T14:{sequence:02d}:00Z",
            "event_kind": kind,
            "turn_id": turn_id,
            "turn_status": turn_status,
            "outcome": outcome,
            "error_sha256": error_sha256,
            "source_record_sha256": hashlib.sha256(
                f"source-{sequence}-{kind}".encode()
            ).hexdigest(),
        }

    def _append(
        self,
        root: Path,
        journal: Path,
        observation: dict,
        *,
        name: str,
        expected: int = 0,
    ) -> dict:
        observation_path = root / f"{name}.observation.json"
        receipt_path = root / f"{name}.receipt.json"
        observation_path.write_text(
            json.dumps(observation, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        completed = self._run(
            str(ADAPTER),
            "--observation",
            str(observation_path),
            "--journal",
            str(journal),
            "--output",
            str(receipt_path),
        )
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return json.loads(receipt_path.read_text(encoding="utf-8"))

    def _verify(
        self,
        root: Path,
        journal: Path,
        *,
        name: str,
        output_name: str | None = None,
        expected: int = 0,
    ) -> dict:
        output = root / f"{output_name or name}.verification.json"
        completed = self._run(
            str(VERIFIER),
            "--observation",
            str(root / f"{name}.observation.json"),
            "--journal",
            str(journal),
            "--receipt",
            str(root / f"{name}.receipt.json"),
            "--output",
            str(output),
        )
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return json.loads(output.read_text(encoding="utf-8"))

    def _complete_progress_state(self, journal: Path) -> None:
        from scripts.append_global_progress_transition_event import (
            append_transition_event,
        )

        lines = journal.read_text(encoding="utf-8").splitlines()
        previous = json.loads(lines[-1])["event_sha256"]
        states = [
            ("GPT-LIFECYCLE-STEP", "step_boundary", "executing", "reviewing"),
            ("GPT-LIFECYCLE-DONE", "task_completion", "reviewing", "completed"),
        ]
        for offset, (transition_id, kind, before, requested) in enumerate(states):
            payload = {
                "schema_version": "0.1.0",
                "transition_id": transition_id,
                "task_id": "TASK-CLI-LIFECYCLE-001",
                "transition_kind": kind,
                "state_before": before,
                "requested_state": requested,
                "state_after": requested,
                "transition_applied": True,
                "decision": "pass",
                "control_codes": ["GPS-TRANSITION-PASS"],
                "source_bindings": {
                    "request_sha256": "1" * 64,
                    "checkpoint_id": f"GPC-LIFECYCLE-{offset + 1}",
                    "checkpoint_file_sha256": "2" * 64,
                    "checkpoint_verification_sha256": "3" * 64,
                    "holistic_assessment_id": None,
                    "holistic_review_sha256": None,
                    "holistic_verification_sha256": None,
                },
            }
            event = {
                "schema_version": "0.1.0-draft",
                "run_id": "RUN-CLI-LIFECYCLE-001",
                "event_id": f"EVT-LIFECYCLE-GPS-{offset + 1}",
                "sequence": len(lines) + offset,
                "timestamp": f"2026-07-25T14:0{offset + 1}:30Z",
                "event_type": "gate_decision",
                "run_manifest_sha256": "a" * 64,
                "previous_event_sha256": previous,
                "payload_schema": "global-progress-transition-receipt-v0.1.schema.json",
                "payload": payload,
                "payload_sha256": hashlib.sha256(_canonical(payload)).hexdigest(),
                "redaction": "metadata_only",
                "event_sha256": "",
            }
            event["event_sha256"] = hashlib.sha256(
                _canonical(
                    {
                        key: value
                        for key, value in event.items()
                        if key != "event_sha256"
                    }
                )
            ).hexdigest()
            append_transition_event(journal, event)
            previous = event["event_sha256"]

    def test_complete_session_maps_to_canonical_journal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            receipts = [
                self._append(
                    root,
                    journal,
                    self._observation(0, "session_started"),
                    name="start",
                ),
                self._append(
                    root,
                    journal,
                    self._observation(1, "turn_started", turn_id="TURN-001"),
                    name="turn-start",
                ),
                self._append(
                    root,
                    journal,
                    self._observation(
                        2,
                        "turn_completed",
                        turn_id="TURN-001",
                        turn_status="completed",
                    ),
                    name="turn-complete",
                ),
                self._append(
                    root,
                    journal,
                    self._observation(
                        3, "session_completed", outcome="completed"
                    ),
                    name="finish",
                ),
            ]
            self.assertEqual(
                [receipt["canonical_event_type"] for receipt in receipts],
                ["run_started", "model_request", "model_output", "run_finished"],
            )
            self.assertEqual(receipts[-1]["lifecycle_state_after"], "terminal")
            verified = self._verify(root, journal, name="finish")
            self.assertTrue(verified["valid"])
            replay = replay_journal(
                run_manifest_path=None,
                journal_path=journal,
                expected_manifest_sha256="a" * 64,
                expected_run_id="RUN-CLI-LIFECYCLE-001",
            )
            self.assertTrue(replay["valid"])
            self.assertEqual(replay["terminal_event"], "run_finished")
            terminal_retry = self._append(
                root,
                journal,
                self._observation(
                    3, "session_completed", outcome="completed"
                ),
                name="finish-retry",
            )
            self.assertEqual(terminal_retry["adapter_status"], "already_recorded")
            self.assertEqual(
                len(journal.read_text(encoding="utf-8").splitlines()),
                4,
            )

    def test_cancellation_in_turn_clears_active_turn(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            receipt = self._append(
                root,
                journal,
                self._observation(2, "session_cancelled", outcome="user_cancelled"),
                name="cancel",
            )
            self.assertEqual(receipt["canonical_event_type"], "run_cancelled")
            self.assertEqual(receipt["lifecycle_state_after"], "terminal")
            self.assertIsNone(receipt["active_turn_id"])

    def test_interrupted_turn_closes_only_the_turn(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn-1",
            )
            interrupted = self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="interrupted",
                ),
                name="turn-1-interrupted",
            )
            self.assertEqual(interrupted["canonical_event_type"], "model_output")
            self.assertEqual(interrupted["turn_status"], "interrupted")
            self.assertEqual(interrupted["lifecycle_state_after"], "active")
            self.assertIsNone(interrupted["active_turn_id"])
            next_turn = self._append(
                root,
                journal,
                self._observation(3, "turn_started", turn_id="TURN-002"),
                name="turn-2",
            )
            self.assertEqual(next_turn["lifecycle_state_after"], "in_turn")
            self.assertEqual(next_turn["active_turn_id"], "TURN-002")
            events = [
                json.loads(line)
                for line in journal.read_text(encoding="utf-8").splitlines()
            ]
            self.assertFalse(
                any(
                    event["event_type"]
                    in {"run_finished", "run_failed", "run_cancelled"}
                    for event in events
                )
            )

    def test_failed_turn_preserves_error_digest_without_terminating_session(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            error_sha256 = hashlib.sha256(b"fixture turn error").hexdigest()
            failed = self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="failed",
                    error_sha256=error_sha256,
                ),
                name="turn-failed",
            )
            self.assertEqual(failed["turn_status"], "failed")
            self.assertEqual(failed["error_sha256"], error_sha256)
            self.assertEqual(failed["lifecycle_state_after"], "active")
            verified = self._verify(root, journal, name="turn-failed")
            self.assertTrue(verified["valid"])

    def test_failure_from_active_maps_to_run_failed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            receipt = self._append(
                root,
                journal,
                self._observation(1, "session_failed", outcome="runtime_error"),
                name="failed",
            )
            self.assertEqual(receipt["canonical_event_type"], "run_failed")

    def test_out_of_order_turn_completion_is_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            receipt = self._append(
                root,
                journal,
                self._observation(
                    1,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="completed",
                ),
                name="bad-complete",
                expected=3,
            )
            self.assertEqual(receipt["adapter_status"], "conflict")
            self.assertEqual(len(journal.read_text(encoding="utf-8").splitlines()), 1)

    def test_turn_terminal_status_is_required_and_error_digest_is_scoped(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            missing_status = self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                ),
                name="missing-status",
                expected=3,
            )
            self.assertEqual(missing_status["adapter_status"], "rejected")
            self.assertIn("turn_status", missing_status["errors"][0])
            invalid_error = self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="interrupted",
                    error_sha256="b" * 64,
                ),
                name="invalid-error",
                expected=3,
            )
            self.assertEqual(invalid_error["adapter_status"], "rejected")
            self.assertIn("only allowed", invalid_error["errors"][0])
            self.assertEqual(len(journal.read_text(encoding="utf-8").splitlines()), 2)

    def test_verifier_detects_turn_status_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="interrupted",
                ),
                name="interrupted",
            )
            self.assertTrue(
                self._verify(root, journal, name="interrupted")["valid"]
            )
            receipt_path = root / "interrupted.receipt.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            receipt["turn_status"] = "completed"
            receipt_path.write_text(
                json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            tampered = self._verify(
                root,
                journal,
                name="interrupted",
                output_name="interrupted-tampered",
                expected=2,
            )
            self.assertFalse(tampered["checks"]["lifecycle_projection_verified"])

    def test_exact_tail_retry_is_idempotent_but_stale_retry_conflicts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            start = self._observation(0, "session_started")
            self._append(root, journal, start, name="start")
            retried = self._append(root, journal, start, name="start-retry")
            self.assertEqual(retried["adapter_status"], "already_recorded")
            self.assertEqual(len(journal.read_text(encoding="utf-8").splitlines()), 1)
            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            stale = self._append(
                root,
                journal,
                start,
                name="stale-start",
                expected=3,
            )
            self.assertEqual(stale["adapter_status"], "conflict")

    def test_source_identity_change_is_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            changed = self._observation(
                1,
                "turn_started",
                turn_id="TURN-001",
                session_id="SESSION-FIXTURE-OTHER",
            )
            receipt = self._append(
                root,
                journal,
                changed,
                name="identity-change",
                expected=3,
            )
            self.assertIn("source identity", receipt["errors"][0])

    def test_source_record_sequence_allows_gaps_but_rejects_reordering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(
                    0,
                    "session_started",
                    source_record_sequence=2,
                ),
                name="start",
            )
            self._append(
                root,
                journal,
                self._observation(
                    1,
                    "turn_started",
                    turn_id="TURN-001",
                    source_record_sequence=7,
                ),
                name="turn",
            )
            reordered = self._append(
                root,
                journal,
                self._observation(
                    2,
                    "turn_completed",
                    turn_id="TURN-001",
                    turn_status="completed",
                    source_record_sequence=6,
                ),
                name="reordered",
                expected=3,
            )
            self.assertEqual(reordered["adapter_status"], "conflict")
            self.assertIn("strictly increasing", reordered["errors"][0])
            self.assertEqual(len(journal.read_text(encoding="utf-8").splitlines()), 2)

    def test_turn_start_is_rejected_after_journal_derived_completion(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self._complete_progress_state(journal)
            before = journal.read_bytes()
            receipt = self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="post-completion-turn",
                expected=3,
            )
            self.assertIn("after journal-derived task completion", receipt["errors"][0])
            self.assertEqual(journal.read_bytes(), before)

    def test_preflight_can_precede_session_start(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            append_event(
                journal,
                run_id="RUN-CLI-LIFECYCLE-001",
                run_manifest_sha256="a" * 64,
                event_type="run_preflight",
                payload_schema="fixture-preflight-v0.1",
                payload={"fixture": True},
            )
            receipt = self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self.assertEqual(receipt["canonical_event_type"], "run_started")
            self.assertEqual(receipt["journal_event_count"], 2)

    def test_verifier_detects_receipt_tampering_and_journal_advance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            self._append(
                root,
                journal,
                self._observation(0, "session_started"),
                name="start",
            )
            self.assertTrue(self._verify(root, journal, name="start")["valid"])
            receipt_path = root / "start.receipt.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            receipt["lifecycle_state_after"] = "in_turn"
            receipt_path.write_text(
                json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            tampered = self._verify(
                root,
                journal,
                name="start",
                output_name="start-tampered",
                expected=2,
            )
            self.assertFalse(tampered["checks"]["lifecycle_projection_verified"])

            self._append(
                root,
                journal,
                self._observation(1, "turn_started", turn_id="TURN-001"),
                name="turn",
            )
            stale = self._verify(
                root,
                journal,
                name="start",
                output_name="start-stale",
                expected=2,
            )
            self.assertFalse(stale["checks"]["journal_snapshot_matches"])
            self.assertFalse(stale["checks"]["event_presence_verified"])


if __name__ == "__main__":
    unittest.main()
