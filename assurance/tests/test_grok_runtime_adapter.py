from __future__ import annotations

import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from typing import Any
from unittest.mock import Mock, patch

from assurance.errors import AssuranceError
from assurance.grok_runtime_adapter import (
    GrokAcpSession,
    GrokRunRequest,
    GrokRuntimeConfig,
    run_grok_headless_once,
    validate_grok_retrieval_mode,
)


ROOT = Path(__file__).resolve().parents[2]


class _FakeProcess:
    def __init__(
        self,
        args: list[str],
        *,
        stdout,
        stderr,
        returncode: int | None = 0,
        version_output: str = "grok 0.2.112 (9bbd559437) [stable]\n",
    ) -> None:
        self.args = args
        self.pid = os.getpid()
        self.returncode = returncode
        self._returncode = returncode
        stdout.write(version_output.encode("utf-8"))
        stderr.write(b"")

    def wait(self, timeout: int | None = None) -> int:
        if self._returncode is None:
            raise subprocess.TimeoutExpired(self.args, timeout)
        self.returncode = self._returncode
        return self.returncode

    def poll(self) -> int | None:
        return self.returncode


class _FakeSupervisor:
    """A do-nothing supervisor that reports containment as active.

    Prevents the real JobObjectSupervisor from assigning the test
    process to a Kill-On-Close Job Object, which would kill the test
    suite when the supervisor is closed.
    """

    def __init__(self) -> None:
        self._assigned = False

    @property
    def is_active(self) -> bool:
        return True

    @property
    def is_assigned(self) -> bool:
        return self._assigned

    def assign_process(self, pid: int) -> None:
        self._assigned = True

    def close(self) -> None:
        pass

    def __enter__(self) -> "_FakeSupervisor":
        return self

    def __exit__(self, *args: object) -> None:  # type: ignore[override]
        pass


def _inspection(binary: Path) -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "valid": True,
        "binary_path": str(binary),
        "observed": {
            "sha256": "a" * 64,
            "version_output": "grok 0.2.112 (9bbd559437) [stable]",
        },
    }


def _trust() -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "valid": True,
        "trust_granted": True,
        "discovery": {"aggregate_sha256": "b" * 64},
    }


class GrokRuntimeAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        self.binary = self.root / "grok.exe"
        self.binary.write_bytes(b"MZ")
        self.config = GrokRuntimeConfig(repo_root=ROOT, timeout_seconds=5)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_version_smoke_writes_valid_receipt_and_events(self) -> None:
        run_root = self.root / "run"

        def popen_factory(args, **kwargs):
            return _FakeProcess(args, stdout=kwargs["stdout"], stderr=kwargs["stderr"])

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ):
            receipt = run_grok_headless_once(
                GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["retrieval"]["mode"], "off")
        self.assertFalse(receipt["retrieval"]["active_for_current_mode"])
        self.assertTrue(receipt["containment"]["no_residue_required"])
        self.assertTrue(receipt["containment"]["no_residue_observed"])
        self.assertTrue(receipt["containment"]["job_object_created"])
        self.assertTrue(receipt["containment"]["job_object_assigned"])
        self.assertEqual(
            receipt["containment"]["containment_provider"],
            "adapter_job_object",
        )
        self.assertTrue(receipt["containment"]["containment_available"])
        self.assertEqual(
            receipt["containment"]["residue_scan_scope"],
            "job_object_contained",
        )
        self.assertTrue(receipt["checks"]["adapter_containment_available"])
        self.assertTrue(receipt["checks"]["adapter_containment_provided"])
        self.assertTrue((run_root / "grok-runtime-receipt.json").is_file())
        self.assertTrue((run_root / "events.jsonl").is_file())
        self.assertIn(
            "run_finished",
            (run_root / "events.jsonl").read_text(encoding="utf-8"),
        )

    def test_process_timeout_does_not_write_valid_receipt(self) -> None:
        run_root = self.root / "run-timeout"

        def popen_factory(args, **kwargs):
            return _FakeProcess(
                args,
                stdout=kwargs["stdout"],
                stderr=kwargs["stderr"],
                returncode=None,
            )

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ):
            with self.assertRaises(AssuranceError) as raised:
                run_grok_headless_once(
                    GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                    config=self.config,
                    popen_factory=popen_factory,
                )

        self.assertIn("did not exit", str(raised.exception))
        self.assertFalse((run_root / "grok-runtime-receipt.json").exists())

    def test_version_smoke_can_create_isolated_workspace_under_run_root(self) -> None:
        run_root = self.root / "run-isolated"
        isolated_workspace = run_root / "workspace"

        def popen_factory(args, **kwargs):
            return _FakeProcess(args, stdout=kwargs["stdout"], stderr=kwargs["stderr"])

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ):
            receipt = run_grok_headless_once(
                GrokRunRequest(
                    run_root=run_root,
                    workspace_path=isolated_workspace,
                    retrieval_mode="local_browser",
                    retrieval_mode_explicit=True,
                ),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["request"]["retrieval_mode"], "local_browser")
        self.assertTrue(receipt["request"]["retrieval_mode_explicit"])
        self.assertTrue(receipt["retrieval"]["runtime_tool_retrieval_allowed"])
        self.assertTrue(receipt["retrieval"]["assurance_receipts_required"])
        self.assertFalse(receipt["retrieval"]["active_for_current_mode"])
        self.assertTrue(isolated_workspace.is_dir())
        self.assertEqual(
            receipt["request"]["workspace_path"],
            str(isolated_workspace.resolve()),
        )

    def test_refuses_nonempty_run_root(self) -> None:
        run_root = self.root / "occupied"
        run_root.mkdir()
        (run_root / "old.txt").write_text("old", encoding="utf-8")

        with self.assertRaises(AssuranceError):
            run_grok_headless_once(
                GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                config=self.config,
            )

    def test_retrieval_mode_allowlist_is_closed(self) -> None:
        self.assertEqual(validate_grok_retrieval_mode("framework_fallback"), "framework_fallback")
        with self.assertRaises(AssuranceError):
            validate_grok_retrieval_mode("implicit_runtime_search")


