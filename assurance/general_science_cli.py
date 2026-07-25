from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Sequence

from .errors import AssuranceError
from .general_science_review import review_general_science_bundle


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Run the deterministic, read-only general-science bundle review."
        )
    )
    parser.add_argument(
        "--bundle-root",
        type=Path,
        required=True,
        help="Directory containing the review bundle and all referenced files.",
    )
    parser.add_argument(
        "--bundle-name",
        default="review-bundle.json",
        help="Safe relative bundle filename (default: review-bundle.json).",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _parser().parse_args(argv)
    try:
        result = review_general_science_bundle(
            bundle_root=args.bundle_root,
            bundle_name=args.bundle_name,
        )
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
            result,
            ensure_ascii=False,
            indent=2,
            sort_keys=True,
            allow_nan=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
