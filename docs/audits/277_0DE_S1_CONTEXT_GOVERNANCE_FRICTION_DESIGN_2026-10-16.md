# 277 批：0de S1 勘定——治理摩擦综合四件设计定案（2026-10-16）

> **用户令**：「请先继续做0de吧」（0dd S2 后续批排程；0de S1 为该线下一步）。
> **性质**：**勘定批**——设计档一卷（[`CONTEXT_GOVERNANCE_FRICTION_COMPOSITE_DESIGN_2026-10-16.md`](../CONTEXT_GOVERNANCE_FRICTION_COMPOSITE_DESIGN_2026-10-16.md)，
> D1–D7 定案）＋267 批档 §3 两处就地勘注＋登记面同步；零源码、零 API、零跑批；
> **计数不变 61**；orz 子仓不动（HEAD `4749f6b1`）。
> **结论先行**：事发 journal（`RUN-2b1f85d2-1`）seq 级只读重建**勘误 267 批 F1/F2
> 机理描述两处**，四件修复落点随之定案——D1 H1 开窗可压集合前置／D3 迟交摘要
> 收束同消息豁免（事发主修）／D5 区间行括号注解段剥除（真实根因＝注解内全角逗号
> 被当分隔符）／D6 投影双字段兼容／D7 遥测计数器贯通；D2 窗内早收口与 D4 收束段
> 独立豁免**不立**（前者无可达路径、后者无机械判定面），登记防膨胀。

## §1 勘察证据（journal seq 级重建，全部只读）

证据基线＝事发工作区 `.gsa/runs/RUN-2b1f85d2-1/events.jsonl`（560 事件全量只读核查）
＋会话侧车（`conversations/2b1f85d2.json`，296 消息）＋归档包（`archives/2b1f85d2.json.gz`）。
尾段重建（本地时刻＝UTC+8）：

- seq 523（19:00:04）：H1 320k fire（面估算 337,261、`blocks=33`）⇒ 开压缩窗口；
- seq 525–535（19:02:26–27）：模型窗内主动清零（96 轮 364,589→148,186，handover 先写黑板）；
- **seq 536：`model_compression` 收口审计 `model_participated=true rounds_used=1`**——
  窗口按设计收口（handover 写数 19→21 翻参与位）；
- seq 549–554（19:03:45）：终报前收尾门（counterexample_gate＋orientation_checkpoint）；
- seq 555（19:05:19）：模型终轮消息＝**自愿迟交摘要块＋终报＋`[RUN_END] intent: conclude`
  同消息**；seq 556：loop-top 消费该摘要 ⇒ `block_selection_unrecognized`
  （`compressible_blocks=0`，原行 `压缩块: 33（窗口期唯一原文块；连同 1–34 区段已由
  主动清零覆盖，本摘要为迟交落账）`）⇒ 注入告知块；seq 558：模型被迫多一轮确认；
  seq 559：`run_finished{completed}`。
- 四件字段实证：5 个 `model_summary` 事件 `rounds_since_last_compaction` 全 0（F4）；
  2 个 `clear` 事件均无 `estimated_tokens_after`（F3）。

## §2 两处勘误（267 批 §3 就地勘注，随批落）

1. **F1**：「机械窗口又开窗一轮」不成立——事发尾段无新阶梯 fire、无新窗口块；H1 窗
   已按设计收口（seq 536）。逼轮来自**摘要消费路径的告知注入**（`compress_blocks_now`
   未识别/未落地回执），非压缩窗口。修复方向由「窗口重武装延迟／收束段豁免」更正为
   「消费路径收束豁免（D3）＋H1 开窗前置（D1）」。
2. **F2**：「解析只认裸 N-M 形」不准确——0bk S1 宽容化在事发载体 0.9.0 在役（其钉
   用例注解用全角分号可解析）。真实根因＝**分隔符切分先于注解容忍**：`parse_block_selection`
   先按 `,／，／、` 切段、每段取数字核心；事发行注解内含**全角逗号**，切出的第二段
   「本摘要为迟交落账）」无数字核心 ⇒ 整条 `Unrecognized`。

