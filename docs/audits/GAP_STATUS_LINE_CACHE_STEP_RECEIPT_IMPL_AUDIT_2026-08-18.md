# GAP-STATUS-LINE-CACHE-STEP-RECEIPT 实施审计（2026-08-18）

- 状态：implemented
- 范围：orz-loop（`agent_loop.rs` / `controller.rs` / `prompt.rs`）
- 关联：ADR-0010 §14.25（v1.25）；CLASSICAL-EXEC §14；CLI_PROJECT_INDEX
- orz 子模块：`ba86910`

## 1. 背景与根因（缓存命中率）

TB2 复验 run（`D:\tb-eval\jobs\2026-08-17__23-29-44`，80 请求）命中率 66.5%，
旧批次 95–99%。18 次 `request_header_change` 全部为 system 变化、tools/config
恒定。根因：常驻状态行 `[任务状态]` 嵌入系统提示词；console 步骤机每笔订单
receipt 成功即推进步骤；状态行随每轮变化 → 前缀缓存整段失效（断点请求命中
0.2–4%、未命中 2万–13万 token）。

## 2. 实施落点

1. **状态行移出系统提示词**（agent_loop.rs）：系统组装删除 `render_status_line`
   注入，system = 预算块 + 基础提示 + plan-first 框架（完全静态）。
2. **尾随追加**（controller.rs `sync_status_line_message` +
   `status_line_appended` 字段）：每轮请求前计算 `render_status_line`，与上次
   追加值比较、变化才推送用户消息并记录；无计划不推送。
3. **步骤语义**（controller.rs `record_console_receipt` 增 `mutate_step` +
   调用点）：发放期拒绝（registry/contract/target/policy）不迁移步骤状态；
   仅执行步（execute/verify）失败标 failed。
4. **状态行当前步**（prompt.rs `build_status_line`）：第一个非 done，与
   `planning::current_step_index` 门禁一致。

## 3. 测试证据

- orz-loop 全量 443 通过 / 0 失败（3 ignored 为既有）。
- 新增/更新：`with_plan_status_line_is_trailing_user_message`、
  `no_plan_means_no_status_line`（补无尾随断言）、
  `status_line_is_byte_stable_across_rounds`（去重断言）、
  `status_line_trailing_appends_only_on_step_change`、
  `policy_denied_order_does_not_fail_bound_step`、
  `execution_failure_marks_bound_step_failed`。
- clippy 无新增可归因告警（基线 22 条不变）。

## 4. 预期收益与实机验证

- 预期把该类 run 命中率从 ~67% 拉回 90%+（system 摘要恒定、断点消失）。
- 实机验证=Linux musl 重建后单题复验（make-doom），核对
  `request_header_change` 次数趋零、命中率回升；本审计不含实机验证。

## 5. 边界

- 状态行尾随消息仅在变化轮计费（~100 token/变化），去重后不随轮增长。
- 无计划/计划清空（None）不追加；计划从未有→有会追加（变化检测）。
- 执行失败仍标 failed（可重试语义保留）；仅发放期拒绝豁免。
