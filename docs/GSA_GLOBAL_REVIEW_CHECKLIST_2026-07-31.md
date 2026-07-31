# GSA 项目全局审查清单

**创建日期**: 2026-07-31
**最后更新**: 2026-07-31（第三轮审查：基于完整设计文档的设计→实现对照评估）
**状态**: 审查完成，剩余差距已登记
**范围**: 全部设计与实现——ADR、P 级合约、Gate 链路、安全沙箱、运行时集成、TUI、GPS、LIF 科学保障、Schema 体系、测试基础设施
**方法**: 逐层对照设计文档（ADR ×4、架构文档 ×12）→ 源代码 → Schema → 测试，按五层所有权模型评估完成度
**触发方式**: `gsa review global` 激活

---

## 项目设计摘要（审查基线）

### ADR（不可逆决策）

- **ADR-0001**: 证据约束型本地 Agent 内核。10 条核心不变量（模型输出≠状态事实、独立状态机、工作区优先、claim 强度受最弱证据约束、blind-first 案例检索等）。案例库作为一等子系统（40 案例、12 错误簇、8 偏误类型、9 捷径标签）。Gate 三档：discussion / guarded / strict。
- **ADR-0002**: 云端运行时显式推迟，保持 Windows 本地。
- **ADR-0003**: Runtime-Neutral。Assurance Kernel 不绑定特定 runtime，Grok 是 reference runtime（证据最完整）不是强制底座。profile 只能要求 capability 不能要求 runtime family。本仓库不自建第二套通用 model/tool/session runtime。
- **ADR-0004**: Profile 分层——`general-science`（通用保障）→ `lif-research`（领域增量），只增不减，通用测试不得使用 LIF 内部任务。

### 所有权模型（五层）

| 层 | 拥有 | 不拥有 |
|----|------|--------|
| **Mature Runtime（Grok）** | model loop、session、tool dispatch、permission UI、sandbox、compaction、ACP、hooks、MCP、subagents、workflows | 科学保障门禁 |
| **Runtime Adapter** | Grok binary 启动/监督、事件翻译（Grok→canonical）、capability discovery、containment | 变成隐藏替代 runtime |
| **Assurance Kernel** | task contract、workspace trust、source visibility、tool availability gate、instruction provenance、audit journal、evidence store、claim gates、evaluation isolation | 实现通用模型循环 |
| **UI（gsa TUI）** | Explorer 式空间化界面、中文优先、菜单/工具栏/地址栏/资源管理器/内容区/状态栏/对话框/属性面板 | 拥有模型执行、证据裁决、runtime 策略 |
| **Fixtures** | fake provider、loopback transport、disposable reproduction、development challenge sets | 成为 production runtime |

### TUI 设计方向

来自 `CLI_UI_INTERACTION_MODEL_v0.1.md` + `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`：

- Explorer / 旧 IE 风格终端 UI 作为 Agent 工作的空间化语言
- 默认中文 UI（菜单/按钮/状态栏/弹窗全中文化）
- 8 区域布局：MenuBar → Toolbar → AddressBar + FindBar → ExplorerPane + ContentPane + ContentMarker → StatusBar
- 大主窗模式（侧栏可全部隐藏）
- 聊天室式可折叠任务流（时间顺序、角色清楚、折叠策略：运行中展开、结束后折叠）
- Address 弹窗化（Ctrl+L）、Find 弹窗化（Ctrl+F）、Help modal overlay（6 tabs）
- Agent→模型、Edit→编辑模式、View 移除
- 架构边界：`用户输入 → UI 命令 → assurance core → 结构化事件 → view model → retro 渲染`
- `prompt_toolkit` 拥有终端机制，GSA retro widgets 拥有交互语义，Assurance core 拥有行为

---

## 逐层审查结果

