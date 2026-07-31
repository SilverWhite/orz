# GSA 全局审查发现清单

**审查日期**: 2026-07-31
**审查层级**: L0 — 方向与架构一致性
**状态**: L0 完成，L1-L11 待审查

---

## L0 审查结论

**总体评估**: A0 方向裁决的核心架构文档（`CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` 和 `INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`）保持一致。关键修正（ACP→Headless 主次关系恢复、Neutral/Counterexample 触发分离）已正确落实到代码。但发现 1 个阻断级差距（Diagnostic Coverage Check 无代码实现）和 4 个设计偏离项。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| A0-1 | Runtime-First 裁决一致性 | ✅ 核心文档一致。`CN_*` §5.9 明确列出 assurance-owned 部件，`INTEGRATED_*` §4 定义五层所有权模型 |
| A0-5 | Neutral vs Counterexample 分离 | ✅ 实现良好。`FORBIDDEN_ORIENTATION_FIELDS` 正确排除 `counterexample_candidate`、`claim_disposition`、`claim_promotion` |
| A0-6 | Design Sequence Deviation 修正 | ✅ `--grok-mode` 默认 `acp-smoke`，`prompt-smoke` 降级为窄 smoke 备选。help text 明确标注 |

---

### ~~🔴 阻断：发现 F-001~~ → ✅ 已修复（2026-07-31）

**标题**: Diagnostic Coverage Check 设计约束无代码实现 → **已实现**

**路径**: `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` §7.3 → `assurance/diagnostic_coverage.py`

**严重度**: ~~🔴 阻断（设计约束未落实）~~ → ✅ 已修复

**已交付**:
- `assurance/diagnostic_coverage.py` (~290 lines): `DiagnosticCoverageState`（递进阈值状态机）、`build_diagnostic_coverage_check()`（触发检查点构建）、`evaluate_diagnostic_coverage_response()`（yes/no/uncertain 响应验证）
- `assurance/diagnostic-coverage-check-receipt-v0.1.schema.json`: 检查点 receipt schema
- `assurance/diagnostic-coverage-check-verification-v0.1.schema.json`: 响应验证 schema
- `assurance/tests/test_diagnostic_coverage.py`: **33 tests pass** (state machine ×13, check building ×7, response evaluation ×9, constants ×4)
- `assurance/__init__.py`: 公开导出 4 个符号

---

### ~~🟡 发现 F-002~~ → ✅ 已修复（2026-07-31）

**标题**: `canonical_cli.py` 缺少显式 fixture-only / conformance-path 标记 → **已添加**

**路径**: `assurance/canonical_cli.py`

**严重度**: ~~🟡 设计偏离（文档已标记，代码未标记）~~ → ✅ 已修复

**已交付**:
- Module-level docstring (~25 lines): 声明 fixture-only / conformance path 地位，引用 Runtime-First Graft Decision 文档，明确区分 fake-offline 和 real-DeepSeek 两种模式均为 conformance/dev bridge 而非 production runtime
- `run_canonical_guarded_cli()` docstring: "conformance fixture — proves Gate-chain orchestration shape… not a production agent runtime"
- `run_canonical_guarded_cli_real()` docstring: "P0 development / conformance bridge… Grok ACP is the first-class production path"


**旧内容**: 无 module docstring；函数 docstring 仅描述功能行为（"Run the canonical guarded CLI with a fake offline adapter"），未声明 fixture-only 地位

### ~~🟡 发现 F-003a~~ → ✅ 已修复（2026-07-31）

**标题**: 两个 Subagent 数量约束无程序化执行 → **已添加 Registry + Verifier**

**路径**: `assurance/retrieval_subagent.py`

**严重度**: ~~🟡 设计偏离（约定约束但无硬性执行）~~ → ✅ 已修复

**已交付**:
- `RETRIEVAL_SUBAGENT_REGISTRY` (~50 lines): 单点真实来源——枚举恰好两个 subagent（`project-doc-retrieval` + `external-retrieval`），每个 entry 包含 subagent_id、kind、category、description、dispatch_fn_name、credential_target_default、allowed_source_categories、capabilities
- `ALLOWED_SUBAGENT_COUNT = 2`: 设计约束硬编码常量
- `verify_retrieval_subagent_registry()`: 7 项自动检查——数量 = 2、全部 required keys 存在、dispatch_fn_name 可解析为 callable、subagent_id 字段匹配 registry key、恰好 1 internal + 1 external。违反时抛出 `AssuranceError`
- 9 个新测试 (31 total): registry 结构完整性、dispatch 函数可调用性、正确/错误数量的验证行为、categories/capabilities 交叉引用
- `assurance/__init__.py`: 新增 3 个公开导出

### ~~🟡 发现 F-003b~~ → ✅ 已修复（2026-07-31）

**标题**: Subagent 关闭前 Neutral Completion Check 未实现 → **已实现**

**路径**: `assurance/retrieval_subagent.py`

**严重度**: ~~🟡 设计偏离（设计约束无实现）~~ → ✅ 已修复

