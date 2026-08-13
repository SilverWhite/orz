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

### 0. 前置收尾（提交前需用户确认）

- 提交当前未提交登记：CLI_PROJECT_INDEX 索引更新、两份设计文档（含优先级标记）、本文件与各指针更新。

### 1. FUS-TOOL-PROBE（`pending`）

- 入口：[设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：用户裁决是否进入实施、ADR-0010 §3.5 修订草案是否采纳。
- 实施序列：
  1. 面 B 每工具探针实现（路径/权限/策略/runner/交互用户判定）；
  2. `tool_availability_check` Schema/fixture/verifier 升级与 producer 接线；
  3. run_tests 条件声明迁移；
  4. 列表投影接线（面 A + 面 B 完整集 + 面 C，仅名称）；
  5. 最小上一轮映射与翻转事件；
  6. 兜底消息中性化改造；
  7. ADR-0010 §3.5 修订裁决与登记。

### 2. FUS-RETRIEVAL-MECH（`pending`）

- 入口：[设计](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：用户裁决是否进入实施。
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
- 工作区收尾：见 P0 前置收尾。

## 变更记录

- 2026-08-13：建立统一待办；P0-P3 优先级按全量回查结果登记；设计/审计文档待办小节改为指针。
