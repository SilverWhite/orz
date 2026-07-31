"""Finding Registry + Permission Record — D3.22.

Enforces the architectural invariant from
:file:`architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2` §4.3:

    **Scanner outputs finding ID + evidence + confidence.**
    **Permission independently records allow/deny/ask + basis.**
    **Heuristic findings must not masquerade as security proofs,**
    **and permissions must not erase findings.**

Two data types with separate lifecycles:

* :class:`Finding` — immutable, content-addressed observation from a scanner.
  Does NOT contain permission recommendations or block/allow signals.
* :class:`PermissionRecord` — dissociated decision record.  References
  findings by SHA-256 identity only; never embeds finding content.

And a session-scoped registry:

* :class:`FindingRegistry` — thread-safe container for findings produced
  during a single adapter session.
"""

from __future__ import annotations

import secrets
import threading
from dataclasses import dataclass, field
from typing import Any

from .contracts import validate_contract
from .utils import canonical_bytes, sha256_bytes, utc_now


FINDING_SCHEMA = "finding-v0.1.schema.json"
PERMISSION_RECORD_SCHEMA = "permission-record-v0.1.schema.json"


# ══════════════════════════════════════════════════════════════════════════════
# Finding
# ══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class Finding:
    """Immutable, content-addressed scanner observation.

    Design invariant: this dataclass deliberately has NO ``recommended_action``,
    ``should_block``, or any permission-field.  Findings observe; they do not
    decide.
    """

    finding_id: str
    source_scanner: str
    category: str
    severity: str          # "low" | "medium" | "high" | "critical"
    snippet_hash: str      # SHA-256 of matched content (never raw text)
    evidence: str          # human-readable justification (never raw content)
    created_at: str
    location: str = ""
    rule_id: str = ""
    channel: str = "mechanical"  # "mechanical" | "semantic" | "manual"

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a schema-conformant dict (without finding_sha256)."""
        return {
            "schema_version": "0.1.0-draft",
            "finding_kind": "assurance_finding",
            "finding_id": self.finding_id,
            "created_at": self.created_at,
            "source_scanner": self.source_scanner,
            "category": self.category,
            "severity": self.severity,
            "location": self.location,
            "snippet_hash": self.snippet_hash,
            "evidence": self.evidence,
            "rule_id": self.rule_id,
            "channel": self.channel,
            "finding_sha256": "",
        }

    def compute_sha256(self) -> str:
        """Content-address this finding (excludes finding_sha256 itself)."""
        d = self.to_dict()
        d.pop("finding_sha256", None)
        return sha256_bytes(canonical_bytes(d))

    @classmethod
    def create(
        cls,
        *,
        source_scanner: str,
        category: str,
        severity: str,
        snippet_hash: str,
        evidence: str,
        location: str = "",
        rule_id: str = "",
        channel: str = "mechanical",
    ) -> Finding:
        """Factory: create a Finding with auto-generated ID and timestamp."""
        finding_id = f"FIND-{secrets.token_hex(8).upper()}"
        created_at = utc_now()
        finding = cls(
            finding_id=finding_id,
            source_scanner=source_scanner,
            category=category,
            severity=severity,
            snippet_hash=snippet_hash,
            evidence=evidence,
            created_at=created_at,
            location=location,
            rule_id=rule_id,
            channel=channel,
        )
        # Validate after computing sha256
        d = finding.to_dict()
        d["finding_sha256"] = finding.compute_sha256()
        validate_contract(d, FINDING_SCHEMA, label="Finding")
        return finding


# ══════════════════════════════════════════════════════════════════════════════
# PermissionRecord
# ══════════════════════════════════════════════════════════════════════════════


@dataclass(frozen=True)
class PermissionRecord:
    """A single permission decision, dissociated from findings.

    Design invariant: this dataclass references findings by SHA-256 identity
    only (``referenced_finding_ids``).  It does NOT contain severity,
    evidence, or finding content.  Decisions and observations are separate
    audit artifacts.
    """

    record_id: str
    session_run_id: str
    decision: str        # "allow_once" | "cancelled" | "denied" | "deferred"
    authority: str       # "user" | "adapter" | "policy"
    tool_name: str
    referenced_finding_ids: list[str] = field(default_factory=list)
    basis: str = ""
    created_at: str = ""

    def to_dict(self) -> dict[str, Any]:
        """Serialize to a schema-conformant dict (without record_sha256)."""
        return {
            "schema_version": "0.1.0-draft",
            "record_kind": "assurance_permission_record",
            "record_id": self.record_id,
            "created_at": self.created_at,
            "session_run_id": self.session_run_id,
            "decision": self.decision,
            "authority": self.authority,
            "tool_name": self.tool_name,
            "referenced_finding_ids": list(self.referenced_finding_ids),
            "basis": self.basis,
            "record_sha256": "",
        }

    def compute_sha256(self) -> str:
        d = self.to_dict()
        d.pop("record_sha256", None)
        return sha256_bytes(canonical_bytes(d))

    @classmethod
    def create(
        cls,
        *,
        session_run_id: str,
        decision: str,
        authority: str,
        tool_name: str,
        referenced_finding_ids: list[str] | None = None,
        basis: str = "",
    ) -> PermissionRecord:
        record_id = f"PERM-{secrets.token_hex(8).upper()}"
        created_at = utc_now()
        record = cls(
            record_id=record_id,
            session_run_id=session_run_id,
            decision=decision,
            authority=authority,
            tool_name=tool_name,
            referenced_finding_ids=list(referenced_finding_ids or []),
            basis=basis,
            created_at=created_at,
        )
        d = record.to_dict()
        d["record_sha256"] = record.compute_sha256()
        validate_contract(d, PERMISSION_RECORD_SCHEMA, label="PermissionRecord")
        return record


# ══════════════════════════════════════════════════════════════════════════════
# FindingRegistry
# ══════════════════════════════════════════════════════════════════════════════


class FindingRegistry:
    """Thread-safe, session-scoped container for :class:`Finding` instances.

    Findings are registered by scanners during an adapter session.
    A :class:`PermissionRecord` can reference any finding in the registry
    by its :attr:`Finding.finding_id` when a permission decision is made.

    Usage::

        registry = FindingRegistry()
        finding = Finding.create(source_scanner="leak_scanner", ...)
        registry.register(finding)
        # ... later, when permission decision is made:
        record = PermissionRecord.create(
            ...,
            referenced_finding_ids=registry.list_ids(),
        )
    """

    def __init__(self) -> None:
        self._findings: dict[str, Finding] = {}
        self._lock = threading.Lock()

    def register(self, finding: Finding) -> None:
        """Register a finding (idempotent — same-ID overwrite is a no-op for
        the same content hash, or raises for mismatched content)."""
        with self._lock:
            existing = self._findings.get(finding.finding_id)
            if existing is not None:
                if existing.compute_sha256() != finding.compute_sha256():
                    raise ValueError(
                        f"Finding ID collision: {finding.finding_id} already "
                        f"registered with different content"
                    )
                return  # idempotent
            self._findings[finding.finding_id] = finding

    def list_ids(self) -> list[str]:
        """Return all registered finding IDs (sorted for determinism)."""
        with self._lock:
            return sorted(self._findings.keys())

    def list_active(self) -> list[Finding]:
        """Return all registered findings."""
        with self._lock:
            return list(self._findings.values())

    def __len__(self) -> int:
        with self._lock:
            return len(self._findings)

    def __contains__(self, finding_id: str) -> bool:
        with self._lock:
            return finding_id in self._findings


# ══════════════════════════════════════════════════════════════════════════════
# Convenience: record a permission decision with a registry snapshot
# ══════════════════════════════════════════════════════════════════════════════


def record_permission_decision(
    *,
    registry: FindingRegistry | None,
    session_run_id: str,
    decision: str,
    authority: str,
    tool_name: str,
    basis: str = "",
) -> PermissionRecord:
    """Create a :class:`PermissionRecord` referencing all current findings.

    This is the primary integration point for :func:`run_grok_acp_once` —
    call it whenever a permission decision (user or auto) is made.

    Parameters
    ----------
    registry:
        The session's :class:`FindingRegistry`.  If ``None``, no findings
        are referenced.
    session_run_id:
        The ``run_id`` from the runtime receipt.
    decision:
        One of ``"allow_once"``, ``"cancelled"``, ``"denied"``.
    authority:
        ``"user"`` for TUI-interactive decisions, ``"adapter"`` for
        auto-decisions, ``"policy"`` for programmatic gates.
    tool_name:
        Human-readable tool name from the permission request.
    basis:
        Free-text rationale (e.g. "user approved via TUI dialog",
        "auto-allowed: allow_once available").
    """
    refs = [f.compute_sha256() for f in registry.list_active()] if registry is not None else []
    return PermissionRecord.create(
        session_run_id=session_run_id,
        decision=decision,
        authority=authority,
        tool_name=tool_name,
        referenced_finding_ids=refs,
        basis=basis,
    )
