# AGENT DELIVERY FLOW 设计（2026-08-23：计划步语义 + 末步递交 + 订单反馈 + 引用修正机会）

> 状态：`current-design`（2026-08-23 设计定稿，用户三轮讨论裁决；先落设计、未实施）。
> 入口：ADR-0010 §14.35 第 19 项 / BACKLOG 0d 后续 7 / TODO P0-0d 后续 7 /
> CLI_PROJECT_INDEX。关联：FUS-PLAN-STEP-GATE（ADR-0010 §14.17）、
> FUS-RETRIEVAL-MECH 引用校验、BLACKBOARD_READ_CACHE_COST（结果板瘦身）。

## 1. 来源与问题

### 1.1 S4 冒烟失败归因（2026-08-23，sweep-s4-g4/g5）

四题未通过（video-processing / mteb-retrieve / sanitize-git-repo /
path-tracing-reverse）经轨迹与验证器定位：**均非架构根因**（无 400、无
transport/哨兵误判影响结果、无 ACAF/权限阻塞），根因分别为解法过拟合、
检索编码约定不一致、漏删密钥、官方超时未产出。但归因过程暴露两个
**架构级摩擦**与一个**核实压力问题**：

1. **计划步自动推进语义**（mteb-retrieve）：controller 在订单完成时自动
   `mark_step_done(receipt)`——步骤"完成"= 该步有订单执行成功，而非该步
   目标达成。mteb 5 步计划全部随探查类订单推进而"完成"，模型在 step 20
   自述「The plan completed but the actual computation never ran」，浪费
   一个 plan epoch（plan_write 2 + 1 次 step 门拒绝）。
2. **引用校验硬阻断无修正**（mteb-retrieve）：最终回答引用容器内
   site-packages 路径（经终端输出查看，不在工具观测台账）→
   `unresolvable_citation` 硬阻断，模型文本永不提交、无修正机会。
3. **订单反馈缺 diff/delta**：actions 结果板每条只有
   `order_id ok step code trace_id`（缓存成本瘦身）；`workspace_delta`
   在事件层已计算但模型不可见；search_replace 响应不回显 diff。模型确认
   "改动落实"需自行 grep/读文件/git diff，核实压力大（video-processing /
   sanitize-git-repo 轨迹可见大量重复核查轮）。

### 1.2 讨论裁决（2026-08-23 用户）

- 计划步采用「执行顺序标记」重定义（比"目标达成证明"更简单、诚实），
  目标是否达成交付门仲裁；**实际动作幻觉需严格处理**。
- 模板末步固定「递交/完成」并自动进入交付流程。
- 递交状态改为**黑板 plan 机械判定**，不做冗长交付物声明（避免复读风险
  与 token 成本）。
- 引用失败先给**有界修正机会一次**；终端内容登记引用暂缓（复杂设计后续
  再想，简单方法够用就不升级）。

## 2. 定案设计

### 2.1 计划步骤语义重定义：执行顺序标记

- **语义**：步骤只约束「按序发放订单、上一步 receipt 机械放行下一步」；
  步骤完成 ≠ 目标达成。目标是否达成交由交付门（最终回答引用校验 +
  验证器）仲裁。
- **理由**：机械上无法一般性地验证"模型意图的计算发生"；订单 receipt
  只能证明命令执行，不能证明意图达成。硬造伪验证不如诚实重定义。
- **变更面**：规划模板/系统提示词中步骤语义段改写（步骤=执行批次标记、
  完成=该批订单已执行）；blackboard plan 渲染文案去掉"完成=目标达成"
  暗示；`step_not_done` 顺序门保留（文案可微调为顺序语义）；ADR-0010
  §14.17 语义澄清登记。
- **动作幻觉的严格处理边界**（如实登记）：运行中途"我以为计算跑过了"
  类幻觉机械不可直接捕捉；其成本被收敛到「最终交付被门拒绝 → 有界修正
  一次 → 再不过即失败」，并由 2.2/2.3 的机械反馈（真实变更证据）降低
  发生面。

### 2.2 模板末步固定「递交/完成」+ 黑板机械交付状态

- **模板强制**：plan 校验层要求末步为固定 terminal step（如
  `deliver`/`submit`），机械拒绝不合法末步（plan_write 校验）。
