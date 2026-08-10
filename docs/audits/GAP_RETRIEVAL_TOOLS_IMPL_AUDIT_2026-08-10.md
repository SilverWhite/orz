# GAP-RETRIEVAL-TOOLS 实施审计（2026-08-10）

- 范围：ADR-0010 §3.3.3（结构化结果五字段）、§3.7.1（检索模式 authority，V11-IMPL-001）、§3.7.4（证据记录）、§3.7.5（可见性分级）、§3.7.9（来源绑定）、§11.1（pre-handoff 独立触发）、§4.6.2（DC 两信号余项）——Phase C 前置序轨道 A 第三切片
- 用户裁决（2026-08-10）：① 工具范围=内部真实（project doc index）+ web 接线（grok_build web_fetch/web_search），local_browser 浏览器自动化留后续（枚举/transition 完整接线，capability 显式 unsupported）② 结构化结果五字段全量 v0.2 schema ③ DC 两信号 + pre_handoff 一并纳入 ④ 结果验证失败=显式降级回退（非硬失败）⑤ conversation 不入侧车 ⑥ web 本切片接真实 client（env key）
- 迁移纪律（§5.3）：Schema/fixture/verifier 先行 → producer → capture 重放 → 全量验证

## 1. 已变更（核心）

### 1.1 Schema/fixtures/verifier（S1，先行）

- `runtime/run-event-v0.2.schema.json`：enum 36→39（+`retrieval_mode_transition`、`retrieval_result_committed`、`retrieval_activation_restored`）
- 新 payload schema ×3（`runtime/`）：
  - `retrieval-mode-transition-event-payload-v0.2.schema.json`：transition_id/session_id/old_mode/new_mode/authority(user|parent_task_contract|session_bootstrap)/reason_code(explicit_selection|session_default|capability_probe)/可选 capability_status(available|unsupported|degraded)；allOf：new_mode=off → capability_status 必须 null，非 off → 必须合法 string
  - `retrieval-result-event-payload-v0.2.schema.json`：五字段（query_summary/source_ledger/filtering_log/organized_response/raw_source_refs）迁移自 `assurance/retrieval-result-v0.1.schema.json`（replay-only 迁移输入，ADR §3.3 末）+ ADR §3.7.4 补字段（source_type/accessed_at/observed_scope/missing_scope/highest_allowed_claim/limitation）+ source_counts（与 assessment payload 同形状）+ visibility_degraded；source_entry 的 content_sha256 可选、ref_entry 的 content_sha256 允许 null（声明行无内容可哈希）
  - `retrieval-activation-restored-event-payload-v0.2.schema.json`：restore_id/activation_id/subagent_session_id/contract_id/contract_revision/status(active|awaiting_disposition)/origin_run_id/sidecar_ref/tool_rounds_used；allOf：awaiting_disposition → assessment_id 必填
- `assurance/run_event_journal_validation.py`：V02 注册表 +3；新规则 ×3——
  - `_verify_v02_retrieval_mode`：off 后无检索派发（tool_started 带 target）/无 assessment/无 result_committed（disposition/close 处置既有 pending 合法）；local_browser 后检索 tool_completed 必须 error；transition old_mode 链一致；bootstrap 至多一次
  - `_verify_v02_result_consistency`：source_counts ≡ ledger 可见性分布；claim×visibility 矩阵（§3.7.5）；同 (activation, revision) 的 assessment digest/source_counts 交叉一致
  - `_verify_v02_activation_restore`：同 activation 至多一次 restore；lifecycle 扩展——restore 声明合法化跨 run disposition 引用（assessment 不在本 journal 时经声明解析，CAS 用 restore 的 contract_revision）+ close 绑定经声明比对 contract_id/result_digest
- `scripts/generate_run_event_fixtures.py`：V02 事件列表 +3、good/bad payload 形状、README 同步；`scripts/check_repository.py`：显式路径清单 +3、journals 精确集合 +5
- 测试：mode 规则正/负例 10 个、result-consistency 7 个、restore 6 个（合成 journals）+ conformance 计数 36→39

### 1.2 mode 状态机（S2）