### 第 0 层：方向与架构一致性

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| A0-1 Runtime-First 裁决 | Grok = production runtime owner；本仓库 = assurance/evidence/UI/adapter/fixture | `canonical_cli.py` 已标记 fixture-only；`INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md` 已冻结所有权 | ✅ 一致 |
| A0-2 canonical CLI 降级 | fixture-only，不充当 production runtime | 源码顶部 docstring 明确声明；`gsa run` 默认 canonical 路径保留为 assurance path | ✅ 一致 |
| A0-3 两个 Subagent 约束 | 项目文档检索 + 外部检索，不超过两个 | `retrieval_subagent.py` 恰好两个 dispatch 函数；`.grok/agents/` 恰好两个 profile | ✅ 一致 |
| A0-4 Retrieval mode 显式 | `local_browser` / `framework_fallback` / `off` 三选一 | `grok_runtime_adapter.py` `SUPPORTED_RETRIEVAL_MODES` + `validate_grok_retrieval_mode()` | ✅ 一致 |
| A0-5 Neutral vs Counterexample | 反例仅在 plan/conclusion 写入前；中立询问仅执行中 | `orientation_runtime_guard.py` + `docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` | ✅ 一致 |
| A0-6 ACP 优先 | `--grok-mode` 默认 `acp-smoke` | `cli.py` 默认值已切换；`prompt_smoke` 降级为窄 smoke 备选 | ✅ 一致 |
| A0-7 Diagnostic Coverage Check | 硬信号递进阈值 2→3→4→5 | 设计文档已记录，实现状态待确认 | ⚠️ 设计已有，代码实现待确认 |

**L0 判定：一致（6/7 ✅，1 待确认）**

---

### 第 1 层：P 级合约审计（P0–P5 + GSA-CORE）

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| P0 数据语义层 | Schema + envelope + contracts | `contracts.py` + `envelope.py` + 97 schemas | ✅ 一致 |
| P1 会话身份与归档 | ConversationNamespace + permit + archive journal | `conversation.py` + `permit.py` + `archive*.py` + `session_governor.py` | ✅ 一致 |
| P2 Docker 沙箱 | 进程隔离 + 状态可观测 | `sandbox.py` + `sandbox_verifier.py`，实测通过 | ✅ 一致 |
| P2 Windows 辅助沙箱 | AppContainer + Job Object + netsh | `windows_sandbox.py`，elevated compliant, non-elevated fail-closed | ✅ 一致 |
| P2.5 守卫执行 | 无模型动作 + 进程追踪 + 输出拦截 | `guarded_execution.py` | ✅ 一致 |
| P3 指令授权 | provenance + capability delegation + child enforcement | `instruction_gate.py` + `instruction_provenance_gate.py` + `child_capability_enforcer.py`（25 注入模式，89 tests） | ✅ 一致 |
| P4 审计/压缩/恢复 | audit ledger + archive verifier + recovery + compaction | `audit.py` + `archive_verifier.py` + `recovery.py` + `compaction_observer.py` + `shadow_recovery.py`（Git backend） | ✅ 一致 |
| P4.5 工作区优先 | workspace trust + integrated run + network permit | `integrated_run.py` + `workspace_trust.py` + `network_permit_gateway.py` | ✅ 一致 |
| P5 合成任务/预检 | synthetic task + mechanical preflight + readonly projection | `user_task_evaluation.py` + `runtime_preflight.py` + `readonly_projection.py` + `task_contract.py` + `ux_safety.py` | ✅ 一致 |
| GSA-CORE | 领域中立只读审查 | `artifact_registry.py` + `validator_bridge.py` + `general_science_review.py` | ✅ 一致 |

**L1 判定：一致（10/10 ✅）**

---

### 第 2 层：Gate 链路逐一审查