- **不随普通订单自动推进**：普通订单绑定末步不产生 done；末步仅允许
  **显式递交路径**推进（见下），杜绝 `echo done` 式虚假递交。
- **递交路径（最小化、无声明）**：
  1. 模型以最简动作请求递交（无参 `submit`，或复用 `console_step_done`
     末步特例；具体形态实施时定，二者择一）；
  2. harness 机械计算**交付状态** = 过滤 `.gsa`/临时文件后的工作区变更
     清单（数据源复用 `workspace_delta` 基础设施；上限 20 条 + 计数行，
     含增删改类型与截断标记）；
  3. 状态渲染进黑板 plan 末步状态行（如
     `[delivery] 状态: 2 个变更 (mystery.c A, output.toml A)`）——
     **工具输出、不进模型内容流，无复读触发风险**；
  4. 模型看到状态后确认（同动作再触发一次）→ 进入最终回答流程
     （引用校验 + 反例门）。
- **复读风险说明**：工具参数与渲染状态均不进复读检测器（工具参数仅标记
  可见输出、状态是工具输出）；冗长模型声明才是风险源，本设计从源头消除。
- **边界**：无文件型任务（纯分析/回答即交付物）显示"无文件变更"，仅作
  信息展示、不作硬门；变更正确性由验证器（黑盒外）仲裁，交付状态只保证
  "有实际变更产物"（覆盖"声称完成但未写文件"的幻觉缺口）。

### 2.3 订单反馈增强（落实确认）

- **编辑类订单（search_replace）**：响应回显改动 diff（old/new 区域或
  统一 diff），有界截断 + 截断标记（复用 8K/50K 注入预算纪律）。
- **终端/运行类订单**：receipt 挂 `workspace_delta`（文件清单 + 增删改 +
  大小 + 截断标记；过滤 `.gsa`）。
- **常驻 actions 板不扩载荷**（缓存命中率约束）：最多加 `changed: N files`
  短计数；完整 diff/delta 走点读 receipt。
- **终端内容级 diff 第一版不做**（需存改前内容比对，成本高）；"文件级
  delta + 命令输出"已消除大部分重复核查。

### 2.4 引用校验有界修正机会

- **现状**：最终回答引用校验失败 → 硬阻断，模型文本永不提交，无修正
  （agent_loop L1844 起）。
- **定案**：失败时返回结构化失败报告（`reason_codes` + `markers`，
  现成），给**一次修正机会**（有界 1 次）；修正轮模型可见失败原因并重写
  最终回答；重试不重置计划/步骤/run 状态；同一最终回答同一失败出现
  2 次（初始 + 1 次修正）恢复硬阻断；journal 记录 attempt（
  `citation_validation` 事件扩展）；修正后重新走完整最终回答门（引用校验
  + 反例门）。
- **与 2.2 衔接**：提前/虚假递交 → 最终回答门拒绝 → 修正机会一次，
  组成"递交-门-修正"闭环。
- **暂缓**：终端输出内容登记引用（命令白名单/行归属/内容哈希绑定，成本
  高、本轮无必需性）；如后续真实需求出现再单独设计。

## 3. 实施路由

S1 代码 → S2 测试 → S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 复验。
计数：设计轮不动（28）；实施放行入账 28 → 29；S3/S4 验证闭环 29 → 28。

## 4. 验收标准（DoD）

- S2 全绿、fmt 干净、clippy 与基线一致；
- S3 重建成功、三件套时间戳更新；
- S4 定向复验（G4/G5 或构造形态）：计划不再出现"完成但计算未执行"空转
  （对照 mteb 形态）；末步递交走机械交付状态并进入最终回答门；引用失败
  可修正一次、二次同失败硬阻断；编辑类订单回显 diff、终端类订单挂
  delta；零真实 400；命中率 ≥90%。

## 5. 风险与回滚

- 末步模板强制可能约束非常规计划 → 豁免白名单或由验证器兜底（实施时
  评估）。
- 订单反馈增大 receipt 载荷 → 有界截断 + 短计数控制；可回退为现状。
- 修正机会增加一轮成本 → 有界 1 次，可关。
- 交付状态误渲染（.gsa/临时文件过滤遗漏）→ 过滤规则单测覆盖。

