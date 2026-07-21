from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any
import uuid

from .approval_ledger import (
    ApprovalLedger,
    ConsoleConfirmationIO,
    InteractivePermitBroker,
    verify_approval_ledger,
)
from .brokered_transport import BrokeredDeepSeekHttpsTransport
from .credentials import InMemoryCredentialProvider
from .deepseek_adapter import classify_http_status, validate_deepseek_profile
from .deepseek_client import (
    build_deepseek_request,
    consume_deepseek_sse,
    decode_strict_json_object,
    decode_tool_arguments,
)
from .errors import PrototypeError
from .fake_https_provider import (
    FakeHttpsExchange,
    InProcessFakeDeepSeekConnectionFactory,
)
from .io_utils import (
    atomic_write_bytes,
    atomic_write_json,
    canonical_bytes,
    load_json,
    safe_relative_path,
    sha256_bytes,
    sha256_file,
    utc_now,
)
from .journal import append_event, replay_journal
from .layout import DESIGN_ROOT, PROTOCOL_ROOT, RUNTIME_ROOT
from .loopback_http import LoopbackHttpTransport
from .loopback_mock_server import LoopbackMockServer, MockHttpExchange
from .model_transport import (
    ScriptedTransport,
    StreamingTransport,
    TransportAttemptContext,
    TransportControl,
    TransportResponse,
)
from .private_transcript import load_private_transcript, save_private_transcript
from .network_broker import (
    PermitBroker,
    ScriptedPermitBroker,
    ScriptedPermitDecision,
    confirmation_summary_sha256,
)
from .schema import validate_instance


MOCK_TOOL_NAME = "mock_echo"


def _sse_data(payload: dict[str, Any]) -> str:
    return "data: " + json.dumps(
        payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    )


def create_default_scripted_transport() -> ScriptedTransport:
    first = TransportResponse(
        status=200,
        headers={"content-type": "text/event-stream"},
        lines=(
            ": keep-alive",
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"reasoning_content": "mock-private-plan"},
                            "finish_reason": None,
                        }
                    ]
                }
            ),
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {
                                "tool_calls": [
                                    {
                                        "index": 0,
                                        "id": "call_mock_echo_001",
                                        "type": "function",
                                        "function": {
                                            "name": MOCK_TOOL_NAME,
                                            "arguments": "{\"value\":",
                                        },
                                    }
                                ]
                            },
                            "finish_reason": None,
                        }
                    ]
                }
            ),
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {
                                "tool_calls": [
                                    {"index": 0, "function": {"arguments": "42}"}}
                                ]
                            },
                            "finish_reason": "tool_calls",
                        }
                    ]
                }
            ),
            "data: [DONE]",
        ),
    )
    second = TransportResponse(
        status=200,
        headers={"content-type": "text/event-stream"},
        lines=(
            ": keep-alive",
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"reasoning_content": "mock-private-final"},
                            "finish_reason": None,
                        }
                    ]
                }
            ),
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {"content": "{\"status\":\"mock-complete\"}"},
                            "finish_reason": None,
                        }
                    ]
                }
            ),
            _sse_data(
                {
                    "choices": [],
                    "usage": {
                        "prompt_tokens": 100,
                        "completion_tokens": 20,
                        "total_tokens": 120,
                    },
                }
            ),
            _sse_data(
                {
                    "choices": [
                        {
                            "index": 0,
                            "delta": {},
                            "finish_reason": "stop",
                        }
                    ]
                }
            ),
            "data: [DONE]",
        ),
    )
    return ScriptedTransport([first, second])


