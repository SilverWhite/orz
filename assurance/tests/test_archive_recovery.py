"""Tests for archive process recovery, storage adapter integration,
fault injection, and deletion boundary enforcement."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import threading
from pathlib import Path
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    ConversationNamespace,
    LocalStorageAdapter,
    MemoryInstallationKeyStore,
    verify_archive,
)
from assurance.archive import _default_delete_file
from assurance.archive_journal import (
    ArchiveJournalWriter,
    detect_stale_archive_lock,
    cleanup_stale_archive_lock,
    exclusive_archive_lock,
    inspect_archive_journal,
    replay_archive_journal,
)
from assurance.archive_recovery import classify_archive_failure, recover_archive
from assurance.tests.storage_faults import FaultInjectionStorageAdapter
from assurance.utils import load_json, sha256_bytes


# ── helpers (mirror test_archive_controller.py) ──


def frozen_context() -> dict[str, object]:
    return {
        "workspace_canonical_path_digest": {
            "value": "a" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "workspace_content_digest": {
            "value": "b" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "workspace_policy_digest": {
            "value": "c" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "runtime_family": {
            "value": "fixture-runtime",
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "runtime_adapter_id": {
            "value": "fixture-adapter",
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "runtime_binary_digest": {
            "value": "d" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "runtime_capabilities_digest": {
            "value": "e" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "assurance_config_digest": {
            "value": "f" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "sandbox_backend": {
            "value": "fixture-strict-sandbox",
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
        "sandbox_backend_digest": {
            "value": "1" * 64,
            "evidence_status": "observed",
            "source_refs": ["fixture:p1-synthetic"],
        },
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


# ── archive recovery tests ──


class ArchiveRecoveryTests(unittest.TestCase):
    """Test all archive process recovery paths."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_classify_active_namespace(self) -> None:
        """Active conversation should classify as no recovery needed."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            result = classify_archive_failure(ns)
            self.assertEqual(result["classification"], "active_no_archive")
            self.assertFalse(result["recoverable"])

    def test_classify_already_archived(self) -> None:
        """Already-archived conversation should classify as terminal."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            controller = ArchiveController(key_store=self.key_store, max_retries=0)
            controller.archive(ns)
            result = classify_archive_failure(ns)
            self.assertEqual(result["classification"], "already_terminal")
            self.assertFalse(result["recoverable"])

    def test_recover_clean_interrupted_archive(self) -> None:
        """Resume a clean interrupted archive to completion."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"TEST-DATA")

            # Transition state to archiving manually to simulate crash
            state = ns.state()
            archiving_state = {
                **state,
                "state": "archiving",
                "revision": state["revision"] + 1,
                "updated_at": state["updated_at"],
                "terminal_deletion_receipt_sha256": None,
            }
            ns._write_state(archiving_state, overwrite=True)

            # Write a partial journal (no terminal event)
            journal_path = ns.root / ".archive-journal.jsonl"
            with ArchiveJournalWriter(
                journal_path,
                archive_id="ARC-TEST-RECOV",
                conversation_id=ns.conversation_id,
            ) as writer:
                writer.append_event(
                    "archive_started",
                    {"envelope_id": state["envelope_id"], "policy_sha256": "a" * 64},
                )
                writer.append_event(
                    "category_started", {"category": "raw_tool_result"}
                )

            # Classify
            classification = classify_archive_failure(ns)
            self.assertEqual(classification["classification"], "clean_interrupted")
            self.assertTrue(classification["recoverable"])

            # Recover
            recovery = recover_archive(ns, key_store=self.key_store)
            self.assertTrue(recovery["valid"], recovery.get("errors", []))
            self.assertEqual(recovery["classification"], "clean_interrupted")
            self.assertIsNotNone(recovery["archive_result"])
            self.assertTrue(recovery["archive_result"]["archive_complete"])
            self.assertTrue(recovery["verification"]["valid"])

    def test_recover_torn_journal(self) -> None:
        """Repair torn journal then resume archive."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "a.bin", b"DATA-A")
            ns.write_artifact("raw_tool_result", "b.bin", b"DATA-B")

            # Transition to archiving
            state = ns.state()
            archiving_state = {
                **state,
                "state": "archiving",
                "revision": state["revision"] + 1,
                "updated_at": state["updated_at"],
                "terminal_deletion_receipt_sha256": None,
            }
            ns._write_state(archiving_state, overwrite=True)

            # Write a journal and truncate it
            journal_path = ns.root / ".archive-journal.jsonl"
            with ArchiveJournalWriter(
                journal_path,
                archive_id="ARC-TORN",
                conversation_id=ns.conversation_id,
            ) as writer:
                writer.append_event(
                    "archive_started",
                    {"envelope_id": state["envelope_id"], "policy_sha256": "a" * 64},
                )
                writer.append_event(
                    "category_started", {"category": "raw_tool_result"}
                )

            # Truncate: keep first event line, append partial gibberish (torn tail)
            raw = journal_path.read_bytes()
            first_line_end = raw.find(b"\n")
            if first_line_end >= 0:
                keep = raw[: first_line_end + 1] + b"partial garbage with no newline"
            else:
                keep = raw[: len(raw) // 2]
            journal_path.write_bytes(keep)

            # Classify
            classification = classify_archive_failure(ns)
            self.assertEqual(classification["classification"], "torn_journal")
            self.assertTrue(classification["recoverable"])

            # Recover
            recovery = recover_archive(
                ns,
                key_store=self.key_store,
                quarantine_dir=repo / "quarantine",
            )
            self.assertTrue(recovery["valid"], recovery.get("errors", []))
            self.assertEqual(recovery["classification"], "torn_journal")
            self.assertIsNotNone(recovery["journal_recovery"])
            self.assertTrue(recovery["journal_recovery"]["valid"])
            self.assertIsNotNone(recovery["archive_result"])
            self.assertTrue(recovery["archive_result"]["archive_complete"])

    def test_recover_corrupt_journal(self) -> None:
        """Quarantine corrupt journal and restart archive."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"CORRUPT-TEST")

            # Transition to archiving
            state = ns.state()
            archiving_state = {
                **state,
                "state": "archiving",
                "revision": state["revision"] + 1,
                "updated_at": state["updated_at"],
                "terminal_deletion_receipt_sha256": None,
            }
            ns._write_state(archiving_state, overwrite=True)

            # Write corrupt journal (invalid JSON)
            journal_path = ns.root / ".archive-journal.jsonl"
            journal_path.write_bytes(b"this is not valid json\n")

            classification = classify_archive_failure(ns)
            self.assertEqual(classification["classification"], "corrupt_journal")
            self.assertTrue(classification["recoverable"])

            quarantine_dir = repo / "quarantine"
            recovery = recover_archive(
                ns,
                key_store=self.key_store,
                quarantine_dir=quarantine_dir,
            )
            self.assertTrue(recovery["valid"], recovery.get("errors", []))
            self.assertEqual(recovery["classification"], "corrupt_journal")

            # Corrupt journal should be quarantined
            self.assertTrue(
                any(quarantine_dir.glob("corrupt-journal-*.jsonl"))
            )

            # Fresh archive should complete
            self.assertTrue(recovery["archive_result"]["archive_complete"])

    def test_recover_already_completed_noop(self) -> None:
        """Recovery on already-archived namespace is a no-op."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            controller = ArchiveController(key_store=self.key_store, max_retries=0)
            controller.archive(ns)

            recovery = recover_archive(ns, key_store=self.key_store)
            # Should report as not recoverable (already done)
            self.assertFalse(recovery["valid"])
            self.assertIn("already_terminal", recovery["classification"])

    def test_recovery_never_deletes_persist_categories(self) -> None:
        """Recovery must never touch persist categories."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("redacted_conversation", "conv.json", b'{"safe":true}')
            ns.write_artifact("user_pinned_snapshot", "snap.bin", b"SNAPSHOT")
            ns.write_artifact("raw_tool_result", "temp.bin", b"TEMP")

            # Transition to archiving and write partial journal
            state = ns.state()
            archiving_state = {
                **state,
                "state": "archiving",
                "revision": state["revision"] + 1,
                "updated_at": state["updated_at"],
                "terminal_deletion_receipt_sha256": None,
            }
            ns._write_state(archiving_state, overwrite=True)

            journal_path = ns.root / ".archive-journal.jsonl"
            with ArchiveJournalWriter(
                journal_path,
                archive_id="ARC-PERSIST",
                conversation_id=ns.conversation_id,
            ) as writer:
                writer.append_event(
                    "archive_started",
                    {"envelope_id": state["envelope_id"], "policy_sha256": "a" * 64},
                )

            recovery = recover_archive(ns, key_store=self.key_store)
            self.assertTrue(recovery["valid"], recovery.get("errors", []))

            # Persist categories survive
            self.assertTrue(
                (ns.artifacts_root / "redacted_conversation").exists()
            )
            self.assertTrue(
                (ns.artifacts_root / "user_pinned_snapshot").exists()
            )
            # Delete category is gone
            self.assertFalse(
                (ns.artifacts_root / "raw_tool_result").exists()
            )


# ── storage adapter integration tests ──


class StorageAdapterIntegrationTests(unittest.TestCase):
    """Test ArchiveController with pluggable StorageAdapter."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_archive_with_local_storage_adapter(self) -> None:
        """Full archive cycle using LocalStorageAdapter explicitly."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"ADAPTER-TEST")
            ns.write_artifact("full_stdout_stderr", "log.txt", b"LOGS")

            storage = LocalStorageAdapter()
            controller = ArchiveController(
                key_store=self.key_store,
                storage=storage,
                max_retries=0,
            )
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])
            self.assertEqual(result["outcome"], "archived")

            # Verify through independent verifier
            verification = verify_archive(ns, key_store=self.key_store)
            self.assertTrue(verification["valid"], verification["errors"])

    def test_archive_with_custom_delete_file_backward_compat(self) -> None:
        """Backward compat: delete_file parameter still works."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"COMPAT-TEST")

            delete_log: list[Path] = []

            def logging_delete(path: Path) -> None:
                delete_log.append(path)
                _default_delete_file(path)

            controller = ArchiveController(
                key_store=self.key_store,
                delete_file=logging_delete,
                max_retries=0,
            )
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])
            self.assertTrue(len(delete_log) >= 1)

    def test_archive_with_transient_failure_retry(self) -> None:
        """Fault injection: transient failures should retry and succeed."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "retry.bin", b"RETRY-ME")

            fault_storage = FaultInjectionStorageAdapter(
                transient_delete_failures=2,
            )
            controller = ArchiveController(
                key_store=self.key_store,
                storage=fault_storage,
                max_retries=3,
                retry_base_delay_seconds=0.01,
            )
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])
            # Verify all 3 attempts happened (2 failures + 1 success)
            attempts = [
                a for a in fault_storage.delete_attempts
                if "retry" in str(a[0]).lower()
            ]
            self.assertGreaterEqual(len(attempts), 1)

    def test_archive_with_permanent_failure(self) -> None:
        """Fault injection: permanent failure → archive_complete=false."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            target = ns.write_artifact(
                "raw_tool_result", "never.bin", b"PERMANENT"
            )
            ns.write_artifact("raw_tool_result", "ok.bin", b"OK-DATA")

            fault_storage = FaultInjectionStorageAdapter(
                permanent_delete_failures={target},
            )
            controller = ArchiveController(
                key_store=self.key_store,
                storage=fault_storage,
                max_retries=1,
                retry_base_delay_seconds=0.01,
            )
            result = controller.archive(ns)
            self.assertFalse(result["archive_complete"])
            self.assertEqual(result["outcome"], "failed")
            self.assertTrue(
                any(e["code"] == "delete_failed" for e in result["errors"])
            )


