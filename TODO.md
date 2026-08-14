# ORZ 待办项记录（TODO）

> 状态：living（实施勾选清单）；建立：2026-08-14。
> 定位：面向后续实施与改动的操作型清单，派生自 [`docs/BACKLOG_AND_PRIORITIES.md`](docs/BACKLOG_AND_PRIORITIES.md) 的未闭合项；优先级、决策门与状态权威仍是 BACKLOG，设计权威是 ADR-0010 / ADR-0011，召回路由是 [`CLI_PROJECT_INDEX.md`](CLI_PROJECT_INDEX.md)。本文件只登记指针与勾选状态，不新增设计裁决，不重复维护设计细节。
> 维护纪律：完成一项勾选一项，并同步 BACKLOG 与索引状态；新增或调整优先级只改 BACKLOG；删除或归档条目前先回查索引入口；每次改动保持本文件与 BACKLOG 的入口一致性。
> 入口：索引 AUTH-TODO（2026-08-14 登记）。

## 使用说明

- `[ ]` = 待办；`[x]` = 已完成（保留供核对，不计入开放项）。
- 每项标注 canonical ID / 优先级 / 关键内容 / 入口；同一概念只出现一次，不复制 BACKLOG 的决策记录。

## P0 — 当前工作集

### P0-B FUS-RETRIEVAL-MECH（`implemented`；批次 1-6 已全部闭合，2026-08-14，保留供核对）

依赖顺序：1（B-1，已闭合）→ 2/3/5（已闭合）→ 4（已闭合）→ 6。

