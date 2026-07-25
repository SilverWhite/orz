# ADR-0004：通用科学保障 profile 与 LIF 领域增量分层

- 状态：accepted
- 日期：2026-07-25
- 关联：ADR-0001、ADR-0003、`GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`

## 1. 背景

LIF 项目暴露出的来源先行、多文件回查、证据分层、独立性、反例搜索、机械验证和 claim
强度约束具有普遍科研意义；但 INDEX/MAP/R/self-check 路由、长期历史和专门术语属于 LIF
领域上下文。

若继续把全部科学保障能力挂在 `lif-research`：

- 通用科研能力会错误依赖 LIF 文件名和历史结构；
- LIF 内部任务会同时参与能力设计与能力测试，产生污染；
- 其他科研领域无法只复用共同不变量；
- profile 的继承与能力边界无法机械验证。

## 2. 决策

profile 层次冻结为：

```text
General Assurance Kernel
        |
general-science
  - SourceRouter
  - EvidenceKernel
  - ClaimBoundary
  - ResearchLifecycle
  - ValidatorBridge
  - ScenarioExporter / LeakScanner / EvaluationRunner
        |
lif-research
  - LifCurrentSourceRouting
  - LifValidatorProfile
  - LIF claim-registry prior-existence/current-source capabilities
```

`general-science` 不得包含 LIF/FEP/R211/INDEX/MAP6 等项目特有标记。`lif-research`
必须显式继承 `general-science`，并只声明领域增量。

profile 继承采用 `additive_no_weakening`：

- parent 必须存在；
- 禁止循环继承；
- child 不得重复声明 parent extension、capability 或 reference runtime；
- effective profile 按 root→leaf 合并；
- reference runtime 身份仍不能授予 acceptance。

## 3. 测试隔离

LIF 内部任务不用于通用复杂测试、阈值校准或 holdout。P5 R211 snapshot 只保留为只读投影、
摘要一致性和“不执行源代码”的机械 fixture。

通用测试应来自独立合成科研包或许可清楚的外部非项目来源，并把 reviewer oracle 与被测包物理隔离。
LIF 只能在通用能力已经独立评估后，用作领域 profile conformance 或回归任务。

## 4. 后果

- `general-science` 当前只冻结 profile 合同，不表示 SourceRouter/EvidenceKernel 等核心已实现；
- GSA-PROFILE-001 可在合同层关闭，GSA-CORE/RUNNER/PARTITION/CAL/CORPUS 继续开放；
- Grok Build 仍只是 `lif-research` 的 observed reference runtime，不成为 general-science 的强制底座；
- 新领域 profile 应优先继承 `general-science`，不得复制通用科学扩展。

## 5. 验收

- schema 显式要求 `profile_inheritance_policy` 与 `extends_profile_id`；
- registry 中恰有一个 domain-neutral `general-science`；
- `lif-research` 只声明冻结的 LIF delta；
- 独立解析器和仓库检查器都拒绝循环、悬空父级与 inherited-state 重复声明；
- effective `lif-research` 可机械重建完整 general-science + LIF 能力集合。