全部 GAK 条目已于 2026-07-31 前关闭。

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| GAK-INJ-001 指令来源 | 25 注入模式 + canonicalizer + obfuscation + 89 tests | `instruction_provenance_gate.py` | ✅ |
| Tool Availability | ACP dual verification (`allow_once` + `cancel_permission`) | `tool_availability_gate.py` + `grok_tool_permission_observer.py` | ✅ |
| Retrieval Subagents | 恰好两个 + parent_envelope + completion check | `retrieval_subagent.py` | ✅ |
| Orientation Runtime Guard | 停滞检测 + Diagnostic Coverage Check | `orientation_runtime_guard.py` | ✅ |
| GAK-TRUST-001 | 全部入口点 + AdapterGateContext | `workspace_trust.py` | ✅ |
| Source Visibility | 全文可见性检查 | `source_visibility.py` | ✅ |
| GAK-CHILD-001 | `spawn_child_context()` 统一入口 | `child_capability_enforcer.py` | ✅ |
| GAK-NET-001 | 全部 HTTP 调用前 `evaluate_network_permit()` | `network_permit_gateway.py` + `deepseek_adapter.py` | ✅ |
| Adapter Gate | preflight + output validation + failure classification | `adapter_gate.py` + `adapter_preflight.py` + `adapter_output_validator.py` + `adapter_failure_classifier.py` | ✅ |
| LBR-001 | CDP 客户端 + 检索工作流 + PDF evidence + 71 tests | `browser_retrieval.py` + `retrieval_workflow.py` + `pdf_evidence.py` + `evidence_store.py` | ✅ |
| **Gate ↔ ACP inline hook** | Gate 检查实时挂到 ACP 事件流上 | 当前 gate 主要从 canonical CLI 路径验证；Grok ACP 路径的 gate 检查主要是 post-hoc（session verifier），非 inline 实时拦截 | ⚠️ 设计未明确要求 inline，但 CLI_PROJECT_INDEX 中待嫁接列表暗示应做 |

**L2 判定：Gate 全部关闭（10/10 ✅），ACP inline hook 是增强项非阻塞项**

---

### 第 3 层：安全与沙箱

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| Docker Sandbox | 进程隔离 + 网络限制 + 状态可观测 | `sandbox.py` + `sandbox_verifier.py` | ✅ |
| Windows Native Sandbox | AppContainer + Job Object + netsh | `windows_sandbox.py`，GAK-SBX-001 closed | ✅ |
| Job Object Containment | `CREATE_SUSPENDED` + Kill-On-Close + 全部 Grok 路径 | `job_object_supervisor.py`，containment 等价裁定已出 | ✅ |
| GAK-WIN-001 | `PROC_THREAD_ATTRIBUTE_JOB_LIST` 内核级原子绑定 | `windows_sandbox.py:856-983`，11 tests pass | ✅ |
| Keystore | 安装级密钥存储（内存 + DPAPI） | `keystore.py` | ✅ |
| Key Lifecycle | rotation + revocation + crash-safe journal | `key_lifecycle.py`，GAK-ID-001 closed | ✅ |
| Envelope | 数据完整性与来源验证 | `envelope.py` | ✅ |
| Permit | 敏感操作许可生命周期 | `permit.py` | ✅ |
| Endpoint Canonicalizer | 路径遍历 + SSRF 防护 | `endpoint_canonicalizer.py` | ✅ |
| Storage Adapter | 统一文件系统访问 + ArchiveController | `storage_adapter.py` | ✅ |
| Subprocess Containment | 全部 Grok 调用路径 Job Object 包裹 | `grok_tool_permission_observer.py` + `grok_runtime_adapter.py` | ✅ |

**L3 判定：一致（11/11 ✅）**

---

### 第 4 层：运行时集成

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| Grok Runtime Adapter | ACP 完整生命周期 + multi-prompt + containment | `grok_runtime_adapter.py` `GrokAcpSession` class | ✅ |
| Grok Event Normalizer | Grok→canonical 事件正规化 | `grok_event_normalizer.py` | ✅ |
| Grok Prompt/Tool Gate | dual ACP + adapter containment → `decision: allow`；fail-closed 默认 | `grok_prompt_tool_gate.py` | ✅ |
| Grok Lifecycle Projection | metadata-only GPS journal events | `grok_lifecycle_projection.py` | ✅ |
| Grok Session Verifier | 7 项 post-run 交叉核验 + 14 tests | `grok_session_verifier.py` | ✅ |
| Grok Profile Drafts | 两个 agent profiles + 一个 workflow + Rhai 验证 | `grok_profile_drafts.py` | ✅ |
| Grok Tool Permission Observer | 静态 registry/permission 观察 | `grok_tool_permission_observer.py` | ✅ |
| DeepSeek Adapter | one-shot/stream 双模式 + 凭证硬化 | `deepseek_adapter.py` | ✅ |
| Codex/VS Code Lifecycle | 保留为观测基础，不升级为完整插件 | 按裁决保留 | ✅ |
| Upstream Lock | `0.2.112 (9bbd559437)` + candidate gate 全量通过 | `upstream/grok-build.lock.json` | ✅ |
| **ACP text_delta 事件** | 流式增量文本暴露 | 当前 adapter 只在最终响应时发送单个 `model_output`；ACP `assistant_message` 增量未作为 `text_delta` 事件暴露 | ❌ |
| **Grok 错误面完整映射** | error/warning notification → TUI 事件 | 部分映射（error 事件存在），未全覆盖 | ❌ |