def _write_model_manifest(
    *,
    export_root: Path,
    output_dir: Path,
    run_id: str,
    profile: dict[str, Any],
    transport: StreamingTransport,
    persist_private_transcript: bool,
) -> tuple[dict[str, Any], Path, Path]:
    export_path = export_root / "scenario-export-manifest.json"
    export_manifest = load_json(export_path)
    validate_instance(
        export_manifest,
        RUNTIME_ROOT / "scenario-export-manifest-v0.1.schema.json",
        label="scenario export manifest",
    )
    if export_manifest["status"] != "ready":
        raise PrototypeError("model loop requires a ready development export")

    system_prompt = output_dir / "system-prompt.txt"
    project_rules = output_dir / "project-rules.txt"
    generated_context = output_dir / "generated-context.json"
    redaction_policy = output_dir / "redaction-policy.json"
    profile_path = output_dir / "deepseek-profile.json"
    atomic_write_bytes(
        system_prompt,
        b"MOCK_ONLY: use mock_echo and return a JSON object; no scientific conclusion.\n",
    )
    atomic_write_bytes(
        project_rules,
        b"NO_REAL_NETWORK_OR_API_KEY; REASONING_CONTENT_IS_PROVIDER_PRIVATE.\n",
    )
    atomic_write_json(
        generated_context,
        {"scenario_content_injected": False, "purpose": "adapter loop contract smoke"},
    )
    atomic_write_json(
        redaction_policy,
        {
            "version": "0.1.0-prototype",
            "request_messages": "digest_and_roles_only",
            "response_content": "digest_and_byte_count_only",
            "provider_reasoning": "digest_and_byte_count_only",
        },
    )
    atomic_write_json(profile_path, profile)
    tools_policy = {
        "allowlist": [MOCK_TOOL_NAME],
        "fixed_fixture_only": True,
        "real_network": transport.real_network,
    }
    manifest = {
        "schema_version": "0.1.0-draft",
        "manifest_kind": "immutable_precommit",
        "run_id": run_id,
        "created_at": utc_now(),
        "execution_mode": "development",
        "scenario_export": {
            "export_id": export_manifest["export_id"],
            "manifest_sha256": sha256_file(export_path),
            "bundle_sha256": export_manifest["bundle_sha256"],
            "leak_scan_report_sha256": export_manifest["leak_scan"]["report_sha256"],
        },
        "adapter": {
            "provider": "deepseek",
            "model_id": profile["model"],
            "adapter_id": "deepseek-openai-scripted-loop",
            "adapter_version": "0.1.0-prototype",
            "capabilities": {
                "structured_output": profile["structured_output"]["mode"]
                == "json_object",
                "tool_calling": profile["tools"]["enabled"],
                "streaming": True,
                "seed_control": False,
                "cancellation": False,
                "context_limit_tokens": 1_000_000,
            },
        },
        "prompts": {
            "system_prompt_sha256": sha256_file(system_prompt),
            "project_rules_sha256": sha256_file(project_rules),
            "generated_context_sha256": sha256_file(generated_context),
        },
        "tools": {
            "allowlist": [MOCK_TOOL_NAME],
            "policy_sha256": sha256_bytes(canonical_bytes(tools_policy)),
            "permission_mode": "preapproved_narrow",
            "filesystem_profile": "strict_ephemeral",
            "network_profile": "disabled",
        },
        "isolation": {
            "memory": "enabled" if persist_private_transcript else "session_only",
            "historical_case_retrieval": "disabled",
            "oracle_mounted": False,
            "reviewer_fixtures_mounted": False,
            "inherited_user_config": False,
            "writable_roots": [str(output_dir.resolve())],
        },
        "budgets": {
            "wall_time_seconds": profile["timeouts"]["total_seconds"],
            "max_turns": 2,
            "max_input_tokens": 1_000_000,
            "max_output_tokens": profile["structured_output"]["max_tokens"] or 1,
            "max_tool_calls": 1,
        },
        "randomness": {
            "seed": None,
            "temperature": 0,
            "retry_policy": (
                "transport_only"
                if profile["retry"]["max_attempts"] > 1
                else "none"
            ),
        },
        "protocol_digests": {
            "agent_protocol": sha256_file(PROTOCOL_ROOT / "agent-protocol-v0.1.schema.json"),
            "reason_codes": sha256_file(PROTOCOL_ROOT / "reason-codes-v0.1.yaml"),
            "gate_matrix": sha256_file(PROTOCOL_ROOT / "gate-matrix-v0.1.yaml"),
            "scoring_protocol": sha256_file(
                DESIGN_ROOT / "evaluation/SCORING_PROTOCOL_v0.1.md"
            ),
            "output_schema": sha256_file(
                DESIGN_ROOT / "evaluation/evaluation-result-v0.1.schema.json"
            ),
        },
        "journal_policy": {
            "format": "jsonl",
            "event_schema_version": "0.1.0-draft",
            "canonicalization": "RFC8785",
            "hash_chain": "sha256",
            "redaction_policy_sha256": sha256_file(redaction_policy),
        },
        "notes": [
            (
                "Brokered in-process fake HTTPS constructs a test-only Bearer header and TLS context but performs no DNS, socket, external endpoint, or real DeepSeek request."
                if transport.transport_id == "deepseek-brokered-fake-https-v0.1"
                else f"Mock transport {transport.transport_id}; no API key, DNS, TLS, proxy, external endpoint, or real DeepSeek request."
            ),
            "Manifest temperature is a v0.1 schema placeholder and is not sent in thinking mode.",
            (
                "Provider-private transcript is encrypted with current-user Windows DPAPI."
                if persist_private_transcript
                else "Provider-private transcript is process-memory-only."
            ),
        ],
    }
    validate_instance(
        manifest,
        RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
        label="model-loop run manifest",
    )
    manifest_path = output_dir / "run-manifest.json"
    atomic_write_json(manifest_path, manifest)
    return manifest, manifest_path, profile_path