class GrokAcpNotificationMappingTests(unittest.TestCase):
    def _session(self) -> GrokAcpSession:
        session = GrokAcpSession.__new__(GrokAcpSession)
        session._last_streamed_text_len = 0
        session._last_streamed_text = ""
        session.turn_count = 0
        session.tool_call_count = 0
        return session

    def _collect(self, session: GrokAcpSession, update: dict[str, Any]) -> list[dict[str, Any]]:
        events: list[dict[str, Any]] = []
        msg = {
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": {"update": update},
        }
        GrokAcpSession._handle_acp_notification(
            session, msg, on_acp_event=lambda event: events.append(event) or None
        )
        return events

    def test_user_message_maps_to_user_message_not_text_delta(self) -> None:
        session = self._session()
        events = self._collect(
            session,
            {"sessionUpdate": "user_message", "text": "hello from user"},
        )
        self.assertEqual(len(events), 1)
        self.assertEqual(events[0]["event_type"], "user_message")
        self.assertEqual(events[0]["payload"]["text"], "hello from user")

    def test_assistant_message_cumulative_text_emits_only_new_suffix(self) -> None:
        session = self._session()
        first = self._collect(
            session, {"sessionUpdate": "assistant_message", "text": "Hel"}
        )
        second = self._collect(
            session, {"sessionUpdate": "assistant_message", "text": "Hello"}
        )
        self.assertEqual([event["payload"]["text"] for event in first + second], ["Hel", "lo"])

    def test_assistant_message_delta_chunks_are_not_dropped(self) -> None:
        session = self._session()
        first = self._collect(
            session, {"sessionUpdate": "assistant_message", "text": "Hel"}
        )
        second = self._collect(
            session, {"sessionUpdate": "assistant_message", "text": "lo"}
        )
        self.assertEqual([event["payload"]["text"] for event in first + second], ["Hel", "lo"])

    def test_warning_notification_preserves_severity(self) -> None:
        session = self._session()
        events: list[dict[str, Any]] = []
        msg = {
            "jsonrpc": "2.0",
            "method": "notification",
            "params": {"type": "warning", "message": "heads up"},
        }
        GrokAcpSession._handle_acp_notification(
            session, msg, on_acp_event=lambda event: events.append(event) or None
        )
        self.assertEqual(events[0]["event_type"], "error_event")
        self.assertEqual(events[0]["payload"]["source"], "acp_warning")
        self.assertEqual(events[0]["payload"]["severity"], "warning")


if __name__ == "__main__":
    unittest.main()