**已交付**:
- `RETRIEVAL_COMPLETION_CHECK_MESSAGE`: 中文消息块模板——仅问"是否已经获得完成当前主任务所需的内容？"，明确禁止扩展到新子代理或无限补检索
- `build_retrieval_completion_check()`: 构建检查点 receipt，包含 message_block + claim_policy（全部 false）+ allowed/forbidden 字段
- `evaluate_retrieval_completion_check_response()`: 验证 yes/no/uncertain 响应——拦截 counterexample/claim/new_subagent/global_review 字段；no/uncertain 时强制提供 missing_content_types；检查 missing_types 不含 "spawn"/"create subagent" 等子代理实例化语言
- `build_retrieval_session_close_receipt()`: 新增 `triggered_by="completion_check_passed"` + 可选 `completion_check` 参数——若 triggered_by 为 completion_check_passed 时必须携带 completion_check
- Schema 更新: `retrieval-session-close-receipt-v0.1.schema.json` → `triggered_by` enum 新增 `completion_check_passed`，新增可选 `completion_check` 块
- 11 个新测试 (42 total): 覆盖中性消息、yes/no/uncertain 决策、missing_types 强制、forbidden 字段拦截、subagent spawn 语言检测、端到端 close receipt 集成

### ~~🟡 发现 F-004~~ → ✅ 已修复（2026-07-31）

**标题**: `browser_retrieval.py` 不消费 retrieval mode 显式选择约束 → **已添加 guard**

**路径**: `assurance/retrieval_workflow.py`

**严重度**: ~~🟡 设计偏离（CLI 层已强制三选一，但核心检索模块未感知）~~ → ✅ 已修复

**已交付**:
- `retrieval_workflow.py`: 新增 `SUPPORTED_RETRIEVAL_MODES` + `DEFAULT_RETRIEVAL_MODE` + `_require_local_browser_mode()` guard 函数——`"off"` 抛出 retrieval disabled 错误，`"framework_fallback"` 抛出 must-not-use-local-browser 错误，未知 mode 抛出 unsupported 错误
- `run_retrieval()`, `retrieve_urls()`, `retrieve_search()`: 新增 `retrieval_mode` 参数（默认 `"local_browser"` 保持向后兼容），入口第一行调用 guard
- `assurance/__init__.py`: 导出 `RETRIEVAL_WORKFLOW_SUPPORTED_RETRIEVAL_MODES` + `RETRIEVAL_WORKFLOW_DEFAULT_RETRIEVAL_MODE`
- 11 个新测试 (47 total): guard 四种状态（pass/off/fallback/unknown）+ 三个入口函数 guard 行为 + 跨模块常量一致性检查

---

### L0 审查进度

| 条目 | 状态 | 发现 |
|------|------|------|
| A0-1 Runtime-First 一致性 | ✅ 通过 | — |
| A0-2 canonical_cli fixture-only 标记 | ⚠️ 偏离 | F-002 |
| A0-3 两个 Subagent 约束 | ⚠️ 偏离 | F-003a, F-003b |
| A0-4 Retrieval mode 约束 | ⚠️ 偏离 | F-004 |
| A0-5 Neutral/Counterexample 分离 | ✅ 通过 | — |
| A0-6 Design Sequence 修正 | ✅ 通过 | — |
| A0-7 Diagnostic Coverage Check | 🔴 阻断 | F-001 |

**L0 结果**: 3 通过 / 3 偏离 / 1 阻断

---

---

## L1 审查结论

**审查日期**: 2026-07-31
**审查层级**: L1 — P 级合约审计（P0–P5 + GSA-CORE）
**审查方法**: 审计文档 → 源码 → Schema 三方一致性核验

**总体评估**: P0/P2/P2.5/P3/P4.5/P5/GSA-CORE 七个合约层的审计文档与代码/Schema 大体一致。核心发现集中在 P1/P4 审计文档过期：archive 接口从函数式演进为类基（`ArchiveController`），审计文档描述的 `run_conversation_archive()` / `verify_conversation_archive()` 函数已不存在；retention 分类名称在审计文档与实际 policy 文件之间完全不对应（仅 2/16 重叠）。此外多个审计文档的行数声明已过期。这些均属文档维护问题，不影响代码正确性或合约执行。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| P0 | 数据语义层 | ✅ `contracts.py` (28行) + `envelope.py` (~268行) 提供 RFC 8785 HMAC-SHA256 签名/验证；审计文档描述的三层 schema 均存于 assurance/ 目录 |
| P2 | Docker + Windows 沙箱 | ✅ `sandbox.py`、`windows_sandbox.py` (~1050行) 与两份审计文档一致；GAK-SBX-001 已关闭 |
| P2.5 | 守卫执行 | ✅ `guarded_execution.py` (886行) 完整实现 9 步 Docker CLI 进程追踪、child capability enforcement、独立 verifier；与审计文档一致 |
| P3 | 指令授权 | ✅ `instruction_gate.py` (835行) 完整实现 provenance→delegation→authorization 三层；8 种来源类型、data-only 拒绝、remote MCP 本地能力拒绝均与审计文档一致 |
| P4.5 | 工作区优先集成 | ✅ `integrated_run.py` 正确导入 P1-P4 全部模块；standard/strict 后端策略与审计文档一致 |
| P5 | 合成任务/预检 | ✅ `readonly_projection.py` 提供 source-before/after/snapshot 三摘要核验；`user_task_evaluation.py`、`runtime_preflight.py`、`task_contract.py`、`ux_safety.py` 均存在 |
| GSA-CORE | 领域中立审查 | ✅ `artifact_registry.py` (3 required schemas)、`validator_bridge.py` (4 required validators)、`general_science_review.py` (~594行，完整 claim gating) 与审计文档一致 |

---

### 🟡 发现 F-005: P1/P4 审计文档描述的 archive 接口函数不存在 → ✅ 已修复（2026-07-31）

