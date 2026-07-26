from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance import (  # noqa: E402
    run_deepseek_stream_observation_fixture,
    verify_deepseek_stream_observation_fixture,
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Project a DeepSeek-shaped stream fixture into public-output guard artifacts."
    )
    parser.add_argument("--fixture", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()

    if args.verify:
        receipt = verify_deepseek_stream_observation_fixture(
            output_root=args.output_root,
        )
    else:
        receipt = run_deepseek_stream_observation_fixture(
            fixture_path=args.fixture,
            output_root=args.output_root,
        )
    print(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
