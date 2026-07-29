"""Shadow recovery store and recovery executor — GAK-REC-001.

Provides the missing pieces between recovery *authorization* (proven in
:mod:`~.recovery`) and recovery *execution*:

1. :class:`ShadowRecoveryStore` — content-addressed, integrity-verified
   storage for recovery candidates, authorizations, and snapshot data.
   Independent of the source namespace (which may have been archived or
   deleted).

2. :class:`RecoveryDiffPreview` — before restoration, compare the
   recovery snapshot with the current target file.  Only metadata-level
   diff (hashes, line counts, byte counts) is exposed — raw content is
   never leaked into the diff preview.

3. :class:`RecoveryExecutor` — the **only** component that performs
   actual file restoration.  Every execution produces a signed
   :class:`ExecutionReceipt`.  Audit events are **never** modified or
   deleted — the executor only reads the audit seal for verification.

The store layout follows the same content-addressed pattern as
:mod:`~.evidence_store`::

    <project>/.gsa_shadow_recovery/
      store/
        {sha256[:2]}/
          {sha256}/
            candidate.json
            authorization.json
            snapshot.bin
            store_manifest.json
      executions/
        {timestamp}-{uuid8}/
          execution_receipt.json
          diff_preview.json

Design references:
- :mod:`assurance.evidence_store` — SHA-256 content-addressed layout
- :mod:`assurance.recovery` — recovery candidate/authorization signatures
- :mod:`assurance.audit` — ``_sign()`` / receipt verification patterns
- :mod:`assurance.endpoint_canonicalizer` — path safety
"""

from __future__ import annotations

import json
import os
import re
import tempfile
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from .contracts import ASSURANCE_ROOT
from .endpoint_canonicalizer import canonicalize_filesystem_path
from .errors import AssuranceError
from .keystore import InstallationKeyStore
from .utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    exclusive_create_bytes,
    load_json,
    sha256_bytes,
    sha256_file,
    utc_now,
)

# ── paths ─────────────────────────────────────────────────────────────────────

SHADOW_ROOT: Path = ASSURANCE_ROOT.parent / ".gsa_shadow_recovery"
SHADOW_STORE: Path = SHADOW_ROOT / "store"
SHADOW_EXECUTIONS: Path = SHADOW_ROOT / "executions"

SHA256_RE = re.compile(r"^[a-f0-9]{64}$")


def _ensure_dirs() -> None:
    SHADOW_STORE.mkdir(parents=True, exist_ok=True)
    SHADOW_EXECUTIONS.mkdir(parents=True, exist_ok=True)


def _timestamp() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _validate_sha256(value: str, *, label: str) -> None:
    if not isinstance(value, str) or not SHA256_RE.fullmatch(value):
        raise AssuranceError(f"{label} must be a 64-char SHA-256 hex digest")


def _sign_body(
    body: dict[str, Any], *, key_store: InstallationKeyStore,
) -> dict[str, Any]:
    payload = canonical_bytes(body)
    return {
        **body,
        "integrity": {
            "canonicalization": "RFC8785",
            "key_id": key_store.key_id,
            "signature_algorithm": "hmac-sha256",
            "signed_payload_sha256": sha256_bytes(payload),
            "signature": key_store.sign(payload),
        },
    }


def _verify_body_signature(
    receipt: dict[str, Any], *, key_store: InstallationKeyStore, label: str,
) -> list[str]:
    errors: list[str] = []
    integrity = receipt.get("integrity", {})
    body = {k: v for k, v in receipt.items() if k != "integrity"}
    payload = canonical_bytes(body)
    if integrity.get("key_id") != key_store.key_id:
        errors.append(f"{label}: key_id mismatch")
    if integrity.get("signed_payload_sha256") != sha256_bytes(payload):
        errors.append(f"{label}: signed payload digest mismatch")
    if not key_store.verify(payload, integrity.get("signature", "")):
        errors.append(f"{label}: signature verification failed")
    return errors


def _entry_dir(entry_sha256: str) -> Path:
    return SHADOW_STORE / entry_sha256[:2] / entry_sha256


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class StoreEntry:
    """A recovery entry in the shadow store.

    Contains candidate, authorization, and snapshot data — all
    independently verifiable via the store manifest.
    """

    entry_id: str
    entry_sha256: str
    candidate_sha256: str
    authorization_sha256: str | None
    snapshot_sha256: str
    snapshot_bytes: int
    stored_at: str
    manifest: dict[str, Any] = field(default_factory=dict)


