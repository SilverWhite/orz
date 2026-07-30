"""Tests for CaseRetrievalGuard — blind-first enforcement against anchoring bias."""

from __future__ import annotations

import unittest

from assurance.case_retrieval_guard import (
    BlindPrecommitment,
    CaseRetrievalGuard,
    GuardState,
    RetrievalRecord,
)
from assurance.errors import AssuranceError


# ══════════════════════════════════════════════════════════════════════════════
# 1. Precommitment
# ══════════════════════════════════════════════════════════════════════════════


class PrecommitmentTests(unittest.TestCase):
    """Tests for blind precommitment recording."""

    def setUp(self) -> None:
        self.guard = CaseRetrievalGuard()

    def test_precommit_creates_immutable_record(self) -> None:
        record = self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Based on visible facts, X is supported.",
            proposed_gates={"EVD-PROVENANCE-001": "defer"},
            claim_candidates=["X is supported by the source."],
            claim_strength="observed_fragment_only",
        )
        self.assertIsInstance(record, BlindPrecommitment)
        self.assertEqual(record.case_id, "FEP-TEST-001")
        self.assertEqual(self.guard.state("FEP-TEST-001"), GuardState.PRECOMMITTED)

    def test_precommit_records_timestamp(self) -> None:
        record = self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Test.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.assertTrue(record.timestamp)
        self.assertIn("T", record.timestamp)
        self.assertTrue(record.timestamp.endswith("Z"))

    def test_precommit_has_content_hash(self) -> None:
        record = self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Test.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.assertEqual(len(record.content_sha256), 64)

    def test_duplicate_precommit_raises(self) -> None:
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="First judgment.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        with self.assertRaises(AssuranceError) as ctx:
            self.guard.precommit(
                case_id="FEP-TEST-001",
                precommitment_text="Revised judgment.",
                proposed_gates={},
                claim_candidates=[],
                claim_strength="none",
            )
        self.assertIn("already exists", str(ctx.exception))

    def test_empty_precommitment_text_raises(self) -> None:
        with self.assertRaises(AssuranceError):
            self.guard.precommit(
                case_id="FEP-TEST-001",
                precommitment_text="   ",
                proposed_gates={},
                claim_candidates=[],
                claim_strength="none",
            )

    def test_has_valid_precommitment_returns_correctly(self) -> None:
        self.assertFalse(self.guard.has_valid_precommitment("FEP-TEST-001"))
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Test.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.assertTrue(self.guard.has_valid_precommitment("FEP-TEST-001"))


# ══════════════════════════════════════════════════════════════════════════════
# 2. Retrieval lifecycle (blind-first enforcement)
# ══════════════════════════════════════════════════════════════════════════════


class RetrievalLifecycleTests(unittest.TestCase):
    """Tests for the retrieval lifecycle and blind-first enforcement."""

    def setUp(self) -> None:
        self.guard = CaseRetrievalGuard()
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Independent judgment.",
            proposed_gates={"EVD-PROVENANCE-001": "defer"},
            claim_candidates=["X is supported."],
            claim_strength="observed_fragment_only",
        )

    def test_retrieval_blocked_without_precommitment(self) -> None:
        with self.assertRaises(AssuranceError) as ctx:
            self.guard.begin_retrieval("FEP-NO-PRECOMMIT")
        self.assertIn("blind precommitment", str(ctx.exception))

    def test_full_retrieval_lifecycle(self) -> None:
        self.guard.begin_retrieval("FEP-TEST-001")
        self.assertEqual(self.guard.state("FEP-TEST-001"), GuardState.RETRIEVING)

        record = self.guard.complete_retrieval(
            case_id="FEP-TEST-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Both involve missing CLI flags."],
            disanalogies=["FEP-REG-005 has a default; this case has no default."],
            conclusion_changing_facts=["No default exists in this version."],
            reasoning_revision="Revised: the flag is truly absent.",
        )
        self.assertEqual(self.guard.state("FEP-TEST-001"), GuardState.RETRIEVED)
        self.assertIsInstance(record, RetrievalRecord)
        self.assertIn("FEP-REG-005", record.retrieved_case_ids)

    def test_complete_retrieval_without_begin_raises(self) -> None:
        with self.assertRaises(AssuranceError):
            self.guard.complete_retrieval(
                case_id="FEP-TEST-001",
                retrieved_case_ids=["FEP-REG-005"],
                similarities=[],
                disanalogies=["Different version."],
                conclusion_changing_facts=[],
                reasoning_revision="No change.",
            )

    def test_empty_disanalogies_blocked(self) -> None:
        """BIAS-SURFACE-001: retrieval without disanalogies is blocked."""
        self.guard.begin_retrieval("FEP-TEST-001")
        with self.assertRaises(AssuranceError) as ctx:
            self.guard.complete_retrieval(
                case_id="FEP-TEST-001",
                retrieved_case_ids=["FEP-REG-005"],
                similarities=["Similar surface pattern."],
                disanalogies=[],  # empty — blocked
                conclusion_changing_facts=[],
                reasoning_revision="No change.",
            )
        self.assertIn("disanalogy", str(ctx.exception))

    def test_record_revision_updates_state(self) -> None:
        self.guard.begin_retrieval("FEP-TEST-001")
        self.guard.complete_retrieval(
            case_id="FEP-TEST-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Similar."],
            disanalogies=["Different."],
            conclusion_changing_facts=[],
            reasoning_revision="Initial revision.",
        )
        self.guard.record_revision(
            case_id="FEP-TEST-001",
            reasoning_revision="Further revised after discussion.",
        )
        self.assertEqual(
            self.guard.state("FEP-TEST-001"), GuardState.REVISION_RECORDED,
        )

    def test_begin_retrieval_twice_raises(self) -> None:
        self.guard.begin_retrieval("FEP-TEST-001")
        with self.assertRaises(AssuranceError):
            self.guard.begin_retrieval("FEP-TEST-001")


