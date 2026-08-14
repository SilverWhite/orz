# ORZ 统一待办与优先级（BACKLOG）

> 状态：living（单一待办路由）；建立：2026-08-13。
> 定位：本文件只做未闭合项召回、优先级和决策门登记；不替代 ADR、Schema、审计、索引或源码。设计裁决以 ADR-0010 / ADR-0011 和 [`CLI_PROJECT_INDEX.md`](../CLI_PROJECT_INDEX.md) 的 canonical entry 为准。
> 维护纪律：新增、关闭或调整优先级只在本文件登记；设计文档与审计的“待办/下一步”小节只保留指针或审计时点历史，不重复维护明细；索引只登记本文件的召回路由。
> 实施勾选清单：见 [`TODO.md`](../TODO.md)（派生投影，勾选状态随本文件同步；优先级、决策门与状态以本文件为准）。

## 优先级总览

| 优先级 | 含义 | 未闭合项 |
|---|---|---|
| P0 | 当前工作集：设计已冻结，裁决后立即实施 | CLASSICAL-EXEC-ASSISTANT（POC 已通） |
| P1 | 无需裁决，可与 P0 并行 | FUS-COMPONENT-REGISTER、GAP-WINDOWS-EVIDENCE、IMPL-DEEPSEEK-TRANSPORT / SEC-CREDENTIALS、ORZ-CACHE-CONTEXT-COST、ORZ-ORIENTATION-FORCED-TEMPLATE、ORZ-SESSION-CONTEXT-MONITOR |
| P2 | 生产化决策门：需用户裁决 | IMPL-CONTROL-FABRIC（fail-closed 启用、Slice 3/4）、OPS-PROTOCOL |
| P3 | 收尾 / 清理 | EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、DC 剩余信号、V11-IMPL-003/007、工作区收尾 |

- 复杂度治理判定（2026-08-13）：LIF 为高要求主项目、实验含相当程度自动运行；复杂度降低只砍冗余（OPS 平行执行层、双实现、文档仪式），保留服务 LIF 不变量的机制（journal/verifier、permission fail-closed、运行守卫、Windows 进程控制、来源证据、ACAF Slice 1/2）；ACAF Slice 3/4 暂缓，按实际自动化模式再定；不做机制×不变量清单，避免后续审查被带偏。
- DSH 借鉴复核（2026-08-14，用户裁决）：orz 自身（除成熟底座外的一切）即整体化二进制薄层，底座可较简单切换；个人开发者无插件生态，不支付子系统化复杂度。三项机制复核结论——A（Windows ACL 沙箱）挂起不立项；B（文件观察策略）收编为 `workspace.search_replace` 动作契约规则（随 CLASSICAL-EXEC-ASSISTANT 小样 2 裁决）；C（工具结果裁剪）收编为纯函数（随 COMPACTION-REDESIGN S2 或 50K 注入预算实施）；其余 DSH 层已覆盖或不适配，不引入。本复核不新增独立实施项。

## P0 — 当前工作集

> 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施；执行顺序 P0-A（工具探针）优先，P0-B（检索机械控制）紧随，P0-C（经典操作台）POC 已通后进入实施序列。

### 0. 前置收尾（提交前需用户确认）

- 已完成（995a384）：提交当前未提交登记——CLI_PROJECT_INDEX 索引更新、两份设计文档（含优先级标记）、本文件与各指针更新。

### 1. FUS-TOOL-PROBE（`implemented`；P0-A 批次 1-7 与 P0-A-2 已闭合）

