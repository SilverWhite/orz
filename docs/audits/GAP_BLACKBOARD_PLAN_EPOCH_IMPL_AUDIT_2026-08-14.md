# ORZ 黑板 plan epoch 轮换实施审计（2026-08-14）

> 状态：`implemented`（S1-S5 全部闭合，2026-08-14）
> 权威：ADR-0010 §14.15（v1.15）；设计文档
> `docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`。
> 范围：黑板生命周期按 plan epoch 轮换、压缩解耦、跨 epoch 回查、恢复与契约同步。
> 触发：用户指示「P0-C 小样 2 暂时不动，先把黑板 plan epoch 落实」。

## 1. 结论

黑板 plan epoch 轮换（BACKLOG 6e / TODO ORZ-BLACKBOARD-PLAN-EPOCH）S1-S5 全部实施并验证
闭合：plan 批准事件携带 `plan_epoch`；`with_plan` 带 `plan_id`+`plan_epoch` 身份并原子轮换
（同 plan_id 修订不清板）；`.gsa/blackboard/epoch-<n>.json` 快照落盘（当前 epoch 批准/修订
upsert + 轮换归档旧 epoch）；压缩不再清黑板、marker 携带 `plan_epoch`、路径槽溢出指针指向
epoch 快照；`blackboard_read` 增 `epoch` 参数跨 epoch 回查；archive dir 构造时装载最新 epoch
快照（恢复入口）；retention 覆盖 `.gsa/blackboard` 7 天清扫。ADR-0010 §3.6 正文、设计文档、
BACKLOG/TODO/索引状态已随实施同步。

## 2. 实施内容

### S1 — plan epoch 身份与批准事件接线

- `PlanSection` 新增 `plan_id: Option<String>` 与 `plan_epoch: u64`（0=未批准计划）。
- `PlanApproved` 运行时事件 payload 新增必填 `plan_epoch`（integer ≥1）：
  - Schema：`runtime/plan-approved-event-payload-v0.1.schema.json`（required + property）。
  - 生产者：`orz-bin/src/main.rs` `run_plan_phase` 接收 `plan_epoch` 并写入事件 payload；
    `run_plan` 在 plan 阶段前以 `orz_loop::epoch::next_plan_epoch_from_archive` 计算
    epoch（磁盘现存 epoch 最大值 +1，跨进程单调唯一）。
  - fixtures：v0.1/v0.2 payload（minimal.valid / constraint.invalid）、envelope、两份
    `plan-run.jsonl` 按 Rust-parity 哈希链重算（`assurance/run_event_journal_validation.py`
    157 项通过）。
  - TUI：`TuiEvent::PlanApproved` 增 `plan_epoch`，投影显示 epoch。
- `AgentLoopController::with_plan(plan_id, plan_epoch, goal, steps)`：同 `plan_id` →
  same-epoch 修订（替换 goal/steps，不清黑板、不轮换）；新 `plan_id` → 原子轮换。

### S2 — 原子轮换与归档

- `Blackboard::rotate_to_plan`：单次写锁内完成「捕获旧 epoch 快照 → 清
  edits/tool_actions/exec → 复写 plan」；gate_log/白名单/检索分区不清；PlanSection
  （analysis/decisions/auth_grants）随新计划复写。
- `.gsa/blackboard/epoch-<n>.json` 快照（`EpochSnapshot`：plan_id/plan_epoch/plan/edits/
  tool_actions/exec/rotated_at）：
  - 批准与修订均持久化当前 epoch（恢复入口需要「最新已批准计划」在盘上）；
  - 新 plan_id 轮换时先归档旧 epoch 快照再写新 epoch；
  - 写入有界重试（3 次），失败 `tracing::warn` 不阻断轮换（审计注记：机制失败仅日志，
    未入事件面——保留为边界）。
- 归档目录由 builder `with_blackboard_archive_dir(Option<PathBuf>)` 注入；CLI plan 路径
  注入 `cwd/.gsa/blackboard`。

### S3 — 压缩解耦

- 移除 `run_template_compact` 成功路径的 `blackboard.edits.clear()`；压缩不再触碰黑板
  （同 epoch 任意次压缩，plan/edits/tool_actions/exec 保持不变）。
- 路径槽语义：本 plan epoch 增量（Top-40 + 5K 双上限不变）；溢出指针从「本次摘要存档」
  改为「当前 epoch 快照」（`mechanical_slots` 增 `epoch_archive` 参数）。
- `build_summary_marker` 增 `plan_epoch`：marker 固定输出「黑板 plan_epoch: N」行；
  回查提示补「历史 plan epoch 用 epoch 参数」。
- 恢复：`with_blackboard_archive_dir` 构造时以 `latest_epoch_snapshot` 装载最高 epoch
  快照（plan/edits/tool_actions/exec），与 marker 一起构成恢复上下文。

### S4 — blackboard_read 跨 epoch 回查

- 工具定义新增可选 `epoch`（integer ≥1）参数；`render_blackboard_section(section, since,
  epoch)`：`epoch=Some(n)` 时从归档读取 `epoch-<n>.json` 渲染；归档未配置或快照缺失时
  返回显式错误文本（不静默回退 live 视图）。
