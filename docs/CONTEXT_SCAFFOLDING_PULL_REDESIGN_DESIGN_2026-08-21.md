# 上下文机械结构块 PUSH→PULL 重设计（2026-08-21）

> 状态：**已定稿**（2026-08-21 用户裁决：方案 A 先行、C 暂缓、状态行保留）；
> **S1 已实施并通过全面审查处理**（2026-08-21，见 §8）。
> 关联：
> [DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md](DEEPSEEK_OUTPUT_BUDGET_AND_STALL_GUARD_DESIGN_2026-08-20.md)
> （256K 输出预算与哨兵）、ADR-0010 §14.35、官方 deepseek-harness
> minimal 模式对照（`tmp_dsh_review/`）。

## 1. 背景与根因链

2026-08-20 跑分冒烟（r1-g1/g1b/g1c）暴露两类失败：网络层（已确认）与
**模型侧失控**（content_repetition 0.87、reasoning 64K 空转、256K 概率性
无限生成）。API 实测定位失控触发条件：

1. `max_tokens=262144` + `reasoning_effort=high` + **重复性上下文** → 150s
   收到 12.5MB 流仍未停止（概率性）；
2. 同请求 `max_tokens=8192`（同上下文）→ 3.7s 自然收尾；
3. 同请求 256K + 不带 effort / low / medium → 1.9–5.3s 自然收尾；
4. 256K + high + **非重复真实源码上下文**（166KB）→ 4s 自然收尾。

结论：失控 = `256K 预算 × high 推理 × 上下文/输出出现重复循环`。模型一旦
在难题上进入复读，256K 预算允许其无限延续，哨兵成为唯一刹车。

## 2. 架构独有性：我们每轮向上下文 PUSH 的机械结构块

对照官方 deepseek-harness minimal 模式（`includeRuntimeContext: false`、
无压缩、仅双工具、固定 persona、idle 48h、contextWindow 1M）与 Codex/Claude
类框架：它们不向对话注入框架自有机械状态，工具调用/结果即任务内容、天然
可变。我们为「机械可见性 + 前缀缓存稳定」选择 PUSH 注入，积累重复块：

| 注入块 | 位置 | 频率 | 说明 |
|---|---|---|---|
| `[TOOL_ROUND_BUDGET] REMAINING` | agent_loop.rs 2434 | **每工具轮 1 条** | D-8 机械重申；旧条不删，历史积累 |
| `[任务状态]` 状态行 | controller.rs 3943 | 仅变化时追加 | 旧条不删，长会话多份 |
| `[本轮编辑]` 摘要 | agent_loop.rs 2343 | 编辑轮后 | 任务内容，中频 |
| 压缩/折叠标记与桥 | compaction/fold | 低频 | 机械标记 |

S4 实测 333 工具轮 → 历史中约 333 条近同预算块；`[任务状态]` 随步骤状态
变化追加多份。该形态正是 §1 触发条件中的「重复性上下文」。

## 3. 为什么这是设计失败（用户裁决方向）

- 我们用「追加不删」换取前缀缓存命中率（2026-08-07：system prompt 内嵌
  REMAINING 命中 17% → 尾随消息 98%+），代价是上下文随轮次膨胀且充满
  框架自有近同文本——这是把「模型舒适度」让位于「计费优化」，且放大了
  复读失控的触发面。
- 机械门禁（预算不足拒绝、工具轮上限、步骤门）本就是 fail-closed 的硬
  约束，**模型并不依赖 REMAINING 文本才能正确执行**——PUSH 只是指导性
  提示，不是正确性前提。

## 4. 设计目标与方案

目标：让会话上下文由「任务内容」主导，消除框架自有重复块的积累；同时不
退回 2026-08-07 的缓存灾难。

### 方案 A（已采纳，先行实施）：预算块 PUSH→PULL

- 退役每工具轮的 `[TOOL_ROUND_BUDGET] REMAINING` 尾随消息（agent_loop.rs
  D-8 注入点）。
- `blackboard_read` 新增 `section=session`：返回工具轮预算剩余 + 状态行
  概要，按需读取；模型在计划/执行轮可自行决定是否读取（与 actions 分区
  同纪律）。**数据源**：remaining = controller activation `tool_rounds_used`
  / `max_tool_rounds`，状态行 = `controller.render_status_line()`——均在
  controller 内，无需跨组件接线；session 面为 live 会话状态、不进 epoch
  归档（黑板数据面不动，工具定义增量扩展）。
