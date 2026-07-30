from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Sequence
import uuid

from .codex_app_server_capture import _artifact, _capture_bytes, _record, _stderr_artifact, _thread
from .errors import AssuranceError
from .io_utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now
from .layout import RUNTIME_ROOT
from .schema import validate_instance


PROBE_SUPERVISOR_ID = "CLIPROBE-CODEX-APP-SERVER-TURN-V0.1"
PROBE_RECEIPT_SCHEMA = RUNTIME_ROOT / "codex-app-server-turn-probe-v0.1.schema.json"
PROVIDER_ID = "lif_lifecycle_loopback"
SYNTHETIC_MODEL = "gpt-5.4"
SYNTHETIC_INPUT = "LIFECYCLE_PROBE_INPUT_0E99A0F4: reply with the fixed synthetic response only."
SYNTHETIC_OUTPUT = "LIFECYCLE_PROBE_OK_7A4F90D1"
INITIALIZE_REQUEST_ID = 0
THREAD_START_REQUEST_ID = 1
TURN_START_REQUEST_ID = 2
TURN_INTERRUPT_REQUEST_ID = 3
THREAD_UNSUBSCRIBE_REQUEST_ID = 4


def _synthetic_sse() -> bytes:
    value = {
        "type": "response.completed",
        "response": {"id": "resp_lif_lifecycle_probe"},
        "output": SYNTHETIC_OUTPUT,
    }
    data = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return f"event: response.completed\ndata: {data}\n\n".encode("utf-8")


def _turn(turn_id: str = "turn_fake_probe", status: str = "inProgress", error: dict[str, Any] | None = None) -> dict[str, Any]:
    return {"id": turn_id, "status": status, "items": [], "error": error}


def _provider_request() -> dict[str, Any]:
    body = {
        "model": SYNTHETIC_MODEL,
        "stream": True,
        "input": [{"type": "message", "role": "user", "content": [{"type": "input_text", "text": SYNTHETIC_INPUT}]}],
    }
    raw = json.dumps(body, separators=(",", ":")).encode("utf-8")
    return {
        "method": "POST",
        "path": "/v1/responses",
        "client_host": "127.0.0.1",
        "body_bytes": len(raw),
        "body_sha256": sha256_bytes(raw),
        "model": SYNTHETIC_MODEL,
        "stream": True,
        "input_marker_present": True,
        "accepts_event_stream": True,
        "authorization_present": False,
    }


def _write_config(path: Path, port: int = 7777) -> bytes:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = f'''model = "{SYNTHETIC_MODEL}"
model_provider = "{PROVIDER_ID}"
web_search = "disabled"

[features]
apps = false
plugins = false

[model_providers.{PROVIDER_ID}]
name = "Lifecycle Loopback"
base_url = "http://127.0.0.1:{port}/v1"
wire_api = "responses"
request_max_retries = 0
stream_max_retries = 0
stream_idle_timeout_ms = 5000
'''
    data = text.encode("utf-8")
    path.write_bytes(data)
    return data


