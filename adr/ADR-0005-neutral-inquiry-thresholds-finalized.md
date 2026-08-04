# ADR-0005：中立问询 4 判定点默认阈值定稿（§4.6.3 Phase 3 接线）

- 状态：accepted
- 日期：2026-08-04
- 关联：ADR-0003、`INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6.3、`CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` §7.3

## 1. 背景

设计文档 `INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6.3 规定 4 判定点
（输出阈值 / 工具调用次数 / 动作次数 / 轮次）任一超阈值即触发同一中立问询，且触发瞬间
4 计数全部清零（隐式冷却）。默认阈值标注"Phase 3 接线时按 §4.6.3 语义定稿，数值调整
以 ADR 记录"。Phase 3 Slice #2 接线时正式定稿。

## 2. 决策

4 判定点默认阈值全部采用**严格大于**语义（与 stagnation 先例 `> threshold` 一致）：

| 判定点 | 阈值 | 触发条件 |
|---|---|---|
| 输出阈值（consecutive / ngram repeat） | 10 | repeat > 10（沿用 stagnation 默认） |
| 工具调用次数 | 10 | tool_calls > 10 |
| 动作次数 | 10 | actions > 10 |
| 轮次 | 8 | rounds > 8 |

配套定稿语义：

- 触发检查顺序固定：output_repeats → tool_calls → actions → rounds，返回第一个命中。
- 触发瞬间**三实例（主 agent + 检索子代理 ×2）4 计数全部清零**——防止近阈值实例在下一轮
  立即补触发（§4.6.3 刷屏理由的跨实例推广）。
- 阈值调整（含按模型/任务类型的差异化）必须以新 ADR 记录。

## 3. 后果

- Rust 实现：`orz-loop/src/inquiry.rs` `DEFAULT_THRESHOLDS`（output_repeats: 10,
  tool_calls: 10, actions: 10, rounds: 8）。
- 作用域：仅轮内中立问询（INFO_SUFFICIENCY）；子代理关闭前 completion check 独立，
  不参与 4 计数、无积累语义。
- 数值变更路径：新 ADR 而非直接改代码。
