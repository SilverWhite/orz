from __future__ import annotations

import unittest

from assurance import (
    build_orientation_checkpoint,
    build_tool_availability_context_block,
    build_tool_availability_gate_receipt,
    build_tool_availability_infused_orientation_block,
    build_tool_availability_infused_orientation_checkpoint,
    evaluate_runtime_stagnation_guard,
    evaluate_tool_belief_mismatch,
    evaluate_tool_belief_stagnation,
    probe_tool_availability,
    verify_orientation_response,
)
from assurance.errors import AssuranceError


TASK_ID = "TASK-TOOL-AVAIL-001"
TASK_CONTRACT_SHA256 = "a" * 64


def _sample_tool_specs() -> list[dict]:
    return [
        {
            "tool_id": "search",
            "tool_name": "Web Search",
            "capability": "search",
            "probe_method": "runtime_tool_registry_check",
        },
        {
            "tool_id": "file_read",
            "tool_name": "File Read",
            "capability": "file_read",
            "probe_method": "runtime_tool_registry_check",
        },
        {
            "tool_id": "file_write",
            "tool_name": "File Write",
            "capability": "file_write",
            "probe_method": "runtime_tool_registry_check",
        },
        {
            "tool_id": "bash_exec",
            "tool_name": "Bash Execution",
            "capability": "bash_exec",
            "probe_method": "runtime_tool_registry_check",
        },
    ]


