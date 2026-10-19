# 316 批：Reddit 评论线追评——失败级联与「坏基底税」检验（分叉假说）＋承诺性失败确认（2026-10-19）

> **批次性质**：查证落账批（零源码、零 API 调用；r4 checkpoint 台账＋journal 只读复算；**计数 60 不变**）。
> **来源**：Reddit 0df 评论线追评〔评论者论点：①非超时问题而是**承诺性失败**（confident while
> wrong、cap 不咬合）；②ckpt2 为失败峰值 ⇒ 路子早期走岔、其后各档在坏基底上重做、故消耗摊开
> 而非堆尾；③任务级计量将给出「分叉前档成本 vs 分叉后重做成本」一个读数；④若分叉确在
> ckpt2，便宜修法＝在那里加一道检查〕＋用户惯例令（登记留痕）。

## §1 检验结果（对评论者四论点逐条对数）

**基准**：`r4_checkpoint_ledger.jsonl` 官方 194 行（剔事故对 ckpt1〔`step_has_prior=None`〕／
ckpt2〔core null〕＋两行零轮机械截停；36 题；对齐 283 批 §6.1 形态学分母）。

| # | 评论者论点 | 检验 | 结果 |
|---|---|---|---|
| ① | 承诺性失败、cap 不咬合 | 50 失败档 `stop_reason` | **全数 `end_turn`**（50/50）、零 killed/cancelled、最长失败档仅用 7200 s 上限的 49% ⇒ **成立** |
| ②a | ckpt2＝独有早期分叉峰 | 暴露归一化逐档失败率 | ckpt1 19%（7/36）／**ckpt2 34%**（12/35）／ckpt3 19%／ckpt4 24%／**ckpt5 35%**（9/26）／ckpt6 21%／**ckpt7 38%**（3/8）／ckpt8 17%——**ckpt2 非独峰**：原始计数峰值是暴露效应（仅 6–14 题有 6–8 档），归一后中后段同风险 |
| ②b | 坏基底传导（其后各档在坏基底上重做） | 档间结构＋级联＋同档位成本 | `step_has_prior`：160/197 档继承前档工作区（**渐进结构属实**）；**P(败 k+1｜败 k)＝57%（20/35）vs P(败 k+1｜过 k)＝19%（23/121）≈3×**；同档位「污染档」（该题已有失败档之后）中位墙钟为洁净档 **1.03×–1.69×**（随深度走高：ckpt5 1.28×、ckpt7 1.69×＝1186 s vs 701 s、轮数 126 vs 73）；污染档通过率 **45%**（25/56）vs 洁净档 **86%**（119/138）；污染档墙钟占全轮 **34%** ⇒ **机理成立且强** |
| ③ | 计量给出「分叉前 vs 重做后」一个读数 | 0df 计量行形态 | 方向一致——`run_usage_metered` 一 run 一行后该读数是直接 join；本批数字即用既有台账+journal 事后算出（同 315 批方法学），下一整轮起免复算 |
| ④ | 便宜修法＝ckpt2 处加检查 | 两点保留 | (a) 分叉非独在 ckpt2——档界风险全程存在（34%/35%/38%），检查须逐档界而非仅 ckpt2；(b) 模型自检是否继承同一承诺性偏差待证（50/50 自信收尾的同一机理）；机械/外部档界门属 harness 侧改动，**是否立项留用户裁决**，本批只登记假说与证据 |

## §2 复算读数汇总（194 行官方口径）

- 逐档失败率：19%／34%／19%／24%／35%／21%／38%／17%（ckpt1–8；n=36/35/36/33/26/14/8/6）。
- 级联：条件失败率 57% vs 19%（比值 ≈3.0；n=35/121 个相邻档对）。
- 坏基底税：同档位污染/洁净中位墙钟比 1.17×／1.03×／1.12×／1.28×／1.25×／1.69×／1.21×
  （ckpt2–8；中位轮数 ckpt7＝126 vs 73）；污染档通过率 45% vs 86%；污染档墙钟占比 34%
  （61,847 s／181,364 s；与 283 §6.1 失败档 33.7% 时间份额同量级、分组口径不同）。
