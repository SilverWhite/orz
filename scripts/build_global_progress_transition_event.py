#!/usr/bin/env python3
"""Build a disposable hash-chained run event that applies or blocks a GPS transition."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from verify_global_progress_holistic_review import (
    HolisticVerificationError,
    verify as independently_verify_holistic_review,
)


ROOT = Path(__file__).resolve().parents[1]
RUNTIME = ROOT / "runtime"
SCHEMAS = {
    "request": RUNTIME / "global-progress-transition-request-v0.1.schema.json",
    "receipt": RUNTIME / "global-progress-transition-receipt-v0.1.schema.json",
    "event": RUNTIME / "run-event-v0.1.schema.json",
    "checkpoint_verification": RUNTIME / "global-progress-checkpoint-verification-v0.1.schema.json",
    "holistic_review": RUNTIME / "global-progress-holistic-review-v0.1.schema.json",
    "holistic_verification": RUNTIME / "global-progress-holistic-verification-v0.1.schema.json",
}


class TransitionError(RuntimeError):
    pass


def _canonical(value: object) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")


def _digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _digest_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _read(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise TransitionError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise TransitionError(f"expected JSON object: {path}")
    return value


def _validate(kind: str, value: dict[str, Any]) -> None:
    schema = json.loads(SCHEMAS[kind].read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        detail = "; ".join(
            f"{'/'.join(map(str, item.absolute_path)) or '<root>'}: {item.message}"
            for item in errors[:8]
        )
        raise TransitionError(f"{kind} schema validation failed: {detail}")


def build_event(
    request_path: Path,
    checkpoint_path: Path,
    checkpoint_verification_path: Path,
    holistic_history_path: Path | None,
    holistic_review_path: Path | None,
    holistic_disposition_path: Path | None,
    holistic_verification_path: Path | None,
) -> dict[str, Any]:
    request = _read(request_path)
    checkpoint = _read(checkpoint_path)
    checkpoint_verification = _read(checkpoint_verification_path)
    _validate("request", request)
    _validate("checkpoint_verification", checkpoint_verification)

    if (
        request["checkpoint_file_sha256"] != _digest_file(checkpoint_path)
        or request["checkpoint_verification_sha256"]
        != _digest_file(checkpoint_verification_path)
        or request["checkpoint_id"] != checkpoint.get("checkpoint_id")
        or request["checkpoint_id"] != checkpoint_verification["checkpoint_id"]
        or request["task_id"] != checkpoint.get("task_id")
        or checkpoint_verification["checkpoint_sha256"] != _digest_file(checkpoint_path)
    ):
        raise TransitionError("checkpoint source binding mismatch")

    holistic_review = None
    holistic_verification = None
    if request["transition_kind"] == "task_completion":
        if any(
            path is None
            for path in (
                holistic_history_path,
                holistic_review_path,
                holistic_disposition_path,
                holistic_verification_path,
            )
        ):
            raise TransitionError(
                "task completion requires holistic history, review, disposition, and verification"
            )
        holistic_review = _read(holistic_review_path)
        holistic_verification = _read(holistic_verification_path)
        holistic_history = _read(holistic_history_path)
        _validate("holistic_review", holistic_review)
        _validate("holistic_verification", holistic_verification)
        try:
            recomputed = independently_verify_holistic_review(
                holistic_history_path,
                holistic_review_path,
                holistic_disposition_path,
            )
        except HolisticVerificationError as exc:
            raise TransitionError(f"holistic verification failed: {exc}") from exc
        if (
            request["holistic_assessment_id"] != holistic_review["assessment_id"]
            or request["holistic_assessment_id"] != holistic_verification["assessment_id"]
            or request["holistic_review_sha256"] != _digest_file(holistic_review_path)
            or request["holistic_verification_sha256"] != _digest_file(holistic_verification_path)
            or holistic_verification["assessment_sha256"] != _digest_file(holistic_review_path)
            or holistic_review["window"]["last_checkpoint_id"] != request["checkpoint_id"]
            or holistic_history["checkpoints"][-1] != checkpoint
            or _canonical(recomputed) != _canonical(holistic_verification)
        ):
            raise TransitionError("holistic source binding mismatch")
    elif (
        holistic_history_path is not None
        or holistic_review_path is not None
        or holistic_disposition_path is not None
        or holistic_verification_path is not None
        or request["holistic_assessment_id"] is not None
        or request["holistic_review_sha256"] is not None
        or request["holistic_verification_sha256"] is not None
    ):
        raise TransitionError("step-boundary transition must not carry holistic artifacts")

    allowed_pair = (
        request["transition_kind"] == "step_boundary"
        and request["current_state"] == "executing"
        and request["requested_state"] == "reviewing"
    ) or (
        request["transition_kind"] == "task_completion"
        and request["current_state"] == "reviewing"
        and request["requested_state"] == "completed"
    )
    codes: list[str]
    if not allowed_pair:
        codes = ["GPS-TRANSITION-NOT-ALLOWED"]
    elif not checkpoint_verification["checks"]["focus_window_within_bounds"]:
        codes = ["GPS-FOCUS-WINDOW-EXCEEDED"]
    elif not checkpoint_verification["valid"]:
        codes = ["GPS-CHECKPOINT-INVALID"]
    elif holistic_verification is not None and not holistic_verification["valid"]:
        codes = ["GPS-HOLISTIC-INVALID"]
    elif (
        holistic_review is not None
        and holistic_review["completion_gate"]["status"] != "eligible"
    ):
        codes = ["GPS-COMPLETION-INELIGIBLE"]
    else:
        codes = ["GPS-TRANSITION-PASS"]
    applied = codes == ["GPS-TRANSITION-PASS"]
    payload = {
        "schema_version": "0.1.0",
        "transition_id": request["transition_id"],
        "task_id": request["task_id"],
        "transition_kind": request["transition_kind"],
        "state_before": request["current_state"],
        "requested_state": request["requested_state"],
        "state_after": request["requested_state"] if applied else request["current_state"],
        "transition_applied": applied,
        "decision": "pass" if applied else "block",
        "control_codes": codes,
        "source_bindings": {
            "request_sha256": _digest_file(request_path),
            "checkpoint_id": request["checkpoint_id"],
            "checkpoint_file_sha256": request["checkpoint_file_sha256"],
            "checkpoint_verification_sha256": request["checkpoint_verification_sha256"],
            "holistic_assessment_id": request["holistic_assessment_id"],
            "holistic_review_sha256": request["holistic_review_sha256"],
            "holistic_verification_sha256": request["holistic_verification_sha256"],
        },
    }
    _validate("receipt", payload)
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": request["run_id"],
        "event_id": f"EVT-{request['transition_id'][4:]}",
        "sequence": request["sequence"],
        "timestamp": request["timestamp"],
        "event_type": "gate_decision",
        "run_manifest_sha256": request["run_manifest_sha256"],
        "previous_event_sha256": request["previous_event_sha256"],
        "payload_schema": "global-progress-transition-receipt-v0.1.schema.json",
        "payload": payload,
        "payload_sha256": _digest_bytes(_canonical(payload)),
        "redaction": "metadata_only",
        "event_sha256": "",
    }
    event["event_sha256"] = _digest_bytes(
        _canonical({key: value for key, value in event.items() if key != "event_sha256"})
    )
    _validate("event", event)
    return event


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise TransitionError(f"refusing to overwrite output: {path}") from exc
    with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--checkpoint-verification", required=True)
    parser.add_argument("--holistic-history")
    parser.add_argument("--holistic-review")
    parser.add_argument("--holistic-disposition")
    parser.add_argument("--holistic-verification")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        event = build_event(
            Path(args.request),
            Path(args.checkpoint),
            Path(args.checkpoint_verification),
            Path(args.holistic_history) if args.holistic_history else None,
            Path(args.holistic_review) if args.holistic_review else None,
            Path(args.holistic_disposition) if args.holistic_disposition else None,
            Path(args.holistic_verification) if args.holistic_verification else None,
        )
        _write_new(Path(args.output), event)
    except TransitionError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"transition_applied": event["payload"]["transition_applied"], "control_codes": event["payload"]["control_codes"]}))
    return 0 if event["payload"]["transition_applied"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