class ToolAvailabilityGateTests(unittest.TestCase):
    def test_probe_returns_available_tools_from_registry(self) -> None:
        probe_registry = {"search": True, "file_read": True, "file_write": False, "bash_exec": None}
        report = probe_tool_availability(
            tool_specs=_sample_tool_specs(),
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )

        self.assertEqual(report["report_kind"], "tool_availability_report")
        self.assertEqual(len(report["available"]), 2)
        self.assertEqual(len(report["unavailable"]), 1)
        self.assertEqual(len(report["unprobed"]), 0)
        self.assertEqual(len(report["degraded"]), 1)
        available_ids = {item["tool_id"] for item in report["available"]}
        self.assertIn("search", available_ids)
        self.assertIn("file_read", available_ids)
        unavailable_ids = {item["tool_id"] for item in report["unavailable"]}
        self.assertIn("file_write", unavailable_ids)
        degraded_ids = {item["tool_id"] for item in report["degraded"]}
        self.assertIn("bash_exec", degraded_ids)

    def test_probe_marks_unregistered_tools_as_unprobed(self) -> None:
        probe_registry = {"search": True}
        report = probe_tool_availability(
            tool_specs=_sample_tool_specs(),
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )

        self.assertEqual(len(report["unprobed"]), 3)
        self.assertEqual(len(report["available"]), 1)
        unprobed_ids = {item["tool_id"] for item in report["unprobed"]}
        self.assertIn("file_read", unprobed_ids)
        self.assertIn("file_write", unprobed_ids)
        self.assertIn("bash_exec", unprobed_ids)

    def test_context_block_lists_available_and_unavailable(self) -> None:
        probe_registry = {"search": True, "file_read": True, "file_write": False}
        specs = _sample_tool_specs()[:3]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )

        context = build_tool_availability_context_block(report)
        self.assertIn("[TOOL_AVAILABILITY v0.1]", context)
        self.assertIn("AVAILABLE:", context)
        self.assertIn("search", context)
        self.assertIn("file_read", context)
        self.assertIn("UNAVAILABLE:", context)
        self.assertIn("file_write", context)
        self.assertIn("STATUS: descriptive_projection_only", context)
        self.assertIn("ENFORCEMENT:", context)
        self.assertIn("[/TOOL_AVAILABILITY]", context)

    def test_gate_receipt_blocks_on_degraded_tools(self) -> None:
        probe_registry = {"search": None, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        receipt = build_tool_availability_gate_receipt(report)

        self.assertEqual(receipt["decisions"]["gate_decision"], "warn")
        self.assertEqual(receipt["degraded_count"], 1)

    def test_gate_receipt_defers_on_unprobed_tools(self) -> None:
        probe_registry = {"search": True}
        specs = _sample_tool_specs()[:3]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        receipt = build_tool_availability_gate_receipt(report)

        self.assertEqual(receipt["decisions"]["gate_decision"], "defer")
        self.assertEqual(receipt["unprobed_count"], 2)

    def test_gate_receipt_allows_when_all_probed_and_no_degraded(self) -> None:
        probe_registry = {"search": True, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        receipt = build_tool_availability_gate_receipt(report)

        self.assertEqual(receipt["decisions"]["gate_decision"], "allow")
        self.assertEqual(receipt["available_count"], 2)

    def test_invalid_tool_spec_rejected(self) -> None:
        specs = [{"tool_id": "search", "tool_name": "Search", "capability": "invalid_capability", "probe_method": "check"}]
        with self.assertRaisesRegex(AssuranceError, "unknown tool capability"):
            probe_tool_availability(
                tool_specs=specs,
                runtime_id=TASK_ID,
                probe_registry={},
            )

    def test_missing_tool_spec_fields_rejected(self) -> None:
        specs = [{"tool_id": "search"}]
        with self.assertRaisesRegex(AssuranceError, "missing required fields"):
            probe_tool_availability(
                tool_specs=specs,
                runtime_id=TASK_ID,
                probe_registry={},
            )


class ToolBeliefMismatchTests(unittest.TestCase):
    def test_model_claiming_unavailable_tool_detected(self) -> None:
        probe_registry = {"search": False, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        result = evaluate_tool_belief_mismatch(
            model_claimed_available=["search", "file_read"],
            model_claimed_unavailable=[],
            actual_report=report,
        )

        self.assertTrue(result["triggered"])
        self.assertEqual(len(result["mismatches"]), 1)
        self.assertEqual(result["mismatches"][0]["tool_id"], "search")
        self.assertEqual(result["mismatches"][0]["mismatch_type"], "wrong_belief_about_availability")
        self.assertIn("TOOL-BELIEF-AVAILABILITY-MISMATCH", result["reason_codes"])

    def test_model_claiming_available_is_unavailable_mismatch(self) -> None:
        probe_registry = {"search": True, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        result = evaluate_tool_belief_mismatch(
            model_claimed_available=[],
            model_claimed_unavailable=["search"],
            actual_report=report,
        )

        self.assertTrue(result["triggered"])
        self.assertEqual(result["mismatches"][0]["mismatch_type"], "wrong_belief_about_unavailability")

    def test_model_claiming_degraded_tool_detected(self) -> None:
        probe_registry = {"search": None, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        result = evaluate_tool_belief_mismatch(
            model_claimed_available=["search"],
            model_claimed_unavailable=[],
            actual_report=report,
        )

        self.assertTrue(result["triggered"])
        self.assertIn("TOOL-BELIEF-DEGRADED-MISMATCH", result["reason_codes"])

    def test_no_mismatch_when_model_claims_match_reality(self) -> None:
        probe_registry = {"search": True, "file_read": True}
        specs = _sample_tool_specs()[:2]
        report = probe_tool_availability(
            tool_specs=specs,
            runtime_id=TASK_ID,
            probe_registry=probe_registry,
        )
        result = evaluate_tool_belief_mismatch(
            model_claimed_available=["search", "file_read"],
            model_claimed_unavailable=["file_write"],
            actual_report=report,
        )

        self.assertFalse(result["triggered"])
        self.assertEqual(result["mismatch_count"], 0)


class ToolBeliefStagnationTests(unittest.TestCase):
    def test_unavailable_tool_mentioned_triggers_stagnation(self) -> None:
        receipt = evaluate_tool_belief_stagnation(
            public_outputs=[
                "Let me use the search tool to find this information.",
            ],
            available_tool_names=["file_read"],
            unavailable_tool_names=["search"],
        )

        self.assertEqual(receipt["decision"], "restart_requested")
        self.assertIn("TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED", receipt["reason_codes"])

    def test_capability_guessing_detected(self) -> None:
        receipt = evaluate_tool_belief_stagnation(
            public_outputs=[
                "我将搜索这个信息，看看能找到什么。",
            ],
            available_tool_names=[],
            unavailable_tool_names=["web_fetch"],
        )

        self.assertEqual(receipt["decision"], "continue")
        self.assertIn("TOOL-BELIEF-CAPABILITY-GUESSING", receipt["reason_codes"])
        self.assertFalse(receipt["checks"]["no_capability_guessing"])

    def test_capability_guessing_english_detected(self) -> None:
        receipt = evaluate_tool_belief_stagnation(
            public_outputs=[
                "I will use the search tool to look that up.",
            ],
            available_tool_names=[],
            unavailable_tool_names=["search"],
        )

        self.assertEqual(receipt["decision"], "restart_requested")
        self.assertIn("TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED", receipt["reason_codes"])
        self.assertIn("TOOL-BELIEF-CAPABILITY-GUESSING", receipt["reason_codes"])

    def test_valid_available_tool_usage_continues(self) -> None:
        receipt = evaluate_tool_belief_stagnation(
            public_outputs=[
                "Based on my analysis, the configuration looks correct.",
            ],
            available_tool_names=["file_read"],
            unavailable_tool_names=["search"],
        )

        self.assertEqual(receipt["decision"], "continue")
        self.assertEqual(receipt["reason_codes"], [])

    def test_no_tool_mention_at_all_continues(self) -> None:
        receipt = evaluate_tool_belief_stagnation(
            public_outputs=[
                "Based on the current design, the next step is to implement the schema.",
                "I should ensure the validator covers all edge cases.",
            ],
            available_tool_names=["file_read"],
            unavailable_tool_names=["search"],
        )

        self.assertEqual(receipt["decision"], "continue")


class ToolInfusedOrientationCheckpointTests(unittest.TestCase):
    def test_infused_checkpoint_contains_tool_availability(self) -> None:
        context_block = (
            "[TOOL_AVAILABILITY v0.1]\n"
            "AVAILABLE: file_read\n"
            "UNAVAILABLE: search\n"
            "[/TOOL_AVAILABILITY]"
        )
        checkpoint = build_tool_availability_infused_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=5,
            tool_availability_context_block=context_block,
            task_contract_sha256=TASK_CONTRACT_SHA256,
        )

        self.assertIn("[TOOL_AVAILABILITY v0.1]", checkpoint["message_block"])
        self.assertIn("[ORIENTATION_CHECKPOINT v0.1]", checkpoint["message_block"])
        self.assertIn("AVAILABLE: file_read", checkpoint["message_block"])
        self.assertIn("UNAVAILABLE: search", checkpoint["message_block"])

    def test_infused_checkpoint_preserves_neutral_orientation(self) -> None:
        context_block = "[TOOL_AVAILABILITY v0.1]\nAVAILABLE: search\n[/TOOL_AVAILABILITY]"
        checkpoint = build_tool_availability_infused_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=5,
            tool_availability_context_block=context_block,
            task_contract_sha256=TASK_CONTRACT_SHA256,
        )

        self.assertIn("当前正在做什么", checkpoint["message_block"])
        self.assertIn("当前任务定位是什么", checkpoint["message_block"])
        self.assertFalse(checkpoint["claim_policy"]["may_generate_counterexample_candidate"])

        receipt = verify_orientation_response(
            checkpoint=checkpoint,
            response={
                "orientation_summary": "正在确认工具可用性。",
                "current_task_position": "工具门禁集成开发。",
                "next_output_target": "完成 schema 定义。",
                "available_tools_acknowledged": ["search"],
            },
        )
        self.assertTrue(receipt["valid"])

    def test_infused_checkpoint_allows_tool_acknowledgement(self) -> None:
        context_block = "[TOOL_AVAILABILITY v0.1]\nAVAILABLE: file_read\nUNAVAILABLE: search\n[/TOOL_AVAILABILITY]"
        checkpoint = build_tool_availability_infused_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=5,
            tool_availability_context_block=context_block,
        )

        receipt = verify_orientation_response(
            checkpoint=checkpoint,
            response={
                "orientation_summary": "确认工具清单。",
                "current_task_position": "初始化阶段。",
                "next_output_target": "使用可用工具。",
                "available_tools_acknowledged": ["file_read"],
            },
        )
        self.assertTrue(receipt["valid"])
        self.assertIn("available_tools_acknowledged", receipt["allowed_fields_observed"])

    def test_infused_checkpoint_rejects_tool_belief_mismatch_statement(self) -> None:
        context_block = "[TOOL_AVAILABILITY v0.1]\nAVAILABLE: file_read\nUNAVAILABLE: search\n[/TOOL_AVAILABILITY]"
        checkpoint = build_tool_availability_infused_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=5,
            tool_availability_context_block=context_block,
        )

        receipt = verify_orientation_response(
            checkpoint=checkpoint,
            response={
                "orientation_summary": "检查工具状态。",
                "tool_belief_mismatch_statement": "search 应该可用但似乎未开启。",
            },
        )
        self.assertFalse(receipt["valid"])
        self.assertIn("tool_belief_mismatch_statement", receipt["forbidden_fields_observed"])

    def test_orientation_checkpoint_with_tool_availability_sha256(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=10,
            task_contract_sha256=TASK_CONTRACT_SHA256,
            tool_availability_sha256="a" * 64,
        )

        self.assertEqual(checkpoint["tool_availability_sha256"], "a" * 64)
        self.assertIn("[ORIENTATION_CHECKPOINT v0.1]", checkpoint["message_block"])


class IntegrationWithExistingGuardTests(unittest.TestCase):
    def test_stagnation_guard_still_detects_repetition(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=["same visible output"] * 11,
            retry_count=0,
            retry_budget=1,
        )
        self.assertEqual(receipt["decision"], "restart_requested")

    def test_stagnation_guard_still_passes_varied_output(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=[
                "Read the current design document.",
                "Added a neutral checkpoint schema.",
            ],
            retry_count=0,
            retry_budget=1,
            progress_markers=["doc-read", "schema-added"],
        )
        self.assertEqual(receipt["decision"], "continue")

    def test_orientation_checkpoint_still_neutral_without_tool_specs(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=12,
            task_contract_sha256=TASK_CONTRACT_SHA256,
        )
        self.assertFalse(checkpoint["claim_policy"]["may_generate_counterexample_candidate"])
        self.assertIsNone(checkpoint["tool_availability_sha256"])
        self.assertNotIn("TOOL_AVAILABILITY", checkpoint["message_block"])

    def test_orientation_checkpoint_response_still_accepts_basic_fields(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=3,
        )
        receipt = verify_orientation_response(
            checkpoint=checkpoint,
            response={
                "orientation_summary": "正在整理实现边界。",
                "current_task_position": "只读 fixture 设计。",
                "next_output_target": "输出测试覆盖结果。",
            },
        )
        self.assertTrue(receipt["valid"])
        self.assertTrue(receipt["checks"]["neutral_orientation_only"])
