# 机械审查层＋半助理层重设计（2026-08-24：首轮 plan 门保留 + direct 执行面 + 静默审计 + 检索恢复 + 引用校验器删除）

> 状态：`current-design`（2026-08-24 设计定稿；S1 实施 + S2 测试 + 全面
> 审查处理已闭合——read-anchor 补 direct 面核证门、退役工具调用面窄门、
> cmd 摘要格式修复、候选错误码透传、订单链测试迁移；S3-S4 待实施）。
> 性质：取代 PLAN-FIRST-BLACKBOARD（2026-08-15）的 console 订单执行面与
> FUS-CONSOLE-DUAL-MODE 的默认路径；保留首轮 plan 门、ACAF、权限轴、
> 预算/候选计数、read-anchor 写前核证；删除输出级引用校验器；恢复主面检索。
> 关联：ADR-0010 §14.39（v1.39）/ BACKLOG 0g / TODO P0-0g /
> CLI_PROJECT_INDEX v2.19。

## 1. 来源与问题

### 1.1 外部对照：同模型 lean harness 显著领先

- DeepSeek 官方：V4-Flash-0731 Terminal Bench 2.1 = **82.7**（DeepSeek
  Harness，89 题全集）。
- 2026-08 四 harness 对照（Maka-Agent PR #2208，同一 DeepSeek V4 Flash、
  同一执行器/预算/镜像）：Codex harness pass@1 = **70/89（78.65%）**、
  Maka 73.03%；60 个未耗尽预算的题上三家 87%–90%；Maka 中位单题
  332.8s。
- 本机同模型 k=1：10 题冒烟 5 通过（50%），完成试次单题 5–21 分钟，
  video-processing 60 分钟耗尽官方 3600s 预算。

### 1.2 本机实机量化：压力来自助理层订单仪式，不是 plan

- 每完成一个真实工作工具需 **2.3–2.9 个模型往返**（git-multibranch 2.85、
  mcmc 2.87、portfolio 2.52、sam-cell-seg 2.28）；模型往返均值 ~12s，
  即每个真实动作被黑板上读计划/下订单/读 receipt/权限票据摊薄到 ~36s。
- plan_write 每轮仅 1–4 次（工具调用 2–5%）；重负集中在
  blackboard_read（8–90 次/轮，26–33%）+ blackboard_action_write
  （5–103 次，20–35%）+ 每单一对 permission/ticket + receipt 点读。
- 22 次订单拒绝（12 invalid_arguments / 8 step_not_done /
  2 policy_denied）各消耗拒绝轮+重试轮。

### 1.3 检索缺口（设计实现偏差）

- console 主面模型可见工具集为黑名单式固定列表，web_search/web_fetch/
  browser_read/retrieve_project_* 均不可见；订单注册表也无任何检索动作。
- 10 试次检索类调用 **0 次**；mteb-retrieve（明摆着要检索的题）全程本地
  读 data.txt 猜答案，写出错误句子。plan-first 阶段 C 隐藏检索工具后
  未给主 agent 留任何触发子代理的路。

### 1.4 引用校验器与终端/文件型工作流错配

- 8 个完成试次 5 次触发引用校验、3 次硬 block 后 run 直接 completed
  且最终回答被吞（portfolio/break-filter/mteb）；TB verifier 不读最终
  回答，机械校验零收益、净消耗轮次。
- 模型在 console 工作流下自然引用「ORD 终端输出」「写入的文件:行号」
  「结果栏 receipt」，与校验器语法（source_id / observed path / 精确
  receipt 格式）频繁不匹配。

### 1.5 读范围门（P1）在 benchmark 下可绕过且拦截合法读取

- allow_shell 下 run_terminal_cmd 本可 cat/sed/grep 任意路径，读范围门
  不构成安全边界；却拦截了 site-packages（mteb）、/tmp 反汇编产物
  （path-tracing）等合法读取，模型直连+订单双路径各试一次再退回终端。

### 1.6 用户裁决过程（2026-08-24 多轮讨论定案）

