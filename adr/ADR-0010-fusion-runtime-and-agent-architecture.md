# ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整

- 状态：**accepted / frozen**（2026-08-09；本文件是 ORZ 当前自然语言设计的唯一权威基线）
- 冻结版本：1.1（2026-08-10 追加 v1.2 补写，见 §14.2；2026-08-11 追加 v1.3 补写，见 §14.3；2026-08-11 追加 v1.4 补写，见 §14.4；2026-08-12 追加 v1.5 补写，见 §14.5）
- 日期：2026-08-09（v1.1 补充裁决同日冻结）
- 决策范围：产品 runtime 所有权、成熟组件复用、自研准入、主/子 Agent 架构、模型与 transport、工具与权限、检索证据、context/compaction、问询与活性守卫、journal/snapshot、隐私、UI、Windows 兼容性、Schema 演进与设计文档治理
- 取代/修订：
  - **取代** ADR-0003 §2.6 与 §3 中“本仓库不拥有通用 model/tool/session runtime、仅通过窄 adapter 使用外部 runtime”的产品所有权裁决；
  - **保留并重释** ADR-0003 的 assurance contract/runtime boundary：保障事实、receipt、Schema 与 verifier 不得依赖特定 provider 的未公开内部状态；
  - **取代** `存档/docs/design-inputs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` §5.2/§5.10 中“不自建 production model loop/tool dispatcher”的限制；
  - **保留** CN 的成熟实现优先、默认最多两个检索子代理、显式任务边界、中立问询与反例问询分离等约束；
  - **取代 ADR-0005**：输出重复、tool calls、tool variety 和 semantic action 均不再驱动方向问询；方向问询改为 session-level 7 轮触发；
  - **修订 ADR-0008 §2.2**：默认工具轮预算由 40 提升到 120；其 anti-runaway、机械告知、deny 轮计数和最终无工具轮语义继续有效；
  - **保留 ADR-0006、ADR-0007、ADR-0009**：凭据目标、transport retry/timeout 与写入落点继续有效，并由本 ADR 纳入统一基线；
  - **取代** `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` 的 current-authority 地位以及其中与本 ADR 冲突的 thinking、问询、session 所有权和子代理降级表述；
  - **取代** `存档/architecture/pre-adr-0010/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` 中“Grok 完整拥有 session 持久化/恢复”的所有权裁决；其中布局和只读会话投影仍作为 UI 输入；
  - 历史融合、问询、黑板、检索、运行守卫与 Windows 文档的有效规范性内容已转录入本 ADR；原文件之后只保留为设计来源、实施记录或审计证据，不再与本文件共同构成 current design。
  - **v1.1 补充**：恢复显式检索模式、Diagnostic Coverage、Global Review 和 IDE 生命周期证据边界；重裁 Information Sufficiency、来源绑定、`run_tests`、UI 投影、模型/轮次与拒绝熔断语义。
  - **2026-08-09 登记**：ADR-0011 承担受信控制与动作授权面（ACAF）的决策权威；ACAF 派生自本 ADR §2.4、§3.2、§3.8、§4、§5.3、§5.4 与 §11.3，不改变本 ADR 任何既有条款；登记见 §11.8。
  - **v1.2 补充（2026-08-10）**：显式化子代理工具轮预算的 session 累计语义——`continue(requirement_delta)` 重入是同一检索 session 的延续，预算跨 dispatch 累计、不得因重入重置；仅 activation 关闭后新激活从 0 起；主 Agent 维持每 run 独立起算的既有语义。正文见 §3.4.6，索引见 §14.2；来源：GAP-SUBAGENT-RUNTIME 实施审计 D-18（用户裁决）。
  - **2026-08-11 登记（含方向修正）**：C2-1 解禁闭合——外部检索 lane 内 web 工具 Host 直执行（lane 自执行，嵌套门对 `retrieve_project_*` 防递归保留），lane 内豁免 per-call 权限门、授权链由 §3.7.1 显式 mode 门承担（用户裁决）。web_search 执行器 = **DeepSeek 服务端 web search**（Responses API `/v1/responses`，同一把 DeepSeek key，服务端执行搜索——曾提议 xAI Grok 搜索后端独立 key（`orz-grok/search`），被用户裁决否决：检索必须来自当前接入的 provider，不依赖外部检索 API）。凭据无新增（ADR-0006 表不变）。**v1.3 补写 §3.7.10**（2026-08-11，用户裁决升级为正文条款）：「禁止引入独立检索 API 供应商」；正文见 §3.7 条 10，索引见 §14.3；C2-1 解禁与 lane 内权限豁免裁决索引见 §14.3 条 2；实施审计见 `docs/audits/ADR_0006_WEB_SEARCH_CREDENTIAL_AND_C2_1_UNBLOCK_IMPL_AUDIT_2026-08-11.md`。
  - **v1.5 补写（2026-08-12，用户裁决）**：工具可用性机制重构——名级策略过滤废止，模型可见工具列表 = registry 能力目录全量（零可用性承诺），可用性判定完全发生在调用时（permission gate 逐次判定）；ReadOnly/Grill 只读保证由执行层 gate 承担；AVAILABLE 块不再注入 prompt；deny 消息只陈述本次调用事实。正文 §3.5 条 1/2 修订，索引见 §14.5。动机：2026-08-11 TB 复盘（声明层与执行层不一致的"假 available"对 DeepSeek 行为不可预测）。

## 1. 背景

ORZ 最初采用 runtime-neutral、upstream-first 路线：由 Grok Build 等成熟 runtime 拥有模型循环、
工具调度、session 与通用运行时，本仓库只提供 assurance、adapter、evidence、UI 和验证器。
该路线避免了无边界自研，也保留了替换 runtime 的可能性。

后续源码级融合暴露出底层冲突：上游 shell、pager、调度生态、provider transport、权限路径、
Windows 行为和 ORZ 所需的 assurance 语义并不能仅靠窄 adapter 稳定组合。继续坚持“产品 loop 必须由
外部 runtime 完整拥有”，会迫使 ORZ 在 sidecar、hook、post-hoc projection 和上游私有控制流之间
反复补缝，无法可靠保证事件顺序、权限、取消、journal、问询和恢复语义。

因此，ORZ 必须采用**融合架构**：产品控制面由 ORZ 统一拥有，成熟组件尽量复用；只有在现有组件
与产品不变量确实冲突、接口无法承载或维护成本明显高于替换时，才自研或替换。融合不是“全部重写”，
也不是“继续把外部 runtime 当黑盒”；它是对控制面所有权和组件复用边界的重新裁决。

同时，近期问询机制与检索子代理接线暴露了两类退化：

1. 设计退化：方向问询与信息充分性问询被合并，输出重复、tool calls、tool variety 等职责不清的
   判定点被混入同一状态机；
2. 实现退化：检索子代理被降为瞬态、零工具单轮模型调用，并把检索、充分性判断和关闭确认压缩为
   同一次回复；测试反而锁定了错误逻辑。

本 ADR 冻结新的最高层裁决，后续子系统设计必须从这里派生，不再从多份互相冲突的历史文档中拼接。

## 2. 核心决策

### 2.1 ORZ 采用融合产品架构

1. ORZ 产品拥有统一的 agent loop、模型轮次控制、工具调度控制面、session 生命周期、事件顺序、
   取消/恢复、journal 接线和 assurance gate 注入点。
2. Grok Build、Rust 生态或其他成熟项目继续提供经过验证的工具、workspace、sandbox、permission、
   ACP、MCP、hooks、compaction、provider client、解析器和平台能力；它们作为融合架构组件，不再作为
   不可观察的完整产品 runtime 黑盒。
3. ORZ 可以 fork、适配、包装或替换成熟组件，但每项决定必须说明：继承内容、修改内容、替换内容、
   冲突证据、维护责任和验收方式。
4. Assurance 的结构化事实边界继续保持 provider-neutral：不得从模型自述、隐藏思维链、provider
   私有 transcript 或未验证内部状态推导 gate PASS。
5. 本 ADR 撤销“任何自研 production loop 都违反架构”的旧判断；但不授权无边界重写、重复实现成熟
   基础设施或为了形式统一而删除可靠组件。

### 2.2 成熟优先不是口号，而是组件准入顺序

每个能力按以下顺序评估：

1. **直接复用**：接口和语义满足 ORZ 不变量，不修改或仅配置；
2. **薄适配**：成熟实现正确，但需要稳定 adapter/trait/event mapping；
3. **受控 fork**：核心实现可复用，但必须修改底层语义或暴露缺失 seam；
4. **局部替换**：成熟组件与控制面冲突，保留其独立子组件，替换耦合根；
5. **自研**：没有可接受的成熟实现，或复用成本/风险已被证据证明高于自研。

进入第 3–5 档必须留下设计记录。至少回答：

- 哪个产品不变量无法满足；
- 检查过哪些成熟实现或公共 seam；
- 为什么 adapter 不足以解决；
- 新增维护面、升级面和回归面是什么；
- 如何证明没有把 inherited 能力错误记为 ORZ 自研成果。

### 2.3 产品所有权重新划分

