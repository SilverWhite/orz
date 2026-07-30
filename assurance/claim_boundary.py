"""Claim Boundary — claim type classification with evidence requirements.

Implements the requirement from :file:`architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4:

    Distinguish **measured**, **direct comparison**, **bridge hypothesis**,
    and **promotion eligibility** claim types.  Each type has distinct
    evidence requirements and promotion paths.

Builds on :class:`~.evidence_kernel.EvidenceKernel` and
:class:`~.evidence_kernel.ClaimType`.  The boundary classifier evaluates
whether a claim's declared type is supported by the available evidence
and determines what additional evidence would be required for promotion.

Design invariant (from :file:`architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2`
§4.5):  claim boundaries are determined by evidence, not by model
self-assessment.  The classifier *does not* consult model output to
determine claim strength — it only examines structured evidence state.
"""
from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any

from .errors import AssuranceError
from .evidence_kernel import (
    ClaimState,
    ClaimStrength,
    ClaimType,
    EvidenceState,
    ProvenanceStatus,
)


# ══════════════════════════════════════════════════════════════════════════════
# evidence requirement model
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceRequirement(Enum):
    """A named evidence requirement that a claim type demands."""

    PROVENANCE_DIRECT = "provenance_direct"
    ARTIFACT_SCHEMA_VALID = "artifact_schema_valid"
    ARTIFACT_FINITE = "artifact_finite"
    ARTIFACT_NOT_STALE = "artifact_not_stale"
    COMPARABILITY_ESTABLISHED = "comparability_established"
    NO_CONFOUNDS = "no_confounds"
    EXECUTION_MATCH = "execution_match"
    SEMANTIC_MAPPING = "semantic_mapping"
    DISCRIMINATING_TEST = "discriminating_test"
    POPULATION_COVERAGE = "population_coverage"
    INDEPENDENCE_MULTI = "independence_multi"
    CONTRARY_SEARCH = "contrary_search"
    PRECOMMITMENT = "precommitment"


@dataclass(frozen=True)
class ClaimTypeProfile:
    """Evidence requirements and promotion rules for a claim type."""

    claim_type: ClaimType
    label: str
    description: str
    required_evidence: frozenset[EvidenceRequirement]
    recommended_evidence: frozenset[EvidenceRequirement]
    minimum_strength: ClaimStrength
    eligible_for_promotion: bool
    promotion_target: ClaimType | None
    promotion_requires: frozenset[EvidenceRequirement]


# ══════════════════════════════════════════════════════════════════════════════
# type registry
# ══════════════════════════════════════════════════════════════════════════════

