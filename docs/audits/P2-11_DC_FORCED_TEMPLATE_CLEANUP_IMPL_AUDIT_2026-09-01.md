# P2-11 第 2 项：DC 强制模板轮清理实施审计（2026-09-01）

> 状态：实施闭合（2026-09-01）；裁决依据：MODEL-RESIDUAL-PRESSURE-FOLLOWUP
> 深度讨论 §8 裁决 2（2026-08-31 用户确认「直接清理」）；入口：BACKLOG P2-11 /
> TODO P2-11 / ADR-0010 §14.49 / CLI_PROJECT_INDEX（MODEL-RESIDUAL-PRESSURE-
> INVENTORY）。

## 1. 范围与裁决

裁决原文（讨论稿 §8 裁决 2）：「DC 强制模板轮清理（裁决：删除）——需求低
（本批 0 触发；`run_tests` 封存后主信号源死亡；模型增强后收益不足），连同
plan 反例变体注册（无点火路径）一并清理；P3「DC 硬信号 4/6」遗留项随机制
退役。checkpoint 共用件按需拆分（orientation 软门与 console 询问轮保留）。」

本批实施内容：

1. 删除 DC 机制（`diagnostic_coverage.rs`：2→3→4→5 递进硬信号 / 信号消费 /
   强制模板轮触发与提交）。
2. 删除 `COUNTEREXAMPLE_GATE_PLAN_BLOCK` plan 反例变体注册（orz-bin
   `run_plan_phase` 的 plan-write 反例门轮一并移除）。
3. P3「DC 硬信号 4/6」（`same_module_no_evidence` / `key_surface_unexamined`）
   退役。
4. 事件面收口：v0.2 枚举退役 `diagnostic_coverage_checkpoint` /
   `checkpoint_response`，载荷 schema / fixtures / verifier / 测试同步删除。
5. checkpoint 共用件拆分：模板校验机制（仅 DC 消费）退役；orientation 软门
   与 console 询问轮保留。

## 2. 代码删除与保留边界

### 2.1 删除（orz 子模块）

- `crates/orz-loop/src/diagnostic_coverage.rs`（整文件；含
  `DebugEpisodeState`、`maybe_consume_dc_signal`、
  `maybe_consume_dc_retrieval_evidence`、`maybe_fire_dc`、`commit_dc_fire`）。
- `checkpoint.rs`：`PendingCheckpoint::DiagnosticCoverage` 变体、模板校验
  共用件（`TEMPLATE_FIELDS` / `NEXT_ACTIONS` / `parse_and_validate` /
  `cross_check` / `decide_outcome` / `checkpoint_response_payload` /
  `refill_feedback_block` / `CHECKPOINT_REFILL_PREFIX` 等）与对应测试；
  `commit_pending` 去掉 `dc_state` 参数。
- `agent_loop.rs`：`LoopProfile.dc_enabled`、`SharedLoopServices.dc_state`、
  两处 `maybe_fire_dc`、两处 `maybe_consume_dc_signal`、
  `PendingCheckpoint::DiagnosticCoverage` 消费分支（JSON 校验 + 一次重填 +
  `checkpoint_response` 事件 + evidence 交叉核对）。
- `controller.rs`：`dc_state` 字段与初始化、`checkpoint_source_identities`、
  SharedLoopServices 接线。
- `retrieval/dispatch.rs`：`dc_state` 接线与
  `maybe_consume_dc_retrieval_evidence` 调用。
- `prompt.rs`：`COUNTEREXAMPLE_GATE_PLAN_BLOCK` 常量、
  `is_injected_block_text` 中 PLAN_BLOCK / DIAGNOSTIC_COVERAGE_PREFIX /
  CHECKPOINT_REFILL_PREFIX 注册。
- `orz-bin/src/main.rs`：`run_plan_phase` 的 plan-write 反例门轮
  （`generate_stream` + `position=plan_write` 的 `counterexample_gate` 事件）
  与 `render_plan_for_gate`；相关测试调整。
- `orz-tui`：`DiagnosticCoverageCheckpoint` / `CheckpointResponse` 的
  bridge / events / projection 映射删除，bridge 对退役类型降级
  `TuiEvent::Unknown`（枚举保留、v0.2 生产者不再写入）。
- `orz-assurance`：`EventType::DiagnosticCoverageCheckpoint` /
  `CheckpointResponse` 标注 RETIRED（变体保留供历史 replay，v0.2 生产者不再
  写入）。

### 2.2 保留（不可破坏边界）

- orientation 软门（THIN-HARNESS-REDESIGN-V2 §9.2：软消费、不禁工具、
  无 `checkpoint_response`）。
- console 双模式询问轮（PLAN-FIRST 阶段 C，§14.17⑱：decision/reason 模板、
  一次重填、降级 stay）。
- 终答反例门 `COUNTEREXAMPLE_GATE_BLOCK`（once-only）——仅 plan 变体移除。
- v0.1 事件面与 journal fixtures 冻结不变。

## 3. 事件面 / 契约收口

- `runtime/run-event-v0.2.schema.json`：`event_type` 枚举移除两类型；描述补
  P2-11 退役说明。
- 删除 `runtime/diagnostic-coverage-checkpoint-event-payload-v0.2.schema.json`
  与 `runtime/checkpoint-response-event-payload-v0.2.schema.json`。
