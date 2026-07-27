from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from unittest.mock import patch

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

    def test_source_gate_summary_output(self) -> None:
        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                ["source", "gate", "--ledger", str(SOURCE_LEDGER), "--summary"]
            )
        text = output.getvalue()
        self.assertEqual(exit_code, 0)  # decision=defer is not block
        self.assertIn("source visibility gate:", text)

    def test_run_fails_on_nonempty_run_root(self) -> None:
        self.run_root.mkdir(parents=True)
        (self.run_root / "stale-file.txt").write_text("occupied", encoding="utf-8")
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root", str(self.run_root),
                    "--source-ledger", str(SOURCE_LEDGER),
                    "--ask", ASK,
                    "--created-at", CREATED_AT,
                    "--json",
                ]
            )
        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("empty or absent", error["error"])

    @patch("assurance.canonical_cli.build_real_deepseek_context")
    @patch("assurance.canonical_cli._read_windows_credential")
    @patch("assurance.canonical_cli.call_deepseek_api")
    def test_run_real_mode_mocked_api_success(
        self, mock_api, mock_cred, mock_context
    ) -> None:
        mock_cred.return_value = "sk-fake-key-0011223344556677"
        mock_context.return_value = [
            {"role": "system", "content": "Gate context."},
            {"role": "user", "content": ASK},
        ]
        mock_api.return_value = {
            "public_assistant_text": "The mechanism involves caspase-3 activation.",
            "finish_reason": "stop",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
            },
            "model": "deepseek-v4-pro",
            "private_reasoning_content_sha256": None,
            "http_status_code": 200,
        }
        exit_code, receipt = self._capture_json(
            [
                "run",
                "--mode", "real",
                "--run-root", str(self.run_root),
                "--source-ledger", str(SOURCE_LEDGER),
                "--ask", ASK,
                "--created-at", CREATED_AT,
                "--json",
            ]
        )
        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["terminal_event"], "run_finished")
        self.assertTrue(receipt["checks"]["real_network_used"])
        self.assertIn("real_network_used", receipt["checks"])
        # Verify that the mock API was called exactly once
        mock_cred.assert_called_once()
        mock_api.assert_called_once()

    @patch("assurance.canonical_cli._read_windows_credential")
    def test_run_real_mode_credential_failure_fallbacks(
        self, mock_cred
    ) -> None:
        mock_cred.side_effect = OSError("credential not found")
        exit_code, receipt = self._capture_json(
            [
                "run",
                "--mode", "real",
                "--run-root", str(self.run_root),
                "--source-ledger", str(SOURCE_LEDGER),
                "--ask", ASK,
                "--created-at", CREATED_AT,
                "--json",
            ]
        )
        # Real adapter failure produces a valid receipt with run_failed
        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["terminal_event"], "run_failed")

    def test_run_with_task_contract_file(self) -> None:
        task_contract_path = self.run_root.parent / "task-contract.json"
        task_contract = {
            "schema_version": "0.1.0-draft",
            "contract_kind": "canonical_cli_task_contract",
            "task_id": "TASK-CLI-TEST-002",
            "created_at": CREATED_AT,
            "entry_mode": "task_file",
            "user_request": {
                "raw_text": ASK,
                "normalized_intent": ASK.strip(),
            },
            "source_ledger": {
                "path": str(SOURCE_LEDGER),
                "sha256": "a" * 64,
                "required": True,
            },
            "output_contract": {
                "format": "canonical_cli_answer_packet",
                "must_include_source_visibility_summary": True,
                "must_include_claim_boundaries": True,
                "must_include_next_actions": True,
            },
            "permissions": {
                "network_allowed": False,
                "real_model_allowed": False,
                "tool_calls_allowed": False,
                "workspace_writes_allowed": False,
                "incremental_retrieval_allowed": True,
            },
            "claim_policy": {
                "allow_bibliographic_claims_without_fulltext": True,
                "require_fulltext_for_mechanism": True,
                "require_fulltext_for_methods": True,
            },
        }
        task_contract_path.write_text(
            json.dumps(task_contract), encoding="utf-8"
        )
        # Note: task_file entry_mode requires a real source ledger at the
        # same path, which we already have.  The task_contract validation
        # will check that the source ledger exists and matches.
        # Since our contract references a SHA that does not match the
        # actual ledger, we only test the --task dispatch path here.
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root", str(self.run_root),
                    "--task", str(task_contract_path),
                    "--created-at", CREATED_AT,
                    "--json",
                ]
            )
        # The sha256 mismatch will cause an AssuranceError
        error = json.loads(error_output.getvalue())
        self.assertEqual(exit_code, 2)
        self.assertFalse(error["valid"])
        self.assertEqual(error["error_type"], "AssuranceError")

    def test_doctor_full_repository_check(self) -> None:
        exit_code, report = self._capture_json(["doctor", "--json"])
        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertIsNotNone(report["repository_check"])
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