**L4 判定：核心完成（10/12 ✅），差 text_delta 和错误面映射**

---

### 第 5 层：TUI 系统

对照 `CLI_UI_INTERACTION_MODEL_v0.1.md`（第一原型）和 `CLI_UI_SIMPLIFICATION_SUPPLEMENT_v0.1.md`（简化补充）。

#### 5.1 控件体系

| 控件 | 设计要求 | 实现状态 |
|------|---------|---------|
| MenuBar | 中文菜单（文件/编辑/视图/来源/模型/运行/验证/帮助） | ✅ |
| Toolbar | 中文按钮（后退/前进/刷新/停止/打开/验证/属性） | ✅ |
| AddressBar + AddressDialog | Ctrl+L 弹窗，支持历史/自动补全/多行 | ✅ |
| FindBar + FindDialog | Ctrl+F 弹窗，多行/高级搜索 | ✅ |
| ExplorerPane | 树形导航（Workspace/Task/Source/Run/Artifact/Adapter/Verifier） | ✅ |
| ContentPane | 当前任务/对话流 | ⚠️ 支持 event groups + message mode，但缺少聊天渲染 |
| ContentMarker | 右侧标记 | ✅ |
| StatusBar | 运行时状态 | ✅ |
| Dialog | 权限弹窗（已接 Grok interactive bridge） | ✅ |
| PropertiesSheet | 多 tab 属性面板 | ✅ |
| CommandPalette | 17 个中文斜杠命令 | ✅ |
| HelpOverlay | 6 tabs, Esc 关闭, ←/→ 导航 | ✅ |
| AnnouncementStrip | L1/L2/L3 checklist | ✅ |

#### 5.2 简化补充设计对照

| 设计要求 | 状态 |
|---------|------|
| 默认中文 UI（菜单/工具栏/状态栏/对话框全中文化） | ✅ |
| Help modal overlay（6 tabs, Esc/←/→） | ✅ |
| Address 弹窗化（Ctrl+L） | ✅ |
| Find 弹窗化（Ctrl+F） | ✅ |
| 大主窗模式（侧栏 toggle） | ✅ |
| Agent→模型, Edit→编辑模式 | ✅ |
| Sources 保留, View 移除 | ✅ |
| 顶部两层语义保留 | ✅ |
| 聊天室式可折叠任务流 | ❌ |
| 折叠策略（运行中展开、结束后折叠） | ❌ |
| 首条 prompt 空会话输入 | ❌ |

#### 5.3 事件系统（28 种事件类型 → projector dispatch）

| 类别 | 事件 | 状态 |
|------|------|------|
| 生命周期 | run_preflight, run_started, run_finished, run_failed, run_cancelled | ✅ |
| Gate | gate_decision, instruction_provenance_gate, source_visibility, tool_availability | ✅ |
| 模型 | model_request, model_output | ✅ |
| 工具 | tool_proposal, tool_started, tool_completed | ✅ |
| 权限 | permission_requested, permission_decision | ✅ |
| Plan | plan_phase_entered, plan_phase_submitted, plan_approval_decision | ✅ |
| Checklist | task_checklist, checklist_item_status, artifact_registered | ✅ |
| ACP | acp_initialize, acp_session_created | ✅ |
| 其他 | orientation_checkpoint, status_update, usage_sample, error | ✅ |
| **流式文本** | text_delta | ❌ |

