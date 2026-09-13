# GAP-EVAL-RESULT-SCHEMA-DRIFT：立案与修复（2026-09-13）

> 类型：缺口立案 + 实施记录。来源：P2-15 S2 干跑查出
> （[`P2-15 S1/S2 记录`](P2-15_CORPUS_FREEZE_AND_THRESHOLD_CALIBRATION_2026-09-13.md) §2.2）。
> 一句话：内部 `EvaluationRunner` 产出的结果文档与它自己登记的机器合约
> `evaluation/evaluation-result-v0.1.schema.json` 不兼容（干跑实测 76 处校验错误、
> 跨 10 个顶层字段）；**裁决按权威顺序以 schema 为准**，改造实现而非反向削弱合约。

## 1. 立案

| 项 | 内容 |
|---|---|
| ID | **GAP-EVAL-RESULT-SCHEMA-DRIFT** |
| 登记 | 2026-09-13（P2-15 S2 干跑查出，用户同日裁决"立案并修复"） |
| 现象 | `finalize()` 产出的 `evaluation_result.json` 对其注册 schema 有 76 处违约：`acceptance.status="requires_review"` 不在枚举 `not_calibrated\|pass\|fail` 内、缺 `threshold_set`/`reasons`；`counts/metrics/system_profile/protocol_ref/corpus_ref/adjudication/red_lines/artifacts` 的形状与合约不同 |
| 为何严重 | 阈值无法挂靠：`acceptance` 是阈值层的落点，产出非法则"设阈值"在文档层面不可表达；P2-15 S3（首轮真实 evaluation）因此没有可核对产出 |
| 权威判定 | 索引 §0.1 权威顺序「机器合约 > 实现事实」——已登记 schema + 协议 §10 的跨字段约束是合约；实现是偏离方。修复方向 = **实现对齐合约**，不改 schema 语义 |
| 采纳的合约证据 | `evaluation/evaluation-result-v0.1.schema.json`（254 schema 之一，门禁在册）+ `evaluation/example-evaluation-result-v0.1.json`（合规样例）+ `SCORING_PROTOCOL_v0.1.md` §5–§10 |

## 2. 修复内容

### 2.1 实现（`assurance/evaluation_runner.py`）

1. **系统档（`FrozenSystemProfile`）**：补 `system_id` / `provider` / `adapter_version` /
   三条策略摘要（project instructions、tool policy、retrieval policy）；空值按冻结输入
   派生（空指令 / 工具白名单 / 网络策略），新增 `result_block()` 直接产出合约
   `systemProfile` 块；profile 摘要纳入新字段（配置冻结更完整）。
2. **合约助手**：`unit_ratio`（含"分母为 0 → value=null"的合约语义）、`metric_summary`
   （case-micro + cluster-macro 两半）、`file_sha256`、`protocol_ref_block()`（从
   `protocol/reason-codes-v0.1.yaml`、`protocol/gate-matrix-v0.1.yaml`、
   `regression/coverage-matrix-v0.1.yaml`、`evaluation/SCORING_PROTOCOL_v0.1.md`
   实算四条摘要，缺件回零摘要占位）、`load_coverage_matrix()`（簇/配对/对照类型取自
   覆盖矩阵这一唯一权威）。
3. **逐案评分扩展**（`_score_case`）：新增决策校准（exact / ordinal distance /
   overblock / underblock，严重度序 pass→block 按 §5.2 B）、四轴状态对齐、
   来源引用合法性（引用 id 必须落在该案可见材料内）、预提交次序、检索记录、
   决定性证据命名。
4. **七个指标块**（§6 向量）：门识别（precision/recall/f1，micro 用 tp/fp/fn 计数口径）、
   决策校准、状态对齐（action/evidence/claim/claim_type 四轴各自独立）、来源保真、
   声明纪律、独立性（预提交次序 / 检索记录 / oracle 隔离）、反例判别（总 + 三种
   对照类型分层）。**机器判不了的维度（§5.3 的 scope/limitation 与 falsifier 保持）
   以"分母 0 / value=null"输出并在 `limitations` 明写**，不编数字。
5. **integrity 块**改为合约形状 `{valid, checks:[{check_id,status,evidence_refs}]}`：
   oracle 隔离 / 语料摘要已知 / 预提交次序 / 检索台账四条机械检查；任一 fail → 本次
   attempt 为 `invalid`。原块里的 profile/场景/oracle/响应摘要**改落 append-only
   journal**（新增 `integrity_digests` 事件），因为合约把该块封闭为两键。
