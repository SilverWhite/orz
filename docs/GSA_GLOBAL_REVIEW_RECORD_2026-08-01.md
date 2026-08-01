# GSA 全局审查记录

**日期**: 2026-08-01  
**状态**: 第一版审查记录 / 后续审查路线图  
**触发请求**: 先查看 `CLI_PROJECT_INDEX.md` 路由，随后回查所需文档，对项目进行全局审查；若项目内容过多，一次性难以完成，则先制定审查清单。  
**范围**: 项目设计与实现的全局审查入口，包括架构文档、审计文档、P 级合约、Gate 链路、Grok runtime adapter、TUI、session/layout、Schema、测试与上游锁文件。  
**结论性质**: 本文不是最终全局审查结论；它记录本轮已完成的路由回查、即时发现、历史清单重分类点和后续可执行审查清单。

---

## 1. 本轮回查方法

本轮按 `CLI_PROJECT_INDEX.md` 明示的回查顺序执行：

1. 先读 `CLI_PROJECT_INDEX.md`，确认其定位为项目主召回索引，而不是完整证明文档。
2. 回查全局审查相关历史文档：
   - `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md`
   - `docs/GSA_GLOBAL_REVIEW_FINDINGS_2026-07-31.md`
   - `README.md`
3. 回查关键架构设计：
   - `architecture/INTEGRATED_AGENT_ASSURANCE_DESIGN_v0.1.md`
   - `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
   - `architecture/CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md`
   - `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md`
4. 盘点当前实现入口：
   - `assurance/cli.py`
   - `assurance/global_review_mode.py`
   - `assurance/grok_runtime_adapter.py`
   - `assurance/tui/widgets.py`
   - `assurance/tui/projector.py`
   - `assurance/tui/app.py`
   - `assurance/tui/bridge.py`
5. 运行窄测试与机械检查，确认审查清单中的高风险点是否仍成立。

---

## 2. 项目规模判断

排除 `.git`、`.observed-runs`、`candidate-gates`、临时目录、pytest 缓存等生成/观测产物后，本轮粗略盘点到：

| 类别 | 数量 |
|------|------|
| 文件 | 5131 |
| Python 文件 | 322 |
| Markdown 文件 | 188 |
| JSON Schema | 182 |
| JSON 文件 | 624 |
| YAML 文件 | 18 |
| 测试文件 | 87 |

因此，本轮不宜直接声称完成“全部设计与实现”的最终全局审查。合理处理方式是：先建立可复用审查清单，并记录已发现的高优先不一致项。

---

## 3. 即时发现

### F-2026-08-01-001: Grok lock 与索引/测试/doctor 不一致

**严重度**: 高  
**状态**: 已处理（2026-08-01 回退至 `0.2.112`）  
**涉及文件**:

- `upstream/grok-build.lock.json`
- `upstream/grok-build.candidate.json`
- `CLI_PROJECT_INDEX.md`
- `README.md`
- `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md`
- `assurance/tests/test_grok_runtime_adapter.py`

**现象**:

初审时 `upstream/grok-build.lock.json` 记录的 binary release 曾变为：

```text
version: 0.2.118
build_id: 1e1687c1cf
bytes: 0
md5: pending-re-verification
sha256: pending-re-verification
authenticode_status: pending-recheck
installed_path: C:\Users\1\.grok\bin\grok.exe
```

但 schema 要求：

- `bytes >= 1`
- `md5` 为 32 位 hex
- `sha256` 为 64 位 hex
- `authenticode_status == "Valid"`
- `installed_path` 匹配 `.tools/grok/<version>/grok.exe`

同时，项目索引、README 和 7 月文档仍多处声明 `0.2.112 (9bbd559437)` 是已验证默认 lock。

**已观测影响**:

- `python gsa.py doctor --json` 返回 invalid，仓库级机械检查失败。
- `assurance/tests/test_cli_dispatcher.py::GsaCliDispatcherTests::test_doctor_full_repository_check` 失败。
- `assurance/tests/test_grok_runtime_adapter.py` 中两个 version-smoke 测试失败；测试 fake process 输出仍为 `0.2.112 (9bbd559437)`，实现则按当前 lock 严格匹配。

**后续审查动作**:

1. 确认应回退到已验证 `0.2.112`，还是正式推进 `0.2.118`。
2. 若推进 `0.2.118`，必须补齐 binary identity、hash、Authenticode、安装路径、candidate gate 和文档更新。
3. 若回退 `0.2.112`，应恢复 lock 与 `upstream/grok-build.candidate.json` 的已验证字段一致，并更新任何中途写入的 `0.2.118` 记录。
4. 修正 `test_grok_runtime_adapter.py` 的测试夹具，使其跟目标 lock 一致。

**处理结果（2026-08-01）**:

- 已裁决暂时回退到 `0.2.112 (9bbd559437)`，不推进未闭合的 `0.2.118`。
- `upstream/grok-build.lock.json` 已恢复为 candidate 中已验证的 `0.2.112` 字段。
- `doctor --json` 已恢复 `valid: true`，repository errors 为 0。
- Grok adapter version-smoke 相关测试已恢复通过。

---

### F-2026-08-01-002: 7 月 31 日全局审查清单对 TUI / text_delta 的状态已过期

**严重度**: 中  
**状态**: 需重分类，不宜继续按缺口处理  
**涉及文件**:

- `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md`
- `assurance/grok_runtime_adapter.py`
- `assurance/tui/widgets.py`
- `assurance/tui/projector.py`
- `assurance/tui/bridge.py`

**现象**:

旧清单仍将以下条目标为缺失：

- ACP `text_delta`
- ContentPane 对话渲染
- Markdown 渲染
- 代码块/语法高亮
- 工具调用折叠卡片
- 折叠策略

但当前代码已有对应实现入口：

- `grok_runtime_adapter.py`: ACP `assistant_message` / `message` / `user_message` 增量转换为 `text_delta`。
- `projector.py`: `TEXT_DELTA` handler 会追加到当前模型消息卡片，并避免最终 `model_output` 重复生成卡片。
- `widgets.py`: `ContentPane` 已包含 `ChatMessage`、`ToolTraceLine`、Markdown 基础格式化、代码块渲染、工具行折叠/展开、`collapse_non_warnings()`。
- `bridge.py`: 已包含 `text_delta` event factory 和 event kind mapping。

**后续审查动作**:

1. 将旧清单中的 TUI 渲染缺口改为“已实现，待代码质量与测试覆盖审查”。
2. 增补直接测试：
   - `TextDeltaEvent` 首 chunk 创建模型卡片。
   - 后续 chunk 追加同一模型卡片。
   - 最终 `model_output` 不重复卡片。
   - 工具同名调用追加同一 `ToolTraceLine`。
   - `run_finished` 后非 warning 工具折叠。
   - Markdown 标题、列表、代码块、Python highlighting 宽度不破坏渲染。
3. 确认实现是否真的满足 `CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` 的反规则，例如工具展开不展示文件内容、自然语言消息才有边框。

---

### F-2026-08-01-003: Session 审查口径已从自建 store 改为 Grok-owned

**严重度**: 中  
**状态**: 需更新旧清单  
**涉及文件**:

- `architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md`
- `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md`
- `assurance/tui/app.py`
- `assurance/tui/widgets.py`
- `assurance/tui/bridge.py`

**现象**:

7 月 31 日全局审查清单仍把以下内容作为缺口：

- `gsa session list`
- `gsa session resume <id>`
- 自建 session index
- `.gsa/sessions/index.jsonl`

但 2026-08-01 的 session/layout 设计已明确修正所有权：

- session 持久化与恢复由 Grok 拥有。
- 本仓库不维护独立 session store。
- TUI 只提供只读会话列表视图，扫描 Grok run root 或 bridge 写出的薄 `session.json` marker。
- 恢复操作使用 `grok session resume <session_id>`。

当前实现中已存在：

- `app.py`: 双 Esc session list 入口。
- `widgets.py`: `toggle_session_mode()` / `load_sessions()`。
- `bridge.py`: 写出 `session.json` marker，供 TUI 只读发现。

**后续审查动作**:

1. 不再要求实现 `gsa session list/resume/archive/delete` 作为本仓库自有 CLI。
2. 审查重点改为：
   - TUI 是否只读扫描 session marker。
   - 是否避免自建恢复逻辑。
   - UI 文案是否明确完整恢复应使用 Grok 原生命令。
   - 旧的 `session_store` 痕迹是否仅存在于缓存或已废弃文件中，不能重新成为设计目标。

---

## 4. 已执行命令与结果

### 4.1 Global Review Mode

命令：

```powershell
python gsa.py review global
```

结果摘要：

```text
global review mode: active
scope: empty_scope (0 paths)
decision: ready_for_global_review_no_paths
```

判定：`global_review_mode` 激活路径正常。该 receipt 只激活并界定全局审查义务，不是最终审查结论。

### 4.2 Repository Doctor（初审时）

命令：

```powershell
python gsa.py doctor --json
```

结果摘要：

```text
valid: false
repository_check.error_count: 7
```

主要错误集中在 `upstream/grok-build.lock.json` 的 binary release 字段不满足 schema。

### 4.3 窄测试（初审时）

命令：

```powershell
python -m pytest assurance/tests/test_global_review_mode.py assurance/tests/test_grok_tui_wiring.py -q
```

结果：

```text
4 passed
```

命令：

```powershell
python -m pytest assurance/tests/test_cli_dispatcher.py -x -vv
```

结果：

```text
failed: test_doctor_full_repository_check
```

命令：

```powershell
python -m pytest assurance/tests/test_grok_runtime_adapter.py -vv
```

结果：

```text
3 passed, 2 failed
```

失败项：

- `test_version_smoke_can_create_isolated_workspace_under_run_root`
- `test_version_smoke_writes_valid_receipt_and_events`

### 4.4 较大测试集

命令：

```powershell
python -m pytest assurance/tests/test_global_review_mode.py assurance/tests/test_cli_dispatcher.py assurance/tests/test_grok_runtime_adapter.py assurance/tests/test_grok_tui_wiring.py assurance/tests/test_tui.py
```

结果：

```text
timeout after 120s
```

超时前已暴露：

- `test_cli_dispatcher.py`: 1 failed
- `test_grok_runtime_adapter.py`: 2 failed
- `test_global_review_mode.py`: passed
- `test_grok_tui_wiring.py`: passed
- `test_tui.py`: 前半段大量通过，但未完整跑完

注意：测试过程中出现 `.pytest_cache` 写入 permission warning，后续复跑可考虑加 `-p no:cacheprovider`，或单独记录为环境限制。

---

## 5. 后续全局审查清单

### L0. 路由与基线

- [ ] 更新 `CLI_PROJECT_INDEX.md` 中 Grok lock、TUI GAP、session 所有权状态。
- [ ] 将 2026-07-31 全局审查清单标为历史基线，不再直接作为当前状态。
- [ ] 明确全局审查模式与普通 code review 的触发边界。

### L1. Upstream / Grok Lock

- [x] 决定 `0.2.112` 回退还是 `0.2.118` 正式推进。已裁决回退 `0.2.112`。
- [x] 补齐或恢复 `upstream/grok-build.lock.json` 的 schema 合规字段。已恢复已验证 `0.2.112` 字段。
- [x] 同步 README、索引、CN graft review、候选门文档。复核后这些文档已与 `0.2.112` lock 一致，无需改动。
- [x] 修复 `doctor --json` 和 `test_grok_runtime_adapter.py`。回退后验证通过。

### L2. Runtime Adapter

- [x] 审查 `version_smoke` 对 lock 的严格匹配策略。判定合理，L1 回退后测试通过。
- [x] 审查并处理 `acp_smoke` 中 `assistant_message` 增量切片是否适配 Grok 当前 ACP shape。已兼容 cumulative 与 delta chunk 两种输入，见 L2-002 / L2-003。
- [x] 审查并处理 `tool_call_update`、`tool_result`、warning/error notification 的事件映射完整性。已修复 `input_summary` 丢失与 warning/error severity 合并问题，见 L2-001 / L2-004。
- [x] 确认 Job Object containment 在全部 Grok 启动路径上仍覆盖。相关窄测试通过。

### L3. TUI ContentPane

- [ ] 对照 `CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` 审查视觉规则。
- [ ] 补 `text_delta` 到 ContentPane 的直接单测。
- [ ] 补 Markdown / code block / width wrapping 测试。
- [ ] 补工具行折叠、展开、pin-to-top、run finish collapse 测试。
- [ ] 审查 ANSI 颜色与 display width 的相互影响。

### L4. Session / Layout

- [ ] 按 2026-08-01 所有权修正审查 session 列表。
- [ ] 确认 `session.json` marker 是薄发现层，不构成独立 session store。
- [ ] 确认 UI 不实现自有恢复，只提示或委托 Grok。
- [ ] 更新旧清单中 `gsa session list/resume` 的错误缺口描述。

### L5. P0-P5 / Gate / Security 抽样复核

- [ ] 抽样验证 P1/P4 archive 文档与代码接口仍一致。
- [ ] 抽样验证 retrieval subagent registry 与 completion check 仍满足“两子代理”约束。
- [ ] 抽样验证 network permit、child capability、workspace trust 仍覆盖当前入口。
- [ ] 判断 ACP inline gate hook 是否仍只是增强项，还是已成为生产路径阻断项。

### L6. Schema 与 Repository Check

- [ ] 修复 `scripts/check_repository.py` 暴露的 schema/lock 错误。
- [ ] 跑 `python gsa.py doctor --json`，要求 `valid: true`。
- [ ] 跑 schema validation 相关测试，确认没有因 lock 漂移产生连带失败。

### L7. 测试策略

- [ ] 先跑窄集：global review、CLI dispatcher、Grok adapter、TUI wiring、TUI ContentPane。
- [ ] 窄集修绿后跑 `assurance/tests` 全量。
- [ ] 最后跑 runtime / integration / scripts 相关测试。
- [ ] 对需要真实 Grok、网络、管理员权限或 Windows 特性的测试单独记录前置条件，不和普通离线测试混为一类。

### L8. 文档修订

- [ ] 更新 `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md` 或新增 2026-08-01 版 checklist。
- [ ] 将本文发现项回填索引或附录，避免后续仍沿用旧 GAP。
- [ ] 明确本文是审查记录，不替代正式审查结论。

---

## 6. L1 审查结果: Upstream / Grok Lock

**审查日期**: 2026-08-01  
**判定**: 初审未通过；回退处理后通过  
**阻断面（初审时）**: repository doctor、Grok version-smoke adapter receipt、项目版本叙述一致性  
**当前结论**: 已按用户裁决暂时回退到 `0.2.112 (9bbd559437)`。`upstream/grok-build.lock.json`、selected candidate、索引/README/CN 文档和 adapter version-smoke 测试已重新对齐。`0.2.118` 不作为当前默认 lock，后续如需升级应重新走 candidate gate。

### L1-001: 默认 lock 与 selected candidate 不一致

**严重度**: 高  
**状态**: 已处理（回退 `0.2.112`）  
**证据**:

- `upstream/grok-build.candidate.json` 的 selected candidate 是 `0.2.112 (9bbd559437)`，且 `selected_as_default: true`。
- 初审时 `upstream/grok-build.lock.json` 记录为 `0.2.118 (1e1687c1cf)`。
- `scripts/check_repository.py` 在 candidate selected 时要求 candidate release 的 `version` 和 `sha256` 与 default lock 一致。

**复核命令**:

```powershell
python gsa.py doctor --json
```

**复核结果**:

```text
valid: false
repository_check.error_count: 7
selected upstream candidate version does not match default lock
selected upstream candidate SHA-256 does not match default lock
```

**影响**:

默认 runtime lock 的可复现性被破坏。当前项目无法同时声称“selected candidate 是 0.2.112”与“默认 lock 是 0.2.118”。

**建议处理路径**:

1. 若 `0.2.112` 仍是默认版本：恢复 `upstream/grok-build.lock.json` 到 `0.2.112` 的已验证 binary release 字段。
2. 若 `0.2.118` 是新默认版本：新增/更新 candidate 文件，完成 `0.2.118` 的 identity、hash、Authenticode、promotion gate 与文档路由。

**处理结果**: 选择路径 1；lock 已恢复到 `0.2.112`，并通过 repository doctor 复核。

### L1-002: `grok-build.lock.json` binary_release 不满足 schema

**严重度**: 高  
**状态**: 已处理（schema 恢复合规）  
**证据**:

当前 lock 中的 `binary_release` 字段：

```text
version: 0.2.118
build_id: 1e1687c1cf
bytes: 0
md5: pending-re-verification
sha256: pending-re-verification
authenticode_status: pending-recheck
installed_path: C:\Users\1\.grok\bin\grok.exe
```

而 `upstream/grok-build-lock-v0.1.schema.json` 要求：

- `bytes` 最小值为 `1`
- `md5` 匹配 `^[0-9a-f]{32}$`
- `sha256` 匹配 `^[0-9a-f]{64}$`
- `authenticode_status` 必须为 `Valid`
- `installed_path` 必须匹配 `^\.tools/grok/[^/]+/grok\.exe$`

**复核结果**:

`doctor --json` 明确报告 5 个 schema 字段错误，全部来自 `upstream/grok-build.lock.json#/binary_release`。

