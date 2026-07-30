from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import time
from typing import Any


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


def _artifact(path: Path) -> dict[str, Any]:
    content = path.read_bytes()
    return {
        "path": str(path.resolve()),
        "bytes": len(content),
        "sha256": _sha256_bytes(content),
    }


def _read_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"expected JSON object: {path}")
    return value


def _result_by_version(summary: dict[str, Any], version: str) -> dict[str, Any]:
    rows = summary.get("results")
    if not isinstance(rows, list):
        raise ValueError("summary.results must be an array")
    matches = [
        row for row in rows if isinstance(row, dict) and row.get("version") == version
    ]
    if len(matches) != 1:
        raise ValueError(f"expected exactly one summary result for version {version}")
    return matches[0]


def _diagnostic_path(row: dict[str, Any]) -> Path:
    output_directory = Path(str(row.get("output_directory", "")))
    return output_directory / "timeout-diagnostic.json"


def _load_diagnostic(row: dict[str, Any]) -> dict[str, Any] | None:
    path = _diagnostic_path(row)
    if not path.is_file():
        return None
    return _read_json(path)


def _tool_sequence(diagnostic: dict[str, Any] | None) -> list[str]:
    value = (
        diagnostic.get("provider", {}).get("tool_sequence")
        if isinstance(diagnostic, dict)
        else []
    )
    return value if isinstance(value, list) and all(isinstance(x, str) for x in value) else []


def _before_count(diagnostic: dict[str, Any] | None) -> int:
    value = (
        diagnostic.get("process_tree", {}).get("before_job_close_processes")
        if isinstance(diagnostic, dict)
        else []
    )
    return len(value) if isinstance(value, list) else -1


def _after_count(diagnostic: dict[str, Any] | None) -> int:
    value = (
        diagnostic.get("process_tree", {}).get("after_job_close_processes")
        if isinstance(diagnostic, dict)
        else []
    )
    return len(value) if isinstance(value, list) else -1


def _provider_terminal_succeeded(diagnostic: dict[str, Any] | None) -> bool:
    return (
        isinstance(diagnostic, dict)
        and diagnostic.get("provider", {}).get("result", {}).get("terminal_state")
        == "succeeded"
    )


def _signature(row: dict[str, Any], diagnostic: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "probe_passed": row.get("passed") is True,
        "probe_failed_with_diagnostic": row.get("passed") is False
        and row.get("timeout_diagnostic_exists") is True
        and isinstance(diagnostic, dict),
        "failure_artifact_exists": row.get("failure_exists") is True,
        "result_artifact_exists": row.get("result_exists") is True,
        "verification_artifact_exists": row.get("verification_exists") is True,
        "diagnostic_kind": diagnostic.get("diagnostic_kind")
        if isinstance(diagnostic, dict)
        else None,
        "scenario": diagnostic.get("scenario") if isinstance(diagnostic, dict) else None,
        "provider_terminal_succeeded": _provider_terminal_succeeded(diagnostic),
        "primary_request_count": diagnostic.get("provider", {}).get(
            "primary_request_count"
        )
        if isinstance(diagnostic, dict)
        else None,
        "tool_sequence": _tool_sequence(diagnostic),
        "grok_returncode_before_job_close": diagnostic.get("grok", {}).get(
            "returncode_before_job_close"
        )
        if isinstance(diagnostic, dict)
        else None,
        "pre_close_nonce_process_count": _before_count(diagnostic),
        "post_job_close_nonce_process_count": _after_count(diagnostic),
        "outer_job_created": diagnostic.get("job", {}).get("created") is True
        if isinstance(diagnostic, dict)
        else False,
        "outer_job_assigned": diagnostic.get("job", {}).get("assigned") is True
        if isinstance(diagnostic, dict)
        else False,
        "outer_job_closed_before_diagnostic": diagnostic.get("job", {}).get(
            "closed_before_diagnostic"
        )
        is True
        if isinstance(diagnostic, dict)
        else False,
        "outer_job_closed_after_diagnostic": diagnostic.get("job", {}).get(
            "closed_after_diagnostic"
        )
        is True
        if isinstance(diagnostic, dict)
        else False,
    }


