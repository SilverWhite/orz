"""Sample 3: deterministic linear script runner (PTC script mode).

A script is a fixed list of registered action instances:

    {"script": [
        {"do": "workspace.read_file", "with": {"path": "a.txt"}, "as": "a"},
        {"do": "workspace.read_file", "with": {"path": {"$ref": "a.path"}}, "as": "b"}
    ]}

- Each step is a registered service call with its own schema validation and
  trace events (reuses ``ServiceRegistry.call``).
- ``as`` names a step output; ``$ref`` references a field of an earlier step
  output (dotted path; numeric list indices are allowed).
- Static checks run before any execution: reference existence, scope
  (earlier named steps only), type match against the referenced step's
  response schema and the consuming input schema, unique names, known
  services, and no nested scripts.
- Limits: step count (JSON schema ``maxItems``), wall clock, cumulative
  response bytes (every step response, named or unnamed, plus the final
  ``result``), and the existing per-request trace cap.  No loops, no
  conditions, no arbitrary code.
"""

from __future__ import annotations

import copy
import json
import time
from typing import Any

from service_registry import ConsoleError, Service, UnknownService


SCRIPT_SERVICE_NAME = "workspace.run_script"
MAX_SCRIPT_STEPS = 20
MAX_SCRIPT_WALLCLOCK_SECONDS = 30.0
MAX_SCRIPT_RESPONSE_BYTES = 4 * 1024 * 1024


class InvalidScript(ConsoleError):
    code = "invalid_script"


class DuplicateStepName(ConsoleError):
    code = "duplicate_step_name"


class InvalidReference(ConsoleError):
    code = "invalid_reference"


class ReferenceScope(ConsoleError):
    code = "reference_scope"


class ReferenceTypeMismatch(ConsoleError):
    code = "reference_type_mismatch"


class NestedScript(ConsoleError):
    code = "nested_script_not_allowed"


class ScriptTimeout(ConsoleError):
    code = "script_timeout"


class ScriptResponseLimit(ConsoleError):
    code = "script_response_limit"


def _schema_type(schema: Any) -> Any:
    if isinstance(schema, dict):
        return schema.get("type")
    return None


def _types_compatible(expected: Any, actual: Any) -> bool:
    if expected is None or actual is None:
        return True
    expected_types = expected if isinstance(expected, list) else [expected]
    actual_types = actual if isinstance(actual, list) else [actual]
    for exp in expected_types:
        for act in actual_types:
            if exp == act or {exp, act} <= {"integer", "number"}:
                return True
    return False


def _collect_refs(
    value: Any,
    schema: Any,
    path: str,
    out: list[tuple[str, str, Any]],
) -> None:
    """Collect ``{"$ref": ...}`` leaves and the expected type at their path."""
    if isinstance(value, dict):
        if set(value) == {"$ref"}:
            ref = value["$ref"]
            if not isinstance(ref, str) or not ref.strip():
                raise InvalidReference(
                    f"{path}: $ref must be a non-empty string",
                    step="contract",
                )
            out.append((path, ref, _schema_type(schema)))
            return
        props: Any = None
        additional: Any = None
        if isinstance(schema, dict):
            props = schema.get("properties")
            additional = schema.get("additionalProperties")
        for key, val in value.items():
            child_schema: Any = None
            if isinstance(props, dict):
                child_schema = props.get(key)
            if child_schema is None and isinstance(additional, dict):
                child_schema = additional
            _collect_refs(val, child_schema, f"{path}.{key}", out)
    elif isinstance(value, list):
        items: Any = schema.get("items") if isinstance(schema, dict) else None
        for idx, val in enumerate(value):
            _collect_refs(val, items, f"{path}[{idx}]", out)


def _resolve_leaf_type(response_schema: dict[str, Any], parts: list[str]) -> Any:
    """Resolve the JSON-schema type of a dotted field path in a response."""
    schema: Any = response_schema
    for idx, part in enumerate(parts):
        if not isinstance(schema, dict):
            return None
        props = schema.get("properties")
        items = schema.get("items")
        is_last = idx == len(parts) - 1
        if isinstance(props, dict) and part in props:
            if is_last:
                return _schema_type(props[part])
            schema = props[part]
            continue
        if part.isdigit() and isinstance(items, dict):
            if is_last:
                return _schema_type(items)
            schema = items
            continue
        return None
    return None


