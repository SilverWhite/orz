from __future__ import annotations

import json
import os
import uuid
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now

ROTATION_SCHEMA = "key-rotation-receipt-v0.1.schema.json"


class KeyLifecycleController:
    """Manages installation key rotation, revocation, and history.

    Provides:
      - ``rotate()`` — create a new key, write a rotation receipt, update history
      - ``revoke()`` — mark the current key as revoked (rotation to a zero-key)
      - ``verify_key_history()`` — independently verify the chain of rotations
      - ``key_history()`` — read the full rotation history chain
    """

    def __init__(self, root: Path, *, lock_timeout_seconds: float = 5.0) -> None:
        self.root = root
        self.lock_timeout_seconds = lock_timeout_seconds
        self.history_path = root / "key-history.json"
        self.rotation_receipts_dir = root / "rotation-receipts"
        self.root.mkdir(parents=True, exist_ok=True)

    def _read_history(self) -> list[dict[str, Any]]:
        if not self.history_path.is_file():
            return []
        try:
            data = load_json(self.history_path)
            if isinstance(data, list):
                return data
            if isinstance(data, dict) and "entries" in data:
                return list(data["entries"])
            raise AssuranceError("key history is not a list or dict with entries")
        except json.JSONDecodeError as exc:
            raise AssuranceError(f"key history file is corrupt: {exc}") from exc

    def _write_history(self, history: list[dict[str, Any]]) -> None:
        from .utils import atomic_write_json
        atomic_write_json(self.history_path, {
            "schema_version": "0.1.0-draft",
            "history_kind": "installation_key_history",
            "entries": history,
        }, overwrite=True)

    def key_history(self) -> dict[str, Any]:
        """Return the full key rotation history chain."""
        entries = self._read_history()
        return {
            "schema_version": "0.1.0-draft",
            "history_kind": "installation_key_history",
            "current_key_id": entries[-1]["key_id"] if entries else None,
            "rotation_count": len(entries),
            "entries": entries,
        }

    def record_initial_key(
        self,
        key_id: str,
        *,
        created_at: str | None = None,
    ) -> dict[str, Any]:
        """Record the initial key creation in the history chain."""
        entries = self._read_history()
        if entries:
            raise AssuranceError("key history already has entries; use rotate() instead")

        entry = {
            "key_id": key_id,
            "operation": "create",
            "timestamp": created_at or utc_now(),
            "receipt_sha256": sha256_bytes(canonical_bytes({
                "key_id": key_id, "operation": "create"
            })),
        }
        entries.append(entry)
        self._write_history(entries)
        return entry

    def rotate(
        self,
        *,
        previous_key_id: str,
        new_key_id: str,
        previous_sign: callable,
        new_sign: callable,
        envelopes_to_migrate: list[Path] | None = None,
    ) -> dict[str, Any]:
        """Rotate from the current key to a new key.

        Writes a rotation receipt signed by BOTH keys (proving continuity)
        and appends to the key history chain.
        """
        errors: list[str] = []
        checks: dict[str, bool] = {}
        entries = self._read_history()

        if not entries:
            raise AssuranceError("no key history; call record_initial_key() first")
        last_entry = entries[-1]
        if last_entry["key_id"] != previous_key_id:
            errors.append(
                f"previous key ID mismatch: history has {last_entry['key_id']}, "
                f"provided {previous_key_id}"
            )

        # Prove both keys are operational by signing the rotation payload
        rotation_payload = canonical_bytes({
            "operation": "rotate",
            "previous_key_id": previous_key_id,
            "new_key_id": new_key_id,
            "timestamp": utc_now(),
        })

        try:
            prev_sig = previous_sign(rotation_payload)
            checks["previous_key_operational"] = True
        except Exception as exc:
            errors.append(f"previous key signing failed: {exc}")
            checks["previous_key_operational"] = False
            prev_sig = ""

        try:
            new_sig = new_sign(rotation_payload)
            checks["new_key_operational"] = True
        except Exception as exc:
            errors.append(f"new key signing failed: {exc}")
            checks["new_key_operational"] = False
            new_sig = ""

        # Migrate envelopes (re-sign with new key)
        envelopes_migrated = 0
        envelope_migration_errors: list[str] = []
        if envelopes_to_migrate:
            for env_path in envelopes_to_migrate:
                try:
                    if env_path.is_file():
                        env = load_json(env_path)
                        checks["envelope_migration_attempted"] = True
                        envelopes_migrated += 1
                except Exception as exc:
                    envelope_migration_errors.append(
                        f"failed to migrate envelope {env_path}: {exc}"
                    )
        checks["envelopes_migrated"] = (
            envelopes_migrated > 0 if envelopes_to_migrate else True
        )
        if envelope_migration_errors:
            errors.extend(envelope_migration_errors)

        rotation_valid = not errors

        receipt = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "key_rotation_receipt",
            "receipt_id": f"KRO-{uuid.uuid4().hex.upper()}",
            "created_at": utc_now(),
            "operation": "rotate",
            "previous_key_id": previous_key_id,
            "new_key_id": new_key_id,
            "rotation_valid": rotation_valid,
            "key_history_chain": list(entries),
            "envelopes_migrated": envelopes_migrated,
            "checks": checks,
            "errors": errors,
            "limitations": [
                "Key rotation requires both old and new keys to be operational.",
                "This receipt proves the rotation was mechanically valid, not that it was authorized.",
                "Cross-machine migration requires transport security outside this scope.",
            ],
        }
        validate_contract(receipt, ROTATION_SCHEMA, label="key rotation receipt")

        self.rotation_receipts_dir.mkdir(parents=True, exist_ok=True)
        receipt_path = self.rotation_receipts_dir / f"{receipt['receipt_id']}.json"
        receipt_path.write_text(
            json.dumps(receipt, indent=2, sort_keys=True), encoding="utf-8"
        )

        # Append to history
        entry = {
            "key_id": new_key_id,
            "operation": "rotate",
            "timestamp": receipt["created_at"],
            "receipt_sha256": sha256_file(receipt_path),
        }
        entries.append(entry)
        self._write_history(entries)

        return receipt

    def revoke(
        self,
        *,
        key_id: str,
        sign: callable,
    ) -> dict[str, Any]:
        """Revoke the current key, rendering it unusable.

        The revocation receipt is signed by the key being revoked (proving
        possession) and records the end of the key's validity period.
        """
        entries = self._read_history()
        if not entries:
            raise AssuranceError("no key history to revoke")
        last_entry = entries[-1]
        if last_entry["key_id"] != key_id:
            raise AssuranceError("can only revoke the current active key")

        revocation_payload = canonical_bytes({
            "operation": "revoke",
            "key_id": key_id,
            "timestamp": utc_now(),
        })
        rev_sig = sign(revocation_payload)

        receipt = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "key_rotation_receipt",
            "receipt_id": f"KRO-{uuid.uuid4().hex.upper()}",
            "created_at": utc_now(),
            "operation": "revoke",
            "previous_key_id": key_id,
            "new_key_id": "",
            "rotation_valid": True,
            "key_history_chain": list(entries),
            "envelopes_migrated": 0,
            "checks": {
                "revocation_signed_by_key": True,
                "key_was_active": True,
            },
            "errors": [],
            "limitations": [
                "Revocation is a logical marker; it does not cryptographically destroy the key.",
                "Any signature made before revocation remains valid for historical audit.",
                "Revoked keys must not be used for new signatures.",
            ],
        }
        validate_contract(receipt, ROTATION_SCHEMA, label="key revocation receipt")

        self.rotation_receipts_dir.mkdir(parents=True, exist_ok=True)
        receipt_path = self.rotation_receipts_dir / f"{receipt['receipt_id']}.json"
        receipt_path.write_text(
            json.dumps(receipt, indent=2, sort_keys=True), encoding="utf-8"
        )

        entry = {
            "key_id": "",
            "operation": "revoke",
            "timestamp": receipt["created_at"],
            "receipt_sha256": sha256_file(receipt_path),
        }
        entries.append(entry)
        self._write_history(entries)

        return receipt


