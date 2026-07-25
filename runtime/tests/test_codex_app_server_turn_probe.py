from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
PROBE = ROOT / "scripts" / "probe_codex_app_server_turn_lifecycle.py"
PROBE_VERIFIER = (
    ROOT / "scripts" / "verify_codex_app_server_turn_probe.py"
)
NORMALIZER = ROOT / "scripts" / "normalize_codex_app_server_lifecycle.py"
NORMALIZATION_VERIFIER = (
    ROOT / "scripts" / "verify_codex_app_server_lifecycle.py"
)
FAKE_SERVER = ROOT / "runtime" / "fixtures" / "fake_codex_app_server.py"


class CodexAppServerTurnProbeTests(unittest.TestCase):
    def _run_probe(
        self,
        root: Path,
        mode: str = "turn-success",
        *,
        expected: int = 0,
    ) -> tuple[Path, Path, Path, subprocess.CompletedProcess[str], dict]:
        capture = root / "capture.jsonl"
        stderr = root / "app-server.stderr"
        receipt = root / "turn-probe.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(PROBE),
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
                "5",
                "--shutdown-grace-seconds",
                "0.5",
            ],
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=15,
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

    def _verify(
        self,
        root: Path,
        capture: Path,
        stderr: Path,
        receipt: Path,
        *,
        expected: int = 0,
    ) -> dict:
        output = root / "turn-probe-verification.json"
        completed = subprocess.run(
            [
                sys.executable,
                str(PROBE_VERIFIER),
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
        self.assertEqual(completed.returncode, expected, completed.stderr)
        return json.loads(output.read_text(encoding="utf-8"))

    def test_success_probe_is_verified_and_normalized(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt_path, _, receipt = self._run_probe(root)
            self.assertTrue(receipt["valid"])
            self.assertEqual(receipt["provider"]["request_count"], 1)
            self.assertEqual(
                receipt["provider"]["base_url"],
                "http://127.0.0.1:<ephemeral>/v1",
            )
            self.assertEqual(receipt["lifecycle"]["terminal_status"], "completed")
            self.assertEqual(receipt["record_count"], 10)

            verification = self._verify(
                root, capture, stderr, receipt_path
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
                    "RUN-CODEX-TURN-PROBE-FAKE",
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
            self.assertEqual(result["observation_count"], 3)
            self.assertEqual(
                result["observation_breakdown"],
                {
                    "session_started": 1,
                    "turn_completed": 1,
                    "turn_started": 1,
                },
            )

            normalization_verification = root / "normalization-verification.json"
            replayed = subprocess.run(
                [
                    sys.executable,
                    str(NORMALIZATION_VERIFIER),
                    "--capture",
                    str(capture),
                    "--observations",
                    str(observations),
                    "--receipt",
                    str(normalization),
                    "--output",
                    str(normalization_verification),
                ],
                cwd=ROOT,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
                check=False,
            )
            self.assertEqual(replayed.returncode, 0, replayed.stderr)
            replay_report = json.loads(
                normalization_verification.read_text(encoding="utf-8")
            )
            self.assertTrue(replay_report["valid"])
            self.assertTrue(all(replay_report["checks"].values()))

    def test_invalid_provider_exchange_is_receipted_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            _, _, _, _, receipt = self._run_probe(
                Path(temporary),
                mode="turn-provider-invalid",
                expected=2,
            )
            self.assertFalse(receipt["valid"])
            self.assertEqual(receipt["probe_status"], "failed")
            self.assertEqual(receipt["error_kind"], "protocol_error")
            self.assertEqual(receipt["lifecycle"]["terminal_status"], "failed")

    def test_capture_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt, _, _ = self._run_probe(root)
            capture.write_bytes(capture.read_bytes() + b"\n")
            report = self._verify(
                root, capture, stderr, receipt, expected=2
            )
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["capture_artifact_matches"])

    def test_provider_projection_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt_path, _, receipt = self._run_probe(root)
            receipt["provider"]["request"]["path"] = "/v1/other"
            receipt_path.write_text(
                json.dumps(receipt, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
            report = self._verify(
                root, capture, stderr, receipt_path, expected=2
            )
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["provider_request_valid"])

    def test_config_tampering_is_detected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture, stderr, receipt_path, _, receipt = self._run_probe(root)
            config = (
                Path(receipt["isolated_state_directory"]) / "config.toml"
            )
            config.write_text(
                config.read_text(encoding="utf-8").replace(
                    'web_search = "disabled"', 'web_search = "live"'
                ),
                encoding="utf-8",
            )
            report = self._verify(
                root, capture, stderr, receipt_path, expected=2
            )
            self.assertFalse(report["valid"])
            self.assertFalse(report["checks"]["config_artifact_matches"])
            self.assertFalse(report["checks"]["loopback_provider_config_valid"])

    def test_existing_output_is_rejected_before_spawn(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            capture = root / "capture.jsonl"
            capture.write_text("sentinel\n", encoding="utf-8")
            completed = subprocess.run(
                [
                    sys.executable,
                    str(PROBE),
                    "--executable",
                    sys.executable,
                    "--app-server-arg",
                    str(FAKE_SERVER),
                    "--app-server-arg",
                    "turn-success",
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
                    "--isolated-state-dir",
                    str(root / "codex-home"),
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
