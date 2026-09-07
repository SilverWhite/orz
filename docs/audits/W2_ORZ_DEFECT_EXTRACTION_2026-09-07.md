# W2 批次 orz 框架缺陷提取（口径修正版，2026-09-07）

> **口径**（用户 2026-09-07 裁定）：high-nist 是**压测**，目的是暴露
> **orz 自身框架的缺陷**；模型侧失误（POSIX 习惯、命令写错等）是墙在
> 正常工作，**不计入** orz 缺陷；orz 不做任何任务特化。本文件对
> chunk1+chunk2 六个 run 的 journal 按此口径重新提取，取代
> [`chunk1 分析 §3`](W2_CHUNK1_031_S4_ANALYSIS_2026-09-07.md) 中把
> 模型侧摩擦列为证据的口径。
> **范围**：仅 orz 框架层（工具面/权限面/事件面/内部状态边界/预算面）；
> harness（评测驱动脚本）缺陷另见 `173ade6`/`db71fb6`/`5b45598`。

## 1. 框架缺陷清单

### D-1（中）`.gsa` agent-invisible 边界的 shell 旁路

- **事实**：6 run 合计 **31 条 shell 命令触碰 `.gsa`，全部 exit 0**——
  make-doom `Get-Content 'C:\app\.gsa\resources_state.json'`（orz 内部
  资源状态）与 `Get-Content '.gsa\ledger\current.md'`（机械审计台账）
  读取成功；train-fasttext 18 条（含自写 python 解析 ledger、findstr
  检索 ledger、遍历 session/terminal）。
- **对照设计**：AUTH-GSA-SESSION-VOLUME（ADR-0010 §14.56）「`.gsa`
  默认 agent-invisible、仅 terminal-log/run_tests 两只读窗口」——工具
  层三分判定有效（12 次 read_file/grep 触 `.gsa` 全部正确拒绝，无误
  杀），但 **run_terminal_cmd 通道无任何拦截**：目录可见（`ls -Force`
  即见结构）→ 内容可读（shell 直读）。
- **影响**：审计链反博弈面失效——模型可读自身机械审计台账与资源状态
  （本批模型确实读了，并在 ledger 缺失线索时反复重试 8 次）；压测
  语义下，模型经由框架自身内部状态可获得非预期信息面。
- **处置**：源冻结期仅登记。修法属 orz 设计决策（命令串内容检查脆弱、
  `.gsa` 卷 ACL 化/移出 cwd 可达域、或设计上明确接受 shell 可见并在
  文档标注），解冻后随 0m 立项裁决。

### D-2（低）`run_finished.turn_count` 硬编码常量 1

- **事实**：4 个完成 run 的 `turn_count` 全为 1，与 `tool_rounds`
  （97/17/269/40）无关；源码 `controller.rs:3541` 该字段为硬编码字面
  量，非运行时计数。
- **影响**：单提示 run 碰巧正确；GAP-CONVERSATION-RESTORE 的跨 prompt
  会话语义下失真（多轮会话仍报 1）。事件面消费者不可用作真实轮次。
- **处置**：解冻后改为真实会话轮计数或字段退役（schema 语义核对）。

### D-3（低）权限门拒绝的结果信封不入事件面

- **事实**：deny 决定后**无任何** tool_started/tool_completed/
  policy_denial 事件——模型实际收到的拒绝文案/原因在 journal 不可
  审计，只剩薄 `permission_decision`（tool+decision 两字段）。
- **对照设计**：FUS-CONSOLE-POLICY-DENIAL 的结构化拒绝
  （`{source, code, reason}`）与 ToolCompleted 可选 `policy_denial`
  字段在权限门路径未落事件面。
- **影响**：压测分析无法区分「拒绝原因清晰使模型一次转向」与「拒绝
  无解释致模型反复重试」（train-fasttext 同一 ledger 文件 8 次重试
  的归因不可判定）。
- **处置**：解冻后评估 deny 结果信封（含 reason 脱敏）入 journal。

### D-4（观察，已登记）key 经 env 对 shell 子进程可见

模型可持 `$env:ORZ_DEEPSEEK_API_KEY` 自调 API（chunk1 mteb 实证，
见 [`v4-pro 路由判定 §3`](W2_PRO_MODEL_ROUTING_VERDICT_2026-09-07.md)）。
防除路径（子进程 env 剥离）解冻后立项。

### D-5（观察）tool_running/自动后台化真实模型面零触发

6/6 任务无 ≥180s 前台命令（最长 34s），机制面已由批次 W1 fake 覆盖，
真实面悬空。是否保留该真实面判据随 chunk3 任务形态评估。

## 2. 复核为「非缺陷」的项

- 12 次 read_file/grep deny：全部针对 `.gsa` 内部面（ledger/
  resources_state），三分判定**正确**执行，无误杀。
- dep_graph 4 条 write 边 consumed_read=null：均为新建文件创建语义，
  正确；read 边 58 条 anchor 齐全。
- `run_invalidated(wallclock)`、F6 cue 阈值语义（remaining<600s）、
  header 稳定性（单次 initial change）、缓存命中率 96.8%、零
  400/哨兵/retry、orientation/counterexample 各触发 1 次：均按设计
  工作。
- `submit` 两阶段（train-fasttext requested→confirmed）、交付流
  （adaptive final-answer 收尾）正常。
