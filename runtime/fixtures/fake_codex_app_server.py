from __future__ import annotations

import json
import sys
import time


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
    sys.stderr.write("fake app-server diagnostic\n")
    sys.stderr.flush()
    while sys.stdin.readline():
        pass
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
