from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from prototype.fep_agent_proto.journal import append_event, replay_journal
from prototype.fep_agent_proto.journal_lock import exclusive_journal_lock


ROOT = Path(__file__).resolve().parents[2]
BUILDER = ROOT / "scripts" / "build_global_progress_transition_event.py"
VERIFIER = ROOT / "scripts" / "verify_global_progress_transition_event.py"
APPENDER = ROOT / "scripts" / "append_global_progress_transition_event.py"
CONTROLLER = ROOT / "scripts" / "run_global_progress_controller.py"
CONTROLLER_VERIFIER = ROOT / "scripts" / "verify_global_progress_controller_receipt.py"
HOLISTIC_BUILDER = ROOT / "scripts" / "build_global_progress_holistic_review.py"
HOLISTIC_VERIFIER = ROOT / "scripts" / "verify_global_progress_holistic_review.py"
HISTORY_FIXTURE = ROOT / "runtime" / "fixtures" / "global-progress-holistic-v0.1" / "input.json"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()

def _canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


class GlobalProgressTransitionGateTests(unittest.TestCase):
    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *args], stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
            timeout=30, check=False,
        )

    def _write(self, root: Path, name: str, value: dict) -> Path:
        path = root / name
        path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        return path

    def _sources(self, root: Path, *, checkpoint_valid: bool = True) -> tuple[Path, Path]:
        history = json.loads(HISTORY_FIXTURE.read_text(encoding="utf-8"))
        checkpoint = history["checkpoints"][-1]
        checkpoint["task_id"] = history["task_id"]
        checkpoint_path = self._write(root, "checkpoint.json", checkpoint)
        checks = {
            "source_bundle_valid": checkpoint_valid,
            "source_artifact_digests_valid": checkpoint_valid,
            "previous_checkpoint_valid": checkpoint_valid,
            "focus_artifact_bundle_valid": checkpoint_valid,
            "checkpoint_schema_valid": checkpoint_valid,
            "checkpoint_rebuilt_exactly": checkpoint_valid,
            "checkpoint_hash_valid": checkpoint_valid,
            "focus_window_within_bounds": checkpoint_valid,
        }
        report = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-checkpoint-verification",
            "valid": checkpoint_valid,
            "checkpoint_id": checkpoint["checkpoint_id"],
            "input_sha256": "1" * 64,
            "review_sha256": "2" * 64,
            "disposition_sha256": "3" * 64,
            "checkpoint_sha256": _digest(checkpoint_path),
            "checks": checks,
            "errors": [] if checkpoint_valid else ["focus window exceeded"],
        }
        return checkpoint_path, self._write(root, "checkpoint-verification.json", report)

    def _holistic_sources(self, root: Path, *, eligible: bool) -> tuple[Path, Path, dict[str, Path]]:
        history = json.loads(HISTORY_FIXTURE.read_text(encoding="utf-8"))
        latest = history["checkpoints"][-1]
        latest["task_id"] = history["task_id"]
        if eligible:
            latest["active_direction_ids"] = ["runtime_acp"]
            latest["deferred_direction_ids"] = []
            latest["unresolved_constraint_refs"] = []
            for item in latest["critical_acceptance_states"]:
                item["state"] = "verified"
        previous = None
        for checkpoint in history["checkpoints"]:
            checkpoint["previous_checkpoint_sha256"] = previous
            material = {key: value for key, value in checkpoint.items() if key != "checkpoint_sha256"}
            checkpoint["checkpoint_sha256"] = hashlib.sha256(_canonical(material)).hexdigest()
            previous = checkpoint["checkpoint_sha256"]
        history_path = self._write(root, "holistic-history.json", history)
        review_path = root / "holistic-review.json"
        completed = self._run(str(HOLISTIC_BUILDER), "--input", str(history_path), "--output", str(review_path))
        self.assertEqual(completed.returncode, 0, completed.stderr)
        review = json.loads(review_path.read_text(encoding="utf-8"))
        disposition = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-holistic-disposition",
            "disposition_id": "GHD-TRANSITION-FIXTURE",
            "assessment_id": review["assessment_id"],
            "assessment_sha256": _digest(review_path),
            "decision": "replan",
            "selected_direction_id": None,
            "warning_dispositions": [
                {"warning_id": warning["warning_id"], "status": "accepted"}
                for warning in review["warnings"]
            ],
            "bounded_focus": None,
            "completion_acknowledgement": "eligible" if eligible else "completion_withheld",
            "next_review_condition": "Before any later transition.",
            "rationale_summary": "Fixture disposition for completion transition integration.",
        }
        disposition_path = self._write(root, "holistic-disposition.json", disposition)
        holistic_verification = root / "holistic-verification.json"
        completed = self._run(
            str(HOLISTIC_VERIFIER), "--input", str(history_path), "--review", str(review_path),
            "--disposition", str(disposition_path), "--output", str(holistic_verification),
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        checkpoint_path = self._write(root, "checkpoint.json", history["checkpoints"][-1])
        checkpoint = history["checkpoints"][-1]
        checks = {name: True for name in (
            "source_bundle_valid", "source_artifact_digests_valid", "previous_checkpoint_valid",
            "focus_artifact_bundle_valid", "checkpoint_schema_valid", "checkpoint_rebuilt_exactly",
            "checkpoint_hash_valid", "focus_window_within_bounds",
        )}
        report = {
            "schema_version": "0.1.0", "artifact_kind": "global-progress-checkpoint-verification",
            "valid": True, "checkpoint_id": checkpoint["checkpoint_id"],
            "input_sha256": "1" * 64, "review_sha256": "2" * 64,
            "disposition_sha256": "3" * 64, "checkpoint_sha256": _digest(checkpoint_path),
            "checks": checks, "errors": [],
        }
        checkpoint_verification = self._write(root, "checkpoint-verification.json", report)
        return checkpoint_path, checkpoint_verification, {
            "history": history_path, "review": review_path,
            "disposition": disposition_path, "verification": holistic_verification,
        }

    def _request(
        self, root: Path, checkpoint: Path, verification: Path, *,
        kind: str = "step_boundary", current: str = "executing",
        requested: str = "reviewing", holistic: dict[str, Path] | None = None,
        sequence: int = 1, previous_event_sha256: str | None = "b" * 64,
    ) -> Path:
        checkpoint_value = json.loads(checkpoint.read_text(encoding="utf-8"))
        review_path = holistic["review"] if holistic else None
        holistic_verification_path = holistic["verification"] if holistic else None
        review = json.loads(review_path.read_text(encoding="utf-8")) if review_path else None
        value = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-transition-request",
            "transition_id": "GPT-FIXTURE-001",
            "task_id": "TASK-GPS-HOLISTIC-001",
            "run_id": "RUN-GPS-TRANSITION-001",
            "sequence": sequence,
            "timestamp": "2026-07-25T12:00:00Z",
            "run_manifest_sha256": "a" * 64,
            "previous_event_sha256": previous_event_sha256,
            "transition_kind": kind,
            "current_state": current,
            "requested_state": requested,
            "checkpoint_id": checkpoint_value["checkpoint_id"],
            "checkpoint_file_sha256": _digest(checkpoint),
            "checkpoint_verification_sha256": _digest(verification),
            "holistic_assessment_id": review["assessment_id"] if review else None,
            "holistic_review_sha256": _digest(review_path) if review_path else None,
            "holistic_verification_sha256": _digest(holistic_verification_path) if holistic_verification_path else None,
        }
        return self._write(root, "request.json", value)

    def _build(self, root: Path, request: Path, checkpoint: Path, verification: Path, *,
               holistic: dict[str, Path] | None = None, expected: int = 0) -> tuple[Path, dict]:
        event = root / "event.json"
        args = [str(BUILDER), "--request", str(request), "--checkpoint", str(checkpoint),
                "--checkpoint-verification", str(verification)]
        if holistic:
            args += [
                "--holistic-history", str(holistic["history"]),
                "--holistic-review", str(holistic["review"]),
                "--holistic-disposition", str(holistic["disposition"]),
                "--holistic-verification", str(holistic["verification"]),
            ]
        args += ["--output", str(event)]
        completed = self._run(*args)
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return event, json.loads(event.read_text(encoding="utf-8"))

    def _control(
        self,
        root: Path,
        request: Path,
        checkpoint: Path,
        verification: Path,
        journal: Path,
        *,
        holistic: dict[str, Path] | None = None,
        output_name: str = "controller-receipt.json",
        expected: int = 0,
    ) -> tuple[subprocess.CompletedProcess[str], dict]:
        output = root / output_name
        args = [
            str(CONTROLLER),
            "--request", str(request),
            "--checkpoint", str(checkpoint),
            "--checkpoint-verification", str(verification),
            "--journal", str(journal),
            "--output", str(output),
        ]
        if holistic:
            args += [
                "--holistic-history", str(holistic["history"]),
                "--holistic-review", str(holistic["review"]),
                "--holistic-disposition", str(holistic["disposition"]),
                "--holistic-verification", str(holistic["verification"]),
            ]
        completed = self._run(*args)
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return completed, json.loads(output.read_text(encoding="utf-8"))

    def _verify_control(
        self,
        root: Path,
        request: Path,
        checkpoint: Path,
        verification: Path,
        journal: Path,
        receipt: Path,
        *,
        holistic: dict[str, Path] | None = None,
        output_name: str = "controller-verification.json",
        expected: int = 0,
    ) -> dict:
        output = root / output_name
        args = [
            str(CONTROLLER_VERIFIER),
            "--request", str(request),
            "--checkpoint", str(checkpoint),
            "--checkpoint-verification", str(verification),
            "--journal", str(journal),
            "--receipt", str(receipt),
            "--output", str(output),
        ]
        if holistic:
            args += [
                "--holistic-history", str(holistic["history"]),
                "--holistic-review", str(holistic["review"]),
                "--holistic-disposition", str(holistic["disposition"]),
                "--holistic-verification", str(holistic["verification"]),
            ]
        completed = self._run(*args)
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return json.loads(output.read_text(encoding="utf-8"))

    def test_valid_step_boundary_advances_state_and_verifies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification = self._sources(root)
            request = self._request(root, checkpoint, verification)
            event, value = self._build(root, request, checkpoint, verification)
            self.assertTrue(value["payload"]["transition_applied"])
            self.assertEqual(value["payload"]["state_after"], "reviewing")
            output = root / "event-verification.json"
            completed = self._run(str(VERIFIER), "--request", str(request), "--checkpoint", str(checkpoint),
                                  "--checkpoint-verification", str(verification), "--event", str(event),
                                  "--output", str(output))
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(json.loads(output.read_text(encoding="utf-8"))["valid"])

    def test_invalid_checkpoint_blocks_and_preserves_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification = self._sources(root, checkpoint_valid=False)
            request = self._request(root, checkpoint, verification)
            _, event = self._build(root, request, checkpoint, verification, expected=2)
            self.assertFalse(event["payload"]["transition_applied"])
            self.assertEqual(event["payload"]["state_after"], "executing")
            self.assertEqual(event["payload"]["control_codes"], ["GPS-FOCUS-WINDOW-EXCEEDED"])

    def test_disallowed_state_pair_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification = self._sources(root)
            request = self._request(root, checkpoint, verification, current="reviewing")
            _, event = self._build(root, request, checkpoint, verification, expected=2)
            self.assertEqual(event["payload"]["control_codes"], ["GPS-TRANSITION-NOT-ALLOWED"])
            self.assertEqual(event["payload"]["state_after"], "reviewing")

    def test_completion_requires_holistic_artifacts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification = self._sources(root)
            request = self._request(root, checkpoint, verification, kind="task_completion",
                                    current="reviewing", requested="completed")
            completed = self._run(str(BUILDER), "--request", str(request), "--checkpoint", str(checkpoint),
                                  "--checkpoint-verification", str(verification), "--output", str(root / "event.json"))
            self.assertEqual(completed.returncode, 3)
            self.assertIn("requires holistic", completed.stderr)

    def test_ineligible_completion_blocks_and_preserves_reviewing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification, holistic = self._holistic_sources(root, eligible=False)
            request = self._request(root, checkpoint, verification, kind="task_completion",
                                    current="reviewing", requested="completed", holistic=holistic)
            _, event = self._build(root, request, checkpoint, verification, holistic=holistic, expected=2)
            self.assertEqual(event["payload"]["control_codes"], ["GPS-COMPLETION-INELIGIBLE"])
            self.assertEqual(event["payload"]["state_after"], "reviewing")

    def test_eligible_completion_advances_to_completed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            checkpoint, verification, holistic = self._holistic_sources(root, eligible=True)
            request = self._request(root, checkpoint, verification, kind="task_completion",
                                    current="reviewing", requested="completed", holistic=holistic)
            event_path, event = self._build(root, request, checkpoint, verification, holistic=holistic)
            self.assertEqual(event["payload"]["control_codes"], ["GPS-TRANSITION-PASS"])
            self.assertTrue(event["payload"]["transition_applied"])
            self.assertEqual(event["payload"]["state_after"], "completed")
            output = root / "completion-event-verification.json"
            completed = self._run(
                str(VERIFIER), "--request", str(request), "--checkpoint", str(checkpoint),
                "--checkpoint-verification", str(verification),
                "--holistic-history", str(holistic["history"]),
                "--holistic-review", str(holistic["review"]),
                "--holistic-disposition", str(holistic["disposition"]),
                "--holistic-verification", str(holistic["verification"]),
                "--event", str(event_path), "--output", str(output),
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)

    def test_transition_event_appends_once_and_replays(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "run-events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root, checkpoint, verification,
                sequence=1, previous_event_sha256=first["event_sha256"],
            )
            event_path, event = self._build(root, request, checkpoint, verification)
            command = [
                sys.executable,
                str(APPENDER),
                "--journal", str(journal),
                "--event", str(event_path),
                "--lock-timeout-seconds", "2",
            ]
            first_append = subprocess.Popen(
                command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
            )
            second_append = subprocess.Popen(
                command, cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE
            )
            first_stdout, first_stderr = first_append.communicate(timeout=10)
            second_stdout, second_stderr = second_append.communicate(timeout=10)
            results = [
                (first_append.returncode, first_stdout, first_stderr),
                (second_append.returncode, second_stdout, second_stderr),
            ]
            self.assertEqual(sorted(item[0] for item in results), [0, 2])
            refused = next(item for item in results if item[0] == 2)
            self.assertIn("does not extend journal", refused[2])
            replay = replay_journal(
                run_manifest_path=None,
                journal_path=journal,
                expected_manifest_sha256="a" * 64,
                expected_run_id="RUN-GPS-TRANSITION-001",
                require_terminal=False,
            )
            self.assertTrue(replay["valid"])
            self.assertEqual(replay["event_count"], 2)
            self.assertEqual(replay["last_event_sha256"], event["event_sha256"])

    def test_appender_times_out_while_cross_process_lock_is_held(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "run-events.jsonl"
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=0,
                previous_event_sha256=None,
            )
            event_path, _ = self._build(root, request, checkpoint, verification)
            with exclusive_journal_lock(journal):
                completed = self._run(
                    str(APPENDER),
                    "--journal", str(journal),
                    "--event", str(event_path),
                    "--lock-timeout-seconds", "0.05",
                )
            self.assertEqual(completed.returncode, 2)
            self.assertIn("timed out acquiring journal lock", completed.stderr)
            self.assertFalse(journal.exists())

    def test_appender_rejects_non_transition_run_event(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "run-events.jsonl"
            foreign = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            foreign_path = self._write(root, "foreign-event.json", foreign)
            empty_journal = root / "empty-run-events.jsonl"
            completed = self._run(
                str(APPENDER), "--journal", str(empty_journal), "--event", str(foreign_path)
            )
            self.assertEqual(completed.returncode, 2)
            self.assertIn("not a gate_decision", completed.stderr)
            self.assertFalse(empty_journal.exists())

    def test_controller_applies_once_and_exact_retry_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            _, receipt = self._control(
                root, request, checkpoint, verification, journal
            )
            self.assertEqual(receipt["controller_status"], "transition_applied")
            self.assertTrue(receipt["event_appended"])
            self.assertEqual(receipt["state_after"], "reviewing")
            _, repeated = self._control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                output_name="controller-retry.json",
            )
            self.assertEqual(repeated["controller_status"], "already_recorded")
            self.assertFalse(repeated["event_appended"])
            verified = self._verify_control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                root / "controller-retry.json",
            )
            self.assertTrue(verified["valid"])
            append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="artifact_registered",
                payload_schema="fixture-artifact-v0.1",
                payload={"fixture": "later-event"},
            )
            _, stale_retry = self._control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                output_name="controller-stale-retry.json",
                expected=3,
            )
            self.assertEqual(stale_retry["controller_status"], "conflict")
            stale_verification = self._verify_control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                root / "controller-retry.json",
                output_name="controller-stale-verification.json",
                expected=2,
            )
            self.assertFalse(stale_verification["valid"])
            self.assertFalse(
                stale_verification["checks"]["journal_snapshot_matches"]
            )
            replay = replay_journal(
                run_manifest_path=None,
                journal_path=journal,
                expected_manifest_sha256="a" * 64,
                expected_run_id="RUN-GPS-TRANSITION-001",
                require_terminal=False,
            )
            self.assertEqual(replay["event_count"], 3)

    def test_controller_records_block_without_advancing_state(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root, checkpoint_valid=False)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            _, receipt = self._control(
                root, request, checkpoint, verification, journal, expected=2
            )
            self.assertEqual(receipt["controller_status"], "transition_blocked")
            self.assertFalse(receipt["transition_applied"])
            self.assertTrue(receipt["event_appended"])
            self.assertEqual(receipt["state_before"], receipt["state_after"])

    def test_controller_requires_recovery_without_mutating_torn_journal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            with journal.open("ab") as handle:
                handle.write(b'{"torn":')
            before = _digest(journal)
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            _, receipt = self._control(
                root, request, checkpoint, verification, journal, expected=3
            )
            self.assertEqual(receipt["controller_status"], "recovery_required")
            self.assertEqual(receipt["recovery_classification"], "torn_tail_removed")
            self.assertFalse(receipt["event_appended"])
            self.assertEqual(_digest(journal), before)

    def test_controller_rejects_stale_head_as_conflict(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="artifact_registered",
                payload_schema="fixture-artifact-v0.1",
                payload={"fixture": "advanced-head"},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            _, receipt = self._control(
                root, request, checkpoint, verification, journal, expected=3
            )
            self.assertEqual(receipt["controller_status"], "conflict")
            self.assertFalse(receipt["event_appended"])
            self.assertEqual(receipt["journal_event_count"], 2)

    def test_controller_applies_eligible_completion(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification, holistic = self._holistic_sources(
                root, eligible=True
            )
            request = self._request(
                root,
                checkpoint,
                verification,
                kind="task_completion",
                current="reviewing",
                requested="completed",
                holistic=holistic,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            _, receipt = self._control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                holistic=holistic,
            )
            self.assertEqual(receipt["controller_status"], "transition_applied")
            self.assertEqual(receipt["state_after"], "completed")

    def test_controller_rejects_source_mismatch_without_journal_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            checkpoint_value = json.loads(checkpoint.read_text(encoding="utf-8"))
            checkpoint_value["task_id"] = "TASK-CONTROLLER-MISMATCH"
            checkpoint.write_text(
                json.dumps(checkpoint_value, ensure_ascii=False),
                encoding="utf-8",
            )
            _, receipt = self._control(
                root, request, checkpoint, verification, journal, expected=3
            )
            self.assertEqual(receipt["controller_status"], "rejected")
            self.assertFalse(receipt["event_appended"])
            replay = replay_journal(
                run_manifest_path=None,
                journal_path=journal,
                expected_manifest_sha256="a" * 64,
                expected_run_id="RUN-GPS-TRANSITION-001",
                require_terminal=False,
            )
            self.assertEqual(replay["event_count"], 1)

    def test_controller_rebuilds_receipt_after_post_append_crash_window(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            event_path, _ = self._build(root, request, checkpoint, verification)
            appended = self._run(
                str(APPENDER), "--journal", str(journal), "--event", str(event_path)
            )
            self.assertEqual(appended.returncode, 0, appended.stderr)
            _, receipt = self._control(
                root, request, checkpoint, verification, journal
            )
            self.assertEqual(receipt["controller_status"], "already_recorded")
            verified = self._verify_control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                root / "controller-receipt.json",
            )
            self.assertTrue(verified["valid"])

    def test_controller_verifier_detects_receipt_state_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            journal = root / "events.jsonl"
            first = append_event(
                journal,
                run_id="RUN-GPS-TRANSITION-001",
                run_manifest_sha256="a" * 64,
                event_type="run_started",
                payload_schema="fixture-run-started-v0.1",
                payload={"fixture": True},
            )
            checkpoint, verification = self._sources(root)
            request = self._request(
                root,
                checkpoint,
                verification,
                sequence=1,
                previous_event_sha256=first["event_sha256"],
            )
            self._control(root, request, checkpoint, verification, journal)
            receipt_path = root / "controller-receipt.json"
            tampered = json.loads(receipt_path.read_text(encoding="utf-8"))
            tampered["state_after"] = "completed"
            tampered_path = self._write(root, "tampered-controller-receipt.json", tampered)
            result = self._verify_control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                tampered_path,
                expected=2,
            )
            self.assertFalse(result["valid"])
            self.assertFalse(result["checks"]["state_projection_verified"])
            tampered["controller_status"] = "forged_status"
            forged_path = self._write(root, "forged-controller-receipt.json", tampered)
            malformed_result = self._verify_control(
                root,
                request,
                checkpoint,
                verification,
                journal,
                forged_path,
                output_name="forged-controller-verification.json",
                expected=2,
            )
            self.assertFalse(malformed_result["valid"])
            self.assertIsNone(malformed_result["controller_status"])
            self.assertFalse(
                malformed_result["checks"]["receipt_schema_valid"]
            )


if __name__ == "__main__":
    unittest.main()
