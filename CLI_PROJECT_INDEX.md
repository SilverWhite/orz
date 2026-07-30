# CLI_PROJECT_INDEX

**更新**: 2026-07-30 (runtime-first 方向校正 + agent 底座嫁接裁决；Grok first slice 已提交 `07c136d`)
**定位**: GSA (General Scientific Assurance) 项目主召回索引 / 组件路由。本文收录**项目架构、P 级合约、Gate 链路、审计文档、Schema 体系、运行时集成、运行时所有权和关键设计约束**的召回入口，目标是让后续开发与回查可便捷定位到正确的文档、源码或 Schema。
**本文不替代审计文档、架构文档、Schema 定义或源代码**；它只负责召回和路由，不负责完整证明。

使用目标:

1. 在新增模块、修改 P 级合约、引入新 Gate 或变更设计约束前，先做 **prior-existence scan**：用关键词、别名、模块名、Gate ID、P 级编号查本文，判断是否已有同义条目、冲突条目、撤回链或废弃条目。
2. 若命中已有条目，沿用其 ID、状态和入口，由入口回归原文/源码进行检查确定，再写结论。
3. 若未命中，可标记为新条目；新增模块/文档后回填本文的短索引项。

回查顺序:

1. 本文：找 ID、别名、状态和入口。
2. 对应审计文档 (`docs/`)：核对具体审计结论、参数、边界条件。
3. 对应架构文档 (`architecture/`)：核对设计意图、合约边界。
4. 对应 Schema 文件 (`assurance/**/*.schema.json`, `runtime/**/*.schema.json`)：核对字段定义与约束。
5. 对应源代码 (`assurance/*.py`)：核对实际实现。
6. ADR (`adr/`)：核对不可变的架构决策记录。

推荐条目格式:

```text
- **ID** (来源/日期, 可选状态): 一句话定义。别名/关键词: xxx, yyy。状态: 事实/设计约束/待实施/撤回/废弃。注意: 主要限制或常见误用。入口: 文件名 / schema 路径 / 源码位置。
```

字段要求:

- **ID**: 优先沿用既有 ID（如 P 级编号、Gate ID）；同一概念改名时保留旧称提示。
- **一句话定义**: 只写可召回核心，不写完整论证。
- **别名/关键词**: 必须覆盖常见换名、误名、英文名和容易混淆的旧词。
- **状态**: 明确区分事实、设计约束、待实施、撤回、废弃。
- **注意**: 写最容易被误用的边界。
- **入口**: 需要精准回指到审计文档、架构文档、Schema 文件或源码位置。

维护规则:

- 本文尽量覆盖全部关键组件和设计约束，但每条保持短索引形态；长解释、完整论证放入审计/架构文档。
- 新审计文档、新模块、重大撤回或术语改名时，更新对应入口即可。
- 本文可以密集，但必须可扫描：ID 加粗、状态清楚、入口明确。

---

## 主题路由 (Topic Routing)

> 按组件领域组织的完整索引。查概念时先从此处浏览或搜索关键词。
> 每条保持短索引形态（≤2行），提供 ID、状态、别名和精准入口。

### A0. 当前方向裁决 (Runtime-First Direction)

- **Runtime-First Graft Decision** (2026-07-30, 设计约束): 通用 agent runtime 不再由本仓库自建；Grok CLI / Grok Build 是第一生产底座候选，本仓库保留 assurance、evidence、UI、adapter、fixture 和审查模式。别名: agent 底座嫁接, runtime-first, ownership correction。入口: `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Agent Base Component Adoption Matrix** (2026-07-30, 设计约束): 部件级采用裁决——Grok 生产采用 model loop/session/tool/permission/sandbox/ACP/workflow/subagent；Claude/Goose/Aider/Codex/Gemini/Qwen 只借鉴 scoped subagent、workflow、MCP、git loop、approval、provider layout 等成熟模式。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Grok CLI Specialization Graft Map** (2026-07-30, 进行中/适配审计): Grok runtime adapter、event normalizer、TUI live source、workflow/subagent/profile、`0.2.112` promoted lock 的初步嫁接位置。注意: CLI/TUI 当前只提升 locked binary doctor 与 `version-smoke`，真实 prompt/tool path 仍需 stronger Windows Job Object containment。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `upstream/grok-build.lock.json`
- **Explicit Retrieval Mode Constraint** (2026-07-30, 设计约束): 检索模式必须显式选择 `local_browser` / `framework_fallback` / `off`；框架自带检索只作本地浏览器不可用或用户允许时的兜底，不得隐式并存或切换。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- **Two-Subagent Default Constraint** (2026-07-30, 设计约束): 默认仅保留项目文档检索子代理和外部检索子代理；不得默认发展第三类或本地多代理调度器。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` / `assurance/retrieval_subagent.py`
- **Neutral vs Counterexample Trigger Split** (2026-07-30, 设计约束): 反例询问只在 plan 写入前和最终结论写入前触发；中立询问发生在执行过程中，用于全局回看和方向辅助；检索子代理关闭前增加“是否已获得完成当前主任务所需内容”的中性完成确认。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` / `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- **Diagnostic Coverage Check** (2026-07-30, 设计约束): debug/问题处理中的递进中性拉回机制。硬信号达到阈值时询问“继续沿当前路线前，关键诊断面是否已经覆盖到足以选择下一步？”阈值按单个 bug 递进 `2 -> 3 -> 4 -> 5`，新证据降噪，用户“继续”不关闭触发。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Global Review Mode** (2026-07-30, 第一切片已完成/审查约束): 显式全局审查模式；日常 review 仍做局部工程审查，明确进入审查模式时才检查原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界。已接 `gsa review global` 生成结构化 activation receipt；该 receipt 只激活和界定全局审查义务，不冒充最终审查结论，也不专门服务 CLI/Grok。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/global_review_mode.py` / `assurance/global-review-mode-receipt-v0.1.schema.json`

### A. P 级合约总览 (P-Level Contract Hierarchy)

- **P0** (2026-07-26, 事实/合约层): 数据语义层——基础 Schema 定义、信封、合约基类。这是所有上层合约的语义基础。入口: `assurance/contracts.py` / `assurance/envelope.py` / `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- **P1** (2026-07-25, 事实/合约层): 会话身份与归档删除生命周期——conversation namespace + permit + archive journal 的基础链路。2026-07-28: StorageAdapter 已接入 ArchiveController；archive_recovery.py（holistic 恢复编排器）；SessionGovernor（thin wrapper 确保所有 adapter artifact 通过 namespace 路由）；canonical CLI 真实 adapter 路径已接入 ConversationNamespace + enforce_adapter_call()。入口: `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md` / `assurance/conversation.py` / `assurance/permit.py` / `assurance/archive.py` / `assurance/archive_journal.py` / `assurance/archive_recovery.py` / `assurance/archive_verifier.py` / `assurance/session_governor.py`
- **P2** (2026-07-24, 事实/合约层): Docker 严格沙箱——进程隔离、状态可观测性。Windows Native Sandbox (AppContainer + Job Object) 作为辅助沙箱选项。入口: `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md` / `assurance/sandbox.py` / `assurance/windows_sandbox.py` / `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **P2.5** (2026-07-24, 事实/合约层): 守卫执行 (Guarded Execution)——无模型动作、进程追踪、输出拦截。入口: `docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md` / `assurance/guarded_execution.py`
- **P3** (2026-07-25, 事实/合约层): 指令授权——instruction provenance、capability delegation、action authorization、child capability enforcement。入口: `docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md` / `assurance/instruction_gate.py` / `assurance/instruction_provenance_gate.py` / `assurance/child_capability_enforcer.py`
- **P4** (2026-07-25, 事实/合约层): 元数据审计 + 压缩 + 恢复授权——audit ledger, archive verifier, recovery candidate。入口: `docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md` / `assurance/audit.py` / `assurance/archive_verifier.py` / `assurance/recovery.py`
- **P4.5** (2026-07-25, 事实/合约层): 工作区优先集成——workspace trust + integrated run + network permit gateway。入口: `docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md` / `assurance/integrated_run.py` / `assurance/workspace_trust.py` / `assurance/network_permit_gateway.py`
- **P5** (2026-07-25, 事实/合约层): 合成用户任务 + 机械预检 + 只读投影——synthetic user task evaluation, runtime preflight, readonly projection。入口: `docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md` / `assurance/user_task_evaluation.py` / `assurance/runtime_preflight.py` / `assurance/readonly_projection.py`
- **GSA-CORE** (2026-07-25, 事实/合约层): 领域中立的只读审查切片——多文件 artifact schema registry + validator bridge + general science review。独立于 P 级框架。入口: `docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md` / `assurance/artifact_registry.py` / `assurance/validator_bridge.py` / `assurance/general_science_review.py`

### B. Gate 链路 (Gate Chain)

> 规范守卫 CLI 的离线主路径 Gate 序列。Gate 之间为串行链：前一个通过后才进入下一个。

- **GAK-INJ-001 指令来源 Gate** (2026-07-27, 事实/2026-07-28 关闭): 入口级多源指令分类/反注入 Gate——batch gate 对多源指令进行 provenance 分类，拦截注入。2026-07-28: 新增 production content parser（parse_instruction_content + detect_obfuscated_injection）、canonicalizer 集成（路径遍历/SSRF 检测）、25 个注入模式、unified gate entry 验证；89 tests pass。别名: instruction provenance gate。入口: `docs/GAK_INJ_001_AUDIT_2026-07-27.md` / `assurance/instruction_provenance_gate.py` / `assurance/tests/test_injection_adversarial.py`
- **Tool Availability Gate** (2026-07-27, 事实/assurance-owned): 机械式工具可用性探测——在模型调用前验证工具声明与运行时实际可用性一致性，检测 belief mismatch/stagnation。Grok 负责实际 tool registry/dispatch，本 Gate 负责状态 receipt 和 UI/prompt 可见性。入口: `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` / `assurance/tool_availability_gate.py`
- **Retrieval Subagents (2 only)** (2026-07-27, 事实/assurance-owned, 2026-07-30 重分类): 默认仅保留两个检索子代理：项目文档检索子代理和外部检索子代理。关闭子代理前执行中性 completion check：只问是否已获得当前主任务所需内容。后续优先映射到 Grok subagent/workflow/profile，不新增本地多代理调度器。入口: `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/retrieval_subagent.py` / `assurance/project_doc_index.py` / `assurance/tests/test_retrieval_subagent_real.py`
- **Orientation Runtime Guard / Neutral Inquiry** (2026-07-27, 事实/assurance-owned): 中性方向检查点 + 运行时停滞守卫——执行过程中辅助全局回看，检测 agent 是否陷入循环/停滞。2026-07-30 补充: debug 路线锁死时使用 Diagnostic Coverage Check，按递进阈值触发。入口: `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/orientation_runtime_guard.py` / `assurance/orientation_runtime_integration.py` / `assurance/orientation_runtime_journal.py` / `assurance/orientation-checkpoint-v0.1.schema.json`
- **Workspace Trust Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-TRUST-001): 统一工作区信任——全部 Python 入口点（canonical CLI + 检索子代理 ×2）在 IPG 评估前建立 workspace trust receipt。`AdapterGateContext` 携带 trust_receipt + trust_status。别名: GAK-TRUST-001。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py` / `assurance/retrieval_subagent.py`
- **Source Visibility Gate** (2026-07-26, 事实): 全文可见性检查 Gate——验证引用源的完整文本可见性。入口: `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md` / `assurance/source_visibility.py` / `assurance/source_visibility_cli.py`
- **Local Browser Retrieval and PDF Evidence** (2026-07-28 设计冻结, 2026-07-29 关闭/assurance-owned): 本地浏览器外部检索与论文 PDF 确定性证据链。2026-07-30 约束: 默认主路径为 `local_browser`；框架自带检索只作 `framework_fallback`，不得隐式并存或切换。别名/关键词: browser retrieval, local_browser, framework_fallback, PDF evidence, LBR-001, CDP, GSA Chrome profile。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/pdf_evidence.py` / `assurance/evidence_store.py` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **Child Capability Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-CHILD-001): 子代理/子进程 capability 传递——`spawn_child_context()` 统一入口；检索子代理接受 parent_envelope；`execute_guarded_no_model_action()` 在 Docker create 前调用 `enforce_child_capabilities()`。别名: GAK-CHILD-001。入口: `assurance/child_capability_enforcer.py` / `assurance/instruction_gate.py` / `assurance/retrieval_subagent.py` / `assurance/guarded_execution.py`
- **Network Permit Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-NET-001): 网络许可统一覆盖——`call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy；`_resolve_and_setup_gates()` 构建 guarded 模式默认 policy。别名: GAK-NET-001。入口: `assurance/network_permit_gateway.py` / `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **Adapter Gate** (2026-07-27, 事实): 适配器调用旁路执行 Gate——防止运行时绕过适配器直接调用模型。别名: adapter gate enforcement, adapter preflight。入口: `docs/ADAPTER_GATE_AUDIT_2026-07-28.md` / `assurance/adapter_gate.py` / `assurance/adapter_preflight.py` / `assurance/adapter_output_validator.py` / `assurance/adapter_failure_classifier.py` / `assurance/adapter-gate-enforcement-receipt-v0.1.schema.json`
- **Canonical Guarded CLI Gate 序列** (2026-07-26, 事实/fixture+assurance path): 离线主路径 Gate 序列定义: instruction provenance gate → tool availability gate → orientation checkpoint → source visibility gate → fake DeepSeek adapter boundary → answer packet → runtime JSONL journal → independent verifier。2026-07-30 后不作为最终 production agent runtime；Grok 嫁接后保留为 conformance/assurance fixture 与回归路径。入口: `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md` / `assurance/canonical_cli.py` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`

