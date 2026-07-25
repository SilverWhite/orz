#!/usr/bin/env python3
"""Capture a no-model Codex app-server initialize plus ephemeral thread/start."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from prototype.fep_agent_proto.codex_app_server_capture import (
    capture_ephemeral_thread,
)
from prototype.fep_agent_proto.errors import PrototypeError


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Launch one explicitly selected Codex app-server executable, "
            "capture initialize plus ephemeral read-only thread/start, and "
            "stop without sending a turn or model input."
        )
    )
    parser.add_argument("--executable", required=True)
    parser.add_argument(
        "--app-server-arg",
        action="append",
        default=[],
        help="Repeat for each executable argument, for example app-server.",
    )
    parser.add_argument("--cwd", required=True)
    parser.add_argument("--runtime-version", required=True)
    parser.add_argument("--capture", required=True)
    parser.add_argument("--stderr", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--isolated-state-dir")
    parser.add_argument("--timeout-seconds", type=float, default=10.0)
    parser.add_argument("--shutdown-grace-seconds", type=float, default=2.0)
    parser.add_argument("--max-line-bytes", type=int, default=1024 * 1024)
    parser.add_argument("--max-records", type=int, default=128)
    parser.add_argument("--max-stderr-bytes", type=int, default=1024 * 1024)
    parser.add_argument("--source-stream-id")
    args = parser.parse_args()

    try:
        receipt = capture_ephemeral_thread(
            command=[args.executable, *args.app_server_arg],
            cwd=Path(args.cwd),
            runtime_version=args.runtime_version,
            capture_path=Path(args.capture),
            stderr_path=Path(args.stderr),
            receipt_path=Path(args.receipt),
            isolated_state_dir=(
                Path(args.isolated_state_dir)
                if args.isolated_state_dir
                else None
            ),
            timeout_seconds=args.timeout_seconds,
            shutdown_grace_seconds=args.shutdown_grace_seconds,
            max_line_bytes=args.max_line_bytes,
            max_records=args.max_records,
            max_stderr_bytes=args.max_stderr_bytes,
            source_stream_id=args.source_stream_id,
        )
    except (OSError, PrototypeError, ValueError) as exc:
        print(f"capture failed before receipt: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, indent=2, allow_nan=False))
    return 0 if receipt["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