1. 读范围放开、朝通用改，仅保留 `.gsa` 证据面不可见、16KB 信封、
   head_limit。
2. 引用校验器整体删除（通用场景同样无必要）。
3. ACAF 值得留（保留前置硬门）。
4. 仅保留部分 plan——**首轮 plan 门形态不变**（处理 deepseek 不同思维链
   的「极简模式」问题）。
5. 助理层拆为**机械审查层＋半助理层**：静默审查；适合助理干的转给助理
   （如命令运行本身）；值得留的依旧维持硬门。
6. 机械审查层不给建议、只给事实；每对象仅保留最后一轮改动的审查结果，
   覆盖着写；审计报告随**最终答案前的中立问询轮**一并注入。
7. round 2+ 不要求动作绑定 step_id。
8. read-anchor 写前核证保留（只是机械填写）。
9. 合并为一个 S1 实施包（读放开 + 引用删除 + 检索恢复 + direct 面 +
   审计层）。

## 2. 定案设计

### 2.1 总形态（三轮模型面演化）

- **第 1 轮（不变）**：plan-first 硬门——模型面仅 blackboard_read +
  plan_write；校验失败重填 1 次，仍失败机械降级并留痕（plan_not_submitted /
  validation_failed）；accepted 或 degraded 后进入第 2 轮。
- **第 2 轮起（direct 执行面）**：模型直接调用工具，一次调用一个往返，
  工具结果即时返回；不再有「下订单 → 读回执」仪式。
- **半助理层（执行背板）**：run_terminal_cmd / search_replace / run_tests /
  web 检索派发仍经 host 侧机械链前置执行——host 拥有 cwd/env/超时；
  ACAF 票据、权限轴、预算/墙钟、候选计数、web_search 并发=1 全部维持
  fail-closed 硬门。
- **机械审查层（静默审计）**：运行中只记录不注入；每对象键仅保留最后一轮
  审查结果（覆盖写）、不给建议；最终答案前中立问询轮与
  [COUNTEREXAMPLE_GATE] 一并注入 [MECHANICAL_AUDIT v0.1] 报告。
- **交付**：submit 保留（信息展示、非硬门）；终答前中立问询（counterexample）
  保留；输出级引用校验器删除。

### 2.2 机制归属表

| 机制 | 归属 | 说明 |
|---|---|---|
| 首轮 plan 门（refill/degrade） | 硬门保留 | 第 1 轮形态不动；钉死极简模式 |
| ACAF 票据链 | 硬门保留 | 变更/shell/网络前置；用户裁决 |
| 权限轴（shell/network/MCP deny） | 硬门保留 | 授权面本身 |
| 预算/墙钟/轮数 | 硬门保留 | 纯计数 |
| web_fetch 候选上限、web_search 并发=1 | 硬门保留 | 纯计数 |
| run_tests host-owned 命令 | 硬门保留 | 模型不提供命令 |
| read-anchor 写前核证 | 硬门保留 | 机械填写、零正常路径摩擦 |
| run_terminal_cmd 执行 | 半助理层 | host 拥有 cwd/env/超时；ACAF/权限前置 |
| search_replace 执行 | 半助理层 | ACAF + anchor 硬门 |
| web 检索派发 | 半助理层 | 子代理路由；候选/并发/模式门 |
| step 绑定/顺序门 | 转事件留痕 | round 2+ 不要求 step_id；只留痕、不上报告 |
| 订单契约预校验 | 退役 | 参数错误由工具自身返回 |
| receipt 点读强制 | 删除 | 工具结果即返回 |
| plan 步骤 done 推进 | 转审计+模型自推进 | plan 退化为方向与状态展示 |
| 输出级引用校验器 | 删除 | 含提示词「机械校验/阻止交付」表述 |

### 2.3 半助理层细则

- `workspace.run_terminal` 语义保留为模型给命令文本、host 决定环境与
  cwd、超时/后台契约不变；模型面直接暴露（不再经订单层）。
- `workspace.search_replace` 语义保留（唯一精确替换、空 old_string 新建、
  可选 expected_anchor 写前核证）。
