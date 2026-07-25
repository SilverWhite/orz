from __future__ import annotations

from copy import deepcopy
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    MemoryInstallationKeyStore,
    execute_workspace_first_integrated_run,
    initialize_workspace_marker,
    load_execution_backend_policy,
    select_execution_backend,
    verify_workspace_first_integrated_run,
)
from assurance.integrated_run import PROBE_FILE


ROOT = Path(__file__).resolve().parents[2]


class P45WorkspaceFirstIntegrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        initialize_workspace_marker(self.workspace)
        (self.workspace / "seed.txt").write_text(
            "stable seed\n", encoding="utf-8"
        )
        self.key_store = MemoryInstallationKeyStore(b"4" * 32)

    def tearDown(self) -> None:
        self.key_store.close()
        self.temporary.cleanup()

    def _run(self) -> dict:
        return execute_workspace_first_integrated_run(
            workspace=self.workspace,
            conversation_repository=self.root / "conversations",
            key_store=self.key_store,
        )

    def test_standard_run_authorizes_executes_audits_archives_without_docker(
        self,
    ) -> None:
        result = self._run()
        receipt = result["receipt"]
        verification = result["verification"]
        self.assertTrue(verification["valid"], verification["errors"])
        self.assertTrue(verification["archive_complete"])
        self.assertTrue(verification["audit_valid"])
        self.assertTrue(verification["raw_payload_absence_proven"])
        self.assertEqual(receipt["mode"], "standard")
        self.assertEqual(receipt["backend"], "workspace_guarded")
        self.assertEqual(
            receipt["security_claim"], "logical_workspace_boundary_only"
        )
        for field in (
            "docker_invoked",
            "process_spawned",
            "model_invoked",
            "network_requested",
        ):
            self.assertFalse(receipt["execution"][field])
        self.assertFalse((self.workspace / PROBE_FILE).exists())
        self.assertEqual(
            receipt["bindings"]["workspace_before_sha256"],
            receipt["bindings"]["workspace_after_sha256"],
        )
        self.assertEqual(result["namespace"].state()["state"], "archived")

    def test_policy_is_workspace_first_and_strict_never_silently_falls_back(
        self,
    ) -> None:
        policy = load_execution_backend_policy()
        self.assertEqual(policy["default_mode"], "standard")
        standard = select_execution_backend("standard")
        self.assertEqual(standard["backend"], "workspace_guarded")
        self.assertFalse(standard["docker_required"])
        self.assertEqual(
            standard["security_claim"], "logical_workspace_boundary_only"
        )
        self.assertFalse(policy["modes"]["strict"]["fallback_to_standard"])
        self.assertFalse(policy["modes"]["strict"]["implicit_start"])
        with self.assertRaisesRegex(
            AssuranceError, "requires an explicit, observed P2 Docker selection"
        ):
            select_execution_backend("strict")

    def test_marker_and_retained_receipt_tampering_fail_closed(self) -> None:
        invalid_workspace = self.root / "invalid-workspace"
        invalid_workspace.mkdir()
        with self.assertRaisesRegex(AssuranceError, "marker"):
            execute_workspace_first_integrated_run(
                workspace=invalid_workspace,
                conversation_repository=self.root / "invalid-conversations",
                key_store=self.key_store,
            )

        result = self._run()
        tampered = deepcopy(result["receipt"])
        tampered["execution"]["docker_invoked"] = True
        verification = verify_workspace_first_integrated_run(
            tampered,
            namespace=result["namespace"],
            key_store=self.key_store,
            workspace=self.workspace,
        )
        self.assertFalse(verification["valid"])


if __name__ == "__main__":
    unittest.main()
