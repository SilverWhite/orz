from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any

from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    sha256_bytes,
    sha256_file,
    utc_now,
)
from .journal import TERMINAL_EVENTS, _event_hash, _replay_journal_unlocked
from .journal_lock import JournalLockError, exclusive_journal_lock
from .layout import RUNTIME_ROOT
from .schema import validate_instance


@dataclass(frozen=True)
class _Inspection:
    status: str
    classification: str
    original: bytes
    retained: bytes
    discarded: bytes
    events: list[dict[str, Any]]
    errors: list[str]

    def public(self) -> dict[str, Any]:
        original_sha = sha256_bytes(self.original)
        discarded_sha = sha256_bytes(self.discarded) if self.discarded else None
        last_event = self.events[-1] if self.events else {}
        return {
            "status": self.status,
            "classification": self.classification,
            "recovery_id": f"JRR-{original_sha[:16].upper()}",
            "original_journal_sha256": original_sha,
            "original_size_bytes": len(self.original),
            "retained_size_bytes": len(self.retained),
            "discarded_offset_bytes": len(self.retained)
            if self.discarded
            else len(self.original),
            "discarded_size_bytes": len(self.discarded),
            "discarded_sha256": discarded_sha,
            "prefix_event_count": len(self.events),
            "prefix_last_event_sha256": last_event.get("event_sha256"),
            "prefix_terminal_event": (
                last_event.get("event_type")
                if last_event.get("event_type") in TERMINAL_EVENTS
                else None
            ),
            "errors": list(self.errors),
        }


def _json_line(value: dict[str, Any]) -> bytes:
    return (
        json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
    )


