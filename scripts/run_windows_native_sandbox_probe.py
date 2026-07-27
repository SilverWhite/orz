#!/usr/bin/env python3
"""Run a disposable Windows native AppContainer sandbox probe.

Writes observation JSON to the workspace (or --output). Does not claim
production security; see docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md.
"""
from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance.sandbox import build_sandbox_selection_receipt  # noqa: E402
from assurance.windows_sandbox import (  # noqa: E402
    run_windows_native_sandbox_probe,
    windows_native_candidate_from_observation,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--workspace",
        type=Path,
        default=None,
        help="Disposable workspace with .assurance-p2-disposable.json marker",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Where to write observation JSON (default: <workspace>/windows-native-observation.json)",
    )
    parser.add_argument(
        "--selection",
        action="store_true",
        help="Also print sandbox selection receipt for windows_native_strict",
    )
    args = parser.parse_args()

    cleanup: tempfile.TemporaryDirectory[str] | None = None
    if args.workspace is None:
        cleanup = tempfile.TemporaryDirectory(prefix="w32-native-probe-")
        workspace = Path(cleanup.name)
        marker = {
            "schema_version": "0.1.0-draft",
            "purpose": "windows-native-sandbox-probe",
            "allow_container_write_probe": True,
        }
        (workspace / ".assurance-p2-disposable.json").write_text(
            json.dumps(marker, indent=2),
            encoding="utf-8",
        )
    else:
        workspace = args.workspace

    try:
        observation = run_windows_native_sandbox_probe(workspace)
        output = args.output or (workspace / "windows-native-observation.json")
        output.write_text(
            json.dumps(observation, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        candidate = windows_native_candidate_from_observation(observation)
        summary = {
            "observation_path": str(output),
            "outcome": observation["outcome"],
            "checks": observation["checks"],
            "firewall": observation.get("firewall", {}),
            "candidate_compliance": candidate["compliance_status"],
            "rejection_reasons": candidate.get("rejection_reasons", []),
        }
        if args.selection:
            receipt = build_sandbox_selection_receipt(
                requested_backend="windows_native_strict",
                candidates=[candidate],
                conversation_id="CONV-WINDOWS-NATIVE-PROBE",
            )
            summary["selection_decision"] = receipt.get("decision")
            summary["selection_receipt"] = receipt
        print(json.dumps(summary, indent=2, sort_keys=True))
        return 0 if observation["outcome"] == "compliant" else 2
    finally:
        if cleanup is not None:
            cleanup.cleanup()


if __name__ == "__main__":
    raise SystemExit(main())
