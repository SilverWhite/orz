"""Smoke tests for the Classical Console first path."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import sample2_editor_benchmark as benchmark
import sample3_script_benchmark as script_benchmark
import script_runner
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
        {"proto": 1, "id": "11", "service": "workspace.search_replace",
         "data": {"path": "hello.txt", "old_string": "hello", "new_string": "hi"}},
        {"proto": 1, "id": "12", "service": "workspace.replace_by_intent",
         "data": {"path": "hello.txt", "intent": "replace 'world' with 'there'"}},
        {"proto": 1, "id": "13", "service": "workspace.search_replace",
         "data": {"path": "hello.txt", "old_string": "zzz", "new_string": "x"}},
        {"proto": 1, "id": "14", "service": "workspace.run_script",
         "data": {"script": [
             {"do": "workspace.read_file", "with": {"path": "sub/a.txt"}, "as": "a"},
             {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"},
         ]}},
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
    check("stdio search_replace ok", by_id["11"]["ok"] is True
          and by_id["11"]["response"]["applied"] == 1
          and by_id["11"]["response"]["new_string"] == "hi")
    check("stdio replace_by_intent ok", by_id["12"]["ok"] is True
          and by_id["12"]["response"]["applied"] == 1
          and by_id["12"]["response"]["intent"] == "replace 'world' with 'there'")
    check("stdio search_replace no_match", by_id["13"]["ok"] is False
          and by_id["13"]["error"]["code"] == "no_match"
          and by_id["13"]["error"]["step"] == "execute")
    check("stdio script ok", by_id["14"]["ok"] is True
          and by_id["14"]["response"]["result"]["content"] == "a"
          and len(by_id["14"]["response"]["steps"]) == 2)
    errors = [o for o in out_lines if o.get("ok") is False]
    check("error envelope complete",
          all({"step", "code", "message", "upstream", "trace_id"} <= set(o["error"])
              for o in errors))


def editor_tests(root: str) -> None:
    registry = ServiceRegistry()
    trace_store = TraceStore()
    register_all(registry, trace_store)

    t1 = Path(root) / "t1.txt"
    t1.write_bytes(b"hello world\n")
    dup = Path(root) / "dup.txt"
    dup.write_bytes(b"a b a\n")
    crlf = Path(root) / "crlf.txt"
    crlf.write_bytes(b"alpha\r\nbeta\r\n")
    gb = Path(root) / "gb.txt"
    gb.write_bytes("中文标题\n正文内容\n".encode("gb18030"))
    intent_f = Path(root) / "intent.txt"
    intent_f.write_bytes(b"keep\n")
    zh_f = Path(root) / "zh.txt"
    zh_f.write_bytes("你好\n".encode("utf-8"))
    quotes_f = Path(root) / "quotes.txt"
    quotes_f.write_bytes(b'use "foo" here\n')
    every_f = Path(root) / "every.txt"
    every_f.write_bytes(b"x y x\n")

    r = registry.call(
        "workspace.search_replace",
        {"path": "t1.txt", "old_string": "world", "new_string": "there"},
        root,
    )
    check("search_replace exact",
          r["applied"] == 1 and r["replaced_all"] is False and r["encoding"] == "utf-8")
    check("search_replace content written",
          registry.call("workspace.read_file", {"path": "t1.txt"}, root)["content"]
          == "hello there\n")

    def expect_error(service: str, data: dict, code: str, step: str):
        try:
            registry.call(service, data, root)
            return None
        except ConsoleError as exc:
            return exc.code == code and exc.step == step

    check("search_replace no_match",
          expect_error("workspace.search_replace",
                       {"path": "t1.txt", "old_string": "nope", "new_string": "x"},
                       "no_match", "execute"))
    check("search_replace ambiguous",
          expect_error("workspace.search_replace",
                       {"path": "dup.txt", "old_string": "a", "new_string": "x"},
                       "ambiguous_match", "execute"))
    r = registry.call(
        "workspace.search_replace",
        {"path": "dup.txt", "old_string": "a", "new_string": "x", "replace_all": True},
        root,
    )
    check("search_replace replace_all",
          r["applied"] == 2 and r["replaced_all"] is True
          and registry.call("workspace.read_file", {"path": "dup.txt"}, root)["content"]
          == "x b x\n")
    check("search_replace same_string",
          expect_error("workspace.search_replace",
                       {"path": "dup.txt", "old_string": "x", "new_string": "x"},
                       "invalid_arguments", "contract"))
    check("search_replace empty_old_string_guard",
          expect_error("workspace.search_replace",
                       {"path": "t1.txt", "old_string": "", "new_string": "x"},
                       "empty_old_string_not_allowed", "contract"))
    r = registry.call(
        "workspace.search_replace",
        {"path": "new.txt", "old_string": "", "new_string": "created"},
        root,
    )
    check("search_replace create file",
          r["applied"] == 1
          and registry.call("workspace.read_file", {"path": "new.txt"}, root)["content"]
          == "created")
    r = registry.call(
        "workspace.search_replace",
        {"path": "crlf.txt", "old_string": "beta", "new_string": "gamma"},
        root,
    )
    check("search_replace crlf preserved",
          r["applied"] == 1 and r["crlf_normalized"] is True
          and crlf.read_bytes() == b"alpha\r\ngamma\r\n")
    r = registry.call(
        "workspace.search_replace",
        {"path": "gb.txt", "old_string": "正文内容", "new_string": "正文替换"},
        root,
    )
    check("search_replace gb18030 decode+write utf-8",
          r["applied"] == 1 and r["encoding"] == "utf-8"
          and registry.call("workspace.read_file", {"path": "gb.txt"}, root)["content"]
          == "中文标题\n正文替换\n")
    check("search_replace out_of_scope",
          expect_error("workspace.search_replace",
                       {"path": os.path.abspath(__file__), "old_string": "x", "new_string": "y"},
                       "out_of_scope", "target"))
    check("search_replace extra_field_rejected",
          expect_error("workspace.search_replace",
                       {"path": "t1.txt", "old_string": "a", "new_string": "b", "extra": 1},
                       "invalid_arguments", "contract"))

    r = registry.call(
        "workspace.replace_by_intent",
        {"path": "intent.txt", "intent": "replace 'keep' with 'change'"},
        root,
    )
    check("replace_by_intent en",
          r["applied"] == 1 and r["old_string"] == "keep" and r["new_string"] == "change"
          and r["intent"] == "replace 'keep' with 'change'")
    r = registry.call(
        "workspace.replace_by_intent",
        {"path": "zh.txt", "intent": "把'你好'替换为'您好'"},
        root,
    )
    check("replace_by_intent zh",
          r["applied"] == 1 and r["new_string"] == "您好"
          and registry.call("workspace.read_file", {"path": "zh.txt"}, root)["content"]
          == "您好\n")
    r = registry.call(
        "workspace.replace_by_intent",
        {"path": "quotes.txt", "intent": "replace \u201cfoo\u201d with \u201cbar\u201d"},
        root,
    )
    check("replace_by_intent curly quotes",
          r["applied"] == 1 and r["old_string"] == "foo" and r["new_string"] == "bar")
    r = registry.call(
        "workspace.replace_by_intent",
        {"path": "every.txt", "intent": "replace 'x' with 'z' everywhere"},
        root,
    )
    check("replace_by_intent everywhere",
          r["applied"] == 2 and r["replaced_all"] is True
          and registry.call("workspace.read_file", {"path": "every.txt"}, root)["content"]
          == "z y z\n")
    check("replace_by_intent unparsable",
          expect_error("workspace.replace_by_intent",
                       {"path": "t1.txt", "intent": "please edit the file"},
                       "unparsable_intent", "contract"))


def script_tests(root: str) -> None:
    registry = ServiceRegistry()
    trace_store = TraceStore()
    register_all(registry, trace_store)

    (Path(root) / "s3_a.txt").write_text("hello script", encoding="utf-8")
    s3_dir = Path(root) / "s3_dir"
    s3_dir.mkdir()
    (s3_dir / "s3_a.txt").write_text("hello script", encoding="utf-8")
    (s3_dir / "s3_b.txt").write_text("other", encoding="utf-8")
    (Path(root) / "s3_empty").mkdir()

    trace = trace_store.new("s3-ok")
    out = run_request(
        {"proto": 1, "id": "s3-1", "service": "workspace.run_script",
         "data": {"script": [
             {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
             {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"},
         ]}},
        registry,
        root,
        trace,
    )
    check("script ref path ok",
          out["ok"] is True
          and out["response"]["result"]["content"] == "hello script"
          and len(out["response"]["steps"]) == 2
          and all(s["ok"] for s in out["response"]["steps"]))
    check("script per-step trace events",
          any(e.get("step") == "script" and e.get("script_step") == 1
              and e.get("action") == "workspace.read_file" and e.get("ok") is True
              for e in trace.events))
    r = registry.call("assistant.trace", {"trace_id": trace.trace_id}, root)
    check("script trace readable via assistant.trace",
          any(e.get("step") == "script" for e in r["events"]))

    def expect_error(script, code, step):
        try:
            registry.call("workspace.run_script", {"script": script}, root)
            return None
        except ConsoleError as exc:
            return exc.code == code and exc.step == step

    check("script unknown service",
          expect_error([{"do": "workspace.no_such", "with": {}}],
                       "unknown_service", "registry"))
    check("script nested forbidden",
          expect_error([{"do": "workspace.run_script", "with": {"script": []}}],
                       "nested_script_not_allowed", "contract"))
    check("script forward ref",
          expect_error([
              {"do": "workspace.read_file",
               "with": {"path": {"$ref": "b.path"}}, "as": "a"},
              {"do": "workspace.read_file",
               "with": {"path": "s3_a.txt"}, "as": "b"},
          ], "reference_scope", "contract"))
    check("script missing ref step",
          expect_error([
              {"do": "workspace.read_file",
               "with": {"path": {"$ref": "nope.path"}}, "as": "a"},
          ], "reference_scope", "contract"))
    check("script ref field not in schema",
          expect_error([
              {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
              {"do": "workspace.read_file",
               "with": {"path": {"$ref": "a.nope"}}, "as": "b"},
          ], "invalid_reference", "contract"))
    check("script ref type mismatch",
          expect_error([
              {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
              {"do": "workspace.search_replace",
               "with": {"path": "s3_a.txt",
                        "old_string": {"$ref": "a.size"},
                        "new_string": "x"}},
          ], "reference_type_mismatch", "contract"))
    check("script duplicate step name",
          expect_error([
              {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
              {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
          ], "duplicate_step_name", "contract"))

    trace_fail = trace_store.new("s3-fail")
    try:
        registry.call(
            "workspace.run_script",
            {"script": [
                {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
                {"do": "workspace.read_file",
                 "with": {"path": "missing_script.txt"}, "as": "b"},
                {"do": "workspace.search_replace",
                 "with": {"path": "late.txt", "old_string": "", "new_string": "x"}},
            ]},
            root,
            trace_fail,
        )
        check("script runtime failure fail-closed", False)
    except ConsoleError as exc:
        check("script runtime failure fail-closed",
              exc.code == "not_found" and exc.step == "target"
              and exc.upstream is not None
              and exc.upstream.get("script_step") == 2)
        check("script no later steps after failure",
              not (Path(root) / "late.txt").exists())
        check("script failure trace event",
              any(e.get("step") == "script" and e.get("script_step") == 2
                  and e.get("ok") is False and e.get("code") == "not_found"
                  for e in trace_fail.events))

    check("script step count limit (schema)",
          expect_error(
              [
                  {"do": "workspace.read_file", "with": {"path": "s3_a.txt"},
                   "as": f"a{i}"}
                  for i in range(script_runner.MAX_SCRIPT_STEPS + 1)
              ],
              "invalid_arguments", "contract"))
    r = registry.call(
        "workspace.run_script",
        {"script": [
            {"do": "workspace.read_file", "with": {"path": "s3_a.txt"},
             "as": f"a{i}"}
            for i in range(script_runner.MAX_SCRIPT_STEPS)
        ]},
        root,
    )
    check("script max step count ok",
          r["result"]["content"] == "hello script"
          and len(r["steps"]) == script_runner.MAX_SCRIPT_STEPS)

    real_monotonic = script_runner.time.monotonic
    call_count = {"n": 0}

    def fake_monotonic():
        call_count["n"] += 1
        return call_count["n"] * 100.0

    script_runner.time.monotonic = fake_monotonic
    try:
        check("script wallclock limit",
              expect_error(
                  [{"do": "workspace.read_file",
                    "with": {"path": "s3_a.txt"}, "as": "a"}],
                  "script_timeout", "execute"))
    finally:
        script_runner.time.monotonic = real_monotonic

    real_response_limit = script_runner.MAX_SCRIPT_RESPONSE_BYTES
    script_runner.MAX_SCRIPT_RESPONSE_BYTES = 1
    try:
        check("script response limit (named)",
              expect_error(
                  [{"do": "workspace.read_file",
                    "with": {"path": "s3_a.txt"}, "as": "a"}],
                  "script_response_limit", "execute"))
    finally:
        script_runner.MAX_SCRIPT_RESPONSE_BYTES = real_response_limit

    script_runner.MAX_SCRIPT_RESPONSE_BYTES = 1
    try:
        check("script response limit (unnamed)",
              expect_error(
                  [{"do": "workspace.read_file",
                    "with": {"path": "s3_a.txt"}}],
                  "script_response_limit", "execute"))
    finally:
        script_runner.MAX_SCRIPT_RESPONSE_BYTES = real_response_limit

    (Path(root) / "s3_big.txt").write_text("x" * 700, encoding="utf-8")
    script_runner.MAX_SCRIPT_RESPONSE_BYTES = 1200
    try:
        check("script response limit final result",
              expect_error(
                  [{"do": "workspace.read_file",
                    "with": {"path": "s3_big.txt"}, "as": "a"}],
                  "script_response_limit", "execute"))
    finally:
        script_runner.MAX_SCRIPT_RESPONSE_BYTES = real_response_limit

    try:
        registry.call(
            "workspace.run_script",
            {"script": [
                {"do": "workspace.list_dir", "with": {"path": "s3_empty"}, "as": "d"},
                {"do": "workspace.read_file",
                 "with": {"path": {"$ref": "d.entries.0.name"}}, "as": "first"},
            ]},
            root,
        )
        check("script runtime ref failure structured", False)
    except ConsoleError as exc:
        check("script runtime ref failure structured",
              exc.code == "invalid_reference" and exc.step == "execute"
              and exc.upstream is not None
              and exc.upstream.get("script_step") == 2
              and exc.upstream.get("code") == "invalid_reference"
              and isinstance(exc.upstream.get("upstream"), dict)
              and exc.upstream["upstream"].get("ref") == "d.entries.0.name")

    r = registry.call(
        "workspace.run_script",
        {"script": [
            {"do": "workspace.read_file", "with": {"path": "s3_a.txt"}, "as": "a"},
            {"do": "workspace.search_replace",
             "with": {"path": "s3_out.txt", "old_string": "",
                      "new_string": {"$ref": "a.content"}}},
            {"do": "workspace.read_file", "with": {"path": "s3_out.txt"}, "as": "out"},
        ]},
        root,
    )
    check("script ref content into edit",
          r["result"]["content"] == "hello script")

    r = registry.call(
        "workspace.run_script",
        {"script": [
            {"do": "workspace.list_dir", "with": {"path": "s3_dir"}, "as": "d"},
            {"do": "workspace.read_file",
             "with": {"path": {"$ref": "d.entries.0.name"}}, "as": "first"},
        ]},
        root,
    )
    check("script ref into list entry",
          r["result"]["content"] == "hello script"
          and r["result"]["path"].endswith("s3_a.txt"))


def script_benchmark_tests(root: str) -> None:
    corpus_path = Path(__file__).with_name("sample3_corpus.json")
    corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
    n = script_benchmark.validate_corpus(corpus)
    check("sample3 corpus validation", n == len(corpus["cases"]) and n == 8)
    registry = ServiceRegistry()
    trace_store = TraceStore()
    register_all(registry, trace_store)
    result = script_benchmark.run_corpus(corpus, Path(root) / "bench3", registry)
    base = result["metrics"]["baseline"]
    cand = result["metrics"]["candidate"]
    check("sample3 benchmark baseline all succeed",
          base["successes"] == 8 and base["success_rate"] == 1.0)
    check("sample3 benchmark candidate all succeed",
          cand["successes"] == 8 and cand["success_rate"] == 1.0)
    check("sample3 candidate one round per case",
          cand["total_rounds"] == 8 and cand["avg_rounds"] == 1.0)
    check("sample3 candidate rounds <= baseline",
          cand["total_rounds"] <= base["total_rounds"])
    check("sample3 candidate strictly fewer rounds",
          cand["total_rounds"] < base["total_rounds"])
    check("sample3 pass criteria", all(result["pass_criteria"].values()))

    out = Path(root) / "bench3" / "sample3_result.json"
    script_benchmark.write_result_artifact(result, out, overwrite=False)
    check("sample3 result artifact written", out.exists())
    try:
        script_benchmark.write_result_artifact(result, out, overwrite=False)
        check("sample3 result artifact no-overwrite", False)
    except FileExistsError:
        check("sample3 result artifact no-overwrite", True)
    script_benchmark.write_result_artifact(result, out, overwrite=True)
    reloaded = json.loads(out.read_text(encoding="utf-8"))
    check("sample3 result artifact round-trip",
          reloaded["metrics"] == result["metrics"]
          and reloaded["pass_criteria"] == result["pass_criteria"])


def benchmark_tests(root: str) -> None:
    corpus_path = Path(__file__).with_name("sample2_corpus.json")
    import json as _json

    corpus = _json.loads(corpus_path.read_text(encoding="utf-8"))
    n = benchmark.validate_corpus(corpus)
    check("corpus validation", n == len(corpus["cases"]) and n == 10)
    result = benchmark.run_corpus(corpus, Path(root) / "bench")
    base = result["metrics"]["baseline"]
    cand = result["metrics"]["candidate"]
    check("benchmark baseline all succeed", base["successes"] == 10 and base["success_rate"] == 1.0)
    check("benchmark candidate all succeed", cand["successes"] == 10 and cand["success_rate"] == 1.0)
    check("benchmark candidate rounds <= baseline",
          cand["total_rounds"] <= base["total_rounds"] and cand["avg_rounds"] <= base["avg_rounds"])
    check("benchmark candidate saves rounds", base["total_rounds"] == 18 and cand["total_rounds"] == 10)
    check("benchmark pass criteria", result["pass_criteria"]["candidate_success_rate_gte_baseline"]
          and result["pass_criteria"]["candidate_avg_rounds_lte_baseline"])

    out = Path(root) / "bench" / "sample2_result.json"
    benchmark.write_result_artifact(result, out, overwrite=False)
    check("result artifact written", out.exists())
    try:
        benchmark.write_result_artifact(result, out, overwrite=False)
        check("result artifact no-overwrite", False)
    except FileExistsError:
        check("result artifact no-overwrite", True)
    benchmark.write_result_artifact(result, out, overwrite=True)
    reloaded = _json.loads(out.read_text(encoding="utf-8"))
    check("result artifact round-trip",
          reloaded["metrics"] == result["metrics"]
          and reloaded["pass_criteria"] == result["pass_criteria"])


def main() -> int:
    with tempfile.TemporaryDirectory() as tmp:
        in_process_tests(tmp)
        editor_tests(tmp)
        script_tests(tmp)
        benchmark_tests(tmp)
        script_benchmark_tests(tmp)
        subprocess_tests(tmp)
    print(f"RESULT pass={PASS} fail={FAIL}")
    return 1 if FAIL else 0


if __name__ == "__main__":
    raise SystemExit(main())