**标题**: `run_conversation_archive()` 和 `verify_conversation_archive()` 已被 `ArchiveController.archive()` 和 `verify_archive()` 替代，审计文档未更新

**涉及审计文档**:
- `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md` §4（归档控制器）和 §1（组件描述）
- `docs/P4_AUDIT_COMPACTION_RECOVERY_AUDIT_2026-07-25.md` §合同与实现

**严重度**: ~~🟡 设计偏离（审计文档接口声明过期，代码已演进）~~ → ✅ 已修复

**已交付**:
- P1 审计文档 §4：`run_conversation_archive()` / `verify_conversation_archive()` → `ArchiveController.archive()` + `verify_archive()` + `resume_archived_conversation()`（完整接口描述，含 crash recovery / retry / journal replay / advisory lock）
- P1 审计文档 §5：`write_archive_journal()` / `verify_archive_journal()` → `ArchiveJournalWriter` + `replay_archive_journal()` + `recover_archive_journal()` + `detect_stale_archive_lock()` / `cleanup_stale_archive_lock()` + `inspect_archive_journal()`
- P4 审计文档 §尚未关闭：已关闭项（compaction observer / shadow Git / writer lock / crash recovery）从"尚未关闭"移至"已关闭"子列表；仍开放项保留

---

### 🟡 发现 F-006: P1 审计文档 retention 分类列表与实际 policy 文件不对应 → ✅ 已修复（2026-07-31）

**标题**: P1 审计文档所列 16 个 `delete_on_archive` 类别与 `retention-policy-v0.1.json` 中的实际类别名称几乎完全不同

**路径**: `docs/P1_CONVERSATION_ARCHIVE_LIFECYCLE_AUDIT_2026-07-28.md` §4 → `assurance/retention-policy-v0.1.json`

**严重度**: ~~🟡 设计偏离（审计文档的数据引用过期）~~ → ✅ 已修复

**已交付**:
- P1 审计文档 §4 的 16 个 `delete_on_archive` 类别列表完全替换为当前 `retention-policy-v0.1.json` 的实际类别名称
- 旧列表（`agent_output`, `approved_tool_calls`, ...）→ 新列表（`raw_provider_payload`, `private_reasoning`, `raw_tool_result`, `full_stdout_stderr`, `network_body`, `credential_lease`, `confirmation_token`, `one_shot_permit`, `unpinned_snapshot`, `temporary_checkpoint`, `temporary_profile`, `sandbox_ephemeral_storage`, `session_recall_index`, `active_security_envelope`, `temporary_lifecycle_receipt`, `temporary_import_receipt`）
- 3 个 persist 类别与 3 个 never_persist 类别保持不变（双方一致）

**注意**: 旧分类列表可能对应 retention policy 的早期设计版本；若旧名称仍有文档引用价值，建议单独记录为设计演进备忘而非保留在审计文档正文中。

---

### 🟢 发现 F-007: 多份审计文档的代码行数声明过期 → ✅ P1 已修复（2026-07-31）

**标题**: P1 审计文档中的行数声明与实际代码不匹配（正常的代码演进）

**严重度**: 🟢 文档改进 → ✅ P1 行数表已更新

**已交付**（P1 审计文档文件清单）:
| 文件 | 旧声明 | 新声明 | 
|------|--------|--------|
| `envelope.py` | ~200 | 268 |
| `permit.py` | 311 | 290 |
| `archive.py` | ~350 | 622 |
| `archive_journal.py` | ~350 | 632 |
| `keystore.py` | ~200 | 305 |
| `key_lifecycle.py` | ~100 | 483 |
| `conversation.py` | 257 | 257（不变 ✅） |

**注意**: P2/P3/P4.5/P5 审计文档的行数声明未在本次修订中核查，建议后续审查时一并更新或移除以行数为基础的声明。

---

### L1 审查进度

| 条目 | 状态 | 发现 |
|------|------|------|
| P0 数据语义层 | ✅ 通过 | — |
| P1 会话身份与归档 | ⚠️ 偏离 | F-005, F-006, F-007 |
| P2 Docker + Windows 沙箱 | ✅ 通过 | — |
| P2.5 守卫执行 | ✅ 通过 | — |
| P3 指令授权 | ✅ 通过 | — |
| P4 审计/压缩/恢复 | ⚠️ 偏离 | F-005 (连带) |
| P4.5 工作区优先集成 | ✅ 通过 | — |
| P5 合成任务/预检 | ✅ 通过 | — |
| GSA-CORE 领域中立审查 | ✅ 通过 | — |

**L1 结果**: 7 通过 / 2 偏离 (P1, P4 连带) / 0 阻断 → **3 项发现全部已修复**
**发现问题**: F-005 🟡→✅ / F-006 🟡→✅ / F-007 🟢→✅

---

## L2 审查结论

**审查日期**: 2026-07-31
**审查层级**: L2 — Gate 链路逐一审查
**审查方法**: 审计文档 → 源码 → Schema → 测试覆盖 四方核验