| 层 | 当前所有权 | 约束 |
|---|---|---|
| ORZ control plane | agent loop、轮次、工具调度策略、session、取消/恢复、事件顺序、gate 注入 | 统一语义；不得分散到 TUI、hook 或测试 harness |
| Mature components | 工具实现、workspace/VCS、sandbox、permission 基础面、ACP/MCP、provider client、compaction 等 | 优先复用；通过窄公开接口接入；继承/修改分开记录 |
| Assurance contracts | trust、provenance、capability、source visibility、receipt、journal、verifier | 结构化、可重建、provider-neutral；模型输出不等于状态事实 |
| Agent roles | 主 Agent、项目文档检索子代理、外部检索子代理 | 同一架构；角色与任务合同不同，不另造降级 runtime |
| UI | 命令、投影、审批交互、可见状态 | 不拥有执行事实或暗中改写 gate 决策 |
| Evaluation/harness | 任务输入、预算、产物收集、评分 | 不伪装产品能力；评测成绩与平台成熟度分开 |

### 2.4 不可退化的产品不变量

1. 依赖方向保持单向：composition root/UI → host → loop → assurance；成熟组件不得反向依赖 ORZ
   control plane。`LoopHost` 是 loop 与工具、workspace、permission、持久化、hooks、凭据和 MCP 的
   稳定公开 seam。
2. Permission 是执行事实的 hard gate。UI、prompt、模型自述、hook、测试 harness 和 tool result
   都不能绕过或伪造 permission/capability 决策。
3. 工具与环境事实优先通过结构化接口机械呈现，避免把可探查事实变成长篇 prompt 纪律。
4. 模型隐藏 reasoning 不向用户展示、不写入 journal、不进入 gate 判定；journal 记录请求配置、usage、
   工具、公开输出和动作结果。
5. 产品不初始化或外发 telemetry；诊断信息只按显式本地 journal/log 合同产生。
6. Memory 保留为成熟的显式 opt-in 能力，默认关闭；“当前未接线”不等于删除许可。
7. Snapshot 是 mutation evidence，不是 permission gate；snapshot 失败必须记录，但不得把已经获准的合法
   动作自动改判为拒绝。
8. UI 是投影层。主 UI 为 `orz-tui` assurance workbench；Codex TUI/app-server surface 可以作为成熟的
   fallback，但两者必须消费同一 host/loop/journal 事实，不建立第二套产品 runtime。

### 2.5 Global Review 与 IDE 生命周期边界

1. **Global Review Mode 保留**，但只在用户明确要求全局/设计/实现符合性审查或显式执行对应命令时
   激活；日常局部工程 review 不自动升级。全局审查固定检查：原设计理念、当前进度真实性、实现内容
   定位、关键设计是否退化、任务内外边界。activation receipt 只证明模式和范围已激活，不等于审查
   结论，更不能把测试通过自动提升为设计符合。
2. VS Code、Codex app-server 与内部终端的 lifecycle capture、normalization、fixture、Schema 和审计
   材料继续保留，定位为**生命周期观测与未来适配证据**。当前不建设完整 VS Code 插件、完整 IDE
   前端或第二套 session/runtime；若未来扩展为产品入口，需要独立设计审查和明确 client/host 边界。

### 2.6 已落地 UI 投影的处置

Toolbar 压缩和只读 session/run-history 投影继续保留为当前 `orz-tui` presentation baseline：Address/Find
入口折叠到 Toolbar、快捷键保持、会话列表只读扫描 ORZ session/journal projection，不自行拥有执行
事实、permission、session persistence 或 restore。旧文档中“Grok 完整拥有 session/restore”的条款仍被
本 ADR 取代；UI 应读取 ORZ 当前 session index/journal，而不是绑定某个上游私有目录。Toolbar 的具体
按钮、间距和手势属于可演进 UI 设计，不是不可逆产品不变量；改动必须通过 UI 可用性与 projection
符合性审查，但不要求为每次布局调整新建 ADR。

## 3. 主 Agent 与检索子代理同构

### 3.1 同构原则

默认保留一个主 Agent 与两个检索子代理：

- 项目文档/工作区检索子代理；
- 外部来源检索子代理。

三个 Agent 复用同一套：

- agent loop 与模型请求/流式响应架构；
- 模型配置、transport、retry、timeout、cancel、stall 和 wallclock 语义；
- tool registry、tool availability、permission、IPG 和 capability 评估机制；
- context、compaction、blackboard、journal、event、snapshot、archive 和恢复机制；
- orientation、信息充分性、停滞守卫和其他适用的 assurance 机制；
- 错误、terminal、budget 与审计语义。

两个检索子代理允许同时处于 `active`，形成最多双并发：项目文档/工作区检索一席、外部来源检索一席；
每个角色只保留一个 active instance，不动态复制同角色子代理。全局 `web_search` 同时只执行一个，
主 Agent 与外部检索子代理共享同一 semaphore；内部与外部检索 session 仍可并行推进。

三个 Agent 使用完全相同的模型、thinking、transport、具体工具 registry、context、compaction 和预算
默认值；各 session 独立记账，不共享递减余额，也不通过削减能力建立“轻量子代理”。工具自身的安全
timeout、网络限流和全局 `web_search = 1` 属资源所有权约束，不构成 Agent 配置降级。

不得再为检索子代理建立零工具、无 session、无 journal、自由文本假 ledger 或一次性模型调用的
特殊 runtime。子代理不是弱化版模型调用，而是**同构 Agent 在检索任务合同下运行**。

### 3.2 允许的差异

主 Agent 与子代理只允许因职责产生以下差异：

- role/system contract；
- task contract、scope boundary 与交付格式；
- blackboard 写入分区和返回对象；
- parent/child session identity；
- 文件写入权限的 deny-only 检索角色约束。

“仅执行检索任务”由任务合同、scope gate 和结构化结果验收保证，不通过硬编码 `tools: []`、绕过
主运行时或删除基础权限机制实现。新增第三类子代理、常驻多模型调度器或不同 runtime 仍需新 ADR。

子代理继续看到并调用与主 Agent 相同的工具，但所有写动作必须经过同一 permission/capability 路径，
且只允许写入以下逻辑域：自身 blackboard 检索分区、当前任务的检索文档、检索记录存档。session
bootstrap 必须把三个逻辑域解析为绝对路径/对象范围并写入 capability receipt；源代码、普通设计文档、
配置、测试和其他工作区文件默认拒绝。shell、MCP、脚本、软链接、junction 或路径重解析不得绕过该约束。

### 3.3 生命周期与结果

检索子代理至少具有以下状态：

```text
created -> active -> result_ready -> assessing -> awaiting_parent_disposition
                 ^                                      | close -> closing -> closed_resumable
                 |                                      \ continue(requirement_delta)
                 \----------- contract_revision + 1 ----------------/

active -> failed/cancelled -> closing -> closed_resumable
```

要求：

1. 主 Agent 通过显式任务合同创建/唤醒子代理；
2. 一个检索任务内 session 持久存在，允许多轮模型/工具动作；
3. 检索结果先形成并验证结构化 `query_summary`、`source_ledger`、`filtering_log`、
   `organized_response` 与 `raw_source_refs`；
4. 机械 assessment 发生在结果形成之后，不能与首次检索模型调用合并，也不新增子代理模型轮；主 Agent
   随后必须给出结构化 parent disposition：`close` 或 `continue(requirement_delta)`；
5. 只有经过验证的 `close` disposition 才提交 close record 并清空 activation 的 live
   assessment/trigger/dedupe state，但不清零 journal、检索文档或归档；满足
   parent identity 与新任务合同后可以用新的 `activation_id` 恢复；
6. 关闭、失败、取消、预算耗尽和 scope 完成必须产生不同 terminal/receipt 语义；
7. 默认工具轮预算为 120；到限后提供一个无工具最终轮报告部分结果与明确终止原因。

现有 `retrieval-result-v0.1.schema.json` 与 `retrieval-session-close-receipt-v0.1.schema.json` 作为
迁移输入；新 writer 必须使用本 ADR 对应的新版本，旧版本只用于 replay。

### 3.4 统一模型、thinking、transport 与凭据

1. 默认 provider family 冻结为 **DeepSeek 系列**，当前 release line 为 **V4**。本次实现审计观察到的
   current model ID 是 `deepseek-v4-flash`；具体 SKU 通过单一 model registry/config 选择，不在三个
   Agent 构造器中分别硬编码。DeepSeek family 之外的默认 provider 变更需要新 ADR；V4 系列内 SKU
   变更必须留下兼容性、质量、延迟和工具调用回归证据，但不必为每次 SKU 更新新建 ADR。
2. 当前默认模型配置由同一 `ModelConfig` 构造并注入三个 Agent：thinking enabled、
   `reasoning_effort = max`、`max_tokens = 160000`。不得在子代理构造器中复制一份较弱默认值。
3. 所有生产模型轮走 streaming Chat Completions transport；连接、流空闲、总时长、取消和 partial
   output 语义统一。当前 retry/timeout 继承 ADR-0007：非流式 transient request 最多 10 次且 32 秒
   窗口封顶；流式请求不在已见输出后重发；20 秒 idle warning、90 秒 idle timeout、30 分钟流总预算，
   取消与超时严格区分。
