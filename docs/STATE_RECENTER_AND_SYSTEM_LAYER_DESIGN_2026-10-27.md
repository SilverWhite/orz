# 状态回中与系统层设计

> 日期：2026-10-27
> 用户令：「按‘想优化的四件事’和‘关于稳住核心’这两部分中你提出的方式来做；请先落新一轮补充设计文档，先不急着做。」
> 基线：0.9.14-3 候选（365 批）＋本设计；本文件**只设计，不落码**。
> 承接：363（常驻策略收敛）、365（尾部文案修复）。

## §0 摘要

一句话：**push 只推事件，state 全部 pull；recenter 是系统调用，不是中断。**

本设计把“提醒”从框架里继续往外挪：

1. 清掉剩余的常驻元指令（permission 行、`<plan_framework>` 系统块、
   context-scale 权限宣讲等），改成工具描述/首轮事实/按需 pull。
2. 新增 `state` 分区：`goal / now / next / open / constraints`，小、稳定、
   只在任务变化时更新；模型和人类看同一份状态。
3. 新增 `context_map` pull 工具：块号、轮次、估算 token、回放路径的紧凑表；
   替代被 360 拿掉的常驻块表。
4. 新增 `recenter` 动作：`context_manage(mode=recenter)` 返回 state ＋最近
   N 轮摘要 ＋ 下一步；只读、无副作用、不留下常驻文本。
5. 文本面采用“早期计算的字节纪律”：模型面是紧凑行式记录，人类面才做
   Markdown/GUI 美化；同一状态，两种渲染。
6. 系统层：把 state/context/process/file/log/scheduler 从“散落在机械层的
   提醒”收敛为一个确定性的本地系统（Rust，Windows/Linux），模型通过窄
   API 使用，不直接管 raw PID/文件。

## §1 问题

- **提醒是第二人称命令，软提醒也是背景任务。** 模型会把“你可以考虑压缩”
  读成“我是不是该压缩”，注意力从任务转到合规。
- **常驻元指令仍在。** `[permission]` 行每轮在系统提示；`<plan_framework>`
  是系统块；context-scale 的“你拥有上下文管理权限……”每轮重复。
- **块表拿掉后没有 pull 替代。** 360 默认关闭块表，模型需要地图时只能读
  3.2MB 的 `.gsa/ledger/current.md`。
- **“回中”没有动作。** 现在只能靠压缩/清零，成本高；没有一个轻量的
  “回到任务状态”的调用。
- **模型面文本没有预算纪律。** Markdown 表格、重复标题、散文提醒都按
  token 计费，且对模型的可读性不一定好。

## §2 原则

1. **push 只推改变下一步动作的事件**：用户输入、工具结果、安全门、真实
   后台事件。其余全部 pull。
2. **state 是事实，不是提醒**：`next: 读 353 审计` 是事实；`请记得读 353`
   是命令。事实可以被忽略，命令会制造背景任务。
3. **recenter 是动作，不是常驻**：调用时加载状态；不调用时不存在。
4. **模型面紧凑，人类面漂亮**：同一份数据，模型看行式记录，人类看
   Markdown/GUI；不为模型做视觉装饰。
5. **系统层确定性，模型不做 raw syscall**：系统管状态、内存、进程、文件、
   日志；模型通过窄 API 读写，判断仍归模型。

## §3 四件事

### 3.1 清掉剩余 resident 元指令

| 项 | 现状 | 处理 |
|---|---|---|
| `[permission] mode=…` | 系统提示每轮 | 首轮事实一次；之后按需 pull（`session` 分区/`help`） |
| `<plan_framework>` | 系统提示块 | 降为 `plan_write` 工具描述；不调用时不存在 |
| context-scale 权限宣讲 | 每轮提醒内 | 删除；只保留读数、阈值、动作选项 |
| `SELF_CHECK` 常驻句 | 已默认 off（360） | 保持 off；工具仍在 |
| `[任务状态]` 行 | 已默认 off（360） | 保持 off；`blackboard_read section=plan` |
| 初始问询/工作台 | 一次性 | 保持一次性，不重复 |

