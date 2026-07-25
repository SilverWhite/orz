from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    build_guarded_frozen_context,
    build_sandbox_selection_receipt,
    docker_candidate_from_observation,
    execute_guarded_no_model_action,
    verify_guarded_execution_receipt,
)
from assurance.guarded_execution import (
    ACTION_CAPABILITY,
    ACTION_KIND,
    PROBE_FILE,
    PROBE_PAYLOAD,
)
from assurance.sandbox import load_docker_profile, windows_native_strict_candidate
from assurance.utils import canonical_bytes, sha256_bytes, sha256_file, utc_now


ROOT = Path(__file__).resolve().parents[2]
ASSURANCE = ROOT / "assurance"


def _observation(workspace: Path) -> dict:
    profile = load_docker_profile()
    workspace_sha = sha256_bytes(str(workspace.resolve()).encode("utf-8"))
    image_id = f"sha256:{profile['image']['digest']}"
    return {
        "schema_version": "0.1.0-draft",
        "observation_kind": "docker_strict_sandbox_observation",
        "observation_id": "DSO-GUARDED-UNIT",
        "created_at": "2026-07-24T00:00:00Z",
        "profile_sha256": sha256_file(
            ASSURANCE / "docker-sandbox-profile-v0.1.json"
        ),
        "workspace_path_sha256": workspace_sha,
        "docker": {
            "client_version": "test",
            "server_version": "test",
            "server_os": "linux",
            "server_arch": "amd64",
            "security_options": ["name=seccomp,profile=builtin"],
        },
        "image": {
            "reference": profile["image"]["reference"],
            "image_id": image_id,
            "repo_digests": [profile["image"]["reference"]],
        },
        "container": {
            "container_id": "a" * 64,
            "user": profile["identity"]["user"],
            "read_only_rootfs": True,
            "network_mode": "none",
            "cap_drop": ["ALL"],
            "cap_add": [],
            "security_opt": ["no-new-privileges=true"],
            "memory_bytes": profile["resources"]["memory_bytes"],
            "nano_cpus": profile["resources"]["nano_cpus"],
            "pids_limit": profile["resources"]["pids_limit"],
            "mounts": [
                {
                    "type": "bind",
                    "target": "/workspace",
                    "read_only": False,
                    "source_path_sha256": workspace_sha,
                }
            ],
            "tmpfs": {
                item["target"]: item["options"]
                for item in profile["filesystem"]["tmpfs"]
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
        "limitations": ["synthetic unit observation"],
    }


class FakeTracker:
    def __init__(self, workspace: Path, *, malformed_logs: bool = False) -> None:
        self.workspace = workspace
        self.malformed_logs = malformed_logs
        self.records: list[dict] = []
        self.container_id = "d" * 64

    def run(
        self,
        role: str,
        command: list[str],
        timeout_seconds: int,
        *,
        allow_nonzero: bool = False,
    ) -> subprocess.CompletedProcess[str]:
        del timeout_seconds
        if role == "create":
            stdout, returncode = self.container_id, 0
        elif role == "start":
            stdout, returncode = self.container_id, 0
        elif role == "inspect_running":
            stdout, returncode = json.dumps(
                [{"State": {"Running": True, "Pid": 4242, "ExitCode": 0}}]
            ), 0
        elif role == "top":
            stdout, returncode = "PID PPID COMMAND\n4242 1 python\n", 0
        elif role == "wait":
            (self.workspace / PROBE_FILE).write_bytes(PROBE_PAYLOAD)
            stdout, returncode = "0\n", 0
        elif role == "logs":
            if self.malformed_logs:
                stdout = "malformed"
            else:
                stdout = json.dumps(
                    {
                        "action": ACTION_KIND,
                        "model_invoked": False,
                        "network_requested": False,
                        "probe_sha256": sha256_bytes(PROBE_PAYLOAD),
                    }
                )
            returncode = 0
        elif role == "inspect_terminal":
            stdout, returncode = json.dumps(
                [{"State": {"Running": False, "Pid": 0, "ExitCode": 0}}]
            ), 0
        elif role in {"remove", "cleanup_remove"}:
            stdout, returncode = self.container_id, 0
        elif role == "inspect_absent":
            stdout, returncode = "", 1
        else:
            raise AssertionError(role)
        stdout_bytes = stdout.encode("utf-8")
        self.records.append(
            {
                "sequence": len(self.records),
                "role": role,
                "executable": "docker",
                "argument_count": len(command) - 1,
                "arguments_sha256": sha256_bytes(canonical_bytes(command[1:])),
                "pid": 5000 + len(self.records),
                "started_at": utc_now(),
                "completed_at": utc_now(),
                "duration_ms": 0.1,
                "exit_code": returncode,
                "timed_out": False,
                "stdout_bytes": len(stdout_bytes),
                "stdout_sha256": sha256_bytes(stdout_bytes),
                "stderr_bytes": 0,
                "stderr_sha256": sha256_bytes(b""),
            }
        )
        completed = subprocess.CompletedProcess(command, returncode, stdout, "")
        if returncode != 0 and not allow_nonzero:
            raise AssuranceError(f"fake Docker command failed: {role}")
        return completed


class GuardedExecutionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"G" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def _context(
        self, root: Path
    ) -> tuple[Path, ConversationNamespace, dict, dict]:
        workspace = root / "workspace"
        workspace.mkdir()
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
        observation = _observation(workspace)
        namespace = ConversationNamespace.create(
            root / "conversations",
            key_store=self.key_store,
            frozen_context=build_guarded_frozen_context(
                workspace=workspace,
                observation=observation,
            ),
            allowed_capabilities=[
                ACTION_CAPABILITY,
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=[
                "host.mount",
                "model.invoke",
                "network.external",
            ],
        )
        selection = build_sandbox_selection_receipt(
            requested_backend="auto",
            candidates=[
                docker_candidate_from_observation(observation),
                windows_native_strict_candidate(),
            ],
            conversation_id=namespace.conversation_id,
        )
        return workspace, namespace, observation, selection

    def test_fixed_action_binds_envelope_selection_processes_and_cleanup(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            workspace, namespace, observation, selection = self._context(
                Path(temporary)
            )
            receipt = execute_guarded_no_model_action(
                namespace=namespace,
                key_store=self.key_store,
                workspace=workspace,
                selection_receipt=selection,
                observation=observation,
                tracker=FakeTracker(workspace),
            )
            self.assertEqual(receipt["outcome"], "completed_verified")
            self.assertEqual(len(receipt["process_trace"]), 9)
            self.assertTrue(receipt["cleanup"]["container_absent"])
            self.assertFalse((workspace / PROBE_FILE).exists())
            self.assertTrue(
                verify_guarded_execution_receipt(
                    receipt,
                    namespace=namespace,
                    key_store=self.key_store,
                    workspace=workspace,
                    selection_receipt=selection,
                    observation=observation,
                )["valid"]
            )
            stored_receipt = (
                namespace.artifacts_root
                / "temporary_lifecycle_receipt"
                / f"{receipt['execution_id']}.json"
            )
            self.assertTrue(stored_receipt.is_file())

            tampered = deepcopy(receipt)
            tampered["process_trace"][2]["arguments_sha256"] = "f" * 64
            self.assertFalse(
                verify_guarded_execution_receipt(
                    tampered,
                    namespace=namespace,
                    key_store=self.key_store,
                    workspace=workspace,
                    selection_receipt=selection,
                    observation=observation,
                )["valid"]
            )
            ArchiveController(key_store=self.key_store).archive(namespace)
            self.assertFalse(stored_receipt.exists())

    def test_cross_conversation_selection_fails_before_execution(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            workspace, namespace, observation, selection = self._context(
                Path(temporary)
            )
            mismatched = deepcopy(selection)
            mismatched["conversation_id"] = "CONV-OTHER"
            tracker = FakeTracker(workspace)
            with self.assertRaisesRegex(AssuranceError, "conversation mismatch"):
                execute_guarded_no_model_action(
                    namespace=namespace,
                    key_store=self.key_store,
                    workspace=workspace,
                    selection_receipt=mismatched,
                    observation=observation,
                    tracker=tracker,
                )
            self.assertEqual(tracker.records, [])

    def test_malformed_container_report_still_cleans_process_and_workspace(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT) as temporary:
            workspace, namespace, observation, selection = self._context(
                Path(temporary)
            )
            tracker = FakeTracker(workspace, malformed_logs=True)
            with self.assertRaisesRegex(AssuranceError, "not valid JSON"):
                execute_guarded_no_model_action(
                    namespace=namespace,
                    key_store=self.key_store,
                    workspace=workspace,
                    selection_receipt=selection,
                    observation=observation,
                    tracker=tracker,
                )
            self.assertFalse((workspace / PROBE_FILE).exists())
            self.assertEqual(tracker.records[-1]["role"], "cleanup_remove")


if __name__ == "__main__":
    unittest.main()