def _static_validate(
    script: list[dict[str, Any]],
    registry,
) -> list[dict[str, Any]]:
    """Validate the whole script before any execution (fail-closed)."""
    available_names: set[str] = set()
    named_services: dict[str, Service] = {}
    steps: list[dict[str, Any]] = []
    for idx, step in enumerate(script, start=1):
        do = step["do"]
        name = step.get("as")
        with_data = step.get("with", {})
        if name is not None and name in available_names:
            raise DuplicateStepName(
                f"script[{idx}].as duplicates an earlier step name: {name}",
                step="contract",
                upstream={"name": name, "script_step": idx},
            )
        service = registry.get(do)
        if service is None:
            raise UnknownService(
                f"script[{idx}].do: unknown service: {do}",
                step="registry",
                upstream={"script_step": idx, "action": do},
            )
        if do == SCRIPT_SERVICE_NAME:
            raise NestedScript(
                f"script[{idx}].do: nested scripts are not allowed",
                step="contract",
                upstream={"script_step": idx},
            )
        refs: list[tuple[str, str, Any]] = []
        _collect_refs(with_data, service.input_schema, f"script[{idx}].with", refs)
        for path, ref, expected_type in refs:
            parts = ref.split(".")
            if len(parts) < 2 or any(not part for part in parts):
                raise InvalidReference(
                    f"{path}: invalid $ref shape: {ref}",
                    step="contract",
                    upstream={"script_step": idx, "ref": ref},
                )
            ref_name = parts[0]
            if ref_name not in available_names:
                raise ReferenceScope(
                    f"{path}: $ref must name an earlier step output: {ref}",
                    step="contract",
                    upstream={"script_step": idx, "ref": ref},
                )
            leaf_type = _resolve_leaf_type(
                named_services[ref_name].response_schema, parts[1:]
            )
            if leaf_type is None:
                raise InvalidReference(
                    f"{path}: referenced field not in {ref_name} response schema: {ref}",
                    step="contract",
                    upstream={"script_step": idx, "ref": ref},
                )
            if not _types_compatible(expected_type, leaf_type):
                raise ReferenceTypeMismatch(
                    f"{path}: $ref type mismatch: expected {expected_type}, "
                    f"referenced {leaf_type} ({ref})",
                    step="contract",
                    upstream={
                        "script_step": idx,
                        "ref": ref,
                        "expected": expected_type,
                        "actual": leaf_type,
                    },
                )
        if name is not None:
            available_names.add(name)
            named_services[name] = service
        steps.append(
            {
                "index": idx,
                "do": do,
                "with": with_data,
                "name": name,
                "service": service,
            }
        )
    return steps


def _substitute(value: Any, outputs: dict[str, Any]) -> Any:
    if isinstance(value, dict):
        if set(value) == {"$ref"}:
            ref = value["$ref"]
            parts = ref.split(".")
            try:
                cur: Any = outputs[parts[0]]
                for part in parts[1:]:
                    if isinstance(cur, dict):
                        cur = cur[part]
                    elif isinstance(cur, list) and part.isdigit():
                        cur = cur[int(part)]
                    else:
                        raise InvalidReference(
                            f"runtime $ref resolution failed: {ref}",
                            step="execute",
                            upstream={"ref": ref},
                        )
                return copy.deepcopy(cur)
            except (KeyError, IndexError, TypeError) as exc:
                raise InvalidReference(
                    f"runtime $ref resolution failed: {ref} ({exc})",
                    step="execute",
                    upstream={"ref": ref},
                ) from exc
        return {key: _substitute(val, outputs) for key, val in value.items()}
    if isinstance(value, list):
        return [_substitute(val, outputs) for val in value]
    return value


