#!/usr/bin/env python3
"""Inspect or recover one archive operation journal.

Usage:
  python scripts/recover_archive_journal.py --journal <path>
      Read-only inspection; prints JSON recovery status to stdout.

  python scripts/recover_archive_journal.py --journal <path> --apply --receipt <path>
      Repair a torn journal by truncating to the last valid newline,
      writing an archive_recovered event, and saving a recovery receipt.

  python scripts/recover_archive_journal.py --journal <path> --apply --receipt <path> --quarantine <path>
      Same as above but also save discarded tail bytes to quarantine path.

Does NOT call any model, network, or credential APIs.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance.archive_journal import (  # noqa: E402
    inspect_archive_journal,
    recover_archive_journal,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--journal",
        type=Path,
        required=True,
        help="Path to the archive journal (.archive-journal.jsonl)",
    )
    parser.add_argument(
        "--apply",
        action="store_true",
        help="Apply recovery (requires --receipt; quarantines torn tail if --quarantine given)",
    )
    parser.add_argument(
        "--receipt",
        type=Path,
        default=None,
        help="Path for recovery receipt JSON (required with --apply)",
    )
    parser.add_argument(
        "--quarantine",
        type=Path,
        default=None,
        help="Path for quarantined torn-tail bytes",
    )
    parser.add_argument(
        "--lock-timeout-seconds",
        type=float,
        default=5.0,
        help="Lock acquisition timeout (default: 5.0)",
    )
    args = parser.parse_args()

    journal_path = args.journal.resolve()
    if not journal_path.is_file():
        print(json.dumps({"valid": False, "errors": [f"journal not found: {journal_path}"]}))
        return 1

    if args.apply:
        if args.receipt is None:
            print(json.dumps({"valid": False, "errors": ["--receipt is required with --apply"]}))
            return 1
        result = recover_archive_journal(
            journal_path,
            receipt_path=args.receipt.resolve(),
            quarantine_path=args.quarantine.resolve() if args.quarantine else None,
            lock_timeout_seconds=args.lock_timeout_seconds,
        )
    else:
        inspection = inspect_archive_journal(
            journal_path, lock_timeout_seconds=args.lock_timeout_seconds
        )
        result = {
            "valid": inspection["valid"],
            "classification": inspection["classification"],
            "prefix_length": inspection["prefix_length"],
            "tail_bytes": inspection["tail_bytes"],
            "replay_valid": inspection["replay"].get("valid", False),
            "replay_event_count": inspection["replay"].get("event_count", 0),
        }

    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if result.get("valid", False) else 2


if __name__ == "__main__":
    raise SystemExit(main())