# ══════════════════════════════════════════════════════════════════════════════
# 3. Queries and audit
# ══════════════════════════════════════════════════════════════════════════════


class QueryTests(unittest.TestCase):
    """Tests for query methods and audit summaries."""

    def setUp(self) -> None:
        self.guard = CaseRetrievalGuard()

    def test_initial_state_is_idle(self) -> None:
        self.assertEqual(self.guard.state("FEP-TEST-001"), GuardState.IDLE)

    def test_has_disanalogies_false_without_retrieval(self) -> None:
        self.assertFalse(self.guard.has_disanalogies("FEP-TEST-001"))

    def test_has_disanalogies_true_after_retrieval(self) -> None:
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Test.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.guard.begin_retrieval("FEP-TEST-001")
        self.guard.complete_retrieval(
            case_id="FEP-TEST-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Similar."],
            disanalogies=["Different version."],
            conclusion_changing_facts=[],
            reasoning_revision="No change.",
        )
        self.assertTrue(self.guard.has_disanalogies("FEP-TEST-001"))

    def test_audit_summary_precommitment_only(self) -> None:
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Independent judgment.",
            proposed_gates={"EVD-PROVENANCE-001": "defer"},
            claim_candidates=["X is supported."],
            claim_strength="observed_fragment_only",
        )
        summary = self.guard.audit_summary("FEP-TEST-001")
        self.assertTrue(summary["blind_first_satisfied"])
        self.assertFalse(summary["retrieval_performed"])
        self.assertIsNotNone(summary["precommitment_timestamp"])

    def test_audit_summary_full_cycle(self) -> None:
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Independent judgment.",
            proposed_gates={"EVD-PROVENANCE-001": "defer"},
            claim_candidates=["X is supported."],
            claim_strength="observed_fragment_only",
        )
        self.guard.begin_retrieval("FEP-TEST-001")
        self.guard.complete_retrieval(
            case_id="FEP-TEST-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Similar surface pattern."],
            disanalogies=["Different underlying mechanism."],
            conclusion_changing_facts=["No default flag exists."],
            reasoning_revision="Revised conclusion.",
        )
        summary = self.guard.audit_summary("FEP-TEST-001")
        self.assertTrue(summary["blind_first_satisfied"])
        self.assertTrue(summary["retrieval_performed"])
        self.assertEqual(summary["retrieved_case_count"], 1)
        self.assertEqual(summary["disanalogies_recorded"], 1)
        self.assertEqual(summary["similarities_recorded"], 1)
        self.assertEqual(summary["conclusion_changing_facts"], 1)
        self.assertTrue(summary["bias_surface_satisfied"])

    def test_get_precommitment_returns_none_for_missing(self) -> None:
        self.assertIsNone(self.guard.get_precommitment("nonexistent"))

    def test_get_retrieval_returns_none_for_missing(self) -> None:
        self.assertIsNone(self.guard.get_retrieval("nonexistent"))

    def test_counts_track_all_entries(self) -> None:
        self.assertEqual(self.guard.precommitment_count(), 0)
        self.assertEqual(self.guard.retrieval_count(), 0)
        self.guard.precommit(
            case_id="FEP-TEST-001",
            precommitment_text="Test.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.assertEqual(self.guard.precommitment_count(), 1)
        self.guard.begin_retrieval("FEP-TEST-001")
        self.guard.complete_retrieval(
            case_id="FEP-TEST-001",
            retrieved_case_ids=["FEP-REG-005"],
            similarities=["Similar."],
            disanalogies=["Different."],
            conclusion_changing_facts=[],
            reasoning_revision="Revised.",
        )
        self.assertEqual(self.guard.retrieval_count(), 1)


# ══════════════════════════════════════════════════════════════════════════════
# 4. Multi-case isolation
# ══════════════════════════════════════════════════════════════════════════════


class MultiCaseTests(unittest.TestCase):
    """Tests that cases are independent."""

    def setUp(self) -> None:
        self.guard = CaseRetrievalGuard()

    def test_independent_precommitments(self) -> None:
        self.guard.precommit(
            case_id="CASE-A",
            precommitment_text="Judgment A.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.guard.precommit(
            case_id="CASE-B",
            precommitment_text="Judgment B.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.assertEqual(self.guard.precommitment_count(), 2)
        self.assertEqual(self.guard.state("CASE-A"), GuardState.PRECOMMITTED)
        self.assertEqual(self.guard.state("CASE-B"), GuardState.PRECOMMITTED)

    def test_retrieval_on_a_does_not_affect_b(self) -> None:
        self.guard.precommit(
            case_id="CASE-A",
            precommitment_text="Judgment A.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.guard.precommit(
            case_id="CASE-B",
            precommitment_text="Judgment B.",
            proposed_gates={},
            claim_candidates=[],
            claim_strength="none",
        )
        self.guard.begin_retrieval("CASE-A")
        self.guard.complete_retrieval(
            case_id="CASE-A",
            retrieved_case_ids=["OLD-CASE"],
            similarities=["Similar."],
            disanalogies=["Different."],
            conclusion_changing_facts=[],
            reasoning_revision="Revised A.",
        )
        # CASE-B is still in PRECOMMITTED state
        self.assertEqual(self.guard.state("CASE-B"), GuardState.PRECOMMITTED)
        self.assertFalse(self.guard.has_disanalogies("CASE-B"))


if __name__ == "__main__":
    unittest.main()
