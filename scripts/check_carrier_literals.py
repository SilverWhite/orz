#!/usr/bin/env python3
"""Carrier literal verification (字面量核证) — byte search of key literals in
carrier binaries, old→new per platform (历次载体重建批同口径；170 批 §2-A 注记：
Windows 侧短字面量可被 LLVM 以 movabs imm64 内联拆开，连续字节搜索记 0 属已知
形态——判据以 key 串＋键形＋长文案为面）。

Usage:
    python scripts/check_carrier_literals.py <bin> [<bin> ...]

Literals are grouped EXPECTED_NEW (0.8.12 窗口新增面：0am P8-a/P8-b/178/179)、
EXPECTED_RETAINED（保留面）、EXPECTED_GONE（退役面——应为 0）。
"""

from __future__ import annotations

import sys

# 0.8.12 窗口新增面（源冻结 54717d06＝179 批 905bc3f5＋bump；基线 0.8.11 冻结于
# 6493fdae＝164+168+169，P8-a/P8-b/178/179 全部为本窗新增）。
EXPECTED_NEW = [
    # 0am P8-a：快照 schema v2、八通道锚点名
    "rli-shadow-v2",
    "u_verify",
    "v_verify",
    "u_ctx",
    "v_ctx",
    "u_infra",
    "v_infra",
    # 0am P8-b/178：成因段标签环词（闭集词表全量）＋源前缀＋bus 标签
    "源：",
    "验证失败",
    "写控拦截",
    "计划/车道拒绝",
    "门/护栏拒绝",
    "检索启用拒绝",
    "权限拒绝",
    "策略拒绝",
    "其他拒绝",
    "变更成功",
    "间隔失节律",
    "折叠推进",
    "折叠写失败",
    "资源拒绝",
    "限额命中",
    "探针翻转",
    "资源跨档",
    "传输重试",
    # 178：验证词表 v2 新词＋首词守卫抽样
    "cargo fmt --check",
    "pnpm test",
    "bun test",
    # 179：kill-switch 告警长文案（长文案跨平台可见性优于短串）
    "0/false/no/off disable",
    "keeping the RLI channel family enabled",
]

# 保留面（170 批在件面＋既有面）。
EXPECTED_RETAINED = [
    "rli.notice.",
    "streak_crossed",
    "domain_spike_entry",
    "migration_confirmed",
    "coverage_gap",
    "ORZ_LIF_RLI_SHADOW",
    "u_prog",
    "slow_prog",
    "carrier-write",
]

# 退役面（应为 0；`预算：` 170 批已退役、`rli-shadow-v1` 随 schema v2 退役）。
EXPECTED_GONE = [
    "预算：",
    "rli-shadow-v1",
]


def count(data: bytes, needle: str) -> int:
    return data.count(needle.encode("utf-8"))


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    rc = 0
    for path in sys.argv[1:]:
        with open(path, "rb") as fh:
            data = fh.read()
        print(f"== {path} ({len(data)} B) ==")
        for group, literals in (
            ("NEW", EXPECTED_NEW),
            ("RETAINED", EXPECTED_RETAINED),
            ("GONE(=0)", EXPECTED_GONE),
        ):
            row = []
            for lit in literals:
                n = count(data, lit)
                mark = ""
                if group == "GONE(=0)" and n != 0:
                    mark = "  <-- UNEXPECTED"
                    rc = 1
                if group == "NEW" and n == 0:
                    mark = "  <-- zero (check imm64 inlining / face list)"
                row.append(f"{lit!r}={n}{mark}")
            print(f"  [{group}] " + " ".join(row))
    return rc


if __name__ == "__main__":
    sys.exit(main())
