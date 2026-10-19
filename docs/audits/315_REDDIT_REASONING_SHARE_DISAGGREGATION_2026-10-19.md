# 315 批：Reddit 评论线后续轮——推理占比离散度分解查证＋「逐题」口径勘正＋SCB 可靠度边界（2026-10-19）

> **批次性质**：查证落账批（零源码、零 API 调用；既有 r4 journal／台账的只读复算；**计数 60 不变**）。
> **来源**：Reddit 0df 评论线新一轮回复〔评论者三点：①effort 设置只是请求参数、reasoning-token
> 计数才是模型实花，28–90% 区间证明旋钮预测不了账单；②缓存角度本轮帮助不大（推理 token 每次
> 调用都是全新输出、无缓存折扣，76% 占比下即账单大头）；③**追问：离散度与任务类型相关，还是
> 相似任务内部也上下跳动？**〕＋用户令「请帮助我翻译和回复，这个和任务级预算相关」＋「这一轮也
> 要按照惯例进行登记留痕，方便后续翻阅」＋口径补充令（SCB 可靠度，§5）。
> 轮次注：284 批登记至「评论线第三轮」；其间用户回帖带出 effort_tier 分列设计与 28–90% 实证
> （287/288 批、设计稿 D5），本轮为其再后续，不另编号。

## §1 口径勘正：「逐题 28%–90%」实为逐会话（run）粒度

对 284 批档 §4-5、287 批档、设计稿 D5 所引「逐题 28%–90%」做只读复算勘正：

- **28%–90%（均值 ~76%）是按会话（run）粒度的区间**——194 份官方会话逐份
  reasoning/completion 的极差，非逐题。
- **逐题聚合（36 题）实际区间为 67%–85%**（均值 76.2%、中位 76.6%、σ≈4.0pp）。
- 区间下界 28% 出自**一次调用的桩会话**（`r4-p30-rejector` checkpoint_4，1 次调用、
  completion 188）——小会话离散天然大。
- 影响面：284 批档 §4-5「逐题 28%–90%」、287 批档 §1、设计稿 D5 同句——**语义结论不变**
  （档位恒定 ≠ 推理量恒定；且逐会话粒度反而强化该论点），粒度标注勘正随本批登记；
  设计稿 D5 就地勘注（§315 批勘注），历史批档不回改。

## §2 复算读数（统一基准：官方 36 题／194 份会话／13,940 次模型调用）

基准＝`jobs-r4` 官方 36 题目录下 `agent/gsa/runs/RUN-*/events.jsonl`（**剔除 p18 事故 stub ×1
与孤儿重跑件 ×5**，与 283 批剔除口径同源；台账「200 份会话／14,363 调用」含该 6 件窗口，差
194/13,940）。逐档行按 completion ≥1K 过滤（193 档）。零 API、零跑批，纯本地聚合。

| 读数 | 值 |
|---|---|
| 逐会话推理占 completion | **28%–90%**，均值 75.5%（n=194） |
| 逐题聚合 | **67%–85%**，均值 76.2%、中位 76.6%、σ 4.0pp（n=36；每题 3–8 档、中位 5） |
| 逐档（≥1K） | 49%–90%，σ 6.1pp（n=193） |
| **档级方差分解** | **题内 61%／题间 39%** |
| 题均占比 × 档通过率（core 满）相关 | r ≈ **0.26**（n=36，弱） |
| 会话规模（ln completion）× 占比相关 | r ≈ **0.45**（正，中等） |
| 端到端全解 4 题（strict 同口径＝p02/p23/p29/p36） | 占比 **75%–79%**，居于全 36 题均值带（76.2%） |
| 区间下界桩会话 | rejector ckpt4：1 调用／completion 188／28% |

**对评论者追问的回答**：两者都有，但**主要在题内**——离散大半（61%）是同题跨档的局部位移，
题间只解释 39%；且占比不预测成败（端到端全解 4 题居均值带、r≈0.26）。SCB 无任务类型标签
（36 个异质项目，类型与题目身份混杂），「类型相关性」在本基准上不可分离——这正是 0df 计量行
（`run_usage_metered`，reasoning 与 effort 分列、一 run 一行随会话存档）的用武之地：下一整轮
起，占比×题目×档序×结果的关联是机械 join，不再做事后复算。

## §3 回复草稿（英文；发布动作与最终措辞留用户，沿 0cm 先例）

