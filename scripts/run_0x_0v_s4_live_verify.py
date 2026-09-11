# 0x / 0v S4 实机复验 — 单题执行器（2026-09-11）
#
# 用途：一题（可多题）单次实机，为两条 S4 同时取证——
#   0x 初始轮中立问询：验证「开局一次、动作中不复发」（journal 面：
#     `orientation_checkpoint` 的 `trigger=initial_round` 恰好 1 次、
#     `injection_position=post_tool_batch_gap`，周期问询独立触发）。
#   0v 检索引擎 SERP（判据见 RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN
#     _2026-09-10 §4）：检索面 `browser_control search` 引擎链/标注/预算。
#
# 口径：k=1、`-r 0`（harbor 侧重试关闭，重试收归单层）、官方墙钟唯一
# （无 agent 超时覆盖）、deepseek-v4-flash、eval_browser=true、Docker 容器；
# 无代理直连 + 本地预拉镜像（2026-09-10 用户裁决；Docker Hub 不可达，
# 只能跑本地镜像——与 0u R4 同纪律）。
# 载体：orz 0.4.1 三件套 D:/tb-eval/orz-linux/orz（0x S3 重建产物，
# sha256 4b83de75…，见 docs/audits/0X_S3_DUAL_PLATFORM_REBUILD_2026-09-11.md）。
# 实现说明：Python 而非 PowerShell——PS 5.1 会破坏内嵌 JSON 引号；Python
# subprocess argv 直传规避（沿 run_r4_unsolved15_per_task.py 教训）。
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

PYTHONPATH_ROOT = 'D:/tb-eval'
HARBOR = Path('D:/tb-eval/venv/Scripts/harbor.exe')
JOBS_DIR = Path('D:/tb-eval/jobs-s4')
VOL_ROOT = Path('D:/tb-eval/gsa-volumes')
BINARY = Path('D:/tb-eval/orz-linux/orz')
MODEL = 'deepseek-v4-flash'
JOB_PREFIX = 's4-0x-0v'
# agent 准备阶段超时余量：harbor 默认 360s，eval_browser=true 需在容器内装
# Chromium（坏线路下 ~15min），故 ×4（沿 0w 第二跑后修订）。
AGENT_SETUP_TIMEOUT_MULTIPLIER = '4'

TASKS = ['dna-assembly']


def log(msg: str) -> None:
    JOBS_DIR.mkdir(parents=True, exist_ok=True)
    line = f'{time.strftime("%Y-%m-%d %H:%M:%S")} {msg}' if msg.startswith(('pass', '==', 'START', 'DONE', 'FAIL', 'SKIP')) else msg
    with open(JOBS_DIR / f'{JOB_PREFIX}-summary.log', 'a', encoding='utf-8') as f:
        f.write(line + '\n')
    print(line, flush=True)


def preflight() -> int:
    """跑批前置门：无残留容器 + harbor 鉴权在线。不通过即返回非 0（零轮次消耗）。"""
    ps = subprocess.run(
        ['docker', 'ps', '-q'], capture_output=True, text=True
    )
    residual = ps.stdout.strip()
    if residual:
        log(f'PREFLIGHT FAIL: residual containers present: {residual!r}')
        return 2
    log('preflight: no residual containers')
    auth = subprocess.run([str(HARBOR), 'auth', 'status'], capture_output=True, text=True)
    if auth.returncode != 0:
        log(f'PREFLIGHT FAIL: harbor auth status exit={auth.returncode}: {auth.stdout.strip()} {auth.stderr.strip()}')
        return 2
    log(f'preflight: harbor auth ok ({auth.stdout.strip().splitlines()[0] if auth.stdout.strip() else "?"})')
    return 0


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
        'run', '-t', f'terminal-bench/{task}', '-n', '1', '-r', '0',
        '--agent-setup-timeout-multiplier', AGENT_SETUP_TIMEOUT_MULTIPLIER,
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
    env = {**os.environ, 'PYTHONPATH': PYTHONPATH_ROOT,
           'TMP': 'D:/tb-eval/tmp', 'TEMP': 'D:/tb-eval/tmp'}
    console = JOBS_DIR / f'{job}-console.log'
    with open(console, 'wb') as f:
        proc = subprocess.run([str(HARBOR), *args], stdout=f, stderr=subprocess.STDOUT, env=env)
    return proc.returncode


def main() -> int:
    code = preflight()
    if code != 0:
        return code
    h = hashlib.sha256(BINARY.read_bytes()).hexdigest()
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} 0x/0v S4 live verify start ==')
    log(f'binary {BINARY.as_posix()} size={BINARY.stat().st_size} sha256={h}')
    log(f'platform orz-bin 0.4.1 (0x S3 rebuild) | model={MODEL} | k=1 | r=0 | tasks={",".join(TASKS)}')
    rc = 0
    for task in TASKS:
        log(f'START {task}')
        c = invoke_one_task(task)
        if c == 0:
            log(f'DONE {task} exit={c}')
        else:
            log(f'FAIL {task} exit={c}')
            rc = 1
    log(f'== finished rc={rc} ==')
    return rc


if __name__ == '__main__':
    sys.exit(main())
