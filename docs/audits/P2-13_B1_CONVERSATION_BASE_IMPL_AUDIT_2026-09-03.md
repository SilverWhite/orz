# P2-13 B1 会话化基础实施审计（2026-09-03）

> 主题：BLACKBOARD-CONVERSATION-SCOPE-FOLD（P2-13）B1 会话化基础
> S1 实施 + S2 测试完成；B2（渲染折叠）/ B3（契约与收尾）/ B4（S3 重建
> + S4 复验）待续。设计权威：
> `BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md` v0.8 定稿 +
> ADR-0010 §14.52。

## 1. 实施范围（B1）

### 1.1 记录写时结构化盖章

- `ExecSection.results/errors` 行从 `Vec<String>` 结构化为
  `ExecEntry { text, round, domain, ts }`：
  - `round` = LIF 会话相对决策轮（与 temporal 行 / failure_agg 段同刻度）；
  - `domain` = 写时 LIF 当前域（`Option<Domain>`，None = 旧无章行）；
  - `ts` = 写时墙钟（ISO 8601）；
  - 旧归档字符串经 untagged serde 兼容读取为 pre-stamp 行（round=0 /
    domain=None / ts 空）；`From<&str>/From<String>` 仅作 legacy 形状
    便捷构造，生产写入一律走 `ExecEntry::stamped`。
- `EditRecord` / `ToolActionRecord` / `ActionResult` 追加
  `round: u64` + `domain: Option<Domain>`（serde default，旧数据兼容）。
- 检索分区为派发级全量覆盖写：`InternalRetSection` / `ExternalRetSection`
  追加分区级 `stamp: Option<DispatchStamp { round, domain, timestamp }>`
  ——单次派发所有行共享同一章（R1「retrieval 行按派发轮所属域盖章」的
  实现落点：分区即派发单元）。
- 统一盖章入口：`AgentLoopController::blackboard_stamp()`（LIF 单一来源）
  + `push_tool_action_stamped()`；exec 行在持锁块内取章后
  `ExecEntry::stamped` 直写。

### 1.2 会话级 live 黑板持久化 + conversation-relative 轴

- `Blackboard` 可 Clone；`conversation_snapshot()` 生成跨 prompt 持久化
  快照：剥离 run 级机械面（动作栏注册板块、pending 订单槽）与 live-only
  依赖图；entities / 检索分区随会话延续（设计 §11.1 W 计量含 entities）。
- `restore_conversation_snapshot()`：整体替换 + 各分区版本计数归位为
  「1 次变化」（跨 prompt 徽章不延续上 run 累积计数）。
- `Blackboard.dep_graph` 序列化跳过（live-only，恢复后从零建图）。
- ACP `StoredConversation` 升级为会话续接包：`messages` +
  `blackboard` + `lif`（`TemporalSessionSnapshot`：round / has_success /
  current_domain / entry_round / spikes）+ `session_started_at`（t 轴原点）；
  legacy `temporal_spikes` 保留兼容读取（冗余投影）。
- ACP 每 prompt 续载而非重建：take-out 续接包 → `with_live_blackboard` +
  `restore_lif_session` / `preset_lif_session_axis` → run；成功才写回
  会话与侧车（失败 run 保持 pre-run 状态，与对话成功-only 纪律一致）。
- LIF：`TemporalState::session_snapshot()` / `restore_session_snapshot()`
  精确续接轮号与域机器（取代 I4 restore_spikes 的 round/entry_round
  近似）；`LifEngine::set_axis_origin()` 把 `t` 轴预置为会话起始墙钟
  （不预置 last_time，跨 prompt 间隙不产生衰减伪差）。
- CLI 单 run 不调用任何续载/预置路径——行为不变（orz-bin 回归验证）。

## 2. S2 验证

- 串行全量：orz-loop **657 passed / 0 failed / 3 ignored**；
  orz-assurance **201 + 9 passed / 0 failed**；orz-host
  **242 passed / 0 failed / 4 ignored**；orz-bin **53 passed / 0 failed**
  （含 conformance 12 ignored）。
- 并行全量：orz-host 仅预存时序 flake
  `call_tool_timeout_kills_process_tree`（仓库已登记 HEAD 基线同 flake，
  串行通过）；其余全绿。
- fmt：`cargo fmt --all --check` 干净；clippy：无新增告警（既有告警位置
  全部落在未改动代码段）。
- 新增覆盖：
  - orz-assurance：`session_snapshot_restores_round_and_domain_machine`、
    `axis_preset_and_session_restore_keep_conversation_relative_t`；
  - orz-loop：`exec_legacy_strings_parse_as_pre_stamp_and_stamped_roundtrips`、
    `conversation_snapshot_and_restore_keep_board_but_strip_run_faces`；
  - orz-host：`conversation_sidecar_roundtrip`（升级为含
    lif/blackboard/session_started_at）、
    `cross_prompt_blackboard_and_lif_axis_continue`（两 prompt 各一次
    read_file：exec 行 round 1→2 续接、LIF 快照 round=2、域章齐全、
    侧车不含 dep_graph）。

## 3. 登记状态

- FUS-BLACKBOARD-CONVERSATION-FOLD：`pending` → `partial`（B1 S1/S2 落地，
  B2/B3/B4 待续）；TODO P2-13 B1 勾选；BACKLOG P2-13 变更记录补登。
- 设计/ADR 不改（B1 未偏离 v0.8 设计语义）；B1 期间将检索分区章的
  实现落点记录为「分区级 DispatchStamp」（单次派发 = 全量覆盖单元，
  与 R1 行盖章同效，B2 折叠按分区章消费）。

## 4. 已知边界（后续阶段处理）

- B2 渲染折叠按 (domain, round) 消费新章（本阶段渲染口径逐字节不变）。
- B3 空槽「（无）」/ 用户侧疲劳提醒 / 单包存档 / plan-epoch 生产退役。
- B4：S3 Linux musl 重建 + S4 实机复验（web 通道 A/B、折叠读取/展开、
  恢复、长会话遥测 §13.3）。

## 5. 复审处理（2026-09-03，全面复审）

- 对 B1 实施做全面复审（设计合理性 / 实现合理性 / 符合性）后的处置登记：
  [`P2-13_B1_REVIEW_HANDLING_2026-09-03.md`](P2-13_B1_REVIEW_HANDLING_2026-09-03.md)。
- 代码修正：entities 恢复版本计数归位 1（restore 后 bump）；检索分区恢复
  顺序 = 会话黑板先、activation 覆盖后（compare-and-set，activation 为
  权威源）；failure_agg 轴在 ACP 路径收口为会话相对（取代 CONTEXT §4.4.4
  的 run 级边界声明）。§1.1/§1.2 的措辞精度以复审处理文档为准（历史审计
  不重写）。
