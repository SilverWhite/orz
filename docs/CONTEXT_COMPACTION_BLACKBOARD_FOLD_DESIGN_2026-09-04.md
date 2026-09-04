# ORZ 上下文压缩 marker 改为黑板折叠视图快照（2026-09-04）

> 状态：`design-final`（v0.2，2026-09-04；用户逐项裁决 R1–R5 无异议，
> 总量 20K 定档；2026-09-04 第三条路放行 + **S1 收口**：主会话压缩转
> v0.3 折叠快照（A–E 装配 + r_keep 接线 + §7 矩阵 1–7 落地）；检索/
> grill 车道与消息无轮章的旧会话按 §6 车道范围裁决回退 v0.2 五段模板；
> **S2 收口（2026-09-04）**：压缩 e2e 全串行绿——rhythm / fallback /
> session_end / 恢复预检（§7 矩阵第 8 项复验），见 §9）。
> 本文只登记机制与裁决记录，不改变任何既有生产语义；替换对象 =
> `CONTEXT_COMPACTION_DESIGN` §4 五段模板（含 marker / 存档 / 机械校验
> 口径）。
>
> 相关权威现状：黑板会话作用域 + 渲染折叠（ADR-0010 §14.52 /
> `BLACKBOARD_CONVERSATION_SCOPE_FOLD_DESIGN_2026-09-03.md`，B1–B3 已
> 实施、B4 待续）；压缩机制 = FUS-COMPACTION-REDESIGN（
> `CONTEXT_COMPACTION_DESIGN_2026-08-14.md`，v1.15 起压缩不触碰黑板）。
>
> 背景问题（详见 §0）：五段槽在会话作用域黑板下的数据源口径未随
> plan-epoch 退役而重新裁决——路径槽按插入序取最老 40 条且生产面溢出
> 指针不可回查、failure_agg 无重置跨 marker 重复、目的/计划恒空；文档
> 校验口径仍停留在「本 plan epoch 增量」。
>
> v0.2 修订（2026-09-04 用户裁决，无异议）：§8 开放裁决项 R1–R5 全部按
> 推荐收口；marker 总量上限定档 20_000 字符；`SUMMARY_MARKER_ESTIMATE_
> TOKENS` 随动重校准（先按 ≈11_000 起步，S4 实测后定档）。本文自本版起
> 为定稿（实施未放行），ADR-0010 §14.54 / CLI_PROJECT_INDEX / BACKLOG
> P2-14 / TODO P2-14 随本版同批登记。

## 0. 目标与病灶

- 黑板侧已定案：保留全量记录、不做存储压缩；blackboard_read 按「域 → 轮
  」渲染折叠（默认展开 = 当前域段 ∪ 最近 K=10 轮 ∪ 最近 20% 行；T=64K
  字符/分区、W=10MiB；行级 200 字符、分区 50 行/4K 渲染 cap）。机械消费
  方读存储结构、不经渲染。
- 压缩侧仍是五段模板，且三处与上述定案冲突：
  1. 路径槽 `edits.iter().take(40)` 取**最老 40 条**（插入序）——会话内
     多次压缩只重复最老内容，最近编辑永远进不了 marker；
  2. 溢出指针生产面回退到本次摘要存档，但存档不含被挤出的路径 → 指针
     不可兑现（原 holder = epoch 快照已退役）；
  3. failure_agg / edits 的清空只发生在 `--plan` 轮换，生产面随会话单调
     累积、压缩不重置 → 注意事项槽跨 marker 重复（原「同一 epoch 内多次
     压缩重复」边界变成常态）。
- 目标：把压缩 marker 从「语义五段槽」改为「压缩点冻结的黑板折叠视图
  快照 + failure_agg 聚合块 + 显式查询指针」，机械、零模型、有界，并与
  blackboard_read 同源。

## 1. 机制总述

- 压缩触发、drain、保留尾机制**全部不动**（本稿不改 `recent_tail_rounds`、
  触发阈值、缩减守卫、存档与事件面）。
- 唯一改动 = marker 内容生成口径：在压缩执行点，对 live 黑板生成一张
  **冻结的折叠视图快照**，输入范围限定为「round < 保留尾首轮」的行
  （即从消息中被 drain、但保留尾未覆盖的那部分；保留尾本身原样留在消息
  里，不进 marker，避免与消息重复）。
