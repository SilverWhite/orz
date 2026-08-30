# AgentLoopController 拆分设计（2026-08-29）

> 性质：设计初稿（机械性重构，行为不变）。背景：2026-08-29 盘点确认
> `orz-loop/src/controller.rs` 为 **29,091 行 / 1.24 MB / 358 个函数**的
> 单文件，`AgentLoopController` 一个 struct 承载 40+ 字段与横跨检索/
> 证据/压缩/ACAF/黑板/console/交付的职责；同日用户裁决：先落拆分设计，
> 并评估检索模块在评测（docker）环境走本地浏览器调用的可行性。
> 权威关系：本文是重构设计草案；拆分前后行为必须逐字节一致（事件序列
> 与 journal 哈希链不变），测试面守护；不产生新设计语义。

## 1. 目标与约束

### 1.1 目标

1. 把 controller.rs 从 29k 行降到一个可维护的核心（预计 ≤10k 行）。
2. 让检索子代理调度（本次 493s 时间放大链的关键路径）独立成模块，
   为后续在子代理入口加 run 级预算/超时策略提供清晰边界。
3. 拆分全程**行为不变**：模型可见面、工具面、事件序列、journal 哈希链、
   权限/ACAF 链、测试全部保持现状。

### 1.2 约束

- **纯机械拆分**：只搬代码、改可见性（`pub(crate)`）、拆 impl 块；
  不重构字段、不改签名语义、不合并/删除机制。
- **单步可验证**：每个批次拆完跑全量测试（orz-loop / orz-host /
  orz-tools / orz-bin / pytest 事件链），绿了再进下一批。
- **不借机加机制**：拆分与编排层增强（预算/超时）是两个工作项；
  本文只负责拆分，编排增强单独设计（见 §7 后续项）。

## 2. 现状事实（源码盘点）

### 2.1 文件规模

| 项 | 值 |
|---|---|
| 总行数 | 29,091 |
| 字节 | 1,241,929 |
| 方法/函数 | 358 |
| `AgentLoopController` 字段 | 40+ |
| 第一个 `impl AgentLoopController` | 1524–1575（+ 11019 附近 Default） |
| 第二个 `impl AgentLoopController` | 2200–11018（约 8,800 行） |
| `ControllerConsoleExecutor` | 11029 起（独立 struct，已在文件内） |
| 测试 | 文件内测试占比约 60%（15000–29000 行） |

### 2.2 职责分布（自然边界）

| 职责簇 | 关键方法（行号） | 依赖字段 |
|---|---|---|
| 检索激活生命周期 | `ActivationRegistry` 1577–1870；`persist_result_artifact` 1530；`journal_activation_restores` 3610；`activation_snapshot_json` 3659；`retrieval_candidate_count` 4241；`close_activation` 7056；`close_all_activations` 7123 | activations / restored_activations / evidence / run_source_ledgers |
| 检索子代理调度 | `run_retrieval_subagent` 5613；`handle_parent_disposition` 6382 | activations / evidence / blackboard / retrieval_mode |
| 检索工具投影 | `apply_retrieval_surface_projection` 2453；`build_retrieval_task_goal` 2473；`subagent_tool_projection` 2500+ | 纯函数 + registry |
| 检索模式/证据 | `RetrievalMode` 1872；`RetrievalCapability` 1904；`with_retrieval_mode` 3516；`with_activation_snapshot` 3570；`with_retrieval_partitions` 3587 | retrieval_* / evidence / main_evidence |
| console 双模式 | `set_console_probe_source` 4154；`sync_console_registrations` 4173；`console_mode` 4232；`console_inquiry_due` 4259；`console_direct_begin/end` 4272/4295；`issue_pending_console_order` 7255；`switch_console_to_direct` 8024；`run_console_target` 8150 | console 相关 + blackboard + acaf |
| 交付/submit | `submit_confirm_message` 7881；`compute_delivery_status` 7897；`refuse_console_tool` 7935 | blackboard / delivery_pending |
| 压缩/折叠 | `with_context_compact` / `with_fold_trigger_tokens` / whitelist 族 | context_compact / whitelist |
| 主循环核心 | `run_turn_with_guards` / `run_agent_loop` 调用 / 门禁链 | 全部 |