- `workspace.run_tests` 保持 host-owned 固定命令。
- web 检索：模型直接调 web_search/web_fetch（及 browser_read /
  retrieve_project_*），relay 路由到子代理；候选计数、并发=1、模式门、
  ACAF 前置不变。

### 2.4 机械审查层细则

- **数据**：运行内审查表，以对象键为单位：
  `file:<path>`（search_replace 结果：matched/created、diff 摘要）、
  `cmd:<call_id>`（exit code、timeout、stdout 截断）、
  `plan`（accepted/degraded、步骤数、最近修订）、
  `budget`（轮数/墙钟用量）、`retrieval:<n>`（候选/上限）。
- **覆盖写**：同一对象键新结果覆盖旧结果；报告时每键至多一条；容量上限
  128 条，超限丢弃最旧键（保证每键保留最新）。
- **报告内容（收敛为执行事实摘要，2026-08-24 用户裁决）**：仅三类——
  ①执行事实摘要（动作清单、文件改动与 diff 规模：`file:<path>` /
  `cmd:<call_id>` 最新结果）；②预算（轮数/墙钟用量）；③异常事实
  （exit code、超时、锚点拒单、检索候选超限）。仅机械事实，无建议、
  无引导（例：`file:/app/result.txt → search_replace 成功（1 处，
  diff +3/-1）`；`cmd:ORD-000012 → exit 1（120s 超时）`；
  `budget → 已用 84/999 轮`）。
- **step/契约类（不上报告）**：step 顺序、契约符合性等检查结果只写
  事件留痕（mechanical_audit_update 轻量事件），不进入最终报告——
  报告是模型终答前的自检输入，不是违规说教清单。
- **静默**：运行中不注入审计反馈（硬门拒绝/工具错误照常返回）；
  报告仅在最终答案前中立问询轮注入，与 [COUNTEREXAMPLE_GATE] 同轮独立块。
- **事件面**：全部覆盖写动作（含 step/契约类）以轻量事件留痕
  （键/轮/摘要），供回放与验证；报告块不进归档（与 counterexample
  注入同语义）。
- **空转防回归**：审计记录「执行事实（工具清单 + 文件 delta）」，模型终答
  前的自查轮可据此核对「计划完成」声明——不再依赖 step_not_done 硬拦截。

### 2.5 模型面（第 2 轮起）

- 只读：read_file / list_dir / grep / search_tool（放开 cwd 范围；
  保留 16KB 信封 / head_limit / `.gsa` 不可见）。
- 执行：search_replace（可选 expected_anchor，硬门）/ run_terminal_cmd /
  run_tests。
- 检索：web_search / web_fetch / browser_read / retrieve_project_*。
- 控制：blackboard_read / plan_write（修订）/ submit。
- 退役（不再暴露）：blackboard_action_write / console_step_done /
  console_return_to_console；console 订单层（console_order_written/
  rejected 事件）不再产生于主流程。

### 2.6 读范围放开

