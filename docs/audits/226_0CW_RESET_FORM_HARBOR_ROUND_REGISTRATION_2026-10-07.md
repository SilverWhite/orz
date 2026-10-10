# 226 批：0cw 立项——SCBench 官方重置形态 Harbor run 轮（每 checkpoint 新会话）（2026-10-07）

> **用户令**：「请立项新的每checkpoint新对话sc bench轮吧，并且这一轮走harbor run」。
> **性质**：立项批（只读勘定落账＋登记）——零源码、零跑批、零子仓改动；未闭合计数 **54 → 55**
>（新立项 0cw）；**未提交、未推送**。
> **结论先行**：0cr 全轮（134/196）经论文与 harness 源码双重核证为**连续会话变体形态**——官方
> runner 逐 checkpoint `reset_context=True` 重置 agent 会话、仅继承工作区，而我方 orz 适配器跨档
> 保留同一 ACP 会话（根源＝158 批 S0 对 claude_code resume 语义的误读、196 批 §4 误登记为「官方
> 长程语义」）。0cw 以**官方同形（逐档新会话）＋ Harbor run 公开管线**重跑全目，与 0cr 构成
> **同模型跨形态 A/B**，并落地 issue #40 ①的榜单路径；追评公开承诺（`issuecomment-6040701347`，
> 2026-10-07）随本批兑现立项。

---

## §1 形态分歧核证（本批前置只读分析的证据面，全部一手来源）

### 1.1 官方形态＝逐 checkpoint 重置会话（三处独立证据）

| 来源 | 原文/代码 | 含义 |
|---|---|---|
| 论文 §2 | 「The agent must reason about changes solely from the code's current structure, **as we do not provide the prior conversation's context**.」；形式化模型 yᵢ=πθ(xᵢ, yᵢ₋₁)（只带工作区、不带对话） | 基准设计即无记忆 |
| 论文 App C | 「Installed packages, shell history, and **agent session data reset between checkpoints**.」；另每 run 有 **2 小时墙钟上限** | 实验形态明示 |
| harness 源码（pin `31ceea3`） | `agent_runner/runner.py` `_setup_for_checkpoint`：首个之后每档 `self.agent.finish_checkpoint(reset_context=True)`（L662）→ `agent.py` `reset()`；`claude_code` 的 `run()` 每 checkpoint 起**全新 CLI 进程不带 `--continue`**、`--continue` 仅在 `retry()`（同档失败重试）使用；`codex` 同形（`resume --last` 仅重试臂） | 官方 runner 逐档强制重置 |
| 社区旁证 | HumanLayer 复测文档：「**Fresh context window per checkpoint** — yes, the conversation resets between checkpoints, but code carries forward」 | 社区复测同形 |

### 1.2 我方 0cr 形态＝连续会话变体（误读链）

- 158 批 S0 §2.1 勘定写「claude_code 实现以 `resume=True`→CLI `--continue` 维持**单连续会话跨全部
  checkpoint**」——**误读**：`resume=True` 只在重试臂，非跨档。
- 该误读进入 orz 适配器：`agents/orz/agent.py` `reset()` 刻意保留跨档 ACP 会话（注释自述
  「mirroring claude_code's reset() (its conversation resumes via --continue; ours simply never
  stops)」）；196 批 §4 进一步把「单题单容器跨档单一 ACP 会话」登记为「官方 SCBench 长程语义」。
- **两处登记均勘正**（dated 档不回改，以本批与 0cw S1 勘正注记为准）。

### 1.3 跨形态读数对照（0cr 连续 vs 论文重置队列；**跨模型混杂在案**）

| 口径 | 0cr（连续会话，0.8.14＋deepseek-v4-flash，k=1） | 论文/榜单（重置形态，15 模型最好条目） |
|---|---|---|
| checkpoint solved（any） | **134/196＝68.4%** | core 均值最高 67.3%（Opus 4.6；口径为部分得分均值，非定档二值） |
| strict 满通过档 | 44/196＝22.4% | **14.8%**（GPT 5.5） |
| isolated 满通过档 | 60/196＝30.6% | **28.1%**（GPT 5.5） |
| 任务级全档 | **11/36** | **0/36**（「无任何 agent 全解任何题」） |
| core 相位曲线（Start→Final） | 94.0% → 77.7%（缓降） | 64.6% → **35.5%**（坍缩） |
| erosion 上升轨迹占比 | 80.6%（29/36，本批轨迹级复算） | 77% |
| verbosity 上升轨迹占比 | 77.8%（28/36，同上） | 75.5% |
| erosion / verbosity 均值 | 0.613（偏高，如实记）／0.289（表内最好） | 最好 0.49（GPT 5.5）／0.32（同） |

形态差的形状：**质量退化轴几乎不随会话形态移动（80.6/77.8 vs 77/75.5），正确性保持轴剧变
（11/36 全解 vs 0；末档 core 77.7% vs 35.5%）**。模型（deepseek-v4-flash vs GPT 5.5/Opus 4.6）、
harness（orz vs 原生 CLI）、载体三变量同时不同 ⇒ 现有对照**不能归因**；0cw 的存在意义即钉死
「会话连续性」单因子。

## §2 0cw 立项定义