**影响**:

`status: verified-local` 与实际证明字段矛盾。该 lock 不能作为 promoted/default lock 的审计证据。

**建议处理路径**:

不要把 pending 字段保留在 `verified-local` 状态中。要么降级状态并同步 schema/doctor 逻辑，要么补完验证字段后保持 `verified-local`。

**处理结果**: 已恢复 `bytes/md5/sha256/authenticode_status/installed_path` 为 `0.2.112` 已验证字段；`doctor --json` 不再报告 lock schema 错误。

### L1-003: Grok adapter version-smoke 测试与当前 lock 不一致

**严重度**: 中高  
**状态**: 已处理（测试恢复通过）  
**证据**:

`assurance/tests/test_grok_runtime_adapter.py` 的 fake process 输出仍是：

```text
grok 0.2.112 (9bbd559437) [stable]
```

而 `run_grok_headless_once()` 会从当前 lock 构造 expected version prefix，并检查 stdout 是否匹配 lock 中的 `version` / `build_id`。

**复核命令**:

```powershell
python -m pytest -p no:cacheprovider assurance/tests/test_grok_runtime_adapter.py -vv
```

**复核结果**:

```text
3 passed, 2 failed
failed:
- test_version_smoke_can_create_isolated_workspace_under_run_root
- test_version_smoke_writes_valid_receipt_and_events
```

