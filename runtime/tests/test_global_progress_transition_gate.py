from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
BUILDER = ROOT / "scripts" / "build_global_progress_transition_event.py"
VERIFIER = ROOT / "scripts" / "verify_global_progress_transition_event.py"
HISTORY_FIXTURE = ROOT / "runtime" / "fixtures" / "global-progress-holistic-v0.1" / "input.json"


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


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

    def _request(
        self, root: Path, checkpoint: Path, verification: Path, *,
        kind: str = "step_boundary", current: str = "executing",
        requested: str = "reviewing", holistic: tuple[Path, Path] | None = None,
    ) -> Path:
        checkpoint_value = json.loads(checkpoint.read_text(encoding="utf-8"))
        review_path, holistic_verification_path = holistic or (None, None)
        review = json.loads(review_path.read_text(encoding="utf-8")) if review_path else None
        value = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-transition-request",
            "transition_id": "GPT-FIXTURE-001",
            "task_id": "TASK-GPS-HOLISTIC-001",
            "run_id": "RUN-GPS-TRANSITION-001",
            "sequence": 1,
            "timestamp": "2026-07-25T12:00:00Z",
            "run_manifest_sha256": "a" * 64,
            "previous_event_sha256": "b" * 64,
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
               holistic: tuple[Path, Path] | None = None, expected: int = 0) -> tuple[Path, dict]:
        event = root / "event.json"
        args = [str(BUILDER), "--request", str(request), "--checkpoint", str(checkpoint),
                "--checkpoint-verification", str(verification)]
        if holistic:
            args += ["--holistic-review", str(holistic[0]), "--holistic-verification", str(holistic[1])]
        args += ["--output", str(event)]
        completed = self._run(*args)
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return event, json.loads(event.read_text(encoding="utf-8"))

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


if __name__ == "__main__":
    unittest.main()
