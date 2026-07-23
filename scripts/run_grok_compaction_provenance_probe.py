from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import uuid
from typing import Any

from fake_deepseek_provider import (
    COMPACTION_POST_RESPONSE_MARKER,
    COMPACTION_SOURCE_OMITTED_MARKER,
    COMPACTION_SOURCE_RETAINED_MARKER,
    COMPACTION_SUMMARY_MARKER,
)
from run_grok_acp_fake_tool_client import AcpClient, _find_session_files
from run_grok_windows_child_tree_probe import (
    CREATE_NO_WINDOW,
    KillOnCloseJob,
    _clean_environment,
    _firewall,
    _inspect_binary,
    _require_windows_admin,
)


MODEL_ALIAS = "lif-fake-deepseek"
MODEL_ID = "deepseek-v4-pro"


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _canonical_bytes(value: Any) -> bytes:
    return json.dumps(
        value,
        ensure_ascii=False,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _write_text(path: Path, value: str) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(value, encoding="utf-8", newline="\n")
    temporary.replace(path)


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected a JSON object: {path}")
    return value


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"expected JSON objects in {path}")
        values.append(value)
    return values


def _artifact(path: Path, *, classification: str) -> dict[str, Any]:
    content = path.read_bytes()
    return {
        "path": str(path.resolve()),
        "bytes": len(content),
        "sha256": _sha256_bytes(content),
        "classification": classification,
    }


def _wait_for_file(path: Path, deadline: float) -> None:
    while not path.is_file():
        if time.monotonic() >= deadline:
            raise TimeoutError(f"timed out waiting for {path}")
        time.sleep(0.05)


def _wait_for_request_count(path: Path, count: int, deadline: float) -> None:
    while True:
        if path.is_file() and len(_read_jsonl(path)) >= count:
            return
        if time.monotonic() >= deadline:
            raise TimeoutError(f"timed out waiting for {count} provider requests")
        time.sleep(0.05)


def _wait_for_unique_artifact(root: Path, pattern: str, deadline: float) -> Path:
    while True:
        candidates = sorted(path for path in root.rglob(pattern) if path.is_file())
        if len(candidates) == 1:
            return candidates[0]
        if len(candidates) > 1:
            raise RuntimeError(f"expected one {pattern} artifact, found {len(candidates)}")
        if time.monotonic() >= deadline:
            raise TimeoutError(f"timed out waiting for {pattern} under {root}")
        time.sleep(0.05)


def _snapshot_first_request(private_capture: Path, target: Path) -> dict[str, Any]:
    requests = [
        row
        for row in _read_jsonl(private_capture)
        if isinstance(row.get("body"), dict)
        and row["body"].get("model") == MODEL_ID
    ]
    if len(requests) != 1:
        raise RuntimeError(
            "pre-compaction snapshot requires exactly one primary provider request"
        )
    _atomic_write_json(target, requests[0])
    return _artifact(target, classification="fake-fixture-private")


def _extract_summary(request: dict[str, Any]) -> str:
    summary = request.get("summary")
    if not isinstance(summary, str) or not summary:
        raise RuntimeError("compaction request artifact has no completed summary")
    return summary


def _extract_chat_history(request: dict[str, Any]) -> list[Any]:
    history = request.get("chat_history")
    if not isinstance(history, list) or not history:
        raise RuntimeError("compaction request artifact has no chat_history source span")
    return history


def _hook_projection(rows: list[dict[str, Any]], session_id: str) -> dict[str, Any]:
    names = [row.get("hook_event_name") for row in rows]
    sources = [row.get("source") for row in rows]
    session_ids = [row.get("session_id") for row in rows]
    return {
        "pre_compact_observed": names.count("pre_compact") == 1,
        "post_compact_observed": names.count("post_compact") == 1,
        "ordered_events": names,
        "sources": sources,
        "session_ids_match": bool(rows)
        and all(value == session_id for value in session_ids),
        "manual_trigger_only": bool(rows)
        and all(value == "manual" for value in sources),
    }


