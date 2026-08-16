# ORZ 统一待办与优先级（BACKLOG）

> 状态：living（单一待办路由）；建立：2026-08-13。
> 定位：本文件只做未闭合项召回、优先级和决策门登记；不替代 ADR、Schema、审计、索引或源码。设计裁决以 ADR-0010 / ADR-0011 和 [`CLI_PROJECT_INDEX.md`](../CLI_PROJECT_INDEX.md) 的 canonical entry 为准。
> 维护纪律：新增、关闭或调整优先级只在本文件登记；设计文档与审计的“待办/下一步”小节只保留指针或审计时点历史，不重复维护明细；索引只登记本文件的召回路由。
> 实施勾选清单：见 [`TODO.md`](../TODO.md)（派生投影，勾选状态随本文件同步；优先级、决策门与状态以本文件为准）。

## 优先级总览

| 优先级 | 含义 | 未闭合项 |
|---|---|---|
| P0 | 当前工作集：设计已冻结，裁决后立即实施 | 评测冒烟暴露问题（P0-E 六项：ACAF 容器供应实施、plan_write 提示词强化、actions 形状校验、计划视图步骤 ID 渲染、grep 侦查纪律、发放前拒绝入事件面；2026-08-17 最优先，见 0a）；CLASSICAL-EXEC-ASSISTANT（生产组件，2026-08-16 用户裁决转正式）；PLAN-FIRST-BLACKBOARD（阶段 A/B/C 全部闭合 2026-08-16） |
| P1 | 无需裁决，可与 P0 并行 | FUS-COMPONENT-REGISTER、GAP-WINDOWS-EVIDENCE、IMPL-DEEPSEEK-TRANSPORT / SEC-CREDENTIALS、ORZ-CACHE-CONTEXT-COST、ORZ-SESSION-CONTEXT-MONITOR |
| P2 | 生产化决策门：需用户裁决 | IMPL-CONTROL-FABRIC（fail-closed 启用、Slice 3/4）、OPS-PROTOCOL |
| P3 | 收尾 / 清理 | EVIDENCE-LOCAL-BROWSER、GATE-CHAIN、DC 剩余信号、V11-IMPL-003/007、工作区收尾 |

- 复杂度治理判定（2026-08-13）：LIF 为高要求主项目、实验含相当程度自动运行；复杂度降低只砍冗余（OPS 平行执行层、双实现、文档仪式），保留服务 LIF 不变量的机制（journal/verifier、permission fail-closed、运行守卫、Windows 进程控制、来源证据、ACAF Slice 1/2）；ACAF Slice 3/4 暂缓，按实际自动化模式再定；不做机制×不变量清单，避免后续审查被带偏。
- DSH 借鉴复核（2026-08-14，用户裁决）：orz 自身（除成熟底座外的一切）即整体化二进制薄层，底座可较简单切换；个人开发者无插件生态，不支付子系统化复杂度。三项机制复核结论——A（Windows ACL 沙箱）挂起不立项；B（文件观察策略）收编为 `workspace.search_replace` 动作契约规则（随 CLASSICAL-EXEC-ASSISTANT 小样 2 裁决）；C（工具结果裁剪）收编为纯函数（随 COMPACTION-REDESIGN S2 或 50K 注入预算实施）；其余 DSH 层已覆盖或不适配，不引入。本复核不新增独立实施项。

## P0 — 当前工作集

> 2026-08-13：P0 决策登记——FUS-TOOL-PROBE、FUS-RETRIEVAL-MECH 经用户裁决放行实施；执行顺序 P0-A（工具探针）优先，P0-B（检索机械控制）紧随，P0-C（经典操作台）POC 已通后进入实施序列。

### 0a. 评测冒烟暴露问题（最优先；2026-08-17 登记，用户将在新窗口处理）

> 来源：正式跑分前最难错题试跑——make-doom-for-mips（TB2 2.0，deepseek-v4-flash，
> plan-first + console 默认 + headless，2026-08-17 01:05–01:12）。证据：
> `D:\tb-eval\jobs\2026-08-17__01-05-31` / `2026-08-17__01-07-59`（ACAF 拒启）、
> `D:\tb-eval\jobs\2026-08-17__01-09-43`（工具名 400）；journal
> `D:\tb-eval\gsa-volumes\b4-900s\a5da4937-2775-47ff-8202-98bf03a74780\runs\RUN-CLI-6a81eeed\events.jsonl`
> （plan-first 首轮→计划接受→console 面→run_failed 全链）。处理窗口=新 Codex 窗口。

- **GAP-ACAF-HARNESS-PASSTHROUGH**（P0，2026-08-17）：ACAF fail-closed 生产默认强制后，
  TB2 harbor 适配器（`tb_agents/orz.py`）只向任务容器转发固定环境变量集合，签发器配置
  （`ORZ_ACAF_MANIFEST`/`ORZ_ACAF_KEYSTORE`/`ORZ_ACAF_FAIL_CLOSED`）无法进入容器，
  `orz --real` 启动即拒（`assurance invariant: ACAF fail-closed is enabled but no signer
  client is configured`）。本轮已做临时解阻：`D:\tb-eval\.env` 加 `ORZ_ACAF_FAIL_CLOSED=0`
  （影子模式）+ 适配器增该变量透传。**2026-08-17 用户裁决：跑分保持 ACAF 强制开启**——
  容器内供应 manifest/keystore/signer（参考 [`scripts/orz_acaf_run.ps1`](../scripts/orz_acaf_run.ps1)
  供应链），不接受影子模式；实施=适配器透传签发器配置 + 任务容器挂载/供应 + 移除
  `D:\tb-eval\.env` 的 `ORZ_ACAF_FAIL_CLOSED=0` 覆盖 + 冒烟验证 `orz --real` 带签发器启动。
