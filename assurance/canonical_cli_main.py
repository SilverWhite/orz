from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Sequence

from .canonical_cli import (
    run_canonical_guarded_cli,
    verify_canonical_guarded_cli_run,
)
from .errors import AssuranceError


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run or verify the canonical guarded CLI offline vertical slice."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    run_parser = subparsers.add_parser("run")
    run_parser.add_argument("--run-root", type=Path, required=True)
    run_parser.add_argument("--source-ledger", type=Path)
    run_parser.add_argument("--instruction-context", type=Path)
    run_parser.add_argument("--ask")
    run_parser.add_argument("--task", type=Path)
    run_parser.add_argument("--run-id", default="RUN-CANONICAL-CLI-FAKE-001")
    run_parser.add_argument("--task-id", default="TASK-CANONICAL-CLI-FAKE-001")
    run_parser.add_argument("--created-at", default=None)
    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--run-root", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        if args.command == "run":
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
        else:
            receipt = verify_canonical_guarded_cli_run(run_root=args.run_root)
    except AssuranceError as exc:
        print(
            json.dumps(
                {
                    "valid": False,
                    "error_type": "AssuranceError",
                    "error": str(exc),
                },
                ensure_ascii=False,
                sort_keys=True,
                allow_nan=False,
            ),
            file=sys.stderr,
        )
        return 2
    print(
        json.dumps(
            receipt,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
            allow_nan=False,
        )
    )
    return 0 if receipt["valid"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
