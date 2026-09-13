#!/usr/bin/env python3
"""Snapshot canonical per-task digests for the pinned TB 2.1 dataset version.

The authoritative corpus identity of the TB 2.1 official round is the Harbor
registry's per-task digest set for ``terminal-bench/terminal-bench-2-1`` at the
pinned ref used by the leaderboard CI (``DATASET_REF`` in the dataset repo's
``leaderboard/src/leaderboard/core/hub.py``).  This tool performs the
metadata-only registry query and writes the digest map as a local evidence
file so the corpus freeze manifest can be rebuilt and audited offline.

Runner environment: the evaluation harness venv, which has both ``harbor`` and
the leaderboard package, e.g.

    D:\\tb-eval\\venv\\Scripts\\python.exe scripts\\fetch_tb21_registry_digests.py

The query downloads task metadata only (no task content).
"""
from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--corpus-root", type=Path, default=Path("D:/tb-eval/terminal-bench-2-1"))
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args(argv)

    leaderboard_src = args.corpus_root / "leaderboard" / "src"
    if not leaderboard_src.is_dir():
        print(f"leaderboard package not found under {leaderboard_src}", file=sys.stderr)
        return 1
    sys.path.insert(0, str(leaderboard_src))

    try:
        from leaderboard.core.hub import DATASET, DATASET_REF, dataset_task_digests
    except Exception as exc:  # noqa: BLE001
        print(f"cannot import leaderboard.core.hub: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1

    try:
        digests = dataset_task_digests()
    except Exception as exc:  # noqa: BLE001
        print(f"registry query failed: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 1

    snapshot = {
        "schema": "tb21-registry-task-digests-v0.1",
        "dataset": DATASET,
        "ref": DATASET_REF,
        "fetched_at": datetime.now(timezone.utc).isoformat(),
        "task_count": len(digests),
        "source": "harbor.registry.client.package.PackageDatasetClient.get_dataset_metadata",
        "note": (
            "metadata-only registry query; task content is not downloaded by "
            "this tool. These digests are the identity of the task content the "
            "harness resolves for this pinned dataset ref."
        ),
        "tasks": dict(sorted(digests.items())),
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(snapshot, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"dataset            : {DATASET}@{DATASET_REF}")
    print(f"tasks              : {len(digests)}")
    print(f"snapshot           : {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
