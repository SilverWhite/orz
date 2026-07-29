# CLI_PROJECT_INDEX

**更新**: 2026-07-29 (R11：GAK-EVT-001 development baseline 达成 — audit_integration 桥接 canonical CLI → AuditLedger、`--run --real` 自动产出审计封印、11 audit integration tests + 263 全量回归 pass)
**定位**: GSA (General Scientific Assurance) 项目主召回索引 / 组件路由。本文收录**项目架构、P 级合约、Gate 链路、审计文档、Schema 体系、运行时集成与关键设计约束**的召回入口，目标是让后续开发与回查可便捷定位到正确的文档、源码或 Schema。
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
- **Tool Availability Gate** (2026-07-27, 事实): 机械式工具可用性探测——在模型调用前验证工具声明与运行时实际可用性一致性，检测 belief mismatch/stagnation。入口: `docs/TOOL_AVAILABILITY_GATE_AUDIT_2026-07-27.md` / `assurance/tool_availability_gate.py`
- **Retrieval Subagent** (2026-07-27, 事实/2026-07-28 关闭): 无模型检索子代理——结构化检索结果的 fixture。2026-07-28: 升级为真实 DeepSeek v4 Pro 驱动的 Project Doc Retrieval Subagent（`dispatch_retrieval_subagent` + `ProjectDocIndex` + `SessionGovernor`）；offline 模式支持；37 tests pass。入口: `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md` / `assurance/retrieval_subagent.py` / `assurance/project_doc_index.py` / `assurance/tests/test_retrieval_subagent_real.py`
- **Orientation Runtime Guard** (2026-07-27, 事实): 中性方向检查点 + 运行时停滞守卫——检测 agent 是否陷入循环/停滞。别名: orientation checkpoint, stagnation guard。入口: `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md` / `assurance/orientation_runtime_guard.py` / `assurance/orientation_runtime_integration.py` / `assurance/orientation_runtime_journal.py` / `assurance/orientation-checkpoint-v0.1.schema.json` / `assurance/runtime-stagnation-guard-receipt-v0.1.schema.json`
- **Workspace Trust Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-TRUST-001): 统一工作区信任——全部 Python 入口点（canonical CLI + 检索子代理 ×2）在 IPG 评估前建立 workspace trust receipt。`AdapterGateContext` 携带 trust_receipt + trust_status。别名: GAK-TRUST-001。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py` / `assurance/retrieval_subagent.py`
- **Source Visibility Gate** (2026-07-26, 事实): 全文可见性检查 Gate——验证引用源的完整文本可见性。入口: `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md` / `assurance/source_visibility.py` / `assurance/source_visibility_cli.py`
- **Local Browser Retrieval and PDF Evidence** (2026-07-28 设计冻结, 2026-07-29 关闭): 本地浏览器外部检索与论文 PDF 确定性证据链。Phase 1-4 全部完成：Evidence Store (42 tests)、CDP 客户端 (27 tests)、检索工作流 (34 tests)、TUI 集成 (217 tests)。2026-07-29 R9 闭合：LiveBrowserTests 改为自启动 headless Chrome（0 skip，不再需手动启动 Chrome）、新增 E2E 集成测试 6 个（Web 检索 5 + 论文 PDF 检索 1，包含真实 arXiv PDF 下载→验证→证据库全流程）、修复 `classify_page` login 关键词误判（arXiv 导航栏 "log in" 不再误触发 LOGIN_REQUIRED）、测试 URL 从 httpbin.org 迁移至 example.com 消除间歇性超时。全量 71 browser + retrieval + E2E tests pass (0 skip)。已知限制: Chrome 主 profile 受企业安全策略拦截 CDP 连接，workaround 为项目隔离 profile (`.gsa_chrome_profile/`)。别名/关键词: browser retrieval, local browser, PDF evidence, LBR-001, CDP, GSA Chrome profile。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `assurance/pdf_evidence.py` / `assurance/evidence_store.py` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/evidence_store.schema.json` / `assurance/tests/test_browser_retrieval.py` / `assurance/tests/test_retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **Child Capability Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-CHILD-001): 子代理/子进程 capability 传递——`spawn_child_context()` 统一入口；检索子代理接受 parent_envelope；`execute_guarded_no_model_action()` 在 Docker create 前调用 `enforce_child_capabilities()`。别名: GAK-CHILD-001。入口: `assurance/child_capability_enforcer.py` / `assurance/instruction_gate.py` / `assurance/retrieval_subagent.py` / `assurance/guarded_execution.py`
- **Network Permit Gate** (2026-07-28, 事实/2026-07-28 关闭 GAK-NET-001): 网络许可统一覆盖——`call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy；`_resolve_and_setup_gates()` 构建 guarded 模式默认 policy。别名: GAK-NET-001。入口: `assurance/network_permit_gateway.py` / `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **Adapter Gate** (2026-07-27, 事实): 适配器调用旁路执行 Gate——防止运行时绕过适配器直接调用模型。别名: adapter gate enforcement, adapter preflight。入口: `docs/ADAPTER_GATE_AUDIT_2026-07-28.md` / `assurance/adapter_gate.py` / `assurance/adapter_preflight.py` / `assurance/adapter_output_validator.py` / `assurance/adapter_failure_classifier.py` / `assurance/adapter-gate-enforcement-receipt-v0.1.schema.json`
- **Canonical Guarded CLI Gate 序列** (2026-07-26, 设计约束): 离线主路径 Gate 序列定义: instruction provenance gate → tool availability gate → orientation checkpoint → source visibility gate → fake DeepSeek adapter boundary → answer packet → runtime JSONL journal → independent verifier。入口: `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md` / `assurance/canonical_cli.py`

### C. 运行时集成 (Runtime Integration)

- **Grok Build** (2026-07-21~23, 事实/参考运行时): 当前参考运行时。通过 upstream build lock 管理版本。ACP 协议集成（initialize, fake tool, event bridge, compaction provenance, workspace trust, child tree probe）。含 fixtures (workspace-control-surfaces: AGENTS.md, config, hooks, skills)。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` / `upstream/grok-build.lock.json` / `docs/GROK_*.md` / `integration/grok/` / `integration/grok/fixtures/`
- **DeepSeek Adapter** (2026-07-23, 事实/首个真实适配器): 首个真实模型适配器——通过 DeepSeek API 进行 one-shot/stream 观察。凭证硬化已完成。入口: `docs/DEEPSEEK_CREDENTIAL_HARDENING_2026-07-23.md` / `architecture/DEEPSEEK_ADAPTER_CONTRACT_v0.1.md` / `assurance/deepseek_adapter.py` / `assurance/deepseek_api_observation.py` / `assurance/deepseek_stream_observation.py`
- **Codex App Server** (2026-07-25, 事实/生命周期研究): Codex CLI 的 app server 生命周期捕获、标准化与验证——用于研究 agent 生命周期模式。入口: `docs/CODEX_APP_SERVER_LIFECYCLE_NORMALIZER_AUDIT_2026-07-25.md` / `docs/CLI_SESSION_LIFECYCLE_ADAPTER_AUDIT_2026-07-25.md` / `runtime/**/codex-app-server-*.schema.json`
- **VS Code Integration** (2026-07, 事实/仅字节码): VS Code ACP 主机集成——direct-deepseek-acp-host, local-deepseek-acp-host, windows-job-host。⚠ 源码 `.py` 文件未入库，仅 `__pycache__/*.pyc` 字节码残留；另有 `real-acp-r0-plugin/hooks/` 目录。入口: `integration/vscode/`

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

