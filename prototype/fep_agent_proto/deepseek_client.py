from __future__ import annotations

import json
from typing import Any, Iterable

from .deepseek_adapter import (
    classify_sse_line,
    validate_deepseek_profile,
    validate_tool_history,
)
from .errors import PrototypeError
from .io_utils import canonical_bytes, sha256_bytes


SUPPORTED_FINISH_REASONS = {
    "stop",
    "length",
    "content_filter",
    "tool_calls",
    "insufficient_system_resource",
}


def _reject_json_constant(value: str) -> None:
    raise ValueError(f"non-standard JSON constant {value}")


def build_deepseek_request(
    *,
    profile: dict[str, Any],
    messages: list[dict[str, Any]],
    tools: list[dict[str, Any]],
) -> tuple[dict[str, Any], dict[str, Any]]:
    profile_report = validate_deepseek_profile(profile)
    if not profile_report["valid"]:
        raise PrototypeError(f"invalid DeepSeek profile: {profile_report['issues']}")
    if profile["transport"] != "openai_chat_completions":
        raise PrototypeError("model-loop spike supports only OpenAI Chat Completions")
    transcript = validate_tool_history(
        messages,
        thinking_enabled=profile["thinking"]["type"] == "enabled",
    )
    if not transcript["valid"]:
        raise PrototypeError(f"invalid DeepSeek transcript: {transcript['issues']}")
    if bool(tools) != profile["tools"]["enabled"]:
        raise PrototypeError("tool definitions must match the frozen DeepSeek profile")

    request: dict[str, Any] = {
        "model": profile["model"],
        "messages": messages,
        "stream": True,
        "stream_options": {"include_usage": True},
        "thinking": {"type": profile["thinking"]["type"]},
    }
    if profile["thinking"]["reasoning_effort"] is not None:
        request["reasoning_effort"] = profile["thinking"]["reasoning_effort"]
    request.update(profile["sampling"])
    if tools:
        request["tools"] = tools
    structured = profile["structured_output"]
    if structured["mode"] == "json_object":
        request["response_format"] = {"type": "json_object"}
        request["max_tokens"] = structured["max_tokens"]
    if profile["user_id"] is not None:
        request["user_id"] = profile["user_id"]

    roles = [message.get("role", "unknown") for message in messages]
    reasoning_messages = sum(
        1
        for message in messages
        if message.get("role") == "assistant" and message.get("reasoning_content")
    )
    metadata = {
        "request_sha256": sha256_bytes(canonical_bytes(request)),
        "model": request["model"],
        "message_count": len(messages),
        "message_roles": roles,
        "tool_definition_count": len(tools),
        "thinking_enabled": profile["thinking"]["type"] == "enabled",
        "reasoning_message_count": reasoning_messages,
        "reasoning_continuity_valid": transcript["valid"],
        "raw_messages_recorded": False,
    }
    return request, metadata


def decode_strict_json_object(value: str, *, label: str) -> dict[str, Any]:
    try:
        decoded = json.loads(
            value,
            parse_constant=_reject_json_constant,
        )
    except (json.JSONDecodeError, ValueError) as exc:
        raise PrototypeError(f"{label} is not strict JSON: {exc}") from exc
    if not isinstance(decoded, dict):
        raise PrototypeError(f"{label} must be a JSON object")
    return decoded


