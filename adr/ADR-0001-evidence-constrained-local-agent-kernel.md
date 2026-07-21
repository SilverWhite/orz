# ADR-0001：构建证据约束型本地 Agent 内核

- 状态：Proposed
- 日期：2026-07-17
- 决策范围：Agent CLI 的产品边界、控制面、案例库、反捷径与思维独立性
- 实现状态：development-only contract probe 已开始；生产内核与模型 runner 未开始

## 1. 背景

当前项目已经形成了大量可复用的研究纪律：claim registry、INDEX→MAP→原始证据路由、任务契约、脚本与结果 validator、纠正链和撤回链。与此同时，历史讨论记录了大量真实错误，包括隐式默认值扩大实验面、冻结语义错误、数据源误判、重复 checkpoint 被误计为独立样本、覆盖缺口、指标方向颠倒和进程退出状态误判。

这些内容适合形成回归案例库，但直接把全部历史案例注入 Agent 上下文会产生新的风险：

1. Agent 可能把“最相似历史案例”当作当前任务的答案。
2. 项目已有叙事可能形成锚定和确认偏误。
3. 为避免历史错误，Agent 可能机械选择保守方案或拒绝新假说。
4. 回归得分可能被提示词泄漏、轮次名和文件名记忆污染。
5. “学会案例”可能替代对当前代码、数据和产物的独立核查。

因此，案例库必须同时承担错误回避和反思维惯性的职责。

## 2. 决策

构建一个本地优先、模型可替换、证据约束的 Agent 内核。内核拥有状态推进权；模型只产生提案。历史案例库作为独立的审计、检索和回归子系统，不作为会话开始时的默认答案上下文。

产品的核心目标是：

> 让任意接入的 AI 能够快速恢复工作面，同时使动作、产物、证据和 claim 的错误可发现、可阻断、可回放、可纠正。

## 3. 核心不变量

### INV-001：模型输出不是状态事实

模型可以提出计划、工具调用、解释和 claim candidate，但不能直接把任务、动作、证据或 claim 标记为完成、有效或已登记。

### INV-002：任务、动作和证据使用独立状态机

工具执行成功不推出产物完整；产物通过机械验证不推出来源正确；来源正确不推出证据独立；证据存在不推出机制结论。

### INV-003：当前工作区优先

INDEX、MAP、R、JSON、log、code 和当前 manifest 优先于历史摘要、模型记忆、案例检索结果和用户记忆。案例只能提供检查问题，不能替代当前来源。

### INV-004：claim 强度受最弱证据维度约束

claim 的允许强度不得超过来源、覆盖、独立性、对照、统计和事实—指标映射中的最弱状态。

### INV-005：工具流恰有一个终止事件

每个 action 可以产生零个或多个 progress event，但必须产生且只能产生一个 terminal event。退出码、摘要完整性和产物验证分别记录，不互相替代。

### INV-006：正式状态变化可回放

任务契约、动作提案、门禁决定、工具事件、产物 hash、证据接纳和 claim 变更必须进入 append-only journal。派生视图可以重建，历史事件不得静默覆盖。

### INV-007：历史案例不能先于独立初判暴露

默认流程为 `blind assessment → reasoning precommitment → case retrieval → contrastive review → final decision`。除教学模式外，Agent 在提交初判前不能看到案例 oracle、历史最终结论或标准修复。

### INV-008：案例相似性不构成因果证据

检索到相似案例后，Agent 必须分别记录相似处、不相似处、当前任务新增证据和可能的反例。若无法写出不相似处，检索结果只能作为偏误风险，不能作为决策依据。

### INV-009：回归库必须包含反惯性案例

每个高频错误模式至少配一个 surface-similar countercase：表面症状相似，但正确根因或门禁结论不同。系统不能因见到某个关键词就复用历史裁决。

### INV-010：自动分类只能提出标签

规则引擎或模型可以生成分类建议、证据片段和置信度；canonical 标签必须由主 Agent 或人工审阅确认。分类器不得改写案例 oracle。

## 4. 逻辑架构

