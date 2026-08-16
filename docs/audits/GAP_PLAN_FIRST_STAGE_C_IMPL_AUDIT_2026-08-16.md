# GAP：PLAN-FIRST 阶段 C 实施审计（console 默认 + direct 受控降级）

> 日期：2026-08-16；范围：ADR-0010 §14.17⑱ / `PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md` §7/§9/§11 阶段 C；
> 关联：阶段 A 审计 `GAP_PLAN_FIRST_STAGE_A_IMPL_AUDIT_2026-08-16.md`、阶段 B 审计
> `GAP_PLAN_FIRST_STAGE_B_IMPL_AUDIT_2026-08-16.md`。

## 1. 结论

PLAN-FIRST 阶段 C（console 默认 + direct 受控降级双模式）实施闭合，验收要点 3/6/8/9/10/11
全部有实现入口与测试证据；阶段 A 审计 §7.4 登记的 S2 遗留债务（`blackboard.action_write`
ToolCompleted 扩展字段不符合通用契约形状）随本次「事件面收敛」一并收口。
未闭合：仓库门禁仅剩「orz submodule working tree is dirty」（本阶段代码未提交，与
阶段 A/B 收尾同一状态）。

## 2. 契约层（Schema/事件先行）

1. **新 v0.2 事件 `console_mode_transition`**——`runtime/console-mode-transition-event-
   payload-v0.2.schema.json`：transition_id / from / to / trigger / streak / order_ids /
   model_decision / model_reason / run_id / round / plan_epoch / related_transition_id。
   覆盖 console→direct（switch）、stay 决策、direct→console（return）三个方向。
2. **新 v0.2 事件 `console_order_written`**——`runtime/console-order-written-event-payload-
   v0.2.schema.json`：order_id / write_call_id / action / step_id / round / plan_epoch /
   run_id。
   `blackboard.action_write` 的 ToolCompleted 收敛到通用契约形状（成功只带 exit_code）；
   订单身份与步骤绑定由本事件承载（阶段 A 审计 §7.4 债务收口）。
3. **run-event-v0.2.schema.json** 事件枚举 48 → 50（+2）。
4. **tool-started / tool-completed payload schema** 增可选 `console_mode`（const direct）/
   `transition_id` / `trace_id`（direct 动作事件链盖章字段）。
5. **Python verifier**：`PAYLOAD_SCHEMA_BY_EVENT_TYPE_V02` +2；
   `_verify_v02_console_mode_transition`（方向↔trigger↔decision 交叉、每 run 至多一次
   询问、direct 工具事件必须匹配当前 direct transition、return 必须引用本 run 早先
   console→direct 切换）；`_verify_v02_console_order_written`（order_id 每 run 唯一、
   action 非空、step_id null/非空字符串、order 记录须有先行的 action_write 成功且
   write_call_id 对拍——审查收口 F1 后以 write_call_id 匹配模型真实 call_id）。
6. **fixtures**：两个新事件的 payload minimal.valid / constraint.invalid + envelope valid
   （`runtime/fixtures/run-event-v0.2/`）；conformance 计数 48→50 同步。
7. **ActionOrder 增 `step_id`**（serde default，向后兼容）；`StepStatus` 状态机化——
   `pending → in_progress → done(receipt_id[, direct 证据]) | failed(receipt_id)`，
   自定义 serde 兼容旧归档单位变体（`Pending`/`InProgress`/`Completed`→空证据 Done/
   `Blocked`）与新对象形状（阶段 A 审计「StepStatus 命名漂移」对齐收口）。

## 3. 实现清单

1. `orz-loop/src/console_mode.rs`（新）——`ConsoleMode` / `ConsoleModeState`（run 级：
   模式、故障连败计数、询问标记、transition_id、direct trace 证据面、阈值）；故障面
   分类器（§7.2 机械定义：verify 与 execute-无业务 exit_code 递增；业务非零退出/
   policy/protocol/registry/contract/order_stale/step_not_done 不计）；询问模板解析/
   一次重填/降级 stay；`transition_payload`；`DirectStamp` 事件盖章。
