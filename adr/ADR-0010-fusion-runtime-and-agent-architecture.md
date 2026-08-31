# ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整

- 状态：**accepted / frozen**（2026-08-09；本文件是 ORZ 当前自然语言设计的唯一权威基线）
- 冻结版本：1.1（2026-08-10 追加 v1.2 补写，见 §14.2；2026-08-11 追加 v1.3 补写，见 §14.3；2026-08-11 追加 v1.4 补写，见 §14.4；2026-08-12 追加 v1.5 补写，见 §14.5；2026-08-12 追加 v1.6 补写，见 §14.6；2026-08-13 追加 v1.7 补写，见 §14.7；2026-08-13 追加 v1.8 补写，见 §14.8；2026-08-14 追加 v1.9 补写，见 §14.9；2026-08-14 追加 v1.10 补写，见 §14.10；2026-08-14 追加 v1.11 补写，见 §14.11；2026-08-14 追加 v1.12 补写，见 §14.12；2026-08-14 追加 v1.13-v1.15 补写，见 §14.13-§14.15；2026-08-15 v1.15⑧/⑨ 补强，见 §14.15 ⑧/⑨；2026-08-15 追加 v1.16 补写，见 §14.16；2026-08-15 追加 v1.17 补写，见 §14.17；2026-08-16 v1.17⑩ 审查收口登记，见 §14.17⑩；2026-08-16 v1.17⑪ S4 实施登记，见 §14.17⑪；2026-08-16 v1.17⑫ 审查收口登记，见 §14.17⑫；2026-08-16 v1.17⑬ 超时语义复核登记，见 §14.17⑬；2026-08-16 v1.17⑭/⑮ 决策门与阶段 A 登记，见 §14.17⑭/⑮；2026-08-16 v1.17⑯ 阶段 A 审查收口登记，见 §14.17⑯；2026-08-16 v1.17⑰ 阶段 B 实施登记，见 §14.17⑰；2026-08-16 v1.17⑱ 阶段 C 实施登记，见 §14.17⑱；2026-08-16 追加 v1.18 补写，见 §14.18；2026-08-17 追加 v1.19 补写，见 §14.19；2026-08-17 追加 v1.20 补写，见 §14.20；2026-08-17 追加 v1.21 补写，见 §14.21；2026-08-17 追加 v1.22 补写，见 §14.22；2026-08-17 追加 v1.23 补写，见 §14.23；2026-08-18 追加 v1.24 补写，见 §14.24；2026-08-18 追加 v1.25 补写，见 §14.25；2026-08-18 追加 v1.26 补写，见 §14.26；2026-08-18 追加 v1.27 补写，见 §14.27；2026-08-18 追加 v1.28 补写，见 §14.28；2026-08-18 追加 v1.29 补写，见 §14.29；2026-08-19 追加 v1.30-v1.33 补写，见 §14.30-§14.33；2026-08-20 追加 v1.34 补写，见 §14.34；2026-08-20 追加 v1.35 补写，见 §14.35；2026-08-21 追加 v1.36 补写，见 §14.36；2026-08-21 追加 v1.37 补写，见 §14.37；2026-08-21 追加 v1.38 补写，见 §14.38；2026-08-24 追加 v1.39 补写，见 §14.39；2026-08-25 追加 v1.40 补写，见 §14.40；2026-08-25 追加 v1.41 补写，见 §14.41；2026-08-29 追加 v1.42 补写，见 §14.42；2026-08-30 追加 v1.43-v1.46 补写，见 §14.43-§14.46；2026-08-30 追加 v1.47 补写，见 §14.47）
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
3. 检索结果先形成并验证结构化 `query_summary`、`source_ledger`、`filtering_log`
   与 `raw_source_refs`（机械四段；v1.45 修订，2026-08-30：`organized_response`
   / `[RESULT_JSON]` 组织块删除——机械 ledger 单轨，见 §14.45）；
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
| temporal | LIF/机械层 | 时间特征域记录（域标签 + 连续电位；PULL 查询面、零常驻 token、无注入；v1.47，2026-08-30，见 §14.47） |

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
6. 外部检索使用受约束的 browser/web 工具时，只控制自己创建或用户显式移交的 tab；**该条 tab
   生命周期措辞为冻结后补写（v1.46，2026-08-30）：tab target 可机械池化复用（会话级有界池，
   默认 4），一次调用独占一个 target 的控制权、归还即重置、模型永不接触 tab 句柄、内容/控制权
   不跨调用共享；「one tab lives exactly for this call」语义从 target 生灭调整为一次调用独占
   控制权（详见 §14.46）**；cookies、
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
    该条为冻结后补写（v1.9，2026-08-30）：主面在 `local_browser` 下可声明
    `web_search` 单一**派发入口**——模型调用即派发外部检索子代理，执行在
    子代理面完成；入口与执行解耦不构成模式混用。执行面仍二存一：外部 lane
    在 `local_browser` 下仅 `browser_read`（引擎 SERP）为检索通道，原生
    `web_search` 兜底是机械路径（`retrieval_mode_transition`，
    authority=mechanical_probe），不是子代理模型的自由选择；`web_fetch`
    族与 `web_search_*` 变体不进入 local_browser 主面（单入口语义，最小
    模型面变化）。来源：门禁观察四轮结构性零检索结论与用户裁决「先恢复
    外部」（2026-08-30），索引见 §14.44。
    该条为冻结后补写（v1.10，2026-08-30，GAP-RETRIEVAL-STRUCTURED-RESULT
    方向 C 用户裁决，索引见 §14.45）：③ 子代理模型加权标注**退役**——
    其唯一载体 `[RESULT_JSON].source_annotations` 依赖运行中不可见的
    后置分配 source_ids（机械 ledger 在子代理跑完后才签发），生产中从未
    生效（每次真实块 organized_response 恒空、
    structured_result_validation_failed 误触发）；`[RESULT_JSON]` 组织块
    契约整体删除，检索结果回归 `[DOC]`/`[SOURCE]` 声明行 + 机械 ledger
    单轨；`[SOURCE]` 声明行 URL 走 ACAF 网络目标规范化（SRC-002 表象
    修复）；`visibility_degraded` 重定义为「机械 ledger 无文本级证据
    （full_text_observed/partial_text_observed 均 0）」；机械来源梯队 ①
    与选择性原文核验 ② 语义不变。

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

### 14.32 v1.32 补写裁决索引（2026-08-19）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **折叠桥接截断设计定稿（2026-08-19 用户裁决：形态甲 + 桥 8K 真实 token
   + 触发维持 128K + 思维链不进桥/不进审计；FUS-LEDGER-FOLD-STATE 修订——
   接续 0c S4 复验归因）**：S4 单题复验（path-tracing，1800s 预算）provider
   口径命中率 88.84%（对照上次窗口 84.07%），仍低于 90% 目标；归因=3 次
   折叠重付 47.8K/50.4K/56.4K（合计 154.6K = 总 miss 66.5%）——折叠从
   视图中间移除轮次后保留尾整体换位，**重付 ≈ 保留尾大小 + 新内容**，本次
   保留尾实测 44–56K 真实 token（远大于外挂文件设计假定的 10–30K）；折叠间
   请求为纯追加（miss 137–3.4K），前缀机制正常。**定案**：折叠后其余内容
   照旧进外挂台账，视图只留最新**桥**（真实 token 预算）；形态甲=先定裁剪
   （完整轮边界）再对桥内容级截断；桥 = 最新完整轮（声明 + 全部工具回复
   结构全保留），超预算只截内容 + 指针；**桥默认 8K 真实 token**
   （`ORZ_FOLD_TAIL_TOKENS` 可配；实现以字符预算近似，初始 4 字符/token，
   S4 以折叠后首请求实际重付校准）；触发维持 128K（用户修正 120K 记忆）；
   **思维链不进桥、不进审计**——审计仅保留 reasoning_tokens 计数，桥视图
   剔除 reasoning_content（思维链具幻觉性、依赖实际数据与机械结果、存储
   压力更高）；指针消息文案更新一次（部署首轮一次性指纹变化）；外挂台账
   文件/行格式、压缩机制、白名单、preamble/配对 400 防线全部不变；
   `fold_tail_rounds` 语义退役。推算：总 miss 234.7K → 约 115K，命中率
   → 约 94%（零折叠成本上限约 95.9%）。性质：FUS-LEDGER-FOLD-STATE
   （§14.26–§14.28 外挂文件形态）修订；取代
   `LEDGER_FOLD_EXTERNAL_FILE_DESIGN_2026-08-18.md` §2.1/§2.2 保留尾语义。
   设计细节见 `LEDGER_FOLD_BRIDGE_TRUNCATION_DESIGN_2026-08-19.md`。
   实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（≥90%、无 400、每窗
   折叠重付 ≤ ~15K、截断频率 ≤30% 校准）；0c S4 前置，未闭合计数不变
   （29）。纯文档登记、未实施。
2. **折叠桥接截断 S1/S2 实施闭合 + 全面审查处理（2026-08-19，用户放行
   实施；FUS-LEDGER-FOLD-STATE 修订落地，orz 提交 52d698c，未推送）**：
   S1 代码——`bridge_cut` 桥预算裁剪（完整轮累加至预算、最新轮恒入桥、
   不完整轮回退；`FOLD_TAIL_CHARS_PER_TOKEN=4` ÷ 2 换算，默认 8K →
   16K 估计口径）、`build_request_view` 桥视图（reasoning 剔除、超预算
   内容级截断=工具回复保留尾部 + 「…（前略）」+ sha256 指针行、最终
   回复 199+…、声明 content/tool_calls 完整、消息不删轮不拆；`bridge_end`
   冻结=推进时消息末尾，折叠之间视图纯追加）、`advance_fold` 入账范围
   `[old_cut .. bridge_start)` 不变、指针文案更新（「约 8K 桥接内容，
   更早轮次已按行归档于 \<abs-path\>」）；`controller.rs` `fold_tail_rounds`
   退役 → `fold_tail_tokens`（默认 8K、`ORZ_FOLD_TAIL_TOKENS` 可配）；
   `agent_loop.rs` 推进触发与请求视图两处透传桥预算。S2 测试：orz-loop
   503 通过（+13 桥测试 +1 越界防御）、fmt 干净、clippy 与基线一致
   （lib 21 / test 26）。审查处理（2026-08-19 全面审查结论：实现无功能
   缺陷）——N1 指针文案按 §3.4 定稿原文落地；N2 实现决策登记（折叠路径
   新增 `bridge_cut`，`collapsed_cut` 保留服务压缩 drain，§2.5「压缩
   不动」）；N3 `bridge_end` 冻结机制补记（§3.1/§3.2，折叠之间纯追加的
   必要落点）；O1 轮内中间 assistant 文本按最终回复口径统一截断（§3.3）；
   O2 `build_request_view` 索引越界防御守卫（§7）；O3 非默认配置时指针
   文案固定不变（§3.4）。设计文档 §3.1–§3.4/§7 修订。S3 重建 → S4
   复验（≥90%、无 400、每窗折叠重付 ≤ ~15K、截断频率 ≤30% 校准）待续；
   0c S4 前置，计数不变（仍在 29）。
3. **折叠桥接截断 S3/S4 复验执行 + 换算系数校准（2026-08-19，用户指示
   重建 + 单题复验；orz 提交 52d698c 后校准调整待提交）**：S3 重建成功
   （orz-linux 07:07 新二进制；USTC/清华镜像源 502 不可达，改用阿里云
   镜像源完成依赖安装）。S4 单题复验（path-tracing 1800s 预算，07:08
   运行，job 2026-08-19__07-08-57）：reward 0.0（wallclock 耗尽正常
   结束）、**无 400**、113 请求；**journal 口径命中率 95.54%**（hit
   4,775,422 / miss 222,869；对照上次 88.57%，提升主因=折叠重付消失）；
   折叠 2 次（rounds_folded 37/82，view_estimate 147,164/132,328 →
   after 782/6,474）；**折叠后首请求重付 3,742 / 6,493 真实 token**
   （DoD ≤ ~15K 达标；对照上次 47.8K/50.4K/56.4K）；**截断频率 0%**
   （两次桥均未超预算，≤30% 达标）。**校准**：第二次折叠桥 12,948
   字符 → 重付 6,493 真实 token → 实测 ≈ **2 字符/真实 token**（初始
   保守值 4 高估一倍）——`FOLD_TAIL_CHARS_PER_TOKEN` 4 → 2 已调整
   （桥回到 8K 真实 token 目标；最新轮 6.5K < 8K 不截断）；orz-loop
   503 通过 / fmt 干净 / clippy 与基线一致。provider 口径待账单 CSV
   （07:00–08:00 时段）对拍确认；计数不变（29，provider 对拍确认后
   29 → 28）。
4. **折叠桥接截断 S4 provider 口径对拍确认 + 0c 验证闭环（2026-08-19，
   用户上传新账单；ADR-0010 §14.32 第 3 项续）**：provider 账单
   （07:00–08:00，cost 1.9082 元）**命中率 95.33%**（hit 4,896,384 /
   miss 239,752 / 115 请求）——**≥90% DoD 达标**；与 journal 口径
   （95.54%，113 请求）差异 2 请求（重试/边界），费用与单价验算完全
   吻合。综合 S4 四项判定（命中率 ≥90%、无 400、每窗折叠重付
   3,742/6,493 ≤ ~15K、截断频率 0% ≤30%）——**0c 验证闭环，未闭合
   计数 29 → 28**；换算系数校准（4 → 2）为 S4 既定产出，校准后代码
   待提交与下次正式跑分使用（本次判定基于已测 07:07 二进制，校准
   方向=桥 16K→8K 真实 token、最新轮 6.5K < 8K 不截断，行为风险低）。

### 14.33 v1.33 补写裁决索引（2026-08-19）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **输出退化防护与工具结果可再读闭环设计定稿（2026-08-19 用户裁决：
   8K 全统一限值 + 桥 8K 不动 + 补读闭环硬约束 + 前两层补强；
   FUS-STAGNATION / ORZ-CACHE-CONTEXT-COST 修订——make-doom 两次失败
   归因）**：2026-08-19 19:04/19:16 make-doom-for-mips 两次运行在
   35/56 请求后被模型请求 hang/传输中断终止（第一次 transport error、
   第二次 AgentTimeoutError）；第一次失败前最后一次模型输出 201,230
   字符 = 点读响应复述 + **2,316 次省略标注重复**（退化复读），
   `finish_reason=length` 后流中断；**历史先例 2026-08-11（479,957
   字符，复述 read_file 结果，同任务）**。排除网络（探针 60/60 稳定、
   容器内 deepseek 401 正常）；停滞守卫仅事后评估、失败轮次未调用；
   根因=模型退化复读 + `REQUEST_MAX_TOKENS=160_000` 放大 + 工具结果
   截断后无可再读闭环（指针不在模型工具面）。**定案**：①限值统一 8K
   ——终端工具输出 20K→8K、点读 8K 确认、桥 8K 不动（"只做一个限值"）；
   ②补读闭环硬约束——截断末尾机械附加"完整内容见 \<路径\>，请使用
   read_file（offset/limit 分页）"，落盘 `.gsa/session/terminal/*.log`
   可读性已验证（符号链接 canonical 在 git root 外，gitignore 放行）；
   点读指针改向落盘文件（存档兜底）、桥截断保留尾部含工具结果自身
   指针（sha256 兜底）；③生成期实时复读检测（on_chunk 连续相同块
   N=5 / 1K token 窗口重复率 >60% / 连续 3 次退化中断 →
   run_invalidated，治本）；④`REQUEST_MAX_TOKENS` 160K→32K（止损，
   正常轮次 p95 合计约 7K、max 23.7K）。准确度判定：8K 截断信息守恒、
   差异被闭环吸收，高严谨性任务"宁可多补读、不可缺信息"。桥 8K 不扩窗
   （扩窗收益递减，命中 -0.3~0.5pp）。性质：FUS-STAGNATION 修订 +
   ORZ-CACHE-CONTEXT-COST 参数修订。设计细节见
   `OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19.md`。实施路由 S1 代码
   → S2 测试 → S3 重建 → S4 复验（无退化中断、无 400、命中率 ≥90%、
   补读路径可用）；纯文档登记、未实施；设计轮不动计数（28）。

2. **S1 代码 + S2 测试实施闭合 + 全面审查处理（2026-08-19 用户放行
   实施；orz 提交 5e968ec + 审查处理 19b839f，未推送）**：S1 已落地——
   transport `on_chunk` 生成期退化检测（连续相同 delta N=5 / 累计 ≥1K
   token 且最近 1K token
   3-gram 重复率 >60%）、`StreamInterrupted{degeneration_detected}` 主动
   中断且不重试（已见输出，ADR-0007）、会话级连续计数
   `DEGENERATION_LIMIT=3` 达限带 `degeneration_limit_reached` 转
   `run_invalidated{status: "degeneration"}`（schema 枚举先行扩展）；
   `REQUEST_MAX_TOKENS`/`ModelConfig::max_tokens` 160K→32K（三 agent
   统一）；终端工具输出 20K→8K + 统一 read_file 补读指针（default/
   concise/chat-completion 三面）+ 落盘路径 display 规范化；点读指针改向
   落盘文件（存档兜底）；桥截断保留工具结果自身落盘指针（sha256 兜底）；
   失败轮次补 stagnation 重复信号审计评估。S2 测试：检测器单测、流式
   中断不重试（连接数=1）、计数达限/重置、32K 请求头断言、8K 截断指针
   三面、点读改向 + 非终端兜底、桥指针保留；orz-loop 510 通过 / orz-tools
   2763 通过（沙箱外）/ fmt 干净 / clippy 与基线一致（lib 21）；仓库门禁
   valid。**全面审查处理（2026-08-19）**：P1=点读指针改向原按响应信封
   `{"output": string}` 判定终端，该信封被 read_file/grep/run_tests 等
   text-output 动作共用（同一 `text_output_response_schema`），非终端
   receipt 超 8K 会得到不存在的落盘路径死指针、违反「指针路径必须真实
   可读」——修复为发放时落盘订单动作名（`ActionResult.action`，serde 默认
   None、旧归档安全回退），点读按动作 + 信封双判定，并补非终端 text-output
   回归测试；P3=终端指针块计入截断预算（内容 + 指针 ≤ 8K）、160K 陈旧
   注释清理（含 live 探针改 32K 与生产一致）、新增 .gsa 符号链接可读性
   专属测试、登记同步；解释登记=「同一 run 连续 3 次」字面不可达
   （run_failed 即终止 run），实现为会话级连续计数（成功请求重置），是
   防循环意图的可行实现。计数纪律：实施放行入账（28→29），S3/S4 验证
   闭环后 29→28；S3 重建 → S4 复验（无退化中断、无 400、命中率 ≥90%、
   补读路径可用）待续。登记于 BACKLOG 0d / TODO P0-0d / CLI_PROJECT_INDEX
   / OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19（§9 审查处理登记）。

3. **S3 重建 + S4 复验闭环 + S4 缺口修复（2026-08-20 用户放行；orz
   提交 f2cb1e0，已推送）**：S3 Linux musl 重建（ORZ-BUILD-MOUNT-001
   契约，两轮：build-20260819.log 11m04s / build-20260819b.log 3m56s，
   三件套时间戳更新）。S4 make-doom-for-mips 单题复验（三轮运行 +
   端到端探针）——无退化复读中断 / 无 hang（agent 全程活跃至 900s 任务
   预算耗尽）、零 400、journal 口径命中率 94.25%（124 请求，job
   2026-08-19__22-50-01）与 91.91%（73 请求，job 2026-08-19__23-25-53）
   均 ≥90%、补读路径真实可用（模型按截断收据指针 read_file 落盘文件
   三次成功：1–1000 行、offset/limit 分页 1001–1200 行、操作台订单
   复核 line-1190–1200）。**S4 发现并修复缺口（2026-08-19/20）**：终端
   截断收据指向 `.gsa/session/terminal/<order>.log` 并指示 read_file，
   但 orz-host `access_in_scope` 的「.gsa 树 agent-invisible」规则拒绝
   全部 .gsa 读取——第二轮复验中模型按指针补读被 `policy_denied`（对
   台账 current.md 的 grep 同被拒），补读闭环实际不可用，违反设计 §2
   「补读闭环为硬约束」。修复=在 `access_in_scope` 白名单会话 .gsa 卷内
   `session/terminal/*.log` 的 read_file/grep（对齐 GAP-RUN-TESTS 的
   `run_tests_output.txt` 受控 artifact 先例，ADR-0010 §3.8.3/F-09）：
   lexical 路径限 `session/terminal/` 目录 + `.log` 扩展名，canonical
   目标必须落在会话 cwd 或会话自身 .gsa 卷内（防符号链接外逃到其它
   .gsa 内部或任意主机路径）；顺带修正 run_tests 白名单为 symlink-aware
   比较（`.gsa` 为符号链接时 canonical 与 lexical 不同）。验证：orz-host
   单测 221 通过（并发下 1 条既有时序偶发超时测试单跑复过）、fmt 干净、
   clippy 无新增、重建成功；端到端探针（debian 容器 + 生产三件套 +
   ORZ_DEEPSEEK_API_KEY）确认截断→指针→read_file 全链路成功。计数：
   S3/S4 验证闭环 **29 → 28**。登记于 BACKLOG 0d / TODO P0-0d /
   CLI_PROJECT_INDEX / OUTPUT_DEGENERATION_GUARD_DESIGN_2026-08-19
   （§6 DoD / §9 复验发现登记）。

