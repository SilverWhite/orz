# ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整

- 状态：**accepted / frozen**（2026-08-09；本文件是 ORZ 当前自然语言设计的唯一权威基线）
- 冻结版本：1.1（2026-08-10 追加 v1.2 补写，见 §14.2；2026-08-11 追加 v1.3 补写，见 §14.3；2026-08-11 追加 v1.4 补写，见 §14.4；2026-08-12 追加 v1.5 补写，见 §14.5；2026-08-12 追加 v1.6 补写，见 §14.6；2026-08-13 追加 v1.7 补写，见 §14.7；2026-08-13 追加 v1.8 补写，见 §14.8；2026-08-14 追加 v1.9 补写，见 §14.9；2026-08-14 追加 v1.10 补写，见 §14.10；2026-08-14 追加 v1.11 补写，见 §14.11；2026-08-14 追加 v1.12 补写，见 §14.12；2026-08-14 追加 v1.13-v1.15 补写，见 §14.13-§14.15；2026-08-15 v1.15⑧/⑨ 补强，见 §14.15 ⑧/⑨；2026-08-15 追加 v1.16 补写，见 §14.16；2026-08-15 追加 v1.17 补写，见 §14.17；2026-08-16 v1.17⑩ 审查收口登记，见 §14.17⑩；2026-08-16 v1.17⑪ S4 实施登记，见 §14.17⑪；2026-08-16 v1.17⑫ 审查收口登记，见 §14.17⑫；2026-08-16 v1.17⑬ 超时语义复核登记，见 §14.17⑬；2026-08-16 v1.17⑭/⑮ 决策门与阶段 A 登记，见 §14.17⑭/⑮；2026-08-16 v1.17⑯ 阶段 A 审查收口登记，见 §14.17⑯；2026-08-16 v1.17⑰ 阶段 B 实施登记，见 §14.17⑰；2026-08-16 v1.17⑱ 阶段 C 实施登记，见 §14.17⑱；2026-08-16 追加 v1.18 补写，见 §14.18；2026-08-17 追加 v1.19 补写，见 §14.19；2026-08-17 追加 v1.20 补写，见 §14.20；2026-08-17 追加 v1.21 补写，见 §14.21；2026-08-17 追加 v1.22 补写，见 §14.22；2026-08-17 追加 v1.23 补写，见 §14.23；2026-08-18 追加 v1.24 补写，见 §14.24；2026-08-18 追加 v1.25 补写，见 §14.25；2026-08-18 追加 v1.26 补写，见 §14.26；2026-08-18 追加 v1.27 补写，见 §14.27；2026-08-18 追加 v1.28 补写，见 §14.28）
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
  - **v1.6 补写（2026-08-12，用户裁决）**：检索来源质量三层结构——机械来源梯队（白名单=政府/机关单位 1.1、命中直接采纳；白名单外默认 1.0；劣质源 0.7，初始含 CSDN/知乎/百家号/B 站个人专栏/独立新闻媒体/自媒体新闻号与财经号/小站）+ 选择性原文核验（framework_fallback 用 web_fetch、local_browser 直接 browser_read，**二存一禁止混用**）+ 子代理模型加权标注（v0 标注排序不拦截）；共享判定器进 evidence ledger/visibility。正文 §3.7 条 12，索引见 §14.6。
  - **v1.7 补写（2026-08-13，用户裁决）**：公众号主体级白名单经评估后**撤回**（收益太小）——`mp.weixin.qq.com` 保持劣质源 0.7，不设主体级升档；微博全站（`weibo.com`）入劣质源 0.7；适用范围：三层结构仅用于 `web_search`（framework_fallback），`local_browser` 直接分级加权（第一层+第三层，无第二层）。正文 §3.7 条 12 修订，索引见 §14.7。
  - **v1.8 补写（2026-08-13，用户裁决）**：工具可见性由 v1.5「registry 全量、零可用性承诺」修订为 v0.2 单一探针面——主 Agent 工作工具统一由每动作机械探针治理：本轮模型可见 = 机械链路完整 ∩ 会话声明集，仅工具名、不标注状态，链路不完整者确定不可提议；全部工作工具不设免检面；调用时机械判定仍为最终兜底；检索车道工具不参与主探针矩阵。正文 §3.5 条 1/2 修订，索引见 §14.8；来源：`docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md` v0.2（2026-08-13 用户裁决，A+C→B 定档）。
  - **v1.9 补写（2026-08-14，用户裁决）**：缓存与上下文成本收敛——保持 v1.8 探针可见性机制不动（工具集变化即真实状态变化，接受其前缀缓存 miss）；否决 per-window 探测、预热轮、工具层后置渲染与工具层 1K 压缩（收益不抵机械化保障与准确性风险）；确定三项值得做：请求 header 变化留痕、探针准确性与稳定性优先、单轮工具结果注入预算 + 策略化读取。正文 §3.5 条 6/7、§3.6 修订，索引见 §14.9；来源：2026-08-14 缓存与上下文成本设计评审。
  - **v1.10 补写（2026-08-14，用户裁决）**：压缩机制重设计——有效窗口 384K（DeepSeek V4 检索质量塌陷最低值）、160K 普通触发 / 200K 兜底；工具调用记录每轮机械坍缩（零模型调用）；五段模板摘要（≤17K 字符、derived_unverified、冷却 3 步、摘要链只进审计）；滚动回查 marker；推翻 2026-08-08「零模型摘要」与「节奏压缩仅最终答案间隙」裁决；恢复缺口 D2-2/D3-1 为实施前置。正文 §3.6 修订，索引见 §14.10；来源：2026-08-14 压缩机制设计评审。
  - **v1.11 补写（2026-08-14，用户裁决）**：检索机械控制裁决登记——候选核验预算 `ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8（web_fetch 家族与 browser_read 共享 per-activation 计数域）；主 Agent 不执行检索任务，主车道模型可见投影移除 `browser_read`、检索子代理投影从 host registry 恢复；候选门禁决策先于权限/ACAF 票据门禁（拒绝无 ToolStarted），但计数消费在权限与票据通过后、执行前提交，被后置门禁拒绝的调用不消耗预算（2026-08-14 审查修复）。索引见 §14.11；来源：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`。
  - **v1.12 补写（2026-08-14，P0-B 步骤 6 实施登记）**：检索机械控制提示词面收敛——主 Agent 提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"（设计 §3.3 语义）；检索子代理提示词移除"候选 ≤5 / never the full reference list"软约束，改指机械预算反馈（"候选 N/M，剩余 K"）；来源加权/引用规则段落去冗余。索引见 §14.12；来源：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`。
  - **v1.15⑧ 补强（2026-08-15，黑板 plan epoch 全面复查 F1/F3 处理，用户裁决）**：plan_epoch 改为时间戳单调编号（unix 毫秒为基底，`next = max(now_ms, 磁盘现存 max + 1)`）——时间戳进入编号本身而非仅文件名，7 天 retention 全量清扫后编号不复用，marker/`blackboard_read epoch` 参数跨窗口仍唯一；身份不变式强制（一一对应、无误用可能）：同 plan_id 修订必须沿用同 plan_epoch、新 plan_id 必须使用严格更大的 plan_epoch，违反在 `rotate_to_plan`/`try_with_plan` 返回错误（`with_plan` fail-fast），拒绝先于任何黑板变更；retention 对 `.gsa/blackboard` 清扫时保留最高编号快照（恢复入口），长生命周期黑板不因当前 epoch 文件超龄而失去恢复。正文见 §3.6 注记，索引见 §14.15 ⑧；来源：2026-08-15 全面复查（F1/F3）与用户裁决。

  - **v1.15⑨ 补强（2026-08-15，黑板 plan epoch 复查遗留 F2/F4-F7/F9/F10 处理）**：epoch 快照写盘原子化（临时文件+rename、恢复回退加载，F2）；epoch 分配加 `.claim-<n>` 原子占号防跨进程撞号（F4）；路径槽溢出指针取 controller 归档目录单一来源（F5）；`blackboard_read` 非法 epoch 显式报错（F6）；归档写失败新增 v0.2 `epoch_archive_write_failed` 事件入事件面（F7）；`EpochSnapshot.rotated_at` 更名 `persisted_at` 并 serde alias 兼容（F9）；设计 §5 恢复措辞对齐 ADR（F10）。索引见 §14.15 ⑨；来源：2026-08-15 全面复查遗留清单（BACKLOG 6e）与用户处理指示。
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

1. **单一探针面（v1.8 修订 2026-08-13，取代 v1.5「registry 全量 + 零可用性承诺」）**：session
   bootstrap 生成一次 **registry 能力目录**（会话声明集）；每个模型请求构造前对全部主 Agent 工作
   工具重算机械链路快照，本轮模型可见 = 机械链路完整 ∩ 会话声明集，**仅工具名、不标注状态**；
   链路不完整者确定不可提议。探针是“探针时刻”的确定判定，不是调用成功承诺——调用时机械门禁仍是
   最终兜底：每次工具调用由 permission gate 逐次判定（AllowOnce/Deny）并返回明确结构化结果。
   `tool_availability_check` 事件只在状态翻转时发出（run-start 空映射 → 当前快照为首翻，先于
   run_started）。tool registry 相同表示三个 Agent 具有同一能力目录，不表示每个参数组合都被授权。
   检索车道工具（web_search/web_fetch/browser_read/pdf_read/project_doc_index）由子代理确定性
   留痕与既有声明门治理，不参与主探针矩阵；§3.7.1 检索 mode 门禁 off 投影不受影响。
2. **单一探针面取代名级过滤与三面分类（v1.8 修订 2026-08-13）**：v1.5“统一声明完整 registry
   目录”与 v0.1 三面分类（恒声明/探针过滤/固定列表）废止——全部主 Agent 工作工具统一先探后列：
   本轮模型可见 = 机械链路完整 ∩ 会话声明集，仅工具名、无状态标注；不完整即移除（确定不可提议）。
   Interactive/ReadOnly/Benchmark 由探针快照驱动各自可见集（ReadOnly 下写探针不完整即移除）；
   ReadOnly/Grill 只读保证由执行层 gate 承担（ReadOnly policy 拒非读），不由可见性承担；写探针为
   metadata-grade，不构成写权限承诺。拒绝消息只陈述本次调用事实，主车道兜底消息与探针 reason 使用
   中性陈述（不使用 available/unavailable 等判定词；事件 error 码、机器 reason 与明确事实性内容
   不受此限）。MCP 名称（`{server}__{tool}`）prefix-spoof 防御保留（slice #16 D2-1），执行层同款
   deny 纵深兜底；`use_tool` 机械链路 = MCP/能力注册存在且会话作用域内。动机（v1.5 保留）：声明层
   与执行层不一致的“假 available”对 DeepSeek 行为不可预测（2026-08-11 TB 复盘）；polyglot 烧轮
   教训由调用时明确拒绝 + 本条 4 连续拒绝熔断承担。子代理 deny-only 写策略不受影响（lane 门禁在
   执行层，GAP-SUBAGENT-RUNTIME）；检索车道与主车道非工作工具保持各自既有声明规则。
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
6. **缓存代价与请求 header 留痕（v1.9，2026-08-14）**：保持 v1.8 的声明面翻转语义——探针每
   模型请求前重算，列表只在真实状态变化时翻转；工具集变化会打穿 DeepSeek 前缀缓存（工具块由
   服务端模板前置渲染，变化即全量 miss）属接受代价，第二轮同前缀请求自动恢复命中。禁止为缓存
   牺牲机械化保障：per-window 探测、预热轮、工具层后置渲染、工具层压缩均否决。每个模型请求的
   header（system+tools 摘要 + config + 变化原因）变化时在 journal 留痕，用于审计翻转、归属
   miss 与核对探针准确性。（v1.19，2026-08-17：console 默认面下注册板块/工具栏为黑板数据、
   经 `blackboard_read` 取回，不参与 tools 摘要，工具栏刷新不构成前缀 miss 源；header 变化
   仅剩 console↔direct 模式切换与只读工具探针翻转等真实状态变化，见 §14.19。）
7. **探针准确性与稳定性优先（v1.9，2026-08-14）**：探针目标是“真实变化才翻转、误判最少”；
   翻转事件与 header 留痕配合，可事后核对每次翻转是否真实合理；可选后端接线时须同步补翻转
   测试（既有边界）；不为缓存调整探针语义或降低 fail-closed 程度。

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

黑板是控制面状态视图，不是内容缓冲：任何大内容（如文件全文）不得写入黑板；黑板/结果栏只放
指针（path/document_id/size/digest/offset），内容本体留在盘上或内容寻址证据区，避免 epoch
快照膨胀与过期副本（v1.22，2026-08-17，见 §14.22）。

Context compaction 对三个 Agent 使用同一策略（v1.10，2026-08-14）：有效窗口按 DeepSeek V4 检索
质量塌陷最低值定为 384K；实测 prompt tokens ≥160K 触发模板摘要（目标=五段模板 ≤17K 字符 +
白名单 + 最近尾），≥200K 为无视冷却的兜底触发（384K − 160K completion 预算 − 24K 余量）。工具调用记录
每轮机械坍缩为动作台账行（工具、目标、结果指针/digest，零模型调用、无冷却），完整记录保留在
journal/侧车审计面；模板摘要采用五段固化结构（目的/计划/变动文件路径/注意事项/后续衔接，字符
上限 3/3/5/3/3K=17K），目的/计划/路径由黑板机械填充，注意事项/后续衔接由模型生成并标
derived_unverified；摘要冷却 ≥2 模型轮（v1.14 审查修复，防长动作累积），摘要链只进审计存档、
不继承旧语义，滚动单 marker 附回查清单。v1.14 审查修复（2026-08-14）追加：缩减守卫不满足时
跨触发轮重试 ≤3 次、仍失败则强制执行一轮压缩并以 `guard_failed` 显式报告机制失败（不使用原始
机械截断）；每次成功会话（主车道与检索子代理一致）在结束时对全量消息估算超过
`session_end_trigger_tokens`（默认 160K）的会话自动执行一轮压缩，把摘要 marker
固定进 sidecar 一起存档（恢复治本，D2-2 恢复预检保留为旧会话兜底）；路径槽按 Top-40+5K 字符
封顶且为「本 plan epoch 增量」，溢出指针指向当前 epoch 快照；压缩不再清空/滚动黑板
（v1.15，2026-08-14：黑板生命周期按 plan epoch 轮换，与压缩解耦，见 §14.15；v1.15⑧，2026-08-15：
plan_epoch 为时间戳单调编号且身份一一对应强制、retention 保留最高编号快照，见 §14.15 ⑧）；退化守卫改为 ORZ 自定
300 等效字符（CJK 一字折算 2 等效字符，替代 orz-compaction 英文向 500 字符门）；摘要存档写失败
显式重试 ≤3 次并在事件/marker 中报告；摘要调用设 120s 专用超时。首个工具批次可以通过
`compaction_whitelist_add` 写入最多 16K 字符的客观任务背景；白名单常驻 preamble、跳过压缩，并随
session journal/retention 记录。详见 §14.10/§14.14 与 `docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`。