- `tool_completed` payload 在调用带 epoch 时附 `epoch` 字段。
- 渲染逻辑收敛到 `orz_loop::epoch::render_section`（live 与归档共用同一视图函数）。

### S5 — 测试、retention 与审计

- retention：`PruneReport` 增 `removed_blackboard_epoch_archives`，`.gsa/blackboard` 按
  7 天 age 清扫（与 compaction/conversations 同纪律）。
- 测试：
  - orz-loop 310 通过 / 0 失败（新增 epoch 模块测试、控制器轮换/归档/跨 epoch/恢复测试；
    `session_end_compact_pins_marker_into_sidecar_and_keeps_edits` 由旧「清板」语义改
    为「不清板」断言）；
  - orz-host 209 / orz-tui 178 / orz-bin 全部通过；
  - Python：runtime journal 校验 157、conformance 14 通过；
  - 仓库门禁 `scripts/check_repository.py` valid、0 错误。

## 3. 契约变更清单

- `runtime/plan-approved-event-payload-v0.1.schema.json`：+required `plan_epoch`。
- `scripts/generate_run_event_fixtures.py`：PAYLOAD_GOOD/BAD plan_approved +`plan_epoch`。
- v0.1/v0.2 fixtures：plan-approved payloads、envelopes、journals（哈希链重算）。
- `orz-tui`：PlanApproved 事件结构、bridge、投影。
- `blackboard_read` ToolDef：+`epoch` 参数。

## 4. 边界与注记

- epoch 快照是「当前状态 upsert + 轮换时归档旧态」：修订会刷新当前 epoch 文件内容；
  跨进程恢复只能回到最近一次持久化的状态（从未轮换的终止态进程不落最终快照——与
  设计「轮换时写快照」一致，恢复以最近持久化 epoch 为准）。
- 归档写失败仅 warn（未新增事件字段）；与压缩存档的显式 `archive_write_failed` 不同，
  属已知边界，后续如需可并入审计事件面。
- CLI plan 一次性路径每进程一个 epoch（磁盘 max+1）；ACP 会话路径当前未接 plan 模式，
  未来接线时设置 archive dir 即自动获得轮换/恢复语义。
- 检索分区生命周期未改动（非目标）；中立问询/DC 锚点因黑板不再随压缩滚动而稳定。

## 5. 入口

- 设计：`docs/BLACKBOARD_PLAN_EPOCH_DESIGN_2026-08-14.md`
- 权威：ADR-0010 §14.15 / §3.6（v1.15 实施登记）
- 代码：`orz/crates/orz-loop/src/blackboard.rs`、`epoch.rs`、`controller.rs`、
  `agent_loop.rs`、`summary.rs`；`orz/crates/orz-host/src/retention.rs`；
  `orz/crates/orz-bin/src/main.rs`；`orz/crates/orz-tui/src/{events,bridge,projection}.rs`
- 契约：`runtime/plan-approved-event-payload-v0.1.schema.json`、
  `runtime/fixtures/run-event-v0.1|v0.2/{payloads,envelope,journals}`
- 状态同步：BACKLOG 6e、TODO、CLI_PROJECT_INDEX（FUS-BLACKBOARD-PLAN-EPOCH /
  AUTH-BLACKBOARD-PLAN-EPOCH → implemented）

## 6. 复查补强（2026-08-15，全面复查 F1/F3，用户裁决）

### F1 — retention 清扫与编号唯一性

- `plan_epoch` 改为**时间戳单调编号**：unix 毫秒为基底，
  `next = max(now_ms, 磁盘现存 max + 1)`；时间戳进入编号本身（而非仅文件名），
  7 天 retention 全量清扫后编号不复用，marker / `blackboard_read epoch` 参数等
  引用方跨窗口仍唯一。
- retention 对 `.gsa/blackboard` 按年龄清扫时**始终保留最高编号快照**（恢复入口），
  长生命周期黑板不因当前 epoch 文件超过 7 天而失去恢复能力。
- 顺带修正：`latest_epoch_snapshot` 改为直接扫描磁盘最高编号（原「next-1」推导在
  时间戳编号下会指向不存在的文件）。

### F3 — 身份一一对应、无误用可能

- `rotate_to_plan`/`try_with_plan` 强制不变式，错误先于任何黑板变更返回：
  - `plan_epoch == 0` → `ZeroEpoch`；
  - 同 `plan_id` 但 `plan_epoch` 不同 → `SamePlanEpochMismatch`（修订必须沿用
    已登记 epoch）；
  - 新 `plan_id` 未严格递增 → `NewPlanEpochNotGreater`。
- `with_plan` 对违反 fail-fast（`.expect` 明确报错）；生产接线经
  `epoch::next_plan_epoch_from_archive` 计算编号，正常路径不会触发。

### 验证（2026-08-15）

- orz-loop 312 / orz-host 210 / orz-tui 178 / orz-bin 全部通过；
- Python runtime 251 通过；仓库门禁 valid、0 错误。
