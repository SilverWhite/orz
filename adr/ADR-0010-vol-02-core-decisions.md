# ADR-0010 分卷 02：§2 核心决策

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 02（§2 核心决策）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

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

