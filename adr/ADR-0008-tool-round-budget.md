# ADR-0008：模型↔工具轮次预算（MAX_TOOL_ROUNDS 8→40 + 双层防失控）

- 状态：accepted
- 日期：2026-08-07
- 关联：`docs/FIX_PLAN_2026-08-06.md` D-8（跑分 P7/LOOP-14）、`docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md` P7、`docs/DESIGN_IMPLEMENTATION_DEVIATION_AUDIT_2026-08-06.md` LOOP-14、`orz-loop/src/controller.rs`、`orz-loop/src/prompt.rs`（`TOOL_ROUND_BUDGET`）、ADR-0007（transport 重试，同批定稿）

## 1. 背景

Polyglot 摸底（2026-08-06）P7：分析型任务（读 3-4 文件 + 多轮 search_replace + 验证）多题撞 `MAX_TOOL_ROUNDS=8` 上限（`gate_decision: tool_rounds_limit` 出现于 transpose/wordy/error-handling/tree-building），且被拒工具重试（P3）加速撞限。

审计 LOOP-14 交叉复核：**P7 记录「Python 参考实现口径移植」失实**——Python 参考实现实为 `max_turns=20/max_tool_calls=0`（无 tool_rounds 痕迹）；`8` 实为 Grok 生态默认值（mcp-grok maxTurns 默认 8）。行业参考：OpenCode maxTurnsPerAgent 30 / Goose 1000 / Gemini 子代理 15 / Claude Code 由调用方设定（社区建议 3/5-10/25）。

## 2. 决策

### 2.1 语义：anti-runaway backstop（不是任务时长预算）

- 轮次上限是**防失控兜底**（无限工具循环的终止保证），不是「任务应当在此内完成」的预算。任务收敛依赖工具反馈环（P6/harness run_tests，D-9）与熔断（D-3），轮次上限只保证终止。
- 业界共识（检索实证）：防失控靠「连续同类失败熔断」而非单靠全局轮次——本 ADR 与 ADR-0007/IP2a 的连续拒绝熔断（3 次/轮 + 总量 10/轮）构成双层：熔断处理「重复失败」，全局预算处理「合法但永不收敛」。注意（2026-08-07 提交前审查 F-05 修正）：**deny 轮计入轮次预算**——deny 结果轮仍是完整模型轮，熔断的缓解手段是名级过滤（策略拒绝的工具不出现在声明中）+ 注入消息，40 轮预算对所有轮（含 deny 轮）保证终止。

### 2.2 数值：40（默认）

- `MAX_TOOL_ROUNDS` 8 → **40**。量级依据：分析型任务实测 8 偏紧（P7），40 覆盖「读多文件 + 多轮编辑 + 验证 + 反馈环闭环」的完整工作量，同时仍是确定性终止边界（40 轮 × 模型轮延迟仍在分钟级）。
- 仍可经 `AgentLoopController::max_tool_rounds` 覆盖（测试/特定 host 收紧）。

### 2.3 模型可见形态（机械告知，模型不猜不漂移）

- **会话声明**：system prompt 注入 `[TOOL_ROUND_BUDGET v0.1]`（BUDGET + REMAINING 初始值）。
- **每轮机械告知**：每个工具轮后控制器注入 `REMAINING: N` 消息（模型可提前收拢——D-8「方便模型提前收拢和记录」）。
- **到限语义**：预算耗尽后模型获得**一个最终无工具轮**报告 partial result（注入「预算耗尽，报告最佳部分结果，勿再调用工具」）；若该轮仍请求工具则拒绝执行、直接结束。journal 记录 `GateDecision{tool_rounds_limit, max_tool_rounds}`。
- 预算消息为注入文本（`is_injected_block_text` 前缀匹配），不计入 stagnation 统计。

### 2.4 记录修正

- P7 的「Python 参考实现移植」表述**撤回**，修正为「Grok 生态默认 8（mcp-grok maxTurns）」。Python 参考实现无 tool_rounds 机制。

## 3. 后果

- `controller.rs`：常量 8→40 + budget 注入（会话声明/每轮剩余/到限收尾）+ `budget_exhausted` 终态轮 + journal 记录 max 值。
- 测试：`round_budget_declared_and_decremented_mechanically`（40→39→38 递减）、`round_budget_exhaustion_reports_partial_result`（cap=1 注入 + 部分结果轮）。
- 与 IP2a 熔断（ADR 同批）：连续拒绝 3 次注入策略切换消息、总量 10 注入更强停止消息。**2026-08-07 提交前审查修正（F-05）**：原「被拒工具不再烧预算」表述撤回——实现为 deny 轮计入轮次预算（deny 结果轮是完整模型轮，`tool_rounds += 1` 无条件）；「重复失败」由名级过滤 + 熔断消息承担缓解，40 轮预算保证终止。
- 遗留：P6 反馈环（run_tests）是「预算内闭环」的消费方，见 D-9/harness 项。
