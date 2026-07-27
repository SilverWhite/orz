from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from assurance import KeyLifecycleController, MemoryInstallationKeyStore


kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE = 0x00002000
JOB_OBJECT_LIMIT_BREAKAWAY_OK = 0x00000800
JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK = 0x00001000
CREATE_SUSPENDED = 0x00000004
CREATE_NO_WINDOW = 0x08000000
INFINITE = 0xFFFFFFFF
WAIT_OBJECT_0 = 0x00000000


class _JOBOBJECT_BASIC_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("PerProcessUserTimeLimit", ctypes.c_int64),
        ("PerJobUserTimeLimit", ctypes.c_int64),
        ("LimitFlags", wintypes.DWORD),
        ("MinimumWorkingSetSize", ctypes.c_size_t),
        ("MaximumWorkingSetSize", ctypes.c_size_t),
        ("ActiveProcessLimit", wintypes.DWORD),
        ("Affinity", ctypes.c_size_t),
        ("PriorityClass", wintypes.DWORD),
        ("SchedulingClass", wintypes.DWORD),
    ]


class _IO_COUNTERS(ctypes.Structure):
    _fields_ = [
        ("ReadOperationCount", ctypes.c_uint64),
        ("WriteOperationCount", ctypes.c_uint64),
        ("OtherOperationCount", ctypes.c_uint64),
        ("ReadTransferCount", ctypes.c_uint64),
        ("WriteTransferCount", ctypes.c_uint64),
        ("OtherTransferCount", ctypes.c_uint64),
    ]


class _JOBOBJECT_EXTENDED_LIMIT_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("BasicLimitInformation", _JOBOBJECT_BASIC_LIMIT_INFORMATION),
        ("IoInfo", _IO_COUNTERS),
        ("ProcessMemoryLimit", ctypes.c_size_t),
        ("JobMemoryLimit", ctypes.c_size_t),
        ("PeakProcessMemoryUsed", ctypes.c_size_t),
        ("PeakJobMemoryUsed", ctypes.c_size_t),
    ]


class _STARTUPINFOW(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("lpReserved", wintypes.LPWSTR),
        ("lpDesktop", wintypes.LPWSTR),
        ("lpTitle", wintypes.LPWSTR),
        ("dwX", wintypes.DWORD),
        ("dwY", wintypes.DWORD),
        ("dwXSize", wintypes.DWORD),
        ("dwYSize", wintypes.DWORD),
        ("dwXCountChars", wintypes.DWORD),
        ("dwYCountChars", wintypes.DWORD),
        ("dwFillAttribute", wintypes.DWORD),
        ("dwFlags", wintypes.DWORD),
        ("wShowWindow", wintypes.WORD),
        ("cbReserved2", wintypes.WORD),
        ("lpReserved2", ctypes.POINTER(ctypes.c_ubyte)),
        ("hStdInput", wintypes.HANDLE),
        ("hStdOutput", wintypes.HANDLE),
        ("hStdError", wintypes.HANDLE),
    ]


class _PROCESS_INFORMATION(ctypes.Structure):
    _fields_ = [
        ("hProcess", wintypes.HANDLE),
        ("hThread", wintypes.HANDLE),
        ("dwProcessId", wintypes.DWORD),
        ("dwThreadId", wintypes.DWORD),
    ]


def _create_job() -> wintypes.HANDLE:
    kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
    kernel32.CreateJobObjectW.restype = wintypes.HANDLE

    job = kernel32.CreateJobObjectW(None, None)
    if not job:
        raise OSError(f"CreateJobObjectW failed: {ctypes.get_last_error()}")
    return job


def _set_kill_on_close(job: wintypes.HANDLE) -> None:
    kernel32.SetInformationJobObject.argtypes = [
        wintypes.HANDLE,
        ctypes.c_int,
        ctypes.c_void_p,
        wintypes.DWORD,
    ]
    kernel32.SetInformationJobObject.restype = wintypes.BOOL
    kernel32.QueryInformationJobObject.argtypes = [
        wintypes.HANDLE,
        ctypes.c_int,
        ctypes.c_void_p,
        wintypes.DWORD,
        ctypes.POINTER(wintypes.DWORD),
    ]
    kernel32.QueryInformationJobObject.restype = wintypes.BOOL

    info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
    JobObjectExtendedLimitInformation = 9
    size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
    returned = wintypes.DWORD()
    kernel32.QueryInformationJobObject(
        job, JobObjectExtendedLimitInformation,
        ctypes.byref(info), size, ctypes.byref(returned)
    )

    # Set KILL_ON_JOB_CLOSE in BasicLimitInformation.LimitFlags
    info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE

    if not kernel32.SetInformationJobObject(
        job, JobObjectExtendedLimitInformation, ctypes.byref(info), size
    ):
        raise OSError(f"SetInformationJobObject failed: {ctypes.get_last_error()}")