def probe_synthetic_turn(
    *,
    command: Sequence[str],
    cwd: Path,
    runtime_version: str,
    capture_path: Path,
    stderr_path: Path,
    receipt_path: Path,
    isolated_state_dir: Path,
    timeout_seconds: float = 10.0,
    shutdown_grace_seconds: float = 2.0,
    terminal_scenario: str = "completed",
    **_: Any,
) -> dict[str, Any]:
    if capture_path.exists() or stderr_path.exists() or receipt_path.exists():
        raise AssuranceError("refusing to overwrite existing probe outputs")
    mode = command[-1] if command else "turn-success"
    stream_id = f"CLISTREAM-CODEX-{uuid.uuid4().hex[:12].upper()}"
    config_path = isolated_state_dir / "config.toml"
    config_data = _write_config(config_path)
    provider_mode = {
        "completed": "success",
        "closed": "success",
        "failed": "http_error",
        "interrupted": "stall",
    }[terminal_scenario]
    terminal_status = {
        "completed": "completed",
        "closed": "completed",
        "failed": "failed",
        "interrupted": "interrupted",
    }[terminal_scenario]
    if mode == "turn-provider-invalid":
        terminal_status = "failed"
    turn_error = {"message": "synthetic provider request failed"} if terminal_status == "failed" else None
    messages: list[tuple[str, dict[str, Any]]] = [
        ("client_to_server", {"id": INITIALIZE_REQUEST_ID, "method": "initialize", "params": {}}),
        ("server_to_client", {"id": INITIALIZE_REQUEST_ID, "result": {"userAgent": "fake-codex/0.1.0"}}),
        ("client_to_server", {"method": "initialized", "params": {}}),
        ("client_to_server", {"id": THREAD_START_REQUEST_ID, "method": "thread/start", "params": {"ephemeral": True, "sandbox": "read-only"}}),
        ("server_to_client", {"id": THREAD_START_REQUEST_ID, "result": {"thread": _thread()}}),
        ("server_to_client", {"method": "thread/started", "params": {"thread": _thread()}}),
        ("client_to_server", {"id": TURN_START_REQUEST_ID, "method": "turn/start", "params": {"threadId": "thr_fake_capture", "input": [{"type": "text", "text": SYNTHETIC_INPUT}]}}),
        ("server_to_client", {"id": TURN_START_REQUEST_ID, "result": {"turn": _turn()}}),
        ("server_to_client", {"method": "turn/started", "params": {"turn": _turn()}}),
    ]
    if terminal_scenario == "interrupted":
        messages.extend([
            ("client_to_server", {"id": TURN_INTERRUPT_REQUEST_ID, "method": "turn/interrupt", "params": {"threadId": "thr_fake_capture", "turnId": "turn_fake_probe"}}),
            ("server_to_client", {"id": TURN_INTERRUPT_REQUEST_ID, "result": {}}),
        ])
    messages.append(("server_to_client", {"method": "turn/completed", "params": {"turn": _turn(status=terminal_status, error=turn_error)}}))
    if terminal_scenario == "closed":
        messages.extend([
            ("client_to_server", {"id": THREAD_UNSUBSCRIBE_REQUEST_ID, "method": "thread/unsubscribe", "params": {"threadId": "thr_fake_capture"}}),
            ("server_to_client", {"id": THREAD_UNSUBSCRIBE_REQUEST_ID, "result": {"status": "unsubscribed"}}),
            ("server_to_client", {"method": "thread/closed", "params": {"threadId": "thr_fake_capture"}}),
        ])
    records = [_record(stream_id, index, direction, message) for index, (direction, message) in enumerate(messages)]
    capture_data = _capture_bytes(records)
    stderr_data = b"fake app-server diagnostic\n"
    capture_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    capture_path.write_bytes(capture_data)
    stderr_path.write_bytes(stderr_data)
    valid = mode != "turn-provider-invalid"
    provider_request = None if terminal_scenario == "interrupted" else _provider_request()
    receipt = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-turn-probe",
        "probe_id": f"CLIPROBE-CODEX-{sha256_bytes(capture_data)[:16].upper()}",
        "supervisor_id": PROBE_SUPERVISOR_ID,
        "valid": valid,
        "probe_status": "captured" if valid else "failed",
        "runtime_family": "codex-app-server",
        "runtime_version": runtime_version,
        "source_stream_id": stream_id,
        "started_at": utc_now(),
        "finished_at": utc_now(),
        "command_sha256": sha256_bytes(canonical_bytes(list(command))),
        "working_directory": str(cwd.resolve()),
        "isolated_state_directory": str(isolated_state_dir.resolve()),
        "process_id": 1,
        "process_exit_code": 0 if valid else 1,
        "shutdown_method": "stdin_eof",
        "containment_kind": "windows_job",
        "containment_assigned": True,
        "config_artifact": _artifact(config_path, config_data),
        "provider": {
            "provider_id": PROVIDER_ID,
            "model": SYNTHETIC_MODEL,
            "response_mode": provider_mode,
            "base_url": "http://127.0.0.1:<ephemeral>/v1",
            "request_count": 0 if provider_request is None else 1,
            "request": provider_request,
            "synthetic_input_sha256": sha256_bytes(SYNTHETIC_INPUT.encode("utf-8")),
            "synthetic_output_sha256": sha256_bytes(SYNTHETIC_OUTPUT.encode("utf-8")),
            "sse_sha256": sha256_bytes(_synthetic_sse()),
        },
        "lifecycle": {
            "terminal_scenario": terminal_scenario,
            "initialize_response_received": True,
            "initialized_sent": True,
            "thread_start_response_received": True,
            "thread_started_received": True,
            "thread_id": "thr_fake_capture",
            "turn_start_response_received": True,
            "turn_started_received": True,
            "interrupt_request_sent": terminal_scenario == "interrupted",
            "interrupt_response_received": terminal_scenario == "interrupted",
            "turn_completed_received": True,
            "turn_id": "turn_fake_probe",
            "terminal_status": terminal_status,
            "thread_unsubscribe_request_sent": terminal_scenario == "closed",
            "thread_unsubscribe_response_received": terminal_scenario == "closed",
            "thread_closed_received": terminal_scenario == "closed",
        },
        "capture_artifact": _artifact(capture_path, capture_data),
        "record_count": len(records),
        "stderr_artifact": _stderr_artifact(stderr_path, stderr_data),
        "error_kind": None if valid else "protocol_error",
        "error_sha256": None if valid else sha256_bytes(b"protocol_error"),
        "limitations": ["Synthetic no-model turn lifecycle probe."],
    }
    validate_instance(receipt, PROBE_RECEIPT_SCHEMA, label="turn probe receipt")
    atomic_write_json(receipt_path, receipt)
    return receipt
