#!/usr/bin/env python3
"""TB 2.1 round friction scan — turn a sweep's journals into an optimisation list.

The V4.1 round's primary purpose is **not** a score: it is to validate the orz
carrier end to end and to surface friction points that feed the next round of
optimisation.  This tool reads a finished (or partial) sweep and reports, per
trial and in aggregate, the signals that historically turned into work items:

  * tool failures / non-zero exits (``tool_completed.status == "error"``,
    ``exit_code != 0``), broken down by tool;
  * permission denials and policy denials (``permission_decision.decision``,
    ``tool_completed.policy_denial``);
  * transport retries (``transport_retry``: outcome/kind) and output-health
    guard trips (``output-health guard trip`` in the agent log);
  * long-session friction (``context_compressed`` / ``context_recovery_truncated``
    / ``session_archive``);
  * retrieval friction (``serp_budget_used`` vs ``serp_budget_cap``,
    ``browser_launch_result`` != success, retrieval call counts);
  * terminal states (``run_invalidated`` — including wall-clock kills);
  * 0z host-resource events (expected to be non-empty for the first time in the
    0.5.0 round).

Trial rows carry reward and wall-clock context, but ``attention_weight`` is a
**heuristic triage aid**, not a quality or score metric.

**Silent, non-blocking by contract.**  This is a side-channel analysis tool: it
reads whatever exists (a partial sweep is fine, mid-write journals are skipped
line by line), writes its report, and **always exits 0** — it never gates,
interrupts or slows the sweep, and it must not be wired into the run chain as a
blocking step.  ``--quiet`` writes the report without console output.

Read-only; writes only the JSON report at ``--out`` when given.
"""
from __future__ import annotations

import argparse
import json
from collections import Counter
from datetime import datetime
from pathlib import Path

SENTINEL_TOKEN = "output-health guard trip"
# The runtime reports a bogus tool name as
# ``tool execution failed: Tool not found: <name>`` — the marker is a substring,
# not a prefix.
TOOL_NOT_FOUND_MARKER = "Tool not found:"
# Framework signals / policy denials that arrive through the same
# ``status=error`` channel but are not tool failures.
FRAMEWORK_SIGNAL_MARKERS = ("refill_requested",)
ROLE_DENIAL_MARKERS = ("retrieval_role_write_denied", "policy_denial")
# Only these tools' exit_code is a process exit status; other tools use the
# field for validation/status codes and must not be read as shell failures.
SHELL_EXIT_TOOLS = ("run_terminal_cmd", "run_tests")
RESOURCE_EVENTS = (
    "host_resource_snapshot",
    "host_resource_denied",
    "resource_exhausted",
    "process_tree_reaped",
    "reclaim_performed",
    "resource_limit_hit",
)
LONGSESSION_EVENTS = ("context_compressed", "context_recovery_truncated", "session_archive")
TERMINAL_EVENTS = ("run_invalidated", "run_failed", "run_cancelled", "run_terminated")
RETRIEVAL_TOOLS = ("web_search", "web_fetch", "browser_read", "browser_control")


def parse_ts(value: str | None) -> datetime | None:
    if not value:
        return None
    try:
        return datetime.fromisoformat(str(value).replace("Z", "+00:00"))
    except Exception:  # noqa: BLE001
        return None