## §3 定案（D1–D7，详见设计档 §2–§5/§10）

| 决策 | 内容 | 采/不立 |
|---|---|---|
| D1 | H1 开窗前置＝可压集合非空（单源复用 `t1_compressible`）；空集仍注入 `hard_reminder_block` 本体（信息面），不附窗口块、不开窗；审计 `form=reminder_only_no_compressible` | 采 |
| D2 | 窗内可压集合清空 ⇒ 提前收口 | **不立**——`clear`/`clear_all` 的 `handover` 机械必填且机械层先写黑板、摘要产出（含未落地）翻 `produced_summary` ⇒ 窗内治理必翻 `participated` ⇒ 收口（事发 seq 536 即证），无空集逼窗可达路径 |
| D3 | `pending_semantic` 识别点同测 `extract_model_stop`；源消息带 STOP 声明 ⇒ 消费点三路（未识别/未落地/台账写失败）**只落审计不注入** | 采（**事发主修**——仅修 F2 时迟交摘要仍会经「区间无命中」注入逼轮） |
| D4 | 收束段独立豁免 | **不立**——收束段无机械判定面（`RUN_END` 是模型自述非可信门）；D1/D3 已覆盖事发形态 |
| D5 | `parse_block_selection` 分隔符切分**前**剥除平衡括号注解段（全角/半角；未闭合剥至段尾）；非法段仍拒（`abc`／`全部`）；0bk 契约文案零改 | 采 |
| D6 | `projection.js` `context_compressed` 模板双字段兼容（`estimated_tokens_after` 缺位回退 `target_tokens`）；否决 payload 加字段（schema 面膨胀＋历史不回填） | 采 |
| D7 | `compress_blocks_now` 增 `rounds_since: u32` 参数写真值；loop-top/run 尾两消费点贯通既有 `rounds_since_compact`；老写点不动 | 采 |

## §4 S2 钉子定案（N1–N5）

- **N1（D5）**：事发行原样 ⇒ `Specified([33])`；0bk/0bm 既有用例全绿；`1-4, abc`／
  `全部` 仍 `Unrecognized`；新增注解含全角逗号＋en-dash 用例。
- **N2（D1）**：极小刻度 ladder 驱动——空可压集合越 H1 线 ⇒ 无窗口＋注入块不含窗口
  文本＋审计 `form=reminder_only_no_compressible`；对照臂＝有可压块照常开窗。
- **N3（D3）**：带 `[RUN_END] conclude` 的回复产出摘要 ⇒ 审计落、`messages` 尾无告知
  块；对照臂＝无 STOP 声明照常注入（0bs ⑥ 不翻）。
- **N4（D7）**：N 轮后分块压缩成功 ⇒ `rounds_since_last_compaction == N`（≥1）。
- **N5（D6）**：clear 形 payload ⇒ 渲染含 `target_tokens` 值、不含「估 ~0K」（web 面）。

## §5 边界

1. 零源码、零跑批、零重建；orz HEAD `4749f6b1` 不动；S2 落码待放行。
2. 693K 自拉回 vs 900K 必压线间距＝用户裁决不处理（267 §1 维持）。
3. 267 批档 §3 勘注为就地勘误（原文保留）；索引 canonical 条目随本批更新时同步勘正。
4. 计数不变 61（S1 非闭合批）；索引 v4.249 → v4.250。

## §6 关键词

277 批、0de S1、治理摩擦综合勘定、journal seq 级重建、F1 勘误迟交摘要逼轮、
F2 勘误注解内全角逗号、D1 H1 开窗可压集合前置、D3 收束同消息豁免、D5 括号注解段
剥除、D6 投影双字段兼容、D7 rounds_since 计数器贯通、N1–N5、设计档
CONTEXT_GOVERNANCE_FRICTION_COMPOSITE_DESIGN、计数 61 不变、索引 v4.250。