_TYPE_PROFILES: dict[ClaimType, ClaimTypeProfile] = {
    ClaimType.MEASURED: ClaimTypeProfile(
        claim_type=ClaimType.MEASURED,
        label="Measured",
        description=(
            "A single measurement or observation grounded in directly "
            "observed data.  The weakest claim type that still carries "
            "evidential weight."
        ),
        required_evidence=frozenset({
            EvidenceRequirement.PROVENANCE_DIRECT,
            EvidenceRequirement.ARTIFACT_SCHEMA_VALID,
            EvidenceRequirement.ARTIFACT_FINITE,
            EvidenceRequirement.ARTIFACT_NOT_STALE,
        }),
        recommended_evidence=frozenset({
            EvidenceRequirement.POPULATION_COVERAGE,
        }),
        minimum_strength=ClaimStrength.OBSERVED_FRAGMENT_ONLY,
        eligible_for_promotion=True,
        promotion_target=ClaimType.DIRECT_COMPARISON,
        promotion_requires=frozenset({
            EvidenceRequirement.COMPARABILITY_ESTABLISHED,
            EvidenceRequirement.EXECUTION_MATCH,
            EvidenceRequirement.INDEPENDENCE_MULTI,
        }),
    ),
    ClaimType.DIRECT_COMPARISON: ClaimTypeProfile(
        claim_type=ClaimType.DIRECT_COMPARISON,
        label="Direct Comparison",
        description=(
            "A comparison of two or more measured entities under controlled "
            "conditions.  Requires that both sides of the comparison share "
            "task, protocol, metric, and evaluation scope."
        ),
        required_evidence=frozenset({
            EvidenceRequirement.PROVENANCE_DIRECT,
            EvidenceRequirement.ARTIFACT_SCHEMA_VALID,
            EvidenceRequirement.COMPARABILITY_ESTABLISHED,
            EvidenceRequirement.EXECUTION_MATCH,
            EvidenceRequirement.NO_CONFOUNDS,
        }),
        recommended_evidence=frozenset({
            EvidenceRequirement.INDEPENDENCE_MULTI,
            EvidenceRequirement.POPULATION_COVERAGE,
            EvidenceRequirement.CONTRARY_SEARCH,
        }),
        minimum_strength=ClaimStrength.FULL_TEXT_GROUNDED,
        eligible_for_promotion=True,
        promotion_target=ClaimType.BRIDGE_HYPOTHESIS,
        promotion_requires=frozenset({
            EvidenceRequirement.SEMANTIC_MAPPING,
            EvidenceRequirement.DISCRIMINATING_TEST,
            EvidenceRequirement.CONTRARY_SEARCH,
        }),
    ),
    ClaimType.BRIDGE_HYPOTHESIS: ClaimTypeProfile(
        claim_type=ClaimType.BRIDGE_HYPOTHESIS,
        label="Bridge Hypothesis",
        description=(
            "A hypothesis that connects two bodies of evidence through "
            "inference.  Requires explicit semantic mapping and at least "
            "one discriminating measurement or intervention."
        ),
        required_evidence=frozenset({
            EvidenceRequirement.SEMANTIC_MAPPING,
            EvidenceRequirement.DISCRIMINATING_TEST,
            EvidenceRequirement.CONTRARY_SEARCH,
        }),
        recommended_evidence=frozenset({
            EvidenceRequirement.PRECOMMITMENT,
            EvidenceRequirement.INDEPENDENCE_MULTI,
        }),
        minimum_strength=ClaimStrength.FULL_TEXT_GROUNDED,
        eligible_for_promotion=False,  # Bridge hypotheses are terminal — they cannot auto-promote
        promotion_target=None,
        promotion_requires=frozenset(),
    ),
    ClaimType.OBSERVATION: ClaimTypeProfile(
        claim_type=ClaimType.OBSERVATION,
        label="Observation",
        description=(
            "A descriptive observation without causal or comparative "
            "interpretation.  The lowest claim tier — carries no "
            "promotion eligibility by itself."
        ),
        required_evidence=frozenset({
            EvidenceRequirement.PROVENANCE_DIRECT,
            EvidenceRequirement.ARTIFACT_NOT_STALE,
        }),
        recommended_evidence=frozenset({
            EvidenceRequirement.ARTIFACT_SCHEMA_VALID,
            EvidenceRequirement.ARTIFACT_FINITE,
        }),
        minimum_strength=ClaimStrength.METADATA_ONLY,
        eligible_for_promotion=True,
        promotion_target=ClaimType.MEASURED,
        promotion_requires=frozenset({
            EvidenceRequirement.ARTIFACT_SCHEMA_VALID,
            EvidenceRequirement.ARTIFACT_FINITE,
            EvidenceRequirement.POPULATION_COVERAGE,
        }),
    ),
}


def get_profile(claim_type: ClaimType) -> ClaimTypeProfile:
    """Return the :class:`ClaimTypeProfile` for *claim_type*."""
    return _TYPE_PROFILES[claim_type]


