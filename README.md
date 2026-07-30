# Scientific-Assurance Agent CLI（LIF-first）

状态：runtime-neutral、Windows-first 的通用科学保障工程，LIF 是首个领域 profile；P0 合同、P1 会话身份/归档删除、
P2 Docker strict sandbox、P2.5 无模型 guarded execution、P3 指令来源/能力继承、
P4 metadata-only 审计/compaction/恢复授权，以及 P4.5 workspace-first 集成链
development 纵向切片已完成；P5 内部合成任务与 LIF 复杂任务只读投影 mechanical preflight
已启动并通过；GSA-CORE 已新增首个非 LIF、多文件、确定性只读审查切片。标准模式默认使用受限工作区内的固定动作；Docker 只保留为显式、
按需的 strict backend，strict 请求绝不静默降级。Windows native strict backend 已具备 AppContainer
profile/observation/verifier 与 live probe：本机已 observed 进程/文件/注册表隔离，但 raw TCP
出站在空 capability AppContainer 下仍可能成功，因此 outcome 保持 noncompliant，selection 不 allow。Grok Build 是当前第一生产 runtime 底座候选，
`0.2.112 (9bbd559437)` 已完成本地锁定、离线核验与 promotion gate；`windows_child_tree_timeout` 仅作为 carried-forward limitation 记录。首个 Codex app-server 双向 ordered-capture lifecycle normalizer
已完成 no-model 离线实现、独立重放与隔离 live app-server smoke；live 路径只执行 initialize 和
ephemeral/read-only thread start，不发送 turn 或模型输入。独立 DeepSeek one-shot transport conformance 与一次固定、单轮、零工具的 Grok→DeepSeek
真实模型会话均已成功。首次 Grok terminal artifact scan 因内置帮助文档的 Bearer 占位符
误报而 fail closed；扫描器已离线修复并通过正反例验证，未自动补跑第二次付费请求。首个 GSA
disposable reproduction 切片已完成 no-model manifest、固定 replay、receipt/verifier/run-proof、runtime preflight
projection、一次性 runtime JSONL journal write/replay、development code/environment lock、no-model runner skeleton、runner
journal sidecar lock、只读 recovery inspection、受控 repair/quarantine receipt 与 lifecycle repair policy
和“不增加独立证据/不提升 claim 强度”边界。中性 orientation checkpoint、独立 counterexample queue
与 runtime stagnation guard 已形成分离设计稿；首个 no-model/read-only fixture、runner 接入前置夹具、
runtime event/journal projection、公开输出抽取层、一次 direct DeepSeek API one-shot 观测投影管线，以及
DeepSeek-shaped streaming/repetition fixture 已实现；真实 API 观测只验证公开输出抽取与 guard 管线接线，streaming
fixture 只验证 DeepSeek 风格 chunk 的边界映射，尚未接入真实 runner。
2026-07-27 起，canonical guarded CLI offline 主路径已扩展为：instruction provenance gate → tool availability
gate → orientation checkpoint → source visibility gate → fake DeepSeek-shaped adapter boundary → answer packet →
runtime JSONL journal → independent verifier。配套新增工具可用性探测/信念停滞 guard、检索子代理 no-model
fixture，以及 Windows native sandbox 合同层；这些均不调用真实模型、不联网、不读取 credential。
P0.5 顶层入口已固定为 `python gsa.py doctor/source gate/run/verify`；`run` 支持 `--ask` 生成冻结 task contract，
也支持 `--task` 复用既有合同。`gsa grok doctor`、`gsa grok run --mode version-smoke` 与 `gsa tui --runtime grok --run version-smoke` 已接入 Grok first slice；默认 `gsa run` 仍保留 canonical fixture/assurance path。

