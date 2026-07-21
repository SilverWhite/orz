from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import uuid

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[3]
CAPTURE = ROOT / "scripts" / "capture_grok_fixture_workspace.py"
CHECKPOINT_SCHEMA = ROOT / "integration" / "grok" / "grok-fixture-workspace-checkpoint-v0.1.schema.json"
DELTA_SCHEMA = ROOT / "integration" / "grok" / "grok-fixture-workspace-delta-v0.1.schema.json"
MARKER = ".lif-disposable-workspace.json"


def _write_marker(workspace: Path, *, disposable: bool = True, workspace_id: str | None = None) -> str:
    identity = workspace_id or f"FIXTURE-{uuid.uuid4().hex}"
    (workspace / MARKER).write_text(
        json.dumps(
            {
                "schema_version": "0.1.0",
                "marker_kind": "lif-disposable-workspace",
                "workspace_id": identity,
                "disposable": disposable,
                "purpose": "unit-test-only",
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return identity


class FixtureWorkspaceCaptureTests(unittest.TestCase):
    def _run(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(CAPTURE), *arguments],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            timeout=20,
            check=False,
        )

    def test_checkpoint_and_delta_classify_hash_only_changes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            _write_marker(workspace)
            (workspace / "keep.txt").write_text("keep\n", encoding="utf-8")
            (workspace / "modify.txt").write_text("before-private-marker\n", encoding="utf-8")
            (workspace / "delete.txt").write_text("delete-private-marker\n", encoding="utf-8")
            checkpoint_path = root / "checkpoint.json"
            checkpoint_run = self._run(
                "checkpoint",
                "--workspace", str(workspace),
                "--output", str(checkpoint_path),
            )
            self.assertEqual(checkpoint_run.returncode, 0, checkpoint_run.stderr)
            checkpoint = json.loads(checkpoint_path.read_text(encoding="utf-8"))
            Draft202012Validator(
                json.loads(CHECKPOINT_SCHEMA.read_text(encoding="utf-8")),
                format_checker=FormatChecker(),
            ).validate(checkpoint)

            (workspace / "modify.txt").write_text("after-private-marker\n", encoding="utf-8")
            (workspace / "delete.txt").unlink()
            (workspace / "created.txt").write_text("created-private-marker\n", encoding="utf-8")
            delta_path = root / "delta.json"
            delta_run = self._run(
                "delta",
                "--workspace", str(workspace),
                "--checkpoint", str(checkpoint_path),
                "--output", str(delta_path),
            )
            self.assertEqual(delta_run.returncode, 0, delta_run.stderr)
            delta = json.loads(delta_path.read_text(encoding="utf-8"))
            Draft202012Validator(
                json.loads(DELTA_SCHEMA.read_text(encoding="utf-8")),
                format_checker=FormatChecker(),
            ).validate(delta)
            self.assertEqual(
                delta["changes"]["counts"],
                {"created": 1, "modified": 1, "deleted": 1, "unchanged": 2},
            )
            self.assertEqual(delta["changes"]["created"][0]["relative_path"], "created.txt")
            self.assertEqual(delta["changes"]["modified"][0]["relative_path"], "modify.txt")
            self.assertEqual(delta["changes"]["deleted"][0]["relative_path"], "delete.txt")
            combined = checkpoint_path.read_text(encoding="utf-8") + delta_path.read_text(encoding="utf-8")
            for private_marker in (
                "before-private-marker",
                "after-private-marker",
                "delete-private-marker",
                "created-private-marker",
            ):
                self.assertNotIn(private_marker, combined)

    def test_requires_explicit_disposable_marker(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            missing = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(root / "missing.json")
            )
            self.assertEqual(missing.returncode, 2)
            self.assertIn("missing regular disposable marker", missing.stderr)
            _write_marker(workspace, disposable=False)
            denied = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(root / "denied.json")
            )
            self.assertEqual(denied.returncode, 2)
            self.assertIn("disposable=true", denied.stderr)

    def test_refuses_overwrite_output_inside_workspace_and_marker_change(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            _write_marker(workspace)
            checkpoint_path = root / "checkpoint.json"
            first = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(checkpoint_path)
            )
            self.assertEqual(first.returncode, 0, first.stderr)
            second = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(checkpoint_path)
            )
            self.assertEqual(second.returncode, 2)
            self.assertIn("refusing to overwrite", second.stderr)
            inside = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(workspace / "inside.json")
            )
            self.assertEqual(inside.returncode, 2)
            self.assertIn("outside the disposable workspace", inside.stderr)
            marker_path = workspace / MARKER
            marker_path.write_text(marker_path.read_text(encoding="utf-8") + " ", encoding="utf-8")
            changed = self._run(
                "delta",
                "--workspace", str(workspace),
                "--checkpoint", str(checkpoint_path),
                "--output", str(root / "changed.json"),
            )
            self.assertEqual(changed.returncode, 2)
            self.assertIn("marker changed", changed.stderr)

    def test_refuses_reparse_entries_when_supported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            _write_marker(workspace)
            target = root / "target.txt"
            target.write_text("target\n", encoding="utf-8")
            link = workspace / "link.txt"
            try:
                os.symlink(target, link)
            except (OSError, NotImplementedError):
                self.skipTest("symlink creation is unavailable on this Windows host")
            completed = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(root / "checkpoint.json")
            )
            self.assertEqual(completed.returncode, 2)
            self.assertIn("reparse file", completed.stderr)

    def test_delta_recomputes_checkpoint_aggregate(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            workspace = root / "workspace"
            workspace.mkdir()
            _write_marker(workspace)
            checkpoint_path = root / "checkpoint.json"
            created = self._run(
                "checkpoint", "--workspace", str(workspace), "--output", str(checkpoint_path)
            )
            self.assertEqual(created.returncode, 0, created.stderr)
            checkpoint = json.loads(checkpoint_path.read_text(encoding="utf-8"))
            checkpoint["state"]["aggregate_sha256"] = "0" * 64
            checkpoint_path.write_text(json.dumps(checkpoint) + "\n", encoding="utf-8")
            delta = self._run(
                "delta",
                "--workspace", str(workspace),
                "--checkpoint", str(checkpoint_path),
                "--output", str(root / "delta.json"),
            )
            self.assertEqual(delta.returncode, 2)
            self.assertIn("checkpoint aggregate does not match", delta.stderr)


if __name__ == "__main__":
    unittest.main()
