from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import tempfile
from typing import Any

import rfc8785

from .errors import PrototypeError


def utc_now() -> str:
    from datetime import datetime, timezone

    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def canonical_bytes(value: Any) -> bytes:
    try:
        return rfc8785.dumps(value)
    except Exception as exc:  # pragma: no cover - dependency error detail
        raise PrototypeError(f"RFC 8785 canonicalization failed: {exc}") from exc


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def load_json(path: Path) -> Any:
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError) as exc:
        raise PrototypeError(f"cannot read JSON {path}: {exc}") from exc


def atomic_write_bytes(path: Path, data: bytes, *, overwrite: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and not overwrite:
        raise PrototypeError(f"refusing to overwrite existing output: {path}")
    temp_name: str | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="wb", dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", delete=False
        ) as handle:
            temp_name = handle.name
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temp_name, path)
    except Exception:
        if temp_name:
            try:
                Path(temp_name).unlink(missing_ok=True)
            except OSError:
                pass
        raise


def atomic_write_json(path: Path, value: Any, *, overwrite: bool = False) -> None:
    data = json.dumps(
        value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False
    ).encode("utf-8") + b"\n"
    atomic_write_bytes(path, data, overwrite=overwrite)


def safe_relative_path(value: str) -> PurePosixPath:
    if not value or "\\" in value:
        raise PrototypeError(f"unsafe relative path: {value!r}")
    path = PurePosixPath(value)
    if path.is_absolute() or any(part in {"", ".", ".."} for part in path.parts):
        raise PrototypeError(f"unsafe relative path: {value!r}")
    if path.parts and ":" in path.parts[0]:
        raise PrototypeError(f"drive-qualified path is forbidden: {value!r}")
    return path


def is_link_or_reparse(path: Path) -> bool:
    try:
        metadata = path.lstat()
    except OSError as exc:
        raise PrototypeError(f"cannot stat path {path}: {exc}") from exc
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    file_attributes = getattr(metadata, "st_file_attributes", 0)
    return stat.S_ISLNK(metadata.st_mode) or bool(file_attributes & reparse_flag)


def ensure_regular_file_under(path: Path, root: Path) -> None:
    root_resolved = root.resolve(strict=True)
    try:
        path_resolved = path.resolve(strict=True)
        path_resolved.relative_to(root_resolved)
    except (OSError, ValueError) as exc:
        raise PrototypeError(f"path escapes declared root: {path}") from exc

    current = path
    while True:
        if is_link_or_reparse(current):
            raise PrototypeError(f"links and reparse points are forbidden: {current}")
        if current.resolve(strict=True) == root_resolved:
            break
        current = current.parent
    if not path.is_file():
        raise PrototypeError(f"expected regular file: {path}")


def collect_file_records(root: Path) -> list[dict[str, Any]]:
    if not root.is_dir():
        raise PrototypeError(f"bundle root does not exist: {root}")
    records: list[dict[str, Any]] = []
    for current_root, dirs, files in os.walk(root, followlinks=False):
        base = Path(current_root)
        kept_dirs: list[str] = []
        for name in sorted(dirs):
            candidate = base / name
            if is_link_or_reparse(candidate):
                raise PrototypeError(f"bundle contains linked directory: {candidate}")
            kept_dirs.append(name)
        dirs[:] = kept_dirs
        for name in sorted(files):
            candidate = base / name
            ensure_regular_file_under(candidate, root)
            rel = candidate.relative_to(root).as_posix()
            safe_relative_path(rel)
            records.append(
                {
                    "path": rel,
                    "size_bytes": candidate.stat().st_size,
                    "sha256": sha256_file(candidate),
                }
            )
    return sorted(records, key=lambda item: item["path"])


def digest_file_records(records: list[dict[str, Any]]) -> str:
    normalized = [
        {"path": item["path"], "size_bytes": item["size_bytes"], "sha256": item["sha256"]}
        for item in sorted(records, key=lambda entry: entry["path"])
    ]
    return sha256_bytes(canonical_bytes(normalized))
