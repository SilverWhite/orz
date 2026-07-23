from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from typing import Any


ROLES = ("root", "child", "grandchild")
CREATE_NO_WINDOW = getattr(subprocess, "CREATE_NO_WINDOW", 0)


def _atomic_write_json(path: Path, value: Any) -> None:
    temporary = path.with_name(f".{path.name}.{os.getpid()}.{time.time_ns()}.tmp")
    temporary.write_text(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False)
        + "\n",
        encoding="utf-8",
    )
    temporary.replace(path)


def _sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def _artifact_matches(record: dict[str, Any]) -> bool:
    try:
        path = Path(record["path"])
        content = path.read_bytes()
        return (
            len(content) == record["bytes"]
            and _sha256_bytes(content) == record["sha256"]
        )
    except (KeyError, OSError, TypeError):
        return False


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    values: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        value = json.loads(line)
        if isinstance(value, dict):
            values.append(value)
    return values


def _tool_sequence(requests: list[dict[str, Any]]) -> list[str]:
    sequence: list[str] = []
    for request in requests:
        body = request.get("body")
        messages = body.get("messages", []) if isinstance(body, dict) else []
        for message in messages:
            if not isinstance(message, dict) or message.get("role") != "assistant":
                continue
            calls = message.get("tool_calls", [])
            if not isinstance(calls, list):
                continue
            for call in calls:
                function = call.get("function") if isinstance(call, dict) else None
                name = function.get("name") if isinstance(function, dict) else None
                if isinstance(name, str) and name not in sequence:
                    sequence.append(name)
    return sequence


def _nonce_process_count(nonce: str) -> int:
    if os.name != "nt":
        return 0
    environment = dict(os.environ)
    environment["LIF_CHILD_TREE_NONCE"] = nonce
    script = (
        "$n=$env:LIF_CHILD_TREE_NONCE;"
        "@(Get-CimInstance Win32_Process | "
        "Where-Object { $_.CommandLine -and $_.CommandLine.Contains($n) }).Count"
    )
    completed = subprocess.run(
        ["powershell", "-NoProfile", "-Command", script],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=environment,
        timeout=20,
        check=False,
        creationflags=CREATE_NO_WINDOW,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"process replay scan failed: {completed.stderr.strip()}")
    return int(completed.stdout.strip())


