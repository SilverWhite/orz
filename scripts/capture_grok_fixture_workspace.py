#!/usr/bin/env python3
"""Capture hash-only checkpoints and deltas for disposable Grok fixtures.

This spike is intentionally non-restoring. It never records file content and
refuses to operate without an explicit disposable-workspace marker.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import sys
import uuid
from datetime import datetime, timezone
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
MARKER_NAME = ".lif-disposable-workspace.json"
CHECKPOINT_SCHEMA = ROOT / "integration" / "grok" / "grok-fixture-workspace-checkpoint-v0.1.schema.json"
DELTA_SCHEMA = ROOT / "integration" / "grok" / "grok-fixture-workspace-delta-v0.1.schema.json"
DEFAULT_MAX_FILES = 10_000
DEFAULT_MAX_BYTES = 100 * 1024 * 1024


class CaptureError(RuntimeError):
    pass


def _utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _canonical_json(value: object) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _sha256_file(path: Path) -> tuple[int, str]:
    digest = hashlib.sha256()
    size = 0
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            size += len(chunk)
            digest.update(chunk)
    return size, digest.hexdigest()


def _is_reparse_point(path: Path) -> bool:
    metadata = os.lstat(path)
    attributes = getattr(metadata, "st_file_attributes", 0)
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0x400)
    return stat.S_ISLNK(metadata.st_mode) or bool(attributes & reparse_flag)


def _inside(root: Path, candidate: Path) -> bool:
    try:
        candidate.relative_to(root)
        return True
    except ValueError:
        return False


def _load_marker(workspace: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    marker_path = workspace / MARKER_NAME
    if not marker_path.is_file() or _is_reparse_point(marker_path):
        raise CaptureError(f"missing regular disposable marker: {marker_path}")
    raw = marker_path.read_bytes()
    try:
        marker = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CaptureError(f"invalid disposable marker JSON: {exc}") from exc
    if not isinstance(marker, dict):
        raise CaptureError("disposable marker must be a JSON object")
    if marker.get("schema_version") != "0.1.0":
        raise CaptureError("unsupported disposable marker schema_version")
    if marker.get("marker_kind") != "lif-disposable-workspace":
        raise CaptureError("unexpected disposable marker_kind")
    if marker.get("disposable") is not True:
        raise CaptureError("workspace marker does not explicitly set disposable=true")
    workspace_id = marker.get("workspace_id")
    if not isinstance(workspace_id, str) or not workspace_id.startswith("FIXTURE-") or len(workspace_id) != 40:
        raise CaptureError("workspace marker has invalid workspace_id")
    try:
        int(workspace_id.removeprefix("FIXTURE-"), 16)
    except ValueError as exc:
        raise CaptureError("workspace marker workspace_id must end in 32 lowercase hex characters") from exc
    if workspace_id != workspace_id.lower().replace("fixture-", "FIXTURE-", 1):
        raise CaptureError("workspace marker workspace_id must use lowercase hex")
    artifact = {
        "relative_path": MARKER_NAME,
        "bytes": len(raw),
        "sha256": _sha256_bytes(raw),
    }
    return marker, artifact


def _scan_workspace(workspace: Path, max_files: int, max_bytes: int) -> tuple[list[dict[str, Any]], int]:
    entries: list[dict[str, Any]] = []
    total_bytes = 0
    casefold_paths: dict[str, str] = {}
    for current, directories, filenames in os.walk(workspace, topdown=True, followlinks=False):
        current_path = Path(current)
        for name in list(directories):
            candidate = current_path / name
            if _is_reparse_point(candidate):
                raise CaptureError(f"reparse directory is outside the spike safety contract: {candidate}")
        directories.sort(key=str.casefold)
        filenames.sort(key=str.casefold)
        for name in filenames:
            path = current_path / name
            if _is_reparse_point(path):
                raise CaptureError(f"reparse file is outside the spike safety contract: {path}")
            metadata_before = os.lstat(path)
            if not stat.S_ISREG(metadata_before.st_mode):
                raise CaptureError(f"non-regular workspace entry is unsupported: {path}")
            relative = path.relative_to(workspace).as_posix()
            folded = relative.casefold()
            previous = casefold_paths.get(folded)
            if previous is not None and previous != relative:
                raise CaptureError(f"case-insensitive path collision: {previous!r} and {relative!r}")
            casefold_paths[folded] = relative
            size, digest = _sha256_file(path)
            metadata_after = os.lstat(path)
            if (
                size != metadata_before.st_size
                or metadata_after.st_size != metadata_before.st_size
                or metadata_after.st_mtime_ns != metadata_before.st_mtime_ns
                or getattr(metadata_after, "st_ino", None) != getattr(metadata_before, "st_ino", None)
            ):
                raise CaptureError(f"file changed while being hashed: {relative}")
            entries.append({"relative_path": relative, "bytes": size, "sha256": digest})
            total_bytes += size
            if len(entries) > max_files:
                raise CaptureError(f"workspace file limit exceeded: {max_files}")
            if total_bytes > max_bytes:
                raise CaptureError(f"workspace byte limit exceeded: {max_bytes}")
    entries.sort(key=lambda entry: entry["relative_path"].casefold())
    return entries, total_bytes


def _stable_scan(workspace: Path, max_files: int, max_bytes: int) -> tuple[list[dict[str, Any]], int]:
    first_entries, first_bytes = _scan_workspace(workspace, max_files, max_bytes)
    second_entries, second_bytes = _scan_workspace(workspace, max_files, max_bytes)
    if first_entries != second_entries or first_bytes != second_bytes:
        raise CaptureError("workspace changed between consecutive scan passes")
    return second_entries, second_bytes


def _aggregate(entries: list[dict[str, Any]]) -> str:
    return _sha256_bytes(_canonical_json(entries))


def _load_schema(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def _validate(schema_path: Path, value: dict[str, Any]) -> None:
    Draft202012Validator(
        _load_schema(schema_path), format_checker=FormatChecker()
    ).validate(value)


def _write_new_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise CaptureError(f"refusing to overwrite output: {path}") from exc
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def _resolve_paths(workspace_arg: str, output_arg: str) -> tuple[Path, Path]:
    requested_workspace = Path(workspace_arg)
    if not requested_workspace.is_dir() or _is_reparse_point(requested_workspace):
        raise CaptureError("workspace must be a non-reparse directory")
    workspace = requested_workspace.resolve(strict=True)
    output = Path(output_arg).resolve(strict=False)
    if _inside(workspace, output):
        raise CaptureError("output must be outside the disposable workspace")
    return workspace, output


def _checkpoint(args: argparse.Namespace) -> dict[str, Any]:
    workspace, output = _resolve_paths(args.workspace, args.output)
    marker, marker_artifact = _load_marker(workspace)
    entries, total_bytes = _stable_scan(workspace, args.max_files, args.max_bytes)
    scanned_marker = next((entry for entry in entries if entry["relative_path"] == MARKER_NAME), None)
    if scanned_marker != marker_artifact:
        raise CaptureError("disposable marker changed during checkpoint scan")
    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "grok-fixture-workspace-checkpoint",
        "checkpoint_id": f"CHECKPOINT-{uuid.uuid4().hex}",
        "created_at": _utc_now(),
        "workspace": {
            "canonical_path": str(workspace),
            "workspace_id": marker["workspace_id"],
            "disposable_marker": marker_artifact,
        },
        "policy": {
            "mode": "hash_only_non_restoring",
            "max_files": args.max_files,
            "max_bytes": args.max_bytes,
            "stability_passes": 2,
            "reparse_points": "deny",
            "casefold_collisions": "deny",
        },
        "state": {
            "file_count": len(entries),
            "total_bytes": total_bytes,
            "aggregate_sha256": _aggregate(entries),
            "files": entries,
        },
        "checks": {
            "disposable_marker_valid": True,
            "output_outside_workspace": True,
            "scan_complete_within_limits": True,
            "regular_files_only": True,
            "casefold_paths_unique": True,
        },
        "valid": True,
        "limitations": [
            "This checkpoint records hashes and metadata, not file contents.",
            "This spike cannot restore deleted or modified files.",
            "Mechanical equality does not establish semantic or scientific correctness.",
        ],
    }
    _validate(CHECKPOINT_SCHEMA, receipt)
    _write_new_json(output, receipt)
    return receipt


def _load_checkpoint(path_arg: str) -> tuple[Path, dict[str, Any], dict[str, Any]]:
    path = Path(path_arg).resolve(strict=True)
    raw = path.read_bytes()
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise CaptureError(f"invalid checkpoint JSON: {exc}") from exc
    if not isinstance(value, dict):
        raise CaptureError("checkpoint must be a JSON object")
    try:
        _validate(CHECKPOINT_SCHEMA, value)
    except Exception as exc:
        raise CaptureError(f"checkpoint schema validation failed: {exc}") from exc
    if value.get("valid") is not True:
        raise CaptureError("checkpoint is not valid")
    files = value["state"]["files"]
    relative_paths = [entry["relative_path"] for entry in files]
    if len(relative_paths) != len(set(relative_paths)):
        raise CaptureError("checkpoint contains duplicate relative paths")
    if len({path.casefold() for path in relative_paths}) != len(relative_paths):
        raise CaptureError("checkpoint contains case-insensitive path collisions")
    if files != sorted(files, key=lambda entry: entry["relative_path"].casefold()):
        raise CaptureError("checkpoint file entries are not in canonical order")
    if _aggregate(files) != value["state"]["aggregate_sha256"]:
        raise CaptureError("checkpoint aggregate does not match file entries")
    if len(files) != value["state"]["file_count"]:
        raise CaptureError("checkpoint file_count does not match file entries")
    if sum(entry["bytes"] for entry in files) != value["state"]["total_bytes"]:
        raise CaptureError("checkpoint total_bytes does not match file entries")
    marker_entries = [entry for entry in files if entry["relative_path"] == MARKER_NAME]
    if marker_entries != [value["workspace"]["disposable_marker"]]:
        raise CaptureError("checkpoint marker artifact does not match file entries")
    artifact = {"path": str(path), "bytes": len(raw), "sha256": _sha256_bytes(raw)}
    return path, value, artifact


def _delta(args: argparse.Namespace) -> dict[str, Any]:
    workspace, output = _resolve_paths(args.workspace, args.output)
    checkpoint_path, checkpoint, checkpoint_artifact = _load_checkpoint(args.checkpoint)
    if _inside(workspace, checkpoint_path):
        raise CaptureError("checkpoint input must be outside the disposable workspace")
    marker, marker_artifact = _load_marker(workspace)
    expected_workspace = checkpoint["workspace"]
    if marker["workspace_id"] != expected_workspace["workspace_id"]:
        raise CaptureError("workspace_id does not match checkpoint")
    if os.path.normcase(str(workspace)) != os.path.normcase(expected_workspace["canonical_path"]):
        raise CaptureError("workspace canonical path does not match checkpoint")
    if marker_artifact["sha256"] != expected_workspace["disposable_marker"]["sha256"]:
        raise CaptureError("disposable marker changed after checkpoint")
    entries, total_bytes = _stable_scan(workspace, args.max_files, args.max_bytes)
    scanned_marker = next((entry for entry in entries if entry["relative_path"] == MARKER_NAME), None)
    if scanned_marker != marker_artifact:
        raise CaptureError("disposable marker changed during delta scan")
    before = {entry["relative_path"]: entry for entry in checkpoint["state"]["files"]}
    after = {entry["relative_path"]: entry for entry in entries}
    created = [after[path] for path in sorted(after.keys() - before.keys(), key=str.casefold)]
    deleted = [before[path] for path in sorted(before.keys() - after.keys(), key=str.casefold)]
    modified = [
        {"relative_path": path, "before": before[path], "after": after[path]}
        for path in sorted(before.keys() & after.keys(), key=str.casefold)
        if before[path]["sha256"] != after[path]["sha256"] or before[path]["bytes"] != after[path]["bytes"]
    ]
    unchanged = sum(
        1
        for path in before.keys() & after.keys()
        if before[path]["sha256"] == after[path]["sha256"] and before[path]["bytes"] == after[path]["bytes"]
    )
    changes = {"created": created, "modified": modified, "deleted": deleted}
    receipt = {
        "schema_version": "0.1.0",
        "receipt_kind": "grok-fixture-workspace-delta",
        "delta_id": f"DELTA-{uuid.uuid4().hex}",
        "created_at": _utc_now(),
        "checkpoint": {
            **checkpoint_artifact,
            "checkpoint_id": checkpoint["checkpoint_id"],
            "aggregate_sha256": checkpoint["state"]["aggregate_sha256"],
        },
        "workspace": {
            "canonical_path": str(workspace),
            "workspace_id": marker["workspace_id"],
            "disposable_marker": marker_artifact,
        },
        "current_state": {
            "file_count": len(entries),
            "total_bytes": total_bytes,
            "aggregate_sha256": _aggregate(entries),
        },
        "changes": {
            **changes,
            "counts": {
                "created": len(created),
                "modified": len(modified),
                "deleted": len(deleted),
                "unchanged": unchanged,
            },
            "aggregate_sha256": _sha256_bytes(_canonical_json(changes)),
        },
        "checks": {
            "checkpoint_valid": True,
            "workspace_identity_matches": True,
            "disposable_marker_unchanged": True,
            "output_outside_workspace": True,
            "scan_complete_within_limits": True,
            "regular_files_only": True,
            "casefold_paths_unique": True,
        },
        "valid": True,
        "limitations": [
            "The delta records hashes and metadata, not file contents.",
            "Created, modified, and deleted classify filesystem state, not causal ownership by a tool.",
            "This spike does not restore files or resolve concurrent-edit conflicts.",
        ],
    }
    _validate(DELTA_SCHEMA, receipt)
    _write_new_json(output, receipt)
    return receipt


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("checkpoint", "delta"):
        subparser = subparsers.add_parser(name)
        subparser.add_argument("--workspace", required=True)
        subparser.add_argument("--output", required=True)
        subparser.add_argument("--max-files", type=int, default=DEFAULT_MAX_FILES)
        subparser.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES)
        if name == "delta":
            subparser.add_argument("--checkpoint", required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    if args.max_files < 1 or args.max_bytes < 1:
        print("error: scan limits must be positive", file=sys.stderr)
        return 2
    try:
        receipt = _checkpoint(args) if args.command == "checkpoint" else _delta(args)
    except (CaptureError, OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
