"""Deterministic service handlers (workspace domain)."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

from service_registry import ConsoleError


MAX_READ_BYTES = 1024 * 1024
MAX_DIR_ENTRIES = 1000
MAX_INDEX_DEPTH = 4
MAX_INDEX_ENTRIES = 2000
SKIP_DIRS = {".git", "target", "node_modules", "__pycache__", ".venv", "venv", ".pytest_cache"}


class OutOfScope(ConsoleError):
    code = "out_of_scope"


class NotFound(ConsoleError):
    code = "not_found"


class InvalidTarget(ConsoleError):
    code = "invalid_target"


class ReadTooLarge(ConsoleError):
    code = "read_too_large"


class ExecutionFailed(ConsoleError):
    code = "execution_failed"


def resolve_path(raw: str, allow_root: str) -> Path:
    if not isinstance(raw, str) or not raw.strip():
        raise InvalidArgumentsPath("path must be a non-empty string", step="target")
    root = Path(allow_root).resolve()
    candidate = Path(raw).expanduser()
    if not candidate.is_absolute():
        candidate = root / candidate
    target = candidate.resolve()
    try:
        target.relative_to(root)
    except ValueError as exc:
        raise OutOfScope(
            f"path outside allow root: {raw}",
            step="target",
            upstream={"resolved_path": str(target)},
        ) from exc
    return target


class InvalidArgumentsPath(ConsoleError):
    code = "invalid_arguments"


def decode_text(data: bytes) -> tuple[str, str]:
    """Fixed decode chain: strip BOM -> UTF-8 strict -> GB18030 -> lossy."""
    if data.startswith(b"\xef\xbb\xbf"):
        body = data[3:]
        try:
            return body.decode("utf-8"), "utf-8-sig"
        except UnicodeDecodeError:
            pass
    try:
        return data.decode("utf-8"), "utf-8"
    except UnicodeDecodeError:
        pass
    try:
        return data.decode("gb18030"), "gb18030"
    except UnicodeDecodeError:
        return data.decode("utf-8", errors="replace"), "utf-8-lossy"


def read_file(data: dict[str, Any], allow_root: str) -> dict[str, Any]:
    target = resolve_path(data["path"], allow_root)
    if target.is_dir():
        raise InvalidTarget(
            f"target is a directory: {data['path']}",
            step="target",
            upstream={"resolved_path": str(target)},
        )
    if not target.exists():
        raise NotFound(
            f"file not found: {data['path']}",
            step="target",
            upstream={"resolved_path": str(target)},
        )
    try:
        raw = target.read_bytes()
    except OSError as exc:
        raise ExecutionFailed(
            f"unreadable: {exc}",
            step="execute",
            upstream={"resolved_path": str(target)},
        ) from exc
    truncated = len(raw) > MAX_READ_BYTES
    if truncated:
        raw = raw[:MAX_READ_BYTES]
    content, encoding = decode_text(raw)
    if truncated:
        content += f"\n[read_file truncated: exceeded {MAX_READ_BYTES} bytes]"
    return {
        "path": str(target),
        "content": content,
        "size": len(content.encode("utf-8")),
        "encoding": encoding,
        "truncated": truncated,
    }


def list_dir(data: dict[str, Any], allow_root: str) -> dict[str, Any]:
    target = resolve_path(data["path"], allow_root)
    if not target.exists():
        raise NotFound(
            f"path not found: {data['path']}",
            step="target",
            upstream={"resolved_path": str(target)},
        )
    if not target.is_dir():
        raise InvalidTarget(
            f"target is not a directory: {data['path']}",
            step="target",
            upstream={"resolved_path": str(target)},
        )
    try:
        entries = sorted(
            (e for e in target.iterdir() if not e.name.startswith(".")),
            key=lambda e: e.name,
        )
    except OSError as exc:
        raise ExecutionFailed(
            f"unreadable: {exc}",
            step="execute",
            upstream={"resolved_path": str(target)},
        ) from exc
    truncated = len(entries) > MAX_DIR_ENTRIES
    entries = entries[:MAX_DIR_ENTRIES]
    return {
        "path": str(target),
        "entries": [{"name": e.name, "is_dir": e.is_dir()} for e in entries],
        "total": len(entries),
        "truncated": truncated,
    }


def index_workspace(allow_root: str) -> dict[str, Any]:
    """Bounded workspace index: relative file paths, deterministic order."""
    root = Path(allow_root).resolve()
    files: list[str] = []
    truncated = False

    def walk(base: Path, depth: int) -> None:
        nonlocal truncated
        if depth > MAX_INDEX_DEPTH or truncated:
            return
        try:
            entries = sorted(base.iterdir(), key=lambda p: p.name)
        except OSError:
            return
        for entry in entries:
            if entry.name.startswith(".") or entry.name in SKIP_DIRS:
                continue
            if entry.is_dir():
                walk(entry, depth + 1)
            elif entry.is_file():
                if len(files) >= MAX_INDEX_ENTRIES:
                    truncated = True
                    return
                files.append(entry.relative_to(root).as_posix())

    walk(root, 0)
    return {"root": str(root), "files": files, "total": len(files), "truncated": truncated}


def workspace_index(data: dict[str, Any], allow_root: str) -> dict[str, Any]:
    return index_workspace(allow_root)


def register_all(registry, trace_store=None) -> None:
    from service_registry import Service

    def assistant_trace(data: dict[str, Any], allow_root: str) -> dict[str, Any]:
        if trace_store is None:
            raise TraceUnavailable("trace store unavailable", step="registry")
        trace = trace_store.get(data["trace_id"])
        if trace is None:
            raise NotFound(
                f"trace not found: {data['trace_id']}",
                step="registry",
            )
        events, truncated = trace.tail(int(data.get("tail", 20)))
        return {
            "trace_id": trace.trace_id,
            "request_id": trace.request_id,
            "events": events,
            "truncated": truncated,
        }

    class TraceUnavailable(ConsoleError):
        code = "trace_unavailable"

    registry.register(
        Service(
            name="assistant.trace",
            input_schema={
                "type": "object",
                "properties": {
                    "trace_id": {"type": "string"},
                    "tail": {"type": "integer", "minimum": 1, "maximum": 200},
                },
                "required": ["trace_id"],
                "additionalProperties": False,
            },
            handler=assistant_trace,
            response_schema={
                "type": "object",
                "properties": {
                    "trace_id": {"type": "string"},
                    "request_id": {"type": ["string", "null"]},
                    "events": {"type": "array", "items": {"type": "object"}},
                    "truncated": {"type": "boolean"},
                },
                "required": ["trace_id", "request_id", "events", "truncated"],
                "additionalProperties": False,
            },
            description="Read a bounded execution trace by id (model-visible log).",
        )
    )

    registry.register(
        Service(
            name="workspace.read_file",
            input_schema={
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
                "additionalProperties": False,
            },
            handler=read_file,
            response_schema={
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"},
                    "size": {"type": "integer"},
                    "encoding": {"type": "string"},
                    "truncated": {"type": "boolean"},
                },
                "required": ["path", "content", "size", "encoding", "truncated"],
                "additionalProperties": False,
            },
            description="Read a text file inside the allow root.",
        )
    )
    registry.register(
        Service(
            name="workspace.list_dir",
            input_schema={
                "type": "object",
                "properties": {"path": {"type": "string"}},
                "required": ["path"],
                "additionalProperties": False,
            },
            handler=list_dir,
            response_schema={
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "entries": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": {"type": "string"},
                                "is_dir": {"type": "boolean"},
                            },
                            "required": ["name", "is_dir"],
                            "additionalProperties": False,
                        },
                    },
                    "total": {"type": "integer"},
                    "truncated": {"type": "boolean"},
                },
                "required": ["path", "entries", "total", "truncated"],
                "additionalProperties": False,
            },
            description="List entries of a directory inside the allow root.",
        )
    )
    registry.register(
        Service(
            name="workspace.index",
            input_schema={
                "type": "object",
                "properties": {},
                "additionalProperties": False,
            },
            handler=workspace_index,
            response_schema={
                "type": "object",
                "properties": {
                    "root": {"type": "string"},
                    "files": {
                        "type": "array",
                        "items": {"type": "string"},
                        "uniqueItems": True,
                    },
                    "total": {"type": "integer"},
                    "truncated": {"type": "boolean"},
                },
                "required": ["root", "files", "total", "truncated"],
                "additionalProperties": False,
            },
            description="Bounded workspace file index (slot-table source).",
        )
    )
