from __future__ import annotations

from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from assurance.errors import AssuranceError
from assurance.grok_runtime_adapter import (
    GrokRunRequest,
    GrokRuntimeConfig,
    run_grok_headless_once,
    validate_grok_retrieval_mode,
)


ROOT = Path(__file__).resolve().parents[2]


class _FakeProcess:
    def __init__(
        self,
        args: list[str],
        *,
        stdout,
        stderr,
        returncode: int | None = 0,
        version_output: str = "grok 0.2.112 (9bbd559437) [stable]\n",
    ) -> None:
        self.args = args
        self.pid = 4242
        self.returncode = returncode
        self._returncode = returncode
        stdout.write(version_output.encode("utf-8"))
        stderr.write(b"")

    def wait(self, timeout: int | None = None) -> int:
        if self._returncode is None:
            raise subprocess.TimeoutExpired(self.args, timeout)
        self.returncode = self._returncode
        return self.returncode

    def poll(self) -> int | None:
        return self.returncode


def _inspection(binary: Path) -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "valid": True,
        "binary_path": str(binary),
        "observed": {
            "sha256": "a" * 64,
            "version_output": "grok 0.2.112 (9bbd559437) [stable]",
        },
    }


def _trust() -> dict[str, object]:
    return {
        "schema_version": "0.1.0",
        "valid": True,
        "trust_granted": True,
        "discovery": {"aggregate_sha256": "b" * 64},
    }


class GrokRuntimeAdapterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(dir=ROOT)
        self.root = Path(self.temporary.name)
        self.workspace = self.root / "workspace"
        self.workspace.mkdir()
        self.binary = self.root / "grok.exe"
        self.binary.write_bytes(b"MZ")
        self.config = GrokRuntimeConfig(repo_root=ROOT, timeout_seconds=5)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def test_version_smoke_writes_valid_receipt_and_events(self) -> None:
        run_root = self.root / "run"

        def popen_factory(args, **kwargs):
            return _FakeProcess(args, stdout=kwargs["stdout"], stderr=kwargs["stderr"])

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()):
            receipt = run_grok_headless_once(
                GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["retrieval"]["mode"], "off")
        self.assertFalse(receipt["retrieval"]["active_for_current_mode"])
        self.assertTrue(receipt["containment"]["no_residue_required"])
        self.assertTrue(receipt["containment"]["no_residue_observed"])
        self.assertTrue((run_root / "grok-runtime-receipt.json").is_file())
        self.assertTrue((run_root / "events.jsonl").is_file())
        self.assertIn(
            "run_finished",
            (run_root / "events.jsonl").read_text(encoding="utf-8"),
        )

    def test_process_timeout_does_not_write_valid_receipt(self) -> None:
        run_root = self.root / "run-timeout"

        def popen_factory(args, **kwargs):
            return _FakeProcess(
                args,
                stdout=kwargs["stdout"],
                stderr=kwargs["stderr"],
                returncode=None,
            )

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()):
            with self.assertRaises(AssuranceError) as raised:
                run_grok_headless_once(
                    GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                    config=self.config,
                    popen_factory=popen_factory,
                )

        self.assertIn("did not exit", str(raised.exception))
        self.assertFalse((run_root / "grok-runtime-receipt.json").exists())

    def test_version_smoke_can_create_isolated_workspace_under_run_root(self) -> None:
        run_root = self.root / "run-isolated"
        isolated_workspace = run_root / "workspace"

        def popen_factory(args, **kwargs):
            return _FakeProcess(args, stdout=kwargs["stdout"], stderr=kwargs["stderr"])

        with patch(
            "assurance.grok_runtime_adapter.inspect_grok_runtime",
            return_value=_inspection(self.binary),
        ), patch("assurance.grok_runtime_adapter._workspace_trust", return_value=_trust()):
            receipt = run_grok_headless_once(
                GrokRunRequest(
                    run_root=run_root,
                    workspace_path=isolated_workspace,
                    retrieval_mode="local_browser",
                    retrieval_mode_explicit=True,
                ),
                config=self.config,
                popen_factory=popen_factory,
            )

        self.assertTrue(receipt["valid"])
        self.assertEqual(receipt["request"]["retrieval_mode"], "local_browser")
        self.assertTrue(receipt["request"]["retrieval_mode_explicit"])
        self.assertTrue(receipt["retrieval"]["runtime_tool_retrieval_allowed"])
        self.assertTrue(receipt["retrieval"]["assurance_receipts_required"])
        self.assertFalse(receipt["retrieval"]["active_for_current_mode"])
        self.assertTrue(isolated_workspace.is_dir())
        self.assertEqual(
            receipt["request"]["workspace_path"],
            str(isolated_workspace.resolve()),
        )

    def test_refuses_nonempty_run_root(self) -> None:
        run_root = self.root / "occupied"
        run_root.mkdir()
        (run_root / "old.txt").write_text("old", encoding="utf-8")

        with self.assertRaises(AssuranceError):
            run_grok_headless_once(
                GrokRunRequest(run_root=run_root, workspace_path=self.workspace),
                config=self.config,
            )

    def test_retrieval_mode_allowlist_is_closed(self) -> None:
        self.assertEqual(validate_grok_retrieval_mode("framework_fallback"), "framework_fallback")
        with self.assertRaises(AssuranceError):
            validate_grok_retrieval_mode("implicit_runtime_search")


if __name__ == "__main__":
    unittest.main()