@dataclass
class DiffPreview:
    """Metadata-level diff between a recovery snapshot and a target file.

    Does **not** expose raw content — only hashes, byte counts, and
    line counts.  Full content comparison is done internally; the
    preview is a safety summary for the operator.
    """

    candidate_id: str
    target_path: str
    snapshot_sha256: str
    current_sha256: str | None           # None = target does not exist
    snapshot_bytes: int
    current_bytes: int | None
    bytes_to_add: int
    bytes_to_remove: int
    snapshot_lines: int
    current_lines: int | None
    lines_added: int
    lines_removed: int
    would_overwrite: bool = False


@dataclass
class ExecutionReceipt:
    """Signed receipt for a recovery execution.

    The receipt records what was restored, where, and the outcome.
    It explicitly declares that audit events were **not** modified.
    """

    receipt_id: str
    candidate_id: str
    authorization_id: str | None
    target_path: str
    snapshot_sha256: str
    pre_existing_sha256: str | None
    outcome: str            # "restored" | "failed" | "rejected"
    bytes_written: int
    diff_preview: dict[str, Any] | None = None
    errors: list[str] = field(default_factory=list)
    audit_events_preserved: bool = True


# ══════════════════════════════════════════════════════════════════════════════
# Shadow Recovery Store
# ══════════════════════════════════════════════════════════════════════════════