4. reasoning replay 只在 provider adapter 单点实现；合法工具轮的空 `reasoning_content` 保持字节语义，
   最终输出为空时按“同请求一次重试 → thinking disabled 一次降级 → 明确失败”处理，不静默吞掉。
5. Credential 继续使用 ADR-0006 的 Windows Credential Manager 注册表：主 Agent
   `orz-deepseek/agent`、外部检索 `orz-deepseek/1`、内部检索 `orz-deepseek/2`。凭据目标不同只用于
   identity/audit，不改变模型能力和配置一致性。
6. Agent 级工具轮预算均为 120、独立计数、模型可见；deny 轮仍计入。每轮剩余量通过尾部机械消息
   提供，不写入随轮变化的 system prompt，以保持前缀缓存稳定。预算是 anti-runaway backstop，不是
   对正常复杂任务工作量的估计。预算按 session（activation 生命周期）连续记账：主 Agent 每 run
   独立起算；检索子代理经 `continue(requirement_delta)` 重入后保持同一 session——已耗工具轮跨
   dispatch 累计，不得因重入重置；只有 activation 关闭（close/失败/取消/预算耗尽）后，下一个新
   activation 才从 0 起算（v1.2 补写，2026-08-10）。

### 3.5 Tool availability、permission 与失败反馈

1. session bootstrap 生成一次 **registry 能力目录**（`tool_availability_check` 目录快照事件）；
   模型可见工具定义列表 = 完整 registry，**零可用性承诺**——可用性判定完全发生在调用时：每次工具
   调用由 permission gate 逐次判定（AllowOnce/Deny）并返回明确结构化结果。tool registry 相同表示
   三个 Agent 具有同一能力目录，不表示每个参数组合都被授权。（v1.5 修订 2026-08-12：原“机械探查
   一次工具状态”语义更新——探查保留为目录快照，不再携带可用性承诺；§3.7.1 检索 mode 门禁对检索
   工具族的 off 投影不受影响，属显式模式语义而非策略过滤。）
2. **名级策略过滤废止（2026-08-12 用户裁决）**：模型可见声明不再按 policy 过滤——Interactive /
   ReadOnly / Benchmark 统一声明完整 registry 目录；可用性/授权在调用时由 permission gate 逐次判定，
   拒绝返回明确结构化原因（“denied by the permission gate for this call”，只陈述本次调用事实，
   不承诺策略级不可用）。ReadOnly/Grill 的只读保证由执行层 gate 承担（ReadOnly policy 拒非读），
   不由可见性承担。**唯一保留的名级排除：MCP 名称（`{server}__{tool}`）**——prefix-spoof 防御
   （slice #16 D2-1），执行层同款 deny 纵深兜底。动机：声明层与执行层不一致的“假 available”对
   DeepSeek 行为不可预测（2026-08-11 TB 复盘：path-tracing 对被拒工具重试 4 次 / gpt2 盲改并声称
   完成）；消除静态声明后模型无法误解不存在的信号。polyglot 烧轮教训（web_search×4，D-3/IP2a 的
   直接动机）由调用时明确拒绝 + 本条 4 连续拒绝熔断承担。子代理 deny-only 写策略不受影响（lane
   门禁在执行层，GAP-SUBAGENT-RUNTIME）。
3. 所有 tool path 必须返回非空 success/error/deny/timeout/cancel 结果；不得让模型从空字符串猜测状态。
4. 保留**同类拒绝连续三轮**的机械熔断，删除“每 run 累计拒绝 10 次”的总量机制。计数单位是已完成的
   tool-call round，不是同一 assistant response 中并列的每个 tool call；只有连续三轮都没有成功工具，
   且归一化 `(tool_name, denial_reason_code, policy_revision)` 相同，才注入一次换策略提示。成功工具、
   denial key 变化或 permission policy revision 变化都会重置连续计数。用户取消、timeout、tool error 与
   permission deny 分开记账。该机制只修正 tool-belief/availability，不触发 Orientation，也不承担总预算
   职责；全局 anti-runaway 已由 120 工具轮预算覆盖。
5. 子代理文件写域继续以 §3.2 为唯一 allowlist，并服从 ADR-0009：blackboard、检索文档与检索记录
   都是 workspace-related A 类写点，默认位于工作区 `.gsa/` 或任务合同显式指定的工作区路径；不得
   落入 B 类本体目录。B 类 runtime 配置/session/memory 仍走安装目录 `grok-home` 与既定降级链。

### 3.6 Blackboard、Mechanical Relay 与 context

Blackboard 是共享的结构化状态视图，采用单写者分区：

| 分区 | 写入方 | 内容 |
|---|---|---|
| plan/workboard | controller/批准后的 plan bridge | goal、步骤、状态、当前步骤和软约束 |
| edit actions | controller | 成功编辑的文件、行变化和时间戳 |
| tool actions | controller | read/edit/terminal/retrieval 等分类动作与时间戳 |
| internal retrieval | 内部检索子代理 | 项目来源、ledger、结构化结果 |
| external retrieval | 外部检索子代理 | 外部来源、ledger、结构化结果 |
| gate log | assurance/controller | gate decision、orientation/check identity |

所有 Agent 可按合同读取，只有指定 writer 可以修改对应分区。不得增加自由随记区或让模型直接写入
controller/gate 分区。Mechanical Relay 只根据结构化 event/function identity 做确定性路由，不调用模型、
不从自然语言猜测事件类型。

Blackboard 对模型采用三层外化：plan 存在时提供跨轮字节稳定的极简状态行；每个工具轮只增量推送本轮
动作/结果摘要；历史通过 `blackboard_read` 按分区和时间范围取用。禁止每轮把完整 blackboard 注入
system prompt。

Context compaction 对三个 Agent 使用同一策略：160K 为节奏候选阈值、目标约 90K、最少间隔 20 轮，
只在候选最终答案后的安全批次间隙执行；250K 为无视冷却的保命阈值。压缩采用确定性整轮丢弃和机械
marker，不调用模型生成摘要。首个工具批次可以通过 `compaction_whitelist_add` 写入最多 16K 字符的
客观任务背景；白名单常驻 preamble、跳过压缩，并随 session journal/retention 记录。

### 3.7 检索证据与外部浏览器边界

1. 检索模式是 session/task-contract 中的显式枚举：`local_browser`、`framework_fallback`、`off`。session
   未授权检索时从 `off` 开始；启用检索时优先选择 `local_browser`。`framework_fallback` 只有在用户或
   parent task contract 明确选择时才能进入，不能因 browser timeout、登录失败、CAPTCHA 或结果不足而
   自动切换。任何 mode transition 都必须产生带旧值、新值、authority 和 reason code 的机械事件。
2. `local_browser` 保留旧 LBR 设计的规范性状态与异常结果，但不冻结旧文档的单一线性步骤、60 秒
   timeout 或具体 tab 数：搜索、打开、读取、PDF 发现/下载/验证/索引、来源验证、清理均为显式状态；
   `LOGIN_REQUIRED`、`CAPTCHA_REQUIRED`、`SOURCE_UNAVAILABLE`、`INVALID_PDF`、`NO_TEXT_LAYER`、
   `PAGE_BLOCKED`、`POLICY_BLOCKED`、`TIMEOUT`、`PARTIAL_EVIDENCE` 等失败不得静默降级为成功。
3. URL policy 在初始导航和每次 redirect 后重新检查；默认拒绝 `file://`、browser internal、localhost、
   private IP、cloud metadata 与未移交的账户/支付/密码管理面。网页内容永远是 evidence，不是 instruction，
   不能扩权、请求无关 tab 或触发本地文件访问。默认只允许预定义读取函数；任意 JavaScript 不进入当前
   MVP，未来如引入必须独立记录、限时限量、仅作用于 Agent-owned tab，且默认禁止表单提交和非 GET 写入。
4. Retrieval result 除 query/filter/organized result 外，每个关键来源必须记录稳定 identity、source type、
   access time、visibility、observed scope、missing scope、最高允许 claim、limitation 和内容 digest（可得时）。
5. `full_text_observed`、`partial_text_observed`、`metadata_only`、`unavailable` 是不同证据等级。未读全文
   不得生成全文级归因，不得用摘要、搜索片段、二手转述或模型记忆关闭反例缺口。
6. 外部检索使用受约束的 browser/web 工具时，只控制自己创建或用户显式移交的 tab；cookies、
   password field、auth header、local storage 和无关 tab 永不返回模型。论文、标准和 PDF-first 来源优先
   下载为内容寻址的本地证据，页文本、metadata 与 extraction record 可重建。
7. 全局 `web_search = 1`、browser tab ownership、站点速率限制和下载大小限制属于检索工具合同；它们
   不削减子代理 session 的模型、thinking、transport 或 120 轮预算。
8. 主 Agent 可以直接使用同一 `web_search`，但不得绕过 source ledger/visibility 规则；通过子代理检索
   的主要价值是隔离上下文、保留原始来源和降低主对话污染，不是把无来源摘要包装成事实。