**影响**:

adapter 本身的严格匹配策略是合理的；当前失败更像是 lock 漂移导致测试夹具与默认 lock 断裂。若不修复，这会持续掩盖 adapter 行为本身是否正确。

**建议处理路径**:

先决定默认 lock 版本，再同步测试 fake output 和 `_inspection()` fixture。不要为了让测试通过而放宽 version-smoke 的 lock 匹配。

**处理结果**: 已决定默认 lock 为 `0.2.112`；测试夹具本来即为 `0.2.112`，回退 lock 后无需修改测试，`test_grok_runtime_adapter.py` 全部通过。

### L1-004: 项目叙述仍大量指向 `0.2.112`

**严重度**: 中  
**状态**: 已处理（叙述与当前 lock 一致）  
**证据**:

以下文档仍叙述 `0.2.112 (9bbd559437)` 为已验证默认 lock：

- `CLI_PROJECT_INDEX.md`
- `README.md`
- `docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md`
- `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md`

**影响**:

初审时，读者会沿索引路由得到 `0.2.112` 的结论，但机械 lock 文件给出 `0.2.118`。这破坏了“先索引，后回查”的项目维护规则；回退后该冲突已消除。

**建议处理路径**:

在 lock 决策完成后统一更新文档。若 `0.2.118` 尚未完成 gate，不应提前替换叙述；若 `0.2.118` 已完成 gate，应补齐相应 candidate/gate 文档而不是只改 lock。

