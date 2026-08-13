# ORZ 统一待办与优先级（BACKLOG）

> 状态：living（单一待办路由）；建立：2026-08-13。
> 定位：本文件只做未闭合项召回、优先级和决策门登记；不替代 ADR、Schema、审计、索引或源码。设计裁决以 ADR-0010 / ADR-0011 和 [`CLI_PROJECT_INDEX.md`](../CLI_PROJECT_INDEX.md) 的 canonical entry 为准。
> 维护纪律：新增、关闭或调整优先级只在本文件登记；设计文档与审计的“待办/下一步”小节只保留指针或审计时点历史，不重复维护明细；索引只登记本文件的召回路由。

## 优先级总览

| 优先级 | 含义 | 未闭合项 |
|---|---|---|
| P0 | 当前工作集：设计已冻结，裁决后立即实施 | FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH |
| P1 | 无需裁决，可与 P0 并行 | FUS-COMPONENT-REGISTER、GAP-WINDOWS-EVIDENCE、IMPL-DEEPSEEK-TRANSPORT / SEC-CREDENTIALS |
| P2 | 生产化决策门：需用户裁决 | IMPL-CONTROL-FABRIC（fail-closed 启用、Slice 3/4）、OPS-PROTOCOL |
| P3 | 收尾 / 清理 | EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、DC 剩余信号、V11-IMPL-003/007、工作区收尾 |

## P0 — 当前工作集

> 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施；执行顺序 P0-A（工具探针）优先，P0-B（检索机械控制）紧随。

### 0. 前置收尾（提交前需用户确认）

- 已完成（995a384）：提交当前未提交登记——CLI_PROJECT_INDEX 索引更新、两份设计文档（含优先级标记）、本文件与各指针更新。

### 1. FUS-TOOL-PROBE（`approved`；P0-A，实施中）