9. 保留 FIX_PLAN D-1 的“claim-bearing 内容在使用处绑定可定位来源”原则，废止把固定
   `[来源: 路径:行号]` 字符串和 grep 命中当成充分验证。writer 使用稳定 `source_id` 绑定 ledger；本地
   代码可记录 observation-time `path:line`，内部文档优先使用文档 ID + section/anchor，外部来源使用
   URL/document identity + observed scope。renderer 可以显示 `[来源: source_id]` 或展开后的可读定位；
   verifier 必须检查 source identity 存在、可见性等级和 claim 上限，而不只检查标记文本存在。
10. 检索执行器（`web_search`）必须使用当前接入的 provider 的服务端搜索能力——DeepSeek 服务端
    web search（Responses API `/v1/responses`，与主 transport 同一把 key、同一供应商）；**禁止引入
    独立检索 API 供应商**（第二供应商、第二 key、独立计费）。该条为冻结后补写（v1.3，2026-08-11）：
    曾提议 xAI Grok 搜索后端独立 key（`orz-grok/search`），被用户裁决否决——检索是模型 API 的
    组成部分，不依赖外部检索 API；来源：web_search 执行器实施审计（2026-08-11），索引见 §14.3。
11. PDF 下载是**双通道路由**：可配置域名白名单（env `ORZ_PDF_BROWSER_DOMAINS`，逗号分隔、`*` 通配
    一个子域 label——`*.cnki.net` 匹配 `kns.cnki.net` 但不匹配 apex `cnki.net`；未配置=全部走直连）
    决定通道——**命中走浏览器**（CDP 下载，利用操作者经隔离 profile 手动登录的文献库会话），
    **未命中走直连**（web_fetch 同源 HTTP 通道）。白名单内的浏览器下载失败（未登录/付费墙/超时/
    取消）是**显式失败**，**不得自动回退直连**（§3.7.2 显式状态；登录后重试）。两种通道产出的 PDF
    都进入同一内容寻址证据管线（§3.7.6）。该条为冻结后补写（v1.4，2026-08-11）：用户裁决
    「每个学校买的文献库不一样，需要留白名单进行范围确定——白名单的走白名单（登录态），不在
    白名单的自动走直连」；来源：PDF 证据管线实施审计（2026-08-11），索引见 §14.4。

### 3.8 受控 `run_tests` / hidden-test 反馈环

保留 FIX_PLAN D-9 的单 run 测试反馈能力，但重裁其安全和控制语义：

1. `run_tests` 只在 harness/session contract 提供固定 command、cwd、timeout、environment/mount policy 时
   出现在工具声明中；Agent 不能改写命令或读取 hidden test 文件。
2. `run_tests` 是**受控代码执行**，不是 read-only 工具。即使它没有编辑 API，测试进程仍可能写文件、
   访问网络或启动子进程，因此必须经过 execution permission、sandbox/Job Object、timeout、输出上限和
   workspace delta/journal 记录。
3. 模型只接收 exit status、结构化 summary、脱敏且截断的 stdout/stderr tail 和完整输出 artifact identity；
   hidden test source、secret、host path 和无关环境信息不得通过失败输出泄露。完整输出保存在受控任务
   artifact 中，可按 permission 读取。
4. 每次 `run_tests` 调用按 §4.2 计一个 tool-call round。测试通过是实现证据，不自动证明设计符合；测试
   失败允许模型继续修复，但不自动扩大权限或暴露测试源码。
5. 不冻结“harness 自动再跑第二轮”为产品默认。额外迭代必须由 Agent 显式再次调用或由外部 harness
   contract 设置独立、有界的 attempt 数，并在不同 run/attempt identity 下记录。

## 4. 问询与停滞机制职责

### 4.1 机制分层

相关功能分为六个互不吞并的职责面。Information Sufficiency 判定和 Close Record 完全机械化；
Orientation、Diagnostic Coverage、Counterexample 以及主 Agent 的 retrieval lifecycle disposition 可以有
模型参与，但 parent disposition 只选择工作流动作，不得重写机械充分性状态：

| 机制 | 家族/性质 | 触发 | 作用 |
|---|---|---|---|
| Orientation Checkpoint | 中立问询 | session-level 7 轮；pre-handoff 为独立生命周期触发 | 中途回看当前任务、位置与下一目标 |
| Information Sufficiency Assessment | 机械评估/记录 | 检索结果形成后 | 记录来源覆盖、可见性与缺失类别；不调用模型、不自行决定关闭 |
| Parent Retrieval Disposition / Close Record | 生命周期控制与机械 receipt | assessment 形成后 / close commit 时 | 主 Agent 显式 `close` 或 `continue(requirement_delta)`；新需求保持 active |
| Diagnostic Coverage Check | 递进中立问询 | 单个 debug episode 的机械硬信号达到 2→3→4→5 阈值 | 防止连续失败后锁死单一路线；最多引导一个最小补诊断动作 |
| Counterexample Gate | 反例/结论自查 | plan 写入前、正式结论前 | 检查前提、反证和结论强度；不在普通执行中扩散 |
| Runtime Stagnation Guard | 独立机械守卫 | 输出连续/ngram 重复等停滞证据 | restart/handoff；不向模型询问是否停滞 |

Information Sufficiency 不再属于 inquiry family，也不产生模型判定。Assessment 与 Close Record 是可重建
的机械事实；`insufficient`/`indeterminate` 本身不是关闭门禁，但主 Agent 在看到 assessment 后提交的
`continue(requirement_delta)` 是有效生命周期指令，会阻止本次 close commit。

### 4.2 Orientation 当前触发裁决

1. 轮次触发按 **session-level 已完成对话轮**计数；`completed_turns_since_orientation >= 7`
   时，在下一安全动作间隙注入一次 Orientation Checkpoint。
2. 7 轮计数不因 Information Sufficiency、Retrieval Parent Disposition、Counterexample 或普通动作 cooldown
   被清零；只有实际发出 Orientation Checkpoint 后才重新计数。
3. 输出重复不触发 Orientation，只进入 Runtime Stagnation Guard。
4. `tool_calls` 不作为 Orientation 判定点。
5. `tool_variety` 不作为 Orientation 判定点。
6. token 数不作为当前 Orientation 判定点；若未来重新引入，必须有稳定公开 token 度量和独立 ADR。
7. semantic action 不作为 Orientation 辅助触发点：其边界依赖语义解释、难以跨模型与工具稳定复现，
   且会重新引入隐式的工具调用计数。未来只有新的运行证据证明单一轮次触发不足时，才可通过新 ADR
   重新提出，不得以“工具调用一次”等临时代码语义替代。
8. `completed_turns_since_orientation` 的机械单位是**完成的逻辑模型轮**：一次 assistant generation 及其
   必需的 tool-result 回放完成后计 1。含一个或多个 tool calls 的轮计 1；Orientation、Diagnostic
   Coverage、Counterexample 等模型问询的回答轮以及 retrieval parent disposition 所在的主 Agent 控制轮
   也计 1；permission deny 的 tool-call round 仍计 1。
   transport retry/同请求空输出重试不额外计数，单个 assistant response 中的多个 tool calls 也不拆分。
9. compaction、handoff 准备和 session recovery 不清零计数。snapshot/session metadata 持久化该计数；
   恢复后第一个完成轮在恢复值上继续累加。只有实际发出 Orientation 后重置，或创建全新的独立 session
   才从 0 开始。三个 Agent 各自独立计数。

### 4.3 信息充分性

Information Sufficiency 在检索结果形成或 activation 关闭前由 controller/verifier 机械计算，完全不调用
主 Agent 或子代理模型。输出状态为 `sufficient | insufficient | indeterminate | not_applicable`，并只包含
机械可验证事实：

- 来源数量；
- 来源类别/覆盖范围；
- 全文、部分文本、metadata-only、unavailable 的可见性分布；
- 缺失类别与过滤原因；
- 相关 source visibility gate 状态；
- assessment version、task/retrieval contract identity、result/ledger digest 与 reason codes。

这些字段来自结构化 task contract、retrieval result/ledger 和 source visibility gate，不得由模型自由文本
自报，也不得因检索工具返回成功就假定信息充分。若 task contract 没有足够的机械 coverage 要求，必须
返回 `indeterminate`，不能让模型代填判定。assessment 呈现给主 Agent 后不会自动唤醒子代理或自动发起
补检索；充分性状态本身不决定关闭。只有主 Agent 显式提交 `continue(requirement_delta)` 才继续检索。

### 4.4 子代理关闭记录

关闭决定属于主 Agent/controller 的生命周期控制面；Information Sufficiency 状态不是关闭门禁，但 parent
disposition 是 close commit 的前置条件。正常结果路径是：

```text
子代理形成结果 -> 结构化结果验证 -> 机械生成 information_sufficiency_assessment
-> 主 Agent 提交 retrieval_parent_disposition
   -> close: 写 retrieval_close_record -> 关闭 activation -> 清空 activation live state
   -> continue(requirement_delta): 验证合同/权限 -> contract_revision + 1 -> 保持 active -> 新检索
```

