from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from assurance import (
    MemoryInstallationKeyStore,
    load_readonly_task_projection,
    load_synthetic_user_task_suite,
    project_complex_task_readonly,
    run_synthetic_user_task_evaluation,
    verify_complex_task_readonly_projection,
    verify_synthetic_user_task_evaluation,
)
from assurance.utils import sha256_file


ROOT = Path(__file__).resolve().parents[2]


class P5SyntheticUserTaskTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.key_store = MemoryInstallationKeyStore(b"5" * 32)

    def tearDown(self) -> None:
        self.key_store.close()
        self.temporary.cleanup()

    def test_two_task_preflight_separates_mechanical_and_human_claims(
        self,
    ) -> None:
        suite = load_synthetic_user_task_suite()
        self.assertEqual(len(suite["tasks"]), 2)
        result = run_synthetic_user_task_evaluation(
            run_root=self.root / "evaluation",
            key_store=self.key_store,
        )
        receipt = result["receipt"]
        verification = result["verification"]
        self.assertTrue(verification["valid"], verification["errors"])
        self.assertEqual(
            receipt["status"], "mechanical_preflight_passed"
        )
        self.assertEqual(verification["verified_passes"], 2)
        self.assertEqual(receipt["environment"]["external_participant_count"], 0)
        self.assertFalse(receipt["claims"]["human_comprehension_assessed"])
        self.assertFalse(receipt["claims"]["human_usability_assessed"])
        self.assertFalse(receipt["claims"]["ordinary_user_safety_established"])
        self.assertFalse(receipt["claims"]["production_readiness_established"])

    def test_strict_task_creates_no_workspace_and_tampering_fails_closed(
        self,
    ) -> None:
        run_root = self.root / "evaluation"
        result = run_synthetic_user_task_evaluation(
            run_root=run_root,
            key_store=self.key_store,
        )
        strict = next(
            item
            for item in result["receipt"]["task_results"]
            if item["requested_mode"] == "strict"
        )
        strict_root = run_root / "tasks" / strict["task_id"]
        self.assertEqual(strict["observed_outcome"], "fail_closed")
        self.assertFalse((strict_root / "workspace").exists())
        self.assertFalse((strict_root / "conversations").exists())
        self.assertFalse(strict["execution"]["fallback_used"])
        self.assertFalse(strict["execution"]["docker_invoked"])

        tampered = deepcopy(result["receipt"])
        tampered["claims"]["human_comprehension_assessed"] = True
        verification = verify_synthetic_user_task_evaluation(
            tampered,
            run_root=run_root,
            key_store=self.key_store,
        )
        self.assertFalse(verification["valid"])

    def test_lif_task_projection_preserves_source_and_detects_snapshot_tamper(
        self,
    ) -> None:
        projection = load_readonly_task_projection()
        source_root = self.root / "lif-source"
        source_root.mkdir()
        for index, item in enumerate(projection["files"]):
            path = source_root / Path(*item["relative_path"].split("/"))
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(
                f"synthetic source {index}: {item['role']}\n",
                encoding="utf-8",
            )
        before = {
            item["relative_path"]: sha256_file(
                source_root / Path(*item["relative_path"].split("/"))
            )
            for item in projection["files"]
        }
        snapshot_root = self.root / "snapshot"
        result = project_complex_task_readonly(
            source_root=source_root,
            snapshot_root=snapshot_root,
            key_store=self.key_store,
        )
        self.assertTrue(
            result["verification"]["valid"],
            result["verification"]["errors"],
        )
        after = {
            item["relative_path"]: sha256_file(
                source_root / Path(*item["relative_path"].split("/"))
            )
            for item in projection["files"]
        }
        self.assertEqual(before, after)
        self.assertEqual(result["receipt"]["result"]["projected_file_count"], 8)
        self.assertFalse(result["receipt"]["safety"]["source_write_attempted"])
        self.assertFalse(result["receipt"]["safety"]["analysis_code_executed"])

        first = projection["files"][0]["relative_path"]
        snapshot = snapshot_root / Path(*first.split("/"))
        snapshot.write_text("tampered snapshot\n", encoding="utf-8")
        verification = verify_complex_task_readonly_projection(
            result["receipt"],
            source_root=source_root,
            snapshot_root=snapshot_root,
            key_store=self.key_store,
        )
        self.assertFalse(verification["valid"])


if __name__ == "__main__":
    unittest.main()
