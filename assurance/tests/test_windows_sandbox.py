from __future__ import annotations

import json
from pathlib import Path
import os
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker

from assurance.sandbox import windows_native_strict_candidate
from assurance.windows_sandbox import (
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
                "exit_code": 0,
                "shell_used": False,
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
                "exit_code": 0,
                "shell_used": False,
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
                "exit_code": 0,
                "shell_used": False,
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
        obs["checks"]["network_connect_blocked"] = False
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
                "exit_code": 0,
                "shell_used": False,
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
                "exit_code": 7,
                "shell_used": False,
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
                ["appcontainer_probe_not_yet_run"],
            )
        else:
            self.assertEqual(candidate["availability"], "unavailable")


if __name__ == "__main__":
    unittest.main()
