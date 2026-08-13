# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。

## P0 — 当前工作集

### P0-B FUS-RETRIEVAL-MECH（`partial`；设计 approved，B-1 已闭合）

依赖顺序：1（B-1，已闭合）→ 2/3 → 5 → 4/6。

- [x] 步骤 1（B-1）：web_search citations 结构化透传进 loop（`ToolResult.structured` 接缝 + 证据账本 `candidate_urls` / `raw_source_refs` 镜像）。
- [x] 步骤 2：web_fetch 候选机械计数门禁与计数反馈——`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8（2026-08-14 用户裁决；activation 累计 + 侧车持久化、去重后 URL 计数、超限显式拒绝、无 ToolStarted、熔断同面、`tool_completed` 计数字段 Schema 先行）。
- [x] 步骤 3：机械预筛模块——候选池净化（canonical URL / host 级去重、已知失败形态、相关性粗筛）+ tier/weight 排序标签，进结构化结果（2026-08-14 闭合；`candidate_urls` 升级为预筛后保留池，`candidate_pool`/`prefilter_log` 契约先行）。
- [ ] 步骤 5：输出级引用校验器与交付边界接线——`[来源]` 结构化解析、source_id/visibility/claim 上限校验、失败显式降级（覆盖 V11-IMPL-004 的 source binding 与 claim verifier）。
- [ ] 步骤 4：browser_read 范围/模式参数扩展（全文/预览/关键词提取；local_browser 第二段复用同一计数域——前置裁决：browser_read 为主车道工具，计数域挂载面待定，见设计 §1.3 注）。
- [ ] 步骤 6：提示词相应缩短（计数/预筛/引用规则）与测试更新。
- [ ] 建议并入：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）接线。

入口：[检索机械控制设计](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / [B-1 实施审计](docs/audits/GAP_RETRIEVAL_MECH_B1_CITATIONS_IMPL_AUDIT_2026-08-13.md) / [步骤 2 实施审计](docs/audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md)。

### P0-C CLASSICAL-EXEC-ASSISTANT（`pending`；POC 已通，v0.5 操作台模型）

- [x] 小样 1：控制台路由（服务注册表 + hassil 意图 + stdio JSON；28/28 检查通过）。
- [x] 槽位表由工作区索引动态生成（POC 闭合；生产接线复用 orz `project_doc_index` 缓存）。
- [ ] 小样 2：编辑执行器 `workspace.search_replace`（收益裁决点：编辑应用成功率 + 主模型工具轮数）。
- [ ] 小样 3：机械组合脚本模式（线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）。
- [ ] orz 内嵌集成：HA 操作台接线（薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝；POC stdio 仅原型隔离）。
- [ ] 正式组件决策门：小样全面达标后裁决；不达标即撤。

入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md)。

## P1 — 可并行审计 / 证据

### FUS-COMPONENT-REGISTER（`partial`）

- [ ] 逐 crate/component 采用审计——65 组件全 `audit_required`；从当前代码可达性与 local diff 出发，不得由 crate 名/编译推断采用档位（V11-IMPL-008）。

入口：[register yaml](upstream/fusion-component-register-v0.1.yaml) / [register schema](upstream/fusion-component-register-v0.1.schema.json)。

### GAP-WINDOWS-EVIDENCE（`partial`）

- [ ] ORZ-WIN-PROC-001/002/003 案例晋级——真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
- [ ] 建立 Windows 平台兼容性设计文档（ADR-0010 §6/§11.7 目标：`architecture/WINDOWS_PLATFORM_COMPATIBILITY_DESIGN_v0.1.md`）。
- [ ] 建立 `regression/windows/` 与 `.observed-runs/windows/` 路由（自动回归/人工复核入口与 git-ignored 原始运行目录）。

入口：[incidents](docs/incidents/windows/README.md) / [cases](docs/cases/windows/README.md)。

### IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- [ ] transport/retry/thinking 按主/子代理同构约束复核（ADR-0007 / DEEPSEEK_ADAPTER_CONTRACT）。
- [ ] DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。

入口：[ADR-0007](adr/ADR-0007-transport-retry-policy.md) / [DEEPSEEK_ADAPTER_CONTRACT](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)。

## P2 — 生产化决策门

### IMPL-CONTROL-FABRIC（`partial`）

- [ ] fail-closed 生产启用（用户裁决；前置核查：web_search 票化形态⑦、host 侧执行参数绑定面⑨、执行面与票据绑定面错位⑩、network 重定向不重新票据⑪）。
- [ ] Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
- [ ] Slice 4：Windows Sandbox backend（D-11）。
- [ ] 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。

入口：[ADR-0011](adr/ADR-0011-authenticated-control-and-action-fabric.md) / [ACAF 设计](docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md) / [fail-closed 审计](docs/audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。

### OPS-PROTOCOL（`pending`；裁剪方向已定）

- [ ] 产出裁剪设计：删除安全保留为 host-owned 工具、跨环境桥接内部化、双执行器收敛单一参考（生产走 Rust 工具面）。
- [ ] 生产接线裁决（先验票，再由协议执行器执行）。

入口：[协议](protocol/structured-operation-protocol-v0.1.md) / [Schema](protocol/structured-operation-protocol-v0.1.schema.json)。

## P3 — 收尾 / 清理

- [ ] EVIDENCE-LOCAL-BROWSER：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
- [ ] GATE-CHAIN：分层 gate 链与融合 runtime 的最终接线随切片审计复核。
- [ ] V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
- [ ] V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留——复核并登记闭合或转 gap。
- [ ] orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`）——复核并登记闭合或转 gap。
- [x] 前置收尾：提交当前未提交登记（995a384，2026-08-13）。

## 审计登记边界（条件触发，不占当前优先级）

- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。

## 近期已闭合（供核对，不计入开放项）

- [x] FUS-TOOL-PROBE：P0-A 步骤 1-7 与 P0-A-2 全部闭合（ADR-0010 v1.8，23 个工作工具单一探针面）。
- [x] FUS-SOURCE-WEIGHTING-IMPL：来源加权实现闭合（机械三档 + 机器可读种子名单 + 模型加权标注）。
- [x] GAP-ENCODING-GATE：机械编码门控闭合。
- [x] GAP-ACAF-SLICE1 / SLICE2A / SLICE2B / FAILCLOSED 与 GAP-DENIAL-POLICY-REVISION：ACAF 实施切片闭合（fail-closed 生产启用仍待裁决）。
- [x] OPS-PROTOCOL 审查判定登记（裁剪方向定案；裁剪设计待产出）。