- **gsa CLI 入口** (2026-07, 事实): 顶级 CLI 分发器——`gsa doctor`, `gsa source gate`, `gsa run`, `gsa verify`。注意: 当前为命令行参数模式；未来 TUI 将通过此入口启动（CLI_UI_INTERACTION_MODEL 定义交互语法，本入口保持为启动点）。入口: `gsa.py` / `assurance/cli.py`；TUI 设计: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`
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
- **CLI UI Interaction Model** (v0.1, 事实/2026-07-29 Phase 2 事件流桥接完成): Windows-only 复古桌面/老 IE 风格终端 UI 交互模型。第一原型已完成（9 个模块 ~3900 行）：静态框架 + prompt_toolkit + 自定义 retro widget 层（10 个 widget 类）、17 个中文斜杠命令 + CommandRegistry + CommandPalette 覆盖层、地址栏命令历史/多行输入/自动补全、Ctrl+Z 取消运行。Phase 2 已完成：canonical CLI 事件流桥接——`bridge.py`（映射层，导入 assurance.* 的唯一 TUI 模块）、`LiveRunEventSource`（后台线程实时消费 CLI 事件）、`JsonlFileSource`（journal 重放，payload 解析为 typed event 子类）、3 个 typed gate event（IPG/Tool Avail/Orientation）、`--run`/`--replay` CLI 参数。217 tests pass。别名: retro TUI, Explorer-style UI, old IE UI, terminal TUI。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md`；代码: `assurance/tui/`（`bridge.py` / `commands.py` / `widgets.py` / `app.py` / `pt_app.py` / `view_models.py` / `main.py` / `events.py` / `event_source.py` / `projector.py`）；Gap Register: GAK-UI-001
- **Grok Build 适配** (v0.1, 设计约束): 以 Grok Build 为参考运行时的适配策略。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md`
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

- **assurance/ Schema 文件** (83 个 JSON Schema): 核心保证层的所有数据合约——receipt, manifest, event, envelope, gate receipt, sandbox profile, checkpoint, journal, retrieval, disposable reproduction 等。入口: `assurance/**/*.schema.json`
- **runtime/ Schema 文件** (47 个 JSON Schema): 运行时控制面的数据合约——run-manifest, run-event, scenario-export, leak-scan, CLI session lifecycle (v0.1/v0.2), Codex app server capture/normalization/verification/turn-probe, GPS review/disposition/checkpoint/holistic/transition/controller/state-reduction, journal recovery, deepseek adapter/development, network approval, action kernel, model loop, windows process。入口: `runtime/**/*.schema.json`
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

### 已关闭 (Closed)

- **GAK-UI-001 CLI UI Interaction Model** (2026-07-28 第一原型完成, 2026-07-29 Phase 2 事件流桥接完成, 2026-07-29 Phase 3 真实 adapter 接入完成): 9 个模块 ~3900 行。Phase 2 已完成 canonical CLI 事件流桥接——`bridge.py`（唯一允许导入 assurance.* 的 TUI 模块）、`LiveRunEventSource`（后台线程实时消费 CLI 事件）、`JsonlFileSource` payload 解析 → typed event 子类、3 个 typed gate event、projector 更新、--run/--replay CLI 参数。Phase 3 已完成真实 DeepSeek adapter 接入——`main.py` 新增 `--real` 和 `--credential-target` 标志、`build_live_run_fn(real_adapter=True)` 接线到 `run_canonical_guarded_cli_real`、真实 Gate 决策流通过 bridge → event_source → projector → retro 渲染全链路贯通。221 TUI tests pass。入口: `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` / `assurance/tui/`
- **LBR-001 Local Browser Retrieval and PDF Evidence** (2026-07-29 关闭): Phase 1-4 全链路完成 + R9 闭合。Evidence Store (42 tests)、CDP 客户端 (27 tests)、检索工作流 (34 tests)、TUI 集成 (217 tests)。R9 新增: ① LiveBrowserTests 自启动 headless Chrome——不再需手动启动 Chrome，`setUpClass` 通过 `_ensure_browser()` 自动启停；skip 条件从 "Chrome 未运行" 改为 "Chrome 未安装"（4 tests pass, 0 skip）。② E2E 集成测试 `test_browser_retrieval_e2e.py` (6 tests)：Web 检索 5 (retrieve_urls 单页/多页/空列表/错误处理/progress callback) + 论文 PDF 检索 1 (run_retrieval arXiv → 下载 → 验证 → 证据库，验证 SHA-256、PDF header、metadata.json、pages.jsonl)。③ 修复 `classify_page` login 检测——"log in"/"login" 从关键词列表移除（arXiv 等网站导航栏中的 login 链接不再误触发 LOGIN_REQUIRED），改为仅匹配强信号 ("please log in to", "log in to continue", "authentication required" 等)。④ 测试可靠性加固——httpbin.org 替换为 example.com/example.org 消除间歇性 WebSocket 超时。全量 71 tests pass (0 skip)。已知限制: Chrome 主 profile 受企业安全策略拦截 CDP 连接，workaround 使用项目隔离 profile (`.gsa_chrome_profile/`)。入口: `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md` / `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/tests/test_browser_retrieval.py` / `assurance/tests/test_retrieval_workflow.py` / `assurance/tests/test_browser_retrieval_e2e.py`
- **GAK-TRUST-001 Workspace Trust 统一入口** (2026-07-28 关闭): `establish_workspace_trust()` 已接入 canonical CLI 和检索子代理全部入口点。`AdapterGateContext` 携带 trust receipt；`_resolve_and_setup_gates()` 在 IPG 前建立 trust；`validate_all_entry_points_establish_trust()` AST 审计覆盖全部入口点。入口: `assurance/workspace_trust.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-CHILD-001 Child Capability 统一传递** (2026-07-28 关闭): 新增 `spawn_child_context()` 统一入口（组合 predicate check + 签名 child envelope 创建）；检索子代理接受可选 `parent_envelope`，通过 `spawn_child_context()` 创建签名 child envelope 并传入 `ConversationNamespace.create(parent_envelope_id=...)`。入口: `assurance/child_capability_enforcer.py` / `assurance/retrieval_subagent.py`
- **GAK-NET-001 Network Permit 统一覆盖** (2026-07-28 关闭): `call_deepseek_api()` 在 HTTP 请求前调用 `evaluate_network_permit()`；`AdapterGateContext` 携带 network_policy/endpoint/category；`_resolve_and_setup_gates()` 构建 `guarded` 模式默认 policy。入口: `assurance/deepseek_adapter.py` / `assurance/adapter_gate.py` / `assurance/canonical_cli.py`
- **GAK-ID-001 Key Rotation/Revocation 产品化** (2026-07-28 关闭): `_FileLock` 文件锁保护所有 key mutation；crash-safe journal（pending → receipt → history → clear）；`recover_pending_rotations()` 恢复；`migrate_envelope()` 真实 envelope 重签名；`rotate()` 支持 `envelopes_to_migrate`。入口: `assurance/key_lifecycle.py` / `assurance/envelope.py`
- **GAK-SBX-001 Windows Native Sandbox** (2026-07-27 关闭): Development baseline 已达成——elevated 路径 compliant（netsh firewall + AppContainer + Job Object），non-elevated 路径 fail-closed。原始 TCP 残余记录为已知 Windows 平台限制，不阻塞 development 门禁。入口: `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- **GAK-WIN-001 Windows Job Object Race** (2026-07-28 关闭): `PROC_THREAD_ATTRIBUTE_JOB_LIST` 内核级 Job Object 原子绑定——进程创建时在内核中分配 Job，消除 `CreateProcess`→`AssignProcessToJobObject` 用户态竞态窗口。`CREATE_SUSPENDED` + `TokenIsAppContainer` + `IsProcessInJob` 在 `ResumeThread` 前全部验证。`PROC_THREAD_ATTRIBUTE_JOB_LIST` 不可用时优雅降级至 post-creation 分配。11 tests pass, 1 skipped（旧 OS 自动跳过）。入口: `assurance/windows_sandbox.py:856-983` / `assurance/tests/test_windows_race_escape.py:383-970`
- **GSA-PROFILE-001 Profile Registry 全量注册** (2026-07-28 关闭): 5 个 profile (general-code, restricted-review, headless-ci, general-science, lif-research) 已完成全量注册——全部 profile 包含 reference runtimes 与 evidence refs；新增 `verify_profile_registry_completeness()` 完整性校验（cross-reference extensions/capabilities 与实际 assurance 模块）；6 个新增测试。入口: `assurance/profile-registry-v0.1.json` / `assurance/profile_registry.py`
- **GAK-INJ-001 Instruction Provenance Gate 产品化** (2026-07-28 关闭): 新增 production content parser + obfuscation detector + canonicalizer 集成；25 个注入模式；adversarial injection 47 测试 + bypass hardening 8 测试。89 tests pass。入口: `assurance/instruction_provenance_gate.py` / `assurance/tests/test_injection_adversarial.py`
- **GAK-RET-001 Retention/Deletion Controller 产品化** (2026-07-28 关闭)
- **GAK-SESSION-001 Conversation Namespace → Real Runtime** (2026-07-28 关闭): `ConversationNamespace` 已接入 canonical CLI 真实 adapter 路径。新增 `SessionGovernor`；`run_canonical_guarded_cli_real()` 通过 `enforce_adapter_call()` 包装 DeepSeek API 调用；10 个 integration 测试。548 tests pass。入口: `assurance/session_governor.py` / `assurance/tests/test_session_namespace.py`: StorageAdapter 已接入 ArchiveController（替代裸 DeleteFile 回调）；archive_recovery.py 提供 holistic 恢复编排器（classify_archive_failure → recover_archive，覆盖 clean_interrupted/torn_journal/corrupt_journal/stale_lock/archiving_no_journal 五个恢复路径）；detect_stale_archive_lock + cleanup_stale_archive_lock 处理崩溃后孤儿锁；FaultInjectionStorageAdapter 支持故障注入测试；新增 18 个测试。484 tests pass。入口: `assurance/archive.py` / `assurance/archive_recovery.py` / `assurance/archive_journal.py` / `assurance/tests/test_archive_recovery.py` / `assurance/tests/storage_faults.py`

### 待实施 / 进行中 (Pending / In Progress)

（当前无进行中条目。所有已注册 Gap 均已关闭或达到 development baseline。）

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

---

## 附A: 审计文档速查

> 按日期倒序排列的审计文档入口。查具体审计结论时从此表定位。

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
| 审计/归档 | `audit.py`, `archive.py`, `archive_journal.py`, `archive_verifier.py`, `recovery.py` |
| P3 授权 | `instruction_gate.py`, `child_capability_enforcer.py` |
| P5 预检 | `user_task_evaluation.py`, `runtime_preflight.py`, `readonly_projection.py`, `task_contract.py` |
| GSA-CORE | `artifact_registry.py`, `validator_bridge.py`, `general_science_review.py`, `general_science_cli.py` |
| DeepSeek 适配 | `deepseek_adapter.py`, `deepseek_api_observation.py`, `deepseek_stream_observation.py` |
| 数据/存储 | `storage_adapter.py`, `conversation.py`, `profile_registry.py` |
| PDF 证据 (LBR-001) | `pdf_evidence.py`, `evidence_store.py`, `evidence_store.schema.json`, `browser_retrieval.py`, `retrieval_workflow.py` |
| TUI (GAK-UI-001) | `tui/commands.py`, `tui/widgets.py`, `tui/app.py`, `tui/pt_app.py`, `tui/view_models.py`, `tui/main.py`, `tui/events.py`, `tui/event_source.py`, `tui/projector.py` |
| 公共 | `__init__.py` (392 行公共 API 导出), `contracts.py`, `errors.py`, `utils.py`, `runner_public_output.py`, `runner_scoring_handoff.py` |

---

## 附C: 使用红线

- 本文只作路由摘要，**不作证据源**。不能作为事实、参数或审计结论的引用来源。
- 引用任何 P 级合约、Gate 行为、Schema 字段、参数或审计结论前，必须回查原始审计文档、架构文档、Schema 文件和源代码。
- 出现与本文冲突的新审计文档或架构文档时，以后续文档、Schema 和代码实现为准，并更新本文对应路由。
- 原型代码 (`prototype/`) 的结论不可直接用于主项目的保证论证。需要引用原型结论时必须标注"仅设计参考"。
- 修改 P 级合约或 Gate 链路时，必须在同一会话内更新本文对应条目和入口。