- [x] 步骤 1（B-1）：web_search citations 结构化透传进 loop（`ToolResult.structured` 接缝 + 证据账本 `candidate_urls` / `raw_source_refs` 镜像）。
- [x] 步骤 2：web_fetch 候选机械计数门禁与计数反馈——`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8（2026-08-14 用户裁决；activation 累计 + 侧车持久化、去重后 URL 计数、超限显式拒绝、无 ToolStarted、熔断同面、`tool_completed` 计数字段 Schema 先行）。
- [x] 步骤 3：机械预筛模块——候选池净化（canonical URL / host 级去重、已知失败形态、相关性粗筛）+ tier/weight 排序标签，进结构化结果（2026-08-14 闭合；`candidate_urls` 升级为预筛后保留池，`candidate_pool`/`prefilter_log` 契约先行）。
- [x] 步骤 5：输出级引用校验器与交付边界接线——`[来源]` 结构化解析、source_id/visibility/claim 上限校验、失败显式降级（覆盖 V11-IMPL-004 的 source binding 与 claim verifier；2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md`）。
- [x] 步骤 4：browser_read 范围/模式参数扩展（全文/预览/关键词提取；local_browser 第二段复用同一计数域——2026-08-14 用户裁决：计数域挂载在检索子代理 activation（复用 web_fetch per-activation 语义），主 Agent 不执行检索任务、主车道投影移除 browser_read（投影移除与子代理恢复已实施），见设计 §1.3 注；2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md`（含 2026-08-14 全面审查修复：计数消费后置、12K 严格上限、输入截断、maxLength、页脚 terms 语义、字段改名））。
- [x] 步骤 6：提示词相应缩短（计数/预筛/引用规则）与测试更新——主提示词引用纪律
  缩减为"标记格式 + verifier 交付前机械校验"，检索提示词移除候选 ≤5 软约束改指
  机械预算反馈（"候选 N/M，剩余 K"），来源加权/引用规则去冗余；prompt.rs 测试同步
  （2026-08-14 闭合，审计见 `docs/audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md`）。
- 注：DC 剩余两信号（`same_module_no_evidence` / `key_surface_unexamined`）未并入
  批次，已转 P3 遗留小项独立跟踪。

入口：[检索机械控制设计](docs/RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md) / [B-1 实施审计](docs/audits/GAP_RETRIEVAL_MECH_B1_CITATIONS_IMPL_AUDIT_2026-08-13.md) / [步骤 2 实施审计](docs/audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md) / [步骤 4 实施审计](docs/audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md)。

### P0-C CLASSICAL-EXEC-ASSISTANT（`pending`；POC 已通，v0.5 操作台模型）

- [x] 小样 1：控制台路由（服务注册表 + hassil 意图 + stdio JSON；28/28 检查通过）。
- [x] 槽位表由工作区索引动态生成（POC 闭合；生产接线复用 orz `project_doc_index` 缓存）。
- [ ] 小样 2：编辑执行器 `workspace.search_replace`（收益裁决点：编辑应用成功率 + 主模型工具轮数）。
- [ ] 小样 3：机械组合脚本模式（线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）。
- [ ] orz 内嵌集成：HA 操作台接线（薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝；POC stdio 仅原型隔离）。
- [ ] 正式组件决策门：小样全面达标后裁决；不达标即撤。

入口：[设计](docs/CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) / [POC](prototype/classical_console/README.md)。

### P0-D ORZ-COMPACTION-REDESIGN（`implemented`；S1-S5 已全部闭合，2026-08-14）

- [x] S1：D2-2 恢复超窗预估算截断——恢复加载后首请求前估算预检 + 整轮截断 + `context_recovery_truncated` 事件（完整侧车审计副本 `recovery-conversation-full.json`）。
- [x] S1：D3-1 恢复保留 marker/白名单——恢复回写过滤放行 `[前文上下文已压缩` 与白名单块，首请求可见（主车道 + 子代理 snapshot 同规则）。
- [x] S2：工具调用记录机械坍缩——动作台账行（工具/目标/指针/digest/最终回复）+ 配对纪律 + 有界尾部（请求视图，侧车保留全文）。
- [x] S3：五段模板摘要接线——机械槽位（黑板目的/计划/路径）+ 模型槽位校验（≤17K、退化拒绝 300 等效字符门（CJK 一字折算 2）、重做 ≤3 次、summary_incomplete 终止态 + fallback 机械截断）。
- [x] S3：`context_compressed` 事件 Schema v0.2 扩展（mode/reason/summary_id/digest/path/summary_incomplete/retained_rounds）+ verifier/fixtures 同步 + `.gsa/compaction/` 存档与 7 天 retention + TUI 投影。
- [x] S4：实施审计、ADR-0010 §14.10 补写、索引/BACKLOG/TODO 状态同步、设计文档状态更新。
- [x] S5：审查修复（ADR-0010 v1.14）——守卫失败重试 3 次后强制压缩并 `guard_failed` 报告；
  会话结束自动压缩（`session_end`，marker 固定进 sidecar，恢复治本）；存档写失败显式重试
  ≤3 次并 `archive_write_failed` 报告；退化守卫 300 等效字符（CJK 一字折算 2）；黑板 edit
  窗口随压缩滚动；冷却 3→2 模型轮；摘要 120s 超时；路径槽 Top-40 双上限；
  Schema/verifier/fixtures 扩展与 `$id` 修正。

入口：[压缩设计](docs/CONTEXT_COMPACTION_DESIGN_2026-08-14.md) / [ADR-0010 §14.10/§14.14](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [P0-D 实施审计](docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) / [BACKLOG 3b](docs/BACKLOG_AND_PRIORITIES.md)。

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

### ORZ-CACHE-CONTEXT-COST（`approved`；P1，2026-08-14 登记）

- [ ] 请求 header 变化留痕：模型请求 header（system+tools 摘要 + config + 原因 initial/change）变化时 journal 留痕（翻转可审计、miss 可归属）。
- [ ] 探针准确性与稳定性：误判审计（假完整/假不完整）、翻转与 header 留痕事后核对、可选后端接线同步补翻转测试。
- [ ] 单轮工具结果注入预算（默认 50K、`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 可调、按轮累计、超限拒批 + offset 续读）与提示词策略化读取（grep/结构优先、证据关键文件才全文）。

入口：[ADR-0010 §14.9](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [探针设计](docs/TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md) / [BACKLOG](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-ORIENTATION-FORCED-TEMPLATE（`approved`；P1，2026-08-14 登记）

- [ ] ADR-0010 §4.2 正文修订登记（v1.13 裁决已登记）：「注入」→「触发点暂停并填写模板」。
- [ ] 强制模板轮实现：触发点下一安全动作间隙暂停、独立无工具 checkpoint 轮、模板字段/机械校验、一次错误反馈重填 + 降级兜底。
- [ ] 缓解必做：`progress_evidence` 与 journal 证据身份存在性交叉校验；`next_action=gather_evidence` 必填缺失面。
- [ ] 事件/Schema/verifier/fixtures 同步（checkpoint 事件扩展模板/响应/校验结果字段或新增响应事件）+ 测试（触发→暂停→填表→恢复、重填、降级、主车道隔离、计数语义）。
- [ ] 实施审计、BACKLOG/TODO/索引状态同步。

入口：[设计](docs/ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md) / [ADR-0010 §14.13](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [BACKLOG 6c](docs/BACKLOG_AND_PRIORITIES.md)。

### ORZ-SESSION-CONTEXT-MONITOR（`approved`；P1，2026-08-14 登记）

- [ ] 度量接线：模型完成轮后累加 usage 输入 token（transport/loop 完成点）；usage 缺失时 journal 估算兜底。
- [ ] 阈值配置（env/TOML，默认 384K 提醒 / 500K 总结推荐）；同一阈值只触发一次。
- [ ] 最简实现：阈值到达的最后一轮模型输出末尾机械附提醒；headless/自动化仅写日志；500K 附五段模板 + 新窗口开场提示骨架。
- [ ] 测试（到达/未到达、usage 缺失兜底、headless 分支、幂等）+ 实施审计 + BACKLOG/TODO/索引状态同步。

入口：[设计](docs/SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md) / [ADR-0010 §14.13](adr/ADR-0010-fusion-runtime-and-agent-architecture.md) / [BACKLOG 6d](docs/BACKLOG_AND_PRIORITIES.md)。

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
- [ ] （可选）提示词补列 observed scope 合法枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`）——P0-B 步骤 6 复核观察登记，verifier 已机械兜底，暂不实施。
- [ ] V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
- [ ] V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留——复核并登记闭合或转 gap。
- [ ] orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`）——复核并登记闭合或转 gap。
- [ ] DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（原建议并入 P0-B，批次已闭合，独立待办）。
- [x] 前置收尾：提交当前未提交登记（995a384，2026-08-13）。

## 审计登记边界（条件触发，不占当前优先级）

- [ ] orz-host 可选后端接线（lsp / memory / 图像 / 视频 / MCP）：接线时翻转能力访问器并补翻转测试（FUS-TOOL-PROBE 边界）。
- [ ] headless 计划模式能力信号（plan 模式探针当前以交互用户信号代理，未来 headless 计划模式需独立信号）。
- [ ] 下一次真实运行捕获自然携带 23 工具分区 journals（当前 12 个为已提交 fixtures 重建）。
- [ ] B-1 后续：canonical URL/host 级去重留待预筛步骤 3；若 host/loop 拆为跨进程边界，补 `ToolResult.structured` 序列化契约。
- [ ] ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇（`TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + 只重试只读/幂等指引）——出现恢复面 400 或副作用未知证据时实施（单点修复）。
- [ ] ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫「同工具同参数」信号——出现「同参循环且输出持续变化」证据时在 stagnation guard 内加最小计数信号。

## 近期已闭合（供核对，不计入开放项）

- [x] FUS-TOOL-PROBE：P0-A 步骤 1-7 与 P0-A-2 全部闭合（ADR-0010 v1.8，23 个工作工具单一探针面）。
- [x] FUS-SOURCE-WEIGHTING-IMPL：来源加权实现闭合（机械三档 + 机器可读种子名单 + 模型加权标注）。
- [x] GAP-ENCODING-GATE：机械编码门控闭合。
- [x] GAP-ACAF-SLICE1 / SLICE2A / SLICE2B / FAILCLOSED 与 GAP-DENIAL-POLICY-REVISION：ACAF 实施切片闭合（fail-closed 生产启用仍待裁决）。
- [x] OPS-PROTOCOL 审查判定登记（裁剪方向定案；裁剪设计待产出）。
