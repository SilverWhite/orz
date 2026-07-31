from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
from typing import Any, Sequence

from .canonical_cli import (
    run_canonical_guarded_cli,
    run_canonical_guarded_cli_real,
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
            "run_fake": "python gsa.py run --ask <q> --source-ledger <path> --run-root <path>",
            "run_real": "python gsa.py run --mode real --ask <q> --source-ledger <path> --run-root <path>",
            "run_grok_gate": "python gsa.py run --runtime grok --ask <q> --run-root <path>",
            "verify": "python gsa.py verify --run-root <path>",
        },
        "runtime_boundaries": {
            "default_network": "disabled",
            "default_credential_use": "disabled",
            "canonical_run_fake": "offline_no_network",
            "canonical_run_real": "deepseek_api_exactly_one_request",
            "grok_run_prompt_tool_gate": "explicit_fail_closed_no_prompt_or_tool_execution",
        },
        "limitations": [
            "doctor verifies repository mechanics and CLI wiring, not scientific correctness.",
            "canonical run supports fake (offline) and real (DeepSeek API) adapter modes.",
            "real adapter reads credentials from Windows Credential Manager and never persists them.",
            "gsa run --runtime grok emits a promotion gate receipt only; it does not launch prompt/tool execution.",
            "GSA-CORE review and evaluation bridges exist but have not been connected to a live runtime.",
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
        f"canonical run: fake (offline) and real (DeepSeek API, {report['runtime_boundaries']['canonical_run_real']})",
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


def _run_grok_prompt_tool_gate(args: argparse.Namespace) -> int:
    from .grok_prompt_tool_gate import (
        DEFAULT_RETRIEVAL_MODE,
        write_grok_prompt_tool_promotion_gate_receipt,
    )

    if args.source_ledger or args.instruction_context or args.task:
        raise AssuranceError(
            "--source-ledger, --instruction-context, and --task are canonical-only "
            "for gsa run; use --runtime canonical or omit them for --runtime grok"
        )
    if args.mode != "fake":
        raise AssuranceError("--mode real is canonical-only for gsa run")

    retrieval_mode_explicit = args.retrieval_mode is not None
    retrieval_mode = args.retrieval_mode or DEFAULT_RETRIEVAL_MODE
    run_id = (
        None
        if args.run_id == "RUN-CANONICAL-CLI-FAKE-001"
        else args.run_id
    )
    receipt = write_grok_prompt_tool_promotion_gate_receipt(
        run_root=args.run_root,
        ask=args.ask,
        retrieval_mode=retrieval_mode,
        retrieval_mode_explicit=retrieval_mode_explicit,
        run_id=run_id,
        timeout_gate_path=args.grok_timeout_gate,
        tool_availability_gate_path=args.grok_tool_availability_gate,
        adapter_receipt_path=args.grok_adapter_receipt,
    )

    # If gate allows and --grok-execute is set, run the selected Grok mode.
    executed = False
    if receipt["decision"] == "allow" and getattr(args, "grok_execute", False):
        if not args.ask:
            raise AssuranceError(
                "--ask is required when --grok-execute is set"
            )
        grok_mode = getattr(args, "grok_mode", "acp-smoke")
        from .grok_runtime_adapter import (
            GrokRunRequest,
            run_grok_headless_once,
            run_grok_acp_once,
        )
        adapter_mode = "acp_smoke" if grok_mode == "acp-smoke" else "prompt_smoke"
        adapter_request = GrokRunRequest(
            run_root=args.run_root,
            workspace_path=args.run_root / "workspace",
            mode=adapter_mode,
            prompt_text=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
        )
        if adapter_mode == "acp_smoke":
            adapter_receipt = run_grok_acp_once(adapter_request)
        else:
            adapter_receipt = run_grok_headless_once(adapter_request)
        executed = True
        # Rebuild the gate receipt with execution outcome attached.
        from .grok_prompt_tool_gate import (
            build_grok_prompt_tool_promotion_gate_receipt,
        )
        from .utils import load_json as _load_json
        receipt = build_grok_prompt_tool_promotion_gate_receipt(
            run_root=args.run_root,
            ask=args.ask,
            retrieval_mode=retrieval_mode,
            retrieval_mode_explicit=retrieval_mode_explicit,
            run_id=run_id,
            timeout_gate_receipt=(
                _load_json(args.grok_timeout_gate)
                if args.grok_timeout_gate else None
            ),
            tool_availability_gate_receipt=(
                _load_json(args.grok_tool_availability_gate)
                if args.grok_tool_availability_gate else None
            ),
            adapter_receipt=(
                _load_json(args.grok_adapter_receipt)
                if args.grok_adapter_receipt else None
            ),
            execution_outcome={
                "outcome": (
                    "completed" if adapter_receipt.get("valid") else "failed"
                ),
                "mode": adapter_mode,
                "receipt_path": str(
                    (args.run_root / "grok-runtime-receipt.json").resolve()
                ),
                "events_path": str(
                    (args.run_root / "events.jsonl").resolve()
                ),
            },
        )
        from .utils import atomic_write_json
        atomic_write_json(
            args.run_root / "grok-prompt-tool-promotion-gate.json",
            receipt,
            overwrite=True,
        )

    human_lines = [
        f"grok prompt/tool gate: {receipt['decision']}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"retrieval mode: {receipt['request']['retrieval_mode']}",
        f"prompt/tool execution attempted: {str(executed).lower()}",
    ]
    if receipt["blocking_reasons"]:
        human_lines.append(
            f"blocking reasons: {', '.join(receipt['blocking_reasons'])}"
        )
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["decision"] == "allow" else 1


def _run_canonical(args: argparse.Namespace) -> int:
    if args.runtime == "grok":
        return _run_grok_prompt_tool_gate(args)
    if args.retrieval_mode or args.grok_timeout_gate or args.grok_tool_availability_gate or args.grok_adapter_receipt:
        raise AssuranceError(
            "--retrieval-mode, --grok-timeout-gate, "
            "--grok-tool-availability-gate, and --grok-adapter-receipt "
            "are only valid with --runtime grok"
        )
    if args.mode == "real":
        receipt = run_canonical_guarded_cli_real(
            run_root=args.run_root,
            source_ledger_path=args.source_ledger,
            instruction_provenance_gate_context_path=args.instruction_context,
            ask=args.ask,
            task_contract_path=args.task,
            run_id=args.run_id,
            task_id=args.task_id,
            created_at=args.created_at,
            credential_target=args.credential_target,
            api_timeout_seconds=args.api_timeout,
        )
    else:
        receipt = run_canonical_guarded_cli(
            run_root=args.run_root,
            source_ledger_path=args.source_ledger,
            instruction_provenance_gate_context_path=args.instruction_context,
            ask=args.ask,
            task_contract_path=args.task,
            run_id=args.run_id,
            task_id=args.task_id,
            created_at=args.created_at,
        )
    mode_label = "real" if args.mode == "real" else "fake"
    human_lines = [
        f"canonical run [{mode_label}]: {'valid' if receipt['valid'] else 'invalid'}",
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


def _run_grok_doctor(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import SUPPORTED_RETRIEVAL_MODES, inspect_grok_runtime

    inspection = inspect_grok_runtime()
    report = {
        "schema_version": "0.1.0-draft",
        "report_kind": "grok_cli_doctor_report",
        "valid": inspection.get("valid") is True,
        "workspace_root": str(ROOT),
        "inspection": inspection,
        "entrypoints": {
            "doctor": "python gsa.py grok doctor",
            "version_smoke": (
                "python gsa.py grok run --mode version-smoke "
                "--run-root <path>"
            ),
            "tui_version_smoke": (
                "python gsa.py tui --runtime grok --run version-smoke "
                "--run-root <path>"
            ),
        },
        "runtime_boundaries": {
            "supported_mode": "version-smoke",
            "retrieval_modes": list(SUPPORTED_RETRIEVAL_MODES),
            "default_retrieval_mode": "off",
            "network": "disabled",
            "prompt_submission": "not_supported_in_this_slice",
            "tool_execution": "not_supported_in_this_slice",
            "no_residue_required": True,
            "residue_scan_scope": "root_process_only_for_version_smoke",
        },
        "limitations": [
            "Grok CLI wiring currently supports only locked-binary doctor and version-smoke execution.",
            "Prompt and tool modes remain blocked until stronger Windows child-tree containment is wired.",
        ],
    }
    observed = inspection.get("observed", {}) if isinstance(inspection, dict) else {}
    human_lines = [
        f"Grok CLI doctor: {'valid' if report['valid'] else 'invalid'}",
        f"binary: {inspection.get('binary_path', '')}",
        f"version: {observed.get('version_output', '')}",
        f"sha256 match: {inspection.get('checks', {}).get('sha256_match')}",
        "supported run mode: version-smoke",
        "retrieval modes: local_browser, framework_fallback, off",
        "no-residue required: true",
    ]
    _print_or_json(report, json_output=args.json, human_lines=human_lines)
    return 0 if report["valid"] else 1


def _run_grok(args: argparse.Namespace) -> int:
    from .grok_runtime_adapter import DEFAULT_RETRIEVAL_MODE, run_grok_version_smoke

    if args.mode != "version-smoke":
        raise AssuranceError(f"unsupported Grok run mode: {args.mode}")
    retrieval_mode_explicit = args.retrieval_mode is not None
    retrieval_mode = args.retrieval_mode or DEFAULT_RETRIEVAL_MODE
    receipt = run_grok_version_smoke(
        run_root=args.run_root,
        workspace_path=args.workspace,
        run_id=args.run_id,
        retrieval_mode=retrieval_mode,
        retrieval_mode_explicit=retrieval_mode_explicit,
    )
    containment = receipt.get("containment", {})
    human_lines = [
        f"grok run [version-smoke]: {'valid' if receipt['valid'] else 'invalid'}",
        f"run_id: {receipt['run_id']}",
        f"run_root: {receipt['request']['run_root']}",
        f"version: {receipt['binary']['version_output']}",
        f"retrieval mode: {receipt['retrieval']['mode']}",
        f"no residue observed: {containment.get('no_residue_observed')}",
        f"external cleanup required: {containment.get('external_cleanup_required')}",
        f"events: {receipt['artifacts']['events_path']}",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_grok_observe_tools(args: argparse.Namespace) -> int:
    from .grok_tool_permission_observer import (
        build_grok_tool_permission_observation_bundle,
        observe_grok_tool_permission_surfaces,
    )
    from .utils import atomic_write_json

    receipt = observe_grok_tool_permission_surfaces(
        acp_verification_paths=args.acp_verification
    )
    bundle = build_grok_tool_permission_observation_bundle(receipt)
    gate = bundle["tool_availability_gate_receipt"]
    human_lines = [
        f"grok observe-tools: {receipt['decision']}",
        f"receipt_id: {receipt['receipt_id']}",
        f"project agents: {', '.join(receipt['surface']['project_agents'])}",
        f"permission controls observed: {receipt['checks']['permission_controls_observed']}",
        f"acp verification attached: {receipt['checks']['acp_permission_probe_attached']}",
        f"tool availability gate: {gate['decisions']['gate_decision']}",
        "prompt/tool promotion: blocked",
    ]
    if args.output_gate_receipt:
        atomic_write_json(args.output_gate_receipt, gate)
        human_lines.append(
            f"gate receipt written: {args.output_gate_receipt}"
        )
    payload = bundle if args.include_tool_availability else receipt
    _print_or_json(payload, json_output=args.json, human_lines=human_lines)
    return 0 if receipt["valid"] else 1


def _run_global_review(args: argparse.Namespace) -> int:
    from .global_review_mode import build_global_review_mode_receipt
    from .utils import atomic_write_json

    receipt = build_global_review_mode_receipt(
        paths=args.path,
        include_git_status=not args.no_git_status,
        review_id=args.review_id,
    )
    if args.output:
        atomic_write_json(args.output, receipt)
    triggered = sorted(
        {
            flag
            for finding in receipt["path_findings"]
            for flag in finding["risk_flags"]
        }
    )
    output_line = f"receipt: {args.output}" if args.output else "receipt: stdout"
    human_lines = [
        "global review mode: active",
        f"review_id: {receipt['review_id']}",
        f"scope: {receipt['scope']['source']} ({receipt['scope']['path_count']} paths)",
        f"dimensions: {len(receipt['dimensions'])} required",
        f"risk flags: {', '.join(triggered) if triggered else 'none'}",
        output_line,
        "ordinary review remains: local_engineering_review",
    ]
    _print_or_json(receipt, json_output=args.json, human_lines=human_lines)
    return 0


def _run_tui(args: argparse.Namespace) -> int:
    from assurance.tui.main import main as tui_main

    if args.grok_retrieval_mode and args.runtime != "grok":
        print("--retrieval-mode is only valid with --runtime grok", file=sys.stderr)
        return 2

    tui_args: list[str] = [
        "--width",
        str(args.width),
        "--height",
        str(args.height),
        "--runtime",
        args.runtime,
    ]
    if args.demo:
        tui_args.append("--demo")
    if args.with_dialog:
        tui_args.append("--with-dialog")
    if args.with_properties:
        tui_args.append("--with-properties")
    if args.demo_events:
        tui_args.extend(["--demo-events", args.demo_events])
    if args.live_retrieval:
        tui_args.append("--live-retrieval")
    if args.replay:
        tui_args.extend(["--replay", str(args.replay)])
    if args.tui_run:
        tui_args.extend(["--run", args.tui_run])
    if args.real:
        tui_args.append("--real")
    if args.credential_target:
        tui_args.extend(["--credential-target", args.credential_target])
    if args.tui_run_root:
        tui_args.extend(["--run-root", str(args.tui_run_root)])
    if args.workspace:
        tui_args.extend(["--workspace", str(args.workspace)])
    if args.grok_retrieval_mode:
        tui_args.extend(["--retrieval-mode", args.grok_retrieval_mode])
    return tui_main(tui_args)


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

    run = subparsers.add_parser(
        "run",
        help="Run the selected guarded CLI runtime path.",
    )
    run.add_argument("--run-root", type=Path, required=True)
    run.add_argument(
        "--runtime",
        choices=["canonical", "grok"],
        default="canonical",
        help=(
            "Runtime path. canonical is the default; grok emits an explicit "
            "fail-closed prompt/tool promotion gate receipt."
        ),
    )
    run.add_argument("--source-ledger", type=Path)
    run.add_argument("--instruction-context", type=Path)
    run.add_argument("--ask")
    run.add_argument("--task", type=Path)
    run.add_argument("--run-id", default="RUN-CANONICAL-CLI-FAKE-001")
    run.add_argument("--task-id", default="TASK-CANONICAL-CLI-FAKE-001")
    run.add_argument("--created-at", default=None)
    run.add_argument("--json", action="store_true")
    run.add_argument(
        "--mode",
        choices=["fake", "real"],
        default="fake",
        help="Adapter mode: fake (offline, no network) or real (calls DeepSeek API).",
    )
    run.add_argument(
        "--credential-target",
        default="FEP-Agent/DeepSeek",
        help="Windows Credential Manager target name for the DeepSeek API key.",
    )
    run.add_argument(
        "--api-timeout",
        type=int,
        default=60,
        help="Timeout in seconds for the DeepSeek API call.",
    )
    run.add_argument(
        "--retrieval-mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for --runtime grok promotion gating. "
            "The gate records the mode but does not perform retrieval."
        ),
    )
    run.add_argument(
        "--grok-timeout-gate",
        type=Path,
        help=(
            "Attach a grok-timeout-gate-split verification receipt for "
            "--runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-tool-availability-gate",
        type=Path,
        help=(
            "Attach a tool-availability gate receipt produced from Grok "
            "observe-tools for --runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-adapter-receipt",
        type=Path,
        help=(
            "Attach a Grok runtime adapter receipt with containment "
            "evidence for --runtime grok promotion gating."
        ),
    )
    run.add_argument(
        "--grok-execute",
        action="store_true",
        help=(
            "When --runtime grok and gate decision is 'allow', execute "
            "a Grok session with the locked binary. Mode is selected "
            "by --grok-mode (default: acp-smoke)."
        ),
    )
    run.add_argument(
        "--grok-mode",
        choices=["prompt-smoke", "acp-smoke"],
        default="acp-smoke",
        help=(
            "Execution mode for --grok-execute. "
            "'acp-smoke' uses grok agent stdio (ACP JSON-RPC, primary); "
            "'prompt-smoke' uses headless --prompt-file (narrow smoke only)."
        ),
    )
    run.set_defaults(handler=_run_canonical)

    verify = subparsers.add_parser("verify", help="Verify canonical guarded CLI run root.")
    verify.add_argument("--run-root", type=Path, required=True)
    verify.add_argument("--json", action="store_true")
    verify.set_defaults(handler=_run_verify)

    grok = subparsers.add_parser("grok", help="Grok runtime adapter commands.")
    grok_subparsers = grok.add_subparsers(dest="grok_command", required=True)
    grok_doctor = grok_subparsers.add_parser(
        "doctor",
        help="Inspect the locked Grok binary and runtime boundary.",
    )
    grok_doctor.add_argument("--json", action="store_true")
    grok_doctor.set_defaults(handler=_run_grok_doctor)
    grok_run = grok_subparsers.add_parser(
        "run",
        help="Run a supported Grok adapter smoke.",
    )
    grok_run.add_argument("--run-root", type=Path, required=True)
    grok_run.add_argument(
        "--workspace",
        type=Path,
        help=(
            "Workspace directory to trust and use as Grok cwd "
            "(default: isolated directory under --run-root)."
        ),
    )
    grok_run.add_argument(
        "--mode",
        choices=["version-smoke"],
        default="version-smoke",
        help="Grok adapter mode. Only version-smoke is currently promoted.",
    )
    grok_run.add_argument(
        "--retrieval-mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for future Grok prompt/tool runs. "
            "version-smoke records the selection but never performs retrieval."
        ),
    )
    grok_run.add_argument("--run-id")
    grok_run.add_argument("--json", action="store_true")
    grok_run.set_defaults(handler=_run_grok)
    grok_observe_tools = grok_subparsers.add_parser(
        "observe-tools",
        help="Observe Grok tool registry and permission controls without prompt/tool execution.",
    )
    grok_observe_tools.add_argument(
        "--acp-verification",
        type=Path,
        action="append",
        default=[],
        help="Attach a grok-acp-fake-tool-probe verification JSON receipt.",
    )
    grok_observe_tools.add_argument(
        "--include-tool-availability",
        action="store_true",
        help="Include the projected tool availability report and gate receipt.",
    )
    grok_observe_tools.add_argument(
        "--output-gate-receipt",
        type=Path,
        help="Write only the tool availability gate receipt to a file for use with --runtime grok promotion gating.",
    )
    grok_observe_tools.add_argument("--json", action="store_true")
    grok_observe_tools.set_defaults(handler=_run_grok_observe_tools)

    review = subparsers.add_parser("review", help="Review-mode utilities.")
    review_subparsers = review.add_subparsers(dest="review_command", required=True)
    global_review = review_subparsers.add_parser(
        "global",
        help="Explicitly activate Global Review Mode.",
    )
    global_review.add_argument(
        "--path",
        type=Path,
        action="append",
        default=[],
        help="Path to include in the global-review scope; repeatable.",
    )
    global_review.add_argument(
        "--no-git-status",
        action="store_true",
        help="Use only explicit --path values instead of also reading git status.",
    )
    global_review.add_argument("--review-id")
    global_review.add_argument("--output", type=Path)
    global_review.add_argument("--json", action="store_true")
    global_review.set_defaults(handler=_run_global_review)

    tui = subparsers.add_parser("tui", help="Render or run the terminal UI prototype.")
    tui.add_argument(
        "-w",
        "--width",
        type=int,
        default=100,
        help="Viewport width in terminal cells.",
    )
    tui.add_argument(
        "-H",
        "--height",
        type=int,
        default=30,
        help="Viewport height in terminal rows.",
    )
    tui.add_argument(
        "--demo",
        action="store_true",
        help="Run the interactive prompt_toolkit demo instead of a static render.",
    )
    tui.add_argument(
        "--with-dialog",
        action="store_true",
        help="Show the permission dialog overlay in static render/demo.",
    )
    tui.add_argument(
        "--with-properties",
        action="store_true",
        help="Show the properties sheet overlay in static render/demo.",
    )
    tui.add_argument(
        "--demo-events",
        choices=["gate_chain", "gate_defer", "run_failed"],
        help="Run the interactive demo with a live event scenario.",
    )
    tui.add_argument(
        "--live-retrieval",
        action="store_true",
        help="Wire the live browser retrieval handler.",
    )
    tui.add_argument(
        "--replay",
        type=Path,
        help="Replay events from an existing events.jsonl file.",
    )
    tui.add_argument(
        "--run",
        dest="tui_run",
        help="Run the selected runtime and display it in the TUI.",
    )
    tui.add_argument(
        "--runtime",
        choices=["canonical", "grok"],
        default="canonical",
        help="Runtime to use for --run.",
    )
    tui.add_argument(
        "--real",
        action="store_true",
        help="Use the real DeepSeek adapter for --run.",
    )
    tui.add_argument(
        "--credential-target",
        default="FEP-Agent/DeepSeek",
        help="Windows Credential Manager target name for the DeepSeek API key.",
    )
    tui.add_argument(
        "--run-root",
        dest="tui_run_root",
        type=Path,
        help="Run root directory for --run.",
    )
    tui.add_argument(
        "--workspace",
        type=Path,
        help="Workspace directory for Grok runtime --run.",
    )
    tui.add_argument(
        "--retrieval-mode",
        dest="grok_retrieval_mode",
        choices=["local_browser", "framework_fallback", "off"],
        default=None,
        help=(
            "Explicit retrieval mode for Grok prompt/tool runs. "
            "Forwarded only when --runtime grok is selected."
        ),
    )
    tui.set_defaults(handler=_run_tui)

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
