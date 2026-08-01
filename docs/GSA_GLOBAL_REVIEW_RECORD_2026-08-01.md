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
**状态**: 已处理（L4 审查完成）
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

- [x] 对照 `CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` 审查视觉规则。已完成第一切片，见 L3 审查结果。
- [x] 补 `text_delta` 到 ContentPane 的直接单测。已覆盖同一卡片追加与最终 `model_output` 不重复。
- [x] 补 Markdown / code block / width wrapping 测试。已覆盖 Python code block 高亮与 display width。
- [x] 补工具行折叠、展开、pin-to-top、run finish collapse 测试。已覆盖 pin-to-top；run finish collapse 由既有 projector 路径与 L2/L3 窄集复核覆盖。
- [x] 审查 ANSI 颜色与 display width 的相互影响。已修复 `widgets.py` 与 `app.py` 双入口宽度规则分叉。

### L4. Session / Layout

- [x] 按 2026-08-01 所有权修正审查 session 列表。已完成，见 L4 审查结果。
- [x] 确认 `session.json` marker 是薄发现层，不构成独立 session store。已覆盖 `.gsa/runs/*/session.json` 扫描，不创建 `.gsa/sessions/index.jsonl`。
- [x] 确认 UI 不实现自有恢复，只提示或委托 Grok。Enter 返回 `grok session resume <session_id>` 提示，不执行恢复。
- [x] 更新旧清单中 `gsa session list/resume` 的错误缺口描述。已重分类为历史 GAP，不再作为本仓库自有 CLI 目标。

### L5. P0-P5 / Gate / Security 抽样复核

- [x] 抽样验证 P1/P4 archive 文档与代码接口仍一致。P1/P4 窄集 64 passed；修复 RecoveryExecutor 测试隔离。
- [x] 抽样验证 retrieval subagent registry 与 completion check 仍满足“两子代理”约束。registry 检查通过，仍为 project-doc + external 两类。
- [x] 抽样验证 network permit、child capability、workspace trust 仍覆盖当前入口。修复 endpoint allow/denylist 与 Adapter Gate network permit 阻断语义。
- [x] 判断 ACP inline gate hook 是否仍只是增强项，还是已成为生产路径阻断项。当前 CLI doctor 仍声明 Grok prompt/tool promotion gate fail-closed；TUI inline tool availability gate 是附加投影，不替代 production promotion gate。

### L6. Schema 与 Repository Check

- [x] 修复 `scripts/check_repository.py` 暴露的 schema/lock 错误。独立运行 `scripts/check_repository.py` 通过，`valid: true` / `error_count: 0`。
- [x] 跑 `python gsa.py doctor --json`，要求 `valid: true`。已通过，doctor repository check `valid: true` / `error_count: 0`。
- [x] 跑 schema validation 相关测试，确认没有因 lock 漂移产生连带失败。Adapter / canonical / network permit 相邻测试通过，见 L6 记录。

### L7. 测试策略

- [x] 先跑窄集：global review、CLI dispatcher、Grok adapter、TUI wiring、TUI ContentPane。L7 narrow set 78 passed。
- [x] 窄集修绿后跑 `assurance/tests` 全量。修复 shadow recovery 测试隔离与 TUI address focus 后，1524 passed / 14 skipped。
- [x] 最后跑 runtime / integration / scripts 相关测试。runtime / integration 明确集合 79 passed；doctor 与 repository check 均 `valid: true`。
- [x] 对需要真实 Grok、网络、管理员权限或 Windows 特性的测试单独记录前置条件，不和普通离线测试混为一类。14 skipped 已按原因记录在 L7。

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

## 8. L3 审查结果: TUI ContentPane

**审查日期**: 2026-08-01
**判定**: 已处理并通过 L3 窄集复核
**通过面**: `text_delta` 追加到同一模型消息卡片、最终 `model_output` 不重复生成卡片、工具展开 pin-to-top、来源条/展开工具存在时仍按请求高度渲染、Python code block 高亮不破坏宽度。
**核心结论**: ContentPane 主体设计已基本符合 `CONTENT_PANE_CONVERSATION_RENDERING_v0.1.md` 的分层规则；本轮发现并修复了两个 L3 渲染不变量缺口，剩余风险主要在完整 `test_tui.py` 后段耗时导致全文件未在本轮跑完。

### L3-001: ANSI 高亮被当作可见宽度，导致 code block 错误折行

