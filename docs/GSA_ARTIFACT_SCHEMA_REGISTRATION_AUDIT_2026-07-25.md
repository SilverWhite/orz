# GSA-ARTIFACT-001 Artifact Schema Registration 审计（2026-07-25）

## 裁决

通用只读审查已不再接受“只要是 JSON 就可作为 artifact”的隐式合同。每个 artifact 必须声明
`artifact_schema_id`，该 ID 必须存在于版本化、SHA-256 绑定的
`general-science-artifact-registry-v0.1.json`，且登记的 artifact kind、media type 和 schema
必须一致。

本轮关闭的是首个 development schema-registration 路径，不是完整 artifact 生态或数据 lineage。

## 初始登记

| Schema ID | Artifact kind | 边界 |
|---|---|---|
| `GSAS_NUMERICAL_TIME_STEP_0_1` | `numerical_result` | 验证确定性时间步结果的 method、equation、initial/final reference 和至少两项 run；每项 run 必须含 step size/count、final value 和 absolute error |
| `GSAS_STATISTICAL_SUMMARY_BASE_0_1` | `statistical_summary` | 只验证版本化 JSON 容器；统计报告字段继续由 `GSV_STATISTICAL_REPORTING` 单独检查 |

统计 base schema 刻意不重复 sampling unit、N、effect、uncertainty 等规则，否则 schema 和 validator
会形成两个可漂移的权威来源。base schema PASS 加 reporting validator PASS 仍只表示合同完整，不表示统计正确。

## 不可弱化与独立复核

`artifact_registry.py` 要求：

- schema ID 和 schema filename 各自唯一；
- 两个基础 schema 不能缺失；
- schema 文件必须存在且是合法 Draft 2020-12 schema；
- 基础 schema 的 version、kind、media type、filename、`block` 决策和 `ART-SCHEMA-001` 映射不能改写；
- registry 可以增加新 ID，但不能通过改写旧 ID 静默改变既有 artifact 语义。

仓库检查器独立重建上述规则，并直接按 registry 验证 checked-in artifact。结构合法但重复 schema ID
的 registry fixture 必须被语义层拒绝。

## 审查结果绑定

最终只读报告新增：

- `artifact_registry_id`；
- `artifact_registry_sha256`；
- `GSV_ARTIFACT_REGISTERED_SCHEMA` 的逐 artifact 结果。

未知 schema ID、kind/media mismatch、目标 schema 缺失或 artifact 不符合登记 schema 都在证据比较前
fail closed。artifact registry 与 validator registry 分开摘要绑定，避免只记录 validator 代码而遗漏
其所消费的 schema 版本。

## 正反回归

- 当前非 LIF 数值 fixture 按登记 schema PASS；
- 未登记 schema ID 被拒绝；
- 删除必需的 `runs` 后，即使重新绑定 artifact digest 仍被 schema validator 拒绝；
- 重复 schema ID registry 被拒绝；
- 将基础 numerical schema 改标为 `other_json` 被不可弱化语义拒绝；
- statistical summary 使用独立 base schema，缺少 reporting 字段仍由统计 validator `defer`。

提交前机械结果：87 schemas、5 validators、2 registered artifact schemas、artifact/validator
registry 各 1 个结构合法但语义无效的负例、prototype 50 tests、Grok integration 44 tests、
runtime 6 tests、assurance 47 tests，共 147 tests，repository error count 为 0。

## 保留缺口

- 尚无 CSV/TSV、表格列类型、单位/量纲、图像、PDF、代码包或环境锁 schema；
- 没有 schema migration、兼容性矩阵、弃用/撤销或历史 registry 保留策略；
- 没有第三方 schema/validator 签名与隔离加载；
- producer action 当前只引用 artifact ID，尚未登记输入数据、变换步骤和派生产物的完整 lineage；
- schema 验证不证明数值由声明的代码、配置或数据产生；
- 尚未验证跨 artifact 的 population、metric、unit、method 和 producer lineage 是否可比。

下一步应实现跨 artifact lineage/comparability contract，再考虑 disposable reproduction。
