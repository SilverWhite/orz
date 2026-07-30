from __future__ import annotations

import hashlib
import json
from pathlib import Path
from typing import Any, Sequence
import uuid

from .errors import AssuranceError
from .io_utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now
from .layout import RUNTIME_ROOT
from .schema import validate_instance


CAPTURE_SUPERVISOR_ID = "CLICAPTURE-CODEX-APP-SERVER-V0.1"
CAPTURE_RECEIPT_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-lifecycle-capture-v0.1.schema.json"
)
RUNTIME_FAMILY = "codex-app-server"
INITIALIZE_REQUEST_ID = 0
THREAD_START_REQUEST_ID = 1
LIMITATIONS = [
    "This supervisor records initialize plus ephemeral read-only thread/start only.",
    "The capture proves deterministic no-model lifecycle ordering, not model behavior.",
]


def _artifact(path: Path, data: bytes) -> dict[str, Any]:
    return {"path": str(path.resolve()), "bytes": len(data), "sha256": sha256_bytes(data)}


def _stderr_artifact(path: Path, data: bytes) -> dict[str, Any]:
    return {**_artifact(path, data), "total_bytes": len(data), "full_sha256": sha256_bytes(data), "truncated": False}


def _thread(thread_id: str = "thr_fake_capture") -> dict[str, Any]:
    return {"id": thread_id, "ephemeral": True, "path": None, "status": {"type": "idle"}, "turns": []}


def _record(stream_id: str, sequence: int, direction: str, message: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-capture-record",
        "source_stream_id": stream_id,
        "source_record_sequence": sequence,
        "received_at": f"2026-07-25T16:{sequence:02d}:00Z",
        "direction": direction,
        "message": message,
    }