# ══════════════════════════════════════════════════════════════════════════════
# evidence assessment
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class EvidenceAssessment:
    """Structured assessment of which evidence requirements are met."""

    satisfied: frozenset[EvidenceRequirement]
    unsatisfied: frozenset[EvidenceRequirement]
    recommended_met: frozenset[EvidenceRequirement]
    recommended_missing: frozenset[EvidenceRequirement]

    @property
    def all_required_met(self) -> bool:
        return not self.unsatisfied

    @property
    def score(self) -> float:
        """Fraction of required + recommended evidence satisfied (0.0–1.0)."""
        total = len(self.satisfied) + len(self.unsatisfied) + len(self.recommended_met) + len(self.recommended_missing)
        if total == 0:
            return 1.0
        met = len(self.satisfied) + len(self.recommended_met)
        return met / total


def assess_evidence(
    evidence: EvidenceState,
    claim_type: ClaimType,
) -> EvidenceAssessment:
    """Evaluate which evidence requirements are met for *claim_type*.

    Examines the structured fields of *evidence* (not model output) to
    determine whether each required and recommended evidence element is
    satisfied.

    Returns an :class:`EvidenceAssessment` with satisfied/unsatisfied
    breakdowns.
    """
    profile = get_profile(claim_type)
    satisfied: set[EvidenceRequirement] = set()
    unsatisfied: set[EvidenceRequirement] = set()
    recommended_met: set[EvidenceRequirement] = set()
    recommended_missing: set[EvidenceRequirement] = set()

    def _check(req: EvidenceRequirement) -> bool:
        """Return True if *req* is satisfied by *evidence*."""
        if req == EvidenceRequirement.PROVENANCE_DIRECT:
            return evidence.provenance_status == ProvenanceStatus.DIRECT
        if req == EvidenceRequirement.ARTIFACT_SCHEMA_VALID:
            return evidence.artifact_schema_valid
        if req == EvidenceRequirement.ARTIFACT_FINITE:
            return evidence.artifact_finite_valid
        if req == EvidenceRequirement.ARTIFACT_NOT_STALE:
            return evidence.artifact_not_stale
        if req == EvidenceRequirement.INDEPENDENCE_MULTI:
            return evidence.independence_sources >= 2
        # The following requirements are not directly encoded in
        # EvidenceState fields — they require external validation.
        # They are reported as unsatisfied unless marked in metadata.
        if req == EvidenceRequirement.COMPARABILITY_ESTABLISHED:
            return evidence.metadata.get("comparability_established", False)
        if req == EvidenceRequirement.NO_CONFOUNDS:
            return evidence.metadata.get("no_confounds", False)
        if req == EvidenceRequirement.EXECUTION_MATCH:
            return evidence.metadata.get("execution_match", False)
        if req == EvidenceRequirement.SEMANTIC_MAPPING:
            return evidence.metadata.get("semantic_mapping_valid", False)
        if req == EvidenceRequirement.DISCRIMINATING_TEST:
            return evidence.metadata.get("discriminating_test_passed", False)
        if req == EvidenceRequirement.POPULATION_COVERAGE:
            return evidence.metadata.get("population_coverage", False)
        if req == EvidenceRequirement.CONTRARY_SEARCH:
            return evidence.metadata.get("contrary_search", False)
        if req == EvidenceRequirement.PRECOMMITMENT:
            return evidence.metadata.get("precommitment_recorded", False)
        return False

    for req in profile.required_evidence:
        if _check(req):
            satisfied.add(req)
        else:
            unsatisfied.add(req)

    for req in profile.recommended_evidence:
        if _check(req):
            recommended_met.add(req)
        else:
            recommended_missing.add(req)

    return EvidenceAssessment(
        satisfied=frozenset(satisfied),
        unsatisfied=frozenset(unsatisfied),
        recommended_met=frozenset(recommended_met),
        recommended_missing=frozenset(recommended_missing),
    )


# ══════════════════════════════════════════════════════════════════════════════
# Claim Boundary classifier
# ══════════════════════════════════════════════════════════════════════════════


@dataclass
class BoundaryVerdict:
    """Result of classifying a claim against its evidence."""

    claim_id: str
    declared_type: ClaimType
    profile: ClaimTypeProfile
    assessment: EvidenceAssessment
    type_appropriate: bool
    strength_appropriate: bool
    promotion_eligible: bool
    promotion_gap: frozenset[EvidenceRequirement]
    notes: list[str] = field(default_factory=list)


