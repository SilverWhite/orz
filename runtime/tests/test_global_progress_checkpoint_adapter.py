from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
GPS_BUILDER = ROOT / "scripts" / "build_global_progress_review.py"
GPS_VERIFIER = ROOT / "scripts" / "verify_global_progress_review.py"
CHECKPOINT_BUILDER = ROOT / "scripts" / "build_global_progress_checkpoint.py"
CHECKPOINT_VERIFIER = ROOT / "scripts" / "verify_global_progress_checkpoint.py"
HOLISTIC_BUILDER = ROOT / "scripts" / "build_global_progress_holistic_review.py"
HOLISTIC_VERIFIER = ROOT / "scripts" / "verify_global_progress_holistic_review.py"
FIXTURE = ROOT / "runtime" / "fixtures" / "global-progress-sentinel-v0.1" / "input.json"
POLICY = (
    ROOT
    / "runtime"
    / "fixtures"
    / "global-progress-checkpoint-adapter-v0.1"
    / "policy.json"
)


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class GlobalProgressCheckpointAdapterTests(unittest.TestCase):
    def _run(
        self,
        *arguments: str,
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *arguments],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
            check=False,
        )

    def _write(self, root: Path, name: str, value: dict) -> Path:
        path = root / name
        path.write_text(
            json.dumps(value, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return path

    def _legacy_bundle(
        self,
        root: Path,
        source: dict,
        label: str,
    ) -> dict[str, Path | dict]:
        input_path = self._write(root, f"{label}-input.json", source)
        review_path = root / f"{label}-review.json"
        completed = self._run(
            str(GPS_BUILDER),
            "--input",
            str(input_path),
            "--output",
            str(review_path),
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        review = json.loads(review_path.read_text(encoding="utf-8"))
        warning_dispositions = [
            {
                "warning_id": warning["warning_id"],
                "status": (
                    "reasoned_continue"
                    if warning["kind"] == "direction_concentration"
                    else "accepted"
                ),
            }
            for warning in review["warnings"]
        ]
        disposition = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-disposition",
            "disposition_id": f"GPD-{label.upper()}",
            "review_id": review["review_id"],
            "review_sha256": _digest(review_path),
            "decision": "continue",
            "selected_step_id": "STEP-ACP-02",
            "acceptance_refs": ["ACC-ACP"],
            "warning_dispositions": warning_dispositions,
            "verification_before_claim": ["refresh ACP verification"],
            "next_review_condition": "At the next checkpoint.",
            "rationale_summary": "Continue the bounded no-model checkpoint fixture.",
        }
        disposition_path = self._write(
            root, f"{label}-disposition.json", disposition
        )
        verification_path = root / f"{label}-verification.json"
        completed = self._run(
            str(GPS_VERIFIER),
            "--input",
            str(input_path),
            "--review",
            str(review_path),
            "--disposition",
            str(disposition_path),
            "--output",
            str(verification_path),
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        return {
            "input": input_path,
            "review": review_path,
            "review_value": review,
            "disposition": disposition_path,
            "verification": verification_path,
        }

    def _checkpoint(
        self,
        root: Path,
        bundle: dict[str, Path | dict],
        label: str,
        *,
        previous: Path | None = None,
        focus: dict[str, Path] | None = None,
        expected_returncode: int = 0,
    ) -> tuple[Path, dict, subprocess.CompletedProcess[str]]:
        output = root / f"{label}-checkpoint.json"
        arguments = [
            str(CHECKPOINT_BUILDER),
            "--input",
            str(bundle["input"]),
            "--review",
            str(bundle["review"]),
            "--disposition",
            str(bundle["disposition"]),
            "--verification",
            str(bundle["verification"]),
            "--policy",
            str(POLICY),
        ]
        if previous:
            arguments.extend(["--previous-checkpoint", str(previous)])
        if focus:
            for name in (
                "focus-history",
                "focus-review",
                "focus-disposition",
                "focus-verification",
            ):
                arguments.extend([f"--{name}", str(focus[name])])
        arguments.extend(["--output", str(output)])
        completed = self._run(*arguments)
        self.assertEqual(completed.returncode, expected_returncode, completed.stderr)
        return (
            output,
            json.loads(output.read_text(encoding="utf-8")),
            completed,
        )

    def _verify_checkpoint(
        self,
        root: Path,
        bundle: dict[str, Path | dict],
        checkpoint: Path,
        label: str,
        *,
        previous: Path | None = None,
        focus: dict[str, Path] | None = None,
    ) -> tuple[subprocess.CompletedProcess[str], dict]:
        output = root / f"{label}-checkpoint-verification.json"
        arguments = [
            str(CHECKPOINT_VERIFIER),
            "--input",
            str(bundle["input"]),
            "--review",
            str(bundle["review"]),
            "--disposition",
            str(bundle["disposition"]),
            "--verification",
            str(bundle["verification"]),
            "--policy",
            str(POLICY),
            "--checkpoint",
            str(checkpoint),
        ]
        if previous:
            arguments.extend(["--previous-checkpoint", str(previous)])
        if focus:
            for name in (
                "focus-history",
                "focus-review",
                "focus-disposition",
                "focus-verification",
            ):
                arguments.extend([f"--{name}", str(focus[name])])
        arguments.extend(["--output", str(output)])
        completed = self._run(*arguments)
        return completed, json.loads(output.read_text(encoding="utf-8"))

    def _source(self, cycle: int, extra_actions: int) -> dict:
        source = json.loads(FIXTURE.read_text(encoding="utf-8"))
        source["plan"]["review_cycle"] = cycle
        next_sequence = source["journal"][-1]["sequence"] + 1
        for offset in range(extra_actions):
            sequence = next_sequence + offset
            source["journal"].append(
                {
                    "sequence": sequence,
                    "event_id": f"EVT-{sequence:03d}",
                    "event_type": "action_terminal",
                    "step_id": "STEP-ACP-02",
                    "direction_id": "runtime_acp",
                    "terminal_state": "succeeded",
                    "write_effect": False,
                    "verification_state": None,
                }
            )
        return source

    def _focus_chain(
        self,
        root: Path,
    ) -> tuple[Path, dict[str, Path]]:
        first_bundle = self._legacy_bundle(root, self._source(4, 0), "cycle4")
        first_path, _, _ = self._checkpoint(root, first_bundle, "cycle4")
        second_bundle = self._legacy_bundle(root, self._source(5, 1), "cycle5")
        second_path, second, _ = self._checkpoint(
            root, second_bundle, "cycle5", previous=first_path
        )
        first = json.loads(first_path.read_text(encoding="utf-8"))
        history = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-history-input",
            "task_id": second["task_id"],
            "task_contract_sha256": second["task_contract_sha256"],
            "completion_requested": False,
            "thresholds": {
                "history_window_checkpoints": 2,
                "direction_dominance_ratio": 0.7,
                "direction_dominance_min_actions": 2,
                "stagnation_checkpoints": 2,
                "deferral_debt_checkpoints": 2,
            },
            "checkpoints": [first, second],
        }
        history_path = self._write(root, "focus-history.json", history)
        assessment_path = root / "focus-review.json"
        completed = self._run(
            str(HOLISTIC_BUILDER),
            "--input",
            str(history_path),
            "--output",
            str(assessment_path),
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        assessment = json.loads(assessment_path.read_text(encoding="utf-8"))
        warning_dispositions = [
            {
                "warning_id": warning["warning_id"],
                "status": (
                    "reasoned_continue"
                    if warning["kind"]
                    in {"direction_budget_dominance", "evidence_stagnation"}
                    else "accepted"
                ),
            }
            for warning in assessment["warnings"]
        ]
        disposition = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-holistic-disposition",
            "disposition_id": "GHD-ADAPTER-FOCUS",
            "assessment_id": assessment["assessment_id"],
            "assessment_sha256": _digest(assessment_path),
            "decision": "continue",
            "selected_direction_id": "runtime_acp",
            "warning_dispositions": warning_dispositions,
            "bounded_focus": {
                "direction_id": "runtime_acp",
                "critical_path_ref": "ACC-ACP",
                "acceptance_refs": ["ACC-ACP"],
                "exit_condition": "Stop after two additional actions.",
                "max_additional_actions": 2,
                "review_by_cycle": 6,
                "deferred_direction_ids": [
                    "evidence_gates",
                    "windows_containment",
                ],
            },
            "completion_acknowledgement": "not_requested",
            "next_review_condition": "At cycle 6.",
            "rationale_summary": "Allow at most two source-observed runtime actions.",
        }
        disposition_path = self._write(
            root, "focus-disposition.json", disposition
        )
        verification_path = root / "focus-verification.json"
        completed = self._run(
            str(HOLISTIC_VERIFIER),
            "--input",
            str(history_path),
            "--review",
            str(assessment_path),
            "--disposition",
            str(disposition_path),
            "--output",
            str(verification_path),
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        return second_path, {
            "focus-history": history_path,
            "focus-review": assessment_path,
            "focus-disposition": disposition_path,
            "focus-verification": verification_path,
        }

    def test_first_checkpoint_is_derived_and_independently_rebuilt(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self._legacy_bundle(root, self._source(4, 0), "first")
            checkpoint_path, checkpoint, _ = self._checkpoint(
                root, bundle, "first"
            )
            self.assertEqual(checkpoint["direction_action_counts"][-2], {
                "direction_id": "runtime_acp",
                "action_count": 3,
            })
            self.assertEqual(checkpoint["novel_evidence_refs"], ["EVT-042"])
            self.assertIsNone(checkpoint["focus_window_consumption"])
            completed, report = self._verify_checkpoint(
                root, bundle, checkpoint_path, "first"
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])
            self.assertTrue(all(report["checks"].values()))

    def test_source_verification_digest_tampering_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self._legacy_bundle(root, self._source(4, 0), "digest")
            verification = json.loads(
                Path(bundle["verification"]).read_text(encoding="utf-8")
            )
            verification["review_sha256"] = "0" * 64
            bundle["verification"] = self._write(
                root, "digest-forged-verification.json", verification
            )
            output = root / "digest-refused.json"
            completed = self._run(
                str(CHECKPOINT_BUILDER),
                "--input",
                str(bundle["input"]),
                "--review",
                str(bundle["review"]),
                "--disposition",
                str(bundle["disposition"]),
                "--verification",
                str(bundle["verification"]),
                "--policy",
                str(POLICY),
                "--output",
                str(output),
            )
            self.assertEqual(completed.returncode, 3)
            self.assertIn(
                "verification does not match independent recomputation",
                completed.stderr,
            )
            self.assertFalse(output.exists())

    def test_checkpoint_tampering_fails_independent_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            bundle = self._legacy_bundle(root, self._source(4, 0), "tamper")
            checkpoint_path, checkpoint, _ = self._checkpoint(
                root, bundle, "tamper"
            )
            checkpoint["direction_action_counts"][-2]["action_count"] = 30
            material = {
                key: value
                for key, value in checkpoint.items()
                if key != "checkpoint_sha256"
            }
            checkpoint["checkpoint_sha256"] = hashlib.sha256(
                json.dumps(
                    material,
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                ).encode("utf-8")
            ).hexdigest()
            checkpoint_path.write_text(
                json.dumps(checkpoint, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            completed, report = self._verify_checkpoint(
                root, bundle, checkpoint_path, "tamper"
            )
            self.assertEqual(completed.returncode, 2)
            self.assertTrue(report["checks"]["checkpoint_hash_valid"])
            self.assertFalse(report["checks"]["checkpoint_rebuilt_exactly"])

    def test_critical_direction_policy_cannot_change_silently(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            first_bundle = self._legacy_bundle(
                root, self._source(4, 0), "policy-first"
            )
            previous, _, _ = self._checkpoint(
                root, first_bundle, "policy-first"
            )
            current_bundle = self._legacy_bundle(
                root, self._source(5, 1), "policy-current"
            )
            policy = json.loads(POLICY.read_text(encoding="utf-8"))
            policy["critical_directions"] = policy["critical_directions"][:-1]
            changed_policy = self._write(root, "changed-policy.json", policy)
            output = root / "changed-policy-checkpoint.json"
            completed = self._run(
                str(CHECKPOINT_BUILDER),
                "--input",
                str(current_bundle["input"]),
                "--review",
                str(current_bundle["review"]),
                "--disposition",
                str(current_bundle["disposition"]),
                "--verification",
                str(current_bundle["verification"]),
                "--policy",
                str(changed_policy),
                "--previous-checkpoint",
                str(previous),
                "--output",
                str(output),
            )
            self.assertEqual(completed.returncode, 3)
            self.assertIn("policy changed without an explicit revision", completed.stderr)
            self.assertFalse(output.exists())

    def test_bounded_focus_consumption_at_limit_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous, focus = self._focus_chain(root)
            bundle = self._legacy_bundle(root, self._source(6, 3), "within")
            checkpoint_path, checkpoint, _ = self._checkpoint(
                root,
                bundle,
                "within",
                previous=previous,
                focus=focus,
            )
            consumption = checkpoint["focus_window_consumption"]
            self.assertEqual(consumption["observed_actions"], 2)
            self.assertTrue(consumption["within_action_limit"])
            self.assertTrue(consumption["within_review_deadline"])
            completed, report = self._verify_checkpoint(
                root,
                bundle,
                checkpoint_path,
                "within",
                previous=previous,
                focus=focus,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])

    def test_bounded_focus_action_overrun_is_recorded_and_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous, focus = self._focus_chain(root)
            bundle = self._legacy_bundle(root, self._source(6, 4), "overrun")
            checkpoint_path, checkpoint, completed = self._checkpoint(
                root,
                bundle,
                "overrun",
                previous=previous,
                focus=focus,
                expected_returncode=2,
            )
            self.assertIn('"focus_window_within_bounds": false', completed.stdout)
            consumption = checkpoint["focus_window_consumption"]
            self.assertEqual(consumption["observed_actions"], 3)
            self.assertFalse(consumption["within_action_limit"])
            completed, report = self._verify_checkpoint(
                root,
                bundle,
                checkpoint_path,
                "overrun",
                previous=previous,
                focus=focus,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertTrue(report["checks"]["checkpoint_rebuilt_exactly"])
            self.assertFalse(report["checks"]["focus_window_within_bounds"])

    def test_failed_actions_also_consume_the_focus_budget(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous, focus = self._focus_chain(root)
            source = self._source(6, 4)
            for event in source["journal"]:
                if event["sequence"] > 46:
                    event["terminal_state"] = "failed"
            bundle = self._legacy_bundle(root, source, "failed-overrun")
            checkpoint_path, checkpoint, _ = self._checkpoint(
                root,
                bundle,
                "failed-overrun",
                previous=previous,
                focus=focus,
                expected_returncode=2,
            )
            consumption = checkpoint["focus_window_consumption"]
            self.assertEqual(consumption["observed_actions"], 3)
            self.assertFalse(consumption["within_action_limit"])
            completed, report = self._verify_checkpoint(
                root,
                bundle,
                checkpoint_path,
                "failed-overrun",
                previous=previous,
                focus=focus,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["focus_window_within_bounds"])

    def test_bounded_focus_deadline_overrun_blocks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous, focus = self._focus_chain(root)
            bundle = self._legacy_bundle(root, self._source(7, 2), "late")
            checkpoint_path, checkpoint, _ = self._checkpoint(
                root,
                bundle,
                "late",
                previous=previous,
                focus=focus,
                expected_returncode=2,
            )
            self.assertFalse(
                checkpoint["focus_window_consumption"]["within_review_deadline"]
            )
            completed, report = self._verify_checkpoint(
                root,
                bundle,
                checkpoint_path,
                "late",
                previous=previous,
                focus=focus,
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["focus_window_within_bounds"])


if __name__ == "__main__":
    unittest.main()