def _append(
    journal_path: Path,
    *,
    run_id: str,
    manifest_sha256: str,
    event_type: str,
    payload: dict[str, Any],
) -> dict[str, Any]:
    return append_event(
        journal_path,
        run_id=run_id,
        run_manifest_sha256=manifest_sha256,
        event_type=event_type,
        payload_schema=f"prototype/model-loop/{event_type}-v0.1",
        payload=payload,
        redaction="metadata_only",
    )


def _perform_turn(
    *,
    turn: int,
    profile: dict[str, Any],
    messages: list[dict[str, Any]],
    tools: list[dict[str, Any]],
    transport: StreamingTransport,
    journal_path: Path,
    run_id: str,
    manifest_sha256: str,
    transport_control: TransportControl | None,
) -> tuple[dict[str, Any], dict[str, Any]]:
    request, request_metadata = build_deepseek_request(
        profile=profile,
        messages=messages,
        tools=tools,
    )
    attempts: list[dict[str, Any]] = []
    previous_status: int | None = None
    for attempt in range(1, profile["retry"]["max_attempts"] + 1):
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="model_request",
            payload={"turn": turn, "attempt": attempt, **request_metadata},
        )
        response = transport.send(
            request,
            transport_control,
            TransportAttemptContext(
                turn=turn,
                attempt=attempt,
                previous_status=previous_status,
            ),
        )
        classification = (
            {"status": 200, "category": "success", "retryable": False}
            if response.status == 200
            else classify_http_status(response.status)
        )
        attempt_record = {
            "attempt": attempt,
            "status": response.status,
            "category": classification["category"],
            "retryable": classification["retryable"],
            "transport_metadata": response.metadata,
        }
        attempts.append(attempt_record)
        if response.status != 200:
            _append(
                journal_path,
                run_id=run_id,
                manifest_sha256=manifest_sha256,
                event_type="model_output",
                payload={"turn": turn, **attempt_record, "response_recorded": False},
            )
            retry_allowed = (
                response.status in profile["retry"]["retry_http_statuses"]
                and classification["retryable"]
                and attempt < profile["retry"]["max_attempts"]
            )
            if retry_allowed:
                previous_status = response.status
                continue
            raise PrototypeError(
                f"DeepSeek scripted transport stopped at HTTP {response.status}"
            )

        assistant_message, public_response = consume_deepseek_sse(response.lines)
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="model_output",
            payload={
                "turn": turn,
                **attempt_record,
                "response_recorded": True,
                "response": public_response,
            },
        )
        return assistant_message, {
            "turn": turn,
            "request_sha256": request_metadata["request_sha256"],
            "message_count": request_metadata["message_count"],
            "reasoning_message_count": request_metadata["reasoning_message_count"],
            "attempts": attempts,
            "response": public_response,
        }
    raise PrototypeError("DeepSeek retry loop exhausted without terminal response")


def _execute_mock_tool(tool_call: dict[str, Any]) -> str:
    if tool_call.get("function", {}).get("name") != MOCK_TOOL_NAME:
        raise PrototypeError("mock model loop proposed a non-allowlisted tool")
    arguments = decode_tool_arguments(tool_call)
    if arguments != {"value": 42}:
        raise PrototypeError("mock_echo arguments differ from the frozen fixture")
    return json.dumps({"echo": 42}, sort_keys=True, separators=(",", ":"))