class ClaimBoundary:
    """Classify claims and validate evidence requirements.

    The boundary is a **mechanical classifier** — it examines structured
    evidence state, not model assertions.  It answers three questions:

    1. **Is the declared claim type appropriate** given the available evidence?
    2. **Is the claim strength supported** by the source visibility / provenance?
    3. **Is the claim eligible for promotion** to the next tier, and if not,
       what evidence is missing?

    Usage::

        boundary = ClaimBoundary()
        verdict = boundary.classify(
            claim=claim_state,
            evidence=evidence_state,
        )
        if not verdict.type_appropriate:
            print(f"Claim type mismatch: {verdict.assessment.unsatisfied}")
        if verdict.promotion_eligible:
            print(f"Eligible for promotion to {verdict.profile.promotion_target}")
    """

    # ── public API ────────────────────────────────────────────────────────

    def classify(
        self,
        claim: ClaimState,
        evidence: EvidenceState,
    ) -> BoundaryVerdict:
        """Classify *claim* against *evidence* and return a verdict.

        Parameters
        ----------
        claim:
            The claim state to evaluate.
        evidence:
            The evidence supporting the claim.

        Returns
        -------
        BoundaryVerdict
            Structured verdict with type appropriateness, strength check,
            and promotion eligibility.
        """
        declared = claim.claim_type
        profile = get_profile(declared)
        assessment = assess_evidence(evidence, declared)

        # 1. Type appropriateness: all required evidence must be met.
        type_appropriate = assessment.all_required_met

        # 2. Strength appropriateness: claim strength must not exceed
        #    the minimum supported by evidence.
        strength_appropriate = self._check_strength(claim, evidence)

        # 3. Promotion eligibility: if the profile allows promotion,
        #    check what's missing for the promotion target.
        promotion_eligible = False
        promotion_gap: frozenset[EvidenceRequirement] = frozenset()
        if profile.eligible_for_promotion and profile.promotion_target is not None:
            promotion_eligible, promotion_gap = self._check_promotion(
                evidence, profile,
            )

        notes: list[str] = []
        if not type_appropriate:
            missing = sorted(r.value for r in assessment.unsatisfied)
            notes.append(
                f"Required evidence missing: {', '.join(missing)}"
            )
        if not strength_appropriate:
            notes.append(
                f"Claim strength '{claim.claim_strength.value}' exceeds "
                f"minimum '{profile.minimum_strength.value}' for type "
                f"'{profile.label}'"
            )
        if profile.eligible_for_promotion and not promotion_eligible:
            gap = sorted(r.value for r in promotion_gap)
            notes.append(
                f"Promotion to {profile.promotion_target.value} requires: "
                f"{', '.join(gap)}"
            )

        return BoundaryVerdict(
            claim_id=claim.claim_id,
            declared_type=declared,
            profile=profile,
            assessment=assessment,
            type_appropriate=type_appropriate,
            strength_appropriate=strength_appropriate,
            promotion_eligible=promotion_eligible,
            promotion_gap=promotion_gap,
            notes=notes,
        )

    def classify_evidence_only(
        self,
        evidence: EvidenceState,
        *,
        declared_type: ClaimType | None = None,
    ) -> ClaimType:
        """Determine the strongest claim type supported by *evidence* alone.

        This is a blind assessment — it does not look at any claim
        declaration.  Useful for precommitment checks (what claim
        *could* I make before knowing what I want to claim?).

        Returns the strongest :class:`ClaimType` whose required evidence
        is fully satisfied.
        """
        # Check types in order of increasing strength.
        candidates = [
            ClaimType.OBSERVATION,
            ClaimType.MEASURED,
            ClaimType.DIRECT_COMPARISON,
            ClaimType.BRIDGE_HYPOTHESIS,
        ]
        best = ClaimType.OBSERVATION
        for ct in candidates:
            assessment = assess_evidence(evidence, ct)
            if assessment.all_required_met:
                best = ct
            else:
                break  # stronger types need everything the weaker ones need
        return best

    # ── internal ──────────────────────────────────────────────────────────

    @staticmethod
    def _check_strength(
        claim: ClaimState,
        evidence: EvidenceState,
    ) -> bool:
        """Check that claim strength does not exceed evidence support."""
        strength_order = {
            ClaimStrength.NONE: 0,
            ClaimStrength.METADATA_ONLY: 1,
            ClaimStrength.OBSERVED_FRAGMENT_ONLY: 2,
            ClaimStrength.FULL_TEXT_GROUNDED: 3,
        }
        claim_level = strength_order.get(claim.claim_strength, 0)

        # Evidence-driven maximum strength:
        # - DIRECT provenance → at least FULL_TEXT_GROUNDED
        # - Schema + finite valid → at least OBSERVED_FRAGMENT_ONLY
        # - Otherwise → METADATA_ONLY
        if evidence.provenance_status == ProvenanceStatus.DIRECT:
            max_level = 3  # FULL_TEXT_GROUNDED
        elif evidence.artifact_schema_valid and evidence.artifact_finite_valid:
            max_level = 2  # OBSERVED_FRAGMENT_ONLY
        elif evidence.artifact_not_stale:
            max_level = 1  # METADATA_ONLY
        else:
            max_level = 0  # NONE

        return claim_level <= max_level

    @staticmethod
    def _check_promotion(
        evidence: EvidenceState,
        profile: ClaimTypeProfile,
    ) -> tuple[bool, frozenset[EvidenceRequirement]]:
        """Check whether *evidence* satisfies the promotion requirements."""
        if profile.promotion_target is None:
            return False, frozenset()

        # We need to assess evidence against the promotion requirements.
        # Build a temporary assessment using the promotion_requires set.
        satisfied: set[EvidenceRequirement] = set()
        missing: set[EvidenceRequirement] = set()

        for req in profile.promotion_requires:
            # Reuse the same _check logic from assess_evidence
            met = _check_requirement(evidence, req)
            if met:
                satisfied.add(req)
            else:
                missing.add(req)

        return not bool(missing), frozenset(missing)


