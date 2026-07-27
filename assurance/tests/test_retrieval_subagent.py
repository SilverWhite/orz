from __future__ import annotations

import unittest
from pathlib import Path
import tempfile

from assurance import (
    build_fake_retrieval_result,
    build_retrieval_session_close_receipt,
    build_retrieval_task_contract,
    run_retrieval_subagent_fixture,
    validate_retrieval_result,
    verify_retrieval_result_sources,
    verify_retrieval_subagent_fixture,
)
from assurance.errors import AssuranceError

PARENT_SESSION_ID = "CONV-AAAAAAAAAAAAAABBBBBBBBBBBBBBBBBB"


class RetrievalTaskContractTests(unittest.TestCase):
    def test_builds_minimal_contract(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Find the latest LIF theoretical framework.",
            allowed_source_categories=["academic_paper", "web_page"],
        )
        self.assertEqual(contract["contract_kind"], "retrieval_task_contract")
        self.assertTrue(contract["contract_id"].startswith("RET-CTR-"))
        self.assertEqual(contract["parent_session_id"], PARENT_SESSION_ID)
        self.assertTrue(contract["return_format"]["source_ledger_required"])
        self.assertTrue(contract["return_format"]["filtering_log_required"])
        self.assertIn("subagent_must_not_expand_scope", contract["scope_boundary"]["delegation_constraint"])

    def test_custom_sections_and_topics(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Explain the P0 contract structure.",
            allowed_source_categories=["documentation"],
            return_sections=["overview", "architecture", "code_analysis"],
            forbidden_topics=["credential", "internal_config"],
            max_sources=10,
            context_budget_tokens=4096,
        )
        self.assertEqual(contract["return_format"]["sections"], ["overview", "architecture", "code_analysis"])
        self.assertIn("credential", contract["scope_boundary"]["forbidden_topics"])
        self.assertEqual(contract["max_sources"], 10)
        self.assertEqual(contract["context_budget_tokens"], 4096)

    def test_invalid_parent_session_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "CONV-"):
            build_retrieval_task_contract(
                parent_session_id="NOT-A-CONV-ID",
                retrieval_question="Test.",
                allowed_source_categories=["web_page"],
            )

    def test_empty_question_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "non-empty"):
            build_retrieval_task_contract(
                parent_session_id=PARENT_SESSION_ID,
                retrieval_question="   ",
                allowed_source_categories=["web_page"],
            )

    def test_no_source_categories_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "non-empty"):
            build_retrieval_task_contract(
                parent_session_id=PARENT_SESSION_ID,
                retrieval_question="Test.",
                allowed_source_categories=[],
            )

    def test_invalid_visibility_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "required_visibility"):
            build_retrieval_task_contract(
                parent_session_id=PARENT_SESSION_ID,
                retrieval_question="Test.",
                allowed_source_categories=["web_page"],
                required_visibility="invalid_level",
            )

    def test_budget_out_of_range_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "between 1 and 50"):
            build_retrieval_task_contract(
                parent_session_id=PARENT_SESSION_ID,
                retrieval_question="Test.",
                allowed_source_categories=["web_page"],
                max_sources=100,
            )


