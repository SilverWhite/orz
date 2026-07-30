#!/usr/bin/env python3
"""Run one no-model Global Progress transition through a disposable controller."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from append_global_progress_transition_event import (
    AppendError,
    append_transition_event,
)
from build_global_progress_transition_event import TransitionError, build_event
from assurance.errors import AssuranceError as PrototypeError
from assurance.io_utils import (
    atomic_write_json,
    sha256_file,
)
from assurance.journal_lock import JournalLockError
from assurance.journal_recovery import inspect_journal_recovery
from assurance.schema import validate_instance


RECEIPT_SCHEMA = ROOT / "runtime" / "global-progress-controller-receipt-v0.1.schema.json"


def _matching_string(value: Any, pattern: str) -> str | None:
    return value if isinstance(value, str) and re.fullmatch(pattern, value) else None


def _safe_digest(path: Path | None) -> str | None:
    if path is None or not path.is_file():
        return None
    try:
        return sha256_file(path)
    except OSError:
        return None


def _identity(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


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


def _base_receipt(
    identity: dict[str, Any],
    *,
    source_bindings: dict[str, str | None],
    journal: Path,
) -> dict[str, Any]:
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "global-progress-controller-receipt",
        "controller_status": "rejected",
        "valid": False,
        "transition_id": _matching_string(
            identity.get("transition_id"), r"GPT-[A-Z0-9._-]+"
        ),
        "task_id": _matching_string(
            identity.get("task_id"), r"[A-Z][A-Z0-9._-]+"
        ),
        "run_id": _matching_string(identity.get("run_id"), r"RUN-[A-Z0-9._-]+"),
        "state_before": (
            identity.get("current_state")
            if identity.get("current_state") in {"executing", "reviewing"}
            else None
        ),
        "state_after": (
            identity.get("current_state")
            if identity.get("current_state") in {"executing", "reviewing"}
            else None
        ),
        "transition_applied": False,
        "event_appended": False,
        "event_sha256": None,
        "journal_sha256": _safe_digest(journal),
        "journal_event_count": None,
        "recovery_status": "not_inspected",
        "recovery_classification": None,
        "source_bindings": source_bindings,
        "errors": [],
        "limitations": [
            "Disposable no-model controller; it is not integrated into a production CLI runtime.",
            "Controller validity proves mechanical transition handling, not scientific correctness.",
        ],
    }


def _validate_and_write(output: Path, receipt: dict[str, Any]) -> None:
    validate_instance(receipt, RECEIPT_SCHEMA, label="Global Progress controller receipt")
    atomic_write_json(output, receipt)


def run_controller(
    *,
    request: Path,
    checkpoint: Path,
    checkpoint_verification: Path,
    journal: Path,
    output: Path,
    holistic_history: Path | None = None,
    holistic_review: Path | None = None,
    holistic_disposition: Path | None = None,
    holistic_verification: Path | None = None,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    identity = _identity(request)
    bindings = _source_bindings(
        request=request,
        checkpoint=checkpoint,
        checkpoint_verification=checkpoint_verification,
        holistic_history=holistic_history,
        holistic_review=holistic_review,
        holistic_disposition=holistic_disposition,
        holistic_verification=holistic_verification,
    )
    receipt = _base_receipt(identity, source_bindings=bindings, journal=journal)
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
        receipt["errors"] = [str(exc)]
        _validate_and_write(output, receipt)
        return receipt

    payload = event["payload"]
    receipt.update(
        {
            "transition_id": payload["transition_id"],
            "task_id": payload["task_id"],
            "run_id": event["run_id"],
            "state_before": payload["state_before"],
            "state_after": payload["state_before"],
        }
    )
    try:
        append_result = append_transition_event(
            journal,
            event,
            lock_timeout_seconds=lock_timeout_seconds,
            allow_idempotent=True,
        )
    except (AppendError, JournalLockError) as exc:
        receipt["controller_status"] = "conflict"
        receipt["errors"] = [str(exc)]
        if journal.is_file():
            try:
                inspection = inspect_journal_recovery(
                    journal,
                    expected_run_id=event["run_id"],
                    expected_manifest_sha256=event["run_manifest_sha256"],
                    lock_timeout_seconds=lock_timeout_seconds,
                )
            except PrototypeError as inspect_exc:
                receipt["errors"].append(str(inspect_exc))
            else:
                receipt["journal_event_count"] = inspection["prefix_event_count"]
                if inspection["status"] == "recoverable":
                    receipt["controller_status"] = "recovery_required"
                    receipt["recovery_status"] = "recoverable"
                    receipt["recovery_classification"] = inspection["classification"]
                elif inspection["status"] == "unrecoverable":
                    receipt["controller_status"] = "journal_unrecoverable"
                    receipt["recovery_status"] = "unrecoverable"
                    receipt["recovery_classification"] = inspection["classification"]
                else:
                    receipt["controller_status"] = "conflict"
                    receipt["recovery_status"] = "not_needed"
        else:
            receipt["controller_status"] = "conflict"
        receipt["journal_sha256"] = _safe_digest(journal)
        _validate_and_write(output, receipt)
        return receipt

    already_recorded = append_result["append_status"] == "already_recorded"
    if already_recorded:
        controller_status = "already_recorded"
    elif payload["transition_applied"]:
        controller_status = "transition_applied"
    else:
        controller_status = "transition_blocked"
    receipt.update(
        {
            "controller_status": controller_status,
            "valid": True,
            "state_after": append_result["current_state"],
            "transition_applied": payload["transition_applied"],
            "event_appended": not already_recorded,
            "event_sha256": event["event_sha256"],
            "journal_sha256": _safe_digest(journal),
            "journal_event_count": append_result["event_count"],
            "recovery_status": "not_needed",
            "recovery_classification": None,
            "errors": [],
        }
    )
    _validate_and_write(output, receipt)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True)
    parser.add_argument("--checkpoint", required=True)
    parser.add_argument("--checkpoint-verification", required=True)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--holistic-history")
    parser.add_argument("--holistic-review")
    parser.add_argument("--holistic-disposition")
    parser.add_argument("--holistic-verification")
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    try:
        receipt = run_controller(
            request=Path(args.request),
            checkpoint=Path(args.checkpoint),
            checkpoint_verification=Path(args.checkpoint_verification),
            journal=Path(args.journal),
            output=Path(args.output),
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
    except PrototypeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True))
    status = receipt["controller_status"]
    if status in {"transition_applied", "already_recorded"}:
        return 0
    if status == "transition_blocked":
        return 2
    return 3


if __name__ == "__main__":
    raise SystemExit(main())