- `orz-assurance/src/journal/event.rs`：EventType +3 变体（非 terminal）
- `orz-loop/src/controller.rs`：`RetrievalMode{Off,LocalBrowser,FrameworkFallback}`（serde snake_case + from_wire）+ `RetrievalCapability{Available,Unsupported(String),Degraded(String)}`；controller 字段 retrieval_mode/retrieval_capability/bootstrap_transition_pending(AtomicBool)/session_id；builder `with_retrieval_mode`；`maybe_journal_mode_transition`（run 启动序列：transition 在 tool_availability_check 前）；mode=off 投影（`relay::is_retrieval_dispatch_name` 从 tool_defs 移除检索族；`retrieval_disposition` 保留——处置既有/restored pending 是合法 off 动作）；dispatch 门禁：off → `retrieval_mode_off`（**无 ToolStarted**——verifier mode 规则禁 off 后检索派发；拒绝=ToolCompleted(error) 单独）+ local_browser unsupported → `retrieval_capability_unavailable`（标准 ToolStarted→ToolCompleted(error)）
- `orz-host/src/acp_server.rs`：`StoredActivationSnapshot`（schema_version/session_id/retrieval_mode/bootstrap_transition_pending/next_seq/activations）+ 侧车三函数（activation_sidecar_path/load/persist——orientation 纪律：best-effort、损坏 warn、take-out 在全部 fallible 步骤后）；`handle_session_new_with_options`（retrieval_mode: Option<RetrievalMode>，显式选择≠持久 mode → transition_pending）；`handle_session_prompt` take-out/write-back + `probe_retrieval_capability`
- `orz-host/src/stdio.rs`：StdioAgentHandler.with_retrieval_mode + run_stdio_server 参数；`orz-bin`：`--retrieval-mode` flag → `ORZ_RETRIEVAL_MODE` env
- TUI：bridge/events/projection 三处 +3 事件投影

### 1.3 结构化结果 + project_doc_index（S3）

- `orz-loop/src/agents/retrieval.rs`：`parse_retrieval_result_json`（`[RESULT_JSON]{...}[/RESULT_JSON]` 块，拒绝宽松模式）；`parse_retrieval_text` 保留作行协议回退
- `orz-loop/src/prompt.rs`：citation rule 迁移 source_id 为主（§3.7.9 废止 `[来源: 路径:行号]` grep 验证含义）；delivery contract 增 `[RESULT_JSON]` 说明（sections/claims 结构、claim_strength 上限）
- `orz-loop/src/controller.rs`：`EvidenceRecord`（tool/identity/title/source_type/visibility/content_sha256/observed_scope/missing_scope/accessed_at）+ `build_evidence_record`（§3.7.5 可见性表：read_file=full、web_fetch=full/截断 partial、web_search=partial、project_doc_index 按 include_content；失败调用无证据）；SharedLoopServices.evidence 钩子（检索 lane 收集）；`build_structured_result`（机械 ledger：evidence 优先 + [DOC]/[SOURCE] 声明行 metadata 补充；query_summary 机械单条；organized_response 模型块校验——source_ids ⊆ ledger + claim×visibility 矩阵，失败 → 空 + visibility_degraded + validation_note；raw_source_refs 投影；source_counts 机械分布；result_digest/ledger_digest）；`retrieval_result_committed` 事件 → artifact 落盘（`{journal_dir}/retrieval-results/{activation_id}-r{rev}-{digest8}.json` best-effort）→ `ActivationState.result_archive_ref`；assessment source_counts 真实化（full_text_observed 非零）+ reason_codes 增 structured_result_validation_failed；`write_close_record` archive_ref 用真实 artifact（terminal close 无结果保持 `run-journal:{run_id}`）
- `orz-host/src/project_doc_index.rs`（新模块）：discovery（排除 .git/.gsa/target/node_modules/.venv/存档/archive/dist/build/.hidden + 文档扩展白名单）+ keyword 匹配（path/title/headings）+ include_content/max_results/max_content_bytes（截断 → partial 标记）；ToolsetRegistry.get/list 声明；call_tool 特判；orz-loop tool.rs risk_class=ReadOnly + action_category=retrieval；orz-host permission.rs access_kind=Read(None)

### 1.4 激活跨 turn 持久化（S4）

- `StoredActivationSnapshot`/`StoredActivation`（controller，serde）：next_seq + 非 Closed 激活（状态机字段 only：activation_id/parent_session_id/subagent_session_id/contract_id/contract_revision/status/tool_rounds_used/result_digest/result_archive_ref/next_goal/pending_assessment_id/pending_expected_contract_revision/origin_run_id）；conversation/submitted **不入侧车**（登记边界）
- `ActivationRegistry::snapshot_json/seed_from_json`；`with_activation_snapshot` builder；`journal_activation_restores`（run 启动序列 transition 后、availability 前——每激活一次 restore 事件）；`activation_snapshot_json`（acp_server run 后回写 sidecar）
- verifier 侧：restore 声明合法化跨 run disposition（S1）

