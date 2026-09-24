# 当前生产架构投影（Current Architecture Projection）

> **状态**：`current-design`（投影文件，不独立产生新设计）；**更新日期**：2026-09-24
> **设计权威**：[`adr/ADR-0010-fusion-runtime-and-agent-architecture.md`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（含 §14.52 会话黑板、§14.54 压缩快照、§14.55 工具执行层改革、§14.65 检索双车道、§14.69 上下文滑块 v8、§14.72 压缩交互第九工具、§14.76 反例门触发条件收窄——answer 变体仅在有执行事实或未完成 plan 时触发）
> **治理边界**：本文件为 ADR-0010 与当前真实代码运行链路的直映投影，不独立产生新设计。

---

## 1. 真实产品面运行拓扑

系统当前实际运行的主链路为**“单主 Agent 推动 + 冻结 10 工具直接执行（十工具地位平等）+ 机械会话黑板 + 离线审计哈希链”**。

```mermaid
flowchart TD
    User["用户入口 (TUI / -p 无头 / --stdio ACP)"] --> Loop["主 Agent 循环 (orz-loop)"]
    Loop --> Prompt["近零系统提示 + 冻结 10 工具面"]
    Prompt --> Model["DeepSeek 模型推理 (thinking 默认开启)"]

    Model -->|"直接调用 10 工具"| HostExec["机械层执行 (orz-host / orz-loop)"]

    HostExec --> Gate["门禁与权限桥"]
    Gate -->|"写/命令/fetch"| ACAF["ACAF 票据签发 (orz-signer, fail-closed)"]
    ACAF --> ToolRun["工具执行器 (Terminal / FS / Web)"]

    ToolRun -->|"长命令超时自动后台化 (180s)"| BgProc["后台进程管理 (idle-kill 300s 兜底)"]
    ToolRun -->|"结构化信封结果"| HostExec

    HostExec --> Blackboard["会话黑板 (单会话作用域 / (domain, round) 盖章)"]
    HostExec --> Journal["不可篡改审计链 (run-event v0.2 / hash-chained)"]

    Loop --> Guard["生成期健康哨兵 (滚动哈希复读检测 / stall 兜底)"]
    Loop --> Delivery["submit 两阶段交付 (请求 → 确认)"]
    Delivery --> Close["会话归档 (session_archive 单 gzip 打包)"]
```

---

## 2. 核心架构要素与真实边界

### 2.1 Agent 层：冻结 10 工具直接提议面
- **主 Agent**：唯一任务推进者。系统提示近零，面对冻结的固定 10 工具面，十工具地位平等（2026-09-18 用户裁决；注册路径史见 ADR §14.72 第 11 条）：
  `read_file`, `grep`, `search_replace`, `run_terminal_cmd`, `web_search`, `web_fetch`, `blackboard_read`, `submit`, `blackboard_write`（计划/笔记写入，单条 ≤8K）, `context_compress`（知情发起模型参与压缩）。
- **外部检索子代理**：检索启用会话恒注册本地浏览器与原生 web 双族工具（静态标注，本地浏览器优先，换道由模型自主；启用面＝`--retrieval-enabled` 独立检索门，默认关，ADR §14.65）。
- **执行方式**：**直接调用（direct execution）**。早期设计的 `console` 动作栏下单及 `plan-first` 硬门在生产路径中处于休眠/退役状态。
- **轮预算**：默认无限制（`MAX_TOOL_ROUNDS = 0`，撤除默认 120 轮硬限，保留可配逃生阀）。

### 2.2 机械层与安全：ACAF 票据与去硬杀
- **去自身硬超时**：不再以 timeout 硬杀活跃进程；前台命令超过预算（默认 180s）自动后台化，由 300s 无输出/无 CPU 活跃的 `idle-kill` 机制接管兜底，外加 10h 绝对兜底。
- **安全授权**：`orz-signer` 独立进程签发一次性票据。本地文件写入、终端命令执行和网络 fetch 必须过票据门（fail-closed）。

### 2.3 上下文与黑板：会话作用域、折叠渲染与模型参与压缩
- **黑板作用域**：单会话作用域（conversation-scoped）。放弃历史的 plan-epoch 轮换设计，黑板跨轮保留全量记录。
- **折叠渲染**：写入时按 `(domain, round)` 盖章；`blackboard_read` 按域与轮数按需做折叠渲染（render fold），响应头携带黑板水位与「滑块外可压缩 N 块」读数。
- **模型写入面**：`blackboard_write`（plan/notes 区，单条 ≤8K；0ae D0 起在面）。
- **上下文＝模型自控注意力窗口**（ADR §14.69）：模型面＝主滑块＋主滑块以外分块指针（分块内容不流出模型面）；机械轨按阶梯收窄模型面（软提醒 → 320K 硬打断 → 500K 硬截断），语义轨经压缩窗口由模型产出结构化摘要，`context_compress` 可知情发起（ADR §14.72）；压缩不覆盖本地面，全量留档、按块回放。
- **会话归档**：交互会话结束时由 `session_archive` 单包原子写入 gzip 归档文件；无头 `-p` run 落会话持久化并按里程碑增量归档（0ak）。

### 2.4 审计与防篡改：Hash-Chained Journal
- 每次运行写入事件 journal（schema v0.2），每条记录计算单向哈希链（SHA-256）。
- 离线可通过 `--replay` 交叉检验完整性，防篡改、防伪造。