**严重度**: 中
**状态**: 已处理
**涉及文件**:

- `assurance/tui/widgets.py`
- `assurance/tui/app.py`
- `assurance/tests/test_tui.py`

**证据**:

本轮探针用 Python fenced code block 渲染：

```text
def add(x):
    return x + 1
```

修复前，`return x + 1` 被错误拆成两行，根因是 `display_width()` 把 ANSI SGR 转义序列计入可见宽度；同时 `pad_to_width()` 在宽度相等时仍按字符切片，可能截断 ANSI reset。`app.py` 内另有一份本地 `pad_to_width()`，会在 ChatInput 反色光标路径复现同类问题。

**影响**:

违反 ContentPane 设计中“Markdown / code block / syntax highlighting 宽度不破坏渲染”的要求。实际终端里可能出现代码行误折、ANSI reset 被截断、整屏行宽计算虚假通过但视觉不满宽。

**处理结果**:

- `widgets.display_width()` 已跳过 ANSI SGR 转义序列。
- `widgets.pad_to_width()` 已在可见宽度相等时保留原字符串，并通过 `truncate_to_width()` 做 display-cell aware 截断。
- `app.pad_to_width()` 已委托到 `widgets.pad_to_width()`，避免宽度规则分叉。
- 新增 `ContentPaneRenderTests::test_python_code_block_highlight_does_not_force_extra_wrap` 覆盖 Python code block 高亮与整行 display width。

### L3-002: Source bar / expanded tool 存在时 ContentPane 未填满请求高度

**严重度**: 中
**状态**: 已处理
**涉及文件**:

- `assurance/tui/widgets.py`
- `assurance/tests/test_tui.py`

**证据**:

修复前探针显示：

```text
height 10 actual 8
height 20 actual 18
expanded actual 9
```

根因是 `_render_items()` 在 source compact bar 或 pinned expanded tool 已经占用行数后，用 `remaining_height` 作为最终补齐目标，导致返回行数少于调用方请求的 `height`。

**影响**:

违反 widget render “填满 width × height”的基础约定。组合到三栏布局时，后续 row composition 需要额外兜底，容易造成 ContentPane 下方空洞、状态栏附近行宽错位，且 expanded tool pin-to-top 场景最容易触发。

**处理结果**:

- `_render_items()` 现在始终补齐到请求的 `height`。
- 新增测试覆盖 source compact bar + conversation、expanded tool pin-to-top 两个场景。

### L3-003: `text_delta` / tool trace / Markdown 直接测试覆盖不足

**严重度**: 中
**状态**: 已处理第一切片
**涉及文件**:

- `assurance/tests/test_tui.py`

**证据**:

L2 已补 bridge / projector 的事件映射测试，但 L3 设计面仍缺少直接断言：

- 首个 `text_delta` 创建模型卡片，后续 chunk 追加同一卡片。
- 最终 `model_output` 不重复创建卡片。
- 展开工具行固定在 ContentPane 顶部。
- Markdown fenced code block 与 ANSI 高亮不破坏 display width。

**处理结果**:

已新增直接测试：

- `ContentPaneRenderTests::test_conversation_with_source_bar_fills_requested_height`
- `ContentPaneRenderTests::test_expanded_tool_is_pinned_and_fills_requested_height`
- `ContentPaneRenderTests::test_python_code_block_highlight_does_not_force_extra_wrap`
- `ProjectorUnitTests::test_text_delta_appends_to_one_model_card_and_output_does_not_duplicate`

### L3 测试记录

