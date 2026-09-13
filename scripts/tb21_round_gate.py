#!/usr/bin/env python3
"""TB 2.1 round gate — score a sweep against the DeepSeek V4.1 bar.

The V4.1 generation round is a **k=1 official-basis sweep** of the 89 pinned
TB 2.1 tasks.  Its purpose is a screening decision, not a leaderboard entry:

    reach the official DeepSeek-V4.1-Flash figure  ->  worth a second round
    (k=5, variance-reducing)                       ->  then chase a formal score

Official bar (one-hand source, fetched 2026-09-13):
``https://api-docs.deepseek.com/updates/`` — Change Log, 2026-09-10 entry
"DeepSeek-V4.1-Flash Release": ``Terminal-Bench 2.1: 90.6``.
Caveat: that entry states no trial count or aggregation for the TB 2.1 figure
(the family's benchmark footnote in the 2026-08-21 entry describes "DeepSeek
Harness minimal mode ... max effort level"), so the bar is applied as a
screening reference for our whole stack (orz + routed V4.1 Flash), not as a
verified like-for-like leaderboard comparison.

This tool reads a finished sweep's ``result.json`` files and reports:

  * coverage (tasks seen vs the frozen 89),
  * per-task outcome (passed if any attempt reached reward 1.0),
  * accuracy (tasks passed / tasks seen) and the trial-weighted rate,
  * the gate verdict against ``--bar`` plus the failed-task list.

**Silent, non-blocking by contract.**  The bar is a *reference*, not a gate: the
tool is a side-channel read-out for triage, tolerates a partial sweep, **always
exits 0**, and must not be wired into the run chain as a blocking step.
``--quiet`` writes the report without console output.  Read-only; writes only
the JSON report at ``--out`` when given.
"""
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path

DEFAULT_BAR = 90.6
EXPECTED_TASKS = 89


def scan(jobs_root: Path, prefixes: list[str]) -> dict[str, dict[str, object]]:
    """Group trial results by task for job dirs matching *prefixes*."""
    tasks: dict[str, dict[str, object]] = defaultdict(
        lambda: {"trials": 0, "passes": 0, "rewards": [], "jobs": set()}
    )
    if not jobs_root.is_dir():
        return {}
    for result in sorted(jobs_root.rglob("result.json")):
        rel = result.relative_to(jobs_root)
        if len(rel.parts) < 3:
            continue
        job = rel.parts[0] if rel.parts[0] != "_interrupted" else rel.parts[1]
        if prefixes and not any(job.startswith(p) for p in prefixes):
            continue
        try:
            data = json.loads(result.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        # Job-level result.json files carry only trial counts, no task identity;
        # they must not be counted as tasks.
        task = (data.get("task_id") or {}).get("name")
        if not task:
            continue
        reward = None
        vr = data.get("verifier_result")
        if isinstance(vr, dict) and isinstance(vr.get("rewards"), dict):
            reward = vr["rewards"].get("reward")
        rec = tasks[task]
        rec["trials"] = int(rec["trials"]) + 1
        rec["jobs"].add(job)
        if reward is not None:
            rec["rewards"].append(float(reward))
            if float(reward) >= 1.0:
                rec["passes"] = int(rec["passes"]) + 1
    return tasks


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--jobs-root", type=Path, default=Path("D:/tb-eval/jobs-official"))
    ap.add_argument(
        "--job-prefix",
        action="append",
        default=[],
        help="only job dirs starting with this prefix (repeatable); default 'official-'",
    )
    ap.add_argument("--bar", type=float, default=DEFAULT_BAR,
                    help=f"official DeepSeek V4.1 TB 2.1 figure (default {DEFAULT_BAR})")
    ap.add_argument("--expected-tasks", type=int, default=EXPECTED_TASKS)
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument("--quiet", action="store_true",
                    help="write the report without console output (silent scan)")
    args = ap.parse_args(argv)

    prefixes = args.job_prefix or ["official-"]
    tasks = scan(args.jobs_root, prefixes)
    if not tasks:
        # Advisory contract: nothing to score yet is not an error.
        if not args.quiet:
            print(f"no trial results yet for prefixes {prefixes} under {args.jobs_root}")
            print("reference read-out only — exit 0, nothing to report")
        if args.out:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps({
                "schema": "tb21-round-gate-v0.1",
                "jobs_root": str(args.jobs_root),
                "job_prefixes": prefixes,
                "bar": args.bar,
                "coverage": {"tasks_seen": 0, "expected_tasks": args.expected_tasks,
                             "complete": False, "trials": 0},
                "note": "no trial results matched; sweep not started or still running",
            }, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        return 0

    passed = sorted(t for t, r in tasks.items() if int(r["passes"]) > 0)
    failed = sorted(t for t in tasks if t not in set(passed))
    trials = sum(int(r["trials"]) for r in tasks.values())
    reward_sum = sum(sum(r["rewards"]) for r in tasks.values())
    reward_n = sum(len(r["rewards"]) for r in tasks.values())

    accuracy = 100.0 * len(passed) / len(tasks)
    trial_rate = 100.0 * reward_sum / reward_n if reward_n else 0.0
    report = {
        "schema": "tb21-round-gate-v0.1",
        "jobs_root": str(args.jobs_root),
        "job_prefixes": prefixes,
        "bar": args.bar,
        "bar_source": {
            "url": "https://api-docs.deepseek.com/updates/",
            "entry": "2026-09-10 DeepSeek-V4.1-Flash Release",
            "text": "Terminal-Bench 2.1: 90.6",
            "note": (
                "vendor figure; trial count/aggregation not stated for TB 2.1 — "
                "used as a screening reference, not a like-for-like comparison"
            ),
        },
        "coverage": {
            "tasks_seen": len(tasks),
            "expected_tasks": args.expected_tasks,
            "complete": len(tasks) == args.expected_tasks,
            "trials": trials,
        },
        "accuracy_tasks": round(accuracy, 4),
        "reward_rate_trials": round(trial_rate, 4),
        "passed_tasks": passed,
        "failed_tasks": failed,
        "reference_line": {
            "reach_bar": accuracy >= args.bar,
            "margin_points": round(accuracy - args.bar, 4),
            "interpretation": (
                "advisory only, never blocking: >= bar means a second round "
                "(k=5, variance-reducing) is worth it before any formal score; "
                "< bar does not stop the round — the sweep's orz-validation and "
                "friction findings stand on their own"
            ),
        },
    }
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if args.quiet:
        return 0

    print(f"job prefixes          : {prefixes}")
    print(f"tasks seen / expected : {len(tasks)} / {args.expected_tasks}"
          f"{'' if len(tasks) == args.expected_tasks else '  <-- coverage gap'}")
    print(f"trials                : {trials}")
    print(f"tasks passed          : {len(passed)}  ({accuracy:.2f}%)")
    print(f"reward rate (trials)  : {trial_rate:.2f}%")
    print(f"reference line (V4.1) : {args.bar:.2f}%  -> "
          f"{'REACHED' if accuracy >= args.bar else 'NOT reached'} "
          f"(margin {accuracy - args.bar:+.2f} pt)")
    print("                        (advisory read-out; this tool never blocks the round)")
    print(f"failed tasks ({len(failed)}): {', '.join(failed) if failed else 'none'}")
    if args.out:
        print(f"report                : {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
