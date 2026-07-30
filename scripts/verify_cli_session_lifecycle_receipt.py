#!/usr/bin/env python3
"""Independently verify a successful CLI session lifecycle adapter receipt."""

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

from append_global_progress_transition_event import AppendError, _replay
from assurance.cli_session_lifecycle import (
    CliLifecycleError,
    EVENT_TYPE,
    PAYLOAD_SCHEMA_NAME,
    digest,
    reduce_cli_lifecycle,
    validate_observation,
)
from assurance.io_utils import (
    atomic_write_json,
    sha256_bytes,
    sha256_file,
)
from assurance.journal_lock import (
    JournalLockError,
    exclusive_journal_lock,
)
from assurance.schema import validate_instance


RECEIPT_SCHEMA = (
    ROOT / "runtime" / "cli-session-lifecycle-adapter-receipt-v0.2.schema.json"
)
VERIFICATION_SCHEMA = (
    ROOT
    / "runtime"
    / "cli-session-lifecycle-adapter-verification-v0.2.schema.json"
)
STATUSES = {"appended", "already_recorded", "conflict", "rejected"}


def _read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CliLifecycleError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise CliLifecycleError(f"expected JSON object: {path}")
    return value


def _schema_valid(value: dict[str, Any], schema_path: Path) -> bool:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    return not list(
        Draft202012Validator(
            schema,
            format_checker=FormatChecker(),
        ).iter_errors(value)
    )


def verify_receipt(
    observation_path: Path,
    journal: Path,
    receipt_path: Path,
    *,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    observation = _read_object(observation_path)
    receipt = _read_object(receipt_path)
    checks = {
        "receipt_schema_valid": _schema_valid(receipt, RECEIPT_SCHEMA),
        "observation_valid": False,
        "observation_digest_matches": False,
        "journal_valid": False,
        "journal_snapshot_matches": False,
        "event_presence_verified": False,
        "lifecycle_projection_verified": False,
        "adapter_status_consistent": False,
    }
    try:
        validate_observation(observation)
    except CliLifecycleError:
        pass
    else:
        checks["observation_valid"] = True
    observation_sha256: str | None = None
    if checks["observation_valid"]:
        observation_sha256 = digest(observation)
        checks["observation_digest_matches"] = (
            receipt.get("observation_sha256") == observation_sha256
            and receipt.get("observation_id") == observation["observation_id"]
        )

    events: list[dict[str, Any]] = []
    raw: bytes | None = None
    reduction: dict[str, Any] | None = None
    journal_error: str | None = None
    if (
        journal.is_file()
        and checks["observation_valid"]
        and isinstance(observation.get("run_id"), str)
        and isinstance(observation.get("run_manifest_sha256"), str)
    ):
        try:
            with exclusive_journal_lock(
                journal,
                timeout_seconds=lock_timeout_seconds,
            ):
                raw = journal.read_bytes()
                events = _replay(journal)
                reduction = reduce_cli_lifecycle(
                    events,
                    expected_run_id=observation["run_id"],
                    expected_manifest_sha256=observation[
                        "run_manifest_sha256"
                    ],
                )
        except (OSError, AppendError, JournalLockError) as exc:
            journal_error = str(exc)
    checks["journal_valid"] = reduction is not None and reduction["valid"]
    journal_sha256 = sha256_bytes(raw) if raw is not None else None
    checks["journal_snapshot_matches"] = (
        receipt.get("journal_sha256") == journal_sha256
        and receipt.get("journal_event_count")
        == (len(events) if raw is not None else None)
    )

    matches: list[dict[str, Any]] = []
    if observation_sha256 is not None:
        matches = [
            event
            for event in events
            if event.get("payload_schema") == PAYLOAD_SCHEMA_NAME
            and event.get("payload", {}).get("observation_id")
            == observation["observation_id"]
            and event.get("payload", {}).get("observation_sha256")
            == observation_sha256
        ]
    event = matches[0] if len(matches) == 1 else None
    event_is_tail = bool(
        event is not None
        and events
        and event["event_sha256"] == events[-1]["event_sha256"]
    )
    checks["event_presence_verified"] = bool(
        event is not None
        and event_is_tail
        and event["event_type"] == EVENT_TYPE[observation["event_kind"]]
        and event["run_id"] == observation["run_id"]
        and event["run_manifest_sha256"]
        == observation["run_manifest_sha256"]
    ) if checks["observation_valid"] else False

    status = receipt.get("adapter_status")
    if (
        event is not None
        and reduction is not None
        and reduction["valid"]
        and status in {"appended", "already_recorded"}
    ):
        prefix = events[: event["sequence"]]
        prefix_reduction = reduce_cli_lifecycle(
            prefix,
            expected_run_id=observation["run_id"],
            expected_manifest_sha256=observation["run_manifest_sha256"],
        )
        expected_before = (
            reduction["state"]
            if status == "already_recorded"
            else prefix_reduction["state"]
        )
        checks["lifecycle_projection_verified"] = (
            prefix_reduction["valid"]
            and receipt.get("lifecycle_state_before") == expected_before
            and receipt.get("lifecycle_state_after") == reduction["state"]
            and receipt.get("active_turn_id") == reduction["active_turn_id"]
            and receipt.get("turn_status") == observation["turn_status"]
            and receipt.get("error_sha256") == observation["error_sha256"]
            and receipt.get("canonical_event_type") == event["event_type"]
            and receipt.get("event_sha256") == event["event_sha256"]
        )
        checks["adapter_status_consistent"] = (
            receipt.get("valid") is True and event_is_tail
        )

    errors = [name for name, passed in checks.items() if not passed]
    if journal_error is not None:
        errors.append(f"journal inspection failed: {journal_error}")
    observation_id = observation.get("observation_id")
    result = {
        "schema_version": "0.2.0",
        "artifact_kind": "cli-session-lifecycle-adapter-verification",
        "valid": not errors,
        "adapter_status": status if status in STATUSES else None,
        "observation_id": (
            observation_id
            if isinstance(observation_id, str)
            and re.fullmatch(r"CLIOBS-[A-Z0-9._-]+", observation_id)
            else None
        ),
        "receipt_sha256": sha256_file(receipt_path),
        "journal_sha256": journal_sha256,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "The final journal cannot prove whether the adapter appended the event or found an identical tail event.",
            "Verification does not validate vendor-specific normalization semantics.",
            "A model_output event is not a success claim; verification preserves and checks turn_status separately.",
            "Mechanical consistency does not prove model or scientific correctness.",
        ],
    }
    validate_instance(
        result,
        VERIFICATION_SCHEMA,
        label="CLI lifecycle adapter verification",
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--observation", required=True)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    try:
        result = verify_receipt(
            Path(args.observation),
            Path(args.journal),
            Path(args.receipt),
            lock_timeout_seconds=args.lock_timeout_seconds,
        )
        atomic_write_json(Path(args.output), result)
    except CliLifecycleError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0 if result["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
