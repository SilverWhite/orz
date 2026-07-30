from __future__ import annotations

from dataclasses import dataclass
import json
from pathlib import Path
from typing import Any

from .errors import AssuranceError
from .io_utils import sha256_bytes
from .journal import TERMINAL_EVENTS, _event_hash
from .journal_lock import JournalLockError, exclusive_journal_lock


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
    previous: str | None = None
    for index, line in enumerate(lines):
        try:
            event = json.loads(line.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            errors.append(f"invalid JSON at line {index + 1}: {exc}")
            continue
        if not isinstance(event, dict):
            errors.append(f"journal line {index + 1} is not an object")
            continue
        if event.get("sequence") != index:
            errors.append(f"sequence mismatch at event {index}")
        if event.get("run_id") != expected_run_id:
            errors.append(f"run ID mismatch at event {index}")
        if event.get("run_manifest_sha256") != expected_manifest_sha256:
            errors.append(f"manifest digest mismatch at event {index}")
        if event.get("previous_event_sha256") != previous:
            errors.append(f"previous-event digest mismatch at event {index}")
        try:
            rebuilt_event_sha = _event_hash(event)
        except Exception as exc:
            errors.append(str(exc))
        else:
            if event.get("event_sha256") != rebuilt_event_sha:
                errors.append(f"event digest mismatch at event {index}")
        previous = event.get("event_sha256")
        if event.get("event_type") in TERMINAL_EVENTS and index != len(lines) - 1:
            errors.append("terminal event is not last")
        events.append(event)
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
        with exclusive_journal_lock(journal_path, timeout_seconds=lock_timeout_seconds):
            raw = journal_path.read_bytes()
    except (JournalLockError, OSError) as exc:
        raise AssuranceError(f"cannot read journal {journal_path}: {exc}") from exc
    return _inspect_bytes(
        raw,
        expected_run_id=expected_run_id,
        expected_manifest_sha256=expected_manifest_sha256,
    ).public()


def recover_journal(*args: Any, **kwargs: Any) -> dict[str, Any]:
    raise AssuranceError(
        "journal recovery write path was not migrated in this runtime-first slice"
    )
