"""Session governor — namespace lifecycle controller for runtime adapters.

A thin wrapper around :class:`ConversationNamespace` that ensures all
adapter artifact paths (gate receipts, answer packets, journal events,
receipts) are routed through the same namespace controller.  Every
write is gated on the namespace being in ``active`` state, and every
read enforces cross-session isolation.

This is NOT a new abstraction layer — it delegates directly to
``ConversationNamespace`` methods.  It exists to provide adapter
authors with a single, obvious entry point rather than requiring
them to know the retention-category taxonomy.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .conversation import ConversationNamespace
from .errors import AssuranceError
from .utils import canonical_bytes, sha256_bytes, utc_now


# ── retention category routing ──

GATE_RECEIPT_CATEGORY = "temporary_lifecycle_receipt"
CHECKPOINT_CATEGORY = "temporary_checkpoint"
ANSWER_CATEGORY = "raw_provider_payload"
JOURNAL_CATEGORY = "temporary_lifecycle_receipt"


class SessionGovernor:
    """Namespace lifecycle controller for a single adapter run session.

    All artifact writes are routed through the governor so that every
    path — gate receipt, checkpoint, answer packet, journal event,
    run receipt — is subject to the same retention policy and
    cross-session isolation enforced by the underlying
    :class:`ConversationNamespace`.
    """

    def __init__(self, namespace: ConversationNamespace) -> None:
        self.namespace = namespace
        self._session_id = f"SG-{namespace.conversation_id}"
        self._created_at = utc_now()
        self._closed = False
        self._artifact_index: list[dict[str, str]] = []

    # ── read-only properties ──

    @property
    def conversation_id(self) -> str:
        return self.namespace.conversation_id

    @property
    def envelope_id(self) -> str:
        state = self.namespace.state()
        return state["envelope_id"]

    @property
    def is_active(self) -> bool:
        return self.namespace.state()["state"] == "active"

    # ── artifact writes ──

    def _require_active(self) -> None:
        if self._closed:
            raise AssuranceError("session governor is closed — no writes allowed")
        if not self.is_active:
            raise AssuranceError(
                f"namespace {self.conversation_id} is not active — "
                "artifact writes are denied"
            )

    def _write_and_index(
        self, category: str, relative_path: str, data: bytes
    ) -> Path:
        self._require_active()
        path = self.namespace.write_artifact(category, relative_path, data)
        self._artifact_index.append({
            "category": category,
            "relative_path": relative_path,
            "sha256": sha256_bytes(data),
            "size_bytes": len(data),
        })
        return path

    def write_gate_receipt(self, name: str, data: dict[str, Any]) -> Path:
        """Write a gate receipt artifact."""
        return self._write_and_index(
            GATE_RECEIPT_CATEGORY,
            f"{name}.json",
            canonical_bytes(data),
        )

    def write_gate_receipt_bytes(self, name: str, data: bytes) -> Path:
        """Write a gate receipt from raw bytes (e.g. pre-serialized JSON)."""
        return self._write_and_index(GATE_RECEIPT_CATEGORY, f"{name}.json", data)

    def write_checkpoint(self, name: str, data: dict[str, Any]) -> Path:
        """Write an orientation/runtime checkpoint."""
        return self._write_and_index(
            CHECKPOINT_CATEGORY,
            f"{name}.json",
            canonical_bytes(data),
        )

    def write_answer_packet(self, data: dict[str, Any]) -> Path:
        """Write the model answer packet."""
        return self._write_and_index(
            ANSWER_CATEGORY,
            "answer-packet.json",
            canonical_bytes(data),
        )

    def write_run_manifest(self, data: dict[str, Any]) -> Path:
        """Write the run manifest."""
        return self._write_and_index(
            GATE_RECEIPT_CATEGORY,
            "run-manifest.json",
            canonical_bytes(data),
        )

    def write_task_contract(self, data: dict[str, Any]) -> Path:
        """Write the task contract copy."""
        return self._write_and_index(
            GATE_RECEIPT_CATEGORY,
            "task-contract.json",
            canonical_bytes(data),
        )

    def write_journal_line(self, line: bytes) -> None:
        """Append a line to the runtime journal.

        Journal appends are cumulative — each call appends to the same
        ``events.jsonl`` artifact.  The caller must manage hash-chaining
        and event sequencing.
        """
        self._require_active()
        journal_path = self.namespace.artifacts_root / JOURNAL_CATEGORY / "events.jsonl"
        journal_path.parent.mkdir(parents=True, exist_ok=True)
        with journal_path.open("ab") as handle:
            handle.write(line)
            handle.flush()

    def write_run_receipt(self, data: dict[str, Any]) -> Path:
        """Write the final run receipt to the receipts directory."""
        self._require_active()
        receipt_path = self.namespace.receipts_root / "canonical-cli-run-receipt.json"
        receipt_path.parent.mkdir(parents=True, exist_ok=True)
        receipt_path.write_text(
            json.dumps(data, indent=2, sort_keys=True, ensure_ascii=False),
            encoding="utf-8",
        )
        return receipt_path

    # ── reads (with cross-session enforcement) ──

    def read_artifact(self, category: str, relative_path: str) -> bytes:
        """Read an artifact from this session's namespace."""
        return self.namespace.read_artifact(
            self.conversation_id, category, relative_path
        )

    def read_gate_receipt(self, name: str) -> dict[str, Any]:
        """Read a gate receipt and parse as JSON."""
        import json as _json
        raw = self.read_artifact(GATE_RECEIPT_CATEGORY, f"{name}.json")
        return _json.loads(raw.decode("utf-8"))

    # ── integrity checks ──

    def verify_namespace_integrity(self) -> dict[str, Any]:
        """Verify the namespace is intact and in the expected state.

        Returns a dict with ``valid``, ``state``, ``artifact_count``,
        and ``errors`` keys.
        """
        errors: list[str] = []
        try:
            state = self.namespace.state()
        except AssuranceError as exc:
            return {
                "valid": False,
                "state": None,
                "artifact_count": len(self._artifact_index),
                "errors": [str(exc)],
            }

        if state["state"] not in ("active", "archiving"):
            errors.append(
                f"unexpected namespace state for session: {state['state']}"
            )

        if state["conversation_id"] != self.conversation_id:
            errors.append("namespace conversation_id mismatch")

        return {
            "valid": not errors,
            "state": state["state"],
            "artifact_count": len(self._artifact_index),
            "errors": errors,
        }

    def artifact_manifest(self) -> list[dict[str, str]]:
        """Return the list of all artifacts written through this governor."""
        return list(self._artifact_index)

    # ── lifecycle ──

    def close_session(self) -> dict[str, Any]:
        """Mark the session as closed.

        After this call, no further writes are permitted.  The namespace
        itself remains active (archiving is handled separately by
        :class:`ArchiveController`).

        Returns a closure receipt.
        """
        if self._closed:
            raise AssuranceError("session is already closed")
        self._closed = True
        return {
            "schema_version": "0.1.0-draft",
            "receipt_kind": "session_governor_closure",
            "session_id": self._session_id,
            "conversation_id": self.conversation_id,
            "envelope_id": self.envelope_id,
            "created_at": self._created_at,
            "closed_at": utc_now(),
            "artifact_count": len(self._artifact_index),
            "artifact_manifest": self._artifact_index,
        }