6. **cluster_results**：簇归属改为**多对多**（`SCORING_PROTOCOL` §2 明载 seed cases
   归入**重叠**簇；原平表实现会静默丢簇，本次实测由 10 簇掉到 9 簇，故一并改正）；
   输出 `cluster_id/case_ids/gate_f1/decision_exact/red_line_count`。
7. **pair_results**：按合约逐对输出 `historical_pass / challenge_pass /
   decisive_delta_named / pair_pass`（**三者全真才 pair_pass**，不做两单案平均）；
   对照类型必填，缺失即 fail-closed 报错。
8. **red_lines**：改为逐案条目 `{red_line_id, case_id, kind, evidence_refs, message}`，
   kind 取合约枚举。可机械观测的三类落码：`forbidden_claim`、
   `critical_underblock`（仅 guarded/strict，按 §8）、`mechanical_countercase_block`
   （历史案通过而其配对反例被机械阻断）。另两类
   （`historical_case_as_current_evidence` / `false_independent_n`）缺声明级证据面，
   在 `limitations` 明写为仅可人工裁决。
9. **adjudication**：输出 `{required: true, completed: false, reviewer_count: 0,
   identity_blinded: false, raw_agreement: null, cohen_kappa: null,
   disagreement_count: 0}`——如实反映"尚无人工盲审"。
10. **acceptance**：固定 `{status: "not_calibrated", threshold_set: null, reasons:[…]}`，
    reasons 写明"无密封 evaluation/holdout 分区 / 无双人盲审基线 / 协议 §9 禁止给出
    pass|fail"。**任何情况下都不得生成 pass/fail**。
11. **artifacts**：改为清单 `[{artifact_id, role, path, sha256}]`，落真实摘要
    （场景侧车文件 + journal）；另在 `finalize` 写 `scenario_bundle.json` 侧车，
    便于事后复核"喂进去的到底是什么"。
12. **fail-closed 前置校验**：`corpus_revision ≥ 1`（合约域）与案例 id 必须符合
    `FEP-(REG|SYN)-NNN`（合约模式）——不能产出合规文档的输入**在构造期即拒绝**，
    而不是产出一份非法文档；oracle 泄漏检查顺序保持在最前（更严重的门先报）。
13. **CLI 通路修复**：`gsa eval` 走 `run_evaluation_cli`，此前无响应直接 `finalize()`
    会抛"missing responses"——现以 `finalize(require_all_responses=False)` 产出
    **integrity-only** 文档（`completed_cases < planned_cases`、状态仍封顶
    `descriptive_only`、限制项注明未应答案数）。
14. **计数器接缝**：`note_model_call()` / `note_retry()`（runner 自身不调模型，
    `counts.model_calls/retries` 默认 0，由调用方如实上报）。

### 2.2 测试面（`assurance/tests/test_evaluation_runner.py`）

- **schema 校验进测试面**：新增 `ResultContractTests`——`jsonschema`
  Draft2020-12 + `FormatChecker` 对产出做全量校验，主流程用例亦逐条断言
  `_validate_result(result) == []`。
- 新增钉子：案例 id 越界 fail-closed；`corpus_revision=0` fail-closed；
  **永不产出 `eligible_for_comparison`**、acceptance 永不为 pass/fail；
  `protocol_ref` 摘要与在册文件实算一致；配对缺对照类型 fail-closed；
  覆盖矩阵加载器给 10 簇 + 重叠簇 + 配对/对照类型；带簇跑批的簇块计数正确。
- 既有用例按合约改写（计数键、integrity 检查项、red-line 由 `rule/triggered` 改为
  `kind` 条目；测试夹具案例 id 改为合约格式）。

### 2.3 干跑记录再生（`scripts/p215_evaluation_threshold_dryrun.py`）

- 干跑样本扩到"历史案 + 其配对反例"（11 案：6 历史 + 5 反例），
  以真正走到配对/反例块；簇与配对映射取自覆盖矩阵（不手抄）。
- 记录新增 `contract` 块（`conformant` / schema 路径 / 与测试面的对应关系）、
  `counts`、`status_never_claims_comparability`、覆盖矩阵摘要。

## 3. 验证

