"""Tests for EvidenceKernel — action / evidence / claim three-layer state machine."""

from __future__ import annotations

import unittest

from assurance.evidence_kernel import (
    ActionState,
    ActionStatus,
    ClaimState,
    ClaimStrength,
    ClaimType,
    EvidenceKernel,
    EvidenceState,
    GateDecision,
    GateMode,
    ProvenanceStatus,
    PromotionGateResult,
)
from assurance.errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# 1. Action lifecycle
# ══════════════════════════════════════════════════════════════════════════════


class ActionLifecycleTests(unittest.TestCase):
    """Tests for action registration, start, complete, fail."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.GUARDED)

    def test_register_action_creates_registered_state(self) -> None:
        action = self.kernel.register_action(
            action_id="ACT-001", action_type="evaluate_gate",
            idempotency_key="task-x::step-1",
        )
        self.assertEqual(action.action_id, "ACT-001")
        self.assertEqual(action.status, ActionStatus.REGISTERED)
        self.assertEqual(action.idempotency_key, "task-x::step-1")

    def test_duplicate_action_id_raises(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        with self.assertRaises(AssuranceError):
            self.kernel.register_action(action_id="ACT-001", action_type="test")

    def test_start_action_transitions_to_running(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        action = self.kernel.start_action("ACT-001")
        self.assertEqual(action.status, ActionStatus.RUNNING)

    def test_start_already_running_raises(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        self.kernel.start_action("ACT-001")
        with self.assertRaises(AssuranceError):
            self.kernel.start_action("ACT-001")

    def test_complete_action_sets_terminal_outcome(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        self.kernel.start_action("ACT-001")
        action = self.kernel.complete_action(
            "ACT-001", terminal_outcome="succeeded",
            artifact_hashes=["sha256:abc123"],
        )
        self.assertEqual(action.status, ActionStatus.COMPLETED)
        self.assertEqual(action.terminal_outcome, "succeeded")
        self.assertIn("sha256:abc123", action.artifact_hashes)

    def test_complete_from_registered_is_allowed(self) -> None:
        """Complete directly from REGISTERED (skip RUNNING) is allowed."""
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        action = self.kernel.complete_action(
            "ACT-001", terminal_outcome="succeeded",
        )
        self.assertEqual(action.status, ActionStatus.COMPLETED)

    def test_fail_action_sets_failed_status(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        action = self.kernel.fail_action("ACT-001", reason="timeout")
        self.assertEqual(action.status, ActionStatus.FAILED)
        self.assertEqual(action.metadata["failure_reason"], "timeout")

    def test_get_action_returns_none_for_missing(self) -> None:
        self.assertIsNone(self.kernel.get_action("nonexistent"))


# ══════════════════════════════════════════════════════════════════════════════
# 2. Action → evidence promotion
# ══════════════════════════════════════════════════════════════════════════════


class ActionToEvidencePromotionTests(unittest.TestCase):
    """Tests for promote_to_evidence gate evaluation."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.GUARDED)

    def _register_and_complete(
        self, action_id: str = "ACT-001", *, idempotency_key: str | None = "key-1",
    ) -> ActionState:
        self.kernel.register_action(
            action_id=action_id, action_type="test",
            idempotency_key=idempotency_key,
        )
        return self.kernel.complete_action(
            action_id, terminal_outcome="succeeded",
        )

    def test_full_promotion_passes_all_gates(self) -> None:
        self._register_and_complete("ACT-001")
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
            independence_sources=2,
        )
        self.assertTrue(result.allowed)
        self.assertEqual(evidence.evidence_id, "EVD-001")
        self.assertIn("ACT-001", evidence.source_action_ids)
        self.assertEqual(evidence.provenance_status, ProvenanceStatus.DIRECT)
        self.assertEqual(len(result.blocking_codes), 0)

    def test_promotion_without_idempotency_key_blocks_in_guarded(self) -> None:
        self._register_and_complete("ACT-001", idempotency_key=None)
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        self.assertFalse(result.allowed)
        self.assertIn("ACT-IDEMPOTENCY-001", result.blocking_codes)

    def test_promotion_with_unknown_provenance_defers_in_guarded(self) -> None:
        self._register_and_complete("ACT-001")
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.UNKNOWN,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        # GUARDED mode: EVD-PROVENANCE-001 defaults to DEFER, not BLOCK.
        # The promotion is still allowed but carries a deferred code.
        self.assertTrue(result.allowed)
        self.assertIn("EVD-PROVENANCE-001", result.deferred_codes)

    def test_promotion_with_invalid_schema_blocks(self) -> None:
        self._register_and_complete("ACT-001")
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=False,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        self.assertFalse(result.allowed)
        self.assertIn("ART-SCHEMA-001", result.blocking_codes)

    def test_cannot_promote_uncompleted_action(self) -> None:
        self.kernel.register_action(action_id="ACT-001", action_type="test")
        with self.assertRaises(AssuranceError):
            self.kernel.promote_to_evidence(
                "ACT-001",
                evidence_id="EVD-001",
                provenance_status=ProvenanceStatus.DIRECT,
            )

    def test_duplicate_evidence_id_raises(self) -> None:
        self._register_and_complete("ACT-001")
        self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        # Second action to promote
        self.kernel.register_action(
            action_id="ACT-002", action_type="test", idempotency_key="k2",
        )
        self.kernel.complete_action("ACT-002", terminal_outcome="succeeded")
        with self.assertRaises(AssuranceError):
            self.kernel.promote_to_evidence(
                "ACT-002", evidence_id="EVD-001",
                provenance_status=ProvenanceStatus.DIRECT,
            )

    def test_all_gate_verdicts_recorded(self) -> None:
        self._register_and_complete("ACT-001")
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
            independence_sources=3,
        )
        # Action gates + evidence gates
        expected_codes = {
            "ACT-IDEMPOTENCY-001",
            "PROC-TERMINAL-001",
            "EVD-PROVENANCE-001",
            "ART-SCHEMA-001",
            "ART-FINITE-001",
            "ART-STALE-001",
            "EVD-INDEPENDENCE-001",
        }
        actual_codes = {v.code for v in result.verdicts}
        self.assertEqual(actual_codes, expected_codes)

    def test_discussion_mode_passes_idempotency_missing(self) -> None:
        kernel = EvidenceKernel(mode=GateMode.DISCUSSION)
        kernel.register_action(
            action_id="ACT-001", action_type="test",
            idempotency_key=None,  # missing
        )
        kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        evidence, result = kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        # DISCUSSION mode: ACT-IDEMPOTENCY-001 defaults to PASS
        self.assertTrue(result.allowed)