- 档界风险形态：**逐档界持续 17–38%**，非单点分叉；失败摊开＝暴露宽度 × 持续档界风险 ×
  级联放大三者叠加。

## §3 回复草稿（英文；发布动作与最终措辞留用户，沿 0cm 先例）

> That reading holds up better than you might expect — we went back to the ledger and most
> of it checks out, with one refinement.
>
> First the confirmation: it is a commitment problem. All 50 failed checkpoints end with
> `end_turn` — zero killed, zero cancelled, the longest one used 49% of the 7200 s cap. The
> model is confident while wrong, and the cap never bites.
>
> The refinement is on checkpoint 2: the raw peak is partly an exposure effect, because only
> 6–14 tasks have 6–8 checkpoints. Normalized by exposure, the per-stage failure rate is
> 19% / 34% / 19% / 24% / 35% / 21% / 38% / 17% across stages 1–8 — stage 2 is not a unique
> early fork; mid and late boundaries carry the same per-stage risk.
>
> But the bad-base mechanism itself is real, and stronger than your framing: checkpoints in
> this benchmark inherit the previous stage's workspace, so a wrong approach does propagate.
> P(next stage fails | this one failed) = 57% vs 19% after a pass — 3×. At the same stage
> index, checkpoints that follow a first failure cost 1.0–1.7× the median wall-clock (the
> ratio grows with depth; at stage 7 it's 1186 s vs 701 s, 126 vs 73 rounds) and pass at 45%
> vs 86%. Checkpoints after a first failure eat 34% of total wall-clock. So the burn
> spreading is: sustained per-boundary risk, amplified by cascade.
>
> On the cheap fix — the economics are right, but the fork isn't uniquely at stage 2, so the
> check would have to live at every boundary, and it's worth being honest that a model
> self-check probably inherits the same overconfidence (all 50 failures self-certified with
> `end_turn`). A mechanical or external boundary gate is the version that could actually
> bite; whether to build that is a separate decision. The metering rows land first —
> cost-before-first-failure vs cost-after will be a direct readout next round instead of a
> ledger reconstruction like this one.

## §4 边界与不得宣称

1. 194 行／36 题／k=1 单轮；§2 全部为描述性统计（相关非因果）；级联比值的小 n 单元
   （ckpt6–8 n=3–5）只作方向性读数。
2. 「污染/洁净」分组以**该题首个失败档**为界，档界风险与级联效应在观测上不可完全分离
   （题目难度本身可同时推高两率）——57% vs 19% 是净效应上界口径，如实记。
3. §1-④ 检查门形态**未立项**：登记假说与证据，不进工作集；机械档界门若立须走独立调研批。
4. 回复发布动作、措辞定稿与后续追问不入仓（0cm charter 先例）。

## §5 台账

- 本档；BACKLOG `0df` 闭合后追记二＋指针行＋计数行（60 不变）；TODO 计数行＋`P1-0df`
  追记二行；索引头行 v4.287 → **v4.288**；[`BACKLOG 第二卷 §1.255`](../BACKLOG_AND_PRIORITIES_2.md)。
- 零源码、零 `orz/` 子树改动；门禁 `python scripts/check_repository.py` ⇒ 目标 `valid: true`。

## §6 关键词

316 批、Reddit 追评、承诺性失败、end_turn 50/50、cap 不咬合、暴露归一化逐档失败率、
ckpt2 非独峰、渐进档结构 step_has_prior、失败级联 57% vs 19%、坏基底税 1.0–1.7×、
污染档通过率 45% vs 86%、污染档墙钟 34%、档界检查门假说不立项、计数 60 不变、索引 v4.288。
