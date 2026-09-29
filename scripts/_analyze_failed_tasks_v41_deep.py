#!/usr/bin/env python3
"""错题深挖（只读）：每题末段事件流＋错误样本＋最终输出。"""
from __future__ import annotations
import json
from pathlib import Path

VOLS = Path('D:/tb-eval/gsa-volumes')

TARGETS = {
    'build-pov-ray': 'official-v41-b1-01-build-pov-ray',
    'git-multibranch': 'official-v41-b1-16-git-multibranch',
    'tune-mjcf': 'official-v41-b2-11-tune-mjcf',
    'gpt2-codegolf': 'official-v41-b2-15-gpt2-codegolf',
    'winning-avg-corewars': 'official-v41-b3-03-winning-avg-corewars',
    'extract-moves-from-video': 'official-v41-b3-06-extract-moves-from-video',
    'pytorch-model-recovery': 'official-v41-b3-09-pytorch-model-recovery',
    'torch-pipeline-parallelism': 'official-v41-b3-12-torch-pipeline-parallelism',
}

for name, job in TARGETS.items():
    vol = VOLS / job
    runs = sorted(vol.glob('*/runs/*/events.jsonl'))
    if not runs:
        continue
    lines = runs[-1].read_text(encoding='utf-8').strip().splitlines()
    events = []
    for ln in lines:
        try:
            events.append(json.loads(ln))
        except ValueError:
            pass
    print('=' * 76)
    print(f'## {name}  ({len(events)} events)')
    # error samples
    errs = []
    outs = []
    for e in events:
        et = e.get('event_type')
        pl = e.get('payload') or {}
        if et == 'tool_completed':
            st = pl.get('status')
            if st and st not in ('ok', 'success'):
                msg = str(pl.get('error') or pl.get('message') or '')[:150]
                errs.append(f'{pl.get("tool")}: {msg}')
        elif et == 'model_output':
            txt = pl.get('message') or pl.get('content') or ''
            if isinstance(txt, str) and txt.strip():
                outs.append(txt)
    seen = set()
    for err in errs:
        key = err.split(':')[0] + err[-60:]
        if key in seen:
            continue
        seen.add(key)
        print(f'  [err] {err}')
        if len(seen) >= 4:
            break
    for o in outs[-2:]:
        print('  [out-tail]', o.replace('\n', ' ')[-450:])
    print('  [last-6-events]')
    for e in events[-6:]:
        pl = e.get('payload') or {}
        brief = {k: (str(v)[:90] if not isinstance(v, (int, float)) else v)
                 for k, v in pl.items() if k in ('tool', 'action', 'status', 'kind', 'reason', 'code', 'message', 'error', 'summary')}
        print(f'    {e.get("event_type")}: {json.dumps(brief, ensure_ascii=False)[:170]}')
    print()
