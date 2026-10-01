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

驱动形态（2026-10-01 用户令「单题单题跑，每题结束后都收一次单题结果并进行检查」）：
  位置参数给一个/多个题名 ⇒ 只跑这些题（必须是冻结 18 题集的子集），每题跑完
  即落一条 `[rerun3-check]` 单题检查记录到轮次日志；缺省（不带参数）＝跑完整
  冻结题集，并在起跑前做一次留档三通道复核。
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
# 命令构造/身份门/预拉纪律一律复用官方起跑器（2026-10-01 修：先前手搓 argv 把
# `-i <镜像>` 当任务名筛选传进去，harbor 0.20 的 `-i`＝`--include-task-name`，
# 结果筛出零任务即刻 rc=1 退出——首题实证，改回单一真源）。
import run_r0_heavy_official as heavy  # noqa: E402

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

# 冻结题集（逐题模式只允许在其内取值，防计划外扩面）
FROZEN_TASKS: list[str] = FIXED_TASKS + EXTRA_TASKS

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

# 软件源修正挂载（2026-10-01 实证；**仅 qemu 两题**）：镜像 bullseye-security 的
# 索引与仓库池不一致 ⇒ 适配器 install() 装 curl/procps 404 ⇒ 作业级死；换镜像站
# 无效、换 snapshot 无效，绕开该 suite 可通。挂载=/etc/apt/sources.list，不动
# 镜像/适配器/身份门/数据集 pin。**偏差须在成绩披露写明**。
APT_FIX_TASKS: set[str] = {'qemu-startup', 'qemu-alpine-ssh'}
APT_FIX_SOURCES = Path('D:/CLI/scripts/aptfix-bullseye-sources.list')

# 单题检查面：上限读数一旦可见即失败；载体现状残留面（elapsed / limit none）
# 只计数、不作失败判据（披露边界见 0cg 与轮次日志起跑行）。
LIMIT_NUMERIC = re.compile(r'WALLCLOCK_LIMIT:\s*\d+s')
LIMIT_NONE = re.compile(r'WALLCLOCK_LIMIT:\s*none')


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


def verdict(job: str, task: str, elapsed_min: float) -> None:
    """单题判分读数：**逐试次** result.json 的 verifier_result.rewards.reward
    （作业级 result.json 里 `stats` 的 reward 直方图不可当作分数——2026-10-01
    首两题实证：作业级会给出 {'0.0': [trial]; '1.0': [trial]} 这类分布）。"""
    job_dir = OUT_DIR / job
    rewards: list = []
    if job_dir.is_dir():
        for rp in job_dir.rglob('result.json'):
            if rp.parent == job_dir:
                continue
            try:
                d = json.loads(rp.read_text(encoding='utf-8'))
            except (OSError, ValueError):
                continue
            vr = d.get('verifier_result')
            if isinstance(vr, dict) and isinstance(vr.get('rewards'), dict):
                r = vr['rewards'].get('reward')
            else:
                r = d.get('reward')
            if r is not None:
                rewards.append(float(r))
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


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding='utf-8', errors='replace')
    except OSError:
        return ''


def inspect_volume(job: str, task: str) -> bool:
    """单题结果检查（每题跑完即收一次；返回是否通过硬判据）。"""
    vol_root = Path('D:/tb-eval/gsa-volumes') / job
    if not vol_root.is_dir():
        log(f'[rerun3-check] {task} FAIL 卷目录缺失：{vol_root}')
        return False
    leak_files: list[str] = []
    limit_numeric = limit_none = remaining = rounds = elapsed = 0
    session_pulls = 0
    text_hits: list[str] = []
    file_count = 0
    for path in vol_root.rglob('*'):
        if not path.is_file():
            continue
        file_count += 1
        body = _read_text(path)
        if WALLCLOCK_LEAK_MARK in body:
            leak_files.append(path.name)
        # 时间面标记：扫**全部留档文件**（工具返回内容不落 journal —— 2026-10-01
        # 实证 blackboard_read 的 tool_completed 只记 section 不记正文，故只在
        # journal 文本里计数会漏；全库扫描＝「有没有任何留档把时间面带出来」）。
        limit_numeric += len(LIMIT_NUMERIC.findall(body))
        limit_none += len(LIMIT_NONE.findall(body))
        remaining += body.count('WALLCLOCK_REMAINING:')
        rounds += body.count('WALLCLOCK_REMAINING_ROUNDS')
        elapsed += body.count('WALLCLOCK_ELAPSED:')
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
                    session_pulls += 1
            text = payload.get('text') or ''
            if text and '[SEMANTIC_SUMMARY]' not in text:
                m = TIME_TEXT.search(text)
                if m:
                    text_hits.append(m.group(0))
    ok = not leak_files and not limit_numeric and not remaining and not rounds
    log(f'[rerun3-check] {task} {"PASS" if ok else "FAIL"} '
        f'files={file_count} 泄漏文件={len(leak_files)}{leak_files[:3]} '
        f'上限读数={limit_numeric} 剩余读数={remaining} 轮次换算行={rounds} '
        f'| 残留（披露边界，非失败）：limit-none={limit_none} '
        f'elapsed行={elapsed} session面读取={session_pulls} '
        f'| 模型时间自述={len(text_hits)}{sorted(set(text_hits))[:3]}')
    return ok