def execute_deepseek_mock_loop(
    *,
    export_root: Path,
    output_dir: Path,
    profile: dict[str, Any],
    transport: StreamingTransport | None = None,
    transport_control: TransportControl | None = None,
    persist_private_transcript: bool = False,
    network_approval_ledger_path: Path | None = None,
) -> dict[str, Any]:
    profile_report = validate_deepseek_profile(profile)
    if not profile_report["valid"]:
        raise PrototypeError(f"invalid DeepSeek profile: {profile_report['issues']}")
    if profile["transport"] != "openai_chat_completions":
        raise PrototypeError("mock model loop requires OpenAI Chat Completions transport")
    if profile["thinking"]["type"] != "enabled" or not profile["tools"]["enabled"]:
        raise PrototypeError("mock model loop requires thinking and tools enabled")
    if profile["structured_output"]["mode"] != "json_object":
        raise PrototypeError("mock model loop requires the frozen JSON output profile")
    if output_dir.exists():
        raise PrototypeError(f"refusing to overwrite existing model-loop root: {output_dir}")
    selected_transport = transport or create_default_scripted_transport()
    if selected_transport.real_network:
        raise PrototypeError("mock model loop refuses real-network transports")
    if persist_private_transcript and os.name != "nt":
        raise PrototypeError("persistent private transcript requires Windows DPAPI")
    if (
        network_approval_ledger_path is not None
        and network_approval_ledger_path.absolute()
        != (output_dir / "network-approvals.jsonl").absolute()
    ):
        raise PrototypeError("network approval ledger must use the fixed output filename")

    output_dir.mkdir(parents=True)
    run_id = f"RUN-MODEL-{uuid.uuid4().hex[:16].upper()}"
    journal_path = output_dir / "events.jsonl"
    manifest_sha256: str | None = None
    try:
        _, manifest_path, profile_path = _write_model_manifest(
            export_root=export_root,
            output_dir=output_dir,
            run_id=run_id,
            profile=profile,
            transport=selected_transport,
            persist_private_transcript=persist_private_transcript,
        )
        manifest_sha256 = sha256_file(manifest_path)
        approval_ledger_metadata: dict[str, Any] | None = None
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="run_preflight",
            payload={
                "profile_valid": True,
                "transport": selected_transport.transport_id,
                "real_network": selected_transport.real_network,
                "api_key_read": False,
            },
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="run_started",
            payload={"planned_model_turns": 2, "planned_tool_calls": 1},
        )

        tools = [
            {
                "type": "function",
                "function": {
                    "name": MOCK_TOOL_NAME,
                    "description": "Return a deterministic echo for adapter-loop testing.",
                    "parameters": {
                        "type": "object",
                        "additionalProperties": False,
                        "required": ["value"],
                        "properties": {"value": {"const": 42}},
                    },
                },
            }
        ]
        messages: list[dict[str, Any]] = [
            {
                "role": "system",
                "content": "Use mock_echo once, then return the documented JSON example.",
            },
            {"role": "user", "content": "Run the no-network adapter contract smoke."},
        ]
        turns: list[dict[str, Any]] = []
        first_message, first_turn = _perform_turn(
            turn=1,
            profile=profile,
            messages=messages,
            tools=tools,
            transport=selected_transport,
            journal_path=journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            transport_control=transport_control,
        )
        turns.append(first_turn)
        if first_turn["response"]["finish_reason"] != "tool_calls":
            raise PrototypeError("first scripted DeepSeek turn must finish with tool_calls")
        tool_calls = first_message.get("tool_calls", [])
        if len(tool_calls) != 1:
            raise PrototypeError("mock model loop requires exactly one tool call")
        tool_call = tool_calls[0]
        public_tool = first_turn["response"]["tool_calls"][0]
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_proposal",
            payload={"turn": 1, **public_tool},
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="permission_decision",
            payload={
                "decision": "allow",
                "policy": "fixed_mock_tool_allowlist",
                "tool_name": MOCK_TOOL_NAME,
            },
        )
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_started",
            payload={"tool_name": MOCK_TOOL_NAME, "id_sha256": public_tool["id_sha256"]},
        )
        tool_result = _execute_mock_tool(tool_call)
        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="tool_completed",
            payload={
                "tool_name": MOCK_TOOL_NAME,
                "terminal_state": "succeeded",
                "result_sha256": sha256_bytes(tool_result.encode("utf-8")),
                "raw_result_recorded": False,
            },
        )
        messages.append(first_message)
        messages.append(
            {
                "role": "tool",
                "tool_call_id": tool_call["id"],
                "content": tool_result,
            }
        )
        private_transcript_path = output_dir / "provider-private-transcript.dpapi"
        private_transcript_metadata: dict[str, Any] | None = None
        if persist_private_transcript:
            private_transcript_metadata = save_private_transcript(
                private_transcript_path,
                run_id=run_id,
                messages=messages,
            )

        second_message, second_turn = _perform_turn(
            turn=2,
            profile=profile,
            messages=messages,
            tools=tools,
            transport=selected_transport,
            journal_path=journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            transport_control=transport_control,
        )
        turns.append(second_turn)
        if second_turn["response"]["finish_reason"] != "stop":
            raise PrototypeError("second scripted DeepSeek turn must finish with stop")
        if profile["structured_output"]["mode"] == "json_object":
            decode_strict_json_object(
                second_message["content"], label="final DeepSeek JSON output"
            )
        messages.append(second_message)
        if persist_private_transcript:
            private_transcript_metadata = save_private_transcript(
                private_transcript_path,
                run_id=run_id,
                messages=messages,
                overwrite=True,
            )

        if network_approval_ledger_path is not None:
            approval_report = verify_approval_ledger(network_approval_ledger_path)
            expected_attempts = sum(len(turn["attempts"]) for turn in turns)
            if not approval_report["valid"]:
                raise PrototypeError(
                    f"network approval ledger is invalid: {approval_report['errors']}"
                )
            if approval_report["event_count"] != expected_attempts:
                raise PrototypeError("network approval count differs from HTTP attempts")
            if approval_report["allow_count"] != expected_attempts:
                raise PrototypeError("completed model loop contains a denied network attempt")
            approval_ledger_metadata = {
                "event_count": approval_report["event_count"],
                "terminal_event_sha256": approval_report["terminal_event_sha256"],
                "sha256": sha256_file(network_approval_ledger_path),
            }

        _append(
            journal_path,
            run_id=run_id,
            manifest_sha256=manifest_sha256,
            event_type="run_finished",
            payload={
                "model_turns": 2,
                "tool_calls": 1,
                "real_network": False,
                "claim_eligibility": "not_assessed",
                "final_content_sha256": second_turn["response"]["content_sha256"],
                "private_transcript_persisted": persist_private_transcript,
                "private_transcript_ciphertext_sha256": (
                    private_transcript_metadata["ciphertext_sha256"]
                    if private_transcript_metadata is not None
                    else None
                ),
                "network_approval_ledger": approval_ledger_metadata,
            },
        )
        replay = replay_journal(
            run_manifest_path=manifest_path,
            journal_path=journal_path,
        )
        http_attempt_count = sum(len(turn["attempts"]) for turn in turns)
        result = {
            "schema_version": "0.1.0-prototype",
            "loop": "deepseek-mock-tool-loop",
            "run_id": run_id,
            "valid": replay["valid"],
            "runtime_ready": replay["valid"],
            "provider": "deepseek",
            "model": profile["model"],
            "transport": selected_transport.transport_id,
            "real_network": selected_transport.real_network,
            "private_transcript_persisted": persist_private_transcript,
            "request_count": len(turns),
            "http_attempt_count": http_attempt_count,
            "retry_count": http_attempt_count - len(turns),
            "tool_call_count": 1,
            "reasoning_continuity_checked": (
                first_turn["response"]["reasoning_present"]
                and second_turn["reasoning_message_count"] == 1
            ),
            "turns": turns,
            "final_output": {
                "finish_reason": second_turn["response"]["finish_reason"],
                "content_bytes": second_turn["response"]["content_bytes"],
                "content_sha256": second_turn["response"]["content_sha256"],
            },
            "journal_replay": {
                "valid": replay["valid"],
                "event_count": replay["event_count"],
                "terminal_event": replay["terminal_event"],
            },
            "artifacts": {
                "run_manifest": {
                    "path": manifest_path.name,
                    "sha256": sha256_file(manifest_path),
                },
                "journal": {
                    "path": journal_path.name,
                    "sha256": sha256_file(journal_path),
                },
                "profile_snapshot": {
                    "path": profile_path.name,
                    "sha256": sha256_file(profile_path),
                },
                "private_transcript": (
                    {
                        "path": private_transcript_path.name,
                        "sha256": private_transcript_metadata["ciphertext_sha256"],
                    }
                    if private_transcript_metadata is not None
                    else None
                ),
                "network_approval_ledger": (
                    {
                        "path": network_approval_ledger_path.name,
                        "sha256": approval_ledger_metadata["sha256"],
                    }
                    if network_approval_ledger_path is not None
                    and approval_ledger_metadata is not None
                    else None
                ),
            },
            "limitations": [
                (
                    "Brokered fake HTTPS performs test-only permit, TLS-context, and Bearer-header mechanics without DNS, socket, real credential, or external DeepSeek request."
                    if selected_transport.transport_id
                    == "deepseek-brokered-fake-https-v0.1"
                    else "Mock transport performs no external DeepSeek request or authentication work."
                ),
                (
                    "Provider-private transcript is current-user DPAPI encrypted; plaintext still exists transiently in process memory."
                    if persist_private_transcript
                    else "Provider-private reasoning exists only in process memory and cannot be crash-recovered."
                ),
                "This verifies adapter mechanics, not model quality, evidence, or claim eligibility.",
            ],
        }
        validate_instance(
            result,
            RUNTIME_ROOT / "model-loop-result-v0.1.schema.json",
            label="model-loop result",
        )
        atomic_write_json(output_dir / "model-loop-result.json", result)
        verification = verify_model_loop(output_dir=output_dir)
        if not verification["valid"]:
            raise PrototypeError(
                f"model-loop cross-file verification failed: {verification['errors']}"
            )
        return result
    except Exception as exc:
        if manifest_sha256 is not None:
            try:
                _append(
                    journal_path,
                    run_id=run_id,
                    manifest_sha256=manifest_sha256,
                    event_type="run_failed",
                    payload={
                        "error_type": type(exc).__name__,
                        "error_sha256": sha256_bytes(str(exc).encode("utf-8")),
                        "raw_error_recorded": False,
                    },
                )
            except Exception:
                pass
        try:
            atomic_write_json(
                output_dir / "_INCOMPLETE.json",
                {
                    "status": "incomplete",
                    "error_type": type(exc).__name__,
                    "error_sha256": sha256_bytes(str(exc).encode("utf-8")),
                    "recorded_at": utc_now(),
                },
            )
        except Exception:
            pass
        raise


