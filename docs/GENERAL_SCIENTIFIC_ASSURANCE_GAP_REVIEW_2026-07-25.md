# 通用科学保障能力缺口初审与深度测试边界 — 2026-07-25

## 结论

当前仓库已经形成 P0–P5 的 runtime-neutral 安全与可审计 development/conformance
纵向切片，但尚未形成可用于正式科学能力评测的通用科研闭环。

LIF 项目暴露出的来源先行、多文件回查、证据分层、独立性、反例搜索和 claim
强度控制问题具有普遍科研意义；LIF 自身的 INDEX/MAP/R/self-check 路由、长期历史和专门术语则具有
明显领域特殊性。后续应抽取前者为 `general-science` profile，把后者保留为 `lif-research`
profile，不能再用 LIF 内部任务充当通用复杂测试或未见深度测试。

本文件是工程缺口与测试边界记录，不登记新的 LIF 科学 claim，不修改 LIF INDEX/MAP/R，
也不把 P5 的 R211 只读投影提升为模型或人类能力测试。

## 已确认的测试边界

1. 通用复杂测试不得直接使用 LIF 内部任务、历史纠错链、INDEX/MAP/R 路由或现成 claim registry。
2. R211 snapshot 只保留为 P5 只读投影、来源摘要一致性和不执行源代码的机械 fixture。
3. LIF 可以在通用能力通过独立测试后，用作领域 profile conformance 或回归任务；不能进入同一轮
   通用能力开发、阈值校准或 holdout。
4. 新复杂任务应来自独立构造的合成科研包或许可清楚的外部非项目来源，并把被测材料与 reviewer
   oracle 物理分离。
5. 多文件回查应由通用来源 manifest、版本冲突、产物 lineage、分析脚本和结果摘要触发，
   而不是依赖 LIF 专有文件名或历史记忆。

## 当前已验证基线

截至本记录创建前，仓库的机械状态为：

- repository contract：79 schemas，0 errors；
- prototype：50 tests；
- Grok reference integration：44 tests；
- runtime + General Assurance Kernel：41 tests；
- 合计：135 tests；
- P0–P5 代码、schema、fixture、审计文档和 CI 已冻结在 commit `6ddca3d` 与
  tag `p0-p5-development-baseline-2026-07-25`。

这些结果只证明当前合同、fixture 和 development 实现自洽，不证明：

- 模型能完成复杂科研任务；
- 科学结论正确；
- 通用性、holdout 表现或人类可用性；
- production hard gate 已覆盖所有 runtime/tool 入口。

## 正式科学评测前的 P0 缺口