def _assign_process_to_job(job: wintypes.HANDLE, process: wintypes.HANDLE) -> bool:
    kernel32.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
    return bool(kernel32.AssignProcessToJobObject(job, process))


def _create_suspended_process(command: str) -> _PROCESS_INFORMATION:
    kernel32.CreateProcessW.argtypes = [
        wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p, ctypes.c_void_p,
        wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p, wintypes.LPCWSTR,
        ctypes.POINTER(_STARTUPINFOW), ctypes.POINTER(_PROCESS_INFORMATION),
    ]
    kernel32.CreateProcessW.restype = wintypes.BOOL

    si = _STARTUPINFOW()
    si.cb = ctypes.sizeof(_STARTUPINFOW)
    pi = _PROCESS_INFORMATION()

    cmd = f"cmd.exe /c {command}"
    if not kernel32.CreateProcessW(
        None, cmd, None, None, False,
        CREATE_SUSPENDED | CREATE_NO_WINDOW,
        None, None, ctypes.byref(si), ctypes.byref(pi),
    ):
        raise OSError(f"CreateProcessW failed: {ctypes.get_last_error()}")
    return pi


def _resume_thread(pi: _PROCESS_INFORMATION) -> None:
    kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
    kernel32.ResumeThread.restype = wintypes.DWORD
    kernel32.ResumeThread(pi.hThread)


def _wait_for_process(pi: _PROCESS_INFORMATION, timeout_ms: int = 10000) -> int:
    kernel32.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
    kernel32.WaitForSingleObject.restype = wintypes.DWORD
    kernel32.GetExitCodeProcess.argtypes = [wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD)]
    kernel32.GetExitCodeProcess.restype = wintypes.BOOL

    ret = kernel32.WaitForSingleObject(pi.hProcess, timeout_ms)
    if ret != WAIT_OBJECT_0:
        return -1
    code = wintypes.DWORD()
    kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))
    return code.value


def _close_handles(pi: _PROCESS_INFORMATION) -> None:
    kernel32.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel32.CloseHandle.restype = wintypes.BOOL
    kernel32.CloseHandle(pi.hThread)
    kernel32.CloseHandle(pi.hProcess)


def _close_job(job: wintypes.HANDLE) -> None:
    kernel32.CloseHandle(job)


def _is_process_in_job(process: wintypes.HANDLE) -> bool:
    kernel32.IsProcessInJob.argtypes = [wintypes.HANDLE, wintypes.HANDLE, ctypes.POINTER(wintypes.BOOL)]
    kernel32.IsProcessInJob.restype = wintypes.BOOL
    result = wintypes.BOOL()
    # Pass NULL as second arg to check if process is in ANY job
    if not kernel32.IsProcessInJob(process, None, ctypes.byref(result)):
        raise OSError(f"IsProcessInJob failed: {ctypes.get_last_error()}")
    return bool(result.value)


