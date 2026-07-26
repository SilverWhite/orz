from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    run_orientation_stagnation_integration_fixture,
    verify_orientation_stagnation_integration_fixture,
)
from assurance.utils import atomic_write_json


ROOT = Path(__file__).resolve().parents[2]
TASK_ID = "TASK-ORIENTATION-INTEGRATION-001"
TASK_CONTRACT_SHA256 = "b" * 64


def _fixture(*, repeated: bool = True, retry_count: int = 0) -> dict[str, object]:
    public_outputs = (
        ["same visible output"] * 11
        if repeated
        else [
            "Read the design document.",
            "Generated the orientation checkpoint.",
            "Verified the stagnation receipt.",
        ]
    )
    return {
        "schema_version": "0.1.0-draft",
        "fixture_kind": "orientation_stagnation_integration_fixture",
        "task_id": TASK_ID,
        "task_contract_sha256": TASK_CONTRACT_SHA256,
        "step_index": 16,
        "public_outputs": public_outputs,
        "retry_count": retry_count,
        "retry_budget": 1,
        "thresholds": {
            "repeated_content_threshold": 10,
            "ngram_repeat_threshold": 10,
        },
        "progress_markers": ["task-contract-read", "checkpoint-ready"],
    }


class OrientationRuntimeIntegrationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.fixture_path = self.root / "fixture.json"
        self.output_root = self.root / "integration-output"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _write_fixture(self, fixture: dict[str, object] | None = None) -> None:
        atomic_write_json(self.fixture_path, fixture or _fixture())

    def test_fixture_writes_artifacts_and_verifies(self) -> None:
        self._write_fixture()

        receipt = run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(
            receipt["decisions"]["stagnation_decision"],
            "restart_requested",
        )
        self.assertTrue(receipt["checks"]["orientation_neutral_checkpoint"])
        self.assertTrue(receipt["checks"]["orientation_counterexample_disabled"])
        self.assertTrue(receipt["checks"]["mechanisms_decoupled"])
        self.assertFalse(receipt["checks"]["runner_attached"])
        self.assertFalse(receipt["checks"]["counterexample_queue_invoked"])
        self.assertTrue((self.output_root / "fixture-input.json").is_file())
        self.assertTrue((self.output_root / "orientation-checkpoint.json").is_file())
        self.assertTrue(
            (self.output_root / "runtime-stagnation-guard-receipt.json").is_file()
        )

        verification = verify_orientation_stagnation_integration_fixture(
            output_root=self.output_root,
        )
        self.assertEqual(verification, receipt)

    def test_non_repeated_fixture_continues_without_restart_packet(self) -> None:
        self._write_fixture(_fixture(repeated=False))

        receipt = run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )
        stagnation = json.loads(
            (self.output_root / "runtime-stagnation-guard-receipt.json").read_text(
                encoding="utf-8"
            )
        )

        self.assertEqual(receipt["decisions"]["stagnation_decision"], "continue")
        self.assertEqual(stagnation["action"], "none")
        self.assertIsNone(stagnation["restart_packet"])

    def test_retry_budget_exhaustion_is_handoff(self) -> None:
        self._write_fixture(_fixture(retry_count=1))

        receipt = run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )

        self.assertEqual(
            receipt["decisions"]["stagnation_decision"],
            "handoff_required",
        )
        self.assertEqual(receipt["decisions"]["stagnation_action"], "stop_and_handoff")

    def test_verifier_detects_orientation_tamper(self) -> None:
        self._write_fixture()
        run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )
        orientation_path = self.output_root / "orientation-checkpoint.json"
        orientation = json.loads(orientation_path.read_text(encoding="utf-8"))
        orientation["message_block"] = orientation["message_block"].replace(
            "当前正在做什么？",
            "当前动作是否正确？",
        )
        orientation_path.write_text(
            json.dumps(orientation, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(AssuranceError, "does not rebuild"):
            verify_orientation_stagnation_integration_fixture(
                output_root=self.output_root,
            )

    def test_verifier_detects_receipt_tamper(self) -> None:
        self._write_fixture()
        run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )
        receipt_path = self.output_root / "orientation-stagnation-integration-receipt.json"
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        receipt["checks"]["runner_attached"] = True
        receipt_path.write_text(
            json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_orientation_stagnation_integration_fixture(
                output_root=self.output_root,
            )

    def test_refuses_nonempty_output_root(self) -> None:
        self._write_fixture()
        self.output_root.mkdir()
        (self.output_root / "existing.txt").write_text("occupied\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "empty or absent"):
            run_orientation_stagnation_integration_fixture(
                fixture_path=self.fixture_path,
                output_root=self.output_root,
            )
