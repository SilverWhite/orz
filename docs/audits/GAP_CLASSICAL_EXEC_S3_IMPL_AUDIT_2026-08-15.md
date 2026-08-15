# P0-C orz 内嵌集成 S3 实施审计（2026-08-15）

> 范围：CLASSICAL-EXEC-ASSISTANT 内嵌集成切片 S3——`assistant.trace`
> 只读服务生产接线、`workspace.run_script`（PTC 线性脚本）生产化、
> Profile/Bundle 按钮组加载（注册板块 = Profile/Bundle ∩ 探针完整集）。
> 权威：设计 [CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md](../CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)
> §3/§8 与 [PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md](../PLAN_FIRST_BLACKBOARD_DESIGN_2026-08-15.md)
> §9（阶段 B 前置面）；待办路由：[BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3 与
> [TODO](../../TODO.md) P0-C。

## 1. 验收点 → 实现映射

| S3 验收点 | 实现位置 | 证据 |
|---|---|---|
| `assistant.trace` 只读服务接线（按 trace_id 取回有界日志；读操作入审计） | `console.rs`：`ActionKind::TraceRead` + 注册 `assistant.trace`（输入契约 trace_id/tail、响应契约 trace_id/request_id/events/truncated）；`issue_action` TraceRead 分支（`TraceStore` 短锁快照、默认 tail=20、`not_found` fail-closed）；`controller.rs` 发放链对内部动作补 ToolStarted/ToolCompleted 事件（复用 v0.1 事件面，无 Schema 变更） | `console::tests::trace_read_service_returns_bounded_events`；`trace_read_missing_trace_fails_not_found`；`controller::tests::console_s3_trace_read_issuance_flow`（事件面断言 exit_code=0） |
| `workspace.run_script`（PTC）生产化：线性脚本 + `$ref`、逐行契约校验 + trace、上限 20 步/30s/4MiB、fail-closed | `console.rs`：`ActionKind::RunScript` + 注册 `workspace.run_script`；`static_validate_script`（名称唯一/服务已知/禁嵌套/`$ref` 形状·作用域·类型）、`substitute`（运行时 `$ref` 解析）、`run_script`/`run_script_with_limits`（逐行经 `issue_action` 复用注册表/契约/目标/执行/验证五步链；失败保留内层 step/code + `script_step`；后续步骤不执行） | `console::tests::run_script_executes_steps_with_refs`；`run_script_static_validation_rejects_bad_scripts`；`run_script_step_failure_stops_and_preserves_inner_error`；`run_script_enforces_wallclock_and_byte_limits`；`controller::tests::console_s3_run_script_issuance_flow`（内层 read_file 事件带 `.s1`/`.s2` call_id） |
| Profile/Bundle 按钮组加载（Benchmark/ReadOnly/标准；注册板块 = 探针 ∩ Bundle） | `console.rs`：`ActionBundle`（standard/read_only/benchmark 三档）挂在 `ActionSpec`；`ServiceRegistry::registrations_for(profile, probe)`（Host 动作按工作工具探针完整集过滤；非工作工具不探不标；内部动作恒加载）；`agent_loop.rs` 同轮探针快照同时驱动工具投影与注册板块；`controller.rs` `console_registrations(profile, probe)` | `console::tests::registrations_for_filters_by_bundle_and_probe`；`controller::tests::console_s2_write_issue_and_receipt_flow` 更新断言（TestHost 无测试运行器 → run_tests 被探针移除，注册板块 7 项） |

## 2. 实现要点

- 内部动作与 Host 动作在 `ActionSpec` 上分流：`ActionKind::{Host, TraceRead,
  RunScript}`，`target_tool` 改为 `Option<String>`（Host 必须携带，注册即
  拒绝；内部动作为 `None`）。内部动作不走 `ActionExecutor`，确定性实现
  内置；Host 动作保持注册表 → 契约 → 目标解析 → 执行委托 → 响应验证五步链。
- `assistant.trace` 在发放链内通过 `TraceStore` 短锁读取快照（锁不跨
  await，发放链保持 `Send`）；读操作本身生成新 trace（本次订单的审计面）
  并补 ToolStarted/ToolCompleted 事件——「读操作本身入 journal」。
- PTC 脚本的每一步是独立 `ActionOrder`（合成 call_id `ord-<n>.s<k>`），
  经 `issue_action` 递归发放：注册表/契约/目标/执行/验证逐行生效，内层
  host 调用仍走 `run_host_tool` 全链路（权限/ACAF/模式门/事件链），不绕过
  既有门。递归 async 调用以 `Box::pin` 引入指针间接层；嵌套脚本在静态
  校验阶段拒绝。
- 静态校验先于任何执行（fail-closed）：重复 `as`、未知服务、嵌套脚本、
  `$ref` 形状/作用域/类型不匹配均按契约错误拒绝，执行器零调用。
- 上限与 POC 定档一致：20 步 / 30s 墙钟 / 4 MiB 累计响应（含最终
  `result` 重复计算）；超时与超限结构化返回 `script_timeout` /
  `script_response_limit`。
- 注册板块投影 = Profile/Bundle 加载集 ∩ 探针完整集：会话场景键取
  `host.tool_policy()`（Interactive=标准 / ReadOnly / Benchmark）；
  Host 动作的工作工具目标必须在 `ToolProbeSnapshot.complete`；非工作工具
  （如 `project_doc_index`）不探不标、按注册表声明保留（与模型可见工具
  投影语义一致）；内部动作无 host 目标、不做探针过滤。

## 3. 测试证据

- `cargo test -p orz-loop --lib`：**377 passed / 0 failed / 3 ignored**
  （S3 新增 console 单测 8 项：注册不变式 1 + bundle/探针投影 1 + trace
  读 2 + 脚本成功/静态校验/失败收口/上限 4；新增 controller 端到端 2 项：
  run_script 发放与 trace 读发放；更新 S2 注册板块断言 1 项）。
- 修改文件 `rustfmt --check` 通过；clippy 无新增告警（既有告警不属本切片）。
- `cargo check -p orz-host -p orz-tui -p orz-bin -p orz-assurance` 通过
  （仅 orz-host 既有 local_browser 告警）。
- 仓库门禁 `python scripts/check_repository.py`：**valid = true、0 错误**
  （`orz_source_manifest.sha256` 已随源码修改重新生成，1406 文件）。

## 4. 边界与登记

- `workspace.run_script` 只接受注册动作实例；嵌套脚本静态拒绝；PTC 步骤
  仍受既有权限/ACAF/模式门逐行约束（ReadOnly 下写动作在发放时得到
  `step=policy` receipt，fail-closed 可诊断）。
- taint 动作组合禁令仍为设计项（运行时未实施）；PTC 组合不新增策略面，
  每步独立过门，适配层 PolicyDenied 归一化已预留。
- 注册板块在无探针间隙（checkpoint 轮）不刷新、保留上一轮内容
  （`probe=None` 时投影只做 bundle 过滤；调用方不刷新即保留）。
- `workspace.index` 目标 `project_doc_index` 为非工作工具不探不标；
  mode=off 下按钮仍显示、发放时按模式门拒绝（沿用 S2 审计边界，不静默
  隐藏）。
- 脚本 trace 事件的 `script_step`/`as` 放入 `upstream`（`TraceEvent`
  结构未扩展，序列化形状不变）。
- 事件面无新增事件类型/Schema 变更：内部动作复用 ToolStarted/ToolCompleted
  v0.1 schema（`tool`=动作名、合成 `call_id`），Python verifier 无需变更。

## 5. 审计结论

切片 S3 的三项验收点（trace 只读服务、PTC run_script 生产化、Profile/Bundle
按钮组加载）全部有实现入口与测试证据；设计-实现符合（含 POC 契约逐项
同构移植：`$ref` 静态/运行时校验、上限、失败信封保留内层 step/code +
script_step）；未发现绕过既有门的执行路径。S4（端到端测试、实施审计与
正式组件决策门材料）按 TODO/BACKLOG 继续。