### 1.5 web 接线（S5）

- `orz-host/src/tools.rs`：`web_search_config_from_env`（`ORZ_WEB_SEARCH_API_KEY` 启用 xAI Grok client，base_url/model 可覆盖；缺失=Disabled）+ `web_fetch_config_default`（恒 Enabled——直接 HTTP 无 key 依赖）；OrzHost.web_search_configured 字段 + 访问器
- `probe_retrieval_capability`：framework_fallback → web_search 配置 → Available / 缺失 → Degraded("web_search_not_configured")（显式，不静默）

### 1.6 DC 两信号 + pre_handoff（S6）

- `diagnostic_coverage.rs`：`evidence_ids`/`edited_modules`/`module_evidence` 字段；read_file 成功登记 evidence；`key_surface_unexamined`（失败输出提取 `File "..."`/`FAILED path::` 引用 ∩ 未读未编辑 → 信号，每次失败 call 至多一个）；`same_module_no_evidence`（同模块重复编辑 ∧ 模块无 evidence）；`maybe_consume_dc_retrieval_evidence`（controller Ok 分支——检索 commit 的 source_id 入 evidence 集）
- controller：stagnation 非 Continue 终态前 `orientation_checkpoint{trigger:"pre_handoff", injection_position:"pre_terminal"}`（**不注入 block、不 commit_fire**——§11.1 独立生命周期触发不参与 7 轮计数；completed 取 loop 前快照——orientation 引用被 loop 消费，登记）；orientation.rs `completed_rounds(role)` 访问器

## 2. 已删除

- （无删除——本切片为增量）

## 3. 决策登记

| ID | 决策 | 依据 |
|---|---|---|
| D-1 | 结构化结果=per-iteration（绑定 revision），blackboard section 仍跨 dispatch 累计 | §3.3.3 result 是迭代产物；write_section 语义不变 |
| D-2 | mode=off 拒绝无 ToolStarted（ToolCompleted(error) 单独） | verifier mode 规则禁 off 后检索派发（tool_started 带 target） |
| D-3 | mode=off 保留 retrieval_disposition 声明 | 处置既有/restored pending 是合法 off 动作 |
| D-4 | [DOC]/[SOURCE] 声明行=metadata-grade ledger 条目（evidence 优先合并） | §3.7.5 未读全文不得全文级归因；无工具调用场景合法 |
| D-5 | [RESULT_JSON] 验证失败 → 显式降级（空 organized_response + visibility_degraded + reason_code），非硬失败 | 用户裁决；部分结果不丢；§3.7.2 不违反（显式记录非静默） |
| D-6 | conversation/submitted 不入侧车 | 用户裁决；journal 是证据；跨 run 幂等重放限进程内（call_id 派生 disposition_id 天然防冲突）。**部分撤销（GAP-CONVERSATION-RESTORE 2026-08-10）**：conversation 改随侧车恢复（`StoredActivation.conversation`，serde default 兼容旧侧车）；`submitted` 仍不入 |
| D-7 | web_search key=env（ORZ_WEB_SEARCH_API_KEY）显式通道 | ADR-0006 凭据注册表完整接线留后续切片（seam 保留） |
| D-8 | 无 key 时 framework_fallback=Degraded("web_search_not_configured")（web_fetch 恒可用） | 显式能力记录，非 Unsupported 非静默 |
| D-9 | pre_handoff completed_turns 取 loop 前快照 | orientation 引用被 run_agent_loop 消费；审计意图是"handoff 前会话进度" |
| D-10 | web_search=1 并发语义不在本切片实现（主/子派发串行，并发窗口极小） | §3.7.7 工具合同；client 内部有速率限制；登记边界 |
| D-11 | session_id 空时 payload 用 run_id 兜底（transition/pre_handoff） | schema minLength 1；裸 controller（测试/capture）无 session |
| D-12 | pre_handoff message_block 允许空串（schema 放宽 minLength） | pre_handoff 是审计-only 不注入 block |

## 4. 边界（明确未做，登记给后续切片）

- local_browser 浏览器自动化（CDP）——枚举/transition/capability 完整接线，mode=local_browser 显式 unsupported（`local_browser_automation_not_implemented`）
- ~~conversation 跨 prompt 恢复（侧车只存状态机字段；跨 run continue 子代理上下文从头开始）~~——**已由 GAP-CONVERSATION-RESTORE 闭合**（2026-08-10，见 [GAP_CONVERSATION_RESTORE_IMPL_AUDIT](GAP_CONVERSATION_RESTORE_IMPL_AUDIT_2026-08-10.md)）
- ADR-0006 凭据注册表 → web_search key 的完整注入（env 通道先行）
- web_search=1 全局 semaphore（§3.7.7 工具合同项）
- project_doc_index 索引缓存/增量扫描（每 query 全扫，正确性优先）
- v0.1 轨任何改动（replay-only 冻结）

