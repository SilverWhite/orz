# Agent 底座嫁接价值中文评估

日期：2026-07-30

状态：评估稿，不是最终架构裁决。

## 1. 这份文档解决什么问题

前面的英文整合文档把边界写清楚了，但不适合快速判断“到底保留了什么”。这份中文文档只做三件事：

1. 用中文确认关键设计和实现是否还在。
2. 评估这些内容嫁接到 Grok CLI / Grok Build 和其他成熟 agent 框架时的价值。
3. 先保留选择空间，尤其是客户端、远程和双端入口，不急着把产品底部锁死到单一生态。

本评估不意味着马上切换默认 runtime，也不意味着删除现有保障层。当前更合理的姿势是：先看成熟底座能替我们省掉什么，再决定哪些本地实现降级为适配器、夹具或保障外壳。

## 2. 先确认：关键内容没有删

### 2.1 检索、浏览器、PDF、来源可见性

保留状态：已保留。

主要文件：

- `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- `assurance/browser_retrieval.py`
- `assurance/browser_retrieval_broker.py`
- `assurance/pdf_evidence.py`
- `assurance/retrieval_workflow.py`
- `assurance/source_visibility.py`
- `assurance/source_visibility_cli.py`
- `assurance/retrieval-result-v0.1.schema.json`
- `assurance/retrieval-task-contract-v0.1.schema.json`
- `assurance/source-visibility-ledger-v0.1.schema.json`
- `assurance/source-visibility-gate-receipt-v0.1.schema.json`
- `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- `assurance/tests/test_browser_retrieval.py`
- `assurance/tests/test_browser_retrieval_e2e.py`
- `assurance/tests/test_retrieval_workflow.py`
- `assurance/tests/test_source_visibility.py`

保留理由：

成熟 agent runtime 通常能“搜索、打开网页、调用浏览器或 MCP”，但它们不一定解决科学任务里最麻烦的问题：来源全文可见性、PDF 哈希、证据分层、snippet 不足、引用能否支撑 claim。这些是本项目的差异价值，不能交给通用 runtime 隐式处理。

嫁接判断：

- Grok/其他 runtime 可以负责实际 tool 调用和浏览动作。
- 本仓库继续负责 source visibility gate、PDF evidence、retrieval receipt、引用可验证性。
- 检索能力不应删，应该变成 Grok 工具调用后的保障层。

### 2.2 两个检索子代理

保留状态：已保留。

主要文件：

- `assurance/retrieval_subagent.py`
- `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- `assurance/capability-delegation-receipt-v0.1.schema.json`
- `assurance/child_capability_enforcer.py`
- `assurance/child-capability-enforcement-receipt-v0.1.schema.json`
- `assurance/tests/test_retrieval_subagent.py`
- `assurance/tests/test_retrieval_subagent_real.py`

两类子代理：

- 项目文档检索子代理：面向本仓库、本地文档、项目上下文。
- 外部检索子代理：面向外部网页或公开资料，需要更严格的网络能力和来源 receipt。

保留理由：

这两个不是“为了炫技的多 agent”。它们解决的是检索任务的能力边界和证据来源边界：谁被允许看什么、谁返回什么 receipt、哪些内容可以进入主回答。

嫁接判断：

- 如果 Grok 原生 subagents/workflows 足够，应把这两个设计映射到 `.grok/agents`、`.grok/workflows` 或 Grok subagent 配置。
- 本仓库保留 capability receipt、child capability enforcement、source/evidence gate、redaction。
- 不应再写一个本地多代理调度器，除非先证明 Grok 的 subagent/workflow 不能满足 receipt 和能力边界。

### 2.3 内部工具状态返回与工具可用性

保留状态：已保留。

主要文件：

- `assurance/tool_availability_gate.py`
- `assurance/tool-availability-report-v0.1.schema.json`
- `assurance/tool-availability-gate-receipt-v0.1.schema.json`
- `assurance/tool-availability-check-event-payload-v0.1.schema.json`
- `assurance/tool-belief-stagnation-event-payload-v0.1.schema.json`
- `scripts/check_tool_availability.py`
- `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md`
- `assurance/tests/test_tool_availability_gate.py`

保留理由：

这个模块防的是一个实际问题：模型经常说“我会搜索/我会调用工具/我会查看文件”，但当前运行环境可能没有那个工具，或者工具不可用。成熟 runtime 有自己的 tool registry 和 permission UI，但它不一定会把“当前可用/不可用工具状态”以我们需要的形式注入保障上下文。

嫁接判断：

- Grok 应负责实际工具注册、工具调用、权限交互。
- 本仓库负责把工具可用性转成显式 receipt，并在 UI/回答前暴露。
- Tool belief stagnation 仍应保留，用来检测“模型持续相信不存在的工具可用”。

### 2.4 中立询问、反例、自我质询

保留状态：已保留。

主要文件：

- `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- `evaluation/SCORING_PROTOCOL_v0.1.md`
- `architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`
- `assurance/README.md`

