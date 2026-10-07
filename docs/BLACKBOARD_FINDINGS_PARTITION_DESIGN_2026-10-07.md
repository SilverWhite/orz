# 0cu 设计稿：黑板模型写入面第三分区 `findings`——已探明内容工作台账（BLACKBOARD-FINDINGS-PARTITION）（2026-10-07）

> **编号**：0cu（BACKLOG/TODO P1；2026-10-07 用户裁决立项，61 → 62，203 批）。
> **线路归属**：黑板模型面线（0ae D0 写入面 → 0aj 写权限放行 → 0ck 注解 → 0cl 瘦身 → 本项），单独立项单独批。
> **状态**：`current-design`（S1 设计定稿＝本批；S2 落码待放行）。
> **来源（用户令，2026-10-07）**：「模型对plan区的使用更多是结论记录的话，那模型就是明确需要这一
> 部分的，我考虑应该且值得加第三变体，具体实现和plan一致即可，直接复制粘贴，不膨胀实现方式和实现
> 机制，这一块新分区形式让模型自由写入，我们只是提供多一个分区来让模型能够更准确清晰的记录其想要
> 记下的内容」；分区名授权：「选最具有通用性即可，要让模型尽可能用得顺手」；并裁决同意「D1 计数
> 不含新区」与「文案同步走 0cl 纪律」两点，且明确「要单独立项并落设计文档，毕竟是新的分区补充」。

---

## §1 背景与需求依据（2026-10-07 主会话实测）

`blackboard_write section∈{plan, notes}`（0ae D0，DP-6）现行只有两个模型可写分区。对
`.gsa/archives/` 全部 29 个会话存档（2026-09-16～09-27 狗粮长轮等）实跑统计：

- **17/29 会话有模型笔记写入**，合计 **226 条 ≈ 338.7K 字符**——`plan` 区 86 条/145,488 字符、
  `notes` 区 140 条/193,245 字符。
- **plan 区被「已探明结论记录」语义实际占用**：峰值样本 `6ab139e6`（0bz 上下文脸面勘定轮）
  plan 区 29 条/80.5K 字符，内容为逐项勘定结论而非前瞻计划；`6ab00c8a` plan 6 条/8.8K、
  `6ab15969` plan 12 条/11.7K 同形。
- notes 区峰值 `6ab6dc02` 34 条/51.1K（自由随记）、`6ab15969` 18 条/18.2K。
- journal 旁证：run `RUN-CLI-6aaad7c8`（0aj 收口取证）`blackboard_write` ×8＝plan 2／notes 6；
  2026-09-22 处理批 run 单轮 `blackboard_write` 参数总量 18.2KB（对照 `search_replace` 108.1KB）。
- 压缩白名单块（A6 §8 C.2，挂 `context_compress`）同期真实使用＝7 会话各 1 块、每块 1 条合并
  条目（411B～10,083B）——模型在压缩时写「一条大而全的保命摘要」，与本项互补而非替代。

**结论**：模型对写面的使用高频且已成习惯；「实际工作中已确立的发现/结论」目前只能挤进 plan
（前瞻语义被侵蚀）或 notes（过程随记语义混淆）。数据支持增设独立分区，且模型会自发使用。

## §2 用户裁决（定案冻结，2026-10-07）

1. **加第三变体**：`ModelNoteSection` 由 {plan, notes} 扩为三值。
2. **机制照抄、零膨胀**：实现与 plan/notes 逐字同构（复制粘贴级），不引入任何新机制、新工具、
   新触发、新聚合档位。
3. **模型自由写入**：只提供分区，不配任何「何时该写」的提示规则或触发逻辑（与 P9 精神一致）；
   描述文案只解释分区用途，用不用、写多少、写什么由模型自决。
4. **D1 计数不含新区**：`model_note_count`（notes＋plan.model_notes）保持原样——findings
   不抵扣 0ae D1 首轮 plan 问询补救规则的 plan 责任。
5. **文案同步走 0cl 纪律**：工具描述/参数描述/guide 只加最小短句，不回涨瘦身成果。
6. **分区名授权主会话定**，取模型最顺手的最通用词——本稿定案 `findings`（§3）。

## §3 分区语义与命名定案

### 3.1 三分区语义（写进文案的方向性口径，不强制）

| 分区 | 语义 | 时间结构 |
|---|---|---|
| `plan` | 工作计划与关键中间结论（前瞻＋计划状态） | 追加，随 plan 视图尾段渲染 |
| `notes` | 自由工作笔记（随记、草稿、过程性） | 追加，时间正序 |
| `findings` | **已探明内容工作台账**——实际工作中已确立/已验证的发现与结论 | 追加，时间正序 |

三分区皆为模型自有、同权：分区只是「归档位置」自由，重叠写入不拦不劝（例如 findings 与 plan
的「关键中间结论」天然相邻，模型自选落点）。文案不使用「必须」「应当」句式。

### 3.2 命名候选比较与定案

