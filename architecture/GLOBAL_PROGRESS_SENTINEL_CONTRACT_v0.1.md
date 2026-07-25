# Global Progress Sentinel contract v0.1

状态：设计级、no-model-first、protocol-compatible。它增加一个由结构化步骤留痕派生的全局回看层，不改变
protocol v0.1 的 task/action/evidence 状态、gate decision 或 reason-code 语义。

## 1. 要解决的问题

长任务中的模型容易在一个局部方向上持续获得“进展感”，同时遗忘最初目标、尚未覆盖的验收维度、用户限制、
验证债务或更高优先级的并行方向。仅在 system prompt 中加入“请回看全局”不够可靠，因为模型可以复述提醒，
却没有把回看结果与实际步骤记录对齐。

本层称为 **Global Progress Sentinel（GPS）**。它不是第二个 Agent、planner 或科学 reviewer，而是一个确定性控制面：

1. 从 task contract、计划项和 append-only journal 生成紧凑 progress snapshot；
2. 用显式、可配置规则产生 WARN；
3. 把 snapshot 和 WARN 注入下一次模型回合；
4. 要求模型提交结构化 disposition，再决定继续、转向、延期、停止或重排计划；
5. 把 snapshot、prompt digest 和 disposition 作为派生产物登记，供后续 verifier 重算。

## 2. 复用的现有来源

本设计只读取本仓库现有事实源：

- [`PROTOCOL_DRAFT_v0.1.md`](../protocol/PROTOCOL_DRAFT_v0.1.md) 的 task lifecycle、task contract、
  planned/completed counts、GateDecision 和 append-only journal 约束；
- [`agent-protocol-v0.1.schema.json`](../protocol/agent-protocol-v0.1.schema.json) 的 `planned`、`reviewing`、
  `warn`、`scope`、`review_required` 等现有形状；
- [`run-event-v0.1.schema.json`](../runtime/run-event-v0.1.schema.json) 的 hash-chain run journal；
- Grok ACP 的 plan、tool-call、permission 和 session updates，经 LIF bridge 脱敏后的结构化投影。

旧研究工作区的 `index/map/self-check` 不成为 GPS 的运行依赖；具体 claim 任务仍按需跨文件读取并单独登记来源。

## 3. 不变量

### 3.1 Journal 是事实源，summary 是派生视图

GPS 只能总结有 event/plan/contract reference 的内容。它不得从最终回答、模型语气或时间邻近推断“已完成”；
无法关联的状态写 `unknown`。模型自己声称完成，若没有 terminal/artifact/verifier 支持，只能列入
`reported_unverified`。

### 3.2 不记录 private chain-of-thought

snapshot 只含目标、步骤状态、方向覆盖、artifact/verifier 状态、限制和 warning。模型 disposition 只要求决策、
短理由和下一步，不要求展示隐藏推理过程。

### 3.3 WARN 不是自动 BLOCK

方向集中可能是合理的。GPS 允许模型选择 `continue`，但必须指出关联 acceptance、仍未覆盖的方向和退出条件。
只有 warning 同时命中既有 hard gate 时，才由原 GateDecision 产生 `block`/`defer`；GPS 不自创绕过权限或证据
门禁的新优先级。

### 3.4 回看不能改写历史

snapshot 和 disposition 都是新派生产物；不能回写旧 plan/event。计划改变必须追加 revision reason，并保留被
延期或放弃的方向。

## 4. 输入与派生快照

### 4.1 最小输入

- immutable task contract digest：intent、MUST、MUST NOT、SOURCE OF TRUTH、ACCEPTANCE；
- 当前 task state 与 plan revision；
- plan items：稳定 `step_id`、`direction_id`、状态、acceptance refs、defer reason；
- 自上次 review 以来的 journal span、action terminal、artifact registration、verifier/gate result；
- 未解决的 user constraint、unknown、warning 和 blocker；
- 当前资源边界：token/time/tool budget 只记配置与已观测用量，不猜剩余能力。

`direction_id` 是工程覆盖标签，不是科学分类。例如当前仓库可用 `runtime_acp`、`windows_containment`、
`deepseek_conformance`、`recovery`、`evidence_gates`、`documentation`。标签必须来自计划，GPS 不让模型事后为
自己的动作挑有利分类。

### 4.2 Snapshot 输出

