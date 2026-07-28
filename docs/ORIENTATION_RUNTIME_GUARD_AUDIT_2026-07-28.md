# Orientation Runtime Guard 审计（2026-07-28）

## 裁决

新增 Orientation Runtime Guard 审计文档，对该 Gate 的三个核心模块进行独立审计。Orientation Runtime Guard 是 Canonical Guarded CLI Gate 序列中位于 Tool Availability Gate 之后、Source Visibility Gate 之前的中性方向检查点与运行时停滞守卫——通过机械注入中性方向问题、检测公开输出中的内容重复/N-gram 循环、以及检测模型对不可用工具的幻觉式提及，在不读取隐藏思维链的前提下判断 agent 是否陷入停滞或循环。

本轮为**审计补漏**（此前该 Gate 无独立审计文档，仅由 schema 与源码约束），不新增代码功能。全部三个模块遵循现有 no-model/offline 边界，不调用真实模型、不联网、不读取凭据。

## 问题背景

Agent 运行时存在两类常见的退化模式：

1. **方向漂移 (Orientation Drift)**：Agent 在长时间运行中逐渐偏离原始任务目标，开始执行无关操作或自我审查。需要在不注入反例偏见的前提下，定期要求 agent 确认当前方向。
2. **运行停滞 (Runtime Stagnation)**：Agent 陷入循环——重复输出相同或高度相似的内容、产生 N-gram 重复模式、或持续提及不可用的工具名。这些停滞信号在公开输出中即可检测，无需访问隐藏思维链。

该 Gate 的设计目标：
- **机械介入**：不依赖模型自我报告是否卡住（模型可能已陷入幻觉）
- **公开输出分析**：仅分析已发出的公开文本，不读取隐藏思维链
- **三层决策**：continue → restart_requested → handoff_required 的渐进升级
- **中性检查点**：方向确认必须中立——不能暗示"当前的行动是否正确"或诱导 counterexample

## 组件描述

### 1. Orientation Runtime Guard (`assurance/orientation_runtime_guard.py`, 384 行)

该模块承担三个独立职责：方向检查点生成/验证、内容停滞检测、工具信念停滞检测。

#### 1a. 方向检查点 (Orientation Checkpoint)

常量定义：
- `ORIENTATION_BLOCK`：中文中性方向提示块，三个问题：
  ```
  [ORIENTATION_CHECKPOINT v0.1]
  当前正在做什么？
  当前任务定位是什么？
  下一步输出应该服务哪个用户目标？
  [/ORIENTATION_CHECKPOINT]
  ```
- `ALLOWED_ORIENTATION_FIELDS`（4 个）：`orientation_summary`、`current_task_position`、`next_output_target`、`available_tools_acknowledged`
- `FORBIDDEN_ORIENTATION_FIELDS`（5 个）：`counterexample_candidate`、`claim_disposition`、`claim_promotion`、`negative_fixture_candidate`、`tool_belief_mismatch_statement`

函数 `build_orientation_checkpoint`：
- 生成固定间隔触发的中性方向检查点
- `trigger_type: "fixed_step_interval"` — 按步数触发
- `checkpoint_id` 格式：`ORIENT-{task_id}-{step:04d}`
- `claim_policy` 三项全部锁定：`may_generate_counterexample_candidate: false`、`may_set_claim_disposition: false`、`claim_strength_effect: "none"`
- 支持可选 `tool_availability_sha256` 绑定
- 输出经 `orientation-checkpoint-v0.1.schema.json` 验证

函数 `build_tool_availability_infused_orientation_checkpoint`：
- 将 Tool Availability context block 前置到方向块之前
- 模型在一次响应中同时确认工具可用性状态和当前方向

函数 `verify_orientation_response`：
- 验证 checkpoint schema 合法性
- 检查模型响应中的字段是否属于 allowed / forbidden
- 生成 verification receipt，含 `allowed_fields_observed`、`forbidden_fields_observed`、`unexpected_fields_observed` 及 4 项 boolean checks

#### 1b. 运行停滞检测 (Runtime Stagnation Guard)

辅助函数：
- `_normalize_text`：提取英文单词 + CJK 字符，用于内容比较
- `_tokenize`：按词边界分词
- `_max_consecutive_repeated`：检测连续重复的规范化输出
- `_max_ngram_repeat`：检测 N-gram (3-8) 的重复模式

函数 `evaluate_runtime_stagnation_guard`：
- 输入：`public_outputs`（公开输出序列）、`retry_count`、`retry_budget`、阈值参数、可选的 `progress_markers`
- 停滞信号：
  - `STAGNATION-CONSECUTIVE-REPEAT`：连续重复内容超过 `repeated_content_threshold`（默认 10）
  - `STAGNATION-NGRAM-REPEAT`：N-gram 重复超过 `ngram_repeat_threshold`（默认 10）
