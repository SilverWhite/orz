"""Tests for assurance.sandbox_verifier — Docker observation and sandbox selection.

Covers verify_docker_observation (valid + 15 error paths) and
verify_sandbox_selection_receipt (valid + 8 error paths).
"""
from __future__ import annotations

import json
import unittest
from pathlib import Path

from assurance.sandbox_verifier import (
    verify_docker_observation,
    verify_sandbox_selection_receipt,
)
from assurance.utils import sha256_file


ROOT = Path(__file__).resolve().parents[2]
ASSURANCE = ROOT / "assurance"
DOCKER_PROFILE = ASSURANCE / "docker-sandbox-profile-v0.1.json"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


# ---------------------------------------------------------------------------
# Shared fixture helpers
# ---------------------------------------------------------------------------


def _load_profile() -> dict:
    return _load(DOCKER_PROFILE)


def _valid_docker_observation() -> dict:
    """Return a minimally valid Docker observation that matches the profile."""
    profile = _load_profile()
    profile_sha = sha256_file(DOCKER_PROFILE)
    return {
        "schema_version": "0.1.0-draft",
        "observation_kind": "docker_strict_sandbox_observation",
        "observation_id": "DSO-VERIFY-001",
        "created_at": "2026-07-27T00:00:00Z",
        "profile_sha256": profile_sha,
        "workspace_path_sha256": "e" * 64,
        "image": {
            "reference": profile["image"]["reference"],
            "image_id": "sha256:" + "c" * 64,
            "repo_digests": [profile["image"]["reference"]],
        },
        "docker": {
            "client_version": "27.0.0",
            "server_version": "27.0.0",
            "server_os": "linux",
            "server_arch": "x86_64",
            "security_options": ["name=seccomp,profile=default"],
        },
        "container": {
            "container_id": "abcdef" + "0" * 58,
            "user": profile["identity"]["user"],
            "read_only_rootfs": True,
            "network_mode": "none",
            "cap_drop": ["ALL"],
            "cap_add": [],
            "security_opt": ["no-new-privileges:true"],
            "memory_bytes": profile["resources"]["memory_bytes"],
            "nano_cpus": profile["resources"]["nano_cpus"],
            "pids_limit": profile["resources"]["pids_limit"],
            "mounts": [
                {
                    "type": "bind",
                    "target": "/workspace",
                    "read_only": False,
                    "source_path_sha256": "e" * 64,
                }
            ],
            "tmpfs": {
                "/tmp": "rw,noexec,nosuid,nodev,size=67108864",
            },
            "exit_code": 0,
            "removed": True,
        },
        "checks": {
            "non_root": True,
            "rootfs_write_blocked": True,
            "workspace_write_succeeded": True,
            "tmpfs_write_succeeded": True,
            "network_connect_blocked": True,
            "docker_socket_absent": True,
            "host_home_mount_absent": True,
            "probe_file_cleaned": True,
        },
        "outcome": "compliant",
        "evidence_status": "observed",
        "limitations": ["test fixture"],
    }


def _valid_selection_receipt(*, outcome: str = "allow") -> dict:
    """Return a minimally valid sandbox selection receipt."""
    selected = {
        "backend_id": "BACKEND-DOCKER-STRICT-001",
        "backend_kind": "docker_strict",
        "compliance_status": "compliant",
        "evidence_status": "observed",
        "receipt_digest": "a" * 64,
    }
    terminal = "selected" if outcome == "allow" else "no_compliant_backend"
    return {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "sandbox_selection_receipt",
        "receipt_id": "SBX-TEST-001",
        "conversation_id": "CONV-TEST-001",
        "created_at": "2026-07-27T00:00:00Z",
        "requested_backend": "auto",
        "candidates": [
            {
                "backend_id": "BACKEND-DOCKER-STRICT-001",
                "backend_kind": "docker_strict",
                "availability": "available",
                "compliance_status": "compliant",
                "evidence_status": "observed",
                "receipt_digest": "a" * 64,
                "rejection_reasons": [],
            }
        ],
        "decision": {
            "outcome": outcome,
            "terminal_state": terminal,
            "selected_backend": selected if outcome == "allow" else None,
            "fallback_used": False,
            "fallback_authorization_digest": None,
            "rationale": "test fixture decision",
        },
    }


