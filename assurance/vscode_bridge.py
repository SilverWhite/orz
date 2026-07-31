"""VS Code assurance bridge — formats GSA state for the VS Code webview.

This module is called by the VS Code extension (via ``gsa doctor --json``
and other CLI commands) to produce structured JSON that the webview's
``renderStatus()`` consumes.  It does NOT import VS Code APIs — it only
produces JSON on stdout.

Schema contract::

    {
      "gates": [{"name": str, "decision": "allow|block|defer", "reason": str}],
      "sources": [{"id": str, "decision": "allow|block|defer"}],
      "tools": [{"name": str, "available": bool}],
      "session": {"id": str, "turns": int, "status": str}
    }
"""

from __future__ import annotations

from dataclasses import asdict, dataclass, field
import json
from pathlib import Path
from typing import Any


# ── data model ───────────────────────────────────────────────────────────────


@dataclass
class GateSnapshot:
    name: str = ""
    decision: str = "?"
    reason: str = ""


@dataclass
class SourceSnapshot:
    id: str = ""
    decision: str = "?"


@dataclass
class ToolSnapshot:
    name: str = ""
    available: bool = False


@dataclass
class SessionSnapshot:
    id: str = ""
    turns: int = 0
    status: str = "idle"


@dataclass
class AssuranceSnapshot:
    gates: list[GateSnapshot] = field(default_factory=list)
    sources: list[SourceSnapshot] = field(default_factory=list)
    tools: list[ToolSnapshot] = field(default_factory=list)
    session: SessionSnapshot = field(default_factory=SessionSnapshot)


# ── public API ───────────────────────────────────────────────────────────────


def build_assurance_snapshot(
    *,
    tool_availability_receipt: dict[str, Any] | None = None,
    source_visibility_ledger: list[dict[str, Any]] | None = None,
    gate_receipts: list[dict[str, Any]] | None = None,
    session_meta: dict[str, Any] | None = None,
) -> AssuranceSnapshot:
    """Build a structured assurance snapshot for the VS Code webview."""

    snap = AssuranceSnapshot()

    # Gates
    for gr in (gate_receipts or []):
        snap.gates.append(GateSnapshot(
            name=gr.get("gate_name", gr.get("gate", "gate")),
            decision=gr.get("decision", "?"),
            reason=gr.get("reason", ""),
        ))

    # Sources
    for src in (source_visibility_ledger or []):
        snap.sources.append(SourceSnapshot(
            id=src.get("ref_id", src.get("source_id", "?")),
            decision=src.get("decision", "?"),
        ))

    # Tools
    if tool_availability_receipt:
        for tool in tool_availability_receipt.get("tools", []):
            snap.tools.append(ToolSnapshot(
                name=tool.get("tool_name", tool.get("name", "?")),
                available=tool.get("available", tool.get("decision") == "allow"),
            ))

    # Session
    if session_meta:
        snap.session = SessionSnapshot(
            id=session_meta.get("session_id", ""),
            turns=session_meta.get("turn_count", 0),
            status=session_meta.get("status", "active"),
        )

    return snap


def build_doctor_status() -> dict[str, Any]:
    """Build a lightweight status payload for ``gsa doctor --json --quick``.

    Called by the VS Code extension's status poll.  Scans the most recent
    Grok run root for assurance artifacts without launching a full audit.
    """
    result: dict[str, Any] = {
        "gates": [],
        "sources": [],
        "tools": [],
        "session": {"status": "idle"},
    }

    # Scan .gsa/runs/ for recent session.
    runs_dir = Path(".gsa/runs")
    if not runs_dir.is_dir():
        return result

    dirs = sorted(runs_dir.iterdir(), reverse=True)
    for child in dirs:
        if not child.is_dir():
            continue
        events_path = child / "events.jsonl"
        if not events_path.is_file():
            continue

        # Found most recent session.
        result["session"] = {"id": child.name, "status": "completed"}

        # Count turns from events.
        try:
            turns = 0
            with events_path.open("r", encoding="utf-8") as handle:
                for line in handle:
                    if '"model_output"' in line or '"text_delta"' in line:
                        turns += 1
            result["session"]["turns"] = turns
        except OSError:
            pass

        # Load gate decisions.
        try:
            for evt_line in events_path.read_text(encoding="utf-8").splitlines():
                if not evt_line.strip():
                    continue
                evt = json.loads(evt_line)
                if evt.get("event_type") == "gate_decision":
                    payload = evt.get("payload", {})
                    result["gates"].append({
                        "name": payload.get("gate_name", "gate"),
                        "decision": payload.get("decision", "?"),
                        "reason": payload.get("reason", ""),
                    })
        except (OSError, json.JSONDecodeError):
            pass
        break  # Only show the most recent.

    return result