每个模型轮的新注入工具结果设置 token 预算（v1.9，2026-08-14）：默认 50K、可经 env
`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 调整，按模型轮累计（并行批内多结果累加），超限拒绝本批
后续调用并显式提示用 offset 续读；提示词层采用策略化读取（grep/结构提取优先、证据关键文件
才全文、大文件 offset 分段），与压缩阈值共同控制上下文增长与单轮缓存 miss 量。

读取工具契约升级为有界返回（v1.22，2026-08-17）：超过粗门的文件返回读取句柄信封
（path、size、encoding、content_sha256、可用范围、有界预览 ≤2–4KB、truncated、
offset 续读指针），不返回全文；小文件保持全文一次返回（一次往返）。粗门默认 16KB、
可配 8–32KB；精门=50K 注入预算为最终兜底。语义适配留在模型，助理层只提供机械原语
（引用 + 范围读），提示词策略化读取（grep/结构提取优先、证据关键文件才全文、大文件
offset 分段）落成工具契约；与 pdf_read 的 `document_id` + `page_range` 先例对齐
（文本文件用 path + offset）。详见 §14.22。

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
12. 检索来源质量：`web_search`（framework_fallback）使用**三层结构**，`local_browser`
    直接**分级加权**（v1.6，2026-08-12 用户裁决；v1.7 修订，2026-08-13：公众号主体级
    白名单已撤回、微博全站入劣质源、三层仅限 web_search；`framework_fallback` 与
    `local_browser` 二存一，禁止混用/隐式切换）：① 机械来源梯队——白名单（政府与
    机关单位，命中即满足权威性、子代理可直接采纳）weight 1.1、白名单外默认 1.0、
    劣质源 0.7（初始含 CSDN、知乎、百家号、哔哩哔哩个人专栏、微博、独立新闻媒体、
    自媒体新闻号与"XX财经"类自媒体号、小型个人站点；黑名单/营销域名与个人新闻号/
    非认证号，账号级由模型层判定；weight 为相对排序乘数，允许 >1，不作 0-1 置信度
    解释），域名级白名单不硬编码；② 选择性原文核验——**仅 web_search 使用**：引用经
    机械预筛+模型初选后，仅对高价值/结论依赖候选用 `web_fetch` 抓原文，禁止全量抓取；
    `local_browser` 不套第二层（`browser_read` 页面读取即原文）；③ 子代理模型加权标注
    ——账号认证状态判断、weight+理由输出，v0 为标注+排序不硬拦截（web_search 与
    local_browser 均使用）。抓到的原文走同一套加权并进 evidence ledger/visibility；
    `output_text` 只作线索不作证据。该机制属结果质量层，不改变 ACAF 授权边界
    （ADR-0011 D-12/D-13）。来源：检索来源加权设计文档（2026-08-12），索引见 §14.6。

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
   时，在下一安全动作间隙暂停并进入**强制模板轮（checkpoint 轮）**——本轮不派发任何
   工具，模型只输出问询模板答案，机械校验通过后才恢复动作（v1.16 修订，取代旧「注入
   文本、循环继续」表述；旧注入块 v0.2 文本同时退役为 v0.3 模板块）。
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
10. 强制模板轮（v1.16，2026-08-15）：模板字段为 `task_position`（必填，≤400 字）、
    `progress_evidence`（数组，可空）、`blockers`（数组，可空）、`next_action`
    （`continue|adjust|gather_evidence|ask_user|handoff`）、`changed_direction`
    （bool），以及条件字段 `missing_evidence`（`next_action=gather_evidence` 时必填
    非空）。非法/未知字段丢弃并记 journal（`checkpoint_response` 事件
    `ignored_fields`）；机械校验=必填/枚举/长度；失败给一次错误反馈重填；仍失败→按已填
    部分机械降级 + journal 记录（事件含 validation 结果与降级原因），不挂死。
11. 范围与计数：主车道（Orientation 与 DC 两族共用同一机制；检索车道保持注入后继续的
    旧行为）。checkpoint 轮计入已完成逻辑模型轮；仅实际完成模板轮（accepted 或
    degraded）才重置计数/推进 DC 阶段；同一安全间隙两族同时到期时 Orientation 优先，
    DC 在下一安全间隙再触发（一次只排一个 checkpoint 轮）。
12. 缓解必做（强制表达、不验证诚实）：`progress_evidence`/`missing_evidence` 与
    journal 证据身份做存在性交叉校验——结果（found/missing 身份列表）随
    `checkpoint_response` 事件记录，不阻断；`next_action=gather_evidence` 必须给出
    缺失证据面（缺失为校验错误）。

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

当前派生设计输入与实施证据（v1.2–v1.8 补写引用；只作来源路由，不新增裁决语义）：

- `docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md`（§3.5 v1.8 单一探针面来源）
- `docs/RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md` 与 `docs/SOURCE_QUALITY_SEED_LISTS_2026-08-12.md`（§3.7 条 12 v1.6/v1.7 来源）
- `docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`（§3.7 条 12 第二层与引用纪律的 P0-B 派生设计）
- `runtime/run-event-v0.2.schema.json`
- `runtime/retrieval-result-event-payload-v0.2.schema.json`
- `runtime/tool-availability-check-event-payload-v0.2.schema.json`
- `runtime/source-quality-seed-lists-v0.1.json`
- `orz/crates/orz-loop/src/tool_probe.rs`
- `orz/crates/orz-assurance/src/source_weighting.rs`

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

> 复核注（2026-08-13）：本条「registry 全量 + 零可用性承诺」的可见性语义已由 §14.8 条 1 修订
> （v0.2 单一探针面：本轮模型可见 = 机械链路完整 ∩ 会话声明集）；调用时 permission gate 兜底
> 语义保留。阅读时以 v1.8 正文 §3.5 条 1/2 为准。

### 14.6 v1.6 补写裁决索引（2026-08-12）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **检索来源加权与原文核验（用户裁决升级为正文条款，§3.7 条 12）**：三层结构——
   ① 机械来源梯队（白名单=政府/机关单位 1.1、命中直接采纳；白名单外默认 1.0；
   劣质源 0.7，初始含 CSDN/知乎/百家号/B 站个人专栏/独立新闻媒体/自媒体新闻号与财经号/小站；相对排序乘数、允许 >1，
   域名级白名单不硬编码）；② 选择性原文核验
   （framework_fallback 下 web_search 引用用 web_fetch 核验、
   local_browser 直接对 browser_read 原文加权核验，两模式二存一禁止混用，禁止全量抓取）；
   ③ 子代理模型加权标注（账号认证状态判断、weight+理由，v0 标注排序不拦截）。
   共享判定器进 evidence ledger/visibility；`output_text` 只作线索不作证据。来源：
   `docs/RETRIEVAL_SOURCE_WEIGHTING_DESIGN_2026-08-12.md`（2026-08-12，用户裁决）。

> 复核注（2026-08-13）：本条 ① 的劣质源名单与适用范围已由 §14.7 条 1/2 修订
> （微博全站 0.7、公众号主体级白名单撤回、三层结构仅用于 web_search）；
> 阅读时以 v1.7 正文 §3.7 条 12 为准。

### 14.7 v1.7 补写裁决索引（2026-08-13）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **来源加权名单修订（2026-08-13，用户裁决，§3.7 条 12 ①修订）**：① 公众号主体级
   白名单**撤回**——官方机构微信公众号不再做主体级升档，`mp.weixin.qq.com` URL 机械
   层保持劣质源 0.7（收益太小，不设核验升档路径）；② 微博全站入劣质源——`weibo.com`
   机械归 0.7，不再走账号级模型判断。来源：
   `docs/SOURCE_QUALITY_SEED_LISTS_2026-08-12.md` §3（2026-08-13，用户裁决）。
2. **模式范围（2026-08-13，用户裁决，§3.7 条 12 修订）**：三层结构仅用于
   `web_search`（framework_fallback）；`local_browser` 直接分级加权（机械来源梯队 +
   模型加权标注），不使用第二层选择性原文核验（`browser_read` 读取即原文）。

### 14.8 v1.8 补写裁决索引（2026-08-13）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **工具可见性：v0.2 单一探针面（2026-08-13，用户裁决，§3.5 条 1/2 修订，取代 v1.5 语义）**：
   主 Agent 工作工具统一由每动作机械探针治理——本轮模型可见 = 机械链路完整 ∩ 会话声明集，仅工具名、
   不标注状态；链路不完整者确定不可提议。全部工作工具均按机械链路判定，不设免检面：原面 A 恒声明
   控制工具（`blackboard_read` / `todo_write` / `update_goal` / `enter_plan_mode` /
   `exit_plan_mode` / `compaction_whitelist_add` / `retrieval_disposition`）与原面 C 固定列表
   工具（`run_terminal_cmd` / `lsp` / `memory_get` / `memory_search` / `image_gen` /
   `image_edit` / `image_to_video` / `reference_to_video` / `use_tool`）各自补机械链路探针，
   面 A/C 撤销。调用时机械门禁仍为最终兜底；`tool_availability_check` 事件只在状态翻转时发出
   （run-start 首翻先于 run_started）；探针快照即用即清，最小上一轮映射仅存 `tool → 完整/不完整`。
   检索车道工具由子代理确定性留痕，不参与主探针矩阵；主车道兜底消息与探针 reason 使用中性陈述。
   来源：`docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md` §9（2026-08-13，用户裁决，
   A+C→B 定档）；实施审计：`docs/audits/GAP_TOOL_PROBE_V02_SINGLE_FACE_IMPL_AUDIT_2026-08-13.md`。

### 14.9 v1.9 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **缓存与上下文成本收敛（2026-08-14，用户裁决，§3.5 条 6/7、§3.6 修订）**：
   ① 保持 v1.8 探针可见性机制不动——探针每模型请求前重算、列表仅真实状态变化时翻转；
   工具集变化打穿 DeepSeek 前缀缓存（工具块前置渲染）属接受代价，第二轮同前缀自动恢复；
   ② 否决——per-window 探测（声明过期窗口 + `retrieval_disposition` 中途激活断裂）、
   预热轮（热身请求须等于真实前缀才命中，等于重复一次全量 miss + 额外调用）、工具层后置
   渲染（OpenAI 兼容面由服务端模板决定位置，客户端不可控）、工具层 1K 压缩（收益量级
   为每 run 数千 token，不抵工具调用准确性风险）；
   ③ 确定三项——请求 header 变化留痕（system+tools 摘要 + config + 原因，变化才记）、
   探针准确性与稳定性优先（误判最少、翻转可审计）、单轮工具结果注入预算（默认 50K、env
   可调、按轮累计、超限拒批续读）+ 提示词策略化读取。
   来源：2026-08-14 缓存与上下文成本设计评审（对照 deepseek-ai/DeepSeek-Harness 请求
   重建与 request-cache e2e 设计，仅借鉴 header 留痕与可验证性思想，不引入其技术栈）。

2. **实施闭环（2026-08-15，ORZ-CACHE-CONTEXT-COST 三项全部闭合）**：
   ① 请求 header 留痕——新增 v0.2 `request_header_change` 事件（Schema/verifier/
   fixtures/TUI 同步）：每个模型请求前计算 header 指纹（system+tools+config 三
   摘要，SHA-256；config 摘要来自 transport 的 `config_fingerprint()`，api_key
   不入摘要），同一 loop 内首请求记 `initial`、后续变化记 `change` 并携带
   `previous_header_sha256`；payload 带 `agent_role` 区分主/检索车道。
   ② 探针准确性与稳定性——验证器新增翻转↔header 交叉核对（`tool_availability_check`
   完整集翻转后、下一 `model_output` 前必须有主车道 `request_header_change
   reason=change`）；新增独立审计模块 `assurance/probe_accuracy_audit.py`
   （假完整/假不完整候选 + 翻转未留痕观察，门禁类 error code 不计误判）；
   旧 journal 兼容边界：无 header 事件的 v0.2 journal 不做该交叉核对。
   ③ 单轮工具结果注入预算——`ORZ_MAX_INJECT_TOKENS_PER_ROUND`（默认 50K，
   chars/2 估算、按模型轮累计、并行批内多结果累加）；超限后本批后续调用
   无 ToolStarted 拒绝，`tool_completed` 携带 `inject_tokens_used/budget`
   （error=`round_inject_budget_exceeded`），模型面显式提示 offset 续读 /
   grep 优先，拒绝键进连续拒绝熔断；提示词新增「读取纪律」段落（grep/结构
   优先、证据关键文件才全文、大文件 offset 分段）。
   验证：orz-loop 338 / orz-assurance 151 / orz-tui 178 / orz-bin 全部通过；
   Python runtime+assurance 1896 通过、14 skipped；仓库门禁 valid、0 错误。
   实施审计：`docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md`。
   **2026-08-15 二次全面审查修复**：① payload 增 `change_kind`
   （system/tools/config/multiple——机械「变化原因」；Schema 在 reason=change
   时必填 `change_kind` 与 `previous_header_sha256`，verifier 校验其与相邻
   事件摘要差一致）；② verifier 允许每车道多条链——每次 loop 调用首请求
   `initial`，子代理多 activation/主车道多 run 合法，`initial` 重置链起点；
   ③ 新增 `_verify_v02_inject_budget`（`round_inject_budget_exceeded` 必带
   `inject_tokens_used/budget`，且两字段仅该码可带）；④ 边界登记——压缩摘要/
   预检等 loop 外辅助模型请求不参与 header 留痕（header 恒定、与探针无
   交互）；⑤ 预算计数口径=本轮实际注入的 tool 消息（含无 ToolStarted 门禁
   拒绝合成消息；预算拒绝提示本身不计）。

### 14.10 v1.10 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **压缩机制重设计（2026-08-14，用户裁决，§3.6 修订）**：
   ① 有效窗口=384K（DeepSeek V4 论文检索质量 MRCR/MMR 塌陷最低值，不做新 live probe；
      legal max 384K 为 2026-08-07 校准注释口径）；普通触发 160K（≈42%）、兜底 200K
      （384K − 160K completion 预算 − 24K 余量），兜底无视摘要冷却；旧 250K 兜底因
      prompt+completion 窗口账不成立而废止。
   ② 工具调用记录：每模型轮完成后机械坍缩为动作台账行（工具、目标、结果指针/digest、
      非空最终回复保留），零模型调用、无冷却、不得拆工具配对；完整记录由 journal/侧车/
      P4 审计面承接，不依赖模型上下文。
   ③ 模板摘要：五段固化结构（目的/计划/变动文件路径/注意事项/后续衔接），字符上限
      3/3/5/3/3K=17K；目的/计划/路径由黑板/plan 机械填充，注意事项/后续衔接由模型生成；
      LLM 段超限拒绝重做 ≤3 次，终止态=保留机械段+扩大最近尾+`summary_incomplete` 标记；
      路径段超限走 Top-N+存档指针（索引化，不截断）；摘要标 derived_unverified + digest。
   ④ 触发与冷却：摘要冷却 ≥3 工具轮（复用 Grok min_steps_before_compact 语义）；
      缩减守卫 min_compactable=5K、缩减门 40%；摘要链只进审计存档（.gsa/compaction/），
      不继承旧语义，滚动单 marker 附回查清单。
   ⑤ 复用边界：机制骨架复用 orz-compaction crate（select.rs、缩减守卫、退化摘要拒绝、
      超时/重试、用户查询保留）；摘要模型名覆盖为 DeepSeek V4；形态=旧前缀摘要+最近尾
      保留（HistoryThenSteps），非 FullReplace。
   ⑥ 实施前置：D2-2（恢复超窗预估算截断）与 D3-1（marker/白名单恢复保留）必须先于
      压缩接线闭合；本补写推翻 2026-08-08「LLM 摘要否决」与「节奏压缩仅最终答案间隙」
      裁决；保留轮/工具配对、白名单优先级、50K 单轮注入预算与 derived_unverified 契约。
   ⑦ 实施闭环（2026-08-14，S1-S4 全部闭合）：D2-2 恢复预检截断 +
      `context_recovery_truncated` 事件、D3-1 marker/白名单恢复保留、动作台账机械坍缩
      （请求视图，侧车保留全文）、五段模板摘要（160K/200K/3 轮/5K/0.6，重做 ≤3，
      `summary_incomplete` 终止态 + fallback 机械截断）、`.gsa/compaction/` 存档 +
      7 天 retention、`context_compressed` v0.2 payload、TUI 投影；检索子代理同构触发，
      摘要调用不计工具轮/orientation 轮。实施审计：
      `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`。
   来源：2026-08-14 压缩机制设计评审（对照 Grok 0.2.111 compaction 组件与 DeepSeek V4
   检索质量数据；设计入口 `docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md`）。

### 14.11 v1.11 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **检索机械控制裁决登记（2026-08-14，用户裁决，§3.7 相关）**：
   ① 候选核验预算 `ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8，web_fetch 家族
      （framework_fallback）与 `browser_read`（local_browser 第二段读取）共享同一
      per-activation 计数域——activation 累计、精确 URL 去重、continue 重入不重置、
      activation 关闭清零（与工具轮预算 session 语义一致）；
   ② 主 Agent 不执行检索任务——主车道模型可见投影移除 `browser_read`，检索子代理
      投影从 host registry 恢复该工具（2026-08-14 用户裁决）；
   ③ 候选门禁决策先于权限/ACAF 票据门禁（候选拒绝无 ToolStarted、不签发票据），但
      计数消费在权限与票据门禁通过后、执行前提交——被权限或票据拒绝的调用不消耗
      候选预算，其拒绝事件不携带计数字段（2026-08-14 审查修复）。
   来源：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`；实施审计：
   `docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md`
   与 `docs/audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md`。

### 14.12 v1.12 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **检索机械控制提示词面收敛（2026-08-14，P0-B 步骤 6，§3.7.9 相关）**：
   ① 主 Agent 提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"（设计 §3.3）；
   ② 检索子代理提示词移除"候选 ≤5 / never the full reference list"软约束，改指
      机械预算反馈（"候选 N/M，剩余 K"）；
   ③ 来源加权与引用规则段落去冗余，prompt.rs 测试同步。
   来源：`docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md`；实施审计：
   `docs/audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md`。

### 14.13 v1.13 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写不改变本 ADR 任何既有条款的语义。

1. **中立问询升级为强制模板轮（2026-08-14，用户确认，§4.2/§4.6 相关）**：
   ① 触发点改为"下一安全动作间隙明确暂停，模型必须填写问询模板后才恢复动作"；
      暂停=独立 checkpoint 轮，不派发任何工具，模型本轮只输出模板答案；
   ② 模板字段：`task_position` / `progress_evidence` / `blockers` /
      `next_action`（continue|adjust|gather_evidence|ask_user|handoff）/
      `changed_direction`；复用 allowed/forbidden 词汇；
   ③ 机械校验：必填/枚举/长度；失败给一次错误反馈重填；仍失败→按已填部分
      机械降级 + journal 记录，不挂死；
   ④ 目的定位：拉回注意力、防跑偏与钻牛角尖；强制表达、不验证诚实；缓解必做——
      `progress_evidence` 与 journal 证据身份存在性交叉校验、`gather_evidence`
      必填缺失面；
   ⑤ 范围：主车道；Orientation 与 DC 两族共用，模板按触发类型微调；与机械助理
      执行层不冲突（暂停点在模型决策边界）。
   来源：`docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md`；实施前置：
   ADR §4.2 正文修订、事件/Schema/verifier/fixtures、测试。
   ⑥ 实施登记（2026-08-15）：ADR §4.2 正文修订（v1.16）+ 强制模板轮实现闭合
      （checkpoint 轮/机械校验/一次重填/降级兜底/证据交叉校验/契约与测试），
      见 §14.16 与 `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。

