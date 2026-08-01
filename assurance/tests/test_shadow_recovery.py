"""Tests for shadow recovery store and recovery executor — GAK-REC-001.

Covers:
1. ShadowRecoveryStore: store/retrieve/verify/list (Git-backed)
2. RecoveryDiffPreview: metadata-level diff
3. RecoveryExecutor: execute/verify/reject/fail
4. End-to-end: candidate → authorize → store → execute → verify
"""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path

from assurance import (
    AssuranceError,
    MemoryInstallationKeyStore,
)
from assurance.shadow_recovery import (
    DiffPreview,
    ExecutionReceipt,
    RecoveryDiffPreview,
    RecoveryExecutor,
    ShadowRecoveryStore,
    StoreEntry,
    verify_execution_receipt,
)
from assurance.utils import sha256_bytes, canonical_bytes


# ── test helpers ──────────────────────────────────────────────────────────────


def _make_candidate(
    candidate_id: str = "RCV-TEST0001",
    source_conv: str = "conv-source-001",
    snapshot_sha: str | None = None,
    snapshot_bytes_val: int = 0,
) -> dict:
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "recovery_candidate",
        "receipt_id": f"RCR-{candidate_id}",
        "candidate_id": candidate_id,
        "created_at": "2026-07-29T00:00:00Z",
        "source": {
            "conversation_id": source_conv,
            "audit_seal_sha256": "a" * 64,
            "snapshot_relative_path": f"artifacts/user_pinned_snapshot/checkpoint-{candidate_id}.bin",
            "snapshot_sha256": snapshot_sha or "b" * 64,
            "snapshot_bytes": snapshot_bytes_val,
            "classification": "untrusted_recovery_candidate",
        },
        "destination": {
            "conversation_id": "conv-dest-001",
            "envelope_id": "ENV-001",
            "target_workspace_sha256": "c" * 64,
        },
        "decision": {
            "state": "candidate_untrusted",
            "authority_effect": "none",
            "action_authorized": False,
            "permit_required": True,
            "restoration_performed": False,
            "reason_codes": ["RECOVERY_CANDIDATE_REQUIRES_SEPARATE_AUTHORIZATION"],
        },
    }
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": "test-key",
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": "0" * 64,
        },
    }


def _make_authorization(
    candidate_id: str = "RCV-TEST0001",
    outcome: str = "allow",
    restoration_performed: bool = False,
) -> dict:
    body = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "recovery_authorization",
        "receipt_id": f"RAR-{candidate_id}",
        "candidate_id": candidate_id,
        "conversation_id": "conv-dest-001",
        "envelope_id": "ENV-001",
        "created_at": "2026-07-29T00:00:01Z",
        "candidate_sha256": "d" * 64,
        "binding": {
            "action_sha256": "e" * 64,
            "target_workspace_sha256": "c" * 64,
            "snapshot_sha256": "b" * 64,
            "attempt": 1,
        },
        "permit_consumption_sha256": "f" * 64,
        "decision": {
            "outcome": outcome,
            "authority_source": (
                "envelope_and_one_shot_permit" if outcome == "allow" else "none"
            ),
            "restoration_performed": restoration_performed,
            "reason_codes": (
                ["RECOVERY_EXACT_ONE_SHOT_PERMIT_CONSUMED"]
                if outcome == "allow"
                else ["RECOVERY_ONE_SHOT_PERMIT_REQUIRED"]
            ),
        },
    }
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": "test-key",
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": "0" * 64,
        },
    }


# ══════════════════════════════════════════════════════════════════════════════
# 1. ShadowRecoveryStore tests
# ══════════════════════════════════════════════════════════════════════════════