`sufficient` 不自动关闭，`insufficient`/`indeterminate` 不自动继续。`close` 是主 Agent 对“当前检索任务
无需追加需求”的显式确认；`continue` 必须携带非空、可验证的 `requirement_delta`，不得仅写“再查一下”。
如果新需求扩大 source、tool、path 或 permission scope，必须重新走 task-contract/capability gate。收到
有效 `continue` 后不能关闭子代理，也不能先 close 再 reopen；原 activation 保持 active，当前 assessment
标记为 consumed/superseded，并以新的 `contract_revision` 进入下一检索循环。

主 Agent 未提交有效 disposition 时进入 `awaiting_parent_disposition`：不关闭、不自动重试子代理，也不
从自由文本猜测决定；controller 可在主 Agent 的下一安全控制轮再次要求结构化 disposition。用户取消、
session cancel、wallclock 或子代理自身 failed/cancelled 等终止 authority 仍可直接产生相应 terminal close。

只有 close commit 后才重置该 activation 的 live assessment、trigger 和 dedupe state；journal、检索文档、
source ledger、result archive、所有 assessment、disposition 与 close receipt 永不因重置删除。相同
contract revision 内以 `(activation_id, contract_revision, result_digest, assessment_version)` 幂等去重；
`continue` 后 revision 递增，因此新结果可以重新触发 assessment；关闭后不跨 activation 抑制新记录。

Disposition 与 close/continue transition 由 controller 单写者串行提交。每个 disposition 必须绑定
`activation_id + expected_contract_revision + assessment_id`：同一 `disposition_id` 重放必须幂等；同一
assessment 的冲突 decision、旧 revision 的迟到 `close` 或新 revision 开始后的旧消息必须拒绝并记录
`stale/conflicting_disposition`。`close` 的 terminal record 与状态切换是一个 commit；`continue` 的合同
revision 递增与保持 active 是另一个互斥 commit，不能出现“新需求已接受但旧 close 随后生效”的竞态。

### 4.5 Counterexample 与运行活性守卫

Counterexample Gate 在 plan 写入前执行一次 plan 变体，在正式答案前执行一次 answer 变体；正式答案
变体必须显式告知“仅出现一次”。它只检查前提、反证和结论强度，不进入普通工具循环、不代替
Orientation、不拥有子代理关闭权。

Orientation 可以读取 checklist/blackboard 中的 current step、task position、next output target 和工具
可用性事实，但不得询问模型“是否错误、是否有偏见、是否漂移、是否卡住”，不得产生 counterexample、
claim disposition 或 hard constraint change。Checklist 是用户可见的 soft workboard，不是 hard gate。

运行活性分为四个相互独立的机械面：

| 守卫 | 默认值 | 模型可见 | 行为 |
|---|---:|---|---|
| Tool execution timeout | 300 秒，可配置 | 返回明确 timeout 结果 | `kill_active` 终止当前进程树但不闩闭后续 spawn，loop 可继续 |
| Stream liveness | 20 秒 warning / 90 秒 idle / 30 分钟 total | 只看到失败/partial 结果 | 中止无进展请求；有 reasoning/content chunk 即刷新 activity |
| Activity stall watchdog | 360 秒，可配置 | 否 | 无 journal/tool/model-stream 活动时写 `run_invalidated{stall}` |
| Max wallclock | host/harness 配置 | 否 | 到点写 `run_invalidated{wallclock}`，保留完整 hash chain 后正常退出 |

Runtime Stagnation Guard 只处理公开输出连续/ngram 重复，是**内容停滞**；Activity watchdog 处理没有任何
可观察进展，是**活动停滞**。两者不能共享 metric 或把 wallclock/timeout 重新包装成问询。所有 Agent
使用同一套守卫默认值；具体工具可以声明更长的安全 timeout，但必须显式、可审计。

### 4.6 Diagnostic Coverage Check

Diagnostic Coverage Check 保留为 debug/problem-solving 专用的递进中立问询，不并入 Orientation、
Information Sufficiency、Counterexample 或 Stagnation：

1. controller 为单个 bug/debug episode 分配稳定 `debug_episode_id`；初始阈值为 2，触发后本 episode
   的硬信号计数清零、下一阈值递增为 3、4、5，并在 5 封顶；bug 明确解决或 episode 显式关闭后恢复
   初始阈值 2。用户说“继续”不重置、不关闭该机制。
2. 只消费结构化硬信号，例如非零测试/命令结果、重复失败 fingerprint、出现新的 error class/stack
   location/reproduction boundary、连续修改集中于同一模块但验证结果未改善、准备扩大 mutation scope
   而诊断证据类别不足。信号必须绑定 event/evidence identity；journal replay 不重复计数。
3. 废止旧草案中不可稳定重放的 `0.5` 主观降噪。新证据通过新 evidence identity 和 failure fingerprint
   变化体现；是否“已吸收进计划”若不能机械验证，不参与 counter。
4. 达阈值后只注入一次中立 checkpoint，要求列出已覆盖面、缺失面和一个最小补诊断动作；它不是 hard
   gate，不要求推翻当前方案，也不自动启动大型复审。该回答轮按 §4.2 计入七轮计数。

## 5. 事件与 Schema 演进

### 5.1 明确类型，不用文案充当协议字段

事件必须具有显式 discriminator。v1.1 至少拆为以下独立 event type 和 payload Schema：

```text
orientation_checkpoint
diagnostic_coverage_checkpoint
information_sufficiency_assessment
retrieval_parent_disposition
retrieval_close_record
```

只有前两者带 `inquiry_family = neutral`，并以 const `inquiry_kind` 与 envelope `event_type` 交叉校验。
Information Sufficiency 与 Retrieval Close 是机械事件，不得带 inquiry family 或模型生成的充分性结论；
Parent Disposition 是主 Agent 的结构化生命周期命令，只能选择 `close|continue`，不能改写 assessment。
不再扩张通用 `neutral_inquiry` 的 `oneOf` 大对象。Counterexample 和 Stagnation 继续使用各自独立 event
type。不得通过文案前缀判断机制；文案用于模型交互，不是稳定协议身份。

### 5.2 Payload 边界

- Orientation payload：session/agent identity、触发点、轮次状态、message block、注入位置；
- Diagnostic Coverage payload：debug episode、threshold stage、去重后的 hard-signal identity、coverage block、
  一个最小补诊断动作；
- Information Sufficiency payload：task/retrieval contract、result/ledger identity、source counts、visibility、
  categories、missing/filtering facts、机械状态、reason codes、assessment version；
- Parent Disposition payload：disposition/parent/subagent/activation identity、assessment identity、
  `expected_contract_revision`、`close|continue`、`requirement_delta`、capability-gate result；`continue` 时
  requirement delta 必填；
- Retrieval Close payload：parent/subagent session、activation/contract/result/assessment/archive identity、
  validated disposition identity、terminal reason、resumable 和 live-state-reset；正常结果 close 必须引用
  `decision=close` 的 disposition，终止 authority close 必须引用 cancel/failure/wallclock reason；
- Stagnation payload：机械 metric、decision、restart/handoff evidence；不得混入 inquiry counters；
- 所有事件必须有明确 producer、consumer、verifier 与 migration version。

### 5.3 迁移原则

1. 先扩展 Schema 和 good/bad fixture，再修改 Rust producer；
2. producer 与 verifier 在同一迁移切片更新；
3. 旧事件版本只用于历史 journal replay，不作为新实现依据；
4. 任何测试不得锁定已知错误或已被裁决废弃的逻辑；
5. 测试名称中的“符合设计/逐字移植”必须引用当前权威设计和版本，不能只比较代码自身常量。

### 5.4 Journal、snapshot 与恢复

1. 产品 journal 使用 bounded background recorder 保持调用面轻量，但事件 append 必须有发送背压和
   acknowledgement；同一 session 内 sequence 单调，writer 串行化 hash chain。
2. 每个已接收事件在返回成功前完成 write、flush 和 `sync_all`。不采用“直到 gate 通过才首次落盘”的
   延迟 persist 旧草案；挂死或进程被杀时，应尽可能保留最后一个已确认事件。
3. Session bootstrap 冻结 agent role、model/provider、capability、source、history mode、parent identity
   和 schema version；恢复不重写旧事件，按 §11.6 建立跨版本链接。
4. 变异类工具在 permission 放行后、`tool_started` 前对可确定目标创建 snapshot，并记录
   `snapshot_created` 或 `snapshot_error`。snapshot 失败属于 evidence degradation，不自动阻断动作；
   无法静态确定目标的 shell 写入必须由 workspace delta/journal 补足证据。
5. Restore/revert 只在用户显式请求并通过相应 permission 后执行，产生独立 restore event/receipt；不得
   把“snapshot 模块存在”写成“恢复路径已完成”。
6. Reasoning text、credential、cookie、auth header 和私有 provider transcript 不进入 journal。必须记录的
   usage/config 只保留结构化数值、枚举、digest 和脱敏错误类别。

## 6. Windows 设计分离

