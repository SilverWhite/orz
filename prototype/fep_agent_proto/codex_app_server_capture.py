from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import queue
import re
import signal
import subprocess
import threading
import time
from typing import Any, BinaryIO, Callable, Sequence
import uuid

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


CAPTURE_SUPERVISOR_ID = "CLICAPTURE-CODEX-APP-SERVER-V0.1"
CAPTURE_RECEIPT_SCHEMA = (
    RUNTIME_ROOT / "codex-app-server-lifecycle-capture-v0.1.schema.json"
)
RUNTIME_FAMILY = "codex-app-server"

INITIALIZE_REQUEST_ID = 0
THREAD_START_REQUEST_ID = 1

LIMITATIONS = [
    "The supervisor exercises only initialize plus an ephemeral read-only thread/start; it never sends turn/start or model input.",
    "The capture proves one supervisor-observed stdio order, not hidden server-internal causality.",
    "Command arguments and stderr content are digest-bound in the receipt rather than copied into it.",
    "A successful handshake intentionally ends with an active partial lifecycle; transport shutdown is not a thread terminal.",
]


class CodexAppServerCaptureError(RuntimeError):
    def __init__(self, kind: str, message: str) -> None:
        super().__init__(message)
        self.kind = kind


class _BoundedDigestCapture:
    def __init__(self, limit: int) -> None:
        self.limit = limit
        self.total = 0
        self.retained = bytearray()
        self.digest = hashlib.sha256()
        self.error: str | None = None

    def drain(self, stream: BinaryIO) -> None:
        try:
            while True:
                chunk = stream.read(64 * 1024)
                if not chunk:
                    break
                self.total += len(chunk)
                self.digest.update(chunk)
                remaining = self.limit - len(self.retained)
                if remaining > 0:
                    self.retained.extend(chunk[:remaining])
        except OSError as exc:
            self.error = str(exc)
        finally:
            stream.close()


class _ProcessContainment:
    def __init__(self) -> None:
        self.kind = "process_group"
        self.assigned = False
        self._windows_job: Any = None

    def launch_options(self) -> dict[str, Any]:
        if os.name == "nt":
            self.kind = "windows_job"
            from .windows_process import _WindowsJob

            self._windows_job = _WindowsJob.create_kill_on_close()
            return {
                "creationflags": (
                    subprocess.CREATE_NEW_PROCESS_GROUP
                    | subprocess.CREATE_NO_WINDOW
                )
            }
        return {"start_new_session": True}

    def assign(self, process: subprocess.Popen[bytes]) -> None:
        if os.name == "nt":
            if self._windows_job is None:
                raise OSError("Windows job object was not created")
            self._windows_job.assign(int(getattr(process, "_handle")))
        self.assigned = True

    def terminate_tree(self, process: subprocess.Popen[bytes]) -> str:
        if process.poll() is not None:
            return "natural_exit"
        if os.name == "nt" and self._windows_job is not None and self.assigned:
            self._windows_job.close()
            return "process_tree_close"
        if os.name != "nt" and self.assigned:
            try:
                os.killpg(process.pid, signal.SIGTERM)
                return "process_group_terminate"
            except ProcessLookupError:
                return "natural_exit"
        process.terminate()
        return "root_terminate"

    def kill_tree(self, process: subprocess.Popen[bytes]) -> str:
        if process.poll() is not None:
            return "natural_exit"
        if os.name != "nt" and self.assigned:
            try:
                os.killpg(process.pid, signal.SIGKILL)
                return "process_group_kill"
            except ProcessLookupError:
                return "natural_exit"
        process.kill()
        return "root_kill"

    def close(self) -> None:
        if self._windows_job is not None:
            self._windows_job.close()


def _validate_launch(
    command: Sequence[str],
    cwd: Path,
    *,
    timeout_seconds: float,
    shutdown_grace_seconds: float,
    max_line_bytes: int,
    max_records: int,
    max_stderr_bytes: int,
) -> tuple[list[str], Path]:
    if not command or not all(isinstance(item, str) and item for item in command):
        raise PrototypeError("command must contain non-empty strings")
    executable = Path(command[0])
    if not executable.is_absolute() or not executable.is_file():
        raise PrototypeError("executable must be an existing absolute file")
    resolved_cwd = cwd.resolve(strict=True)
    if not resolved_cwd.is_dir():
        raise PrototypeError("cwd must be a directory")
    if timeout_seconds <= 0 or shutdown_grace_seconds <= 0:
        raise PrototypeError("timeouts must be positive")
    if max_line_bytes < 256:
        raise PrototypeError("max line bytes must be at least 256")
    if max_records < 4:
        raise PrototypeError("max records must be at least 4")
    if max_stderr_bytes < 0:
        raise PrototypeError("max stderr bytes cannot be negative")
    return [str(executable.resolve()), *command[1:]], resolved_cwd