```json
{
  "review_id": "GPR-001",
  "task_contract_sha256": "...",
  "plan_revision": 3,
  "journal_span": {"first_sequence": 41, "last_sequence": 67},
  "objective": "short contract-derived text",
  "progress": [
    {"step_id": "STEP-ACP-01", "direction_id": "runtime_acp", "state": "completed", "source_refs": ["EVT-052"]}
  ],
  "acceptance_coverage": [
    {"acceptance_ref": "ACC-03", "state": "uncovered", "source_refs": []}
  ],
  "unresolved": [],
  "warnings": [],
  "next_review_trigger": "before_external_or_costly_action"
}
```

`objective` 可以由 contract 文本机械裁剪；任何模型生成的自然语言摘要必须标记 `derived_unverified`，不能替代
上述结构字段。

## 5. WARN 规则

| warning kind | 最小机械条件 | 防误报条件 |
|---|---|---|
| `direction_concentration` | 最近可配置数量的 completed/running actions 均属于同一 direction，且其他 active direction 仍 pending | 若当前步骤是明确 critical path，允许 continue，但登记退出条件 |
| `pending_direction_starvation` | 某 active direction 跨过可配置数量的 review 周期仍无 action/artifact，且未显式 defer | 已写 defer reason 或不在当前 acceptance scope 时不报 |
| `plan_stale` | journal 出现未映射 action，或已完成步骤仍被计划标为 pending | 仅 UI/展示延迟不得自动提升为任务失败 |
| `acceptance_uncovered` | 某 acceptance 没有 planned/completed step 或 verification source | 未到适用 phase 时保持 info/warn，不提前 block |
| `verification_debt` | 连续产生写入/产物但没有对应 verifier/gate，或 verification 已 stale | 探索性 disposable fixture 可按 contract 延期，但必须可见 |
| `scope_drift` | action direction、target 或产物不映射 task contract/plan revision | 只读诊断可解释后继续；新写入/外部副作用走现有权限门禁 |
| `repeated_failure` | 同一 idempotency/action family 连续失败或 retry 达阈值 | 输入、实现或环境已发生有来源的实质变化时重置计数 |
| `unresolved_user_constraint` | 用户明确限制仍未映射到 MUST/MUST NOT/plan/gate | 只有用户随后明确撤回才清除，不靠模型推断 |

默认阈值不是科学常数，必须冻结在 run manifest；不同阈值只能改变提醒频率，不能改变 evidence/claim 状态。

## 6. 触发时机

为避免 token 与 warning fatigue，GPS 不在每个 tool call 后注入 prompt。v0.1 只在下列边界运行：

1. `planned → ready` 前；
2. 一个计划步骤完成、延期或失败后；
3. 达到 action-count/review-cycle 阈值时；
4. 多次失败、计划外 action 或方向饥饿出现时；
5. 任何 destructive、外部网络写入、真实付费模型调用或大范围 restore 前；
6. `executing → reviewing` 和 `reviewing → completed` 前。

如果没有新 event，GPS 不重复生成同一 warning；使用 `(task contract digest, plan revision, journal head,
warning kind)` 作为去重键。

## 7. 注入给模型的最小提示

```text
[GLOBAL PROGRESS REVIEW]
Objective: <contract-derived>
Completed since last review: <source-linked steps>
Still active/uncovered: <directions and acceptance refs>
Warnings: <mechanical warning + evidence refs>
Before continuing, return one disposition:
continue | pivot | defer | stop | replan
Include: selected next step, acceptance served, unresolved directions,
verification needed, and the condition for reviewing again.
Do not infer completion from this summary; consult referenced records when needed.
```

该提示不包含 raw reasoning、secret、完整工具输出或 reviewer-only/evaluation oracle。若 snapshot 超过预算，优先保留
warning、uncovered acceptance、unresolved user constraint 和 source refs；已完成详情可以只留 count/digest。

## 8. Disposition 与执行约束

模型必须返回：

- `decision`: `continue | pivot | defer | stop | replan`；
- `selected_step_id` 或 `null`；
- `acceptance_refs`；
- `warning_dispositions`: 对每条 warning 选择 `accepted | mitigated | reasoned_continue | needs_user`；
- `verification_before_claim`；
- `next_review_condition`；
- 不超过固定长度的 `rationale_summary`。

