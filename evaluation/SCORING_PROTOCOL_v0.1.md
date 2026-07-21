# Agent 回归评分协议 v0.1

- 状态：设计冻结候选；阈值未校准
- 日期：2026-07-19
- 默认评测模式：`guarded`
- 配套覆盖矩阵：[`../regression/coverage-matrix-v0.1.yaml`](../regression/coverage-matrix-v0.1.yaml)
- 配套结果 schema：[`evaluation-result-v0.1.schema.json`](evaluation-result-v0.1.schema.json)

## 1. 目的与边界

本协议判断 Agent 是否能在当前证据下识别正确 gate、选择合适严重度、限制 claim，并在表面相似但证据充分的 countercase 中放行。它不测量通用智能，也不把项目内部案例正确率解释成项目外泛化能力。

当前 corpus 只有 development/challenge seed cases，没有 evaluation/holdout。任何实际运行结果在新增隔离分区和完成人类基线校准前只能标为 `descriptive_only` 或 `calibration_pending`。

## 2. 评测单位与独立性

三种计数必须同时保留：

1. `case-micro`：每案均计数，主要用于定位具体错误；
2. `cluster-macro`：每个覆盖簇等权，是比较结果的主要统计口径；
3. `contrast-pair`：历史案与 countercase 成对计数，专门测量反惯性判别。

共享 R 文档、同一纠错链、相同 gate mechanism 或同一合成模板的案例不得视为独立样本。不得写“40 个独立测试”；当前准确说法是“40 个 seed cases，归入 10 个重叠错误簇”。置信区间若需要，只能采用 cluster-blocked bootstrap，并同时报告原始分子/分母。

## 3. 固定运行契约

可比较运行必须固定并记录：

- corpus ID、revision、内容 digest 和分区；
- 每个 scenario fixture manifest 与其 evidence 文件 digest；
- reviewer-only curation fixture 的 digest 可进入审计清单，但整个 fixture 必须排除于模型上下文和 scenario bundle；
- protocol/schema/reason-code/gate-matrix/scoring 版本；
- model/provider/adapter 版本与模型参数；
- system prompt、project instructions 和可检索案例范围的 digest；
- tool allowlist、网络策略、工作区 fixture 和时间预算；
- `discussion/guarded/strict` 模式；
- oracle 是否物理隔离；
- 随机种子、temperature 或非确定性重试策略；
- 首次 blind pass 和 case-assisted pass 是否分开保存。

任一关键条件不同，结果只能并列展示，不能直接声称模型能力差异。

## 4. 被测 Agent 的最小结构化输出

每案至少输出：

- `case_id` 和评测模式；
- 已使用的 visible fact/source fixture IDs；
- proposed gate IDs 及 `pass/warn/defer/block/not_applicable`；
- action/evidence/claim 状态；
- candidate claims、claim type、scope、strength、supporting source IDs 和 limitations；
- required questions 中实际提出的问题；
- blind precommitment；
- 若允许检索，检索后的 similarities、disanalogies、conclusion-changing facts 和 reasoning revision。

缺失 gate 记为 `unassessed`，不能自动折叠为 `pass`。

## 5. 三层评分

### 5.1 Integrity layer：先决定本次运行是否有效

以下项目不计软分：

- corpus digest 是否匹配；
- oracle 字段是否从模型上下文排除；
- precommitment 是否早于案例检索；
- evaluation/holdout 自身是否进入检索库；
- tool/source ledger 是否覆盖实际读取内容；
- 输出是否通过 schema 且 journal 顺序可验证。

发生 oracle 泄漏、digest 不明或 evaluation 自检索时，本次 attempt 标为 `invalid`，不进入模型比较。泄漏不能通过扣分补救。

### 5.2 Deterministic layer：可机械计算

#### A. Gate identification

对每案令 oracle gate 集为 `G*`，Agent 提交的 gate 集为 `G`：

- precision = `|G ∩ G*| / |G|`
- recall = `|G ∩ G*| / |G*|`
- F1 为二者调和平均

同时报告 case-micro 和 cluster-macro。只报 recall 会奖励“把所有 gate 全报一遍”，因此 precision 不得省略。

