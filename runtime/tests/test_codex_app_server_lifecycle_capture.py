from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
CAPTURE = ROOT / "scripts" / "capture_codex_app_server_lifecycle.py"
CAPTURE_VERIFIER = (
    ROOT / "scripts" / "verify_codex_app_server_lifecycle_capture.py"
)
NORMALIZER = ROOT / "scripts" / "normalize_codex_app_server_lifecycle.py"
FAKE_SERVER = ROOT / "runtime" / "fixtures" / "fake_codex_app_server.py"


class CodexAppServerLifecycleCaptureTests(unittest.TestCase):
    def _run_capture(
        self,
        root: Path,
        mode: str,
        *,
        timeout_seconds: float = 3.0,
        expected: int = 0,
    ) -> tuple[Path, Path, Path, subprocess.CompletedProcess[str], dict]:
        capture = root / "capture.jsonl"
        stderr = root / "app-server.stderr"
        receipt = root / "capture-receipt.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(CAPTURE),
                "--executable",
                sys.executable,
                "--app-server-arg",
                str(FAKE_SERVER),
                "--app-server-arg",
                mode,
                "--cwd",
                str(ROOT),
                "--runtime-version",
                "0.0.0-fake",
                "--capture",
                str(capture),
                "--stderr",
                str(stderr),
                "--receipt",
                str(receipt),
                "--isolated-state-dir",
                str(root / "codex-home"),
                "--timeout-seconds",
                str(timeout_seconds),
                "--shutdown-grace-seconds",
                "0.5",
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=10,
            check=False,
        )
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return (
            capture,
            stderr,
            receipt,
            completed,
            json.loads(receipt.read_text(encoding="utf-8")),
        )

    def test_success_capture_is_no_model_partial_lifecycle(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt_path, _, receipt = self._run_capture(
                root, "success"
            )
            self.assertTrue(receipt["valid"])
            self.assertEqual(receipt["capture_status"], "captured")
            self.assertTrue(receipt["containment_assigned"])
            self.assertEqual(receipt["handshake"]["thread_id"], "thr_fake_capture")
            self.assertEqual(receipt["record_count"], 6)
            records = [
                json.loads(line)
                for line in capture.read_text(encoding="utf-8").splitlines()
            ]
            client_methods = [
                record["message"].get("method")
                for record in records
                if record["direction"] == "client_to_server"
            ]
            self.assertEqual(
                client_methods, ["initialize", "initialized", "thread/start"]
            )
            self.assertNotIn("turn/start", client_methods)
            self.assertNotIn('"input"', capture.read_text(encoding="utf-8"))
            self.assertEqual(
                stderr.read_text(encoding="utf-8"),
                "fake app-server diagnostic\n",
            )
            self.assertNotIn(
                "fake app-server diagnostic",
                receipt_path.read_text(encoding="utf-8"),
            )

            capture_verification = root / "capture-verification.json"
            verified = subprocess.run(
                [
                    sys.executable,
                    str(CAPTURE_VERIFIER),
                    "--capture",
                    str(capture),
                    "--stderr",
                    str(stderr),
                    "--receipt",
                    str(receipt_path),
                    "--output",
                    str(capture_verification),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(verified.returncode, 0, verified.stderr)
            verification = json.loads(
                capture_verification.read_text(encoding="utf-8")
            )
            self.assertTrue(verification["valid"])
            self.assertTrue(all(verification["checks"].values()))

            observations = root / "observations.jsonl"
            normalization = root / "normalization.json"
            normalized = subprocess.run(
                [
                    sys.executable,
                    str(NORMALIZER),
                    "--capture",
                    str(capture),
                    "--runtime-version",
                    "0.0.0-fake",
                    "--run-id",
                    "RUN-CODEX-CAPTURE-FAKE",
                    "--run-manifest-sha256",
                    "a" * 64,
                    "--observations",
                    str(observations),
                    "--receipt",
                    str(normalization),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(normalized.returncode, 0, normalized.stderr)
            result = json.loads(normalization.read_text(encoding="utf-8"))
            self.assertTrue(result["valid"])
            self.assertEqual(result["normalization_status"], "partial")
            self.assertEqual(result["lifecycle_state"], "active")
            self.assertEqual(result["observation_count"], 1)

    def test_notification_before_response_is_correlated(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            _, _, _, _, receipt = self._run_capture(
                Path(temporary), "notification-first"
            )
            self.assertTrue(receipt["valid"])
            self.assertEqual(receipt["handshake"]["thread_id"], "thr_fake_capture")

    def test_timeout_is_bounded_and_receipted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            _, _, _, _, receipt = self._run_capture(
                Path(temporary),
                "timeout",
                timeout_seconds=0.2,
                expected=2,
            )
            self.assertFalse(receipt["valid"])
            self.assertEqual(receipt["capture_status"], "failed")
            self.assertEqual(receipt["error_kind"], "timeout")
            self.assertIsNotNone(receipt["error_sha256"])

    def test_malformed_stdout_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            _, _, _, _, receipt = self._run_capture(
                Path(temporary), "malformed", expected=2
            )
            self.assertFalse(receipt["valid"])
            self.assertEqual(receipt["error_kind"], "invalid_stdout")

    def test_thread_identity_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            _, _, _, _, receipt = self._run_capture(
                Path(temporary), "mismatch", expected=2
            )
            self.assertFalse(receipt["valid"])
            self.assertEqual(receipt["error_kind"], "protocol_error")
            self.assertIsNone(receipt["handshake"]["thread_id"])

    def test_capture_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt, _, _ = self._run_capture(root, "success")
            capture.write_bytes(capture.read_bytes() + b"\n")
            output = root / "verification.json"
            verified = subprocess.run(
                [
                    sys.executable,
                    str(CAPTURE_VERIFIER),
                    "--capture",
                    str(capture),
                    "--stderr",
                    str(stderr),
                    "--receipt",
                    str(receipt),
                    "--output",
                    str(output),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(verified.returncode, 2, verified.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["capture_artifact_matches"])

    def test_stderr_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt, _, _ = self._run_capture(root, "success")
            stderr.write_bytes(stderr.read_bytes() + b"tampered\n")
            output = root / "verification.json"
            verified = subprocess.run(
                [
                    sys.executable,
                    str(CAPTURE_VERIFIER),
                    "--capture",
                    str(capture),
                    "--stderr",
                    str(stderr),
                    "--receipt",
                    str(receipt),
                    "--output",
                    str(output),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(verified.returncode, 2, verified.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["stderr_artifact_matches"])

    def test_existing_output_is_rejected_before_spawn(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture = root / "capture.jsonl"
            capture.write_text("sentinel\n", encoding="utf-8")
            completed = subprocess.run(
                [
                    sys.executable,
                    str(CAPTURE),
                    "--executable",
                    sys.executable,
                    "--app-server-arg",
                    str(FAKE_SERVER),
                    "--app-server-arg",
                    "success",
                    "--cwd",
                    str(ROOT),
                    "--runtime-version",
                    "0.0.0-fake",
                    "--capture",
                    str(capture),
                    "--stderr",
                    str(root / "stderr.txt"),
                    "--receipt",
                    str(root / "receipt.json"),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(completed.returncode, 1)
            self.assertEqual(capture.read_text(encoding="utf-8"), "sentinel\n")
            self.assertFalse((root / "receipt.json").exists())


if __name__ == "__main__":
    unittest.main()
