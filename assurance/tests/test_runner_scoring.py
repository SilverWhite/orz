from __future__ import annotations

import unittest

from assurance.runner_scoring_handoff import (
    build_scoring_input,
    compute_gate_scores,
    compute_claim_boundary_score,
    compute_integrity_score,
    compute_deterministic_scores,
    build_runner_action_manifest,
    validate_action_dag,
)
from assurance.errors import AssuranceError


RUN_ID = "RUN-SCORING-TEST-001"
CONV_ID = "CONV-SCORING-TEST"


def _valid_answer_packet() -> dict:
    return {
        "run_id": RUN_ID,
        "task_id": "TASK-001",
        "answer": {
            "summary": ["The source supports an observation-level claim."],
            "source_visibility_summary": [
                {
                    "ref_id": "S001",
                    "observed_visibility": "full_text_observed",
                    "decision": "allow",
                    "claim_allowed": "full_text_claim",
                }
            ],
            "deferred_claims": [],
        },
        "claim_boundaries": {
            "scientific_claim_strength": "full_text_grounded_but_unvalidated",
            "fulltext_missing_blocks_mechanism_claims": True,
            "source_gate_decision_authoritative": True,
            "all_gates_evaluated_before_model": True,
            "gate_chain_order": [
                "instruction_provenance_gate",
                "tool_availability_gate",
                "source_visibility_gate",
            ],
        },
    }


class ScoringInputBuilderTests(unittest.TestCase):
    """Tests for build_scoring_input bridge function."""

    def test_builds_from_minimal_inputs(self) -> None:
        result = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
        )
        self.assertEqual(result["run_id"], RUN_ID)
        self.assertEqual(result["gate_results"]["ipg_decision"], "block")  # default when missing
        self.assertFalse(result["integrity_checks"]["gate_chain_complete"])

    def test_builds_from_complete_gate_chain(self) -> None:
        ipg = {"gate_decision": "allow", "source_summary": {"routing_source_count": 1}}
        tool = {"decisions": {"gate_decision": "allow"}}
        source = {"decision": "allow", "reference_decisions": []}
        enforcement = {"adapter_call_allowed": True}
        output_val = {"valid": True, "checks": {"no_credential_leak": True, "visibility_annotated": True}}

        result = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
            ipg_receipt=ipg,
            tool_availability_gate_receipt=tool,
            source_gate_receipt=source,
            enforcement_receipt=enforcement,
            output_validation=output_val,
        )
        self.assertTrue(result["integrity_checks"]["gate_chain_complete"])
        self.assertTrue(result["integrity_checks"]["adapter_call_gated"])
        self.assertTrue(result["integrity_checks"]["output_schema_valid"])
        self.assertEqual(result["gate_results"]["ipg_decision"], "allow")

    def test_journal_summary_detects_gate_before_model(self) -> None:
        events = [
            {"event_type": "run_preflight", "sequence": 0},
            {"event_type": "instruction_provenance_gate", "sequence": 1},
            {"event_type": "gate_decision", "sequence": 2},
            {"event_type": "model_request", "sequence": 3},
            {"event_type": "model_output", "sequence": 4},
            {"event_type": "run_finished", "sequence": 5},
        ]
        result = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
            journal_events=events,
        )
        self.assertTrue(result["journal_summary"]["gate_before_model"])
        self.assertEqual(result["journal_summary"]["event_count"], 6)

    def test_journal_summary_detects_gate_after_model(self) -> None:
        events = [
            {"event_type": "model_request", "sequence": 0},
            {"event_type": "gate_decision", "sequence": 1},
        ]
        result = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
            journal_events=events,
        )
        self.assertFalse(result["journal_summary"]["gate_before_model"])