- **GAP-CONSOLE-TOOLNAME-PATTERN**（P0，2026-08-17；**已闭合 2026-08-17**）：console 默认面三个工具名含点号
  （`blackboard.action_write`/`console.step_done`/`console.return_to_console`），违反
  OpenAI 兼容工具名模式 `^[a-zA-Z0-9_-]+$`；计划落板后下一轮请求 400（`invalid_request_error`）
  → `run_failed`。FakeProvider 不校验工具名，orz-loop 433 单测未暴露。修复=改名下划线
  （`blackboard_action_write`/`console_step_done`/`console_return_to_console`），同步约
  91 处 Rust、Python verifier 交叉校验（`run_event_journal_validation.py` 中
  `payload.get("tool") == "blackboard.action_write"` 等）、schema 注释、设计文档；改后
  重建 Linux 二进制并重跑。闭合证据：orz 子模块 0304b23（11 文件 91 处 + fmt 收口）；
  orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2 通过；
  clippy 无新增告警；manifest 重生成 1401 条目、仓库门禁 valid；Linux musl 重建后冒烟
  重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`（30m21s 跑满 1740s 预算、
  `run_invalidated{wallclock}` 正常收尾，对比旧运行 400 即死）。
- 冒烟重跑（`D:\tb-eval\jobs\2026-08-17__03-48-57`，工具名修复后）任务结果 reward 0.0——
  机制层面通过（无 400/无异常/跑满 1740s 预算），任务层面未完成（无 ELF/无帧）。失败原因
  定位：① **步骤门模型面缺口**——`blackboard_read section=plan` 只渲染
  `[status] goal (actions: N; evidence: M)`，不渲染步骤 `id`；步骤门又要求订单
  `step_id` 精确绑定，模型只能猜测（轨迹 13/18/23 步自述 "the plan view strips them /
  my guessed step_id values get rejected"），导致大量读板/计划重写轮次（4 次 plan_write）；
  ② grep 侦查低效——模型用不存在的目标字符串（`doomgeneric_mips|frame\.bmp`）grep 全树，
  空结果被过度泛化为「/app 无 C 源码」，一度错误转向；③ 预算耗尽于侦查/步骤门摩擦，
  未及完成 ELF 构建（vm.js 契约已正确读出，最终发起 search_replace 但未闭环）。
- **P0-E 下一步实施项**（2026-08-17 对齐确认）：
  1. GAP-ACAF-HARNESS-PASSTHROUGH 实施（用户裁决：跑分保持 ACAF 强制，容器内供应）；
  2. plan_write 提示词/示例强化（P1 观察①：首次模型把计划序列化为 JSON 字符串被拒
     `missing_required_field: plan` → refill，重填对象后通过）；
  3. `steps[].actions` 形状校验收紧（P1 观察②：实证审计空/宽松形状并补探针测试）；
  4. **计划视图渲染步骤 ID**（新发现，步骤门模型面闭环：`section=plan` 补 `step.id`，
     模型无需猜测；ADR-0010 §14.21 登记）；
  5. **grep 侦查纪律与空结果解读**（新观察，用户 2026-08-17 确认一并处理）：提示词/
     计划框架补侦查纪律（先 list_dir 建清单、pattern 用实际存在的字符串、空结果≠无
     文件）+ 注册板块 grep 参数提示补空结果语义 + 回归验证（重跑 round 数/计划重写
     次数下降）；
  6. **订单发放前拒绝入事件面**（新观察，用户 2026-08-17 指示处理）：冒烟重跑中
     ORD-000011（workspace.run_tests，`arguments:{}`）写入后发放前被拒，失败只进结果栏
     receipt + TraceStore（`consume_console_order` 不写 journal 事件），事后核对看不到
     拒绝码。目标=发放前拒绝（order_stale / step_not_done / budget_insufficient /
     registry / contract / target / ACAF / policy / mode 门）统一入 v0.2 事件面（新增
     `console_order_rejected`：order_id / step / phase / code / reason / round /
     plan_epoch / run_id），Schema/verifier/fixtures 先行，结果栏 receipt 保留。

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
  2026-08-15 全面检查修复：策略拒绝（权限/ACAF/taint/模式门）由执行器适配层
  归一化为 `step=policy` + `code=policy_denied`；响应契约强制必填（注册时校验
  并缓存 schema，任何输出过机械验证）；`exit_code=Some(0)` 成功契约。
  2026-08-15 P1-2 定案补登记：拒绝路径（权限/ACAF/模式门；taint 预留）返回
  结构化信号 `ToolResult.policy_denial = {source, code, reason}`，controller
  删除「稳定输出前缀」字符串判定（`console_policy_refusal` 退役）；ToolCompleted
  增可选 `policy_denial`（Schema/verifier/fixtures 先行）；实施随 S3 前置。
- 动作粒度与稳定性裁决（2026-08-13）：细粒度优先——粗按钮（run_terminal_cmd 什么都做）才是限制模型（参数幻觉面大）；细动作必须配套机械组合层（PTC/管道/意图）避免碎片化。负担=构建期线性成本（注册+schema+handler+探针/策略映射+测试），运行时近零；风险在边界漂移与变更连锁；护栏=动作契约按版本化 API 管理（新增优先、废弃走迁移期、参数向后兼容）、探针决定可见性、Profile/Bundle 分区、契约即测试。小样 2 收益量化裁决点已闭合（2026-08-15 用户裁决通过，独立判定一致）。
- 执行失败特殊反馈与日志可见性（v0.4，已落地）：错误信封恒带 `trace_id`；`step=execute` 失败附有界 trace 尾部；新增只读服务 `assistant.trace`（有界、读入审计）——主模型可结合日志与操作台覆盖未预录内容；POC 冒烟 28/28。
- 机械组合模型（v0.4 设计）：线性脚本模式（PTC）——步骤=注册动作实例 + `$ref` 数据引用，每步独立契约校验 + trace，任一步 fail-closed；无任意代码/隐式控制流，循环/条件暂不做；列入实施序列小样 3。
- 黑板动作栏（v0.5 用户提案，定为生产协作形态）：黑板拆三块——注册板块（助理层维护、当前轮动作投影、常驻按需读）、动作栏（模型写订单，无副作用）、结果栏（receipt+trace_id+fail-closed）；发放=机械单一出口（模型轮结束触发、消费一次、round 防重）；模型面不再出现执行/发送工具；stdio 仍为原型隔离。
- 进度（2026-08-13）：小样 1（控制台路由小样）已跑通——`prototype/classical_console/`（HA 式服务注册表 + hassil 意图 + stdio JSON 协议），服务模式（`workspace.read_file` / `workspace.list_dir` / `workspace.index` / `assistant.trace`）+ 意图模式（en/zh，槽位表由工作区索引动态生成），含 fail-closed 契约与执行日志，28/28 检查通过。
- 进度（2026-08-15）：小样 3（机械组合脚本模式）已实施——`workspace.run_script`
  服务（线性脚本 + `$ref` 数据引用，执行前静态校验引用存在/作用域/类型、名称唯一、
  禁嵌套脚本；每步独立 schema 校验 + trace；失败保留内层 step/code 并带
  script_step；上限 20 步/30s/4MiB）；固定语料 8 场景对照实验（baseline 逐轮串行
  20 轮 vs candidate 单脚本 8 轮，成功率均 100%），冒烟 87/87；结果工件
  `prototype/classical_console/sample3_result.json`；**已闭合（2026-08-15 用户
  裁决通过，独立判定一致）**。
- 进度（2026-08-15）：orz 内嵌集成 S1 已落地——`orz-loop/src/console.rs`
  操作台核心（ServiceRegistry + ActionSpec 契约 + issue_action 五步路由
  （注册表/契约/目标/执行/验证）+ fail-closed 信封（step/code/message/
  upstream/trace_id，execute 附有界 trace 尾部）+ 有界 Trace/TraceStore；
  执行经 ActionExecutor 抽象委托，生产实现由 controller 复用 run_host_tool
  的权限/ACAF/事件链，禁止绕过既有门）+ 黑板动作栏数据面
  （`blackboard::ActionBoard`：注册板块/动作栏单槽（单轮一单）/
  结果栏有界 50，随 plan epoch 快照归档与轮换）。**2026-08-15 全面检查修复**——
  执行器返回 ExecuteError（执行失败/策略拒绝，`step=policy` + `policy_denied`）；
  响应契约强制必填（注册时校验并缓存 schema，任何输出过机械验证）；
  `exit_code=Some(0)` 成功契约（None/非零按执行失败）；TraceStore 提交语义
  （发放收口 commit，失败事件满员滚动保底）；注册板块最小参数提示投影；
  S2 验收点显式登记（round/epoch 防重放、目标解析、ACAF 票据、策略表、
  step=policy）。新增 18 项测试（console 15 + blackboard 3）；orz-loop
  356 通过 / 0 失败。**2026-08-15 S2 已落地**——模型面投影（
   `blackboard_read` section=actions：注册板块/动作栏单槽/结果栏有界渲染，
   随 plan epoch 归档可读 + `blackboard_action_write` 写单按钮，pending 机械
  拒绝，round/plan_epoch/run_id 机械盖章，主车道专属三重守卫）+ 轮末机械
  发放（post-tool-batch 安全间隙、pending checkpoint 优先；round/plan_epoch/
  run_id 防重放与过期 → 注册表/契约 → 经 ControllerConsoleExecutor 委托
  run_host_tool（权限/ACAF/模式门/事件链）→ 响应 schema 验证 → 结果栏
  receipt + trace_id → TraceStore.commit；策略拒绝归一化 step=policy +
  policy_denied）；注册板块每轮机械刷新（基础动作集 6 项）。新增 8 项测试，
  orz-loop 364 通过 / 0 失败；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`。
  assistant.trace 服务与 PTC/Profile 生产化随后续切片落地。