def create_deepseek_model_loop_smoke(
    *, export_root: Path, profile_path: Path, output_dir: Path
) -> dict[str, Any]:
    profile = load_json(profile_path)
    return execute_deepseek_mock_loop(
        export_root=export_root,
        output_dir=output_dir,
        profile=profile,
        transport=create_default_scripted_transport(),
    )


def create_deepseek_loopback_http_smoke(
    *, export_root: Path, profile_path: Path, output_dir: Path
) -> dict[str, Any]:
    profile = load_json(profile_path)
    scripted = create_default_scripted_transport()
    first = scripted.send({"fixture": "loopback-first"})
    second = scripted.send({"fixture": "loopback-second"})
    exchanges = [
        MockHttpExchange(status=first.status, lines=first.lines),
        MockHttpExchange(status=second.status, lines=second.lines),
    ]
    with LoopbackMockServer(exchanges) as server:
        transport = LoopbackHttpTransport(server.base_url)
        control = TransportControl(
            connect_seconds=min(profile["timeouts"]["connect_seconds"], 5),
            first_semantic_seconds=min(
                profile["timeouts"]["first_inference_seconds"], 30
            ),
            total_seconds=min(profile["timeouts"]["total_seconds"], 60),
        )
        result = execute_deepseek_mock_loop(
            export_root=export_root,
            output_dir=output_dir,
            profile=profile,
            transport=transport,
            transport_control=control,
            persist_private_transcript=os.name == "nt",
        )
        requests = server.requests
    if len(requests) != 2:
        raise PrototypeError("loopback mock server did not receive exactly two requests")
    if requests[1].get("messages", [])[2].get("reasoning_content") != "mock-private-plan":
        raise PrototypeError("loopback second request lost provider reasoning continuity")
    return result


