from __future__ import annotations

import unittest

from assurance import (
    build_orientation_checkpoint,
    evaluate_runtime_stagnation_guard,
    verify_orientation_response,
)
from assurance.errors import AssuranceError


TASK_ID = "TASK-ORIENTATION-001"
TASK_CONTRACT_SHA256 = "a" * 64


class OrientationCheckpointTests(unittest.TestCase):
    def test_orientation_checkpoint_uses_neutral_block(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=12,
            task_contract_sha256=TASK_CONTRACT_SHA256,
        )

        self.assertIn("[ORIENTATION_CHECKPOINT v0.1]", checkpoint["message_block"])
        self.assertIn("当前正在做什么", checkpoint["message_block"])
        self.assertIn("当前任务定位是什么", checkpoint["message_block"])
        self.assertNotIn("是否正确", checkpoint["message_block"])
        self.assertNotIn("是否偏移", checkpoint["message_block"])
        self.assertFalse(
            checkpoint["claim_policy"]["may_generate_counterexample_candidate"]
        )
        self.assertEqual(checkpoint["claim_policy"]["claim_strength_effect"], "none")

    def test_orientation_response_accepts_only_position_fields(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=12,
            task_contract_sha256=TASK_CONTRACT_SHA256,
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
        self.assertTrue(receipt["checks"]["no_counterexample_candidate"])
        self.assertTrue(receipt["checks"]["no_claim_disposition"])

    def test_orientation_response_rejects_counterexample_or_claim_fields(self) -> None:
        checkpoint = build_orientation_checkpoint(
            task_id=TASK_ID,
            trigger_step=12,
            task_contract_sha256=TASK_CONTRACT_SHA256,
        )

        receipt = verify_orientation_response(
            checkpoint=checkpoint,
            response={
                "orientation_summary": "正在整理实现边界。",
                "counterexample_candidate": "反向审查不应由 orientation 产生。",
                "claim_disposition": "defer",
            },
        )

        self.assertFalse(receipt["valid"])
        self.assertIn("counterexample_candidate", receipt["forbidden_fields_observed"])
        self.assertIn("claim_disposition", receipt["forbidden_fields_observed"])

    def test_orientation_trigger_step_must_be_non_negative(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "non-negative"):
            build_orientation_checkpoint(task_id=TASK_ID, trigger_step=-1)


class RuntimeStagnationGuardTests(unittest.TestCase):
    def test_high_consecutive_repetition_requests_restart(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=["same visible output"] * 11,
            retry_count=0,
            retry_budget=1,
        )

        self.assertEqual(receipt["decision"], "restart_requested")
        self.assertEqual(receipt["action"], "stop_and_restart")
        self.assertIn("STAGNATION-CONSECUTIVE-REPEAT", receipt["reason_codes"])
        self.assertFalse(receipt["restart_packet"]["runaway_suffix_retained"])
        self.assertFalse(receipt["checks"]["hidden_chain_of_thought_saved"])
        self.assertFalse(receipt["checks"]["asks_model_if_stuck"])

    def test_retry_budget_exhaustion_requires_handoff(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=["loop loop loop"] * 11,
            retry_count=1,
            retry_budget=1,
        )

        self.assertEqual(receipt["decision"], "handoff_required")
        self.assertEqual(receipt["action"], "stop_and_handoff")
        self.assertIsNone(receipt["restart_packet"])

    def test_normal_varied_output_continues(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=[
                "Read the current design document.",
                "Added a neutral checkpoint schema.",
                "Ran the fixture verifier.",
            ],
            retry_count=0,
            retry_budget=1,
            progress_markers=["doc-read", "schema-added", "tests-ran"],
        )

        self.assertEqual(receipt["decision"], "continue")
        self.assertEqual(receipt["action"], "none")
        self.assertEqual(receipt["reason_codes"], [])
        self.assertIsNone(receipt["restart_packet"])

    def test_repeated_ngram_requests_restart(self) -> None:
        receipt = evaluate_runtime_stagnation_guard(
            public_outputs=["alpha beta gamma " * 11],
            retry_count=0,
            retry_budget=1,
        )

        self.assertEqual(receipt["decision"], "restart_requested")
        self.assertIn("STAGNATION-NGRAM-REPEAT", receipt["reason_codes"])

    def test_invalid_retry_budget_is_rejected(self) -> None:
        with self.assertRaisesRegex(AssuranceError, "non-negative"):
            evaluate_runtime_stagnation_guard(
                public_outputs=["same visible output"] * 11,
                retry_count=-1,
                retry_budget=1,
            )