2. **会话累计上下文监测（2026-08-14，用户确认）**：
   ① 度量=当前会话累计模型可见输入 token（usage 实报优先，journal 估算兜底）；
      与压缩独立——压缩把请求窗口钉在 384K 内，本监测是会话生命周期信号；
   ② 阈值：384K 机械提醒、500K 机械总结推荐（可配）；
   ③ 最简实现：到达阈值的最后一轮模型输出末尾机械附一句提醒；headless/自动化
      仅写日志；
   ④ 500K 推荐复用压缩五段模板 + 新窗口开场提示骨架；不自动开新窗口，只给产物；
   ⑤ TUI 横幅与 journal 事件为进入用户侧 beta 前的后续可选。
   来源：`docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md`。

3. **崩溃恢复工具结果词汇（2026-08-14 记录，条件触发）**：
   恢复中断轮次时补合成 Tool 消息——`TOOL_NOT_STARTED`（工具从未开始）与
   `TOOL_OUTCOME_UNKNOWN`（已开始但结果未知），并明确告诉模型"只重试只读/幂等
   操作，验证可能副作用或询问"。出现恢复面 400 或副作用未知证据时实施；单点修复，
   不建子系统。来源：`docs/DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md`
   附录候选 1。

4. **DeepSeek Harness 借鉴复核（2026-08-14，用户裁决）**：
   orz 自身（除成熟底座外的一切）即整体化二进制薄层，底座可较简单切换；个人开发者
   无插件生态，不支付子系统化复杂度。三项机制复核结论——A（Windows ACL 沙箱）挂起
   （与 BACKLOG"ACAF Slice 3/4 暂缓"一致）；B（文件观察策略）收编为
   `workspace.search_replace` 动作契约规则（随 CLASSICAL-EXEC-ASSISTANT 小样 2
   裁决）；C（工具结果裁剪）收编为纯函数（在 COMPACTION-REDESIGN S2 或 50K 注入
   预算实施时顺带实现）。其余层已覆盖（hooks/transport 重试/token 计量/持久化/
   停滞守卫/plan/goal/skills/terminal/lsp/web/ACP/凭据/子代理角色契约）或不适配
   （jobs/schedule/workflow 与异步调度封禁裁决冲突；插件/UI 生态形态不引入）。
   来源：`docs/DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md` /
   `docs/DSH_BORROW_DESIGN_DECOMPOSITION_2026-08-14.md`。

### 14.14 v1.14 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代既往条款。

1. **P0-D 审查修复（2026-08-14，基于全面检查的六项用户裁决，§3.6 修订）**：
   ① 缩减守卫失败不再跳过或机械截断——跨触发轮重试守卫（不打断当前内容），连续
      `GUARD_RETRY_LIMIT`=3 次仍失败时强制执行一轮模板压缩，事件带
      `guard_failed=true`、marker 附"机制失败，需处理"提示；兜底 200K 的"摘要失败
      后机械截断"保留（仅限摘要 LLM 失败路径，不复用于守卫失败路径）。
   ② 恢复会话治本：每次成功 run 结束（主车道与检索子代理一致；grill 除外）在
      terminal 事件前对全量消息估算超过 `session_end_trigger_tokens`（默认 160K）
      的会话自动执行一轮压缩（`reason=session_end`、强制、不受缩减守卫约束），
      摘要 marker 随 sidecar 写回固定存档；D2-2 恢复预检保留为旧侧车兜底。
   ③ 摘要存档写失败：显式重试 ≤3 次；仍失败时事件带 `archive_write_failed=true`、
      marker 附"存档写入失败：摘要未落盘，需处理"，不再静默。
   ④ 退化守卫：废止 orz-compaction 500 字符英文向门，改为 ORZ 自定 300 等效字符
      门——CJK 表意字一字折算 2 等效字符（150 个汉字即达标）。
   ⑤ 黑板窗口滚动：压缩成功后清空 blackboard edit 窗口（用后擦净），路径槽天然为
      "本窗口增量"；路径槽按 Top-40 条 + 5K 字符双上限，溢出指针指向本次摘要存档
      （全量路径不再依赖黑板上累积查询）。——已由 §14.15 ① 废止（2026-08-14 用户
      裁决：黑板按 plan epoch 轮换，压缩不再清空/滚动黑板；实施已闭合）。
   ⑥ 摘要冷却：确认按模型轮计数，默认 3 轮降为 2 轮（避免长动作累积；160K/200K
      双阈值仍是主要安全网）；摘要调用补 120s 专用超时（`SUMMARY_CALL_TIMEOUT`）。
   ⑦ 事件契约：`context_compressed` v0.2 的 reason 增加 `session_end`，新增可选
      `guard_failed` / `archive_write_failed` 布尔（旧 v0.2 payload 无此二字段仍可
      replay）；verifier 交叉规则：guard_failed 只允许 rhythm/fallback、
      archive_write_failed 只允许完整摘要；`context-recovery-truncated` payload
      schema `$id` 版本号修正为 v0.2。
   来源：P0-D 全面检查结论与用户逐项裁决（2026-08-14）；实施审计
   `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md` §7。

2. **P0-D 二次复查文档对齐（2026-08-14，用户要求处理复查全部问题，§3.6 口径修订）**：
   ① 会话结束压缩触发阈值补写：全量消息估算 > `session_end_trigger_tokens`
      （默认 160K）才执行，正文 §3.6 与设计投影 §6 同步（实现不变，审计 §7 已登记）。
   ② 设计投影 §7 复用边界更新为语义等价内联：orz-loop 不再依赖 orz-compaction，
      退化守卫为 ORZ 自定 300 等效字符门（500 字符门表述全部移除）。
   ③ 冷却残留修正：代码注释与 schema 描述中"≥3-round cooldown"改为 2 轮（v1.14 口径）。
   ④ 终止态 marker 的 64 位 "0" 占位 digest 改为显式"（未生成）"，与事件
      `summary_digest=null` 一致。
   ⑤ fixtures 生成器回写 P0-B step 5 手工修订：`FIXTURES_README_V02`、
      `citation_validation` 负样例（移除 `message_block`）与
      `citation-validation` 信封时间戳（2026-08-14）对齐提交树，消除生成器漂移。
   ⑥ 恢复预检估算校准（chars/2 对中文可能低估）登记为 ORZ-SESSION-CONTEXT-MONITOR
      （P1 6d）实施前置校准项。
   来源：P0-D 二次全面复查结论与用户处理指示（2026-08-14）；实施审计
   `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md` §8。

### 14.15 v1.15 补写裁决索引（2026-08-14）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代既往条款。
2026-08-14 实施已闭合（见 `docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`），
ADR §3.6 正文修订随实施登记。

