from __future__ import annotations

from copy import deepcopy
import os
from pathlib import Path
import tempfile
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    WindowsDpapiInstallationKeyStore,
    resume_archived_conversation,
    verify_archive,
    verify_security_envelope,
)
from assurance.utils import atomic_write_json, load_json


def evidenced(value: str, *, sha256: bool = False) -> dict[str, object]:
    if sha256:
        assert len(value) == 64
    return {
        "value": value,
        "evidence_status": "observed",
        "source_refs": ["fixture:p1-synthetic"],
    }


def frozen_context(runtime_family: str = "fixture-runtime") -> dict[str, object]:
    return {
        "workspace_canonical_path_digest": evidenced("a" * 64, sha256=True),
        "workspace_content_digest": evidenced("b" * 64, sha256=True),
        "workspace_policy_digest": evidenced("c" * 64, sha256=True),
        "runtime_family": evidenced(runtime_family),
        "runtime_adapter_id": evidenced("fixture-adapter"),
        "runtime_binary_digest": evidenced("d" * 64, sha256=True),
        "runtime_capabilities_digest": evidenced("e" * 64, sha256=True),
        "assurance_config_digest": evidenced("f" * 64, sha256=True),
        "sandbox_backend": evidenced("fixture-strict-sandbox"),
        "sandbox_backend_digest": evidenced("1" * 64, sha256=True),
    }


def create_namespace(
    root: Path, key_store: MemoryInstallationKeyStore
) -> ConversationNamespace:
    return ConversationNamespace.create(
        root,
        key_store=key_store,
        frozen_context=frozen_context(),
        allowed_capabilities=[
            "filesystem.workspace_read",
            "filesystem.workspace_write",
        ],
        denied_capabilities=["network.unrestricted", "secret.raw_read"],
    )


