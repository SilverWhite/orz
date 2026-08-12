# GAP-ACAF-SLICE2-FAILCLOSED 实施审计（2026-08-13）

> 状态：**Slice 2 fail-closed 机制已实施并验证**（D-13~D-16）；生产翻转仍由
> `ORZ_ACAF_FAIL_CLOSED=1` 显式开启，默认保持影子模式——最终启用由用户观察
> 台账后裁决。范围：ADR-0011 §7 Slice 2（fail-closed 切换里程碑）、设计文档
> §11 D-12~D-16、Slice 2B 审计 §6 核查清单 ③④⑥⑧。
> 设计权威回查：[`ADR-0011`](../../adr/ADR-0011-authenticated-control-and-action-fabric.md)
> 与 [`AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md`](../../docs/AUTHENTICATED_CONTROL_AND_ACTION_FABRIC_DESIGN_2026-08-09.md)。

## 1. 已变更（核心）

### 1.1 Schema / fixture 先行（主仓）

- `runtime/control-ticket-rejected-event-payload-v0.2.schema.json`：
  `reject_code` 枚举 +3 —— `missing_target_argument`（D-14）、
  `missing_snapshot_store` / `missing_goal_context`（D-15）；
- `scripts/generate_run_event_fixtures.py`：新增
  `EXTRA_V02_PAYLOAD_POSITIVES`（三个新拒绝码各一个正例，均
  `ticket_id: null` —— 预签发拒绝无票据）；`FIXTURES_README_V02` 补说明；
  v0.2 payload 计数 22 → 25；
- `scripts/check_repository.py`：三个额外正例注册进
  `run_event_v02_payload_positive_contracts`（11 → 14）；
- 生成三个新 fixture 文件，`runtime/fixtures/run-event-v0.2/README.md`
  同步。

### 1.2 机制核心（orz-assurance）

- `TicketKind::forbids_activation()`（Orientation 唯一禁止）；动作票
  **activation 可选**——检索 lane 的 network 票可绑定真实 activation
  （D-13），主 lane 保持 null；`issue_ticket` 的
  `UnexpectedActivation` 检查改为 `forbids_activation()`；
- `RejectCode` +3（含 `as_str`）。

### 1.3 签发器（orz-bin）

- `sign_network_v1` 接受**可选** `activation_id`（新
  `handle_sign_optional_activation`：缺省/null → None，主 lane 兼容）；
  其余动作方法协议不变；方法清单文档同步。

### 1.4 controller（orz-loop）

- **fail-closed 开关**：`AgentLoopController.acaf_fail_closed` +
  `with_acaf_fail_closed(bool)`；main.rs 接 `ORZ_ACAF_FAIL_CLOSED`
  （值为 1/true 时启用，大小写不敏感；0/false/未设置保持影子——
  复核修复 2026-08-13，由“存在即开”改为取值语义）；**D-15 启动 fail-fast**：
  fail-closed + 未配置 fabric →
  `run_turn_with_guards` 直接 `Assurance` 错误，无静默降级；
- **`TicketGate`**（Proceed / Blocked{code,detail}）：`ticket_flow`、
  `run_action_ticket`、`acaf_action_event`（file/network/command/run_tests）
  全部返回 gate；`acaf_control_event`、`write_close_record` 同；
- **D-14**：`file_path` / `url` / `command` 缺失或空 → fail-closed 时
  journal `control_ticket_rejected(missing_target_argument, ticket_id=null)`
  + Blocked（工具不执行、无 ToolStarted）；影子模式保持既有静默 skip
  （注册边界，既有测试锁定）；复核修复 2026-08-13：`file_path`/`url`
  空串（含纯空白）与缺失同码 `missing_target_argument`（fail-closed），
  影子仍走解析失败台账 `target_mismatch`（既有行为不变）；
- **D-15**：snapshot_store 缺失（file_write/command cwd 无法绑定）→
  `missing_snapshot_store`；goal_context 缺失 → `missing_goal_context`
  （ticket_flow / file 分支 / run_action_ticket 三处）；
- **D-16**：AcceptedContinue 的 GoalRevisionV1 票 Blocked →
  `update_goal` / contract_revision bump / next_goal / status 迁移**均不
  执行**，disposition 结果向父 Agent 表面 unauthorized（exit 1）；
  DispositionV1 / CloseV1 / OrientationV1 同 gate（fail-closed 下控制事件
  拒绝即不迁移）；
- **D-13 activation 线程化**：`LoopProfile.activation_id`（retrieval
  构造时携带真实 activation）→ `run_host_tool(activation_id)` →
  `acaf_action_event`/`run_action_ticket` sign+verify 均用 live
  activation（check 4 非自指）；
- 拒绝呈现：`refuse_ticketed_tool`（ToolCompleted error +
  `control_ticket_rejected:{code}`，无 ToolStarted，与 mode-off 拒绝同形）。
