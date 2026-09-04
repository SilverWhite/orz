# P2-14 S1 全面复审处理（2026-09-04）

> 范围：对 2026-09-04 工作区中 P2-14 S1（压缩 marker 折叠视图快照，
> ADR-0010 §14.54 / `CONTEXT_COMPACTION_BLACKBOARD_FOLD_DESIGN
> _2026-09-04.md`）已落代码的全面复审（设计合理性 / 实现合理性 / 设计
> 实现符合性）及其全部问题的处理登记。状态：`handled`（S1 收口；S2–S4
> 待续）。入口：BACKLOG §14 / TODO P2-14 / ADR-0010 §14.54 补注 5–6。

## 1. 复审发现与处理映射

### 1.1 硬性问题（已修复）

| # | 发现 | 处理 |
|---|---|---|
| 1 | `cargo fmt --check` 全仓失败：约 1300 行 diff、10+ 文件出现
  `round: None, });` 顶格拼行（controller_test_support / delivery /
  action_ledger / agent_loop / host_exec / fake / activation / dispatch /
  disposition / controller 等） | `cargo fmt` 全仓修复；fmt/diff 净 |
| 2 | `handle_parent_disposition` audit mirror（disposition.rs）仍取
  live `blackboard_stamp()`，写共享折叠分区时留下子车道轮号 | 改取
  `effective_blackboard_stamp()`（执行窗主轮章）并登记注释 |
| 3 | DispatchStamp 提交点在子车道 loop 后仍取 live 章（2/3…），而共享
  镜像行已取主轮 1——「与 DispatchStamp 同轮」注释失真 | DispatchStamp
  提交点改取 effective；cadence 测试补 DispatchStamp.round == 1 断言 |
| 4 | 子车道消息盖实时 LIF 轮章 + 行章 pin 主轮 → 子会话内双轴错配
  （若按「保留尾首条声明轮章」推 r_keep 会把保留尾行误收进 marker） | 车道
  范围裁决：v0.3 marker 只用于主会话；子车道/旧会话回退 v0.2 五段模板；
  消息轮章只盖 Main 车道声明。裁决登记 ADR §14.54 补注 5、设计稿 §6 |

### 1.2 结构性问题（已处理）

| # | 发现 | 处理 |
|---|---|---|
| 5 | 快照入口无 C 上限/排序/溢出信息，且 B 明细可能一次给出超视图 cap
  的行（「有界」声明悬空） | 装配分层：快照入口提供同源候选；
  `select_annotations_closest_to_window`（最接近近窗 ≤N + 溢出计数）；
  summary.rs v0.3 装配层执行 B 50 行/4K 视图 cap、C ≤30、D ≤3K、总量
  20K（含 env 覆盖），超限一律指针；文档口径修正 |
| 6 | §6/§7「快照与 blackboard_read 逐字节一致」与 §2 块级结构（B/C
  分块）冲突；测试夹具只覆盖整段同态 | 口径修正为「共享分段/展开子集/
  标注词汇/行截断；块级分区呈现，不承诺穿插位置逐字节一致」；测试覆盖
  部分展开段的选择器排序/溢出 |
| 7 | 未达 T/W 阈值强制折叠的「有界」缺装配级测试 | summary v0.3 装配
  测试：小预算下 marker ≤ 总量、B/C 裁切指针可见（矩阵 4/6） |
| 8 | §7 矩阵 1–7 未齐（原仅 1/2/5 部分有） | 补 snapshot pre-stamp
  edits/tool_actions、选择器 30 上限/溢出、装配空态/预算/排除/annex、
  e2e v0.3 marker 断言；1–7 全落地，8 由 S2 复验 |
| 9 | summary.rs 仍 v0.2 五段、17K、估算 9_000；run_template_compact
  未接线 | 主会话 v0.3 A–E 装配 + 存档 + annex + 接线（r_keep 在
  drain/fallback 后取保留尾首条声明轮章，槽构建移到 drain 后）；
  `SUMMARY_MARKER_ESTIMATE_TOKENS` 重校准 11_000；v0.2 路径保留为回退 |
| 10 | BACKLOG/TODO 描述落后于代码（epoch 快照入口已存在却说待续） | 同步
  BACKLOG §14 / TODO P2-14 / ADR §14.54 / 设计稿 §6/§7/§9，登记本审计 |

## 2. 代码面改动摘要（orz 子模块）

- gateway `Message.round`：serde default/skip、transport 映射不上 wire；
  仅 Main 车道 assistant 声明盖主决策轮章。
- `AgentLoopController`：`board_stamp_pin` + RAII 守卫；
  `effective_blackboard_stamp`（pin 优先，None 回退 live）；共享折叠写面
  （edits / tool_actions / exec）、dispatch 镜像、DispatchStamp 提交、
  disposition audit mirror 统一取 effective。
- epoch.rs：`render_exec/edits/tool_actions_snapshot`、pre-stamp 强制折叠
  归 C、`cap_fold_view` pub(crate) 复用、`select_annotations_closest_to_window`。
- summary.rs：v0.3 A–E 装配（CompactionBudget 常量/env、FoldSnapshotCtx、
  FoldSnapshotInput/Output/Stats、marker+archive+annex）；v0.2 函数保留。
- agent_loop.rs：run_template_compact 增 `fold_ctx` 参数与 v0.3/v0.2 双轨；
  controller.rs session-end 传主会话 ctx；dispatch.rs 子车道传 None。

## 3. 验证证据

- `cargo fmt --check`：通过。
- `cargo check -p orz-loop --lib`：通过（仅既有 `run_host_tool` dead_code
  警告，非本次引入）。
- `cargo test -p orz-loop --lib`：720 passed / 0 failed / 3 ignored。
- `cargo test -p orz-host --lib acp`：43 passed。
- 新增测试：epoch 快照/选择器 6 项、summary v0.3 装配 4 项、e2e 主车道
  v0.3 marker 断言、DispatchStamp 同轴断言、edits/tool_actions pre-stamp。

## 4. 残余与边界

- S2：压缩 e2e（rhythm / fallback / session_end / 恢复预检全串行，§7 矩阵
  第 8 项复验）；S3：Linux musl 重建；S4：实机复验 + 遥测（marker 尺寸
  分布 / 块溢出频率 / blackboard_read 跟随率 / restore 可用性）。
- 检索/grill 车道压缩保持 v0.2 五段模板为既有语义（车道范围裁决）；
  若 S4 显示需要，另立项统一车道面 marker。
- 部分展开段的视图穿插顺序在 v0.3 块级结构中不保留（登记为块级结构
  差异，非 gap）。
