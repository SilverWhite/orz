# GAP-INQUIRY-SPLIT 实施审计（2026-08-10）

- 范围：ADR-0010 §4.1/§4.2/§4.3/§5.1/§5.3 的机制拆分（Phase C 前置序·轨道 A 第一切片）
- 用户裁决：本次只做 GAP-INQUIRY-SPLIT；Diagnostic Coverage 不纳入；子代理同构（GAP-SUBAGENT-RUNTIME）下一窗口
- 迁移纪律（§5.3）：Schema/fixture 先行（Phase B 已完成）→ producer 与 verifier 同切片 → 旧版本仅 replay → 删除锁定旧逻辑的测试

## 1. 已删除

- `orz/crates/orz-loop/src/inquiry.rs`（四计数器 + `TriggerReason` + `parse_completion_decision` 自由文本解析）与 `lib.rs` 导出
- `INFO_SUFFICIENCY_BLOCK` / `RETRIEVAL_COMPLETION_CHECK_BLOCK`（prompt.rs）——模型自由文本自报问询（违反 §4.3）
- controller `maybe_fire_neutral_inquiry`（三实例混合触发 + 全清零冷却）
- 每轮 `orientation_checkpoint` 事件（`OrientationMonitor` interval=1，block 从不注入对话）
- `feed_output_repeats` 双重消费（stagnation metrics 同一度量两个消费者；R-8 回归点由 `is_injected_block_text` 过滤测试承担）
- `feed_tool_call` / `feed_semantic_action`（工具级计数——§4.2 明确不作为 Orientation 判定点）

## 2. 已实现

### Orientation producer（§4.2）—— `orz-loop/src/orientation.rs` 重写

- `OrientationSessionState`（serde）：`main/internal/external` 三车道独立计数 + `threshold=7` + `session_id` + fire `sequence`
- 计数单位 = 完成的逻辑模型轮（controller `run_turn_inner` 模型轮完成点喂数；deny/gate-answer/tool 轮各计 1；gateway 重试不经过该点不计；一轮多工具调用不拆分）
- 会话级持久化：`StoredSession.orientation` + 侧车 `{cwd}/.gsa/orientation/<session8>.json`（进程重启恢复后续计；grill/一次性 CLI 传 `None` 跳过）
- 双注入点：post-tool-batch gap（工具轮后）为主 + loop-top gap（final/deny 轮 crossing 跨 turn 携带 + 恢复续计点）；fire 时重置（§4.2 只有实际发出才重置）
- v0.2 `orientation_checkpoint` 事件：10 字段 payload（`inquiry_family=neutral` + `inquiry_kind` const + `agent_role` + `trigger=completed_turns_interval` + `completed_turns_since_orientation` + `message_block` + `injection_position`）
- `ORIENTATION_BLOCK` 换 v0.2 标记（`[ORIENTATION v0.2]`），`is_injected_block_text` 前缀注册（注入块不进停滞测量）

### Information Sufficiency producer（§4.3）—— 机械 assessment

- `run_retrieval_subagent` 成功路径以 `information_sufficiency_assessment` 替换 `RetrievalCompletionCheck`
- 完全机械：ledger 快照计数（blackboard 单写者分区）、`result_digest`/`ledger_digest` 实算 sha256、`status=indeterminate`（无机械覆盖要求——§4.3 明确规定该情形不得由模型代填）、`source_visibility_gate=not_applicable`
- 可见性归属：scripted 子代理的 `[DOC]`/`[SOURCE]` 行是声明级引用 → `metadata_only = total`
- **close_record 本切片不产**：schema allOf 要求 `normal_close` 必填 `validated_disposition_id`，disposition 生产者需主 Agent 结构化决策（留子代理切片）；verifier 已接受孤 assessment

### Stagnation 去重

- 输出重复唯一入口 = turn 末尾 `evaluate_runtime_stagnation_guard`（§4.5 内容停滞）

### 事件迁移（§5.3/§11.6.2）—— 整轨翻 v0.2

- `EventTrack{V01,V02}`（orz-assurance）：`payload_schema_id` + `schema_version`；生产构造点全部 `V02`（同质哈希链）；**V02 轨写退役类型（NeutralInquiry/RetrievalCompletionCheck）= panic**（镜像 verifier negative fixture）
- `EventType` +4：`DiagnosticCoverageCheckpoint` / `InformationSufficiencyAssessment` / `RetrievalParentDisposition` / `RetrievalCloseRecord`（对齐 v0.2 enum 36 项；退役类型保留 replay）
- `RunEvent::new_v01`/`new_v02` 构造器；`RunEvent::new` 私有化（schema_version 参数）
- session.rs 预检、orz-bin `record_plan_event`、acp_server `RunRecorder` 全部翻 v0.2
- **manifest 内 `schema_version` 保持 `"0.1.0-draft"`**（payload 字段，v0.1 payload schema const；Python canonical_cli parity）——只有信封顶层翻 `0.2.0-draft`
- `replay_journal`（Rust verifier）零改动：只做链校验 + EventType 强类型解析

## 3. 验证

- Rust：orz-assurance 95 / orz-loop 117 / orz-host 98 / orz-tui 177 / orz-bin 9（含 stdio_e2e）全绿；conformance capture **7/7**（新增 `orientation-fire-run`：7 轮检索 → 第 7 轮后恰一次 orientation fire + 7 个机械 assessment）
- Python：`runtime/tests` 151 passed（新增 V02JournalConformanceTests 8 项：同质 v0.2 轨、退役类型缺席、序列 staleness、orientation payload 形状、恰一次 fire、assessment 机械性、§4.4 链规则）
- `check_repository.py` 0 errors（新增 v0.2 journals 精确集合 + 同质轨硬断言）
- v0.1 fixtures / `EXPECTED_SEQUENCES`（v0.1）字节未动（历史冻结，git diff 验证）

