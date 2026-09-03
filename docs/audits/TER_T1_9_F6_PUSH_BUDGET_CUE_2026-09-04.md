# TER T1.9 F6 push 档实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.9——F6 push 档：剩余评测墙钟跨 <600/300/120s
> 阈值机械注入中性事实（只报剩余/上限/已用轮，不附建议）≤4 次/run +
> `budget_cue_injected` 事件；开关 env/config 默认 off。验收：off 零注入
> （PUSH→PULL 回归）；on 时次数上限与事件可审。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.3（F6 三档）/ §10 S1-6；T0.2 schema/verifier/fixtures（
> budget-cue-injected payload + 每 run ≤4 + remaining < threshold）；
> T1.8（pull 面 + wallclock 读源）同目录审计。

## 1. 目标与验收

- 显式开启（`ORZ_F6_PUSH`）且配置评测墙钟上限时，主车道每轮模型请求前
  检查剩余；首次低于 600/300/120 各自注入一次中性事实并记
  `budget_cue_injected`（payload：remaining_seconds / rounds_used /
  threshold_seconds）。
- 每 run 每档至多一次 → ≤3 次/run（T0.2 verifier 上限 4 兼容；一跳多档
  时逐轮补注入，总次数仍 ≤3）。
- 默认 off：即使配置了墙钟上限也零注入、零事件、零文本（PUSH→PULL
  纪律保持）。
- 注入块注册为机械注入文本（绝不持久化回会话）。

## 2. 代码改动

### 2.1 orz-assurance（事件枚举）

`journal/event.rs`：`EventType` 增 `BudgetCueInjected`（v0.2 轨；serde
snake_case 序列化为 `budget_cue_injected`，与 run-event schema 一致）。

### 2.2 orz-loop

- `controller.rs`：
  - `f6_push_enabled_override()`——env `ORZ_F6_PUSH`（1/on/true/yes；
    缺失 = off）；
  - 字段 `f6_push_enabled` / `f6_push_limit_secs`（构造时取
    `ORZ_MAX_WALLCLOCK`）/ `f6_push_crossed: Mutex<[bool;3]>`（run 起始
    复位，随 `blackboard_read_cursors` 同点）；
  - `maybe_push_f6_budget_cue()`——每轮请求前调用：仅主车道 + 开关开 +
    上限配置时计算 remaining（LIF run-relative，T1.8 同源），跨档则记
    事件（guard 不跨 await 持有）+ push 中性文本。
- `prompt.rs`：
  - `F6_BUDGET_CUE_PREFIX` + `f6_budget_cue_block()`（只报剩余/上限/已用
    轮，无建议）；注册进 `is_injected_block_text`；
  - `f6_push_cue_for_remaining()` 纯函数（600/300/120 逐档一次）。
- `agent_loop.rs`：模型请求构建处（`sync_status_line_message` 后）调
  `maybe_push_f6_budget_cue`。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| prompt.rs | `f6_push_cue_is_neutral_and_crosses_each_threshold_once` | 文本中性（无建议/请）；逐档一次 ≤3；一跳多档逐轮补注入仍 ≤3 |
| host_exec.rs | `f6_push_cue_injects_once_when_enabled_and_crossed` | 开启 + 上限 2s：三个 BudgetCueInjected（threshold 120/300/600 各一），remaining < threshold、rounds_used 存在；模型面收到 `[F6_BUDGET_CUE` |
| host_exec.rs | `f6_push_off_by_default_injects_nothing` | 默认 off（即使配置上限）：零事件、零文本 |

## 4. 验证证据

- `cargo test -p orz-loop f6_push`：3 passed。
- **`cargo test -p orz-loop --lib`：703 passed / 0 failed / 3 ignored**。
- **`cargo test -p orz-assurance --lib`：201 passed / 0 failed**（含
  journal/event 枚举面）。
- `cargo fmt -p orz-loop -p orz-assurance -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。
- 说明：Python 侧 schema/verifier/fixtures（`budget_cue_injected`、≤4
  /run、remaining<threshold）已由 T0.2 定稿并保持；事件链复验归 T1.13。

## 5. 边界声明

- 注入只报中性事实（剩余/上限/已用轮），不附建议（P5 预算面纪律）；
  文本机械前缀注册为注入块，不持久化。
- 阈值判定按「每档一次」：一跳多档在后续模型轮逐档补注入（最多 3 次/
  run）；同轮至多注入一档，避免刷屏。
- push 档读源与 pull 面同源（`ORZ_MAX_WALLCLOCK` / runner 施加值；
  M2 T2.1 墙钟单一化后接官方 agent_timeout_seconds）。
- 检索子代理车道不注入（仅主车道）；wallclock 未配置或开关 off 均零
  注入（PUSH→PULL 回归）。
