"""GAK-01: DeepSeek thinking continuity — offline verification tests."""

from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance.deepseek_thinking_continuity import (
    EXPECTED_MARKER,
    ThinkingContinuityEvidence,
    build_thinking_continuity_result,
    verify_thinking_continuity,
    _extract_evidence,
    _parse_grok_streaming_output,
)


class ParseStreamingOutputTests(unittest.TestCase):
    """Unit tests for Grok streaming-json parsing."""

    def test_parses_valid_ndjson(self) -> None:
        lines = '\n'.join([
            '{"type":"text","data":"hello"}',
            '{"type":"end","usage":{"total_tokens":100}}',
        ])
        events = _parse_grok_streaming_output(lines)
        self.assertEqual(len(events), 2)
        self.assertEqual(events[0]["type"], "text")
        self.assertEqual(events[1]["type"], "end")

    def test_skips_empty_lines(self) -> None:
        lines = '\n\n{"type":"text","data":"x"}\n\n'
        events = _parse_grok_streaming_output(lines)
        self.assertEqual(len(events), 1)

    def test_skips_invalid_json(self) -> None:
        lines = 'not json\n{"type":"text","data":"valid"}'
        events = _parse_grok_streaming_output(lines)
        self.assertEqual(len(events), 1)

    def test_skips_non_dict(self) -> None:
        lines = '[1,2,3]\n{"type":"text","data":"dict"}'
        events = _parse_grok_streaming_output(lines)
        self.assertEqual(len(events), 1)


class ExtractEvidenceTests(unittest.TestCase):
    """Tests for evidence extraction from Grok output."""

    def test_extracts_marker_and_usage(self) -> None:
        lines = '\n'.join([
            '{"type":"text","data":"Some thinking output "}',
            '{"type":"text","data":"' + EXPECTED_MARKER + '"}',
            json.dumps({
                "type": "end",
                "usage": {
                    "input_tokens": 500,
                    "output_tokens": 200,
                    "reasoning_tokens": 1024,
                    "total_tokens": 1724,
                },
            }),
        ])
        evidence = _extract_evidence(lines)
        self.assertTrue(evidence.marker_matched)
        self.assertEqual(evidence.reasoning_tokens, 1024)
        self.assertEqual(evidence.total_tokens, 1724)

    def test_marker_not_found(self) -> None:
        lines = '\n'.join([
            '{"type":"text","data":"wrong output"}',
            '{"type":"end","usage":{"total_tokens":50}}',
        ])
        evidence = _extract_evidence(lines)
        self.assertFalse(evidence.marker_matched)

    def test_missing_end_event_yields_zeros(self) -> None:
        lines = '{"type":"text","data":"no end event"}'
        evidence = _extract_evidence(lines)
        self.assertEqual(evidence.reasoning_tokens, 0)
        self.assertEqual(evidence.total_tokens, 0)


