from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
NORMALIZER = ROOT / "scripts" / "normalize_codex_app_server_lifecycle.py"
VERIFIER = ROOT / "scripts" / "verify_codex_app_server_lifecycle.py"
ADAPTER = ROOT / "scripts" / "append_cli_session_lifecycle_event.py"
ADAPTER_VERIFIER = ROOT / "scripts" / "verify_cli_session_lifecycle_receipt.py"
RUN_ID = "RUN-CODEX-NORMALIZER-001"
MANIFEST_SHA256 = "a" * 64
STREAM_ID = "CLISTREAM-CODEX-FIXTURE-001"
FIXTURE_ROOT = ROOT / "runtime" / "fixtures" / "codex-app-server-lifecycle-v0.1"


class CodexAppServerLifecycleNormalizerTests(unittest.TestCase):
    def _run(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, *args],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=30,
            check=False,
        )

    def _record(self, sequence: int, message: dict) -> dict:
        direction = (
            "client_to_server"
            if message.get("method") in {"turn/start", "turn/interrupt"}
            and "id" in message
            else "server_to_client"
        )
        return {
            "schema_version": "0.1.0",
            "artifact_kind": "codex-app-server-capture-record",
            "source_stream_id": STREAM_ID,
            "source_record_sequence": sequence,
            "received_at": f"2026-07-25T16:{sequence:02d}:00Z",
            "direction": direction,
            "message": message,
        }

    def _write_capture(
        self,
        path: Path,
        messages: list[dict],
        *,
        sequences: list[int] | None = None,
    ) -> None:
        records = [
            self._record(
                sequences[index] if sequences is not None else index,
                message,
            )
            for index, message in enumerate(messages)
        ]
        path.write_text(
            "".join(
                json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n"
                for record in records
            ),
            encoding="utf-8",
        )

    def _normalize(
        self,
        root: Path,
        *,
        messages: list[dict] | None = None,
        sequences: list[int] | None = None,
        capture_name: str = "capture.jsonl",
        expected: int = 0,
    ) -> tuple[Path, Path, Path, dict]:
        capture = root / capture_name
        observations = root / "observations.jsonl"
        receipt = root / "normalization.json"
        if messages is not None:
            self._write_capture(capture, messages, sequences=sequences)
        completed = self._run(
            str(NORMALIZER),
            "--capture",
            str(capture),
            "--runtime-version",
            "0.0.0-fixture",
            "--run-id",
            RUN_ID,
            "--run-manifest-sha256",
            MANIFEST_SHA256,
            "--observations",
            str(observations),
            "--receipt",
            str(receipt),
        )
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return (
            capture,
            observations,
            receipt,
            json.loads(receipt.read_text(encoding="utf-8")),
        )

    @staticmethod
    def _thread_started(thread_id: str = "thr_fixture") -> dict:
        return {
            "method": "thread/started",
            "params": {
                "thread": {
                    "id": thread_id,
                    "status": {"type": "idle"},
                    "turns": [],
                }
            },
        }

    @staticmethod
    def _turn_started(turn_id: str) -> dict:
        return {
            "method": "turn/started",
            "params": {
                "turn": {
                    "id": turn_id,
                    "items": [],
                    "status": "inProgress",
                    "error": None,
                }
            },
        }

    @staticmethod
    def _turn_start_request(request_id: int, thread_id: str = "thr_fixture") -> dict:
        return {
            "method": "turn/start",
            "id": request_id,
            "params": {
                "threadId": thread_id,
                "input": [{"type": "text", "text": "fixture input"}],
            },
        }

    @staticmethod
    def _turn_start_response(request_id: int, turn_id: str) -> dict:
        return {
            "id": request_id,
            "result": {
                "turn": {
                    "id": turn_id,
                    "items": [],
                    "status": "inProgress",
                    "error": None,
                }
            },
        }

    @staticmethod
    def _turn_completed(
        turn_id: str,
        status: str,
        *,
        error: dict | None = None,
    ) -> dict:
        return {
            "method": "turn/completed",
            "params": {
                "turn": {
                    "id": turn_id,
                    "items": [],
                    "status": status,
                    "error": error,
                }
            },
        }

    @staticmethod
    def _thread_closed(thread_id: str = "thr_fixture") -> dict:
        return {
            "method": "thread/closed",
            "params": {"threadId": thread_id},
        }

    def test_happy_path_normalizes_and_independently_verifies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            messages = [
                {"id": 1, "result": {"thread": {"id": "thr_fixture"}}},
                self._thread_started(),
                self._turn_start_request(2),
                self._turn_start_response(2, "turn_1"),
                self._turn_started("turn_1"),
                {
                    "method": "item/completed",
                    "params": {
                        "threadId": "thr_fixture",
                        "turnId": "turn_1",
                        "item": {
                            "id": "item_1",
                            "type": "agentMessage",
                            "text": "PRIVATE-FIXTURE-CONTENT",
                        },
                    },
                },
                self._turn_completed("turn_1", "completed"),
                self._thread_closed(),
            ]
            capture, observations, receipt_path, receipt = self._normalize(
                root, messages=messages
            )
            self.assertTrue(receipt["valid"])
            self.assertEqual(receipt["normalization_status"], "complete")
            self.assertEqual(receipt["record_count"], 8)
            self.assertEqual(receipt["ignored_record_count"], 4)
            self.assertEqual(receipt["observation_count"], 4)
            self.assertEqual(receipt["lifecycle_state"], "terminal")
            observation_values = [
                json.loads(line)
                for line in observations.read_text(encoding="utf-8").splitlines()
            ]
            self.assertEqual(
                [item["event_kind"] for item in observation_values],
                [
                    "session_started",
                    "turn_started",
                    "turn_completed",
                    "session_completed",
                ],
            )
            self.assertNotIn(
                "PRIVATE-FIXTURE-CONTENT",
                observations.read_text(encoding="utf-8"),
            )

            verification = root / "verification.json"
            completed = self._run(
                str(VERIFIER),
                "--capture",
                str(capture),
                "--observations",
                str(observations),
                "--receipt",
                str(receipt_path),
                "--output",
                str(verification),
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            report = json.loads(verification.read_text(encoding="utf-8"))
            self.assertTrue(report["valid"])
            self.assertTrue(all(report["checks"].values()))

    def test_repository_happy_fixture_is_replayable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture = FIXTURE_ROOT / "happy.capture.jsonl"
            observations = root / "observations.jsonl"
            receipt = root / "normalization.json"
            normalized = self._run(
                str(NORMALIZER),
                "--capture",
                str(capture),
                "--runtime-version",
                "0.0.0-static-fixture",
                "--run-id",
                RUN_ID,
                "--run-manifest-sha256",
                MANIFEST_SHA256,
                "--observations",
                str(observations),
                "--receipt",
                str(receipt),
            )
            self.assertEqual(normalized.returncode, 0, normalized.stderr)
            verification = root / "verification.json"
            verified = self._run(
                str(VERIFIER),
                "--capture",
                str(capture),
                "--observations",
                str(observations),
                "--receipt",
                str(receipt),
                "--output",
                str(verification),
            )
            self.assertEqual(verified.returncode, 0, verified.stderr)
            report = json.loads(verification.read_text(encoding="utf-8"))
            self.assertTrue(report["valid"])

    def test_interrupted_turn_allows_second_turn_and_stays_partial(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            messages = [
                self._thread_started(),
                self._turn_start_request(1),
                self._turn_start_response(1, "turn_1"),
                self._turn_started("turn_1"),
                self._turn_completed("turn_1", "interrupted"),
                self._turn_start_request(2),
                self._turn_start_response(2, "turn_2"),
                self._turn_started("turn_2"),
                self._turn_completed("turn_2", "completed"),
            ]
            _, observations, _, receipt = self._normalize(root, messages=messages)
            self.assertEqual(receipt["normalization_status"], "partial")
            self.assertEqual(receipt["lifecycle_state"], "active")
            values = [
                json.loads(line)
                for line in observations.read_text(encoding="utf-8").splitlines()
            ]
            terminals = [
                item["turn_status"]
                for item in values
                if item["event_kind"] == "turn_completed"
            ]
            self.assertEqual(terminals, ["interrupted", "completed"])

    def test_failed_turn_keeps_only_error_digest(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            error = {
                "message": "PRIVATE-UPSTREAM-ERROR",
                "codexErrorInfo": "InternalServerError",
            }
            messages = [
                self._thread_started(),
                self._turn_start_request(1),
                self._turn_start_response(1, "turn_1"),
                self._turn_started("turn_1"),
                self._turn_completed("turn_1", "failed", error=error),
            ]
            _, observations, _, receipt = self._normalize(root, messages=messages)
            self.assertEqual(receipt["lifecycle_state"], "active")
            values = [
                json.loads(line)
                for line in observations.read_text(encoding="utf-8").splitlines()
            ]
            terminal = values[-1]
            expected_error_sha256 = hashlib.sha256(
                json.dumps(
                    error,
                    ensure_ascii=False,
                    sort_keys=True,
                    separators=(",", ":"),
                    allow_nan=False,
                ).encode("utf-8")
            ).hexdigest()
            self.assertEqual(terminal["turn_status"], "failed")
            self.assertEqual(terminal["error_sha256"], expected_error_sha256)
            self.assertNotIn(
                "PRIVATE-UPSTREAM-ERROR",
                observations.read_text(encoding="utf-8"),
            )

    def test_interrupt_request_without_terminal_remains_in_turn_partial(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            messages = [
                self._thread_started(),
                self._turn_start_request(1),
                self._turn_start_response(1, "turn_1"),
                self._turn_started("turn_1"),
                {
                    "method": "turn/interrupt",
                    "id": 31,
                    "params": {
                        "threadId": "thr_fixture",
                        "turnId": "turn_1",
                    },
                },
                {"id": 31, "result": {}},
            ]
            _, observations, _, receipt = self._normalize(root, messages=messages)
            self.assertEqual(receipt["normalization_status"], "partial")
            self.assertEqual(receipt["lifecycle_state"], "in_turn")
            self.assertEqual(receipt["active_turn_id"], "turn_1")
            self.assertEqual(len(observations.read_text(encoding="utf-8").splitlines()), 2)

    def test_bad_sequence_cross_thread_and_truncated_json_fail_closed(self) -> None:
        cases: list[tuple[str, list[dict] | None, list[int] | None, str | None]] = [
            (
                "sequence",
                [
                    self._thread_started(),
                    self._turn_start_request(1),
                ],
                [0, 0],
                None,
            ),
            (
                "cross-thread",
                [self._thread_started(), self._thread_closed("thr_other")],
                None,
                None,
            ),
            (
                "unbound-turn",
                [self._thread_started(), self._turn_started("turn_unbound")],
                None,
                None,
            ),
            (
                "cross-thread-turn-request",
                [
                    self._thread_started(),
                    self._turn_start_request(1, thread_id="thr_other"),
                ],
                None,
                None,
            ),
            ("truncated", None, None, '{"schema_version":"0.1.0"'),
        ]
        for name, messages, sequences, raw in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                capture = root / "capture.jsonl"
                if raw is not None:
                    capture.write_text(raw, encoding="utf-8")
                _, observations, _, receipt = self._normalize(
                    root,
                    messages=messages,
                    sequences=sequences,
                    expected=2,
                )
                self.assertFalse(receipt["valid"])
                self.assertEqual(receipt["normalization_status"], "rejected")
                self.assertTrue(receipt["errors"])
                self.assertFalse(observations.exists())

    def test_verifier_detects_source_and_observation_tampering(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, observations, receipt, _ = self._normalize(
                root,
                messages=[
                    self._thread_started(),
                    self._turn_start_request(1),
                    self._turn_start_response(1, "turn_1"),
                    self._turn_started("turn_1"),
                    self._turn_completed("turn_1", "completed"),
                ],
            )
            observations.write_text(
                observations.read_text(encoding="utf-8") + "{}\n",
                encoding="utf-8",
            )
            verification = root / "tampered-observations.json"
            completed = self._run(
                str(VERIFIER),
                "--capture",
                str(capture),
                "--observations",
                str(observations),
                "--receipt",
                str(receipt),
                "--output",
                str(verification),
            )
            self.assertEqual(completed.returncode, 2, completed.stderr)
            report = json.loads(verification.read_text(encoding="utf-8"))
            self.assertFalse(report["checks"]["observations_artifact_matches"])
            self.assertFalse(report["checks"]["observation_content_matches"])

    def test_normalized_observations_feed_canonical_adapter_chain(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _, observations, _, _ = self._normalize(
                root,
                messages=[
                    self._thread_started(),
                    self._turn_start_request(1),
                    self._turn_start_response(1, "turn_1"),
                    self._turn_started("turn_1"),
                    self._turn_completed("turn_1", "completed"),
                    self._thread_closed(),
                ],
            )
            journal = root / "events.jsonl"
            canonical_types: list[str] = []
            for index, line in enumerate(
                observations.read_text(encoding="utf-8").splitlines()
            ):
                observation_path = root / f"observation-{index}.json"
                receipt_path = root / f"adapter-{index}.json"
                observation_path.write_text(line + "\n", encoding="utf-8")
                completed = self._run(
                    str(ADAPTER),
                    "--observation",
                    str(observation_path),
                    "--journal",
                    str(journal),
                    "--output",
                    str(receipt_path),
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                adapter_receipt = json.loads(
                    receipt_path.read_text(encoding="utf-8")
                )
                canonical_types.append(adapter_receipt["canonical_event_type"])
                verification_path = root / f"adapter-{index}.verification.json"
                verified = self._run(
                    str(ADAPTER_VERIFIER),
                    "--observation",
                    str(observation_path),
                    "--journal",
                    str(journal),
                    "--receipt",
                    str(receipt_path),
                    "--output",
                    str(verification_path),
                )
                self.assertEqual(verified.returncode, 0, verified.stderr)
            self.assertEqual(
                canonical_types,
                ["run_started", "model_request", "model_output", "run_finished"],
            )


if __name__ == "__main__":
    unittest.main()