1. **黑板擦除机制重设计（2026-08-14，用户裁决）**：
   ① 解耦：黑板生命周期与压缩生命周期分离——压缩不再清空/滚动黑板；「压缩成功后清空
      blackboard edit 窗口（用后擦净）」机制废止（取代 §14.14 条目 1 ⑤ 的机制面）。
   ② 轮换触发：黑板按 plan epoch 轮换——plan 区为单写者复写区，每个已批准计划携带
      `plan_id`/`plan_epoch`；只有新 plan epoch 批准触发轮换；同 epoch 修订不清黑板；
      不按 plan 文本变化、不按当前任务完成判定。
   ③ 轮换范围：清 edits/tool_actions/exec 工作记录并复写 plan（含 analysis/decisions/
      auth_grants）；保留 gate_log、白名单与 internal/external retrieval 分区（检索
      分区继续按 activation close/continue 生命周期管理）。
   ④ 原子性与归档：轮换=归档旧 epoch 快照 → 清工作区 → 写新 plan 的单次提交；epoch
      快照（如 `.gsa/blackboard/epoch-<id>.json`）承载全量路径/动作记录；`blackboard_read`
      跨 epoch 回查走归档或 epoch 参数。
   ⑤ 压缩衔接：路径槽语义改为「本 plan epoch 增量」（Top-40 + 5K 双上限，溢出指针指向
      epoch 快照/摘要存档）；marker 携带 `plan_epoch`；恢复随 sidecar 恢复 epoch 快照。
   ⑥ 中立问询/DC：同 epoch 压缩不动黑板锚点；轮换在新 epoch 首动作前完成，问询不读
      半空黑板。
   ⑦ 实施（2026-08-14 闭合）：plan 批准事件携带 `plan_epoch`（Schema/fixtures 同步）；
      `with_plan` 改为带 `plan_id`+`plan_epoch` 身份并原子轮换（同 plan_id 修订不清板）；
      `.gsa/blackboard/epoch-<n>.json` 快照（批准/修订时持久化当前 epoch，轮换时归档旧
      epoch）；`blackboard_read` 增 `epoch` 参数跨 epoch 回查；压缩不再清黑板、marker 携带
      plan_epoch、路径槽溢出指针指向 epoch 快照；恢复经 archive dir 装载最新 epoch 快照；
      retention 纳入 7 天清扫。
   ⑧ 编号与身份不变式补强（2026-08-15，全面复查 F1/F3，用户裁决）：`plan_epoch` 改为
      时间戳单调编号——unix 毫秒为基底，`next = max(now_ms, 磁盘现存 max + 1)`；时间戳进入
      编号本身（非仅文件名），7 天 retention 清扫后编号不复用，marker/`blackboard_read`
      epoch 参数跨窗口仍唯一。身份不变式强制（一一对应、无误用可能）：同 `plan_id` 修订
      必须沿用同 `plan_epoch`；新 `plan_id` 必须使用严格更大的 `plan_epoch`；违反在
      `rotate_to_plan`/`try_with_plan` 返回错误（`with_plan` fail-fast），拒绝先于任何
      黑板变更。retention 对 `.gsa/blackboard` 清扫时始终保留最高编号快照（恢复入口），
      长生命周期黑板不因当前 epoch 文件超龄而失去恢复。
   来源：2026-08-14 用户裁决（黑板擦除机制重设计讨论）；设计文档
   `docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`；实施审计
   `docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`。
   ⑨ 复查遗留闭合（2026-08-15，F2/F4-F7/F9/F10，用户处理指示）：
      ① F2——epoch 快照写盘原子化：先写 `<name>.json.tmp` 并自检解析，再 rename 到最终
         文件；崩溃只可能留下 `.tmp`，不会产生「半截文件成为最高编号恢复入口」；
         `latest_epoch_snapshot` 从高到低回退到第一个可解析快照。
      ② F4——跨进程分配：CLI 启动以 `.claim-<n>` 原子 claim（`create_new`）占号，同毫秒
         并发进程只有一个能赢得编号，碰撞方递增重试（有界）；claim 计入下次分配扫描，
         崩溃不导致复用；retention 按年龄清扫过期 claim。
      ③ F5——归档目录单一来源：路径槽溢出指针取 controller 配置的
         `blackboard_archive_dir`，不再由 `session_cwd/.gsa/blackboard` 重算。
      ④ F6——`blackboard_read` 的 `epoch` 参数区分「缺失」（live 视图）与「非法」
         （0/负数/浮点等显式报错，`exit_code=1`），不静默回退。
      ⑤ F7——epoch 归档写失败并入事件面：新增 v0.2 `epoch_archive_write_failed`
         （archive_dir/plan_epoch/kind=rotated|current/attempts）；builder 阶段无
         journal writer，失败先排队，run 启动时随事件写入。
      ⑥ F9——`EpochSnapshot.rotated_at` 更名 `persisted_at`，旧归档键经 serde alias
         兼容读取。
      ⑦ F10——设计 §5「随 sidecar 恢复」措辞改为「archive dir 装载最新 epoch 快照，
         与 marker 一起构成恢复上下文」，与本节 ⑦/⑧ 一致。

### 14.16 v1.16 补写裁决索引（2026-08-15）

本节记录冻结后的显式补写；规范正文以所指章节为准（§4.2 正文已随本节修订）。

1. **中立问询强制模板轮实施登记（2026-08-15，用户指示实施）**：
   ① ADR §4.2 正文修订：触发点「注入文本、循环继续」→「下一安全动作间隙暂停并进入
      强制模板轮（不派发任何工具，模型只输出 JSON 问询模板答案）」；模板字段与校验、
      计数语义、缓解必做见 §4.2 条 10-12。
   ② 事件面：新增 v0.2 `checkpoint_response`（checkpoint_id/inquiry_kind/agent_role/
      attempt/outcome/response/validation/cross_check/degrade_reason）——fire 事件
      仍在注入间隙写入，响应事件在模板轮完成后写入；Orientation 与 DC 共用同一事件
      类型，`inquiry_kind` 标明所答 fire 族。`attempt` 1-2；`outcome`=
      accepted|refill_requested|degraded。2026-08-15 复核：DC fire 事件可选携带
      `agent_role=main`（主车道恒 main），验证器对未携带该字段的历史 fire 兼容；
      `checkpoint_response.agent_role` 收紧为主车道 `main`（检索车道不产生响应事件）。
   ③ 运行时：pending checkpoint 单槽（一次一轮）；pending 期间跳过压缩与再次触发、
      工具探针与工具列表置空；响应校验失败给一次 `[CHECKPOINT_REFILL]` 错误反馈重填，
      仍失败按已填部分机械降级（`degrade_reason=validation_failed_after_refill`），
      不挂死。checkpoint 轮计入已完成逻辑模型轮；accepted/degraded 才提交
      Orientation fire / 推进 DC 阶段（降级轮按各触发族既定语义提交）。同一间隙两族
      同时到期时 Orientation 优先、DC 下一安全间隙再触发。
   ④ 缓解必做：`progress_evidence`/`missing_evidence` 与 journal 证据身份（主车道
      证据 `EvidenceRecord.identity` + 已提交检索 ledger source_id/url/title + DC
      已检视面）存在性交叉校验，found/missing 随响应事件记录（非阻断）；
      `next_action=gather_evidence` 必须给出 `missing_evidence`（校验错误）；
      验证器同时复核 outcome↔validation 一致性与 gather_evidence 条件面
      （2026-08-15 复核）。
   ⑤ 范围：主车道（Orientation + DC）；检索车道保持注入后继续的旧行为（§14.16
      不改变 §4.2 三个 Agent 各自独立计数的既有语义）。v0.3 模板块取代 v0.2 三问块
      （`[ORIENTATION v0.3]` / `[DIAGNOSTIC_COVERAGE v0.3]`，前缀注册不变）。
   来源：`docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md`；实施审计
   `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。

### 14.17 v1.17 补写登记（2026-08-15）

本节为**正式补写登记**：设计内容已由用户定案（2026-08-15），进入实施路由
（BACKLOG/TODO P0-C）。规范正文修订（§3.6/§4.2 等）随实施登记。

1. **模型面重构设计确认（计划-执行分离与黑板化指挥，用户裁决）**：
   ① 模板去人格——除机械契约外，人格化内容从主/子代理模板与 Orchestrator prompt body
      全删；persona 配置与子代理 `<persona>` 段退役（roles 保留）。
   ② AGENTS.md 计划型机械包裹——注入固定前缀，唯一机制；不做规范模板、不做 schema 校验。
   ③ 首轮计划轮硬门——首轮模型面=黑板只读（workboard/plan + actions 注册板块）+
      `plan_write`；不派发执行工具；注册板块=探针投影唯一事实源（S2 静态基础集仅为
      S3 接线前中间态）。
   ④ 助理层全承接（双模式定案，2026-08-15 用户裁决）——主模型写面=`plan_write`/
      `action_write`，读面=`blackboard_read`/`assistant.trace`/工作区只读；执行、变更、
      shell、子代理 spawn、检索全部经助理层订单下发；核查面必须覆盖全部副作用出口；
      放弃「直接执行面永久移除」——console 为默认，direct 为受控降级。
   ⑤ 分步计划硬契约——计划=有序步骤数组；步骤状态机
      `pending → in_progress → done(receipt_id) | failed(receipt_id)`；下一步订单需
      上一步 receipt 机械放行，防惯性幻觉。
   ⑥ 阶段——A 当前（去人格 + AGENTS.md 包裹 + 首轮计划轮）；B（注册板块=探针投影、
      工具栏刷新绑定黑板模型栏）；C（助理层实战验证后模型面收敛为黑板读写 + 只读核查；
      direct 受控降级保留，不永久移除）。
   ⑦ 双模式机制——console（默认，§4 面）↔ direct（受控降级）：连续 3 次助理层故障面
      失败（verify；execute 且无 exit code/host 错误；业务非零退出与
      policy/protocol/registry/contract/order_stale/step_not_done 不计）→ 机械询问
      （无工具轮、一次重填、降级 stay、每 run 至多一次）→ 模型选择 switch 后写
      `console_mode_transition` + gate_log；direct 动作带 transition_id；权限/ACAF/模式门
      不变；计划门约束 console 订单，direct 为有记录的例外（`console_step_done` 需
      transition_id + trace_id 证据置 done）；`console_return_to_console` 单向返回或 run
      结束复位；模式为 run 级状态，plan epoch 轮换不清；ActionOrder 增 `step_id`
      （Schema 先行）。
   ⑧ 结构化策略拒绝（P1-2 定案，2026-08-15）——拒绝路径（权限/ACAF/模式门；
      taint 预留）在 `run_host_tool` 边界返回 `ToolResult.policy_denial` 结构化
      信号，console 适配层删除字符串前缀判定；ToolCompleted 增可选
      `policy_denial`（Schema/verifier/fixtures 先行）；实施随 S3 前置。
   ⑨ P0-C S3 实施闭合（2026-08-15）——`assistant.trace` 只读服务接线
      （`ActionKind::TraceRead`：按 trace_id 有界取回、读操作入 trace 与
      ToolStarted/ToolCompleted 事件面）；`workspace.run_script` PTC 线性脚本
      生产化（`ActionKind::RunScript`：`$ref` 静态/运行时校验、逐行契约校验 +
      trace、8 步/30s/4MiB 上限、禁嵌套、fail-closed 保留内层 step/code +
      `script_step`；内层步骤仍逐行经既有权限/ACAF/模式门）；注册板块 =
      Profile/Bundle ∩ 探针完整集（`ActionBundle` standard/read_only/benchmark，
      同轮探针快照同时驱动工具投影与注册板块）。实施审计：
      `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`。
   ⑩ P0-C S3 审查收口（2026-08-16，用户逐项裁决；实施随 S3 收口、S4 登记）——
      a) 单订单步数上限 20→8（`MAX_SCRIPT_STEPS_PER_ORDER`，schema `maxItems`
      同步）；b) `assistant.trace` 查无 trace_id 定案 `step=execute` +
      `code=not_found`（服务已解析、存储查询失败属执行阶段，失败信封附有界
      trace 尾部）；c) checkpoint 轮（无探针间隙）跳过注册板块刷新，保留上一轮
      探针过滤后的内容（`pending_checkpoint` 守卫）；d) 注册不变式补齐——内部
      动作携带 host 目标注册即拒绝、内部动作类（TraceRead/RunScript）全局唯一、
      bundle 至少启用一个场景、嵌套脚本按 `ActionKind::RunScript` 拒绝；e) 脚本
      内允许调用只读内部动作（含 `assistant.trace`，非嵌套、逐行过既有门）；
      f) trace 生命周期=会话级（会话开始时空、结束即弃、侧车恢复不携带，50 条
      为会话内环形上限，跨会话 trace_id 一律 not_found）。
   ⑪ P0-C S4 实施登记（2026-08-16；用户裁决项在 ⑩ 与 S3 审计 §6 已定案）——
      a) 单步超时：30s 墙钟=总墙钟+单步受控；`LoopHost::call_tool_with_timeout`
      为每调用携带显式覆盖，host 按 `min(覆盖, 配置预算)` 截止并进程树收口
      （用户裁决：不得用脚本层 timeout 替代进程收口）；脚本每步传剩余截止，
      host 截止以结构化 `ToolResult.timed_out` 上浮——直接订单
      `step=execute`+`code=tool_timeout`，脚本归一化 `script_timeout`+
      `script_step`（不解析文案前缀）。
      b) 脚本消耗 tool-round 预算：每个实际执行动作计 1 单位（直接订单 1、
      脚本每步 1；执行前被拒不计数）；发放前预检=当前模型轮 1 单位 + 脚本
      长度 ≤ 剩余，不足零执行拒绝（`step=protocol`+`code=budget_insufficient`）；
      按实际执行步数减计并计入 `tool_rounds`，下一轮预算块机械反映，耗尽同走
      最后无工具轮。
      c) 端到端测试：FakeProvider 完整任务会话、checkpoint 轮板块保留断言、
      超时/预算边界；orz-loop 392 通过 / 0 失败。实施审计：
      `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`。
   ⑫ P0-C S4 审查收口（2026-08-16 二次；S4 全面审查发现处理）——
      a) host-owned 同步工具（`project_doc_index`/`browser_read`/`pdf_read`
      /PDF 路由 `web_fetch`）不经 host timeout 包装为既有边界（配置预算为
      「经注册表执行的调用」的硬上限）；脚本层每步完成后核对 30s 总截止，
      超时按 `script_timeout`+`script_step` 事后 fail-closed（不中断在途
      同步工作），总墙钟对全部步骤生效。
      b) 预算预检先对脚本做静态校验（无执行）——失败或超上限（>8 步）不
      预检，交注册表/契约校验产生真实错误码（不掩盖 `unknown_service`/
      契约错误）；max=1 时直接订单 `remaining=0` 零执行拒绝语义显式测试
      锁定。
      c) 决策门材料补齐小样 1 结果工件（`sample1_result.json`，
      `smoke_test.py` 90/90 复跑）；orz-loop 396 通过 / 0 失败。实施审计：
      `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md` §8。
   ⑬ P0-C S4 超时语义复核登记（2026-08-16；用户复核裁决，对照 Codex
      `command/exec timeoutMs` 与 Grok Build `toolset.*.timeout_secs`/
      `ProcessScope` 成熟设计）——撤销 ⑫ a) 的「30s 总墙钟含进程时间」
      语义：脚本每步不传收缩剩余（`None`），由 host 每调用超时独立约束
      （配置预算，默认 5 分钟，进程树收口不变）；host-owned 同步工具不经
      timeout 包装为既有边界；脚本层保留 8 步 / 4MiB / tool-round 预算上限；
      orz-loop 395 通过 / 0 失败。实施审计：
      `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md` §8。
   ⑭ P0-C 正式组件决策门（2026-08-16 用户裁决）——小样 1/2/3 与 S1-S4
      实施审计材料齐备，用户裁决「P0-C 可转正式组件」（不达标即撤条款未
      触发）；CLASSICAL-EXEC-ASSISTANT 进入生产组件基线，后续模型面重构
      按 PLAN-FIRST 阶段 B/C 推进。
   ⑮ PLAN-FIRST 阶段 A 实施登记（2026-08-16）——
      a) 模板去人格：主/子代理/apply-patch 模板与 `ORCHESTRATOR_PROMPT_BODY`
         删除身份宣告与语气/人格内容，保留机械契约（action_safety、
         tool_calling、格式化、项目指令/用户信息、编排职责分工）；XOR 加密
         模板重生成；渲染测试锁定无人格关键词（`released by xAI`/`friendly`/
         `curious`/`expert peers`/`aggressively` 等）；子代理 `<persona>`
         模板段退役，persona 指令不再进入系统提示词（配置解析类型保留为
         外部 shell 兼容层，审计边界登记）。
      b) AGENTS.md 计划型机械包裹：`render_agents_md` 固定前缀注入
         `<plan_first_framework>`（用户内容之前；主/子代理同一入口；唯一
         机制，不做规范模板/schema 校验，§10 非目标不变）。
      c) 首轮计划轮硬门：新增 `plan_write` 工具（结构化分步计划，§5 机械
         校验；一次错误反馈重填，仍失败机械降级并留痕——`plan_write` v0.2
         事件，Schema/verifier/fixtures/TUI 先行）；首轮模型工具面=
         `blackboard_read`+`plan_write`，其余工具派发前机械拒绝
         （`plan_round_tool_denied`）；校验通过后结构化计划落黑板 plan
         epoch（`rotate_to_structured_plan`，同 plan_id 修订复用 epoch、
         新 plan_id 单调递增）；连续 3 轮未提交 → `plan_not_submitted`
         降级放行（不挂死）；计划门为会话级（每会话一次；恢复会话/后续
         run 不重复触发），生产接线=ACP server + CLI run（控制器开关
         `with_plan_first_enabled`/`with_plan_first_session_done`）。
         orz-loop 407 / orz-tui 178 / orz-assurance 151 通过，Python
         run-event 一致性 14 通过。实施审计：
         `docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`。
   ⑯ PLAN-FIRST 阶段 A 审查收口登记（2026-08-16；全面审查 + 用户逐项
      裁决）——
      a) 计划轮不消耗 tool-round 预算（用户裁决）：批处理前固定「计划轮
         身份」，计划轮（blackboard_read/plan_write/拒绝轮）不执行
         `tool_rounds += 1`；compaction whitelist 的 `tool_rounds==0`
         窗口随之顺延到计划落板后的首个执行轮（P2-2 顺延裁决，测试锁定）。
      b) 结构上限定稿（成熟参照：AutoGPT ≤5 子目标、oh-my-loop <10 子任务
         且最多 2 次重规划、joyagent 上限 40 步、编排计划工具 2-5 里程碑、
         LangChain「一步≈一次工具调用 + 有界重规划」）——步骤 ≤32、每步
         动作 16→8（对齐 `MAX_SCRIPT_STEPS_PER_ORDER=8`）、每步证据 ≤16
         条且单条 ≤500、计划整体序列化 ≤32K 字符；上限为机械兜底而非质量
         目标；未提交计划轮上限 3 保持。
      c) P1 修复（事件契约）：plan_write ToolStarted/ToolCompleted 收敛到
         通用契约形状——成功只带 `exit_code`，失败 `status=error`+`error`，
         计划身份/outcome/epoch 由 PlanWrite 事件承载；主车道拒绝事件不再
         携带 `target`（通用 schema target 枚举仅检索车道）；Python 校验
         测试锁定新形状。
      d) P2-1/P3-4 修复：`plan_write` 加入 subagent 投影剔除与
         `ToolFilter::Retrieval.write_gate`；声明与调用均随
         `plan_first_enabled` 收敛（关闭态/grill 拒绝
         `plan_write_disabled`，不旋转黑板）。
      e) D2 全覆盖（用户指出 AGENTS.md 为 Codex/Grok 生态契约、orz 生产
         路径未注入 AGENTS.md 段）：框架块提升为 orz-assurance canonical
         常量（`plan::framework`），plan_first 会话在系统提示词层无条件
         注入（主/检索子代理同一入口，不依赖 AGENTS.md 存在）；
         `render_agents_md` 固定前缀保留给外部 AGENTS.md 消费者。
      f) P3 修复：TUI 按 outcome+degrade_reason 投影（plan_not_submitted/
         plan_rotate_failed 不再误显「校验通过」）；同一计划轮最多一次
         plan_write（`plan_write_already_submitted`）；plan_write ToolDef
         参数嵌套 schema 与校验常量对齐（8/16/32/32K）；证据条数与计划
         总量上限；旧 epoch 归档兼容（`PlanStep` serde
         `alias="description"` + 默认值，升级前快照可读）；Python verifier
         新增 `_verify_v02_plan_write` 序列规则；orz-agent persona 旧测试
         改为断言 persona 不注入。
      g) 测试与证据：orz-loop 416 / orz-tui 178 / orz-assurance 152 通过，
         Python run-event 一致性 15 通过、runtime 全量 299 通过（P1 形状
         测试 + PlanWrite 序列规则测试）；orz-agent 除 35 项沙箱 git2
         环境失败外通过（persona 测试已更新）。实施审计：
         `docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`
         §7。
   ⑰ PLAN-FIRST 阶段 B 实施登记（2026-08-16）——
      a) 注册板块=探针投影收口：controller 新增单一探针源
         （`console_probe_source`，仅内存/随轮覆盖、run 起始复位、不持久化），
         `sync_console_registrations` 为派生唯一路径（Profile/Bundle ∩
         探针完整集并持久化回黑板 actions 板块）；无探针轮次不再
         bundle-only 刷新、沿用上一轮内容（移除静态基础集中间态）。
      b) 工具栏刷新绑定黑板模型栏：`blackboard_read section=actions`
         （live 视图）读取时由最近探针源派生再渲染；归档 epoch 读保持
         快照不派生；工具投影与注册板块共用同一探针源，同源一致性测试
         锁定（注册板块中工作工具目标 ⊆ 工具栏投影；探针移除的工作工具
         不入注册板块；绑定=同源一致性，非工具栏反向读板块）。
      c) 验证：orz-loop 421 通过 / 0 失败 / 3 ignored（新增 3 项阶段 B
         单测 + 2 项审查收口单测：归档读不派生、探针源 run 起始复位）；
         clippy 无新增告警；仓库门禁仅剩「orz submodule working tree is
         dirty」（本阶段代码未提交）。实施审计：
         `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。
   ⑱ PLAN-FIRST 阶段 C 实施登记（2026-08-16；console 默认 + direct 受控
      降级双模式）——
      a) 契约层先行：v0.2 事件 +2——`console_mode_transition`（transition_id/
         from/to/trigger/streak/order_ids/model_decision/model_reason/run_id/
         round/plan_epoch/related_transition_id；覆盖 switch/stay/return）与
         `console_order_written`（order_id/write_call_id/action/step_id/
         round/plan_epoch/run_id；`blackboard_action_write` ToolCompleted
         收敛到通用形状，阶段 A 审计 §7.4 的 S2 payload-shape 债务随本次
         收口）；run-event v0.2 枚举 48→50；tool-started/tool-completed 增
         可选 console_mode/transition_id/trace_id（direct 事件链盖章）；
         Python verifier + 方向↔trigger↔decision 交叉、每 run 至多一次
         询问、direct 事件必须匹配当前 direct transition、return 引用本
         run 早先切换、order 记录须有先行的 action_write 成功（write_call_id
         对拍）；fixtures 与 conformance 计数同步。
      b) 双模式状态机（§7）：run 级 `ConsoleModeState`（模式/连败/询问标记/
         transition_id/direct trace 证据面；run 起始复位、plan epoch 轮换
         不清除）；故障面机械分类（verify 与 execute 无业务 exit_code 递增；
         业务非零退出/policy/protocol/registry/contract/order_stale/
         step_not_done 不计；`ORZ_CONSOLE_DIRECT_FALLBACK_THRESHOLD` 默认 3）；
         无工具询问轮（复用强制模板轮语义，一次重填、降级默认 stay、
         每 run 至多一次）；switch 写 `console_mode_transition` + gate_log、
         当前步骤置 in_progress；stay/`console_return_to_console` 复位留痕。
      c) 步骤状态机（§5/§6）：`StepStatus` 状态机化
         `pending → in_progress → done(receipt_id[, direct 证据]) |
         failed(receipt_id)`（自定义 serde 兼容旧归档单位变体与
         description-only 快照）；`ActionOrder.step_id`（Schema 先行）；
         步骤门=console 订单必须绑定当前可执行步骤，否则 `step_not_done`
         显式拒绝；发放时 in_progress、receipt 置 done/failed；direct 为
         有记录例外——`console_step_done` 需 transition_id（本 run direct
         切换）+ trace_id（对应已发生 ToolCompleted）双重证据，不匹配拒绝。
      d) 模型面收敛（§4/§9）：console 默认面=黑板读写（plan_write +
         action_write）+ 只读核查（read/list/grep 类），执行/变更/shell/
         子代理/检索隐藏（投影 + `console_mode_tool_denied` 调用面门禁）；
         direct 恢复探针过滤后的工作工具投影；direct 直接动作 ToolStarted/
         ToolCompleted 携带 console_mode:"direct" + transition_id +
         trace_id（事件链关联），权限/ACAF/模式门/IPG/预算/探针全部照旧；
         生产接线=CLI run + ACP server 随 plan_first 一并开启。
      e) 验证：orz-loop 433 / orz-tui 178 / orz-assurance / orz-bin
         stdio_e2e 通过；orz-host acp_server 36 通过（12 项既有 ACP 测试
         按 console 默认面收敛为计划→订单形态）；Python runtime 305 /
         assurance 1614+14 skipped 通过；仓库门禁仅剩
         「orz submodule working tree is dirty」（未提交）。实施审计：
         `docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md`。
   来源：`docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md`；关联：
   CLASSICAL-EXEC-ASSISTANT v0.6。