### C. 运行时集成 (Runtime Integration)

- **Grok Build / Grok CLI** (2026-07-21~30, runtime-owned/第一生产底座): 当前优先生产 runtime 底座。Grok 应拥有 model loop、session、tool dispatch、permission 基础面、sandbox/进程基础、custom model config、ACP、workflow、subagent、MCP/plugins/skills、compaction/checkpoint。项目默认 lock 已提升至 `0.2.112 (9bbd559437)`；Windows child-tree `tool_timeout` 经对照重分类为 carried-forward limitation，prompt/tool 模式仍需 stronger containment 后才能接入。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `docs/GROK_0_2_112_CANDIDATE_GATE_2026-07-30.md` / `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md` / `upstream/grok-build.lock.json` / `integration/grok/`
- **DeepSeek Adapter** (2026-07-23, 事实/首个真实适配器): 首个真实模型适配器——通过 DeepSeek API 进行 one-shot/stream 观察。凭证硬化已完成。入口: `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md` / `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` / `assurance/deepseek_adapter.py` / `assurance/deepseek_api_observation.py` / `assurance/deepseek_stream_observation.py`
- **Codex App Server / VS Code Internal Terminal Lifecycle** (2026-07-25~30, 事实/生命周期研究): Codex app server、VS Code 内部终端方向的初步会话生命周期捕获、标准化与验证。2026-07-30 裁决: 保留为本地 IDE/session 观测基础，不升级为完整 VS Code 插件或完整 IDE 前端适配层。入口: `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md` / `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `runtime/codex-app-server-*.schema.json` / `scripts/capture_codex_app_server_lifecycle.py`

### D. Global Progress Sentinel (GPS)

> 全局进度哨兵系统——跨 agent 会话的进度追踪、检查点、状态归约与日志恢复。

- **GPS Holistic Gate** (2026-07-25, 事实): 全局进度整体门控——跨会话的 holistic 进度评估。入口: `docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`
- **GPS Checkpoint Adapter** (2026-07-25, 事实): 检查点适配器——策略化检查点保存/恢复。入口: `docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`
- **GPS Transition Gate** (2026-07-25, 事实): 进度转换门控——状态迁移的守卫逻辑。入口: `docs/GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md`
- **GPS Atomic Journal & Reason Migration** (2026-07-25, 事实): 原子日志 + reason code 迁移。入口: `docs/GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md` / `protocol/reason-codes-v0.1.yaml`
- **GPS Journal Recovery** (2026-07-25, 事实): 日志恢复机制。入口: `docs/GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md`
- **GPS Runtime Controller** (2026-07-25, 事实): 运行时控制器。入口: `docs/GLOBAL_PROGRESS_RUNTIME_CONTROLLER_AUDIT_2026-07-25.md`
- **GPS Controller Verifier** (2026-07-25, 事实): 控制器输出验证器。入口: `docs/GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md`
- **GPS State Reducer** (2026-07-25, 事实): 状态归约器——跨会话状态聚合。入口: `docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`
- **GPS 合约设计** (2026-07-21, 设计约束): GPS 的架构合约定义。入口: `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`

### E. CLI 与执行 (CLI & Execution)

- **gsa CLI 入口** (2026-07, 事实): 顶级 CLI 分发器——`gsa doctor`, `gsa source gate`, `gsa run`, `gsa verify`, `gsa tui`, `gsa grok doctor`, `gsa grok run --mode version-smoke`, `gsa review global`。注意: `gsa run` 仍保持 canonical 默认路径；Grok 仅接入 locked-binary doctor/version-smoke；全局审查需显式触发。入口: `gsa.py` / `assurance/cli.py`；TUI 设计: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`
- **Canonical Guarded CLI** (2026-07-26, 事实): 规范守卫 CLI 主路径——manifests, fake answer packets, verification。注意: TUI 将消费本模块的结构化事件流（gate_decision, run_state 等），通过 view model 投影渲染到终端；本模块保持事件发射职责，不耦合 UI 渲染。入口: `docs/CANONICAL_CLI_QUICKSTART_2026-07-26.md` / `assurance/canonical_cli.py` / `assurance/canonical_cli_main.py`；TUI 设计: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`
- **Disposable Reproduction** (2026-07-26, 事实): 可处置复现——manifest, receipt, proof 的完整生命周期。入口: `docs/GSA_DISPOSABLE_REPRODUCTION_AUDIT_2026-07-26.md` / `assurance/disposable_reproduction.py` / `assurance/execution_lock.py`
- **Guarded Execution** (2026-07-24, 事实): P2.5 守卫执行——进程追踪与输出拦截。入口: `assurance/guarded_execution.py` / `assurance/child_capability_enforcer.py`
- **Runner (No-Model)** (2026-07, 事实): 无模型 runner 骨架——journal recovery/repair。入口: `assurance/runner.py` / `assurance/runner_public_output.py` / `assurance/runner_scoring_handoff.py`
- **Integrated Run** (2026-07-25, 事实): P4.5 工作区优先集成运行。入口: `assurance/integrated_run.py`
- **Task Contract** (2026-07, 事实): 从 ask 构建任务合约。入口: `assurance/task_contract.py`
- **Execution Lock** (2026-07, 事实): 可处置复现的执行锁。入口: `assurance/execution_lock.py`
- **Archive Journal Recovery** (2026-07, 事实): 归档日志的恢复与修复脚本。入口: `scripts/recover_archive_journal.py`

### F. 安全与沙箱 (Security & Sandbox)

- **Docker Sandbox** (2026-07-24, 事实): P2 Docker 严格沙箱——进程隔离、网络限制、状态可观测。入口: `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md` / `assurance/sandbox.py` / `assurance/sandbox_verifier.py`
- **Windows Native Sandbox** (2026-07-27, 事实): AppContainer + Job Object + netsh firewall 的 Windows 原生沙箱。Elevated 路径 compliant（firewall rule 创建成功 → selection allow），non-elevated 路径 fail-closed（正确行为）。别名: GAK-SBX-001。入口: `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md` / `assurance/windows_sandbox.py`
- **Network Permit Gateway** (2026-07-25, 事实): 网络许可评估——控制 agent 对外网络访问权限。入口: `assurance/network_permit_gateway.py` / `architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`
- **Workspace Trust** (2026-07-21, 事实): 工作区信任建立与验证——防止 agent 在不受信任的目录中执行。入口: `docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md` / `assurance/workspace_trust.py`
- **Keystore / Key Lifecycle** (2026-07, 事实): 安装级密钥存储（内存 + Windows DPAPI）+ 密钥生命周期控制。入口: `assurance/keystore.py` / `assurance/key_lifecycle.py`
- **Envelope** (2026-07, 事实): 安全信封——数据完整性与来源验证。入口: `assurance/envelope.py`
- **Permit** (2026-07, 事实): 敏感操作许可生命周期。入口: `assurance/permit.py`
- **Endpoint Canonicalizer** (2026-07, 事实): 文件系统路径与网络端点的规范化——防止路径遍历与 SSRF。入口: `assurance/endpoint_canonicalizer.py`
- **Storage Adapter** (2026-07, 事实): 本地存储适配器——统一的文件系统访问接口。入口: `assurance/storage_adapter.py`

### G. 架构文档 (Architecture)

> 设计意图、合约边界与战略决策。每篇文档承担一个独立的设计主题。

- **产品定位与参考策略** (v0.1, 设计约束): 核心定位——运行时中立的通用科学保证内核，不绑定单一 LLM 运行时。入口: `architecture/PRODUCT_POSITIONING_AND_REFERENCE_STRATEGY_v0.1.md`
- **Integrated Agent Assurance Design** (v0.1, 设计约束/2026-07-30 修正): 保留全部关键设计，但明确成熟 runtime 拥有通用 agent 平台，本仓库拥有 assurance/evidence/UI/adapter/fixture。入口: `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`
- **Agent 底座嫁接价值中文评估** (2026-07-30, 设计约束/中文主入口): 中文确认保留内容、检索显式开关、两个子代理限制、中立/反例触发分离、全局审查模式、VS Code 内部终端保留、部件级采用裁决。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Grok CLI Specialization Adaptation Audit** (2026-07-30, 进行中/嫁接审计): Grok runtime adapter、event normalizer、TUI live source、workflow/subagent/profile、`0.2.112` promoted gate 的实现路由；CLI/TUI 已接 first-slice version-smoke。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`
- **CLI UI Interaction Model** (v0.1, 事实/2026-07-30 Grok first-slice 接入): Windows-only 复古桌面/老 IE 风格终端 UI 交互模型。第一原型已完成（9 个模块 ~3900 行）：静态框架 + prompt_toolkit + 自定义 retro widget 层（10 个 widget 类）、17 个中文斜杠命令 + CommandRegistry + CommandPalette 覆盖层、地址栏命令历史/多行输入/自动补全、Ctrl+Z 取消运行。Phase 2：canonical CLI 事件流桥接（`bridge.py`、`LiveRunEventSource`、`JsonlFileSource`、3 个 typed gate event、`--run`/`--replay`）。Phase 3：真实 DeepSeek adapter 接入（`--real`、`build_live_run_fn(real_adapter=True)` 全链路贯通）。Phase 4 (R15)：默认中文 UI + HelpOverlay modal (6 tabs) + FindDialog modal + 大主窗模式（侧栏 toggle）+ `Agent`→`模型` + `Edit`→`编辑模式`。2026-07-30: 新增 `--runtime grok` first-slice，只运行 `version-smoke` 并消费 Grok normalized events；221 tests pass。别名: retro TUI, Explorer-style UI, old IE UI, terminal TUI。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`；代码: `assurance/tui/`（`bridge.py` / `commands.py` / `widgets.py` / `app.py` / `pt_app.py` / `view_models.py` / `main.py` / `events.py` / `event_source.py` / `projector.py`）；Gap Register: GAK-UI-001
- **CLI UI Simplification Supplement** (v0.1, 设计约束/2026-07-29 R16 实施闭合): 下一轮 TUI 精简与中文化方向——R15 完成默认中文 UI（菜单/工具栏/状态栏/对话框全中文化）、HelpOverlay modal（6 tabs, Esc 关闭, ←/→ 导航）、FindDialog modal（Ctrl+F 多行/高级搜索）、大主窗模式（侧栏 toggle）、顶部两层布局保留、`Agent`→`模型`、`Edit`→`编辑模式`；R16 补齐 Address 完整输入弹窗化、聊天室式可折叠任务流与 Task Checklist 公告/展开联动。别名: UI 精简, 中文 TUI, Help 弹窗, 大主窗, AddressDialog, 可折叠任务流。入口: `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` / `assurance/tui/app.py` / `assurance/tui/widgets.py`
- **Task Checklist Announcement Supplement** (v0.1, 设计约束/2026-07-29 R16 已实施): 主对话顶部公告式 checklist 与展开式工作清单页——Plan mode 批准后生成稳定 step ID 与 plan annotations；常态只显示步骤，展开显示软约束、运行记录和验收引用；Orientation Runtime Guard 只读取中性定位上下文，不询问是否偏移/正确/卡住。2026-07-29 R15 冻结架构文档（268 行）；R16 完成 7 个实施切片（checklist view model → plan mode bridge → announcement strip projection → expanded checklist page → orientation context bridge → journal/artifact registration → GPS mapping），25 checklist tests + 221 TUI tests + 80 plan mode tests pass。别名: checklist 公告, 工作清单, plan annotations, soft workboard, orientation checkpoint context。入口: `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md` / `assurance/task_checklist.py` / `assurance/tui/projector.py` / `assurance/tui/bridge.py`
- **Plan Mode and Process Usage Monitor** (v0.1, 设计约束/2026-07-29 R15 已实施): 稳定 Plan mode 与运行进程占用监控契约——计划固定四部分（前期调查/具体计划/具体设计/实施方案）、plan approval 与 action approval 分离、root PID 进程树 CPU/MEM/time 采样、系统终端即时显示、VS Code terminal title 2 秒刷新。R15 已交付：`plan_mode.py` (~700 行) — PlanArtifact（4-section structured plan）、PlanVerifier（7 check categories）、PlanStateMachine（8-state lifecycle）、ProcessUsageSampler（psutil 后台采样 + adaptive backoff）、VSCodeTitleUpdater、terminal status line；`plan-mode-result-v0.1.schema.json`；4 个新 TUI event 类型（plan_phase_entered/submitted/approval_decision/usage_sample）；80 tests pass。别名: plan mode, process usage monitor, CPU/MEM, VS Code terminal title。入口: `architecture/PLAN_MODE_AND_PROCESS_USAGE_MONITOR_v0.1.md` / `assurance/plan_mode.py` / `assurance/plan-mode-result-v0.1.schema.json` / `assurance/tests/test_plan_mode.py`
- **Grok Build 适配** (v0.1, 设计约束/2026-07-30 follow-up alignment): 以 Grok Build 为参考运行时的适配策略；旧 V0 中 headless-first、subagent 延后、TUI 延后的表述已按 Runtime-First 裁决修正为 ACP observability 优先、两个检索子代理映射到 Grok-compatible profile/workflow、TUI 消费 normalized runtime state。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md`
- **Upstream First 集成** (v0.1, 设计约束): 上游优先集成策略——先锁定上游版本再进行适配。入口: `architecture/UPSTREAM_FIRST_INTEGRATION_v0.1.md`
- **Upstream 版本策略** (v0.1, 设计约束): 上游版本锁定、候选评估与升级流程。入口: `architecture/UPSTREAM_VERSION_STRATEGY_v0.1.md`
- **可观测执行信封** (v0.1, 设计约束): 可观测执行包装器设计——将 agent 执行包装为可审计的事件流。入口: `architecture/OBSERVABLE_EXECUTION_ENVELOPE_v0.1.md`
- **成熟 Agent 设计解构** (v0.2, 设计约束): Grok/Codex/Gemini/OpenCode/Goose/Cline 的深度设计分析。入口: `architecture/MATURE_AGENT_DESIGN_DECOMPOSITION_v0.2.md`
- **通用保证内核 Gap 登记** (v0.1, 设计约束): 当前内核与理想状态的功能差距登记。入口: `architecture/GENERAL_ASSURANCE_KERNEL_GAP_REGISTER_v0.1.md`
- **DeepSeek Adapter 合约** (v0.1, 设计约束): DeepSeek API 适配器的合约定义。入口: `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md`
- **DeepSeek 外部传输安全** (v0.1, 设计约束): 外部 API 调用的传输安全策略。入口: `architecture/DEEPSEEK_EXTERNAL_TRANSPORT_SAFETY_v0.1.md`
- **Windows 运行时合约** (v0.1, 设计约束): Windows 平台的运行时约束与接口定义。入口: `architecture/WINDOWS_RUNTIME_CONTRACT_v0.1.md`
- **Action Kernel 合约** (v0.1, 设计约束): 无模型动作内核的 spike 设计。入口: `architecture/ACTION_KERNEL_CONTRACT_v0.1.md`
- **Model Adapter Loop 合约** (v0.1, 设计约束): 模型适配器循环的设计——适配器与 runner 之间的控制流。入口: `architecture/MODEL_ADAPTER_LOOP_CONTRACT_v0.1.md`
- **Network Permit Broker 合约** (v0.1, 设计约束): 网络许可代理的架构设计。入口: `architecture/NETWORK_PERMIT_BROKER_CONTRACT_v0.1.md`
- **Interactive Approval Ledger 合约** (v0.1, 设计约束): 交互式审批账本——用户审批的持久化与审计。入口: `architecture/INTERACTIVE_APPROVAL_LEDGER_CONTRACT_v0.1.md`
- **Loopback Transport & Private Transcript** (v0.1, 设计约束): 回环传输与私有转录——本地的安全通信通道。入口: `architecture/LOOPBACK_TRANSPORT_AND_PRIVATE_TRANSCRIPT_v0.1.md`
- **D Salvage Matrix** (v0.1, 设计约束): 项目 D 源码挽救矩阵——标识可从旧项目复用的组件。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`
- **Open Source Agent Gap Audit** (v0.1, 设计约束): 开源 agent 框架的功能差距审计。入口: `architecture/OPEN_SOURCE_AGENT_GAP_AUDIT_v0.1.md`
- **GPS 合约** (v0.1, 设计约束): Global Progress Sentinel 的架构合约。入口: `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`

### H. ADR (Architecture Decision Records)

> 不可逆的架构决策记录。ADR 一旦记录即生效，修改需新 ADR。

- **ADR-0001** (2026-07, 事实/不可变): 证据约束的本地 agent 内核——产品边界与不变量定义。入口: `adr/ADR-0001-evidence-constrained-local-agent-kernel.md`
- **ADR-0002** (2026-07, 事实/不可变): 推迟云端运行时——显式将云端运行时排除在当前范围外。入口: `adr/ADR-0002-defer-cloud-runtime.md`
- **ADR-0003** (2026-07, 事实/不可变): 运行时中立的保证内核——保证内核不绑定任何特定 LLM 运行时。入口: `adr/ADR-0003-runtime-neutral-assurance-kernel.md`
- **ADR-0004** (2026-07, 事实/不可变): 通用科学 Profile 分层——从 general-science 到 lif-research 的 profile 层次结构。入口: `adr/ADR-0004-general-science-profile-layering.md`

### I. Schema 与合约体系 (Schema & Contract System)

- **assurance/ Schema 文件** (87 个 JSON Schema): 核心保证层的所有数据合约——receipt, manifest, event, envelope, gate receipt, sandbox profile, checkpoint, journal, retrieval, disposable reproduction 等。入口: `assurance/**/*.schema.json`
- **runtime/ Schema 文件** (48 个 JSON Schema): 运行时控制面的数据合约——run-manifest, run-event, scenario-export, leak-scan, CLI session lifecycle (v0.1/v0.2), Codex app server capture/normalization/verification/turn-probe, GPS review/disposition/checkpoint/holistic/transition/controller/state-reduction, journal recovery, deepseek adapter/development, network approval, action kernel, model loop, windows process。入口: `runtime/**/*.schema.json`
- **integration/grok/ Schema 文件** (24 个 JSON Schema): Grok 探针结果的数据合约——observed plan, fake provider, tool continuity, workspace discovery/trust, event bridge, ACP initialize/fake-tool, windows child tree, compaction provenance, real deepseek plan/result/failure。入口: `integration/grok/**/*.schema.json`
- **protocol/ 协议定义** (v0.1, 设计约束): 语言无关的协议草案——agent protocol schema, reason codes, gate matrix, global-progress reason code migration, 协议语义草案。入口: `protocol/agent-protocol-v0.1.schema.json` / `protocol/reason-codes-v0.1.yaml` / `protocol/gate-matrix-v0.1.yaml` / `protocol/PROTOCOL_DRAFT_v0.1.md` / `protocol/global-progress-reason-code-migration-v0.1.yaml`
- **regression/ Schema** (v0.1, 设计约束): 回归测试的完整 schema 体系——case corpus, coverage matrix, fixture, historical excerpt provenance。入口: `regression/case-corpus-v0.1.schema.json` / `regression/coverage-matrix-v0.1.schema.json` / `regression/fixture-v0.1.schema.json` / `regression/historical-excerpt-provenance-v0.1.schema.json` / `regression/cases-v0.1.yaml`
- **evaluation/ Schema** (v0.1, 设计约束): 评估结果与分区清单的 schema。入口: `evaluation/evaluation-result-v0.1.schema.json` / `evaluation/partition-manifest-v0.1.schema.json`
- **upstream/ Schema** (v0.1, 设计约束): 上游构建锁与候选版本的 schema。入口: `upstream/grok-build-lock-v0.1.schema.json` / `upstream/grok-build-candidate-v0.1.schema.json` / `upstream/grok-build.lock.json` / `upstream/grok-build.candidate.json`

### J. 评估与回归 (Evaluation & Regression)

- **Scoring Protocol** (v0.1, 设计约束): cluster-macro 评分、红线规则、盲审协议。入口: `evaluation/SCORING_PROTOCOL_v0.1.md`
- **Partition & Oracle Isolation** (v0.1, 设计约束): 评估分区与 oracle 物理隔离协议。入口: `evaluation/PARTITION_AND_ORACLE_ISOLATION_v0.1.md`
- **Human Baseline Protocol** (v0.1, 设计约束): 双盲人类基线的评估协议。入口: `evaluation/HUMAN_BASELINE_PROTOCOL_v0.1.md`
- **Regression Case Corpus** (v0.1, 事实): 30 个历史 + 10 个合成反惯性案例的回归语料库。入口: `regression/cases-v0.1.yaml`
- **Coverage Matrix** (v0.1, 事实): 10 个错误簇的覆盖矩阵。入口: `regression/coverage-matrix-v0.1.yaml`
- **Curation Protocol** (v0.1, 设计约束): 回归案例的策展协议。入口: `regression/CURATION_PROTOCOL_v0.1.md`
- **Self-Question Counterexample Design** (2026-07-26, 设计约束): 自问反例设计——GSA 的自检能力边界验证。入口: `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- **General Scientific Assurance Gap Review** (2026-07-25, 事实): 通用科学保证的差距审查。入口: `docs/GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW_2026-07-25.md`
- **Cross-Artifact Lineage Comparability** (2026-07-25, 事实): 跨制品的 lineage 可比性审计。入口: `docs/GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT_2026-07-25.md`

