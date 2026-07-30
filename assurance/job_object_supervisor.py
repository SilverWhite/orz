from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
import subprocess
from typing import Any

from .errors import AssuranceError


JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS = 9

PROCESS_SET_QUOTA = 0x0100
PROCESS_TERMINATE = 0x0001
PROCESS_QUERY_INFORMATION = 0x0400
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000


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


def _create_kill_on_close_job() -> wintypes.HANDLE | None:
    """Create a Job Object with JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE.

    Returns None if Job Object creation fails.  Caller decides whether to
    treat this as a fail-closed error or a degraded path.
    """
    if os.name != "nt":
        return None
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
        return None
    limits = _ExtendedLimitInformation()
    limits.BasicLimitInformation.LimitFlags = wintypes.DWORD(
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    )
    limits.JobMemoryLimit = 0
    succeeded = kernel32.SetInformationJobObject(
        wintypes.HANDLE(handle),
        JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
        ctypes.byref(limits),
        ctypes.sizeof(limits),
    )
    if not succeeded:
        kernel32.CloseHandle(wintypes.HANDLE(handle))
        return None
    return wintypes.HANDLE(handle)


def _open_process(pid: int) -> wintypes.HANDLE | None:
    """Open a process handle for Job Object assignment.

    Requires PROCESS_SET_QUOTA (AssignProcessToJobObject),
    PROCESS_TERMINATE (kill-on-close), and PROCESS_QUERY_INFORMATION
    (status checks).  Falls back to PROCESS_QUERY_LIMITED_INFORMATION
    when PROCESS_QUERY_INFORMATION is denied.
    """
    if os.name != "nt":
        return None
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel32.OpenProcess.restype = wintypes.HANDLE

    desired = wintypes.DWORD(
        PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_INFORMATION
    )
    handle = kernel32.OpenProcess(desired, False, wintypes.DWORD(pid))
    if handle:
        return wintypes.HANDLE(handle)

    desired = wintypes.DWORD(
        PROCESS_SET_QUOTA | PROCESS_TERMINATE | PROCESS_QUERY_LIMITED_INFORMATION
    )
    handle = kernel32.OpenProcess(desired, False, wintypes.DWORD(pid))
    if handle:
        return wintypes.HANDLE(handle)

    desired = wintypes.DWORD(PROCESS_SET_QUOTA | PROCESS_TERMINATE)
    handle = kernel32.OpenProcess(desired, False, wintypes.DWORD(pid))
    if handle:
        return wintypes.HANDLE(handle)

    return None


def _assign_process_to_job(
    job: wintypes.HANDLE, process_handle: wintypes.HANDLE
) -> bool:
    """Assign a process handle to a Job Object."""
    if os.name != "nt":
        return False
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
    return bool(kernel32.AssignProcessToJobObject(job, process_handle))


def _close_handle(handle: wintypes.HANDLE) -> None:
    """Close a Windows kernel handle.  No-op on null handles."""
    if not handle:
        return
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32.CloseHandle.restype = wintypes.BOOL
    kernel32.CloseHandle(handle)


# ---------------------------------------------------------------------------
# CREATE_SUSPENDED + resume helpers
# ---------------------------------------------------------------------------

CREATE_SUSPENDED = 0x00000004
TH32CS_SNAPTHREAD = 0x00000004
THREAD_SUSPEND_RESUME = 0x0002
INVALID_HANDLE_VALUE = wintypes.HANDLE(-1).value


class _THREADENTRY32(ctypes.Structure):
    _fields_ = [
        ("dwSize", wintypes.DWORD),
        ("cntUsage", wintypes.DWORD),
        ("th32ThreadID", wintypes.DWORD),
        ("th32OwnerProcessID", wintypes.DWORD),
        ("tpBasePri", wintypes.LONG),
        ("tpDeltaPri", wintypes.LONG),
        ("dwFlags", wintypes.DWORD),
    ]