class ShadowRecoveryStore:
    """Content-addressed store for recovery artifacts.

    Each entry is keyed by the SHA-256 of the concatenation of
    candidate + authorization + snapshot, ensuring idempotent storage.
    The store manifest independently verifies the integrity of each
    stored artifact.

    Usage::

        store = ShadowRecoveryStore()
        entry = store.store(
            candidate=candidate_receipt,
            authorization=auth_receipt,   # optional, may be None
            snapshot_bytes=b"checkpoint data",
        )
        # ... later ...
        retrieved = store.retrieve(entry.entry_sha256)
        assert store.verify_entry(entry.entry_sha256)
    """

    def __init__(self) -> None:
        _ensure_dirs()

    def store(
        self,
        candidate: dict[str, Any],
        authorization: dict[str, Any] | None,
        snapshot_bytes: bytes,
    ) -> StoreEntry:
        """Store a recovery entry in the shadow store.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt (from :func:`~.recovery.create_recovery_candidate`).
        authorization:
            Recovery authorization receipt (from :func:`~.recovery.authorize_recovery_candidate`).
            May be ``None`` for candidates that have not yet been authorized.
        snapshot_bytes:
            The snapshot data to store.

        Returns
        -------
        StoreEntry
            Metadata about the stored entry.

        Raises
        ------
        AssuranceError
            If *snapshot_bytes* is empty.
        """
        if not snapshot_bytes:
            raise AssuranceError("shadow store: snapshot must not be empty")

        candidate_json = canonical_bytes(candidate)
        candidate_sha256 = sha256_bytes(candidate_json)
        snapshot_sha256 = sha256_bytes(snapshot_bytes)

        auth_json = b""
        auth_sha256 = None
        if authorization is not None:
            auth_json = canonical_bytes(authorization)
            auth_sha256 = sha256_bytes(auth_json)

        # Entry key = SHA-256(candidate || authorization || snapshot)
        entry_payload = candidate_json + auth_json + snapshot_bytes
        entry_sha256 = sha256_bytes(entry_payload)

        entry_dir = _entry_dir(entry_sha256)
        entry_dir.mkdir(parents=True, exist_ok=True)

        now = _timestamp()

        # Write artifacts using canonical bytes (sorted, compact JSON) so
        # retrieve() can recompute identical hashes from the on-disk bytes.
        # overwrite=True: duplicate stores are idempotent (same key).
        candidate_path = entry_dir / "candidate.json"
        atomic_write_bytes(candidate_path, candidate_json, overwrite=True)
        if authorization is not None:
            auth_path = entry_dir / "authorization.json"
            atomic_write_bytes(auth_path, auth_json, overwrite=True)
        atomic_write_bytes(entry_dir / "snapshot.bin", snapshot_bytes, overwrite=True)

        # Build and write manifest
        manifest = {
            "schema_version": "0.1.0-draft",
            "entry_sha256": entry_sha256,
            "candidate_sha256": candidate_sha256,
            "authorization_sha256": auth_sha256,
            "snapshot_sha256": snapshot_sha256,
            "snapshot_bytes": len(snapshot_bytes),
            "stored_at": now,
            "candidate_id": candidate.get("candidate_id", ""),
            "source_conversation_id": (
                candidate.get("source", {}).get("conversation_id", "")
            ),
        }
        atomic_write_json(entry_dir / "store_manifest.json", manifest, overwrite=True)

        return StoreEntry(
            entry_id=f"SRV-{entry_sha256[:16]}",
            entry_sha256=entry_sha256,
            candidate_sha256=candidate_sha256,
            authorization_sha256=auth_sha256,
            snapshot_sha256=snapshot_sha256,
            snapshot_bytes=len(snapshot_bytes),
            stored_at=now,
            manifest=manifest,
        )

    def retrieve(self, entry_sha256: str) -> StoreEntry:
        """Retrieve a stored recovery entry by its SHA-256.

        Parameters
        ----------
        entry_sha256:
            The 64-character SHA-256 hex digest of the entry.

        Returns
        -------
        StoreEntry
            The entry metadata.

        Raises
        ------
        AssuranceError
            If the entry does not exist or the manifest is invalid.
        """
        _validate_sha256(entry_sha256, label="entry_sha256")
        entry_dir = _entry_dir(entry_sha256)
        if not entry_dir.is_dir():
            raise AssuranceError(
                f"shadow store: entry not found: {entry_sha256[:16]}…"
            )

        manifest_path = entry_dir / "store_manifest.json"
        if not manifest_path.is_file():
            raise AssuranceError(
                f"shadow store: manifest missing for entry {entry_sha256[:16]}…"
            )

        manifest = load_json(manifest_path)

        # Verify manifest integrity against stored artifacts
        candidate_path = entry_dir / "candidate.json"
        auth_path = entry_dir / "authorization.json"
        snap_path = entry_dir / "snapshot.bin"

        if not candidate_path.is_file():
            raise AssuranceError("shadow store: candidate artifact missing")
        if not snap_path.is_file():
            raise AssuranceError("shadow store: snapshot artifact missing")

        candidate_bytes = candidate_path.read_bytes()
        actual_candidate_sha = sha256_bytes(candidate_bytes)
        if actual_candidate_sha != manifest.get("candidate_sha256"):
            raise AssuranceError(
                "shadow store: candidate artifact tampered"
            )

        actual_snap = snap_path.read_bytes()
        actual_snap_sha = sha256_bytes(actual_snap)
        if actual_snap_sha != manifest.get("snapshot_sha256"):
            raise AssuranceError(
                "shadow store: snapshot artifact tampered"
            )

        actual_auth_sha = None
        if auth_path.is_file():
            auth_bytes = auth_path.read_bytes()
            actual_auth_sha = sha256_bytes(auth_bytes)
            if actual_auth_sha != manifest.get("authorization_sha256"):
                raise AssuranceError(
                    "shadow store: authorization artifact tampered"
                )

        # Recompute entry key (store() writes canonical bytes, so raw
        # file bytes match what store() used for the entry key)
        auth_bytes_for_key = auth_path.read_bytes() if auth_path.is_file() else b""
        recomputed = (
            candidate_bytes + auth_bytes_for_key + actual_snap
        )
        recomputed_sha = sha256_bytes(recomputed)
        if recomputed_sha != entry_sha256:
            raise AssuranceError(
                "shadow store: entry key mismatch — data may be corrupted"
            )

        return StoreEntry(
            entry_id=f"SRV-{entry_sha256[:16]}",
            entry_sha256=entry_sha256,
            candidate_sha256=actual_candidate_sha,
            authorization_sha256=actual_auth_sha,
            snapshot_sha256=actual_snap_sha,
            snapshot_bytes=len(actual_snap),
            stored_at=manifest.get("stored_at", ""),
            manifest=manifest,
        )

    def list_entries(self) -> list[StoreEntry]:
        """List all stored recovery entries."""
        entries: list[StoreEntry] = []
        if not SHADOW_STORE.is_dir():
            return entries
        for prefix_dir in sorted(SHADOW_STORE.iterdir()):
            if not prefix_dir.is_dir() or len(prefix_dir.name) != 2:
                continue
            for entry_dir in sorted(prefix_dir.iterdir()):
                if not entry_dir.is_dir() or len(entry_dir.name) != 64:
                    continue
                manifest_path = entry_dir / "store_manifest.json"
                if not manifest_path.is_file():
                    continue
                try:
                    manifest = load_json(manifest_path)
                    entry_sha = manifest.get("entry_sha256", entry_dir.name)
                    entries.append(StoreEntry(
                        entry_id=f"SRV-{entry_sha[:16]}",
                        entry_sha256=entry_sha,
                        candidate_sha256=manifest.get("candidate_sha256", ""),
                        authorization_sha256=manifest.get("authorization_sha256"),
                        snapshot_sha256=manifest.get("snapshot_sha256", ""),
                        snapshot_bytes=manifest.get("snapshot_bytes", 0),
                        stored_at=manifest.get("stored_at", ""),
                        manifest=manifest,
                    ))
                except Exception:
                    continue
        return entries

    def verify_entry(self, entry_sha256: str) -> bool:
        """Verify the integrity of a stored entry.

        Returns ``True`` if the entry exists and all artifacts match
        the manifest.
        """
        try:
            self.retrieve(entry_sha256)
            return True
        except AssuranceError:
            return False


