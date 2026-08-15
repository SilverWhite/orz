"""Sample 2: deterministic editor executor + rule-layer hunk generation.

Two surfaces:

- ``workspace.search_replace``: apply a direct hunk (``old_string`` /
  ``new_string``) with exact matching, unique-match-by-default,
  ``replace_all`` opt-in, CRLF-aware matching, the same fixed decode chain
  as read_file, and atomic writes.
- ``workspace.replace_by_intent``: a deterministic rule layer turns a small
  quoted intent grammar into a hunk, then applies it through the same
  executor.  No model participates in hunk generation.

The contract mirrors the production SearchReplace tool in
``orz/crates/codegen/orz-tools/src/implementations/grok_build/search_replace``
with one stricter default: an empty ``old_string`` is create-only and never
silently overwrites an existing non-empty file.
"""

from __future__ import annotations

import os
import re
import tempfile
from pathlib import Path
from typing import Any

from actions import (
    MAX_READ_BYTES,
    ExecutionFailed,
    InvalidTarget,
    NotFound,
    decode_text,
    resolve_path,
)
from service_registry import ConsoleError, InvalidArguments, Service


class EditTooLarge(ConsoleError):
    code = "edit_too_large"


class NoMatch(ConsoleError):
    code = "no_match"


class AmbiguousMatch(ConsoleError):
    code = "ambiguous_match"


class UnparsableIntent(ConsoleError):
    code = "unparsable_intent"


class EmptyOldStringGuard(ConsoleError):
    code = "empty_old_string_not_allowed"