def verify(result_path: Path) -> dict[str, Any]:
    result_path = result_path.resolve()
    result = _read_json(result_path)
    scenario = result.get("scenario")
    checks: dict[str, bool] = {}
    checks["result_declared_valid"] = result.get("valid") is True
    checks["scenario_known"] = scenario in {
        "tool_timeout",
        "task_cancel",
        "parent_exit",
    }

    binary = result.get("binary", {})
    binary_path = Path(str(binary.get("path", "")))
    release_path = Path(str(binary.get("release_metadata_path", "")))
    try:
        binary_content = binary_path.read_bytes()
        release_content = release_path.read_bytes()
        release = json.loads(release_content)
        release_binary = release["binary_release"]
        checks["binary_live_matches"] = (
            len(binary_content) == binary.get("bytes")
            and _sha256_bytes(binary_content) == binary.get("sha256")
        )
        checks["release_metadata_matches"] = (
            _sha256_bytes(release_content) == binary.get("release_metadata_sha256")
            and release_binary.get("version") == binary.get("release_version")
            and release_binary.get("bytes") == binary.get("bytes")
            and release_binary.get("sha256") == binary.get("sha256")
            and release_binary.get("authenticode_status")
            == binary.get("authenticode_status")
        )
    except (OSError, KeyError, TypeError, json.JSONDecodeError):
        checks["binary_live_matches"] = False
        checks["release_metadata_matches"] = False

    fixture = result.get("fixture", {})
    script_record = fixture.get("script", {})
    checks["fixture_script_matches"] = isinstance(
        script_record, dict
    ) and _artifact_matches(script_record)
    nonce = str(fixture.get("nonce", ""))
    checks["nonce_shape_valid"] = nonce.startswith("LIFCHILD-") and nonce[9:].isalnum()

    process_tree = result.get("process_tree", {})
    role_artifacts = process_tree.get("role_artifacts", [])
    roles: dict[str, dict[str, Any]] = {}
    role_artifacts_match = isinstance(role_artifacts, list) and len(role_artifacts) == 3
    if role_artifacts_match:
        for artifact in role_artifacts:
            if not isinstance(artifact, dict) or not _artifact_matches(artifact):
                role_artifacts_match = False
                continue
            value = _read_json(Path(artifact["path"]))
            role = value.get("role")
            if role not in ROLES or role in roles:
                role_artifacts_match = False
                continue
            roles[str(role)] = value
    checks["role_artifacts_match"] = role_artifacts_match
    checks["all_roles_match_nonce"] = (
        set(roles) == set(ROLES)
        and all(record.get("nonce") == nonce for record in roles.values())
    )
    checks["role_script_identity_matches"] = (
        checks["fixture_script_matches"]
        and set(roles) == set(ROLES)
        and all(
            record.get("script_sha256") == script_record.get("sha256")
            for record in roles.values()
        )
    )
    checks["pid_relationships_match"] = (
        set(roles) == set(ROLES)
        and roles["child"].get("parent_pid") == roles["root"].get("pid")
        and roles["grandchild"].get("parent_pid") == roles["child"].get("pid")
    )
    checks["no_role_completed_normally"] = (
        set(roles) == set(ROLES)
        and all(record.get("normal_completion") is False for record in roles.values())
    )
    pre_trigger = process_tree.get("pre_trigger_processes", [])
    pre_pids = {
        row.get("pid")
        for row in pre_trigger
        if isinstance(row, dict) and row.get("nonce_present") is True
    }
    checks["pre_trigger_snapshot_contains_tree"] = set(roles) == set(ROLES) and all(
        record.get("pid") in pre_pids for record in roles.values()
    )
    checks["recorded_post_trigger_empty"] = (
        process_tree.get("post_trigger_processes") == []
        and process_tree.get("residue_zero") is True
    )
    try:
        live_nonce_count = _nonce_process_count(nonce)
        checks["replay_nonce_residue_zero"] = live_nonce_count == 0
    except (RuntimeError, ValueError):
        live_nonce_count = -1
        checks["replay_nonce_residue_zero"] = False

    grok = result.get("grok", {})
    checks["outer_job_provenance_matches"] = (
        grok.get("outer_job_created") is True
        and grok.get("outer_job_assigned") is True
        and grok.get("outer_job_closed") is True
    )
    checks["parent_exit_policy_matches"] = (
        process_tree.get("parent_exit_triggered") is (scenario == "parent_exit")
    )

    provider = result.get("provider", {})
    private_capture = provider.get("private_capture", {})
    provider_result = provider.get("result", {})
    checks["provider_artifacts_match"] = (
        isinstance(private_capture, dict)
        and isinstance(provider_result, dict)
        and _artifact_matches(private_capture)
        and _artifact_matches(provider_result)
    )
    primary: list[dict[str, Any]] = []
    if checks["provider_artifacts_match"]:
        primary = [
            row
            for row in _read_jsonl(Path(private_capture["path"]))
            if isinstance(row.get("body"), dict)
            and row["body"].get("model") == "deepseek-v4-pro"
        ]
    replay_tool_sequence = _tool_sequence(primary)
    checks["provider_request_count_replays"] = (
        len(primary) == provider.get("primary_request_count")
        and len(primary) == provider.get("expected_primary_request_count")
    )
    checks["provider_tool_sequence_replays"] = (
        replay_tool_sequence == provider.get("tool_sequence")
        and replay_tool_sequence == provider.get("expected_tool_sequence")
    )
    provider_value = (
        _read_json(Path(provider_result["path"]))
        if checks["provider_artifacts_match"]
        else {}
    )
    checks["provider_terminal_replays"] = (
        provider_value.get("terminal_state") == "succeeded"
    )
    checks["provider_parent_disconnect_replays"] = (
        provider_value.get("continuity", {}).get("parent_exit_disconnect_observed")
        is True
        if scenario == "parent_exit"
        else True
    )
    checks["real_model_not_invoked"] = (
        provider.get("real_model_invoked") is False
        and provider_value.get("response", {}).get("real_model_invoked") is False
    )

    capture = result.get("capture", {})
    capture_names = (
        "grok_stdout",
        "grok_stderr",
        "provider_stdout",
        "provider_stderr",
    )
    checks["stream_artifacts_match"] = all(
        isinstance(capture.get(name), dict)
        and _artifact_matches(capture[name])
        for name in capture_names
    )
    combined = b""
    if checks["provider_artifacts_match"] and checks["stream_artifacts_match"]:
        combined = b"".join(
            Path(record["path"]).read_bytes()
            for record in [
                private_capture,
                capture["grok_stdout"],
                capture["grok_stderr"],
            ]
        )
    replay_markers = {
        role: {
            "stdout": f"LIF_CHILD_TREE_STDOUT:{nonce}:{role}:ready".encode()
            in combined,
            "stderr": f"LIF_CHILD_TREE_STDERR:{nonce}:{role}:ready".encode()
            in combined,
        }
        for role in ROLES
    }
    checks["marker_projection_replays"] = (
        replay_markers == capture.get("marker_projection")
    )
    output_drain_observed = all(
        value["stdout"] and value["stderr"] for value in replay_markers.values()
    )
    output_drain_required = scenario in {"tool_timeout", "task_cancel"}
    checks["output_drain_policy_replays"] = (
        capture.get("output_drain_required") is output_drain_required
        and capture.get("output_drain_observed") is output_drain_observed
        and (output_drain_observed if output_drain_required else True)
    )

    workspace = result.get("workspace", {})
    receipt_names = ("preflight", "launch", "postrun")
    receipt_values: list[dict[str, Any]] = []
    workspace_artifacts_match = True
    for name in receipt_names:
        record = workspace.get(name)
        if not isinstance(record, dict) or not _artifact_matches(record):
            workspace_artifacts_match = False
            continue
        receipt_values.append(_read_json(Path(record["path"])))
    checks["workspace_artifacts_match"] = workspace_artifacts_match
    checks["workspace_control_replays"] = (
        len(receipt_values) == 3
        and all(
            receipt.get("valid") is True
            and receipt.get("discovery", {}).get("aggregate_sha256")
            == workspace.get("control_aggregate_sha256")
            and receipt.get("discovery", {}).get("scan_policy_sha256")
            == workspace.get("scan_policy_sha256")
            for receipt in receipt_values
        )
    )

    firewall = result.get("firewall", {})
    firewall_record = firewall.get("receipt", {})
    checks["firewall_artifact_matches"] = isinstance(
        firewall_record, dict
    ) and _artifact_matches(firewall_record)
    firewall_value = (
        _read_json(Path(firewall_record["path"]))
        if checks["firewall_artifact_matches"]
        else {}
    )
    checks["firewall_cleanup_replays"] = (
        firewall.get("removed") is True
        and firewall.get("remaining_rule_count") == 0
        and firewall_value.get("removed") is True
        and firewall_value.get("remaining_rule_count") == 0
    )

    result_checks = result.get("checks", {})
    checks["embedded_checks_all_true"] = (
        isinstance(result_checks, dict)
        and bool(result_checks)
        and all(value is True for value in result_checks.values())
    )
    verification = {
        "schema_version": "0.1.0",
        "verifier": "grok-windows-child-tree-independent",
        "result": {
            "path": str(result_path),
            "bytes": result_path.stat().st_size,
            "sha256": _sha256_bytes(result_path.read_bytes()),
        },
        "scenario": scenario,
        "valid": all(checks.values()),
        "checks": checks,
        "replay": {
            "role_count": len(roles),
            "primary_request_count": len(primary),
            "tool_sequence": replay_tool_sequence,
            "live_nonce_process_count": live_nonce_count,
            "marker_projection": replay_markers,
        },
        "limitations": [
            "The verifier reconstructs containment evidence and current nonce residue; it does not prove future OS state.",
            "Authenticode was enforced by the launcher and release receipt; this replay matches the recorded binary bytes and release metadata.",
            "Mechanical verification is not scientific validation.",
        ],
    }
    return verification


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Independently replay a Grok Windows child-tree probe"
    )
    parser.add_argument("--result", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        print(f"refusing to overwrite verifier output: {args.output}", file=sys.stderr)
        return 2
    try:
        verification = verify(args.result)
        _atomic_write_json(args.output.resolve(), verification)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(verification, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if verification["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