### 2.3 拆分后现状（2026-08-30，B1–B8 完成后盘点）

`controller.rs` 29,091 → **24,861 行**（生产 7,548 / 测试 17,313；
生产约占 30%、测试约占 70%）。生产区职责簇精确构成：

| 职责簇 | 行数 | 内容 |
|---|---|---|
| host 工具执行家族 | 2,615 | `run_host_tool` / 带计划门 / 带超时（单函数 2,382 行）+ 候选计数门 |
| 主循环家族 | 988 | `run_turn` 四件套 + `run_turn_inner` |
| ACAF 票务与动作事件 | 941 | ticket_flow、acaf_action/network/command 事件、run_action_ticket |
| 控制台订单执行 | 667 | 订单发放、内容锚校验、回执、trace |
| 类型/枚举/纯函数 | 689 | AgentLoopError、模式面、denial 状态机、消息预算、compact_messages |
| checkpoint / orientation | 198 | 身份收集、方向触发、票证拒绝 |
| 检索/控制台 builder 配置面 | 174 | with_retrieval_mode、激活快照、分区等 |
| 控制台探针/模式状态 | 168 | probe source、双模式开关、direct 进出 |
| 黑板渲染 | 162 | section / session 渲染 |
| 控制台模式转换 | 120 | transition/stay/return |
| plan 摄取 | 117 | with_plan / try_with_plan / apply_structured_plan |
| 其余（构造、probe、epoch 归档、状态行、EventWriter 等） | ~570 | — |

测试区（17,313 行 / 242 个测试）以集成测试为主：黑板读取 16、
plan_first 8、压缩 7、web_fetch 7、run_tests 6、console 系 12、
检索/子代理系 25 等；均经完整 controller 跑 `run_turn` 断言事件链，
依赖 TestHost/DenyHost/假网关等整套测试脚手架。

二轮拆解推进中（N1–N4 已完成，2026-08-30）：controller.rs 24,861 →
**20,264 行**；`acaf_flow.rs` 948 行、`console_exec.rs` 1,171 行、
`host_exec.rs` 2,647 行、`compact.rs` 421 行、`denial.rs` 51 行、
`retrieval/mode.rs` 67 行。

## 3. 拆分方案

### 3.1 目标结构

```
orz-loop/src/
  controller.rs          # 主循环核心 + 门禁 + 运行面（瘦身后）
  retrieval/
    mod.rs               # 检索模块入口（re-export 到 controller）
    activation.rs        # ActivationRegistry + ActivationState + StoredActivation
    dispatch.rs          # run_retrieval_subagent + 候选计数
    disposition.rs       # handle_parent_disposition + close_activation 链
    projection.rs        # 工具投影 + task goal + 模式面（纯函数优先）
    evidence.rs          # evidence 收集 + source ledger + 加权/预筛消费
  console_exec.rs        # ControllerConsoleExecutor + 订单执行 + 失败诊断派发
  delivery.rs            # submit 两阶段 + 交付状态
  compact.rs             # ContextCompactConfig + 压缩/折叠/白名单参数面
```

### 3.2 拆解原则

1. **先纯函数、后状态方法**：`projection.rs`、`delivery.rs` 的纯函数
   部分（`submit_confirm_message`、`build_retrieval_task_goal`、
   `apply_retrieval_surface_projection`）先行，零风险。
2. **独立 struct 整体搬**：`ControllerConsoleExecutor`（11029 起）已是
   独立 struct，直接移到 `console_exec.rs`，字段可见性改 `pub(crate)`。
3. **状态方法按字段依赖分组**：检索方法只碰 `activations/evidence/
   retrieval_*/run_source_ledgers` 等字段，可整体搬入 `retrieval/`，
   `AgentLoopController` 对这些字段保留 `pub(crate)` 访问。
4. **共享字段用方法接缝**：跨模块共享的字段（blackboard / acaf /
   writer）通过已有方法或 `&self` 上的 `pub(crate)` 访问器传递，
   不引入新的全局状态。