- 进度（2026-08-15）：**S3 前置（P1-2 结构化策略拒绝）已闭合**——
  `ToolResult.policy_denial = {source, code, reason}`（source ∈
  permission/acaf/retrieval_mode/taint）在 `run_host_tool` 边界五条拒绝
  路径接线（权限 deny/defer、ACAF 票据门、检索模式门 ×3）；console 适配层
  只按结构化信号映射 `step=policy`，`console_policy_refusal` 前缀判定退役；
    ToolCompleted 增可选 `policy_denial`（Schema/verifier/fixtures 先行，
    verifier 交叉规则=exit_code 非 0 + 工具命中已知拒绝路径）；内容碰撞回归
    （成功输出含旧前缀判成功）。**2026-08-15 全面审查修复（F1-F8）闭合**：
    拒绝事件补 `exit_code=1` + `status=error`（含 host 级拒绝）、verifier
    ACAF 家族补 `web_fetch`/`browser_read`、新增生产者事件→验证器对拍
    测试、`orz/` 登记为父仓库 git 子模块（SilverWhite/CLI
    `feat/fusion-architecture`，提交 a0c9ffc 已推送远端）+ 源码完整性清单
    接入仓库门禁。验证：orz-loop 367 通过 / 0 失败、acaf_e2e 21 通过、
    Python runtime 213 通过、仓库门禁 valid（含 orz 清单 1406 文件）。
    实施审计见
    `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`。
- 进度（2026-08-15）：**S3（trace/PTC/Profile）已闭合**——
  `assistant.trace` 注册为控制台内部动作（`ActionKind::TraceRead`：按
  trace_id 有界取回、`not_found` fail-closed；读操作本身入 trace 与
  ToolStarted/ToolCompleted 事件面）；`workspace.run_script` 注册为控制台
  内部动作（`ActionKind::RunScript`：PTC 线性脚本生产化——`$ref` 静态/
  运行时校验、逐行契约校验 + trace、上限 8 步/30s/4MiB、禁嵌套、失败
  保留内层 step/code + `script_step`；内层步骤逐行经 `issue_action` 复用
  注册表/契约/目标/执行/验证五步链，仍走 `run_host_tool` 全链路）；注册
  板块升级为 Profile/Bundle ∩ 探针完整集（`ActionBundle` standard/
  read_only/benchmark + `registrations_for`，同轮探针快照同时驱动工具投影
  与注册板块）。验证：orz-loop 377 通过 / 0 失败（3 ignored live）、
  fmt/clippy 无新增告警、orz-host/tui/bin/assurance check 通过、仓库门禁
  valid（含 orz 清单 1406 文件，已重新生成）。实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`。
  2026-08-16 全面审查收口（用户逐项裁决）：单订单步数上限 20→8
  （`MAX_SCRIPT_STEPS_PER_ORDER`）、`assistant.trace` 查无 id 定案
  `step=execute`+`not_found`、checkpoint 轮跳过注册板块刷新、注册不变式
  补齐（内部 target/kind 唯一/bundle 非空/嵌套按 kind）、最终超限信封补
  `script_step`；orz-loop 384 通过 / 0 失败；S4 登记项=单步超时（host 层
  进程树收口）与脚本消耗 tool-round 预算，详见审计 §6。**S4 已闭合
  （2026-08-16）**——单步超时下沉 host 层（`call_tool_with_timeout` 覆盖 +
  结构化 `timed_out` → `tool_timeout`/`script_timeout`+`script_step`）、
  脚本 tool-round 预算（预检 `budget_insufficient`/实际步数减计/下一轮预算
  块反映）、端到端测试（完整会话/checkpoint 板块保留/超时/预算边界）；
  orz-loop 392 通过 / 0 失败；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`。
- 实施序列：
  1. 槽位表由工作区索引动态生成（POC 已闭合；生产接线复用 orz `project_doc_index` 缓存）；
  2. 编辑执行器 `workspace.search_replace`（小样 2，收益裁决点：编辑应用成功率 + 主模型工具轮数）——**已闭合（2026-08-15）**，测量与结果工件见 `prototype/classical_console/sample2_result.json`；
  3. 机械组合脚本模式（小样 3：线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）——**已闭合（2026-08-15）**，用户裁决通过 + 独立判定一致，测量与结果工件见 `prototype/classical_console/sample3_result.json`；
   4. orz 内嵌集成（HA 操作台作为 orz 组件接线；薄接缝在 orz ↔ 底座模型后端；黑板动作栏为生产协作接缝，POC stdio 仅原型隔离）——**S1 已落地（2026-08-15）**：操作台核心 + 黑板动作栏数据面；S1 全面检查修复已闭合（2026-08-15）；**S2 已落地（2026-08-15）**：模型面投影 + 轮末发放（含 round/epoch/run_id 防重放、step=policy 归一化、TraceStore.commit 收口），实施审计见 `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`；**S3 前置（P1-2 结构化策略拒绝）已闭合（2026-08-15，全面审查修复 F1-F8 已登记）**，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`；**S3（assistant.trace 接线 + run_script PTC 生产化 + Profile/Bundle 加载）已闭合（2026-08-15）**，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`（2026-08-16 审查收口 §6）；**S4 已闭合（2026-08-16）**：端到端测试 + 单步超时（host 层进程树收口）+ 脚本 tool-round 预算消耗 + 决策门材料，审计见 `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`；
  5. 小样全面达标后裁决正式组件（决策门）；不达标即撤——**已闭合（2026-08-16
     用户裁决「P0-C 可转正式组件」）**。

### 3a. PLAN-FIRST-BLACKBOARD（模型面重构；2026-08-15 用户定案）

- 入口：[设计](PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)；索引：
  [CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)（AUTH-PLAN-FIRST-BLACKBOARD /
  FUS-PLAN-FIRST-MODEL-SURFACE / FUS-PLAN-STEP-GATE / FUS-AGENTS-MD-PLAN-WRAPPER /
  FUS-PROMPT-DEPERSONALIZE / FUS-CONSOLE-DUAL-MODE / FUS-CONSOLE-POLICY-DENIAL）；
  权威：ADR-0010 §14.17（v1.17）。
- 定案（2026-08-15 用户裁决）：放弃「直接执行面永久移除」；双模式 console 默认 +
  direct 受控降级（3 连败助理层故障面 → 无工具询问轮 → 切换留痕；计划门约束
   console 订单，direct 为有记录的例外，`console_step_done` 需证据置 done）；
  结构化策略拒绝（P1-2）为 S3 前置。
