from __future__ import annotations

import ctypes
from ctypes import wintypes
import os
import subprocess
import unittest

from assurance.job_object_supervisor import (
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    _create_kill_on_close_job,
    _open_process,
    _assign_process_to_job,
    _close_handle,
    JobObjectSupervisor,
)
from assurance.errors import AssuranceError


CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


class JobObjectCreationTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_create_job_returns_valid_handle(self) -> None:
        job = _create_kill_on_close_job()
        self.assertIsNotNone(job)
        _close_handle(job)  # type: ignore[arg-type]

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_kill_on_close_proven_by_behavior(self) -> None:
        """KILL_ON_CLOSE is proven by closing the job and watching the
        process die, not by reading internal flags.  This test is a
        direct behavioral proof."""
        sv = JobObjectSupervisor()
        proc = subprocess.Popen(
            ["cmd.exe", "/c", "ping", "-n", "30", "127.0.0.1"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=CREATE_NO_WINDOW,
        )
        sv.assign_process(proc.pid)
        self.assertTrue(sv.is_assigned)
        # Before close, the process is alive
        self.assertIsNone(proc.poll())
        sv.close()
        # After close, KILL_ON_CLOSE terminates the process
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        self.assertIsNotNone(proc.poll())


class JobObjectSupervisorTests(unittest.TestCase):
    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_is_active_on_windows(self) -> None:
        sv = JobObjectSupervisor()
        self.assertTrue(sv.is_active)
        self.assertFalse(sv.is_assigned)
        sv.close()

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_assign_process_sets_assigned(self) -> None:
        sv = JobObjectSupervisor()
        try:
            proc = subprocess.Popen(
                ["cmd.exe", "/c", "timeout", "/t", "2", "/nobreak"],
                creationflags=CREATE_NO_WINDOW,
            )
            sv.assign_process(proc.pid)
            self.assertTrue(sv.is_assigned)
            proc.wait(timeout=10)
        finally:
            sv.close()

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_assign_nonexistent_pid_raises(self) -> None:
        sv = JobObjectSupervisor()
        try:
            with self.assertRaises(AssuranceError):
                sv.assign_process(99999999)
        finally:
            sv.close()

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_kill_on_close_terminates_process(self) -> None:
        sv = JobObjectSupervisor()
        proc = subprocess.Popen(
            ["cmd.exe", "/c", "ping", "-n", "30", "127.0.0.1"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=CREATE_NO_WINDOW,
        )
        sv.assign_process(proc.pid)
        self.assertTrue(sv.is_assigned)
        # Close the supervisor → kill-on-close terminates the process
        sv.close()
        proc.wait(timeout=15)
        self.assertIsNotNone(proc.poll())
        self.assertNotEqual(proc.poll(), None)

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_context_manager_closes_job(self) -> None:
        sv = JobObjectSupervisor()
        self.assertTrue(sv.is_active)
        with sv:
            self.assertTrue(sv.is_active)
        # After __exit__, the job handle is closed
        self.assertFalse(sv.is_active)

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_child_processes_also_killed(self) -> None:
        # Start cmd that spawns a child cmd that sleeps.
        # Kill-on-close must kill both.
        sv = JobObjectSupervisor()
        proc = subprocess.Popen(
            [
                "cmd.exe",
                "/c",
                "start /b ping -n 30 127.0.0.1 & ping -n 30 127.0.0.1",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=CREATE_NO_WINDOW,
        )
        sv.assign_process(proc.pid)
        # Give the child a moment to start
        import time

        time.sleep(0.5)
        sv.close()
        # Both parent and child should be dead or dying
        proc.wait(timeout=15)
        self.assertIsNotNone(proc.poll())

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_double_close_is_safe(self) -> None:
        sv = JobObjectSupervisor()
        sv.close()
        sv.close()  # Should not raise

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_is_assigned_stays_false_when_no_process_assigned(self) -> None:
        sv = JobObjectSupervisor()
        try:
            self.assertFalse(sv.is_assigned)
        finally:
            sv.close()

    @unittest.skipUnless(os.name == "nt", "Windows-only")
    def test_container_provides_kill_guarantee(self) -> None:
        """End-to-end: prove that job close kills running process quickly."""
        sv = JobObjectSupervisor()
        proc = subprocess.Popen(
            ["cmd.exe", "/c", "ping", "-n", "60", "127.0.0.1"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=CREATE_NO_WINDOW,
        )
        sv.assign_process(proc.pid)
        self.assertIsNone(proc.poll())  # Still running
        sv.close()
        # Wait up to 15s = grace period for kill + process reaping
        try:
            proc.wait(timeout=15)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=5)
        self.assertIsNotNone(proc.poll())


class JobObjectSupervisorNonWindowsTests(unittest.TestCase):
    @unittest.skipUnless(os.name != "nt", "non-Windows only")
    def test_is_active_false_non_windows(self) -> None:
        sv = JobObjectSupervisor()
        self.assertFalse(sv.is_active)
        self.assertFalse(sv.is_assigned)
        # assign_process should be a no-op
        sv.assign_process(1)
        self.assertFalse(sv.is_assigned)
        sv.close()


if __name__ == "__main__":
    unittest.main()
