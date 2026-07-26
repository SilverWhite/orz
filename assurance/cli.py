from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Sequence

from .canonical_cli import (
    run_canonical_guarded_cli,
    verify_canonical_guarded_cli_run,
)
from .errors import AssuranceError
from .source_visibility import (
    dumps_receipt as dumps_source_visibility_receipt,
    evaluate_source_visibility_ledger_file,
    render_visibility_summary,
)


ROOT = Path(__file__).resolve().parents[1]


def _json_dump(value: Any) -> str:
    return json.dumps(
        value,
        ensure_ascii=False,
        indent=2,
        sort_keys=True,
        allow_nan=False,
    )


def _git_status() -> dict[str, Any]:
    try:
        completed = subprocess.run(
            ["git", "status", "--short", "--branch"],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=10,
        )
    except Exception as exc:
        return {
            "available": False,
            "exit_code": None,
            "summary": f"{type(exc).__name__}: {exc}",
        }
    return {
        "available": completed.returncode == 0,
        "exit_code": completed.returncode,
        "summary": completed.stdout.strip(),
    }


def _doctor_report(*, include_repository_check: bool) -> dict[str, Any]:
    repository_check: dict[str, Any] | None = None
    if include_repository_check:
        from scripts.check_repository import check_repository

        repository_check = check_repository()
    git = _git_status()
    report = {
        "schema_version": "0.1.0-draft",
        "report_kind": "gsa_cli_doctor_report",
        "valid": bool(repository_check["valid"]) if repository_check else True,
        "workspace_root": str(ROOT),
        "python": {
            "version": sys.version.split()[0],
            "executable": sys.executable,
        },
        "git": git,
        "repository_check": repository_check,
        "entrypoints": {
            "doctor": "python gsa.py doctor",
            "source_gate": "python gsa.py source gate --ledger <path>",
            "run": "python gsa.py run --ask <question> --source-ledger <path> --run-root <path>",
            "verify": "python gsa.py verify --run-root <path>",
        },
        "runtime_boundaries": {
            "default_network": "disabled",
            "default_credential_use": "disabled",
            "canonical_run_adapter": "fake_offline",
            "real_model_invocation": "not_performed_by_p0_5_cli",
        },
        "limitations": [
            "doctor verifies repository mechanics and CLI wiring, not scientific correctness.",
            "canonical run remains fake/offline until a separate real adapter path is added.",
        ],
    }
    return report


def _print_or_json(value: dict[str, Any], *, json_output: bool, human_lines: list[str]) -> None:
    if json_output:
        print(_json_dump(value))
        return
    for line in human_lines:
        print(line)


def _run_doctor(args: argparse.Namespace) -> int:
    report = _doctor_report(include_repository_check=not args.quick)
    repository = report["repository_check"]
    if repository is None:
        repo_line = "repository check: skipped"
    else:
        repo_line = (
            f"repository check: {'valid' if repository['valid'] else 'invalid'} "
            f"({repository.get('error_count', 0)} errors)"
        )
    human_lines = [
        f"GSA CLI doctor: {'valid' if report['valid'] else 'invalid'}",
        f"workspace: {report['workspace_root']}",
        f"python: {report['python']['version']}",
        repo_line,
        "canonical run: fake/offline, network disabled, credentials disabled",
    ]
    _print_or_json(report, json_output=args.json, human_lines=human_lines)
    return 0 if report["valid"] else 1


def _run_source_gate(args: argparse.Namespace) -> int:
    receipt = evaluate_source_visibility_ledger_file(args.ledger)
    if args.summary:
        print(render_visibility_summary(receipt))
    elif args.json:
        print(dumps_source_visibility_receipt(receipt))
    else:
        lines = [
            f"source gate: {receipt['decision']}",
            f"sources: {receipt['source_count']}",
            f"references: {receipt['reference_count']}",
            f"context tokens estimate: {receipt['context_tokens_estimate']}",
        ]
        for item in receipt["reference_decisions"]:
            lines.append(
                f"{item['ref_id']}: {item['observed_visibility']} -> "
                f"{item['decision']} ({item['claim_allowed']})"
            )
        print("\n".join(lines))
    return 0 if receipt["decision"] != "block" else 1


def _run_canonical(args: argparse.Namespace) -> int:
    receipt = run_canonical_guarded_cli(
        run_root=args.run_root,
        source_ledger_path=args.source_ledger,
        ask=args.ask,
        task_contract_path=args.task,
        run_id=args.run_id,
        task_id=args.task_id,
        created_at=args.created_at,
    )
    human_lines = [
        f"canonical run: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['run_root']}",
        f"events: {receipt['event_count']} ({', '.join(receipt['event_types'])})",
        f"terminal: {receipt['terminal_event']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_verify(args: argparse.Namespace) -> int:
    receipt = verify_canonical_guarded_cli_run(run_root=args.run_root)
    human_lines = [
        f"verify: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"events: {receipt['event_count']}",
        f"terminal: {receipt['terminal_event']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="gsa",
        description="General Scientific Assurance CLI dispatcher.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    doctor = subparsers.add_parser("doctor", help="Check local CLI readiness.")
    doctor.add_argument("--json", action="store_true", help="Emit JSON report.")
    doctor.add_argument(
        "--quick",
        action="store_true",
        help="Skip repository contract check for a fast wiring check.",
    )
    doctor.set_defaults(handler=_run_doctor)

    source = subparsers.add_parser("source", help="Source-related gates.")
    source_subparsers = source.add_subparsers(dest="source_command", required=True)
    gate = source_subparsers.add_parser("gate", help="Evaluate source visibility gate.")
    gate.add_argument("--ledger", type=Path, required=True)
    gate.add_argument("--json", action="store_true")
    gate.add_argument("--summary", action="store_true")
    gate.set_defaults(handler=_run_source_gate)

    run = subparsers.add_parser("run", help="Run canonical guarded CLI offline path.")
    run.add_argument("--run-root", type=Path, required=True)
    run.add_argument("--source-ledger", type=Path)
    run.add_argument("--ask")
    run.add_argument("--task", type=Path)
    run.add_argument("--run-id", default="RUN-CANONICAL-CLI-FAKE-001")
    run.add_argument("--task-id", default="TASK-CANONICAL-CLI-FAKE-001")
    run.add_argument("--created-at", default=None)
    run.add_argument("--json", action="store_true")
    run.set_defaults(handler=_run_canonical)

    verify = subparsers.add_parser("verify", help="Verify canonical guarded CLI run root.")
    verify.add_argument("--run-root", type=Path, required=True)
    verify.add_argument("--json", action="store_true")
    verify.set_defaults(handler=_run_verify)

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        return int(args.handler(args))
    except AssuranceError as exc:
        print(
            _json_dump(
                {
                    "valid": False,
                    "error_type": "AssuranceError",
                    "error": str(exc),
                }
            ),
            file=sys.stderr,
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