- 三态决策：
  | 条件 | decision | action | restart_packet |
  |---|---|---|---|
  | 无停滞信号 | `continue` | `none` | `null` |
  | 有停滞信号 + retry_count < retry_budget | `restart_requested` | `stop_and_restart` | 含 4 个保留状态项 |
  | 有停滞信号 + retry_count >= retry_budget | `handoff_required` | `stop_and_handoff` | `null` |
- `restart_packet` 保留四项状态：`task_contract`、`verified_artifact_ledger`、`unresolved_questions`、`last_valid_checkpoint_digest`
- `runaway_suffix_retained: false` — 明确不保留导致停滞的尾部输出

#### 1c. 工具信念停滞检测 (Tool Belief Stagnation)

函数 `evaluate_tool_belief_stagnation`：
- 扫描公开输出中的两种退化模式：
  1. **不可用工具名被显式提及** (`TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED`)：模型在输出中提到了已知不可用的工具名称 → 触发 restart
  2. **工具使用语言模式** (`TOOL-BELIEF-CAPABILITY-GUESSING`)：检测中英文工具使用语言（如"我将搜索"、"I will use the search tool"） → 仅记录警告，不触发 restart
- 检测基于子串匹配（已知工具名）和正则模式（工具使用语言）
- 仅当 `TOOL-BELIEF-UNAVAILABLE-TOOL-MENTIONED` 被触发时才设置 `valid: false` 和 `decision: "restart_requested"`
- `limitations` 字段记录已知边界：子串匹配遗漏别名、关键词模式可能误报、不读隐藏思维链

### 2. Orientation Runtime Integration (`assurance/orientation_runtime_integration.py`, 318 行)

集成夹具层，将三个独立检测组合为单一可运行的工作流。

函数 `run_orientation_stagnation_integration_fixture`：
- 加载并验证 fixture JSON（schema: `orientation-stagnation-integration-fixture-v0.1.schema.json`）
- 调用 `_build_artifacts` 一次性生成 5 个制品：
  1. `orientation-checkpoint.json`
  2. `runtime-stagnation-guard-receipt.json`
  3. `tool-availability-report.json`（仅当 fixture 含 `tool_specs` 时）
  4. `tool-availability-gate-receipt.json`（仅当 fixture 含 `tool_specs` 时）
  5. `tool-belief-stagnation-receipt.json`（仅当 fixture 含 `tool_specs` 时）
- 原子写入全部制品到 `output_root`
- 生成 `orientation-stagnation-integration-receipt.json`

函数 `verify_orientation_stagnation_integration_fixture`：
- **重建式验证**：从 fixture 重新计算全部制品，与磁盘上的观测值逐字节比对
- 任何制品不匹配时抛出 `AssuranceError` 并指明具体失败的制品

设计决策：
- 条件性 tool availability：无 `tool_specs` 时保持向后兼容，不写入额外文件
- 不使用模型、不联网、不读取隐藏思维链

### 3. Orientation Runtime Journal (`assurance/orientation_runtime_journal.py`, 544 行)

无模型运行时日志投影——生成符合 `run-manifest-v0.1.schema.json` 和 `run-event-v0.1.schema.json` 的 JSONL 事件流，模拟真实 runner 的事件形态。

事件序列（5-8 个事件，取决于 tool specs 是否存在）：

```
run_preflight → [tool_availability_check] → run_started → orientation_checkpoint → runtime_stagnation_guard → [tool_belief_stagnation] → run_finished | run_invalidated
```

核心约束：
- **Hash chain**：每个事件含 `previous_event_sha256`（首事件为 null），形成防篡改链
- **Event self-digest**：每个事件的 `event_sha256` 是对除自身外所有字段的 SHA256
- **Payload digest**：`payload_sha256` 独立于事件摘要
- **严禁模型/工具事件**：`MODEL_OR_TOOL_EVENTS` 集合中的 6 种事件类型（model_request, model_output, tool_proposal, permission_decision, tool_started, tool_completed）在任何情况下不得出现
- **Redaction policy**：全部事件 `redaction: "metadata_only"`
- **Manifest 协议 binding**：run manifest 包含对 agent protocol、reason codes、gate matrix、scoring protocol 文档的 SHA256 digest，形成对协议版本的密码学承诺

函数 `write_orientation_stagnation_runtime_journal`：
- 先验证 integration fixture 可重建
- 生成 run manifest → 构建事件序列 → 写入 JSONL → 生成 journal receipt

函数 `verify_orientation_stagnation_runtime_journal`：
- 完整的 N 步重建验证：
  1. 验证 integration fixture
  2. 重建 manifest 并与磁盘比对
  3. 加载 JSONL 事件并逐行验证 schema
  4. `_verify_events`：事件数量（5-8）、必需事件类型、排序约束、hash chain 完整性、payload digest、event self-digest、redaction 策略、禁止事件类型
  5. 重建 receipt 并与磁盘比对

