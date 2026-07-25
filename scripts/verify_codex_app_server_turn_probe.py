#!/usr/bin/env python3
"""Independently verify a Codex app-server synthetic turn probe."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tomllib
from typing import Any
from urllib.parse import urlparse


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from prototype.fep_agent_proto.codex_app_server_lifecycle import (
    CAPTURE_RECORD_SCHEMA,
)
from prototype.fep_agent_proto.codex_app_server_turn_probe import (
    INITIALIZE_REQUEST_ID,
    PROBE_RECEIPT_SCHEMA,
    PROVIDER_ID,
    SYNTHETIC_INPUT,
    SYNTHETIC_MODEL,
    SYNTHETIC_OUTPUT,
    THREAD_START_REQUEST_ID,
    TURN_START_REQUEST_ID,
    _synthetic_sse,
)
from prototype.fep_agent_proto.errors import PrototypeError
from prototype.fep_agent_proto.io_utils import (
    atomic_write_json,
    load_json,
    sha256_bytes,
)
from prototype.fep_agent_proto.layout import RUNTIME_ROOT
from prototype.fep_agent_proto.schema import validate_instance


VERIFICATION_SCHEMA = (
    RUNTIME_ROOT
    / "codex-app-server-turn-probe-verification-v0.1.schema.json"
)

LIMITATIONS = [
    "The verifier reconstructs supplied artifact consistency but cannot recover messages the supervisor never observed.",
    "The loopback provider binding does not establish an operating-system network sandbox for unrelated app-server subsystems.",
    "Verification establishes lifecycle mechanics only, not model quality, task correctness, or scientific evidence.",
]


def _read_capture(path: Path) -> tuple[bytes, list[dict[str, Any]]]:
    data = path.read_bytes()
    records: list[dict[str, Any]] = []
    for line_number, raw_line in enumerate(data.splitlines(), 1):
        if not raw_line.strip():
            raise PrototypeError(f"blank capture record at line {line_number}")
        try:
            value = json.loads(raw_line.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise PrototypeError(
                f"invalid capture JSON at line {line_number}: {exc}"
            ) from exc
        validate_instance(
            value,
            CAPTURE_RECORD_SCHEMA,
            label=f"capture record line {line_number}",
        )
        records.append(value)
    return data, records


def _artifact_matches(
    artifact: Any, path: Path, data: bytes
) -> bool:
    return (
        isinstance(artifact, dict)
        and artifact.get("path") == str(path.resolve())
        and artifact.get("bytes") == len(data)
        and artifact.get("sha256") == sha256_bytes(data)
    )


def _message_index(
    records: list[dict[str, Any]],
    *,
    direction: str,
    request_id: int | None = None,
    method: str | None = None,
) -> int | None:
    for index, record in enumerate(records):
        if record["direction"] != direction:
            continue
        message = record["message"]
        if request_id is not None and message.get("id") != request_id:
            continue
        if method is not None and message.get("method") != method:
            continue
        return index
    return None


def _message_at(
    records: list[dict[str, Any]], index: int | None
) -> dict[str, Any] | None:
    if index is None:
        return None
    return records[index]["message"]


def verify(
    *,
    capture_path: Path,
    stderr_path: Path,
    receipt_path: Path,
) -> dict[str, Any]:
    errors: list[str] = []
    checks = {
        "receipt_schema_valid": False,
        "receipt_semantics_valid": False,
        "capture_artifact_matches": False,
        "capture_records_valid": False,
        "config_artifact_matches": False,
        "loopback_provider_config_valid": False,
        "provider_request_valid": False,
        "synthetic_exchange_bound": False,
        "ordered_lifecycle_replayed": False,
        "stderr_artifact_matches": False,
    }
    receipt: dict[str, Any] | None = None
    try:
        value = load_json(receipt_path)
        if not isinstance(value, dict):
            raise PrototypeError("turn probe receipt must be an object")
        validate_instance(value, PROBE_RECEIPT_SCHEMA, label="turn probe receipt")
        receipt = value
        checks["receipt_schema_valid"] = True
    except (OSError, PrototypeError, ValueError) as exc:
        errors.append(str(exc))

    capture_data = b""
    records: list[dict[str, Any]] = []
    try:
        capture_data, records = _read_capture(capture_path)
    except (OSError, PrototypeError, ValueError) as exc:
        errors.append(str(exc))

    if receipt is not None:
        checks["capture_artifact_matches"] = (
            _artifact_matches(
                receipt.get("capture_artifact"), capture_path, capture_data
            )
            and receipt.get("record_count") == len(records)
        )
        if not checks["capture_artifact_matches"]:
            errors.append("capture artifact projection mismatch")

        checks["capture_records_valid"] = (
            bool(records)
            and {
                record["source_stream_id"] for record in records
            }
            == {receipt["source_stream_id"]}
            and all(
                record["source_record_sequence"] == index
                for index, record in enumerate(records)
            )
        )
        if not checks["capture_records_valid"]:
            errors.append("capture record stream or sequence mismatch")

        config_path = (
            Path(receipt["isolated_state_directory"]) / "config.toml"
        )
        try:
            config_data = config_path.read_bytes()
            checks["config_artifact_matches"] = _artifact_matches(
                receipt.get("config_artifact"), config_path, config_data
            )
            config = tomllib.loads(config_data.decode("utf-8"))
            provider_config = config.get("model_providers", {}).get(PROVIDER_ID)
            base_url = (
                provider_config.get("base_url")
                if isinstance(provider_config, dict)
                else None
            )
            parsed = urlparse(base_url) if isinstance(base_url, str) else None
            checks["loopback_provider_config_valid"] = (
                config.get("model") == SYNTHETIC_MODEL
                and config.get("model_provider") == PROVIDER_ID
                and config.get("web_search") == "disabled"
                and isinstance(provider_config, dict)
                and parsed is not None
                and parsed.scheme == "http"
                and parsed.hostname == "127.0.0.1"
                and isinstance(parsed.port, int)
                and parsed.path == "/v1"
                and provider_config.get("wire_api") == "responses"
                and provider_config.get("request_max_retries") == 0
                and provider_config.get("stream_max_retries") == 0
                and provider_config.get("stream_idle_timeout_ms") == 5000
                and config.get("features", {}).get("apps") is False
                and config.get("features", {}).get("plugins") is False
            )
        except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError, ValueError) as exc:
            errors.append(f"cannot verify isolated config: {exc}")
        if not checks["config_artifact_matches"]:
            errors.append("config artifact projection mismatch")
        if not checks["loopback_provider_config_valid"]:
            errors.append("loopback provider config mismatch")

        provider = receipt.get("provider")
        request = provider.get("request") if isinstance(provider, dict) else None
        checks["provider_request_valid"] = (
            isinstance(provider, dict)
            and provider.get("request_count") == 1
            and provider.get("provider_id") == PROVIDER_ID
            and provider.get("model") == SYNTHETIC_MODEL
            and provider.get("base_url")
            == "http://127.0.0.1:<ephemeral>/v1"
            and isinstance(request, dict)
            and request.get("method") == "POST"
            and request.get("path") == "/v1/responses"
            and request.get("client_host") == "127.0.0.1"
            and request.get("model") == SYNTHETIC_MODEL
            and request.get("stream") is True
            and request.get("input_marker_present") is True
            and request.get("accepts_event_stream") is True
            and request.get("authorization_present") is False
        )
        if not checks["provider_request_valid"]:
            errors.append("loopback provider request mismatch")

        checks["synthetic_exchange_bound"] = (
            isinstance(provider, dict)
            and provider.get("synthetic_input_sha256")
            == sha256_bytes(SYNTHETIC_INPUT.encode("utf-8"))
            and provider.get("synthetic_output_sha256")
            == sha256_bytes(SYNTHETIC_OUTPUT.encode("utf-8"))
            and provider.get("sse_sha256") == sha256_bytes(_synthetic_sse())
        )
        if not checks["synthetic_exchange_bound"]:
            errors.append("synthetic exchange digest mismatch")

        initialize_request_index = _message_index(
            records,
            direction="client_to_server",
            request_id=INITIALIZE_REQUEST_ID,
            method="initialize",
        )
        initialize_response_index = _message_index(
            records,
            direction="server_to_client",
            request_id=INITIALIZE_REQUEST_ID,
        )
        initialized_index = _message_index(
            records,
            direction="client_to_server",
            method="initialized",
        )
        thread_request_index = _message_index(
            records,
            direction="client_to_server",
            request_id=THREAD_START_REQUEST_ID,
            method="thread/start",
        )
        thread_response_index = _message_index(
            records,
            direction="server_to_client",
            request_id=THREAD_START_REQUEST_ID,
        )
        thread_started_index = _message_index(
            records,
            direction="server_to_client",
            method="thread/started",
        )
        turn_request_index = _message_index(
            records,
            direction="client_to_server",
            request_id=TURN_START_REQUEST_ID,
            method="turn/start",
        )
        turn_response_index = _message_index(
            records,
            direction="server_to_client",
            request_id=TURN_START_REQUEST_ID,
        )
        turn_started_index = _message_index(
            records,
            direction="server_to_client",
            method="turn/started",
        )
        turn_completed_index = _message_index(
            records,
            direction="server_to_client",
            method="turn/completed",
        )

        thread_response = _message_at(records, thread_response_index)
        thread_notification = _message_at(records, thread_started_index)
        turn_request = _message_at(records, turn_request_index)
        turn_response = _message_at(records, turn_response_index)
        turn_started = _message_at(records, turn_started_index)
        turn_completed = _message_at(records, turn_completed_index)
        response_thread = (
            thread_response.get("result", {}).get("thread")
            if isinstance(thread_response, dict)
            else None
        )
        notification_thread = (
            thread_notification.get("params", {}).get("thread")
            if isinstance(thread_notification, dict)
            else None
        )
        response_turn = (
            turn_response.get("result", {}).get("turn")
            if isinstance(turn_response, dict)
            else None
        )
        started_turn = (
            turn_started.get("params", {}).get("turn")
            if isinstance(turn_started, dict)
            else None
        )
        completed_turn = (
            turn_completed.get("params", {}).get("turn")
            if isinstance(turn_completed, dict)
            else None
        )
        client_methods = [
            record["message"].get("method")
            for record in records
            if record["direction"] == "client_to_server"
        ]
        indices = [
            initialize_request_index,
            initialize_response_index,
            initialized_index,
            thread_request_index,
            turn_request_index,
            turn_response_index,
            turn_started_index,
            turn_completed_index,
        ]
        lifecycle = receipt.get("lifecycle")
        checks["ordered_lifecycle_replayed"] = (
            client_methods
            == ["initialize", "initialized", "thread/start", "turn/start"]
            and all(isinstance(index, int) for index in indices)
            and indices == sorted(indices)
            and isinstance(thread_started_index, int)
            and isinstance(thread_response_index, int)
            and thread_started_index > thread_request_index
            and thread_response_index > thread_request_index
            and isinstance(response_thread, dict)
            and isinstance(notification_thread, dict)
            and response_thread.get("id") == notification_thread.get("id")
            and response_thread.get("ephemeral") is True
            and isinstance(turn_request, dict)
            and turn_request.get("params", {}).get("threadId")
            == response_thread.get("id")
            and turn_request.get("params", {}).get("input")
            == [{"type": "text", "text": SYNTHETIC_INPUT}]
            and isinstance(response_turn, dict)
            and isinstance(started_turn, dict)
            and isinstance(completed_turn, dict)
            and response_turn.get("id") == started_turn.get("id")
            == completed_turn.get("id")
            and started_turn.get("status") == "inProgress"
            and completed_turn.get("status") == "completed"
            and completed_turn.get("error") is None
            and isinstance(lifecycle, dict)
            and lifecycle
            == {
                "initialize_response_received": True,
                "initialized_sent": True,
                "thread_start_response_received": True,
                "thread_started_received": True,
                "thread_id": response_thread.get("id"),
                "turn_start_response_received": True,
                "turn_started_received": True,
                "turn_completed_received": True,
                "turn_id": response_turn.get("id"),
                "terminal_status": "completed",
            }
        )
        if not checks["ordered_lifecycle_replayed"]:
            errors.append("ordered lifecycle replay mismatch")

        checks["receipt_semantics_valid"] = (
            receipt.get("valid") is True
            and receipt.get("probe_status") == "captured"
            and receipt.get("error_kind") is None
            and receipt.get("error_sha256") is None
            and receipt.get("containment_assigned") is True
            and receipt.get("process_id") is not None
            and receipt.get("process_exit_code") is not None
        )
        if not checks["receipt_semantics_valid"]:
            errors.append("successful probe receipt semantics mismatch")

        try:
            stderr_data = stderr_path.read_bytes()
            stderr_artifact = receipt.get("stderr_artifact")
            projection_matches = (
                _artifact_matches(stderr_artifact, stderr_path, stderr_data)
                and stderr_artifact.get("total_bytes") >= len(stderr_data)
                and stderr_artifact.get("truncated")
                == (stderr_artifact.get("total_bytes") > len(stderr_data))
            )
            if projection_matches and not stderr_artifact.get("truncated"):
                projection_matches = (
                    stderr_artifact.get("full_sha256")
                    == sha256_bytes(stderr_data)
                )
            checks["stderr_artifact_matches"] = projection_matches
        except OSError as exc:
            errors.append(f"cannot read stderr artifact: {exc}")
        if not checks["stderr_artifact_matches"]:
            errors.append("stderr artifact projection mismatch")

    valid = all(checks.values()) and not errors
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-turn-probe-verification",
        "valid": valid,
        "probe_id": receipt["probe_id"] if receipt is not None else None,
        "source_stream_id": (
            receipt["source_stream_id"] if receipt is not None else None
        ),
        "checks": checks,
        "errors": errors,
        "limitations": LIMITATIONS,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify a Codex app-server synthetic turn probe."
    )
    parser.add_argument("--capture", required=True)
    parser.add_argument("--stderr", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()

    output = Path(args.output).resolve()
    if output.exists():
        print(f"refusing to overwrite existing output: {output}", file=sys.stderr)
        return 1
    try:
        report = verify(
            capture_path=Path(args.capture).resolve(),
            stderr_path=Path(args.stderr).resolve(),
            receipt_path=Path(args.receipt).resolve(),
        )
        validate_instance(
            report, VERIFICATION_SCHEMA, label="turn probe verification"
        )
        atomic_write_json(output, report)
    except (OSError, PrototypeError, ValueError) as exc:
        print(f"turn probe verification failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
