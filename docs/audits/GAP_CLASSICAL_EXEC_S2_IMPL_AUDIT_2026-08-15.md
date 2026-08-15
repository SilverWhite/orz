# P0-C orz 内嵌集成 S2 实施审计（2026-08-15）

> 范围：CLASSICAL-EXEC-ASSISTANT 内嵌集成切片 S2——模型面投影（注册板块
> 读取 + 动作栏写单）与轮末机械发放（round/plan_epoch/run_id 防重放 →
> 注册表/契约 → 目标解析/ACAF/策略门 → 执行 → 响应验证 → 结果栏 receipt
> + trace_id → TraceStore.commit）。
> 权威：设计 [CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md](../CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md) §8；
> 待办路由：[BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3 与 [TODO](../../TODO.md) P0-C。

## 1. 验收点 → 实现映射

| S2 验收点 | 实现位置 | 证据 |
|---|---|---|
| 模型面投影=注册板块读取（最小参数提示，不复制 schema） | `blackboard_read` section 枚举扩展 `actions`；`epoch::render_section` actions 分支（注册/单槽/结果有界渲染，归档可读）；注册板块每轮机械刷新（`run_agent_loop` loop-top → `set_registration`） | `console_s2_write_issue_and_receipt_flow`（6 项注册 + 渲染回复断言）；`render_actions_section_shows_registration_order_and_results` |
| 动作栏写单工具 `blackboard.action_write`（pending 机械拒绝） | ToolDef + `run_host_tool` handler（ReadOnly 类；round/plan_epoch/run_id 机械盖章；`order_slot_busy`）；主车道专属三重守卫（subagent 投影剥除 / ToolFilter write gate / activation 守卫） | `console_s2_action_write_slot_busy_refuses_second_order`；`subagent_projection_restores_browser_read`（剥除断言） |
| round/plan_epoch 防重放与过期 | `issue_pending_console_order` 三重校验（round/plan_epoch/run_id），过期消费并 `order_stale` receipt | `console_s2_stale_order_is_consumed_with_explicit_receipt`；`console_s2_cross_run_leftover_order_is_stale` |
| 注册表/契约 | `console::issue_action` registry/contract 步（注册时校验并缓存 validator） | `issue_action_routes_contracts_executes_and_verifies` 等 S1 测试 + 新增基础集注册测试 |
| 真实目标解析（路径/作用域，复用 orz-paths） | 由既有 host 链路承担：`run_host_tool` → `acaf_action_event`（snapshot_store worktree 解析 file_path）/ `host.call_tool`（resolve_model_path）；console 层不重写解析器 | 既有 ACAF 测试；`console_s2_permission_denial_maps_to_policy_receipt`（经 run_host_tool 全链路） |
| ACAF 票据（解析后真实目标验票） | 委托 `run_host_tool`（`acaf_action_event`/`acaf_command_exec_event`），拒绝归一化 `step=policy` | `console_s2_policy_refusal_classification_locked`（ACAF 前缀判定） |
| 策略表（taint/模式门） | 权限门 PolicyFeedback::Denied + ACAF/模式门稳定前缀 → `ExecuteError::PolicyDenied` → `step=policy`/`policy_denied`（无 execute trace 尾部）；taint 组合禁令为设计项，运行时未实施（边界 §4） | `console_s2_permission_denial_maps_to_policy_receipt`；`console_s2_policy_refusal_classification_locked` |
| 经 `run_host_tool` 执行（权限 + 事件链） | `ControllerConsoleExecutor` → `run_console_target` → `run_host_tool`（main-lane 参数：permission_gated=true、probe_writeback=true）；工具回复写 scratch 丢弃，反馈走结果栏 | `console_s2_write_issue_and_receipt_flow`（ToolCompleted call_id=ord-000001、exit_code=0） |
| 响应 schema 验证 | `issue_action` verify 步 | S1 verify 测试 |
| 结果栏 receipt + trace_id | `push_console_result`（ActionResult，有界 50，随 epoch 归档） | `console_s2_write_issue_and_receipt_flow` receipt 断言 |
| 策略拒绝映射 step=policy | `ExecuteError::PolicyDenied` → `failure_envelope`（policy 不带 trace 尾部） | `console_s2_permission_denial_maps_to_policy_receipt`（error.trace 为 None） |
| 发放收口 `TraceStore.commit` | `commit_console_trace`（成功/失败/过期统一收口） | receipt trace_id 可经 TraceStore.get 取回 5 步事件 |

## 2. 实现要点

- `blackboard.action_write` 是唯一写单按钮：写无副作用（ReadOnly 类，仅内存
  单槽），副作用只在轮末单一发放出口；模型不提供 round/plan_epoch/run_id
  （机械盖章的防重放信任锚）。
- 轮末发放位于 post-tool-batch 安全间隙；pending checkpoint 优先——本间隙
  已有 checkpoint 时跳过发放，订单留在槽中，下一工具轮按 `order_stale`
  显式拒绝（fail-closed，不静默丢单也不跨轮误发）。
- 跨 run 防重放：ActionOrder 增加 `run_id` 绑定（写单 run），轮号/epoch 重合
  的遗留订单不会在新 run 被误发（S2 实施中发现的防重放缺口，已闭合）。
- 生产执行器适配（`ControllerConsoleExecutor`）委托 `run_host_tool`：权限桥
  （PermissionRequested/PermissionDecision）、ACAF 票据、模式门、IP5 快照、
  ToolStarted/ToolCompleted 事件链全部复用，禁止绕过既有门直接调
  `host.call_tool`；工具回复消息写入 scratch 缓冲区后丢弃（模型不等待该
  回复——反馈闭环是结果栏 receipt）。
- 策略拒绝归一化分两级：权限门经 `PolicyFeedback::Denied`（结构化）；ACAF/
  模式门在 controller 侧无反馈标记，适配层以稳定输出前缀机械判定（测试锁定
  文案，防静默退化）。

## 3. 测试证据

- `cargo test -p orz-loop --lib`：**364 passed / 0 failed / 3 ignored**
  （S2 新增 8 项：console 基础集 1 + controller 发放链路 5 + 策略分类 1 +
  epoch actions 渲染 1；另更新 4 项投影期望与 1 项子代理投影断言）。
- `cargo test -p orz-host --lib permission`：18 passed / 0 failed
  （access_kind 含 `blackboard.action_write` Read(None) 映射）。
- 修改文件 `rustfmt --check` 通过；clippy 无新增告警（全 crate 既有 17 项
  不属本切片）。
- `cargo check -p orz-host -p orz-tui -p orz-bin` 通过（仅 orz-host 既有
  local_browser 告警）。

## 4. 边界与登记

- 注册板块当前为静态基础动作集（6 项）；「探针完整集 ∩ Profile/Bundle 加载集」
  过滤随 S3（Profile/Bundle）接线。模式门会拒绝的按钮（如 mode=off 下的
  workspace.index）发放时得到显式 `step=policy` receipt——fail-closed 可诊断，
  不静默隐藏按钮。
- taint 动作组合禁令是设计 §6 登记项，运行时尚未实施；适配层的 PolicyDenied
  归一化已预留该门接入点（接入后同样映射 `step=policy`/`policy_denied`）。
- 与单一探针面交互：`blackboard.action_write` 不是工作工具（不进入
  WORK_TOOLS/探针分区），恒声明；注册板块内容由机械注册表投影，与探针面
  正交。Python verifier 的 `_WORK_TOOLS` 无需变更。
- 事件面无新增事件类型/Schema 变更（复用 ToolStarted/ToolCompleted/
  PermissionRequested/PermissionDecision），TUI 投影无需改动。

## 5. 审计结论

切片 S2 的 11 项验收点全部有实现入口与测试证据；设计-实现符合（含 S2 期间
发现的跨 run 防重放缺口修复）；未发现静默降级路径。S3（assistant.trace 只读
服务接线、workspace.run_script 生产化、Profile/Bundle）与 S4（端到端 + 正式
组件决策门材料）按 TODO/BACKLOG 继续。