class BuildResultTests(unittest.TestCase):
    """Tests for result construction and schema validation."""

    def test_builds_valid_real_result(self) -> None:
        evidence = ThinkingContinuityEvidence(
            run_id="RUN-GAK01-TEST",
            provider="deepseek",
            model="deepseek-v4-pro",
            turn_count=2,
            tool_call_count=1,
            tool_completed_count=1,
            input_tokens=500,
            output_tokens=200,
            reasoning_tokens=1024,
            total_tokens=1724,
            marker_matched=True,
            provider_request_count=0,
            first_request_has_reasoning=None,
            second_request_preserved_reasoning=None,
            raw_reasoning_in_artifacts=False,
            credential_leak_hit=False,
            exit_code=0,
            duration_ms=3500,
        )
        result = build_thinking_continuity_result(
            run_id="RUN-GAK01-TEST",
            evidence=evidence,
            fake_provider=False,
        )
        self.assertTrue(result["valid"])
        self.assertEqual(result["result_kind"], "deepseek-thinking-continuity-real")
        self.assertEqual(result["tool_call_count"], 1)
        self.assertEqual(result["usage"]["reasoning_tokens"], 1024)

    def test_builds_valid_fake_result(self) -> None:
        evidence = ThinkingContinuityEvidence(
            run_id="RUN-GAK01-FAKE",
            provider="deepseek",
            model="deepseek-v4-pro",
            turn_count=2,
            tool_call_count=1,
            tool_completed_count=1,
            input_tokens=300,
            output_tokens=100,
            reasoning_tokens=512,
            total_tokens=912,
            marker_matched=True,
            provider_request_count=2,
            first_request_has_reasoning=True,
            second_request_preserved_reasoning=True,
            raw_reasoning_in_artifacts=False,
            credential_leak_hit=False,
            exit_code=0,
            duration_ms=1500,
        )
        result = build_thinking_continuity_result(
            run_id="RUN-GAK01-FAKE",
            evidence=evidence,
            fake_provider=True,
            continuity_raw={
                "raw_reasoning_found_in_artifacts": False,
                "first_request_has_reasoning": True,
                "second_request_preserved_reasoning": True,
                "provider_request_count": 2,
            },
        )
        self.assertTrue(result["valid"])
        self.assertEqual(result["result_kind"], "deepseek-thinking-continuity-fake")
        self.assertTrue(result["continuity"]["second_request_preserved_reasoning"])


class VerifyThinkingContinuityTests(unittest.TestCase):
    """Tests for the verification function."""

    def _valid_result(self, **overrides: object) -> dict:
        evidence = ThinkingContinuityEvidence(
            run_id="RUN-VERIFY",
            provider="deepseek",
            model="deepseek-v4-pro",
            turn_count=2,
            tool_call_count=1,
            tool_completed_count=1,
            input_tokens=500,
            output_tokens=200,
            reasoning_tokens=1024,
            total_tokens=1724,
            marker_matched=True,
            provider_request_count=0,
            first_request_has_reasoning=None,
            second_request_preserved_reasoning=None,
            raw_reasoning_in_artifacts=False,
            credential_leak_hit=False,
            exit_code=0,
            duration_ms=3000,
        )
        result = build_thinking_continuity_result(
            run_id="RUN-VERIFY",
            evidence=evidence,
            fake_provider=False,
        )
        for key, value in overrides.items():
            result[key] = value
        return result

    def test_valid_result_passes(self) -> None:
        verification = verify_thinking_continuity(self._valid_result())
        self.assertTrue(verification["valid"], verification["errors"])

    def test_zero_reasoning_tokens_fails(self) -> None:
        result = self._valid_result()
        result["usage"]["reasoning_tokens"] = 0
        verification = verify_thinking_continuity(result)
        self.assertFalse(verification["valid"])
        self.assertFalse(verification["checks"]["reasoning_tokens_positive"])

    def test_marker_mismatch_fails(self) -> None:
        result = self._valid_result(marker_matched=False)
        verification = verify_thinking_continuity(result)
        self.assertFalse(verification["valid"])
        self.assertFalse(verification["checks"]["marker_matched"])

    def test_raw_reasoning_leak_fails(self) -> None:
        result = self._valid_result()
        result["continuity"]["raw_reasoning_found_in_artifacts"] = True
        verification = verify_thinking_continuity(result)
        self.assertFalse(verification["valid"])
        self.assertFalse(verification["checks"]["reasoning_content_not_leaked"])

    def test_credential_leak_fails(self) -> None:
        result = self._valid_result()
        result["artifact_leak_scan"]["hit_count"] = 1
        verification = verify_thinking_continuity(result)
        self.assertFalse(verification["valid"])
        self.assertFalse(verification["checks"]["no_credential_leak"])

    def test_tool_not_called_when_enabled_fails(self) -> None:
        result = self._valid_result()
        result["tool_call_count"] = 0
        result["tool_completed_count"] = 0
        result["controls"]["tools_enabled"] = True
        verification = verify_thinking_continuity(result)
        self.assertFalse(verification["valid"])
        self.assertFalse(verification["checks"]["tool_called"])

    def test_tools_disabled_skips_tool_checks(self) -> None:
        result = self._valid_result()
        result["tool_call_count"] = 0
        result["tool_completed_count"] = 0
        result["controls"]["tools_enabled"] = False
        verification = verify_thinking_continuity(result)
        # Should still be valid — tool checks are not applicable
        self.assertTrue(verification["checks"]["tool_completed_matches_called"])

    def test_fake_provider_request_log_verifies_continuity(self) -> None:
        result = self._valid_result()
        result["result_kind"] = "deepseek-thinking-continuity-fake"
        result["controls"]["fake_provider"] = True
        result["continuity"]["provider_request_count"] = 2

        with tempfile.TemporaryDirectory() as tmp:
            log_path = Path(tmp) / "provider-requests.json"
            log_path.write_text(json.dumps({
                "requests": [
                    {
                        "messages": [
                            {"role": "user", "content": "read fixture"},
                        ],
                    },
                    {
                        "messages": [
                            {"role": "user", "content": "read fixture"},
                            {"role": "assistant", "reasoning_content": "thinking...", "tool_calls": []},
                            {"role": "tool", "content": "fixture result"},
                        ],
                    },
                ],
            }), encoding="utf-8")
            verification = verify_thinking_continuity(
                result, fake_provider_request_log=log_path,
            )
            self.assertTrue(
                verification["checks"].get("fake_second_request_has_reasoning"),
                verification["errors"],
            )

    def test_fake_provider_missing_reasoning_in_req2_fails(self) -> None:
        result = self._valid_result()
        result["result_kind"] = "deepseek-thinking-continuity-fake"
        result["controls"]["fake_provider"] = True

        with tempfile.TemporaryDirectory() as tmp:
            log_path = Path(tmp) / "provider-requests.json"
            log_path.write_text(json.dumps({
                "requests": [
                    {"messages": [{"role": "user", "content": "q"}]},
                    {"messages": [
                        {"role": "user", "content": "q"},
                        {"role": "assistant", "content": "no reasoning here"},
                        {"role": "tool", "content": "result"},
                    ]},
                ],
            }), encoding="utf-8")
            verification = verify_thinking_continuity(
                result, fake_provider_request_log=log_path,
            )
            self.assertFalse(
                verification["checks"].get("fake_second_request_has_reasoning"),
            )


