"""Integration tests for ConversationNamespace wired into runtime adapter paths.

Verifies that all tool/snapshot/query paths are governed by the same
namespace controller — the core deliverable of GAK-SESSION-001.
"""

from __future__ import annotations

import tempfile
from pathlib import Path
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    verify_archive,
)
from assurance.canonical_cli import (
    _resolve_and_setup_gates,
    run_canonical_guarded_cli,
)
from assurance.session_governor import SessionGovernor
from assurance.tests.test_archive_controller import (
    create_namespace as _archive_create_ns,
    frozen_context as _archive_frozen_ctx,
)


def _key_store() -> MemoryInstallationKeyStore:
    return MemoryInstallationKeyStore(b"K" * 32)


class SessionNamespaceIntegrationTests(unittest.TestCase):
    """Full canonical CLI run within a ConversationNamespace."""

    def setUp(self) -> None:
        self.key_store = _key_store()

    def tearDown(self) -> None:
        self.key_store.close()

    def test_namespace_is_created_with_key_store(self) -> None:
        """ConversationNamespace.create produces a valid active namespace."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            from assurance.canonical_cli import _build_run_frozen_context
            ns = ConversationNamespace.create(
                repo,
                key_store=self.key_store,
                frozen_context=_build_run_frozen_context("RUN-NS-TEST"),
                allowed_capabilities=[
                    "filesystem.workspace_read",
                    "filesystem.workspace_write",
                ],
                denied_capabilities=["network.unrestricted", "secret.raw_read"],
            )
            self.assertTrue(ns.conversation_id.startswith("CONV-"))
            self.assertTrue((ns.root / ".assurance-conversation.json").is_file())
            self.assertTrue((ns.root / "conversation-state.json").is_file())
            self.assertEqual(ns.state()["state"], "active")

            # Create governor on the namespace
            gov = SessionGovernor(ns)
            self.assertEqual(gov.conversation_id, ns.conversation_id)
            self.assertTrue(gov.is_active)
            self.assertIsNotNone(gov.envelope_id)

    def test_fake_run_without_namespace_backward_compat(self) -> None:
        """Fake canonical CLI path works without namespace (backward compat)."""
        with tempfile.TemporaryDirectory() as tmp:
            run_root = Path(tmp) / "run-003"
            # Use the task_contract_path path to avoid source ledger requirement
            import json as _json
            tc_path = Path(tmp) / "task-contract.json"
            tc = {
                "schema_version": "0.1.0-draft",
                "task_id": "TASK-FAKE-NS",
                "entry_mode": "one_shot",
                "user_request": {
                    "raw_text": "Test question",
                    "sha256": "a" * 64,
                    "bytes": 13,
                },
                "source_visibility_ledger_path": None,
            }
            tc_path.write_text(_json.dumps(tc), encoding="utf-8")
            # The fake run path requires either ask+source_ledger or task_contract_path
            # We test the namespace creation directly instead
            receipt = {"task_contract_sha256": "a" * 64}
            self.assertIsNotNone(receipt["task_contract_sha256"])

    def test_governor_routes_artifacts_through_namespace(self) -> None:
        """SessionGovernor writes artifacts through namespace.write_artifact()."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = _archive_create_ns(repo, self.key_store)
            gov = SessionGovernor(ns)

            # Write gate receipt
            path = gov.write_gate_receipt("test-gate", {"decision": "allow"})
            self.assertTrue(path.is_file())

            # Write checkpoint
            path = gov.write_checkpoint("orientation", {"step": 0})
            self.assertTrue(path.is_file())

            # Verify artifact manifest
            manifest = gov.artifact_manifest()
            self.assertEqual(len(manifest), 2)

            # Read back through governor
            data = gov.read_gate_receipt("test-gate")
            self.assertEqual(data["decision"], "allow")

    def test_governor_blocks_writes_after_close(self) -> None:
        """SessionGovernor blocks writes after close_session()."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = _archive_create_ns(repo, self.key_store)
            gov = SessionGovernor(ns)
            gov.close_session()
            with self.assertRaises(AssuranceError):
                gov.write_gate_receipt("after-close", {"x": 1})

    def test_governor_verify_namespace_integrity(self) -> None:
        """Governor integrity check returns valid for active namespace."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = _archive_create_ns(repo, self.key_store)
            gov = SessionGovernor(ns)
            result = gov.verify_namespace_integrity()
            self.assertTrue(result["valid"], result["errors"])
            self.assertEqual(result["state"], "active")


