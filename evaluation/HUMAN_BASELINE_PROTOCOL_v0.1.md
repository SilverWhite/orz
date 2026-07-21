# 双人盲审 Human Baseline 协议 v0.1

状态：设计草案；尚未执行 baseline。

## 1. 目标

Human baseline 用于回答两个问题：任务本身是否可判，以及 Agent 的错误率相对受相同证据约束的人类审阅者处于什么位置。它不是用两个人投票制造“真相”，oracle 仍需独立维护。

## 2. 参与者

- reviewer A 与 reviewer B 独立工作；至少熟悉研究证据纪律，但不得接触该分区的来源历史或 oracle。
- 两人不能参与被比较系统的逐案提示词调试。
- adjudicator C 只处理预先标记的分歧，不取代 A/B 的原始记录。
- 若人员重叠不可避免，必须披露重叠角色并把结论标为受限 baseline。

## 3. 盲法

每位 reviewer 获得与 tested Agent 相同的：

- scenario export；
- 可用工具与证据访问边界；
- 时间/步骤预算；
- 输出 schema 与 reason-code 定义。

reviewer 不得看到系统身份、另一位 reviewer 的答案、case family、countercase 配对、历史纠正摘录或 oracle。Agent 与人类输出在评分前统一匿名化和顺序随机化。

## 4. 预注册

运行前冻结并记录：

- 分区与 scenario digest；
- oracle 与评分协议 digest；
- reviewer 资格与角色冲突；
- 时间、工具和检索预算；
- 随机顺序种子；
- 缺失答案、超时和工具故障的处理；
- 主要指标、红线与分歧处理方式。

任何运行后规则变更都必须作为新版本重跑，不能静默改分。

## 5. 执行流程

1. reviewer 仅在 development 练习集上熟悉格式；练习答案不计分。
2. A/B 分别对冻结分区作答，不交流、不共享草稿。
3. 每个案例先提交 gate/state/claim 结论，再提交说明，避免由叙事反推标签。
4. runner 对 Agent 与人类输出统一做 schema 检查和匿名化。
5. A/B 原始答案分别对 oracle 评分；不先求共识。
6. C 只对预定义的语义歧义或 A/B 分歧作 adjudication，并记录理由。
7. 发布原始分歧、裁决后结果和不确定性；不只发布合并均值。

## 6. 携带效应控制

- 每位 reviewer 使用独立随机案例顺序。
- 明显配对案例不得相邻出现。
- 禁止在正式分区中提供逐案即时反馈。
- 多轮评审之间设置预注册的间隔，并记录之前暴露过的案例。
- 已看过 oracle 或历史纠正摘录的人不得再作为该案例的 blind reviewer。

## 7. 评分与一致性

沿用 `SCORING_PROTOCOL_v0.1.md` 的 cluster-macro 结构，至少分别报告：

- gate decision、state transition、reason-code、claim-boundary、remediation 的准确率；
- forbidden-claim 与 authority/red-line 事件数；
- cluster-macro 与每簇分布；
- A/B 原始一致率；
- 对类别输出使用 Cohen's kappa 或适用的加权 kappa；
- adjudication 比例；
- 缺失、超时和工具故障率；
- paired case 的结论翻转正确率。

样本量不足时报告原始计数和区间，不用单一总分掩盖不确定性。

## 8. 阈值校准

接受阈值只能在密封分区和 baseline 方案冻结后设定。建议顺序：

1. 先确认 A/B 对任务有可接受的一致性；
2. 再估计人类表现区间与案例难度；
3. 最后确定 Agent 的最低维度阈值和零容忍红线；
4. 把阈值应用到未用于校准的 holdout。

若人类一致性低，优先修订案例或 rubric，不能把含糊案例当作 Agent 失败。

## 9. 结果解释边界

- Agent 超过两位 reviewer 的平均分，不等于超过人类或具备科学判断能力。
- reviewer 与 Agent 同时失败，可能反映场景信息不足、rubric 不稳或共同偏误。
- 当前项目内部案例可用于 development，但不能单独证明跨表述、跨时期或跨项目泛化。
- baseline 结果必须与所用模型、提示词、工具、预算、分区 digest 绑定。

## 10. 当前状态

本文件只定义程序。当前没有招募 reviewer、没有执行盲审、没有产生 kappa，也没有据此设定通过阈值。
