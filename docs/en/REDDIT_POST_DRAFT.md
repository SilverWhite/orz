# Reddit 帖子草稿（REDDIT_POST_DRAFT）

> **性质**：纯草稿（charter §3 S4 可选件）；发帖动作与最终措辞归用户。
> **语言**：英文发帖（帖子正文自包含）；D1–D5 作帖内链接。
> **目标**：征求意见与纠正（非宣传）；诚实声明前置。
> **状态**：v0.1 草稿（2026-10-07，本窗口产出；未定稿）。

---

## 标题候选（三选一或改写）

1. **Unplanned ablation: our coding agent kept its memory across SlopCodeBench checkpoints (the benchmark resets it) — correctness collapse mostly vanished, code rot didn't** *(推荐：准确＋悬念；n=1 放正文首行)*
2. I accidentally ran SlopCodeBench in "continuous memory" mode — 11/36 problems fully solved (paper: none), but the same code-erosion trends
3. What happens if a coding agent never forgets? An honest post-mortem of my harness's benchmark-mode mixup

## 发到哪（建议）

- **首选 r/LocalLLaMA**（flair: Discussion）——DeepSeek 低价 API、agent harness、benchmark 实测都是该区高频话题；$22 全轮成本是天然钩子；自述技术帖+开源可容忍。
- 备选 r/ChatGPTCoding（agent 工具向）、r/singularity（泛 AI 向、回声大但技术讨论浅）。
- HN（Show HN）建议**等重置形态 A/B 结果出来再发**——故事更完整、可现场答疑。

---

## 正文草稿

I run a small open-source project called **orz** — a coding-agent harness I've been building solo for a few months (journal-first architecture, mechanical write-control, an approval fabric for dangerous actions, sliding-window context management). English is not my first language, so the English docs are LLM-assisted translations — corrections are very welcome, they're free review.

Last week I ran my agent through **SlopCodeBench** (the new UW benchmark: 36 problems, 196 checkpoints, where agents repeatedly extend their own solutions) end-to-end with DeepSeek's cheap Flash model. Total API cost: about **$22** (off-peak pricing, cache-heavy). Then I found out my harness had accidentally run the benchmark in a **different mode than the paper**.

**The mixup.** The official harness resets the agent's conversation at every checkpoint — only the workspace carries forward, so the agent must re-discover its own code each round. My adapter kept **one continuous conversation per problem**: the agent remembers every design decision it made. I had misread the reference agents' resume semantics; I publicly corrected this on the benchmark's issue tracker rather than quietly delete the run ([thread here](https://github.com/SprocketLab/slop-code-bench/issues/40)).

**So these numbers are NOT comparable to the leaderboard, and I'm not claiming either mode is harder.** But as an unplanned, n=1 supplement to the paper's ablations, the shape is interesting:

| Metric | Paper (reset mode, best of 15 agents) | Ours (continuous memory, 1 cheap model) |
|---|---|---|
| Problems fully solved (all checkpoints) | **0 / 36** | **11 / 36** |
| Strict pass (share of 196 checkpoints) | 14.8% | 22.4% |
| Isolated pass | 28.1% | 30.6% |
| Core pass rate, Start → Final phase | 64.6% → 35.5% (collapse) | 94.0% → 77.7% (mild decline) |
| Erosion rising across trajectories | 77% | 80.6% |
| Verbosity rising across trajectories | 75.5% | 77.8% |

**The observation**: keeping memory across checkpoints seems to mostly eliminate the *correctness* collapse the benchmark is designed to measure — but the *code-quality* degradation (structural erosion, verbosity) rises at essentially the same rates as the paper's reset-mode cohort. Session memory protected correctness persistence, not code quality. One run, one snapshot, one mid-tier model, different harness — I'm explicitly **not** claiming attribution. That's why I'm now rerunning in the official reset mode as a public Harbor run (same model, same problems, only the session mode differs), and I'll post those numbers too.

Side note: for my harness this doubled as the heaviest context-management stress test so far — each problem ran as a single unbroken session (30–190 minutes), ~4,600 model steps total, with mechanical context compression firing 186 times.

**What I'd love feedback on:**

1. Poke holes in the ablation reading — what confounds am I missing beyond model/harness/n=1?
2. The English docs linked below are machine-assisted; if you spot translation errors, that's genuinely useful feedback.
3. If you build or use coding-agent harnesses: what would you design differently? (The write-control and approval-fabric docs below are the two I lose sleep over.)

**Links:**
- orz (repo + docs): https://github.com/SilverWhite/orz
- English README: [docs/en/README.en.md](https://github.com/SilverWhite/orz/blob/main/docs/en/README.en.md)
- Write-control design (EN): [docs/en/WRITE_CONTROL_CURRENT_EN.md](https://github.com/SilverWhite/orz/blob/main/docs/en/WRITE_CONTROL_CURRENT_EN.md)
- Approval fabric (EN): [docs/en/ACAF_CURRENT_EN.md](https://github.com/SilverWhite/orz/blob/main/docs/en/ACAF_CURRENT_EN.md)
- Full-round numbers (EN): [docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md](https://github.com/SilverWhite/orz/blob/main/docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md)
- The mixup thread: https://github.com/SprocketLab/slop-code-bench/issues/40

All numbers are one run (k=1) of one frozen snapshot; the benchmark authors themselves warn against reading single runs as leaderboard entries. Be gentle(ish).

---

## 发帖注记（给用户的操作提示）

- **数字出自**：`SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md` §3/§13＋226 批轨迹级复算（80.6%/77.8%）——与本仓账面逐字可对。
- **表格口径**：strict/isolated 与论文同定义（固定 196 分母）；core 行两边同为部分得分均值；「11/36 全解」为 any 口径（core 全绿定档）——如被追问，答「checkpoint counts solved when all core tests pass, matching the harness's own pass_policy=any」。
- **80.6%/77.8% 复算口径**：逐题首档 vs 末档（轨迹级，与论文 RQ2 同分母）——与报告 §13 的转移级 65.0%/58.75% 不同分母，勿混用。
- **若评论区要逐题数据**：Harbor 数据集已公开（silverwhite/slopcodebench-friction），可指过去。
- **发帖时机**：避开周末深夜（美区黄金时段 ≈ 北京时间 0:00–4:00 前发）；重置形态 A/B 出数后可发续帖。

---

## 边界（charter 对齐）

- 发帖动作与发帖后回灌**不入仓**（charter §1/§3）；本文件为纯草稿、随时可改可删。
- 评论区纠错若触发英文档修订，走 D1 术语表→受影响档重出机制（charter §2.1），不散改。