1. `存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md` 第 1–5 节继续只描述早期 process runtime spike，不再承载
   产品级 Windows beta、发行、案例治理或兼容性声明。
2. Windows-first 产品定位、环境矩阵、兼容性边界、安装/升级体验和 beta 要求进入独立的
   `architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`。
3. Windows 证据分为：追加式事故台账、经审计的精选案例、自动回归/人工复核、beta 环境证据。
4. 候选事故不得因写入设计文档就晋级为案例；必须复核 provenance、根因边界、分类、脱敏和回归。
5. `windows_native`、`cross_platform_agent`、`harness_environment` 分开统计；任务跑分不能替代
   Windows 工程成熟度。
6. 在上述制品建立前，状态只能写 planned/not started 或 maintainer-observed，不得写“案例闭环完成”。

Windows beta 的观察面至少覆盖：Job Object/ConPTY/取消与输出 drain；PowerShell 5.1/7、CMD、Git Bash、
MSYS 参数与转义；盘符、UNC、长路径、Unicode、CRLF、链接与文件占用；Credential Manager、
`GROK_HOME` 和写点分层；Git/MSVC/Python/Node/Rust/.NET 工具链；ACL、AppContainer、网络隔离；
Windows 10/11、企业策略、EDR、Docker Desktop/WSL；安装、升级、卸载和诊断体验。

事故晋级案例至少需要稳定 case ID、环境与 binary/commit provenance、症状、预期、复现、原始证据、
根因或明确失败边界、分类、修复归属、剩余限制、回归入口、复核结果和脱敏状态。单一维护者机器通过
只能写 maintainer-observed；正式兼容性声明必须同时提供已验证环境、已知限制和案例/测试入口。

## 7. 设计权威与归档

### 7.1 当前设计权威层级

从高到低：

1. 当前 accepted ADR；
2. current architecture；
3. subsystem contract/Schema；
4. implementation plan 与 acceptance matrix；
5. source code 与测试；
6. audit/incident/case evidence；
7. archive 中的历史设计。

实现与测试不能反向把已废弃逻辑升级为设计。审计和事故是证据，不自动成为产品规范。

### 7.2 归档规则

1. 被取代的设计用 `git mv` 移入统一 archive，保留历史；
2. 归档文件头必须记录原路径、归档日期、最后状态和 `superseded_by`；
3. current index 只路由当前权威，不把 archive 作为默认实现依据；
4. 历史 audit、incident、raw evidence 不因设计过时而删除；它们进入独立 evidence/history 路由；
5. Schema 若仍需 replay 历史 journal，必须保留并标明 legacy/replay-only，不能简单移动后失联；
6. 批量归档前必须做引用扫描，同一提交更新 current index、链接和 supersede 标记。

冻结目标结构：

```text
adr/                                      # 当前 accepted ADR
architecture/current/                     # 仍需独立呈现的 current projection/contract
存档/architecture/pre-adr-0010/        # 已由本 ADR 取代的融合前/过渡设计
存档/architecture/runtime-spikes/      # 早期 process/runtime spike
docs/audits/
docs/incidents/
docs/cases/
存档/docs/design-inputs/
存档/docs/implementation-history/
```

## 8. 后果

### 8.1 正面

- ORZ 对关键控制流拥有明确责任，不再依赖 sidecar/hook 补缝；
- 保留成熟组件价值，避免把融合误解为全量重写；
- 主/子 Agent 共享同一可靠架构，不再出现零工具、无生命周期的降级实现；
- 问询、关闭和停滞职责可被 Schema、事件与测试分别验证；
- Windows 兼容性与早期 spike、任务跑分分离；
- current 设计不再被大量历史文档污染。

### 8.2 代价与风险

- ORZ 正式承担 agent control plane 的长期维护、升级和安全责任；
- fork/适配上游组件需要持续维护差异清单和升级验证；
- 子代理同构会增加 session、工具、权限、journal 和资源治理复杂度；
- Schema 迁移必须兼容历史 journal replay；
- 文档归档和引用更新工作量较大，且错误归档可能丢失关键证据路由；
- 在重整完成前，当前实现存在已知设计偏差，不能因旧测试通过而升级成熟度声明。

## 9. 实施顺序

### Phase A：权威冻结

1. **已完成**：接受并冻结本 ADR；
2. **已完成**：标记 ADR-0003/0005/0008 与历史融合设计的 supersede 状态；
3. **已完成**：建立 current design authority index 和 archive 路由；
4. 从本 ADR 冻结起，历史设计不得直接指导新实现。

### Phase B：设计与 Schema 重整

1. 本 ADR 已承担融合架构唯一自然语言基线，不再另写并列的“current 总设计”；
2. Schema/contract 只细化同构检索子代理 state machine，不得复制或改变本 ADR 的产品语义；
3. 问询、机械 assessment 与生命周期通过版本化 Schema/fixture 细化，不再创建并列的方向 ADR；
4. 扩展 orientation/diagnostic/retrieval assessment/close/event Schema 和 fixtures；
5. 独立编写 Windows platform compatibility design。

### Phase C：实现重构

1. 子代理复用主 Agent runtime；
2. 接入真实检索工具、结构化结果与持久 session；
3. 分离 orientation、diagnostic coverage、mechanical information sufficiency、retrieval close、counterexample
   和 stagnation；
4. 接入显式 retrieval mode、结构化 source binding、受控 `run_tests` 语义和新的 denial breaker；
5. 删除废弃字段、旧 producer 和只为错误逻辑存在的代码；
6. 按新 acceptance matrix 重建测试。

### Phase D：文档归档

1. **首批已完成**：以 §12 处置矩阵和归档 metadata 形成 supersede/引用关系；
2. **首批已完成**：移动 17 份历史设计、设计输入、实现计划与 runtime spike；
3. **首批已完成**：更新项目索引、README 和所有命中旧路径的 current 引用；
4. **文档侧已完成 / 实现侧待 Phase B/C**：已运行旧路径、authority wording 和归档 metadata 检查；Schema、journal replay 与实现测试在 producer/Schema 迁移切片执行。

## 10. 验收条件

本 ADR 已满足以下冻结条件；后续实现验收必须持续保持：

- 明确确认 ORZ 融合 control plane 所有权以及成熟优先准入顺序；
- ADR-0003/ADR-0005/CN 的 supersede 状态无歧义；
- current architecture 不再同时声称“外部 runtime 拥有 loop”和“ORZ 自研 loop”；
- 子代理设计明确同构范围、允许差异、生命周期与结构化结果；
- 问询/评估设计明确六类职责、7 轮 session-level 语义、机械 Information Sufficiency，以及 normal close
  必须由 parent disposition 明确确认；
- 新事件 Schema 有稳定 discriminator，不依赖文案区分；
- Windows 产品设计与 process spike 分离；
- 归档方案保留历史证据、Schema replay 和 Git 历史；
- 已知错误测试已登记为 implementation migration blocker；删除/重建属于 Phase C 验收，不得因旧测试
  仍通过而宣称符合 current contract。

## 11. 配套工程裁决

本节原有七个开放问题及 v1.1 六组补充裁决均已关闭。子系统设计可以细化字段、API 和迁移步骤，但
不得重新打开这些产品语义。

### 11.1 Orientation 只采用轮次触发

不增加 semantic action 辅助触发器。当前唯一常规触发是 session-level 7 个已完成对话轮；pre-handoff
等显式生命周期检查可以使用独立 trigger reason，但不参与七轮计数。理由是 semantic action 需要依赖
模型、工具或启发式分类器解释，无法形成稳定、可重放的机械事实，并会变相恢复已否决的 tool count。

### 11.2 Inquiry 拆为显式事件类型

采用独立的 `orientation_checkpoint`、`diagnostic_coverage_checkpoint`、
`information_sufficiency_assessment`、`retrieval_parent_disposition` 与 `retrieval_close_record` event type 和
payload Schema，不采用单个 `neutral_inquiry` 加 `oneOf`。前两者是中立问询；assessment 和 close record
是机械记录；parent disposition 是结构化生命周期命令。旧 `neutral_inquiry` 和
`retrieval_completion_check` 只用于 v0.1 journal replay，不得由新 writer 产生。

### 11.3 子代理工具、预算、并发和写权限

1. 两个检索子代理可以双并发；并发上限为两个 active retrieval session，项目文档/工作区检索和外部
   来源检索各占一席，不动态生成同角色副本。
2. 子代理使用与主 Agent 相同的模型、模型参数、具体工具 registry、单轮预算和 session 总预算默认值；
   默认工具轮预算均为 120；每个 session 独立计账，主 Agent 不与子代理争用同一个递减余额。
3. 工具一致不等于文件写权限无限。子代理只可写自身 blackboard 检索分区、当前任务检索文档和检索
   记录存档；除此之外的文件写入一律拒绝。
4. 写域通过 session capability receipt 冻结，使用解析后的绝对路径/对象 identity 校验；所有 shell、
   MCP 和脚本间接写入仍走同一门禁，并拒绝通过 symlink、junction、reparse point 或路径穿越逃逸。
5. 内部/外部检索角色各一个 active instance，允许双并发；全局 `web_search` concurrency 固定为 1。

