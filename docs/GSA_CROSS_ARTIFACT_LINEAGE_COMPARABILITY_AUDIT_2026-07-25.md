# GSA-LINEAGE-001 跨 Artifact Lineage/Comparability 审计（2026-07-25）

## 裁决

通用只读审查已接通首个跨 artifact lineage/comparability 纵向切片。直接比较可以分别引用 left/right
artifact 与各自 JSON Pointer；在数值关系求值前，`GSV_CROSS_ARTIFACT_COMPARABILITY` 必须核对两侧
声明的研究语境、producer/source lineage 和唯一变化条件。

本轮证明的是声明与多文件记录之间的机械一致性，不证明这些声明真实、实验独立或科学上可比。

## 非 LIF 正例迁移

指数衰减 fixture 从一个含两项 run 的 artifact 拆为两个独立、分别摘要绑定的 artifact：

- `result-dt-0.1.json`；
- `result-dt-0.05.json`。

两者按 `GSAS_NUMERICAL_SINGLE_RUN_0_1` 验证，并分别记录：

- task、protocol、metric、population、unit；
- comparison family 与唯一 varied dimension；
- producer action、source IDs、transformation ID；
- condition dimension/value 及指回 artifact 实际字段的 JSON Pointer。

direct-comparison evidence 明确把 left pointer 绑定 `dt=0.05` artifact，把 right pointer 绑定
`dt=0.1` artifact。最终报告把比较规范化为 `left_artifact_id/right_artifact_id`；旧的单 artifact
比较形状仍可读取，但 cross-artifact validator 对它返回 `not_applicable`。

## 机械门禁

跨 artifact validator 要求：

1. task、protocol、metric、population、unit、comparison family 和 varied dimension 一致；
2. method、equation、initial value、final time 和 exact reference 一致；
3. 两侧 lineage producer 同时匹配各自 artifact record 和 action manifest；
4. 两侧 lineage source 集合与 action manifest 完全一致；
5. transformation ID 一致；
6. condition dimension 与 varied dimension 一致；
7. condition value 必须不同，并通过各自 `value_pointer` 与 artifact 实值对账。

不满足时使用既有 `EVD-COMPARABILITY-001`，总体裁决 `defer`。它不是输入损坏，因此不与 digest/schema
失败的 `block` 混为一类。

仓库检查器不调用 reviewer/validator，而是独立重建 checked-in fixture 的 context、lineage、condition
pointer 和 `lt` 比较，防止正例只靠实现自身宣称 PASS。

## 正反例

- 全部 context/lineage 一致，仅 step size 不同：comparability PASS，原窄 observation 可 `allow`；
- unit 不同：`defer`；
- condition 声明值与 pointer 指向的实际 step size 不同：`defer`；
- lineage source 少于 action manifest：`defer`；
- 未登记 artifact schema、schema 结构错误、摘要错误或非有限数仍在更早阶段 fail closed；
- mechanism/generality 仍不会因 comparability PASS 获得升级。

该正反对形成 permission reversal：不是“跨文件比较一律拒绝”，而是只允许记录上确实可比的窄比较。

## 独立性边界

两个 artifact 由同一 action、相同 source 与相同 transformation 产生。拆文件不会增加独立 N，也不是
外部复现。`independence_key` 仍只代表当前 evidence 单元；本实现没有把两个文件计为两份独立科学证据。

## 保留缺口

- context 和 lineage 仍由 producer 声明，尚未与代码 commit、环境锁、输入数据 digest 和事件 journal
  做不可绕过的联合证明；
- 没有表格 join key、单位换算、population harmonization、metric semantic mapping 或时间窗口 validator；
- 没有多 action、多 producer、独立复现或随机/数据独立性核算；
- 没有 derived-artifact transformation DAG、cycle detection 或双向 claim lineage；
- comparability PASS 不证明设计无混淆，也不支持 causal/mechanism/generality；
- fixture 仍是 development/conformance 资料，不是 evaluation 或 holdout。

提交前机械结果：88 schemas、6 validators、3 artifact schemas、1 个独立重建的 cross-artifact
comparison fixture、prototype 50 tests、Grok integration 44 tests、runtime 6 tests、assurance
48 tests，共 148 tests，repository error count 为 0。该计数不增加独立科学 N。

下一步应构建 disposable reproduction，但必须先冻结 reproduction manifest、输入快照、允许的固定动作、
输出 schema 和“replay 不增加独立证据”的报告边界。
