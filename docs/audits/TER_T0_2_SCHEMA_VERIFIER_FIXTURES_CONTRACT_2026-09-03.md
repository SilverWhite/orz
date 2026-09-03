# TER T0.2 schema/verifier/fixtures 先行 + 黑板分区契约（2026-09-03）

> 主题：工具执行层改革（TER）M0 设计门第 2 步——事件面机器合约先行：
> ① 新事件 `budget_cue_injected`；② `tool_completed` 增截断三字段
> （payload v0.1 → v0.2）；③ `tool_running` idle-kill 形态
> （status=idle_killed + reason）；④ 黑板 `section=processes` /
> `section=env` live 分区渲染契约与越权边界（对齐 session 面先例）。
> 本步只改机器合约（schema/verifier/fixtures/测试），**未写业务代码**
> （生产者接线在 M1 T1.x）。
> 依据：[`TODO2.md`](../../TODO2.md) T0.2；设计稿
> [`TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md`](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.1–§3.6/§10-S0；[`BACKLOG2.md`](../BACKLOG2.md) TER-0；
> T0.1 核对表
> [`TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md`](TER_T0_1_DEFAULT_CONVERGENCE_CHECKLIST_2026-09-03.md)。

## 1. Schema diff 摘要（版本号 bump）

| 文件 | 变更 | 版本语义 |
|---|---|---|
| [`run-event-v0.2.schema.json`](../../runtime/run-event-v0.2.schema.json) | 枚举 53 → 54（+`budget_cue_injected`）；description 补 TER 注记 | 事件面枚举 bump |
| [`budget-cue-injected-event-payload-v0.2.schema.json`](../../runtime/budget-cue-injected-event-payload-v0.2.schema.json) | 新增（v0.2 轨） | 新事件载荷 |
| [`tool-completed-event-payload-v0.2.schema.json`](../../runtime/tool-completed-event-payload-v0.2.schema.json) | 由 v0.1 派生 + `output_truncated` / `total_bytes` / `output_object_id` | tool_completed 载荷 **v0.1 → v0.2**（v0.1 文件继续服务 v0.1 replay 轨） |
| [`tool-running-event-payload-v0.2.schema.json`](../../runtime/tool-running-event-payload-v0.2.schema.json) | +`status`（闭合枚举，当前仅 `idle_killed`）+ `reason` + 成对 if/then | v0.2 轨内扩展（idle-kill 形态） |

## 2. 新事件 `budget_cue_injected`（F6 push 档）

机械单写记录；默认 off 时零注入（PUSH→PULL 纪律保持，ADR §14.40 第 10
项）。载荷为中性事实，不带建议：

| 字段 | 类型/约束 | 语义 |
|---|---|---|
| `remaining_seconds` | int ≥ 0 | 读取时刻剩余评测墙钟（秒） |
| `rounds_used` | int ≥ 0 | 该 run 已用工具轮数 |
| `threshold_seconds` | enum 600 / 300 / 120 | 刚跨过的档位阈值 |

verifier 跨字段规则（`_verify_v02_budget_cue_injected`）：
每 run ≤ 4 次注入；`remaining_seconds` 必须严格小于 `threshold_seconds`
（只在阈值以下触发）。

## 3. `tool_completed` W-F13b 截断三字段（v0.2 轨）

| 字段 | 类型/约束 | 语义 |
|---|---|---|
| `output_truncated` | `const: true`（出现即截断） | 显式截断标记 |
| `total_bytes` | int ≥ 0 | 截断前真实单调输出字节 |
| `output_object_id` | string ≥ 1 | 持久化输出检索对象指针（pattern / 行区间 / 尾部 N 行） |

verifier 配对规则（`_verify_v02_output_truncation`）：
`output_truncated` ⇒ 必须有 `total_bytes`；`output_object_id` ⇒ 必须有
`output_truncated` + `total_bytes`（对象指针不与未截断投递配对）。

## 4. `tool_running` idle-kill 形态

- 缺省（无 `status`）= S5-2 auto-background 中间回报（原语义不变）。
- `status: idle_killed` + `reason` = 5min 无活跃（输出字节 + CPU 辅助判定）
  机械 kill 的后台生命周期事件；语义上**晚于**该 call 自身的
  `running: true` `tool_completed`，每 call_id 至多一次。
- 链规则（`_verify_v02_tool_running` 扩展）：idle-kill 必须引用同
  run/工具/call_id 的既有 mid-run + 已完成 auto-bg call；不得先于完成
  事件；不引入第二个 tool_completed。
- 闭合枚举：其它生命周期状态入事件需 schema 变更（fail-closed）。

## 5. 黑板 `section=processes` / `section=env` live 渲染契约与越权边界

> 对齐先例：`section=session`（CONTEXT_SCAFFOLDING_PULL_REDESIGN
> 2026-08-21，O4/O5 口径：越权组合由渲染层文本 + 工具级回达测试锁定）；
> 本契约是 T1.6 / T1.8 / T1.12 的机器实现输入，M0 只定契约不写代码。

### 5.1 processes 分区

- live-only、读取时现算（≤1s 新鲜度），不逐秒写事件；状态跃迁（start /
  auto-background / complete / idle 首现 / kill）才落 journal（tool_running
  形态，见 §4）。
- 行结构：`task_id` / 命令摘要（≤80B，含身份摘要语义沿用 F4 cmd_preview
  纪律，不落全命令） / `elapsed_ms` / `status`（running | idle |
  waiting_input | completed | killed）/ 活跃度（窗口内输出字节增长 +
  CPU 时间）/ `total_bytes` / `killable`。
- 渲染预算：整分区 ≤ 8KiB 字符（同 session 面先例）；超限机械截断并标注。
- kill 动作面：经现有后台生命周期工具/等价动作（T1.6 核对具体形态）；
  模型面不暴露 `&` / `is_background`。

### 5.2 env 分区（白名单 PULL 面）

- 内容=机械层代码工具环境快照：工具 / 语言 / 包 / 版本存在性与版本、
  关键输入在场判定、连通性判定（通/不通，单项 ≤1–2s，整快照 ≤5s）。
- 明确不出：任务专属结论、allowlist 内容、"无 mips 工具链"类压测提示
  （P4 压测信息最小化）——该分区只给通用快照，不给"须知环境"事实。
- Linux 端本地快速判定闭环（T1.12）；Windows 快速性依赖 W-F12（T2.2）。

### 5.3 越权边界（两分区共用）

1. live-only：`epoch`（跨 epoch 回看）与 `receipt_id`（归档点读）参数
   组合一律显式拒绝（exit_code 1 + 结构化错误），不得静默回退整段读取；
2. 只读渲染分区：无写入/变更副作用；kill 只能经生命周期动作面；
3. 不入归档：会话存档包（P2-13 单 gzip）只含会话 + 黑板持久分区，
   processes/env 读取快照不累积、不落盘；
4. env 白名单键集合由实现侧登记（T1.12），越权键=渲染层拒绝。

## 6. verifier / fixtures / tests 落地清单

- [`run_event_journal_validation.py`](../../assurance/run_event_journal_validation.py)：
  V02 注册表 +`tool_completed`（v0.2 载荷）与 `budget_cue_injected`；
  新增 `_verify_v02_output_truncation` / `_verify_v02_budget_cue_injected`；
  `_verify_v02_tool_running` 扩展 idle-kill 链规则；validate_journal_text
  挂钩。
- fixtures（`runtime/fixtures/run-event-v0.2/`）：
  `budget-cue-injected.minimal.valid/.constraint.invalid` + envelope valid；
  `tool-completed.minimal.valid/.constraint.invalid`（v0.2 轨）；
  extras `tool-completed.output-object.valid`、
  `tool-running.idle-killed.valid`、
  `tool-running.idle-killed-missing-reason.constraint.invalid`。
- [`generate_run_event_fixtures.py`](../../scripts/generate_run_event_fixtures.py)：
  V02_EVENT_TYPES / V02_PAYLOAD_EVENTS / PAYLOAD_GOOD_V02 / PAYLOAD_BAD_V02
  / EXTRA 映射 + 时间戳 override 同步（保持未来再生成一致）。
- 测试：conformance 枚举 53→54；journal 校验新增 idle-kill / 截断 /
  budget-cue 三族用例。**串行全量 273 passed / 0 failed**（上述两文件）。

## 7. 验收对照

- [x] orz-assurance（Python 校验分支）新增/扩展分支，全量测试绿；
- [x] fixtures 先行（payload/envelope/extras 落盘并过 conformance）；
- [x] schema 版本号 bump（run-event 枚举 53→54；tool_completed 载荷
  v0.1→v0.2；tool_running v0.2 扩展）；
- [x] 黑板 processes/env 分区渲染契约与越权边界定稿（§5，供 T1.x 引用）；
- [x] 未写业务代码（无生产者/渲染器改动）。

> 状态登记：T0.2 完成（TODO2.md 勾选）；T0.3（ADR-0010 §14.xx 候选登记）
> 与 T0.4（放行签名）待续。