## Gate 链位置

Orientation Runtime Guard 在 Canonical CLI Gate 序列中的位置：

```
IPG (GAK-INJ-001) → Tool Availability → [Orientation Checkpoint] → Source Visibility → Adapter Gate → Answer Packet → Journal → Verifier
```

Orientation Checkpoint 位于 Tool Availability Gate 之后——这使得方向检查点可以包含工具可用性上下文（infused checkpoint），同时不影响后续 Source Visibility Gate 对引用源的全文检查。

## Schema 体系

该 Gate 共关联 8 个 JSON Schema，分为三层：

### 核心层（assurance/）
| Schema | 用途 |
|---|---|
| `orientation-checkpoint-v0.1.schema.json` | 检查点结构：checkpoint_id 格式、message_block 正则（允许 `[TOOL_AVAILABILITY` 或 `[ORIENTATION_CHECKPOINT` 前缀）、字段枚举、claim_policy 常量约束 |
| `orientation-checkpoint-verification-v0.1.schema.json` | 验证 receipt：4 项 checks、allowed/forbidden fields 枚举 |
| `runtime-stagnation-guard-receipt-v0.1.schema.json` | 停滞 receipt：decision 三态枚举、action 三态枚举、reason_codes、restart_packet |
| `runtime-stagnation-guard-event-payload-v0.1.schema.json` | 停滞事件 payload：public_output_only + asks_model_if_stuck + hidden_chain_of_thought_saved 均为常量约束 |

### 集成层（assurance/）
| Schema | 用途 |
|---|---|
| `orientation-stagnation-integration-fixture-v0.1.schema.json` | 集成夹具：14 个必需字段、可选 tool_specs 数组、probe_registry |
| `orientation-stagnation-integration-receipt-v0.1.schema.json` | 集成 receipt：13 项 checks（含 5 个 const true、3 个 const false）、可选 tool availability artifacts |

### 日志层（assurance/）
| Schema | 用途 |
|---|---|
| `orientation-stagnation-journal-receipt-v0.1.schema.json` | 日志 receipt：7 项 checks、terminal_event 二态枚举 |
| `orientation-checkpoint-event-payload-v0.1.schema.json` | 检查点事件 payload：neutral_orientation_only const true、counterexample_queue_invoked const false、claim_strength_effect const "none" |

## 与其他 Gate 的集成

### 与 Tool Availability Gate 的关系
- `TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` 记录的扩展（infused checkpoint、tool belief stagnation）均实现在本 Gate 的模块中
- `build_tool_availability_infused_orientation_checkpoint` 将工具可用性上下文注入方向检查点
- `evaluate_tool_belief_stagnation` 作为独立检测与 content stagnation 并行运行
- Integration fixture 条件性地调用 `probe_tool_availability` 和 `build_tool_availability_gate_receipt`

### 与 Canonical CLI 的关系
- Journal 模块为离线主路径提供事件形态参考——真实 runner 接入后，`MODEL_OR_TOOL_EVENTS` 中的事件将不再被禁止

## 文件清单

### 审计覆盖的模块
| 文件 | 行数 | 用途 |
|---|---|---|
| `assurance/orientation_runtime_guard.py` | 384 | 方向检查点、停滞检测（内容 + 工具信念） |
| `assurance/orientation_runtime_integration.py` | 318 | 集成夹具：组合全部检测、原子写入、重建验证 |
| `assurance/orientation_runtime_journal.py` | 544 | 无模型运行时日志投影：JSONL + hash chain + run manifest |

### 关联 Schema 文件（8 个）
| 文件 | 用途 |
|---|---|
| `assurance/orientation-checkpoint-v0.1.schema.json` | 检查点结构 |
| `assurance/orientation-checkpoint-verification-v0.1.schema.json` | 验证 receipt |
| `assurance/runtime-stagnation-guard-receipt-v0.1.schema.json` | 停滞 receipt |
| `assurance/runtime-stagnation-guard-event-payload-v0.1.schema.json` | 停滞事件 payload |
| `assurance/orientation-stagnation-integration-fixture-v0.1.schema.json` | 集成夹具 |
| `assurance/orientation-stagnation-integration-receipt-v0.1.schema.json` | 集成 receipt |
| `assurance/orientation-stagnation-journal-receipt-v0.1.schema.json` | 日志 receipt |
| `assurance/orientation-checkpoint-event-payload-v0.1.schema.json` | 检查点事件 payload |

