# GSA orientation checkpoint and stagnation guard design（2026-07-26）

## 裁决

本文件记录设计与首个 no-model/read-only fixture，不启用新的 runner 行为。本轮修订把原先混在一起的“自我质询/反例/停滞哨兵”
拆成三个互相隔离的设计面：

1. **Orientation Checkpoint**：中性定位问题，用来把模型拉回当前任务、原始目标和已授权边界；
2. **Counterexample Queue**：独立的反例候选队列，只在明确进入 claim review 或科学保障审查时使用；
3. **Runtime Stagnation Guard**：机械重复/停滞检测，用来处理网页端或 runtime 输出无限重复等 runaway 状态。

该机制不得读取或保存隐藏 chain-of-thought。它只处理公开输出、结构化中间摘要、artifact digest、journal event
metadata、task contract 和显式 claim/counterclaim 表。当前没有把这些设计声称为已实测经验；DeepSeek 网页端
“思维与输出无限重复单一内容”只作为用户观察到的风险形态，进入未启用的 runtime guard 设计。

## 设计目标

1. 周期性提供中性 orientation checkpoint，提醒模型回答“当前正在做什么/任务定位是什么”；
2. 避免把 orientation checkpoint 设计成反向诱导、质疑模型正确性或强制寻找反例；
3. 把 counterexample queue 与 orientation checkpoint 分离，防止普通长任务被无意带入反向审查模式；
4. 用外部机械规则识别连续重复内容，并在阈值触发后停止、记录、有限重试或交回用户；
5. 所有设计产物只能降低风险或提出 review question，不能直接提升 claim 强度。

## Orientation Checkpoint

Orientation checkpoint 是中性的任务定位问题，不是科学 reviewer、反例搜索器或第二个 planner。最小版本可按固定步数、
固定 token 区间或进入 handoff 前插入，问题本身保持短、稳定、可审计。

建议使用与普通用户消息明显不同但仍稳定的格式，例如：

```text
[ORIENTATION_CHECKPOINT v0.1]
当前正在做什么？
当前任务定位是什么？
下一步输出应该服务哪个用户目标？
[/ORIENTATION_CHECKPOINT]
```

允许问题：

- 当前正在做什么；
- 当前任务定位是什么；
- 下一步输出应该服务哪个用户目标；
- 当前可用来源、权限和输出边界是什么；
- 哪些事项已经完成，哪些事项仍未完成。

避免问题：

- 当前动作是否正确；
- 当前是否已经偏移；
- 请证明当前方向错误；
- 请寻找反例推翻当前结论；
- 是否应该停止当前任务。

这些禁用形式会把中性定位变成反向诱导，增加模型防御性、自我审判或过度纠错的风险。orientation checkpoint
只允许写入 `orientation_summary`、`current_task_position` 或 `next_output_target`，不得生成 `counterexample_candidate`
或 claim disposition。

## Counterexample Queue

Counterexample queue 是单独机制，只在 claim-bearing review、科学保障审查或用户明确要求反例/缺口审查时进入。
它不由普通 orientation checkpoint 自动触发。

允许候选对象：

- `negative_fixture_candidate`;
- `source_gap_candidate`;
- `lineage_conflict_candidate`;
- `independence_double_count_candidate`;
- `claim_scope_reduction_candidate`.

每个候选必须包含：

- 关联 claim ID 或明确说明尚无 claim ID；
- 最小触发条件；
- 需要新增或检查的 source/artifact；
- 预期 gate 结果；
- 为什么它不提升当前 claim 强度。

Counterexample queue 的输出只允许进入 `review_question` 或 `counterexample_candidate`，不能直接写 PASS，也不能由同一轮
orientation checkpoint 自动升级为正式反例结论。

## Runtime Stagnation Guard

Runtime stagnation guard 是外部机械检测，不问模型“你是不是卡住了”。它只根据公开输出或 event/artifact
变化判断是否进入重复 runaway。

初始触发条件建议保守、metadata-only：

- 连续重复内容次数大于 10；
- 任意 3-8 token n-gram 在最近输出中重复不少于 10 次；
- 连续多个段落 canonical-normalized 后相同，或仅数字/标点不同；
- 连续 2 个 assistant turns 没有新增 artifact digest、verification result、source pointer 或 decision delta；
- 下一步计划文本与上一轮计划 canonical 相同，且没有新增 task/journal state。

