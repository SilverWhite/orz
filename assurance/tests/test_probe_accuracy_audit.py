"""Tests for the probe-accuracy audit (ORZ-CACHE-CONTEXT-COST, 2026-08-15)."""

from __future__ import annotations

import unittest

from assurance.probe_accuracy_audit import GATE_ONLY_CODES, audit_probe_accuracy
from assurance.run_event_journal_validation import _WORK_TOOLS as VERIFIER_WORK_TOOLS
from assurance.probe_accuracy_audit import WORK_TOOLS


def _ev(event_type: str, payload: dict) -> dict:
    return {
        "payload_schema": "run-event-v0.2.schema.json",
        "event_type": event_type,
        "payload": payload,
    }


def _probe(complete: list[str], incomplete: list[str] | None = None) -> dict:
    return _ev(
        "tool_availability_check",
        {
            "complete": complete,
            "incomplete": [
                {"tool": t, "reason": "缺少测试运行器"} for t in (incomplete or [])
            ],
        },
    )


def _header(role: str = "main", reason: str = "change") -> dict:
    return _ev(
        "request_header_change",
        {
            "agent_role": role,
            "reason": reason,
            "header_sha256": "h" * 64,
            "tools": ["read_file"],
            "tool_count": 1,
        },
    )


class ProbeAccuracyAuditTests(unittest.TestCase):
    def test_work_tools_match_reference_verifier(self) -> None:
        # Review fix (2026-08-15): the audit module's tool set is
        # single-sourced from the verifier — this guards the import.
        self.assertEqual(WORK_TOOLS, VERIFIER_WORK_TOOLS)

    def test_flip_with_header_change_is_clean(self) -> None:
        events = [
            _probe(["read_file"], ["run_tests"]),
            _probe(["read_file", "grep"], ["run_tests"]),
            _header(),
            _ev("model_output", {"text": "ok"}),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(report["flip_without_header_change"], [])

    def test_flip_without_header_change_is_flagged(self) -> None:
        events = [
            _probe(["read_file"], ["run_tests"]),
            _probe(["read_file", "grep"], ["run_tests"]),
            _ev("model_output", {"text": "ok"}),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(len(report["flips"]), 1)
        self.assertEqual(len(report["flip_without_header_change"]), 1)

    def test_subagent_header_does_not_clear_main_flip(self) -> None:
        events = [
            _probe(["read_file"], []),
            _probe(["read_file", "grep"], []),
            _header(role="external_retrieval"),
            _ev("model_output", {"text": "ok"}),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(len(report["flip_without_header_change"]), 1)

    def test_false_complete_candidate_excludes_gate_codes(self) -> None:
        events = [
            _probe(["read_file"], []),
            _ev(
                "tool_completed",
                {"tool": "read_file", "status": "error", "error": "not found"},
            ),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(
            report["false_complete_candidates"],
            [(1, "read_file", "not found")],
        )

        gate = _ev(
            "tool_completed",
            {"tool": "read_file", "status": "error", "error": "permission_denied"},
        )
        report = audit_probe_accuracy([events[0], gate])
        self.assertEqual(report["false_complete_candidates"], [])
        self.assertIn("permission_denied", GATE_ONLY_CODES)

    def test_fail_closed_ticket_and_budget_codes_are_gate_only(self) -> None:
        # Review fix (2026-08-15): the fail-closed control-ticket prefix
        # (`control_ticket_rejected:<code>`) and the injection-budget
        # refusal are call-time mechanical gates — never probe
        # misjudgments.
        events = [
            _probe(["read_file"], []),
            _ev(
                "tool_completed",
                {
                    "tool": "read_file",
                    "status": "error",
                    "error": "control_ticket_rejected:missing_goal_context",
                },
            ),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(report["false_complete_candidates"], [])

        events = [
            _probe(["read_file"], []),
            _ev(
                "tool_completed",
                {
                    "tool": "read_file",
                    "status": "error",
                    "error": "round_inject_budget_exceeded",
                    "inject_tokens_used": 50_000,
                    "inject_tokens_budget": 50_000,
                },
            ),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(report["false_complete_candidates"], [])
        self.assertIn("round_inject_budget_exceeded", GATE_ONLY_CODES)

    def test_false_incomplete_candidate(self) -> None:
        events = [
            _probe(["read_file"], ["grep"]),
            _ev("tool_started", {"tool": "grep", "call_id": "c"}),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(report["false_incomplete_candidates"], [(1, "grep")])

    def test_retrieval_lane_events_are_ignored(self) -> None:
        events = [
            _probe(["read_file"], []),
            _ev(
                "tool_completed",
                {
                    "tool": "web_search",
                    "target": "external_retrieval",
                    "status": "error",
                    "error": "provider_down",
                },
            ),
        ]
        report = audit_probe_accuracy(events)
        self.assertEqual(report["false_complete_candidates"], [])


if __name__ == "__main__":
    unittest.main()
