from __future__ import annotations

import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import re
import time
from typing import Any


HOST = "127.0.0.1"
RESPONSE_MARKER = "LIF_FAKE_PROVIDER_OK"
TOOL_RESPONSE_MARKER = "LIF_FAKE_TOOL_CONTINUITY_OK"
REASONING_MARKER = "LIF_FAKE_REASONING_CONTINUITY_001"
TOOL_RESULT_MARKER = "LIF_TOOL_FIXTURE_CONTENT_001"
TOOL_CALL_ID = "call_lif_read_fixture_001"
PROCESS_TREE_TOOL_CALL_ID = "call_lif_process_tree_001"
PROCESS_TREE_KILL_CALL_ID = "call_lif_process_tree_kill_001"
PROCESS_TREE_OUTPUT_CALL_ID = "call_lif_process_tree_output_001"
PROCESS_TREE_PRE_TRIGGER_MARKER = "driver-pre-trigger-ready.json"
MAX_BODY_BYTES = 1024 * 1024


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


class CaptureState:
    def __init__(
        self,
        output_directory: Path,
        scenario: str,
        tool_fixture_path: Path | None,
        terminal_command: str | None,
        tool_timeout_ms: int,
        timeout_seconds: float,
        process_record_directory: Path | None,
    ) -> None:
        self.output_directory = output_directory
        self.scenario = scenario
        self.tool_fixture_path = tool_fixture_path
        self.terminal_command = terminal_command
        self.tool_timeout_ms = tool_timeout_ms
        self.timeout_seconds = timeout_seconds
        self.process_record_directory = process_record_directory
        self.parent_exit_disconnect_observed = False
        self.background_task_id: str | None = None
        self.requests: list[dict[str, Any]] = []

    def primary_requests(self) -> list[dict[str, Any]]:
        return [
            request
            for request in self.requests
            if isinstance(request.get("body"), dict)
            and request["body"].get("model") == "deepseek-v4-pro"
        ]

    def record(self, handler: BaseHTTPRequestHandler, body: bytes) -> None:
        try:
            parsed = json.loads(body)
        except (UnicodeDecodeError, json.JSONDecodeError):
            parsed = None
        authorization = handler.headers.get("Authorization")
        safe_headers: dict[str, Any] = {}
        for name in sorted(handler.headers.keys(), key=str.lower):
            value = handler.headers.get(name, "")
            lowered = name.lower()
            if lowered in {"content-type", "accept", "user-agent", "host"}:
                safe_headers[lowered] = value
            else:
                encoded = value.encode("utf-8", errors="replace")
                safe_headers[lowered] = {
                    "present": True,
                    "bytes": len(encoded),
                    "sha256": _sha256_bytes(encoded),
                    "value_recorded": False,
                }
        record = {
            "sequence": len(self.requests) + 1,
            "received_at_unix_ns": time.time_ns(),
            "client_ip": handler.client_address[0],
            "method": handler.command,
            "path": handler.path,
            "headers": safe_headers,
            "authorization_present": authorization is not None,
            "authorization_value_recorded": False,
            "body_bytes": len(body),
            "body_sha256": _sha256_bytes(body),
            "body": parsed,
        }
        self.requests.append(record)
        private_path = self.output_directory / "requests.private.jsonl"
        with private_path.open("a", encoding="utf-8", newline="\n") as handle:
            handle.write(json.dumps(record, ensure_ascii=False, sort_keys=True, allow_nan=False))
            handle.write("\n")


def _completion_chunks(model: str, marker: str = RESPONSE_MARKER) -> bytes:
    created = int(time.time())
    common = {
        "id": "chatcmpl-lif-fake-0001",
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
    }
    chunks = [
        {
            **common,
            "choices": [
                {"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}
            ],
        },
        {
            **common,
            "choices": [
                {
                    "index": 0,
                    "delta": {"content": marker},
                    "finish_reason": None,
                }
            ],
        },
        {
            **common,
            "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2,
            },
        },
    ]
    lines = [f"data: {json.dumps(chunk, separators=(',', ':'))}\n\n" for chunk in chunks]
    lines.append("data: [DONE]\n\n")
    return "".join(lines).encode("utf-8")


