from __future__ import annotations

import json
from pathlib import Path
import tempfile
import unittest

from assurance import (
    AssuranceError,
    run_deepseek_api_observation_pipeline,
    verify_deepseek_api_observation_pipeline,
)
from assurance.utils import atomic_write_json, sha256_bytes


ROOT = Path(__file__).resolve().parents[2]
LAUNCHER = ROOT / "scripts" / "invoke_deepseek_public_output_observation.ps1"


def _api_result() -> dict[str, object]:
    public_text = "LIF_DEEPSEEK_PUBLIC_OUTPUT_OBSERVATION_OK"
    return {
        "schema_version": "0.1.0-draft",
        "result_kind": "deepseek_api_observation",
        "valid": True,
        "started_at": "2026-07-26T00:00:00Z",
        "completed_at": "2026-07-26T00:00:01Z",
        "request_count": 1,
        "retry_count": 0,
        "provider": "deepseek",
        "endpoint": "https://api.deepseek.com/chat/completions",
        "model": "deepseek-v4-pro",
        "http_status_code": 200,
        "marker_matched": True,
        "public_assistant_text": public_text,
        "response_content_sha256": sha256_bytes(public_text.encode("utf-8")),
        "private_reasoning_content_sha256": "a" * 64,
        "credential_source": "windows-credential-manager-current-user",
        "credential_target": "FEP-Agent/DeepSeek",
        "credential_value_recorded": False,
        "raw_response_recorded": False,
        "usage": {
            "prompt_tokens": 8,
            "completion_tokens": 9,
            "total_tokens": 17,
        },
        "controls": {
            "max_completion_tokens": 128,
            "temperature": 0,
            "tool_count": 0,
            "stream": False,
            "retry_budget": 0,
            "web_enabled": False,
            "subagents_enabled": False,
            "raw_request_recorded": False,
        },
        "limitations": ["fake schema-valid API observation"],
    }


class DeepSeekApiObservationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.api_result_path = self.root / "api-result.json"
        self.output_root = self.root / "projection"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_pipeline_projects_real_api_shape_to_existing_guard_layers(self) -> None:
        atomic_write_json(self.api_result_path, _api_result())

        receipt = run_deepseek_api_observation_pipeline(
            api_result_path=self.api_result_path,
            output_root=self.output_root,
            task_id="TASK-DEEPSEEK-API-OBSERVATION-001",
            task_contract_sha256="d" * 64,
            step_index=3,
        )

        self.assertEqual(receipt["decisions"]["public_output_count"], 1)
        self.assertEqual(receipt["decisions"]["stagnation_decision"], "continue")
        self.assertTrue(
            (self.output_root / "runner-public-output" / "runner-output-stream.json").is_file()
        )
        self.assertTrue(
            (
                self.output_root
                / "orientation-stagnation"
                / "runtime-stagnation-guard-receipt.json"
            ).is_file()
        )
        self.assertTrue((self.output_root / "runtime-journal" / "events.jsonl").is_file())
        verification = verify_deepseek_api_observation_pipeline(
            output_root=self.output_root,
        )
        self.assertEqual(verification, receipt)

    def test_pipeline_does_not_copy_private_reasoning_text(self) -> None:
        result = _api_result()
        atomic_write_json(self.api_result_path, result)

        run_deepseek_api_observation_pipeline(
            api_result_path=self.api_result_path,
            output_root=self.output_root,
            task_id="TASK-DEEPSEEK-API-OBSERVATION-001",
            task_contract_sha256="d" * 64,
            step_index=3,
        )

        stream = json.loads(
            (self.output_root / "runner-public-output" / "runner-output-stream.json").read_text(
                encoding="utf-8"
            )
        )
        private_records = [
            record
            for record in stream["records"]
            if record["visibility"] == "private_hidden"
        ]
        self.assertEqual(len(private_records), 1)
        self.assertNotIn("text", private_records[0])
        self.assertEqual(private_records[0]["content_sha256"], "a" * 64)

    def test_pipeline_rejects_retry_or_raw_recording(self) -> None:
        result = _api_result()
        result["retry_count"] = 1
        atomic_write_json(self.api_result_path, result)

        with self.assertRaises(AssuranceError):
            run_deepseek_api_observation_pipeline(
                api_result_path=self.api_result_path,
                output_root=self.output_root,
                task_id="TASK-DEEPSEEK-API-OBSERVATION-001",
                task_contract_sha256="d" * 64,
                step_index=3,
            )

    def test_launcher_has_one_request_boundary_and_no_raw_credential_parameter(self) -> None:
        source = LAUNCHER.read_text(encoding="utf-8")
        lowered = source.lower()
        self.assertIn("[ValidateSet('Plan', 'Execute')]", source)
        self.assertIn("$credentialTarget = 'FEP-Agent/DeepSeek'", source)
        self.assertIn("$endpoint = 'https://api.deepseek.com/chat/completions'", source)
        self.assertIn("CredRead(target, CRED_TYPE_GENERIC", source)
        self.assertIn("CredFree(pointer)", source)
        self.assertIn("Marshal.WriteByte(credential.CredentialBlob", source)
        self.assertIn("max_tokens = 128", source)
        self.assertIn("retry_budget = 0", source)
        self.assertIn("raw_response_recorded = $false", source)
        self.assertIn("raw_request_recorded = $false", source)
        self.assertIn("Assert-NoCommonSecretPattern", source)
        self.assertNotIn("[string]$apikey", lowered)
        self.assertNotIn("$env:lif_deepseek_api_key", lowered)
        self.assertNotIn("response.content | out-file", lowered)