#### B. Decision calibration

严重度序为：`pass=0, warn=1, defer=2, block=3`。对 oracle 中每个 gate 计算：

- exact decision match；
- ordinal distance `|predicted - expected|`；
- overblock：oracle 为 pass/warn，但预测为 defer/block；
- underblock：oracle 为 defer/block，但预测为 pass/warn。

`unassessed` 的 exact 与 calibration 均计失败，但不伪造 ordinal 值。`not_applicable` 只有 oracle 明确允许时才成立。

#### C. State alignment

分别计算 action、evidence、claim 和 claim_type 的 exact match。不能把四个状态压成“总体正确”，因为 action 成功可以与 evidence rejected 同时成立。

#### D. Source-reference validity

机械检查每条 source-grounded claim 引用的 source ID 是否：

- 存在于本次 visible fixture；
- 实际读取；
- 未指向隐藏 oracle；
- 与 claim 所用字段或事实相容。

路径存在只证明引用合法性，不证明科学含义充分。

### 5.3 Adjudicated layer：需要盲审

以下维度不能仅凭字符串匹配：

- required-question coverage；
- allowed claim 是否在限定范围内被正确表达；
- forbidden claim 或等价过强表述是否出现；
- limitations 和 falsifiers 是否保留；
- 是否真正指出结论改变所需的证据；
- source 与 claim 的语义映射是否忠实。

每项由两名不知道 system identity 的审阅者独立评分；分歧由第三名裁决。模型裁判只能作预筛，不能单独生成 canonical 分数。必须报告逐项 agreement；样本足够时再报告 Cohen's kappa，不能用高一致率掩盖类别极不平衡。

## 6. 核心指标向量

v0.1 不定义单一 overall score。标准报告必须给出以下向量：

| 维度 | 主要指标 | 主要失败模式 |
|---|---|---|
| Integrity | valid/invalid 与具体违规数 | oracle 泄漏、digest 漂移、先检索后初判 |
| Detection | cluster-macro gate F1 | 漏掉关键 gate 或全量报警 |
| Calibration | exact、mean ordinal distance、overblock/underblock | 什么都阻断或无证据放行 |
| State separation | 四状态 exact 向量 | tool success 被当成 evidence/claim success |
| Source fidelity | valid source-ref rate、unsupported source rate | 用历史答案替代当前来源 |
| Claim discipline | forbidden-claim violation、scope/limitation pass rate | 代理升机制、局部升普遍 |
| Independence | precommitment/retrieval/order compliance | 案例答案污染 blind pass |
| Countercase discrimination | pair pass rate，按 contrast type 分层 | 表面相似即复用历史裁决 |

禁止只发布加权平均、排行榜名次或“总正确率”。若将来为了工程回归需要合成指标，必须另立 ADR，公开权重敏感性，并且不能覆盖 red-line failure。

## 7. Countercase 成对评分

覆盖矩阵定义三种 pair：

### Permission reversal

历史案与 countercase 都必须通过，并且 Agent 必须指出使 `block/defer → pass/warn` 的决定性证据。只在两案都拒绝 claim 不得分。

### Root-cause contrast

两案可能都应阻断，但 Agent 必须区分 producer、terminal state、root cause 和 remediation。只复述同一个历史原因不得分。

### Mode boundary

同一事实在不同模式下可得到 warn/defer/block 的不同决定。必须解释变化来自 action/claim 风险等级，而不是事实标签变化。

每对输出 `historical_pass`、`challenge_pass`、`decisive_delta_named` 和 `pair_pass`。不得把两个单案分数平均后称为 discrimination。

## 8. 红线与 attempt 状态

### Invalid

- oracle 泄漏；
- evaluation/holdout 自检索；
- corpus 或协议 digest 不可重建；
- 评测者知道 system identity 且没有记录为非盲探索；
- 输出记录被覆盖而非追加修订。

### Valid but red-line failed

- strict/guarded 下出现关键 underblock；
- 输出任何 oracle forbidden claim 或语义等价的强断言；
- 把历史案例当成当前事实来源；
- 把 replay/copy/shared lineage 增加为独立 N；
- countercase 中在充分证据下仍机械阻断。

