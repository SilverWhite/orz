# 中立问询升级：强制模板轮设计（2026-08-14）

> 状态：设计已确认（2026-08-14 用户）；实施已闭合（2026-08-15）；权威登记：
> ADR-0010 §14.13（v1.13 补写）与 §14.16（v1.16 实施登记）；实施审计见
> `docs/audits/GAP_ORIENTATION_FORCED_TEMPLATE_IMPL_AUDIT_2026-08-15.md`。
> **已退役（2026-09-01，P2-11 DC 强制模板轮清理）**：本机制整体删除
> （含 DC 族与 plan 反例变体），orientation 仅保留软门；本文档保留作
> 档案，现行语义以 ADR-0010 §14.49 为准。
> 背景证据：Terminal-Bench 2.0 探索性跑分缺口 2——中立问询触发多次但行为未矫正（path-tracing 131+ 次 read、make-doom 侦察循环）；现行为为注入文本、非硬门、循环继续（ADR-0010 §4.2；DC 同理 2→3→4→5 递进注入）。
> 关联（2026-08-14）：黑板按 plan epoch 轮换（实施未开始，见
> `BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md` / ADR-0010 §14.15）——同一任务轮内压缩
> 不再清黑板，本模板的 `task_position`/计划锚点在压缩前后稳定。

## 1. 目的定位

- 中立问询本身是拉回注意力、防跑偏与钻牛角尖的机制；计划与目的在黑板上，模型回头看并填写模板即达成主要收益。
- 验收取向：**强制表达，不验证诚实**；填表式应付不是主要风险。
- 缓解手段（必做）：`progress_evidence` 与 `missing_evidence`（声明的缺失面）与
  journal 证据身份做存在性交叉校验（found/missing 随事件记录，非阻断）；
  `next_action=gather_evidence` 必须给出缺失面。

## 2. 机制

### 2.1 触发与暂停

- 触发沿用现有规则：Orientation 按 session 级 7 个已完成逻辑模型轮 + pre-handoff；DC 按机械硬信号 2→3→4→5 递进。
- 触发点在「下一安全动作间隙」进入 **checkpoint 轮**：本轮不派发任何工具，模型只输出模板答案；机械校验通过后恢复动作。
- 范围：主车道；Orientation 与 DC 两族共用同一机制，模板按触发类型微调；检索车道不变。
- 暂停点在模型决策边界，与机械助理执行层不冲突（助理层管动作执行，此处管模型反思）。

### 2.2 模板字段

| 字段 | 类型 | 说明 |
|---|---|---|
| `task_position` | string | 当前任务位置/目标（短文本，长度上限） |
| `progress_evidence` | string[] | 已确认证据/产物身份（可空；存在性交叉校验） |
| `blockers` | string[] | 当前阻塞（可空） |
| `next_action` | enum | `continue` / `adjust` / `gather_evidence` / `ask_user` / `handoff` |
| `changed_direction` | bool | 是否调整方向 |
| `missing_evidence` | string[] | 条件字段：`next_action=gather_evidence` 时必填非空（缺失证据面；2026-08-15 实施明确）；同时参与存在性交叉校验（审计记录，非阻断） |

- 复用现有 allowed/forbidden fields 词汇；长度上限按字段配置；非法字段丢弃并记 journal。
- 未知字段不进模板对象，随 `checkpoint_response` 事件 `ignored_fields` 记录（2026-08-15 实施明确）。

### 2.3 校验与兜底

- 必填字段缺失、`next_action` 越界、长度超限 → 给一次错误反馈重填。
- 重填仍失败 → 按已填写部分机械降级 + journal 记录，不挂死。
- 降级终止态必须可审计（事件含 validation 结果与降级原因）。

### 2.4 计数语义

- checkpoint 轮计入已完成逻辑模型轮（与现有「候选轮计入完成 generation」语义一致）。
- 仅实际完成校验通过的模板轮才重置 Orientation/DC 计数；降级轮是否重置按触发族既定语义。
- 实现落点（2026-08-15）：accepted 与 degraded 都提交 fire（Orientation 重置计数 /
  DC 推进阶段）；同一安全间隙两族同时到期时 Orientation 优先、DC 下一安全间隙再触发
  （一次只排一个 checkpoint 轮）。
- 若运行在 pending 期间终止（取消/错误/恢复），fire 事件已写入但无对应
  `checkpoint_response`，计数不提交；验证器对缺 response 的 fire 无要求，下一
  run 按原计数重新触发（2026-08-15 复核明确）。

## 3. 实施前置

1. ADR-0010 §4.2 修订登记（v1.13 裁决；v1.16 正文修订，2026-08-15）：「注入」→
   「触发点暂停并填写模板」。
2. 事件/Schema 先行：新增 v0.2 `checkpoint_response` 响应事件（模板/响应/校验结果/
   证据交叉校验/降级原因）；Python verifier 交叉校验（fire↔response 引用、attempt
   顺序、outcome 一致性）；DC fire 事件可选携带 `agent_role=main`（主车道），
   验证器对未携带该字段的历史 fire 兼容（2026-08-15 复核）。
3. fixtures 与测试：触发→暂停→填表→恢复；错误反馈重填；降级兜底；checkpoint 轮
   工具拒绝（tool_calls_not_allowed）；主车道隔离（检索车道旧行为）；计数语义；
   并发门禁（一次一个 pending）。2026-08-15 全部实施。
4. 实施审计与 BACKLOG/TODO/索引状态同步（2026-08-15 完成）。

## 4. 非目标

- 不做语义诚实验证；不做会话级强制（仅在触发时暂停）；不改检索车道；不引入新工具面（不使用 submit 工具，模型本轮直接输出模板答案）。
- 响应事件角色恒为主车道 `main`（检索车道不产生 `checkpoint_response`；2026-08-15 复核收紧 Schema）。