def _tool_call_chunks(model: str, tool_fixture_path: Path) -> bytes:
    created = int(time.time())
    common = {
        "id": "chatcmpl-lif-fake-tool-0001",
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
    }
    arguments = json.dumps(
        {"target_file": str(tool_fixture_path)}, separators=(",", ":")
    )
    chunks = [
        {
            **common,
            "choices": [
                {"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}
            ],
        },
        {
            **common,
            "choices": [
                {
                    "index": 0,
                    "delta": {"reasoning_content": REASONING_MARKER},
                    "finish_reason": None,
                }
            ],
        },
        {
            **common,
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "tool_calls": [
                            {
                                "index": 0,
                                "id": TOOL_CALL_ID,
                                "type": "function",
                                "function": {
                                    "name": "read_file",
                                    "arguments": arguments,
                                },
                            }
                        ]
                    },
                    "finish_reason": None,
                }
            ],
        },
        {
            **common,
            "choices": [
                {"index": 0, "delta": {}, "finish_reason": "tool_calls"}
            ],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 1,
                "total_tokens": 2,
            },
        },
    ]
    lines = [f"data: {json.dumps(chunk, separators=(',', ':'))}\n\n" for chunk in chunks]
    lines.append("data: [DONE]\n\n")
    return "".join(lines).encode("utf-8")


def _function_tool_call_chunks(
    model: str,
    *,
    call_id: str,
    function_name: str,
    arguments: dict[str, Any],
    reasoning_marker: str | None = None,
) -> bytes:
    created = int(time.time())
    common = {
        "id": f"chatcmpl-{call_id}",
        "object": "chat.completion.chunk",
        "created": created,
        "model": model,
    }
    chunks: list[dict[str, Any]] = [
        {
            **common,
            "choices": [
                {"index": 0, "delta": {"role": "assistant"}, "finish_reason": None}
            ],
        }
    ]
    if reasoning_marker is not None:
        chunks.append(
            {
                **common,
                "choices": [
                    {
                        "index": 0,
                        "delta": {"reasoning_content": reasoning_marker},
                        "finish_reason": None,
                    }
                ],
            }
        )
    chunks.extend(
        [
            {
                **common,
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "tool_calls": [
                                {
                                    "index": 0,
                                    "id": call_id,
                                    "type": "function",
                                    "function": {
                                        "name": function_name,
                                        "arguments": json.dumps(
                                            arguments, separators=(",", ":")
                                        ),
                                    },
                                }
                            ]
                        },
                        "finish_reason": None,
                    }
                ],
            },
            {
                **common,
                "choices": [
                    {"index": 0, "delta": {}, "finish_reason": "tool_calls"}
                ],
                "usage": {
                    "prompt_tokens": 1,
                    "completion_tokens": 1,
                    "total_tokens": 2,
                },
            },
        ]
    )
    lines = [
        f"data: {json.dumps(chunk, separators=(',', ':'))}\n\n" for chunk in chunks
    ]
    lines.append("data: [DONE]\n\n")
    return "".join(lines).encode("utf-8")


def _iter_string_values(value: Any) -> list[str]:
    values: list[str] = []
    if isinstance(value, str):
        values.append(value)
    elif isinstance(value, dict):
        for key, child in value.items():
            if key in {"task_id", "taskId"} and isinstance(child, str):
                values.insert(0, child)
            else:
                values.extend(_iter_string_values(child))
    elif isinstance(value, list):
        for child in value:
            values.extend(_iter_string_values(child))
    return values


def _iter_explicit_task_ids(value: Any) -> list[str]:
    values: list[str] = []
    if isinstance(value, dict):
        for key, child in value.items():
            if key in {"task_id", "taskId"} and isinstance(child, str):
                values.append(child)
            else:
                values.extend(_iter_explicit_task_ids(child))
    elif isinstance(value, list):
        for child in value:
            values.extend(_iter_explicit_task_ids(child))
    return values


