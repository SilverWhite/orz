# S3 连接与多端（surface-connection）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-web/src/acp_pump.rs`（`AgentBridge`）、`server.rs`（`/ws/acp`）、`assets/app/acp.js`、`main.js`（`onBridgeAttached`）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dk、329。

## 1. 状态机

```text
WS:      closed → connecting → attached ⇄ detached(grace) → reaped
agent:   spawning → running → exited
```

- `attach`：按 workspace key 取槽；槽已 attach ⇒ 409；否则新建/复用 child。
- `detach`：WS 断开不杀 child；保活窗口默认 1800s（`ORZ_WEB_KEEPALIVE_SECS`，0＝旧行为，硬顶 6h）。
- `reaped`：grace 到期或 child 退出，槽回收。
- 重连：新 WS 先收 `_orz/bridge_attached`（`session_id/reused/prompt_in_flight/grace_secs/cwd`），再补发 8 MiB 有界 backlog。

## 2. 不变量

1. **独占**：同一 workspace 同一时刻至多一个 attach；第二个返回 409，不静默接管。
2. **前置帧**：attach 后第一帧必是 `_orz/bridge_attached`，且先于任何 backlog。
3. **不杀在跑**：保活窗口内刷新/断线不重启 child、不中断在途 turn。
4. **有界**：backlog 上限 8 MiB；溢出丢最旧并置 `truncated` 标志，客户端可见。
5. **诚实边界**：窗口外/child 退出后，排队补充与后台任务**不恢复**，必须显式告知。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 半开连接 | CC 由 supervisor 托管；Codex WS 队列满返回 `-32001` 要求退避 | 无心跳，依赖 OS FIN/RST |
| 第二客户端 | CC 后台会话独占；Codex app-server 多订阅；opencode 多端 | 409＋`serverBusy` 重同步 |
| 桥进程崩溃 | CC supervisor 重启；turn 中断 | reaper 回收槽，turn 丢失 |
| 长断线 | CC 关机 48h 内 failed 可续 | 窗口外不恢复（登记 0dk） |

## 4. 明确不做

- 多客户端并发写、跨机附着、WS 消息级 gap/去重游标（业界也无文档化 replay cursor）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 保活 | 1800s，可配；329 落码未进装机位 | `acp_pump.rs:108-124` | 阻塞（需发行） |
| 心跳 | 无；半开连接只能等 OS | 无实现 | 应修或显式接受 |
| 第二客户端文案 | 409 后前端 busy 重同步，未说明「谁在占用」 | `main.js:224` 起 | 应修 |
| 附着协议 | 私有 `_orz/bridge_attached`，非 ACP `session/resume` | `acp.js:92` | 应修（并入 ACP v1 对齐） |
| 跨进程恢复 | 未实现 | 0dk 验收⑤ | 可延 |

## 6. 0.9.8 判决

- **必闭**：把「窗口外不恢复」做成显式事件/文案；第二客户端占用者提示；`_orz/bridge_attached` 与 ACP `resume` 的关系写清（短期兼容层）。
- **可延**：心跳、跨进程恢复。
- **不做**：多写者。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 独占 | `attach_is_exclusive_and_detached_frames_replay_on_reattach` | 409 文案无测试 |
| 2 前置帧 | 同上（断言前置帧先于 backlog） | — |
| 3 不杀在跑 | `attach_is_exclusive_and_detached_frames_replay_on_reattach` | 真实 WS 端到端未跑（329 档自述） |
| 4 有界 | `detached_backlog_is_bounded_and_flags_truncation`、`oversized_single_frame_is_dropped_not_buffered` | — |
| 5 诚实边界 | `keepalive_grace_parsing_is_bounded` | 窗口外丢失无测试 |