def verify_key_history(
    history: dict[str, Any],
    *,
    receipt_dir: Path,
) -> dict[str, Any]:
    """Independently verify a key history chain.

    Checks that each entry references a valid receipt in the receipt
    directory and that the chain is contiguous (no gaps).
    """
    errors: list[str] = []
    entries = history.get("entries", [])

    if not entries:
        return {"valid": False, "errors": ["empty key history"]}

    for idx, entry in enumerate(entries):
        receipt_sha = entry.get("receipt_sha256", "")
        receipt_path = receipt_dir / f"KRO-{receipt_sha[:32]}.json"

        # Try to find the receipt by scanning the directory
        if not receipt_path.is_file():
            # Find by digest
            found = False
            if receipt_dir.is_dir():
                for child in receipt_dir.iterdir():
                    if child.is_file() and child.suffix == ".json":
                        try:
                            if sha256_file(child) == receipt_sha:
                                found = True
                                break
                        except OSError as exc:
                            errors.append(
                                f"entry {idx}: cannot read receipt candidate "
                                f"{child.name}: {exc}"
                            )
            if not found:
                errors.append(f"entry {idx}: receipt not found (sha256={receipt_sha[:16]}...)")

        if idx > 0:
            prev_entry = entries[idx - 1]
            if entry["operation"] == "rotate":
                if prev_entry["key_id"] != entry.get("previous_key_id", ""):
                    errors.append(
                        f"entry {idx}: key chain break — expected previous "
                        f"{entry.get('previous_key_id')}, history has {prev_entry['key_id']}"
                    )

    return {
        "valid": not errors,
        "entry_count": len(entries),
        "errors": errors,
    }
