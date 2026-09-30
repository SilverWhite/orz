#!/usr/bin/env python3
"""TB 2.1 V4.1 下半场（0.8.7 代际）未跑 44 题逐题串行驱动器 — 2026-09-30。

口径来源：
  判定档 ``docs/TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md`` §10（用户裁决：
  b3-13 起未跑面**直接重跑、不续跑**——跨包体版本续跑不合适，
  ``run_official_v41_full.py`` 断点续跑形态**退役**〔留作历史不再用〕；
  未跑 44 题改以 0.8.7 代际**逐题独立作业**执行，作业名不带 rerun 语义）；
  上半场形态沿 ``run_official_v41_full.py``（单题一作业、串行、k=1、--upload --public、
  官方墙钟透传、失败重试一次、每题 rmi 镜像、简单起跑门）。

与退役驱动器的差异：
  * 独立轮次日志（``official-v41-second-half-round.log``），不写旧总账——
    下半场不是同一 run 的「续跑」（披露口径 §10：轮内代际分裂须明示）；
  * 作业名仍用官方冻结序名 ``official-v41-b<批>-<序>-<题名>``（未跑题非重跑，
    与上半场同系列；b3-13+ 与 B4/B5 各题在 0.8.4 时代从未起跑，无目录冲突）；
  * 其余纪律（冻结批序解析、job_done 骨架防御、残目录 park、延后清单、
    镜像 last-use 清理、deadline 起跑门、失败 90 s 后重试一次）逐条同退役驱动器。

代际身份由被调 runner（``run_r0_heavy_official.py``）硬门承载：载体 0.8.7
``3332b38f…`` ＋ 适配器 ``6d55c26e…``，不符即中止。
"""
from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

TB_EVAL = Path('D:/tb-eval')
RUNNER = Path('D:/CLI/scripts/run_r0_heavy_official.py')
FROZEN_SH = TB_EVAL / 'run_official_2.1.sh'
JOBS_DIR = TB_EVAL / 'jobs-official'
INCOMPLETE = JOBS_DIR / '_incomplete'
ROUND_LOG = JOBS_DIR / 'official-v41-second-half-round.log'

# 重跑系列（2026-09-29/30）权威作业目录：任务级口径＝重跑替换原试次，
# 主批序扫描时这 5 题的 done 判定以 rerun2 作业为准（原 bX 作业目录可能
# 只剩首半场中断骨架）。
RERUN_JOBS: dict[str, str] = {
    'build-pov-ray': 'official-v41-rerun2-build-pov-ray',
    'torch-pipeline-parallelism': 'official-v41-rerun2-torch-pipeline-parallelism',
    'extract-moves-from-video': 'official-v41-rerun2-extract-moves-from-video',
    'git-multibranch': 'official-v41-rerun2-git-multibranch',
    'winning-avg-corewars': 'official-v41-rerun2-winning-avg-corewars',
}
MANIFEST = (
    Path('D:/CLI/evaluation/corpus-freeze')
    / 'tb21-official-89-2026-09-13.manifest.json'
)
RETRY_DELAY_SECS = 90

_MANIFEST_CACHE: dict[str, dict[str, object]] | None = None


def log(msg: str) -> None:
    line = f'{time.strftime("%Y-%m-%d %H:%M:%S")} {msg}'
    ROUND_LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(ROUND_LOG, 'a', encoding='utf-8') as f:
        f.write(line + '\n')
    print(line, flush=True)


def frozen_batches() -> list[list[str]]:
    """从冻结 runner 原文解析 BATCH1..BATCH5（权威题序，不在本脚本复制题单）。"""
    text = FROZEN_SH.read_text(encoding='utf-8')
    batches: list[list[str]] = []
    for i in range(1, 6):
        m = re.search(rf'^BATCH{i}=\(([^)]*)\)', text, re.M)
        if not m:
            raise SystemExit(f'冻结 runner 中未找到 BATCH{i}')
        names = m.group(1).split()
        batches.append(names)
    total = sum(len(b) for b in batches)
    if total != 89:
        raise SystemExit(f'冻结批次题数合计 {total} != 89')
    return batches