def _extract_background_task_id(requests: list[dict[str, Any]]) -> str | None:
    if len(requests) < 2:
        return None
    body = requests[-1].get("body")
    messages = body.get("messages", []) if isinstance(body, dict) else []
    tool_messages = [
        message
        for message in messages
        if isinstance(message, dict)
        and message.get("role") == "tool"
        and message.get("tool_call_id") == PROCESS_TREE_TOOL_CALL_ID
    ]
    for message in reversed(tool_messages):
        for value in _iter_explicit_task_ids(message):
            if re.fullmatch(r"[A-Za-z][A-Za-z0-9_.:-]{2,127}", value):
                return value
        for value in _iter_string_values(message):
            stripped = value.strip("`'\" \t\r\n")
            if re.fullmatch(
                r"(?:[A-Za-z][A-Za-z0-9_.:-]{2,127}|"
                r"[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-"
                r"[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12})",
                stripped,
            ):
                if stripped not in {
                    "tool",
                    "assistant",
                    "completed",
                    "running",
                    "background",
                } and not stripped.startswith("call_"):
                    return stripped
            match = re.search(
                r"(?i)(?:<task-id>\s*|(?:task[_ ]?id|task)\s*[:=]\s*[`'\"]?)"
                r"([A-Za-z0-9][A-Za-z0-9_.:-]{2,127})",
                value,
            )
            if match:
                return match.group(1)
    return None


def _process_tree_tool_chunks(state: CaptureState, model: str) -> bytes:
    assert state.terminal_command is not None
    is_timeout = state.scenario == "process-timeout"
    return _function_tool_call_chunks(
        model,
        call_id=PROCESS_TREE_TOOL_CALL_ID,
        function_name="run_terminal_command",
        arguments={
            "command": state.terminal_command,
            "description": "Run the fixed fake-only child-tree containment fixture.",
            "background": not is_timeout,
            "timeout": state.tool_timeout_ms if is_timeout else 0,
        },
        reasoning_marker=REASONING_MARKER,
    )


def _process_cancel_chunks(state: CaptureState, model: str) -> bytes:
    if not _wait_for_process_tree_records(state):
        return _completion_chunks(model, "LIF_PROCESS_TREE_RECORDS_MISSING")
    assert state.process_record_directory is not None
    marker = state.process_record_directory / PROCESS_TREE_PRE_TRIGGER_MARKER
    deadline = time.monotonic() + state.timeout_seconds
    while not marker.is_file() and time.monotonic() < deadline:
        time.sleep(0.025)
    if not marker.is_file():
        return _completion_chunks(model, "LIF_PROCESS_PRE_TRIGGER_MARKER_MISSING")
    task_id = _extract_background_task_id(state.primary_requests())
    if task_id is None:
        return _completion_chunks(model, "LIF_PROCESS_TASK_ID_MISSING")
    state.background_task_id = task_id
    return _function_tool_call_chunks(
        model,
        call_id=PROCESS_TREE_KILL_CALL_ID,
        function_name="kill_command_or_subagent",
        arguments={"task_id": task_id},
    )


def _wait_for_process_tree_records(state: CaptureState) -> bool:
    if state.process_record_directory is None:
        return False
    deadline = time.monotonic() + min(10.0, state.timeout_seconds)
    expected = [
        state.process_record_directory / f"{role}.json"
        for role in ("root", "child", "grandchild")
    ]
    while time.monotonic() < deadline:
        if all(path.is_file() for path in expected):
            return True
        time.sleep(0.025)
    return False


def _process_output_chunks(state: CaptureState, model: str) -> bytes:
    if state.background_task_id is None:
        return _completion_chunks(model, "LIF_PROCESS_TASK_ID_MISSING")
    return _function_tool_call_chunks(
        model,
        call_id=PROCESS_TREE_OUTPUT_CALL_ID,
        function_name="get_command_or_subagent_output",
        arguments={"task_ids": [state.background_task_id], "timeout_ms": 5000},
    )


def _final_tool_chunks(model: str) -> bytes:
    return _completion_chunks(model, TOOL_RESPONSE_MARKER)


