#!/usr/bin/env python3
"""Append one verified Global Progress transition event to a replayable JSONL journal."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from prototype.fep_agent_proto.journal_lock import (
    JournalLockError,
    exclusive_journal_lock,
)


EVENT_SCHEMA = ROOT / "runtime" / "run-event-v0.1.schema.json"
RECEIPT_SCHEMA = ROOT / "runtime" / "global-progress-transition-receipt-v0.1.schema.json"
RECEIPT_SCHEMA_NAME = RECEIPT_SCHEMA.name
TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}


class AppendError(RuntimeError):
    pass


def _canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def _event_hash(event: dict[str, Any]) -> str:
    return hashlib.sha256(_canonical({key: value for key, value in event.items() if key != "event_sha256"})).hexdigest()


def _validate(event: dict[str, Any], label: str) -> None:
    schema = json.loads(EVENT_SCHEMA.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(event),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        raise AppendError(f"{label} schema invalid: {errors[0].message}")
    if event["payload_sha256"] != hashlib.sha256(_canonical(event["payload"])).hexdigest():
        raise AppendError(f"{label} payload digest mismatch")
    if event["event_sha256"] != _event_hash(event):
        raise AppendError(f"{label} event digest mismatch")


def _validate_transition_candidate(event: dict[str, Any]) -> None:
    _validate(event, "candidate event")
    if event["event_type"] != "gate_decision":
        raise AppendError("candidate event is not a gate_decision")
    if event["payload_schema"] != RECEIPT_SCHEMA_NAME:
        raise AppendError("candidate payload schema is not the Global Progress transition receipt")
    receipt_schema = json.loads(RECEIPT_SCHEMA.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(
            receipt_schema,
            format_checker=FormatChecker(),
        ).iter_errors(event["payload"]),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        raise AppendError(f"candidate transition receipt invalid: {errors[0].message}")


def _load_event(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise AppendError(f"cannot read event: {exc}") from exc
    if not isinstance(value, dict):
        raise AppendError("event must be a JSON object")
    return value


def _replay(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    events: list[dict[str, Any]] = []
    previous = None
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        raise AppendError(f"cannot read journal: {exc}") from exc
    for index, line in enumerate(lines):
        try:
            event = json.loads(line)
        except json.JSONDecodeError as exc:
            raise AppendError(f"journal line {index} is invalid JSON: {exc}") from exc
        if not isinstance(event, dict):
            raise AppendError(f"journal line {index} is not an object")
        _validate(event, f"journal event {index}")
        if event["sequence"] != index or event["previous_event_sha256"] != previous:
            raise AppendError(f"journal chain mismatch at sequence {index}")
        if index and (
            event["run_id"] != events[0]["run_id"]
            or event["run_manifest_sha256"] != events[0]["run_manifest_sha256"]
        ):
            raise AppendError(f"journal run binding mismatch at sequence {index}")
        if event["event_type"] in TERMINAL_EVENTS and index != len(lines) - 1:
            raise AppendError("terminal event is not final")
        events.append(event)
        previous = event["event_sha256"]
    return events


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--event", required=True)
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    journal = Path(args.journal)
    try:
        event = _load_event(Path(args.event))
        _validate_transition_candidate(event)
        with exclusive_journal_lock(
            journal,
            timeout_seconds=args.lock_timeout_seconds,
        ):
            existing = _replay(journal)
            if existing and existing[-1]["event_type"] in TERMINAL_EVENTS:
                raise AppendError("refusing to append after terminal event")
            previous = existing[-1]["event_sha256"] if existing else None
            if event["sequence"] != len(existing) or event["previous_event_sha256"] != previous:
                raise AppendError("candidate sequence/previous digest does not extend journal")
            if existing and (
                event["run_id"] != existing[0]["run_id"]
                or event["run_manifest_sha256"] != existing[0]["run_manifest_sha256"]
            ):
                raise AppendError("candidate run binding differs from journal")
            journal.parent.mkdir(parents=True, exist_ok=True)
            encoded = _canonical(event) + b"\n"
            with journal.open("ab") as handle:
                handle.write(encoded)
                handle.flush()
                os.fsync(handle.fileno())
            replayed = _replay(journal)
    except (AppendError, JournalLockError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(json.dumps({"valid": True, "event_count": len(replayed), "last_event_sha256": replayed[-1]["event_sha256"]}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