def _resume_main_thread(pid: int) -> None:
    """Resume the main thread of a process created with ``CREATE_SUSPENDED``.

    A newly-created suspended process has exactly one thread (the main
    thread).  We snapshot all threads, pick the one owned by *pid*, open
    it with ``THREAD_SUSPEND_RESUME``, call ``ResumeThread``, and close
    the thread handle.

    Raises :class:`AssuranceError` when the thread cannot be found,
    opened, or resumed (fail-closed — a hung suspended process is a
    resource leak).
    """
    if os.name != "nt":
        return

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

    kernel32.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    kernel32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE

    kernel32.Thread32First.argtypes = [wintypes.HANDLE, ctypes.POINTER(_THREADENTRY32)]
    kernel32.Thread32First.restype = wintypes.BOOL

    kernel32.Thread32Next.argtypes = [wintypes.HANDLE, ctypes.POINTER(_THREADENTRY32)]
    kernel32.Thread32Next.restype = wintypes.BOOL

    kernel32.OpenThread.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel32.OpenThread.restype = wintypes.HANDLE

    kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
    kernel32.ResumeThread.restype = wintypes.DWORD

    h_snapshot = kernel32.CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0)
    if h_snapshot == INVALID_HANDLE_VALUE:
        raise AssuranceError(
            f"cannot create thread snapshot to resume process {pid}"
        )

    try:
        te = _THREADENTRY32()
        te.dwSize = ctypes.sizeof(_THREADENTRY32)

        if not kernel32.Thread32First(h_snapshot, ctypes.byref(te)):
            raise AssuranceError(
                f"cannot enumerate threads to resume process {pid}"
            )

        while True:
            if te.th32OwnerProcessID == pid:
                h_thread = kernel32.OpenThread(
                    wintypes.DWORD(THREAD_SUSPEND_RESUME), False,
                    wintypes.DWORD(te.th32ThreadID),
                )
                if not h_thread:
                    raise AssuranceError(
                        f"cannot open thread {te.th32ThreadID} "
                        f"to resume process {pid}"
                    )
                try:
                    # ResumeThread returns the previous suspend count.
                    # A value of 1 is expected for CREATE_SUSPENDED.
                    kernel32.ResumeThread(h_thread)
                finally:
                    _close_handle(h_thread)
                return

            if not kernel32.Thread32Next(h_snapshot, ctypes.byref(te)):
                break

        raise AssuranceError(
            f"no thread found for process {pid} — "
            f"cannot resume suspended process"
        )
    finally:
        _close_handle(h_snapshot)


class JobObjectSupervisor:
    """A Kill-On-Close Job Object that contains a root process tree.

    Creates a Windows Job Object with ``JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE``
    set.  When the supervisor closes the Job Object handle the kernel
    terminates every process that was assigned to the job — root and all
    descendants.  No breakaway flags are set; child processes inherit the
    Job constraint automatically.

    This is a thin supervision layer, not a full sandbox.  It provides a
    containment guarantee that the adapter can offer even when the
    upstream runtime (Grok) has not yet proven internal child-process
    cleanup.

    The :func:`contained_run` helper uses ``CREATE_SUSPENDED`` +
    assign + resume to close the post-creation race window: no code
    executes before Job membership is established.
    ``PROC_THREAD_ATTRIBUTE_JOB_LIST`` (GAK-WIN-001 creation-time
    kernel assignment) remains the planned refinement for a cleaner
    implementation that avoids the suspend/resume dance, but the
    effective containment guarantee is equivalent.
    """

    def __init__(self) -> None:
        """Create a Kill-On-Close Job Object.

        Raises :class:`AssuranceError` when the Job Object cannot be
        created (fail-closed — the adapter must not launch without
        containment on Windows).
        """
        self._job: wintypes.HANDLE | None = None
        self._assigned: bool = False

        if os.name != "nt":
            return

        job = _create_kill_on_close_job()
        if not job:
            raise AssuranceError(
                "cannot create Kill-On-Close Job Object for adapter containment"
            )
        self._job = job

    # -----------------------------------------------------------------
    # Public API
    # -----------------------------------------------------------------

    @property
    def is_active(self) -> bool:
        """``True`` when a Job Object was created (Windows only)."""
        return self._job is not None

    @property
    def is_assigned(self) -> bool:
        """``True`` when at least one process was assigned to the Job."""
        return self._assigned

    def assign_process(self, pid: int) -> None:
        """Assign a running process (by *pid*) to the Job Object.

        Opens the process handle with the required access rights and
        calls ``AssignProcessToJobObject``.  Must be called immediately
        after the process is created, before it spawns child processes.

        Raises :class:`AssuranceError` when the process cannot be opened
        or assigned (fail-closed).
        """
        if self._job is None:
            return
        handle = _open_process(pid)
        if not handle:
            raise AssuranceError(
                f"cannot open process {pid} for Job Object assignment"
            )
        try:
            if not _assign_process_to_job(self._job, handle):
                raise AssuranceError(
                    f"cannot assign process {pid} to Kill-On-Close Job Object"
                )
            self._assigned = True
        finally:
            _close_handle(handle)

    def close(self) -> None:
        """Close the Job Object handle.

        Closing a Kill-On-Close Job Object causes the kernel to
        terminate every process still associated with the job.
        """
        if self._job is not None:
            _close_handle(self._job)
            self._job = None

    def __enter__(self) -> "JobObjectSupervisor":
        return self

    def __exit__(self, *args: object) -> None:
        self.close()