def manifest_index() -> dict[str, dict[str, object]]:
    global _MANIFEST_CACHE
    if _MANIFEST_CACHE is None:
        data = json.loads(MANIFEST.read_text(encoding='utf-8'))
        idx: dict[str, dict[str, object]] = {}
        for rec in data.get('tasks', []):
            name = rec.get('task')
            if not name:
                continue
            meta = rec.get('metadata') or {}
            idx[str(name)] = {
                'image': str(meta.get('docker_image') or ''),
                'timeout': int(float(meta.get('agent_timeout_sec') or 0)),
            }
        _MANIFEST_CACHE = idx
    return _MANIFEST_CACHE


def _find_reward(obj) -> bool:
    """递归找任意非空 reward（trial 级 result 的 reward 在
    verifier_result.rewards.reward，2026-09-30 实测）。"""
    if isinstance(obj, dict):
        if obj.get('reward') is not None:
            return True
        return any(_find_reward(v) for v in obj.values())
    if isinstance(obj, list):
        return any(_find_reward(v) for v in obj)
    return False


def job_done(job_dir: Path) -> bool:
    """完成判定（2026-09-30 按 harbor 0.20.0 实测 schema 重写）。

    本版 harbor 的作业汇总（顶层 result.json）没有 ``trials``/``results``
    数组——完成标记是 ``stats.n_completed_trials``；试次级 result 的 reward
    在 ``verifier_result.rewards.reward``。kill/中断时落下的骨架
    （finished_at 有值、无完成计数、无 reward）**不得**判完成
    （b4-14 双杀与 b1-01 首半场骨架双实证；旧实现靠“试次级文件存在”
    旁路碰巧工作，对骨架误判为完成）。
    """
    if not job_dir.is_dir():
        return False
    for rp in job_dir.rglob('result.json'):
        try:
            data = json.loads(rp.read_text(encoding='utf-8'))
        except (OSError, ValueError):
            continue
        if not data.get('finished_at'):
            continue
        if data.get('trials') or data.get('results'):  # 旧版 schema 兼容
            return True
        # harbor 0.20：完成＝见到 reward 实体（verifier_result.rewards.reward）。
        # 顶层 stats.n_completed_trials 不可作准——kill 时 harbor 会把被杀
        # trial 记成 n_completed=1 的僵尸汇总（b4-14 实证：n_completed=1 但
        # verifier_result=None），无 reward 即无有效结果。
        if _find_reward(data):
            return True
    return False


def park_incomplete(job_dir: Path) -> None:
    """作业残目录（无 result.json）移入 _incomplete/，保留取证后重跑。"""
    if not job_dir.is_dir():
        return
    INCOMPLETE.mkdir(parents=True, exist_ok=True)
    dest = INCOMPLETE / f'{job_dir.name}-{time.strftime("%Y%m%d-%H%M%S")}'
    shutil.move(str(job_dir), str(dest))
    log(f'[park] 残作业目录移入 {dest}')


def run_one(task: str, job_name: str) -> int:
    argv = [
        sys.executable, str(RUNNER),
        '--tasks', task,
        '--job-name', job_name,
    ]
    log(f'[task] ==> {job_name} ({task})')
    t0 = time.time()
    proc = subprocess.run(argv, capture_output=True, text=True)
    tail = '\n'.join((proc.stdout or '').strip().splitlines()[-6:])
    log(f'[task] <-- {job_name} rc={proc.returncode} '
        f'elapsed={(time.time() - t0) / 60:.1f} min')
    if tail:
        for ln in tail.splitlines():
            log(f'[task]   | {ln}')
    if (proc.stderr or '').strip():
        for ln in proc.stderr.strip().splitlines()[-3:]:
            log(f'[task]   ! {ln}')
    return proc.returncode


def prune_image(image: str) -> None:
    """磁盘纪律：镜像引用不再被后续题使用时删除（cache 类可再生）。"""
    proc = subprocess.run(
        ['docker', 'rmi', '-f', image], capture_output=True, text=True
    )
    log(f'[rmi] rc={proc.returncode} {image}')


