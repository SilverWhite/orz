"""Audit integration — GAK-EVT-001.

Bridges canonical CLI journal events into the :class:`~.audit.AuditLedger`.
The audit ledger provides a retained metadata-only projection with
hash-chained events, source-sequence tracking, and a signed seal receipt
that can be independently verified after raw payloads are deleted.

Key design decisions:
- Every journal event is also an audit event — no gap between "journal"
  and "audit"
- Source kinds are assigned by event type (kernel / session / provider /
  runtime)
- Model output is marked ``derived_unverified`` (the model's claims are
  not ground truth)
- The terminal event is marked exactly-once and final
"""

from __future__ import annotations

import json
from typing import Any, Callable

from .audit import AuditLedger, RAW_RETENTION_CATEGORIES, TERMINAL_OUTCOMES
from .conversation import ConversationNamespace
from .keystore import InstallationKeyStore
from .utils import canonical_bytes


# ══════════════════════════════════════════════════════════════════════════════
# Event type → audit mapping tables
# ══════════════════════════════════════════════════════════════════════════════

# source_kind per event_type
_SOURCE_KIND: dict[str, str] = {
    "run_preflight": "kernel",
    "instruction_provenance_gate": "kernel",
    "tool_availability_check": "kernel",
    "orientation_checkpoint": "kernel",
    "run_started": "kernel",
    "gate_decision": "session",
    "model_request": "provider",
    "model_output": "provider",
    "artifact_registered": "runtime",
    "run_finished": "kernel",
    "run_failed": "kernel",
    "run_cancelled": "kernel",
    "error_event": "kernel",
}


# provenance_status per event_type — model output is never ground truth
_PROVENANCE: dict[str, str] = {
    "model_output": "derived_unverified",
    "model_request": "direct",
    "run_preflight": "direct",
    "instruction_provenance_gate": "direct",
    "tool_availability_check": "direct",
    "orientation_checkpoint": "direct",
    "run_started": "direct",
    "gate_decision": "direct",
    "artifact_registered": "direct",
    "run_finished": "direct",
    "run_failed": "direct",
    "run_cancelled": "direct",
    "error_event": "direct",
}


# retention category for raw payload
_RETENTION: dict[str, str] = {
    "model_request": "raw_provider_payload",
    "model_output": "raw_provider_payload",
    "run_preflight": "full_stdout_stderr",
    "instruction_provenance_gate": "full_stdout_stderr",
    "tool_availability_check": "full_stdout_stderr",
    "orientation_checkpoint": "full_stdout_stderr",
    "run_started": "full_stdout_stderr",
    "gate_decision": "full_stdout_stderr",
    "artifact_registered": "full_stdout_stderr",
    "run_finished": "full_stdout_stderr",
    "run_failed": "full_stdout_stderr",
    "run_cancelled": "full_stdout_stderr",
    "error_event": "full_stdout_stderr",
}

# ── source completeness per event_type (when the source is definitely complete) ──
_SOURCE_COMPLETENESS: dict[str, str] = {
    "run_preflight": "complete",
    "instruction_provenance_gate": "complete",
    "tool_availability_check": "complete",
    "orientation_checkpoint": "complete",
    "run_started": "complete",
    "gate_decision": "complete",
    "artifact_registered": "complete",
    "error_event": "complete",
    # model events: complete only when real_network_used=True
}

# ── terminal outcome mapping ──
_TERMINAL_OUTCOME: dict[str, str] = {
    "run_finished": "succeeded",
    "run_failed": "failed",
    "run_cancelled": "cancelled",
}


# ══════════════════════════════════════════════════════════════════════════════
# Public API
# ══════════════════════════════════════════════════════════════════════════════