class ShadowRecoveryStoreTests(unittest.TestCase):
    """Tests for :class:`ShadowRecoveryStore` (Git-backed)."""

    def setUp(self) -> None:
        # Isolate each test in its own temp directory containing a fresh Git repo.
        self._tmp = tempfile.TemporaryDirectory()
        self.store = ShadowRecoveryStore(repo_root=Path(self._tmp.name))

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def test_store_and_retrieve_cycle(self) -> None:
        """Store → retrieve produces matching entry."""
        candidate = _make_candidate("RCV-STORE-001")
        auth = _make_authorization("RCV-STORE-001")
        snapshot = b"recovery checkpoint data v1"

        entry = self.store.store(candidate, auth, snapshot)
        self.assertEqual(entry.snapshot_sha256, sha256_bytes(snapshot))
        self.assertEqual(entry.snapshot_bytes, len(snapshot))
        self.assertIsNotNone(entry.authorization_sha256)
        self.assertTrue(entry.commit_sha)
        self.assertIsNone(entry.entry_sha256)  # Git mode: no SHA-256 entry key

        retrieved = self.store.retrieve(entry.commit_sha)
        self.assertEqual(retrieved.commit_sha, entry.commit_sha)
        self.assertEqual(retrieved.candidate_sha256, entry.candidate_sha256)
        self.assertEqual(retrieved.snapshot_sha256, entry.snapshot_sha256)
        self.assertEqual(retrieved.snapshot_bytes, len(snapshot))

    def test_store_without_authorization(self) -> None:
        """Store without authorization (pre-auth candidate) is valid."""
        candidate = _make_candidate("RCV-NOAUTH-001")
        snapshot = b"pre-auth snapshot"

        entry = self.store.store(candidate, None, snapshot)
        self.assertIsNone(entry.authorization_sha256)
        self.assertTrue(entry.commit_sha)

        retrieved = self.store.retrieve(entry.commit_sha)
        self.assertIsNone(retrieved.authorization_sha256)

    def test_verify_entry_passes_for_valid_entry(self) -> None:
        """verify_entry returns True for valid entries."""
        candidate = _make_candidate("RCV-VERIFY-001")
        auth = _make_authorization("RCV-VERIFY-001")
        snapshot = b"verifiable data"

        entry = self.store.store(candidate, auth, snapshot)
        self.assertTrue(self.store.verify_entry(entry.commit_sha))

    def test_verify_entry_fails_for_missing_entry(self) -> None:
        """verify_entry returns False for non-existent commits."""
        fake_sha = "f" * 40  # 40-char hex for Git commit SHA
        self.assertFalse(self.store.verify_entry(fake_sha))

    def test_tampered_git_object_detected_on_verify(self) -> None:
        """Corrupting a Git object file causes verify_entry to fail."""
        candidate = _make_candidate("RCV-TAMPER-001")
        auth = _make_authorization("RCV-TAMPER-001")
        snapshot = b"data to tamper"

        entry = self.store.store(candidate, auth, snapshot)
        self.assertTrue(self.store.verify_entry(entry.commit_sha))

        # Corrupt a blob object in the Git object store.
        # Git object files are read-only on Windows, so chmod first.
        objects_dir = self.store._repo / ".git" / "objects"
        for obj_root, _dirs, files in os.walk(objects_dir):
            for fname in files:
                obj_path = Path(obj_root) / fname
                obj_path.chmod(0o644)
                obj_path.write_bytes(b"corrupted git object")
                break
            else:
                continue
            break

        self.assertFalse(self.store.verify_entry(entry.commit_sha))

    def test_list_entries(self) -> None:
        """list_entries returns all stored entries."""
        snap1 = b"first snapshot"
        snap2 = b"second snapshot"

        entry1 = self.store.store(
            _make_candidate("RCV-LIST-001"), _make_authorization("RCV-LIST-001"), snap1,
        )
        entry2 = self.store.store(
            _make_candidate("RCV-LIST-002"), _make_authorization("RCV-LIST-002"), snap2,
        )

        entries = self.store.list_entries()
        entry_ids = {e.entry_id for e in entries}
        self.assertIn(entry1.entry_id, entry_ids)
        self.assertIn(entry2.entry_id, entry_ids)

    def test_store_empty_snapshot_raises(self) -> None:
        """Empty snapshot raises AssuranceError."""
        candidate = _make_candidate("RCV-EMPTY-001")
        with self.assertRaises(AssuranceError):
            self.store.store(candidate, None, b"")

    def test_duplicate_store_is_idempotent(self) -> None:
        """Storing the same data twice produces the same commit SHA."""
        candidate = _make_candidate("RCV-DUP-001")
        auth = _make_authorization("RCV-DUP-001")
        snapshot = b"duplicate snapshot"

        entry1 = self.store.store(candidate, auth, snapshot)
        entry2 = self.store.store(candidate, auth, snapshot)
        self.assertEqual(entry1.commit_sha, entry2.commit_sha)