def _write_config(
    profile: Path, provider_port: int, python_path: Path, hook_script: Path, hook_log: Path
) -> tuple[Path, Path]:
    config_root = profile / ".grok"
    hooks_root = config_root / "hooks"
    hooks_root.mkdir(parents=True)
    config_path = config_root / "config.toml"
    config = f"""
[cli]
auto_update = false
use_leader = false

[features]
telemetry = false
feedback = false
lsp_tools = false
codebase_indexing = false
remote_fetch = false

[session]
load_envrc = false
auto_compact_threshold_percent = 99

[models]
default = "{MODEL_ALIAS}"
default_reasoning_effort = "high"

[model.{MODEL_ALIAS}]
model = "{MODEL_ID}"
base_url = "http://127.0.0.1:{provider_port}"
name = "LIF Fake DeepSeek Compaction Fixture"
env_key = "LIF_FAKE_DEEPSEEK_KEY"
api_backend = "chat_completions"
max_completion_tokens = 256
context_window = 65536

[permission]
rules = [
  {{ action = "deny", tool = "read" }},
  {{ action = "deny", tool = "edit" }},
  {{ action = "deny", tool = "bash" }},
  {{ action = "deny", tool = "grep" }},
  {{ action = "deny", tool = "mcp" }},
  {{ action = "deny", tool = "webfetch" }},
  {{ action = "deny", tool = "websearch" }},
]
""".strip()
    _write_text(config_path, config + "\n")
    command = f'& "{python_path}" "{hook_script}" "{hook_log}"'
    handler = {
        "type": "command",
        "command": command,
        "timeout": 5,
        "env": {"LIF_COMPACTION_HOOK_LOG": str(hook_log)},
    }
    hook_config = {
        "hooks": {
            "PreCompact": [
                {
                    "matcher": "manual",
                    "hooks": [handler],
                }
            ],
            "PostCompact": [
                {
                    "matcher": "manual",
                    "hooks": [handler],
                }
            ],
        }
    }
    hook_config_path = hooks_root / "compaction-provenance.json"
    _atomic_write_json(hook_config_path, hook_config)
    if hook_log.exists():
        raise ValueError(f"hook log already exists: {hook_log}")
    return config_path, hook_config_path


def _initialize(client: AcpClient, workspace: Path) -> str:
    initialized = client.request(
        "compact-init-1",
        "initialize",
        {
            "protocolVersion": 1,
            "clientCapabilities": {},
            "clientInfo": {
                "name": "lif-compaction-provenance-probe",
                "title": "LIF Compaction Provenance Probe",
                "version": "0.1.0",
            },
        },
    )
    if initialized.get("protocolVersion") != 1:
        raise RuntimeError("Grok did not agree to ACP protocol version 1")
    session = client.request(
        "compact-session-1",
        "session/new",
        {"cwd": str(workspace), "mcpServers": []},
    )
    session_id = session.get("sessionId")
    if not isinstance(session_id, str) or not session_id:
        raise RuntimeError("session/new did not return a sessionId")
    client.session_id = session_id
    return session_id


