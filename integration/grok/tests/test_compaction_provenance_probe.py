from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import jsonschema


ROOT = Path(__file__).resolve().parents[3]
VERIFIER = ROOT / "scripts" / "verify_grok_compaction_provenance_probe.py"
RESULT_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-compaction-provenance-probe-result-v0.1.schema.json"
)
VERIFICATION_SCHEMA = (
    ROOT
    / "integration"
    / "grok"
    / "grok-compaction-provenance-probe-verification-v0.1.schema.json"
)
RETAINED = "LIF_COMPACTION_SOURCE_RETAINED_001"
OMITTED = "LIF_COMPACTION_SOURCE_OMITTED_001"
SUMMARY_MARKER = "LIF_COMPACTION_DERIVED_SUMMARY_001"
POST_MARKER = "LIF_COMPACTION_POST_RESPONSE_001"


def _sha(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _canonical(value: object) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def _write_json(path: Path, value: object) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def _write_jsonl(path: Path, values: list[dict[str, object]]) -> None:
    path.write_text(
        "".join(json.dumps(value, sort_keys=True) + "\n" for value in values),
        encoding="utf-8",
    )


def _artifact(path: Path, classification: str = "fake-fixture-private") -> dict[str, object]:
    content = path.read_bytes()
    return {
        "path": str(path),
        "bytes": len(content),
        "sha256": _sha(content),
        "classification": classification,
    }


class CompactionProvenanceVerifierTests(unittest.TestCase):
    def _fixture(self, root: Path) -> tuple[Path, Path, dict[str, object]]:
        binary_sha = "a" * 64
        lock = root / "lock.json"
        _write_json(
            lock,
            {
                "binary_release": {
                    "sha256": binary_sha,
                    "version": "0.2.111",
                    "build_id": "94172f2aa4",
                }
            },
        )
        history = [
            {"role": "system", "content": "fixture system"},
            {"role": "user", "content": f"{RETAINED} {OMITTED}"},
            {"role": "assistant", "content": "fixture response"},
        ]
        summary = f"{SUMMARY_MARKER}; retained: {RETAINED}"
        request_path = root / "compaction-request.json"
        _write_json(
            request_path,
            {
                "request_id": "request-fixture",
                "trigger": "manual",
                "chat_history": history,
                "summary": summary,
            },
        )
        checkpoint_path = root / "checkpoint.json"
        _write_json(
            checkpoint_path,
            {
                "checkpoint_id": "checkpoint-fixture",
                "prompt_index_at_compaction": 1,
                "summary": summary,
            },
        )
        hooks_path = root / "hooks.jsonl"
        _write_jsonl(
            hooks_path,
            [
                {
                    "hook_event_name": "pre_compact",
                    "source": "manual",
                    "session_id": "session-fixture",
                },
                {
                    "hook_event_name": "post_compact",
                    "source": "manual",
                    "session_id": "session-fixture",
                },
            ],
        )
        first_request = {
            "body": {
                "model": "deepseek-v4-pro",
                "messages": [{"role": "user", "content": f"{RETAINED} {OMITTED}"}],
            }
        }
        source_path = root / "source.json"
        _write_json(source_path, first_request)
        provider_path = root / "provider.jsonl"
        _write_jsonl(
            provider_path,
            [
                first_request,
                {"body": {"model": "deepseek-v4-pro", "messages": history}},
                {
                    "body": {
                        "model": "deepseek-v4-pro",
                        "messages": [{"role": "user", "content": "post"}],
                    }
                },
            ],
        )
        provider_result_path = root / "provider-result.json"
        _write_json(
            provider_result_path,
            {
                "primary_request_count": 3,
                "response": {"marker": POST_MARKER, "real_model_invoked": False},
            },
        )
        artifacts: dict[str, dict[str, object]] = {
            "hook_receipts": _artifact(hooks_path),
            "source_snapshot": _artifact(source_path),
            "compaction_request": _artifact(request_path),
            "compaction_checkpoint": _artifact(checkpoint_path),
            "provider_capture": _artifact(provider_path),
            "provider_result": _artifact(provider_result_path, "fake-fixture"),
        }
        for name in ("config", "hook_config", "session_events", "session_updates"):
            path = root / f"{name}.json"
            _write_json(path, {"fixture": name})
            artifacts[name] = _artifact(
                path,
                "fake-fixture" if name in {"config", "hook_config"} else "fake-fixture-private",
            )
        result: dict[str, object] = {
            "schema_version": "0.1.0",
            "probe_id": "COMPACT-" + "b" * 32,
            "terminal_state": "succeeded",
            "started_at_unix_ns": 1,
            "finished_at_unix_ns": 2,
            "release": {
                "metadata_path": str(lock),
                "binary_path": str(root / "grok.exe"),
                "binary_sha256": binary_sha,
                "version": "0.2.111",
                "build_id": "94172f2aa4",
                "inspection_valid": True,
            },
            "session": {
                "session_id": "session-fixture",
                "workspace": str(root),
                "profile": str(root),
                "trigger": "manual",
                "model_alias": "lif-fake-deepseek",
                "provider_model": "deepseek-v4-pro",
                "request_count": 3,
                "first_prompt_stop_reason": "end_turn",
                "compact_stop_reason": "end_turn",
                "post_prompt_stop_reason": "end_turn",
                "event_count": 1,
                "update_count": 1,
            },
            "lifecycle": {
                "pre_compact_observed": True,
                "post_compact_observed": True,
                "ordered_events": ["pre_compact", "post_compact"],
                "sources": ["manual", "manual"],
                "session_ids_match": True,
                "manual_trigger_only": True,
            },
            "provenance": {
                "source_span": {
                    "kind": "compaction_request.chat_history",
                    "item_count": len(history),
                    "first_index": 0,
                    "last_index": len(history) - 1,
                    "canonical_sha256": _sha(_canonical(history)),
                    "recorded_before_summary_replacement": True,
                },
                "boundary": {
                    "request_id": "request-fixture",
                    "trigger": "manual",
                    "checkpoint_id": "checkpoint-fixture",
                    "prompt_index_at_compaction": 1,
                },
                "summary": {
                    "status": "derived_unverified",
                    "chars": len(summary),
                    "sha256": _sha(summary.encode()),
                    "may_replace_source_evidence": False,
                },
                "ranges": {
                    "summarized": [
                        {
                            "source": "compaction_request.chat_history",
                            "first_index": 0,
                            "last_index": len(history) - 1,
                            "basis": "direct_request_artifact",
                        }
                    ],
                    "retained": [],
                    "discarded": [],
                    "unknown": [
                        {
                            "scope": "source-index retention/discard mapping",
                            "reason": "not directly recorded",
                        }
                    ],
                },
            },
            "canary_checks": {
                "retained_marker_in_source": True,
                "omitted_marker_in_source": True,
                "summary_marker_in_summary": True,
                "retained_marker_in_summary": True,
                "omitted_marker_absent_from_summary": True,
                "summary_marker_in_checkpoint": True,
                "post_response_marker_observed": True,
                "source_snapshot_immutable": True,
            },
            "containment": {
                "loopback_only_provider": True,
                "firewall_added": True,
                "job_created": True,
                "job_assigned": True,
                "real_model_invoked": False,
            },
            "artifacts": artifacts,
            "limitations": [
                "fixed fixture",
                "manual trigger",
                "derived summary",
                "unknown mapping",
            ],
        }
        result_path = root / "result.json"
        _write_json(result_path, result)
        return result_path, lock, result

    def _run_verifier(self, result: Path, lock: Path, output: Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(VERIFIER),
                "--result",
                str(result),
                "--output",
                str(output),
                "--lock",
                str(lock),
            ],
            cwd=ROOT,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )

    def test_fixed_fixture_passes_and_matches_schemas(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, lock, result = self._fixture(root)
            output = root / "verification.json"
            completed = self._run_verifier(result_path, lock, output)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            verification = json.loads(output.read_text(encoding="utf-8"))
            self.assertTrue(verification["passed"])
            self.assertTrue(all(verification["checks"].values()))
            jsonschema.validate(
                result,
                json.loads(RESULT_SCHEMA.read_text(encoding="utf-8")),
            )
            jsonschema.validate(
                verification,
                json.loads(VERIFICATION_SCHEMA.read_text(encoding="utf-8")),
            )

    def test_artifact_tampering_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result_path, lock, result = self._fixture(root)
            request_path = Path(
                result["artifacts"]["compaction_request"]["path"]  # type: ignore[index]
            )
            request_path.write_text("{}\n", encoding="utf-8")
            output = root / "verification.json"
            completed = self._run_verifier(result_path, lock, output)
            self.assertEqual(completed.returncode, 2)
            verification = json.loads(output.read_text(encoding="utf-8"))
            self.assertFalse(verification["passed"])
            self.assertFalse(verification["checks"]["artifact_hashes_valid"])


if __name__ == "__main__":
    unittest.main()