def _capture_bytes(records: list[dict[str, Any]]) -> bytes:
    return b"".join(
        json.dumps(
            record,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
        for record in records
    )


def _artifact(path: Path, data: bytes) -> dict[str, Any]:
    return {
        "path": str(path.resolve()),
        "bytes": len(data),
        "sha256": sha256_bytes(data),
    }


def _stderr_artifact(
    path: Path, data: bytes, capture: _BoundedDigestCapture
) -> dict[str, Any]:
    return {
        **_artifact(path, data),
        "total_bytes": capture.total,
        "full_sha256": capture.digest.hexdigest(),
        "truncated": capture.total > len(data),
    }


def _stdout_reader(
    stream: BinaryIO,
    output: queue.Queue[tuple[str, str, Any]],
    max_line_bytes: int,
    record_message: Callable[[dict[str, Any], str], None],
) -> None:
    try:
        while True:
            line = stream.readline(max_line_bytes + 1)
            timestamp = utc_now()
            if not line:
                output.put(("eof", timestamp, None))
                return
            if len(line) > max_line_bytes:
                output.put(("line_too_large", timestamp, line))
                return
            try:
                value = json.loads(line.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError):
                output.put(("invalid_stdout", timestamp, None))
                return
            if not isinstance(value, dict) or not value:
                output.put(("invalid_stdout", timestamp, None))
                return
            try:
                record_message(value, timestamp)
            except CodexAppServerCaptureError as exc:
                output.put(("capture_error", timestamp, exc))
                return
            output.put(("message", timestamp, value))
    except OSError as exc:
        output.put(("read_error", utc_now(), str(exc).encode("utf-8")))
    finally:
        stream.close()


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

    state_dir: Path | None = None
    if isolated_state_dir is not None:
        state_dir = isolated_state_dir.resolve()
        state_dir.mkdir(parents=True, exist_ok=True)
        if not state_dir.is_dir():
            raise PrototypeError("isolated state path must be a directory")

    stream_id = source_stream_id or f"CLISTREAM-CODEX-{uuid.uuid4().hex.upper()}"
    if not re.fullmatch(r"CLISTREAM-[A-Z0-9._-]+", stream_id):
        raise PrototypeError("source stream id is invalid")

    capture_id = f"CLICAP-CODEX-{uuid.uuid4().hex.upper()}"
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
    response_thread_id: str | None = None
    notification_thread_id: str | None = None

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
                "timeout", "app-server handshake timed out"
            )
        try:
            kind, _, payload = output_queue.get(timeout=remaining)
        except queue.Empty as exc:
            raise CodexAppServerCaptureError(
                "timeout", "app-server handshake timed out"
            ) from exc
        if kind == "eof":
            raise CodexAppServerCaptureError(
                "process_exit", "app-server stdout closed during handshake"
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
        environment = os.environ.copy()
        if state_dir is not None:
            environment["CODEX_HOME"] = str(state_dir)
            environment["CODEX_SQLITE_HOME"] = str(state_dir)
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
            name="codex-app-server-stdout",
            daemon=True,
        )
        stderr_thread = threading.Thread(
            target=stderr_capture.drain,
            args=(process.stderr,),
            name="codex-app-server-stderr",
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
                        "name": "lif_lifecycle_capture",
                        "title": "LIF Lifecycle Capture",
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
            if "error" in message:
                raise CodexAppServerCaptureError(
                    "protocol_error", "initialize returned a JSON-RPC error"
                )
            if not isinstance(message.get("result"), dict):
                raise CodexAppServerCaptureError(
                    "protocol_error", "initialize result was missing"
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
                },
            }
        )
        while not (thread_start_response_received and thread_started_received):
            message = receive()
            if message.get("id") == THREAD_START_REQUEST_ID:
                if "error" in message:
                    raise CodexAppServerCaptureError(
                        "protocol_error", "thread/start returned a JSON-RPC error"
                    )
                result = message.get("result")
                thread = result.get("thread") if isinstance(result, dict) else None
                response_thread_id = (
                    thread.get("id") if isinstance(thread, dict) else None
                )
                if not isinstance(response_thread_id, str) or not response_thread_id:
                    raise CodexAppServerCaptureError(
                        "protocol_error", "thread/start response lacks thread.id"
                    )
                if thread.get("ephemeral") is not True:
                    raise CodexAppServerCaptureError(
                        "protocol_error", "thread/start response is not ephemeral"
                    )
                thread_start_response_received = True
            elif message.get("method") == "thread/started":
                params = message.get("params")
                thread = params.get("thread") if isinstance(params, dict) else None
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
                "protocol_error", "thread response and notification ids differ"
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
    success = (
        error_kind is None
        and initialize_response_received
        and initialized_sent
        and thread_start_response_received
        and thread_started_received
        and response_thread_id == notification_thread_id
    )
    receipt = {
        "schema_version": "0.1.0",
        "artifact_kind": "codex-app-server-lifecycle-capture",
        "capture_id": capture_id,
        "supervisor_id": CAPTURE_SUPERVISOR_ID,
        "valid": success,
        "capture_status": "captured" if success else "failed",
        "runtime_family": RUNTIME_FAMILY,
        "runtime_version": runtime_version,
        "source_stream_id": stream_id,
        "started_at": started_at,
        "finished_at": utc_now(),
        "command_sha256": sha256_bytes(canonical_bytes(command_list)),
        "working_directory": str(resolved_cwd),
        "isolated_state_directory": str(state_dir) if state_dir is not None else None,
        "process_id": process.pid if process is not None else None,
        "process_exit_code": process.poll() if process is not None else None,
        "shutdown_method": shutdown_method,
        "containment_kind": containment.kind,
        "containment_assigned": containment.assigned,
        "handshake": {
            "initialize_response_received": initialize_response_received,
            "initialized_sent": initialized_sent,
            "thread_start_response_received": thread_start_response_received,
            "thread_started_received": thread_started_received,
            "thread_id": (
                response_thread_id
                if response_thread_id == notification_thread_id
                else None
            ),
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
    validate_instance(receipt, CAPTURE_RECEIPT_SCHEMA, label="capture receipt")
    atomic_write_json(receipt_path, receipt)
    return receipt
