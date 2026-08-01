from __future__ import annotations

import tempfile
from pathlib import Path
import unittest

from assurance import (
    AdapterGateBlockedError,
    AdapterGateContext,
    AssuranceError,
    enforce_adapter_call,
)
from assurance.adapter_preflight import (
    run_adapter_preflight,
    verify_adapter_preflight,
)
from assurance.adapter_output_validator import validate_adapter_output
from assurance.adapter_failure_classifier import (
    classify_adapter_error,
    build_failure_recovery_plan,
)
from assurance.instruction_provenance_gate import (
    build_instruction_provenance_gate_context,
    evaluate_instruction_provenance_gate,
)
from assurance.utils import sha256_bytes


RUN_ID = "RUN-ADAPTER-TEST-001"
CONV_ID = "CONV-ADAPTER-TEST"
ADAPTER_ID = "canonical-cli-real-deepseek-adapter"


def _build_valid_ipg() -> tuple[dict, dict]:
    content_bytes = "Test question".encode("utf-8")
    ctx = build_instruction_provenance_gate_context(
        run_id=RUN_ID,
        conversation_id=CONV_ID,
        instructions=[
            {
                "entry_id": "INS-USER-001",
                "declared_source_type": "user",
                "source_id": "user-prompt-main",
                "content_sha256": sha256_bytes(content_bytes),
                "content_bytes": len(content_bytes),
                "instruction_kind": "user_prompt",
            },
        ],
    )
    receipt = evaluate_instruction_provenance_gate(gate_context=ctx)
    return ctx, receipt


