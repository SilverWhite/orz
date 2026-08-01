"""Unified cross-file action kernel verification — GAK-05.

After a run completes, :func:`verify_action_kernel` reads every output file
from the run root and checks cross-file consistency: same run_id across
manifest/journal/process/session, matching digests, terminal state mapping,
and absence of forbidden model/tool events in no-model runs.

See :file:`architecture/ACTION_KERNEL_CONTRACT_v0.1.md` §2 and §5.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import load_json, sha256_file, utc_now


RESULT_SCHEMA = "action-kernel-result-v0.1.schema.json"

# State mapping: windows process terminal_state → journal terminal event
_STATE_MAP: dict[str, str] = {
    "succeeded": "run_finished",
    "failed": "run_failed",
    "cancelled": "run_cancelled",
    "unknown": "run_invalidated",
}

FORBIDDEN_EVENTS: frozenset[str] = frozenset({
    "model_request", "model_output", "tool_proposal",
    "permission_decision", "tool_started", "tool_completed",
})


def verify_action_kernel(
    output_root: Path,
    *,
    require_process_result: bool = True,
    require_session_record: bool = True,
) -> dict[str, Any]:
    """Cross-verify all artifacts in an action kernel output root.

    Parameters
    ----------
    output_root:
        Directory containing run-manifest.json, events.jsonl, and
        optionally windows-process-result.json + session-record.json.
    require_process_result / require_session_record:
        When ``True`` (default), missing optional files are errors.

    Returns a schema-validated ``action-kernel-result`` dict.
    """
    errors: list[str] = []
    checks: dict[str, bool] = {}
    artifacts: dict[str, dict[str, Any]] = {}

    # ── helpers ──────────────────────────────────────────────────────────
    def _read_json(path: Path) -> dict[str, Any]:
        try:
            data = load_json(path)
            return data if isinstance(data, dict) else {}
        except (json.JSONDecodeError, OSError, AssuranceError):
            return {}

    def _read_journal(path: Path) -> list[dict[str, Any]]:
        if not path.is_file():
            return []
        events: list[dict[str, Any]] = []
        for line in path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            try:
                ev = json.loads(line)
                if isinstance(ev, dict):
                    events.append(ev)
            except json.JSONDecodeError:
                pass
        return events

    # ── 1. run-manifest.json ─────────────────────────────────────────────
    manifest_path = output_root / "run-manifest.json"
    manifest = _read_json(manifest_path) if manifest_path.is_file() else {}
    checks["manifest_present"] = manifest_path.is_file()
    if not checks["manifest_present"]:
        errors.append("run-manifest.json missing")

    run_id = manifest.get("run_id", "") if manifest else ""
    manifest_sha256 = sha256_file(manifest_path) if manifest_path.is_file() else ""
    artifacts["run-manifest"] = {
        "path": str(manifest_path), "sha256": manifest_sha256, "run_id": run_id,
    }

    # ── 2. events.jsonl ──────────────────────────────────────────────────
    journal_path = output_root / "events.jsonl"
    journal_events = _read_journal(journal_path)
    checks["journal_present"] = bool(journal_events) or journal_path.is_file()

    # Verify hash chain
    prev: str | None = None
    chain_ok = True
    terminal_events: list[str] = []
    last_event_sha256: str | None = None
    event_count = len(journal_events)

    for i, ev in enumerate(journal_events):
        if ev.get("sequence") != i:
            chain_ok = False
        if ev.get("run_id") != run_id and run_id:
            chain_ok = False
        if ev.get("previous_event_sha256") != prev and not (prev is None and i == 0):
            chain_ok = False
        et = ev.get("event_type", "")
        if et in ("run_finished", "run_failed", "run_cancelled", "run_invalidated"):
            terminal_events.append(et)
        # Compute event_sha256 from the event itself (excluding the field)
        body = dict(ev)
        body.pop("event_sha256", None)
        from .utils import canonical_bytes as _cb, sha256_bytes as _shab
        expected_digest = _shab(_cb(body))
        if ev.get("event_sha256") != expected_digest:
            chain_ok = False
        prev = ev.get("event_sha256")
        last_event_sha256 = prev

    checks["journal_hash_chain_valid"] = chain_ok
    checks["journal_replay_valid"] = chain_ok and event_count > 0
    checks["exactly_one_terminal"] = len(terminal_events) == 1
    if not checks["exactly_one_terminal"]:
        errors.append(
            f"expected exactly 1 terminal event, found {len(terminal_events)}"
        )
    if not chain_ok:
        errors.append("journal hash chain is broken")

    # Forbidden events
    found_forbidden = {e.get("event_type", "") for e in journal_events} & FORBIDDEN_EVENTS
    checks["no_model_or_tool_events"] = not found_forbidden
    if found_forbidden:
        errors.append(
            f"forbidden model/tool events in no-model journal: {sorted(found_forbidden)}"
        )

    artifacts["events.jsonl"] = {
        "path": str(journal_path),
        "sha256": sha256_file(journal_path) if journal_path.is_file() else "",
        "event_count": event_count,
        "terminal_event": terminal_events[-1] if terminal_events else None,
        "last_event_sha256": last_event_sha256,
    }

    # ── 3. windows-process-result.json ───────────────────────────────────
    proc_path = output_root / "windows-process-result.json"
    proc = _read_json(proc_path) if proc_path.is_file() else {}
    checks["process_result_present"] = proc_path.is_file()
    if not checks["process_result_present"] and require_process_result:
        errors.append("windows-process-result.json missing")

    artifacts["windows-process-result"] = {
        "path": str(proc_path),
        "sha256": sha256_file(proc_path) if proc_path.is_file() else "",
        "exit_code": proc.get("exit_code") if proc else None,
        "terminal_state": proc.get("terminal_state") if proc else None,
    }

    # ── 4. session-record.json ───────────────────────────────────────────
    session_path = output_root / "session-record.json"
    session = _read_json(session_path) if session_path.is_file() else {}
    checks["session_record_present"] = session_path.is_file()
    if not checks["session_record_present"] and require_session_record:
        errors.append("session-record.json missing")

    artifacts["session-record"] = {
        "path": str(session_path),
        "sha256": sha256_file(session_path) if session_path.is_file() else "",
        "session_id": session.get("session_id", "") if session else "",
    }

    # ── 5. Cross-file consistency ────────────────────────────────────────

    # 5a. run_id alignment
    proc_rid = proc.get("run_id", "") if proc else ""
    sess_rid = session.get("session_id", "").replace("SESS-", "") if session else ""
    checks["run_id_process_matches"] = (not proc_rid) or (proc_rid == run_id)
    checks["run_id_session_matches"] = (
        (not session) or (session.get("run_id", "") == run_id)
    )
    if proc_rid and proc_rid != run_id:
        errors.append(
            f"process result run_id {proc_rid!r} != manifest {run_id!r}"
        )
    if session and session.get("run_id", "") != run_id and session.get("run_id"):
        errors.append(
            f"session record run_id {session.get('run_id','')!r} != manifest {run_id!r}"
        )

    # 5b. Terminal state mapping
    proc_state = proc.get("terminal_state", "") if proc else ""
    journal_terminal = terminal_events[-1] if terminal_events else None
    if proc_state and journal_terminal:
        expected = _STATE_MAP.get(proc_state)
        if expected and journal_terminal != expected:
            errors.append(
                f"process {proc_state!r} → journal {journal_terminal!r} "
                f"(expected {expected!r})"
            )
            checks["terminal_state_mapping"] = False
        else:
            checks["terminal_state_mapping"] = True

    # 5c. Session journal_head → journal last event
    jhead = session.get("journal_head") if session else None
    if isinstance(jhead, str) and last_event_sha256 and jhead != last_event_sha256:
        errors.append("session journal_head != journal last event sha256")
        checks["session_journal_head_matches"] = False
    elif jhead and last_event_sha256:
        checks["session_journal_head_matches"] = True

    # ── 6. Result ────────────────────────────────────────────────────────
    valid = not errors
    result = {
        "schema_version": "0.1.0-draft",
        "result_kind": "action_kernel_cross_file_verification",
        "result_id": f"AKR-{manifest_sha256[:16].upper() if manifest_sha256 else 'NONE'}",
        "created_at": utc_now(),
        "output_root": str(output_root),
        "valid": valid,
        "verifier_version": "0.1.0",
        "artifacts": artifacts,
        "checks": checks,
        "errors": errors,
        "limitations": [
            "Cross-file verification proves internal consistency, not correctness.",
            "Files outside the 4 expected artifacts are not checked.",
            "Digest verification cannot detect coordinated tampering.",
        ],
    }
    validate_contract(result, RESULT_SCHEMA, label="action kernel result")
    return result
