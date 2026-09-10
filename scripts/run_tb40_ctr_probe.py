# TB 4.0 单题摩擦探针 — ctr-optimization（official-tb40-ctr-optimization，2026-09-10）
# 定位：单题、单次（k=1）、官方口径的**摩擦探针**，不是成绩批次（TB 4.0 无
#       DeepSeek 同模型参照，见 docs/TB40_CTR_OPTIMIZATION_FRICTION_PROBE_SCHEDULE_2026-09-10.md §0）。
# 口径：官方墙钟唯一（本题 [agent] timeout_sec = 28800，实际约 4.8 小时）、
#       eval_browser=true、Docker 容器、TEMP 重定向 D:/tb-eval/tmp。
# 判据硬化（沿 r4b）：job_complete 除 result.json reward 面外，还要求 gsa 卷内
#       存在真实 run（events.jsonl 事件数 >10），杜绝 stub/秒死被计为有效试次。
#       并排除出错试次（n_errored_trials>0 或 eval 级 n_errors>0）——环境/传输类
#       中断会以 reward 0.0 + exception_stats 落账，不得被计为「批次完成」。
# 重试：无效试次（无 reward 或事件数 ≤10）至多 3 轮；有效试次（含 reward 0.0
#       且事件数正常）不重跑——本题目的本就是看框架跑长任务的表现。
#       harbor 侧固定 -r 0：试用例级自动重试关闭，重试策略收归本脚本单层
#       （可见、可审计，避免与脚本三轮叠加成最多 12 次尝试）。
#
# 2026-09-10 修订（首跑暴露的执行器缺陷）：
#   1) 每 pass 独立控制台日志：原实现以 'wb' 覆写同一文件，首跑时 pass 1 的
#      控制台证据被 pass 2/3 冲掉，而 pass 1 恰是唯一有真实试次的轮次。
#   2) 前置鉴权预检门：harbor 自身的 API-key 交换（hub 的 TLS 通路）失败时，
#      原实现照常消耗 pass 2/3，把重试预算烧在环境问题上。现在起跑前先做一次
#      hub / 模型端点 TLS+HTTP 预检，不通过即整体顺延（退出码 2，零 pass 消耗）；
#      批内一旦识别出同类鉴权失败，同样立即中止剩余 pass。
#   3) 验收口径补漏：job_complete 原只看「有 reward + 卷内有事件」，而出错试次
#      同样带 reward 0.0 记录，作业若正常收尾即被误判为完成（把环境中断写成有效
#      试次）。现在出错试次一律判为无效试次并走重试。
#   4) 关闭 harbor 侧 -r（原为 3）：与脚本自身三轮重试叠加最坏可达 12 次尝试，
#      对一道 4.8 小时题成本与归因均失控；重试策略统一由本脚本承担。
import json
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
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

# 前置预检：hub 的 TLS 通路（鉴权换票所在）与模型端点。任一不可用即顺延。
HUB_PROBE_URL = 'https://hub.harborframework.com/datasets'
MODEL_PROBE_URL = 'https://api.deepseek.com/'
PREFLIGHT_TIMEOUT_S = 20

# 鉴权类失败签名：出现在某一 pass 的控制台日志里即判为「前置类失败」，
# 不消耗剩余 pass（环境问题而非框架/模型问题）。
AUTH_FAILURE_SIGNATURES = (
    'AuthenticationError',
    'API-key exchange request failed',
    'NotAuthenticatedError',
)

# 退出码口径：0=完成；1=未闭合（试次用尽）；2=前置不可用（顺延，未消耗 pass）。
EXIT_DONE = 0
EXIT_UNRESOLVED = 1
EXIT_DEFERRED = 2


def log(msg: str) -> None:
    with open(SUMMARY, 'a', encoding='utf-8') as f:
        f.write(msg + '\n')
    print(msg, flush=True)


def probe(url: str) -> tuple[bool, str]:
    """TLS + HTTP 可达性预检。拿到任何 HTTP 状态码（含 401/403/404）即视为通路可用。"""
    req = urllib.request.Request(
        url, method='HEAD', headers={'User-Agent': 'orz-probe-preflight'})
    try:
        with urllib.request.urlopen(req, timeout=PREFLIGHT_TIMEOUT_S) as resp:
            return True, f'HTTP {resp.status}'
    except urllib.error.HTTPError as exc:
        return True, f'HTTP {exc.code} (reachable)'
    except Exception as exc:  # URLError / ssl.SSLError / timeout 等
        return False, f'{type(exc).__name__}: {exc}'


def preflight() -> bool:
    all_ok = True
    for label, url in (('harbor-hub', HUB_PROBE_URL), ('model-endpoint', MODEL_PROBE_URL)):
        ok, detail = probe(url)
        log(f'preflight {label}: {"OK" if ok else "FAIL"} ({detail})')
        all_ok = all_ok and ok
    return all_ok


def console_path(pass_no: int) -> Path:
    return JOBS_DIR / f'{JOB}-console-pass{pass_no}.log'


def console_has_auth_failure(pass_no: int) -> bool:
    try:
        text = console_path(pass_no).read_text(encoding='utf-8', errors='replace')
    except OSError:
        return False
    return any(sig in text for sig in AUTH_FAILURE_SIGNATURES)


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
    stats = r.get('stats') or {}
    # 出错试次 = 无效试次：环境/传输类中断以 reward 0.0 + exception_stats 落账，
    # 不得计入「批次完成」，否则会把一次环境问题写成一条有效试次结论。
    if (stats.get('n_errored_trials') or 0) > 0:
        return False
    evals = stats.get('evals') or {}
    if not evals:
        return False
    ev = next(iter(evals.values()))
    if (ev or {}).get('n_errors'):
        return False
    reward = ((ev or {}).get('reward_stats') or {}).get('reward') or {}
    has_reward = any(isinstance(v, list) and len(v) > 0 for v in reward.values())
    return has_reward and real_run_exists()


def invoke(pass_no: int) -> int:
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
        'run', '-t', f'terminal-bench/{TASK}', '-n', '1', '-r', '0',
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
    console = console_path(pass_no)  # 每 pass 独立，不再覆写
    with open(console, 'wb') as f:
        proc = subprocess.run([str(HARBOR), *args], stdout=f, stderr=subprocess.STDOUT, env=env)
    return proc.returncode


def main() -> int:
    log(f'== {time.strftime("%Y-%m-%d %H:%M:%S")} TB4.0 ctr-optimization friction probe start ==')
    log(f'task={TASK} (single-task resolution; dataset-level resolution unavailable) binary={BINARY} model={MODEL} k=1')
    if job_complete():
        log('SKIP (complete)')
        return EXIT_DONE
    if not preflight():
        log('PREFLIGHT FAILED — batch deferred, no pass consumed (rerun when the line is stable)')
        return EXIT_DEFERRED
    for pass_no in range(1, MAX_PASSES + 1):
        log(f'pass {pass_no}')
        code = invoke(pass_no)
        if code == 0 and job_complete():
            log(f'DONE exit={code}')
            return EXIT_DONE
        log(f'FAIL exit={code}')
        if console_has_auth_failure(pass_no):
            log(f'AUTH FAILURE in pass {pass_no} — aborting batch; remaining passes not consumed')
            return EXIT_DEFERRED
    log('== finished; unresolved (invalid trials exhausted) ==')
    return EXIT_UNRESOLVED


if __name__ == '__main__':
    sys.exit(main())
