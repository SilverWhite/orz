#!/usr/bin/env python3
"""墙钟暴露题重跑（2026-10-01 用户令：目标＝给 orz 一个公平评价）。

背景：本轮 0.8.4/0.8.7 官方跑批经 `--ak max_wallclock` 透传，而 orz 侧
TER T1.8（session 面墙钟行）与 0am S1 Part A（常驻 `[任务状态]` 的
「轮次预算」行）把它变成模型可见读数——偏离 Terminal Bench 惯例（agent
不见墙钟）。此外 `pgrep -af` / `ps` 还能从自己的启动命令行读到
`--max-wallclock`（留档实证 14 卷命中）。

条件（2026-10-01 用户令）：
  * **官方墙钟不去掉**——harbor 的 task 级官方超时仍为唯一外边界；
  * **去掉 agent 侧墙钟**——不带 `--ak max_wallclock` ⇒ orz 内部门不生效、
    session 面 `WALLCLOCK_LIMIT/REMAINING` 与常驻「轮次预算」行一并消失、
    启动命令行不再含 `--max-wallclock`（ps/pgrep 泄漏通道随之关闭）；
    `WALLCLOCK_ELAPSED` 行按载体现状仍渲染，披露如实注明此边界；
  * **官方级重跑**：作业前缀 `official-v41-rerun3-`、`--upload --public`、
    输出 `jobs-official/`——结果按任务级口径替换原试次。

题集＝固定 16 题（`FIXED_TASKS`，三通道扫留档＋权威试次过滤）＋ 136 批裁决
并入的 2 题（`EXTRA_TASKS`），共 18 题；**启动不再动态重扫**，只做一次
只读复核，发现计划外受影响题则记 WARN。

启动纪律：
  * **等当前官方跑批完全结束**；
  * **不与官方跑批抢资源**。
"""
from __future__ import annotations

import json
import re
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import run_official_v41_second_half as base  # noqa: E402  (复用 manifest/纪律)

HARBOR = r'D:\tb-eval\venv\Scripts\harbor.exe'
DATASET = ('terminal-bench/terminal-bench-2-1@'
           'sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a')
OUT_DIR = Path('D:/tb-eval/jobs-official')
LOG = OUT_DIR / 'official-v41-rerun3-round.log'

# ---- 固定题集（2026-10-01 用户令：固定下来，不再动态重扫） -----------------
#
# 来源＝对全轮 95 个作业卷的留档做三通道扫描，再按「权威试次」过滤：
#   A = blackboard_read section=session（墙钟面被渲染）
#   C = 模型正文自述时间压力（例："Time is tight (~8 min left)"）
#   P = 留档出现 `max-wallclock`（pgrep/ps 读到自己启动命令行；实证 14 卷）
# 权威试次口径＝0cc S4 重跑系列替换过原试次的 5 题取 rerun2 ⇒
# build-pov-ray / extract-moves-from-video 的权威试次干净，剔出本批。
# 留档只是**下界**：常驻「轮次预算」行不落 journal，无法从留档判定是否命中。
# 扫描件：D:/tb-eval/rerun3-wallclock-scope-2026-10-01.{json,txt}
FIXED_TASKS: list[str] = [
    'configure-git-webserver',    # A+C+P
    'count-dataset-tokens',       # A
    'mailman',                    # A+P
    'mteb-retrieve',              # A
    'protein-assembly',           # A
    'pytorch-model-recovery',     # A
    'gpt2-codegolf',              # C
    'headless-terminal',          # C+P
    'rstan-to-pystan',            # C+P
    'caffe-cifar-10',             # P
    'fix-ocaml-gc',               # P
    'git-multibranch',            # P（权威试次＝rerun2）
    'install-windows-3.11',       # P
    'kv-store-grpc',              # P
    'sqlite-with-gcov',           # P
    'torch-tensor-parallelism',   # P
]

# 136 批用户裁决：b1-08/b5-13（setup apt 404 作业级失败）并入本批同条件重跑
EXTRA_TASKS: list[str] = ['qemu-startup', 'qemu-alpine-ssh']