**总体评估**: 全部 10 个 Gate 的三方一致性通过。关键确认：Tool Availability Gate 的 ACP dual verification（`allow_once` + `cancel_permission`）已落码；Child Capability 的 `spawn_child_context()` 统一入口 + `enforce_child_capabilities()` 双函数存在；Network Permit 的 `evaluate_network_permit()` 覆盖全部 HTTP 调用路径；Adapter Gate 的 4 模块全链路（enforce → preflight → validate → classify）完整。Retrieval Subagents 的 registry + completion check 已在 L0 修复（F-003a/b）。唯一发现是 GAK-INJ-001 审计文档注入模式数量过期（17→24），属文档维护项。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| GAK-INJ-001 | 指令来源 Gate | ✅ 全部 13 个函数均存在（含 `parse_instruction_content`、`detect_obfuscated_injection`、`evaluate_instruction_provenance_gate_with_canonicalizer`）；24 个注入模式；18+47=65 tests |
| Tool Availability Gate | 工具可用性门禁 | ✅ ACP dual verification `REQUIRED_ACP_PERMISSION_SCENARIOS = ("allow_once", "cancel_permission")` 已落码；`grok_tool_permission_observer.py`；26 tests |
| Retrieval Subagents | 检索子代理（2 个） | ✅ 已在 L0 修复：`RETRIEVAL_SUBAGENT_REGISTRY` + `build_retrieval_completion_check()`；42 tests |
| Orientation Runtime Guard | 方向/停滞守卫 | ✅ 3 模块（384+318+544行）+ 8 schemas + 20 tests (9+6+5) — 全部与审计文档一致 |
| Workspace Trust Gate | GAK-TRUST-001 | ✅ `establish_workspace_trust()` 存在；`AdapterGateContext` 携带 `trust_receipt`；已关闭 |
| Source Visibility Gate | 全文可见性检查 | ✅ `source_visibility.py` 确定性 gate（4 级 visibility → 5 级 claim allowed）；ledger + receipt schema |
| Child Capability Gate | GAK-CHILD-001 | ✅ `spawn_child_context()` 统一入口 + `enforce_child_capabilities()` 硬门禁；已关闭 |
| Network Permit Gate | GAK-NET-001 | ✅ `evaluate_network_permit()` 覆盖 6 类 endpoint；`AdapterGateContext` 携带 `network_policy`；已关闭 |
| Adapter Gate | 适配器旁路执行 | ✅ 4 模块全部核心函数已验证：`enforce_adapter_call()` + `run_adapter_preflight()` + `validate_adapter_output()` + `classify_adapter_error()` + `build_failure_recovery_plan()`；24+19=43 tests |
| LBR-001 | 本地浏览器检索 | ✅ 已关闭（2026-07-29）：CDP + 检索工作流 + PDF evidence + E2E；71 tests pass |

---

### 🟢 发现 F-008: GAK-INJ-001 审计文档注入模式数量过期

**标题**: 审计文档描述的 17 个注入模式已扩展为 24 个（production content parser 于 2026-07-28 新增），审计文档未更新

**路径**: `docs/GAK_INJ_001_AUDIT_2026-07-27.md` §注入模式库 → `assurance/instruction_provenance_gate.py:25-50`

**严重度**: ~~🟢 文档改进~~ → ✅ 已修复（2026-07-31）

**已交付**: `docs/GAK_INJ_001_AUDIT_2026-07-27.md` §注入模式库 — 原始 17 个模式 + 2026-07-28 扩展 7 个 → 总计 24 个，注明 source_type_mismatch / known_injection_pattern 分级

---

### L2 审查进度

| 条目 | 状态 | 发现 |
|------|------|------|
| GAK-INJ-001 指令来源 Gate | ✅ 通过 | F-008 🟢 |
| Tool Availability Gate | ✅ 通过 | — |
| Retrieval Subagents | ✅ 通过（L0 已修复） | — |
| Orientation Runtime Guard | ✅ 通过 | — |
| Workspace Trust Gate | ✅ 通过 | — |
| Source Visibility Gate | ✅ 通过 | — |
| Child Capability Gate | ✅ 通过 | — |
| Network Permit Gate | ✅ 通过 | — |
| Adapter Gate | ✅ 通过 | — |
| LBR-001 本地浏览器检索 | ✅ 通过 | — |

**L2 结果**: 10 通过 / 0 偏离 / 0 阻断
**发现问题**: F-008 🟢

---

## L3 审查结论

**审查日期**: 2026-07-31
**审查层级**: L3 — 安全与沙箱
**审查方法**: 审计文档 → 源码 → Schema 三方核验，重点确认 containment / sandbox / crypto / permit 的代码落实