def run(args: argparse.Namespace) -> dict[str, Any]:
    _require_windows_admin()
    repo_root = Path(__file__).resolve().parents[1]
    output = args.output_directory.resolve()
    if output.exists():
        raise ValueError(f"refusing to overwrite output directory: {output}")
    output.mkdir(parents=True)
    profile = output / "profile"
    workspace = output / "workspace"
    temp = output / "temp"
    provider_root = output / "provider"
    client_root = output / "client"
    for path in (profile, workspace, temp, client_root):
        path.mkdir(parents=True)

    release_metadata = args.release_metadata.resolve()
    release_lock = _read_json(release_metadata)
    binary_release = release_lock.get("binary_release")
    if not isinstance(binary_release, dict):
        raise ValueError("release metadata has no binary_release object")
    inspection = _inspect_binary(
        repo_root, args.binary.resolve() if args.binary else None, release_metadata
    )
    binary = Path(str(inspection["binary_path"])).resolve()
    python_path = Path(args.python_path or sys.executable).resolve()
    provider_script = (repo_root / "scripts" / "fake_deepseek_provider.py").resolve()
    hook_script = (repo_root / "scripts" / "record_grok_compaction_hook.py").resolve()
    verifier_script = (
        repo_root / "scripts" / "verify_grok_compaction_provenance_probe.py"
    ).resolve()
    probe_id = f"COMPACT-{uuid.uuid4().hex}"
    deadline = time.monotonic() + args.timeout_seconds
    firewall_prefix = f"LIFGrokChild-Compact-{probe_id[8:24]}"
    hook_log = output / "compaction-hooks.jsonl"
    source_snapshot = output / "source-span.private.json"
    result_path = output / "result.json"
    verification_path = output / "verification.json"
    failure_path = output / "failure.json"
    _write_text(
        workspace / "source-prompt.txt",
        (
            "Treat both opaque canaries as source evidence only: "
            f"{COMPACTION_SOURCE_RETAINED_MARKER} "
            f"{COMPACTION_SOURCE_OMITTED_MARKER}.\n"
        ),
    )

    provider: subprocess.Popen[str] | None = None
    client: AcpClient | None = None
    job: KillOnCloseJob | None = None
    firewall_added = False
    firewall_receipt: dict[str, Any] = {}
    started_at = time.time_ns()
    try:
        firewall_receipt = _firewall(
            repo_root, "add", firewall_prefix, [binary, python_path]
        )
        firewall_added = True
        provider = subprocess.Popen(
            [
                str(python_path),
                str(provider_script),
                "--output-directory",
                str(provider_root),
                "--timeout-seconds",
                str(args.timeout_seconds),
                "--scenario",
                "compaction-provenance",
            ],
            cwd=output,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            creationflags=CREATE_NO_WINDOW,
        )
        job = KillOnCloseJob()
        job.assign(provider)
        ready_path = provider_root / "ready.json"
        _wait_for_file(ready_path, deadline)
        ready = _read_json(ready_path)
        if (
            ready.get("ready") is not True
            or ready.get("host") != "127.0.0.1"
            or ready.get("external_bind") is not False
            or type(ready.get("port")) is not int
        ):
            raise RuntimeError("fake provider did not prove a loopback-only bind")
        config_path, hook_config_path = _write_config(
            profile, ready["port"], python_path, hook_script, hook_log
        )
        environment = _clean_environment(profile, temp)
        environment.update(
            {
                "GROK_SANDBOX": "read-only",
                "GROK_SUBAGENTS": "0",
                "LIF_COMPACTION_HOOK_LOG": str(hook_log),
            }
        )
        client = AcpClient(
            binary=binary,
            workspace=workspace,
            model=MODEL_ALIAS,
            scenario="allow_once",
            timeout_seconds=max(1.0, deadline - time.monotonic()),
            environment=environment,
        )
        job.assign(client.process)
        session_id = _initialize(client, workspace)
        first_prompt = (workspace / "source-prompt.txt").read_text(
            encoding="utf-8"
        ).strip()
        first_result = client.prompt(
            session_id, first_prompt, request_id="compact-prompt-source"
        )
        provider_private = provider_root / "requests.private.jsonl"
        _wait_for_request_count(provider_private, 1, deadline)
        source_artifact = _snapshot_first_request(provider_private, source_snapshot)
        events_path, updates_path = _find_session_files(profile, session_id)
        session_root = events_path.parent

        compact_result = client.prompt(
            session_id,
            f"/compact preserve {COMPACTION_SOURCE_RETAINED_MARKER}",
            request_id="compact-prompt-command",
        )
        request_path = _wait_for_unique_artifact(
            session_root / "compaction_requests", "*.json", deadline
        )
        checkpoint_path = _wait_for_unique_artifact(
            session_root / "compaction_checkpoints", "*.json", deadline
        )
        _wait_for_file(hook_log, deadline)
        while len(_read_jsonl(hook_log)) < 2:
            if time.monotonic() >= deadline:
                raise TimeoutError("timed out waiting for both compaction hooks")
            time.sleep(0.05)

        post_result = client.prompt(
            session_id,
            "Continue after compaction with the fixed post marker.",
            request_id="compact-prompt-post",
        )
        client.close_input()
        _wait_for_request_count(provider_private, 3, deadline)
        try:
            client.process.wait(timeout=max(1.0, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as error:
            raise TimeoutError("Grok ACP process did not exit after input close") from error
        provider_stdout, provider_stderr = provider.communicate(
            timeout=max(1.0, deadline - time.monotonic())
        )
        if provider.returncode != 0:
            raise RuntimeError(
                f"fake provider failed ({provider.returncode}): {provider_stderr.strip()}"
            )
        _write_text(client_root / "transcript.private.jsonl", "\n".join(
            json.dumps(row, ensure_ascii=False, sort_keys=True, allow_nan=False)
            for row in client.transcript
        ) + "\n")
        _write_text(client_root / "grok.stderr.log", "".join(client.stderr_chunks))
        _write_text(provider_root / "process.stdout.log", provider_stdout)
        _write_text(provider_root / "process.stderr.log", provider_stderr)

        request = _read_json(request_path)
        checkpoint = _read_json(checkpoint_path)
        summary = _extract_summary(request)
        chat_history = _extract_chat_history(request)
        source_text = json.dumps(chat_history, ensure_ascii=False, sort_keys=True)
        checkpoint_text = json.dumps(checkpoint, ensure_ascii=False, sort_keys=True)
        hooks = _read_jsonl(hook_log)
        lifecycle = _hook_projection(hooks, session_id)
        source_after = _artifact(
            source_snapshot, classification="fake-fixture-private"
        )
        source_immutable = source_after["sha256"] == source_artifact["sha256"]
        events = _read_jsonl(events_path)
        updates = _read_jsonl(updates_path)
        provider_result = _read_json(provider_root / "provider-result.json")

        result = {
            "schema_version": "0.1.0",
            "probe_id": probe_id,
            "terminal_state": "succeeded",
            "started_at_unix_ns": started_at,
            "finished_at_unix_ns": time.time_ns(),
            "release": {
                "metadata_path": str(release_metadata),
                "binary_path": str(binary),
                "binary_sha256": inspection.get("observed", {}).get("sha256"),
                "version": binary_release.get("version"),
                "build_id": binary_release.get("build_id"),
                "inspection_valid": inspection.get("valid") is True,
            },
            "session": {
                "session_id": session_id,
                "workspace": str(workspace),
                "profile": str(profile),
                "trigger": "manual",
                "model_alias": MODEL_ALIAS,
                "provider_model": MODEL_ID,
                "request_count": provider_result.get("primary_request_count"),
                "first_prompt_stop_reason": first_result.get("stopReason"),
                "compact_stop_reason": compact_result.get("stopReason"),
                "post_prompt_stop_reason": post_result.get("stopReason"),
                "event_count": len(events),
                "update_count": len(updates),
            },
            "lifecycle": lifecycle,
            "provenance": {
                "source_span": {
                    "kind": "compaction_request.chat_history",
                    "item_count": len(chat_history),
                    "first_index": 0,
                    "last_index": len(chat_history) - 1,
                    "canonical_sha256": _sha256_bytes(
                        _canonical_bytes(chat_history)
                    ),
                    "recorded_before_summary_replacement": True,
                },
                "boundary": {
                    "request_id": request.get("request_id"),
                    "trigger": request.get("trigger"),
                    "checkpoint_id": checkpoint.get("checkpoint_id"),
                    "prompt_index_at_compaction": checkpoint.get(
                        "prompt_index_at_compaction"
                    ),
                },
                "summary": {
                    "status": "derived_unverified",
                    "chars": len(summary),
                    "sha256": _sha256_bytes(summary.encode("utf-8")),
                    "may_replace_source_evidence": False,
                },
                "ranges": {
                    "summarized": [
                        {
                            "source": "compaction_request.chat_history",
                            "first_index": 0,
                            "last_index": len(chat_history) - 1,
                            "basis": "direct_request_artifact",
                        }
                    ],
                    "retained": [],
                    "discarded": [],
                    "unknown": [
                        {
                            "scope": "source-index retention/discard mapping",
                            "reason": "Grok 0.2.111 artifacts do not directly label each source item as retained or discarded.",
                        }
                    ],
                },
            },
            "canary_checks": {
                "retained_marker_in_source": COMPACTION_SOURCE_RETAINED_MARKER
                in source_text,
                "omitted_marker_in_source": COMPACTION_SOURCE_OMITTED_MARKER
                in source_text,
                "summary_marker_in_summary": COMPACTION_SUMMARY_MARKER in summary,
                "retained_marker_in_summary": COMPACTION_SOURCE_RETAINED_MARKER
                in summary,
                "omitted_marker_absent_from_summary": COMPACTION_SOURCE_OMITTED_MARKER
                not in summary,
                "summary_marker_in_checkpoint": COMPACTION_SUMMARY_MARKER
                in checkpoint_text,
                "post_response_marker_observed": any(
                    COMPACTION_POST_RESPONSE_MARKER
                    in json.dumps(row, ensure_ascii=False)
                    for row in client.transcript
                ),
                "source_snapshot_immutable": source_immutable,
            },
            "containment": {
                "loopback_only_provider": ready.get("external_bind") is False,
                "firewall_added": firewall_receipt.get("action") == "add"
                and type(firewall_receipt.get("rule_count")) is int
                and firewall_receipt["rule_count"] > 0,
                "job_created": job is not None,
                "job_assigned": job is not None and job.assigned,
                "real_model_invoked": False,
            },
            "artifacts": {
                "config": _artifact(config_path, classification="fake-fixture"),
                "hook_config": _artifact(
                    hook_config_path, classification="fake-fixture"
                ),
                "hook_receipts": _artifact(
                    hook_log, classification="fake-fixture-private"
                ),
                "source_snapshot": source_after,
                "compaction_request": _artifact(
                    request_path, classification="fake-fixture-private"
                ),
                "compaction_checkpoint": _artifact(
                    checkpoint_path, classification="fake-fixture-private"
                ),
                "session_events": _artifact(
                    events_path, classification="fake-fixture-private"
                ),
                "session_updates": _artifact(
                    updates_path, classification="fake-fixture-private"
                ),
                "provider_capture": _artifact(
                    provider_private, classification="fake-fixture-private"
                ),
                "provider_result": _artifact(
                    provider_root / "provider-result.json",
                    classification="fake-fixture",
                ),
            },
            "limitations": [
                "This is a fixed fake-provider conformance observation, not a real DeepSeek call.",
                "The manual /compact path is observed; automatic threshold triggering remains a separate test.",
                "Summary content is model-derived and unverified, even when fixed canaries match.",
                "Per-item retained/discarded classification remains unknown when not directly recorded.",
            ],
        }
        _atomic_write_json(result_path, result)
        completed = subprocess.run(
            [
                str(python_path),
                str(verifier_script),
                "--result",
                str(result_path),
                "--output",
                str(verification_path),
                "--lock",
                str(release_metadata),
            ],
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=30,
            creationflags=CREATE_NO_WINDOW,
            check=False,
        )
        if completed.returncode != 0:
            raise RuntimeError(
                f"independent verifier failed: {completed.stderr.strip() or completed.stdout.strip()}"
            )
        return result
    except Exception as error:
        _atomic_write_json(
            failure_path,
            {
                "schema_version": "0.1.0",
                "probe_id": probe_id,
                "terminal_state": "failed",
                "error_type": type(error).__name__,
                "error": str(error),
            },
        )
        raise
    finally:
        if client is not None:
            client.close_input()
            if client.process.poll() is None:
                client.process.terminate()
        if provider is not None and provider.poll() is None:
            provider.terminate()
        if job is not None and not job.closed:
            job.close()
        if firewall_added:
            removal = _firewall(repo_root, "remove", firewall_prefix)
            firewall_receipt.update(removal)
            _atomic_write_json(output / "firewall-receipt.json", firewall_receipt)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Observe Grok manual compaction provenance with a fake provider"
    )
    parser.add_argument("--output-directory", type=Path, required=True)
    parser.add_argument("--release-metadata", type=Path, required=True)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--python-path", type=Path)
    parser.add_argument("--timeout-seconds", type=float, default=90.0)
    args = parser.parse_args()
    if args.timeout_seconds <= 0 or args.timeout_seconds > 300:
        parser.error("--timeout-seconds must be in (0, 300]")
    try:
        result = run(args)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