- 入口：[设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施；ADR-0010 §3.5 修订采纳按批次末第 7 步登记。
- 进度（2026-08-13）：步骤 1 已完成——面 B 探针模块（orz-loop `tool_probe.rs`：两态中性判定 + 失败兜底 + 面矩阵成员与判定单测 15 项）；步骤 2 已完成——v0.2 事件升级（新 payload Schema + verifier 交叉校验 + fixture 重生成 + producer 接线 + TUI 消费面适配），13 个真实 journals 重捕并验证；步骤 3 已完成——`run_tests` 条件声明迁移（controller 删除直接 `test_runner()` 条件声明，声明改由面 B 探针快照驱动：完整才列出、不完整即移除，中性 reason `缺少测试运行器`；新增 registry 已声明但 runner 缺失/存在两条迁移语义测试）；步骤 4 已完成——列表投影接线（tool_probe 补面 A/面 C 常量与 `is_main_agent_work_tool` 判定；controller 模型可见列表 = 面A + (面B完整集 ∩ 会话声明集) + 面C + 非工作工具，仅名称；ReadOnly 下 `search_replace` 写探针不完整即移除，面 C/非工作工具不探不标；新增投影分区、交互会话、面 A/C 成员判定测试，Benchmark/ReadOnly 全量声明测试同步新语义）；步骤 3/4 全面审查清理已完成——run_tests 无 runner 竞态调用改为 dispatch 前无 ToolStarted 中性拒绝（`missing_test_runner`），补 workspace 不可读投影移除与竞态兜底测试；步骤 5 已完成——最小上一轮映射与翻转事件（`MinimalProbeMap` 仅存 `tool → 完整/不完整`、不缓存 reason、不跨 run；每个模型请求构造前重算面 B 快照并重算列表投影，翻转才发 `tool_availability_check` 事件、无变化不发；调用即探针：面 B 工具 ToolCompleted(error) 回写最小映射——run_tests 竞态拒绝/spawn 失败/ACAF fail-closed 拒绝/宿主调用错误，回写仅主/grill 车道、检索车道不写主映射（审查修复）；主/grill 车道探、检索车道不探不发；新增翻转重投影、调用失败回写恢复翻转、检索车道零污染、跨 run 重置四项集成测试）；步骤 6 已完成——兜底消息中性化改造（权限门禁拒绝消息改 `tool 'X' — 本次调用未获权限门禁放行`；连续拒绝熔断块改中文中性陈述，移除 available/refused 旧措辞；系统提示词改"工具列表由运行时按轮声明"；检索车道按复核裁决保持原设计，拒绝/宿主执行消息不脱敏；run_tests 管道错误改 unreadable；DC 最小动作改"完整错误输出"；补中性词契约测试）；orz-loop/host/tui 测试全绿、仓库门禁 valid。下一步：步骤 7（ADR-0010 §3.5 修订裁决与登记）。
- 审查裁定（2026-08-13，子代理三路审查）：步骤 4 列表投影语义定为 `面A + (面B完整集 ∩ registry 声明集) + 面C`（部分会话变体可移除 ask_user_question/search_tool，探针不得声明会话不存在的工具）；探针粒度=工作区根级机械检查、写探针 metadata-grade、交互用户信号=ACP live gateway，均已登记进设计文档。
- 实施序列：
  1. 面 B 每工具探针实现（路径/权限/策略/runner/交互用户判定）；
  2. `tool_availability_check` Schema/fixture/verifier 升级与 producer 接线；
  3. run_tests 条件声明迁移；
  4. 列表投影接线（面 A + 面 B 完整集 + 面 C，仅名称）；
  5. 最小上一轮映射与翻转事件；
  6. 兜底消息中性化改造；
  7. ADR-0010 §3.5 修订裁决与登记。

### 2. FUS-RETRIEVAL-MECH（`approved`；P0-B）

- 入口：[设计](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施。
- 依赖顺序：1（B-1 闭合）→ 2/3 → 5 → 4/6。
  1. B-1 闭合：web_search citations 结构化透传 loop；
  2. web_fetch 计数门禁 + 计数反馈 + `ORZ_WEB_FETCH_CANDIDATE_CAP` 接线；
  3. 机械预筛模块（候选池净化 + 排序标签）与结构化结果扩展；
  4. browser_read 范围/模式参数（全文/预览/关键词提取）工具能力扩展；
  5. 输出级引用校验器与交付边界接线；
  6. 提示词相应缩短（计数/预筛/引用规则）与测试更新。
- 建议并入：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）接线。

## P1 — 可并行审计 / 证据

### 3. FUS-COMPONENT-REGISTER（`partial`）

- 开放内容：65 组件全 `audit_required`，逐 crate 采用审计未开始（V11-IMPL-008）；不得从 crate 名/编译推断采用档位。
- 入口：[register](../upstream/fusion-component-register-v0.1.yaml)；[V1.1 复核](audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。

### 4. GAP-WINDOWS-EVIDENCE（`partial`）

- 开放内容：ORZ-WIN-PROC-001/002/003 仍为 `candidate`；需真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
- 入口：[incidents](incidents/windows/README.md)；[cases](cases/windows/README.md)。

### 5. IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- 开放内容：transport/retry/thinking 按主/子代理同构约束复核；DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7）。
- 入口：[DEEPSEEK_ADAPTER_CONTRACT](../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)；[ADR-0007](../adr/ADR-0007-transport-retry-policy.md)；[ADR-0006](../adr/ADR-0006-credential-target-registry.md)。

## P2 — 生产化决策门

### 6. IMPL-CONTROL-FABRIC（`partial`）

- 决策门：用户裁决 fail-closed 生产启用（前置：探针矩阵 + Slice 2B §6 核查清单 ①-⑪，⑦⑨⑩⑪ 仍登记）。
- 剩余核查项：web_search 票化形态；host 侧执行参数绑定面；执行面与票据绑定面错位；network 重定向不重新票据。
- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
- Slice 4：Windows Sandbox backend（D-11）。
- 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。
- 入口：[ADR-0011](../adr/ADR-0011-authenticated-control-and-action-fabric.md)；[fail-closed 审计](audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。

### 7. OPS-PROTOCOL（`pending`）

- 开放内容：v0.1 协议、Schema、Python/PowerShell 执行器已就位；生产接线待裁决（先验票，再由协议执行器执行）。
- 入口：[协议](../protocol/structured-operation-protocol-v0.1.md)。

## P3 — 收尾 / 清理

### 8. EVIDENCE-LOCAL-BROWSER（`partial`）

- 开放内容：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
- 入口：[LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE](../存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md)；[GAP_PDF_EVIDENCE_IMPL_AUDIT](audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md)。

### 9. GATE-CHAIN（`partial`）

- 开放内容：分层 gate 链与融合 runtime 的最终接线随各切片审计复核（不单开大项）。
- 入口：[assurance](../assurance/)；[orz-assurance](../orz/crates/orz-assurance/)。

### 10. 遗留小项

- DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（建议并入 P0/检索机械控制批次）。
- V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
- V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留检查——复核并登记闭合或转 gap。
- orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`，顺序/负载相关、与本批无关）——复核并登记闭合或转 gap。
- 工作区收尾：见 P0 前置收尾。

## 变更记录

- 2026-08-13：P0-A 步骤 6 完成登记——兜底消息中性化改造（权限门禁拒绝消息、连续拒绝熔断块、系统提示词、run_tests 管道错误与 DC 最小动作中性化，error 码与行为不变；新增系统提示词/熔断块中性词契约测试）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 7。
- 2026-08-13：步骤 6 全面复核裁决与修正——检索车道不适用本设计（回退检索分发拒绝消息与对应测试断言至原措辞）；事件 error 码/机器 reason 与明确事实性内容可进模型面；面 C 工具 schema 描述保留；deny 消息标点统一、中性词测试覆盖补齐 8 项；设计文档 §2/§3 登记适用范围裁决；orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 2026-08-13：步骤 5 审查修复登记——调用即探针按车道隔离（`run_host_tool` 增 `probe_writeback` 标志，主/grill 车道才回写最小映射，检索车道失败只走 ToolCompleted(error) 审计，防主车道审计/事件流污染）；补检索车道零污染与跨 run 映射重置两项回归测试；设计文档补 §5 车道边界与 §8 run-start 首翻澄清；orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 2026-08-13：P0-A 步骤 5 完成登记——最小上一轮映射与翻转事件（每模型请求前重算、仅翻转发事件、调用即探针回写、列表投影移入共享循环按轮重算、检索车道不探）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 6。
- 2026-08-13：步骤 3/4 全面审查清理——run_tests 无 runner 竞态调用改为 dispatch 前无 ToolStarted 中性拒绝（`missing_test_runner`）；补 workspace 不可读投影移除与竞态兜底测试；设计文档状态转 `approved`、删除 image_edit 探针行、登记面 A/C 同源约束与竞态兜底；controller/agent_loop 注释与新语义对齐；索引 FUS-TOOL-PROBE 转 `partial`。
- 2026-08-13：P0-A 步骤 4 完成登记——列表投影接线（面A + 面B完整集∩声明集 + 面C + 非工作工具；tool_probe 补面 A/C 常量与判定；ReadOnly 写探针过滤落地）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 5。
- 2026-08-13：P0-A 步骤 3 完成登记——`run_tests` 条件声明迁移至面 B 探针（controller 删除直接条件声明；探针完整才声明、不完整即移除；新增两条迁移语义测试）；orz-loop/host/tui 测试全绿、仓库门禁 valid；批次下一步为步骤 4。
- 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施（ADR-0010 §3.5 修订按 P0-A 批次末第 7 步登记）；执行顺序裁定：P0-A 工具探针 → P0-B 检索机械控制 → P1 并行审计 → P2 核查收口/裁决/Slice 3/OPS 接线/Slice 4 → P3 收尾。
- 2026-08-13：P0-A 步骤 1-2 完成登记——面 B 探针模块与 `tool_availability_check` v0.2 事件升级（Schema/verifier/fixture/producer/TUI），真实 journals 重捕；批次下一步为步骤 3。
- 2026-08-13：审查处理登记——修复 Rust TUI incomplete 计数缺陷、README/设计示例一致性、TUI 状态段语义与判定词中性化、gate_decision 预留注释；登记步骤 4 交集语义与 orz-host 既有 flaky。
- 2026-08-13：建立统一待办；P0-P3 优先级按全量回查结果登记；设计/审计文档待办小节改为指针。
