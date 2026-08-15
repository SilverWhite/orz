"""Sample 2 benchmark: direct model hunk vs intent -> rule-layer hunk.

Each corpus case runs twice against fresh file copies:

- baseline: apply the ordered direct hunks (each attempt = 1 tool round)
  until success or the attempts are exhausted;
- candidate: parse the structured intent through the deterministic rule
  layer and apply the generated hunk (1 tool round).

Metrics per arm: success rate, total and average tool rounds, and a failure
mode breakdown. The pass criteria from the CLASSICAL-EXEC-ASSISTANT design
are reported but not enforced: candidate success rate >= baseline success
rate and candidate average rounds <= baseline average rounds.

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

import editor
from actions import decode_text
from service_registry import ConsoleError


REQUIRED_CASE_KEYS = {
    "id",
    "file",
    "content",
    "expected",
    "baseline_attempts",
    "intent",
}
REQUIRED_ATTEMPT_KEYS = {"old_string", "new_string"}


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
        for key in ("file", "content", "expected", "intent"):
            if not isinstance(case[key], str) or not case[key]:
                raise ValueError(f"case {case['id']}: {key} must be a non-empty string")
        if not isinstance(case["baseline_attempts"], list) or not case["baseline_attempts"]:
            raise ValueError(f"case {case['id']}: baseline_attempts must be non-empty")
        for attempt in case["baseline_attempts"]:
            if not isinstance(attempt, dict):
                raise ValueError(f"case {case['id']}: attempt must be an object")
            missing = REQUIRED_ATTEMPT_KEYS - set(attempt)
            if missing:
                raise ValueError(f"case {case['id']}: attempt missing keys: {sorted(missing)}")
            for key in REQUIRED_ATTEMPT_KEYS:
                if not isinstance(attempt[key], str):
                    raise ValueError(f"case {case['id']}: attempt {key} must be a string")
        encoding = case.get("content_encoding", "utf-8")
        if encoding not in ("utf-8", "gb18030"):
            raise ValueError(f"case {case['id']}: unsupported content_encoding {encoding}")
    return len(cases)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_case_file(root: Path, case: dict[str, Any]) -> Path:
    target = (root / case["file"]).resolve()
    target.parent.mkdir(parents=True, exist_ok=True)
    encoding = case.get("content_encoding", "utf-8")
    payload = case["content"].encode(encoding)
    target.write_bytes(payload)
    return target


def read_case_text(target: Path) -> str:
    return decode_text(target.read_bytes())[0]


def run_baseline(case: dict[str, Any], root: Path) -> dict[str, Any]:
    target = write_case_file(root, case)
    attempts: list[dict[str, Any]] = []
    rounds = len(case["baseline_attempts"])
    outcome_code: str | None = "exhausted"
    success = False
    for idx, attempt in enumerate(case["baseline_attempts"], start=1):
        data = dict(attempt)
        data["path"] = case["file"]
        try:
            editor.search_replace(data, str(root))
            code = None
        except ConsoleError as exc:
            code = exc.code
        if code is None:
            if read_case_text(target) == case["expected"]:
                success = True
                outcome_code = None
                rounds = idx
                attempts.append({"round": idx, "success": True, "code": None})
                break
            success = False
            outcome_code = "wrong_result"
            rounds = idx
            attempts.append({"round": idx, "success": False, "code": "wrong_result"})
            break
        attempts.append({"round": idx, "success": False, "code": code})
    return {
        "success": success,
        "rounds": rounds,
        "code": outcome_code,
        "attempts": attempts,
    }


def run_candidate(case: dict[str, Any], root: Path) -> dict[str, Any]:
    target = write_case_file(root, case)
    try:
        editor.replace_by_intent({"path": case["file"], "intent": case["intent"]}, str(root))
        code = None
    except ConsoleError as exc:
        code = exc.code
    success = False
    outcome_code = code
    if code is None:
        if read_case_text(target) == case["expected"]:
            success = True
            outcome_code = None
        else:
            success = False
            outcome_code = "wrong_result"
    return {
        "success": success,
        "rounds": 1,
        "code": outcome_code,
        "attempts": [{"round": 1, "success": success, "code": outcome_code}],
    }


def run_corpus(corpus: dict[str, Any], root: Path, case_ids: list[str] | None = None) -> dict[str, Any]:
    cases = corpus["cases"]
    if case_ids is not None:
        cases = [c for c in cases if c["id"] in case_ids]
    baseline_root = root / "baseline"
    candidate_root = root / "candidate"
    baseline_cases: list[dict[str, Any]] = []
    candidate_cases: list[dict[str, Any]] = []
    for case in cases:
        base = run_baseline(case, baseline_root)
        cand = run_candidate(case, candidate_root)
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
    fd, tmp_name = tempfile.mkstemp(prefix=".sample2_result_", suffix=".tmp", dir=str(out.parent))
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
            "output_schema": "sample2_result/v1",
            "python_version": sys.version.split()[0],
            "argv": list(sys.argv),
            "smoke": bool(args.smoke),
            "started_at": run["started_at"],
            "finished_at": run["finished_at"],
            "parameter_provenance": {
                "corpus": "fixed/reused sample-2 corpus (10 cases)",
                "baseline_attempts": "hand-authored simulated model-hunk sequence (NOT real model output)",
                "candidate_intent": "fixed structured intent consumed by rule layer",
                "evidence_boundary": "baseline attempts are hand-authored, not generated by a real model; fixed-corpus POC-level benefit evidence, not a production end-to-end conclusion",
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
        description="Sample 2 editor executor comparison benchmark (direct hunk vs intent -> rule-layer hunk)."
    )
    parser.add_argument(
        "--corpus",
        type=Path,
        default=Path(__file__).with_name("sample2_corpus.json"),
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
        args.out = Path("sample2_smoke_result.json" if args.smoke else "sample2_result.json")
    try:
        corpus_path = args.corpus.resolve()
        corpus = json.loads(corpus_path.read_text(encoding="utf-8"))
        validate_corpus(corpus)
        case_ids = [c["id"] for c in corpus["cases"][:2]] if args.smoke else None
        started_at = datetime.now(timezone.utc).isoformat()
        with tempfile.TemporaryDirectory(prefix="sample2_bench_") as tmp:
            body = run_corpus(corpus, Path(tmp), case_ids)
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