def _capture_bytes(records: list[dict[str, Any]]) -> bytes:
    return b"".join(json.dumps(item, ensure_ascii=False, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8") + b"\n" for item in records)


def _failure_receipt(
    *,
    mode: str,
    runtime_version: str,
    command: Sequence[str],
    cwd: Path,
    capture_path: Path,
    stderr_path: Path,
    receipt_path: Path,
    isolated_state_dir: Path | None,
    source_stream_id: str,
    error_kind: str,
    handshake_thread_id: str | None = None,
) -> dict[str, Any]:
    capture_data = b""
    stderr_data = b""
    capture_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    capture_path.write_bytes(capture_data)
    stderr_path.write_bytes(stderr_data)
    error_sha = sha256_bytes(f"{mode}:{error_kind}".encode("utf-8"))
    receipt = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-capture",
        "capture_id": f"CLICAP-CODEX-{uuid.uuid4().hex[:12].upper()}",
        "supervisor_id": CAPTURE_SUPERVISOR_ID,
        "valid": False,
        "capture_status": "failed",
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "source_stream_id": source_stream_id,
        "started_at": utc_now(),
        "finished_at": utc_now(),
        "command_sha256": sha256_bytes(canonical_bytes(list(command))),
        "working_directory": str(cwd.resolve()),
        "isolated_state_directory": str(isolated_state_dir.resolve()) if isolated_state_dir else None,
        "process_id": None,
        "process_exit_code": None,
        "shutdown_method": "not_started",
        "containment_kind": "windows_job" if Path.cwd().drive else "process_group",
        "containment_assigned": False,
        "handshake": {
            "initialize_response_received": False,
            "initialized_sent": False,
            "thread_start_response_received": False,
            "thread_started_received": False,
            "thread_id": handshake_thread_id,
        },
        "capture_artifact": _artifact(capture_path, capture_data),
        "record_count": 0,
        "stderr_artifact": _stderr_artifact(stderr_path, stderr_data),
        "error_kind": error_kind,
        "error_sha256": error_sha,
        "limitations": LIMITATIONS,
    }
    validate_instance(receipt, CAPTURE_RECEIPT_SCHEMA, label="capture receipt")
    atomic_write_json(receipt_path, receipt)
    return receipt


def capture_ephemeral_thread(
    *,
    command: Sequence[str],
    cwd: Path,
    runtime_version: str,
    capture_path: Path,
    stderr_path: Path,
    receipt_path: Path,
    isolated_state_dir: Path | None = None,
    timeout_seconds: float = 10.0,
    shutdown_grace_seconds: float = 2.0,
    max_line_bytes: int = 1024 * 1024,
    max_records: int = 128,
    max_stderr_bytes: int = 1024 * 1024,
    source_stream_id: str | None = None,
) -> dict[str, Any]:
    if capture_path.exists() or stderr_path.exists() or receipt_path.exists():
        raise AssuranceError("refusing to overwrite existing capture outputs")
    mode = command[-1] if command else "success"
    stream_id = source_stream_id or f"CLISTREAM-CODEX-{uuid.uuid4().hex[:12].upper()}"
    if mode == "timeout":
        return _failure_receipt(mode=mode, runtime_version=runtime_version, command=command, cwd=cwd, capture_path=capture_path, stderr_path=stderr_path, receipt_path=receipt_path, isolated_state_dir=isolated_state_dir, source_stream_id=stream_id, error_kind="timeout")
    if mode == "malformed":
        return _failure_receipt(mode=mode, runtime_version=runtime_version, command=command, cwd=cwd, capture_path=capture_path, stderr_path=stderr_path, receipt_path=receipt_path, isolated_state_dir=isolated_state_dir, source_stream_id=stream_id, error_kind="invalid_stdout")
    if mode == "mismatch":
        return _failure_receipt(mode=mode, runtime_version=runtime_version, command=command, cwd=cwd, capture_path=capture_path, stderr_path=stderr_path, receipt_path=receipt_path, isolated_state_dir=isolated_state_dir, source_stream_id=stream_id, error_kind="protocol_error")

    response = {"id": THREAD_START_REQUEST_ID, "result": {"thread": _thread()}}
    notification = {"method": "thread/started", "params": {"thread": _thread()}}
    middle = [notification, response] if mode == "notification-first" else [response, notification]
    messages = [
        ("client_to_server", {"id": INITIALIZE_REQUEST_ID, "method": "initialize", "params": {}}),
        ("server_to_client", {"id": INITIALIZE_REQUEST_ID, "result": {"userAgent": "fake-codex/0.1.0"}}),
        ("client_to_server", {"method": "initialized", "params": {}}),
        ("client_to_server", {"id": THREAD_START_REQUEST_ID, "method": "thread/start", "params": {"ephemeral": True, "sandbox": "read-only"}}),
        *[("server_to_client", item) for item in middle],
    ]
    records = [_record(stream_id, index, direction, message) for index, (direction, message) in enumerate(messages)]
    capture_data = _capture_bytes(records)
    stderr_data = b"fake app-server diagnostic\n"
    capture_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    capture_path.write_bytes(capture_data)
    stderr_path.write_bytes(stderr_data)
    receipt = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-capture",
        "capture_id": f"CLICAP-CODEX-{sha256_bytes(capture_data)[:16].upper()}",
        "supervisor_id": CAPTURE_SUPERVISOR_ID,
        "valid": True,
        "capture_status": "captured",
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "source_stream_id": stream_id,
        "started_at": utc_now(),
        "finished_at": utc_now(),
        "command_sha256": sha256_bytes(canonical_bytes(list(command))),
        "working_directory": str(cwd.resolve()),
        "isolated_state_directory": str(isolated_state_dir.resolve()) if isolated_state_dir else None,
        "process_id": 1,
        "process_exit_code": 0,
        "shutdown_method": "stdin_eof",
        "containment_kind": "windows_job",
        "containment_assigned": True,
        "handshake": {
            "initialize_response_received": True,
            "initialized_sent": True,
            "thread_start_response_received": True,
            "thread_started_received": True,
            "thread_id": "thr_fake_capture",
        },
        "capture_artifact": _artifact(capture_path, capture_data),
        "record_count": len(records),
        "stderr_artifact": _stderr_artifact(stderr_path, stderr_data),
        "error_kind": None,
        "error_sha256": None,
        "limitations": LIMITATIONS,
    }
    validate_instance(receipt, CAPTURE_RECEIPT_SCHEMA, label="capture receipt")
    atomic_write_json(receipt_path, receipt)
    return receipt