- marker 内部结构 = 框架行 + 近窗明细块 + 旧段聚合块 + 失败目标聚合块 +
  查询指针行（§2）；旧区超出聚合上限的部分**省略，只留显式查询指针**。
- 生成与 blackboard_read 折叠渲染**同源**：复用 `render_fold` 的域段切分 /
  展开子集 / 标注行渲染 / 行截断口径，只是加「round 上界过滤」与
  「块级字符上限」两个参数；保证 marker 快照与压缩后模型主动
  blackboard_read 看到的是同一套标注语言。

### 1.1 术语

| 术语 | 定义 |
|---|---|
| 保留尾（recent tail） | 压缩后原样保留在消息里的最新轮；现状 = 最近 2 个完整轮（`recent_tail_rounds=2`，不完整轮回溯保留 → 实际 ≥2；ledger fold 已触发时 = 折叠桥 `fold_cut`，预算 8K） |
| r_keep | 保留尾首轮（第一条原样保留消息所属的 LIF 会话轮）——排除边界 |
| 近窗 | blackboard_read 折叠默认展开子集：当前域段 ∪ 最近 K 轮 ∪ 最近 20% 行 |
| 近窗明细块 | 近窗 ∩（round < r_keep）的真实行（保留尾的行不重复进 marker） |
| 旧段聚合块 | 近窗外、round < r_keep 的行按「域 → 轮段」聚合的标注行 |
| 省略区 | 超出聚合上限的旧段——只留查询指针 |

## 2. marker 结构与块定义（草案 v0.3）

### 2.1 总体形态

```text
[前文上下文已压缩 v0.3]
摘要 ID: compaction-{run}-{seq}
被压轮次: N 轮（会话轮 r{from}–r{to}，保留尾首轮 r_keep={n}）
摘要存档: {path}      摘要 digest: sha256:{digest}
黑板会话: {session_id | （无）}

== 近窗明细（round < r_keep，已排除保留尾） ==
{分区名 + 行；空则「（无）」}

== 旧段聚合（≤30 条标注行；空则「（无）」） ==
[域段 normal r1–r30 · 123 条 · f0.py…]
…
（其余 N 段见 blackboard_read section=…）

== 失败目标聚合 ==
（无）或 F4 聚合行（≤3K；溢出随存档补全段）

== 查询指针 ==
回查：blackboard_read section=exec|edits|tool_actions，先看域段标注，
再带 domain + round_from/round_to 精确展开；pre-stamp 旧行用
since/receipt_id 展开；更早历史见摘要存档与 run journal。
[/前文上下文已压缩]
```

### 2.2 块定义与预算（建议值，待裁决，见 §8）

| 块 | 内容 | 来源 | 建议上限 |
|---|---|---|---|
| A 框架 | ID / 被压轮次与轮区间 / r_keep / 会话 / 存档路径 + digest / 守卫或存档失败注记 | 机械字段 | 600 字符 |
| B 近窗明细 | 近窗 ∩（round < r_keep）的真实行；分区顺序 exec → edits → tool_actions（与折叠渲染链序一致）；行级沿用 200 字符截断、分区沿用 50 行/4K cap；块级再封顶 | live 黑板折叠分区 | 8_000 字符 |
| C 旧段聚合 | 近窗外、round < r_keep 的行按域段聚合标注（`[域段 {domain} rX–rY · N 条 · 预览]`），复用 segment_runs + failure_agg/路径截断预览；**取最接近近窗的 30 条**（渲染序尾部），更早省略 | live 黑板折叠分区 | 30 条 × 200 字符 + 溢出指针行 |
| D 失败目标聚合 | F4 失败目标聚合行（failure_agg 非 blackboard_read 查询分区，必须随 marker/存档携带） | `failure_agg.rows` | 3_000 字符 + 存档补全段 annex（既有机制保留） |
| E 查询指针 | 省略区/旧区的显式查询指令：section、先看标注、domain + round_from/round_to；pre-stamp 用 since/receipt_id；存档与 journal 兜底 | 机械模板 | 1_000 字符 |
| 合计 | — | — | ≤ 20_000 字符（定档；原 17K 上调，估算常量随动） |

- 空块统一渲染「（无）」（沿用 P2-13 D4 空槽口径，只对真实存在内容渲染）。
- 块 B 与块 C 互补不重叠：B = 展开子集内的真实行，C = 展开子集外的标注
  行——与 blackboard_read 折叠视图的内部结构完全一致；因此 B+C 合计
  「近窗明细 + 旧段标注」在裁剪前即一张完整的折叠视图快照。

