from __future__ import annotations

import ctypes
from ctypes import wintypes
import hashlib
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import threading
import time
from typing import Any, BinaryIO, Sequence
import uuid

from .errors import PrototypeError
from .io_utils import atomic_write_json, canonical_bytes, sha256_bytes, utc_now
from .layout import RUNTIME_ROOT
from .schema import validate_instance


JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS = 9


class _IOCounters(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_ulonglong),
        ("WriteOperationCount", ctypes.c_ulonglong),
        ("OtherOperationCount", ctypes.c_ulonglong),
        ("ReadTransferCount", ctypes.c_ulonglong),
        ("WriteTransferCount", ctypes.c_ulonglong),
        ("OtherTransferCount", ctypes.c_ulonglong),
    ]


class _BasicLimitInformation(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_longlong),
        ("PerJobUserTimeLimit", ctypes.c_longlong),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class _ExtendedLimitInformation(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _BasicLimitInformation),
        ("IoInfo", _IOCounters),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class _WindowsJob:
    def __init__(self, handle: int) -> None:
        self.handle = handle
        self.closed = False

    @classmethod
    def create_kill_on_close(cls) -> "_WindowsJob":
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
        kernel32.CreateJobObjectW.restype = wintypes.HANDLE
        kernel32.SetInformationJobObject.argtypes = [
            wintypes.HANDLE,
            ctypes.c_int,
            wintypes.LPVOID,
            wintypes.DWORD,
        ]
        kernel32.SetInformationJobObject.restype = wintypes.BOOL
        handle = kernel32.CreateJobObjectW(None, None)
        if not handle:
            raise ctypes.WinError(ctypes.get_last_error())
        job = cls(int(handle))
        limits = _ExtendedLimitInformation()
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        succeeded = kernel32.SetInformationJobObject(
            wintypes.HANDLE(job.handle),
            JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
            ctypes.byref(limits),
            ctypes.sizeof(limits),
        )
        if not succeeded:
            error = ctypes.WinError(ctypes.get_last_error())
            job.close()
            raise error
        return job

    def assign(self, process_handle: int) -> None:
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
        kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
        succeeded = kernel32.AssignProcessToJobObject(
            wintypes.HANDLE(self.handle), wintypes.HANDLE(process_handle)
        )
        if not succeeded:
            raise ctypes.WinError(ctypes.get_last_error())

    def close(self) -> None:
        if self.closed:
            return
        kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel32.CloseHandle.restype = wintypes.BOOL
        kernel32.CloseHandle(wintypes.HANDLE(self.handle))
        self.closed = True


class _Capture:
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

    def record(self) -> dict[str, Any]:
        return {
            "total_bytes": self.total,
            "retained_bytes": len(self.retained),
            "sha256": self.digest.hexdigest(),
            "truncated": self.total > len(self.retained),
            "content_in_report": False,
        }


def _validate_launch(
    command: Sequence[str], cwd: Path, timeout_seconds: float, capture_limit: int
) -> tuple[list[str], Path]:
    if os.name != "nt":
        raise PrototypeError("Windows process spike can run only on Windows")
    if not command or not all(isinstance(item, str) and item for item in command):
        raise PrototypeError("command must be a non-empty sequence of non-empty strings")
    executable = Path(command[0])
    if not executable.is_absolute() or not executable.is_file():
        raise PrototypeError("Windows process executable must be an existing absolute file")
    resolved_cwd = cwd.resolve(strict=True)
    if not resolved_cwd.is_dir():
        raise PrototypeError("process cwd must be a directory")
    if timeout_seconds <= 0:
        raise PrototypeError("timeout must be positive")
    if capture_limit < 0:
        raise PrototypeError("capture limit cannot be negative")
    return [str(executable.resolve()), *command[1:]], resolved_cwd


def _terminal_event(state: str, exit_code: int | None, reason_codes: list[str]) -> dict[str, Any]:
    return {
        "event_kind": "terminal",
        "event_type": f"process.{state}",
        "terminal_state": state,
        "exit_code": exit_code,
        "reason_codes": reason_codes,
    }


