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
    NO_TOOL_DISALLOWED_TOOLS,
    GrokRunRequest,
    GrokRuntimeConfig,
    run_grok_acp_once,
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


class _FakeAcpStdin:
    def __init__(self) -> None:
        self.closed = False
        self.writes: list[str] = []

    def write(self, text: str) -> int:
        self.writes.append(text)
        return len(text)

    def flush(self) -> None:
        pass

    def close(self) -> None:
        self.closed = True


class _FakeAcpStdout:
    def __init__(self) -> None:
        self._lines = [
            json_line({
                "jsonrpc": "2.0",
                "id": "acp-init-1",
                "result": {"protocolVersion": 1},
            }),
            json_line({
                "jsonrpc": "2.0",
                "id": "acp-session-1",
                "result": {"sessionId": "S-FAKE-ACP-001"},
            }),
            json_line({
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "update": {
                        "sessionUpdate": "agent_message_chunk",
                        "content": {"type": "text", "text": "Hello from ACP."},
                    }
                },
            }),
            json_line({
                "jsonrpc": "2.0",
                "method": "_x.ai/session/update",
                "params": {
                    "update": {
                        "sessionUpdate": "turn_completed",
                        "stop_reason": "end_turn",
                    }
                },
            }),
            json_line({
                "jsonrpc": "2.0",
                "id": "acp-prompt-1",
                "result": {
                    "stopReason": "end_turn",
                },
            }),
        ]

    def readline(self) -> str:
        if self._lines:
            return self._lines.pop(0)
        return ""


class _FakeAcpProcess:
    def __init__(self, args: list[str], **kwargs: Any) -> None:
        self.args = args
        self.pid = os.getpid()
        self.returncode: int | None = None
        self.stdin = _FakeAcpStdin()
        self.stdout = _FakeAcpStdout()

    def poll(self) -> int | None:
        return self.returncode

    def wait(self, timeout: int | None = None) -> int:
        self.returncode = 0
        return 0

    def kill(self) -> None:
        self.returncode = -9


