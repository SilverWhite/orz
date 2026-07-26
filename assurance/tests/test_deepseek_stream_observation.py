from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    run_deepseek_stream_observation_fixture,
    verify_deepseek_stream_observation_fixture,
)
from assurance.utils import atomic_write_json


ROOT = Path(__file__).resolve().parents[2]


def _fixture(*, repeated: bool = True) -> dict[str, object]:
    if repeated:
        chunks = [
            {
                "chunk_id": f"CHK-REPEAT-{index:03d}",
                "sequence": index,
                "delta": {"content": "same visible stream delta"},
            }
            for index in range(11)
        ]
    else:
        chunks = [
            {
                "chunk_id": "CHK-PRIVATE-000",
                "sequence": 0,
                "delta": {
                    "role": "assistant",
                    "reasoning_content_sha256": "a" * 64,
                },
            },
            {
                "chunk_id": "CHK-PUBLIC-001",
                "sequence": 1,
                "delta": {"content": "visible answer"},
            },
            {
                "chunk_id": "CHK-DONE-002",
                "sequence": 2,
                "finish_reason": "stop",
                "usage": {
                    "prompt_tokens": 5,
                    "completion_tokens": 3,
                    "total_tokens": 8,
                },
            },
        ]
    return {
        "schema_version": "0.1.0-draft",
        "fixture_kind": "deepseek_stream_observation_fixture",
        "stream_id": "STREAM-DEEPSEEK-STREAM-001",
        "task_id": "TASK-DEEPSEEK-STREAM-001",
        "task_contract_sha256": "d" * 64,
        "step_index": 12,
        "retry_count": 0,
        "retry_budget": 1,
        "thresholds": {
            "repeated_content_threshold": 10,
            "ngram_repeat_threshold": 10,
        },
        "progress_markers": ["deepseek-stream-fixture"],
        "chunks": chunks,
    }


class DeepSeekStreamObservationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.fixture_path = self.root / "fixture.json"
        self.output_root = self.root / "output"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_repeated_public_stream_triggers_restart_projection(self) -> None:
        atomic_write_json(self.fixture_path, _fixture(repeated=True))

        receipt = run_deepseek_stream_observation_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )

        self.assertEqual(receipt["decisions"]["public_output_count"], 11)
        self.assertEqual(receipt["decisions"]["stagnation_decision"], "restart_requested")
        self.assertEqual(receipt["decisions"]["terminal_event_type"], "run_invalidated")
        verification = verify_deepseek_stream_observation_fixture(
            output_root=self.output_root,
        )
        self.assertEqual(verification, receipt)

    def test_private_reasoning_digest_is_excluded_from_public_outputs(self) -> None:
        atomic_write_json(self.fixture_path, _fixture(repeated=False))

        receipt = run_deepseek_stream_observation_fixture(
            fixture_path=self.fixture_path,
            output_root=self.output_root,
        )

        self.assertEqual(receipt["decisions"]["public_output_count"], 1)
        self.assertEqual(receipt["decisions"]["private_hidden_excluded_count"], 2)
        stream = json.loads((self.output_root / "runner-output-stream.input.json").read_text(
            encoding="utf-8"
        ))
        private_records = [
            record
            for record in stream["records"]
            if record["visibility"] == "private_hidden"
        ]
        self.assertEqual(len(private_records), 1)
        self.assertNotIn("text", private_records[0])
        self.assertEqual(private_records[0]["content_sha256"], "a" * 64)

    def test_rejects_noncontiguous_chunks(self) -> None:
        fixture = _fixture(repeated=False)
        fixture["chunks"][1]["sequence"] = 7
        atomic_write_json(self.fixture_path, fixture)

        with self.assertRaisesRegex(AssuranceError, "contiguous"):
            run_deepseek_stream_observation_fixture(
                fixture_path=self.fixture_path,
                output_root=self.output_root,
            )