| 模块 | 职责 | 不得承担 |
|---|---|---|
| `protocol` | Task/Event/Artifact/Evidence/Claim/Gate 数据契约 | 科学裁决 |
| `kernel` | 状态机、事务边界、恢复与取消 | 模型推理 |
| `agent-profile` | 模型、工具、权限、项目策略组合 | 会话可变状态 |
| `context-router` | INDEX/MAP/R/raw 证据路由和 files-read ledger | 把 INDEX 当证明 |
| `tool-runtime` | 文件、shell、Git、Python 工具执行和事件流 | 宣布证据成立 |
| `gate-engine` | 权限、配置、来源、覆盖、独立性和 claim 门禁 | 用正则替代科学审查 |
| `evidence-ledger` | hash、来源、覆盖、独立性、限制、反证 | 生成未经核查的结论 |
| `case-library` | 案例归类、检索、隔离、评分和反惯性变体 | 在 blind pass 前泄漏答案 |
| `journal/replay` | append-only 事件与派生视图重建 | 静默覆盖历史 |
| `adapters` | 模型、Windows 进程、Python validator 等适配 | 控制内核状态机 |

## 5. 案例库作为一等子系统

### 5.1 案例不是“错误故事”

每个案例必须至少包含：

- 当前 Agent 当时可见的输入；
- 被选择的错误捷径；
- 后续补充的反证；
- 错误影响到的状态层；
- 应触发的 gate 和稳定 reason code；
- 允许结论、禁止结论和降级边界；
- 原始来源路径与行号；
- 是否为真实历史案例、合成 countercase 或 metamorphic 变体。

### 5.2 多标签分类

案例采用多标签而非单一目录。第一版 taxonomy：

- `routing`
- `configuration`
- `execution`
- `artifact`
- `provenance`
- `coverage`
- `independence`
- `causal_inference`
- `semantic_mapping`
- `claim_strength`
- `state_sync`
- `safety`

偏误标签与错误类别分开记录：

- `confirmation_bias`
- `anchoring`
- `availability_bias`
- `surface_analogy`
- `automation_bias`
- `narrative_completion`
- `premature_causal_attribution`
- `project_inertia`

捷径标签描述“系统当时偷了什么懒”，例如：

- `trust_label_over_execution`
- `trust_cli_omission_as_absence`
- `trust_exit_code_over_artifact`
- `treat_run_as_independent_evidence`
- `reuse_cross_experiment_numbers`
- `fill_missing_rows`
- `promote_proxy_to_mechanism`
- `reuse_nearest_precedent`
- `overwrite_history`

### 5.3 自动归类流程

1. 确定性扫描提取来源、轮次、纠正词、撤回词、参数名、文件名和已有 claim ID。
2. 分类器提出多标签、错误阶段、证据片段和候选重复案例。
3. 主 Agent 对照原文确认 canonical 标签。
4. 生成 gate fingerprint，用于聚类但不合并来源不同的案例。
5. 无法确定的标签保留为 `classification_status: proposed`，不得伪装成事实。

自动归类的目标是减少整理成本，而不是让模型决定历史真相。

## 6. 反捷径与思维独立性机制

### 6.1 Blind-first

在检索历史案例前，Agent 必须写入 `ReasoningPrecommitment`：

- 当前最小事实集；
- 至少两个候选解释，若确实只有一个则说明排除过程；
- 需要寻找的反证；
- 当前不确定项；
- 哪些结论现在禁止升级；
- 建议读取的当前来源。

这份记录进入 journal，案例检索后不得覆盖，只能追加修订理由。

### 6.2 Contrastive retrieval

案例检索不只返回相似度，还必须返回：

- `similarities`
- `disanalogies`
- `missing_current_evidence`
- `countercase_ids`
- `retrieval_influence`

如果 Agent 只写相似处而没有不相似处，`BIAS-SURFACE-001` 至少产生 WARN；正式 claim 模式下可 DEFER。

### 6.3 Oracle 隔离

案例分区：

- `teaching`：完成后可展示完整解释；
- `development`：开发时可见 oracle；
- `evaluation`：运行时隐藏 oracle；
- `holdout`：不参与检索，只用于版本验收；
- `challenge`：专门放置 surface-similar countercase。

模型提示中不得包含 evaluation/holdout 的 `expected_*`、`allowed_claims` 或 `forbidden_claims`。

### 6.4 反项目惯性

每次 case-assisted review 都必须回答：

1. 当前证据是否支持历史解释，还是只有表面词汇相似？
2. 哪条新证据会推翻当前项目的惯用解释？
3. 是否存在项目过去未遇到的新类别？
4. 若用户期待相反结论，当前证据是否仍会得到同一判断？
5. 不检索案例时，初始判断与最终判断有何差异，差异来自哪条来源？

