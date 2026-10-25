# S8 权限与审批（surface-approval）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)。
> 代码：`orz/crates/orz-host/src/permission.rs`、`orz/crates/orz-web/assets/app/{acp.js,dialogs.js}`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dk、0dl、0n。

## 1. 状态机

```text
tool → policy 判定（deny 短路）→ yolo? auto-approve
  → 否则 session/request_permission → 客户端弹窗
      → allow / deny / cancelled
      → 300s 无应答 → FailClosed(deny)
```

## 2. 不变量

1. **策略先于 yolo**：policy deny 在 yolo 之前短路，yolo 不能覆盖硬拒。
2. **fail-closed**：无客户端/超时/弹窗被新请求挤掉 ⇒ deny，不默认放行。
3. **强制审批类**：浏览器下载/脚本动作即使 yolo 也要逐实例人工批准。
4. **来源可审计**：`PermitSource{Yolo,User,Policy,FailClosed,Timeout}` 进 journal。
5. **不静默卡死**：静默客户端不得让循环永久停住（300s 上限）。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 唯一客户端断线 | CC 后台会话停在 blocked，需 attach 才能答；Codex 未查证 | 300s 后 deny |
| 并发第二个请求 | 无产品文档 | 新请求把前一个结算为 cancelled（不排队）；333 已在新弹窗显式提示（含本页累计顶掉次数） |
| 多端归属 | CC 独占；Codex 多订阅 | 第二 WS 409，无仲裁面 |
| 决策持久化 | CC「Yes-don't-ask」写 settings；Codex 跨会话持久 | 本地面不持久化（逐次问） |

## 4. 明确不做

- 按程序名持久放行（Codex #22181 作用域过宽的教训）。
- 审批替代写控（写控是灾难兜底，不是审批组件）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| yolo 默认 | 写/命令/网络默认放行 | `permission.rs:6-9` | 接受（须文档明示） |
| 超时 fail-closed | 300s | `permission.rs:311-312` | 接受 |
| 并发请求 | **333 已落**：第二个请求顶掉第一个时，新弹窗顶部提示「上一条已回 cancelled」＋本页累计次数；**334** 已加 permission 未应答跨 attach 重放（`pending_permissions`，上限 32 条） | `dialogs.js`；`acp_pump.rs` | 已闭（真机待 S4） |
| 多端归属 | 不存在 | 0dk | 可延 |
| 持久化 | 无「本会话不再问」 | 无实现 | 可延 |

## 6. 0.9.8 判决

- **必闭**：并发审批请求的可见队列（或明确提示「上一次审批已被取消」）——333 已落提示；334 已加未应答 permission 跨 attach 重放。**剩余边界**：300s fail-closed 超时不变；桥进程重启后未应答队列丢失（新页只能等 agent 到 300s 后 deny）。
- **可延**：决策持久化、多端仲裁。
- **不做**：按程序名持久放行。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 策略先于 yolo | `permission::tests::read_only_policy_denies_non_read_before_manager`（manager 为 allow-all，仍断言 deny＋`source=policy`）；`codex_app::tests::read_only_and_workspace_write_threads_coexist`（端到端同类工具分流） | 浏览器强制审批类与普通 policy deny 的组合顺序仍无单一端到端矩阵 |
| 2 fail-closed | `permission.rs` 300s 单测；334 `unanswered_permission_is_replayed_on_reattach` | 真机断线无测试；桥重启丢队列未测 |
| 3 强制审批类 | smoke §5 | 浏览器下载类无端到端 |
| 4 来源可审计 | PermitSource 断言 | 全枚举无覆盖 |
| 5 不静默卡死 | 300s 超时单测 | — |