#### 5.4 对话渲染

当前 `ContentPane` 同时支持 event groups 模式和 message 模式（`add_message()` 方法在 `app.py:496` 被 `_dispatch_command` 调用）。但 `projector.py` 的 28 个 handler 全部使用 event group 渲染路径——将 ACP 事件渲染为事件日志行，而非聊天气泡。

差距明细：

| 差距 | 说明 | 量级 |
|------|------|------|
| 聊天气泡渲染 | user/model/tool/result 四种气泡样式，时间顺序，角色标签 | ~500 行 |
| Markdown 渲染 | 标题/粗体/列表/表格/代码块（可用 `rich` 库） | ~300 行 |
| 代码语法高亮 | pygments 集成 | ~150 行 |
| 工具调用可折叠卡片 | tool_proposal→展开卡片，tool_completed→折叠，显示摘要 | ~250 行 |
| 折叠策略 | 运行中展开、结束后折叠（设计明确要求） | ~150 行 |
| 流式 token 追加 | text_delta 事件 → 追加到当前气泡底部 | ~200 行 |

**L5 判定：控件框架 100%、事件投影系统 95%、对话渲染 ~20%。综合 ~65%。瓶颈极度集中——所有 event 数据已到位、ACP 实时通道已到位、projector dispatch 已到位，差 ContentPane 的聊天气泡 rendering。**

---

### 第 6 层：GPS 系统（Global Progress Sentinel）

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| GPS 合约 | 架构定义 | `GLOBAL_PROGRESS_SENTINEL_CONTRACT_v0.1.md` | ✅ |
| GPS Holistic Gate | 跨会话 holistic 进度评估 | 设计文档 + schema 完整 | ✅ |
| GPS Checkpoint Adapter | 策略化检查点保存/恢复 | 设计文档 + 脚本完整 | ✅ |
| GPS Transition Gate | 状态迁移守卫逻辑 | 设计文档 + schema 完整 | ✅ |
| GPS Atomic Journal & Reason Migration | 原子日志 + reason code | `protocol/reason-codes-v0.1.yaml` | ✅ |
| GPS Journal Recovery | 日志恢复 | 恢复脚本完整 | ✅ |
| GPS Runtime Controller + Verifier | 运行时控制 + 验证 | 脚本完整 | ✅ |
| GPS State Reducer | 跨会话状态聚合 | 脚本完整 | ✅ |
| GPS Grok Lifecycle Projection | metadata-only 约束 | `grok_lifecycle_projection.py` | ✅ |
| GPS ↔ Checklist 映射 | 设计中有（`TASK_CHECKLIST_ANNOUNCEMENT_SUPPLEMENT` §3） | 实现状态待确认 | ⚠️ |

**L6 判定：基本一致（9/10），GPS↔Checklist 映射待确认**

---

### 第 7 层：LIF 科学保障（D2.15-20）

| 条目 | 设计要求 | 状态 |
|------|---------|------|
| D2.14 SourceRouter | 明确不实现（LIF 通用纪律，非本仓库科学性问题） | — |
| D2.15 EvidenceKernel | action/evidence/claim 三层独立状态机 | ✅ 第一切片 |
| D2.16 ClaimBoundary | 四种 claim 类型 | ✅ 第一切片 |
| D2.17 LeakScanner | 机械检查 + 独立语义审阅双通道 | ✅ 第一切片 |
| D2.18 ScenarioExporter | 匿名场景包 | ✅ 第一切片 |
| D2.19 EvaluationRunner | 冻结配置 + append-only journal + oracle 隔离 | ✅ 第一切片 |
| D2.20 CaseRetrievalGuard | blind-first | ✅ 第一切片 |
| D2.21 ClaimRegistrySync | 明确不实现 | — |

**L7 判定：一致（6/6 第一切片已完成，2 项明确不实现）**

---

### 第 8 层：D3 设计分离

