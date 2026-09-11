# 0x / 0v S4 实机复验 — journal 分析器（2026-09-11）
#
# 用途：对 gsa 卷里落下的 run journal 做机械对账，输出两份判据的取证表：
#   0x（初始轮中立问询）：
#     - `orientation_checkpoint` 事件逐条列出（trigger / injection_position /
#       completed_turns_since_orientation / message_block 前缀）
#     - `trigger=initial_round` 恰好 1 条；位置 = post_tool_batch_gap；
#       块文本与设计常量逐字一致；动作中不复发（无第二条、模型输出无复读）
#     - 周期问询（completed_turns_interval）独立触发、间隔成序
#     - 终答前机械审查（mechanical_audit_update）不含三问
#   0v（检索引擎 SERP）：
#     - tool_started / tool_completed 按工具与车道统计
#     - `browser_control` 调用明细（action/url，取自 model_output.tool_calls）
#     - `retrieval_role_write_denied` 计数（期望 0）
#     - 预算/底线拒绝码、`browser_launch_result`
#     - Chrome profile History 里的实际访问 URL（Bing setmkt=en-US 等）
#
# 用法：python scripts/analyze_0x_0v_s4_journal.py <gsa-job-dir> [<gsa-job-dir> ...]
import json
import sqlite3
import sys
from collections import Counter
from pathlib import Path

INITIAL_PREFIX = "[INITIAL_ROUND_INQUIRY v0.1]"
PERIODIC_PREFIX = "[ORIENTATION"
THREE_QUESTIONS = [
    "本任务实际要交付什么",
    "大方向是什么",
    "当前做法优劣如何",
]
DENIAL_CODES = [
    "retrieval_role_write_denied",
    "browser_control_search_budget_exceeded",
    "browser_control_search_session_reserved",
]


def load_events(journal: Path) -> list[dict]:
    return [json.loads(l) for l in journal.read_text(encoding="utf-8").splitlines() if l.strip()]


def find_journals(root: Path) -> list[Path]:
    if root.is_file():
        return [root]
    return sorted(root.glob("*/runs/*/events.jsonl")) or sorted(root.glob("runs/*/events.jsonl"))


def section_0x(events: list[dict], out: list[str]) -> None:
    out.append("### 0x 初始轮中立问询")
    ori = [e for e in events if e.get("event_type") == "orientation_checkpoint"]
    if not ori:
        out.append("- **无 orientation_checkpoint 事件**（本轮未触发任何中立问询）")
        return
    initial = []
    periodic = []
    for e in ori:
        pl = e["payload"]
        row = (e.get("sequence"), pl.get("trigger"), pl.get("injection_position"),
               pl.get("agent_role"), pl.get("completed_turns_since_orientation"),
               (pl.get("message_block") or "")[:32].replace("\n", "\\n"))
        (initial if pl.get("trigger") == "initial_round" else periodic).append(row)
        out.append(f"- seq={row[0]} trigger={row[1]} pos={row[2]} role={row[3]} "
                   f"turns={row[4]} block={row[5]!r}")
    out.append(f"- initial_round 条数 = **{len(initial)}**（判据：恰好 1）")
    if len(initial) == 1:
        e = next(x for x in ori if x["payload"].get("trigger") == "initial_round")
        pl = e["payload"]
        first_tool = next((x.get("sequence") for x in events
                           if x.get("event_type") == "tool_completed"), None)
        out.append(f"  - 位置 = {pl.get('injection_position')}（判据：post_tool_batch_gap）")
        out.append(f"  - 首个 tool_completed seq={first_tool} → fire seq={e.get('sequence')} "
                   f"（判据：fire 在首轮动作批次之后）")
        exact = pl.get("message_block", "").startswith(INITIAL_PREFIX)
        qhit = sum(1 for q in THREE_QUESTIONS if q in pl.get("message_block", ""))
        out.append(f"  - 块前缀命中 = {exact}；三问命中 = {qhit}/3")
        out.append(f"  - completed_turns_since_orientation = "
                   f"{pl.get('completed_turns_since_orientation')}")
    elif len(initial) > 1:
        out.append("  - **超标：动作中出现复发**")
    out.append(f"- 周期问询条数 = {len(periodic)}")
    for r in periodic:
        out.append(f"  - seq={r[0]} pos={r[2]} turns={r[4]}")
    audits = [e for e in events if e.get("event_type") == "mechanical_audit_update"]
    leak = [e.get("sequence") for e in audits
            if any(q in json.dumps(e.get("payload", {}), ensure_ascii=False) for q in THREE_QUESTIONS)]
    out.append(f"- 机械审查事件 {len(audits)} 条，含三问的 = {leak or '无'}（判据：无）")