5. **测试随代码搬**：每个模块的 `#[cfg(test)]` 随方法搬走；跨模块
   测试留在 controller.rs 主文件（用 `super::*` 或公共 API 断言）。

### 3.3 拆解批次（每批独立可验证）

| 批次 | 内容 | 预估工作量 | 验证 |
|---|---|---|---|
| B1 | `retrieval/projection.rs`（纯函数：模式投影/task goal/候选族）+ 搬对应测试 | 小 | orz-loop 全量 |
| B2 | `delivery.rs`（submit 两阶段/交付状态/拒绝信封）+ 搬对应测试 | 小 | orz-loop 全量 |
| B3 | `compact.rs`（压缩/折叠/白名单配置面 + 参数方法） | 小 | orz-loop 全量 |
| B4 | `retrieval/activation.rs` + `disposition.rs`（激活注册表/关闭链） | 中 | orz-loop + pytest 事件链 |
| B5 | `retrieval/dispatch.rs`（run_retrieval_subagent + 候选计数） | 中 | orz-loop + pytest + 单题冒烟 |
| B6 | `console_exec.rs`（ControllerConsoleExecutor + 订单执行 + direct 切换） | 中 | orz-loop + orz-tui + pytest |
| B7 | `retrieval/evidence.rs`（证据/加权/预筛消费） | 中 | orz-loop + pytest |
| B8 | controller.rs 瘦身收尾（清理 `#[allow]`、孤儿注释、文件内测试重组） | 小 | 全量回归 |

批次顺序按风险递增：B1–B3 纯函数/小状态，B4–B7 状态方法，B8 收尾。
每批独立提交，保持 `git bisect` 可定位。

B1–B8 已于 2026-08-30 全部完成并独立提交：
`855d2a8`（B1–B3 + S5-2）、`266f947`（B4）、`9bec39c`（B5）、
`d9f3dd0`（B6）、`ff5fb34`（B7）、`6c1b7dc`（B8）。每批 orz-loop
618/0/3 + pytest 1588/14 全绿（doctor 门禁在提交 + manifest 重生成后
单独复跑通过）；无行为改动。

### 3.5 二轮拆解（2026-08-30 构成盘点后，用户指示继续）

目标：把 controller.rs 压至设计验收 ≤10,000 行。构成盘点结论——
生产区只剩 7,548 行（已低于 10k），**体量瓶颈在测试区 17,313 行**；
因此二轮 = 生产簇继续归位（N1–N4）+ 测试区按主题归位（N5）。

| 批次 | 内容 | 预估减行 | 风险 |
|---|---|---|---|
| N1 | `acaf_flow.rs`（ACAF 票务/动作/网络/命令事件 + run_action_ticket，1096–2036） | ~940 | 低（簇内自洽，外部仅 3 个调用点升 pub(crate)） |
| N2 | `host_exec.rs`（host 工具执行家族：run_host_tool 三件套 + 候选门） | ~2,600 | 中高（依赖面广，run_host_tool_with_timeout 单函数 2,382 行） |
| N3 | 控制台订单执行 + 模式转换并入 `console_exec.rs` | ~790 | 中（B6 延续，同簇已在 console_exec） |
| N4 | 类型/纯函数归位（模式面回 retrieval/，消息预算/compact_messages 回 agent_loop/compact，denial 状态机独立） | ~690 | 低 |
| N5 | 测试区按主题归位各模块 `#[cfg(test)]`（集成测试脚手架随迁） | 17,300 潜力 | 中（纯机械，分批） |

每批独立提交 + 全量回归（orz-loop / orz-tui / orz-bin / orz-host /
pytest 事件链），行为不变纪律与 §3.4 相同。