- 机械硬门禁保留：`budget_insufficient` 预检拒绝（反馈文本含剩余预算）、
  工具轮上限耗尽块、`run_invalidated` 收尾——模型即使不读也绝不会超跑。
- 系统提示词中的**总预算声明块**（`tool_round_budget_session_block`，
  agent_loop.rs 1234）保留——静态一次、声明总上限，不构成轮次重复。
- 预期收益：333 轮会话从 ~333 条预算块 → 0 条常驻；输入随会话增长放缓，
  上下文更接近任务主导；复读失控的「燃料」减少。
- 权衡：模型可能每轮读一次 session（工具结果注入一段），但那是任务驱动
  读取、内容短小，且机械拒绝仍兜底；前缀缓存形状变化需 S4 实测命中率。

### 方案 B（去重替代，不推荐）：仅保留最新预算块

在历史中替换旧 REMAINING 块而非追加——会破坏提供方前缀缓存（2026-08-07
17% 教训），且实现侵入消息数组，否决。

### 方案 C（暂缓，待 A 的 S4 复验后裁决）：输出预算收紧

`REQUEST_MAX_TOKENS` 256K → **64K**（主代理；S4 单轮均值 <1K，正常轮次
不受影响；复读轮快速触顶止损）。**用户裁决先做 A 观察效果再定 C**——若 A
消除重复燃料后哨兵触发率显著下降，256K 可维持；否则再收紧。

### 方案 D（已定：保留现状）：状态行与编辑摘要

- `[任务状态]` **保留现状**（仅变化追加、体量小），不随本次改动；观察 A
  落地后效果再定是否并入 session 读取面。
- `[本轮编辑]` 属任务内容（本轮真实改动），保留。

## 5. 实施路由

S1 代码（退役 D-8 REMAINING 尾随注入 + blackboard_read session 面 +
工具定义增量 + 机械门禁回归）→ S2 测试（单测：无 REMAINING 尾随消息、
session 面渲染/越权报错、budget_insufficient 拒绝文本仍含剩余、既有
TOOL_ROUND_BUDGET 断言更新——controller.rs 12078 附近；回归全绿）→
S3 Linux musl 重建（新冻结版本）→ **S4 复验（单题重点观测）**：

1. **缓存命中率**：逐请求 `cache_hit_tokens/cache_miss_tokens`（journal 已
   有），对照 A 落地前基线（S4 make-doom：98%+ 稳态），目标不减；
2. 零 HTTP 400、零 idle 死线；
3. 哨兵触发率（content_repetition / reasoning_stall）应下降；
4. 输入 token 随会话增长趋势放缓（无 333 条预算块积累）。

计数不变（28）至 S3/S4 闭环。

## 6. 决策点（待用户裁决）

已裁决（2026-08-21）：① 采纳方案 A 先行；② 方案 C 暂缓（S4 复验后定）；
③ 状态行保留现状。实施路由 S1→S4 照常，S4 以缓存命中率为重点观测项。

## 7. 关联登记

- 本设计定稿后：ADR-0010 追加项、BACKLOG（P0 项）、TODO（S1–S4 勾选）、
  CLI_PROJECT_INDEX 登记。
- 官方对照：deepseek-harness minimal 模式（无运行时上下文注入、无压缩、
  contextWindow 默认 1M、idle 48h）为「结构性避免」参照，非照搬（我们不
  放弃机械可见性，改为按需可见）。

## 8. 全面审查处理登记（2026-08-21）

S1 全面审查结论：设计合理、实现合理、设计与实现符合性良好，无功能缺陷。
审查 O 项处理如下：

- **O1（口径差异，已接受）**：session 面 `TOOL_ROUNDS_REMAINING` = budget
  − used（含在飞轮轮），退役块为轮后口径（max − 已耗）。轮首读取两者数值
  一致；括注「the round in flight counts when it completes」消除歧义；
  符合 §4 数据源公式字面。S4 复验观察模型解读，必要时再统一口径。
- **O2（已接受边界）**：PULL 读取本身消耗工具轮（每次读计 1 轮）；机械
  门禁兜底、内容短小，设计 §4 权衡已登记。S4 观测实际读取频率，若异常可
  加读取频率引导（阶段 2 可选后续，同 receipt 点读频率引导方向，不占计数）。
