# orz Agent OS 架构设计 v1

> 日期：2026-10-27
> 用户令：「就按照你的需求进行判定就行，请落设计文档吧。」
> 先验：`AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md`、`STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md`。
> 性质：**设计，不落码**。本文是 v1 的实现权威；v2/v3 只给演进方向。

## §0 摘要与总决策

**orz 从 harness 转成微内核风格的 agent runtime。** 宿主 OS 提供文件、进程、网络、内存、凭据；中间层（host adapters）做兼容路由；小核只拥有语义：对象、命名、隔离、生命周期、状态、调度、IPC、能力、日志。

| 决策 | 选择 | 理由 |
|---|---|---|
| 形态 | 微内核风格 agent runtime，不是内核/VM/hypervisor | 宿主已有资源管理；我们只要 agent 语义 |
| v1 部署 | 进程内 Rust 核心，复用 blackboard/journal | 最小改动，先验证语义 |
| v2 | 本地服务 `orz-system`（named pipe / Unix socket） | 跨进程状态与崩溃恢复 |
| v3 | 宿主适配器（Job Objects/cgroups、文件锁、网络） | 跨平台进程/文件管理 |
| 模型面 | 紧凑行式记录；只有事件 push，state 全部 pull | token 预算 + 注意力 |
| 权限 | 工具即 capability；session preopen；父即沙箱 | seL4/Fuchsia/WASI 的共识 |
| 上下文 | 窗口=物理内存，块=页，context_map=页表，压缩=swap，recenter=context switch | BEAM/OS 分页的类比可落数据结构 |
| 持久化 | OID + 检查点 + journal；不持久化 native 栈/JIT/OS 句柄 | Smalltalk/KeyKOS 的教训 |

**10 条不变量**（来自先验调研，全文遵守）：

1. 小核只给机制，语义在工具/用户态。
2. 命名即权限：能力/句柄/私有命名空间 > 路径 + ACL。
3. 同步会合 + 有界消息；内核不存储转发。
4. 回复通道是一次性能力（v1 用 call_id；v2 再做真正的 reply cap）。
5. 每会话私有命名空间；组合在命名空间层。
6. 任务隔离 + 监督树；故障处理是结构。
7. 轻量执行体 + 私有内存 + 时间片/reduction。
8. 能力路由：静态声明 + 运行时校验；父即沙箱；环检测；弱依赖。
9. 描述符即能力；preopen 注入初始句柄；无全局 open。
10. 持久性作为系统属性：OID + 检查点；崩溃回最近完整检查点。

## §1 范围与非目标

**范围**：对象/命名、state store、context manager、scheduler、IPC/event、capability、journal、host adapter 契约、模型面 ABI、v1 迁移。

**非目标**：完整内核/驱动/MMU；完整 Smalltalk image；完整 BEAM；完整 Fuchsia component framework；WASI 绝对纯度；Erlang 无界 mailbox；人类 UI（独立控制面）；质量 A/B。

## §2 总体架构

```
human control plane (CLI/TUI/UI)         —— 观察同一组对象，不在核内
        │  read/observe/command
        ▼
model (stateless CPU)  ◄── tool/syscall ABI ──►  core
                                              ├─ object registry / namespace
                                              ├─ state store (OID + checkpoint + journal)
                                              ├─ context manager (blocks/pages/context_map/recenter)
                                              ├─ scheduler (turn loop, reductions, budgets)
                                              ├─ IPC / event bus (bounded, reply by call_id)
                                              ├─ capability registry (tools as caps)
                                              └─ journal / observability
                                                      │  host adapter ABI
                                                      ▼
                                      host adapters: fs / process / net / credential / terminal
                                                      │
                                                      ▼
                                                host OS (Windows/Linux)
```

- **core**：确定性、可测试、无业务语义；只做机制。
- **adapters**：薄；把宿主差异挡在 core 外；每个 adapter 有契约测试。
- **tools**：core 暴露给模型的 syscall；工具描述承担教学，不做 resident 教程。
- **model**：无状态；每轮是一次调度量子；通过 ABI 读写状态。

## §3 对象模型与命名