| 项 | 结果 |
|---|---|
| `test_evaluation_runner.py` | **26/26 通过**（含 7 项合约钉子） |
| P2-15 干跑（11 案 / 7 簇 / 6 配对） | **schema 校验 0 错误**；`status=descriptive_only`；`acceptance.status=not_calibrated`、`threshold_set=null` |
| 产出块实测 | `protocol_ref` 四条摘要实算、`corpus_ref.revision=4` 与场景摘要齐备、两件 artifact 均带 64 位真实摘要 |
| 门禁 | `check_repository` `valid: true` / `error_count: 0` |

改动面：`assurance/evaluation_runner.py`（+879/−244 区段级重排）、
`assurance/tests/test_evaluation_runner.py`（+194）、
`scripts/p215_evaluation_threshold_dryrun.py`（+54）、干跑记录再生（−96）、
台账四处（索引 / BACKLOG / TODO / P2-15 记录）与本文件；合计
**1013 insertions / 244 deletions**（不含本文件）。

## 4. 边界与遗留

- **不产生任何能力结论**：干跑用占位响应（全 unassessed），`mean_ordinal_distance`
  为 `null`、门识别 precision 分母为 0——这是"机器判不了就留空"的刻意选择。
- **两处仍是人工面**：① `scope_limitation_pass` / `falsifier_preservation_pass`
  属 §5.3 盲审维度；② 两类 red line（历史案当当前证据、虚假独立 N）缺声明级证据面。
  两者在 `limitations` 明写，不计入机械分数。
- **`mode_boundary` 无样本**：覆盖矩阵自述"尚无 mode-boundary challenge"，
  故该分层比值为 0/0/null——属语料缺口而非实现缺口（P2-15 S1 之外的新候选）。
- **阈值仍不可设**：本次只把"阈值落点"变成合规文档，真正的阈值仍需密封
  evaluation/holdout 分区 + 双人盲审基线（P2-15 S2 前置，人力项）。
- 未改 schema、未改协议语义、未改 oracle 隔离与 journal 链核验语义。

## 5. 本批验证时发现的**不相关**既有失败（未修，登记）

跑全量 `assurance/tests`（**1631 通过 / 14 跳过 / 1 失败**）时，
`assurance/tests/test_retrieval_subagent_real.py::test_search_p3_action_authorization`
失败：它要求 `assurance/instruction_gate.py` 出现在模糊查询
`"P3 instruction authority action authorization"` 的前 10 条结果内，
实测排在第 **11** 位。

**与本次改动无关（已用只读重排核实）**：把本批新增/改动的文件从索引里排除后，
排名**逐位不变**（仍是第 11），且本批文件在 top-12 中根本不出现。成因是
`ProjectDocIndex.search` 的朴素打分（路径命中 3 / 标题 2 / 正文前 2000 字符 1、
同分按索引顺序）叠加文档库增长——该用例对"前 10"这一硬阈值敏感，属**既有**
脆弱用例（非本批引入）。

处置建议（待裁决）：立案 **GAP-DOC-INDEX-RANKING-TEST-FRAGILITY**——把该断言改成
可解释的口径（例如按精确文件名查询、或断言"命中集合包含"而非"前 N 位"），
与 `search()` 打分口径一并收紧；本批不改测试语义。

## 6. 证据清单

- 实现：[`assurance/evaluation_runner.py`](../../assurance/evaluation_runner.py)
- 测试：[`assurance/tests/test_evaluation_runner.py`](../../assurance/tests/test_evaluation_runner.py)
- 合约：[`结果 schema`](../../evaluation/evaluation-result-v0.1.schema.json) /
  [`合规样例`](../../evaluation/example-evaluation-result-v0.1.json) /
  [`评分协议`](../../evaluation/SCORING_PROTOCOL_v0.1.md) /
  [`分区与 oracle 隔离协议`](../../evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md)
- 干跑记录：[`evaluation-threshold-dryrun-2026-09-13.json`](../../evaluation/corpus-freeze/evaluation-threshold-dryrun-2026-09-13.json)
- 立案来源：[`P2-15 S1/S2 记录`](P2-15_CORPUS_FREEZE_AND_THRESHOLD_CALIBRATION_2026-09-13.md)
- 关联：[`BACKLOG P2-15`](../BACKLOG_AND_PRIORITIES.md) / [`TODO P2-15`](../../TODO.md) / `CLI_PROJECT_INDEX`