缺失 disposition 时，框架不能把 review 标为已处理；普通低风险工作最多保持 WARN，外部/破坏性/claim promotion
仍由既有 gate fail closed。模型若选择 `reasoned_continue`，后续 action 必须仍属于所选 step，或者先追加 replan。

## 9. 与现有 protocol 的兼容方式

v0.1 不扩展 `RunEvent.event_type` 枚举，也不把 heuristic warning 冒充 `GateDecision`：

1. 生成 `global-progress-review.json` 和 `global-progress-disposition.json` 两个派生产物；
2. 通过现有 `artifact_registered` event 登记路径、digest、journal span 和 producer；
3. 下一次 `model_request` 只记录注入 prompt digest 与 review artifact ID；
4. warning 若确实命中已有 reason code，另走正常 `gate_decision`；否则只留在 review artifact；
5. verifier 从 contract + plan ledger + journal 重建 snapshot 的结构字段，并检查 prompt/disposition linkage。

这避免为了一个提示层修改 protocol v0.1，同时保留未来把它升级为通用 framework event 的空间。

## 10. 首个 implementation spike

先做 no-model fixture，不接 Grok、DeepSeek 或用户真实 workspace：

1. 固定一个含六个 direction 的 task contract/plan；
2. 构造“单方向连续完成、另一 acceptance 未覆盖、一个 verifier stale”的 append-only fixture journal；
3. 机械生成 snapshot，断言三类 WARN 及 source refs；
4. 输入 `reasoned_continue` 与 `replan` 两种 disposition，验证 linkage、去重和下一 review 条件；
5. 篡改 plan revision、journal head 或 warning disposition 时 verifier 必须失败；
6. 通过后才评估如何从 Grok ACP plan/session updates 建立稳定 `step_id`/`direction_id` 映射。

本 spike 只证明留痕、提醒和处置链可重放；不证明模型一定会克服 tunnel vision，也不把方向多样性当成正确性。

当前进度：上述 no-model fixture 已完成。六方向输入机械产生 `direction_concentration`、
`acceptance_uncovered`、`verification_debt` 三类 source-linked WARN；`reasoned_continue` 与 `replan` 两种 disposition
均通过独立重建，plan revision、journal head、warning coverage 与 review digest 篡改均失败。详见
[`GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md`](../docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md)。

## 11. 跨检查点整体性扩展

单次 snapshot 只能回答“现在是否集中”，不能区分短期关键路径和长期单方向发散。整体性扩展因此读取最近若干个
GPS checkpoint 的 hash-chain 投影，并只使用可观测量：

- 每个 direction 的已终止 action count，而不是猜测 token、认知努力或剩余能力；
- 新登记的 evidence ref 与新通过的 acceptance ref；
- 显式 `continue/pivot/defer/stop/replan`、selected direction 和延期方向；
- 当前关键 acceptance 状态、未解决用户限制与是否请求整体完成。

它派生四类 warning：

| warning kind | 跨检查点条件 | 防误报/解除条件 |
|---|---|---|
| `direction_budget_dominance` | 窗口内一个方向的 action 占比达到冻结阈值，同时其他 active critical direction 存在 | 占比只是提醒；可通过有界聚焦处置继续 |
| `evidence_stagnation` | 同方向连续 `continue` 达阈值，且没有新增 evidence ref 或 verified acceptance | 任一有来源的新证据会打断连续计数，但不会自动清除预算集中 |
| `critical_direction_deferral_debt` | 同一 critical direction 连续多个 checkpoint 被显式延期 | 必须实际结束延期；改写理由或计划 revision 不清零 |
| `local_pass_global_incomplete` | 请求整体完成，但仍有关键 acceptance 非 `verified` 或用户限制未解决 | 局部 verifier PASS 不能抵消其他 blocker |

### 11.1 整体完成门

整体完成门输出 `not_requested | eligible | ineligible`。它不创造新的科学结论，也不把 heuristic WARN 冒充
protocol GateDecision；但 runtime 在接入时不得将 `ineligible` 当作完成授权。`ineligible` disposition 必须明确
`completion_withheld`，从而区分“停止继续动作”和“宣称任务已整体完成”。

### 11.2 有界聚焦 permission reversal

方向集中有时是正确的关键路径，因此 `reasoned_continue` 不要求机械轮转。对预算集中或证据停滞继续推进时，处置
必须同时给出：

