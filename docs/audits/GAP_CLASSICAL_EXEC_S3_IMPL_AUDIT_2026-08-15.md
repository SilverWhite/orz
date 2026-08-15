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
| `assistant.trace` 只读服务接线（按 trace_id 取回有界日志；读操作入审计） | `console.rs`：`ActionKind::TraceRead` + 注册 `assistant.trace`（输入契约 trace_id/tail、响应契约 trace_id/request_id/events/truncated）；`issue_action` TraceRead 分支（`TraceStore` 短锁快照、默认 tail=20、`not_found` fail-closed——2026-08-16 定案 `step=execute`）；`controller.rs` 发放链对内部动作补 ToolStarted/ToolCompleted 事件（复用 v0.1 事件面，无 Schema 变更） | `console::tests::trace_read_service_returns_bounded_events`；`trace_read_missing_trace_fails_not_found`；`controller::tests::console_s3_trace_read_issuance_flow`（事件面断言 exit_code=0） |
| `workspace.run_script`（PTC）生产化：线性脚本 + `$ref`、逐行契约校验 + trace、上限 8 步/30s/4MiB、fail-closed | `console.rs`：`ActionKind::RunScript` + 注册 `workspace.run_script`；`static_validate_script`（名称唯一/服务已知/禁嵌套/`$ref` 形状·作用域·类型）、`substitute`（运行时 `$ref` 解析）、`run_script`/`run_script_with_limits`（逐行经 `issue_action` 复用注册表/契约/目标/执行/验证五步链；失败保留内层 step/code + `script_step`；后续步骤不执行） | `console::tests::run_script_executes_steps_with_refs`；`run_script_static_validation_rejects_bad_scripts`；`run_script_step_failure_stops_and_preserves_inner_error`；`run_script_enforces_wallclock_and_byte_limits`；`run_script_rejects_over_cap_scripts`；`controller::tests::console_s3_run_script_issuance_flow`（内层 read_file 事件带 `.s1`/`.s2` call_id） |
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
- 上限：单订单 8 步（POC 定档 20 → 2026-08-16 审查收口）/ 30s 墙钟 /
  4 MiB 累计响应（含最终 `result` 重复计算）；超时与超限结构化返回
  `script_timeout` / `script_response_limit`。
- 注册板块投影 = Profile/Bundle 加载集 ∩ 探针完整集：会话场景键取
  `host.tool_policy()`（Interactive=标准 / ReadOnly / Benchmark）；
  Host 动作的工作工具目标必须在 `ToolProbeSnapshot.complete`；非工作工具
  （如 `project_doc_index`）不探不标、按注册表声明保留（与模型可见工具
  投影语义一致）；内部动作无 host 目标、不做探针过滤。
- 单订单步数上限 8（`MAX_SCRIPT_STEPS_PER_ORDER` + schema `maxItems`
  同步；9 步脚本零执行拒绝，测试锁定）。
- `assistant.trace` 查无 trace_id 定案 `step=execute` + `code=not_found`
  （2026-08-16）：服务已解析、契约已过、存储查询失败属执行阶段；失败
  信封按契约附本订单有界 trace 尾部（可区分无效 id 与环形淘汰）。
- checkpoint 轮（无探针间隙）**跳过注册板块刷新、保留上一轮内容**
  （2026-08-16 审查收口：agent_loop 加 `pending_checkpoint` 守卫；其他
  无探针轮次仍只做 bundle 过滤）。
- 脚本内允许调用只读内部动作（含 `assistant.trace`）：非嵌套脚本、逐行
  过五步链与既有门；读步骤输出进入脚本 steps/result 并受 4MiB 上限约束
  （2026-08-16 登记为有意边界）。
- trace 生命周期为会话级：TraceStore 挂在会话级控制器、会话开始时空、
  结束即弃、侧车恢复不携带；50 条为会话内环形上限，跨会话 trace_id 一律
  `not_found`（2026-08-16 登记为有意边界）。
- 注册不变式（2026-08-16 补齐）：内部动作携带 host 目标注册即拒绝；
  内部动作类（TraceRead/RunScript）全局唯一；bundle 至少启用一个场景；
  嵌套脚本按 `ActionKind::RunScript` 拒绝（非服务名比较）。

## 3. 测试证据

