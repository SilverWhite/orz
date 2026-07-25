from __future__ import annotations

from copy import deepcopy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import time
from typing import Any, BinaryIO, Sequence
import uuid

from .codex_app_server_capture import (
    CodexAppServerCaptureError,
    _BoundedDigestCapture,
    _ProcessContainment,
    _artifact,
    _capture_bytes,
    _stderr_artifact,
    _stdout_reader,
    _validate_launch,
)
from .errors import PrototypeError
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    sha256_bytes,
    utc_now,
)
from .layout import RUNTIME_ROOT
from .schema import validate_instance


PROBE_SUPERVISOR_ID = "CLIPROBE-CODEX-APP-SERVER-TURN-V0.1"
PROBE_RECEIPT_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-turn-probe-v0.1.schema.json"
)
RUNTIME_FAMILY = "codex-app-server"
PROVIDER_ID = "lif_lifecycle_loopback"
SYNTHETIC_MODEL = "gpt-5.4"
SYNTHETIC_INPUT = (
    "LIFECYCLE_PROBE_INPUT_0E99A0F4: reply with the fixed synthetic response only."
)
SYNTHETIC_OUTPUT = "LIFECYCLE_PROBE_OK_7A4F90D1"
INITIALIZE_REQUEST_ID = 0
THREAD_START_REQUEST_ID = 1
TURN_START_REQUEST_ID = 2
MAX_PROVIDER_BODY_BYTES = 8 * 1024 * 1024

LIMITATIONS = [
    "The probe validates one synthetic completed turn, not model quality or task correctness.",
    "The loopback provider proves the configured model request reached 127.0.0.1; it is not an operating-system network sandbox for every app-server subsystem.",
    "The capture proves one supervisor-observed stdio order, not hidden server-internal causality.",
    "The fixed synthetic prompt and response are present in the raw capture; normalized lifecycle observations omit raw content.",
]


def _contains_string(value: Any, expected: str) -> bool:
    if isinstance(value, str):
        return expected in value
    if isinstance(value, list):
        return any(_contains_string(item, expected) for item in value)
    if isinstance(value, dict):
        return any(_contains_string(item, expected) for item in value.values())
    return False


def _sse_event(value: dict[str, Any]) -> bytes:
    event_type = value["type"]
    data = json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    )
    return f"event: {event_type}\ndata: {data}\n\n".encode("utf-8")


def _synthetic_sse() -> bytes:
    response_id = "resp_lif_lifecycle_probe"
    return b"".join(
        [
            _sse_event(
                {
                    "type": "response.created",
                    "response": {"id": response_id},
                }
            ),
            _sse_event(
                {
                    "type": "response.output_item.done",
                    "item": {
                        "type": "message",
                        "role": "assistant",
                        "id": "msg_lif_lifecycle_probe",
                        "content": [
                            {"type": "output_text", "text": SYNTHETIC_OUTPUT}
                        ],
                    },
                }
            ),
            _sse_event(
                {
                    "type": "response.completed",
                    "response": {
                        "id": response_id,
                        "usage": {
                            "input_tokens": 0,
                            "input_tokens_details": None,
                            "output_tokens": 0,
                            "output_tokens_details": None,
                            "total_tokens": 0,
                        },
                    },
                }
            ),
        ]
    )


class _ResponsesServer(ThreadingHTTPServer):
    daemon_threads = True
    allow_reuse_address = False

    def __init__(self) -> None:
        super().__init__(("127.0.0.1", 0), _ResponsesHandler)
        self._requests: list[dict[str, Any]] = []
        self._lock = threading.Lock()
        self.request_recorded = threading.Event()

    def record_request(self, request: dict[str, Any]) -> None:
        with self._lock:
            self._requests.append(deepcopy(request))
            self.request_recorded.set()

    def request_snapshot(self) -> list[dict[str, Any]]:
        with self._lock:
            return deepcopy(self._requests)


