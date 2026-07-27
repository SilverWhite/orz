from __future__ import annotations

import os
from pathlib import Path
import time
from typing import Any, Callable
import uuid

from .contracts import ASSURANCE_ROOT, validate_contract
from .conversation import ConversationNamespace, retention_delete_categories
from .envelope import verify_security_envelope
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_json,
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    require_within,
    sha256_bytes,
    sha256_file,
    utc_now,
)


DeleteFile = Callable[[Path], None]


def _default_delete_file(path: Path) -> None:
    path.unlink()


def _scan_category(
    category_root: Path, conversation_root: Path
) -> tuple[list[Path], int]:
    if not category_root.exists():
        return [], 0
    require_within(category_root, conversation_root, must_exist=True)
    if is_link_or_reparse(category_root):
        return [], 1
    files: list[Path] = []
    unsafe_entries = 0
    for current_root, dirs, names in os.walk(category_root, followlinks=False):
        base = Path(current_root)
        safe_dirs: list[str] = []
        for name in sorted(dirs):
            candidate = base / name
            if is_link_or_reparse(candidate):
                unsafe_entries += 1
            else:
                safe_dirs.append(name)
        dirs[:] = safe_dirs
        for name in sorted(names):
            candidate = base / name
            require_within(candidate, conversation_root, must_exist=True)
            if is_link_or_reparse(candidate) or not candidate.is_file():
                unsafe_entries += 1
            else:
                files.append(candidate)
    return sorted(files), unsafe_entries


def _remove_empty_directories(
    category_root: Path, conversation_root: Path
) -> list[str]:
    """Remove empty directories under *category_root*, then the root itself.

    Returns a (possibly empty) list of diagnostic strings for any directory
    that could not be removed, so callers can surface the failures rather
    than silently discarding them.
    """
    diagnostics: list[str] = []
    if not category_root.exists() or is_link_or_reparse(category_root):
        return diagnostics
    directories: list[Path] = []
    for current_root, dirs, _ in os.walk(category_root, followlinks=False):
        base = Path(current_root)
        for name in dirs:
            candidate = base / name
            if not is_link_or_reparse(candidate):
                directories.append(candidate)
    for directory in sorted(directories, key=lambda item: len(item.parts), reverse=True):
        require_within(directory, conversation_root, must_exist=True)
        try:
            directory.rmdir()
        except OSError as exc:
            diagnostics.append(
                f"could not remove directory {directory}: {exc}"
            )
    try:
        category_root.rmdir()
    except OSError as exc:
        diagnostics.append(
            f"could not remove category root {category_root}: {exc}"
        )
    return diagnostics


def _signed_receipt(
    body: dict[str, Any], *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "key_id": key_store.key_id,
            "algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _cleanup_projection(
    category_results: list[dict[str, Any]],
) -> dict[str, Any]:
    remaining = {
        item["category"]: item["remaining_count"] for item in category_results
    }
    raw_categories = {
        "raw_provider_payload",
        "private_reasoning",
        "raw_tool_result",
        "full_stdout_stderr",
        "network_body",
    }
    temporary_categories = {
        "confirmation_token",
        "one_shot_permit",
        "temporary_checkpoint",
        "temporary_profile",
        "sandbox_ephemeral_storage",
        "session_recall_index",
        "active_security_envelope",
        "temporary_lifecycle_receipt",
        "temporary_import_receipt",
    }
    return {
        "raw_payloads_remaining": sum(remaining.get(item, 0) for item in raw_categories),
        "credential_leases_remaining": remaining.get("credential_lease", 0),
        "temporary_storage_remaining": sum(
            remaining.get(item, 0) for item in temporary_categories
        ),
        "unpinned_snapshots_remaining": remaining.get("unpinned_snapshot", 0),
    }


