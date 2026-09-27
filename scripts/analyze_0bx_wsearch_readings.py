# 0bx S4 真机承接读数提取器（2026-09-27）
#
# 判据（TODO P1-0bx）：agent 级采样实证 `web_search` 链首**承接**（不再让渡），
# 或如实让渡于真不可用。
#
# 机械读数面：
#   ① runs/RUN-CLI-*/serp-attempts/*.json —— 引擎级车道归属
#      （lane / envelope.engine / envelope.engine_attempts[] / browser_type）；
#   ② journal `browser_launch_result` 条数 vs 浏览器调用
#      （`browser_read`／`browser_control`）条数——修复前＝逐调用各落一条；
#      承接＝首次之后不再重复启动；
#   ③ 让渡痕迹：`browser_unavailable`／`local_http` 出现次数（承接＝0）。
# 附：`web_search` 起讫配对、检索结果 committed 的来源类型分布。
import json
import sys
from pathlib import Path

BROWSER_TOOLS = {'browser_read', 'browser_control'}


def analyze_run(run_dir: Path) -> dict:
    jp = run_dir / 'events.jsonl'
    out = {
        'run': run_dir.name,
        'journal': str(jp),
        'events': 0,
        'prompt_head': None,
        'launch_results': [],
        'browser_calls': 0,
        'browser_calls_ok': 0,
        'web_search_started': 0,
        'web_search_completed': 0,
        'browser_unavailable_hits': 0,
        'local_http_hits': 0,
        'result_source_types': {},
        'serp_attempts': [],
        'finished': False,
    }
    text = jp.read_text(encoding='utf-8', errors='replace')
    for line in text.splitlines():
        if not line.strip():
            continue
        out['events'] += 1
        try:
            e = json.loads(line)
        except ValueError:
            continue
        k = e.get('event_type', '')
        pl = e.get('payload', {}) or {}
        if k == 'run_started':
            p = pl.get('prompt')
            if isinstance(p, str):
                out['prompt_head'] = p.strip().replace('\n', ' ')[:90]
        elif k == 'browser_launch_result':
            out['launch_results'].append({
                'attempt_id': pl.get('attempt_id'),
                'status': pl.get('status'),
                'cause': pl.get('cause'),
                'seq': e.get('sequence'),
            })
        elif k == 'tool_started' and pl.get('tool') == 'web_search':
            out['web_search_started'] += 1
        elif k == 'tool_completed' and pl.get('tool') == 'web_search':
            out['web_search_completed'] += 1
        if k in ('tool_started', 'tool_completed') and pl.get('tool') in BROWSER_TOOLS:
            if k == 'tool_started':
                out['browser_calls'] += 1
            elif pl.get('exit_code') == 0:
                out['browser_calls_ok'] += 1
        if k == 'retrieval_result_committed':
            ledger = pl.get('source_ledger') or []
            for s in ledger:
                t = s.get('source_type') or 'unknown'
                out['result_source_types'][t] = out['result_source_types'].get(t, 0) + 1
    out['browser_unavailable_hits'] = text.count('browser_unavailable')
    out['local_http_hits'] = text.count('local_http')
    for f in sorted((run_dir / 'serp-attempts').glob('*.json')):
        try:
            a = json.loads(f.read_text(encoding='utf-8'))
        except (OSError, ValueError):
            continue
        env = a.get('envelope') or {}
        attempts = env.get('engine_attempts') or []
        first_ok = next((x for x in attempts if x.get('status') == 'ok'), None)
        out['serp_attempts'].append({
            'file': f.name,
            'tool_round': a.get('tool_round'),
            'lane': a.get('lane'),
            'engine': env.get('engine'),
            'browser_type': env.get('browser_type'),
            'action_status': env.get('action_status'),
            'error_class': env.get('error_class'),
            'results_count': a.get('results_count'),
            'first_ok_engine': (first_ok or {}).get('engine'),
            'first_ok_wall_ms': (first_ok or {}).get('wall_ms'),
            'engine_attempts': [(x.get('engine'), x.get('status')) for x in attempts],
        })
    return out


def main() -> int:
    if len(sys.argv) < 2:
        print('usage: analyze_0bx_wsearch_readings.py <runs-root> [<runs-root> ...]')
        return 2
    rc = 0
    for root_arg in sys.argv[1:]:
        root = Path(root_arg)
        runs = sorted(root.glob('*/runs/RUN-CLI-*')) + sorted(root.glob('RUN-CLI-*'))
        print(f'== root {root}  runs={len(runs)} ==')
        for rd in runs:
            if not (rd / 'events.jsonl').is_file():
                continue
            r = analyze_run(rd)
            print(f"-- {r['run']}  events={r['events']}")
            if r['prompt_head']:
                print(f"   prompt: {r['prompt_head']}")
            print(f"   browser_launch_result n={len(r['launch_results'])}"
                  f" statuses={[x['status'] for x in r['launch_results']]}"
                  f" seqs={[x['seq'] for x in r['launch_results']]}")
            print(f"   browser calls (started/ok)={r['browser_calls']}/{r['browser_calls_ok']}"
                  f"  web_search (started/completed)={r['web_search_started']}/{r['web_search_completed']}")
            print(f"   fallback traces: browser_unavailable={r['browser_unavailable_hits']}"
                  f" local_http={r['local_http_hits']}  (承接判据=0)")
            print(f"   serp_attempts n={len(r['serp_attempts'])}")
            for a in r['serp_attempts']:
                print(f"     - {a['file']} round={a['tool_round']} lane={a['lane']}"
                      f" engine={a['engine']} browser={a['browser_type']}"
                      f" status={a['action_status']} err={a['error_class']}"
                      f" results={a['results_count']}"
                      f" first_ok={a['first_ok_engine']}@{a['first_ok_wall_ms']}ms")
            print(f"   committed source types: {r['result_source_types']}")
        print()
    return rc


if __name__ == '__main__':
    sys.exit(main())
