from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
BUILDER = ROOT / "scripts" / "build_global_progress_review.py"
VERIFIER = ROOT / "scripts" / "verify_global_progress_review.py"
FIXTURE = ROOT / "runtime" / "fixtures" / "global-progress-sentinel-v0.1" / "input.json"
CONTINUE_DISPOSITION = FIXTURE.with_name("reasoned-continue.disposition.json")
REPLAN_DISPOSITION = FIXTURE.with_name("replan.disposition.json")


def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class GlobalProgressSentinelTests(unittest.TestCase):
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

    def _build(self, root: Path, input_path: Path = FIXTURE, name: str = "review.json") -> tuple[Path, dict]:
        review_path = root / name
        completed = self._run(
            str(BUILDER), "--input", str(input_path), "--output", str(review_path)
        )
        self.assertEqual(completed.returncode, 0, completed.stderr)
        return review_path, json.loads(review_path.read_text(encoding="utf-8"))

    def _disposition(
        self,
        root: Path,
        review_path: Path,
        review: dict,
        *,
        decision: str = "continue",
        missing_warning: bool = False,
        wrong_digest: bool = False,
        name: str = "disposition.json",
    ) -> Path:
        warning_dispositions = []
        for warning in review["warnings"]:
            status = "reasoned_continue" if warning["kind"] == "direction_concentration" else "accepted"
            warning_dispositions.append({"warning_id": warning["warning_id"], "status": status})
        if missing_warning:
            warning_dispositions = warning_dispositions[:-1]
        if decision == "replan":
            selected_step = None
            acceptance_refs = []
            warning_dispositions = [
                {"warning_id": warning["warning_id"], "status": "accepted"}
                for warning in review["warnings"]
            ]
        else:
            selected_step = "STEP-ACP-02"
            acceptance_refs = ["ACC-ACP"]
        disposition = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-disposition",
            "disposition_id": "GPD-FIXTURE-001",
            "review_id": review["review_id"],
            "review_sha256": "0" * 64 if wrong_digest else _sha256_file(review_path),
            "decision": decision,
            "selected_step_id": selected_step,
            "acceptance_refs": acceptance_refs,
            "warning_dispositions": warning_dispositions,
            "verification_before_claim": ["refresh ACP fixture verification"],
            "next_review_condition": "after the selected step or before an external action",
            "rationale_summary": "Continue the bounded fixture while preserving unresolved directions."
            if decision == "continue"
            else "Replan before any additional implementation direction is advanced.",
        }
        path = root / name
        path.write_text(json.dumps(disposition, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        return path

    def _verify(
        self,
        root: Path,
        input_path: Path,
        review_path: Path,
        disposition_path: Path,
        name: str,
    ) -> tuple[subprocess.CompletedProcess[str], dict]:
        output = root / name
        completed = self._run(
            str(VERIFIER),
            "--input", str(input_path),
            "--review", str(review_path),
            "--disposition", str(disposition_path),
            "--output", str(output),
        )
        return completed, json.loads(output.read_text(encoding="utf-8"))

    def test_builds_three_source_linked_warnings_and_six_directions(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, review = self._build(root)
            self.assertEqual(
                {warning["kind"] for warning in review["warnings"]},
                {"direction_concentration", "acceptance_uncovered", "verification_debt"},
            )
            self.assertEqual(len(review["direction_coverage"]), 6)
            self.assertTrue(all(warning["source_refs"] for warning in review["warnings"]))
            handoff = next(
                item for item in review["acceptance_coverage"] if item["acceptance_ref"] == "ACC-HANDOFF"
            )
            self.assertEqual(handoff["state"], "uncovered")
            self.assertEqual(review["objective"]["state"], "contract_derived")

    def test_reasoned_continue_and_replan_both_verify(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            review_path, review = self._build(root)
            completed, report = self._verify(
                root, FIXTURE, review_path, CONTINUE_DISPOSITION, "continue-verification.json"
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])
            self.assertTrue(all(report["checks"].values()))

            completed, report = self._verify(
                root, FIXTURE, review_path, REPLAN_DISPOSITION, "replan-verification.json"
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])

    def test_dedupe_suppresses_unchanged_warnings_without_losing_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, first = self._build(root, name="first.json")
            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            keys = [warning["dedupe_key"] for warning in first["warnings"]]
            source["previous_warning_dedupe_keys"] = keys
            deduped_input = root / "deduped-input.json"
            deduped_input.write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
            _, second = self._build(root, deduped_input, "second.json")
            self.assertEqual(second["warnings"], [])
            self.assertEqual(second["suppressed_warning_dedupe_keys"], sorted(keys))

    def test_plan_revision_and_journal_head_tampering_fail_rebuild(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            review_path, review = self._build(root)
            disposition = self._disposition(root, review_path, review)
            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            for label, mutate in (
                ("plan", lambda value: value["plan"].update({"revision": 4})),
                ("head", lambda value: value["journal"][-1].update({"event_id": "EVT-999"})),
            ):
                tampered = json.loads(json.dumps(source))
                mutate(tampered)
                input_path = root / f"{label}-input.json"
                input_path.write_text(json.dumps(tampered, indent=2) + "\n", encoding="utf-8")
                completed, report = self._verify(
                    root, input_path, review_path, disposition, f"{label}-verification.json"
                )
                self.assertEqual(completed.returncode, 2)
                self.assertFalse(report["valid"])
                self.assertFalse(report["checks"]["review_rebuilt_exactly"])

    def test_missing_warning_disposition_and_wrong_digest_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            review_path, review = self._build(root)
            missing = self._disposition(root, review_path, review, missing_warning=True)
            completed, report = self._verify(
                root, FIXTURE, review_path, missing, "missing-verification.json"
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["warning_dispositions_complete"])

            wrong = self._disposition(
                root, review_path, review, wrong_digest=True, name="wrong-digest.json"
            )
            completed, report = self._verify(
                root, FIXTURE, review_path, wrong, "wrong-digest-verification.json"
            )
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["review_linkage_valid"])

    def test_builder_refuses_overwrite_and_semantic_unknown_acceptance(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            review_path, _ = self._build(root)
            repeated = self._run(
                str(BUILDER), "--input", str(FIXTURE), "--output", str(review_path)
            )
            self.assertEqual(repeated.returncode, 2)
            self.assertIn("refusing to overwrite", repeated.stderr)

            source = json.loads(FIXTURE.read_text(encoding="utf-8"))
            source["plan"]["steps"][0]["acceptance_refs"] = ["ACC-UNKNOWN"]
            invalid = root / "invalid-input.json"
            invalid.write_text(json.dumps(source, indent=2) + "\n", encoding="utf-8")
            failed = self._run(
                str(BUILDER), "--input", str(invalid), "--output", str(root / "invalid-review.json")
            )
            self.assertEqual(failed.returncode, 2)
            self.assertIn("unknown acceptance", failed.stderr)

            contract_tampered = json.loads(FIXTURE.read_text(encoding="utf-8"))
            contract_tampered["task_contract"]["objective"] += " Tampered."
            contract_input = root / "contract-tampered.json"
            contract_input.write_text(
                json.dumps(contract_tampered, indent=2) + "\n", encoding="utf-8"
            )
            failed = self._run(
                str(BUILDER),
                "--input", str(contract_input),
                "--output", str(root / "contract-tampered-review.json"),
            )
            self.assertEqual(failed.returncode, 2)
            self.assertIn("task_contract sha256", failed.stderr)


if __name__ == "__main__":
    unittest.main()
