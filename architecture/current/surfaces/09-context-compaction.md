# S9 上下文与压缩（surface-context）

> 状态：`current-design`（2026-10-21 330 批建立）。设计权威 [ADR-0010 §14.69/§14.72](../../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)；设计稿 [滑块 v8](../../../docs/CONTEXT_SLIDER_V8_DESIGN_2026-09-16.md)。
> 代码：`orz/crates/orz-loop/src/{model_face.rs,agent_loop.rs}`、`orz/crates/orz-web/assets/app/projection.js`。
> 审计：[330 批档](../../../docs/audits/330_SURFACE_CONTRACT_BASELINE_2026-10-21.md)；归属项：0cz、0da、0ah、0de。

## 1. 状态机

```text
水位: 正常 → 500K 软提醒（会话级一次）→ 900K 必须压缩 → 950K 最后防线
动作: 机械压缩（结构内容移出视野） | 模型压缩（context_manage mode=compress）
      clear/clear_all（移出视野，不销毁）
```

## 2. 不变量

1. **压缩不销毁**：原文落盘（`.gsa/compaction/blocks/`＋journal），可按块回放。
2. **滑块不压**：驻留带逐字保留，模型面只收窄其外分块。
3. **clear ≠ 删除**：移出模型视野，本地全量保留（0cz⑤）。
4. **水位会话级**：500K 提醒每会话一次，跨 prompt 不重发。
5. **估算可见**：压缩事件带 `trigger_tokens`/`estimated_tokens_after`。

## 3. 失败模式

| 失败 | 业界事实 | 本地面 |
|---|---|---|
| 压缩后失忆 | CC 摘要替换、原消息仍在本地；Gemini 有 checkpoint | 本地全量留档＋可回放 |
| 无可压内容 | — | 空转窗有护栏（0de） |
| 估算为 0 | — | 0de F3 已修 |
| 用户想手动压 | CC `/compact`、Codex `/compact` | 本地面只有模型工具，**Web 无入口** |

## 4. 明确不做

- 用户手动压缩按钮（当前设计：压缩归模型/机械层；若要改属设计变更）。
- Web 端上下文滑块（现状：只有一行压缩摘要）。

## 5. 现状与差距

| 项 | 现状 | 证据 | 等级 |
|---|---|---|---|
| 压缩事件投影 | Web 一行「[上下文压缩] …（估 ~NK）」 | `projection.js:237-243` | 接受 |
| 水位可见 | Web 无占用率/水位；TUI 有事件 | `model_face.rs:2259` | 应修 |
| 手动压缩 | Web 无 `/compress`；模型可 `context_manage` | `agent_loop.rs:9820` | 接受（设计） |
| 压缩失败提示 | 有 `context_recovery_truncated` 事件 | `projection.js:248` | 接受 |
| 0cz/0da S4 | 未真机 | BACKLOG | 应修（验证） |

## 6. 0.9.8 判决

- **必闭**：Web 端至少显示「上下文占用/最近一次压缩」的可读读数（不要求滑块）。
- **可延**：手动压缩入口（需设计裁决）。
- **不做**：默认用户可操作滑块。

## 7. 测试映射

| 不变量 | 现有测试 | 缺口 |
|---|---|---|
| 1 压缩不销毁 | `model_face_hides_compressed_blocks_and_keeps_the_slider_verbatim` | 回放路径无端到端 |
| 2 滑块不压 | 同上 | — |
| 3 clear≠删除 | `cleared_face_honors_post_boundary_compression_and_estimate_falls` | — |
| 4 水位会话级 | `slider_readout_matches_block_and_marker_state` | 跨 prompt 不重发无钉 |
| 5 估算可见 | smoke §10 | Web 读数缺失 |
