from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
BUILDER = ROOT / "scripts" / "build_global_progress_holistic_review.py"
VERIFIER = ROOT / "scripts" / "verify_global_progress_holistic_review.py"
FIXTURE = (
    ROOT / "runtime" / "fixtures" / "global-progress-holistic-v0.1" / "input.json"
)


def _canonical(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _rehash_checkpoints(source: dict) -> None:
    previous = None
    for checkpoint in source["checkpoints"]:
        checkpoint["previous_checkpoint_sha256"] = previous
        material = {
            key: value
            for key, value in checkpoint.items()
            if key != "checkpoint_sha256"
        }
        checkpoint["checkpoint_sha256"] = hashlib.sha256(
            _canonical(material)
        ).hexdigest()
        previous = checkpoint["checkpoint_sha256"]


class GlobalProgressHolisticReviewTests(unittest.TestCase):
    def _run(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *arguments],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=20,
            check=False,
        )

    def _build(
        self,
        root: Path,
        input_path: Path = FIXTURE,
        name: str = "assessment.json",
    ) -> tuple[Path, dict]:
        output = root / name
        completed = self._run(
            str(BUILDER), "--input", str(input_path), "--output", str(output)
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        return output, json.loads(output.read_text(encoding="utf-8"))

    def _write_source(self, root: Path, source: dict, name: str) -> Path:
        path = root / name
        path.write_text(
            json.dumps(source, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return path

    def _disposition(
        self,
        root: Path,
        assessment_path: Path,
        assessment: dict,
        *,
        bounded_focus: bool = True,
        completion_acknowledgement: str | None = None,
        name: str = "disposition.json",
    ) -> Path:
        warning_dispositions = []
        for warning in assessment["warnings"]:
            status = (
                "reasoned_continue"
                if warning["kind"]
                in {"direction_budget_dominance", "evidence_stagnation"}
                else "accepted"
            )
            warning_dispositions.append(
                {"warning_id": warning["warning_id"], "status": status}
            )
        gate_status = assessment["completion_gate"]["status"]
        expected_ack = {
            "not_requested": "not_requested",
            "eligible": "eligible",
            "ineligible": "completion_withheld",
        }[gate_status]
        disposition = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-holistic-disposition",
            "disposition_id": "GHD-FIXTURE-001",
            "assessment_id": assessment["assessment_id"],
            "assessment_sha256": _file_digest(assessment_path),
            "decision": "continue",
            "selected_direction_id": "runtime_acp",
            "warning_dispositions": warning_dispositions,
            "bounded_focus": {
                "direction_id": "runtime_acp",
                "critical_path_ref": "ACC-ACP",
                "acceptance_refs": ["ACC-ACP"],
                "exit_condition": "Stop after two actions or any new verifier result.",
                "max_additional_actions": 2,
                "review_by_cycle": 5,
                "deferred_direction_ids": ["evidence_gates"],
            }
            if bounded_focus
            else None,
            "completion_acknowledgement": completion_acknowledgement
            or expected_ack,
            "next_review_condition": "At cycle 5 or before any completion transition.",
            "rationale_summary": (
                "Continue only inside the bounded critical-path window; completion remains "
                "withheld until the global blockers are cleared."
            ),
        }
        path = root / name
        path.write_text(
            json.dumps(disposition, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        return path

    def _verify(
        self,
        root: Path,
        input_path: Path,
        assessment_path: Path,
        disposition_path: Path,
        name: str = "verification.json",
    ) -> tuple[subprocess.CompletedProcess[str], dict]:
        output = root / name
        completed = self._run(
            str(VERIFIER),
            "--input",
            str(input_path),
            "--review",
            str(assessment_path),
            "--disposition",
            str(disposition_path),
            "--output",
            str(output),
        )
        return completed, json.loads(output.read_text(encoding="utf-8"))

    def test_detects_four_cross_checkpoint_failure_modes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, assessment = self._build(root)
            self.assertEqual(
                {warning["kind"] for warning in assessment["warnings"]},
                {
                    "direction_budget_dominance",
                    "evidence_stagnation",
                    "critical_direction_deferral_debt",
                    "local_pass_global_incomplete",
                },
            )
            runtime = next(
                item
                for item in assessment["direction_budget"]
                if item["direction_id"] == "runtime_acp"
            )
            self.assertEqual(runtime["action_count"], 12)
            self.assertAlmostEqual(runtime["share"], 12 / 13, places=6)
            self.assertEqual(
                assessment["evidence_novelty"][
                    "same_direction_continue_without_novelty"
                ],
                4,
            )
            self.assertEqual(assessment["completion_gate"]["status"], "ineligible")
            self.assertEqual(
                assessment["completion_gate"]["blocker_refs"],
                ["ACC-EVIDENCE", "ACC-WINDOWS"],
            )

    def test_bounded_focus_is_a_valid_permission_reversal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assessment_path, assessment = self._build(root)
            disposition = self._disposition(root, assessment_path, assessment)
            completed, report = self._verify(
                root, FIXTURE, assessment_path, disposition
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])
            self.assertTrue(all(report["checks"].values()))

    def test_unbounded_reasoned_continue_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assessment_path, assessment = self._build(root)
            disposition = self._disposition(
                root, assessment_path, assessment, bounded_focus=False
            )
            completed, report = self._verify(
                root, FIXTURE, assessment_path, disposition
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["decision_semantics_valid"])
            self.assertIn(
                "reasoned_continue requires a bounded_focus control",
                report["errors"],
            )

    def test_checkpoint_tampering_breaks_the_hash_chain(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assessment_path, assessment = self._build(root)
            disposition = self._disposition(root, assessment_path, assessment)
            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            source["checkpoints"][1]["direction_action_counts"][1]["action_count"] = 40
            tampered = self._write_source(root, source, "tampered-input.json")

            refused = self._run(
                str(BUILDER),
                "--input",
                str(tampered),
                "--output",
                str(root / "refused.json"),
            )
            self.assertEqual(refused.returncode, 2)
            self.assertIn("checkpoint_sha256 mismatch", refused.stderr)

            completed, report = self._verify(
                root,
                tampered,
                assessment_path,
                disposition,
                "tampered-verification.json",
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["checkpoint_chain_valid"])
            self.assertFalse(report["checks"]["input_valid"])

    def test_novel_evidence_breaks_stagnation_without_hiding_dominance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            source["checkpoints"][3]["novel_evidence_refs"] = ["EVD-NEW-004"]
            _rehash_checkpoints(source)
            input_path = self._write_source(root, source, "novel-input.json")
            _, assessment = self._build(root, input_path)
            kinds = {warning["kind"] for warning in assessment["warnings"]}
            self.assertNotIn("evidence_stagnation", kinds)
            self.assertIn("direction_budget_dominance", kinds)
            self.assertEqual(
                assessment["evidence_novelty"][
                    "same_direction_continue_without_novelty"
                ],
                0,
            )

    def test_completion_gate_requires_exact_acknowledgement(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assessment_path, assessment = self._build(root)
            disposition = self._disposition(
                root,
                assessment_path,
                assessment,
                completion_acknowledgement="eligible",
            )
            completed, report = self._verify(
                root, FIXTURE, assessment_path, disposition
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["completion_gate_acknowledged"])
            self.assertIn(
                "completion acknowledgement must be completion_withheld for gate status ineligible",
                report["errors"],
            )

    def test_assessment_tampering_fails_independent_reconstruction(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            assessment_path, assessment = self._build(root)
            assessment["direction_budget"][1]["share"] = 0.5
            assessment_path.write_text(
                json.dumps(assessment, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            disposition = self._disposition(root, assessment_path, assessment)
            completed, report = self._verify(
                root, FIXTURE, assessment_path, disposition
            )
            self.assertEqual(completed.returncode, 2)
            self.assertTrue(report["checks"]["assessment_schema_valid"])
            self.assertTrue(report["checks"]["assessment_linkage_valid"])
            self.assertFalse(report["checks"]["assessment_rebuilt_exactly"])

    def test_globally_verified_state_is_completion_eligible(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            latest = source["checkpoints"][-1]
            latest["deferred_direction_ids"] = []
            latest["active_direction_ids"] = ["runtime_acp"]
            for state in latest["critical_acceptance_states"]:
                state["state"] = "verified"
            _rehash_checkpoints(source)
            input_path = self._write_source(root, source, "eligible-input.json")
            _, assessment = self._build(root, input_path)
            self.assertEqual(assessment["completion_gate"]["status"], "eligible")
            self.assertEqual(assessment["completion_gate"]["blocker_refs"], [])
            self.assertNotIn(
                "local_pass_global_incomplete",
                {warning["kind"] for warning in assessment["warnings"]},
            )


if __name__ == "__main__":
    unittest.main()
