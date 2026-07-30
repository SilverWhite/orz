from __future__ import annotations

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

import jsonschema


ROOT = Path(__file__).resolve().parents[3]
VERIFIER_PATH = ROOT / "scripts" / "verify_grok_timeout_gate_split.py"
SCHEMA_PATH = (
    ROOT
    / "integration"
    / "grok"
    / "grok-timeout-gate-split-verification-v0.1.schema.json"
)

SPEC = importlib.util.spec_from_file_location("timeout_gate_split", VERIFIER_PATH)
assert SPEC is not None and SPEC.loader is not None
VERIFIER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFIER)


def _write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


class TimeoutGateSplitVerifierTests(unittest.TestCase):
    def _diagnostic(self, version: str, pre_close_count: int = 4) -> dict[str, object]:
        return {
            "schema_version": "0.1.0",
            "diagnostic_kind": "grok-windows-child-tree-timeout",
            "probe_id": "CHILDTREE-" + "a" * 32,
            "scenario": "tool_timeout",
            "nonce": "LIFCHILD-" + "b" * 32,
            "elapsed_seconds": 127.0,
            "timeout_seconds": 90,
            "exit_grace_seconds": 30,
            "tool_timeout_ms": 3000,
            "grok": {"pid": 1000, "returncode_before_job_close": 0},
            "provider": {
                "pid": 1001,
                "returncode_before_job_close": None,
                "primary_request_count": 2,
                "tool_sequence": ["run_terminal_command"],
                "result_exists": True,
                "result": {"terminal_state": "succeeded"},
            },
            "process_tree": {
                "role_files_present": ["root.json", "child.json", "grandchild.json"],
                "before_job_close_processes": [
                    {
                        "pid": 2000 + index,
                        "parent_pid": 1000,
                        "name": "python.exe",
                        "creation_date": "",
                        "command_line_bytes": 100,
                        "command_line_sha256": "c" * 64,
                        "nonce_present": True,
                        "command_line_recorded": False,
                    }
                    for index in range(pre_close_count)
                ],
                "after_job_close_processes": [],
            },
            "capture": {"marker_projection": {}},
            "job": {
                "created": True,
                "assigned": True,
                "closed_before_diagnostic": False,
                "closed_after_diagnostic": True,
            },
            "diagnostic_boundary": f"fixture {version}",
        }

    def _summary_fixture(self, root: Path, candidate_pre_close_count: int = 4) -> Path:
        rows = []
        for version, pre_close_count in (
            ("0.2.111", 4),
            ("0.2.112", candidate_pre_close_count),
        ):
            output = root / f"{version}-tool-timeout"
            output.mkdir()
            _write_json(output / "timeout-diagnostic.json", self._diagnostic(version, pre_close_count))
            _write_json(output / "failure.json", {"error": "TimeoutError"})
            rows.append(
                {
                    "version": version,
                    "output_directory": str(output),
                    "stdout": str(root / f"{version}.stdout.txt"),
                    "stderr": str(root / f"{version}.stderr.txt"),
                    "started_at": "2026-07-30T00:00:00Z",
                    "finished_at": "2026-07-30T00:02:07Z",
                    "exit_code": 1,
                    "passed": False,
                    "result_exists": False,
                    "verification_exists": False,
                    "failure_exists": True,
                    "timeout_diagnostic_exists": True,
                }
            )
        summary = {
            "schema_version": "0.1.0",
            "receipt_kind": "grok-timeout-triage-matrix",
            "run_id": "fixture",
            "run_root": str(root),
            "created_at": "2026-07-30T00:00:00Z",
            "timeout_seconds": 90,
            "tool_timeout_ms": 3000,
            "exit_grace_seconds": 30,
            "all_passed": False,
            "results": rows,
        }
        path = root / "summary.json"
        _write_json(path, summary)
        return path

    def test_split_receipt_marks_baseline_passed_and_owned_cleanup_carried(self) -> None:
        schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            summary = self._summary_fixture(Path(temporary))
            receipt = VERIFIER.verify(summary, "0.2.111", "0.2.112")
            self.assertTrue(receipt["valid"])
            self.assertFalse(receipt["prompt_tool_promotion_ready"])
            self.assertEqual(
                receipt["gates"]["windows_child_tree_baseline_regression"]["status"],
                "passed",
            )
            self.assertEqual(
                receipt["gates"]["windows_child_tree_owned_cleanup"]["status"],
                "carried-limitation",
            )
            jsonschema.validate(receipt, schema)

    def test_candidate_with_more_pre_close_residue_fails_baseline_gate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            summary = self._summary_fixture(Path(temporary), candidate_pre_close_count=5)
            receipt = VERIFIER.verify(summary, "0.2.111", "0.2.112")
            self.assertTrue(receipt["valid"])
            self.assertEqual(
                receipt["gates"]["windows_child_tree_baseline_regression"]["status"],
                "failed",
            )
            self.assertFalse(
                receipt["gates"]["windows_child_tree_baseline_regression"]["checks"][
                    "candidate_pre_close_residue_not_greater"
                ]
            )


if __name__ == "__main__":
    unittest.main()