def _valid_answer_packet() -> dict:
    return {
        "schema_version": "0.1.0-draft",
        "packet_kind": "canonical_guarded_cli_answer_packet",
        "run_id": RUN_ID,
        "task_id": "TASK-001",
        "task_contract": {"sha256": "a" * 64, "entry_mode": "ask"},
        "adapter": {
            "adapter_id": ADAPTER_ID,
            "provider": "deepseek",
            "model_id": "deepseek-v4-pro",
            "mode": "real_development",
            "real_network_used": True,
            "tool_calls_used": False,
        },
        "instruction_provenance_gate": {
            "receipt_sha256": "b" * 64,
            "decision": "allow",
            "all_sources_classified": True,
            "no_injection_escalation": True,
        },
        "tool_availability_gate": {
            "receipt_sha256": "c" * 64,
            "decision": "allow",
            "available_count": 1,
            "unavailable_count": 5,
            "context_injected": True,
        },
        "source_visibility_gate": {
            "receipt_sha256": "d" * 64,
            "decision": "allow",
            "must_report_visibility_status": True,
            "reference_decision_count": 1,
        },
        "answer": {
            "summary": ["The cited source supports the observation claim."],
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
        "next_actions": ["Review the source visibility summary."],
        "limitations": ["Test fixture only."],
    }


class AdapterPreflightTests(unittest.TestCase):
    """Tests for adapter preflight readiness checks."""

    def test_preflight_passes_with_valid_config(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            credential_target="FEP-Agent/DeepSeek",
            ipg_receipt_valid=True,
            gate_chain_complete=True,
            output_schema_known=True,
        )
        self.assertTrue(receipt["preflight_passed"])
        self.assertTrue(receipt["checks"]["endpoint_canonicalized"])
        self.assertTrue(receipt["checks"]["credential_readable"])
        self.assertTrue(receipt["checks"]["ipg_receipt_valid"])
        self.assertTrue(receipt["checks"]["gate_chain_complete"])

    def test_preflight_fails_without_ipg(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            ipg_receipt_valid=False,
            gate_chain_complete=False,
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertTrue(any("IPG" in e for e in receipt["errors"]))

    def test_preflight_fails_with_bad_endpoint(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="ftp://evil.com/steal",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            ipg_receipt_valid=True,
            gate_chain_complete=True,
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["endpoint_canonicalized"])

    def test_preflight_verifier_detects_inconsistency(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            ipg_receipt_valid=True,
            gate_chain_complete=True,
            output_schema_known=True,
        )
        verified = verify_adapter_preflight(receipt)
        self.assertTrue(verified["valid"], verified["errors"])

        # Tamper: preflight_passed but add an error
        receipt["preflight_passed"] = True
        receipt["errors"].append("fake error")
        verified2 = verify_adapter_preflight(receipt)
        self.assertFalse(verified2["valid"])

    # ── GAK-07: deprecated model aliases ──

    def test_preflight_rejects_deprecated_deepseek_chat(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-chat",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["model_id_not_deprecated"])
        self.assertTrue(any("deprecated" in e for e in receipt["errors"]))

    def test_preflight_rejects_deprecated_deepseek_reasoner(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-reasoner",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["model_id_not_deprecated"])

    def test_preflight_accepts_current_deepseek_models(self) -> None:
        for model_id in ("deepseek-v4-pro", "deepseek-v4-flash"):
            with self.subTest(model_id=model_id):
                receipt = run_adapter_preflight(
                    adapter_id=ADAPTER_ID,
                    provider="deepseek",
                    model_id=model_id,
                    endpoint="https://api.deepseek.com/chat/completions",
                    conversation_id=CONV_ID,
                    run_id=RUN_ID,
                )
                self.assertTrue(
                    receipt["checks"]["model_id_not_deprecated"],
                    f"{model_id} was rejected as deprecated",
                )

    def test_preflight_deprecated_check_case_insensitive(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="DeepSeek-Chat",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
        )
        self.assertFalse(receipt["checks"]["model_id_not_deprecated"])

    # ── GAK-08: thinking / sampling mutual exclusion ──

    def test_preflight_rejects_thinking_with_temperature(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            thinking_mode=True,
            sampling_params=["temperature", "top_p"],
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["thinking_sampling_exclusive"])
        self.assertTrue(any("thinking" in e.lower() for e in receipt["errors"]))

    def test_preflight_allows_thinking_without_sampling_params(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            thinking_mode=True,
            sampling_params=None,
        )
        self.assertTrue(receipt["checks"]["thinking_sampling_exclusive"])

    def test_preflight_rejects_thinking_with_presence_penalty(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            thinking_mode=True,
            sampling_params=["presence_penalty", "frequency_penalty"],
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["thinking_sampling_exclusive"])

    def test_preflight_thinking_exclusive_ignores_unknown_params(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            thinking_mode=True,
            sampling_params=["max_tokens", "stop"],
        )
        self.assertTrue(receipt["checks"]["thinking_sampling_exclusive"])

    # ── GAK-09: /models capability discovery ──

    def test_preflight_model_probed_available(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            probed_model_available=True,
        )
        self.assertTrue(receipt["checks"]["model_id_known_by_provider"])

    def test_preflight_model_probed_unavailable_fails(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="nonexistent-model-xyz",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            probed_model_available=False,
        )
        self.assertFalse(receipt["preflight_passed"])
        self.assertFalse(receipt["checks"]["model_id_known_by_provider"])
        self.assertTrue(any("not found" in e for e in receipt["errors"]))

    def test_preflight_model_not_probed_key_absent(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            probed_model_available=None,
        )
        self.assertNotIn("model_id_known_by_provider", receipt["checks"])

    # ── verifier cross-checks for new fields ──

    def test_verifier_rejects_deprecated_model_with_pass(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            ipg_receipt_valid=True,
            gate_chain_complete=True,
            output_schema_known=True,
        )
        # Tamper: flip deprecated check but keep passed
        receipt["checks"]["model_id_not_deprecated"] = False
        receipt["preflight_passed"] = True
        verified = verify_adapter_preflight(receipt)
        self.assertFalse(verified["valid"])
        self.assertTrue(any("deprecated" in e for e in verified["errors"]))

    def test_verifier_rejects_thinking_conflict_with_pass(self) -> None:
        receipt = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
        )
        # Tamper: flip thinking check but keep passed
        receipt["checks"]["thinking_sampling_exclusive"] = False
        receipt["preflight_passed"] = True
        verified = verify_adapter_preflight(receipt)
        self.assertFalse(verified["valid"])
        self.assertTrue(any("thinking" in e for e in verified["errors"]))


class AdapterOutputValidatorTests(unittest.TestCase):
    """Tests for structured output validation."""

    def test_valid_output_passes(self) -> None:
        result = validate_adapter_output(
            model_output={"choices": [{"message": {"content": "test"}}]},
            answer_packet=_valid_answer_packet(),
        )
        self.assertTrue(result["valid"], result["errors"])
        self.assertTrue(result["checks"]["schema_conforms"])

    def test_missing_visibility_annotations_detected(self) -> None:
        pkt = _valid_answer_packet()
        pkt["answer"]["source_visibility_summary"] = []
        result = validate_adapter_output(
            model_output={"choices": [{"message": {"content": "test"}}]},
            answer_packet=pkt,
            source_gate_receipt={
                "decision": "allow",
                "reference_decisions": [{"ref_id": "REF-001", "decision": "allow"}],
            },
        )
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["visibility_annotated"])

    def test_credential_leak_detected(self) -> None:
        pkt = _valid_answer_packet()
        pkt["answer"]["summary"] = ["Here is the key: sk-ant-api03-abc123secret"]
        result = validate_adapter_output(
            model_output={"choices": [{"message": {"content": "test"}}]},
            answer_packet=pkt,
        )
        self.assertFalse(result["checks"]["no_credential_leak"])

    def test_raw_internals_leak_detected(self) -> None:
        pkt = _valid_answer_packet()
        pkt["answer"]["raw_response"] = "leaked raw bytes"
        result = validate_adapter_output(
            model_output={"choices": [{"message": {"content": "test"}}]},
            answer_packet=pkt,
        )
        self.assertFalse(result["checks"]["no_raw_internals_leak"])

    def test_claim_boundary_blocked_respected(self) -> None:
        pkt = _valid_answer_packet()
        pkt["claim_boundaries"]["scientific_claim_strength"] = "mechanism"
        result = validate_adapter_output(
            model_output={"choices": [{"message": {"content": "test"}}]},
            answer_packet=pkt,
            source_gate_receipt={"decision": "block", "reference_decisions": []},
        )
        self.assertFalse(result["valid"])
        self.assertFalse(result["checks"]["source_gate_respected"])


class AdapterFailureClassifierTests(unittest.TestCase):
    """Tests for adapter failure classification and recovery planning."""

    def test_classifies_auth_failure(self) -> None:
        result = classify_adapter_error(http_status=401, error_message="Unauthorized")
        self.assertEqual(result["category"], "auth_failure")
        self.assertFalse(result["retryable"])

    def test_classifies_rate_limit(self) -> None:
        result = classify_adapter_error(http_status=429)
        self.assertEqual(result["category"], "rate_limited")
        self.assertTrue(result["retryable"])

    def test_classifies_server_error(self) -> None:
        result = classify_adapter_error(http_status=503)
        self.assertEqual(result["category"], "server_error")
        self.assertTrue(result["retryable"])

    def test_classifies_timeout(self) -> None:
        result = classify_adapter_error(timeout=True, error_message="timed out")
        self.assertEqual(result["category"], "network_timeout")
        self.assertTrue(result["retryable"])

    def test_classifies_connection_error(self) -> None:
        result = classify_adapter_error(
            connection_error=True, error_message="Connection refused"
        )
        self.assertEqual(result["category"], "network_unreachable")
        self.assertFalse(result["retryable"])

    def test_classifies_output_validation_failure(self) -> None:
        result = classify_adapter_error(output_validation_failed=True)
        self.assertEqual(result["category"], "invalid_output")
        self.assertTrue(result["retryable"])

    def test_recovery_plan_respects_max_retries(self) -> None:
        classification = classify_adapter_error(http_status=429)
        plan = build_failure_recovery_plan(
            classification, max_retries=3, current_retry=3
        )
        self.assertFalse(plan["should_retry"])

    def test_recovery_plan_backoff_increases(self) -> None:
        classification = classify_adapter_error(http_status=429)
        plan1 = build_failure_recovery_plan(
            classification, max_retries=5, current_retry=0
        )
        plan2 = build_failure_recovery_plan(
            classification, max_retries=5, current_retry=2
        )
        self.assertTrue(plan1["should_retry"])
        self.assertTrue(plan2["should_retry"])
        self.assertGreater(plan2["retry_delay_seconds"], plan1["retry_delay_seconds"])

    def test_non_retryable_does_not_retry(self) -> None:
        classification = classify_adapter_error(http_status=401)
        plan = build_failure_recovery_plan(classification)
        self.assertFalse(plan["should_retry"])


class AdapterEndToEndGateIntegrationTests(unittest.TestCase):
    """End-to-end: preflight → gate enforce → adapter call → output validate."""

    def test_full_gate_chain_for_real_adapter(self) -> None:
        """Simulate the complete production path for a real adapter call."""
        ctx, ipg_receipt = _build_valid_ipg()

        # Step 1: Preflight
        preflight = run_adapter_preflight(
            adapter_id=ADAPTER_ID,
            provider="deepseek",
            model_id="deepseek-v4-pro",
            endpoint="https://api.deepseek.com/chat/completions",
            conversation_id=CONV_ID,
            run_id=RUN_ID,
            credential_target="FEP-Agent/DeepSeek",
            ipg_receipt_valid=True,
            gate_chain_complete=True,
            output_schema_known=True,
        )
        self.assertTrue(preflight["preflight_passed"])

        # Step 2: Gate enforcement
        gate_ctx = AdapterGateContext(
            ipg_receipt=ipg_receipt,
            ipg_context=ctx,
            adapter_id=ADAPTER_ID,
            conversation_id=CONV_ID,
            run_id=RUN_ID,
        )
        result = enforce_adapter_call(
            gate_context=gate_ctx,
            adapter_call=lambda: {"choices": [{"message": {"content": "test response"}}]},
        )
        self.assertTrue(result["adapter_call_allowed"])

        # Step 3: Output validation
        pkt = _valid_answer_packet()
        validation = validate_adapter_output(
            model_output=result["adapter_result"],
            answer_packet=pkt,
        )
        self.assertTrue(validation["valid"], validation["errors"])

        # Step 4: If step 3 failed, classify and plan recovery
        if not validation["valid"]:
            classification = classify_adapter_error(output_validation_failed=True)
            self.assertEqual(classification["category"], "invalid_output")
            plan = build_failure_recovery_plan(classification, max_retries=3, current_retry=0)
            self.assertTrue(plan["should_retry"])


if __name__ == "__main__":
    unittest.main()