对象类型（v1）：

| 对象 | 说明 | 关键字段 |
|---|---|---|
| `session` | 一次会话/工作区 | id, root, namespace, created |
| `run` | 一次用户回合/执行 | id, session, prompt, status, started/finished |
| `task` | 可调度的工作单元（含后台 job） | id, run, kind, status, pid/handle, budget |
| `artifact` | 文件/产物/快照 | id, path/handle, hash, mime, owner |
| `message` | 用户/模型/工具消息 | id, role, content, call_id, round |
| `tool` | 可调用能力 | name, schema, rights, provider |
| `capability` | (object, rights) 的授权 | id, object, rights, parent, expiry |
| `checkpoint` | 状态检查点 | id, generation, hash, path |

- **稳定 ID**：128-bit（ULID/UUIDv7），跨进程/重启不变；路径/PID 只是属性，不是身份。
- **命名空间**：每 session 一棵私有树；`/state`、`/plan`、`/notes`、`/artifacts`、`/tools`、`/journal`；组合=挂载，不是 API 胶水。
- **句柄**：模型拿到的是一枚 handle（id + rights），不是裸路径/PID；句柄可衰减（rights reduction）、可转移（transfer）、可撤销（revoke）。
- **生命周期**：create → active → closing → closed → archived；父对象关闭时子对象按策略处理（kill/keep/archive）。
- **审计**：每次对象创建/能力使用/状态写都落 journal；带 actor、run、capability、结果。

## §4 State store

### 4.1 state schema（v1）

```
state v1
goal: <一句话目标>
now: <当前正在做什么>
next: <下一步动作>
open: <未决问题；none 或 1-3 条>
constraints: <硬约束；none 或 1-3 条>
updated: <round/timestamp>
```

- 6 个字段；`risks`/`decisions` 暂不单列，放 `open`/`notes`；若实际不够，v1.1 再加。
- ≤ 200 token；超过拒绝写入并返回裁剪建议。
- 只在任务变化时写；不每轮刷新；`updated` 由系统写。
- 存储：黑板 `state` 分区（与 plan/notes/findings 同级），持久化、跨压缩/截断保留；`state_history` 落 journal。
- 工具：`state_read` / `state_write`（窄 schema、大小校验）；也允许 `blackboard_read/write section=state` 兼容。
- 人类面：pane 显示同一份文本；不在本文设计 UI。

### 4.2 持久化与检查点

- **OID + object table**：盘上引用存 OID；加载时 swizzle 回句柄。
- **检查点**：state/plan/notes/artifacts/capabilities 的逻辑状态；不持久化 native 栈/JIT/OS 句柄。
- **写路径**：临时文件 → fsync → 原子 rename；头部 magic/格式版本/端序/generation/校验和。
- **journal**：检查点之间的增量事件；恢复=最近完整检查点 + 重放；未完成 run 标记 `interrupted`。
- **恢复策略**：校验失败拒绝加载（fail-closed），不“尽力而为”；版本不符拒绝；OS 句柄按策略重建/作废。
- v1 先做 state 检查点 + journal；全系统正交持久化留到 v2/v3。

## §5 Context manager

- **窗口=物理内存；块=页；context_map=页表；压缩/截断=swap；recenter=context switch；回放=page fault。**
- 块状态：`live` / `compressed` / `truncated`；块号锚在会话起点，跨轮稳定。
- `context_manage(mode=map)`：返回 `blk / rounds / tok / state / decisions / path` 紧凑表；≤ 1K token；不常驻。M1.5 扩展见 [`DECISION_LEDGER_AND_CONTEXT_EDITOR_DESIGN_2026-10-27.md`](DECISION_LEDGER_AND_CONTEXT_EDITOR_DESIGN_2026-10-27.md)。
- 压缩：模型产出语义摘要；机械层把已闭合块换成分块摘要；逐字原文全量留档。
- 截断：硬门兜底；只保留工作现场；marker 带块档案路径。
- `recenter`：`context_manage(mode=recenter)` 返回 state + 最近 N 轮摘要 + `next` + active decisions（≤5）；只读；≤ 1K token；不写会话、不注入常驻文本。
- 预算：slider/block/guard 沿用现有；token 计量单源；context_map/recenter 计入预算。
- 不做 resident 块表；不做“质量衰减”判断；只报读数与阈值。

