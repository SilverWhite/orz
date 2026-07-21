# Evidence-Constrained Agent CLI Design

状态：upstream-first 集成设计；官方 Grok Windows binary 已在忽略目录中完成离线核验，尚未登录或调用真实 Grok/DeepSeek 模型。

本目录记录面向当前 FEP/LIF 共同研究工作流的本地 Agent CLI 设计。目标不是复活 D，也不是复制 Grok Build，而是把当前项目已经形成的证据纪律、纠错记录和机械 validator 固化为模型无关的协议、状态机和回归测试。

## 当前文件

- [`adr/ADR-0001-evidence-constrained-local-agent-kernel.md`](adr/ADR-0001-evidence-constrained-local-agent-kernel.md)：产品边界、核心不变量、案例库和反捷径决策。
- [`architecture/GROK_BUILD_ADAPTATION_v0.1.md`](architecture/GROK_BUILD_ADAPTATION_v0.1.md)：基于官方开源快照的 adopt/adapt/defer/reject 矩阵与项目专化层。
- [`architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`](architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md)：Windows 预编译 Grok + 薄 LIF sidecar 的正式范围，以及现有 prototype 的降级分类。
- [`architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`](architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md)：优先准确性的全程可观测 wrapper、记录分层、资源取舍与反补全约束。
- [`architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)：对 Grok ACP、Codex、Gemini CLI、OpenCode、Goose 与 Cline 的职责深拆；将生产 runtime 收回 Grok，只保留 LIF assurance sidecar 与 shadow-Git 恢复边界。
- [`architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)：从 task contract、计划和 append-only journal 派生全局进度摘要与 WARN，要求模型结构化处置，降低单方向过推进和遗漏风险而不增加新协议状态。
- [`architecture/D_SALVAGE_MATRIX_v0.1.md`](architecture/D_SALVAGE_MATRIX_v0.1.md)：Google Drive 中 Project D 核心源码的 source ledger、采用/改造/拒绝裁决与安全发现。
- [`adr/ADR-0002-defer-cloud-runtime.md`](adr/ADR-0002-defer-cloud-runtime.md)：冻结云端执行/运维范围，保持 Windows 本地 runtime，并记录未来重启条件。
- [`integration/grok/grok-observed-plan-v0.1.schema.json`](integration/grok/grok-observed-plan-v0.1.schema.json)：不执行模型的 observed dry-run 计划格式；配套 PowerShell 生成器只冻结 provenance 与控制意图。
- [`integration/grok/grok-fake-provider-result-v0.1.schema.json`](integration/grok/grok-fake-provider-result-v0.1.schema.json)：Windows 临时防火墙、隔离环境、loopback 请求捕获与 credential-leak 检查的 conformance 结果格式。
- [`upstream/grok-build.lock.json`](upstream/grok-build.lock.json)：当前上游 build commit、`SOURCE_REV`、README digest 与职责所有权锁。
- [`integration/grok/deepseek-custom-model.example.toml`](integration/grok/deepseek-custom-model.example.toml)：不含凭据、不设默认模型的 DeepSeek discovery-only custom-model 模板。
- [`docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`](docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md)：官方 stable Windows binary 的体积、hash、签名、离线能力和未证明项审计。
- [`docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)：loopback 请求、临时防火墙、debug credential 泄漏发现、修复与 post-fix artifact ledger。
- [`docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`](docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md)：两轮 `read_file`、DeepSeek reasoning continuity、Job Object 与 streaming-json 工具事件缺口审计。
- [`architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`](architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md)：基于 DeepSeek 官方 API 文档的专项 adapter、thinking/tool-call、兼容层和重试边界。
- [`architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`](architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md)：Windows-first Job Object、进程树、取消、输出 drain 与 terminal-event 契约。
- [`architecture/ACTION_KERNEL_CONTRACT_v0.1.md`](architecture/ACTION_KERNEL_CONTRACT_v0.1.md)：将 manifest、journal、Windows runner、artifact 与 session 串联的 no-model action-kernel spike。
- [`architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`](architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md)：provider-neutral scripted transport、DeepSeek streaming/tool loop 与 private reasoning 落盘边界。
- [`architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`](architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md)：仅 127.0.0.1 的 HTTP/SSE transport、取消/timeout 与 Windows DPAPI transcript 恢复边界。
- [`architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`](architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md)：固定 DeepSeek HTTPS endpoint、一次性请求许可、Windows Credential Manager 与离线 fake-TLS 验证边界。
- [`architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`](architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md)：每轮/每次 retry 的独立 permit、脱敏确认摘要与三-attempt fake HTTPS 模型循环。
- [`architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`](architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md)：fake-only 逐 attempt digest 确认、append-only hash-chain ledger 与跨文件 verifier。
- [`protocol/PROTOCOL_DRAFT_v0.1.md`](protocol/PROTOCOL_DRAFT_v0.1.md)：协议语义、状态机、事件和独立性流程。
- [`protocol/agent-protocol-v0.1.schema.json`](protocol/agent-protocol-v0.1.schema.json)：语言无关的初步 JSON Schema。
- [`protocol/reason-codes-v0.1.yaml`](protocol/reason-codes-v0.1.yaml)：原因码语义、默认决策、误报保护和补救动作注册表。
- [`protocol/gate-matrix-v0.1.yaml`](protocol/gate-matrix-v0.1.yaml)：discussion/guarded/strict 三档门禁和阶段依赖。
- [`regression/case-corpus-v0.1.schema.json`](regression/case-corpus-v0.1.schema.json)：案例结构、状态词表和 historical/synthetic 分区约束。
- [`regression/fixture-v0.1.schema.json`](regression/fixture-v0.1.schema.json)：oracle-free 机器可读 fixture manifest 契约。
- [`regression/historical-excerpt-provenance-v0.1.schema.json`](regression/historical-excerpt-provenance-v0.1.schema.json)：reviewer-only 历史摘录的源文件、行段、归一化和 digest 契约。
- [`regression/fixtures/`](regression/fixtures/)：5 份 scenario-visible synthetic evidence packs，以及 5 份 reviewer-only 历史来源摘录；后者含纠正结论，禁止进入被测 Agent 上下文。
- [`regression/cases-v0.1.yaml`](regression/cases-v0.1.yaml)：30 个历史回归案例与 10 个合成反惯性案例。
- [`regression/CURATION_PROTOCOL_v0.1.md`](regression/CURATION_PROTOCOL_v0.1.md)：自动发现、人工归类、oracle 隔离与反惯性评测协议。
- [`regression/coverage-matrix-v0.1.yaml`](regression/coverage-matrix-v0.1.yaml)：10 个错误簇、challenge 类型、覆盖缺口与 holdout 前优先级。
- [`regression/coverage-matrix-v0.1.schema.json`](regression/coverage-matrix-v0.1.schema.json)：覆盖矩阵的机器约束。
- [`evaluation/SCORING_PROTOCOL_v0.1.md`](evaluation/SCORING_PROTOCOL_v0.1.md)：cluster-macro、多维评分、红线、盲审和阈值校准协议。
- [`evaluation/evaluation-result-v0.1.schema.json`](evaluation/evaluation-result-v0.1.schema.json)：不含单一总分的评测结果格式。
- [`evaluation/example-evaluation-result-v0.1.json`](evaluation/example-evaluation-result-v0.1.json)：仅用于 schema 自证的非运行示例。
- [`evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`](evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md)：evaluation/holdout 的物理隔离、角色分离、污染和生命周期协议。
- [`evaluation/partition-manifest-v0.1.schema.json`](evaluation/partition-manifest-v0.1.schema.json)：密封分区 digest、状态和暴露策略契约；当前没有真实实例。
- [`evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md`](evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md)：双人独立盲审、adjudication 和阈值校准协议。
- [`runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md`](runtime/SCENARIO_EXPORT_AND_RUNNER_CONTRACT_v0.1.md)：scenario 裁剪、匿名化、leak gate、model adapter、tool broker 与可重放 runner 契约。
- [`runtime/scenario-export-manifest-v0.1.schema.json`](runtime/scenario-export-manifest-v0.1.schema.json)：场景包、opaque token、文件 digest 和导出策略 schema。
- [`runtime/scenario-instance-v0.1.schema.json`](runtime/scenario-instance-v0.1.schema.json)：被测 Agent 可见的最小场景实例 schema；排除 oracle、内部 case ID 和来源路径。
- [`runtime/leak-scan-report-v0.1.schema.json`](runtime/leak-scan-report-v0.1.schema.json)：机械/语义泄漏检查、人工签发与 fail-closed 决策 schema。
- [`runtime/run-manifest-v0.1.schema.json`](runtime/run-manifest-v0.1.schema.json)：不可变运行前置条件 schema。
- [`runtime/run-event-v0.1.schema.json`](runtime/run-event-v0.1.schema.json)：append-only、hash-chained JSONL event schema。
- [`runtime/examples/`](runtime/examples/)：仅用于 schema 自证的零 digest 示例，不代表真实导出或运行。
- [`prototype/`](prototype/)：一次性 Python conformance fixtures；固定 LIF schema、DeepSeek thinking continuity、redaction 与 Windows 进程边界，不是待扩展的生产 runner。
- [`scripts/check_repository.py`](scripts/check_repository.py)：CI 使用的仓库级机械完整性检查；验证 schema、corpus、coverage、fixture digest、交叉引用和本地文档链接。
- [`docs/REPOSITORY_AUDIT_2026-07-21.md`](docs/REPOSITORY_AUDIT_2026-07-21.md)：迁移后审计、初始提交边界和下一阶段 session-validator spike 范围。

## 证据路由

迁移前的设计遵循旧研究工作区的优先顺序：

1. `LIF_CURRENT_INDEX.md`
2. `fep_env_research.md`
3. 当前 MAP
4. 对应 R/JSON/log/code
5. `self_check_protocol.md`

这些旧工作区入口不属于本独立仓库，也不需要迁入。普通 CLI 工程工作不得依赖它们；只有具体 LIF claim-bearing 任务才按需跨目录读取原文件，并在当次 source ledger 中记录绝对路径与内容 hash。本仓库中的设计摘要不能替代这些证据。本仓库是新设计提案，不是现有实验结论入口，也不替代 MAP、INDEX、R 文档或原始产物。

## 当前边界

- 正式通用 runtime 采用 Grok Build，不再在本仓库复刻 model/session/tool/permission/sandbox 栈。
- 当前包含 development-only disposable conformance fixtures、zero-model dry-run 和 loopback fake-provider launcher；没有真实 provider 模型调用、评分或生产 runner。
- 不决定全部使用 Rust；只把 Rust 视为控制面的候选实现语言。
- 不把历史案例直接作为模型提示词答案。
- 不自动修改 MAP/INDEX。
- D 的候选机制已完成定向源码 salvage；云端产品面依 ADR-0002 延期，未核本地 body 不作为当前依赖。

## 当前设计冻结点

- 自动分类器只能提出候选，不能自动写入 canonical label。
- 默认 CLI 模式为 guarded；机械错误可 block，证据不足通常 defer dependent claim。
- 用户授权可以解决权限问题，但不能把不独立、混淆或无来源的证据改标为合格。
- 项目历史案例只能在独立初判后检索，并必须与结论相反的 countercase 共同评测。
- 当前案例全属 development/challenge seed corpus，不宣称为未泄漏 holdout 成绩。
- 当前 10 个错误簇均有历史检测案例和至少一个 challenge，7 个高频簇均有 permission-reversal challenge；但尚无 evaluation/holdout，评测阈值也未校准。
- 5 个历史案例已有 source-hashed reviewer-only excerpt fixture；它们只提高 curation provenance 可审计性，不提高未见评测覆盖。
- Grok Build 是正式通用 CLI/runtime 所有者；项目差异层冻结为 SourceRouter、EvidenceKernel、ValidatorBridge、ScenarioExporter、LeakScanner、EvaluationRunner 和窄 Windows/DeepSeek bridge，不扩张通用产品功能面。