本仓库记录一个不绑定单一 runtime 的本地 Agent 保障层。它从 FEP/LIF 工作流暴露的问题出发，但把
来源先行、证据分层、独立性、反例搜索、机械验证和 claim 边界抽为通用科学保障能力，目标是让接入的模型
尽可能遵守来源先行、Ask-Don't-Guess、证据分层、机械验证、全程留痕和独立复核，从而降低幻觉补全、过度推进与
错误 claim promotion 的风险。它不能保证模型输出必然科学正确，也不把 LIF/FEP 理论本身实现为 Agent 控制算法。

## 产品定位

- **runtime-neutral / runtime-first**：通用 model/tool/session runtime 由通过 capability gate 的外部框架提供；Grok Build
  是当前第一生产底座候选与默认锁定 runtime，其他框架保留为部件级设计参考。
- **核心差异**：`general-science` profile 负责领域无关的来源/证据/claim 边界、研究生命周期、
  validator、场景导出和评测隔离；`lif-research` 只添加 LIF 当前来源路由与 validator 增量。
- **多源借鉴**：Grok、Codex CLI、Gemini CLI、Claude Code、Goose 与 OpenCode 均可提供设计参考；新增
  runtime adapter 必须独立通过同类门禁，不继承其他框架的 PASS。
- **上游策略**：每个实际 adapter 各自维护 observed baseline、candidate 和 promotion gate；reference
  身份不能代替 observed acceptance。
- **研究边界**：旧研究工作区的 INDEX/MAP/self-check 不迁入本仓库，只在具体 LIF claim-bearing 任务中按需
  跨目录读取并登记来源。

