"""Archive process recovery orchestrator.

Provides holistic recovery for interrupted archive operations — handling
torn journals, corrupt journals, stale locks, and partially-applied
deletions — without expanding the deletion boundary.

Follows the inspect → classify → recover pattern established by the
GSA runner journal recovery in ``assurance/runner.py``.

Distinct from ``assurance/recovery.py``, which handles snapshot-based
*conversation* recovery (restoring data from an archived conversation).
This module handles archive *process* recovery (repairing the archive
operation itself).
"""

from __future__ import annotations

from pathlib import Path
from typing import Any
import uuid

from .archive import ArchiveController
from .archive_journal import (
    inspect_archive_journal,
    recover_archive_journal,
    replay_archive_journal,
)
from .archive_verifier import verify_archive
from .conversation import ConversationNamespace
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .storage_adapter import StorageAdapter
from .utils import (
    atomic_write_json,
    canonical_bytes,
    sha256_bytes,
    sha256_file,
    utc_now,
)


# ── failure classification ──


def classify_archive_failure(
    namespace: ConversationNamespace,
    *,
    storage: StorageAdapter | None = None,
) -> dict[str, Any]:
    """Read-only classification of an interrupted archive's failure mode.

    Returns a dict with keys:
      - classification: str — failure mode identifier
      - archive_state: str — current conversation state
      - journal_classification: str — journal health
      - terminal_event: str | None — journal terminal event type
      - recoverable: bool — whether recovery can safely proceed
      - recommended_action: str — human-readable recommendation
      - safety_assessment: str — safety notes
    """
    state = namespace.state()
    archive_state: str = state["state"]

    journal_path = namespace.root / ".archive-journal.jsonl"
    inspection = inspect_archive_journal(journal_path)
    journal_classification: str = inspection["classification"]
    terminal_event: str | None = inspection.get("terminal_event")

    # Determine classification
    if archive_state == "active":
        classification = "active_no_archive"
        recoverable = False
        action = "No recovery needed — conversation has not started archiving."
        safety = "No archive data to protect."

    elif archive_state in ("archived", "failed"):
        classification = "already_terminal"
        recoverable = False
        action = (
            "Archive is already in terminal state. "
            "Run verify_archive() to validate integrity."
        )
        safety = "No deletion actions will be taken."

    elif archive_state == "archiving":
        if journal_classification == "empty":
            classification = "archiving_no_journal"
            recoverable = True
            action = (
                "State is archiving but no journal exists. "
                "Likely crashed before any deletions. Restart archive fresh."
            )
            safety = (
                "Deletion boundary is preserved — only delete_on_archive "
                "categories will be removed."
            )

        elif journal_classification == "clean":
            if terminal_event is None:
                classification = "clean_interrupted"
                recoverable = True
                action = (
                    "Clean journal with no terminal event — archive was "
                    "interrupted. Resume from journal to complete."
                )
                safety = (
                    "Already-deleted files will be skipped. "
                    "Only remaining delete_on_archive files will be removed."
                )
            else:
                classification = "clean_completed"
                recoverable = True
                action = (
                    f"Journal has terminal event '{terminal_event}'. "
                    "Archive may already be complete. Rebuild result from disk."
                )
                safety = "No additional deletions needed."

        elif journal_classification in ("torn_tail", "missing_newline"):
            classification = "torn_journal"
            recoverable = True
            action = (
                f"Journal has {journal_classification}. "
                "Repair journal by truncating to last valid newline, "
                "then resume archive."
            )
            safety = (
                "Torn tail bytes will be quarantined. "
                "Only valid prefix events are preserved."
            )

        elif journal_classification == "corrupt":
            classification = "corrupt_journal"
            recoverable = True
            action = (
                "Journal is corrupt and cannot be recovered. "
                "Quarantine corrupt journal, create fresh journal, "
                "and restart archive from scratch."
            )
            safety = (
                "Corrupt journal will be quarantined (not deleted). "
                "Fresh archive will re-scan and delete all remaining "
                "delete_on_archive files."
            )

        else:
            classification = "unknown"
            recoverable = False
            action = f"Unrecognized journal classification: {journal_classification}"
            safety = "Manual inspection required before any action."

    else:
        classification = "unknown_state"
        recoverable = False
        action = f"Unrecognized conversation state: {archive_state}"
        safety = "Manual inspection required."

    return {
        "classification": classification,
        "archive_state": archive_state,
        "journal_classification": journal_classification,
        "terminal_event": terminal_event,
        "recoverable": recoverable,
        "recommended_action": action,
        "safety_assessment": safety,
    }


# ── recovery orchestrator ──