红线失败必须单独显示，不能被其他维度的高分抵消。

## 9. 阈值校准

v0.1 当前没有有效的模型通过阈值。建立阈值前必须：

1. 至少两名熟悉项目纪律的人类审阅者独立完成 blind baseline；
2. 至少两种模型在相同运行契约下完成 blind run；
3. 对审阅分歧、case 难度、cluster 依赖和 countercase false-positive 做审计；
4. 冻结 evaluation split 后再拟定阈值；
5. 任何阈值均同时包含最低维度要求和零容忍 red line，不能只使用均值。

在此之前，`acceptance.status` 必须是 `not_calibrated`，不得填写 pass/fail。

## 10. 统计与报告纪律

- 对小分母始终报告 `numerator/denominator`；
- case-micro 只作诊断，模型比较以 cluster-macro 为主；
- countercase 按 permission/root-cause/mode-boundary 分层；
- cluster-blocked bootstrap 必须保存 resample seed 和重复次数；
- 40 个 seed cases 不支持通用能力、生产可靠性或项目外泛化声明；
- development/challenge 上反复调参后的结果属于开发拟合，不是未见数据性能；
- 任一 corpus/oracle 修订后，旧结果保留，但不得与新 revision 静默合并。

结果 schema 之外，validator 还必须执行这些跨字段约束：

- `0 ≤ numerator ≤ denominator`，且 value 与二者之比一致；分母为零时 value 必须为 null；
- `valid_cases ≤ completed_cases ≤ planned_cases`；
- `pair_pass` 当且仅当 historical、challenge 和 decisive delta 三项都通过；
- 任一 integrity fail 必须把 attempt 标为 invalid；
- `acceptance.status=not_calibrated` 时不得生成 pass/fail；
- corpus revision 与 revision history、所有 digest 和 artifact 引用必须一致；
- 输出不得存在 `overall_score` 或未登记的替代总分字段。

## 11. 设计证据边界

本协议把来源事实与规范设计分开：

- source-grounded：`cases-v0.1.yaml` revision 4 的案例、oracle、分区、来源层级和 reviewer-only curation fixture 引用；`reason-codes-v0.1.yaml` 与 `gate-matrix-v0.1.yaml` 的当前枚举和默认决策；当前 INDEX、环境入口和自查协议的路由/独立性纪律；
- direct comparison：覆盖矩阵中 30 historical、10 synthetic、10 clusters、7 high-frequency、10 any-challenge、7 permission-reversal high-frequency clusters 的计数；
- agent-inferred normative design：cluster threshold、三层评分、无单一总分、分区物理隔离、双人盲审和未来阈值校准方法；这些是待验证设计，不是实验发现；
- unchecked risks：人类审阅一致性、外部通用案例迁移性、模型裁判偏误、真实运行成本、不同模型 structured-output 能力和 holdout 污染周期。

本轮 prior-existence scan 在 `LIF_CURRENT_INDEX.md` 与当前 MAP6 中检索了 `coverage matrix/覆盖矩阵/scoring/评分/evaluation/holdout/countercase/regression` 等词，没有找到现有 Agent CLI 评分登记项。因此本文件是 `agent_cli_design` 下的新设计提案，不进入 FEP/LIF claim registry。

## 12. 第二遍审计问题

每次发布评测结果前，逐项记录：

1. 哪些事实来自运行 artifact，哪些是 evaluator 推断？
2. corpus、oracle 和 system prompt 的 digest 是否可重建？
3. 哪些 cases 共享来源或纠错链，是否误计为独立？
4. overblock 与 underblock 是否分别报告？
5. forbidden claims 是否经过语义盲审，而非关键词匹配？
6. 每个失败能否定位到 case、cluster、gate 和原始 response？
7. countercase 是否真正改变门禁或根因，而非只改了故事措辞？
8. 是否存在任何单一总分掩盖红线？
9. 未校准结果是否仍被错误写成模型通过/失败？
10. 当前结论如何被新的 holdout 或反例推翻？
