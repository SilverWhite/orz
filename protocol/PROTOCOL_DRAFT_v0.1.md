# Agent Protocol Draft v0.1

- 状态：Draft
- 日期：2026-07-17
- 配套 schema：[`agent-protocol-v0.1.schema.json`](agent-protocol-v0.1.schema.json)
- 配套 ADR：[`../adr/ADR-0001-evidence-constrained-local-agent-kernel.md`](../adr/ADR-0001-evidence-constrained-local-agent-kernel.md)

## 1. 目的

本协议定义模型、CLI 内核、工具、科学 validator、案例库和 claim registry 之间的最小公共语言。它不定义某个模型如何推理，也不把项目纪律压缩为提示词。

协议需要确保：

- 模型输出只能成为提案；
- 状态变化有明确主体和理由；
- 工具成功、产物有效、证据充分和 claim 可登记相互分离；
- 历史案例检索不会覆盖盲审阶段；
- 每项正式判断可追到当前工作区来源；
- 任一模型适配器都接受同一 gate 和 journal 约束。

## 2. 协议封装

一个 session record 包含：

```text
SessionRecord
├── session
├── task_contract
├── reasoning_precommitment
├── sources[]
├── actions[]
│   └── events[]
├── artifacts[]
├── evidence[]
├── gate_decisions[]
├── case_retrievals[]
└── claims[]
```

所有正式记录必须带 `protocol_version`。新增字段优先采用向后兼容扩展；枚举语义变化需要提升协议次版本或主版本。

## 3. 三套状态机

### 3.1 Task lifecycle

```text
discovered
  → routed
  → contracted
  → planned
  → ready
  → executing
  → reviewing
  → completed
```

旁路终态或暂停态：

```text
needs_input | paused | rejected | failed | cancelled
```

约束：

- `routed` 必须有当前 workspace source ledger。
- `contracted` 必须有 MUST/MUST NOT/SOURCE OF TRUTH/ACCEPTANCE。
- `completed` 只表示任务契约满足，不表示所有候选 claim 已登记。

### 3.2 Action lifecycle

```text
proposed
  → policy_checked
  → approved
  → running
  → succeeded | failed | cancelled | unknown
```

约束：

- 模型只能创建 `proposed` action。
- kernel/gate engine 才能推进 `policy_checked` 和 `approved`。
- tool runtime 才能写 `running` 和 terminal 状态。
- `unknown` 是正式状态，不能为了流程顺滑而折叠为成功或失败。
- 每个 action 恰有一个 `terminal` event。

### 3.3 Evidence lifecycle

```text
unchecked
  → observed
  → mechanically_validated
  → source_grounded
  → coverage_assessed
  → independence_assessed
  → claim_eligible
```

分支状态：

```text
rejected | degraded | superseded | withdrawn
```

这些状态不是简单总分。`EvidenceRecord.assessments` 分别记录：

- provenance
- completeness
- coverage
- independence
- controls
- statistics
- semantic_mapping

只有适用于当前 claim 的维度通过或被明确限定后，证据才能进入 `claim_eligible`。

## 4. TaskContract

最小字段：

```json
{
  "task_id": "TASK-20260717-001",
  "intent": "draft_agent_cli_design",
  "mode": "design",
  "must": ["write ADR", "write protocol", "seed regression corpus"],
  "must_not": ["modify experiment results", "start CLI implementation"],
  "source_of_truth": ["SRC-INDEX", "SRC-MAP6", "SRC-R-DOCS"],
  "acceptance": ["JSON schema parses", "YAML corpus parses"],
  "risk_class": "claim_bearing_design"
}
```

`source_of_truth` 引用 `SourceRecord.source_id`，不能只保存自然语言文件列表。

## 5. SourceRecord

来源必须区分“已打开并用于判断”和“仅由搜索发现”。

关键字段：

- `source_id`
- `kind`: workspace_file / raw_artifact / command_output / web_source / user_statement / memory_hint
- `path_or_uri`
- `content_hash`（可用时）
- `accessed_at`
- `usage`: routed / read / searched / cited / unchecked
- `epistemic_role`

`epistemic_role` 枚举：

- `source_grounded`
- `direct_comparison`
- `bridge_hypothesis`
- `user_supplied_unverified`
- `agent_inferred`
- `unchecked`

memory、历史摘要和用户叙述默认不能标为 `source_grounded`。

## 6. ReasoningPrecommitment

该对象必须在首次案例检索前写入 journal。

