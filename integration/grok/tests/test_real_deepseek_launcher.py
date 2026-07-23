from __future__ import annotations

import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[3]
LAUNCHER = ROOT / "scripts" / "invoke_grok_real_deepseek_conformance.ps1"
PLAN_SCHEMA = ROOT / "integration" / "grok" / "grok-real-deepseek-plan-v0.1.schema.json"
RESULT_SCHEMA = ROOT / "integration" / "grok" / "grok-real-deepseek-result-v0.1.schema.json"
FAILURE_SCHEMA = ROOT / "integration" / "grok" / "grok-real-deepseek-failure-v0.1.schema.json"


class RealDeepSeekLauncherTests(unittest.TestCase):
    def test_launcher_has_no_raw_credential_input_or_debug_surface(self) -> None:
        source = LAUNCHER.read_text(encoding="utf-8")
        lowered = source.lower()
        self.assertNotIn("[string]$apikey", lowered)
        self.assertNotIn("$env:lif_deepseek_api_key", lowered)
        self.assertNotIn("--debug-file", lowered)
        self.assertNotIn("'--debug'", lowered)
        self.assertIn("$credentialTarget = 'FEP-Agent/DeepSeek'", source)
        self.assertIn("$credentialEnvironmentName = 'LIF_DEEPSEEK_API_KEY'", source)
        self.assertIn("CredRead(target, CRED_TYPE_GENERIC", source)
        self.assertIn("CredFree(pointer)", source)
        self.assertIn("Marshal.WriteByte(credential.CredentialBlob", source)

    def test_launcher_freezes_minimal_grok_execution_boundary(self) -> None:
        source = LAUNCHER.read_text(encoding="utf-8")
        for required in (
            "$baseUrl = 'https://api.deepseek.com'",
            "$modelName = 'deepseek-v4-pro'",
            "'--max-turns', '1'",
            "'--no-memory'",
            "'--no-subagents'",
            "'--disable-web-search'",
            "'--permission-mode', 'plan'",
            "'--sandbox', 'read-only'",
            "'--tools', ''",
            "ConfigureNoHeapWer()",
            "CreateKillOnCloseJob()",
            "Assert-NoCommonSecretPattern",
            "raw_exception_recorded = $false",
            "raw_stdout_recorded = $false",
            "raw_stderr_recorded = $false",
        ):
            self.assertIn(required, source)
        self.assertNotIn("grok.stdout.streaming.jsonl", source)
        self.assertNotIn("grok.stderr.log", source)
        self.assertIn("[ValidateSet('Plan', 'Execute')]", source)
        self.assertIn("retry_budget = 0", source)

    def test_plan_and_result_schemas_accept_minimal_valid_documents(self) -> None:
        plan_schema = json.loads(PLAN_SCHEMA.read_text(encoding="utf-8"))
        result_schema = json.loads(RESULT_SCHEMA.read_text(encoding="utf-8"))
        failure_schema = json.loads(FAILURE_SCHEMA.read_text(encoding="utf-8"))
        plan = {
            "schema_version": "0.1.0",
            "plan_kind": "grok-real-deepseek-conformance",
            "created_at": "2026-07-23T00:00:00Z",
            "confirmation_summary": {},
            "confirmation_summary_sha256": "a" * 64,
            "confirmation_token_hint": "ALLOW-GROK-" + "A" * 12,
            "execution": {
                "credential_read": False,
                "grok_started": False,
                "network_attempted": False,
                "billable_request_made": False,
            },
            "limitations": ["offline plan"],
        }
        result = {
            "schema_version": "0.1.0",
            "result_kind": "grok-real-deepseek-conformance",
            "valid": True,
            "started_at": "2026-07-23T00:00:00Z",
            "completed_at": "2026-07-23T00:00:01Z",
            "confirmation_summary_sha256": "b" * 64,
            "request_count": 1,
            "retry_count": 0,
            "provider": "deepseek",
            "endpoint": "https://api.deepseek.com/chat/completions",
            "model": "deepseek-v4-pro",
            "marker_matched": True,
            "response_content_recorded": False,
            "credential_value_recorded": False,
            "process": {},
            "usage": {},
            "controls": {},
            "artifact_leak_scan": {
                "complete": True,
                "actual_credential_read_for_scan": False,
                "hit_count": 0,
            },
            "limitations": ["one fixed conformance"],
        }
        self.assertEqual(list(Draft202012Validator(plan_schema).iter_errors(plan)), [])
        self.assertEqual(list(Draft202012Validator(result_schema).iter_errors(result)), [])
        failure = {
            "schema_version": "0.1.0",
            "result_kind": "grok-real-deepseek-conformance-failure",
            "valid": False,
            "failed_at": "2026-07-23T00:00:00Z",
            "confirmation_summary_sha256": "c" * 64,
            "stage": "credential_acquire_and_process_start",
            "error_type": "MethodInvocationException",
            "raw_exception_recorded": False,
            "grok_started": False,
            "provider_receipt": "not_attempted",
            "billing_status": "not_attempted",
            "retry_count": 0,
            "artifact_leak_scan": {
                "policy": "grok-real-artifact-common-secret-patterns-v0.1",
                "complete": True,
                "scanned_file_count": 6,
                "pending_document_scanned": True,
                "actual_credential_read_for_scan": False,
                "hit_count": 0,
            },
        }
        self.assertEqual(
            list(Draft202012Validator(failure_schema).iter_errors(failure)), []
        )


if __name__ == "__main__":
    unittest.main()