# ══════════════════════════════════════════════════════════════════════════════
# 2. RecoveryDiffPreview tests
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryDiffPreviewTests(unittest.TestCase):
    """Tests for :class:`RecoveryDiffPreview`."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.previewer = RecoveryDiffPreview()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_target_exists_shows_diff(self) -> None:
        """When target file exists, diff reflects the difference."""
        target = self.root / "existing.txt"
        target.write_text("old content\nline 2\n", encoding="utf-8")

        snapshot = b"new content\nline 2\nline 3\n"
        candidate = _make_candidate("RCV-DIFF-001")
        diff = self.previewer.preview(candidate, snapshot, str(target))

        self.assertTrue(diff.would_overwrite)
        self.assertIsNotNone(diff.current_sha256)
        self.assertIsNotNone(diff.current_bytes)
        self.assertGreater(diff.bytes_to_add, 0)
        self.assertEqual(diff.snapshot_lines, 4)  # trailing \n → 4 lines

    def test_target_does_not_exist(self) -> None:
        """When target doesn't exist, all bytes are 'to add'."""
        target = self.root / "nonexistent.txt"
        snapshot = b"brand new file\n"
        candidate = _make_candidate("RCV-DIFF-002")

        diff = self.previewer.preview(candidate, snapshot, str(target))

        self.assertFalse(diff.would_overwrite)
        self.assertIsNone(diff.current_sha256)
        self.assertEqual(diff.bytes_to_add, len(snapshot))
        self.assertEqual(diff.bytes_to_remove, 0)

    def test_identical_content_no_diff(self) -> None:
        """When target content matches snapshot, diff shows zero changes."""
        target = self.root / "identical.txt"
        content = b"identical content\n"
        target.write_bytes(content)

        candidate = _make_candidate("RCV-DIFF-003")
        diff = self.previewer.preview(candidate, content, str(target))

        self.assertTrue(diff.would_overwrite)
        self.assertEqual(diff.bytes_to_add, 0)
        self.assertEqual(diff.bytes_to_remove, 0)
        self.assertEqual(diff.snapshot_sha256, diff.current_sha256)

    def test_path_traversal_blocked(self) -> None:
        """Path traversal outside base_root is blocked."""
        snapshot = b"escape attempt\n"
        candidate = _make_candidate("RCV-DIFF-004")

        with self.assertRaises(Exception):
            self.previewer.preview(
                candidate, snapshot, "../../etc/passwd",
                base_root=self.root,
            )

    def test_large_diff_boundary(self) -> None:
        """Diff works correctly for larger files."""
        target = self.root / "large.txt"
        old = b"x" * 10000
        target.write_bytes(old)
        new = b"y" * 15000

        candidate = _make_candidate("RCV-DIFF-005")
        diff = self.previewer.preview(candidate, new, str(target))

        self.assertEqual(diff.bytes_to_add, 5000)
        self.assertEqual(diff.bytes_to_remove, 0)
        self.assertEqual(diff.current_bytes, 10000)
        self.assertEqual(diff.snapshot_bytes, 15000)


