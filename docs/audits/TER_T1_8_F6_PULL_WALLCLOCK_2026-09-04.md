# TER T1.8 F6 pull 面 wallclock 实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.8——F6 预算可见性 pull 档：
> `blackboard_read section=session` 补 wallclock 面（elapsed / limit /
> remaining）；`blackboard_read` 返回含时间轴。验收 = 渲染单测 + 越权
> 边界（沿用 session 面既有 epoch/receipt_id 拒绝）。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.3（F6 三档）/ §10 S1-6；T0.2 契约（黑板分区渲染与越权边界）/
> 既有 PUSH→PULL 先例（CONTEXT_SCAFFOLDING_PULL_REDESIGN，session 面）。

## 1. 目标与验收

- `section=session` pull 面补 run 级墙钟：`WALLCLOCK_ELAPSED` 恒渲染；
  配置了评测墙钟上限时渲染 `WALLCLOCK_LIMIT` + `WALLCLOCK_REMAINING`；
  未施加时渲染 `WALLCLOCK_LIMIT: none`（不虚构 remaining）。
- elapsed 数据源 = LIF run 相对时间轴（读取只读、无 side-effect 锚定）；
  limit 数据源 = `ORZ_MAX_WALLCLOCK`（orz-bin `--max-wallclock` 施加；
  评测侧 runner/sandbox 值在 M2 T2.1 墙钟单一化后接同一读源）。
- 越权边界：epoch / receipt_id 与 session 组合保持既有显式报错（O4
  exit_code 1 + error）；本步不新增分区/存储副作用。

## 2. 代码改动

### 2.1 orz-assurance（LIF 只读访问器）

`lif/mod.rs`：`LifEngine::run_origin_secs()`——run 时间轴原点
（`Option<f64>`）；供读取侧换算 elapsed，且不触发 `ensure_run_origin`
 类写副作用。

### 2.2 orz-loop

- `controller.rs`：
  - `parse_main_wallclock_limit_secs()` / `main_wallclock_limit_secs_override()`
    ——env `ORZ_MAX_WALLCLOCK`（秒；0/缺失/非法 = 无上限）；
  - `run_elapsed_wallclock_secs()`——读 LIF origin 换算 run 相对 elapsed
    （未锚定 = 0）；
  - `render_session_section()` 传 `Some((elapsed, limit))` 给扩展渲染入口。
- `prompt.rs`：新增 `session_face_block_with_wallclock()`——`wallclock`
  `Some((elapsed, limit))` 时追加三行（elapsed 恒在；limit Some →
  LIMIT+REMAINING；limit None → LIMIT none）；原
  `session_face_block()` 作为无墙钟包装保持逐字节旧输出。
- `blackboard.rs`：serves-session 回达用例断言 wallclock 行存在。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| prompt.rs | `session_face_wallclock_renders_elapsed_limit_remaining` | elapsed/limit/remaining 渲染；未施加墙钟不虚构 remaining |
| prompt.rs | `session_face_reports_unlimited_when_no_cap`（既有，经 wrapper） | 旧 wrapper 输出不变（T1.7 语义无回归） |
| blackboard.rs | `blackboard_read_serves_session_section`（增强） | 真实工具链回达含 `WALLCLOCK_ELAPSED:` 与 `WALLCLOCK_LIMIT: none` |
| blackboard.rs | `blackboard_read_session_combination_errors_are_explicit`（既有） | epoch/receipt_id 越权边界不回归 |

## 4. 验证证据

- `cargo test -p orz-loop session_face`：2 passed。
- `cargo test -p orz-loop blackboard_read_serves_session`：1 passed。
- **`cargo test -p orz-loop --lib`：700 passed / 0 failed / 3 ignored**
  （全量回归，含 T1.7 unlimited 面与既有 session 越权用例）。
- `cargo fmt -p orz-loop -p orz-assurance -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。

## 5. 边界声明

- pull 面默认 off 语义保持：wallclock 只出现在被读取的 session 分区内，
  不逐轮注入（PUSH→PULL 纪律）；push 档（<600/300/120s 注入 +
  `budget_cue_injected`）归 T1.9。
- 评测墙钟单一化（runner/sandbox `--timeout` = 官方
  agent_timeout_seconds）归 M2 T2.1；本步的 limit 读源为 env 逃生阀，
  T2.1 会把 runner 施加值接到同一渲染入口（结构已就位）。
- elapsed 精度为秒（LIF run-relative 轴，与 temporal/failure_agg 同
  刻度）；读取不锚定 run origin（无写副作用）。