### 6.5 评价指标

不能只统计“是否复现历史答案”。至少同时统计：

- gate detection recall；
- false positive rate；
- unsupported claim rate；
- correct defer rate；
- countercase discrimination；
- source citation completeness；
- oracle leakage rate；
- initial/final reasoning delta 的来源可解释性。

## 7. Gate 等级

- `discussion`：记录风险，不阻断普通讨论或假说形成。
- `guarded`：默认模式；破坏性动作、隐式高风险 sweep、来源/覆盖/独立性红线和 claim promotion 可阻断或延期。
- `strict`：正式长跑、结果登记和 MAP/INDEX claim promotion。

现有 hook/validator 的 observation-first 行为应保留，但其输出只作为 gate input；新的内核不能把 hook 命中等同于科学失败。

逐项默认决策与 bypass 边界已在
[`../protocol/gate-matrix-v0.1.yaml`](../protocol/gate-matrix-v0.1.yaml) 中冻结为设计候选；reason code 的机器可读定义见
[`../protocol/reason-codes-v0.1.yaml`](../protocol/reason-codes-v0.1.yaml)。

## 8. Rust 与 Python 边界

语言选择按职责决定，不进行目的不明的全量迁移。

Rust 适合候选控制面：

- 状态机和类型化协议；
- 长生命周期本地进程；
- 并发工具事件流；
- journal 和恢复；
- 权限、工作区和进程控制；
- 单二进制 CLI 分发。

Python 保留科学面：

- PyTorch/NumPy 实验和分析；
- 当前脚本与结果 validator；
- 快速诊断和领域规则；
- 数据与 checkpoint 审计。

初期通过版本化 JSONL/stdin/stdout 连接，不采用 Python FFI。协议和回归 corpus 在语言选择前冻结；若 Python 原型更适合验证状态机，可先做可丢弃原型。

## 9. D 与 Grok Build 的使用方式

### D

只进行能力级迁移。幂等服务、本地动作扫描、双重确认、异常锁、共享 validator、审查管线和 sandbox safety 是候选；其行为必须在读取源码后重新裁决。Cloud Run、Cloudflare、社交平台、人格和旧产品记忆不进入 V0。

2026-07-21 已对 Drive 存档一的 state/brain/memory/tools/LLM/MCP/scheduled/schema 与存档零的
设定、DeepSeek prompt、安全阀和 settings 做定向源码审计。裁决见
[`../architecture/D_SALVAGE_MATRIX_v0.1.md`](../architecture/D_SALVAGE_MATRIX_v0.1.md)。结果不是迁入 D runtime：
只保留事实日志优先、脑/身分离和危险动作确认的设计方向；默认值伪装读取失败、prompt-only 约束、
任意远程 Python、静默删除 history、raw thought 日志和明文凭据均拒绝。

### Grok Build

借鉴 Agent definition/session 分离、actor-owned state、类型化工具事件、workspace/permission/sandbox 分层和统一构建路径。V0 不复制其包数量、完整 TUI 或通用产品面。

2026-07-19 以官方 `xai-org/grok-build` `SOURCE_REV=f9736c7b86f8e1c0e99e20ebbbd1195cd0c147e3` 刷新后，通用结构进一步确认：composition root、pager、shell runtime、tools 和 workspace 分层，另有 headless JSON、ACP、session journal、hooks、permissions 与 sandbox。适配裁决见 [`../architecture/GROK_BUILD_ADAPTATION_v0.1.md`](../architecture/GROK_BUILD_ADAPTATION_v0.1.md)。Grok 的 sandbox-off 与 hook fail-open 默认值不进入本项目 evaluation/evidence hard gate。

## 10. 后果

### 正面

- 历史纠错可以变成可执行回归而非散落经验。
- 模型可以替换，工作纪律保留在协议和 gate 中。
- 错误能够定位到配置、动作、产物、证据或 claim 层。
- 案例检索不会天然覆盖独立推理。
- Rust/Python 边界由职责而非品牌或潮流决定。

### 代价

- blind-first 增加少量会话延迟。
- 案例需要人工确认 canonical 标签和 oracle。
- journal、schema 版本和来源 hash 会增加工程复杂度。
- 过严 gate 可能抑制探索，因此必须区分普通探索与 claim promotion。
- 现有历史文档并不都具备同等强度的原始产物来源，部分案例只能先标为 discussion-grounded。

