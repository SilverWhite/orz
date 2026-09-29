#!/usr/bin/env python3
"""TB 2.1 V4.1 整轮重跑（0.8.4 载体）逐题串行驱动器 — 2026-09-28 用户令。

口径来源（**不修改冻结 corpus runner** ``run_official_2.1.sh``）：
  ``docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md`` §3（官方口径 + Harbor 上传 +
  k=1 筛查轮）与 §3.2 追加裁决（C6 撤回：整轮重跑 89 题，第 0 轮账面只作摩擦证据）；
  2026-09-28 用户令：不开狗粮轮、直接进跑分、**每道题单跑（串行）**、
  尽量留出宿主机资源、结果上传 Harbor；同日二令：**只跑到 2026-09-29 09:00**
  （DeepSeek 计费高峰期不跑；不做硬截止准入——整题整题跑，到点不起跑新题、
  存档进度收尾）⇒ ``--deadline`` 简单起跑门；同日三令：**单跑单题＋主会话逐题
  监督**（``--stop-after 1`` 逐次放题，题间核对时间与跑分情况）。

形态：
  * 逐题一作业（``official-v41-b<批>-<序>-<题名>``），题序 = 冻结 runner 的
    BATCH1..BATCH5 官方划分（16/17/19/18/19 = 89）；
  * 每题经 ``run_r0_heavy_official.py --tasks <题> --job-name <作业>`` 执行：
    身份门（0.8.4 载体 + 现行适配器锁定值）→ 预拉镜像（官方清单 digest 落盘）→
    ``-k 1 -n 1 --upload --public -y`` → 每题官方 agent 超时经 ``--ak max_wallclock``
    透传（2026-09-02 口径收口：官方值 = 唯一评测墙钟）；
  * **窗口边界（2026-09-28 用户令，二次修订）**：``--deadline "YYYY-MM-DD HH:MM"``
    （本地时间）＝**简单起跑门，不做硬截止准入**——整题整题跑（在跑的题按官方墙钟
    自然跑完，不题中截杀）；到达截止时间后不再起跑新题，存档当前进度收尾；
    退出时输出完成 / 失败清单；
  * 断点续跑：作业目录已有 ``result.json`` 即跳过；有作业残目录而无 result 则先移入
    ``jobs-official/_incomplete/``（取证保留）再重跑；
  * 磁盘纪律：每题跑完 ``docker rmi`` 该题镜像（若后续题不共用同一镜像引用），
    峰值占用压到单题级；
  * 失败处置：单题失败重试一次（90 s 后，须仍过截止准入）；整轮跑完对失败清单
    自动补跑一遍（同样过准入）；仍失败者留轮次日志，不阻断后续题；
  * 静默旁路读数件（``tb21_round_gate.py`` / ``tb21_friction_scan.py``）不接入本链。

宿主机资源纪律：驱动器自身只做子进程等待；跑批窗口内不做构建/测试/其他重活。
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
ROUND_LOG = JOBS_DIR / 'official-v41-full-round.log'
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


def job_done(job_dir: Path) -> bool:
    """完成判定＝result.json 有 finished_at 且（trials 非空或为逐试次文件）。

    harbor 在作业**开跑时**即写骨架 result.json（finished_at=None、trials=[]），
    被杀作业会遗留该骨架 ⇒ 只查文件存在会把半程作业误判为已完成
    （2026-09-28 b1-02 实证，已修复）。
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
        if data.get('trials') or data.get('results'):
            return True
        if rp.parent != job_dir:  # 逐试次 result（作业目录的下层）
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
    """延后题清单（jobs-official/deferred-tasks.txt，一行一题，# 注释）。

    用途＝已定案挂起的题（如上游镜像腐烂待修）在轮内被稳定跳过，
    防止续跑逻辑反复选中重试；轮末重试时从清单移除即可。
    """
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
    ap.add_argument('--only', default=None,
                    help='只跑指定题（逗号分隔；补跑用，跳过批次序）')
    ap.add_argument('--stop-after', type=int, default=0,
                    help='跑完 N 题即停（0 = 不限）')
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

    if args.only:
        wanted = {t.strip() for t in args.only.split(',') if t.strip()}
        plan = [p for p in plan if p[2] in wanted]
        missing = wanted - {p[2] for p in plan}
        if missing:
            raise SystemExit(f'--only 题名不在冻结批次：{sorted(missing)}')

    index = manifest_index()
    # 镜像最后一次被使用的计划序号：跑完该题才允许删镜像（后续题可能共用同一引用）
    last_use: dict[str, int] = {}
    for idx, (_b, _i, task, _j) in enumerate(plan):
        last_use[str(index[task]['image'])] = idx

    if args.dry_run:
        for b, i, task, job in plan:
            done = 'DONE' if job_done(JOBS_DIR / job) else 'todo'
            print(f'  b{b}-{i:02d} [{done}] to={index[task]["timeout"]}s '
                  f'{task} -> {job}')
        return 0

    log(f'== 计划 {len(plan)} 题（逐题一作业、串行、k=1、--upload --public、'
        f'deadline={args.deadline or "无"}）==')

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
            if args.stop_after and ran >= args.stop_after:
                return
            job_dir = JOBS_DIR / job
            if task in deferred_names:
                log(f'[defer] {job} 在延后清单，跳过（轮末/下窗重试）')
                continue
            if job_done(job_dir):
                log(f'[skip] {job} 已有 result.json（断点续跑）')
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

    log(f'== 结束：完成 {len(done_jobs)} 题，失败 {len(failed)} 题'
        f'{"（窗口截止收尾）" if window_closed else ""} ==')
    for tag, job in failed:
        log(f'  FAILED {tag} {job}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