def build_audit_writer(
    namespace: ConversationNamespace,
    key_store: InstallationKeyStore,
    *,
    run_id: str,
) -> tuple[
    Callable[[dict[str, Any]], None],  # on_event callback
    Callable[[], dict[str, Any]],       # seal function → receipt
]:
    """Build an audit-writing callback and seal closure.

    The returned ``on_event`` callback should be called for every journal
    event dict emitted by the canonical CLI.  After the last event, call
    ``seal()`` to close the ledger and produce a signed audit seal receipt.

    Parameters
    ----------
    namespace:
        The conversation namespace for this run.  Artifacts are written
        here; the seal receipt is stored under the namespace receipts root.
    key_store:
        Installation key store used to sign the seal receipt.
    run_id:
        Canonical run ID shared by all journal events in this run.

    Returns
    -------
    :
        A 2-tuple of ``(on_event, seal)``.

    Example
    -------

    .. code-block:: python

        on_audit, seal_audit = build_audit_writer(namespace, key_store, run_id="RUN-001")
        for journal_event in journal_events:
            on_audit(journal_event)
        receipt = seal_audit()
    """
    ledger = AuditLedger(namespace=namespace, key_store=key_store)
    # Per-(source_kind, source_id) sequence counters
    _sequences: dict[tuple[str, str], int] = {}

    def _on_event(journal_event: dict[str, Any]) -> None:
        event_type = str(journal_event.get("event_type", ""))
        if not event_type:
            return  # skip events with no type

        source_kind = _SOURCE_KIND.get(event_type, "kernel")
        source_id = _source_id_for(journal_event, run_id, event_type)

        key = (source_kind, source_id)
        seq = _sequences.get(key, 0)
        _sequences[key] = seq + 1

        # Completeness — model events are always "partial" because we
        # cannot prove completeness of an external provider's event stream
        completeness = _SOURCE_COMPLETENESS.get(event_type, "complete")
        if event_type in ("model_request", "model_output"):
            completeness = "partial"

        # Canonical event type name
        canonical_type = f"canonical_cli.{event_type}"

        # Raw payload: canonical JSON bytes of the journal event
        raw = canonical_bytes(journal_event)

        # Retention category
        retention = _RETENTION.get(event_type, "full_stdout_stderr")
        if retention not in RAW_RETENTION_CATEGORIES:
            retention = "full_stdout_stderr"

        # Provenance
        provenance = _PROVENANCE.get(event_type, "direct")

        # Source references from payload
        source_refs = _extract_source_refs(journal_event)

        # Terminal outcome
        terminal = _TERMINAL_OUTCOME.get(event_type)

        # Facts — type-level metadata only
        facts = _build_facts(journal_event)

        ledger.append(
            source_kind=source_kind,
            source_id=source_id,
            source_sequence=seq,
            source_completeness=completeness,
            canonical_event_type=canonical_type,
            raw_payload=raw,
            raw_retention_category=retention,
            provenance_status=provenance,
            source_refs=source_refs,
            occurred_at=journal_event.get("timestamp"),
            correlation=None,
            facts=facts,
            compaction=None,
            terminal_outcome=terminal,
        )

    def _seal() -> dict[str, Any]:
        """Seal the audit ledger and return the signed receipt."""
        # Collect observed source kinds from the sequence keys
        observed_kinds: set[str] = {k for (k, _) in _sequences}
        source_completeness: dict[str, str] = {}
        for kind in ("acp", "session", "provider", "supervisor"):
            source_completeness[kind] = "complete" if kind in observed_kinds else "unknown"
        source_completeness["kernel"] = "complete" if "kernel" in observed_kinds else "unknown"
        source_completeness["runtime"] = "complete" if "runtime" in observed_kinds else "unknown"
        # Provider events are always "partial" — we cannot prove completeness
        # of an external model provider's event stream
        if "provider" in observed_kinds:
            source_completeness["provider"] = "partial"
        return ledger.seal(source_completeness=source_completeness)

    return _on_event, _seal


# ══════════════════════════════════════════════════════════════════════════════
# helpers
# ══════════════════════════════════════════════════════════════════════════════


def _source_id_for(
    journal_event: dict[str, Any],
    run_id: str,
    event_type: str,
) -> str:
    """Derive a stable source_id from the event.

    Most events use *run_id* directly.  Per-source events (model, session)
    append a suffix so they are tracked on independent sequences.
    """
    kind = _SOURCE_KIND.get(event_type, "kernel")
    if kind == "provider":
        payload = journal_event.get("payload", {}) or {}
        model = payload.get("model_id", "unknown")
        return f"{run_id}-provider-{model}"
    if kind == "session":
        payload = journal_event.get("payload", {}) or {}
        receipt = payload.get("receipt_sha256", "unknown")
        return f"{run_id}-session-gate"
    if kind == "runtime":
        payload = journal_event.get("payload", {}) or {}
        artifact = payload.get("artifact_sha256", payload.get("artifact_path", "unknown"))
        return f"{run_id}-runtime-{str(artifact)[:16]}"
    return run_id


def _extract_source_refs(journal_event: dict[str, Any]) -> list[str]:
    """Extract audit source references from a journal event's payload.

    Returns a list of stable identifiers that link this event to its
    source material.  At minimum returns the event's own sha256.
    """
    refs: list[str] = []
    sha = journal_event.get("event_sha256", "")
    if sha:
        refs.append(sha)
    payload = journal_event.get("payload", {}) or {}
    # Harvest receipt/artifact sha256s from the payload
    for key in (
        "receipt_sha256", "context_sha256", "answer_packet_sha256",
        "artifact_sha256", "checkpoint_sha256",
        "task_contract_sha256", "tool_availability_report_sha256",
    ):
        val = payload.get(key, "")
        if isinstance(val, str) and len(val) == 64:
            refs.append(val)
    return sorted(set(refs))


def _build_facts(journal_event: dict[str, Any]) -> dict[str, Any]:
    """Build type-level metadata facts from the event.

    Facts must be metadata-safe — booleans, small integers, or
    identifier/status tokens.  No free-text or user content.
    """
    event_type = str(journal_event.get("event_type", ""))
    payload = journal_event.get("payload", {}) or {}
    facts: dict[str, Any] = {"event_type": event_type}

    if event_type == "run_preflight":
        facts["real_network_allowed"] = bool(payload.get("real_network_allowed", False))
        facts["model_id"] = str(payload.get("model_id", ""))

    elif event_type == "instruction_provenance_gate":
        facts["receipt_sha256"] = str(payload.get("receipt_sha256", ""))

    elif event_type == "tool_availability_check":
        facts["available_count"] = int(payload.get("available_count", 0))
        facts["unavailable_count"] = int(payload.get("unavailable_count", 0))

    elif event_type == "gate_decision":
        facts["decision"] = str(payload.get("decision", ""))
        facts["reference_count"] = int(payload.get("reference_count", 0))

    elif event_type == "model_request":
        facts["provider"] = str(payload.get("provider", ""))
        facts["real_network_used"] = bool(payload.get("real_network_used", False))

    elif event_type == "model_output":
        facts["real_network_used"] = bool(payload.get("real_network_used", False))
        facts["schema_valid"] = bool(payload.get("structured_output_valid", True))

    elif event_type in ("run_finished", "run_failed", "run_cancelled"):
        facts["status"] = str(payload.get("status", event_type))

    return facts