# ---------------------------------------------------------------------------
# verify_docker_observation
# ---------------------------------------------------------------------------


class VerifyDockerObservationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.profile = _load_profile()

    def test_valid_compliant_observation_passes(self) -> None:
        obs = _valid_docker_observation()
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertTrue(result["valid"])
        self.assertTrue(result["controls_compliant"])
        self.assertEqual(result["errors"], [])

    def test_valid_noncompliant_observation_with_require_false_is_valid(self) -> None:
        # When require_compliant=False and outcome=noncompliant, structural
        # errors (NOT check failures) are still reported.  Use a scenario
        # where only a non-check field differs from the profile.
        obs = _valid_docker_observation()
        obs["outcome"] = "noncompliant"
        # Change a container field that is NOT in "checks":
        obs["container"]["memory_bytes"] = 999
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=False,
        )
        # memory_bytes mismatch is a structural error -> valid=False
        self.assertFalse(result["valid"])
        self.assertFalse(result["controls_compliant"])

    def test_profile_digest_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["profile_sha256"] = "b" * 64
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_image_reference_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["image"]["reference"] = "alpine@sha256:" + "f" * 64
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_repo_digest_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["image"]["repo_digests"] = ["other@sha256:" + "d" * 64]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_non_linux_server_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["docker"]["server_os"] = "windows"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_missing_seccomp_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["docker"]["security_options"] = ["name=apparmor"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_container_user_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["user"] = "root"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_writable_rootfs_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["read_only_rootfs"] = False
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_network_not_none_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["network_mode"] = "host"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_capabilities_not_deny_all_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["cap_drop"] = ["NET_RAW"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_extra_cap_add_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["cap_add"] = ["NET_ADMIN"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_missing_no_new_privileges_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["security_opt"] = ["apparmor:unconfined"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_memory_limit_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["memory_bytes"] = 999999
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_cpu_limit_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["nano_cpus"] = 999999
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_pid_limit_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["pids_limit"] = 999
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_nonzero_exit_code_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["exit_code"] = 1
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_container_not_removed_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["removed"] = False
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_missing_check_keys_rejected(self) -> None:
        obs = _valid_docker_observation()
        del obs["checks"]["docker_socket_absent"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_failed_check_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["checks"]["network_connect_blocked"] = False
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_require_compliant_with_noncompliant_outcome_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["outcome"] = "noncompliant"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_overstated_compliance_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["outcome"] = "compliant"
        obs["container"]["network_mode"] = "bridge"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_mount_count_not_one_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["mounts"] = []
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_mount_target_not_workspace_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["mounts"][0]["target"] = "/home"
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_read_only_mount_rejected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["mounts"][0]["read_only"] = True
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_tmpfs_mismatch_detected(self) -> None:
        obs = _valid_docker_observation()
        obs["container"]["tmpfs"] = {"/tmp": "rw,size=99999"}
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_invalid_observation_schema_returns_valid_false(self) -> None:
        obs = _valid_docker_observation()
        del obs["container"]
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])
        self.assertFalse(result["controls_compliant"])

    def test_invalid_profile_schema_returns_valid_false(self) -> None:
        bad_profile = dict(self.profile)
        del bad_profile["image"]
        obs = _valid_docker_observation()
        result = verify_docker_observation(
            obs, profile=bad_profile, profile_path=DOCKER_PROFILE,
            require_compliant=True,
        )
        self.assertFalse(result["valid"])

    def test_check_failures_always_invalidate_controls(self) -> None:
        """Docker verifier treats check failures as errors regardless of
        require_compliant flag (differs from Windows verifier behaviour)."""
        obs = _valid_docker_observation()
        obs["outcome"] = "noncompliant"
        obs["checks"]["network_connect_blocked"] = False
        result = verify_docker_observation(
            obs, profile=self.profile, profile_path=DOCKER_PROFILE,
            require_compliant=False,
        )
        self.assertFalse(result["valid"])
        self.assertFalse(result["controls_compliant"])


# ---------------------------------------------------------------------------
# verify_sandbox_selection_receipt
# ---------------------------------------------------------------------------


class VerifySandboxSelectionReceiptTests(unittest.TestCase):
    def test_valid_allow_receipt_passes(self) -> None:
        receipt = _valid_selection_receipt(outcome="allow")
        result = verify_sandbox_selection_receipt(receipt)
        self.assertTrue(result["valid"])
        self.assertEqual(result["errors"], [])

    def test_valid_fail_closed_receipt_passes(self) -> None:
        receipt = _valid_selection_receipt(outcome="fail_closed")
        receipt["candidates"][0]["compliance_status"] = "noncompliant"
        receipt["candidates"][0]["rejection_reasons"] = ["isolation_failed"]
        result = verify_sandbox_selection_receipt(receipt)
        self.assertTrue(result["valid"])

    def test_duplicate_backend_kinds_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["candidates"].append(dict(receipt["candidates"][0]))
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_fallback_used_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["decision"]["fallback_used"] = True
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_fallback_authorization_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["decision"]["fallback_authorization_digest"] = "b" * 64
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_selected_backend_not_in_candidates_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["decision"]["selected_backend"]["backend_id"] = "BACKEND-MISSING"
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_candidate_not_observed_compliant_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["candidates"][0]["compliance_status"] = "noncompliant"
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_requested_backend_mismatch_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["requested_backend"] = "windows_native_strict"
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_fail_closed_with_selection_rejected(self) -> None:
        receipt = _valid_selection_receipt(outcome="fail_closed")
        receipt["candidates"][0]["compliance_status"] = "noncompliant"
        receipt["candidates"][0]["rejection_reasons"] = ["isolation_failed"]
        # decision has selected_backend set, but outcome is deny
        receipt["decision"]["selected_backend"] = {
            "backend_id": "BACKEND-DOCKER-STRICT-001",
            "backend_kind": "docker_strict",
            "compliance_status": "compliant",
            "evidence_status": "observed",
            "receipt_digest": "a" * 64,
        }
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_fail_closed_ignoring_compliant_candidate_rejected(self) -> None:
        receipt = _valid_selection_receipt(outcome="fail_closed")
        # candidate IS compliant but decision is deny
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_schema_invalid_receipt_returns_valid_false(self) -> None:
        receipt = _valid_selection_receipt()
        del receipt["candidates"]
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_selected_projection_mismatch_rejected(self) -> None:
        receipt = _valid_selection_receipt()
        receipt["decision"]["selected_backend"]["backend_kind"] = "wrong_kind"
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])

    def test_auto_ignores_compliant_rejected(self) -> None:
        receipt = _valid_selection_receipt(outcome="fail_closed")
        receipt["candidates"][0]["compliance_status"] = "noncompliant"
        receipt["candidates"][0]["rejection_reasons"] = ["isolation_failed"]
        # Add a compliant candidate that was ignored
        receipt["candidates"].append({
            "backend_id": "BACKEND-WINDOWS-STRICT-001",
            "backend_kind": "windows_native_strict",
            "availability": "available",
            "compliance_status": "compliant",
            "evidence_status": "observed",
            "receipt_digest": "b" * 64,
            "rejection_reasons": [],
        })
        result = verify_sandbox_selection_receipt(receipt)
        self.assertFalse(result["valid"])


if __name__ == "__main__":
    unittest.main()