### 测试文件
| 文件 | 测试数 | 覆盖范围 |
|---|---|---|
| `assurance/tests/test_orientation_runtime_guard.py` | 9 | 检查点内容 (4) + 停滞检测 (5) |
| `assurance/tests/test_orientation_runtime_integration.py` | 6 | 完整夹具运行 + 验证、non-repeated 继续、retry 耗尽 handoff、message_block 篡改、receipt 篡改、非空输出根拒绝 |
| `assurance/tests/test_orientation_runtime_journal.py` | 5 | restart 投影 (run_invalidated)、continue 投影 (run_finished)、事件排序篡改、payload 篡改、非空日志根拒绝 |

**总计：20 个测试**

## 测试覆盖

### Orientation Checkpoint（4 个测试）
- 检查点使用中性提示块（含 `[ORIENTATION_CHECKPOINT v0.1]`，不含"是否正确"等诱导性语句）
- 响应接受仅定位字段（orientation_summary 等 4 个 allowed fields）
- 响应拒绝 counterexample 或 claim 字段
- trigger_step 必须为非负整数

### Runtime Stagnation Guard（5 个测试）
- 高连续重复（11 次相同输出）触发 `restart_requested`
- retry budget 耗尽（retry_count >= retry_budget）触发 `handoff_required`
- 正常多样化输出返回 `continue`
- N-gram 重复（重复 token 模式）触发 `restart_requested`
- 非法 retry budget（负数）被拒绝

### Integration Fixture（6 个测试）
- 完整夹具运行 + 重建验证
- 无重复的输出（正常继续，无 restart）
- retry budget 耗尽（handoff）
- Verifier 检测 orientation message_block 篡改
- Verifier 检测 integration receipt 篡改
- 非空输出根被拒绝

### Runtime Journal（5 个测试）
- Stagnation 触发时的 restart 投影（terminal event = run_invalidated）
- 无 stagnation 时的 continue 投影（terminal event = run_finished）
- 事件排序篡改（orientation 在 stagnation 之后）被检测
- Payload 篡改（`asks_model_if_stuck` 改为 true）被检测
- 非空日志根被拒绝

## 与 Tool Availability Gate 审计文档的关系

`TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` 记录了在本 Gate 模块中实装的以下扩展（详见该文档 §"Orientation Checkpoint 扩展" 和 §"Integration 扩展"）：

- `orientation-checkpoint-v0.1.schema.json` 的 `message_block` 正则放宽、字段枚举扩展、`tool_availability_sha256` 新增
- `orientation-checkpoint-verification-v0.1.schema.json` 的字段枚举扩展
- 三个新增函数：`build_tool_availability_infused_orientation_block`、`build_tool_availability_infused_orientation_checkpoint`、`evaluate_tool_belief_stagnation`
- Integration fixture 的条件性 tool availability 支持
- Journal 事件序列的可选 `tool_availability_check` 和 `tool_belief_stagnation` 事件

本审计文档覆盖 Orientation Runtime Guard 的**独立核心功能**（方向检查点、内容停滞检测、集成夹具、日志投影），Tool Availability Gate 审计文档覆盖其**与本 Gate 的交叉功能**。两文档互补，共同构成完整的审计覆盖。

## 边界与限制

- **公开输出分析**：停滞检测仅基于已发出的公开文本，不读取隐藏思维链。这意味着模型可能在进行有意义的深度推理时被误判为"重复"（如果公开输出的摘要部分确实重复）
- **语言检测的局限性**：中文方向检查点使用固定模板，不支持其他语言；工具信念检测的正则模式覆盖中英文但不覆盖所有可能的表达变体
- **子串匹配的假阴性**：`_extract_tool_names_from_text` 使用简单子串匹配，若模型使用工具的别名、缩写或描述性名称（而非声明名称），可能不被检测到
- **Journal 的事件顺序固定**：事件序列（run_preflight → tool_availability_check → run_started → orientation_checkpoint → runtime_stagnation_guard → tool_belief_stagnation → terminal）假定特定的 runner 架构。真实 runner 可能需要额外或重新排序的事件
- **Manifest 协议 digest**：Run manifest 中的协议文档 SHA256 在文档更新后会变化，导致历史上生成的 journal 无法重建验证——这是预期行为（版本绑定）而非缺陷
- **当前仍为 offline/fake 模式**：不调用真实模型、不发起网络请求。所有公共输出来自 fixture 而非真实 agent 运行

## 下一步

后续适合接入：
- 真实 runner 的事件钩子——将 Journal 的事件投影升级为真实 runner 的 JSONL 输出
- 多语言方向检查点模板（当前仅中文）
- N-gram 停滞检测的参数自适应（根据任务类型动态调整阈值）
- 工具信念检测的语义级升级（从子串匹配到工具名规范化 + 别名注册表）
- 将 restart_packet 的 4 个保留状态项与 runner 的实际状态恢复机制对接
- 增加进度标记（progress markers）的自动提取（当前需外部提供）