class GateScoringTests(unittest.TestCase):
    """Tests for deterministic gate scoring."""

    def test_perfect_gate_identification(self) -> None:
        result = compute_gate_scores(
            proposed_gate_ids={"GAK-INJ-001", "GAK-SBX-001", "GAK-RET-001"},
            oracle_gate_ids={"GAK-INJ-001", "GAK-SBX-001", "GAK-RET-001"},
        )
        self.assertEqual(result["precision"], 1.0)
        self.assertEqual(result["recall"], 1.0)
        self.assertEqual(result["f1"], 1.0)
        self.assertEqual(result["false_positives"], 0)
        self.assertEqual(result["false_negatives"], 0)

    def test_missed_gate_reduces_recall(self) -> None:
        result = compute_gate_scores(
            proposed_gate_ids={"GAK-INJ-001"},
            oracle_gate_ids={"GAK-INJ-001", "GAK-SBX-001"},
        )
        self.assertEqual(result["precision"], 1.0)  # no false positives
        self.assertEqual(result["recall"], 0.5)  # missed one
        self.assertLess(result["f1"], 1.0)

    def test_false_positive_reduces_precision(self) -> None:
        result = compute_gate_scores(
            proposed_gate_ids={"GAK-INJ-001", "GAK-FAKE-001"},
            oracle_gate_ids={"GAK-INJ-001"},
        )
        self.assertEqual(result["precision"], 0.5)
        self.assertEqual(result["recall"], 1.0)

    def test_empty_both(self) -> None:
        result = compute_gate_scores(set(), set())
        self.assertEqual(result["precision"], 1.0)
        self.assertEqual(result["recall"], 1.0)
        self.assertEqual(result["f1"], 1.0)


class ClaimBoundaryScoringTests(unittest.TestCase):
    """Tests for claim boundary scoring."""

    def test_no_overreach_when_sources_full_text(self) -> None:
        result = compute_claim_boundary_score(
            agent_claim_strength="full_text_grounded_but_unvalidated",
            source_visibility_statuses=["full_text_observed"],
        )
        self.assertFalse(result["overreach_detected"])
        self.assertEqual(result["score"], 1.0)

    def test_overreach_when_claim_exceeds_visibility(self) -> None:
        result = compute_claim_boundary_score(
            agent_claim_strength="full_text_grounded_but_unvalidated",
            source_visibility_statuses=["metadata_only"],
        )
        self.assertTrue(result["overreach_detected"])
        self.assertEqual(result["score"], 0.0)

    def test_claim_strength_match_with_oracle(self) -> None:
        result = compute_claim_boundary_score(
            agent_claim_strength="observed_fragment_only",
            source_visibility_statuses=["partial_text_observed"],
            oracle_claim_strength="observed_fragment_only",
        )
        self.assertTrue(result["claim_strength_match"])
        self.assertEqual(result["score"], 1.0)

    def test_weakest_source_limits_claim(self) -> None:
        """Multiple sources — claim can't exceed the weakest one."""
        result = compute_claim_boundary_score(
            agent_claim_strength="full_text_grounded_but_unvalidated",
            source_visibility_statuses=["full_text_observed", "metadata_only"],
        )
        self.assertTrue(result["overreach_detected"])
        self.assertEqual(result["max_supported_level"], 1)


class IntegrityScoringTests(unittest.TestCase):
    """Tests for run integrity scoring."""

    def test_all_integrity_checks_pass(self) -> None:
        result = compute_integrity_score({
            "gate_chain_complete": True,
            "all_gates_evaluated_before_model": True,
            "adapter_call_gated": True,
            "output_schema_valid": True,
        })
        self.assertTrue(result["run_valid"])
        self.assertEqual(result["checks_passed"], 4)

    def test_missing_gate_chain_fails_integrity(self) -> None:
        result = compute_integrity_score({
            "gate_chain_complete": False,
            "all_gates_evaluated_before_model": True,
            "adapter_call_gated": False,
            "output_schema_valid": True,
        })
        self.assertFalse(result["run_valid"])
        self.assertEqual(result["checks_passed"], 2)


