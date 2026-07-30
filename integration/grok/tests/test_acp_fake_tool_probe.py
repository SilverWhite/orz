from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[3]
VERIFIER_PATH = ROOT / "scripts" / "verify_grok_acp_fake_tool_probe.py"
CLIENT_PATH = ROOT / "scripts" / "run_grok_acp_fake_tool_client.py"
RESULT_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-acp-fake-tool-probe-result-v0.1.schema.json"
)
VERIFICATION_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-acp-fake-tool-probe-verification-v0.1.schema.json"
)

spec = importlib.util.spec_from_file_location("acp_fake_tool_verifier", VERIFIER_PATH)
assert spec and spec.loader
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)

client_spec = importlib.util.spec_from_file_location(
    "acp_fake_tool_client", CLIENT_PATH
)
assert client_spec and client_spec.loader
client_module = importlib.util.module_from_spec(client_spec)
client_spec.loader.exec_module(client_module)


def _write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _write_jsonl(path: Path, values: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(value, ensure_ascii=False, sort_keys=True) + "\n" for value in values),
        encoding="utf-8",
    )


def _artifact(path: Path) -> dict[str, object]:
    payload = path.read_bytes()
    return {
        "path": str(path.resolve()),
        "bytes": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def _transcript_row(
    sequence: int, direction: str, message: dict[str, object]
) -> dict[str, object]:
    payload = json.dumps(
        message, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")
    return {
        "sequence": sequence,
        "direction": direction,
        "message_bytes": len(payload),
        "message_sha256": hashlib.sha256(payload).hexdigest(),
        "message": message,
    }


def _build_fixture(
    root: Path, scenario: str, *, cancel_failed_terminal: bool = False
) -> tuple[Path, Path]:
    session_id = "session-fixture-001"
    tool_id = "call_lif_read_fixture_001"
    binary = root / "grok.exe"
    binary.write_bytes(b"fixture-binary")
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    lock_path = root / "grok-build.lock.json"
    _write_json(
        lock_path,
        {
            "binary_release": {
                "sha256": binary_sha256,
                "bytes": binary.stat().st_size,
                "version": "0.2.106",
                "authenticode_status": "Valid",
            }
        },
    )

    messages: list[tuple[str, dict[str, object]]] = [
        (
            "client_to_agent",
            {
                "jsonrpc": "2.0",
                "id": "acp-init-1",
                "method": "initialize",
                "params": {"protocolVersion": 1},
            },
        ),
        (
            "agent_to_client",
            {
                "jsonrpc": "2.0",
                "id": "acp-init-1",
                "result": {"protocolVersion": 1},
            },
        ),
        (
            "client_to_agent",
            {
                "jsonrpc": "2.0",
                "id": "acp-session-1",
                "method": "session/new",
                "params": {"cwd": str(root), "mcpServers": []},
            },
        ),
        (
            "agent_to_client",
            {
                "jsonrpc": "2.0",
                "id": "acp-session-1",
                "result": {"sessionId": session_id},
            },
        ),
        (
            "client_to_agent",
            {
                "jsonrpc": "2.0",
                "id": "acp-prompt-1",
                "method": "session/prompt",
                "params": {"sessionId": session_id, "prompt": []},
            },
        ),
        (
            "agent_to_client",
            {
                "jsonrpc": "2.0",
                "method": "session/update",
                "params": {
                    "sessionId": session_id,
                    "update": {
                        "sessionUpdate": "tool_call",
                        "toolCallId": tool_id,
                        "title": "read_file",
                    },
                },
            },
        ),
        (
            "agent_to_client",
            {
                "jsonrpc": "2.0",
                "id": "permission-1",
                "method": "session/request_permission",
                "params": {
                    "sessionId": session_id,
                    "toolCall": {"toolCallId": tool_id},
                    "options": [
                        {
                            "optionId": "allow-once",
                            "name": "Allow once",
                            "kind": "allow_once",
                        },
                        {
                            "optionId": "reject-once",
                            "name": "Reject",
                            "kind": "reject_once",
                        },
                    ],
                },
            },
        ),
    ]
    if scenario == "allow_once":
        messages.extend(
            [
                (
                    "client_to_agent",
                    {
                        "jsonrpc": "2.0",
                        "id": "permission-1",
                        "result": {
                            "outcome": {
                                "outcome": "selected",
                                "optionId": "allow-once",
                            }
                        },
                    },
                ),
                (
                    "agent_to_client",
                    {
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": {
                            "sessionId": session_id,
                            "update": {
                                "sessionUpdate": "tool_call_update",
                                "toolCallId": tool_id,
                                "status": "completed",
                            },
                        },
                    },
                ),
                (
                    "agent_to_client",
                    {
                        "jsonrpc": "2.0",
                        "id": "acp-prompt-1",
                        "result": {"stopReason": "end_turn"},
                    },
                ),
            ]
        )
    else:
        messages.extend(
            [
                (
                    "client_to_agent",
                    {
                        "jsonrpc": "2.0",
                        "method": "session/cancel",
                        "params": {"sessionId": session_id},
                    },
                ),
                (
                    "client_to_agent",
                    {
                        "jsonrpc": "2.0",
                        "id": "permission-1",
                        "result": {"outcome": {"outcome": "cancelled"}},
                    },
                ),
                (
                    "agent_to_client",
                    {
                        "jsonrpc": "2.0",
                        "id": "acp-prompt-1",
                        "result": {"stopReason": "cancelled"},
                    },
                ),
            ]
        )
        if cancel_failed_terminal:
            messages.insert(
                -1,
                (
                    "agent_to_client",
                    {
                        "jsonrpc": "2.0",
                        "method": "session/update",
                        "params": {
                            "sessionId": session_id,
                            "update": {
                                "sessionUpdate": "tool_call_update",
                                "toolCallId": tool_id,
                                "status": "failed",
                            },
                        },
                    },
                ),
            )
    transcript = root / "transcript.private.jsonl"
    transcript_rows = [
        _transcript_row(index, direction, message)
        for index, (direction, message) in enumerate(messages, start=1)
    ]
    _write_jsonl(transcript, transcript_rows)

    events = root / "events.jsonl"
    event_rows: list[dict[str, object]] = [
        {"type": "permission_requested", "tool_name": "read_file"}
    ]
    if scenario == "allow_once":
        event_rows.extend(
            [
                {
                    "type": "permission_resolved",
                    "tool_name": "read_file",
                    "decision": "allow",
                },
                {"type": "tool_completed", "tool_name": "read_file"},
            ]
        )
    else:
        event_rows.append(
            {
                "type": "permission_resolved",
                "tool_name": "read_file",
                "decision": "cancelled",
            }
        )
    _write_jsonl(events, event_rows)

    updates = root / "updates.jsonl"
    session_updates: list[dict[str, object]] = [
        {
            "method": "session/update",
            "params": {
                "sessionId": session_id,
                "update": {
                    "sessionUpdate": "tool_call",
                    "toolCallId": tool_id,
                },
            },
        }
    ]
    if scenario == "allow_once":
        session_updates.append(
            {
                "method": "session/update",
                "params": {
                    "sessionId": session_id,
                    "update": {
                        "sessionUpdate": "tool_call_update",
                        "toolCallId": tool_id,
                        "status": "completed",
                    },
                },
            }
        )
    elif cancel_failed_terminal:
        session_updates.append(
            {
                "method": "session/update",
                "params": {
                    "sessionId": session_id,
                    "update": {
                        "sessionUpdate": "tool_call_update",
                        "toolCallId": tool_id,
                        "status": "failed",
                    },
                },
            }
        )
    _write_jsonl(updates, session_updates)

    provider_private = root / "requests.private.jsonl"
    private_rows = [
        {
            "sequence": 1,
            "method": "POST",
            "path": "/chat/completions",
            "client_ip": "127.0.0.1",
            "headers": {
                "authorization": {
                    "present": True,
                    "value_recorded": False,
                }
            },
            "body": {
                "model": "grok-4.5",
                "stream": True,
                "tool_choice": {
                    "type": "function",
                    "function": {"name": "session_title"},
                },
                "tools": [
                    {
                        "type": "function",
                        "function": {"name": "session_title"},
                    }
                ],
            },
        },
        {
            "sequence": 2,
            "body": {
                "model": "deepseek-v4-pro",
                "messages": [{"role": "user"}],
            },
        }
    ]
    if scenario == "allow_once":
        private_rows.append(
            {
                "sequence": 3,
                "body": {
                    "model": "deepseek-v4-pro",
                    "messages": [{"role": "tool"}],
                },
            }
        )
    _write_jsonl(provider_private, private_rows)
    provider_result = root / "provider-result.json"
    continuity = (
        {
            "second_request_observed": True,
            "reasoning_marker_preserved": True,
            "tool_call_id_preserved": True,
            "tool_result_observed": True,
            "tool_result_marker_observed": True,
        }
        if scenario == "allow_once"
        else {
            "first_tool_request_observed": True,
            "second_request_observed": False,
        }
    )
    _write_json(
        provider_result,
        {
            "scenario": "tool-continuity"
            if scenario == "allow_once"
            else "tool-cancel",
            "terminal_state": "succeeded",
            "continuity": continuity,
            "response": {"real_model_invoked": False},
        },
    )

    receipt_paths = []
    for name in ("preflight", "launch", "postrun"):
        path = root / f"workspace-trust.{name}.json"
        _write_json(
            path,
            {
                "valid": True,
                "workspace": {"canonical_path": str(root.resolve())},
                "discovery": {
                    "complete": True,
                    "candidate_count": 0,
                    "aggregate_sha256": "a" * 64,
                    "scan_policy_sha256": "b" * 64,
                },
                "decision": {"mode": "restricted", "launch_permitted": True},
            },
        )
        receipt_paths.append(path)

    acp = {
        "protocol_version": 1,
        "session_id": session_id,
        "request_methods": ["initialize", "session/new", "session/prompt"],
        "update_types": ["tool_call"]
        + (
            ["tool_call_update"]
            if scenario == "allow_once" or cancel_failed_terminal
            else []
        ),
        "tool_call_id": tool_id,
        "tool_call_count": 1,
        "tool_update_count": (
            1 if scenario == "allow_once" or cancel_failed_terminal else 0
        ),
        "terminal_tool_update_count": (
            1 if scenario == "allow_once" or cancel_failed_terminal else 0
        ),
        "tool_statuses": (
            ["completed"]
            if scenario == "allow_once"
            else (["failed"] if cancel_failed_terminal else [])
        ),
        "permission_request_count": 1,
        "permission_option_kinds": ["allow_once", "reject_once"],
        "permission_outcome": "allow_once"
        if scenario == "allow_once"
        else "cancelled",
        "cancel_notification_sent": scenario != "allow_once",
        "prompt_response_count": 1,
        "prompt_stop_reason": "end_turn"
        if scenario == "allow_once"
        else "cancelled",
        "extension_notification_counts": {
            method: 0 for method in sorted(verifier.ALLOWED_EXTENSION_NOTIFICATIONS)
        },
        "unexpected_message_count": 0,
        "parse_failure_count": 0,
    }
    session_evidence = {
        "discovered": True,
        "event_types": [row["type"] for row in event_rows],
        "update_types": ["tool_call"]
        + (
            ["tool_call_update"]
            if scenario == "allow_once" or cancel_failed_terminal
            else []
        ),
        "tool_call_ids": [tool_id],
        "permission_decisions": ["allow"]
        if scenario == "allow_once"
        else ["cancelled"],
        "completed_tool_event_count": 1 if scenario == "allow_once" else 0,
    }
    provider = {
        "scenario": "tool-continuity"
        if scenario == "allow_once"
        else "tool-cancel",
        "terminal_state": "succeeded",
        "primary_request_count": 2 if scenario == "allow_once" else 1,
        "auxiliary_request_count": 1,
        "auxiliary_request_validated": True,
        "second_request_observed": scenario == "allow_once",
        "reasoning_marker_preserved": scenario == "allow_once",
        "tool_call_id_preserved": scenario == "allow_once",
        "tool_result_observed": scenario == "allow_once",
        "tool_result_marker_observed": scenario == "allow_once",
        "real_model_invoked": False,
    }
    result = {
        "schema_version": "0.1.0",
        "probe_kind": "grok-acp-fake-tool-continuity",
        "probe_id": "ACPTOOL-" + "1" * 32,
        "scenario": scenario,
        "valid": True,
        "started_at": "2026-07-23T00:00:00Z",
        "completed_at": "2026-07-23T00:00:01Z",
        "binary": {
            **_artifact(binary),
            "locked_sha256": binary_sha256,
            "version": "0.2.106",
            "authenticode_status": "Valid",
        },
        "artifacts": {
            "transcript": _artifact(transcript),
            "provider_result": _artifact(provider_result),
            "provider_private_capture": _artifact(provider_private),
            "workspace_trust_preflight": _artifact(receipt_paths[0]),
            "workspace_trust_launch": _artifact(receipt_paths[1]),
            "workspace_trust_postrun": _artifact(receipt_paths[2]),
            "session_events": _artifact(events),
            "session_updates": _artifact(updates),
        },
        "acp": acp,
        "session_evidence": session_evidence,
        "provider": provider,
        "process": {
            "exit_code": 0,
            "timed_out": False,
            "duration_ms": 1.0,
            "job_object_created": True,
            "job_object_assigned": True,
            "job_object_closed": True,
        },
        "controls": {
            "loopback_only_provider": True,
            "clean_environment": True,
            "credential_value_recorded": False,
            "outbound_block_created": True,
            "outbound_rules_removed": True,
            "remaining_rule_count": 0,
            "workspace_disposable": True,
            "workspace_modified": False,
            "output_overwritten": False,
        },
        "checks": {"fixture_check": True},
        "limitations": ["Synthetic verifier fixture only."],
    }
    result_path = root / "result.json"
    _write_json(result_path, result)
    Draft202012Validator(json.loads(RESULT_SCHEMA.read_text(encoding="utf-8"))).validate(
        result
    )
    return result_path, lock_path


class AcpFakeToolVerifierTests(unittest.TestCase):
    def test_client_accepts_early_mcp_session_id_and_richer_running_queue(self) -> None:
        client = client_module.AcpClient.__new__(client_module.AcpClient)
        client.session_id = ""
        client.early_session_id = ""
        session_id = "session-fixture-001"
        self.assertTrue(
            client._is_allowed_extension_notification(
                {
                    "jsonrpc": "2.0",
                    "method": "_x.ai/mcp_initialized",
                    "params": {
                        "elapsedMs": 0,
                        "mcpToolCount": 0,
                        "sessionId": session_id,
                    },
                }
            )
        )
        self.assertEqual(client.early_session_id, session_id)
        client.session_id = session_id
        self.assertTrue(
            client._is_allowed_extension_notification(
                {
                    "jsonrpc": "2.0",
                    "method": "_x.ai/queue/changed",
                    "params": {
                        "entries": [],
                        "runningKind": "prompt",
                        "runningPromptId": "prompt-fixture-001",
                        "runningText": "fixture prompt",
                        "sessionId": session_id,
                    },
                }
            )
        )

    def test_allow_once_fixture_rebuilds_all_three_evidence_sources(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(Path(temporary), "allow_once")
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertTrue(report["valid"], report["errors"])
            self.assertTrue(all(report["checks"].values()))
            self.assertEqual(report["scenario"], "allow_once")
            self.assertEqual(report["permission_outcome"], "allow_once")
            self.assertEqual(report["provider_scenario"], "tool-continuity")
            Draft202012Validator(
                json.loads(VERIFICATION_SCHEMA.read_text(encoding="utf-8"))
            ).validate(report)

    def test_cancel_fixture_requires_cancelled_prompt_and_no_second_request(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(
                Path(temporary), "cancel_permission"
            )
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertTrue(report["valid"], report["errors"])
            self.assertEqual(report["scenario"], "cancel_permission")
            self.assertEqual(report["permission_outcome"], "cancelled")
            self.assertEqual(report["provider_scenario"], "tool-cancel")
            Draft202012Validator(
                json.loads(VERIFICATION_SCHEMA.read_text(encoding="utf-8"))
            ).validate(report)

    def test_cancel_fixture_accepts_failed_terminal_without_tool_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(
                Path(temporary),
                "cancel_permission",
                cancel_failed_terminal=True,
            )
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertTrue(report["valid"], report["errors"])

    def test_transcript_tampering_is_detected_even_when_result_is_unchanged(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(Path(temporary), "allow_once")
            result = json.loads(result_path.read_text(encoding="utf-8"))
            transcript = Path(result["artifacts"]["transcript"]["path"])
            transcript.write_text(
                transcript.read_text(encoding="utf-8").replace(
                    '"stopReason": "end_turn"', '"stopReason": "cancelled"'
                ),
                encoding="utf-8",
            )
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["artifact_digests_match"])
            self.assertFalse(report["checks"]["transcript_projection_matches"])

    def test_invalid_extension_shape_is_not_accepted_after_rehash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(Path(temporary), "allow_once")
            result = json.loads(result_path.read_text(encoding="utf-8"))
            transcript = Path(result["artifacts"]["transcript"]["path"])
            rows = [
                json.loads(line)
                for line in transcript.read_text(encoding="utf-8").splitlines()
            ]
            rows.append(
                _transcript_row(
                    len(rows) + 1,
                    "agent_to_client",
                    {
                        "jsonrpc": "2.0",
                        "method": "_x.ai/mcp_initialized",
                        "params": {
                            "elapsedMs": 0,
                            "mcpToolCount": 0,
                            "sessionId": "wrong-session",
                        },
                    },
                )
            )
            _write_jsonl(transcript, rows)
            result["artifacts"]["transcript"] = _artifact(transcript)
            _write_json(result_path, result)
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertFalse(report["valid"])
            self.assertTrue(report["checks"]["artifact_digests_match"])
            self.assertFalse(report["checks"]["transcript_projection_matches"])

    def test_non_title_auxiliary_request_is_rejected_after_rehash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path, lock_path = _build_fixture(Path(temporary), "allow_once")
            result = json.loads(result_path.read_text(encoding="utf-8"))
            private_path = Path(
                result["artifacts"]["provider_private_capture"]["path"]
            )
            rows = [
                json.loads(line)
                for line in private_path.read_text(encoding="utf-8").splitlines()
            ]
            rows[0]["body"]["tools"][0]["function"]["name"] = "unexpected_tool"
            _write_jsonl(private_path, rows)
            result["artifacts"]["provider_private_capture"] = _artifact(private_path)
            _write_json(result_path, result)
            report = verifier.verify(result_path, lock_path=lock_path)
            self.assertFalse(report["valid"])
            self.assertTrue(report["checks"]["artifact_digests_match"])
            self.assertFalse(report["checks"]["provider_capture_matches"])
            self.assertFalse(report["checks"]["scenario_semantics_match"])


if __name__ == "__main__":
    unittest.main()
