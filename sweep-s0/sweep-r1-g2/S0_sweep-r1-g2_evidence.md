# S0 证据门原始采集 — sweep-r1-g2

生成时间：2026-08-21 06:41:03 +08:00

- 冻结二进制：orz SHA256 D42F33B85095076817891612263C47045F7C884138AEDCC3D494E9BDAEF98320
- job 日志起始：08/21/2026 06:40:23
- 口径：每对 WARN（stream interrupted + degrading）计 1 次哨兵触发；紧邻bb=触发轮次前 1 事件为 blackboard_read(session/actions)；基率为该 run 全部轮次中的占比。
- 说明：工具输出超长截断指针后 在持久化产物（events/orz/trajectory）中不可观测（截断指针只存在于工具输出正文，未落盘），以折叠/压缩事件（ledger_fold_advance/context_compressed）作为实际代理。

| 题目 | 哨兵 | 连续 | 降档 | 触发时刻 | 持续s | 轮次 | 前3事件 | 紧邻bb | bb基率(紧邻/前3) | 折叠后首请求 | 截断后 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| feal-differential-cryptanalysis | reasoning_stall | 1 | EnabledLow | 2026-08-20T22:00:12.547Z | 374 |  |  | - | 0/0 | False | False |
