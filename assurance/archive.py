from __future__ import annotations

import os
from pathlib import Path
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


def _remove_empty_directories(category_root: Path, conversation_root: Path) -> None:
    if not category_root.exists() or is_link_or_reparse(category_root):
        return
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
        except OSError:
            pass
    try:
        category_root.rmdir()
    except OSError:
        pass


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
    """Explicit-category archive deletion controller for disposable P1 fixtures."""

    def __init__(
        self,
        *,
        key_store: InstallationKeyStore,
        delete_file: DeleteFile | None = None,
    ) -> None:
        self.key_store = key_store
        self._delete_file = delete_file or _default_delete_file

    def archive(self, namespace: ConversationNamespace) -> dict[str, Any]:
        state = namespace.state()
        if state["state"] != "active":
            raise AssuranceError("archive requires an active conversation")
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

        error_pairs: set[tuple[str, str]] = set()
        category_results: list[dict[str, Any]] = []
        for category in retention_delete_categories():
            category_root = namespace.artifacts_root / category
            files, unsafe_entries = _scan_category(category_root, namespace.root)
            discovered_count = len(files) + unsafe_entries
            deleted_records: list[dict[str, Any]] = []
            if unsafe_entries:
                error_pairs.add((category, "unsafe_entry"))
            for path in files:
                record = {
                    "size_bytes": path.stat().st_size,
                    "sha256": sha256_file(path),
                }
                try:
                    self._delete_file(path)
                except OSError:
                    error_pairs.add((category, "delete_failed"))
                    continue
                if path.exists():
                    error_pairs.add((category, "delete_failed"))
                    continue
                deleted_records.append(record)
            _remove_empty_directories(category_root, namespace.root)
            remaining_files, remaining_unsafe = _scan_category(
                category_root, namespace.root
            )
            remaining_count = len(remaining_files) + remaining_unsafe
            if category_root.exists() and remaining_count == 0:
                error_pairs.add((category, "delete_failed"))
            deleted_count = len(deleted_records)
            if discovered_count != deleted_count + remaining_count:
                error_pairs.add((category, "count_mismatch"))
            category_results.append(
                {
                    "category": category,
                    "discovered_count": discovered_count,
                    "deleted_count": deleted_count,
                    "remaining_count": remaining_count,
                    "deleted_content_aggregate_sha256": sha256_bytes(
                        canonical_bytes(
                            sorted(
                                deleted_records,
                                key=lambda item: (
                                    item["sha256"],
                                    item["size_bytes"],
                                ),
                            )
                        )
                    ),
                }
            )

        totals = {
            "discovered": sum(item["discovered_count"] for item in category_results),
            "deleted": sum(item["deleted_count"] for item in category_results),
            "remaining": sum(item["remaining_count"] for item in category_results),
        }
        archive_complete = totals["remaining"] == 0 and not error_pairs
        outcome = "archived" if archive_complete else "failed"
        errors = [
            {"category": category, "code": code}
            for category, code in sorted(error_pairs)
        ]
        if not archive_complete and not errors:
            errors = [{"category": "unknown", "code": "count_mismatch"}]
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
            **archiving_state,
            "state": outcome,
            "revision": archiving_state["revision"] + 1,
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
