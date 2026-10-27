# 决断台账与上下文编辑设计（0ds M1.5）

> 日期：2026-10-27
> 用户令：「summary 和 detail 上限看你的需求」「当前轮编辑允许就行，反正实际内容会落日志跟着一起存档」「没问题的话就先按这个确定设计吧」。
> 上游：[`ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md`](ORZ_AGENT_OS_ARCHITECTURE_V1_2026-10-27.md)、[`STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md`](STATE_RECENTER_AND_SYSTEM_LAYER_DESIGN_2026-10-27.md)、[`AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md`](AGENT_OS_PRIOR_ART_SURVEY_2026-10-27.md)。
> 性质：设计，不落码；0ds 的 M1.5 子项。

## §0 结论

1. **一个工具**：`context_manage`，模式 `map / recenter / compress / edit / slider / decision_read / decision_write / pin / drop`；0ap 的 `context_compress` 以此子工具面完成。
2. **decisions 是独立追加台账**：`.gsa/decisions/<session>.jsonl`，只追加、不原地改；模型经 `context_manage(decision_*)` 读写，不能直接编辑文件。`summary` ≤300 token（索引行），`detail` ≤2000 token（按 id 拉）；更长挂 `artifact` 指针。
3. **可编辑面**：模型上下文视图（块/摘要/当前轮/滑块）＋黑板（state/plan/notes/findings）。**不可编辑面**：`.gsa` journal/sidecar/archive、系统/安全/工具定义。编辑动作本身落 journal，原块保留 archive，只向前。
4. **当前轮可编辑**：原文永远在 journal；记录 `original_user_message_sha256`；建议先把硬约束抄进 `state.constraints`。
5. **自动压缩照常**；`decisions` 不是压缩替代。压缩摘要从“关键决策散文”改为“原因/结果/事件帧＋decision IDs”。
6. **前缀成本**：`edit` 返回 `prefix_impact_tokens`（第一个被改块之后的估算 token）；尾部/当前轮编辑 miss 小，头部编辑一轮较大 miss；优先尾部/已闭合块。
7. **客观兜底**：journal/sidecar/archive 只追加；`context_edited`/`decision_written`/`slider_*` 事件可审计；原块可回放，编辑可逆（以新 edit 回填，不删历史）。

## §1 分层

| 层 | 回答什么 | 载体 | 编辑 |
|---|---|---|---|
| 当前上下文 | 现在做什么、下一步 | 窗口 | 模型可 edit/slider |
| `state` | 当前事实 | 黑板 | 可写 |
| `plan` | 打算怎么做 | 黑板 | 可写 |
| `notes` | 草稿/过程 | 黑板 | 可写 |
| `findings` | 已成立结果 | 黑板 | 可写 |
| `decisions` | 为什么这么定、具体怎么落 | `.gsa/decisions/*.jsonl` | 经工具追加，不原地改 |
| journal/GSA | 实际发生什么 | `.gsa` | 只追加 |
| archive | 原文 | `.gsa/compaction/blocks` | 只追加 |

## §2 `context_manage` 模板

### 2.1 字段

```
mode: "map" | "recenter" | "compress" | "edit" | "slider" | "decision_read" | "decision_write" | "pin" | "drop"
select?: "blocks 12-14" | "rounds 123-145" | "current" | "all-closed" | "slider"
replace?: "<text>" | "@D-0007" | "@state" | "@journal:<seq>" | "@archive:<path>"
reason?: "<one line>"
decision_id?: "D-0007"
keep?: "blocks 10-12"
drop?: "blocks 1-9"
handover?: "<text>"
status?: "active" | "superseded" | "rejected"
summary?: "<≤300 token>"
detail?: "<≤2000 token>"
evidence?: "<journal seq / state / plan / findings / artifact>"
supersedes?: "D-000x"
query?: "id=D-0007" | "block=12" | "status=active" | "lif=pressure"
```

### 2.2 各模式

