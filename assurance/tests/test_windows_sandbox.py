from __future__ import annotations

import ctypes
import json
from pathlib import Path
import os
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker


def _is_elevated() -> bool:
    """Return True if the current process has administrator privileges."""
    if os.name != "nt":
        return os.geteuid() == 0  # type: ignore[attr-defined]
    try:
        return ctypes.windll.shell32.IsUserAnAdmin() != 0
    except Exception:
        return False

from assurance.sandbox import windows_native_strict_candidate
from assurance.windows_sandbox import (
    run_windows_native_sandbox_probe,
    windows_native_candidate_from_observation,
)
from assurance.sandbox_verifier import verify_windows_native_observation
from assurance.errors import AssuranceError


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


class WindowsNativeProfileTests(unittest.TestCase):
    def test_profile_is_valid_by_self_schema(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        errors = _schema_errors(
            profile, "windows-native-sandbox-profile-v0.1.schema.json"
        )
        self.assertEqual(errors, [])

    def test_profile_schema_rejects_missing_forbidden_paths(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        profile["filesystem"]["forbidden_paths"] = [
            "system32",
            "program_files",
            "windows",
        ]
        errors = _schema_errors(
            profile, "windows-native-sandbox-profile-v0.1.schema.json"
        )
        self.assertTrue(len(errors) > 0)


class WindowsNativeObservationSchemaTests(unittest.TestCase):
    def test_schema_rejects_missing_checks(self) -> None:
        minimal = {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-TEST",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "a" * 64,
            "workspace_path_sha256": "b" * 64,
            "appcontainer": {
                "sid_derived": True,
                "profile_created": False,
                "profile_deleted": False,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": True,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {
                "pid": 0,
                "exit_code": 0,
                "shell_used": False,
                "create_new_process_group": True,
            },
            "cancellation": {
                "timeout_seconds": 30,
                "cancellation_method": None,
                "ctrl_break_sent": False,
                "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": True,
                "rule_name": "GSA-P2-Native-Sandbox-test",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": True,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": True,
                "registry_protected_blocked": True,
                "probe_file_cleaned": True,
            },
            "outcome": "compliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["test"],
        }
        errors = _schema_errors(
            minimal,
            "windows-native-sandbox-observation-v0.1.schema.json",
        )
        self.assertEqual(errors, [])

    def test_compliant_requires_all_checks_true(self) -> None:
        obs = {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-TEST2",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "a" * 64,
            "workspace_path_sha256": "b" * 64,
            "appcontainer": {
                "sid_derived": True,
                "profile_created": False,
                "profile_deleted": False,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": True,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {
                "pid": 0,
                "exit_code": 0,
                "shell_used": False,
                "create_new_process_group": True,
            },
            "cancellation": {
                "timeout_seconds": 30,
                "cancellation_method": None,
                "ctrl_break_sent": False,
                "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": False,
                "rule_name": "",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": False,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": True,
                "registry_protected_blocked": True,
                "probe_file_cleaned": True,
            },
            "outcome": "compliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["test"],
        }
        errors = _schema_errors(
            obs,
            "windows-native-sandbox-observation-v0.1.schema.json",
        )
        self.assertTrue(len(errors) > 0)


class WindowsNativeObservationVerifierTests(unittest.TestCase):
    def _valid_observation(self) -> dict:
        return {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-VERIFY-001",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "d" * 64,
            "workspace_path_sha256": "e" * 64,
            "appcontainer": {
                "sid_derived": True,
                "profile_created": False,
                "profile_deleted": False,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": True,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {
                "pid": 0,
                "exit_code": 0,
                "shell_used": False,
                "create_new_process_group": True,
            },
            "cancellation": {
                "timeout_seconds": 30,
                "cancellation_method": None,
                "ctrl_break_sent": False,
                "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": True,
                "rule_name": "GSA-P2-Native-Sandbox-test-fixture",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": True,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": True,
                "registry_protected_blocked": True,
                "probe_file_cleaned": True,
            },
            "outcome": "compliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["test"],
        }

    def test_verifier_rejects_sid_not_derived(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["appcontainer"]["sid_derived"] = False
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_job_not_created(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["job_object"]["created"] = False
        obs["job_object"]["assigned"] = False
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_job_not_assigned(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["job_object"]["assigned"] = False
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_shell_used(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["process"]["shell_used"] = True
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_failed_checks(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["checks"]["system32_write_blocked"] = False
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_missing_profile_deletion(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["appcontainer"]["profile_created"] = True
        obs["appcontainer"]["profile_deleted"] = False
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_capabilities_not_empty(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["appcontainer"]["capabilities"] = ["internetClient"]
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])

    def test_verifier_rejects_overstated_compliance(self) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = self._valid_observation()
        obs["checks"]["system32_write_blocked"] = False
        obs["outcome"] = "compliant"
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=True
        )
        self.assertFalse(result["valid"])


class WindowsNativeCandidateTests(unittest.TestCase):
    def test_candidate_from_compliant_observation(self) -> None:
        obs = {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-CAND",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "a" * 64,
            "workspace_path_sha256": "b" * 64,
            "appcontainer": {
                "sid_derived": True,
                "profile_created": False,
                "profile_deleted": False,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": True,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {
                "pid": 0,
                "exit_code": 0,
                "shell_used": False,
                "create_new_process_group": True,
            },
            "cancellation": {
                "timeout_seconds": 30,
                "cancellation_method": None,
                "ctrl_break_sent": False,
                "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": True,
                "rule_name": "GSA-P2-Native-Sandbox-test",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": True,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": True,
                "registry_protected_blocked": True,
                "probe_file_cleaned": True,
            },
            "outcome": "compliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["test"],
        }
        candidate = windows_native_candidate_from_observation(obs)
        self.assertEqual(candidate["backend_kind"], "windows_native_strict")
        self.assertEqual(candidate["compliance_status"], "compliant")
        self.assertEqual(candidate["availability"], "available")

    def test_noncompliant_observation_produces_noncompliant_candidate(
        self,
    ) -> None:
        obs = {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-NONCOMPL",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "a" * 64,
            "workspace_path_sha256": "b" * 64,
            "appcontainer": {
                "sid_derived": False,
                "profile_created": False,
                "profile_deleted": False,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": False,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {
                "pid": 0,
                "exit_code": 7,
                "shell_used": False,
                "create_new_process_group": True,
            },
            "cancellation": {
                "timeout_seconds": 30,
                "cancellation_method": None,
                "ctrl_break_sent": False,
                "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": False,
                "rule_name": "",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": False,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": False,
                "registry_protected_blocked": False,
                "probe_file_cleaned": True,
            },
            "outcome": "noncompliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["test"],
        }
        candidate = windows_native_candidate_from_observation(obs)
        self.assertEqual(candidate["compliance_status"], "noncompliant")
        self.assertTrue(len(candidate["rejection_reasons"]) > 0)

    def test_static_candidate_is_noncompliant_on_windows(self) -> None:
        candidate = windows_native_strict_candidate()
        if os.name == "nt":
            self.assertEqual(candidate["compliance_status"], "noncompliant")
            self.assertEqual(
                candidate["rejection_reasons"],
                ["windows_native_live_observation_required"],
            )
        else:
            self.assertEqual(candidate["availability"], "unavailable")

    def test_verifier_allows_structural_noncompliant_when_not_required(
        self,
    ) -> None:
        profile = _load(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        obs = {
            "schema_version": "0.1.0-draft",
            "observation_kind": "windows_native_strict_sandbox_observation",
            "observation_id": "WNO-STRUCT-OK",
            "created_at": "2026-01-01T00:00:00Z",
            "profile_sha256": "d" * 64,
            "workspace_path_sha256": "e" * 64,
            "appcontainer": {
                "sid_derived": True,
                "profile_created": True,
                "profile_deleted": True,
                "capabilities": [],
            },
            "job_object": {
                "created": True,
                "assigned": True,
                "kill_on_close": True,
                "memory_limit_bytes": 268435456,
            },
            "process": {"pid": 0, "exit_code": 0, "shell_used": False, "create_new_process_group": True},
            "cancellation": {
                "timeout_seconds": 30, "cancellation_method": None,
                "ctrl_break_sent": False, "ctrl_break_effective": False,
                "grace_period_seconds": 0,
            },
            "firewall": {
                "outbound_block_rule_created": False,
                "rule_name": "",
                "diagnostic": "test fixture",
            },
            "checks": {
                "non_admin": True,
                "system32_write_blocked": True,
                "workspace_write_succeeded": True,
                "temp_write_succeeded": True,
                "network_connect_blocked": False,
                "registry_protected_blocked": True,
                "probe_file_cleaned": True,
            },
            "outcome": "noncompliant",
            "evidence_status": "observed",
            "diagnostics": [],
            "limitations": ["network residual"],
        }
        # Bypass profile digest match for this structural unit test.
        from assurance.utils import sha256_file

        obs["profile_sha256"] = sha256_file(
            ASSURANCE / "windows-native-sandbox-profile-v0.1.json"
        )
        result = verify_windows_native_observation(
            obs, profile=profile, profile_path=None, require_compliant=False
        )
        self.assertTrue(result["valid"])
        self.assertFalse(result["controls_compliant"])


@unittest.skipUnless(os.name == "nt", "Windows native live probe requires Windows")
class WindowsNativeLiveProbeTests(unittest.TestCase):
    """Live AppContainer probe. May take several seconds; no network mocks."""

    def test_live_probe_process_fs_registry_isolation(self) -> None:
        with tempfile.TemporaryDirectory(prefix="w32-native-live-") as tmp:
            workspace = Path(tmp)
            marker = {
                "schema_version": "0.1.0-draft",
                "purpose": "windows-native-sandbox-probe",
                "allow_container_write_probe": True,
            }
            (workspace / ".assurance-p2-disposable.json").write_text(
                json.dumps(marker),
                encoding="utf-8",
            )
            try:
                observation = run_windows_native_sandbox_probe(workspace)
            except AssuranceError as exc:
                message = str(exc)
                if "Windows native sandbox process creation failed" in message:
                    self.skipTest(
                        "Windows host cannot create an AppContainer probe "
                        f"process in this environment: {message}"
                    )
                raise
            checks = observation["checks"]
            # Proven on Windows 11: AppContainer token + Job + FS/registry isolation.
            self.assertTrue(observation["appcontainer"]["sid_derived"] or observation["appcontainer"]["profile_created"])
            self.assertTrue(observation["job_object"]["created"])
            self.assertTrue(observation["job_object"]["assigned"])
            self.assertTrue(checks["non_admin"])
            self.assertTrue(checks["system32_write_blocked"])
            if (
                os.environ.get("GITHUB_ACTIONS") == "true"
                and (
                    not checks["workspace_write_succeeded"]
                    or not checks["temp_write_succeeded"]
                )
            ):
                self.skipTest(
                    "Windows GitHub runner AppContainer probe lacks expected "
                    "temporary/workspace write capability"
                )
            self.assertTrue(checks["workspace_write_succeeded"])
            self.assertTrue(checks["temp_write_succeeded"])
            self.assertTrue(checks["registry_protected_blocked"])
            self.assertTrue(checks["probe_file_cleaned"])
            self.assertEqual(observation["process"]["shell_used"], False)
            self.assertEqual(observation["evidence_status"], "observed")
            # Raw Win32 TCP may still succeed under empty-capability AppContainer.
            # Do not claim full strict compliance until network is observed blocked.
            if checks["network_connect_blocked"]:
                if observation["outcome"] != "compliant":
                    # Non-elevated: AppContainer can block TCP via empty
                    # capabilities, but netsh firewall rules (required for
                    # "compliant" verdict) need admin.  Skip rather than fail.
                    if not _is_elevated():
                        self.skipTest(
                            "Windows native sandbox live probe requires "
                            "administrator elevation for full compliance. "
                            "Network blocked but outcome is "
                            f"'{observation['outcome']}'."
                        )
                self.assertEqual(observation["outcome"], "compliant")
            else:
                self.assertEqual(observation["outcome"], "noncompliant")
                self.assertTrue(
                    any("raw TCP" in item or "1.1.1.1" in item for item in observation["limitations"])
                )
            candidate = windows_native_candidate_from_observation(observation)
            if observation["outcome"] == "compliant":
                self.assertEqual(candidate["compliance_status"], "compliant")
            else:
                self.assertEqual(candidate["compliance_status"], "noncompliant")


if __name__ == "__main__":
    unittest.main()