N1 已闭合（`8f5e058`）：ACAF 票务/动作/网络/命令事件 + run_action_ticket
搬入 `acaf_flow.rs`（932 行），3 字段 + 3 方法升 pub(crate)，外部调用点
仅 `acaf_control_event` / `acaf_action_event` / `acaf_command_exec_event`。
N2 已闭合（`d5cbd60`）：host 工具执行家族（run_host_tool /
run_host_tool_with_plan_gate / run_host_tool_with_timeout +
candidate_gate / refuse_candidate，2,619 行）并入 `host_exec.rs`
（2,647 行）；`delivery_pending`/`blackboard_archive_dir`/
`plan_first_enabled` 字段与 `refuse_ticketed_tool`/
`maybe_note_probe_call_failure`/`apply_structured_plan`/
`render_blackboard_section`/`render_session_section`/`candidate_gate`
方法及 `candidate_tool_prefix`/`commit_candidate`/
`compose_test_output_message` 纯函数升 pub(crate)；controller.rs
23,132 → 20,511 行。
N3 已闭合（`f1cf0b0`）：控制台订单执行（发放/锚校验/回执/trace）+
模式转换（transition/stay/return）并入 `console_exec.rs`（788 行），
`console_registry`/`console_traces`/`console_order_seq`/
`console_default_enabled` 字段与 `verify_content_anchor`/
`return_console_to_console`/`record_console_receipt` 方法升 pub(crate)。
N4 已闭合（`13908c0`）：类型/纯函数归位——`RetrievalMode`/
`RetrievalCapability` → `retrieval/mode.rs`（67 行）；折叠阈值/桥预算
常量、`CompactionStats`/`estimate_message_tokens`/
`estimate_messages_tokens`/`compact_messages` → `compact.rs`（421 行）；
`DenialState`/`DenialKey`/`PolicyFeedback`/`DENIAL_BREAKER_CONSECUTIVE`
→ 独立 `denial.rs`（51 行）。controller.rs 保留兼容重导出（外部调用面
`orz_loop::controller::*` 不变），20,511 → 20,264 行。
  每批 orz-loop 618/0/3 全绿；pytest N1/N3 1588/14、N2 1589/14 全绿
  （doctor 门禁提交后复跑通过）；N4 pytest 1589/14 全绿。

