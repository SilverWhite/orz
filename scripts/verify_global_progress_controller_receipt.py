#!/usr/bin/env python3
"""Independently cross-check a disposable Global Progress controller receipt."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from append_global_progress_transition_event import _replay
from build_global_progress_transition_event import TransitionError, build_event
from assurance.errors import AssuranceError as PrototypeError
from assurance.global_progress_state import (
    reduce_global_progress_events,
)
from assurance.io_utils import (
    atomic_write_json,
    sha256_bytes,
    sha256_file,
)
from assurance.journal import TERMINAL_EVENTS
from assurance.journal_lock import (
    JournalLockError,
    exclusive_journal_lock,
)
from assurance.journal_recovery import _inspect_bytes
from assurance.schema import validate_instance


RECEIPT_SCHEMA = ROOT / "runtime" / "global-progress-controller-receipt-v0.1.schema.json"
VERIFICATION_SCHEMA = (
    ROOT / "runtime" / "global-progress-controller-verification-v0.1.schema.json"
)
CONTROLLER_STATUSES = {
    "transition_applied",
    "transition_blocked",
    "already_recorded",
    "recovery_required",
    "journal_unrecoverable",
    "conflict",
    "rejected",
}


def _read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise PrototypeError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise PrototypeError(f"expected JSON object: {path}")
    return value


def _safe_digest(path: Path | None) -> str | None:
    if path is None or not path.is_file():
        return None
    try:
        return sha256_file(path)
    except OSError:
        return None


def _source_bindings(
    *,
    request: Path,
    checkpoint: Path,
    checkpoint_verification: Path,
    holistic_history: Path | None,
    holistic_review: Path | None,
    holistic_disposition: Path | None,
    holistic_verification: Path | None,
) -> dict[str, str | None]:
    return {
        "request_sha256": _safe_digest(request),
        "checkpoint_sha256": _safe_digest(checkpoint),
        "checkpoint_verification_sha256": _safe_digest(checkpoint_verification),
        "holistic_history_sha256": _safe_digest(holistic_history),
        "holistic_review_sha256": _safe_digest(holistic_review),
        "holistic_disposition_sha256": _safe_digest(holistic_disposition),
        "holistic_verification_sha256": _safe_digest(holistic_verification),
    }


def _receipt_schema_valid(receipt: dict[str, Any]) -> bool:
    schema = json.loads(RECEIPT_SCHEMA.read_text(encoding="utf-8"))
    return not list(
        Draft202012Validator(
            schema,
            format_checker=FormatChecker(),
        ).iter_errors(receipt)
    )


def verify_controller_receipt(
    *,
    request: Path,
    checkpoint: Path,
    checkpoint_verification: Path,
    journal: Path,
    receipt_path: Path,
    holistic_history: Path | None = None,
    holistic_review: Path | None = None,
    holistic_disposition: Path | None = None,
    holistic_verification: Path | None = None,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    receipt = _read_object(receipt_path)
    request_value = _read_object(request)
    checks = {
        "receipt_schema_valid": _receipt_schema_valid(receipt),
        "source_bindings_valid": False,
        "candidate_outcome_verified": False,
        "journal_condition_verified": False,
        "journal_snapshot_matches": False,
        "event_presence_verified": False,
        "state_projection_verified": False,
        "controller_status_consistent": False,
    }
    expected_bindings = _source_bindings(
        request=request,
        checkpoint=checkpoint,
        checkpoint_verification=checkpoint_verification,
        holistic_history=holistic_history,
        holistic_review=holistic_review,
        holistic_disposition=holistic_disposition,
        holistic_verification=holistic_verification,
    )
    checks["source_bindings_valid"] = (
        receipt.get("source_bindings") == expected_bindings
    )

    event: dict[str, Any] | None = None
    build_error: str | None = None
    try:
        event = build_event(
            request,
            checkpoint,
            checkpoint_verification,
            holistic_history,
            holistic_review,
            holistic_disposition,
            holistic_verification,
        )
    except TransitionError as exc:
        build_error = str(exc)

    status = receipt.get("controller_status")
    checks["candidate_outcome_verified"] = (
        build_error is not None if status == "rejected" else event is not None
    )
    expected_run_id = (
        event["run_id"] if event is not None else request_value.get("run_id")
    )
    expected_manifest = (
        event["run_manifest_sha256"]
        if event is not None
        else request_value.get("run_manifest_sha256")
    )
    raw: bytes | None = None
    inspection: dict[str, Any] | None = None
    events: list[dict[str, Any]] = []
    state_reduction: dict[str, Any] | None = None
    journal_error: str | None = None
    if journal.is_file() and isinstance(expected_run_id, str) and isinstance(
        expected_manifest, str
    ):
        try:
            with exclusive_journal_lock(
                journal,
                timeout_seconds=lock_timeout_seconds,
            ):
                raw = journal.read_bytes()
                inspection_value = _inspect_bytes(
                    raw,
                    expected_run_id=expected_run_id,
                    expected_manifest_sha256=expected_manifest,
                )
                inspection = inspection_value.public()
                if inspection["status"] == "valid":
                    events = _replay(journal)
                    state_reduction = reduce_global_progress_events(
                        events,
                        expected_run_id=expected_run_id,
                        expected_manifest_sha256=expected_manifest,
                    )
                else:
                    events = inspection_value.events
        except (JournalLockError, OSError, PrototypeError) as exc:
            journal_error = str(exc)

    current_journal_sha = sha256_bytes(raw) if raw is not None else None
    observed_count = (
        len(events)
        if inspection is not None
        else (0 if not journal.is_file() else None)
    )
    checks["journal_snapshot_matches"] = (
        receipt.get("journal_sha256") == current_journal_sha
        and receipt.get("journal_event_count") == observed_count
    )

    if status == "recovery_required":
        checks["journal_condition_verified"] = (
            inspection is not None
            and inspection["status"] == "recoverable"
            and receipt.get("recovery_status") == "recoverable"
            and receipt.get("recovery_classification")
            == inspection["classification"]
        )
    elif status == "journal_unrecoverable":
        checks["journal_condition_verified"] = (
            inspection is not None
            and inspection["status"] == "unrecoverable"
            and receipt.get("recovery_status") == "unrecoverable"
            and receipt.get("recovery_classification")
            == inspection["classification"]
        )
    elif status == "rejected":
        checks["journal_condition_verified"] = True
    else:
        checks["journal_condition_verified"] = (
            inspection is not None
            and inspection["status"] == "valid"
            and state_reduction is not None
            and state_reduction["valid"]
        )

    exact_events: list[dict[str, Any]] = []
    same_id: list[dict[str, Any]] = []
    if event is not None:
        exact_events = [
            item for item in events if item.get("event_sha256") == event["event_sha256"]
        ]
        same_id = [
            item for item in events if item.get("event_id") == event["event_id"]
        ]
    event_is_tail = bool(
        event is not None
        and events
        and events[-1].get("event_sha256") == event["event_sha256"]
    )
    if status in {"transition_applied", "transition_blocked", "already_recorded"}:
        checks["event_presence_verified"] = len(exact_events) == 1 and event_is_tail
    elif status == "conflict" and event is not None and inspection is not None:
        terminal = bool(events and events[-1].get("event_type") in TERMINAL_EVENTS)
        previous = events[-1].get("event_sha256") if events else None
        can_extend = (
            event["sequence"] == len(events)
            and event["previous_event_sha256"] == previous
            and not terminal
            and not same_id
            and state_reduction is not None
            and state_reduction["valid"]
            and event["payload"]["state_before"]
            == state_reduction["current_state"]
            and (
                state_reduction["task_id"] is None
                or event["payload"]["task_id"] == state_reduction["task_id"]
            )
        )
        checks["event_presence_verified"] = not can_extend
    elif status in {"recovery_required", "journal_unrecoverable"} and event is not None:
        checks["event_presence_verified"] = not exact_events
    elif status == "rejected":
        checks["event_presence_verified"] = event is None

    if event is not None:
        payload = event["payload"]
        if status in {"transition_applied", "transition_blocked", "already_recorded"}:
            checks["state_projection_verified"] = (
                receipt.get("state_before") == payload["state_before"]
                and receipt.get("state_after") == payload["state_after"]
                and receipt.get("transition_applied")
                == payload["transition_applied"]
                and receipt.get("event_sha256") == event["event_sha256"]
                and state_reduction is not None
                and state_reduction["valid"]
                and state_reduction["current_state"] == payload["state_after"]
            )
        else:
            checks["state_projection_verified"] = (
                receipt.get("state_after") == receipt.get("state_before")
                and receipt.get("transition_applied") is False
                and receipt.get("event_appended") is False
            )
    else:
        checks["state_projection_verified"] = (
            receipt.get("transition_applied") is False
            and receipt.get("event_appended") is False
        )

    if event is not None:
        payload = event["payload"]
        if status == "transition_applied":
            checks["controller_status_consistent"] = (
                payload["transition_applied"] is True
                and receipt.get("event_appended") is True
            )
        elif status == "transition_blocked":
            checks["controller_status_consistent"] = (
                payload["transition_applied"] is False
                and receipt.get("event_appended") is True
            )
        elif status == "already_recorded":
            checks["controller_status_consistent"] = (
                receipt.get("event_appended") is False and event_is_tail
            )
        elif status == "recovery_required":
            checks["controller_status_consistent"] = checks[
                "journal_condition_verified"
            ]
        elif status == "journal_unrecoverable":
            checks["controller_status_consistent"] = checks[
                "journal_condition_verified"
            ]
        elif status == "conflict":
            checks["controller_status_consistent"] = checks[
                "event_presence_verified"
            ]
    elif status == "rejected":
        checks["controller_status_consistent"] = build_error is not None

    errors = [name for name, passed in checks.items() if not passed]
    if journal_error is not None:
        errors.append(f"journal inspection failed: {journal_error}")
    result = {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-controller-verification",
        "valid": not errors,
        "controller_status": status if status in CONTROLLER_STATUSES else None,
        "transition_id": (
            receipt.get("transition_id")
            if isinstance(receipt.get("transition_id"), str)
            and re.fullmatch(r"GPT-[A-Z0-9._-]+", receipt["transition_id"])
            else None
        ),
        "receipt_sha256": sha256_file(receipt_path),
        "journal_sha256": current_journal_sha,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Final journal state cannot independently prove whether the controller appended the event or found it already present.",
            "Verification proves mechanical consistency, not scientific correctness.",
        ],
    }
    validate_instance(
        result,
        VERIFICATION_SCHEMA,
        label="Global Progress controller verification",
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--checkpoint-verification", required=True)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--holistic-history")
    parser.add_argument("--holistic-review")
    parser.add_argument("--holistic-disposition")
    parser.add_argument("--holistic-verification")
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    try:
        result = verify_controller_receipt(
            request=Path(args.request),
            checkpoint=Path(args.checkpoint),
            checkpoint_verification=Path(args.checkpoint_verification),
            journal=Path(args.journal),
            receipt_path=Path(args.receipt),
            holistic_history=Path(args.holistic_history) if args.holistic_history else None,
            holistic_review=Path(args.holistic_review) if args.holistic_review else None,
            holistic_disposition=(
                Path(args.holistic_disposition) if args.holistic_disposition else None
            ),
            holistic_verification=(
                Path(args.holistic_verification) if args.holistic_verification else None
            ),
            lock_timeout_seconds=args.lock_timeout_seconds,
        )
        atomic_write_json(Path(args.output), result)
    except PrototypeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": result["valid"], "errors": result["errors"]}))
    return 0 if result["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