| 模式 | 必填 | 行为 |
|---|---|---|
| `map` | — | 返回块表＋state＋decisions 行；≤1K token |
| `recenter` | — | 返回 state＋最近 N 轮摘要＋next＋active decisions（≤5 条）；≤1K＋决策表 |
| `compress` | `select` | 压缩已闭合块；沿用现有语义 |
| `edit` | `select`、`replace`、`reason` | 原子替换选中块/当前轮；可选 `decision_id` |
| `slider` | `keep`/`drop`、`handover`、`reason` | 先写 handover，再开新滑块 epoch |
| `decision_read` | `query` 可选 | 默认 active 的 summary 表（≤20 行）；按 id 返回 detail |
| `decision_write` | `summary` | 追加一条 decision；元数据机械填 |
| `pin` / `drop` | `select` | 标记不可压/可压；元数据变更 |

### 2.3 校验

- `edit`：只允许已闭合块或 `current`；拒绝系统/安全/工具定义、未闭合工具调用、无 `reason`、替换后总上下文增长、替换超过选中块预算。
- `slider`：`keep`/`drop` 必须解析到块；`handover` 先落黑板/state；失败不开新 epoch。
- `decision_write`：`summary` ≤300、`detail` ≤2000；超限拒绝并提示拆条/挂 artifact；`status` 缺省 `active`。
- `decision_read`：默认只返回 active 的 summary；detail 只在按 id 查询时返回。
- 所有模式失败均 fail-closed，不改上下文、不写半条记录。

## §3 decision 模板

```
decision v1
id: D-0007                  # auto
session: <session8>         # auto
block: 12                   # auto（首个选中块/当前轮）
rounds: 123-145             # auto
slider_epoch: 4             # auto
ts: 2026-10-27T...          # auto（北京时间）
lif: normal                 # auto
state_version: <hash>       # auto
status: active              # model
summary: <=300 token        # model（索引行）
detail: <=2000 token        # model（按 id 拉）
  why: ...
  alternatives: ...
  consequences: ...
  implementation: ...
evidence: journal seq / state / plan / findings / artifact
supersedes: D-000x          # optional
artifact: <path>            # optional；detail 超长时
```

- 追加式：`supersedes` 指向旧条；旧条不原地改；`decision_read` 解析最新链。
- `summary` 只进索引/表；`detail` 只在按 id 拉时进上下文。
- 元数据由机械层填，模型不写 `block/rounds/ts/lif`。
- 一条 decision 只记一个决断；实现细节长文挂 `artifact`，不把单条撑大。

## §4 编辑语义

### 4.1 `select`

- `blocks 12-14`：按 `context_map` 的块号。
- `rounds 123-145`：解析为完全落在区间内的已闭合块。
- `current`：当前用户轮＋本轮已闭合的模型/工具交换；若有未闭合工具调用，拒绝。
- `all-closed`：滑块外全部已闭合块（用于 compress）。
- `slider`：当前滑块（用于 slider/pin/drop）。

### 4.2 `replace`

- 纯文本：事件帧/摘要/decision 指针。
- `@D-0007`：替换为 decision 的 summary 引用。
- `@state`：替换为当前 state 快照。
- `@journal:<seq>` / `@archive:<path>`：替换为指针。
- 替换文本 ≤ 选中块预算；机械层报 `saved_tokens`，不强制最小压缩率。

### 4.3 当前轮

- `select="current"` 允许；原用户消息写 journal，记 `original_user_message_sha256`。
- 若当前轮含明确硬约束且 `state.constraints` 为空，工具返回 warning（不拒绝），提示先抄约束。
- 当前轮有未闭合工具调用时拒绝，直到调用闭合。

### 4.4 滑块

- `mode=slider`：`keep`/`drop`/`handover`/`reason`；先写 handover 到黑板/state，再 `slider_closed`＋`slider_opened` 新 epoch。
- 被 drop 的块留 archive；其 decision IDs 留在新 `context_map`。
- 新滑块从 handover＋kept blocks 起步；`state` 继续有效。

### 4.5 黑板

- 黑板可自由编辑（state/plan/notes/findings）；每次写记 `blackboard_write`，历史不删。
- `decisions` 不在黑板；走 `.gsa/decisions` 追加台账，避免双源。

### 4.6 `pin` / `drop`