> Both — but mostly within tasks, it turns out. I re-aggregated the round to check.
>
> The 28–90% is per-session (one run = one checkpoint attempt), and the 28% end is a
> degenerate one-call stub session. Aggregate per task over its 3–8 checkpoint sessions and
> the range compresses to 67–85% around the same ~76% mean. So task identity explains some
> of it, but decomposing the per-checkpoint variance: roughly 60% is checkpoint-to-checkpoint
> *within* a task, only 40% between tasks. The bounce is mostly local — same task, same
> effort knob, adjacent phases swing the share by tens of points.
>
> It also doesn't track success: the four fully-solved tasks sit at 75–79% vs 76% overall,
> and a task's mean share correlates only weakly with its checkpoint pass rate (r ≈ 0.26).
> There is a mild size effect — longer sessions run a higher reasoning share (r ≈ 0.45) —
> but no clustering by project beyond that.
>
> One nuance on "task type": this benchmark is 36 heterogeneous projects with no type labels
> beyond identity, so type and task are confounded here. That's part of why we just shipped
> the per-run metering line (observed reasoning tokens + configured effort, one row per run,
> into the session archive) — next full round, correlating share against task, checkpoint
> index and outcome is a mechanical join instead of a forensic re-aggregation like this one.
>
> And agreed on cache: reasoning tokens are fresh output on every call, no discount touches
> them — at ~76% share they effectively are the bill.
>
> One honesty note on the source data itself: SlopCode Bench's headline numbers are low
> across the board — the paper itself reports zero fully-solved end-to-end runs (0/36) — so
> I'd treat anything computed on it as indicative rather than authoritative. It's just the
> only benchmark I've found that treats the harness as a first-class variable.

## §4 口径补充（用户令，随批登记为 SCB 数据引用边界）

用户口径：「SlopCode Bench 这个测试目前的各参数指标的成绩都不是很高，论文中报告的端到端全解
也只是 0/36，我考虑实际上用这个测试的数据来分析，可靠度不会很高，只能说是仅供参考吧，不过目前
我只找到这个测试是重点考虑了 harness 的 XD」。

- **仓内证据对齐**：论文「no agent fully solves any problem end-to-end」＝0/36
  （[`V4 四方报告`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md) §成绩表论文列；
  按论文同口径重算本仓 r4＝4/36）；论文各指标成绩不高与 269 批线终判、第二卷 §1.215 缺陷定因
  （口径错配＋层级未声明）一致。
- **登记为引用边界**：凡以 SCB（含 r4）为源的分析读数（含本批 §2），对外引用须带「仅供参考/
  indicative」限定；选型理由如实记＝当前唯一重点考虑 harness 的基准，替代面未找到。
- 本边界**不回改** 283/284/287/288/293 等既有批档（其边界节已各有 k=1／口径声明）；自本批起
  新增引用适用。

## §5 边界与不得宣称

1. §2 全部读数为**单遍只读复算**（未做二次独立复核）；r 值为描述性相关、**不作因果宣称**；
   n=36／n=194 小样本。
2. SCB 无任务类型标签，「任务类型相关性」不可分离——只能以题目身份／结果／档序作代理。
3. 「roughly 60%/40%」方差分解基于档级行（≥1K 过滤，n=193）单基准；不同过滤阈值下有 ±pp 级
   漂移，引用带「约」。
4. 台账旧值「200 份会话／14,363 调用」与本批「194／13,940」的差异＝事故 stub＋孤儿件窗口
   （6 文件），**非数据矛盾**；两口径并存在案。
5. 回复的发布动作、发帖后的再追问与措辞定稿不入仓（0cm charter 先例）；如后续轮再有数据
   追问，按需另批。

## §6 台账

- 本档；设计稿 D5 就地勘注（[`PER_TASK_METERING_DESIGN`](../PER_TASK_METERING_DESIGN_2026-10-17.md)）；
  BACKLOG `0df` 闭合后追记＋指针行＋计数行（60 不变）；TODO 计数行＋`P1-0df` 追记行；
  索引头行 v4.286 → **v4.287**；[`BACKLOG 第二卷 §1.254`](../BACKLOG_AND_PRIORITIES_2.md)。
- 零源码、零 `orz/` 子树改动、零 `orz_source_manifest.sha256` 重生成；计数不变 60。
- 门禁 `python scripts/check_repository.py` ⇒ 目标 `valid: true`。

## §7 关键词

315 批、Reddit 评论线后续轮、推理占比离散度、逐题→逐会话口径勘正、题内 61%／题间 39%、
67–85% 逐题聚合、桩会话 28%、r≈0.26 通过率、r≈0.45 规模效应、端到端全解 4 题均值带、
SCB 0/36 论文口径、仅供参考边界、run_usage_metered 机械 join、计数 60 不变、索引 v4.287。