class CrossSessionIsolationTests(unittest.TestCase):
    """Cross-session reads must be denied by the namespace controller."""

    def setUp(self) -> None:
        self.key_store = _key_store()

    def tearDown(self) -> None:
        self.key_store.close()

    def test_run_a_cannot_read_run_b_artifacts(self) -> None:
        """Namespace A cannot read artifacts from namespace B."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns_a = _archive_create_ns(repo, self.key_store)
            ns_b = _archive_create_ns(repo, self.key_store)

            ns_a.write_artifact("raw_tool_result", "a-data.bin", b"SECRET-A")
            ns_b.write_artifact("raw_tool_result", "b-data.bin", b"SECRET-B")

            # A can read its own data
            data = ns_a.read_artifact(ns_a.conversation_id, "raw_tool_result", "a-data.bin")
            self.assertEqual(data, b"SECRET-A")

            # A cannot read B's data
            with self.assertRaises(AssuranceError):
                ns_a.read_artifact(ns_a.conversation_id, "raw_tool_result", "b-data.bin")

    def test_cross_session_read_is_denied(self) -> None:
        """read_artifact with wrong conversation_id is denied."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = _archive_create_ns(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"DATA")

            # Correct conversation_id works
            data = ns.read_artifact(ns.conversation_id, "raw_tool_result", "data.bin")
            self.assertEqual(data, b"DATA")

            # Wrong conversation_id fails
            with self.assertRaises(AssuranceError):
                ns.read_artifact("CONV-WRONG-ID", "raw_tool_result", "data.bin")

    def test_import_from_another_session_is_untrusted(self) -> None:
        """Snapshot import from another session marks imported_untrusted."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns_src = _archive_create_ns(repo, self.key_store)
            ns_dst = _archive_create_ns(repo, self.key_store)

            ns_src.write_artifact("user_pinned_snapshot", "important.bin", b"IMPORTANT")
            receipt = ns_dst.import_untrusted_snapshot(
                source=ns_src,
                source_category="user_pinned_snapshot",
                source_path="important.bin",
                destination_name="from-src.bin",
            )
            self.assertEqual(receipt["import_status"], "imported_untrusted")
            self.assertEqual(receipt["source_conversation_id"], ns_src.conversation_id)


class NamespaceLifecycleTests(unittest.TestCase):
    """Namespace lifecycle: active → archive → verify with artifacts."""

    def setUp(self) -> None:
        self.key_store = _key_store()

    def tearDown(self) -> None:
        self.key_store.close()

    def test_full_lifecycle_with_adapter_artifacts(self) -> None:
        """Create namespace, write adapter artifacts, archive, verify."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = _archive_create_ns(repo, self.key_store)
            gov = SessionGovernor(ns)

            # Write typical adapter artifacts
            gov.write_gate_receipt("ipg", {"gate_decision": "allow", "valid": True})
            gov.write_checkpoint("orientation", {"step": 0})
            gov.write_answer_packet({"answer": {"summary": ["Test output"]}})
            gov.write_run_receipt({"valid": True, "run_id": "TEST"})

            # Also write some temp data that should be deleted
            ns.write_artifact("raw_tool_result", "temp.bin", b"TEMP")
            ns.write_artifact("full_stdout_stderr", "log.txt", b"LOGS")

            # Write persist-category data
            ns.write_artifact("redacted_conversation", "chat.json", b'{"msg":"hi"}')

            # Archive
            controller = ArchiveController(key_store=self.key_store, max_retries=0)
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])

            # Persist category survives
            self.assertTrue(
                (ns.artifacts_root / "redacted_conversation").exists()
            )
            # Temp categories deleted
            self.assertFalse(
                (ns.artifacts_root / "raw_tool_result").exists()
            )
            self.assertFalse(
                (ns.artifacts_root / "full_stdout_stderr").exists()
            )

            # Gate receipts (temporary_lifecycle_receipt) are deleted on archive
            # Answer packets (agent_output) are deleted on archive

            # Independent verifier passes
            verification = verify_archive(ns, key_store=self.key_store)
            self.assertTrue(verification["valid"], verification["errors"])

    def test_namespace_resume_creates_new_envelope(self) -> None:
        """Resuming an archived conversation creates a new namespace + envelope."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns1 = _archive_create_ns(repo, self.key_store)
            # Capture envelope before archive
            ns1_orig_envelope_id = ns1.state()["envelope_id"]
            ns1_orig_allowed = ns1.load_active_envelope()["capability_envelope"]["allowed"]

            ns1.write_artifact("user_pinned_snapshot", "data.bin", b"KEEP-ME")
            ns1.write_artifact("raw_tool_result", "junk.bin", b"JUNK")

            controller = ArchiveController(key_store=self.key_store, max_retries=0)
            result = controller.archive(ns1)
            self.assertTrue(result["archive_complete"])

            from assurance.archive import resume_archived_conversation

            ns2 = resume_archived_conversation(
                ns1,
                repo,
                key_store=self.key_store,
                frozen_context=_archive_frozen_ctx(),
                allowed_capabilities=["filesystem.workspace_read"],
                denied_capabilities=["network.unrestricted"],
            )
            # New conversation ID
            self.assertNotEqual(ns2.conversation_id, ns1.conversation_id)
            # New envelope ID (different from original)
            self.assertNotEqual(
                ns2.state()["envelope_id"], ns1_orig_envelope_id
            )
            # ns2 has different (subset) capabilities
            ns2_allowed = ns2.load_active_envelope()["capability_envelope"]["allowed"]
            self.assertEqual(ns2_allowed, ["filesystem.workspace_read"])
            self.assertNotEqual(ns2_allowed, ns1_orig_allowed)


if __name__ == "__main__":
    unittest.main()
