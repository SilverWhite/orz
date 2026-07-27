"""Tests for assurance.deepseek_adapter — real DeepSeek API adapter.

Covers: build_real_deepseek_context, build_real_deepseek_answer_packet,
call_deepseek_api (mocked HTTP), and _read_windows_credential error paths.
"""
from __future__ import annotations

import json
import os
import unittest
import urllib.error
import urllib.request
from io import BytesIO
from pathlib import Path
from unittest.mock import MagicMock, patch, Mock

from assurance.deepseek_adapter import (
    _read_windows_credential,
    build_real_deepseek_answer_packet,
    build_real_deepseek_context,
    call_deepseek_api,
    DEFAULT_CREDENTIAL_TARGET,
    DEFAULT_MODEL,
    DEEPSEEK_ENDPOINT,
)
from assurance.errors import AssuranceError


ROOT = Path(__file__).resolve().parents[2]
SOURCE_LEDGER = ROOT / "assurance/fixtures/source_visibility/mixed-visibility-ledger.json"


def _make_task_contract() -> dict:
    """Minimal valid task contract matching the canonical CLI contract shape."""
    return {
        "schema_version": "0.1.0-draft",
        "contract_kind": "canonical_cli_task_contract",
        "task_id": "TASK-TEST-DEEPSEEK-001",
        "created_at": "2026-07-27T12:00:00Z",
        "entry_mode": "ask",
        "user_request": {
            "raw_text": "What is the mechanism of action described in S2?",
            "normalized_intent": "What is the mechanism of action described in S2?",
        },
        "source_ledger": {
            "path": str(SOURCE_LEDGER),
            "sha256": "a" * 64,
            "required": True,
        },
        "output_contract": {
            "format": "canonical_cli_answer_packet",
            "must_include_source_visibility_summary": True,
            "must_include_claim_boundaries": True,
            "must_include_next_actions": True,
        },
        "permissions": {
            "network_allowed": True,
            "real_model_allowed": True,
            "tool_calls_allowed": False,
            "workspace_writes_allowed": False,
            "incremental_retrieval_allowed": False,
        },
        "claim_policy": {
            "allow_bibliographic_claims_without_fulltext": True,
            "require_fulltext_for_mechanism": True,
            "require_fulltext_for_methods": True,
        },
    }


def _make_source_gate_receipt(*, decision: str = "allow") -> dict:
    """Source visibility gate receipt with varying decisions."""
    return {
        "receipt_kind": "source_fulltext_visibility_gate_receipt",
        "decision": decision,
        "reference_count": 3,
        "reference_decisions": [
            {
                "ref_id": "S1",
                "observed_visibility": "metadata_only",
                "required_visibility": "metadata_only",
                "decision": "allow",
                "claim_allowed": "metadata_only",
                "claim_type": "bibliographic_presence",
            },
            {
                "ref_id": "S2",
                "observed_visibility": "full_text_observed",
                "required_visibility": "full_text_observed",
                "decision": "allow",
                "claim_allowed": "full_text_claim",
                "claim_type": "mechanism_claim",
            },
            {
                "ref_id": "S3",
                "observed_visibility": "partial_text_observed",
                "required_visibility": "full_text_observed",
                "decision": "defer",
                "claim_allowed": "observed_fragment_only",
                "claim_type": "methods_claim",
            },
        ],
        "source_count": 3,
        "reference_count_by_visibility": {
            "metadata_only": 1,
            "partial_text_observed": 1,
            "full_text_observed": 1,
        },
        "context_tokens_estimate": 1500,
    }


def _make_ipg_receipt() -> dict:
    """Instruction provenance gate receipt."""
    return {
        "receipt_kind": "instruction_provenance_gate_receipt",
        "gate_decision": "allow",
        "checks": {
            "all_sources_classified": True,
            "no_injection_escalation": True,
        },
    }


def _make_tool_availability_report() -> dict:
    """Tool availability report with some tools available."""
    return {
        "available": ["search", "file_read", "file_write"],
        "unavailable": ["bash_exec", "web_fetch", "subagent"],
        "unprobed": [],
        "degraded": [],
    }


def _make_tool_availability_receipt() -> dict:
    """Tool availability gate receipt."""
    return {
        "available_count": 3,
        "unavailable_count": 3,
        "decisions": {
            "gate_decision": "defer",
            "context_injected": True,
        },
    }


def _make_model_output(
    *,
    public_text: str = "The mechanism involves caspase-3 mediated apoptosis.",
    finish_reason: str = "stop",
    prompt_tokens: int = 500,
    completion_tokens: int = 100,
    total_tokens: int = 600,
) -> dict:
    return {
        "public_assistant_text": public_text,
        "finish_reason": finish_reason,
        "usage": {
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "total_tokens": total_tokens,
        },
        "model": DEFAULT_MODEL,
        "private_reasoning_content_sha256": None,
        "http_status_code": 200,
    }


