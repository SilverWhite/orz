"""Sample 3 benchmark: sequential model calls vs one deterministic script.

Each corpus case runs twice against fresh file copies:

- baseline: the sequential model-call sequence (each call = 1 tool round);
- candidate: one ``workspace.run_script`` request carrying the same work
  as a linear script with ``$ref`` data references (1 tool round).

Metrics per arm: success rate, total and average tool rounds, and a failure
mode breakdown.  The pass criteria are reported but not enforced: candidate
success rate >= baseline success rate, candidate average rounds <= baseline
average rounds, and candidate total rounds < baseline total rounds.

The result JSON is written atomically and never overwrites an existing file
unless --overwrite is passed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import sys
import tempfile
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from actions import register_all
from service_registry import ConsoleError, ServiceRegistry
from trace import TraceStore


REQUIRED_CASE_KEYS = {
    "id",
    "description",
    "files",
    "script",
    "baseline_calls",
    "expect",
}
REQUIRED_STEP_KEYS = {"do"}
REQUIRED_CALL_KEYS = {"service", "data"}


def validate_corpus(corpus: dict[str, Any]) -> int:
    if not isinstance(corpus, dict) or not isinstance(corpus.get("cases"), list):
        raise ValueError("corpus must be an object with a cases list")
    cases = corpus["cases"]
    if not cases:
        raise ValueError("corpus cases must be non-empty")
    ids: set[str] = set()
    for case in cases:
        if not isinstance(case, dict):
            raise ValueError("each case must be an object")
        missing = REQUIRED_CASE_KEYS - set(case)
        if missing:
            raise ValueError(f"case missing keys: {sorted(missing)}")
        if case["id"] in ids:
            raise ValueError(f"duplicate case id: {case['id']}")
        ids.add(case["id"])
        for key in ("id", "description"):
            if not isinstance(case[key], str) or not case[key]:
                raise ValueError(f"case {case['id']}: {key} must be a non-empty string")
        if not isinstance(case["files"], dict) or not case["files"]:
            raise ValueError(f"case {case['id']}: files must be a non-empty object")
        for rel, content in case["files"].items():
            if not isinstance(rel, str) or not rel or not isinstance(content, str):
                raise ValueError(f"case {case['id']}: file entry must be path/content strings")
        encodings = case.get("encodings", {})
        if not isinstance(encodings, dict):
            raise ValueError(f"case {case['id']}: encodings must be an object")
        for rel, encoding in encodings.items():
            if rel not in case["files"]:
                raise ValueError(f"case {case['id']}: encoding for unknown file {rel}")
            if encoding not in ("utf-8", "gb18030"):
                raise ValueError(f"case {case['id']}: unsupported encoding {encoding}")
        if not isinstance(case["script"], list) or not case["script"]:
            raise ValueError(f"case {case['id']}: script must be non-empty")
        for step in case["script"]:
            if not isinstance(step, dict):
                raise ValueError(f"case {case['id']}: step must be an object")
            missing = REQUIRED_STEP_KEYS - set(step)
            if missing:
                raise ValueError(f"case {case['id']}: step missing keys: {sorted(missing)}")
            extra = set(step) - {"do", "with", "as"}
            if extra:
                raise ValueError(f"case {case['id']}: step unknown keys: {sorted(extra)}")
            if not isinstance(step["do"], str) or not step["do"]:
                raise ValueError(f"case {case['id']}: step do must be a non-empty string")
            if "with" in step and not isinstance(step["with"], dict):
                raise ValueError(f"case {case['id']}: step with must be an object")
            if "as" in step and (not isinstance(step["as"], str) or not step["as"]):
                raise ValueError(f"case {case['id']}: step as must be a non-empty string")
        if not isinstance(case["baseline_calls"], list) or not case["baseline_calls"]:
            raise ValueError(f"case {case['id']}: baseline_calls must be non-empty")
        for call in case["baseline_calls"]:
            if not isinstance(call, dict):
                raise ValueError(f"case {case['id']}: call must be an object")
            missing = REQUIRED_CALL_KEYS - set(call)
            if missing:
                raise ValueError(f"case {case['id']}: call missing keys: {sorted(missing)}")
            if not isinstance(call["service"], str) or not call["service"]:
                raise ValueError(f"case {case['id']}: call service must be a non-empty string")
            if not isinstance(call["data"], dict):
                raise ValueError(f"case {case['id']}: call data must be an object")
        if not isinstance(case["expect"], dict) or not case["expect"]:
            raise ValueError(f"case {case['id']}: expect must be a non-empty object")
    return len(cases)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def setup_files(case: dict[str, Any], root: Path) -> None:
    encodings = case.get("encodings", {})
    for rel, content in case["files"].items():
        target = (root / rel).resolve()
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content.encode(encodings.get(rel, "utf-8")))


def _get_path(obj: Any, path: str) -> Any:
    cur = obj
    for part in path.split("."):
        if isinstance(cur, dict):
            cur = cur[part]
        elif isinstance(cur, list) and part.isdigit():
            cur = cur[int(part)]
        else:
            raise KeyError(part)
    return cur


def check_expect(response: dict[str, Any], expect: dict[str, Any]) -> bool:
    for field_path, expected in expect.items():
        try:
            actual = _get_path(response, field_path)
        except (KeyError, IndexError, TypeError):
            return False
        if actual != expected:
            return False
    return True


def run_baseline(
    case: dict[str, Any], root: Path, registry: ServiceRegistry
) -> dict[str, Any]:
    case_root = root / case["id"]
    case_root.mkdir(parents=True, exist_ok=True)
    setup_files(case, case_root)
    attempts: list[dict[str, Any]] = []
    outcome_code: str | None = None
    final_response: dict[str, Any] | None = None
    for idx, call in enumerate(case["baseline_calls"], start=1):
        try:
            response = registry.call(call["service"], call["data"], str(case_root))
            code = None
        except ConsoleError as exc:
            code = exc.code
        if code is not None:
            outcome_code = code
            attempts.append(
                {
                    "round": idx,
                    "service": call["service"],
                    "success": False,
                    "code": code,
                }
            )
            break
        attempts.append(
            {
                "round": idx,
                "service": call["service"],
                "success": True,
                "code": None,
            }
        )
        final_response = response
    success = False
    if outcome_code is None and final_response is not None:
        if check_expect(final_response, case["expect"]):
            success = True
        else:
            outcome_code = "wrong_result"
    return {
        "success": success,
        "rounds": len(case["baseline_calls"]),
        "code": outcome_code,
        "attempts": attempts,
    }


def run_candidate(
    case: dict[str, Any], root: Path, registry: ServiceRegistry
) -> dict[str, Any]:
    case_root = root / case["id"]
    case_root.mkdir(parents=True, exist_ok=True)
    setup_files(case, case_root)
    try:
        response = registry.call(
            "workspace.run_script", {"script": case["script"]}, str(case_root)
        )
        code = None
    except ConsoleError as exc:
        code = exc.code
    success = False
    outcome_code = code
    if code is None:
        result = response.get("result")
        if isinstance(result, dict) and check_expect(result, case["expect"]):
            success = True
            outcome_code = None
        else:
            outcome_code = "wrong_result"
    return {
        "success": success,
        "rounds": 1,
        "code": outcome_code,
        "attempts": [{"round": 1, "success": success, "code": outcome_code}],
    }


def run_corpus(
    corpus: dict[str, Any],
    root: Path,
    registry: ServiceRegistry,
    case_ids: list[str] | None = None,
) -> dict[str, Any]:
    cases = corpus["cases"]
    if case_ids is not None:
        cases = [c for c in cases if c["id"] in case_ids]
    baseline_root = root / "baseline"
    candidate_root = root / "candidate"
    baseline_cases: list[dict[str, Any]] = []
    candidate_cases: list[dict[str, Any]] = []
    for case in cases:
        base = run_baseline(case, baseline_root, registry)
        cand = run_candidate(case, candidate_root, registry)
        baseline_cases.append({"id": case["id"], **base})
        candidate_cases.append({"id": case["id"], **cand})

    def aggregate(arm: list[dict[str, Any]]) -> dict[str, Any]:
        n = len(arm)
        successes = sum(1 for r in arm if r["success"])
        total_rounds = sum(r["rounds"] for r in arm)
        failures = Counter(r["code"] for r in arm if not r["success"])
        return {
            "cases": n,
            "successes": successes,
            "success_rate": round(successes / n, 4) if n else 0.0,
            "total_rounds": total_rounds,
            "avg_rounds": round(total_rounds / n, 4) if n else 0.0,
            "failure_breakdown": dict(sorted(failures.items())),
        }

    baseline_metrics = aggregate(baseline_cases)
    candidate_metrics = aggregate(candidate_cases)
    pass_criteria = {
        "candidate_success_rate_gte_baseline": (
            candidate_metrics["success_rate"] >= baseline_metrics["success_rate"] - 1e-9
        ),
        "candidate_avg_rounds_lte_baseline": (
            candidate_metrics["avg_rounds"] <= baseline_metrics["avg_rounds"] + 1e-9
        ),
        "candidate_total_rounds_lt_baseline": (
            candidate_metrics["total_rounds"] < baseline_metrics["total_rounds"]
        ),
    }
    return {
        "metrics": {"baseline": baseline_metrics, "candidate": candidate_metrics},
        "pass_criteria": pass_criteria,
        "cases": {"baseline": baseline_cases, "candidate": candidate_cases},
    }


def write_result_artifact(result: dict[str, Any], out: Path, overwrite: bool) -> None:
    if out.exists() and not overwrite:
        raise FileExistsError(f"output exists (use --overwrite): {out}")
    payload = json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False).encode("utf-8")
    fd, tmp_name = tempfile.mkstemp(prefix=".sample3_result_", suffix=".tmp", dir=str(out.parent))
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(payload)
            fh.flush()
            os.fsync(fh.fileno())
        os.replace(tmp_name, out)
    except OSError as exc:
        try:
            os.unlink(tmp_name)
        except OSError:
            pass
        raise RuntimeError(f"failed to write result: {exc}") from exc


def build_result(
    corpus_path: Path,
    corpus: dict[str, Any],
    run: dict[str, Any],
    args: argparse.Namespace,
) -> dict[str, Any]:
    started = run["started_at"]
    run_id = started.replace(":", "").replace("-", "").replace("+00:00", "Z")
    out_path = args.out.resolve()
    return {
        "schema_version": 1,
        "run": {
            "run_id": run_id,
            "script_path": str(Path(__file__).resolve()),
            "script_sha256": sha256_file(Path(__file__).resolve()),
            "corpus_sha256": sha256_file(corpus_path),
            "config_sha256": sha256_file(corpus_path),
            "corpus_path": str(corpus_path.resolve()),
            "input_files": [str(corpus_path.resolve())],
            "output_files": [str(out_path)],
            "output_dir": str(out_path.parent),
            "output_schema": "sample3_result/v1",
            "python_version": sys.version.split()[0],
            "argv": list(sys.argv),
            "smoke": bool(args.smoke),
            "started_at": run["started_at"],
            "finished_at": run["finished_at"],
            "parameter_provenance": {
                "corpus": "fixed/reused sample-3 corpus (8 cases)",
                "baseline_calls": "hand-authored simulated sequential model-call sequence (each call = 1 round; NOT real model output)",
                "candidate_script": "fixed deterministic script consumed by workspace.run_script (1 round)",
                "evidence_boundary": "baseline calls are hand-authored, not generated by a real model; fixed-corpus POC-level benefit evidence, not a production end-to-end conclusion",
                "extrapolated": "none",
                "status": "new_hypothesis",
            },
            "intentional_omissions": [
                "backend/seed/checkpoint/dataset flags: not applicable (deterministic POC, no model/backend, no RNG, dataset = fixed corpus)",
                "shape/slice checks: not applicable (no tensor/array inputs)",
                "all-zero/all-constant guard: zero success rate is a valid observed outcome, not a data fault",
            ],
        },
        "corpus": {"case_count": len(corpus["cases"]), "schema_version": corpus.get("schema_version", 1)},
        **run["body"],
    }


def validate_finite_metrics(metrics: dict[str, Any]) -> None:
    for arm_name, arm in metrics.items():
        for key, value in arm.items():
            if isinstance(value, float) and not math.isfinite(value):
                raise ValueError(f"non-finite metric: {arm_name}.{key}={value}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=(
            "Sample 3 mechanical script benchmark (sequential model calls vs "
            "one deterministic linear script)."
        )
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path(__file__).with_name("sample3_corpus.json"),
        help="corpus JSON path",
    )
    parser.add_argument("--out", type=Path, default=None, help="result JSON path")
    parser.add_argument("--overwrite", action="store_true", help="allow replacing an existing result file")
    parser.add_argument(
        "--smoke",
        action="store_true",
        help="run only the first two corpus cases (quick sanity path)",
    )
    args = parser.parse_args(argv)
    if args.out is None:
        args.out = Path("sample3_smoke_result.json" if args.smoke else "sample3_result.json")
    registry = ServiceRegistry()
    register_all(registry, TraceStore())
    try:
        corpus_path = args.corpus.resolve()
        corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
        validate_corpus(corpus)
        case_ids = [c["id"] for c in corpus["cases"][:2]] if args.smoke else None
        started_at = datetime.now(timezone.utc).isoformat()
        with tempfile.TemporaryDirectory(prefix="sample3_bench_") as tmp:
            body = run_corpus(corpus, Path(tmp), registry, case_ids)
        validate_finite_metrics(body["metrics"])
        finished_at = datetime.now(timezone.utc).isoformat()
        result = build_result(
            corpus_path,
            corpus,
            {"started_at": started_at, "finished_at": finished_at, "body": body},
            args,
        )
        out_path = args.out.resolve()
        out_path.parent.mkdir(parents=True, exist_ok=True)
        write_result_artifact(result, out_path, args.overwrite)
    except (ValueError, FileExistsError, json.JSONDecodeError, OSError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2
    except Exception as exc:  # fail-closed: report, do not promote
        print(f"INTERNAL ERROR: {exc}", file=sys.stderr)
        return 1

    base = result["metrics"]["baseline"]
    cand = result["metrics"]["candidate"]
    print(f"baseline : success={base['success_rate']:.2%} rounds_avg={base['avg_rounds']} total={base['total_rounds']}")
    print(f"candidate: success={cand['success_rate']:.2%} rounds_avg={cand['avg_rounds']} total={cand['total_rounds']}")
    print("pass_criteria:", json.dumps(result["pass_criteria"], ensure_ascii=False))
    print(f"result: {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