def create_deepseek_brokered_fake_https_smoke(
    *,
    export_root: Path,
    profile_path: Path,
    output_dir: Path,
    permit_broker: PermitBroker | None = None,
    network_approval_ledger_path: Path | None = None,
) -> dict[str, Any]:
    profile = load_json(profile_path)
    scripted = create_default_scripted_transport()
    first = scripted.send({"fixture": "brokered-first"})
    second = scripted.send({"fixture": "brokered-second"})

    def require_reasoning_continuity(request: dict[str, Any]) -> None:
        messages = request.get("messages", [])
        if len(messages) < 4:
            raise PrototypeError("brokered fake second turn lacks tool history")
        if messages[2].get("reasoning_content") != "mock-private-plan":
            raise PrototypeError("brokered fake second turn lost reasoning continuity")

    factory = InProcessFakeDeepSeekConnectionFactory(
        [
            FakeHttpsExchange(status=503),
            FakeHttpsExchange(status=first.status, lines=first.lines),
            FakeHttpsExchange(
                status=second.status,
                lines=second.lines,
                request_validator=require_reasoning_continuity,
            ),
        ]
    )
    broker = permit_broker or ScriptedPermitBroker(
        [
            ScriptedPermitDecision(allow=True, expected_turn=1, expected_attempt=1),
            ScriptedPermitDecision(allow=True, expected_turn=1, expected_attempt=2),
            ScriptedPermitDecision(allow=True, expected_turn=2, expected_attempt=1),
        ]
    )
    transport = BrokeredDeepSeekHttpsTransport(
        permit_broker=broker,
        credential_provider=InMemoryCredentialProvider(b"sk-fake-provider-only"),
        connection_factory=factory,
    )
    control = TransportControl(
        connect_seconds=min(profile["timeouts"]["connect_seconds"], 5),
        first_semantic_seconds=min(
            profile["timeouts"]["first_inference_seconds"], 30
        ),
        total_seconds=min(profile["timeouts"]["total_seconds"], 60),
    )
    result = execute_deepseek_mock_loop(
        export_root=export_root,
        output_dir=output_dir,
        profile=profile,
        transport=transport,
        transport_control=control,
        persist_private_transcript=os.name == "nt",
        network_approval_ledger_path=network_approval_ledger_path,
    )
    summaries = broker.summaries
    records = factory.records
    if (
        getattr(broker, "remaining_decision_count", 0) != 0
        or factory.remaining_exchange_count != 0
    ):
        raise PrototypeError("brokered fake sequence was not fully consumed")
    if len(summaries) != 3 or len(records) != 3:
        raise PrototypeError("brokered fake smoke did not perform three attempts")
    if summaries[0]["request_body_sha256"] != summaries[1]["request_body_sha256"]:
        raise PrototypeError("brokered retry changed the frozen request body")
    if summaries[1]["previous_status"] != 503 or not summaries[1]["is_retry"]:
        raise PrototypeError("brokered retry summary lost previous HTTP status")
    if summaries[2]["private_reasoning_message_count"] != 1:
        raise PrototypeError("brokered second-turn disclosure missed private reasoning")
    if any(record["authorization_recorded"] for record in records):
        raise PrototypeError("fake provider recorded an authorization value")
    return result


