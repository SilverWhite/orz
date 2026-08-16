# PLAN-FIRST 阶段 B 实施审计（2026-08-16）

> 范围：PLAN-FIRST 阶段 B——注册板块 = 探针投影（移除静态基础集中间态）、
> 工具栏刷新绑定黑板模型栏（注册板块由黑板模型栏派生，替代 loop-top 静态
> 刷新）。权威：设计 [PLAN_FIRST_BLACKBOARD_DESIGN](../PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)
> §2.4/§9 与 [CLASSICAL_EXECUTION_ASSISTANT_DESIGN](../CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)
> §8；ADR-0010 §14.17 ⑥。待办路由：[BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3a 与
> [TODO](../../TODO.md) P0-C2。

## 1. 验收点 → 实现映射

| 验收点 | 实现位置 | 证据 |
|---|---|---|
| 注册板块 = 探针投影（移除静态基础集中间态） | `controller.rs`：`console_probe_source`（单一探针源） + `sync_console_registrations`（派生唯一路径，Profile/Bundle ∩ 探针完整集并持久化）；`agent_loop.rs`：主车道探针计算后记录探针源、注册板块经同源同步；无探针轮次不再 bundle-only 刷新、沿用上一轮内容 | `console_stage_b_actions_read_derives_registration_from_probe_source`（陈旧静态基础集残留被派生结果替换）；`console_s4_checkpoint_round_retains_probe_filtered_registration`（保留语义回归） |
| 工具栏刷新绑定黑板模型栏（注册板块由黑板状态派生，替代 loop-top 静态刷新） | `controller.rs`：`render_blackboard_section("actions")` 读取时先 `sync_console_registrations` 派生再渲染（live 视图），归档 epoch 读保持快照；工具栏投影与注册板块共用同一探针源（绑定=同源一致性，非工具栏反向读板块） | `console_stage_b_actions_read_derives_registration_from_probe_source`；`console_stage_b_actions_read_retains_stored_registration_without_probe_source`；`console_stage_b_registration_matches_tool_projection`（同源一致性）；`console_stage_b_actions_read_archived_epoch_keeps_snapshot`（归档读不派生、live 读对照） |

## 2. 实现要点

- 单一事实源：controller 新增 `console_probe_source: Mutex<Option<(ToolPolicy,
  ToolProbeSnapshot)>>`——仅内存、随轮覆盖、run 起始复位、不持久化（沿用
  探针快照生命周期纪律）；主车道每轮探针计算后记录，检索车道不记录；
  **审查收口（2026-08-16）：run 起始复位**（`run_turn_inner` 与
  `probe_state` 同纪律调用 `reset_console_probe_source`，本 run 首个有
  探针轮重新记录，不跨 run 沿用）。
- 派生唯一路径：`sync_console_registrations` 由最近探针源派生
  Profile/Bundle ∩ 探针完整集并写入黑板 actions 板块；无探针源时不改写
  （checkpoint 轮/无探针轮次沿用上一轮内容，替代 bundle-only 静态刷新
  中间态）。
- 读取时绑定：`render_blackboard_section("actions")`（live 视图）先同步派生
  再渲染；归档 epoch 读保持快照原样，不派生。
- 与 S3 关系：S3 已实现 Profile/Bundle ∩ 探针完整集（阶段 B 前置面）；本阶段
  收口「读取时派生 + 同源绑定 + 移除 bundle-only 中间态」。
- 工具栏一致性：模型可见工具投影与注册板块共用同一探针源；测试锁定「注册
  板块中工作工具目标 ⊆ 工具栏投影」与「探针移除的工作工具不入注册板块」。

## 3. 测试证据

- `cargo test -p orz-loop --lib`：**421 passed / 0 failed / 3 ignored**
  （较阶段 A 的 416 新增 3 项阶段 B 单测 + 2 项审查收口单测：
  `console_stage_b_actions_read_archived_epoch_keeps_snapshot`、
  `console_stage_b_probe_source_resets_across_runs`）。
- 修改文件 rustfmt：本阶段新增代码格式通过；仓库存在阶段 A 既有格式漂移
  （agent_loop/planning/blackboard/controller 部分行），不属本阶段，登记为
  后续清理观察（§4）。
- clippy：无新增告警（既有告警不属本阶段）。
- `cargo check -p orz-host -p orz-tui -p orz-bin -p orz-assurance` 通过
  （既有告警：orz-host unused_assignments、orz-loop run_host_tool
  dead_code；均已登记 §4，非本阶段引入）。
- 仓库门禁 `python scripts/check_repository.py`：valid=false 仅因
  「orz submodule working tree is dirty」（本阶段代码未提交），其余 0 错误；
  `orz_source_manifest.sha256` 覆盖子模块 HEAD 规范内容，重新生成结果与
  提交版一致（1399 条目），无需变更。

## 4. 边界与登记

- 派生仅作用于 live actions 读取；归档 epoch 读取保持快照原样（设计「跨
  epoch 走归档」不变）。
- `console_probe_source` 不持久化；run 起始复位（与 `probe_state` 同纪律，
  审查收口 2026-08-16），恢复会话后首个有探针轮重新记录。
- S2 遗留债务（`blackboard.action_write` ToolCompleted 扩展字段与通用
  payload schema 对齐）仍随阶段 C console 面收敛处理（阶段 A 审计 §7.4
  已登记）。
- 既有 rustfmt 漂移（阶段 A 代码，agent_loop/planning/blackboard/controller
  部分行）登记为后续清理项，非本阶段阻塞。
- orz-loop `run_host_tool` dead_code 告警（既有，非本阶段代码）登记为后续
  清理项，非本阶段阻塞。

## 5. 审计结论

PLAN-FIRST 阶段 B 两项验收（注册板块=探针投影、工具栏刷新绑定黑板模型栏）
均有实现入口与测试证据；设计-实现符合（ADR-0010 §14.17 ⑥）。剩余未闭合：
PLAN-FIRST 阶段 C（console 默认 + direct 受控降级，含 D5 步骤状态机 /
`step_not_done` 门、双模式、`console.step_done` 证据门）与阶段 A 审计
§5/§7.4 边界项。

审查收口（2026-08-16，全面复核）：补 run 起始复位（与 `probe_state` 同
纪律，不跨 run 沿用）、归档 epoch 读不派生测试锁定、`console_registrations`
注释收敛，并将既有 `run_host_tool` dead_code 告警与 rustfmt 漂移登记为
后续清理项；均不改变阶段 B 验收语义。
