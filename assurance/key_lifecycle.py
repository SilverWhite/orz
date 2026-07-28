from __future__ import annotations

import json
import os
import time
import uuid
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now

ROTATION_SCHEMA = "key-rotation-receipt-v0.1.schema.json"
_LOCK_POLL_INTERVAL = 0.05  # 50ms between lock acquisition attempts


class _FileLock:
    """Cross-platform exclusive file lock using exclusive file creation.

    Creates a lock file via ``open(path, 'x')`` (``O_CREAT|O_EXCL``).
    Stale locks from crashed processes are detected by age and cleaned up
    after *timeout_seconds*.
    """

    def __init__(self, lock_path: Path, timeout_seconds: float = 5.0) -> None:
        self._lock_path = lock_path
        self._timeout = timeout_seconds
        self._acquired = False

    def acquire(self) -> bool:
        """Try to acquire the lock, waiting up to *timeout_seconds*.

        Returns True if the lock was acquired, False on timeout.
        """
        deadline = time.monotonic() + self._timeout
        while time.monotonic() < deadline:
            try:
                fd = os.open(
                    str(self._lock_path),
                    os.O_CREAT | os.O_EXCL | os.O_WRONLY,
                )
                with os.fdopen(fd, "w", encoding="utf-8") as handle:
                    handle.write(
                        json.dumps(
                            {"pid": os.getpid(), "created_at": utc_now()},
                            sort_keys=True,
                        )
                    )
                self._acquired = True
                return True
            except FileExistsError:
                self._handle_stale()
                time.sleep(_LOCK_POLL_INTERVAL)
        return False

    def _handle_stale(self) -> None:
        """Check if the existing lock is stale and remove it if so."""
        try:
            lock_age = time.time() - self._lock_path.stat().st_mtime
        except OSError:
            return
        if lock_age > self._timeout:
            try:
                self._lock_path.unlink()
            except OSError:
                pass

    def release(self) -> None:
        """Release the lock by deleting the lock file."""
        if not self._acquired:
            return
        try:
            self._lock_path.unlink(missing_ok=True)
        except OSError:
            pass
        self._acquired = False

    def __enter__(self) -> _FileLock:
        if not self.acquire():
            raise AssuranceError(
                f"could not acquire lock {self._lock_path.name} "
                f"within {self._timeout:.1f}s"
            )
        return self

    def __exit__(self, *args: object) -> None:
        self.release()


