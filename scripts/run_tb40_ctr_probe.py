# TB 4.0 单题摩擦探针 — ctr-optimization（official-tb40-ctr-optimization，2026-09-10）
# 定位：单题、单次（k=1）、官方口径的**摩擦探针**，不是成绩批次（TB 4.0 无
#       DeepSeek 同模型参照，见 docs/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10.md §0）。
# 口径：官方墙钟唯一（本题 [agent] timeout_sec = 28800，实际约 4.8 小时）、
#       eval_browser=true、Docker 容器、TEMP 重定向 D:/tb-eval/tmp。
# 判据硬化（沿 r4b）：job_complete 除 result.json reward 面外，还要求 gsa 卷内
#       存在真实 run（events.jsonl 事件数 >10），杜绝 stub/秒死被计为有效试次。
# 重试：无效试次（无 reward 或事件数 ≤10）至多 3 轮；有效试次（含 reward 0.0
#       且事件数正常）不重跑——本题目的本就是看框架跑长任务的表现。
import json
import os
import subprocess
import sys
import time
from pathlib import Path

PYTHONPATH_ROOT = 'D:/tb-eval'
HARBOR = Path('D:/tb-eval/venv/Scripts/harbor.exe')
# 2026-09-10 复测：TB 4.0 的**数据集级**解析持续失败（服务端 DB statement timeout /
# HTTP/2 ConnectionState.CLOSED），但**单题级**解析正常（ctr-optimization 已实测可取）。
# 故本探针走 `harbor run -t <org/name>` 单题路径，不用 `-d` 数据集路径。
# 代价：不落数据集级 digest 钉；题目自身 ref 见每 trial result.json 的 task_id.ref。
TASK = 'ctr-optimization'
JOBS_DIR = Path('D:/tb-eval/jobs-official')
VOL_ROOT = Path('D:/tb-eval/gsa-volumes')
BINARY = Path('D:/tb-eval/orz-linux/orz')
MODEL = 'deepseek-v4-flash'
JOB = 'official-tb40-ctr-optimization'
SUMMARY = JOBS_DIR / f'{JOB}-summary.log'
MAX_PASSES = 3


def log(msg: str) -> None:
    with open(SUMMARY, 'a', encoding='utf-8') as f:
        f.write(msg + '\n')
    print(msg, flush=True)


def real_run_exists() -> bool:
    vol = VOL_ROOT / JOB
    for f in vol.glob('*/runs/RUN-CLI-*/events.jsonl'):
        try:
            n = sum(1 for line in f.read_text(encoding='utf-8', errors='replace').splitlines() if line.strip())
        except OSError:
            continue
        if n > 10:
            return True
    return False


def job_complete() -> bool:
    rj = JOBS_DIR / JOB / 'result.json'
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
    return has_reward and real_run_exists()


def invoke() -> int:
    job_dir = JOBS_DIR / JOB
    if job_dir.exists():
        stale = f'{JOB}-stale-{time.strftime("%Y%m%d-%H%M%S")}'
        job_dir.rename(JOBS_DIR / stale)
    vol_dir = VOL_ROOT / JOB
    vol_dir.mkdir(parents=True, exist_ok=True)
    mounts = json.dumps([{
        'type': 'bind',
        'source': str(vol_dir).replace('\\', '/'),
        'target': '/orz-gsa',
    }])
    args = [
        'run', '-t', f'terminal-bench/{TASK}', '-n', '1', '-r', '3',
        '-a', 'tb_agents.orz:Orz', '-m', MODEL,
        '--ak', f'orz_binary={BINARY.as_posix()}',
        '--ak', f'model_id={MODEL}',
        '--ak', f'gsa_volume={vol_dir.as_posix()}',
        '--ak', 'eval_browser=true',
        '--mounts', mounts,
        '--env-file', 'D:/tb-eval/.env',
        '--job-name', JOB,
        '-o', str(JOBS_DIR),
        '-k', '1', '-y',
    ]
    env = {**os.environ, 'PYTHONPATH': PYTHONPATH_ROOT, 'TMP': 'D:/tb-eval/tmp', 'TEMP': 'D:/tb-eval/tmp'}
    console = JOBS_DIR / f'{JOB}-console.log'
    with open(console, 'wb') as f:
        proc = subprocess.run([str(HARBOR), *args], stdout=f, stderr=subprocess.STDOUT, env=env)
    return proc.returncode


def main() -> int:
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} TB4.0 ctr-optimization friction probe start ==')
    log(f'task={TASK} (single-task resolution; dataset-level resolution unavailable) binary={BINARY} model={MODEL} k=1')
    if job_complete():
        log('SKIP (complete)')
        return 0
    for pass_no in range(1, MAX_PASSES + 1):
        log(f'pass {pass_no}')
        code = invoke()
        if code == 0 and job_complete():
            log(f'DONE exit={code}')
            return 0
        log(f'FAIL exit={code}')
    log('== finished; unresolved (invalid trials exhausted) ==')
    return 1


if __name__ == '__main__':
    sys.exit(main())
