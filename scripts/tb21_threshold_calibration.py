#!/usr/bin/env python3
"""P2-15 S2 threshold calibration dry-run over the TB 2.1 official ledger.

The TB 2.1 V4.1 generation round is judged by three通用统计 thresholds
(``docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md`` §3/§6.5):

    zero HTTP 400   |   output-health sentinel triggers <= 3 per run  |   cache hit rate >= 90%

This tool calibrates those thresholds against the *existing* official ledger
(``D:\\tb-eval\\jobs-official``) — a measurement of the thresholds' operating
point, **not** a score baseline: no reward comparison against V4 Flash is
produced, and reward values are only carried as context.

Measured per trial: reward, exception, model, wall-clock window.
Measured per run  (``agent/gsa/runs/<RUN>/events.jsonl``):
  * cache hit rate    = sum(cache_hit_tokens) / sum(cache_hit + cache_miss)
                        over ``model_output`` events — the provider-side
                        (journal 口径) hit rate used by prior S4 audits;
  * transport retries = ``transport_retry`` events by outcome/kind;
  * terminal state    = ``run_finished`` / ``run_failed`` / ``run_cancelled``
                        / ``run_invalidated`` / ``run_terminated``;
  * resource family   = the 0z host-resource events (snapshot/denied/
                        exhausted/process_tree_reaped/reclaim/limit_hit).

It additionally prints an *inventory* of sentinel / HTTP-400 marker families
found in the ledger (journal payloads and agent logs), so the calibration
record states which thresholds are measurable on which surface instead of
assuming a surface exists.

Read-only; the only write is the JSON report at ``--out``.
"""
from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from collections import Counter, defaultdict
from pathlib import Path

# One trip emits three log lines; the trip line is the countable marker:
#   `output-health guard trip detail=degeneration_detected:reasoning_stall: ...`
SENTINEL_TOKEN = "output-health guard trip"
SENTINEL_DETAIL_TOKEN = "degeneration_detected:"
SENTINEL_FAMILIES = ("reasoning_stall", "reasoning_repetition")
ZERO_CHUNK_TOKEN = "zero_chunk"

HTTP400_MARKERS = (
    "HTTP 400",
    "400 Bad Request",
    "status_code=400",
    '"status_code": 400',
    '"status_code":400',
    '"status": 400',
    '"status":400',
    "status=400",
    "status 400",
    "BadRequestError",
    'HTTP/1.1" 400',
)

RESOURCE_EVENTS = (
    "host_resource_snapshot",
    "host_resource_denied",
    "resource_exhausted",
    "process_tree_reaped",
    "reclaim_performed",
    "resource_limit_hit",
)

TERMINAL_EVENTS = (
    "run_finished",
    "run_failed",
    "run_cancelled",
    "run_invalidated",
    "run_terminated",
)