> #### N5 分批设计（2026-08-30 用户指示：先设计再执行）

  N5 = 测试区（17,319 行 / 221 个测试）按主题归位各模块 `#[cfg(test)]`。
  集成测试仍经完整 controller 跑 `run_turn` 断言事件链，但按被测主题
  归入拥有对应生产代码的模块，便于后续随模块演进；行为不变纪律同前
  （事件序列与 journal 哈希链不动），每批独立提交 + 全量回归。

  **脚手架策略**：测试区顶部共享脚手架（`mod tests` 内 2946–3300：
  TestHost/DeliveryHost/DenyHost/QueueHost/EmptyRegistry、fail_result/
  ok_result、TEST_COUNTER、test_dir、MANIFEST、tool_call 等 13 个辅助
  函数、events/event_types 事件读取）先独立为
  `#[cfg(test)] pub(crate) mod controller_test_support`（新文件
  controller_test_support.rs，lib.rs 登记）；各模块测试与 controller
  主文件测试统一 `use crate::controller_test_support::*`，不复制不重复。
  簇内私有脚手架（RecordingHost/SeqHost/TimeoutOnceHost/MidRunOnceHost/
  BlockedArchiveHost/CheckpointProbeHost/FullRegistry/PolicyHost/
  SelectiveHost/R1SurfaceHost/ScriptedTestRunnerHost/TestRunnerHost/
  MixedProjectionHost/BrowserDeclaringRegistry/FlipRunnerHost/
  FailingReadHost/PolicyTestRunnerHost/FailingRunnerHost/PartialThenAbort/
  Retry*Gateway/DeferHost/SlowFirstToolHost/GatedHost/stage_c_controller/
  disposition_call/issue_guard_order/conv_message/tool_round 等）随各自
  测试块一起搬，不留在 controller。

  **留守 controller.rs（跨模块主循环测试）**：run_turn_full_gate_sequence、
  no_plan/status_line ×2、error_path/stream_abort/transport_retry ×2、
  counterexample_gate、pre_cancelled/cancel ×3、grill ×2、run_turn_
  conversation ×3/recovery/seed_from_json——预计 ~1,600 行；controller
  最终 ≈ 生产 2,944 + 留守测试 ≈ 4.5–5k 行，达成 ≤10,000 验收。

  | 批次 | 内容（测试簇） | 目标 | 预估减行 |
  |---|---|---|---|
  | N5-0 | 共享脚手架独立为 `controller_test_support.rs` | 新文件 | ~355 |
  | N5-1 | 检索模式/回放（goal_context/policy_revision/mode_off×2/bootstrap/mode_a_degrade/explicit_change_to_off/local_browser_unsupported/framework_fallback 9 项）→ `retrieval/mode.rs`；证据/来源（structured_result×3/web_page_evidence/source_annotations×2/used_low_quality 7 项）→ `retrieval/evidence.rs`；激活/恢复（activation_snapshot/has_live/restored_activation_journaled/stored_activation×2/restored_activation_continue 6 项）→ `retrieval/activation.rs` | retrieval/ | ~1,300 |
  | N5-2a | 检索调度/子代理/候选门：with_retrieval_partitions、retrieval_dispatch×3、retrieval_result_channel、retrieval_call_dispatches、subagent_loop/denies_mutation/refuses_nested/dispatch_fresh/auto_close×3/budget×2/lane_orientation、retrieval_lane×2、mode_off_gate、local_browser_lane、web_fetch/browser_read 候选门+计数 12 项、user_cancel、main_lane_web_search/retrieve_project_docs → `retrieval/dispatch.rs` | retrieval/ | ~2,750 |
  | N5-2b | 投影面：list_projection×3、main/subagent/internal_lane 投影、subagent_projection×2、retrieval_surface_projection、retrieval_task_goal、main_lane_projection_keeps、r1_main_surface、main_surface_hides → `retrieval/projection.rs` | retrieval/ | ~800 |
  | N5-3 | 控制台簇：console_stage_b×5、console_s4、console_s2×5、console_anchor×5、console_search_replace、r2×3、console_s2_structured_denial_sources、old_refusal_prefix、policy_denied_order、direct_surface_journal、direct_search_replace×2、retired/sealed_console_tools、direct_surface_retires、legacy_plan_last_step（24 项，stage_c_controller 随迁）→ `console_exec.rs` | console_exec.rs | ~2,200 |
  | N5-4 | host 工具/权限/run_tests/denial：denied_tool_round、benchmark_policy、readonly_projection、ip2a×3、tool_round_replays、mutation_tool、acaf_disabled×2、ipg_block、text_deltas、tool_call_round_trips、inject_budget、file_edit_records、tool_completed、run_tests×2、run_tests_removed/never/race、d9×4、run_tests_spawn_failure、compose_test_output_message、render_session_section、round_budget×2、deferred_permission、tool_timeout、mid_run_result、failed_edit_records、read_tool_records → `host_exec.rs`；policy_denial_source_as_str → `denial.rs` | host_exec.rs / denial.rs | ~3,200 |
  | N5-5 | 压缩/折叠/白名单：compact_messages×2、context_compact×6、fold×4、whitelist×4、session_end_compact、whitelist_write_sealed、conversation_writeback_retains_marker → `compact.rs`；summary_archive_write_failure → `summary.rs` | compact.rs / summary.rs | ~2,300 |
  | N5-6 | plan/epoch + 黑板上读：with_plan_status_line、plan_epoch_rotation、epoch_archive_write_failure、try_with_plan、rotate_to_plan、plan_first×11、planned_session_skips、plan_write_disabled → `planning.rs`；blackboard_read×16 → `blackboard.rs` | planning.rs / blackboard.rs | ~2,600 |
  | N5-7 | 收尾归位：submit×4 + final_answer → `delivery.rs`；dc×4 → `diagnostic_coverage.rs`；orientation×3 → `orientation.rs`；checkpoint×2 → `checkpoint.rs`；probe×3 + retrieval_lane_failure → `tool_probe.rs`；mechanical_audit → `mechanical_audit.rs`；同步 §3.5/§6/TODO/索引 | 多模块 | ~1,400 |

  每批独立提交 + 全量回归（orz-loop 618/0/3、orz-tui/orz-bin/orz-host
  --tests、pytest 事件链 1589/14）；doctor 门禁：提交 orz 后重生成
  manifest 再单独复跑 pytest。

  N5-0 已闭合（`541c99b`）：共享脚手架（2946–3300）独立为
  `controller_test_support.rs`（349 行，条目与结构体字段升 pub(crate)），
  lib.rs 登记 `#[cfg(test)] pub(crate) mod controller_test_support`，
  controller.rs 20,264 → 19,935 行；orz-loop 618/0/3 + pytest 1589/14
  全绿（manifest 1416 → 1417 条）。
  N5-1 已闭合（`74ef451`）：检索模式/回放 9 项 → `retrieval/mode.rs`、
  证据/来源 7 项 → `retrieval/evidence.rs`、激活/恢复 6 项 →
  `retrieval/activation.rs`（conv_message/tool_round 并入共享脚手架，
  `goal_digest_of` 升 pub(crate)），controller.rs 19,935 → 18,533 行；
  orz-loop 618/0/3 + pytest 1589/14 全绿。
  N5-2a 已闭合（`fb8682c`）：检索调度/子代理/候选门 34 项 →
  `retrieval/dispatch.rs`（RETRIEVAL_CHANNEL_ENV_LOCK/FullRegistry/
  ScriptedTestRunnerHost 并入共享脚手架，`parse_web_fetch_candidate_cap`/
  `parse_max_inject_tokens_per_round`/`EventWriter::new` 升 pub(crate)），
  controller.rs 18,533 → 15,798 行；orz-loop 618/0/3 + pytest 1589/14
  全绿。
  N5-2b 已闭合（`0e4f4ff`）：工具面投影 12 项 → `retrieval/projection.rs`
  （R1/MixedProjection/BrowserDeclaring 脚手架随迁，MixedProjectionRegistry
  并入共享脚手架），controller.rs 15,798 → 15,065 行；orz-loop 618/0/3 +
  pytest 1589/14 全绿。
  N5-3 已闭合（`693c260`）：控制台订单/锚/诊断 24 项 → `console_exec.rs`
  （policy_denial_source_as_str 归位 `denial.rs`，stage_c_controller 并入
  共享脚手架），controller.rs 15,065 → 12,820 行；orz-loop 618/0/3 +
  pytest 1589/14 全绿。
  N5-4 已闭合（`00f298a`）：host 工具/权限/run_tests 37 项 →
  `host_exec.rs`（RunTestsDeclaringRegistry 并入共享脚手架），
  controller.rs 12,820 → 10,039 行；orz-loop 618/0/3 + pytest 1589/14
  全绿。
  N5-5 已闭合（`e828299`）：压缩/折叠/白名单 21 项 → `compact.rs`
  （summary 存档测试 → `summary.rs`，BlockedArchiveHost 并入共享脚手架），
  controller.rs 10,039 → 8,033 行；orz-loop 618/0/3 + pytest 1589/14
  全绿。
  N5-6 已闭合（`52ce3f5`）：plan/epoch/plan_first 15 项 → `planning.rs`、
  黑板上读 16 项 → `blackboard.rs`，controller.rs 8,033 → 5,632 行；
  orz-loop 618/0/3 + pytest 1589/14 全绿。
  N5-7 已闭合（`d6f771b`）：收尾归位——submit×4 + final_answer →
  `delivery.rs`、dc×4 → `diagnostic_coverage.rs`、orientation×3 →
  `orientation.rs`、checkpoint×2 → `checkpoint.rs`、probe×4 →
  `tool_probe.rs`、mechanical_audit → `mechanical_audit.rs`（final_answer
  自 console_exec 误入纠正），controller.rs 5,632 → 4,142 行。

  **N5 全部闭合（2026-08-30）**：测试区 221 项中 202 项按主题归位各
  模块，controller.rs 仅留守 19 项跨模块主循环测试（run_turn 门禁链/
  会话恢复/取消/grill/状态行）。N5-0…N5-7 八批独立提交，每批 orz-loop
  618/0/3 + pytest 1589/14 全绿；**controller.rs 29,091 → 4,142 行**，
  达成设计验收 ≤10,000 行。