触发后动作：

1. 停止消费当前模型流；
2. 在 journal 中记录候选 `runtime_stagnation_detected` 或 `continuity_restart_requested` metadata event；
3. 生成 terminal-safe restart packet：task contract、已验证 artifact ledger、未解决问题、最后一个有效 checkpoint digest；
4. 丢弃 runaway suffix，只把 restart packet 交给新的 runtime session；
5. 限制自动重试次数，超过阈值后交回用户或 reviewer。

对于 DeepSeek 网页端这类明显重复单一内容的现象，最小可行策略是：连续重复内容次数 `> 10` 时自动断开并重试。
该策略仍需 runtime-specific fixture 验证，且不得把一次重试成功解释为科学能力提升。

## 初始实现

首个只读实现位于 `assurance/orientation_runtime_guard.py`，并新增三个结构化合同：

- `assurance/orientation-checkpoint-v0.1.schema.json`;
- `assurance/orientation-checkpoint-verification-v0.1.schema.json`;
- `assurance/runtime-stagnation-guard-receipt-v0.1.schema.json`.

当前实现只提供：

- `build_orientation_checkpoint`：生成固定格式的中性 `[ORIENTATION_CHECKPOINT v0.1]` block；
- `verify_orientation_response`：确认 response 只含 `orientation_summary`、`current_task_position` 和
  `next_output_target`，拒绝 `counterexample_candidate`、`claim_disposition` 等字段；
- `evaluate_runtime_stagnation_guard`：基于公开输出的连续重复和 n-gram 重复阈值，返回
  `continue`、`restart_requested` 或 `handoff_required` receipt。

初始测试在 `assurance/tests/test_orientation_runtime_guard.py` 中覆盖中性问题、反例字段拒绝、连续重复
`> 10` 触发重启、retry budget 用尽后 handoff，以及普通变化输出不误伤。

接入前置 fixture 位于 `assurance/orientation_runtime_integration.py`，并新增两个结构化合同：

- `assurance/orientation-stagnation-integration-fixture-v0.1.schema.json`;
- `assurance/orientation-stagnation-integration-receipt-v0.1.schema.json`.

该 fixture 输入包含 `task_id`、`task_contract_sha256`、`step_index`、公开输出列表、retry 计数/预算和阈值；
运行后只写出 `orientation-checkpoint.json`、`runtime-stagnation-guard-receipt.json` 与汇总 receipt。独立 verifier
会从 `fixture-input.json` 重建两个 artifact 和汇总 receipt，检查 digest 与边界字段。它证明 orientation 与
stagnation guard 能被运行流程安全消费，但仍不接 runner、不写 journal、不截断真实会话。

Runtime event/journal 接入面位于 `assurance/orientation_runtime_journal.py`，并新增三个结构化合同：

- `assurance/orientation-checkpoint-event-payload-v0.1.schema.json`;
- `assurance/runtime-stagnation-guard-event-payload-v0.1.schema.json`;
- `assurance/orientation-stagnation-journal-receipt-v0.1.schema.json`.

`runtime/run-event-v0.1.schema.json` 新增 `orientation_checkpoint` 与 `runtime_stagnation_guard` 两个事件类型。
当前 journal projection 固定事件顺序：

```text
run_preflight
run_started
orientation_checkpoint
runtime_stagnation_guard
run_finished | run_invalidated
```

若 stagnation decision 为 `continue`，terminal event 为 `run_finished`；若为 `restart_requested` 或
`handoff_required`，terminal event 为 `run_invalidated`。verifier 会重建 manifest、检查 JSONL hash-chain、
payload schema、orientation-before-stagnation 顺序，以及无 model/tool event、无 runner、无 counterexample queue。

公开输出抽取层位于 `assurance/runner_public_output.py`，并新增两个结构化合同：

- `assurance/runner-public-output-stream-v0.1.schema.json`;
- `assurance/runner-public-output-extraction-receipt-v0.1.schema.json`.