## §6 Scheduler

- **协作式 turn loop**：turn 内不抢占；只在安全点（工具返回、模型输出、事件边界）调度。
- **run queue**：优先级 `user_input > tool_result > background > maintenance`；同优先级 FIFO。
- **reductions**：token 预算 + tool 轮预算 + 墙钟；用尽即让出/收口；不在 turn 中强杀。
- **任务/监督**：`task` 有状态与重启策略；失败按监督树重启/降级/归档；不静默吞错。
- **后台 job**：`task_spawn` → handle；`task_wait`/`task_status`/`task_kill`；完成事件 push；结果投递有界。
- **看门狗**：stall/wallclock/tool timeout；只报事实，不注入“催办”文案。

## §7 IPC / syscall ABI

- **工具调用=同步 syscall**：请求（tool name + bounded args）→ 执行 → 结果（bounded）；`call_id` 是 v1 的 reply 标识。
- **有界**：参数/结果有大小上限；超出分页/artifact，不塞进 IPC；不做无界内核队列。
- **异步事件**：通知/事件总线；只 push 改变下一步动作的事件（用户输入、工具结果、安全门、后台完成）。
- **pull**：state/context_map/plan/notes/guide/RLI 趋势/编辑历史/run 基线全部按需。
- **消息格式**（模型面）：行式、`key: value`、字段顺序稳定、短标签、无装饰；错误用 `error: <code>; hint: <one line>`。
- **push/pull 矩阵**：见 §5/§7；软提醒删除；阈值只推一次事实。

## §8 Capability / security

- **工具即 capability**：`(tool, rights)`；rights：`read / write / exec / transfer / derive / revoke`。
- **preopen**：session 启动时注入初始工具集/目录句柄；之后只能相对句柄解析。
- **父即沙箱**：父 session 决定子 session 的能力；子不能自行扩大；环检测；弱依赖可降级。
- **无 ambient authority**：不靠 uid/gid/全局路径；路径在工具边界受控解析；`..`/绝对路径/越界符号链接拒绝。
- **密钥**：OS credential store；不进 state/journal/模型面。
- **审计**：每次能力使用落 journal（actor/run/tool/rights/result）；不可关闭。
- **威胁模型**：prompt injection 不能凭空获得能力；工具升级需显式 grant；数据外发需网络 capability。

## §9 Journal / observability

- 事件：`run_started/finished`、`turn`、`tool_started/completed/failed`、`model_output`、`state_write`、`capability_use`、`context_compress/truncate`、`checkpoint`、`error`。
- 回放：按 run/turn 重放；能回答“这一步为什么发生”（输入、能力、状态、结果）。
- 指标：token 预算、工具轮、延迟、失败率、recenter 次数、state 读写次数。
- 终态落盘：run 终态/检查点原子写；崩溃恢复标记 `interrupted`；不丢已交付。
- 不做隐藏状态：模型可见的状态都能在 journal/state 找到来源。

## §10 Host adapters

| adapter | Windows | Linux | 契约 |
|---|---|---|---|
| fs | std::fs + 原子 rename + 文件锁 | 同左 | 路径规范化、工作区根、快照、权限 |
| process | Job Objects | cgroups v2 | spawn/wait/kill tree/timeout/资源上限 |
| net | WinHTTP/reqwest | reqwest | 能力门控、超时、重试、审计 |
| credential | Credential Manager | libsecret/keyring | 只读、不落日志 |
| terminal | ConPTY | PTY | 会话、日志、kill |
| IPC | named pipe | Unix domain socket | 长度前缀、版本、超时、权限 |

- adapter 只做“翻译”，不做业务判断；所有错误映射为 typed error。
- v1 复用现有 host_exec/terminal/fs；v2 抽成独立 adapter crate；v3 做跨平台进程/文件管理。

## §11 模型面 ABI（v1）

新增/收敛的工具：