### 14.34 v1.34 补写裁决索引（2026-08-20）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **流式请求重试节奏设计定稿（2026-08-20 用户裁决：重试间隔缩短——
   idle 无数据判定 5s 一轮、10 次上限（总 50s 窗口），zero-chunk 重试
   窗口同步 50s；ADR-0007 / D-7 参数修订——P0-0d 换题复验后续）**：
   P0-0d S4 闭环后按用户指示换题复验（gpt2-codegolf，历史 7 次失败、
   build/run 型需读约 500MB 权重），三轮运行（00-22-19 / 00-24-29 /
   00-41-54）：第一轮首请求 20s idle 警告 → 90s 内连接中断 → zero-chunk
   指数退避重试 2 次（478ms/721ms）→ **32s 窗口耗尽**放弃
   （`NonZeroAgentExitCodeError`，瞬时连接错误）；第二/三轮首轮
   request→model_output 约 10.5/10.6 分钟（`AgentTimeoutError` 正常收尾），
   有效模型工作时间约 3 分钟、仅 8 请求、journal 口径命中率 85.19%
   （样本不足非机制退化；三轮均零 400、无退化中断、ACAF 票据全过）。
   **环境排查**：容器 DNS/TLS/TTFB 0.38s 正常、宿主机流式 TTFB 0.19s
   且带 tools 大请求 60–80s 完整流完（reasoning 持续流动非死线）、API
   探测稳定；对照昨天 make-doom S4 首轮 8.3s——差异在 DeepSeek 端首轮
   生成慢 + 一次瞬时连接错误，非容器/网络/机制退化。**定案**：
   `stream_idle_warn` 20s→5s、`stream_idle_timeout` 90s→50s
   （=5s×10 轮）、`request_retry_window` 32s→50s（zero-chunk 重试窗口
   同步；非流式 create 退避窗口同步放宽）、`request_max_retries` 10 次
   不变；idle 只看完全无数据（慢速 reasoning 流不误杀）、重试仍指数
   退避（不改为固定 5s 间隔）、退化中断不重试纪律不变、无新增配置旋钮、
   retry 参数参与请求头指纹（部署后首次请求一次性指纹变化，既有纪律）。
   设计轮不动计数（28）。设计细节见
   `STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md`。实施路由 S1 代码 →
   S2 测试 → S3 重建 → S4 复验（≥90%、无 400、无退化中断、首轮不再
   10 分钟级长等）；纯文档登记、未实施。