验收：模型面 fingerprint 里不再出现这些前缀；只剩安全门（压缩/截断、
permission 决策、终态语法行）和工具结果。

### 3.2 `state` 分区

轻量 schema（≤ 200 token，超过拒绝/裁剪）：

```
state v1
goal: <一句话目标>
now: <当前正在做什么>
next: <下一步动作>
open: <未决问题；none 或 1-3 条>
constraints: <硬约束；none 或 1-3 条>
updated: <round/timestamp>
```

- 存储：黑板新增 `state` 分区（与 plan/notes/findings 同级），持久化、
  跨压缩/截断保留；`state_history` 落 journal。
- 工具：`state_read` / `state_write`（比 `blackboard_write` 更窄，带 schema
  校验与大小上限）；或复用 `blackboard_read/write section=state`。
- 更新策略：任务变化时写；不每轮刷新；last-write-wins，`updated` 由系统写。
- 人类视图：工作台一个 pane；与模型看到的文本同源。
- 与 plan 的关系：plan 是可选的重的结构化计划；state 是永远小的当前事实。
  有 plan 时 `next` 可以指向 plan 的 step id，但不复制整个 plan。

### 3.3 `context_map` pull 工具

- 数据：块号、轮次区间、估算 token、状态（live/compressed/truncated）、
  回放路径。
- 来源：`.gsa/ledger/current.md` ＋ 压缩/截断 marker ＋ 块档案；不新增存储。
- 输出：紧凑行式表，≤ 1K token，列顺序稳定：

```
blk rounds tok state path
1 1-8 32K comp .gsa/compaction/blocks/xxxx-b1.md
...
```

- 工具：`context_map`（或 `blackboard_read section=context_map`）；按需调用，
  不常驻。
- 验收：不再有常驻块表；一次调用返回完整地图；与 marker/ledger 对账一致。

### 3.4 `recenter` 动作

- API：`context_manage(mode=recenter)`（或 `state_recenter`）。
- 返回：`state` ＋ 最近 N 轮摘要 ＋ `next`；**不含**“你应该”的句子；
  ≤ 1K token。
- 副作用：默认只读；v1 不做 trim；不写会话、不注入常驻文本。
- 触发：模型自觉漂移时调用；人类可点一下；不在每轮自动 push。
- 验收：调用后下一步动作与 `state.next` 一致（可观察）；没有留下常驻提醒。

## §4 稳住核心：push/pull 矩阵

| 类别 | 处理 |
|---|---|
| 用户输入 | push |
| 工具结果 | push |
| 安全门（压缩/截断、permission、终态） | push（只推事实与选项） |
| 后台任务完成/结果投递 | push（真实事件） |
| state / context_map / plan / notes / findings | pull |
| guide / 使用讲解 / RLI 趋势 / 编辑历史 / run 基线 | pull |
| 分块表 / D4 / 任务状态行 | pull（已默认 off） |
| 软提醒 | 删除；阈值只在越线时推一次事实，不推“你应该” |

“state anchor”不是每轮注入的状态行；它是 `state` 分区，模型在任务开始、
压缩后、recenter 时读。人类看同一个 pane。**状态由写者维护，不由提醒者维护。**

## §5 文本面：早期计算的字节纪律

模型面（我看到的）与人类面（你看到的）分开：

- **模型面**：行式、`key: value`、字段顺序稳定、短标签、无装饰、无重复
  标题、无散文指令；一行一个事实；ASCII；不用 emoji/ASCII art。
- **人类面**：Markdown/GUI 可以精美；同一份数据，渲染器加颜色/面板/表格。
- **预算**：state ≤ 200 token；context_map ≤ 1K；recenter ≤ 1K；单条 push
  事件 ≤ 200；除工具结果外单块 ≤ 2K。
