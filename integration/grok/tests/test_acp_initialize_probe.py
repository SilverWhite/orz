from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[3]
VERIFIER = ROOT / "scripts" / "verify_grok_acp_initialize_probe.py"


def _write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def _artifact(path: Path) -> dict:
    raw = path.read_bytes()
    return {"path": str(path.resolve()), "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()}


class AcpInitializeProbeVerifierTests(unittest.TestCase):
    def _fixture(self, root: Path) -> tuple[Path, dict]:
        binary = root / "grok.exe"
        binary.write_bytes(b"fixture-grok-binary")
        request_path = root / "request.json"
        request = {
            "jsonrpc": "2.0",
            "id": "lif-acp-init-1",
            "method": "initialize",
            "params": {
                "protocolVersion": 1,
                "clientCapabilities": {},
                "clientInfo": {
                    "name": "lif-acp-capability-probe",
                    "title": "LIF ACP Capability Probe",
                    "version": "0.1.0",
                },
            },
        }
        _write_json(request_path, request)
        response_path = root / "response.json"
        response = {
            "jsonrpc": "2.0",
            "id": "lif-acp-init-1",
            "result": {
                "protocolVersion": 1,
                "agentCapabilities": {
                    "loadSession": True,
                    "promptCapabilities": {"image": True, "audio": False},
                },
                "agentInfo": {"name": "fixture-agent", "title": "Fixture", "version": "1.2.3"},
                "authMethods": [],
                "_meta": {"x.ai": {"methods": ["x.ai/fs/list", "x.ai/git/status"]}},
            },
        }
        _write_json(response_path, response)
        stdout_path = root / "stdout.jsonl"
        stdout_path.write_text(json.dumps(response, separators=(",", ":")) + "\n", encoding="utf-8")
        stderr_path = root / "stderr.txt"
        stderr_path.write_text("", encoding="utf-8")
        pre_path = root / "preflight.json"
        post_path = root / "postrun.json"
        trust = {
            "valid": True,
            "decision": {"mode": "restricted", "launch_permitted": True},
            "discovery": {"candidate_count": 0, "aggregate_sha256": "a" * 64},
        }
        _write_json(pre_path, trust)
        _write_json(post_path, trust)
        binary_record = _artifact(binary)
        all_checks = {
            "binary_locked": True,
            "signature_valid": True,
            "workspace_restricted_empty_and_stable": True,
            "request_initialize_only": True,
            "response_id_matches": True,
            "protocol_version_agreed": True,
            "rpc_result_observed": True,
            "no_unexpected_stdout": True,
            "clean_environment": True,
            "network_blocked_and_cleaned": True,
            "job_contained": True,
            "leak_scan_clean": True,
        }
        result = {
            "schema_version": "0.1.0",
            "probe_kind": "grok-acp-initialize-no-model",
            "probe_id": "ACPINIT-" + "1" * 32,
            "valid": True,
            "started_at": "2026-07-21T10:00:00Z",
            "completed_at": "2026-07-21T10:00:01Z",
            "binary": {
                **binary_record,
                "locked_sha256": binary_record["sha256"],
                "version": "0.2.106",
                "authenticode_status": "Valid",
                "signer_subject": "CN=X.AI LLC",
            },
            "request": {
                "artifact": _artifact(request_path),
                "jsonrpc": "2.0",
                "id": "lif-acp-init-1",
                "method": "initialize",
                "protocol_version": 1,
                "client_capability_paths": [],
                "session_or_prompt_requests_sent": 0,
            },
            "response": {
                "state": "observed",
                "artifact": _artifact(response_path),
                "jsonrpc": "2.0",
                "id_matches": True,
                "has_result": True,
                "has_error": False,
                "protocol_version": 1,
                "agent_capability_paths": [
                    "agentCapabilities.loadSession",
                    "agentCapabilities.promptCapabilities.audio",
                    "agentCapabilities.promptCapabilities.image",
                ],
                "extension_meta_paths": ["_meta.x.ai.methods[]"],
                "agent_info": {"name": "fixture-agent", "title": "Fixture", "version": "1.2.3"},
                "auth_method_count": 0,
                "allowed_notification_count": 0,
                "allowed_notification_methods": [],
                "unexpected_stdout_line_count": 0,
            },
            "process": {
                "exit_code": 0,
                "timed_out": False,
                "duration_ms": 1.0,
                "terminal_state": "exited",
                "stdout": _artifact(stdout_path),
                "stderr": _artifact(stderr_path),
                "containment": {
                    "job_object_created": True,
                    "job_object_assigned": True,
                    "kill_on_close": True,
                    "job_object_closed": True,
                    "assignment_race_known": True,
                },
            },
            "workspace_trust": {
                "preflight_receipt": _artifact(pre_path),
                "postrun_receipt": _artifact(post_path),
                "preflight_valid": True,
                "postrun_valid": True,
                "candidate_count": 0,
                "aggregate_unchanged": True,
            },
            "network": {
                "all_profiles_enabled": True,
                "outbound_block_created": True,
                "rule_name": "fixture-rule",
                "rule_removed": True,
                "remaining_rule_count": 0,
            },
            "environment": {
                "clean": True,
                "variable_names": ["GROK_HOME", "PATH"],
                "credential_variable_names": [],
                "proxy_fail_closed": True,
            },
            "leak_scan": {
                "scanned_artifacts": [str(request_path), str(response_path)],
                "forbidden_markers": ["XAI_API_KEY"],
                "hit_count": 0,
                "hits": [],
            },
            "checks": all_checks,
        }
        result_path = root / "result.json"
        _write_json(result_path, result)
        return result_path, result

    def _verify(self, root: Path, result_path: Path, name: str) -> tuple[subprocess.CompletedProcess[str], dict]:
        output = root / name
        completed = subprocess.run(
            [sys.executable, str(VERIFIER), "--result", str(result_path), "--output", str(output)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=20,
            check=False,
        )
        return completed, json.loads(output.read_text(encoding="utf-8"))

    def test_valid_fixture_rebuilds_response_and_controls(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, _ = self._fixture(root)
            completed, report = self._verify(root, result_path, "verification.json")
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["valid"])
            self.assertTrue(all(report["checks"].values()))

    def test_request_method_tamper_is_detected_even_when_artifact_is_rehashed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, result = self._fixture(root)
            request_path = Path(result["request"]["artifact"]["path"])
            request = json.loads(request_path.read_text(encoding="utf-8"))
            request["method"] = "session/new"
            _write_json(request_path, request)
            result["request"]["artifact"] = _artifact(request_path)
            _write_json(result_path, result)
            completed, report = self._verify(root, result_path, "request-tampered.json")
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["request_initialize_only"])

    def test_response_and_capability_projection_tampering_are_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, result = self._fixture(root)
            response_path = Path(result["response"]["artifact"]["path"])
            response = json.loads(response_path.read_text(encoding="utf-8"))
            response["result"]["agentCapabilities"]["loadSession"] = False
            _write_json(response_path, response)
            completed, report = self._verify(root, result_path, "raw-response-tampered.json")
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["response_artifact_matches"])

            result["response"]["artifact"] = _artifact(response_path)
            result["response"]["agent_capability_paths"] = ["agentCapabilities.invented"]
            _write_json(result_path, result)
            completed, report = self._verify(root, result_path, "projection-tampered.json")
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["capability_projection_matches"])

    def test_workspace_receipt_mismatch_is_detected_after_rehash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, result = self._fixture(root)
            post_path = Path(result["workspace_trust"]["postrun_receipt"]["path"])
            post = json.loads(post_path.read_text(encoding="utf-8"))
            post["discovery"]["aggregate_sha256"] = "b" * 64
            _write_json(post_path, post)
            result["workspace_trust"]["postrun_receipt"] = _artifact(post_path)
            _write_json(result_path, result)
            completed, report = self._verify(root, result_path, "trust-tampered.json")
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["control_receipts_match"])

    def test_only_exact_empty_mcp_update_notification_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, result = self._fixture(root)
            stdout_path = Path(result["process"]["stdout"]["path"])
            notification = {
                "jsonrpc": "2.0",
                "method": "_x.ai/mcp/servers_updated",
                "params": {"mcpServers": []},
            }
            with stdout_path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(notification, separators=(",", ":")) + "\n")
            result["process"]["stdout"] = _artifact(stdout_path)
            result["response"]["allowed_notification_count"] = 1
            result["response"]["allowed_notification_methods"] = ["_x.ai/mcp/servers_updated"]
            _write_json(result_path, result)
            completed, report = self._verify(root, result_path, "allowed-notification.json")
            self.assertEqual(completed.returncode, 0, completed.stderr)
            self.assertTrue(report["checks"]["response_semantics_match"])

            notification["params"] = {"mcpServers": [{"name": "untrusted"}]}
            response = json.loads(Path(result["response"]["artifact"]["path"]).read_text(encoding="utf-8"))
            stdout_path.write_text(
                json.dumps(response, separators=(",", ":"))
                + "\n"
                + json.dumps(notification, separators=(",", ":"))
                + "\n",
                encoding="utf-8",
            )
            result["process"]["stdout"] = _artifact(stdout_path)
            _write_json(result_path, result)
            completed, report = self._verify(root, result_path, "rejected-notification.json")
            self.assertEqual(completed.returncode, 2)
            self.assertFalse(report["checks"]["response_semantics_match"])


if __name__ == "__main__":
    unittest.main()