### 3.4 风险与守护

- **行为不变守护**：journal 事件序列由 `run_event_journal_validation.py`
  （236 断言级）+ conformance 捕获锁定；拆分不触碰事件 payload。
- **测试守护**：orz-loop 616 项 + orz-host 229 + orz-tools 2659 +
  orz-bin + pytest 236，每批全绿才进下一批。
- **最大风险点**：`run_retrieval_subagent`（5613）是递归调用
  （Box::pin + nested-dispatch gate），搬动时保持调用签名与
  `&mut act.conversation` 语义逐字一致；用 B5 单独一批隔离验证。
- **非目标**：本文不做字段合并、不做异步改造、不引入 trait 抽象。

## 4. 检索模块专项：docker 内本地浏览器

### 4.1 现状

- `local_browser` lane 已完整实现（orz-host/src/local_browser/）：
  CDP 客户端（`CdpBrowserSession`）、`LocalBrowserManager`（每 session
  一个浏览器进程）、浏览器发现（`ORZ_BROWSER_PATH` / PATH / Windows
  路径）、URL 门、PDF 证据管线、`browser_read` 工具（full/preview/
  keywords 三模式）。
- 已支持 `ORZ_BROWSER_HEADLESS=1` → `--headless=new`（display-less
  环境）。TB 下 probe 失败原因正是容器内无浏览器二进制：
  `retrieval_mode_transition { old: local_browser, new: framework_fallback,
  reason: browser_launch_failed }`（事件面已确认）。

