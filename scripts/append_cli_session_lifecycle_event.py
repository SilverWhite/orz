#!/usr/bin/env python3
"""Append one normalized CLI lifecycle observation to the canonical run journal."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from append_global_progress_transition_event import AppendError, _canonical, _replay
from prototype.fep_agent_proto.cli_session_lifecycle import (
    CliLifecycleError,
    PAYLOAD_SCHEMA_NAME,
    build_run_event,
    digest,
    expected_next_state,
    reduce_cli_lifecycle,
    validate_observation,
)
from prototype.fep_agent_proto.global_progress_state import (
    reduce_global_progress_events,
)
from prototype.fep_agent_proto.io_utils import atomic_write_json, sha256_file
from prototype.fep_agent_proto.journal_lock import (
    JournalLockError,
    exclusive_journal_lock,
)
from prototype.fep_agent_proto.schema import validate_instance


RECEIPT_SCHEMA = (
    ROOT / "runtime" / "cli-session-lifecycle-adapter-receipt-v0.2.schema.json"
)
LIMITATIONS = [
    "The adapter trusts a normalized observation and does not parse a vendor-private event stream.",
    "Source-record digest binding does not prove the normalizer interpreted the source correctly.",
    "A model_output event closes a turn; its turn_status, not the event type, determines success, interruption, or failure.",
    "Lifecycle mapping proves journal mechanics, not model or scientific correctness.",
]
TERMINAL_EVENTS = {"run_finished", "run_failed", "run_cancelled", "run_invalidated"}


def _read_object(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CliLifecycleError(f"cannot read observation: {exc}") from exc
    if not isinstance(value, dict):
        raise CliLifecycleError("observation must be a JSON object")
    return value


def _base_receipt(observation: dict[str, Any] | None, journal: Path) -> dict[str, Any]:
    observation_id = observation.get("observation_id") if observation else None
    run_id = observation.get("run_id") if observation else None
    session_id = observation.get("session_id") if observation else None
    turn_status = observation.get("turn_status") if observation else None
    error_sha256 = observation.get("error_sha256") if observation else None
    try:
        observation_sha256 = digest(observation) if observation else None
    except (TypeError, ValueError):
        observation_sha256 = None
    return {
        "schema_version": "0.2.0",
        "artifact_kind": "cli-session-lifecycle-adapter-receipt",
        "adapter_status": "rejected",
        "valid": False,
        "observation_id": (
            observation_id
            if isinstance(observation_id, str)
            and re.fullmatch(r"CLIOBS-[A-Z0-9._-]+", observation_id)
            else None
        ),
        "observation_sha256": observation_sha256,
        "run_id": (
            run_id
            if isinstance(run_id, str)
            and re.fullmatch(r"RUN-[A-Z0-9._-]+", run_id)
            else None
        ),
        "session_id": session_id if isinstance(session_id, str) and session_id else None,
        "lifecycle_state_before": None,
        "lifecycle_state_after": None,
        "active_turn_id": None,
        "turn_status": (
            turn_status
            if turn_status in {"completed", "interrupted", "failed"}
            else None
        ),
        "error_sha256": (
            error_sha256
            if isinstance(error_sha256, str)
            and re.fullmatch(r"[a-f0-9]{64}", error_sha256)
            else None
        ),
        "canonical_event_type": None,
        "event_sha256": None,
        "journal_sha256": sha256_file(journal) if journal.is_file() else None,
        "journal_event_count": None,
        "errors": [],
        "limitations": LIMITATIONS,
    }


def append_observation(
    observation: dict[str, Any],
    journal: Path,
    *,
    lock_timeout_seconds: float = 5.0,
) -> tuple[dict[str, Any], dict[str, Any]]:
    validate_observation(observation)
    observation_sha256 = digest(observation)
    with exclusive_journal_lock(journal, timeout_seconds=lock_timeout_seconds):
        existing = _replay(journal)
        if existing and (
            existing[0]["run_id"] != observation["run_id"]
            or existing[0]["run_manifest_sha256"]
            != observation["run_manifest_sha256"]
        ):
            raise CliLifecycleError("observation run binding differs from journal")
        reduction = reduce_cli_lifecycle(
            existing,
            expected_run_id=observation["run_id"],
            expected_manifest_sha256=observation["run_manifest_sha256"],
        )
        if not reduction["valid"]:
            raise CliLifecycleError(
                "journal CLI lifecycle is invalid: "
                + "; ".join(reduction["errors"][:3])
            )
        same_id = [
            event
            for event in existing
            if event.get("payload_schema") == PAYLOAD_SCHEMA_NAME
            and event.get("payload", {}).get("observation_id")
            == observation["observation_id"]
        ]
        if same_id:
            if (
                len(same_id) == 1
                and same_id[0]["payload"]["observation_sha256"]
                == observation_sha256
                and same_id[0]["event_sha256"] == existing[-1]["event_sha256"]
            ):
                return same_id[0], {
                    **reduction,
                    "append_status": "already_recorded",
                    "state_before": reduction["state"],
                }
            raise CliLifecycleError(
                "observation ID conflicts or is no longer at the journal tail"
            )
        if existing and existing[-1]["event_type"] in TERMINAL_EVENTS:
            raise CliLifecycleError("refusing to append after terminal run event")
        if observation["source_sequence"] != reduction["source_event_count"]:
            raise CliLifecycleError(
                "observation source_sequence does not extend lifecycle sequence"
            )
        last_source_record_sequence = reduction["last_source_record_sequence"]
        if (
            last_source_record_sequence is not None
            and observation["source_record_sequence"]
            <= last_source_record_sequence
        ):
            raise CliLifecycleError(
                "observation source_record_sequence must be strictly increasing"
            )
        identity = reduction["identity"]
        observed_identity = (
            observation["adapter_id"],
            observation["runtime_family"],
            observation["runtime_version"],
            observation["session_id"],
            observation["source_stream_id"],
        )
        if identity is not None and identity != observed_identity:
            raise CliLifecycleError("observation source identity differs from journal")
        state_before = reduction["state"]
        state_after, active_turn_after = expected_next_state(reduction, observation)

        if observation["event_kind"] == "session_started":
            if any(event["event_type"] == "run_started" for event in existing):
                raise CliLifecycleError(
                    "journal already contains a run_started state anchor"
                )
        else:
            progress = reduce_global_progress_events(
                existing,
                expected_run_id=observation["run_id"],
                expected_manifest_sha256=observation["run_manifest_sha256"],
            )
            if not progress["valid"]:
                raise CliLifecycleError(
                    "journal Global Progress state is invalid: "
                    + "; ".join(progress["errors"][:3])
                )
            if (
                progress["current_state"] == "completed"
                and observation["event_kind"] == "turn_started"
            ):
                raise CliLifecycleError(
                    "cannot start a CLI turn after journal-derived task completion"
                )

        previous = existing[-1]["event_sha256"] if existing else None
        event = build_run_event(
            observation,
            sequence=len(existing),
            previous_event_sha256=previous,
        )
        projected_progress = reduce_global_progress_events(
            [*existing, event],
            expected_run_id=observation["run_id"],
            expected_manifest_sha256=observation["run_manifest_sha256"],
        )
        if not projected_progress["valid"]:
            raise CliLifecycleError(
                "candidate would make Global Progress state invalid: "
                + "; ".join(projected_progress["errors"][:3])
            )
        journal.parent.mkdir(parents=True, exist_ok=True)
        with journal.open("ab") as handle:
            handle.write(_canonical(event) + b"\n")
            handle.flush()
            os.fsync(handle.fileno())
        replayed = _replay(journal)
        reduced = reduce_cli_lifecycle(
            replayed,
            expected_run_id=observation["run_id"],
            expected_manifest_sha256=observation["run_manifest_sha256"],
        )
        if (
            not reduced["valid"]
            or reduced["state"] != state_after
            or reduced["active_turn_id"] != active_turn_after
        ):
            raise CliLifecycleError("lifecycle reduction failed after append")
        return event, {
            **reduced,
            "append_status": "appended",
            "state_before": state_before,
        }


def run_adapter(
    observation_path: Path,
    journal: Path,
    output: Path,
    *,
    lock_timeout_seconds: float = 5.0,
) -> dict[str, Any]:
    observation: dict[str, Any] | None = None
    try:
        observation = _read_object(observation_path)
    except CliLifecycleError as exc:
        receipt = _base_receipt(None, journal)
        receipt["errors"] = [str(exc)]
        validate_instance(receipt, RECEIPT_SCHEMA, label="CLI lifecycle adapter receipt")
        atomic_write_json(output, receipt)
        return receipt
    receipt = _base_receipt(observation, journal)
    try:
        validate_observation(observation)
    except CliLifecycleError as exc:
        receipt["errors"] = [str(exc)]
        validate_instance(receipt, RECEIPT_SCHEMA, label="CLI lifecycle adapter receipt")
        atomic_write_json(output, receipt)
        return receipt
    try:
        event, reduction = append_observation(
            observation,
            journal,
            lock_timeout_seconds=lock_timeout_seconds,
        )
    except (CliLifecycleError, AppendError, JournalLockError) as exc:
        receipt["adapter_status"] = "conflict"
        receipt["errors"] = [str(exc)]
        receipt["journal_sha256"] = sha256_file(journal) if journal.is_file() else None
        if journal.is_file():
            try:
                receipt["journal_event_count"] = len(_replay(journal))
            except AppendError:
                receipt["journal_event_count"] = None
        validate_instance(receipt, RECEIPT_SCHEMA, label="CLI lifecycle adapter receipt")
        atomic_write_json(output, receipt)
        return receipt
    receipt.update(
        {
            "adapter_status": reduction["append_status"],
            "valid": True,
            "lifecycle_state_before": reduction["state_before"],
            "lifecycle_state_after": reduction["state"],
            "active_turn_id": reduction["active_turn_id"],
            "turn_status": observation["turn_status"],
            "error_sha256": observation["error_sha256"],
            "canonical_event_type": event["event_type"],
            "event_sha256": event["event_sha256"],
            "journal_sha256": sha256_file(journal),
            "journal_event_count": len(_replay(journal)),
            "errors": [],
        }
    )
    validate_instance(receipt, RECEIPT_SCHEMA, label="CLI lifecycle adapter receipt")
    atomic_write_json(output, receipt)
    return receipt


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--observation", required=True)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    receipt = run_adapter(
        Path(args.observation),
        Path(args.journal),
        Path(args.output),
        lock_timeout_seconds=args.lock_timeout_seconds,
    )
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True))
    return 0 if receipt["valid"] else 3


if __name__ == "__main__":
    raise SystemExit(main())
