from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts" / "new_grok_workspace_trust_receipt.ps1"
SCHEMA = ROOT / "integration" / "grok" / "grok-workspace-trust-receipt-v0.1.schema.json"
POWERSHELL = shutil.which("powershell") or shutil.which("pwsh")
RUNS_WINDOWS_POWERSHELL_FIXTURE = os.name == "nt" and POWERSHELL


@unittest.skipUnless(
    RUNS_WINDOWS_POWERSHELL_FIXTURE,
    "Windows PowerShell fixture is required",
)
class WorkspaceTrustReceiptTests(unittest.TestCase):
    def _run(self, workspace: Path, output: Path, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                POWERSHELL,
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                str(SCRIPT),
                "-WorkspacePath",
                str(workspace),
                "-OutputPath",
                str(output),
                "-ProjectRootPath",
                str(workspace),
                *extra,
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            encoding="utf-8",
            errors="replace",
            timeout=30,
            check=False,
        )

    def _load_and_validate(self, output: Path) -> dict[str, object]:
        receipt = json.loads(output.read_text(encoding="utf-8"))
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        errors = sorted(
            Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(receipt),
            key=lambda item: list(item.absolute_path),
        )
        self.assertEqual([], [error.message for error in errors])
        return receipt

    def test_empty_restricted_workspace_is_launchable_and_schema_valid(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            output = root / "receipt.json"
            completed = self._run(workspace, output)
            self.assertEqual(0, completed.returncode, completed.stderr)
            receipt = self._load_and_validate(output)
            self.assertEqual(0, receipt["discovery"]["candidate_count"])
            self.assertTrue(receipt["discovery"]["complete"])
            self.assertTrue(receipt["decision"]["launch_permitted"])
            self.assertFalse(receipt["safety"]["project_code_executed"])

    def test_restricted_workspace_records_controls_and_denies_launch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            (workspace / ".grok" / "hooks").mkdir(parents=True)
            (workspace / ".claude").mkdir()
            (workspace / "AGENTS.md").write_text("fixture\n", encoding="utf-8")
            (workspace / ".grok" / "config.toml").write_text(
                '[permission]\nrules = [{ action = "deny", tool = "edit" }]\n',
                encoding="utf-8",
            )
            (workspace / ".grok" / "hooks" / "start.json").write_text(
                "{}\n", encoding="utf-8"
            )
            (workspace / ".claude" / "settings.json").write_text(
                "{}\n", encoding="utf-8"
            )
            output = root / "receipt.json"
            completed = self._run(workspace, output)
            self.assertEqual(0, completed.returncode, completed.stderr)
            receipt = self._load_and_validate(output)
            self.assertEqual(4, receipt["discovery"]["candidate_count"])
            self.assertFalse(receipt["decision"]["launch_permitted"])
            surfaces = {item["surface"] for item in receipt["discovery"]["candidates"]}
            self.assertEqual(
                {
                    "project_instruction",
                    "grok_project_config",
                    "grok_hooks",
                    "claude_project_settings",
                },
                surfaces,
            )

    def test_trusted_receipt_requires_current_aggregate_and_actor(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            control = workspace / "AGENTS.md"
            control.write_text("fixture-v1\n", encoding="utf-8")
            restricted_output = root / "restricted.json"
            restricted = self._run(workspace, restricted_output)
            self.assertEqual(0, restricted.returncode, restricted.stderr)
            first = self._load_and_validate(restricted_output)
            aggregate = first["discovery"]["aggregate_sha256"]

            trusted_output = root / "trusted.json"
            trusted = self._run(
                workspace,
                trusted_output,
                "-Decision",
                "trusted",
                "-DecisionActor",
                "fixture-reviewer",
                "-ExpectedAggregateSha256",
                aggregate,
            )
            self.assertEqual(0, trusted.returncode, trusted.stderr)
            second = self._load_and_validate(trusted_output)
            self.assertTrue(second["decision"]["launch_permitted"])
            self.assertTrue(second["decision"]["explicit_digest_confirmation"])

            control.write_text("fixture-v2\n", encoding="utf-8")
            stale_output = root / "stale.json"
            stale = self._run(
                workspace,
                stale_output,
                "-Decision",
                "trusted",
                "-DecisionActor",
                "fixture-reviewer",
                "-ExpectedAggregateSha256",
                aggregate,
            )
            self.assertNotEqual(0, stale.returncode)
            self.assertFalse(stale_output.exists())
            self.assertIn("aggregate changed", stale.stderr)

    def test_existing_output_is_never_overwritten(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            output = root / "receipt.json"
            output.write_text("sentinel\n", encoding="utf-8")
            completed = self._run(workspace, output)
            self.assertNotEqual(0, completed.returncode)
            self.assertEqual("sentinel\n", output.read_text(encoding="utf-8"))
            self.assertIn("refusing to overwrite", completed.stderr)

    def test_checked_in_fixture_freezes_five_control_surfaces(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            shutil.copytree(
                ROOT / "integration" / "grok" / "fixtures" / "workspace-control-surfaces",
                workspace,
            )
            output = root / "receipt.json"
            completed = self._run(workspace, output)
            self.assertEqual(0, completed.returncode, completed.stderr)
            receipt = self._load_and_validate(output)
            self.assertEqual(5, receipt["discovery"]["candidate_count"])
            self.assertFalse(receipt["decision"]["launch_permitted"])
            self.assertEqual(
                {
                    "project_instruction",
                    "grok_project_config",
                    "grok_hooks",
                    "grok_skills",
                    "claude_project_settings",
                },
                {item["surface"] for item in receipt["discovery"]["candidates"]},
            )

    def test_scan_limit_emits_incomplete_receipt_and_denies_launch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            (workspace / ".grok").mkdir(parents=True)
            (workspace / "AGENTS.md").write_text("fixture\n", encoding="utf-8")
            (workspace / ".grok" / "config.toml").write_text(
                "[permission]\nrules = []\n", encoding="utf-8"
            )
            output = root / "receipt.json"
            completed = self._run(
                workspace,
                output,
                "-MaxCandidateFiles",
                "1",
            )
            self.assertEqual(2, completed.returncode, completed.stderr)
            receipt = self._load_and_validate(output)
            self.assertFalse(receipt["discovery"]["complete"])
            self.assertFalse(receipt["decision"]["launch_permitted"])
            self.assertFalse(receipt["valid"])
            self.assertEqual(
                ["candidate_file_limit_reached"],
                receipt["discovery"]["limit_reasons"],
            )


if __name__ == "__main__":
    unittest.main()
