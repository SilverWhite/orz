"""Post-run cross-verification of Grok ACP session artifacts — D1.12.

Verifies consistency between three independent data sources produced by
:func:`~.grok_runtime_adapter.run_grok_acp_once`:

1. **Runtime receipt** — the ``acp`` block with aggregate counts
   (tool_call_count, permission_requests_count, turn_count, etc.)
2. **events.jsonl** — normalized runtime events written by
   :func:`~.grok_event_normalizer.normalize_grok_runtime_receipt`
3. **acp_transcript.jsonl** — metadata-only transcript of the raw
   ACP JSON-RPC messages (direction + message_sha256)

Cross-verification is a mechanical assurance check — it does not inspect
raw prompt/output content (those are SHA-256 hashed in the transcript).
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .contracts import validate_contract
from .errors import AssuranceError
from .utils import atomic_write_json, canonical_bytes, sha256_bytes, sha256_file, utc_now


RECEIPT_SCHEMA = "grok-session-verification-receipt-v0.1.schema.json"
ROOT = Path(__file__).resolve().parents[1]


def verify_acp_session(
    receipt: dict[str, Any],
    *,
    events_path: Path | None = None,
    transcript_path: Path | None = None,
) -> dict[str, Any]:
    """Cross-verify ACP session artifacts.

    Parameters
    ----------
    receipt:
        The full runtime receipt returned by :func:`run_grok_acp_once`.
    events_path:
        Path to ``events.jsonl``.  Defaults to the path recorded in the receipt.
    transcript_path:
        Path to ``acp_transcript.jsonl``.  Defaults to the path recorded in the receipt.

    Returns
    -------
    :
        A verification receipt dict conforming to
        :file:`grok-session-verification-receipt-v0.1.schema.json`.
    """
    import secrets
    import hashlib

    verification_id = f"GSV-{secrets.token_hex(8).upper()}"
    created_at = utc_now()
    run_id = str(receipt.get("run_id", ""))

    errors: list[str] = []
    checks: dict[str, bool] = {}

    # ── Resolve paths ──────────────────────────────────────────────────────
    if events_path is None:
        events_path = Path(str(receipt.get("artifacts", {}).get("events_path", "")))
    if transcript_path is None:
        transcript_path = Path(str(receipt.get("execution", {}).get("stdout_path", "")))

    # ── Load events.jsonl ──────────────────────────────────────────────────
    events: list[dict[str, Any]] = []
    events_raw = b""
    if events_path.is_file():
        events_raw = events_path.read_bytes()
        for line in events_raw.decode("utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                evt = json.loads(line)
            except json.JSONDecodeError:
                errors.append(f"events.jsonl: non-JSON line in {events_path}")
                continue
            if isinstance(evt, dict):
                events.append(evt)

    events_sha256_val = sha256_bytes(events_raw) if events_raw else ("0" * 64)

    # ── Load transcript ────────────────────────────────────────────────────
    transcript_entries: list[dict[str, Any]] = []
    transcript_raw = b""
    if transcript_path.is_file():
        transcript_raw = transcript_path.read_bytes()
        for line in transcript_raw.decode("utf-8", errors="replace").splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                errors.append(f"acp_transcript.jsonl: non-JSON line in {transcript_path}")
                continue
            if isinstance(entry, dict):
                transcript_entries.append(entry)

    transcript_sha256_val = sha256_bytes(transcript_raw) if transcript_raw else ("0" * 64)
    receipt_sha256_val = sha256_bytes(canonical_bytes(receipt))

    # ── Event type counts ──────────────────────────────────────────────────
    event_type_counts: dict[str, int] = {}
    for evt in events:
        et = evt.get("event_type", "")
        event_type_counts[et] = event_type_counts.get(et, 0) + 1

    # ── Check 1: ACP lifecycle complete ────────────────────────────────────
    has_init = event_type_counts.get("acp_initialize", 0) >= 1
    has_session = event_type_counts.get("acp_session_created", 0) >= 1
    checks["acp_lifecycle_complete"] = has_init and has_session
    if not checks["acp_lifecycle_complete"]:
        missing = []
        if not has_init:
            missing.append("acp_initialize")
        if not has_session:
            missing.append("acp_session_created")
        errors.append(f"ACP lifecycle incomplete: missing events {missing}")

    # ── Check 2: Session ID consistency ────────────────────────────────────
    receipt_session_hash = str(receipt.get("acp", {}).get("session_id_hash", ""))
    event_session_hash = ""
    for evt in events:
        if evt.get("event_type") == "acp_session_created":
            event_session_hash = str(evt.get("payload", {}).get("session_id_hash", ""))
            break
    checks["session_id_consistent"] = (
        bool(receipt_session_hash)
        and receipt_session_hash == event_session_hash
    )
    if not checks["session_id_consistent"]:
        errors.append(
            "session_id_hash mismatch: "
            f"receipt={receipt_session_hash[:16]}..., "
            f"events={event_session_hash[:16]}..."
        )

    # ── Check 3: Tool call count match ─────────────────────────────────────
    receipt_tool_count = int(receipt.get("acp", {}).get("tool_call_count", 0))
    event_tool_count = event_type_counts.get("tool_proposal", 0)
    checks["tool_call_count_match"] = receipt_tool_count == event_tool_count
    if not checks["tool_call_count_match"]:
        errors.append(
            f"tool_call count mismatch: receipt={receipt_tool_count}, events={event_tool_count}"
        )

    # ── Check 4: Permission count match ────────────────────────────────────
    receipt_perm_count = int(receipt.get("acp", {}).get("permission_requests_count", 0))
    event_perm_count = event_type_counts.get("permission_requested", 0)
    checks["permission_count_match"] = receipt_perm_count == event_perm_count
    if not checks["permission_count_match"]:
        errors.append(
            f"permission_requests count mismatch: "
            f"receipt={receipt_perm_count}, events={event_perm_count}"
        )

    # ── Check 5: Permission outcomes match ─────────────────────────────────
    receipt_outcomes: list[str] = list(receipt.get("acp", {}).get("permission_outcomes", []))
    event_outcomes: list[str] = []
    for evt in events:
        if evt.get("event_type") == "permission_decision":
            decision = str(evt.get("payload", {}).get("decision", ""))
            if decision:
                event_outcomes.append(decision)
    checks["permission_outcomes_match"] = receipt_outcomes == event_outcomes
    if not checks["permission_outcomes_match"]:
        errors.append(
            f"permission outcomes mismatch: "
            f"receipt={receipt_outcomes}, events={event_outcomes}"
        )

    # ── Check 6: Transcript has expected ACP lifecycle ─────────────────────
    # The transcript records message direction + SHA-256.  Verify that it
    # contains at minimum the expected lifecycle phases.
    directions = [e.get("direction", "") for e in transcript_entries]
    c2a_count = directions.count("client_to_agent")
    a2c_count = directions.count("agent_to_client")
    has_min_transcript = (
        c2a_count >= 3  # initialize, session/new, session/prompt
        and a2c_count >= 3  # initialize response, session/new response, prompt response
    )
    # Also verify every message_sha256 is a valid hex string
    all_hashes_valid = True
    for entry in transcript_entries:
        digest = entry.get("message_sha256", "")
        if not isinstance(digest, str) or len(digest) != 64:
            all_hashes_valid = False
            break
        try:
            int(digest, 16)
        except ValueError:
            all_hashes_valid = False
            break
    checks["transcript_has_expected_lifecycle"] = has_min_transcript and all_hashes_valid
    if not checks["transcript_has_expected_lifecycle"]:
        if not has_min_transcript:
            errors.append(
                f"transcript lifecycle incomplete: "
                f"client_to_agent={c2a_count}, agent_to_client={a2c_count} (need >=3 each)"
            )
        if not all_hashes_valid:
            errors.append("transcript contains invalid message_sha256 entries")

    # ── Check 7: Event hash chain valid ────────────────────────────────────
    chain_valid = True
    prev_sha = None
    for i, evt in enumerate(events):
        event_sha = evt.get("event_sha256", "")
        prev_in_event = evt.get("previous_event_sha256")
        if not isinstance(event_sha, str) or len(event_sha) != 64:
            chain_valid = False
            errors.append(f"event[{i}]: invalid or missing event_sha256")
            break
        if i > 0 and prev_sha is not None and prev_in_event != prev_sha:
            chain_valid = False
            errors.append(
                f"event[{i}]: hash chain broken — "
                f"expected prev={prev_sha[:16]}..., "
                f"got prev={str(prev_in_event)[:16]}..."
            )
            break
        prev_sha = event_sha
    checks["event_hash_chain_valid"] = chain_valid

    # ── Receipt ACP block SHA-256 ──────────────────────────────────────────
    acp_block = receipt.get("acp", {})
    receipt_acp_block_sha256 = sha256_bytes(canonical_bytes(acp_block))

    # ── Assemble verification receipt ──────────────────────────────────────
    all_checks_pass = all(checks.values())
    verification_receipt: dict[str, Any] = {
        "schema_version": "0.1.0-draft",
        "receipt_kind": "grok_session_verification_receipt",
        "verification_id": verification_id,
        "created_at": created_at,
        "run_id": run_id,
        "valid": all_checks_pass and len(errors) == 0,
        "checks": checks,
        "errors": errors,
        "transcript_message_count": len(transcript_entries),
        "event_count": len(events),
        "event_type_counts": event_type_counts,
        "receipt_acp_block_sha256": receipt_acp_block_sha256,
        "events_sha256": events_sha256_val,
        "transcript_sha256": transcript_sha256_val,
        "receipt_sha256": receipt_sha256_val,
    }
    validate_contract(verification_receipt, RECEIPT_SCHEMA, label="Grok session verification receipt")
    return verification_receipt


def write_verification_receipt(receipt: dict[str, Any], output_path: Path) -> None:
    """Write a cross-verification receipt to disk (atomic)."""
    atomic_write_json(output_path, receipt, overwrite=True)
