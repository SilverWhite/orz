# S6 撤回与编辑重发（surface-recall）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；设计稿 [0di](../../../docs/0DI_RECALL_SUPPLEMENT_DESIGN_2026-10-18.md)。
> 代码：`orz/crates/orz-host/src/acp_server.rs`（`_orz/recall`、`recall_log`）、`orz/crates/orz-web/src/runs.rs`（`recall_log` 投影）、`orz/crates/orz-web/assets/app/{main.js,state.js}`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0di（S4）、329。

## 1. 状态机

```text
turn N: user card → assistant/tool items → marker → (recall target)
rewind(target): items 截断 → markers 按锚重建 → 原 prompt 回输入框 → 用户编辑重发
```

- 撤回目标＝某条 user 卡之前；`recall_log` 记录每次撤回的 ordinal 区间，供重放过滤。
- 重发＝普通 `prompt`，不新建会话。

## 2. 不变量

1. **只回会话面**：不恢复工作区文件；文件面 git 兜底（用户 296 裁决，与 Codex `revert` 同侧）。
2. **同源同刻**：前端 `items` 截断与后端上下文 `messages` 截断是同一动作的两面，不得只做一边。
3. **从锚重建**：撤回后 marker/run 列表由剩余条目锚重建，不留孤儿卡片或空洞。
4. **重放过滤**：`openConversation` 重放时按 `recall_log` 丢弃被撤 tail，多次级联撤回同样成立。
5. **不误删**：目标之前的内容逐字保留；撤回自身不产生新的 user 卡。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 文件副作用 | CC 只回滚 Edit/Write，Bash/subagent 不回滚；Gemini `/rewind` 不回滚手改与 shell | 不回滚，git 兜底（同侧） |
| 快照过期 | CC 30 天后 `No files were restored` | 无文件快照，不适用 |
| 大小写误删 | CC #102206 在 macOS 静默删同名文件 | 无文件恢复，不适用 |
| 事件量过大 | 无对应 | >200k 事件时禁用 recall 过滤（无提示） |
| 序号错位 | — | ordinal 与 journal 事件空间可能 off-by-one（304 登记） |

## 4. 明确不做

- 文件级 checkpoint、`code/conversation/both` 三选、跨会话撤回。
- turn 中插入消息的撤回（CC 同样无 checkpoint）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 投影截断 | 329 落码（本地截断＋`rebuildMarkers`） | `main.js` rewindRecall、`state.js` rebuildMarkers | 阻塞（需发行） |
| 重放过滤 | 按 `recall_log` 过滤；>200k 事件关闭 | `main.js:filterEventsByRecall` | 应修（降级需提示） |
| ordinal 复核 | 304 登记 off-by-one 残留 | 304 档 | 应修 |
| 文件面 | 不做，git 兜底 | 296 裁决 | 接受 |
| UI 文案 | 未说明「文件不回退」 | 待核 | 应修 |

## 6. 0.9.8 判决

- **必闭**：降级提示（事件量超限/撤回不可用）；UI 明示「只回会话，不回文件」；ordinal 复核结论落档。
- **可延**：多次撤回的极端组合。
- **不做**：文件级回滚。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 只回会话面 | `recall_truncates_persists_sidecar_and_returns_recall_text` | UI 文案无测试 |
| 2 同源同刻 | `recall_preview_lists_user_ordinals_from_sidecar_truth` | 前后端一致性无端到端 |
| 3 从锚重建 | `recall_log_projects_sidecar_watermark_and_rejects_bad_ids` | marker 重建无单测（329 仅 smoke） |
| 4 重放过滤 | `recall_log_projects…`＋smoke §15 | >200k 降级无测试 |
| 5 不误删 | `recall_persist_failure_keeps_conversation_and_queue` | ordinal off-by-one 无保护 |
