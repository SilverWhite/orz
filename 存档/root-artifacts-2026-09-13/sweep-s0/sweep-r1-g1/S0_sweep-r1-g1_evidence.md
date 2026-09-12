# S0 证据门原始采集 — sweep-r1-g1

生成时间：2026-08-21 05:41:01 +08:00

| 题目 | 哨兵 | 连续 | 降档 | 触发时刻 | 持续s | 轮次 | 前3事件 | 紧邻bb | bb基率(紧邻/前3) | 折叠后首请求 | 截断后 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| dna-assembly | reasoning_stall | 1 | EnabledLow | 2026-08-20T19:16:49.792Z | 362 |  |  | - | 0/0 | False | False |
| dna-assembly | reasoning_repetition | 1 | EnabledLow | 2026-08-20T19:20:57.355Z |  |  |  | - | 0/0 | False | False |
| dna-assembly | reasoning_repetition | 2 | Disabled | 2026-08-20T19:21:34.421Z |  |  |  | - | 0/0 | False | False |
| llm-inference-batching-scheduler | reasoning_stall | 1 | EnabledLow | 2026-08-20T19:47:21.092Z | 381 |  |  | - | 0/0 | False | False |
| schemelike-metacircular-eval | reasoning_stall | 1 | EnabledLow | 2026-08-20T18:52:05.205Z | 308 | 34 | model_output; tool_started:blackboard_read; tool_completed:blackboard_read(actions) | Y | 0.36/0.517 | False | False |
| schemelike-metacircular-eval | reasoning_stall | 1 | EnabledLow | 2026-08-20T18:58:12.606Z | 303 | 47 | model_output; tool_started:blackboard_read; tool_completed:blackboard_read(actions) | Y | 0.36/0.517 | False | False |
