#!/usr/bin/env python3
"""Independently verify a Grok ACP initialize no-model probe result."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Any

from jsonschema import Draft202012Validator, FormatChecker


ROOT = Path(__file__).resolve().parents[1]
RESULT_SCHEMA = ROOT / "integration" / "grok" / "grok-acp-initialize-probe-result-v0.1.schema.json"
VERIFICATION_SCHEMA = ROOT / "integration" / "grok" / "grok-acp-initialize-probe-verification-v0.1.schema.json"


class ProbeVerificationError(RuntimeError):
    pass


def _load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ProbeVerificationError(f"cannot read JSON {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise ProbeVerificationError(f"expected JSON object: {path}")
    return value


def _sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _artifact_matches(artifact: Any) -> bool:
    if not isinstance(artifact, dict):
        return False
    path = Path(str(artifact.get("path", "")))
    return (
        path.is_file()
        and path.stat().st_size == artifact.get("bytes")
        and _sha256_file(path) == artifact.get("sha256")
    )


def _schema_errors(schema_path: Path, value: dict[str, Any], label: str) -> list[str]:
    schema = _load_json(schema_path)
    failures = sorted(
        Draft202012Validator(schema, format_checker=FormatChecker()).iter_errors(value),
        key=lambda error: list(error.absolute_path),
    )
    return [
        f"{label}#/{'/'.join(map(str, failure.absolute_path))}: {failure.message}"
        for failure in failures
    ]


def _leaf_paths(value: Any, prefix: str) -> list[str]:
    if isinstance(value, dict):
        paths: list[str] = []
        for key in sorted(value):
            child = f"{prefix}.{key}" if prefix else key
            paths.extend(_leaf_paths(value[key], child))
        return paths
    if isinstance(value, list):
        if not value:
            return [f"{prefix}[]"]
        paths = []
        for item in value:
            paths.extend(_leaf_paths(item, f"{prefix}[]"))
        return sorted(set(paths))
    return [prefix]


def _request_checks(result: dict[str, Any], errors: list[str]) -> tuple[bool, bool, dict[str, Any] | None]:
    artifact = result.get("request", {}).get("artifact")
    artifact_ok = _artifact_matches(artifact)
    if not artifact_ok:
        errors.append("request artifact mismatch")
        return False, False, None
    try:
        request = _load_json(Path(artifact["path"]))
    except ProbeVerificationError as exc:
        errors.append(str(exc))
        return True, False, None
    params = request.get("params")
    exact = (
        request.get("jsonrpc") == "2.0"
        and request.get("id") == result["request"].get("id")
        and request.get("method") == "initialize"
        and isinstance(params, dict)
        and params.get("protocolVersion") == 1
        and params.get("clientCapabilities") == {}
        and set(request) == {"jsonrpc", "id", "method", "params"}
        and set(params) == {"protocolVersion", "clientCapabilities", "clientInfo"}
        and result["request"].get("session_or_prompt_requests_sent") == 0
        and "session/prompt" not in json.dumps(request, ensure_ascii=False)
        and "session/new" not in json.dumps(request, ensure_ascii=False)
    )
    if not exact:
        errors.append("request is not the exact initialize-only no-model shape")
    return True, exact, request


def _response_checks(
    result: dict[str, Any], errors: list[str]
) -> tuple[bool, bool, bool]:
    response_record = result.get("response", {})
    artifact = response_record.get("artifact")
    artifact_ok = _artifact_matches(artifact)
    if not artifact_ok:
        errors.append("response artifact mismatch")
        return False, False, False
    try:
        response = _load_json(Path(artifact["path"]))
    except ProbeVerificationError as exc:
        errors.append(str(exc))
        return True, False, False
    rpc_result = response.get("result")
    semantics = (
        response.get("jsonrpc") == response_record.get("jsonrpc") == "2.0"
        and response.get("id") == result["request"].get("id")
        and response_record.get("id_matches") is True
        and isinstance(rpc_result, dict)
        and response_record.get("has_result") is True
        and response_record.get("has_error") is False
        and response_record.get("state") == "observed"
        and rpc_result.get("protocolVersion") == response_record.get("protocol_version") == 1
    )
    if not semantics:
        errors.append("response JSON-RPC semantics do not match the recorded projection")
    stdout_artifact = result.get("process", {}).get("stdout")
    stderr_artifact = result.get("process", {}).get("stderr")
    transcript_ok = _artifact_matches(stdout_artifact) and _artifact_matches(stderr_artifact)
    allowed_method = "_x.ai/mcp/servers_updated"
    parsed_lines: list[dict[str, Any]] = []
    if transcript_ok:
        try:
            stdout_text = Path(stdout_artifact["path"]).read_text(encoding="utf-8")
            stderr_text = Path(stderr_artifact["path"]).read_text(encoding="utf-8")
            for line in stdout_text.splitlines():
                value = json.loads(line)
                if not isinstance(value, dict):
                    raise ValueError("stdout JSONL line is not an object")
                parsed_lines.append(value)
            transcript_ok = stderr_text == "" and bool(parsed_lines) and parsed_lines[0] == response
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError, KeyError):
            transcript_ok = False
    notifications = parsed_lines[1:] if transcript_ok else []
    notification_ok = transcript_ok and all(
        set(value) == {"jsonrpc", "method", "params"}
        and value.get("jsonrpc") == "2.0"
        and value.get("method") == allowed_method
        and value.get("params") == {"mcpServers": []}
        for value in notifications
    )
    notification_ok = notification_ok and (
        response_record.get("allowed_notification_count") == len(notifications)
        and response_record.get("allowed_notification_methods")
        == ([allowed_method] if notifications else [])
        and response_record.get("unexpected_stdout_line_count") == 0
    )
    semantics = semantics and notification_ok
    if not notification_ok:
        errors.append("stdout/stderr transcript or initialize notification classification mismatch")
    capabilities = rpc_result.get("agentCapabilities", {}) if isinstance(rpc_result, dict) else {}
    meta = rpc_result.get("_meta", {}) if isinstance(rpc_result, dict) else {}
    agent_info = rpc_result.get("agentInfo", {}) if isinstance(rpc_result, dict) else {}
    auth_methods = rpc_result.get("authMethods") if isinstance(rpc_result, dict) else None
    projection = (
        sorted(response_record.get("agent_capability_paths", []))
        == _leaf_paths(capabilities, "agentCapabilities")
        and sorted(response_record.get("extension_meta_paths", [])) == _leaf_paths(meta, "_meta")
        and response_record.get("agent_info")
        == {
            "name": agent_info.get("name") if isinstance(agent_info, dict) else None,
            "title": agent_info.get("title") if isinstance(agent_info, dict) else None,
            "version": agent_info.get("version") if isinstance(agent_info, dict) else None,
        }
        and response_record.get("auth_method_count")
        == (len(auth_methods) if isinstance(auth_methods, list) else None)
    )
    if not projection:
        errors.append("response capability/meta projection mismatch")
    return True, semantics, projection


def _control_receipts_match(result: dict[str, Any], errors: list[str]) -> bool:
    trust = result.get("workspace_trust", {})
    pre_artifact = trust.get("preflight_receipt")
    post_artifact = trust.get("postrun_receipt")
    if not _artifact_matches(pre_artifact) or not _artifact_matches(post_artifact):
        errors.append("workspace trust receipt artifact mismatch")
        return False
    try:
        pre = _load_json(Path(pre_artifact["path"]))
        post = _load_json(Path(post_artifact["path"]))
    except ProbeVerificationError as exc:
        errors.append(str(exc))
        return False
    matched = (
        pre.get("valid") is True
        and post.get("valid") is True
        and pre.get("decision", {}).get("mode") == "restricted"
        and post.get("decision", {}).get("mode") == "restricted"
        and pre.get("decision", {}).get("launch_permitted") is True
        and post.get("decision", {}).get("launch_permitted") is True
        and pre.get("discovery", {}).get("candidate_count") == trust.get("candidate_count") == 0
        and post.get("discovery", {}).get("candidate_count") == 0
        and pre.get("discovery", {}).get("aggregate_sha256")
        == post.get("discovery", {}).get("aggregate_sha256")
        and trust.get("aggregate_unchanged") is True
    )
    if not matched:
        errors.append("workspace trust receipts do not prove an empty stable restricted workspace")
    return matched


def verify(result_path: Path) -> dict[str, Any]:
    result = _load_json(result_path)
    errors = _schema_errors(RESULT_SCHEMA, result, "result")
    checks = {
        "result_schema_valid": not errors,
        "result_valid_flag": result.get("valid") is True,
        "binary_artifact_matches": False,
        "request_artifact_matches": False,
        "request_initialize_only": False,
        "response_artifact_matches": False,
        "response_semantics_match": False,
        "capability_projection_matches": False,
        "control_receipts_match": False,
        "safety_checks_all_true": False,
    }
    if not checks["result_valid_flag"]:
        errors.append("result valid flag is not true")
    binary = result.get("binary", {})
    binary_artifact = {
        "path": binary.get("path"),
        "bytes": binary.get("bytes"),
        "sha256": binary.get("sha256"),
    }
    checks["binary_artifact_matches"] = (
        _artifact_matches(binary_artifact)
        and binary.get("sha256") == binary.get("locked_sha256")
    )
    if not checks["binary_artifact_matches"]:
        errors.append("binary artifact or lock digest mismatch")

    request_artifact, request_exact, _ = _request_checks(result, errors)
    checks["request_artifact_matches"] = request_artifact
    checks["request_initialize_only"] = request_exact
    response_artifact, response_semantics, projection = _response_checks(result, errors)
    checks["response_artifact_matches"] = response_artifact
    checks["response_semantics_match"] = response_semantics
    checks["capability_projection_matches"] = projection
    checks["control_receipts_match"] = _control_receipts_match(result, errors)
    safety = result.get("checks", {})
    checks["safety_checks_all_true"] = (
        bool(safety)
        and all(value is True for value in safety.values())
        and result.get("network", {}).get("remaining_rule_count") == 0
        and result.get("environment", {}).get("credential_variable_names") == []
        and result.get("leak_scan", {}).get("hit_count") == 0
    )
    if not checks["safety_checks_all_true"]:
        errors.append("one or more recorded safety checks are not true")

    valid = all(checks.values()) and not errors
    report = {
        "schema_version": "0.1.0",
        "verification_kind": "grok-acp-initialize-probe-verification",
        "valid": valid,
        "probe_id": result.get("probe_id") if isinstance(result.get("probe_id"), str) else None,
        "result_sha256": _sha256_file(result_path),
        "checks": checks,
        "errors": errors,
    }
    verification_errors = _schema_errors(VERIFICATION_SCHEMA, report, "verification")
    if verification_errors:
        raise ProbeVerificationError("verification report schema failure: " + "; ".join(verification_errors))
    return report


def _write_new(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise ProbeVerificationError(f"refusing to overwrite output: {path}") from exc
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
    except BaseException:
        path.unlink(missing_ok=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--result", required=True)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    try:
        report = verify(Path(args.result))
        _write_new(Path(args.output), report)
    except ProbeVerificationError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 3
    print(json.dumps({"valid": report["valid"], "checks": report["checks"]}))
    return 0 if report["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