- 入口：[设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施；ADR-0010 §3.5 修订采纳按批次末第 7 步登记。
- 进度（2026-08-13）：步骤 1 已完成——面 B 探针模块（orz-loop `tool_probe.rs`：两态中性判定 + 失败兜底 + 面矩阵成员与判定单测 15 项）；步骤 2 已完成——v0.2 事件升级（新 payload Schema + verifier 交叉校验 + fixture 重生成 + producer 接线 + TUI 消费面适配），13 个真实 journals 重捕并验证；步骤 3 已完成——`run_tests` 条件声明迁移（controller 删除直接 `test_runner()` 条件声明，声明改由面 B 探针快照驱动：完整才列出、不完整即移除，中性 reason `缺少测试运行器`；新增 registry 已声明但 runner 缺失/存在两条迁移语义测试）；步骤 4 已完成——列表投影接线（tool_probe 补面 A/面 C 常量与 `is_main_agent_work_tool` 判定；controller 模型可见列表 = 面A + (面B完整集 ∩ 会话声明集) + 面C + 非工作工具，仅名称；ReadOnly 下 `search_replace` 写探针不完整即移除，面 C/非工作工具不探不标；新增投影分区、交互会话、面 A/C 成员判定测试，Benchmark/ReadOnly 全量声明测试同步新语义）；步骤 3/4 全面审查清理已完成——run_tests 无 runner 竞态调用改为 dispatch 前无 ToolStarted 中性拒绝（`missing_test_runner`），补 workspace 不可读投影移除与竞态兜底测试；步骤 5 已完成——最小上一轮映射与翻转事件（`MinimalProbeMap` 仅存 `tool → 完整/不完整`、不缓存 reason、不跨 run；每个模型请求构造前重算面 B 快照并重算列表投影，翻转才发 `tool_availability_check` 事件、无变化不发；调用即探针：面 B 工具 ToolCompleted(error) 回写最小映射——run_tests 竞态拒绝/spawn 失败/ACAF fail-closed 拒绝/宿主调用错误，回写仅主/grill 车道、检索车道不写主映射（审查修复）；主/grill 车道探、检索车道不探不发；新增翻转重投影、调用失败回写恢复翻转、检索车道零污染、跨 run 重置四项集成测试）；步骤 6 已完成——兜底消息中性化改造（权限门禁拒绝消息改 `tool 'X' — 本次调用未获权限门禁放行`；连续拒绝熔断块改中文中性陈述，移除 available/refused 旧措辞；系统提示词改"工具列表由运行时按轮声明"；检索车道按复核裁决保持原设计，拒绝/宿主执行消息不脱敏；run_tests 管道错误改 unreadable；DC 最小动作改"完整错误输出"；补中性词契约测试）；步骤 7 已完成——ADR-0010 §3.5 修订登记为 v1.8（单一探针面取代 v1.5「registry 全量 + 零可用性承诺」语义，A+C→B 定档合并登记，ADR §14.8；§14.5 补复核注）；P0-A-2 已闭合——单一探针面覆盖全部 23 个工作工具（见下）。orz-loop/host/tui 测试全绿、仓库门禁 valid。
- 方向裁决（2026-08-13，定档）：**A 面与 C 面全部并入探针面（B）**——单一规则：本轮模型可见 = 机械链路完整 ∩ 会话声明集（非工作工具与检索车道走各自既有门）。A 面 7 个控制工具（`blackboard_read` / `todo_write` / `update_goal` / `enter_plan_mode` / `exit_plan_mode` / `compaction_whitelist_add` / `retrieval_disposition`）与 C 面 9 个工具（`run_terminal_cmd` / `lsp` / `memory_get` / `memory_search` / `image_gen` / `image_edit` / `image_to_video` / `reference_to_video` / `use_tool`）各自补机械链路探针（journal/工作区/goal context/会话类型/权限策略/后端配置/MCP 能力注册等），面 A/C 撤销，B 面升级为全工作工具探针面。该定档与步骤 7 ADR-0010 §3.5 修订合并登记。
- 审查裁定（2026-08-13，子代理三路审查）：步骤 4 列表投影语义定为 `面A + (面B完整集 ∩ registry 声明集) + 面C`（部分会话变体可移除 ask_user_question/search_tool，探针不得声明会话不存在的工具）；探针粒度=工作区根级机械检查、写探针 metadata-grade、交互用户信号=ACP live gateway，均已登记进设计文档。（该投影语义已被 v0.2 单一探针面取代，见上一条方向裁决。）
- 实施序列：
  1. 面 B 每工具探针实现（路径/权限/策略/runner/交互用户判定）；
  2. `tool_availability_check` Schema/fixture/verifier 升级与 producer 接线；
  3. run_tests 条件声明迁移；
  4. 列表投影接线（面 A + 面 B 完整集 + 面 C，仅名称）；
  5. 最小上一轮映射与翻转事件；
  6. 兜底消息中性化改造；
  7. ADR-0010 §3.5 修订裁决与登记。

### 1b. FUS-TOOL-PROBE v0.2 单一探针面扩展（P0-A-2，已闭合）

- 完成内容（2026-08-13）：面 A/C 撤销——`tool_probe.rs` 以单一 `WORK_TOOLS`（23 个工具）取代
  三面常量，全部工具按设计 §2.0 表各自补机械链路判定（读/写/存储/goal 上下文/plan 模式/
  检索激活/终端/lsp/memory/图像/视频/MCP 注册，全部为廉价确定性检查）；`LoopHost` 新增
  terminal/lsp/memory/image/video/MCP 六项 fail-closed 能力访问器（orz-host 覆盖终端=true，
  其余按当前 build_toolset 未接线=false）；controller 新增 goal_context_present /
  has_live_activation（待处置激活，2026-08-13 审查复核收紧为未决 pending assessment）
  探针信号；列表投影切换为 探针完整集 ∩ 会话声明集 + 非工作工具；
  `tool_availability_check` 事件 complete/incomplete 覆盖全部工作工具（Schema 描述、Python
  verifier `_WORK_TOOLS`、fixture 生成器与 12 个 v0.2 journal fixtures 同步并重算哈希链）；
  调用即探针回写覆盖全部工作工具（仍限主/grill 车道）；投影/翻转/车道隔离测试更新。
- 验证：orz-loop 236 通过 / 0 失败、orz-host 199 通过 / 0 失败、orz-tui 178 通过 / 0 失败；
  Python verifier 与相关 assurance 测试 395 通过；仓库门禁 valid、0 错误。
- 边界：orz-host 可选后端（lsp/memory/图像/视频/MCP）当前全部未接线——探针按 fail-closed
  移除这些工具；未来接线须翻转 orz-host 对应能力访问器并补翻转测试。真实 journals 为已提交
  fixtures 重建（12 个含探针事件），非新捕获运行。
- 边界（2026-08-13 审查复核）：`retrieval_disposition` 探针收紧为"未决 pending
  assessment"（Active 无 pending / continue 已决均不完整）；plan 模式探针以交互用户
  信号代理，未来 headless 计划模式会话需独立能力信号。

### 2. FUS-RETRIEVAL-MECH（`implemented`；P0-B，批次 1-6 已闭合 2026-08-14）

- 入口：[设计](RETRIEVAL_MECHANICAL_CONTROLS_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)。
- 决策门：2026-08-13 用户已裁决放行实施。
- 进度（2026-08-13）：**步骤 1（B-1 闭合）已完成**——`ToolResult` 新增
  `structured` 接缝（host 仅 web_search 填充 citations），controller 机械提取
  为证据 `candidate_urls` 并写入 `source_ledger`/`raw_source_refs`（Schema
  与 Python verifier 先行，`_verify_v02_search_candidate_pool` 校验镜像）；
  orz-host 200 / orz-loop 238 / orz-assurance 139 / orz-tui 178 通过，Python
  全量 1821 通过，仓库门禁 valid。边界：候选池暂不进模型提示词（预筛步骤负责
  展示），web_search 摘要条目仍不写 tier。审计：
  [GAP-RETRIEVAL-MECH B-1 实施审计](audits/GAP_RETRIEVAL_MECH_B1_CITATIONS_IMPL_AUDIT_2026-08-13.md)。
- 进度（2026-08-14）：**步骤 2 已完成**——web_fetch 候选机械计数门禁与计数
  反馈（`ORZ_WEB_FETCH_CANDIDATE_CAP` 用户裁决定档 8；per-activation 计数域
  随 activation 侧车持久化，经 LoopProfile 穿过子代理循环；精确字符串去重；
  未超限结果携带“候选 N/M，剩余 K”反馈 + `tool_completed` 审计字段
  candidate_count/candidate_cap（Schema 先行）；超限无 ToolStarted 拒绝 +
  连续拒绝熔断同面；缺 url/无计数域 fail-closed）；orz-loop 244 / orz-host 200 /
  orz-assurance 139 / orz-tui 178 / orz-bin 全绿，Python 全量 1828 通过，
  仓库门禁 valid（含审查修复：verifier 派发包装误报、黑板计数反馈一致、
  continue 跨派发累计测试）。审计：
  [GAP-RETRIEVAL-MECH 步骤 2 实施审计](audits/GAP_RETRIEVAL_MECH_STEP2_WEB_FETCH_CANDIDATE_COUNT_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 3 已完成**——机械预筛模块（候选池净化 + 排序：
  canonical/host 级去重、已知失败形态 bad_url/login_wall/redirect_chain 移除、
  tier/weight + 词法相关性排序；`candidate_urls` 升级为预筛后保留池，新增
  `candidate_pool` 每候选元数据与 `prefilter_log` 移除日志，Schema/verifier
  先行；配置种子 `runtime/candidate-prefilter-config-v0.1.json` +
  `ORZ_CANDIDATE_PREFILTER_CONFIG` 覆盖；orz-assurance 新模块 + orz-loop
  接线）；边界：超大页/robots 无法离线判定保留 unknown、计数域精确去重
  语义不变、模型面展示留待步骤 6。审计：
  [GAP-RETRIEVAL-MECH 步骤 3 实施审计](audits/GAP_RETRIEVAL_MECH_STEP3_CANDIDATE_PREFILTER_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 5 已完成**——输出级引用校验器与交付边界接线
  （主 Agent 最终回答在 counterexample gate 同一交付边界机械校验 `[来源: ...]`
  标记：结构化解析、ledger source_id / 主车道 path:line / URL+observed scope /
  文档身份绑定、§3.7.5 claim 上限；失败以 `[CITATION_VALIDATION_FAILED]` 机械
  降级块替代交付并 journal `citation_validation` 事件，reason codes + marker
  明细；Schema/verifier/fixtures 先行，v0.2 事件枚举 42→43；主车道补
  main_evidence 读取证据 + run_source_ledgers，TUI 投影同步；conformance 新增
  第 14 个场景并捕获 journal）；验证：orz-loop 268 / orz-assurance 151 /
  orz-tui 178 / orz-bin 42 通过，Python runtime 239 / assurance 1621 通过，
  仓库门禁 valid；既有 13 份 journals 重捕无漂移。审计：
  [GAP-RETRIEVAL-MECH 步骤 5 实施审计](audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md)。
- 进度（2026-08-14）：**步骤 6 已完成**——提示词相应缩短与测试更新：
  主 Agent 提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"
  （设计 §3.3）；检索子代理提示词移除"候选 ≤5 / never the full reference
  list"软约束，改指机械预算反馈（"候选 N/M，剩余 K"）；来源加权/引用规则
  段落去冗余；prompt.rs 测试同步更新。验证：orz-loop 281 / orz-assurance
  151+1 doctest / orz-host 207 / orz-tui 178 / orz-bin 42 通过，Python
  runtime 244 / assurance 1607+14 skipped 通过，仓库门禁 valid、0 错误。
  审计：
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md)。
- 依赖顺序：1（B-1 闭合）→ 2/3/5 → 4（闭合）→ 6。
  1. （已完成）B-1 闭合：web_search citations 结构化透传 loop；
  2. （已完成）web_fetch 计数门禁 + 计数反馈 + `ORZ_WEB_FETCH_CANDIDATE_CAP` 接线；
  3. （已完成）机械预筛模块（候选池净化 + 排序标签）与结构化结果扩展；
  4. （已完成）browser_read 范围/模式参数（全文/预览/关键词提取）工具能力扩展；
  5. （已完成）输出级引用校验器与交付边界接线；
  6. （已完成）提示词相应缩短（计数/预筛/引用规则）与测试更新。
- 批次已闭合（2026-08-14）：DC 剩余两信号（`same_module_no_evidence` /
  `key_surface_unexamined`）未并入批次，转 P3 遗留小项独立跟踪。

### 3. CLASSICAL-EXEC-ASSISTANT（POC 已通；实施中）

- 入口：[设计](CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；POC：[`prototype/classical_console/README.md`](../prototype/classical_console/README.md)。
- 定位：给主模型套一个真正的操作台（而不是工作环境）——模型面只注册工具名/动作名，助理层按主模型输出逐层定位组件并执行，不理解动作语义；注册表路由 → 契约校验 → 目标解析 → ACAF 票据 → 执行 → verifier；承接 OPS-PROTOCOL 裁剪方向；不新增平行协议、不拥有决策权、执行层全部确定性。
- 威胁模型（2026-08-13 用户裁决）：只防幻觉与注入；助理层 = 单一策略执行点（围栏合并为策略表 + taint 动作组合禁令）；动作级策略沙盒成立，OS 级执行沙盒按动作另行挂载。
- 调研结论（2026-08-13）：Rhasspy 已归档（2025-10）、Rasa 维护模式、n8n 非真开源、Windmill AGPL——均不作底座；**Home Assistant 服务模型 + hassil（Apache-2.0、活跃）为首选成熟参考**；StackStorm/Node-RED/OVOS 备选。
- 集成形态（v0.3 用户裁决）：HA 助理层与 orz 深度融合，是 orz 的一部分；薄接缝改置 orz 本体 ↔ 底座（模型后端，Grok/Codex 等），不是 orz ↔ ConsoleClient；POC 的 stdio 协议仅为原型隔离，不作为生产接缝。
- 借鉴登记（2026-08-13）：DeepSeek Harness（v0.1 预览、MIT）只借设计本身、不引入其技术栈——PTC 程序化工具调用（模型输出程序化动作脚本，操作台逐行确定性执行、失败 fail-closed）+ Profile/Bundle 动作组合（按 Benchmark/ReadOnly/标准场景加载动作集）。
- fail-closed 返回契约（2026-08-13，已落地）：调研 HA 原项目——WebSocket 成功回 `result:{context,response}`、失败回 `error:{code,message}`（message 带校验路径），无失败点/上游结果，不足；错误信封扩展为 `step`（失败点枚举 protocol/intent/registry/contract/target/execute/verify/policy）+ `code` + `message` + `upstream`（解析后真实目标/票据 id/部分输出/verifier 摘要，无则 null），模型可独立排障；POC 已实现并纳入冒烟（21/21）。
- 动作粒度与稳定性裁决（2026-08-13）：细粒度优先——粗按钮（run_terminal_cmd 什么都做）才是限制模型（参数幻觉面大）；细动作必须配套机械组合层（PTC/管道/意图）避免碎片化。负担=构建期线性成本（注册+schema+handler+探针/策略映射+测试），运行时近零；风险在边界漂移与变更连锁；护栏=动作契约按版本化 API 管理（新增优先、废弃走迁移期、参数向后兼容）、探针决定可见性、Profile/Bundle 分区、契约即测试。小样 2 仍为收益量化裁决点。
- 执行失败特殊反馈与日志可见性（v0.4，已落地）：错误信封恒带 `trace_id`；`step=execute` 失败附有界 trace 尾部；新增只读服务 `assistant.trace`（有界、读入审计）——主模型可结合日志与操作台覆盖未预录内容；POC 冒烟 28/28。
- 机械组合模型（v0.4 设计）：线性脚本模式（PTC）——步骤=注册动作实例 + `$ref` 数据引用，每步独立契约校验 + trace，任一步 fail-closed；无任意代码/隐式控制流，循环/条件暂不做；列入实施序列小样 3。
- 黑板动作栏（v0.5 用户提案，定为生产协作形态）：黑板拆三块——注册板块（助理层维护、当前轮动作投影、常驻按需读）、动作栏（模型写订单，无副作用）、结果栏（receipt+trace_id+fail-closed）；发放=机械单一出口（模型轮结束触发、消费一次、round 防重）；模型面不再出现执行/发送工具；stdio 仍为原型隔离。
- 进度（2026-08-13）：小样 1（控制台路由小样）已跑通——`prototype/classical_console/`（HA 式服务注册表 + hassil 意图 + stdio JSON 协议），服务模式（`workspace.read_file` / `workspace.list_dir` / `workspace.index` / `assistant.trace`）+ 意图模式（en/zh，槽位表由工作区索引动态生成），含 fail-closed 契约与执行日志，28/28 检查通过。
- 实施序列：
  1. 槽位表由工作区索引动态生成（POC 已闭合；生产接线复用 orz `project_doc_index` 缓存）；
  2. 编辑执行器 `workspace.search_replace`（小样 2，收益裁决点：编辑应用成功率 + 主模型工具轮数）；
  3. 机械组合脚本模式（小样 3：线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）；
  4. orz 内嵌集成（HA 操作台作为 orz 组件接线；薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝，POC stdio 仅原型隔离）；
  5. 小样全面达标后裁决正式组件（决策门）；不达标即撤。

### 3b. ORZ-COMPACTION-REDESIGN（`implemented`；P0，S1-S4 已闭合 2026-08-14）

- 入口：[设计](CONTEXT_COMPACTION_DESIGN_2026-08-14.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；
  权威：ADR-0010 §3.6 / §14.10（v1.10）；实施审计：
  [GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md)。
- 决策门：2026-08-14 用户裁决定稿——有效窗口 384K、160K 普通触发 / 200K 兜底、工具调用记录每轮
  机械坍缩（零模型调用）、五段模板摘要（≤17K 字符、derived_unverified、冷却 3 步、摘要链只进
  审计、滚动回查 marker）；推翻 2026-08-08 零模型摘要与仅最终答案间隙裁决；2026-08-14 用户
  放行实施。
- 实施切片：
  1. S1：D2-2 恢复超窗预估算截断 + D3-1 marker/白名单恢复保留（含测试）——**已闭合**；
  2. S2：工具调用记录机械坍缩（动作台账行 + 配对纪律 + 指针完整性 + 测试）——**已闭合**；
  3. S3：五段模板摘要接线（orz-compaction 退化守卫复用、会话模型覆盖、17K 校验、重做/终止态、
     `context_compressed` 事件 Schema v0.2 + verifier/fixtures）——**已闭合**；
  4. S4：实施审计、ADR-0010 §14.10 补写、索引/BACKLOG/TODO 同步、设计文档状态更新——**已闭合**。
- 进度（2026-08-14）：S1-S4 全部闭合——恢复预检（`context_recovery_truncated` 事件 +
  完整侧车审计副本）、marker/白名单恢复保留、动作台账请求视图坍缩、五段模板摘要
  （160K/200K/3 轮/5K/0.6、重做 ≤3、summary_incomplete 终止态 + fallback 机械截断、
  `.gsa/compaction/` 存档 + 7 天 retention、TUI 投影）；orz-loop 296 / orz-host 208 /
  orz-tui 178 通过，工作区全量 exit 0，Python 事件校验 164 通过，仓库门禁 valid。

## P1 — 可并行审计 / 证据

### 4. FUS-COMPONENT-REGISTER（`partial`）

- 开放内容：65 组件全 `audit_required`，逐 crate 采用审计未开始（V11-IMPL-008）；不得从 crate 名/编译推断采用档位。
- 入口：[register](../upstream/fusion-component-register-v0.1.yaml)；[V1.1 复核](audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。

### 5. GAP-WINDOWS-EVIDENCE（`partial`）

- 开放内容：ORZ-WIN-PROC-001/002/003 仍为 `candidate`；需真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
- 入口：[incidents](incidents/windows/README.md)；[cases](cases/windows/README.md)。

### 6. IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- 开放内容：transport/retry/thinking 按主/子代理同构约束复核；DeepSeek live 通道与 Windows 实机晋级证据（ADR-0010 §11.7）。
- 入口：[DEEPSEEK_ADAPTER_CONTRACT](../architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)；[ADR-0007](../adr/ADR-0007-transport-retry-policy.md)；[ADR-0006](../adr/ADR-0006-credential-target-registry.md)。

### 6b. ORZ-CACHE-CONTEXT-COST（`approved`；P1，2026-08-14 登记）

- 定位：缓存与上下文成本收敛——保持 v1.8 探针可见性机制不动（工具集变化=真实状态
  变化，接受前缀 miss，第二轮自动恢复）；否决 per-window 探测 / 预热轮 / 工具层后置
  渲染 / 工具层 1K 压缩；短期仅 DeepSeek OpenAI 兼容面。
- 三项待实施：
  1. 请求 header 变化留痕——模型请求 header（system+tools 摘要 + config + 原因
     initial/change）变化时 journal 留痕；翻转可审计、miss 可归属。
  2. 探针准确性与稳定性——误判审计（假完整/假不完整）、翻转与 header 留痕事后核对、
     可选后端接线同步补翻转测试。
  3. 单轮工具结果注入预算 + 策略化读取——`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 默认
     50K、按模型轮累计、超限拒批并提示 offset 续读；提示词 grep/结构优先、证据关键
     文件才全文。
- 入口：[ADR-0010 §14.9](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [探针设计](TOOL_AVAILABILITY_PROBE_DESIGN_2026-08-13.md)；[TODO](../TODO.md)。

### 6c. ORZ-ORIENTATION-FORCED-TEMPLATE（`approved`；P1，2026-08-14 登记）

- 定位：中立问询升级为强制模板轮——触发点下一安全动作间隙明确暂停，模型填写问询
  模板后才恢复动作（Orientation 与 DC 两族共用，主车道）；目的=拉回注意力、防跑偏
  与钻牛角尖（强制表达、不验证诚实）；缓解必做（`progress_evidence` 存在性交叉
  校验、`gather_evidence` 必填缺失面）。
- 实施前置：ADR-0010 §4.2 正文修订（v1.13 已登记裁决）；事件/Schema/verifier/
  fixtures；测试；实施审计与索引同步。
- 入口：[设计](ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

### 6d. ORZ-SESSION-CONTEXT-MONITOR（`approved`；P1，2026-08-14 登记）

- 定位：会话累计上下文监测——度量=会话累计模型可见输入 token（usage 优先、journal
  估算兜底）；384K 机械提醒、500K 机械总结推荐（可配）；最简实现=阈值到达的最后一轮
  模型输出末尾机械附提醒；headless 仅日志；TUI/journal 事件为 beta 前可选；与压缩独立。
- 实施前置：度量接线、阈值配置、机械附言注入点、测试、实施审计与索引同步。
- 入口：[设计](SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

## P2 — 生产化决策门

### 7. IMPL-CONTROL-FABRIC（`partial`）

- 决策门：用户裁决 fail-closed 生产启用（前置：探针矩阵 + Slice 2B §6 核查清单 ①-⑪，⑦⑨⑩⑪ 仍登记）。
- 剩余核查项：web_search 票化形态；host 侧执行参数绑定面；执行面与票据绑定面错位；network 重定向不重新票据。
- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
- Slice 4：Windows Sandbox backend（D-11）。
- 可选：检索车道 web_fetch activation 绑定接线；conformance capture 票据场景；normalize_lexical 单源化；ACP 会话路径接 ACAF。
- 入口：[ADR-0011](../adr/ADR-0011-authenticated-control-and-action-fabric.md)；[fail-closed 审计](audits/GAP_ACAF_SLICE2_FAILCLOSED_IMPL_AUDIT_2026-08-13.md)。

### 8. OPS-PROTOCOL（`pending`）

- 开放内容：v0.1 协议、Schema、Python/PowerShell 执行器已就位；生产接线待裁决（先验票，再由协议执行器执行）。
- 审查判定（2026-08-13，用户无异议）：平行执行层过重，不按原样生产接线。收敛方向——保留“删除安全”（回收站 + 缓存机械分类 + 容量 fail-closed）为 host-owned 工具；跨环境桥接保留为内部执行能力，不向模型暴露 op 信封；双执行器收敛为单一参考实现，生产走 Rust 工具面。裁剪设计待产出后登记。
- 入口：[协议](../protocol/structured-operation-protocol-v0.1.md)。

## P3 — 收尾 / 清理

### 9. EVIDENCE-LOCAL-BROWSER（`partial`）

- 开放内容：Python 路径（retrieval_workflow / evidence_store / pdf_evidence）按 ADR-0010 重新符合性审查或退役。
- 入口：[LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE](../存档/architecture/pre-adr-0010/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md)；[GAP_PDF_EVIDENCE_IMPL_AUDIT](audits/GAP_PDF_EVIDENCE_IMPL_AUDIT_2026-08-11.md)。

### 10. GATE-CHAIN（`partial`）

- 开放内容：分层 gate 链与融合 runtime 的最终接线随各切片审计复核（不单开大项）。
- 入口：[assurance](../assurance/)；[orz-assurance](../orz/crates/orz-assurance/)。

### 11. 遗留小项

- DC 硬信号 4/6：`same_module_no_evidence` / `key_surface_unexamined` 接线（建议并入 P0/检索机械控制批次）。
- prompt observed-scope 枚举补列（可选优化，P0-B 步骤 6 复核观察登记）：主提示词/检索提示词未列出合法 scope 枚举（`full_text_observed` / `partial_text_observed` / `metadata_only`），模型可能先踩一次 verifier 拒绝（`url_missing_observed_scope`）再修正；verifier 机械兜底已覆盖，暂不实施。
- V11-IMPL-003：Global Review receipt 与真正审查结论严格分离——复核并登记闭合或转 gap。
- V11-IMPL-007：Toolbar/run-history 数据源统一到 ORZ session ownership、旧路径残留检查——复核并登记闭合或转 gap。
- orz-host 既有 flaky（`approval_allow_persists_for_identical_bash`，顺序/负载相关、与本批无关）——复核并登记闭合或转 gap。
- 工作区收尾：见 P0 前置收尾。

## 条件触发（不占当前优先级）

- ORZ-RECOVERY-TOOL-OUTCOME：崩溃恢复工具结果词汇——恢复中断轮次补
  `TOOL_NOT_STARTED` / `TOOL_OUTCOME_UNKNOWN` 合成 Tool 消息 + "只重试只读/幂等
  操作、验证副作用或询问"指引。出现恢复面 400 或副作用未知证据时实施（单点修复，
  不建子系统）。入口：[调研附录候选 1](DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md)。
- ORZ-STAGNATION-TOOL-SIGNAL：停滞守卫补「同工具同参数」信号——出现「同参循环且
  输出持续变化」的具体证据时，在 stagnation guard 内加最小计数信号（同一工具连续
  N 次调用），不复刻 reminder 链。入口：[调研附录候选 2](DEEPSEEK_HARNESS_BORROW_RESEARCH_2026-08-14.md)。

## 变更记录

- 2026-08-14：P0-D 压缩机制实施闭合登记（用户放行）——S1（D2-2 恢复预检截断 +
  `context_recovery_truncated` 事件 + D3-1 marker/白名单恢复保留）、S2（动作台账机械坍缩，
  请求视图）、S3（五段模板摘要接线：160K/200K/3 轮/5K/0.6、重做 ≤3、summary_incomplete
  终止态 + fallback 机械截断、`.gsa/compaction/` 存档 + 7 天 retention、
  `context_compressed` v0.2 payload + verifier/fixtures、TUI 投影）、S4（审计 + ADR-0010
  §14.10 补写 + 索引/TODO/设计文档同步）；FUS-COMPACTION-REDESIGN 转 `implemented`；
  实施审计 `docs/audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md`。
- 2026-08-14：DSH 借鉴复核与两项设计确认登记——ADR-0010 v1.13（中立问询强制模板轮、
  会话累计上下文监测、崩溃恢复工具结果词汇条件项、DSH 借鉴复核结论）；新增设计文档
  `ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md` 与
  `SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md`；P1 登记 6c/6d；条件触发两候选；
  索引新增 FUS-ORIENTATION-FORCED-TEMPLATE / FUS-SESSION-CONTEXT-MONITOR /
  FUS-RECOVERY-TOOL-OUTCOME / FUS-DSH-BORROW-REVIEW。
- 2026-08-14：P0-B 步骤 6 全面复核补记——索引 GAP-SOURCE-WEIGHTING-IMPL
  条目补"候选 ≤5 已机械取代"注记（P3 已处理）；审查观察登记：提示词未列
  observed scope 合法枚举（P3 可选优化，暂不实施）。详见
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md) §7。
- 2026-08-14：P0-B 步骤 6 闭合登记——提示词相应缩短与测试更新实施完成
  （主提示词引用纪律缩减为"标记格式 + verifier 交付前机械校验"、检索提示词
  移除候选 ≤5 软约束改指机械预算反馈、来源加权/引用规则去冗余、prompt.rs
  测试同步）；P0-B 批次 1-6 全部闭合，FUS-RETRIEVAL-MECH 转 `implemented`，
  DC 剩余两信号转 P3 独立跟踪。详见
  [GAP-RETRIEVAL-MECH 步骤 6 实施审计](audits/GAP_RETRIEVAL_MECH_STEP6_PROMPT_SHORTENING_IMPL_AUDIT_2026-08-14.md)。
- 2026-08-14：压缩机制重设计定稿登记——ADR-0010 v1.10（§3.6 修订 + §14.10 裁决索引）、
  设计文档 `CONTEXT_COMPACTION_DESIGN_2026-08-14.md`、索引 FUS-COMPACTION-REDESIGN pending；
  参数=384K 有效窗口 / 160K 触发 / 200K 兜底 / 五段模板 17K / 冷却 3 步；推翻零模型摘要与
  仅最终答案间隙裁决；D2-2/D3-1 定为实施前置（S1）；实施待用户放行。
- 2026-08-14：P0-B 步骤 5 复核修复批次——审查发现的全部可处理问题已处理：
  `SRC-###` 改 run 级唯一分配（orientation-fire-run 重捕，5 commit →
  `SRC-001..005`）、URL 归一化复用共享 canonical 化、主车道 web 证据接入
  URL 绑定、全角冒号变体解析 + 围栏/行内代码块字面标记跳过、文档 `§`
  锚点不再参与 claim 上限、fixture message_block 对齐 producer、审计
  数字修正（orz-bin 42/14 ignored）、行界 TOCTOU 显式登记；验证重跑全绿
  （orz-loop 275 / orz-assurance 152 / orz-tui 178 / orz-bin 42、Python
  runtime 239 / assurance 1607+14 skipped、仓库门禁 valid）。详见
  [GAP-RETRIEVAL-MECH 步骤 5 实施审计](audits/GAP_RETRIEVAL_MECH_STEP5_CITATION_VALIDATION_IMPL_AUDIT_2026-08-14.md) §7。
- 2026-08-14：P0-B 步骤 5 闭合登记——输出级引用校验器与交付边界接线实施
  完成（`[来源: ...]` 结构化解析 + ledger/path:line/URL/文档绑定 + §3.7.5
  上限 + 机械降级块 + `citation_validation` 事件 Schema/verifier/fixtures
  先行、TUI 投影、conformance 第 14 场景；主车道补 main_evidence 与
  run_source_ledgers）；orz-loop 268 / orz-assurance 151 / orz-tui 178 /
  orz-bin 42、Python runtime 239 / assurance 1621 通过，仓库门禁 valid；
  批次下一步为步骤 4（browser_read 范围/模式参数扩展，前置裁决已登记）。
- 2026-08-14：P0-B 步骤 4 前置裁决登记——用户裁决：browser_read 第二段计数域
  挂载在检索子代理 activation（复用 web_fetch per-activation 语义：activation
  累计、去重 URL 计数、continue 重入不重置、关闭清零）；主 Agent 不执行检索
  任务——主车道模型可见投影移除 browser_read，子代理投影从 host registry
  恢复（实现 + 单测，设计 §1.3 注更新）；步骤 4（browser_read 范围/模式参数
  扩展）按此实施。
- 2026-08-14：P0-B 步骤 4 闭合登记——browser_read 范围/模式参数扩展与第二段
  计数域复用实施完成（mode=full/preview/keywords + keywords 数组契约，
  preview 4K / keywords 16/64/3/160/12K 常量；browser_read 与 web_fetch 共用
  同一 activation 计数域与 cap，拒绝码 browser_read_candidate_*，
  tool_completed 计数字段覆盖 browser_read；证据按 mode 降级；Schema/
  verifier 先行，`_verify_v02_candidate_count` 覆盖两家族；conformance 14
  场景重捕，local-browser-read 携带计数字段；ACAF e2e browser_read 场景
  迁移到子代理车道，主车道直呼 fail-closed）；orz-host 205 / orz-loop 280 /
  orz-assurance 151 / orz-tui 178 / orz-bin 42、Python runtime 244 /
  assurance 1607+14 skipped、仓库门禁 valid；批次下一步为步骤 6（提示词
  相应缩短与测试更新）。详见
  [GAP-RETRIEVAL-MECH 步骤 4 实施审计](audits/GAP_RETRIEVAL_MECH_STEP4_BROWSER_READ_MODES_IMPL_AUDIT_2026-08-14.md)。
- 2026-08-14：P0-B 步骤 4 全面审查修复登记——候选门禁改为“决策先行、消费后置”
  （`candidate_gate` 只决策，`commit_candidate` 在权限/ACAF 票据通过后、
  ToolStarted 前提交；被权限/票据拒绝的调用不消耗预算、拒绝事件不携带计数）；
  keywords 摘录正文严格 ≤12K（分隔符/省略号计入）、提取输入按 100K 截断并打
  “input capped”页脚、工具定义补 `maxLength=64`、页脚 terms 改为实际输出词数；
  `ActivationState.web_fetch_candidates` 改名 `candidate_urls`；候选拒绝事件
  仅在车道内携带 target；新增门禁/提交拆分单测与 e2e“拒绝不消耗→重试计数=1”
  断言。登记：ADR-0010 §14.11、步骤 2/4 审计、设计 §1.1/§1.3 注。复核后：
  orz-host 207 / orz-loop 281 / orz-bin 42、Python runtime + assurance
  1851+14 skipped、仓库门禁 valid。
- 2026-08-14：ORZ-CACHE-CONTEXT-COST 登记（P1）——缓存与上下文成本收敛三项
  （请求 header 留痕、探针准确性、单轮注入预算 + 策略化读取），ADR-0010 v1.9、
  探针设计 §11 同步；否决方向一并登记（per-window 探测、预热轮、工具层后置渲染、
  工具层 1K 压缩）。
- 2026-08-14：P0-B 步骤 3 审查修复——verifier 允许“全净化空池”（空保留池
  须有 prefilter_log 移除记录）、`redirect_query_keys` 默认收窄（移除
  url/next/goto/target/continue）、redirect pattern 改 host 边界匹配、
  scheme-less 带端口引用解析回退、clippy 文档 lint 与审计计数口径修正；
  orz-assurance 151 / orz-loop 247 / Python 全量 1840 通过，门禁 valid。
- 2026-08-14：P0-B 步骤 3 闭合登记——机械预筛模块实施完成（canonical/host
  级去重、bad_url/login_wall/redirect_chain 移除、tier/weight + 词法相关性
  排序；`candidate_urls` 升级为预筛后保留池，`candidate_pool` +
  `prefilter_log` Schema/verifier 先行；配置种子 JSON + env 覆盖）；orz-assurance
  149 / orz-loop 246 / orz-host 200 / orz-tui 178 / orz-bin 全绿，Python 全量
  1838 通过，仓库门禁 valid；批次下一步为步骤 5（输出级引用校验器）。
- 2026-08-14：P0-B 步骤 2 闭合登记——web_fetch 候选机械计数门禁与计数反馈
  实施完成（`ORZ_WEB_FETCH_CANDIDATE_CAP` 定档 8、per-activation 计数域 +
  侧车持久化、无 ToolStarted 拒绝 + 熔断同面、`tool_completed` 计数字段
  Schema 先行、verifier 规则 + 9 测试、Rust +6 测试（含 continue 跨派发累计）
  与快照扩展）；审查修复——verifier 排除派发包装误报、黑板 exec 镜像与
  对话消息一致、cap-exceeded 缺字段加固、browser_read 计数域挂载面注记；
  批次下一步为步骤 3（机械预筛）。
- 2026-08-14：新增 AUTH-TODO 路由——建立根目录 [`TODO.md`](../TODO.md) 实施勾选清单（派生自本文件未闭合项），本文件仍为优先级/决策门权威。
- 2026-08-14：文档审查复核——FUS-TOOL-PROBE 小节头部由“实施中”改为已闭合（与索引 `implemented` 对齐）；CLASSICAL-EXEC-ASSISTANT 设计文档状态头、投影入口与 README 冻结版本表述联动修正。
- 2026-08-14：P0-B 步骤 1 审查闭环处理——旧来源加权审计 B-1/D-2/D-4 边界补
  闭合/取代注记、设计文档范围措辞更新、B-1 审计补进程内接缝与精确去重边界、
  verifier 补 ref 反向镜像负例测试 2 条（Python 全量 1821 通过）。
- 2026-08-13：P0-B 步骤 1（B-1 闭合）完成登记——web_search citations 结构化
  透传进 loop（ToolResult.structured 接缝 + 证据 candidate_urls + Schema/
  verifier 先行）；orz-loop/host/assurance/tui 测试全绿、Python 全量 1821 通过、
  仓库门禁 valid；批次下一步为步骤 2（web_fetch 计数门禁）。
- 2026-08-13：P0-A-2 闭合登记——v0.2 单一探针面扩展实施完成（23 个工作工具统一机械探针、面 A/C 撤销、投影切换为探针完整集∩声明集、`tool_availability_check` 事件/verifier/fixtures 同步、`LoopHost` 六项能力访问器接线）；orz-loop 236 / orz-host 199 / orz-tui 178 通过，Python verifier 与相关测试 395 通过，仓库门禁 valid；边界=orz-host 可选后端（lsp/memory/图像/视频/MCP）未接线，接线时翻转能力访问器。
- 2026-08-13：P0-A 步骤 7 完成登记——ADR-0010 v1.8 §3.5 修订登记（v0.2 单一探针面取代 v1.5「registry 全量 + 零可用性承诺」；A+C→B 定档合并登记，ADR §14.5 补复核注）；批次步骤 1-7 全部闭合；登记实现差距——代码仍为 v0.1 三面语义，v0.2 单一探针面扩展实施（P0-A-2）待续。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.5 两点获用户确认——注册板块常驻但内容按需读取（防上下文膨胀）；单轮一单先行（防并发写单竞态，反馈闭环驱动）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.5 登记——黑板动作栏定为生产协作形态（注册板块/动作栏/结果栏；模型面=读板块+写订单，发放=机械单一出口；写订单无副作用，执行幻觉只能污染订单）；发放语义与配合规则登记。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.4 登记——执行失败特殊反馈（错误信封恒带 trace_id、step=execute 附有界 trace 尾部）+ 模型可见执行日志（只读 `assistant.trace` 服务）落地，POC 冒烟 28/28；机械组合模型（PTC 线性脚本模式）设计登记，列入实施序列小样 3。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 动作粒度裁决登记——细粒度优先（粗按钮才是限制模型），细动作必须配套机械组合层（PTC/管道/意图）；负担评估=构建期线性、运行时近零，风险在边界漂移与变更连锁；护栏=版本化动作契约 + 探针可见性 + Profile/Bundle 分区 + 契约即测试。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT fail-closed 返回契约定义并落地——调研 HA 原项目（成功 `result:{context,response}`；失败 `error:{code,message}`，message 带校验路径，无失败点/上游结果）；错误信封扩展为 step/code/message/upstream 四键；POC 冒烟升至 21/21。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT v0.3 登记——集成形态改为 HA 助理层与 orz 深度融合（HA 是 orz 的一部分），薄接缝改置 orz 本体 ↔ 底座（Grok/Codex 等模型后端），POC stdio 协议仅原型隔离；登记 DeepSeek Harness 借鉴（PTC 程序化工具调用 + Profile/Bundle 动作组合，只借设计不引栈）；实施序列第 3 步改为 orz 内嵌集成。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 升入 P0（P0-C）——小样 1 跑通（20/20）、槽位表由工作区索引动态生成闭合（POC，生产复用 `project_doc_index` 缓存）；待办重排为 槽位表 → 编辑执行器 → orz 薄适配 → 决策门。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 小样 1 跑通登记——`prototype/classical_console/` 服务注册表 + hassil 意图第一条通路，17/17 检查通过；决策门保持（控制台路由小样达标后裁决正式组件）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 威胁模型与策略执行点补充登记——只防幻觉与注入；助理层 = 单一策略执行点（围栏合并为策略表 + taint 组合禁令）；沙盒分层（策略层成立、OS 层按动作挂载）。
- 2026-08-13：CLASSICAL-EXEC-ASSISTANT 设计草案登记——新建设计文档（v0.1）；同日升 v0.2 操作台模型（不需要理解层、模型面只注册名称），P2 探索项；决策门改为控制台路由小样，通过后再裁决正式组件。
- 2026-08-13：本轮判定登记——A+C→B 单一探针面定档；复杂度治理判定（LIF 不变量优先，砍冗余不砍不变量，ACAF Slice 3/4 暂缓，不做机制×不变量清单）。
- 2026-08-13：OPS-PROTOCOL 审查判定登记——用户无异议，平行执行层裁剪方向定案（保留删除安全为 host-owned 工具、桥接内部化、执行器单一化）；裁剪设计待产出。
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