class RetrievalResultValidationTests(unittest.TestCase):
    def _sample_contract(self) -> dict:
        return build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Find the LIF source routing rules.",
            allowed_source_categories=["academic_paper", "documentation"],
            required_visibility="full_text_observed",
            return_sections=["summary", "key_findings", "source_breakdown"],
        )

    def _sample_sources(self) -> list[dict]:
        return [
            {
                "source_id": "SRC-LIF-001",
                "source_title": "LIF Framework v3",
                "source_url_or_ref": "https://example.com/lif-v3",
                "visibility": "full_text_observed",
                "relevance": "direct",
                "used_in_sections": ["summary", "key_findings"],
                "content_sha256": "a" * 64,
                "full_text_retrieved": True,
                "notes": "primary source",
            },
            {
                "source_id": "SRC-LIF-002",
                "source_title": "LIF Routing Addendum",
                "source_url_or_ref": "https://example.com/lif-routing",
                "visibility": "full_text_observed",
                "relevance": "direct",
                "used_in_sections": ["source_breakdown"],
                "content_sha256": "b" * 64,
                "full_text_retrieved": True,
                "notes": "secondary source",
            },
        ]

    def test_result_validates_against_contract(self) -> None:
        contract = self._sample_contract()
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            query_texts=["LIF source routing rules"],
            source_entries=self._sample_sources(),
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["checks"]["source_ledger_valid"])
        self.assertTrue(receipt["checks"]["scope_compliant"])
        self.assertTrue(receipt["checks"]["return_format_complete"])

    def test_contract_id_mismatch_detected(self) -> None:
        contract = self._sample_contract()
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            source_entries=self._sample_sources(),
        )
        result["contract_id"] = "RET-CTR-WRONG"
        with self.assertRaisesRegex(AssuranceError, "contract_id mismatch"):
            validate_retrieval_result(contract=contract, result=result)

    def test_insufficient_visibility_detected(self) -> None:
        contract = self._sample_contract()
        sources = self._sample_sources()
        sources[0]["visibility"] = "metadata_only"
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            source_entries=sources,
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertFalse(receipt["valid"])
        self.assertEqual(len(receipt["source_ledger_verification"]["insufficient_visibility"]), 1)

    def test_scope_violation_detected(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Find information about LIF.",
            allowed_source_categories=["academic_paper", "documentation"],
            required_visibility="full_text_observed",
            return_sections=["summary", "key_findings", "source_breakdown"],
            forbidden_topics=["credential", "internal_config"],
        )
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            query_texts=["credential management secrets"],
            source_entries=self._sample_sources(),
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertFalse(receipt["valid"])
        self.assertFalse(receipt["scope_compliance"]["checks"]["no_forbidden_topics"])

    def test_missing_section_detected(self) -> None:
        contract = self._sample_contract()
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            source_entries=self._sample_sources(),
            sections_content=[
                {"section_title": "summary", "content": "Only one section."},
            ],
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertFalse(receipt["valid"])
        self.assertIn("key_findings", receipt["return_format_check"]["missing_sections"])

    def test_filtering_log_entries_preserved(self) -> None:
        contract = self._sample_contract()
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            source_entries=self._sample_sources(),
            filter_entries=[
                {
                    "source_id": "SRC-EXCLUDED-001",
                    "reason": "scope_violation",
                    "action": "excluded",
                    "filtered_at": "2026-07-27T00:00:00Z",
                },
            ],
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["source_ledger_verification"]["checks"]["filtering_log_present"])


class SessionCloseReceiptTests(unittest.TestCase):
    def test_close_receipt_confirms_no_zeroing(self) -> None:
        receipt = build_retrieval_session_close_receipt(
            parent_session_id=PARENT_SESSION_ID,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            contract_id="RET-CTR-TEST-001",
            result_id="RET-RES-TEST-001",
        )
        self.assertTrue(receipt["valid"])
        self.assertFalse(receipt["archive_policy"]["conversation_zeroed"])
        self.assertTrue(receipt["archive_policy"]["journal_preserved"])
        self.assertTrue(receipt["archive_policy"]["can_resume"])
        self.assertTrue(receipt["next_availability"]["subagent_resumable"])

    def test_close_trigger_types_accepted(self) -> None:
        for trigger in ("main_agent", "budget_exhausted", "scope_completed", "main_agent_abort"):
            receipt = build_retrieval_session_close_receipt(
                parent_session_id=PARENT_SESSION_ID,
                subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
                contract_id="RET-CTR-TEST-001",
                result_id="RET-RES-TEST-001",
                triggered_by=trigger,
                reason=f"Triggered by {trigger}.",
            )
            self.assertEqual(receipt["close_trigger"]["triggered_by"], trigger)

    def test_invalid_close_trigger_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "close trigger"):
            build_retrieval_session_close_receipt(
                parent_session_id=PARENT_SESSION_ID,
                subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
                contract_id="RET-CTR-TEST-001",
                result_id="RET-RES-TEST-001",
                triggered_by="unknown_trigger",
            )


class RetrievalSubagentFixtureTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.output_root = Path(self.temp_dir.name) / "subagent-output"

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def test_fixture_writes_and_verifies(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Test retrieval question.",
            allowed_source_categories=["academic_paper"],
        )
        summary = run_retrieval_subagent_fixture(
            contract=contract,
            parent_session_id=PARENT_SESSION_ID,
            output_root=self.output_root,
            query_texts=["test query"],
            source_entries=[
                {
                    "source_id": "SRC-TEST-001",
                    "source_title": "Test Paper",
                    "source_url_or_ref": "https://example.com/test",
                    "visibility": "full_text_observed",
                    "relevance": "direct",
                    "full_text_retrieved": True,
                    "notes": "test source",
                },
            ],
        )
        self.assertTrue(summary["valid"])
        self.assertTrue(summary["checks"]["close_receipt_confirms_no_zeroing"])

        verified = verify_retrieval_subagent_fixture(
            output_root=self.output_root,
            parent_session_id=PARENT_SESSION_ID,
        )
        self.assertTrue(verified["valid"])

    def test_fixture_refuses_nonempty_output_root(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Test.",
            allowed_source_categories=["web_page"],
        )
        self.output_root.mkdir()
        (self.output_root / "existing.txt").write_text("occupied\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "empty or absent"):
            run_retrieval_subagent_fixture(
                contract=contract,
                parent_session_id=PARENT_SESSION_ID,
                output_root=self.output_root,
            )

    def test_verifier_detects_result_tamper(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Test.",
            allowed_source_categories=["academic_paper"],
        )
        run_retrieval_subagent_fixture(
            contract=contract,
            parent_session_id=PARENT_SESSION_ID,
            output_root=self.output_root,
        )
        result_path = self.output_root / "retrieval-result.json"
        import json
        result = json.loads(result_path.read_text(encoding="utf-8"))
        result["contract_id"] = "RET-CTR-TAMPERED"
        result_path.write_text(
            json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_retrieval_subagent_fixture(
                output_root=self.output_root,
                parent_session_id=PARENT_SESSION_ID,
            )

    def test_verifier_detects_close_receipt_tamper(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Test.",
            allowed_source_categories=["academic_paper"],
        )
        run_retrieval_subagent_fixture(
            contract=contract,
            parent_session_id=PARENT_SESSION_ID,
            output_root=self.output_root,
        )
        close_path = self.output_root / "retrieval-session-close-receipt.json"
        import json
        close_data = json.loads(close_path.read_text(encoding="utf-8"))
        close_data["archive_policy"]["conversation_zeroed"] = True
        close_path.write_text(
            json.dumps(close_data, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_retrieval_subagent_fixture(
                output_root=self.output_root,
                parent_session_id=PARENT_SESSION_ID,
            )

    def test_verify_retrieval_result_sources_alias(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Test.",
            allowed_source_categories=["academic_paper"],
        )
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            source_entries=[
                {
                    "source_id": "SRC-TEST-001",
                    "source_title": "Test Paper",
                    "source_url_or_ref": "https://example.com/test",
                    "visibility": "full_text_observed",
                    "relevance": "direct",
                    "full_text_retrieved": True,
                    "notes": "test",
                },
            ],
        )
        receipt = verify_retrieval_result_sources(contract=contract, result=result)
        self.assertTrue(receipt["valid"])

    def test_result_without_source_entries_still_validates(self) -> None:
        contract = build_retrieval_task_contract(
            parent_session_id=PARENT_SESSION_ID,
            retrieval_question="Find something that may not exist.",
            allowed_source_categories=["academic_paper"],
            required_visibility="metadata_only",
        )
        result = build_fake_retrieval_result(
            contract=contract,
            subagent_session_id="CONV-BBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCC",
            query_texts=["nonexistent topic"],
            source_entries=[],
        )
        receipt = validate_retrieval_result(contract=contract, result=result)
        self.assertTrue(receipt["valid"])