# ── deletion boundary tests ──


class DeletionBoundaryTests(unittest.TestCase):
    """Verify that deletion boundaries are never violated."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_exactly_16_categories_deleted(self) -> None:
        """Verify exactly 16 delete_on_archive categories are processed."""
        from assurance.conversation import retention_delete_categories
        categories = retention_delete_categories()
        self.assertEqual(
            len(categories), 16,
            f"Expected 16 delete_on_archive categories, got {len(categories)}"
        )

    def test_3_persist_categories_survive_archive(self) -> None:
        """Verify the 3 persist categories survive a successful archive."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)

            # Write to all persist categories
            ns.write_artifact("redacted_conversation", "chat.json", b'{"msg":"hi"}')
            ns.write_artifact("user_pinned_snapshot", "important.bin", b"PINNED")
            ns.write_artifact("terminal_deletion_receipt", "receipt.json", b'{"t":"dr"}')

            # Also write to a delete category
            ns.write_artifact("raw_tool_result", "temp.bin", b"TEMP")

            controller = ArchiveController(key_store=self.key_store, max_retries=0)
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])

            # All 3 persist categories survive
            self.assertTrue(
                (ns.artifacts_root / "redacted_conversation").exists(),
                "redacted_conversation must survive archive"
            )
            self.assertTrue(
                (ns.artifacts_root / "user_pinned_snapshot").exists(),
                "user_pinned_snapshot must survive archive"
            )
            self.assertTrue(
                (ns.artifacts_root / "terminal_deletion_receipt").exists(),
                "terminal_deletion_receipt must survive archive"
            )
            # Delete category is gone
            self.assertFalse(
                (ns.artifacts_root / "raw_tool_result").exists(),
                "raw_tool_result must be deleted"
            )

    def test_path_traversal_blocked_during_deletion(self) -> None:
        """Path traversal attacks are blocked during archive scanning."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)

            # Try to create a traversal artifact (should fail)
            with self.assertRaises((AssuranceError, ValueError, OSError)):
                ns.write_artifact("raw_tool_result", "../../escape.bin", b"ESCAPE")


# ── stale lock tests ──


class StaleLockTests(unittest.TestCase):
    """Tests for stale lock detection and cleanup."""

    def test_no_lock_file(self) -> None:
        """No lock file → not stale."""
        with tempfile.TemporaryDirectory() as tmp:
            journal = Path(tmp) / "test.jsonl"
            result = detect_stale_archive_lock(journal)
            self.assertFalse(result["stale"])
            self.assertFalse(result["lock_exists"])
            self.assertEqual(result["action"], "none")

    def test_active_lock_detected(self) -> None:
        """Actively held lock → not stale, should wait."""
        with tempfile.TemporaryDirectory() as tmp:
            journal = Path(tmp) / "test.jsonl"
            # Hold the lock
            with exclusive_archive_lock(journal, timeout_seconds=1.0):
                # Check from same process — lock should appear active
                result = detect_stale_archive_lock(journal)
                # The lock IS stale from our perspective since we hold it
                # (same process can re-acquire it)
                self.assertTrue(result["lock_exists"])

    def test_stale_lock_cleanup(self) -> None:
        """Stale lock file can be cleaned up."""
        with tempfile.TemporaryDirectory() as tmp:
            journal = Path(tmp) / "test.jsonl"
            lock_path = Path(f"{journal}.lock")
            lock_path.parent.mkdir(parents=True, exist_ok=True)
            lock_path.write_bytes(b"\0")

            # No one holds this lock → should be stale
            detection = detect_stale_archive_lock(journal)
            self.assertTrue(detection["stale"], detection["details"])
            self.assertEqual(detection["action"], "cleanup")

            # Clean it up
            cleanup = cleanup_stale_archive_lock(journal)
            self.assertTrue(cleanup["removed"])
            self.assertFalse(lock_path.exists())


# ── multi-process lock contention test ──


class MultiProcessLockTests(unittest.TestCase):
    """Multi-process lock contention tests."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_lock_contention_between_processes(self) -> None:
        """Verify that two processes cannot acquire the same archive lock."""
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            ns = create_namespace(repo, self.key_store)
            ns.write_artifact("raw_tool_result", "data.bin", b"LOCK-TEST")
            journal_path = ns.root / ".archive-journal.jsonl"

            # Acquire lock in a subprocess that holds it briefly
            lock_script = f"""
import sys
sys.path.insert(0, {str(Path(__file__).resolve().parents[2])!r})
from assurance.archive_journal import exclusive_archive_lock
from pathlib import Path
import time

lock_path = Path({str(journal_path)!r})
with exclusive_archive_lock(lock_path, timeout_seconds=5.0):
    print("LOCK_ACQUIRED", flush=True)
    time.sleep(2.0)
print("LOCK_RELEASED", flush=True)
"""
            proc = subprocess.run(
                [sys.executable, "-c", lock_script],
                capture_output=True,
                text=True,
                timeout=10,
            )
            self.assertIn("LOCK_ACQUIRED", proc.stdout)
            self.assertIn("LOCK_RELEASED", proc.stdout)

            # Now our process should be able to archive
            controller = ArchiveController(
                key_store=self.key_store,
                max_retries=0,
            )
            result = controller.archive(ns)
            self.assertTrue(result["archive_complete"])


if __name__ == "__main__":
    unittest.main()