2. `orz-loop/src/controller.rs`——
   - 双模式开关 `with_console_default_enabled`（生产接线：CLI run + ACP server 随
     plan_first 一并开启；测试默认关闭保持既有面）；
   - `reset_console_mode`（run 起始复位，与探针源同点）；
   - `console_direct_begin/end`（direct 直接动作创建 console trace + 证据面登记，
     §7.5 step_done 的 trace_id↔ToolCompleted 对应关系）；
   - `record_console_receipt`（订单 receipt 统一记账：故障连败 + 步骤 done/failed 迁移）；
   - `switch_console_to_direct` / `record_console_stay` / `return_console_to_console`
     （`console_mode_transition` 事件 + 黑板 gate_log 同步；switch 时将当前步骤置
     in_progress——direct 为有记录例外）；
   - `console.step_done` / `console.return_to_console` 工具 handler（main lane only；
     step_done 机械校验 step_id/transition_id/trace_id 三重证据，不匹配拒绝）；
   - `issue_pending_console_order` 步骤门（console 默认态 + 计划在案时：订单必须绑定
     当前可执行步骤，否则 `step_not_done` 消费并写显式 receipt；direct 模式豁免步骤门，
     不豁免安全门）；发放时 pending/failed → in_progress；
   - `console_mode_transition` / `console_order_written` 事件生产；
   - direct 模式 ToolStarted/ToolCompleted 盖章（console_mode/transition_id/trace_id）
     覆盖工作工具主路径、run_tests 路径与检索模式门拒绝路径；
   - 投影：`is_console_surface_tool` / `project_console_default_tool_defs`
     （console 默认面 = 黑板读写 + 只读核查；执行/变更/shell/子代理/检索隐藏）；
     `console.step_done` / `console.return_to_console` 声明随开关收敛、子代理投影剥除。
3. `orz-loop/src/agent_loop.rs`——
   - console 默认面投影接线（计划轮面 → console 面 → direct 恢复工作工具投影）；
   - 调用面门禁 `console_mode_tool_denied`（belt-and-braces，防幻觉直接调用隐藏工具）；
   - 轮末询问轮触发（post-console-batch 安全间隙、优先级低于 orientation/DC、模板块注入）；
   - 询问轮处理（无工具轮、一次重填、仍失败默认 stay、每 run 至多一次）；
   - direct 模式直接动作：`console_direct_begin` → 盖章传入事件链 → `console_direct_end`。
4. `orz-loop/src/planning.rs`——`current_step_index` / `order_step_gate` /
   `mark_step_in_progress` / `mark_step_done` / `mark_step_failed`（§6 状态机与
   step_not_done 门）。
5. `orz-loop/src/checkpoint.rs`——`PendingCheckpoint::ConsoleModeInquiry` 变体
   （无工具轮语义复用强制模板轮；不 commit orientation/DC 状态）。
6. `orz-tui`——`ConsoleModeTransition` / `ConsoleOrderWritten` 事件桥接 + 投影渲染。
7. 既有测试改造——ACP/stdio/TUI 侧脚本随 console 默认面收敛为「计划 → 订单」形态
   （`action_write` 经轮末发放触达权限桥/ACAF/快照链）；事件计数同步。

## 4. 验收点对照（设计 §11）

| 验收点 | 实现/测试证据 |
|---|---|
| 3/11 上一步未 done 拒绝 | `console_step_gate_refuses_wrong_step_then_progresses`（step_not_done receipt；s1→s2 顺序放行）+ `order_step_gate` 单测 |
| 6 主车道无执行工具 | `console_default_surface_hides_execution_tools`（首轮=黑板+plan_write；后续=console 面，断言无执行/变更工具）+ `console_mode_tool_denied` 调用面门禁 |
| 8 三连败→询问轮→switch | `console_fault_streak_inquiry_switch_and_direct_stamp`（streak=3、无工具询问轮、`console_mode_transition` + gate_log、direct 事件携带 transition_id） |
| 9 direct 权限/ACAF/模式门照常 | direct 走 `run_host_tool` 全链路（盖章事件 + 拒绝路径）；ACP `session_prompt_bash_denied_without_interactive_client` / `session_prompt_respects_session_policy` 验证订单目标权限拒绝/放行 |
| 10 stay/return 复位且不再询问 | `console_stay_resets_and_never_reasks`（仅一次 stay 决策，后续失败不重复询问）；`console_step_done_evidence_gate_and_return`（return 单向返回 + 事件） |
| 11 step_done 证据门 | `console_step_done_evidence_gate_and_return`（坏 transition 拒绝）+ `console_step_done_positive_evidence_marks_done`（transition_id+trace_id 证据置 done(direct)） |

## 5. 验证证据