| 命令 | 结果 |
|------|------|
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::BridgeEventFactoryTests -q` | 38 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_tui_wiring.py assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::BridgeEventFactoryTests -q` | 39 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::FullScreenRenderTests::test_lines_are_consistent_width assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::BridgeEventFactoryTests -q` | 39 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::DisplayWidthTests assurance/tests/test_tui.py::PadToWidthTests assurance/tests/test_tui.py::FullScreenRenderTests::test_lines_are_consistent_width assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::BridgeEventFactoryTests -q` | 50 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py -x -q` | timed out after 180s；超时前无失败，进度约 31%+ |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |

### L3 下一步

1. L3-001 至 L3-003 已处理并通过窄集复核。
2. 仍建议后续拆分或标记 `test_tui.py` 后段慢测试，以便全文件 TUI 回归能在常规审查窗口内完成。
3. 后续若增加 rich markdown、更多语言高亮或真实终端截图测试，应继续复用同一 display-width 工具，避免 `app.py` / `widgets.py` 宽度规则再次分叉。

---

## 9. L4 审查结果: Session / Layout

**审查日期**: 2026-08-01
**判定**: 已处理并通过 L4 窄集复核
**通过面**: Address / Find 折叠至 Toolbar 的既有布局路径保持；session 持久化与恢复所有权已按 2026-08-01 裁决归 Grok；TUI 会话列表只读扫描 `.gsa/runs/*/session.json` 薄 marker；UI 不执行本仓库自有恢复。
**核心结论**: L4 的主要问题不是继续实现 `gsa session list/resume`，而是确保 TUI 产生的 Grok ACP run root 能被只读 session list 发现，并且 UI 只给出 Grok 原生命令提示。本轮已修复 run root 位置与恢复提示缺口。

### L4-001: TUI 新 ACP run root 不在 `.gsa/runs/`，会话列表发现链断裂

**严重度**: 中
**状态**: 已处理
**涉及文件**:

- `assurance/tui/app.py`
- `assurance/tui/bridge.py`
- `assurance/tests/test_tui.py`

**证据**:

`architecture/SESSION_PERSISTENCE_AND_LAYOUT_COMPACTION_v0.1.md` 要求 TUI 只读扫描 `.gsa/runs/<session_id>/session.json`。但修复前 `TuiPrototype._start_run()` 使用 `tempfile.mkdtemp(prefix="gsa-run-")` 创建系统临时 run root；bridge 虽会在该 run root 写 `session.json`，但 `_discover_sessions()` 只扫描 `.gsa/runs`，因此 TUI 自己启动的 ACP session 不会出现在 session list 中。

**影响**:

会话列表 UI 与实际 ACP run 产物脱节。用户双 Esc 进入 session list 时，可能看不到刚刚由 TUI 创建的 Grok ACP 会话，从而误判 session persistence 未工作。

**处理结果**:

- `_start_run()` 现在创建 `.gsa/runs/S-YYYYMMDD-<8-hex>/workspace`。
- `run_id` 显式传给 `build_grok_acp_live_run_fn()`，与 run root 目录名一致。
- 新增 `_new_session_run_root()`，只负责分配发现用 run root，不引入独立 session store。
- 新增 `SessionListOwnershipTests::test_start_run_allocates_gsa_runs_session_id_and_passes_it_to_bridge`。

### L4-002: Session list Enter 只返回 session id，未明确 Grok 原生恢复命令

**严重度**: 中
**状态**: 已处理
**涉及文件**:

- `assurance/tui/app.py`
- `assurance/tests/test_tui.py`

**证据**:

设计要求选中会话后显示 session ID，并说明完整恢复使用：

```text
grok session resume <session_id>
```

修复前 `_navigate_session_list("enter")` 仅返回：

```text
会话: <session_id>
```

**影响**:

UI 没有明确恢复所有权，容易被误读为本仓库即将或已经实现自有恢复逻辑，也不利于用户按正确命令恢复 Grok session。

**处理结果**:

Enter 现在返回：

```text
会话: <session_id>；恢复: grok session resume <session_id>
```

新增 `SessionListOwnershipTests::test_enter_on_session_reports_grok_resume_command` 覆盖该行为。

### L4-003: `session.json` marker 边界缺少直接测试

**严重度**: 中
**状态**: 已处理第一切片
**涉及文件**:

- `assurance/tui/app.py`
- `assurance/tui/widgets.py`
- `assurance/tests/test_tui.py`

**证据**:

L4 清单要求确认 `session.json` 是薄发现层，不构成独立 session store。修复前没有直接测试证明：

- `_discover_sessions()` 只读扫描 `.gsa/runs/*/session.json`。
- 不创建或依赖 `.gsa/sessions/index.jsonl`。
- 缺少 `created_at` 时，session list 仍可从 `S-YYYYMMDD-...` session id 回退出日期分组。

**处理结果**:

- 新增 `SessionListOwnershipTests::test_discover_sessions_reads_only_session_markers_under_gsa_runs`。
- 新增 `SessionListOwnershipTests::test_session_list_uses_session_id_date_when_marker_timestamp_is_empty`。
- `ExplorerPane._render_session_list()` 已在 marker 时间戳为空时从 session id 推导日期分组。
- 只读扫描仍不写 index，不实现自有 resume/archive/delete。

### L4 测试记录

| 命令 | 结果 |
|------|------|
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::SessionListOwnershipTests assurance/tests/test_tui.py::FullScreenRenderTests::test_lines_are_consistent_width -q` | 5 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py::SessionListOwnershipTests assurance/tests/test_tui.py::DisplayWidthTests assurance/tests/test_tui.py::PadToWidthTests assurance/tests/test_tui.py::FullScreenRenderTests::test_lines_are_consistent_width assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::BridgeEventFactoryTests -q` | 54 passed |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |

### L4 下一步

1. L4-001 至 L4-003 已处理并通过窄集复核。
2. 2026-07-31 checklist 中 `gsa session list/resume` 相关 GAP 已重分类为历史 GAP，不再作为本仓库自有 CLI 目标。
3. 下一层审查可进入 L5 P0-P5 / Gate / Security 抽样复核。

---

## 10. L5 审查结果: P0-P5 / Gate / Security 抽样复核

**审查日期**: 2026-08-01
**判定**: 已处理并通过 L5 窄集复核
**通过面**: P1/P4 archive / audit / recovery 抽样测试通过；retrieval subagent registry 仍强制两类默认子代理；workspace trust entry-point auditor 通过；child capability / network permit / DeepSeek adapter permit 测试通过；Grok prompt/tool promotion 仍是 fail-closed gate，TUI ACP inline tool availability 只是附加投影。
**核心结论**: L5 抽样确认大部分安全链路仍保持现有设计分类；本轮修复了 network permit endpoint allowlist 的边界漏洞、Adapter Gate network policy 只记录不阻断的歧义，以及 P4 recovery executor 在测试中依赖仓库级 execution receipt 目录的问题。

### L5-001: Network permit endpoint allowlist 对已 canonical endpoint 未必生效

**严重度**: 高
**状态**: 已处理
**涉及文件**:

- `assurance/network_permit_gateway.py`
- `assurance/tests/test_trust_child_network.py`

**证据**:

修复前，`evaluate_network_permit()` 只有在 `allowed_endpoints is not None and canonical != endpoint` 时才检查 endpoint allowlist。若输入 endpoint 本身已经是 canonical 形式，例如：

```text
https://evil.example/api
```

则 `canonical == endpoint`，代码进入 `else` 并把 `endpoint_allowed` 置为 true，即使 `allowed_endpoints={"api.deepseek.com"}`。

**影响**:

类别 allowlist 仍然有效，但 endpoint allowlist 对已规范化的恶意或错误 endpoint 可能被绕过。这直接影响 `DeepSeek` / adapter gate 的“允许固定 provider endpoint”语义。

**处理结果**:

- endpoint allowlist 现在始终在 `allowed_endpoints` 存在时执行。
- 支持 host allowlist（如 `api.deepseek.com`）、完整 canonical endpoint、以及 canonical prefix。
- 新增 denylist 检查，`denied_endpoints` 命中时即使 category allow 也 block。
- 新增测试：
  - `NetworkPermitGatewayTests::test_endpoint_allowlist_blocks_already_canonical_disallowed_endpoint`
  - `NetworkPermitGatewayTests::test_endpoint_denylist_blocks_even_when_category_allowed`

### L5-002: Adapter Gate network policy 只写 receipt 字段，不阻断 adapter call

**严重度**: 高
**状态**: 已处理
**涉及文件**:

- `assurance/adapter_gate.py`
- `assurance/tests/test_adapter_gate_bypass.py`

**证据**:

修复前，`enforce_adapter_call()` 会计算：

```text
network_permit_required = true
network_permit_granted = false
```

但 `adapter_call_allowed` 只取决于 IPG receipt 是否有效且 decision 是否为 allow。也就是说，当 `AdapterGateContext` 携带 require-all network policy 时，receipt 可能显示 permit 未授予，但 adapter call 仍会被执行。DeepSeek adapter 自身会在 HTTP 前再调用 `evaluate_network_permit()`，但 Adapter Gate receipt 语义仍不一致，且非 DeepSeek adapter 可能没有下层兜底。

**影响**:

破坏“Adapter Gate 是模型实际调用前最后一道机械门禁”的审计语义。对真实 adapter 扩展来说，调用方可能误以为只要把 network policy 放进 `AdapterGateContext` 就完成了硬阻断。

**处理结果**:

- `enforce_adapter_call()` 现在在 `network_policy.require_permit_for_all` 为 true 时，先调用 `evaluate_network_permit()`。
- 缺少 endpoint/category metadata 或 permit 被拒时，`adapter_call_allowed=false`，并抛出 `AdapterGateBlockedError`，adapter call 不执行。
- enforcement receipt 中 `network_permit_required` / `network_permit_granted` 现在与实际执行一致。
- 新增测试：
  - `AdapterGateEnforcementTests::test_network_policy_permit_allows_adapter_call`
  - `AdapterGateEnforcementTests::test_network_policy_permit_blocks_adapter_call`

### L5-003: P4 RecoveryExecutor 测试依赖仓库级 shadow executions 目录

**严重度**: 中
**状态**: 已处理
**涉及文件**:

- `assurance/shadow_recovery.py`
- `assurance/tests/test_p4_audit_compaction_recovery.py`

**证据**:

P1/P4 抽样复核时，`test_shadow_store_and_execute_cycle` 失败：

```text
PermissionError: [WinError 5] 拒绝访问:
D:\CLI\.gsa_shadow_recovery\executions\...
```

测试已把 `ShadowRecoveryStore(repo_root=self.root)` 隔离到临时目录，但 `RecoveryExecutor.execute()` 固定把 execution receipt 写入模块级 `SHADOW_EXECUTIONS`，也就是仓库根下 `.gsa_shadow_recovery/executions`。

**影响**:

测试隔离不完整；在受限 workspace 或权限异常的仓库级 shadow dir 中，P4 recovery executor 测试会失败。更重要的是，恢复执行 receipt 的落点无法由调用方隔离，削弱了 conformance / disposable-run 场景的可复现性。

**处理结果**:

- `RecoveryExecutor.execute()` 新增可选 `execution_root` 参数，默认仍使用 `SHADOW_EXECUTIONS`。
- P4 测试显式传入 `execution_root=self.root / "executions"`。
- P1/P4 抽样集合复跑通过。

### L5-004: ACP inline tool availability gate 仍是附加投影，不替代 production promotion gate

**严重度**: 中
**状态**: 已重分类 / 无代码修改
**涉及文件**:

- `assurance/tui/bridge.py`
- `assurance/cli.py`
- `assurance/grok_prompt_tool_gate.py`

**证据**:

TUI ACP bridge 在 `tool_proposal` 事件旁边追加 inline `gate_decision`，用于 UI 可见性和运行中提示。但 CLI doctor 仍声明：

```text
gsa run --runtime grok emits a promotion gate receipt only; it does not launch prompt/tool execution.
```

`gsa run --runtime grok` 仍走 `grok_prompt_tool_gate` fail-closed promotion receipt；只有显式 `--grok-execute` 且 gate allow 时才进入 Grok smoke 路径。

**判定**:

ACP inline hook 是增强项 / runtime projection，不是替代 `grok_prompt_tool_gate` 的 production 阻断项。后续若要提升为生产阻断，需要单独审查 schema、receipt、terminal outcome 和 fake-tool permission verification，不应从 TUI bridge 的 inline projection 直接推断。

**2026-08-01 后续处理**:

- `gsa grok run` 新增显式 `--mode acp-smoke --ask <prompt>` 入口，用于直接运行 Grok ACP adapter smoke；这不改变 `gsa run` 默认 canonical 路径，也不替代 `gsa run --runtime grok` promotion gate。
- `grok_runtime_adapter.py` 的 ACP receipt 已补齐 schema 合规的 prompt / acp / containment / artifact 字段；`runtime/run-event-v0.1.schema.json` 已登记 `prompt_submitted`、`model_response_received`、`acp_initialize`、`acp_session_created`、`permission_requested` 等 Grok normalized event 类型。
- ACP drain 对不支持 `select()` 的 stdout 流 fail-soft 退出，避免 Windows / fake stream 在会话结束阶段破坏 receipt 写入。
- `gsa.py tui --runtime grok --fake-provider --run <prompt>` 已接入 TUI bridge；loopback ACP smoke 会实时投影 `text_delta`，并从 normalized events 回放最终 `model_output`。该路径仍是固定 fixture，不代表真实 DeepSeek 自动接入。

### L5 测试记录

| 命令 | 结果 |
|------|------|
| `python -m pytest -p no:cacheprovider assurance/tests/test_adapter_gate_bypass.py::AdapterGateEnforcementTests assurance/tests/test_trust_child_network.py::NetworkPermitGatewayTests assurance/tests/test_trust_child_network.py::AdapterGateContextTrustTests assurance/tests/test_trust_child_network.py::DeepSeekNetworkPermitTests -q` | 26 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_retrieval_subagent.py::RetrievalSubagentRegistryTests assurance/tests/test_trust_child_network.py::EntryPointAuditorTests -q` | 10 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_p1_identity_and_retention.py assurance/tests/test_session_namespace.py assurance/tests/test_archive_controller.py assurance/tests/test_archive_recovery.py assurance/tests/test_p4_audit_compaction_recovery.py assurance/tests/test_audit_integration.py -q` | initial 63 passed / 1 failed; after fix 64 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_adapter_gate_bypass.py::AdapterGateEnforcementTests assurance/tests/test_trust_child_network.py::NetworkPermitGatewayTests assurance/tests/test_trust_child_network.py::AdapterGateContextTrustTests assurance/tests/test_trust_child_network.py::DeepSeekNetworkPermitTests assurance/tests/test_retrieval_subagent.py::RetrievalSubagentRegistryTests assurance/tests/test_trust_child_network.py::EntryPointAuditorTests -q` | 36 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_adapter_integration.py assurance/tests/test_endpoint_canonicalizer.py assurance/tests/test_deepseek_adapter.py assurance/tests/test_canonical_cli.py -q` | 119 passed, 1 skipped |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |

### L5 下一步

1. L5-001 至 L5-003 已处理并通过窄集复核；L5-004 已重分类。
2. 后续可继续 L6 Schema 与 Repository Check；当前 doctor 已为 `valid: true`，但 L6 仍应单独关注 schema drift、receipt schema 是否需要补字段，以及 repository check 的覆盖盲区。
3. 若后续要把 ACP inline hook 提升为生产阻断项，需新增独立 promotion receipt / verifier，而不是复用 TUI projection。

---

## 11. L6 审查结果: Schema 与 Repository Check

**审查日期**: 2026-08-01
**判定**: 已处理并通过 L6 repository / schema 复核
**通过面**: `scripts/check_repository.py` 独立通过；`python gsa.py doctor --json` 通过；P0 contract、Adapter Gate、Network Permit、Adapter Integration、Endpoint Canonicalizer、DeepSeek Adapter、Canonical CLI 相邻测试通过。

### L6-001: Adapter Gate verifier 未把 network permit 状态纳入允许条件

**状态**: 已修复
**位置**: `assurance/adapter_gate.py`

L5 已将 `network_policy.require_permit_for_all` 接入 `enforce_adapter_call()`，执行路径会在 `gate_decision == "allow"` 之外继续要求 `network_permit_granted`。但 `verify_adapter_gate_enforcement()` 的独立复核仍只按 `gate_valid && gate_decision == "allow"` 计算 `adapter_call_allowed`，会把 “IPG allow 但 network permit 未授予且 adapter_call_allowed=false” 的合法阻断收据误判为不一致，也会降低对伪造允许收据的语义约束。

修复后 verifier 的期望允许条件为：

```text
gate_valid && gate_decision == "allow" && (!network_permit_required || network_permit_granted)
```

新增 `test_verifier_accounts_for_network_permit_state` 覆盖两类情况：

1. `network_permit_required=true` 且 `network_permit_granted=false` 时，`adapter_call_allowed=false` 是一致状态。
2. 同一状态下伪造 `adapter_call_allowed=true` 会被 verifier 判 invalid。

### L6-002: Network Permit verifier 未按 policy 重放 endpoint 与 attempt 约束

**状态**: 已修复
**位置**: `assurance/network_permit_gateway.py`

`evaluate_network_permit()` 已支持 category allowlist、endpoint allowlist、endpoint denylist、attempt budget、retry previous status 等执行时约束；但 `verify_network_permit_receipt()` 只复核 category，导致结构合法的收据在不同 policy 下可能被误判为 valid。

修复后 verifier 会独立复核：

1. `permit_granted` 与 `errors` 的一致性。
2. first attempt / retry previous status 语义。
3. `policy.allowed_categories`。
4. `policy.max_attempts_per_turn`。
5. canonical endpoint against `policy.allowed_endpoints`。
6. canonical endpoint against `policy.denied_endpoints`。

新增测试覆盖 policy endpoint allowlist mismatch、denylist mismatch、attempt budget mismatch。

### L6-003: Repository / schema lock 状态

**状态**: 通过

| 命令 | 结果 |
|------|------|
| `python .\scripts\check_repository.py` | passed, `valid: true`, `error_count: 0`, schemas = 182 |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |
| `python -m pytest -p no:cacheprovider assurance/tests/test_adapter_gate_bypass.py::AdapterGateEnforcementTests assurance/tests/test_trust_child_network.py::NetworkPermitGatewayTests -q` | 25 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_p0_contracts.py assurance/tests/test_adapter_gate_bypass.py assurance/tests/test_trust_child_network.py -q` | 70 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_adapter_integration.py assurance/tests/test_endpoint_canonicalizer.py assurance/tests/test_deepseek_adapter.py assurance/tests/test_canonical_cli.py -q` | 119 passed, 1 skipped |

### L6 下一步

1. L6 已完成；当前未发现需要新增 receipt schema 字段，现有 schema 已允许 `network_permit_required` / `network_permit_granted` / `network_endpoint` 与 network receipt `checks.endpoint_not_denied`。
2. 下一阶段进入 L7 测试策略：先跑窄集，再视时间与风险跑 `assurance/tests` 全量。
3. 若后续新增 promotion receipt 或 ACP inline hook 阻断项，需要同步新增 schema 与 verifier，而不能只改 UI projection。

---

## 12. L7 审查结果: 测试策略

**审查日期**: 2026-08-01
**判定**: 已处理并通过 L7 测试策略复核
**通过面**: 窄集、全量 `assurance/tests`、runtime / integration 明确集合、doctor、repository check 均通过；跳过项均有明确环境前置条件。

### L7-001: `test_shadow_recovery.py` 仍有多处执行收据写入仓库级 shadow executions

**状态**: 已修复
**位置**: `assurance/tests/test_shadow_recovery.py`

L5 已给 `RecoveryExecutor.execute()` 增加 `execution_root` 参数，并修复了 P4 抽样中的一个隔离调用点。L7 全量测试暴露同类缺口仍存在于 `test_shadow_recovery.py`：7 个恢复执行测试仍使用默认仓库级 `.gsa_shadow_recovery/executions`，在受限 workspace 中触发 `PermissionError: [WinError 5]`。

处理结果：

1. `RecoveryExecutorTests.setUp()` 与 `EndToEndRecoveryTests.setUp()` 新增临时 `self.execution_root`。
2. 所有会产生 execution receipt 的 `self.executor.execute()` 测试调用显式传入 `execution_root=self.execution_root`。
3. `test_execution_receipt_verification` 改为从临时 execution root 查找持久化 receipt。
4. 生产默认路径保持不变，测试隔离不改变 runtime 行为。

### L7-002: TUI focus cycle 缺少 `address` pane，导致 slash-command / cancel-run 测试无限循环

**状态**: 已修复
**位置**: `assurance/tui/app.py` / `assurance/tests/test_tui.py`

`SlashCommandIntegrationTests` 与 `EscCancelRunTests` 多处通过 F6 循环到 address bar；`TuiPrototype._cancel_run()` 内部也会把焦点切回 address bar。但 `_focusable_panes` 当前只包含 `explorer / checklist / content / marker / chat`，不含 `address`，导致测试循环和真实 cancel-run refocus 路径都有无限循环风险。

处理结果：

1. `_focusable_panes` 重新纳入 `address`，顺序为 `explorer -> checklist -> content -> marker -> address -> chat`。
2. 默认 `_active_pane_index` 调整为 `chat` 对应的新 index，保留默认输入焦点。
3. `KeyboardNavigationTests::test_f6_cycles_focus` 更新为 6-pane cycle，保留默认 chat 下 F6 进入 explorer 的既有手感。
4. `TuiEventDataclassTests::test_all_event_kinds_recognised` 的事件数量更新为 29，匹配已存在的 `TEXT_DELTA` 事件类型。

### L7 测试记录

| 命令 | 结果 |
|------|------|
| `python -m pytest -p no:cacheprovider assurance/tests/test_global_review_mode.py assurance/tests/test_cli_dispatcher.py assurance/tests/test_grok_runtime_adapter.py assurance/tests/test_grok_tui_wiring.py assurance/tests/test_tui.py::ContentPaneRenderTests assurance/tests/test_tui.py::ProjectorUnitTests assurance/tests/test_tui.py::SessionListOwnershipTests assurance/tests/test_tui.py::BridgeEventFactoryTests assurance/tests/test_tui.py::ProjectorTypedGateHandlerTests -q` | 78 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_shadow_recovery.py -q` | 24 passed |
| `python -m pytest -p no:cacheprovider assurance/tests/test_tui.py -q` | 236 passed |
| `python -m pytest -p no:cacheprovider assurance/tests -q` | 1524 passed, 14 skipped |
| `python -m pytest -p no:cacheprovider assurance/tests -q -rs` | 1524 passed, 14 skipped; skip reasons recorded below |
| `python -m pytest -p no:cacheprovider assurance/tests/test_grok_runtime_adapter.py assurance/tests/test_grok_tui_wiring.py assurance/tests/test_adapter_integration.py assurance/tests/test_canonical_cli.py assurance/tests/test_cli_dispatcher.py -q` | 79 passed |
| `python gsa.py doctor --json` | passed, `valid: true`, repository errors = 0 |
| `python .\scripts\check_repository.py` | passed, `valid: true`, `error_count: 0`, schemas = 182 |