该层面向真实 runner adapter 之前的混合 stream fixture。只有 source 为 `assistant`、visibility 为 `public`、
channel 为 `assistant_delta` 或 `assistant_final` 的记录可以进入 `public_outputs`；`reasoning_private`、
`tool_private`、`system_internal` 和 redacted metadata 均被排除。schema 要求 private/redacted 记录只能保存
`content_sha256`，禁止保存 text。extraction receipt 同时冻结 restart packet 来源策略：只允许 task contract、
verified artifact ledger、unresolved questions 和 last valid checkpoint digest，不允许 runaway suffix 或 hidden reasoning。

Direct DeepSeek API one-shot 观测位于 `assurance/deepseek_api_observation.py`、
`scripts/build_deepseek_public_output_observation.py` 与 `scripts/invoke_deepseek_public_output_observation.ps1`，
并新增两个结构化合同：

- `assurance/deepseek-api-observation-result-v0.1.schema.json`;
- `assurance/deepseek-api-observation-pipeline-receipt-v0.1.schema.json`.

该路径用于在缺少专门 runaway 样例时直接观测真实 API 的公开输出接线。PowerShell launcher 固定为
Plan/Execute 两阶段：Plan 不读凭据、不联网；Execute 只允许一次 `deepseek-v4-pro` chat/completions 请求，
`retry_budget=0`，无工具、无 web、无 subagents，并记录固定 public marker 是否匹配。落盘 result 不保存 credential、
raw request、raw response 或 hidden reasoning text；如 provider 返回 private reasoning 字段，只保存 digest。
随后 Python pipeline 把 public assistant text 投影到 runner public-output extraction、orientation/stagnation integration
和 runtime journal。该观测只证明真实 API response 可以被当前公开输出 guard 管线消费，不证明真实 runner 已接入，
也不能单独覆盖重复停滞重启分支。

DeepSeek-shaped streaming/repetition fixture 位于 `assurance/deepseek_stream_observation.py` 与
`scripts/build_deepseek_stream_observation_fixture.py`，并新增两个结构化合同：

- `assurance/deepseek-stream-observation-fixture-v0.1.schema.json`;
- `assurance/deepseek-stream-observation-pipeline-receipt-v0.1.schema.json`.

该 fixture 消费已脱敏的 DeepSeek 风格 stream chunks，不捕获真实 SSE，也不调用模型。public `delta.content`
映射为 runner public `assistant_delta`；private reasoning 只能以 `reasoning_content_sha256` 映射为
`reasoning_private` digest record；finish/usage 只进入 redacted metadata。回归测试覆盖 11 次相同 public delta
触发 `restart_requested` 和 `run_invalidated`，同时验证非连续 chunk sequence fail-closed。该层证明 DeepSeek
风格 chunk 可被公开输出抽取和 stagnation guard 消费，但仍不执行真实 stream 截断、session restart 或 retry。

## 风险

- Orientation checkpoint 可能变成形式化自证；因此问题必须短、中性、固定，并与普通消息格式区分；
- 同一模型可能受思维定势和惯性动作影响；特殊块格式只能作为轻微扰动，不能视为可靠独立审查；
- Counterexample queue 若自动挂到普通任务上，可能把模型向反向结论诱导；
- Stagnation guard 过早截断会降低长任务效率，因此初期只对高重复与无进展循环启用；
- 重启可能丢失真实上下文，因此 restart packet 必须只来自已验证 ledger；
- 对 DeepSeek V4 这类长推理模型，不应保存私有思维链，只保存公开、短摘要和 digest；
- 自动重试本身可能形成外部无限循环，因此 retry budget 必须固定且默认很小。

## 下一步

下一步适合补真实 streaming 场景下的截断/重启边界和 runner adapter 策略：固定 step/token 触发条件、
真实 SSE/runner record 到公开输出抽取层的映射，以及 restart packet 如何绑定已验证 state。
Counterexample queue 暂不接入 orientation checkpoint；后续只在明确的 claim review 路径中单独实现。

---

# 设计修正与补充（2026-08-02）

状态：alpha 机械检测完成后，重新审查中立问询机制的触发模型和注入时机，发现当前实现与设计意图存在
结构性偏差。以下修正原文档中未明确写出的关键设计约束。

