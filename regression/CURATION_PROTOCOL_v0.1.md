# 案例发现、归类与反惯性协议 v0.1

状态：设计冻结候选；不包含分类器实现。

## 1. 目标

自动化用于发现和整理候选，不用于自动制造项目事实。案例库的价值是复现“应当检查什么”，而不是向 Agent 灌输“遇到相似表面就回答什么”。

## 2. 两阶段流水线

### 阶段 A：候选发现

规则或模型可以扫描 R/MAP/INDEX 中的下列信号：

- 撤回、修正、无效、重复、不是独立证据；
- label/flag 与实际执行不一致；
- feature order、metric direction、schema、NaN、缺行；
- 首次发现、普适、机制、因果、证明等强断言；
- seed、数据、训练链、评估集或 pipeline 同时变化；
- 文档状态与 manifest/result summary 不一致。

候选输出只允许包含：source span、命中规则、建议 taxonomy、建议 reason code、置信度和待核问题。候选状态固定为 `machine_proposed`，不得进入 evaluation corpus。

### 阶段 B：人工规范化

规范化者必须：

1. 阅读完整纠错上下文，不只读取命中句；
2. 区分当时可见事实、后验裁决和当前维护者注释；
3. 把错误压缩为可迁移的检查问题；
4. 写出至少一个允许 claim 和一个禁止 claim；
5. 指出什么新事实会让历史裁决不再适用；
6. 为高相似、高影响模式编写结论相反的 countercase；
7. 通过 reason-code、路径、行号与枚举交叉校验后，才可标为 `curated_initial`。

结构校验以 [`case-corpus-v0.1.schema.json`](case-corpus-v0.1.schema.json) 为准；schema 不能表达的跨文件、跨案例约束必须由 validator 单独执行。

## 3. 分类优先级

每案只有一个 `primary`，它表示最早足以阻止错误传播的边界，而不是文档里出现次数最多的主题：

1. routing：查错来源或跳过 prior-existence；
2. configuration/execution：实际做的不是声称做的；
3. artifact：产物本体或字段映射不合格；
4. provenance/coverage/independence/causal_inference：产物不能承担所赋予的证据角色；
5. semantic_mapping/claim_strength：证据被翻译成过强或错误的科学语言；
6. state_sync：正确结论没有同步到权威状态入口。

`secondary` 用于记录后续传播，不得用多个 primary 回避裁决。

## 4. 防案例泄漏

- development/teaching 可被检索，但必须晚于 `ReasoningPrecommitment`；
- evaluation/holdout 的 scenario 与 oracle 分离存储，运行时只暴露 scenario；
- 与当前任务同源或由同一次讨论改写的案例不得计作独立评测；
- oracle 泄漏后，该次评分作废，不能用“模型本来也会答对”补救；
- 案例检索记录必须同时写 similarities、disanalogies 和 conclusion-changing facts。

### 4.1 Fixture 边界

- fixture manifest 必须通过 [`fixture-v0.1.schema.json`](fixture-v0.1.schema.json)；
- scenario-visible 文件逐项记录相对路径、media type、role 和 SHA256；
- manifest 与 evidence 文件不得包含 expected gate、allowed/forbidden claim 或其他 oracle 字段；
- `oracle_fields_present: false` 只证明没有结构化 oracle 字段，不能证明自然语言不泄露答案；
- `synthetic_fixture_grounded` 只表示合成事实已变成可机械读取的证据包，不提升为历史事实或 raw-artifact-grounded；
- historical excerpt/raw snapshot 必须另记来源、许可、裁剪规则和原文件 digest，不能由 synthetic fixture 冒充；
- 含历史最终裁决、纠正摘要或撤回结论的 excerpt 必须标为 `reviewer_only`，只用于核查 curation/source fidelity；即使 manifest 不含 oracle 字段，也不得作为 scenario fixture；
- `scenario.fixture_manifest` 只能引用 `scenario_only`；历史 reviewer fixture 只能由 `curation_fixture_manifest` 引用，导出器必须机械拒绝把后者复制到运行目录；
- fixture hash 或内容变化必须提升 corpus revision，并使旧评测结果保持在旧 revision。

## 5. 防思维惯性

每个高频错误簇至少维护一个 countercase。countercase 应当保持表面特征相似，但加入能改变结论的决定性证据，例如：

- 旧案例是伪独立，新案例提供独立 producer 与 lineage；
- 旧案例是非配对比较，新案例的 manifest 证明 only-one-factor delta；
- 旧案例只有桥接假说，新案例有预注册的判别性干预；
- 旧案例样本不足，新案例的 claim 与代表性抽样范围严格同构。

countercase 必须显式归入一种 contrast type：

- `permission_reversal`：充分证据使历史 block/defer 变为 pass/warn；
- `root_cause_contrast`：仍需阻断，但根因和修复路径不同；
- `mode_boundary`：事实不变，discussion/guarded/strict 风险边界改变决定。

只统计 countercase 数量不构成覆盖证明。当前规范覆盖状态以
[`coverage-matrix-v0.1.yaml`](coverage-matrix-v0.1.yaml) 为准，并同时报告 detection、permission reversal 和 root-cause coverage。

回归分数不能只奖励“发现风险”。至少分别统计：

- detection：是否发现真正适用的 gate；
- calibration：是否选择 pass/warn/defer/block 的正确强度；
- discrimination：是否能在 countercase 中放行有充分证据的结论；
- source fidelity：是否以当前来源而非历史答案作裁决；
- claim discipline：是否把结论限制在有效证据范围内。

## 6. 数据版本与审计

- source span 或 oracle 变化必须产生 corpus revision；
- 删除案例采用 tombstone，不复用 case ID；
- taxonomy 和 reason-code 变化必须记录迁移理由；
- 自动分类器版本、规则命中和人工覆写都进入 append-only curation journal；
- 首批语料为 development/challenge，尚不能宣称是独立性能评测集。
- case 数量只作描述；共享来源、纠错链或 gate mechanism 的案例按 cluster 处理，不得伪装为独立 N。
- evaluation/holdout 的创建与污染处置遵循 [`../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`](../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md)；现有案例不得通过改分区字段获得“未见”身份。