def _read_target(target: Path) -> tuple[str, bool]:
    if not target.exists():
        raise NotFound(
            f"file not found: {target}",
            step="target",
            upstream={"resolved_path": str(target)},
        )
    if target.is_dir():
        raise InvalidTarget(
            f"target is a directory: {target}",
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
    if len(raw) > MAX_READ_BYTES:
        raise EditTooLarge(
            f"edit target exceeds {MAX_READ_BYTES} bytes",
            step="target",
            upstream={"resolved_path": str(target), "size": len(raw)},
        )
    content, _encoding = decode_text(raw)
    return content, "\r\n" in content


def _match_text(content: str, old_string: str) -> str:
    """CRLF-aware matching: old_string without \\r matches LF-only text."""
    if "\r\n" in content and "\r" not in old_string:
        return content.replace("\r\n", "\n")
    return content


def _restore_line_endings(result: str, had_crlf: bool, old_string: str) -> str:
    if had_crlf and "\r" not in old_string:
        return result.replace("\r\n", "\n").replace("\n", "\r\n")
    return result


def _write_atomic(target: Path, payload: bytes) -> None:
    fd, tmp_name = tempfile.mkstemp(
        prefix=".search_replace_", suffix=".tmp", dir=str(target.parent)
    )
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(payload)
            fh.flush()
            os.fsync(fh.fileno())
        os.replace(tmp_name, target)
    except OSError as exc:
        try:
            os.unlink(tmp_name)
        except OSError:
            pass
        raise ExecutionFailed(
            f"write failed: {exc}",
            step="execute",
            upstream={"resolved_path": str(target)},
        ) from exc


def _apply_hunk(
    target_path: str,
    old_string: str,
    new_string: str,
    replace_all: bool,
    allow_root: str,
) -> dict[str, Any]:
    target = resolve_path(target_path, allow_root)
    if old_string == new_string:
        raise InvalidArguments(
            "old_string and new_string must differ",
            step="contract",
        )
    if old_string == "":
        if target.exists():
            content, _ = _read_target(target)
            if content:
                raise EmptyOldStringGuard(
                    "empty old_string may only create a new file or fill an empty one",
                    step="contract",
                    upstream={"resolved_path": str(target)},
                )
        payload = new_string.encode("utf-8")
        _write_atomic(target, payload)
        return {
            "path": str(target),
            "old_string": "",
            "new_string": new_string,
            "applied": 1,
            "replaced_all": False,
            "size": len(payload),
            "encoding": "utf-8",
            "crlf_normalized": False,
        }
    content, had_crlf = _read_target(target)
    text = _match_text(content, old_string)
    count = text.count(old_string)
    if count == 0:
        raise NoMatch(
            f"old_string not found in file: {target_path}",
            step="execute",
            upstream={"resolved_path": str(target), "matches": 0},
        )
    if count > 1 and not replace_all:
        raise AmbiguousMatch(
            f"old_string matches {count} places; add context or set replace_all",
            step="execute",
            upstream={"resolved_path": str(target), "matches": count},
        )
    result = text.replace(old_string, new_string) if replace_all else text.replace(old_string, new_string, 1)
    result = _restore_line_endings(result, had_crlf, old_string)
    payload = result.encode("utf-8")
    _write_atomic(target, payload)
    return {
        "path": str(target),
        "old_string": old_string,
        "new_string": new_string,
        "applied": count,
        "replaced_all": replace_all,
        "size": len(payload),
        "encoding": "utf-8",
        "crlf_normalized": had_crlf and "\r" not in old_string,
    }


# --- Intent -> rule-layer hunk -------------------------------------------

_CURLY_QUOTES = {
    "\u201c": '"',
    "\u201d": '"',
    "\u2018": "'",
    "\u2019": "'",
}
_ESCAPES = {
    r"\n": "\n",
    r"\t": "\t",
    r"\\": "\\",
    r"\"": '"',
    r"\'": "'",
}
_REPLACE_ALL_MARKERS = ("everywhere", "all occurrences", "全部", "所有", "到处")


def _normalize_intent(text: str) -> str:
    return "".join(_CURLY_QUOTES.get(ch, ch) for ch in text)


def _unescape_quoted(value: str) -> str:
    for key, repl in _ESCAPES.items():
        value = value.replace(key, repl)
    return value


_EN_PATTERNS = [
    re.compile(r'replace\s+(["\'])(.*?)\1\s+with\s+(["\'])(.*?)\3', re.IGNORECASE),
    re.compile(r'change\s+(["\'])(.*?)\1\s+to\s+(["\'])(.*?)\3', re.IGNORECASE),
]
_ZH_PATTERNS = [
    re.compile(r'把\s*(["\'])(.*?)\1\s*(?:替换为|改成|改为)\s*(["\'])(.*?)\3'),
    re.compile(r'将\s*(["\'])(.*?)\1\s*(?:替换为|改成|改为)\s*(["\'])(.*?)\3'),
    re.compile(r'替换\s*(["\'])(.*?)\1\s*为\s*(["\'])(.*?)\3'),
]


def parse_intent(intent: str) -> tuple[str, str, bool]:
    """Deterministic intent -> (old_string, new_string, replace_all).

    Grammar (English): replace "X" with "Y" / change 'X' to 'Y'
    Grammar (Chinese): 把"X"替换为"Y" / 将'X'改成'Y' / 替换"X"为"Y"
    A marker such as "everywhere" / "all occurrences" / 全部 / 所有 / 到处
    turns on replace_all.
    """
    if not isinstance(intent, str) or not intent.strip():
        raise UnparsableIntent("intent must be a non-empty string", step="contract")
    norm = _normalize_intent(intent.strip())
    match = None
    for pattern in (*_EN_PATTERNS, *_ZH_PATTERNS):
        match = pattern.search(norm)
        if match is not None:
            break
    if match is None:
        raise UnparsableIntent(
            "intent grammar not recognized; use replace/change with quoted text",
            step="contract",
        )
    old_string = _unescape_quoted(match.group(2))
    new_string = _unescape_quoted(match.group(4))
    if not old_string or not new_string:
        raise UnparsableIntent("intent quotes must contain non-empty text", step="contract")
    replace_all = any(marker in norm for marker in _REPLACE_ALL_MARKERS)
    return old_string, new_string, replace_all


# --- Service handlers ------------------------------------------------------

_RESPONSE_SCHEMA = {
    "type": "object",
    "properties": {
        "path": {"type": "string"},
        "old_string": {"type": "string"},
        "new_string": {"type": "string"},
        "applied": {"type": "integer"},
        "replaced_all": {"type": "boolean"},
        "size": {"type": "integer"},
        "encoding": {"type": "string"},
        "crlf_normalized": {"type": "boolean"},
    },
    "required": [
        "path",
        "old_string",
        "new_string",
        "applied",
        "replaced_all",
        "size",
        "encoding",
        "crlf_normalized",
    ],
    "additionalProperties": False,
}


def search_replace(
    data: dict[str, Any], allow_root: str, trace: Any = None
) -> dict[str, Any]:
    return _apply_hunk(
        data["path"],
        data["old_string"],
        data["new_string"],
        bool(data.get("replace_all", False)),
        allow_root,
    )


def replace_by_intent(
    data: dict[str, Any], allow_root: str, trace: Any = None
) -> dict[str, Any]:
    old_string, new_string, replace_all = parse_intent(data["intent"])
    response = _apply_hunk(
        data["path"], old_string, new_string, replace_all, allow_root
    )
    response["intent"] = data["intent"]
    return response


def register_editor_services(registry) -> None:
    registry.register(
        Service(
            name="workspace.search_replace",
            input_schema={
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "old_string": {"type": "string"},
                    "new_string": {"type": "string"},
                    "replace_all": {"type": "boolean"},
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": False,
            },
            handler=search_replace,
            response_schema=_RESPONSE_SCHEMA,
            description=(
                "Apply an exact search/replace hunk to a file inside the allow "
                "root (unique match unless replace_all)."
            ),
        )
    )
    registry.register(
        Service(
            name="workspace.replace_by_intent",
            input_schema={
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "intent": {"type": "string"},
                },
                "required": ["path", "intent"],
                "additionalProperties": False,
            },
            handler=replace_by_intent,
            response_schema={
                **_RESPONSE_SCHEMA,
                "properties": {**_RESPONSE_SCHEMA["properties"], "intent": {"type": "string"}},
                "required": [*_RESPONSE_SCHEMA["required"], "intent"],
            },
            description=(
                "Deterministically turn a quoted edit intent into a hunk and "
                "apply it through workspace.search_replace."
            ),
        )
    )