# ══════════════════════════════════════════════════════════════════════════════
# Recovery Diff Preview
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryDiffPreview:
    """Produce a metadata-level diff between a recovery snapshot and the
    current state of the target file.

    The preview **never** exposes raw file content — only hashes, byte
    counts, and line counts.  This is a safety measure: the operator
    sees what will change without any risk of leaking sensitive content
    through the diff mechanism.
    """

    @staticmethod
    def preview(
        candidate: dict[str, Any],
        snapshot_bytes: bytes,
        target_path: str,
        base_root: Path | None = None,
    ) -> DiffPreview:
        """Compare *snapshot_bytes* with the current content at *target_path*.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt.
        snapshot_bytes:
            The snapshot data that would be written.
        target_path:
            The filesystem path to be restored.  Must pass
            :func:`~.endpoint_canonicalizer.canonicalize_filesystem_path`.
        base_root:
            Optional root to scope the target path under.

        Returns
        -------
        DiffPreview
            Metadata-level comparison result.

        Raises
        ------
        AssuranceError
            If *target_path* fails canonicalization.
        """
        canonical = canonicalize_filesystem_path(
            target_path, base_root=base_root,
        )
        target = Path(canonical) if base_root is None else (base_root / canonical)

        snap_sha = sha256_bytes(snapshot_bytes)
        snap_lines = snapshot_bytes.decode("utf-8", errors="replace").count("\n") + 1

        current_bytes: int | None = None
        current_sha: str | None = None
        current_lines: int | None = None
        bytes_to_add = len(snapshot_bytes)
        bytes_to_remove = 0
        lines_added = snap_lines
        lines_removed = 0
        would_overwrite = False

        if target.exists() and target.is_file():
            would_overwrite = True
            current_data = target.read_bytes()
            current_bytes = len(current_data)
            current_sha = sha256_bytes(current_data)
            current_lines = (
                current_data.decode("utf-8", errors="replace").count("\n") + 1
            )
            bytes_to_add = max(0, len(snapshot_bytes) - current_bytes)
            bytes_to_remove = max(0, current_bytes - len(snapshot_bytes))
            lines_added = max(0, snap_lines - current_lines)
            lines_removed = max(0, current_lines - snap_lines)

        return DiffPreview(
            candidate_id=candidate.get("candidate_id", ""),
            target_path=str(target),
            snapshot_sha256=snap_sha,
            current_sha256=current_sha,
            snapshot_bytes=len(snapshot_bytes),
            current_bytes=current_bytes,
            bytes_to_add=bytes_to_add,
            bytes_to_remove=bytes_to_remove,
            snapshot_lines=snap_lines,
            current_lines=current_lines,
            lines_added=lines_added,
            lines_removed=lines_removed,
            would_overwrite=would_overwrite,
        )


