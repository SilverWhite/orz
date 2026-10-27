# 369 批：压缩摘要控制面工具化（`context_manage(mode=compress, summary=…)`）

> 日期：2026-10-27
> 用户令：「把压缩块从 assistant 中挪出去……转为紧凑提示，和工具提示一起……教学部分不要进 prompt，改一下黑板中的说明书和工具栏注释就行」「summary 就不给模板了，省得有些情况中可能出现局限」。
> 上游：[`368 纯摘要控制轮修复`](368_SEMANTIC_SUMMARY_CONTROL_TURN_2026-10-27.md) / `context_scale.rs` v8 压缩 / `context_manage`（0cz S2 单工具带 mode）。
> 审查：[`370 M1-S1 与 369 审查`](370_M1_S1_AND_369_REVIEW_2026-10-27.md)（P1 窗口顺序已修；P2 prompt/whitelist 待裁决）。
> 性质：**最小控制面迁移**；压缩窗口/分块/marker/档案/journal 事件/预算全部不变。工作树未提交。

## §0 结论

压缩摘要从 **assistant 正文** 迁到 **`context_manage` 工具参数**：

```json
{"mode":"compress","summary":"<自由文本工作现场摘要>","whitelist":["…"]}
```

- 工具调用是类型化控制面，天然不参与终答判定；不会再有“摘要块被当终答”的歧义。
- `summary` **不设字段模板/段模板**：工具调用本身已声明“这是摘要”，无需靠“至少两个段标签”防误识别；只校验非空与长度（≤8192 字符）。
- 可含 `压缩块: 1-4` 行指定范围；不写则沿用现有缺省（模型自选＝最旧闭合块；强制窗＝全部已闭合块）。
- 不提供 `summary` ⇒ 保留现有“开压缩窗口 ≤3 轮”的兼容路径。
- `[SEMANTIC_SUMMARY]` 正文路径保留为 368 的兼容 fallback，不删。

## §1 改动点

### 1.1 `context_manage` schema / handler

- `controller.rs`：`context_manage` 参数新增可选 `summary`（string，maxLength 8192）；描述改为“有 summary 一次性压缩，无 summary 走窗口；不要写成 assistant 正文”。
- `host_exec/tool_run.rs`：`mode=compress` 且 `summary` 非空时：
  - 校验非空/长度（`context_scale::normalize_tool_summary`）；
  - 统一包成内部 `[SEMANTIC_SUMMARY]` 块（复用既有 marker/区间解析）；
  - 写入 controller 的 `compression_summary` 一次性请求位；
  - 返回紧凑回执：`压缩摘要已接收（N 字），已排队；继续当前任务。` + 滑块读数；
  - 不置窗口请求位、不开多轮窗口。
- 无可压缩分块 ⇒ 返回 `NothingToCompress`，不排队；非法摘要 ⇒ `ToolCompleted exit 1`，不排队、不压缩。

### 1.2 loop 消费

- `agent_loop.rs` loop-top：`take_compression_summary()` 取走摘要，注入既有 `pending_semantic`（`CompressionWindowKind::ModelRequested`），后续仍走 `compress_blocks_now` 单一路径。
- 不需要新事件、新字段、新工具；`context_compressed{mode=model_summary, reason=model_selected}` 不变。

### 1.3 教学位置（不进 prompt）

- `board_guide_body()`：压缩摘要用 `context_manage` 的 `mode=compress+summary` 提交，自由文本、无固定模板，不写成 assistant 正文。
- `context_manage` ToolDef 描述 + `summary` 参数注释：一行说明“有 summary 一次性压缩；无 summary 走窗口；不要写成 assistant 正文”。
- `context_compress_response()`：把“落点＝回复文本”改为“首选工具参数；回复文本块仅兼容”。
- **未改**系统 prompt / resident 提醒 / 常驻教学文案。

## §2 测试

- `context_scale::tests::tool_summary_accepts_free_text_without_section_template`：自由文本（无固定段）可接受；已带标记不重复包裹；空/超长拒绝。
- `agent_loop::tests::context_manage_summary_tool_call_compresses_without_assistant_text`：两工具轮 → `context_manage(mode=compress, summary=自由文本)` → 真实终答；断言 `context_compressed` 恰一次、摘要不出现在 `model_output.text`、`ToolCompleted{summary_queued:true}`。
- 既有 368 纯摘要 fallback 测试继续覆盖文本路径。
- 读数：`cargo test -p orz-loop --lib` **936 passed / 0 failed / 3 ignored**。

## §3 边界

- 不改 host / assurance / Python 工具表；`context_manage` 仍是既有单工具。
- 不改 UI；TUI/Web 已有 `context_compressed` 紧凑系统消息，工具参数中的摘要不会以 assistant 正文进入聊天流。
- 未提交、未重建载体、未换装；当前在役进程仍是旧逻辑。
- 后续可在观测到文本 fallback 归零后删除 `[SEMANTIC_SUMMARY]` 正文路径；本批不删。

## §4 入口

- 代码：`orz/crates/orz-loop/src/controller.rs`、`host_exec/tool_run.rs`、`agent_loop.rs`、`context_scale.rs`。
- 前序：[`368 批`](368_SEMANTIC_SUMMARY_CONTROL_TURN_2026-10-27.md)。