### L7 skip 前置条件记录

| 跳过范围 | 数量 | 原因 |
|------|------:|------|
| `test_browser_retrieval.py` live browser | 4 | 需要 `GSA_RUN_LIVE_BROWSER_TESTS=1` 与 Chrome/Edge |
| `test_browser_retrieval_e2e.py` live browser E2E | 6 | 需要 `GSA_RUN_LIVE_BROWSER_TESTS=1` 与 Chrome/Edge |
| `test_deepseek_adapter.py` non-Windows path | 1 | 当前为 Windows 环境，该用例覆盖非 Windows 分支 |
| `test_job_object_supervisor.py` non-Windows only | 1 | 当前为 Windows 环境，该用例只覆盖非 Windows 分支 |
| `test_windows_race_escape.py` Windows attribute-list probe | 1 | `InitializeProcThreadAttributeList failed: 122`，环境探针前置条件未满足 |
| `test_windows_sandbox.py` AppContainer probe | 1 | 当前环境无法创建 AppContainer probe process，缺少对应系统可执行路径/能力 |

### L7 下一步

1. L7 已完成；当前测试状态可作为 L1-L7 闭合后的基线。
2. 后续优先更新 `CLI_PROJECT_INDEX.md` 或新增 2026-08-01 checklist，将 L1-L7 已闭合状态回填。
3. 若需要正式收尾，可产出“2026-08-01 全局审查结论”，并明确 loopback Grok ACP transcript 已观察，真实 DeepSeek Grok ACP transcript 仍是外部证据待回填项。