```json
{
  "precommitment_id": "PRE-001",
  "created_before_case_retrieval": true,
  "minimum_facts": [
    {"text": "checkpoint hash identical", "source_ids": ["SRC-A", "SRC-B"]}
  ],
  "candidate_hypotheses": [
    {"hypothesis": "deterministic replay", "status": "open"},
    {"hypothesis": "stale checkpoint reuse", "status": "open"}
  ],
  "disconfirming_evidence_sought": ["training completion log", "checkpoint creation time"],
  "uncertainties": ["process exit status not yet verified"],
  "forbidden_promotions": ["do not call the run independent"],
  "recommended_current_sources": ["train log", "manifest", "checkpoint hashes"]
}
```

案例检索后允许创建新的 precommitment revision，但必须保留旧版本和 `revision_reason`。

## 7. ActionProposal 与 ToolEvent

### 7.1 ActionProposal

动作必须包含：

- `action_id`
- `action_type`
- `requested_by`
- `state`
- `arguments`
- `risk_class`
- `expected_outputs`
- `required_gate_ids`
- `idempotency_key`

高风险动作如果没有稳定 `idempotency_key`，至少触发 `ACT-IDEMPOTENCY-001`。

### 7.2 ToolEvent

事件采用 append-only envelope：

```json
{
  "event_id": "EVT-001",
  "sequence": 17,
  "timestamp": "2026-07-17T10:00:00Z",
  "action_id": "ACT-001",
  "event_kind": "progress",
  "event_type": "process.stdout",
  "payload": {},
  "causal_parent": "EVT-000"
}
```

terminal event 额外记录：

- process exit state；
- exit code 或 `null`；
- completion summary presence；
- produced artifact IDs；
- terminal reason code。

空退出码不得自动映射为成功或失败。

## 8. ArtifactRecord 与 EvidenceRecord

### 8.1 ArtifactRecord

记录产物本身：

- path/URI；
- SHA256；
- media/schema type；
- producer action；
- atomic write 状态；
- parse/finite 检查；
- planned/completed/record count；
- sidecar/manifest 关系。

### 8.2 EvidenceRecord

EvidenceRecord 表示“某产物或观察被如何用于判断”，不能与 ArtifactRecord 合并。

```json
{
  "evidence_id": "EVD-001",
  "artifact_ids": ["ART-001"],
  "claim_ids": ["CLM-001"],
  "state": "coverage_assessed",
  "assessments": {
    "provenance": "pass",
    "completeness": "pass",
    "coverage": "limited",
    "independence": "not_applicable",
    "controls": "unchecked",
    "statistics": "unchecked",
    "semantic_mapping": "pass"
  },
  "limitations": ["1d/3d missing seed 131"],
  "independence_key": null
}
```

## 9. GateDecision

Gate decision 是结构化判断，不是自由文本警告。

```json
{
  "decision_id": "GDEC-001",
  "gate_id": "EVD-COVERAGE-001",
  "level": "guarded",
  "decision": "defer",
  "reason_codes": ["EVD-COVERAGE-001"],
  "subject_refs": ["EVD-001", "CLM-001"],
  "source_ids": ["SRC-R194"],
  "message": "1d/3d coverage is N=3, not N=4",
  "remediation": ["downgrade scope", "find raw rows", "targeted rerun"]
}
```

Decision 枚举：

- `pass`
- `warn`
- `block`
- `defer`
- `not_applicable`

`defer` 表示证据不足，不能被 UI 当成失败或成功。

## 10. ClaimCandidate

Claim 需显式声明：

- `claim_type`: fact / comparison / hypothesis / mechanism / recommendation / correction / withdrawal
- `scope`
- `strength`
- supporting/refuting evidence IDs
- limitations
- falsifiers
- prior-existence scan
- registry target
- state

强度建议枚举：

```text
observation
signal
compatible_with
supports
confirmed_within_scope
mechanism_candidate
mechanism_supported
```

`mechanism_supported` 不是普通数值结果的默认升级路径，必须由对照、消融或干预 gate 明确放行。

## 11. CaseRetrievalRecord

```json
{
  "retrieval_id": "RET-001",
  "performed_after_precommitment": true,
  "query": "identical checkpoint after completed run",
  "retrieved_case_ids": ["FEP-REG-006"],
  "similarities": ["same seed and configuration", "identical tensors"],
  "disanalogies": ["current run log has not yet been checked"],
  "missing_current_evidence": ["completion log", "checkpoint timestamps"],
  "countercase_ids": ["FEP-SYN-001"],
  "retrieval_influence": "expanded hypotheses; did not choose root cause"
}
```

