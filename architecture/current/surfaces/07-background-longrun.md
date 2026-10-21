# S7 长任务与后台（surface-background）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010 §14.87](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；设计稿 [0dh](../../../docs/LONGRUNNING_PROCESS_EXPLICIT_CHANNEL_DESIGN_2026-10-18.md)。
> 代码：`orz/crates/orz-host/src/tools.rs`（工具名单）、`process_tree.rs`、`orz/crates/codegen/xai-tty-utils/src/resource_job.rs`（`lifetime="session"`）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dh、0dk。

## 1. 状态机

```text
tool call(is_background) → spawn → registered(task_id, lifetime="session")
  → running ⇄ idle(300s no output & no CPU) → completed / killed / 30h backstop
run 结束 → Refuse(SessionOwned)：不收割，跨 run 存活
session 结束 → 随会话域终止
```

## 2. 不变量

1. **跨 run 不收割**：`lifetime="session"` 的登记行在 run 尾 finalize 被拒绝收割（审计可见不杀）。
2. **同台**：`get_task_output` 与 `kill_task` 必与 `is_background` 同面（`requires_expr`）。
3. **绝对兜底 30h**（`GROK_BG_MAX_RUNTIME_MS` 可调，下限 1h），idle-kill 300s 保留。
4. **增量读出**：`get_task_output` 返回 `new_bytes` 游标，长日志不整读。
5. **不静默**：kill/完成/兜底都要有终态记录。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 跨轮存活 | Codex #10767 曾裁掉；CC 会话内跨轮 | 本地按用户裁决跨 run 存活 |
| 会话结束 | CC 后台 bash 随会话结束 | 随会话域终止；死 session 登记行留盘（0dh 余项） |
| 进程树回收 | Codex 有专门 process tree 处理 | per-call Job + actor 域；298 已修 |
| 刷新/断线 | 无产品承诺 | 329 后随 child 存活；窗口外不恢复 |

## 4. 明确不做

- 后台完成主动唤醒模型（业界共识是「下次拉取时见」）。
- 跨进程后台任务恢复。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 工具面 | `is_background`/`get_task_output`/`kill_task` 同台 | `tools.rs:429-430` | 接受 |
| lifetime | `session` 登记＋finalize 豁除 | `resource_job.rs:379` | 接受 |
| 30h 兜底 | 已落码，S4 未真机 | 0dh 档 | 应修（验证） |
| 死 session 清理 | 登记行留盘 | 0dh 余项 | 可延 |
| Web 可见面 | 仅 `tool_running` 行＋`backgroundTasks` 计数；**333**：属性面板明确「仅模型可见；本页不提供任务表，刷新/关闭不恢复」 | `projection.js:164`／`dialogs.js` | 已落（实时面板仍未做） |

## 6. 0.9.8 判决

- **必闭**：Web 端后台任务可见面（任务列表、状态、增量输出入口或明确「仅模型可见」——333 选了「明确仅模型可见」分支）；窗口外不恢复的显式提示（333 已落 attach 前置帧）。
- **可延**：死 session 清理。
- **不做**：主动唤醒。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 跨 run 不收割 | `session_terminal_backend_survives_host_rebuild_and_close_teardown` | 跨 run 真机未验（0dh S4） |
| 2 同台 | `explicit_background_surface_visible_in_production_toolset` | — |
| 3 30h 兜底 | 常量断言（298） | 真机超时未跑 |
| 4 增量读出 | `new_bytes` 三态单测 | 长日志真机未验 |
| 5 不静默 | `kill_task_killed`、`kill_task_not_found_returns_typed_output` | 兜底终态无测试 |