def run_with_job_object_containment(
    command: list[str],
    *,
    cwd: str,
    env: dict[str, str] | None = None,
    timeout: int = 30,
    popen_factory: Any = subprocess.Popen,
) -> subprocess.Popen[bytes]:
    """Launch *command* inside a Kill-On-Close Job Object.

    Convenience wrapper that creates a :class:`JobObjectSupervisor`,
    starts the process via *popen_factory*, assigns it to the Job, and
    returns the :class:`~subprocess.Popen` instance.  The caller **must**
    close the supervisor (or use it as a context manager) to release
    the Job Object handle.

    Returns the :class:`~subprocess.Popen` instance along with the
    supervisor attached as ``_job_supervisor``.

    Raises :class:`AssuranceError` on containment failure.
    """
    supervisor = JobObjectSupervisor()
    process = popen_factory(
        command,
        cwd=cwd,
        env=env,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
    )
    if supervisor.is_active:
        supervisor.assign_process(process.pid)
    process._job_supervisor = supervisor  # type: ignore[attr-defined]
    return process


# For test injection — tests can patch contained_run with a no-op that
# delegates to plain subprocess.run.
CONTAINMENT_ENABLED = True


def contained_run(
    command: list[str],
    *,
    cwd: Path | str,
    timeout: int,
    env: dict[str, str] | None = None,
    text: bool = False,
    encoding: str = "utf-8",
    errors: str = "replace",
    stdin: int | None = None,
) -> subprocess.CompletedProcess[bytes] | subprocess.CompletedProcess[str]:
    """Run *command* inside a Kill-On-Close Job Object.

    Drop-in replacement for :func:`subprocess.run` that wraps every
    subprocess call in a :class:`JobObjectSupervisor`.  The Job handle
    is closed in a ``finally`` block, so even if the command hangs or
    the caller's timeout kills the Python thread, the kernel guarantees
    the entire process tree is terminated.

    On Windows the process is created with ``CREATE_SUSPENDED``,
    assigned to the Job Object while frozen, then resumed.  No code
    executes before Job membership is established — the post-creation
    race window is closed.

    On non-Windows platforms this is a thin passthrough to
    :func:`subprocess.run`.
    """
    if not CONTAINMENT_ENABLED or os.name != "nt":
        return subprocess.run(
            command,
            cwd=cwd,
            env=env,
            stdin=subprocess.DEVNULL if stdin is None else stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=text,
            encoding=encoding,
            errors=errors,
            timeout=timeout,
            check=False,
            creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        )

    CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)
    supervisor = JobObjectSupervisor()
    process: subprocess.Popen[bytes] | subprocess.Popen[str] | None = None
    try:
        # CREATE_SUSPENDED: the process is created but its initial thread
        # is frozen.  We assign it to the Job Object and resume — no code
        # executes before containment is established.
        process = subprocess.Popen(
            command,
            cwd=cwd,
            env=env,
            stdin=subprocess.DEVNULL if stdin is None else stdin,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=text,
            encoding=encoding,
            errors=errors,
            creationflags=CREATE_NO_WINDOW | CREATE_SUSPENDED,
        )
        supervisor.assign_process(process.pid)
        _resume_main_thread(process.pid)

        stdout_bytes, stderr_bytes = process.communicate(timeout=timeout)
        returncode = process.poll()
        if returncode is None:
            process.kill()
            process.wait(timeout=5)
            returncode = process.poll() or -1
    except subprocess.TimeoutExpired:
        if process is not None:
            process.kill()
            try:
                stdout_bytes, stderr_bytes = process.communicate(timeout=5)
                returncode = process.poll() or -1
            except subprocess.TimeoutExpired:
                stdout_bytes, stderr_bytes = (b"", b"")
                returncode = -1
    finally:
        if supervisor is not None:
            supervisor.close()
        if process is not None:
            try:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)
            except (OSError, subprocess.TimeoutExpired):
                pass

    return subprocess.CompletedProcess(
        args=command,
        returncode=returncode,
        stdout=stdout_bytes,
        stderr=stderr_bytes,
    )
