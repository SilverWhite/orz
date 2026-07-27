from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    run_orientation_stagnation_integration_fixture,
    verify_orientation_stagnation_runtime_journal,
    write_orientation_stagnation_runtime_journal,
)
from assurance.utils import atomic_write_json


ROOT = Path(__file__).resolve().parents[2]
TASK_ID = "TASK-ORIENTATION-JOURNAL-001"
TASK_CONTRACT_SHA256 = "c" * 64


def _fixture(*, repeated: bool = True) -> dict[str, object]:
    return {
        "schema_version": "0.1.0-draft",
        "fixture_kind": "orientation_stagnation_integration_fixture",
        "task_id": TASK_ID,
        "task_contract_sha256": TASK_CONTRACT_SHA256,
        "step_index": 21,
        "public_outputs": (
            ["same visible output"] * 11
            if repeated
            else [
                "Read the user request.",
                "Built a neutral checkpoint.",
                "Recorded a non-stagnating journal event.",
            ]
        ),
        "retry_count": 0,
        "retry_budget": 1,
        "thresholds": {
            "repeated_content_threshold": 10,
            "ngram_repeat_threshold": 10,
        },
        "progress_markers": ["fixture-built"],
    }


class OrientationRuntimeJournalTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.fixture_path = self.root / "fixture.json"
        self.integration_root = self.root / "integration"
        self.journal_root = self.root / "journal"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _write_integration(self, fixture: dict[str, object] | None = None) -> None:
        atomic_write_json(self.fixture_path, fixture or _fixture())
        run_orientation_stagnation_integration_fixture(
            fixture_path=self.fixture_path,
            output_root=self.integration_root,
        )

    def _read_events(self) -> list[dict[str, object]]:
        return [
            json.loads(line)
            for line in (self.journal_root / "events.jsonl")
            .read_text(encoding="utf-8")
            .splitlines()
        ]

    def test_restart_projection_writes_invalidated_journal_and_verifies(self) -> None:
        self._write_integration()

        receipt = write_orientation_stagnation_runtime_journal(
            integration_root=self.integration_root,
            journal_root=self.journal_root,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(
            receipt["event_types"],
            [
                "run_preflight",
                "run_started",
                "orientation_checkpoint",
                "runtime_stagnation_guard",
                "run_invalidated",
            ],
        )
        self.assertTrue(receipt["checks"]["orientation_before_stagnation"])
        self.assertTrue(receipt["checks"]["payload_schemas_valid"])
        self.assertFalse(receipt["checks"]["runner_attached"])
        self.assertFalse(receipt["checks"]["counterexample_queue_invoked"])

        verification = verify_orientation_stagnation_runtime_journal(
            integration_root=self.integration_root,
            journal_root=self.journal_root,
        )
        self.assertEqual(verification, receipt)

    def test_continue_projection_finishes_journal(self) -> None:
        self._write_integration(_fixture(repeated=False))

        receipt = write_orientation_stagnation_runtime_journal(
            integration_root=self.integration_root,
            journal_root=self.journal_root,
        )

        self.assertEqual(receipt["terminal_event"], "run_finished")
        self.assertEqual(receipt["event_types"][-1], "run_finished")

    def test_verifier_detects_event_order_tamper(self) -> None:
        self._write_integration()
        write_orientation_stagnation_runtime_journal(
            integration_root=self.integration_root,
            journal_root=self.journal_root,
        )
        journal_path = self.journal_root / "events.jsonl"
        events = self._read_events()
        events[2], events[3] = events[3], events[2]
        journal_path.write_text(
            "\n".join(
                json.dumps(event, sort_keys=True, separators=(",", ":"))
                for event in events
            )
            + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_orientation_stagnation_runtime_journal(
                integration_root=self.integration_root,
                journal_root=self.journal_root,
            )

    def test_verifier_detects_payload_tamper(self) -> None:
        self._write_integration()
        write_orientation_stagnation_runtime_journal(
            integration_root=self.integration_root,
            journal_root=self.journal_root,
        )
        journal_path = self.journal_root / "events.jsonl"
        events = self._read_events()
        events[3]["payload"]["asks_model_if_stuck"] = True
        journal_path.write_text(
            "\n".join(
                json.dumps(event, sort_keys=True, separators=(",", ":"))
                for event in events
            )
            + "\n",
            encoding="utf-8",
        )

        with self.assertRaises(AssuranceError):
            verify_orientation_stagnation_runtime_journal(
                integration_root=self.integration_root,
                journal_root=self.journal_root,
            )

    def test_refuses_nonempty_journal_root(self) -> None:
        self._write_integration()
        self.journal_root.mkdir()
        (self.journal_root / "existing.txt").write_text("occupied\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "empty or absent"):
            write_orientation_stagnation_runtime_journal(
                integration_root=self.integration_root,
                journal_root=self.journal_root,
            )
