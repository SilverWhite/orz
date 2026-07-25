from __future__ import annotations

import json
import os
from pathlib import Path
import sys
import time
import tomllib
import urllib.error
import urllib.request


def receive() -> dict:
    line = sys.stdin.readline()
    if not line:
        raise RuntimeError("unexpected stdin EOF")
    value = json.loads(line)
    if not isinstance(value, dict):
        raise RuntimeError("request is not an object")
    return value


def send(value: dict) -> None:
    sys.stdout.write(json.dumps(value, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def thread() -> dict:
    return {
        "id": "thr_fake_capture",
        "ephemeral": True,
        "path": None,
        "status": {"type": "idle"},
        "turns": [],
    }


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "success"
    initialize = receive()
    if initialize.get("method") != "initialize":
        return 3
    if mode == "timeout":
        time.sleep(30)
        return 4
    if mode == "malformed":
        sys.stdout.write("{not-json}\n")
        sys.stdout.flush()
        return 5
    send(
        {
            "id": initialize.get("id"),
            "result": {
                "userAgent": "fake-codex/0.1.0",
                "platformFamily": "windows",
                "platformOs": "windows",
            },
        }
    )
    initialized = receive()
    request = receive()
    if initialized.get("method") != "initialized":
        return 6
    if request.get("method") != "thread/start":
        return 7
    params = request.get("params", {})
    if (
        params.get("ephemeral") is not True
        or params.get("sandbox") != "read-only"
        or "input" in params
    ):
        return 8

    response_thread = thread()
    notification_thread = thread()
    if mode == "mismatch":
        notification_thread["id"] = "thr_other"
    response = {"id": request.get("id"), "result": {"thread": response_thread}}
    notification = {
        "method": "thread/started",
        "params": {"thread": notification_thread},
    }
    if mode == "notification-first":
        send(notification)
        send(response)
    else:
        send(response)
        send(notification)

    if mode in {"turn-success", "turn-provider-invalid"}:
        turn_request = receive()
        if turn_request.get("method") != "turn/start":
            return 9
        turn_params = turn_request.get("params", {})
        if (
            turn_params.get("threadId") != "thr_fake_capture"
            or turn_params.get("input")
            != [
                {
                    "type": "text",
                    "text": (
                        "LIFECYCLE_PROBE_INPUT_0E99A0F4: reply with the "
                        "fixed synthetic response only."
                    ),
                }
            ]
        ):
            return 10
        turn = {
            "id": "turn_fake_probe",
            "status": "inProgress",
            "items": [],
            "error": None,
        }
        send({"id": turn_request.get("id"), "result": {"turn": turn}})
        send({"method": "turn/started", "params": {"turn": turn}})

        config_path = Path(os.environ["CODEX_HOME"]) / "config.toml"
        config = tomllib.loads(config_path.read_text(encoding="utf-8"))
        provider_id = config["model_provider"]
        provider = config["model_providers"][provider_id]
        input_text = turn_params["input"][0]["text"]
        body = {
            "model": config["model"],
            "stream": mode != "turn-provider-invalid",
            "input": [
                {
                    "type": "message",
                    "role": "user",
                    "content": [{"type": "input_text", "text": input_text}],
                }
            ],
        }
        request_data = json.dumps(body, separators=(",", ":")).encode("utf-8")
        provider_request = urllib.request.Request(
            provider["base_url"].rstrip("/") + "/responses",
            data=request_data,
            headers={
                "Accept": "text/event-stream",
                "Content-Type": "application/json",
            },
            method="POST",
        )
        provider_ok = False
        try:
            with urllib.request.urlopen(provider_request, timeout=5) as upstream:
                upstream.read()
                provider_ok = upstream.status == 200
        except urllib.error.HTTPError:
            provider_ok = False

        terminal = {
            "id": "turn_fake_probe",
            "status": "completed" if provider_ok else "failed",
            "items": [],
            "error": (
                None
                if provider_ok
                else {"message": "synthetic provider request failed"}
            ),
        }
        send({"method": "turn/completed", "params": {"turn": terminal}})

    sys.stderr.write("fake app-server diagnostic\n")
    sys.stderr.flush()
    while sys.stdin.readline():
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
