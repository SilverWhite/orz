"""Tests for diagnostic coverage check — progressive neutral pullback during debug stagnation."""

from __future__ import annotations

import unittest

from assurance.diagnostic_coverage import (
    ALLOWED_DIAGNOSTIC_RESPONSE_FIELDS,
    FORBIDDEN_DIAGNOSTIC_RESPONSE_FIELDS,
    HARD_SIGNAL_TYPES,
    INITIAL_THRESHOLD,
    MAX_THRESHOLD,
    DiagnosticCoverageState,
    build_diagnostic_coverage_check,
    build_diagnostic_coverage_check_from_state,
    evaluate_diagnostic_coverage_response,
)
from assurance.errors import AssuranceError


BUG_ID = "BUG-NULL-POINTER-001"


# ═══════════════════════════════════════════════════════════════════
# DiagnosticCoverageState — state machine tests
# ═══════════════════════════════════════════════════════════════════


class DiagnosticCoverageStateInitTests(unittest.TestCase):
    """Tests for DiagnosticCoverageState initialisation and properties."""

    def test_initial_threshold_is_2(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        self.assertEqual(state.current_threshold, 2)
        self.assertEqual(state.hard_signal_count, 0.0)
        self.assertEqual(state.trigger_count, 0)
        self.assertEqual(state.bug_id, BUG_ID)
        self.assertEqual(state.signals, [])
        self.assertEqual(state.covered_surfaces, [])
        self.assertEqual(state.missing_surfaces, [])

    def test_empty_bug_id_raises(self) -> None:
        with self.assertRaises(AssuranceError):
            DiagnosticCoverageState("")
        with self.assertRaises(AssuranceError):
            DiagnosticCoverageState("   ")

    def test_unknown_signal_type_raises(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        with self.assertRaises(AssuranceError):
            state.record_hard_signal(
                signal_type="not_a_real_signal",
                description="should raise",
            )


class DiagnosticCoverageStateThresholdTests(unittest.TestCase):
    """Tests for the progressive threshold mechanism."""

    def test_single_hard_signal_does_not_trigger_at_threshold_2(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        triggered = state.record_hard_signal(
            signal_type="consecutive_same_failure",
            description="test fails with same assertion error",
        )
        self.assertFalse(triggered)
        self.assertEqual(state.hard_signal_count, 1.0)

    def test_two_hard_signals_trigger_at_threshold_2(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure",
            description="test fails with same assertion error",
        )
        triggered = state.record_hard_signal(
            signal_type="repeated_pattern",
            description="same NullPointerException in log",
        )
        self.assertTrue(triggered)
        self.assertEqual(state.hard_signal_count, 2.0)

    def test_threshold_increments_after_trigger_2_to_3_to_4_to_5_to_5(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)

        # Threshold 2 → trigger
        state.record_hard_signal(
            signal_type="consecutive_same_failure", description="fail 1")
        triggered = state.record_hard_signal(
            signal_type="repeated_pattern", description="fail 2")
        self.assertTrue(triggered)
        self.assertEqual(state.current_threshold, 2)
        state.acknowledge_trigger()
        self.assertEqual(state.current_threshold, 3)
        self.assertEqual(state.hard_signal_count, 0.0)
        self.assertEqual(state.trigger_count, 1)

        # Threshold 3 → trigger
        for desc in ["fail a", "fail b", "fail c"]:
            state.record_hard_signal(
                signal_type="consecutive_same_failure", description=desc)
        self.assertTrue(state.is_threshold_reached())
        state.acknowledge_trigger()
        self.assertEqual(state.current_threshold, 4)
        self.assertEqual(state.trigger_count, 2)

        # Threshold 4 → trigger
        for desc in ["fail x", "fail y", "fail z", "fail w"]:
            state.record_hard_signal(
                signal_type="consecutive_same_failure", description=desc)
        self.assertTrue(state.is_threshold_reached())
        state.acknowledge_trigger()
        self.assertEqual(state.current_threshold, 5)
        self.assertEqual(state.trigger_count, 3)

        # Threshold 5 → trigger
        for desc in ["f1", "f2", "f3", "f4", "f5"]:
            state.record_hard_signal(
                signal_type="consecutive_same_failure", description=desc)
        self.assertTrue(state.is_threshold_reached())
        state.acknowledge_trigger()
        # Threshold should stay at 5 (capped)
        self.assertEqual(state.current_threshold, 5)
        self.assertEqual(state.trigger_count, 4)


class DiagnosticCoverageStateWeightTests(unittest.TestCase):
    """Tests for soft signal weight handling (ADR-0010 §4.6.3)."""

    def test_every_pointable_evidence_counts_full_weight(self) -> None:
        # V11-IMPL-002: the 0.5 partial weight is abolished — new evidence
        # enters as a new hard signal at weight 1.0.
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure",
            description="full-weight failure",
        )
        self.assertEqual(state.hard_signal_count, 1.0)

        triggered = state.record_hard_signal(
            signal_type="key_surface_unexamined",
            description="new stack trace with different origin",
        )
        # 1.0 + 1.0 = 2.0 — at threshold
        self.assertTrue(triggered)
        self.assertEqual(state.hard_signal_count, 2.0)

    def test_soft_signal_does_not_count(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        triggered = state.record_soft_signal(
            description="found a completely new error category — direction validated",
        )
        self.assertFalse(triggered)
        self.assertEqual(state.hard_signal_count, 0.0)
        # Signal is still recorded in history for audit
        self.assertEqual(len(state.signals), 1)
        self.assertEqual(state.signals[0]["weight"], 0.0)


class DiagnosticCoverageStateResolveTests(unittest.TestCase):
    """Tests for the resolve() reset behaviour."""

    def test_resolve_resets_everything(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure", description="fail 1")
        state.record_hard_signal(
            signal_type="repeated_pattern", description="fail 2")
        state.acknowledge_trigger()
        state.set_diagnostic_surfaces(
            covered=["logs"],
            missing=["source inspection", "trace"],
        )

        self.assertEqual(state.current_threshold, 3)
        self.assertEqual(state.trigger_count, 1)
        self.assertGreater(len(state.signals), 0)

        state.resolve()

        self.assertEqual(state.current_threshold, 2)
        self.assertEqual(state.hard_signal_count, 0.0)
        self.assertEqual(state.trigger_count, 0)
        self.assertEqual(state.signals, [])
        self.assertEqual(state.covered_surfaces, [])
        self.assertEqual(state.missing_surfaces, [])


class DiagnosticCoverageStateSignalHistoryTests(unittest.TestCase):
    """Tests for signal history tracking."""

    def test_signals_record_timestamps_and_weights(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure",
            description="assertion error repeated 3x",
        )
        signals = state.signals
        self.assertEqual(len(signals), 1)
        self.assertEqual(signals[0]["signal_type"], "consecutive_same_failure")
        self.assertEqual(signals[0]["weight"], 1.0)
        self.assertIn("recorded_at", signals[0])

    def test_mixed_signal_types_preserved_in_order(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure", description="h1")
        state.record_soft_signal(description="s1")
        self.assertEqual(len(state.signals), 2)
        self.assertEqual(state.signals[0]["weight"], 1.0)
        self.assertEqual(state.signals[1]["weight"], 0.0)

    def test_snapshot_reflects_current_state(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure", description="h1")
        state.set_diagnostic_surfaces(
            covered=["logs"], missing=["trace"])
        snap = state.snapshot()
        self.assertEqual(snap["bug_id"], BUG_ID)
        self.assertEqual(snap["hard_signal_count"], 1.0)
        self.assertEqual(snap["current_threshold"], 2)
        self.assertEqual(snap["covered_surfaces"], ["logs"])
        self.assertEqual(snap["missing_surfaces"], ["trace"])


class DiagnosticCoverageStateSurfaceTests(unittest.TestCase):
    """Tests for diagnostic surface tracking."""

    def test_set_diagnostic_surfaces(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.set_diagnostic_surfaces(
            covered=["error logs", "stack trace"],
            missing=["source code inspection", "boundary condition test"],
        )
        self.assertEqual(
            state.covered_surfaces,
            ["error logs", "stack trace"],
        )
        self.assertEqual(
            state.missing_surfaces,
            ["source code inspection", "boundary condition test"],
        )


# ═══════════════════════════════════════════════════════════════════
# build_diagnostic_coverage_check tests
# ═══════════════════════════════════════════════════════════════════


class DiagnosticCoverageCheckBuildTests(unittest.TestCase):
    """Tests for checkpoint building."""

    def _sample_signals(self) -> list[dict[str, Any]]:
        from assurance.utils import utc_now

        return [
            {
                "signal_type": "consecutive_same_failure",
                "description": "test_foo fails with same assertion",
                "weight": 1.0,
                "recorded_at": utc_now(),
            },
            {
                "signal_type": "repeated_pattern",
                "description": "same NullPointer in production log",
                "weight": 1.0,
                "recorded_at": utc_now(),
            },
        ]

    def test_build_check_uses_neutral_message_block(self) -> None:
        check = build_diagnostic_coverage_check(
            bug_id=BUG_ID,
            hard_signal_count=2.0,
            current_threshold=2,
            trigger_count=0,
            signals=self._sample_signals(),
            covered_surfaces=["error logs"],
            missing_surfaces=["source inspection", "trace"],
        )

        self.assertIn("[DIAGNOSTIC_COVERAGE_CHECK v0.1]", check["message_block"])
        self.assertIn("关键诊断面", check["message_block"])
        self.assertIn("yes / no / uncertain", check["message_block"])
        # Must NOT contain correctness-evaluation language
        self.assertNotIn("是否正确", check["message_block"])
        self.assertNotIn("是否偏移", check["message_block"])
        self.assertNotIn("是否卡住", check["message_block"])

    def test_build_check_includes_signal_summary(self) -> None:
        check = build_diagnostic_coverage_check(
            bug_id=BUG_ID,
            hard_signal_count=2.0,
            current_threshold=2,
            trigger_count=0,
            signals=self._sample_signals(),
            covered_surfaces=[],
            missing_surfaces=[],
        )
        self.assertIn("test_foo fails with same assertion", check["message_block"])
        self.assertIn("same NullPointer in production log", check["message_block"])

    def test_build_check_claim_policy_all_false(self) -> None:
        check = build_diagnostic_coverage_check(
            bug_id=BUG_ID,
            hard_signal_count=2.0,
            current_threshold=2,
            trigger_count=1,
            signals=self._sample_signals(),
            covered_surfaces=[],
            missing_surfaces=[],
        )
        cp = check["claim_policy"]
        self.assertFalse(cp["may_generate_counterexample_candidate"])
        self.assertFalse(cp["may_set_claim_disposition"])
        self.assertEqual(cp["claim_strength_effect"], "none")
        self.assertFalse(cp["may_enter_global_review"])

    def test_build_check_rejects_invalid_threshold(self) -> None:
        with self.assertRaises(AssuranceError):
            build_diagnostic_coverage_check(
                bug_id=BUG_ID,
                hard_signal_count=1.0,
                current_threshold=1,  # below minimum
                trigger_count=0,
                signals=[],
                covered_surfaces=[],
                missing_surfaces=[],
            )

    def test_build_check_rejects_above_max_threshold(self) -> None:
        with self.assertRaises(AssuranceError):
            build_diagnostic_coverage_check(
                bug_id=BUG_ID,
                hard_signal_count=1.0,
                current_threshold=6,  # above maximum
                trigger_count=0,
                signals=[],
                covered_surfaces=[],
                missing_surfaces=[],
            )

    def test_build_check_from_state(self) -> None:
        state = DiagnosticCoverageState(BUG_ID)
        state.record_hard_signal(
            signal_type="consecutive_same_failure", description="h1")
        state.record_hard_signal(
            signal_type="repeated_pattern", description="h2")
        state.set_diagnostic_surfaces(
            covered=["logs"], missing=["trace"])

        check = build_diagnostic_coverage_check_from_state(state)
        self.assertEqual(check["bug_id"], BUG_ID)
        self.assertEqual(check["trigger"]["hard_signal_count"], 2.0)
        self.assertEqual(check["trigger"]["current_threshold"], 2)
        self.assertEqual(
            check["diagnostic_surfaces"]["covered"], ["logs"])
        self.assertEqual(
            check["diagnostic_surfaces"]["missing"], ["trace"])

    def test_empty_surfaces_renders_placeholder(self) -> None:
        check = build_diagnostic_coverage_check(
            bug_id=BUG_ID,
            hard_signal_count=2.0,
            current_threshold=2,
            trigger_count=0,
            signals=[],
            covered_surfaces=[],
            missing_surfaces=[],
        )
        self.assertIn("(无已记录硬信号)", check["message_block"])
        self.assertIn("(无已覆盖诊断面)", check["message_block"])
        self.assertIn("(未声明缺失诊断面)", check["message_block"])


# ═══════════════════════════════════════════════════════════════════
# evaluate_diagnostic_coverage_response tests
# ═══════════════════════════════════════════════════════════════════


class DiagnosticCoverageResponseTests(unittest.TestCase):
    """Tests for response evaluation."""

    def _build_check(self) -> dict[str, Any]:
        from assurance.utils import utc_now

        return build_diagnostic_coverage_check(
            bug_id=BUG_ID,
            hard_signal_count=2.0,
            current_threshold=2,
            trigger_count=1,
            signals=[{
                "signal_type": "consecutive_same_failure",
                "description": "repeated failure",
                "weight": 1.0,
                "recorded_at": utc_now(),
            }],
            covered_surfaces=["logs"],
            missing_surfaces=["source inspection"],
        )

    def test_evaluate_yes_response_valid(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "yes",
                "covered_diagnostic_surfaces": ["logs", "stack trace"],
                "missing_diagnostic_surfaces": [],
                "next_step": "continue with fix",
            },
        )
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["response_decision"], "yes")
        self.assertTrue(receipt["checks"]["neutral_diagnostic_only"])
        self.assertTrue(receipt["checks"]["no_counterexample_candidate"])
        self.assertTrue(receipt["checks"]["no_claim_disposition"])
        self.assertTrue(receipt["checks"]["no_global_review"])

    def test_evaluate_no_response_valid_with_single_next_step(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "no",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": ["source inspection", "trace"],
                "next_step": "check the error boundary in api_handler.py",
            },
        )
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["response_decision"], "no")
        self.assertEqual(receipt["next_step_count"], 1)
        self.assertTrue(receipt["checks"]["next_step_bounded"])

    def test_evaluate_uncertain_response_valid(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "uncertain",
                "covered_diagnostic_surfaces": ["error logs"],
                "missing_diagnostic_surfaces": ["full trace analysis"],
                "next_step": "run a smaller reproduction test",
            },
        )
        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["response_decision"], "uncertain")

    def test_rejects_counterexample_field(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "no",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": ["trace"],
                "next_step": "check the boundary",
                "counterexample_candidate": "maybe the approach is wrong",
            },
        )
        self.assertFalse(receipt["valid"])
        self.assertIn(
            "counterexample_candidate", receipt["forbidden_fields_observed"])
        self.assertFalse(receipt["checks"]["no_counterexample_candidate"])

    def test_rejects_claim_disposition_field(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "yes",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": [],
                "claim_disposition": "defer",
            },
        )
        self.assertFalse(receipt["valid"])
        self.assertIn("claim_disposition", receipt["forbidden_fields_observed"])

    def test_rejects_global_review_requested(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "uncertain",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": ["everything"],
                "next_step": "analyze",
                "global_review_requested": True,
            },
        )
        self.assertFalse(receipt["valid"])
        self.assertIn(
            "global_review_requested", receipt["forbidden_fields_observed"])
        self.assertFalse(receipt["checks"]["no_global_review"])

    def test_invalid_decision_reported_as_unexpected(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "maybe",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": ["trace"],
            },
        )
        self.assertFalse(receipt["valid"])
        # "decision" is an allowed field name — the invalid *value* is
        # reported via unexpected_fields_observed, not forbidden_fields_observed.
        self.assertTrue(
            any("decision" in u for u in receipt["unexpected_fields_observed"]),
            f"expected decision:... in unexpected_fields_observed, got {receipt['unexpected_fields_observed']}",
        )

    def test_unexpected_fields_reported(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={
                "decision": "yes",
                "covered_diagnostic_surfaces": ["logs"],
                "missing_diagnostic_surfaces": [],
                "random_extra_field": "should not be here",
            },
        )
        self.assertFalse(receipt["valid"])
        self.assertIn(
            "random_extra_field", receipt["unexpected_fields_observed"])

    def test_empty_response_invalid(self) -> None:
        check = self._build_check()
        receipt = evaluate_diagnostic_coverage_response(
            check=check,
            response={},
        )
        self.assertFalse(receipt["valid"])
        # Missing decision is reported via unexpected_fields_observed
        self.assertTrue(
            any("decision:missing" in u for u in receipt["unexpected_fields_observed"]),
            f"expected decision:missing in unexpected, got {receipt['unexpected_fields_observed']}",
        )