保留理由：

这部分不是 runtime 能自然替代的功能。它要求 agent 在形成结论前主动问：有没有反例、有没有相似但不等价的案例、有没有证据缺口、当前 claim 有没有过度外推。

嫁接判断：

- Grok/Claude/Codex/Aider 等都可以作为执行模型。
- 中立询问和反例质询应作为保障策略、prompt/plan 结构、评估 gate 和 UI 提醒保留在本仓库。
- 这部分可以利用 mature runtime 的 planning/subagent 能力，但不能被 runtime 黑箱吞掉。

### 2.5 反哨兵、防单方向过推进、停滞检测

保留状态：已保留。

主要文件：

- `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`
- `docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md`
- `runtime/global-progress-*.schema.json`
- `runtime/tests/test_global_progress_sentinel.py`
- `assurance/orientation_runtime_guard.py`
- `assurance/orientation_runtime_integration.py`
- `assurance/orientation_runtime_journal.py`
- `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- `assurance/orientation-*.schema.json`

这里的“反哨兵”可以理解为两层：

- Global Progress Sentinel：防止项目只朝一个方向猛推，缺少回看、缺少新证据、缺少反例。
- Orientation Runtime Guard：防止运行过程停滞、重复、工具信念错误、输出方向偏离。

保留理由：

成熟 runtime 负责执行，但不一定知道“这个研究项目是不是在自我强化错误方向”。这个判断属于外部保障层。

嫁接判断：

- Grok workflow 可以提供任务组、恢复、rerun 和 session 状态。
- 本仓库继续保留 Global Progress、Orientation Guard、stagnation guard。
- UI 应展示这些 guard 的状态，而不是把它们藏在日志里。

### 2.6 UI 功能性和设计

保留状态：已保留。

主要文件：

- `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`
- `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`
- `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`
- `assurance/tui/`

保留理由：

UI 不是可有可无的壳。它承载的是用户需要的工作方式：大窗口、任务流、计划/清单、来源/claim/receipt/journal 检查、工具状态、运行状态、中文操作语义。

嫁接判断：

- Grok TUI 可以提供成熟交互经验和 runtime 状态来源。
- 本项目 UI 继续作为保障工作台，显示真实 runtime 事件和保障 receipt。
- UI 不应自造模型循环，也不应被简单删除。

## 3. Grok CLI / Grok Build 的嫁接价值

### 3.1 高价值部分

Grok CLI / Grok Build 对本项目最有价值的是：

- 开源 CLI/runtime 底座，可读、可审、可 fork。
- 已有模型循环、session、工具调用、权限、sandbox、headless、ACP、TUI。
- 有 subagents/workflows/config/MCP/hooks/plugins/skills 等可嫁接面。
- Windows 可运行，和当前用户环境匹配。
- 官方 Grok 产品有 web/mobile/API 生态，虽然移动端 app 源码未确认开源，但账号、会话和远程入口本身仍有生态价值。

这意味着：Grok 很适合作为底层 agent runtime，而本仓库保留上层保障、证据、UI 和适配。

### 3.2 不能误解的部分

目前没有确认 xAI 官方开源了 Grok iOS/Android 客户端源码。之前“Grok 移动端也开源”的信息应撤回。

因此移动端价值要改写成：

- 不是“直接复用开源移动端代码”。
- 而是“Grok 生态有官方移动端/网页入口，CLI/runtime 又开源，这给未来双端/远程体验留下空间”。

### 3.3 当前版本关系

当前本地 PATH 上的 `grok --version` 返回：

```text
grok 0.2.112 (9bbd559437) [stable]
```

但仓库里已验证并提升的 lock 仍是：

```text
0.2.111 (94172f2aa4)
```

所以 `0.2.112` 现在只是候选，不是项目默认。后续要先做 identity、hash/signature、ACP initialize、fake tool/permission/cancel、child-tree、workspace trust、workflow/subagent、event bridge、TUI projection 等 gate，再考虑提升。

## 4. 其他成熟框架的借鉴价值

### 4.1 Claude Code

主要借鉴：

- subagent 的 scoped context。
- 每个 subagent 的工具权限边界。
- hooks、MCP、slash commands、permission 模型。

嫁接位置：

- 用来校准我们两个 retrieval subagent 的边界设计。
- 不一定作为主 runtime。

### 4.2 Gemini CLI / Qwen Code

主要借鉴：

- CLI agent 的插件、扩展、MCP、provider/model 配置经验。
- Qwen Code 作为 Gemini CLI fork，说明“成熟 CLI 底座上魔改”是可行路线。

嫁接位置：

- provider 抽象、skills/agents 组织、MCP 发现。
- 可作为 Grok 不足时的参考实现，不急着替换主底座。

### 4.3 Goose

主要借鉴：

- MCP-native extension。
- recipes/workflows。
- subagents、compaction、ACP provider 形态。

嫁接位置：

- 工作流 recipe 和扩展注册。
- 远程/本地能力桥接方式。

### 4.4 OpenAI Codex CLI / ChatGPT 移动端

主要借鉴：

- sandbox/approval 的产品化体验。
- ChatGPT 移动端可远程查看 Codex 工作，但客户端源码不是开源底座。

嫁接位置：

- 作为远程任务体验、审批体验、移动查看体验的参考。
- 不适合作为“可 fork 的移动端代码来源”。

### 4.5 Aider

主要借鉴：

- git-first edit loop。
- repo map。
- lint/test 集成。
- prompt caching。

嫁接位置：

- 编辑、diff、repo context、测试建议。
- 不作为主 agent runtime。

## 5. 部件级采用裁决

本节由 Codex 负责做初步取舍。目标是把“参考成熟 agent 框架”拆成具体部件，明确哪些生产采用、哪些只借鉴设计、哪些暂不采用，避免后续再次变成无边界扩张。

### 5.1 总原则

默认规则：

- 生产 runtime 底座：优先采用 Grok CLI / Grok Build。
- 保障层：继续采用本仓库自己的 assurance 设计。
- UI：继续采用本仓库已设计的保障工作台 UI。
- 其他 agent 框架：只借鉴成熟部件和交互模式，不作为默认 runtime。
- 新增任何 agent、检索模式、远程入口或调度层，都必须显式评审。

一句话：

```text
Grok 做通用 agent runtime；
本仓库做 evidence / assurance / UI / adapter；
其他框架只贡献成熟模式，不直接扩大产品范围。
```

### 5.2 Grok CLI / Grok Build：生产采用部件

以下部件采用 Grok 作为默认生产所有者：

| 部件 | 采用方式 | 本仓库边界 |
|---|---|---|
| 模型请求/响应循环 | 采用 Grok runtime | 不再自造 production model loop |
| session 生命周期 | 采用 Grok session/ACP/headless 能力 | 只做 lifecycle receipt 和 UI 投影 |
| tool dispatch | 采用 Grok 工具调用循环 | 本仓库只做 tool availability gate、receipt、source gate |
| permission 基础面 | 采用 Grok permission/safety surface | 本仓库保留 hard gate 和审计 receipt |
| sandbox/进程基础 | 采用 Grok + Windows supervisor | 本仓库只记录边界、验证和进程树 metadata |
| provider/custom model 配置 | 采用 Grok `config.toml`、custom model、query params、env headers | Direct DeepSeek 仅保留为 conformance/fallback diagnostics |
| ACP | 产品主嫁接面优先采用 ACP | headless 只用于 smoke 或窄场景 |
| workflows | 采用 Grok workflow 作为任务组/重跑/恢复载体 | Global Progress/Orientation Guard 不交给 workflow 决策 |
| subagents | 采用 Grok subagent/profile 机制承载两个检索子代理 | 不新增本地 multi-agent scheduler |
| MCP/plugins/skills | 仅显式启用项目需要的 scoped 能力 | 不默认自动扩展能力面 |
| hooks | 只用于观察、注释、辅助记录 | 不承载 hard gate，因为 hook 可能 fail-open |
| compaction/checkpoint | 采用 Grok 原生状态与事件 | 本仓库记录 provenance、digest、verification |

### 5.3 Grok 采用时的具体限制

Grok 不是“全权接管”。采用 Grok 的同时，必须遵守这些限制：

- `0.2.112` 目前只是候选；项目默认仍以已验证 lock 为准，直到候选 gate 通过。
- 不能因为 Grok 有 subagents 就发展出多 agent 生态。
- 不能因为 Grok 有 workflow 就删除 Global Progress Sentinel 或 Orientation Guard。
- 不能因为 Grok 有 web/search/MCP 就隐式绕过本地浏览器检索和 source visibility。
- 不能把 hard gate 放进 Grok hook。
- 不能把 provider-private reasoning、Authorization、raw hidden transcript 写进本仓库 receipt。

### 5.4 Claude Code：借鉴部件

Claude Code 不作为本项目默认 runtime，但采用它的这些设计经验：

- 子代理边界：每个 subagent 有明确职责、上下文和工具权限。
- 子代理配置形态：适合借鉴为两个 retrieval subagent 的 profile/frontmatter 结构。
- hooks 经验：用于观察点、提醒点、辅助记录，而不是安全硬门禁。
- 权限体验：用户能看懂“这个子代理能用什么工具，不能用什么工具”。

采用结论：

- 借鉴 scoped subagent 和 permission UX。
- 不采用 Claude Code 作为主底座。
- 不引入 Claude 风格的任意多子代理生态。

### 5.5 Goose：借鉴部件

Goose 的价值主要在 extension/workflow/ACP 方向。

采用它的这些设计经验：

- MCP-native extension 组织方式。
- recipes/workflows 作为可保存、可复跑的任务流程。
- ACP provider 形态。
- subagent 与 compaction 的状态暴露方式。

采用结论：

- 借鉴 recipes/workflows 和 MCP extension registry。
- 不采用 Goose 作为默认 runtime。
- 不把远程入口提前纳入当前本地优先阶段。

### 5.6 Aider：借鉴部件

Aider 的价值主要在代码修改循环，而不是通用 agent runtime。

采用它的这些设计经验：

- git-first edit loop。
- repo map / 代码上下文压缩。
- diff/patch 生成和审查体验。
- lint/test 与修改循环绑定。
- prompt caching 对长仓库任务的成本控制思路。

采用结论：

- 后续可借鉴 Aider 做 repo context、diff、test loop。
- 不采用 Aider 作为主 runtime。
- 不让 Aider 式编辑循环覆盖本仓库 assurance gate。

### 5.7 OpenAI Codex / ChatGPT：借鉴部件

Codex/ChatGPT 的价值主要在 sandbox、approval、任务远程体验和产品化交互。

采用它的这些设计经验：

- sandbox/approval 的用户体验。
- 任务状态、等待、继续、接管的产品逻辑。
- 移动端远程查看任务的交互经验。
- VS Code / Codex app server / 内部终端生命周期观测经验。

采用结论：

- 借鉴审批体验、任务状态体验、移动查看体验。
- 保留 VS Code 内部终端初步适配证据。
- 不采用 ChatGPT 移动端作为可 fork 客户端来源。
- 不把 Codex 作为当前主 runtime。

### 5.8 Gemini CLI / Qwen Code：借鉴部件

Gemini CLI 和 Qwen Code 的价值主要在“成熟 CLI 底座可 fork/可适配”的路线证明。

采用它们的这些设计经验：

- provider/model 配置方式。
- extension/skill/agent 组织方式。
- MCP discovery 的工程模式。
- 从成熟 CLI fork 后针对模型生态做窄改造的路径。

采用结论：

- 借鉴 provider adaptation 和 extension layout。
- 不替换 Grok 主底座。
- 不做多 runtime 平级支持，除非后续 Grok gate 明确失败。

### 5.9 本仓库继续生产拥有的部件

以下部件不交给任何成熟 runtime：

| 部件 | 本仓库继续拥有的原因 |
|---|---|
| source visibility | 这是科学证据边界，不是通用搜索 |
| PDF evidence | 需要本地哈希、可见性、receipt |
| retrieval receipt | 需要区分本地浏览器、外部检索、fallback |
| tool availability gate | 需要把真实工具状态显式返回给模型和 UI |
| tool belief stagnation | 防止模型持续相信不可用工具 |
| counterexample gate | 只在 plan/conclusion 写入前触发 |
| neutral inquiry | 执行中辅助全局回看 |
| Global Progress Sentinel | 防止整体方向单向过推进 |
| Orientation Runtime Guard | 防停滞、防重复、防方向漂移 |
| capability receipts | 子代理和网络能力边界需要本地审计 |
| UI assurance workbench | 用户专门需要的操作面 |
| 全局审查模式 | 防止局部正确但整体背离设计理念 |

### 5.10 明确不采用的路线

暂不采用：

- 新建本地 production model loop。
- 新建本地 production tool dispatcher。
- 新建本地通用 multi-agent scheduler。
- 默认启用超过两个子代理。
- 隐式混用本地浏览器检索和框架检索。
- 把反例询问放进每个执行步骤。
- 把 hard gate 放进 fail-open hook。
- 把远程/Web/移动入口作为当前实现重点。
- 为了兼容多个框架而提前做大型 runtime abstraction。

这些路线不是永远不能做，而是必须先有明确缺口、明确收益和明确评审。

## 6. 底部可以多混搭

既然 Grok 移动端源码没有确认开源，底部客户端层就不必被 Grok 单一绑定。可以拆成三层看：

### 6.1 Runtime 层

优先候选：

- Grok CLI / Grok Build。

原因：

- 开源。
- 当前仓库已有大量 Grok 适配证据。
- Windows 和 ACP/headless 路线已有基础。

### 6.2 Assurance 层

必须保留本仓库自有：

- 检索证据。
- 来源可见性。
- PDF evidence。
- tool availability。
- counterexample/self-question。
- Global Progress Sentinel。
- Orientation Runtime Guard。
- capability receipts。
- UI 状态投影。

原因：

- 这是产品差异，不是通用 agent runtime。

### 6.3 Client / Remote 层

可以混搭：

- 本地 TUI：保留现有设计，作为主保障工作台。
- Web UI：以后可作为远程监控和审批入口。
- 移动端：可以参考 Grok/ChatGPT/Codex 的移动体验，但不假设能直接复用移动端源码。
- 远程任务：可参考 Codex 的移动远程查看体验、Grok 官方 web/mobile 生态、Goose/ACP 的 provider 形态。

这层应该晚一点定。现在先把 runtime 与 assurance 的边界评估清楚。

## 7. 本轮新增设计约束

这些约束来自 2026-07-30 本轮方向复盘。它们不是最终实现方案，但应作为后续嫁接评估的默认判断标准。

### 7.1 检索模式必须显式切换

普通框架检索和本地浏览器检索不应隐式切换，也不应默认并存。

建议后续把检索入口明确拆成：

- `local_browser`：本地浏览器检索，作为默认主路径。
- `framework_fallback`：框架自带检索，只在本地浏览器不可用或用户明确允许 fallback 时使用。
- `off`：关闭检索。

评判：

- 赞成显式开关。
- 隐式切换会破坏来源层级和 receipt 可信度。
- 框架自带检索可以保留，但主要作为本地浏览器检索无法使用时的兜底。

### 7.2 默认仅保留两个子代理

默认只保留两个子代理：

- 项目文档检索子代理。
- 外部检索子代理。

评判：

- 赞成作为硬约束。
- 不默认发展第三类子代理。
- 不因为 Grok、Claude Code、Goose 等框架支持 subagents 就自然扩张成多代理生态。
- 新增任何子代理都必须显式提出、显式评审、显式确定。

### 7.3 中立询问与反例询问必须分开

反例询问和中立询问不是同一个机制。

反例询问：

- 发生在 plan 设计方案写入前。
- 发生在最终结论写入前。
- 目的是防止 plan 或 conclusion 在关键写入点过度自信。
- 不应在执行过程中频繁触发，避免过度质疑和反复打断。

中立询问：

- 发生在执行进程中。
- 目的是辅助模型回看全局方向、任务目标、遗漏输入、局部优化和执行偏移。
- 更接近 runtime orientation aid，而不是 claim/conclusion gate。

评判：

- 赞成明确分离。
- 反例询问是 plan/conclusion gate。
- 中立询问是执行过程中的全局回看辅助。
- 后续嫁接到 Grok workflow 或 UI 时也要保留这两个触发位置差异。

### 7.4 新增专门审查模式

本轮过重自制内容清扫暴露了一个重要案例：过去多次审查看到了局部实现是否成立，却没有充分结合原设计理念判断整体方向是否偏离。

后续应增加一个显式的“全局审查模式”。

日常普通任务：

- 默认只做局部工程审查。
- 关注 bug、测试、实现质量、局部设计。
- 不强制每次都回到全局产品理念，避免拖慢普通开发。

明确进入审查模式时：

- 检查当前实现是否背离原设计理念。
- 检查是否又在自建成熟 runtime 已经提供的能力。
- 检查 UI、检索、工具状态、中立询问、反例询问、反哨兵等关键设计是否被误删或降级。
- 检查本地实现是产品能力、adapter、fixture，还是过度自制。
- 检查与 Grok/其他成熟框架的职责边界是否仍成立。

评判：

- 赞成新增。
- 这不应影响日常普通任务。
- 只有用户明确进入“审查模式”或类似措辞时，才启动全局审查。
- 本轮问题应作为该模式的首个案例记录：局部正确不等于整体方向正确。

### 7.5 远程入口暂缓，先做本地

远程、Web、移动端入口暂不作为当前实现重点。

评判：

- 赞成。
- 当前先把本地 runtime、assurance、TUI、检索和子代理边界理顺。
- 远程入口保留为未来 client 层选项，不影响本轮嫁接判断。

### 7.6 VS Code 内部终端初步适配保留

VS Code 相关内容的当前保留状态应表述为：

- 保留了 VS Code / Codex app server / 内部终端方向的初步会话生命周期观测与规范化材料。
- 当前没有完整 VS Code 插件或完整 IDE 前端适配层。
- 这与本轮目标一致：先保留内部终端和生命周期适配证据，不急着发展成完整远程/IDE 产品线。

主要相关文件：

- `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`
- `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`
- `runtime/codex-app-server-lifecycle-*.schema.json`
- `runtime/fixtures/codex-app-server-lifecycle-v0.1/`
- `runtime/tests/test_codex_app_server_lifecycle_capture.py`
- `runtime/tests/test_codex_app_server_lifecycle_normalizer.py`
- `runtime/tests/test_codex_app_server_turn_probe.py`
- `scripts/capture_codex_app_server_lifecycle.py`
- `scripts/normalize_codex_app_server_lifecycle.py`
- `scripts/probe_codex_app_server_turn_lifecycle.py`

评判：

- 保留是有价值的。
- 它可作为未来 VS Code 内部终端、Codex app server 或本地 IDE session 观测的基础。
- 暂时不把它升级成完整 VS Code 适配项目。

## 8. 当前推荐评估顺序

不要先做大重构。建议按这个顺序评估嫁接价值：

1. Grok `0.2.112` 候选能力审计：确认 subagents/workflows/config/MCP/hooks 是否真能覆盖我们当前自制逻辑。
2. 两个 retrieval subagent 的 Grok profile 草案：只写配置/映射，不写新调度器。
3. 检索与 source visibility 的嫁接路径：先定义显式检索模式开关，再让 Grok 负责调用工具、本仓库负责证据 gate。
4. Tool availability 与 Grok tool registry/permission 的对齐：确认内部工具状态如何返回给 UI 和 prompt。
5. 中立询问和反例询问触发点对齐：反例只进 plan/conclusion gate，中立询问进执行过程回看。
6. Global Progress / Orientation Guard 与 Grok workflow 的对齐：确认 workflow 能否给足够状态用于反哨兵。
7. UI 投影评估：现有 UI 显示哪些 Grok 原生状态，哪些仍显示本仓库 receipt。
8. VS Code 内部终端初步适配复核：只确认生命周期观测价值，不扩展为完整 IDE 产品线。
9. 底部客户端混搭评估：最后再决定本地 TUI、Web、移动、远程入口组合。

## 9. 一句话结论

目前不是“删掉自研设计，换成 Grok”。更准确的方向是：

```text
Grok 等成熟 runtime 负责通用 agent 底座；
本仓库保留检索证据、工具状态、中立反例、反哨兵、UI 和审计保障；
本地优先，客户端和远程入口先保持开放，后面可以混搭。
```

这样既能吸收成熟 agent 框架的省事价值，也不会丢掉真正专门设计过、对用户有用的部分。
