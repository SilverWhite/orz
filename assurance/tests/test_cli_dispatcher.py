from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
from unittest.mock import patch

from assurance.cli import main as gsa_main


ROOT = Path(__file__).resolve().parents[2]
SOURCE_LEDGER = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"
CREATED_AT = "2026-07-26T12:45:00Z"
ASK = "Check whether the source visibility gate permits this claim."


class GsaCliDispatcherTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "run"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _capture_json(self, argv: list[str]) -> tuple[int, dict[str, object]]:
        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(argv)
        return exit_code, json.loads(output.getvalue())

    def test_doctor_quick_json_reports_offline_boundaries(self) -> None:
        exit_code, report = self._capture_json(["doctor", "--quick", "--json"])

        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertEqual(report["report_kind"], "gsa_cli_doctor_report")
        self.assertIsNone(report["repository_check"])
        self.assertEqual(report["runtime_boundaries"]["default_network"], "disabled")
        self.assertIn("chat", report["entrypoints"])
        self.assertIn("tui", report["entrypoints"])

    def test_source_gate_json_uses_visibility_receipt(self) -> None:
        exit_code, receipt = self._capture_json(
            ["source", "gate", "--ledger", str(SOURCE_LEDGER), "--json"]
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(receipt["receipt_kind"], "source_fulltext_visibility_gate_receipt")
        self.assertEqual(receipt["decision"], "warn")

    def test_run_and_verify_json_roundtrip(self) -> None:
        run_exit, run_receipt = self._capture_json(
            [
                "run",
                "--run-root",
                str(self.run_root),
                "--source-ledger",
                str(SOURCE_LEDGER),
                "--ask",
                ASK,
                "--created-at",
                CREATED_AT,
                "--json",
            ]
        )
        verify_exit, verify_receipt = self._capture_json(
            ["verify", "--run-root", str(self.run_root), "--json"]
        )

        self.assertEqual(run_exit, 0)
        self.assertEqual(verify_exit, 0)
        self.assertEqual(run_receipt, verify_receipt)
        self.assertEqual(run_receipt["terminal_event"], "run_finished")

    def test_run_reports_invalid_task_entry_mode_as_json_error(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root",
                    str(self.run_root),
                    "--source-ledger",
                    str(SOURCE_LEDGER),
                    "--created-at",
                    CREATED_AT,
                    "--json",
                ]
            )
        error = json.loads(error_output.getvalue())

        self.assertEqual(exit_code, 2)
        self.assertFalse(error["valid"])
        self.assertEqual(error["error_type"], "AssuranceError")
        self.assertIn("exactly one", error["error"])
        self.assertFalse(self.run_root.exists())

    def test_run_runtime_grok_json_emits_fail_closed_gate(self) -> None:
        exit_code, receipt = self._capture_json(
            [
                "run",
                "--runtime", "grok",
                "--run-root", str(self.run_root),
                "--ask", "hello Grok",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 1)
        self.assertEqual(
            receipt["receipt_kind"],
            "grok_prompt_tool_promotion_gate_receipt",
        )
        self.assertEqual(receipt["decision"], "block")
        self.assertFalse(receipt["checks"]["prompt_tool_execution_attempted"])
        self.assertTrue(receipt["checks"]["canonical_default_unchanged"])
        self.assertIn(
            "windows_child_tree_owned_cleanup_not_passed",
            receipt["blocking_reasons"],
        )
        self.assertTrue(
            (self.run_root / "grok-prompt-tool-promotion-gate.json").is_file()
        )

    def test_run_canonical_rejects_grok_only_options(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root", str(self.run_root),
                    "--source-ledger", str(SOURCE_LEDGER),
                    "--ask", ASK,
                    "--retrieval-mode", "local_browser",
                    "--json",
                ]
            )

        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("only valid with --runtime grok", error["error"])
        self.assertFalse(self.run_root.exists())

    def test_source_gate_summary_output(self) -> None:
        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                ["source", "gate", "--ledger", str(SOURCE_LEDGER), "--summary"]
            )
        text = output.getvalue()
        self.assertEqual(exit_code, 0)  # decision=defer is not block
        self.assertIn("source visibility gate:", text)

    def test_run_fails_on_nonempty_run_root(self) -> None:
        self.run_root.mkdir(parents=True)
        (self.run_root / "stale-file.txt").write_text("occupied", encoding="utf-8")
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root", str(self.run_root),
                    "--source-ledger", str(SOURCE_LEDGER),
                    "--ask", ASK,
                    "--created-at", CREATED_AT,
                    "--json",
                ]
            )
        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("empty or absent", error["error"])

    @patch("assurance.grok_runtime_adapter.inspect_grok_runtime")
    def test_grok_doctor_json_reports_locked_binary_boundary(self, mock_inspect) -> None:
        mock_inspect.return_value = {
            "valid": True,
            "binary_path": str(ROOT / ".tools" / "grok" / "0.2.112" / "grok.exe"),
            "checks": {"sha256_match": True},
            "observed": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
                "sha256": "2" * 64,
            },
        }

        exit_code, report = self._capture_json(["grok", "doctor", "--json"])

        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertEqual(report["report_kind"], "grok_cli_doctor_report")
        self.assertEqual(report["runtime_boundaries"]["supported_mode"], "version-smoke")
        self.assertEqual(
            report["runtime_boundaries"]["supported_modes"],
            ["version-smoke", "acp-smoke"],
        )
        self.assertEqual(
            report["runtime_boundaries"]["retrieval_modes"],
            ["local_browser", "framework_fallback", "off"],
        )
        self.assertTrue(report["runtime_boundaries"]["no_residue_required"])

    @patch("assurance.grok_runtime_adapter.run_grok_version_smoke")
    def test_grok_run_version_smoke_json_preserves_no_residue(self, mock_run) -> None:
        events_path = self.run_root / "events.jsonl"
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GROK-CLI-TEST-001",
            "request": {
                "run_root": str(self.run_root.resolve()),
                "workspace_path": str(ROOT.resolve()),
                "mode": "version_smoke",
                "retrieval_mode": "local_browser",
                "retrieval_mode_explicit": True,
            },
            "retrieval": {
                "mode": "local_browser",
                "active_for_current_mode": False,
            },
            "binary": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            },
            "containment": {
                "no_residue_required": True,
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
            "artifacts": {
                "events_path": str(events_path.resolve()),
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "grok",
                "run",
                "--mode", "version-smoke",
                "--run-root", str(self.run_root),
                "--workspace", str(ROOT),
                "--retrieval-mode", "local_browser",
                "--run-id", "RUN-GROK-CLI-TEST-001",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["containment"]["no_residue_required"])
        self.assertTrue(receipt["containment"]["no_residue_observed"])
        mock_run.assert_called_once_with(
            run_root=self.run_root,
            workspace_path=ROOT,
            run_id="RUN-GROK-CLI-TEST-001",
            retrieval_mode="local_browser",
            retrieval_mode_explicit=True,
        )

    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_grok_run_acp_smoke_json_requires_prompt_and_runs_adapter(self, mock_run) -> None:
        events_path = self.run_root / "events.jsonl"
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GROK-ACP-CLI-TEST-001",
            "request": {
                "run_root": str(self.run_root.resolve()),
                "workspace_path": str((self.run_root / "workspace").resolve()),
                "mode": "acp_smoke",
                "retrieval_mode": "off",
                "retrieval_mode_explicit": False,
            },
            "retrieval": {
                "mode": "off",
                "active_for_current_mode": False,
            },
            "binary": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            },
            "containment": {
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
            "artifacts": {
                "events_path": str(events_path.resolve()),
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "grok",
                "run",
                "--mode", "acp-smoke",
                "--run-root", str(self.run_root),
                "--run-id", "RUN-GROK-ACP-CLI-TEST-001",
                "--ask", "hello ACP",
                "--fake-provider",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        request = mock_run.call_args.args[0]
        self.assertEqual(request.mode, "acp_smoke")
        self.assertEqual(request.prompt_text, "hello ACP")
        self.assertEqual(request.workspace_path, self.run_root / "workspace")
        self.assertTrue(request.fake_provider)

    @patch("assurance.deepseek_adapter._read_windows_credential")
    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_grok_run_acp_smoke_real_deepseek_reads_credential_without_serializing_secret(
        self, mock_run, mock_read_credential
    ) -> None:
        mock_read_credential.return_value = "TEST-SECRET-DO-NOT-SERIALIZE"
        events_path = self.run_root / "events.jsonl"
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GROK-REAL-ACP-CLI-TEST-001",
            "request": {
                "run_root": str(self.run_root.resolve()),
                "workspace_path": str((self.run_root / "workspace").resolve()),
                "mode": "acp_smoke",
                "retrieval_mode": "off",
                "retrieval_mode_explicit": False,
                "fake_provider": False,
            },
            "retrieval": {
                "mode": "off",
                "active_for_current_mode": False,
            },
            "binary": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            },
            "containment": {
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
            "artifacts": {
                "events_path": str(events_path.resolve()),
            },
        }

        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                [
                    "grok",
                    "run",
                    "--mode", "acp-smoke",
                    "--run-root", str(self.run_root),
                    "--run-id", "RUN-GROK-REAL-ACP-CLI-TEST-001",
                    "--ask", "reply with a fixed marker",
                    "--real-deepseek",
                    "--credential-target", "orz-deepseek/agent-Test",
                    "--json",
                ]
            )
        receipt = json.loads(output.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertNotIn("TEST-SECRET-DO-NOT-SERIALIZE", output.getvalue())
        mock_read_credential.assert_called_once_with("orz-deepseek/agent-Test")
        request = mock_run.call_args.args[0]
        self.assertFalse(request.fake_provider)
        self.assertEqual(request.model_id, "lif-deepseek-v4-pro")
        self.assertFalse(request.disable_builtin_tools)
        self.assertIn('env_key = "LIF_DEEPSEEK_API_KEY"', request.model_config_toml)
        self.assertEqual(
            request.provider_environment["LIF_DEEPSEEK_API_KEY"],
            "TEST-SECRET-DO-NOT-SERIALIZE",
        )
        self.assertEqual(request.provider_environment["HTTPS_PROXY"], "")

    def test_grok_real_deepseek_model_config_overrides_builtin_defaults(self) -> None:
        import tomllib

        from assurance.cli import _grok_real_deepseek_model_config

        config = tomllib.loads(_grok_real_deepseek_model_config())

        self.assertEqual(config["models"]["default"], "lif-deepseek-v4-pro")
        self.assertEqual(config["models"]["web_search"], "lif-deepseek-v4-pro")
        self.assertEqual(config["models"]["default_reasoning_effort"], "none")
        self.assertFalse(config["models"]["stream_tool_calls"])
        primary = config["model"]["lif-deepseek-v4-pro"]
        builtin_override = config["model"]["grok-4.5"]
        self.assertEqual(primary["model"], "deepseek-v4-pro")
        self.assertEqual(builtin_override["model"], "deepseek-v4-pro")
        self.assertEqual(builtin_override["base_url"], "https://api.deepseek.com")
        self.assertFalse(primary["stream_tool_calls"])
        self.assertFalse(builtin_override["stream_tool_calls"])
        self.assertEqual(primary["thinking"]["type"], "disabled")
        self.assertEqual(builtin_override["thinking"]["type"], "disabled")
        self.assertEqual(primary["extra_body"]["thinking"]["type"], "disabled")
        self.assertEqual(
            builtin_override["extra_body"]["thinking"]["type"], "disabled"
        )
        self.assertNotIn("api_key", primary)
        self.assertNotIn("api_key", builtin_override)
        self.assertEqual(primary["env_key"], "LIF_DEEPSEEK_API_KEY")
        self.assertEqual(builtin_override["env_key"], "LIF_DEEPSEEK_API_KEY")

    def test_grok_run_acp_smoke_rejects_missing_prompt(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "grok",
                    "run",
                    "--mode", "acp-smoke",
                    "--run-root", str(self.run_root),
                    "--json",
                ]
            )

        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("--ask is required", error["error"])

    def test_grok_run_version_smoke_rejects_fake_provider(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "grok",
                    "run",
                    "--mode", "version-smoke",
                    "--run-root", str(self.run_root),
                    "--fake-provider",
                    "--json",
                ]
            )

        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("only valid with --mode acp-smoke", error["error"])

    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_alpha_smoke_json_runs_fixed_fake_provider_acp_path(self, mock_run) -> None:
        events_path = self.run_root / "events.jsonl"
        receipt_path = self.run_root / "grok-runtime-receipt.json"
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GSA-ALPHA-SMOKE-0123ABCD",
            "request": {
                "run_root": str(self.run_root.resolve()),
                "workspace_path": str((self.run_root / "workspace").resolve()),
                "mode": "acp_smoke",
                "retrieval_mode": "off",
                "retrieval_mode_explicit": False,
            },
            "retrieval": {
                "mode": "off",
                "active_for_current_mode": False,
            },
            "binary": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            },
            "containment": {
                "no_residue_observed": True,
                "external_cleanup_required": False,
            },
            "artifacts": {
                "events_path": str(events_path.resolve()),
                "receipt_path": str(receipt_path.resolve()),
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "alpha",
                "smoke",
                "--run-root", str(self.run_root),
                "--ask", "alpha hello",
                "--run-id", "RUN-GSA-ALPHA-SMOKE-0123ABCD",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        request = mock_run.call_args.args[0]
        self.assertEqual(request.mode, "acp_smoke")
        self.assertEqual(request.prompt_text, "alpha hello")
        self.assertEqual(request.workspace_path, self.run_root / "workspace")
        self.assertEqual(request.retrieval_mode, "off")
        self.assertFalse(request.retrieval_mode_explicit)
        self.assertTrue(request.fake_provider)

    @patch("assurance.grok_runtime_adapter.inspect_grok_runtime")
    def test_alpha_ui_check_json_reports_dual_ui_strategy(self, mock_inspect) -> None:
        mock_inspect.return_value = {
            "valid": True,
            "binary_path": str(ROOT / ".tools" / "grok" / "0.2.112" / "grok.exe"),
            "observed": {"version_output": "grok 0.2.112 (9bbd559437)"},
        }

        exit_code, receipt = self._capture_json(
            [
                "alpha",
                "ui-check",
                "--run-root", str(self.run_root),
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "alpha_ui_usability_check_receipt")
        self.assertEqual(receipt["ui_strategy"]["primary"], "gsa_tui")
        self.assertEqual(receipt["ui_strategy"]["fallback"], "grok_native_ui")
        self.assertTrue(receipt["checks"]["self_ui_static_rendered"])
        self.assertTrue(receipt["checks"]["self_ui_event_projection_drained"])
        self.assertTrue(receipt["checks"]["grok_fallback_binary_valid"])
        self.assertEqual(receipt["live_smoke"]["status"], "skipped")
        self.assertTrue((self.run_root / "alpha-ui-check.json").is_file())

    @patch("assurance.grok_runtime_adapter.inspect_grok_runtime")
    @patch("assurance.tui.bridge.build_grok_acp_live_run_fn")
    def test_alpha_ui_check_live_static_smoke_uses_noninteractive_acp(
        self, mock_build_run, mock_inspect
    ) -> None:
        mock_inspect.return_value = {
            "valid": True,
            "binary_path": str(ROOT / ".tools" / "grok" / "0.2.112" / "grok.exe"),
            "observed": {"version_output": "grok 0.2.112 (9bbd559437)"},
        }

        def _run_fn(on_event):
            on_event({
                "event_type": "user_message",
                "payload": {"text": "alpha UI", "turn": 0},
            })
            on_event({
                "event_type": "text_delta",
                "payload": {"text": "LIF_FAKE_PROVIDER_OK", "turn": 0},
            })
            on_event({
                "event_type": "run_finished",
                "payload": {"status": "completed"},
            })
            return {
                "valid": True,
                "artifacts": {"receipt_path": str(self.run_root / "live-receipt.json")},
            }

        mock_build_run.return_value = _run_fn

        exit_code, receipt = self._capture_json(
            [
                "alpha",
                "ui-check",
                "--run-root", str(self.run_root),
                "--run-live-smoke",
                "--ask", "alpha UI",
                "--run-id", "RUN-GSA-ALPHA-UI-TEST-001",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["live_smoke"]["status"], "completed")
        self.assertTrue(receipt["checks"]["live_static_acp_marker_rendered"])
        self.assertFalse(mock_build_run.call_args.kwargs["interactive"])
        self.assertTrue((self.run_root / "live-static-tui-output.txt").is_file())

    @patch("assurance.grok_tool_permission_observer.build_grok_tool_permission_observation_bundle")
    @patch("assurance.grok_tool_permission_observer.observe_grok_tool_permission_surfaces")
    def test_alpha_tool_check_json_requires_block_then_allow(
        self, mock_observe, mock_bundle
    ) -> None:
        negative_observation = {
            "decision": "defer",
            "checks": {"no_model_invoked": True, "no_network_requested": True},
        }
        positive_observation = {
            "decision": "allow",
            "checks": {"no_model_invoked": True, "no_network_requested": True},
            "acp_permission_observation": {
                "required_scenarios_verified": True,
                "covered_scenarios": ["allow_once", "cancel_permission"],
                "permission_outcomes": ["allow_once", "cancelled"],
            },
        }
        mock_observe.side_effect = [negative_observation, positive_observation]
        mock_bundle.side_effect = [
            {
                "observation_receipt": negative_observation,
                "tool_availability_report": {
                    "degraded": [{"tool_id": "grok_acp_permission_probe"}],
                    "unavailable": [],
                    "unprobed": [],
                },
                "tool_availability_gate_receipt": {
                    "decisions": {"gate_decision": "block"},
                },
            },
            {
                "observation_receipt": positive_observation,
                "tool_availability_report": {
                    "degraded": [],
                    "unavailable": [],
                    "unprobed": [],
                },
                "tool_availability_gate_receipt": {
                    "decisions": {"gate_decision": "allow"},
                    "available_count": 5,
                },
            },
        ]

        # Create mock ACP verification files so the default path lookup succeeds
        (self.run_root / "acp-allow").mkdir(parents=True, exist_ok=True)
        (self.run_root / "acp-allow" / "verification.json").write_text(
            '{"verification_id": "fake-allow", "valid": true}'
        )
        (self.run_root / "acp-allow" / "result.json").write_text('{}')
        (self.run_root / "acp-cancel").mkdir(parents=True, exist_ok=True)
        (self.run_root / "acp-cancel" / "verification.json").write_text(
            '{"verification_id": "fake-cancel", "valid": true}'
        )
        (self.run_root / "acp-cancel" / "result.json").write_text('{}')

        exit_code, receipt = self._capture_json(
            [
                "alpha",
                "tool-check",
                "--run-root", str(self.run_root),
                "--acp-verification",
                str(self.run_root / "acp-allow" / "verification.json"),
                "--acp-verification",
                str(self.run_root / "acp-cancel" / "verification.json"),
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "alpha_tool_check_receipt")
        self.assertTrue(receipt["checks"]["negative_without_acp_probe_blocks"])
        self.assertTrue(receipt["checks"]["positive_tool_availability_allows"])
        self.assertEqual(receipt["negative_case"]["gate_decision"], "block")
        self.assertEqual(receipt["positive_case"]["gate_decision"], "allow")
        self.assertEqual(mock_observe.call_args_list[0].kwargs["acp_verification_paths"], ())
        self.assertEqual(len(mock_observe.call_args_list[1].kwargs["acp_verification_paths"]), 2)
        self.assertTrue((self.run_root / "alpha-tool-check.json").is_file())

    @patch("assurance.grok_tool_permission_observer.build_grok_tool_permission_observation_bundle")
    @patch("assurance.grok_tool_permission_observer.observe_grok_tool_permission_surfaces")
    def test_alpha_tool_check_missing_verifications_fails_positive_case(
        self, mock_observe, mock_bundle
    ) -> None:
        negative_observation = {
            "decision": "defer",
            "checks": {"no_model_invoked": True, "no_network_requested": True},
        }
        mock_observe.return_value = negative_observation
        mock_bundle.return_value = {
            "observation_receipt": negative_observation,
            "tool_availability_report": {
                "degraded": [{"tool_id": "grok_acp_permission_probe"}],
                "unavailable": [],
                "unprobed": [],
            },
            "tool_availability_gate_receipt": {
                "decisions": {"gate_decision": "block"},
            },
        }
        missing = self.run_root / "missing-verification.json"

        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                [
                    "alpha",
                    "tool-check",
                    "--run-root", str(self.run_root),
                    "--acp-verification", str(missing),
                    "--json",
                ]
            )
        receipt = json.loads(output.getvalue())

        self.assertEqual(exit_code, 1)
        self.assertFalse(receipt["valid"])
        self.assertFalse(receipt["checks"]["verification_paths_present"])
        self.assertEqual(receipt["positive_case"]["gate_decision"], "missing")
        mock_observe.assert_called_once_with(acp_verification_paths=())

    @patch("assurance.deepseek_adapter._read_windows_credential")
    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_alpha_real_call_json_runs_real_deepseek_without_serializing_secret(
        self, mock_run, mock_read_credential
    ) -> None:
        mock_read_credential.return_value = "TEST-ALPHA-REAL-SECRET"
        adapter_receipt_path = self.run_root / "grok-real-acp" / "grok-runtime-receipt.json"
        events_path = self.run_root / "grok-real-acp" / "events.jsonl"
        mock_run.return_value = {
            "valid": True,
            "run_id": "RUN-GSA-ALPHA-REAL-TEST-001",
            "request": {
                "run_root": str((self.run_root / "grok-real-acp").resolve()),
                "workspace_path": str(
                    (self.run_root / "grok-real-acp" / "workspace").resolve()
                ),
                "mode": "acp_smoke",
                "retrieval_mode": "off",
                "retrieval_mode_explicit": False,
                "fake_provider": False,
                "disable_builtin_tools": False,
            },
            "retrieval": {
                "mode": "off",
                "active_for_current_mode": False,
            },
            "binary": {
                "version_output": "grok 0.2.112 (9bbd559437) [stable]",
            },
            "checks": {
                "binary_inspection_valid": True,
                "workspace_trust_valid": True,
                "workspace_trust_granted": True,
                "acp_initialize_ok": True,
                "acp_session_created": True,
                "acp_prompt_completed": True,
                "acp_response_received": True,
                "provider_api_errors_absent": True,
            },
            "prompt": {
                "response_sha256": "a" * 64,
            },
            "containment": {
                "no_residue_observed": True,
            },
            "artifacts": {
                "receipt_path": str(adapter_receipt_path.resolve()),
                "events_path": str(events_path.resolve()),
            },
        }

        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                [
                    "alpha",
                    "real-call",
                    "--run-root", str(self.run_root),
                    "--ask", "Return exactly: GSA_ALPHA_REAL_CALL_OK",
                    "--run-id", "RUN-GSA-ALPHA-REAL-TEST-001",
                    "--credential-target", "orz-deepseek/agent-Test",
                    "--json",
                ]
            )
        receipt = json.loads(output.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "alpha_real_call_receipt")
        self.assertNotIn("TEST-ALPHA-REAL-SECRET", output.getvalue())
        self.assertNotIn(
            "TEST-ALPHA-REAL-SECRET",
            (self.run_root / "alpha-real-call.json").read_text(encoding="utf-8"),
        )
        self.assertTrue(receipt["checks"]["credential_read_succeeded"])
        self.assertTrue(receipt["checks"]["fake_provider_disabled"])
        self.assertTrue(receipt["checks"]["builtin_tools_enabled"])
        self.assertTrue(receipt["checks"]["retrieval_off"])
        self.assertTrue(receipt["checks"]["no_secret_serialized"])
        mock_read_credential.assert_called_once_with("orz-deepseek/agent-Test")
        request = mock_run.call_args.args[0]
        self.assertFalse(request.fake_provider)
        self.assertEqual(request.model_id, "lif-deepseek-v4-pro")
        self.assertEqual(request.retrieval_mode, "off")
        self.assertFalse(request.retrieval_mode_explicit)
        self.assertFalse(request.disable_builtin_tools)
        self.assertIn('env_key = "LIF_DEEPSEEK_API_KEY"', request.model_config_toml)
        self.assertEqual(
            request.provider_environment["LIF_DEEPSEEK_API_KEY"],
            "TEST-ALPHA-REAL-SECRET",
        )
        self.assertEqual(request.provider_environment["HTTPS_PROXY"], "")

    @patch("assurance.deepseek_adapter._read_windows_credential")
    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_alpha_real_call_credential_failure_returns_structured_failure(
        self, mock_run, mock_read_credential
    ) -> None:
        mock_read_credential.side_effect = OSError("credential not found")

        exit_code, receipt = self._capture_json(
            [
                "alpha",
                "real-call",
                "--run-root", str(self.run_root),
                "--credential-target", "orz-deepseek/missing",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 1)
        self.assertFalse(receipt["valid"])
        self.assertFalse(receipt["checks"]["credential_read_succeeded"])
        self.assertTrue(receipt["checks"]["no_secret_serialized"])
        self.assertIn("credential not found", receipt["provider"]["credential_error"])
        self.assertFalse(receipt["adapter"]["valid"])
        self.assertTrue((self.run_root / "alpha-real-call.json").is_file())
        mock_run.assert_not_called()

    @patch("assurance.deepseek_adapter._read_windows_credential")
    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    @patch("assurance.deepseek_runtime_adapter.run_deepseek_direct_once")
    def test_alpha_real_call_direct_deepseek_uses_fixed_prompt_without_grok(
        self, mock_direct, mock_run, mock_read_credential
    ) -> None:
        mock_read_credential.return_value = "TEST-DIRECT-SECRET"
        mock_direct.return_value = {
            "valid": True,
            "artifacts": {
                "receipt_path": str((self.run_root / "deepseek-runtime-receipt.json").resolve()),
                "events_path": str((self.run_root / "events.jsonl").resolve()),
            },
            "execution": {
                "http_status_code": 200,
            },
            "prompt": {
                "response_sha256": "a" * 64,
                "response_finish_reason": "stop",
                "model_id": "deepseek-v4-pro",
                "usage": {
                    "prompt_tokens": 9,
                    "completion_tokens": 6,
                    "total_tokens": 15,
                },
                "private_reasoning_content_sha256": None,
            },
        }

        output = StringIO()
        with redirect_stdout(output):
            exit_code = gsa_main(
                [
                    "alpha",
                    "real-call",
                    "--transport", "direct-deepseek",
                    "--run-root", str(self.run_root),
                    "--ask", "Return exactly: GSA_ALPHA_REAL_CALL_OK",
                    "--credential-target", "orz-deepseek/agent-Test",
                    "--json",
                ]
        )
        receipt = json.loads(output.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["request"]["transport"], "direct-deepseek")
        self.assertTrue(receipt["direct_provider"]["valid"])
        self.assertTrue(receipt["checks"]["direct_http_status_ok"])
        self.assertTrue(receipt["checks"]["direct_response_received"])
        self.assertNotIn("TEST-DIRECT-SECRET", output.getvalue())
        self.assertNotIn(
            "TEST-DIRECT-SECRET",
            (self.run_root / "alpha-real-call.json").read_text(encoding="utf-8"),
        )
        mock_run.assert_not_called()
        mock_direct.assert_called_once()
        request = mock_direct.call_args.args[0]
        self.assertEqual(request.prompt_text, "Return exactly: GSA_ALPHA_REAL_CALL_OK")
        self.assertEqual(request.credential_target, "orz-deepseek/agent-Test")
        self.assertEqual(request.run_root, self.run_root / "grok-real-acp")
        self.assertEqual(mock_direct.call_args.kwargs["credential_reader"]("ignored"), "TEST-DIRECT-SECRET")

    @patch("assurance.deepseek_runtime_adapter.run_deepseek_direct_smoke")
    def test_deepseek_run_json_uses_direct_runtime_adapter(self, mock_direct) -> None:
        mock_direct.return_value = {
            "valid": True,
            "run_id": "RUN-DEEPSEEK-DIRECT-CLI-TEST-001",
            "request": {"run_root": str(self.run_root.resolve())},
            "execution": {"http_status_code": 200},
            "artifacts": {
                "events_path": str((self.run_root / "events.jsonl").resolve()),
                "receipt_path": str(
                    (self.run_root / "deepseek-runtime-receipt.json").resolve()
                ),
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "deepseek",
                "run",
                "--run-root", str(self.run_root),
                "--ask", "Return marker",
                "--run-id", "RUN-DEEPSEEK-DIRECT-CLI-TEST-001",
                "--credential-target", "orz-deepseek/agent-Test",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        mock_direct.assert_called_once_with(
            run_root=self.run_root,
            prompt_text="Return marker",
            credential_target="orz-deepseek/agent-Test",
            run_id="RUN-DEEPSEEK-DIRECT-CLI-TEST-001",
            timeout_seconds=60,
        )

    @patch("assurance.deepseek_runtime_adapter.run_deepseek_direct_smoke")
    def test_run_runtime_deepseek_requires_real_and_uses_direct_adapter(
        self, mock_direct
    ) -> None:
        mock_direct.return_value = {
            "valid": True,
            "run_id": "RUN-DEEPSEEK-DIRECT-RUNTIME-TEST-001",
            "request": {"run_root": str(self.run_root.resolve())},
            "execution": {"http_status_code": 200},
            "artifacts": {
                "events_path": str((self.run_root / "events.jsonl").resolve()),
                "receipt_path": str(
                    (self.run_root / "deepseek-runtime-receipt.json").resolve()
                ),
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "run",
                "--runtime", "deepseek",
                "--mode", "real",
                "--run-root", str(self.run_root),
                "--ask", "Return marker",
                "--run-id", "RUN-DEEPSEEK-DIRECT-RUNTIME-TEST-001",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        mock_direct.assert_called_once()

        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--runtime", "deepseek",
                    "--run-root", str(self.run_root / "fake"),
                    "--ask", "Return marker",
                    "--json",
                ]
            )
        self.assertEqual(exit_code, 2)
        self.assertIn("requires --mode real", error_output.getvalue())

    @patch("assurance.grok_tool_permission_observer.observe_grok_tool_permission_surfaces")
    def test_grok_observe_tools_json_reports_permission_surface(self, mock_observe) -> None:
        mock_observe.return_value = {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "grok_tool_permission_observation_receipt",
            "receipt_id": "GROK-TOOL-PERM-0123456789ABCDEF",
            "valid": True,
            "decision": "defer",
            "observed_at": "2026-07-30T00:00:00Z",
            "runtime_owner": "grok",
            "runtime_id": "GROK-0.2.112",
            "inputs": {
                "inspect_source": "fixture",
                "grok_help_source": "fixture",
                "agent_help_source": "fixture",
                "acp_verification_count": 0,
            },
            "surface": {
                "grok_version": "0.2.112",
                "project_trusted": False,
                "project_agent_count": 2,
                "builtin_agent_count": 2,
                "project_agents": [
                    "gsa-external-retrieval",
                    "gsa-project-doc-retrieval",
                ],
                "builtin_agents": ["explore", "general-purpose"],
                "mcp_server_count": 0,
                "permissions": {
                    "loaded": 0,
                    "skipped_count": 0,
                    "sources_count": 0,
                    "managed_settings_active": False,
                },
            },
            "controls": {
                "grok_help_flags": {
                    "--allow": True,
                    "--deny": True,
                    "--permission-mode": True,
                    "--always-approve": True,
                    "--disable-web-search": True,
                    "--tools": True,
                    "--disallowed-tools": True,
                },
                "agent_help_flags": {
                    "--agent-profile": True,
                    "--always-approve": True,
                },
                "permission_control_flags_present": True,
                "tool_filter_flags_present": True,
                "web_disable_flag_present": True,
                "agent_profile_flag_present": True,
            },
            "acp_permission_observation": {
                "attached": False,
                "valid_count": 0,
                "invalid_count": 0,
                "probe_ids": [],
                "result_sha256": [],
                "covered_scenarios": [],
                "required_scenarios": ["allow_once", "cancel_permission"],
                "required_scenarios_verified": False,
                "permission_outcomes": [],
                "provider_scenarios": [],
                "scenario_semantics_verified": False,
                "safety_checks_verified": False,
            },
            "checks": {
                "grok_inspect_observed": True,
                "grok_help_observed": True,
                "agent_help_observed": True,
                "expected_project_agents_discovered": True,
                "permission_controls_observed": True,
                "agent_profile_control_observed": True,
                "web_disable_control_observed": True,
                "acp_permission_probe_attached": False,
                "acp_permission_probe_valid_when_attached": True,
                "acp_permission_required_scenarios_verified_when_attached": True,
                "no_model_invoked": True,
                "no_network_requested": True,
                "prompt_tool_promotion_blocked": True,
            },
            "limitations": [
                "fixture observation only.",
            ],
        }

        exit_code, bundle = self._capture_json(
            ["grok", "observe-tools", "--include-tool-availability", "--json"]
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            bundle["observation_receipt"]["receipt_kind"],
            "grok_tool_permission_observation_receipt",
        )
        self.assertEqual(
            bundle["tool_availability_gate_receipt"]["decisions"]["gate_decision"],
            "block",
        )
        mock_observe.assert_called_once_with(acp_verification_paths=[])

    @patch("assurance.tui.main.main")
    def test_tui_runtime_grok_dispatches_to_tui_main(self, mock_tui_main) -> None:
        mock_tui_main.return_value = 0

        exit_code = gsa_main(
            [
                "tui",
                "--runtime", "grok",
                "--run", "version-smoke",
                "--fake-provider",
                "--run-root", str(self.run_root),
                "--workspace", str(ROOT),
                "--retrieval-mode", "framework_fallback",
            ]
        )

        self.assertEqual(exit_code, 0)
        forwarded = mock_tui_main.call_args.args[0]
        self.assertIn("--runtime", forwarded)
        self.assertIn("grok", forwarded)
        self.assertIn("--fake-provider", forwarded)
        self.assertIn("--workspace", forwarded)
        self.assertIn(str(ROOT), forwarded)
        self.assertIn("--retrieval-mode", forwarded)
        self.assertIn("framework_fallback", forwarded)

    @patch("assurance.tui.main.main")
    def test_tui_runtime_deepseek_dispatches_real_to_tui_main(
        self, mock_tui_main
    ) -> None:
        mock_tui_main.return_value = 0

        exit_code = gsa_main(
            [
                "tui",
                "--runtime", "deepseek",
                "--real",
                "--run", "Return marker",
                "--run-root", str(self.run_root),
                "--credential-target", "orz-deepseek/agent-Test",
            ]
        )

        self.assertEqual(exit_code, 0)
        forwarded = mock_tui_main.call_args.args[0]
        self.assertIn("--runtime", forwarded)
        self.assertIn("deepseek", forwarded)
        self.assertIn("--real", forwarded)
        self.assertIn("--credential-target", forwarded)
        self.assertIn("orz-deepseek/agent-Test", forwarded)

    def test_tui_runtime_deepseek_requires_real(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "tui",
                    "--runtime", "deepseek",
                    "--run", "Return marker",
                    "--run-root", str(self.run_root),
                ]
            )

        self.assertEqual(exit_code, 2)
        self.assertIn("requires --real", error_output.getvalue())

    def test_review_global_json_activates_explicit_mode(self) -> None:
        exit_code, receipt = self._capture_json(
            [
                "review",
                "global",
                "--no-git-status",
                "--path", "CLI_PROJECT_INDEX.md",
                "--path", "assurance/global_review_mode.py",
                "--json",
            ]
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "global_review_mode_receipt")
        self.assertTrue(receipt["mode_activation"]["explicit_only"])
        self.assertEqual(receipt["scope"]["source"], "explicit_paths")
        self.assertEqual(len(receipt["dimensions"]), 5)
        self.assertEqual(
            [item["dimension_id"] for item in receipt["dimensions"]],
            [
                "design_intent_alignment",
                "current_progress_judgment",
                "implementation_content_positioning",
                "critical_design_preservation",
                "project_task_boundary",
            ],
        )

    @patch("assurance.canonical_cli.build_real_deepseek_context")
    @patch("assurance.canonical_cli._read_windows_credential")
    @patch("assurance.canonical_cli.call_deepseek_api")
    def test_run_real_mode_mocked_api_success(
        self, mock_api, mock_cred, mock_context
    ) -> None:
        mock_cred.return_value = "sk-fake-key-0011223344556677"
        mock_context.return_value = [
            {"role": "system", "content": "Gate context."},
            {"role": "user", "content": ASK},
        ]
        mock_api.return_value = {
            "public_assistant_text": "The mechanism involves caspase-3 activation.",
            "finish_reason": "stop",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
            },
            "model": "deepseek-v4-pro",
            "private_reasoning_content_sha256": None,
            "http_status_code": 200,
        }
        exit_code, receipt = self._capture_json(
            [
                "run",
                "--mode", "real",
                "--run-root", str(self.run_root),
                "--source-ledger", str(SOURCE_LEDGER),
                "--ask", ASK,
                "--created-at", CREATED_AT,
                "--json",
            ]
        )
        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["terminal_event"], "run_finished")
        self.assertTrue(receipt["checks"]["real_network_used"])
        self.assertIn("real_network_used", receipt["checks"])
        # Verify that the mock API was called exactly once
        mock_cred.assert_called_once()
        mock_api.assert_called_once()

    @patch("assurance.canonical_cli._read_windows_credential")
    def test_run_real_mode_credential_failure_fallbacks(
        self, mock_cred
    ) -> None:
        mock_cred.side_effect = OSError("credential not found")
        exit_code, receipt = self._capture_json(
            [
                "run",
                "--mode", "real",
                "--run-root", str(self.run_root),
                "--source-ledger", str(SOURCE_LEDGER),
                "--ask", ASK,
                "--created-at", CREATED_AT,
                "--json",
            ]
        )
        # Real adapter failure produces a valid receipt with run_failed
        self.assertEqual(exit_code, 0)
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["terminal_event"], "run_failed")

    def test_run_with_task_contract_file(self) -> None:
        task_contract_path = self.run_root.parent / "task-contract.json"
        task_contract = {
            "schema_version": "0.1.0-draft",
            "contract_kind": "canonical_cli_task_contract",
            "task_id": "TASK-CLI-TEST-002",
            "created_at": CREATED_AT,
            "entry_mode": "task_file",
            "user_request": {
                "raw_text": ASK,
                "normalized_intent": ASK.strip(),
            },
            "source_ledger": {
                "path": str(SOURCE_LEDGER),
                "sha256": "a" * 64,
                "required": True,
            },
            "output_contract": {
                "format": "canonical_cli_answer_packet",
                "must_include_source_visibility_summary": True,
                "must_include_claim_boundaries": True,
                "must_include_next_actions": True,
            },
            "permissions": {
                "network_allowed": False,
                "real_model_allowed": False,
                "tool_calls_allowed": False,
                "workspace_writes_allowed": False,
                "incremental_retrieval_allowed": True,
            },
            "claim_policy": {
                "allow_bibliographic_claims_without_fulltext": True,
                "require_fulltext_for_mechanism": True,
                "require_fulltext_for_methods": True,
            },
        }
        task_contract_path.write_text(
            json.dumps(task_contract), encoding="utf-8"
        )
        # Note: task_file entry_mode requires a real source ledger at the
        # same path, which we already have.  The task_contract validation
        # will check that the source ledger exists and matches.
        # Since our contract references a SHA that does not match the
        # actual ledger, we only test the --task dispatch path here.
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--run-root", str(self.run_root),
                    "--task", str(task_contract_path),
                    "--created-at", CREATED_AT,
                    "--json",
                ]
            )
        # The sha256 mismatch will cause an AssuranceError
        error = json.loads(error_output.getvalue())
        self.assertEqual(exit_code, 2)
        self.assertFalse(error["valid"])
        self.assertEqual(error["error_type"], "AssuranceError")

    def test_doctor_full_repository_check(self) -> None:
        exit_code, report = self._capture_json(["doctor", "--json"])
        self.assertEqual(exit_code, 0)
        self.assertTrue(report["valid"])
        self.assertIsNotNone(report["repository_check"])
        completed = subprocess.run(
            [sys.executable, "gsa.py", "doctor", "--quick", "--json"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=30,
        )

        self.assertEqual(completed.returncode, 0, completed.stderr)
        report = json.loads(completed.stdout)
        self.assertEqual(report["report_kind"], "gsa_cli_doctor_report")

    def test_grok_observe_tools_output_gate_receipt_writes_file(self) -> None:
        from assurance.grok_tool_permission_observer import GrokToolPermissionObservationConfig

        if not GrokToolPermissionObservationConfig().binary_path.exists():
            self.skipTest("Grok binary not installed")
        temp_dir = Path(self.temporary.name)
        gate_path = temp_dir / "tool-gate.json"
        exit_code, _stdout = self._capture_json(
            [
                "grok",
                "observe-tools",
                "--json",
                "--output-gate-receipt",
                str(gate_path),
            ],
        )

        self.assertEqual(exit_code, 0)
        self.assertTrue(gate_path.is_file())
        gate = json.loads(gate_path.read_text(encoding="utf-8"))
        self.assertEqual(gate["receipt_kind"], "tool_availability_gate_receipt")
        self.assertIn("gate_decision", gate["decisions"])

    def test_run_runtime_grok_allow_with_dual_acp_and_containment(
        self,
    ) -> None:
        temp_dir = Path(self.temporary.name)
        gate_path = temp_dir / "tool-gate.json"
        adapter_path = temp_dir / "adapter-receipt.json"
        gate_path.write_text(
            json.dumps(
                {
                    "schema_version": "0.1.0-draft",
                    "receipt_kind": "tool_availability_gate_receipt",
                    "valid": True,
                    "report_id": "TOOL-AVAIL-GROK-001",
                    "report_sha256": "a" * 64,
                    "available_count": 6,
                    "unavailable_count": 0,
                    "unprobed_count": 0,
                    "degraded_count": 0,
                    "context_block_sha256": "b" * 64,
                    "decisions": {
                        "gate_decision": "allow",
                        "context_injected": True,
                        "model_must_not_guess": True,
                    },
                }
            ),
            encoding="utf-8",
        )
        adapter_path.write_text(
            json.dumps(
                {
                    "schema_version": "0.1.0",
                    "receipt_kind": "grok_runtime_adapter_receipt",
                    "valid": True,
                    "containment": {
                        "no_residue_required": True,
                        "no_residue_observed": True,
                        "root_process_exited": True,
                        "external_cleanup_required": False,
                        "residue_scan_scope": "job_object_contained",
                        "job_object_created": True,
                        "job_object_assigned": True,
                        "containment_provider": "adapter_job_object",
                        "containment_available": True,
                    },
                }
            ),
            encoding="utf-8",
        )

        exit_code, receipt = self._capture_json(
            [
                "run",
                "--runtime",
                "grok",
                "--ask",
                "review the project",
                "--run-root",
                str(self.run_root),
                "--grok-tool-availability-gate",
                str(gate_path),
                "--grok-adapter-receipt",
                str(adapter_path),
                "--retrieval-mode",
                "local_browser",
                "--json",
            ],
        )

        self.assertEqual(receipt["decision"], "allow")
        self.assertEqual(receipt["blocking_reasons"], [])
        self.assertTrue(receipt["checks"]["tool_availability_gate_allows"])
        self.assertTrue(receipt["checks"]["containment_requirement_satisfied"])
        self.assertFalse(receipt["checks"]["prompt_tool_execution_attempted"])

    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    def test_run_runtime_grok_execute_uses_separate_execution_root(
        self, mock_acp
    ) -> None:
        temp_dir = Path(self.temporary.name)
        gate_path = temp_dir / "tool-gate.json"
        adapter_path = temp_dir / "adapter-receipt.json"
        events_path = self.run_root / "grok-execution" / "events.jsonl"
        receipt_path = self.run_root / "grok-execution" / "grok-runtime-receipt.json"
        gate_path.write_text(
            json.dumps(
                {
                    "schema_version": "0.1.0-draft",
                    "receipt_kind": "tool_availability_gate_receipt",
                    "valid": True,
                    "report_id": "TOOL-AVAIL-GROK-001",
                    "report_sha256": "a" * 64,
                    "available_count": 6,
                    "unavailable_count": 0,
                    "unprobed_count": 0,
                    "degraded_count": 0,
                    "context_block_sha256": "b" * 64,
                    "decisions": {
                        "gate_decision": "allow",
                        "context_injected": True,
                        "model_must_not_guess": True,
                    },
                }
            ),
            encoding="utf-8",
        )
        adapter_path.write_text(
            json.dumps(
                {
                    "schema_version": "0.1.0",
                    "receipt_kind": "grok_runtime_adapter_receipt",
                    "valid": True,
                    "containment": {
                        "no_residue_required": True,
                        "no_residue_observed": True,
                        "root_process_exited": True,
                        "external_cleanup_required": False,
                        "residue_scan_scope": "job_object_contained",
                        "job_object_created": True,
                        "job_object_assigned": True,
                        "containment_provider": "adapter_job_object",
                        "containment_available": True,
                    },
                }
            ),
            encoding="utf-8",
        )
        mock_acp.return_value = {
            "valid": True,
            "artifacts": {
                "receipt_path": str(receipt_path.resolve()),
                "events_path": str(events_path.resolve()),
            },
            "containment": {
                "job_object_assigned": True,
                "containment_provider": "adapter_job_object",
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "run",
                "--runtime", "grok",
                "--ask", "review the project",
                "--run-root", str(self.run_root),
                "--grok-tool-availability-gate", str(gate_path),
                "--grok-adapter-receipt", str(adapter_path),
                "--grok-execute",
                "--grok-fake-provider",
                "--json",
            ],
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(receipt["decision"], "allow")
        self.assertTrue(receipt["checks"]["prompt_tool_execution_attempted"])
        self.assertFalse(receipt["checks"]["canonical_default_unchanged"])
        self.assertEqual(receipt["execution"]["events_path"], str(events_path.resolve()))
        request = mock_acp.call_args.args[0]
        self.assertEqual(request.run_root, self.run_root / "grok-execution")
        self.assertEqual(request.workspace_path, self.run_root / "grok-execution" / "workspace")
        self.assertTrue(request.fake_provider)

    @patch("assurance.grok_runtime_adapter.run_grok_acp_once")
    @patch("assurance.grok_runtime_adapter.run_grok_version_smoke")
    def test_run_runtime_grok_alpha_auto_evidence_executes_acp_smoke(
        self, mock_version_smoke, mock_acp
    ) -> None:
        evidence_receipt_path = (
            self.run_root / "evidence" / "adapter-containment" / "grok-runtime-receipt.json"
        )
        evidence_events_path = (
            self.run_root / "evidence" / "adapter-containment" / "events.jsonl"
        )
        execution_receipt_path = self.run_root / "grok-execution" / "grok-runtime-receipt.json"
        execution_events_path = self.run_root / "grok-execution" / "events.jsonl"
        mock_version_smoke.return_value = {
            "valid": True,
            "artifacts": {
                "receipt_path": str(evidence_receipt_path.resolve()),
                "events_path": str(evidence_events_path.resolve()),
            },
            "containment": {
                "job_object_assigned": True,
                "containment_provider": "adapter_job_object",
            },
        }
        mock_acp.return_value = {
            "valid": True,
            "artifacts": {
                "receipt_path": str(execution_receipt_path.resolve()),
                "events_path": str(execution_events_path.resolve()),
            },
            "containment": {
                "job_object_assigned": True,
                "containment_provider": "adapter_job_object",
            },
        }

        exit_code, receipt = self._capture_json(
            [
                "run",
                "--runtime", "grok",
                "--ask", "alpha execute",
                "--run-root", str(self.run_root),
                "--grok-execute",
                "--grok-alpha-auto-evidence",
                "--json",
            ],
        )

        self.assertEqual(exit_code, 0)
        self.assertEqual(receipt["decision"], "allow")
        self.assertEqual(receipt["blocking_reasons"], [])
        self.assertTrue(receipt["checks"]["tool_availability_gate_allows"])
        self.assertTrue(receipt["checks"]["adapter_containment_provided"])
        self.assertTrue(receipt["checks"]["prompt_tool_execution_attempted"])
        self.assertTrue((self.run_root / "evidence" / "alpha-tool-availability-gate.json").is_file())
        mock_version_smoke.assert_called_once()
        request = mock_acp.call_args.args[0]
        self.assertEqual(request.mode, "acp_smoke")
        self.assertEqual(request.run_root, self.run_root / "grok-execution")
        self.assertTrue(request.fake_provider)

    def test_run_runtime_grok_alpha_auto_evidence_requires_execute(self) -> None:
        error_output = StringIO()
        with redirect_stderr(error_output):
            exit_code = gsa_main(
                [
                    "run",
                    "--runtime", "grok",
                    "--ask", "alpha execute",
                    "--run-root", str(self.run_root),
                    "--grok-alpha-auto-evidence",
                    "--json",
                ],
            )

        self.assertEqual(exit_code, 2)
        error = json.loads(error_output.getvalue())
        self.assertIn("requires --grok-execute", error["error"])
