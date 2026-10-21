# S4 发送与回合（surface-turn）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-web/assets/app/{main.js,acp.js}`、`orz/crates/orz-host/src/acp_server.rs`（`session/prompt`、`session/cancel`）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0di、0dk。

## 1. 状态机

```text
idle → submitting(prompt) → running → (queued supplement)* → terminal
running + 用户再发 → queueSupplement（不打断）
running + 停止 → session/cancel(notification) → 等 cancelled/终态
terminal: run_finished | run_failed | run_cancelled | run_invalidated
```

## 2. 不变量

1. **单飞**：同一会话同一时刻只有一个 `session/prompt` 在途。
2. **不打断**：运行中提交＝排队，在下一次 LLM 调用边界投递（与 CC queue 同形）。
3. **取消可收敛**：`session/cancel` 后必须落到 `cancelled` 终态；已执行工具副作用不回滚（业界同侧）。
4. **终态必达**：任何失败路径都要写终态事件，前端不得停在 running。
5. **状态重同步**：刷新/外部 busy 时，前端以 journal 终态复位，不靠本地猜测。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 取消在工具中 | CC Esc 保留已完成工作；Gemini 未查证 | 取消在途工具，副作用保留 |
| 取消后队列 | CC Esc 后队列立刻发出 | 本地面队列保留，等下一次运行 |
| 模型/网络错误 | CC 自动重试、partial 标 incomplete | `run_failed` 显示 `[错误]`；无自动重试 |
| 重复发送 | CC #2.1.296 曾重复执行 | 单飞＋submit 幂等由 ACP 保证 |

## 4. 明确不做

- mid-turn steering / Send-Now / 打断后立即转向（用户 295 裁决明确不做）。
- 自动重试（由模型自行决定重发）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 排队 | 运行中提交入队并提示深度 | `main.js:204-214` | 接受 |
| 停止 | `stopRun` → `session/cancel` | `main.js:350-363` | 接受 |
| 取消终态 | 依赖 agent 回 `cancelled`；无超时兜底 | `acp.js:152` | 应修 |
| 错误面 | 仅 `[错误]` 文本，无下一步 | `projection.js:294` | 应修 |
| partial output | 已产生内容保留 | 事件流 | 接受 |

## 6. 0.9.8 判决

- **必闭**：取消后 30s 未收敛的兜底（前端复位＋提示）；错误面给可执行下一步（重发/查看 journal）。
- **可延**：自动重试。
- **不做**：steering。

> 错误文案四要素（发生了什么/为什么/下一步/证据）与六类分法见 [`0dk S1 §9`](../../../docs/SESSION_LIFECYCLE_OWNERSHIP_AND_CLOSE_DESIGN_2026-10-21.md)。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 单飞 | `prompt_bootstrap_failure_releases_cancel_token` | 并发 prompt 拒绝无测试 |
| 2 不打断 | smoke §14.5 | 队列边界无端到端 |
| 3 取消可收敛 | `session_prompt_default_yolo_auto_approves_with_dead_gateway` | 取消超时无测试 |
| 4 终态必达 | journal 终态断言 | 前端兜底无测试 |
| 5 状态重同步 | `serverBusy` 逻辑（smoke） | 真实外部 busy 未跑 |
