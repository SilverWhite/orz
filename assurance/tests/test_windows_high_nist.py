from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker

from assurance.errors import AssuranceError
from assurance.sandbox_verifier import verify_windows_native_run_observation
from assurance.utils import sha256_file
from assurance.windows_sandbox import (
    WINDOWS_RUN_ARMS,
    WINDOWS_RUN_RESTRICTED_PRIVILEGES,
    _build_environment_block,
    _build_sandbox_env,
    _command_line_from,
    build_restricted_token_spec,
    run_observation_checks_for_arm,
)


ROOT = Path(__file__).resolve().parents[2]
ASSURANCE = ROOT / "assurance"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _schema_errors(instance: dict, schema_name: str) -> list[str]:
    validator = Draft202012Validator(
        _load(ASSURANCE / schema_name),
        format_checker=FormatChecker(),
    )
    return [error.message for error in validator.iter_errors(instance)]


def _valid_run_observation(arm: str) -> dict:
    """Minimal structurally valid run observation fixture for the arm."""
    profile = _load(ASSURANCE / "windows-native-sandbox-profile-v0.1.json")
    checks = dict.fromkeys(run_observation_checks_for_arm(arm), True)
    is_high = arm == "high-nist"
    is_nonadmin = arm == "non-admin"
    return {
        "schema_version": "0.1.0-draft",
        "observation_kind": "windows_native_sandbox_run_observation",
        "observation_id": "WNR-TEST-001",
        "created_at": "2026-01-01T00:00:00Z",
        "arm": arm,
        "profile_sha256": sha256_file(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        ),
        "workspace_path_sha256": "b" * 64,
        "command_sha256": "c" * 64,
        "command": "cmd /c echo hi",
        "cwd": "C:\\ws",
        "env_overrides": {},
        "temp_redirect": {
            "workspace_tmp": "C:\\ws\\.tmp",
            "temp_env": "C:\\ws\\.tmp" if is_high else "",
            "tmp_env": "C:\\ws\\.tmp" if is_high else "",
        },
        "token": {
            "restricted": is_nonadmin or is_high,
            "virtualization_allowed": not (is_nonadmin or is_high),
            "low_integrity": is_high,
            "appcontainer": is_high,
            "privileges_removed": is_nonadmin or is_high,
        },
        "appcontainer": {
            "sid_derived": is_high,
            "profile_created": False,
            "profile_deleted": False,
            "capabilities": [],
        },
        "job_object": {
            "created": True,
            "assigned": True,
            "creation_time_assignment": True,
            "kill_on_close": True,
            "memory_limit_bytes": int(profile["resources"]["memory_bytes"]),
            "active_process_limit": int(profile["resources"]["pids_limit"]),
        },
        "firewall": {
            "outbound_block_rule_created": is_high,
            "rule_name": "GSA-P2-Native-Sandbox-test" if is_high else "",
            "allowlist_ips": [] if is_high else [],
            "diagnostic": "test fixture",
        },
        "process": {
            "pid": 0,
            "exit_code": 0,
            "timed_out": False,
            "cancellation_method": None,
            "shell_used": False,
        },
        "output": {
            "stdout_bytes": 0,
            "stderr_bytes": 0,
            "stdout_sha256": "d" * 64,
            "stderr_sha256": "e" * 64,
        },
        "checks": checks,
        "outcome": "compliant" if arm == "control" or (is_nonadmin or is_high) else "noncompliant",
        "evidence_status": "observed",
        "diagnostics": [],
        "limitations": ["test fixture"],
    }


class RestrictedTokenSpecTests(unittest.TestCase):
    def test_control_spec_is_open(self) -> None:
        spec = build_restricted_token_spec("control")
        self.assertEqual(spec["disable_sids"], [])
        self.assertEqual(spec["deny_only_sids"], [])
        self.assertEqual(spec["remove_privileges"], [])
        self.assertFalse(spec["low_integrity"])
        self.assertFalse(spec["appcontainer"])
        self.assertTrue(spec["virtualization_allowed"])

    def test_non_admin_spec(self) -> None:
        spec = build_restricted_token_spec("non-admin")
        self.assertIn("S-1-5-32-544", spec["disable_sids"])
        self.assertIn("S-1-5-32-544", spec["deny_only_sids"])
        self.assertEqual(
            set(spec["remove_privileges"]),
            set(WINDOWS_RUN_RESTRICTED_PRIVILEGES),
        )
        self.assertFalse(spec["low_integrity"])
        self.assertFalse(spec["appcontainer"])
        self.assertFalse(spec["virtualization_allowed"])

    def test_high_nist_spec_adds_low_il_and_appcontainer(self) -> None:
        spec = build_restricted_token_spec("high-nist")
        self.assertTrue(spec["low_integrity"])
        self.assertTrue(spec["appcontainer"])
        self.assertFalse(spec["virtualization_allowed"])
        self.assertIn("SeCreateSymbolicLinkPrivilege", spec["remove_privileges"])

    def test_invalid_arm_raises(self) -> None:
        with self.assertRaises(AssuranceError):
            build_restricted_token_spec("bogus")


