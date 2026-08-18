#!/usr/bin/env python3
"""机械模板命中率探针（2026-08-18，诊断用，非评测运行）。

目的：直接对真实 provider（deepseek-v4-flash）测一组确定性请求模板，
量化各类视图变更在 DeepSeek 前缀缓存下的实际 cache_hit/cache_miss：

  S1 纯追加        —— 主循环请求形态：固定 system+tools，逐轮 append。
  S2 换系统提示词  —— 压缩摘要调用形态：summary system prompt + 相同视图
                       （loop 外辅助请求，journal 不可见）。
  S3 重复同请求    —— 摘要失败重试：相同输入再次发送。
  S4 中段替换      —— 折叠形态：中段历史替换为固定指针消息。
  S5 整段重写      —— 压缩形态：新 marker + 截断视图。

所有内容确定性生成，max_tokens 取极小值，总成本可忽略。
"""

import json
import time
import urllib.request
import urllib.error
from pathlib import Path

BASE_URL = "https://api.deepseek.com/chat/completions"
MODEL = "deepseek-v4-flash"
ENV_FILE = Path(r"D:\tb-eval\.env")

MAIN_SYS = (
    "You are Orz, a fusion-architecture coding agent. "
    "You plan first, then execute through the console action bar. "
    "Keep every reply minimal and deterministic."
)
SUMMARY_SYS = (
    "You are the context compaction summarizer. "
    "Compress the conversation history into the five template slots. "
    "Reply only with the slot JSON."
)
SUMMARY_USER = "Summarize the following history into the five template slots."
POINTER = "【历史折叠】更早轮次的机械摘要已外挂存档：/app/.gsa/ledger/current.md（本会话内固定）。"
MARKER = "[上下文已压缩] 早期轮次已归档；以下为保留的最近轮次。"

TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "read_file",
            "description": "Read a file with optional offset.",
            "parameters": {
                "type": "object",
                "properties": {
                    "target_file": {"type": "string"},
                    "offset": {"type": "integer"},
                },
                "required": ["target_file"],
            },
        },
    }
]


def load_key() -> str:
    for line in ENV_FILE.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line.startswith("ORZ_DEEPSEEK_API_KEY="):
            return line.split("=", 1)[1].strip()
    raise SystemExit("ORZ_DEEPSEEK_API_KEY not found in " + str(ENV_FILE))


def call(key: str, messages: list[dict], tools: list | None, label: str) -> dict:
    body = {
        "model": MODEL,
        "messages": messages,
        "max_tokens": 8,
        "stream": False,
    }
    if tools is not None:
        body["tools"] = tools
    req = urllib.request.Request(
        BASE_URL,
        data=json.dumps(body).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {key}",
        },
    )
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=90) as resp:
        data = json.loads(resp.read())
    ms = int((time.time() - t0) * 1000)
    usage = data.get("usage", {})
    hit = int(usage.get("prompt_cache_hit_tokens") or usage.get("cache_hit_tokens") or 0)
    miss = int(usage.get("prompt_cache_miss_tokens") or usage.get("cache_miss_tokens") or 0)
    total = int(usage.get("prompt_tokens") or 0)
    print(
        f"{label:<34} hit={hit:>8} miss={miss:>8} total={total:>8} "
        f"rate={100.0 * hit / (hit + miss) if hit + miss else 0:6.2f}% {ms:>6}ms"
    )
    return {"hit": hit, "miss": miss, "total": total, "ms": ms}


def rounds(n: int) -> list[dict]:
    """确定性轮次：user -> assistant -> tool result，每轮约 100-200 token。"""
    msgs: list[dict] = []
    for k in range(1, n + 1):
        msgs.append(
            {
                "role": "user",
                "content": f"Step {k}: inspect /app/doomgeneric/doomgeneric/{'a' * 40}.c",
            }
        )
        msgs.append(
            {
                "role": "assistant",
                "content": f"ack {k}",
                "tool_calls": [
                    {
                        "id": f"call_{k}",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": f'{{"target_file": "f{k}.c"}}',
                        },
                    }
                ],
            }
        )
        msgs.append(
            {
                "role": "tool",
                "tool_call_id": f"call_{k}",
                "content": f"result {k}: file has {200 + k * 17} lines; build target make-doom.",
            }
        )
    msgs.append({"role": "user", "content": "Continue the build plan."})
    return msgs


def main() -> None:
    key = load_key()
    print(f"provider={MODEL} base={BASE_URL}\n")

    # S1: 纯追加（主循环形态），R1..R7 逐步增长。
    print("--- S1 pure append (fixed system + tools) ---")
    s1 = {}
    last = None
    for k in range(1, 8):
        msgs = [{"role": "system", "content": MAIN_SYS}] + rounds(k)
        last = call(key, msgs, TOOLS, f"S1 R{k}")
        s1[k] = last

    # S3: 重复 S1 R7 同输入（摘要重试形态）。
    print("--- S3 repeat identical request ---")
    msgs = [{"role": "system", "content": MAIN_SYS}] + rounds(7)
    call(key, msgs, TOOLS, "S3 repeat 1/2")
    call(key, msgs, TOOLS, "S3 repeat 2/2")

    # S2: 换系统提示词（摘要调用形态：summary sys + 相同视图 + 空 tools）。
    print("--- S2 summary-style (different system prompt, same view) ---")
    view = rounds(7)
    msgs2 = [{"role": "system", "content": SUMMARY_SYS}] + [
        {"role": "user", "content": SUMMARY_USER}
    ] + view
    call(key, msgs2, None, "S2 summary attempt 1/3")
    call(key, msgs2, None, "S2 summary attempt 2/3")
    call(key, msgs2, None, "S2 summary attempt 3/3")

    # S4: 中段折叠替换（fold 形态）：中段 5 条历史换成固定指针。
    print("--- S4 fold-style (middle replaced by pointer) ---")
    view = rounds(7)
    folded = view[:3] + [{"role": "user", "content": POINTER}] + view[-4:]
    call(
        key,
        [{"role": "system", "content": MAIN_SYS}] + folded,
        TOOLS,
        "S4 folded view",
    )
    call(
        key,
        [{"role": "system", "content": MAIN_SYS}] + folded,
        TOOLS,
        "S4 folded repeat",
    )

    # S5: 压缩整段重写（compaction 形态：新 marker + 截断视图）。
    print("--- S5 compaction-style (marker + truncated view) ---")
    kept = rounds(3)
    comp = [{"role": "user", "content": MARKER}] + kept
    call(
        key,
        [{"role": "system", "content": MAIN_SYS}] + comp,
        TOOLS,
        "S5 compacted view",
    )
    call(
        key,
        [{"role": "system", "content": MAIN_SYS}] + comp,
        TOOLS,
        "S5 compacted repeat",
    )

    # 汇总：S1 R7 与各类变更的增量 miss。
    print("\n--- summary vs S1 R7 baseline ---")
    base_miss = s1[7]["miss"]
    print(f"S1 R7 baseline miss={base_miss} (last request of pure append)")


if __name__ == "__main__":
    main()
