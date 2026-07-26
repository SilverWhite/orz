from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from assurance import (
    run_deepseek_api_observation_pipeline,
    verify_deepseek_api_observation_pipeline,
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Project one DeepSeek API observation into public-output guard artifacts."
    )
    parser.add_argument("--api-result", required=True, type=Path)
    parser.add_argument("--output-root", required=True, type=Path)
    parser.add_argument("--task-id", default="TASK-DEEPSEEK-API-OBSERVATION-001")
    parser.add_argument("--task-contract-sha256", default="d" * 64)
    parser.add_argument("--step-index", default=1, type=int)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()

    if args.verify:
        receipt = verify_deepseek_api_observation_pipeline(
            output_root=args.output_root,
        )
    else:
        receipt = run_deepseek_api_observation_pipeline(
            api_result_path=args.api_result,
            output_root=args.output_root,
            task_id=args.task_id,
            task_contract_sha256=args.task_contract_sha256,
            step_index=args.step_index,
        )
    print(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