若 `performed_after_precommitment=false`：

- 教学模式：允许，但必须标记 `oracle_exposure=true`；
- evaluation/holdout：协议违规，触发 `BIAS-LEAKAGE-001`。

## 12. 案例库协议

案例文件与 session schema 分离，但共享 reason code、source record 和 claim vocabulary。
案例本体必须通过 [`../regression/case-corpus-v0.1.schema.json`](../regression/case-corpus-v0.1.schema.json)；跨案例引用对称性、source path/line 存在性和 reason-code registry 覆盖由 kernel validator 检查。
错误簇与 challenge 覆盖以 [`../regression/coverage-matrix-v0.1.yaml`](../regression/coverage-matrix-v0.1.yaml) 为准；评分输出遵循 [`../evaluation/SCORING_PROTOCOL_v0.1.md`](../evaluation/SCORING_PROTOCOL_v0.1.md)，不得用单一总分覆盖 red-line failure。

每个案例分成两部分：

### 可给被测 Agent 的 scenario

- 当前时点可见材料；
- task；
- 允许使用的工具；
- 隐藏信息声明；
- 不含历史轮次最终裁决。
- 可选 oracle-free、`scenario_only` fixture manifest；其中 evidence 文件必须有 SHA256，且 synthetic fixture 不得标成历史原始证据。

### 仅供 curation/review 的 historical fixture

- 可通过 case 根部的 `curation_fixture_manifest` 记录 source-hashed 历史摘录；
- 必须标为 `reviewer_only`，不得进入 scenario export、模型上下文或可检索索引；
- 即使不存在结构化 oracle 字段，若自然语言含最终纠正或撤回结论，仍视为语义答案泄漏；
- 它只提高来源复核能力，不提升案例 evidence tier，也不构成当前任务证据。

### 隐藏 oracle

- expected gates；
- expected action/evidence/claim states；
- forbidden/allowed claims；
- required source questions；
- score dimensions；
- correction source。

自动分类器只能读取 source text 和当前 canonical metadata；evaluation/holdout 运行时不得读取 oracle 或 reviewer-only fixture。密封分区遵循 [`../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`](../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md)。

## 13. Reason code 初始命名

本节是便于阅读的摘要；机器可读语义、默认决策、误报保护和补救动作以
[`reason-codes-v0.1.yaml`](reason-codes-v0.1.yaml) 为准。相同 minor version 内不得静默改变 code 含义。

```text
ROUTE-SOURCE-001       当前工作区来源未回查
ROUTE-PRIOR-EXISTENCE-001 首次发现/新颖性未做先存扫描
TASK-CONTRACT-001      任务契约不完整
CFG-EXPLICIT-001       高风险维度使用隐式默认值
CFG-SEMANTIC-001       参数/flag 语义与实验定义不一致
ACT-IDEMPOTENCY-001    正式动作缺少幂等标识
PROC-TERMINAL-001      工具没有唯一终止事件
PROC-EXIT-UNKNOWN-001  退出状态未知
ART-SCHEMA-001         产物 schema 不合格
ART-FINITE-001         非有限数或非标准 JSON
ART-STALE-001          产物与当前 action 的生产者关系未成立
ART-FEATURE-ORDER-001  特征名/切片未与 producer 顺序核对
EVD-PROVENANCE-001     标签与实际执行链未核对
EVD-COVERAGE-001       planned/completed/coverage 不一致
EVD-INDEPENDENCE-001   重复执行被误计为独立证据
EVD-CONFOUND-001       比较包含未控制混淆变量
EVD-COMPARABILITY-001  任务、协议、指标或评估人群不可比
EVD-EXECUTION-MATCH-001 命名干预与实际计算不一致
SEM-MAPPING-001        指标方向或术语映射错误
CLM-STRENGTH-001       claim 强度超过证据
CLM-GENERALITY-001     generalization 超过抽样和验证人群
CLM-MECHANISM-001      机制解释缺少判别测量或干预
SYNC-STATUS-001        文档状态与当前产物不一致
BIAS-PRECOMMIT-001     案例检索前无独立初判
BIAS-SURFACE-001       只记录相似处，未检验不相似处
BIAS-LEAKAGE-001       evaluation/holdout oracle 泄漏
BIAS-CONFIRM-001       未主动搜索反向证据
```

## 14. 模式

