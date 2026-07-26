# GSA self-question and counterexample sentinel design（2026-07-26）

## 裁决

本文件只记录设计，不启用新的 runner 行为。目标是给后续 DeepSeek/Grok 等 runtime adapter 增加一个外部、
可审计的自我质询与停滞截断层，用来降低重复性漏洞、单一路径过推进和反例覆盖不足风险。

该机制不得读取或保存隐藏 chain-of-thought。它只处理公开输出、结构化中间摘要、artifact digest、journal event
metadata 和显式 claim/counterclaim 表。

## 设计目标

1. 在 claim promotion 前生成最小反例清单；
2. 把“自我质询”产物降级为 challenge candidates，不直接提升或降低科学结论；
3. 在模型连续输出高重复词、重复短语或无新增 artifact/digest 的停滞段时截断当前对话；
4. 用已验证 task state、artifact ledger 和最近 terminal-safe summary 重启 runtime，而不是把 runaway suffix 带入新会话。

## Self-Question Pass

候选 checkpoint：

- `pre_claim`: 每个新 claim 首次进入 report 前；
- `pre_verification`: 机械 verifier 通过后、人工/模型解释前；
- `pre_handoff`: 交接给下一个 runtime 或 reviewer 前；
- `stagnation_recovery`: 重复截断并重启后。

每个 checkpoint 只产出结构化问题：

- claim 是否依赖未登记来源；
- 是否存在单位、population、condition、transformation 或 producer lineage 不一致；
- 是否存在同一证据被重复计为独立证据；
- 能否构造一个最小 negative fixture，使当前 claim 应降级或 defer；
- 当前结论是否把 mechanical PASS 误解释为 scientific truth。

输出只允许进入 `counterexample_candidate` 或 `review_question`，不能直接写 PASS。

## Repetition Sentinel

初始触发条件建议全部 metadata-only：

- 最近 128 个 normalized word token 中，同一非停用词出现不少于 16 次；
- 任意 3-8 token n-gram 在最近输出中重复不少于 6 次；
- 连续 3 个段落 canonical-normalized 后相同，或仅数字/标点不同；
- 连续 2 个 assistant turns 没有新增 artifact digest、verification result、source pointer 或 decision delta；
- 模型输出进入明显循环，并且下一步计划文本与上一轮计划 canonical 相同。

触发后动作：

1. 停止消费当前模型流；
2. 在 journal 中记录 `run_invalidated` 或候选 `continuity_restart_requested` metadata event；
3. 生成 terminal-safe restart packet：task contract、已验证 artifact ledger、未解决问题、最后一个有效 checkpoint digest；
4. 丢弃 runaway suffix，只把 restart packet 交给新的 runtime session；
5. 限制自动重启次数，超过阈值后交回用户或 reviewer。

## Counterexample Queue

自我质询通过后，只能新增以下候选对象：

- `negative_fixture_candidate`;
- `source_gap_candidate`;
- `lineage_conflict_candidate`;
- `independence_double_count_candidate`;
- `claim_scope_reduction_candidate`.

每个候选必须包含：

- 关联 claim ID；
- 最小触发条件；
- 需要新增或检查的 source/artifact；
- 预期 gate 结果；
- 为什么它不提升当前 claim 强度。

## 风险

- 自我质询容易变成形式化自证，因此必须外部 schema 化、可重放、可反例测试；
- 重启可能丢失真实上下文，因此 restart packet 必须只来自已验证 ledger；
- 过早截断会降低长任务效率，因此初期只对高重复与无进展循环启用；
- 对 DeepSeek V4 这类长推理模型，不应保存私有思维链，只保存公开、短摘要和 digest。

## 下一步

先实现只读 detector 和 fixture，不接入自动截断；通过正反例证明它能识别高重复 runaway，同时不会误伤正常长报告。
