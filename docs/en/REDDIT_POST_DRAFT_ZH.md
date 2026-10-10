# Reddit 帖子中文工作稿（REDDIT_POST_DRAFT_ZH）

> **性质**：给用户改用的中文镜像稿（2026-10-08 由 v0.1 英文稿翻译）；**你改这里，改定后我回译成英文落
> `REDDIT_POST_DRAFT.md`**；发帖动作与最终措辞归用户（charter §1/§3）。
> 与英文稿的差异：新增一处【可选新增】段（压缩凹陷 v0 证据），不想要就删。

---

## 标题候选（先定中文意思，回译时再磨英文）

1. **计划外的消融实验：我们（我和我的AI）的 coding agent 在 SlopCodeBench 各 checkpoint 之间保留了记忆（基准本身会重置）——正确性坍缩基本消失，代码腐化照旧**（推荐）
2. 我意外用「连续记忆」模式跑了 SlopCodeBench——36 题全解 11 题（论文：0 题），但代码退化趋势相同
3. 如果 coding agent 从不遗忘会怎样？一次 harness 跑错基准模式的诚实复盘

## 发到哪（建议不变）

- 首选 **r/LocalLLaMA**（flair: Discussion）：DeepSeek 低价 API、agent harness、基准实测都是该区母题；$22 全轮成本是天然钩子。
- 备选 r/ChatGPTCoding、r/singularity；HN（Show HN）建议等重置形态 A/B 出数后再发。

---

## 正文（中文工作稿）

我维护一个小型开源项目 **orz**——一个我这两个多月使用AI开发的 coding-agent harness（journal 优先的架构、机械写控、危险动作的审批面、滑窗上下文管理）。英语不是我的母语，所以英文文档是 LLM 辅助翻译的——纠错非常欢迎。

这几天我让我的 agent 用 DeepSeek-V4.1-Flash 模型完整跑了一遍 **SlopCodeBench**（威斯康星大学的新基准：36 题、196 个 checkpoint，agent 要反复扩展自己的解法），API 总成本约 **22 美元**（低谷价、缓存吃满）。然后我发现，我的 harness 意外用了一种**和论文不同的模式**跑了这个基准。

官方 harness 在每个 checkpoint 重置 agent 的会话——只有工作区被继承，agent 每轮都得重新考古自己写的代码。而我的适配器存在误差，让**每道题保持一个连续会话**：agent 记得自己每一个设计决策。

**所以这些数字不能和排行榜直接比，我也不主张哪种模式更难。** 但作为一个计划外的、n=1 的论文消融补充，结果的形状很有意思：

| 指标 | 论文（重置模式，15 个 agent 中最好） | 我们（连续记忆，1 个廉价模型） |
|---|---|---|
| 完整解出的题目（全 checkpoint） | **0 / 36** | **11 / 36** |
| strict 通过（占 196 checkpoint） | 14.8% | 22.4% |
| isolated 通过 | 28.1% | 30.6% |
| core 通过率，Start → Final 相位 | 64.6% → 35.5%（坍缩） | 94.0% → 77.7%（缓降） |
| erosion 上升的轨迹占比 | 77% | 80.6% |
| verbosity 上升的轨迹占比 | 75.5% | 77.8% |

**观察**：跨 checkpoint 保留记忆，似乎基本消除了基准所要测量的*正确性*坍缩——但*代码质量*的退化（结构侵蚀、冗余膨胀）的上升率与论文重置组基本相同。会话记忆保住了正确性的保持，没保住代码质量。单次运行、单一快照、一个中档模型、不同的 harness——我明确**不**主张归因。这也是我现在正以官方重置模式重跑一轮公开 Harbor run 的原因（同模型、同题目、只有会话模式不同），出数后我也会发出来。

> 【可选新增，不想要就删】一个补充观察：在我们自己的条件里，「记忆」并不是逐字记录——整轮发生了 186 次机械上下文压缩，旧记录大多早已被折叠成摘要，而正确性是穿过这个折叠流保住的；并且我们在压缩事件后的紧邻行为里测不到任何扰动信号。看起来起作用的不是「记得多少记录」，而是那条可读的事件流本身。

旁注：对我的 harness 来说，这一轮同时有史以来最重的一次上下文管理压力测试——每道题一个不间断的会话（30–190 分钟），总共约 4,600 模型步，机械压缩触发了 186 次。

下面链接的英文文档是机器辅助翻译的；欢迎审查和指正错误。

**链接：**

- orz（仓库＋文档）：https://github.com/SilverWhite/orz
- 英文 README：`docs/en/README.en.md`
- 写控设计（英文）：`docs/en/WRITE_CONTROL_CURRENT_EN.md`
- 审批面（英文）：`docs/en/ACAF_CURRENT_EN.md`
- 全轮数字（英文）：`docs/en/SCB_V1_36_FULL_ROUND_REPORT_EN.md`
- 跑错模式那条线程：https://github.com/SprocketLab/slop-code-bench/issues/40

---

## 发帖注记（不变，中文原样）

- 数字出自全轮报告 §3/§13＋229 批轨迹级复算，与本仓账面逐字可对；
- 表格口径：strict/isolated 与论文同定义（固定 196 分母）；core 行两边同为部分得分均值；「11/36 全解」为 any 口径，被追问时的标准答案＝「checkpoint counts solved when all core tests pass, matching the harness's own pass_policy=any」；
- 80.6%/77.8% 为逐题首末档（轨迹级，与论文 RQ2 同分母），勿与转移级 65.0%/58.75% 混用；
- 评论区要逐题数据→指 Harbor 公开数据集（silverwhite/slopcodebench-friction）；
- 发帖时机：避开周末深夜（美区黄金时段≈北京时间 0:00–4:00 前发）。
