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

## 审查进度追踪

| 层级 | 名称 | 状态 | 发现 |
|------|------|------|------|
| L0 | 方向与架构一致性 | ✅ 完成 | F-001 🔴 / F-002 🟡 / F-003a 🟡 / F-003b 🟡 / F-004 🟡 |
| L1 | P 级合约 | 待审查 | - |
| L2 | Gate 链路 | 待审查 | - |
| L3 | 安全与沙箱 | 待审查 | - |
| L4 | 运行时集成 | 待审查 | - |
| L5 | TUI 系统 | 待审查 | - |
| L6 | GPS 系统 | 待审查 | - |
| L7 | LIF 科学保障 | 待审查 | - |
| L8 | D3 设计分离 | 待审查 | - |
| L9 | Schema 体系 | 待审查 | - |
| L10 | 测试基础设施 | 待审查 | - |
| L11 | 跨领域一致性 | 待审查 | - |
