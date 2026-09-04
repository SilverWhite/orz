"""Export the run-event payload-schema registry to a machine-readable JSON.

Task D (双实现终局治理, 2026-09-04) batch-1: the Rust-side offline
conformance judge must resolve the per-event payload schema without importing
the Python validator.  The single source of the mapping remains
`assurance/run_event_journal_validation.py`
(`PAYLOAD_SCHEMA_BY_EVENT_TYPE` / `PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02`);
this script materializes that mapping as
`runtime/run-event-payload-registry-v0.1.json` with repo-root-relative schema
paths so both judges consume the same table.

`scripts/check_repository.py` recomputes the payload in memory and fails the
gate when the committed file drifts from the Python registry (a registry
change without regeneration is a silent gap).

Run: `python scripts/export_run_event_payload_registry.py`
"""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "runtime" / "run-event-payload-registry-v0.1.json"


def _relative(schema_path: Path) -> str:
    return schema_path.resolve().relative_to(ROOT.resolve()).as_posix()


def _track_payload(track_map: dict) -> dict[str, dict[str, str]]:
    """event_type -> {slug, schema} with repo-root-relative schema paths."""
    return {
        event_type: {
            "slug": slug,
            "schema": _relative(schema_path),
        }
        for event_type, (slug, schema_path) in track_map.items()
    }


def build_registry_payload() -> dict:
    """Recompute the registry payload from the Python registry dicts."""
    # Late import: the assurance module is importable only from the repo root.
    from assurance.run_event_journal_validation import (
        PAYLOAD_SCHEMA_BY_EVENT_TYPE,
        PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02,
    )

    return {
        "kind": "run-event-payload-schema-registry",
        "version": "0.1",
        "source": (
            "assurance/run_event_journal_validation.py "
            "PAYLOAD_SCHEMA_BY_EVENT_TYPE / PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02"
        ),
        "paths": "repo-root-relative",
        "tracks": {
            "v01": _track_payload(PAYLOAD_SCHEMA_BY_EVENT_TYPE),
            "v02": _track_payload(PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02),
        },
    }


def main() -> None:
    payload = build_registry_payload()
    OUT.write_text(
        json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(
        f"wrote {OUT.relative_to(ROOT)}: "
        f"v01={len(payload['tracks']['v01'])} "
        f"v02={len(payload['tracks']['v02'])}"
    )


if __name__ == "__main__":
    main()
