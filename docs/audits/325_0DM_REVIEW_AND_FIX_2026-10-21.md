# 325 批：0dm S1 全面审查修复（v0.2，2026-10-21）

> 状态：`partial`（S1 v0.2 达成；S2 交付状态面板／S3 0.9.6 重建／S4 真机待续）。
> 关系：修订 [`0dm 设计稿`](../TERMINAL_DECLARATION_AND_SELF_CHECK_PULL_DESIGN_2026-10-21.md)（v0.1 → v0.2，§11 记录）；
> 承接 324 批立项＋S1 落码（orz `19638ee2`）。

## §1 审查方式

三路并行只读审查（设计合理性／实现正确性／设计-实现符合性）＋主会话亲验：

- 逐文件读 `19638ee2` 的全部 17 文件 diff 与设计稿、ADR §14.35 O1/§14.76；
- 对 `run_finished` / `gate_decision` 新 payload 用 Python `jsonschema` 实跑，确认均为 `INVALID`；
- 用 `git grep` 核对 `terminal_source` / `terminal_nudged` / `self_check_called` 只存在于 orz 源码与 docs，schema/fixture/Python/Rust judge 全未同步；
- 复核 ACP 黑板恢复路径（`acp_server.rs:3785/3858`），确认 `run_has_substance` 读的是会话级 edits/plan。

## §2 已修（P0/P1）

### 2.1 合约四同步

- `runtime/run-finished-event-payload-v0.1.schema.json` 增 `terminal_source`（7 值闭集）、`terminal_nudged`、`terminal_nudge_count`、`self_check_called`；
- `runtime/gate-decision-event-payload-v0.1.schema.json` 的 `decision` 增 `nudge`；
- `runtime/fixtures/run-event-v0.2/envelope/run-finished.valid.json` 补新字段；
- `orz-assurance/journal/conformance.rs` 新增 schema 测试；`runtime/tests/test_run_event_journal_validation.py` 新增 `ZeroDmPayloadSchemaTests`。

### 2.2 nudge 状态机与草稿

- `terminal_nudged` bool → `terminal_nudge_count` + `last_nudge_tool_rounds`；每次工具轮产生新动作后才重挂，最多 `TERMINAL_NUDGE_MAX=2` 次；用尽 fail-open。
- nudge 前把候选草稿作为 assistant 轮提交；声明-only 的下一轮复用草稿作为终答，不重写全文；`pending_draft` 在工具轮清空。
- `TERMINAL_NOTICE` 文案改为「上文是你的草稿……只输出 [RUN_END]，不要重复正文」。

### 2.3 run 作用域

- `run_has_substance(svc, tool_rounds, run_id, plan_baseline)`：edits 按 `EditRecord.run == current_run_id`，plan 用 run 起点快照比较；恢复会话的旧 edits/未完成 plan 不再触发声明。
- `LoopOutcome` 携带 `terminal_nudge_count` 与 `terminal_declaration`；短聊 `short_text` 不再被会话级状态击穿。

### 2.4 终态来源与解析

- `terminal_source` 实际闭集：`run_end`/`text_without_terminal`/`short_text`/`budget_exhausted`/`gate_block`/`close_round`/`unspecified`；删除不可达的 `cancelled`/`degraded`（取消/降级分别走 `run_cancelled`/`run_terminated`）。
- `extract_model_stop` 改行级、fence 感知；`[RUN_END]`/`[/RUN_END]` 必须独占一行；代码块示例、行内提及不再误判；后文未闭合标记不再吞掉前文合法声明。
- 新增 `strip_model_stop`；终答正文剥离声明块，`intent/reason/summary` 从 `terminal_declaration` 落账。

### 2.5 车道隔离与开关

- `LoopProfile` 增 `terminal_declaration`/`self_check_prompt`：main true，grill/retrieval false；常驻行只按车道位注入。
- `self_check` 装配条件改为 `console || terminal_declaration`；handler 增 activation 拒绝；子代理投影剔除 `self_check`。
- `declaration_required = controller && env`；旧门与声明要求共用该解析；off + 有执行事实记 `text_without_terminal`。
- `--plan` 生产入口补 `with_terminal_declaration_required(true)`。
- `submit` handler 按 `submit_tool_enabled()` 拒绝（`submit_retired`），不再执行交付/推进计划。
- `deliver` 末步由 `[RUN_END]`（conclude/completed）机械置 Done。

### 2.6 UI

- Web 工具栏「反例自查」初始 disabled，`syncToolbar` 按 `state.running` 同步；
- TUI `Ctrl+G` 投递 `SELF_CHECK_SUPPLEMENT`，非运行中只提示；
- `plan_write` 描述/校验错误文案收窄为 `deliver`。

## §3 验证

- `orz-loop` **908/0**（3 ignored）
- `orz-assurance` **304/0**
- `orz-tui` **186/0**
- `orz-web` **52/0** ＋ frontend smoke 全过（含空闲 disabled 断言）
- 新增：nudge 重挂/上限、短聊作用域、notice 不落侧车、草稿复用、deliver 完成、refused self_check 不计、submit 退役拒绝、RUN_END fence/未闭合、schema 两测。
- `orz-bin --bins` check 通过（含 `--plan` builder）。

## §4 未做／后续

- S2：交付状态 UI 面板（`compute_delivery_status` 的按需展示）；
- S3：0.9.6 双平台重建＋装机位换装＋身份门；
- S4：三会话真机（短聊/进度文本/显式自查）＋旧 journal 回放；
- 旧 journal 的 `counterexample_gate`/`submit` 事件仍可解释，Schema 允许；未新增以旧刊为输入的专门回放测试（S4 覆盖）。

## §5 关键词

0dm、v0.2、run_finished schema、gate_decision nudge、terminal_nudge_count、run 作用域、草稿复用、声明剥离、车道隔离、submit_retired、deliver、325。