---

## 13. 索引回填结果: CLI_PROJECT_INDEX

**审查日期**: 2026-08-01
**判定**: 已完成索引回填
**位置**: `CLI_PROJECT_INDEX.md`

处理结果：

1. 顶部更新时间更新为 2026-08-01，摘要写入 L1-L7 闭合状态。
2. A0 新增 `2026-08-01 Global Review L1-L7 Closure` 短索引入口，指向本审计文档。
3. Network Permit Gate / Adapter Gate / Grok Build / CLI UI Interaction Model 条目补入 2026-08-01 复核状态与关键边界。
4. 附A审计文档速查新增 2026-08-01 全局审查 L1-L7 闭合入口。

### 索引回填下一步

1. 可产出正式“2026-08-01 全局审查结论”。
2. loopback Grok ACP transcript 已观察；真实 DeepSeek Grok ACP transcript 仍为外部证据待回填项，不应在结论中写成已观察。

---

## 14. 当前优先级建议

第一优先级：

1. 将 2026-07-31 全局审查清单中过期的 TUI / session GAP 做重分类。L3/L4 第一切片已完成，L6/L7 已完成。
2. 产出正式“2026-08-01 全局审查结论”。
3. 若后续拿到真实 DeepSeek Grok ACP transcript，回填 runtime adapter streaming shape 证据。

第二优先级：

1. 继续抽样复核 P0-P5 / Gate / Security 与当前实现的一致性。
2. 如需进一步细化，可新增单独 2026-08-01 checklist，但当前 `CLI_PROJECT_INDEX.md` 已完成主索引回填。

第三优先级：

1. 重新运行窄集和全量测试。
2. 在测试绿后产出正式“2026-08-01 全局审查结论”或继续登记剩余 findings。