- `pin`：标记块不可压/不可 edit；`drop`：标记可压/可 drop。
- 元数据变更记 `context_block_marked`；不移动内容。

## §5 机械层记录

- `context_edited`：`selected`、`before_sha256`、`after_sha256`、`replacement_sha256`、`reason`、`decision_id`、`prefix_impact_tokens`、`first_changed_block`、`original_user_message_sha256`、`ts`、`round`、`slider_epoch`、`lif`。
- `decision_written`：`id`、`summary`、`detail_sha256`、`status`、`evidence`、`supersedes`、`block`、`rounds`、`slider_epoch`、`ts`、`lif`、`state_version`。
- `slider_closed` / `slider_opened`：`from_epoch`、`to_epoch`、`keep`、`drop`、`handover_sha256`、`reason`、`ts`。
- `context_block_marked`：`block`、`mark`、`ts`。
- journal/sidecar/archive 只追加；`context_edited` 不改写历史；原块保留 archive，可按块回放。
- 时间统一北京时间；LIF 用现有 domain（start/normal/pressure/low_progress/stuck）。

## §6 压缩策略

- 自动窗口压缩保留；`decisions`/`edit` 是加法，不替代。
- 压缩摘要模板：`原因 → 结果 → 事件帧`＋`decisions: D-...`＋`state: goal/now/next`；不再写长篇“关键决策”。
- `context_map` 每块附 `decisions: D-...`；`recenter` 返回 active decisions（≤5 条）。
- `findings` 仍是“已成立”；`decisions` 是“为什么”；journal 是“发生了什么”。

## §7 前缀成本

- `prefix_impact_tokens = 第一个被改块之后的估算 token`；`edit` 响应返回。
- 尾部/当前轮编辑：miss ≈ 替换段＋新内容，一轮；用户直觉成立。
- 头部编辑：miss ≈ 第一个被改块之后的全部 token，一轮；DeepSeek 上 miss 价 1/50，钱不贵，贵在注意力/连续性。
- 优先尾部/已闭合块，不动稳定头；`0bz` 的“第二针”教训继续适用。
- 机械层只报 estimate，不做质量判断。

## §8 验收

- `context_manage` 九模式可用；`edit` 原子、失败回滚、可逆（以新 edit 回填）。
- 原块可回放；`context_edited` 含 before/after/diff/decision_id/prefix_impact。
- `decision_write` summary ≤300/detail ≤2000；元数据自动填；`decision_read` 默认只出 active summary。
- `context_map` 有 decisions 行；`recenter` 有 active decisions；压缩 marker 带 decision IDs。
- 当前轮编辑：原文在 journal、`original_user_message_sha256` 有值；有未闭合工具调用时拒绝。
- 黑板写入留痕；`.gsa` 只追加；系统/安全/工具定义不可编辑。
- 前缀影响估算返回；头部编辑一轮 miss 可观测。
- 故障消融：选开放块、替换超预算、总上下文增长、系统/安全编辑、decision 超长、缺 reason、当前轮有未闭合调用。
- 不做质量 A/B。

## §9 非目标

- 不直接编辑 `.gsa`/journal/sidecar/archive；不改写历史。
- 不编辑系统/安全/工具定义。
- 不做“删除”，只做“替换＋指针”；原文可回放。
- 不自动推断 decision；不把 decision 当压缩替代。
- 不做常驻 decision 文本；只做工具和台账。
- 不做质量判断；只报 token/前缀影响。

## §10 与 0ds 的关系

- 作为 **M1.5** 插在 M1（基础内核）与 M2（能力薄层）之间；目标 2026-11-07。
- 更新架构 §5/§11：`context_manage` 统一 `map/recenter/compress/edit/slider/decision_*`；`context_map` 作为 `mode=map`，不再单列工具。
- 状态文档：`CLI_PROJECT_INDEX` 新增 `DESIGN-DECISION-LEDGER-CONTEXT-EDITOR`（`current-design`）；`TODO`/`BACKLOG` 的 `0ds` 增加 M1.5；计数仍为 66（0ds 一项）。
- 落码前先定 `context_manage` schema 版本与 `.gsa/decisions` 文件格式；本设计不落码。