- **复核修复（2026-08-13）**：`run_action_ticket`/file 分支的消费期
  RPC 失败路径原重复记账（同一 `signer_unreachable` 记两条
  `control_ticket_rejected`），已改为单次记账；`AcafClient` 新增 e2e
  注入 seam（`inject_verify_failure`，doc(hidden)，按 ticket_kind 精确
  命中），用于覆盖签发后验票失败路径与 D-16 拒绝分支。

## 2. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-13 | 动作票 activation 规则 = 可选（Orientation 唯一禁止） | 检索 lane web_fetch 必须绑定真实 activation；主 lane 保持 null；签发器协议只在 network 方法开放可选参数 |
| D-14 | 缺失/空 target 参数：fail-closed 用 `missing_target_argument`（file_path/url 空串复核修复 2026-08-13 同码）；影子保留既有 `target_mismatch` 台账/静默 skip | D-14 语义显式；既有影子测试锁定空命令 target_mismatch（不破坏） |
| D-15 | 未配置 fabric = 启动 fail-fast（非逐票拒绝） | 设计文档 §11 D-15 允许二选一；启动期拒绝更早暴露配置错误；显式降级开关留未来 |
| D-16 | 终端 close（close_activation）Blocked → 激活保持开放（best-effort） | fail-closed 下无未持票状态迁移；运行终结路径不接受静默关闭 |
| D-17 | `refuse_ticketed_tool` 不喂 denial breaker（feedback None） | 票据拒绝是安全事件非权限拒绝；熔断语义（ADR-0010 §3.5.4）不动 |

## 3. 验证证据

- **orz-bin e2e（Windows-only，真实签发器）21/21**：新增 8 个 fail-closed
  测试 —— ① 未配置 fabric 启动拒绝；② 缺 url → missing_target_argument
  + 无 ToolStarted + ToolCompleted error；③ 缺 command 同；④ 死签发器 +
  fail-closed → signer_unreachable + 工具不执行；⑤ **检索 lane web_fetch
  票据绑定真实 activation（`retrieval-external_retrieval-…-00`）**；
  ⑥ continue 快乐路径：DispositionV1 + GoalRevisionV1 双票 consumed；
  ⑦ 验票 RPC 失败（注入 seam）→ 单次 `signer_unreachable` 拒绝 +
  工具不执行（回归锁：消费失败单次记账）；⑧ D-16 拒绝分支：GoalRevisionV1
  验票失败 → 状态不迁移 + disposition exit 1（注入 seam 覆盖原测试边界）；
- orz-assurance acaf **33/0**（+1：动作票 activation 可选/必须语义）；
- orz-signer **12/0**（+1：network 带 activation 签发 + 主 lane null 回归）；
- orz-loop **196/0/3**（既有影子/零事件回归全绿）；
- 全 workspace `cargo test --workspace -j2`：除 orz-host
  `call_tool_timeout_kills_process_tree`（**已知并行 flaky**——注释记载
  python 冷启动争用；单独重跑 1/1 通过，非本切片引入）外全绿；
- `check_repository.py` **valid**（新 fixture 入正例集合）；
- `python -m pytest runtime -q` **199 passed**；
- `git diff --check` 干净。

## 4. 边界登记

- **生产翻转待用户裁决**：机制与 env 开关已就绪，默认仍 shadow；
  `ORZ_ACAF_FAIL_CLOSED=1` 即 fail-closed；最终启用前建议按 Slice 2B
  §6 核查清单 ①-⑪ 过一遍探针矩阵（⑦⑨⑩⑪ 仍登记）；
- **D-16 拒绝分支 e2e 覆盖（复核修复 2026-08-13）**：真实签发器自洽，
  外部 kill 无法精确命中 GoalRevisionV1 验票时点；现由 `AcafClient`
  注入 seam（doc(hidden)，e2e 专用）在验票入口强制 RPC 失败，覆盖
  D-16 状态不迁移与单次记账两个分支，原“注入 seam 缺位”边界撤销；
- ACP 会话路径未接线 ACAF（沿用现状：controller 未挂 client，fail-closed
  默认 false，零行为变化）；
- 终端 close 在 fail-closed 被拒时激活保持 open（best-effort，运行已终结）；
- E2E Windows-only（DPAPI）；Linux 侧由 mechanism/signer 单测覆盖；
- 环境备注：本次验证触发 **D 盘写满 + target 159GB** → 按 Claude 记忆
  先例 `cargo clean` 释放 159.3GiB + `-j2` 冷缓存重建（protoc 需提权执行
  仓库自带 `bin/protoc.exe`）。

## 5. 下一步

- **用户裁决生产启用 fail-closed**（观察影子台账零误阻断 + 探针矩阵）；
- Slice 3：ModeChangeTicket → `bump_policy_revision` 首个生产递增来源 +
  policy_digest 真摘要切换 + 会话级计数器 gate（D3-1）；
- Slice 4：Windows Sandbox backend（D-11）。