1. 当前 critical path reference 与所服务的 critical acceptance；
2. 可机械计数的最多追加 action 数；
3. 明确退出条件；
4. 不晚于下一 review cycle 的复查期限；
5. 被该聚焦窗口挤出的其他 active critical direction。

缺少任一项时 verifier fail closed；满足这些条件只授权一个短窗口，不证明所选方向正确。

### 11.3 来源边界

checkpoint 记录 prior review/disposition digest 并自身形成 hash chain，可检测历史投影被静默改写。当前 no-model
spike 尚未同时接收 prior artifact 文件，因此只能验证 digest 被稳定引用，不能证明 checkpoint 中的投影与原始
review/disposition 语义一致。正式 runtime adapter 必须从已登记 artifact 和 journal 机械生成 checkpoint，不能由
模型自由填写。

### 11.4 来源绑定 checkpoint adapter

no-model adapter 已实现上述来源约束。每个新 checkpoint 必须同时绑定：

- 原始 GPS input、确定性 review、模型 disposition 与既有独立 verification report；
- task/contract-bound critical-direction policy；
- 前一 checkpoint（第二周期起）；
- 若存在有界聚焦：产生授权的 history、holistic review/disposition/verification 四件套。

adapter 不只读取 verification 中的 `valid=true`，还重新运行既有独立 verifier，并要求重算报告与输入报告完全一致。
critical-direction policy digest 必须与前一 checkpoint 相同；当前版本没有 policy revision protocol，因此任何变化都
fail closed。

action budget 统计前一 journal head 之后该授权方向的所有 `action_terminal`，包括成功、失败与取消，防止失败重试
绕过预算。等于 `max_additional_actions` 仍合法；超过动作上限或 `review_by_cycle` 时，adapter 会保留带
`within_* = false` 的 checkpoint 作为审计证据，同时进程返回非零，独立 verifier 也将整体结果标记 invalid。

实现与实测见
[`GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`](../docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md)。
checkpoint adapter 与 prior-artifact linkage 的 no-model spike 已完成，见
[`GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`](../docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md)。
在 runtime event integration 和 policy revision protocol 完成前，仍不把本扩展注入真实模型回合，也不把它用于
LIF 专有任务替代通用复杂任务测试。

### 11.5 Disposable transition gate

transition gate 已把 checkpoint/holistic verification 接到 `executing → reviewing` 与
`reviewing → completed` 前。结果写成现有 `run-event-v0.1` 的 `gate_decision`：

- pass 才令 `state_after=requested_state`；
- block 必须令 `state_after=state_before`，但仍保留事件；
- completion 重新运行 holistic 独立 verifier，并要求 history head 与当前 checkpoint 完全一致；
- event 只有在 run、manifest、sequence 与 previous digest 精确延伸现有 journal 时才能追加，追加后立即重放。

runtime-local `GPS-*` control code 尚未注册为 protocol reason code，不能对外伪装为正式 GateDecision reason。详见
[`GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md`](../docs/GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md)。

### 11.6 Atomic journal boundary and reason-code migration

所有仓库内已知 journal writer 必须使用同一个 `<journal>.lock` sidecar，并在一把独占锁内完成：

1. replay 现有 journal；
2. 校验 terminal/run/manifest/sequence/previous digest；
3. 追加一条完整 canonical JSON line 并 `fsync`；
4. 必要时在释放锁前重放。

锁文件作为稳定 inode 保留，不能在释放后删除；删除会使已等待旧 inode 的 writer 与新建 lock file 的 writer 分裂。
该锁是同机协作进程的 advisory boundary，不宣称对不遵守锁的外部 writer 或网络文件系统提供多主一致性。

冻结的 [`reason-codes-v0.1.yaml`](../protocol/reason-codes-v0.1.yaml) 不因 runtime 集成而原地扩写。
[`global-progress-reason-code-migration-v0.1.yaml`](../protocol/global-progress-reason-code-migration-v0.1.yaml)
只记录三种迁移状态：pass 不产生 protocol reason、稳定的新语义进入下一修订候选、聚合失败从独立 verifier
投影已有细粒度原因。候选码在正式 protocol revision 接纳前不得写入 claim-bearing GateDecision。

### 11.7 Torn-tail recovery boundary

