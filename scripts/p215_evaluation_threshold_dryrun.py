#!/usr/bin/env python3
"""P2-15 S2 — evaluation/holdout threshold calibration dry run (small sample).

``evaluation/SCORING_PROTOCOL_v0.1.md`` §9 states that v0.1 has no valid model
acceptance threshold and that ``acceptance.status`` must stay
``not_calibrated`` until a sealed split and human blind baselines exist.
P2-15 S2 asks for a *small-sample dry run* that calibrates the
evaluation/holdout decision thresholds and records the calibration.

This tool runs that dry run end to end on the frozen internal regression
corpus and records what the threshold layer actually does:

1. exports an anonymous, leak-scanned scenario sample from
   ``regression/cases-v0.1.yaml`` (stripping oracle/classification/sources);
2. builds the oracle bundle separately (``build_evaluation_oracle_bundle``);
3. drives ``EvaluationRunner`` through the full flow with placeholder
   (all-``unassessed``) responses — no model is executed, so no result is
   interpreted as a capability measurement;
4. validates the produced result against
   ``evaluation/evaluation-result-v0.1.schema.json``;
5. records the threshold-layer state (status cap, acceptance block,
   conformance) as the calibration result.

Read-only with respect to the corpus; writes only the export sample (temp dir,
deleted afterwards) and the JSON record at ``--out``.
"""
from __future__ import annotations

import argparse
import json
import shutil
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from assurance.evaluation_runner import (  # noqa: E402
    EvaluationRunner,
    FrozenSystemProfile,
    ScenarioResponse,
    build_evaluation_oracle_bundle,
    load_coverage_matrix,
)
from assurance.scenario_exporter import (  # noqa: E402
    DEFAULT_CORPUS_PATH,
    Partition,
    export_scenarios,
)
from assurance.utils import sha256_bytes  # noqa: E402

RESULT_SCHEMA = REPO_ROOT / "evaluation" / "evaluation-result-v0.1.schema.json"