### 14.18 v1.18 补写裁决索引（2026-08-16）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **会话累计上下文监测度量重定（2026-08-16，用户裁决，§14.13 条目 2 修订）**：
   ① 度量由「会话累计模型可见输入 token（usage 实报优先、journal 估算兜底）」改为
      「会话内压缩次数」——以 `context_compressed` 事件为准，仅计 reason∈{rhythm,fallback}
      的会话内压缩；`session_end` 不计（run 结束存档动作，计入会稀释会话寿命信号）；
      一次压缩只计一次（重试/守卫失败/存档失败不重复计数）。
   ② 阈值由 token 制改为次数制——≥2 次机械提醒、≥3 次机械总结推荐（可配，默认待校准；
      语义对应旧 384K/500K：每次压缩约代表一个窗口 160K–200K 累积）。
   ③ 理由：压缩机制已把请求窗口钉在 384K 内，累计 token 对模型不可见、阈值无对应质量
      边界；压缩次数是会话寿命与摘要链损耗的更直接代理；原 chars/2 中文估算校准项
      随 token 度量废止。
   ④ 其余不变：与压缩独立；非目标（不做窗口预测、不自动调整压缩触发、不自动开新窗口、
      不打断 headless/自动化）；最简实现=阈值到达的最后一轮模型输出末尾机械附言
      （附当前压缩次数）、headless 仅日志；TUI 横幅/面板（含压缩次数显示）与 journal
      事件为进入用户侧 beta 前的后续可选。
   来源：2026-08-16 用户裁决；设计入口
   `docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md`（2026-08-16 度量重定版）。

### 14.19 v1.19 补写裁决索引（2026-08-17）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **console 面工具栏缓存注记（2026-08-17，用户指示补记，§3.5 条 6 注记）**：
   ① 界定：console 默认面（§14.17⑱）下，注册板块/工具栏为黑板数据、经
      `blackboard_read section=actions`（live 读取时由最近探针源派生）取回，不参与
      请求 header 指纹（tools 摘要 = 模型可见工具列表 schema）；注册板块内容变化
      不改变 console 面工具列表，不再计入 v1.9「工具集变化即前缀缓存全量 miss」
      的接受代价。
   ② 保留边界：header 仍随真实状态变化而变——console↔direct 模式切换（整块工具
      面替换）与 console 面只读工具（read/list/grep 类）探针翻转各产生一次全量
      miss，第二轮同前缀恢复命中，v1.9 语义不变；direct 模式工作工具探针翻转照旧。
   ③ 性质：纯澄清注记，无行为变更、无代码调整；实施事实依据=阶段 B
      （`sync_console_registrations` / `render_blackboard_section("actions")` live
      派生）与阶段 C（`project_console_default_tool_defs` console 固定小工具面）。
   来源：2026-08-17 用户指示补记（承接 2026-08-16 审查结论）。

### 14.20 v1.20 补写裁决索引（2026-08-17）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **console 面工具名去点号（2026-08-17，GAP-CONSOLE-TOOLNAME-PATTERN 闭合，§14.17
   工具面定名修正）**：console 默认面三个模型工具名由点号改下划线——
   `blackboard.action_write`→`blackboard_action_write`、
   `console.step_done`→`console_step_done`、
   `console.return_to_console`→`console_return_to_console`。
   ① 原因：OpenAI 兼容工具名模式 `^[a-zA-Z0-9_-]+$`；真实 DeepSeek API 在计划落板
      后下一轮请求 400（`invalid_request_error`，2026-08-17 评测冒烟
      `D:\tb-eval\jobs\2026-08-17__01-09-43`），FakeProvider 不校验故单测未暴露。
   ② 范围：Rust 生产/测试 11 文件 91 处 + Python verifier 交叉校验
      （`run_event_journal_validation.py` 4 处）+ 4 个 runtime schema 描述 +
      verifier 测试 + 本 ADR 与两份设计文档；注册板块/console 注册表动作名
      （`workspace.read_file`/`workspace.run_script`/`assistant.trace` 等）为
      订单数据而非 API 工具名，不在本改名范围。
   ③ 验证：orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 +
      real_flag 2 通过（orz-host 5 项 codex_app 审批流失败与 1 项挂起为既有
      环境问题，HEAD 基线复现）；clippy 无新增告警、cargo fmt 收口；
      仓库门禁 valid（`orz_source_manifest.sha256` 重生成 1401 条目）。
   ④ 冒烟重跑：Linux musl 二进制重建（`rust:latest` + aliyun 镜像，39m33s）后
      make-doom-for-mips 试跑 30m21s 跑满 1740s 预算，`run_invalidated{wallclock}`
      正常收尾（对比旧运行 400 即死）；console 全链路（计划落板/修订/订单发放/
      上下文压缩）无 400。
   来源：2026-08-17 评测冒烟暴露；BACKLOG 0a / TODO P0-E。

### 14.21 v1.21 补写裁决索引（2026-08-17）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **ACAF 评测跑分供应链决策（2026-08-17 用户裁决，GAP-ACAF-HARNESS-PASSTHROUGH）**：
   正式跑分时 ACAF 保持强制开启——在 TB2 任务容器内供应
   `ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE`/签发器（参考
   `scripts/orz_acaf_run.ps1` 供应链），不采用影子模式；临时解阻
   （`D:\tb-eval\.env` 的 `ORZ_ACAF_FAIL_CLOSED=0`）在实施完成后移除。
   性质：评测链路运营决策（ADR-0011 机制不变），实施登记 BACKLOG 0a / TODO P0-E。
