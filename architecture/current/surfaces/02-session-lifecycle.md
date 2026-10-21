# S2 会话生命周期（surface-session）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；本页只写**用户可见面**的机械契约，不是实现说明。
> 代码：`orz/crates/orz-host/src/acp_server.rs`（new/load/prompt/cancel/lease）、`orz/crates/orz-host/src/stdio.rs`、`orz/crates/orz-web/src/{server.rs,acp_pump.rs}`、`orz/crates/orz-web/assets/app/main.js`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0dk（余项）、0cy（收卷口径）。

## 1. 状态机

| 状态 | 进入 | 退出 | 持久化 |
|---|---|---|---|
| `none` | 无侧车 | `session/new` | — |
| `created` | `session/new` 返回 sessionId；写入 `.gsa/sessions/<s8>` 侧车 | `prompt` / `load` / `close` / `delete` | 侧车 |
| `active` | `session/prompt` 在途 | 终态事件（`run_finished`/`run_failed`/`run_cancelled`/`run_invalidated`） | 侧车＋journal |
| `detached` | 客户端断开（WS detach） | 重连 attach / 保活过期 reaped | 桥内存 |
| `loaded` | `session/load`（全量回放后） | `prompt` / `close` | 侧车 |
| `archived` | `orz archive` / 归档键 | `unarchive` | `.gsa/archives/*.gz` |
| `deleted` | `orz delete-session` / 删除键 | — | 侧车与归档删除 |

## 2. 不变量（违反即 bug）

1. **单一写者**：同一 `<s8>` 同一时刻只有一个进程可写；第二进程由 `.gsa/sessions/<s8>.lease` OS 锁 fail-closed。
2. **id 同源**：web/ACP/journal 三处会话标识同源（`sessionId` 前 8 hex ＝ `RUN-<s8>-<n>` 前缀）。
3. **load 不收卷**：`session/load` 与交互式 `session/new` 均不触发 0cy 启动侧 sweep（329 落码）。
4. **回放边界**：`session/load` 只回放 user/assistant 消息；工具/系统消息不回放——客户端必须知道历史不完整。
5. **close 先行**：关闭会话必须先取消在途 run 再释放资源（ACP v1 `session/close` 语义；本地面尚未实现）。
6. **删除不留尾**：删除会话后，下一条 prompt 不得把侧车写回（未达成，登记在 0dk）。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 运行中崩溃 | CC 把未完成 tool 标 cut off、不重跑；OpenHands 用 `get_unmatched_actions()` 兜底 | journal 留 `tool_started` 无 `tool_completed`；327 已修配对读取 |
| 归档后继续聊 | CC `claude rm` 只移出列表、transcript 保留 | orz 回档成功即删包（既定语义），回档后可继续 |
| 删除已载入会话 | opencode 需先停会话；Codex 运行中 delete 被拒 | 删除后绑定未清、下一条 prompt 复活（0dk 余项） |
| 第二进程打开 | CC 普通 resume 不锁、后台会话独占；无一家有跨进程租约 | 本地已加 OS lease（领先业界） |
| 协议缺 close | ACP v1 有可选 `session/close`（先 cancel 再释放） | 本地按 0.10.4 实现，无 close、无 list |

## 4. 明确不做

- 跨机共享与云同步（Codex #52050 仍是 FR）。
- 多写者并发（单写者＋lease 是本地裁决）。
- TUI 恢复可写会话（TUI 只读回看，319 维持）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| close 状态机 | 无 `session/close`；web 无「关闭会话」入口，关页面＝断开 WS | `acp_server.rs:4931` | 阻塞 |
| `sessions` 有界 | map 无上限，长期运行只增不减 | `acp_server.rs` sessions map | 应修 |
| 删除后复活 | 删除绑定未清，prompt 写回侧车 | 318 §3 P2、0dk 验收⑦ | 应修 |
| load 回放工具面 | 只回放 user/assistant | `acp_server.rs:3822` 起 | 接受（须 UI 标注） |
| ACP v1 对齐 | 停在 0.10.4；缺 resume/close/list | vendored `xai-acp-lib` 0.1.0 | 阻塞 |
| TUI 恢复 | 只读，never resumes | `orz-tui/src/input.rs:185` | 接受 |

## 6. 0.9.8 判决

- **必闭**：close 状态机（含 cancel 先行）、ACP v1 `resume/close/list` 对齐、删除后不复活、`sessions` 有界。
- **可延**：TUI 可写恢复。
- **不做**：跨机、多写者。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 单一写者 | `session_lease_is_exclusive_and_released_on_drop` | 跨进程真机未验 |
| 3 load 不收卷 | `session/load must not run the 0cy sweep`、`interactive_session_new_does_not_sweep_sibling_sessions` | — |
| 4 回放边界 | `session/load history replay`（host 单测） | 工具面口径无测试 |
| 5 close 先行 | 无 | 整条缺 |
| 6 删除不留尾 | `close_session_releases_metadata` | 复活路径无测试 |
