# GSA-VALIDATOR-001 ValidatorBridge 初始审计（2026-07-25）

## 裁决

`general-science` 已获得首个可执行、版本化、摘要绑定的 ValidatorBridge。它将设计、动作和 artifact
校验从 `general_science_review.py` 的单体逻辑中抽出，由
`general-science-validator-registry-v0.1.json` 登记，并把每个 validator 的适用性、目标、状态、
reason code 和细节写入只读审查结果。

本轮关闭的是 development 级初始桥接，不是完整科学 validator 体系。

## 登记的 validator

| Validator | 目标 | 结果语义 |
|---|---|---|
| `GSV_DESIGN_SCHEMA` | study design | 按版本化 schema 检查问题、假设、设计类型、对照、证伪条件、分析计划和 coverage 声明 |
| `GSV_ACTION_SCHEMA` | completed action manifest | 检查 action identity、completed、exactly-one terminal、source/artifact 引用形状和 review-time 安全声明 |
| `GSV_ARTIFACT_FINITE_JSON` | JSON artifact | 拒绝非 JSON 或非有限数；不把 `NaN/Infinity` 当作可用科学数值 |
| `GSV_STATISTICAL_REPORTING` | `statistical_summary` | 检查 sampling unit、N、effect estimate、不确定性、缺失数据、多重比较、分析角色和停止规则是否明确报告 |

注册表使用现有协议原因码：控制合同失败映射 `TASK-CONTRACT-001`，非有限 artifact 映射
`ART-FINITE-001`，统计报告不完整映射 `EVD-COVERAGE-001`。没有另建与协议竞争的原因码体系。

## 决策边界

- schema 或 finite validator 的 `block` 表示输入无法安全进入证据比较，审查 fail closed；
- statistical-reporting 缺失或不完整为 `defer`，因为“未报告”不等于结果必然为假；
- 非统计 artifact 对 statistical validator 返回 `not_applicable`，不能因没有 P 值或置信区间而误伤
  确定性数值结果；
- 完整性 PASS 只表示所需字段存在且满足窄机械关系，例如区间上下界有序且覆盖 effect estimate；
- PASS 不证明 sampling unit 选择正确、样本独立、模型假设成立、区间计算正确、功效充分或结论有效；
- validator 不能提升 causal/mechanism/generality；强主张仍由 claim gate 固定 defer。

## 注册表完整性

registry schema 限制 stage、implementation、schema filename、artifact kind、failure decision 和 reason
code。语义解析器另行要求：

- validator ID 唯一；
- 四个基础 validator 不能缺失；
- schema validator 只能用于 design/action，且目标 schema 必须真实存在；
- built-in artifact validator 必须声明非空 applicability，不能伪装成 schema validator。
- 四个基础 validator 的 stage、implementation、failure decision 和 reason code 固定为最小策略；
  registry 可以增加 validator，但不能把 finite/schema 的 `block` 弱化成 `defer`。

仓库检查器独立重建同类语义，并验证一个“结构合法、重复 ID、语义无效”的负例，避免只依赖执行器自证。

## 回归

通用科学审查测试新增：

1. 正例报告固定包含四项 validator result；
2. 非统计数值 artifact 对统计 validator 为 `not_applicable`；
3. 把 artifact 标成 statistical summary 但不报告统计要素时总体裁决为 `defer`；
4. 临时补齐基础报告字段后统计 validator PASS，但 claim 仍保持原窄 observation；
5. 重复 validator ID registry 被语义解析器拒绝；
6. finite 基础策略从 `block` 弱化为 `defer` 被语义解析器拒绝；
7. 原有摘要篡改、路径逃逸、非有限数、未知 bridge、机制过强、无支持和多 terminal 负例继续保留。

提交前机械结果：84 schemas、4 个登记 validator、1 个结构合法但语义无效的 registry 负例、
prototype 50 tests、Grok integration 44 tests、runtime 6 tests、assurance 46 tests，共 146 tests，
repository error count 为 0。该结果不构成统计正确性或科学有效性证据。

## 保留缺口

- 当前 statistical validator 是 reporting-completeness gate，不是统计分析器；
- 没有单位/量纲、表格列、图像、PDF、代码、环境锁或跨 artifact 变换 validator；
- 没有插件签名、第三方 validator 隔离、资源配额或不可信 validator 执行；
- 没有 validator 版本迁移、撤销、冲突解析或领域 profile 动态组合；
- design/action schema 仍是首版窄合同，尚未覆盖预注册偏离、协议修订、伦理、许可和数据 lineage；
- 正例仍是 development fixture，不是 evaluation、holdout、真实用户或科学有效性证据。

下一步可以在此桥上增加 `artifact schema registration + cross-artifact lineage/comparability`，随后再进入
disposable reproduction；不应直接跳到正式评分和 holdout。
