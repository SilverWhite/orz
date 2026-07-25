from __future__ import annotations

from copy import deepcopy
import os
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .conversation import ConversationNamespace, retention_delete_categories
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    canonical_bytes,
    is_link_or_reparse,
    load_json,
    require_within,
    sha256_bytes,
    sha256_file,
)


def _independent_remaining_count(
    category_root: Path, conversation_root: Path
) -> int:
    if not category_root.exists():
        return 0
    require_within(category_root, conversation_root, must_exist=True)
    if is_link_or_reparse(category_root):
        return 1
    count = 0
    for current_root, directories, filenames in os.walk(
        category_root, followlinks=False
    ):
        base = Path(current_root)
        kept: list[str] = []
        for name in directories:
            candidate = base / name
            if is_link_or_reparse(candidate):
                count += 1
            else:
                kept.append(name)
        directories[:] = kept
        for name in filenames:
            candidate = base / name
            require_within(candidate, conversation_root, must_exist=True)
            count += 1
    return count


def _independent_cleanup_projection(
    categories: list[dict[str, Any]],
) -> dict[str, int]:
    remaining = {
        item["category"]: item["remaining_count"] for item in categories
    }
    return {
        "raw_payloads_remaining": sum(
            remaining.get(item, 0)
            for item in (
                "raw_provider_payload",
                "private_reasoning",
                "raw_tool_result",
                "full_stdout_stderr",
                "network_body",
            )
        ),
        "credential_leases_remaining": remaining.get("credential_lease", 0),
        "temporary_storage_remaining": sum(
            remaining.get(item, 0)
            for item in (
                "confirmation_token",
                "one_shot_permit",
                "temporary_checkpoint",
                "temporary_profile",
                "sandbox_ephemeral_storage",
                "session_recall_index",
                "active_security_envelope",
                "temporary_lifecycle_receipt",
                "temporary_import_receipt",
            )
        ),
        "unpinned_snapshots_remaining": remaining.get("unpinned_snapshot", 0),
    }


def _verify_signature(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore
) -> str:
    integrity = receipt["integrity"]
    if integrity["key_id"] != key_store.key_id:
        raise AssuranceError("archive receipt installation key ID mismatch")
    body = {key: deepcopy(value) for key, value in receipt.items() if key != "integrity"}
    payload = canonical_bytes(body)
    digest = sha256_bytes(payload)
    if digest != integrity["signed_payload_sha256"]:
        raise AssuranceError("archive receipt signed-payload digest mismatch")
    if not key_store.verify(payload, integrity["signature"]):
        raise AssuranceError("archive receipt signature verification failed")
    return digest


def verify_archive(
    namespace: ConversationNamespace, *, key_store: InstallationKeyStore
) -> dict[str, Any]:
    """Rebuild a terminal archive verdict without invoking the deletion controller."""

    errors: list[str] = []
    try:
        state = namespace.state()
        if state["state"] not in {"archived", "failed"}:
            raise AssuranceError("conversation is not in a terminal archive state")
        deletion_path = namespace.receipts_root / "archive-deletion.json"
        terminal_path = namespace.receipts_root / "lifecycle-terminal.json"
        if sha256_file(deletion_path) != state["terminal_deletion_receipt_sha256"]:
            raise AssuranceError("terminal deletion receipt file digest mismatch")
        deletion_receipt = load_json(deletion_path)
        terminal_receipt = load_json(terminal_path)
        validate_contract(
            deletion_receipt,
            "archive-deletion-receipt-v0.1.schema.json",
            label="archive deletion receipt",
        )
        validate_contract(
            terminal_receipt,
            "session-lifecycle-receipt-v0.1.schema.json",
            label="archive terminal lifecycle receipt",
        )
        signed_payload_digest = _verify_signature(
            deletion_receipt, key_store=key_store
        )
        if (
            terminal_receipt["cleanup"]["deletion_receipt_digest"]
            != signed_payload_digest
        ):
            errors.append("terminal lifecycle deletion-receipt digest mismatch")
        if deletion_receipt["conversation_id"] != namespace.conversation_id:
            errors.append("deletion receipt conversation ID mismatch")
        if deletion_receipt["envelope_id"] != state["envelope_id"]:
            errors.append("deletion receipt envelope ID mismatch")
        expected_state = (
            "archived" if deletion_receipt["archive_complete"] else "failed"
        )
        if state["state"] != expected_state:
            errors.append("conversation state disagrees with deletion receipt")
        if terminal_receipt["to_state"] != expected_state:
            errors.append("terminal lifecycle disagrees with deletion receipt")

        categories = deletion_receipt["categories"]
        expected_categories = list(retention_delete_categories())
        if [item["category"] for item in categories] != expected_categories:
            errors.append("deletion receipt category order or coverage mismatch")
        rebuilt_totals = {
            "discovered": sum(item["discovered_count"] for item in categories),
            "deleted": sum(item["deleted_count"] for item in categories),
            "remaining": sum(item["remaining_count"] for item in categories),
        }
        if rebuilt_totals != deletion_receipt["totals"]:
            errors.append("deletion receipt totals do not rebuild")
        for item in categories:
            if (
                item["discovered_count"]
                != item["deleted_count"] + item["remaining_count"]
            ):
                errors.append(f"category count mismatch: {item['category']}")
            actual_remaining = _independent_remaining_count(
                namespace.artifacts_root / item["category"], namespace.root
            )
            if actual_remaining != item["remaining_count"]:
                errors.append(
                    f"remaining artifact count mismatch: {item['category']}"
                )

        rebuilt_cleanup = _independent_cleanup_projection(categories)
        for key, value in rebuilt_cleanup.items():
            if terminal_receipt["cleanup"][key] != value:
                errors.append(f"terminal cleanup projection mismatch: {key}")
        unresolved = sorted(
            {
                item["category"]
                for item in categories
                if item["remaining_count"] > 0
            }
            | {
                item["category"]
                for item in deletion_receipt["errors"]
            }
        )
        if terminal_receipt["cleanup"]["unresolved_categories"] != unresolved:
            errors.append("terminal unresolved-category projection mismatch")
        if deletion_receipt["archive_complete"]:
            if deletion_receipt["errors"]:
                errors.append("successful archive contains deletion errors")
            if deletion_receipt["totals"]["remaining"] != 0:
                errors.append("successful archive retains temporary artifacts")
        elif not unresolved:
            errors.append("failed archive lacks unresolved categories")
    except (AssuranceError, OSError, KeyError, TypeError, ValueError) as exc:
        errors.append(str(exc))
        state = {}
        deletion_receipt = {"archive_complete": False, "outcome": "unknown"}
    return {
        "valid": not errors,
        "archive_complete": bool(deletion_receipt.get("archive_complete")),
        "outcome": deletion_receipt.get("outcome"),
        "conversation_id": namespace.conversation_id,
        "state": state.get("state"),
        "errors": errors,
        "limitations": [
            "Verification covers the declared conversation namespace only.",
            "It cannot prove deletion from pagefile, backups, provider logs, or storage remanence."
        ],
    }