- `state_read` / `state_write`：state 分区；窄 schema；≤ 200 token。
- `context_manage`：单工具九模式（map/recenter/compress/edit/slider/decision_read/decision_write/pin/drop）；`map` ≤1K、`recenter` ≤1K＋active decisions ≤5；`edit/slider/decision_*` 见 M1.5 设计。
- `task_spawn` / `task_status` / `task_wait` / `task_kill`：后台 job；返回 handle。
- 现有 `read_file`/`write`/`grep`/`run_terminal_cmd`/`blackboard_*`/`self_check` 保留；v1 用路径+能力检查，v2 逐步换 handle。

**模型面文本规则**：行式、稳定顺序、短标签、空值 `none`、不写“请/必须”、不做 ASCII art；人类面才做 Markdown/GUI。

## §12 人类控制面

不在核内；观察同一组对象（state/plan/context_map/journal/tasks）。首屏一屏“这是什么”，之后 `help`/`guide` 按需拉取；不做 resident 教程。

## §13 v1 范围与迁移

**v1 落码（最小集）**：

1. `state` 分区 + `state_read/write`（复用 blackboard）。
2. `context_manage(mode=map)`（复用 block/marker/ledger）。
3. `context_manage(mode=recenter)`（复用 context_manage）。
3b. M1.5：`decisions` 追加台账 + `context_manage(mode=edit/slider/decision_*)`。
4. capability registry 薄层（工具白名单 + session preopen + 审计；不实现 seL4 CSpace）。
5. journal 事件补齐（state/capability/context/checkpoint）。
6. 清掉剩余 resident 元指令（permission 行、`<plan_framework>` 系统块、context-scale 权限宣讲）。

**迁移映射**：

| 现有 | 新 |
|---|---|
| `blackboard` plan/notes/findings | state store 分区 |
| `journal` | event log |
| `context_scale` | context manager |
| `runs` | run/task |
| `host_exec`/`terminal` | syscall + host adapters |
| permission mode | capability registry（v1 薄层） |

不重写：复用现有 Rust 模块与数据；先加边界，再抽服务。

## §14 验证

- **不变量**：10 条逐条测试（命名即权限、有界 IPC、检查点恢复、无 ambient authority 等）。
- **预算**：state ≤ 200、context_map ≤ 1K、recenter ≤ 1K、单条 push ≤ 200。
- **正确性**：state 写读回环、原子写、跨压缩保留；context_map 与 marker/ledger 对账；recenter 后下一步与 `state.next` 一致。
- **恢复**：崩溃/换装/中断后从最近检查点恢复；未完成 run 标记 `interrupted`。
- **能力**：越权调用拒绝；审计完整；密钥不泄漏。
- **跨平台**：Windows/Linux adapter 契约测试。
- **故障消融**：state 过期/缺失、map 漂移、recenter 循环、state 膨胀、capability 环。
- 不做质量 A/B。

## §15 风险与开放决策

**风险**：过度设计（先 v1 薄层）；state 漂移（`updated` + 审计）；capability 复杂度（v1 白名单）；跨平台进程语义（adapter 契约测试）；token 开销（预算硬门）；性能（同步 syscall 足够，v1 不做异步内核）。

**我已决定**：state 6 字段、黑板分区存储、`context_manage` 单工具（map/recenter/compress/edit/slider/decision_*）、`recenter` 只读、v1 进程内、capability v1 薄层、M1.5 decisions/context editor。

**留待 v2**：真正 reply cap、独立 `orz-system` 服务、handle-based 文件/进程 syscall、全系统正交持久化、跨平台 adapter crate。

## §16 先验映射

| 决策 | 来源 |
|---|---|
| 同步会合 + 有界消息 + reply | seL4 |
| 每会话私有命名空间 + 9P 式请求 | Plan 9 |
| 监督树 + 隔离 + reduction 调度 | Erlang/OTP + BEAM |
| 能力路由 + 父即沙箱 | Fuchsia |
| 描述符/preopen/无全局 open | WASI |
| OID + 检查点 + 不持久化 native 状态 | Smalltalk/KeyKOS/Oberon |

## §17 参考

见 `AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md` §6；本设计不重复引用。
