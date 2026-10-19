# 317 批：Reddit 评论线再追评——core/strict/iso 优势不对称回应（用户译文稿）＋「需好开局」假说登记（2026-10-19）

> **批次性质**：回复翻译落账批（零源码、零跑批、零 API 调用；**计数 60 不变**）。
> **来源**：Reddit 0df 评论线再追评后用户自撰回复令「请翻译我以下的内容」＋用户开场定性
> 「这不是一个很明显的问题吗，之前也都已经探究过了」——即 **core（checkpoint solve）优势
> 相较 strict/isolated 优势不对称**问题，V4 四方报告既有在案（core 73.5% vs 榜首 core 最高 67.3%＝Opus 4.6、GPT 5.5 core 66.8%／
> strict 33.7% vs 14.8%／iso 44.9% vs 28.1%；[`V4 报告`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md) §成绩表与命题 2/10）。

## §1 用户回复原文（中文权威）

> 很敏锐的观察OvO。
> 我想要优先补充一个点，ckpt 2不是独峰，还有ckpt5和ckpt7。
> 这个问题我第一次发现是意识到orz v0.9.0中的DeepSeek V4.1 Flash在SlopCode Bench中checkpoint
> solve成绩与当前官方成绩中第一的GPT 5.5(high)+Codex v0.124.0的checkpoint solve成绩相差不大
> （73.5% vs 67.3%）但strict solve（33.7% vs 14.8%）和isolated（44.9% vs 28.1%）成绩却相差巨大。
> 在运行过程中我也查看了orz的运行情况，在框架层面并没有找到原因，因此我个人只能初步考虑，
> 可能是orz中的DeepSeek V4.1 Flash本身确实是有些「偏科」，或许可能是因为其自身的稀疏注意力，
> 他本身不像GPT 5.5一样全面，他更需要一个相对好的开始来进行任务。
> 不过在测试中，单调塌陷的题目很少，很多题目的轨迹都是波动的，后期有回升。
> 这也是SlopCode Bench这个测试本身想要研究的问题，它致力于观察模型在长程多阶段任务中的表现。
> 因为我个人一直更多的在意模型自身的动作自由度和在尽可能少外界干扰中本身的真正的能力释放，
> 所以我没有对这一问题进行额外处理，我考虑任由模型表现即可，反正日常任务中我不会让模型独自
> 承担这样的连续性多阶段需求任务，我会和模型一起做。
> 非常高兴您对这个轨迹问题感兴趣，在此附上SlopCode Bench的成绩具体分析：
> https://github.com/SilverWhite/orz/wiki/SCBench-V4-Four%E2%80%90Way-Full%E2%80%90Round-Report-(0cr-0cw-r3-r4)

## §2 英译稿（发布动作与最终措辞留用户，沿 0cm 先例）

> Sharp observation OvO.
>
> One point I'd like to add first: checkpoint 2 is not a unique peak — checkpoints 5 and 7
> behave the same.
>
> I first noticed this when I saw that the DeepSeek V4.1 Flash running in orz v0.9.0 posts a
> checkpoint-solve score on SlopCode Bench close to the current official leaderboard leader,
> GPT 5.5(high) + Codex v0.124.0 (73.5% vs 66.8%), while its strict-solve (33.7% vs 14.8%)
> and isolated (44.9% vs 28.1%) scores diverge hugely.
>
> While the round was running I also watched orz's own runs and couldn't find a harness-level
> cause, so my personal working hypothesis is that DeepSeek V4.1 Flash itself is somewhat
> "lopsided" — perhaps due to its sparse attention, it isn't as all-around as GPT 5.5, and it
> depends more on getting a relatively good start on the task.
>
> That said, truly monotonic collapses were rare in our runs — most task trajectories
> fluctuate and recover in later stages. That's exactly what SlopCode Bench is designed to
> study: model performance in long-horizon, multi-stage tasks.
>
> Since I personally care more about a model's freedom of action and its genuine capability
> release under as little external interference as possible, I didn't apply any special
> handling for this — I'm fine just letting the model behave. In day-to-day work I wouldn't
> leave a continuous multi-stage task like this to a model alone anyway; I'd work on it
> together with the model.
>
> Really glad you're interested in this trajectory question. Here's the detailed score
> analysis for SlopCode Bench:
> https://github.com/SilverWhite/orz/wiki/SCBench-V4-Four%E2%80%90Way-Full%E2%80%90Round-Report-(0cr-0cw-r3-r4)

## §3 数字核验与一处表述提示（如实登记）

- r4 三项成绩与 [`V4 报告`](../SCB_V4_FOUR_WAY_CLEAN_ROOM_FULL_ROUND_REPORT_2026-10-16.md) §成绩表
  逐格一致：core 144/196＝**73.5%**、strict 66/196＝**33.7%**、isolated 88/196＝**44.9%**。
- 榜单侧：strict 14.8%／isolated 28.1% 均为「GPT 5.5 最好」✔；但 **core 榜首 67.3% 属
  Opus 4.6，GPT 5.5 的 core 为 66.8%**（报告成绩表注：Core 最高 67.3%〔Opus 4.6〕；GPT 5.5 66.8%）。
  用户原句将 67.3 配于 GPT 5.5——**2026-10-19 用户令「请直接帮我修正错误的数字」＝译文定稿
  修正为 GPT 5.5 core 66.8%**（§2 已改正；§1 中文原文照录不回改，作历史权威）。
- 「单调塌陷的题目很少、轨迹波动后期回升」与 V4 报告 §4.3／第二卷 §1.215 缺陷定因②
  （论文坍缩为五相位队列均值、非单题坍缩）一致。
- wiki 链接＝V4 报告的对外 github wiki 版；URL 含括号，发帖时建议纯文本或尖括号自链形态，
  避免markdown断链。

## §4 边界与不得宣称

1. 「偏科／稀疏注意力／需要好开局」＝用户个人假说（harness 侧已查无因），本批如实登记
   **不作机制结论**；r4 为 k=1 单轮、单模型价位混杂（V4 报告命题 2 边界同此）。
2. 「未做额外处理」的裁决在案＝动作自由度优先、少干扰观察真实能力释放（与 269 批线收官
   及 165 批「包装层不干预」裁决同向）；日常形态「与模型一起做」为用户使用姿态，不入框架面。
3. 回复发布动作、措辞定稿与后续追问不入仓（0cm charter 先例）。

## §5 台账

- 本档；BACKLOG `0df` 闭合后追记三＋指针行＋计数行（60 不变）；TODO 计数行＋`P1-0df` 追记三行；
  索引头行 v4.288 → **v4.289**；[`BACKLOG 第二卷 §1.256`](../BACKLOG_AND_PRIORITIES_2.md)。
- 零源码、零 `orz/` 子树改动；门禁 `python scripts/check_repository.py` ⇒ 目标 `valid: true`。

## §6 关键词

317 批、Reddit 再追评、core/strict/iso 优势不对称、73.5% vs 67.3%、33.7% vs 14.8%、44.9% vs 28.1%、
Opus 4.6 core 榜首、GPT 5.5 core 66.8%、偏科假说、稀疏注意力、好开局、单调塌陷少见、轨迹波动回升、
动作自由度、与模型一起做、V4 wiki 链接、计数 60 不变、索引 v4.289。