| 条目 | 设计要求 | 状态 |
|------|---------|------|
| D3.22 Finding ↔ Permission | Finding（不可变/content-addressed）+ PermissionRecord（独立决策/SHA-256 引用）+ FindingRegistry（session-scoped） | ✅ |
| D3.23 Audit shadow_refs | `AuditLedger.seal()` 可选 `shadow_refs` 参数（commit_sha + tree_sha，40-char Git SHA） | ✅ |

**L8 判定：一致（2/2 ✅）**

---

### 第 9 层：Schema 体系

| 目录 | 数量 | 状态 |
|------|------|------|
| `assurance/` | 97 个 | ✅ |
| `runtime/` | 48 个 | ✅ |
| `integration/grok/` | 25 个 | ✅ |
| `protocol/` | 4 个（agent-protocol, reason-codes, gate-matrix, migration） | ✅ |
| `regression/` | 4 个（case-corpus, coverage-matrix, fixture, provenance） | ✅ |
| `evaluation/` | 2 个（evaluation-result, partition-manifest） | ✅ |
| `upstream/` | 3 个（grok-build-lock, candidate, lock.json） | ✅ |

**L9 判定：一致（183 个 schema）**

---

### 第 10 层：测试基础设施

| 目录 | 文件数 | tests | 状态 |
|------|--------|-------|------|
| `assurance/tests/` | 68 | ~1400 | ✅ |
| `runtime/tests/` | 8 | GPS + CLI lifecycle + Codex | ✅ |
| `integration/grok/tests/` | 10 | Grok probes | ✅ |
| 回归语料库 | — | 30 historical + 10 synthetic + 5/5 fixtures | ✅ |
| CI Pipeline | GitHub Actions | Ubuntu + Windows, Python 3.11/3.12 | ✅ |

**L10 判定：一致，~1513 tests pass**

---

### 第 11 层：CLI 入口 + 会话管理

| 条目 | 设计要求 | 实现状态 | 判定 |
|------|---------|---------|------|
| `gsa doctor` | 机械诊断 | ✅ | — |
| `gsa source gate` | 来源可见性检查 | ✅ | — |
| `gsa run`（canonical + real + grok） | 三种 runtime 路径 | ✅ | — |
| `gsa verify` | 独立验证 | ✅ | — |
| `gsa tui`（static + demo + events + replay + run + grok） | 完整 TUI 入口 | ✅ | — |
| `gsa grok doctor/run/observe-tools` | Grok 管理命令 | ✅ | — |
| `gsa review global` | 全局审查模式 | ✅ | — |
| **`gsa`（无参数）** | 默认启动交互式对话 | ❌ | 无默认命令 |
| **`gsa session list`** | 历史会话列表 | ❌ | 无 session index |
| **`gsa session resume <id>`** | 恢复历史会话 | ❌ | — |
| **`gsa init`** | 首次安装引导 | ❌ | — |
| **Session index** | 跨会话元数据索引 | ❌ | — |

**L11 判定：功能命令齐全（7/7 ✅），差默认入口 + 会话管理（4 项缺失）**

---

### 第 12 层：跨领域一致性

| 条目 | 状态 |
|------|------|
| 所有权分类一致性（runtime-owned / assurance-owned / fixture-only） | ✅ |
| ADR 合规（ADR-0001~0004 未违反） | ✅ |
| 使用红线合规（CLI_PROJECT_INDEX.md 附 C 的 9 条） | ✅ |
| 术语一致性（GSA / Gate / P 级 / GAK / GPS / ACP / Envelope / Receipt / Manifest / Journal） | ✅ |
| 废弃标记（`prototype/`、旧 CLI 入口 `p2_cli.py`/`p45_cli.py`/`p5_cli.py`） | ✅ |
| 设计文档交叉引用完整性（架构↔审计↔ADR 无循环/死链） | ✅ |

**L12 判定：一致（6/6 ✅）**

---

## 综合完成度