| ID | 缺口 | 当前状态 | 初步完成条件 |
|---|---|---|---|
| GSA-BASE-001 | 可复现基线未冻结 | **已关闭（development baseline）**：commit `6ddca3d`，tag `p0-p5-development-baseline-2026-07-25` | 后续正式评测仍需单独冻结 runner、任务包、oracle、模型和阈值 |
| GSA-PROFILE-001 | 缺少通用科学 profile | **合同层已关闭**：`general-science` 为 domain-neutral root，`lif-research` 以 `additive_no_weakening` 只添加领域 delta | 核心实现继续由 GSA-CORE/RUNNER 等条目跟踪 |
| GSA-CORE-001 | 科学保障核心没有可执行纵向切片 | **初始只读与 disposable reproduction 切片已完成，完整核心仍开放**：已接通非 LIF 多文件来源→任务契约→设计→既有动作/产物→JSON Pointer 比较→证据→claim→确定性报告，并新增固定 no-model replay receipt/verifier、code/env/input/output/journal run-proof、runtime preflight projection、一次性 disposable runtime journal write/replay、development code/environment lock、no-model runner skeleton、sidecar journal lock、recovery inspection、受控 repair/quarantine receipt 与 lifecycle repair policy | 下一步增加真实输入数据、完整 dependency/OS/container lock、adapter preflight、可扩展 validator、统计/推断审查与跨制品语义；不得把本机械切片视为完整科学审查 |
| GSA-VALIDATOR-001 | ValidatorBridge 只有扩展名称 | **初始桥接已完成，完整 validator 体系仍开放**：6 个版本化 validator 覆盖设计/动作/registered-artifact schema、JSON finite、统计报告完整性和跨 artifact comparability，registry 与结果均摘要绑定 | 增加版本迁移和隔离的第三方 validator；统计/comparability PASS 不得解释为科学正确 |
| GSA-COUNTEREXAMPLE-001 | orientation、反例补全和停滞 guard 未接入真实 runner | **只读 fixture、runner 接入前置夹具、no-model journal projection、公开输出抽取层、direct DeepSeek API one-shot 观测投影与 DeepSeek-shaped streaming/repetition fixture 已实现，真实 runner 未启用**：中性 orientation checkpoint 只回答任务定位并拒绝 counterexample/claim 字段；counterexample queue 仍只在 claim review 路径单独使用；runtime stagnation guard 用公开输出重复阈值和有限 retry budget 处理 runaway；接入前置夹具可写出两个 artifact 和汇总 receipt 并独立重建；journal projection 已冻结 `orientation_checkpoint`/`runtime_stagnation_guard` 事件顺序和 hash-chain；公开输出抽取层只允许 public assistant 文本进入 guard 输入，private/redacted 只保留 digest；direct API 观测只发起一次固定 marker 请求并投影公开输出，不保存 credential/raw response/隐藏 reasoning 文本；DeepSeek-shaped stream fixture 将 public delta、private reasoning digest 和 terminal metadata 分层投影，并用 11 次重复 public delta 覆盖 restart projection | 下一步设计真实 streaming 截断边界和 runner adapter 策略；不得保存隐藏 chain-of-thought；不得让 orientation 自动生成反例或 claim disposition |
| GSA-ARTIFACT-001 | Artifact 没有版本化 schema registration | **初始 registration 已完成，完整 artifact 体系仍开放**：artifact 必须引用登记 schema ID；registry kind/media/schema 映射和摘要写入结果，未知或不匹配项在证据比较前 block | 增加表格、单位、图像、代码/环境 schema，以及版本迁移与撤销 |
| GSA-LINEAGE-001 | 跨 artifact lineage/comparability 只能靠叙述 | **初始数值比较与 disposable replay/run-proof 切片已完成，完整 lineage 仍开放**：left/right artifact 分别绑定 pointer；task/protocol/metric/population/unit、producer/source/transformation 和 condition 实值机械对账；replay 输出与原 artifact 只做值级复放对账，run-proof 绑定当前实现/环境/输入/输出摘要，disposable runtime journal 绑定 projection event digest，execution lock 绑定受控源码与 Python 分发版本 | 接入真实输入数据、完整 dependency/OS/container lock，增加 transformation DAG、单位换算、population harmonization、多 action/producer 和独立性核算 |
| GSA-ADAPTER-001 | 真实 runtime/tool 入口未接保障层 | P3/P4 为 development API；P4.5 只允许固定进程内 roundtrip；2026-07-27 offline CLI 已串 instruction provenance / tool availability / source visibility，真实 adapter 仍开放 | 至少一个 runtime adapter 的读取、工具、授权、审计和 terminal 不可绕过同一 envelope/gate |
| GSA-RUNNER-001 | 正式 EvaluationRunner 未实现 | **no-model runner skeleton 与 canonical CLI offline 路径已完成，正式 runner 仍开放**：manifest/receipt/verifier/run-proof、runtime manifest/event projection、disposable JSONL replay、development source/env lock、sidecar locked runner journal append/replay、recovery inspection、受控 repair/quarantine receipt 与 lifecycle repair policy 只覆盖一个固定 in-process replay；canonical CLI 另接通 instruction provenance gate、tool availability gate、orientation checkpoint、source visibility gate、fake DeepSeek-shaped adapter boundary、answer packet、runtime journal 和 verifier，但仍不接真实模型、外部代码、工具 broker、评分或结构化失败分类 | 实现真实 adapter preflight、结构化输出校验、失败分类与评分交接 |
| GSA-PARTITION-001 | evaluation/holdout 为零 | 当前 40 个案例均为 development/challenge seed | 创建物理隔离的新来源 evaluation 和未见 holdout；冻结 digest、oracle、角色与污染生命周期 |
| GSA-CAL-001 | 人类 baseline 和阈值为零 | 未招募 reviewer、未测一致性、无有效模型通过阈值 | 双人独立盲审、adjudication、可判定性与一致性检查后冻结阈值，再应用于未参与校准的 holdout |
| GSA-CORPUS-001 | 案例仍为项目形状 | 十个错误簇来自 FEP/LIF 历史与合成 countercase | 增加跨领域、非 LIF、正反成对且来源独立的复杂科研任务 |