| 候选 | 判定 |
|---|---|
| **findings** | **定案**。英文「（调查/工作中）得出的发现」标准词，审计/研究语境模型极熟（本项目主场景）；与 notes（随记）、plan（前瞻）三分清晰；短、enum 值稳定 |
| facts | 泛化歧义（环境事实？输入事实？），不带「工作过程得出」语义 |
| log / worklog | 与模型面既有 `journal`（定位符点读）和机械 `exec` 工作记录撞车，模型易混淆记录层级 |
| results | 与工具结果/执行结果语义撞车 |
| knowledge | 过泛，暗示跨会话长期记忆——黑板是会话作用域，名字不得误导 |

定案：**`findings`**（小写、单数形；与 plan/notes 同风格）。

## §4 机制设计（照抄 plan/notes，零新机制）

- **写入**：`blackboard_write section="findings"`，`content` 单条 ≤8K 字符（同一 `maxLength`
  机械钳制）；写时盖 (round, domain) 章＋`ts` 墙钟，`NoteEntry` 复用；`plan_write` 事件出账
  （`section:"findings"` 自由文本直接通过，schema 零改）；ReadOnly 类全策略自动放行（同工具同
  arm，权限面零改）。
- **存储**：黑板根新增 `findings: Vec<NoteEntry>`（serde(default)）——与 notes 同构（plan 的
  model_notes 在 PlanSection 内，机制逐字相同仅归属结构不同；取 notes 形态避免触碰 PlanSection）。
  `PartitionRevisions` 新增 `findings` 字段，`push_model_note` 新增一臂，版本计数同一落点。
- **读取**：`blackboard_read section="findings"`——照抄 notes 渲染（时间正序、逐条盖章头、
  空分区＝「（无）」）；**不进折叠展开集**（折叠展开仅 exec/edits/tool_actions，notes 即不在集
  内，findings 同）。`section=plan` 读取的模型笔记尾段只渲染 plan.model_notes，不注入 findings
  （零交叉、零注入）。
- **PULL 增量头**：`partition_revisions()` 固定序追加 `("findings", …)` 一条——未读徽章自动
  生效，读后清零；256B 头帽不变（+1 分区 ≈ +十余字节）。
- **水位**：`live_compact_bytes()` 走 serde 全量序列化，findings 自动计入 10MiB 软水位（会话
  疲劳提醒面现成，零新增）。
- **epoch 快照/恢复**：`epoch_snapshot`/`restore_epoch_snapshot`/`EpochSnapshot` 各 +1 字段
  （随 notes 同点）；会话快照 `restore_conversation_snapshot` 的 PartitionRevisions 归位 +1
  字段（=1）。
- **压缩折叠快照**（AUTH-COMPACTION-FOLD-SNAPSHOT v0.3）：findings 行与 notes 行同规则进入
  近窗明细行池（(round,domain) 盖章行同源渲染），**不设单独聚合档位**。黑板本体不随压缩消失的
  既有性质（fold-proof durable memory）对 findings 自然成立，无需任何新机制。
  〔**§4.1 勘误（2026-10-07，209 审查处置批）**：本条首句与 v0.3 上游设计不符——B 近窗明细池
  按其设计表只含折叠分区 `exec → edits → tool_actions`，notes 行从来不在池内（与本稿 §6
  「折叠展开集不含 notes 类分区」同口径）。findings 与 notes 同为**非折叠分区**：marker 不携带、
  压缩后经 `blackboard_read section=findings` 恢复（折叠快照 E 查询指针「plan/actions/…等
  非折叠分区不在 marker 内，直接 blackboard_read 对应分区」措辞覆盖），fold-proof 性质照旧
  自动成立。S2 实现（零 compact/summary 触点）结果正确，本句不作为触点依据；203 批档 §2.2
  「压缩折叠行池全部随动 +1」同此勘误。〕
- **与压缩白名单块的关系**：不动。模型压缩时仍可自行把 findings 精华浓缩进 `context_compress`
  whitelist（常驻前言）——两机制互补（台账在板、保命在前言），无耦合。

## §5 触点清单（2026-10-07 时点行号；S2 落码批先复核漂移）

**写入侧**
1. `orz/crates/orz-loop/src/blackboard.rs:216-237`——`ModelNoteSection` +`Findings` 变体、
   `as_str`、`parse` 各一臂。
2. `blackboard.rs:726-735`——`PartitionRevisions` +`findings` 字段。
3. `blackboard.rs:788-799`——`push_model_note` +1 arm（落 `self.findings`＋bump 版本）。
4. `blackboard.rs:801-804`——`model_note_count` **保持不动**（裁决 4）。
5. `blackboard.rs:905-914`——`partition_revisions()` 固定序 +1 条。
6. `controller.rs:3820`——`blackboard_write` 描述 +1 短句（`or "findings" (established
   findings & conclusions from actual work — an append-only work ledger)`，0cl 纪律）。
7. `controller.rs:3826-3827`——write 参数 section enum +1 值＋描述 +1 分句。

