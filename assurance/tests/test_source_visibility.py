from __future__ import annotations

from copy import deepcopy
import json
from pathlib import Path
import unittest
from contextlib import redirect_stdout
from io import StringIO

from assurance import AssuranceError, evaluate_source_visibility_gate
from assurance.source_visibility_cli import main as source_visibility_cli_main


ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"


def _load_fixture() -> dict[str, object]:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


class SourceVisibilityGateTests(unittest.TestCase):
    def test_mixed_visibility_allows_metadata_defers_mechanism_and_allows_full_thread(
        self,
    ) -> None:
        receipt = evaluate_source_visibility_gate(_load_fixture())

        self.assertEqual(receipt["decision"], "defer")
        self.assertTrue(receipt["must_report_visibility_status"])
        decisions = {item["ref_id"]: item for item in receipt["reference_decisions"]}
        self.assertEqual(decisions["S1"]["decision"], "allow")
        self.assertEqual(decisions["S1"]["claim_allowed"], "metadata_only")
        self.assertEqual(decisions["S2"]["decision"], "defer")
        self.assertEqual(decisions["S2"]["required_visibility"], "full_text_observed")
        self.assertIn(
            "SOURCE-VISIBILITY-FULLTEXT-REQUIRED",
            decisions["S2"]["reason_codes"],
        )
        self.assertEqual(decisions["S2"]["claim_allowed"], "observed_fragment_only")
        self.assertEqual(decisions["S3"]["decision"], "allow")
        self.assertEqual(decisions["S3"]["claim_allowed"], "full_text_claim")

    def test_full_text_mechanism_claim_allows_without_forcing_all_sources_fulltext(
        self,
    ) -> None:
        ledger = _load_fixture()
        ledger["retrieval_policy"]["max_fulltext_sources_per_pass"] = 2
        abstract_source = next(
            item
            for item in ledger["sources"]
            if item["source_id"] == "SRC-ABSTRACT-001"
        )
        abstract_source["visibility_status"] = "full_text_observed"
        abstract_source["observed_scope"].append("full_body")
        abstract_source["missing_scope"] = []
        abstract_source["retrieval_attempts"].append(
            {
                "attempt_id": "ATTEMPT-ABSTRACT-FULL-001",
                "attempted_at": "2026-07-26T12:20:00Z",
                "retrieval_mode": "full_text",
                "result": "observed",
                "observed_bytes": 12000,
                "context_tokens_estimate": 2200,
                "note": "Full text observed on second pass.",
            }
        )
        ledger["gate_request"]["cited_ref_ids"] = ["S2"]

        receipt = evaluate_source_visibility_gate(ledger)

        self.assertEqual(receipt["decision"], "allow")
        self.assertEqual(receipt["source_count"], 3)
        self.assertEqual(receipt["reference_decisions"][0]["claim_allowed"], "full_text_claim")

    def test_missing_reference_blocks_even_when_sources_exist(self) -> None:
        ledger = _load_fixture()
        ledger["gate_request"]["cited_ref_ids"].append("S99")

        receipt = evaluate_source_visibility_gate(ledger)

        self.assertEqual(receipt["decision"], "block")
        missing = receipt["reference_decisions"][-1]
        self.assertEqual(missing["ref_id"], "S99")
        self.assertEqual(
            missing["reason_codes"], ["SOURCE-VISIBILITY-UNREGISTERED-REFERENCE"]
        )

    def test_context_budget_exceeded_defers_even_for_full_text_reference(self) -> None:
        ledger = _load_fixture()
        ledger["retrieval_policy"]["max_context_tokens_per_pass"] = 1
        ledger["gate_request"]["cited_ref_ids"] = ["S3"]

        receipt = evaluate_source_visibility_gate(ledger)

        self.assertEqual(receipt["decision"], "defer")
        self.assertEqual(
            receipt["warnings"],
            ["context token estimate exceeds max_context_tokens_per_pass"],
        )
        self.assertIn(
            "SOURCE-VISIBILITY-BUDGET-EXCEEDED",
            receipt["reference_decisions"][0]["reason_codes"],
        )

    def test_unavailable_source_blocks_evidence_use(self) -> None:
        ledger = _load_fixture()
        source = deepcopy(ledger["sources"][1])
        source["source_id"] = "SRC-UNAVAILABLE-001"
        source["visibility_status"] = "unavailable"
        source["observed_scope"] = []
        source["missing_scope"] = ["full body"]
        source["retrieval_attempts"][0]["result"] = "blocked"
        ledger["sources"] = [source]
        ledger["references"] = [
            {
                "ref_id": "S1",
                "source_id": "SRC-UNAVAILABLE-001",
                "claim_type": "section_summary",
                "claim_text": "The source reports a limitation.",
                "intended_use": "evidence",
                "requires_full_text_if_used_as_written": False,
            }
        ]
        ledger["gate_request"]["cited_ref_ids"] = ["S1"]

        receipt = evaluate_source_visibility_gate(ledger)

        self.assertEqual(receipt["decision"], "block")
        self.assertEqual(
            receipt["reference_decisions"][0]["reason_codes"],
            ["SOURCE-VISIBILITY-UNAVAILABLE"],
        )

    def test_cli_emits_json_receipt(self) -> None:
        output = StringIO()
        with redirect_stdout(output):
            exit_code = source_visibility_cli_main(["--ledger", str(FIXTURE)])
        receipt = json.loads(output.getvalue())

        self.assertEqual(exit_code, 0)
        self.assertEqual(receipt["decision"], "defer")

    def test_schema_rejects_fulltext_without_full_body_scope(self) -> None:
        ledger = _load_fixture()
        source = ledger["sources"][2]
        source["observed_scope"] = ["main_post", "comments"]

        with self.assertRaisesRegex(AssuranceError, "does not contain"):
            evaluate_source_visibility_gate(ledger)