## 5. 验证

- `cargo test -p orz-loop`：**158 passed / 3 ignored**（S1 151 → S4 +2 → S6 +4 → 审查修复批 +1（`explicit_change_to_off_journals_real_old_mode`））
- `cargo test -p orz-host`：**103 passed**（+3 project_doc_index 单测 +2 web config）
- `cargo test -p orz-assurance -p orz-tui -p orz-bin`：95 / 177 / 6+12 ignored 全绿
- conformance capture 12/12（7 旧场景重捕：6 字节同（仅 per-run 时间戳）+ orientation-fire 增 result_committed×5；5 新场景：mode-off-refusal / local-browser-capability / real-doc-retrieval / cross-prompt-restore / pre-handoff-checkpoint）。**2026-08-10 审查修复批后全量重捕**：off 模式场景（mode-off-refusal / pre-handoff-checkpoint / plain-run / cancelled-run 等）的 `available` 列表移除 `project_doc_index`（H1 投影），其余事件序列不变
- Python：`test_run_event_journal_validation.py` 81 passed（+23 新规则测试 +1 审查修复批 D1 测试）、`test_run_event_conformance.py` 14 passed（36→39）、assurance 全套 1781 passed / 13 skipped
- `check_repository.py` valid（schemas 235、v0.2 journals 12 精确集合）
- clippy：变更文件零新增（pre-existing 仅 orz-config 1 / xai-fast-worktree 2 / agents/main.rs too_many_arguments；顺手修 GAP-SUBAGENT-RUNTIME 遗留 2 条：controller 测试 unused e、orz-bin capture filter→find）
- git diff --check：无错误（fixtures README CRLF 提示为既有属性）

## 6. 审查修复批（2026-08-10 三面审查闭环回填）

三面审查（设计/实现/符合性）发现并修复：

- **H1（high）`project_doc_index` 绕过 mode=off 门禁**——修复：`relay::is_retrieval_mode_gated_host_tool`（host 路由的检索工具家族）+ off 投影移除 + `run_host_tool` dispatch 门禁（无 ToolStarted 拒绝，`retrieval_mode_off`）+ available 断言扩展 + journals 全量重捕。
- **H2（high）web_fetch 截断误标 `full_text_observed`**——修复：截断判定匹配真实 footer（`[web_fetch content truncated`，codegen overflow.rs）+ 预算兜底 marker + 长度 backstop；测试改用真实 footer 样例。
- **M3（medium）session_id 字节切片 panic**——修复：`controller.rs` 侧车 ref 派生改 `chars().take(8)`（与 acp_server 一致）。
- **M4（medium）transition `old_mode` 失真 + 显式切 off 不 journal**——修复：sidecar 新增 `previous_retrieval_mode`（持久旧值，journal 后清）；`pending=true` 覆盖含显式 off；off 时 `capability_status=null`（schema allOf）；新增回归测试。
- **R5（文档）fixtures README 与事实矛盾**——修复：`journals/ is intentionally empty` 段恢复为 12 个真实 journals 表；"five v0.2-payload events" → eight；Producer/consumer/verifier 段补三个新机械事件。
- **D1（设计闭环）`highest_allowed_claim` 死契约**——修复：verifier `_verify_v02_result_consistency` 新增值与 visibility 的机械一致性校验（full→observed / partial→derived / metadata→synthesized / unavailable→none）+ 反例测试。
- **D2（设计闭环）§3.7.4 必填 scope**——修复：schema `source_entry.required` 补 `observed_scope`/`missing_scope`（producer 已写，真实 journals 已含）；`limitation` 保持可选并登记：producer 当前不生成，后续切片补。
- **D3（登记）restore `status` 两值压缩**——`result_ready`/`assessing` 等状态映射为 `active`（controller.rs 状态映射），属刻意压缩，verifier 只消费 `awaiting_disposition` 分支；登记不修。
- **L5-L8（low）登记不修**：`action_category("project_doc_index")=="read"`（分类标记，无安全影响）；session8 派生路径未消毒（stdio UUID 通道免疫，ACP 自定义 session 可构造——后续统一字符白名单）；侧车写非原子（崩溃丢激活状态，低概率）；web_fetch 全文 hash 进审计日志（设计内，预期知悉）。