| 层 | 完成度 | 关键剩余 |
|----|--------|---------|
| L0 方向一致性 | 95% | Diagnostic Coverage Check 代码确认 |
| L1 P 级合约 | **100%** | — |
| L2 Gate 链路 | 95% | ACP inline hook（增强项） |
| L3 安全沙箱 | **100%** | — |
| L4 Runtime 集成 | **88%** | text_delta 事件、错误面映射 |
| **L5 TUI** | **65%** | **对话渲染、流式显示、Markdown、代码高亮、折叠策略** |
| L6 GPS | 95% | GPS↔Checklist 映射确认 |
| L7 LIF 科学保障 | **100%** | — |
| L8 D3 设计分离 | **100%** | — |
| L9 Schema 体系 | **100%** | — |
| L10 测试基础设施 | **100%** | — |
| L11 CLI + Session | **60%** | 默认命令、session 子命令 |
| L12 跨领域一致性 | **100%** | — |

**综合加权完成度：~73%**

---

## 剩余工作登记

### 阻断级（无此则不是可用的对话程序）

| # | 差距 | 层 | 估量 | 依赖 |
|---|------|----|------|------|
| GAP-01 | **ContentPane 对话渲染**（设计已冻结 `architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` — 用户/模型极简边框 + 工具调用 `[工具名]` 无边框单行滚动/缩进清单展开 + 流式 text_delta + Markdown/语法高亮） | TUI | ~500 行 | GAP-02 |
| GAP-02 | **流式 text_delta**（ACP assistant_message delta → `text_delta` 事件 → TUI 逐 token 追加） | Adapter + TUI | ~200 行 | — |
| GAP-03 | **`gsa` 默认命令**（设计已冻结 `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` §1 — Address/Find 折叠至 Toolbar，`命令...`/`查找...` 按钮激活弹窗） | CLI + TUI | ~150 行 | — |

### 高优先（严重损害可用性）

| # | 差距 | 层 | 估量 | 依赖 |
|---|------|----|------|------|
| GAP-04 | **Markdown 渲染**（标题/粗体/列表/表格/代码块） | TUI | ~300 行 | GAP-01 |
| GAP-05 | **代码语法高亮**（pygments 集成） | TUI | ~150 行 | GAP-01 |
| GAP-06 | **工具调用可折叠卡片**（设计 §11 明确要求） | TUI | ~250 行 | GAP-01 |
| GAP-07 | **折叠策略**（运行中展开、结束后折叠） | TUI | ~150 行 | GAP-01 |
| GAP-08 | **会话持久化**（设计已冻结 `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` §2 — `.gsa/sessions/` 目录 + `index.jsonl` 追加索引 + 双击 Esc 切换 ExplorerPane 会话列表模式 + `gsa session list/resume/archive/delete`） | Session + CLI | ~350 行 | — |
| GAP-09 | **启动 UX**（与 GAP-03 合并，已冻结） | CLI + TUI | ~200 行 | GAP-03 |

### 增强项（有则更好）

| # | 差距 | 层 | 估量 | 依赖 |
|---|------|----|------|------|
| GAP-10 | Gate inline hook 到 ACP 事件流 | Assurance | ~250 行 | — |
| GAP-11 | Grok 错误面完整映射 | Adapter + TUI | ~150 行 | — |
| GAP-12 | ACP 通知中 tool_call_update / tool_result 实时投影优化 | TUI | ~200 行 | GAP-01 |

**剩余总量：约 2,500-2,800 行，其中 ~1,700 行（~65%）集中在 TUI 渲染层（GAP-01/02/04/05/06/07）。**

---

## 审查规则

1. 每层审查结论来自设计文档 ↔ 源代码 ↔ Schema ↔ 测试的对照检查。
2. 问题按严重度：🔴 阻断（架构违规、功能缺失致不可用）、🟡 设计偏离（文档不一致、未贯彻裁决）、🟢 增强（可改进项）。
3. 引用任何 P 级合约、Gate 行为或 Schema 字段前，必须回查原始审计文档、架构文档和源代码——本清单只作路由指引。
4. 出现新文档或代码修改与本清单冲突时，以后续文档和代码为准，并更新本清单对应条目。
5. 所有权分类（runtime-owned / assurance-owned / fixture-only / adapter-owned / ui-owned）在每次修改时重新核对。