def percentile(values: list[float], q: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    pos = q * (len(ordered) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(ordered) - 1)
    frac = pos - lo
    return ordered[lo] * (1 - frac) + ordered[hi] * frac


def describe(values: list[float]) -> dict[str, object]:
    if not values:
        return {"n": 0}
    return {
        "n": len(values),
        "min": round(min(values), 6),
        "p10": round(percentile(values, 0.10), 6),
        "p50": round(percentile(values, 0.50), 6),
        "p90": round(percentile(values, 0.90), 6),
        "max": round(max(values), 6),
        "mean": round(statistics.fmean(values), 6),
    }


def is_official_population(job_name: str) -> bool:
    """Official TB 2.1 batches only (excludes smokes, dry-runs, TB 4.0 probe)."""
    return job_name.startswith("official-") and "tb40" not in job_name


def scan_trials(jobs_root: Path):
    trials = []
    for result_path in jobs_root.rglob("result.json"):
        rel = result_path.relative_to(jobs_root)
        parts = rel.parts
        if len(parts) < 3:
            # jobs-official/<job>/<trial>/result.json (interrupted batches add a level)
            continue
        trial_dir = result_path.parent
        job_name = rel.parts[0] if rel.parts[0] != "_interrupted" else rel.parts[1]
        try:
            data = json.loads(result_path.read_text(encoding="utf-8"))
        except Exception:  # noqa: BLE001
            continue
        reward = None
        vr = data.get("verifier_result") or {}
        if isinstance(vr, dict):
            rewards = vr.get("rewards") or {}
            if isinstance(rewards, dict):
                reward = rewards.get("reward")
        agent_cfg = (data.get("config") or {}).get("agent") or {}
        journals = sorted(trial_dir.glob("agent/gsa/runs/*/events.jsonl"))
        logs = sorted(trial_dir.glob("agent/orz.txt"))
        trial_logs = sorted(trial_dir.glob("trial.log"))
        trials.append(
            {
                "job": job_name,
                "trial": trial_dir.name,
                "task": (data.get("task_id") or {}).get("name"),
                "model": agent_cfg.get("model_name"),
                "reward": reward,
                "started_at": data.get("started_at"),
                "finished_at": data.get("finished_at"),
                "exception": bool(data.get("exception_info"))
                or (trial_dir / "exception.txt").is_file(),
                "journals": journals,
                "logs": logs,
                "trial_logs": trial_logs,
            }
        )
    return trials


def scan_journal(journal_path: Path):
    hit = miss = 0
    transport = Counter()
    terminal = Counter()
    resource = Counter()
    bad_request_events = 0
    for line in journal_path.read_text(encoding="utf-8", errors="replace").splitlines():
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except Exception:  # noqa: BLE001
            continue
        etype = event.get("event_type") or "?"
        payload = event.get("payload") or {}
        if etype == "model_output" and isinstance(payload, dict):
            hit += int(payload.get("cache_hit_tokens") or 0)
            miss += int(payload.get("cache_miss_tokens") or 0)
        elif etype == "transport_retry":
            transport[f"{payload.get('outcome')}/{payload.get('kind')}"] += 1
        elif etype in TERMINAL_EVENTS:
            terminal[etype] += 1
        elif etype in RESOURCE_EVENTS:
            resource[etype] += 1
        if isinstance(payload, dict) and payload:
            blob = json.dumps(payload, ensure_ascii=False)
            if any(m.lower() in blob.lower() for m in HTTP400_MARKERS):
                bad_request_events += 1
    total = hit + miss
    return {
        "journal": str(journal_path),
        "cache_hit_tokens": hit,
        "cache_miss_tokens": miss,
        "cache_hit_rate": round(hit / total, 6) if total else None,
        "transport_retry": dict(transport),
        "terminal": dict(terminal),
        "resource_events": dict(resource),
        "http400_marker_events": bad_request_events,
    }


def scan_text_surface(path: Path, markers) -> dict[str, int]:
    text = path.read_text(encoding="utf-8", errors="replace")
    low = text.lower()
    return {m: low.count(m.lower()) for m in markers if m.lower() in low}


def sentinel_trips(log_path: Path) -> dict[str, object]:
    text = log_path.read_text(encoding="utf-8", errors="replace")
    low = text.lower()
    count = low.count(SENTINEL_TOKEN)
    families = Counter(
        family
        for family in SENTINEL_FAMILIES
        if f"{SENTINEL_DETAIL_TOKEN}{family}".lower() in low
    )
    details = []
    start = 0
    while len(details) < 2:
        idx = low.find(SENTINEL_TOKEN, start)
        if idx < 0:
            break
        details.append(text[idx : idx + 170].replace("\n", " "))
        start = idx + 1
    return {
        "trips": count,
        "families": dict(families),
        "zero_chunk_markers": low.count(ZERO_CHUNK_TOKEN),
        "details": details,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--jobs-root", type=Path, default=Path("D:/tb-eval/jobs-official"))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--calibrated-at", default="2026-09-13")
    args = ap.parse_args(argv)

    trials = scan_trials(args.jobs_root)
    official = [t for t in trials if is_official_population(str(t["job"]))]

    trial_rows: list[dict[str, object]] = []
    journal_rows: list[dict[str, object]] = []
    log_surface_hits: Counter = Counter()
    trial_log_surface_hits: Counter = Counter()
    resource_totals: Counter = Counter()
    terminal_totals: Counter = Counter()

    for trial in official:
        run_infos = []
        hit = miss = 0
        for journal in trial["journals"]:
            info = scan_journal(journal)
            run_infos.append(info)
            journal_rows.append({**info, "job": trial["job"], "task": trial["task"]})
            hit += int(info["cache_hit_tokens"])
            miss += int(info["cache_miss_tokens"])
            resource_totals.update(info["resource_events"])
            terminal_totals.update(info["terminal"])

        trips = 0
        families: Counter = Counter()
        zero_chunk = 0
        details: list[str] = []
        for log in trial["logs"]:
            info = sentinel_trips(log)
            trips += int(info["trips"])
            families.update(info["families"])
            zero_chunk += int(info["zero_chunk_markers"])
            for detail in info["details"]:
                if len(details) < 2:
                    details.append(detail)
            log_surface_hits.update(scan_text_surface(log, HTTP400_MARKERS))
        for tlog in trial["trial_logs"]:
            trial_log_surface_hits.update(scan_text_surface(tlog, HTTP400_MARKERS))

        total = hit + miss
        trial_rows.append(
            {
                "job": trial["job"],
                "trial": trial["trial"],
                "task": trial["task"],
                "model": trial["model"],
                "reward": trial["reward"],
                "started_at": trial["started_at"],
                "exception": trial["exception"],
                "journals": len(run_infos),
                "cache_hit_tokens": hit,
                "cache_miss_tokens": miss,
                "cache_hit_rate": round(hit / total, 6) if total else None,
                "sentinel_trips": trips,
                "sentinel_families": dict(families),
                "sentinel_zero_chunk_markers": zero_chunk,
                "sentinel_details": details,
                "terminal": dict(
                    Counter(
                        k
                        for info in run_infos
                        for k in info["terminal"]
                        for _ in range(info["terminal"][k])
                    )
                ),
                "transport_retry": dict(
                    Counter(
                        k
                        for info in run_infos
                        for k in info["transport_retry"]
                        for _ in range(info["transport_retry"][k])
                    )
                ),
            }
        )

    rates = [r["cache_hit_rate"] for r in trial_rows if r["cache_hit_rate"] is not None]
    trip_counts = [int(r["sentinel_trips"]) for r in trial_rows]
    below = sorted(
        (r for r in trial_rows if r["cache_hit_rate"] is not None and r["cache_hit_rate"] < 0.90),
        key=lambda r: r["cache_hit_rate"],
    )
    invalidated = [r for r in trial_rows if r["terminal"].get("run_invalidated")]
    journal_400_events = sum(int(r["http400_marker_events"]) for r in journal_rows)

    report = {
        "schema": "tb21-threshold-calibration-v0.1",
        "calibrated_at": args.calibrated_at,
        "jobs_root": str(args.jobs_root),
        "population": {
            "trials_total": len(trials),
            "trials_official_tb21": len(trial_rows),
            "journals_scanned": len(journal_rows),
            "jobs_official": sorted({str(t["job"]) for t in official}),
            "trials_by_model": dict(Counter(str(t["model"]) for t in trial_rows)),
            "note": (
                "reward values are carried as context only; this report is a "
                "threshold calibration, not a score baseline and not a "
                "cross-generation (V4 / V4.1) comparison"
            ),
        },
        "thresholds": {
            "cache_hit_rate_ge_0.90": {
                "definition": (
                    "sum(cache_hit_tokens) / sum(cache_hit_tokens + cache_miss_tokens) "
                    "per trial, journal 口径 over model_output events"
                ),
                "surface": "agent/gsa/runs/<RUN>/events.jsonl",
                "distribution": describe(rates),
                "trials_meeting": sum(1 for r in rates if r >= 0.90),
                "trials_measured": len(rates),
                "pass_rate": (
                    round(sum(1 for r in rates if r >= 0.90) / len(rates), 4) if rates else None
                ),
                "below_threshold": [
                    {
                        "job": r["job"],
                        "task": r["task"],
                        "cache_hit_rate": r["cache_hit_rate"],
                        "cache_hit_tokens": r["cache_hit_tokens"],
                        "cache_miss_tokens": r["cache_miss_tokens"],
                    }
                    for r in below
                ],
            },
            "output_health_sentinel_le_3": {
                "definition": "output-health guard trips per trial (run)",
                "surface": (
                    "agent/orz.txt — `output-health guard trip ... "
                    "detail=degeneration_detected:`"
                ),
                "distribution": describe([float(t) for t in trip_counts]),
                "trials_meeting": sum(1 for t in trip_counts if t <= 3),
                "trials_measured": len(trip_counts),
                "max_observed": max(trip_counts) if trip_counts else None,
                "trips_total": sum(trip_counts),
                "families": dict(
                    Counter(k for r in trial_rows for k in r["sentinel_families"])
                ),
                "trip_details": [
                    d for r in trial_rows for d in r["sentinel_details"]
                ][:6],
                "boundary": (
                    "sentinel trips are observable only on the agent-log surface; "
                    "the run journal records no sentinel event (by design a "
                    "sentinel interrupt does not produce transport_retry)"
                ),
            },
            "zero_http_400": {
                "definition": "no HTTP 400 provider/transport failures",
                "surfaces_checked": {
                    "journal_payloads": {
                        "events_with_marker": journal_400_events,
                        "journals": len(journal_rows),
                    },
                    "agent_logs": dict(log_surface_hits),
                    "harness_trial_logs": dict(trial_log_surface_hits),
                    "markers": list(HTTP400_MARKERS),
                },
                "zero_confirmed_on_checked_surfaces": (
                    not log_surface_hits
                    and not trial_log_surface_hits
                    and journal_400_events == 0
                ),
                "boundary": (
                    "the marker set is the checked definition of '400'; a run "
                    "failing with a 400 in a form outside this set would not be "
                    "counted, so the round must record the marker set alongside "
                    "the result rather than reporting a bare zero"
                ),
            },
        },
        "context": {
            "terminal_states": dict(terminal_totals),
            "resource_events": dict(resource_totals),
            "trials_run_invalidated": [
                {"job": r["job"], "task": r["task"], "trial": r["trial"]} for r in invalidated
            ],
            "trials_with_exception": sum(1 for r in trial_rows if r["exception"]),
            "note": (
                "terminal_states / resource_events are carried as context for the "
                "round's telemetry criterion (schedule §6.4); they are not "
                "thresholded by this calibration"
            ),
        },
        "trials": sorted(
            (
                {
                    "job": r["job"],
                    "task": r["task"],
                    "trial": r["trial"],
                    "model": r["model"],
                    "cache_hit_rate": r["cache_hit_rate"],
                    "cache_hit_tokens": r["cache_hit_tokens"],
                    "cache_miss_tokens": r["cache_miss_tokens"],
                    "sentinel_trips": r["sentinel_trips"],
                    "sentinel_families": r["sentinel_families"],
                    "sentinel_details": r["sentinel_details"],
                    "terminal": r["terminal"],
                    "transport_retry": r["transport_retry"],
                }
                for r in trial_rows
            ),
            key=lambda r: (r["job"], r["task"] or ""),
        ),
        "journals": journal_rows,
    }

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    dist = report["thresholds"]["cache_hit_rate_ge_0.90"]
    sent = report["thresholds"]["output_health_sentinel_le_3"]
    p400 = report["thresholds"]["zero_http_400"]
    print(f"trials total / official TB2.1 : {len(trials)} / {len(trial_rows)}")
    print(f"journals scanned              : {len(journal_rows)}")
    print(f"models                        : {report['population']['trials_by_model']}")
    print(f"cache hit rate distribution   : {dist['distribution']}")
    print(f"trials meeting >= 90%         : {dist['trials_meeting']}/{dist['trials_measured']}")
    print(f"sentinel trips total / max    : {sent['trips_total']} / {sent['max_observed']}")
    print(f"trials meeting <= 3 trips     : {sent['trials_meeting']}/{sent['trials_measured']}")
    print(f"zero 400 on checked surfaces  : {p400['zero_confirmed_on_checked_surfaces']}")
    print(f"run_invalidated trials        : {len(invalidated)}")
    print(f"terminal states               : {report['context']['terminal_states']}")
    print(f"resource events               : {report['context']['resource_events']}")
    print(f"report                        : {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