## 修正 1：Orientation Checkpoint 是中途引导，不是前置拦截

### 原理解（错误）

```
用户输入 → [塞入 orientation block] → 模型 → 回复
```

把 orientation checkpoint 当作 prompt 发送前的静态文本注入，和 tool availability context block
同一性质。这在 `canonical_cli.py` 中就是这样实现的——所有 gate 在模型调用前按固定顺序跑一遍，
orientation checkpoint 只是其中一个步骤。

### 纠正后的理解（正确）

```
硬门控下发 → 模型思考/执行动作
  → [某个动作完成，动作间隙]
      → token 计数器 ≥ 阈值? 或 action 计数器 ≥ 阈值?
         是 → 注入 orientation checkpoint（不打断思维链）
         否 → 继续
  → 模型收到 orientation，回复定位摘要
  → 模型继续下一个动作
  → [检索/子代理返回后，正式回复前]
      → 信息收集确认: "已获取 {N} 个来源，覆盖范围 {scope}。是否足够回答？"
  → 最终回复
```

### 关键约束

1. **触发位置：动作间隙**。只在 `tool_completed`、`turn_completed` 等动作边界事件之后注入，
   绝不在模型思考过程中打断。

2. **触发条件：硬门控**。由 token 计数器或 action 计数器的机械阈值决定，不由模型自述
   "我觉得该检查了" 触发。阈值应为固定配置项（默认：单轮 5000 token、8 个 action、
6 种不同工具类型，OR 触发）。

3. **不打断思维链**。Orientation checkpoint 是一条独立的、格式区别于普通对话的消息。
   它不应要求模型"停止当前推理并重新思考"，而是"在当前位置做一个轻量定位标记"。

4. **格式区分**。`[ORIENTATION_CHECKPOINT v0.1]` 块必须格式明显区别于用户消息和系统消息，
   使模型能将其识别为"中途引导"而非"新任务"或"质疑"。

## 修正 2：新增信息收集确认（Post-Retrieval Information Sufficiency Check）

### 触发时机

子代理检索或任何外部信息获取动作完成后、模型给出正式回复前。

### 触发条件（硬门控）

- 检索动作已返回结果（`tool_completed` for retrieval tools）
- 模型尚未发出面向用户的 `assistant_final` 回复
- 动作间隙中注入

### 内容

```
[INFO_SUFFICIENCY_CHECK v0.1]
已获取来源: {source_count} 项
覆盖范围: {categories}
全文可见: {fulltext_count}/{source_count}
缺失: {missing_categories}

当前信息是否足够回答用户问题？
如不足，还需哪些信息？
[/INFO_SUFFICIENCY_CHECK]
```

### 约束

- 绝不暗示"你应该质疑当前结论"
- 只陈述**机械可验证**的事实（来源数量、类别、全文可见性）
- 不允许模型用此机制绕过 source visibility gate 的 block/defer 决策
- 不可作为"模型已经查过了"的证明——这只是引导，证据在 gate receipt

## 修正 3：触发模型从"线性管道"改为"事件驱动"

### 原设计（canonical_cli.py 当前）

```
run_preflight → IPG → tool_availability → orientation → source_visibility
→ gate_decision → model_request → model_output → artifact → run_finished
```

所有 gate 在模型调用前按固定顺序一次性执行。这适合 conformance fixture（证明形状能跑通），
但不适合生产运行时——生产中的动作是多轮的，gate 需要在运行时按事件触发。

### 纠正后的设计

```
run_preflight
  → [IPG: 每个新输入/指令来源]
  → [tool_availability context: session 开始时注入一次]
  → run_started
  → [action 循环]
      ├─ action_proposed
      ├─ action_approved/denied (P3 permit)
      ├─ action_running
      ├─ action_completed ← 触发点: token 计数检查
      │   ├─ 达到阈值? → orientation checkpoint 注入
      │   └─ 未达到? → 继续
      ├─ [如果是检索动作]
      │   └─ action_completed ← 触发点: 信息收集确认
      └─ ...
  → model_output
  → [stagnation guard: 公开输出重复检测]
  → run_finished | run_invalidated
```