def _continuity_summary(requests: list[dict[str, Any]]) -> dict[str, Any]:
    if len(requests) < 2 or not isinstance(requests[1].get("body"), dict):
        return {
            "second_request_observed": False,
            "reasoning_marker_preserved": False,
            "tool_call_id_preserved": False,
            "tool_result_observed": False,
            "tool_result_marker_observed": False,
        }
    messages = requests[1]["body"].get("messages", [])
    assistant_messages = [
        item for item in messages if isinstance(item, dict) and item.get("role") == "assistant"
    ]
    tool_messages = [
        item for item in messages if isinstance(item, dict) and item.get("role") == "tool"
    ]
    reasoning_preserved = any(
        item.get("reasoning_content") == REASONING_MARKER for item in assistant_messages
    )
    tool_call_preserved = False
    for item in assistant_messages:
        calls = item.get("tool_calls")
        if isinstance(calls, list) and any(
            isinstance(call, dict) and call.get("id") == TOOL_CALL_ID for call in calls
        ):
            tool_call_preserved = True
    result_observed = any(item.get("tool_call_id") == TOOL_CALL_ID for item in tool_messages)
    result_marker_observed = any(
        TOOL_RESULT_MARKER in str(item.get("content", "")) for item in tool_messages
    )
    return {
        "second_request_observed": True,
        "reasoning_marker_preserved": reasoning_preserved,
        "tool_call_id_preserved": tool_call_preserved,
        "tool_result_observed": result_observed,
        "tool_result_marker_observed": result_marker_observed,
    }


def make_handler(state: CaptureState) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        protocol_version = "HTTP/1.1"

        def do_POST(self) -> None:  # noqa: N802 - stdlib callback name
            if self.client_address[0] != HOST:
                self.send_error(403)
                return
            raw_length = self.headers.get("Content-Length")
            try:
                length = int(raw_length or "0")
            except ValueError:
                self.send_error(400)
                return
            if length < 0 or length > MAX_BODY_BYTES:
                self.send_error(413)
                return
            body = self.rfile.read(length)
            state.record(self, body)
            model = "deepseek-v4-pro"
            try:
                parsed = json.loads(body)
                if isinstance(parsed, dict) and isinstance(parsed.get("model"), str):
                    model = parsed["model"]
            except (UnicodeDecodeError, json.JSONDecodeError):
                pass
            primary_sequence = len(state.primary_requests())
            if model != "deepseek-v4-pro":
                response = _completion_chunks(model, "LIF fake session")
            elif state.scenario in {"tool-continuity", "tool-cancel"} and primary_sequence == 1:
                assert state.tool_fixture_path is not None
                response = _tool_call_chunks(model, state.tool_fixture_path)
            elif state.scenario == "tool-continuity":
                response = _final_tool_chunks(model)
            elif (
                state.scenario
                in {"process-timeout", "process-cancel", "process-parent-exit"}
                and primary_sequence == 1
            ):
                response = _process_tree_tool_chunks(state, model)
            elif state.scenario == "process-cancel" and primary_sequence == 2:
                response = _process_cancel_chunks(state, model)
            elif state.scenario == "process-cancel" and primary_sequence == 3:
                response = _process_output_chunks(state, model)
            elif state.scenario == "process-parent-exit" and primary_sequence == 2:
                if not _wait_for_process_tree_records(state):
                    response = _completion_chunks(
                        model, "LIF_PROCESS_TREE_RECORDS_MISSING"
                    )
                    self.send_response(200)
                    self.send_header(
                        "Content-Type", "text/event-stream; charset=utf-8"
                    )
                    self.send_header("Connection", "close")
                    self.send_header("Content-Length", str(len(response)))
                    self.end_headers()
                    self.wfile.write(response)
                    self.wfile.flush()
                    self.close_connection = True
                    return
                self.send_response(200)
                self.send_header("Content-Type", "text/event-stream; charset=utf-8")
                self.send_header("Cache-Control", "no-cache")
                self.send_header("Connection", "close")
                self.end_headers()
                deadline = time.monotonic() + state.timeout_seconds
                try:
                    while time.monotonic() < deadline:
                        self.wfile.write(b": LIF_PARENT_EXIT_HOLD\n\n")
                        self.wfile.flush()
                        time.sleep(0.1)
                except (BrokenPipeError, ConnectionResetError):
                    state.parent_exit_disconnect_observed = True
                self.close_connection = True
                return
            elif state.scenario in {"process-timeout", "process-cancel"}:
                response = _completion_chunks(model, "LIF_PROCESS_TREE_PROBE_OK")
            else:
                response = _completion_chunks(model)
            self.send_response(200)
            self.send_header("Content-Type", "text/event-stream; charset=utf-8")
            self.send_header("Cache-Control", "no-cache")
            self.send_header("Connection", "close")
            self.send_header("Content-Length", str(len(response)))
            self.end_headers()
            self.wfile.write(response)
            self.wfile.flush()
            self.close_connection = True

        def log_message(self, _format: str, *args: object) -> None:
            return

    return Handler


