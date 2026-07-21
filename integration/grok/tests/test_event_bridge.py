from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[3]
BUILDER = ROOT / "scripts" / "build_grok_event_bridge.py"
VERIFIER = ROOT / "scripts" / "verify_grok_event_bridge.py"
POSTRUN_ORCHESTRATOR = ROOT / "scripts" / "invoke_grok_postrun_evidence_bridge.ps1"
EVENT_SCHEMA = ROOT / "integration" / "grok" / "grok-event-bridge-event-v0.1.schema.json"
MANIFEST_SCHEMA = ROOT / "integration" / "grok" / "grok-event-bridge-manifest-v0.1.schema.json"
VERIFICATION_SCHEMA = ROOT / "integration" / "grok" / "grok-event-bridge-verification-v0.1.schema.json"
EMPTY_SHA = hashlib.sha256(b"").hexdigest()
POLICY_SHA = "1" * 64
RUN_ID = "FAKE-" + "2" * 32
SESSION_ID = "00000000-0000-4000-8000-000000000001"


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def _write_jsonl(path: Path, values: list[object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("".join(json.dumps(value) + "\n" for value in values), encoding="utf-8")


def _artifact(path: Path) -> dict[str, object]:
    raw = path.read_bytes()
    return {
        "path": str(path.resolve()),
        "bytes": len(raw),
        "sha256": hashlib.sha256(raw).hexdigest(),
    }


def _receipt(path: Path, receipt_id: str) -> None:
    _write_json(
        path,
        {
            "schema_version": "0.1.0",
            "receipt_kind": "grok-workspace-trust-preflight",
            "receipt_id": receipt_id,
            "created_at": "2026-07-21T00:00:00Z",
            "discovery": {
                "complete": True,
                "candidate_count": 0,
                "aggregate_sha256": EMPTY_SHA,
                "scan_policy_sha256": POLICY_SHA,
            },
            "decision": {"mode": "restricted", "launch_permitted": True},
            "valid": True,
        },
    )


class EventBridgeTests(unittest.TestCase):
    def test_postrun_uses_public_windows_process_environment(self) -> None:
        source = POSTRUN_ORCHESTRATOR.read_text(encoding="utf-8")
        self.assertIn("$info.EnvironmentVariables.Clear()", source)
        self.assertIn("$info.EnvironmentVariables[$name]", source)
        self.assertNotIn("GetField(\n        'environment'", source)

    def _fixture(self, temporary: str) -> tuple[Path, Path]:
        root = Path(temporary)
        run = root / "run"
        evidence = root / "evidence"
        workspace = run / "workspace"
        grok_home = run / "profile" / ".grok"
        session = grok_home / "sessions" / "encoded-workspace" / SESSION_ID
        provider = run / "provider"
        workspace.mkdir(parents=True)
        provider.mkdir(parents=True)
        evidence.mkdir(parents=True)

        config = grok_home / "config.toml"
        config.parent.mkdir(parents=True)
        config.write_text("[cli]\nuse_leader = false\n", encoding="utf-8")
        prompt = workspace / "prompt.txt"
        prompt.write_text("fixture prompt\n", encoding="utf-8")
        stdout = run / "grok.stdout.streaming.jsonl"
        _write_jsonl(
            stdout,
            [
                {"type": "thought", "data": "PRIVATE_REASONING_MARKER"},
                {"type": "text", "data": "PRIVATE_OUTPUT_MARKER"},
                {
                    "type": "end",
                    "stopReason": "EndTurn",
                    "sessionId": SESSION_ID,
                    "requestId": "request-1",
                    "usage": {"input_tokens": 1},
                    "num_turns": 2,
                },
            ],
        )
        provider_private = provider / "requests.private.jsonl"
        provider_body = b'{"messages":["PRIVATE_PROVIDER_MARKER"]}'
        provider_private.write_bytes(provider_body + b"\n")
        provider_result = provider / "provider-result.json"
        _write_json(
            provider_result,
            {
                "continuity": {
                    "reasoning_marker_preserved": True,
                    "tool_call_id_preserved": True,
                    "tool_result_observed": True,
                    "tool_result_marker_observed": True,
                },
                "requests": [
                    {
                        "sequence": 1,
                        "request_class": "primary",
                        "method": "POST",
                        "path": "/chat/completions",
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "message_count": 2,
                        "message_roles": ["user", "tool"],
                        "tool_names": ["read_file"],
                        "authorization_present": True,
                        "authorization_value_recorded": False,
                        "received_at_unix_ns": 1784592000000000000,
                        "body_bytes": len(provider_body),
                        "body_sha256": hashlib.sha256(provider_body).hexdigest(),
                    }
                ],
            },
        )
        preflight = run / "workspace-trust-receipt.preflight.json"
        launch = run / "workspace-trust-receipt.launch.json"
        postrun = evidence / "workspace-trust-receipt.postrun.json"
        _receipt(preflight, "TRUST-" + "3" * 32)
        _receipt(launch, "TRUST-" + "4" * 32)
        _receipt(postrun, "TRUST-" + "5" * 32)

        _write_jsonl(
            session / "events.jsonl",
            [
                {"ts": "2026-07-21T00:00:01Z", "type": "turn_started", "turn_number": 0},
                {"ts": "2026-07-21T00:00:02Z", "type": "tool_started", "tool_name": "read_file"},
                {"ts": "2026-07-21T00:00:03Z", "type": "tool_completed", "tool_name": "read_file", "outcome": "success"},
                {"ts": "2026-07-21T00:00:04Z", "type": "turn_ended", "outcome": "completed"},
            ],
        )
        _write_jsonl(
            session / "updates.jsonl",
            [
                {
                    "timestamp": 1784592001,
                    "method": "session/update",
                    "params": {
                        "sessionId": SESSION_ID,
                        "update": {
                            "sessionUpdate": "tool_call",
                            "toolCallId": "call-fixture-1",
                            "rawInput": "PRIVATE_TOOL_INPUT",
                        },
                    },
                },
                {
                    "timestamp": 1784592002,
                    "method": "session/update",
                    "params": {
                        "sessionId": SESSION_ID,
                        "update": {
                            "sessionUpdate": "tool_call_update",
                            "toolCallId": "call-fixture-1",
                            "status": "completed",
                            "rawOutput": "PRIVATE_TOOL_OUTPUT",
                        },
                    },
                },
            ],
        )
        _write_jsonl(session / "chat_history.jsonl", [{"type": "user", "content": "PRIVATE_CHAT"}])
        _write_json(
            session / "summary.json",
            {"info": {"id": SESSION_ID, "cwd": str(workspace.resolve())}},
        )
        _write_json(session / "signals.json", {"session_id": SESSION_ID})
        trace_status = evidence / "trace-status.json"
        export_status = evidence / "export-status.json"
        _write_json(
            trace_status,
            {
                "state": "timed_out",
                "exit_code": -1,
                "timed_out": True,
                "artifact_created": False,
                "completed_at": "2026-07-21T00:01:00Z",
            },
        )
        _write_json(
            export_status,
            {
                "state": "failed",
                "exit_code": 1,
                "timed_out": False,
                "artifact_created": False,
                "completed_at": "2026-07-21T00:01:01Z",
            },
        )

        artifacts = {
            "config": _artifact(config),
            "prompt": _artifact(prompt),
            "stdout": _artifact(stdout),
            "provider_result": _artifact(provider_result),
            "provider_private": _artifact(provider_private),
            "workspace_trust_receipt_preflight": _artifact(preflight),
            "workspace_trust_receipt_launch": _artifact(launch),
        }
        _write_json(
            run / "result.json",
            {
                "schema_version": "0.3.0",
                "run_id": RUN_ID,
                "session_id": SESSION_ID,
                "created_at": "2026-07-21T00:00:05Z",
                "scenario": "tool-continuity",
                "process": {"exit_code": 0, "timed_out": False},
                "workspace_trust": {
                    "aggregate_sha256": EMPTY_SHA,
                    "scan_policy_sha256": POLICY_SHA,
                },
                "artifacts": artifacts,
                "valid": True,
            },
        )
        return run, evidence

    def _run(self, run: Path, evidence: Path, output: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(BUILDER),
                "--run-directory", str(run),
                "--evidence-directory", str(evidence),
                "--output-directory", str(output),
                "--postrun-receipt", str(evidence / "workspace-trust-receipt.postrun.json"),
                "--trace-status", str(evidence / "trace-status.json"),
                "--export-status", str(evidence / "export-status.json"),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=20,
            check=False,
        )

    def _verify(self, bridge: Path, output: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--bridge-directory", str(bridge),
                "--schema-directory", str(ROOT / "integration" / "grok"),
                "--output", str(output),
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=20,
            check=False,
        )

    def test_builds_schema_valid_hash_chain_without_copying_raw_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run, evidence = self._fixture(temporary)
            output = evidence / "bridge"
            completed = self._run(run, evidence, output)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            manifest = json.loads((output / "manifest.json").read_text(encoding="utf-8"))
            manifest_schema = json.loads(MANIFEST_SCHEMA.read_text(encoding="utf-8"))
            Draft202012Validator(
                manifest_schema, format_checker=FormatChecker()
            ).validate(manifest)
            self.assertTrue(manifest["valid"])
            self.assertEqual(manifest["completeness"]["overall"], "partial")
            self.assertEqual(manifest["completeness"]["session_event_tool_lifecycle"], "observed")
            self.assertEqual(manifest["completeness"]["session_update_tool_identity"], "observed")
            self.assertEqual(manifest["completeness"]["trace"], "timed_out")
            self.assertEqual(manifest["completeness"]["export"], "failed")

            event_schema = json.loads(EVENT_SCHEMA.read_text(encoding="utf-8"))
            validator = Draft202012Validator(event_schema, format_checker=FormatChecker())
            previous = None
            event_text = (output / "events.bridge.jsonl").read_text(encoding="utf-8")
            for line in event_text.splitlines():
                event = json.loads(line)
                validator.validate(event)
                self.assertEqual(event["previous_event_sha256"], previous)
                claimed = event.pop("event_sha256")
                actual = hashlib.sha256(
                    json.dumps(
                        event,
                        ensure_ascii=False,
                        sort_keys=True,
                        separators=(",", ":"),
                    ).encode("utf-8")
                ).hexdigest()
                self.assertEqual(claimed, actual)
                previous = claimed
            for marker in (
                "PRIVATE_REASONING_MARKER",
                "PRIVATE_OUTPUT_MARKER",
                "PRIVATE_TOOL_INPUT",
                "PRIVATE_TOOL_OUTPUT",
                "PRIVATE_PROVIDER_MARKER",
                "PRIVATE_CHAT",
            ):
                self.assertNotIn(marker, event_text)
            verification_path = evidence / "verification.json"
            verified = self._verify(output, verification_path)
            self.assertEqual(verified.returncode, 0, verified.stderr)
            verification = json.loads(verification_path.read_text(encoding="utf-8"))
            verification_schema = json.loads(
                VERIFICATION_SCHEMA.read_text(encoding="utf-8")
            )
            Draft202012Validator(
                verification_schema, format_checker=FormatChecker()
            ).validate(verification)
            self.assertTrue(verification["valid"])
            self.assertTrue(all(verification["checks"].values()))

    def test_refuses_overwrite_and_detects_source_artifact_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run, evidence = self._fixture(temporary)
            output = evidence / "bridge"
            first = self._run(run, evidence, output)
            self.assertEqual(first.returncode, 0, first.stderr)
            second = self._run(run, evidence, output)
            self.assertNotEqual(second.returncode, 0)
            self.assertIn("refusing to overwrite", second.stderr)

        with tempfile.TemporaryDirectory() as temporary:
            run, evidence = self._fixture(temporary)
            (run / "grok.stdout.streaming.jsonl").write_text("{}\n", encoding="utf-8")
            completed = self._run(run, evidence, evidence / "bridge")
            self.assertEqual(completed.returncode, 2, completed.stderr)
            manifest = json.loads(
                (evidence / "bridge" / "manifest.json").read_text(encoding="utf-8")
            )
            self.assertFalse(manifest["valid"])
            self.assertFalse(manifest["checks"]["source_result_artifacts_match"])

    def test_replay_verifier_rejects_event_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            run, evidence = self._fixture(temporary)
            bridge = evidence / "bridge"
            built = self._run(run, evidence, bridge)
            self.assertEqual(built.returncode, 0, built.stderr)
            events_path = bridge / "events.bridge.jsonl"
            events = events_path.read_text(encoding="utf-8").splitlines()
            first = json.loads(events[0])
            first["details"]["candidate_count"] = 99
            events[0] = json.dumps(first, sort_keys=True)
            events_path.write_text("\n".join(events) + "\n", encoding="utf-8")
            verified = self._verify(bridge, evidence / "tampered-verification.json")
            self.assertEqual(verified.returncode, 2, verified.stderr)
            report = json.loads(
                (evidence / "tampered-verification.json").read_text(encoding="utf-8")
            )
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["event_journal_artifact_matches"])
            self.assertFalse(report["checks"]["event_hash_chain_valid"])


if __name__ == "__main__":
    unittest.main()