class _ResponsesHandler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    server_version = "LIFLifecycleResponsesProbe/0.1"

    @property
    def responses_server(self) -> _ResponsesServer:
        return self.server  # type: ignore[return-value]

    def do_POST(self) -> None:  # noqa: N802 - stdlib handler API
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            self.send_error(400)
            return
        if length <= 0 or length > MAX_PROVIDER_BODY_BYTES:
            self.send_error(413)
            return
        raw = self.rfile.read(length)
        try:
            body = json.loads(
                raw.decode("utf-8"),
                parse_constant=lambda value: (_ for _ in ()).throw(
                    ValueError(f"non-standard JSON constant {value}")
                ),
            )
        except (UnicodeDecodeError, json.JSONDecodeError, ValueError):
            self.send_error(400)
            return
        valid_body = isinstance(body, dict)
        request_record = {
            "method": "POST",
            "path": self.path,
            "client_host": self.client_address[0],
            "body_bytes": len(raw),
            "body_sha256": sha256_bytes(raw),
            "model": body.get("model") if valid_body else None,
            "stream": body.get("stream") if valid_body else None,
            "input_marker_present": (
                _contains_string(body, SYNTHETIC_INPUT) if valid_body else False
            ),
            "accepts_event_stream": (
                "text/event-stream"
                in self.headers.get("Accept", "").lower()
            ),
            "authorization_present": "Authorization" in self.headers,
        }
        self.responses_server.record_request(request_record)

        valid_request = (
            self.path == "/v1/responses"
            and valid_body
            and body.get("model") == SYNTHETIC_MODEL
            and body.get("stream") is True
            and request_record["input_marker_present"] is True
            and request_record["accepts_event_stream"] is True
            and request_record["authorization_present"] is False
            and self.client_address[0] == "127.0.0.1"
        )
        if not valid_request:
            response = json.dumps(
                {"error": {"type": "invalid_lifecycle_probe_request"}},
                separators=(",", ":"),
            ).encode("utf-8")
            self.send_response(400)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(response)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
            self.close_connection = True
            return

        response = _synthetic_sse()
        try:
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Content-Length", str(len(response)))
            self.send_header("Connection", "close")
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
            self.close_connection = True
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError):
            self.close_connection = True

    def log_message(self, format: str, *args: object) -> None:
        return


class LoopbackResponsesProvider:
    def __init__(self) -> None:
        self._server = _ResponsesServer()
        self._thread = threading.Thread(
            target=self._server.serve_forever,
            kwargs={"poll_interval": 0.05},
            name="codex-responses-loopback-provider",
            daemon=True,
        )
        self._started = False

    @property
    def base_url(self) -> str:
        return f"http://127.0.0.1:{self._server.server_port}/v1"

    @property
    def requests(self) -> list[dict[str, Any]]:
        return self._server.request_snapshot()

    def wait_for_request(self, timeout_seconds: float) -> bool:
        return self._server.request_recorded.wait(timeout=timeout_seconds)

    def __enter__(self) -> "LoopbackResponsesProvider":
        self._thread.start()
        self._started = True
        return self

    def __exit__(self, exc_type: object, exc: object, traceback: object) -> None:
        if self._started:
            self._server.shutdown()
            self._server.server_close()
            self._thread.join(timeout=5)
            self._started = False


def _config_bytes(provider_base_url: str) -> bytes:
    content = "\n".join(
        [
            f'model = "{SYNTHETIC_MODEL}"',
            f'model_provider = "{PROVIDER_ID}"',
            'web_search = "disabled"',
            "",
            f"[model_providers.{PROVIDER_ID}]",
            'name = "LIF Lifecycle Loopback Provider"',
            f'base_url = "{provider_base_url}"',
            'wire_api = "responses"',
            "request_max_retries = 0",
            "stream_max_retries = 0",
            "stream_idle_timeout_ms = 5000",
            "",
            "[features]",
            "apps = false",
            "plugins = false",
            "",
        ]
    )
    return content.encode("utf-8")


