from __future__ import annotations

from pathlib import Path
import tempfile
import unittest

from assurance.global_review_mode import build_global_review_mode_receipt


ROOT = Path(__file__).resolve().parents[2]
CREATED_AT = "2026-07-30T08:30:00Z"


class GlobalReviewModeTests(unittest.TestCase):
    def test_explicit_paths_build_valid_global_review_receipt(self) -> None:
        receipt = build_global_review_mode_receipt(
            paths=[
                "CLI_PROJECT_INDEX.md",
                "assurance/global_review_mode.py",
                "assurance/tui/bridge.py",
                "assurance/tests/test_global_review_mode.py",
                "scripts/check_repository.py",
            ],
            include_git_status=False,
            repo_root=ROOT,
            review_id="GRM-TEST-001",
            created_at=CREATED_AT,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "global_review_mode_receipt")
        self.assertEqual(receipt["scope"]["source"], "explicit_paths")
        self.assertTrue(receipt["mode_activation"]["explicit_only"])
        self.assertEqual(receipt["mode_activation"]["ordinary_review_scope"], "local_engineering_review")
        self.assertEqual(len(receipt["dimensions"]), 5)
        dimension_ids = [item["dimension_id"] for item in receipt["dimensions"]]
        self.assertEqual(dimension_ids, [
            "design_intent_alignment",
            "current_progress_judgment",
            "implementation_content_positioning",
            "critical_design_preservation",
            "project_task_boundary",
        ])

        flags = {
            flag
            for finding in receipt["path_findings"]
            for flag in finding["risk_flags"]
        }
        self.assertIn("design_reference_changed", flags)
        self.assertIn("adapter_or_integration_changed", flags)
        self.assertIn("ui_surface_changed", flags)
        self.assertIn("critical_design_surface_changed", flags)
        self.assertIn("progress_status_changed", flags)
        self.assertIn("project_task_boundary_changed", flags)
        self.assertIn("implementation_surface_changed", flags)
        self.assertIn("test_surface_changed", flags)

    def test_empty_scope_still_requires_explicit_dimensions(self) -> None:
        receipt = build_global_review_mode_receipt(
            paths=[],
            include_git_status=False,
            repo_root=ROOT,
            review_id="GRM-TEST-EMPTY",
            created_at=CREATED_AT,
        )

        self.assertEqual(receipt["decision"], "ready_for_global_review_no_paths")
        self.assertEqual(receipt["scope"]["path_count"], 0)
        self.assertTrue(all(item["required"] for item in receipt["dimensions"]))
        self.assertEqual(receipt["path_findings"], [])

    def test_rejects_absolute_path_outside_workspace(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            outside = Path(temporary) / "outside.txt"
            outside.write_text("outside", encoding="utf-8")
            with self.assertRaises(Exception):
                build_global_review_mode_receipt(
                    paths=[outside],
                    include_git_status=False,
                    repo_root=ROOT,
                )
