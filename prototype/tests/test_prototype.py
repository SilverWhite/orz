from __future__ import annotations

from contextlib import redirect_stderr, redirect_stdout
import ctypes
import io
import json
import os
from pathlib import Path
import re
import sys
import tempfile
import threading
import unittest
from unittest.mock import patch

from jsonschema import Draft202012Validator

from fep_agent_proto.errors import PrototypeError
from fep_agent_proto.cli import main as cli_main
from fep_agent_proto.action_kernel import (
    execute_local_process_action,
    verify_action_kernel,
)
from fep_agent_proto.deepseek_adapter import (
    classify_http_status,
    classify_sse_line,
    validate_deepseek_profile,
    validate_tool_history,
)
from fep_agent_proto.deepseek_client import (
    build_deepseek_request,
    consume_deepseek_sse,
)
from fep_agent_proto.brokered_transport import BrokeredDeepSeekHttpsTransport
from fep_agent_proto.approval_ledger import (
    ApprovalLedger,
    ConsoleConfirmationIO,
    InteractivePermitBroker,
    InteractiveRealNetworkPermitBroker,
    verify_approval_ledger,
)
from fep_agent_proto.credentials import (
    CredentialLease,
    InMemoryCredentialProvider,
    UnavailableCredentialProvider,
    _copy_ascii_utf16le_credential_blob,
)
from fep_agent_proto.deepseek_external import inspect_deepseek_external_readiness
from fep_agent_proto.deepseek_https import (
    DeepSeekHttpsTransport,
    SanitizedDeepSeekTransportError,
    prepare_deepseek_https_request,
)
from fep_agent_proto.external_network import (
    DEEPSEEK_CHAT_ENDPOINT,
    OneShotNetworkPermit,
)
from fep_agent_proto.network_broker import (
    DenyAllPermitBroker,
    ScriptedPermitBroker,
    ScriptedPermitDecision,
    build_network_confirmation_summary,
    render_network_confirmation,
)
from fep_agent_proto.exporter import export_scenarios
from fep_agent_proto.io_utils import load_json, safe_relative_path, sha256_file
from fep_agent_proto.journal import create_journal_smoke, replay_journal
from fep_agent_proto.layout import DESIGN_ROOT, PROTOTYPE_ROOT, REGRESSION_ROOT, RUNTIME_ROOT
from fep_agent_proto.loopback_http import LoopbackHttpTransport
from fep_agent_proto.loopback_mock_server import LoopbackMockServer, MockHttpExchange
from fep_agent_proto.model_loop import (
    create_default_scripted_transport,
    execute_deepseek_mock_loop,
    verify_model_loop,
)
from fep_agent_proto.model_transport import (
    ScriptedTransport,
    TransportCancelled,
    TransportAttemptContext,
    TransportControl,
    TransportResponse,
    TransportTimeout,
)
from fep_agent_proto.private_transcript import (
    load_private_transcript,
    save_private_transcript,
)
from fep_agent_proto.real_development_probe import (
    _scan_artifact_candidates,
    create_real_development_plan,
    execute_real_development_probe,
)
from fep_agent_proto.scanner import scan_bundle
from fep_agent_proto.schema import validate_instance
from fep_agent_proto.session_validator import validate_session_record
from fep_agent_proto.windows_process import run_windows_process
from fep_agent_proto.windows_process_security import configure_secret_process_security


CORPUS = REGRESSION_ROOT / "cases-v0.1.yaml"


def _fake_process_security() -> dict[str, object]:
    return {
        "windows_wer_noheap_verified": True,
        "python_faulthandler_disabled": True,
        "scope": "current-short-lived-cli-process",
        "limitations": ["deterministic unit-test fixture"],
    }


class PrototypeRegressionTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(dir=PROTOTYPE_ROOT)
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def _export(self, name: str, *, policy_mode: str = "development") -> tuple[Path, dict]:
        root = self.root / name
        manifest = export_scenarios(
            corpus_path=CORPUS,
            output_dir=root,
            case_ids=["FEP-SYN-006"],
            policy_mode=policy_mode,
            token_seed="unit-test-seed",
        )
        return root, manifest

    def test_development_export_strips_oracle_and_copies_only_visible_fixture(self) -> None:
        root, manifest = self._export("development")
        self.assertEqual(manifest["status"], "ready")
        self.assertEqual(manifest["leak_scan"]["verdict"], "warn")
        token = manifest["case_tokens"][0]["token"]
        scenario_path = root / "scenario_bundle" / f"cases/{token}.json"
        scenario = load_json(scenario_path)
        self.assertEqual(
            set(scenario),
            {"schema_version", "scenario_token", "mode", "task", "visible_facts", "fixtures"},
        )
        self.assertNotIn("FEP-SYN-006", scenario_path.read_text(encoding="utf-8"))
        self.assertEqual(len(scenario["fixtures"]), 1)
        fixture = scenario["fixtures"][0]
        self.assertEqual(
            sha256_file(root / "scenario_bundle" / fixture["path"]), fixture["sha256"]
        )
        identity = load_json(root / "reviewer/identity-map.json")
        self.assertEqual(identity["cases"][0]["case_id"], "FEP-SYN-006")

    def test_challenge_export_is_deferred_without_semantic_review(self) -> None:
        root, manifest = self._export("challenge", policy_mode="challenge")
        self.assertEqual(manifest["status"], "blocked")
        report = load_json(root / "reviewer/leak-scan-report.json")
        self.assertEqual(report["verdict"], "warn")
        self.assertEqual(report["gate_action"], "defer")

    def test_reviewer_only_historical_fixture_is_not_exported(self) -> None:
        root = self.root / "historical"
        manifest = export_scenarios(
            corpus_path=CORPUS,
            output_dir=root,
            case_ids=["FEP-REG-009"],
            policy_mode="development",
            token_seed="unit-test-seed",
        )
        token = manifest["case_tokens"][0]["token"]
        scenario = load_json(root / "scenario_bundle" / f"cases/{token}.json")
        self.assertEqual(scenario["fixtures"], [])
        self.assertFalse((root / "scenario_bundle/fixtures").exists())
        self.assertNotIn("FEP-REG-009", json.dumps(scenario))

    def test_export_refuses_overwrite_and_unsafe_paths(self) -> None:
        root, _ = self._export("no-overwrite")
        with self.assertRaises(PrototypeError):
            export_scenarios(
                corpus_path=CORPUS,
                output_dir=root,
                case_ids=["FEP-SYN-006"],
                policy_mode="development",
                token_seed="unit-test-seed",
            )
        for unsafe in ("../oracle.json", "/absolute.json", "C:/drive.json", "a\\b.json"):
            with self.subTest(unsafe=unsafe), self.assertRaises(PrototypeError):
                safe_relative_path(unsafe)

    def test_scanner_fails_after_internal_id_tamper(self) -> None:
        root, manifest = self._export("tampered-bundle")
        token = manifest["case_tokens"][0]["token"]
        scenario_path = root / "scenario_bundle" / f"cases/{token}.json"
        scenario = load_json(scenario_path)
        scenario["visible_facts"].append("Internal reference FEP-REG-009")
        scenario_path.write_text(json.dumps(scenario), encoding="utf-8")
        report = scan_bundle(
            root / "scenario_bundle",
            export_id=manifest["export_id"],
            policy_mode="development",
            expected_bundle_sha256=manifest["bundle_sha256"],
        )
        self.assertEqual(report["verdict"], "fail")
        self.assertEqual(report["gate_action"], "block")
        self.assertTrue(
            any("LEAK-ID-INTERNAL-001" in item["summary"] for item in report["findings"])
        )

    def test_journal_replay_detects_payload_tampering(self) -> None:
        export_root, _ = self._export("journal-export")
        run_root = self.root / "journal-run"
        report = create_journal_smoke(export_root=export_root, output_dir=run_root)
        self.assertTrue(report["valid"])
        journal_path = run_root / "events.jsonl"
        lines = journal_path.read_text(encoding="utf-8").splitlines()
        first = json.loads(lines[0])
        first["payload"]["model_invoked"] = True
        lines[0] = json.dumps(first, sort_keys=True, separators=(",", ":"))
        journal_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
        tampered = replay_journal(
            run_manifest_path=run_root / "run-manifest.json",
            journal_path=journal_path,
        )
        self.assertFalse(tampered["valid"])
        self.assertTrue(any("digest mismatch" in error for error in tampered["errors"]))

    def test_schemas_and_runtime_examples_validate(self) -> None:
        schema_paths = sorted(DESIGN_ROOT.rglob("*.schema.json"))
        self.assertGreaterEqual(len(schema_paths), 12)
        for schema_path in schema_paths:
            with self.subTest(schema=schema_path.name):
                Draft202012Validator.check_schema(load_json(schema_path))

        examples = {
            "example-leak-scan-report.json": "leak-scan-report-v0.1.schema.json",
            "example-run-event.json": "run-event-v0.1.schema.json",
            "example-run-manifest.json": "run-manifest-v0.1.schema.json",
            "example-scenario-export-manifest.json": "scenario-export-manifest-v0.1.schema.json",
        }
        for example_name, schema_name in examples.items():
            with self.subTest(example=example_name):
                validate_instance(
                    load_json(RUNTIME_ROOT / "examples" / example_name),
                    RUNTIME_ROOT / schema_name,
                    label=example_name,
                )

    def _session_record(self) -> dict:
        return {
            "protocol_version": "0.1.0-draft",
            "session": {
                "session_id": "SESSION-001",
                "workspace_root": "D:/workspace",
                "created_at": "2026-07-21T00:00:00Z",
                "updated_at": "2026-07-21T00:01:00Z",
                "task_state": "reviewing",
                "mode": "guarded",
                "agent_profile_id": "AGENT-001",
            },
            "task_contract": {
                "task_id": "TASK-001",
                "intent": "validate_session_semantics",
                "mode": "implementation",
                "must": ["validate references"],
                "must_not": ["decide scientific validity"],
                "source_of_truth": ["SRC-001"],
                "acceptance": ["terminal event is unique"],
                "risk_class": "ordinary",
            },
            "reasoning_precommitment": None,
            "sources": [
                {
                    "source_id": "SRC-001",
                    "kind": "workspace_file",
                    "path_or_uri": "protocol/PROTOCOL_DRAFT_v0.1.md",
                    "content_hash": None,
                    "accessed_at": "2026-07-21T00:00:00Z",
                    "usage": "read",
                    "epistemic_role": "source_grounded",
                }
            ],
            "actions": [
                {
                    "action_id": "ACT-001",
                    "action_type": "read_only_probe",
                    "requested_by": "AGENT-001",
                    "state": "succeeded",
                    "arguments": {},
                    "risk_class": "read_only",
                    "expected_outputs": [],
                    "required_gate_ids": [],
                    "idempotency_key": None,
                    "events": [
                        {
                            "event_id": "EVT-001",
                            "sequence": 0,
                            "timestamp": "2026-07-21T00:00:30Z",
                            "action_id": "ACT-001",
                            "event_kind": "terminal",
                            "event_type": "process.finished",
                            "payload": {},
                            "terminal_state": "succeeded",
                            "exit_code": 0,
                            "produced_artifact_ids": [],
                            "reason_codes": [],
                        }
                    ],
                }
            ],
            "artifacts": [],
            "evidence": [],
            "gate_decisions": [],
            "case_retrievals": [],
            "claims": [],
        }

    def test_session_validator_accepts_minimal_consistent_record(self) -> None:
        report = validate_session_record(self._session_record())
        self.assertTrue(report["valid"])
        self.assertEqual(report["violation_count"], 0)

    def test_session_validator_rejects_terminal_and_reference_mismatches(self) -> None:
        record = self._session_record()
        record["actions"][0]["events"].insert(
            0,
            {
                "event_id": "EVT-000",
                "sequence": 1,
                "timestamp": "2026-07-21T00:00:15Z",
                "action_id": "ACT-001",
                "event_kind": "progress",
                "event_type": "process.started",
                "payload": {},
                "terminal_state": None,
                "exit_code": None,
                "produced_artifact_ids": [],
                "reason_codes": [],
            },
        )
        record["actions"][0]["events"][1]["terminal_state"] = "failed"
        record["task_contract"]["source_of_truth"] = ["SRC-MISSING"]
        report = validate_session_record(record)
        self.assertFalse(report["valid"])
        codes = {item["code"] for item in report["violations"]}
        self.assertIn("PROC-TERMINAL-001", codes)
        self.assertIn("PROC-SEQUENCE-001", codes)
        self.assertIn("ROUTE-SOURCE-001", codes)

    def _deepseek_profile(self) -> dict:
        return load_json(
            RUNTIME_ROOT / "examples/example-deepseek-adapter-profile.json"
        )

    def test_deepseek_profile_accepts_explicit_v4_configuration(self) -> None:
        report = validate_deepseek_profile(self._deepseek_profile())
        self.assertTrue(report["valid"])
        self.assertEqual(report["model_resolved"], "deepseek-v4-pro")

    def test_deepseek_profile_rejects_retired_alias(self) -> None:
        profile = self._deepseek_profile()
        profile["model"] = "deepseek-reasoner"
        report = validate_deepseek_profile(profile)
        self.assertFalse(report["valid"])
        self.assertEqual(report["issues"][0]["code"], "DS-MODEL-RETIRED-001")

    def test_deepseek_profile_rejects_sampling_that_thinking_would_ignore(self) -> None:
        profile = self._deepseek_profile()
        profile["sampling"] = {"temperature": 0}
        with self.assertRaises(PrototypeError):
            validate_deepseek_profile(profile)

    def test_deepseek_tool_history_requires_reasoning_continuity(self) -> None:
        messages = [
            {"role": "user", "content": "inspect the fixture"},
            {
                "role": "assistant",
                "content": "",
                "tool_calls": [
                    {
                        "id": "call_001",
                        "type": "function",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"fixture.json\"}"},
                    }
                ],
            },
            {"role": "tool", "tool_call_id": "call_001", "content": "{}"},
        ]
        missing = validate_tool_history(messages, thinking_enabled=True)
        self.assertFalse(missing["valid"])
        self.assertEqual(missing["issues"][0]["code"], "DS-REASONING-CONTINUITY-001")

        messages[1]["reasoning_content"] = "provider-private reasoning"
        complete = validate_tool_history(messages, thinking_enabled=True)
        self.assertTrue(complete["valid"])
        self.assertNotIn("provider-private reasoning", json.dumps(complete))

    def test_deepseek_stream_and_http_classification(self) -> None:
        self.assertEqual(classify_sse_line(": keep-alive")["kind"], "keep_alive")
        self.assertEqual(classify_sse_line("data: [DONE]")["kind"], "done")
        self.assertEqual(
            classify_sse_line('data: {"choices":[]}')["payload"], {"choices": []}
        )
        with self.assertRaises(PrototypeError):
            classify_sse_line("data: {\"value\": NaN}")
        self.assertTrue(classify_http_status(503)["retryable"])
        self.assertFalse(classify_http_status(422)["retryable"])

    def test_deepseek_request_builder_preserves_private_tool_reasoning_in_memory(self) -> None:
        profile = self._deepseek_profile()
        messages = [
            {"role": "user", "content": "run fixture"},
            {
                "role": "assistant",
                "content": "",
                "reasoning_content": "private-continuity-value",
                "tool_calls": [
                    {
                        "id": "call_001",
                        "type": "function",
                        "function": {
                            "name": "mock_echo",
                            "arguments": "{\"value\":42}",
                        },
                    }
                ],
            },
            {"role": "tool", "tool_call_id": "call_001", "content": "{}"},
        ]
        tools = [
            {
                "type": "function",
                "function": {
                    "name": "mock_echo",
                    "description": "fixture",
                    "parameters": {"type": "object"},
                },
            }
        ]

        request, metadata = build_deepseek_request(
            profile=profile,
            messages=messages,
            tools=tools,
        )

        self.assertEqual(
            request["messages"][1]["reasoning_content"], "private-continuity-value"
        )
        self.assertNotIn("temperature", request)
        self.assertTrue(metadata["reasoning_continuity_valid"])
        self.assertEqual(metadata["reasoning_message_count"], 1)
        self.assertNotIn("private-continuity-value", json.dumps(metadata))

    def test_deepseek_sse_assembler_rejects_missing_done(self) -> None:
        with self.assertRaises(PrototypeError):
            consume_deepseek_sse(
                [
                    'data: {"choices":[{"index":0,"delta":{"content":"x"},"finish_reason":"stop"}]}'
                ]
            )

    def test_deepseek_scripted_model_loop_journals_redacted_two_turn_flow(self) -> None:
        export_root, _ = self._export("model-loop-export")
        output_dir = self.root / "model-loop"
        transport = create_default_scripted_transport()
        result = execute_deepseek_mock_loop(
            export_root=export_root,
            output_dir=output_dir,
            profile=self._deepseek_profile(),
            transport=transport,
        )

        self.assertTrue(result["valid"])
        self.assertTrue(result["runtime_ready"])
        self.assertFalse(result["real_network"])
        self.assertEqual(result["request_count"], 2)
        self.assertEqual(result["http_attempt_count"], 2)
        self.assertEqual(result["retry_count"], 0)
        self.assertEqual(result["tool_call_count"], 1)
        self.assertTrue(result["reasoning_continuity_checked"])
        self.assertEqual(result["journal_replay"]["event_count"], 11)
        self.assertEqual(
            transport.sent_requests[1]["messages"][2]["reasoning_content"],
            "mock-private-plan",
        )
        self.assertTrue(verify_model_loop(output_dir=output_dir)["valid"])
        persisted = "\n".join(
            path.read_text(encoding="utf-8", errors="replace")
            for path in output_dir.iterdir()
            if path.is_file()
        )
        self.assertNotIn("mock-private-plan", persisted)
        self.assertNotIn("mock-private-final", persisted)
        self.assertNotIn("reasoning_content", persisted)

    def test_deepseek_scripted_model_loop_retries_frozen_503_once(self) -> None:
        base = create_default_scripted_transport()
        first = base.send({"fixture": "extract-first"})
        second = base.send({"fixture": "extract-second"})
        transport = ScriptedTransport(
            [
                TransportResponse(
                    status=503,
                    headers={"content-type": "application/json"},
                    lines=(),
                ),
                first,
                second,
            ]
        )
        export_root, _ = self._export("model-loop-retry-export")
        result = execute_deepseek_mock_loop(
            export_root=export_root,
            output_dir=self.root / "model-loop-retry",
            profile=self._deepseek_profile(),
            transport=transport,
        )

        self.assertEqual(result["http_attempt_count"], 3)
        self.assertEqual(result["retry_count"], 1)
        self.assertEqual(result["turns"][0]["attempts"][0]["status"], 503)
        self.assertEqual(
            transport.sent_requests[0], transport.sent_requests[1]
        )

    def test_model_loop_verifier_rejects_persisted_private_reasoning_field(self) -> None:
        export_root, _ = self._export("model-loop-leak-export")
        output_dir = self.root / "model-loop-leak"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                [
                    "deepseek-model-loop-smoke",
                    "--export-root",
                    str(export_root),
                    "--profile",
                    str(
                        RUNTIME_ROOT
                        / "examples/example-deepseek-adapter-profile.json"
                    ),
                    "--output-dir",
                    str(output_dir),
                ]
            )
        self.assertEqual(exit_code, 0)
        self.assertTrue(json.loads(stdout.getvalue())["valid"])
        (output_dir / "unexpected-private.json").write_text(
            json.dumps({"reasoning_content": "must-not-persist"}),
            encoding="utf-8",
        )

        verification = verify_model_loop(output_dir=output_dir)
        self.assertFalse(verification["valid"])
        self.assertTrue(
            any("provider-private" in error for error in verification["errors"])
        )

    def test_loopback_http_transport_rejects_nonliteral_or_external_endpoints(self) -> None:
        for endpoint in (
            "https://127.0.0.1:8443",
            "http://localhost:8080",
            "http://192.0.2.1:8080",
            "http://127.0.0.1",
            "http://127.0.0.1:8080/path",
        ):
            with self.subTest(endpoint=endpoint), self.assertRaises(PrototypeError):
                LoopbackHttpTransport(endpoint)

        transport = LoopbackHttpTransport("http://127.0.0.1:1")
        with self.assertRaises(PrototypeError):
            transport.send(
                {"body": "too-large"},
                TransportControl(
                    connect_seconds=0.1,
                    first_semantic_seconds=0.1,
                    total_seconds=0.2,
                    max_request_bytes=1,
                ),
            )

    def test_deepseek_loopback_http_cli_smoke_uses_no_external_endpoint(self) -> None:
        export_root, _ = self._export("loopback-model-export")
        output_dir = self.root / "loopback-model"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                [
                    "deepseek-loopback-http-smoke",
                    "--export-root",
                    str(export_root),
                    "--profile",
                    str(
                        RUNTIME_ROOT
                        / "examples/example-deepseek-adapter-profile.json"
                    ),
                    "--output-dir",
                    str(output_dir),
                ]
            )
        result = json.loads(stdout.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertTrue(result["valid"])
        self.assertEqual(result["transport"], "loopback-http-v0.1")
        self.assertFalse(result["real_network"])
        self.assertEqual(result["http_attempt_count"], 2)
        self.assertTrue(
            result["turns"][0]["attempts"][0]["transport_metadata"][
                "first_semantic_observed"
            ]
        )
        self.assertEqual(result["private_transcript_persisted"], os.name == "nt")
        self.assertTrue(verify_model_loop(output_dir=output_dir)["valid"])

    def test_loopback_http_transport_observes_cooperative_cancellation(self) -> None:
        exchange = MockHttpExchange(
            status=200,
            lines=tuple(": keep-alive" for _ in range(40)),
            line_delay_seconds=0.03,
        )
        with LoopbackMockServer([exchange]) as server:
            transport = LoopbackHttpTransport(server.base_url)
            control = TransportControl(
                connect_seconds=0.5,
                first_semantic_seconds=2,
                total_seconds=3,
            )
            timer = threading.Timer(0.12, control.cancel)
            timer.start()
            try:
                with self.assertRaises(TransportCancelled):
                    transport.send({"fixture": "cancel"}, control)
            finally:
                timer.cancel()
                timer.join(timeout=1)

    def test_loopback_http_transport_times_out_without_semantic_event(self) -> None:
        exchange = MockHttpExchange(
            status=200,
            lines=tuple(": keep-alive" for _ in range(20)),
            line_delay_seconds=0.03,
        )
        with LoopbackMockServer([exchange]) as server:
            transport = LoopbackHttpTransport(server.base_url)
            control = TransportControl(
                connect_seconds=0.1,
                first_semantic_seconds=0.12,
                total_seconds=1,
            )
            with self.assertRaises(TransportTimeout) as captured:
                transport.send({"fixture": "first-semantic-timeout"}, control)
        self.assertEqual(captured.exception.phase, "first_semantic")

    def test_external_network_permit_is_digest_bound_expiring_and_one_shot(self) -> None:
        prepared = prepare_deepseek_https_request({"model": "deepseek-v4-pro"})
        now = [100.0]
        permit = OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=prepared.body_sha256,
            ttl_seconds=5,
            clock=lambda: now[0],
        )
        with self.assertRaises(PrototypeError):
            permit.consume(
                provider="deepseek",
                endpoint=DEEPSEEK_CHAT_ENDPOINT,
                request_sha256="0" * 64,
            )
        receipt = permit.consume(
            provider="deepseek",
            endpoint=DEEPSEEK_CHAT_ENDPOINT,
            request_sha256=prepared.body_sha256,
        )
        self.assertEqual(receipt.request_sha256, prepared.body_sha256)
        self.assertTrue(permit.consumed)
        with self.assertRaises(PrototypeError):
            permit.consume(
                provider="deepseek",
                endpoint=DEEPSEEK_CHAT_ENDPOINT,
                request_sha256=prepared.body_sha256,
            )

        expired = OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=prepared.body_sha256,
            ttl_seconds=1,
            clock=lambda: now[0],
        )
        now[0] = 102.0
        with self.assertRaises(PrototypeError):
            expired.consume(
                provider="deepseek",
                endpoint=DEEPSEEK_CHAT_ENDPOINT,
                request_sha256=prepared.body_sha256,
            )

    def test_transport_attempt_context_requires_retry_provenance(self) -> None:
        TransportAttemptContext(turn=1, attempt=1)
        TransportAttemptContext(turn=2, attempt=3, previous_status=503)
        with self.assertRaises(PrototypeError):
            TransportAttemptContext(turn=1, attempt=1, previous_status=503)
        with self.assertRaises(PrototypeError):
            TransportAttemptContext(turn=1, attempt=2)

    def test_network_confirmation_summary_is_redacted_and_deny_default(self) -> None:
        private_text = "private-reasoning-must-not-appear"
        prompt_text = "prompt-body-must-not-appear"
        request = {
            "model": "deepseek-v4-pro",
            "messages": [
                {"role": "user", "content": prompt_text},
                {
                    "role": "assistant",
                    "content": "",
                    "reasoning_content": private_text,
                },
            ],
            "tools": [],
            "stream": True,
            "response_format": {"type": "json_object"},
            "max_tokens": 128,
        }
        summary = build_network_confirmation_summary(
            request=request,
            control=TransportControl(
                connect_seconds=1,
                first_semantic_seconds=2,
                total_seconds=3,
            ),
            attempt_context=TransportAttemptContext(turn=2, attempt=1),
        )
        rendered = render_network_confirmation(summary)
        encoded = json.dumps(summary)
        self.assertNotIn(prompt_text, encoded)
        self.assertNotIn(private_text, encoded)
        self.assertNotIn(prompt_text, rendered)
        self.assertNotIn(private_text, rendered)
        self.assertTrue(summary["disclosure"]["sends_message_content"])
        self.assertTrue(
            summary["disclosure"]["sends_provider_private_reasoning"]
        )
        with self.assertRaises(PrototypeError):
            DenyAllPermitBroker().authorize(summary)

        scripted = ScriptedPermitBroker(
            [ScriptedPermitDecision(allow=False, expected_turn=2, expected_attempt=1)]
        )
        with self.assertRaises(PrototypeError):
            scripted.authorize(summary)
        self.assertEqual(len(scripted.summaries), 1)

        with self.assertRaises(PrototypeError):
            BrokeredDeepSeekHttpsTransport(
                permit_broker=ScriptedPermitBroker(
                    [
                        ScriptedPermitDecision(
                            allow=True, expected_turn=1, expected_attempt=1
                        )
                    ]
                ),
                credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
            )

    def test_interactive_broker_records_denial_and_ledger_tampering(self) -> None:
        request = {
            "model": "deepseek-v4-pro",
            "messages": [{"role": "user", "content": "not-persisted"}],
            "tools": [],
            "stream": True,
        }
        summary = build_network_confirmation_summary(
            request=request,
            control=TransportControl(
                connect_seconds=1,
                first_semantic_seconds=2,
                total_seconds=3,
            ),
            attempt_context=TransportAttemptContext(turn=1, attempt=1),
        )
        path = self.root / "approval-deny.jsonl"
        output = io.StringIO()
        broker = InteractivePermitBroker(
            ledger=ApprovalLedger(path),
            confirmation_io=ConsoleConfirmationIO(
                input_func=lambda: "DENY",
                output=output,
            ),
        )
        with self.assertRaises(PrototypeError):
            broker.authorize(summary)
        report = verify_approval_ledger(path)
        self.assertTrue(report["valid"])
        self.assertEqual(report["allow_count"], 0)
        self.assertEqual(report["deny_count"], 1)
        self.assertNotIn("not-persisted", path.read_text(encoding="utf-8"))
        self.assertIn("Type ALLOW-", output.getvalue())

        original = path.read_text(encoding="utf-8")
        path.write_text(
            original.replace('"decision":"deny"', '"decision":"allow"'),
            encoding="utf-8",
        )
        self.assertFalse(verify_approval_ledger(path)["valid"])

    def test_interactive_broker_is_restricted_to_in_process_fake_provider(self) -> None:
        broker = InteractivePermitBroker(
            ledger=ApprovalLedger(self.root / "never-written.jsonl"),
            confirmation_io=ConsoleConfirmationIO(
                input_func=lambda: "DENY",
                output=io.StringIO(),
            ),
        )
        with self.assertRaises(PrototypeError):
            BrokeredDeepSeekHttpsTransport(
                permit_broker=broker,
                credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
            )

    def test_credential_providers_fail_closed_and_redact_secret_representations(self) -> None:
        with self.assertRaises(PrototypeError):
            UnavailableCredentialProvider().acquire()
        with self.assertRaises(PrototypeError):
            InMemoryCredentialProvider(b"bad\nsecret")

        provider = InMemoryCredentialProvider(b"sk-test-not-real")
        self.assertNotIn("sk-test-not-real", repr(provider))
        lease = provider.acquire()
        self.assertNotIn("sk-test-not-real", repr(lease))
        self.assertEqual(lease.authorization_value(), "Bearer sk-test-not-real")
        lease.close()
        with self.assertRaises(PrototypeError):
            lease.authorization_value()

        owned = bytearray(b"sk-owned-buffer-test")
        owned_lease = CredentialLease._take_ownership(
            owned,
            source_id="owned-buffer-test",
        )
        self.assertEqual(owned_lease.authorization_value(), "Bearer sk-owned-buffer-test")
        owned_lease.close()
        self.assertEqual(owned, bytearray(len(owned)))

        encoded = "sk-direct-pointer-test\x00".encode("utf-16-le")
        native_blob = (ctypes.c_ubyte * len(encoded))(*encoded)
        copied = _copy_ascii_utf16le_credential_blob(native_blob, len(encoded))
        self.assertEqual(copied, bytearray(b"sk-direct-pointer-test"))

    def test_deepseek_https_transport_uses_fake_connection_and_records_no_key(self) -> None:
        class FakeSocket:
            def __init__(self) -> None:
                self.timeout: float | None = None

            def settimeout(self, value: float) -> None:
                self.timeout = value

        class FakeResponse:
            status = 200

            def __init__(self) -> None:
                self._lines = iter(
                    (
                        b": keep-alive\r\n",
                        b'data: {"choices":[],"usage":{"total_tokens":1}}\r\n',
                        b"data: [DONE]\r\n",
                        b"",
                    )
                )
                self.closed = False

            def getheaders(self) -> list[tuple[str, str]]:
                return [("Content-Type", "text/event-stream")]

            def readline(self) -> bytes:
                return next(self._lines)

            def close(self) -> None:
                self.closed = True

        class FakeConnection:
            def __init__(self) -> None:
                self.sock = FakeSocket()
                self.connected = False
                self.closed = False
                self.request_record: dict[str, object] | None = None
                self.response = FakeResponse()

            def connect(self) -> None:
                self.connected = True

            def request(
                self,
                method: str,
                path: str,
                *,
                body: bytes,
                headers: dict[str, str],
            ) -> None:
                self.request_record = {
                    "method": method,
                    "path": path,
                    "body": body,
                    "headers": dict(headers),
                }

            def getresponse(self) -> FakeResponse:
                return self.response

            def close(self) -> None:
                self.closed = True

        request = {"model": "deepseek-v4-pro", "stream": True, "messages": []}
        prepared = prepare_deepseek_https_request(request)
        permit = OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=prepared.body_sha256
        )
        fake = FakeConnection()
        factory_record: dict[str, object] = {}

        def factory(host: str, port: int, timeout: float, context: object) -> FakeConnection:
            factory_record.update(
                {"host": host, "port": port, "timeout": timeout, "context": context}
            )
            return fake

        transport = DeepSeekHttpsTransport(
            permit=permit,
            credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
            connection_factory=factory,
        )
        response = transport.send(
            request,
            TransportControl(
                connect_seconds=1,
                first_semantic_seconds=1,
                total_seconds=2,
            ),
        )

        self.assertEqual(factory_record["host"], "api.deepseek.com")
        self.assertEqual(factory_record["port"], 443)
        self.assertEqual(fake.request_record["method"], "POST")
        self.assertEqual(fake.request_record["path"], "/chat/completions")
        self.assertEqual(
            fake.request_record["headers"]["Authorization"],
            "Bearer sk-test-not-real",
        )
        self.assertEqual(response.status, 200)
        self.assertTrue(response.metadata["external_network"])
        self.assertEqual(response.metadata["connection_factory"], "injected")
        self.assertTrue(response.metadata["endpoint_pinned"])
        self.assertFalse(response.metadata["proxy_environment_used"])
        self.assertFalse(response.metadata["redirects_followed"])
        self.assertFalse(response.metadata["http_debug_output"])
        self.assertFalse(response.metadata["authorization_recorded"])
        self.assertNotIn("sk-test-not-real", json.dumps(response.metadata))
        self.assertTrue(fake.response.closed)
        self.assertTrue(fake.closed)
        with self.assertRaises(PrototypeError):
            transport.send(request)

    def test_deepseek_https_transport_does_not_construct_connection_without_credential(self) -> None:
        request = {"model": "deepseek-v4-pro", "stream": True, "messages": []}
        prepared = prepare_deepseek_https_request(request)
        permit = OneShotNetworkPermit.issue_for_deepseek_chat(
            request_sha256=prepared.body_sha256
        )
        factory_called = [False]

        def forbidden_factory(
            host: str, port: int, timeout: float, context: object
        ) -> object:
            factory_called[0] = True
            raise AssertionError("connection factory must not be reached")

        transport = DeepSeekHttpsTransport(
            permit=permit,
            credential_provider=UnavailableCredentialProvider(),
            connection_factory=forbidden_factory,
        )
        with self.assertRaises(PrototypeError):
            transport.send(request)
        self.assertFalse(factory_called[0])
        self.assertTrue(permit.consumed)

    def test_deepseek_https_transport_does_not_reuse_timed_out_buffered_reader(
        self,
    ) -> None:
        import socket

        class FakeSocket:
            def __init__(self) -> None:
                self.timeouts: list[float] = []

            def settimeout(self, value: float) -> None:
                self.timeouts.append(value)

        class DelayedResponse:
            status = 200

            def __init__(self) -> None:
                self.read_count = 0

            def getheaders(self) -> list[tuple[str, str]]:
                return [("Content-Type", "text/event-stream")]

            def readline(self) -> bytes:
                self.read_count += 1
                if self.read_count == 1:
                    raise socket.timeout()
                raise OSError("timed-out buffered reader must not be reused")

            def close(self) -> None:
                pass

        class DelayedConnection:
            def __init__(self) -> None:
                self.sock = FakeSocket()
                self.response = DelayedResponse()

            def connect(self) -> None:
                pass

            def request(self, method: str, path: str, **kwargs: object) -> None:
                pass

            def getresponse(self) -> DelayedResponse:
                return self.response

            def close(self) -> None:
                pass

        request = {"model": "deepseek-v4-pro", "stream": True, "messages": []}
        prepared = prepare_deepseek_https_request(request)
        transport = DeepSeekHttpsTransport(
            permit=OneShotNetworkPermit.issue_for_deepseek_chat(
                request_sha256=prepared.body_sha256
            ),
            credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
            connection_factory=lambda host, port, timeout, context: DelayedConnection(),
        )
        with self.assertRaises(TransportTimeout) as captured:
            transport.send(
                request,
                TransportControl(
                    connect_seconds=1,
                    first_semantic_seconds=1,
                    total_seconds=2,
                ),
            )
        self.assertEqual(captured.exception.phase, "first_semantic")

    def test_deepseek_https_transport_sanitizes_connection_errors(self) -> None:
        class FailingConnection:
            sock = None

            def connect(self) -> None:
                pass

            def request(self, method: str, path: str, **kwargs: object) -> None:
                raise OSError("unexpected diagnostic containing sk-test-not-real")

            def close(self) -> None:
                pass

        request = {"model": "deepseek-v4-pro", "stream": True, "messages": []}
        prepared = prepare_deepseek_https_request(request)
        transport = DeepSeekHttpsTransport(
            permit=OneShotNetworkPermit.issue_for_deepseek_chat(
                request_sha256=prepared.body_sha256
            ),
            credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
            connection_factory=lambda host, port, timeout, context: FailingConnection(),
        )
        with self.assertRaises(PrototypeError) as captured:
            transport.send(request)
        self.assertIsInstance(captured.exception, SanitizedDeepSeekTransportError)
        self.assertEqual(captured.exception.stage, "request")
        self.assertEqual(captured.exception.error_type, "OSError")
        self.assertNotIn("sk-test-not-real", str(captured.exception))
        self.assertIn("failed at request (OSError)", str(captured.exception))
        self.assertIsNone(captured.exception.__cause__)

    def test_real_development_probe_is_one_shot_and_persists_only_redacted_result(
        self,
    ) -> None:
        class FakeSocket:
            def settimeout(self, value: float) -> None:
                self.timeout = value

        class FakeResponse:
            status = 200

            def __init__(self) -> None:
                self._lines = iter(
                    (
                        b'data: {"model":"deepseek-v4-pro","choices":[{"index":0,"delta":{"role":"assistant","content":"LIF_REAL_DEEPSEEK_OK"},"finish_reason":"stop"}]}\r\n',
                        b'data: {"model":"deepseek-v4-pro","choices":[],"usage":{"prompt_tokens":12,"completion_tokens":5,"total_tokens":17}}\r\n',
                        b"data: [DONE]\r\n",
                        b"",
                    )
                )

            def getheaders(self) -> list[tuple[str, str]]:
                return [("Content-Type", "text/event-stream")]

            def readline(self) -> bytes:
                return next(self._lines)

            def close(self) -> None:
                pass

        class FakeConnection:
            def __init__(self) -> None:
                self.sock = FakeSocket()
                self.request_count = 0

            def connect(self) -> None:
                pass

            def request(self, method: str, path: str, **kwargs: object) -> None:
                self.request_count += 1

            def getresponse(self) -> FakeResponse:
                return FakeResponse()

            def close(self) -> None:
                pass

        class PinnedTestCredentialProvider:
            source_id = "windows-credential-manager-current-user"

            def acquire(self) -> CredentialLease:
                return CredentialLease(
                    b"sk-test-not-real",
                    source_id=self.source_id,
                )

        output_dir = self.root / "real-probe"
        plan = create_real_development_plan(output_dir=output_dir)
        token = "ALLOW-" + plan["confirmation_summary_sha256"][:12].upper()
        confirmation_output = io.StringIO()
        connection = FakeConnection()
        result = execute_real_development_probe(
            plan_path=output_dir / "plan.json",
            confirmation_io=ConsoleConfirmationIO(
                input_func=lambda: token,
                output=confirmation_output,
                attempt_label="real-network",
            ),
            credential_provider=PinnedTestCredentialProvider(),
            connection_factory=lambda host, port, timeout, context: connection,
            process_security_configurator=_fake_process_security,
        )

        self.assertTrue(result["valid"])
        self.assertEqual(connection.request_count, 1)
        self.assertEqual(result["request_count"], 1)
        self.assertEqual(result["retry_count"], 0)
        self.assertEqual(result["resolved_model"], "deepseek-v4-pro")
        self.assertTrue(result["marker_matched"])
        self.assertTrue(result["process_security"]["windows_wer_noheap_verified"])
        self.assertEqual(result["artifact_leak_scan"]["hit_count"], 0)
        self.assertFalse(result["transport"]["proxy_environment_used"])
        self.assertFalse(result["transport"]["redirects_followed"])
        self.assertFalse(result["transport"]["http_debug_output"])
        self.assertIn("REAL DEEPSEEK DEVELOPMENT REQUEST", confirmation_output.getvalue())
        persisted = (output_dir / "result.json").read_text(encoding="utf-8")
        ledger = (output_dir / "network-approvals.jsonl").read_text(encoding="utf-8")
        self.assertNotIn("LIF_REAL_DEEPSEEK_OK", persisted)
        self.assertNotIn("sk-test-not-real", persisted)
        self.assertNotIn(token, persisted)
        self.assertNotIn(token, ledger)
        self.assertTrue(verify_approval_ledger(output_dir / "network-approvals.jsonl")["valid"])

    def test_real_development_probe_writes_redacted_failure_artifact(self) -> None:
        class FailingConnection:
            sock = None

            def connect(self) -> None:
                pass

            def request(self, method: str, path: str, **kwargs: object) -> None:
                raise OSError("must-not-persist sk-test-not-real")

            def close(self) -> None:
                pass

        class PinnedTestCredentialProvider:
            source_id = "windows-credential-manager-current-user"

            def acquire(self) -> CredentialLease:
                return CredentialLease(
                    b"sk-test-not-real",
                    source_id=self.source_id,
                )

        output_dir = self.root / "real-probe-failure"
        plan = create_real_development_plan(output_dir=output_dir)
        token = "ALLOW-" + plan["confirmation_summary_sha256"][:12].upper()
        failure = execute_real_development_probe(
            plan_path=output_dir / "plan.json",
            confirmation_io=ConsoleConfirmationIO(
                input_func=lambda: token,
                output=io.StringIO(),
                attempt_label="real-network",
            ),
            credential_provider=PinnedTestCredentialProvider(),
            connection_factory=lambda host, port, timeout, context: FailingConnection(),
            process_security_configurator=_fake_process_security,
        )

        self.assertFalse(failure["valid"])
        self.assertEqual(failure["authorized_attempt_count"], 1)
        self.assertEqual(failure["retry_count"], 0)
        self.assertEqual(failure["error"]["category"], "sanitized_transport_error")
        self.assertEqual(failure["error"]["stage"], "request")
        self.assertFalse(failure["error"]["raw_exception_recorded"])
        self.assertEqual(failure["artifact_leak_scan"]["hit_count"], 0)
        persisted = (output_dir / "failure.json").read_text(encoding="utf-8")
        self.assertNotIn("must-not-persist", persisted)
        self.assertNotIn("sk-test-not-real", persisted)

    def test_real_development_artifact_scan_rejects_common_secret_shape(self) -> None:
        output_dir = self.root / "real-probe-leak"
        output_dir.mkdir()
        (output_dir / "existing.txt").write_bytes(
            b"sk-deliberately-long-test-secret-1234567890"
        )
        with self.assertRaises(PrototypeError) as captured:
            _scan_artifact_candidates(
                output_dir=output_dir,
                pending_name="result.json",
                pending_bytes=b'{"valid":true}\n',
            )
        self.assertIn("deepseek_key_shape", str(captured.exception))
        self.assertNotIn("deliberately-long-test-secret", str(captured.exception))

    @unittest.skipUnless(os.name == "nt", "Windows WER control")
    def test_windows_secret_process_security_disables_wer_heap_collection(self) -> None:
        result = configure_secret_process_security()
        self.assertTrue(result["windows_wer_noheap_verified"])
        self.assertTrue(result["python_faulthandler_disabled"])

    def test_real_network_broker_rejects_in_process_fake_provider(self) -> None:
        broker = InteractiveRealNetworkPermitBroker(
            ledger=ApprovalLedger(self.root / "real-never-written.jsonl"),
            confirmation_io=ConsoleConfirmationIO(
                input_func=lambda: "DENY",
                output=io.StringIO(),
            ),
        )
        factory = type(
            "MarkedFakeFactory",
            (),
            {"is_in_process_fake_provider": True, "__call__": lambda self, *args: None},
        )()
        with self.assertRaises(PrototypeError):
            BrokeredDeepSeekHttpsTransport(
                permit_broker=broker,
                credential_provider=InMemoryCredentialProvider(b"sk-test-not-real"),
                connection_factory=factory,
            )

    def test_deepseek_external_readiness_cli_is_offline_and_blocked(self) -> None:
        profile_path = RUNTIME_ROOT / "examples/example-deepseek-adapter-profile.json"
        report = inspect_deepseek_external_readiness(load_json(profile_path))
        self.assertTrue(report["valid"])
        self.assertFalse(report["ready_for_network"])
        self.assertFalse(report["credential"]["accessed"])
        self.assertFalse(report["network_request_performed"])

        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                ["deepseek-external-readiness", "--profile", str(profile_path)]
            )
        self.assertEqual(exit_code, 2)
        self.assertEqual(json.loads(stdout.getvalue()), report)

    def test_deepseek_brokered_fake_https_cli_requires_three_per_attempt_permits(self) -> None:
        export_root, _ = self._export("brokered-fake-model-export")
        output_dir = self.root / "brokered-fake-model"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                [
                    "deepseek-brokered-fake-https-smoke",
                    "--export-root",
                    str(export_root),
                    "--profile",
                    str(
                        RUNTIME_ROOT
                        / "examples/example-deepseek-adapter-profile.json"
                    ),
                    "--output-dir",
                    str(output_dir),
                ]
            )
        result = json.loads(stdout.getvalue())
        self.assertEqual(exit_code, 0)
        self.assertTrue(result["valid"])
        self.assertEqual(result["transport"], "deepseek-brokered-fake-https-v0.1")
        self.assertFalse(result["real_network"])
        self.assertEqual(result["http_attempt_count"], 3)
        self.assertEqual(result["retry_count"], 1)
        attempts = [
            attempt
            for turn in result["turns"]
            for attempt in turn["attempts"]
        ]
        summaries = [
            attempt["transport_metadata"]["confirmation_summary"]
            for attempt in attempts
        ]
        self.assertEqual(
            [(item["turn"], item["attempt"]) for item in summaries],
            [(1, 1), (1, 2), (2, 1)],
        )
        self.assertEqual(summaries[1]["previous_status"], 503)
        self.assertTrue(summaries[2]["disclosure"]["sends_provider_private_reasoning"])
        self.assertTrue(verify_model_loop(output_dir=output_dir)["valid"])
        persisted = b"\n".join(
            path.read_bytes()
            for path in output_dir.iterdir()
            if path.is_file() and path.suffix != ".dpapi"
        )
        self.assertNotIn(b"sk-fake-provider-only", persisted)
        self.assertNotIn(b"mock-private-plan", persisted)
        self.assertNotIn(b"mock-private-final", persisted)

    def test_deepseek_interactive_fake_https_cli_hash_chains_three_approvals(self) -> None:
        export_root, _ = self._export("interactive-fake-model-export")
        output_dir = self.root / "interactive-fake-model"
        stdout = io.StringIO()
        stderr = io.StringIO()

        def answer_digest_challenge() -> str:
            tokens = re.findall(r"ALLOW-[A-F0-9]{12}", stderr.getvalue())
            if not tokens:
                raise AssertionError("interactive confirmation token was not rendered")
            return tokens[-1]

        with patch("builtins.input", side_effect=answer_digest_challenge):
            with redirect_stdout(stdout), redirect_stderr(stderr):
                exit_code = cli_main(
                    [
                        "deepseek-interactive-fake-https-smoke",
                        "--export-root",
                        str(export_root),
                        "--profile",
                        str(
                            RUNTIME_ROOT
                            / "examples/example-deepseek-adapter-profile.json"
                        ),
                        "--output-dir",
                        str(output_dir),
                    ]
                )
        result = json.loads(stdout.getvalue())
        self.assertEqual(exit_code, 0)
        self.assertTrue(result["valid"])
        self.assertFalse(result["real_network"])
        self.assertEqual(result["http_attempt_count"], 3)
        ledger_artifact = result["artifacts"]["network_approval_ledger"]
        self.assertIsNotNone(ledger_artifact)
        ledger_report = verify_approval_ledger(output_dir / ledger_artifact["path"])
        self.assertTrue(ledger_report["valid"])
        self.assertEqual(ledger_report["event_count"], 3)
        self.assertEqual(ledger_report["allow_count"], 3)
        self.assertEqual(ledger_report["deny_count"], 0)
        self.assertEqual(
            stderr.getvalue().count("FAKE PROVIDER DRY RUN — no network, no billing"),
            3,
        )
        self.assertEqual(
            stderr.getvalue().count("DeepSeek external-request approval:"), 3
        )
        self.assertTrue(verify_model_loop(output_dir=output_dir)["valid"])
        persisted = b"\n".join(
            path.read_bytes()
            for path in output_dir.iterdir()
            if path.is_file() and path.suffix != ".dpapi"
        )
        self.assertNotIn(b"sk-fake-provider-only", persisted)
        self.assertNotIn(b"mock-private-plan", persisted)

        result_path = output_dir / "model-loop-result.json"
        tampered = load_json(result_path)
        tampered["turns"][0]["attempts"][0]["transport_metadata"][
            "confirmation_summary"
        ]["message_count"] += 1
        result_path.write_text(
            json.dumps(tampered, ensure_ascii=False, sort_keys=True),
            encoding="utf-8",
        )
        tamper_report = verify_model_loop(output_dir=output_dir)
        self.assertFalse(tamper_report["valid"])
        self.assertTrue(
            any("confirmation summary" in error for error in tamper_report["errors"])
        )

    @unittest.skipUnless(os.name == "nt", "Windows DPAPI test")
    def test_windows_dpapi_private_transcript_roundtrip_and_tamper_detection(self) -> None:
        path = self.root / "private-transcript.dpapi"
        messages = [
            {
                "role": "assistant",
                "content": "",
                "reasoning_content": "dpapi-private-reasoning",
            }
        ]
        metadata = save_private_transcript(
            path,
            run_id="RUN-DPAPI-TEST",
            messages=messages,
        )

        self.assertNotIn(b"dpapi-private-reasoning", path.read_bytes())
        self.assertFalse(metadata["plaintext_recorded_outside_dpapi"])
        recovered = load_private_transcript(
            path, expected_run_id="RUN-DPAPI-TEST"
        )
        self.assertEqual(recovered["messages"], messages)
        with self.assertRaises(PrototypeError):
            load_private_transcript(path, expected_run_id="RUN-WRONG")

        damaged = bytearray(path.read_bytes())
        damaged[-1] ^= 0x01
        path.write_bytes(damaged)
        with self.assertRaises(PrototypeError):
            load_private_transcript(path, expected_run_id="RUN-DPAPI-TEST")

    @unittest.skipUnless(os.name == "nt", "Windows Job Object runtime test")
    def test_windows_process_success_contains_process_and_hashes_output(self) -> None:
        unicode_argument = "space and 中文"
        command = [
            str(Path(sys.executable).resolve()),
            "-c",
            (
                "import sys; "
                "print('x' * 64); "
                "print('stderr', file=sys.stderr); "
                f"raise SystemExit(0 if sys.argv[1] == {unicode_argument!r} else 9)"
            ),
            unicode_argument,
        ]
        report = run_windows_process(
            command,
            cwd=self.root,
            timeout_seconds=5,
            capture_limit_bytes=8,
        )

        self.assertTrue(report["valid"])
        self.assertTrue(report["runtime_ready"])
        self.assertTrue(report["containment"]["job_object_assigned"])
        self.assertFalse(report["command"]["shell"])
        self.assertEqual(report["process"]["terminal_state"], "succeeded")
        self.assertEqual(report["process"]["exit_code"], 0)
        self.assertTrue(report["capture"]["stdout"]["truncated"])
        self.assertEqual(report["capture"]["stdout"]["retained_bytes"], 8)
        self.assertGreater(report["capture"]["stderr"]["total_bytes"], 0)
        self.assertEqual(report["terminal_event"]["event_kind"], "terminal")

    @unittest.skipUnless(os.name == "nt", "Windows Job Object runtime test")
    def test_windows_process_preserves_nonzero_exit(self) -> None:
        report = run_windows_process(
            [str(Path(sys.executable).resolve()), "-c", "raise SystemExit(7)"],
            cwd=self.root,
            timeout_seconds=5,
        )

        self.assertTrue(report["runtime_ready"])
        self.assertEqual(report["process"]["terminal_state"], "failed")
        self.assertEqual(report["process"]["exit_code"], 7)
        self.assertEqual(
            report["terminal_event"]["reason_codes"], ["PROC-EXIT-NONZERO-001"]
        )

    @unittest.skipUnless(os.name == "nt", "Windows Job Object runtime test")
    def test_windows_process_timeout_reaches_one_cancelled_terminal_event(self) -> None:
        report = run_windows_process(
            [
                str(Path(sys.executable).resolve()),
                "-c",
                "import time; print('started', flush=True); time.sleep(30)",
            ],
            cwd=self.root,
            timeout_seconds=0.2,
            cancel_grace_seconds=0.5,
        )

        self.assertTrue(report["runtime_ready"])
        self.assertTrue(report["process"]["timed_out"])
        self.assertEqual(report["process"]["terminal_state"], "cancelled")
        self.assertTrue(report["cancellation"]["requested"])
        self.assertEqual(report["cancellation"]["trigger"], "timeout")
        self.assertEqual(report["terminal_event"]["event_type"], "process.cancelled")
        self.assertEqual(
            report["terminal_event"]["reason_codes"], ["PROC-TIMEOUT-001"]
        )

    @unittest.skipUnless(os.name == "nt", "Windows Job Object runtime test")
    def test_windows_process_cli_smoke_writes_schema_valid_result(self) -> None:
        output_dir = self.root / "windows-smoke"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                ["windows-process-smoke", "--output-dir", str(output_dir)]
            )
        report = json.loads(stdout.getvalue())
        persisted = load_json(output_dir / "windows-process-result.json")

        self.assertEqual(exit_code, 0)
        self.assertEqual(persisted, report)
        self.assertTrue(report["runtime_ready"])
        self.assertEqual(report["process"]["terminal_state"], "succeeded")

    @unittest.skipUnless(os.name == "nt", "Windows action-kernel runtime test")
    def test_action_kernel_cli_smoke_connects_manifest_journal_and_session(self) -> None:
        export_root, _ = self._export("kernel-export")
        output_dir = self.root / "kernel-success"
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = cli_main(
                [
                    "action-kernel-smoke",
                    "--export-root",
                    str(export_root),
                    "--output-dir",
                    str(output_dir),
                ]
            )
        result = json.loads(stdout.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertTrue(result["valid"])
        self.assertTrue(result["runtime_ready"])
        self.assertEqual(result["action_state"], "succeeded")
        self.assertEqual(result["terminal_run_event"], "run_finished")
        self.assertEqual(result["journal_replay"]["event_count"], 8)
        self.assertTrue(verify_action_kernel(output_dir=output_dir)["valid"])
        session = load_json(output_dir / "session-record.json")
        self.assertEqual(session["actions"][0]["state"], "succeeded")
        self.assertEqual(len(session["actions"][0]["events"]), 2)
        self.assertEqual(
            session["actions"][0]["events"][-1]["produced_artifact_ids"],
            ["ART-PROCESS-RESULT-001"],
        )

    @unittest.skipUnless(os.name == "nt", "Windows action-kernel runtime test")
    def test_action_kernel_preserves_failed_action_inside_valid_kernel(self) -> None:
        export_root, _ = self._export("kernel-failure-export")
        result = execute_local_process_action(
            export_root=export_root,
            output_dir=self.root / "kernel-failure",
            command=[str(Path(sys.executable).resolve()), "-c", "raise SystemExit(7)"],
            timeout_seconds=5,
        )

        self.assertTrue(result["valid"])
        self.assertTrue(result["runtime_ready"])
        self.assertEqual(result["action_state"], "failed")
        self.assertEqual(result["terminal_run_event"], "run_failed")

    @unittest.skipUnless(os.name == "nt", "Windows action-kernel runtime test")
    def test_action_kernel_preserves_timeout_as_cancelled(self) -> None:
        export_root, _ = self._export("kernel-timeout-export")
        result = execute_local_process_action(
            export_root=export_root,
            output_dir=self.root / "kernel-timeout",
            command=[
                str(Path(sys.executable).resolve()),
                "-c",
                "import time; print('started', flush=True); time.sleep(30)",
            ],
            timeout_seconds=0.2,
            cancel_grace_seconds=0.5,
        )

        self.assertTrue(result["valid"])
        self.assertTrue(result["runtime_ready"])
        self.assertEqual(result["action_state"], "cancelled")
        self.assertEqual(result["terminal_run_event"], "run_cancelled")

    @unittest.skipUnless(os.name == "nt", "Windows action-kernel runtime test")
    def test_action_kernel_verifier_detects_process_artifact_tampering(self) -> None:
        export_root, _ = self._export("kernel-tamper-export")
        output_dir = self.root / "kernel-tamper"
        execute_local_process_action(
            export_root=export_root,
            output_dir=output_dir,
            command=[str(Path(sys.executable).resolve()), "-c", "print('ok')"],
            timeout_seconds=5,
        )
        process_path = output_dir / "windows-process-result.json"
        process_report = load_json(process_path)
        process_report["process"]["exit_code"] = 99
        process_path.write_text(json.dumps(process_report), encoding="utf-8")

        changed_process_sha = sha256_file(process_path)
        session_path = output_dir / "session-record.json"
        session = load_json(session_path)
        session["artifacts"][0]["sha256"] = changed_process_sha
        session["actions"][0]["events"][-1]["payload"][
            "process_result_sha256"
        ] = changed_process_sha
        session_path.write_text(json.dumps(session), encoding="utf-8")

        result_path = output_dir / "action-kernel-result.json"
        result = load_json(result_path)
        result["artifacts"]["process_result"]["sha256"] = changed_process_sha
        result["artifacts"]["session_record"]["sha256"] = sha256_file(session_path)
        result_path.write_text(json.dumps(result), encoding="utf-8")

        verification = verify_action_kernel(output_dir=output_dir)
        self.assertFalse(verification["valid"])
        self.assertTrue(
            any("journal registered" in error for error in verification["errors"])
        )


if __name__ == "__main__":
    unittest.main()