def run_windows_process(
    command: Sequence[str],
    *,
    cwd: Path,
    timeout_seconds: float,
    cancel_grace_seconds: float = 1.0,
    capture_limit_bytes: int = 1024 * 1024,
    require_job_object: bool = True,
) -> dict[str, Any]:
    command_list, resolved_cwd = _validate_launch(
        command, cwd, timeout_seconds, capture_limit_bytes
    )
    if cancel_grace_seconds <= 0:
        raise PrototypeError("cancel grace period must be positive")

    run_id = f"WPROC-{uuid.uuid4()}".upper()
    started_at = utc_now()
    started_monotonic = time.monotonic()
    stdout_capture = _Capture(capture_limit_bytes)
    stderr_capture = _Capture(capture_limit_bytes)
    job: _WindowsJob | None = None
    job_created = False
    job_assigned = False
    degraded_reason: str | None = None
    cancellation_requested = False
    cancellation_trigger = "none"
    ctrl_break_attempted = False
    ctrl_break_error: str | None = None
    escalation = "none"
    process: subprocess.Popen[bytes] | None = None
    reader_threads: list[threading.Thread] = []

    try:
        try:
            job = _WindowsJob.create_kill_on_close()
            job_created = True
        except OSError as exc:
            degraded_reason = f"job creation failed: {exc}"
            if require_job_object:
                state = "failed"
                report = _build_report(
                    run_id=run_id,
                    started_at=started_at,
                    started_monotonic=started_monotonic,
                    command=command_list,
                    cwd=resolved_cwd,
                    pid=None,
                    exit_code=None,
                    state=state,
                    timed_out=False,
                    require_job_object=require_job_object,
                    job_created=False,
                    job_assigned=False,
                    degraded_reason=degraded_reason,
                    stdout_capture=stdout_capture,
                    stderr_capture=stderr_capture,
                    capture_limit=capture_limit_bytes,
                    cancellation_requested=False,
                    cancellation_trigger="none",
                    ctrl_break_attempted=False,
                    ctrl_break_error=None,
                    escalation="none",
                    reason_codes=["WIN-JOB-CREATE-001"],
                    runtime_ready=False,
                )
                return report

        creation_flags = subprocess.CREATE_NEW_PROCESS_GROUP | subprocess.CREATE_NO_WINDOW
        try:
            process = subprocess.Popen(
                command_list,
                cwd=resolved_cwd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                shell=False,
                creationflags=creation_flags,
                close_fds=True,
            )
        except OSError as exc:
            degraded_reason = degraded_reason or f"process spawn failed: {exc}"
            return _build_report(
                run_id=run_id,
                started_at=started_at,
                started_monotonic=started_monotonic,
                command=command_list,
                cwd=resolved_cwd,
                pid=None,
                exit_code=None,
                state="failed",
                timed_out=False,
                require_job_object=require_job_object,
                job_created=job_created,
                job_assigned=False,
                degraded_reason=degraded_reason,
                stdout_capture=stdout_capture,
                stderr_capture=stderr_capture,
                capture_limit=capture_limit_bytes,
                cancellation_requested=False,
                cancellation_trigger="none",
                ctrl_break_attempted=False,
                ctrl_break_error=None,
                escalation="none",
                reason_codes=["WIN-PROC-SPAWN-001"],
                runtime_ready=False,
            )

        assert process.stdout is not None and process.stderr is not None
        for name, capture, stream in (
            ("stdout", stdout_capture, process.stdout),
            ("stderr", stderr_capture, process.stderr),
        ):
            thread = threading.Thread(
                target=capture.drain,
                args=(stream,),
                name=f"windows-process-{name}",
                daemon=True,
            )
            thread.start()
            reader_threads.append(thread)

        if job is not None:
            try:
                process_handle = int(getattr(process, "_handle"))
                job.assign(process_handle)
                job_assigned = True
            except (AttributeError, OSError, TypeError, ValueError) as exc:
                degraded_reason = f"job assignment failed: {exc}"
                if require_job_object:
                    cancellation_requested = True
                    cancellation_trigger = "containment_failure"
                    process.terminate()
                    escalation = "terminate_root"

        timed_out = False
        if cancellation_trigger == "containment_failure":
            try:
                process.wait(timeout=cancel_grace_seconds)
            except subprocess.TimeoutExpired:
                process.kill()
                escalation = "kill_root"
                process.wait(timeout=cancel_grace_seconds)
        else:
            try:
                process.wait(timeout=timeout_seconds)
            except subprocess.TimeoutExpired:
                timed_out = True
                cancellation_requested = True
                cancellation_trigger = "timeout"
                ctrl_break_attempted = True
                try:
                    process.send_signal(signal.CTRL_BREAK_EVENT)
                except OSError as exc:
                    ctrl_break_error = str(exc)
                try:
                    process.wait(timeout=cancel_grace_seconds)
                except subprocess.TimeoutExpired:
                    if job is not None and job_assigned:
                        job.close()
                        escalation = "job_close"
                    else:
                        process.terminate()
                        escalation = "terminate_root"
                    try:
                        process.wait(timeout=cancel_grace_seconds)
                    except subprocess.TimeoutExpired:
                        process.kill()
                        escalation = "kill_root"
                        process.wait(timeout=cancel_grace_seconds)

        if job is not None and not job.closed:
            job.close()
        for thread in reader_threads:
            thread.join(timeout=5)

        exit_code = process.returncode
        if cancellation_trigger == "timeout":
            state = "cancelled"
            reason_codes = ["PROC-TIMEOUT-001"]
        elif cancellation_trigger == "containment_failure":
            state = "failed"
            reason_codes = ["WIN-JOB-ASSIGN-001"]
        elif exit_code is None:
            state = "unknown"
            reason_codes = ["PROC-EXIT-UNKNOWN-001"]
        elif exit_code == 0:
            state = "succeeded"
            reason_codes = []
        else:
            state = "failed"
            reason_codes = ["PROC-EXIT-NONZERO-001"]

        capture_errors = [
            error for error in (stdout_capture.error, stderr_capture.error) if error
        ]
        if capture_errors:
            reason_codes.append("WIN-PIPE-DRAIN-001")
        runtime_ready = (
            not capture_errors
            and (job_assigned or not require_job_object)
            and cancellation_trigger != "containment_failure"
        )
        return _build_report(
            run_id=run_id,
            started_at=started_at,
            started_monotonic=started_monotonic,
            command=command_list,
            cwd=resolved_cwd,
            pid=process.pid,
            exit_code=exit_code,
            state=state,
            timed_out=timed_out,
            require_job_object=require_job_object,
            job_created=job_created,
            job_assigned=job_assigned,
            degraded_reason=degraded_reason,
            stdout_capture=stdout_capture,
            stderr_capture=stderr_capture,
            capture_limit=capture_limit_bytes,
            cancellation_requested=cancellation_requested,
            cancellation_trigger=cancellation_trigger,
            ctrl_break_attempted=ctrl_break_attempted,
            ctrl_break_error=ctrl_break_error,
            escalation=escalation,
            reason_codes=reason_codes,
            runtime_ready=runtime_ready,
        )
    finally:
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=max(cancel_grace_seconds, 1.0))
        if job is not None and not job.closed:
            job.close()