class SchemaRejectsInvalidTests(unittest.TestCase):
    """Schema-level rejection tests."""

    def test_missing_required_field_rejected(self) -> None:
        result = {
            "schema_version": "0.1.0",
            "result_kind": "deepseek-thinking-continuity-real",
            # missing run_id, created_at, etc.
        }
        from assurance.contracts import validate_contract
        from assurance.errors import AssuranceError
        with self.assertRaises(AssuranceError):
            validate_contract(
                result,
                "deepseek-thinking-continuity-result-v0.1.schema.json",
                label="test",
            )

    def test_wrong_result_kind_rejected(self) -> None:
        evidence = ThinkingContinuityEvidence(
            run_id="R", provider="deepseek", model="deepseek-v4-pro",
            turn_count=1, tool_call_count=0, tool_completed_count=0,
            input_tokens=10, output_tokens=10, reasoning_tokens=5, total_tokens=25,
            marker_matched=True, provider_request_count=0,
            first_request_has_reasoning=None,
            second_request_preserved_reasoning=None,
            raw_reasoning_in_artifacts=False, credential_leak_hit=False,
            exit_code=0, duration_ms=100,
        )
        result = build_thinking_continuity_result(
            run_id="R", evidence=evidence, fake_provider=False,
        )
        # Tamper with result_kind
        result["result_kind"] = "wrong-kind"
        from assurance.contracts import validate_contract
        from assurance.errors import AssuranceError
        with self.assertRaises(AssuranceError):
            validate_contract(
                result,
                "deepseek-thinking-continuity-result-v0.1.schema.json",
                label="test",
            )


if __name__ == "__main__":
    unittest.main()