## 3. 选区规则

### 3.1 排除边界 r_keep

- r_keep = 保留尾首轮：压缩执行点实际 drain cut（`fold_state.fold_cut`
  或 `collapsed_cut(messages, recent_tail_rounds)`）之后第一条保留消息
  所属的 LIF 会话轮。
- marker 行排除规则：`round >= r_keep` 的行不进 marker（它们对应的消息
  已原样保留，重复只会浪费 token）。
- **S1 前置验证**：确认 LIF 决策轮与消息轮同一 cadence（每完成一轮 +1）。
  若一致，r_keep 可由调用点 LIF 快照与保留轮数推算；若不一致，则在写入
  点给消息/行补同一轮章（否决并行索引，沿用 B1 结构化字段纪律）。
- **S1 验证结论（2026-09-04 第三条路裁决，已落地）**：不一致成立——检索
  子车道决策计入共享 LIF（主 1 + 子 2 = 3 的锁定测试为证），消息轮 ≠ LIF
  轮；落地 = 消息补轮章（仅 Main 车道 assistant 声明消息）+ 主车道决策轮
  执行窗 pin，共享折叠行 / dispatch 镜像 / DispatchStamp / disposition
  audit mirror 统一盖执行窗主轮章（effective_blackboard_stamp）。
- pre-stamp 旧行（round=0/domain=None）不满足轮过滤：一律归旧区，不进
  块 B；只能经 `since`/`receipt_id` 展开，块 E 指针显式说明。

### 3.2 近窗明细（块 B）

- 对 exec / edits / tool_actions 三个可折叠分区，以压缩点的 LIF
  (round, domain) 为快照上下文，计算折叠默认展开子集（当前域段 ∪ 最近
  K 轮 ∪ 最近 20% 行），再过滤 `round < r_keep`。
- 与 blackboard_read 的差异仅两点：① 固定按折叠视图生成（不依赖 T/W 是
  否已达阈值，保证有界）；② 过滤保留尾行。
- 未达折叠阈值的会话同样走本口径（强制折叠渲染），避免 marker 无界。

### 3.3 旧段聚合（块 C）

- 展开子集外的旧行按既有 `segment_runs` 语义聚合为标注行（逐段独立、
  时间序、pre-stamp 独立段）。
- 行内预览复用 failure_agg 行渲染与路径截断口径；标注行自带轮区间 =
  天然展开索引。
- 上限 30 条：取**最接近近窗的 30 条**（确定性），更早段省略；溢出指针
  行给出剩余段数 + 查询方式。

### 3.4 失败目标聚合（块 D）

- `failure_agg.rows` 全量按序渲染 ≤3K；溢出随本次摘要存档补全段保存
  （既有 `failure_annex` 机制原样保留）——保证「其余 N 条见 … 摘要存档」
  指针可兑现。
- 跨 marker 重复问题随 rolling 单 marker 自动消失（压缩删除旧 marker）；
  本稿不引入轮水位去重（见 §8 裁决项 4）。

### 3.5 省略与查询指针（块 E）

- 块 C 超限段、块 B/C 未覆盖的非折叠分区（plan/entities/检索分区）一律
  不进 marker；块 E 用模板 + 生成行给出显式指令，不写空洞指针：
  - 折叠分区旧内容：`blackboard_read section=…` 先看标注，再
    `domain + round_from/round_to` 展开；
  - pre-stamp：`since` / `receipt_id`；
  - 兜底：本次摘要存档与 run journal。

## 4. 预算与常量（v0.2 定档）

- marker 总量上限由 17K 上调至 **20_000 字符**（A+B+C+D+E，定档）；
  `SUMMARY_MARKER_ESTIMATE_TOKENS` 由 9_000 重校准（先按 20K/2 + 余量
  ≈ 11_000 起步，S4 实测后定档）；缩减守卫用估算常量随动。
- 新常量（编译期默认 + env 覆盖，沿用 ORZ_* 模式，默认值定档）：
  - `ORZ_COMPACTION_SNAPSHOT_DETAIL_CHARS`（块 B 预算，默认 8_000）
  - `ORZ_COMPACTION_SNAPSHOT_SEGMENT_LINES`（块 C 条数，默认 30）
  - `ORZ_COMPACTION_MARKER_TOTAL_CHARS`（总上限，默认 20_000）
