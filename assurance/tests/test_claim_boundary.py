"""Tests for ClaimBoundary — claim type classification with evidence requirements."""

from __future__ import annotations

import unittest

from assurance.claim_boundary import (
    BoundaryVerdict,
    ClaimBoundary,
    ClaimTypeProfile,
    EvidenceAssessment,
    EvidenceRequirement,
    assess_evidence,
    get_profile,
)
from assurance.evidence_kernel import (
    ClaimState,
    ClaimStrength,
    ClaimType,
    EvidenceState,
    ProvenanceStatus,
)


# ── helpers ──────────────────────────────────────────────────────────────────


def _evidence(
    *,
    evidence_id: str = "EVD-001",
    provenance: ProvenanceStatus = ProvenanceStatus.DIRECT,
    schema_valid: bool = True,
    finite_valid: bool = True,
    not_stale: bool = True,
    independence: int = 1,
    metadata: dict | None = None,
) -> EvidenceState:
    return EvidenceState(
        evidence_id=evidence_id,
        provenance_status=provenance,
        artifact_schema_valid=schema_valid,
        artifact_finite_valid=finite_valid,
        artifact_not_stale=not_stale,
        independence_sources=independence,
        metadata=metadata or {},
    )


def _claim(
    *,
    claim_id: str = "CLM-001",
    claim_type: ClaimType = ClaimType.MEASURED,
    claim_strength: ClaimStrength = ClaimStrength.FULL_TEXT_GROUNDED,
    population_covered: bool = True,
    contrary_searched: bool = True,
    precommitment_recorded: bool = True,
) -> ClaimState:
    return ClaimState(
        claim_id=claim_id,
        claim_type=claim_type,
        claim_strength=claim_strength,
        population_covered=population_covered,
        contrary_searched=contrary_searched,
        precommitment_recorded=precommitment_recorded,
    )


# ══════════════════════════════════════════════════════════════════════════════
# 1. Type profiles
# ══════════════════════════════════════════════════════════════════════════════


class TypeProfileTests(unittest.TestCase):
    """Tests for ClaimTypeProfile registry."""

    def test_measured_profile_requires_direct_evidence(self) -> None:
        profile = get_profile(ClaimType.MEASURED)
        self.assertIn(EvidenceRequirement.PROVENANCE_DIRECT, profile.required_evidence)
        self.assertIn(EvidenceRequirement.ARTIFACT_SCHEMA_VALID, profile.required_evidence)
        self.assertTrue(profile.eligible_for_promotion)
        self.assertEqual(profile.promotion_target, ClaimType.DIRECT_COMPARISON)

    def test_direct_comparison_requires_comparability(self) -> None:
        profile = get_profile(ClaimType.DIRECT_COMPARISON)
        self.assertIn(EvidenceRequirement.COMPARABILITY_ESTABLISHED, profile.required_evidence)
        self.assertIn(EvidenceRequirement.NO_CONFOUNDS, profile.required_evidence)

    def test_bridge_hypothesis_not_eligible_for_promotion(self) -> None:
        profile = get_profile(ClaimType.BRIDGE_HYPOTHESIS)
        self.assertFalse(profile.eligible_for_promotion)
        self.assertIsNone(profile.promotion_target)

    def test_observation_has_minimal_requirements(self) -> None:
        profile = get_profile(ClaimType.OBSERVATION)
        self.assertIn(EvidenceRequirement.PROVENANCE_DIRECT, profile.required_evidence)
        self.assertIn(EvidenceRequirement.ARTIFACT_NOT_STALE, profile.required_evidence)
        self.assertEqual(profile.minimum_strength, ClaimStrength.METADATA_ONLY)
        self.assertTrue(profile.eligible_for_promotion)
        self.assertEqual(profile.promotion_target, ClaimType.MEASURED)

    def test_all_four_types_registered(self) -> None:
        for ct in ClaimType:
            profile = get_profile(ct)
            self.assertEqual(profile.claim_type, ct)
            self.assertIsInstance(profile.label, str)
            self.assertIsInstance(profile.description, str)