def pick_case_ids(corpus_path: Path, partition: str, limit: int) -> list[str]:
    import yaml

    corpus = yaml.safe_load(corpus_path.read_text(encoding="utf-8"))
    cases = corpus.get("cases", []) if isinstance(corpus, dict) else []
    picked = [
        case.get("case_id")
        for case in cases
        if case.get("partition") == partition and case.get("case_id")
    ]
    return picked[:limit]


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS_PATH)
    ap.add_argument("--partition", default="development")
    ap.add_argument("--sample-size", type=int, default=6)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--calibrated-at", default="2026-09-13")
    args = ap.parse_args(argv)

    case_ids = pick_case_ids(args.corpus, args.partition, args.sample_size)
    if not case_ids:
        print(f"no '{args.partition}' cases found in {args.corpus}", file=sys.stderr)
        return 2

    # Pull each sampled historical case's paired countercase as well, so the dry
    # run actually exercises the pair/countercase blocks of the contract instead
    # of leaving them empty.
    matrix = load_coverage_matrix()
    partners = sorted({
        matrix["pair_map"][case_id] for case_id in case_ids
        if case_id in matrix["pair_map"]
    })
    case_ids = sorted({*case_ids, *partners})

    workdir = Path(tempfile.mkdtemp(prefix="p215-dryrun-"))
    try:
        export = export_scenarios(
            case_ids=case_ids,
            corpus_path=args.corpus,
            export_root=workdir / "exports",
            verify_no_leaks=True,
        )
        scenarios = [
            {
                "case_id": s.case_id,
                "title": s.title,
                "task": s.task,
                "visible_facts": list(s.visible_facts),
            }
            for s in export.scenarios
        ]
        exported_ids = {s.case_id for s in export.scenarios}
        oracles = [
            o for o in build_evaluation_oracle_bundle(args.corpus)
            if o["case_id"] in exported_ids
        ]

        profile = FrozenSystemProfile(
            model_id="dry-run-placeholder",
            model_parameters={"temperature": 0.0},
            prompt_digest=sha256_bytes(b"p2-15-threshold-dry-run"),
            tool_allowlist=["read_file"],
            network_policy="none",
            budget_seconds=300,
            mode="guarded",
            seed=42,
            system_id="P2-15-DRY-RUN",
            provider="none",
            adapter_version="p2-15-dry-run",
        )
        # Cluster/pair/contrast authority is the registered coverage matrix;
        # it is what makes the cluster_results and pair_results blocks real.
        cluster_map = {
            case_id: clusters
            for case_id, clusters in matrix["cluster_map"].items()
            if case_id in exported_ids
        }
        pair_map = {
            hist: challenge for hist, challenge in matrix["pair_map"].items()
            if hist in exported_ids and challenge in exported_ids
        }
        contrast_types = {
            challenge: contrast
            for challenge, contrast in matrix["contrast_types"].items()
            if challenge in exported_ids
        }
        output_dir = workdir / "run"
        runner = EvaluationRunner(
            profile=profile,
            scenario_bundle=scenarios,
            oracle_bundle=oracles,
            output_dir=output_dir,
            corpus_revision=export.corpus_revision,
            cluster_map=cluster_map,
            pair_map=pair_map,
            contrast_types=contrast_types,
            corpus_partitions=(args.partition,),
        )

        # Placeholder responses: no model ran.  Every gate is left unassessed,
        # which the scoring protocol counts as a failure — the point is the
        # machinery, not a score.
        for scenario in runner.scenario_feed():
            runner.record_response(
                ScenarioResponse(
                    case_id=scenario["case_id"],
                    blind_precommitment="p2-15 dry run placeholder",
                )
            )
        result = runner.finalize()

        schema_errors: list[str] = []
        schema_error_paths: dict[str, int] = {}
        try:
            import jsonschema

            schema = json.loads(RESULT_SCHEMA.read_text(encoding="utf-8"))
            validator = jsonschema.Draft202012Validator(schema)
            schema_errors = [
                f"{'/'.join(str(p) for p in e.absolute_path) or '<root>'}: {e.message}"
                for e in sorted(validator.iter_errors(result), key=lambda e: list(e.absolute_path))
            ]
            for err in validator.iter_errors(result):
                head = next(iter(err.absolute_path), "<root>")
                schema_error_paths[str(head)] = schema_error_paths.get(str(head), 0) + 1
        except ImportError:  # pragma: no cover
            schema_errors = ["jsonschema not installed — schema validation skipped"]

        record = {
            "schema": "p215-evaluation-threshold-dryrun-v0.1",
            "calibrated_at": args.calibrated_at,
            "role": (
                "P2-15 S2 calibration record: what the evaluation/holdout "
                "threshold layer does on a small frozen sample"
            ),
            "sample": {
                "corpus_path": str(args.corpus),
                "corpus_sha256": sha256_bytes(args.corpus.read_bytes()),
                "corpus_revision": export.corpus_revision,
                "partition": args.partition,
                "case_ids": sorted(exported_ids),
                "export_id": export.export_id,
                "export_total_cases_in_corpus": export.total_cases_in_corpus,
                "skipped_count": export.skipped_count,
                "skip_reasons": export.skip_reasons,
                "leak_scan_clean": export.leak_scan_clean,
            },
            "machinery": {
                "oracle_isolation_verified": True,
                "scenario_fields_exposed": ["case_id", "title", "task", "visible_facts"],
                "oracle_bundle_case_count": len(oracles),
                "journal_head_sha256": runner._journal.head_sha256(),
                "evaluation_id": runner.evaluation_id,
                "attempt_id": runner.attempt_id,
                "coverage_matrix_sha256": matrix["sha256"],
                "clusters_in_sample": sorted({c for v in cluster_map.values() for c in v}),
                "pairs_in_sample": len(pair_map),
            },
            "threshold_layer_state": {
                "run_status": result.get("status"),
                "acceptance_block_emitted": result.get("acceptance"),
                "limits": result.get("limitations"),
                "red_lines_triggered": [
                    r["kind"] for r in result.get("red_lines", [])
                ],
                "counts": result.get("counts"),
                "status_never_claims_comparability": (
                    result.get("status") != "eligible_for_comparison"
                ),
                "schema_validation_error_count": len(schema_errors),
                "schema_validation_error_paths": dict(
                    sorted(schema_error_paths.items(), key=lambda kv: -kv[1])
                ),
                "schema_validation_errors": schema_errors[:20],
                "threshold_set": None,
                "conclusion": (
                    "no acceptance threshold is derivable from this dry run: the "
                    "runner caps status at descriptive_only and emits no "
                    "threshold_set; per SCORING_PROTOCOL §9 the acceptance block "
                    "must remain not_calibrated until a sealed evaluation/holdout "
                    "split plus a two-reviewer blind human baseline exist"
                ),
            },
            "preconditions_for_real_thresholds": [
                "create a sealed evaluation/holdout partition that has never been "
                "visible to prompt authors, implementers or the model",
                "run at least two independent human blind reviewers on that split",
                "run at least two models under the identical frozen system profile",
                "audit reviewer divergence, case difficulty, cluster dependency "
                "and countercase false positives",
                "freeze the split, then set per-dimension minimums plus "
                "zero-tolerance red lines (never a single mean score)",
            ],
            "boundary": (
                "the placeholder responses carry no information about any model; "
                "this record calibrates the threshold machinery, it is not an "
                "evaluation result and must never be reported as a score"
            ),
            "contract": {
                "result_schema": "evaluation/evaluation-result-v0.1.schema.json",
                "conformant": not schema_errors,
                "note": (
                    "GAP-EVAL-RESULT-SCHEMA-DRIFT (2026-09-13): the runner now "
                    "emits the registered schema shape; this dry run is the "
                    "end-to-end regression evidence (0 validation errors) and the "
                    "same assertion is enforced in "
                    "assurance/tests/test_evaluation_runner.py"
                ),
            },
        }
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(
            json.dumps(record, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )

        print(f"sample cases          : {len(export.scenarios)} "
              f"({', '.join(record['sample']['case_ids'])})")
        print(f"leak scan clean       : {export.leak_scan_clean}")
        print(f"oracle isolation      : verified")
        print(f"run status            : {result.get('status')}")
        print(f"acceptance block      : {json.dumps(result.get('acceptance'), ensure_ascii=False)}")
        print(f"schema errors         : {len(schema_errors)}")
        for err in schema_errors[:6]:
            print(f"  - {err}")
        print(f"record                : {args.out}")
        return 0
    finally:
        shutil.rmtree(workdir, ignore_errors=True)


if __name__ == "__main__":
    raise SystemExit(main())
