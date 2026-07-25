#!/usr/bin/env python3
"""Verify a disposable Global Progress transition event and state projection."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from build_global_progress_transition_event import build_event


ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "runtime"
EVENT_SCHEMA = RUNTIME / "run-event-v0.1.schema.json"
RECEIPT_SCHEMA = RUNTIME / "global-progress-transition-receipt-v0.1.schema.json"
REPORT_SCHEMA = RUNTIME / "global-progress-transition-verification-v0.1.schema.json"


def _canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def _digest(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _read(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def _schema_errors(path: Path, value: dict[str, Any], label: str) -> list[str]:
    schema = json.loads(path.read_text(encoding="utf-8"))
    return [
        f"{label}#/{'/'.join(map(str, error.absolute_path))}: {error.message}"
        for error in sorted(
            Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
            key=lambda item: list(item.absolute_path),
        )
    ]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--checkpoint-verification", required=True)
    parser.add_argument("--holistic-history")
    parser.add_argument("--holistic-review")
    parser.add_argument("--holistic-disposition")
    parser.add_argument("--holistic-verification")
    parser.add_argument("--event", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    paths = {name: Path(getattr(args, name)) for name in ("request", "checkpoint", "checkpoint_verification", "event")}
    holistic_review = Path(args.holistic_review) if args.holistic_review else None
    holistic_history = Path(args.holistic_history) if args.holistic_history else None
    holistic_disposition = Path(args.holistic_disposition) if args.holistic_disposition else None
    holistic_verification = Path(args.holistic_verification) if args.holistic_verification else None
    try:
        event = _read(paths["event"])
        errors = _schema_errors(EVENT_SCHEMA, event, "event")
        if isinstance(event.get("payload"), dict):
            errors.extend(_schema_errors(RECEIPT_SCHEMA, event["payload"], "payload"))
        checks = {
            "schemas_valid": not errors,
            "source_bindings_valid": False,
            "event_rebuilt_exactly": False,
            "event_hashes_valid": False,
            "state_projection_valid": False,
        }
        expected = build_event(
            paths["request"], paths["checkpoint"], paths["checkpoint_verification"],
            holistic_history, holistic_review, holistic_disposition, holistic_verification,
        )
        checks["event_rebuilt_exactly"] = _canonical(expected) == _canonical(event)
        if not checks["event_rebuilt_exactly"]:
            errors.append("event does not match deterministic reconstruction")
        payload = event.get("payload", {})
        checks["source_bindings_valid"] = (
            isinstance(payload, dict)
            and payload.get("source_bindings") == expected["payload"]["source_bindings"]
        )
        checks["event_hashes_valid"] = (
            event.get("payload_sha256") == _digest(_canonical(payload))
            and event.get("event_sha256")
            == _digest(_canonical({key: value for key, value in event.items() if key != "event_sha256"}))
        )
        applied = payload.get("transition_applied")
        checks["state_projection_valid"] = (
            payload.get("decision") == ("pass" if applied else "block")
            and payload.get("state_after")
            == (payload.get("requested_state") if applied else payload.get("state_before"))
        )
        for name, valid in checks.items():
            if not valid and not any(name in error for error in errors):
                errors.append(f"{name} failed")
        report = {
            "schema_version": "0.1.0",
            "artifact_kind": "global-progress-transition-verification",
            "valid": all(checks.values()) and not errors,
            "transition_id": payload.get("transition_id") if isinstance(payload.get("transition_id"), str) else None,
            "event_sha256": hashlib.sha256(paths["event"].read_bytes()).hexdigest(),
            "checks": checks,
            "errors": errors,
        }
        generated = _schema_errors(REPORT_SCHEMA, report, "report")
        if generated:
            raise ValueError("; ".join(generated))
        output = Path(args.output)
        if output.exists():
            raise ValueError(f"refusing to overwrite output: {output}")
        descriptor = os.open(output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
            handle.flush()
            os.fsync(handle.fileno())
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": report["valid"], "checks": report["checks"]}))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