def probe_synthetic_turn(
    *,
    command: Sequence[str],
    cwd: Path,
    runtime_version: str,
    capture_path: Path,
    stderr_path: Path,
    receipt_path: Path,
    isolated_state_dir: Path,
    timeout_seconds: float = 20.0,
    shutdown_grace_seconds: float = 2.0,
    max_line_bytes: int = 1024 * 1024,
    max_records: int = 256,
    max_stderr_bytes: int = 1024 * 1024,
    source_stream_id: str | None = None,
) -> dict[str, Any]:
    command_list, resolved_cwd = _validate_launch(
        command,
        cwd,
        timeout_seconds=timeout_seconds,
        shutdown_grace_seconds=shutdown_grace_seconds,
        max_line_bytes=max_line_bytes,
        max_records=max_records,
        max_stderr_bytes=max_stderr_bytes,
    )
    if not runtime_version:
        raise PrototypeError("runtime version is required")
    paths = [capture_path.resolve(), stderr_path.resolve(), receipt_path.resolve()]
    if len(set(paths)) != 3:
        raise PrototypeError("capture, stderr, and receipt paths must be distinct")
    for path in paths:
        if path.exists():
            raise PrototypeError(f"refusing to overwrite existing output: {path}")

    state_dir = isolated_state_dir.resolve()
    state_dir.mkdir(parents=True, exist_ok=True)
    if not state_dir.is_dir():
        raise PrototypeError("isolated state path must be a directory")
    config_path = state_dir / "config.toml"
    if config_path.exists():
        raise PrototypeError(f"refusing to overwrite existing config: {config_path}")

    stream_id = source_stream_id or f"CLISTREAM-CODEX-{uuid.uuid4().hex.upper()}"
    if not re.fullmatch(r"CLISTREAM-[A-Z0-9._-]+", stream_id):
        raise PrototypeError("source stream id is invalid")

    probe_id = f"CLIPROBE-CODEX-{uuid.uuid4().hex.upper()}"
    started_at = utc_now()
    records: list[dict[str, Any]] = []
    stderr_capture = _BoundedDigestCapture(max_stderr_bytes)
    process: subprocess.Popen[bytes] | None = None
    containment = _ProcessContainment()
    stdout_thread: threading.Thread | None = None
    stderr_thread: threading.Thread | None = None
    output_queue: queue.Queue[tuple[str, str, Any]] = queue.Queue()
    record_lock = threading.Lock()
    error_kind: str | None = None
    error_message: str | None = None
    shutdown_method = "not_started"
    initialize_response_received = False
    initialized_sent = False
    thread_start_response_received = False
    thread_started_received = False
    turn_start_response_received = False
    turn_started_received = False
    turn_completed_received = False
    response_thread_id: str | None = None
    notification_thread_id: str | None = None
    response_turn_id: str | None = None
    started_turn_id: str | None = None
    completed_turn_id: str | None = None
    terminal_status: str | None = None
    config_data = b""
    provider_base_url: str | None = None
    provider_requests: list[dict[str, Any]] = []

    def append_record_unlocked(
        direction: str, message: dict[str, Any], received_at: str | None = None
    ) -> None:
        if len(records) >= max_records:
            raise CodexAppServerCaptureError(
                "output_limit", "capture record limit exceeded"
            )
        records.append(
            {
                "schema_version": "0.1.0",
                "artifact_kind": "codex-app-server-capture-record",
                "source_stream_id": stream_id,
                "source_record_sequence": len(records),
                "received_at": received_at or utc_now(),
                "direction": direction,
                "message": message,
            }
        )

    def record_server_message(message: dict[str, Any], timestamp: str) -> None:
        with record_lock:
            append_record_unlocked("server_to_client", message, timestamp)

    def send(message: dict[str, Any]) -> None:
        if process is None or process.stdin is None:
            raise CodexAppServerCaptureError(
                "process_exit", "app-server stdin is unavailable"
            )
        wire = json.dumps(
            message,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8") + b"\n"
        try:
            with record_lock:
                append_record_unlocked("client_to_server", message)
                process.stdin.write(wire)
                process.stdin.flush()
        except (BrokenPipeError, OSError) as exc:
            raise CodexAppServerCaptureError(
                "process_exit", f"app-server stdin write failed: {exc}"
            ) from exc

    deadline = time.monotonic() + timeout_seconds

    def receive() -> dict[str, Any]:
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise CodexAppServerCaptureError(
                "timeout", "app-server synthetic turn probe timed out"
            )
        try:
            kind, _, payload = output_queue.get(timeout=remaining)
        except queue.Empty as exc:
            raise CodexAppServerCaptureError(
                "timeout", "app-server synthetic turn probe timed out"
            ) from exc
        if kind == "eof":
            raise CodexAppServerCaptureError(
                "process_exit", "app-server stdout closed during probe"
            )
        if kind == "line_too_large":
            raise CodexAppServerCaptureError(
                "output_limit", "app-server stdout line exceeded limit"
            )
        if kind == "read_error":
            raise CodexAppServerCaptureError(
                "stdout_error", "app-server stdout reader failed"
            )
        if kind == "invalid_stdout":
            raise CodexAppServerCaptureError(
                "invalid_stdout", "app-server stdout was not one JSON object"
            )
        if kind == "capture_error":
            assert isinstance(payload, CodexAppServerCaptureError)
            raise payload
        assert kind == "message" and isinstance(payload, dict)
        return payload

    try:
        with LoopbackResponsesProvider() as provider:
            provider_base_url = provider.base_url
            config_data = _config_bytes(provider_base_url)
            atomic_write_bytes(config_path, config_data)
            environment = os.environ.copy()
            environment["CODEX_HOME"] = str(state_dir)
            environment["CODEX_SQLITE_HOME"] = str(state_dir)
            environment["NO_PROXY"] = "127.0.0.1,localhost"
            environment["no_proxy"] = "127.0.0.1,localhost"
            launch_options = containment.launch_options()
            process = subprocess.Popen(
                command_list,
                cwd=resolved_cwd,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env=environment,
                shell=False,
                close_fds=True,
                **launch_options,
            )
            containment.assign(process)
            assert process.stdout is not None and process.stderr is not None
            stdout_thread = threading.Thread(
                target=_stdout_reader,
                args=(
                    process.stdout,
                    output_queue,
                    max_line_bytes,
                    record_server_message,
                ),
                name="codex-app-server-turn-probe-stdout",
                daemon=True,
            )
            stderr_thread = threading.Thread(
                target=stderr_capture.drain,
                args=(process.stderr,),
                name="codex-app-server-turn-probe-stderr",
                daemon=True,
            )
            stdout_thread.start()
            stderr_thread.start()

            send(
                {
                    "method": "initialize",
                    "id": INITIALIZE_REQUEST_ID,
                    "params": {
                        "clientInfo": {
                            "name": "lif_lifecycle_turn_probe",
                            "title": "LIF Lifecycle Turn Probe",
                            "version": "0.1.0",
                        },
                        "capabilities": {
                            "optOutNotificationMethods": [
                                "remoteControl/status/changed"
                            ]
                        },
                    },
                }
            )
            while not initialize_response_received:
                message = receive()
                if message.get("id") != INITIALIZE_REQUEST_ID:
                    continue
                if "error" in message or not isinstance(
                    message.get("result"), dict
                ):
                    raise CodexAppServerCaptureError(
                        "protocol_error", "initialize failed"
                    )
                initialize_response_received = True

            send({"method": "initialized", "params": {}})
            initialized_sent = True
            send(
                {
                    "method": "thread/start",
                    "id": THREAD_START_REQUEST_ID,
                    "params": {
                        "cwd": str(resolved_cwd),
                        "sandbox": "read-only",
                        "ephemeral": True,
                        "model": SYNTHETIC_MODEL,
                    },
                }
            )
            while not (thread_start_response_received and thread_started_received):
                message = receive()
                if message.get("id") == THREAD_START_REQUEST_ID:
                    if "error" in message:
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "thread/start returned a JSON-RPC error",
                        )
                    result = message.get("result")
                    thread = (
                        result.get("thread") if isinstance(result, dict) else None
                    )
                    response_thread_id = (
                        thread.get("id") if isinstance(thread, dict) else None
                    )
                    if (
                        not isinstance(response_thread_id, str)
                        or not response_thread_id
                        or thread.get("ephemeral") is not True
                    ):
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "thread/start response is invalid",
                        )
                    thread_start_response_received = True
                elif message.get("method") == "thread/started":
                    params = message.get("params")
                    thread = (
                        params.get("thread")
                        if isinstance(params, dict)
                        else None
                    )
                    notification_thread_id = (
                        thread.get("id") if isinstance(thread, dict) else None
                    )
                    if (
                        not isinstance(notification_thread_id, str)
                        or not notification_thread_id
                    ):
                        raise CodexAppServerCaptureError(
                            "protocol_error", "thread/started lacks thread.id"
                        )
                    thread_started_received = True
            if response_thread_id != notification_thread_id:
                raise CodexAppServerCaptureError(
                    "protocol_error",
                    "thread response and notification ids differ",
                )

            send(
                {
                    "method": "turn/start",
                    "id": TURN_START_REQUEST_ID,
                    "params": {
                        "threadId": response_thread_id,
                        "input": [{"type": "text", "text": SYNTHETIC_INPUT}],
                    },
                }
            )
            while not (
                turn_start_response_received
                and turn_started_received
                and turn_completed_received
            ):
                message = receive()
                if message.get("id") == TURN_START_REQUEST_ID:
                    if "error" in message:
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "turn/start returned a JSON-RPC error",
                        )
                    result = message.get("result")
                    turn = (
                        result.get("turn") if isinstance(result, dict) else None
                    )
                    response_turn_id = (
                        turn.get("id") if isinstance(turn, dict) else None
                    )
                    if not isinstance(response_turn_id, str) or not response_turn_id:
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "turn/start response lacks turn.id",
                        )
                    turn_start_response_received = True
                elif message.get("method") == "turn/started":
                    if not turn_start_response_received:
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "turn/started preceded turn/start response",
                        )
                    params = message.get("params")
                    turn = (
                        params.get("turn")
                        if isinstance(params, dict)
                        else None
                    )
                    started_turn_id = (
                        turn.get("id") if isinstance(turn, dict) else None
                    )
                    if (
                        started_turn_id != response_turn_id
                        or turn.get("status") != "inProgress"
                    ):
                        raise CodexAppServerCaptureError(
                            "protocol_error", "turn/started is invalid"
                        )
                    turn_started_received = True
                elif message.get("method") == "turn/completed":
                    if not turn_started_received:
                        raise CodexAppServerCaptureError(
                            "protocol_error",
                            "turn/completed preceded turn/started",
                        )
                    params = message.get("params")
                    turn = (
                        params.get("turn")
                        if isinstance(params, dict)
                        else None
                    )
                    completed_turn_id = (
                        turn.get("id") if isinstance(turn, dict) else None
                    )
                    terminal_status = (
                        turn.get("status") if isinstance(turn, dict) else None
                    )
                    if (
                        completed_turn_id != response_turn_id
                        or terminal_status != "completed"
                        or turn.get("error") is not None
                    ):
                        raise CodexAppServerCaptureError(
                            "protocol_error", "turn/completed is invalid"
                        )
                    turn_completed_received = True

            remaining = max(0.0, deadline - time.monotonic())
            provider.wait_for_request(remaining)
            provider_requests = provider.requests
            if len(provider_requests) != 1:
                raise CodexAppServerCaptureError(
                    "provider_error",
                    "expected exactly one loopback Responses request",
                )
    except CodexAppServerCaptureError as exc:
        error_kind = exc.kind
        error_message = str(exc)
    except (OSError, ValueError) as exc:
        error_kind = "spawn_or_containment_error"
        error_message = str(exc)
    finally:
        if process is not None:
            if process.stdin is not None:
                try:
                    process.stdin.close()
                except OSError:
                    pass
            try:
                process.wait(timeout=shutdown_grace_seconds)
                shutdown_method = "stdin_eof"
            except subprocess.TimeoutExpired:
                shutdown_method = containment.terminate_tree(process)
                try:
                    process.wait(timeout=shutdown_grace_seconds)
                except subprocess.TimeoutExpired:
                    shutdown_method = containment.kill_tree(process)
                    try:
                        process.wait(timeout=shutdown_grace_seconds)
                    except subprocess.TimeoutExpired:
                        error_kind = error_kind or "containment_error"
                        error_message = error_message or (
                            "spawned process did not exit after tree termination"
                        )
        containment.close()
        if stdout_thread is not None:
            stdout_thread.join(timeout=shutdown_grace_seconds)
        if stderr_thread is not None:
            stderr_thread.join(timeout=shutdown_grace_seconds)
        if stderr_capture.error and error_kind is None:
            error_kind = "stderr_capture_error"
            error_message = "stderr capture failed"

    capture_data = _capture_bytes(records)
    stderr_data = bytes(stderr_capture.retained)
    atomic_write_bytes(capture_path, capture_data)
    atomic_write_bytes(stderr_path, stderr_data)
    provider_request = (
        provider_requests[0] if len(provider_requests) == 1 else None
    )
    provider_request_valid = (
        isinstance(provider_request, dict)
        and provider_request.get("method") == "POST"
        and provider_request.get("path") == "/v1/responses"
        and provider_request.get("client_host") == "127.0.0.1"
        and provider_request.get("model") == SYNTHETIC_MODEL
        and provider_request.get("stream") is True
        and provider_request.get("input_marker_present") is True
        and provider_request.get("accepts_event_stream") is True
        and provider_request.get("authorization_present") is False
    )
    success = (
        error_kind is None
        and initialize_response_received
        and initialized_sent
        and thread_start_response_received
        and thread_started_received
        and turn_start_response_received
        and turn_started_received
        and turn_completed_received
        and response_thread_id == notification_thread_id
        and response_turn_id == started_turn_id == completed_turn_id
        and terminal_status == "completed"
        and provider_request_valid
    )
    receipt = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-turn-probe",
        "probe_id": probe_id,
        "supervisor_id": PROBE_SUPERVISOR_ID,
        "valid": success,
        "probe_status": "captured" if success else "failed",
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "source_stream_id": stream_id,
        "started_at": started_at,
        "finished_at": utc_now(),
        "command_sha256": sha256_bytes(canonical_bytes(command_list)),
        "working_directory": str(resolved_cwd),
        "isolated_state_directory": str(state_dir),
        "process_id": process.pid if process is not None else None,
        "process_exit_code": process.poll() if process is not None else None,
        "shutdown_method": shutdown_method,
        "containment_kind": containment.kind,
        "containment_assigned": containment.assigned,
        "config_artifact": _artifact(config_path, config_data),
        "provider": {
            "provider_id": PROVIDER_ID,
            "model": SYNTHETIC_MODEL,
            "base_url": (
                re.sub(r":\d+/v1$", ":<ephemeral>/v1", provider_base_url)
                if provider_base_url is not None
                else None
            ),
            "request_count": len(provider_requests),
            "request": provider_request,
            "synthetic_input_sha256": sha256_bytes(
                SYNTHETIC_INPUT.encode("utf-8")
            ),
            "synthetic_output_sha256": sha256_bytes(
                SYNTHETIC_OUTPUT.encode("utf-8")
            ),
            "sse_sha256": sha256_bytes(_synthetic_sse()),
        },
        "lifecycle": {
            "initialize_response_received": initialize_response_received,
            "initialized_sent": initialized_sent,
            "thread_start_response_received": thread_start_response_received,
            "thread_started_received": thread_started_received,
            "thread_id": (
                response_thread_id
                if response_thread_id == notification_thread_id
                else None
            ),
            "turn_start_response_received": turn_start_response_received,
            "turn_started_received": turn_started_received,
            "turn_completed_received": turn_completed_received,
            "turn_id": (
                response_turn_id
                if response_turn_id == started_turn_id == completed_turn_id
                else None
            ),
            "terminal_status": terminal_status,
        },
        "capture_artifact": _artifact(capture_path, capture_data),
        "record_count": len(records),
        "stderr_artifact": _stderr_artifact(
            stderr_path, stderr_data, stderr_capture
        ),
        "error_kind": error_kind,
        "error_sha256": (
            sha256_bytes(error_message.encode("utf-8"))
            if error_message is not None
            else None
        ),
        "limitations": LIMITATIONS,
    }
    validate_instance(receipt, PROBE_RECEIPT_SCHEMA, label="turn probe receipt")
    atomic_write_json(receipt_path, receipt)
    return receipt