def _validate_bytes(
    data: bytes,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> tuple[list[dict[str, Any]], list[str]]:
    errors: list[str] = []
    if not data or not data.endswith(b"\n"):
        return [], ["candidate journal is empty or lacks a final newline"]
    lines = data.splitlines()
    events: list[dict[str, Any]] = []
    for line_number, line in enumerate(lines, 1):
        if not line.strip():
            errors.append(f"blank journal line {line_number}")
            continue
        try:
            decoded = line.decode("utf-8")
            event = json.loads(decoded)
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON at line {line_number}: {exc}")
            continue
        if not isinstance(event, dict):
            errors.append(f"journal line {line_number} is not an object")
            continue
        try:
            validate_instance(
                event,
                RUNTIME_ROOT / "run-event-v0.1.schema.json",
                label=f"journal line {line_number}",
            )
        except PrototypeError as exc:
            errors.append(str(exc))
        events.append(event)

    previous: str | None = None
    for expected_sequence, event in enumerate(events):
        if event.get("sequence") != expected_sequence:
            errors.append(f"sequence mismatch at event {expected_sequence}")
        if event.get("run_id") != expected_run_id:
            errors.append(f"run ID mismatch at event {expected_sequence}")
        if event.get("run_manifest_sha256") != expected_manifest_sha256:
            errors.append(f"manifest digest mismatch at event {expected_sequence}")
        if event.get("previous_event_sha256") != previous:
            errors.append(f"previous-event digest mismatch at event {expected_sequence}")
        payload = event.get("payload")
        if not isinstance(payload, dict):
            errors.append(f"payload is not an object at event {expected_sequence}")
        else:
            try:
                payload_sha = sha256_bytes(canonical_bytes(payload))
            except PrototypeError as exc:
                errors.append(str(exc))
            else:
                if event.get("payload_sha256") != payload_sha:
                    errors.append(f"payload digest mismatch at event {expected_sequence}")
        try:
            rebuilt_event_sha = _event_hash(event)
        except PrototypeError as exc:
            errors.append(str(exc))
        else:
            if event.get("event_sha256") != rebuilt_event_sha:
                errors.append(f"event digest mismatch at event {expected_sequence}")
        previous = event.get("event_sha256")
        if event.get("event_type") in TERMINAL_EVENTS and expected_sequence != len(events) - 1:
            errors.append("terminal event is not last")
    return events, errors


def _inspect_bytes(
    raw: bytes,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> _Inspection:
    if not raw:
        return _Inspection(
            "unrecoverable",
            "empty_journal",
            raw,
            b"",
            b"",
            [],
            ["empty journal has no independently valid prefix"],
        )
    if raw.endswith(b"\n"):
        events, errors = _validate_bytes(
            raw,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
        if not errors:
            return _Inspection("valid", "valid_journal", raw, raw, b"", events, [])
        return _Inspection(
            "unrecoverable",
            "history_corruption",
            raw,
            raw,
            b"",
            events,
            errors,
        )

    normalized = raw + b"\n"
    events, errors = _validate_bytes(
        normalized,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    )
    if not errors:
        return _Inspection(
            "recoverable",
            "missing_newline_normalized",
            raw,
            normalized,
            b"",
            events,
            [],
        )

    last_newline = raw.rfind(b"\n")
    if last_newline < 0:
        return _Inspection(
            "unrecoverable",
            "no_valid_prefix",
            raw,
            b"",
            raw,
            [],
            errors,
        )
    retained = raw[: last_newline + 1]
    discarded = raw[last_newline + 1 :]
    prefix_events, prefix_errors = _validate_bytes(
        retained,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    )
    if prefix_errors or not prefix_events or not discarded:
        return _Inspection(
            "unrecoverable",
            "history_corruption",
            raw,
            retained,
            discarded,
            prefix_events,
            [*prefix_errors, *errors],
        )
    return _Inspection(
        "recoverable",
        "torn_tail_removed",
        raw,
        retained,
        discarded,
        prefix_events,
        errors,
    )


def inspect_journal_recovery(
    journal_path: Path,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    try:
        with exclusive_journal_lock(
            journal_path,
            timeout_seconds=lock_timeout_seconds,
        ):
            raw = journal_path.read_bytes()
    except (JournalLockError, OSError) as exc:
        raise PrototypeError(f"cannot read journal {journal_path}: {exc}") from exc
    return _inspect_bytes(
        raw,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    ).public()


def _build_recovery_event(
    inspection: _Inspection,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
    quarantine_sha256: str | None,
) -> dict[str, Any]:
    public = inspection.public()
    previous = inspection.events[-1]["event_sha256"]
    sequence = len(inspection.events)
    payload = {
        "schema_version": "0.1.0",
        "artifact_kind": "journal-recovery-event",
        "recovery_id": public["recovery_id"],
        "classification": inspection.classification,
        "original_journal_sha256": public["original_journal_sha256"],
        "original_size_bytes": public["original_size_bytes"],
        "retained_size_bytes": public["retained_size_bytes"],
        "discarded_offset_bytes": public["discarded_offset_bytes"],
        "discarded_size_bytes": public["discarded_size_bytes"],
        "discarded_sha256": public["discarded_sha256"],
        "quarantine_sha256": quarantine_sha256,
        "prefix_last_event_sha256": previous,
    }
    validate_instance(
        payload,
        RUNTIME_ROOT / "journal-recovery-event-v0.1.schema.json",
        label="journal recovery event payload",
    )
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": expected_run_id,
        "event_id": f"EVT-{expected_run_id[4:]}-{sequence:06d}",
        "sequence": sequence,
        "timestamp": utc_now(),
        "event_type": "artifact_registered",
        "run_manifest_sha256": expected_manifest_sha256,
        "previous_event_sha256": previous,
        "payload_schema": "journal-recovery-event-v0.1.schema.json",
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": "metadata_only",
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    validate_instance(
        event,
        RUNTIME_ROOT / "run-event-v0.1.schema.json",
        label="journal recovery run event",
    )
    return event


def recover_journal(
    journal_path: Path,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
    receipt_path: Path,
    quarantine_path: Path | None = None,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    targets = [journal_path.resolve(), receipt_path.resolve()]
    if quarantine_path is not None:
        targets.append(quarantine_path.resolve())
    if len(targets) != len(set(targets)):
        raise PrototypeError("journal, receipt, and quarantine paths must be distinct")
    if receipt_path.exists():
        raise PrototypeError(f"refusing to overwrite existing recovery receipt: {receipt_path}")
    if quarantine_path is not None and quarantine_path.exists():
        raise PrototypeError(f"refusing to overwrite existing quarantine: {quarantine_path}")

    try:
        with exclusive_journal_lock(
            journal_path,
            timeout_seconds=lock_timeout_seconds,
        ):
            original = journal_path.read_bytes()
            inspection = _inspect_bytes(
                original,
                expected_run_id=expected_run_id,
                expected_manifest_sha256=expected_manifest_sha256,
            )
            if inspection.status != "recoverable":
                raise PrototypeError(
                    f"journal recovery refused: {inspection.classification}"
                )
            if inspection.discarded and quarantine_path is None:
                raise PrototypeError("torn-tail recovery requires an explicit quarantine path")

            quarantine_sha: str | None = None
            if inspection.discarded:
                assert quarantine_path is not None
                atomic_write_bytes(quarantine_path, inspection.discarded)
                quarantine_sha = sha256_file(quarantine_path)

            repaired = inspection.retained
            recovery_event: dict[str, Any] | None = None
            prefix_is_terminal = (
                inspection.events[-1]["event_type"] in TERMINAL_EVENTS
            )
            if not prefix_is_terminal:
                recovery_event = _build_recovery_event(
                    inspection,
                    expected_run_id=expected_run_id,
                    expected_manifest_sha256=expected_manifest_sha256,
                    quarantine_sha256=quarantine_sha,
                )
                repaired += _json_line(recovery_event)

            atomic_write_bytes(journal_path, repaired, overwrite=True)
            replay = _replay_journal_unlocked(
                run_manifest_path=None,
                journal_path=journal_path,
                expected_manifest_sha256=expected_manifest_sha256,
                expected_run_id=expected_run_id,
                require_terminal=prefix_is_terminal,
            )
            if not replay["valid"]:
                atomic_write_bytes(journal_path, original, overwrite=True)
                raise PrototypeError(
                    f"recovered journal failed replay and was restored: {replay['errors']}"
                )

            public = inspection.public()
            receipt = {
                "schema_version": "0.1.0",
                "artifact_kind": "journal-recovery-receipt",
                "valid": True,
                "recovery_id": public["recovery_id"],
                "classification": inspection.classification,
                "run_id": expected_run_id,
                "run_manifest_sha256": expected_manifest_sha256,
                "original_journal_sha256": public["original_journal_sha256"],
                "original_size_bytes": public["original_size_bytes"],
                "retained_size_bytes": public["retained_size_bytes"],
                "discarded_offset_bytes": public["discarded_offset_bytes"],
                "discarded_size_bytes": public["discarded_size_bytes"],
                "discarded_sha256": public["discarded_sha256"],
                "quarantine_path": (
                    str(quarantine_path.resolve()) if quarantine_sha is not None else None
                ),
                "quarantine_sha256": quarantine_sha,
                "recovery_event_sha256": (
                    recovery_event["event_sha256"] if recovery_event is not None else None
                ),
                "final_journal_sha256": sha256_file(journal_path),
                "final_event_count": replay["event_count"],
                "final_last_event_sha256": replay["last_event_sha256"],
                "recovered_at": utc_now(),
                "limitations": [
                    "Recovery proves mechanical prefix continuity, not semantic event truth.",
                    "Advisory locking cannot constrain non-cooperating or multi-host writers.",
                ],
            }
            validate_instance(
                receipt,
                RUNTIME_ROOT / "journal-recovery-receipt-v0.1.schema.json",
                label="journal recovery receipt",
            )
            atomic_write_json(receipt_path, receipt)
            return receipt
    except (JournalLockError, OSError) as exc:
        raise PrototypeError(f"journal recovery failed: {exc}") from exc
