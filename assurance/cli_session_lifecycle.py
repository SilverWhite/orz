from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker

from .errors import AssuranceError


RUNTIME_ROOT = Path(__file__).resolve().parents[1] / "runtime"
PAYLOAD_SCHEMA_NAME = "cli-session-lifecycle-event-v0.2.schema.json"
OBSERVATION_SCHEMA = RUNTIME_ROOT / "cli-session-lifecycle-observation-v0.2.schema.json"
PAYLOAD_SCHEMA = RUNTIME_ROOT / PAYLOAD_SCHEMA_NAME
EVENT_SCHEMA = RUNTIME_ROOT / "run-event-v0.1.schema.json"
EVENT_TYPE = {
    "session_started": "run_started",
    "turn_started": "model_request",
    "turn_completed": "model_output",
    "session_completed": "run_finished",
    "session_failed": "run_failed",
    "session_cancelled": "run_cancelled",
}


class CliLifecycleError(RuntimeError):
    pass


def canonical_bytes(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def digest(value: object) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _validate(value: dict[str, Any], schema_path: Path, label: str) -> None:
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    errors = sorted(
        Draft202012Validator(
            schema,
            format_checker=FormatChecker(),
        ).iter_errors(value),
        key=lambda item: list(item.absolute_path),
    )
    if errors:
        detail = "; ".join(
            f"{'/'.join(map(str, item.absolute_path)) or '<root>'}: {item.message}"
            for item in errors[:5]
        )
        raise CliLifecycleError(f"{label} schema invalid: {detail}")


def validate_observation(observation: dict[str, Any]) -> None:
    _validate(observation, OBSERVATION_SCHEMA, "lifecycle observation")
    kind = observation["event_kind"]
    turn_id = observation["turn_id"]
    turn_status = observation["turn_status"]
    outcome = observation["outcome"]
    error_sha256 = observation["error_sha256"]
    if kind == "turn_started":
        if (
            turn_id is None
            or turn_status is not None
            or outcome is not None
            or error_sha256 is not None
        ):
            raise CliLifecycleError(
                "turn_started requires turn_id and forbids turn_status, "
                "outcome, and error_sha256"
            )
    elif kind == "turn_completed":
        if turn_id is None or turn_status is None or outcome is not None:
            raise CliLifecycleError(
                "turn_completed requires turn_id and turn_status and forbids outcome"
            )
        if error_sha256 is not None and turn_status != "failed":
            raise CliLifecycleError(
                "turn error_sha256 is only allowed when turn_status is failed"
            )
    elif kind == "session_started":
        if (
            turn_id is not None
            or turn_status is not None
            or outcome is not None
            or error_sha256 is not None
        ):
            raise CliLifecycleError(
                "session_started forbids turn_id, turn_status, outcome, "
                "and error_sha256"
            )
    else:
        if turn_id is not None or turn_status is not None or outcome is None:
            raise CliLifecycleError(
                "session terminal observation forbids turn_id and turn_status "
                "and requires outcome"
            )
        if error_sha256 is not None and kind != "session_failed":
            raise CliLifecycleError(
                "session error_sha256 is only allowed for session_failed"
            )


def _payload_errors(payload: dict[str, Any]) -> list[str]:
    try:
        _validate(payload, PAYLOAD_SCHEMA, "lifecycle event payload")
    except CliLifecycleError as exc:
        return [str(exc)]
    return []


def reduce_cli_lifecycle(
    events: list[dict[str, Any]],
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
) -> dict[str, Any]:
    errors: list[str] = []
    state = "new"
    active_turn_id: str | None = None
    identity: tuple[str, str, str, str, str] | None = None
    source_count = 0
    last_source_record_sequence: int | None = None
    last_observation_id: str | None = None
    last_event_sha256: str | None = None

    for event in events:
        if event.get("payload_schema") != PAYLOAD_SCHEMA_NAME:
            continue
        sequence = event.get("sequence")
        payload = event.get("payload")
        if not isinstance(payload, dict):
            errors.append(f"lifecycle payload is not an object at sequence {sequence}")
            continue
        payload_errors = _payload_errors(payload)
        if payload_errors:
            errors.extend(
                f"sequence {sequence}: {detail}" for detail in payload_errors
            )
            continue
        expected_type = EVENT_TYPE[payload["source_event_kind"]]
        if event.get("event_type") != expected_type:
            errors.append(f"lifecycle event type mismatch at sequence {sequence}")
        if (
            event.get("run_id") != expected_run_id
            or event.get("run_manifest_sha256") != expected_manifest_sha256
        ):
            errors.append(f"lifecycle run binding mismatch at sequence {sequence}")
        current_identity = (
            payload["adapter_id"],
            payload["runtime_family"],
            payload["runtime_version"],
            payload["session_id"],
            payload["source_stream_id"],
        )
        if identity is None:
            identity = current_identity
        elif identity != current_identity:
            errors.append(f"lifecycle source identity changed at sequence {sequence}")
        if payload["source_sequence"] != source_count:
            errors.append(f"lifecycle source sequence mismatch at sequence {sequence}")
        source_record_sequence = payload["source_record_sequence"]
        if (
            last_source_record_sequence is not None
            and source_record_sequence <= last_source_record_sequence
        ):
            errors.append(
                f"source record sequence is not strictly increasing at sequence {sequence}"
            )

        kind = payload["source_event_kind"]
        if kind == "session_started":
            if state != "new":
                errors.append(f"duplicate/out-of-order session start at sequence {sequence}")
            else:
                state = "active"
        elif kind == "turn_started":
            if state != "active":
                errors.append(f"turn start outside active state at sequence {sequence}")
            else:
                state = "in_turn"
                active_turn_id = payload["turn_id"]
        elif kind == "turn_completed":
            if state != "in_turn" or active_turn_id != payload["turn_id"]:
                errors.append(f"turn completion mismatch at sequence {sequence}")
            else:
                state = "active"
                active_turn_id = None
        elif kind == "session_completed":
            if state != "active":
                errors.append(f"session completion outside active state at sequence {sequence}")
            else:
                state = "terminal"
        elif kind in {"session_failed", "session_cancelled"}:
            if state not in {"active", "in_turn"}:
                errors.append(f"session terminal outside live state at sequence {sequence}")
            else:
                state = "terminal"
                active_turn_id = None
        source_count += 1
        last_source_record_sequence = source_record_sequence
        last_observation_id = payload["observation_id"]
        last_event_sha256 = event["event_sha256"]

    return {
        "valid": not errors,
        "state": state,
        "active_turn_id": active_turn_id,
        "source_event_count": source_count,
        "last_source_record_sequence": last_source_record_sequence,
        "identity": identity,
        "last_observation_id": last_observation_id,
        "last_event_sha256": last_event_sha256,
        "errors": errors,
    }


def expected_next_state(
    reduction: dict[str, Any],
    observation: dict[str, Any],
) -> tuple[str, str | None]:
    state = reduction["state"]
    active_turn = reduction["active_turn_id"]
    kind = observation["event_kind"]
    if kind == "session_started" and state == "new":
        return "active", None
    if kind == "turn_started" and state == "active":
        return "in_turn", observation["turn_id"]
    if (
        kind == "turn_completed"
        and state == "in_turn"
        and active_turn == observation["turn_id"]
    ):
        return "active", None
    if kind == "session_completed" and state == "active":
        return "terminal", None
    if kind in {"session_failed", "session_cancelled"} and state in {
        "active",
        "in_turn",
    }:
        return "terminal", None
    raise CliLifecycleError(
        f"observation {kind} is not allowed from lifecycle state {state}"
    )


def build_run_event(
    observation: dict[str, Any],
    *,
    sequence: int,
    previous_event_sha256: str | None,
) -> dict[str, Any]:
    observation_sha256 = digest(observation)
    payload = {
        "schema_version": "0.2.0",
        "observation_id": observation["observation_id"],
        "observation_sha256": observation_sha256,
        "adapter_id": observation["adapter_id"],
        "runtime_family": observation["runtime_family"],
        "runtime_version": observation["runtime_version"],
        "session_id": observation["session_id"],
        "source_stream_id": observation["source_stream_id"],
        "source_sequence": observation["source_sequence"],
        "source_record_sequence": observation["source_record_sequence"],
        "source_event_kind": observation["event_kind"],
        "turn_id": observation["turn_id"],
        "turn_status": observation["turn_status"],
        "outcome": observation["outcome"],
        "error_sha256": observation["error_sha256"],
        "source_record_sha256": observation["source_record_sha256"],
    }
    _validate(payload, PAYLOAD_SCHEMA, "lifecycle event payload")
    event = {
        "schema_version": "0.1.0-draft",
        "run_id": observation["run_id"],
        "event_id": f"EVT-CLI-{observation_sha256[:24].upper()}",
        "sequence": sequence,
        "timestamp": observation["timestamp"],
        "event_type": EVENT_TYPE[observation["event_kind"]],
        "run_manifest_sha256": observation["run_manifest_sha256"],
        "previous_event_sha256": previous_event_sha256,
        "payload_schema": PAYLOAD_SCHEMA_NAME,
        "payload": payload,
        "payload_sha256": digest(payload),
        "redaction": "metadata_only",
        "event_sha256": "",
    }
    event["event_sha256"] = digest(
        {key: value for key, value in event.items() if key != "event_sha256"}
    )
    _validate(event, EVENT_SCHEMA, "lifecycle run event")
    return event