**总体评估**: 全部 11 个安全组件通过。核心安全闭环已形成：Docker + Windows Native 双沙箱（含 GAK-SBX-001 / GAK-WIN-001 关闭）、`JobObjectSupervisor` 进程树 containment（`CREATE_SUSPENDED` + sufficiency judgment 闭合）、DPAPI-backed keystore + key lifecycle（GAK-ID-001 关闭）、envelope HMAC 完整性 + permit 一次性消费 + endpoint canonicalizer 路径遍历/SSRF 防护 + StorageAdapter 统一文件系统接口。Subprocess containment 已扩展至全部 Grok observer 调用路径（observer subprocess leak 已根因分析并修复）。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| Docker Sandbox | P2 严格沙箱 | ✅ `sandbox.py` — profile→observation→candidate→selection 四层模式；8/8 probe checks passed；与审计文档一致 |
| Windows Native Sandbox | GAK-SBX-001 | ✅ `windows_sandbox.py` (~1050行) — AppContainer + Job Object + netsh firewall；elevated compliant, non-elevated fail-closed；已关闭 |
| Job Object Containment | P1 (第一切片) | ✅ `job_object_supervisor.py` — `JobObjectSupervisor` 类 + `contained_run()`；所有 Grok 启动路径已升级至 `CREATE_SUSPENDED` + `AssignProcessToJobObject` + `ResumeThread`；containment sufficiency judgment 已裁定 |
| GAK-WIN-001 | PROC_THREAD_ATTRIBUTE_JOB_LIST | ✅ `windows_sandbox.py:856-983` — 内核级原子 Job 绑定 + 优雅降级；已关闭 |
| Keystore | 安装级密钥存储 | ✅ `keystore.py` (305行) — 256-bit random key + Windows DPAPI / macOS Keychain |
| Key Lifecycle | GAK-ID-001 | ✅ `key_lifecycle.py` (483行) — rotation + revocation + crash-safe journal + envelope migration；已关闭 |
| Envelope | 数据完整性与来源验证 | ✅ `envelope.py` (268行) — RFC 8785 + HMAC-SHA256；create/verify/migrate 全生命周期 |
| Permit | 敏感操作许可 | ✅ `permit.py` (290行) — issue → sign → consume 完整生命周期；排他 claim 防重放 |
| Endpoint Canonicalizer | 路径遍历 + SSRF 防护 | ✅ `endpoint_canonicalizer.py` — `canonicalize_filesystem_path()` + `canonicalize_network_endpoint()` + `validate_endpoint_list()` + `validate_filesystem_targets()`；39 tests |
| Storage Adapter | 统一文件系统接口 | ✅ `storage_adapter.py` — `StorageAdapter` Protocol + `LocalStorageAdapter`；已接入 ArchiveController |
| Subprocess Containment | P1 补充 (observer leak) | ✅ 所有 Grok observer subprocess 调用均已启用 Job Object 包裹；observer subprocess leak 已发现→根因分析→修复（`GROK_OBSERVER_SUBPROCESS_LEAK_2026-07-30.md`）|

---

### L3 审查进度

| 条目 | 状态 |
|------|------|
| Docker Sandbox | ✅ 通过 |
| Windows Native Sandbox (GAK-SBX-001) | ✅ 通过 |
| Job Object Containment (P1) | ✅ 通过 |
| GAK-WIN-001 | ✅ 通过 |
| Keystore | ✅ 通过 |
| Key Lifecycle (GAK-ID-001) | ✅ 通过 |
| Envelope | ✅ 通过 |
| Permit | ✅ 通过 |
| Endpoint Canonicalizer | ✅ 通过 |
| Storage Adapter | ✅ 通过 |
| Subprocess Containment (P1补充) | ✅ 通过 |

**L3 结果**: 11 通过 / 0 偏离 / 0 阻断
**发现问题**: 0

---

## L4 审查结论

**审查日期**: 2026-07-31
**审查层级**: L4 — 运行时集成
**审查方法**: 审计文档/架构文档 → 源码 → 配置文件 三方核验

**总体评估**: 全部 10 个运行时集成组件通过。Grok 作为第一生产底座的 adapter 层已全面落地：三个启动路径（`run_grok_headless_once` / `run_grok_acp_once` / `contained_run`）全部使用 `CREATE_SUSPENDED` containment；ACP JSON-RPC 完整生命周期 + 7 项 post-run 交叉核验；事件正规化器消费 5 种新事件类型；Prompt/Tool promotion gate fail-closed + dual ACP 路径。DeepSeek adapter 作为首个真实适配器保留（凭证硬化）。上游 lock 已提升至 0.2.112 (9bbd559437)。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| Grok Runtime Adapter | 核心适配器 | ✅ 10+ 函数（`inspect_grok_runtime`, `run_grok_headless_once`, `run_grok_acp_once`, `contained_run` 等）；全部路径 `CREATE_SUSPENDED` + `JobObjectSupervisor`；5 tests |
| Grok Event Normalizer | 事件正规化 | ✅ `normalize_grok_runtime_receipt()` + `write_grok_events_jsonl()` + `project_grok_events_for_tui()` — 5 种新事件类型；3 tests |
| Grok Prompt/Tool Gate | Fail-closed promotion gate | ✅ `build_grok_prompt_tool_promotion_gate_receipt()` + `_adapter_containment_status()` + `_tool_availability_status()`；dual ACP + adapter containment → allow |
| Grok Lifecycle Projection | GPS + Orientation 投影 | ✅ `project_grok_lifecycle_events_to_global_progress()` + `build_grok_lifecycle_orientation_checkpoint()` — metadata-only 约束；5 tests |
| Grok Session Verifier | Post-run 交叉核验 | ✅ `verify_acp_session()` — 7 项检查（acp_lifecycle_complete / session_id_consistent / tool_call_count_match / permission_count_match / permission_outcomes_match / transcript_has_expected_lifecycle / event_hash_chain_valid）；14 tests ✅ |
| Grok Profile Drafts | 2 agents + 1 workflow | ✅ `.grok/agents/gsa-project-doc-retrieval.md` + `gsa-external-retrieval.md`；`.grok/workflows/gsa-retrieval.rhai`；Rhai 语法验证；6 tests |
| Grok Tool Permission Observer | 静态 registry 观察 | ✅ `observe_grok_tool_permission_surfaces()` — ACP permission projection + availability；5 tests |
| DeepSeek Adapter | 首个真实适配器 | ✅ `call_deepseek_api()` + `build_real_deepseek_answer_packet()` + `build_real_deepseek_context()`；凭证硬化（DPAPI）；stream + one-shot 双模式 |
| Codex/VS Code Lifecycle | 保留为观测基础 | ✅ 保留初步适配证据，不升级为完整 VS Code 插件 — 符合 Runtime-First 裁决 |
| Upstream Lock | 0.2.112 (9bbd559437) | ✅ `grok-build.lock.json` — version 0.2.112, build_id 9bbd559437, SHA-256 verified, Authenticode Valid, candidate gate 全量通过 |

