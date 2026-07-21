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
    def __init__(self, output_directory: Path) -> None:
        self.output_directory = output_directory
        self.requests: list[dict[str, Any]] = []

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


def _completion_chunks(model: str) -> bytes:
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
                    "delta": {"content": RESPONSE_MARKER},
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


def run_server(output_directory: Path, timeout_seconds: float) -> dict[str, Any]:
    if output_directory.exists():
        raise ValueError(f"refusing to overwrite provider output: {output_directory}")
    output_directory.mkdir(parents=True)
    state = CaptureState(output_directory)
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
    while not state.requests and time.monotonic() - started < timeout_seconds:
        server.handle_request()
    server.server_close()
    terminal_state = "succeeded" if len(state.requests) == 1 else "timed_out"
    private_path = output_directory / "requests.private.jsonl"
    result = {
        "schema_version": "0.1.0",
        "terminal_state": terminal_state,
        "bind": {"host": HOST, "port": port, "external": False},
        "request_count": len(state.requests),
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
            "marker": RESPONSE_MARKER,
            "streaming_sse": True,
            "real_model_invoked": False,
        },
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
    args = parser.parse_args()
    if args.timeout_seconds <= 0 or args.timeout_seconds > 300:
        parser.error("--timeout-seconds must be in (0, 300]")
    result = run_server(args.output_directory.resolve(), args.timeout_seconds)
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if result["terminal_state"] == "succeeded" else 2


if __name__ == "__main__":
    raise SystemExit(main())
