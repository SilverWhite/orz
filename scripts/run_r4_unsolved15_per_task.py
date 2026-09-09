# 官方 R4 未通过 15 题复跑 — 一题一作业顺序执行器（2026-09-10）
# 用途：R3（official-r3-unsolved20）未通过 15 题按官方口径再跑一轮；同时作为
# 0t S4 实机复验载体（判据见 RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09 §5，
# 判据预登记见 BACKLOG 0u）。口径与 R3 一致：k=1、一题一作业、deepseek-v4-flash、
# eval_browser=true、官方数据集 pin、官方墙钟唯一（无超时覆盖）、Docker 容器；
# 无代理直连 + 本地预拉镜像（2026-09-10 用户裁决）。
# 载体：orz 0.4.0 发布三件套 D:/tb-eval/orz-linux/orz（109,066,552 B，SHA256
# 0797610e…，docs/audits/0.4.0_RELEASE_2026-09-09.md 锁定；orz a467d0f9 =
# 0t S3 92875fd5 + 版本 bump，双车道代码同一）。
# 实现说明：Python 而非 PowerShell——R3 教训（母审计 §4.2）：PS 5.1 原生参数
# JSON 内嵌引号被破坏（JSONDecodeError at char 2），且当前机器已无 pwsh 7；
# Python subprocess argv 直传规避引号问题。harbor 参数与
# run_r3_unsolved20_per_task.ps1 逐项一致。
import json
import hashlib
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
JOB_PREFIX = 'official-r4-unsolved15'
SUMMARY = JOBS_DIR / f'{JOB_PREFIX}-per-task-summary.log'
MAX_PASSES = 3

TASKS = [
    'adaptive-rejection-sampler',
    'dna-assembly',
    'dna-insert',
    'extract-elf',
    'extract-moves-from-video',
    'filter-js-from-html',
    'gcode-to-text',
    'gpt2-codegolf',
    'make-doom-for-mips',
    'make-mips-interpreter',
    'model-extraction-relu-logits',
    'path-tracing',
    'path-tracing-reverse',
    'protein-assembly',
    'train-fasttext',
]


def log(msg: str) -> None:
    line = f'{time.strftime("%Y-%m-%d %H:%M:%S")} {msg}' if msg.startswith(('pass', '==')) else msg
    with open(SUMMARY, 'a', encoding='utf-8') as f:
        f.write(line + '\n')
    print(line, flush=True)


def job_complete(task: str) -> bool:
    """完成判据与 r3 执行器一致：result.json finished_at 非空且
    stats.evals[*].reward_stats.reward 有非空数值键（harbor exit 0 会掩盖
    AgentSetupTimeout 等基建错误，不能只看退出码）。"""
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
    return any(isinstance(v, list) and len(v) > 0 for v in reward.values())


def invoke_one_task(task: str) -> int:
    job = f'{JOB_PREFIX}-{task}'
    job_dir = JOBS_DIR / job
    # 同名 job 重跑陷阱（r3 教训）：harbor 会把既有错误试次直接计入而不重新
    # 执行，先把无效/未完成作业目录移存归档强制开新 job。
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
    env = {**os.environ, 'PYTHONPATH': PYTHONPATH_ROOT}
    console = JOBS_DIR / f'{job}-console.log'
    with open(console, 'wb') as f:
        proc = subprocess.run([str(HARBOR), *args], stdout=f, stderr=subprocess.STDOUT, env=env)
    return proc.returncode


def main() -> int:
    h = hashlib.sha256(BINARY.read_bytes()).hexdigest()
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} R4 per-task run start (15 tasks) ==')
    log(f'binary {BINARY.as_posix()} size={BINARY.stat().st_size} sha256={h}')
    log(f'dataset {DATASET}')
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