- 删除 envelope/payload fixtures 6 个（envelope 2 + payloads 4）。
- `assurance/run_event_journal_validation.py`：v0.2 schema 注册移除两类型；
  `_V02_NEUTRAL_INQUIRY_EVENTS` 收为 `{orientation_checkpoint}`；
  `_verify_v02_checkpoint_responses` 删除（含调用点）。
- `scripts/generate_run_event_fixtures.py` / `scripts/check_repository.py`：
  移除两类型条目与 schema 引用。
- `runtime/tests`：conformance 计数 54 → 52、neutral-inquiry 枚举收口、
  checkpoint-response 测试类与辅助件删除、inquiry-kind 反例改用其他类型。
- `runtime/fixtures/run-event-v0.2/journals/plan-run.jsonl`：按当前事件流重抓
  （去除 plan-write 反例门事件；含 2026-08-15 起的 `request_header_change`，
  见 §5 观察项 1）；v0.1 `plan-run.jsonl` 冻结不动。

## 4. 验证证据

- orz-loop lib：**635 passed / 0 failed / 3 ignored**（删除 DC 专项测试后的
  预期数量；orientation 软门 / console 询问轮既有测试保持通过）。
- orz-tui lib：178 passed；orz-assurance lib 193 + 集成 8，全绿。
- `cargo clippy -p orz-assurance -p orz-loop -p orz-tui --all-targets`：
  无错误，无新增告警（既有告警均位于未改动代码）。
- `cargo fmt --all -- --check`：干净（本轮一并重排 orz-bin main.rs 测试调用
  与既有 `orz-host/src/acp_server.rs:562` 一处 fmt 偏离，见 §5 观察项 2）。
- Python：`assurance/tests` + `runtime/tests` 全量 **1920 passed / 14 skipped**，
  唯一失败为 `test_doctor_full_repository_check`（orz_source_manifest 既有
  漂移，§5 观察项 3；orz 提交 + manifest 重生成后复验）。
- conformance capture：`capture_plan_run` 重抓通过（11 事件，terminal
  run_finished，链校验 valid）。

## 5. 观察项（非本批引入）

1. **conformance capture 既有漂移**：提交的 v0.2 journal fixtures 捕获于
   2026-08-13，早于 `request_header_change`（2026-08-15）与
   `mechanical_audit_update`（2026-08-24）；`capture_*` 除 plan-run 外均
   因事件序列漂移失败。本批仅按当前事件流重抓 plan-run（含
   request_header_change），其余场景的整体重抓登记为后续收尾项（与
   TODO「下一次真实运行捕获自然携带 23 工具分区 journals」同类纪律）。
2. `cargo fmt --all` 重排了 `orz-host/src/acp_server.rs:562` 一处既有
   fmt 偏离（StoredConversation::new 换行）；仅格式、无行为变化。
3. `orz_source_manifest.sha256` 落后于 orz 子模块 HEAD（entities.rs /
   epoch.rs / host_exec.rs / planning.rs 等既有失配，非本批引入）；按仓库
   纪律在 orz 提交后重生成并随父仓库提交。

## 6. 登记

- BACKLOG P2-11 第 2 项：闭合；变更记录补 2026-09-01 条目。
- TODO P2-11：DC 清理勾选闭合；未闭合计数 30 → 29 → 28。
- ADR-0010 §14.49：DC 强制模板轮清理补写裁决索引。
- CLI_PROJECT_INDEX：MODEL-RESIDUAL-PRESSURE-INVENTORY 补实施闭合记录。

## 7. 复审清理（2026-09-01，审查后残留处理）

对已提交清理做一轮复审，发现并处理以下残留：

- 代码：删除 `orz-assurance` 的 `TEMPLATE_ANSWER_INSTRUCTIONS` 常量与宏
  （零引用死代码，注释仍称「保留供强制模板轮未来恢复」，与裁决冲突）；
  移除 `maybe_fire_orientation` 的 `force_template_round` 休眠参数（两
  调用点恒传 false，`deferred` 简化为 `role == Main`）；收口 host_exec.rs
  对已删 `ScriptedTestRunnerHost` 的类比注释与 main.rs capture 注释中的
  「强制模板轮」残留措辞。
- 契约：v0.2 fixture README 与 `generate_run_event_fixtures.py`
  `FIXTURES_README_V02` 的枚举计数统一为 **52**（54 − 两 DC 类型；
  原 README 写 50、算式不自洽，脚本模板仍停在 53 的更旧文本）；
  并修复本批提交在生成器中引入的 `SLUGS_V02` 缩进解析错误（整文件
  无法解析），`V02_EVENT_TYPES` / `SLUGS_V02` / `V02_PAYLOAD_EVENTS`
  与 schema / verifier 权威注册表对齐（枚举 52 项、payload slug 24 项；
  补 `tool_running` / `transport_retry`，移除已退役的
  `runtime_stagnation_guard`——既有脱节一并收口）。
- 文档：FRAMEWORK_EFFECTIVE_DESIGN_INVENTORY §1.3/§7.3 收口（删除三个
  注入块条目与「代码保留休眠」表述）；CLI_PROJECT_INDEX 三个机制条目
  标注退役并移入 withdrawn 速查；BACKLOG P2 汇总行更新；ORIENTATION_
  FORCED_TEMPLATE_DESIGN 状态行补退役标记。
