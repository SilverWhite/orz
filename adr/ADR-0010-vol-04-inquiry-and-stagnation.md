# ADR-0010 分卷 04：§4 问询与停滞机制职责

> 本卷为 [`ADR-0010：ORZ 融合运行时、同构 Agent 与设计权威重整`](ADR-0010-fusion-runtime-and-agent-architecture.md)（AUTH-ADR-0010，ORZ 当前自然语言设计的唯一权威基线）的物理分卷 04（§4 问询与停滞机制职责）；规范正文以所指分卷章节为准，分卷总目录见主文件。分卷为物理拆分、**语义零增删零改写**——本行以下正文与分卷前原文逐字一致；卷题与本声明为分卷批新增（分卷批：0ca，2026-09-28）。

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

Counterexample Gate 在正式答案前执行一次 answer 变体，并显式告知“仅出现一次”（**2026-09-24 勘误**：
plan 写入前的 plan 变体已随 §14.49 于 2026-09-01 退役——原句「在 plan 写入前执行一次 plan 变体」
为措辞残留，生产面仅存 answer 变体，与实现一致）。它只检查前提、反证和结论强度，不进入普通工具
循环、不代替 Orientation、不拥有子代理关闭权。

**2026-09-23 收窄（用户裁决；0bi ⑩，见 §14.76）**：answer 变体**只在「本 run 有执行事实
（`tool_rounds > 0`，或存在编辑/产物）或 plan 存在且未完成」时触发**；纯文本短答（无工具轮、
无未完成 plan）**跳过**。`once_only` 语义不变（plan 变体已于 2026-09-01 退役，见 §14.49）。**跑分口径不做
特殊处理**（历史成绩因模型代际更换整体失效，将重跑）。

Orientation 可以读取 checklist/blackboard 中的 current step、task position、next output target 和工具
可用性事实，但不得询问模型“是否错误、是否有偏见、是否漂移、是否卡住”，不得产生 counterexample、
claim disposition 或 hard constraint change。Checklist 是用户可见的 soft workboard，不是 hard gate。

运行活性分为四个相互独立的机械面：

| 守卫 | 默认值 | 模型可见 | 行为 |
|---|---:|---|---|
| Tool execution timeout | 300 秒，可配置 | 返回明确 timeout 结果 | `kill_active` 终止当前进程树但不闩闭后续 spawn，loop 可继续（**2026-09-04 本行已由 §14.55 第 1 项取代**：工具执行层去自身硬超时——`timeout` 只作 auto-backgrounding deadline 引用，kill-on-timeout 仅存于显式 `auto_background_on_timeout=false` 逃生阀；评测墙钟由 runner/sandbox 施加，`BACKGROUND_MAX_RUNTIME` 10h 为绝对安全兜底例外） |
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