- 分区块级超限一律「截断 + 指针」，绝不小 block 静默丢失；块 B 行级 /
  分区级沿用 blackboard_read 折叠渲染既有 cap（200 字符 / 50 行 / 4K）。

## 5. 与既有机制的关系（不变项）

| 机制 | 状态 |
|---|---|
| 压缩触发（192K rhythm / 200K fallback / session_end / 恢复预检） | 不变 |
| drain 与保留尾（`recent_tail_rounds=2`、完整轮纪律、fold_cut 优先） | 不变 |
| 缩减守卫 / min_rounds / 存档恒写入 + digest | 不变（估算常量随新预算重校准） |
| rolling 单 marker + restore 保留（`[前文上下文已压缩` 前缀） | 不变（版本号升 v0.3） |
| `context_compressed` 事件 v0.2 字段 | 不变（不含槽文本） |
| `--plan` 诊断域与 epoch 归档读 | 不变（快照只取 live 面） |
| 压缩不触碰黑板（v1.15 纪律） | 不变——快照只读 |
| 五段槽 / 17K 合计机械校验 | **替换**为块级校验（§7） |

## 6. 同源渲染与函数面（实施影响）

- `render_fold` 快照入口（纯函数，S1 落地）：`epoch.rs`
  `render_exec/edits/tool_actions_snapshot` = r_keep 上界过滤 + 强制折叠
  + pre-stamp 一律归 C；`select_annotations_closest_to_window` = 最接近
  近窗 ≤N 条（确定性）+ 溢出计数；装配层按 §2.2 块预算执行 B 明细
  50 行/4K 视图 cap 与 A/C/D/E 块级封顶——快照入口只保证同源候选行/标注，
  块级有界是装配层（summary.rs v0.3）的合成性质（2026-09-04 复审处理
  口径修正）。blackboard_read 路径不受影响（默认行为逐字节不变）。
- **车道范围裁决（2026-09-04 复审处理）**：v0.3 折叠快照 marker 只用于
  主会话压缩；检索/grill 车道会话按自己的消息轮次 drain、行轴不适用 →
  其压缩与消息无轮章的旧会话一起回退 v0.2 五段模板（既有语义不变）。
  主会话 `Message.round` 只盖 Main 车道声明消息。
- `run_template_compact` 调用点传入压缩点 LIF (round, domain)
  （`SharedLoopServices` 无 LIF 引用 → 调用方传 `FoldSnapshotCtx`），
  r_keep 在 drain/fallback 后由保留尾首条声明消息轮章得出；槽构建移到
  drain 后（S1 落地）。
- `summary.rs`：主会话 v0.3 = §2.2 A–E 块结构（A 600 / B 8K / C 30 /
  D 3K / E 1K，总量 20K + env 覆盖）；v0.2 五段结构保留为车道/旧会话回退
  路径（不退役其代码面）；存档 markdown 与 `failure_annex` 补全段同步。
- 标记前缀不变（`CONTEXT_COMPRESSED_PREFIX`），版本号 v0.2 → v0.3；
  旧 v0.2 marker 文本在恢复路径继续作为普通文本保留，不要求解析兼容
  （前缀级识别不受影响）。

## 7. 校验与测试影响

- 机械校验新口径：块齐全且每块 ≤ 上限、合计 ≤ 上限、B/C 行来源 =
  live 黑板且 `round < r_keep`、digest 绑定、C 溢出指针存在且可回查。
- 同源一致性测试：同一黑板上，快照入口输出与 blackboard_read 折叠渲染
  （相同 LIF 上下文、去掉 r_keep 过滤）共享分段/展开子集/标注词汇/行
  截断；按 marker 块级结构分区呈现（B 明细块 + C 聚合块），不承诺视图
  穿插位置的逐字节一致（部分展开段标注行落聚合块内——块级结构差异，
  2026-09-04 复审处理口径修正）。
