from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import time
from typing import Any, Iterable

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
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
UPSTREAM_LOCK = ROOT / "upstream" / "grok-build.lock.json"
TERMINAL_TOOL_STATUSES = {"completed", "failed"}
ALLOWED_EXTENSION_NOTIFICATIONS = {
    "_x.ai/mcp/servers_updated",
    "_x.ai/mcp_initialized",
    "_x.ai/queue/changed",
    "_x.ai/session/prompt_complete",
    "_x.ai/session_notification",
    "_x.ai/sessions/changed",
}
ALLOWED_SESSION_NOTIFICATION_UPDATES = {
    "interaction_resolved",
    "pending_interaction",
    "session_summary_generated",
    "tool_call_delta_chunk",
    "turn_completed",
}


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), start=1
    ):
        if not line.strip():
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{line_number} is not a JSON object")
        rows.append(value)
    return rows


def _atomic_write_json(path: Path, value: Any) -> None:
    if path.exists():
        raise ValueError(f"refusing to overwrite verification output: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _artifact_matches(record: dict[str, Any]) -> bool:
    path = Path(record["path"])
    if not path.is_file():
        return False
    payload = path.read_bytes()
    return len(payload) == record["bytes"] and _sha256_bytes(payload) == record["sha256"]


def _response_for_id(
    messages: Iterable[dict[str, Any]], request_id: str
) -> list[dict[str, Any]]:
    return [
        row["message"]
        for row in messages
        if row.get("direction") == "agent_to_client"
        and isinstance(row.get("message"), dict)
        and str(row["message"].get("id")) == request_id
        and "method" not in row["message"]
    ]


def _is_allowed_extension_notification(
    message: dict[str, Any], session_id: str
) -> bool:
    method = message.get("method")
    params = message.get("params")
    if (
        method not in ALLOWED_EXTENSION_NOTIFICATIONS
        or message.get("jsonrpc") != "2.0"
        or "id" in message
        or not isinstance(params, dict)
    ):
        return False
    if method == "_x.ai/mcp/servers_updated":
        return set(params) == {"mcpServers"} and params.get("mcpServers") == []
    if method == "_x.ai/mcp_initialized":
        return (
            set(params) == {"elapsedMs", "mcpToolCount", "sessionId"}
            and type(params.get("elapsedMs")) is int
            and params["elapsedMs"] >= 0
            and params.get("mcpToolCount") == 0
            and params.get("sessionId") == session_id
        )
    if method == "_x.ai/queue/changed":
        entries = params.get("entries")
        valid_entries = isinstance(entries, list) and all(
            isinstance(entry, dict)
            and set(entry) == {"id", "kind", "position", "text", "version"}
            and isinstance(entry.get("id"), str)
            and entry.get("kind") == "prompt"
            and type(entry.get("position")) is int
            and isinstance(entry.get("text"), str)
            and type(entry.get("version")) is int
            for entry in entries
        )
        keys_valid = set(params).issubset(
            {
                "entries",
                "runningKind",
                "runningPromptId",
                "runningText",
                "sessionId",
            }
        )
        running_metadata_valid = (
            "runningKind" not in params and "runningText" not in params
        ) or (
            params.get("runningKind") == "prompt"
            and isinstance(params.get("runningText"), str)
            and isinstance(params.get("runningPromptId"), str)
        )
        return (
            keys_valid
            and {"entries", "sessionId"}.issubset(params)
            and valid_entries
            and params.get("sessionId") == session_id
            and running_metadata_valid
            and (
                "runningPromptId" not in params
                or isinstance(params.get("runningPromptId"), str)
            )
        )
    if method == "_x.ai/session/prompt_complete":
        return (
            set(params) == {"agentResult", "promptId", "sessionId", "stopReason"}
            and params.get("agentResult") is None
            and isinstance(params.get("promptId"), str)
            and params.get("sessionId") == session_id
            and params.get("stopReason") in {"end_turn", "cancelled"}
        )
    if method == "_x.ai/session_notification":
        update = params.get("update")
        if not (
            set(params).issubset({"_meta", "sessionId", "update"})
            and {"sessionId", "update"}.issubset(params)
            and params.get("sessionId") == session_id
            and isinstance(update, dict)
            and update.get("sessionUpdate") in ALLOWED_SESSION_NOTIFICATION_UPDATES
            and ("_meta" not in params or isinstance(params.get("_meta"), dict))
        ):
            return False
        update_type = update["sessionUpdate"]
        if update_type == "tool_call_delta_chunk":
            return (
                set(update)
                == {
                    "arguments_delta",
                    "name",
                    "sessionUpdate",
                    "tool_call_id",
                    "tool_index",
                }
                and isinstance(update.get("arguments_delta"), str)
                and update.get("name") == "read_file"
                and isinstance(update.get("tool_call_id"), str)
                and type(update.get("tool_index")) is int
            )
        if update_type == "pending_interaction":
            return (
                set(update) == {"kind", "sessionUpdate", "tool_call_id"}
                and update.get("kind") == "permission"
                and isinstance(update.get("tool_call_id"), str)
            )
        if update_type == "interaction_resolved":
            return (
                set(update) == {"sessionUpdate", "tool_call_id"}
                and isinstance(update.get("tool_call_id"), str)
            )
        if update_type == "session_summary_generated":
            return (
                set(update) == {"sessionUpdate", "session_summary"}
                and isinstance(update.get("session_summary"), str)
            )
        return (
            set(update) == {"prompt_id", "sessionUpdate", "stop_reason", "usage"}
            and isinstance(update.get("prompt_id"), str)
            and update.get("stop_reason") in {"end_turn", "cancelled"}
            and isinstance(update.get("usage"), dict)
        )
    if method == "_x.ai/sessions/changed":
        upserted = params.get("upserted")
        return (
            set(params) == {"removed", "upserted"}
            and params.get("removed") == []
            and isinstance(upserted, list)
            and len(upserted) == 1
            and isinstance(upserted[0], dict)
            and set(upserted[0])
            == {
                "activity",
                "cwd",
                "isWorktree",
                "lastChangeUnixMs",
                "modelId",
                "origin",
                "resident",
                "sessionId",
                "title",
                "yolo",
            }
            and upserted[0].get("activity") in {"working", "idle"}
            and upserted[0].get("sessionId") == session_id
        )
    return False


def project_transcript(rows: list[dict[str, Any]]) -> dict[str, Any]:
    for expected_sequence, row in enumerate(rows, start=1):
        if row.get("sequence") != expected_sequence:
            raise ValueError("ACP transcript sequence is not contiguous")
        message = row.get("message")
        if not isinstance(message, dict):
            raise ValueError("ACP transcript row is missing an object message")
        encoded = json.dumps(
            message,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
        if row.get("message_bytes") != len(encoded):
            raise ValueError("ACP transcript message_bytes mismatch")
        if row.get("message_sha256") != _sha256_bytes(encoded):
            raise ValueError("ACP transcript message_sha256 mismatch")
    client_messages = [
        row["message"]
        for row in rows
        if row.get("direction") == "client_to_agent"
        and isinstance(row.get("message"), dict)
    ]
    agent_messages = [
        row["message"]
        for row in rows
        if row.get("direction") == "agent_to_client"
        and isinstance(row.get("message"), dict)
    ]
    request_methods = [
        message["method"]
        for message in client_messages
        if "id" in message
        and message.get("method") in {"initialize", "session/new", "session/prompt"}
    ]
    session_responses = _response_for_id(rows, "acp-session-1")
    session_id = ""
    if len(session_responses) == 1:
        result = session_responses[0].get("result")
        if isinstance(result, dict) and isinstance(result.get("sessionId"), str):
            session_id = result["sessionId"]

    updates: list[dict[str, Any]] = []
    permission_requests: list[dict[str, Any]] = []
    extension_counts = {
        method: 0 for method in sorted(ALLOWED_EXTENSION_NOTIFICATIONS)
    }
    unexpected = 0
    for message in agent_messages:
        method = message.get("method")
        if method == "session/update":
            params = message.get("params")
            update = params.get("update") if isinstance(params, dict) else None
            if isinstance(update, dict):
                updates.append(update)
            else:
                unexpected += 1
        elif method == "session/request_permission" and "id" in message:
            permission_requests.append(message)
        elif _is_allowed_extension_notification(message, session_id):
            extension_counts[str(method)] += 1
            continue
        elif "method" in message:
            unexpected += 1

    tool_calls = [
        update for update in updates if update.get("sessionUpdate") == "tool_call"
    ]
    tool_updates = [
        update
        for update in updates
        if update.get("sessionUpdate") == "tool_call_update"
    ]
    tool_ids = [
        update.get("toolCallId")
        for update in tool_calls + tool_updates
        if isinstance(update.get("toolCallId"), str)
    ]
    unique_tool_ids = list(dict.fromkeys(tool_ids))
    tool_statuses = [
        str(update["status"]).lower()
        for update in tool_updates
        if isinstance(update.get("status"), str)
    ]
    terminal_tool_update_count = sum(
        status in TERMINAL_TOOL_STATUSES for status in tool_statuses
    )

    option_kinds: list[str] = []
    permission_outcome = ""
    if len(permission_requests) == 1:
        params = permission_requests[0].get("params")
        options = params.get("options", []) if isinstance(params, dict) else []
        option_kinds = sorted(
            {
                option["kind"]
                for option in options
                if isinstance(option, dict)
                and option.get("kind")
                in {"allow_once", "allow_always", "reject_once", "reject_always"}
            }
        )
        permission_id = str(permission_requests[0].get("id"))
        responses = [
            message
            for message in client_messages
            if str(message.get("id")) == permission_id and "method" not in message
        ]
        if len(responses) == 1:
            result = responses[0].get("result")
            outcome = result.get("outcome") if isinstance(result, dict) else None
            if isinstance(outcome, dict):
                if outcome.get("outcome") == "cancelled":
                    permission_outcome = "cancelled"
                elif outcome.get("outcome") == "selected":
                    selected = outcome.get("optionId")
                    for option in options:
                        if (
                            isinstance(option, dict)
                            and option.get("optionId") == selected
                            and option.get("kind") == "allow_once"
                        ):
                            permission_outcome = "allow_once"
                            break

    prompt_responses = _response_for_id(rows, "acp-prompt-1")
    prompt_stop_reason = ""
    if len(prompt_responses) == 1:
        result = prompt_responses[0].get("result")
        if isinstance(result, dict) and isinstance(result.get("stopReason"), str):
            prompt_stop_reason = result["stopReason"]

    cancel_sent = any(
        message.get("method") == "session/cancel" and "id" not in message
        for message in client_messages
    )
    return {
        "protocol_version": 1,
        "session_id": session_id,
        "request_methods": request_methods,
        "update_types": [
            str(update["sessionUpdate"])
            for update in updates
            if isinstance(update.get("sessionUpdate"), str)
        ],
        "tool_call_id": unique_tool_ids[0] if len(unique_tool_ids) == 1 else "",
        "tool_call_count": len(tool_calls),
        "tool_update_count": len(tool_updates),
        "terminal_tool_update_count": terminal_tool_update_count,
        "tool_statuses": tool_statuses,
        "permission_request_count": len(permission_requests),
        "permission_option_kinds": option_kinds,
        "permission_outcome": permission_outcome,
        "cancel_notification_sent": cancel_sent,
        "prompt_response_count": len(prompt_responses),
        "prompt_stop_reason": prompt_stop_reason,
        "extension_notification_counts": extension_counts,
        "unexpected_message_count": unexpected,
        "parse_failure_count": 0,
    }


def project_session_evidence(
    event_rows: list[dict[str, Any]], update_rows: list[dict[str, Any]]
) -> dict[str, Any]:
    event_types = [
        str(row["type"]) for row in event_rows if isinstance(row.get("type"), str)
    ]
    update_values: list[dict[str, Any]] = []
    for row in update_rows:
        params = row.get("params")
        update = params.get("update") if isinstance(params, dict) else None
        if isinstance(update, dict):
            update_values.append(update)
    update_types = [
        str(update["sessionUpdate"])
        for update in update_values
        if isinstance(update.get("sessionUpdate"), str)
    ]
    tool_call_ids = sorted(
        {
            update["toolCallId"]
            for update in update_values
            if isinstance(update.get("toolCallId"), str)
        }
    )
    permission_decisions = [
        str(row["decision"])
        for row in event_rows
        if row.get("type") == "permission_resolved"
        and isinstance(row.get("decision"), str)
    ]
    return {
        "discovered": True,
        "event_types": event_types,
        "update_types": update_types,
        "tool_call_ids": tool_call_ids,
        "permission_decisions": permission_decisions,
        "completed_tool_event_count": event_types.count("tool_completed"),
    }


def project_provider(
    provider_result: dict[str, Any], private_rows: list[dict[str, Any]]
) -> dict[str, Any]:
    primary = [
        row
        for row in private_rows
        if isinstance(row.get("body"), dict)
        and row["body"].get("model") == "deepseek-v4-pro"
    ]
    auxiliary = [row for row in private_rows if row not in primary]
    auxiliary_valid = len(auxiliary) == 1
    if auxiliary_valid:
        row = auxiliary[0]
        body = row.get("body")
        headers = row.get("headers")
        tools = body.get("tools") if isinstance(body, dict) else None
        tool_choice = body.get("tool_choice") if isinstance(body, dict) else None
        tool = tools[0] if isinstance(tools, list) and len(tools) == 1 else None
        tool_function = tool.get("function") if isinstance(tool, dict) else None
        choice_function = (
            tool_choice.get("function") if isinstance(tool_choice, dict) else None
        )
        auxiliary_valid = (
            row.get("method") == "POST"
            and row.get("path") == "/chat/completions"
            and row.get("client_ip") == "127.0.0.1"
            and isinstance(body, dict)
            and body.get("model") == "grok-4.5"
            and body.get("stream") is True
            and isinstance(tools, list)
            and len(tools) == 1
            and isinstance(tool_function, dict)
            and tool_function.get("name") == "session_title"
            and isinstance(choice_function, dict)
            and choice_function.get("name") == "session_title"
            and isinstance(headers, dict)
            and headers.get("authorization", {}).get("present") is True
            and headers.get("authorization", {}).get("value_recorded") is False
        )
    scenario = provider_result.get("scenario")
    continuity = provider_result.get("continuity")
    continuity = continuity if isinstance(continuity, dict) else {}
    is_allow = scenario == "tool-continuity"
    return {
        "scenario": scenario,
        "terminal_state": provider_result.get("terminal_state"),
        "primary_request_count": len(primary),
        "auxiliary_request_count": len(auxiliary),
        "auxiliary_request_validated": auxiliary_valid,
        "second_request_observed": len(primary) > 1,
        "reasoning_marker_preserved": bool(
            continuity.get("reasoning_marker_preserved")
        )
        if is_allow
        else False,
        "tool_call_id_preserved": bool(continuity.get("tool_call_id_preserved"))
        if is_allow
        else False,
        "tool_result_observed": bool(continuity.get("tool_result_observed"))
        if is_allow
        else False,
        "tool_result_marker_observed": bool(
            continuity.get("tool_result_marker_observed")
        )
        if is_allow
        else False,
        "real_model_invoked": bool(
            provider_result.get("response", {}).get("real_model_invoked", True)
        ),
    }


def _receipt_projection(receipt: dict[str, Any]) -> tuple[Any, ...]:
    workspace = receipt.get("workspace", {})
    discovery = receipt.get("discovery", {})
    decision = receipt.get("decision", {})
    return (
        receipt.get("valid"),
        workspace.get("canonical_path"),
        discovery.get("complete"),
        discovery.get("candidate_count"),
        discovery.get("aggregate_sha256"),
        discovery.get("scan_policy_sha256"),
        decision.get("mode"),
        decision.get("launch_permitted"),
    )


def _scenario_matches(result: dict[str, Any]) -> bool:
    scenario = result["scenario"]
    acp = result["acp"]
    session = result["session_evidence"]
    provider = result["provider"]
    common = (
        acp["request_methods"] == ["initialize", "session/new", "session/prompt"]
        and acp["tool_call_count"] == 1
        and acp["permission_request_count"] == 1
        and acp["prompt_response_count"] == 1
        and acp["unexpected_message_count"] == 0
        and acp["parse_failure_count"] == 0
        and acp["tool_call_id"] in session["tool_call_ids"]
        and provider["terminal_state"] == "succeeded"
        and provider["auxiliary_request_count"] == 1
        and provider["auxiliary_request_validated"]
        and not provider["real_model_invoked"]
    )
    if scenario == "allow_once":
        return common and all(
            [
                acp["permission_outcome"] == "allow_once",
                not acp["cancel_notification_sent"],
                acp["prompt_stop_reason"] == "end_turn",
                acp["terminal_tool_update_count"] == 1,
                "completed" in acp["tool_statuses"],
                provider["scenario"] == "tool-continuity",
                provider["primary_request_count"] == 2,
                provider["second_request_observed"],
                provider["reasoning_marker_preserved"],
                provider["tool_call_id_preserved"],
                provider["tool_result_observed"],
                provider["tool_result_marker_observed"],
                session["completed_tool_event_count"] == 1,
                session["permission_decisions"] == ["allow"],
            ]
        )
    return common and all(
        [
            acp["permission_outcome"] == "cancelled",
            acp["cancel_notification_sent"],
            acp["prompt_stop_reason"] == "cancelled",
            acp["terminal_tool_update_count"] in {0, 1},
            set(acp["tool_statuses"]).issubset({"failed"}),
            provider["scenario"] == "tool-cancel",
            provider["primary_request_count"] == 1,
            not provider["second_request_observed"],
            not provider["tool_result_observed"],
            session["completed_tool_event_count"] == 0,
            session["permission_decisions"] == ["cancelled"],
        ]
    )


def verify(result_path: Path, *, lock_path: Path = UPSTREAM_LOCK) -> dict[str, Any]:
    errors: list[str] = []
    checks = {
        "result_schema_valid": False,
        "result_valid_flag": False,
        "binary_lock_matches": False,
        "artifact_digests_match": False,
        "transcript_projection_matches": False,
        "provider_capture_matches": False,
        "session_evidence_matches": False,
        "workspace_receipts_match": False,
        "scenario_semantics_match": False,
        "safety_checks_all_true": False,
    }
    result_bytes = result_path.read_bytes()
    result = json.loads(result_bytes)
    result_schema = _read_json(RESULT_SCHEMA)
    schema_errors = sorted(
        Draft202012Validator(
            result_schema, format_checker=FormatChecker()
        ).iter_errors(result),
        key=lambda error: list(error.absolute_path),
    )
    if schema_errors:
        errors.extend(
            f"result schema: {'/'.join(map(str, error.absolute_path))}: {error.message}"
            for error in schema_errors
        )
    else:
        checks["result_schema_valid"] = True
    checks["result_valid_flag"] = result.get("valid") is True
    if not checks["result_valid_flag"]:
        errors.append("result valid flag is not true")

    try:
        lock = _read_json(lock_path)
        locked_release = lock["binary_release"]
        checks["binary_lock_matches"] = (
            result["binary"]["sha256"] == locked_release["sha256"]
            and result["binary"]["locked_sha256"] == locked_release["sha256"]
            and result["binary"]["bytes"] == locked_release["bytes"]
            and result["binary"]["version"] == locked_release["version"]
            and result["binary"]["authenticode_status"]
            == locked_release["authenticode_status"]
        )
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        errors.append(f"binary lock verification failed: {error}")
    if not checks["binary_lock_matches"]:
        errors.append("binary projection does not match upstream lock")

    try:
        artifacts = result["artifacts"]
        checks["artifact_digests_match"] = all(
            _artifact_matches(record) for record in artifacts.values()
        ) and _artifact_matches(result["binary"])
    except (KeyError, OSError, TypeError, ValueError) as error:
        errors.append(f"artifact verification failed: {error}")
    if not checks["artifact_digests_match"]:
        errors.append("one or more artifact digests do not match")

    try:
        transcript_rows = _read_jsonl(Path(result["artifacts"]["transcript"]["path"]))
        checks["transcript_projection_matches"] = (
            project_transcript(transcript_rows) == result["acp"]
        )
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        errors.append(f"transcript projection failed: {error}")
    if not checks["transcript_projection_matches"]:
        errors.append("ACP transcript projection does not match result")

    try:
        provider_result = _read_json(
            Path(result["artifacts"]["provider_result"]["path"])
        )
        provider_rows = _read_jsonl(
            Path(result["artifacts"]["provider_private_capture"]["path"])
        )
        checks["provider_capture_matches"] = (
            project_provider(provider_result, provider_rows) == result["provider"]
        )
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        errors.append(f"provider projection failed: {error}")
    if not checks["provider_capture_matches"]:
        errors.append("provider capture projection does not match result")

    try:
        event_rows = _read_jsonl(Path(result["artifacts"]["session_events"]["path"]))
        update_rows = _read_jsonl(
            Path(result["artifacts"]["session_updates"]["path"])
        )
        checks["session_evidence_matches"] = (
            project_session_evidence(event_rows, update_rows)
            == result["session_evidence"]
        )
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        errors.append(f"session projection failed: {error}")
    if not checks["session_evidence_matches"]:
        errors.append("session evidence projection does not match result")

    try:
        receipts = [
            _read_json(Path(result["artifacts"][name]["path"]))
            for name in (
                "workspace_trust_preflight",
                "workspace_trust_launch",
                "workspace_trust_postrun",
            )
        ]
        projections = [_receipt_projection(receipt) for receipt in receipts]
        checks["workspace_receipts_match"] = (
            len(set(projections)) == 1
            and projections[0][0] is True
            and projections[0][2] is True
            and projections[0][3] == 0
            and projections[0][6] == "restricted"
            and projections[0][7] is True
        )
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        errors.append(f"workspace receipt verification failed: {error}")
    if not checks["workspace_receipts_match"]:
        errors.append("workspace trust receipts do not preserve one restricted state")

    if checks["result_schema_valid"]:
        checks["scenario_semantics_match"] = (
            checks["transcript_projection_matches"]
            and checks["provider_capture_matches"]
            and checks["session_evidence_matches"]
            and _scenario_matches(result)
        )
        checks["safety_checks_all_true"] = (
            all(value is True for value in result["checks"].values())
            and result["process"]["exit_code"] == 0
            and not result["process"]["timed_out"]
            and result["process"]["job_object_created"]
            and result["process"]["job_object_assigned"]
            and result["process"]["job_object_closed"]
            and result["controls"]["loopback_only_provider"]
            and result["controls"]["clean_environment"]
            and not result["controls"]["credential_value_recorded"]
            and result["controls"]["outbound_block_created"]
            and result["controls"]["outbound_rules_removed"]
            and result["controls"]["remaining_rule_count"] == 0
            and result["controls"]["workspace_disposable"]
            and not result["controls"]["workspace_modified"]
            and not result["controls"]["output_overwritten"]
        )
    if not checks["scenario_semantics_match"]:
        errors.append("scenario-specific ACP semantics do not match")
    if not checks["safety_checks_all_true"]:
        errors.append("result safety checks are not all true")

    verification = {
        "schema_version": "0.1.0",
        "verification_kind": "grok-acp-fake-tool-probe-verification",
        "valid": all(checks.values()) and not errors,
        "probe_id": result.get("probe_id", "ACPTOOL-" + "0" * 32),
        "result_sha256": _sha256_bytes(result_bytes),
        "checks": checks,
        "errors": errors,
    }
    verification_schema = _read_json(VERIFICATION_SCHEMA)
    Draft202012Validator(verification_schema).validate(verification)
    return verification


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Independently verify a Grok ACP fake-tool probe result"
    )
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument(
        "--lock",
        type=Path,
        default=UPSTREAM_LOCK,
        help="Release metadata containing the binary_release identity",
    )
    args = parser.parse_args()
    verification = verify(args.result.resolve(), lock_path=args.lock.resolve())
    if args.output:
        _atomic_write_json(args.output.resolve(), verification)
    print(json.dumps(verification, ensure_ascii=False, sort_keys=True))
    return 0 if verification["valid"] else 1


if __name__ == "__main__":
    sys.exit(main())