# ---------------------------------------------------------------------------
# build_real_deepseek_context
# ---------------------------------------------------------------------------


class BuildRealDeepSeekContextTests(unittest.TestCase):
    def test_returns_system_and_user_messages(self) -> None:
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=_make_tool_availability_report(),
        )
        self.assertEqual(len(messages), 2)
        self.assertEqual(messages[0]["role"], "system")
        self.assertEqual(messages[1]["role"], "user")

    def test_system_message_contains_tool_availability_block(self) -> None:
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=_make_tool_availability_report(),
        )
        system = messages[0]["content"]
        self.assertIn("[TOOL_AVAILABILITY v0.1]", system)
        self.assertIn("AVAILABLE tools:", system)
        self.assertIn("search", system)
        self.assertIn("UNAVAILABLE tools:", system)
        self.assertIn("bash_exec", system)
        self.assertIn("MUST NOT guess", system)

    def test_system_message_contains_source_visibility_summary(self) -> None:
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=_make_tool_availability_report(),
        )
        system = messages[0]["content"]
        self.assertIn("S1", system)
        self.assertIn("metadata_only", system)
        self.assertIn("S2", system)
        self.assertIn("full_text_observed", system)
        self.assertIn("S3", system)
        self.assertIn("defer", system)

    def test_user_message_is_task_text(self) -> None:
        task = _make_task_contract()
        messages = build_real_deepseek_context(
            task_contract=task,
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=_make_tool_availability_report(),
        )
        self.assertEqual(
            messages[1]["content"], task["user_request"]["raw_text"]
        )

    def test_no_references_produces_placeholder(self) -> None:
        gate = _make_source_gate_receipt()
        gate["reference_decisions"] = []
        gate["reference_count"] = 0
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=gate,
            tool_availability_report=_make_tool_availability_report(),
        )
        system = messages[0]["content"]
        self.assertIn("No source references available", system)

    def test_no_available_tools_marked_none(self) -> None:
        report = _make_tool_availability_report()
        report["available"] = []
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=report,
        )
        self.assertIn("AVAILABLE tools: none", messages[0]["content"])

    def test_system_message_includes_assurance_gate_instructions(self) -> None:
        messages = build_real_deepseek_context(
            task_contract=_make_task_contract(),
            source_gate_receipt=_make_source_gate_receipt(),
            tool_availability_report=_make_tool_availability_report(),
        )
        system = messages[0]["content"]
        self.assertIn("scientific-assurance gate", system)
        self.assertIn("metadata_only", system)
        self.assertIn("mechanism", system)


# ---------------------------------------------------------------------------
# build_real_deepseek_answer_packet
# ---------------------------------------------------------------------------