def section_0v(events: list[dict], out: list[str]) -> None:
    out.append("### 0v 检索引擎 SERP")
    started = Counter()
    denied = Counter()
    role = "main"
    for e in events:
        pl = e.get("payload", {})
        if e.get("event_type") == "request_header_change":
            role = pl.get("agent_role") or role
            continue
        if e.get("event_type") == "tool_started":
            # 子代理调用不带 target 字段 → 用 request_header_change 推定的角色标注
            started[(pl.get("tool"), pl.get("target") or role)] += 1
        if e.get("event_type") == "tool_completed":
            for code in DENIAL_CODES:
                if pl.get("error") == code:
                    denied[code] += 1
    out.append("- tool_started（工具, 车道/角色）：")
    for (tool, lane), n in sorted(started.items()):
        out.append(f"  - {tool} @ {lane}: {n}")
    out.append(f"- 拒绝码计数：{dict(denied) or '无'}"
               f"（判据：retrieval_role_write_denied = 0）")
    bc = []
    for e in events:
        if e.get("event_type") != "model_output":
            continue
        for c in e["payload"].get("tool_calls") or []:
            if c.get("name") == "browser_control":
                a = c.get("arguments") or {}
                bc.append((e.get("sequence"), a.get("action"), a.get("url") or a.get("query")))
    out.append(f"- browser_control 调用明细（{len(bc)} 次）：")
    for seq, action, arg in bc:
        out.append(f"  - seq={seq} action={action} arg={str(arg)[:140]}")
    out.append("- 检索类工具完成统计（工具: 次数 / exit!=0 / wall_ms 均值·最大）：")
    stats: dict[str, list[int]] = {}
    errs = Counter()
    for e in events:
        if e.get("event_type") != "tool_completed":
            continue
        pl = e["payload"]
        tool = pl.get("tool") or "?"
        if tool not in ("browser_control", "web_search", "web_fetch", "browser_read"):
            continue
        stats.setdefault(tool, []).append(int(pl.get("wall_ms") or 0))
        if pl.get("exit_code") not in (0, None):
            errs[tool] += 1
    for tool, walls in sorted(stats.items()):
        out.append(f"  - {tool}: n={len(walls)} err={errs.get(tool, 0)} "
                   f"wall_avg={sum(walls) // max(1, len(walls))}ms max={max(walls)}ms")
    hit = sum(e["payload"].get("cache_hit_tokens") or 0
              for e in events if e.get("event_type") == "model_output")
    miss = sum(e["payload"].get("cache_miss_tokens") or 0
               for e in events if e.get("event_type") == "model_output")
    if hit + miss:
        out.append(f"- 模型缓存命中率 = {hit / (hit + miss) * 100:.2f}%"
                   f"（hit={hit} miss={miss}）")
    b400 = []
    for e in events:
        pl = e.get("payload", {})
        probe = " ".join(str(pl.get(k) or "") for k in ("error", "status", "finish_reason", "reason"))
        if "400" in probe:
            b400.append(e.get("sequence"))
    out.append(f"- HTTP 400 迹象（error/status/reason 字段）seq = {b400 or '无'}")
    for e in events:
        if e.get("event_type") == "browser_launch_result":
            out.append(f"- browser_launch_result: {json.dumps(e['payload'], ensure_ascii=False)}")


def section_chrome(journal: Path, out: list[str]) -> None:
    out.append("### Chrome profile 实际访问（判据 1/6 的 URL 面证据）")
    # journal = <vol>/<uuid>/runs/<RUN>/events.jsonl → <uuid> = parents[2]
    run_root = journal.parents[2] if len(journal.parents) > 2 else journal.parent
    hist = sorted(run_root.glob("chrome-profile-*/Default/History"))
    if not hist:
        out.append("- 未找到 chrome-profile History")
        return
    for h in hist:
        try:
            con = sqlite3.connect(f"file:{h}?immutable=1", uri=True)
            rows = con.execute(
                "select url, title from urls order by last_visit_time"
            ).fetchall()
            con.close()
        except sqlite3.Error as exc:
            out.append(f"- {h.name}: 读取失败 {exc}")
            continue
        out.append(f"- {h} 命中 {len(rows)} 条 URL：")
        for url, title in rows:
            out.append(f"  - {(title or '')[:60]} | {url[:180]}")


def analyze(root: Path) -> str:
    out: list[str] = [f"## {root}"]
    journals = find_journals(root)
    if not journals:
        out.append("- 未找到 events.jsonl")
        return "\n".join(out)
    for j in journals:
        events = load_events(j)
        out.append(f"### journal {j}")
        types = Counter(e.get("event_type") for e in events)
        turns = types.get("model_output", 0)
        out.append(f"- 事件总数 {len(events)}；model_output（模型轮）= {turns}；"
                   f"tool_started = {types.get('tool_started', 0)}")
        finished = [e for e in events if e.get("event_type") == "run_finished"]
        out.append(f"- 终止事件 = {json.dumps(finished[0]['payload'], ensure_ascii=False) if finished else '无（墙钟耗尽/被杀）'}")
        section_0x(events, out)
        section_0v(events, out)
        section_chrome(j, out)
    return "\n".join(out)


def main() -> int:
    roots = [Path(a) for a in sys.argv[1:]]
    if not roots:
        print(__doc__)
        return 2
    for r in roots:
        print(analyze(r))
        print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
