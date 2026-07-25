from __future__ import annotations

import argparse
import os
from pathlib import Path

from .keystore import (
    MemoryInstallationKeyStore,
    WindowsDpapiInstallationKeyStore,
)
from .readonly_projection import (
    project_complex_task_readonly,
    verify_complex_task_readonly_projection,
)
from .user_task_evaluation import (
    run_synthetic_user_task_evaluation,
    verify_synthetic_user_task_evaluation,
)
from .utils import atomic_write_json, load_json


def _parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="P5 synthetic task and read-only LIF projection preflight"
    )
    commands = parser.add_subparsers(dest="command", required=True)
    run = commands.add_parser(
        "run",
        help="run synthetic conformance and create a read-only LIF task snapshot",
    )
    run.add_argument("--run-root", type=Path, required=True)
    run.add_argument("--lif-source-root", type=Path, required=True)
    verify = commands.add_parser(
        "verify",
        help="independently verify a completed P5 preflight",
    )
    verify.add_argument("--run-root", type=Path, required=True)
    verify.add_argument("--lif-source-root", type=Path, required=True)
    return parser


def main() -> int:
    args = _parser().parse_args()
    run_root = args.run_root.resolve()
    if args.command == "run":
        if run_root.exists():
            raise FileExistsError(f"refusing to overwrite run root: {run_root}")
        run_root.mkdir(parents=True)
        if os.name == "nt":
            key_store = WindowsDpapiInstallationKeyStore.create(
                run_root / "installation-key"
            )
        else:
            key_store = MemoryInstallationKeyStore()
        synthetic = run_synthetic_user_task_evaluation(
            run_root=run_root / "synthetic",
            key_store=key_store,
        )
        projection = project_complex_task_readonly(
            source_root=args.lif_source_root,
            snapshot_root=run_root / "lif-readonly" / "snapshot",
            key_store=key_store,
        )
        atomic_write_json(
            run_root / "synthetic-evaluation-receipt.json",
            synthetic["receipt"],
        )
        atomic_write_json(
            run_root / "synthetic-evaluation-verification.json",
            synthetic["verification"],
        )
        atomic_write_json(
            run_root / "readonly-projection-receipt.json",
            projection["receipt"],
        )
        atomic_write_json(
            run_root / "readonly-projection-verification.json",
            projection["verification"],
        )
        summary = {
            "schema_version": "0.1.0-draft",
            "phase": "P5",
            "status": "mechanical_preflight_passed",
            "synthetic_task_count": synthetic["verification"][
                "planned_tasks"
            ],
            "synthetic_verified_passes": synthetic["verification"][
                "verified_passes"
            ],
            "lif_projected_file_count": projection["verification"][
                "projected_file_count"
            ],
            "lif_source_content_unchanged": projection["verification"][
                "source_content_unchanged"
            ],
            "external_participant_count": 0,
            "human_comprehension_assessed": False,
            "human_usability_assessed": False,
            "ordinary_user_safety_established": False,
            "production_readiness_established": False,
            "docker_invoked": False,
            "model_invoked": False,
            "network_requested": False,
            "child_process_spawned": False,
        }
        atomic_write_json(run_root / "p5-summary.json", summary)
        print(run_root / "p5-summary.json")
        return 0

    if not run_root.is_dir():
        raise FileNotFoundError(f"P5 run root does not exist: {run_root}")
    if os.name != "nt":
        raise RuntimeError(
            "persisted P5 verification currently requires Windows DPAPI"
        )
    key_store = WindowsDpapiInstallationKeyStore(
        run_root / "installation-key"
    )
    synthetic_receipt = load_json(
        run_root / "synthetic-evaluation-receipt.json"
    )
    projection_receipt = load_json(
        run_root / "readonly-projection-receipt.json"
    )
    synthetic_verification = verify_synthetic_user_task_evaluation(
        synthetic_receipt,
        run_root=run_root / "synthetic",
        key_store=key_store,
    )
    projection_verification = verify_complex_task_readonly_projection(
        projection_receipt,
        source_root=args.lif_source_root,
        snapshot_root=run_root / "lif-readonly" / "snapshot",
        key_store=key_store,
    )
    verification = {
        "valid": (
            synthetic_verification["valid"]
            and projection_verification["valid"]
        ),
        "synthetic": synthetic_verification,
        "readonly_projection": projection_verification,
    }
    output = run_root / "independent-verification.json"
    atomic_write_json(output, verification)
    print(output)
    return 0 if verification["valid"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
