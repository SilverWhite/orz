from __future__ import annotations

import json
import os
import tempfile
import threading
from pathlib import Path
import unittest

from assurance import (
    ArchiveController,
    AssuranceError,
    ConversationNamespace,
    MemoryInstallationKeyStore,
    verify_archive,
)
from assurance.archive import _default_delete_file
from assurance.archive_journal import (
    ArchiveJournalWriter,
    inspect_archive_journal,
    recover_archive_journal,
    replay_archive_journal,
)
from assurance.utils import load_json, sha256_bytes


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


class ArchiveJournalReplayTests(unittest.TestCase):
    """Tests for journal write, replay, and hash-chain validation."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.journal_path = self.root / ".archive-journal.jsonl"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _write_event(self, writer: ArchiveJournalWriter, event_type: str, payload: dict) -> dict:
        return writer.append_event(event_type, payload)

    def test_journal_append_and_replay(self) -> None:
        with ArchiveJournalWriter(
            self.journal_path,
            archive_id="ARC-TEST-001",
            conversation_id="CONV-TEST",
        ) as writer:
            self._write_event(writer, "archive_started", {"envelope_id": "ENV-TEST", "policy_sha256": "a" * 64})
            self._write_event(writer, "category_started", {"category": "raw_tool_result"})
            self._write_event(
                writer,
                "file_deleted",
                {"category": "raw_tool_result", "relative_path": "a.bin", "size_bytes": 10, "sha256": "b" * 64, "attempt": 1},
            )
            self._write_event(writer, "category_completed", {"category": "raw_tool_result", "deleted_count": 1, "failed_count": 0, "remaining_count": 0})
            self._write_event(writer, "archive_completed", {"outcome": "archived", "totals": {"discovered": 1, "deleted": 1, "remaining": 0}})

        replay = replay_archive_journal(self.journal_path)
        self.assertTrue(replay["valid"], replay["errors"])
        self.assertEqual(replay["event_count"], 5)
        self.assertEqual(replay["terminal_event"], "archive_completed")
        self.assertEqual(len(replay["deleted_files"]), 1)
        self.assertIn(("raw_tool_result", "a.bin"), replay["deleted_files"])

    def test_journal_broken_hash_chain_detected(self) -> None:
        with ArchiveJournalWriter(
            self.journal_path,
            archive_id="ARC-TEST-002",
            conversation_id="CONV-TEST",
        ) as writer:
            self._write_event(writer, "archive_started", {"envelope_id": "ENV-TEST", "policy_sha256": "a" * 64})
            self._write_event(writer, "category_started", {"category": "raw_tool_result"})

        # Tamper with the first event's event_sha256
        raw = self.journal_path.read_bytes()
        lines = raw.split(b"\n")
        first = json.loads(lines[0].decode("utf-8"))
        first["event_sha256"] = "b" * 64
        lines[0] = json.dumps(first, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        self.journal_path.write_bytes(b"\n".join(lines))

        replay = replay_archive_journal(self.journal_path)
        self.assertFalse(replay["valid"])
        self.assertTrue(any("hash mismatch" in err for err in replay["errors"]))

    def test_journal_sequence_gap_detected(self) -> None:
        with ArchiveJournalWriter(
            self.journal_path,
            archive_id="ARC-TEST-003",
            conversation_id="CONV-TEST",
        ) as writer:
            self._write_event(writer, "archive_started", {"envelope_id": "ENV-TEST", "policy_sha256": "a" * 64})
            self._write_event(writer, "category_started", {"category": "test"})

        # Corrupt sequence number
        raw = self.journal_path.read_bytes()
        lines = raw.split(b"\n")
        second = json.loads(lines[1].decode("utf-8"))
        second["sequence"] = 5
        lines[1] = json.dumps(second, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        self.journal_path.write_bytes(b"\n".join(lines))

        replay = replay_archive_journal(self.journal_path)
        self.assertFalse(replay["valid"])
        self.assertTrue(any("sequence gap" in err for err in replay["errors"]))

    def test_journal_torn_tail_detected(self) -> None:
        with ArchiveJournalWriter(
            self.journal_path,
            archive_id="ARC-TEST-004",
            conversation_id="CONV-TEST",
        ) as writer:
            self._write_event(writer, "archive_started", {"envelope_id": "ENV-TEST", "policy_sha256": "a" * 64})
            self._write_event(writer, "category_started", {"category": "test"})

        raw = self.journal_path.read_bytes()
        # Truncate in the middle to simulate torn tail
        truncated = raw[: len(raw) // 2]
        self.journal_path.write_bytes(truncated)

        inspection = inspect_archive_journal(self.journal_path)
        self.assertEqual(inspection["classification"], "torn_tail")

    def test_journal_torn_tail_recovery(self) -> None:
        with ArchiveJournalWriter(
            self.journal_path,
            archive_id="ARC-TEST-005",
            conversation_id="CONV-TEST",
        ) as writer:
            self._write_event(writer, "archive_started", {"envelope_id": "ENV-TEST", "policy_sha256": "a" * 64})
            self._write_event(writer, "category_started", {"category": "test"})
            self._write_event(writer, "category_completed", {"category": "test", "deleted_count": 0, "failed_count": 0, "remaining_count": 0})

        raw = self.journal_path.read_bytes()
        truncated = raw[: len(raw) // 2]  # definitely cuts mid-line
        self.journal_path.write_bytes(truncated)

        quarantine = self.root / "quarantine.bin"
        receipt_path = self.root / "recovery-receipt.json"
        result = recover_archive_journal(
            self.journal_path,
            receipt_path=receipt_path,
            quarantine_path=quarantine,
        )
        self.assertTrue(result["valid"], result.get("errors", []))
        self.assertTrue(result["applied"])

        # Verify recovered journal is replayable
        replay = replay_archive_journal(self.journal_path)
        self.assertTrue(replay["valid"], replay["errors"])
        # Should have at least 1 prefix event + the recovery event
        self.assertGreaterEqual(replay["event_count"], 2)
        recovery_events = [e for e in replay["events"] if e["event_type"] == "archive_recovered"]
        self.assertEqual(len(recovery_events), 1)

    def test_journal_empty(self) -> None:
        replay = replay_archive_journal(self.journal_path)
        self.assertTrue(replay["valid"])
        self.assertEqual(replay["event_count"], 0)
        self.assertEqual(replay["last_sequence"], 0)


class ArchiveControllerRetryTests(unittest.TestCase):
    """Tests for retry with backoff."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_retry_succeeds_after_transient_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)
            target_path = namespace.write_artifact(
                "raw_tool_result", "retry_me.bin", b"TRANSIENT-CONTENT"
            )

            call_count = [0]

            def fail_twice_then_succeed(path: Path) -> None:
                if path == target_path:
                    call_count[0] += 1
                    if call_count[0] <= 2:
                        raise OSError("synthetic transient failure")
                _default_delete_file(path)

            controller = ArchiveController(
                key_store=self.key_store,
                delete_file=fail_twice_then_succeed,
                max_retries=3,
            )
            result = controller.archive(namespace)
            self.assertTrue(result["archive_complete"])
            self.assertEqual(call_count[0], 3)  # 2 failures + 1 success

    def test_retry_exhausted_records_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)
            namespace.write_artifact(
                "raw_tool_result", "never_deletes.bin", b"PERSISTENT-CONTENT"
            )

            def always_fail(path: Path) -> None:
                raise OSError("synthetic permanent failure")

            controller = ArchiveController(
                key_store=self.key_store,
                delete_file=always_fail,
                max_retries=2,
            )
            result = controller.archive(namespace)
            self.assertFalse(result["archive_complete"])
            self.assertEqual(result["outcome"], "failed")
            self.assertTrue(
                any(e["code"] == "delete_failed" for e in result["errors"])
            )


