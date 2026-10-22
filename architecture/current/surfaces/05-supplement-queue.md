# S5 中途补充与排队（surface-queue）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；设计稿 [0di](../../../docs/0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18.md)。
> 代码：`orz/crates/orz-host/src/acp_server.rs`（`_orz/queue_append`、`SupplementQueue`）、`orz/crates/orz-web/assets/app/main.js`（`queueSupplement`）。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0di。

## 1. 状态机

```text
running → append(text) → queue[depth] → 下一 LLM 调用边界 drain → 作为 user 补充进入 messages
run 结束仍有残留 → 丢弃并计数（dropped_supplements）→ UI 告知
```

## 2. 不变量

1. **不打断**：补充永不中断当前工具/模型调用。
2. **边界投递**：只在 loop-top（下一次 LLM 调用）drain；不在工具执行中插入。
3. **可见**：用户能看到「已排队（第 N 条）」与投递事件。
4. **有界**：队列有上限；丢弃必须计数并告知（不静默）。
5. **顺序**：多条补充按追加顺序投递。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| run 先结束 | CC 队列消息随 turn 结束发出 | 残留丢弃并计数（`dropped_supplements`） |
| 刷新/断线 | CC 队列在进程内 | 329 后随 child 存活；跨进程不恢复 |
| 补充与撤回交错 | 无对应 | 以 recall_log 过滤，边界未单独验证 |

## 4. 明确不做

- 持久化队列到侧车（0dk 余项，未实现）。
- 优先级/插队/即时转向。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 追加/深度 | 已实现，UI 提示；**333**：回执带 `capacity/remaining/scope`，属性面板显示深度/额度；**334**：切会话/解绑时读数清零，属性面板以服务端 `remaining/scope` 为准 | `main.js`／`dialogs.js` | 已落（真机待 S4） |
| 丢弃告知 | `dropped_supplements` 文案 | `main.js:999` | 接受 |
| 投递可见 | `supplement_delivered` 事件 | `projection.js:78` | 接受 |
| 队列持久化 | 无；进程退出即丢（333 在回执/属性面板明示「仅本会话进程，刷新/换页不保留」） | 0dk 验收⑤ | 可延（已诚实标注） |
| 队列上限 | 有帽（16）；**333 已显示剩余额度**；**334** attached ingress 有界（慢/半开 WS 不再无界增长） | `acp_server.rs` `handle_queue_append`／`acp_pump.rs` | 已落 |

## 6. 0.9.8 判决

- **必闭**：明确「队列随进程存活、不跨进程」文案（与 S3 合并一处；333 已在入队回执/属性面板落地）。
- **已落**：额度显示（333 回执/属性面板）；334 修复批补切会话/解绑清零、以服务端 `remaining/scope` 为准、attached ingress 有界。
- **可延**：持久化。
- **不做**：插队/steering。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 不打断 | `queue_append_enqueues_fifo_with_depth_and_drains` | 工具批内不插入无直接测试 |
| 2 边界投递 | 同上（loop-top drain） | — |
| 3 可见 | smoke §13①② | 深度/丢弃 UI 无钉 |
| 4 有界 | `queue_append_fails_closed_on_empty_and_depth_cap` | 帽值（16/256 KiB）无断言字面 |
| 5 顺序 | `queue_append_enqueues_fifo_with_depth_and_drains` | 与 recall 交错无测试 |
