from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from .errors import AssuranceError
from .io_utils import canonical_bytes, load_json, sha256_bytes, sha256_file, utc_now
from .journal_lock import JournalLockError, exclusive_journal_lock
from .schema import validate_instance


RUNTIME_ROOT = Path(__file__).resolve().parents[1] / "runtime"
TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}


def _event_hash(event: dict[str, Any]) -> str:
    projection = dict(event)
    projection.pop("event_sha256", None)
    return sha256_bytes(canonical_bytes(projection))


def _append_event_unlocked(
    journal_path: Path,
    *,
    run_id: str,
    run_manifest_sha256: str,
    event_type: str,
    payload_schema: str,
    payload: dict[str, Any],
    redaction: str = "none",
) -> dict[str, Any]:
    previous: str | None = None
    sequence = 0
    if journal_path.exists():
        replay = _replay_journal_unlocked(
            run_manifest_path=None,
            journal_path=journal_path,
            expected_manifest_sha256=run_manifest_sha256,
            expected_run_id=run_id,
            require_terminal=False,
        )
        if not replay["valid"]:
            raise AssuranceError("refusing to append to an invalid journal")
        sequence = replay["event_count"]
        previous = replay["last_event_sha256"]
        if replay["terminal_event"] is not None:
            raise AssuranceError("refusing to append after terminal event")

    event = {
        "schema_version": "0.1.0-draft",
        "run_id": run_id,
        "event_id": f"EVT-{run_id[4:]}-{sequence:06d}",
        "sequence": sequence,
        "timestamp": utc_now(),
        "event_type": event_type,
        "run_manifest_sha256": run_manifest_sha256,
        "previous_event_sha256": previous,
        "payload_schema": payload_schema,
        "payload": payload,
        "payload_sha256": sha256_bytes(canonical_bytes(payload)),
        "redaction": redaction,
        "event_sha256": "0" * 64,
    }
    event["event_sha256"] = _event_hash(event)
    validate_instance(
        event,
        RUNTIME_ROOT / "run-event-v0.1.schema.json",
        label=f"run event {sequence}",
    )
    line = json.dumps(
        event,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8") + b"\n"
    journal_path.parent.mkdir(parents=True, exist_ok=True)
    with journal_path.open("ab") as handle:
        handle.write(line)
        handle.flush()
        os.fsync(handle.fileno())
    return event


def append_event(
    journal_path: Path,
    *,
    run_id: str,
    run_manifest_sha256: str,
    event_type: str,
    payload_schema: str,
    payload: dict[str, Any],
    redaction: str = "none",
) -> dict[str, Any]:
    try:
        with exclusive_journal_lock(journal_path):
            return _append_event_unlocked(
                journal_path,
                run_id=run_id,
                run_manifest_sha256=run_manifest_sha256,
                event_type=event_type,
                payload_schema=payload_schema,
                payload=payload,
                redaction=redaction,
            )
    except JournalLockError as exc:
        raise AssuranceError(str(exc)) from exc


def _replay_journal_unlocked(
    *,
    run_manifest_path: Path | None,
    journal_path: Path,
    expected_manifest_sha256: str | None = None,
    expected_run_id: str | None = None,
    require_terminal: bool = True,
) -> dict[str, Any]:
    errors: list[str] = []
    if run_manifest_path is not None:
        manifest = load_json(run_manifest_path)
        validate_instance(
            manifest,
            RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
            label="run manifest",
        )
        expected_manifest_sha256 = sha256_file(run_manifest_path)
        expected_run_id = manifest["run_id"]
    if not expected_manifest_sha256 or not expected_run_id:
        raise AssuranceError("replay requires a run manifest or explicit expected identifiers")
    if not journal_path.is_file():
        raise AssuranceError(f"journal does not exist: {journal_path}")

    events: list[dict[str, Any]] = []
    with journal_path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, 1):
            if not line.strip():
                errors.append(f"blank journal line {line_number}")
                continue
            try:
                event = json.loads(line)
            except json.JSONDecodeError as exc:
                errors.append(f"invalid JSON at line {line_number}: {exc}")
                continue
            try:
                validate_instance(
                    event,
                    RUNTIME_ROOT / "run-event-v0.1.schema.json",
                    label=f"journal line {line_number}",
                )
            except AssuranceError as exc:
                errors.append(str(exc))
            events.append(event)

    previous: str | None = None
    terminal_events: list[str] = []
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
        if isinstance(payload, dict):
            if event.get("payload_sha256") != sha256_bytes(canonical_bytes(payload)):
                errors.append(f"payload digest mismatch at event {expected_sequence}")
        if event.get("event_sha256") != _event_hash(event):
            errors.append(f"event digest mismatch at event {expected_sequence}")
        previous = event.get("event_sha256")
        if event.get("event_type") in TERMINAL_EVENTS:
            terminal_events.append(event["event_type"])
            if expected_sequence != len(events) - 1:
                errors.append("terminal event is not last")

    if require_terminal and len(terminal_events) != 1:
        errors.append(f"expected exactly one terminal event, found {len(terminal_events)}")
    if len(terminal_events) > 1:
        errors.append("multiple terminal events")
    return {
        "schema_version": "0.1.0",
        "valid": not errors,
        "run_id": expected_run_id,
        "run_manifest_sha256": expected_manifest_sha256,
        "journal_path": str(journal_path),
        "event_count": len(events),
        "last_event_sha256": previous,
        "terminal_event": terminal_events[-1] if terminal_events else None,
        "errors": errors,
        "checked_at": utc_now(),
        "limitations": ["Replay verifies mechanics and does not score model correctness."],
    }


def replay_journal(
    *,
    run_manifest_path: Path | None,
    journal_path: Path,
    expected_manifest_sha256: str | None = None,
    expected_run_id: str | None = None,
    require_terminal: bool = True,
) -> dict[str, Any]:
    if not journal_path.is_file():
        return _replay_journal_unlocked(
            run_manifest_path=run_manifest_path,
            journal_path=journal_path,
            expected_manifest_sha256=expected_manifest_sha256,
            expected_run_id=expected_run_id,
            require_terminal=require_terminal,
        )
    try:
        with exclusive_journal_lock(journal_path):
            return _replay_journal_unlocked(
                run_manifest_path=run_manifest_path,
                journal_path=journal_path,
                expected_manifest_sha256=expected_manifest_sha256,
                expected_run_id=expected_run_id,
                require_terminal=require_terminal,
            )
    except JournalLockError as exc:
        raise AssuranceError(str(exc)) from exc
