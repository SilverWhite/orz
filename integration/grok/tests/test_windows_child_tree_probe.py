from __future__ import annotations

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

import jsonschema


ROOT = Path(__file__).resolve().parents[3]
VERIFIER_PATH = ROOT / "scripts" / "verify_grok_windows_child_tree_probe.py"
RESULT_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-windows-child-tree-probe-result-v0.1.schema.json"
)
VERIFICATION_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-windows-child-tree-probe-verification-v0.1.schema.json"
)

SPEC = importlib.util.spec_from_file_location("child_tree_verifier", VERIFIER_PATH)
assert SPEC is not None and SPEC.loader is not None
VERIFIER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(VERIFIER)


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _artifact(path: Path) -> dict[str, object]:
    value = path.read_bytes()
    return {"path": str(path), "bytes": len(value), "sha256": _sha(value)}


class WindowsChildTreeProbeVerifierTests(unittest.TestCase):
    def _fixture(self, root: Path, scenario: str) -> Path:
        nonce = "LIFCHILD-" + "a" * 32
        script = root / "child_tree_fixture.py"
        script.write_text("# fixture\n", encoding="utf-8")
        script_sha = _sha(script.read_bytes())
        binary = root / "grok.exe"
        binary.write_bytes(b"MZ-fake-binary")
        release = root / "release.json"
        _write_json(
            release,
            {
                "binary_release": {
                    "version": "0.2.111",
                    "bytes": binary.stat().st_size,
                    "sha256": _sha(binary.read_bytes()),
                    "authenticode_status": "Valid",
                }
            },
        )

        roles: list[dict[str, object]] = []
        role_artifacts: list[dict[str, object]] = []
        pids = {"root": 41001, "child": 41002, "grandchild": 41003}
        parents = {"root": 40000, "child": 41001, "grandchild": 41002}
        for role in ("root", "child", "grandchild"):
            value = {
                "schema_version": "0.1.0",
                "fixture_kind": "windows-child-tree",
                "nonce": nonce,
                "role": role,
                "pid": pids[role],
                "parent_pid": parents[role],
                "started_at_unix_ns": 1000 + pids[role],
                "script_path": str(script),
                "script_sha256": script_sha,
                "normal_completion": False,
            }
            path = root / f"{role}.json"
            _write_json(path, value)
            roles.append(value)
            role_artifacts.append(_artifact(path))

        expected_sequences = {
            "tool_timeout": ["run_terminal_command"],
            "task_cancel": [
                "run_terminal_command",
                "kill_command_or_subagent",
                "get_command_or_subagent_output",
            ],
            "parent_exit": ["run_terminal_command"],
        }
        expected_counts = {"tool_timeout": 2, "task_cancel": 4, "parent_exit": 2}
        sequence = expected_sequences[scenario]
        private_capture = root / "requests.private.jsonl"
        rows = []
        for index in range(expected_counts[scenario]):
            messages: list[dict[str, object]] = [{"role": "user", "content": "fixture"}]
            for tool_index, tool_name in enumerate(sequence[:index], start=1):
                messages.append(
                    {
                        "role": "assistant",
                        "tool_calls": [
                            {
                                "id": f"call-{tool_index}",
                                "type": "function",
                                "function": {"name": tool_name, "arguments": "{}"},
                            }
                        ],
                    }
                )
                messages.append(
                    {
                        "role": "tool",
                        "tool_call_id": f"call-{tool_index}",
                        "content": "fixture result",
                    }
                )
            rows.append(
                {
                    "body": {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": messages,
                    }
                }
            )
        private_capture.write_text(
            "".join(json.dumps(row, sort_keys=True) + "\n" for row in rows),
            encoding="utf-8",
        )
        provider_result = root / "provider-result.json"
        _write_json(
            provider_result,
            {
                "terminal_state": "succeeded",
                "response": {"real_model_invoked": False},
                "continuity": {
                    "parent_exit_disconnect_observed": scenario == "parent_exit"
                },
            },
        )
        marker_text = "".join(
            f"LIF_CHILD_TREE_STDOUT:{nonce}:{role}:ready\n"
            f"LIF_CHILD_TREE_STDERR:{nonce}:{role}:ready\n"
            for role in ("root", "child", "grandchild")
        )
        grok_stdout = root / "grok.stdout.log"
        grok_stdout.write_text(
            marker_text if scenario != "parent_exit" else "", encoding="utf-8"
        )
        grok_stderr = root / "grok.stderr.log"
        grok_stderr.write_text("", encoding="utf-8")
        provider_stdout = root / "provider.stdout.log"
        provider_stdout.write_text("provider\n", encoding="utf-8")
        provider_stderr = root / "provider.stderr.log"
        provider_stderr.write_text("", encoding="utf-8")

        aggregate = "b" * 64
        scan_policy = "c" * 64
        receipt_artifacts = {}
        for name in ("preflight", "launch", "postrun"):
            path = root / f"workspace-trust.{name}.json"
            _write_json(
                path,
                {
                    "valid": True,
                    "discovery": {
                        "aggregate_sha256": aggregate,
                        "scan_policy_sha256": scan_policy,
                    },
                },
            )
            receipt_artifacts[name] = _artifact(path)
        firewall_receipt = root / "firewall-receipt.json"
        _write_json(
            firewall_receipt,
            {"removed": True, "remaining_rule_count": 0},
        )
        marker_projection = {
            role: {
                "stdout": scenario != "parent_exit",
                "stderr": scenario != "parent_exit",
            }
            for role in ("root", "child", "grandchild")
        }
        pre_processes = [
            {
                "pid": pids[role],
                "parent_pid": parents[role],
                "name": "python.exe",
                "creation_date": "",
                "command_line_bytes": 100,
                "command_line_sha256": "d" * 64,
                "nonce_present": True,
                "command_line_recorded": False,
            }
            for role in ("root", "child", "grandchild")
        ]
        result = {
            "schema_version": "0.1.0",
            "probe_kind": "grok-windows-child-tree",
            "probe_id": "CHILDTREE-" + "e" * 32,
            "scenario": scenario,
            "valid": True,
            "started_at_unix_ns": 1000,
            "completed_at_unix_ns": 2000,
            "binary": {
                "path": str(binary),
                "version": "grok 0.2.111 (fixture)",
                "bytes": binary.stat().st_size,
                "sha256": _sha(binary.read_bytes()),
                "authenticode_status": "Valid",
                "release_metadata_path": str(release),
                "release_metadata_sha256": _sha(release.read_bytes()),
                "release_version": "0.2.111",
            },
            "fixture": {
                "nonce": nonce,
                "script": _artifact(script),
                "command_sha256": "f" * 64,
                "tool_timeout_ms": 3000,
                "hold_seconds": 300,
            },
            "process_tree": {
                "roles": roles,
                "role_artifacts": role_artifacts,
                "pre_trigger_processes": pre_processes,
                "post_trigger_processes": [],
                "parent_exit_triggered": scenario == "parent_exit",
                "residue_zero": True,
            },
            "grok": {
                "pid": 40000,
                "exit_code": 0,
                "outer_job_created": True,
                "outer_job_assigned": True,
                "outer_job_closed": True,
            },
            "provider": {
                "scenario": {
                    "tool_timeout": "process-timeout",
                    "task_cancel": "process-cancel",
                    "parent_exit": "process-parent-exit",
                }[scenario],
                "primary_request_count": expected_counts[scenario],
                "expected_primary_request_count": expected_counts[scenario],
                "tool_sequence": sequence,
                "expected_tool_sequence": sequence,
                "real_model_invoked": False,
                "private_capture": _artifact(private_capture),
                "result": _artifact(provider_result),
            },
            "capture": {
                "grok_stdout": _artifact(grok_stdout),
                "grok_stderr": _artifact(grok_stderr),
                "provider_stdout": _artifact(provider_stdout),
                "provider_stderr": _artifact(provider_stderr),
                "marker_projection": marker_projection,
                "output_drain_required": scenario != "parent_exit",
                "output_drain_observed": scenario != "parent_exit",
            },
            "workspace": {
                **receipt_artifacts,
                "control_aggregate_sha256": aggregate,
                "scan_policy_sha256": scan_policy,
            },
            "firewall": {
                "receipt": _artifact(firewall_receipt),
                "removed": True,
                "remaining_rule_count": 0,
            },
            "checks": {f"check_{index}": True for index in range(15)},
            "limitations": ["one", "two", "three"],
        }
        result_path = root / "result.json"
        _write_json(result_path, result)
        return result_path

    def test_all_three_scenarios_replay_and_validate(self) -> None:
        result_schema = json.loads(RESULT_SCHEMA.read_text(encoding="utf-8"))
        verification_schema = json.loads(
            VERIFICATION_SCHEMA.read_text(encoding="utf-8")
        )
        for scenario in ("tool_timeout", "task_cancel", "parent_exit"):
            with self.subTest(scenario=scenario), tempfile.TemporaryDirectory() as temporary:
                result_path = self._fixture(Path(temporary), scenario)
                result = json.loads(result_path.read_text(encoding="utf-8"))
                jsonschema.validate(result, result_schema)
                verification = VERIFIER.verify(result_path)
                self.assertTrue(
                    verification["valid"],
                    [name for name, value in verification["checks"].items() if not value],
                )
                jsonschema.validate(verification, verification_schema)

    def test_provider_capture_tamper_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            result_path = self._fixture(Path(temporary), "tool_timeout")
            result = json.loads(result_path.read_text(encoding="utf-8"))
            capture = Path(result["provider"]["private_capture"]["path"])
            capture.write_text("{}\n", encoding="utf-8")
            verification = VERIFIER.verify(result_path)
            self.assertFalse(verification["valid"])
            self.assertFalse(verification["checks"]["provider_artifacts_match"])


if __name__ == "__main__":
    unittest.main()
