from __future__ import annotations

import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
from pathlib import Path
import time
from typing import Any


HOST = "127.0.0.1"
RESPONSE_MARKER = "LIF_FAKE_PROVIDER_OK"
TOOL_RESPONSE_MARKER = "LIF_FAKE_TOOL_CONTINUITY_OK"
REASONING_MARKER = "LIF_FAKE_REASONING_CONTINUITY_001"
TOOL_RESULT_MARKER = "LIF_TOOL_FIXTURE_CONTENT_001"
TOOL_CALL_ID = "call_lif_read_fixture_001"
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
        self, output_directory: Path, scenario: str, tool_fixture_path: Path | None
    ) -> None:
        self.output_directory = output_directory
        self.scenario = scenario
        self.tool_fixture_path = tool_fixture_path
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
) -> dict[str, Any]:
    if output_directory.exists():
        raise ValueError(f"refusing to overwrite provider output: {output_directory}")
    output_directory.mkdir(parents=True)
    if scenario in {"tool-continuity", "tool-cancel"}:
        if tool_fixture_path is None or not tool_fixture_path.is_file():
            raise ValueError(f"{scenario} requires an existing --tool-fixture-path")
        tool_fixture_path = tool_fixture_path.resolve()
    state = CaptureState(output_directory, scenario, tool_fixture_path)
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
    expected_requests = 2 if scenario == "tool-continuity" else 1
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
        choices=("single", "tool-continuity", "tool-cancel"),
        default="single",
    )
    parser.add_argument("--tool-fixture-path", type=Path)
    args = parser.parse_args()
    if args.timeout_seconds <= 0 or args.timeout_seconds > 300:
        parser.error("--timeout-seconds must be in (0, 300]")
    result = run_server(
        args.output_directory.resolve(),
        args.timeout_seconds,
        scenario=args.scenario,
        tool_fixture_path=args.tool_fixture_path,
    )
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if result["terminal_state"] == "succeeded" else 2


if __name__ == "__main__":
    raise SystemExit(main())
