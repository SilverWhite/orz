"""Case Retrieval Guard — blind-first enforcement against anchoring bias.

Implements the requirement from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

    Blind-first principle — historical case retrieval must happen
    AFTER independent precommitment to prevent anchoring effect
    and hindsight bias.

Implements the reason codes from :file:`protocol/reason-codes-v0.1.yaml`:

- ``BIAS-PRECOMMIT-001`` — precommitment must be recorded before case retrieval
- ``BIAS-SURFACE-001`` — retrieval must test disanalogies, not just similarities

**Design invariants:**

1. A :class:`BlindPrecommitment` must be recorded and timestamped BEFORE any
   case retrieval is permitted.
2. Once recorded, the precommitment is immutable — it cannot be revised after
   seeing historical cases (preventing hindsight bias).
3. Post-retrieval, the guard requires explicit recording of similarities,
   disanalogies, and conclusion-changing facts before the retrieval is
   marked complete.
4. The guard can be queried by :class:`~.evidence_kernel.EvidenceKernel` to
   satisfy the ``BIAS-PRECOMMIT-001`` gate.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from .errors import AssuranceError
from .utils import sha256_bytes, utc_now


# ══════════════════════════════════════════════════════════════════════════════
# data types
# ══════════════════════════════════════════════════════════════════════════════


class GuardState(Enum):
    """State machine for the blind-first retrieval guard."""

    IDLE = "idle"                          # No precommitment yet
    PRECOMMITTED = "precommitted"          # Blind judgment recorded
    RETRIEVING = "retrieving"              # Historical cases being examined
    RETRIEVED = "retrieved"                # Retrieval complete with analysis
    REVISION_RECORDED = "revision_recorded"  # Post-retrieval reasoning revision done


@dataclass(frozen=True)
class BlindPrecommitment:
    """An immutable, timestamped blind judgment recorded before case retrieval.

    Once created, the precommitment cannot be modified — this is the
    mechanism that prevents hindsight bias.  Any revision after retrieval
    is recorded separately as a :class:`RetrievalRecord`.
    """

    case_id: str
    precommitment_text: str
    proposed_gates: dict[str, str]     # gate_id → decision
    claim_candidates: list[str]
    claim_strength: str
    timestamp: str                     # ISO-8601 UTC
    content_sha256: str                # hash of precommitment content


@dataclass
class RetrievalRecord:
    """What was learned from historical case retrieval and how it changed
    the agent's reasoning.

    The record explicitly separates similarities (which risk anchoring)
    from disanalogies (which guard against surface-level pattern matching).
    """

    case_id: str
    retrieved_case_ids: list[str]
    similarities: list[str]            # "This case is similar because..."
    disanalogies: list[str]            # "This case differs because..."
    conclusion_changing_facts: list[str]  # "These facts change the conclusion..."
    reasoning_revision: str            # How the agent's reasoning changed
    retrieved_at: str = field(default_factory=utc_now)


# ══════════════════════════════════════════════════════════════════════════════
# CaseRetrievalGuard
# ══════════════════════════════════════════════════════════════════════════════


class CaseRetrievalGuard:
    """Enforce blind-first ordering for historical case retrieval.

    Usage::

        guard = CaseRetrievalGuard()

        # Step 1: Record blind precommitment BEFORE seeing any historical cases.
        precommitment = guard.precommit(
            case_id="FEP-REG-001",
            precommitment_text="Based on visible facts alone, I believe...",
            proposed_gates={"EVD-PROVENANCE-001": "defer"},
            claim_candidates=["The evidence suggests X."],
            claim_strength="observed_fragment_only",
        )

        # Step 2: Retrieve historical cases (guard enforces precommitment exists).
        guard.begin_retrieval("FEP-REG-001")

        # Step 3: Record what was learned.
        guard.complete_retrieval(
            case_id="FEP-REG-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Both involve missing CLI flags."],
            disanalogies=["FEP-REG-005 had a default; this case has no default."],
            conclusion_changing_facts=["No default exists in this version."],
            reasoning_revision="Revised: the flag is truly absent.",
        )

        # Query for EvidenceKernel integration.
        precommitment_satisfied = guard.has_valid_precommitment("FEP-REG-001")
    """

    def __init__(self) -> None:
        self._precommitments: dict[str, BlindPrecommitment] = {}
        self._retrievals: dict[str, RetrievalRecord] = {}
        self._states: dict[str, GuardState] = {}

    # ── state queries ─────────────────────────────────────────────────────

    def state(self, case_id: str) -> GuardState:
        """Return the current guard state for *case_id*."""
        return self._states.get(case_id, GuardState.IDLE)

    def has_valid_precommitment(self, case_id: str) -> bool:
        """Return ``True`` if a valid precommitment exists for *case_id*.

        This is the integration point for
        :class:`~.evidence_kernel.EvidenceKernel`'s ``BIAS-PRECOMMIT-001``
        gate — the gate can query this method to determine whether a
        timestamped precommitment was recorded before case retrieval.
        """
        state = self._states.get(case_id, GuardState.IDLE)
        return state in (
            GuardState.PRECOMMITTED,
            GuardState.RETRIEVING,
            GuardState.RETRIEVED,
            GuardState.REVISION_RECORDED,
        )

    def has_disanalogies(self, case_id: str) -> bool:
        """Return ``True`` if disanalogies were recorded for *case_id*.

        Integration point for ``BIAS-SURFACE-001`` — retrieval must
        test disanalogies, not just surface similarities.
        """
        record = self._retrievals.get(case_id)
        if record is None:
            return False
        return len(record.disanalogies) > 0

    # ── precommitment ─────────────────────────────────────────────────────

    def precommit(
        self,
        *,
        case_id: str,
        precommitment_text: str,
        proposed_gates: dict[str, str],
        claim_candidates: list[str],
        claim_strength: str,
    ) -> BlindPrecommitment:
        """Record a blind precommitment for *case_id*.

        Must be called BEFORE any case retrieval.  The precommitment is
        timestamped and immutable — it cannot be modified after creation.

        Raises :class:`AssuranceError` if:
        - A precommitment already exists for *case_id* (no overwriting).
        - *precommitment_text* is empty.
        """
        if case_id in self._precommitments:
            raise AssuranceError(
                f"precommitment already exists for '{case_id}' — "
                "precommitments are immutable"
            )
        if not precommitment_text.strip():
            raise AssuranceError(
                f"precommitment text must not be empty for '{case_id}'"
            )

        timestamp = utc_now()
        content_sha256 = sha256_bytes(
            f"{case_id}\n{precommitment_text}\n{timestamp}".encode("utf-8")
        )

        record = BlindPrecommitment(
            case_id=case_id,
            precommitment_text=precommitment_text,
            proposed_gates=dict(proposed_gates),
            claim_candidates=list(claim_candidates),
            claim_strength=claim_strength,
            timestamp=timestamp,
            content_sha256=content_sha256,
        )
        self._precommitments[case_id] = record
        self._states[case_id] = GuardState.PRECOMMITTED
        return record

    # ── retrieval lifecycle ───────────────────────────────────────────────

    def begin_retrieval(self, case_id: str) -> None:
        """Begin the historical case retrieval phase.

        Requires a valid precommitment to exist.  Transitions state from
        ``PRECOMMITTED`` to ``RETRIEVING``.

        Raises :class:`AssuranceError` if no precommitment exists.
        """
        state = self._states.get(case_id, GuardState.IDLE)
        if state == GuardState.IDLE:
            raise AssuranceError(
                f"cannot retrieve cases for '{case_id}' — "
                "a blind precommitment must be recorded first "
                "(blind-first principle)"
            )
        if state != GuardState.PRECOMMITTED:
            raise AssuranceError(
                f"cannot begin retrieval for '{case_id}' — "
                f"current state is '{state.value}', expected 'precommitted'"
            )
        self._states[case_id] = GuardState.RETRIEVING

    def complete_retrieval(
        self,
        *,
        case_id: str,
        retrieved_case_ids: list[str],
        similarities: list[str],
        disanalogies: list[str],
        conclusion_changing_facts: list[str],
        reasoning_revision: str,
    ) -> RetrievalRecord:
        """Complete the retrieval phase with structured analysis.

        Requires the guard to be in ``RETRIEVING`` state.  Records
        similarities, disanalogies, and conclusion-changing facts.
        Transitions state to ``RETRIEVED``.

        Raises :class:`AssuranceError` if not in ``RETRIEVING`` state
        or if disanalogies are empty (surface-only retrieval is blocked).
        """
        state = self._states.get(case_id, GuardState.IDLE)
        if state != GuardState.RETRIEVING:
            raise AssuranceError(
                f"cannot complete retrieval for '{case_id}' — "
                f"current state is '{state.value}', expected 'retrieving'"
            )
        if not disanalogies:
            raise AssuranceError(
                f"retrieval for '{case_id}' must include at least one "
                "disanalogy — surface-similarity-only retrieval is blocked "
                "(BIAS-SURFACE-001)"
            )

        record = RetrievalRecord(
            case_id=case_id,
            retrieved_case_ids=list(retrieved_case_ids),
            similarities=list(similarities),
            disanalogies=list(disanalogies),
            conclusion_changing_facts=list(conclusion_changing_facts),
            reasoning_revision=reasoning_revision,
        )
        self._retrievals[case_id] = record
        self._states[case_id] = GuardState.RETRIEVED
        return record

    def record_revision(
        self,
        case_id: str,
        *,
        reasoning_revision: str,
    ) -> None:
        """Record a post-retrieval reasoning revision.

        Transitions state from ``RETRIEVED`` to ``REVISION_RECORDED``.
        The original precommitment remains immutable — the revision is
        additive, not a replacement.
        """
        state = self._states.get(case_id, GuardState.IDLE)
        if state != GuardState.RETRIEVED:
            raise AssuranceError(
                f"cannot record revision for '{case_id}' — "
                f"current state is '{state.value}', expected 'retrieved'"
            )
        if case_id in self._retrievals:
            self._retrievals[case_id].reasoning_revision = reasoning_revision
        self._states[case_id] = GuardState.REVISION_RECORDED

    # ── queries ───────────────────────────────────────────────────────────

    def get_precommitment(self, case_id: str) -> BlindPrecommitment | None:
        """Return the precommitment for *case_id*, or ``None``."""
        return self._precommitments.get(case_id)

    def get_retrieval(self, case_id: str) -> RetrievalRecord | None:
        """Return the retrieval record for *case_id*, or ``None``."""
        return self._retrievals.get(case_id)

    def precommitment_count(self) -> int:
        return len(self._precommitments)

    def retrieval_count(self) -> int:
        return len(self._retrievals)

    # ── audit summary ─────────────────────────────────────────────────────

    def audit_summary(self, case_id: str) -> dict[str, Any]:
        """Return an audit summary for *case_id* suitable for evaluation
        journal inclusion.

        The summary records whether the blind-first protocol was followed
        and whether disanalogies were tested.
        """
        pre = self.get_precommitment(case_id)
        ret = self.get_retrieval(case_id)
        return {
            "case_id": case_id,
            "blind_first_satisfied": self.has_valid_precommitment(case_id),
            "precommitment_timestamp": pre.timestamp if pre else None,
            "precommitment_sha256": pre.content_sha256 if pre else None,
            "retrieval_performed": ret is not None,
            "retrieved_case_count": len(ret.retrieved_case_ids) if ret else 0,
            "disanalogies_recorded": len(ret.disanalogies) if ret else 0,
            "similarities_recorded": len(ret.similarities) if ret else 0,
            "conclusion_changing_facts": (
                len(ret.conclusion_changing_facts) if ret else 0
            ),
            "bias_surface_satisfied": self.has_disanalogies(case_id),
        }