### 硬门控参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| `orientation_trigger_tokens` | 7000 | 单轮累计公开输出 token 超过此值时触发 |
| `orientation_trigger_actions` | 10 | 单轮累计动作数超过此值时触发 |
| `orientation_trigger_tool_variety` | 7 | 单轮使用不同工具种类数（按 tool_id 去重）超过此值时触发 |
| `orientation_trigger_turn_interval` | 8 | 每 N 轮对话后，下一轮开始时必定触发一次 |
| `orientation_cooldown_actions` | 3 | 两次 orientation 之间的最小动作间隔 |
| `info_sufficiency_after_retrieval` | true | 任何检索动作完成后触发 |
| `stagnation_repeat_threshold` | 10 | 连续重复内容触发停滞检测 |
| `stagnation_ngram_threshold` | 10 | n-gram 重复触发停滞检测 |

触发条件为 **OR 关系**。`tool_variety` 按 `tool_id` 去重计数。
`turn_interval` 不受 cooldown 限制——跨轮漂移不能因"上轮刚查过"而跳过。

### 设计依据

两条防线分工：

| 防线 | 阈值 | 防范目标 |
|------|------|----------|
| **轮次触发** | 每 8 轮必定触发 | 跨轮缓慢漂移（每轮都短、都正常，但累计已偏） |
| **单轮阈值** | 7000 token / 10 actions / 7 tool types | 单轮异常发散（体量、复杂度、工具面异常大） |

轮次触发让单轮阈值可以从"尽早发现"调高到"发现真正的异常"：

| 阈值 | 为什么是这个值 | 为什么轮次触发让它可以从低值上调 |
|------|---------------|----------------------------------|
| 7000 token | 大型重构/多文件改动可能产生 5000-7000 token，超过才异常 | 即使每轮只输出 2000 token，第 8 轮也会被强制检查 |
| 10 actions | 大型任务 8-10 步是正常体量（如逐个文件重构） | 即使每轮只做 3-4 步，第 8 轮也会被强制检查 |
| 7 tool types | ~10 种工具中用到 7 种 = 70%，高度发散 | 正常中型任务可能用到 5-6 种，7 种才是异常 |

### 触发伪代码

```text
# 轮次触发 — 不受 cooldown 限制
on turn_start(next_turn_number):
    if next_turn_number % 8 == 0 and next_turn_number > 0:
        inject orientation checkpoint
        reset cooldown counter
        return

# 动作间隙触发 — 受 cooldown 限制
on action_completed:
    if cooldown_actions > 0:
        decrement cooldown; return
    if (turn_output_tokens >= 7000)
       or (turn_action_count >= 10)
       or (distinct_tool_types_this_turn >= 7):
        inject orientation checkpoint
        reset cooldown counter
```

## 修正 4：Counterexample Queue 与 Orientation 的隔离

（原文档已明确，此处重申关键规则）

- Orientation checkpoint **绝不**自动触发 counterexample queue
- Counterexample queue 只在显式 claim review 或用户要求反例审查时进入
- 两者之间无自动桥接——需要独立的 gate decision

## 实现差距

当前实现状态（`assurance/orientation_runtime_guard.py`）：

| 组件 | 设计状态 | 实现状态 |
|------|----------|----------|
| `build_orientation_checkpoint` | 格式正确 ✅ | 已实现 ✅ |
| `verify_orientation_response` | 验证逻辑正确 ✅ | 已实现 ✅ |
| `evaluate_runtime_stagnation_guard` | 检测逻辑正确 ✅ | 已实现 ✅ |
| 动作间隙触发 | 事件驱动，动作边界 | **未实现** ❌ — 当前是 `canonical_cli.py` 中静态调用 |
| 硬门控（token/action 阈值） | 机械计数器 | **未实现** ❌ |
| 思维链保护 | 不打断 in-progress 思考 | **未实现** ❌ — 当前在 model_request 前一次性注入 |
| 信息收集确认 | 检索后、回复前 | **未实现** ❌ |
| 事件驱动触发模型 | 运行时事件 → gate | **未实现** ❌ — 当前是线性管道 |

核心 gap：**checkpoint 格式和验证逻辑是正确且可复用的，但触发和注入机制需要完全重写。**
