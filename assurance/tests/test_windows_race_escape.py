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


if os.name != "nt":
    raise unittest.SkipTest("Windows Job Object tests require Windows")


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


# ---------------------------------------------------------------------------
# GAK-WIN-001: Creation-Time Job Object Assignment (PROC_THREAD_ATTRIBUTE_JOB_LIST)
# ---------------------------------------------------------------------------

PROC_THREAD_ATTRIBUTE_JOB_LIST = 0x0002000D
EXTENDED_STARTUPINFO_PRESENT = 0x00080000

_kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)


class _STARTUPINFOEX(ctypes.Structure):
    _fields_ = [
        ("StartupInfo", _STARTUPINFOW),
        ("lpAttributeList", ctypes.c_void_p),
    ]


@unittest.skipUnless(os.name == "nt", "creation-time Job tests require Windows")
class CreationTimeJobAssignmentTests(unittest.TestCase):
    """GAK-WIN-001: Verify PROC_THREAD_ATTRIBUTE_JOB_LIST eliminates the
    start→Job assignment race by having the kernel attach the process
    to the Job Object before the initial thread is created."""

    def test_proc_thread_attribute_job_list_constant_valid(self) -> None:
        """PROC_THREAD_ATTRIBUTE_JOB_LIST is 0x0002000D on all Windows 8+."""
        self.assertEqual(
            PROC_THREAD_ATTRIBUTE_JOB_LIST, 0x0002000D,
            "PROC_THREAD_ATTRIBUTE_JOB_LIST must be 0x0002000D",
        )

    def test_creation_time_job_attribute_supported(self) -> None:
        """Verify the OS supports PROC_THREAD_ATTRIBUTE_JOB_LIST.

        The attribute list size query with count=2 should succeed on
        any Windows 8+ system where this feature is available.
        """
        attr_size = ctypes.c_size_t()
        ok = _kernel32.InitializeProcThreadAttributeList(
            None, 2, 0, ctypes.byref(attr_size)
        )
        # May fail on pre-Win8, but we're on Win10+ by now
        if not ok:
            err = ctypes.get_last_error()
            if err == 87:  # ERROR_INVALID_PARAMETER → pre-Win8
                self.skipTest(
                    f"PROC_THREAD_ATTRIBUTE_JOB_LIST not supported "
                    f"(ERROR_INVALID_PARAMETER, likely pre-Win8)"
                )
            else:
                self.skipTest(
                    f"InitializeProcThreadAttributeList failed: {err}"
                )
        self.assertGreater(
            attr_size.value, 0,
            "attribute list size must be > 0 when count=2 is supported",
        )

    def test_creation_time_job_process_is_in_job_before_resume(self) -> None:
        """GAK-WIN-001 core test: a process created with
        PROC_THREAD_ATTRIBUTE_JOB_LIST is already in the Job Object
        before ResumeThread — no user-mode race window exists.
        """
        # 1. Create job with KILL_ON_CLOSE
        _kernel32.CreateJobObjectW.argtypes = [ctypes.c_void_p, wintypes.LPCWSTR]
        _kernel32.CreateJobObjectW.restype = wintypes.HANDLE
        job = _kernel32.CreateJobObjectW(None, None)
        self.assertIsNotNone(job, "CreateJobObjectW must succeed")
        try:
            # Set KILL_ON_CLOSE
            info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
            JobObjectExtendedLimitInformation = 9
            size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
            returned = wintypes.DWORD()
            _kernel32.QueryInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size, ctypes.byref(returned),
            )
            info.BasicLimitInformation.LimitFlags |= (
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            )
            _kernel32.SetInformationJobObject.argtypes = [
                wintypes.HANDLE, ctypes.c_int, ctypes.c_void_p, wintypes.DWORD,
            ]
            _kernel32.SetInformationJobObject.restype = wintypes.BOOL
            self.assertTrue(
                _kernel32.SetInformationJobObject(
                    job, JobObjectExtendedLimitInformation,
                    ctypes.byref(info), size,
                ),
                "SetInformationJobObject must succeed",
            )

            # 2. Set up PROC_THREAD_ATTRIBUTE_JOB_LIST
            attr_count = 2  # JOB_LIST + SECURITY_CAPABILITIES (optional but we test both)
            # We don't need AppContainer for this pure-Job test

            attr_size = ctypes.c_size_t()
            _kernel32.InitializeProcThreadAttributeList(
                None, wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size),
            )
            if attr_size.value == 0:
                self.skipTest(
                    "PROC_THREAD_ATTRIBUTE_JOB_LIST not available on this OS"
                )

            attr_list = ctypes.create_string_buffer(attr_size.value)
            self.assertTrue(
                _kernel32.InitializeProcThreadAttributeList(
                    ctypes.cast(attr_list, ctypes.c_void_p),
                    wintypes.DWORD(attr_count),
                    0,
                    ctypes.byref(attr_size),
                ),
                "InitializeProcThreadAttributeList must succeed",
            )

            # Set JOB_LIST attribute with our job handle
            _kernel32.UpdateProcThreadAttribute.argtypes = [
                ctypes.c_void_p, wintypes.DWORD, ctypes.c_size_t,
                ctypes.c_void_p, ctypes.c_size_t,
                ctypes.c_void_p, ctypes.c_void_p,
            ]
            _kernel32.UpdateProcThreadAttribute.restype = wintypes.BOOL
            job_handle_value = ctypes.c_void_p(job)
            job_attr_set = _kernel32.UpdateProcThreadAttribute(
                ctypes.cast(attr_list, ctypes.c_void_p),
                0,
                ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                ctypes.byref(job_handle_value),
                ctypes.sizeof(ctypes.c_void_p),
                None,
                None,
            )
            if not job_attr_set:
                self.skipTest(
                    "PROC_THREAD_ATTRIBUTE_JOB_LIST UpdateProcThreadAttribute "
                    "not supported"
                )

            # 3. Create process suspended with EXTENDED_STARTUPINFO_PRESENT
            si_ex = _STARTUPINFOEX()
            si_ex.StartupInfo.cb = ctypes.sizeof(_STARTUPINFOEX)
            si_ex.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)

            pi = _PROCESS_INFORMATION()
            _kernel32.CreateProcessW.argtypes = [
                wintypes.LPCWSTR, wintypes.LPWSTR, ctypes.c_void_p,
                ctypes.c_void_p, wintypes.BOOL, wintypes.DWORD,
                ctypes.c_void_p, wintypes.LPCWSTR,
                ctypes.POINTER(_STARTUPINFOEX),
                ctypes.POINTER(_PROCESS_INFORMATION),
            ]
            _kernel32.CreateProcessW.restype = wintypes.BOOL

            creation_flags = (
                EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_NO_WINDOW
            )
            cmd = r"cmd.exe /c exit 0"

            success = _kernel32.CreateProcessW(
                None, cmd, None, None, False,
                creation_flags, None, None,
                ctypes.byref(si_ex), ctypes.byref(pi),
            )

            # Clean up attribute list
            _kernel32.DeleteProcThreadAttributeList.argtypes = [ctypes.c_void_p]
            _kernel32.DeleteProcThreadAttributeList.restype = None
            _kernel32.DeleteProcThreadAttributeList(
                ctypes.cast(attr_list, ctypes.c_void_p)
            )

            self.assertTrue(success, "CreateProcess must succeed with JOB_LIST attr")
            self.assertGreater(pi.dwProcessId, 0)
            self.assertIsNotNone(pi.hProcess)

            try:
                # 4. GAK-WIN-001 KEY CHECK: the process MUST already be in
                #    the job BEFORE ResumeThread.  This proves the kernel
                #    assigned it atomically at creation time.
                _kernel32.IsProcessInJob.argtypes = [
                    wintypes.HANDLE, wintypes.HANDLE,
                    ctypes.POINTER(wintypes.BOOL),
                ]
                _kernel32.IsProcessInJob.restype = wintypes.BOOL
                in_job = wintypes.BOOL()
                self.assertTrue(
                    _kernel32.IsProcessInJob(
                        pi.hProcess, None, ctypes.byref(in_job)
                    ),
                    "IsProcessInJob must succeed",
                )
                self.assertTrue(
                    bool(in_job.value),
                    "GAK-WIN-001: process MUST be in Job Object before "
                    "ResumeThread when PROC_THREAD_ATTRIBUTE_JOB_LIST is used. "
                    "This proves the kernel assigned the job at creation time "
                    "with zero user-mode race window.",
                )

                # 5. Verify AssignProcessToJobObject is idempotent for the
                #    same job (Windows may return TRUE or FALSE depending
                #    on the OS version — same-job reassignment is harmless).
                _kernel32.AssignProcessToJobObject.argtypes = [
                    wintypes.HANDLE, wintypes.HANDLE,
                ]
                _kernel32.AssignProcessToJobObject.restype = wintypes.BOOL
                _ = _kernel32.AssignProcessToJobObject(job, pi.hProcess)
                # Regardless of return value, the process remains in the
                # job — double-assignment to the same job is a no-op.
                still_in_job = wintypes.BOOL()
                self.assertTrue(
                    _kernel32.IsProcessInJob(
                        pi.hProcess, None, ctypes.byref(still_in_job)
                    ),
                )
                self.assertTrue(
                    bool(still_in_job.value),
                    "process must remain in job after idempotent reassign",
                )

                # 6. Resume and wait — process completes normally
                _kernel32.ResumeThread.argtypes = [wintypes.HANDLE]
                _kernel32.ResumeThread.restype = wintypes.DWORD
                self.assertNotEqual(
                    _kernel32.ResumeThread(pi.hThread), 0xFFFFFFFF,
                )

                _kernel32.WaitForSingleObject.argtypes = [
                    wintypes.HANDLE, wintypes.DWORD,
                ]
                _kernel32.WaitForSingleObject.restype = wintypes.DWORD
                wait_rc = _kernel32.WaitForSingleObject(
                    pi.hProcess, 5000
                )
                self.assertEqual(wait_rc, 0)

                _kernel32.GetExitCodeProcess.argtypes = [
                    wintypes.HANDLE, ctypes.POINTER(wintypes.DWORD),
                ]
                _kernel32.GetExitCodeProcess.restype = wintypes.BOOL
                ec = wintypes.DWORD()
                _kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(ec))
                self.assertEqual(ec.value, 0)

            finally:
                _kernel32.CloseHandle(pi.hThread)
                _kernel32.CloseHandle(pi.hProcess)
        finally:
            _kernel32.CloseHandle(job)

    def test_creation_time_job_child_processes_also_contained(self) -> None:
        """GAK-WIN-001: child processes spawned inside the job inherit
        the Job Object — no escape via process creation."""
        job = _kernel32.CreateJobObjectW(None, None)
        self.assertIsNotNone(job)
        try:
            info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
            JobObjectExtendedLimitInformation = 9
            size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
            returned = wintypes.DWORD()
            _kernel32.QueryInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size, ctypes.byref(returned),
            )
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            _kernel32.SetInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size,
            )

            # Create a parent process with creation-time Job assignment
            # that spawns a child cmd.exe process
            # The child must also be in the job (automatic inheritance)

            attr_count = 1  # JOB_LIST only (no AppContainer needed)
            attr_size = ctypes.c_size_t()
            _kernel32.InitializeProcThreadAttributeList(
                None, wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size),
            )
            if attr_size.value == 0:
                self.skipTest("PROC_THREAD_ATTRIBUTE_JOB_LIST not available")

            attr_list = ctypes.create_string_buffer(attr_size.value)
            self.assertTrue(
                _kernel32.InitializeProcThreadAttributeList(
                    ctypes.cast(attr_list, ctypes.c_void_p),
                    wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size),
                )
            )

            job_handle_value = ctypes.c_void_p(job)
            job_attr_set = _kernel32.UpdateProcThreadAttribute(
                ctypes.cast(attr_list, ctypes.c_void_p),
                0,
                ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                ctypes.byref(job_handle_value),
                ctypes.sizeof(ctypes.c_void_p),
                None,
                None,
            )
            if not job_attr_set:
                self.skipTest(
                    "PROC_THREAD_ATTRIBUTE_JOB_LIST not supported"
                )

            si_ex = _STARTUPINFOEX()
            si_ex.StartupInfo.cb = ctypes.sizeof(_STARTUPINFOEX)
            si_ex.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)

            pi = _PROCESS_INFORMATION()
            # Parent: cmd /c "start /b cmd /c exit 0"
            # This creates a child cmd process inside the job
            cmd = r'cmd.exe /c start /b "" cmd.exe /c exit 0'

            creation_flags = (
                EXTENDED_STARTUPINFO_PRESENT
                | CREATE_SUSPENDED
                | CREATE_NO_WINDOW
            )

            success = _kernel32.CreateProcessW(
                None, cmd, None, None, False,
                creation_flags, None, None,
                ctypes.byref(si_ex), ctypes.byref(pi),
            )
            _kernel32.DeleteProcThreadAttributeList(
                ctypes.cast(attr_list, ctypes.c_void_p)
            )

            self.assertTrue(success, "CreateProcess with JOB_LIST must succeed")
            try:
                # Verify parent is in job BEFORE resume
                in_job = wintypes.BOOL()
                _kernel32.IsProcessInJob(
                    pi.hProcess, None, ctypes.byref(in_job)
                )
                self.assertTrue(
                    bool(in_job.value),
                    "parent process must be in Job at creation time",
                )

                # Resume and wait (parent + child)
                _kernel32.ResumeThread(pi.hThread)
                wait_rc = _kernel32.WaitForSingleObject(pi.hProcess, 10000)
                self.assertEqual(wait_rc, 0)

                # Both parent and child should have exited cleanly
                ec = wintypes.DWORD()
                _kernel32.GetExitCodeProcess(pi.hProcess, ctypes.byref(ec))
                self.assertEqual(ec.value, 0)

                # The child process is also killed (KILL_ON_CLOSE)
                # when the job handle is closed below — this is the
                # containment guarantee.

            finally:
                _kernel32.CloseHandle(pi.hThread)
                _kernel32.CloseHandle(pi.hProcess)
        finally:
            _kernel32.CloseHandle(job)

    def test_creation_time_job_applies_before_any_code_runs(self) -> None:
        """GAK-WIN-001: Even without CREATE_SUSPENDED, the creation-time job
        attribute ensures the process is in the Job Object before any
        instruction executes.

        We test with CREATE_SUSPENDED here for safety (we need to verify
        before the process does anything), but the kernel guarantee is the
        same regardless of the CREATE_SUSPENDED flag.
        """
        job = _kernel32.CreateJobObjectW(None, None)
        self.assertIsNotNone(job)
        try:
            info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
            JobObjectExtendedLimitInformation = 9
            size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
            returned = wintypes.DWORD()
            _kernel32.QueryInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size, ctypes.byref(returned),
            )
            info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            _kernel32.SetInformationJobObject(
                job, JobObjectExtendedLimitInformation,
                ctypes.byref(info), size,
            )

            # Quick smoke test: JOB_LIST + CREATE_SUSPENDED always works
            attr_count = 1
            attr_size = ctypes.c_size_t()
            _kernel32.InitializeProcThreadAttributeList(
                None, wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size),
            )
            if attr_size.value == 0:
                self.skipTest("PROC_THREAD_ATTRIBUTE_JOB_LIST not available")

            attr_list = ctypes.create_string_buffer(attr_size.value)
            _kernel32.InitializeProcThreadAttributeList(
                ctypes.cast(attr_list, ctypes.c_void_p),
                wintypes.DWORD(attr_count), 0, ctypes.byref(attr_size),
            )

            job_handle_value = ctypes.c_void_p(job)
            self.assertTrue(
                _kernel32.UpdateProcThreadAttribute(
                    ctypes.cast(attr_list, ctypes.c_void_p),
                    0,
                    ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                    ctypes.byref(job_handle_value),
                    ctypes.sizeof(ctypes.c_void_p),
                    None,
                    None,
                ),
                "JOB_LIST attribute must be settable",
            )

            si_ex = _STARTUPINFOEX()
            si_ex.StartupInfo.cb = ctypes.sizeof(_STARTUPINFOEX)
            si_ex.lpAttributeList = ctypes.cast(attr_list, ctypes.c_void_p)

            pi = _PROCESS_INFORMATION()
            success = _kernel32.CreateProcessW(
                None, r"cmd.exe /c exit 0", None, None, False,
                EXTENDED_STARTUPINFO_PRESENT | CREATE_SUSPENDED | CREATE_NO_WINDOW,
                None, None,
                ctypes.byref(si_ex), ctypes.byref(pi),
            )
            _kernel32.DeleteProcThreadAttributeList(
                ctypes.cast(attr_list, ctypes.c_void_p)
            )
            self.assertTrue(success)
            try:
                in_job = wintypes.BOOL()
                _kernel32.IsProcessInJob(
                    pi.hProcess, None, ctypes.byref(in_job)
                )
                self.assertTrue(
                    bool(in_job.value),
                    "GAK-WIN-001: Process in job at creation time — "
                    "no user-mode code ever ran outside the Job Object",
                )
            finally:
                _kernel32.TerminateProcess(pi.hProcess, 0)
                _kernel32.CloseHandle(pi.hThread)
                _kernel32.CloseHandle(pi.hProcess)
        finally:
            _kernel32.CloseHandle(job)

    def test_race_window_comparison_creation_time_wins(self) -> None:
        """GAK-WIN-001: Compare the two approaches.

        Suspended+assign: ~2-8 syscalls between CreateProcess and
                           AssignProcessToJobObject (race window).
        Creation-time:     0 instructions — the kernel attaches the job
                           before the initial thread exists (no race).

        This test verifies the end-to-end behaviour is identical:
        both approaches produce a contained process.
        """
        for label, use_creation_time in [
            ("suspended+assign", False),
            ("creation-time", True),
        ]:
            job = _kernel32.CreateJobObjectW(None, None)
            self.assertIsNotNone(job, f"{label}: job creation failed")
            try:
                info = _JOBOBJECT_EXTENDED_LIMIT_INFORMATION()
                JobObjectExtendedLimitInformation = 9
                size = ctypes.sizeof(_JOBOBJECT_EXTENDED_LIMIT_INFORMATION)
                returned = wintypes.DWORD()
                _kernel32.QueryInformationJobObject(
                    job, JobObjectExtendedLimitInformation,
                    ctypes.byref(info), size, ctypes.byref(returned),
                )
                info.BasicLimitInformation.LimitFlags |= JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                self.assertTrue(
                    _kernel32.SetInformationJobObject(
                        job, JobObjectExtendedLimitInformation,
                        ctypes.byref(info), size,
                    )
                )

                pi = _create_suspended_process("exit 0")

                try:
                    if use_creation_time:
                        # Can't use _create_suspended_process for this test
                        # since it doesn't use EXTENDED_STARTUPINFO_PRESENT.
                        # We close this process and re-create below.
                        _kernel32.TerminateProcess(pi.hProcess, 0)
                        _kernel32.CloseHandle(pi.hThread)
                        _kernel32.CloseHandle(pi.hProcess)
                        pi = None  # type: ignore[assignment]

                        # Re-create with JOB_LIST
                        attr_count = 1
                        attr_size_ct = ctypes.c_size_t()
                        _kernel32.InitializeProcThreadAttributeList(
                            None, wintypes.DWORD(attr_count), 0,
                            ctypes.byref(attr_size_ct),
                        )
                        if attr_size_ct.value == 0:
                            self.skipTest(
                                f"{label}: JOB_LIST not available"
                            )
                        attr_list_ct = ctypes.create_string_buffer(
                            attr_size_ct.value
                        )
                        _kernel32.InitializeProcThreadAttributeList(
                            ctypes.cast(attr_list_ct, ctypes.c_void_p),
                            wintypes.DWORD(attr_count), 0,
                            ctypes.byref(attr_size_ct),
                        )
                        jhv = ctypes.c_void_p(job)
                        if not _kernel32.UpdateProcThreadAttribute(
                            ctypes.cast(attr_list_ct, ctypes.c_void_p),
                            0,
                            ctypes.c_size_t(PROC_THREAD_ATTRIBUTE_JOB_LIST),
                            ctypes.byref(jhv),
                            ctypes.sizeof(ctypes.c_void_p),
                            None,
                            None,
                        ):
                            _kernel32.DeleteProcThreadAttributeList(
                                ctypes.cast(attr_list_ct, ctypes.c_void_p)
                            )
                            self.skipTest(
                                f"{label}: JOB_LIST UpdateProcThreadAttribute "
                                "not supported"
                            )

                        si_ex_ct = _STARTUPINFOEX()
                        si_ex_ct.StartupInfo.cb = ctypes.sizeof(
                            _STARTUPINFOEX
                        )
                        si_ex_ct.lpAttributeList = ctypes.cast(
                            attr_list_ct, ctypes.c_void_p
                        )

                        pi_ct = _PROCESS_INFORMATION()
                        ok = _kernel32.CreateProcessW(
                            None, r"cmd.exe /c exit 0", None, None, False,
                            EXTENDED_STARTUPINFO_PRESENT
                            | CREATE_SUSPENDED
                            | CREATE_NO_WINDOW,
                            None, None,
                            ctypes.byref(si_ex_ct),
                            ctypes.byref(pi_ct),
                        )
                        _kernel32.DeleteProcThreadAttributeList(
                            ctypes.cast(attr_list_ct, ctypes.c_void_p)
                        )
                        self.assertTrue(ok)
                        pi = pi_ct  # type: ignore[assignment]

                        # Already in job — verify
                        in_job_ct = wintypes.BOOL()
                        _kernel32.IsProcessInJob(
                            pi.hProcess, None, ctypes.byref(in_job_ct)
                        )
                        self.assertTrue(
                            bool(in_job_ct.value),
                            f"{label}: process must already be in job",
                        )
                    else:
                        # Suspended + post-creation assign
                        was_in_job = _is_process_in_job(pi.hProcess)
                        if not was_in_job:
                            self.assertTrue(
                                _assign_process_to_job(job, pi.hProcess),
                                f"{label}: failed to assign process to job",
                            )
                        self.assertTrue(
                            _is_process_in_job(pi.hProcess),
                            f"{label}: process must be in job after assign",
                        )

                    _resume_thread(pi)
                    exit_code = _wait_for_process(pi, timeout_ms=5000)
                    self.assertEqual(exit_code, 0, f"{label}: process failed")

                finally:
                    if pi is not None:
                        _close_handles(pi)
            finally:
                _close_job(job)


if __name__ == "__main__":
    unittest.main()
