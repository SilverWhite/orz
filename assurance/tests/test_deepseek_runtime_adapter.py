from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance.deepseek_runtime_adapter import (
    DeepSeekRunRequest,
    run_deepseek_direct_once,
)


ROOT = Path(__file__).resolve().parents[2]


class DeepSeekRuntimeAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.run_root = Path(self.temporary.name) / "run"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_direct_runtime_writes_receipt_and_normalized_events_without_secret(self) -> None:
        def credential_reader(target: str) -> str:
            self.assertEqual(target, "orz-deepseek/agent-Test")
            return "TEST-DEEPSEEK-RUNTIME-SECRET"

        def api_caller(api_key: str, messages: list[dict[str, str]], **kwargs: object) -> dict:
            self.assertEqual(api_key, "TEST-DEEPSEEK-RUNTIME-SECRET")
            self.assertEqual(messages, [{"role": "user", "content": "Return marker"}])
            self.assertEqual(kwargs["model"], "deepseek-v4-pro")
            return {
                "public_assistant_text": "GSA_ALPHA_REAL_CALL_OK",
                "finish_reason": "stop",
                "usage": {
                    "prompt_tokens": 3,
                    "completion_tokens": 4,
                    "total_tokens": 7,
                },
                "model": "deepseek-v4-pro",
                "private_reasoning_content_sha256": None,
                "http_status_code": 200,
                "network_permit_id": "",
            }

        receipt = run_deepseek_direct_once(
            DeepSeekRunRequest(
                run_root=self.run_root,
                run_id="RUN-DEEPSEEK-DIRECT-TEST-001",
                prompt_text="Return marker",
                credential_target="orz-deepseek/agent-Test",
            ),
            credential_reader=credential_reader,
            api_caller=api_caller,
        )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["receipt_kind"], "deepseek_runtime_adapter_receipt")
        self.assertTrue(receipt["checks"]["credential_read_succeeded"])
        self.assertTrue(receipt["checks"]["http_status_ok"])
        self.assertTrue(receipt["checks"]["no_secret_serialized"])
        receipt_text = (self.run_root / "deepseek-runtime-receipt.json").read_text(
            encoding="utf-8"
        )
        self.assertNotIn("TEST-DEEPSEEK-RUNTIME-SECRET", receipt_text)
        self.assertNotIn("GSA_ALPHA_REAL_CALL_OK", receipt_text)

        events = [
            json.loads(line)
            for line in (self.run_root / "events.jsonl").read_text(
                encoding="utf-8"
            ).splitlines()
            if line.strip()
        ]
        self.assertEqual(
            [event["event_type"] for event in events],
            [
                "run_preflight",
                "run_started",
                "prompt_submitted",
                "model_response_received",
                "model_output",
                "artifact_registered",
                "run_finished",
            ],
        )
        self.assertEqual(events[0]["payload"]["provider"], "deepseek")
        self.assertEqual(events[-1]["payload"]["status"], "completed")


if __name__ == "__main__":
    unittest.main()