# ══════════════════════════════════════════════════════════════════════════════
# 3. Evidence → claim promotion
# ══════════════════════════════════════════════════════════════════════════════


class EvidenceToClaimPromotionTests(unittest.TestCase):
    """Tests for promote_to_claim gate evaluation."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.GUARDED)
        self._make_evidence("EVD-001")

    def _make_evidence(self, evidence_id: str) -> EvidenceState:
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        evidence, _ = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id=evidence_id,
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
            independence_sources=2,
        )
        return evidence

    def test_full_claim_promotion_passes_all_gates(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        self.assertTrue(result.allowed)
        self.assertEqual(claim.claim_id, "CLM-001")
        self.assertEqual(claim.claim_type, ClaimType.MEASURED)
        self.assertEqual(len(result.blocking_codes), 0)

    def test_none_strength_defers_in_guarded(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.OBSERVATION,
            claim_strength=ClaimStrength.NONE,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        # GUARDED mode: CLM-STRENGTH-001 defaults to DEFER.
        self.assertTrue(result.allowed)
        self.assertIn("CLM-STRENGTH-001", result.deferred_codes)

    def test_missing_population_coverage_defers(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=False,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        # GUARDED mode: CLM-GENERALITY-001 defaults to DEFER.
        self.assertTrue(result.allowed)
        self.assertIn("CLM-GENERALITY-001", result.deferred_codes)

    def test_missing_contrary_search_defers(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=False,
            precommitment_recorded=True,
        )
        # GUARDED mode: BIAS-CONFIRM-001 defaults to DEFER.
        self.assertTrue(result.allowed)
        self.assertIn("BIAS-CONFIRM-001", result.deferred_codes)

    def test_bridge_hypothesis_defers_mechanism(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.BRIDGE_HYPOTHESIS,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        # GUARDED mode: CLM-MECHANISM-001 defaults to DEFER for bridge_hypothesis.
        self.assertTrue(result.allowed)
        self.assertIn("CLM-MECHANISM-001", result.deferred_codes)

    def test_measured_and_direct_comparison_pass_mechanism(self) -> None:
        for ct in (ClaimType.MEASURED, ClaimType.DIRECT_COMPARISON):
            claim, result = self.kernel.promote_to_claim(
                "EVD-001",
                claim_id=f"CLM-{ct.value}",
                claim_type=ct,
                claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
                population_covered=True,
                contrary_searched=True,
                precommitment_recorded=True,
            )
            self.assertTrue(
                result.allowed,
                f"ClaimType.{ct.name} should pass CLM-MECHANISM-001",
            )

    def test_all_claim_gate_verdicts_recorded(self) -> None:
        claim, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        # Evidence-for-claim gates + claim gates
        expected_codes = {
            "EVD-COVERAGE-001",
            "EVD-EXECUTION-MATCH-001",
            "CLM-STRENGTH-001",
            "CLM-GENERALITY-001",
            "CLM-MECHANISM-001",
            "BIAS-CONFIRM-001",
            "BIAS-PRECOMMIT-001",
        }
        actual_codes = {v.code for v in result.verdicts}
        self.assertEqual(actual_codes, expected_codes)

    def test_duplicate_claim_id_raises(self) -> None:
        self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True, contrary_searched=True,
            precommitment_recorded=True,
        )
        with self.assertRaises(AssuranceError):
            self.kernel.promote_to_claim(
                "EVD-001", claim_id="CLM-001",
                claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            )


# ══════════════════════════════════════════════════════════════════════════════
# 4. Hard invariants
# ══════════════════════════════════════════════════════════════════════════════


class HardInvariantTests(unittest.TestCase):
    """Tests for the no-auto-upgrade invariants."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.GUARDED)

    def test_action_not_auto_promoted_to_evidence(self) -> None:
        """Completing an action does NOT create evidence — explicit call required."""
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        # Evidence does not exist until promote_to_evidence is called
        self.assertEqual(self.kernel.evidence_count(), 0)
        self.assertIsNone(self.kernel.get_evidence("EVD-001"))

    def test_evidence_not_auto_promoted_to_claim(self) -> None:
        """Promoting to evidence does NOT create a claim — explicit call required."""
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        # Claim does not exist until promote_to_claim is called
        self.assertEqual(self.kernel.claim_count(), 0)
        self.assertIsNone(self.kernel.get_claim("CLM-001"))

    def test_failed_action_cannot_be_promoted(self) -> None:
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.fail_action("ACT-001", reason="crash")
        with self.assertRaises(AssuranceError):
            self.kernel.promote_to_evidence(
                "ACT-001",
                evidence_id="EVD-001",
                provenance_status=ProvenanceStatus.DIRECT,
            )

    def test_blocked_evidence_still_registered_with_reason_codes(self) -> None:
        """Even when gates block, the evidence state is recorded for audit."""
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key=None,
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        evidence, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.UNKNOWN,
            artifact_schema_valid=False,
            artifact_finite_valid=False,
            artifact_not_stale=False,
        )
        self.assertFalse(result.allowed)
        self.assertEqual(self.kernel.evidence_count(), 1)
        retrieved = self.kernel.get_evidence("EVD-001")
        self.assertIsNotNone(retrieved)
        self.assertGreater(len(retrieved.reason_codes), 0)


