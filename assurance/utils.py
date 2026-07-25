from __future__ import annotations

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import tempfile
from typing import Any

import rfc8785

from .errors import AssuranceError


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def canonical_bytes(value: Any) -> bytes:
    try:
        return rfc8785.dumps(value)
    except Exception as exc:
        raise AssuranceError(f"RFC 8785 canonicalization failed: {exc}") from exc


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError) as exc:
        raise AssuranceError(f"cannot read JSON {path}: {exc}") from exc


def atomic_write_bytes(path: Path, value: bytes, *, overwrite: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not overwrite:
        raise AssuranceError(f"refusing to overwrite existing file: {path}")
    temporary: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as handle:
            temporary = handle.name
            handle.write(value)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except Exception:
        if temporary is not None:
            try:
                Path(temporary).unlink(missing_ok=True)
            except OSError:
                pass
        raise


def exclusive_create_bytes(path: Path, value: bytes) -> None:
    """Create a small claim/marker exactly once, including under contention."""
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor: int | None = None
    created = False
    try:
        descriptor = os.open(
            path,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_BINARY", 0),
            0o600,
        )
        created = True
        with os.fdopen(descriptor, "wb") as handle:
            descriptor = None
            handle.write(value)
            handle.flush()
            os.fsync(handle.fileno())
    except FileExistsError as exc:
        raise AssuranceError(f"exclusive artifact already exists: {path}") from exc
    except Exception:
        if descriptor is not None:
            os.close(descriptor)
        if created:
            try:
                path.unlink(missing_ok=True)
            except OSError:
                pass
        raise


def atomic_write_json(path: Path, value: Any, *, overwrite: bool = False) -> None:
    encoded = (
        json.dumps(
            value,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
            allow_nan=False,
        ).encode("utf-8")
        + b"\n"
    )
    atomic_write_bytes(path, encoded, overwrite=overwrite)


def safe_relative_path(value: str) -> PurePosixPath:
    if not value or "\\" in value:
        raise AssuranceError(f"unsafe relative path: {value!r}")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise AssuranceError(f"unsafe relative path: {value!r}")
    if path.parts and ":" in path.parts[0]:
        raise AssuranceError(f"drive-qualified path is forbidden: {value!r}")
    return path


def is_link_or_reparse(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except OSError as exc:
        raise AssuranceError(f"cannot inspect path {path}: {exc}") from exc
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    attributes = getattr(metadata, "st_file_attributes", 0)
    return stat.S_ISLNK(metadata.st_mode) or bool(attributes & reparse_flag)


def require_within(path: Path, root: Path, *, must_exist: bool) -> Path:
    try:
        root_resolved = root.resolve(strict=True)
        path_resolved = path.resolve(strict=must_exist)
        path_resolved.relative_to(root_resolved)
    except (OSError, ValueError) as exc:
        raise AssuranceError(f"path escapes declared root: {path}") from exc
    return path_resolved


def require_no_linked_ancestors(path: Path, root: Path) -> None:
    root_resolved = require_within(root, root, must_exist=True)
    if is_link_or_reparse(root_resolved):
        raise AssuranceError(f"declared root is linked or reparse-backed: {root}")
    try:
        relative = path.relative_to(root)
    except ValueError as exc:
        raise AssuranceError(f"path is not lexically below declared root: {path}") from exc
    current = root
    for part in relative.parts:
        current = current / part
        if not current.exists():
            break
        if is_link_or_reparse(current):
            raise AssuranceError(f"linked or reparse-backed path is forbidden: {current}")
