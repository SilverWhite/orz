from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    extract_public_outputs_from_runner_stream,
    run_runner_public_output_extraction_fixture,
    verify_runner_public_output_extraction_fixture,
)
from assurance.utils import atomic_write_json


ROOT = Path(__file__).resolve().parents[2]
TASK_ID = "TASK-PUBLIC-OUTPUT-001"
TASK_CONTRACT_SHA256 = "d" * 64


def _stream() -> dict[str, object]:
    return {
        "schema_version": "0.1.0-draft",
        "stream_kind": "runner_public_output_stream_fixture",
        "stream_id": "STREAM-PUBLIC-OUTPUT-001",
        "records": [
            {
                "record_id": "REC-PUBLIC-000",
                "sequence": 0,
                "source": "assistant",
                "channel": "assistant_delta",
                "visibility": "public",
                "text": "same visible output",
            },
            {
                "record_id": "REC-PRIVATE-001",
                "sequence": 1,
                "source": "assistant",
                "channel": "reasoning_private",
                "visibility": "private_hidden",
                "content_sha256": "1" * 64,
            },
            {
                "record_id": "REC-USER-002",
                "sequence": 2,
                "source": "user",
                "channel": "user_input",
                "visibility": "public",
                "text": "Please continue.",
            },
            {
                "record_id": "REC-PUBLIC-003",
                "sequence": 3,
                "source": "assistant",
                "channel": "assistant_final",
                "visibility": "public",
                "text": "same visible output",
            },
        ],
        "extraction_policy": {
            "public_assistant_channels": ["assistant_delta", "assistant_final"],
            "private_channels_forbidden": True,
            "restart_packet_source_policy": {
                "included_state": [
                    "task_contract",
                    "verified_artifact_ledger",
                    "unresolved_questions",
                    "last_valid_checkpoint_digest",
                ],
                "runaway_suffix_allowed": False,
                "hidden_reasoning_allowed": False,
            },
        },
    }


class RunnerPublicOutputTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.stream_path = self.root / "stream.json"
        self.output_root = self.root / "public-output"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def _write_stream(self, stream: dict[str, object] | None = None) -> None:
        atomic_write_json(self.stream_path, stream or _stream())

    def test_extracts_only_public_assistant_outputs(self) -> None:
        receipt = extract_public_outputs_from_runner_stream(_stream())

        self.assertEqual(
            receipt["public_outputs"],
            ["same visible output", "same visible output"],
        )
        self.assertEqual(receipt["included_record_ids"], ["REC-PUBLIC-000", "REC-PUBLIC-003"])
        self.assertIn("REC-PRIVATE-001", receipt["excluded_record_ids"])
        self.assertIn("REC-USER-002", receipt["excluded_record_ids"])
        self.assertEqual(receipt["counts"]["private_hidden_excluded_count"], 1)
        self.assertEqual(receipt["counts"]["nonassistant_excluded_count"], 1)
        self.assertFalse(receipt["checks"]["hidden_chain_of_thought_saved"])
        self.assertFalse(receipt["restart_packet_source_policy"]["hidden_reasoning_allowed"])

    def test_fixture_writes_and_verifies_receipt(self) -> None:
        self._write_stream()

        receipt = run_runner_public_output_extraction_fixture(
            stream_path=self.stream_path,
            output_root=self.output_root,
        )

        self.assertTrue((self.output_root / "runner-output-stream.json").is_file())
        self.assertTrue(
            (self.output_root / "runner-public-output-extraction-receipt.json").is_file()
        )
        verification = verify_runner_public_output_extraction_fixture(
            output_root=self.output_root,
        )
        self.assertEqual(verification, receipt)

    def test_private_record_cannot_store_text(self) -> None:
        stream = _stream()
        stream["records"][1]["text"] = "private reasoning text must not be saved"

        with self.assertRaises(AssuranceError):
            extract_public_outputs_from_runner_stream(stream)

    def test_sequence_gap_is_rejected(self) -> None:
        stream = _stream()
        stream["records"][3]["sequence"] = 8

        with self.assertRaisesRegex(AssuranceError, "contiguous"):
            extract_public_outputs_from_runner_stream(stream)

    def test_verifier_detects_receipt_tamper(self) -> None:
        self._write_stream()
        run_runner_public_output_extraction_fixture(
            stream_path=self.stream_path,
            output_root=self.output_root,
        )
        receipt_path = self.output_root / "runner-public-output-extraction-receipt.json"
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        receipt["public_outputs"].append("tampered")
        receipt_path.write_text(
            json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
            encoding="utf-8",
        )

        with self.assertRaisesRegex(AssuranceError, "does not rebuild"):
            verify_runner_public_output_extraction_fixture(output_root=self.output_root)

    def test_refuses_nonempty_output_root(self) -> None:
        self._write_stream()
        self.output_root.mkdir()
        (self.output_root / "existing.txt").write_text("occupied\n", encoding="utf-8")

        with self.assertRaisesRegex(AssuranceError, "empty or absent"):
            run_runner_public_output_extraction_fixture(
                stream_path=self.stream_path,
                output_root=self.output_root,
            )