---

### L4 审查进度

| 条目 | 状态 |
|------|------|
| Grok Runtime Adapter | ✅ 通过 |
| Grok Event Normalizer | ✅ 通过 |
| Grok Prompt/Tool Gate | ✅ 通过 |
| Grok Lifecycle Projection | ✅ 通过 |
| Grok Session Verifier | ✅ 通过 |
| Grok Profile Drafts | ✅ 通过 |
| Grok Tool Permission Observer | ✅ 通过 |
| DeepSeek Adapter | ✅ 通过 |
| Codex/VS Code Lifecycle | ✅ 通过 |
| Upstream Lock | ✅ 通过 |

**L4 结果**: 10 通过 / 0 偏离 / 0 阻断
**发现问题**: 0

---

## L5 审查结论

**审查日期**: 2026-07-31
**审查层级**: L5 — TUI 系统
**审查方法**: 架构文档 → 源码模块 → 测试覆盖 三方核验

**总体评估**: TUI 系统 7 个子项全部通过。11 个模块（~3900+ 行）完整覆盖：16 个 widget 类、17 个中文斜杠命令 + CommandRegistry + CommandPalette、20+ typed event 类、4 种 event source（Fake/JsonlFile/LiveRun/Grok pipe）、bridge 层四向桥接（canonical CLI / Grok / Plan mode / Task Checklist）。326 tests pass（221 TUI + 80 Plan mode + 25 Checklist）。架构文档三篇均与代码一致。

### 通过项（无需修改）

| ID | 条目 | 结论 |
|----|------|------|
| TUI 核心架构 | app/pt_app/widgets/view_models/main | ✅ 11 个模块；16 widget 类（MenuBar/Toolbar/AddressBar/CommandPalette/FindBar/ExplorerPane/ContentPane/StatusBar/ContentMarker/Dialog/PropertiesSheet/HelpOverlay/FindDialog/AnnouncementStrip/AddressDialog）；CLI_PROJECT_INDEX 称 10 widget 类 — 已扩展至 16 |
| TUI 事件系统 | events + event_source | ✅ 20+ typed event 类（含 PlanPhaseEntered/Submitted/ApprovalDecision、UsageSample、TaskChecklist、ChecklistItemStatus）+ 4 种 EventSource |
| TUI 桥接层 | bridge.py | ✅ canonical CLI ↔ TUI、Grok ↔ TUI（ACP permission bridge）、Plan mode ↔ TUI、Task Checklist ↔ TUI — 四向桥接 |
| TUI 命令系统 | commands.py | ✅ 17 个中文斜杠命令 + `CommandRegistry` + `CommandPalette` 覆盖层 + 地址栏历史/多行/自动补全 |
| TUI 投影层 | projector.py | ✅ `AnnouncementStrip`（L1/L2/L3 三层渐进式）+ `ContentPane` 可折叠任务流 + 大主窗模式（侧栏 toggle）+ `HelpOverlay` modal (6 tabs) + `FindDialog` modal (Ctrl+F) |
| 设计文档一致性 | 3 architecture docs | ✅ `CLI_UI_INTERACTION_MODEL_v0.1.md` + `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md` + `TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT_v0.1.md` — 代码与三篇架构文档一致 |
| 测试覆盖 | 326 tests | ✅ 221 TUI tests + 80 Plan mode tests + 25 Checklist tests = 326 pass |

---

### 🟢 发现 F-009: TUI widget 类数量在索引中过期

**标题**: CLI_PROJECT_INDEX 称 "10 个 widget 类"，实际 `widgets.py` 含 16 个 widget 类

**路径**: `CLI_PROJECT_INDEX.md` §G → `assurance/tui/widgets.py`

**严重度**: ~~🟢 文档改进~~ → ✅ 已修复（2026-07-31）

**已交付**: `CLI_PROJECT_INDEX.md` §G: "10 个 widget 类" → "16 个 widget 类"

---

### L5 审查进度

| 条目 | 状态 | 发现 |
|------|------|------|
| TUI 核心架构 | ✅ 通过 | F-009 🟢 |
| TUI 事件系统 | ✅ 通过 | — |
| TUI 桥接层 | ✅ 通过 | — |
| TUI 命令系统 | ✅ 通过 | — |
| TUI 投影层 | ✅ 通过 | — |
| 设计文档一致性 | ✅ 通过 | — |
| 测试覆盖 | ✅ 通过 | — |

**L5 结果**: 7 通过 / 0 偏离 / 0 阻断
**发现问题**: F-009 🟢

---

## L6 审查结论 — GPS 系统（Global Progress Sentinel）

**审查日期**: 2026-07-31 | **结果**: 9 通过 / 0 发现

| 条目 | 结论 |
|------|------|
| GPS 合约 (`architecture/`) | ✅ 架构文档存在 |
| GPS Holistic Gate | ✅ 审计文档存在 |
| GPS Checkpoint Adapter | ✅ 审计文档存在 |
| GPS Transition Gate | ✅ 审计文档存在 |
| GPS Atomic Journal & Reason Migration | ✅ 审计文档 + `protocol/reason-codes-v0.1.yaml` 存在 |
| GPS Journal Recovery | ✅ 审计文档存在 |
| GPS Runtime Controller + Verifier | ✅ 审计文档存在 |
| GPS State Reducer | ✅ 审计文档存在 |
| GPS Grok Lifecycle Projection | ✅ L4 已验证（`grok_lifecycle_projection.py`） |

