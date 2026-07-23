from __future__ import annotations

import json
import os
from pathlib import Path
import sys
import time
from typing import Any


ALLOWED_EVENTS = {"pre_compact", "post_compact"}


def main() -> int:
    target = (
        sys.argv[1]
        if len(sys.argv) == 2
        else os.environ.get("LIF_COMPACTION_HOOK_LOG")
    )
    if not target:
        print("LIF_COMPACTION_HOOK_LOG is required", file=sys.stderr)
        return 2
    try:
        payload: Any = json.load(sys.stdin)
    except (json.JSONDecodeError, UnicodeDecodeError) as error:
        print(f"invalid hook JSON: {error}", file=sys.stderr)
        return 2
    if not isinstance(payload, dict):
        print("hook input must be a JSON object", file=sys.stderr)
        return 2
    event = payload.get("hookEventName")
    if event not in ALLOWED_EVENTS:
        print(f"unexpected hook event: {event!r}", file=sys.stderr)
        return 2
    record = {
        "schema_version": "0.1.0",
        "observed_at_unix_ns": time.time_ns(),
        "hook_event_name": event,
        "session_id": payload.get("sessionId"),
        "source": payload.get("source"),
        "cwd": payload.get("cwd"),
        "workspace_root": payload.get("workspaceRoot"),
    }
    path = Path(target)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8", newline="\n") as handle:
        handle.write(
            json.dumps(record, ensure_ascii=False, sort_keys=True, allow_nan=False)
        )
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