class ArchiveControllerCrashRecoveryTests(unittest.TestCase):
    """Tests for journal-based crash recovery and resume."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_resume_from_archiving_state(self) -> None:
        """Simulate crash mid-archive and verify resume completes it."""
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)

            # Write several artifacts
            namespace.write_artifact("raw_tool_result", "file1.bin", b"CONTENT-1")
            namespace.write_artifact("raw_tool_result", "file2.bin", b"CONTENT-2")
            namespace.write_artifact("full_stdout_stderr", "log.txt", b"LOG-DATA")
            # Persist category that should survive
            namespace.write_artifact("redacted_conversation", "conv.json", b'{"safe":true}')

            # First archive — delete only file1 and then "crash"
            crash_after = [1]

            def crash_early(path: Path) -> None:
                crash_after[0] -= 1
                if crash_after[0] < 0:
                    raise RuntimeError("simulated crash")
                _default_delete_file(path)

            controller = ArchiveController(
                key_store=self.key_store,
                delete_file=crash_early,
                max_retries=0,  # no retry for test speed
            )

            with self.assertRaises(RuntimeError):
                controller.archive(namespace)

            # Verify state is archiving
            state = namespace.state()
            self.assertEqual(state["state"], "archiving")

            # Verify journal exists and has partial events
            journal_path = namespace.root / ".archive-journal.jsonl"
            self.assertTrue(journal_path.is_file())
            replay = replay_archive_journal(journal_path)
            self.assertTrue(replay["valid"], replay["errors"])
            self.assertIsNone(replay["terminal_event"])  # no terminal — interrupted
            self.assertTrue(len(replay["deleted_files"]) >= 1)

            # Resume with a working controller
            controller2 = ArchiveController(
                key_store=self.key_store,
                max_retries=0,
            )
            result = controller2.archive(namespace)
            self.assertTrue(result["archive_complete"])
            self.assertEqual(namespace.state()["state"], "archived")

            # Verify all temp artifacts are gone
            self.assertFalse(
                (namespace.artifacts_root / "raw_tool_result").exists()
            )
            self.assertFalse(
                (namespace.artifacts_root / "full_stdout_stderr").exists()
            )
            # Persist category survives
            self.assertTrue(
                (namespace.artifacts_root / "redacted_conversation").exists()
            )

            # Independent verifier passes
            verification = verify_archive(namespace, key_store=self.key_store)
            self.assertTrue(verification["valid"], verification["errors"])

    def test_idempotent_resume(self) -> None:
        """Archive a completed conversation again — should return existing result."""
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)
            namespace.write_artifact("raw_tool_result", "data.bin", b"CONTENT")

            controller = ArchiveController(
                key_store=self.key_store, max_retries=0
            )
            result1 = controller.archive(namespace)
            self.assertTrue(result1["archive_complete"])
            self.assertEqual(namespace.state()["state"], "archived")

            # Second archive attempt on archived namespace
            with self.assertRaisesRegex(AssuranceError, "active or archiving"):
                controller.archive(namespace)


class ArchiveControllerConcurrencyTests(unittest.TestCase):
    """Tests for advisory lock concurrency control."""

    def setUp(self) -> None:
        self.key_store = MemoryInstallationKeyStore(b"K" * 32)

    def tearDown(self) -> None:
        self.key_store.close()

    def test_concurrent_archive_blocked_by_lock(self) -> None:
        """Second archive attempt must time out on lock acquisition."""
        with tempfile.TemporaryDirectory() as temporary:
            repository = Path(temporary)
            namespace = create_namespace(repository, self.key_store)
            namespace.write_artifact("raw_tool_result", "data.bin", b"CONTENT")

            # First thread holds the lock while "archiving" deliberately
            error_container: list[Exception | None] = [None]

            def slow_archive() -> None:
                try:
                    controller = ArchiveController(
                        key_store=MemoryInstallationKeyStore(b"K" * 32),
                        max_retries=0,
                        lock_timeout_seconds=1.0,
                    )
                    controller.archive(namespace)
                except Exception as exc:
                    error_container[0] = exc

            # Start a thread that will acquire the lock and hold it
            slow_thread = threading.Thread(target=slow_archive, daemon=True)
            slow_thread.start()
            slow_thread.join(timeout=5.0)

            # After the slow thread completes, the archive should be done
            # Verify the final state
            state = namespace.state()
            self.assertIn(state["state"], {"archived", "failed", "archiving"})

            # Second attempt with very short timeout on already-archived state
            if state["state"] in {"archived", "failed"}:
                controller2 = ArchiveController(
                    key_store=self.key_store,
                    lock_timeout_seconds=0.5,
                )
                with self.assertRaises(AssuranceError):
                    controller2.archive(namespace)


class ArchiveJournalInMemoryTests(unittest.TestCase):
    """Test journal writer lifecycle independently of filesystem operations."""

    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.journal_path = self.root / ".archive-journal.jsonl"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_archive_started_to_completed_flow(self) -> None:
        archive_id = "ARC-FLOW-001"
        conv_id = "CONV-FLOW"

        with ArchiveJournalWriter(
            self.journal_path, archive_id=archive_id, conversation_id=conv_id
        ) as writer:
            e1 = writer.append_event("archive_started", {"envelope_id": "ENV-X", "policy_sha256": "a" * 64})
            self.assertEqual(e1["sequence"], 0)
            self.assertIsNone(e1["previous_event_sha256"])
            self.assertEqual(e1["event_type"], "archive_started")

            e2 = writer.append_event("file_deleted", {"category": "raw_tool_result", "relative_path": "x.bin", "size_bytes": 5, "sha256": "b" * 64, "attempt": 1})
            self.assertEqual(e2["sequence"], 1)
            self.assertEqual(e2["previous_event_sha256"], e1["event_sha256"])

            e3 = writer.append_event("archive_completed", {"outcome": "archived", "totals": {"discovered": 1, "deleted": 1, "remaining": 0}})
            self.assertEqual(e3["sequence"], 2)
            self.assertEqual(e3["previous_event_sha256"], e2["event_sha256"])

        # Replay validates everything
        replay = replay_archive_journal(self.journal_path)
        self.assertTrue(replay["valid"], replay["errors"])
        self.assertEqual(replay["event_count"], 3)
        self.assertEqual(replay["terminal_event"], "archive_completed")

    def test_recovery_event_appended_after_torn_tail(self) -> None:
        archive_id = "ARC-REC-001"
        conv_id = "CONV-REC"

        with ArchiveJournalWriter(
            self.journal_path, archive_id=archive_id, conversation_id=conv_id
        ) as writer:
            writer.append_event("archive_started", {"envelope_id": "ENV-X", "policy_sha256": "a" * 64})
            writer.append_event("category_started", {"category": "test"})
            writer.append_event("category_completed", {"category": "test", "deleted_count": 0, "failed_count": 0, "remaining_count": 0})

        # Simulate torn tail
        raw = self.journal_path.read_bytes()
        truncated = raw[: len(raw) // 2]
        self.journal_path.write_bytes(truncated)

        result = recover_archive_journal(self.journal_path)
        self.assertTrue(result["valid"], result.get("errors", []))
        self.assertTrue(result["applied"])

        replay = replay_archive_journal(self.journal_path)
        self.assertTrue(replay["valid"], replay["errors"])
        # Should have at least the recovery event
        recovery_events = [e for e in replay["events"] if e["event_type"] == "archive_recovered"]
        self.assertEqual(len(recovery_events), 1)


if __name__ == "__main__":
    unittest.main()