## 6. 实施记录（S1 代码 + S2 测试闭合，2026-08-23 用户放行）

### 6.1 S1 代码落地

- **计划步语义重定义**：`PLAN_FIRST_FRAMEWORK_BLOCK`（orz-assurance
  `plan::framework`）步骤语义改写——步骤=执行顺序标记、完成=该批订单已
  执行、目标交交付门仲裁、末步固定「递交/完成」（id 用 deliver/submit）；
  `plan_write` 工具描述同步（末步 terminal 强制）。
- **末步模板强制**：`parse_and_validate_plan` 校验最后一步 id ∈
  {`deliver`, `submit`}，不合法末步机械拒绝（refill 错误含形状说明）；
  `TERMINAL_STEP_IDS` 常量。
- **不随普通订单自动推进**：`record_console_receipt` 对末步订单成功不再
  `mark_step_done`（保持 in_progress，仅显式递交推进）；`console_step_done`
  对末步显式拒绝（`console_step_done_terminal_step`），direct 证据门不可
  绕过递交门。
- **递交路径**：新增无参 `submit` 工具（console 默认态主车道，
  `is_console_surface_tool` 白名单 + 声明面注入）。两阶段：首次调用机械
  计算交付状态（工作区 walk 相对计划批准基线的 delta；基线在计划批准时
  经 `host.workspace_snapshot()` 捕获，同 plan_id 修订不重置）渲染进黑板
  plan 末步状态行（`[delivery] 状态: N 个变更 (…, 截断标记)`）并置 pending；
  再次调用确认置末步 done、进入最终回答流程。非当前步骤递交拒绝
  （`submit_not_current`）；无计划拒绝（`submit_no_plan`）。
- **订单反馈增强**：workspace delta 单源化迁入 orz-loop `host.rs`
  （`DELTA_EXCLUDED_DIRS` / `RUN_TESTS_DELTA_MAX_ENTRIES` /
  `DELIVERY_DELTA_MAX_ENTRIES` / `TOOL_DELTA_MAX_ENTRIES` /
  `workspace_delta_walk` / `workspace_delta_diff`；`WorkspaceDeltaEntry`
  补 size 字段）；orz-host 对 `run_terminal_cmd` 前后 walk 附加 delta、
  `run_tests` 与通用工具路径透传 `ToolResult.workspace_delta*`；console
  receipt 编辑类订单（search_replace）回显 diff（old/new 区域、单侧 4K
  截断 + 标记）、终端/运行类挂 workspace_delta + truncated；actions 板
  只加 `changed: N files` 短计数（完整内容走 receipt 点读）。
- **引用校验有界修正**：首次失败 journal
  `citation_validation{decision:retry, attempt:1, correction_allowed:true}`
  并把失败报告作为用户消息注入（injected block 前缀不变、不持久化），
  模型重写后重新走完整最终回答门；同失败第二次恢复硬阻断
  （decision:block / attempt:2 / correction_allowed:false）。Schema 与
  verifier 同步。

### 6.2 S2 测试闭合

- planning：末步 terminal 校验（deliver/submit 接受、非 terminal 拒绝）。
- controller：submit 两阶段（状态渲染 + 确认置 done）、submit_not_current
  拒绝、console_step_done 末步拒绝、search_replace diff receipt +
  `changed: 1 files`、run_terminal delta receipt（path/kind/size/truncated）
  + `changed: 1 files`、引用修正一次后通过 / 二次硬阻断。
- 既有用例适配：plan fixtures 末步改 `deliver`；console_step_done 正向用例
  目标改为非末步；步骤门用例断言末步普通订单后保持 in_progress。
- orz-loop 574 通过 / 0 失败 / 3 ignored；fmt 干净；clippy 无新增告警。

### 6.3 边界登记

- 交付状态过滤复用 `DELTA_EXCLUDED_DIRS`（.gsa/.git/缓存目录等），未另设
  临时文件后缀规则（"临时文件"= walk 排除面语义）；过滤规则由单测锁定。
- `submit` 双阶段确认时重新计算交付状态（确认内容为最新快照）。
- 引用修正机会按 run 计数（不跨 run）；修正轮仍走完整最终回答门。
