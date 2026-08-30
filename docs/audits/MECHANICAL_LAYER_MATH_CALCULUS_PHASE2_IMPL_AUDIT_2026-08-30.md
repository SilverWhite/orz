# P2-10 阶段 2 I2–I6 实施记录（事件面 / temporal 运行时 / 侧车 / 信封 / pipe）

> 日期：2026-08-30；切片：I2–I6（TODO P2-10 / BACKLOG 10）；依赖：F1–F4/D1/D7
> （已定稿）。状态：各切片 S1 代码 + S2 测试完成，阶段 3 V1–V3 验证待续
> （放行入账按既有纪律）。

## I2 — 失败目标身份入事件面（F4 §5）

- Schema：`runtime/tool-completed-event-payload-v0.1.schema.json` 增可选
  `failure_target`（kind=cmd_target|anchor_target|file_target|url_target、
  id=64-hex digest、cmd_preview ≤80B、anchor 三元组、canonical_url）；
  并补顶层 `file_path`（既有 content_anchor_mismatch 生产者字段的漂移修复）。
- Verifier：`assurance/run_event_journal_validation.py` 新增并注册
  `_verify_v02_failure_target`（kind 合法、id 格式、kind 专属字段、工具族一致、
  仅 status=error）。
- Fixtures：4 正例 + 2 schema 负例，`scripts/check_repository.py` 映射注册；
  `runtime/tests/test_run_event_journal_validation.py` 新增
  `FailureTargetCrossCheckTests`（11 用例；计数勘误 F9）。
- Rust 生产者：`orz-loop/src/failure_target.rs`（canonical_command /
  canonical_url=ACAF 规范化 / bounded_preview / 工具→kind 映射 + 9 测试），
  host_exec 三处失败事件接线（锚点拒单、宿主错误、候选门拒绝）。

## I3 — temporal 分区运行时（F2 §3 / F3）

- `AgentLoopController` 增 `lif: Mutex<LifEngine>`（两个构造器初始化）。
- 决策轮喂入：`agent_loop.rs` model_output（tool_calls 非空）→
  `on_decision_round`；工具事件喂入：`host_exec.rs` Ok/Err 两路径
  （timed_out=错误、exit 0=成功、exit≠0=值语义）。
- 查询面：`render_temporal_section`（now/recent(k≤20)/history/feature(name,k≤20)，
  ≤1 KiB、fires 不渲染、零注入）；`blackboard_read section=temporal` 接线
  （selector/k/name 参数 fail-loud、epoch/receipt_id 组合显式拒绝）；工具定义
  section 枚举增 temporal。
- 测试：2 个端到端（正常 Now 渲染 + 非法 selector fail-loud）。

## I4 — 域 spike 存档随会话侧车（F2 §3.5 / D7）

- `StoredConversation` 增可选 `temporal_spikes`（serde(default) +
  skip_serializing_if=None → 旧侧车零迁移）；`persist_conversation_sidecar`
  携带 spikes（空则省略）；ACP load 路径恢复进 controller
  （`restore_temporal_spikes` 重建域机器）；7 天 retention 复用既有 sweep
  （测试确认）；子代理不持久化（侧车仅主 ACP 会话）。
- 测试：roundtrip（spikes 往返）、legacy 无字段解析、空会话不落盘、retention。

## I5 — 工具类型化信封（F1 §2.1/§2.2）

- `orz-assurance/src/tool_envelope.rs`：OkEnvelope（summary ≤200B + cap +
  payload + pointer）/ FailEnvelope（step/code/message ≤200B/trace_id）/
  Pointer（File/Board/Evidence/Cmd）+ 边界强制（UTF-8 安全截断+标记，5 测试）。
- blackboard_read 成功路径结构化 Board 信封（partition/entries/total_cap，
  temporal ≤1 KiB、其余 ≤8 KiB）。
- 四项既有行为核查并契约锁定：file.read View（ORZ-LARGE-FILE-READ-CONTRACT 已
  落地）、search_replace GetPut（anchor_mismatch 类型化拒单 + no_match/ambiguous
  类型化输出 + 新增端到端锁定测试）、terminal.run 值语义（exit_code≠0 是值 +
  wall_ms 一等字段）、blackboard temporal 截断（≤1 KiB 断言）。

## I6 — 一层组合/pipe（F1 §2.3/§2.4 折中档）

- `orz-assurance/src/reducer.rs`：最小项集（Apply + 单层 Pipe，无 let/条件/
  嵌套）、归约边界 ≤4 步、类型化 splice（View path → 第二参数；read→
  search_replace 锚点透镜）、Fail 短路、effect_count 透传硬门复用（6 测试）。
- 顶层保持 tool_calls 形态；模型面 pipe 语法接线留待 V1（FakeProvider 验证面）。

## 回归与门禁

- orz-assurance 178、orz-loop 644、orz-host 240 测试通过（1 项既有时序 flaky
  独立跑通过）；Python runtime + p0_contracts 347 通过；clippy 无新增告警
  （orz-loop 28 项为既有基线）；`git diff --check` 双仓库干净。（orz-loop
  644 为 `cargo test --list` 实测，原写 641 勘误 F9。）
- 仓库门禁 check_repository：除「orz 子模块工作树 dirty」（未提交期间的固有
  项）外全部通过。
- 未闭合计数 32 → 38（I1–I6 放行入账；V1–V3 验证待续）。
