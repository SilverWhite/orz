"""Classical Console: stdin/stdout JSON-lines service console (thin seam)."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import yaml

from actions import index_workspace, register_all
from service_registry import (
    ConsoleError,
    IntentUnavailable,
    InvalidArguments,
    ProtocolError,
    ServiceRegistry,
    UnknownIntent,
)
from trace import Trace, TraceStore

try:
    from hassil import recognize
    from hassil.intents import Intents
except ImportError:  # pragma: no cover
    recognize = None
    Intents = None


def load_intents(allow_root: str) -> list[Any]:
    if Intents is None:
        return []
    files = index_workspace(allow_root)["files"]
    loaded: list[Any] = []
    base = Path(__file__).resolve().parent
    for path in (base / "intents.en.yaml", base / "intents.zh.yaml"):
        try:
            with open(path, encoding="utf-8") as fh:
                data = yaml.safe_load(fh)
        except FileNotFoundError:
            continue
        data["lists"] = {"file": {"values": files, "wildcard": False}}
        loaded.append(Intents.from_dict(data))
    return loaded


def handle_request(
    req: dict[str, Any],
    registry: ServiceRegistry,
    allow_root: str,
    trace: Trace,
) -> dict[str, Any]:
    if req.get("proto") != 1:
        raise ProtocolError("unsupported proto", step="protocol")
    req_id = req.get("id")
    if not isinstance(req_id, str) or not req_id:
        raise ProtocolError("missing id", step="protocol")
    if "service" in req and "text" in req:
        raise ProtocolError("service and text are mutually exclusive", step="protocol")
    trace.add(step="protocol", ok=True)
    if "service" in req:
        service = req["service"]
        data = req.get("data", {})
        if not isinstance(service, str) or not isinstance(data, dict):
            raise InvalidArguments(
                "service must be str, data must be object",
                step="protocol",
            )
        response = registry.call(service, data, allow_root, trace)
    elif "text" in req:
        response = handle_intent(str(req["text"]), registry, allow_root, trace)
    else:
        raise ProtocolError("expected service or text", step="protocol")
    return {"proto": 1, "id": req_id, "ok": True, "response": response}


def handle_intent(
    text: str,
    registry: ServiceRegistry,
    allow_root: str,
    trace: Trace,
) -> dict[str, Any]:
    if recognize is None:
        raise IntentUnavailable("hassil not installed", step="intent")
    intents = load_intents(allow_root)
    if not intents:
        raise IntentUnavailable("no intent grammars loaded", step="intent")
    result = None
    for grammars in intents:
        result = recognize(text, grammars)
        if result is not None:
            break
    if result is None:
        raise UnknownIntent("no intent matched", step="intent")
    trace.add(step="intent", ok=True, intent=result.intent.name)
    if result.intent.name == "ReadFile":
        entity = result.entities.get("file")
        if entity is None or not entity.value:
            raise InvalidArguments("missing file slot", step="intent")
        return registry.call(
            "workspace.read_file",
            {"path": str(entity.value)},
            allow_root,
            trace,
        )
    raise UnknownIntent(f"unhandled intent: {result.intent.name}", step="intent")


def build_error_envelope(
    req_id: str | None,
    exc: ConsoleError,
    trace: Trace,
    tail: int = 10,
) -> dict[str, Any]:
    error: dict[str, Any] = {
        "step": exc.step,
        "code": exc.code,
        "message": str(exc),
        "upstream": exc.upstream,
        "trace_id": trace.trace_id,
    }
    if exc.step == "execute":
        events, _ = trace.tail(tail)
        error["trace"] = events
    return {"proto": 1, "id": req_id, "ok": False, "error": error}


def run_request(
    req: dict[str, Any],
    registry: ServiceRegistry,
    allow_root: str,
    trace: Trace,
) -> dict[str, Any]:
    try:
        out = handle_request(req, registry, allow_root, trace)
        out["trace_id"] = trace.trace_id
        return out
    except ConsoleError as exc:
        trace.add(
            step=exc.step,
            action=req.get("service"),
            ok=False,
            code=exc.code,
            message=str(exc),
            upstream=exc.upstream,
        )
        return build_error_envelope(req.get("id"), exc, trace)
    except Exception as exc:  # fail-closed: never leak an unformatted failure
        trace.add(
            step="execute",
            ok=False,
            code="internal_error",
            message=f"unexpected failure: {exc}",
        )
        return build_error_envelope(
            req.get("id"),
            ConsoleError(f"unexpected failure: {exc}", step="execute"),
            trace,
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--allow-root", required=True)
    args = parser.parse_args()
    registry = ServiceRegistry()
    trace_store = TraceStore()
    register_all(registry, trace_store)
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        trace = trace_store.new(None)
        try:
            req = json.loads(line)
            if not isinstance(req, dict):
                raise ValueError("request must be an object")
        except (json.JSONDecodeError, ValueError) as exc:
            trace.add(step="protocol", ok=False, code="protocol_error", message=str(exc))
            out = build_error_envelope(
                None,
                ProtocolError(str(exc), step="protocol"),
                trace,
            )
            print(json.dumps(out, ensure_ascii=False), flush=True)
            continue
        trace.request_id = req.get("id")
        out = run_request(req, registry, args.allow_root, trace)
        print(json.dumps(out, ensure_ascii=False), flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
