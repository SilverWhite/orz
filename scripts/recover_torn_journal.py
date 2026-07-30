#!/usr/bin/env python3
"""Inspect or explicitly recover one mechanically valid JSONL journal prefix."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance.errors import AssuranceError as PrototypeError
from assurance.journal_recovery import (
    inspect_journal_recovery,
    recover_journal,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--expected-run-id", required=True)
    parser.add_argument("--expected-manifest-sha256", required=True)
    parser.add_argument("--apply", action="store_true")
    parser.add_argument("--receipt")
    parser.add_argument("--quarantine")
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    try:
        if args.apply:
            if not args.receipt:
                raise PrototypeError("--apply requires --receipt")
            result = recover_journal(
                Path(args.journal),
                expected_run_id=args.expected_run_id,
                expected_manifest_sha256=args.expected_manifest_sha256,
                receipt_path=Path(args.receipt),
                quarantine_path=Path(args.quarantine) if args.quarantine else None,
                lock_timeout_seconds=args.lock_timeout_seconds,
            )
        else:
            if args.receipt or args.quarantine:
                raise PrototypeError("--receipt and --quarantine require --apply")
            result = inspect_journal_recovery(
                Path(args.journal),
                expected_run_id=args.expected_run_id,
                expected_manifest_sha256=args.expected_manifest_sha256,
                lock_timeout_seconds=args.lock_timeout_seconds,
            )
    except PrototypeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(result, ensure_ascii=False, sort_keys=True))
    return 0 if result.get("status") != "unrecoverable" else 2


if __name__ == "__main__":
    raise SystemExit(main())