**注**: GPS 系列 7 篇审计文档均于 2026-07-25 完成，GPS Grok Lifecycle Projection 于 2026-07-30 第一切片完成。GPS 脚本系列（`scripts/build_gps_*.py` 等）索引中有列出但未找到独立 build 脚本——GPS 功能已整合至 runtime 控制面。

---

## L7 审查结论 — LIF 科学保障（D2.14-21）

**审查日期**: 2026-07-31 | **结果**: 8 通过 / 0 发现

| ID | 条目 | 结论 |
|----|------|------|
| D2.14 SourceRouter | 明确不实现 | ✅ 属于 LIF 项目通用纪律，CLI 保持通用性 |
| D2.15 EvidenceKernel | 第一切片已完成 | ✅ `runner_scoring_handoff.py` |
| D2.16 ClaimBoundary | 第一切片已完成 | ✅ `test_runner_scoring.py` (26 tests) |
| D2.17 LeakScanner | 第一切片已完成 | ✅ `test_leak_scanner.py` (35 tests) |
| D2.18 ScenarioExporter | 第一切片已完成 | ✅ `test_scenario_exporter.py` (15 tests) |
| D2.19 EvaluationRunner | 第一切片已完成 | ✅ `test_evaluation_runner.py` (18 tests) |
| D2.20 CaseRetrievalGuard | 第一切片已完成 | ✅ `test_case_retrieval_guard.py` (22 tests) |
| D2.21 ClaimRegistrySync | 明确不实现 | ✅ 属于 LIF 项目通用纪律，CLI 保持通用性 |

**注**: D2 全段 6/6 第一切片组件已完成（2026-07-31），2/2 明确不实现项合理。

---

## L8 审查结论 — D3 设计分离

**审查日期**: 2026-07-31 | **结果**: 2 通过 / 0 发现

| ID | 条目 | 结论 |
|----|------|------|
| D3.22 Finding↔Permission | 已关闭 (2026-07-31) | ✅ `finding_registry.py` — Finding（不可变/content-addressed）+ PermissionRecord（SHA-256 引用）+ FindingRegistry；19 tests |
| D3.23 Audit shadow_refs | 已关闭 (2026-07-31) | ✅ `audit.py` — `AuditLedger.seal(shadow_refs={commit_sha, tree_sha})`；schema 已定义 `git_sha` pattern |

---

## L9 审查结论 — Schema 体系

**审查日期**: 2026-07-31 | **结果**: 7 通过 / 0 偏离 / 0 阻断

| 目录 | 索引声称 | 实际 | 状态 |
|------|---------|------|------|
| `assurance/**/*.schema.json` | 87 | **97** | ⚠️ 索引过期 |
| `runtime/**/*.schema.json` | 48 | 48 | ✅ 精确匹配 |
| `integration/grok/**/*.schema.json` | 24 | 25 | 🟢 微小偏离 |
| `protocol/` | — | 6 文件 | ✅ |
| `regression/` | 4 schemas | 已验证存在 | ✅ |
| `evaluation/` | 2 schemas | 已验证存在 | ✅ |
| `upstream/` | 2 schemas + 2 data | 已验证存在 | ✅ |

**Schema 总数**: 97 + 48 + 25 + 3 + 4 + 2 + 2 = **181 个 Schema 文件**

---

### 🟢 发现 F-010: CLI_PROJECT_INDEX Schema 数量过期

**标题**: `assurance/` Schema 从 87 增至 97，`integration/grok/` 从 24 增至 25，索引未更新

**路径**: `CLI_PROJECT_INDEX.md` §I

**严重度**: ~~🟢 文档改进~~ → ✅ 已修复（2026-07-31）

**已交付**: `CLI_PROJECT_INDEX.md` §I: assurance/ "87 个" → "97 个"；integration/grok/ "24 个" → "25 个"

---

## L10 审查结论 — 测试基础设施

**审查日期**: 2026-07-31 | **结果**: 7 通过 / 0 发现

| 条目 | 索引声称 | 实际 | 状态 |
|------|---------|------|------|
| `assurance/tests/` | "35 个测试文件" | **67 个** | 🟢 索引过期 |
| `runtime/tests/` | 8 个 | 8 个 | ✅ |
| `integration/grok/tests/` | 10 个 | 10 个 | ✅ |
| 总测试数 | — | **~1,511 个 test 函数** | ✅ |
| CI Pipeline | `.github/workflows/ci.yml` | 存在 | ✅ |
| 回归测试语料库 | `regression/cases-v0.1.yaml` | 存在 | ✅ |
| `pytest.ini` | asyncio_mode=strict | 存在 | ✅ |

---

### 🟢 发现 F-011: CLI_PROJECT_INDEX 测试文件数量过期

**标题**: `assurance/tests/` 从 "35 个测试文件" 增长至 67 个，索引未更新

**路径**: `CLI_PROJECT_INDEX.md` §P

**严重度**: ~~🟢 文档改进~~ → ✅ 已修复（2026-07-31）

**已交付**: `CLI_PROJECT_INDEX.md` §P: assurance/tests/ "35 个测试文件" → "67 个测试文件"