## 4. 边界（明确未做，登记给后续切片）

- Diagnostic Coverage Check 的 Rust producer（仅 EventType + schema 就位，`diagnostic_coverage_checkpoint` 事件未产）
- 子代理同构 runtime（internal/external 车道结构保留、零投喂；orientation 三车道独立性由状态机结构保证，投喂在 GAP-SUBAGENT-RUNTIME 接）
- `retrieval_parent_disposition` / `retrieval_close_record` producer（schema/verifier/fixtures 已覆盖；verifier 接受孤 assessment）
- pre_handoff 触发变体（`CheckpointTrigger::PreHandoff` 保留未接线；当前循环无 handoff 路径）
- v0.1 轨任何改动（replay-only 历史冻结）

## 5. 三代理审查闭环（2026-08-10）

三独立代理审查（设计合理性 / 实现合理性 / 符合性）——**无 P1/D1/C1 级设计背离**。修复批与登记如下：

**修复（实现）**：
- **P1-1**：acp_server `session.orientation.take()` 推迟到 restore-in-flight 检查/bootstrap/build_host 全部成功之后（早期错误路径不再静默丢计数）；惰性实例化兜底到 sidecar 而非 0
- **P2-1**：侧车读写失败加 `tracing::warn!`（区分 NotFound 与损坏，同 grill JSONL 模式）
- **P2-2**：`fire()` 拆 `build_fire_record`（无副作用）+ `commit_fire`（journal 写入与注入成功后提交重置）——写失败不再持久化"已重置但未 fire"的计数
- **P2-3**：assessment_id 掺入 call_id digest 分量（同输出不同 activation 不再撞 id → 不再触发 verifier conflict 判定）
- **P3-2**：删除 `let _ = target;` 残留
- **P3-6**：EventWriter V02 退役类型守卫从 panic 改为 `AgentLoopError` 错误返回（未来 producer 失误只失败单 run 不炸进程）
- **D2-1/D3-1**：post-tool-batch 与 loop-top 注入点注释修正（预算耗尽轮"同 turn 最终轮消费"；deny 轮有 tool batch 其 crossing 走 post-tool-batch）
- **C3-1**：orz-bin `record_guard_terminal` 注释 v0.1 → v0.2
- **C2-1/C2-2**：索引 IMPL-RUN-EVENT-SCHEMA / GAP-SUFFICIENCY-SCHEMA 条目体更新（producer 已迁移+捕获事实；partial 缘由改为 disposition/close 未产）

**登记（设计/记录项）**：
- **D2-2**：assessment 的 result/ledger digest 输入（子代理输出、blackboard ledger）本切片不进 journal——digest 是 producer 侧确定性机械事实，但外部 verifier 只能格式校验不能重建；重建性随 GAP-SUBAGENT-RUNTIME（子代理 journal 携带 ledger）自然闭合
- **P3-1**：counterexample gate 候选草稿轮计入已完成代（§4.2 单位="一次 assistant generation"，候选即已完成代）——模块文档已注明，fixture 钉住
- **P3-3**：checkpoint_id 序号 0 基 vs last_fire.sequence 1 基——注释说明
- **P3-5/D3-4**：侧车以 8 字符 session 前缀命名——前缀碰撞时新会话续接旧计数（§4.2"全新 session 从 0 开始"的已知边界）；session_id 无字符校验，含非 `[A-Za-z0-9._-]` 字符的 id 会产生违反 pattern 的 checkpoint_id——已知限制，后续会话创建处加 ASCII-safe 校验
- **D3-2**：`step_index: 0` 恒为占位（§5.2 轮次状态由 completed_turns_since_orientation 承载）——后续切片可从 blackboard plan 派生
- **D3-3**：activation_id/contract_id 为临时身份（call_id/工具名派生）——语义随 GAP-SUBAGENT-RUNTIME 正规化
- **D3-5**：`[ORIENTATION` 前缀过匹配（含旧 `[ORIENTATION_CHECKPOINT…]` 文本）——保守方向，测试钉住
- **D3-6**：counterexample gate 打在 orientation 回答轮（首个无工具轮即被 gate 拦截）——既有循环对任何中间无工具轮的行为，非本切片引入
- **D3-7**：pytest 全套须从仓库根目录调用（`python -m pytest runtime/tests/`）——runtime cwd 下 `scripts.*` 导入失败（pre-existing 环境事实）
- **D3-8**：EventWriter V01 分支不可达（生产构造点全 V02）——表达性保留；V02 禁写已由错误返回 + Python negative fixtures 覆盖
- **D3-9**：grill 车道零计数——与 counterexample skip 同前例（run 语义之外），计划 D5 已明示
- **D3-10**：fire 与 budget 耗尽同轮时，最终轮同时回答 orientation 与 exhaustion 两个指令（接受：run 即将结束）

## 6. 关键文件

- `orz/crates/orz-loop/src/orientation.rs`（重写）、`controller.rs`（改线）、`prompt.rs`、`agents/retrieval.rs`
- `orz/crates/orz-assurance/src/journal/event.rs`（EventTrack/EventType/RunEvent）、`orientation/checkpoint.rs`（ORIENTATION_BLOCK v0.2）
- `orz/crates/orz-host/src/session.rs`、`acp_server.rs`（会话持久化 + 侧车）
- `orz/crates/orz-bin/src/main.rs`（生产构造 + 7 个 capture 场景）
- `runtime/fixtures/run-event-v0.2/journals/`（7 个真实 v0.2 journals）
- `CLI_PROJECT_INDEX.md`（GAP-INQUIRY-SPLIT → implemented）