### K. Grok 探针系列 (Grok Probes)

> 对参考运行时 Grok Build 的系统性探针测试。每项探针验证 Grok 的一个具体行为维度。

- **Grok Windows Binary** (2026-07-21, 事实): Grok 在 Windows 上的二进制执行审计。入口: `docs/GROK_WINDOWS_BINARY_AUDIT_2026-07-21.md`
- **Grok ACP Initialize** (2026-07-21, 事实): ACP 协议初始化探针——验证 initialize 握手。入口: `docs/GROK_ACP_INITIALIZE_AUDIT_2026-07-21.md`
- **Grok ACP Fake Tool** (2026-07-23, 事实): ACP 假工具 spike——验证工具调用的协议层行为。入口: `docs/GROK_ACP_FAKE_TOOL_SPIKE_2026-07-23.md`
- **Grok Fake Provider** (2026-07-21, 事实): 假模型提供者探针——验证 provider 抽象边界。入口: `docs/GROK_FAKE_PROVIDER_AUDIT_2026-07-21.md`
- **Grok Event Bridge** (2026-07-21, 事实): 事件桥探针——验证 agent 事件流的完整性。入口: `docs/GROK_EVENT_BRIDGE_AUDIT_2026-07-21.md`
- **Grok Compaction Provenance** (2026-07-23, 事实): 压缩溯源——验证上下文压缩不丢失关键来源信息。入口: `docs/GROK_COMPACTION_PROVENANCE_2026-07-23.md`
- **Grok Workspace Trust** (2026-07-21, 事实): 工作区信任探针。入口: `docs/GROK_WORKSPACE_TRUST_AUDIT_2026-07-21.md`
- **Grok Workspace Discovery / Fixture Checkpoint Delta** (2026-07-21, 事实): 工作区发现机制 + 夹具检查点 delta spike。入口: `docs/GROK_FIXTURE_CHECKPOINT_DELTA_SPIKE_2026-07-21.md`
- **Grok Windows Child Tree** (2026-07-23, 事实): Windows 子进程树探针——验证进程树追踪。入口: `docs/GROK_WINDOWS_CHILD_TREE_PROBE_2026-07-23.md`
- **Grok Tool Continuity** (2026-07-21, 事实): 工具连续性探针——验证跨轮工具状态保持。入口: `docs/GROK_TOOL_CONTINUITY_AUDIT_2026-07-21.md`
- **Grok Real DeepSeek Launcher** (2026-07-23, 事实): 真实 DeepSeek 模型启动器——Grok 框架内调用真实模型。入口: `docs/GROK_REAL_DEEPSEEK_LAUNCHER_2026-07-23.md`
- **Grok Upstream Candidate** (2026-07-23, 事实): 上游候选版本评估。入口: `docs/GROK_UPSTREAM_CANDIDATE_AUDIT_2026-07-23.md`
- **Grok Upstream Promotion** (2026-07-23, 事实): 上游版本升级。入口: `docs/GROK_UPSTREAM_PROMOTION_2026-07-23.md`

