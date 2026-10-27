# 368 批：纯语义摘要控制轮不再被当终答（run 半路收口修复）

> 日期：2026-10-27
> 用户令：「请查看本轮日志，我怀疑是 run_end 退役以后你输出的压缩块会被识别成终答导致触发运行完成。」
> 上游：[`353 回合原子性与终态契约`](353_TURN_ATOMICITY_AND_TERMINAL_CONTRACT_2026-10-25.md) / [`355 处置`](355_353_354_REVIEW_REMEDIATION_2026-10-25.md) / v8 滑块上下文（`context_scale.rs`）。
> 后续：[`369 批`](369_CONTEXT_MANAGE_SUMMARY_CONTROL_PLANE_2026-10-27.md) 把摘要载体迁到 `context_manage(mode=compress, summary=…)` 工具控制面；本批的纯文本 fallback 保留。
> 性质：**缺陷修复**；不改 M1-S1 语义、不改工具面、不改 journal 契约。工作树未提交。

## §0 结论

用户判断正确。**压缩摘要块被 353 §5.1 的「无 tool_calls ⇒ 回合结束」规则当成了终答**：

1. 模型按 H1 硬提醒／压缩窗口提示，只输出一个 `[SEMANTIC_SUMMARY]` 块，`finish_reason=stop`、无工具调用；
2. 终答分支把该文本当最终答案，`terminal_source="text_without_terminal"`，`break`；
3. run 尾安全网消费摘要、落 `context_compressed`，随后 `run_finished`——任务停在压缩动作上，没有继续原任务。

实锤（本会话）：

| run | 关键序列 | 结果 |
|---|---|---|
| `RUN-7ef4c983-1` | seq 1140 `model_output(text=[SEMANTIC_SUMMARY]…)` → 1141 `context_compressed` → 1143 `run_finished{terminal_source:"text_without_terminal"}` | 半路收口 |
| `RUN-7ef4c983-2` | seq 832 同上 → 833 `context_compressed` → 835 `run_finished{terminal_source:"text_without_terminal"}` | 半路收口 |

## §1 根因

- v8 的语义摘要块是**模型面控制信号**，不是用户可见终答；它由 `context_scale::extract_model_summary` 识别，进入 `pending_semantic`，在下一个 loop-top 安全间隙按块压缩。
- 353／355 把回合边界改为结构判定（无 `tool_calls` 的文本＝回合结束），但终答分支没有区分「摘要控制轮」与「真实终答」；纯摘要文本没有工具调用，于是提前 break。
- run 尾本来就有「终止轮语义摘要必须消费」的安全网（2026-09-15 审查 P1②），所以摘要不会丢，但 run 已经结束——症状表现为「压缩完成后直接运行完成」。

## §2 修复

### 2.1 纯摘要识别（`context_scale.rs`）

新增 `pub fn is_summary_only(text: &str) -> bool`：

- 与 `extract_model_summary` 同源（至少命中 2 个小节才可识别）；
- 摘要块**之前/之后**的可见文本必须为空；
- 少于两个段标签、摘要块前后有实质文本 ⇒ `false`（按终答处理）。

### 2.2 loop 控制轮分流（`agent_loop.rs`）

- 识别点同时记录 `summary_control_turn`：主车道 + 有摘要 + `tool_calls` 为空 + 无 `[RUN_END]` + `is_summary_only`。
- checkpoint（ModelCompression / Orientation）分支处理完毕后、plan/终答分支之前：
  - 清掉 `defer_semantic_once`（防止摘要跨轮悬挂）；
  - `continue` 回 loop-top，消费 `pending_semantic` 并按块压缩；
  - 不把摘要块推入 `messages`，不进入终答交付。
- 边界保持：
  - `[RUN_END]` + 摘要 ⇒ 真实终答（控制轮 false）；
  - 摘要 + 实质文本 ⇒ 真实终答（run 尾消费摘要，既有 `final_answer_semantic_summary_is_consumed_at_the_run_tail` 仍覆盖）；
  - 工具轮文本 + 摘要 ⇒ 照常派发工具，loop-top 消费摘要（不变）。

## §3 测试

| 测试 | 覆盖 |
|---|---|
| `context_scale::tests::summary_only_distinguishes_control_turn_from_mixed_final_answer` | 纯块 / 前后有文本 / 段标签不足 |
| `agent_loop::tests::pure_semantic_summary_turn_continues_instead_of_finishing` | 两工具轮 → 纯摘要 → 真实终答；断言 `context_compressed` 恰一次、模型调用 4 次、`run_finished{text_without_terminal}` 在真实终答后 |
| `final_answer_semantic_summary_is_consumed_at_the_run_tail`（改写） | 摘要 + 终答正文同轮 ⇒ 仍按终答、run 尾消费摘要 |
| `context_compress_end_to_end_request_window_summary_and_selected_reason`（改写） | 控制轮后补真实终答；压缩事件/归因不变 |
| `block_compaction_event_carries_the_true_rounds_since_counter`（改写） | 控制轮后补真实终答；`rounds_since_last_compaction=3` 不变 |

读数：`cargo test -p orz-loop --lib` **934 passed / 0 failed / 3 ignored**。本批只改 loop 内部判定，host / assurance / Python 无契约面变化；未重跑全量宿主（M1-S1 批已串行 453/0/6）。

## §4 边界与后续

- 这不是 M1-S1 state 代码的回归；是 v8 压缩控制面与 353 文本回合契约的交互缺陷，M1-S1 的上下文增长只是更早触发了它。
- 未提交、未重建载体、未换装。
- 后续可考虑（未做）：在 `model_output` 增加 `semantic_summary_only` 观测位，便于统计控制轮频率；当前用 journal 文本 + `context_compressed` 已可审计。
- 下一步回 M1-S2 `context_map`。

## §5 入口

- 代码：`orz/crates/orz-loop/src/context_scale.rs`（`is_summary_only`）、`orz/crates/orz-loop/src/agent_loop.rs`（`summary_control_turn` 分流）。
- 实锤：`D:\CLI\.gsa\runs\RUN-7ef4c983-1\events.jsonl` seq 1140-1143；`RUN-7ef4c983-2\events.jsonl` seq 832-835。