class BuildRealDeepSeekAnswerPacketTests(unittest.TestCase):
    def setUp(self) -> None:
        self.task = _make_task_contract()
        self.src_gate = _make_source_gate_receipt()
        self.ipg = _make_ipg_receipt()
        self.tool_rec = _make_tool_availability_receipt()
        self.model_out = _make_model_output()

    def test_packet_with_model_output_includes_public_text(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-001",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="a" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(packet["answer"]["summary"][0],
                         "The mechanism involves caspase-3 mediated apoptosis.")
        self.assertEqual(packet["adapter"]["mode"], "real_development")
        self.assertTrue(packet["adapter"]["real_network_used"])

    def test_packet_includes_gate_chain_order(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-002",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="b" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(
            packet["claim_boundaries"]["gate_chain_order"],
            ["instruction_provenance_gate", "tool_availability_gate",
             "source_visibility_gate"],
        )
        self.assertTrue(
            packet["claim_boundaries"]["all_gates_evaluated_before_model"]
        )

    def test_packet_binds_all_gate_receipts(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-003",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="c" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertIsNotNone(packet["source_visibility_gate"]["receipt_sha256"])
        self.assertIsNotNone(packet["instruction_provenance_gate"]["receipt_sha256"])
        self.assertIsNotNone(packet["tool_availability_gate"]["receipt_sha256"])
        self.assertIsNotNone(packet["task_contract"]["sha256"])

    def test_packet_without_model_output_falls_back_to_error(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-004",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="d" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=None,
        )
        self.assertIn("no output content",
                       packet["answer"]["summary"][0].lower())
        self.assertTrue(packet["adapter"]["real_network_used"])

    def test_packet_includes_usage_when_model_output_present(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-005",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="e" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(packet["adapter"]["usage"]["prompt_tokens"], 500)
        self.assertEqual(packet["adapter"]["usage"]["completion_tokens"], 100)
        self.assertEqual(packet["adapter"]["usage"]["total_tokens"], 600)
        self.assertEqual(packet["adapter"]["http_status_code"], 200)
        self.assertEqual(packet["adapter"]["finish_reason"], "stop")

    def test_packet_deferred_claims_match_source_gate(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-006",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="f" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        # S3 has decision=defer, should appear in deferred_claims
        deferred = packet["answer"]["deferred_claims"]
        self.assertTrue(any("S3" in d for d in deferred))

    def test_packet_source_visibility_summary_covers_all_refs(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-007",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="0" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        ref_ids = [
            item["ref_id"]
            for item in packet["answer"]["source_visibility_summary"]
        ]
        self.assertIn("S1", ref_ids)
        self.assertIn("S2", ref_ids)
        self.assertIn("S3", ref_ids)

    def test_packet_claim_strength_is_full_text_when_allow(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-008",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="1" * 64,
            source_gate_receipt=_make_source_gate_receipt(decision="allow"),
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(
            packet["claim_boundaries"]["scientific_claim_strength"],
            "full_text_grounded_but_unvalidated",
        )

    def test_packet_claim_strength_is_metadata_only_when_not_allow(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-009",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="2" * 64,
            source_gate_receipt=_make_source_gate_receipt(decision="defer"),
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(
            packet["claim_boundaries"]["scientific_claim_strength"],
            "metadata_only",
        )

    def test_packet_validates_against_schema(self) -> None:
        """build_real_deepseek_answer_packet calls validate_contract internally
        and will raise if the packet is invalid."""
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-010",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="3" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        self.assertEqual(packet["packet_kind"],
                         "canonical_guarded_cli_answer_packet")

    def test_packet_includes_instruction_provenance_gate_info(self) -> None:
        packet = build_real_deepseek_answer_packet(
            run_id="RUN-TEST-011",
            task_id="TASK-TEST-DEEPSEEK-001",
            task_contract=self.task,
            task_contract_sha256="4" * 64,
            source_gate_receipt=self.src_gate,
            ipg_receipt=self.ipg,
            tool_availability_receipt=self.tool_rec,
            model_output=self.model_out,
        )
        ipg = packet["instruction_provenance_gate"]
        self.assertEqual(ipg["decision"], "allow")
        self.assertTrue(ipg["all_sources_classified"])
        self.assertTrue(ipg["no_injection_escalation"])


# ---------------------------------------------------------------------------
# call_deepseek_api (mocked HTTP)
# ---------------------------------------------------------------------------


class CallDeepSeekApiMockedTests(unittest.TestCase):
    def _make_response(self, status: int, body: dict) -> MagicMock:
        raw_bytes = json.dumps(body).encode("utf-8")
        mock_resp = MagicMock()
        mock_resp.status = status
        mock_resp.read.return_value = raw_bytes
        mock_resp.__enter__.return_value = mock_resp
        mock_resp.__exit__.return_value = False
        return mock_resp

    def test_successful_call_returns_public_text(self) -> None:
        body = {
            "choices": [
                {
                    "message": {
                        "content": "  Caspase-3 is the primary executioner caspase.  ",
                    },
                    "finish_reason": "stop",
                }
            ],
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150,
            },
            "model": "deepseek-v4-pro",
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            result = call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Hello"}],
            )
        self.assertEqual(
            result["public_assistant_text"],
            "Caspase-3 is the primary executioner caspase.",
        )
        self.assertEqual(result["finish_reason"], "stop")
        self.assertEqual(result["usage"]["prompt_tokens"], 100)
        self.assertEqual(result["http_status_code"], 200)

    def test_strips_whitespace_from_content(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "\n\n  Answer text.  \n"},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
            "model": "deepseek-v4-pro",
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            result = call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Q"}],
            )
        self.assertEqual(result["public_assistant_text"], "Answer text.")

    def test_empty_content_raises_assurance_error(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": ""},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
            "model": "deepseek-v4-pro",
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            with self.assertRaises(AssuranceError) as ctx:
                call_deepseek_api(
                    "sk-test-key-12345678",
                    [{"role": "user", "content": "Q"}],
                )
            self.assertIn("empty public content", str(ctx.exception))

    def test_whitespace_only_content_raises(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "   \t\n  "},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0},
            "model": "deepseek-v4-pro",
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            with self.assertRaises(AssuranceError):
                call_deepseek_api(
                    "sk-test-key-12345678",
                    [{"role": "user", "content": "Q"}],
                )

    def test_http_error_raises_assurance_error(self) -> None:
        mock_error = urllib.error.HTTPError(
            url=DEEPSEEK_ENDPOINT,
            code=401,
            msg="Unauthorized",
            hdrs=MagicMock(),
            fp=BytesIO(b"{}"),
        )
        with patch.object(urllib.request, "urlopen", side_effect=mock_error):
            with self.assertRaises(AssuranceError) as ctx:
                call_deepseek_api(
                    "sk-bad-key-12345678",
                    [{"role": "user", "content": "Q"}],
                )
            self.assertIn("401", str(ctx.exception))

    def test_network_error_raises_assurance_error(self) -> None:
        mock_error = urllib.error.URLError("Connection refused")
        with patch.object(urllib.request, "urlopen", side_effect=mock_error):
            with self.assertRaises(AssuranceError) as ctx:
                call_deepseek_api(
                    "sk-test-key-12345678",
                    [{"role": "user", "content": "Q"}],
                )
            self.assertIn("Connection refused", str(ctx.exception))

    def test_passes_custom_model_and_parameters(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "OK"},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            "model": "deepseek-chat",
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp) as mock_urlopen:
            result = call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Q"}],
                model="deepseek-chat",
                max_tokens=512,
                temperature=0.3,
                timeout_seconds=30,
            )
        # Check that the request was constructed correctly
        call_args = mock_urlopen.call_args
        req = call_args[0][0]
        self.assertIsInstance(req, urllib.request.Request)
        sent_body = json.loads(req.data.decode("utf-8"))
        self.assertEqual(sent_body["model"], "deepseek-chat")
        self.assertEqual(sent_body["max_tokens"], 512)
        self.assertEqual(sent_body["temperature"], 0.3)
        self.assertFalse(sent_body["stream"])
        self.assertEqual(result["model"], "deepseek-chat")

    def test_default_model_is_deepseek_v4_pro(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "OK"},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            "model": DEFAULT_MODEL,
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp) as mock_urlopen:
            call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Q"}],
            )
        req = mock_urlopen.call_args[0][0]
        sent_body = json.loads(req.data.decode("utf-8"))
        self.assertEqual(sent_body["model"], DEFAULT_MODEL)

    def test_records_reasoning_content_sha256_when_present(self) -> None:
        body = {
            "choices": [
                {
                    "message": {
                        "content": "Public answer.",
                        "reasoning_content": "Private chain-of-thought here.",
                    },
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15},
            "model": DEFAULT_MODEL,
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            result = call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Q"}],
            )
        self.assertIsNotNone(result["private_reasoning_content_sha256"])
        self.assertEqual(len(result["private_reasoning_content_sha256"]), 64)

    def test_no_reasoning_content_means_none_sha256(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "Public only."},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            "model": DEFAULT_MODEL,
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp):
            result = call_deepseek_api(
                "sk-test-key-12345678",
                [{"role": "user", "content": "Q"}],
            )
        self.assertIsNone(result["private_reasoning_content_sha256"])

    def test_authorization_header_is_set(self) -> None:
        body = {
            "choices": [
                {
                    "message": {"content": "OK"},
                    "finish_reason": "stop",
                }
            ],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2},
            "model": DEFAULT_MODEL,
        }
        mock_resp = self._make_response(200, body)
        with patch.object(urllib.request, "urlopen", return_value=mock_resp) as mock_urlopen:
            call_deepseek_api(
                "sk-my-api-key-00112233",
                [{"role": "user", "content": "Q"}],
            )
        req = mock_urlopen.call_args[0][0]
        self.assertEqual(req.get_header("Authorization"),
                         "Bearer sk-my-api-key-00112233")
        req_headers = dict(req.header_items())
        self.assertIn("application/json",
                       req_headers.get("Content-type", ""))


# ---------------------------------------------------------------------------
# _read_windows_credential (error paths)
# ---------------------------------------------------------------------------


@unittest.skipUnless(os.name == "nt", "Credential Manager tests require Windows")
class ReadWindowsCredentialWindowsTests(unittest.TestCase):
    def test_missing_credential_raises_assurance_error(self) -> None:
        with self.assertRaises(AssuranceError) as ctx:
            _read_windows_credential("GSA-TEST-NONEXISTENT-TARGET-00000")
        self.assertIn("Cannot read Windows credential", str(ctx.exception))


class ReadWindowsCredentialPlatformTests(unittest.TestCase):
    @unittest.skipIf(os.name == "nt", "tests non-Windows code path")
    def test_non_windows_raises_assurance_error(self) -> None:
        with self.assertRaises(AssuranceError) as ctx:
            _read_windows_credential("any-target")
        self.assertIn("only available on Windows", str(ctx.exception))


if __name__ == "__main__":
    unittest.main()