def scan_trial(trial_dir: Path) -> dict[str, object] | None:
    result_path = trial_dir / "result.json"
    if not result_path.is_file():
        return None
    try:
        result = json.loads(result_path.read_text(encoding="utf-8"))
    except Exception:  # noqa: BLE001
        return None
    task = (result.get("task_id") or {}).get("name")
    if not task:
        return None

    reward = None
    vr = result.get("verifier_result")
    if isinstance(vr, dict) and isinstance(vr.get("rewards"), dict):
        reward = vr["rewards"].get("reward")
    started, finished = parse_ts(result.get("started_at")), parse_ts(result.get("finished_at"))
    wall_minutes = (
        round((finished - started).total_seconds() / 60.0, 2)
        if started and finished else None
    )

    tool_calls: Counter = Counter()
    tool_errors: Counter = Counter()
    model_tool_name_errors: Counter = Counter()
    framework_signals: Counter = Counter()
    role_denials: Counter = Counter()
    nonzero_exit: Counter = Counter()
    non_shell_exit_codes: Counter = Counter()
    denials: Counter = Counter()
    decisions: Counter = Counter()
    policy_denials: Counter = Counter()
    transport: Counter = Counter()
    terminal: Counter = Counter()
    resource: Counter = Counter()
    longsession: Counter = Counter()
    browser_failures: Counter = Counter()
    serp_at_cap = 0
    timed_out = 0
    retrieval_requests = 0
    error_examples: dict[str, str] = {}

    for journal in trial_dir.glob("agent/gsa/runs/*/events.jsonl"):
        try:
            raw = journal.read_text(encoding="utf-8", errors="replace")
        except OSError:
            # The sweep may still be writing; a partial read is fine.
            continue
        for line in raw.splitlines():
            if not line.strip():
                continue
            try:
                event = json.loads(line)
            except Exception:  # noqa: BLE001
                continue
            etype = event.get("event_type") or "?"
            payload = event.get("payload") or {}
            if not isinstance(payload, dict):
                payload = {}
            if etype == "tool_completed":
                tool = str(payload.get("tool") or "?")
                tool_calls[tool] += 1
                error_text = str(payload.get("error") or "")
                if payload.get("status") == "error":
                    if TOOL_NOT_FOUND_MARKER in error_text:
                        # The model asked for a tool that does not exist; this is
                        # a model-side naming error, not a failure of the tool.
                        model_tool_name_errors[tool] += 1
                    elif any(m in error_text for m in FRAMEWORK_SIGNAL_MARKERS):
                        # e.g. plan_write returning ``refill_requested``: the
                        # framework asking for a refill, not a failure.
                        framework_signals[f"{tool}:{error_text}"] += 1
                    elif any(m in error_text for m in ROLE_DENIAL_MARKERS):
                        role_denials[f"{tool}:{error_text}"] += 1
                    else:
                        tool_errors[tool] += 1
                        error_examples.setdefault(f"tool:{tool}", error_text[:180])
                code = payload.get("exit_code")
                if isinstance(code, int) and code != 0:
                    if tool in SHELL_EXIT_TOOLS:
                        nonzero_exit[tool] += 1
                    else:
                        non_shell_exit_codes[tool] += 1
                if payload.get("timed_out"):
                    timed_out += 1
                if payload.get("policy_denial"):
                    policy_denials[str(payload["policy_denial"])] += 1
                used, cap = payload.get("serp_budget_used"), payload.get("serp_budget_cap")
                if isinstance(used, int) and isinstance(cap, int) and cap > 0 and used >= cap:
                    serp_at_cap += 1
                if tool in RETRIEVAL_TOOLS:
                    retrieval_requests += 1
            elif etype == "permission_decision":
                decision = str(payload.get("decision"))
                decisions[decision] += 1
                if decision == "deny":
                    denials[str(payload.get("tool") or "?")] += 1
            elif etype == "transport_retry":
                transport[f"{payload.get('outcome')}/{payload.get('kind')}"] += 1
            elif etype in TERMINAL_EVENTS:
                terminal[f"{etype}:{payload.get('status') or '-'}"] += 1
            elif etype in RESOURCE_EVENTS:
                resource[etype] += 1
            elif etype in LONGSESSION_EVENTS:
                longsession[etype] += 1
            elif etype == "browser_launch_result":
                outcome = str(payload.get("outcome") or payload.get("status") or "unknown")
                if outcome != "success":
                    browser_failures[outcome] += 1

    sentinel_trips = 0
    for log in trial_dir.glob("agent/orz.txt"):
        try:
            sentinel_trips += log.read_text(encoding="utf-8", errors="replace").lower().count(
                SENTINEL_TOKEN
            )
        except OSError:
            continue

    attention = (
        sum(tool_errors.values())
        + sum(model_tool_name_errors.values())
        + sum(denials.values())
        + sentinel_trips
        + sum(transport.values())
        + sum(policy_denials.values())
    )
    return {
        "task": task,
        "trial": trial_dir.name,
        "reward": reward,
        "wall_minutes": wall_minutes,
        "exception": bool(result.get("exception_info")) or (trial_dir / "exception.txt").is_file(),
        "attention_weight": attention,
        "tool_calls": dict(tool_calls),
        "tool_errors": dict(tool_errors),
        "model_tool_name_errors": dict(model_tool_name_errors),
        "framework_signals": dict(framework_signals),
        "role_denials": dict(role_denials),
        "nonzero_exit": dict(nonzero_exit),
        "non_shell_exit_codes": dict(non_shell_exit_codes),
        "permission_denials": dict(denials),
        "permission_decisions": dict(decisions),
        "policy_denials": dict(policy_denials),
        "transport_retry": dict(transport),
        "sentinel_trips": sentinel_trips,
        "terminal": dict(terminal),
        "resource_events": dict(resource),
        "longsession_events": dict(longsession),
        "browser_failures": dict(browser_failures),
        "retrieval_requests": retrieval_requests,
        "serp_at_cap": serp_at_cap,
        "timed_out": timed_out,
        "error_examples": error_examples,
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--jobs-root", type=Path, default=Path("D:/tb-eval/jobs-official"))
    ap.add_argument("--job-prefix", action="append", default=[],
                    help="only job dirs starting with this prefix (repeatable); default 'official-'")
    ap.add_argument("--top", type=int, default=20, help="how many friction trials to print")
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument("--scanned-at", default="2026-09-13")
    ap.add_argument("--quiet", action="store_true",
                    help="write the report without console output (silent scan)")
    args = ap.parse_args(argv)

    prefixes = args.job_prefix or ["official-"]
    trials = []
    for result in sorted(args.jobs_root.rglob("result.json")):
        rel = result.relative_to(args.jobs_root)
        if len(rel.parts) < 3:
            continue
        job = rel.parts[0] if rel.parts[0] != "_interrupted" else rel.parts[1]
        if not any(job.startswith(p) for p in prefixes):
            continue
        info = scan_trial(result.parent)
        if info:
            info["job"] = job
            trials.append(info)

    if not trials:
        # Advisory contract: no data is not an error (the sweep may not have
        # produced results yet), and nothing here may block the run chain.
        if not args.quiet:
            print(f"no trial results yet for prefixes {prefixes} under {args.jobs_root}")
            print("advisory scan only — exit 0, nothing to report")
        if args.out:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps({
                "schema": "tb21-friction-scan-v0.1",
                "scanned_at": args.scanned_at,
                "jobs_root": str(args.jobs_root),
                "job_prefixes": prefixes,
                "population": {"trials": 0, "tasks": 0},
                "signals": {},
                "friction_trials": [],
                "note": "no trial results matched; sweep not started or still running",
            }, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        return 0

    tools: Counter = Counter()
    tool_errors: Counter = Counter()
    model_name_errors: Counter = Counter()
    framework: Counter = Counter()
    role_denial: Counter = Counter()
    nonzero: Counter = Counter()
    non_shell_codes: Counter = Counter()
    denials: Counter = Counter()
    policy: Counter = Counter()
    transport: Counter = Counter()
    terminal: Counter = Counter()
    resource: Counter = Counter()
    longsession: Counter = Counter()
    browser: Counter = Counter()
    decisions: Counter = Counter()
    error_examples: dict[str, str] = {}
    for t in trials:
        tools.update(t["tool_calls"])
        tool_errors.update(t["tool_errors"])
        model_name_errors.update(t["model_tool_name_errors"])
        framework.update(t["framework_signals"])
        role_denial.update(t["role_denials"])
        nonzero.update(t["nonzero_exit"])
        non_shell_codes.update(t["non_shell_exit_codes"])
        denials.update(t["permission_denials"])
        policy.update(t["policy_denials"])
        transport.update(t["transport_retry"])
        terminal.update(t["terminal"])
        resource.update(t["resource_events"])
        longsession.update(t["longsession_events"])
        browser.update(t["browser_failures"])
        decisions.update(t["permission_decisions"])
        for k, v in t["error_examples"].items():
            error_examples.setdefault(k, v)

    def affected(key: str) -> int:
        return sum(1 for t in trials if t[key])

    report = {
        "schema": "tb21-friction-scan-v0.1",
        "scanned_at": args.scanned_at,
        "jobs_root": str(args.jobs_root),
        "job_prefixes": prefixes,
        "population": {
            "trials": len(trials),
            "tasks": len({t["task"] for t in trials}),
            "tool_calls": sum(tools.values()),
            "retrieval_requests": sum(int(t["retrieval_requests"]) for t in trials),
            "note": (
                "attention_weight is a heuristic ranking aid for triage, not a "
                "quality or score metric"
            ),
        },
        "signals": {
            "tool_calls_by_tool": dict(tools.most_common()),
            "tool_errors_by_tool": dict(tool_errors.most_common()),
            "model_tool_name_errors_by_requested_name": dict(model_name_errors.most_common()),
            "framework_signals": dict(framework.most_common()),
            "role_denials": dict(role_denial.most_common()),
            "nonzero_exit_by_tool": dict(nonzero.most_common()),
            "non_shell_exit_codes_by_tool": dict(non_shell_codes.most_common()),
            "permission_denials_by_tool": dict(denials.most_common()),
            "permission_decisions": dict(decisions.most_common()),
            "policy_denials": dict(policy.most_common()),
            "transport_retry": dict(transport.most_common()),
            "sentinel_trips": sum(int(t["sentinel_trips"]) for t in trials),
            "terminal_states": dict(terminal.most_common()),
            "resource_events": dict(resource.most_common()),
            "longsession_events": dict(longsession.most_common()),
            "browser_failures": dict(browser.most_common()),
            "serp_at_cap_trials": sum(1 for t in trials if int(t["serp_at_cap"]) > 0),
            "timed_out_tool_calls": sum(int(t["timed_out"]) for t in trials),
            "trials_with_exception": sum(1 for t in trials if t["exception"]),
            "trials_with_any_tool_error": affected("tool_errors"),
            "trials_with_model_tool_name_errors": affected("model_tool_name_errors"),
            "trials_with_any_denial": affected("permission_denials"),
            "trials_with_zero_reward": sum(
                1 for t in trials if (t["reward"] is not None and float(t["reward"]) < 1.0)
            ),
        },
        "error_examples": error_examples,
        "friction_trials": sorted(
            trials, key=lambda t: int(t["attention_weight"]), reverse=True
        ),
    }
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if args.quiet:
        return 0

    s = report["signals"]
    print(f"job prefixes        : {prefixes}")
    print(f"trials / tasks      : {report['population']['trials']} / {report['population']['tasks']}")
    print(f"tool calls          : {report['population']['tool_calls']}"
          f" (retrieval {report['population']['retrieval_requests']})")
    print(f"trials w/ tool error: {s['trials_with_any_tool_error']}"
          f"  trials w/ denial: {s['trials_with_any_denial']}"
          f"  trials w/ model-name error: {s['trials_with_model_tool_name_errors']}")
    print(f"tool failures       : {s['tool_errors_by_tool']}")
    print(f"model tool-name errs: {s['model_tool_name_errors_by_requested_name']}")
    print(f"framework signals   : {s['framework_signals']}")
    print(f"role/policy denials : {s['role_denials']}")
    print(f"nonzero exit (shell): {s['nonzero_exit_by_tool']}")
    print(f"other status codes  : {s['non_shell_exit_codes_by_tool']}")
    print(f"permission denials  : {s['permission_denials_by_tool']}")
    print(f"transport retries   : {s['transport_retry']}  sentinel trips: {s['sentinel_trips']}")
    print(f"terminal states     : {s['terminal_states']}")
    print(f"resource events     : {s['resource_events']}")
    print(f"longsession events  : {s['longsession_events']}  browser failures: {s['browser_failures']}")
    print(f"serp at cap trials  : {s['serp_at_cap_trials']}"
          f"  timed-out calls: {s['timed_out_tool_calls']}")
    if report["error_examples"]:
        print("\nerror text examples (triage):")
        for key, text in list(report["error_examples"].items())[:6]:
            print(f"  {key}: {text}")
    print("\ntop friction trials (attention weight):")
    for t in report["friction_trials"][: args.top]:
        print(f"  {t['attention_weight']:>3}  {str(t['task']):<32} reward={t['reward']} "
              f"err={t['tool_errors']} deny={t['permission_denials']} wall={t['wall_minutes']}m")
    if args.out:
        print(f"\nreport              : {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
