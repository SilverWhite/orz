#!/usr/bin/env python3
"""错题解剖（只读）：9 道 0 分题的轨迹与判分细节。"""
from __future__ import annotations
import json
from pathlib import Path
from collections import Counter

JOBS = Path('D:/tb-eval/jobs-official')
VOLS = Path('D:/tb-eval/gsa-volumes')

FAILED = [
    'official-v41-b1-01-build-pov-ray',
    'official-v41-b1-16-git-multibranch',
    'official-v41-b2-11-tune-mjcf',
    'official-v41-b2-14-fix-code-vulnerability',
    'official-v41-b2-15-gpt2-codegolf',
    'official-v41-b3-03-winning-avg-corewars',
    'official-v41-b3-06-extract-moves-from-video',
    'official-v41-b3-09-pytorch-model-recovery',
    'official-v41-b3-12-torch-pipeline-parallelism',
]

for job in FAILED:
    jd = JOBS / job
    task = job.split('-', 4)[-1].replace('-', '-', 0)
    name = job.split('official-v41-')[1].split('-', 2)[2]
    print('=' * 72)
    print(f'## {name}')
    # per-trial result
    for rp in jd.rglob('result.json'):
        d = json.loads(rp.read_text(encoding='utf-8'))
        if rp.parent == jd:
            continue
        st, fi = d.get('started_at'), d.get('finished_at')
        dur = ''
        if st and fi:
            from datetime import datetime
            t0 = datetime.fromisoformat(st.replace('Z', '+00:00'))
            t1 = datetime.fromisoformat(fi.replace('Z', '+00:00'))
            dur = f'{(t1 - t0).total_seconds():.0f}s'
        vr = d.get('verifier_result') or {}
        vstd = (vr.get('stdout') or '')[-300:].replace('\n', ' | ')
        print(f'  duration: {dur} | reward: {vr.get("rewards", {}).get("reward")}')
        ei = d.get('exception_info')
        if ei:
            print(f'  exception: {ei.get("exception_type")}')
        if vstd:
            print(f'  verifier tail: {vstd[:280]}')
    # journal
    vol = VOLS / job
    runs = sorted(vol.glob('*/runs/*/events.jsonl'))
    if not runs:
        print('  (no journal)')
        continue
    lines = runs[-1].read_text(encoding='utf-8').strip().splitlines()
    tools: Counter = Counter()
    fails: Counter = Counter()
    outputs: list[str] = []
    compress = 0
    for ln in lines:
        try:
            e = json.loads(ln)
        except ValueError:
            continue
        et = e.get('event_type')
        pl = e.get('payload') or {}
        if et == 'tool_started':
            tools[pl.get('tool') or pl.get('tool_name') or '?'] += 1
        elif et in ('tool_completed',):
            if pl.get('status') not in (None, 'ok', 'success'):
                fails[str(pl.get('status'))[:40]] += 1
        elif et == 'model_output':
            txt = (pl.get('message') or pl.get('content') or '')
            if isinstance(txt, str) and txt.strip():
                outputs.append(txt)
        elif et == 'context_compressed':
            compress += 1
    print(f'  journal: {len(lines)} events | compressions: {compress}')
    print(f'  tools: {dict(tools.most_common(8))}')
    if fails:
        print(f'  tool-fail statuses: {dict(fails.most_common(5))}')
    if outputs:
        tail = outputs[-1].replace('\n', ' ')[:400]
        print(f'  final model_output tail: {tail}')
    print()