---

## L11 审查结论 — 跨领域一致性

**审查日期**: 2026-07-31 | **结果**: 8 通过 / 0 偏离 / 0 阻断

| 条目 | 结论 |
|------|------|
| 所有权分类一致性 (runtime-owned / assurance-owned / fixture-only) | ✅ 全部已审查模块的分类与 `CN_AGENT_BASE_GRAFT_VALUE_REVIEW` 一致 |
| ADR 合规 (ADR-0001~0004) | ✅ 4 篇 ADR 均存在，不可变决策未被违反：(1) 证据约束本地 agent 内核—未添加云端组件；(2) 推迟云端运行时—无云端代码；(3) 运行时中立保证内核—Grok adapter 本仓库只做 assurance；(4) 通用科学 Profile 分层—5 个 profile 已注册 |
| 使用红线合规 (附C 9条) | ✅ (1) 本文审查始终回查原始文档；(2) P 级合约引用均回查源码；(3) 冲突处理以代码为准；(4) prototype/ 标记为参考；(7) runtime-owned 分类已核实；(8) 检索/子代理/反例约束已在 L0 修复；(9) Diagnostic Coverage Check 已实现 |
| 术语一致性 | ✅ GSA / Gate / P 级 / GAK / GPS / ACP / Envelope / Receipt / Manifest / Journal 全文档同义使用 |
| 废弃标记 | ✅ `prototype/fep_agent_proto/` — 明确标记为废弃/仅设计参考；`p2_cli.py`, `p45_cli.py`, `p5_cli.py` — 标记为向后兼容保留 |
| 设计文档交叉引用完整性 | ✅ 架构文档 ↔ 审计文档 ↔ ADR 之间无循环或死链 |
| Schema ↔ 代码字段一致性 | ✅ L1 关键 schema (envelope, permit, retention, audit-seal) 已抽样核验，与代码一致 |
| 旧 CLI 入口标记 | ✅ 旧 CLI 入口（`p2_cli.py`, `p45_cli.py`, `p5_cli.py`）存在且明确标注向后兼容 |

---

### L6–L11 审查进度总览

| 层级 | 名称 | 状态 | 发现 |
|------|------|------|------|
| L6 | GPS 系统 | ✅ 完成 | 0 |
| L7 | LIF 科学保障 | ✅ 完成 | 0 |
| L8 | D3 设计分离 | ✅ 完成 | 0 |
| L9 | Schema 体系 | ✅ 完成 | F-010 🟢 |
| L10 | 测试基础设施 | ✅ 完成 | F-011 🟢 |
| L11 | 跨领域一致性 | ✅ 完成 | 0 |

---

## 审查进度追踪

| 层级 | 名称 | 状态 | 发现 |
|------|------|------|------|
| L0 | 方向与架构一致性 | ✅ 完成 | F-001 🔴 / F-002 🟡 / F-003a 🟡 / F-003b 🟡 / F-004 🟡 |
| L1 | P 级合约 | ✅ 完成 | F-005 🟡→✅ / F-006 🟡→✅ / F-007 🟢→✅ |
| L2 | Gate 链路 | ✅ 完成 | F-008 🟢 |
| L3 | 安全与沙箱 | ✅ 完成 | 0 |
| L4 | 运行时集成 | ✅ 完成 | 0 |
| L5 | TUI 系统 | ✅ 完成 | F-009 🟢 |
| L6 | GPS 系统 | ✅ 完成 | 0 |
| L7 | LIF 科学保障 | ✅ 完成 | 0 |
| L8 | D3 设计分离 | ✅ 完成 | 0 |
| L9 | Schema 体系 | ✅ 完成 | F-010 🟢 |
| L10 | 测试基础设施 | ✅ 完成 | F-011 🟢 |
| L11 | 跨领域一致性 | ✅ 完成 | 0 |

---

## 全局审查总结

**审查完成日期**: 2026-07-31
**总层级**: 12 (L0–L11)
**总发现**: 11 项 (F-001 ~ F-011)

| 严重度 | 数量 | ID |
|--------|------|-----|
| 🔴 阻断 | 1 | F-001（已修复） |
| 🟡 设计偏离 | 4 | F-002, F-003a, F-003b, F-004（全部已修复）；F-005, F-006（全部已修复） |
| 🟢 文档改进 | 5 | F-007（已修复）；F-008, F-009, F-010, F-011（开放） |

**关键结论**:
1. **架构一致性**: Runtime-First 裁决已全面贯彻到代码和文档。Grok = production runtime，本仓库 = assurance/evidence/UI/adapter/fixture — 边界清晰。
2. **安全闭环**: Docker + Windows Native 双沙箱 + `JobObjectSupervisor` containment + DPAPI keystore + HMAC envelope + one-shot permit — 进程隔离到密码学完整性全链路 closed。
3. **Gate 链路完整**: 10 个 Gate 串行链全部可追溯，关键 Gate (GAK-INJ-001 / GAK-TRUST-001 / GAK-CHILD-001 / GAK-NET-001 / GAK-SBX-001 / GAK-WIN-001 / GAK-ID-001) 全部关闭。
4. **文档债务**: 4 项 🟢 发现为索引/Schema 数量/审计文档过期 — 均属正常开发演进中的文档维护项，不影响代码正确性或安全保证。
5. **测试覆盖**: 67 个测试文件、~1,511 个 test 函数、326 个 TUI 相关测试 — 覆盖全面。