class P1IdentityAndRetentionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_envelope_is_signed_and_tampering_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            namespace = create_namespace(Path(temporary), self.key_store)
            envelope = namespace.load_active_envelope()
            verification = verify_security_envelope(
                envelope, key_store=self.key_store
            )
            self.assertTrue(verification["valid"])
            self.assertEqual(
                envelope["integrity"]["key_store"]["value"], "memory-test-only"
            )

            tampered = deepcopy(envelope)
            tampered["frozen_context"]["runtime_family"]["value"] = "other-runtime"
            with self.assertRaisesRegex(
                AssuranceError, "signed payload digest mismatch"
            ):
                verify_security_envelope(tampered, key_store=self.key_store)

    def test_cross_conversation_read_is_denied_and_import_is_untrusted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            source = create_namespace(repository, self.key_store)
            destination = create_namespace(repository, self.key_store)
            source.write_artifact(
                "redacted_conversation", "conversation.json", b'{"safe":true}'
            )
            with self.assertRaisesRegex(AssuranceError, "cross-conversation"):
                source.read_artifact(
                    destination.conversation_id,
                    "redacted_conversation",
                    "conversation.json",
                )
            receipt = destination.import_untrusted_snapshot(
                source=source,
                source_category="redacted_conversation",
                source_path="conversation.json",
                destination_name="imported.json",
            )
            self.assertEqual(receipt["import_status"], "imported_untrusted")
            self.assertEqual(
                receipt["source_conversation_id"], source.conversation_id
            )
            ArchiveController(key_store=self.key_store).archive(destination)
            self.assertFalse(
                (
                    destination.artifacts_root / "temporary_import_receipt"
                ).exists()
            )
            verification = verify_archive(
                destination, key_store=self.key_store
            )
            self.assertTrue(verification["valid"], verification["errors"])

    def test_artifact_paths_fail_closed_on_traversal(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            namespace = create_namespace(Path(temporary), self.key_store)
            with self.assertRaisesRegex(AssuranceError, "unsafe relative path"):
                namespace.write_artifact(
                    "raw_tool_result", "../outside.bin", b"ESCAPE"
                )
            with self.assertRaisesRegex(AssuranceError, "unknown retention category"):
                namespace.write_artifact(
                    "arbitrary_category", "file.bin", b"OUT-OF-POLICY"
                )

    def test_successful_archive_deletes_temporary_data_and_retains_minimum(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            namespace = create_namespace(Path(temporary), self.key_store)
            namespace.write_artifact(
                "raw_provider_payload", "provider.bin", b"RAW-PROVIDER-SECRET"
            )
            namespace.write_artifact(
                "full_stdout_stderr", "process.log", b"RAW-PROCESS-LOG"
            )
            namespace.write_artifact(
                "unpinned_snapshot", "scratch.bin", b"UNPINNED-SNAPSHOT"
            )
            namespace.write_artifact(
                "redacted_conversation", "conversation.json", b'{"redacted":true}'
            )
            namespace.write_artifact(
                "user_pinned_snapshot", "important.json", b'{"pinned":true}'
            )

            result = ArchiveController(key_store=self.key_store).archive(namespace)
            self.assertTrue(result["archive_complete"])
            verification = verify_archive(namespace, key_store=self.key_store)
            self.assertTrue(verification["valid"], verification["errors"])
            self.assertTrue(verification["archive_complete"])
            self.assertEqual(namespace.state()["state"], "archived")
            self.assertFalse(
                (namespace.artifacts_root / "raw_provider_payload").exists()
            )
            self.assertFalse(
                (namespace.artifacts_root / "active_security_envelope").exists()
            )
            self.assertEqual(
                namespace.read_artifact(
                    namespace.conversation_id,
                    "redacted_conversation",
                    "conversation.json",
                ),
                b'{"redacted":true}',
            )
            receipt_text = (
                namespace.receipts_root / "archive-deletion.json"
            ).read_text(encoding="utf-8")
            self.assertNotIn("provider.bin", receipt_text)
            self.assertNotIn("RAW-PROVIDER-SECRET", receipt_text)
            self.assertNotIn("process.log", receipt_text)

    def test_delete_failure_reaches_failed_terminal_and_blocks_resume(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)
            blocked = namespace.write_artifact(
                "raw_provider_payload", "blocked.bin", b"DELETE-ME"
            )

            def fail_one(path: Path) -> None:
                if path == blocked:
                    raise OSError("synthetic deletion failure")
                path.unlink()

            result = ArchiveController(
                key_store=self.key_store, delete_file=fail_one
            ).archive(namespace)
            self.assertFalse(result["archive_complete"])
            self.assertEqual(result["outcome"], "failed")
            self.assertTrue(blocked.exists())
            verification = verify_archive(namespace, key_store=self.key_store)
            self.assertTrue(verification["valid"], verification["errors"])
            self.assertFalse(verification["archive_complete"])
            self.assertEqual(namespace.state()["state"], "failed")
            with self.assertRaisesRegex(AssuranceError, "verified complete archive"):
                resume_archived_conversation(
                    namespace,
                    repository,
                    key_store=self.key_store,
                    frozen_context=frozen_context(),
                    allowed_capabilities=["filesystem.workspace_read"],
                    denied_capabilities=["network.unrestricted"],
                )

    def test_resume_creates_new_conversation_and_new_envelope(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            previous = create_namespace(repository, self.key_store)
            old_envelope = previous.load_active_envelope()
            previous.write_artifact(
                "redacted_conversation", "conversation.json", b'{"turns":[]}'
            )
            ArchiveController(key_store=self.key_store).archive(previous)

            resumed = resume_archived_conversation(
                previous,
                repository,
                key_store=self.key_store,
                frozen_context=frozen_context("replacement-runtime"),
                allowed_capabilities=["filesystem.workspace_read"],
                denied_capabilities=[
                    "filesystem.workspace_write",
                    "network.unrestricted",
                ],
            )
            new_envelope = resumed.load_active_envelope()
            self.assertNotEqual(
                resumed.conversation_id, previous.conversation_id
            )
            self.assertNotEqual(
                new_envelope["envelope_id"], old_envelope["envelope_id"]
            )
            self.assertEqual(
                new_envelope["resumed_from_conversation_id"],
                previous.conversation_id,
            )
            self.assertEqual(
                new_envelope["parent_envelope_id"], old_envelope["envelope_id"]
            )
            self.assertNotEqual(
                new_envelope["capability_envelope"]["capability_digest"]["value"],
                old_envelope["capability_envelope"]["capability_digest"]["value"],
            )
            with self.assertRaisesRegex(AssuranceError, "cross-conversation"):
                resumed.read_artifact(
                    previous.conversation_id,
                    "redacted_conversation",
                    "conversation.json",
                )

    def test_terminal_receipt_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            namespace = create_namespace(Path(temporary), self.key_store)
            namespace.write_artifact(
                "raw_tool_result", "tool.bin", b"TOOL-RESULT"
            )
            ArchiveController(key_store=self.key_store).archive(namespace)
            receipt_path = namespace.receipts_root / "archive-deletion.json"
            receipt = load_json(receipt_path)
            receipt["totals"]["deleted"] += 1
            atomic_write_json(receipt_path, receipt, overwrite=True)
            verification = verify_archive(namespace, key_store=self.key_store)
            self.assertFalse(verification["valid"])
            self.assertTrue(
                any("digest mismatch" in error for error in verification["errors"])
            )

    @unittest.skipUnless(os.name == "nt", "Windows DPAPI is Windows-only")
    def test_windows_dpapi_installation_key_roundtrip_and_tamper(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "keystore"
            created = WindowsDpapiInstallationKeyStore.create(root)
            payload = b"synthetic-envelope-payload"
            signature = created.sign(payload)
            self.assertTrue(created.verify(payload, signature))
            reopened = WindowsDpapiInstallationKeyStore(root)
            self.assertEqual(reopened.key_id, created.key_id)
            self.assertTrue(reopened.verify(payload, signature))
            metadata_text = reopened.metadata_path.read_text(encoding="utf-8")
            self.assertNotIn(signature, metadata_text)
            self.assertNotIn("synthetic-envelope-payload", metadata_text)
            with self.assertRaisesRegex(AssuranceError, "refusing to overwrite"):
                WindowsDpapiInstallationKeyStore.create(root)

            stored = bytearray(reopened.blob_path.read_bytes())
            stored[-1] ^= 0x01
            reopened.blob_path.write_bytes(stored)
            with self.assertRaisesRegex(AssuranceError, "digest mismatch"):
                reopened.verify(payload, signature)


if __name__ == "__main__":
    unittest.main()
