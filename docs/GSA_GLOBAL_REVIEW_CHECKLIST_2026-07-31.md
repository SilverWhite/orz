# GSA 项目全局审查清单

**创建日期**: 2026-07-31
**状态**: 进行中（临时审计清单，审查完成后归档或删除）
**范围**: 项目全部设计与实现——P 级合约、Gate 链路、安全沙箱、运行时集成、TUI、GPS、LIF 科学保障、Schema 体系、测试基础设施、跨领域一致性
**触发方式**: `gsa review global` 激活后使用本清单逐层审查

---

## 第 0 层：方向与架构一致性（最高优先级）

> 检查 A0 裁决是否已贯彻到代码、文档和 Schema 中。`docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`

- [ ] **A0-1** Runtime-First 裁决一致性: Grok = production runtime owner；本仓库 = assurance/evidence/UI/adapter/fixture。入口: `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- [ ] **A0-2** `canonical_cli.py` fixture-only 标记: 已标记为 conformance/assurance fixture，不充当 production runtime。入口: `assurance/canonical_cli.py` / `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- [ ] **A0-3** 两个 Subagent 约束: 项目文档检索 + 外部检索，不超过两个；关闭前 neutral completion check。入口: `assurance/retrieval_subagent.py` / `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- [ ] **A0-4** Retrieval mode 显式约束: `local_browser` / `framework_fallback` / `off` 三选一，不得隐式并存或切换。入口: `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `architecture/LOCAL_BROWSER_RETRIEVAL_AND_PDF_EVIDENCE_v0.1.md`
- [ ] **A0-5** Neutral vs Counterexample 触发分离: 反例仅在 plan/conclusion 写入前触发；中立询问仅执行中触发。入口: `assurance/orientation_runtime_guard.py` / `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md`
- [ ] **A0-6** Design Sequence Deviation 修正: `--grok-mode` 默认为 `acp-smoke`（ACP 主可观测路径），`prompt_smoke` 降级为窄 smoke 备选。入口: `assurance/cli.py` / `docs/GROK_CLI_SPECIALIZATION_ADAPTATION_AUDIT_2026-07-30.md` §7
- [ ] **A0-7** Diagnostic Coverage Check: 硬信号递进阈值 `2→3→4→5`、bug 解决重置、新证据降噪、用户"继续"不关闭。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`

## 第 1 层：P 级合约审计（P0–P5 + GSA-CORE）

> 设计文档 ↔ Schema ↔ 代码三方一致性。

- [ ] **P0** 数据语义层: `contracts.py` + `envelope.py` ↔ schema ↔ `docs/CANONICAL_GUARDED_CLI_P0_AUDIT_2026-07-26.md`
- [ ] **P1** 会话身份与归档: `conversation.py` + `permit.py` + `archive.py` + `archive_journal.py` + `archive_recovery.py` + `archive_verifier.py` + `session_governor.py` ↔ `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md`
- [ ] **P2** Docker 沙箱: `sandbox.py` + `sandbox_verifier.py` ↔ `docs/P2_DOCKER_SANDBOX_AUDIT_2026-07-24.md`
- [ ] **P2** Windows 辅助沙箱: `windows_sandbox.py` ↔ `docs/GAK_SBX_001_WINDOWS_NATIVE_SANDBOX_AUDIT_2026-07-27.md`
- [ ] **P2.5** 守卫执行: `guarded_execution.py` ↔ `docs/P2_5_GUARDED_EXECUTION_AUDIT_2026-07-24.md`
- [ ] **P3** 指令授权: `instruction_gate.py` + `instruction_provenance_gate.py` + `child_capability_enforcer.py` ↔ `docs/P3_INSTRUCTION_AUTHORITY_AUDIT_2026-07-25.md`
- [ ] **P4** 审计/压缩/恢复: `audit.py` + `archive_verifier.py` + `recovery.py` + `compaction_observer.py` + `shadow_recovery.py` ↔ `docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md`
- [ ] **P4.5** 工作区优先: `integrated_run.py` + `workspace_trust.py` + `network_permit_gateway.py` ↔ `docs/P4_5_WORKSPACE_FIRST_INTEGRATION_AUDIT_2026-07-25.md`
- [ ] **P5** 合成任务/预检: `user_task_evaluation.py` + `runtime_preflight.py` + `readonly_projection.py` + `task_contract.py` + `ux_safety.py` ↔ `docs/P5_SYNTHETIC_USER_TASK_PREFLIGHT_2026-07-25.md`
- [ ] **GSA-CORE** 领域中立审查: `artifact_registry.py` + `validator_bridge.py` + `general_science_review.py` ↔ `docs/GSA_CORE_READONLY_SLICE_AUDIT_2026-07-25.md`

## 第 2 层：Gate 链路逐一审查

> 每个 Gate 的设计意图 ↔ 实现 ↔ 测试覆盖。

- [ ] **GAK-INJ-001** 指令来源 Gate: 25 个注入模式 + canonicalizer 集成 + obfuscation detector + 89 tests。入口: `assurance/instruction_provenance_gate.py` / `docs/GAK_INJ_001_AUDIT_2026-07-27.md`
- [ ] **Tool Availability Gate**: ACP dual verification (`allow_once` + `cancel_permission` 同时 valid)。入口: `assurance/tool_availability_gate.py` / `assurance/grok_tool_permission_observer.py`
- [ ] **Retrieval Subagents**: 恰好两个 + parent_envelope 传递 + completion check。入口: `assurance/retrieval_subagent.py` / `docs/RETRIEVAL_SUBAGENT_AUDIT_2026-07-27.md`
- [ ] **Orientation Runtime Guard**: 停滞检测 + Diagnostic Coverage Check 递进阈值。入口: `assurance/orientation_runtime_guard.py` / `docs/ORIENTATION_RUNTIME_GUARD_AUDIT_2026-07-28.md`
- [ ] **Workspace Trust Gate (GAK-TRUST-001)**: 全部入口点 coverage + AdapterGateContext 携带 trust receipt。入口: `assurance/workspace_trust.py`
- [ ] **Source Visibility Gate**: 全文可见性检查。入口: `assurance/source_visibility.py` / `docs/SOURCE_FULLTEXT_VISIBILITY_RULE_2026-07-26.md`
- [ ] **Child Capability Gate (GAK-CHILD-001)**: `spawn_child_context()` 统一入口 + `execute_guarded_no_model_action()` 前 `enforce_child_capabilities()`。入口: `assurance/child_capability_enforcer.py`
- [ ] **Network Permit Gate (GAK-NET-001)**: 所有 HTTP 调用前 `evaluate_network_permit()`。入口: `assurance/network_permit_gateway.py` / `assurance/deepseek_adapter.py`
- [ ] **Adapter Gate**: preflight + output validation + failure classification + schema receipt。入口: `assurance/adapter_gate.py` / `assurance/adapter_preflight.py` / `assurance/adapter_output_validator.py` / `assurance/adapter_failure_classifier.py`
- [ ] **Local Browser Retrieval (LBR-001)**: CDP 客户端 + 检索工作流 + PDF evidence store + E2E tests (71 pass)。入口: `assurance/browser_retrieval.py` / `assurance/retrieval_workflow.py` / `assurance/pdf_evidence.py` / `assurance/evidence_store.py`

## 第 3 层：安全与沙箱

- [ ] **Docker Sandbox**: 进程隔离 + 网络限制 + 状态可观测。入口: `assurance/sandbox.py` / `assurance/sandbox_verifier.py`
- [ ] **Windows Native Sandbox (GAK-SBX-001)**: AppContainer + Job Object + netsh firewall；elevated compliant, non-elevated fail-closed。入口: `assurance/windows_sandbox.py`
- [ ] **Job Object Containment (P1)**: `CREATE_SUSPENDED` + `AssignProcessToJobObject` + `ResumeThread`；Kill-On-Close；全部 Grok 启动路径 coverage；与 GAK-WIN-001 `PROC_THREAD_ATTRIBUTE_JOB_LIST` 等价裁定。入口: `assurance/job_object_supervisor.py` / `docs/JOB_OBJECT_CONTAINMENT_SUFFICIENCY_JUDGMENT_2026-07-31.md`
- [ ] **GAK-WIN-001**: `PROC_THREAD_ATTRIBUTE_JOB_LIST` 内核级原子绑定 + 优雅降级。入口: `assurance/windows_sandbox.py:856-983`
- [ ] **Keystore**: 安装级密钥存储（内存 + Windows DPAPI）。入口: `assurance/keystore.py`
- [ ] **Key Lifecycle (GAK-ID-001)**: rotation + revocation + crash-safe journal + envelope 重签名。入口: `assurance/key_lifecycle.py`
- [ ] **Envelope**: 数据完整性与来源验证。入口: `assurance/envelope.py`
- [ ] **Permit**: 敏感操作许可生命周期。入口: `assurance/permit.py`
- [ ] **Endpoint Canonicalizer**: 路径遍历 + SSRF 防护。入口: `assurance/endpoint_canonicalizer.py`
- [ ] **Storage Adapter**: 统一文件系统访问 + ArchiveController 集成。入口: `assurance/storage_adapter.py`
- [ ] **Subprocess Containment (P1 补充)**: 所有 Grok subprocess 调用路径启用 Job Object 包裹。入口: `assurance/grok_tool_permission_observer.py` / `docs/GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`

## 第 4 层：运行时集成

- [ ] **Grok Runtime Adapter**: `contained_run()`, `run_grok_headless_once()`, `run_grok_acp_once()`, `run_with_job_object_containment()` 全部 `CREATE_SUSPENDED`。入口: `assurance/grok_runtime_adapter.py`
- [ ] **Grok Event Normalizer**: normalized events + metadata-only projection + 5 个新事件类型。入口: `assurance/grok_event_normalizer.py`
- [ ] **Grok Prompt/Tool Gate**: dual ACP + adapter containment → `decision: allow`；fail-closed 默认；`--grok-execute` 显式执行。入口: `assurance/grok_prompt_tool_gate.py`
- [ ] **Grok Lifecycle Projection**: metadata-only GPS journal events + 中性 Orientation context。入口: `assurance/grok_lifecycle_projection.py`
- [ ] **Grok Session Verifier**: 7 项 post-run 交叉核验 (acp_lifecycle_complete, session_id_consistent, tool_call_count_match, permission_count_match, permission_outcomes_match, transcript_has_expected_lifecycle, event_hash_chain_valid) + 14 tests。入口: `assurance/grok_session_verifier.py`
- [ ] **Grok Profile Drafts**: 两个 agent profiles (`.grok/agents/`) + 一个 workflow (`.grok/workflows/`) + Rhai 语法验证。入口: `assurance/grok_profile_drafts.py`
- [ ] **Grok Tool Permission Observer**: 静态 registry/permission 观察 + availability projection。入口: `assurance/grok_tool_permission_observer.py`
- [ ] **DeepSeek Adapter**: 首个真实适配器 + 凭证硬化 + stream/one-shot 双模式。入口: `assurance/deepseek_adapter.py`
- [ ] **Codex/VS Code Lifecycle**: 保留为观测基础，不升级为完整 VS Code 插件。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- [ ] **Upstream Lock**: `grok-build.lock.json` → `0.2.112 (9bbd559437)` + candidate gate 全量通过。入口: `upstream/grok-build.lock.json` / `docs/GROK_0_2_112_CANDIDATE_GATE_2026-07-30.md`

## 第 5 层：TUI 系统

- [ ] **TUI 核心架构**: `app.py` + `pt_app.py` + `widgets.py` (10 widget 类) + `view_models.py` + `main.py`。入口: `assurance/tui/`
- [ ] **TUI 事件系统**: `events.py` (typed gate events) + `event_source.py` (LiveRunEventSource, JsonlFileSource, Grok pipe)。入口: `assurance/tui/events.py` / `assurance/tui/event_source.py`
- [ ] **TUI 桥接层**: canonical CLI ↔ TUI, Grok ↔ TUI, Plan mode ↔ TUI, Task Checklist ↔ TUI。入口: `assurance/tui/bridge.py`
- [ ] **TUI 命令系统**: 17 个中文斜杠命令 + CommandRegistry + CommandPalette + 地址栏历史/多行/自动补全。入口: `assurance/tui/commands.py`
- [ ] **TUI 投影层**: checklist announcement strip + 可折叠任务流 + 大主窗模式 + HelpOverlay modal + FindDialog modal。入口: `assurance/tui/projector.py`
- [ ] **TUI 设计文档一致性**: ↔ `architecture/CLI_UI_INTERACTION_MODEL_v0.1.md` + `architecture/CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` + `architecture/TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md`
- [ ] **TUI 测试**: 221 tests pass + Plan mode 80 tests + Checklist 25 tests。入口: `assurance/tui/` / `assurance/tests/test_plan_mode.py` / `assurance/tests/test_task_checklist.py`

## 第 6 层：GPS 系统（Global Progress Sentinel）

- [ ] **GPS 合约**: 架构定义。入口: `architecture/GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md`
- [ ] **GPS Holistic Gate**: 跨会话 holistic 进度评估。入口: `docs/GLOBAL_PROGRESS_HOLISTIC_GATE_AUDIT_2026-07-25.md`
- [ ] **GPS Checkpoint Adapter**: 策略化检查点保存/恢复。入口: `docs/GLOBAL_PROGRESS_CHECKPOINT_ADAPTER_AUDIT_2026-07-25.md`
- [ ] **GPS Transition Gate**: 状态迁移守卫逻辑。入口: `docs/GLOBAL_PROGRESS_TRANSITION_GATE_AUDIT_2026-07-25.md`
- [ ] **GPS Atomic Journal & Reason Migration**: 原子日志 + reason code 迁移 ↔ `protocol/reason-codes-v0.1.yaml`。入口: `docs/GLOBAL_PROGRESS_ATOMIC_JOURNAL_AND_REASON_MIGRATION_AUDIT_2026-07-25.md`
- [ ] **GPS Journal Recovery**: 日志恢复。入口: `docs/GLOBAL_PROGRESS_JOURNAL_RECOVERY_AUDIT_2026-07-25.md`
- [ ] **GPS Runtime Controller + Verifier**: 运行时控制 + 验证。入口: `docs/GLOBAL_PROGRESS_RUNTIME_CONTROLLER_AUDIT_2026-07-25.md` / `docs/GLOBAL_PROGRESS_CONTROLLER_VERIFIER_AUDIT_2026-07-25.md`
- [ ] **GPS State Reducer**: 跨会话状态聚合。入口: `docs/GLOBAL_PROGRESS_STATE_REDUCER_AUDIT_2026-07-25.md`
- [ ] **GPS Grok Lifecycle Projection**: metadata-only 约束 + GPS journal events 投影。入口: `assurance/grok_lifecycle_projection.py`

## 第 7 层：LIF 科学保障（D2.15-20）

> 2026-07-31 第一切片完成。Design from `GROK_BUILD_ADAPTATION_v0.1` §4。

- [ ] **D2.14 SourceRouter**: 明确不实现（LIF 项目通用纪律，非本仓库科学性问题）。
- [ ] **D2.15 EvidenceKernel**: action/evidence/claim 三层独立状态机；action↛evidence↛claim。入口: `assurance/runner_scoring_handoff.py`
- [ ] **D2.16 ClaimBoundary**: measured / direct comparison / bridge hypothesis / promotion eligibility 四种类型。入口: `assurance/tests/test_runner_scoring.py`
- [ ] **D2.17 LeakScanner**: 机械检查 + 独立语义审阅双通道；evaluation/holdout fail-closed。入口: runtime schema
- [ ] **D2.18 ScenarioExporter**: 匿名场景包导出、不泄露原始数据/模型内部状态。入口: `regression/cases-v0.1.yaml` + runtime schema
- [ ] **D2.19 EvaluationRunner**: 冻结配置 + append-only journal + oracle 物理隔离。入口: `evaluation/SCORING_PROTOCOL_v0.1.md`
- [ ] **D2.20 CaseRetrievalGuard**: blind-first 原则——precommitment 之后检索历史案例。入口: `architecture/GROK_BUILD_ADAPTATION_v0.1.md` §4 第 9 项
- [ ] **D2.21 ClaimRegistrySync**: 明确不实现（LIF 项目通用纪律，非本仓库科学性问题）。

## 第 8 层：D3 设计分离

- [ ] **D3.22 Finding ↔ Permission 分离**: Finding（不可变/content-addressed/scanner 观察）+ PermissionRecord（独立决策/SHA-256 引用/不嵌入内容）+ FindingRegistry（session-scoped 线程安全）。不变量: Finding 不含 permission 字段；PermissionRecord 不含 severity/evidence 字段。入口: `assurance/finding_registry.py` / `docs/D3_22_FINDING_PERMISSION_SEPARATION_DECISION_2026-07-31.md`
- [ ] **D3.23 Audit shadow_refs**: `AuditLedger.seal()` 可选 `shadow_refs` 参数（commit_sha + tree_sha，40-char Git SHA）→ `audit-seal-receipt` schema。入口: `assurance/audit.py` / `assurance/shadow_recovery.py`

## 第 9 层：Schema 体系

> 抽样核验关键 Schema 是否与实际代码字段一致。

- [ ] **assurance/ Schema (95 个)**: 抽样——receipt, manifest, event, envelope, gate receipt, sandbox profile, checkpoint, journal, retrieval, disposable reproduction。入口: `assurance/**/*.schema.json`
- [ ] **runtime/ Schema (48 个)**: run-manifest, run-event, scenario-export, leak-scan, GPS series, CLI session lifecycle, Codex app server, deepseek adapter。入口: `runtime/**/*.schema.json`
- [ ] **integration/grok/ Schema (25 个)**: observed plan, fake provider, tool continuity, workspace discovery/trust, event bridge, ACP initialize/fake-tool, windows child tree, compaction provenance, real deepseek。入口: `integration/grok/**/*.schema.json`
- [ ] **protocol/ 定义**: agent-protocol, reason-codes, gate-matrix, global-progress reason code migration。入口: `protocol/`
- [ ] **regression/ Schema**: case-corpus, coverage-matrix, fixture, historical-excerpt-provenance。入口: `regression/`
- [ ] **evaluation/ Schema**: evaluation-result, partition-manifest。入口: `evaluation/`
- [ ] **upstream/ Schema**: grok-build-lock, grok-build-candidate。入口: `upstream/`

## 第 10 层：测试基础设施

- [ ] **assurance/tests/ (68 文件)**: 每个核心模块对应测试文件。
- [ ] **P 级合约测试覆盖**: P0-P5 + GSA-CORE 全覆盖。
- [ ] **Gate 测试覆盖**: instruction provenance (89 tests), tool availability, retrieval subagent (real), orientation guard, adapter gate (integration)。
- [ ] **integration/grok/tests/ (10 文件)**: ACP initialize, fake-tool, windows child tree, timeout gate split, fake provider, compaction provenance, real deepseek launcher。
- [ ] **runtime/tests/ (8 文件)**: GPS, CLI session lifecycle, Codex app server。
- [ ] **回归测试语料库**: 30 historical + 10 synthetic + 5/5 fixtures。入口: `regression/cases-v0.1.yaml` / `regression/fixtures/`
- [ ] **CI Pipeline**: GitHub Actions — Ubuntu + Windows, Python 3.11/3.12, repository check, compileall, PowerShell 语法检查, 单元测试。入口: `.github/workflows/ci.yml`

## 第 11 层：跨领域一致性

- [ ] **所有权分类一致性**: runtime-owned / assurance-owned / fixture-only 在所有文档和代码注释中一致。入口: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- [ ] **ADR 合规**: ADR-0001 (证据约束本地 agent 内核)、ADR-0002 (推迟云端运行时)、ADR-0003 (运行时中立保证内核)、ADR-0004 (通用科学 Profile 分层) 不可变决策未被违反。入口: `adr/`
- [ ] **使用红线合规**: CLI_PROJECT_INDEX.md 附 C 的 9 条红线在代码中是否被遵守。入口: `CLI_PROJECT_INDEX.md` 附 C
- [ ] **术语一致性**: GSA / Gate / P 级 / GAK / GPS / ACP / Envelope / Receipt / Manifest / Journal / Canonical CLI / Upstream 在所有文档中同义使用。
- [ ] **废弃标记**: `prototype/fep_agent_proto/` 明确标记为废弃/仅设计参考。入口: `architecture/D_SALVAGE_MATRIX_v0.1.md`
- [ ] **旧 CLI 入口标记**: `p2_cli.py`, `p45_cli.py`, `p5_cli.py` 标记为向后兼容保留。
- [ ] **设计文档交叉引用完整性**: 架构文档 ↔ 审计文档 ↔ ADR 之间的交叉引用不出现循环或死链。
- [ ] **Schema ↔ 代码字段一致性**: 抽样检查关键 Schema (receipt, gate, sandbox profile, session lifecycle) 的字段定义是否与对应 Python 代码中 `build_*_receipt()` 的输出字段严格匹配。

---

## 审查进度追踪

| 层级 | 名称 | 状态 | 审查日期 | 发现问题数 |
|------|------|------|----------|------------|
| L0 | 方向与架构一致性 | ✅ 完成 | 2026-07-31 | 5 (1🔴 / 3🟡 / 1🟢) |
| L1 | P 级合约 | 待审查 | - | - |
| L2 | Gate 链路 | 待审查 | - | - |
| L3 | 安全与沙箱 | 待审查 | - | - |
| L4 | 运行时集成 | 待审查 | - | - |
| L5 | TUI 系统 | 待审查 | - | - |
| L6 | GPS 系统 | 待审查 | - | - |
| L7 | LIF 科学保障 | 待审查 | - | - |
| L8 | D3 设计分离 | 待审查 | - | - |
| L9 | Schema 体系 | 待审查 | - | - |
| L10 | 测试基础设施 | 待审查 | - | - |
| L11 | 跨领域一致性 | 待审查 | - | - |

## 审查规则

1. 每层完成后在进度表中更新状态、日期和发现问题数。
2. 发现的问题按严重度分级：🔴 阻断（架构违规、安全漏洞）、🟡 设计偏离（文档不一致、未贯彻裁决）、🟢 文档改进（可改进的表述或交叉引用）。
3. 发现的具体问题写入单独的问题清单 `docs/GSA_GLOBAL_REVIEW_FINDINGS_2026-07-31.md`，本清单只追踪进度。
4. 引用任何 P 级合约、Gate 行为或 Schema 字段前，必须回查原始审计文档、架构文档和源代码——本清单只作路由指引。
5. 出现新文档或代码修改与本清单冲突时，以后续文档和代码为准，并更新本清单对应条目。
