from __future__ import annotations

import argparse
import os
from pathlib import Path

from .conversation import ConversationNamespace
from .integrated_run import (
    execute_workspace_first_integrated_run,
    initialize_workspace_marker,
    verify_workspace_first_integrated_run,
)
from .keystore import (
    MemoryInstallationKeyStore,
    WindowsDpapiInstallationKeyStore,
)
from .utils import atomic_write_json, load_json


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="P4.5 workspace-first integrated assurance tools"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    initialize = commands.add_parser(
        "workspace-init",
        help="mark an existing disposable workspace for the fixed P4.5 action",
    )
    initialize.add_argument("--workspace", type=Path, required=True)
    run = commands.add_parser(
        "standard-run",
        help="run P3 authorization, a fixed in-process action, P4 audit, and archive",
    )
    run.add_argument("--workspace", type=Path, required=True)
    run.add_argument("--run-root", type=Path, required=True)
    verify = commands.add_parser(
        "verify",
        help="independently verify an archived P4.5 run",
    )
    verify.add_argument("--workspace", type=Path, required=True)
    verify.add_argument("--run-root", type=Path, required=True)
    return parser


def _single_namespace(run_root: Path) -> ConversationNamespace:
    roots = [
        path
        for path in (run_root / "conversations").iterdir()
        if path.is_dir()
    ]
    if len(roots) != 1:
        raise RuntimeError(
            "P4.5 run must contain exactly one conversation namespace"
        )
    return ConversationNamespace(roots[0])


def main() -> int:
    args = _parser().parse_args()
    if args.command == "workspace-init":
        marker = initialize_workspace_marker(args.workspace)
        print(marker)
        return 0

    run_root = args.run_root.resolve()
    if args.command == "standard-run":
        if run_root.exists():
            raise FileExistsError(f"refusing to overwrite run root: {run_root}")
        run_root.mkdir(parents=True)
        if os.name == "nt":
            key_store = WindowsDpapiInstallationKeyStore.create(
                run_root / "installation-key"
            )
        else:
            key_store = MemoryInstallationKeyStore()
        result = execute_workspace_first_integrated_run(
            workspace=args.workspace,
            conversation_repository=run_root / "conversations",
            key_store=key_store,
        )
        atomic_write_json(
            run_root / "integrated-run-receipt.json", result["receipt"]
        )
        atomic_write_json(
            run_root / "integrated-run-verification.json",
            result["verification"],
        )
        atomic_write_json(
            run_root / "run-summary.json",
            {
                "schema_version": "0.1.0-draft",
                "mode": "standard",
                "backend": "workspace_guarded",
                "security_claim": "logical_workspace_boundary_only",
                "conversation_id": result["namespace"].conversation_id,
                "archive_complete": result["archive"]["archive_complete"],
                "verification_valid": result["verification"]["valid"],
                "docker_invoked": False,
                "process_spawned": False,
                "model_invoked": False,
                "network_requested": False,
            },
        )
        print(run_root / "integrated-run-verification.json")
        return 0

    if not run_root.is_dir():
        raise FileNotFoundError(f"P4.5 run root does not exist: {run_root}")
    if os.name != "nt":
        raise RuntimeError(
            "persisted P4.5 verification currently requires Windows DPAPI"
        )
    key_store = WindowsDpapiInstallationKeyStore(
        run_root / "installation-key"
    )
    namespace = _single_namespace(run_root)
    receipt = load_json(run_root / "integrated-run-receipt.json")
    verification = verify_workspace_first_integrated_run(
        receipt,
        namespace=namespace,
        key_store=key_store,
        workspace=args.workspace,
    )
    output = run_root / "independent-verification.json"
    atomic_write_json(output, verification)
    print(output)
    return 0 if verification["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