def verify(summary_path: Path, baseline_version: str, candidate_version: str) -> dict[str, Any]:
    summary_path = summary_path.resolve()
    summary = _read_json(summary_path)
    baseline_row = _result_by_version(summary, baseline_version)
    candidate_row = _result_by_version(summary, candidate_version)
    baseline_diagnostic = _load_diagnostic(baseline_row)
    candidate_diagnostic = _load_diagnostic(candidate_row)
    baseline_signature = _signature(baseline_row, baseline_diagnostic)
    candidate_signature = _signature(candidate_row, candidate_diagnostic)

    evidence_checks = {
        "summary_kind_matches": summary.get("receipt_kind") == "grok-timeout-triage-matrix",
        "scenario_is_tool_timeout": all(
            row.get("output_directory", "").endswith("-tool-timeout")
            for row in (baseline_row, candidate_row)
        ),
        "baseline_diagnostic_replays": baseline_signature[
            "probe_failed_with_diagnostic"
        ],
        "candidate_diagnostic_replays": candidate_signature[
            "probe_failed_with_diagnostic"
        ]
        or candidate_signature["probe_passed"],
        "baseline_provider_completed": baseline_signature[
            "provider_terminal_succeeded"
        ],
        "candidate_provider_completed": (
            candidate_signature["provider_terminal_succeeded"]
            if not candidate_signature["probe_passed"]
            else True
        ),
        "baseline_outer_job_cleanup_zero": baseline_signature[
            "post_job_close_nonce_process_count"
        ]
        == 0,
        "candidate_outer_job_cleanup_zero": (
            candidate_signature["post_job_close_nonce_process_count"] == 0
            if not candidate_signature["probe_passed"]
            else True
        ),
    }
    if candidate_signature["probe_passed"]:
        non_regression_checks = {
            "candidate_probe_passed": True,
            "candidate_has_no_unexplained_probe_failure": True,
            "candidate_pre_close_residue_not_greater": True,
        }
    else:
        comparable_fields = (
            "probe_passed",
            "diagnostic_kind",
            "scenario",
            "provider_terminal_succeeded",
            "primary_request_count",
            "tool_sequence",
            "grok_returncode_before_job_close",
            "post_job_close_nonce_process_count",
        )
        non_regression_checks = {
            f"same_{field}": baseline_signature[field] == candidate_signature[field]
            for field in comparable_fields
        }
        non_regression_checks["candidate_pre_close_residue_not_greater"] = (
            candidate_signature["pre_close_nonce_process_count"]
            <= baseline_signature["pre_close_nonce_process_count"]
        )
        non_regression_checks["candidate_has_no_unexplained_probe_failure"] = (
            candidate_signature["probe_passed"]
            or candidate_signature["probe_failed_with_diagnostic"]
        )

    owned_cleanup_checks = {
        "candidate_probe_reached_terminal_state": candidate_signature["probe_passed"],
        "candidate_had_no_pre_close_nonce_residue": (
            candidate_signature["pre_close_nonce_process_count"] == 0
            if not candidate_signature["probe_passed"]
            else True
        ),
        "candidate_did_not_need_outer_job_cleanup": (
            candidate_signature["outer_job_closed_before_diagnostic"]
            if not candidate_signature["probe_passed"]
            else True
        ),
    }
    baseline_status = (
        "passed"
        if all(evidence_checks.values()) and all(non_regression_checks.values())
        else "failed"
    )
    owned_cleanup_status = (
        "passed"
        if all(evidence_checks.values()) and all(owned_cleanup_checks.values())
        else "carried-limitation"
        if all(evidence_checks.values())
        else "failed"
    )
    prompt_tool_promotion_ready = (
        baseline_status == "passed" and owned_cleanup_status == "passed"
    )
    return {
        "schema_version": "0.1.0",
        "receipt_kind": "grok-timeout-gate-split-verification",
        "verifier": "grok-timeout-gate-split-independent",
        "summary": _artifact(summary_path),
        "baseline_version": baseline_version,
        "candidate_version": candidate_version,
        "valid": all(evidence_checks.values()),
        "prompt_tool_promotion_ready": prompt_tool_promotion_ready,
        "evidence_checks": evidence_checks,
        "gates": {
            "windows_child_tree_baseline_regression": {
                "status": baseline_status,
                "checks": non_regression_checks,
            },
            "windows_child_tree_owned_cleanup": {
                "status": owned_cleanup_status,
                "checks": owned_cleanup_checks,
            },
        },
        "observed_signatures": {
            baseline_version: baseline_signature,
            candidate_version: candidate_signature,
        },
        "limitations": [
            "This verifier splits default-version promotion from prompt/tool-mode promotion readiness.",
            "A passed baseline-regression gate only means the candidate is not worse than the recorded baseline for this timeout shape.",
            "A carried-limitation owned-cleanup gate means Grok-owned full child-tree timeout cleanup is not proven; outer Job Object cleanup remains the containment fallback.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify split Grok Windows child-tree timeout gates"
    )
    parser.add_argument("--summary", type=Path, required=True)
    parser.add_argument("--baseline-version", default="0.2.111")
    parser.add_argument("--candidate-version", default="0.2.112")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        print(f"refusing to overwrite verifier output: {args.output}", file=sys.stderr)
        return 2
    try:
        receipt = verify(args.summary, args.baseline_version, args.candidate_version)
        _atomic_write_json(args.output.resolve(), receipt)
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr)
        return 1
    print(json.dumps(receipt, ensure_ascii=False, sort_keys=True, allow_nan=False))
    return 0 if receipt["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
