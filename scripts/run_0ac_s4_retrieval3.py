# 0ac S4 实机复验 — 检索主导小任务集（s4-0ac-retrieval3，2026-09-27）
# 口径：载体 0.8.0 双平台在役（D:/tb-eval/orz-linux/orz）、k=1、官方墙钟形态、
# eval_browser=true、max_wallclock=840（FR-D02 旗标传递读数随本轮收取）、
# 不上传（--upload 缺席＝不评正式分，0v-B 探针先例）。
# 判据（0ac 索引条目 / 设计稿 §4.3）：
#   ① 检索类首个结果 wall_ms p99 ≤ 10 s（本地路径按引擎单独计时 + 30 s 整体兜底）；
#   ② `subagent_wallclock_timeout_mid_tool` = 0；
#   ③ 等待路径零事件 = 0（任何等待型调用在截止后必须产出带稳定码的结果）。
# 任务集＝R1/R4 检索主导死因族抽 3 题：mteb-leaderboard / count-dataset-tokens /
# model-extraction-relu-logits（S4 不评正式分，reward 不作判据）。
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
JOB_PREFIX = 's4-0ac-retrieval3'
SUMMARY = JOBS_DIR / f'{JOB_PREFIX}-per-task-summary.log'
MAX_PASSES = 2

TASKS = [
    'mteb-leaderboard',
    'count-dataset-tokens',
    'model-extraction-relu-logits',
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
    return real_run_exists(task)


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
        '--ak', 'max_wallclock=840',
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
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} 0ac S4 retrieval-3 run start ==')
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
            ok = code == 0 and job_complete(task)
            log(f'END {task} exit={code} complete={ok}')
            if not ok:
                nxt.append(task)
        pending = nxt
    log('== done ==')
    return 0 if not pending else 1


if __name__ == '__main__':
    sys.exit(main())
