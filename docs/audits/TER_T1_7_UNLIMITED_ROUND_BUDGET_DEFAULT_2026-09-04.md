# TER T1.7 轮预算默认无限制实施审计（2026-09-04）

> 范围：TODO2 M1 步 T1.7——主车道 `max_tool_rounds` 默认 120 硬限撤除
> （“120 per turn” 静态宣示动态化/移除）；保留可配上限逃生阀与
> `budget_insufficient` / exhaustion 机制（仅显式配置非零上限时生效）。
> 验收 = 默认运行无 120 拦截；显式配置上限时原机制仍生效。
> 上级：[TODO2.md](../../TODO2.md) / 设计稿
> [TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md](../TOOL_EXECUTION_LAYER_REFORM_DESIGN_2026-09-03.md)
> §3.8（轮预算撤除）/ §10 S1-5；用户 2026-09-03 拍板记录见设计稿 §9-10；
> ADR-0010 §14.53 候选（AUTH-TOOL-EXECUTION-REFORM）与历史 ADR-0008 /
> ADR-0010 v1.1（120 冻结）回看入口在 controller.rs 常量文档。

## 1. 目标与验收

- 默认 `max_tool_rounds = 0`（unlimited）：无 120 轮硬闸、无 exhaustion
  注入、无 `budget_insufficient` 预检。
- 显式配置（`ORZ_MAX_TOOL_ROUNDS` >0 / `with_max_tool_rounds(>0)` /
  `controller.max_tool_rounds > 0` seam）恢复既有硬闸与
  `budget_insufficient` / exhaustion 语义（escape hatch）。
- 模型面不再宣示一个不存在的静态 120 轮档位：`blackboard_read
  section=session` 在无上限时显示 unlimited；显式上限时按生效值渲染。
- 检索子代理档位上限与主车道取 min 的组合语义在“主车道无上限”下正确
  落到检索档位自身（不能把 0 当 1 轮）。

## 2. 代码改动（orz-loop）

- `controller.rs`：
  - `MAX_TOOL_ROUNDS` 120 → **0**（0 = unlimited），常量文档改写为
    TER T1.7 口径并保留 8→40→120→撤默认 的历史供审计回看；
  - `max_tool_rounds_override()` / `with_max_tool_rounds()` 文档同步
    （0 = unlimited；>0 = 显式逃生阀）。
- `agent_loop.rs`：轮数闸加 `profile.max_tool_rounds > 0` 守卫——默认
  无硬限时不挂载 `tool_rounds_limit` 闸 / 不注入 exhaustion 块。
- `prompt.rs`：`session_face_block()` 对 `budget == 0` 渲染
  `TOOL_ROUND_BUDGET: unlimited` 与 `TOOL_ROUNDS_REMAINING: unlimited`
  （不再宣称静态档位）；显式上限时原数字档与 remaining 不变。
- `console_exec.rs`：发放前 `budget_insufficient` 预检仅在
  `max_tool_rounds != 0` 时计算/生效（无上限时订单恒可发放，交由墙钟
  等其它闸兜底）。
- `retrieval/dispatch.rs`：检索子代理有效轮数 = 主车道显式上限（>0）与
  检索档位上限的 min；主车道无上限（0）时取检索档位自身；两者均无 →
  0（unlimited）。

## 3. 测试

| 位置 | 用例 | 覆盖 |
|---|---|---|
| prompt.rs | `session_face_reports_unlimited_when_no_cap` | budget=0 → unlimited 宣示；显式 120 → 原数字档 + remaining 保留 |
| host_exec.rs | `round_budget_unlimited_by_default_does_not_intercept` | 默认 max_tool_rounds==0；三轮工具调用全放行、无 `tool_rounds_limit` GateDecision、无 exhaustion 注入 |
| host_exec.rs | `round_budget_exhaustion_reports_partial_result`（既有） | 显式上限（1）时原 exhaustion 闸仍生效 |
| planning.rs | `plan_first_round_does_not_consume_tool_round_budget`（既有） | 显式上限语义不回归 |

## 4. 验证证据

- `cargo test -p orz-loop session_face`：1 passed。
- `cargo test -p orz-loop round_budget`：4 passed（含新 unlimited 用例）。
- `cargo test -p orz-loop blackboard_read_serves_session`：1 passed。
- **`cargo test -p orz-loop --lib`：699 passed / 0 failed / 3 ignored**
  （全量回归，覆盖 dispatch/planning/console 等上限依赖面）。
- `cargo fmt -p orz-loop -- --check`：净。
- orz 仓库 `git diff --check`：exit 0。

## 5. 边界声明

- 0 = unlimited 的语义与既有 `ORZ_RETRIEVAL_MAX_TOOL_ROUNDS=0`（禁用
  独立上限）及检索墙钟 0=禁用 口径同族；机械审计 `record_budget` 在
  无上限时以 0 记录（unlimited），事件/审计字段不误导为 1 轮。
- 评测侧不额外设轮上限（设计 §3.8：墙钟为准，见 M2 T2.1）；连续拒绝
  断路器（IP2a/D-3）与工具层 idle+CPU 兜底（T1.5）仍承担防失控主职责，
  本次只撤 120 轮默认硬限。
- 检索子代理档位默认（standard 30 / extended 60 / deep 90，effort.rs）
  不受影响：主车道无上限时它们仍按自身档位约束单次派发。
