# P0-C orz 内嵌集成 S4 实施审计（2026-08-16）

> 范围：CLASSICAL-EXEC-ASSISTANT 内嵌集成切片 S4——单步超时下沉 host 层
> （进程树收口）、脚本 tool-round 预算消耗、端到端测试补齐、正式组件决策门
> 材料登记。权威：设计
> [CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md](../CLASSICAL_EXECUTION_ASSISTANT_DESIGN_2026-08-13.md)
> §8（S3 审查收口 S4 登记段，2026-08-16 S4 落地段）；用户逐项裁决记录于
> S3 审计 [§6](GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md)；待办路由：
> [BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3 与 [TODO](../../TODO.md) P0-C。

## 1. 验收点 → 实现映射

| S4 验收点 | 实现位置 | 证据 |
|---|---|---|
| 单步超时下沉 host 层、host 负责进程树收口；脚本无总墙钟（复核裁决撤销 30s 含进程时间语义，见 §8） | `host.rs`：`LoopHost::call_tool_with_timeout`（显式覆盖；默认实现委托 `call_tool`）；`orz-host/src/lib.rs`：`call_tool_inner(timeout_override)`——`effective_timeout = min(覆盖, 配置预算)`，到期 `kill_active` 进程树收口（Windows Job Object/TaskKill 路径不变）；`console.rs`：`run_script_with_limits` 每步传 `None`（host 配置预算独立约束，不传收缩剩余）；`controller.rs`：`run_host_tool_with_timeout` 透传覆盖 | `orz-host tests::call_tool_with_timeout_override_is_honored`（10s 配置 + 1.2s 覆盖 → 按覆盖截止、会话存活）；`console::tests::script_host_timeout_maps_to_script_timeout_with_script_step`（`timed_out` 结构化信号 → `script_timeout` + `script_step`、每步覆盖为 `None`=host 默认超时） |
| 脚本消耗 tool-round 预算：直接订单 1 / 脚本每步 1；发放前预检；按实际步数减计；下一轮预算块机械反映 | `console.rs`：`issue_action_inner(..., timeout, consumed)`——Host 越过执行边界 +1、TraceRead 执行/查无 +1、脚本每步经 `consumed` 累计；`controller.rs`：`issue_pending_console_order` 发放前预检（剩余 = max − (当前轮 1 + 已用)；脚本长度 > 剩余 → `step=protocol` + `code=budget_insufficient` 零执行拒绝、不消耗）；`agent_loop.rs`：`tool_rounds = tool_rounds + consumed + 1`，下一轮 remaining 块机械反映、耗尽同走最后无工具轮 | `console::tests::script_consumes_only_actually_executed_steps`；`script_policy_denied_step_consumes_nothing`；`direct_order_host_timeout_maps_to_tool_timeout`；`controller::tests::console_s4_budget_precheck_rejects_over_budget_script`（required=2/remaining=1 拒绝、零执行、下一轮 REMAINING: 1）；`console_s4_script_consumes_budget_and_next_block_reflects`（2 步 → 下一轮 REMAINING: 1） |
| 端到端测试（FakeProvider 完整任务会话；checkpoint 轮板块保留；超时/预算边界） | `controller::tests`：`console_s4_full_session_script_trace_feedback_next_order`（写 run_script → 发放 → trace 读取 → 结果栏反馈 → 下一订单）、`console_s4_checkpoint_round_retains_probe_filtered_registration`（取消落在 checkpoint 轮结束后、下一轮刷新前，断言保留探针过滤内容）、`console_s4_budget_precheck_*` / `console_s4_script_consumes_*` | 见各测试；orz-loop 392 通过 / 0 失败 / 3 ignored |
| 实施审计与正式组件决策门材料 | 本文档 + [决策门材料清单](#6-正式组件决策门材料) | TODO/BACKLOG/索引/ADR 同步 |

## 2. 实现要点

### 2.1 单步超时（host 层收口）

- `LoopHost` 新增默认方法 `call_tool_with_timeout(name, args, call_id,
  timeout)`，默认等价 `call_tool`——既有测试 host 无需改动；生产
  `OrzHost` 覆盖实现，把现有 `call_tool` 主体抽为固有方法
  `call_tool_inner(timeout_override)`。
- host 侧有效预算 = `min(覆盖, self.tool_timeout)`：调用方只能收紧、不能
  放宽 host 配置上限。到期仍走既有 `ToolError::Timeout` 路径（执行态
  `kill_active` 进程树收口；等待态不误杀），错误文案携带实际使用的预算。
- 超时信号结构化上浮：`ToolResult` 新增 `timed_out: bool`，`run_host_tool`
  在 `ToolError::Timeout` 分支置位（成功路径保留 host 传来的值）；控制台
  适配层据此映射（不解析文案前缀）——直接订单失败信封
  `step=execute`+`code=tool_timeout`；脚本 runner 归一化为
  `step=execute`+`code=script_timeout` 并携带 `script_step`/`action`。
- 脚本层不再持有总截止（2026-08-16 二次审查收口后的复核裁决，见 §8）：
  撤销「30s 总墙钟含进程时间」语义——每步不传收缩剩余（`None`），由 host
  每调用超时独立约束（配置预算，默认 5 分钟，进程树收口不变）；脚本层保留
  8 步 / 4MiB / tool-round 预算上限。

### 2.2 tool-round 预算

- 计数规则（用户裁决）：每个实际执行动作计 1 单位——直接订单（Host /
  TraceRead）1；脚本每步 1；执行前被拒的步骤（契约/目标/策略拒绝）不计数；
  host 已启动并被截止/失败的步骤计数。`issue_action_inner` 通过共享
  `&mut u32 consumed` 累计（脚本内层步骤直接写同一计数器）。
- 发放前预检在 `issue_pending_console_order`：剩余 = `max_tool_rounds −
  (tool_rounds + 1)`（当前模型轮已消耗 1 单位）。脚本长度/直接动作 1 > 剩余
  → 零执行拒绝：订单清槽、`step=protocol` + `code=budget_insufficient` +
  `upstream{action, required, remaining, tool_rounds, max_tool_rounds}`、
  失败 trace commit、不消耗预算。未知动作/畸形脚本不预检（交注册表/契约
  校验产生对应错误码）。
- 消耗回流：`issue_pending_console_order` 返回实际消耗单位，`agent_loop`
  先加消耗再加当前轮 1；下一轮 remaining 块机械反映；耗尽后进入最后
  无工具轮（D-8 语义不变，tool_rounds_limit 门事件携带合计值）。

### 2.3 测试补充（S4 新增 9 项：orz-loop +8、orz-host +1）

- orz-loop 单元：`script_host_timeout_maps_to_script_timeout_with_script_step`、
  `script_consumes_only_actually_executed_steps`、
  `script_policy_denied_step_consumes_nothing`、
  `direct_order_host_timeout_maps_to_tool_timeout`。
- orz-loop e2e：`console_s4_full_session_script_trace_feedback_next_order`、
  `console_s4_budget_precheck_rejects_over_budget_script`、
  `console_s4_script_consumes_budget_and_next_block_reflects`、
  `console_s4_checkpoint_round_retains_probe_filtered_registration`。
- orz-host：`call_tool_with_timeout_override_is_honored`。
- 二次审查收口增量：新增 4 项（L2 e2e ×2、L4 e2e、事后截止单测）；用户
  复核裁决撤销 30s 总墙钟后移除事后截止单测，并将墙钟+字节测试更名
  `run_script_enforces_byte_limits`——orz-loop 392 → 395（净 +3，另
  orz-host +1 不变）。

## 3. 设计-实现符合性

- 用户裁决逐项落地：单步超时由 host 层收口（未引入脚本层 timeout 替代）；
  trace 生命周期会话级（S3 已定，S4 未改变）；脚本消耗 tool-round 预算
  （预检/实际步数减计/下一轮机械反映均实现并测试锁定）；`assistant.trace`
  查无 id 定案（S3 已实现）未回归。
- 既有门未被绕过：脚本内层步骤与直接订单仍经 `run_console_target` →
  `run_host_tool` 全链路（权限/ACAF/模式门/事件链）；预算预检只发生在发放
  出口，不改变模型面写单语义。
- 超时/超限失败信封保留 `script_step`（逐步与最终两条路径均有测试）。

## 4. 验证证据

- `cargo test -p orz-loop --lib`：**392 通过 / 0 失败 / 3 ignored**（S3 收口
  384 + S4 新增 8；另 orz-host 新增 1，合计新增 9 项）。二次审查收口与
  超时语义复核（§8）后 orz-loop **395 通过 / 0 失败 / 3 ignored**
  （392 + 新增 4 − 撤销 1）。
- `cargo test -p orz-host --lib call_tool_with_timeout_override_is_honored`
  单独运行通过；`cargo test -p orz-host --lib` 全量（216 项）：209 通过
  （含本切片新增 1 项）/ 3 失败 / 4 ignored——3 项失败为既有
  `acp_server::tests::second_prompt_continues_hash_chain`、
  `session_prompt_produces_valid_journal_chain`（HEAD 基线上已失败，与 S4
  无关）与 `call_tool_timeout_kills_process_tree`（并行全量下偶发、单测
  通过，既有计时敏感波动）。
- `cargo fmt --check` 通过；`cargo clippy -p orz-loop --all-targets` 与 HEAD
  基线告警数一致（17 lib / 22 test），无新增告警。orz-host 的 clippy 受
  `orz-tools-api` 构建脚本缺 `protoc` 的环境限制阻塞（与本次改动无关）。
- `cargo check -p orz-host -p orz-tui -p orz-bin -p orz-assurance` 通过。

## 5. 已知边界

- 脚本无总墙钟（2026-08-16 超时语义复核裁决，见 §8）：每步由 host 每调用
  超时独立约束（配置预算，默认 5 分钟）；权限/ACAF 等 controller 侧等待不
  计入任何单步覆盖。host 配置预算为**经注册表执行的调用**的硬上限——
  host-owned 同步工具（`project_doc_index`/`browser_read`/`pdf_read`/PDF
  路由 `web_fetch`）不经 timeout 包装（既有行为，无法中断在途同步工作），
  与直接订单语义一致，不由脚本层事后判失败。
- 预算预检把当前模型轮计为 1 单位（该轮结束后 `tool_rounds += 1` 必然发生），
  因此 max=1 时该轮写单会被零执行拒绝——与「预算耗尽后进入最后无工具轮」
  语义一致，测试已锁定。
- trace 会话结束归档/摘要为可选增强（S3 登记），S4 端到端测试未发现必要
  性证据，暂不实施。
- orz-host 既有 2 项 acp 测试失败与 1 项计时敏感测试波动不属于本切片范围，
  已登记待查（不在 S4 验收点内）。

## 6. 正式组件决策门材料

正式组件决策门（「小样全面达标后裁决；不达标即撤」）材料清单：

1. 小样 1：`prototype/classical_console/sample1_result.json`（控制台路由
   POC；2026-08-13 28/28 检查通过；2026-08-16 复跑 `smoke_test.py`
   90/90 通过并落盘结果工件，见 §8 M2）。
2. 小样 2：`prototype/classical_console/sample2_result.json`（编辑执行器
   `workspace.search_replace`，2026-08-15 用户裁决通过 + 独立判定一致）。
3. 小样 3：`prototype/classical_console/sample3_result.json`（机械组合脚本
   模式，2026-08-15 用户裁决通过 + 独立判定一致）。
4. 实施审计汇总：S1 落地与全面检查修复（无独立审计文件，登记于
   [BACKLOG](../BACKLOG_AND_PRIORITIES.md) 3 与 [TODO](../../TODO.md) P0-C、
   索引 2026-08-15 S1 行）；S2
   [`GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md`](GAP_CLASSICAL_EXEC_S2_IMPL_AUDIT_2026-08-15.md)、
   S3 前置 [`GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md`](GAP_CLASSICAL_EXEC_S3_PRELUDE_POLICY_DENIAL_IMPL_AUDIT_2026-08-15.md)、
   S3 [`GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md`](GAP_CLASSICAL_EXEC_S3_IMPL_AUDIT_2026-08-15.md)、
   S4（本文档）。
5. 待决策门裁决时汇总：POC 与生产实现对照、测试/门禁证据、已知边界清单。

## 7. 审计结论

切片 S4 的三项验收点（host 层单步超时收口、tool-round 预算消耗、端到端
测试补齐）全部有实现入口与测试证据；设计-实现符合（用户裁决逐项落地）；
未发现绕过既有门的执行路径。S4 闭合后，P0-C CLASSICAL-EXEC-ASSISTANT 的
剩余未闭合项为：正式组件决策门（材料已齐备，待用户裁决）。

2026-08-16 二次全面审查收口与超时语义复核（§8）后上述结论维持：测试
392→395、决策门材料补齐、超时语义对齐成熟设计（Codex/Grok 对照）。

## 8. 全面审查收口（2026-08-16 二次）

基于 S4 全面审查（设计/实现/符合性三路）与用户指示，对全部发现处理如下：

- **超时语义复核（用户复核裁决，2026-08-16 二次之后）**：撤销「30s 总墙钟
  含进程时间」语义——脚本每步不传收缩剩余（`None`），由 host 每调用超时
  独立约束（配置预算，默认 5 分钟，进程树收口不变）；依据 Codex
  `command/exec timeoutMs` 与 Grok Build `toolset.*.timeout_secs`/
  `ProcessScope` 成熟设计对照。删除 `MAX_SCRIPT_WALLCLOCK_SECONDS`/
  deadline/事后核对；`run_script_enforces_wallclock_and_byte_limits` 更名
  `run_script_enforces_byte_limits`；移除
  `script_step_overrunning_wallclock_fails_after_step`。
- **M1（host 硬上限表述与早退路径）**：审计 §5 措辞修正——配置预算为
  「经注册表执行的调用」的硬上限；host-owned 同步工具（`project_doc_index`
  /`browser_read`/`pdf_read`/PDF 路由 `web_fetch`）不经 timeout 包装为
  既有行为，与直接订单语义一致（不由脚本层事后判失败）。
- **M2（决策门材料缺失）**：小样 1 原无结果工件。2026-08-16 复跑
  `prototype/classical_console/smoke_test.py`（90/90 通过）并落盘
  `sample1_result.json`（含运行溯源/检查清单），审计 §6 材料清单第 1 项
  指向真实文件。
- **L1（30s 总墙钟边界）**：由超时语义复核裁决解决——总墙钟含进程时间的
  语义撤销，每步与直接订单同界；controller 侧 gate 等待边界在审计 §5
  显式登记。
- **L2（预算预检掩盖内层错误）**：`issue_pending_console_order` 预检改为
  先对脚本做静态校验（无执行）——校验失败或超上限（>8 步）不预检，交
  注册表/契约校验产生真实错误码（`unknown_service`/`invalid_arguments` 等）；
  新增 e2e 两项（未知动作、9 步超上限）。
- **L3（测试计数表述）**：§2.3/§4 改写为「orz-loop +8、orz-host +1，合计
  新增 9 项」；orz-host 全量按 216 总数（209 通过/3 失败/4 ignored）写明。
- **L4（max=1 边界测试）**：新增 e2e
  `console_s4_max_one_round_rejects_order_with_zero_remaining`
  （remaining=0 零执行拒绝，显式锁定审计 §5 语义）。
- **L5（trait 默认实现退化面）**：`LoopHost::call_tool_with_timeout` 文档
  明示默认实现忽略覆盖、未覆盖 host 不保证单步受控（生产 `OrzHost` 已
  实现，无实际风险）。

验证：orz-loop **395 通过 / 0 失败 / 3 ignored**；`cargo fmt --check` 通过；
clippy 告警数与基线一致（17 lib / 22 test，无新增）；仓库门禁 valid、0 错误。
