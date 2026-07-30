#!/usr/bin/env python3
"""Reduce a replayable journal to its journal-authoritative Global Progress state."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from append_global_progress_transition_event import AppendError, _replay
from assurance.global_progress_state import (
    reduce_global_progress_events,
)
from assurance.io_utils import atomic_write_json
from assurance.journal_lock import (
    JournalLockError,
    exclusive_journal_lock,
)
from assurance.schema import validate_instance


SCHEMA = ROOT / "runtime" / "global-progress-state-reduction-v0.1.schema.json"


def reduce_journal(
    journal: Path,
    *,
    expected_run_id: str,
    expected_manifest_sha256: str,
    lock_timeout_seconds: float = 5.0,
) -> dict:
    with exclusive_journal_lock(journal, timeout_seconds=lock_timeout_seconds):
        events = _replay(journal)
        report = reduce_global_progress_events(
            events,
            expected_run_id=expected_run_id,
            expected_manifest_sha256=expected_manifest_sha256,
        )
    validate_instance(report, SCHEMA, label="Global Progress state reduction")
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--journal", required=True)
    parser.add_argument("--expected-run-id", required=True)
    parser.add_argument("--expected-manifest-sha256", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--lock-timeout-seconds", type=float, default=5.0)
    args = parser.parse_args()
    try:
        report = reduce_journal(
            Path(args.journal),
            expected_run_id=args.expected_run_id,
            expected_manifest_sha256=args.expected_manifest_sha256,
            lock_timeout_seconds=args.lock_timeout_seconds,
        )
        atomic_write_json(Path(args.output), report)
    except (AppendError, JournalLockError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps(report, ensure_ascii=False, sort_keys=True))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