def create_deepseek_interactive_fake_https_smoke(
    *, export_root: Path, profile_path: Path, output_dir: Path
) -> dict[str, Any]:
    ledger = ApprovalLedger(output_dir / "network-approvals.jsonl")
    broker = InteractivePermitBroker(
        ledger=ledger,
        confirmation_io=ConsoleConfirmationIO(),
    )
    return create_deepseek_brokered_fake_https_smoke(
        export_root=export_root,
        profile_path=profile_path,
        output_dir=output_dir,
        permit_broker=broker,
        network_approval_ledger_path=ledger.path,
    )


def verify_model_loop(*, output_dir: Path) -> dict[str, Any]:
    errors: list[str] = []
    result: dict[str, Any] | None = None
    replay: dict[str, Any] | None = None
    events: list[dict[str, Any]] = []
    paths = {
        "result": output_dir / "model-loop-result.json",
        "manifest": output_dir / "run-manifest.json",
        "journal": output_dir / "events.jsonl",
        "profile": output_dir / "deepseek-profile.json",
        "private": output_dir / "provider-private-transcript.dpapi",
        "approval": output_dir / "network-approvals.jsonl",
    }
    try:
        result = load_json(paths["result"])
        validate_instance(
            result,
            RUNTIME_ROOT / "model-loop-result-v0.1.schema.json",
            label="model-loop result",
        )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        manifest = load_json(paths["manifest"])
        validate_instance(
            manifest,
            RUNTIME_ROOT / "run-manifest-v0.1.schema.json",
            label="model-loop run manifest",
        )
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        profile = load_json(paths["profile"])
        profile_report = validate_deepseek_profile(profile)
        if not profile_report["valid"]:
            errors.append("persisted DeepSeek profile is invalid")
    except PrototypeError as exc:
        errors.append(str(exc))
    try:
        replay = replay_journal(
            run_manifest_path=paths["manifest"], journal_path=paths["journal"]
        )
        errors.extend(f"journal: {item}" for item in replay["errors"])
        with paths["journal"].open("r", encoding="utf-8") as handle:
            events = [json.loads(line) for line in handle if line.strip()]
    except (OSError, json.JSONDecodeError, PrototypeError) as exc:
        errors.append(f"cannot replay or inspect model-loop journal: {exc}")

    if result is not None:
        for name, record in result["artifacts"].items():
            if record is None:
                continue
            try:
                relative = safe_relative_path(record["path"])
                candidate = output_dir.joinpath(*relative.parts)
                if sha256_file(candidate) != record["sha256"]:
                    errors.append(f"artifact digest mismatch: {name}")
            except (OSError, PrototypeError) as exc:
                errors.append(f"artifact verification failed for {name}: {exc}")
        request_events = [event for event in events if event.get("event_type") == "model_request"]
        output_events = [event for event in events if event.get("event_type") == "model_output"]
        if len(request_events) != result["http_attempt_count"]:
            errors.append("journal model_request count differs from result")
        if len(output_events) != result["http_attempt_count"]:
            errors.append("journal model_output count differs from result")
        if len(result["turns"]) != result["request_count"]:
            errors.append("turn count differs from request_count")
        if not result["reasoning_continuity_checked"]:
            errors.append("reasoning continuity was not established")
        if result["private_transcript_persisted"]:
            if os.name != "nt":
                errors.append("DPAPI transcript is present on a non-Windows verifier")
            else:
                try:
                    private = load_private_transcript(
                        paths["private"], expected_run_id=result["run_id"]
                    )
                    if private["message_count"] != 5:
                        errors.append("private transcript message count is not five")
                except PrototypeError as exc:
                    errors.append(f"private transcript verification failed: {exc}")
        elif result["artifacts"]["private_transcript"] is not None:
            errors.append("private transcript artifact exists but persistence is false")
        approval_artifact = result["artifacts"]["network_approval_ledger"]
        if approval_artifact is not None:
            approval_report = verify_approval_ledger(paths["approval"])
            errors.extend(
                f"network approval ledger: {item}"
                for item in approval_report["errors"]
            )
            attempts = [
                (turn["turn"], turn["request_sha256"], attempt)
                for turn in result["turns"]
                for attempt in turn["attempts"]
            ]
            if approval_report["event_count"] != len(attempts):
                errors.append("network approval event count differs from attempts")
            if approval_report["allow_count"] != len(attempts):
                errors.append("network approval ledger contains a non-allow decision")
            for event, attempt_entry in zip(approval_report["events"], attempts):
                expected_turn, expected_request_sha256, attempt = attempt_entry
                metadata = attempt["transport_metadata"]
                summary = metadata.get("confirmation_summary")
                if not isinstance(summary, dict):
                    errors.append("approved attempt lacks confirmation summary")
                    continue
                try:
                    recomputed_summary_sha256 = confirmation_summary_sha256(summary)
                except PrototypeError as exc:
                    errors.append(f"invalid persisted confirmation summary: {exc}")
                    continue
                if summary.get("turn") != expected_turn:
                    errors.append("confirmation summary turn differs from result turn")
                if summary.get("attempt") != attempt["attempt"]:
                    errors.append("confirmation summary attempt differs from result attempt")
                if summary.get("request_body_sha256") != expected_request_sha256:
                    errors.append("confirmation summary request digest differs from turn")
                if event["turn"] != summary.get("turn"):
                    errors.append("approval turn differs from confirmation summary")
                if event["attempt"] != summary.get("attempt"):
                    errors.append("approval attempt differs from confirmation summary")
                if event["request_body_sha256"] != summary.get("request_body_sha256"):
                    errors.append("approval request digest differs from confirmation summary")
                if metadata.get("confirmation_summary_sha256") != recomputed_summary_sha256:
                    errors.append("transport confirmation summary digest is not reproducible")
                if event["confirmation_summary_sha256"] != recomputed_summary_sha256:
                    errors.append("approval summary digest differs from transport metadata")
            terminal_events = [
                event for event in events if event.get("event_type") == "run_finished"
            ]
            if len(terminal_events) == 1:
                anchor = terminal_events[0].get("payload", {}).get(
                    "network_approval_ledger"
                )
                if not isinstance(anchor, dict):
                    errors.append("run_finished lacks network approval ledger anchor")
                else:
                    if anchor.get("sha256") != approval_artifact["sha256"]:
                        errors.append("run_finished approval ledger digest differs from artifact")
                    if anchor.get("event_count") != approval_report["event_count"]:
                        errors.append("run_finished approval event count differs from ledger")
                    if (
                        anchor.get("terminal_event_sha256")
                        != approval_report["terminal_event_sha256"]
                    ):
                        errors.append("run_finished approval terminal digest differs from ledger")
        elif paths["approval"].exists():
            errors.append("unregistered network approval ledger exists")
    if result is not None and replay is not None:
        if replay["terminal_event"] != result["journal_replay"]["terminal_event"]:
            errors.append("journal terminal event differs from result")
        if replay["event_count"] != result["journal_replay"]["event_count"]:
            errors.append("journal event count differs from result")

    for path in output_dir.iterdir() if output_dir.is_dir() else []:
        if path.is_file() and path.suffix != ".dpapi":
            try:
                if b"reasoning_content" in path.read_bytes():
                    errors.append(f"provider-private reasoning field leaked into {path.name}")
            except OSError as exc:
                errors.append(f"cannot scan output for private reasoning: {exc}")

    return {
        "schema_version": "0.1.0-prototype",
        "valid": not errors,
        "output_dir": str(output_dir),
        "errors": sorted(set(errors)),
        "checked_at": utc_now(),
        "limitations": [
            "Verification covers persisted mock mechanics and cannot test a real DeepSeek service."
        ],
    }