## 11. 非目标

- V0 不做云端服务。
- V0 不做多 Agent 自治组织。
- V0 不让案例库训练项目专属“正确观点”。
- V0 不自动把普通探索写入 MAP/INDEX。
- V0 不保证消灭科学错误；它保证错误更难静默通过，并留下可审计路径。

## 12. 进入实现前的验收条件

1. 协议 schema 能表达任务、动作、证据、claim、precommitment 和 case retrieval。
2. 至少 15 个真实案例和 5 个 countercase 有明确 oracle（当前 seed corpus 为 30 + 10，数量门槛已满足，来源强度仍需分层提升）。
3. 每个高频错误类别至少有一个 false-positive challenge。
4. 可从 journal 重放一个历史错误的决策路径。
5. 两种不同模型在不暴露 oracle 的情况下运行同一 corpus。
6. 能统计 unsupported claim 和 countercase discrimination，而不只统计答案一致率。
7. D 的 in-scope 候选文件完成源码级 salvage matrix；核心 cloud-brain 已审计，local-body 正文因 DriveFS 流式读取失败保持 unchecked，但依 ADR-0002 不再是当前实现前置条件。
8. Windows 进程终止、空退出码和摘要完整性完成独立探针。
9. evaluation/holdout 使用独立密封存储、scenario/oracle 分离 digest 和污染日志；不能从已参与设计的 seed corpus 晋升。
10. 双人独立盲审 baseline 完成后再冻结通过阈值，并保留原始分歧与 adjudication 记录。
11. ScenarioExporter、LeakScanner 与 EvaluationRunner 契约通过 schema/example 自证，并用至少一个 development fixture 做真实 export/replay smoke。

2026-07-21 覆盖审计状态：

- 条件 2 的数量门槛满足，但这只是 seed corpus 数量，不是独立评测样本数；
- 条件 3 在 development/challenge 层满足：10 个错误簇均有 challenge，7 个高频簇均有 permission-reversal challenge；但这些 challenge 已参与设计，不能转作 evaluation/holdout；
- 条件 6 已有评分协议与结果 schema，但尚未用人类基线和隔离 evaluation split 校准，因此执行验收仍未满足；
- 条件 9–10 已有协议与 partition schema，但没有真实密封分区、baseline 或阈值，因此只完成设计定义；
- 条件 11 的机械部分已有五个 runtime schema、development-only ScenarioExporter/LeakScanner、无模型 hash-chain journal 和真实 fixture export/replay smoke；模型 adapter、tool broker、隔离 runner 与评测执行仍未实现，因此该条件只算部分满足；
- 覆盖来源：[`../regression/coverage-matrix-v0.1.yaml`](../regression/coverage-matrix-v0.1.yaml)；评分来源：[`../evaluation/SCORING_PROTOCOL_v0.1.md`](../evaluation/SCORING_PROTOCOL_v0.1.md)。

## 13. 当前依据与未核风险

工作区依据：

- `LIF_CURRENT_INDEX.md`（旧研究工作区；未随独立仓库迁移）
- `fep_env_research.md`（旧研究工作区；未随独立仓库迁移）
- `self_check_protocol.md`（旧研究工作区；未随独立仓库迁移）
- `fep_env_research/EXPERIMENT_PROGRESS_MAP6_2026-06-22.md`（旧研究工作区；未随独立仓库迁移）
- 首批案例中列出的 R 文档
- [`../regression/CURATION_PROTOCOL_v0.1.md`](../regression/CURATION_PROTOCOL_v0.1.md)
- `.github/skills/fep-script-validation/references/script_contract.yaml`（旧研究工作区；未随独立仓库迁移）

以上旧工作区路径在独立仓库中只保留为 provenance locator；未重新挂载并读取前，不得把它们视为本轮已核来源。

未核风险：

- 本 ADR 的模块边界和 blind-first 流程是设计提案，尚未通过可运行原型验证。
- 自动分类器的准确率、成本和误分类模式未知。
- D 核心候选机制已有源码级 salvage matrix；local-body 实现仍是明确未核项，不能由 cloud-brain 注释代替，也不进入当前依赖。
- 当前历史案例可能对项目高频错误过采样，需使用外部通用软件/科学案例和合成 countercase 平衡。
