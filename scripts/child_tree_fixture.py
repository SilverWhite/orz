from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Any


ROLES = ("root", "child", "grandchild")


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _script_sha256() -> str:
    return hashlib.sha256(Path(__file__).resolve().read_bytes()).hexdigest()


def _write_role_record(output_directory: Path, nonce: str, role: str) -> Path:
    record_path = output_directory / f"{role}.json"
    if record_path.exists():
        raise RuntimeError(f"refusing to overwrite role record: {record_path}")
    record = {
        "schema_version": "0.1.0",
        "fixture_kind": "windows-child-tree",
        "nonce": nonce,
        "role": role,
        "pid": os.getpid(),
        "parent_pid": os.getppid(),
        "started_at_unix_ns": time.time_ns(),
        "script_path": str(Path(__file__).resolve()),
        "script_sha256": _script_sha256(),
        "normal_completion": False,
    }
    _atomic_write_json(record_path, record)
    print(f"LIF_CHILD_TREE_STDOUT:{nonce}:{role}:ready", flush=True)
    print(
        f"LIF_CHILD_TREE_STDERR:{nonce}:{role}:ready",
        file=sys.stderr,
        flush=True,
    )
    return record_path


def _spawn_role(
    *,
    role: str,
    output_directory: Path,
    nonce: str,
    hold_seconds: float,
) -> subprocess.Popen[bytes]:
    return subprocess.Popen(
        [
            sys.executable,
            str(Path(__file__).resolve()),
            "--role",
            role,
            "--output-directory",
            str(output_directory),
            "--nonce",
            nonce,
            "--hold-seconds",
            str(hold_seconds),
        ],
        stdin=subprocess.DEVNULL,
        stdout=None,
        stderr=None,
        close_fds=False,
    )


def _hold(record_path: Path, hold_seconds: float) -> None:
    deadline = time.monotonic() + hold_seconds
    while time.monotonic() < deadline:
        time.sleep(min(0.25, max(0.01, deadline - time.monotonic())))
    record = json.loads(record_path.read_text(encoding="utf-8"))
    record["normal_completion"] = True
    record["completed_at_unix_ns"] = time.time_ns()
    _atomic_write_json(record_path, record)


def run(
    *,
    role: str,
    output_directory: Path,
    nonce: str,
    hold_seconds: float,
) -> None:
    if role not in ROLES:
        raise ValueError(f"unsupported role: {role}")
    if not nonce.startswith("LIFCHILD-") or not nonce[9:].isalnum():
        raise ValueError("nonce must match LIFCHILD- followed by alphanumeric text")
    if hold_seconds < 5 or hold_seconds > 3600:
        raise ValueError("--hold-seconds must be in [5, 3600]")
    output_directory.mkdir(parents=True, exist_ok=True)
    record_path = _write_role_record(output_directory, nonce, role)
    child: subprocess.Popen[bytes] | None = None
    if role == "root":
        child = _spawn_role(
            role="child",
            output_directory=output_directory,
            nonce=nonce,
            hold_seconds=hold_seconds,
        )
    elif role == "child":
        child = _spawn_role(
            role="grandchild",
            output_directory=output_directory,
            nonce=nonce,
            hold_seconds=hold_seconds,
        )
    try:
        _hold(record_path, hold_seconds)
    finally:
        if child is not None and child.poll() is None:
            child.terminate()
            try:
                child.wait(timeout=2)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=2)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Spawn a fixed root/child/grandchild tree for Windows containment probes"
    )
    parser.add_argument("--role", choices=ROLES, default="root")
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--nonce", required=True)
    parser.add_argument("--hold-seconds", type=float, default=300.0)
    args = parser.parse_args()
    try:
        run(
            role=args.role,
            output_directory=args.output_directory.resolve(),
            nonce=args.nonce,
            hold_seconds=args.hold_seconds,
        )
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