### 11.4 子代理不设置专属资源限制

不为子代理额外设计内存、磁盘、保留期、模型降级或更短 wallclock 上限；它们继承主 Agent 的 session、
archive、compaction、retention、cancel 和宿主安全策略。这里的“不设限”是“不设置子代理专属限额”，
不表示绕过操作系统资源约束、用户取消、全局故障保护或与主 Agent 相同的预算终止语义。

### 11.5 融合组件来源登记

旧 CN 文档 §5.2 的“Grok 完整拥有 model loop/session/tool dispatch”矩阵与本 ADR 的融合 control plane
冲突，**不得原样恢复**。保留“逐 component 记录来源、采用档位和修改边界”的治理方法，并在下一轮
实现审计中建立以下成对制品：

- `upstream/fusion-component-register-v0.1.schema.json`：机械 Schema；
- `upstream/fusion-component-register-v0.1.yaml`：机器可读 current inventory；它是 ADR 投影，不独立改变设计。

截至 ADR v1.1 冻结，这两个文件均不存在，状态必须写 `not_started / audit_required`，不能根据 crate 名、
上游来源或编译通过推定 `inherited`、`modified` 或 `local`。下一轮审计至少逐 crate/component 回看当前
代码、Cargo dependency/feature、公开 seam、调用入口、实际可达性、上游 revision、local diff、license、
测试证据和升级责任人；tool/workspace/VCS、sandbox/permission primitives、ACP/MCP、provider client、
compaction、host/loop/assurance/UI 等能力群都必须覆盖。

每个 component entry 至少包含：稳定 component ID、能力、代码路径、origin kind/project/version/revision/
license、采用档位（direct reuse/thin adapter/controlled fork/local replacement/self-build）、ownership
分类（inherited/modified/replaced/local）、继承/修改/替换路径集合、公开 seam、冲突证据、设计记录、
验证引用、升级责任人与当前状态。三个路径集合必须互斥，lock/candidate 文件只通过引用关联，不复制
版本事实。进入受控 fork、局部替换或自研档位时，register entry 必须引用 §2.2 要求的设计记录。

### 11.6 Journal 版本迁移与 replay

1. 已写 journal 和已发布 Schema 不原地修改；新事件体系进入 `run-event-v0.2.schema.json` 及版本化
   payload Schema，新 writer 只产生当前版本。
2. 同一 hash chain 不混写 envelope 版本。旧 journal 保持字节不变，不通过“迁移”重算历史 digest。
3. offline replay/verifier 通过版本 registry 永久保留所有进入仓库或证据索引的历史版本；不设置按日期
   到期的 replay 兼容期。
4. 产品热路径只向当前版本追加。需要恢复旧 session 时，先只读 replay，再生成 migration receipt 和
   新版本 snapshot/journal；新 journal 记录旧 terminal/event digest，形成可追溯跨版本链接。
5. 旧版本 live-resume adapter 保留到所有被标记为 resumable 的旧 session 已迁移或显式关闭；纯历史
   evidence 之后只由 offline replay 支持，避免永久扩大产品热路径。

### 11.7 Windows 案例首批裁决与目录

正式路由冻结为：

```text
architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md
docs/incidents/windows/                 # 追加式事故与未闭合限制
docs/cases/windows/                     # 审计通过的脱敏精选案例
regression/windows/                     # 自动 fixture、脚本与人工复核入口
.observed-runs/windows/                 # git-ignored 原始运行；案例只引用 digest/manifest
```

首批审计采用保守晋级：

- `GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md` 的 `tool_timeout`、`task_cancel`、`parent_exit` 具有明确
  fixture、result/verification Schema、baseline/candidate 对照、digest 和零残留检查，分别晋级为
  `ORZ-WIN-PROC-001`、`ORZ-WIN-PROC-002`、`ORZ-WIN-PROC-003` 精选案例候选；整理时不得合并为一个
  模糊的“Job Object 已通过”案例。
- `GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md` 保留在事故路由，待 raw artifact identity 和结构化
  verification 补齐后再晋级，不能只凭修复说明与测试总数进入案例库。
- Windows native sandbox 的 raw TCP 残余保留为 open limitation/incident；host firewall 补偿不能被
  重写为 AppContainer 自身完成网络隔离。
- credential hardening 当前主要是 offline implementation evidence，不能晋级为 Windows Credential
  Manager 实机兼容案例，直至真实 credential-read 路径完成脱敏验证。2026-08-11 方向修正后
  web_search 读取路径复用主 DeepSeek key 通道（零化 + `redacted()` 唯一序列化出口，当前生产
  接线=构建时 `tracing::info!(redacted)`），DeepSeek 实机 live 测试已跑通
  （`live_deepseek_web_search_roundtrip`）；Windows 实机晋级仍待精选案例路由（2026-08-11 登记）。

### 11.8 受信控制与动作授权面（ADR-0011）