# 0cc S4 重跑系列已替换原试次的题 → 权威试次作业名（供留档复核用）
AUTHORITATIVE_JOB = {
    'build-pov-ray': 'official-v41-rerun2-build-pov-ray',
    'extract-moves-from-video': 'official-v41-rerun2-extract-moves-from-video',
    'git-multibranch': 'official-v41-rerun2-git-multibranch',
    'torch-pipeline-parallelism': 'official-v41-rerun2-torch-pipeline-parallelism',
    'winning-avg-corewars': 'official-v41-rerun2-winning-avg-corewars',
}

# C 通道判据（模型正文自述时间压力）；刻意不含裸「预算」以免命中检索额度
TIME_TEXT = re.compile(
    r'墙钟|时间预算|剩余时间|时间(?:紧|不够|有限|不多了)|没时间|来不及了|抓紧时间'
    r'|wall\s?clock|time is (?:tight|short|limited|running out)'
    r'|\d+\s*min(?:ute)?s?\s+(?:left|remaining)|budget is limited|speeding up',
    re.IGNORECASE,
)

# 泄漏标记：一旦出现在启动命令行里，pgrep/ps 就能读到
WALLCLOCK_LEAK_MARK = 'max-wallclock'


def log(msg: str) -> None:
    line = f'{datetime.now().strftime("%Y-%m-%d %H:%M:%S")} {msg}'
    print(line, flush=True)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    with LOG.open('a', encoding='utf-8') as f:
        f.write(line + '\n')


def _task_of(job: str) -> str:
    if job.startswith('official-v41-b'):
        return job[len('official-v41-'):].split('-', 2)[2]
    if 'rerun2-' in job:
        return job[len('official-v41-rerun2-'):]
    return job[len('official-v41-rerun-'):]


def exposed_jobs() -> set[str]:
    """只读重扫留档三通道（A/C/P），返回有墙钟暴露痕迹的作业名集合。"""
    exposed: set[str] = set()
    for vol in Path('D:/tb-eval/gsa-volumes').glob('official-v41-*'):
        if not vol.is_dir() or '-xp-' in vol.name:
            continue
        pull = texted = leaked = False
        for path in vol.rglob('*'):
            if not path.is_file():
                continue
            try:
                body = path.read_text(encoding='utf-8', errors='replace')
            except OSError:
                continue
            if WALLCLOCK_LEAK_MARK in body:
                leaked = True
            if path.name != 'events.jsonl':
                continue
            for line in body.splitlines():
                if '"event_type":"model_output"' not in line:
                    continue
                try:
                    ev = json.loads(line)
                except ValueError:
                    continue
                payload = ev.get('payload') or {}
                for call in payload.get('tool_calls') or []:
                    if not isinstance(call, dict):
                        continue
                    args = call.get('arguments') or {}
                    if isinstance(args, str):
                        try:
                            args = json.loads(args)
                        except ValueError:
                            args = {}
                    if (call.get('name') == 'blackboard_read'
                            and (args or {}).get('section') == 'session'):
                        pull = True
                text = payload.get('text') or ''
                if (text and '[SEMANTIC_SUMMARY]' not in text
                        and TIME_TEXT.search(text)):
                    texted = True
        if pull or texted or leaked:
            exposed.add(vol.name)
    return exposed


def audit_scope() -> set[str]:
    """留档复核：三通道暴露集 → 权威试次口径的任务集合。"""
    jobs = exposed_jobs()
    tasks: set[str] = set()
    for job in jobs:
        task = _task_of(job)
        if AUTHORITATIVE_JOB.get(task, job) in jobs:
            tasks.add(task)
    return tasks


def collect_rewards(obj, out: list) -> None:
    if isinstance(obj, dict):
        for k, v in obj.items():
            if k == 'reward' and v is not None:
                out.append(v)
            else:
                collect_rewards(v, out)
    elif isinstance(obj, list):
        for v in obj:
            collect_rewards(v, out)


