from __future__ import annotations

from io import StringIO
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from assurance.tui.bridge import (
    build_deepseek_live_run_fn,
    build_grok_acp_live_run_fn,
    build_grok_live_run_fn,
)


ROOT = Path(__file__).resolve().parents[2]


class GrokTuiWiringTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "run"
        self.run_root.mkdir()
        self.events_path = self.run_root / "events.jsonl"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    @patch("assurance.grok_runtime_adapter.run_grok_version_smoke")
    def test_grok_live_run_fn_streams_normalized_events(self, mock_run) -> None:
        events = [
            {
                "event_type": "run_preflight",
                "payload": {
                    "adapter_id": "grok-runtime-adapter",
                    "no_residue_required": True,
                },
            },
            {
                "event_type": "run_finished",
                "payload": {
                    "status": "completed",
                    "no_residue_observed": True,
                    "external_cleanup_required": False,
                },
            },
        ]
        self.events_path.write_text(
            "".join(json.dumps(event) + "\n" for event in events),
            encoding="utf-8",
        )
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GROK-TUI-TEST-001",
            "artifacts": {"events_path": str(self.events_path)},
            "containment": {
                "no_residue_required": True,
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
        }
        emitted: list[dict[str, object]] = []
        run_fn = build_grok_live_run_fn(
            run_root=str(self.run_root),
            workspace=str(ROOT),
            run_id="RUN-GROK-TUI-TEST-001",
            retrieval_mode="framework_fallback",
            retrieval_mode_explicit=True,
        )

        receipt = run_fn(lambda event: emitted.append(event))

        self.assertTrue(receipt["valid"])
        self.assertEqual([event["event_type"] for event in emitted], [
            "run_preflight",
            "run_finished",
        ])
        self.assertTrue(emitted[1]["payload"]["no_residue_observed"])
        mock_run.assert_called_once_with(
            run_root=self.run_root,
            workspace_path=ROOT,
            run_id="RUN-GROK-TUI-TEST-001",
            retrieval_mode="framework_fallback",
            retrieval_mode_explicit=True,
        )

    def test_grok_acp_live_run_fn_passes_fake_provider_to_request(self) -> None:
        captured: dict[str, object] = {}
        events_path = self.events_path

        class FakeSession:
            def __init__(self, request, *args, **kwargs) -> None:
                captured["request"] = request
                self.turn_count = 1
                self.receipt = {
                    "valid": True,
                    "created_at": "2026-08-01T00:00:00Z",
                    "artifacts": {"events_path": str(events_path)},
                }

            def __enter__(self):
                return self

            def __exit__(self, *args) -> None:
                pass

            def send_prompt(self, prompt_text, *, on_acp_event=None):
                if on_acp_event is not None:
                    on_acp_event({
                        "event_type": "acp_initialize",
                        "timestamp": "2026-08-01T00:00:00Z",
                        "payload": {"protocol_version": 1},
                        "redaction": "metadata_only",
                    })
                    on_acp_event({
                        "event_type": "text_delta",
                        "timestamp": "2026-08-01T00:00:01Z",
                        "payload": {"text": "LIF_FAKE_PROVIDER_OK", "turn": 0},
                        "redaction": "metadata_only",
                    })
                return {"stopReason": "end_turn"}

        events = [
            {
                "event_type": "model_output",
                "payload": {
                    "response_sha256": "f" * 64,
                    "structured_output_valid": True,
                },
            },
            {"event_type": "run_finished", "payload": {"status": "completed"}},
        ]
        self.events_path.write_text(
            "".join(json.dumps(event) + "\n" for event in events),
            encoding="utf-8",
        )
        emitted: list[dict[str, object]] = []

        with patch("assurance.grok_runtime_adapter.GrokAcpSession", FakeSession):
            run_fn = build_grok_acp_live_run_fn(
                run_root=str(self.run_root),
                workspace=str(ROOT),
                run_id="RUN-GROK-ACP-TUI-FAKE-001",
                prompt_text="Return the fixture marker only.",
                fake_provider=True,
            )
            receipt = run_fn(lambda event: emitted.append(event))

        request = captured["request"]
        self.assertTrue(request.fake_provider)
        self.assertEqual(request.run_id, "RUN-GROK-ACP-TUI-FAKE-001")
        self.assertTrue(receipt["valid"])
        self.assertIn("text_delta", [event["event_type"] for event in emitted])
        self.assertIn("model_output", [event["event_type"] for event in emitted])
        self.assertTrue((self.run_root / "session.json").is_file())

    @patch("assurance.deepseek_runtime_adapter.run_deepseek_direct_smoke")
    def test_deepseek_live_run_fn_streams_normalized_events(self, mock_run) -> None:
        events = [
            {"event_type": "run_preflight", "payload": {"provider": "deepseek"}},
            {"event_type": "model_output", "payload": {"response_sha256": "a" * 64}},
            {"event_type": "run_finished", "payload": {"status": "completed"}},
        ]
        self.events_path.write_text(
            "".join(json.dumps(event) + "\n" for event in events),
            encoding="utf-8",
        )
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-DEEPSEEK-TUI-TEST-001",
            "artifacts": {"events_path": str(self.events_path)},
        }
        emitted: list[dict[str, object]] = []
        run_fn = build_deepseek_live_run_fn(
            run_root=str(self.run_root),
            run_id="RUN-DEEPSEEK-TUI-TEST-001",
            prompt_text="Return marker",
            credential_target="orz-deepseek/agent-Test",
        )

        receipt = run_fn(lambda event: emitted.append(event))

        self.assertTrue(receipt["valid"])
        self.assertEqual(
            [event["event_type"] for event in emitted],
            ["run_preflight", "model_output", "run_finished"],
        )
        mock_run.assert_called_once_with(
            run_root=self.run_root,
            prompt_text="Return marker",
            credential_target="orz-deepseek/agent-Test",
            run_id="RUN-DEEPSEEK-TUI-TEST-001",
            timeout_seconds=60,
        )

    def test_tui_grok_version_smoke_run_initializes_optional_queues(self) -> None:
        from assurance.tui.main import main as tui_main

        run_fn = lambda on_event: {"valid": True}
        with patch("assurance.tui.bridge.build_grok_live_run_fn") as build_run:
            with patch("assurance.tui.event_source.LiveRunEventSource") as source_cls:
                with patch("assurance.tui.main._interactive_console_available", return_value=True):
                    with patch("assurance.tui.main._run_demo", return_value=0) as run_demo:
                        build_run.return_value = run_fn
                        source_cls.return_value.start.return_value = None

                        exit_code = tui_main([
                            "--runtime", "grok",
                            "--run", "version-smoke",
                            "--run-root", str(self.run_root),
                        ])

        self.assertEqual(exit_code, 0)
        build_run.assert_called_once_with(
            run_root=str(self.run_root),
            workspace=None,
            retrieval_mode="off",
            retrieval_mode_explicit=False,
        )
        source_cls.assert_called_once_with(
            run_fn=run_fn,
            permission_queue=None,
            prompt_queue=None,
        )
        source_cls.return_value.start.assert_called_once_with()
        run_demo.assert_called_once()

    def test_tui_grok_run_uses_static_mode_without_windows_console(self) -> None:
        from assurance.tui.main import main as tui_main

        run_fn = lambda on_event: {"valid": True}
        with patch("assurance.tui.bridge.build_grok_live_run_fn") as build_run:
            with patch("assurance.tui.event_source.LiveRunEventSource") as source_cls:
                with patch("assurance.tui.main._interactive_console_available", return_value=False):
                    with patch("assurance.tui.main._run_demo", return_value=99) as run_demo:
                        with patch("sys.stdout", new_callable=StringIO) as stdout:
                            build_run.return_value = run_fn
                            source_cls.return_value.is_active.return_value = False

                            exit_code = tui_main([
                                "--runtime", "grok",
                                "--run", "version-smoke",
                                "--run-root", str(self.run_root),
                            ])

        self.assertEqual(exit_code, 0)
        self.assertIn("static output mode", stdout.getvalue())
        source_cls.return_value.start.assert_called_once_with()
        source_cls.return_value.close.assert_called_once_with()
        run_demo.assert_not_called()

    def test_tui_grok_acp_static_mode_disables_interactive_loop(self) -> None:
        from assurance.tui.main import main as tui_main

        run_fn = lambda on_event: {"valid": True}
        with patch("assurance.tui.bridge.build_grok_acp_live_run_fn") as build_run:
            with patch("assurance.tui.event_source.LiveRunEventSource") as source_cls:
                with patch("assurance.tui.main._interactive_console_available", return_value=False):
                    with patch("sys.stdout", new_callable=StringIO):
                        build_run.return_value = run_fn
                        source_cls.return_value.is_active.return_value = False

                        exit_code = tui_main([
                            "--runtime", "grok",
                            "--fake-provider",
                            "--run", "hello",
                            "--run-root", str(self.run_root),
                        ])

        self.assertEqual(exit_code, 0)
        self.assertFalse(build_run.call_args.kwargs["interactive"])
        source_cls.assert_called_once_with(
            run_fn=run_fn,
            permission_queue=None,
            prompt_queue=None,
        )

    def test_tui_deepseek_run_uses_direct_runtime_in_static_mode(self) -> None:
        from assurance.tui.main import main as tui_main

        run_fn = lambda on_event: {"valid": True}
        with patch("assurance.tui.bridge.build_deepseek_live_run_fn") as build_run:
            with patch("assurance.tui.event_source.LiveRunEventSource") as source_cls:
                with patch("assurance.tui.main._interactive_console_available", return_value=False):
                    with patch("sys.stdout", new_callable=StringIO):
                        build_run.return_value = run_fn
                        source_cls.return_value.is_active.return_value = False

                        exit_code = tui_main([
                            "--runtime", "deepseek",
                            "--real",
                            "--run", "hello",
                            "--run-root", str(self.run_root),
                            "--credential-target", "orz-deepseek/agent-Test",
                        ])

        self.assertEqual(exit_code, 0)
        build_run.assert_called_once_with(
            run_root=str(self.run_root),
            prompt_text="hello",
            credential_target="orz-deepseek/agent-Test",
        )
        source_cls.assert_called_once_with(
            run_fn=run_fn,
            permission_queue=None,
            prompt_queue=None,
        )