# ══════════════════════════════════════════════════════════════════════════════
# 5. Queries and counts
# ══════════════════════════════════════════════════════════════════════════════


class QueryTests(unittest.TestCase):
    """Tests for get_*, count_*, and property accessors."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.GUARDED)

    def test_counts_track_all_layers(self) -> None:
        self.assertEqual(self.kernel.action_count(), 0)
        self.assertEqual(self.kernel.evidence_count(), 0)
        self.assertEqual(self.kernel.claim_count(), 0)

        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.assertEqual(self.kernel.action_count(), 1)

        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        self.assertEqual(self.kernel.evidence_count(), 1)

        self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=True,
        )
        self.assertEqual(self.kernel.claim_count(), 1)

    def test_properties_return_copies(self) -> None:
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        actions = self.kernel.actions
        actions.pop("ACT-001")
        # Original is unaffected
        self.assertEqual(self.kernel.action_count(), 1)


# ══════════════════════════════════════════════════════════════════════════════
# 6. Strict mode
# ══════════════════════════════════════════════════════════════════════════════


class StrictModeTests(unittest.TestCase):
    """Tests for strict mode behavior."""

    def setUp(self) -> None:
        self.kernel = EvidenceKernel(mode=GateMode.STRICT)

    def test_strict_mode_blocks_unknown_provenance(self) -> None:
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        _, result = self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.UNKNOWN,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        self.assertFalse(result.allowed)
        # In STRICT mode, EVD-PROVENANCE-001 is BLOCK (not DEFER)
        self.assertIn("EVD-PROVENANCE-001", result.blocking_codes)

    def test_strict_mode_blocks_missing_precommitment(self) -> None:
        self.kernel.register_action(
            action_id="ACT-001", action_type="test", idempotency_key="k1",
        )
        self.kernel.complete_action("ACT-001", terminal_outcome="succeeded")
        self.kernel.promote_to_evidence(
            "ACT-001",
            evidence_id="EVD-001",
            provenance_status=ProvenanceStatus.DIRECT,
            artifact_schema_valid=True,
            artifact_finite_valid=True,
            artifact_not_stale=True,
        )
        _, result = self.kernel.promote_to_claim(
            "EVD-001",
            claim_id="CLM-001",
            claim_type=ClaimType.MEASURED,
            claim_strength=ClaimStrength.FULL_TEXT_GROUNDED,
            population_covered=True,
            contrary_searched=True,
            precommitment_recorded=False,
        )
        self.assertFalse(result.allowed)
        self.assertIn("BIAS-PRECOMMIT-001", result.blocking_codes)


if __name__ == "__main__":
    unittest.main()