# ══════════════════════════════════════════════════════════════════════════════
# 2. Evidence assessment
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceAssessmentTests(unittest.TestCase):
    """Tests for assess_evidence function."""

    def test_measured_with_full_evidence_satisfies_all(self) -> None:
        ev = _evidence()
        assessment = assess_evidence(ev, ClaimType.MEASURED)
        self.assertTrue(assessment.all_required_met)
        self.assertEqual(len(assessment.unsatisfied), 0)

    def test_measured_with_unknown_provenance_fails(self) -> None:
        ev = _evidence(provenance=ProvenanceStatus.UNKNOWN)
        assessment = assess_evidence(ev, ClaimType.MEASURED)
        self.assertFalse(assessment.all_required_met)
        self.assertIn(EvidenceRequirement.PROVENANCE_DIRECT, assessment.unsatisfied)

    def test_measured_with_invalid_schema_fails(self) -> None:
        ev = _evidence(schema_valid=False)
        assessment = assess_evidence(ev, ClaimType.MEASURED)
        self.assertFalse(assessment.all_required_met)
        self.assertIn(EvidenceRequirement.ARTIFACT_SCHEMA_VALID, assessment.unsatisfied)

    def test_direct_comparison_requires_metadata_fields(self) -> None:
        ev = _evidence(metadata={
            "comparability_established": True,
            "no_confounds": True,
            "execution_match": True,
        })
        assessment = assess_evidence(ev, ClaimType.DIRECT_COMPARISON)
        self.assertTrue(assessment.all_required_met)

    def test_direct_comparison_without_metadata_fails(self) -> None:
        ev = _evidence()
        assessment = assess_evidence(ev, ClaimType.DIRECT_COMPARISON)
        self.assertFalse(assessment.all_required_met)
        self.assertIn(EvidenceRequirement.COMPARABILITY_ESTABLISHED, assessment.unsatisfied)

    def test_score_ranges_zero_to_one(self) -> None:
        ev = _evidence(schema_valid=False, finite_valid=False)
        assessment = assess_evidence(ev, ClaimType.MEASURED)
        self.assertGreaterEqual(assessment.score, 0.0)
        self.assertLessEqual(assessment.score, 1.0)

    def test_recommended_not_blocking(self) -> None:
        """Missing recommended evidence does NOT block — only required does."""
        ev = _evidence(provenance=ProvenanceStatus.DIRECT)
        assessment = assess_evidence(ev, ClaimType.MEASURED)
        # Even without population_coverage (recommended), all required is met.
        self.assertTrue(assessment.all_required_met)
        self.assertIn(
            EvidenceRequirement.POPULATION_COVERAGE,
            assessment.recommended_missing,
        )

    def test_direct_comparison_recommends_independence(self) -> None:
        ev = _evidence(
            independence=1,
            metadata={
                "comparability_established": True,
                "no_confounds": True,
                "execution_match": True,
            },
        )
        assessment = assess_evidence(ev, ClaimType.DIRECT_COMPARISON)
        self.assertTrue(assessment.all_required_met)
        # independence_multi is recommended (needs >= 2 sources)
        self.assertIn(
            EvidenceRequirement.INDEPENDENCE_MULTI,
            assessment.recommended_missing,
        )


# ══════════════════════════════════════════════════════════════════════════════
# 3. ClaimBoundary classifier
# ══════════════════════════════════════════════════════════════════════════════


class ClaimBoundaryClassifierTests(unittest.TestCase):
    """Tests for ClaimBoundary.classify()."""

    def setUp(self) -> None:
        self.boundary = ClaimBoundary()

    def test_measured_with_full_evidence_is_type_appropriate(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=_evidence(),
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertTrue(verdict.strength_appropriate)
        self.assertEqual(verdict.declared_type, ClaimType.MEASURED)

    def test_measured_with_weak_evidence_is_not_type_appropriate(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=_evidence(provenance=ProvenanceStatus.UNKNOWN, schema_valid=False),
        )
        self.assertFalse(verdict.type_appropriate)
        self.assertGreater(len(verdict.notes), 0)

    def test_direct_comparison_with_full_metadata_is_type_appropriate(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.DIRECT_COMPARISON),
            evidence=_evidence(metadata={
                "comparability_established": True,
                "no_confounds": True,
                "execution_match": True,
            }),
        )
        self.assertTrue(verdict.type_appropriate)

    def test_strength_exceeds_evidence_is_detected(self) -> None:
        """Claiming FULL_TEXT_GROUNDED with DERIVED_UNVERIFIED evidence."""
        verdict = self.boundary.classify(
            claim=_claim(
                claim_type=ClaimType.MEASURED,
                claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            ),
            evidence=_evidence(
                provenance=ProvenanceStatus.DERIVED_UNVERIFIED,
                schema_valid=False,
                finite_valid=False,
            ),
        )
        self.assertFalse(verdict.strength_appropriate)

    def test_measured_eligible_for_promotion_to_direct_comparison(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=_evidence(
                independence=1,
                metadata={
                    "comparability_established": False,
                    "execution_match": False,
                },
            ),
        )
        self.assertTrue(verdict.type_appropriate)
        # Promotion requires comparability + execution_match + independence_multi
        self.assertFalse(verdict.promotion_eligible)
        self.assertIn(
            EvidenceRequirement.COMPARABILITY_ESTABLISHED,
            verdict.promotion_gap,
        )

    def test_measured_with_promotion_requirements_met(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=_evidence(
                independence=2,
                metadata={
                    "comparability_established": True,
                    "execution_match": True,
                },
            ),
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertTrue(verdict.promotion_eligible)
        self.assertEqual(len(verdict.promotion_gap), 0)

    def test_bridge_hypothesis_never_eligible_for_promotion(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.BRIDGE_HYPOTHESIS),
            evidence=_evidence(metadata={
                "semantic_mapping_valid": True,
                "discriminating_test_passed": True,
                "contrary_search": True,
            }),
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertFalse(verdict.promotion_eligible)

    def test_observation_promotion_to_measured(self) -> None:
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.OBSERVATION),
            evidence=_evidence(
                schema_valid=True,
                finite_valid=True,
                metadata={"population_coverage": True},
            ),
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertTrue(verdict.promotion_eligible)
        self.assertEqual(verdict.profile.promotion_target, ClaimType.MEASURED)


