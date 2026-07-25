from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker

from assurance.sandbox import (
    build_sandbox_selection_receipt,
    run_docker_sandbox_probe,
    windows_native_strict_candidate,
)
from assurance.sandbox_verifier import verify_sandbox_selection_receipt
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


def _observed_candidate(kind: str = "docker") -> dict:
    return {
        "backend_id": f"BACKEND-{kind.upper().replace('_', '-')}-001",
        "backend_kind": kind,
        "availability": "available",
        "compliance_status": "compliant",
        "evidence_status": "observed",
        "receipt_digest": "a" * 64,
        "rejection_reasons": [],
    }


class P2SandboxSelectionTests(unittest.TestCase):
    def test_auto_selects_observed_compliant_candidate(self) -> None:
        receipt = build_sandbox_selection_receipt(
            requested_backend="auto",
            candidates=[windows_native_strict_candidate(), _observed_candidate()],
            conversation_id="CONV-P2-UNIT",
        )
        self.assertEqual(receipt["decision"]["outcome"], "allow")
        self.assertEqual(
            receipt["decision"]["selected_backend"]["backend_kind"], "docker"
        )

    def test_specific_noncompliant_request_fails_without_fallback(self) -> None:
        receipt = build_sandbox_selection_receipt(
            requested_backend="windows_native_strict",
            candidates=[windows_native_strict_candidate(), _observed_candidate()],
            conversation_id="CONV-P2-UNIT",
        )
        self.assertEqual(receipt["decision"]["outcome"], "fail_closed")
        self.assertIsNone(receipt["decision"]["selected_backend"])
        self.assertFalse(receipt["decision"]["fallback_used"])

    def test_verifier_rejects_tampered_selection_projection(self) -> None:
        receipt = build_sandbox_selection_receipt(
            requested_backend="docker",
            candidates=[_observed_candidate()],
            conversation_id="CONV-P2-UNIT",
        )
        tampered = deepcopy(receipt)
        tampered["decision"]["selected_backend"]["receipt_digest"] = "b" * 64
        self.assertFalse(verify_sandbox_selection_receipt(tampered)["valid"])

    def test_extension_points_are_open_without_weakening_gates(self) -> None:
        custom = build_sandbox_selection_receipt(
            requested_backend="future_isolator",
            candidates=[_observed_candidate("future_isolator")],
            conversation_id="CONV-P2-UNIT",
        )
        self.assertEqual(custom["decision"]["outcome"], "allow")

        registry = _load(ASSURANCE / "profile-registry-v0.1.json")
        registry["profiles"][0]["profile_id"] = "custom-profile"
        self.assertEqual(
            _schema_errors(
                registry, "assurance-profile-registry-v0.1.schema.json"
            ),
            [],
        )

        retention = _load(ASSURANCE / "retention-policy-v0.1.json")
        retention["delete_on_archive"].append("future_temporary_category")
        retention["never_persist"].append("future_secret_category")
        self.assertEqual(
            _schema_errors(retention, "retention-policy-v0.1.schema.json"),
            [],
        )

        docker_profile = _load(
            ASSURANCE / "docker-sandbox-profile-v0.1.json"
        )
        docker_profile["profile_id"] = "custom-docker-profile"
        docker_profile["resources"]["memory_bytes"] = 4_294_967_296
        docker_profile["image"]["reference"] = (
            "localhost:5000/python@sha256:" + docker_profile["image"]["digest"]
        )
        self.assertEqual(
            _schema_errors(
                docker_profile, "docker-sandbox-profile-v0.1.schema.json"
            ),
            [],
        )

    def test_malformed_probe_output_still_cleans_container_and_probe_file(
        self,
    ) -> None:
        commands: list[list[str]] = []
        with tempfile.TemporaryDirectory(dir=ROOT) as directory:
            workspace = Path(directory)
            (workspace / ".assurance-p2-disposable.json").write_text(
                json.dumps(
                    {
                        "schema_version": "0.1.0-draft",
                        "purpose": "docker-sandbox-probe",
                        "allow_container_write_probe": True,
                    }
                ),
                encoding="utf-8",
            )

            def runner(
                command: list[str], timeout_seconds: int
            ) -> subprocess.CompletedProcess[str]:
                del timeout_seconds
                command = list(command)
                commands.append(command)
                if command[1] == "version":
                    stdout = json.dumps(
                        {
                            "Client": {"Version": "test"},
                            "Server": {
                                "Version": "test",
                                "Os": "linux",
                                "Arch": "amd64",
                            },
                        }
                    )
                elif command[1] == "info":
                    stdout = json.dumps(
                        {"SecurityOptions": ["name=seccomp,profile=builtin"]}
                    )
                elif command[1:3] == ["image", "inspect"]:
                    profile = _load(
                        ASSURANCE / "docker-sandbox-profile-v0.1.json"
                    )
                    stdout = json.dumps(
                        [
                            {
                                "Id": f"sha256:{profile['image']['digest']}",
                                "RepoDigests": [profile["image"]["reference"]],
                            }
                        ]
                    )
                elif command[1] == "create":
                    stdout = "c" * 64
                elif command[1] == "start":
                    (workspace / ".p2-container-write-probe").write_text(
                        "workspace-ok", encoding="utf-8"
                    )
                    stdout = "malformed"
                elif command[1] == "rm":
                    stdout = "removed"
                else:
                    raise AssertionError(command)
                return subprocess.CompletedProcess(command, 0, stdout, "")

            with self.assertRaises(AssuranceError):
                run_docker_sandbox_probe(workspace, runner=runner)
            self.assertFalse((workspace / ".p2-container-write-probe").exists())
            self.assertTrue(any(command[1] == "rm" for command in commands))


if __name__ == "__main__":
    unittest.main()