def recover_archive(
    namespace: ConversationNamespace,
    *,
    key_store: InstallationKeyStore,
    storage: StorageAdapter | None = None,
    quarantine_dir: Path | None = None,
) -> dict[str, Any]:
    """Holistic archive process recovery.

    Inspects the conversation state and journal, classifies the failure
    mode, and executes the appropriate recovery path. Never expands the
    deletion boundary beyond the 16 ``delete_on_archive`` categories.

    Returns a recovery receipt dict with keys:
      - recovery_id: str
      - classification: str — failure mode that was addressed
      - action_taken: str — what was done
      - archive_result: dict | None — result from ArchiveController if archive was run
      - journal_recovery: dict | None — result from journal recovery if performed
      - verification: dict | None — result from verify_archive if applicable
      - valid: bool
      - errors: list[str]
    """
    classification = classify_archive_failure(namespace, storage=storage)
    errors: list[str] = []
    recovery_id = f"AREC-{uuid.uuid4().hex.upper()}"
    journal_recovery: dict[str, Any] | None = None
    archive_result: dict[str, Any] | None = None
    verification: dict[str, Any] | None = None
    action_taken: str

    journal_path = namespace.root / ".archive-journal.jsonl"

    if not classification["recoverable"]:
        return {
            "recovery_id": recovery_id,
            "classification": classification["classification"],
            "action_taken": "none — not recoverable",
            "archive_result": None,
            "journal_recovery": None,
            "verification": None,
            "valid": False,
            "errors": [classification["recommended_action"]],
        }

    failure_mode = classification["classification"]

    # ── active_no_archive / already_terminal ──
    if failure_mode == "active_no_archive":
        action_taken = "no action — conversation is active"
        verification = verify_archive(namespace, key_store=key_store)

    elif failure_mode == "already_terminal":
        action_taken = "no action — archive already in terminal state"
        verification = verify_archive(namespace, key_store=key_store)

    # ── clean_completed ──
    elif failure_mode == "clean_completed":
        action_taken = "rebuild result from existing journal"
        # Journal has terminal event — rebuild from disk
        controller = ArchiveController(
            key_store=key_store,
            storage=storage,
        )
        archive_result = controller.archive(namespace)
        verification = verify_archive(namespace, key_store=key_store)

    # ── clean_interrupted ──
    elif failure_mode == "clean_interrupted":
        action_taken = "resume archive from clean interrupted journal"
        controller = ArchiveController(
            key_store=key_store,
            storage=storage,
        )
        archive_result = controller.archive(namespace)
        verification = verify_archive(namespace, key_store=key_store)

    # ── archiving_no_journal ──
    elif failure_mode == "archiving_no_journal":
        action_taken = "restart archive — state is archiving but no journal exists"
        controller = ArchiveController(
            key_store=key_store,
            storage=storage,
        )
        archive_result = controller.archive(namespace)
        verification = verify_archive(namespace, key_store=key_store)

    # ── torn_journal ──
    elif failure_mode == "torn_journal":
        action_taken = "repair torn journal then resume archive"
        receipt_path = (
            namespace.receipts_root
            / f"journal-recovery-{recovery_id}.json"
        )
        quarantine_path = (
            quarantine_dir / f"torn-tail-{recovery_id}.bin"
            if quarantine_dir
            else namespace.root / f".torn-tail-quarantine-{recovery_id}.bin"
        )
        journal_recovery = recover_archive_journal(
            journal_path,
            receipt_path=receipt_path,
            quarantine_path=quarantine_path,
            lock_timeout_seconds=5.0,
        )
        if not journal_recovery["valid"]:
            errors.append(
                f"journal recovery failed: {journal_recovery.get('errors', [])}"
            )
            action_taken = "journal recovery failed — archive not resumed"
        else:
            controller = ArchiveController(
                key_store=key_store,
                storage=storage,
            )
            try:
                archive_result = controller.archive(namespace)
            except AssuranceError as exc:
                errors.append(f"archive resume after journal repair failed: {exc}")
            verification = (
                verify_archive(namespace, key_store=key_store)
                if archive_result
                else None
            )

    # ── corrupt_journal ──
    elif failure_mode == "corrupt_journal":
        action_taken = "quarantine corrupt journal and restart archive"
        # Quarantine the corrupt journal
        corrupt_backup = (
            quarantine_dir / f"corrupt-journal-{recovery_id}.jsonl"
            if quarantine_dir
            else namespace.root / f".corrupt-journal-quarantine-{recovery_id}.jsonl"
        )
        if quarantine_dir:
            quarantine_dir.mkdir(parents=True, exist_ok=True)
        try:
            corrupt_content = journal_path.read_bytes()
            corrupt_backup.write_bytes(corrupt_content)
            journal_path.unlink()
        except OSError as exc:
            errors.append(f"could not quarantine corrupt journal: {exc}")
            action_taken = "failed to quarantine corrupt journal"

        if not errors:
            controller = ArchiveController(
                key_store=key_store,
                storage=storage,
            )
            try:
                archive_result = controller.archive(namespace)
            except AssuranceError as exc:
                errors.append(f"archive restart after corrupt journal failed: {exc}")
            verification = (
                verify_archive(namespace, key_store=key_store)
                if archive_result
                else None
            )

    else:
        action_taken = f"unhandled failure mode: {failure_mode}"
        errors.append(action_taken)

    valid = not errors and (
        verification["valid"] if verification else archive_result is not None
    )

    return {
        "recovery_id": recovery_id,
        "classification": failure_mode,
        "action_taken": action_taken,
        "archive_result": archive_result,
        "journal_recovery": journal_recovery,
        "verification": verification,
        "valid": valid,
        "errors": errors,
    }