### 4.2 docker 内跑浏览器的可行性：可以，两个技术前提

1. **缺 docker 必需的启动参数**：当前 `browser_launch_args` 无
   `--no-sandbox`（root 容器内 Chrome sandbox 无法初始化）与
   `--disable-dev-shm-usage`（容器默认 /dev/shm 64MB，Chrome 会崩）。
   补这两个参数 + `--disable-gpu`（容器无 GPU）即可在任意 debian/
   ubuntu 镜像内 headless 启动。
2. **缺浏览器二进制与依赖**：Chrome/Chromium 需要 libnss3、libatk、
   libgbm 等约 20 个系统库（`apt-get install -y chromium` 在 Debian
   bookworm 自动带依赖，或 `google-chrome-stable` 需手动补依赖）。

### 4.3 注入方案（推荐：agent install 层，不改 89 个 task 镜像）

TB 2.1 的 89 个 task 镜像各自独立（ubuntu:24.04 / python slim 等），
无统一 base。改 89 个 Dockerfile 侵入面大且与官方任务定义漂移。
推荐在 **orz Harbor adapter 的 `install()` 阶段**注入浏览器：

```text
orz.py install():
  现有：上传 orz/signer/provision 二进制 + ACAF provision
  新增（可选开关 ORZ_EVAL_BROWSER=1）：
    apt-get update && apt-get install -y --no-install-recommends chromium
    （或下载 chromium 静态包到 /opt/chromium + 依赖）
  环境：
    ORZ_BROWSER_PATH=/usr/bin/chromium
    ORZ_BROWSER_HEADLESS=1
```

> **2026-08-30 已实施（0k-5）**：`tb_agents/orz.py` install() 新增
> `eval_browser` 开关（`--ak eval_browser=true` / `ORZ_EVAL_BROWSER=1`，
> 缺省关闭）。apt 真实包优先（Debian bookworm → /usr/bin/chromium，实测
> Chromium 151）；apt 仅 snap 过渡的镜像（ubuntu:24.04）fallback 官方
> Chromium 快照（storage.googleapis.com/chromium-browser-snapshots →
> /opt/chrome-linux/chrome，实测 Chromium 154 headless 渲染 OK）；
> run() 显式白名单注入 ORZ_BROWSER_PATH + ORZ_BROWSER_HEADLESS=1
> （env_clear 不发继承）；所有失败路径零退出（harbor _exec 非零抛异常，
> install 不因浏览器失败 fail），无 apt 系跳过、local_browser 机械降级
> framework_fallback 不变。登记：TODO P0-0k。