def run_script(
    data: dict[str, Any],
    allow_root: str,
    registry,
    trace: Any = None,
) -> dict[str, Any]:
    script = data["script"]
    steps_meta = _static_validate(script, registry)
    started = time.monotonic()
    outputs: dict[str, Any] = {}
    executed: list[dict[str, Any]] = []
    response_bytes = 0
    final_response: dict[str, Any] | None = None
    for meta in steps_meta:
        if time.monotonic() - started > MAX_SCRIPT_WALLCLOCK_SECONDS:
            raise ScriptTimeout(
                f"script exceeded {MAX_SCRIPT_WALLCLOCK_SECONDS:.0f}s wall clock",
                step="execute",
                upstream={"script_step": meta["index"], "action": meta["do"]},
            )
        try:
            with_data = _substitute(meta["with"], outputs)
            response = registry.call(meta["do"], with_data, allow_root, trace)
        except ConsoleError as exc:
            if trace is not None:
                trace.add(
                    step="script",
                    script_step=meta["index"],
                    action=meta["do"],
                    ok=False,
                    code=exc.code,
                    message=str(exc),
                )
            outer = ConsoleError(
                f"script step {meta['index']} ({meta['do']}) failed: {exc}",
                step=exc.step,
                upstream={
                    "script_step": meta["index"],
                    "action": meta["do"],
                    "code": exc.code,
                    "message": str(exc),
                    "upstream": exc.upstream,
                },
            )
            outer.code = exc.code
            raise outer from exc
        entry = {
            "index": meta["index"],
            "do": meta["do"],
            "as": meta["name"],
            "ok": True,
            "response": response,
        }
        entry_bytes = len(json.dumps(entry, ensure_ascii=False).encode("utf-8"))
        if response_bytes + entry_bytes > MAX_SCRIPT_RESPONSE_BYTES:
            raise ScriptResponseLimit(
                f"script response exceeded {MAX_SCRIPT_RESPONSE_BYTES} bytes",
                step="execute",
                upstream={
                    "script_step": meta["index"],
                    "action": meta["do"],
                    "response_bytes": response_bytes + entry_bytes,
                    "limit": MAX_SCRIPT_RESPONSE_BYTES,
                },
            )
        response_bytes += entry_bytes
        if meta["name"] is not None:
            outputs[meta["name"]] = response
        if trace is not None:
            trace.add(
                step="script",
                script_step=meta["index"],
                action=meta["do"],
                ok=True,
                as_name=meta["name"],
            )
        executed.append(entry)
        final_response = response
    response = {"steps": executed, "result": final_response}
    total_bytes = len(json.dumps(response, ensure_ascii=False).encode("utf-8"))
    if total_bytes > MAX_SCRIPT_RESPONSE_BYTES:
        raise ScriptResponseLimit(
            f"script response exceeded {MAX_SCRIPT_RESPONSE_BYTES} bytes "
            "(includes final result duplication)",
            step="execute",
            upstream={
                "response_bytes": total_bytes,
                "limit": MAX_SCRIPT_RESPONSE_BYTES,
            },
        )
    return response


_INPUT_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "script": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "do": {"type": "string"},
                    "with": {"type": "object"},
                    "as": {
                        "type": "string",
                        "pattern": "^[A-Za-z_][A-Za-z0-9_]*$",
                    },
                },
                "required": ["do"],
                "additionalProperties": False,
            },
            "minItems": 1,
            "maxItems": MAX_SCRIPT_STEPS,
        }
    },
    "required": ["script"],
    "additionalProperties": False,
}


_RESPONSE_SCHEMA: dict[str, Any] = {
    "type": "object",
    "properties": {
        "steps": {
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "index": {"type": "integer"},
                    "do": {"type": "string"},
                    "as": {"type": ["string", "null"]},
                    "ok": {"type": "boolean"},
                    "response": {"type": "object"},
                },
                "required": ["index", "do", "as", "ok", "response"],
                "additionalProperties": False,
            },
        },
        "result": {"type": ["object", "null"]},
    },
    "required": ["steps", "result"],
    "additionalProperties": False,
}


def register_script_services(registry) -> None:
    def run_script_handler(
        data: dict[str, Any],
        allow_root: str,
        trace: Any = None,
    ) -> dict[str, Any]:
        return run_script(data, allow_root, registry, trace)

    registry.register(
        Service(
            name=SCRIPT_SERVICE_NAME,
            input_schema=_INPUT_SCHEMA,
            handler=run_script_handler,
            response_schema=_RESPONSE_SCHEMA,
            description=(
                "Execute a deterministic linear script of registered actions "
                "($ref data references, per-step validation, fail-closed)."
            ),
        )
    )
