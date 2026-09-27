# 0ac S4 读数提取器（s4-0ac-retrieval3，2026-09-27）
# 判据（0ac 索引条目 / 设计稿 §4.3 / §10.4）：
#   ① 检索类首个结果 wall_ms：p99 ≤ 10 000 ms（本地车道；provider 合成车道单独列、不计入判据），
#     本地引擎链整体兜底 30 000 ms；⑦ 排队（retrieval_progress stage=progress / queued）单独列。
#   ② `subagent_wallclock_timeout_mid_tool` = 0（全卷扫描）。
#   ③ 等待路径零事件 = 0：每个检索族 tool_started 必须有同 call_id 的 tool_completed（稳定码或结果）。
# 附：FR-D02（max_wallclock 传递读数）、旗标对账（TB21_RUN_FLAG_CHECKLIST §二）、
#     稳定码计数（capability_unreachable / network_no_response / empty_result）、引擎链 engine_attempts、
#     reward 面（S4 不评正式分，仅记录）、token 用量（成本估算用）。
import json
import sys
from pathlib import Path

VOL_ROOT = Path('D:/tb-eval/gsa-volumes')
PREFIX = 's4-0ac-retrieval3'
TASKS = ['mteb-leaderboard', 'count-dataset-tokens', 'model-extraction-relu-logits']

RETRIEVAL_TOOLS = {'web_search', 'web_fetch', 'browser_read', 'browser_control'}


def task_run_paths(task: str):
    vol = VOL_ROOT / f'{PREFIX}-{task}'
    return sorted(vol.glob('*/runs/RUN-CLI-*/events.jsonl'))


def analyze_task(task: str) -> dict:
    out = {'task': task, 'runs': []}
    for jp in task_run_paths(task):
        run = {'journal': str(jp), 'events': 0}
        started, completed = {}, {}
        wall_local, wall_provider = [], []
        local_over30 = []
        codes = {'capability_unreachable': 0, 'network_no_response': 0,
                 'empty_result': 0, 'other_cause': 0}
        mid_tool = 0
        progress_events = 0
        delivered = 0
        wallclock_flag = None
        retrieval_enabled = None
        engines = {}
        tokens = {'input': 0, 'output': 0, 'cached': 0}
        for line in jp.read_text(encoding='utf-8', errors='replace').splitlines():
            if not line.strip():
                continue
            run['events'] += 1
            try:
                e = json.loads(line)
            except ValueError:
                continue
            k = e.get('event_type', '')
            pl = e.get('payload', {}) or {}
            raw = line
            if k == 'tool_started':
                t = pl.get('tool')
                if t in RETRIEVAL_TOOLS:
                    started[pl.get('call_id')] = t
            elif k == 'tool_completed':
                t = pl.get('tool')
                cid = pl.get('call_id')
                if t in RETRIEVAL_TOOLS:
                    completed[cid] = t
                if t in RETRIEVAL_TOOLS and cid in started:
                    wm = pl.get('wall_ms')
                    cause = pl.get('cause') or ''
                    if isinstance(wm, (int, float)):
                        # 车道归类：本地分段车道结果带 encoding/引擎注记或 short wall；
                        # provider 合成典型 ≥15s。此处按 wall 阈与 target 归类只作读数，判据另算。
                        wall_local.append(int(wm))
                        if wm > 30000:
                            local_over30.append(int(wm))
                    for c in codes:
                        if c in raw:
                            codes[c] += 1
                            break
            elif k == 'retrieval_progress':
                progress_events += 1
            elif k == 'result_delivered':
                delivered += 1
            elif k == 'subagent_wallclock_timeout_mid_tool':
                mid_tool += 1
            elif k == 'run_started':
                if 'max_wallclock' in raw or 'wallclock' in raw:
                    wallclock_flag = pl.get('max_wallclock') or pl.get('wallclock') or 'present'
            elif k == 'tool_availability_check':
                if 'retrieval' in raw:
                    retrieval_enabled = True
            elif k == 'run_finished':
                usage = pl.get('usage') or pl.get('model_usage') or {}
                if isinstance(usage, dict):
                    tokens['input'] += usage.get('prompt_tokens', 0) or 0
                    tokens['output'] += usage.get('completion_tokens', 0) or 0
            elif k == 'model_usage':
                tokens['input'] += pl.get('prompt_tokens', 0) or 0
                tokens['cached'] += pl.get('prompt_cache_hit_tokens', 0) or 0
                tokens['output'] += pl.get('completion_tokens', 0) or 0
        # ③ 等待路径零事件：started 无 completed
        unmatched = [cid for cid in started if cid not in completed]
        ws = sorted(wall_local)
        def pct(p):
            if not ws:
                return None
            i = max(0, min(len(ws) - 1, round(p / 100 * len(ws)) - 1))
            return ws[i]
        run.update({
            'retrieval_started': len(started),
            'retrieval_completed': len([c for c in completed if c in started]),
            'zero_event_waits': len(unmatched),
            'wall_ms_list': ws,
            'wall_p50': pct(50), 'wall_p99': pct(99), 'wall_max': ws[-1] if ws else None,
            'local_over_30s': local_over30,
            'codes': codes,
            'subagent_wallclock_timeout_mid_tool': mid_tool,
            'retrieval_progress_events': progress_events,
            'result_delivered': delivered,
            'max_wallclock_flag': wallclock_flag,
            'retrieval_probe_present': retrieval_enabled,
            'tokens': tokens,
        })
        out['runs'].append(run)
    return out


def main():
    for task in TASKS:
        r = analyze_task(task)
        print(f"== {task} ==")
        for run in r['runs']:
            print(f"  journal: {run['journal']}  events={run['events']}")
            print(f"  retrieval started/completed={run['retrieval_started']}/{run['retrieval_completed']}"
                  f"  zero_event_waits={run['zero_event_waits']} (判据③=0)")
            print(f"  wall_ms n={len(run['wall_ms_list'])} p50={run['wall_p50']} p99={run['wall_p99']}"
                  f" max={run['wall_max']} over30s={run['local_over_30s']}")
            print(f"  codes={run['codes']}")
            print(f"  subagent_wallclock_timeout_mid_tool={run['subagent_wallclock_timeout_mid_tool']} (判据②=0)")
            print(f"  progress_events={run['retrieval_progress_events']} result_delivered={run['result_delivered']}")
            print(f"  max_wallclock_flag={run['max_wallclock_flag']} retrieval_probe={run['retrieval_probe_present']}")
            print(f"  tokens={run['tokens']}")
        print()


if __name__ == '__main__':
    main()