### L. 原型代码 (Prototype)

> 旧原型子系统 `fep_agent_proto`——显式标记为 **不作为生产 runner**。保留用于设计参考。

- **FEP Agent Proto** (2026-06~07, 废弃/参考): 旧原型包——包含 action kernel, DeepSeek adapter/client, model transport, loopback mock server, network broker, approval ledger, journal (with lock/recovery), CLI session lifecycle, codex app server capture, global progress state 等模块。状态: 废弃/仅设计参考。入口: `prototype/fep_agent_proto/`
- **D Salvage Matrix** (v0.1, 设计约束): 标识可从此原型挽救到主项目的组件。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`

### M. 脚本与工具 (Scripts & Tools)

- **Repository Check** (2026-07, 事实): CI 仓库完整性检查脚本——schema/corpus/coverage/fixture 全面验证。入口: `scripts/check_repository.py`
- **GPS 脚本系列**: 构建/验证/追加/归约 GPS 状态的脚本。入口: `scripts/build_gps_*.py` / `scripts/verify_gps_*.py`
- **Grok 探针脚本**: Grok 调用的 PowerShell 启动器 (`invoke_grok_*.ps1`, 13 个) 与 Python 验证器 (`verify_grok_*.py`, 6 个)。入口: `scripts/`
- **DeepSeek 脚本**: DeepSeek public output observation 的 builder 与 launcher。入口: `scripts/build_deepseek_public_output_observation.py` / `scripts/build_deepseek_stream_observation_fixture.py` / `scripts/invoke_deepseek_public_output_observation.ps1`
- **Windows Sandbox Probe** (2026-07-27, 事实): Windows 原生沙箱的探针运行器。入口: `scripts/run_windows_native_sandbox_probe.py`
- **Archive Journal Recovery** (2026-07, 事实): 归档日志恢复脚本。入口: `scripts/recover_archive_journal.py`

### N. CI 与工程配置 (CI & Engineering)

- **CI Pipeline** (2026-07, 事实): GitHub Actions——Ubuntu + Windows, Python 3.11/3.12, 多步测试（repository check, compileall, PowerShell 语法检查, 单元测试）。入口: `.github/workflows/ci.yml`
- **pyproject.toml** (2026-07, 事实): 包构建配置——`gsa-assurance` 包, `gsa = "assurance.cli:main"` 入口点, jsonschema + rfc8785 依赖。入口: `pyproject.toml`
- **pytest.ini** (2026-07, 事实): Pytest 配置——`asyncio_mode = strict`, function-scoped fixtures。入口: `pytest.ini`
- **.claude/settings.local.json** (2026-07, 事实): Claude Code 本地权限配置。入口: `.claude/settings.local.json`

### O. 项目级文档 (Project-Level Documents)

- **README.md** (2026-07, 事实): 项目主 README——状态总结、P 级合约概览、Gate 链路描述、当前边界与已知限制。中文。入口: `README.md`
- **assurance/README.md** (2026-07, 事实): 保证内核包 README——P0-P5 合约摘要、权威所有权 ADR 引用、当前合同列表。入口: `assurance/README.md`
- **integration/grok/README.md** (2026-07, 事实): Grok 集成目录 README。入口: `integration/grok/README.md`

### P. 测试基础设施 (Test Infrastructure)

- **assurance/tests/** (34 个测试文件): 每个核心模块对应 1 个测试文件——P0-P5 合约、Gate 链路 (instruction provenance, tool availability, retrieval subagent)、orientation guard、DeepSeek adapter/observation、canonical CLI、sandbox verifier、archive controller、adapter gate/integration、runner scoring、key lifecycle、trust/child/network、windows sandbox/race escape、endpoint canonicalizer、disposable reproduction、source visibility、general science review。入口: `assurance/tests/`
- **runtime/tests/** (8 个测试文件): 运行时控制面测试——GPS sentinel/holistic/checkpoint/transition、CLI session lifecycle adapter、Codex app server lifecycle normalizer/capture/turn probe。入口: `runtime/tests/`
- **integration/grok/tests/** (9 个测试文件): Grok 探针测试——workspace trust, event bridge, fixture workspace capture, ACP initialize/fake-tool, windows child tree, fake provider, compaction provenance, real deepseek launcher。入口: `integration/grok/tests/`
- **assurance/fixtures/** (P0/P1/P5/general_science/source_visibility): 测试夹具数据——valid/invalid/semantic-invalid JSON 文件，覆盖各 P 级合约、GSA-CORE computational_decay 示例、source visibility ledger。入口: `assurance/fixtures/`
- **runtime/fixtures/** (GPS + Codex): GPS 输入与 disposition、Codex app server lifecycle 捕获 (happy/failed/interrupted/truncated/cross-thread/duplicate)。入口: `runtime/fixtures/`
- **regression/fixtures/** (10 个案例): 5 个合成反惯性 (FEP-SYN-006~010) + 5 个历史摘录 (FEP-REG-009/015/016/025/029)，各含 evidence/fixture/provenance。入口: `regression/fixtures/`

---

## 状态路由 (Status Routing)

> 按组件状态分类的 ID 速查列表。**Prior-existence scan 第一步**：先查此表确认某组件是否已有同义/相反/撤回/废弃条目。

### 已撤回 / 已废弃 (Withdrawn / Deprecated)

- **FEP Agent Proto** (废弃): 旧原型包 `prototype/fep_agent_proto/` 不作为生产 runner。仅保留用于设计参考。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`
- **旧 P2 CLI 入口** (废弃): 部分旧 CLI 入口点已被 canonical CLI 替代。所有新开发应使用 `assurance/canonical_cli.py` 的 Gate 序列。入口: `assurance/p2_cli.py` / `assurance/p45_cli.py` / `assurance/p5_cli.py`（仅作向后兼容保留）