Authenticated Control and Action Fabric（ACAF）的决策权威由 [`ADR-0011`](ADR-0011-authenticated-control-and-action-fabric.md)
承担，详细设计见 [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。
ACAF 派生自 §2.4（permission hard gate）、§3.2/§3.8（子代理写域与受控 `run_tests`）、
§4.1/§4.2（机制分层与 7 轮机械计数）、§5.3/§5.4（Schema 迁移纪律与 journal/snapshot）与
§11.3（写域解析验证），**不改变本 ADR 任何既有条款**，不得反向削弱本 ADR 的机制分层、
机械判定语义或权威层级。ACAF 为 GAP-DENIAL-POLICY-REVISION 提供必接消费面，并为
GAP-RUN-TESTS RT-001/002/003 建立执行器前置依赖；其实施切片独立于 §9 Phase C 排期，
仅共享"先扩展 Schema/fixture 再修改 producer"的迁移纪律。

## 12. 旧设计条款处置

| 来源 | 处置 | 仍有效内容/说明 | 归档路由 |
|---|---|---|---|
| ADR-0003 | partially superseded | provider-neutral assurance boundary 保留；外部 runtime 完整拥有 loop/session 的裁决废止 | `adr/` 原位保留并加状态标记 |
| ADR-0005 | superseded | 历史阈值仅供 replay/案例解释；当前方向问询以 §4.2 为准 | `adr/` 原位保留并加状态标记 |
| ADR-0006 | inherited | 三个 Credential Manager target 与统一注入方式 | `adr/` current |
| ADR-0007 | inherited | transport retry、stream timeout、cancel/timeout 分离 | `adr/` current |
| ADR-0008 | partially superseded | 40 改为 120；其余预算语义保留 | `adr/` 原位保留并加状态标记 |
| ADR-0009 | inherited | A/B/C 写点与 `GROK_HOME` 降级链 | `adr/` current |
| Fork/Agent Loop/Integrated v0.1 系列 | transferred + superseded | 融合来源、LoopHost、blackboard、relay 保留；旧逐 crate/component 结论待 §11.5 重新审计 | `存档/architecture/pre-adr-0010/` |
| Integrated v0.2 | transferred + superseded | 薄 host、同构 Agent、assurance 分类已转录；旧 runtime ownership matrix、thinking disabled 与旧问询废止 | `存档/architecture/pre-adr-0010/` |
| Implementation Deviation and Correction | evidence_only | sidecar/fork 与早期 inquiry 偏差历史 | `存档/architecture/pre-adr-0010/` |
| CN Agent Base review | transferred + evidence_only | 成熟优先、双子代理、显式检索模式、Diagnostic Coverage、Global Review、IDE lifecycle 边界已转录；不自建 loop 限制废止 | `存档/docs/design-inputs/` |
| Self-Question/Counterexample design | transferred + evidence_only | neutral/counterexample/stagnation 分离和一次性 answer gate 已转录 | `存档/docs/design-inputs/` |
| Inquiry Fix and Blackboard supplement | clause-split | output/tool/action 方向信号废止；blackboard、compaction、whitelist 转录 | `存档/docs/implementation-history/` |
| Run Stall Guards plan | transferred + evidence_only | tool timeout、wallclock、heartbeat 与实施修正已转录 | `存档/docs/implementation-history/` |
| FIX_PLAN 2026-08-06 | clause-split + evidence_only | model/thinking/transport/budget、D-1 来源绑定和 D-9 测试反馈经重裁后保留；D-3 累计拒绝 10 次废止 | `存档/docs/implementation-history/` |
| Retrieval Sub-Agent audit | transferred + audit | task/result/close lifecycle 保留为审计证据；no-model fixture 不代表生产能力 | `docs/audits/` |
| Source Fulltext Visibility rule | transferred | visibility 与 claim 边界已转录 §3.7 | `存档/docs/design-inputs/` |
| Local Browser Retrieval/PDF design | transferred | 显式状态/异常、browser ownership、PDF evidence、URL/JS/prompt-injection 边界已转录；旧线性流程和具体 timeout/tab 数不冻结 | `存档/architecture/pre-adr-0010/` |
| Session Persistence/Layout | clause-split | Grok-owned session 废止；Toolbar 与只读 session/run-history 投影保留为可演进 current UI baseline | `存档/architecture/pre-adr-0010/` |
| Windows Runtime Contract | clause-split | §1–5 为 runtime spike；产品 compatibility 内容转录 §6 | `存档/architecture/runtime-spikes/` |
| Python Reference-Spec Contract | inherited | Schema authority、镜像同步与变更纪律继续有效 | `architecture/current/`（后续移动） |
| audit/incident/raw run | evidence_only | 不因设计转录而删除或升级为规范 | `docs/audits/`、`docs/incidents/`、ignored evidence store |

## 13. 设计与证据来源

当前 accepted ADR：

- `adr/ADR-0003-runtime-neutral-assurance-kernel.md`
- `adr/ADR-0005-neutral-inquiry-thresholds-finalized.md`
- `adr/ADR-0006-credential-target-registry.md`
- `adr/ADR-0007-transport-retry-policy.md`
- `adr/ADR-0008-tool-round-budget.md`
- `adr/ADR-0009-write-placement-policy.md`

历史设计输入（归档后按 §12 路由）：

- `存档/architecture/pre-adr-0010/INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md`
- `存档/architecture/pre-adr-0010/IMPLEMENTATION_DEVIATION_AND_CORRECTION_v0.1.md`
- `存档/docs/design-inputs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- `存档/docs/design-inputs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- `存档/docs/implementation-history/INQUIRY_FIX_AND_BLACKBOARD_PARTITION_2026-08-08.md`
- `存档/docs/implementation-history/RUN_STALL_GUARDS_PLAN_2026-08-08.md`
- `存档/docs/implementation-history/FIX_PLAN_2026-08-06.md`
- `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- `存档/docs/design-inputs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- `存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- `存档/architecture/pre-adr-0010/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md`
- `architecture/PYTHON_REFERENCE_SPEC_CONTRACT_v0.1.md`

Schema 与机械证据：

- `assurance/retrieval-result-v0.1.schema.json`
- `assurance/retrieval-session-close-receipt-v0.1.schema.json`
- `runtime/neutral-inquiry-event-payload-v0.1.schema.json`
- `runtime/orientation-checkpoint-event-payload-v0.1.schema.json`
- `runtime/run-event-v0.1.schema.json`
- `runtime/retrieval-completion-check-event-payload-v0.1.schema.json`
- `assurance/diagnostic_coverage.py`
- `assurance/global_review_mode.py`
- `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`
- `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`
- `orz/crates/orz-tui/src/explorer.rs`
- `orz/crates/orz-tui/src/view_model.rs`
- `orz/crates/orz-loop/src/controller.rs`
- `orz/crates/orz-loop/src/agents/retrieval.rs`
- `存档/architecture/runtime-spikes/WINDOWS_RUNTIME_CONTRACT_v0.1.md`
- `docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- `docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`
- `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`

## 14. v1.1 六项补写裁决索引

本节只提供补写结果索引，规范正文以所指章节为准：

1. **显式检索模式与禁止隐式切换**：保留，见 §3.7；未授权默认 `off`，启用时优先
   `local_browser`，`framework_fallback` 必须显式选择。
2. **Diagnostic Coverage、Global Review、IDE lifecycle 边界**：全部保留，见 §4.6 与 §2.5；分别属于
   debug 递进检查、显式全局审查和 evidence-only client lifecycle 材料。
3. **D-1、D-9、Local Browser 与旧 UI**：均为 clause-split 保留，见 §3.7、§3.8、§2.6；保留产品
   价值，废止 grep-only 引用验证、`run_tests=read-only`、旧固定 timeout/线性流程和 Grok-owned session。
4. **Information Sufficiency 与子代理关闭**：充分性 assessment 完全机械，状态本身不决定关闭；主 Agent
   必须显式 `close`，若返回 `continue(requirement_delta)` 则保持子代理 active，见 §4.1、§4.3、§4.4
   和 §5；只有 close commit 后重置 live state，归档证据不清除。
5. **逐 crate/component 采用矩阵**：旧矩阵不恢复，下一轮按当前代码重新审计，见 §11.5；当前状态
   `not_started / audit_required`。
6. **模型、轮次与拒绝熔断**：默认 DeepSeek family、当前 V4；tool-call/inquiry/recovery 后轮次均按
   §4.2 计数；删除累计拒绝 10 次，只保留同类拒绝连续三轮熔断，见 §3.4、§3.5。

### 14.2 v1.2 补写裁决索引（2026-08-10）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **子代理预算 session 累计**：`continue(requirement_delta)` 重入是同一检索 session 的延续——120
   轮工具预算跨 dispatch 累计（`ActivationState.tool_rounds_used` 读入 `LoopProfile.initial_tool_rounds`，
   循环结束写回），不得因重入重置；仅 activation 关闭后新激活从 0 起；主 Agent 维持每 run 独立起算
   的既有语义（ADR-0008/GAP-TOOL-BUDGET 未改）。见 §3.4.6；来源：GAP-SUBAGENT-RUNTIME 实施审计
   D-18（用户裁决 2026-08-10）。

### 14.3 v1.3 补写裁决索引（2026-08-11）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **检索执行器禁止引入独立检索 API 供应商**：`web_search` 执行器必须使用当前接入的 provider 的
   服务端搜索能力（DeepSeek 服务端 web search，Responses API `/v1/responses`，与主 transport 同一把
   key、同一供应商、同一计费面）；禁止第二供应商/第二 key（曾提议 xAI Grok 搜索后端 `orz-grok/search`，
   被用户裁决否决）。见 §3.7 条 10；来源：web_search 执行器实施审计（2026-08-11，用户裁决升级为
   正文条款）。
2. **C2-1 解禁与 lane 内权限豁免（2026-08-11 登记裁决，未升级正文条款）**：外部检索 lane 内 web 工具
   由嵌套派发门拒绝改为 Host 直执行（`lane_self_execute`，嵌套门仅对 `retrieve_project_*` 保留防递归
   语义）；lane 内自执行豁免 per-call 权限桥（无 PermissionRequested/PermissionDecision 事件），授权链
   由 §3.7.1 显式 mode 门承担——web 族仅 `framework_fallback` 可执行（新门
   `retrieval_mode_requires_framework_fallback`）、off 门覆盖、mode transition 事件 journaled；主 Agent
   直用 web_search 的既有无 per-call 权限门语义不变（§3.7 条 8）。见 §3.7.1、§3.7 条 8；来源：
   web_search 执行器实施审计（2026-08-11，用户裁决，D-5）。

### 14.4 v1.4 补写裁决索引（2026-08-11）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **PDF 下载双通道路由（用户裁决升级为正文条款，§3.7 条 11）**：域名白名单
   `ORZ_PDF_BROWSER_DOMAINS`（逗号分隔、`*` 通配一个子域 label、未配置=全直连）命中 → 浏览器 CDP
   下载（操作者隔离 profile 手动登录的文献库会话）；未命中 → 直连（web_fetch 同源通道）；白名单内
   浏览器失败显式失败不自动回退直连（登录后重试）。两种通道产出进入同一内容寻址 PDF 证据管线
   （§3.7.6：`{cwd}/.gsa/pdf-evidence/{sha256[..2]}/{sha256_full}/`，页文本/metadata/extraction record
   可重建）；`pdf_read(document_id, page_range)` 读取已入库证据（≤20 页/调用，跨 run 复用，retrieval
   mode 门禁）。来源：PDF 证据管线实施审计
   `docs/audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md`（2026-08-11，用户裁决 D-1/D-2/D-3/D-4）。

### 14.5 v1.5 补写裁决索引（2026-08-12）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **工具可用性机制重构（用户裁决，§3.5 条 1/2 修订）**：模型可见工具列表 = registry 能力目录
   全量，零可用性承诺；可用性判定完全发生在调用时，permission gate 逐次判定（AllowOnce/Deny）并
   返回明确结构化结果。名级策略过滤（D-3/IP2a）废止——Interactive/ReadOnly/Benchmark 统一全量目录；
   ReadOnly/Grill 只读保证由执行层 gate 承担；唯一保留名级排除 = MCP `{server}__{tool}`（prefix-spoof
   防御，slice #16 D2-1）。AVAILABLE 块（`[TOOL_AVAILABILITY v0.1]`）不再注入 prompt；deny 消息改为
   只陈述本次调用事实（“denied by the permission gate for this call”）。`tool_availability_check`
   事件保留为 registry 目录快照（审计面）。动机：2026-08-11 TB 复盘——声明层放行而执行层拒绝的
   “假 available”对 DeepSeek 行为不可预测（path-tracing 对被拒 run_terminal_cmd 重试 4 次 / gpt2
   盲改并声称完成）；polyglot 烧轮教训由调用时明确拒绝 + §3.5.4 连续拒绝熔断承担。实现：orz
   `tool.rs`（policy_refuses 缩减为仅 MCP 防御）、`controller.rs`（全量目录 / run_tests 声明条件
   简化为 host 携带 runner / deny 措辞）、`agent_loop.rs`+`prompt.rs`（AVAILABLE 块删除）；测试锁定
   （orz-loop 174/0/3、orz-bin 6+13 capture 全绿）。