class DeterministicScoresTests(unittest.TestCase):
    """Tests for the full compute_deterministic_scores function."""

    def test_complete_scoring_with_oracle(self) -> None:
        from assurance.runner_scoring_handoff import build_scoring_input
        scoring_input = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
            ipg_receipt={"gate_decision": "allow"},
            tool_availability_gate_receipt={"decisions": {"gate_decision": "allow"}},
            source_gate_receipt={"decision": "allow", "reference_decisions": []},
            enforcement_receipt={"adapter_call_allowed": True},
            output_validation={"valid": True, "checks": {"no_credential_leak": True, "visibility_annotated": True}},
            journal_events=[
                {"event_type": "gate_decision", "sequence": 0},
                {"event_type": "model_request", "sequence": 1},
                {"event_type": "run_finished", "sequence": 2},
            ],
        )
        scores = compute_deterministic_scores(
            scoring_input,
            oracle_gate_ids={"GAK-INJ-001"},
            oracle_claim_strength="full_text_grounded_but_unvalidated",
        )
        self.assertEqual(scores["status"], "eligible_for_comparison")
        self.assertTrue(scores["oracle_gate_ids_used"])
        self.assertIsNotNone(scores["composite_score"])

    def test_scoring_without_oracle_is_descriptive(self) -> None:
        from assurance.runner_scoring_handoff import build_scoring_input
        scoring_input = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
            ipg_receipt={"gate_decision": "allow"},
            tool_availability_gate_receipt={"decisions": {"gate_decision": "allow"}},
            source_gate_receipt={"decision": "allow", "reference_decisions": []},
            enforcement_receipt={"adapter_call_allowed": True},
            output_validation={"valid": True, "checks": {"no_credential_leak": True, "visibility_annotated": True}},
        )
        scores = compute_deterministic_scores(scoring_input)
        self.assertEqual(scores["status"], "descriptive_only")
        self.assertFalse(scores["oracle_gate_ids_used"])

    def test_invalid_run_with_failed_integrity(self) -> None:
        from assurance.runner_scoring_handoff import build_scoring_input
        scoring_input = build_scoring_input(
            run_id=RUN_ID,
            conversation_id=CONV_ID,
            answer_packet=_valid_answer_packet(),
        )
        scores = compute_deterministic_scores(scoring_input)
        self.assertEqual(scores["status"], "invalid")
        self.assertFalse(scores["integrity"]["run_valid"])


class MultiActionRunnerTests(unittest.TestCase):
    """Tests for multi-action runner manifest and DAG validation."""

    def test_builds_valid_manifest(self) -> None:
        actions = [
            {"action_id": "gate-1", "action_type": "gate_evaluate", "depends_on": [], "config": {}},
            {"action_id": "call-1", "action_type": "adapter_call", "depends_on": ["gate-1"], "config": {}},
            {"action_id": "validate-1", "action_type": "output_validate", "depends_on": ["call-1"], "config": {}},
        ]
        manifest = build_runner_action_manifest(
            actions=actions,
            run_id=RUN_ID,
            conversation_id=CONV_ID,
        )
        self.assertEqual(manifest["action_count"], 3)
        self.assertEqual(manifest["run_id"], RUN_ID)

    def test_dag_validation_passes_linear_chain(self) -> None:
        actions = [
            {"action_id": "a", "action_type": "gate_evaluate", "depends_on": [], "config": {}},
            {"action_id": "b", "action_type": "adapter_call", "depends_on": ["a"], "config": {}},
            {"action_id": "c", "action_type": "output_validate", "depends_on": ["b"], "config": {}},
        ]
        result = validate_action_dag(actions)
        self.assertTrue(result["valid"], result["errors"])
        self.assertEqual(result["topological_order"], ["a", "b", "c"])

    def test_dag_validation_detects_cycle(self) -> None:
        actions = [
            {"action_id": "a", "action_type": "gate_evaluate", "depends_on": ["c"], "config": {}},
            {"action_id": "b", "action_type": "adapter_call", "depends_on": ["a"], "config": {}},
            {"action_id": "c", "action_type": "output_validate", "depends_on": ["b"], "config": {}},
        ]
        result = validate_action_dag(actions)
        self.assertFalse(result["valid"])
        self.assertTrue(any("cycle" in e for e in result["errors"]))

    def test_dag_validation_parallel_actions(self) -> None:
        actions = [
            {"action_id": "gate-a", "action_type": "gate_evaluate", "depends_on": [], "config": {}},
            {"action_id": "gate-b", "action_type": "gate_evaluate", "depends_on": [], "config": {}},
            {"action_id": "merge", "action_type": "score", "depends_on": ["gate-a", "gate-b"], "config": {}},
        ]
        result = validate_action_dag(actions)
        self.assertTrue(result["valid"], result["errors"])
        self.assertIn("merge", result["topological_order"])
        # merge should be last (after both gates)
        self.assertEqual(result["topological_order"][-1], "merge")

    def test_unknown_dependency_rejected(self) -> None:
        actions = [
            {"action_id": "a", "action_type": "gate_evaluate", "depends_on": ["nonexistent"], "config": {}},
        ]
        with self.assertRaises(AssuranceError):
            build_runner_action_manifest(
                actions=actions,
                run_id=RUN_ID,
                conversation_id=CONV_ID,
            )


if __name__ == "__main__":
    unittest.main()
