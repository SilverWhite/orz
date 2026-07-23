from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any, Sequence

from .action_kernel import create_action_kernel_smoke, verify_action_kernel
from .approval_ledger import ConsoleConfirmationIO
from .errors import PrototypeError
from .deepseek_adapter import validate_deepseek_profile, validate_tool_history
from .deepseek_external import inspect_deepseek_external_readiness
from .exporter import export_scenarios
from .io_utils import atomic_write_json, load_json
from .journal import create_journal_smoke, replay_journal
from .layout import REGRESSION_ROOT, RUNTIME_ROOT
from .model_loop import (
    create_deepseek_brokered_fake_https_smoke,
    create_deepseek_interactive_fake_https_smoke,
    create_deepseek_loopback_http_smoke,
    create_deepseek_model_loop_smoke,
    verify_model_loop,
)
from .real_development_probe import (
    create_real_development_plan,
    execute_real_development_probe,
)
from .scanner import scan_bundle
from .schema import validate_instance
from .session_validator import validate_session_record
from .windows_process import create_windows_process_smoke


def _print_json(value: Any, *, stream: Any | None = None) -> None:
    if stream is None:
        stream = sys.stdout
    print(
        json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True, allow_nan=False),
        file=stream,
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="fep-agent-proto",
        description=(
            "Development-only contract probes; real model access exists only in the "
            "explicit two-phase one-shot command."
        ),
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    export = subparsers.add_parser("export", help="Export oracle-trimmed scenarios")
    export.add_argument(
        "--corpus",
        type=Path,
        default=REGRESSION_ROOT / "cases-v0.1.yaml",
    )
    export.add_argument("--case-id", action="append", required=True, dest="case_ids")
    export.add_argument(
        "--policy-mode", choices=("development", "challenge"), required=True
    )
    export.add_argument("--token-seed", required=True)
    export.add_argument("--output-dir", type=Path, required=True)

    scan = subparsers.add_parser("scan", help="Mechanically scan an exported bundle")
    scan.add_argument("--export-root", type=Path, required=True)
    scan.add_argument("--report-out", type=Path)

    smoke = subparsers.add_parser(
        "journal-smoke", help="Create a no-model manifest and hash-chained journal"
    )
    smoke.add_argument("--export-root", type=Path, required=True)
    smoke.add_argument("--output-dir", type=Path, required=True)

    replay = subparsers.add_parser("replay", help="Verify a journal hash chain")
    replay.add_argument("--run-manifest", type=Path, required=True)
    replay.add_argument("--journal", type=Path, required=True)
    replay.add_argument("--report-out", type=Path)

    validate_session = subparsers.add_parser(
        "validate-session",
        help="Validate a session record against the draft schema and kernel invariants",
    )
    validate_session.add_argument("--input", type=Path, required=True, dest="input_path")
    validate_session.add_argument("--report-out", type=Path)

    deepseek = subparsers.add_parser(
        "validate-deepseek-profile",
        help="Validate a DeepSeek adapter profile and optional normalized message history",
    )
    deepseek.add_argument("--input", type=Path, required=True, dest="input_path")
    deepseek.add_argument("--messages", type=Path)
    deepseek.add_argument("--report-out", type=Path)

    external_readiness = subparsers.add_parser(
        "deepseek-external-readiness",
        help="Inspect the fail-closed DeepSeek HTTPS path without credentials or network",
    )
    external_readiness.add_argument(
        "--profile", type=Path, required=True, dest="profile_path"
    )
    external_readiness.add_argument("--report-out", type=Path)

    windows_process = subparsers.add_parser(
        "windows-process-smoke",
        help="Run a no-shell Windows Job Object process-control smoke",
    )
    windows_process.add_argument("--output-dir", type=Path, required=True)

    action_kernel = subparsers.add_parser(
        "action-kernel-smoke",
        help="Run one fixed no-model Windows action through manifest, journal, and session",
    )
    action_kernel.add_argument("--export-root", type=Path, required=True)
    action_kernel.add_argument("--output-dir", type=Path, required=True)

    verify_kernel = subparsers.add_parser(
        "verify-action-kernel",
        help="Verify an action-kernel output root and its cross-file digests",
    )
    verify_kernel.add_argument("--output-dir", type=Path, required=True)

    model_loop = subparsers.add_parser(
        "deepseek-model-loop-smoke",
        help="Run a two-turn DeepSeek tool loop through a scripted no-network transport",
    )
    model_loop.add_argument("--export-root", type=Path, required=True)
    model_loop.add_argument("--profile", type=Path, required=True)
    model_loop.add_argument("--output-dir", type=Path, required=True)

    loopback_model = subparsers.add_parser(
        "deepseek-loopback-http-smoke",
        help="Run the DeepSeek mock loop over an ephemeral 127.0.0.1 HTTP/SSE server",
    )
    loopback_model.add_argument("--export-root", type=Path, required=True)
    loopback_model.add_argument("--profile", type=Path, required=True)
    loopback_model.add_argument("--output-dir", type=Path, required=True)

    brokered_fake = subparsers.add_parser(
        "deepseek-brokered-fake-https-smoke",
        help="Run the two-turn loop through per-attempt permits and an in-process fake HTTPS provider",
    )
    brokered_fake.add_argument("--export-root", type=Path, required=True)
    brokered_fake.add_argument("--profile", type=Path, required=True)
    brokered_fake.add_argument("--output-dir", type=Path, required=True)

    interactive_fake = subparsers.add_parser(
        "deepseek-interactive-fake-https-smoke",
        help="Require a typed digest confirmation for each in-process fake HTTPS attempt",
    )
    interactive_fake.add_argument("--export-root", type=Path, required=True)
    interactive_fake.add_argument("--profile", type=Path, required=True)
    interactive_fake.add_argument("--output-dir", type=Path, required=True)

    real_plan = subparsers.add_parser(
        "deepseek-real-development-plan",
        help="Create a fixed, no-network plan for one minimal DeepSeek development request",
    )
    real_plan.add_argument("--output-dir", type=Path, required=True)

    real_execute = subparsers.add_parser(
        "deepseek-real-development-execute",
        help="Interactively authorize and execute the exact planned one-shot request",
    )
    real_execute.add_argument("--plan", type=Path, required=True, dest="plan_path")

    verify_model = subparsers.add_parser(
        "verify-model-loop",
        help="Verify a persisted scripted model-loop output root",
    )
    verify_model.add_argument("--output-dir", type=Path, required=True)
    return parser