- 阶段：A（模板去人格 + AGENTS.md 计划型机械包裹 + 首轮计划轮硬门）；B（注册板块=
  探针投影 + 工具栏刷新绑定黑板模型栏）；C（console 默认 + direct 受控降级）。
  明细与验收见设计 §9/§11；实施勾选见 TODO P0-C。
- 进度（2026-08-16）：**阶段 A 已闭合**——模板去人格（主/子代理/apply-patch
  模板 + ORCHESTRATOR_PROMPT_BODY，XOR 加密模板重生成，无人格关键词渲染测试）、
  AGENTS.md 计划型机械包裹（`<plan_first_framework>` 固定前缀，用户内容之前）、
  首轮计划轮硬门（`plan_write` v0.2 事件 + 结构化校验/一次重填/降级留痕 +
  黑板 plan epoch 落板；首轮工具面=blackboard_read+plan_write，其余机械拒绝；
  会话级门，ACP server/CLI run 生产接线）；实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`。
- 进度（2026-08-16 审查收口）：计划轮不消耗 tool-round 预算（用户裁决）、
  compaction whitelist 窗口顺延至计划落板后首个执行轮、结构上限定稿
  （步骤 ≤32/动作 ≤8/证据 ≤16/总量 ≤32K，参照 AutoGPT/oh-my-loop/joyagent/
  编排工具/LangChain 成熟设计）、AGENTS.md 包裹改为系统提示词层无条件注入
  （canonical 常量入 orz-assurance，不依赖 AGENTS.md 存在）、P1 工具事件
  契约形状收敛、plan_write 面随开关收敛（子代理/grill 剔除）、P3 全部闭环
  （TUI 降级投影/同轮单次/参数 schema/上限/persona 测试）；ADR-0010
  §14.17⑯、审计 §7 登记。
- 进度（2026-08-16）：**阶段 B 已闭合**——注册板块=探针投影收口：controller
  单一探针源（`console_probe_source`，仅内存/随轮覆盖、run 起始复位）→
  `sync_console_registrations` 为派生唯一路径（Profile/Bundle ∩ 探针完整集），
  无探针轮次不再 bundle-only 刷新、沿用上一轮内容（移除静态基础集中间态）；
  工具栏刷新绑定黑板模型栏：`blackboard_read section=actions` 读取时由最近
  探针源派生并持久化（live 视图），归档 epoch 读保持快照（测试锁定）；工具
  投影与注册板块共用同一探针源（同源一致性测试锁定；审查收口：run 起始复位
  与归档读不派生单测）。实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。
- 进度（2026-08-16）：**阶段 C 已闭合**——console 默认 + direct 受控降级
  双模式：v0.2 事件 +2（`console_mode_transition` / `console_order_written`，
  action_write ToolCompleted 收敛通用形状）、双模式状态机（故障连败/询问轮/
  switch/stay/return）、步骤状态机（`pending → in_progress → done|failed`、
   步骤门 step_not_done、`console_step_done` 证据门）、模型面收敛（console 面=
  黑板读写+只读核查；direct 恢复工作工具投影并全链路盖章）；实施审计见
  `docs/audits/GAP_PLAN_FIRST_STAGE_C_IMPL_AUDIT_2026-08-16.md`。

### 3b. ORZ-COMPACTION-REDESIGN（`implemented`；P0，S1-S4 已闭合 2026-08-14）

- 入口：[设计](CONTEXT_COMPACTION_DESIGN_2026-08-14.md)；索引：[CLI_PROJECT_INDEX.md](../CLI_PROJECT_INDEX.md)；
  权威：ADR-0010 §3.6 / §14.10 / §14.14（v1.10 + v1.14）；实施审计：
  [GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md)。
- 决策门：2026-08-14 用户裁决定稿——有效窗口 384K、160K 普通触发 / 200K 兜底、工具调用记录每轮
  机械坍缩（零模型调用）、五段模板摘要（≤17K 字符、derived_unverified、冷却 2 模型轮、摘要链只进
  审计、滚动回查 marker）；推翻 2026-08-08 零模型摘要与仅最终答案间隙裁决；2026-08-14 用户
  放行实施；2026-08-14 审查修复裁决（v1.14）——守卫失败重试 3 次后强制压缩并报告、会话结束
  自动压缩治本、存档写失败显式重试报告、退化守卫 300 等效字符（CJK 折算 2）、黑板 edit 窗口
  滚动、冷却 3→2 模型轮。
- 实施切片：
  1. S1：D2-2 恢复超窗预估算截断 + D3-1 marker/白名单恢复保留（含测试）——**已闭合**；
  2. S2：工具调用记录机械坍缩（动作台账行 + 配对纪律 + 指针完整性 + 测试）——**已闭合**；
  3. S3：五段模板摘要接线（退化守卫 300 等效字符门、会话模型覆盖、17K 校验、重做/终止态、
     `context_compressed` 事件 Schema v0.2 + verifier/fixtures）——**已闭合**；
  4. S4：实施审计、ADR-0010 §14.10 补写、索引/BACKLOG/TODO 同步、设计文档状态更新——**已闭合**。
  5. S5：审查修复（守卫重试/强制压缩 + `guard_failed`、会话结束压缩 `session_end`、
     存档写失败 `archive_write_failed`、黑板窗口滚动、120s 超时、Top-40、契约扩展）——
     **已闭合**。
- 进度（2026-08-14）：S1-S5 全部闭合——恢复预检（`context_recovery_truncated` 事件 +
  完整侧车审计副本）、marker/白名单恢复保留、动作台账请求视图坍缩、五段模板摘要
  （160K/200K/2 模型轮/5K/0.6、重做 ≤3、summary_incomplete 终止态 + fallback 机械截断、
  `.gsa/compaction/` 存档 + 7 天 retention、TUI 投影、守卫强制报告、会话结束压缩、
  存档写失败显式报告、黑板窗口滚动）；orz-loop 305 / orz-host 208 / orz-tui 178 通过，
  工作区编译 exit 0，Python 事件校验 171 通过，仓库门禁 valid。
- v1.15 注（2026-08-14 用户裁决，实施未开始）：其中「黑板 edit 窗口随压缩滚动」机制
  已废止，黑板生命周期改按 plan epoch 轮换（见 6e）；压缩机制其余部分不变。

## P1 — 可并行审计 / 证据

### 4. FUS-COMPONENT-REGISTER（`partial`）

- 开放内容：65 组件全 `audit_required`，逐 crate 采用审计未开始（V11-IMPL-008）；不得从 crate 名/编译推断采用档位。
- 入口：[register](../upstream/fusion-component-register-v0.1.yaml)；[V1.1 复核](audits/ADR_0010_V1_1_SUPPLEMENT_REVIEW_2026-08-09.md)。

### 5. GAP-WINDOWS-EVIDENCE（`partial`）

- 开放内容：ORZ-WIN-PROC-001/002/003 仍为 `candidate`；需真实 provenance、脱敏、分类与回归门槛（FUS-DOC-001）。
- 入口：[incidents](incidents/windows/README.md)；[cases](cases/windows/README.md)。

### 6. IMPL-DEEPSEEK-TRANSPORT + SEC-CREDENTIALS（`partial`）

- 开放内容：transport/retry/thinking 按主/子代理同构约束复核——**已闭合（2026-08-16）**：
  三实例共享同一 `DeepSeekTransport`（ModelConfig/RetryPolicy/thinking 单一来源、
  `REQUEST_MAX_TOKENS=160_000` 单一常量；请求级覆盖仅 `-p` 预检轮，文档化 F-07），
  证据与边界见变更记录。仍开放：DeepSeek live 通道与 Windows 实机晋级证据
  （ADR-0010 §11.7；当前仍为 offline / 构建时 evidence）。
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
- 2026-08-15 实施闭合：① 请求 header 变化留痕——新增 v0.2
  `request_header_change` 事件（system+tools+config 三摘要 + `agent_role` +
  initial/change + `previous_header_sha256`；Schema/verifier/fixtures/TUI 同步）；
  ② 探针准确性——验证器翻转↔header 交叉核对 + `assurance/probe_accuracy_audit.py`
  （假完整/假不完整候选、门禁类 error 不计误判、旧 journal 兼容边界）；
  ③ 单轮注入预算——`ORZ_MAX_INJECT_TOKENS_PER_ROUND` 默认 50K、按模型轮累计、
  超限无 ToolStarted 拒批 + `inject_tokens_used/budget` 字段 + offset/grep 提示 +
  提示词读取纪律；验证 orz-loop 337 / orz-assurance 151 / orz-tui 178 /
  orz-bin 全绿，Python 1888+14 skipped，仓库门禁 valid。审计：
  `docs/audits/GAP_CACHE_CONTEXT_COST_IMPL_AUDIT_2026-08-15.md`。
- 2026-08-15 二次全面审查修复：payload 增 `change_kind`（机械「变化原因」，
  Schema/verifier 校验摘要差一致）；verifier 允许每车道多链 initial（子代理
  多 activation/主车道多 run 合法）；新增 `_verify_v02_inject_budget`；
  辅助模型请求（压缩摘要/预检）留痕边界登记；预算计数口径注记。详见审计 §7。

### 6c. ORZ-ORIENTATION-FORCED-TEMPLATE（`implemented`；P1，2026-08-15 闭合）

- 定位：中立问询升级为强制模板轮——触发点下一安全动作间隙明确暂停，模型填写问询
  模板后才恢复动作（Orientation 与 DC 两族共用，主车道）；目的=拉回注意力、防跑偏
  与钻牛角尖（强制表达、不验证诚实）；缓解必做（`progress_evidence` 存在性交叉
  校验、`gather_evidence` 必填缺失面）。
- 实施前置：ADR-0010 §4.2 正文修订（v1.13 已登记裁决）；事件/Schema/verifier/
  fixtures；测试；实施审计与索引同步。
- 2026-08-15 实施闭合：ADR-0010 §4.2 正文修订（v1.16）——「注入」→「触发点暂停并
  填写模板」；强制模板轮实现（checkpoint 轮无工具、模板字段/机械校验、一次错误反馈
  重填 + 降级兜底）；缓解必做（`progress_evidence` 与 journal 证据身份存在性交叉
  校验、`gather_evidence` 必填缺失面）；v0.2 `checkpoint_response` 事件
  （Schema/verifier/fixtures/TUI/枚举同步）；测试覆盖触发→暂停→填表→恢复、重填、
  降级、checkpoint 轮工具拒绝、主车道隔离与计数语义。实施审计见
  `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
  2026-08-15 二次全面审查后复核修复：DC fire 增可选 `agent_role=main`
  （Schema/fixtures/生成器同步），验证器兼容历史 fire 并新增 outcome↔validation
  与 gather_evidence 条件交叉；响应角色收紧主车道；Rust 长度按 trim 后值；
  conformance 计数更名；orz-loop 333 / Python runtime 264 通过。
