# R4 补跑 4 题 — 余额恢复后重跑（official-r4b-unsolved4，2026-09-10）
# 对象：R4 中受 DeepSeek 余额耗尽影响的 4 题（用户裁决补跑）：
#   protein-assembly / train-fasttext（从未运行）、
#   path-tracing-reverse（1272s 中断）、filter-js-from-html（pass-1 无效）。
# 口径与 R4 一致（载体 0.4.0、k=1、官方墙钟、eval_browser=true、无代理 + 本地镜像）。
# 判据硬化（R4 盲区登记落实）：job_complete 除 result.json reward 面外，还要求
# gsa 卷内存在真实 run（events.jsonl 事件数 >10），杜绝「余额秒死 + verifier-only
# reward」被误判为有效试次。
import json
import os
import subprocess
import sys
import time
from pathlib import Path

PYTHONPATH_ROOT = 'D:/tb-eval'
HARBOR = Path('D:/tb-eval/venv/Scripts/harbor.exe')
DATASET = 'terminal-bench/terminal-bench-2-1@sha256:7d7bdc1cbedad549fc1140404bd4dc45e5fd0ea7c4186773687d177ad3a0699a'
JOBS_DIR = Path('D:/tb-eval/jobs-official')
VOL_ROOT = Path('D:/tb-eval/gsa-volumes')
BINARY = Path('D:/tb-eval/orz-linux/orz')
MODEL = 'deepseek-v4-flash'
JOB_PREFIX = 'official-r4b-unsolved4'
SUMMARY = JOBS_DIR / f'{JOB_PREFIX}-per-task-summary.log'
MAX_PASSES = 3

TASKS = [
    'protein-assembly',
    'train-fasttext',
    'path-tracing-reverse',
    'filter-js-from-html',
]


def log(msg: str) -> None:
    with open(SUMMARY, 'a', encoding='utf-8') as f:
        f.write(msg + '\n')
    print(msg, flush=True)


def real_run_exists(task: str) -> bool:
    vol = VOL_ROOT / f'{JOB_PREFIX}-{task}'
    for f in vol.glob('*/runs/RUN-CLI-*/events.jsonl'):
        try:
            n = sum(1 for line in f.read_text(encoding='utf-8', errors='replace').splitlines() if line.strip())
        except OSError:
            continue
        if n > 10:
            return True
    return False


def job_complete(task: str) -> bool:
    rj = JOBS_DIR / f'{JOB_PREFIX}-{task}' / 'result.json'
    if not rj.is_file():
        return False
    try:
        r = json.loads(rj.read_text(encoding='utf-8'))
    except (OSError, ValueError):
        return False
    if not r.get('finished_at'):
        return False
    evals = ((r.get('stats') or {}).get('evals')) or {}
    if not evals:
        return False
    ev = next(iter(evals.values()))
    reward = ((ev or {}).get('reward_stats') or {}).get('reward') or {}
    has_reward = any(isinstance(v, list) and len(v) > 0 for v in reward.values())
    # 判据硬化：必须有真实 agent run（verifier-only reward 不算完成）
    return has_reward and real_run_exists(task)


def invoke_one_task(task: str) -> int:
    job = f'{JOB_PREFIX}-{task}'
    job_dir = JOBS_DIR / job
    if job_dir.exists():
        stale = f'{job}-stale-{time.strftime("%Y%m%d-%H%M%S")}'
        job_dir.rename(JOBS_DIR / stale)
    vol_dir = VOL_ROOT / job
    vol_dir.mkdir(parents=True, exist_ok=True)
    mounts = json.dumps([{
        'type': 'bind',
        'source': str(vol_dir).replace('\\', '/'),
        'target': '/orz-gsa',
    }])
    args = [
        'run', '-d', DATASET, '-i', f'terminal-bench/{task}', '-n', '1', '-r', '3',
        '-a', 'tb_agents.orz:Orz', '-m', MODEL,
        '--ak', f'orz_binary={BINARY.as_posix()}',
        '--ak', f'model_id={MODEL}',
        '--ak', f'gsa_volume={vol_dir.as_posix()}',
        '--ak', 'eval_browser=true',
        '--mounts', mounts,
        '--env-file', 'D:/tb-eval/.env',
        '--job-name', job,
        '-o', str(JOBS_DIR),
        '-k', '1', '-y',
    ]
    env = {**os.environ, 'PYTHONPATH': PYTHONPATH_ROOT, 'TMP': 'D:/tb-eval/tmp', 'TEMP': 'D:/tb-eval/tmp'}
    console = JOBS_DIR / f'{job}-console.log'
    with open(console, 'wb') as f:
        proc = subprocess.run([str(HARBOR), *args], stdout=f, stderr=subprocess.STDOUT, env=env)
    return proc.returncode


def main() -> int:
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} R4b supplement run start (4 tasks) ==')
    pending = list(TASKS)
    for pass_no in range(1, MAX_PASSES + 1):
        if not pending:
            break
        log(f'pass {pass_no}: {len(pending)} pending')
        nxt = []
        for task in pending:
            if job_complete(task):
                log(f'SKIP (complete) {task}')
                continue
            log(f'START {task}')
            code = invoke_one_task(task)
            if code == 0 and job_complete(task):
                log(f'DONE {task} exit={code}')
            else:
                log(f'FAIL {task} exit={code}')
                nxt.append(task)
        pending = nxt
    log(f'== finished; unresolved: {", ".join(pending)} ==')
    return 0 if not pending else 1


if __name__ == '__main__':
    sys.exit(main())