- 单测矩阵：
  1. r_keep 排除：保留尾行不进 marker；
  2. pre-stamp 行归旧区、不进 B；
  3. C 超过 30 段 → 取最近 30 + 溢出指针；
  4. 未达 T/W 阈值也强制折叠且有界；
  5. B/C/D 全空 → 「（无）」；
  6. 20K 总量封顶不静默丢失；
  7. 存档 annex（D 溢出）与旧机制一致；
  8. 恢复预检后 marker 仍在、内容原样。

  S1 落地登记（2026-09-04）：1–7 项已有快照/装配/e2e 测试覆盖（epoch.rs
  快照与选择器测试、summary.rs v0.3 装配测试、compact.rs 主车道 e2e v0.3
  断言）。第 8 项由 **S2 恢复预检 e2e 复验收口（2026-09-04）**：恢复预检
  整轮机械截断后 v0.3 marker 仍在、内容逐字节原样（preamble 恒保留；
  截断 marker 追加其后，D3-1 write-back 均存活），证据见
  [P2-14 S2 e2e 审计](docs/audits/P2-14_S2_E2E_2026-09-04.md)。

## 8. 裁决记录（v0.2，2026-09-04 用户逐项裁决，无异议）

1. **R1 r_keep 口径 —— 定案**：按实际 drain cut 的保留尾首轮；消息轮与
   LIF 轮 cadence 一致性为 S1 前置验证（§3.1），不一致时在写入点补同轮
   章；pre-stamp 行归旧区，不经 domain/round 展开。
2. **R2 块 B 覆盖 —— 定案**：近窗明细包含「当前域段」中的连续旧行（与
   blackboard_read 同源，靠 50 行/4K cap 有界）；20% 放宽行参与过滤，
   过滤条件只有 `round < r_keep`。
3. **R3 块 C 定义 —— 定案**：30 条 = 段标注行上限，取最接近近窗的 30
   条，更早省略 + 指针；保留独立 failure_agg 块 D（failure_agg 非
   blackboard_read 查询分区，必须随 marker/存档携带）。
4. **R4 预算 —— 定案**：marker 总量上调至 20_000 字符；env 常量命名与
   默认值按 §4（`ORZ_COMPACTION_SNAPSHOT_DETAIL_CHARS`=8_000 /
   `ORZ_COMPACTION_SNAPSHOT_SEGMENT_LINES`=30 /
   `ORZ_COMPACTION_MARKER_TOTAL_CHARS`=20_000）。
5. **R5 五段退役与 ADR 转录 —— 定案**：本稿随定稿取代
   CONTEXT_COMPACTION_DESIGN §4 与 ADR-0010 §14.10③/§14.29/§14.30、
   P2-12 注意事项槽、P2-13 D4 的相关语义；取代关系登记 ADR-0010
   §14.54（实施放行后转正），索引/BACKLOG/TODO 随本版同批登记。

## 9. 验证计划（定稿放行后）

- S1（2026-09-04 完成，全面复审处理收口）：r_keep 接线验证（cadence
  锁定 + DispatchStamp 同轴断言）+ render_fold 快照入口纯函数 + v0.3
  A–E 装配 + run_template_compact 接线 + §7 矩阵 1–7。证据：orz-loop
  lib 720 passed / 0 failed / 3 ignored、fmt/diff 净、orz-host ACP 43
  项全绿；复审证据见
  `docs/audits/P2-14_S1_REVIEW_HANDLING_2026-09-04.md`。
- S2（2026-09-04 完成）：压缩 e2e 全串行绿——同一主会话一次 run 真实
  触发 rhythm → fallback（绕冷却 ×2）→ session_end（全机械零模型调用、
  滚动单 v0.3 marker 逐请求与收尾会话断言），下一 run 恢复预检越过触发
  时整轮截断、v0.3 marker 保留且内容逐字节原样（矩阵第 8 项复验）。证据：
  orz-loop lib 722 passed / 0 failed / 3 ignored、orz-host ACP 43
  passed、fmt/diff 净；审计见
  `docs/audits/P2-14_S2_E2E_2026-09-04.md`。
- S3：Linux musl 三件套重建与核证（沿用 ORZ-BUILD-MOUNT-001 契约）。
- S4：实机复验 + 遥测：marker 实际字符分布、块 B/C 溢出频率、压缩后
  blackboard_read 跟随调用频率（验证块 E 指令是否真的被模型使用）、
  restore 后 marker 可用性。

## 10. 非目标（红线）

- 不恢复 LLM/语义摘要；不新增模型调用。
- 不改黑板存储与渲染折叠本身；不延长保留尾（本稿只改 marker 内容生成）。
- 不把折叠触发绑定压缩阈值（D3 保持）。
- 不新增压缩轮水位/增量去重（rolling 单 marker 已消除跨 marker 重复；
  若 S4 显示需要，另立项）。