完整裁决见
[`adr/ADR-0003-runtime-neutral-assurance-kernel.md`](adr/ADR-0003-runtime-neutral-assurance-kernel.md) 与
[`adr/ADR-0004-general-science-profile-layering.md`](adr/ADR-0004-general-science-profile-layering.md)、
[`architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)。

## 当前文件

- [`adr/ADR-0001-evidence-constrained-local-agent-kernel.md`](adr/ADR-0001-evidence-constrained-local-agent-kernel.md)：产品边界、核心不变量、案例库和反捷径决策。
- [`adr/ADR-0003-runtime-neutral-assurance-kernel.md`](adr/ADR-0003-runtime-neutral-assurance-kernel.md)：冻结 runtime-neutral 所有权、capability-gated 选择与 Grok `reference_only` 边界。
- [`adr/ADR-0004-general-science-profile-layering.md`](adr/ADR-0004-general-science-profile-layering.md)：冻结 `general-science → lif-research` 的只增不减分层和通用测试不得使用 LIF 内部任务的隔离边界。
- [`assurance/README.md`](assurance/README.md)：P0–P5 runtime-neutral 合同与 development fixture，包括会话生命周期、workspace-first/Docker backend policy、guarded execution、指令来源、能力子集、metadata audit、恢复授权和合成用户任务预检。
- [`assurance/source-visibility-ledger-v0.1.schema.json`](assurance/source-visibility-ledger-v0.1.schema.json)：外部文献、帖子和网页引用后的全文可见性 ledger；配套 gate receipt 会按 claim 类型要求 metadata/partial/full-text 状态并输出 allow/defer/block。
- [`assurance/canonical_cli.py`](assurance/canonical_cli.py)：canonical guarded CLI 的 offline/fake 纵向路径；串联 run manifest、instruction provenance gate、tool availability gate、orientation checkpoint、source visibility gate、fake adapter boundary、answer packet、runtime journal 和独立 verifier。
- [`gsa.py`](gsa.py)：P0.5 顶层 CLI dispatcher；提供 `doctor`、`source gate`、`run` 和 `verify`。
- [`assurance/task-contract-v0.1.schema.json`](assurance/task-contract-v0.1.schema.json)：冻结单次 CLI run 的任务合同；记录原始问题、source ledger、权限、claim 上限和输出要求。
- [`assurance/instruction_provenance_gate.py`](assurance/instruction_provenance_gate.py)：入口级指令来源/反注入门禁（GAK-INJ-001 offline）；在模型调用前对多来源指令做 batch 分类与 block/defer/allow。
- [`assurance/tool_availability_gate.py`](assurance/tool_availability_gate.py)：工具可用性机械探测与 gate receipt；生成必须注入模型上下文的 AVAILABLE/UNAVAILABLE 列表，并检测工具信念错配/停滞。
- [`assurance/retrieval_subagent.py`](assurance/retrieval_subagent.py)：检索子代理 no-model fixture；冻结委托合同、结构化结果、来源可见性校验与关闭不清零协议。
- [`assurance/windows_sandbox.py`](assurance/windows_sandbox.py)：Windows native AppContainer + Job Object 探针与 candidate builder；live probe 通过前 selection 保持 noncompliant。
- [`docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md`](docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md)：当前 offline/fake CLI 入口的最短使用说明。
- [`docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`](docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md)：首个 canonical guarded CLI offline 主路径审计；确认 gate-before-model、fake no-network adapter、answer packet 与 verifier 边界。
- [`docs/GAK_INJ_001_AUDIT_2026-07-27.md`](docs/GAK_INJ_001_AUDIT_2026-07-27.md)：instruction provenance gate offline 切片与 canonical CLI 接入状态。
- [`docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md`](docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md)：工具可用性门禁、belief mismatch/stagnation 与 orientation 注入审计。
- [`docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`](docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md)：检索子代理合同/结果/关闭协议 no-model 审计。
- [`docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`](docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md)：Windows native sandbox 合同与 verifier；live probe 仍为关闭条件。
- [`docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md`](docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md)：首个非 LIF 通用科学多文件只读闭环、claim 强度门禁、正反回归与保留缺口。
- [`docs/GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md`](docs/GSA_VALIDATOR_BRIDGE_AUDIT_2026-07-25.md)：版本化 validator registry、设计/动作 schema、artifact finite gate 与基础统计报告完整性边界。
- [`docs/GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md`](docs/GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT_2026-07-25.md)：artifact schema registry、结果摘要绑定、不可弱化映射与首个数值/统计基础 schema。
- [`docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md`](docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md)：跨 artifact JSON Pointer、producer/source lineage、比较不变量和 permission-reversal 门禁。
- [`docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md`](docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md)：首个 no-model disposable reproduction manifest、固定 replay、receipt/verifier/run-proof/runtime projection、一次性 runtime journal write/replay、execution lock、no-model runner skeleton、runner journal lock/recovery repair receipt、lifecycle repair policy 与“不增加独立证据”边界。
- [`docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`](docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md)：文献、帖子、网页和 thread 检索时必须回报全文可见性状态；未完整抓取并浏览全文时明确标注 partial/metadata-only/unavailable，禁止摘要或片段冒充全文证据。
- [`docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`](docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md)：中性 orientation checkpoint、独立 counterexample queue，以及 DeepSeek/Grok 等 runtime 高重复输出截断与重启的设计和首个只读 fixture。
- [`assurance/orientation_runtime_guard.py`](assurance/orientation_runtime_guard.py)：生成中性 orientation checkpoint、验证其不产生反例/claim disposition，并用公开输出重复阈值生成 runtime stagnation guard receipt。
- [`assurance/orientation_runtime_integration.py`](assurance/orientation_runtime_integration.py)：no-model 接入前置夹具；从 fixture input 写出 orientation/stagnation 两个 artifact 和汇总 receipt，并由 verifier 独立重建。
- [`assurance/orientation_runtime_journal.py`](assurance/orientation_runtime_journal.py)：no-model runtime event/journal projection；生成 `orientation_checkpoint` 与 `runtime_stagnation_guard` 事件并验证 JSONL hash-chain。
- [`assurance/runner_public_output.py`](assurance/runner_public_output.py)：真实 runner 前的公开输出抽取夹具；只把 public assistant 输出送入 `public_outputs`，私有 reasoning/redacted metadata 只能保留 digest 且不得复制文本。
- [`assurance/deepseek_api_observation.py`](assurance/deepseek_api_observation.py)：direct DeepSeek API one-shot 观测投影；消费单次受控 API result，把公开 assistant 输出接入 extraction、orientation/stagnation 和 runtime journal，不保存 credential、raw response 或隐藏 reasoning 文本。
- [`assurance/deepseek_stream_observation.py`](assurance/deepseek_stream_observation.py)：DeepSeek-shaped streaming fixture；把 public `delta.content`、private reasoning digest 和 terminal metadata 投影到 runner output stream，并用重复 public delta 验证 restart projection。
- [`scripts/invoke_deepseek_public_output_observation.ps1`](scripts/invoke_deepseek_public_output_observation.ps1)：两阶段真实 DeepSeek API 观测 launcher；先生成离线 plan，执行时从 Windows Credential Manager `FEP-Agent/DeepSeek` 读取凭据并只发起一次固定 marker 请求。
- [`docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md`](docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md)：P2 Docker 实测、Windows native fail-closed 状态、限制与正反例收敛策略。
- [`docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md`](docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md)：P1 envelope、P2 selector、无模型 action、宿主/容器进程追踪、HMAC 回执与残留复核的端到端实测。
- [`docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md`](docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md)：P3 来源不可提权、内核动作授权、能力子集委派与 digest-bound 一次性许可的测试和限制。
- [`docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md`](docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md)：P4 runtime-neutral metadata ledger、归档后复核、compaction provenance 与恢复候选/授权分离。
- [`docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md`](docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md)：P4.5 标准模式 workspace-first 策略、P3→固定动作→P4→archive 实测与无 Docker/子进程残留复核。
- [`docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md`](docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md)：P5 两项合成任务、R211 复杂任务只读投影、LIF 源文件不变证明及人类可用性未评估边界。
- [`docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`](docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md)：将本轮缺口登记为通用科学保障 backlog，并明确 LIF 内部任务不得作为通用复杂测试或 holdout。
- [`architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`](architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md)：多源借鉴分工、通用科学保障与 LIF 领域增量分层，以及未来 LIF-informed Agent 想法的隔离边界。
- [`architecture/GROK_BUILD_ADAPTATION_v0.1.md`](architecture/GROK_BUILD_ADAPTATION_v0.1.md)：基于官方开源快照的 adopt/adapt/defer/reject 矩阵与项目专化层。
- [`architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`](architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md)：Grok reference adapter 的 Windows 集成范围，以及现有 prototype 的降级分类。
- [`architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md`](architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md)：将可复现实测 baseline 与当前上游 candidate 分离，以 conformance gate 选择更优版本而非永久锁死。
- [`architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`](architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md)：优先准确性的全程可观测 wrapper、记录分层、资源取舍与反补全约束。
- [`architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`](architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md)：对 Grok ACP、Codex、Gemini CLI、OpenCode、Goose 与 Cline 的职责深拆；把通用 runtime 交给可替换的外部框架，只保留保障层与 shadow-Git 恢复边界。
- [`architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`](architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md)：将能力拆为通用 Assurance Kernel、`general-science` 与 LIF 领域 profile，登记安全、审计和用户测试缺口。
- [`architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`](architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md)：从 task contract、计划和 append-only journal 派生全局进度摘要，并以跨检查点方向预算、证据停滞、延期债务和整体完成门降低单方向过推进与局部完成误判风险。
- [`architecture/D_SALVAGE_MATRIX_v0.1.md`](architecture/D_SALVAGE_MATRIX_v0.1.md)：Google Drive 中 Project D 核心源码的 source ledger、采用/改造/拒绝裁决与安全发现。
- [`adr/ADR-0002-defer-cloud-runtime.md`](adr/ADR-0002-defer-cloud-runtime.md)：冻结云端执行/运维范围，保持 Windows 本地 runtime，并记录未来重启条件。
- [`integration/grok/grok-observed-plan-v0.1.schema.json`](integration/grok/grok-observed-plan-v0.1.schema.json)：不执行模型的 observed dry-run 计划格式；配套 PowerShell 生成器只冻结 provenance 与控制意图。
- [`integration/grok/grok-fake-provider-result-v0.1.schema.json`](integration/grok/grok-fake-provider-result-v0.1.schema.json)：Windows 临时防火墙、隔离环境、loopback 请求捕获与 credential-leak 检查的 conformance 结果格式。
- [`upstream/grok-build.lock.json`](upstream/grok-build.lock.json)：当前上游 build commit、`SOURCE_REV`、README digest 与职责所有权锁。
- [`integration/grok/deepseek-custom-model.example.toml`](integration/grok/deepseek-custom-model.example.toml)：不含凭据、不设默认模型的 DeepSeek discovery-only custom-model 模板。
- [`docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`](docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md)：官方 stable Windows binary 的体积、hash、签名、离线能力和未证明项审计。
- [`docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`](docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md)：loopback 请求、临时防火墙、debug credential 泄漏发现、修复与 post-fix artifact ledger。
- [`docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`](docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md)：两轮 `read_file`、DeepSeek reasoning continuity、Job Object 与 streaming-json 工具事件缺口审计。
- [`docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`](docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md)：锁定 Windows binary 的 ACP protocol v1/capability/扩展通知无模型实测、独立验证与下一步 fake-tool 边界。
- [`docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`](docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md)：ACP fake tool allow/cancel 双场景的 Windows 实测、三路证据对账、取消竞态与独立 verifier。
- [`docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md`](docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md)：Grok `0.2.111` 的官方发现、本地身份、ACP/DeepSeek fake-only 对照与已完成的 promotion gate。
- [`docs/DEEPSEEK_REAL_DEVELOPMENT_PROBE_2026-07-23.md`](docs/DEEPSEEK_REAL_DEVELOPMENT_PROBE_2026-07-23.md)：一次性真实 DeepSeek probe 的两阶段入口、离线测试、首个 real transport attempt 与 fail-closed 结果。
- [`docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md`](docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md)：WER `NOHEAP`、短进程、固定 endpoint、无 proxy/redirect/debug、失败 artifact 与无真实 key 模式扫描的安全边界。
- [`docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`](docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md)：Grok Build 两阶段真实 DeepSeek 薄 launcher、Credential Manager 内部注入、Job Object、双 trust receipt 与离线 fail-closed 证据。
- [`docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md`](docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md)：将默认 Grok 从 `0.2.106` 提升到 `0.2.111` 的独立裁决与回退锚点。
- [`docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`](docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md)：`run_terminal_command` timeout、background task cancel 与 parent-exit 三场景的 Windows 子进程树门禁及已完成的管理员 observed 矩阵。
- [`docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md`](docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md)：Grok `0.2.111` 手工 compaction 的 `PreCompact`/`PostCompact`、source span/digest、checkpoint、`derived_unverified` 摘要边界与独立 verifier。
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
- [`runtime/global-progress-review-v0.1.schema.json`](runtime/global-progress-review-v0.1.schema.json)：由 task contract、计划和 journal 机械派生的全局进度、方向覆盖、验收覆盖与三类 WARN。
- [`runtime/global-progress-disposition-v0.1.schema.json`](runtime/global-progress-disposition-v0.1.schema.json)：模型对每条全局 WARN 的显式处置，以及 `continue/pivot/defer/stop/replan` 决策边界。
- [`runtime/fixtures/global-progress-sentinel-v0.1/`](runtime/fixtures/global-progress-sentinel-v0.1/)：六方向 no-model 输入与 `reasoned_continue`/`replan` 两种合法处置 fixture。
- [`runtime/global-progress-history-input-v0.1.schema.json`](runtime/global-progress-history-input-v0.1.schema.json)：跨检查点 hash-chain 投影，记录已观测方向动作数、证据新颖性、关键延期和整体验收状态；不估算隐藏 token/努力。
- [`runtime/global-progress-holistic-review-v0.1.schema.json`](runtime/global-progress-holistic-review-v0.1.schema.json)：方向预算占比、同方向空转、关键延期债务和整体完成资格的确定性派生视图。
- [`runtime/global-progress-holistic-disposition-v0.1.schema.json`](runtime/global-progress-holistic-disposition-v0.1.schema.json)：集中推进的有界许可，要求关键路径、退出条件、追加动作上限、下一复查周期和受影响关键方向。
- [`runtime/fixtures/global-progress-holistic-v0.1/`](runtime/fixtures/global-progress-holistic-v0.1/)：四检查点 no-model 历史，覆盖四种整体性失效模式及有界聚焦 permission reversal。
- [`runtime/global-progress-checkpoint-policy-v0.1.schema.json`](runtime/global-progress-checkpoint-policy-v0.1.schema.json)：冻结 task/contract 绑定、critical direction 与 acceptance 映射，防止跨周期静默缩小整体范围。
- [`scripts/build_global_progress_checkpoint.py`](scripts/build_global_progress_checkpoint.py)：从 GPS input/review/disposition/verification、policy 和可选 focus artifact 链机械派生 checkpoint，并对动作上限/复查期限超限返回阻断状态。
- [`scripts/verify_global_progress_checkpoint.py`](scripts/verify_global_progress_checkpoint.py)：重新运行上游独立 verifier、重建 checkpoint、检查 hash/provenance/policy continuity 与 focus 消耗。
- [`scripts/build_global_progress_transition_event.py`](scripts/build_global_progress_transition_event.py)：将 checkpoint verification 投影为 hash-chained `gate_decision` run event；阻断时保持原状态并保留 receipt。
- [`scripts/append_global_progress_transition_event.py`](scripts/append_global_progress_transition_event.py)：在共享跨进程锁内，只把能精确延伸 run/manifest/sequence/hash chain 的 transition event 追加到 JSONL journal，并在追加后重放。
- [`scripts/recover_torn_journal.py`](scripts/recover_torn_journal.py)：默认只读检查 journal；仅在显式 `--apply`、有效前缀可独立重放且损坏限于无换行尾部时执行锁内恢复，并生成 quarantine 与恢复收据。
- [`scripts/run_global_progress_controller.py`](scripts/run_global_progress_controller.py)：no-model disposable controller；组合 transition builder、幂等锁内 append、journal 恢复分类与状态投影，但不自动修复 journal。
- [`scripts/verify_global_progress_controller_receipt.py`](scripts/verify_global_progress_controller_receipt.py)：只读重建 controller 候选结果，并交叉检查 source digests、当前 journal 快照、事件唯一性与状态投影。
- [`scripts/reduce_global_progress_state.py`](scripts/reduce_global_progress_state.py)：在共享锁内从完整 journal 归约 controller 状态；`run_started` 唯一锚定 `executing`，后续 transition 必须保持 task/state 连续，并输出最近一次 gate-accepted checkpoint binding。
- [`scripts/append_cli_session_lifecycle_event.py`](scripts/append_cli_session_lifecycle_event.py)：有限、厂商无关的 CLI lifecycle ingress；把已标准化且有序的 session/turn/terminal observation 原子映射为 canonical run event，并复用 GPS journal-derived 完成态。
- [`scripts/verify_cli_session_lifecycle_receipt.py`](scripts/verify_cli_session_lifecycle_receipt.py)：只读回放 observation、adapter receipt 与当前 journal，检查快照、事件唯一性和 lifecycle projection。
- [`runtime/cli-session-lifecycle-observation-v0.2.schema.json`](runtime/cli-session-lifecycle-observation-v0.2.schema.json)：区分 completed/interrupted/failed turn terminal，并把 turn terminal 与 session terminal 分离的标准化 observation。
- [`scripts/capture_codex_app_server_lifecycle.py`](scripts/capture_codex_app_server_lifecycle.py)：在 Windows Job Object/POSIX process group 内启动显式 app-server executable，只执行 initialize 与 ephemeral/read-only thread start，并原子输出双向 ordered capture、独立 stderr 和 digest-bound receipt。
- [`scripts/verify_codex_app_server_lifecycle_capture.py`](scripts/verify_codex_app_server_lifecycle_capture.py)：独立核对 capture/receipt/stderr、连续 sequence、thread 身份、containment 和 no-turn/no-input 边界。
- [`scripts/normalize_codex_app_server_lifecycle.py`](scripts/normalize_codex_app_server_lifecycle.py)：只读消费可信 supervisor 的单一双向有序 Codex app-server capture，用 turn/start request/response 证明 turn→thread 绑定，再把 thread/turn/close 通知投影为 lifecycle v0.2 observation；忽略正文类 item/delta，EOF 保持 partial。
- [`scripts/verify_codex_app_server_lifecycle.py`](scripts/verify_codex_app_server_lifecycle.py)：独立重放 capture，核对 source/observation digest、状态投影、内容省略和 receipt。
- [`runtime/fixtures/codex-app-server-lifecycle-v0.1/`](runtime/fixtures/codex-app-server-lifecycle-v0.1/)：happy、interrupted、failed、missing-terminal 与 sequence/thread/truncation 反例 capture。
- [`protocol/global-progress-reason-code-migration-v0.1.yaml`](protocol/global-progress-reason-code-migration-v0.1.yaml)：GPS runtime-local control code 到冻结 protocol registry 的设计期迁移登记；候选码不视为已注册 reason code。
- [`runtime/examples/`](runtime/examples/)：仅用于 schema 自证的零 digest 示例，不代表真实导出或运行。
- [`prototype/`](prototype/)：一次性 Python conformance fixtures；固定 LIF schema、DeepSeek thinking continuity、redaction 与 Windows 进程边界，不是待扩展的生产 runner。
- [`scripts/check_repository.py`](scripts/check_repository.py)：CI 使用的仓库级机械完整性检查；验证 schema、corpus、coverage、fixture digest、交叉引用和本地文档链接。
- [`scripts/invoke_grok_acp_fake_tool_probe.ps1`](scripts/invoke_grok_acp_fake_tool_probe.ps1)：管理员级 fake-only ACP tool/permission/cancel continuity launcher；synthetic/tamper tests 与 Windows live smoke 均已通过。
- [`scripts/invoke_grok_windows_child_tree_probe.ps1`](scripts/invoke_grok_windows_child_tree_probe.ps1)：管理员级 fake-only Windows root/child/grandchild containment launcher；显式接受 baseline/candidate release metadata，不自动提升默认版本。
- [`docs/REPOSITORY_AUDIT_2026-07-21.md`](docs/REPOSITORY_AUDIT_2026-07-21.md)：迁移后审计、初始提交边界和下一阶段 session-validator spike 范围。
- [`docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md`](docs/GLOBAL_PROGRESS_SENTINEL_SPIKE_2026-07-21.md)：no-model 全局回看生成、独立重建、合法处置、去重与篡改失败的实测记录。
- [`docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md)：跨检查点整体性门禁、正反处置、hash-chain 篡改与保留边界的实测记录。
- [`docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md)：来源绑定 checkpoint adapter、上游 verifier 重算、policy continuity 与有界聚焦实际消费核算记录。
- [`docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`](docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md)：journal-derived 状态权威、初态锚点、锁内连续性校验及伪造状态反例记录。
- [`docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md`](docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md)：有限成熟 CLI 接入面、生命周期映射、厂商 normalizer 边界及 receipt 独立复核记录。
- [`docs/CLI_LIFECYCLE_SOURCE_COMPARISON_AND_CODEX_DIRECTION_2026-07-25.md`](docs/CLI_LIFECYCLE_SOURCE_COMPARISON_AND_CODEX_DIRECTION_2026-07-25.md)：Codex、Gemini、Qwen 与 Copilot lifecycle surface 比较，以及首个 Codex app-server read-only normalizer 的冻结方向和离线实现。
- [`docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`](docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md)：ordered capture、正文省略、partial/terminal 语义、独立 verifier 与 canonical adapter 端到端实测。

## 证据路由

迁移前的设计遵循旧研究工作区的优先顺序：

1. `LIF_CURRENT_INDEX.md`
2. `fep_env_research.md`
3. 当前 MAP
4. 对应 R/JSON/log/code
5. `self_check_protocol.md`

这些旧工作区入口不属于本独立仓库，也不需要迁入。普通 CLI 工程工作不得依赖它们；只有具体 LIF claim-bearing 任务才按需跨目录读取原文件，并在当次 source ledger 中记录绝对路径与内容 hash。本仓库中的设计摘要不能替代这些证据。本仓库是新设计提案，不是现有实验结论入口，也不替代 MAP、INDEX、R 文档或原始产物。

## 当前边界

- 正式通用 runtime 由通过门禁的外部框架提供；Grok Build 是当前第一生产底座候选。本仓库不复刻
  model/session/tool/permission 栈。P2 只实现 runtime-neutral sandbox launcher/verifier，不实现平行 runtime。
- 当前包含 development-only disposable conformance fixtures、zero-model dry-run、loopback fake-provider launcher、已成功的显式确认/零重试 DeepSeek one-shot probe，以及一次成功完成的固定 Grok→DeepSeek 真实模型会话；尚未执行任意 prompt、真实 workspace、评分或生产 Agent session，修复后的 terminal `result.json` 路径也尚未用第二次真实请求复验。
- 不决定全部使用 Rust；只把 Rust 视为少量保障组件与 Windows 进程边界的候选实现语言。
- 不把历史案例直接作为模型提示词答案。
- 不自动修改 MAP/INDEX。
- D 的候选机制已完成定向源码 salvage；云端产品面依 ADR-0002 延期，未核本地 body 不作为当前依赖。
- “让 LIF/FEP 原理参与 Agent 控制或规划”是单独的 deferred research concept，不属于当前保障层语义。

## 当前设计冻结点

- 自动分类器只能提出候选，不能自动写入 canonical label。
- 默认 CLI 模式为 guarded；机械错误可 block，证据不足通常 defer dependent claim。
- 用户授权可以解决权限问题，但不能把不独立、混淆或无来源的证据改标为合格。
- 项目历史案例只能在独立初判后检索，并必须与结论相反的 countercase 共同评测。
- 当前案例全属 development/challenge seed corpus，不宣称为未泄漏 holdout 成绩。
- 当前 10 个错误簇均有历史检测案例和至少一个 challenge，7 个高频簇均有 permission-reversal challenge；但尚无 evaluation/holdout，评测阈值也未校准。
- 5 个历史案例已有 source-hashed reviewer-only excerpt fixture；它们只提高 curation provenance 可审计性，不提高未见评测覆盖。
- 实际选中的外部 runtime 是通用 model/tool/session 能力所有者；`general-science` 冻结为
  SourceRouter、EvidenceKernel、ClaimBoundary、ResearchLifecycle、ValidatorBridge、ScenarioExporter、
  LeakScanner 和 EvaluationRunner；`lif-research` 只添加当前来源路由与 validator 增量。Grok 仅作为
  当前 reference adapter，不扩张通用产品功能面。

## License

This project is licensed under the Apache License, Version 2.0. See [`LICENSE`](LICENSE) and [`NOTICE`](NOTICE).