def run_server(
    output_directory: Path,
    timeout_seconds: float,
    *,
    scenario: str = "single",
    tool_fixture_path: Path | None = None,
    terminal_command_path: Path | None = None,
    tool_timeout_ms: int = 3000,
    process_record_directory: Path | None = None,
) -> dict[str, Any]:
    if output_directory.exists():
        raise ValueError(f"refusing to overwrite provider output: {output_directory}")
    output_directory.mkdir(parents=True)
    if scenario in {"tool-continuity", "tool-cancel"}:
        if tool_fixture_path is None or not tool_fixture_path.is_file():
            raise ValueError(f"{scenario} requires an existing --tool-fixture-path")
        tool_fixture_path = tool_fixture_path.resolve()
    terminal_command: str | None = None
    if scenario in {"process-timeout", "process-cancel", "process-parent-exit"}:
        if terminal_command_path is None or not terminal_command_path.is_file():
            raise ValueError(
                f"{scenario} requires an existing --terminal-command-path"
            )
        terminal_command = terminal_command_path.resolve().read_text(
            encoding="utf-8"
        ).strip()
        if not terminal_command:
            raise ValueError("terminal command fixture is empty")
        if tool_timeout_ms < 500 or tool_timeout_ms > 30000:
            raise ValueError("--tool-timeout-ms must be in [500, 30000]")
        if process_record_directory is None:
            raise ValueError(f"{scenario} requires --process-record-directory")
        process_record_directory = process_record_directory.resolve()
    state = CaptureState(
        output_directory,
        scenario,
        tool_fixture_path,
        terminal_command,
        tool_timeout_ms,
        timeout_seconds,
        process_record_directory,
    )
    server = HTTPServer((HOST, 0), make_handler(state))
    server.timeout = 0.25
    port = int(server.server_address[1])
    ready = {
        "schema_version": "0.1.0",
        "ready": True,
        "host": HOST,
        "port": port,
        "external_bind": False,
        "authorization_value_recorded": False,
    }
    _atomic_write_json(output_directory / "ready.json", ready)
    started = time.monotonic()
    expected_requests = {
        "tool-continuity": 2,
        "process-timeout": 2,
        "process-cancel": 4,
        "process-parent-exit": 2,
    }.get(scenario, 1)
    while len(state.primary_requests()) < expected_requests and time.monotonic() - started < timeout_seconds:
        server.handle_request()
    server.server_close()
    terminal_state = (
        "succeeded"
        if len(state.primary_requests()) == expected_requests
        else "timed_out"
    )
    private_path = output_directory / "requests.private.jsonl"
    result = {
        "schema_version": "0.1.0",
        "scenario": scenario,
        "terminal_state": terminal_state,
        "bind": {"host": HOST, "port": port, "external": False},
        "request_count": len(state.requests),
        "primary_request_count": len(state.primary_requests()),
        "auxiliary_request_count": len(state.requests) - len(state.primary_requests()),
        "requests": [
            {
                key: value
                for key, value in request.items()
                if key not in {"body", "headers"}
            }
            | {
                "header_names": sorted(request["headers"]),
                "model": (
                    request["body"].get("model")
                    if isinstance(request["body"], dict)
                    else None
                ),
                "stream": (
                    request["body"].get("stream")
                    if isinstance(request["body"], dict)
                    else None
                ),
                "message_count": (
                    len(request["body"].get("messages", []))
                    if isinstance(request["body"], dict)
                    and isinstance(request["body"].get("messages"), list)
                    else None
                ),
                "message_roles": (
                    [
                        message.get("role")
                        for message in request["body"].get("messages", [])
                        if isinstance(message, dict)
                    ]
                    if isinstance(request["body"], dict)
                    and isinstance(request["body"].get("messages"), list)
                    else None
                ),
                "tool_names": (
                    [
                        tool.get("function", {}).get("name")
                        for tool in request["body"].get("tools", [])
                        if isinstance(tool, dict)
                        and isinstance(tool.get("function"), dict)
                    ]
                    if isinstance(request["body"], dict)
                    and isinstance(request["body"].get("tools"), list)
                    else None
                ),
                "request_class": (
                    "primary"
                    if isinstance(request["body"], dict)
                    and request["body"].get("model") == "deepseek-v4-pro"
                    else "auxiliary"
                ),
            }
            for request in state.requests
        ],
        "private_capture": {
            "path": str(private_path),
            "exists": private_path.is_file(),
            "sha256": _sha256_bytes(private_path.read_bytes())
            if private_path.is_file()
            else None,
            "contains_raw_headers": False,
            "contains_request_body": private_path.is_file(),
            "classification": "fake-fixture-private",
        },
        "response": {
            "marker": (
                TOOL_RESPONSE_MARKER
                if scenario == "tool-continuity"
                else TOOL_CALL_ID
                if scenario == "tool-cancel"
                else "LIF_PROCESS_TREE_PROBE_OK"
                if scenario in {"process-timeout", "process-cancel"}
                else "LIF_PARENT_EXIT_HOLD"
                if scenario == "process-parent-exit"
                else RESPONSE_MARKER
            ),
            "streaming_sse": True,
            "real_model_invoked": False,
        },
        "continuity": (
            _continuity_summary(state.primary_requests())
            if scenario == "tool-continuity"
            else {
                "first_tool_request_observed": len(state.primary_requests()) == 1,
                "second_request_observed": len(state.primary_requests()) > 1,
            }
            if scenario == "tool-cancel"
            else {
                "background_task_id_observed": state.background_task_id is not None,
                "background_task_id": state.background_task_id,
                "parent_exit_disconnect_observed": state.parent_exit_disconnect_observed,
            }
            if scenario
            in {"process-timeout", "process-cancel", "process-parent-exit"}
            else None
        ),
        "limitations": [
            "This fake fixture captures request bodies in an ignored local artifact and is not suitable for sensitive prompts.",
            "Successful parsing proves compatibility only with this fixed SSE response, not with the real DeepSeek service.",
        ],
    }
    _atomic_write_json(output_directory / "provider-result.json", result)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description="Single-request loopback DeepSeek fixture")
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument(
        "--scenario",
        choices=(
            "single",
            "tool-continuity",
            "tool-cancel",
            "process-timeout",
            "process-cancel",
            "process-parent-exit",
        ),
        default="single",
    )
    parser.add_argument("--tool-fixture-path", type=Path)
    parser.add_argument("--terminal-command-path", type=Path)
    parser.add_argument("--tool-timeout-ms", type=int, default=3000)
    parser.add_argument("--process-record-directory", type=Path)
    args = parser.parse_args()
    if args.timeout_seconds <= 0 or args.timeout_seconds > 300:
        parser.error("--timeout-seconds must be in (0, 300]")
    result = run_server(
        args.output_directory.resolve(),
        args.timeout_seconds,
        scenario=args.scenario,
        tool_fixture_path=args.tool_fixture_path,
        terminal_command_path=args.terminal_command_path,
        tool_timeout_ms=args.tool_timeout_ms,
        process_record_directory=args.process_record_directory,
    )
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if result["terminal_state"] == "succeeded" else 2


if __name__ == "__main__":
    raise SystemExit(main())
