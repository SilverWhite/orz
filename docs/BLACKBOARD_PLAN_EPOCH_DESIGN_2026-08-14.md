# ORZ 黑板 plan epoch 轮换设计（2026-08-14）

> 状态：`design-confirmed`（2026-08-14 用户裁决；实施未开始）
> 权威：ADR-0010 §14.15（v1.15 补写）；本文件是黑板生命周期与压缩解耦的设计入口，
> 不新增与 ADR 冲突的语义。
> 取代范围：实施后取代 ADR-0010 §3.6 / §14.14 条目 1 ⑤ 与
> `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` 中「黑板 edit 窗口随压缩滚动（用后擦净）」
> 机制；实施前旧机制仍是当前代码行为（压缩成功后清空黑板 edit 窗口）。

## 1. 背景与问题

- 压缩是上下文窗口管理（何时塞不下），黑板是任务工作状态（何时该换任务）；两个生命周期
  本不该同步。
- 长任务中 160K/200K 压缩会反复触发；压缩成功即清空黑板 edit 窗口会把同一任务的编辑痕迹
  从活板上抹掉，只留下摘要指针，产生任务状态漂移风险。
- 中立问询强制模板轮靠黑板 plan/task_position 做锚点（`ORIENTATION_FORCED_TEMPLATE_DESIGN`）；
  黑板若随压缩擦除，问询锚点随之动摇。

## 2. 核心语义

- 黑板生命周期 = plan epoch；压缩生命周期 = 上下文窗口；两者解耦。
- plan 区为单写者复写区：永远只保存当前已批准计划；每个已批准计划携带 `plan_id` +
  `plan_epoch`。
- 轮换触发 = 新 plan epoch 批准（机械事件）；**不是** plan 文本变化，**不是**当前任务完成。
  - 同 epoch 修订（同 plan_id 下步骤细化、用户纠偏）不轮换、不清黑板。
  - 新任务轮即使 plan 文本与旧轮相同，也因 epoch 身份不同而轮换。
- 原子轮换序列（单次提交）：
  1. 归档旧 epoch 黑板快照；
  2. 清空 epoch 作用域分区；
  3. 写入新 plan（复写）；
  4. 之后才允许新 epoch 首个动作与下一次问询。

## 3. 轮换范围

| 分区/项 | 行为 | 理由 |
|---|---|---|
| plan（goal / steps / analysis / decisions / auth_grants） | 复写 | 新任务轮的新计划 |
| edits / tool_actions / exec 工作记录 | 清空（归档后） | 本 epoch 工作状态 |
| gate_log | 保留 | 审计链 |
| 白名单 | 保留 | 设计上常驻，压缩与轮换均跳过 |
| internal / external retrieval | 保留，随 activation close/continue 生命周期管理 | 证据可跨任务复用，已有生命周期 |

## 4. 归档与回查

- 每个 epoch 轮换时写确定性快照（建议 `.gsa/blackboard/epoch-<plan_epoch>.json`；
  内容=plan + edits + tool_actions + exec + 轮换时间戳；随既有 7 天 retention 管理）。
- epoch 快照是全量路径/动作记录的结构化承载；摘要存档保持人类可读投影，不再承担「全量」。
- `blackboard_read` 跨 epoch 查询：live 视图清空后回查归档（或增加 `epoch` 参数），
  保持「按分区和时间范围取用」的既有契约。

## 5. 与压缩机制衔接

- 移除压缩成功后 `blackboard.edits.clear()`；压缩只处理上下文，不再触碰黑板。
- 路径槽语义：本 plan epoch 增量（Top-40 条 + 5K 字符双上限不变）；溢出指针指向当前
  epoch 快照（摘要存档保持人类可读投影）。
- marker 携带 `plan_epoch`；恢复时随 sidecar 恢复 epoch 快照与 marker。
- 目的/计划槽继续机械取自黑板 plan，压缩前后一致。

## 6. 中立问询 / DC 耦合

- 同 epoch 内任意次压缩不动黑板 → Orientation/DC 的 `task_position` / 计划锚点稳定。
- 轮换在新 epoch 首动作前原子完成 → 问询轮不会读到半空黑板。
- 与 `ORIENTATION_FORCED_TEMPLATE_DESIGN` 互补，不新增模板字段。

## 7. 实施切片（登记于 BACKLOG 6e / TODO）

- S1：plan epoch 身份与批准事件接线——`plan_id`/`plan_epoch` 随 plan 批准事件写入
  （当前 `with_plan` 仅构造时一次）；区分同 epoch 修订（不清）与新 epoch 批准（轮换）。
- S2：原子轮换与归档——epoch 快照格式/落盘；清空范围；gate_log/白名单/检索分区豁免。
- S3：压缩解耦——移除压缩清黑板；路径槽/指针语义调整；marker 带 `plan_epoch`；恢复接线。
- S4：`blackboard_read` 跨 epoch 回查（归档或 `epoch` 参数）。
- S5：测试与审计——压缩不清板、轮换原子性/范围、跨 epoch 回查、恢复、事件/契约；
  实施审计 + BACKLOG/TODO/索引同步。

## 8. 非目标

- 不新增模型自由随记区；不新增 per-epoch LLM 摘要（归档是机械快照）。
- 不改动作台账机械坍缩（S2）与五段模板摘要其余部分。
- 不在本轮实施中改动检索分区生命周期。
