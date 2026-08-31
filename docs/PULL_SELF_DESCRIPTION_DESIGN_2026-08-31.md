# PULL 自描述设计（blackboard_read 增量 + temporal 一次返回）

> 状态：`current-design`（2026-08-31 设计定稿 + S1 代码 / S2 测试完成 +
> **2026-08-31 审查处理**：M1 迁移基线独立化、M2 失败形状不挂头不推进、
> M3 actions 内容变化才计数、L3 点读字节口径等全部修复，见
> [`P2-11 PULL 自描述 S1 审查审计`](audits/P2-11_PULL_SELF_DESCRIPTION_S1_REVIEW_AUDIT_2026-08-31.md)；
> 实施为 `partial`——S3 重建 + S4 实机复验待放行，验证闭环后随既有纪律转
> `implemented`）；权威转录见
> ADR-0010 §14.48；裁决来源：MODEL_RESIDUAL_PRESSURE_DISCUSSION_2026-08-31 §8
> 第 1 项（P2-11 第 1 项，用户确认采用）。

## 1. 目标与约束

目标：`blackboard_read` 是模型唯一的按需信息面（PULL、零常驻 token），但存在
两个负担——① 模型"只会查明确需要的"，不会做辅助判断式的轮询（10 题小批
temporal 分区 0 查询、session 查询也稀疏）；② temporal 单次查询若只回一行，
模型常需二次查询补齐趋势/迁移信息。

本设计把 `blackboard_read` 响应改造为**自描述**：任何成功响应携带「自上次读取
以来」增量头（各分区变化计数 + temporal 域迁移摘要），temporal 单次查询按模型
意图尽量一次返回所需内容。让"查一个分区"顺带回答"还有没有别的值得查"，消除
为补齐信息而做的二次查询。

硬约束（沿用既有冻结边界）：

- 零注入：增量头只出现在模型**主动 PULL** 的响应内，不进系统提示、无主动注入、
  无新常驻 token；模型无感边界不变（模型面不新增任何机制）。
- 不干预模型怎么做：增量头只给**事实**（计数/迁移），不给建议、不推动作。
- 8 工具面冻结：不新增/删除/改名工具，只增强 `blackboard_read` 的返回内容。
- 有界：增量头自身 ≤256 B；temporal 分区整响应（增量头 + 查询体）≤1 KiB；
  其余分区沿用既有 ≤8 KiB structured 截断。
- live-only：增量头与读取游标是 run 级会话状态，不进 epoch 快照、不持久化；
  读归档 epoch（`epoch=n`）是历史视图，不挂增量头、不推进游标。

## 2. 分区版本计数（revision counters）

每个分区维护一个**单调递增的版本计数**（u64，run 内只增不减；`#[serde(skip)]`
不进任何序列化面）。每次**可见内容变化**（追加、覆盖、清空、替换）计 1 次。

| 分区 | 计数器归属 | 计数的变化点 |
|---|---|---|
| plan | `Blackboard.revisions.plan` | 计划轮换/同 epoch 修订、步骤状态迁移（mark_step_*）、交付状态行写入、epoch 快照恢复 |
| exec | `Blackboard.revisions.exec` | 结果/错误/观察/授权请求追加、轮换清空、快照恢复 |
| edits | `Blackboard.revisions.edits` | 编辑记录追加、轮换清空、快照恢复 |
| tool_actions | `Blackboard.revisions.tool_actions` | 工具动作记录追加、轮换清空、快照恢复 |
| actions | `ActionBoard.revision`（自含） | 注册板块替换、订单写入/消费、receipt 追加、轮换清空、快照恢复 |
| internal_ret / external_ret | `Blackboard.revisions.*` | `write_section` 全量覆盖、会话恢复灌回 |
| entities | `EntityRegistry.revision`（自含） | 实体登记/更新/诊断覆盖（insert / set_diagnostic） |
| session | 派生 = `tool_rounds`（已完成轮数） | 每完成一个工具轮 +1（与 session 面显示口径一致） |
| temporal | 派生 = `LifEngine.temporal.round()`（徽章）；迁移段基线 = `migration_count` | 每决策轮 +1；域迁移独立单调计数（`migration_count`，供迁移段增量基线，2026-08-31 审查处理 M1 双基线口径） |

实现纪律：追加类分区统一走 `Blackboard::push_edit / push_tool_action /
push_exec_result / push_exec_error` 方法（计数与写入同函数，杜绝漏点）；覆盖类
分区在既有方法内自增（`ActionBoard::set_registration/write_order/take_order/
push_result`、`EntityRegistry::insert/set_diagnostic`）；plan 的零散写点
（mark_step_* 调用处、delivery_status 写处）调用 `bump_plan()`。

## 3. 读取游标（read cursors）

控制器持有 run 级 `blackboard_read_cursors: Mutex<HashMap<section, u64>>`
（live-only，run 起始复位为空）。语义：

- 游标默认 0（= run 起点）；`delta(分区) = 当前版本计数 − 游标`（饱和减）。
  temporal 分区有**双基线**（2026-08-31 审查处理 M1）：round 徽章游标与迁移计数基线
  （`migration_count`）分存；读 temporal 时两者同时推进，读其它分区只推进该分区。
- **成功读取某分区后**推进该分区游标到当前版本计数；其余分区游标不动——
  即"未读徽章"模型：读 `tool_actions` 只清 `tool_actions` 的徽章，其它分区
  保持未读计数，直到被读取。
- 失败路径（exit_code 1 / 参数错误）不推进游标——模型没拿到内容不算读过。
- 读归档 epoch（`epoch=n`）不推进游标、不挂增量头。
- 平行调用边界：`blackboard_read` 在并行读集合内，两个并发同分区读取可能
  都读到同一增量、后到者推进游标；增量头是辅助信息，允许这种轻微竞态，
  登记为已知边界（不做串行化）。