`GSA-PROFILE-001` 合同增量完成后的机械结果为 79 schemas、1 个结构合法但语义无效的
inheritance-cycle 负例、137 tests、0 errors。该计数不提升 GSA-CORE 或正式评测状态。

`GSA-CORE-001` 初始切片的设计与限制见
[`GSA-CORE read-only audit`](GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md)。其新增的指数衰减 fixture
是非 LIF development/conformance 正例，不进入 evaluation、holdout 或阈值校准。
该切片完成后的机械结果为 81 schemas、144 tests、0 repository errors。

`GSA-VALIDATOR-001` 初始桥接见
[`ValidatorBridge audit`](GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md)。它复用协议 reason code，只检查
合同和报告完整性；不运行统计分析，也不提升 claim 强度。
该桥接完成后的机械结果为 84 schemas、4 validators、146 tests、0 repository errors。

`GSA-ARTIFACT-001` 初始登记见
[`artifact registration audit`](GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md)。schema PASS
只证明 artifact 符合登记结构，不证明 producer、数值或科学解释正确。
该登记完成后的机械结果为 87 schemas、5 validators、2 artifact schemas、147 tests、
0 repository errors。

`GSA-LINEAGE-001` 初始切片见
[`cross-artifact audit`](GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md)。拆分文件不会
增加独立 N；comparability PASS 只允许窄比较，不支持机制或普遍性升级。
该切片完成后的机械结果为 88 schemas、6 validators、3 artifact schemas、148 tests、
0 repository errors。

`GSA-RUNNER-001` 的首个 disposable reproduction 切片见
[`disposable reproduction audit`](GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md)。它只证明同一
development fixture 的固定 replay 输出可被 receipt/verifier 复核；replay 明确不增加独立证据、不提升
claim 强度，也不构成正式 EvaluationRunner。
该切片完成后的机械结果为 123 schemas、1 个 disposable reproduction fixture、12 个定向测试、
当前源码全量回归 242/242，0 repository errors。

`GSA-RUNNER-001` 的首个 canonical guarded CLI offline P0 路径见
[`canonical guarded CLI audit`](CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md)。它证明 run manifest、
source visibility gate、fake DeepSeek-shaped adapter boundary、answer packet、runtime journal 和 verifier
可以被串成一个不可绕过 gate 的主路径；不重新调用真实 DeepSeek，不证明真实模型能力、检索能力或科学正确性。

## 通用科研语义缺口

现有 reason code 已覆盖来源、配置、执行、产物、独立性、混淆、可比性、机制、普遍性和确认偏误。
后续不应推翻这些基础，而应补充以下通用层：

### 统计与推断

- 效应量、不确定性和区间报告；
- 样本量/功效依据与停止规则；
- 统计假设、模型诊断和稳健性分析；
- 多重比较、探索性分析与验证性分析隔离；
- 缺失数据、异常值、排除规则和敏感性分析；
- sampling unit、重复测量、层级结构与有效独立 N。

### 文献与外部证据

- 可定位的引用、版本、DOI/稳定标识符和访问时间；
- 文献、帖子、网页和 thread 检索必须报告全文可见性状态；未完整抓取并浏览全文时必须标注为
  `partial_text_observed`、`metadata_only` 或 `unavailable`，不得用摘要/引言/片段冒充全文证据；
- 原始来源与二手转述分层；
- 撤回、勘误、版本替换与相反证据检查；
- 文献检索范围、遗漏风险和“未检出不等于不存在”边界；
- 文献事实、跨论文直接比较和机制桥接推断分离。

### 可复现性与数据 lineage

- 数据版本、生产者、许可和变换链；
- 代码 commit、依赖/环境锁、随机源、设备/后端与数值精度；
- 计划参数、解析后参数、实际执行参数和结果 schema 对账；
- 原始数据、派生数据、分析产物和报告 claim 的双向追踪；
- deterministic replay 与新增独立证据的语义分离。

### 研究生命周期

- 问题定义、假设、可证伪条件和替代解释；
- 预注册、对照/基线、干预边界和允许的探索性偏离；
- 运行中偏差、协议修订、失败和负结果；
- 复现、外部验证、claim 降级/撤回和版本化报告；
- reviewer 与执行者的角色、独立性和冲突披露。

### 治理与适用边界

