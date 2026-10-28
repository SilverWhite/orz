# 376 · M1 v1 六项全量整改（2026-10-28）

## 0. 结论

本批按用户令把 **366 §1 / 架构 §13 的 v1 六项全部落齐**，不再用“路线最小闭包”缩小范围：

1. state 薄层 + `state_read/state_write` + `blackboard_read/write section=state`；
2. `context_manage(mode=map)` 压缩页表；
3. `context_manage(mode=recenter)` 回中；
4. capability registry 薄层（工具白名单 + session preopen + 审计）；
5. journal 的 state/context/checkpoint 事件闭环，checkpoint 有真实 producer；
6. 清理 resident 元指令（`[permission]` 行与 `<plan_framework>` 常驻前置）。

同时处理了 370/375 审查暴露的 P1/P2：provider tokenizer 硬预算、state no-op/空态、context_manage fail-closed、recenter `next` 不可截、context_map 实际交付块数、capability 越权审计、checkpoint 原子写。

**状态：代码与测试已落，工作树未提交、未重建、未换装。** 在役 0.9.14-3 不含本批。

## 1. 六项落点

### 1.1 state（366 §1 item 1）

- `orz-loop/src/state.rs`：固定 6 字段 `goal/now/next/open/constraints/updated`；合并语义；空串/`none`/`无` 清空单字段；`updated` 系统写 `r{round}@{domain} {ts}`；`parse_document` 只认逐字 `state v1`，未知 key/缺冒号 fail-closed；`updated`-only 文档 → `EmptyUpdate`。
- 预算改为 **provider tokenizer**（`tokenizer.rs`，`deepseek-ai/DeepSeek-V4.1-Flash/tokenizer.json` 嵌入资源，vocab 128000），不再用 `chars/2`；`estimated_tokens() -> Result<u64, StateError>`。
- `apply -> Result<bool>`：相同内容 no-op 返回 `false`，**不刷新 `updated`**；`blackboard.rs` 的 `write_state/write_state_document/replace_state` 只在 changed 时 bump revision。
- `tool_run.rs`：no-op 不落 `StateWrite`，ToolCompleted 带 `changed=false`；空 state 落 `outcome=cleared`（无 `updated`、0 token），非空 accepted 才写 checkpoint。
- 换行/回车与 items 分号拒绝，避免字段内容伪造多行 state 文档。

### 1.2 context_map（item 2）

- `model_face.rs`：`ContextMapRender{text,total_blocks,rendered_blocks,token_estimate}`；只列滑块外已闭合块，块号/轮次/token/状态 T/C/X/B/L/R/台账 seq/pointer/回放路径；header/marker/legend 全部计入预算；超限显式 `+N blocks`。
- provider tokenizer 硬预算 ≤1024；测试补 `B` 状态断言、实际交付行数 < 总块数、provider 计数。
- 只读、不常驻、不写会话；夹带 `summary/whitelist/handover/keep_recent_rounds` 整调用 reject。

### 1.3 recenter（item 3）

- `recenter.rs`：`RecenterRender{text,available_rounds,rendered_rounds,token_estimate}`；输出 `[recenter v1]` + state + 最近 5 个完整工具轮摘要 + 显式 `next`。
- provider tokenizer ≤1024；先保留 header/state/next，再逐条加 recent；超预算截 recent 并标 `+N rounds`；**`next` 绝不截断**；base state+next 超预算 fail-closed。
- 560 个不同 CJK goal 的截断回归：`rendered < available` 且 `next` 保留。
- 只读、无副作用、不写会话/marker、不置压缩窗口、不落 `ContextCompressed`；active decisions 留 M1.5。

### 1.4 capability registry（item 4）

- 新建 `orz-loop/src/capability.rs`：
  - 白名单 = `tool_probe::WORK_TOOLS` + `ToolDispatcher::READ_ONLY_EXEMPT_TOOLS` + supplemental（`web_search/web_fetch/bash/submit/project_doc_index/browser_read/browser_control/spawn_scout/self_check`）；
  - rights 由既有 `RiskClass` 机械映射（read/write/execute/network）；
  - session preopen root 记录在决策中；
  - 未声明工具 deny，稳定 reason `capability_not_declared`。
- `tool_run.rs` 在 permission/ticket 之前发 `CapabilityUse` journal 行；deny 时写 `ToolCompleted{error:"capability_denied"}`、**不写 ToolStarted、无副作用**；payload 不含 arguments/secret。
- 新增 `EventType::CapabilityUse`、v0.2 enum、`runtime/capability-use-event-payload-v0.2.schema.json`、registry v02=47、fixtures、generator、TUI Unknown。

### 1.5 checkpoint producer（item 5）

- `EventType::ContextPulled` 由 map/recenter 成功分支写审计行：mode/read_only/token_estimate/blocks/total_blocks/omitted_blocks 或 recent_rounds/total_recent_rounds/omitted_rounds/output_sha256/output_chars。
- `EventType::CheckpointCreated` 不再 schema-only：新建 `orz-loop/src/system_checkpoint.rs`，state 写 accepted 且非空时原子写 `<session>/.gsa/checkpoints/CKPT-<gen>-<sha12>.json`，发 id/generation/state_sha256/path；空 state 跳过；checkpoint 失败不回滚已接受的 StateWrite。
- 新增 `runtime/checkpoint-created-event-payload-v0.2.schema.json`（描述已改为有 producer）、registry/fixtures/generator；M4 的完整 checkpoint/recovery 服务仍留后。