def _check_requirement(
    evidence: EvidenceState,
    req: EvidenceRequirement,
) -> bool:
    """Check a single evidence requirement against *evidence* state."""
    if req == EvidenceRequirement.PROVENANCE_DIRECT:
        return evidence.provenance_status == ProvenanceStatus.DIRECT
    if req == EvidenceRequirement.ARTIFACT_SCHEMA_VALID:
        return evidence.artifact_schema_valid
    if req == EvidenceRequirement.ARTIFACT_FINITE:
        return evidence.artifact_finite_valid
    if req == EvidenceRequirement.ARTIFACT_NOT_STALE:
        return evidence.artifact_not_stale
    if req == EvidenceRequirement.INDEPENDENCE_MULTI:
        return evidence.independence_sources >= 2
    if req == EvidenceRequirement.COMPARABILITY_ESTABLISHED:
        return evidence.metadata.get("comparability_established", False)
    if req == EvidenceRequirement.NO_CONFOUNDS:
        return evidence.metadata.get("no_confounds", False)
    if req == EvidenceRequirement.EXECUTION_MATCH:
        return evidence.metadata.get("execution_match", False)
    if req == EvidenceRequirement.SEMANTIC_MAPPING:
        return evidence.metadata.get("semantic_mapping_valid", False)
    if req == EvidenceRequirement.DISCRIMINATING_TEST:
        return evidence.metadata.get("discriminating_test_passed", False)
    if req == EvidenceRequirement.POPULATION_COVERAGE:
        return evidence.metadata.get("population_coverage", False)
    if req == EvidenceRequirement.CONTRARY_SEARCH:
        return evidence.metadata.get("contrary_search", False)
    if req == EvidenceRequirement.PRECOMMITMENT:
        return evidence.metadata.get("precommitment_recorded", False)
    return False