def fits_window(deadline: datetime | None) -> bool:
    """窗口门：截止时间未到才放行起跑新题（在跑题不受影响，自然跑完）。"""
    return deadline is None or datetime.now() < deadline


def defer_list() -> set[str]:
    """延后题清单（jobs-official/deferred-tasks.txt，一行一题，# 注释）。"""
    path = JOBS_DIR / 'deferred-tasks.txt'
    if not path.is_file():
        return set()
    items: set[str] = set()
    for ln in path.read_text(encoding='utf-8').splitlines():
        ln = ln.split('#', 1)[0].strip()
        if ln:
            items.add(ln)
    return items


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--dry-run', action='store_true', help='只打印计划')
    ap.add_argument('--deadline', default=None,
                    help='本地时间 YYYY-MM-DD HH:MM；简单起跑门：到点后不再起跑'
                         '新题（在跑题自然跑完），存档进度收尾')
    args = ap.parse_args(argv)

    deadline: datetime | None = None
    if args.deadline:
        deadline = datetime.strptime(args.deadline, '%Y-%m-%d %H:%M')

    batches = frozen_batches()
    plan: list[tuple[int, int, str, str]] = []
    for b, names in enumerate(batches, start=1):
        for i, task in enumerate(names, start=1):
            plan.append((b, i, task, f'official-v41-b{b}-{i:02d}-{task}'))

    index = manifest_index()
    # 镜像最后一次被使用的计划序号：跑完该题才允许删镜像（后续题可能共用同一引用）
    last_use: dict[str, int] = {}
    for idx, (_b, _i, task, _j) in enumerate(plan):
        last_use[str(index[task]['image'])] = idx

    if args.dry_run:
        todo = 0
        for b, i, task, job in plan:
            done = job_done(JOBS_DIR / job)
            if not done:
                todo += 1
                print(f'  b{b}-{i:02d} [todo] to={index[task]["timeout"]}s '
                      f'{task} -> {job}')
        print(f'  == todo {todo} / plan {len(plan)} ==')
        return 0

    log(f'== 下半场（0.8.7 代际）计划扫描 {len(plan)} 题（逐题一作业、串行、k=1、'
        f'--upload --public、deadline={args.deadline or "无"}）==')

    done_jobs: list[str] = []
    failed: list[tuple[str, str]] = []
    window_closed = False
    deferred_names = defer_list()
    if deferred_names:
        log(f'[defer-list] 轮内挂起 {len(deferred_names)} 题：{sorted(deferred_names)}')

    def execute(pass_tag: str) -> None:
        nonlocal window_closed
        ran = 0
        for idx, (b, i, task, job) in enumerate(plan):
            job_dir = JOBS_DIR / job
            if task in deferred_names:
                log(f'[defer] {job} 在延后清单，跳过（轮末/下窗重试）')
                continue
            if job_done(job_dir):
                continue
            auth = RERUN_JOBS.get(task)
            if auth and job_done(JOBS_DIR / auth):
                continue
            if not fits_window(deadline):
                log(f'[deadline] {args.deadline} 已到，停止起跑新题——'
                    f'存档当前进度（{job} 及其后延下窗）')
                window_closed = True
                return
            park_incomplete(job_dir)
            rc = run_one(task, job)
            ran += 1
            if rc != 0:
                log(f'[retry] {job} 90 s 后重试一次（rc={rc}）')
                time.sleep(RETRY_DELAY_SECS)
                park_incomplete(job_dir)
                rc = run_one(task, job)
            if job_done(job_dir):
                done_jobs.append(job)
                log(f'[done] {job}')
            else:
                failed.append((pass_tag, job))
                log(f'[FAIL] {job} 重试后仍无 result.json')
            if last_use[str(index[task]['image'])] == idx:
                prune_image(str(index[task]['image']))

    execute('pass1')
    if failed and not window_closed:
        log(f'== pass2：对 {len(failed)} 个失败题补跑一遍 ==')
        failed.clear()
        execute('pass2')

    log(f'== 结束：本窗完成 {len(done_jobs)} 题，失败 {len(failed)} 题'
        f'{"（窗口截止收尾）" if window_closed else ""} ==')
    for tag, job in failed:
        log(f'  FAILED {tag} {job}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
