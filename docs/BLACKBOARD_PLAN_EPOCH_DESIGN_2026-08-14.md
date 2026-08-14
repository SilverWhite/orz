# ORZ 黑板 plan epoch 轮换设计（2026-08-14）

> 状态：`implemented`（2026-08-14 用户裁决放行；S1-S5 实施闭合；2026-08-15
> 复查补强 v1.15⑧（编号时间戳化、身份不变式强制、retention 保留最新快照）与
> v1.15⑨（复查遗留 F2/F4-F7/F9/F10——原子写盘与回退加载、跨进程 claim、归档目录
> 单一来源、非法 epoch 显式报错、归档失败入事件面、`persisted_at` 语义修正、
> §5 恢复措辞对齐）；审计见
> `docs/audits/GAP_BLACKBOARD_PLAN_EPOCH_IMPL_AUDIT_2026-08-14.md`）
> 权威：ADR-0010 §14.15（v1.15 补写）；本文件是黑板生命周期与压缩解耦的设计入口，
> 不新增与 ADR 冲突的语义。
> 取代范围：实施后取代 ADR-0010 §3.6 / §14.14 条目 1 ⑤ 与
> `CONTEXT_COMPACTION_DESIGN_2026-08-14.md` 中「黑板 edit 窗口随压缩滚动（用后擦净）」
> 机制；实施已闭合，当前代码行为为 v1.15（压缩不再触碰黑板）。

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
- `plan_epoch` 为**时间戳单调编号**（v1.15⑧，2026-08-15）：unix 毫秒为基底，
  `next = max(now_ms, 磁盘现存 max + 1)`。时间戳进入编号本身（而非仅文件名），
  因此 marker / schema / `blackboard_read epoch` 参数等引用方在 7 天 retention
  清扫后仍唯一：旧编号永远小于未来编号，不会复用、不会歧义。
- 跨进程分配（v1.15⑨，F4）：CLI 启动时用 `.claim-<n>` 原子 claim 文件占号
  （`create_new`），同毫秒并发进程只有一个能赢得该编号，碰撞方递增重试；
  崩溃留下的 claim 永久保留该编号（时间戳基底使占号成本为零），retention 按
  年龄清扫过期 claim。
- **身份不变式（一一对应，无误用可能）**：同 `plan_id` 修订必须沿用同
  `plan_epoch`；新 `plan_id` 必须使用严格更大的 `plan_epoch`。违反在
  `rotate_to_plan`/`try_with_plan` 返回错误（`with_plan` fail-fast），拒绝
  发生在任何黑板变更之前。
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
- 快照落盘为原子写（v1.15⑨，F2）：先写 `<name>.json.tmp` 并自检解析，再 rename
  到最终文件名；崩溃只可能留下 `.tmp`，不会产生「半截文件成为最高编号恢复入口」。
  恢复入口 `latest_epoch_snapshot` 从高到低回退到第一个可解析快照。
- retention 对 `.gsa/blackboard` 按年龄清扫，但**始终保留最高编号 epoch 快照**
  （v1.15⑧，2026-08-15）：最高编号即恢复入口，长生命周期黑板即使当前 epoch
  文件超过 7 天也不会失去恢复能力；旧编号因时间戳基底不会与新编号冲突。
- epoch 快照是全量路径/动作记录的结构化承载；摘要存档保持人类可读投影，不再承担「全量」。
- `blackboard_read` 跨 epoch 查询：live 视图清空后回查归档（或增加 `epoch` 参数），
  保持「按分区和时间范围取用」的既有契约。`epoch` 缺省=live 视图；参数存在但非法
  （0 / 负数 / 浮点等）显式报错（v1.15⑨，F6），不静默回退。
- 快照时间戳字段为 `persisted_at`（v1.15⑨，F9）：当前 epoch 每次批准/修订持久化时
  也会刷新，语义不是「仅轮换时刻」；旧归档的 `rotated_at` 键经 serde alias 兼容读取。

## 5. 与压缩机制衔接

- 移除压缩成功后 `blackboard.edits.clear()`；压缩只处理上下文，不再触碰黑板。
- 路径槽语义：本 plan epoch 增量（Top-40 条 + 5K 字符双上限不变）；溢出指针指向当前
  epoch 快照（摘要存档保持人类可读投影）。指针路径取自 controller 配置的归档目录
  （v1.15⑨，F5——单一来源，自定义归档目录不失真）。
- marker 携带 `plan_epoch`；恢复经 archive dir 装载最新 epoch 快照，与 marker
  一起构成恢复上下文（v1.15⑨，F10——与 ADR-0010 §14.15 ⑦/⑧ 措辞一致）。
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
