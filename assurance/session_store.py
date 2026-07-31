"""Session persistence store — Phase 3.

Writes session metadata and a global index (``index.jsonl``) under
``.gsa/sessions/`` in the project root.  The index is append-only;
each session also gets its own ``metadata.json`` in a per-session
subdirectory.

Usage::

    store = SessionStore()
    store.record(session_id="S-20260801-a1b2c3d4", metadata={...})
    sessions = store.list_sessions()
    meta = store.load_metadata("S-20260801-a1b2c3d4")
"""

from __future__ import annotations

import json
from pathlib import Path

from .contracts import ASSURANCE_ROOT
from .utils import atomic_write_json, utc_now

ROOT = ASSURANCE_ROOT.parent
SESSIONS_DIR = ROOT / ".gsa" / "sessions"
INDEX_PATH = SESSIONS_DIR / "index.jsonl"


class SessionStore:
    """Persistent index of past GSA sessions."""

    def __init__(self, sessions_dir: Path | None = None) -> None:
        self._dir = sessions_dir or SESSIONS_DIR

    # ── write ──────────────────────────────────────────────────────────────

    def record(self, session_id: str, metadata: dict) -> None:
        """Write *metadata* for *session_id* and append to the global index."""
        session_dir = self._dir / session_id
        session_dir.mkdir(parents=True, exist_ok=True)

        # Write full metadata.
        atomic_write_json(session_dir / "metadata.json", metadata)

        # Append index line.
        self._dir.mkdir(parents=True, exist_ok=True)
        index_entry = {
            "session_id": session_id,
            "created_at": metadata.get("created_at", utc_now()),
            "last_active_at": metadata.get("last_active_at", metadata.get("created_at", utc_now())),
            "first_prompt": metadata.get("first_prompt", ""),
            "prompt_preview": _preview(metadata.get("first_prompt", ""), 80),
            "turn_count": metadata.get("turn_count", 0),
            "run_root": metadata.get("run_root", ""),
            "status": metadata.get("status", "active"),
        }
        with INDEX_PATH.open("a", encoding="utf-8") as handle:
            handle.write(
                json.dumps(index_entry, ensure_ascii=False, sort_keys=True, allow_nan=False)
                + "\n"
            )

    # ── read ───────────────────────────────────────────────────────────────

    def list_sessions(self) -> list[dict]:
        """Return all indexed sessions, newest first."""
        if not INDEX_PATH.is_file():
            return []
        sessions: list[dict] = []
        with INDEX_PATH.open("r", encoding="utf-8") as handle:
            for line in handle:
                line = line.strip()
                if not line:
                    continue
                try:
                    sessions.append(json.loads(line))
                except json.JSONDecodeError:
                    continue
        sessions.reverse()  # newest first
        return sessions

    def load_metadata(self, session_id: str) -> dict | None:
        """Load the full metadata dict for *session_id*."""
        meta_path = self._dir / session_id / "metadata.json"
        if not meta_path.is_file():
            return None
        try:
            return json.loads(meta_path.read_text(encoding="utf-8"))
        except (json.JSONDecodeError, OSError):
            return None

    def update_status(self, session_id: str, status: str) -> None:
        """Update the status field for *session_id* (e.g. mark 'archived')."""
        meta = self.load_metadata(session_id)
        if meta is None:
            return
        meta["status"] = status
        meta["last_active_at"] = utc_now()
        atomic_write_json(self._dir / session_id / "metadata.json", meta)
        # Rebuild index to reflect status change (simpler than in-place edit).
        self._rebuild_index()

    def _rebuild_index(self) -> None:
        """Rebuild index.jsonl from per-session metadata files."""
        sessions: list[dict] = []
        if not self._dir.is_dir():
            return
        for child in sorted(self._dir.iterdir()):
            if child.is_dir():
                meta = self.load_metadata(child.name)
                if meta:
                    sessions.append({
                        "session_id": child.name,
                        "created_at": meta.get("created_at", ""),
                        "last_active_at": meta.get("last_active_at", ""),
                        "first_prompt": meta.get("first_prompt", ""),
                        "prompt_preview": _preview(meta.get("first_prompt", ""), 80),
                        "turn_count": meta.get("turn_count", 0),
                        "run_root": meta.get("run_root", ""),
                        "status": meta.get("status", "active"),
                    })
        sessions.sort(key=lambda s: s.get("created_at", ""), reverse=True)
        INDEX_PATH.parent.mkdir(parents=True, exist_ok=True)
        with INDEX_PATH.open("w", encoding="utf-8") as handle:
            for entry in sessions:
                handle.write(
                    json.dumps(entry, ensure_ascii=False, sort_keys=True, allow_nan=False)
                    + "\n"
                )


def _preview(text: str, max_len: int) -> str:
    """Truncate *text* to *max_len* chars, replacing newlines."""
    flat = text.replace("\n", " ").replace("\r", "")
    if len(flat) <= max_len:
        return flat
    return flat[: max_len - 3] + "..."
