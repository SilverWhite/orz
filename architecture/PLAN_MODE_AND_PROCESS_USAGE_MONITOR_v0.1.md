> **pre-ADR-0010 冻结状态头（2026-09-24，审查修复批）**：本文件属 pre-ADR-0010 时期冻结 fixture／development-only spike——**非 `current`、仅历史基线**，不作为实现依据；当前设计权威＝[`ADR-0010`](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)（本状态头为其 §7.2 归档规则的**就地冻结**轻量替代，不移档不改正文，2026-09-24 批登记）。

# Plan Mode and Process Usage Monitor v0.1

状态：stable design candidate；固定工作流语义与运行监控契约，不决定最终 UI 改版细节。

## 1. 目标

本设计新增两个稳定能力：

- Plan mode：在执行前形成稳定、可审批、可回放的计划阶段。
- Process usage monitor：在运行期间持续显示本次运行进程树的 CPU、内存和时长，帮助用户发现异常占用。

这两个能力都应作为 CLI 的常驻基础能力，而不是某个模型、某个 provider 或一次性脚本的附加行为。Claude Code、DeepSeek、Grok、Codex 或其他 adapter 可以触发这些能力，但不能拥有这些能力的语义。

## 2. 非目标

- 不复制 Claude Code 的 UI 或 permission prompt 文案。
- 不把 Plan mode 变成固定格式的长模板。
- 不在本设计中决定顶部菜单、地址栏、Windows 设计语言或其他 UI 精简方案。
- 不实现性能 profiling、火焰图、线程级诊断或全系统资源监控。
- 不把资源采样作为任务正确性、模型质量或科学 claim 的证据。

## 3. Plan Mode

### 3.1 固定大框架

Plan mode 的输出必须包含四个稳定部分，但每个部分内部格式由任务上下文决定：

1. 前期调查
2. 具体计划
3. 具体设计
4. 实施方案

固定的是阶段语义，不是每个阶段的 bullet 数、标题层级、表格形式或文字长度。模型可以使用短段落、清单、表格或混合格式，只要四个部分都能被用户和 verifier 明确识别。

### 3.2 阶段语义

前期调查：

- 读取或检查相关文件、配置、既有实现、近期上下文和已知约束。
- 标记哪些事实是 observed，哪些只是 inferred。
- 对需要联网或外部文档确认的内容，记录来源和可见性状态。
- 若无法完成必要调查，应明确写出阻塞项，不能用猜测替代调查。

具体计划：

- 给出执行步骤、先后顺序和依赖关系。
- 标明哪些步骤会修改文件、运行命令、联网、产生费用或请求额外权限。
- 对高风险或不可逆步骤给出拆分策略。

具体设计：

- 描述核心状态、接口、数据流、权限流、记录层和用户可见行为。
- 对边界情况给出处理原则，例如审批被拒、采样失败、子进程逃逸、VS Code terminal title 不可写等。
- 明确哪些设计决定稳定，哪些仍是 deferred 或 implementation choice。

实施方案：

- 列出建议修改位置、验证方式、测试范围和回滚/降级行为。
- 说明进入实施前需要的审批模式。
- 如果任务适合分阶段执行，应给出最小可交付切片。

### 3.3 进入和退出

Plan mode 可以通过显式命令、配置默认值、任务风险分类或用户自然语言触发。进入后，运行状态应标记为 `planning`。

Plan mode 期间允许：

- 读取 workspace 文件。
- 查询本地状态。
- 做只读分析。
- 生成或更新计划 artifact。
- 请求用户澄清。

Plan mode 期间默认不允许：

- 修改项目文件。
- 执行会改变外部状态的命令。
- 发起真实网络请求。
- 安装依赖、写入配置、提交 git、删除或移动文件。

例外必须由 approval policy 明确授权，并在计划 artifact 中留下原因。即使有例外，Plan mode 也不应静默进入实施。

退出 Plan mode 必须产生一个用户可见的计划审批点。审批结果至少包含：

- approve：按指定 approval mode 进入实施。
- revise：继续规划并保留计划版本。
- reject：停止本次任务或返回普通对话。

### 3.4 与审批模式的关系

Plan mode 只决定“先计划再执行”的阶段门禁；它不替代自动审批或手动审批。

推荐抽象：

```text
PlanningPolicy = none | suggested | required
ApprovalPolicy = manual | auto | mixed
```

组合示例：

- `required + manual`：必须计划，计划通过后每个敏感动作仍需人工确认。
- `required + auto`：必须计划，计划通过后低风险动作可自动执行。
- `suggested + mixed`：高复杂度任务自动建议计划，普通只读任务直接执行。

Plan approval 只能批准“进入实施阶段”，不能一次性批准计划之外的所有未来动作。实施中若发生计划外高风险动作，应重新请求审批或 replan。

### 3.5 记录与验证

每个 Plan mode run 应产生可引用 artifact：

```text
plan-mode-result.json
plan-mode-result.md
```

最小字段：

- run ID、task ID、workspace root、时间。
- 触发方式和 planning policy。
- 四个固定部分的 presence check。
- 计划版本和 previous plan digest。
- 计划审批结果、审批时间、审批 authority label。
- 进入实施时选定的 approval policy。
- deferred decisions。

Verifier 只检查结构、版本、审批链和边界，不判断计划质量。计划质量仍由用户、后续实现和测试结果共同决定。

## 4. Process Usage Monitor

### 4.1 监控对象

监控对象是“本次运行的 root process 及其 child process tree”，不是整个系统，也不是终端窗口进程本身。

运行 launcher 必须尽量记录：

- root PID。
- start time。
- command label。
- terminal kind：`system_terminal`、`vscode_terminal` 或 `unknown_terminal`。
- process containment mechanism，例如 Windows Job Object、shell wrapper、PTY owner 或 adapter-provided PID。

