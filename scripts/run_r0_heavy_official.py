#!/usr/bin/env python3
"""TB 2.1 第 0 轮（内存重题前置轮）官方口径起跑器 — k=1、串行、单作业。

口径来源（**不修改冻结 runner**）：
  ``docs/audits/TB21_V41_EVAL_CONTAINER_AUDIT_2026-09-13.md`` §5.3（命令）
  ``docs/TB21_V41_GENERATION_ROUND_SCHEDULE_2026-09-13.md`` §3.2（四项用户裁决）

* 题集：冻结 89 题中 ``memory_mb == 8192`` 的 **8 题全集**（实测量，非 6 题）；
  亦可用 ``--tasks`` 指定任意子集（补跑/复跑用），``--job-name`` 指定作业名；
* **预拉镜像（2026-09-13 用户裁决）**：起跑前按冻结清单的 ``metadata.docker_image``
  逐个 ``docker pull``（默认每镜像最多 4 次重试、**实时落盘日志**），并把**解析后的
  repo digest** 记入轮次日志——本轮的镜像缺口与拉取中断已证明「按需拉取」不可靠；
  ``--no-pre-pull`` 可跳过（仅在镜像已确认在位时使用）；
* 形态：``-k 1``（每题 1 试次）、``-n 1``（串行，降并发落点）、``--upload --public``、
  官方数据集 pin、无时间倍率、无 ``max_wallclock``、无 ``--max-retries``（= 官方默认 0）；
* 归属：第 0 轮**计入**本轮 89 题（2026-09-13 用户裁决）——后续 1–5 批据此显式剔除这 8 题
  （16/15/15/17/18 = 81 题，81 + 8 = 89，每题恰 1 次）；分批偏离在台账登记；
* 卷 / 产物：``gsa-volumes/official-r0-heavy``、``jobs-official/official-r0-heavy``；
* 代际记录（排期 §2）：起跑时打印载体 / 适配器 / 数据集 pin 身份哈希与起止时间戳，
  身份不符即**硬中止**（跑出第二代人不可比的数据才是真代价）。

Python 而非 PowerShell：R3 教训（母审计 §4.2）——PS 5.1 原生参数把内嵌引号
（``--mounts`` 的 JSON）打坏，Python ``subprocess`` argv 直传规避引号问题
（与 ``run_r4_unsolved15_per_task.py`` 同一形态）。

静默旁路读数件（``tb21_round_gate.py`` / ``tb21_friction_scan.py``）**不接入本脚本**：
其契约是不进跑批链条、不阻断跑批，由人工另行读取。
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

TB_EVAL = Path('D:/tb-eval')
HARBOR = TB_EVAL / 'venv/Scripts/harbor.exe'
ORZ = TB_EVAL / 'orz-linux/orz'
ADAPTER = TB_EVAL / 'tb_agents/orz.py'
ENV_FILE = TB_EVAL / '.env'
JOBS_DIR = TB_EVAL / 'jobs-official'
VOL_ROOT = TB_EVAL / 'gsa-volumes'

DATASET = (
    'terminal-bench/terminal-bench-2-1@'
    'sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
)
MODEL = 'deepseek-v4-flash'
JOB_NAME = 'official-r0-heavy'
ROUND_LOG = JOBS_DIR / f'{JOB_NAME}-round.log'
CONSOLE_LOG = JOBS_DIR / f'{JOB_NAME}-console.log'
MANIFEST = (
    Path(__file__).resolve().parent.parent
    / 'evaluation/corpus-freeze/tb21-official-89-2026-09-13.manifest.json'
)
PULL_LOG = JOBS_DIR / 'preroll-images.log'
PULL_TRIES = 4

# 0.5.0 载体三件套锁定值（D:/tb-eval/orz-0.5.0-carrier-hashes.txt）
EXPECTED_CARRIER_SHA256 = (
    '393eee34dd0357cab088f289fa1068a39294a13fff48d80d33c50a40684dd623'
)
# 适配器锁定值（2026-09-13 容器侧排查记录 §1）
EXPECTED_ADAPTER_SHA256 = (
    '2737cfadc5c43603b73164b51343e58a671c0efeee8d9ab7a626dda8dae51490'
)

# 默认题集（第 0 轮 = 内存重题 8 题）：冻结批次 / 官方 agent 超时（秒），按冻结清单序
DEFAULT_TASKS: list[tuple[str, str, int]] = [
    ('mcmc-sampling-stan', 'B2', 1800),
    ('gpt2-codegolf', 'B2', 900),
    ('mteb-leaderboard', 'B3', 3600),
    ('caffe-cifar-10', 'B3', 3600),
    ('torch-pipeline-parallelism', 'B3', 900),
    ('rstan-to-pystan', 'B3', 1800),
    ('torch-tensor-parallelism', 'B4', 900),
    ('filter-js-from-html', 'B5', 1800),
]


def manifest_index() -> dict[str, dict[str, object]]:
    """冻结清单的逐题元数据（镜像 / 批次 / agent 超时）——选题与预拉的权威来源。"""
    data = json.loads(MANIFEST.read_text(encoding='utf-8'))
    index: dict[str, dict[str, object]] = {}
    for rec in data.get('tasks', []):
        name = rec.get('task')
        if not name:
            continue
        meta = rec.get('metadata') or {}
        index[str(name)] = {
            'image': meta.get('docker_image'),
            'agent_timeout_sec': meta.get('agent_timeout_sec'),
            'batches': rec.get('batches') or [],
        }
    return index


def select_tasks(spec: str | None) -> list[tuple[str, str, int]]:
    """``--tasks a,b,c`` 选题；缺省 = 第 0 轮 8 题。批次/超时取自冻结清单。"""
    if not spec:
        return list(DEFAULT_TASKS)
    index = manifest_index()
    picked: list[tuple[str, str, int]] = []
    for name in [t.strip() for t in spec.split(',') if t.strip()]:
        meta = index.get(name)
        if meta is None:
            raise SystemExit(f'未知题名（不在冻结清单内）：{name}')
        batch = (meta['batches'] or ['?'])[0]
        picked.append((name, str(batch), int(meta['agent_timeout_sec'] or 0)))
    return picked


def image_digest(image: str) -> str:
    insp = subprocess.run(
        ['docker', 'image', 'inspect', '--format', '{{index .RepoDigests 0}}', image],
        capture_output=True, text=True,
    )
    return (insp.stdout or '').strip() or '<none>'


def pre_pull(tasks: list[tuple[str, str, int]]) -> bool:
    """预拉镜像（2026-09-13 用户裁决）：逐题 ``docker pull``，重试 + 实时落盘 + 记 digest。

    本轮实测「按需拉取」会在跑批中途被网络中断打成 RuntimeError（2 题未起跑），
    故预拉改为默认动作；**拉取未全绿即中止，不带残批起跑**。日志 `preroll-images.log`。
    """
    index = manifest_index()
    t_all = time.time()
    ok = True
    PULL_LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(PULL_LOG, 'a', encoding='utf-8') as fh:
        fh.write(f'== preroll {time.strftime("%Y-%m-%d %H:%M:%S")} '
                 f'({len(tasks)} images) ==\n')
        fh.flush()
        for task, _batch, _timeout in tasks:
            image = (index.get(task) or {}).get('image')
            if not image:
                fh.write(f'FAIL {task}: 冻结清单无 docker_image\n')
                fh.flush()
                ok = False
                continue
            image = str(image)
            t0 = time.time()
            rc = 1
            for attempt in range(1, PULL_TRIES + 1):
                fh.write(f'--- {task} {image} try {attempt}/{PULL_TRIES} '
                         f'{time.strftime("%H:%M:%S")}\n')
                fh.flush()
                proc = subprocess.run(
                    ['docker', 'pull', image], capture_output=True, text=True
                )
                tail = (proc.stdout or proc.stderr or '').strip().splitlines()[-3:]
                fh.write(''.join(f'    {ln}\n' for ln in tail))
                fh.flush()
                rc = proc.returncode
                if rc == 0:
                    break
                log(f'[preroll] retry {attempt}/{PULL_TRIES} failed rc={rc} {image}')
                time.sleep(15)
            if rc != 0:
                fh.write(f'FAIL {task} {image} rc={rc} after {PULL_TRIES} tries\n')
                fh.flush()
                ok = False
                continue
            digest = image_digest(image)
            fh.write(f'OK {task} {image} digest={digest} ({time.time() - t0:.0f}s)\n')
            fh.flush()
            log(f'[preroll] OK {task} {image} digest={digest}')
    log(f'[preroll] done in {(time.time() - t_all) / 60:.1f} min ok={ok}')
    return ok


def log(msg: str) -> None:
    line = f'{time.strftime("%Y-%m-%d %H:%M:%S")} {msg}'
    ROUND_LOG.parent.mkdir(parents=True, exist_ok=True)
    with open(ROUND_LOG, 'a', encoding='utf-8') as f:
        f.write(line + '\n')
    print(line, flush=True)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def snapshot(tag: str) -> None:
    """资源与身份快照（宿主视角；容器内磁盘读数只反映盘内视角，另记）。"""
    try:
        usage = shutil.disk_usage('D:/')
        disk = f'D: free {usage.free / 2**30:.2f} GiB / total {usage.total / 2**30:.2f} GiB'
    except OSError as exc:  # pragma: no cover - 宿主查询失败不阻断跑批
        disk = f'D: free <unavailable: {exc}>'
    counts: dict[str, str] = {}
    for key, argv in (
        ('images', ['docker', 'images', '-q']),
        ('containers', ['docker', 'ps', '-aq']),
    ):
        try:
            proc = subprocess.run(argv, capture_output=True, text=True, timeout=60)
            items = [ln for ln in proc.stdout.splitlines() if ln.strip()]
            counts[key] = str(len(items)) if proc.returncode == 0 else f'<rc={proc.returncode}>'
        except (OSError, subprocess.SubprocessError) as exc:
            counts[key] = f'<unavailable: {exc}>'
    log(
        f'[snapshot:{tag}] {disk} | docker images={counts["images"]} '
        f'containers={counts["containers"]}'
    )


def build_argv(
    vol_dir: Path, mounts: str, tasks: list[tuple[str, str, int]], job_name: str
) -> list[str]:
    args = ['run', '-d', DATASET]
    for task, _batch, _timeout in tasks:
        args += ['-i', f'terminal-bench/{task}']
    args += [
        '-a', 'tb_agents.orz:Orz',
        '-m', MODEL,
        '--ak', f'orz_binary={ORZ.as_posix()}',
        '--ak', f'model_id={MODEL}',
        '--ak', f'gsa_volume={vol_dir.as_posix()}',
        '--mounts', mounts,
        '--env-file', ENV_FILE.as_posix(),
        '--job-name', job_name,
        '-o', JOBS_DIR.as_posix(),
        '-k', '1',
        '-n', '1',
        '--upload',
        '--public',
        '-y',
    ]
    return args


def job_summary(job_name: str) -> dict[str, object]:
    """读 harbor 产出的作业结果（作业级 + 逐题级），只作读数，不判分。"""
    job_dir = JOBS_DIR / job_name
    out: dict[str, object] = {'job_dir': str(job_dir), 'tasks': {}}
    if not job_dir.is_dir():
        return out
    trials: dict[str, dict[str, object]] = {}
    for result in sorted(job_dir.rglob('result.json')):
        try:
            data = json.loads(result.read_text(encoding='utf-8'))
        except (OSError, ValueError):
            continue
        name = (data.get('task_id') or {}).get('name')
        if not name:
            continue
        reward = None
        vr = data.get('verifier_result')
        if isinstance(vr, dict) and isinstance(vr.get('rewards'), dict):
            reward = vr['rewards'].get('reward')
        rec = trials.setdefault(name, {'trials': 0, 'rewards': []})
        rec['trials'] = int(rec['trials']) + 1
        if reward is not None:
            rec['rewards'].append(float(reward))
    out['tasks'] = trials
    return out


def main(argv: list[str] | None = None) -> int:
    global JOB_NAME, ROUND_LOG, CONSOLE_LOG
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument('--dry-run', action='store_true',
                    help='print the resolved harbor argv and pre-flight, then exit')
    ap.add_argument('--allow-identity-drift', action='store_true',
                    help='run even if carrier/adapter hashes differ from the locked values')
    ap.add_argument('--tasks', default=None,
                    help='逗号分隔的题名子集（缺省 = 第 0 轮 8 题）；批次/超时取自冻结清单')
    ap.add_argument('--job-name', default=JOB_NAME, help=f'作业名（缺省 {JOB_NAME}）')
    ap.add_argument('--no-pre-pull', action='store_true',
                    help='跳过预拉（仅在确认镜像已全在位时使用）')
    ap.add_argument('--pull-only', action='store_true',
                    help='只预拉镜像，不起跑')
    args = ap.parse_args(argv)

    tasks = select_tasks(args.tasks)
    JOB_NAME = args.job_name
    ROUND_LOG = JOBS_DIR / f'{JOB_NAME}-round.log'
    CONSOLE_LOG = JOBS_DIR / f'{JOB_NAME}-console.log'

    vol_dir = VOL_ROOT / JOB_NAME
    mounts = json.dumps([{
        'type': 'bind',
        'source': vol_dir.as_posix(),
        'target': '/orz-gsa',
    }])
    harbor_argv = build_argv(vol_dir, mounts, tasks, JOB_NAME)

    carrier = sha256_file(ORZ)
    adapter = sha256_file(ADAPTER)
    plan = {
        'job_name': JOB_NAME,
        'tasks': [t for t, _b, _s in tasks],
        'n_attempts_k': 1,
        'n_concurrent': 1,
        'upload': 'public',
        'dataset': DATASET,
        'model': MODEL,
        'volume': vol_dir.as_posix(),
        'carrier_orz_sha256': carrier,
        'adapter_orz_py_sha256': adapter,
        'worst_case_agent_seconds': sum(s for _t, _b, s in tasks),
        'pre_pull': not args.no_pre_pull,
    }

    if args.dry_run:
        print(json.dumps(plan, indent=2, ensure_ascii=False))
        for task, _b, _s in tasks:
            print('image:', (manifest_index().get(task) or {}).get('image'))
        print('argv:', ' '.join([str(HARBOR), *harbor_argv]))
        return 0

    vol_dir.mkdir(parents=True, exist_ok=True)
    JOBS_DIR.mkdir(parents=True, exist_ok=True)
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} 起跑 job={JOB_NAME} '
        f'{len(tasks)} 题 ==')
    log(f'plan {json.dumps(plan, ensure_ascii=False)}')

    if carrier != EXPECTED_CARRIER_SHA256 or adapter != EXPECTED_ADAPTER_SHA256:
        log(
            'FAIL 代际身份不符：carrier '
            f'{"OK" if carrier == EXPECTED_CARRIER_SHA256 else "DRIFT"} / adapter '
            f'{"OK" if adapter == EXPECTED_ADAPTER_SHA256 else "DRIFT"} '
            '（排期 §2 代际纪律；确认后用 --allow-identity-drift 显式放行）'
        )
        if not args.allow_identity_drift:
            return 2
    else:
        log('OK 代际身份核验通过（载体 0.5.0 + 适配器锁定值）')

    if not args.no_pre_pull:
        if not pre_pull(tasks):
            log('FAIL 预拉未全绿——按用户裁决不带残批起跑，已中止')
            return 3
    if args.pull_only:
        log('== 仅预拉，按 --pull-only 结束 ==')
        return 0

    snapshot('pre')
    log(f'harness {HARBOR} ; console log -> {CONSOLE_LOG}')
    env = {**os.environ, 'PYTHONPATH': TB_EVAL.as_posix()}
    started = time.time()
    with open(CONSOLE_LOG, 'wb') as f:
        code = subprocess.run(
            [str(HARBOR), *harbor_argv], stdout=f, stderr=subprocess.STDOUT, env=env
        ).returncode
    elapsed = time.time() - started
    snapshot('post')
    summary = job_summary(JOB_NAME)
    log(f'harbor exit={code} elapsed={elapsed / 3600:.2f} h')
    log(f'per-task {json.dumps(summary["tasks"], ensure_ascii=False)}')
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} 结束 job={JOB_NAME} '
        f'（exit={code}）==')
    return code


if __name__ == '__main__':
    sys.exit(main())