# ══════════════════════════════════════════════════════════════════════════════
# 4. Evidence-only classification (blind / precommitment)
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceOnlyClassificationTests(unittest.TestCase):
    """Tests for classify_evidence_only."""

    def setUp(self) -> None:
        self.boundary = ClaimBoundary()

    def test_full_evidence_supports_direct_comparison(self) -> None:
        best = self.boundary.classify_evidence_only(_evidence(metadata={
            "comparability_established": True,
            "no_confounds": True,
            "execution_match": True,
        }))
        self.assertEqual(best, ClaimType.DIRECT_COMPARISON)

    def test_minimal_evidence_only_supports_observation(self) -> None:
        best = self.boundary.classify_evidence_only(_evidence(
            provenance=ProvenanceStatus.DIRECT,
            schema_valid=False,
            finite_valid=False,
        ))
        self.assertEqual(best, ClaimType.OBSERVATION)

    def test_no_provenance_only_supports_observation(self) -> None:
        """OBSERVATION requires provenance + not_stale. If not_stale is True
        but provenance is UNKNOWN, OBSERVATION fails; fallback is still
        OBSERVATION (it's the floor)."""
        best = self.boundary.classify_evidence_only(_evidence(
            provenance=ProvenanceStatus.UNKNOWN,
            schema_valid=False,
            finite_valid=False,
        ))
        # When nothing passes, classify_evidence_only returns OBSERVATION
        # (the floor — but the caller must check whether evidence actually
        # supports it via assess_evidence).
        self.assertEqual(best, ClaimType.OBSERVATION)

    def test_direct_evidence_with_all_metadata_supports_bridge(self) -> None:
        best = self.boundary.classify_evidence_only(_evidence(metadata={
            "comparability_established": True,
            "no_confounds": True,
            "execution_match": True,
            "semantic_mapping_valid": True,
            "discriminating_test_passed": True,
            "contrary_search": True,
        }))
        self.assertEqual(best, ClaimType.BRIDGE_HYPOTHESIS)


# ══════════════════════════════════════════════════════════════════════════════
# 5. Integration with claim evidence metadata
# ══════════════════════════════════════════════════════════════════════════════


class IntegrationTests(unittest.TestCase):
    """Tests for ClaimBoundary + EvidenceState metadata-driven gates."""

    def setUp(self) -> None:
        self.boundary = ClaimBoundary()

    def test_full_pipeline_measured_to_bridge(self) -> None:
        """Simulate evidence accumulating across promotion chain."""
        # Start with measured-level evidence.
        ev = _evidence()
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=ev,
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertFalse(verdict.promotion_eligible)

        # Add comparability + execution_match + independence → promotion eligible.
        ev = _evidence(
            independence=2,
            metadata={
                "comparability_established": True,
                "execution_match": True,
            },
        )
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.MEASURED),
            evidence=ev,
        )
        self.assertTrue(verdict.promotion_eligible)
        self.assertEqual(verdict.profile.promotion_target, ClaimType.DIRECT_COMPARISON)

        # Now as DIRECT_COMPARISON with semantic mapping + discriminating test
        # + contrary search → eligible for BRIDGE_HYPOTHESIS.
        ev = _evidence(
            independence=2,
            metadata={
                "comparability_established": True,
                "no_confounds": True,
                "execution_match": True,
                "semantic_mapping_valid": True,
                "discriminating_test_passed": True,
                "contrary_search": True,
            },
        )
        verdict = self.boundary.classify(
            claim=_claim(claim_type=ClaimType.DIRECT_COMPARISON),
            evidence=ev,
        )
        self.assertTrue(verdict.type_appropriate)
        self.assertTrue(verdict.promotion_eligible)
        self.assertEqual(verdict.profile.promotion_target, ClaimType.BRIDGE_HYPOTHESIS)


if __name__ == "__main__":
    unittest.main()