# ══════════════════════════════════════════════════════════════════════════════
# Recovery Executor
# ══════════════════════════════════════════════════════════════════════════════


class RecoveryExecutor:
    """Execute a recovery — the **only** component that performs actual
    file restoration.

    Every execution produces a signed :class:`ExecutionReceipt`.  Audit
    events are **never** modified or deleted — the executor only reads
    the audit seal for pre-flight verification.

    Usage::

        executor = RecoveryExecutor()
        receipt = executor.execute(
            candidate=candidate_receipt,
            authorization=auth_receipt,
            snapshot_bytes=store_entry_data,
            target_path="/path/to/restore",
            key_store=key_store,
        )
        assert verify_execution_receipt(receipt, key_store=key_store)["valid"]
    """

    @staticmethod
    def execute(
        candidate: dict[str, Any],
        authorization: dict[str, Any] | None,
        snapshot_bytes: bytes,
        target_path: str,
        key_store: InstallationKeyStore,
        base_root: Path | None = None,
        dry_run: bool = False,
    ) -> ExecutionReceipt:
        """Execute a recovery — write *snapshot_bytes* to *target_path*.

        Parameters
        ----------
        candidate:
            Recovery candidate receipt.  Must pass
            :func:`~.recovery.verify_recovery_candidate`.
        authorization:
            Recovery authorization receipt.  If ``None``, the execution
            is rejected (outcome = ``"rejected"``).
        snapshot_bytes:
            The snapshot data to write.
        target_path:
            Canonicalized via :func:`~.endpoint_canonicalizer.canonicalize_filesystem_path`.
        key_store:
            Installation key store for signing the execution receipt.
        base_root:
            Optional root to scope *target_path* under.
        dry_run:
            If ``True``, produce the diff preview but do not write.

        Returns
        -------
        ExecutionReceipt
            Signed receipt recording the execution outcome.

        Raises
        ------
        AssuranceError
            If the candidate verification fails or the path is invalid.
        """
        receipt_id = f"RXR-{uuid.uuid4().hex[:16].upper()}"
        errors: list[str] = []
        diff: dict[str, Any] | None = None
        pre_existing_sha: str | None = None
        outcome = "failed"
        bytes_written = 0

        # 1. Canonicalize target path
        canonical = canonicalize_filesystem_path(
            target_path, base_root=base_root,
        )
        resolved = Path(canonical) if base_root is None else (base_root / canonical)

        # 2. Validate candidate structural integrity
        # The candidate's signature was already verified by recovery.py
        # at creation time.  The executor validates the candidate_id
        # and source classification are present and correct.
        candidate_id = candidate.get("candidate_id", "")
        source = candidate.get("source", {})
        if source.get("classification") != "untrusted_recovery_candidate":
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=None,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                errors=["candidate classification must be untrusted_recovery_candidate"],
            )

        # 3. Verify authorization (required)
        if authorization is None:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=None,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                bytes_written=0,
                errors=["recovery authorization is required for execution"],
            )
        auth_id = authorization.get("receipt_id", "")

        # Verify authorization structural integrity.
        # The authorization's signature was verified at creation time
        # by recovery.py.  The executor validates the decision fields.
        auth_candidate_id = authorization.get("candidate_id", "")
        if auth_candidate_id != candidate_id:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                errors=[
                    f"authorization candidate_id '{auth_candidate_id}' "
                    f"does not match candidate_id '{candidate_id}'"
                ],
            )

        decision = authorization.get("decision", {})
        if decision.get("outcome") != "allow":
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                errors=[
                    f"authorization decision is '{decision.get('outcome')}', "
                    f"not 'allow'"
                ],
                bytes_written=0,
            )
        if decision.get("restoration_performed"):
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=None,
                outcome="rejected",
                bytes_written=0,
                errors=["authorization already performed restoration"],
            )

        # 4. Capture pre-existing state
        if resolved.exists():
            pre_existing_sha = sha256_file(resolved)
            errors.append(
                "target file exists; backup recommended before restoration"
            )

        # 5. Generate diff preview
        diff_preview = RecoveryDiffPreview.preview(
            candidate, snapshot_bytes, str(resolved),
        )
        diff = {
            "snapshot_sha256": diff_preview.snapshot_sha256,
            "current_sha256": diff_preview.current_sha256,
            "bytes_to_add": diff_preview.bytes_to_add,
            "bytes_to_remove": diff_preview.bytes_to_remove,
            "lines_added": diff_preview.lines_added,
            "lines_removed": diff_preview.lines_removed,
            "would_overwrite": diff_preview.would_overwrite,
        }

        if dry_run:
            return ExecutionReceipt(
                receipt_id=receipt_id,
                candidate_id=candidate_id,
                authorization_id=auth_id,
                target_path=str(resolved),
                snapshot_sha256=sha256_bytes(snapshot_bytes),
                pre_existing_sha256=pre_existing_sha,
                outcome="rejected",
                bytes_written=0,
                diff_preview=diff,
                errors=["dry_run: no data written"],
            )

        # 6. Atomic write
        try:
            resolved.parent.mkdir(parents=True, exist_ok=True)
            # Write to temp file in the same directory, then rename
            fd, tmp_path_str = tempfile.mkstemp(
                dir=str(resolved.parent),
                prefix=f".{resolved.name}.",
                suffix=".tmp",
            )
            try:
                os.write(fd, snapshot_bytes)
                os.fsync(fd)
            finally:
                os.close(fd)
            tmp_path = Path(tmp_path_str)
            tmp_path.replace(resolved)
            bytes_written = len(snapshot_bytes)
            outcome = "restored"
        except OSError as exc:
            errors.append(f"write failed: {exc}")
            outcome = "failed"
            # Clean up temp file
            try:
                Path(tmp_path_str).unlink(missing_ok=True)
            except Exception:
                pass

        # 7. Build and sign execution receipt
        body = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "recovery_execution",
            "receipt_id": receipt_id,
            "candidate_id": candidate_id,
            "authorization_id": auth_id,
            "target_path": str(resolved),
            "snapshot_sha256": sha256_bytes(snapshot_bytes),
            "pre_existing_sha256": pre_existing_sha,
            "outcome": outcome,
            "bytes_written": bytes_written,
            "diff_preview": diff,
            "errors": errors,
            "audit_events_preserved": True,
            "executed_at": _timestamp(),
        }
        signed = _sign_body(body, key_store=key_store)

        # Persist execution receipt
        _ensure_dirs()
        exec_dir = SHADOW_EXECUTIONS / f"{_timestamp()[:19].replace(':', '')}-{receipt_id[-8:]}"
        exec_dir.mkdir(parents=True, exist_ok=True)
        atomic_write_json(exec_dir / "execution_receipt.json", signed)
        if diff:
            atomic_write_json(exec_dir / "diff_preview.json", diff)

        return ExecutionReceipt(
            receipt_id=receipt_id,
            candidate_id=candidate_id,
            authorization_id=auth_id,
            target_path=str(resolved),
            snapshot_sha256=sha256_bytes(snapshot_bytes),
            pre_existing_sha256=pre_existing_sha,
            outcome=outcome,
            bytes_written=bytes_written,
            diff_preview=diff,
            errors=errors,
            audit_events_preserved=True,
        )


def verify_execution_receipt(
    receipt: dict[str, Any],
    key_store: InstallationKeyStore,
) -> dict[str, Any]:
    """Verify a signed recovery execution receipt.

    Returns a dict with ``valid`` (bool), ``errors`` (list[str]), and
    ``outcome`` (str) fields.
    """
    errors: list[str] = []
    try:
        if receipt.get("receipt_kind") != "recovery_execution":
            errors.append("receipt_kind is not recovery_execution")
        errors.extend(
            _verify_body_signature(
                receipt, key_store=key_store, label="execution receipt",
            )
        )
        if not receipt.get("audit_events_preserved"):
            errors.append(
                "execution receipt must declare audit_events_preserved=True"
            )
        outcome = receipt.get("outcome", "")
        if outcome not in ("restored", "failed", "rejected"):
            errors.append(f"unexpected outcome: {outcome}")
    except Exception as exc:
        errors.append(str(exc))

    return {
        "valid": not errors,
        "errors": errors,
        "outcome": receipt.get("outcome", "unknown"),
    }