- 优点：89 个任务镜像零改动；浏览器只在需要 local_browser 的评测
  启用；与 ACAF provision 同为"agent 侧环境准备"。
- 代价：chromium 包约 300–500MB（含依赖），每容器安装 30–60s；
  对 wall-clock 预算紧张的任务（900s 档）需计入 install 时间。

### 4.4 替代方案对比

| 方案 | 侵入面 | 耗时 | 一致性 | 结论 |
|---|---|---|---|---|
| A. agent install 层 apt 装 chromium（推荐） | 仅 orz.py | 30–60s/容器 | 与官方任务镜像零漂移 | 首选 |
| B. 89 个 task Dockerfile 加 chromium | 89 文件 | 镜像构建期 | 与官方镜像漂移、维护重 | 否决 |
| C. 独立 browser sidecar 容器（host 起 Chrome，容器内连 CDP） | host + adapter | 启动快 | 网络/权限边界复杂 | 备选 |
| D. 复用宿主机 Chrome（挂载 + CDP 端口映射） | adapter + host 配置 | 最快 | 非隔离、评测一致性差 | 仅本机冒烟 |

### 4.5 浏览器对 TB 评测的价值（对齐调研结论）

- 官方 minimal mode **没有浏览器**、没有 web_search——82.7% 不依赖
  浏览器。我们的 12/18 超时挂在 web_search 而非浏览器缺失。
- 但 local_browser 的价值在**渲染型页面**（JS 动态内容、需要登录的
  站点）：web_fetch 拿不到渲染后文本时，browser_read 是补充通道。
- 结论：docker 浏览器是**能力补全**（模式 A 的 local_browser 分支
  真正可用），不是超时问题的解药；检索效率（子代理预算/并发/超时）
  仍是主线。

## 5. 与既有文档的关系

- 本设计是 THIN-HARNESS-REDESIGN V2 执行侧"半助理层加厚"的代码结构
  配套：拆分让检索调度获得独立边界，后续编排增强（子代理 run 级
  预算/超时）在 `retrieval/dispatch.rs` 内落地。
- 拆分不改变 ADR-0010 语义；B1–B8 完成后已按既有纪律登记
  CLI_PROJECT_INDEX（AUTH-CONTROLLER-SPLIT，状态 `implemented`，
  2026-08-30）。二轮拆解批次计划见 §3.5，实施跟踪见 TODO。

## 6. 验收标准

1. controller.rs 从 29,091 行降至目标 ≤10,000 行（不含测试则更少）。
2. 全量测试通过：orz-loop / orz-host / orz-tools / orz-bin / pytest
   事件链，与拆分前基线一致（0 新增失败）。
3. 一个真实 run 的 journal 事件序列与拆分前同输入逐字节一致
   （run_event_journal_validation 严格校验通过）。
4. `git diff` 无行为改动：只含移动/可见性/注释调整。

进度（2026-08-30）：B1–B8 完成后 controller.rs 24,861 行（生产
7,548 / 测试 17,313）；生产区已低于 10k，测试区仍为体量主体；二轮
批次 N1–N5（§3.5）执行中，N1–N4 已闭合
（8f5e058/f1cf0b0/d5cbd60/13908c0），controller.rs 23,132 → 20,264
行，目标全文件 ≤10,000 行。**2026-08-30 N1–N5 全部闭合**：
controller.rs 20,264 → **4,142 行**（测试区 202 项归位，留守 19 项
主循环测试），二轮拆解完成，验收标准 1（≤10,000 行）达成。

## 7. 后续项（拆分完成后，独立设计）

- **子代理 run 级预算与超时**：`run_retrieval_subagent` 入口加轮数/
   墙钟双层预算；慢调用中断（并行工具调用先完成者优先）。
- **检索结果通道收紧**：blackboard 指针摘要 + 子代理结果有界化，
   避免单次 dispatch 拉 8 页串行 fetch 的分钟级放大。
- **docker 浏览器评测试点**：B 批次任一完成后，用 1–2 道渲染型题
   （mteb-leaderboard / configure-git-webserver）试 local_browser
   通道，验证 4.3 方案与 120s 超时共存。
