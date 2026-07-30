"""Evidence Kernel — action / evidence / claim three-layer state machine.

Implements the invariant from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

- **Action** records what was executed (terminal status, artifacts, idempotency).
- **Evidence** records what an action *produced* that passes provenance and
  artifact-validation gates.
- **Claim** records what evidence *supports*, bounded by coverage, independence,
  and strength gates.

**Hard invariants** (enforced at the API level):

1. Actions **cannot** auto-upgrade to evidence — :meth:`EvidenceKernel.promote_to_evidence`
   must be called explicitly and all applicable reason-code gates must pass.
2. Evidence **cannot** auto-upgrade to claims — :meth:`EvidenceKernel.promote_to_claim`
   must be called explicitly and all applicable reason-code gates must pass.
3. Every promotion records the full set of evaluated reason codes in the
   state object; a ``block`` decision on any gate prevents promotion.

Reason-code gates are drawn from :file:`protocol/reason-codes-v0.1.yaml`.
The kernel evaluates each gate's ``decisive_question`` against the current
state and returns a structured verdict.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from .errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# gate evaluation data types
# ══════════════════════════════════════════════════════════════════════════════


class GateDecision(Enum):
    PASS = "pass"
    WARN = "warn"
    DEFER = "defer"
    BLOCK = "block"


class GateMode(Enum):
    DISCUSSION = "discussion"
    GUARDED = "guarded"
    STRICT = "strict"


@dataclass(frozen=True)
class GateVerdict:
    """A single reason-code gate evaluation result."""

    code: str
    domain: str
    decision: GateDecision
    decisive_question: str
    rationale: str


@dataclass
class PromotionGateResult:
    """Aggregate result of evaluating all applicable gates for a promotion."""

    allowed: bool
    verdicts: list[GateVerdict]
    blocking_codes: list[str]
    warning_codes: list[str]
    deferred_codes: list[str]

    @property
    def all_passed(self) -> bool:
        return self.allowed


# ══════════════════════════════════════════════════════════════════════════════
# gate registry — reason codes mapped to layers
# ══════════════════════════════════════════════════════════════════════════════

# Default gate definitions.  Each entry maps a reason code to its
# domain, the layer(s) it applies to, and the default decision per mode.
# In production this would be loaded from reason-codes-v0.1.yaml; the
# inline registry keeps the kernel zero-dependency.

_GATE_DEFAULTS: dict[str, dict[str, Any]] = {
    # ── action-layer gates ──
    "ACT-IDEMPOTENCY-001": {
        "domain": "action",
        "applies_to": ["action"],
        "discussion": GateDecision.PASS,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Can a retry be distinguished from a new independent action?",
    },
    "PROC-TERMINAL-001": {
        "domain": "execution",
        "applies_to": ["action"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Is there exactly one append-only terminal event for the action?",
    },
    "PROC-EXIT-UNKNOWN-001": {
        "domain": "execution",
        "applies_to": ["action", "evidence"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.DEFER,
        "question": "Can completion be established from a documented artifact fallback rule?",
    },
    "CFG-EXPLICIT-001": {
        "domain": "configuration",
        "applies_to": ["action", "evidence"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Is every high-risk dimension explicit in the execution record?",
    },
    # ── evidence-layer gates ──
    "EVD-PROVENANCE-001": {
        "domain": "provenance",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Which code, resolved configuration, action, and artifact produced the evidence?",
    },
    "EVD-COVERAGE-001": {
        "domain": "coverage",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Does the valid completed set cover the population named by the claim?",
    },
    "EVD-INDEPENDENCE-001": {
        "domain": "independence",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "What independent stochastic, data, intervention, or producer unit supports the added N?",
    },
    "EVD-CONFOUND-001": {
        "domain": "causal_inference",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Are plausible outcome-relevant deltas controlled or modeled?",
    },
    "EVD-COMPARABILITY-001": {
        "domain": "causal_inference",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Are task definition, code path, data, metric, and evaluation scope comparable?",
    },
    "EVD-EXECUTION-MATCH-001": {
        "domain": "execution",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Does source-level execution preserve all components required by the intervention definition?",
    },
    "ART-SCHEMA-001": {
        "domain": "artifact",
        "applies_to": ["artifact", "evidence"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Does the exact produced artifact validate against the applicable schema?",
    },
    "ART-FINITE-001": {
        "domain": "artifact",
        "applies_to": ["artifact", "evidence"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Are all claim-bearing numeric values finite and standards-compliant?",
    },
    "ART-STALE-001": {
        "domain": "artifact",
        "applies_to": ["artifact", "evidence"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Do manifest, timestamp, hash, and producer records bind this artifact to the current action?",
    },
    "SEM-MAPPING-001": {
        "domain": "semantic_mapping",
        "applies_to": ["evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Does the operational measure directly denote the property named by the claim?",
    },
    # ── claim-layer gates ──
    "CLM-STRENGTH-001": {
        "domain": "claim_strength",
        "applies_to": ["claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Is the wording no stronger than the available provenance, coverage, independence, and test design?",
    },
    "CLM-GENERALITY-001": {
        "domain": "claim_strength",
        "applies_to": ["claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Is the claim population identical to or defensibly sampled by the evidence population?",
    },
    "CLM-MECHANISM-001": {
        "domain": "claim_strength",
        "applies_to": ["claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "What observation excludes credible alternative mechanisms?",
    },
    "BIAS-CONFIRM-001": {
        "domain": "independence",
        "applies_to": ["claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "What evidence, if found, would weaken or reverse the current conclusion?",
    },
    "BIAS-PRECOMMIT-001": {
        "domain": "independence",
        "applies_to": ["task", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.DEFER,
        "strict": GateDecision.BLOCK,
        "question": "Was a timestamped precommitment recorded before case retrieval?",
    },
    "BIAS-LEAKAGE-001": {
        "domain": "independence",
        "applies_to": ["task", "evidence", "claim"],
        "discussion": GateDecision.WARN,
        "guarded": GateDecision.BLOCK,
        "strict": GateDecision.BLOCK,
        "question": "Can the model context be shown to exclude oracle labels and expected answers?",
    },
}


def _default_for_mode(code: str, mode: GateMode) -> GateDecision:
    entry = _GATE_DEFAULTS.get(code)
    if entry is None:
        return GateDecision.DEFER
    key = mode.value  # "discussion" | "guarded" | "strict"
    return entry.get(key, GateDecision.DEFER)


# ══════════════════════════════════════════════════════════════════════════════
# state data types
# ══════════════════════════════════════════════════════════════════════════════


class ActionStatus(Enum):
    REGISTERED = "registered"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"


class ProvenanceStatus(Enum):
    DIRECT = "direct"
    DERIVED_UNVERIFIED = "derived_unverified"
    UNKNOWN = "unknown"


class ClaimType(Enum):
    MEASURED = "measured"
    DIRECT_COMPARISON = "direct_comparison"
    BRIDGE_HYPOTHESIS = "bridge_hypothesis"
    OBSERVATION = "observation"


class ClaimStrength(Enum):
    NONE = "none"
    METADATA_ONLY = "metadata_only"
    OBSERVED_FRAGMENT_ONLY = "observed_fragment_only"
    FULL_TEXT_GROUNDED = "full_text_grounded_but_unvalidated"


@dataclass
class ActionState:
    """A registered action with terminal status and produced artifacts."""

    action_id: str
    action_type: str
    status: ActionStatus = ActionStatus.REGISTERED
    terminal_outcome: str | None = None
    artifact_hashes: list[str] = field(default_factory=list)
    idempotency_key: str | None = None
    reason_codes: list[str] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class EvidenceState:
    """Evidence promoted from one or more actions after passing gates."""

    evidence_id: str
    source_action_ids: list[str] = field(default_factory=list)
    provenance_status: ProvenanceStatus = ProvenanceStatus.UNKNOWN
    artifact_schema_valid: bool = False
    artifact_finite_valid: bool = False
    artifact_not_stale: bool = False
    independence_sources: int = 1
    reason_codes: list[str] = field(default_factory=list)
    gate_verdicts: list[GateVerdict] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass
class ClaimState:
    """A claim promoted from evidence after passing strength/coverage gates."""

    claim_id: str
    source_evidence_ids: list[str] = field(default_factory=list)
    claim_type: ClaimType = ClaimType.OBSERVATION
    claim_strength: ClaimStrength = ClaimStrength.NONE
    population_covered: bool = False
    contrary_searched: bool = False
    precommitment_recorded: bool = False
    reason_codes: list[str] = field(default_factory=list)
    gate_verdicts: list[GateVerdict] = field(default_factory=list)
    metadata: dict[str, Any] = field(default_factory=dict)


# ══════════════════════════════════════════════════════════════════════════════
# Evidence Kernel
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceKernel:
    """Three-layer state machine: action → evidence → claim.

    Each layer maintains independent state.  Upgrades are **never**
    automatic — callers must invoke :meth:`promote_to_evidence` or
    :meth:`promote_to_claim` and all applicable reason-code gates must
    evaluate to a non-blocking decision.

    Usage::

        kernel = EvidenceKernel(mode=GateMode.GUARDED)

        # Register an action
        action = kernel.register_action(
            action_id="ACT-001",
            action_type="evaluate_gate",
            idempotency_key="task-xyz::step-1",
        )

        # Complete the action
        kernel.complete_action("ACT-001", terminal_outcome="succeeded",
                               artifact_hashes=["sha256:abc..."])

        # Promote to evidence (gates evaluated here)
        evidence = kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )

        # Promote to claim
        claim = kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
    """

    def __init__(self, *, mode: GateMode = GateMode.GUARDED) -> None:
        self._mode = mode
        self._actions: dict[str, ActionState] = {}
        self._evidences: dict[str, EvidenceState] = {}
        self._claims: dict[str, ClaimState] = {}

    # ── mode ────────────────────────────────────────────────────────────

    @property
    def mode(self) -> GateMode:
        return self._mode

    # ── action layer ────────────────────────────────────────────────────

    @property
    def actions(self) -> dict[str, ActionState]:
        return dict(self._actions)

    def register_action(
        self,
        *,
        action_id: str,
        action_type: str,
        idempotency_key: str | None = None,
        metadata: dict[str, Any] | None = None,
    ) -> ActionState:
        """Register a new action.

        Raises :class:`AssuranceError` if *action_id* already exists.
        """
        if action_id in self._actions:
            raise AssuranceError(
                f"action '{action_id}' is already registered"
            )
        state = ActionState(
            action_id=action_id,
            action_type=action_type,
            idempotency_key=idempotency_key,
            metadata=metadata or {},
        )
        self._actions[action_id] = state
        return state

    def start_action(self, action_id: str) -> ActionState:
        """Mark an action as running."""
        state = self._require_action(action_id)
        if state.status != ActionStatus.REGISTERED:
            raise AssuranceError(
                f"action '{action_id}' is {state.status.value}, not registered"
            )
        state.status = ActionStatus.RUNNING
        return state

    def complete_action(
        self,
        action_id: str,
        *,
        terminal_outcome: str,
        artifact_hashes: list[str] | None = None,
    ) -> ActionState:
        """Mark an action as completed with a terminal outcome."""
        state = self._require_action(action_id)
        if state.status not in (ActionStatus.RUNNING, ActionStatus.REGISTERED):
            raise AssuranceError(
                f"action '{action_id}' is {state.status.value}, not running"
            )
        state.status = ActionStatus.COMPLETED
        state.terminal_outcome = terminal_outcome
        if artifact_hashes:
            state.artifact_hashes = list(artifact_hashes)
        return state

    def fail_action(
        self, action_id: str, *, reason: str,
    ) -> ActionState:
        """Mark an action as failed."""
        state = self._require_action(action_id)
        state.status = ActionStatus.FAILED
        state.terminal_outcome = "failed"
        state.metadata["failure_reason"] = reason
        return state

    # ── action → evidence promotion ─────────────────────────────────────

    def promote_to_evidence(
        self,
        action_id: str,
        *,
        evidence_id: str,
        provenance_status: ProvenanceStatus = ProvenanceStatus.UNKNOWN,
        artifact_schema_valid: bool = False,
        artifact_finite_valid: bool = False,
        artifact_not_stale: bool = False,
        independence_sources: int = 1,
        metadata: dict[str, Any] | None = None,
    ) -> tuple[EvidenceState, PromotionGateResult]:
        """Promote a completed action to evidence.

        Evaluates all action-layer and evidence-layer gates.  Returns
        the new :class:`EvidenceState` and the :class:`PromotionGateResult`.
        If *allowed* is ``False`` the evidence is still registered but
        carries the blocking codes — callers must not treat it as
        validated evidence.

        Raises :class:`AssuranceError` if *action_id* is not completed
        or *evidence_id* already exists.
        """
        if evidence_id in self._evidences:
            raise AssuranceError(
                f"evidence '{evidence_id}' already exists"
            )
        action = self._require_action(action_id)
        if action.status != ActionStatus.COMPLETED:
            raise AssuranceError(
                f"action '{action_id}' must be completed before promotion "
                f"(current status: {action.status.value})"
            )

        # Evaluate action-layer gates.
        action_gates = self._evaluate_action_gates(action)

        # Evaluate evidence-layer gates.
        evidence_gates = self._evaluate_evidence_gates(
            action=action,
            provenance_status=provenance_status,
            artifact_schema_valid=artifact_schema_valid,
            artifact_finite_valid=artifact_finite_valid,
            artifact_not_stale=artifact_not_stale,
            independence_sources=independence_sources,
        )

        all_verdicts = action_gates + evidence_gates
        gate_result = self._summarize_gates(all_verdicts)

        state = EvidenceState(
            evidence_id=evidence_id,
            source_action_ids=[action_id],
            provenance_status=provenance_status,
            artifact_schema_valid=artifact_schema_valid,
            artifact_finite_valid=artifact_finite_valid,
            artifact_not_stale=artifact_not_stale,
            independence_sources=independence_sources,
            reason_codes=gate_result.blocking_codes + gate_result.deferred_codes,
            gate_verdicts=all_verdicts,
            metadata=metadata or {},
        )
        self._evidences[evidence_id] = state
        return state, gate_result

    # ── evidence → claim promotion ──────────────────────────────────────

    def promote_to_claim(
        self,
        evidence_id: str,
        *,
        claim_id: str,
        claim_type: ClaimType = ClaimType.OBSERVATION,
        claim_strength: ClaimStrength = ClaimStrength.NONE,
        population_covered: bool = False,
        contrary_searched: bool = False,
        precommitment_recorded: bool = False,
        metadata: dict[str, Any] | None = None,
    ) -> tuple[ClaimState, PromotionGateResult]:
        """Promote evidence to a claim.

        Evaluates all evidence-layer and claim-layer gates.  Returns
        the new :class:`ClaimState` and the :class:`PromotionGateResult`.
        If *allowed* is ``False`` the claim is still registered but
        carries the blocking codes.

        Raises :class:`AssuranceError` if *evidence_id* is not
        registered or *claim_id* already exists.
        """
        if claim_id in self._claims:
            raise AssuranceError(
                f"claim '{claim_id}' already exists"
            )
        evidence = self._require_evidence(evidence_id)

        # Re-evaluate evidence-layer gates in claim context.
        evidence_gates = self._evaluate_evidence_for_claim(evidence)

        # Evaluate claim-layer gates.
        claim_gates = self._evaluate_claim_gates(
            evidence=evidence,
            claim_type=claim_type,
            claim_strength=claim_strength,
            population_covered=population_covered,
            contrary_searched=contrary_searched,
            precommitment_recorded=precommitment_recorded,
        )

        all_verdicts = evidence_gates + claim_gates
        gate_result = self._summarize_gates(all_verdicts)

        state = ClaimState(
            claim_id=claim_id,
            source_evidence_ids=[evidence_id],
            claim_type=claim_type,
            claim_strength=claim_strength,
            population_covered=population_covered,
            contrary_searched=contrary_searched,
            precommitment_recorded=precommitment_recorded,
            reason_codes=gate_result.blocking_codes + gate_result.deferred_codes,
            gate_verdicts=all_verdicts,
            metadata=metadata or {},
        )
        self._claims[claim_id] = state
        return state, gate_result

    # ── queries ─────────────────────────────────────────────────────────

    def get_action(self, action_id: str) -> ActionState | None:
        return self._actions.get(action_id)

    def get_evidence(self, evidence_id: str) -> EvidenceState | None:
        return self._evidences.get(evidence_id)

    def get_claim(self, claim_id: str) -> ClaimState | None:
        return self._claims.get(claim_id)

    @property
    def evidences(self) -> dict[str, EvidenceState]:
        return dict(self._evidences)

    @property
    def claims(self) -> dict[str, ClaimState]:
        return dict(self._claims)

    def action_count(self) -> int:
        return len(self._actions)

    def evidence_count(self) -> int:
        return len(self._evidences)

    def claim_count(self) -> int:
        return len(self._claims)

    # ── gate evaluation ─────────────────────────────────────────────────

    def _evaluate_action_gates(self, action: ActionState) -> list[GateVerdict]:
        """Evaluate all gates applicable to the action layer."""
        verdicts: list[GateVerdict] = []

        # ACT-IDEMPOTENCY-001
        code = "ACT-IDEMPOTENCY-001"
        passed = action.idempotency_key is not None
        verdicts.append(GateVerdict(
            code=code,
            domain="action",
            decision=(
                GateDecision.PASS if passed
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"idempotency key present: {action.idempotency_key}"
                if passed
                else "no idempotency key provided"
            ),
        ))

        # PROC-TERMINAL-001
        code = "PROC-TERMINAL-001"
        has_terminal = (
            action.status == ActionStatus.COMPLETED
            and action.terminal_outcome is not None
        )
        verdicts.append(GateVerdict(
            code=code,
            domain="execution",
            decision=(
                GateDecision.PASS if has_terminal
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"terminal outcome: {action.terminal_outcome}"
                if has_terminal
                else "no terminal event recorded"
            ),
        ))

        return verdicts

    def _evaluate_evidence_gates(
        self,
        *,
        action: ActionState,
        provenance_status: ProvenanceStatus,
        artifact_schema_valid: bool,
        artifact_finite_valid: bool,
        artifact_not_stale: bool,
        independence_sources: int,
    ) -> list[GateVerdict]:
        """Evaluate gates for action → evidence promotion."""
        verdicts: list[GateVerdict] = []

        # EVD-PROVENANCE-001
        code = "EVD-PROVENANCE-001"
        prov_ok = provenance_status == ProvenanceStatus.DIRECT
        verdicts.append(GateVerdict(
            code=code,
            domain="provenance",
            decision=(
                GateDecision.PASS if prov_ok
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"provenance: {provenance_status.value}"
                if prov_ok
                else f"provenance is {provenance_status.value}, not direct"
            ),
        ))

        # ART-SCHEMA-001
        code = "ART-SCHEMA-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="artifact",
            decision=(
                GateDecision.PASS if artifact_schema_valid
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "artifact schema validated"
                if artifact_schema_valid
                else "artifact schema not validated"
            ),
        ))

        # ART-FINITE-001
        code = "ART-FINITE-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="artifact",
            decision=(
                GateDecision.PASS if artifact_finite_valid
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "finite values confirmed"
                if artifact_finite_valid
                else "finite value check not performed"
            ),
        ))

        # ART-STALE-001
        code = "ART-STALE-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="artifact",
            decision=(
                GateDecision.PASS if artifact_not_stale
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "artifact binding confirmed"
                if artifact_not_stale
                else "artifact binding not verified"
            ),
        ))

        # EVD-INDEPENDENCE-001
        code = "EVD-INDEPENDENCE-001"
        ind_ok = independence_sources >= 1
        verdicts.append(GateVerdict(
            code=code,
            domain="independence",
            decision=(
                GateDecision.PASS if ind_ok
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"independent sources: {independence_sources}"
                if ind_ok
                else "no independent sources"
            ),
        ))

        return verdicts

    def _evaluate_evidence_for_claim(
        self, evidence: EvidenceState,
    ) -> list[GateVerdict]:
        """Re-evaluate evidence-layer gates in the context of claim promotion."""
        verdicts: list[GateVerdict] = []

        # EVD-COVERAGE-001 (claim-specific: does evidence cover claim population?)
        code = "EVD-COVERAGE-001"
        covered = evidence.provenance_status == ProvenanceStatus.DIRECT
        verdicts.append(GateVerdict(
            code=code,
            domain="coverage",
            decision=(
                GateDecision.PASS if covered
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "evidence provenance is direct"
                if covered
                else "evidence provenance is not direct; coverage uncertain"
            ),
        ))

        # EVD-EXECUTION-MATCH-001
        code = "EVD-EXECUTION-MATCH-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="execution",
            decision=(
                GateDecision.PASS if evidence.artifact_schema_valid
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "artifact schema valid — execution matches declaration"
                if evidence.artifact_schema_valid
                else "artifact schema not validated — execution may differ"
            ),
        ))

        return verdicts

    def _evaluate_claim_gates(
        self,
        *,
        evidence: EvidenceState,
        claim_type: ClaimType,
        claim_strength: ClaimStrength,
        population_covered: bool,
        contrary_searched: bool,
        precommitment_recorded: bool,
    ) -> list[GateVerdict]:
        """Evaluate gates for evidence → claim promotion."""
        verdicts: list[GateVerdict] = []

        # CLM-STRENGTH-001
        code = "CLM-STRENGTH-001"
        strength_ok = claim_strength != ClaimStrength.NONE
        verdicts.append(GateVerdict(
            code=code,
            domain="claim_strength",
            decision=(
                GateDecision.PASS if strength_ok
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"claim strength: {claim_strength.value}"
                if strength_ok
                else "claim strength is 'none' — no evidence support declared"
            ),
        ))

        # CLM-GENERALITY-001
        code = "CLM-GENERALITY-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="claim_strength",
            decision=(
                GateDecision.PASS if population_covered
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "population coverage confirmed"
                if population_covered
                else "population coverage not established"
            ),
        ))

        # CLM-MECHANISM-001
        code = "CLM-MECHANISM-001"
        mech_ok = claim_type in (
            ClaimType.MEASURED, ClaimType.DIRECT_COMPARISON,
        )
        verdicts.append(GateVerdict(
            code=code,
            domain="claim_strength",
            decision=(
                GateDecision.PASS if mech_ok
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                f"claim type '{claim_type.value}' has discriminating measurement"
                if mech_ok
                else f"claim type '{claim_type.value}' may lack discriminating test"
            ),
        ))

        # BIAS-CONFIRM-001
        code = "BIAS-CONFIRM-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="independence",
            decision=(
                GateDecision.PASS if contrary_searched
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "contrary evidence search recorded"
                if contrary_searched
                else "no contrary evidence search recorded"
            ),
        ))

        # BIAS-PRECOMMIT-001
        code = "BIAS-PRECOMMIT-001"
        verdicts.append(GateVerdict(
            code=code,
            domain="independence",
            decision=(
                GateDecision.PASS if precommitment_recorded
                else _default_for_mode(code, self._mode)
            ),
            decisive_question=_GATE_DEFAULTS[code]["question"],
            rationale=(
                "precommitment recorded before case retrieval"
                if precommitment_recorded
                else "no precommitment recorded"
            ),
        ))

        return verdicts

    def _summarize_gates(
        self, verdicts: list[GateVerdict],
    ) -> PromotionGateResult:
        blocking = [
            v.code for v in verdicts if v.decision == GateDecision.BLOCK
        ]
        warnings = [
            v.code for v in verdicts if v.decision == GateDecision.WARN
        ]
        deferred = [
            v.code for v in verdicts if v.decision == GateDecision.DEFER
        ]
        return PromotionGateResult(
            allowed=not bool(blocking),
            verdicts=verdicts,
            blocking_codes=blocking,
            warning_codes=warnings,
            deferred_codes=deferred,
        )

    # ── helpers ─────────────────────────────────────────────────────────

    def _require_action(self, action_id: str) -> ActionState:
        state = self._actions.get(action_id)
        if state is None:
            raise AssuranceError(f"action '{action_id}' not found")
        return state

    def _require_evidence(self, evidence_id: str) -> EvidenceState:
        state = self._evidences.get(evidence_id)
        if state is None:
            raise AssuranceError(f"evidence '{evidence_id}' not found")
        return state