## 4. 增量头格式（contract）

所有**成功的 live 读取**（`section=temporal|session|其余分区`）响应头部追加
一行（无增量且无迁移摘要时不追加，避免噪音）：

```text
[黑板增量] tool_actions+3 exec+1 session+12 域迁移+2: stuck→normal@r14
```

规则：

- 徽章按固定分区序（plan, exec, edits, tool_actions, actions, internal_ret,
  external_ret, entities, session, temporal），只列 `delta > 0` 的分区，
  格式 `<分区>+<n>`，空格分隔。
- `域迁移` 段：`temporal.migration_count` 超过**迁移计数基线**（上次成功读取
  temporal 时的 `migration_count`，2026-08-31 审查处理 M1——不复用 round 游标，
  两者刻度不同）时输出 `域迁移+<n>: <最近一次迁移 from→to>@r<轮>`（只带最近
  一次，防膨胀；n = 自上次读取 temporal 以来的迁移总数）。
- 增量头自身经 UTF-8 安全截断，上限 256 B（含截断标记）。
- 分区计数是"该分区自上次读取以来变化过多少次"，不是条目数（覆盖式变化
  如注册板块替换、计划步骤状态迁移都计 1）。

示例（模型先读 exec，期间发生 3 次工具调用与一次域迁移，再读 tool_actions）：

```text
[黑板增量] tool_actions+3 session+2 域迁移+1: normal→pressure@r12
== tool_actions ==
...
```

## 5. temporal 单次查询按意图一次返回

在既有四查询面（now / recent(k≤20) / history / feature(name, k≤20)，fires 永不
渲染，体 ≤1 KiB）之上做**一次返回**增强——每个查询自包含其意图所需上下文，
不让模型为补齐信息二次查询：

- `now`（当前状态）：保留当前行，追加**近 5 轮趋势行**（轮数 <2 时省略该行；
  实现按实际长度标注「近 N 轮」，示例中的 5 为上限值）
  （`近 5 轮: u_prog 0.9→0.2, u_err 0.5→2.5, u_stuck 0.1→5.0`）与**上一迁移
  行**（`上一迁移: normal→pressure@r12`，无迁移则省略）——回答"我现在处于什么
  域、往哪个方向走、最近一次域切换是什么"。
- `recent(k)`（最近 k 行）：首行追加压缩摘要
  （`近 k 轮: 域=stuck, 入域 r12, 驻留 3; u_prog 0.9→0.2`）——被 1 KiB 截断时
  摘要仍保住要点。
- `history`（域迁移日志）：维持现状（本就自包含）。
- `feature(name, k)`（单特征序列）：尾部追加当前值与域
  （`当前 u_prog=0.2 (域=stuck)`）——把"数值序列"和"当前含义"合并。

整响应（增量头 + 查询体）仍 ≤1 KiB：增量头先行，查询体沿用既有渲染截断，
host 层对合并结果做最终 UTF-8 安全截断。

## 6. 与既有契约的关系

- 不改 `blackboard_read` 参数 schema；工具描述补一句增量头说明（模型可预期
  响应首行形状）。
- structured 信封 `entries` 字段沿用既有 `enforce_bound`（temporal 1 KiB /
  其余 8 KiB）；增量头随 `content` 一并进入该字段（文本面完整优先；机器面
  entries 在 cap 内尽力一致——超限时机器面截尾属既有有界语义，2026-08-31
  审查处理 L2 措辞收窄）。
- 事件面（ToolCompleted）不新增字段；`tool_actions` 仍按既有纪律记录每次
  `blackboard_read` 调用（含失败路径）。

## 7. 已知边界（登记，不阻塞）

- 子代理与主代理共享同一控制器；若子代理调用 `blackboard_read` 会共享游标
  （子代理工具投影面不含 `blackboard_read`——投影测试锁定，检索车道提示词
  亦不包含该工具，运行中不触发；登记为防御性边界，2026-08-31 审查处理 L5
  措辞修正）。
- 增量头计数口径是"run 内自上次读取以来"；跨 run（多 prompt 会话）游标随
  run 复位，续 run 首次读取会把既有分区内容计为未读（符合"该 run 尚未读过"
  语义，且计划轮换/恢复会真实触发变化计数）。
- 并行同分区读取的游标竞态（见 §3），增量头为辅助信息，允许轻微重复/遗漏。

## 8. 测试矩阵（S2）

| 项 | 覆盖 |
|---|---|
| 版本计数 | push_* 追加计 1；rotate 轮换计 1（plan/exec/edits/tool_actions/actions）；快照恢复计 1；ActionBoard 四方法各计 1；EntityRegistry 登记/更新计 1 |
| 迁移计数 | temporal.migration_count 跨迁移单调；restore_spikes 重建计数 |
| 增量头 | 首次读取显示全未读徽章；读某分区后该分区徽章清零、其余保留；无增量时不追加头；域迁移段格式与轮号 |
| temporal 一次返回 | now 含近 5 轮趋势行与上一迁移行；feature 含当前值/域尾注；recent 含压缩摘要 |
| 有界 | temporal 整响应（含增量头）≤1 KiB；增量头 ≤256 B（含全分区大徽章 + 迁移段的独立截断测试，审查处理 N1）；UTF-8 安全截断 |
| 迁移基线 | 读 temporal → 新迁移 → 读其它分区仍显示准确的「域迁移+n」；读 temporal 后双基线清零（审查处理 M1 回归） |
| 回归 | 四查询面标签/自描述测试、fires 不渲染、1 KiB 截断测试保持绿 |