- `cargo test -p orz-loop --lib`：**377 passed / 0 failed / 3 ignored**
  （S3 新增 console 单测 8 项：注册不变式 1 + bundle/探针投影 1 + trace
  读 2 + 脚本成功/静态校验/失败收口/上限 4；新增 controller 端到端 2 项：
  run_script 发放与 trace 读发放；更新 S2 注册板块断言 1 项）。
  2026-08-16 审查收口后复测：**384 passed / 0 failed / 3 ignored**
  （新增 7 项单测，见 §6）。
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
- 注册板块在无探针间隙（checkpoint 轮）跳过刷新、保留上一轮内容
  （2026-08-16 审查收口：`pending_checkpoint` 守卫；其他无探针轮次仍只做
  bundle 过滤）。
- `workspace.index` 目标 `project_doc_index` 为非工作工具不探不标；
  mode=off 下按钮仍显示、发放时按模式门拒绝（沿用 S2 审计边界，不静默
  隐藏）。
- 脚本 trace 事件的 `script_step`/`as` 放入 `upstream`（`TraceEvent`
  结构未扩展，序列化形状不变）。
- 事件面无新增事件类型/Schema 变更：内部动作复用 ToolStarted/ToolCompleted
  v0.1 schema（`tool`=动作名、合成 `call_id`），Python verifier 无需变更。

## 6. 全面审查收口（2026-08-16）

> 范围：S3 三层审查（设计合理性/实现合理性/符合性）后的裁决收口；用户
> 逐项裁决，S4 项只登记不实施。

### 6.1 本切片已实施修复

- F1 单订单步数上限 20 → 8：`MAX_SCRIPT_STEPS_PER_ORDER` + schema
  `maxItems` + 描述同步；`run_script_rejects_over_cap_scripts` 锁定 9 步
  零执行拒绝。
- F2 `assistant.trace` 查无 id：`step=registry` → `step=execute` +
  `code=not_found`（服务已解析、存储查询失败属执行阶段；失败信封附有界
  trace 尾部）；测试与设计/ADR 同步。
- F3 checkpoint 轮跳过注册板块刷新：agent_loop 注册板块刷新加
  `pending_checkpoint.is_none()` 守卫，保留上一轮探针过滤后的内容；其他
  无探针轮次仍 bundle-only。e2e 断言列入 S4。
- F4 注册不变式补齐：内部动作携带 host 目标注册即拒绝（双向 fail-fast）；
  内部动作类（TraceRead/RunScript）全局唯一；bundle 至少启用一个场景；
  嵌套脚本拒绝由服务名比较改为按 `ActionKind::RunScript`（配合唯一性防
  第二个脚本类名称绕过）。`registry_rejects_invalid_internal_kind_and_bundle`
  锁定。
- F5 最终 `result` 重复计算超限分支：失败信封 `upstream` 补
  `script_step`（最后一步）+ `action`；
  `run_script_final_response_limit_reports_script_step` 锁定。
- F6 测试补齐：运行时 `$ref` 失败结构化（`substitute_runtime_ref_failure_
  is_structured`）、`$ref` 形状/字段缺失静态拒绝、数组 items 路径
  （`run_script_array_items_ref_resolves`）、`tail>200` 契约拒绝
  （`tail_beyond_contract_rejected`）、脚本内 TraceRead
  （`run_script_step_trace_read_allowed`）。

### 6.2 S4 登记项（不在本切片实施；2026-08-16 已由 S4 实施闭合）

- 30s 墙钟语义定案：总墙钟 + 单步受控；脚本截止时间下沉 host 层，由 host
  层负责进程树收口（用户裁决），不得用脚本层 timeout 替代进程收口。
- 脚本消耗 tool-round 预算：每个已执行动作（直接订单 1、脚本每步 1）计
  1 个预算单位；发放前预检（脚本长度 ≤ 剩余预算，不足零执行拒绝并显式
  错误码）；按实际执行步数减计；下一轮预算块机械反映。
- trace 会话级生命周期登记完成；会话结束归档/摘要为可选增强（S4 视
  端到端测试结果决定）。

> 2026-08-16：上述前三项（单步超时、tool-round 预算、trace 会话级）已实施
> 并登记，见 [`GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md`](GAP_CLASSICAL_EXEC_S4_IMPL_AUDIT_2026-08-16.md)。

## 5. 审计结论

切片 S3 的三项验收点（trace 只读服务、PTC run_script 生产化、Profile/Bundle
按钮组加载）全部有实现入口与测试证据；设计-实现符合（含 POC 契约逐项
同构移植：`$ref` 静态/运行时校验、上限、失败信封保留内层 step/code +
script_step）；未发现绕过既有门的执行路径。S4（端到端测试、实施审计与
正式组件决策门材料）按 TODO/BACKLOG 继续。