class RunObservationCheckSetTests(unittest.TestCase):
    def test_control_checks(self) -> None:
        self.assertEqual(run_observation_checks_for_arm("control"), ("workspace_writable",))

    def test_non_admin_checks(self) -> None:
        checks = run_observation_checks_for_arm("non-admin")
        self.assertEqual(
            set(checks),
            {"workspace_writable", "non_admin", "token_virtualization_disabled", "privileges_removed"},
        )

    def test_high_nist_checks(self) -> None:
        checks = run_observation_checks_for_arm("high-nist")
        self.assertEqual(
            set(checks),
            {
                "workspace_writable",
                "non_admin",
                "token_virtualization_disabled",
                "privileges_removed",
                "low_integrity",
                "appcontainer_token",
                "temp_redirected",
                "job_object_assigned",
            },
        )

    def test_invalid_arm_raises(self) -> None:
        with self.assertRaises(AssuranceError):
            run_observation_checks_for_arm("bogus")


class RunEnvironmentPureHelperTests(unittest.TestCase):
    def test_sandbox_env_high_nist_redirects_temp(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            workspace = Path(tmp)
            env = _build_sandbox_env(workspace, "high-nist", {"X": "y"})
            self.assertTrue(str(workspace / ".tmp").lower() == env["TEMP"].lower())
            self.assertTrue(str(workspace / ".tmp").lower() == env["TMP"].lower())
            self.assertTrue((workspace / ".tmp").is_dir())
            self.assertEqual(env["X"], "y")

    def test_sandbox_env_non_admin_keeps_temp(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            env = _build_sandbox_env(Path(tmp), "non-admin", None)
            self.assertNotIn("TEMP", set(env) - set(os.environ))

    def test_environment_block(self) -> None:
        block = _build_environment_block({"A": "1", "B": "2"})
        self.assertTrue(block.endswith(b"\x00\x00\x00\x00"))
        text = block.decode("utf-16-le")
        self.assertIn("A=1\x00", text)
        self.assertIn("B=2\x00", text)

    def test_environment_block_skips_invalid_keys(self) -> None:
        block = _build_environment_block({"bad=key": "v", "ok": "v"})
        text = block.decode("utf-16-le")
        self.assertNotIn("bad=key", text)
        self.assertIn("ok=v", text)

    def test_command_line_from_list(self) -> None:
        line = _command_line_from(["powershell.exe", "-File", r"C:\p x\probe.ps1"])
        self.assertIn("powershell.exe", line)
        self.assertIn('"C:\\p x\\probe.ps1"', line)

    def test_command_line_from_string_passthrough(self) -> None:
        self.assertEqual(_command_line_from("echo hi"), "echo hi")


class RunObservationSchemaTests(unittest.TestCase):
    def test_schema_is_valid_self(self) -> None:
        Draft202012Validator.check_schema(
            _load(ASSURANCE / "windows-native-sandbox-run-v0.1.schema.json")
        )

    def test_valid_fixtures_pass_for_all_arms(self) -> None:
        for arm in WINDOWS_RUN_ARMS:
            obs = _valid_run_observation(arm)
            obs["observation_id"] = f"WNR-SCHEMA-{arm.upper()}"
            errors = _schema_errors(
                obs, "windows-native-sandbox-run-v0.1.schema.json"
            )
            self.assertEqual(errors, [], f"arm={arm}")

    def test_high_nist_missing_check_rejected(self) -> None:
        obs = _valid_run_observation("high-nist")
        del obs["checks"]["low_integrity"]
        errors = _schema_errors(
            obs, "windows-native-sandbox-run-v0.1.schema.json"
        )
        self.assertTrue(len(errors) > 0)

    def test_compliant_with_false_check_rejected(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["checks"]["low_integrity"] = False
        errors = _schema_errors(
            obs, "windows-native-sandbox-run-v0.1.schema.json"
        )
        self.assertTrue(len(errors) > 0)

    def test_invalid_arm_rejected(self) -> None:
        obs = _valid_run_observation("control")
        obs["arm"] = "bogus"
        errors = _schema_errors(
            obs, "windows-native-sandbox-run-v0.1.schema.json"
        )
        self.assertTrue(len(errors) > 0)


class RunObservationVerifierTests(unittest.TestCase):
    def _verify(self, obs: dict, *, require_compliant: bool) -> dict:
        profile = _load(ASSURANCE / "windows-native-sandbox-profile-v0.1.json")
        return verify_windows_native_run_observation(
            obs,
            profile=profile,
            profile_path=None,
            require_compliant=require_compliant,
        )

    def test_verifier_accepts_compliant_high_nist(self) -> None:
        result = self._verify(
            _valid_run_observation("high-nist"), require_compliant=True
        )
        self.assertTrue(result["valid"], msg=result["errors"])
        self.assertTrue(result["controls_compliant"])

    def test_verifier_accepts_compliant_control(self) -> None:
        result = self._verify(
            _valid_run_observation("control"), require_compliant=True
        )
        self.assertTrue(result["valid"], msg=result["errors"])

    def test_verifier_rejects_overstated_compliance(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["checks"]["job_object_assigned"] = False
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])

    def test_verifier_rejects_compliant_high_nist_without_appcontainer_token(
        self,
    ) -> None:
        obs = _valid_run_observation("high-nist")
        obs["token"]["appcontainer"] = False
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(
            any("AppContainer token" in error for error in result["errors"])
        )

    def test_verifier_rejects_understated_compliance(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["outcome"] = "noncompliant"
        result = self._verify(obs, require_compliant=False)
        self.assertFalse(result["valid"])
        self.assertTrue(any("understates" in error for error in result["errors"]))

    def test_verifier_rejects_profile_digest_mismatch(self) -> None:
        obs = _valid_run_observation("control")
        obs["profile_sha256"] = "f" * 64
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(any("digest" in error for error in result["errors"]))

    def test_verifier_rejects_control_with_restricted_token(self) -> None:
        obs = _valid_run_observation("control")
        obs["token"]["restricted"] = True
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])

    def test_verifier_rejects_high_nist_without_egress_wall(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["firewall"]["outbound_block_rule_created"] = False
        obs["firewall"]["allowlist_ips"] = []
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(any("egress wall" in error for error in result["errors"]))

    def test_verifier_rejects_allowlist_without_rule_creation(self) -> None:
        """allowlist 只声明 intent 不算墙：规则必须实际创建成功。"""
        obs = _valid_run_observation("high-nist")
        obs["firewall"]["outbound_block_rule_created"] = False
        obs["firewall"]["allowlist_ips"] = ["1.1.1.1"]
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(any("egress wall" in error for error in result["errors"]))

    def test_verifier_rejects_profile_created_but_not_deleted(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["appcontainer"]["profile_created"] = True
        obs["appcontainer"]["profile_deleted"] = False
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(any("not deleted" in error for error in result["errors"]))

    def test_verifier_rejects_active_process_limit_mismatch(self) -> None:
        obs = _valid_run_observation("high-nist")
        obs["job_object"]["active_process_limit"] = 1
        result = self._verify(obs, require_compliant=True)
        self.assertFalse(result["valid"])
        self.assertTrue(
            any("active-process limit" in error for error in result["errors"])
        )


@unittest.skipUnless(os.name == "nt", "PowerShell parser check requires Windows")
class WindowsHardeningScriptParseTests(unittest.TestCase):
    def _parse(self, script: Path) -> list[str]:
        result = subprocess.run(
            [
                "powershell.exe",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                (
                    "$errors=$null; "
                    "[System.Management.Automation.Language.Parser]::ParseFile("
                    f"'{script}', [ref]$null, [ref]$errors) | Out-Null; "
                    "$errors | ForEach-Object { $_.Message }"
                ),
            ],
            capture_output=True,
            shell=False,
            timeout=60,
        )
        out = (result.stdout or b"").decode("utf-8", errors="replace")
        return [line for line in out.splitlines() if line.strip()]

    def test_hardening_script_parses(self) -> None:
        errors = self._parse(ROOT / "_windows_high_nist" / "hardening" / "apply_hardening.ps1")
        self.assertEqual(errors, [])

    def test_enforcement_probe_parses(self) -> None:
        errors = self._parse(ROOT / "_windows_high_nist" / "policy" / "enforcement_probe.ps1")
        self.assertEqual(errors, [])

    def test_enforcement_runner_parses(self) -> None:
        errors = self._parse(ROOT / "_windows_high_nist" / "run" / "run_enforcement_probe.ps1")
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