- 人类参与、隐私/PII、许可、伦理审批和数据最小化；
- 敏感领域、双重用途与高影响行动的额外 profile gate；
- 不适用、未知、待人工审查与硬阻断的明确区分；
- 不能由本地机械 PASS 推导出的外部安全或伦理结论。

## 通用复杂测试的初步构成

首批复杂任务不追求数量，应先覆盖相互独立的四种任务形态：

1. **计算实验审查**：设计说明、配置、代码、manifest、日志和结果之间存在可判断的不一致或合法差异。
2. **观察数据分析审查**：包含缺失值、相关样本、混淆因素、多个分析口径和受限结论范围。
3. **文献综合审查**：多篇来源有版本、定义和证据层级冲突，要求定位引用并保留未知项。
4. **跨文件复现审查**：数据 lineage、环境、随机性、产物 producer 和报告 claim 需要多文件对账。

每种任务至少准备：

- 一个应当阻断或降级的 detection case；
- 一个表面相似但证据充分、应当允许窄结论的 permission-reversal case；
- 一个结论随 discussion/guarded/strict 模式变化的 mode-boundary case；
- 被测包、reviewer-only oracle、来源 ledger、评分 rubric 和污染记录；
- 能推翻预期答案的明确条件，避免测试只奖励固定措辞。

任务作者、保障层实现者和评分维护者重叠时必须披露；这类结果只能标为 development 或受限评测。

## P1：外部用户测试前仍需关闭

- `GAK-SBX-001`：Windows native strict sandbox；
- `GAK-RET-001`：真实 storage 的归档删除、失败恢复、并发与 crash；
- `GAK-INJ-001`：所有真实入口不可绕过的 instruction provenance/action gate；
- `GAK-TRUST-001`、`GAK-CHILD-001`、`GAK-NET-001`：trust、子能力与网络许可统一接入；
- `GAK-EVT-001`、`GAK-REC-001`：真实事件投影、shadow store、diff preview 和恢复执行；
- `GAK-WIN-001`：Windows start→Job assignment race；
- `GAK-UX-001`：新手/熟练用户理解、错误恢复和认知负担。

内部、network-off、disposable 的科学评测可以在固定 Docker strict backend 下先行，不必把
Windows native backend 误设为所有内部测试的前置条件；真实项目、凭据、外部副作用或普通用户默认入口
仍必须等待相应 production blocker 关闭。

## P2：产品化与维护缺口

- 根级可安装包、统一 CLI、quickstart 和版本输出；
- 依赖锁、schema migration、backward/forward compatibility fixture；
- coverage、lint、类型检查、依赖/供应链安全检查；
- LICENSE、SECURITY、CHANGELOG、发布、升级与回滚流程；
- 长时运行、磁盘增长、并发、故障注入和资源预算；
- Linux/macOS 独立 observed 矩阵。

## 基线冻结验收

本轮第一步只冻结已完成的 P0–P5 development/conformance 状态，不借提交动作关闭以上缺口。
提交前必须：

1. repository contract、全部 135 tests 和 Python compile 通过；
2. PowerShell 脚本语法检查通过；
3. `git diff --check` 通过；
4. 暂存清单不包含 `.observed-runs/`、`.tools/`、cache、真实凭据或 LIF 源项目文件；
5. 提交说明明确是 reproducible development baseline，而不是 production/scientific-evaluation release；
6. 提交后工作树干净，并记录 commit ID。

## 证据边界

- **source-grounded**：当前仓库 README、General Assurance Kernel gap register、profile registry、
  assurance README、coverage matrix、scoring/partition/human-baseline 协议、runner contract、
  P5 audit、代码/测试/CI 和本轮机械命令。
- **user decision**：LIF 的领域特殊性会污染通用复杂测试，因此不得继续使用 LIF 内部任务作为深度测试。
- **agent-inferred design**：`general-science` profile 的分层、四类复杂任务和新增语义缺口。
- **user decision**：外部文献、帖子或网页检索必须固定回报全文抓取/浏览状态；未读完整全文时必须显式
  标注缺漏，避免摘要、介绍或片段被误用为全文证据。
- **unchecked**：这些任务类型的实际难度、reviewer 一致性、模型成本、跨学科代表性和最终通过阈值。

本轮没有提出新的 LIF 科学 claim，因此无需在 LIF claim registry 中做 prior-existence 登记。
