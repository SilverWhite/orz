"""Smoke tests for the Classical Console first path."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

from actions import register_all
from console import build_error_envelope, run_request
from service_registry import ConsoleError, ServiceRegistry
from trace import TraceStore


PASS = 0
FAIL = 0


def check(name: str, cond: bool, detail: str = "") -> None:
    global PASS, FAIL
    if cond:
        PASS += 1
        print(f"PASS {name}")
    else:
        FAIL += 1
        print(f"FAIL {name} {detail}")


def in_process_tests(root: str) -> None:
    registry = ServiceRegistry()
    trace_store = TraceStore()
    register_all(registry, trace_store)

    src = Path(root) / "hello.txt"
    src.write_text("hello world", encoding="utf-8")
    gb = Path(root) / "gb.txt"
    gb.write_bytes("中文".encode("gb18030"))
    big = Path(root) / "big.txt"
    big.write_text("x" * (1024 * 1024 + 100), encoding="utf-8")
    sub = Path(root) / "sub"
    sub.mkdir()
    (sub / "a.txt").write_text("a", encoding="utf-8")
    (sub / "b.txt").write_text("b", encoding="utf-8")

    r = registry.call("workspace.read_file", {"path": "hello.txt"}, root)
    check("read_file utf-8", r["content"] == "hello world" and r["encoding"] == "utf-8")

    r = registry.call("workspace.read_file", {"path": "gb.txt"}, root)
    check("read_file gb18030", r["content"] == "中文" and r["encoding"] == "gb18030")

    r = registry.call("workspace.read_file", {"path": "big.txt"}, root)
    check("read_file truncated", r["truncated"] is True and "read_file truncated" in r["content"])

    def err(**data):
        try:
            registry.call("workspace.read_file", data, root)
            return None
        except ConsoleError as exc:
            return exc

    e = err(path=os.path.abspath(__file__))
    check("out_of_scope", e is not None and e.code == "out_of_scope" and e.step == "target")
    e = err(path="missing.txt")
    check("not_found", e is not None and e.code == "not_found" and e.step == "target"
          and e.upstream == {"resolved_path": str((Path(root) / "missing.txt").resolve())})
    e = err(path=123)
    check("invalid_arguments", e is not None and e.code == "invalid_arguments"
          and e.step == "contract")
    e = err(path="hello.txt", extra=1)
    check("extra_field_rejected", e is not None and e.code == "invalid_arguments"
          and e.step == "contract")

    r = registry.call("workspace.list_dir", {"path": "sub"}, root)
    check("list_dir sorted",
          [e["name"] for e in r["entries"]] == ["a.txt", "b.txt"] and r["total"] == 2)

    r = registry.call("workspace.index", {}, root)
    check("workspace.index",
          "hello.txt" in r["files"] and "sub/a.txt" in r["files"]
          and r["total"] == len(r["files"]) and r["truncated"] is False)

    try:
        registry.call("workspace.no_such", {"path": "."}, root)
        check("unknown_service", False)
    except ConsoleError as exc:
        check("unknown_service", exc.code == "unknown_service" and exc.step == "registry")

    trace = trace_store.new("req-x")
    out = run_request(
        {"proto": 1, "id": "x1", "service": "workspace.read_file",
         "data": {"path": "missing.txt"}},
        registry,
        root,
        trace,
    )
    check("trace failure event",
          out["ok"] is False and out["error"]["step"] == "target"
          and trace.events[-1]["ok"] is False
          and trace.events[-1]["code"] == "not_found")
    check("trace store lookup", trace_store.get(trace.trace_id) is trace)

    r = registry.call("assistant.trace", {"trace_id": trace.trace_id}, root)
    check("assistant.trace read",
          r["trace_id"] == trace.trace_id and r["request_id"] == "req-x"
          and r["truncated"] is False and r["events"] == trace.events)

    try:
        registry.call("assistant.trace", {"trace_id": "t000000"}, root)
        check("assistant.trace unknown", False)
    except ConsoleError as exc:
        check("assistant.trace unknown", exc.code == "not_found" and exc.step == "registry")

    trace2 = trace_store.new("req-y")
    trace2.add(step="protocol", ok=True)
    trace2.add(step="execute", action="workspace.broken", ok=False,
               code="execution_failed", message="boom")
    env = build_error_envelope(
        "9",
        ConsoleError("boom", step="execute", upstream={"x": 1}),
        trace2,
    )
    check("execute failure trace feedback",
          env["error"]["trace_id"] == trace2.trace_id
          and env["error"]["step"] == "execute"
          and [e["code"] for e in env["error"]["trace"] if "code" in e]
          == ["execution_failed"])


def subprocess_tests(root: str) -> None:
    script = Path(__file__).with_name("console.py")
    proc = subprocess.Popen(
        [sys.executable, str(script), "--allow-root", root],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        text=True,
        encoding="utf-8",
    )
    assert proc.stdin and proc.stdout
    reqs = [
        {"proto": 1, "id": "1", "service": "workspace.read_file", "data": {"path": "hello.txt"}},
        {"proto": 1, "id": "2", "service": "workspace.read_file", "data": {"path": "../escape"}},
        {"proto": 1, "id": "3", "service": "workspace.missing", "data": {}},
        {"proto": 1, "id": "4", "text": "read the file hello.txt"},
        {"proto": 1, "id": "5", "text": "please fly to the moon"},
        {"proto": 1, "id": "6", "text": "读取文件 hello.txt"},
        {"proto": 1, "id": "8", "text": "read the file sub/a.txt"},
        {"proto": 1, "id": "9", "text": "read the file nope.txt"},
        {"proto": 1, "id": "7", "service": "workspace.read_file"},
        {"proto": 1, "id": "10", "service": "assistant.trace",
         "data": {"trace_id": "t000001"}},
        "not-json",
    ]
    for req in reqs:
        proc.stdin.write(json.dumps(req, ensure_ascii=False) + "\n")
    proc.stdin.flush()
    proc.stdin.close()
    out_lines = [json.loads(line) for line in proc.stdout]
    proc.wait(timeout=20)
    by_id = {o.get("id"): o for o in out_lines}

    check("stdio service ok",
          by_id["1"]["ok"] is True and by_id["1"]["response"]["content"] == "hello world")
    check("stdio trace_id present", isinstance(by_id["1"].get("trace_id"), str)
          and len(by_id["1"]["trace_id"]) > 0)
    check("stdio out_of_scope", by_id["2"]["ok"] is False
          and by_id["2"]["error"]["code"] == "out_of_scope"
          and by_id["2"]["error"]["step"] == "target"
          and by_id["2"]["error"]["upstream"]["resolved_path"].endswith("escape")
          and isinstance(by_id["2"]["error"]["trace_id"], str))
    check("stdio unknown_service", by_id["3"]["ok"] is False
          and by_id["3"]["error"]["code"] == "unknown_service"
          and by_id["3"]["error"]["step"] == "registry")
    check("stdio intent en", by_id["4"]["ok"] is True
          and by_id["4"]["response"]["content"] == "hello world")
    check("stdio unknown_intent", by_id["5"]["ok"] is False
          and by_id["5"]["error"]["code"] == "unknown_intent"
          and by_id["5"]["error"]["step"] == "intent")
    check("stdio intent zh", by_id["6"]["ok"] is True
          and by_id["6"]["response"]["content"] == "hello world")
    check("stdio intent indexed subpath", by_id["8"]["ok"] is True
          and by_id["8"]["response"]["content"] == "a")
    check("stdio intent non-indexed", by_id["9"]["ok"] is False
          and by_id["9"]["error"]["code"] == "unknown_intent"
          and by_id["9"]["error"]["step"] == "intent")
    check("stdio missing data", by_id["7"]["ok"] is False
          and by_id["7"]["error"]["code"] == "invalid_arguments"
          and by_id["7"]["error"]["step"] == "contract")
    malformed = [o for o in out_lines if o.get("id") is None and o.get("ok") is False]
    check("stdio malformed json", len(malformed) == 1
          and malformed[0]["error"]["code"] == "protocol_error"
          and malformed[0]["error"]["step"] == "protocol")
    check("stdio trace service", by_id["10"]["ok"] is True
          and by_id["10"]["response"]["trace_id"] == "t000001"
          and any(e.get("action") == "workspace.read_file" and e.get("ok")
                  for e in by_id["10"]["response"]["events"]))
    errors = [o for o in out_lines if o.get("ok") is False]
    check("error envelope complete",
          all({"step", "code", "message", "upstream", "trace_id"} <= set(o["error"])
              for o in errors))


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        in_process_tests(tmp)
        subprocess_tests(tmp)
    print(f"RESULT pass={PASS} fail={FAIL}")
    return 1 if FAIL else 0


if __name__ == "__main__":
    raise SystemExit(main())