- **O3（已接受）**：状态行双通道——尾随推送（仅变化时追加）与 session 面
  内嵌（`render_status_line`）同轮可能重复；内容小、任务驱动，不构成重复
  燃料；S4 观察。
- **O4（口径收紧）**：session 越权组合（epoch / receipt_id）由渲染层文本
  错误升级为参数级显式报错——ToolCompleted `exit_code: 1` + `error` 字段、
  工具结果 exit_code 1、错误文本回达模型；与非法 epoch/receipt_id 同纪律，
  绝不静默回退。既有的 receipt_id+非 actions 先例（2026-08-19 方案 B）
  保持渲染层文本错误（exit_code 0），为已登记差异、不改动冻结行为。
- **O5（测试补齐）**：越权组合新增工具级回达测试（事件 exit_code 1 +
  error 字段、错误文本回达模型），补足单测之外的端到端覆盖。
- **O6（文案清理）**：`blackboard_read` 工具描述去除内部标签「PUSH→PULL
  2026-08-21」，改为纯语义文案（模型面），与其余模型可见描述保持一致。

## 9. S4 复验阻断与修复（2026-08-21）

**阻断现象**：7529a71 冻结版 S4 单题复验（make-doom-for-mips，job
`2026-08-21__01-18-47`）首轮工具轮后，第二轮请求即被 DeepSeek 拒绝：
`The reasoning_content in the thinking mode must be passed back to the
API. (code=invalid_request_error)` → `run_failed`。

**根因链**：PUSH→PULL 退役每轮 REMAINING 尾随 user 消息后，暴露了一条
2026-08-04 遗留的「工具输出汇总 assistant 文本消息」（`assistant_parts`
块：每轮把 `[tool] output` 汇总成一条无 tool_calls、无 reasoning_content
的 assistant 消息）。2026-08-06 协议修复后工具结果已以 Role::Tool 消息
落库，该汇总本已是冗余副本；此前它一直被 REMAINING 尾随消息「遮住」，
现在成为请求序列末条，恰好命中 DeepSeek thinking 模式校验。

**API 实测（2026-08-21 探针 V1–V7，最小成本直连 api.deepseek.com）**：

| 变体 | 序列 | 结果 |
|---|---|---|
| V1 | 工具结果后紧跟 assistant 文本（无 rc，末条） | **400**（同线上报错） |
| V2 | 同 V1 + 尾部 user REMAINING（旧形态） | 200 |
| V3 | 同 V1 + 汇总消息带 reasoning_content | 200 |
| V4 | 无汇总消息（末条=最后一条 Role::Tool） | 200 |
| V5 | 纯文本轮（无 tools 参数）无 rc | 200 |
| V6 | 纯文本轮（带 tools 参数）无 rc | 200 |
| V7 | 同 V6 + rc | 200 |

结论：触发条件精确为「工具结果之后紧跟的 assistant 文本消息且未回传
reasoning_content」；纯文本轮不受影响（对照官方
`packages/llm/llm-deepseek/src/serialize.ts`：工具轮回传、纯文本轮丢弃）。

**修复（S4 缺口修复，orz 新提交）**：退役 `assistant_parts` 汇总消息——
工具结果本身已完整以 Role::Tool 落库，汇总为冗余副本；移除同时带来每轮
输入 token 节省（make-doom 333 轮量级下收益显著），与方案 A 的上下文
极简目标一致。消息协议形态 4→3（user + assistant 声明 + tool 结果）。
同步更新两处协议形状测试（`protocol_shape_…` / 
`tool_round_replays_reasoning_content_on_declaration`）；clippy 清理
`refuse_inject_budget`/`plan_round_denied` 返回值的未用变量。
验证：orz-loop 536 通过 / 0 失败 / 3 ignored、fmt 干净、clippy 无新增
（基线 31 不变）。

**后续**：S3 重建（新 orz 提交）→ S4 make-doom 重跑，重点观测与 §5 一致
（命中率 ≥90% 且对照基线不减、零 400、哨兵触发率下降、输入增长放缓）；
全绿后闭环登记（TODO P0-0e 勾 S3/S4、BACKLOG 0e 转 implemented、计数
28→27、ADR-0010 §14.35 追加、CLI_PROJECT_INDEX 登记）。
