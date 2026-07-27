from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO

from assurance.cli import main as gsa_main


ROOT = Path(__file__).resolve().parents[2]
SOURCE_LEDGER = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"
CREATED_AT = "2026-07-26T12:45:00Z"
ASK = "Check whether the source visibility gate permits this claim."


class GsaCliDispatcherTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "run"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _capture_json(self, argv: list[str]) -> tuple[int, dict[str, object]]:
        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(argv)
        return exit_code, json.loads(output.getvalue())

    def test_doctor_quick_json_reports_offline_boundaries(self) -> None:
        exit_code, report = self._capture_json(["doctor", "--quick", "--json"])

        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertEqual(report["report_kind"], "gsa_cli_doctor_report")
        self.assertIsNone(report["repository_check"])
        self.assertEqual(report["runtime_boundaries"]["default_network"], "disabled")
        self.assertIn("run_fake", report["entrypoints"])
        self.assertIn("run_real", report["entrypoints"])

    def test_source_gate_json_uses_visibility_receipt(self) -> None:
        exit_code, receipt = self._capture_json(
            ["source", "gate", "--ledger", str(SOURCE_LEDGER), "--json"]
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(receipt["receipt_kind"], "source_fulltext_visibility_gate_receipt")
        self.assertEqual(receipt["decision"], "defer")

    def test_run_and_verify_json_roundtrip(self) -> None:
        run_exit, run_receipt = self._capture_json(
            [
                "run",
                "--run-root",
                str(self.run_root),
                "--source-ledger",
                str(SOURCE_LEDGER),
                "--ask",
                ASK,
                "--created-at",
                CREATED_AT,
                "--json",
            ]
        )
        verify_exit, verify_receipt = self._capture_json(
            ["verify", "--run-root", str(self.run_root), "--json"]
        )

        self.assertEqual(run_exit, 0)
        self.assertEqual(verify_exit, 0)
        self.assertEqual(run_receipt, verify_receipt)
        self.assertEqual(run_receipt["terminal_event"], "run_finished")

    def test_run_reports_invalid_task_entry_mode_as_json_error(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root",
                    str(self.run_root),
                    "--source-ledger",
                    str(SOURCE_LEDGER),
                    "--created-at",
                    CREATED_AT,
                    "--json",
                ]
            )
        error = json.loads(error_output.getvalue())

        self.assertEqual(exit_code, 2)
        self.assertFalse(error["valid"])
        self.assertEqual(error["error_type"], "AssuranceError")
        self.assertIn("exactly one", error["error"])
        self.assertFalse(self.run_root.exists())

    def test_root_gsa_py_doctor_entrypoint(self) -> None:
        completed = subprocess.run(
            [sys.executable, "gsa.py", "doctor", "--quick", "--json"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )

        self.assertEqual(completed.returncode, 0, completed.stderr)
        report = json.loads(completed.stdout)
        self.assertEqual(report["report_kind"], "gsa_cli_doctor_report")