### 14.38 v1.38 补写裁决索引（2026-08-21）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **read_file 内容锚点下传与写前机械核证设计定稿（2026-08-21 用户裁决：
   锚=哈希值——内容摘要太重、read_file 须机械返回且助理层零理解、简单机械
   核证；仅针对 orz；先设计、不动作）**：问题=主 agent 读取快照后委派写订单，
   窗口内文件被别处改动时可能基于旧快照修改更新文档；助理层为纯机械无提示词
   执行层，只需"是否同一份"的等值判定。**定案**：①`read_file`（orz）文本路径
   统一返回内容锚点 `{size, mtime, sha256}`——大文件信封（§14.22）已有
   `content_sha256`、补 size/mtime，小文件全文路径同样返回锚点，锚点与内容
   同一读取快照生成；②主 agent 写订单携带期望锚点 `expected {size, mtime,
   sha256}`（委派消息透传，可选字段，缺失保持既有行为）；③orz 写门禁执行
   编辑前机械核证——stat 一次做 mtime/size 快速预检，重算当前 sha256 为权威
   比较；不匹配→拒单返回机械错误码信封（复用 `order_stale` 形态、入
   `console_order_rejected` 事件面），不执行任何编辑；④主 agent 收到拒单→
   重读→以新锚点重下（重读是唯一补救动作）。时间戳可被保留/取整（git
   checkout / cp -p / touch -r），"内容变而 mtime 不变"存在，故 mtime 仅作
   快筛、sha256 为权威。边界：校验-写入 TOCTOU 极小窗口接受（可选后续=临时
   文件+原子替换）；expected 必填加严为可选后续；哈希只答"是否同一份"、不答
   "差异是什么"（补救=重读）。性质：FUS-LARGE-FILE-READ-CONTRACT（§14.22）
   信封扩展 + 写订单契约；实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验，
   待用户放行实施；设计轮不动计数（27）。登记于
   [设计](../docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
   BACKLOG 0f / TODO P0-0f / CLI_PROJECT_INDEX。

2. **S1 代码实施登记（2026-08-21 用户指示开始 S1 实施；orz 工作树未提交）**：
   read_file 锚点——新增 `ReadAnchor {size, mtime, sha256}`；`FileContent`
   增可选 `read_anchor`（文本路径填充、PDF/PPTX/图片/二进制 None）；
   `ReadHandleEnvelope` 增可选 `mtime`；小文件 prompt 附 `[read anchor]`
   尾行、信封头增 `mtime=…`；锚点与内容同一读取快照（读后 stat、size
   不一致重 stat 一次）；full/concise 工具描述同步。写订单——`workspace.
   search_replace` 契约 schema 增可选 `expected_anchor`（size/sha256 必填、
   mtime 可空、sha256 模式 `^[0-9a-f]{64}$`）。写门禁——`issue_pending_
   console_order` 发放前 pre_issue 门：search_replace 携带 expected_anchor
   时 stat 快筛 size/mtime + sha256 权威比对，不匹配拒单（复用 order_stale
   信封形态：phase=pre_issue / step=protocol / code=content_anchor_mismatch）
   入 `console_order_rejected` 事件面、清槽写失败 receipt、零编辑；目标文件
   不存在（新建）跳过；**其余 stat/read I/O 错误 fail-closed 拒单**（同
   code、消息注明失败原因，如 "failed to stat/read target"）——无法取得
   当前内容锚点即不放行编辑；错误消息指引重读重下。2026-08-21 审查收口：
   补 `console_anchor_mismatch_rejects_before_issue_with_zero_edits` 与
   `console_anchor_io_error_rejects_fail_closed` 两条用例。验证：orz-tools
   read_file 202 / types::output 84 / orz-loop console 69 / orz-loop 全量
   544 + console_anchor 2 通过、fmt 干净、clippy 无新增告警（审查修复
   build_read_anchor collapsible_if）；orz-tools 全量 44 个 grep/glob 失败
   为本机 rg 环境性既有失败（stash 基线复现一致）。计数：实施入账 27 → 28
   （S2-S4 待续）。登记于
   [设计 §8](../docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
   BACKLOG 0f / TODO P0-0f / CLI_PROJECT_INDEX。

3. **S2 测试闭合登记（2026-08-21 用户指示开始 S2；orz 工作树未提交）**：
   新增 10 条用例。①read_file 锚点返回正确性——小文件（sha256/size 与读取
   快照逐字节一致、mtime 与 metadata 一致、prompt 附 `[read anchor]` 尾行）、
   空文件（size=0、空串 sha256、`File is empty.` 后附尾行）、大文件信封
   （mtime + content_sha256）；②PDF 路径无文本解码链（`raw_text_to_file_
   content` read_anchor 恒 None）；③prompt 尾行渲染与 `ReadAnchor` serde
   round-trip（mtime=None 省略字段、旧 reader 兼容）；④写门禁四场景——
   锚点匹配放行（mtime=null 跳过快筛、sha256 权威）、同 size 同 mtime 异
   内容 sha256 兜底 fixture（`FileTimes::set_times` 保留 mtime，快筛通过但
   哈希权威拒单）、陈旧拒绝→重读重下成功（S4 场景单元级预演）、
   expected_anchor 缺失保持既有行为；⑤错误信封/事件面完整断言——receipt
   error（step/code/message 含 expected/actual sha256 与 re-read 指引）、
   upstream expected/actual、trace 末事件 protocol/content_anchor_mismatch、
   事件面机械盖章（phase=pre_issue / step=protocol / round / plan_epoch /
   run_id）、零编辑。验证：orz-tools read_file 207 / types::output 86 /
   orz-loop 全量 550 通过（0 失败）、fmt 干净、clippy 无新增告警（30 条
   既有位置核对）、cargo check --workspace 通过。计数：仍 28（S3-S4
   待续）。登记于
   [设计 §9](../docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
   BACKLOG 0f / TODO P0-0f / CLI_PROJECT_INDEX。

4. **S3 重建登记（2026-08-23 用户指示登记 S3；Linux musl，
   ORZ-BUILD-MOUNT-001；计数不变仍 28，验证闭环后 28 → 27）**：本主题
   S1/S2 代码（orz `3ad09e4` + `554c131`）已随 orz HEAD `05231f7` 进入
   NGRAM-GUARD-CALIBRATION S3 构建轮（同一 Linux musl 三件套，无需重复
   构建）——容器增量构建（`rust:1.97-slim`；挂载 `D:\CLI:/orz`、工作目录
   `/orz/orz`；apt 阿里云镜像 + 官方 static.rust-lang.org + 静态 rg
   15.0.0 源码安装；`-j 1`）**BUILD_EXIT=0**；三件套时间戳
   **2026-08-23 08:50 HKT**（orz 104,664,664 B / orz-signer 1,388,592 B
   / orz-acaf-provision 1,206,568 B；SHA256 与最小可执行冒烟同
   NGRAM-GUARD-CALIBRATION 设计 §3）。对应源码=orz `05231f7` + 父仓库
   `35788a0`。S4 复验待续（read_file 锚点返回、写门禁陈旧拒单→重读重下、
   零误拒、事件面留痕；命中率 ≥90%、零 400）。登记于
   [设计 §10](../docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
   BACKLOG 0f / TODO P0-0f / CLI_PROJECT_INDEX。

5. **FUS-READ-ANCHOR-WRITE-GUARD S4 复验闭环（2026-08-23 用户指示补登记；
   复用 NGRAM-GUARD-CALIBRATION S4 同一批实机数据，sweep-s4n-g4 /
   sweep-s4n-g5（code-from-image 经充值后单题重跑 sweep-s4n-g5-cfi），
   k=1、官方方式、n-concurrent=1；**S4 验证闭环，计数 28 → 27**）**：
   新二进制（orz 05231f7 构建轮，104,664,664 B）10 试次实机复验——
   ① read_file 锚点返回：小文件文本路径实机可见（mteb-retrieve 终答
   引用「读取锚点 sha256=1dec6f9c…d24bb，size=…」；break-filter 137 次
   read_file 调用）；锚点形状（小文件 sha256/size/mtime + prompt 尾行、
   空文件、大文件信封 mtime/content_sha256、PDF 路径无锚点、serde
   round-trip）由 S2 单测锁定；② 写门禁陈旧拒单→重读重下：S2 单元级
   预演（陈旧拒绝→重读重下成功）覆盖修改后拒绝→重读重下全链，实机
   10 试次无陈旧写入自然发生；③ 零误拒：10/10 试次零
   content_anchor_mismatch 拒单，全部 search_replace/写入订单正常执行
   （tool_completed exit 0）；合计 22 次 console_order_rejected 全为
   模型输入错误（step_not_done 8 / invalid_arguments 12 / policy_denied
   2，无设计/机制误拒）；④ 事件面留痕：console_order_rejected 事件面
   实机留痕 22 条（phase/step/code/order_id 机械盖章）；read-anchor 拒单
   事件面形态（phase=pre_issue / step=protocol / code=
   content_anchor_mismatch、upstream expected/actual、re-read 指引、
   零编辑）由 S2 错误信封完整断言锁定；⑤ 命中率 ≥90%、零 400：10/10
   有 journal、94.11%–98.55% 全 ≥90%（含 video-processing 300 请求
   98.55%）；10/10 零 HTTP 400（仅余额 invalid_request_error 与网络
   zero_chunk transport_retry exhausted 两次异常，均非 400、非设计
   问题）。边界：expected_anchor 为模型可选下发字段，实机未出现携带
   场景（核证链路由 S2 四场景单测覆盖：锚点匹配放行（mtime null 跳
   快筛）、同 size 同 mtime 异内容 sha256 权威兜底拒单、陈旧拒绝→
   重读重下、缺失锚点保持既有行为）。登记于
   [设计 §11](../docs/READ_ANCHOR_WRITE_GUARD_DESIGN_2026-08-21.md) /
   BACKLOG 0f / TODO P0-0f / CLI_PROJECT_INDEX。

### 14.37 v1.37 补写裁决索引（2026-08-21）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **哨兵退化 fail-fast 化设计定稿（2026-08-21 用户裁决：harness
   fail-fast 方向有道理——失败本身意味着当前模式不适合当前任务；前置
   证据门=确认空转/重复与架构本身无关；确认后实施、显式标明终止原因；
   先设计、不动作）**：sweep r1-g1 12 次哨兵（8× stall + 4× rep，
   约 ¥2.5）归因=跨请求烧 stall 是恢复机制的副产品——降级梯是
   `generate_stream` 调用内局部变量（每请求回 high）、`degeneration_
   consecutive` 成功即清零，stall→成功→stall 永远到不了 3、可无限烧
   预算；deepseek-harness 源码对照=step 边界有界重试（normal 默认 2
   次，README"five retries"注释与实际代码不符）、无降级/无生成期哨兵、
   空即 step 失败。**定案（主案）**：①会话级 thinking 档位——哨兵触发
   后本 run 后续请求从降级档起、不再回 config 默认档；②哨兵计数单调
   （成功不再清零，仅 run 边界重置），达 `DEGENERATION_LIMIT=3` →
   `run_invalidated{status: degeneration}` 显式终止；③disabled 档哨兵
   即终止；④终止原因显式进 journal/TUI。严格案（哨兵即 step 失败、
   harness 原样）保留为对照。**S0 证据门**=下一批扫描采集哨兵触发上下文
   （轮次/近 3 事件/是否紧邻 blackboard 重读或折叠压缩后首请求），与
   机械结构块无稳定相关且跨任务分布不均才放行 S1。设计轮不动计数（27）。
   登记于
   [设计](../docs/STALL_DEGENERATION_FAILFAST_DESIGN_2026-08-21.md) /
   BACKLOG 0d / TODO P0-0d 后续。

2. **流式中段解码错误有界重试设计定稿（2026-08-21 用户裁决：按「无完整
   tool_calls 即重试（有界）」实施，无异议；先设计、不动作）**：
   dna-assembly Disabled 档重试遇 `error decoding response body`（已见
   chunk 后截断）按 ADR-0007「已见输出不重试」直接杀 run、reward 0；
   幂等性重新核对=工具调用只在响应完整成功后执行、失败流全部丢弃，错误
   路径重发无副作用，「已见输出不重试」的原始动机在错误路径不成立。
   **定案**：重试判定标准从「零 chunk」改为「**无完整 tool_calls**」——
   Transport/Timeout 类中断（含解码截断签名、stream ended without
   finish_reason）且失败流未解码出完整 tool_calls → 可重试（零 chunk
   维持 180s 窗口/10 次上限；已见 chunk 至多 1 次额外重试）；已见完整
   tool_calls / Model / Parse / Cancelled 维持不重试；重试计数入事件面
   （P1 既有登记项一并实施）。与 fail-fast 设计正交（解码重试不增减退化
   计数）。设计轮不动计数（27）。登记于
   [设计](../docs/MIDSTREAM_DECODE_RETRY_DESIGN_2026-08-21.md) /
   ADR-0007 修订注记 / BACKLOG 0d / TODO P0-0d 后续。

3. **S0 证据门通过 + S1/S2 实施闭合登记（2026-08-21；用户放行实施 +
   全面审查处理）**：S0 证据门（r1-g1 + r1-g2 两批、10 题、7 次触发对）
   判定通过——机械结构相关性 0 例强相关（g2 feal-differential 首请求
   stall 直接构成机械块无关反例；g1 schemelike 紧邻占比与基率一致）、
   跨任务分布不均（集中于复杂实现类），结合既有探针结论判定为模型/任务
   侧原因。**实施**（主案 ①-④ + 解码兜底，合并一次落地）：会话级
   thinking 档位 `session_thinking` + 哨兵计数 run 内单调（成功不再
   清零）+ disabled 档哨兵即终止 + detail 显式化（族 + consecutive +
   round，schema 增可选 `detail`）；解码兜底重试判定改「无完整
   tool_calls」（`wrap_no_tool_side_effects` + `has_complete_tool_call`
   双保险边界）+ 中段有界 1 次（`CHUNKED_MIDSTREAM_MAX_RETRIES`）；
   **事件面重试计数一并实施**——v0.2 新增 `transport_retry` 事件
   （recovered/exhausted，schema/fixtures/conformance/TUI 同步，
   run-event enum 54 项）。**正式路径 per-run 隔离修正**：长驻进程
   （ACP server）跨 run 共享 transport，原「仅 run 边界重置=新
   transport」假设仅一键 CLI 成立；现 `ModelGateway::for_new_run()`
   每 run 换新实例（controller `run_turn_inner` 开头调用，主 agent 与
   检索子代理共享同一 run 实例；`run_retrieval_subagent` 不再引用
   controller 常驻 subagent 字段），计数/档位零跨 run 泄漏、并发会话
   零干扰。**S4 验收口径**（fail-fast 设计 §4 修正）：单 run 哨兵预算
   有界=≤3 次触发 × 单次预算（原「≤ 首档 64K + 低档快速拦截」为
   schemelike 类场景期望非硬上限）。测试：orz-loop 544 / 0 失败 /
   3 ignored、orz-tui 178、Python conformance 230 通过、workspace
   check 通过；S3 重建 → S4 复验待续（与窗口 180s 批次合并一次到位）。

### 14.36 v1.36 补写裁决索引（2026-08-21）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **zero-chunk 重试窗口 50s → 180s（2026-08-21 用户裁决：简单拉长窗口；
   网络抖动本质是 DeepSeek 节点超时，降级无实际作用）**：第一轮 5 题
   冒烟扫描（sweep-r1-g1，冻结版 cf0be20）llm-inference-batching-scheduler
   trial 于 2026-08-20 19:55–19:56 UTC 连续 `error sending request`
   零 chunk 中断——transport 内链在 50s 窗口内完成 5 次指数退避重试
   （约 0.9s/1.7s/2.9s 递增）即窗口耗尽放弃，run 以
   `stream interrupted before any chunk (after 5 re-sends)` 非零退出、
   reward 0（约 1 分钟节点抖动杀死 30 分钟 trial）。**定案**：
   `RetryPolicy::request_retry_window` 50s → **180s**（model.rs 默认值，
   非流式 create 退避窗口同步放宽）；`request_max_retries` 10 不变
   （双上限先到者止——实测 10 次 ≈ 约 2 分钟重试跨度，窗口放宽后次数
   上限成为主要约束）；不引入降级（节点超时场景降级无增益，用户裁决）。
   边界保持 ADR-0007 §4：零 chunk 且错误类 ∈ {Transport, Timeout} 才
   重试、已见输出不重试、退化中断不重试、窗口含尝试时长（慢速失败仍
   受窗口约束）；retry 参数参与请求头指纹（部署后首次请求一次性指纹
   变化，既有纪律）。实施路由 S1 代码（model.rs 默认值 + 测试断言）+
   S2 测试已闭合；S3 重建 → S4 复验（与终端解码重试兜底设计批次合并
   时一次到位，待用户确认解码兜底边界）待续。登记于
   [STREAM_RETRY_RHYTHM_DESIGN](../docs/STREAM_RETRY_RHYTHM_DESIGN_2026-08-20.md)
   修订（§3.1 现值列）/ BACKLOG 变更记录 / TODO P0-0d 后续。

### 14.35 v1.35 补写裁决索引（2026-08-20）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **DeepSeek 输出预算恢复与空流止损设计定稿（2026-08-20 用户方向：评估
   256K+max、空流处理向官方 harness 靠拢、退化检测器大升级；先设计、
   不直接做）**：官方 deepseek-harness（llm-deepseek/llm-retry，2026-08
   master）源码对照——默认 `reasoning_effort=high`、`maxTokens=256_000`、
   idle 5 分钟、EMPTY_RESPONSE 空即错即退（normal 5 次、退避 500ms→10s +
   10% jitter、重试在 durable step 边界）、无生成期退化防护。今晚 7 次
   运行对照 + pcap + 账单对账定论：**空流根因链 = max 档思考 + 32K
   max_tokens 截断 reasoning → 完成型空响应 → D-6 原样重试再烧一遍**
   （10 分钟级空流链、废弃 reasoning 每请求约 32K）；退化检测器只喂
   content delta、空流场景全程沉默。**定案**：①`REQUEST_MAX_TOKENS`
   32K → **256K**（回落档 128K，S4 实测校准）；②D-6 空流链官方化收窄——
   完成型空响应快速有界重试 ≤2 次（500ms→10s+10% jitter）→ thinking 禁用
   降级；reasoning 族异常不原样、直接降级；③退化检测器升级为输出健康哨兵
   ——观测面扩到 content+reasoning+tool arguments，新增 reasoning 复读
   （**灵敏层**：1K 窗口 3-gram 重复率 >60%，循环特征出现即触发，不依赖大
   预算阈值）与 reasoning-stall（**预算兜底层**：自首 chunk 起 600s 无
   content/tool_calls、或 reasoning 估算 ≥64K tokens，OR 触发——**空转预算
   与 max_tokens 解耦**，256K 恢复后空转不随预算放大；
   `REASONING_CHARS_PER_TOKEN=2` 沿用桥校准值）；重试分类纪律=有可见输出
   不重试（content 族，ADR-0007）、无可见输出走降级；会话级
   `DEGENERATION_LIMIT=3` 三族共享。**二轮修订（2026-08-20，实测校准）**：
   合法难题首轮（WSL 宿主机 RUN-CLI-6a85f668，gpt2-codegolf 同任务）
   **17,757 reasoning tokens / 约 184s 后正常产出 plan**——原定 120s/16K
   会在其出结果前误杀，曾上调至 **240s/24K**。**三轮修订（2026-08-20，
   用户裁决：灵敏层负责快速、兜底负责兼容 max 思考）**：成本账（实测
   ¥4.592/M output）——32K ≈ ¥0.147、64K ≈ ¥0.294、128K ≈ ¥0.588、
   256K ≈ ¥1.176；现状空流链（2×32K+降级）≈ ¥0.30；**64K 兜底单次最坏
   成本 ≈ 现状整条链且消除链式等待**（stall 中断直接降级），故兜底定
   **600s/64K**（初值，S4 校准 300–900s / 32–128K）= 合法锚点约 3.6 倍
   思考空间；兜底管单次上限、D-6 管重试次数，两本账解耦、机制可控。
   **同日修订**：①idle 死线 50s→**30s**（取代 STREAM-RETRY-RHYTHM 未实施
   的 50s 定值；warn 5s / retry window 50s 不变）；②160K 复读浪费归因=
   架构工具设计（工具结果截断无可再读闭环）已由 P0-0d 修正，作为恢复 256K
   的安全依据（残留风险=reasoning 空转，与本设计新信号正交覆盖）；③重试层
   结论=保留 transport 内链（快速、降级出口、生成期止损、不重跑工具）+
   吸收官方空流节奏（快速有界退避），不迁移 step 边界，可观测性缺口登记
   可选后续（P1 重试计数入事件面）。设计轮不动计数（28）。
   设计细节见
   `DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md`。
   实施路由 S1 代码 → S2 测试 → S3 重建 → S4 复验（难题单题 + 账单对账 +
   空流率观测）；纯文档登记、未实施。

2. **S1 代码实施登记（2026-08-20 用户放行实施；暂不重建/测试）**：
   `REQUEST_MAX_TOKENS` / `ModelConfig::max_tokens` 32K → **256K**（回落档
   128K 注释保留）；`stream_idle_timeout` 50s → **30s**（warn 5s / retry
   window 50s 不变）；D-6 流式空流链官方化收窄（`generate_stream`：完成型
   空响应快速有界重试 ≤2 次、退避 500ms→10s+10% jitter → thinking 禁用
   降级 → 仍空显式失败；reasoning 族哨兵中断不原样、直接跳降级；content
   族维持不重试透传）；退化检测器升级为输出健康哨兵——观测面扩到
   content+reasoning+tool arguments，三族信号
   `content_repetition`（保留）/ `reasoning_repetition`（灵敏层，仅
   content/tool_calls 全空时启用）/ `reasoning_stall`（600s/64K 预算兜底、
   OR 触发、与 max_tokens 解耦），`REASONING_CHARS_PER_TOKEN=2` 估算 +
   usage 复核留痕；detail 前缀
   `degeneration_detected:content_repetition|reasoning_repetition|
   reasoning_stall`，`is_degeneration_detail` 收窄为 content 族 +
   新增 `is_reasoning_guard_detail`，`DEGENERATION_LIMIT=3` 三族共享不变；
   run 层失败轮次审计补触发族标签。既有断言同步（256K 请求头 / idle 30s
   默认值 / 检测器 feed 签名）；live 探针与指纹/注释同步。计数：实施放行
   入账 1 项（**28 → 29**），S3/S4 验证闭环后 29 → 28。登记于设计 §4.1 /
   BACKLOG 0d / CLI_PROJECT_INDEX / TODO P0-0d。

3. **S2 测试实施登记（2026-08-20 用户指示进行 S2）**：新增 15 项测试
   （orz-loop lib，527 通过 / 0 失败 / 3 ignored）——退化检测器单测 10 项
   （reasoning 复读灵敏层、content/tool 可见输出停用 reasoning 族、stall
   双信号 600s/64K OR 语义、无首 chunk 不触发、估算校准、空转预算与
   max_tokens 解耦、空流重试参数/退避形状）+ 空流链 e2e 5 项（完成型空
   响应快速重试 2 次→降级、链尾显式失败、reasoning 复读→降级、
   reasoning-stall→降级、重试中途哨兵→跳过剩余原样重试）；S1 已同步断言
   继续覆盖 256K 请求头 / idle 30s / content 退化不重试。回归：fmt 干净、
   clippy 无新增告警（transport.rs 零告警；lib 21 与基线一致）、
   `cargo check --workspace` 通过。计数不变（仍 29），S3/S4 验证闭环后
   29 → 28。登记于设计 §4.2 / BACKLOG 0d / CLI_PROJECT_INDEX / TODO
   P0-0d。

4. **S3 重建 + S4 复验 + 对账登记（2026-08-20 用户指示重建与复验＋对账；
   S1-S4 全部闭合）**：S3 Linux musl 重建成功（ORZ-BUILD-MOUNT-001 契约，
   三件套时间戳更新，orz 104.4MB，18:45）。S4 gpt2-codegolf 单题复验
   （job `2026-08-20__18-46-52`，RUN-CLI-6a86db35，wallclock 1740s 跑满、
   reward 0.0、无异常）——**完成型空流 0**（32K 截断空流链根因消除；首
   请求约 66s 出首输出，对照 32K 时代 10.5 分钟级）；**零 400、零 idle
   死线、零 timeout**；**journal 口径命中率 95.28%**（85 请求，hit
   3,110,656 / miss 153,990，≥90% 达成）；`reasoning_repetition` 灵敏层
   触发 1 次并直接降级收尾（10:58:24 拦截 → 10:58:34 降级轮正常产出）；
   reasoning-stall 兜底（600s/64K）零触发零误杀（合法 reasoning 峰值
   20,234 tokens，低于兜底；对照合法锚点 17.7K/184s）；用量（journal）：
   output 191,623 tokens（含 reasoning 170,906）、prompt 3,264,646
   （hit 3,110,656 / miss 153,990）、单请求最大 completion 20,776；按
   ¥4.592/M 估算输出成本 ≈ ¥0.88；控制台 CSV 待刷新后补精确对账。
   **校准结论**：600s/64K/30s 初值维持不调（无漏判无误杀）。**计数：
   S3/S4 验证闭环 29 → 28**。登记于设计 §4.3 / BACKLOG 0d /
   CLI_PROJECT_INDEX / TODO P0-0d。

5. **默认档 high + 三级降级梯修订定稿（2026-08-20 用户裁决：方案 B +
   中间档；先设计、不动作）**：S4 实测（256K+max：空流 0、命中率
   95.28%、复读灵敏层拦截 1/85 并降级收尾、stall 兜底零误杀）表明机制
   已稳、max 不再是必要工作点；思考禁用本身质量影响大（「快答模式」），
   用户裁决：①默认 `reasoning_effort` **max → high**（官方 harness
   默认档，官方工作点即 256K+high），`EnabledMax` 保留为显式可选档
   （难题专用，仍受哨兵保护）；②降级梯插入 **low** 中间档——**high →
   low → disabled → 失败**（空响应快速重试与 reasoning 族哨兵跳转共用；
   「middle」映射 DeepSeek `reasoning_effort=low`，官方四档 off/low/high/
   max 的中间档）；③兜底/重试节奏不变（stall 600s/64K 与 max_tokens
   解耦、idle 30s、`EMPTY_RESPONSE_MAX_RETRIES=2`、退避 500ms→10s+10%
   jitter、`REASONING_CHARS_PER_TOKEN=2`）；④指纹含 thinking 档 → 部署
   后首次请求一次性变化（既有纪律）。代价=失败路径多一轮完整思考（每级
   受 64K/600s 兜底保护），病态率低（S4 1/85）且复读数 K 内被抓，可
   接受。实施路由 S1 代码（`ThinkingMode` 增 `EnabledLow`、默认
   `EnabledHigh`、三级梯接线）→ S2 测试（high/low 请求头、三级梯路径、
   回归）→ S3 重建 → S4 复验（难题单题 + high vs max 成本/产出对照）。
   设计轮不动计数（28）；实施放行 28 → 29，验证闭环 29 → 28。登记于
   设计 §3.6/§4.4 / BACKLOG 0d / CLI_PROJECT_INDEX / TODO P0-0d。

6. **默认 high + 三级降级梯 S1 代码 + S2 测试实施登记（2026-08-20 用户
   指示进行 S1 与 S2；orz b72a0a4 已推送）**：`ThinkingMode` 增
   `EnabledLow`（`reasoning_effort=low`）、默认档 `EnabledMax` →
   `EnabledHigh`（官方默认档）、`EnabledMax` 保留显式可选档；新增
   `apply_thinking` 统一 thinking 块 + `reasoning_effort` 双旋钮（
   `build_request` 仅按 config 映射，梯级/覆盖档必须同时覆盖两旋钮——
   修复「high 配置降级到 low 时 effort 仍为 high」的隐患）；
   `generate_stream` 降级梯 **high → low → disabled → 失败**——空响应
   每档快速有界重试 ≤2 次（换档重置计数与退避，每档独立 500ms→10s+10%
   jitter 节奏）、reasoning 族哨兵逐级下降一档；**`EnabledMax` 显式档
   保留 S4 验证基线**（哨兵命中/空流链耗尽直跳 disabled，三级梯按默认
   high 起定义，不额外多烧 high/low 两轮）；请求头指纹 thinking 映射
   含 high/low；探针注释同步。**S2 测试**：默认 high + max/low 显式档
   请求头断言（原 max 默认断言改 high）、三级梯 e2e（空响应
   high→low→disabled→失败、哨兵 high→low、low 级哨兵→disabled）、既有
   max 基线测试核对。回归：orz-loop lib **531 通过 / 0 失败 / 3 ignored**
   （+4 项）、fmt 干净、clippy 与基线一致（lib 21 / test 28 均既有位置，
   transport.rs 零告警）、`cargo check --workspace` 通过。计数：S1 实施
   放行入账 28 → 29（S2 不改变未闭合计数），S3/S4 验证闭环后 29 → 28。
   登记于设计 §4.5 / BACKLOG 0d / CLI_PROJECT_INDEX / TODO P0-0d。

7. **默认 high + 三级梯 S1 全面审查处理登记（2026-08-20 用户指示处理
   审查全部问题；orz 1651f59，已提交、未推送）**：审查结论=未发现功能
   缺陷，设计合理、实现合理、设计与实现符合；处理 5 项观察级建议——
   **O1（已知边界）**：非流式 `generate` 链保持「原样重试 1 次 →
   thinking 禁用」基线、不引入 low 档（三级梯作用域明确为
   `generate_stream`；`generate` 仅服务 preflight/gate 快轮），登记为
   有意不对称；**O2（代码）**：`build_request` 与 `apply_thinking` 双份
   thinking 映射补同步注释（直调 build_request 仅测试场景，改档位两处
   须同步）；**O3（登记）**：`empty_response_backoff` 的 60s
   max_elapsed_time 为参数表外兜底（每档 ≤2 次重试不可达）；**O4（测试
   补强）**：新增 `config_fingerprint_reflects_thinking_tier`——默认档
   指纹 == 显式 EnabledHigh，且与 low/max/disabled 互异（设计 §3.6
   「指纹含 thinking 档」补断言），orz-loop lib 532 通过 / 0 失败 / 3
   ignored（+1 项）、fmt 干净、clippy 基线一致（lib 21 均既有位置，
   transport.rs 零告警）；**O5（已接受）**：e2e 请求体子串匹配登记为
   已接受边界（mock 可控、无实际风险）。计数：审查处理不改变未闭合计数
   （仍 29），S3/S4 闭环后 29 → 28。登记于设计 §4.6 / BACKLOG 0d /
   CLI_PROJECT_INDEX / TODO P0-0d。

8. **默认 high + 三级梯 S3 重建 + S4 复验登记（2026-08-20 用户指示推送
   后重建、换题复验；orz 1651f59 已推送；S3/S4 验证闭环 29 → 28）**：
   S3 Linux musl 重建成功（ORZ-BUILD-MOUNT-001 契约，三件套时间戳
   12:08，orz 104.4MB）。S4 **换题 make-doom-for-mips**（P0-0d 退化
   防护起源题）单题复验（harbor job `2026-08-20__20-08-50`，trial
   `make-doom-for-mips__rTheTXd`，RUN-CLI-6a86ee6c，wallclock 1740s
   跑满、reward 0.0、零异常）——**完成型空流 0**（194 个 model_output
   双空 = 0、零空响应重试警告）；**零 HTTP 400、零 idle 死线、零
   timeout**；**journal 口径命中率 92.18%**（hit 8,122,240 /
   miss 689,249，194 请求，≥90% 达成）；reasoning 复读 / stall / idle
   全零触发、**stall 兜底零误杀**（600s/64K/30s 初值维持不调）；首输出
   延迟 5.5s（对照 max 基线约 66s）；用量（journal）：output 169,182
   tokens（reasoning 130,689，占 77%，对照 max 89%）、prompt
   8,811,489（hit 8,122,240 / miss 689,249）、单请求最大 completion
   7,082；输出成本估算 ≈ **¥0.78**（对照 max 基线 ¥0.88）。**high vs
   max 跨题参照**（任务不同，非严格同题）：high 档 30 分钟内 194 请求 /
   333 工具轮（max 85 请求），每轮更快、reasoning 占比更低、输出成本
   更低；命中率 92.18% 达标但低于 max 95.28%，属跨题差异非档位回归；
   input 侧不可直接对照（工具轮多 2.3 倍致 prompt 8.8M vs 3.3M）。
   **计数：S3/S4 验证闭环 29 → 28**。登记于设计 §4.7 / BACKLOG 0d /
   CLI_PROJECT_INDEX / TODO P0-0d。

9. **模型舒适度原则 + 会话数据边界先导引导登记（2026-08-20 用户指示：
   不过度限制模型、适度控制幻觉、让模型舒服些；orz d250f11，已提交、
   未推送）**：将「模型舒适度」确立为显性设计原则——机械层负责兜底与
   防错（退化哨兵/权限门禁/契约校验），把自由留给模型的思考与规划；
   限制集中在「不让模型出事」与「不让模型分心」（隐私边界/工具面收敛/
   上下文管理），正常路径零打扰。落地=提示词层改善（无新机械限制）：
   `BASE_SYSTEM_PROMPT` 读取纪律块后新增「会话数据边界」引导——`.gsa`
   树（journal/台账/会话日志）为运行时内部数据、不进入直接工具面，正向
   引导走 blackboard_read 分区读取或操作台动作反馈，并如实告知「直接
   访问只会得到拒绝」——减少模型反复碰壁（本次 make-doom 复验 14 次
   `.gsa` 直读尝试的提示词层归因：模型起初不知道边界，被拒后才改走
   受控面）。`system_sha256` 变化 → 部署后首次请求一次性指纹变化（既有
   纪律）。测试：新增
   `base_system_prompt_carries_session_data_boundary_guidance`；
   orz-loop lib 533 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 基线
   一致。登记于 CLI_PROJECT_INDEX；未闭合计数不变（28）。

10. **上下文机械结构块 PUSH→PULL 重设计 S1 实施 + 全面审查处理登记
    （2026-08-21 用户放行实施 S1，审查后指示处理全部问题；orz 7529a71）**：
    设计定稿见
    `docs/CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md`（方案 A
    先行、C 暂缓、状态行保留；§8 全面审查处理登记）。S1 代码：退役每工具轮
    `[TOOL_ROUND_BUDGET] REMAINING` 尾随注入（agent_loop.rs D-8 注入点 +
    prompt.rs `tool_round_budget_remaining_block` 移除，零残留）；系统提示词
    总预算块保留并改为「按需经 `blackboard_read section=session` 读取」
    （静态一次，前缀缓存纪律不变）。`blackboard_read` 新增 `section=session`
    live 会话面——controller `render_session_section` 返回 BUDGET/USED/
    REMAINING（数据源=in-run tool_rounds，含 activation 累计 /
    max_tool_rounds）+ `render_status_line`；live 面不进 epoch 归档
    （session+epoch 显式报错）；session+receipt_id 越权显式报错；工具定义
    enum/描述增量。机械硬门禁（`budget_insufficient` 预检文本含剩余 / 上限
    耗尽块 / `run_invalidated`）原样保留。S2 测试随 S1 交付：协议形态 5→4
    消息、无 REMAINING 尾随断言、session 面渲染/越权单测 + 工具级回达
    （`blackboard_read_serves_session_section` /
    `blackboard_read_session_combination_errors_are_explicit`）、
    `budget_insufficient` 拒绝文本仍含剩余、既有 TOOL_ROUND_BUDGET 断言更新
    （console_s4 三例改「无 REMAINING + 拒绝文本含剩余」、
    `round_budget_declared_static_no_per_round_remaining`）。验证：orz-loop
    536 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 与基线一致（31）、
    workspace check 通过、事件一致性 15 通过。全面审查（设计合理性/实现
    合理性/符合性）结论=无功能缺陷；处理：O4 session 越权组合升级参数级
    显式报错（ToolCompleted exit_code 1 + error 字段，同非法 epoch/receipt_id
    纪律；既有 receipt_id+非 actions 渲染层先例保持 exit_code 0，差异登记
    设计 §8）、O5 补工具级越权回达测试、O6 工具描述去内部标签、O1/O2/O3
    登记已接受边界（REMAINING 口径含在飞轮差 1 / PULL 读取消耗工具轮 /
    状态行双通道）。登记于 CLI_PROJECT_INDEX / BACKLOG 0e / TODO P0-0e；
    未闭合计数不变（28）至 S3/S4 闭环。

11. **上下文机械结构块 PUSH→PULL S4 复验阻断与缺口修复（2026-08-21）**：
    7529a71 冻结版 S4 单题复验（make-doom-for-mips，job
    `2026-08-21__01-18-47`）首轮工具轮后第二轮请求被 DeepSeek 拒绝：
    `reasoning_content must be passed back`（invalid_request_error）→
    `run_failed`。根因=PUSH→PULL 退役 REMAINING 尾随 user 消息后，暴露
    2026-08-04 遗留的「工具输出汇总 assistant 文本消息」（`assistant_parts`
    块，2026-08-06 协议修复后已是冗余副本）成为请求末条；DeepSeek thinking
    模式对「工具结果后紧跟的 assistant 文本轮」强制要求回传
    reasoning_content。API 探针 V1–V7 实测钉死触发面（V1 400 / V2 旧形态
    200 / V3 带 rc 200 / V4 无汇总 200 / V5–V7 纯文本轮 200）。修复=
    退役 `assistant_parts` 汇总消息（工具结果已以 Role::Tool 完整落库，
    冗余移除同时节省每轮输入 token），协议形态 4→3；同步更新两处协议
    形状测试 + clippy 未用变量清理；orz-loop 536 通过 / 0 失败 / 3
    ignored、fmt 干净、clippy 无新增。设计细节见
    `docs/CONTEXT_SCAFFOLDING_PULL_REDESIGN_DESIGN_2026-08-21.md` §9。
    **2026-08-21 S3 重建（修复版）+ S4 复验闭环（计数 28→27）**：修复版
    Linux musl 重建（orz cf0be20，三件套哈希刷新 FROZEN；bookworm 冒烟全过）。
    S4 单题复验（make-doom-for-mips，job `2026-08-21__01-48-26`，
    RUN-CLI-6a873e04，wallclock 1740s 跑满、reward 0.0、零异常）——
    **journal 命中率 94.45%**（147 请求，hit 6,503,808 / miss 382,108）
    ≥90% 且高于上轮同题 92.18%；零 HTTP 400、零 idle 死线、哨兵/stall 全零
    触发；output 165,644 tokens（reasoning 77.8%）对照基线 169,182/77%
    基本持平；工具轮 226（基线 333）——REMAINING 零残留 + 汇总消息退役，
    输入增长放缓达成；终态 `run_invalidated{status:wallclock}`（正常预算
    耗尽）。S1-S4 全部闭合，计数 28 → 27。

12. **上下文机械结构块 PUSH→PULL 方案 C 裁决登记（2026-08-21 用户裁决：
    暂不收紧、先看当前情况）**：0e S4 复验闭环后，S4 实测哨兵/stall 全零
    触发（content_repetition / reasoning_stall 0/147）、journal 命中率
    94.45%，`REQUEST_MAX_TOKENS` 256K 无收紧必需性，维持现状暂不实施
    方案 C（64K 回落档）。后续正式跑分（冒烟扫描 → 89 题 5 批）中观察
    哨兵触发率与输出预算，若回升/失控再评估 64K（方案 C 重新启用）。
    登记于设计 §6 / BACKLOG 0e / TODO P0-0e / CLI_PROJECT_INDEX；
    计数不变（27）。

13. **复读检测粒度改滚动哈希任意偏移设计定稿（2026-08-21 用户裁决：同意
    实现；先登记、不直接动作）**：dna-assembly 复跑（RUN-CLI-6a885faa，
    11m57s、reward 0）误杀实证——触发点 seq 95 完整输出 6439 字符为
    连贯正常 DNA 组装分析（finish_reason=length、无 tool_calls、无 ≥20
    字符连续重复，但含低熵特征 ttttt/aaaaa/ggggg/N N N N N/GGTCTC），
    现有路径①「连续 5 个相同 content delta」在 DeepSeek 小 chunk 粒度 +
    低熵文本下天然命中（detail="5 identical content deltas in a row"）。
    **定案**：路径①替换为**滑动窗口 + 滚动哈希任意偏移重复检测**——维护
    最近 L+W=144 字符缓冲（尾部 48 字符 L-gram + 最近 96 字符比较区）、
    记录比较区内（起点偏移 ∈ [48, 96]）全部 48 字符 L-gram 哈希；新
    L-gram 哈希在比较区内已出现（起点偏移 ≥48）即触发，哈希命中后字符级
    比对防碰撞；语义=流中存在 ≥48 字符内容与邻近之前的 48 字符完全相同，
    **窗口内任意周期可命中（p ≤ 96）**（修复固定偏移相位对齐缺陷：周期
    10 短语循环在 16/48 固定偏移下相邻窗口永不相同）；同字符连串 ≥96
    触发、DNA 正常序列免疫、O(1)/字符摊销。**否决链**：拉长 N（治标）、
    内容熵过滤（真复读同低熵被放过）、
    「重复字符后切块」（语义不自洽）、固定 16 切块 ×3（相位对齐缺陷）。
    3-gram 路径②（≥1K token 窗口 >60%）保留兜底（另行裁决）；content/
    reasoning 两族共用算法（灵敏层语义不变）；stall 兜底（600s/64K）与
    fail-fast 纪律（会话级档位/计数单调/disabled 即终止/detail 前缀）
    不变。成熟产品佐证=openclaw text-repetition-guard suffix cycle
    （任意偏移重复，建议阈值 ≥40 字符模式重复 ≥5 次，量级一致）。设计轮
    不动计数（28）。设计细节见
    `DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md`
    §3.3/§4.8；实施路由 S1 代码（transport.rs）→ S2 测试 → S3 重建 →
    S4 复验（dna-assembly 低熵误杀消除 + 真复读仍触发）。登记于
    BACKLOG 0d / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

14. **复读检测粒度滚动哈希 S1 代码实施登记（2026-08-21 用户放行实施；
    orz 工作树未提交）**：`DegenerationDetector` 复读判定路径①（连续相同
    delta N=5）替换为滑动窗口滚动哈希任意偏移——退役
    `DEGENERATION_CONSECUTIVE_DELTAS` 与两个 `recent_*_deltas` 字段，
    新增 `RollingRepetitionWindow`（最近 L+W=144 字符缓冲 + 记录区 48
    字符 L-gram 哈希集，新尾部 L-gram 哈希命中后字符级比对防碰撞；
    常量 `REPETITION_MIN_RUN_CHARS=48` / `REPETITION_WINDOW_CHARS=96`）；
    `feed_repetition` 路径①改字符流级判定（两个起点距离 ≥48 的相同 48
    字符 span 触发，与 delta 切块粒度无关）、路径②（≥1K token 3-gram
    >60%）保留兜底；content/reasoning 两族共用、detail 前缀不变、触发
    描述更新。既有断言同步：5×"same"/"think"/"hi" 不再触发，改为 96
    字符重复 span；3-gram 测试内容重构（共享核心 <L + 互异长尾部）。
    验证：orz-loop 550 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无
    新增告警（transport.rs 新代码零告警）、`cargo check --workspace`
    通过。计数：实施放行入账 **28 → 29**（S2 不变，S3/S4 闭环后回 28）。
    登记于设计 §4.9 / BACKLOG 0d / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。

15. **复读检测粒度滚动哈希 S2 测试实施登记（2026-08-21 用户指示进行
    S2）**：新增 8 项测试（orz-loop lib 550 → **558 通过 / 0 失败 /
    3 ignored**）——短低熵块不触发（5×"a"/5×"same"）、DNA 低熵样本
    6439 字符不触发（ACGT + 散点短特征，确定性伪随机）、poly-A 精确
    阈值（95 不触发/96 触发）、周期 10 短语循环触发（对齐缺陷回归，
    content 与 reasoning 灵敏层各一）、单一大 chunk（2000 字符互异）
    不触发、`spans_equal` 字符级比对直接验证（相同判等/单字符差异判
    不等）；真实 u64 多项式哈希碰撞构造不可行（B=1_000_003 奇数、48
    位置），**碰撞用例登记为已接受边界**；近重复（单字符差异）不触发
    + 精确复读触发补充用例。回归：fmt 干净、clippy 无新增告警
    （transport.rs 仅 2 条既有 doc 告警）、`cargo check --workspace`
    通过。计数不变（仍 29），S3/S4 闭环后 29 → 28。登记于设计 §4.10 /
    BACKLOG 0d / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。
16. **停滞守卫退役 + 缺口 A 审计留痕实施登记（2026-08-22 用户裁决：
    生成期检测覆盖实际退化面，跨轮停滞守卫一并全部退役）**：
    **退役**（S4 复验实证：跨轮停滞守卫对全会话 3–8-gram 统计、阈值
    10，几乎所有长会话误杀 restart_requested——dna-assembly 等
    12–341 次重复触发；0d 记录其「未拦截」make-doom 真实退化；复读
    主要显现在思维链，生成期 reasoning 灵敏层已接管）——主/子代理
    链路 `evaluate_stagnation` 终止判定、失败轮次审计、`stagnation.rs`
    模块、`RuntimeStagnationGuard` 事件（v0.2 面）、TUI 投影、Python
    reference/verifier/schema/doctor 清单全部退役；v0.1 冻结面保留
    （历史 replay）。**缺口 A**（退化触发内容留痕）：transport 触发
    时 WARN 输出重复 span + 两匹配偏移 + 窗口尾部；DNA 重跑
    （RUN-CLI-6a88905f，26m26s、136 请求、命中率 98.64%、
    `run_finished completed`）实证 reasoning 层 2 次触发（consecutive
    1→2）均为正常思考对任务内容的重复引用（DNA 序列等式 + 技术短语
    的 48 字符 span），**误杀坐实非病态复读**；停滞守卫退役后 run
    不再 `run_invalidated restart_requested`。orz-loop 557 通过、
    verifier 230 通过、assurance 1588 通过、doctor 仅剩 orz 未提交
    dirty。登记于设计 §3.3/§4.11 / BACKLOG 0d / TODO P0-0d 后续 6 /
    CLI_PROJECT_INDEX。
17. **复读判定再校准设计定稿（2026-08-22 用户裁决：L=200 + 流内累计
    3 次命中才中断+降级；content/reasoning 统一；先落设计；S1/S2 已
    实施 2026-08-22）**：
    缺口 A 实证 reasoning 灵敏层 48 字符任意偏移对正常任务内容重复
    引用误杀（DNA 序列等式/技术短语 span）。**定案**：
    `REPETITION_MIN_RUN_CHARS` 48→**200**、`REPETITION_WINDOW_CHARS`
    96→**400**（=2L、缓冲 144→600）；触发门槛改**流内累计命中 ≥3 次
    才中断+降级**（命中计数不因中间未命中内容重置；1–2 次命中仅审计
    留痕；计数随流结束丢弃，会话级 consecutive 与 `DEGENERATION_LIMIT`
    不变）；content/reasoning 统一；3-gram 兜底与 stall 兜底/fail-fast
    纪律不变。判定语义=流内出现 ≥3 次完全相同的 200 字符 span（任意
    偏移、起点距离 ≥200）才触发。漏判边界=短周期小量循环（<200 字符
    仅 2–3 次）不再触发（危害可控）。**S1/S2 实施闭环（2026-08-22）**：
    transport.rs 常量与命中门槛落地（`REPETITION_HIT_LIMIT=3`、窗口
    命中后继续喂入、1–2 次命中仅审计留痕 WARN、3-gram/stall 兜底不
    变）；orz-loop 560 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无
    新增、workspace check 通过。同日 S1 全面审查处理（审查发现 4 项
    全部处理）：设计 §3.3 信号表与头部同步再校准参数；`feed_chars_capped`
    命中上限喂入（达门槛停止消费超大退化帧，子门槛全量消费语义不变）；
    审计 WARN 与触发判定同 chunk 聚合（触发时不再单独输出「audit
    only, not tripping」误导文案）；3-gram/stall 触发清空
    `trigger_context`（防残留旧 span 误标为本次触发）。验证仍 560 通过、
    fmt 干净、clippy transport.rs 仅 2 条既有 doc 告警。设计细节见
    设计 §3.3 修订 / §4.11；
    实施不改变计数（仍 29，S3/S4 验证闭环后 29 → 28）。登记于 BACKLOG
    0d / TODO P0-0d 后续 6 / CLI_PROJECT_INDEX。
18. **复读判定二级再校准设计定稿（2026-08-23 用户裁决：L=400 + 二级
    标点块内部重复确认；G4 冒烟实证代码引用型误杀仍存在；先落设计；
    2026-08-23 **S1 实施**（transport.rs 常量 + 二级确认逻辑），S2 待
    放行）**：第四轮冒烟（sweep-r1-g4-official，官方方式 k=1、无
    max_wallclock）5/5 reward 1.0、零真实 400、命中率 93.66–98.48%；
    sam-cell-seg reasoning 层 2 次触发（203/204 字符 span）均为模型
    推理中完整引用读过的代码块——**L=200/3 命中无法豁免「同流内完整
    重复引用 ≥200 字符内容 ≥2 遍」结构性模式**；查证非设计泄露（提示词
    无哨兵参数、模型无规避行为）。**定案**：`REPETITION_MIN_RUN_CHARS`
    200→**400**、`REPETITION_WINDOW_CHARS` 400→**800**（缓冲 600→1200）、
    命中门槛 3 不变（同字符连串触发线 402→802）；新增二级「标点块内部
    重复确认」——命中后按标点+空白切块，大块内部标点块重复覆盖占比
    ≥`REPETITION_PUNCT_BLOCK_MIN_RATIO=0.50`（初值）才计命中（**每对
    命中都过**）、无切分点直接判真；3-gram 路径②与 stall 兜底不变。
    判定语义=流内 ≥3 次「400 字符完全相同且内部由重复标点块构成（或
    无切分点）」的 span 才触发。漏判边界=<400 字符短循环、结构性重复
    内容、无标点长引用（概率可控）。验证方案=G4 样本离线回放 + 构造
    样本 + S3 重建 + S4 冒烟复验。设计轮不动计数（仍 29）。设计细节见
    设计 §3.3 修订 / §3.5 / §4.11；登记于 BACKLOG 0d / TODO P0-0d
    后续 6 / CLI_PROJECT_INDEX。**2026-08-23 S1 审查处理（5 项全部
    处理）**：§3.3 信号表同步 L=400/W=800 + 二级；拒绝候选审计按 delta
    聚合为「一条摘要 + 命中计数」（用户裁决）；触发 chunk 审计条目不再
    被 trip 分支丢弃；切分符集合常用子集边界登记；补阈值边界/每对确认
    累计/G4 短引用形态回放三项测试（orz-loop 565 通过）。**2026-08-23
    S2 测试实施登记（用户放行 S2）**：正式 S2 轮=离线回放记录 + 结果
    归档——新增字节级真实回放测试（从 sweep-r1-g4-official
    sam-cell-seg__CT5JrD3 agent/orz.txt WARN 记录提取旧 L=200 两次触发
    的真实匹配 span（各 200 字符；设计早前 203/204 为完整重复引用区域
    口径），reasoning 族「引用-再确认循环」形态回放 3 遍）——首级 L=400
    均不命中（无候选、无审计、无触发）；orz-loop lib 566 通过 / 0 失败 /
    3 ignored、fmt 干净、clippy 无新增；计数不变仍 29（S3/S4 验证闭环后
    29 → 28）。**2026-08-23 S3 重建登记（用户放行）**：Linux musl
    （ORZ-BUILD-MOUNT-001 契约，build_orz_aliyun.sh；rust:1.97-slim
    容器增量构建，挂载 D:\CLI:/orz、工作目录 /orz/orz、-j 1）
    BUILD_EXIT=0；三件套时间戳 2026-08-23 02:01（orz 104,521,992 B /
    orz-signer 1,388,592 B / orz-acaf-provision 1,206,568 B，SHA256 见
    设计 §4.11）；最小可执行冒烟通过（orz 无 TTY 报 TUI io error 属预期、
    provision 打印 usage、signer 报 manifest 缺失）；对应源码=orz
    6178050 + 父仓库 dfe39a1；计数不变仍 29。登记于 BACKLOG 0d / TODO
    P0-0d 后续 6 / CLI_PROJECT_INDEX。**2026-08-23 S4 冒烟复验登记（用户
    放行；G4/G5 对照，sweep-s4-g4/g5，k=1、官方方式；**S3/S4 验证闭环
    29 → 28**）**：G4 4/5 reward 1.0——sam-cell-seg 零复读触发（对照旧
    L=200 二进制同题 2 次触发+2 次降级，误杀消除且 reward 仍 1.0）、
    portfolio-optimization 零触发；G5 2/5 reward 1.0（path-tracing-reverse
    AgentTimeoutError=官方超时，机制无异常）；全 10 试次零真实 400；有
    journal 9 试次命中率 96.30%–98.34% 全 ≥90%；构造真循环触发由 S2 测试
    套件覆盖（566 全绿）；观察项=video-processing 3-gram 兜底触发 1 次
    （0.60 边界、降级一次、任务继续，与旧轮 portfolio-optimization 同界，
    登记不改范围）。登记于 BACKLOG 0d / TODO P0-0d 后续 6 /
    CLI_PROJECT_INDEX。
19. **AGENT DELIVERY FLOW 设计定稿（2026-08-23 用户三轮讨论裁决；先落
    设计、未实施）**：来源=S4 冒烟失败归因（四失败均非架构根因）暴露的
    两个架构摩擦与一个核实压力——① 计划步自动推进语义（controller 订单
    成功即 `mark_step_done(receipt)`，步骤"完成"=订单执行而非目标达成，
    mteb 出现"计划完成但计算未跑"）；② 引用校验硬阻断无修正
    （unresolvable_citation 后模型文本永不提交）；③ 订单反馈缺 diff/delta
    （workspace_delta 事件层已算但模型不可见，模型需自行 grep/读文件/diff
    确认落实）。**定案**：A1 步骤重定义=执行顺序标记（目标交交付门仲裁，
    中途动作幻觉由「交付门拒绝→有界修正→失败」收敛）；A2 模板末步固定
    「递交/完成」且不随普通订单自动推进（杜绝 echo done 虚假递交）；A3
    递交状态=黑板 plan 机械渲染（复用 workspace_delta、过滤 .gsa/临时
    文件、上限 20+计数；工具输出不进内容流→无复读风险；模型无声明）；
    A4 订单反馈增强（编辑类回显 diff、终端类挂 delta、actions 板只加
    `changed: N files` 短计数）；A5 交付门=最终回答引用校验+验证器仲裁；
    B 引用失败→有界修正机会一次（返回 reason_codes+markers、不重置状态、
    同失败 2 次恢复硬阻断、journal 记 attempt）；终端内容登记引用暂缓。
    实施路由 S1-S4；设计轮不动计数（28）。设计细节见
    `AGENT_DELIVERY_FLOW_DESIGN_2026-08-23.md`；登记于 BACKLOG 0d 后续 7
    / TODO P0-0d 后续 7 / CLI_PROJECT_INDEX。
20. **N-GRAM GUARD CALIBRATION 设计定稿（2026-08-23 用户裁决；先落设计、
    未实施）**：来源=两次真实任务 3-gram 路径②边界误触发（旧轮
    portfolio-optimization 与本轮 video-processing，ratio 均显示 0.60、
    实际 0.600–0.609、正常推理收尾自引用、单发即 trip、非致命）。
    **定案**：`DEGENERATION_NGRAM_REPEAT_RATIO` 0.60→**0.70**（`>` 严格
    大于保留）；新增**流内累计命中**（每次 feed 超阈值计 1 次、累计
    ≥3 才 trip、1–2 次仅审计（ratio+窗口+族）、间隔不重置、流结束丢弃，
    与路径①纪律对齐）；统一口径（WARN `{:.2}`→`{:.3}`/原始值、信号表
    同步）。边界登记：0.60–0.70 近重复循环漏判由 stall 兜底（600s/64K）
    兜住；两次真实触发无原始字节、验证靠单测+e2e+实机观察。实施路由
    S1-S4；设计轮不动计数（28）。设计细节见
    `NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md`；登记于 BACKLOG 0d
    后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
21. **AGENT DELIVERY FLOW S1 实施 + S2 测试闭合（2026-08-23 用户放行；
    orz 工作树未提交）**：计划步语义重定义——`PLAN_FIRST_FRAMEWORK_BLOCK`
    步骤语义改写（执行顺序标记、完成=该批订单已执行、目标交交付门仲裁、
    末步固定递交/完成）+ `plan_write` 校验强制末步 id ∈ {`deliver`,
    `submit`}（不合法末步机械拒绝）；`record_console_receipt` 对末步订单
    成功不再自动置 done（仅显式递交路径推进）；`console_step_done` 对末步
    显式拒绝（`console_step_done_terminal_step`，防 direct 证据门绕过递交
    门）；新增无参 `submit` 工具（console 默认态主车道、双阶段——首次机械
    计算交付状态渲染进黑板 plan 末步状态行 [delivery] 状态: N 个变更、同
    动作再触发确认置 done；交付状态=工作区 walk 相对计划批准基线的 delta，
    过滤 .gsa/缓存目录、上限 20+计数行+截断标记、无模型声明）。订单反馈
    增强——workspace delta 单源化迁入 orz-loop host.rs（`WorkspaceDeltaEntry`
    补 size、`TOOL_DELTA_MAX_ENTRIES`、walk/diff 公共函数），orz-host 对
    `run_terminal_cmd` 调用前后 walk 附加 delta、`run_tests`/通用路径透传
    ToolResult 字段；console receipt 编辑类订单（search_replace）回显 diff
    （有界 8K 截断）、终端/运行类挂 workspace_delta+truncated；actions 板
    只加 `changed: N files` 短计数。引用校验有界修正——首次失败 journal
    `citation_validation{decision:retry, attempt:1, correction_allowed:true}`
    并把失败报告作为用户消息注入（injected block 不持久化）、模型重写后
    重新走完整最终回答门；同失败第二次恢复硬阻断（attempt:2/block）。
    Schema/verifier 同步（citation-validation-event-payload-v0.2 增
    decision=retry + attempt/correction_allowed；tool-completed-event-payload
    -v0.1 delta 条目补 size；run_event_journal_validation.py retry 语义）。
    S2 测试闭合——新增/适配：末步 terminal 校验（deliver/submit 接受、
    非 terminal 拒绝）、submit 两阶段状态渲染与确认、submit_not_current
    拒绝、console_step_done 末步拒绝、search_replace diff receipt +
    changed 计数、run_terminal delta receipt + changed 计数、引用修正一次
    后通过/二次硬阻断；orz-loop 574 通过 / 0 失败 / 3 ignored，fmt 干净，
    clippy 无新增告警（与基线一致）。实施入账 28 → 29；S3 重建 / S4 复验
    待续。登记于设计 §6 / BACKLOG 0d 后续 7 / TODO P0-0d 后续 7 /
    CLI_PROJECT_INDEX。
22. **AGENT DELIVERY FLOW S1 全面审查处理（2026-08-23 用户指示处理审查
    全部问题）**：审查结论=S1 设计与实现整体一致（机制完整、事件面/工具面
    收敛、schema/verifier 三同步），无功能缺陷；处理如下——① 修复项：
    F1 过滤规则单测锁定（新增 `workspace_delta_walk` 排除面单测 + 元数据
    diff 单测）、O2 terminal 判定改按末步 id ∈ {`deliver`,`submit`}
    （`planning::is_terminal_step`，旧/恢复计划末步为普通 id 时保持 S1 前
    自动推进与 console_step_done 语义，新增 predicate 与 legacy 行为单测）、
    O6 actions 板截断 delta 短计数渲染 `changed: N+ files`；② 口径/边界
    登记：F2 引用修正=本 run 至多 1 次修正总数（attempt 不按失败类型分，
    第二次失败无论 reason 均 block）、O1 递交状态为信息展示非最终回答硬门
    （跳过 submit 直接终答由引用校验+验证器仲裁，S4 观察）、O3 修正轮重走
    引用校验而 counterexample 门 once-only 不重跑、O4 交付基线仅 plan_write
    新 epoch 捕获（恢复/run_plan 路径 fail-closed「变更清单不可用」）、O5
    submit 每次全工作树 walk 成本登记、O7 末步强制不设豁免（全量强制）。
    orz-loop 579 通过 / 0 失败 / 3 ignored，fmt 干净，clippy 无新增告警。
    计数不变（仍 29，审查处理属 S1 内收尾）；S3 重建 / S4 复验待续。登记
    于设计 §6.4 / BACKLOG 0d 后续 7 / TODO P0-0d 后续 7 /
    CLI_PROJECT_INDEX。
23. **N-GRAM GUARD CALIBRATION S1 实施（2026-08-23 用户放行；实施入账
    29 → 30，S2-S4 待续）**：`DEGENERATION_NGRAM_REPEAT_RATIO` 0.60→**0.70**
    （`>` 严格大于保留）；新增 `NGRAM_HIT_LIMIT=3`——3-gram 路径②由
    「单发即 trip」改为**流内累计命中**（每次 feed 时 1K token 窗口
    ratio > 0.70 计 1 次命中、累计 ≥3 才 trip；1–2 次仅审计留痕=ratio+
    窗口 token 数+族、间隔不重置、流结束丢弃——与路径①纪律对齐）；
    WARN 口径 `{:.2}`→**`{:.3}`**（trip detail 与审计条目同步）；信号表/
    参数表同步（DEEPSEEK_OUTPUT_BUDGET... 设计 §3.3/§3.5）。既有 3-gram
    ratio 用例适配：单份 core 的 ratio≈0.694 恰为 0.69x 边界样本（留给
    S2 作「不触发」断言）、用例改双份 core（ratio≈0.825 > 0.70）+ 断言
    2 次子门槛审计条目（3-gram 口径）与 trip detail 的 `3/3`；滚动哈希
    路径①/stall 兜底/会话级 DEGENERATION_LIMIT 语义不变。orz-loop 579
    通过 / 0 失败 / 3 ignored，fmt 干净，clippy 无新增告警（transport.rs
    仅 2 条既有 doc 告警、位于改动区外）。登记于设计
    `NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md` / BACKLOG 0d 后续 8 /
    TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
24. **N-GRAM GUARD CALIBRATION S2 测试实施（2026-08-23 用户放行）**：
    新增 6 项单测——① 0.69x 边界：content 单份 8 词 core（ratio≈0.694
    < 0.70）永不计数、永不触发（+ reasoning 镜像，两族统一）；② 0.70x
    边界：9 词 core（ratio≈0.720 > 0.70）窗口填满后每次超阈值 feed 计
    1 次命中，审计 1/3→2/3、第 3 次 trip 3/3（`>` 严格大于两侧明确
    断言）；③ 命中按 feed 粒度：单个约 1200 token 高重复 feed 仅计 1 次
    （1 审计、不 trip），第 2/3 个同类 feed 才累计至 trip；④ 间隔不重置：
    命中 2 后插入单 feed 300 互异 token（窗口重复率压到 0.70 以下、间隔
    自身不计命中），`ngram_hits` 保持 2，重灌恢复超阈值后第 3 次命中即
    trip（3/3）；⑤ 流结束丢弃：新流（新 `DegenerationDetector`）首命中
    仅审计不 trip——计数不跨请求累积；⑥ 既有 3-gram 用例适配已于 S1
    完成（0.694 边界样本转为 ①）。orz-loop **585 通过 / 0 失败 / 3
    ignored**（579 + 6 新增），fmt 干净，clippy 无新增告警。实施入账
    不变仍 30（S2 属同一实施里程碑，S1 已入账）；S3 重建 / S4 复验待续。
    登记于设计 `NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md` / BACKLOG
    0d 后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
25. **N-GRAM GUARD CALIBRATION S1 全面审查处理（2026-08-23 用户指示处理
    审查全部问题）**：审查结论=S1 设计与实现整体一致、无功能缺陷；处理
    2 项——① D1 边界登记修正（校准设计 §2.4）：0.60–0.70 近重复循环
    漏判的「stall 兜底」仅对 reasoning 族成立（stall 触发条件=无
    content/tool_calls 且 reasoning 在流动）；content 族为已接受漏判——
    精确循环仍由路径① 400 字符精确匹配兜住、带变体循环输出至
    REQUEST_MAX_TOKENS（256K）截断（成本受上限约束）、S4 复验观察实机
    表现，若出现真实 content 族退化再评估保留低阈值或独立兜底；基础
    设计 §3.3 注记同步；② I1 审计日志字段（调用方审计 WARN 按路径标注
    命中门槛 `rolling_hit_limit` / `ngram_hit_limit`，消除 NGRAM_HIT_LIMIT
    日后独立调整时 3-gram 条目被误标的风险）。orz-loop **585 通过 / 0
    失败 / 3 ignored**、fmt 干净、clippy 无新增告警。计数不变仍 30
    （审查处理属 S1 内收尾）；S3 重建 / S4 复验待续。登记于设计 §2.4 /
    BACKLOG 0d 后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
26. **N-GRAM GUARD CALIBRATION S3 重建登记（2026-08-23 用户指示进行
    重建；Linux musl，ORZ-BUILD-MOUNT-001 契约，`build_orz_aliyun.sh`；
    计数不变仍 30，S4 复验闭环后回落 30 → 29）**：容器增量构建
    （`rust:1.97-slim`；挂载 `D:\CLI:/orz`、工作目录 `/orz/orz`；apt
    阿里云镜像 + 官方 static.rust-lang.org + 静态 rg 15.0.0 源码安装；
    `-j 1`）**BUILD_EXIT=0**；三件套产物时间戳 **2026-08-23 08:50
    HKT**（orz 104,664,664 B / orz-signer 1,388,592 B / orz-acaf-
    provision 1,206,568 B；SHA256 见设计 §3）。最小可执行冒烟=三件均
    正常加载执行（orz 无 TTY 报 TUI io error 属预期——headless 真机面
    由 S4 任务容器验证；provision 打印 usage；signer 报 manifest 缺失）。
    对应源码=orz 05231f7 + 父仓库 35788a0。待续：S4 复验（正常任务零
    3-gram trip、0.60–0.70 流仅审计、≥0.70 三连才 trip、零真实 400、
    命中率 ≥90%；content 族 0.60–0.70 区间实机观察）。登记于设计
    `NGRAM_GUARD_CALIBRATION_DESIGN_2026-08-23.md` §3 / BACKLOG 0d
    后续 8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
27. **N-GRAM GUARD CALIBRATION S4 复验闭环（2026-08-23 用户放行；
    G4/G5 对照，sweep-s4n-g4 / sweep-s4n-g5（code-from-image 经充值
    后单题重跑 sweep-s4n-g5-cfi），k=1、官方方式（无 max_wallclock）、
    n-concurrent=1；**S3/S4 验证闭环，计数 30 → 29**）**：新二进制
    （orz 05231f7 构建轮，104,664,664 B）实机运行——G4（git-multibranch
    / sam-cell-seg / portfolio-optimization / video-processing /
    mcmc-sampling-stan，2h05m，3/5 reward 1.0）**全 5 题零 3-gram trip、
    零哨兵触发、零 400**（含历史误触发对照：portfolio-optimization 与
    video-processing 旧轮各 1 次 3-gram 0.60 边界触发，本轮均零触发；
    video-processing 本轮 AgentTimeoutError=官方超时、机制无异常、
    journal 完整 300 请求；sam-cell-seg 零复读触发）；G5（5 题，主轮
    4/5 后 harbor 遇 httpx ConnectError 网络抖动退出，resume 补跑
    code-from-image 时 API 余额不足 NonZeroAgentExitCodeError，充值后
    单题重跑 reward 1.0）**全 5 题零 3-gram trip、零触发、零 400**；
    两次异常均非哨兵、非 400、非设计问题。命中率 10/10 有 journal：
    94.11%–98.55% 全 ≥90%。构造流行为（判定层实测）：0.694 不计数不
    触发、0.720 三连才 trip、e2e 退化流中断+降级全绿（全量 orz-loop
    585/0/3）。观察项：10 试次均无 3-gram 审计条目（历史 0.60–0.61
    边界带未再现；content 族 0.60–0.70 实机样本仍空，保持 §2.4 已接受
    漏判登记）。**计数：S3/S4 验证闭环 30 → 29（AGENT-DELIVERY-FLOW
    S3/S4 待续，闭环后 29 → 28）。**登记于设计 §3 / BACKLOG 0d 后续
    8 / TODO P0-0d 后续 8 / CLI_PROJECT_INDEX。
28. **AGENT DELIVERY FLOW S3 重建登记（2026-08-23 用户指示登记 S3；
    Linux musl，ORZ-BUILD-MOUNT-001；计数不变仍 29，S4 闭环后 29 →
    28）**：本主题 S1/S2/审查处理代码（orz `663be55` + `ed8e902`）已随
    orz HEAD `05231f7` 进入 NGRAM-GUARD-CALIBRATION S3 构建轮（同一
    Linux musl 三件套，无需重复构建）——容器增量构建（`rust:1.97-slim`；
    挂载 `D:\CLI:/orz`、工作目录 `/orz/orz`；apt 阿里云镜像 + 官方
    static.rust-lang.org + 静态 rg 15.0.0 源码安装；`-j 1`）
    **BUILD_EXIT=0**；三件套时间戳 **2026-08-23 08:50 HKT**（orz
    104,664,664 B / orz-signer 1,388,592 B / orz-acaf-provision
    1,206,568 B；SHA256 与最小可执行冒烟同 NGRAM-GUARD-CALIBRATION
    设计 §3）。对应源码=orz `05231f7` + 父仓库 `35788a0`。S4 复验待续
    （计划无空转、末步递交、引用修正、订单反馈、零 400、命中率 ≥90%）。
    登记于设计 §6.5 / BACKLOG 0d 后续 7 / TODO P0-0d 后续 7 /
    CLI_PROJECT_INDEX。

29. **AGENT DELIVERY FLOW S4 复验闭环（2026-08-23 用户指示补登记；复用
    NGRAM-GUARD-CALIBRATION S4 同一批实机数据，sweep-s4n-g4 /
    sweep-s4n-g5（code-from-image 经充值后单题重跑 sweep-s4n-g5-cfi），
    k=1、官方方式、n-concurrent=1；**S4 验证闭环，计数 29 → 28**）**：
    新二进制（orz 05231f7 构建轮，104,664,664 B）10 试次实机复验——
    ① 计划无空转：8 个完成试次全部按计划步执行至末步、最终回答绑定
    receipts/交付状态，无「计划完成但计算未执行」空转形态（历史 mteb
    形态未再现——本轮 mteb 实际执行计算并写出 /app/result.txt 后递交）；
    ② 末步递交走机械交付状态：8/8 完成试次调用无参 submit 双阶段
    （首次机械渲染 [delivery] 状态行进黑板 plan → 模型 blackboard_read
    section=plan 复核 → 再确认置 done 后终答；git-multibranch 终答
    引用「交付状态显示 0 个工作区文件变更——属预期（基础设施任务）」；
    sam-cell-seg 出现末步当前订单被 step_not_done 拒绝（"current step
    is deliver, not observe"）——末步不随普通订单推进语义实机生效）；
    ③ 引用修正一次/二次硬阻断：portfolio-optimization / break-filter /
    mteb-retrieve 均出现 citation_validation retry（attempt=1、
    correction_allowed=true）→ 注入失败报告重写 → 二次同失败 block
    （attempt=2、correction_allowed=false）；git-multibranch /
    sam-cell-seg retry 一次后通过；④ 订单反馈：search_replace /
    run_terminal 订单全部成功执行并走 receipt 点读链（每订单后
    blackboard_read receipt_id=ORD-…），diff/delta 与 changed 短计数
    机械形态由 S2 单测锁定、交付状态行实机可见；⑤ 零真实 400：10/10
    试次零 HTTP 400（仅余额 invalid_request_error 与网络 zero_chunk
    transport_retry exhausted 两次异常，均非 400、非设计问题）；
    ⑥ 命中率 10/10 有 journal：94.11%–98.55% 全 ≥90%（含
    video-processing 300 请求 98.55%）。边界：O1 递交为信息展示非最终
    回答硬门——本轮 8 个完成试次均走 submit 路径、无跳过递交样本；
    O4 恢复/run_plan 路径 fail-closed「变更清单不可用」本轮无样本。
    登记于设计 §6.6 / BACKLOG 0d 后续 7 / TODO P0-0d 后续 7 /
    CLI_PROJECT_INDEX。

### 14.31 v1.31 补写裁决索引（2026-08-19）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **黑板读取缓存成本处理设计定稿（2026-08-19 用户裁决：大机制不再更改、只
   补回应命中而未命中的缓存成本；FUS-REQUEST-CACHE / ORZ-CACHE-CONTEXT-COST
   修订——S4 换题复验归因）**：path-tracing 正式 1800s 预算复验（40 次请求、
   reward 0.0、无 400）provider 口径命中率 85.43% / journal 86.04%；归因=
   **8/8 大 miss 尖峰（≥10K，合计约 190K = 总 miss 64%）全部紧跟
   `blackboard_read`（actions/exec）**——单次分区结果 17–32K token 作为全新
   工具结果注入，前缀缓存按位置匹配无法命中；非折叠频率、非大机制问题。
   **定案**：大机制（折叠 + 机械压缩 + 黑板/外挂台账）保留；`blackboard_read`
   渲染瘦身——actions 结果板去 response JSON（`order_id/ok/step/code/trace_id`，
   缺失回退 `?`）、exec 行截断 200 字符 + 段总长 4K 字符上限、registration/order
   板不变；（阶段 2 可选）`since` 扩展至 actions + 读取频率提示词引导；零模型
   调用、工具契约/schema 不变、折叠（128K）/压缩（192K/200K）阈值不动。
   预期命中率 86% → 约 95%。设计细节见
   `BLACKBOARD_READ_CACHE_COST_DESIGN_2026-08-19.md`。性质：FUS-REQUEST-CACHE
   修订；实施路由 S1 渲染瘦身 → S2 测试 → S3 重建 → S4 复验（0c S4 前置，
   不新增未闭合计数）。**2026-08-19 账单核对补充**：Codex 窗口账单
   （`api_key_name=codex`，08-19 00:00–03:00）563 请求、聚合命中率 97.91%
   （97.12%/98.05%/99.53%）、单请求 miss 均值 3,937（1.45–5.4K 递减）——
   orz 复验为 85.43%、单请求 miss 均值 7,428，差距全部集中在 blackboard_read
   尖峰；修复后 orz 命中率约 95% = Codex 水平下沿（详见设计 §2.1）。纯文档
   登记、未实施。

2. **S1/S2 渲染瘦身实施完成 + 全面审查发现 F1 + F1 处理定稿=方案 B 按需点读
   （2026-08-19）**：S1 渲染瘦身已实施（orz 子模块工作树，未提交）——actions
   结果板固定形态行（`order_id/ok/step/code/trace_id`，缺失回退 `?`，复用
   summary `failure_envelope_fields`）、exec 行截断 200 字符（复用
   `truncate_chars`）+ 段总长 4K 上限（超限仅头行+计数行）；orz-loop 479
   通过 / fmt 干净 / clippy 与基线一致。**审查发现 F1（P1，设计层面）**：
   设计假设「详情留 TraceStore、模型按需经 trace 回查」在 console 默认面
   机械上不成立——`assistant.trace` 不在 console 直接工具面（只能经订单
   下发），其响应同样是结果栏 receipt、同样被瘦身隐藏；执行类订单
   （run_script / run_tests / run_terminal）的成功 response 与失败信封
   message/upstream 因此不可回查。**用户 2026-08-19 裁决：方案 B（按需
   点读）**——`blackboard_read` 新增可选 `receipt_id`（仅 actions 分区；
   值=结果行行首 order_id；支持 epoch 归档点读；与 since 组合时忽略
   since；非法/未找到显式报错），点读返回固定形态行 + 有界完整内容
   （成功 response / 失败 error 全文，`RECEIPT_DETAIL_MAX_CHARS=8_000`
   字符，超限截断 + 存档/TraceStore 指针）；无 receipt_id 时整段输出与
   S1 逐字节一致（缓存最优）。零模型、纯机械、黑板数据面/事件面不动，
   仅工具定义增量扩展（header 指纹部署首轮一次性变化，v1.19 纪律）。
   实施路由 S1 代码（epoch.rs 点读分支 + controller 参数解析 + 工具定义）
   → S2 测试 → S3 重建 → S4 复验；计数不变（0c S4 前置修订）。设计细节
   见 `BLACKBOARD_READ_CACHE_COST_DESIGN_2026-08-19.md` §4.5。
   **2026-08-19 方案 B S1 代码 + S2 测试已实施（用户放行；orz 提交 ad74714、
   未推送）**：
   epoch.rs `render_section` 增可选 `receipt_id`（非 actions 分区携带=显式
   报错；actions 分支点读优先）+ `render_receipt_point_read`（固定形态行 +
   成功 `response=<JSON 完整内容（重序列化）>` / 失败 `error=<JSON 完整内容
   （重序列化）>`——键/值/嵌套完整、非字节级原文（键序/空白可能规范化，
   全面审查 O3 登记），`RECEIPT_DETAIL_
   MAX_CHARS=8_000` 超限截断 + 「…」+ 存档/TraceStore 指针行；未找到显式
   not found + 旧 epoch 归档提示）；controller.rs 参数解析（非字符串/空串=
   显式报错、exit_code 1）与透传（live 与 epoch 归档两条路径共用）、
   blackboard_read 工具定义参数/描述增量扩展（事件面不变）；S2 单测 5 项
   （完整 response/error、8K 截断+指针、未找到、非 actions 报错、无
   receipt_id 与 S1 逐字节相等）+ 工具级 4 项（点读回达、非法参数报错、
   非 actions 报错、跨 epoch 点读）；orz-loop 488 通过 / fmt 干净 / clippy
   与基线一致（lib 21 / test 26）。S3 重建 → S4 复验（≥90%、无 400）待续。
   **2026-08-19 全面审查处理登记（用户指示处理审查全部问题；orz 提交 +
   父仓库指针、未推送）**：N1 点读截断尾部记账注释修正（「…」已计入
   truncate_chars 输出，注释与数学口径对齐）；N2 receipt_id trim 规范化口径
   登记（前后空白忽略、trim 后为空同空串报错，schema minLength 1 与运行时
   一致性）；N3 `blackboard_read` section 非字符串显式报错（同非法
   epoch/receipt_id 纪律，绝不静默回退 "plan"，消除与 receipt_id 组合时的
   误导性报错；新增工具级测试）；N4 本项及设计稿/BACKLOG/TODO 提交状态措辞
   统一为「已提交、未推送」；O1 点读次数无机械上限登记为已接受边界（单次
   ≤8K 字符；频率引导属阶段 2 §4.4 可选后续、不占计数）；O2 当前 epoch 超
   8K live receipt 的存档指针轮转前不成立、TraceStore 不在 console 直接
   工具面，登记为已接受边界（8K 为用户定档，console 面文件读取可按
   trace_id 直达）；O3 「JSON 原文」措辞收敛为「重序列化完整内容」（见上）。
   实施侧：orz-loop 489 通过 / fmt 干净 / 无新增 clippy 告警。

### 14.30 v1.30 补写裁决索引（2026-08-19）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **压缩注意事项槽 HA 结构化事实聚合设计定稿（2026-08-19 用户裁决：D1=(c)
   设计定稿、先设计不直接动作；FUS-COMPACTION-REDESIGN 修订——接续 §14.29
   D1=(b) 的阶段 (c)）**：重看 HA（Home Assistant，助理层借鉴的上游开源项目）
   当前实现与相关源码后定稿——HA 的 State/CompressedState（`s/a/c/lc/lu`
   压缩、`lu==lc` 省略、context 仅 id 时降字符串）、StateMachine 变更判定
   （same_state && same_attr 只推 last_reported 不重登记）、recorder 字典化/
   哈希去重、assist 管线事件流、`GetLiveContextTool` 静态投影+动态回查分离，
   映射为本设计的机械事实聚合边界。**注意事项槽=HA 结构化事实聚合（助理层唯一
   新增输出）**：数据源全部为 controller 已机械写入的结构化记录——`plan.steps`
   中 `Failed(receipt_id)` / `Blocked` 步骤（step id + 目标 + receipt_id）、
   `exec.errors` 最近 5 条（每条截断约 200 字符）、`actions.results` 最近 3 条
   失败 receipt（order_id / step / code / trace_id）；排序=计划面失败/阻塞 →
   执行错误 → 动作失败；空时显示「（无注意事项）」；上限 ≤3K，超限截断并给
   「其余 N 条见 blackboard_read 分区/摘要存档」指针（2026-08-19 全面审查补充：单条超长整行放不下时退化为仅指针、N 计 1，由 blackboard_read 回查恢复；同一失败事件可同时以步骤行与动作失败行双视角呈现，属有意冗余）；压缩内部失败
   （`guard_failed` / `archive_write_failed` / 外挂台账写入失败）继续走 marker
   既有独立标注、不进本槽。**后续衔接槽不交助理层**（用户裁决）：不聚合任何
   「当前步/下一步/待办」内容，避免限制或机械性误导（助理层不变量=不理解语义，
   机械建议可能与主模型实际评估冲突）；保持固定中性占位，措辞显式声明「后续
   衔接由主模型自行判断」，仅保留回查入口（`blackboard_read` 分区 + 摘要存档 +
   外挂台账路径）。**不变项**：零模型调用；五槽结构与 17K 合计上限；存档恒写入、
   marker 恒带 digest；schema 无变化（两槽仍为字符串）；仅 `summary.rs` 聚合
   渲染与 `run_template_compact` 接线变化。**已知边界**：同一 plan epoch 内
   多次压缩时 `exec.errors`/失败步骤跨 marker 重复——滚动单 marker 模型只见
   最新一份、重复为仍成立的事实，接受；`exec.errors` 黑板侧无界累积（渲染取最近 5）为已知边界，控制器侧加保留上限属可选后续、不占计数。性质：FUS-COMPACTION-REDESIGN
   （§14.10/§14.29）修订；实施路由 BACKLOG 0c / TODO P0-0c（S1 代码 → S2 测试
   → S3 重建 → S4 命中复验）；设计细节见 CONTEXT_COMPACTION_DESIGN §4.4。
   纯文档登记、未实施；未闭合计数不变。
   **2026-08-19 S1/S2 实施闭合登记（用户放行实施）**：S1 代码——
   `summary.rs` 新增 `render_facts_notes`（HA 结构化事实聚合：plan 失败/
   受阻步骤 + exec 错误最近 5 条（每条截断约 200 字符）+ 动作失败 receipt
   最近 3 条；排序=计划面→执行错误→动作失败；空时「（无注意事项）」；
   ≤3K 超限截断 + 「其余 N 条见 blackboard_read 分区/摘要存档」指针）与
   `render_notes_capped`/`truncate_chars`/`failure_envelope_fields` 辅助；
   后续衔接占位改中性措辞（「后续衔接由主模型自行判断」+ 回查入口含外挂
   台账路径）；`run_template_compact` notes 槽接线 `render_facts_notes`；
   存档/marker 空 notes 防御回退同步为「（无注意事项）」。S2 测试——
   summary 事实聚合单测 6 项（空态/三源排序/最近 5+截断/最近 3 失败含
   信封缺失回退/3K 溢出指针/单条超长退化指针）+ 压缩 e2e 1 项（marker+存档三源事实槽同序）
   + 空黑板 e2e 断言「（无注意事项）」；orz-loop 473 / 0 失败、fmt 干净、
   clippy 与基线一致（lib 21 / test 26）。S3/S4 待验证；计数不变
   （0c 验证闭环后 29 → 28）。

### 14.29 v1.29 补写裁决索引（2026-08-18）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确取代以下既往条款。

1. **压缩机械模式（2026-08-18 用户裁决：B 定案，D1=(b)；FUS-COMPACTION-
   REDESIGN 修订——撤销 §14.10 五段模板摘要的 LLM 调用部分）**：账单对账
   （`amount-2026-08-18` CSV vs journal `model_output` 事件）发现——压缩摘要
   调用（`run_template_compact` 内 `summary_system_prompt` + 空 tools + 折叠
   视图）以**独立系统提示词**发起模型请求，provider 前缀缓存从第 0 token 起
   全部 miss；22:17 复验运行 3 次压缩 ≈ 9 次摘要调用 + 1 次未定位请求 = 10
   个账单请求无 journal 对应，额外 miss ≈ 676K，账单口径命中率 **89.71%**
   （<90% DoD），而事件口径 93.26% 虚高；且两轮运行全部 `summary_incomplete`
   （摘要调用 100% 失败、零产出）。机械模板探针（`scripts/probe_cache_hit_
   template.py`）实测：换系统提示词 + 相同视图 = 冷启动 0% 命中整段重付；
   同输入重发 94.7%。**定案=纯机械压缩**：`run_template_compact` 移除模型
   摘要调用（`agent`/`cancel`/`heartbeat` 参数退役，`CompactDecision::Executed`
   简化），五段槽位全由黑板（目的/计划/路径）+ 固定机械占位（注意事项/后续
   衔接，D1=(b)——阶段 (c) HA 结构化事实聚合落地前由主模型按 marker 回查
   入口自行承接）构成；存档恒写入（审计副本 + digest）、marker 恒携带真实
   digest/路径，无 `summary_incomplete` 终止态；fallback 紧急机械截断保留；
   事件 `context_compressed.mode` = `mechanical`（schema enum 保留
   `template_summary` 供旧 journal 回放）。**收益**：每次压缩省下整段视图
   miss（~150–460K）+ 失败调用纯浪费；事件口径与账单口径对齐（压缩零模型
   调用后账单请求数 = 主循环 `model_output` 数）。**边界**：HA 语义部分
   （结构化事实聚合）为阶段 (c)，待重看 HA 项目并补好助理层后另行裁决；
   本阶段只做固定占位。性质：FUS-COMPACTION-REDESIGN（§14.10）修订——五段
   结构/冷却/守卫/存档/事件面保留，LLM 槽位退役。实现见 orz 子模块提交
   （2026-08-18，B 定案实施）。
   **2026-08-19 全面审查处理登记（B 定案收口）**：三项收口——(a) fallback
   双段截断轮数口径修复：事件/存档/marker「被压轮次」= 常规 drain + 紧急
   截断之和（此前覆盖赋值少报、三方不一致），事件估计改在 marker 插入后
   按实际数组重算；(b) `ledger_fold_write_failed` 契约补齐：payload
   schema / envelope+payload fixtures / verifier 交叉校验 / 生成器
   （conformance 52→53，修复 2b755d6 遗留红）；(c)
   `orz_source_manifest.sha256` 重生成、生成器 `context_compressed` 模板
   对齐 mechanical（防重生成回退）、文档/注释/schema 描述清理（summary.rs
   退化门注释、schema 终止态字段描述、LEDGER 设计 widened tail 文本）。
   验证：orz-loop 466 / verifier+conformance 230 / 仓库门禁 valid。
   **2026-08-19 全面审查处理登记（B 定案收口）**：三项收口——(a) fallback
   双段截断轮数口径修复：事件/存档/marker「被压轮次」= 常规 drain + 紧急
   截断之和（此前覆盖赋值少报、三方不一致），事件估计改在 marker 插入后
   按实际数组重算；(b) `ledger_fold_write_failed` 契约补齐：payload
   schema / envelope+payload fixtures / verifier 交叉校验 / 生成器
   （conformance 52→53，修复 2b755d6 遗留红）；(c)
   `orz_source_manifest.sha256` 重生成、生成器 `context_compressed` 模板
   对齐 mechanical（防重生成回退）、文档/注释/schema 描述清理（summary.rs
   退化门注释、schema 终止态字段描述、LEDGER 设计 widened tail 文本）。
   验证：orz-loop 466 / verifier+conformance 230 / 仓库门禁 valid。

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
   **2026-08-18 二次审查修复（S1 收口；orz 子模块提交见后）**：全面审查
   逐项处理——(a) 推进写失败不再 `continue` 跳过模型请求（持久失败 = 会话
   空转）；改为回滚 + 计数 + 事件留痕，连续 3 次失败后本循环禁用折叠、
   视图退回全量原文（压缩兜底），新增事件面 `ledger_fold_write_failed`
   （payload: ledger_path / attempt / disabled / rows / view_estimate_tokens
   / agent_role；run-event-v0.2 schema 同步）；(b) 外挂文件仅主车道折叠
   （检索车道共享 session_cwd，避免多车道行混入同一文件），检索车道压缩
   marker 不携带台账提示；(c) 行格式 `[<全局序号>]`（per-row 全局序号，
   跨压缩连续）与 `轮次`（窗口内 round_index+1，沿用现有台账语义）解耦；
   (d) `tail_seq` 长行稳健化（尾窗增长至完整末行）+ 非空文件尾行无
   `[seq]` 报错回滚，杜绝静默从 1 重新编号；(e) `advance_fold` 落行前补
   preamble / safe_fold_cut 校验（视图拒绝折叠时行不入文件，与视图一致）；
   (f) marker「历史摘要累积于」行仅在外挂文件已存在或本窗口已折叠时写入
   （避免悬空提示）；(g) `with_context_compact` 不再重置
   `fold_tail_rounds`；`ledger_fold_advance` 增 `view_estimate_after`
   （推进后估算，补足「触发复位」直接断言）。S2 增 6 项测试（action_ledger
   5 + controller 写失败降级 e2e 1），orz-loop 全量 **468 通过**（-j 1）、
   fmt 干净、clippy 与基线一致（lib 21 / test 26）；orz-assurance 152 +
   orz-tui 178 通过、`cargo check --workspace` 通过。

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

### 14.39 v1.39 补写裁决索引（2026-08-24）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **机械审查层＋半助理层重设计定稿（2026-08-24 用户多轮裁决；
   设计轮不动计数）**：问题=同模型 lean harness 对照（Codex harness
   pass@1 70/89=78.65%、Maka 73.03%、官方 DeepSeek V4-Flash-0731
   TB2.1=82.7）vs 本机 10 题 50%、每真实动作 2.3–2.9 模型往返、检索
   10 试次零调用、引用校验 5/8 触发 3/8 硬 block 吞终答、P1 读范围门
   在 allow_shell 下可绕过且拦合法读取。**定案**：①第 1 轮 plan-first
   硬门形态不变（钉死「极简模式」，处理 deepseek 不同思维链的起点收敛）；
   ②第 2 轮起 direct 执行面（模型直接调工具，一次调用一个往返）；
   ③助理层拆机械审查层＋半助理层——机械审查静默记录、每对象键仅保留
   最后一轮结果覆盖写、不给建议、报告随最终答案前中立问询轮注入
   （2026-08-24 用户裁决收敛：报告仅执行事实摘要=动作/文件 delta/
   预算/异常事实，step/契约类只事件留痕、不上报告）；半助理层承接
   命令运行/写执行/检索派发（host 拥有 cwd/env/超时）；
   ④ACAF、权限轴、预算/墙钟、候选计数、run_tests host-owned、
   read-anchor 写前核证维持前置硬门；⑤round 2+ 不要求 step_id 绑定，
   step/契约/receipt 仪式退役（转审计或删除）；⑥输出级引用校验器整体
   删除（含 BASE_SYSTEM_PROMPT「机械校验/阻止交付」表述改写）；
   ⑦读范围放开（保留 `.gsa` 证据面不可见、16KB 信封、head_limit）；
   ⑧主面恢复检索（web_search/web_fetch/browser_read/retrieve_project_*，
   relay 路由仍派发子代理，候选/并发/模式门不变）。登记于
   [设计](../docs/MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md) /
   BACKLOG 0g / TODO P0-0g / CLI_PROJECT_INDEX v2.19。

2. **机械审查层＋半助理层 S1 实施（2026-08-24，合并实施包；计数不动，
   S2-S4 待实施）**：①读范围放开——`access_in_scope` 删除 read/grep 的
   cwd 包含性要求，仅保留 `.gsa` 证据面不可见（白名单仅
   `run_tests_output.txt` 与 `session/terminal/*.log`）、16KB 信封、
   head_limit；②输出级引用校验器整体删除——agent_loop 引用门、
   `citation_validation.rs` 模块、prompt CITATION_VALIDATION_FAILED
   块/前缀、controller validate、EventType::CitationValidation（含 TUI
   投影）与父仓库 schema/fixtures/Python verifier 对应事件全部移除；
   BASE_SYSTEM_PROMPT 引用纪律段改写为轻量纪律（引用需绑定本 run 已观测
   证据、无机械校验、无"阻止交付"表述）；③主面恢复检索——主车道投影
   （round 2+）恢复 web_search/web_fetch/browser_read/retrieve_project_*，
   relay 路由仍派发检索子代理，候选计数/并发=1/模式门/ACAF 前置不变；
   ④direct 执行面——第 2 轮起模型直接调用工作工具（一次调用一个往返），
   console 订单面退役：blackboard_action_write / console_step_done /
   console_return_to_console 不再声明（handler 保留为休眠路径）；
   console 调用面 belt-and-braces 门移除；submit 转为信息展示、非硬门
   （前序步骤未 done 不再拒绝；step 绑定/顺序转事件留痕，终答前反例自查
   轮 + 审计报告承接计划完成声明核对）；⑤机械审查层——
   `mechanical_audit.rs` 运行内审查表（对象键 file:<path>/cmd:<call_id>/
   plan/budget/retrieval:<n>，每键仅最后一轮结果覆盖写、容量上限 128 超限
   丢最旧、不给建议）；轻量 `mechanical_audit_update` 事件留痕（kind ∈
   {tool_result, plan_gate, budget}，主车道专属）；报告收敛为执行事实摘要
   （动作/文件 delta/预算/异常事实），随最终答案前 [COUNTEREXAMPLE_GATE]
   同轮以 [MECHANICAL_AUDIT v0.1] 独立块注入（注册进 injected-block
   filter，不进归档）；step/契约类只事件留痕、不上报告；⑥半助理层——
   run_terminal_cmd/search_replace/run_tests/检索派发仍经 host 侧机械链
   前置执行（host 拥有 cwd/env/超时），ACAF/权限轴/预算/墙钟/候选计数/
   read-anchor 写前核证维持前置硬门。实施验证：orz-loop 563 通过 /
   0 失败 / 3 ignored、fmt 干净、clippy 无新增告警；orz-host/tui 失败集
   与 HEAD 基线一致（23/5 项为 AGENT-DELIVERY-FLOW 末步校验引入的既有
   脚本化用例失配，非本 S1 引入）。登记于 [设计](../docs/MECHANICAL_
   AUDIT_LAYER_DESIGN_2026-08-24.md) §4/§5 / BACKLOG 0g / TODO P0-0g。

3. **机械审查层＋半助理层 S2 测试（2026-08-24；设计 §4 路由；计数不动，
   S3-S4 待实施）**：①删/改既有 gate 测试——orz-host/tui 脚本化计划
   统一补固定末步 deliver（AGENT-DELIVERY-FLOW 末步校验引入的既有失配
   23+5 项全部适配，`acp_server::AcpServer::new` 与 13 处
   `plan_write_response` 经 `ensure_terminal_step` 归一、codex_app 助手
   valid/two_step plan 补 deliver、orz-tui 两处内联计划补 deliver）；
   acp_server 事件计数断言 16→18（S1 引入 plan_gate/budget 两条
   `mechanical_audit_update` 留痕）；②新增 6 项专项测试——审计报告随终答前
   反例自查轮同轮注入（[MECHANICAL_AUDIT v0.1] 独立块、执行事实/预算/
   异常事实三类收敛、无建议、journal 留痕 kind ∈ {plan_gate, budget,
   tool_result}）、direct 面 journal 零退役工具调用（blackboard_action_
   write/console_step_done/console_return_to_console 零残留）、终答携带
   未绑定 `[来源: SRC-999]` 引用原样交付（行为侧零残留）、主面检索族声明
   （web_search/web_fetch/browser_read/retrieve_project_*）、主车道
   web_search 派发检索子代理（ToolStarted target=external_retrieval，
   构造题检索可达）、检索候选计数/超限分类；③验证——orz-loop 569 通过 /
   0 失败 / 3 ignored、orz-tui 178 / 0、orz-host 串行 221 / 0（并行仅
   `call_tool_timeout_kills_process_tree` 预存时序 flake，HEAD 基线同样
   失败、单跑通过）、orz-assurance 144 / 0、orz-bin 11 / 0、fmt 干净、
   clippy 无新增告警。登记于 [设计](../docs/MECHANICAL_AUDIT_LAYER_DESIGN_
   2026-08-24.md) §4/§5 / BACKLOG 0g / TODO P0-0g。

4. **机械审查层＋半助理层全面审查处理（2026-08-24；设计 §2.3/§2.5/
   §2.8 符合性收口；计数不动，S3-S4 待实施）**：①**P1-1 read-anchor
   写前核证补 direct 面落点**——审查发现 host 级 `search_replace` 无
   `expected_anchor` 参数/校验（核证仅存于休眠订单链），direct 面该硬门
   实际缺失；修复=`verify_content_anchor` 解耦为 (file_path, label) 签名，
   `run_host_tool_with_timeout` 执行前加核证门（NotFound=新建跳过、其余
   I/O 错误 fail-closed、不匹配返回结构化 `content_anchor_mismatch` 拒绝、
   无 ToolStarted），`search_replace` 工具声明补可选 `expected_anchor`
   参数；审计层锚点拒单异常事实真实化。②**P1-2 退役工具调用面窄门**——
   审查发现 `blackboard_action_write`/`console_step_done`/
   `console_return_to_console` 的休眠 handler 在 round 2+ 仍可达（调用面
   belt-and-braces 已删、轮末发放入口存活），幻觉调用可真实执行订单；
   修复=三个退役名在 `run_host_tool_with_timeout` 调用面机械拒绝
   （`retired_tool_denied`、无 ToolStarted、零 console_order_written/
   rejected 事件、零副作用、计入连败熔断），handler 与轮末发放保留为
   真正休眠路径（与第 2 项"handler 保留为休眠路径"字面一致）。③**P2-1**
   `cmd:` 审计摘要格式修复（原实现括号不配对、以文件 delta 顶替 stdout
   截断；改为 `[truncated:` 机械标记单列"输出截断"）。④**P2-2** 候选拒绝
   结构化错误码透传（`candidate_cap_exceeded`/`candidate_count_unbound`
   精确异常分类，替代剩余池近似）。⑤**测试迁移**——21 项已退役订单链
   e2e 删除（console_s2/s3/s4 流、预算预检、slot busy、step_done、双模式
   问询、结果栏 receipt、状态行/步骤失败订单驱动版本）；acp_server/
   codex_app/orz-tui 8 处订单脚本迁移为 direct 工具调用；订单层
   FUS-READ-ANCHOR 四场景保留为休眠路径验证（upstream `order_id` →
   `label`）；新增 direct 锚点门四场景与退役工具窄门测试。验证：orz-loop
   551 通过 / 0 失败 / 3 ignored、orz-tui 178 / 0、orz-host 串行 220 / 0
   （并行仅预存 flake `call_tool_timeout_kills_process_tree`，单跑通过）、
   fmt 干净。登记于 [设计](../docs/MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md)
   §2.3/§2.5/§2.8 / BACKLOG 0g / TODO P0-0g。

5. **机械审查层＋半助理层 S3 重建（2026-08-24；设计 §4 路由；计数不动，
   S4 待实施）**：Linux musl 重建（ORZ-BUILD-MOUNT-001 契约，
   build_orz_aliyun.sh，rust:1.97-slim 增量构建 -j 1，父仓库挂载
   /orz、工作目录 /orz/orz，输出三件套至 D:/tb-eval/orz-linux）
   BUILD_EXIT=0；三件套时间戳 2026-08-24 06:15 HKT（orz 104,718,240 B /
   orz-signer 1,388,592 B / orz-acaf-provision 1,206,576 B），编译
   6m47s；最小可执行冒烟=三件正常加载执行（orz 无 TTY io error 属预期、
   provision usage、signer manifest 缺失）；守卫符号
   retired_tool_denied / content_anchor_mismatch 二进制各 8 命中；
   ELF 无 ld-linux / GLIBC_2 动态解释器字符串（musl 静态确认）；对应
   源码 orz 033fd26 + 父 54560b4（S1/S2+审查处理提交，均已推送）。
   **二次构建轮 s3b（2026-08-24 08:19 HKT，reasoning_content 修复
   orz 4ca60c2 + 父 c1ce4c1，均已推送）**：三件套 orz 104,726,432 B /
   orz-signer 1,388,592 B / orz-acaf-provision 1,206,576 B，编译
   5m04s，冒烟同前（三件正常加载执行、守卫符号各 8 命中、musl 静态）。
   登记于 [设计](../docs/MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md)
   §4 / BACKLOG 0g / TODO P0-0g。

6. **机械审查层＋半助理层 S4 复验闭环（2026-08-25 补登记；计数 31 → 30，
   `implemented`）**：S4 实机=2026-08-24 sweep-mal-s4（5 题 k=1：
   git-multibranch / break-filter-js-from-html / code-from-image /
   mteb-retrieve / sam-cell-seg，deepseek-v4-flash）5/5 reward 1.00、
   零异常、48m06s；5/5 journal 事件链完整性 100%、零 fail 事件、
   [MECHANICAL_AUDIT] 报告注入齐全（22–106 处引用）；审计报告覆盖写
   由 S2 专项测试预演覆盖；检索可达项在该轮暴露缺口（检索族零可见，
   即 0h 主题根因）——由 RETRIEVAL-SUBAGENT-WIRING（0h）S4 单题实测
   补足（2026-08-25 mteb-leaderboard：web_search×19 / web_fetch×10、
   降级 transition 落盘、reward 1.00）。计数 31 → 30（0g S1 放行入账
   29 → 30 补记于 TODO 快照）。登记于
   [设计](../docs/MECHANICAL_AUDIT_LAYER_DESIGN_2026-08-24.md) §4 /
   BACKLOG 0g / TODO P0-0g。

### 14.40 v1.40 补写裁决索引（2026-08-25）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **检索子代理接线重设计定稿（2026-08-25 用户裁决；设计轮不动计数）**：
   问题=MECHANICAL-AUDIT-LAYER S4 复验（2026-08-24，5 题 reward 全 1.00）
   主模型工具面 13 工具、检索族零可见——根因①harness 不传
   `--retrieval-mode`、会话停默认 off（mode=off 投影剔除检索族 + 调用面
   拒绝 `retrieval_mode_off`）；根因②内部子代理触发工具
   `retrieve_project_docs` / `retrieve_project_source_ledger` 从未注册
   （仅 relay 路由与测试引用，投影「未声明不发明」→ 内部子代理触发面断）。
   **定案**：①外部子代理=模式 A 自动定档——local_browser probe 失败
   （browser_launch_failed）机械降级 framework_fallback 并记
   `retrieval_mode_transition`（old/new 实值、authority=mechanical_probe、
   reason=browser_launch_failed），取代 §3.7.1「禁止因浏览器不可用自动
   切换」；页面级失败（LOGIN_REQUIRED/CAPTCHA/PAGE_BLOCKED 等 §3.7.2
   显式状态）不降级；工具面定档=浏览器可用只有 browser_read、不可用只有
   web 族；TB harness（`tb_agents/orz.py`）PUBLIC 时传
   `--retrieval-mode local_browser`（容器无浏览器→自动降级 web 族）。
   ②内部子代理重新定位=结构化检索外包（主代理点读保留；多文件/跨目录
   调研打包派发，`[DOC]` 结构化结果+ledger 回传，隔离上下文、降低主对话
   污染，§3.7 条 8 语义）；controller 声明面注册 `retrieve_project_docs`
   ToolDef（relay 路由已存在，声明即触发），内部 lane 工具面仅读族
   （read_file/list_dir/grep/search_tool/project_doc_index），web 族与
   browser_read 不进入。③prompt 以框架使用提示（tips，≤1 句）告知使用
   方式（多文件调研用检索外包、点读用 read_file/grep），模型自主决定、
   不做硬门不加仪式。候选计数/并发=1/ACAF 前置/子代理状态机/写域
   deny-only 不变。登记于
   [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md) /
   BACKLOG 0h / TODO P0-0h / CLI_PROJECT_INDEX v2.21。

2. **检索子代理接线 S1 实施（2026-08-25；计数不动，S2-S4 待实施）**：
   ①A 档降级——`apply_mode_a_auto_degrade`（acp_server）：local_browser
   probe 失败（browser_launch_failed）→ 快照降级 framework_fallback +
   bootstrap_transition_pending；controller `with_retrieval_mode` 增可选
   transition 元数据（authority/reason_code，降级时
   mechanical_probe/browser_launch_failed，缺省保持 session_bootstrap/
   session_default）；降级后 capability 以 framework_fallback 重探
   （transition capability_status 不残留浏览器失败原因）。②主面声明
   `retrieve_project_docs` ToolDef（controller 声明面，参数 query 必填 +
   可选 scope/max_results；relay 路由已存在；mode=off 被检索族投影剔除）。
   ③`subagent_tool_projection` 加 role 参数——内部 lane 剔除 web 族
   （web_search/web_fetch 及变体）/browser_read/retrieve_project_*（仅
   读族），外部 lane 维持 web 族 + browser_read（registry 声明时）。
   ④prompt 执行面段补 ≤1 句内部检索 tips（多文件/跨目录调研用
   retrieve_project_docs、点读用 read_file/grep，无硬门无仪式）。
   ⑤TB harness（tb_agents/orz.py）PUBLIC 时追加 `--retrieval-mode
   local_browser`（容器无浏览器 → probe 失败自动降级 web 族）。
   验证：orz-loop 556 / 0 / 3、orz-host 222 / 0 / 4（串行）、orz-tui
   178 / 0、orz-assurance 144 / 0、orz-bin 11 / 0 + acaf_e2e 23 +
   signer 14 + provision 2 + stdio_e2e 1 + real_flag 2、fmt 干净、
   clippy 无新增告警。登记于 [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_
   DESIGN_2026-08-25.md) §4 / BACKLOG 0h / TODO P0-0h。

3. **检索子代理接线 S2 测试（2026-08-25；设计 §4 路由；计数不动，
   S3-S4 待实施）**：新增 5 项专项测试——①模式 A 降级规则
   （browser_launch_failed 前缀降级 framework_fallback + transition
   pending；浏览器可用不降级；非启动原因 Degraded 不降级；
   framework_fallback 不降级）；②降级 transition 元数据透传（old=
   local_browser/new=framework_fallback/authority=mechanical_probe/
   reason_code=browser_launch_failed/capability_status 正确）；③主面
   `retrieve_project_docs` 声明（mode≠off 可见、mode=off 隐藏）；④主车道
   `retrieve_project_docs` 派发内部子代理（ToolStarted target=
   internal_retrieval、内部 lane 执行读族工具、事件面零 web/browser
   调用）；⑤内部 lane 投影仅读族（web/browser/retrieve 全剔除、外部 lane
   维持 web 族 + browser_read）。验证：orz-loop 556 通过 / 0 失败 /
   3 ignored、orz-host 222 / 0、orz-tui 178 / 0、orz-assurance 144 / 0、
   orz-bin 11 / 0（+ 集成目标全过）、fmt 干净、clippy 无新增。登记于
   [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md) §4 /
   BACKLOG 0h / TODO P0-0h。

4. **检索子代理接线 S1/S2 全面审查处理（2026-08-25；S3 重建待实施）**：
   审查发现并处理 5 项——①P0：CLI 一次性运行（`orz -p ...`）此前不消费
   `ORZ_RETRIEVAL_MODE`（仅 stdio 入口读取），harness 传
   `--retrieval-mode local_browser` 对 TB 会话无效、恒停 off——
   `run()` 现经共享入口 `retrieval_mode::probe_retrieval_with_mode_a`
   （ACP/CLI 共用，`apply_mode_a_auto_degrade` 并入共享决策）接线 probe
   + 模式 A 降级 + transition 落盘；②P1：`scope`/`max_results` 此前
   声明但派发只读 `query`、参数被静默丢弃——`build_retrieval_task_goal`
   机械并入子代理任务契约；③P1：工具面跟随模式 A 定档（DoD 第 2 条
   收敛）——`apply_retrieval_surface_projection`：local_browser 隐藏
   web 族、framework_fallback 隐藏 browser_read，外部 lane 的
   browser_read 恢复按模式门控；④收紧 `browser_launch_failed` 前缀判定
   （带冒号分隔符，拒绝 `browser_launch_failedX` 宽松前缀）；⑤P3：降级
   transition 元数据跨 run 持久化（`StoredActivationSnapshot.
   pending_transition_authority`，journal 成功后清除，防失败重试退化为
   session_bootstrap）。验证：orz-loop 558 / 0 / 3、orz-host 223 / 0 /
   4（串行）、orz-tui 178 / 0、orz-assurance 144 / 0、orz-bin 11 / 0 +
   acaf_e2e 23 + real_flag 2 + stdio_e2e 1、fmt 干净、clippy 无新增。
  新增专项测试 4 项：模式 A 降级决策与快照应用（含前缀收紧）、CLI 接线
  决策（off 不 journal / 显式 session_bootstrap / 降级
  mechanical_probe）、检索工具面跟随模式、检索任务契约并入
  scope/max_results（含子代理 system goal 断言）。登记于
  [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md) §4 /
  BACKLOG 0h / TODO P0-0h。

5. **检索子代理接线 S3 重建（2026-08-25；S4 待实施）**：Linux musl
   三件套（ORZ-BUILD-MOUNT-001 契约，build_orz_aliyun.sh，
   rust:1.97-slim 增量构建 -j 1）BUILD_EXIT=0；三件套时间戳
   2026-08-25 02:46 HKT（orz 104,796,704 B / orz-signer 1,388,496 B
   / orz-acaf-provision 1,206,480 B），编译 6m10s；最小可执行冒烟=
   三件正常加载执行（orz 无 TTY io error 属预期、provision usage、
   signer manifest 缺失）；守卫符号 retired_tool_denied /
   content_anchor_mismatch 各 8 命中；新检索接线符号（retrieval-mode
   / browser_launch_failed / retrieve_project_docs）在二进制内；无
   glibc 动态解释器（musl 静态确认）；对应源码 orz f4f1b81 + 父
   48cb030（均已推送）。登记于
   [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md) §4 /
   BACKLOG 0h / TODO P0-0h。

6. **检索子代理接线 S4 复验闭环（2026-08-25；计数 30 → 29，
   `implemented`）**：单道检索题实机（terminal-bench 2.1
   mteb-leaderboard，k=1，deepseek-v4-flash，sweep-0h-s4-lb，
   33m23s）reward 1.00、零异常——模型经 GitHub API 锁定 2025-08-29
   结果仓库快照（71f6b62）后用 mteb 1.38.41 计算 Scandinavian 全任务
   Mean (Task) 并按全任务过滤，终答 GritLM/GritLM-7B（17 字节，
   read_file 锚点核证）；检索调用出现（主面 web_search×19 /
   web_fetch×10，外部子代理检索结果侧车 2 份落盘）；降级 transition
   落盘（old=local_browser → new=framework_fallback，
   authority=mechanical_probe、reason_code=browser_launch_failed、
   capability_status=available）；零真实 400 / 零 tool_failed / 零
   run_invalidated；journal 事件链完整性 100%（619/619，命中率 ≥90%
   达标）；DoD 1-7 全部满足。计数 30 → 29（0h S1 放行入账 30 → 31
   补记于 TODO 快照）。登记于
   [设计](../docs/RETRIEVAL_SUBAGENT_WIRING_DESIGN_2026-08-25.md) §4 /
   BACKLOG 0h / TODO P0-0h。

7. **初版发布前修复——transport reasoning_content 规范化（2026-08-25；
   计数不动）**：初版打包（orz 0.1.0 linux-x86_64）端到端实测发现
   v4-flash 纯文本完成响应偶尔完全不带 reasoning 增量，流式组装返回
   `None`，回推助手消息时字段被省略，下一请求触发 DeepSeek
   `invalid_request_error`（"reasoning_content in the thinking mode must
   be passed back"；最小题实测 3/6 失败）。修复：流式与非流式解析统一将
   无 reasoning 的完成响应规范化为 `Some("")`（D-6 既有契约：空串正常、
   省略 400），新增流式文本轮回归测试
   `generate_stream_text_round_preserves_empty_reasoning`；orz-loop
   559 / 0 / 3、fmt 干净、clippy 无新增；修复后最小题 6/6 通过。
   对应源码 orz 8bcf18c（已推送 cli/feat/fusion-architecture）。登记于
   [初版发布包](../releases/orz-0.1.0-linux-x86_64/README.md)。

8. **初版发布 0.1.0（2026-08-25；计数不动）**：双平台发布——Windows
   x86_64 release 构建（orz.exe 48,580,608 B / orz-signer.exe
   6,742,016 B / orz-acaf-provision.exe 6,642,176 B，编译 24m54s，
   守卫符号 retired_tool_denied / content_anchor_mismatch 各 8 命中、
   检索接线符号在二进制内）+ Linux musl 三件套（第 7 项）；Windows
   端到端冒烟通过（凭据管理器 `orz-deepseek/agent` 注入 + DPAPI ACAF
   初始化 + 真实最小跑 EXIT=0）；GitHub Release v0.1.0
   （SilverWhite/CLI，[releases/tag/v0.1.0](https://github.com/SilverWhite/CLI/releases/tag/v0.1.0)），
   资产=orz-0.1.0-linux-x86_64.tar.gz（33.5 MB）+ orz-0.1.0-windows-
   x86_64.zip（25.6 MB）；API Key 不入包，注入=Windows 凭据管理器 /
   Linux `ORZ_DEEPSEEK_API_KEY` env（ADR-0006 §2.2/§2.3）。

9. **0.2.0 发布（2026-08-31；计数不动）**：双平台发布——版本号
   orz-bin 0.1.0 → 0.2.0（orz a539a21b，已推送
   cli/feat/fusion-architecture）；Windows x86_64 release 构建
   （orz.exe 50,398,720 B / orz-signer.exe 6,737,920 B /
   orz-acaf-provision.exe 6,642,176 B，续编 9m51s）+ Linux musl
   三件套（orz 106,905,448 B / orz-signer 1,390,072 B /
   orz-acaf-provision 1,207,928 B，编译 10m53s，musl 静态；bookworm
   容器冒烟通过——marker 计数 / 三件套启动行为 / `orz --real` TTY
   错误路径）；两平台二进制均内嵌版本串 0.2.0；0.2.0 主要变更=机械层
   数学计算体 I1–I6 + V1–V3 验证闭环、R2 deny 通道、R5 grep→read
   管线、0k 检索编排机械层（同轮并行 / 子代理预算 / [DOC] 截断 /
   主面 web_search 恢复 / 方向 C 收口）、0k 第二批（project_doc_index
   v2 / tab 池 / 委托分档）、序列内容门、controller 拆分（行为不变）；
   GitHub Release v0.2.0（SilverWhite/CLI，
   [releases/tag/v0.2.0](https://github.com/SilverWhite/CLI/releases/tag/v0.2.0)），
   资产=orz-0.2.0-linux-x86_64.tar.gz（32.6 MB）+ orz-0.2.0-windows-
   x86_64.zip（24.4 MB）；API Key 不入包，注入=Windows 凭据管理器 /
   Linux `ORZ_DEEPSEEK_API_KEY` env（ADR-0006 §2.2/§2.3）。登记于
   [0.2.0 发布包](../releases/orz-0.2.0-linux-x86_64/README.md)。

### 14.41 v1.41 补写裁决索引（2026-08-25）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **复读检测序列内容门设计定稿（2026-08-25 用户裁决；设计轮不动
   计数）**：问题=final-smoke-2026-08-25（5 题 k=1 官方标准）dna-assembly
   试次中 EGFP 400 字符 DNA 序列合法重复引用（3 次、偏移 18/801）触发
   路径①滚动哈希 3/3 流内命中（无切分点直接判真）、降级 high→low→
   disabled——合法任务内容误杀，缺口 A 延续（GAP-REPETITION-DETECTOR-
   DNA-FALSE-POSITIVE）。**定案**：`REPETITION_MIN_RUN_CHARS`（L）=400
   维持，不采用提档到 1000（移动边界、削弱真退化灵敏度）；路径①保留给
   所有内容；二级确认「无可切分点→直接判真」分支加内容判别门——span
   字符 ∈ `{A,C,G,T,N,U}`（大小写）占比 ≥ 0.90
   （`REPETITION_SEQUENCE_LIKE_RATIO` 初值）判为序列样，序列样 span 的
   流内命中门槛 3 → **5**（`REPETITION_SEQUENCE_HIT_LIMIT` 初值；1–4 次
   仅审计、≥5 才 trip）；非序列样维持 3；有切分点路径（标点块 0.50
   确认）不变；content/reasoning 两族统一；间隔不重置、流结束丢弃、
   consecutive / `DEGENERATION_LIMIT` 语义不变；路径②（3-gram 比例
   0.70）与 stall 兜底（600s/64K）不变。否决方向登记：整体序列内容走
   路径② + stall（检测盲区=同 span 变上下文循环路径②无感、stall 只抓
   无输出、散文+序列混排需内容路由）。实施路由 S1 代码 → S2 测试 →
   S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 复验（dna-assembly
   重跑零误杀降级、真循环仍触发、零 400、命中率 ≥90%）。登记于
   [设计](../docs/REPETITION_DETECTOR_SEQUENCE_CONTENT_GATE_DESIGN_2026-08-25.md)
   / BACKLOG 0i / TODO P0-0i / CLI_PROJECT_INDEX。
   **实施放行（2026-08-25，计数 31 → 32）**：S1 代码完成（transport.rs
   新常量 `REPETITION_SEQUENCE_LIKE_RATIO`=0.90 /
   `REPETITION_SEQUENCE_HIT_LIMIT`=5、`sequence_like` 整数比较判定、无
   切分点分支三态确认分派（Confirmed/SequenceGated/Rejected）、序列族
   流内命中分别计数与双门槛 feed 封顶、`sequence_gated` 审计标注
   （ratio+hits/limit）与 WARN `sequence_hit_limit` 字段；既有 'a'/'t'
   重复字符测试按序列门语义更新为 'x'）；orz-loop 563/0/3 全绿、fmt
   干净、clippy 无新增；核心测试随 S1 自证（序列 3 次仅审计、5/6 次
   触发、混合分别计数、占比边界 0.89/0.90/0.91、非序列 3 门槛回归）。
   **S2 测试完成（2026-08-26，计数仍 32）**：final-smoke dna EGFP 400
   字符真实 span 原样回放——4 段引用=3 次命中 0 trip（审计带
   `sequence_gated`/ratio/hits-limit、按命中时刻累计标注 1/5→3/5）、
   6 段=5 次命中触发；poly-A 399/400/401 静默；序列族间隔不重置 + 流
   结束丢弃（新建流重新计数、同流第 5 次仍触发）；路径②（3-gram
   0.70）/stall（600s/64K）/标点块（sam G4 真实 span）/混合分别计数
   回归全绿；orz-loop 567/0/3、fmt 干净、clippy 无新增。S3 重建
   （Linux musl，ORZ-BUILD-MOUNT-001）待实施。

2. **序列内容门蛋白/氨基酸序列覆盖扩展（2026-08-26 全面审查处理；
   计数维持 32）**：审查发现蛋白/氨基酸序列不在 DNA/RNA 字母表
   {A,C,G,T,N,U} 内，合法蛋白序列 3 次回显仍会误杀。**定案**：新增
   蛋白族判定——严格 20 标准氨基酸字母 {A,C,D,E,F,G,H,I,K,L,M,N,P,Q,
   R,S,T,V,W,Y}（大小写），占比 ≥ **0.95**
   （`REPETITION_PROTEIN_LIKE_RATIO`，初值，S4 校准 0.93–0.97）判为
   蛋白样；DNA/RNA 族维持 0.90 不变；两族任一判真即序列样（门槛 5），
   同判真时 DNA/RNA 优先；审计与触发 detail 带 `kind=`（dna_rna /
   protein）。**否决方向**：蛋白共用 0.90（覆盖面实证=英文无间隔长串
   蛋白 20 字母占比 0.8975，裕量仅 ~0.25%，过广）；蛋白字母表纳入
   B/X/Z/U（纳入即引入英文常见字母 B/O/U，覆盖过广）。**实现**：
   `sequence_kind` 单次扫描双族判定（整数比较）、阈值百分比提为编译期
   常量、同 delta 多次命中审计按各自累计时刻标注、设计文档 §5 数字非
   切分符勘误；orz-loop 569/0/3、fmt 干净、clippy 无新增。登记于设计
   文档 / BACKLOG 0i / TODO P0-0i / CLI_PROJECT_INDEX。

3. **序列内容门 S3 重建 + S4 复验闭环（2026-08-26；未闭合 30 → 29，
   S1-S4 全部闭合）**：S3 Linux musl 重建完成（ORZ-BUILD-MOUNT-001
   契约，rust:1.97-slim 容器增量构建 6m19s，三件套 orz 104,796,752 B /
   orz-signer 1,388,496 B / orz-acaf-provision 1,206,480 B；守卫符号
   retired_tool_denied / content_anchor_mismatch 各 8 命中、序列门审计
   字段（sequence_gated/dna_rna/protein）与检索接线符号在二进制内、无
   PT_INTERP（musl 静态）；容器冒烟三件正常；对应源码 orz 6b208fb 已推
   cli 远端）。**S4 复验（2026-08-26 闭环）**：dna-assembly 新二进制 k=1
   官方标准重跑（s4-dna-2026-08-26 / dna-assembly__3EyUPDL，1800s 墙钟
   超时 reward 0.0，与旧 run 结局类别一致）——① EGFP 式合法引用零误杀
   降级：1–4/5 次命中全部仅审计（sequence_gated kind=dna_rna，旧 3/3
   即直降 Disabled）；② 真复读仍触发：同一 DNA span 5/5 次命中 trip，
   只降 EnabledLow（非旧直降 Disabled），降级后继续工作；③ 零真实 400
   （事件链 8 处“400”均为哈希串）；④ 事件链严格校验除「缺终止事件」1
   项豁免（墙钟超时 harness 杀进程边界，require_terminal=false 回放
   语义；与旧 run 同构）外 0 错误；⑤ journal 口径命中率 82.36%（22
   请求，762,496/925,805）vs 旧 82.33% 持平——web 检索注入相关，观察项
   不阻塞；请求 36→22、reasoning 46,626→136,956、request_header_change
   6→3、无 reasoning_stall 触发。计数：S3/S4 验证闭环，未闭合 30 → 29。
   登记于设计文档 / BACKLOG 0i / TODO P0-0i / CLI_PROJECT_INDEX
   （GAP-REPETITION-DETECTOR-DNA-FALSE-POSITIVE `partial` →
   `implemented`）。

### 14.42 v1.42 补写裁决索引（2026-08-29）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **THIN-HARNESS-REDESIGN-V2 S5-1 实施 + 全面审查处理（2026-08-29；
   取代 §14.32 第 1/2 项「桥视图剔除 reasoning_content（每条消息）」
   的剥除语义、§14.16 checkpoint 轮无工具语义在 orientation 软门触发
   轮的适用边界）**：S4 复验（31 题全量 k=1，2026-08-29）暴露两问题
   并定案（设计 §9.2/§9.4/§9.6，用户裁决）：
   - **A 折叠桥 reasoning 保留**：DeepSeek /responses 对非空 reasoning
     的 assistant 消息要求原样回传；旧 `build_bridge` 对桥内每条消息置
     `reasoning_content = None`，orientation 纯文本回答被 fold 落入桥内
     末条剥除 → 400（make-doom 103 步 / gcode-to-text 52 步）。**修订**：
     桥内**纯文本** assistant 消息（无 tool_calls）保留
     `reasoning_content`，声明消息（带 tool_calls）仍剥除（实机边界：
     纯工具轮 fold 不触发 400——声明 reasoning 恒空依赖后端行为，保留
     边界注释；预算口径已含 reasoning，`estimate_message_tokens`）。
   - **B orientation 软门触发轮放行工具**：§14.16 checkpoint 轮无工具
     语义仅适用于 DC 强制模板轮 / console 询问轮；orientation 软门触发
     轮（设计 §9.2）按常规轮投影探针/工具栏/console 注册并允许工具
     派发，pending 在工具轮亦提交消费（无重复注入）。
   - **C web_search 客户端超时**：reqwest builder 补总超时 120s +
     connect 10s；超时经 `map_transport_error` 映射为
     `ToolErrorKind::Timeout` 结构化错误（含已用时长、阶段上下文、连接
     超时单独措辞、建议重试/换查询/直读页面），不新增自动重试；orz-host
     `map_tool_error` 保留 Timeout 类别（不再降级 ExecutionFailed）。
   - **D 事件面**：`tool_completed` schema 增 `wall_ms`（integer ≥ 0）
     与 `timed_out`（const true）；通用工具路径与 run_tests 完成均落
     wall_ms，超时落 timed_out；审查处理补 run_tests F-09 墙钟掐杀的
     结构化透传（`TestRunResult.timed_out` 新字段端到端：宿主超时路径
     置位 → 控制器事件标记 + 模型明确 TIMED OUT 文案，取代 exit_code
     缺失的 "timed out?" 启发式）。
   实施：orz ad5f9ee（S1 代码/S2 测试）+ 全面审查处理批（S1 修正 +
   回归测试补强：桥双形态、触发轮工具面、超时事件字段断言、web_search
   超时映射单测）；S3 重建完成（2026-08-29，Linux musl 三件套，orz
   105,026,288 B / orz-signer 1,388,744 B / orz-acaf-provision
   1,206,720 B）。S4 复验批次作废重排（2026-08-29 用户指示：不再沿用
   原 official-r2-failures-s5-1 四题批次，全部处理完成后重新安排补跑）。
   登记于设计 §9.2/§9.4/§9.6 / 调研笔记 §5 / TODO W4-R4；R3 纪律
   （CLI_PROJECT_INDEX 登记 + 计数）随 S4 复验收口执行。

### 14.43 v1.43 补写裁决索引（2026-08-30）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **检索双模式定案（2026-08-30 用户确认最终评判；修订 §3.7.1/§3.7.10
   检索来源语义）**：
   - **双模式保留**：`local_browser`（引擎 SERP，Google 主序、Bing 回退）
     与 `framework_fallback`（DeepSeek 原生 web_search）双保留，原生
     web_search 作为机械兜底。引擎 SERP 通道非「第二 API 供应商」（无
     key/计费，不违反 §3.7 条 10 禁止引入独立检索 API 供应商），但检索
     来源从 DeepSeek 服务端生成式搜索变更为引擎 SERP 有机结果，§3.7.10
     来源语义按此登记修订。
   - **SERP 页面级失败直接原生兜底**：SERP 页面级失败（CAPTCHA /
     PAGE_BLOCKED / 429 / consent 等 §3.7.2 显式失败态）直接触发原生
     web_search 兜底，显式记录——走 `retrieval_mode_transition`
     （authority=mechanical_probe、reason 显式），沿用模式 A 语义、
     不静默混用；candidate 页面读取（browser_read）仍按 §3.7.2 显式失败
     不降级。
   - **人化输入延迟（机械层）**：引擎 SERP 查询注入键入模拟（50–150ms
     抖动）+ 提交前停顿（300–800ms）+ 查询间冷却（默认 2–5s，
     `ORZ_ENGINE_QUERY_PACING_MS` 可配）+ 会话级频率上限（建议 20–30，
     超限显式失败 → 原生兜底）；模型不可见、不进 prompt、不计模型轮次；
     pacing 不影响原生 web_search 兜底路径。
   - **Google 门禁观察实验先行**：小批检索密集题 k=1、个人使用强度，
     观测 consent cookie 处理 / CAPTCHA 频率 / IP 节流 / 页面结构稳定性，
     结果决定 Google/Bing 主序并校准 pacing 默认值。
2. **检索编排机械层第一批（2026-08-30 定案；S1 实施 + S2 测试完成
   2026-08-30，S3 重建/实机验证待续）**：
   ①同轮读类/检索类工具并行（FuturesUnordered，写类保持串行，web_search
   信号量维持 1）；②子代理 run 级预算/超时（轮数+墙钟双层，落点
   `retrieval/dispatch.rs`）；③[DOC] 回传机械截断（结构化头部 + evidence
   指针）；④浏览器容器参数补齐（`--no-sandbox` / `--disable-dev-shm-usage`
   / `--disable-gpu`）+ 资源拦截 + 等待语义（「可用文本就绪」替代
   loadEventFired，full 模式保持既有终态）。web_search 全局并发维持 1；
   `retrieve_project_docs` 维持封存（`R1_SEALED_MAIN_TOOLS` 不动）。
   第二批（独立设计轮）：`project_doc_index` v2（Blake3 增量 + 驻留）、
   会话级 tab 池 + 同轮多页并行读取、委托契约复杂度分档。
3. **登记**：BACKLOG 0k / TODO P0-0k / CLI_PROJECT_INDEX
   （FUS-RETRIEVAL-ENGINE-SERP）；实施放行入账按既有纪律（+1），
   S4 闭环 -1。

### 14.44 v1.44 补写裁决索引（2026-08-30）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **主面恢复外部 web_search 派发入口（2026-08-30 用户裁决「先恢复
   外部」；修订 §3.7 条 12「二存一」）**：
   - 背景：门禁观察四轮 10 个 gate run 检索调用全部为 0——结构性缺检索
     入口（主面 web 族在 local_browser 下隐藏 + browser_read 方向 A
     封存），从未真正测到「模型看到 web_search 时会不会用」；Google
     SERP 门禁无样本是入口缺失的必然结果，非「模型倾向不用检索」。
   - 恢复语义（最小模型面变化）：主面 local_browser 保留 `web_search`
     单一入口，工具描述标注「External retrieval entry: dispatches the
     external retrieval subagent」（纯机械侧标注）；`web_fetch` 族与
     `web_search_*` 变体不进入主面。模型调用 web_search → `relay::route`
     → ExternalRetrieval 子代理派发。
   - 执行面二存一不变：外部 lane 在 local_browser 下剔除继承的 web 族
     （`subagent_tool_projection`），仅 `browser_read`（引擎 SERP）为
     检索通道；原生 web_search 兜底是机械路径
     （`retrieval_mode_transition`，authority=mechanical_probe、reason
     显式），不是子代理模型的自由选择。framework_fallback 下外部 lane
     维持 web 族（既有语义）。
   - 内部检索（`retrieve_project_docs`）维持 R1 封存不动（`R1_SEALED_
     MAIN_TOOLS` 不含恢复项）。
   - 目标：一次验证「模型在有入口时用不用检索」与「Google SERP 门禁
     （CAPTCHA / 429 / 页面结构）」两个问题。
2. **登记**：BACKLOG 0k / TODO P0-0k / 门禁观察报告
   `docs/GATE_GOOGLE_OBSERVATION_2026-08-30.md` §8–§10 /
   CLI_PROJECT_INDEX（FUS-RETRIEVAL-ENGINE-SERP）。

### 14.45 v1.45 补写裁决索引（2026-08-30）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **检索结果形成契约修复（GAP-RETRIEVAL-STRUCTURED-RESULT，方向 C，
   2026-08-30 用户裁决「按 P1 方向 C 进行」；修订 §3.3.3 条 3 / §3.7 条 12
   第三层）**：
   - **背景**：`[RESULT_JSON]` 组织块的 `source_ids` 契约结构性不可满足——
     机械 ledger 的 `SRC-001` 等 id 在子代理跑完后才由
     `build_structured_result` 后置分配、运行中不可见，模型只能自造 id；
     实证 0h S4（2 次）/ mteb-retrieve（5 次）/ R5（1 次）全部
     organized_response 空 / visibility_degraded=true /
     structured_result_validation_failed，唯一通过路径是知情单测；
     SRC-002（声明行 ref 污染：URL+标题整行入 source_url_or_ref）为其
     表象。
   - **修复**：删除 `[RESULT_JSON]` 契约——子代理提示词移除模板段、
     `parse_retrieval_result_json` 删除、`build_structured_result` 删
     block 解析/校验/annotation 合并、`organized_response` 从 committed
     payload/schema 删除；回归 `[DOC]`/`[SOURCE]` 声明行 + 机械 ledger
     单轨（§3.3.3 条 3 机械四段）；`[SOURCE]` 声明行 URL 走 ACAF 网络
     目标规范化（scheme/host 小写、去默认端口、去 fragment、userinfo
     拒绝、尾部标点剥离），标题独立入 source_title；
     `visibility_degraded` 语义重定义为「机械 ledger 无文本级证据
     （full_text_observed 与 partial_text_observed 均 0）」，reason code
     为 `no_fulltext_evidence`；dispatch 摘要结论计数改口径为文本证据
     计数；schema（retrieval-result-event-payload-v0.2）/ Python verifier
     （organized_response 校验与 source_weighting annotation 校验删除、
     补 visibility_degraded 一致性规则）/ fixtures / 测试同步；
     §3.7 条 12 第三层（子代理模型加权标注，生产中也从未生效）退役——
     机械来源梯队 ① 与选择性原文核验 ② 语义不变。
   - **登记**：BACKLOG 0k / TODO P0-0k / 门禁观察报告
     `docs/GATE_GOOGLE_OBSERVATION_2026-08-30.md` §10.3 /
     CLI_PROJECT_INDEX（GAP-RETRIEVAL-STRUCTURED-RESULT）。

2. **方向 C 全面审查轮修复（2026-08-30 三路审查：设计合理性 / 实现
   合理性 / 设计与实现符合性；审查发现的处理）**：
   - **内容寻址纪律**：`result_digest` = sha256(canonical(四段机械
     payload：query_summary/source_ledger/filtering_log/
     raw_source_refs))、`ledger_digest` = sha256(canonical(source_ledger))、
     `result_id` 携带 result_digest 前 16 位——verifier 复核；修改 journal
     后必须按 verifier 自身算法重算 digest 并重链事件哈希（含
     disposition/close 的 assessment_id 与 result_digest 重映射）。
   - **reason_codes 词汇表**：v0.2 检索评估仅允许
     `no_mechanical_coverage_requirement` 与 `no_fulltext_evidence`
     （degraded ⇒ 必须携带后者）；块时代词条
     `structured_result_validation_failed` /
     `low_quality_source_without_annotation` 退役并在 verifier 拒绝。
   - **used_in_sections 退役**：v0.2 source_entry 删除该恒空残留字段
     （无消费者；v0.1 replay-only 通道的 retrieval-result-v0.1.schema
     不受影响）。
   - **[SOURCE] 规范化边界固化**：无空格「URL，标题」形态在 scheme 后
     首个标点处切分（ASCII ':' 不作为分隔符，保留 scheme/port）；
     userinfo URL 被 ACAF 解析器拒绝时原样保留——「拒绝」语义为不规范化、
     不丢弃声明行。
   - **范围注明**：方向 C 退役范围 = Rust v0.2 生产轨道；Python
     `assurance/retrieval_subagent.py` / `deepseek_runtime_adapter.py`
    的 v0.1 replay-only 通道保留 organized_response，不在退役范围，
     S4 复验不得误用该通道。
   - **S4 实机复验闭环（2026-08-31）**：检索密集题 k=1 实机三轮
     （s4-2026-08-31 / -31b / -31c），终轮 2/2 reward 1.0、0 异常、
     两 journal 事件链 verifier 0 错误；payload 无 organized_response、
     visibility_degraded=false、URL 尾巴 0、空 title 0；close record
     effort=extended 落盘。复验发现并修复 3 项：① 并行批次预算拒绝
     重复写 tool_completed 的 F11 违例（`refuse_inject_budget` 增
     write_completed 参数，并行路径只注入消息面 + deny）；②
     split_source_declaration 空格分支全角左括号提前截断 + 无空格
     分隔符集合补「（」；③ tool_completed payload 补 `epoch` 声明 +
     非 URL 声明行 source_title 回退整行（对齐 retrieval-result
     schema nonempty）。详见
     [`S4 复验记录`](../docs/audits/GAP_RETRIEVAL_STRUCTURED_RESULT_AND_BATCH2_S4_VERIFICATION_AUDIT_2026-08-31.md)。

### 14.46 v1.46 补写裁决索引（2026-08-30）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **检索编排机械层第二批设计定稿（2026-08-30 独立设计轮；用户裁决
   「按建议落实」；修订 §3.7.6 条 6 的 tab 生命周期措辞）**：
   - **`project_doc_index` v2**：git HEAD 基线 + 工作树增量层
     （`git status --porcelain -z --no-renames --untracked-files=all`）+
     Blake3 内容哈希（条目内容身份）+ 驻留索引（dirty 未置 + 节流窗口内
     query 零全树 stat）+ 写后失效钩子（search_replace / run_terminal_cmd
     完成后 `mark_dirty`，模型刚写内容立即可搜）；非 git 工作区回退 v1
     （每 query 全树 stat 语义不变）。同 size+mtime 且 git 双 clean 的
     残余盲区登记收敛（逃生阀保留）。
   - **会话级 tab 池 + 同轮多页并行读取**（§3.7.6 条 6 措辞修订）：
     有界 N 个 CDP target 常驻（默认 4，`ORZ_BROWSER_TAB_POOL_SIZE`，
     0=回退每调用 create/close），每次读取**独占租约**、归还复用、LRU
     选空闲；「one tab lives exactly for this call」语义调整为「一次调用
     独占一个 target 的控制权」——target 可机械池化复用，模型永不接触
     tab 句柄、内容/控制权不跨调用共享（同一 session 单 profile 的
     cookie/localStorage 共享是既有事实）；manager 不再整读持锁，同轮
     多个 `browser_read` 真正并发（`PARALLEL_READ_TOOLS` 已含
     browser_read）。下载路径（`download_or_read`）不池化。顺带 DNS
     预检会话级缓存（host + TTL，默认 300s，redirect 重检门保留）。
   - **委托契约复杂度分档**：纯函数 `classify_retrieval_effort`
     （query 长度 / 广度聚合词面 / scope 形态 / max_results / lane）→
     三档 `standard`/`extended`/`deep`；档位只调执行预算——墙钟
     240/600/900s、轮数 30/60/90（与主车道取 min）、max_results 默认
     5/8/12、同轮 browser_read 并行 2/4/不限；**不扩大 [DOC]/[SOURCE]
     16 行/8K 与注入预算上限**；显式 env
     （`ORZ_RETRIEVAL_SUBAGENT_TIMEOUT_SECS` /
     `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS` / `ORZ_RETRIEVAL_EFFORT`）优先于
     档位默认；`retrieval_close_record` 登记可选 `effort`（schema 先行）。
   - **登记**：BACKLOG 0k / TODO P0-0k / 设计文档
     `docs/RETRIEVAL_ORCHESTRATION_MECHANICAL_BATCH2_DESIGN_2026-08-30.md` /
     CLI_PROJECT_INDEX（FUS-RETRIEVAL-ENGINE-SERP /
     GAP-PROJECT-DOC-INDEX-CACHE）。
   - **审查处理（2026-08-30 S1 全面审查轮）**：max_results 缺省语义裁决为
     「档位默认 5/8/12 机械并入契约后执行面恒有界，故无界不加分（<5/缺省
     → +0）」；档位阈值按连续边界 ≤1.0/≤3.0/>3.0 裁决（1.5/3.5 分别归
     extended/deep）；project_doc_index v2 补 HEAD 移动闭环（刷新重取
     `git ls-files` 合并 tracked 条目）与 D1-1 缓存 path 重建、git 命令
     统一 10s 超时回退 v1；子模块内容盲区与 DNS TTL 内重绑定窗口登记为
     已知边界（逃生阀兜底）；详见设计文档 §10。

### 14.47 v1.47 补写裁决索引（2026-08-30）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **机械层数学计算体阶段 1 设计定稿（P2-10 F1–F6；2026-08-30；转录后
   AUTH-MECHANICAL-LAYER-MATH-CALCULUS 由 `pending` 转 `current-design`）**：
   - **总体形态**：三层——模型（调用者/决策者，只发 term、只读表面）、类型化
     项重写系统（纯函数确定性归约 + 效应过监视自动机 + 依赖图内部状态）、LIF
     （全局时间监控外挂，独立于重写语义）。唯一耦合面 = 事件流。不变量：term
     不能写 LIF；LIF 只读事件、只按自身动力学演化、只发 fires（运行起始配置
     除外）；fires 仅内部留痕（§9.7 裁决：不入事件面 schema/verifier、不渲染、
     不注入），round50 维持现有软门提醒形态，不扩展为强制模板轮。
   - **F1 工具类型契约（正式契约草案，实施以 I5 为准）**：8 工具统一 Result
     信封（成功 = 摘要 ≤200 B + cap + pointer；失败 = Fail{step:
     arg_validation|gate|execution|delivery, code, message ≤200 B, trace_id}）；
     纯子项错误（参数/正则编译失败/no_match/ambiguous）= 类型化错误值
     `step=arg_validation`；效应错误 = fail-closed 信封不变；terminal
     `exit_code≠0` 为结构化值。`file.read` → View{path,hash,size,window≤16 KiB,
     truncated}（hash/size 即写前锚点）；`file.grep` → Grep{files_searched,
     matches≤64, truncated}（纯函数）；`file.search_replace` GetPut 律（anchor
     不符 → anchor_mismatch；n=0 → no_match；n>1 → ambiguous，歧义
     fail-closed）+ 返回 new_anchor；`terminal.run` → Cmd{exit_code?, wall_ms,
     timed_out, stdout/stderr_tail≤8 KiB}（wall_ms 一等字段；host 拥有
     cwd/env/超时）；`retrieval.web_search` → Search{sources≤10, count,
     EvidencePtr}；`retrieval.web_fetch`/`browser_read` 同族 → Fetch{
     canonical_url, status, size, EvidencePtr}；`blackboard.read` →
     Board{entries≤8 KiB, total_cap}，temporal 分区为时间特征查询口；
     `delivery.submit` 两阶段（stage 2 后无工具事件 LTL 不变量）。折中档（D1）：
     顶层保留 tool_calls + 一层组合（pipe），单轮 ≤4 步、效应数复用候选计数/
     预算硬门、不引入部分结果注入。
   - **F2 temporal 分区渲染规格（修订 §3.6 黑板分区表，新增 temporal 分区）**：
     观测记录面；TemporalRecord{t, domain, entry_round, dwell_rounds, u_prog,
     u_err, u_stuck, T̂, err10, succ10}；域 = 语义谓词（Start/Normal/Pressure/
     LowProgress/Stuck；Recovery 为轨迹属性）：low_progress = u_prog<0.5、
     pressure = (u_err≥2)∨(u_stuck≥1.5·T̂)；查询面 Now/Recent(k≤20)/History/
     Feature(name,k≤20)，Board ≤1 KiB；渲染约束 = 只渲染 ≤ 查询时刻、fires
     不渲染、启动态单独标记、有界内存 O(1)/决策、每决策轮重算；存档 = 域切换
     spike 随会话侧车（StoredConversation envelope 可选字段
     `temporal_spikes?: Vec<DomainSpike>`，serde(default)、7 天 retention、
     子代理不同构不持久化）；阈值初值 u_err≥2、u_prog<0.5、θ_stuck=1.5·T̂、
     err10/succ10 窗口 10（语义推导非拟合）。
   - **F3 LIF 时间外挂计算规格**：特征集按决策点反推（u_prog/u_err/u_stuck/
     T̂/err10/succ10，每个特征对应一个模型真实决策）；一阶事件驱动 LIF
     u(t)=u(t₀)·e^(−Δt/τ)+w；通道登记——err（τ=180s 固定、θ=4 事件、
     refractory 60s、full-reset）、stall（90s 固定墙钟间隔探测）、slow
     （τ=600s、θ=3）、deny（τ=120s、θ=4）、prog（成功置 1、τ=8·T̂）、stuck
     二阶（I=u_err/θ_err·(1−u_prog)、τ=3·T̂、θ=1.5·T̂、refractory=8·T̂）；
     速率/间隔探测器（err/stall）用固定时间语义，持续/新鲜度通道
     （stuck/prog）用轮语义（秒值经 T̂ 换算、非学习）；二阶闭式解
     （α=1/τ_err−1/τ_stuck、β=1/τ_err+1/τ_prog−1/τ_stuck，带泄漏核精确积分、
     expm1 实现、禁止右端点离散化——已证明 7–21× 伪迹）；T̂ 估计器 = 有界窗
     （8–32 决策间隔）中位数、>300s 截尾、T̂₀=8s、≥8 样本启用、钳制
     [3,600]s；err 输入定义（2026-08-31 审查处理 R1 裁定）= 宿主级错误
     （status=error 且无 exit_code 值）+ 超时，exit≠0 为 D2 值（Other 中性），
     离线复验与生产同谓词（102-run：err 1 run/1 fire）；四对照门（C1 时间
     打乱 / C2 spike 移除 / C3 leak=0 计数 / C4 EMA 对照；二阶通道 C2≡C4），
     V2 离线复验为正式判定门；fires 仅内部留痕。
   - **F4 失败目标身份入事件面（设计定稿，实施 I2 前置）**：目标身份 = 命令
     digest（sha256(canonical(cmd)) + ≤80 B 预览）/ 锚点 {sha256,size}
     （运行时形状，2026-08-31 审查处理 F13 统一）/
     文件路径 / canonical URL；失败事件增可选 failure_target{kind,id}；归约
     receipt 与事件链逐段同构、身份字段一致（verifier 交叉核对）；命令/检索
     副作用不建图（D3），只入事件面字段。
   - **F6 参考系重定位与观测面扩充**：LIF = 时间性参考系（低精度容忍、模型
     无感硬边界：计算全落机械层，模型仅 PULL temporal 渲染结果）；双尺度基线
     （长窗 = 整 run 环境基线、短窗 ≈3 分钟 = 当前偏离、渲染 = 基线 + 当前 +
     比值）；每轮校准分层（语义固定、秒值经 T̂ 换算；err 墙钟 180s 锚 + T̂ 钳制
     防自指反馈；早期小样本标注可信度）；观测面扩充优先级 = 失败目标身份 →
     stated-vs-done → 环境进展接地 → 序结构 → 混杂控制；动作层耦合试验 P1–P3
     （progress/curvature、域序列预测、stated-vs-done 可预测性；102 runs 离线
     可跑；成立才宣称"时间性 + 全局视野外挂"值得做）。
2. **登记**：BACKLOG P2-10 / TODO P2-10 / 正式设计
   `docs/MECHANICAL_LAYER_MATH_CALCULUS_DESIGN_2026-08-30.md` / 讨论稿
   `docs/MECHANICAL_LAYER_MATH_CALCULUS_DISCUSSION_2026-08-30.md` /
   CLI_PROJECT_INDEX（AUTH-MECHANICAL-LAYER-MATH-CALCULUS）。阶段 2 实施
   切片（I1–I6）与阶段 3 验证（V1–V3）按 BACKLOG 纪律放行时入账。

### 14.48 v1.48 补写裁决索引（2026-08-31）

本节记录冻结后的显式补写；规范正文以所指章节为准，补写明确
取代以下既往条款。

1. **PULL 自描述（P2-11 第 1 项，2026-08-31 用户确认采用；转录后
   AUTH-PULL-SELF-DESCRIPTION 登记为 `current-design`，实施为 `partial`——
   S1 代码 / S2 测试完成，S3 重建 + S4 实机复验待放行，验证闭环后转
   `implemented`）**：`blackboard_read` 响应自描述化——任何**成功的 live
   读取**（`epoch` 省略）响应头部携带「自上次读取以来」增量行
   `[黑板增量] <分区>+<n> … 域迁移+<n>: <from→to>@r<轮>`：各分区变化计数
   （分区版本计数 = run 内单调、每次可见内容变化计 1，覆盖式变化如注册板块
   替换/计划状态迁移/轮换清空亦计 1；`session` 派生自 tool_rounds、`temporal`
   派生自 LIF round）只列 delta>0 者；temporal 域迁移摘要以 `migration_count`
   （单调总计数，区别于有界迁移队列长度）对**独立迁移计数基线**（上次成功
   读取 temporal 时的 migration_count，与 round 徽章游标分存，2026-08-31
   审查处理 M1 双基线）的增量输出最近一次迁移。读取游标 = run 级每分区
   「上次成功读取」位置，读某分区只推进该分区游标（未读徽章模型；temporal
   双基线同时推进），失败路径与归档 epoch 读不推进、不挂头；渲染层失败形状
   （未知分区 / receipt_id 组合误用 / 点读未找到，O4 先例保持 exit_code 0）
   同样不挂头、不推进（审查处理 M2）；增量头自身
   ≤256 B，无增量且无迁移时零噪音不加头。**temporal 单次查询按模型意图一次
   返回**：now 追加近 5 轮趋势行与上一迁移行、recent(k) 首行压缩摘要
   （域/入域/驻留 + 起止特征）、feature(name,k) 尾注当前值与域；整响应（增量
   头 + 查询体）仍 ≤1 KiB。硬边界不变：零注入（增量头只出现在模型主动 PULL
   的响应内）、模型无感边界不变（无新常驻 token、无主动注入、不新增/改名/
   删除工具）、只给事实不给建议。已知边界：子代理与主代理共享游标（子代理
   工具投影面不含该工具，运行中不触发）；并行同分区读取允许轻微竞态；游标随 run
   复位（跨 prompt 续 run 首次读取计为未读）。入口：正式设计
   `docs/PULL_SELF_DESCRIPTION_DESIGN_2026-08-31.md` / BACKLOG P2-11 /
   TODO P2-11 / CLI_PROJECT_INDEX（AUTH-PULL-SELF-DESCRIPTION）/ 审查修复审计
   `docs/audits/P2-11_PULL_SELF_DESCRIPTION_S1_REVIEW_AUDIT_2026-08-31.md`。