### 保留但重分类 (Retained / Reclassified)

- **Canonical Guarded CLI** (保留为 fixture+assurance path): 不再作为最终 production agent runtime 叙事；后续 Grok 嫁接后用于 conformance、离线回归、Gate 行为证明。入口: `assurance/canonical_cli.py` / `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- **Local Browser Retrieval / PDF Evidence** (保留为 assurance-owned): 主检索证据链保留；必须通过显式 `local_browser` 模式启用，framework 检索只作 fallback。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Retrieval Subagents** (保留为两个固定子代理): 项目文档检索与外部检索保留；子代理关闭前做中性 completion check，确认是否已获得当前主任务所需内容；后续优先映射到 Grok profiles/workflows，不新增本地 scheduler。入口: `assurance/retrieval_subagent.py` / `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- **Counterexample / Neutral Inquiry** (保留但触发分离): 反例询问只在 plan/conclusion 写入前；中立询问在执行中辅助全局回看。入口: `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` / `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- **Diagnostic Coverage Check** (保留为中立询问子机制): 单个 bug/debug episode 内硬信号递进触发；每次触发后计数清零、阈值 +1，bug 解决后重置。用户继续不覆盖；明确新证据可降噪。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- **Codex App Server / VS Code Internal Terminal Lifecycle** (保留为生命周期观测): 保留初步适配证据，不升级为完整 VS Code 插件。入口: `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md`

### 待嫁接 / Runtime-First In Progress

当前待办顺序（初步确定，2026-07-30）:

1. **Grok `0.2.112` Candidate Gate** (已完成/已提升): PATH 已发现并登记 `0.2.112 (9bbd559437) [stable]`；candidate metadata、binary identity、static Grok integration tests、TUI baseline、repository regression、ACP initialize、fake tool allow/cancel、child-tree task_cancel/parent_exit、compaction provenance 均已通过；Windows child-tree `tool_timeout` 经 `0.2.111` / `0.2.112` 管理员对照后重分类为 carried-forward limitation，不作为相对升级阻断；默认 lock 已更新为 `0.2.112`。入口: `docs/GROK_0_2_112_CANDIDATE_GATE_2026-07-30.md` / `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md` / `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `upstream/grok-build.candidate.json` / `upstream/grok-build.lock.json`
2. **Grok `0.2.112` Upgrade Value Breakdown + Timeout Triage Decision** (已完成/已提升): `0.2.112` 更新集中在 background task lifecycle、terminal output capture、workflow/subagent、MCP inheritance、permission/search controls、Windows auth/provider helpers 和 session resume/fork；升级价值足够高，且 `tool_timeout` 不是新版本回归，因此按 carried limitation 提升。入口: `docs/GROK_0_2_112_UPGRADE_VALUE_BREAKDOWN_2026-07-30.md` / `docs/GROK_0_2_112_TIMEOUT_TRIAGE_2026-07-30.md`
3. **Grok Runtime Adapter + Event Normalizer** (第一切片已完成): 已新增薄 adapter 与事件正规化层；当前切片覆盖锁定 Grok binary inspection、workspace trust 先行、隔离 profile/temp、`grok --version` headless smoke、metadata-only stdout/stderr artifact、canonical runtime event JSONL、TUI-ready metadata projection，并强制记录 `no_residue_required` / `no_residue_observed`。该切片不实现 model loop/tool dispatcher/session runtime；后续 prompt/tool 模式必须使用更强 Windows Job Object containment。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `assurance/grok_runtime_adapter.py` / `assurance/grok_event_normalizer.py` / `assurance/grok-runtime-receipt-v0.1.schema.json`
4. **CLI / TUI Grok Wiring** (第一切片已完成): 已新增 `gsa grok doctor`、`gsa grok run --mode version-smoke`、`gsa tui --runtime grok --run version-smoke`。默认 Grok smoke 使用 `run_root/workspace` 干净隔离目录；显式传入含控制面的项目根会被 restricted workspace trust 正确拒绝。真实 smoke `candidate-gates/grok-cli-version-smoke-r2` 通过: locked `0.2.112` binary、root process exited、`no_residue_observed=true`、`external_cleanup_required=false`、normalized `events.jsonl` 已生成。暂不替换 `gsa run` 默认路径，真实 prompt/tool path 仍待 stronger containment。入口: `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` / `assurance/cli.py` / `assurance/tui/main.py` / `assurance/tui/bridge.py`
5. **Global Review Mode** (第一切片已完成/2026-07-30 通用化修正): 已新增 `gsa review global` 和 `global_review_mode_receipt`，固定五个通用全局审查维度：原设计理念、当前进度判断、当前实现内容定位、关键设计保留、项目任务边界。默认可从 git status 收集范围，也可 `--no-git-status --path ...` 显式指定；输出只表示全局审查模式已激活并界定范围，不替代最终审查判断，不影响日常局部 engineering review，也不专门服务 CLI/Grok。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `assurance/global_review_mode.py` / `assurance/global-review-mode-receipt-v0.1.schema.json` / `assurance/tests/test_global_review_mode.py`
6. **Runtime-First Follow-up Alignment** (对齐记录已完成/实现切片待续): 已新增 follow-up alignment 记录，明确两个 retrieval subagent 只映射到 Grok-compatible profile/workflow、不新增本地 scheduler；retrieval mode 固定为 `local_browser` / `framework_fallback` / `off`；Tool Availability、Global Progress、Orientation、UI 投影继续由 assurance/UI 层消费 Grok registry/permission/workflow 事件；旧 `prototype/` 文档入口改为 retired/reference。后续仍需实现真实 prompt/tool path 的 CLI/TUI option plumbing、Grok profile/workflow syntax verification、tool registry/permission observation adapter、workflow/subagent event mapping。入口: `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` / `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` / `architecture/GROK_BUILD_ADAPTATION_v0.1.md`