2. **步骤门模型面缺口登记（2026-08-17 冒烟重跑定位，§14.17⑱ 补充）**：
   `blackboard_read section=plan` 渲染不包含步骤 `id`（epoch.rs 仅
   `[status] goal (actions: N; evidence: M)`），而步骤门要求 console 订单 `step_id`
   精确绑定当前可执行步骤；冒烟重跑中模型只能猜测 step_id（轨迹 13/18/23 步自述），
   导致大量读板/计划重写轮次（4 次 plan_write）并最终 wallclock 耗尽
   （reward 0.0）。修复方向=计划视图（含归档 epoch 读）补渲染 `step.id`，步骤门
   模型面闭环。另记 grep 侦查纪律项（2026-08-17 用户确认一并处理；
   **2026-08-17 复核更正归因**）：原记「模型用不存在的目标字符串 grep 全树、
   空结果被泛化为『无源码』——工具行为正确（无匹配 exit_code=1），属提示词/
   侦查策略与反馈解读范畴」；复核 journal/trajectory/orz.txt 证据后撤回——
   冒烟中两次 `doomgeneric_mips|frame\.bmp` grep 实际被 plan/console 门机械拒绝、
   未执行（journal 序列 24/38）；执行的 7 次 grep 全部无匹配（wall_ms 1–36ms），
   包括 vm.js 中实测存在的字符串（entryPoint/symbolName/sectionsToLoad/
   syscallNum/runElf/program counter），故「工具行为正确」不成立，属系统性工具层
   空结果（疑搜索范围/路径解析异常）。更正后实施方向=先容器内 grep 冒烟（对已知
   字符串断言匹配）定位根因 + grep 结果补机械范围报告（解析路径/搜索文件数/忽略
   文件数）+ exit 2 语法错误保持硬失败；模型侧侦查纪律（先 list_dir 建清单、
   pattern 用实测存在的字符串、空结果≠无文件）与注册板块 grep 参数提示保留为
   次要契约提示；回归验证仍以重跑 round 数/计划重写次数为度量。实施登记
   BACKLOG 0a / TODO P0-E。
   来源：2026-08-17 冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`。
   **2026-08-17 实施闭合（P0-E 第 1/2 项）**：①plan_write 校验消息形状明确——
   `plan` 缺失报 `missing required field: plan (expected an object with
   plan_id / goal / steps[])`；`plan` 非对象（如 got string）报
   `plan must be an object with plan_id / goal / steps[] (got string)`；
   回归测试=字符串计划→机械拒绝→错误消息含形状说明。②`steps[].actions`
   形状实证审计+探针测试锁定——14 种宽松形状（空串/裸串/空对象/null/缺 with/
   缺 do/空 step_id/step_id 不匹配/with 错类型/actions 非数组等）全部被机械
   拒绝，「空字符串仍通过校验」原观察不成立（与 grep 归因更正同类），无需收紧、
   以探针测试防回归；`with` 内容与 `do` 注册表核对仍留订单发放时契约校验
   （v0.5 操作台模型，plan 层只约束结构与长度，不判断语义——§10 不变量）。
   orz 子模块 11540fa；测试 planning 13 / plan_first 9 通过、clippy 无新增
   告警。P0-E 全部闭合（0 项）、未闭合 27 项。
3. **订单发放前拒绝事件面（2026-08-17 用户指示处理，§14.17⑱ 补充；已实施闭合）**：
   冒烟重跑中 ORD-000011（workspace.run_tests，`arguments:{}`）写入后发放前被拒，
   失败只进结果栏 receipt + TraceStore（`consume_console_order` 不写 journal 事件），
   runtime journal 无结构化拒绝记录，事后核对看不到拒绝码。定案=新增 v0.2
   `console_order_rejected`（order_id / step / phase / code / reason / round /
   plan_epoch / run_id），发放前拒绝统一入事件面：phase=pre_issue（order_stale /
   step_not_done / budget_insufficient，step=protocol）+ phase=issue（registry /
   contract / target / ACAF / policy / mode 门，ACAF/模式/权限归一化
   step=policy / code=policy_denied）；execute/verify 不入本事件（已执行订单经
   tool_started/tool_completed 留痕）。Schema/verifier/fixtures 先行；verifier
   交叉核对——拒绝必须先有同 run 同 order_id 的 `console_order_written`、机械盖章
   （round/plan_epoch/run_id）一致、每订单至多一次拒绝、phase/step/code 一致性；
   结果栏 receipt 保留为人类可读视图。实施：orz 子模块 c67a452（事件变体 +
   三处 pre_issue 路径 + 发放期 Err 分支 issue 路径发事件 + TUI 投影 +
   测试断言）；orz-loop 436 / orz-tui 178 / orz-assurance 152 / orz-bin 全量通过、
   clippy 与基线一致（lib 21 / lib test 26）、manifest 重生成 1401 条目、
   仓库门禁 valid。实施登记 BACKLOG 0a / TODO P0-E。
4. **Linux 文件型安装密钥库后端（2026-08-17 用户裁决，GAP-ACAF-HARNESS-PASSTHROUGH
   前置）**：签发器与 provision 在非 Windows 下原 fail-closed（DPAPI 不可用，
   Slice1 审计 §6 登记为设计边界——评测容器内签发器不可用）。用户裁决放行新增
   `file-0600-installation` 文件型安装密钥库：`<root>/installation-key.bin` =
   `MAGIC` ‖ 32B 密钥，0600 权限，元数据沿用 `installation-key-metadata-v0.1` schema
  （`storage` 枚举新增该值，`protected_blob_sha256` 为明文 blob 摘要）；`K_install`
  只被签发器进程读入并在使用后清零；信任锚=harness 供应链（容器内供应、临时容器），
  同用户暴露与 Windows DPAPI 同级（同用户进程均可 `CryptUnprotectData`）。
  `orz-acaf-provision` 非 Windows 分支创建文件型密钥库（Windows 分支行为不变）；
  `orz-signer` 按元数据 `storage` 分派后端（`windows-dpapi-current-user` /
  `file-0600-installation`，其余 fail-closed）；`AcafClient` 启动契约不变。
  性质：ACAF 密钥库边界变更（ADR-0011 §5 补充），实施登记 BACKLOG 0a / TODO P0-E。

### 14.22 v1.22 补写裁决索引（2026-08-17）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **大文件读取契约（2026-08-17 用户裁决，FUS-LARGE-FILE-READ-CONTRACT）**：
   语义适配留在模型，助理层只提供机械原语（引用 + 范围读），不做语义总结。
   读取工具契约升级为有界返回——超过粗门的文件返回读取句柄信封（path、size、
   encoding、content_sha256、可用范围、有界预览 ≤2–4KB、truncated、offset
   续读指针），不返回全文；小文件保持全文一次返回（一次往返）。粗门默认 16KB、
   可配 8–32KB（env/TOML 口子）；精门=单轮注入预算（默认 50K、
   `ORZ_MAX_INJECT_TOKENS_PER_ROUND`）为最终兜底。模型侧结构化读取沿用既有
   提示词策略化读取（grep/结构提取优先、证据关键文件才全文、大文件 offset 分段），
   并落成工具契约；文本文件与 pdf_read 的 `document_id` + `page_range` 先例对齐
   （path + offset）。性质：正文 §3.6 修订；设计登记
   `docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md` §11 /
   `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4；实施路由
   BACKLOG 6f / TODO P1。
2. **黑板只存指针、不存内容（2026-08-17 用户裁决，同一裁决）**：黑板是控制面
   状态视图，不是内容缓冲；文件全文等大内容不得写入黑板，黑板/结果栏只放指针
   （path/document_id/size/digest/offset），内容本体留在盘上或内容寻址证据区。
   维持「不新增自由随记区」约束；epoch 快照不承载大内容，避免快照膨胀、retention/
   恢复成本上升与过期副本。性质：§3.6 黑板分区边界补充（与 FUS-BLACKBOARD-PLAN-EPOCH
   一致）。
   来源：2026-08-17 用户裁决（大文件读取设计讨论）；CLI_PROJECT_INDEX 登记。