逐 gate 的 stage、默认决策、聚合优先级和 bypass 边界以
[`gate-matrix-v0.1.yaml`](gate-matrix-v0.1.yaml) 为准。

### Discussion

- 路由和独立性检查 observation-first。
- 允许探索性假说。
- 不允许静默注册强 claim。

### Guarded work

- 默认 CLI 模式。
- 权限、覆盖、隐式 sweep、来源、独立性和强 claim 可 block/defer。

### Strict claim promotion

- MAP/INDEX/result registration 或长跑启动。
- 要求完整 task contract、source ledger、artifact/evidence records、prior-existence scan 和第二轮 self-check。

### Regression evaluation

- 隐藏 oracle。
- 禁止检索 evaluation/holdout 自身。
- 允许检索 development/teaching，但必须经过 precommitment。
- 默认按 guarded oracle 评测；其他模式必须单独记录，不能混算。
- case-micro 用于定位，cluster-macro 用于主要比较，contrast-pair 用于反惯性判别。
- 未建立隔离分区和人类基线前，结果只能是 descriptive/calibration pending。

## 15. 不能由 JSON Schema 单独保证的约束

以下约束必须由 kernel validator 执行：

1. 每个 action 恰有一个 terminal event。
2. sequence 单调且 event 不可改写。
3. case retrieval 时间晚于 precommitment。
4. claim 引用的 evidence 必须存在且状态足够。
5. completed task 满足 acceptance。
6. source-grounded 事实的 source 必须实际读取。
7. evaluation/holdout oracle 没有进入模型上下文。
8. planned/completed/count 和 artifact 记录一致。
9. independence key 冲突时不能增加独立 N。
10. claim strength 不超过最弱适用 assessment。
11. corpus revision、oracle、prompt、tool policy 和评分协议 digest 可重建。
12. cluster/countercase 交叉引用存在，且 pair 类型与 expected delta 一致。
13. scenario export 不含 oracle、curation fixture、source path、countercase identity 或 `hidden_from_subject` 原文。
14. evaluation/holdout 的 leak scanner 未完成、warn、crash 或未知版本时 fail-closed。
15. run manifest 在运行前冻结，journal sequence/hash chain 可重放且未被 summary 替代。

## 16. 初步兼容策略

- 协议版本采用语义化版本。
- journal event 永不就地迁移；使用派生 migrator 生成新视图。
- 未知枚举值必须 fail-closed 或保留为 unknown，不能映射到最接近的旧值。
- Python 科学工具首选 JSONL/stdin/stdout adapter。
- Rust 控制面候选实现必须通过同一 schema 和 regression corpus。

## 17. 当前开放问题

1. journal 默认进入 `.agent/` 还是单独工作区外目录。
2. MAP/INDEX 更新是否始终要求人工确认 patch。
3. 大文件 hash、checkpoint fingerprint 和 Windows 文件锁的性能边界。
4. case classifier 使用纯规则、轻量模型还是混合模式。
5. 外部通用研究案例如何与项目内部案例平衡。
6. holdout corpus 的维护者和解封规则。
7. Rust 控制面采用原生 JSONL、ACP adapter 还是两者并存的版本边界。

## 18. Runtime 导出与执行边界

scenario 裁剪、opaque identity、fixture 复制、leak scan、immutable run manifest、model adapter、tool broker 和 hash-chained journal 遵循
[`../runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`](../runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md)。

- 通用 CLI/runtime 结构参考 Grok Build，但 oracle isolation 与 evidence gate 是本项目内核职责；
- evaluation/holdout 不继承用户 memory、项目历史检索、reviewer fixture 或任意用户配置；
- hook 可以观察或通知，不能代替 fail-closed leak/evidence gate；
- ACP 是可选 transport adapter，不是本协议的 source-of-truth。

## 19. 工作区来源

- `LIF_CURRENT_INDEX.md`（旧研究工作区；未随独立仓库迁移）
- `fep_env_research.md`（旧研究工作区；未随独立仓库迁移）
- `self_check_protocol.md`（旧研究工作区；未随独立仓库迁移）
- `.github/skills/fep-script-validation/references/script_contract.yaml`（旧研究工作区；未随独立仓库迁移）
- [`../regression/cases-v0.1.yaml`](../regression/cases-v0.1.yaml) 中列出的 R/MAP 来源

这些旧工作区路径在本独立仓库中是 provenance locator，不是已核当前来源；claim-bearing 使用前必须重新挂载并读取。