- **可读性规则**：稳定顺序（goal→now→next→open→constraints）、单位明确、
  空值写 `none`、不写“请/必须”、不嵌套长句、不重复表头。
- **不做**：给模型看的 ASCII 艺术、大表格、彩色标记、重复的“说明”段。

早期计算机的教训不是“回到纯文本”，而是**每个字节都要有信息量**；对模型
来说就是每个 token 都要改变下一步动作，否则就是噪音。

## §6 系统层：同一台机器上的另一个 OS

### 6.1 管什么

- **State store**：state/plan/notes 的持久化、版本、原子写。
- **Context/memory manager**：context_map、压缩、recenter、块档案。
- **Process/job manager**：run、工具 job、后台任务、PID、超时、kill。
- **File/artifact registry**：工作区文件、产物、快照、锁。
- **Event/log store**：journal、审计、回放。
- **Scheduler/interrupt policy**：只 push 改变下一步动作的事件；state 全部 pull。

### 6.2 不管什么

- 模型判断、规划、代码推理；
- 模型直接发 raw syscall / 管 PID / 改文件系统；
- 密钥与凭据（留在 OS credential store）。

### 6.3 形态

- v1（本设计落码的最小集）：进程内 Rust 模块（state、context_map、recenter），
  复用现有 blackboard/journal，不改部署形态。
- v2：本地服务 `orz-system`（Windows named pipe / Unix domain socket），
  CLI/工具通过窄 API 访问；崩溃恢复、原子写、版本化。
- v3：跨平台进程/文件管理（Windows Job Objects / Linux cgroups、文件锁、
  日志轮转），把 run/tool/background 的生命周期收进系统层。

### 6.4 与现有 orz 的关系

现有 journal、blackboard、runs、context_scale、terminal 已经是“原型系统”；
本设计是**收敛**，不是重写：把散在 `agent_loop`/`host_exec`/`blackboard` 的
状态与推送策略，收进一个明确的系统边界。模型仍然通过工具面使用它。

### 6.5 模型会不会更擅长？

会——**如果系统把状态整理成紧凑、稳定、可寻址的事实**，我比“记住一堆提醒”
更擅长在上面推理；但我不会因为有了系统就自动可靠地管 PID/文件。系统的价值
是把“记忆/元认知维护”从模型的注意力里拿走，交给确定性的存储与调度；判断
仍归模型。系统不能替代判断，只能减少噪音和丢失。

## §7 非目标

- 不做常驻教程/使用讲解；不做首屏大段说明。
- 不给模型做视觉美化/ASCII art。
- 不恢复 plan_first 首轮门。
- 不让模型直接管 raw PID/文件/进程。
- 不新增“提醒”机制（包括软提醒）。
- 不做质量 A/B；只做预算、正确性、故障消融。

## §8 验证

- **预算**：state/context_map/recenter 的 token 实测 ≤ 上限。
- **state 正确性**：写读回环、原子写、跨压缩保留、`updated` 只在写时变。
- **context_map 正确性**：块号/轮次/路径与 marker/ledger 对账一致。
- **recenter 效果**：调用后下一步动作与 `state.next` 一致；无残留常驻文本。
- **resident 面**：模型 fingerprint 里不再有 permission/plan_framework/
  context permission/self-check 等前缀。
- **故障消融**：state 过期、state 缺失、map 漂移、recenter 循环、state 膨胀。
- **跨平台**：Windows/Linux 服务启停、IPC、job 控制。

## §9 开放问题

1. state schema 五个字段够不够？要不要 `risks` / `decisions`？
2. state 存黑板分区还是独立 `state.json`？版本化怎么做？
3. `context_map` 做独立工具还是 `blackboard_read` 的一个 section？
4. `recenter` 只读，还是允许可选 `trim`（丢掉过期工作现场）？
5. 系统层 v1 先进程内，还是直接上本地服务？
6. 工作台 UI：面板式还是板式？首屏 onboarding 到什么程度？
