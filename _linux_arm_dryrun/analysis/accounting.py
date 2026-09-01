"""Linux ARM 干跑记账：policy blockage / adapted verifier / not_applicable。

输入：harbor job trials 目录（result.json + agent 事件链）。
输出：干跑记账汇总（JSON + 人类可读表格）。

用法:
    python accounting.py <trials_dir> [--out summary.json]
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path


# BoundaryBench 官方 not_applicable 名单（Linux 语义；high-nist 下必不适用）。
# 来源：github.com/boundary-bench/boundary-bench task_applicability.py。
NOT_APPLICABLE_TASKS = {
    "build-pmars",
    "build-pov-ray",
    "build-cython-ext",
    "kv-store-grpc",
    "nginx-request-logging",
    "mailman",
    "count-dataset-tokens",
}

# BoundaryBench 官方 adapted-verifier 名单（stock verifier 检查 root-owned 路径）。
ADAPTED_VERIFIER_TASKS = {
    "adaptive-rejection-sampler",
    "configure-git-webserver",
    "make-doom-for-mips",
    "mcmc-sampling-stan",
    "sqlite-with-gcov",
}


# OS 错误文本 -> 机械层通道映射（设计 §7 判据 1 的干跑口径）。
_ERR_PATTERNS = [
    (r"read-only file system|EROFS|Read-only file system", "err_eroFS"),
    (r"permission denied|EPERM|Operation not permitted|EACCES", "err_epErm"),
    (r"no space left|ENOSPC", "err_enoSpc"),
    (r"connection refused|ECONNREFUSED", "err_connRefused"),
    (r"connection timed out|ETIMEDOUT", "slow_timeout"),
    (r"network is unreachable|ENETUNREACH", "err_netUnreach"),
    (r"host is down|EHOSTDOWN|Name or service not known|NXDOMAIN", "err_dns"),
]


def _scan_os_errors(text: str) -> list[str]:
    hits = []
    for pattern, label in _ERR_PATTERNS:
        if re.search(pattern, text, re.IGNORECASE):
            hits.append(label)
    return hits


def classify_task(task_name: str) -> dict[str, bool]:
    """not_applicable / adapted-verifier 分类（按官方名单前缀匹配）。"""
    base = task_name.split("/")[-1].lower()
    for marker in (base, base.split("-arm")[0]):
        if marker in NOT_APPLICABLE_TASKS:
            return {"not_applicable": True, "adapted_verifier": False}
        if marker in ADAPTED_VERIFIER_TASKS:
            return {"not_applicable": False, "adapted_verifier": True}
    return {"not_applicable": False, "adapted_verifier": False}


def _trial_dir_events(trial: Path) -> str:
    """提取 trial 的事件链文本（agent/gsa/runs/*/events.jsonl 与 orz.txt）。"""
    parts = []
    for events in sorted((trial / "agent" / "gsa" / "runs").glob("RUN-*/events.jsonl")):
        try:
            for line in events.read_text(encoding="utf-8", errors="replace").splitlines():
                try:
                    ev = json.loads(line)
                except json.JSONDecodeError:
                    parts.append(line)
                    continue
                parts.append(json.dumps(ev.get("payload") or ev, ensure_ascii=False))
        except OSError:
            continue
    for log in (trial / "agent" / "orz.txt", trial / "trial.log"):
        if log.exists():
            try:
                parts.append(log.read_text(encoding="utf-8", errors="replace"))
            except OSError:
                continue
    return "\n".join(parts)


def _reward_of(trial: Path) -> float | None:
    reward = trial / "verifier" / "reward.txt"
    if reward.exists():
        try:
            return float(reward.read_text(encoding="utf-8").strip())
        except (OSError, ValueError):
            return None
    return None


def analyze_trial(trial: Path) -> dict:
    result = {}
    result_path = trial / "result.json"
    if result_path.exists():
        try:
            result = json.loads(result_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            result = {}
    task_name = result.get("task_name") or trial.name
    cls = classify_task(task_name)
    text = _trial_dir_events(trial)
    errs = _scan_os_errors(text)
    return {
        "trial": trial.name,
        "task_name": task_name,
        "arm": _arm_of(trial.name),
        "reward": _reward_of(trial),
        "exception": bool(result.get("exception")),
        **cls,
        "os_errors": errs,
        "os_error_channels": sorted({e.split("_", 1)[1] for e in errs}),
    }


def _arm_of(trial_name: str) -> str:
    low = trial_name.lower()
    if "high-nist" in low or "highnist" in low:
        return "high-nist"
    if "non-root" in low or "nonroot" in low:
        return "non-root"
    return "control"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("trials_dir", type=Path)
    parser.add_argument("--out", type=Path, default=None)
    args = parser.parse_args(argv)

    trials_dir = args.trials_dir
    trials = [p for p in trials_dir.iterdir() if p.is_dir() and (p / "result.json").exists()]
    if not trials:
        print(f"No trials found under {trials_dir}", file=sys.stderr)
        return 1

    rows = [analyze_trial(t) for t in sorted(trials)]
    summary = {
        "generated": "2026-09-01",
        "n_trials": len(rows),
        "trials": rows,
        "counts": {
            "reward_1_0": sum(1 for r in rows if r["reward"] == 1.0),
            "reward_0_0": sum(1 for r in rows if r["reward"] == 0.0),
            "exceptions": sum(1 for r in rows if r["exception"]),
            "not_applicable": sum(1 for r in rows if r["not_applicable"]),
            "adapted_verifier": sum(1 for r in rows if r["adapted_verifier"]),
        },
        "os_error_channel_totals": {},
    }
    for row in rows:
        for ch in row["os_error_channels"]:
            summary["os_error_channel_totals"][ch] = (
                summary["os_error_channel_totals"].get(ch, 0) + 1
            )

    if args.out:
        args.out.write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding="utf-8")
        print(f"wrote {args.out}")
    else:
        print(json.dumps(summary, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