- `access_in_scope` 删除 read/grep 的 cwd 包含性要求；保留 `.gsa` 证据面
  不可见（白名单仅 run_tests_output.txt 与 session/terminal/*.log）、
  大文件 16KB 信封、grep head_limit / files_searched 信封。
- 后果登记：不开 shell 的会话同样放开（通用方向）；越权读由权限策略轴与
  ACAF 承担；`.gsa` 之外的运行时内部文件（keystore/journal/snapshot）
  仍不可见。

### 2.7 检索恢复

- `is_console_surface_tool` / direct 投影恢复检索族；探针投影对检索族
  豁免或纳入（实施点）；route 仍派发子代理（主 agent 不直接执行检索任务）。
- 候选计数（web_fetch/browser_read）、并发=1、模式门、ACAF 前置不变。

### 2.8 引用校验器删除

- 删除：agent_loop 输出级引用门（retry/block、citation_failures 计数、
  citation_validation 事件）、prompt 的 CITATION_VALIDATION_FAILED_PREFIX /
  failure block、citation_validation.rs 模块、controller validate 调用、
  schema/fixtures/Python verifier 对应事件。
- BASE_SYSTEM_PROMPT 引用纪律段改写：去掉「由 verifier 在交付前机械校验，
  不通过即阻止交付」，改为轻量纪律（引用需绑定本 run 证据；无机械拦截）。
- 关联更新：ADR-0010 §3.7.9 / RETRIEVAL_MECHANICAL_CONTROLS_DESIGN
  step 5 / 对应审计文档标记被取代。

### 2.9 首轮 plan 门（保留，登记目的）

- 第 1 轮形态、校验、重填、降级逻辑不变；degrade 后同样进入 direct 面。
- 目的登记：强制模型起点先给目标与步骤，收敛 deepseek 不同思维链的
  「极简模式」风险（先计划后执行）。

## 3. 边界与残留

- 审计门 fail-open：坏动作先执行、报告后由模型自行纠正；最终态由外部
  verifier 仲裁（benchmark 判分语义不变）。
- ACAF/权限/预算/候选计数仍 fail-closed（用户裁决保留）。
- 中立问询轮（counterexample）保留；审计报告同轮注入；模型不递交/不终答
  时审计无出口（记录仍在事件面留痕）。
- 既有主题影响：FUS-READ-ANCHOR（硬门保留，设计不变）、
  AGENT-DELIVERY-FLOW（submit 双阶段保留；step 门转审计；
  「末步不随普通订单推进」由审计+终答流程承接）、
  PLAN-FIRST / CONSOLE-DUAL-MODE（console 订单面退役，双模式状态机简化
  为「第 1 轮 plan 门 → direct 面」）。
- 引用纪律降为提示词级：通用场景防幻觉依赖模型自律，不再机械拦截。

## 4. 实施路由与计数

- S1 代码（合并：读放开 + 引用删除 + 检索恢复 + direct 面 + 审计层）
  → S2 测试（删/改既有 gate 测试、补审计覆盖写/检索可达/引用零残留）
  → S3 重建（Linux musl，ORZ-BUILD-MOUNT-001）→ S4 复验（同一 10 题
  k=1 + 构造题：检索调用出现、审计报告覆盖写、零 400、命中率 ≥90%、
  轮次/耗时下降、reward 对比）。
- 计数：设计轮不动计数；实施放行入账 +1；S4 闭环 -1（以 TODO 未闭合
  扫描快照为准）。

## 5. 验收标准（DoD）

1. 第 1 轮 plan 门行为不变（既有 plan 门测试保持通过）。
2. 第 2 轮起模型面=direct 工具面：每动作 1 个模型往返；
   blackboard_action_write / console_step_done 不再出现在 journal。
3. 检索可达：实机/构造题出现 web_search 调用（mteb 类任务）。
4. 审计报告：最终答案前中立问询轮注入 [MECHANICAL_AUDIT]；每对象一条、
   覆盖写、无建议、容量有界；报告收敛为执行事实摘要（动作/文件 delta/
   预算/异常事实）；step/契约类仅事件留痕、不上报告。
5. 引用校验器零残留：无 citation_validation 事件/模块/提示词机械拦截
   表述。
6. read-anchor 硬门保留（既有四场景测试保持通过）。
7. 读范围放开：cwd 外路径可读；`.gsa` 仍不可见（白名单除外）；
   信封/head_limit 生效。
8. 零 HTTP 400、journal 命中率 ≥90%；10 题 reward 对比不降（目标提升）；
   单题轮次/耗时下降。
9. orz-loop 全量测试通过、fmt/clippy 干净、Linux musl 三件套重建。

## 6. 关联登记

- ADR-0010 §14.39（v1.39）/ BACKLOG 0g / TODO P0-0g /
  CLI_PROJECT_INDEX v2.19。
- 取代：PLAN_FIRST_BLACKBOARD_DESIGN（console 订单执行面）、
  RETRIEVAL_MECHANICAL_CONTROLS_DESIGN step 5（输出级引用校验器）。
