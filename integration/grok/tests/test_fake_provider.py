from __future__ import annotations

import http.client
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parents[3]
PROVIDER = ROOT / "scripts" / "fake_deepseek_provider.py"


class FakeProviderTests(unittest.TestCase):
    def _post(self, port: int, body: dict[str, object]) -> str:
        connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
        connection.request(
            "POST",
            "/chat/completions",
            body=json.dumps(body).encode("utf-8"),
            headers={"Content-Type": "application/json"},
        )
        response = connection.getresponse()
        content = response.read().decode("utf-8")
        connection.close()
        self.assertEqual(response.status, 200)
        return content

    def _start_process_provider(
        self, root: Path, scenario: str
    ) -> tuple[subprocess.Popen[str], Path]:
        output = root / "provider"
        command_path = root / "command.txt"
        command_path.write_text("Write-Output fixture\n", encoding="utf-8")
        records = root / "records"
        records.mkdir()
        for role in ("root", "child", "grandchild"):
            (records / f"{role}.json").write_text("{}\n", encoding="utf-8")
        (records / "driver-pre-trigger-ready.json").write_text(
            "{}\n", encoding="utf-8"
        )
        process = subprocess.Popen(
            [
                sys.executable,
                str(PROVIDER),
                "--output-directory",
                str(output),
                "--timeout-seconds",
                "10",
                "--scenario",
                scenario,
                "--terminal-command-path",
                str(command_path),
                "--process-record-directory",
                str(records),
                "--tool-timeout-ms",
                "1500",
            ],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        ready_path = output / "ready.json"
        deadline = time.monotonic() + 5
        while not ready_path.is_file() and time.monotonic() < deadline:
            if process.poll() is not None:
                break
            time.sleep(0.05)
        self.assertTrue(ready_path.is_file())
        return process, output

    def test_process_timeout_emits_fixed_terminal_tool_call(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            process, output = self._start_process_provider(
                Path(temporary), "process-timeout"
            )
            try:
                ready = json.loads(
                    (output / "ready.json").read_text(encoding="utf-8")
                )
                first = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [{"role": "user", "content": "fixture"}],
                    },
                )
                self.assertIn("run_terminal_command", first)
                self.assertIn(r"\"background\":false", first)
                self.assertIn(r"\"timeout\":1500", first)
                second = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [
                            {
                                "role": "assistant",
                                "tool_calls": [
                                    {
                                        "id": "call_lif_process_tree_001",
                                        "type": "function",
                                        "function": {
                                            "name": "run_terminal_command",
                                            "arguments": "{}",
                                        },
                                    }
                                ],
                            },
                            {
                                "role": "tool",
                                "tool_call_id": "call_lif_process_tree_001",
                                "content": "timed out",
                            },
                        ],
                    },
                )
                self.assertIn("LIF_PROCESS_TREE_PROBE_OK", second)
                _stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
                result = json.loads(
                    (output / "provider-result.json").read_text(encoding="utf-8")
                )
                self.assertEqual(result["primary_request_count"], 2)
                self.assertFalse(result["response"]["real_model_invoked"])
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)

    def test_process_cancel_uses_observed_task_id_then_fetches_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            process, output = self._start_process_provider(
                Path(temporary), "process-cancel"
            )
            try:
                ready = json.loads(
                    (output / "ready.json").read_text(encoding="utf-8")
                )
                first = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [{"role": "user", "content": "fixture"}],
                    },
                )
                self.assertIn("run_terminal_command", first)
                self.assertIn(r"\"background\":true", first)
                task_id = "task-fixture-001"
                second = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [
                            {
                                "role": "assistant",
                                "tool_calls": [
                                    {
                                        "id": "call_lif_process_tree_001",
                                        "type": "function",
                                        "function": {
                                            "name": "run_terminal_command",
                                            "arguments": "{}",
                                        },
                                    }
                                ],
                            },
                            {
                                "role": "tool",
                                "tool_call_id": "call_lif_process_tree_001",
                                "content": {"task_id": task_id},
                            },
                        ],
                    },
                )
                self.assertIn("kill_command_or_subagent", second)
                self.assertIn(task_id, second)
                third = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [
                            {
                                "role": "assistant",
                                "tool_calls": [
                                    {
                                        "id": "call_lif_process_tree_kill_001",
                                        "type": "function",
                                        "function": {
                                            "name": "kill_command_or_subagent",
                                            "arguments": json.dumps(
                                                {"task_id": task_id}
                                            ),
                                        },
                                    }
                                ],
                            },
                            {
                                "role": "tool",
                                "tool_call_id": "call_lif_process_tree_kill_001",
                                "content": "killed",
                            },
                        ],
                    },
                )
                self.assertIn("get_command_or_subagent_output", third)
                self.assertIn(task_id, third)
                fourth = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [
                            {
                                "role": "assistant",
                                "tool_calls": [
                                    {
                                        "id": "call_lif_process_tree_output_001",
                                        "type": "function",
                                        "function": {
                                            "name": "get_command_or_subagent_output",
                                            "arguments": "{}",
                                        },
                                    }
                                ],
                            },
                            {
                                "role": "tool",
                                "tool_call_id": "call_lif_process_tree_output_001",
                                "content": "fixture output",
                            },
                        ],
                    },
                )
                self.assertIn("LIF_PROCESS_TREE_PROBE_OK", fourth)
                _stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
                result = json.loads(
                    (output / "provider-result.json").read_text(encoding="utf-8")
                )
                self.assertEqual(result["primary_request_count"], 4)
                self.assertEqual(
                    result["continuity"]["background_task_id"], task_id
                )
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)

    def test_process_cancel_accepts_grok_uuid_task_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            process, output = self._start_process_provider(
                Path(temporary), "process-cancel"
            )
            try:
                ready = json.loads(
                    (output / "ready.json").read_text(encoding="utf-8")
                )
                self._post(
                    ready["port"],
                    {"model": "deepseek-v4-pro", "stream": True, "messages": []},
                )
                task_id = "019f8e12-30e3-7a01-9f6a-1fe6720d434b"
                second = self._post(
                    ready["port"],
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [
                            {
                                "role": "tool",
                                "tool_call_id": "call_lif_process_tree_001",
                                "content": (
                                    f"<task-id>{task_id}</task-id>\n"
                                    "<task-type>bash</task-type>\n"
                                    "<status>running</status>"
                                ),
                            }
                        ],
                    },
                )
                self.assertIn("kill_command_or_subagent", second)
                self.assertIn(task_id, second)
            finally:
                if process.poll() is None:
                    process.kill()
                process.communicate(timeout=5)

    def test_tool_cancel_stops_after_one_tool_request(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "provider"
            fixture = root / "tool-fixture.txt"
            fixture.write_text("LIF_TOOL_FIXTURE_CONTENT_001\n", encoding="utf-8")
            process = subprocess.Popen(
                [
                    sys.executable,
                    str(PROVIDER),
                    "--output-directory",
                    str(output),
                    "--timeout-seconds",
                    "10",
                    "--scenario",
                    "tool-cancel",
                    "--tool-fixture-path",
                    str(fixture),
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                ready_path = output / "ready.json"
                deadline = time.monotonic() + 5
                while not ready_path.is_file() and time.monotonic() < deadline:
                    time.sleep(0.05)
                ready = json.loads(ready_path.read_text(encoding="utf-8"))
                connection = http.client.HTTPConnection(
                    "127.0.0.1", ready["port"], timeout=5
                )
                connection.request(
                    "POST",
                    "/chat/completions",
                    body=json.dumps(
                        {
                            "model": "deepseek-v4-pro",
                            "stream": True,
                            "messages": [{"role": "user", "content": "read fixture"}],
                        }
                    ).encode("utf-8"),
                    headers={"Content-Type": "application/json"},
                )
                response = connection.getresponse().read().decode("utf-8")
                connection.close()
                self.assertIn("read_file", response)
                _stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
                result = json.loads(
                    (output / "provider-result.json").read_text(encoding="utf-8")
                )
                self.assertEqual(result["primary_request_count"], 1)
                self.assertFalse(result["continuity"]["second_request_observed"])
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)

    def test_tool_continuity_requires_two_requests_and_preserves_markers(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "provider"
            fixture = root / "tool-fixture.txt"
            fixture.write_text("LIF_TOOL_FIXTURE_CONTENT_001\n", encoding="utf-8")
            process = subprocess.Popen(
                [
                    sys.executable,
                    str(PROVIDER),
                    "--output-directory",
                    str(output),
                    "--timeout-seconds",
                    "10",
                    "--scenario",
                    "tool-continuity",
                    "--tool-fixture-path",
                    str(fixture),
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                ready_path = output / "ready.json"
                deadline = time.monotonic() + 5
                while not ready_path.is_file() and time.monotonic() < deadline:
                    time.sleep(0.05)
                ready = json.loads(ready_path.read_text(encoding="utf-8"))

                first = {
                    "model": "deepseek-v4-pro",
                    "stream": True,
                    "messages": [{"role": "user", "content": "read fixture"}],
                }
                connection = http.client.HTTPConnection(
                    "127.0.0.1", ready["port"], timeout=5
                )
                connection.request(
                    "POST",
                    "/chat/completions",
                    body=json.dumps(first).encode("utf-8"),
                    headers={"Content-Type": "application/json"},
                )
                first_response = connection.getresponse().read().decode("utf-8")
                connection.close()
                self.assertIn("LIF_FAKE_REASONING_CONTINUITY_001", first_response)
                self.assertIn("read_file", first_response)

                second = {
                    "model": "deepseek-v4-pro",
                    "stream": True,
                    "messages": [
                        {"role": "user", "content": "read fixture"},
                        {
                            "role": "assistant",
                            "reasoning_content": "LIF_FAKE_REASONING_CONTINUITY_001",
                            "tool_calls": [
                                {
                                    "id": "call_lif_read_fixture_001",
                                    "type": "function",
                                    "function": {
                                        "name": "read_file",
                                        "arguments": json.dumps(
                                            {"target_file": str(fixture)}
                                        ),
                                    },
                                }
                            ],
                        },
                        {
                            "role": "tool",
                            "tool_call_id": "call_lif_read_fixture_001",
                            "content": "1→LIF_TOOL_FIXTURE_CONTENT_001",
                        },
                    ],
                }
                connection = http.client.HTTPConnection(
                    "127.0.0.1", ready["port"], timeout=5
                )
                connection.request(
                    "POST",
                    "/chat/completions",
                    body=json.dumps(second).encode("utf-8"),
                    headers={"Content-Type": "application/json"},
                )
                second_response = connection.getresponse().read().decode("utf-8")
                connection.close()
                self.assertIn("LIF_FAKE_TOOL_CONTINUITY_OK", second_response)

                _stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
                result = json.loads(
                    (output / "provider-result.json").read_text(encoding="utf-8")
                )
                self.assertEqual(result["request_count"], 2)
                self.assertTrue(all(result["continuity"].values()))
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)

    def test_single_loopback_request_is_captured_with_authorization_redacted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "provider"
            process = subprocess.Popen(
                [
                    sys.executable,
                    str(PROVIDER),
                    "--output-directory",
                    str(output),
                    "--timeout-seconds",
                    "10",
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            try:
                ready_path = output / "ready.json"
                deadline = time.monotonic() + 5
                while not ready_path.is_file() and time.monotonic() < deadline:
                    if process.poll() is not None:
                        break
                    time.sleep(0.05)
                self.assertTrue(ready_path.is_file())
                ready = json.loads(ready_path.read_text(encoding="utf-8"))
                self.assertEqual(ready["host"], "127.0.0.1")
                self.assertFalse(ready["external_bind"])

                body = json.dumps(
                    {
                        "model": "deepseek-v4-pro",
                        "stream": True,
                        "messages": [{"role": "user", "content": "fixture-only"}],
                    }
                ).encode("utf-8")
                connection = http.client.HTTPConnection(
                    "127.0.0.1", ready["port"], timeout=5
                )
                connection.request(
                    "POST",
                    "/chat/completions",
                    body=body,
                    headers={
                        "Content-Type": "application/json",
                        "Authorization": "Bearer fake-unit-test-value",
                    },
                )
                response = connection.getresponse()
                response_body = response.read().decode("utf-8")
                connection.close()
                self.assertEqual(response.status, 200)
                self.assertIn("LIF_FAKE_PROVIDER_OK", response_body)

                stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 0, stderr)
                self.assertTrue(stdout.strip())
                result = json.loads(
                    (output / "provider-result.json").read_text(encoding="utf-8")
                )
                self.assertEqual(result["request_count"], 1)
                observed = result["requests"][0]
                self.assertEqual(observed["client_ip"], "127.0.0.1")
                self.assertTrue(observed["authorization_present"])
                self.assertFalse(observed["authorization_value_recorded"])
                private = (output / "requests.private.jsonl").read_text(
                    encoding="utf-8"
                )
                self.assertNotIn("fake-unit-test-value", private)
                self.assertIn("fixture-only", private)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)

    def test_existing_output_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            completed = subprocess.run(
                [
                    sys.executable,
                    str(PROVIDER),
                    "--output-directory",
                    temporary,
                    "--timeout-seconds",
                    "1",
                ],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=5,
                check=False,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("refusing to overwrite", completed.stderr)


if __name__ == "__main__":
    unittest.main()