### 1.6 resident 清理（item 6）

- `agent_loop.rs` 删除 `[permission]` 系统提示追加；审批事实仍由工具结果/宿主 UI 承担。
- 删除 `<plan_framework>` 常驻前置；`plan_write` 自身工具描述成为按需指针。
- 删除 `SELF_CHECK_RESIDENT_LINE` 常驻软提示；`self_check` 仍可由模型按需调用。
- 删除 `context_scale::context_permission_line()` 及其在六类压缩/截断提醒块中的引用——context-scale 不再常驻“权限宣讲”，压缩工具能力由 `context_manage` 工具描述与按需 pull 承担。
- `planning.rs` 测试改为断言系统提示**不再**包含 `<plan_framework>`。

## 2. 测试证据

| 面 | 结果 |
|---|---|
| orz-loop | **969 passed / 0 failed / 3 ignored** |
| orz-assurance | **311 passed / 0 failed**；fixtures 9/4/9；doctest 1 |
| orz-host（串行，`RUST_MIN_STACK=16MB`） | **453 passed / 0 failed / 6 ignored** |
| orz-tui bridge | **8 passed / 0 failed** |
| runtime conformance | **17 passed** |
| Python journal/conformance 定向 | **40 passed** |
| Python 全套 | 1684 passed / 1 failed（仅 `doctor --json` 全仓检查因未提交/未重建预期失败；非本批代码回归） |
| `cargo fmt --all -- --check` | 全绿 |
| `cargo check --workspace` | `Finished` |

新增回归钉：

- capability allow/deny/secret 单测 + 未声明工具 e2e（无 ToolStarted、有 CapabilityUse/ToolCompleted capability_denied）；
- state no-op + 空态 cleared e2e（2 条 StateWrite、1 条 CheckpointCreated、changed 序列 true/false/true）；
- context_map B 状态/实际交付块数；recenter next 不截断 + CJK 预算回归；
- context_manage 参数白名单/fail-closed；plan framework 不再常驻。

## 3. 已知边界

1. provider tokenizer 资源 6.37MB，构建产物增大；这是“真实 provider 计数”的代价，换来 state/map/recenter 硬帽不再被 CJK 低估。V4.1-Flash 与 V3 的 tokenizer.json 不是同一文件（6.37MB vs 7.85MB，vocab 128000 vs 129280），本批已改用实际在役模型 `deepseek-flash`（DeepSeek-V4.1-Flash）的 tokenizer。
2. capability 白名单是**手工维护**的机械表；新增工具必须补名，否则 deny 是安全网。preopen 目前记录 root 标签，路径级执行仍由既有 permission/snapshot 门承担。
3. checkpoint producer 只覆盖 state 写路径；M4 的 plan/session 全量 checkpoint 与恢复服务未在本批实现。
4. `context_scale` 的高水位提醒仍含压缩工具指针（`summary_block_guide`），但不再含 `context_permission_line` 权限宣讲；它是动态提醒、不是系统提示常驻文本。
5. tokenizer 与在役模型绑定为 `deepseek-flash`（DeepSeek-V4.1-Flash）；切到其他 SKU/版本时必须同步替换 tokenizer 资产或增加模型-tokenizer 绑定检查，否则估算会漂移。
6. 未提交态下 `doctor --json` / `check_repository` 会报未跟踪文件/digest/manifest 差异，提交后应重跑确认。

## 4. 关键文件

- `orz/crates/orz-loop/src/{tokenizer.rs,state.rs,blackboard.rs,model_face.rs,recenter.rs,capability.rs,system_checkpoint.rs,controller.rs,agent_loop.rs,host_exec/tool_run.rs,lib.rs,planning.rs}`
- `orz/crates/orz-loop/assets/deepseek-v4.1-flash-tokenizer.json`
- `orz/crates/orz-assurance/src/journal/{event.rs,conformance.rs}`
- `orz/crates/orz-tui/src/bridge.rs`
- `runtime/{run-event-v0.2.schema.json,run-event-payload-registry-v0.1.json,context-pulled-event-payload-v0.2.schema.json,checkpoint-created-event-payload-v0.2.schema.json,capability-use-event-payload-v0.2.schema.json,fixtures/run-event-v0.2/**}`
- `scripts/generate_run_event_fixtures.py`
- `assurance/run_event_journal_validation.py`
- 前序批档：366/367/370/371/373/374/375。

## 5. 放行建议

M1 v1 六项已具备代码、schema、审计与测试证据。建议先由用户裁决是否提交/重建；**本批未提交、未重建、未换装**。提交前应重跑 `doctor`/`check_repository` 清点未提交态预期差异，提交后再做一次全量门禁。

剩余 P2/P3、M1.5/M4 边界与 tokenizer 纠正记录见 [`377 挂账`](377_M1_V1_OPEN_ITEMS_2026-10-28.md)。