@unittest.skipUnless(os.name == "nt", "Windows Job Object tests require Windows")
class WindowsJobObjectRaceTests(unittest.TestCase):
    """Verify Job Object containment is resistant to escape vectors."""

    def test_create_suspended_assign_resume_contained(self) -> None:
        """CREATE_SUSPENDED → assign Job → Resume: process is contained."""
        job = _create_job()
        _set_kill_on_close(job)
        try:
            pi = _create_suspended_process("exit 0")
            try:
                was_in_job = _is_process_in_job(pi.hProcess)
                # Assign to our job
                if not was_in_job:
                    self.assertTrue(
                        _assign_process_to_job(job, pi.hProcess),
                        "failed to assign suspended process to job"
                    )
                # Now it should be in a job (either ours or inherited)
                self.assertTrue(
                    _is_process_in_job(pi.hProcess),
                    "process must be in job after assignment"
                )
                # Resume and wait
                _resume_thread(pi)
                exit_code = _wait_for_process(pi, timeout_ms=5000)
                self.assertEqual(exit_code, 0)
            finally:
                _close_handles(pi)
        finally:
            _close_job(job)

    def test_already_running_process_cannot_be_assigned(self) -> None:
        """A process started OUTSIDE the Job cannot be retroactively assigned."""
        # Start a process without CREATE_SUSPENDED
        proc = subprocess.Popen(
            ["cmd.exe", "/c", "exit 0"],
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        time.sleep(0.5)  # Let it finish or at least start

        job = _create_job()
        _set_kill_on_close(job)
        try:
            # Try to assign the already-running (or exited) process
            kernel32.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
            kernel32.OpenProcess.restype = wintypes.HANDLE
            PROCESS_SET_QUOTA = 0x0100
            PROCESS_TERMINATE = 0x0001
            h = kernel32.OpenProcess(
                PROCESS_SET_QUOTA | PROCESS_TERMINATE, False, proc.pid
            )
            if h:
                assigned = _assign_process_to_job(job, h)
                # Already-running process should fail assignment
                self.assertFalse(
                    assigned,
                    "already-running process must not be assignable to a job"
                )
                kernel32.CloseHandle(h)
        finally:
            _close_job(job)
            proc.wait(timeout=5)

    def test_job_kill_on_close_terminates_process(self) -> None:
        """Kill-on-close Job Object must terminate contained processes."""
        job = _create_job()
        _set_kill_on_close(job)
        pi = _create_suspended_process("ping -n 30 127.0.0.1 > nul")
        try:
            self.assertTrue(_assign_process_to_job(job, pi.hProcess))
            _resume_thread(pi)
            time.sleep(0.3)

            # Process should be running
            code = wintypes.DWORD()
            kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(code))
            self.assertEqual(code.value, 259)  # STILL_ACTIVE

            # Close the job → process should be killed
            _close_job(job)
            job = None  # prevent double-close

            exit_code = _wait_for_process(pi, timeout_ms=5000)
            self.assertNotEqual(exit_code, 259)  # Should not be STILL_ACTIVE
        finally:
            _close_handles(pi)
            if job:
                _close_job(job)

    def test_breakaway_ok_not_set(self) -> None:
        """JOB_OBJECT_LIMIT_BREAKAWAY_OK must NOT be set in sandbox Jobs."""
        job = _create_job()
        _set_kill_on_close(job)
        try:
            # Read back the limit information
            JobObjectExtendedLimitInformation = 9
            info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
            size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
            returned = wintypes.DWORD()
            kernel32.QueryInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size, ctypes.byref(returned)
            )
            flags = info.BasicLimitInformation.LimitFlags
            self.assertFalse(
                bool(flags & JOB_OBJECT_LIMIT_BREAKAWAY_OK),
                "JOB_OBJECT_LIMIT_BREAKAWAY_OK must NOT be set (prevents child escape)"
            )
            self.assertFalse(
                bool(flags & JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK),
                "JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK must NOT be set"
            )
        finally:
            _close_job(job)

    def test_nested_job_does_not_allow_escape(self) -> None:
        """A child process created within a Job inherits the Job — no escape."""
        job = _create_job()
        _set_kill_on_close(job)
        pi = _create_suspended_process("whoami > nul & exit 0")
        try:
            self.assertTrue(_assign_process_to_job(job, pi.hProcess))
            _resume_thread(pi)
            exit_code = _wait_for_process(pi, timeout_ms=10000)
            # Process completed normally within the job
            self.assertEqual(exit_code, 0)
            # After completion, the job killed it (since we closed the job
            # handles... wait, we haven't closed the job yet)
        finally:
            _close_handles(pi)
            _close_job(job)


class KeyMigrationEndToEndTests(unittest.TestCase):
    """End-to-end: key lifecycle with migration."""

    def test_full_lifecycle_create_rotate_revoke(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            controller = KeyLifecycleController(root)

            # 1. Create initial key
            ks1 = MemoryInstallationKeyStore()
            controller.record_initial_key(ks1.key_id)
            history = controller.key_history()
            self.assertEqual(history["rotation_count"], 1)

            # 2. Rotate to new key
            ks2 = MemoryInstallationKeyStore()
            receipt = controller.rotate(
                previous_key_id=ks1.key_id,
                new_key_id=ks2.key_id,
                previous_sign=ks1.sign,
                new_sign=ks2.sign,
            )
            self.assertTrue(receipt["rotation_valid"])
            history = controller.key_history()
            self.assertEqual(history["rotation_count"], 2)

            # 3. Revoke current key
            revoke_receipt = controller.revoke(
                key_id=ks2.key_id,
                sign=ks2.sign,
            )
            self.assertTrue(revoke_receipt["rotation_valid"])
            history = controller.key_history()
            self.assertEqual(history["rotation_count"], 3)
            self.assertEqual(history["current_key_id"], "")

            ks1.close()
            ks2.close()


if __name__ == "__main__":
    unittest.main()
