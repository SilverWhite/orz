# 366 批：Agent OS v1 立项与排期（0ds）

> 日期：2026-10-27
> 用户令：「就按照你的需求进行判定就行，请落设计文档吧」「你确定就当前设计了的话就可以正式立项排期并同步 index/todo/backlog 等状态文档了」。
> 设计权威：[`ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md`](../ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md)；先验：[`AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md`](../AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md)；状态/回中：[`STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md`](../STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md)。
> 性质：立项与排期文档；不改设计语义，不落码。

## §0 立项结论

- **项目 ID**：`0ds`；**批次**：366；**优先级**：P1；**状态**：`current-design`（设计定稿，实施未开始）。
- **计数**：65 → 66（BACKLOG 口径）。
- **一句话**：把 orz 从 harness 转成微内核风格 agent runtime——小核（对象/state/context/scheduler/IPC/capability/journal）＋宿主适配（fs/process/net/credential/terminal）。
- **设计已定**：10 条不变量、v1 进程内最小集、v2 `orz-system` 本地服务、v3 host adapters。

## §1 范围

**v1 必做**：

1. `state` 分区 + `state_read/write`（6 字段：goal/now/next/open/constraints/updated；≤200 token；任务变化时写）。
2. `context_map` 工具（块号/轮次/token/状态/回放路径；≤1K token；不常驻）。
3. `context_manage(mode=recenter)`（state + 最近 N 轮摘要 + next；只读；≤1K token）。
4. capability registry 薄层（工具白名单 + session preopen + 审计；v1 不做完整 rights/derive/revoke）。
5. journal 补齐 `state/capability/context/checkpoint` 事件。
6. 清掉剩余 resident 元指令（`[permission]` 行、`<plan_framework>` 系统块、context-scale 权限宣讲）。
7. decisions 台账 + `context_manage(mode=edit/slider/decision_*)`（M1.5；`.gsa/decisions` 追加；summary ≤300/detail ≤2000；edit 原子/可逆/原块留档）。

**v1 不做**：真正 reply cap、独立 `orz-system` 服务、handle-based 文件/进程 syscall、全系统正交持久化、跨平台 adapter crate（均延到 v2/v3）。

## §2 里程碑与排期

> 日期为估算，按用户放行调整；每阶段独立放行，不跳步合批。

| 阶段 | 交付 | 估算 | 依赖 | 出口判据 |
|---|---|---|---|---|
| **M0 设计定稿** | 架构设计、先验调研、状态/回中设计 | 已完成 2026-10-27 | — | 用户令立项 |
| **M1 基础内核** | state 分区 + `state_read/write` + `context_map` + `recenter` + journal 事件 | 1 周（目标 2026-11-03） | M0 | 四工具可用；token 预算达标；测试全绿 |
| **M1.5 分层压缩与上下文编辑** | decisions 台账（`.gsa/decisions` 追加）+ `context_manage(edit/slider/decision_*)` + 压缩摘要 decision IDs + `context_map` decisions | 1 周（目标 2026-11-07） | M1 | 决策可回查；edit 原子/可回滚；前缀影响可读；原块可回放 |
| **M2 能力薄层 + resident 清理** | capability registry（白名单/preopen/audit）；清 permission/plan_framework/context permission | 1 周（目标 2026-11-10） | M1 | 越权拒绝可审；模型面 fingerprint 无残留 |
| **M3 v1 验收** | 10 不变量逐条、故障消融、预算、跨压缩 | 1 周（目标 2026-11-17） | M2 | 验收清单全过；零回归 |
| **M4 orz-system 本地服务** | daemon + named pipe/Unix socket + checkpoint/recovery | 3 周（目标 2026-12-08） | M3 | 跨进程 state/checkpoint；崩溃恢复 |
| **M5 host adapters** | fs/process/net/credential/terminal 契约 + Windows/Linux | 4 周（目标 2027-01-05） | M4 | adapter 契约测试；Job Objects/cgroups 冒烟 |

## §3 验收

- **10 条不变量**逐条（见架构 §0）：小核只给机制；命名即权限；同步会合＋有界消息；reply 一次性；每会话私有命名空间；任务隔离＋监督树；轻量执行体＋reduction；能力路由/父即沙箱；描述符/preopen；OID＋检查点。
- **预算**：state ≤200、context_map ≤1K、recenter ≤1K、单条 push ≤200、decision summary ≤300、decision detail ≤2000。
- **state**：写读回环、原子写、跨压缩保留；`updated` 只在写时变。
- **context_map**：与 marker/ledger 对账一致；块号/轮次/路径匹配。
- **recenter**：调用后下一步与 `state.next` 一致；无残留常驻文本。
- **decisions/edit**：summary/detail 上限；元数据自动填；`decision_read` 默认 active summary；`edit` 原子/可逆；原块 archive 可回放；当前轮编辑原文留 journal。
- **capability**：越权调用拒绝；审计完整；密钥不泄漏。
- **恢复**：崩溃/换装/中断后从最近检查点恢复；未完成 run 标记 `interrupted`。
- **resident 面**：模型 fingerprint 无 permission/plan_framework/context permission/self-check 等前缀。
- **不做质量 A/B**；只做预算、正确性、故障消融。

## §4 依赖与风险

**依赖**：363（常驻策略收敛）、365（尾部文案修复）、blackboard/journal、现有 Rust 模块；不新增外部服务。

**风险**：过度设计（v1 薄层，每阶段放行）；state 漂移（`updated`＋审计）；capability 复杂度（v1 白名单）；跨平台进程语义（adapter 契约测试）；token 开销（预算硬门）；性能（同步 syscall 足够，v1 不做异步内核）。

## §5 入口

- 设计：[`ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md`](../ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md)
- 先验：[`AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md`](../AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md)
- 状态/回中：[`STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md`](../STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md)
- M1.5：[`DECISION_LEDGER_AND_CONTEXT_EDITOR_DESIGN_2026-10-27.md`](../DECISION_LEDGER_AND_CONTEXT_EDITOR_DESIGN_2026-10-27.md)
- 路由：`CLI_PROJECT_INDEX.md` `DESIGN-AGENT-OS-V1`；`TODO.md` `P1-0ds`；`docs/BACKLOG_AND_PRIORITIES.md` `0ds`。
