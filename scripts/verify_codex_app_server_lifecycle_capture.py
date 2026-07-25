#!/usr/bin/env python3
"""Independently verify a no-model Codex app-server lifecycle capture receipt."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from prototype.fep_agent_proto.codex_app_server_capture import (
    CAPTURE_RECEIPT_SCHEMA,
    INITIALIZE_REQUEST_ID,
    THREAD_START_REQUEST_ID,
)
from prototype.fep_agent_proto.codex_app_server_lifecycle import (
    CAPTURE_RECORD_SCHEMA,
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
    RUNTIME_ROOT / "codex-app-server-lifecycle-capture-verification-v0.1.schema.json"
)

LIMITATIONS = [
    "The verifier reconstructs capture/receipt consistency but cannot recover messages the supervisor never observed.",
    "The command digest cannot be independently reconstructed because command arguments are intentionally absent from the receipt.",
    "When stderr retention is truncated, the verifier can check the retained artifact but not independently recompute the full-stream digest.",
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
        "handshake_replayed": False,
        "no_model_input": False,
        "stderr_artifact_matches": False,
    }
    receipt: dict[str, Any] | None = None
    try:
        value = load_json(receipt_path)
        if not isinstance(value, dict):
            raise PrototypeError("capture receipt must be an object")
        validate_instance(value, CAPTURE_RECEIPT_SCHEMA, label="capture receipt")
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
        artifact = receipt["capture_artifact"]
        checks["capture_artifact_matches"] = (
            artifact["path"] == str(capture_path.resolve())
            and artifact["bytes"] == len(capture_data)
            and artifact["sha256"] == sha256_bytes(capture_data)
            and receipt["record_count"] == len(records)
        )
        if not checks["capture_artifact_matches"]:
            errors.append("capture artifact projection mismatch")

        stream_ids = {
            record["source_stream_id"]
            for record in records
            if isinstance(record, dict) and "source_stream_id" in record
        }
        checks["capture_records_valid"] = (
            bool(records)
            and stream_ids == {receipt["source_stream_id"]}
            and all(
                record["source_record_sequence"] == index
                for index, record in enumerate(records)
            )
        )
        if not checks["capture_records_valid"]:
            errors.append("capture record stream or sequence mismatch")

        client_messages = [
            record["message"]
            for record in records
            if record["direction"] == "client_to_server"
        ]
        expected_client_methods = ["initialize", "initialized", "thread/start"]
        observed_client_methods = [
            message.get("method") for message in client_messages
        ]
        thread_request = next(
            (
                message
                for message in client_messages
                if message.get("method") == "thread/start"
            ),
            None,
        )
        thread_params = (
            thread_request.get("params")
            if isinstance(thread_request, dict)
            else None
        )
        checks["no_model_input"] = (
            observed_client_methods == expected_client_methods
            and all(message.get("method") != "turn/start" for message in client_messages)
            and isinstance(thread_params, dict)
            and "input" not in thread_params
            and thread_params.get("ephemeral") is True
            and thread_params.get("sandbox") == "read-only"
        )
        if not checks["no_model_input"]:
            errors.append("capture contains an unexpected client or model-input surface")

        initialize_response = next(
            (
                record["message"]
                for record in records
                if record["direction"] == "server_to_client"
                and record["message"].get("id") == INITIALIZE_REQUEST_ID
            ),
            None,
        )
        thread_response = next(
            (
                record["message"]
                for record in records
                if record["direction"] == "server_to_client"
                and record["message"].get("id") == THREAD_START_REQUEST_ID
            ),
            None,
        )
        thread_notification = next(
            (
                record["message"]
                for record in records
                if record["direction"] == "server_to_client"
                and record["message"].get("method") == "thread/started"
            ),
            None,
        )
        response_result = (
            thread_response.get("result")
            if isinstance(thread_response, dict)
            else None
        )
        response_thread = (
            response_result.get("thread")
            if isinstance(response_result, dict)
            else None
        )
        notification_params = (
            thread_notification.get("params")
            if isinstance(thread_notification, dict)
            else None
        )
        notification_thread = (
            notification_params.get("thread")
            if isinstance(notification_params, dict)
            else None
        )
        response_id = (
            response_thread.get("id")
            if isinstance(response_thread, dict)
            else None
        )
        notification_id = (
            notification_thread.get("id")
            if isinstance(notification_thread, dict)
            else None
        )
        checks["handshake_replayed"] = (
            isinstance(initialize_response, dict)
            and isinstance(initialize_response.get("result"), dict)
            and isinstance(response_id, str)
            and bool(response_id)
            and response_id == notification_id
            and response_thread.get("ephemeral") is True
            and receipt["handshake"]
            == {
                "initialize_response_received": True,
                "initialized_sent": True,
                "thread_start_response_received": True,
                "thread_started_received": True,
                "thread_id": response_id,
            }
        )
        if not checks["handshake_replayed"]:
            errors.append("initialize/thread handshake replay mismatch")

        checks["receipt_semantics_valid"] = (
            receipt["valid"] is True
            and receipt["capture_status"] == "captured"
            and receipt["error_kind"] is None
            and receipt["error_sha256"] is None
            and receipt["containment_assigned"] is True
            and receipt["process_id"] is not None
            and receipt["process_exit_code"] is not None
        )
        if not checks["receipt_semantics_valid"]:
            errors.append("successful capture receipt semantics mismatch")

        try:
            stderr_data = stderr_path.read_bytes()
            stderr_artifact = receipt["stderr_artifact"]
            stderr_projection_matches = (
                stderr_artifact["path"] == str(stderr_path.resolve())
                and stderr_artifact["bytes"] == len(stderr_data)
                and stderr_artifact["sha256"] == sha256_bytes(stderr_data)
                and stderr_artifact["total_bytes"] >= len(stderr_data)
                and stderr_artifact["truncated"]
                == (stderr_artifact["total_bytes"] > len(stderr_data))
            )
            if not stderr_artifact["truncated"]:
                stderr_projection_matches = (
                    stderr_projection_matches
                    and stderr_artifact["full_sha256"]
                    == sha256_bytes(stderr_data)
                )
            checks["stderr_artifact_matches"] = stderr_projection_matches
            if not stderr_projection_matches:
                errors.append("stderr artifact projection mismatch")
        except OSError as exc:
            errors.append(f"cannot read stderr artifact: {exc}")

    valid = all(checks.values())
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-capture-verification",
        "valid": valid,
        "capture_id": receipt["capture_id"] if receipt is not None else None,
        "source_stream_id": (
            receipt["source_stream_id"] if receipt is not None else None
        ),
        "checks": checks,
        "errors": errors,
        "limitations": LIMITATIONS,
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify a no-model Codex app-server capture and receipt."
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
            report, VERIFICATION_SCHEMA, label="capture verification"
        )
        atomic_write_json(output, report)
    except (OSError, PrototypeError, ValueError) as exc:
        print(f"capture verification failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