# ══════════════════════════════════════════════════════════════════════════════
# 3. RecoveryExecutor tests
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryExecutorTests(unittest.TestCase):
    """Tests for :class:`RecoveryExecutor`."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.key_store = MemoryInstallationKeyStore()
        self.execution_root = self.root / "executions"
        self.executor = RecoveryExecutor()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_valid_execution_produces_restored_receipt(self) -> None:
        """Valid candidate + authorization → restored outcome."""
        target = self.root / "restored.txt"
        snapshot = b"restored content\n"
        candidate = _make_candidate("RCV-EXEC-001", snapshot_sha=sha256_bytes(snapshot))
        auth = _make_authorization("RCV-EXEC-001", outcome="allow")

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            shadow_commit_sha="a" * 40,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertEqual(receipt.bytes_written, len(snapshot))
        self.assertIsNotNone(receipt.diff_preview)
        self.assertTrue(receipt.audit_events_preserved)
        self.assertEqual(receipt.shadow_commit_sha, "a" * 40)
        self.assertTrue(target.exists())
        self.assertEqual(target.read_bytes(), snapshot)

    def test_shadow_commit_sha_defaults_to_none(self) -> None:
        """Omitting shadow_commit_sha produces receipt with None."""
        target = self.root / "default_sha.txt"
        snapshot = b"default sha test\n"
        candidate = _make_candidate("RCV-DEFAULTSHA")
        auth = _make_authorization("RCV-DEFAULTSHA", outcome="allow")

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertIsNone(receipt.shadow_commit_sha)

    def test_execution_receipt_verification(self) -> None:
        """Execution receipt can be independently verified."""
        target = self.root / "verify_target.txt"
        snapshot = b"verifiable execution\n"
        candidate = _make_candidate("RCV-VEXEC-001")
        auth = _make_authorization("RCV-VEXEC-001", outcome="allow")

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            shadow_commit_sha="b" * 40,
            execution_root=self.execution_root,
        )

        # Read the persisted receipt
        exec_dirs = list(self.execution_root.iterdir())
        self.assertGreater(len(exec_dirs), 0)

        import json
        persisted = None
        for exec_dir in exec_dirs:
            receipt_path = exec_dir / "execution_receipt.json"
            if not receipt_path.exists():
                continue
            candidate_data = json.loads(receipt_path.read_text(encoding="utf-8"))
            if candidate_data.get("receipt_id") == receipt.receipt_id:
                persisted = candidate_data
                break
        self.assertIsNotNone(persisted, f"receipt not found: {receipt.receipt_id}")
        result = verify_execution_receipt(persisted, self.key_store)
        self.assertTrue(result["valid"], result["errors"])
        self.assertEqual(result["outcome"], "restored")

    def test_no_authorization_rejected(self) -> None:
        """Without authorization, execution is rejected."""
        target = self.root / "rejected.txt"
        snapshot = b"should not write\n"
        candidate = _make_candidate("RCV-REJECT-001")

        receipt = self.executor.execute(
            candidate, None, snapshot, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "rejected")
        self.assertFalse(target.exists())

    def test_authorization_with_deny_outcome_rejected(self) -> None:
        """Authorization with outcome='deny' is rejected."""
        target = self.root / "denied.txt"
        snapshot = b"denied write\n"
        candidate = _make_candidate("RCV-DENY-001")
        auth = _make_authorization("RCV-DENY-001", outcome="deny")

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "rejected")
        self.assertFalse(target.exists())

    def test_authorization_already_performed_rejected(self) -> None:
        """Authorization with restoration_performed=True is rejected (replay protection)."""
        target = self.root / "replay.txt"
        snapshot = b"replay attempt\n"
        candidate = _make_candidate("RCV-REPLAY-001")
        auth = _make_authorization(
            "RCV-REPLAY-001", outcome="allow", restoration_performed=True,
        )

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "rejected")
        self.assertFalse(target.exists())

    def test_overwrite_existing_preserves_pre_existing_sha(self) -> None:
        """Writing over an existing file records pre_existing_sha256."""
        target = self.root / "overwrite.txt"
        old_content = b"original content\n"
        target.write_bytes(old_content)

        new_content = b"new restored content\nmore data\n"
        candidate = _make_candidate("RCV-OVER-001")
        auth = _make_authorization("RCV-OVER-001", outcome="allow")

        receipt = self.executor.execute(
            candidate, auth, new_content, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertEqual(receipt.pre_existing_sha256, sha256_bytes(old_content))
        self.assertEqual(target.read_bytes(), new_content)

    def test_dry_run_does_not_write(self) -> None:
        """Dry run produces diff preview but does not write."""
        target = self.root / "dry_run.txt"
        target.write_text("existing\n", encoding="utf-8")

        snapshot = b"would replace\n"
        candidate = _make_candidate("RCV-DRY-001")
        auth = _make_authorization("RCV-DRY-001", outcome="allow")

        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            dry_run=True,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "rejected")
        self.assertEqual(receipt.bytes_written, 0)
        self.assertIsNotNone(receipt.diff_preview)
        self.assertEqual(target.read_text(encoding="utf-8"), "existing\n")


# ══════════════════════════════════════════════════════════════════════════════
# 4. End-to-end tests
# ══════════════════════════════════════════════════════════════════════════════


class EndToEndRecoveryTests(unittest.TestCase):
    """End-to-end tests: candidate → authorize → store → execute → verify."""

    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.key_store = MemoryInstallationKeyStore()
        self.store = ShadowRecoveryStore(repo_root=self.root)
        self.execution_root = self.root / "executions"
        self.executor = RecoveryExecutor()

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_full_store_execute_verify_cycle(self) -> None:
        """Store → retrieve → execute → verify completes successfully."""
        snapshot = b"end-to-end recovery checkpoint\nversion: 2\n"
        candidate = _make_candidate("RCV-E2E-001", snapshot_sha=sha256_bytes(snapshot))
        auth = _make_authorization("RCV-E2E-001", outcome="allow")

        # 1. Store in shadow store (Git-backed)
        entry = self.store.store(candidate, auth, snapshot)
        self.assertTrue(self.store.verify_entry(entry.commit_sha))

        # 2. Retrieve from shadow store
        retrieved = self.store.retrieve(entry.commit_sha)
        self.assertEqual(retrieved.snapshot_sha256, sha256_bytes(snapshot))

        # 3. Execute recovery with shadow_commit_sha
        target = self.root / "e2e_restored.bin"
        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            shadow_commit_sha=entry.commit_sha,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertEqual(receipt.shadow_commit_sha, entry.commit_sha)

        # 4. Verify file was restored correctly
        self.assertTrue(target.exists())
        self.assertEqual(target.read_bytes(), snapshot)

    def test_archived_source_recover_to_fresh_destination(self) -> None:
        """Simulate recovering from an archived source to a new destination."""
        snapshot = b"archived checkpoint data\nline 2\nline 3\n"

        # Build candidate pointing to an archived source
        candidate = _make_candidate(
            "RCV-ARCH-001",
            source_conv="archived-conversation-001",
            snapshot_sha=sha256_bytes(snapshot),
            snapshot_bytes_val=len(snapshot),
        )
        auth = _make_authorization("RCV-ARCH-001", outcome="allow")

        # Store
        entry = self.store.store(candidate, auth, snapshot)
        self.assertIsNotNone(entry.authorization_sha256)

        # Pre-flight diff
        target = self.root / "from_archive.txt"
        diff = RecoveryDiffPreview.preview(candidate, snapshot, str(target))
        self.assertFalse(diff.would_overwrite)

        # Execute
        receipt = self.executor.execute(
            candidate, auth, snapshot, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertEqual(target.read_bytes(), snapshot)

    def test_multi_entry_store_and_selective_restore(self) -> None:
        """Store multiple entries, restore only one."""
        suffix = self.root.name.replace("\\", "-").replace(":", "")
        id_a = f"RCV-MULTI-A-{suffix}"
        id_b = f"RCV-MULTI-B-{suffix}"
        id_c = f"RCV-MULTI-C-{suffix}"
        snap_a = f"checkpoint A {suffix}".encode("utf-8")
        snap_b = f"checkpoint B {suffix}".encode("utf-8")
        snap_c = f"checkpoint C {suffix}".encode("utf-8")

        before = len(self.store.list_entries())

        entry_a = self.store.store(
            _make_candidate(id_a), _make_authorization(id_a), snap_a,
        )
        entry_b = self.store.store(
            _make_candidate(id_b), _make_authorization(id_b), snap_b,
        )
        entry_c = self.store.store(
            _make_candidate(id_c), _make_authorization(id_c), snap_c,
        )

        entries = self.store.list_entries()
        self.assertEqual(len(entries), before + 3)
        self.assertTrue(entry_a.commit_sha)
        self.assertTrue(entry_c.commit_sha)

        # Restore only B
        target = self.root / "restore_b.txt"
        receipt = self.executor.execute(
            _make_candidate(id_b),
            _make_authorization(id_b),
            snap_b, str(target), self.key_store,
            execution_root=self.execution_root,
        )
        self.assertEqual(receipt.outcome, "restored")
        self.assertEqual(target.read_bytes(), snap_b)


if __name__ == "__main__":
    unittest.main()