**读取侧**
8. `controller.rs:3723-3741`——`blackboard_read` section enum +1 值。
9. `controller.rs:3687` 邻域——read 描述分区清单 notes 行后 +1 行单句。
10. `blackboard.rs:808-810` 邻域——`render_findings_section`（照抄 `render_notes_section`）。
11. `orz/crates/orz-loop/src/host_exec/tool_run.rs:2665-2668`——派发 +1 臂（照抄 notes 臂）。

**快照/恢复管道**
12. `blackboard.rs:746-758`——`epoch_snapshot` +1 行。
13. `blackboard.rs:763-775`——`restore_epoch_snapshot` +2 行（恢复＋版本计 1）。
14. `blackboard.rs:849-880`——`restore_conversation_snapshot` PartitionRevisions 归位 +1 字段。
15. `epoch.rs` `EpochSnapshot` 结构 +1 字段（serde default，旧快照零迁移）。

**文案面**
16. `controller.rs:1940`——guide 说明书「plan/notes 模型可写」→「plan/notes/findings 模型可写」
    （一词，0cl 纪律）。

## §6 零改动面（2026-10-07 逐一核实；**S2 落码批勘误一处**——见 §6.1）

- `runtime/run-event-v0.2.schema.json`：`plan_write` **信封**（envelope）无 section
  闭枚举（全文件 0 处 section 字样）——**零改**。
- **§6.1 勘误（204 批 S2 落码时复核发现）**：`runtime/plan-write-event-payload-v0.2.schema.json`
  **payload** 的 `section` 字段实有闭枚举 `["plan","notes"]`（0ae D0 增量；本稿 §6
  初稿只核了信封文件、漏核 payload 文件）——findings 进体需 **payload schema 枚举 +1 值
  + `immediate_delivery.rs` 枚举同源钉同步**（批内已落，随批验证）。判官不校验
  section 值、fixtures 不携带 section，均不受影响；旧 journal 回放零影响（枚举加宽向后兼容）。
- `orz-host/src/permission.rs`：同工具 ReadOnly arm——**零改**。
- 探针表 `WORK_TOOLS`（Rust 两表＋Python `_WORK_TOOLS`）：同工具不新增——**零改**。
- 折叠展开参数集（exec/edits/tool_actions）：findings 不入集——**零改**。
- 工具面计数：不加工具（8→9 冻结例外与 0ap 第九工具口径均不动）——**零改**。

## §7 兼容与迁移

- 旧黑板/旧会话快照无 `findings` 字段：serde(default) 反序列化为空 Vec——**零迁移**。
- 旧 journal 只读回放：plan_write 事件无新校验——**零影响**。
- 在役载体 0.8.14 不含本面；S3 载体重建前进体，与惯例同。

## §8 钉子判据（S2 落码批，先红后绿）

1. **表级**：`parse("findings")`→Some；非法 section（如 "finding"/空串）仍 None（负例）。
2. **端到端**：`blackboard_write(section=findings)` → permission allow → `plan_write
   {section:"findings"}` 出账 → `blackboard_read(section=findings)` 读回逐字一致。
3. **徽章**：写后 `partition_revisions()` findings +1；PULL 增量头带 findings 徽章；读后清零。
4. **渲染**：空分区「（无）」；多条时间正序盖章头；`section=plan`/`notes` 读取不含 findings
   内容（零交叉负例）。
5. **快照**：epoch 快照含 findings；restore 后 `revisions.findings == 1`；会话快照归位同。
6. **D1 不含**：`model_note_count` 不随 findings 写入增长（断言）。
7. **水位**：findings 写入后 `live_compact_bytes()` 增长（serde 自动性的护栏断言）。
8. **文案**：write/read 描述与 guide 含 `findings` 字样各恰一处（0cl 瘦身不回涨的护栏）。

## §9 边界与不做清单

- **不配提示规则/触发逻辑**：无「每当探明就提醒写入」式外部规则（P9 精神）；描述文案只教学用途。
- **不常驻注入**：findings 仍是 PULL-only；压缩后模型经压缩快照聚合段＋主动读取可见，不自动进
  常驻模型面（常驻语义属白名单块，两者不合并）。
- **不动 plan 描述**：plan 现文案「工作计划与关键中间结论」保持原样（三分区语义自由、不收窄）。
- **不设聚合档/不设 TTL/不做分区轮换**：会话作用域、全量保留照黑板既有性质。
- **0cl 瘦身成果不回涨**：全部门案文案增量 ≤3 短句（§5-6/7/9/16）。

## §10 批序

- **S1（本批，203 批）**：设计定稿＋立项落账（BACKLOG/TODO/第二卷/索引；零源码）。
- **S2（待放行）**：落码＋钉子 §8（触点 §5；先红后绿；全量档验证口径沿 071 批纪律）。
- **S3（随下一重建批）**：载体进体（字节判据：`findings` enum/描述/guide 字样 0→N 两平台）。
- **S4（随下次真机轮）**：真机核证（模型自发使用形态、徽章/水位/折叠零回归；读数如实记，不设
  使用率判据——自由写入口径）。