def json_line(value: dict[str, Any]) -> str:
    import json
    return json.dumps(value, separators=(",", ":")) + "\n"


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

    def test_acp_smoke_writes_schema_valid_receipt_and_events(self) -> None:
        run_root = self.root / "run-acp"

        fake_provider_root = run_root / "fake-provider"
        fake_provider_ready_path = fake_provider_root / "ready.json"
        fake_provider_ready = {
            "schema_version": "0.1.0",
            "ready": True,
            "host": "127.0.0.1",
            "port": 45678,
            "external_bind": False,
            "authorization_value_recorded": False,
        }

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ), patch("assurance.grok_runtime_adapter._resume_main_thread"), patch(
            "assurance.grok_runtime_adapter._start_loopback_fake_provider",
            return_value=(
                _FakeAcpProcess(["fake-provider"]),
                fake_provider_ready,
                fake_provider_root,
                fake_provider_ready_path,
            ),
        ):
            receipt = run_grok_acp_once(
                GrokRunRequest(
                    run_root=run_root,
                    workspace_path=self.workspace,
                    run_id="RUN-GROK-ACP-TEST-001",
                    mode="acp_smoke",
                    prompt_text="Say hello.",
                    fake_provider=True,
                    acp_permission_mode="auto_allow_once",
                ),
                config=self.config,
                popen_factory=lambda args, **kwargs: _FakeAcpProcess(args, **kwargs),
            )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["request"]["mode"], "acp_smoke")
        self.assertEqual(receipt["execution"]["command_kind"], "grok_acp")
        self.assertEqual(receipt["prompt"]["output_format"], "acp_json_rpc")
        self.assertEqual(receipt["prompt"]["response_summary"], "Hello from ACP.")
        self.assertTrue(receipt["containment"]["job_object_assigned"])
        self.assertTrue(receipt["checks"]["acp_response_received"])
        self.assertEqual(receipt["acp"]["prompt_count"], 1)
        self.assertTrue(receipt["request"]["fake_provider"])
        self.assertEqual(receipt["fake_provider"]["host"], "127.0.0.1")
        self.assertEqual(receipt["fake_provider"]["port"], 45678)
        self.assertFalse(receipt["fake_provider"]["real_model_invoked"])
        self.assertTrue(
            (run_root / "profile" / ".grok" / "config.toml").read_text(
                encoding="utf-8"
            ).find("http://127.0.0.1:45678") >= 0
        )
        self.assertTrue((run_root / "grok-runtime-receipt.json").is_file())
        self.assertTrue((run_root / "events.jsonl").is_file())
        self.assertTrue((run_root / "acp_transcript.jsonl").is_file())

    def test_acp_smoke_disable_builtin_tools_passes_grok_tool_filter_flags(self) -> None:
        run_root = self.root / "run-acp-no-tools"
        launched_args: list[str] = []

        def popen_factory(args, **kwargs):
            launched_args.extend(args)
            return _FakeAcpProcess(args, **kwargs)

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ), patch("assurance.grok_runtime_adapter._resume_main_thread"):
            receipt = run_grok_acp_once(
                GrokRunRequest(
                    run_root=run_root,
                    workspace_path=self.workspace,
                    run_id="RUN-GROK-ACP-NO-TOOLS-TEST-001",
                    mode="acp_smoke",
                    prompt_text="Say hello.",
                    model_config_toml="[models]\ndefault = \"custom\"\n",
                    disable_builtin_tools=True,
                    acp_permission_mode="auto_allow_once",
                ),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["request"]["disable_builtin_tools"])
        self.assertIn("--disable-web-search", launched_args)
        self.assertIn("--no-subagents", launched_args)
        self.assertIn("--tools", launched_args)
        self.assertIn("", launched_args)
        self.assertIn("--disallowed-tools", launched_args)
        self.assertIn(NO_TOOL_DISALLOWED_TOOLS, launched_args)
        self.assertIn("--reasoning-effort", launched_args)
        self.assertIn("none", launched_args)
        self.assertLess(launched_args.index("--tools"), launched_args.index("agent"))
        self.assertLess(launched_args.index("--disallowed-tools"), launched_args.index("agent"))

    def test_real_provider_acp_smoke_marks_receipt_invalid_on_provider_api_error_stderr(self) -> None:
        run_root = self.root / "run-acp-provider-error"

        def popen_factory(args, **kwargs):
            kwargs["stderr"].write(b"ERROR chat/completions API error 400 Bad Request")
            kwargs["stderr"].flush()
            return _FakeAcpProcess(args, **kwargs)

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()), patch(
            "assurance.grok_runtime_adapter.JobObjectSupervisor",
            return_value=_FakeSupervisor(),
        ), patch("assurance.grok_runtime_adapter._resume_main_thread"):
            receipt = run_grok_acp_once(
                GrokRunRequest(
                    run_root=run_root,
                    workspace_path=self.workspace,
                    run_id="RUN-GROK-ACP-PROVIDER-ERROR-TEST-001",
                    mode="acp_smoke",
                    prompt_text="Say hello.",
                    model_config_toml="[models]\ndefault = \"custom\"\n",
                    disable_builtin_tools=True,
                    acp_permission_mode="auto_allow_once",
                ),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertFalse(receipt["valid"])
        self.assertFalse(receipt["checks"]["provider_api_errors_absent"])


class GrokAcpNotificationMappingTests(unittest.TestCase):
    def _session(self) -> GrokAcpSession:
        session = GrokAcpSession.__new__(GrokAcpSession)
        session._last_streamed_text_len = 0
        session._last_streamed_text = ""
        session._last_response_text = ""
        session.turn_count = 0
        session.stop_reason = ""
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

    def test_agent_message_chunk_content_dict_sets_response_text(self) -> None:
        session = self._session()
        events = self._collect(
            session,
            {
                "sessionUpdate": "agent_message_chunk",
                "content": {"type": "text", "text": "LIF_FAKE_PROVIDER_OK"},
            },
        )
        self.assertEqual(events[0]["event_type"], "text_delta")
        self.assertEqual(events[0]["payload"]["text"], "LIF_FAKE_PROVIDER_OK")
        self.assertEqual(session._last_response_text, "LIF_FAKE_PROVIDER_OK")

    def test_xai_session_update_turn_completed_sets_stop_reason(self) -> None:
        session = self._session()
        events: list[dict[str, Any]] = []
        msg = {
            "jsonrpc": "2.0",
            "method": "_x.ai/session/update",
            "params": {
                "update": {
                    "sessionUpdate": "turn_completed",
                    "stop_reason": "end_turn",
                }
            },
        }
        GrokAcpSession._handle_acp_notification(
            session, msg, on_acp_event=lambda event: events.append(event) or None
        )
        self.assertEqual(events, [])
        self.assertEqual(session.turn_count, 1)
        self.assertEqual(session.stop_reason, "end_turn")

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