def snapshot(tag: str) -> None:
    """起跑资源快照（宿主视角；读数口径同官方起跑器，落本批轮次日志）。"""
    try:
        usage = shutil.disk_usage('D:/')
        disk = (f'D: free {usage.free / 2**30:.2f} GiB / '
                f'total {usage.total / 2**30:.2f} GiB')
    except OSError as exc:
        disk = f'D: free <unavailable: {exc}>'
    counts: dict[str, str] = {}
    for key, argv in (
        ('images', ['docker', 'images', '-q']),
        ('containers', ['docker', 'ps', '-aq']),
    ):
        try:
            proc = subprocess.run(argv, capture_output=True, text=True, timeout=60)
            items = [ln for ln in proc.stdout.splitlines() if ln.strip()]
            counts[key] = (str(len(items)) if proc.returncode == 0
                           else f'<rc={proc.returncode}>')
        except (OSError, subprocess.SubprocessError) as exc:
            counts[key] = f'<unavailable: {exc}>'
    log(f'[snapshot:{tag}] {disk} | docker images={counts["images"]} '
        f'containers={counts["containers"]}')


def main(argv: list[str] | None = None) -> int:
    raw = list(sys.argv[1:] if argv is None else argv)
    job_suffix = ''
    if '--job-suffix' in raw:  # 同名作业目录不覆盖：给作业名加后缀（显式、入日志）
        i = raw.index('--job-suffix')
        job_suffix = raw[i + 1] if i + 1 < len(raw) else ''
        del raw[i:i + 2]
    requested = [a for a in raw if not a.startswith('-')]
    index = heavy.manifest_index()
    if requested:
        unknown = [t for t in requested if t not in index]
        nonfrozen = [t for t in requested if t not in FROZEN_TASKS]
        if unknown or nonfrozen:
            log(f'== 指定题非法（不在 manifest：{unknown}；不在冻结 '
                f'{len(FROZEN_TASKS)} 题集：{nonfrozen}）——退出，不跑 ==')
            return 1
        tasks = list(dict.fromkeys(requested))
        log(f'== 逐题模式：本次仅跑 {len(tasks)} 题 {tasks}'
            f'（冻结题集共 {len(FROZEN_TASKS)} 题） ==')
    else:
        missing = [t for t in FIXED_TASKS if t not in index]
        if missing:
            log(f'== 固定题集有 {len(missing)} 题不在 manifest：'
                f'{missing}——退出，不跑 ==')
            return 1
        tasks = list(FIXED_TASKS)
        for t in EXTRA_TASKS:
            if t in index and t not in tasks:
                tasks.append(t)
        log(f'== 墙钟重跑固定题集 {len(tasks)} 题'
            f'（16 固定 + 2 qemu 由令并入）：{tasks} ==')
        drift = sorted(audit_scope() - set(tasks))
        if drift:
            log(f'[WARN] 留档复核发现计划外受影响题（本次未纳入，需裁决）：{drift}')
        else:
            log('== 留档复核：三通道（A/C/P）受影响集与固定题集一致 ==')
    log('== 条件：不带 --ak max_wallclock ⇒ session 面 WALLCLOCK_LIMIT/REMAINING '
        '与常驻「轮次预算」行均无上限读数，启动命令行不含 --max-wallclock'
        '（ps/pgrep 泄漏通道随之关闭）；harbor task 级官方超时仍是唯一外边界 ==')
    # 代际身份门（排期 §2 代际纪律；同官方起跑器锁定值）——不符即中止
    carrier = heavy.sha256_file(heavy.ORZ)
    adapter = heavy.sha256_file(heavy.ADAPTER)
    if (carrier != heavy.EXPECTED_CARRIER_SHA256
            or adapter != heavy.EXPECTED_ADAPTER_SHA256):
        log('FAIL 代际身份不符：carrier '
            f'{"OK" if carrier == heavy.EXPECTED_CARRIER_SHA256 else "DRIFT"} / '
            'adapter '
            f'{"OK" if adapter == heavy.EXPECTED_ADAPTER_SHA256 else "DRIFT"}'
            '（本批冻结为 0.8.7 代际；确认后再改锁定值）')
        return 2
    log(f'OK 代际身份核验通过：carrier {carrier[:8]}… / adapter {adapter[:8]}…')

    planned: list[tuple[str, str, int]] = []
    for t in tasks:
        meta = index[t]
        planned.append((t, str((meta.get('batches') or ['?'])[0]),
                        int(meta.get('agent_timeout_sec') or 0)))
    if not heavy.pre_pull(planned):
        log('FAIL 预拉未全绿——按用户裁决不带残批起跑，已中止')
        return 3

    snapshot('pre')
    rc_all = 0
    for task, batch, timeout in planned:
        job = f'official-v41-rerun3-{task}{job_suffix}'
        vol = heavy.VOL_ROOT / job
        vol.mkdir(parents=True, exist_ok=True)
        mount_list = [{
            'type': 'bind', 'source': vol.as_posix(), 'target': '/orz-gsa',
        }]
        if task in APT_FIX_TASKS:
            mount_list.append({
                'type': 'bind', 'source': APT_FIX_SOURCES.as_posix(),
                'target': '/etc/apt/sources.list',
            })
        mounts = json.dumps(mount_list)
        # 复用官方起跑器 argv（含 `-i terminal-bench/<题>`）；wallclock=None ⇒
        # 不传 --ak max_wallclock（agent 侧墙钟不施加，官方 task 级超时保留）。
        hargv = heavy.build_argv(vol, mounts, [(task, batch, timeout)], job, None)
        if any('wallclock' in str(a).lower() for a in hargv):
            log(f'[FATAL] {job} 的 argv 含 wallclock——会重开 ps/pgrep 泄漏，中止')
            return 1
        log(f'[rerun3] ==> {job}（题 {task}｜批 {batch}｜官方 agent 超时 '
            f'{timeout}s｜agent 侧墙钟：不施加）')
        if task in APT_FIX_TASKS:
            log(f'[rerun3] 软件源修正挂载（本批偏差，须披露）：'
                f'/etc/apt/sources.list ← {APT_FIX_SOURCES}')
        console_log = OUT_DIR / f'{job}-console.log'
        t0 = time.time()
        env = {**os.environ, 'PYTHONPATH': heavy.TB_EVAL.as_posix()}
        with open(console_log, 'wb') as f:
            rc = subprocess.run(
                [str(heavy.HARBOR), *hargv], stdout=f, stderr=subprocess.STDOUT,
                env=env,
            ).returncode
        elapsed = (time.time() - t0) / 60.0
        log(f'[rerun3] <-- {job} rc={rc} elapsed={elapsed:.1f} min '
            f'控制台={console_log.name}')
        if rc and not rc_all:
            rc_all = rc
        verdict(job, task, elapsed)
        inspect_volume(job, task)
        # 每题镜像清理（官方跑批纪律：题目镜像只在本题用一次）——宿主盘余量
        # 是本批的硬约束，残图堆到中途爆盘会打断整批。
        image = str(index[task].get('image') or '')
        if image:
            try:
                p = subprocess.run(['docker', 'rmi', image],
                                   capture_output=True, text=True, timeout=300)
                log(f'[rerun3] 镜像清理 {image} rc={p.returncode}')
            except (OSError, subprocess.SubprocessError) as exc:
                log(f'[rerun3] 镜像清理失败（不阻断）{image}: {exc}')
    snapshot('post')
    log('== rerun3 结束 ==')
    return rc_all


if __name__ == '__main__':
    raise SystemExit(main())