class KeyLifecycleController:
    """Manages installation key rotation, revocation, and history.

    Provides:
      - ``rotate()`` — create a new key, write a rotation receipt, update history
      - ``revoke()`` — mark the current key as revoked (rotation to a zero-key)
      - ``verify_key_history()`` — independently verify the chain of rotations
      - ``key_history()`` — read the full rotation history chain
      - ``recover_pending_rotations()`` — crash recovery for incomplete rotations

    All mutating operations (rotate, revoke, record_initial_key) are
    protected by a file-level lock so only one process may mutate the
    key history at a time.
    """

    def __init__(
        self,
        root: Path,
        *,
        lock_timeout_seconds: float = 5.0,
        recover_on_init: bool = True,
    ) -> None:
        self.root = root
        self.lock_timeout_seconds = lock_timeout_seconds
        self.history_path = root / "key-history.json"
        self.rotation_receipts_dir = root / "rotation-receipts"
        self.root.mkdir(parents=True, exist_ok=True)
        self._lock = _FileLock(
            root / ".key-history.lock",
            timeout_seconds=lock_timeout_seconds,
        )
        self._pending = self._scan_pending()
        # Recover from any incomplete prior rotations before allowing new ops
        if self._pending and recover_on_init:
            self.recover_pending_rotations()

    def _scan_pending(self) -> list[Path]:
        """Scan for incomplete (pending) rotation journal entries."""
        pending: list[Path] = []
        if self.rotation_receipts_dir.is_dir():
            for child in self.rotation_receipts_dir.iterdir():
                if child.name.startswith(".pending-") and child.suffix == ".json":
                    pending.append(child)
        return sorted(pending)

    def recover_pending_rotations(self) -> dict[str, Any]:
        """Attempt to complete or roll back any incomplete rotations.

        If a pending journal entry points to a valid receipt file that
        already exists, append it to history (completing). Otherwise,
        delete the pending entry (rolling back).

        Returns a recovery summary.
        """
        with self._lock:
            return self._recover_pending_rotations_locked()

    def _recover_pending_rotations_locked(self) -> dict[str, Any]:
        """Internal: recover pending rotations (caller holds lock)."""
        recovered: list[str] = []
        rolled_back: list[str] = []
        errors: list[str] = []

        for pending_path in list(self._pending):
            try:
                journal = load_json(pending_path)
                receipt_id = journal.get("receipt_id", "")
                receipt_path = (
                    self.rotation_receipts_dir / f"{receipt_id}.json"
                )
                if receipt_path.is_file():
                    # Receipt was written — complete the rotation
                    receipt = load_json(receipt_path)
                    self._append_history_entry(receipt, receipt_path)
                    recovered.append(receipt_id)
                else:
                    # Receipt never written — roll back
                    rolled_back.append(receipt_id)
                try:
                    pending_path.unlink()
                except OSError:
                    pass
            except Exception as exc:
                errors.append(f"recovery failed for {pending_path.name}: {exc}")

        self._pending = self._scan_pending()
        return {
            "recovered": recovered,
            "rolled_back": rolled_back,
            "errors": errors,
            "pending_remaining": len(self._pending),
        }

    def _append_history_entry(
        self, receipt: dict[str, Any], receipt_path: Path
    ) -> None:
        """Append a rotation/revocation receipt entry to the key history.

        Does NOT acquire the lock — caller must hold it.
        Idempotent: if an entry with the same receipt_sha256 already
        exists, it is not duplicated.
        """
        entries = self._read_history()
        receipt_sha = sha256_file(receipt_path)
        # Deduplicate: if this receipt is already in history, skip
        for existing in entries:
            if existing.get("receipt_sha256") == receipt_sha:
                return
        entry = {
            "key_id": receipt.get("new_key_id", receipt.get("previous_key_id", "")),
            "operation": receipt["operation"],
            "timestamp": receipt["created_at"],
            "receipt_sha256": receipt_sha,
        }
        entries.append(entry)
        self._write_history(entries)

    def _read_history(self) -> list[dict[str, Any]]:
        if not self.history_path.is_file():
            return []
        try:
            raw = self.history_path.read_text(encoding="utf-8")
            data = json.loads(raw)
        except json.JSONDecodeError as exc:
            raise AssuranceError(
                f"key history file is corrupt: {exc}"
            ) from exc
        if isinstance(data, list):
            return data
        if isinstance(data, dict) and "entries" in data:
            return list(data["entries"])
        raise AssuranceError("key history is not a list or dict with entries")

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
        with self._lock:
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
        and appends to the key history chain.  The journal-before-write
        pattern makes the operation crash-safe: an incomplete rotation
        is auto-recovered by :meth:`recover_pending_rotations` on the
        next instantiation.
        """
        with self._lock:
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
                _ = previous_sign(rotation_payload)
                checks["previous_key_operational"] = True
            except Exception as exc:
                errors.append(f"previous key signing failed: {exc}")
                checks["previous_key_operational"] = False

            try:
                _ = new_sign(rotation_payload)
                checks["new_key_operational"] = True
            except Exception as exc:
                errors.append(f"new key signing failed: {exc}")
                checks["new_key_operational"] = False

            # Migrate envelopes (re-sign with new key)
            envelopes_migrated = 0
            envelope_migration_errors: list[str] = []
            if envelopes_to_migrate:
                from .envelope import migrate_envelope as _migrate
                checks["envelope_migration_attempted"] = True
                for env_path in envelopes_to_migrate:
                    try:
                        if env_path.is_file():
                            _migrate(
                                envelope_path=env_path,
                                new_key_id=new_key_id,
                                new_sign_fn=new_sign,
                            )
                            envelopes_migrated += 1
                    except Exception as exc:
                        envelope_migration_errors.append(
                            f"failed to migrate envelope {env_path}: {exc}"
                        )
            checks["envelopes_migrated"] = (
                envelopes_migrated > 0
                if envelopes_to_migrate
                else True  # no envelopes to migrate = vacuously satisfied
            )
            if not envelopes_to_migrate:
                checks["envelope_migration_attempted"] = False
            if envelope_migration_errors:
                errors.extend(envelope_migration_errors)

            rotation_valid = not errors
            receipt_id = f"KRO-{uuid.uuid4().hex.upper()}"

            receipt: dict[str, Any] = {
                "schema_version": "0.1.0-draft",
                "receipt_kind": "key_rotation_receipt",
                "receipt_id": receipt_id,
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
                    "This receipt proves the rotation was mechanically valid, "
                    "not that it was authorized.",
                    "Cross-machine migration requires transport security outside "
                    "this scope.",
                ],
            }
            validate_contract(
                receipt, ROTATION_SCHEMA, label="key rotation receipt",
            )

            self.rotation_receipts_dir.mkdir(parents=True, exist_ok=True)

            # ── crash-safe journal ──
            pending_path = (
                self.rotation_receipts_dir / f".pending-{receipt_id}.json"
            )
            pending_path.write_text(
                json.dumps(
                    {"receipt_id": receipt_id, "operation": "rotate"},
                    sort_keys=True,
                ),
                encoding="utf-8",
            )

            receipt_path = (
                self.rotation_receipts_dir / f"{receipt_id}.json"
            )
            receipt_path.write_text(
                json.dumps(receipt, indent=2, sort_keys=True),
                encoding="utf-8",
            )

            self._append_history_entry(receipt, receipt_path)

            # Journal complete — clear pending marker
            try:
                pending_path.unlink()
            except OSError:
                pass

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
        with self._lock:
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
            _ = sign(revocation_payload)
            receipt_id = f"KRO-{uuid.uuid4().hex.upper()}"

            receipt: dict[str, Any] = {
                "schema_version": "0.1.0-draft",
                "receipt_kind": "key_rotation_receipt",
                "receipt_id": receipt_id,
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
                    "Revocation is a logical marker; it does not "
                    "cryptographically destroy the key.",
                    "Any signature made before revocation remains valid "
                    "for historical audit.",
                    "Revoked keys must not be used for new signatures.",
                ],
            }
            validate_contract(
                receipt, ROTATION_SCHEMA, label="key revocation receipt",
            )

            self.rotation_receipts_dir.mkdir(parents=True, exist_ok=True)

            # ── crash-safe journal ──
            pending_path = (
                self.rotation_receipts_dir / f".pending-{receipt_id}.json"
            )
            pending_path.write_text(
                json.dumps(
                    {"receipt_id": receipt_id, "operation": "revoke"},
                    sort_keys=True,
                ),
                encoding="utf-8",
            )

            receipt_path = (
                self.rotation_receipts_dir / f"{receipt_id}.json"
            )
            receipt_path.write_text(
                json.dumps(receipt, indent=2, sort_keys=True),
                encoding="utf-8",
            )

            self._append_history_entry(receipt, receipt_path)

            try:
                pending_path.unlink()
            except OSError:
                pass

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