**处理结果**: 已裁决不推进 `0.2.118`；`CLI_PROJECT_INDEX.md`、`README.md`、`docs/CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` 与 `docs/GSA_GLOBAL_REVIEW_CHECKLIST_2026-07-31.md` 中的 `0.2.112` 叙述现在与 lock 一致。

### L1 测试记录

| 命令 | 结果 |
|------|------|
| `python gsa.py doctor --json` | 初审 failed；回退后 passed, `valid: true`, repository errors = 0 |
| `python -m pytest -p no:cacheprovider assurance/tests/test_cli_dispatcher.py::GsaCliDispatcherTests::test_doctor_full_repository_check -vv` | 初审 failed；回退后 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_runtime_adapter.py -vv` | 初审 3 passed / 2 failed；回退后 5 passed |

### L1 下一步

1. 版本裁决已完成：暂时回退 `0.2.112`。
2. lock / candidate / tests / docs 四者一致性已恢复。
3. `doctor --json`、CLI dispatcher doctor 测试和 Grok adapter 测试已重跑通过。

### L1 回退裁决（2026-08-01）

**裁决**: 暂时回退到 `0.2.112 (9bbd559437)`。  
**理由**: 当前没有明确升级需求；`0.2.118` 尚未完成本仓库要求的 binary identity、hash、Authenticode、candidate gate 和文档闭环。  
**执行范围**: 将 `upstream/grok-build.lock.json` 的 `binary_release` 回退为 `upstream/grok-build.candidate.json` 中已验证并 selected-as-default 的 `0.2.112` 字段。  
**非目标**: 不推进 `0.2.118`，不修改 Grok runtime adapter 的严格 version-smoke 匹配策略。

---

## 7. L2 审查结果: Runtime Adapter

**审查日期**: 2026-08-01  
**判定**: 已处理并通过窄集复核  
**通过面**: version-smoke 严格 lock 匹配、Job Object containment、prompt-tool promotion gate、Grok event/session/tool observer 相关既有测试。  
**已处理面**: ACP streaming / tool event / warning-event 映射缺口已补齐实现与窄测试。  
**核心结论**: Runtime adapter 的 containment 与 lock 校验主链路已恢复；ACP 事件映射层已补齐用户消息、assistant delta/cumulative、tool proposal 摘要与 warning severity 的直接处理。

### L2-001: Tool proposal 的 `input_summary` 在 TUI bridge 中丢失

**严重度**: 中  
**状态**: 已处理  
**涉及文件**:

- `assurance/grok_runtime_adapter.py`
- `assurance/tui/bridge.py`
- `assurance/tui/events.py`
- `assurance/tui/projector.py`

**证据**:

`grok_runtime_adapter.py` 在 `tool_call` / `tool_call_update` 中向 event payload 写入 `input_summary`。但 `bridge.py` 的 `_make_tool_proposal()` 只构造 `tool_name` 与 `tool_call_id`，没有把 `input_summary` 传入 `ToolProposalEvent`。

只读探针结果：

```text
ToolProposalEvent read_file id1 
```

第三列后为空，说明 `input_summary` 已丢失。

**影响**:

`projector.py` 的 `_on_tool_proposal()` 预期优先使用 `event.input_summary` 作为 ContentPane tool trace 的 target；丢失后会回退到 `tool_call_id`，导致工具行无法显示用户最需要看的操作摘要，例如文件名、参数摘要或搜索词。

**建议处理**:

在 `assurance/tui/bridge.py::_make_tool_proposal()` 中传递 `input_summary=payload.get("input_summary", "")`，并补测试覆盖 bridge factory 与 projector tool trace。

**处理结果**: `bridge.py::_make_tool_proposal()` 已保留 `input_summary`；`test_tui.py::BridgeEventFactoryTests::test_build_tool_proposal_preserves_input_summary` 覆盖该映射。

### L2-002: ACP `user_message` 被误投影为模型 `text_delta`

**严重度**: 高  
**状态**: 已处理  
**涉及文件**:

- `assurance/grok_runtime_adapter.py`
- `assurance/tui/projector.py`
- `assurance/tui/widgets.py`

**证据**:

`grok_runtime_adapter.py::_handle_acp_notification()` 将 `assistant_message`、`message`、`user_message` 放在同一分支中，统一发出 `text_delta`。`projector.py` 收到 `TEXT_DELTA` 时会创建或追加“模型”消息卡片。

只读探针输入：

```python
{"sessionUpdate": "user_message", "text": "hello from user"}
```

输出：

```text
[{'event_type': 'text_delta', ..., 'payload': {'text': 'hello from user', 'turn': 0}}]
```

**影响**:

若 Grok ACP 发送用户消息回显，TUI 会把用户内容渲染成模型输出。这会污染对话记录、误导审计轨迹，也会破坏 ContentPane 中“用户/模型消息分离”的设计。

**建议处理**:

将 `user_message` 从 `text_delta` 分支中拆出，映射为 `user_message` / `model_request` / system echo 中更合适的一类，或在 adapter 层忽略用户回显。至少不能投影成模型 `text_delta`。

**处理结果**: `grok_runtime_adapter.py` 已将 ACP `user_message` 拆为独立 `user_message` 事件；TUI 新增 `UserMessageEvent`、bridge factory 与 projector handler，渲染为用户消息卡片而不是模型输出。

### L2-003: `assistant_message` 切片逻辑假设 ACP 内容是累计全文，若实际为增量 chunk 会丢 token

**严重度**: 高  
**状态**: 已处理  
**涉及文件**:

- `assurance/grok_runtime_adapter.py`

**证据**:

当前逻辑使用 `_last_streamed_text_len` 对 `assistant_message` 文本做差量切片：

```text
new_text = text[self._last_streamed_text_len:]
```

如果 ACP 每次发送累计全文，例如 `Hel` → `Hello`，该逻辑成立。  
如果 ACP 每次发送真正增量 chunk，例如 `Hel` → `lo`，第二个 chunk 会因长度小于 `_last_streamed_text_len` 而被吞掉。

只读探针：

```text
input chunks: ["Hel", "lo"]
output text_delta: ["Hel"]
```

**影响**:

若 Grok 当前或未来 ACP shape 是 chunk-delta 而非 cumulative-content，TUI 会静默丢失流式文本。该问题尤其危险，因为最终 `model_output` 会避免重复卡片，用户可能只看到不完整流式输出。

**建议处理**:

先用真实 ACP transcript 或 fixture 明确 Grok `assistant_message` 是 cumulative 还是 delta。随后：

- 若 cumulative：保留切片逻辑，但为 out-of-order / shorter text 做显式 guard 和测试。
- 若 delta：直接发送整段 `text` 为 `text_delta`，不要按历史长度切片。
- 若两种都可能出现：在 adapter 中记录 per-turn streaming mode，或按 message id / content shape 区分。

**处理结果**: adapter 已记录上一段完整 streaming text；当新文本以前文为前缀时按 cumulative suffix 发出，否则按 delta chunk 整段发出，并在每轮 prompt 重置状态。

### L2-004: Warning notification 被合并为 `error_event`，severity 丢失

**严重度**: 中  
**状态**: 已处理  
**涉及文件**:

- `assurance/grok_runtime_adapter.py`
- `assurance/tui/events.py`
- `assurance/tui/projector.py`

**证据**:

`method == "notification"` 时，`type == "error"` 和 `type == "warning"` 都被映射为 `event_type: "error_event"`，source 分别为 `acp_error` 或 `acp_warning`。TUI 的 `ErrorEvent` 无 severity 字段，projector 会把它们统一作为错误处理并点亮错误状态。

**影响**:

warning 与 error 的运行语义被合并。对用户来说，非阻断 warning 会显示成错误；对审计来说，也无法区分 runtime degraded signal 与 actual failure。

**建议处理**:

为 `ErrorEvent` 增加 `severity`，或新增 `WarningEvent` / `status_update` 映射。至少应让 `acp_warning` 在 UI 和 journal 中保留 warning 级别。

**处理结果**: ACP warning/error payload 已携带 `severity`；`ErrorEvent` 增加 `severity` 字段；bridge 保留 severity；projector 将 warning 放入 `Warnings` 分组并显示 warning 状态。

### L2-005: ACP event mapping 缺少直接测试覆盖

**严重度**: 中  
**状态**: 已处理  
**涉及文件**:

- `assurance/tests/test_grok_runtime_adapter.py`
- `assurance/tests/test_tui.py`
- `assurance/tests/test_grok_tui_wiring.py`

**证据**:

本轮 `Select-String` 检查显示，现有 Grok adapter 测试主要覆盖 version-smoke、containment 和 prompt-tool gate；未直接覆盖：

- `assistant_message` → `text_delta`
- `user_message` 不应变成模型 `text_delta`
- `tool_call_update` payload 到 `ToolProposalEvent.input_summary`
- `tool_result` 到 tool trace
- warning/error severity 分离

**影响**:

L2 中最容易回归的是 event mapping 层，而当前通过的 48 个 Grok/adapter 相关测试无法证明这些映射正确。

**建议处理**:

新增小型 no-model tests，直接调用 `_handle_acp_notification()` 和 `build_event_from_jsonl_line()`，再用 `projector.apply_event()` 验证 ContentPane 状态。

**处理结果**: 已新增 no-model mapping 测试覆盖 `user_message`、assistant cumulative、assistant delta、warning severity、tool proposal `input_summary`、bridge `UserMessageEvent` 与 projector user/warning 行为。

### L2 测试记录

| 命令 | 结果 |
|------|------|
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_runtime_adapter.py assurance/tests/test_grok_tui_wiring.py assurance/tests/test_grok_prompt_tool_gate.py assurance/tests/test_job_object_supervisor.py -q` | 26 passed, 1 skipped |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_event_normalizer.py assurance/tests/test_grok_session_verifier.py assurance/tests/test_grok_tool_permission_observer.py -q` | 22 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py -q` | timed out after 120s; partial progress reached 32%+ with no failure emitted before timeout |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_runtime_adapter.py -q` | 9 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::BridgeEventFactoryTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::ProjectorTypedGateHandlerTests -q` | 35 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_tui_wiring.py assurance/tests/test_grok_prompt_tool_gate.py -q` | 10 passed |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |

### L2 下一步

1. L2-001 至 L2-005 已处理并通过窄集复核。
2. 后续若取得真实 ACP transcript，可继续把当前 cumulative/delta 兼容逻辑替换或收敛为与上游 shape 完全一致的策略。
3. 下一层审查可转入 L3 TUI ContentPane 视觉规则与渲染测试覆盖。

---

## 8. 当前优先级建议

第一优先级：

1. 将 2026-07-31 全局审查清单中过期的 TUI / session GAP 做重分类。
2. 开始 L3 TUI ContentPane 审查，重点复核 `text_delta` / ContentPane / tool trace 的直接测试与视觉规则。
3. 若后续拿到真实 Grok ACP transcript，回填 runtime adapter streaming shape 证据。

第二优先级：

1. 继续抽样复核 P0-P5 / Gate / Security 与当前实现的一致性。
2. 更新 `CLI_PROJECT_INDEX.md` 或新增 2026-08-01 checklist，将 L1 已闭合状态回填。

第三优先级：

1. 重新运行窄集和全量测试。
2. 在测试绿后产出正式“2026-08-01 全局审查结论”或继续登记剩余 findings。
