# 359 批：终态声明教学面残留清理（353/355 补批）

> 日期：2026-10-26
> 用户令：「耶？怎么还在 run_end，修摩擦和 bug 的时候有没清干净的地方吗？」→「请进行吧，先把这个问题处理掉」。
> 基线：父仓 HEAD `08b448458e62`（0dq 闭合批）＋ orz `95faff3a`＋未提交改动（358 scout 7 文件＋本批）。
> 计数：65 不变（缺陷补批，不新增项；358 立项的账实同步仍待其收口，见 §4）。

## §0 结论

**353/355 只关了强制面，没关教学面。** 交互式缺省（`ORZ_TERMINAL_DECLARATION` unset）下，终答文本已无条件交付、零 `[TERMINAL_NOTICE]`，但模型仍从三处被告知“结束要输出 `[RUN_END]`”；另有一处语义缺口——plan 的 `deliver` 末步唯一完成源仍挂在声明上。本批四处全清。

## §1 残留清单（修前）

| # | 位置 | 残留 |
|---|---|---|
| 1 | `orz-loop/src/controller.rs` `self_check` 工具描述 | 无条件 `…then end with [RUN_END]` |
| 2 | `orz-loop/src/prompt.rs` `SELF_CHECK_BLOCK` | 「然后按 §4.1 输出 [RUN_END]」 |
| 3 | `orz-loop/src/prompt.rs` `SELF_CHECK_SUPPLEMENT` | 「然后按需继续或输出 [RUN_END]」；`orz-tui/src/runner.rs` 无条件投递 |
| 4 | `agent_loop.rs` 未声明分支／`planning.rs`／`controller.rs` plan 文案 | `deliver` 只在 `intent=conclude && reason=completed` 的 `[RUN_END]` 时 `mark_deliver_done`；未声明路径直接 `break` |

注释级两处：`orz-web/src/delivery.rs` 头注、`orz-loop/src/model_face.rs` 尾行注释（“每轮尾随”未标 required 车道）。

## §2 落码

- `prompt.rs`：新增 `SELF_CHECK_BLOCK_NEUTRAL`／`SELF_CHECK_SUPPLEMENT_NEUTRAL`；`self_check_block()`／`self_check_supplement()` 按 `terminal_declaration_required()` 选择；注入块识别两处纳入 NEUTRAL（写回/`/rewind` 过滤不把新块误判为用户文本）。
- `controller.rs`：新增 `SELF_CHECK_TOOL_DESC`／`_REQUIRED` 两常量，`self_check` 工具描述按 env 选择；`plan_write` 描述补“或实质文本轮（声明关闭时）”。
- `host_exec/tool_run.rs`：self_check 返回块改用 `self_check_block()`。
- `orz-tui/src/runner.rs`：工具栏补充消息改用 `self_check_supplement()`。
- `agent_loop.rs`：未声明分支收尾前，`terminal_source == "text_without_terminal"`（`run_has_substance` 为真）时 `mark_deliver_done`；`[RUN_END]` 声明路径不变。
- `planning.rs`：plan 校验错误文案补“或实质文本轮”。
- `delivery.rs`／`model_face.rs`：注释勘误。

## §3 验证

- `cargo check -p orz-loop -p orz-tui -p orz-bin`：exit 0（17.49 s）。
- `cargo test -p orz-loop --lib`：**914 passed / 0 failed / 3 ignored**（原 911＋3 钉）。
- 新增钉：
  - `prompt::tests::self_check_texts_follow_terminal_declaration`（required 含／缺省不含 `[RUN_END]`）；
  - `controller::tests::self_check_tool_description_follows_terminal_declaration`；
  - `controller::tests::substantive_text_turn_completes_deliver_without_declaration`（无声明实质文本轮完成 deliver）。
- 既有钉扩展：`self_check_block_carries_shared_questions` 同文检查覆盖 NEUTRAL；`injected_block_registry_covers_all_generated_forms` 增 NEUTRAL 条目。
- `cargo fmt -p orz-loop -- --check`：本批改动面净；仅剩 358 未提交面的两处（`agent_loop.rs:3259`、`retrieval/projection.rs:305`），随 358 收口处理。

## §4 边界

- 未提交、未推送、未重建；358 scout 改动仍在同一工作树。
- 交互式缺省语义不变：文本轮无条件交付；required 车道（benchmark／headless／外部 ACP）契约照旧。
- `SELF_CHECK_BLOCK`／`SELF_CHECK_SUPPLEMENT` 常量保留为 required 版与历史识别，不删。
- 358 立项尚未同步进 BACKLOG/TODO/索引（计数 65 对 358 设计稿自记 66），随 358 收口一并核对。
