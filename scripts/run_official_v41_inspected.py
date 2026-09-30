#!/usr/bin/env python3
"""单题跑＋单题即收结果（2026-09-30 用户令：不仅仅是单题跑，而是单题跑且单题收
结果，出现异常问题后能尽快发现）。

在 ``run_official_v41_second_half`` 的全部作业纪律之上（冻结批序、骨架 result
防御、延后清单、逐题 rmi、rc≠0 重试一次、起跑门），每题完成后**立即**读取：

  * ``result.json`` 的 reward（骨架防御同源：无 finished_at 不读）；
  * ``exception.txt``（harbor ``AgentTimeoutError`` 硬超时形态）；
  * journal ``run_invalidated``（载体侧优雅终态，如 wallclock）与模型轮数；

并即时打一行 ``[verdict]``；操作性异常（exception／invalidated／rc≠0／无
result）另打 ``[ALERT]`` 行——异常不再留到批末才被发现。

只做裁决与浮出，不中断批（跑分连续性优先；ALERT 供实时察看与事后追溯）。
"""
from __future__ import annotations

import argparse
import json
import time
from datetime import datetime
from pathlib import Path

import run_official_v41_second_half as base


def _collect_rewards(obj, out: list) -> None:
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k == 'reward' and v is not None:
                out.append(v)
            else:
                _collect_rewards(v, out)
    elif isinstance(obj, list):
        for v in obj:
            _collect_rewards(v, out)


def _rewards(rp: Path) -> list[float]:
    try:
        data = json.loads(rp.read_text(encoding='utf-8'))
    except (OSError, ValueError):
        return []
    if not data.get('finished_at'):
        return []
    out: list[float] = []
    _collect_rewards(data, out)
    return out


def inspect(job_name: str, task: str, rc: int, elapsed_min: float) -> str:
    """单题即收结果：reward＋死法形态，verdict／ALERT 行即时落日志。"""
    job_dir = base.JOBS_DIR / job_name
    rewards: list[float] = []
    for rp in job_dir.rglob('result.json'):
        rewards.extend(_rewards(rp))
    has_exception = any(job_dir.rglob('exception.txt'))

    invalidated: str | None = None
    rounds = 0
    vol = base.TB_EVAL / 'gsa-volumes' / job_name
    for j in vol.glob('*/runs/*/events.jsonl'):
        try:
            for line in j.open(encoding='utf-8', errors='replace'):
                if '"model_output"' in line:
                    rounds += 1
                if '"run_invalidated"' in line and invalidated is None:
                    try:
                        e = json.loads(line)
                        p = e.get('payload') or {}
                        invalidated = str(
                            p.get('status') or p.get('reason') or 'invalidated'
                        )
                    except ValueError:
                        invalidated = 'invalidated'
        except OSError:
            continue

    if not rewards:
        mode = 'no-result'
    elif has_exception:
        mode = 'harbor-timeout(exception)'
    elif invalidated:
        mode = f'invalidated:{invalidated}'
    else:
        mode = 'self-completed'
    reward = rewards[0] if rewards else None

    base.log(f'[verdict] {task} reward={reward} mode={mode} '
             f'rounds={rounds} elapsed={elapsed_min:.1f} min')
    alerts: list[str] = []
    if rc != 0:
        alerts.append(f'rc={rc}')
    if has_exception:
        alerts.append('harbor AgentTimeoutError（exception.txt）')
    if invalidated:
        alerts.append(f'run_invalidated={invalidated}')
    if not rewards:
        alerts.append('result.json 无可用 reward')
    for a in alerts:
        base.log(f'[ALERT] {job_name}: {a}')
    return mode


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--deadline', default=None,
                    help='本地时间 YYYY-MM-DD HH:MM；到点后不再起跑新题')
    args = ap.parse_args(argv)
    deadline = (datetime.strptime(args.deadline, '%Y-%m-%d %H:%M')
                if args.deadline else None)

    batches = base.frozen_batches()
    plan: list[tuple[int, str, str]] = []
    for b, names in enumerate(batches, start=1):
        for i, task in enumerate(names, start=1):
            plan.append((b, task, f'official-v41-b{b}-{i:02d}-{task}'))
    index = base.manifest_index()
    last_use: dict[str, int] = {}
    for idx, (_b, task, _j) in enumerate(plan):
        last_use[str(index[task]['image'])] = idx

    base.log(f'== 下半场·单题收果模式（deadline={args.deadline or "无"}；'
             f'每题完成后立即裁决 reward＋异常）==')

    done_jobs = 0
    deferred_names = base.defer_list()
    window_closed = False

    def execute(pass_tag: str) -> None:
        nonlocal done_jobs, window_closed
        for idx, (b, task, job) in enumerate(plan):
            job_dir = base.JOBS_DIR / job
            if task in deferred_names:
                continue
            if base.job_done(job_dir):
                continue
            auth = base.RERUN_JOBS.get(task)
            if auth and base.job_done(base.JOBS_DIR / auth):
                continue
            if not base.fits_window(deadline):
                base.log(f'[deadline] {args.deadline} 已到，停止起跑新题'
                         f'（{job} 及其后延下窗）')
                window_closed = True
                return
            base.park_incomplete(job_dir)
            t0 = time.time()
            rc = base.run_one(task, job)
            elapsed = (time.time() - t0) / 60.0
            mode = inspect(job, task, rc, elapsed)
            if rc != 0 or mode in ('no-result',):
                base.log(f'[retry] {job} 90 s 后重试一次（rc={rc} mode={mode}）')
                time.sleep(base.RETRY_DELAY_SECS)
                base.park_incomplete(job_dir)
                t0 = time.time()
                rc = base.run_one(task, job)
                elapsed = (time.time() - t0) / 60.0
                mode = inspect(job, task, rc, elapsed)
            if base.job_done(job_dir):
                done_jobs += 1
                base.log(f'[done] {job}')
            else:
                base.log(f'[FAIL] {job} 重试后仍无 result.json')
            if last_use[str(index[task]['image'])] == idx:
                base.prune_image(str(index[task]['image']))

    # 单题收果模式下失败题已在 pass1 内即时重试过一次；无 pass2。
    execute('pass1')
    base.log(f'== 结束：本窗完成 {done_jobs} 题'
             f'{"（窗口截止收尾）" if window_closed else ""} ==')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
