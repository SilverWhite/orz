from __future__ import annotations

import unittest

from assurance import (
    build_orientation_checkpoint,
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