如果无法可靠定位 root PID，应显示 `usage unavailable`，并把原因写入 run event。禁止用窗口标题、进程名或最近启动时间猜测并伪装为可靠绑定。

### 4.2 采样内容

默认采样进程树聚合值：

- CPU percent。
- memory working set 或 RSS。
- private bytes，平台可用时显示。
- elapsed time。
- process count。
- optional：I/O bytes、handle count、thread count。

CPU 百分比按逻辑核心聚合口径显示，允许超过 100%。例如 8 核机器上 400% 表示约 4 个逻辑核心被持续占用。

采样事件应写入 append-only run event，至少记录：

- timestamp。
- sample interval。
- root PID binding。
- process count。
- CPU and memory values。
- completeness：`complete`、`partial` 或 `unavailable`。
- reason when unavailable。

资源采样可以降频或丢样，但必须标注；不能插值为真实观测值。

### 4.3 刷新频率

系统终端：

- 默认 1 秒刷新一次。
- 可在长时间稳定空闲时退避到 2 秒。
- tool/model 状态切换、CPU/MEM 阈值越界或用户请求详情时恢复 1 秒。

VS Code 终端：

- title 层默认 2 秒刷新一次。
- 目标是异常感知，不是性能分析。
- title 内容保持短，不覆盖主要输出。

推荐 title 格式：

```text
CPU 185% MEM 2.1G 14m
```

异常时可使用短状态前缀：

```text
HIGH CPU 420% MEM 3.8G 22m
HIGH MEM 7.6G 31m
IDLE? CPU 0% MEM 2.2G 18m
```

2 秒刷新是当前稳定默认值。除非后续真实使用证明需要更高频，否则 VS Code title 不应高频刷新。

### 4.4 展示位置

系统终端：

- CLI 内部状态栏或运行状态行显示。
- 不应反复向主日志追加大量重复资源行。
- 用户需要复制日志时，应能关闭动态状态行或导出 clean log。

VS Code 终端：

- 首选 terminal title 或等效轻量位置显示。
- 若 title 写入失败，降级为低频状态行。
- 不要求 VS Code extension；若未来存在 extension，可把详细数据放入 status bar 或侧栏。

### 4.5 负载预算

Process usage monitor 必须轻量：

- 默认只采样绑定的进程树。
- 避免每次全系统 WMI 深扫。
- Windows 上优先使用 root PID + child tree 缓存，必要时周期性刷新 child set。
- title 更新与 terminal repaint 不超过默认 2 秒频率。
- telemetry 写入使用 bounded queue；压力过高时降低采样频率，不丢失语义事件。

目标预算：

- 普通运行下监控自身 CPU 占用接近不可感知。
- 资源采样不得成为模型输出卡顿、terminal 卡顿或日志膨胀的主要原因。
- 监控失败不应中断主任务，除非用户显式要求 strict monitor。

### 4.6 阈值与异常提示

默认只做轻提示，不自动 kill：

- sustained high CPU。
- sustained high memory。
- process count 持续增长。
- CPU 近 0 但任务长时间没有语义事件。
- root PID exited but children remain。

自动终止属于 approval-controlled action，不属于 monitor 默认行为。monitor 可以建议用户检查、暂停、终止或继续观察。

## 5. 与现有可观测执行层的关系

本设计延续 `OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md` 的原则：记录可观察事实，不猜测不可见状态。

Plan mode 产生执行前 artifact。Process usage monitor 产生运行时 telemetry artifact。二者都应能进入 run journal，但它们的证据含义不同：

- Plan artifact 证明某次任务曾形成并审批过计划。
- Usage sample 证明某个时间点观察到的进程树资源占用。
- 二者都不证明最终任务完成、claim 正确或模型推理可靠。

## 6. 实施切片

建议按以下顺序实现：

1. Plan mode artifact schema and verifier：先固定四部分 presence、版本链和审批记录。
2. CLI planning state：实现 `planning -> awaiting_plan_approval -> executing` 状态转移。
3. Approval policy adapter：把 plan approval 与 manual/auto action approval 分离。
4. Process tree sampler：实现 Windows root PID + child tree 聚合采样。
5. System terminal status line：先在 CLI 内部显示 CPU/MEM/time。
6. VS Code terminal title adapter：以 2 秒刷新为稳定默认，失败时降级。
7. Journal integration：把 plan artifact 和 usage telemetry 接入 run event。
8. UX review：结合当前 Windows 设计语言，再决定顶部区域、菜单和地址栏的精简方式。

## 7. Open Decisions

- Plan mode 默认策略是 `suggested` 还是 `required`。
- 哪些任务类型必须计划，例如多文件修改、真实网络、长运行、费用、删除、git 操作。
- 自动审批 classifier 是否只做建议，还是允许在白名单内直接放行。
- CPU/MEM 阈值默认值。
- VS Code terminal title 写入的具体机制。
- UI 顶部区域如何在当前设计语言中替换冗余内容。

## 8. 决策冻结

当前冻结：

- Plan mode 是稳定常驻能力。
- Plan mode 固定四个大部分：前期调查、具体计划、具体设计、实施方案。
- 四个部分内部格式不固定。
- Plan approval 与 action approval 分离。
- 进程占用监控以本次运行进程树为对象。
- 系统终端在 CLI 内部即时显示。
- VS Code terminal title 默认 2 秒刷新。
- 资源监控用于异常感知，不用于高频性能分析。

当前延期：

- 顶部 UI 精简方案。
- Windows 设计语言对齐细节。
- 自动审批与手动审批的完整策略表。
- 严格资源阈值与自动处置策略。