def _run(args: argparse.Namespace) -> tuple[dict[str, Any], int]:
    if args.command == "export":
        result = export_scenarios(
            corpus_path=args.corpus,
            output_dir=args.output_dir,
            case_ids=args.case_ids,
            policy_mode=args.policy_mode,
            token_seed=args.token_seed,
        )
        return result, 0 if result["status"] == "ready" else 2

    if args.command == "scan":
        manifest_path = args.export_root / "scenario-export-manifest.json"
        manifest = load_json(manifest_path)
        validate_instance(
            manifest,
            RUNTIME_ROOT / "scenario-export-manifest-v0.1.schema.json",
            label="scenario export manifest",
        )
        result = scan_bundle(
            args.export_root / "scenario_bundle",
            export_id=manifest["export_id"],
            policy_mode=manifest["partition"],
            expected_bundle_sha256=manifest["bundle_sha256"],
        )
        if args.report_out:
            atomic_write_json(args.report_out, result)
        return result, 0 if result["gate_action"] == "allow" else 2

    if args.command == "journal-smoke":
        result = create_journal_smoke(
            export_root=args.export_root,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] else 2

    if args.command == "replay":
        result = replay_journal(
            run_manifest_path=args.run_manifest,
            journal_path=args.journal,
        )
        if args.report_out:
            atomic_write_json(args.report_out, result)
        return result, 0 if result["valid"] else 2

    if args.command == "validate-session":
        result = validate_session_record(load_json(args.input_path))
        if args.report_out:
            atomic_write_json(args.report_out, result)
        return result, 0 if result["valid"] else 2

    if args.command == "validate-deepseek-profile":
        profile = load_json(args.input_path)
        result = validate_deepseek_profile(profile)
        if args.messages:
            messages = load_json(args.messages)
            if not isinstance(messages, list):
                raise PrototypeError("DeepSeek message history must be a JSON array")
            transcript = validate_tool_history(
                messages,
                thinking_enabled=profile["thinking"]["type"] == "enabled",
            )
            result["transcript"] = transcript
            result["valid"] = result["valid"] and transcript["valid"]
        if args.report_out:
            atomic_write_json(args.report_out, result)
        return result, 0 if result["valid"] else 2

    if args.command == "deepseek-external-readiness":
        result = inspect_deepseek_external_readiness(load_json(args.profile_path))
        if args.report_out:
            atomic_write_json(args.report_out, result)
        return result, 0 if result["valid"] and result["ready_for_network"] else 2

    if args.command == "windows-process-smoke":
        result = create_windows_process_smoke(output_dir=args.output_dir)
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "action-kernel-smoke":
        result = create_action_kernel_smoke(
            export_root=args.export_root,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "verify-action-kernel":
        result = verify_action_kernel(output_dir=args.output_dir)
        return result, 0 if result["valid"] else 2

    if args.command == "deepseek-model-loop-smoke":
        result = create_deepseek_model_loop_smoke(
            export_root=args.export_root,
            profile_path=args.profile,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "deepseek-loopback-http-smoke":
        result = create_deepseek_loopback_http_smoke(
            export_root=args.export_root,
            profile_path=args.profile,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "deepseek-brokered-fake-https-smoke":
        result = create_deepseek_brokered_fake_https_smoke(
            export_root=args.export_root,
            profile_path=args.profile,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "deepseek-interactive-fake-https-smoke":
        result = create_deepseek_interactive_fake_https_smoke(
            export_root=args.export_root,
            profile_path=args.profile,
            output_dir=args.output_dir,
        )
        return result, 0 if result["valid"] and result["runtime_ready"] else 2

    if args.command == "deepseek-real-development-plan":
        result = create_real_development_plan(output_dir=args.output_dir)
        return result, 0

    if args.command == "deepseek-real-development-execute":
        result = execute_real_development_probe(
            plan_path=args.plan_path,
            confirmation_io=ConsoleConfirmationIO(attempt_label="real-network"),
        )
        return result, 0 if result["valid"] else 2

    if args.command == "verify-model-loop":
        result = verify_model_loop(output_dir=args.output_dir)
        return result, 0 if result["valid"] else 2

    raise PrototypeError(f"unknown command: {args.command}")


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    args = parser.parse_args(argv)
    try:
        result, exit_code = _run(args)
    except PrototypeError as exc:
        _print_json(
            {"status": "error", "error_type": type(exc).__name__, "error": str(exc)},
            stream=sys.stderr,
        )
        return 3
    _print_json(result)
    return exit_code


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