def verdict(job: str, task: str, elapsed_min: float) -> None:
    job_dir = OUT_DIR / job
    rewards: list = []
    if job_dir.is_dir():
        for rp in job_dir.rglob('result.json'):
            try:
                d = json.loads(rp.read_text(encoding='utf-8'))
            except (OSError, ValueError):
                continue
            if d.get('finished_at'):
                collect_rewards(d, rewards)
    has_exception = bool(job_dir.is_dir() and list(job_dir.rglob('exception.txt')))
    invalidated = None
    for j in Path('D:/tb-eval/gsa-volumes').glob(f'{job}/*/runs/*/events.jsonl'):
        for line in open(j, encoding='utf-8', errors='replace'):
            if '"run_invalidated"' in line:
                try:
                    p = (json.loads(line).get('payload') or {})
                    invalidated = p.get('status') or p.get('reason') or 'invalidated'
                except ValueError:
                    invalidated = 'invalidated'
    if not rewards:
        mode = 'no-result'
    elif has_exception:
        mode = 'harbor-timeout(exception)'
    elif invalidated:
        mode = f'invalidated:{invalidated}'
    else:
        mode = 'self-completed'
    log(f'[rerun3-verdict] {task} reward={rewards[0] if rewards else None} '
        f'mode={mode} elapsed={elapsed_min:.1f} min')


def main() -> int:
    index = base.manifest_index()
    missing = [t for t in FIXED_TASKS if t not in index]
    if missing:
        log(f'== 固定题集有 {len(missing)} 题不在 manifest：{missing}——退出，不跑 ==')
        return 1
    tasks = list(FIXED_TASKS)
    for t in EXTRA_TASKS:
        if t in index and t not in tasks:
            tasks.append(t)
    log(f'== 墙钟重跑固定题集 {len(tasks)} 题（16 固定 + 2 qemu 由令并入）：{tasks} ==')
    log('== 条件：不带 --ak max_wallclock ⇒ session 面 WALLCLOCK_LIMIT/REMAINING '
        '与常驻「轮次预算」行均无上限读数，启动命令行不含 --max-wallclock'
        '（ps/pgrep 泄漏通道随之关闭）；harbor task 级官方超时仍是唯一外边界 ==')
    drift = sorted(audit_scope() - set(tasks))
    if drift:
        log(f'[WARN] 留档复核发现计划外受影响题（本次未纳入，需裁决）：{drift}')
    else:
        log('== 留档复核：三通道（A/C/P）受影响集与固定题集一致 ==')
    for task in tasks:
        job = f'official-v41-rerun3-{task}'
        image = index[task]['image']
        vol = f'D:/tb-eval/gsa-volumes/{job}'
        cmd = [
            HARBOR, 'run',
            '-d', DATASET,
            '-i', str(image),
            '-a', 'tb_agents.orz:Orz',
            '-m', 'deepseek-v4-flash',
            '--ak', 'orz_binary=D:/tb-eval/orz-linux/orz',
            '--ak', 'model_id=deepseek-v4-flash',
            '--ak', f'gsa_volume={vol}',
            '--mounts',
            json.dumps([{'type': 'bind', 'source': vol, 'target': '/orz-gsa'}]),
            '--env-file', 'D:/tb-eval/.env',
            '--job-name', job,
            '-o', str(OUT_DIR),
            '-k', '1', '-n', '1', '--upload', '--public', '-y',
            # 无 --ak max_wallclock：黑板 WALLCLOCK_LIMIT/REMAINING 显示消失
            # （对照条件）；官方期限由 harbor task 级超时保留执行。
            # --upload --public：官方级重跑，结果按任务级口径替换原试次。
        ]
        if any('wallclock' in str(a).lower() for a in cmd):
            log(f'[FATAL] {job} 的 argv 含 wallclock——会重开 ps/pgrep 泄漏，中止')
            return 1
        log(f'[rerun3] ==> {job}')
        t0 = time.time()
        proc = subprocess.run(cmd, capture_output=True, text=True)
        elapsed = (time.time() - t0) / 60.0
        log(f'[rerun3] <-- {job} rc={proc.returncode} elapsed={elapsed:.1f} min')
        verdict(job, task, elapsed)
    log('== rerun3 结束 ==')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