- **canonical ID**：`EVAL-SLOPCODE-BENCH-RESET-FORM-HARBOR-ROUND`（**0cw**，P1，54 → 55）。
- **目标**（三重产出）：① **同模型跨形态 A/B**——deepseek-flash（底模 DeepSeek-V4.1-Flash，0cv
  208 批核证与 0cr 的 legacy 名同底模，模型侧可比）＋重置形态，与 0cr 唯一目标差异＝会话形态，
  隔离「会话连续性」因子；② **榜单可见条目**——走 Harbor run 官方管线公开上传（TB2.1 官方轮
  `--upload --public` 先例），即 208 批核证的「事实入榜通道」，兑现 #40 追评承诺；③ 摩擦主线延续
  （0cr 定性不变：主线找摩擦，成绩是副产品）。
- **冻结口径**：k=1；seed 42；pass_policy any；one_shot off；全目 36 题／196 checkpoint；
  **每 checkpoint 新会话**（官方重置形态）；载体＝0.8.15 在役线。
- **已登记混杂与边界**：
  - **载体差**：0cr＝0.8.14、0cw＝0.8.15（内含 0ct/0cs/0cu/0cv 进体件）。沿 0cr 口径「在役摩擦
    如实计入、修复后预期只升不降」，登记为已知混杂、不作单因断言。
  - **成本**：重置形态跨档缓存命中归零（0cr hit 占输入 53.4%；重置后仅档内命中）⇒ 有效成本显著
    高于 0cr 的 $22.0；量级估计 **$40–80**（off-peak 价）。**S3 起跑前设成本门**，按控制台实价
    复核后由用户放行。
  - **墙钟**：论文形态有 2h/run 上限、0cr 无帽（test_translator 189m）。是否施加 2h 帽随 S1 勘定
    （Harbor 侧 timeout 语义）裁决；不施加则登记为形态轴差异。
  - **定性**：205 批「friction-hunting-artifact／not-official-scores」双标签是**数据集件**的定性，
    不变；本轮的 **run** 以公开可榜单条目为目标（208 批已核证公开 run＝事实入榜通道），报告如实
    标注 k=1／单快照／重置形态。
- **批序（各步独立放行，不得跳步合批）**：
  - **S1 勘定**：a) Harbor 运行形态——SCBench checkpoint 链在 Harbor run 中的承载方式（上游数据集
    `gabeorlanski/slopcodebench` 与我方转换集 `silverwhite/slopcodebench-friction` 的任务结构差异；
    一 Harbor 任务＝一题内嵌 checkpoint 循环还是逐档任务；工作区跨档持久与超时语义；harbor 版本
    pin）；b) 适配器重置模式设计——`agents/orz/agent.py` `reset()` 改真重启 ACP 会话（每档新
    session id／新 journal RUN 为重置证据面），连续模式保留为开关（0cr 形态保持可复现）；c) 0cr
    全轮报告 §5「证伪」/§6「同构口径方向性略高」两处可比性勘正随 S1 落。
  - **S2 适配器落码＋冒烟**：重置模式实现；冒烟判据＝逐档新会话证据（session id/journal RUN 每档
    焕新、工作区跨档持久）；连续模式零回归。
  - **S3 Harbor run 执行**：成本门（用户放行）→ k=1 全目起跑、公开上传。
  - **S4 收取与对拍**：官方多维读数收取；与 0cr 同模型跨形态对拍（逐档/逐题/相位曲线/侵蚀与冗余
    轨迹级比率四层）；榜单条目核证；收口裁决＋报告更新。
- **明确不做**：不改 0cr 已归档读数与档（勘正以注记落）；不动 205 批 Harbor 数据集件；0ct/0cu 的
  S4 真机复核**不搭本轮**（判定面不同：S4 需真机狗粮形态读数，Harbor 评测容器形态不充当载体——
  如 S1 勘定发现可搭，另批裁决）；不追分特化（8 工具面冻结纪律沿用）。

## §3 台账

- 本档：`docs/audits/226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md`。
- BACKLOG：新增 `0cw` 专节＋计数行（**55**）＋本批指针行＋P1 总览行锚点。
- TODO：计数行＋P1 路由行＋新增 `P1-0cw` 勾选节。
- BACKLOG 第二卷：§1.172。
- 索引：头行 v4.198 → **v4.199**；§6 新增 0cw 条目＋0cr 条目状态与形态勘正注记；§8 pending 桶收编。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑（见 §4）。

## §4 关联与关键词

issue #40 追评（`issuecomment-6040701347`，2026-10-07）／[`207 全轮报告`](../SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md)
（§5/§6 勘正随 0cw S1）／[`208 批档`](208_0CV_DEEPSEEK_IFACE_ALIGN_2026-10-07.md)（提交路径核证先例）／
[`196 批档 §4`](196_0CR_S1_HARBOR_CHAIN_AND_MANIFEST_FREEZE_2026-10-05.md)（误登记源，本批勘正）／
[`158 批档 §2.1`](158_0CJ_S0_SURVEY_2026-10-02.md)（误读源，本批勘正）。

关键词：0cw、重置形态、逐 checkpoint 新会话、Harbor run、连续会话变体、跨形态 A/B、会话连续性、
榜单条目、reset_context、158 误读勘正、196 误登记勘正、成本门、计数 55。