3. **大文件读取契约实施登记（2026-08-17 本窗口实施闭合；性质：§3.6 修订实施）**：
   实施落点=GrokBuild `read_file` 工具文本路径（console 默认面
   `workspace.read_file` 与 direct 面共用同一工具）：超过粗门（默认 16KB；
   `ORZ_READ_FILE_COARSE_GATE_BYTES` 可配 8–32KB，env/TOML 口子——
   `ReadFileParams.coarse_gate_bytes` 优先）的文件返回读取句柄信封（path / size /
   encoding / content_sha256 / available_range / 有界预览 ≤4KB / truncated /
   offset 续读指针），不返回全文；小文件保持全文一次返回；信封 terminal-only
   不流式。有界预览沿用 `N→` 行锚点格式；单行超长时在预算内截断并报 truncated
   （offset 指向下一行，长行尾部经 grep/execute 侧取）；尾部幻影行仅在正常窗口
   耗尽时追加（预算中断不越界）。边界：SKILL.md / `skills` 路径 Markdown 保持
   全量读取豁免（技能文档不被静默截断）；PDF/PPTX/图片路径不变；`FileTooLarge`
   （25K token 事后拒绝）在文本路径被信封取代，保留为防御兜底。测试：orz-tools
   read_file 199 / output 信封序列化 / orz-loop 440 / orz-host read_file e2e 2
   通过；clippy 无新增可归因告警；orz 子模块 172b14e；manifest 重生成 1401 条目、
   仓库门禁 valid。实施登记 BACKLOG 6f / TODO P1 / 实施审计
   `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。
4. **大文件读取契约全面检查修复登记（2026-08-17 本窗口审查修复；性质：项 3 实施
   修正）**：按设计合理性 / 实现合理性 / 设计与实现符合性三面复查后处理：
   ① 空窗口/越界 offset 语义——旧实现起始行超过 EOF 或 `limit=0` 时返回
   `truncated=true` + `offset=Some(1)`（误导续读绕回首行）；修复后 past-EOF
   窗口 `truncated=false`、`offset=None`、渲染报实际行数（与 FileContent 路径
   past-EOF 提示对齐）；范围内空窗口 `offset=Some(start_line)` 续读；行内截断
   恰为文件最后一行时 `offset=None`（长行尾部经 grep/execute 侧取）；
   `preview_range.start_line` 恒承载请求起始行（空窗口 `end_line=0`）。
   ② TOML 口子端到端接线——新增 `[toolset.read_file] coarse_gate_bytes`
   配置节：host 构建时经 `orz_config::load_effective_config_disk_only()`
   （system-managed > managed > user 分层合并、无远程）解析并钳制 8–32KB，
   注入 `GrokBuild:read_file` 工具参数（`ReadFileParams.coarse_gate_bytes`
   优先于 env）；env 口子（`ORZ_READ_FILE_COARSE_GATE_BYTES`）保持为无配置时
   的回退。AgentBuilder 侧同步提供 `with_read_file_params` 注入通路（测试
   锁定）。③ concise 变体工具描述同步信封说明；提示词读取纪律措辞精确化
   （只有证据关键的小文件才读全文，大文件一律经信封分段续读）。④ 边界登记：
   envelope 路径不追加 cursor rules（有界预览保持纯文件内容；
   `cursor_rules_on_read` 仅作用于全文路径）。测试：orz-tools read_file 201 /
   output 84 / orz-agent 工具参数通路 1 / orz-loop 440 / orz-host read_file
   e2e 5（含配置节解析/钳制单测与 8KB 门限 e2e）通过；clippy 无新增可归因
   告警；orz 子模块 7c4a99e + bd8d485；manifest 重生成 1401 条目、仓库门禁
   valid。登记见实施审计
   `docs/audits/GAP_LARGE_FILE_READ_CONTRACT_IMPL_AUDIT_2026-08-17.md`。

### 14.23 v1.23 补写裁决索引（2026-08-17）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **grep 搜索范围契约（2026-08-17 用户复核定案，FUS-TOOL-SCOPE-CONTRACT）**：
   用户复核 P0-E 第 6 项后撤回「补文本范围报告」的实施方向（§14.21 项 2 更正后
   方向）——补范围报告只增可见性、不修机制，且局限在 grep 单点。根因定位（工具
   层，非模型纪律）：`finalize_grep` 将 exit 1 + 空 stdout（或 exit 2 +
   "No files were searched"）统一转为 "No matches found"；而 ORZ 的 grep 调用
   总是显式传路径——ripgrep 仅在隐式路径时打印 "No files were searched" 警告，
   故该分支在 ORZ 下为死代码；「rg 搜索 0 文件」（ignore/隐藏/glob/二进制/超限
   过滤干净）与「搜索 N 文件、真无匹配」在工具面机械上不可区分。冒烟重跑中模型
   因此收到 7 次空结果（pattern 含 vm.js 实测存在的字符串）并错误泛化为
   「无源码」。设计定案：
   - grep 返回**结构化搜索信封**：`{resolved_root, files_searched, files_skipped,
     match_count, truncated}`，机械来源 `rg --stats`（stderr 解析）或 `--json`，
     不依赖模型自报；
   - **结局三型分型**：searched>0 且有匹配（正常命中）/ searched>0 且无匹配
     （真无匹配）/ searched=0（范围空——显式报 resolved root 下候选全部被过滤
     及过滤类别，绝不表述为 "No matches found"）；
   - **搜索范围语义显式化**：当前 read_file/list_dir 可见而 grep 全空，三工具
     可见集不一致（rg 默认尊重 ignore/隐藏文件，只读工具不）；定案=二选一，按
     容器内冒烟结果定：grep 默认与只读工具可见集对齐（关 ignore/隐藏过滤），或
     保留 rg 默认语义但显式提供 `--no-ignore`/`--hidden` 开关（与 glob 同进参数
     面），skipped 计数必须可见；
   - **契约泛化**：与 §14.22 读取信封同属「范围/截断必须机械报告」工具契约
     家族——读/搜/列三族统一（list_dir 补 ignored/truncated 计数），不是 grep
     单点补丁；
   - exit 2 语法错误（非法正则/glob/type）保持硬失败（既有行为，保留）。
   实施前置=容器内 grep 冒烟（对已知字符串断言匹配 + `rg --debug` 定位过滤
   来源）；模型侧侦查纪律（先 list_dir 建清单、pattern 用实测存在的字符串、
   空结果≠无文件）与注册板块 grep 参数提示降为次要契约提示。性质：§3.6/§14.22
   工具契约家族补充；设计登记
   `docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md` §12 /
   `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4；实施路由
   BACKLOG 0a / TODO P0-E。
   来源：2026-08-17 冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57` +
   用户复核（设计文档讨论）；CLI_PROJECT_INDEX 登记。
   **2026-08-17 冒烟实机复现结论（同一容器 `alexgshaw/make-doom-for-mips:
   20251031`）**：根因=构建侧打包 glibc 动态 rg 进 musl orz——`GROK_TOOLS_
   BUNDLE_RG_PATH=/usr/bin/rg`（rust:1.97-slim/trixie 产物，要求 GLIBC_2.39），
   任务容器为 bookworm（glibc 2.36），加载失败 `version 'GLIBC_2.39' not
   found`，退出码恰为 1、stdout 空，stderr 在 exit-1 分支被丢弃，7 次 grep 全部
   显示 "No matches found"。装正常 rg 后同命令命中（vm.js `syscallNum` 20 处，
   `--stats`: 10 files searched；doomgeneric .gitignore 只忽略构建产物）——
   默认搜索语义无问题。**范围语义定案=b（保留 rg 默认 ignore/隐藏语义）**：
   参数面新增 `--no-ignore`/`--hidden` 开关（与 glob 同进）；工具契约补两点
   机械规则——①任何非零退出且 stderr 非空先显式报错（先于空结果判断，杜绝把
   加载/运行失败吞成 "No matches found"）；②`--stats` 解析 files_searched 入
   搜索信封。构建侧修复（与实施同批）：Linux musl 构建不用 glibc 覆盖路径，
   走 build.rs 官方静态 musl ripgrep 下载（或显式静态 rg 路径）；冒烟回归=
   容器内 `rg --version` 可运行 + 已知字符串断言匹配。
   **2026-08-17 实施闭合（工具契约 + 构建侧）**：
   - 机械来源定稿=v1 用 `rg --files` 探针而非 `--stats`：rg 15 将 `--stats`
     写 stdout、旧版写 stderr，且会污染流式面与卡片一致性（跨版本行为差异）；
     `--files` 探针只跑在空结果路径（非零退出 + 双流为空），与主搜索同过滤集
     （glob/type/deny/ignore/hidden/max-filesize），计数有界（10K 截断杀子进程），
     零范围=0、真无匹配=≥1；命中路径 `files_searched` 留空；
   - `finalize_grep` 结局三型落地：非零退出且 stderr 非空→显式报错（修复把
     GLIBC 加载失败吞成 "No matches found" 的回归）；`files_searched=Some(0)`→
     "Searched 0 files … Retry with --no-ignore/--hidden"；空 stdout 且
     searched>0→"No matches found in N files"；exit 2 保持硬失败；
   - 参数面新增 `hidden`/`no_ignore` 开关（GrepSearchInput + console 注册表
     `workspace.grep` schema），映射 rg `--hidden`/`--no-ignore`（主搜索与探针
     同传）；
   - 构建侧守卫：build.rs 对非 Windows 的覆盖路径做 ELF PT_INTERP 静态链接
     校验，动态二进制直接构建失败并提示（musl 静态或省略覆盖走官方静态下载）；
     两份评测构建脚本改 `cargo install ripgrep 15.0.0 --target
     x86_64-unknown-linux-musl` 产静态 rg 供覆盖，不再指向 `/usr/bin/rg`；
   - 测试：grep 模块 42（新增 stderr 显式报错 / 零范围分型 / 带计数 no-match /
     隐藏目录探针 0 与 --hidden=1）、types 561、orz-loop console 68 全部通过；
     冒烟回归已执行（2026-08-17，Linux musl 重建后任务容器实机）——打包 rg
     静态（无 PT_INTERP）、`rg --version`=15.0.0、精确 orz 命令对 /app/vm.js
     搜 `syscallNum` 命中 20 处；端到端 `orz --real`（ACAF 签发器 + 文件
     密钥库）模型报告「命中 20 行」并 done，exit 0（对照旧二进制 7/7
     "No matches found"）。证据：
     `D:\tb-eval\jobs\2026-08-17__GREP-FIX-SMOKE\smoke-notes.md`。
   剩余开放面（随 FUS-TOOL-SCOPE-CONTRACT 后续项）**2026-08-18 两项闭合
   （orz 614bb3b）**：① list_dir 目录信封——`ListDirContent` 增
   listed/ignored/truncated 机械计数（ignored=未过滤走−可见走、同过滤语义、
   SCOPE_COUNT_CAP=200K 封顶、超限为下界、可见侧触顶报 None；truncated=可见
   总数−实际渲染）+ 卡片尾部 `(scope: ...)` 脚注；legacy/codex 面不报。
   ② grep files_searched 全结局探针——机械来源收敛为 v1 `rg --files` 探针，
   扩展为每次完成搜索都运行（含命中；摘要行内嵌 `(searched N files)`；错误
   路径 stderr 非空保持 None）；`--stats` 跨 rg 版本 stdout/stderr 位置差异
   污染流式面、`--json` 需重写输出契约，均不采用（位置收敛定案=探针）。
   验证：grep 99 / list_dir 60 / orz-tools 全量 2761 通过、clippy 无新增
   告警；FUS-TOOL-SCOPE-CONTRACT 转 `implemented`。

### 14.28 v1.28 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **折叠历史外挂文件（2026-08-18 用户裁决：先设计、不实施；同日用户指示
   优先实施——「得先处理命中率问题，不然成本太高了」；FUS-LEDGER-FOLD-STATE
   外挂文件形态，设计文档
   `docs/LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md`）**：缓存命中率
   复验（`D:\tb-eval\jobs\2026-08-18__19-40-12`，57 请求）实测 **81.9%**
   （miss 1,116K；其中 39 次折叠推进 + 1 次压缩后的相邻请求贡献 ~1,044K =
   93.5%）。根因三：A 触发线不复位（视图内台账块随推进累积，推进后视图仍
   ≥128K → 每 ~1.5 轮推一次）、B 重写成本（每次推进重写视图内台账块，
   公共前缀在旧台账末尾截断、其后整段重新计费）、C 压缩联动（288K 兜底
   整段重写 ~173K）。**定案=单纯外挂文件**：已折叠视图改为
   `[U0][固定指针消息][最近 1 轮原文]`——指针消息字节级固定（路径固定、
   文本固定、不含变化 ID/序号），折叠行机械追加到
   `{session_cwd}/.gsa/ledger/current.md`（只增不轮转、per-row 全局序号
   跨压缩连续、推进时读文件尾行取最大序号 +1、O_APPEND 单写 + flush）；
   不激活 memory_get/search（用户裁定：不需要记忆本身、接口=固定路径 +
   既有 read_file/grep）、不做模型总结负担；压缩不重置外挂文件，marker
   追加「历史摘要累积于 <abs-path>」；`messages` 全量保留 + journal/sidecar
   逐事件全文轨迹不变（审计双轨不变），外挂文件为可确定性重推的模型可读
   投影（损坏不致命）。**推进失败不阻塞会话**：外挂写入失败 → fold 状态
   回滚、下次触发重试（行不丢、视图不回退）。**参数分离**：`recent_tail_
   rounds`（压缩 drain 尾，2）与折叠视图尾轮分离——新增 `fold_tail_rounds`
   默认 1（压缩失败 widened tail 时 +1 → 2）、`ORZ_FOLD_TAIL_ROUNDS` 可配。
   预期收益：推进次数 39 → ~8-15、推进总 miss ~1,044K → ~150-300K、
   命中率 81.9% → **~91-93%**。性质：§14.26/§14.27 折叠状态化家族修订
   （`folded_ledger` 语义=固定指针消息，取代视图内台账块；§14.27 的 400
   修复不变量——retain 后移、GuardBlocked 零副作用、preamble/safe_fold_cut
   放弃折叠——全部保留）。
   **2026-08-18 实施闭合（S1/S2；orz 子模块提交见后）**：S1 代码——
   action_ledger.rs 新增 `ledger_file_path` / `build_pointer_message` /
   `external_row_line` / `append_ledger_rows`（尾行续号 + 原子追加）与
   `LEDGER_FOLD_POINTER_PREFIX`；`advance_fold` 改返回
   `Option<Vec<ActionLedgerRow>>`（仅返回本次新增折叠行，纯函数、IO 由
   调用方执行；指针消息首次推进设置后不再重写，`build_request_view` 前缀
   跨推进字节稳定）；agent_loop.rs 推进触发点写外挂文件（失败回滚 fold +
   warn 重试）并改用 `fold_tail_rounds`，压缩 marker 增路径提示行；
   controller.rs `ContextCompactConfig.fold_tail_rounds` 默认 1 +
   `ORZ_FOLD_TAIL_ROUNDS` 解析 + `with_fold_tail_rounds` 测试缝；summary.rs
   marker「历史摘要累积于」行 + 归档段改「折叠视图（冻结快照：外挂指针）」。
   S2 测试——action_ledger 18 项（新增：外挂追加续号、跨压缩续号（fold
   reset 后文件续号）、指针字节稳定、advance 仅返回新增行、触发复位前缀
   稳定）、summary 12 项、controller 折叠 e2e 2 项更新（指针前缀全请求
   字节稳定 + 外挂文件断言 + marker 路径提示 + 压缩后继续续号）；orz-loop
   全量 462 通过（-j 1）、fmt 干净、clippy 与基线一致（lib 21 / lib test
   26，无新增可归因告警）。S3（Linux musl 重建）与 S4（make-doom-for-mips
   复验断言命中率 ≥90%、无 400、journal 断言不变）待执行；验收 DoD 见设计
   文档 §7。

### 14.27 v1.27 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **折叠视图 400 修复：压缩触发未执行不得变更 messages（2026-08-18 用户
   裁决实施，FUS-LEDGER-FOLD-STATE 验证③前置；处理文档
   `docs/LEDGER_FOLD_MARKER_INDEX_FIX_HANDLING_2026-08-18.md`）**：
   make-doom-for-mips 单题复验三次 reward 0、以 400 `insufficient tool
   messages` 退出。**根因（复核修正）**：真正破坏点=压缩触发（未执行）时
   `run_template_compact` 顶部 `messages.retain()` 删除旧 marker 而折叠索引
   未失效——`fold_state.reset()` 只存在于执行路径（agent_loop.rs 657/732），
   `GuardBlocked`/`NoOp` 返回前折叠三态保持冻结；冻结 `fold_start=2` 过期后
   preamble=`[U0, A[...]]` 吞入首轮 plan_write 声明（其 tool 回复在折叠区）
   → Provider 判定「assistant tool_calls 后无 tool 回复」→ 400。折叠 cut
   本身始终在完整轮起点（`collapsed_cut` 整轮纪律 + `safe_fold_cut` 双向
   校验），`safe_fold_cut` 只防 cut 不防 fold_start（idx==0 原样返回 cut 为
   防御缺口之一）。**修复定案**：主修复=`run_template_compact` retain 后移
   （守卫判定后；GuardBlocked/NoOp 零副作用）+ 执行路径 `kept_start` 重算
   （marker 删除使索引左移：折叠态 `fold_cut - had_marker`，未折叠态重算
   `collapsed_cut`）；防御补强=`safe_fold_cut` 改 `Option<usize>`（idx==0
   且首轮不完整返回 `None`）+ `build_request_view` 新增 preamble 边界校验
   （`messages[..fold_start]` 末条为 assistant 声明即放弃折叠回原文）。
   **不变量**：冻结折叠索引仅在 messages 纯追加时有效；任何就地变更必须走
   折叠状态失效（执行压缩路径 = `fold_state.reset()`），未执行路径必须零
   副作用。性质：§14.26 折叠状态化的缺陷修正（FUS-LEDGER-FOLD-STATE 家族）。
   来源：三次复验取证（`D:\tb-eval\jobs\2026-08-18__07-43-34 / __08-13-30 /
   __08-44-56`）；取证存档 `__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`。
   **2026-08-18 实施闭合（orz 提交见后）**：S1 代码修复（agent_loop.rs
   retain 后移 + kept_start 重算；action_ledger.rs `safe_fold_cut`→`Option`
   + preamble 校验）；S2 新增 6 项单测（guard 无副作用 / 执行路径重算 /
   preamble 校验 / idx==0 None / 中轮回退适配 / 旧 bug 场景回原文）——
   orz-loop 全量 460 通过（-j 1）、fmt 干净、clippy 与基线一致无新增告警；
   S3 Linux musl 重建（build_orz_aliyun.sh，ORZ-BUILD-MOUNT-001）三件套
   输出时间戳更新。**S4 复验结果（2026-08-18 19:40-20:10，
   `D:\tb-eval\jobs\2026-08-18__19-40-12`）**：0 异常（此前 1×
   NonZeroAgentExitCodeError）、无 400、会话跑满 29 分钟墙钟（596 条事件、
   118 工具轮、39 次折叠推进、6 笔 console 订单、5 组 ACAF
   control_ticket issued/consumed、零 permission 拒绝）——**关键验证**：
   `context_compressed`（fallback 终止态、guard_failed=true）执行后会话
   继续 102 条事件零失败，即此前必现 400 的场景已闭环。reward 仍 0：
   agent 未在墙钟内产出可运行的 `doomgeneric_mips` ELF（验证器
   `node vm.js` 超时、`/tmp/frame.bmp` 缺失）——属任务完成度问题，非
   机制回归；验证③ reward 项保持开放。`ORZ_DEBUG_VIEW=1` 因验证③未闭合
   暂保留并登记为常驻诊断（通过后移除）。

### 14.26 v1.26 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **动作台账折叠状态化 + 压缩触发点调整（2026-08-18 用户裁决：先设计、不
   实施；FUS-LEDGER-FOLD-STATE `current-design`）**：命中率审计（DeepSeek
   控制台 8,845,056/4,279,939 ≈ 67.4%；TB2 复验 66.5%）定位到 ba86910 状态行
   修复之外的残余主因——`action_ledger::build_collapsed_request` 每请求无状态
   重算折叠边界（保留最近 tail=2 轮），每轮请求多一个完整轮次即滑动一次，
   请求前缀每轮被重写（复刻模拟：首次折叠重合率 1.5%，此后 50%→84% 爬升），
   与 v1.9 前缀缓存纪律（§3.5 条 6）冲突（2026-08-14 折叠定稿时未纳入缓存
   约束）。定案=折叠点状态化：controller 会话级持有 `fold_start`/`fold_cut`/
   `folded_ledger` 三态，请求视图 = preamble + 冻结台账 + `messages[fold_cut..]`
   （纯追加）；折叠推进改低频机械触发（视图估算 ≥ `ORZ_FOLD_TRIGGER_TOKENS`
   默认 **128K**，2026-08-18 定案；工具轮间隙执行），推进之间前缀字节级稳定；
   压缩执行时旧台账归档进摘要存档、fold 三态重置、摘要输入与主请求同源；
   恢复后 fold=None 重新累积（运行期状态不持久化）。**参数定案（2026-08-18
   用户裁决，统一参数、不做跑分特化）**：压缩普通触发 160K→**192K**、压缩
   兜底 200K→**256K**（动作触发线语义：上一请求实测 prompt >256K 即强制
   压缩、压缩后回落）。**口径修正**：384K 为 prompt 维度质量线（Max 档官方
   评估窗口），压缩触发/兜底同为 prompt 维度，直接比较 192K/256K < 384K
   成立；completion 预算 160K 为输出维度，不参与检索质量线、不作减法——旧
   「384K−160K=224K 理论上限」「192K+160K=352K 缓冲」推导**作废**。
   384K 有效窗口出处复核（2026-08-18 补精确）：DeepSeek V4 技术报告
   （arXiv:2606.19348）Figure 9 MRCR-8-needle / Average MMR 曲线，数据点
   来自论文 SVG 矢量源逐点读取——V4-Pro-Max：8K=0.900、16K=0.850、
   32K=0.940、64K=0.900、128K=0.920、256K=0.820、512K=0.660、1M=0.590；
   V4-Flash-Max（orz 实际模型）：8K=0.910、16K=0.840、32K=0.870、64K=0.850、
   128K=0.870、256K=0.760、512K=0.600、1M=0.490（此前「1M≈0.49」为 Flash
   读数，Pro 为 0.590）。两档一致：**128K→256K 为 128K 后下滑最快区段**
   （Flash ≈ −0.00086/K、Pro ≈ −0.00078/K），256K→512K 放缓（≈ −0.00063/K）。
   **定案参数与质量对应**：折叠推进 128K = MRCR 平台期边界（Flash 0.870，
   模型带原文工作不超过质量平台期；本仓库开发场景多文档读取——index 30.5K +
   ADR 43K + BACKLOG 27.8K 全量 ≈101K + preamble 8K，128K 允许连续读完关键
   文档集不被折叠打断）；压缩触发 192K = 陡降段内 Flash ≈ 0.81（压缩周期
   ≈71 轮，实测 1.6K/轮视图增长；比 96K 的 51 轮长 40%，减少长任务压缩打断、
   文档指针存活更久）；兜底 256K = Flash ≈ 0.76（仍明显高于 512K 的 0.60）。
   V4 输入上限实为 1M、max output 384K，Max 档官方评估窗口 384K（论文
   §5.3.1，对应 Flash 插值 MMR ≈ 0.68），orz 取 384K 为 512K 前保守质量下沿
   （非输入硬上限；「legal max」措辞与 max output 同数，已澄清）。
   命中率估算 ≈95.5%（现状 72%、控制台实测 67.4%）、成本约为现状 1/4
   （128K 阈值：$89/窗口、$1.18/轮；96K 为 94%/$59、160K 为 96.4%/$124）。
   性质：§3.5 前缀缓存纪律 / §3.6
   context 机制家族修订；设计文档
   `docs/LEDGER_FOLD_STATE_CACHE_DESIGN_2026-08-18.md`；实施路由=实施前登记
   BACKLOG / TODO（本设计轮不动未闭合计数）。
   **2026-08-18 实施闭合（用户指示优先；orz a5bea77）**：S1-S4 全部实施——
   `action_ledger.rs` 新增 `LedgerFoldState`（三态+不变量）、`build_request_view`
   （未折叠=原文；已折叠=preamble+冻结台账+`[fold_cut..]`）与 `advance_fold`
   （整轮配对纪律、轮次号延续、防空转）；`run_agent_loop` 每请求前估算折叠
   视图、≥ `fold_trigger_tokens`（默认 128K、`ORZ_FOLD_TRIGGER_TOKENS`）时
   loop-top 机械推进（checkpoint 轮优先）；主请求视图与压缩摘要输入同源；
   压缩执行后 fold 三态重置、drain 保留起点基于 `fold_cut`；折叠状态为每轮
   循环实例局部（随 `LoopOutcome` 返回）——同一 controller 被主车道与嵌套
   检索子代理共用，控制器共享字段会被子代理调度污染，实现偏离设计字面
   「controller 会话级字段」但语义等价：每次循环起始 fold=None 重新累积，
   与「恢复后重新累积」一致。参数接线：压缩普通触发 160K→192K、兜底
   200K→256K。测试：action_ledger 5 项单测 + orz-loop 循环级 2 项；orz-loop
   450 / orz-assurance / orz-tui 178 / orz-bin 全量通过、clippy 无新增告警、
   manifest 1401、仓库门禁 valid；实施审计见
   `docs/audits/GAP_LEDGER_FOLD_STATE_IMPL_AUDIT_2026-08-18.md`。
   **2026-08-18 二次全面审查收口（orz 5274b39）**：① 设计 §3.5 第 1 步
   归档缺口补实现——压缩成功分支把冻结台账块以「折叠台账（冻结快照）」段
   追加进摘要存档（drain 后存档为唯一台账快照）；② 新增 v0.2 事件
   `ledger_fold_advance`（fold_start/fold_cut/rounds_folded/
   view_estimate_tokens/agent_role；真实推进才发、防空转不发），每窗口一次
   前缀重写可在事件面归因（Schema/verifier/fixtures/TUI 全链同步，verifier
   窗口不变量：fold_start 恒定、fold_cut 严格递增、rounds_folded 不递减、
   context_compressed 重置开新窗）；③ `collapsed_cut` 完整性回退覆盖全部
   被折叠轮（中途不完整轮不再折叠成 no_result 行）；④ 删除死代码
   `collapsed_round_count`；⑤ 压缩联动测试修正（脚本 prompt_tokens 与真实
   视图量级一致，首个 rhythm 触发走成功路径——原测试实际走终止态未被断言
   暴露）；⑥ 口径/文档修正（session_end 全量估算显式区分、设计 §3.1 补
   per-loop local 注记、审计计数 450→452）；⑦ fixture 生成器回填 console
   三事件与身份覆盖（既有脱节隐患）。验证：orz-loop 452 / orz-tui 178 /
   orz-assurance 152、Python conformance 15 + journal validation 214 通过、
   clippy 无新增可归因告警、manifest 1401、仓库门禁 valid。
   来源：2026-08-18 缓存调研（dsh 对照 + orz 审计 + 复刻模拟）；DeepSeek
   TTL 经用户体感（社区 5 小时说法、长任务正常命中）排除为现实风险；
   CLI_PROJECT_INDEX 登记。

### 14.25 v1.25 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **前缀缓存纪律修正：状态行移出系统提示词（2026-08-18 用户裁决，
   FUS-REQUEST-CACHE 家族）**：TB2 复验（`D:\tb-eval\jobs\2026-08-17__23-29-44`）
   命中率 66.5%（旧批次 95–99%），80 次请求出现 18 次 `request_header_change`
   全部为 system 变化（tools/config 摘要恒定）——根因=常驻状态行
   `[任务状态 v0.1]`（`render_status_line`）嵌在系统提示词内，而 console 步骤机
   在每笔订单 receipt 成功后推进步骤（pending→in_progress→done），状态行随
   每轮变化使前缀缓存整段失效（断点后请求命中 0.2–4%、未命中 2万–13万 token）。
   定案=状态行移出系统提示词，改为**变化时追加**的尾随用户消息（与
   `[TOOL_ROUND_BUDGET] REMAINING` 同纪律）：系统提示词恢复完全静态
   （预算块 + 基础提示 + plan-first 框架），前缀持续命中，状态行小段增量仅在
   变化轮作为新尾随消息计费。模型仍每轮可见当前步（不回归 P0-E 第 4 项
   step_id 渲染）。性质：§3.5 前缀缓存纪律修正（2026-08-07「每轮变化内容放
   尾随消息」纪律的落实，状态行此前为该纪律的例外）。
2. **订单 receipt 步骤语义修正：发放期拒绝不标 failed（2026-08-18 用户裁决，
   P0-C 步骤机）**：`run_console_order` 的 Err 分支此前对所有失败统一
   `record_console_receipt(ok=false)` → 绑定步骤标 failed——发放期拒绝
   （registry/contract/target/policy 等非执行步，含 policy_denied/ACAF/模式门）
   并未执行任何动作，标 failed 是错误语义且造成状态行与门禁显示不一致。
   定案=步骤状态只随**执行 receipt**（execute/verify）迁移：执行失败标 failed
   （可重试）；发放期拒绝不改步骤状态（保持发放时置的 in_progress，订单可
   重试）；`build_status_line` 当前步取第一个非 done（与
   `planning::current_step_index` 门禁一致），failed 步骤显示为当前可重试。
   性质：P0-C 步骤机 receipt 语义修正（CLASSICAL-EXEC §6/§7.2）。
   来源：2026-08-18 用户裁决（命中问题检查 + 顺带观察处理）；CLI_PROJECT_INDEX
   登记；实施登记 `docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md` §14。
   **实施登记（2026-08-18 本窗口实施闭合）**：orz 子模块 `ba86910`；
   orz-loop 全量 443 通过 / 0 失败；clippy 与基线一致；实施审计
   `docs/audits/GAP_STATUS_LINE_CACHE_STEP_RECEIPT_IMPL_AUDIT_2026-08-18.md`。

### 14.24 v1.24 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **Benchmark 完全体执行面（2026-08-18 用户裁决，FUS-BENCHMARK-FULL-EXEC）**：
   TB2 跑分采用 orz 完全体——Benchmark 策略按任务合规放开 shell 与网络，但
   shell 不开放为模型直接工具：console 默认面仍只读+下单，执行全经助理层订单
   （与 `workspace.run_tests` 同构）。三层同时使能（缺一不可）：
   ① 权限层 `PermissionPolicy::Benchmark { allow_shell, allow_network }` 两轴
   参数化（默认 false/false，旧语义与旧测试不变；LocalMutation 非 shell 仍
   AllowOnce、MCP 恒 deny、工作区读限定不变；shell 工具与 SandboxEscape 别名在
   allow_shell 下 AllowOnce；NetworkCall 在 allow_network 下 AllowOnce）；
   ② 探针层 `ToolPolicy::BenchmarkFull`（`tool_policy()` 由
   `Benchmark{allow_shell:true,..}` 映射；`policy_allows_exec` 增加 BenchmarkFull；
   console `ActionBundle::allows` 加臂复用 benchmark 档）；
   ③ 控制台动作注册表新增 `workspace.run_terminal`（target=run_terminal_cmd，
   bundle=READ_WRITE，input 镜像 BashToolInput：command/description 必填、
   timeout/is_background 可选，不暴露 env/cwd；动作栏仍由探针收敛）。
   CLI 增 `--allow-shell`/`--allow-network`（→ ORZ_ALLOW_SHELL/ORZ_ALLOW_NETWORK，
   必须与 `--allow-write` 同用，否则 exit 2）。适配器按任务
   `network_mode == PUBLIC` 透传网络开关（89 题全 PUBLIC；`allow_internet` 已
   弃用，活字段为 network_mode）。安全面：ACAF fail-closed 票据
   （command_exec_v1/network_v1）仍为最终授权兜底，事件审计链与预算/墙钟/停滞
   守卫不变；「放开」=策略允许面，非审计面。性质：§3.5 探针投影 / §3.6 工具
   契约家族 / P0-C 操作台执行面补充；设计登记
   `docs/BENCHMARK_FULL_EXEC_DESIGN_2026-08-18.md` /
   `docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md` §13 /
   `docs/PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §4；实施路由=实施前登记
   BACKLOG / TODO（本设计轮不动未闭合计数）。
   来源：2026-08-17 TB2 冒烟 `D:\tb-eval\jobs\2026-08-17__23-29-44`（reward 0）
   + 用户裁决（orz 完全体、shell 挂助理层）；CLI_PROJECT_INDEX 登记。
   实施登记（2026-08-18，用户指示实施、暂不测试）：orz 子模块
   3f43478（见 TODO P0-F）——权限层 `Benchmark{allow_shell,allow_network}` 决策表
   （shell/SandboxEscape 在 allow_shell、NetworkCall 在 allow_network 下
   AllowOnce；MCP 恒 deny；默认 false/false 保旧语义）；探针层
   `ToolPolicy::BenchmarkFull` + `policy_allows_exec` +
   `ActionBundle::allows` 复用 benchmark 档；console 注册表
   `workspace.run_terminal`（READ_WRITE、input 镜像 BashToolInput 且不暴露
   env/cwd）；CLI `--allow-shell`/`--allow-network`（须与 `--allow-write`
   同用否则 exit 2）；适配器 `tb_agents/orz.py` allow_shell=True、
   allow_network 按有效 agent-phase `network_policy.network_mode == PUBLIC`
   透传（实施注记：取 environment.network_policy 而非 task_env_config 基线，
   严格不更宽）。验证暂缓（用户指示）：orz 测试/clippy、Linux musl 重建、
   单题 make-doom-for-mips 复验、2–3 题交叉、89 题分批；未闭合计数
   27 → 28（BACKLOG 0b / TODO P0-F）。
   审查收口处理（2026-08-18 全面审查后）：`is_shell_tool` 补 `sh` 名级兜底
   （默认轴 Deny / allow_shell 下 AllowOnce）；`workspace.run_terminal` timeout
   契约 lenient（integer 或纯数字字符串、补 default）；CLI `--allow-shell=<v>` /
   `--allow-network=<v>` 值形式由静默忽略改显式报错 exit 2；bundle 保持
   READ_WRITE 实施选择确认；`is_background` 后台任务完成提醒的 console 面
   可见性留验证④实机观察。详见设计 §12 / BACKLOG 0b / TODO P0-F。
   验证①闭合（2026-08-18 用户放行执行）：orz 各 crate 全量测试全绿
   （orz-loop 453 / orz-host 221 / orz-tui 178 / orz-assurance 152 /
   orz-bin 11+14+23+2+1 / orz-tools 2761，0 失败）；clippy 无新增可归因
   告警；manifest 1401 + 仓库门禁 valid。过程中修复 PLAN-FIRST/console
   双模式落地后的既有测试漂移（codex_app 12 + acp_server 1，orz c4772fc；
   orz-host 需 `--test-threads=1` 规避负载敏感的进程树超时竞争）。验证②
   （Linux musl 重建）进行中——Docker 引擎卡死，用户裁定重启电脑后续跑。
   验证③取证（2026-08-18）：make-doom-for-mips 三次复验均 400
   `insufficient tool messages` 退出；核心机制已验证（订单→发放→
   run_terminal_cmd exit=0 + ACAF 票据路径）；根因链闭合=折叠 cut 破坏
   plan_write 轮配对 + safe_fold_cut idx==0 兜底（取证存档
   `D:\tb-eval\jobs\2026-08-18__08-44-56\ROOTCAUSE_FORENSICS_20260818.md`；
   orz 3bd09fc/5bc3add 取证 WIP），修复待下一窗口。