class ArchiveController:
    """Explicit-category archive deletion controller with crash recovery.

    Supports:
      - Retry with exponential backoff for transient deletion failures
      - Append-only operation journal for crash recovery
      - Advisory lock for concurrency control
      - Pluggable storage adapter
      - Resume from interrupted ``archiving`` state
    """

    def __init__(
        self,
        *,
        key_store: InstallationKeyStore,
        delete_file: DeleteFile | None = None,
        max_retries: int = 3,
        retry_base_delay_seconds: float = 0.1,
        lock_timeout_seconds: float = 30.0,
    ) -> None:
        self.key_store = key_store
        self.max_retries = max_retries
        self.retry_base_delay_seconds = retry_base_delay_seconds
        self.lock_timeout_seconds = lock_timeout_seconds
        self._delete_file = delete_file or _default_delete_file

    def _journal_path(self, namespace: ConversationNamespace) -> Path:
        return namespace.root / ".archive-journal.jsonl"

    def _delete_with_retry(self, path: Path) -> tuple[bool, int, str | None]:
        """Attempt deletion with exponential backoff.

        Returns (success, retry_count, error_message).
        """
        for attempt in range(1, self.max_retries + 2):  # +2 because range is [1, N+1]
            try:
                self._delete_file(path)
            except OSError as exc:
                if attempt <= self.max_retries:
                    delay = self.retry_base_delay_seconds * (2 ** (attempt - 1))
                    time.sleep(delay)
                    continue
                return False, attempt - 1, str(exc)
            if path.exists():
                if attempt <= self.max_retries:
                    delay = self.retry_base_delay_seconds * (2 ** (attempt - 1))
                    time.sleep(delay)
                    continue
                return False, attempt - 1, "file still exists after deletion"
            return True, attempt - 1, None
        return False, self.max_retries, "max retries exceeded"

    def archive(self, namespace: ConversationNamespace) -> dict[str, Any]:
        state = namespace.state()
        current_state: str = state["state"]

        if current_state not in {"active", "archiving"}:
            raise AssuranceError(
                f"archive requires an active or archiving conversation, got {current_state}"
            )

        is_resume = current_state == "archiving"
        archive_id = f"ARC-{uuid.uuid4().hex.upper()}"
        journal_path = self._journal_path(namespace)

        from .archive_journal import (
            ArchiveJournalWriter,
            exclusive_archive_lock,
            replay_archive_journal,
        )

        with exclusive_archive_lock(
            journal_path, timeout_seconds=self.lock_timeout_seconds
        ):
            already_deleted: set[tuple[str, str]] = set()
            next_sequence = 0
            prev_hash: str | None = None

            if is_resume:
                replay = replay_archive_journal(journal_path)
                if not replay["valid"]:
                    raise AssuranceError(
                        "cannot resume: archive journal is invalid — "
                        "run archive inspection tool first"
                    )
                if replay["terminal_event"] is not None:
                    # Archive was already completed — journal has terminal event.
                    # Reconstruct the result from what's on disk.
                    return self._rebuild_result_from_disk(namespace, state)
                already_deleted = replay["deleted_files"]
                next_sequence = replay["last_sequence"]
                prev_hash = replay["last_event_sha256"]
                # Use the original archive_id from the journal if available
                if replay["events"]:
                    archive_id = replay["events"][0]["archive_id"]
            else:
                # Fresh archive — validate envelope and transition state
                envelope = namespace.load_active_envelope()
                verify_security_envelope(envelope, key_store=self.key_store)

                start_receipt = {
                    "schema_version": "0.1.0-draft",
                    "receipt_kind": "session_lifecycle_receipt",
                    "transition_id": f"LIFE-{uuid.uuid4().hex.upper()}",
                    "conversation_id": namespace.conversation_id,
                    "envelope_id": state["envelope_id"],
                    "created_at": utc_now(),
                    "from_state": "active",
                    "to_state": "archiving",
                    "terminal_state": "in_progress",
                    "resume_requires_new_envelope": True,
                    "cleanup": None,
                }
                validate_contract(
                    start_receipt,
                    "session-lifecycle-receipt-v0.1.schema.json",
                    label="archive start lifecycle receipt",
                )
                atomic_write_json(
                    namespace.artifacts_root
                    / "temporary_lifecycle_receipt"
                    / "lifecycle-start.json",
                    start_receipt,
                )
                archiving_state = {
                    **state,
                    "state": "archiving",
                    "revision": state["revision"] + 1,
                    "updated_at": utc_now(),
                    "terminal_deletion_receipt_sha256": None,
                }
                namespace._write_state(archiving_state, overwrite=True)
                state = archiving_state

            # Open journal writer
            with ArchiveJournalWriter(
                journal_path,
                archive_id=archive_id,
                conversation_id=namespace.conversation_id,
                last_sequence=next_sequence,
                last_event_sha256=prev_hash,
            ) as writer:
                if not is_resume:
                    writer.append_event(
                        "archive_started",
                        {
                            "envelope_id": state["envelope_id"],
                            "policy_sha256": sha256_file(
                                ASSURANCE_ROOT / "retention-policy-v0.1.json"
                            ),
                        },
                    )

                error_pairs: set[tuple[str, str]] = set()
                category_results: list[dict[str, Any]] = []

                for category in retention_delete_categories():
                    writer.append_event("category_started", {"category": category})
                    category_root = namespace.artifacts_root / category
                    files, unsafe_entries = _scan_category(
                        category_root, namespace.root
                    )
                    discovered_count = len(files) + unsafe_entries
                    deleted_records: list[dict[str, Any]] = []
                    category_failed = 0

                    if unsafe_entries:
                        error_pairs.add((category, "unsafe_entry"))

                    for path in files:
                        rel = path.relative_to(namespace.root).as_posix()
                        if (category, rel) in already_deleted:
                            # File was already deleted in a previous run
                            record = {
                                "size_bytes": 0,  # cannot stat deleted file
                                "sha256": "",
                            }
                            deleted_records.append(record)
                            continue

                        size_before = path.stat().st_size
                        sha_before = sha256_file(path)
                        success, retries, err_msg = self._delete_with_retry(path)
                        if success:
                            writer.append_event(
                                "file_deleted",
                                {
                                    "category": category,
                                    "relative_path": rel,
                                    "size_bytes": size_before,
                                    "sha256": sha_before,
                                    "attempt": retries + 1,
                                },
                            )
                            deleted_records.append(
                                {"size_bytes": size_before, "sha256": sha_before}
                            )
                        else:
                            writer.append_event(
                                "file_failed",
                                {
                                    "category": category,
                                    "relative_path": rel,
                                    "error": err_msg or "deletion failed",
                                    "attempt": retries + 1,
                                },
                            )
                            error_pairs.add((category, "delete_failed"))
                            category_failed += 1

                    cleanup_diags = _remove_empty_directories(category_root, namespace.root)
                    if cleanup_diags:
                        for diag in cleanup_diags:
                            writer.append_event(
                                "file_deleted",
                                {
                                    "category": category,
                                    "relative_path": "",
                                    "error": diag,
                                    "attempt": 0,
                                },
                            )
                    remaining_files, remaining_unsafe = _scan_category(
                        category_root, namespace.root
                    )
                    remaining_count = len(remaining_files) + remaining_unsafe
                    if category_root.exists() and remaining_count == 0:
                        error_pairs.add((category, "delete_failed"))
                    deleted_count = len(deleted_records)
                    if discovered_count != deleted_count + remaining_count:
                        error_pairs.add((category, "count_mismatch"))
                    writer.append_event(
                        "category_completed",
                        {
                            "category": category,
                            "deleted_count": deleted_count,
                            "failed_count": category_failed,
                            "remaining_count": remaining_count,
                        },
                    )
                    category_results.append(
                        {
                            "category": category,
                            "discovered_count": discovered_count,
                            "deleted_count": deleted_count,
                            "remaining_count": remaining_count,
                            "deleted_content_aggregate_sha256": sha256_bytes(
                                canonical_bytes(
                                    sorted(
                                        [
                                            r
                                            for r in deleted_records
                                            if r["sha256"]
                                        ],
                                        key=lambda item: (
                                            item["sha256"],
                                            item["size_bytes"],
                                        ),
                                    )
                                )
                            ),
                        }
                    )

            # Journal writer is closed — compute final outcome
            totals = {
                "discovered": sum(
                    item["discovered_count"] for item in category_results
                ),
                "deleted": sum(item["deleted_count"] for item in category_results),
                "remaining": sum(
                    item["remaining_count"] for item in category_results
                ),
            }
            archive_complete = totals["remaining"] == 0 and not error_pairs
            outcome = "archived" if archive_complete else "failed"
            errors = [
                {"category": category, "code": code}
                for category, code in sorted(error_pairs)
            ]
            if not archive_complete and not errors:
                errors = [{"category": "unknown", "code": "count_mismatch"}]

            # Re-open journal to write terminal event
            with ArchiveJournalWriter(
                journal_path,
                archive_id=archive_id,
                conversation_id=namespace.conversation_id,
                last_sequence=writer.next_sequence,
                last_event_sha256=writer.last_event_sha256,
            ) as writer2:
                if archive_complete:
                    writer2.append_event(
                        "archive_completed",
                        {"outcome": "archived", "totals": totals},
                    )
                else:
                    writer2.append_event(
                        "archive_failed",
                        {
                            "outcome": "failed",
                            "errors": [
                                {"category": e["category"], "code": e["code"]}
                                for e in errors
                            ],
                        },
                    )

            policy_sha256 = sha256_file(
                ASSURANCE_ROOT / "retention-policy-v0.1.json"
            )
            body = {
                "schema_version": "0.1.0-draft",
                "receipt_kind": "archive_deletion_receipt",
                "receipt_id": f"DEL-{uuid.uuid4().hex.upper()}",
                "conversation_id": namespace.conversation_id,
                "envelope_id": state["envelope_id"],
                "created_at": utc_now(),
                "outcome": outcome,
                "archive_complete": archive_complete,
                "policy_sha256": policy_sha256,
                "categories": category_results,
                "totals": totals,
                "errors": errors,
            }
            deletion_receipt = _signed_receipt(body, key_store=self.key_store)
            validate_contract(
                deletion_receipt,
                "archive-deletion-receipt-v0.1.schema.json",
                label="archive deletion receipt",
            )
            deletion_path = namespace.receipts_root / "archive-deletion.json"
            atomic_write_json(deletion_path, deletion_receipt)

            cleanup = _cleanup_projection(category_results)
            unresolved = sorted(
                {
                    item["category"]
                    for item in category_results
                    if item["remaining_count"] > 0
                }
                | {item["category"] for item in errors}
            )
            terminal_receipt = {
                "schema_version": "0.1.0-draft",
                "receipt_kind": "session_lifecycle_receipt",
                "transition_id": f"LIFE-{uuid.uuid4().hex.upper()}",
                "conversation_id": namespace.conversation_id,
                "envelope_id": state["envelope_id"],
                "created_at": utc_now(),
                "from_state": "archiving",
                "to_state": outcome,
                "terminal_state": (
                    "archive_complete" if archive_complete else "archive_failed"
                ),
                "resume_requires_new_envelope": True,
                "cleanup": {
                    **cleanup,
                    "deletion_receipt_digest": deletion_receipt["integrity"][
                        "signed_payload_sha256"
                    ],
                    "unresolved_categories": unresolved,
                },
            }
            validate_contract(
                terminal_receipt,
                "session-lifecycle-receipt-v0.1.schema.json",
                label="archive terminal lifecycle receipt",
            )
            terminal_path = namespace.receipts_root / "lifecycle-terminal.json"
            atomic_write_json(terminal_path, terminal_receipt)

            terminal_state = {
                **state,
                "state": outcome,
                "revision": state["revision"] + 1,
                "updated_at": utc_now(),
                "terminal_deletion_receipt_sha256": sha256_file(deletion_path),
            }
            namespace._write_state(terminal_state, overwrite=True)
            return {
                "archive_complete": archive_complete,
                "outcome": outcome,
                "conversation_id": namespace.conversation_id,
                "envelope_id": state["envelope_id"],
                "deletion_receipt_path": str(deletion_path),
                "terminal_lifecycle_path": str(terminal_path),
                "totals": totals,
                "errors": errors,
            }

    def _rebuild_result_from_disk(
        self, namespace: ConversationNamespace, state: dict[str, Any]
    ) -> dict[str, Any]:
        """Reconstruct archive result when journal indicates prior completion."""
        deletion_path = namespace.receipts_root / "archive-deletion.json"
        terminal_path = namespace.receipts_root / "lifecycle-terminal.json"
        if not deletion_path.is_file() or not terminal_path.is_file():
            raise AssuranceError(
                "archive journal has terminal event but receipts are missing "
                "— filesystem may have been modified externally"
            )
        deletion_receipt = load_json(deletion_path)
        return {
            "archive_complete": deletion_receipt.get("archive_complete", False),
            "outcome": deletion_receipt.get("outcome", "failed"),
            "conversation_id": namespace.conversation_id,
            "envelope_id": state["envelope_id"],
            "deletion_receipt_path": str(deletion_path),
            "terminal_lifecycle_path": str(terminal_path),
            "totals": deletion_receipt.get("totals", {}),
            "errors": deletion_receipt.get("errors", []),
        }


def resume_archived_conversation(
    previous: ConversationNamespace,
    repository_root: Path,
    *,
    key_store: InstallationKeyStore,
    frozen_context: dict[str, Any],
    allowed_capabilities: list[str],
    denied_capabilities: list[str],
) -> ConversationNamespace:
    from .archive_verifier import verify_archive

    verification = verify_archive(previous, key_store=key_store)
    if not verification["valid"] or not verification["archive_complete"]:
        raise AssuranceError("only a verified complete archive can be resumed")
    deletion_receipt = load_json(
        previous.receipts_root / "archive-deletion.json"
    )
    return ConversationNamespace.create(
        repository_root,
        key_store=key_store,
        frozen_context=frozen_context,
        allowed_capabilities=allowed_capabilities,
        denied_capabilities=denied_capabilities,
        parent_envelope_id=deletion_receipt["envelope_id"],
        resumed_from_conversation_id=previous.conversation_id,
    )