- 入口：[设计](ORIENTATION_FORCED_TEMPLATE_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

### 6d. ORZ-SESSION-CONTEXT-MONITOR（`approved`；P1，2026-08-14 登记；2026-08-16 度量重定）

- 定位：会话压缩次数监测——度量=会话内压缩次数（`context_compressed`
  reason∈rhythm/fallback 计数、session_end 不计、一次一计）；≥2 次机械提醒、
  ≥3 次机械总结推荐（可配，默认待校准；2≈旧 384K、3≈旧 500K）；最简实现=阈值到达的
  最后一轮模型输出末尾机械附言（附次数）；headless 仅日志；TUI/journal 事件为
  beta 前可选；与压缩独立。
- 实施前置：压缩事件计数接线、阈值配置、机械附言注入点、测试、实施审计与索引同步；
  原 token 度量与 chars/2 中文估算校准项随 2026-08-16 用户裁决废止（理由：累计 token
  对模型不可见、阈值无质量边界，压缩次数为更直接的会话寿命代理）。
- 入口：[设计](SESSION_CONTEXT_MONITOR_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.13](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [ADR-0010 §14.18](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [TODO](../TODO.md)。

### 6e. ORZ-BLACKBOARD-PLAN-EPOCH（`implemented`；P1，2026-08-14 实施闭合）

- 定位：黑板生命周期按 plan epoch 轮换，与压缩生命周期解耦——plan 区为单写者复写区
  （`plan_id`/`plan_epoch`）；仅新 plan epoch 批准触发原子轮换（归档旧 epoch 快照 →
  清 edits/tool_actions/exec 工作区并复写 plan → 写新 plan）；gate_log、白名单与
  检索分区不清；压缩不再清黑板（废止 v1.14「黑板 edit 窗口随压缩滚动」）；路径槽=
  本 epoch 增量（Top-40 + 5K + 指针）；marker 带 `plan_epoch`；`blackboard_read`
  跨 epoch 走归档。
- 决策依据：长任务压缩频繁，黑板随压缩擦除会破坏任务工作状态并动摇中立问询的
  task_position/计划锚点（2026-08-14 用户裁决）。
- 进度（2026-08-14）：S1-S5 全部闭合——plan 批准事件携带 `plan_epoch`（Schema/
  fixtures 同步）；`with_plan` 带 `plan_id`+`plan_epoch` 身份，同 plan_id 修订不清板、
  新 plan_id 原子轮换（归档旧 epoch → 清 edits/tool_actions/exec → 复写 plan）；
  `.gsa/blackboard/epoch-<n>.json` 快照（批准/修订持久化当前 epoch，轮换归档旧 epoch；
  写入有界重试、失败 warn 不阻断）；`blackboard_read` 增 `epoch` 参数跨 epoch 回查
  （缺失显式提示，不静默回退）；压缩不再清黑板（`context_compressed` marker 携带
  `plan_epoch`，路径槽溢出指针指向 epoch 快照）；archive dir 构造时装载最新 epoch
  快照（恢复入口）；retention 覆盖 `.gsa/blackboard` 7 天清扫。验证：orz-loop 310 /
  orz-host 209 / orz-tui 178 / orz-bin 全部通过；Python runtime journal 校验 157 +
  conformance 14 通过；仓库门禁 valid、0 错误。
- 补强（2026-08-15，全面复查 F1/F3，用户裁决）：`plan_epoch` 改为时间戳单调编号
  （unix 毫秒为基底，`next = max(now_ms, 磁盘现存 max + 1)`）——清扫后编号不复用、
  marker/`blackboard_read epoch` 引用跨窗口唯一；身份不变式强制（一一对应）：同
  plan_id 必须沿用同 plan_epoch、新 plan_id 必须严格大于当前 plan_epoch，
  `rotate_to_plan`/`try_with_plan` 返回错误、`with_plan` fail-fast、拒绝先于任何
  黑板变更；retention 对 `.gsa/blackboard` 清扫保留最高编号快照（恢复入口）。
  验证：orz-loop 312 / orz-host 210 / orz-tui 178 / orz-bin 全部通过；Python
  runtime 251 通过；仓库门禁 valid、0 错误。
- 复查遗留处理（2026-08-15 明确记录并全部闭合，v1.15⑨；来源：全面复查问题清单，
  F1/F3 已由 v1.15⑧ 补强处理，F8 随补强顺带修复）：
  - F2（P2，已处理）：epoch 归档写盘改为临时文件+自检解析+rename 原子提交，
    崩溃只可能留下 `.tmp`；`latest_epoch_snapshot` 从高到低回退到第一个可解析
    快照，半截文件不再阻断恢复。
  - F4（P3，已处理）：epoch 分配增加 `.claim-<n>` 原子占号（`create_new`），
    同毫秒并发进程只有一个能赢得编号，碰撞方递增重试；claim 计入下次分配扫描，
    崩溃不导致复用，retention 按年龄清扫过期 claim。
  - F5（P3，已处理）：路径槽溢出指针改从 controller 配置的归档目录单一来源
    取路径，不再由 `session_cwd/.gsa/blackboard` 重算，自定义归档目录不失真。
  - F6（P3，已处理）：`blackboard_read` 的 `epoch` 参数区分“缺失”与“非法”，
    0/负数/浮点等非法值显式报错（exit_code=1），不静默回退 live 视图。
  - F7（P3，已处理）：旧 epoch 归档写失败并入事件面——新增 v0.2
    `epoch_archive_write_failed`（rotated/current + attempts），builder 阶段
    排队、run 启动随 journal 写入，schema/fixtures/TUI 同步。
  - F9（P4，已处理）：`EpochSnapshot.rotated_at` 更名 `persisted_at`
    （serde alias 兼容旧归档），语义与实现一致。
  - F10（P4，已处理）：设计文档 §5 恢复措辞对齐 ADR ⑦/⑧（archive dir
    装载最新 epoch 快照）。
  - 验证（2026-08-15）：orz-assurance 151 / orz-loop 317 / orz-host 211 /
    orz-tui 178 / orz-bin 全部通过；Python runtime + assurance 1858 通过、
    14 skipped；仓库门禁 valid、0 错误。设计/ADR/BACKLOG/TODO/审计已同步。
- 入口：[设计](BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md)；
  [ADR-0010 §14.15](../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；
  [实施审计](audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md)；
  [TODO](../TODO.md)。

## P2 — 生产化决策门

### 7. IMPL-CONTROL-FABRIC（`partial`）

- 决策门：**2026-08-15 用户裁决 fail-closed 生产启用放行；2026-08-16
  翻转执行已闭合**——fail-closed 改为默认（未设置即强制；显式
  `0|false|no|off` 影子；非法值 exit 2），CLI run / ACP stdio / TUI 三个
  生产入口全部接线（ACP/TUI 此前未挂签名器客户端），核查清单 ⑦⑨⑩⑪ 收口
  （⑦ web_search 显式排除走 provider 原生搜索；⑨ host 稳定面不补绑定；
  ⑩ URL gate 与票据摘要规范化等价；⑪ 重定向逐跳 URL gate 覆盖），新增
  `orz-acaf-provision` 供应工具 + `scripts/orz_acaf_run.ps1` 启动链；审计见
  [`GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md`](audits/GAP_ACAF_FAILCLOSED_PRODUCTION_ENABLEMENT_IMPL_AUDIT_2026-08-16.md)。
- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 + policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）。
- Slice 4：Windows Sandbox backend（D-11）。
- 可选：conformance capture 票据场景；normalize_lexical 单源化（检索车道
  activation 绑定与 ACP 会话接线已随 Slice 2 / fail-closed 翻转完成，
  2026-08-16 收口）。
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

- 2026-08-17：订单发放前拒绝入事件面项登记（用户指示处理）——冒烟重跑 ORD-000011
  （workspace.run_tests，arguments={}）写入后发放前被拒，失败只进结果栏 receipt +
  TraceStore（`consume_console_order` 未写 journal 事件），journal 无结构化拒绝记录。
  目标=发放前拒绝（order_stale / step_not_done / budget_insufficient / registry /
  contract / target / ACAF / policy / mode 门）统一入 v0.2 事件面（新增
  `console_order_rejected`，Schema/verifier/fixtures 先行）。P0-E 6 项、未闭合 33 项。
- 2026-08-17：grep 侦查纪律项登记（用户确认一并处理）——冒烟重跑中模型用不存在的
  目标字符串 grep 全树、空结果被过度泛化为「/app 无 C 源码」；工具行为正确（无匹配
  exit_code=1），属侦查策略/反馈解读问题。实施方向：提示词/计划框架侦查纪律 + 注册
  板块 grep 参数提示空结果语义 + 回归验证。P0-E 5 项、未闭合 32 项。
- 2026-08-17：ACAF 跑分决策登记（用户裁决）+ P0-E 下一步实施项对齐——① GAP-ACAF-
  HARNESS-PASSTHROUGH：跑分保持 ACAF 强制开启（容器内供应 manifest/keystore/signer，
  不接受影子模式），实施=适配器透传 + 容器供应 + 移除临时 `ORZ_ACAF_FAIL_CLOSED=0`
  覆盖 + 冒烟验证；② 两项 P1 观察升为实施项（plan_write 提示词/示例强化、
  `steps[].actions` 形状校验收紧）；③ 冒烟重跑定位新增步骤门模型面缺口——计划视图
  `section=plan` 不渲染步骤 `id`（`epoch.rs` 渲染仅 `[status] goal (actions; evidence)`），
  而订单 step_id 需精确绑定，模型靠猜测导致大量空转（轨迹 13/18/23 步、4 次 plan_write）；
  同时记录 grep 侦查低效（目标字符串不存在 → 空结果被泛化为「无源码」）。证据：
  `D:\tb-eval\jobs\2026-08-17__03-48-57`（reward 0.0、3 项 verifier 失败：
  test_vm_execution Timeout / test_frame_bmp_exists / _similar_to_reference FileNotFound）。
- 2026-08-17：GAP-CONSOLE-TOOLNAME-PATTERN 闭合——三个 console 面工具名点号改下划线
  （`blackboard.action_write`→`blackboard_action_write`、
  `console.step_done`→`console_step_done`、
  `console.return_to_console`→`console_return_to_console`）；同步 11 个 Rust 文件
  （91 处）+ Python verifier/schema/测试 + ADR-0010 §14.20 与设计文档；orz 子模块
  commit 0304b23、`orz_source_manifest.sha256` 重生成 1401 条目、仓库门禁 valid；
  orz-loop 434 / orz-tui 178 / orz-assurance / orz-bin acaf_e2e 23 + real_flag 2
  通过，clippy 无新增告警；Linux musl 二进制重建（rust:latest + aliyun 镜像）后
  make-doom-for-mips 冒烟重跑 `D:\tb-eval\jobs\2026-08-17__03-48-57`——30m21s 跑满
  1740s 预算、`run_invalidated{wallclock}` 正常收尾，console 全链路（计划落板/修订/
  订单发放/上下文压缩）无 400（对比旧运行 `01-09-43` 400 即死）。GAP-ACAF-HARNESS-
  PASSTHROUGH 保持开放（正式跑分决策待定）。
- 2026-08-17：评测冒烟暴露问题登记（最优先）——正式跑分前最难错题试跑
  （make-doom-for-mips，flash + plan-first + console 默认 + headless）暴露两项阻断：
  ① GAP-ACAF-HARNESS-PASSTHROUGH：ACAF fail-closed 默认强制后 TB2 适配器不转发
  签发器配置，`orz --real` 拒启（`no signer client is configured`）；已临时以
  `ORZ_ACAF_FAIL_CLOSED=0`（影子模式）+ 适配器透传解阻，正式跑分决策待定。
  ② GAP-CONSOLE-TOOLNAME-PATTERN：console 面工具名点号违反 OpenAI 兼容工具名模式
  `^[a-zA-Z0-9_-]+$`，计划落板后下一轮 400 → run_failed（FakeProvider 不校验故单测
  未暴露）；修复=改名下划线 + 同步 verifier/schema/文档 + 重建重跑。另记录两项 P1
  观察（plan_write 字符串序列化→提示词强化；actions 空串→校验偏宽）。证据：
  `D:\tb-eval\jobs\2026-08-17__01-05-31` / `01-07-59` / `01-09-43`、
  `D:\tb-eval\gsa-volumes\b4-900s\a5da4937-2775-47ff-8202-98bf03a74780\runs\RUN-CLI-6a81eeed\events.jsonl`；
  本轮已重建 Linux 评测二进制（旧版备份 `orz-20260812.bak`）。处理窗口=新 Codex 窗口，
  详见 P0 0a。
- 2026-08-16：IMPL-DEEPSEEK-TRANSPORT 主/子代理同构复核闭合登记——
  transport/retry/thinking 三实例同构成立（`AgentLoopController::with_gateway`
  单 gateway 三实例克隆，controller.rs 2056-2064；`DeepSeekTransport::deepseek_v4`
  单 ModelConfig，transport.rs 92-107；`RetryPolicy::default()`，model.rs 47-70；
  `REQUEST_MAX_TOKENS=160_000` 单一常量，agent_loop.rs 49/1146-1148；主/子代理请求
  均 `thinking: None` → transport 默认 EnabledMax，agents/main.rs 50、
  agents/retrieval.rs 84；唯一请求级覆盖=`-p` plan gate，main.rs 630，F-07
  文档化例外；全仓无 per-agent 模型/重试 env）。边界登记：① 压缩摘要轮
  （SUMMARY_MAX_TOKENS=12_000）与 `-p` 预检轮为 loop 外辅助请求，header 留痕
  明确排除，非同构范畴；② DEEPSEEK_ADAPTER_CONTRACT §2.1 旧别名拒绝与 /models
  能力预检、§2.5 首事件语义未在 production transport 实现，属契约符合性另行
  跟踪。跑分决定（用户）：启用 plan-first（用作该设计的可使用性验证）；
  模型使用 `deepseek-v4-flash`（= 生产默认 `MAIN_AGENT_MODEL`，无需
  `ORZ_MAIN_AGENT_MODEL` 覆盖）。
- 2026-08-16：FUS-SESSION-CONTEXT-MONITOR 度量重定（用户裁决）——度量由会话累计
  模型可见输入 token（384K/500K）改为会话内压缩次数（`context_compressed`
  reason∈rhythm/fallback 计数、session_end 不计、一次一计）；阈值改次数制
  （≥2 提醒 / ≥3 推荐，可配、默认待校准）；chars/2 中文估算校准项废止；设计文档、
  ADR-0010 §14.18、索引、TODO 同步；实施未动（仍 pending）。
- 2026-08-15：推送惯例恢复登记——用户说明此前「Rust 修复不入 git、
  用户手动推送」惯例源于分类器类故障（已修复），恢复正常推送；父仓库
  main（`2ef2aa7`）与 orz 子模块分支 `feat/fusion-architecture`
  （`a0c9ffc`）均已推送远端。
- 2026-08-16：P0-C S3 全面审查收口登记——三层审查（设计/实现/符合性）
  后用户逐项裁决：单订单步数上限 20→8（`MAX_SCRIPT_STEPS_PER_ORDER`）；
  `assistant.trace` 查无 id 定案 `step=execute`+`not_found`；checkpoint 轮
  跳过注册板块刷新（`pending_checkpoint` 守卫）；注册不变式补齐（内部动作
  带 target 拒绝/内部动作类唯一/bundle 非空/嵌套按 kind 拒绝）；最终超限
  信封补 `script_step`+`action`；测试补齐 7 项（运行时 `$ref` 失败、形状/
  字段缺失、数组 items、tail>200、脚本内 TraceRead、9 步拒绝、注册不变式）；
  orz-loop 384 通过 / 0 失败；S4 登记项：30s 墙钟=总墙钟+单步受控（截止
  时间下沉 host 层、host 层进程树收口）、脚本按实际执行步数消耗 tool-round
  预算（预检/减计/错误码/预算块反映）；详见 S3 审计 §6。
- 2026-08-16：P0-C S4 实施闭合登记——单步超时下沉 host 层
  （`LoopHost::call_tool_with_timeout`：显式覆盖 = min(覆盖, 配置预算)，
  到期 `kill_active` 进程树收口；脚本每步传剩余截止；`ToolResult.timed_out`
  结构化信号 → 直接订单 `tool_timeout`、脚本归一化 `script_timeout`+
  `script_step`）；脚本 tool-round 预算（发放前预检 `budget_insufficient`
  零执行拒绝、实际执行步数减计、下一轮预算块机械反映）；端到端测试
  （FakeProvider 完整任务会话、checkpoint 轮板块保留、超时/预算边界）；
  orz-loop 392 通过 / 0 失败；决策门材料清单齐备；详见
  `docs/audits/GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`。
- 2026-08-16：P0-C S4 全面审查收口登记（二次）——host-owned 同步工具
  （`project_doc_index`/`browser_read`/`pdf_read`/PDF 路由 `web_fetch`）
  不经 host timeout 包装为既有边界（配置预算=经注册表执行调用的硬上限）；
  脚本层每步完成后核对 30s 总截止（超时 `script_timeout`+`script_step`
  事后 fail-closed）；预算预检先静态校验脚本（不掩盖 `unknown_service`/
  契约错误）；小样 1 结果工件补齐（`sample1_result.json`，`smoke_test.py`
  90/90 复跑）；max=1 零剩余边界测试锁定；orz-loop 396 通过 / 0 失败；
  详见 S4 审计 §8。
- 2026-08-16：P0-C S4 超时语义复核裁决（用户复核 + Codex/Grok 成熟设计
  对照）——撤销上条「30s 总墙钟含进程时间」语义：脚本每步不传收缩剩余，
  由 host 每调用超时独立约束（配置预算，默认 5 分钟，进程树收口不变）；
  `MAX_SCRIPT_WALLCLOCK_SECONDS`/deadline/事后核对删除，墙钟+字节测试更名
  `run_script_enforces_byte_limits`；orz-loop 395 通过 / 0 失败；详见 S4
  审计 §8。
- 2026-08-15：P0-C S3 前置全面审查修复（F1-F8）闭合登记——拒绝事件补
  `exit_code=1` + `status=error`（含 host 级拒绝）、verifier ACAF 家族补
  `web_fetch`/`browser_read`、permission 家族补 host 路由检索工具、新增
  生产者事件→验证器对拍测试（`PolicyDenialProducerParityTests`）、
    `orz/` 源码完整性清单（`orz_source_manifest.sha256`，1406 文件）接入
    仓库门禁，并登记为父仓库 git 子模块（SilverWhite/CLI
    `feat/fusion-architecture`，提交 a0c9ffc 已推送远端）；验证：orz-loop
    367 / acaf_e2e 21 / Python runtime 213 通过，仓库门禁 valid；详见 S3
    前置审计 §7。
- 2026-08-15：P0-C S3 前置（P1-2 结构化策略拒绝）闭合登记——`ToolResult.
  policy_denial`（source/code/reason）接线五条拒绝路径（权限/ACAF/检索
  模式门 ×3）、console 适配层退役前缀判定、ToolCompleted 增可选
  `policy_denial`（Schema/verifier/fixtures 先行 + 交叉规则）、内容碰撞
  回归；orz-loop 366 / acaf_e2e 21 / Python runtime 209 通过，仓库门禁
  valid；实施审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`；
  同日本切片工具事故导致 `diagnostic_coverage.rs` 生产实现重建（按测试/
  调用面契约，非逐字节恢复），详见审计 §5。
- 2026-08-15：P0-C 内嵌集成 S2 落地登记——模型面投影（`blackboard_read`
  section=actions + `blackboard_action_write` 写单按钮，pending 机械拒绝、
  round/plan_epoch/run_id 机械盖章、主车道专属）与轮末机械发放
  （post-tool-batch 安全间隙、checkpoint 优先；round/plan_epoch/run_id
  防重放与过期（`order_stale`）→ 注册表/契约 → 经 ControllerConsoleExecutor
  委托 run_host_tool（权限/ACAF/模式门/事件链）→ 响应 schema 验证 → 结果栏
  receipt + trace_id → TraceStore.commit；策略拒绝归一化 step=policy +
  policy_denied）；注册板块每轮机械刷新（基础动作集 6 项）；新增 8 项测试，
  orz-loop 364 通过 / 0 失败；审计见
  `docs/audits/GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`。
- 2026-08-15：P0-C 内嵌集成 S1 全面检查修复登记——执行器返回 ExecuteError
  （执行失败/策略拒绝，`step=policy` + `policy_denied`）；响应契约强制必填
  （注册时校验并缓存 schema，任何输出过机械验证）；`exit_code=Some(0)`
  成功契约（None/非零按执行失败）；TraceStore commit 提交语义 + 失败事件
  满员滚动保底；注册板块最小参数提示投影；S2 验收点显式化（round/epoch
  防重放、真实目标解析、ACAF 票据、策略表、step=policy）；新增 18 项测试
  （console 15 + blackboard 3），orz-loop 356 通过 / 0 失败。
- 2026-08-15：P0-C 内嵌集成 S1 落地登记——orz-loop 新增 `console` 模块
  （ServiceRegistry/ActionSpec/issue_action/信封/Trace+TraceStore，执行经
  ActionExecutor 抽象委托）与黑板动作栏数据面（`blackboard::ActionBoard`
  注册板块/动作栏单槽/结果栏有界，随 plan epoch 归档轮换）；S2 模型面投影
  + 轮末发放待续。
- 2026-08-15：P0-C 小样 2 闭合登记——编辑执行器 `workspace.search_replace` 对照实验实施并测量（固定 10 场景语料；baseline 成功率 100%/平均 1.8 轮 vs candidate 100%/平均 1.0 轮，通过标准两项满足；POC smoke 57/57），用户裁决通过、独立判定一致；DSH B 项（文件观察策略收编为 search_replace 动作契约规则）随之落地；结果工件 `prototype/classical_console/sample2_result.json`，待办路由见 TODO P0-C。
- 2026-08-15：P0-C 小样 3 闭合登记——机械组合脚本模式 `workspace.run_script`
  （线性脚本 + `$ref` 数据引用 + 逐行 trace + fail-closed）对照实验实施并测量
  （固定 8 场景语料；baseline 成功率 100%/共 20 轮/平均 2.5 轮 vs candidate
  100%/共 8 轮/平均 1.0 轮，通过标准三项满足；POC smoke 87/87），用户裁决通过、
  独立判定一致；结果工件 `prototype/classical_console/sample3_result.json`，
  下一步=orz 内嵌集成，待办路由见 TODO P0-C。
- 2026-08-15：P0-C 全面审查 P1 修复登记——三项 P1 处理完成：4 MiB 上限改为
  最终响应累计（含未命名步骤与 `result`，错误码 `script_response_limit`）；
  `$ref` 运行期解析失败结构化（`invalid_reference`/`step=execute`，upstream 带
  `ref` 与 `script_step`）；baseline 人工模拟边界显式写入 README、语料描述与
  结果工件 provenance（含 `evidence_boundary`）；结果工件重生成（指标不变），
  POC smoke 90/90 通过。
- 2026-08-15：ACAF fail-closed 生产启用裁决登记——用户裁决放行（P2
  IMPL-CONTROL-FABRIC 决策门）；翻转执行与核查清单 ⑦⑨⑩⑪ 收口待实施，
  TODO/索引/ADR-0011 同步。
- 2026-08-14：黑板擦除机制重设计登记（用户裁决）——ADR-0010 v1.15（§14.15 补写）、
  新设计文档 `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`、压缩设计 v1.15 注记、
  P1 6e 登记；废止「压缩成功后清空黑板 edit 窗口」机制（设计层面，2026-08-14 实施闭合，
  见 `GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`）；
  黑板按 plan epoch 轮换（归档/清工作区/复写 plan 原子提交；gate_log/白名单/检索分区
  豁免）；路径槽=本 epoch 增量、marker 带 plan_epoch、blackboard_read 跨 epoch 走归档；
  中立问询/DC 锚点跨压缩稳定。
- 2026-08-14：P0-D 二次复查处理登记（用户要求处理复查全部问题）——设计投影 §7
  复用边界对齐（orz-compaction 500 字符门表述移除、语义等价内联）、ADR §3.6/§14.14
  补写 session_end 160K 触发阈值、代码注释与 schema 冷却残留修正、终止态 marker
  占位 digest 改显式"（未生成）"、fixtures 生成器回写 P0-B step 5 手工修订（README/
  负样例/信封时间戳/LF 行尾）、chars/2 中文低估登记为 6d 实施前置校准项；
  ADR-0010 §14.14 条目 2、审计 §8；详见
  [GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md](audits/GAP_COMPACTION_REDESIGN_IMPL_AUDIT_2026-08-14.md) §8。
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
- 2026-08-15：ORZ-ORIENTATION-FORCED-TEMPLATE 实施闭合（用户指示优先）——
  ADR-0010 §4.2 正文修订（v1.16）；强制模板轮实现（checkpoint 轮无工具、模板字段/
  机械校验、一次重填 + 降级兜底、pending 单槽与 Orientation 优先、主车道/检索车道
  边界）；缓解必做（`progress_evidence` 证据身份交叉校验、`gather_evidence` 必填
  缺失面）；v0.2 `checkpoint_response` 事件（Schema/verifier/fixtures/TUI 同步）；
  测试 orz-loop 332 通过（新增触发→暂停→恢复/重填/降级/工具拒绝/主车道隔离等用例）；
  FUS-ORIENTATION-FORCED-TEMPLATE 转 `implemented`；实施审计
  `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
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
