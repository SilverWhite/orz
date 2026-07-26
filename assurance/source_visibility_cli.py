from __future__ import annotations

import argparse
from pathlib import Path
import sys
from typing import Sequence

from .errors import AssuranceError
from .source_visibility import (
    dumps_receipt,
    evaluate_source_visibility_ledger_file,
    render_visibility_summary,
)


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Evaluate source full-text visibility gate for a source ledger."
    )
    parser.add_argument(
        "--ledger",
        type=Path,
        required=True,
        help="Path to source-visibility-ledger-v0.1 JSON.",
    )
    parser.add_argument(
        "--summary",
        action="store_true",
        help="Emit a compact text summary instead of JSON.",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        receipt = evaluate_source_visibility_ledger_file(args.ledger)
    except AssuranceError as exc:
        print(f"source visibility gate failed: {exc}", file=sys.stderr)
        return 2
    if args.summary:
        print(render_visibility_summary(receipt))
    else:
        print(dumps_receipt(receipt))
    return 0 if receipt["decision"] != "block" else 1


if __name__ == "__main__":
    raise SystemExit(main())