# ═══════════════════════════════════════════════════════════════════
# Constants tests
# ═══════════════════════════════════════════════════════════════════


class ConstantsTests(unittest.TestCase):
    """Sanity checks for module-level constants."""

    def test_hard_signal_types_valid(self) -> None:
        self.assertIn("consecutive_same_failure", HARD_SIGNAL_TYPES)
        self.assertIn("repeated_pattern", HARD_SIGNAL_TYPES)
        self.assertIn("same_module_no_evidence", HARD_SIGNAL_TYPES)
        self.assertIn("unabsorbed_new_evidence", HARD_SIGNAL_TYPES)
        self.assertIn("large_scope_low_diag", HARD_SIGNAL_TYPES)
        self.assertIn("key_surface_unexamined", HARD_SIGNAL_TYPES)
        self.assertEqual(len(HARD_SIGNAL_TYPES), 6)

    def test_allowed_fields_exclude_counterexample(self) -> None:
        self.assertNotIn(
            "counterexample_candidate", ALLOWED_DIAGNOSTIC_RESPONSE_FIELDS)
        self.assertNotIn(
            "claim_disposition", ALLOWED_DIAGNOSTIC_RESPONSE_FIELDS)

    def test_forbidden_fields_are_comprehensive(self) -> None:
        self.assertIn(
            "counterexample_candidate", FORBIDDEN_DIAGNOSTIC_RESPONSE_FIELDS)
        self.assertIn(
            "global_review_requested", FORBIDDEN_DIAGNOSTIC_RESPONSE_FIELDS)

    def test_threshold_constants(self) -> None:
        self.assertEqual(INITIAL_THRESHOLD, 2)
        self.assertEqual(MAX_THRESHOLD, 5)
