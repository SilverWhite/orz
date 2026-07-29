"""Deprecated P2 CLI entry point.

This module is retained for backward compatibility only.
New development should use :mod:`~.canonical_cli` and the canonical
Gate sequence defined in :file:`CLI_PROJECT_INDEX.md` §B.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path

from .conversation import ConversationNamespace
from .guarded_execution import (
    ACTION_CAPABILITY,
    build_guarded_frozen_context,
    execute_guarded_no_model_action,
    verify_guarded_execution_receipt,
)
from .keystore import (
    MemoryInstallationKeyStore,
    WindowsDpapiInstallationKeyStore,
)
from .sandbox import (
    build_sandbox_selection_receipt,
    docker_candidate_from_observation,
    run_docker_sandbox_probe,
    windows_native_strict_candidate,
)
from .utils import atomic_write_json


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="P2 sandbox observation tools")
    commands = parser.add_subparsers(dest="command", required=True)
    observe = commands.add_parser(
        "docker-observe",
        help="run the pinned Docker sandbox probe and write an observation",
    )
    observe.add_argument("--workspace", type=Path, required=True)
    observe.add_argument("--output", type=Path, required=True)
    select = commands.add_parser(
        "select",
        help="build and verify a sandbox selection receipt from an observation",
    )
    select.add_argument("--observation", type=Path, required=True)
    select.add_argument("--requested-backend", required=True)
    select.add_argument("--conversation-id", required=True)
    select.add_argument("--output", type=Path, required=True)
    execute = commands.add_parser(
        "guarded-execute",
        help="run one fixed no-model action through P1/P2 gates",
    )
    execute.add_argument("--workspace", type=Path, required=True)
    execute.add_argument("--observation", type=Path, required=True)
    execute.add_argument("--run-root", type=Path, required=True)
    verify = commands.add_parser(
        "guarded-verify",
        help="independently verify a completed guarded execution run",
    )
    verify.add_argument("--workspace", type=Path, required=True)
    verify.add_argument("--observation", type=Path, required=True)
    verify.add_argument("--run-root", type=Path, required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    if args.command == "docker-observe":
        output = args.output.resolve()
        if output.exists():
            raise FileExistsError(f"refusing to overwrite observation: {output}")
        observation = run_docker_sandbox_probe(args.workspace)
        atomic_write_json(output, observation)
        print(output)
        return 0
    if args.command == "select":
        output = args.output.resolve()
        if output.exists():
            raise FileExistsError(f"refusing to overwrite receipt: {output}")
        observation = json.loads(args.observation.read_text(encoding="utf-8"))
        receipt = build_sandbox_selection_receipt(
            requested_backend=args.requested_backend,
            candidates=[
                docker_candidate_from_observation(observation),
                windows_native_strict_candidate(),
            ],
            conversation_id=args.conversation_id,
        )
        atomic_write_json(output, receipt)
        print(output)
        return 0
    if args.command == "guarded-verify":
        run_root = args.run_root.resolve(strict=True)
        if os.name != "nt":
            raise RuntimeError(
                "persisted guarded-run verification currently requires Windows DPAPI"
            )
        key_store = WindowsDpapiInstallationKeyStore(
            run_root / "installation-key"
        )
        conversation_roots = [
            path
            for path in (run_root / "conversations").iterdir()
            if path.is_dir()
        ]
        if len(conversation_roots) != 1:
            raise RuntimeError(
                "guarded run must contain exactly one conversation namespace"
            )
        namespace = ConversationNamespace(conversation_roots[0])
        observation = json.loads(args.observation.read_text(encoding="utf-8"))
        selection = json.loads(
            (run_root / "sandbox-selection-receipt.json").read_text(
                encoding="utf-8"
            )
        )
        receipt = json.loads(
            (run_root / "guarded-execution-receipt.json").read_text(
                encoding="utf-8"
            )
        )
        verification = verify_guarded_execution_receipt(
            receipt,
            namespace=namespace,
            key_store=key_store,
            workspace=args.workspace,
            selection_receipt=selection,
            observation=observation,
        )
        output = run_root / "guarded-execution-verification.json"
        atomic_write_json(output, verification)
        print(output)
        return 0 if verification["valid"] else 2
    if args.command == "guarded-execute":
        run_root = args.run_root.resolve()
        if run_root.exists():
            raise FileExistsError(f"refusing to overwrite run root: {run_root}")
        run_root.mkdir(parents=True)
        observation = json.loads(args.observation.read_text(encoding="utf-8"))
        if os.name == "nt":
            key_store = WindowsDpapiInstallationKeyStore.create(
                run_root / "installation-key"
            )
        else:
            key_store = MemoryInstallationKeyStore()
        frozen_context = build_guarded_frozen_context(
            workspace=args.workspace,
            observation=observation,
        )
        namespace = ConversationNamespace.create(
            run_root / "conversations",
            key_store=key_store,
            frozen_context=frozen_context,
            allowed_capabilities=[
                ACTION_CAPABILITY,
                "filesystem.workspace_read",
                "filesystem.workspace_write",
            ],
            denied_capabilities=[
                "host.mount",
                "model.invoke",
                "network.external",
                "secret.raw_read",
            ],
        )
        selection = build_sandbox_selection_receipt(
            requested_backend="auto",
            candidates=[
                docker_candidate_from_observation(observation),
                windows_native_strict_candidate(),
            ],
            conversation_id=namespace.conversation_id,
        )
        atomic_write_json(run_root / "sandbox-selection-receipt.json", selection)
        receipt = execute_guarded_no_model_action(
            namespace=namespace,
            key_store=key_store,
            workspace=args.workspace,
            selection_receipt=selection,
            observation=observation,
        )
        output = run_root / "guarded-execution-receipt.json"
        atomic_write_json(output, receipt)
        print(output)
        return 0
    raise AssertionError(f"unhandled command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