journal recovery 默认是只读检查，且与 writer 使用同一 sidecar lock。只有以下两种状态可进入显式修复：

- 最后一个完整事件只缺最终换行，补换行后整条链可重放；
- 最后一段无换行字节不是有效链成员，但它之前至少一个完整事件组成的前缀可独立重放。

包含中间坏行、空白行、错误 digest/sequence/run/manifest、无有效前缀或以换行结束但无效的 journal 一律不可自动
截断。torn bytes 必须先原样写入不覆盖的 quarantine；修复后的非终态链追加 metadata-only recovery event，
终态链不得追加事件，只能由外部恢复收据记录。journal 通过同一锁内的原子替换更新，随后必须重新 replay。

此机制处理进程中断留下的 JSONL 尾部残片，不宣称提供磁盘控制器级断电持久性、目录项 fsync 保证或多主恢复共识。

### 11.8 Disposable runtime controller

controller adapter 只组合既有机械边界，不引入模型调用或新的科学判定：

1. 从 transition request 与 source-bound checkpoint/holistic artifacts 纯函数式重建 gate event；
2. 在共享锁内重放 journal，并验证 run、manifest、sequence、previous digest 与 terminal 状态；
3. pass 和 block event 都必须入链，状态只从已入链 payload 的 `state_after` 投影；
4. 精确相同且仍位于链尾的 event 重试返回 `already_recorded`；不同内容复用 event ID、旧 event 后已有新事件或陈旧
   链头均返回 conflict；
5. journal 尾部损坏返回 `recovery_required`，历史损坏返回 `journal_unrecoverable`，controller 不自动调用恢复；
6. event 已追加但外部 controller receipt 尚未写出时，重试可从相同 event hash 收敛而不重复追加。

该 controller 仍是 no-model disposable fixture。它不替代成熟 CLI 的 session/runtime controller，也不建立实际进程状态、
模型回合、用户确认或外部副作用的生产级一致性。

### 11.9 Controller receipt verification

controller verifier 必须只读，并在共享锁内观察 journal。它重新运行 transition builder，独立计算所有输入文件 digest，
并检查 receipt schema、候选 event、journal hash/count、事件唯一性、链尾状态与 `state_before/state_after` 投影。

若 event 已追加而 controller receipt 因崩溃未写出，原 request 重试必须得到链尾 `already_recorded` receipt；verifier
随后应通过。若 journal 已继续推进，旧 receipt 的 journal hash/count 必须失效。receipt 内容篡改不得通过状态投影检查。

最终 journal 快照无法证明 event 是本次 controller 刚追加还是此前已经存在，因此 verifier 不能把 `event_appended`
当作独立可重建的历史事实；它只验证该声明与 receipt schema、候选状态和当前链快照不矛盾。

### 11.10 Journal-derived controller state

controller 不再把 transition request 的 `current_state` 当作状态权威。状态归约使用同一完整 journal，并采用以下冻结规则：

1. 一个且仅一个 `run_started` event 将 controller 初态锚定为 `executing`；缺失、重复或 transition 早于该锚点均 fail closed；
2. 只识别 `gate_decision` 中绑定 `global-progress-transition-receipt-v0.1.schema.json` 的 GPS transition；
3. 每条 receipt 必须通过 schema 与 pass/block 语义检查，task ID 必须稳定，`state_before` 必须等于上一归约态；
4. block event 保持状态，pass event 才前移；`completed` 后不得再出现 GPS transition；
5. reducer 输出当前状态、最后 transition 与最近一次 gate 实际接受的 checkpoint binding。后者只证明 gate 接受，不是科学
   正确性证明。

appender 在共享独占锁内先 replay、再归约既有状态，并将 candidate 的 task/state 与归约结果比较；append 后再次 replay 与
归约。精确链尾重试先验证既有归约结果，再返回链上当前状态，因此调用方无法用自报 `current_state` 跳过
`executing → reviewing → completed`。独立 controller verifier 使用同一确定性 reducer 复核 success 与 state-conflict
receipt，但不复用 controller 的写入路径。

该规则把 `run_started` 作为控制器状态机的显式协议假设；它不声称从模型行为或外部 CLI session 中推断状态。成熟 CLI
适配必须保证真实 session 的启动事件与该锚点语义一致，或在未来 protocol revision 中引入更细的 bootstrap event。
实现与反例见
[`GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`](../docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md)。