### 已关闭 (Closed)

- **GAK-UI-001 CLI UI Interaction Model** (2026-07-28 第一原型完成, 2026-07-29 Phase 2-4 全部完成): 9 个模块 ~3900 行。Phase 2：canonical CLI 事件流桥接（`bridge.py`、`LiveRunEventSource`、`JsonlFileSource`、3 个 typed gate event、`--run`/`--replay`）。Phase 3：真实 DeepSeek adapter 接入（`--real`、`build_live_run_fn(real_adapter=True)` 全链路贯通）。Phase 4 (R15)：默认中文 UI（菜单/工具栏/状态栏/对话框全中文化）+ HelpOverlay modal (6 tabs, Esc 关闭, ←/→ 导航) + FindDialog modal (Ctrl+F 多行/高级搜索) + 大主窗模式（侧栏 toggle）+ `Agent`→`模型` + `Edit`→`编辑模式`。221 TUI tests pass。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `assurance/tui/`
- **LBR-001 Local Browser Retrieval and PDF Evidence** (2026-07-29 关闭): Phase 1-4 全链路完成 + R9 闭合。Evidence Store (42 tests)、CDP 客户端 (27 tests)、检索工作流 (34 tests)、TUI 集成 (217 tests)。R9 新增: ① LiveBrowserTests 自启动 headless Chrome——不再需手动启动 Chrome，`setUpClass` 通过 `_ensure_browser()` 自动启停；skip 条件从 "Chrome 未运行" 改为 "Chrome 未安装"（4 tests pass, 0 skip）。② E2E 集成测试 `test_browser_retrieval_e2e.py` (6 tests)：Web 检索 5 (retrieve_urls 单页/多页/空列表/错误处理/progress callback) + 论文 PDF 检索 1 (run_retrieval arXiv → 下载 → 验证 → 证据库，验证 SHA-256、PDF header、metadata.json、pages.jsonl)。③ 修复 `classify_page` login 检测——"log in"/"login" 从关键词列表移除（arXiv 等网站导航栏中的 login 链接不再误触发 LOGIN_REQUIRED），改为仅匹配强信号 ("please log in to", "log in to continue", "authentication required" 等)。④ 测试可靠性加固——httpbin.org 替换为 example.com/example.org 消除间歇性 WebSocket 超时。全量 71 tests pass (0 skip)。已知限制: Chrome 主 profile 受企业安全策略拦截 CDP 连接，workaround 使用项目隔离 profile (`.gsa_chrome_profile/`)。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/tests/test_browser_retrieval.py` / `assurance/tests/test_retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **GAK-TRUST-001 Workspace Trust 统一入口** (2026-07-28 关闭): `establish_workspace_trust()` 已接入 canonical CLI 和检索子代理全部入口点。`AdapterGateContext` 携带 trust receipt；`_resolve_and_setup_gates()` 在 IPG 前建立 trust；`validate_all_entry_points_establish_trust()` AST 审计覆盖全部入口点。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-CHILD-001 Child Capability 统一传递** (2026-07-28 关闭): 新增 `spawn_child_context()` 统一入口（组合 predicate check + 签名 child envelope 创建）；检索子代理接受可选 `parent_envelope`，通过 `spawn_child_context()` 创建签名 child envelope 并传入 `ConversationNamespace.create(parent_envelope_id=...)`。入口: `assurance/child_capability_enforcer.py` / `assurance/retrieval_subagent.py`
- **GAK-NET-001 Network Permit 统一覆盖** (2026-07-28 关闭): `call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy/endpoint/category；`_resolve_and_setup_gates()` 构建 `guarded` 模式默认 policy。入口: `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-ID-001 Key Rotation/Revocation 产品化** (2026-07-28 关闭): `_FileLock` 文件锁保护所有 key mutation；crash-safe journal（pending → receipt → history → clear）；`recover_pending_rotations()` 恢复；`migrate_envelope()` 真实 envelope 重签名；`rotate()` 支持 `envelopes_to_migrate`。入口: `assurance/key_lifecycle.py` / `assurance/envelope.py`
- **GAK-SBX-001 Windows Native Sandbox** (2026-07-27 关闭): Development baseline 已达成——elevated 路径 compliant（netsh firewall + AppContainer + Job Object），non-elevated 路径 fail-closed。原始 TCP 残余记录为已知 Windows 平台限制，不阻塞 development 门禁。入口: `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **GAK-WIN-001 Windows Job Object Race** (2026-07-28 关闭): `PROC_THREAD_ATTRIBUTE_JOB_LIST` 内核级 Job Object 原子绑定——进程创建时在内核中分配 Job，消除 `CreateProcess`→`AssignProcessToJobObject` 用户态竞态窗口。`CREATE_SUSPENDED` + `TokenIsAppContainer` + `IsProcessInJob` 在 `ResumeThread` 前全部验证。`PROC_THREAD_ATTRIBUTE_JOB_LIST` 不可用时优雅降级至 post-creation 分配。11 tests pass, 1 skipped（旧 OS 自动跳过）。入口: `assurance/windows_sandbox.py:856-983` / `assurance/tests/test_windows_race_escape.py:383-970`
- **GSA-PROFILE-001 Profile Registry 全量注册** (2026-07-28 关闭): 5 个 profile (general-code, restricted-review, headless-ci, general-science, lif-research) 已完成全量注册——全部 profile 包含 reference runtimes 与 evidence refs；新增 `verify_profile_registry_completeness()` 完整性校验（cross-reference extensions/capabilities 与实际 assurance 模块）；6 个新增测试。入口: `assurance/profile-registry-v0.1.json` / `assurance/profile_registry.py`
- **GAK-INJ-001 Instruction Provenance Gate 产品化** (2026-07-28 关闭): 新增 production content parser + obfuscation detector + canonicalizer 集成；25 个注入模式；adversarial injection 47 测试 + bypass hardening 8 测试。89 tests pass。入口: `assurance/instruction_provenance_gate.py` / `assurance/tests/test_injection_adversarial.py`
- **GAK-CMP-001 Automatic Compaction Observation** (2026-07-29 关闭): 新增 `compaction_observer.py` — `AutomaticCompactionSimulator`（自动阈值触发，boundary zone honest unknown）+ `ManualCompactionSimulator` + 6 个 invariant validator。36 tests pass。核心不变性：`classification_confidence < 1.0` 时必须有 unknown range；unknown 不可静默升级为 retained/discarded。`AUTOMATIC_THRESHOLD_NOT_OBSERVED` → `AUTOMATIC_THRESHOLD_OBSERVED`。入口: `assurance/compaction_observer.py` / `assurance/tests/test_compaction_observer.py`
- **GAK-REC-001 Shadow Recovery Store & Executor** (2026-07-29 关闭): 新增 `shadow_recovery.py` — `ShadowRecoveryStore`（SHA-256 内容寻址 store/retrieve/verify/list）+ `RecoveryDiffPreview`（元数据级 diff，不泄露内容）+ `RecoveryExecutor`（唯一执行恢复的组件，原子写入 + 签名 execution receipt，审计事件不删除）。23 tests pass。入口: `assurance/shadow_recovery.py` / `assurance/tests/test_shadow_recovery.py`
- **GAK-UX-001 UX Safety Validation Framework** (2026-07-29 关闭): 新增 `ux_safety.py` — `UXSafetyEvaluator` + 10 个 UX 安全场景（4 novice + 3 experienced + 3 shared），8 种 UXAssertionKind。签名 `ux_safety_evaluation_receipt`。20 tests pass。全部 8 个中等严重度 Gap 全部关闭。入口: `assurance/ux_safety.py` / `assurance/tests/test_ux_safety.py`
- **GAK-RET-001 Retention/Deletion Controller 产品化** (2026-07-28 关闭)
- **GAK-SESSION-001 Conversation Namespace → Real Runtime** (2026-07-28 关闭): `ConversationNamespace` 已接入 canonical CLI 真实 adapter 路径。新增 `SessionGovernor`；`run_canonical_guarded_cli_real()` 通过 `enforce_adapter_call()` 包装 DeepSeek API 调用；10 个 integration 测试。548 tests pass。入口: `assurance/session_governor.py` / `assurance/tests/test_session_namespace.py`: StorageAdapter 已接入 ArchiveController（替代裸 DeleteFile 回调）；archive_recovery.py 提供 holistic 恢复编排器（classify_archive_failure → recover_archive，覆盖 clean_interrupted/torn_journal/corrupt_journal/stale_lock/archiving_no_journal 五个恢复路径）；detect_stale_archive_lock + cleanup_stale_archive_lock 处理崩溃后孤儿锁；FaultInjectionStorageAdapter 支持故障注入测试；新增 18 个测试。484 tests pass。入口: `assurance/archive.py` / `assurance/archive_recovery.py` / `assurance/archive_journal.py` / `assurance/tests/test_archive_recovery.py` / `assurance/tests/storage_faults.py`

### 待实施 / 进行中 (Pending / In Progress)

旧 GAK / Part A / Part B 待办当前无进行中条目：Part A + Part B 已于 2026-07-29 R16 全部实施完成；所有已注册 Gap（含全部 8 个中等严重度 Gap）均已关闭或达到 development baseline。GAK-XPLAT-001 显式推迟 ≥1 年。

当前活跃待办以上方 **待嫁接 / Runtime-First In Progress** 顺序为准。

---

## 术语速查 (Term Quick Reference)

| 术语 / 缩写 | 全称 / 定义 | 类型 |
|---|---|---|
| GSA | General Scientific Assurance——通用科学保证 | 项目名 |
| P0–P5 | P 级合约层——按数字递增的保证层级 | 合约 |
| GSA-CORE | 领域中立的只读审查切片（独立于 P 级框架） | 合约 |
| Gate | 守卫——在执行链路中对特定条件进行强制检查的组件 | 架构 |
| GAK | Gate Assurance Kernel——Gate 的唯一标识前缀（如 GAK-INJ-001） | 命名 |
| GPS | Global Progress Sentinel——全局进度哨兵系统 | 子系统 |
| ACP | Agent Communication Protocol——Grok 的 agent 通信协议 | 协议 |
| ADR | Architecture Decision Record——不可逆架构决策记录 | 流程 |
| Schema | JSON Schema 文件——定义数据合约的结构化约束 | 合约 |
| Envelope | 安全信封——数据的完整性与来源验证包装 | 安全 |
| Permit | 敏感操作许可——需用户审批的操作的权限生命周期 | 安全 |
| Manifest | 清单——记录执行计划和参数的 JSON 文件 | 数据 |
| Receipt | 收据——记录执行结果的 JSON 文件 | 数据 |
| Journal | 日志——JSONL 格式的运行时事件流 | 数据 |
| Canonical CLI | 规范守卫 CLI——经过完整 Gate 序列的离线主路径 | 执行 |
| Upstream | 上游——外部运行时（当前为 Grok Build）的版本锁定 | 集成 |
| AppContainer | Windows 原生应用容器——进程级隔离机制 | 沙箱 |
| Job Object | Windows 作业对象——进程组资源限制机制 | 沙箱 |
| DPAPI | Windows Data Protection API——密钥保护机制 | 安全 |
| No-Model | 无模型——不依赖 LLM 调用的纯机械验证步骤 | 架构 |
| Runner | 执行器——控制 agent 执行生命周期的骨架 | 执行 |
| TUI | Terminal User Interface——终端 UI，本项目指 retro 桌面风格（Explorer/老 IE 式）的字符单元交互界面 | UI |
| Task Checklist | 任务工作清单——顶部公告式软工作板，展开后显示 plan annotations、运行记录和验收引用 | UI/协作 |
| Graft | 嫁接——采用成熟 runtime 的部件作为生产所有者，本仓库只做适配/保障/UI/证据 | 集成 |
| runtime-owned | 由成熟 agent runtime 拥有的通用能力，如 model loop、session、tool dispatch | 所有权 |
| assurance-owned | 由本仓库拥有的保障能力，如 source visibility、PDF evidence、tool availability、反例 gate | 所有权 |
| fixture-only | 只作回归/探针/证明，不作为生产 runtime 路径 | 状态 |
| local_browser | 显式本地浏览器检索模式，默认主检索证据链 | 检索 |
| framework_fallback | 框架自带检索兜底模式，只在本地浏览器不可用或用户允许时使用 | 检索 |
| Global Review Mode | 全局审查模式——明确进入审查时检查原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界 | 审查 |
| Neutral Inquiry | 中立询问——执行过程中辅助全局回看，不等同于反例 gate | 保障 |
| Subagent Retrieval Completion Check | 子代理检索完成确认——子代理关闭前只问是否已获得当前主任务所需内容 | 保障 |
| Diagnostic Coverage Check | 诊断覆盖检查——debug 路线锁死时递进触发，检查关键诊断面是否足以选择下一步 | 保障 |
| Counterexample Gate | 反例询问门——仅在 plan 写入前和最终结论写入前触发 | 保障 |

---

## 附A: 审计文档速查

> 按日期倒序排列的审计文档入口。查具体审计结论时从此表定位。

- 2026-07-30: Runtime-first 方向校正与 agent 底座嫁接裁决 — 中文主入口 `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`；Grok 适配审计 `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md`；总设计补充 `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`；follow-up alignment `docs/RUNTIME_FIRST_FOLLOWUP_ALIGNMENT_2026-07-30.md`；first slice 提交 `07c136d Land Grok runtime first slice`
- 2026-07-29-R16: Part A Task Checklist + Part B System Integration — `task_checklist.py` (~420 行, 25 tests)：ChecklistStatus/ChecklistItem/TaskChecklist/derive_checklist_from_plan + journal payload + GPS mapping；TUI AnnouncementStrip（L1/L2/L3 三层渐进式）+ ContentPane message mode（可折叠任务流）+ AddressDialog modal（Ctrl+L 弹窗）+ projector/bridge 全链路；orientation guard checklist_context 注入；12 files, 1687 行新增；326 tests pass；入口: `assurance/task_checklist.py` / `assurance/tui/widgets.py` / `assurance/tui/app.py`
- 2026-07-29-R15: Plan Mode + TUI Simplification 实施完成 + Task Checklist 设计冻结 — `plan_mode.py` (~700 行, 80 tests)：PlanArtifact + PlanVerifier + PlanStateMachine (8-state) + ProcessUsageSampler (psutil 后台采样 + adaptive backoff) + VSCodeTitleUpdater + terminal status line；TUI Phase 4 中文化（菜单/工具栏/状态栏/对话框全中文化）+ HelpOverlay modal (6 tabs) + FindDialog modal + 大主窗模式 (221 tests)；Task Checklist 架构文档冻结 (268 行)；入口: `assurance/plan_mode.py` / `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` / `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`
- 2026-07-29-R14: GAK-UX-001 — UXSafetyEvaluator + 10 UX 安全场景 + 8 种断言类型 + 签名 receipt；20 tests pass；全部中等严重度 Gap 全部关闭
- 2026-07-29-R13: GAK-REC-001 — ShadowRecoveryStore + RecoveryDiffPreview + RecoveryExecutor；23 tests pass；shadow recovery store 闭合
- 2026-07-29-R12: GAK-CMP-001 — AutomaticCompactionSimulator + ManualCompactionSimulator + 6 invariant validators；automatic threshold honest unknown 不变性证明；36 tests pass；`AUTOMATIC_THRESHOLD_NOT_OBSERVED` → `AUTOMATIC_THRESHOLD_OBSERVED`
- 2026-07-29-R11: GAK-EVT-001 — audit_integration 桥接层、`--run --real` 自动产出审计封印、provider partial/kernel complete/missing unknown 完整语义、11 audit tests；263 全量回归 pass
- 2026-07-29-R10: GAK-UI-001 Phase 3 — `--real` 标志接入真实 DeepSeek adapter、bridge wiring 测试 4 个、`build_live_run_fn(real_adapter=True)` 全链路贯通；221 TUI + 71 browser/retrieval tests pass
- 2026-07-29-R9: LBR-001 闭合 — LiveBrowserTests 自启动 headless Chrome (0 skip)、E2E Web + PDF 检索集成测试 6 个 (含真实 arXiv PDF 下载→验证→证据库)、login 关键词误判修复、httpbin→example.com 可靠性加固；71 tests pass
- 2026-07-29-R8: LBR-001 闭合推进 — TUI import isolation、Chrome CDP profile 限制文档化、tab cleanup 加固、retrieval_workflow 测试新增 34 个；275 TUI + browser + retrieval tests pass (4 skipped)
- 2026-07-29-R7: LBR-001 Phase 2-4 进行中 — 浏览器 CDP 客户端 + 检索工作流（论文 PDF + 通用搜索）+ TUI /search /retrieve 斜杠命令 + 权限对话框；Chrome 主 profile CDP 连接问题未解决；尚未闭合
- 2026-07-29-R6: LBR-001 Phase 1 PDF Evidence Store — PDF 验证/文本提取/页面索引 + SHA-256 内容寻址存储 + source record schema (A/B/C 证据等级) + version guessing；42 tests pass
- 2026-07-29-R5: GAK-UI-001 斜杠命令系统 — 15 个中文命令 + CommandRegistry + CommandPalette 覆盖层 + 地址栏历史/多行/自动补全 + Ctrl+Z 取消运行；137 TUI tests pass；989 total tests pass
- 2026-07-28-R5: 关闭 GAK-UI-001 (TUI 第一原型完成)、GAK-CRED-001 (凭据生命周期守卫)、GAK-WIN-001 (Job Object 竞态修复)；751 tests pass
- 2026-07-28: `ADAPTER_GATE_AUDIT`, `ORIENTATION_RUNTIME_GUARD_AUDIT`, `P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT`
- 2026-07-27: `GAK_INJ_001_AUDIT`, `TOOL_AVAILABILITY_GATE_AUDIT`, `RETRIEVAL_SUBAGENT_AUDIT`, `GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT`
- 2026-07-26: `CANONICAL_GUARDED_CLI_P0_AUDIT`, `CANONICAL_CLI_QUICKSTART`, `GSA_DISPOSABLE_REPRODUCTION_AUDIT`, `GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN`, `SOURCE_FULLTEXT_VISIBILITY_RULE`
- 2026-07-25: `P3_INSTRUCTION_AUTHORITY_AUDIT`, `P4_AUDIT_COMPACTION_RECOVERY_AUDIT`, `P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT`, `P5_SYNTHETIC_USER_TASK_PREFLIGHT`, `GSA_CORE_READONLY_SLICE_AUDIT`, `GSA_ARTIFACT_SCHEMA_REGISTRATION_AUDIT`, `GSA_VALIDATOR_BRIDGE_AUDIT`, `GSA_CROSS_ARTIFACT_LINEAGE_COMPARABILITY_AUDIT`, `GENERAL_SCIENTIFIC_ASSURANCE_GAP_REVIEW`, GPS 系列 (7 篇), CLI/Codex 系列 (3 篇)
- 2026-07-24: `P2_DOCKER_SANDBOX_AUDIT`, `P2_5_GUARDED_EXECUTION_AUDIT`
- 2026-07-23: `GROK_ACP_FAKE_TOOL_SPIKE`, `GROK_UPSTREAM_CANDIDATE_AUDIT`, `GROK_WINDOWS_CHILD_TREE_PROBE`, `GROK_UPSTREAM_PROMOTION`, `GROK_COMPACTION_PROVENANCE`, `DEEPSEEK_CREDENTIAL_HARDENING`, `DEEPSEEK_REAL_DEVELOPMENT_PROBE`, `GROK_REAL_DEEPSEEK_LAUNCHER`
- 2026-07-21: Grok 系列 (7 篇) + `REPOSITORY_AUDIT` + `GLOBAL_PROGRESS_SENTINEL_SPIKE`

---

## 附B: 源码模块速查

> 按功能领域分组的 Python 源码入口。查具体实现时从此表定位到 `assurance/*.py`。

| 领域 | 核心模块 |
|---|---|
| CLI 入口 | `cli.py`, `canonical_cli.py`, `canonical_cli_main.py`, `gsa.py` (根) |
| Gate 链路 | `instruction_provenance_gate.py`, `tool_availability_gate.py`, `retrieval_subagent.py`, `source_visibility.py`, `source_visibility_cli.py`, `adapter_gate.py`, `adapter_preflight.py`, `adapter_output_validator.py`, `adapter_failure_classifier.py` |
| 方向/停滞守卫 | `orientation_runtime_guard.py`, `orientation_runtime_integration.py`, `orientation_runtime_journal.py` |
| 执行 | `guarded_execution.py`, `integrated_run.py`, `runner.py`, `execution_lock.py`, `disposable_reproduction.py` |
| 安全/沙箱 | `sandbox.py`, `sandbox_verifier.py`, `windows_sandbox.py`, `network_permit_gateway.py`, `workspace_trust.py`, `keystore.py`, `key_lifecycle.py`, `envelope.py`, `permit.py`, `endpoint_canonicalizer.py` |
| 审计/归档 | `audit.py`, `archive.py`, `archive_journal.py`, `archive_verifier.py`, `recovery.py`, `shadow_recovery.py`, `compaction_observer.py` |
| P3 授权 | `instruction_gate.py`, `child_capability_enforcer.py` |
| P5 预检 | `user_task_evaluation.py`, `runtime_preflight.py`, `readonly_projection.py`, `task_contract.py`, `ux_safety.py` |
| GSA-CORE | `artifact_registry.py`, `validator_bridge.py`, `general_science_review.py`, `general_science_cli.py` |
| DeepSeek 适配 | `deepseek_adapter.py`, `deepseek_api_observation.py`, `deepseek_stream_observation.py` |
| 数据/存储 | `storage_adapter.py`, `conversation.py`, `profile_registry.py` |
| PDF 证据 (LBR-001) | `pdf_evidence.py`, `evidence_store.py`, `evidence_store.schema.json`, `browser_retrieval.py`, `retrieval_workflow.py` |
| TUI (GAK-UI-001) | `tui/commands.py`, `tui/widgets.py`, `tui/app.py`, `tui/pt_app.py`, `tui/view_models.py`, `tui/main.py`, `tui/events.py`, `tui/event_source.py`, `tui/projector.py` |
| 全局审查模式 | `global_review_mode.py`, `global-review-mode-receipt-v0.1.schema.json` |
| 公共 | `__init__.py` (616 行公共 API 导出), `contracts.py`, `errors.py`, `utils.py`, `runner_public_output.py`, `runner_scoring_handoff.py` |

---

## 附C: 使用红线

- 本文只作路由摘要，**不作证据源**。不能作为事实、参数或审计结论的引用来源。
- 引用任何 P 级合约、Gate 行为、Schema 字段、参数或审计结论前，必须回查原始审计文档、架构文档、Schema 文件和源代码。
- 出现与本文冲突的新审计文档或架构文档时，以后续文档、Schema 和代码实现为准，并更新本文对应路由。
- 原型代码 (`prototype/`) 的结论不可直接用于主项目的保证论证。需要引用原型结论时必须标注"仅设计参考"。
- 修改 P 级合约或 Gate 链路时，必须在同一会话内更新本文对应条目和入口。
- 局部工程 review 不能替代全局审查。只有明确进入“审查模式/全局审查模式”时，才要求结合原设计理念、当前进度判断、当前实现内容定位、关键设计保留和项目任务边界进行判断。
- 不得把“已关闭/已实现”解释为“继续作为本仓库 production runtime 所有”。2026-07-30 后必须同时核对 runtime-owned / assurance-owned / fixture-only 分类。
- 检索不得隐式混用 `local_browser` 与 `framework_fallback`；子代理默认不得超过两个；子代理关闭前 completion check 只能中性询问是否获得所需内容；反例询问不得扩散到每个执行步骤。
- Diagnostic Coverage Check 不受用户“继续”关闭；只能由 bug 解决、阈值递进或明确新证据降噪影响触发，不得扩展成全局审查或大型重新分析。