- orz-loop 433 通过 / 0 失败（含新增 console_mode 单测 5 项 + 阶段 C 集成测试 6 项）；
- orz-tui 178 通过 / 0 失败（含新事件桥接测试）；
- orz-assurance 通过；orz-bin stdio_e2e 通过（事件计数随计划轮更新）；
- orz-host acp_server 36 通过 / 0 失败（12 项既有 ACP 测试按 console 默认面收敛）；
- Python：runtime 306 通过（含新 verifier 规则 6 项与 conformance 50 类型；
  审查收口 F1 后新增多订单回归 1 项）；
  assurance 1614 通过 + 14 skipped（唯一失败为 doctor 的
  「orz submodule working tree is dirty」门禁项，未提交所致）；
- 仓库门禁：仅剩 dirty-submodule 一项；schema/fixture 计数一致（schemas 250、
  v0.2 payload positive 29、envelope positive 51、journal fixtures 14）；
- fmt 通过；clippy 无新增告警（新增两处 too_many_arguments 已按契约字段集显式 allow）。

## 6. 边界与登记

- direct 模式只恢复工作工具可见性：权限桥/ACAF/模式门/IPG/预算/探针全部照旧（豁免的是
  步骤门，不是安全门）；计划门约束 console 订单发放。
- 询问轮为 run 级状态：plan epoch 轮换/黑板旋转不清除模式；run 结束自动复位
  （下一 run 起始 console）。
- `console_order_written` 事件为 action_write ToolCompleted 形状收敛的载体；旧期刊
  无该事件不受 verifier 影响（规则只约束新事件）。
- 全量 orz-host 并行测试在本环境出现既有 flaky 挂起（`tests::` 模块；单测逐个通过，
  与阶段 C 改动无关；既有审计已登记 orz-host flaky 待复核）。

## 7. 二次全面审查收口登记（2026-08-16）

阶段 C 全面审查（设计/实现/符合性三维度）发现的全部问题已处理：

- **F1（主要）**：`console_order_written` verifier 原以「action_write 完成事件
  call_id == order_id」对拍，但 producer 的完成事件 call_id 是模型真实工具调用 id
  （`ORD-xxxxx` 为内部订单 id），真实期刊必然误报；且 per-run 单槽 `writes` 记录在
  多订单 run 下产生假阳性。收口：payload 增 `write_call_id`（Schema/verifier/
  fixtures/测试同步），producer 在订单记录中携带所关闭完成事件的 call_id，verifier
  按 per-run 未消费写入队列匹配 write_call_id（一次写入至多支撑一条订单记录）；
  新增 producer 真实形状多订单回归测试。
- **F2（主要）**：`cargo fmt --check` 失败（acp_server.rs 两处）→ 已 `cargo fmt`，
  fmt --check 通过。
- **F3（次要）**：direct 模式 controller 内建工具（blackboard_read /
  compaction_whitelist_add / run_tests 缺运行器拒绝）的 ToolCompleted 未盖章 →
  已补 `stamp_direct`，与 ToolStarted 对称（§7.4）。
- **F4（次要）**：`step_not_done` / `order_stale` / `budget_insufficient` 拒绝不再把
  未执行的目标步骤置 failed——拒绝路径只做连败记账，步骤状态只随执行 receipt
  迁移（§6）。
- **F5（次要）**：TODO P0-C 父项 checkbox 补勾，`- [ ]` 计数与「未闭合 30 项」一致。
- **F6（次要）**：ADR §14.17⑱a「order 记录须先于 action_write 成功」措辞修正为
  「order 记录须有先行的 action_write 成功（write_call_id 对拍）」。
- **F7（次要）**：verifier docstring 删除未实现的「main lane」声明，改为实际检查的
  描述。
- **F8（次要）**：clippy 20 条告警核对——均为既有基线模式（if-collapse/map_or/
  format!/既存 too_many_arguments 等），新文件 console_mode.rs 无告警，两处契约字段
  集函数已显式 allow；`run_host_tool` dead_code 与 HEAD 基线一致（仅测试调用）。
- **边界补登记**：direct 证据面（direct_trace_ids）有界保留最近 100 条 trace；
  `counts_as_assistant_fault` 机械口径明确为「仅整数 exit_code 视为业务结果」；
  verifier 增 transition_id 每 run 唯一与 switch/stay 时 related_transition_id 必须
  null 两项交叉校验。
