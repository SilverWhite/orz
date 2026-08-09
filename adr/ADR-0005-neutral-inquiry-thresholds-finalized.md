# ADR-0005：中立问询 4 判定点默认阈值定稿（§4.6 Phase 3 接线；2026-08-09 修订绑定对象）

- 状态：accepted（2026-08-09 修订：判定点绑定对象回归方向问询）
- 日期：2026-08-04（2026-08-09 修订）
- 关联：ADR-0003、`INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6、`CN_AGENT_BASE_GRAFT_VALUE_REVIEW_2026-07-30.md` §7.3、`docs/GSA_SELF_QUESTION_COUNTEREXAMPLE_DESIGN_2026-07-26.md` 修正 2/3

## 1. 背景

设计文档 `INTEGRATED_AGENT_LOOP_AND_FORK_DESIGN_v0.2.md` §4.6 规定判定点
（输出阈值 / 工具调用次数 / 动作次数 / 轮次）任一超阈值即触发问询，且触发瞬间
计数全部清零（隐式冷却）。默认阈值标注"Phase 3 接线时按 §4.6 语义定稿，数值调整
以 ADR 记录"。Phase 3 Slice #2 接线时正式定稿。

**2026-08-09 修订**：TB hard B 组复盘发现 2026-08-04 定稿的合并错误——4 判定点被绑到
信息充分性问询（INFO_SUFFICIENCY），原始设计（07-26）中**方向问询**（ORIENTATION_CHECKPOINT，
询问当前任务内容与进度）被吞并。本次修订将判定点绑定对象**回归方向问询**（设计文档 §4.6.2）；
信息充分性问询恢复为独立开关触发（§4.6.3，不绑判定点）。另补**工具种类判定点**（原始
`orientation_trigger_tool_variety: 7` 回归）——判定点由 4 增至 5。

## 2. 决策

5 判定点默认阈值全部采用**严格大于**语义（与 stagnation 先例 `> threshold` 一致），
绑定**方向问询**（ORIENTATION_CHECKPOINT）：

| 判定点 | 阈值 | 触发条件 |
|---|---|---|
| 输出阈值（consecutive / ngram repeat） | 10 | repeat > 10（沿用 stagnation 默认，按轮测量） |
| 工具调用次数 | 10 | tool_calls > 10 |
| 动作次数 | 10 | actions > 10 |
| 轮次 | 8 | rounds > 8 |
| 工具种类（tool_variety） | 7 | 单轮按 tool_id 去重 > 7（2026-08-09 补回原始门控） |

配套定稿语义：

- 触发检查顺序固定：output_repeats → tool_calls → actions → rounds → tool_variety，返回第一个命中。
- 触发瞬间**三实例（主 agent + 检索子代理 ×2）计数全部清零**——防止近阈值实例在下一轮
  立即补触发（§4.6.2 刷屏理由的跨实例推广）。
- 轮次必触发（每 8 轮）不受清零冷却限制（设计 §4.6.2；原始 `orientation_trigger_turn_interval: 8`）。
- 阈值调整（含按模型/任务类型的差异化）必须以新 ADR 记录。

## 3. 后果

- Rust 实现：`orz-loop/src/inquiry.rs` `DEFAULT_THRESHOLDS`（output_repeats: 10,
  tool_calls: 10, actions: 10, rounds: 8 + tool_variety: 7 新增）。
- 作用域：判定点仅驱动**方向问询**（ORIENTATION_CHECKPOINT 注入，事件沿用 `neutral_inquiry`，
  message_block 区分）；信息充分性问询（INFO_SUFFICIENCY）为独立开关（检索/信息获取动作后，
  不绑判定点）；子代理关闭前 completion check 独立，不参与计数、无积累语义。
- 数值变更路径：新 ADR 而非直接改代码。
