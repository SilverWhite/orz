from __future__ import annotations

from collections import Counter
import hashlib
import json
from pathlib import Path
from typing import Any

from .cli_session_lifecycle import canonical_bytes, validate_observation
from .errors import AssuranceError
from .layout import RUNTIME_ROOT
from .schema import validate_instance


ADAPTER_ID = "CLIADAPTER-CODEX-APP-SERVER-V0.1"
NORMALIZER_ID = "CLINORMALIZER-CODEX-APP-SERVER-V0.1"
RUNTIME_FAMILY = "codex-app-server"
CAPTURE_RECORD_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-capture-record-v0.1.schema.json"
)
NORMALIZATION_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-lifecycle-normalization-v0.1.schema.json"
)


class CodexLifecycleNormalizationError(RuntimeError):
    pass


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def observations_bytes(observations: list[dict[str, Any]]) -> bytes:
    return b"".join(
        json.dumps(
            item,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
        for item in observations
    )


def _require_object(value: Any, label: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise CodexLifecycleNormalizationError(f"{label} must be an object")
    return value


def _require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise CodexLifecycleNormalizationError(f"{label} must be a non-empty string")
    return value


def _request_key(value: Any, label: str) -> str:
    if isinstance(value, bool) or not isinstance(value, (int, str)):
        raise CodexLifecycleNormalizationError(
            f"{label} must be a string or integer JSON-RPC id"
        )
    return json.dumps(value, ensure_ascii=False, sort_keys=True)


def _validate_capture_record(record: dict[str, Any], line_number: int) -> None:
    try:
        validate_instance(
            record,
            CAPTURE_RECORD_SCHEMA,
            label=f"capture record line {line_number}",
        )
    except AssuranceError as exc:
        raise CodexLifecycleNormalizationError(str(exc)) from exc


def _observation(
    *,
    record: dict[str, Any],
    raw_line: bytes,
    source_sequence: int,
    runtime_version: str,
    session_id: str,
    run_id: str,
    run_manifest_sha256: str,
    event_kind: str,
    turn_id: str | None = None,
    turn_status: str | None = None,
    outcome: str | None = None,
    error_sha256: str | None = None,
) -> dict[str, Any]:
    source_record_sha256 = sha256_bytes(raw_line)
    value = {
        "schema_version": "0.2.0",
        "artifact_kind": "cli-session-lifecycle-observation",
        "observation_id": (
            f"CLIOBS-CODEX-{source_sequence:06d}-"
            f"{source_record_sha256[:16].upper()}"
        ),
        "adapter_id": ADAPTER_ID,
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "session_id": session_id,
        "run_id": run_id,
        "run_manifest_sha256": run_manifest_sha256,
        "source_stream_id": record["source_stream_id"],
        "source_sequence": source_sequence,
        "source_record_sequence": record["source_record_sequence"],
        "timestamp": record["received_at"],
        "event_kind": event_kind,
        "turn_id": turn_id,
        "turn_status": turn_status,
        "outcome": outcome,
        "error_sha256": error_sha256,
        "source_record_sha256": source_record_sha256,
    }
    validate_observation(value)
    return value


def normalize_capture(
    capture_path: Path,
    *,
    runtime_version: str,
    run_id: str,
    run_manifest_sha256: str,
) -> dict[str, Any]:
    try:
        source_bytes = capture_path.read_bytes()
    except OSError as exc:
        raise CodexLifecycleNormalizationError(
            f"cannot read capture {capture_path}: {exc}"
        ) from exc
    if not source_bytes:
        raise CodexLifecycleNormalizationError("capture is empty")

    observations: list[dict[str, Any]] = []
    source_stream_id: str | None = None
    session_id: str | None = None
    active_turn_id: str | None = None
    lifecycle_state = "new"
    ignored_records = 0
    record_count = 0
    pending_turn_starts: dict[str, str] = {}
    turn_thread_bindings: dict[str, str] = {}

    for line_number, raw_line in enumerate(source_bytes.splitlines(), 1):
        if not raw_line.strip():
            raise CodexLifecycleNormalizationError(
                f"blank capture record at line {line_number}"
            )
        try:
            record_value = json.loads(raw_line.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise CodexLifecycleNormalizationError(
                f"invalid JSON capture record at line {line_number}: {exc}"
            ) from exc
        record = _require_object(record_value, f"capture record line {line_number}")
        _validate_capture_record(record, line_number)

        expected_record_sequence = record_count
        if record["source_record_sequence"] != expected_record_sequence:
            raise CodexLifecycleNormalizationError(
                "source_record_sequence must be contiguous from zero: "
                f"line {line_number} has {record['source_record_sequence']}, "
                f"expected {expected_record_sequence}"
            )
        if source_stream_id is None:
            source_stream_id = record["source_stream_id"]
        elif source_stream_id != record["source_stream_id"]:
            raise CodexLifecycleNormalizationError(
                f"source_stream_id changed at line {line_number}"
            )
        record_count += 1

        message = _require_object(record["message"], f"message at line {line_number}")
        method = message.get("method")
        params = message.get("params")
        direction = record["direction"]

        if direction == "client_to_server":
            if method == "turn/start":
                if lifecycle_state != "active" or session_id is None:
                    raise CodexLifecycleNormalizationError(
                        f"turn/start request outside an active thread at line {line_number}"
                    )
                request_id = _request_key(
                    message.get("id"), f"turn/start id at line {line_number}"
                )
                request_params = _require_object(
                    params, f"turn/start params at line {line_number}"
                )
                requested_thread_id = _require_string(
                    request_params.get("threadId"),
                    f"turn/start threadId at line {line_number}",
                )
                if requested_thread_id != session_id:
                    raise CodexLifecycleNormalizationError(
                        f"turn/start targets another thread at line {line_number}"
                    )
                pending_turn_starts[request_id] = requested_thread_id
            ignored_records += 1
            continue

        if "id" in message:
            response_id = _request_key(
                message.get("id"), f"response id at line {line_number}"
            )
            requested_thread_id = pending_turn_starts.pop(response_id, None)
            if requested_thread_id is not None and "result" in message:
                result = _require_object(
                    message.get("result"), f"turn/start result at line {line_number}"
                )
                turn = _require_object(
                    result.get("turn"), f"turn/start result.turn at line {line_number}"
                )
                turn_id = _require_string(
                    turn.get("id"), f"turn/start result.turn.id at line {line_number}"
                )
                turn_thread_bindings[turn_id] = requested_thread_id
            ignored_records += 1
            continue

        if not isinstance(method, str):
            ignored_records += 1
            continue
        params = _require_object(params, f"{method} params at line {line_number}")
        source_sequence = len(observations)

        if method == "thread/started":
            thread = _require_object(
                params.get("thread"), f"thread/started thread at line {line_number}"
            )
            observed_session_id = _require_string(
                thread.get("id"), f"thread/started thread.id at line {line_number}"
            )
            if lifecycle_state != "new" or session_id is not None:
                raise CodexLifecycleNormalizationError(
                    f"duplicate or out-of-order thread/started at line {line_number}"
                )
            session_id = observed_session_id
            observations.append(
                _observation(
                    record=record,
                    raw_line=raw_line,
                    source_sequence=source_sequence,
                    runtime_version=runtime_version,
                    session_id=session_id,
                    run_id=run_id,
                    run_manifest_sha256=run_manifest_sha256,
                    event_kind="session_started",
                )
            )
            lifecycle_state = "active"
            continue

        if method == "turn/started":
            if lifecycle_state != "active" or session_id is None:
                raise CodexLifecycleNormalizationError(
                    f"turn/started outside an active thread at line {line_number}"
                )
            turn = _require_object(
                params.get("turn"), f"turn/started turn at line {line_number}"
            )
            turn_id = _require_string(
                turn.get("id"), f"turn/started turn.id at line {line_number}"
            )
            if turn_thread_bindings.get(turn_id) != session_id:
                raise CodexLifecycleNormalizationError(
                    f"turn/started lacks a turn/start response binding at line {line_number}"
                )
            observations.append(
                _observation(
                    record=record,
                    raw_line=raw_line,
                    source_sequence=source_sequence,
                    runtime_version=runtime_version,
                    session_id=session_id,
                    run_id=run_id,
                    run_manifest_sha256=run_manifest_sha256,
                    event_kind="turn_started",
                    turn_id=turn_id,
                )
            )
            active_turn_id = turn_id
            lifecycle_state = "in_turn"
            continue

        if method == "turn/completed":
            if lifecycle_state != "in_turn" or session_id is None:
                raise CodexLifecycleNormalizationError(
                    f"turn/completed outside an active turn at line {line_number}"
                )
            turn = _require_object(
                params.get("turn"), f"turn/completed turn at line {line_number}"
            )
            turn_id = _require_string(
                turn.get("id"), f"turn/completed turn.id at line {line_number}"
            )
            if turn_id != active_turn_id:
                raise CodexLifecycleNormalizationError(
                    f"turn/completed id mismatch at line {line_number}"
                )
            turn_status = turn.get("status")
            if turn_status not in {"completed", "interrupted", "failed"}:
                raise CodexLifecycleNormalizationError(
                    f"unsupported turn terminal status at line {line_number}"
                )
            error = turn.get("error")
            error_sha256: str | None = None
            if turn_status == "failed":
                error_object = _require_object(
                    error, f"failed turn error at line {line_number}"
                )
                error_sha256 = sha256_bytes(canonical_bytes(error_object))
            elif error is not None:
                raise CodexLifecycleNormalizationError(
                    f"non-failed turn carries an error at line {line_number}"
                )
            observations.append(
                _observation(
                    record=record,
                    raw_line=raw_line,
                    source_sequence=source_sequence,
                    runtime_version=runtime_version,
                    session_id=session_id,
                    run_id=run_id,
                    run_manifest_sha256=run_manifest_sha256,
                    event_kind="turn_completed",
                    turn_id=turn_id,
                    turn_status=turn_status,
                    error_sha256=error_sha256,
                )
            )
            active_turn_id = None
            lifecycle_state = "active"
            continue

        if method == "thread/closed":
            if lifecycle_state != "active" or session_id is None:
                raise CodexLifecycleNormalizationError(
                    f"thread/closed outside an idle active thread at line {line_number}"
                )
            closed_thread_id = _require_string(
                params.get("threadId"), f"thread/closed threadId at line {line_number}"
            )
            if closed_thread_id != session_id:
                raise CodexLifecycleNormalizationError(
                    f"thread/closed identity mismatch at line {line_number}"
                )
            observations.append(
                _observation(
                    record=record,
                    raw_line=raw_line,
                    source_sequence=source_sequence,
                    runtime_version=runtime_version,
                    session_id=session_id,
                    run_id=run_id,
                    run_manifest_sha256=run_manifest_sha256,
                    event_kind="session_completed",
                    outcome="thread_closed",
                )
            )
            lifecycle_state = "terminal"
            continue

        ignored_records += 1

    if source_stream_id is None:
        raise CodexLifecycleNormalizationError("capture has no records")
    return {
        "source_bytes": source_bytes,
        "source_sha256": sha256_bytes(source_bytes),
        "source_stream_id": source_stream_id,
        "record_count": record_count,
        "ignored_record_count": ignored_records,
        "observations": observations,
        "observation_breakdown": dict(
            sorted(Counter(item["event_kind"] for item in observations).items())
        ),
        "session_id": session_id,
        "lifecycle_state": lifecycle_state,
        "active_turn_id": active_turn_id,
        "normalization_status": "complete" if lifecycle_state == "terminal" else "partial",
    }