def _build_report(
    *,
    run_id: str,
    started_at: str,
    started_monotonic: float,
    command: list[str],
    cwd: Path,
    pid: int | None,
    exit_code: int | None,
    state: str,
    timed_out: bool,
    require_job_object: bool,
    job_created: bool,
    job_assigned: bool,
    degraded_reason: str | None,
    stdout_capture: _Capture,
    stderr_capture: _Capture,
    capture_limit: int,
    cancellation_requested: bool,
    cancellation_trigger: str,
    ctrl_break_attempted: bool,
    ctrl_break_error: str | None,
    escalation: str,
    reason_codes: list[str],
    runtime_ready: bool,
) -> dict[str, Any]:
    command_sha = sha256_bytes(canonical_bytes(command))
    arguments_sha = sha256_bytes(canonical_bytes(command[1:]))
    report = {
        "schema_version": "0.1.0-prototype",
        "runner": "windows-process-spike",
        "run_id": run_id,
        "valid": True,
        "runtime_ready": runtime_ready,
        "started_at": started_at,
        "completed_at": utc_now(),
        "duration_ms": round((time.monotonic() - started_monotonic) * 1000, 3),
        "command": {
            "executable": command[0],
            "argument_count": len(command) - 1,
            "arguments_sha256": arguments_sha,
            "command_sha256": command_sha,
            "cwd": str(cwd),
            "shell": False,
        },
        "process": {
            "pid": pid,
            "exit_code": exit_code,
            "terminal_state": state,
            "timed_out": timed_out,
        },
        "containment": {
            "required": require_job_object,
            "job_object_created": job_created,
            "job_object_assigned": job_assigned,
            "kill_on_close": job_created,
            "degraded_reason": degraded_reason,
        },
        "capture": {
            "limit_bytes_per_stream": capture_limit,
            "stdout": stdout_capture.record(),
            "stderr": stderr_capture.record(),
        },
        "cancellation": {
            "requested": cancellation_requested,
            "trigger": cancellation_trigger,
            "ctrl_break_attempted": ctrl_break_attempted,
            "ctrl_break_error": ctrl_break_error,
            "escalation": escalation,
        },
        "terminal_event": _terminal_event(state, exit_code, sorted(set(reason_codes))),
        "limitations": [
            "Popen starts the process before Job Object assignment, leaving a small containment race in this spike.",
            "This is process-control evidence only and does not validate scientific outputs or provide an OS sandbox.",
            f"Host platform: {platform.platform()}.",
        ],
    }
    validate_instance(
        report,
        RUNTIME_ROOT / "windows-process-result-v0.1.schema.json",
        label="Windows process result",
    )
    return report


def create_windows_process_smoke(*, output_dir: Path) -> dict[str, Any]:
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing smoke root: {output_dir}")
    output_dir.mkdir(parents=True)
    command = [
        str(Path(sys.executable).resolve()),
        "-c",
        "import sys; print('windows-process-smoke'); print('stderr-smoke', file=sys.stderr)",
    ]
    report = run_windows_process(command, cwd=output_dir, timeout_seconds=15)
    atomic_write_json(output_dir / "windows-process-result.json", report)
    return report