def consume_deepseek_sse(
    lines: Iterable[str],
) -> tuple[dict[str, Any], dict[str, Any]]:
    content_parts: list[str] = []
    reasoning_parts: list[str] = []
    tool_parts: dict[int, dict[str, Any]] = {}
    keep_alive_count = 0
    data_event_count = 0
    semantic_event_count = 0
    done_seen = False
    finish_reason: str | None = None
    usage: dict[str, Any] | None = None

    for line in lines:
        classified = classify_sse_line(line)
        if classified["kind"] == "separator":
            continue
        if classified["kind"] == "keep_alive":
            keep_alive_count += 1
            continue
        if classified["kind"] == "done":
            done_seen = True
            break
        payload = classified["payload"]
        data_event_count += 1
        if "usage" in payload and payload["usage"] is not None:
            if not isinstance(payload["usage"], dict):
                raise PrototypeError("DeepSeek SSE usage must be an object")
            usage = payload["usage"]
        choices = payload.get("choices")
        if not isinstance(choices, list):
            raise PrototypeError("DeepSeek SSE data event must contain choices array")
        if not choices:
            continue
        if len(choices) != 1 or choices[0].get("index") != 0:
            raise PrototypeError("model-loop spike requires exactly choice index 0")
        choice = choices[0]
        current_finish = choice.get("finish_reason")
        if current_finish is not None:
            if current_finish not in SUPPORTED_FINISH_REASONS:
                raise PrototypeError(f"unsupported DeepSeek finish reason: {current_finish}")
            if finish_reason is not None and finish_reason != current_finish:
                raise PrototypeError("conflicting DeepSeek finish reasons")
            finish_reason = current_finish
        delta = choice.get("delta", {})
        if not isinstance(delta, dict):
            raise PrototypeError("DeepSeek SSE choice delta must be an object")
        content = delta.get("content")
        if content is not None:
            if not isinstance(content, str):
                raise PrototypeError("DeepSeek content delta must be a string")
            content_parts.append(content)
            if content:
                semantic_event_count += 1
        reasoning = delta.get("reasoning_content")
        if reasoning is not None:
            if not isinstance(reasoning, str):
                raise PrototypeError("DeepSeek reasoning delta must be a string")
            reasoning_parts.append(reasoning)
            if reasoning:
                semantic_event_count += 1
        tool_deltas = delta.get("tool_calls", [])
        if not isinstance(tool_deltas, list):
            raise PrototypeError("DeepSeek tool_calls delta must be an array")
        for tool_delta in tool_deltas:
            if not isinstance(tool_delta, dict):
                raise PrototypeError("DeepSeek tool call delta must be an object")
            index = tool_delta.get("index")
            if not isinstance(index, int) or index < 0:
                raise PrototypeError("DeepSeek tool call delta requires non-negative index")
            assembled = tool_parts.setdefault(
                index,
                {"id": None, "type": "function", "name": "", "arguments": ""},
            )
            if "id" in tool_delta and tool_delta["id"] is not None:
                if assembled["id"] not in {None, tool_delta["id"]}:
                    raise PrototypeError("conflicting DeepSeek tool call IDs")
                assembled["id"] = tool_delta["id"]
            if tool_delta.get("type") not in {None, "function"}:
                raise PrototypeError("only function tool calls are supported")
            function = tool_delta.get("function", {})
            if not isinstance(function, dict):
                raise PrototypeError("DeepSeek tool function delta must be an object")
            name = function.get("name")
            arguments = function.get("arguments")
            if name is not None:
                if not isinstance(name, str):
                    raise PrototypeError("DeepSeek tool name delta must be a string")
                assembled["name"] += name
            if arguments is not None:
                if not isinstance(arguments, str):
                    raise PrototypeError("DeepSeek tool arguments delta must be a string")
                assembled["arguments"] += arguments
            semantic_event_count += 1

    if not done_seen:
        raise PrototypeError("DeepSeek SSE stream ended without data: [DONE]")
    if finish_reason is None:
        raise PrototypeError("DeepSeek SSE stream ended without finish_reason")

    tool_calls: list[dict[str, Any]] = []
    public_tools: list[dict[str, Any]] = []
    for expected_index, index in enumerate(sorted(tool_parts)):
        if index != expected_index:
            raise PrototypeError("DeepSeek tool call indexes are not contiguous")
        assembled = tool_parts[index]
        if not isinstance(assembled["id"], str) or not assembled["id"]:
            raise PrototypeError("assembled DeepSeek tool call is missing ID")
        if not assembled["name"]:
            raise PrototypeError("assembled DeepSeek tool call is missing function name")
        decode_strict_json_object(assembled["arguments"], label="tool arguments")
        tool_calls.append(
            {
                "id": assembled["id"],
                "type": "function",
                "function": {
                    "name": assembled["name"],
                    "arguments": assembled["arguments"],
                },
            }
        )
        public_tools.append(
            {
                "id_sha256": sha256_bytes(assembled["id"].encode("utf-8")),
                "name": assembled["name"],
                "arguments_sha256": sha256_bytes(
                    assembled["arguments"].encode("utf-8")
                ),
            }
        )

    content = "".join(content_parts)
    reasoning_content = "".join(reasoning_parts)
    if finish_reason == "tool_calls" and not tool_calls:
        raise PrototypeError("tool_calls finish reason lacks assembled tool calls")
    if finish_reason != "tool_calls" and tool_calls:
        raise PrototypeError("assembled tool calls require tool_calls finish reason")
    if finish_reason == "stop" and not content:
        raise PrototypeError("final DeepSeek stop response contains empty content")

    message: dict[str, Any] = {"role": "assistant", "content": content}
    if reasoning_content:
        message["reasoning_content"] = reasoning_content
    if tool_calls:
        message["tool_calls"] = tool_calls
    public = {
        "finish_reason": finish_reason,
        "content_bytes": len(content.encode("utf-8")),
        "content_sha256": sha256_bytes(content.encode("utf-8")),
        "reasoning_present": bool(reasoning_content),
        "reasoning_bytes": len(reasoning_content.encode("utf-8")),
        "reasoning_sha256": (
            sha256_bytes(reasoning_content.encode("utf-8"))
            if reasoning_content
            else None
        ),
        "tool_call_count": len(tool_calls),
        "tool_calls": public_tools,
        "keep_alive_count": keep_alive_count,
        "data_event_count": data_event_count,
        "semantic_event_count": semantic_event_count,
        "usage": usage,
        "raw_content_recorded": False,
        "raw_reasoning_recorded": False,
    }
    return message, public


def decode_tool_arguments(tool_call: dict[str, Any]) -> dict[str, Any]:
    return decode_strict_json_object(
        tool_call["function"]["arguments"], label="tool arguments"
    )
