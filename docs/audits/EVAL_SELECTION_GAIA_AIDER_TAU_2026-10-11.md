# 评测选型审计——GAIA／Aider Polyglot／τ²-bench 可跑性评估（2026-10-11）

> **批次**：未领号（避与并行在跑轮次撞号；登记随下一次台账批）。
> **日期**：2026-10-11。
> **用户令**：「GAIA值得跑」「Aider Polyglot和τ也可以进行尝试」「请落一份审计文档吧」。
> **性质**：选型审计——只读研究。零源码改动、零载体改动、零记账文件改动
> （TODO／BACKLOG／索引不动，随台账批登记）。外部资料经本地浏览器车道取回。
> **结论先行**：三条候选与在跑的 SCBench 互补，且均**不依赖 Harbor 容器编排**
> （成本结构与 TB／SCBench 不同）。按「外部认可度 × 成本 × 与项目能力面匹配」排序为
> **GAIA → Aider Polyglot → τ²-bench**。三条共同前置＝固定题集 pin、判分器核证、
> k≥5 试次口径；共同风险＝污染与脚手架主导，须与项目既有「k=1 找摩擦」用法分账。
> **同日续查与裁决（见 §9）**：GAIA 降为**自跑专用**（HF 账号活跃度门槛，不可提交）；
> Aider Polyglot **不采纳**；**τ²-bench 选定先行**。

---

## §0 背景与范围

前序事实（一手）：

- **TB 2.1 成本门**：用户裁决「tb目前没得跑，成本太高」。官方口径为题 89×5 试次＝445 trials，
  且 2.1 暂不接社区提交（只收维护者跑的结果）。
- **SCBench 重置形态轮**由并行轮承担（0cw），本审计不重复该线。
- 已登记评测线（不在本审计范围）：0cr／0cw（SCBench）、0ci（Frontier-Bench）、
  0cj（SCBench 挑题摩擦）、0w（TB 4.0 单题探针）、EVAL-TB21 系列、EVAL-SWE-BENCH、EVAL-POLYGLOT。

本审计目的：在 TB 之外筛出**可跑、成本可控、能力面互补**的候选线，并给出可执行批序与判据建议。

---

## §1 检索方法与工具限制（诚实记录）

本轮取外部资料时实测到的车道状态，供后续回查者复现：

- **HTTP 车道不可靠**：`bing_cn`／`bing_global` 对长技术查询返回无关回退页
  （实测三次：Windows Terminal 通用结果、bilibili 等热门站）；`duckduckgo` 经代理 `network_error`。
- **本地浏览器车道可用**：可见 Edge ＋ 带登录态的 profile 副本
  （`%TEMP%\orz_retrieval_browser_profile`，含 Cookies／Login Data）。
  `google_browser` 与 `baidu_browser` 返回相关结果。
- **登录态与客户端不是 Bing 的瓶颈**：`bing_browser` 即使带登录态、放宽关键词
  （`terminal bench`／`Terminal-Bench 评测`／`tbench.ai` 等）仍回退；
  对照查询 `rust programming` 返回 rust-lang.org 等正确结果
  ⇒ 判定为**Bing 对该小众主题的索引覆盖缺口**，非登录或客户端问题。
- 本审计外部事实取自 `google_browser` 与 HTTP fetch；**二手聚合源已逐条标注**。

---

## §2 候选逐项核证

### §2.1 GAIA（General AI Assistants）

- **形态**：450 题、三档难度（L1 可由很好的 LLM 突破，L3 为能力跃迁）；
  Princeton HAL 榜在其**公开验证集 165 题**上评测。
- **能力面**：推理＋多模态（图／表／PDF／音频）＋网页浏览＋工具使用。
- **判分**：exact-match（规范化后精确匹配）——判分客观且便宜。
- **参考读数**：人类约 92%；聚合源记顶尖 agent 约 74.6%（Claude Sonnet 4.5，二手）。
- **榜源**：Princeton HAL 榜与 HF Space 榜；**HAL 已暂停更新新模型，转向可靠性度量**。
- **来源**：Mialon et al., 2023（arXiv 2311.12983）。

### §2.2 Aider Polyglot

- **形态**：225 道 Exercism 难题，跨 C++／Go／Java／JavaScript／Python／Rust；
  每题**两次机会**，第一次失败后给出单元测试错误反馈（考「按反馈改对」）。
- **判分**：单元测试；**无容器、无长程**——三条候选里成本结构最低。
- **榜源**：aider 官方榜、Epoch AI、LLM-Stats（后者记 22 模型，榜 GPT-5 0.880）。
- **同族**：项目已有 EVAL-POLYGLOT 存档线（`docs/POLYGLOT_BENCHMARK_FINDINGS_2026-08-06.md`）。

### §2.3 τ²-bench（tau-squared-bench）

- **来源**：Sierra Research（`sierra-research/tau2-bench`）。
- **形态**：**双控制**（用户也能对世界状态操作）——Telecom 域建模为 Dec-POMDP；
  组成式任务生成器保证域覆盖与复杂度可控；配可靠用户模拟器。
- **域**：airline／retail／telecom／banking（含 knowledge 子域）。
- **能力面**：工具调用＋策略遵循＋**知识检索**（约 700 文档知识库检索后应用政策）＋
  人机协调沟通；提供「推理错误 vs 沟通／协调错误」的细分归因。
- **榜源**：Artificial Analysis τ²-Bench Telecom 榜（报每题成本与 token 拆解）；官方 taubench.com。
- **版本纪律**：v1.0.1 判分更新（修 banking_knowledge 题错）——**跨版本结果不可比**。

---

## §3 与项目能力面的匹配

| 候选 | 对应项目面 | 既有摩擦／立项可承接处 |
|---|---|---|
| GAIA | 检索车道、附件／PDF 读取、工具面、多步推理 | 检索投量与转化（EVAL-TB21-RETRIEVAL-VOLUME-CONVERSION）、URL 完整性（0ax）、PDF 证据（GAP-PDF-EVIDENCE） |
| Aider Polyglot | 编辑面、写控、工具反馈循环 | 编码门（GAP-ENCODING-GATE）、lossy 细化（0as）、写控三件（0cb／0cq／0ch） |
| τ²-bench | 工具编排与订单面、策略拒绝、知识检索 | CLASSICAL-EXEC-ASSISTANT、`policy_denial`（FUS-CONSOLE-POLICY-DENIAL）、检索结构化结果 |

三者的共同结构意义：**都不依赖 Harbor 容器编排**。GAIA 需出网与文件读取；
Polyglot 为纯文件编辑；τ² 需 API 数据库与用户模拟器。这与 TB／SCBench 的 Docker 多容器
成本结构不同，是本批单列的主要理由。

---

## §4 成本量级（结构，非官方口径）

| 候选 | 题量 | 单遍试次 | 5 试次跑量（派生） | 判分 | 基础设施 |
|---|---|---|---|---|---|
| GAIA | 165（公开验证集） | 165 | **825** | exact-match | 出网＋浏览器；无容器 |
| Aider Polyglot | 225 题 | 450（每题 2 次机会） | **2,250 attempts** | 单元测试 | 无容器 |
| τ²-bench | 待勘 | 待勘（含用户模拟器额外模型调用） | 待勘 | 规则＋模拟器判定 | API 数据库＋用户模拟器 |

- 「5 试次跑量」为按题量×试次派生的估算，**非官方公布值**。
- 对照：TB 2.1＝89×5＝445 trials。GAIA 与 Polyglot 的量级与 TB 同阶或更低；
  τ² 的成本需按 Artificial Analysis 的「每题成本」口径单独勘。

---

## §5 领域级风险（与选型直接相关）

- **污染与 reward hacking**：2026-04 UC Berkeley 研究称八大 agent 基准均可被 reward-hack 到接近 100%；
  OpenAI 在确认评测集泄漏后已停止报告 SWE-bench Verified 分数（二手聚合）。
- **脚手架主导**：254,254 次 SWE-bench 提交的审计显示，同模型不同 scaffold 的分数跨度达 **29.8pp**，
  超过前三十名之间 8.8pp 的总跨度；**换 harness 的影响大于多数轮次**。
- **排序不可读**：同审计显示 SWE-bench Verified 前三十的 29 对相邻名次，
  在 α=0.05 下**没有一对**能被配对检验分开。
- 由此对小样本基准（GAIA 165、Polyglot 225）要求：**固定题集 pin ＋ k≥5 ＋ 逐题配对比较**；
  差距 <3pp 不下排序判断。

---

## §6 建议批序与判据

每条线建议按 S0–S4 形态（沿项目既有先例）：

- **S0 勘定**：数据集仓库与 revision pin 钉死＋判分器核证＋许可核证＋与既有线装置差异清单。
- **S1 冻结**：题集 manifest（含逐题 sha256）＋批次划分。
- **S2 适配核证**：自建驱动脚本（**非 Harbor 系**）核证；容器内／外可行性。
- **S3 冒烟**：1–2 题真机冒烟＋读数收取面核证。
- **S4 轮次**：k≥5 串行，逐题 `result` 机械提取，报告档落档后回查闭合门。

建议判据（首轮）：

1. 首轮目标是**摩擦产出**，不作对外成绩主张（沿 152 批裁决）。
2. 必须固定题集 pin；未钉 pin 的读数不可比，不得跨版本合并。
3. 报告须给**逐题结果＋聚合＋k 与方差**，并标注脚手架。
4. 与 SCBench／TB 线**分账**，不合并统计。

建议顺位：**GAIA 先行**（认可度与匹配度最高、量级可控）→ **Aider Polyglot**（最便宜、可高频）→
**τ²-bench**（工具／策略轴，待成本勘定）。

---

## §7 边界与未决项

- **未做**：任何真机跑批、任何源码／载体重建、任何记账文件改动。本件为选型审计。
- **批号未领**：避与并行在跑轮次撞号，登记留待台账批。
- **待勘项**：
  - τ²-bench 的题量与每题成本（Artificial Analysis 有口径，未逐项取回）。
  - GAIA 的浏览需求是否撞「容器内无浏览器」缺口（TB 轮实测 14 次 launch failure）——
    须先确认 GAIA 是否可在无头容器内解析，或需浏览器车道。
  - 各线判分器是否可离线运行、出网许可范围。
- 本文外部事实的来源分级：官方页（tbench.ai／sierra-research／hal.cs.princeton）为一级；
  聚合站（codingfleet／rapidclaw／benchmarklist 等）为二级，已在 §8 标注。

---

## §8 来源

一级（官方／论文）：

- GAIA：<https://hal.cs.princeton.edu/gaia>；<https://huggingface.co/spaces/gaia-benchmark/leaderboard>；
  论文 <https://arxiv.org/pdf/2311.12983>
- Aider Polyglot：<https://aider.chat/docs/leaderboards/>；<https://epoch.ai/benchmarks/aider-polyglot>
- τ²-bench：<https://github.com/sierra-research/tau2-bench>；<https://taubench.com/>；
  <https://artificialanalysis.ai/evaluations/tau2-bench>；论文 <https://arxiv.org/pdf/2506.07982>
- Terminal-Bench（对照）：<https://www.tbench.ai/>；<https://github.com/harbor-framework/terminal-bench-2-1>；
  <https://fast-agent.ai/benchmarks/methodology/terminal-bench-2-1/>
- SWE-bench Pro（对照与风险）：<https://labs.scale.com/leaderboard/swe_bench_pro>；
  <https://epoch.ai/benchmarks/swe-bench-pro/review>；<https://arxiv.org/html/2609.08149v1>；
  <https://arxiv.org/html/2609.17394v1>

二级（聚合，仅作交叉核对）：

- <https://codingfleet.com/blog/terminal-bench-leaderboard-2026/>
- <https://rapidclaw.dev/blog/ai-agent-benchmarks-2026>
- <https://llm-stats.com/benchmarks/aider-polyglot>
- <https://www.vals.ai/benchmarks/swebench>
- <https://news.ycombinator.com/item?id=48837396>（OpenAI 审计综述）
- <https://metr.org/notes/2026-03-10-many-swe-bench-passing-prs-would-not-be-merged-into-main/>

---

## §9 增补（2026-10-11 同日续查）：成本、提交政策、饱和度与选型裁决

### §9.1 内账标尺（一手：项目既有轮次实收）

| 轮次 | 规模 | 实收 | 单位成本 |
|---|---|---|---|
| SCBench V1（连续形态，0cr） | 36 题／196 档 | **¥157.49 ≈ $22.0** | ¥0.80/档、**$0.112/档**；1,447.7M tokens |
| SCBench V2（重置形态，0cw） | 196 档 | **¥212.41 ≈ $30.4** | ¥1.08/档 |
| TB 2.1（89 题） | 89 题、115 作业目录、33.8h 墙钟 | **账面无 RMB 记录** | — |

- 由 SCBench V1 反推混合单价 ≈ **¥0.109/百万 token**（国庆低谷价、缓存命中 47.9%）。
- 缺口登记：TB 2.1 整轮未记 RMB 花销（`docs/TB21_V41_*` 四份均无），致跨线成本对照缺一极。

### §9.2 成本量级估算（派生，非账面）

方法：成本 ≈ 跑量 × 每题 token × ¥0.109/M。三条均**不用容器**，省基建不省 token；
主不确定项＝每题 token 消耗。

| 候选 | 跑量（5 试次口径） | 每题 token 假设 | 估算 |
|---|---|---|---|
| Aider Polyglot | 225×2 attempts×5 ＝ 2,250 | 20–50k | **¥5–12** |
| GAIA | 165×5 ＝ 825 | 0.1–0.5M | **¥9–45**（检索长尾重则 ¥90+） |
| τ²-bench | 题量待勘（按约 300 假设）×5 ＝ 1,500 | ~0.3M（含用户模拟器） | **¥30–150** |

合计大概率 **< ¥150**（约 0.5–1 个 SCBench 轮）。

### §9.3 社区提交政策（外部，逐条核证）

| 基准 | 提交面 | 实测口径 |
|---|---|---|
| GAIA | **有门槛** | HF Space 明示「账号 HF 活跃度不足不得提交，防刷分」；Princeton HAL 榜已暂停更新新模型 |
| Aider Polyglot | 开放自报 | 榜由维护者收；官方页自述最后更新 2025-11-20，活跃跟踪转至第三方（Epoch AI／llm-stats／evals.report） |
| τ²-bench | **最开放** | 官方 `docs/leaderboard-submission.md`；Sierra 新榜为社区驱动，带完整轨迹经独立验证后标 `verified` |
| TB 2.1（对照） | **关闭** | 只收维护者跑的结果；上传要求每题 ≥5 trials |

### §9.4 饱和度（头部读数；越高越挤）

| 基准 | 头部 | 余量判断 |
|---|---|---|
| GAIA | 顶尖 agent ~74.6%（人类 92%） | 余量最大 |
| **τ²-bench** | **80.0%**（Airline，2026-10-10 OpenRouter 连续跑） | 有 20pp 余量 |
| Aider Polyglot | **88.0%**（GPT-5 high，2026-03 快照） | 有 12pp 余量；分布 88.0／84.9／83.1…61.0（跨度 27pp） |
| TB 2.1 | 90.6%（公共榜） | 开始挤 |
| SWE-bench Verified | 97.0% | 已饱和（Vals 停跑） |

- τ² 的性质注记：OpenRouter 自述选 τ² Airline 因其**地板高**、用于测 **provider／基建差异**而非模型能力
  ⇒ 对 harness 层摩擦敏感，与项目「找摩擦」目标同向。

### §9.5 用户裁决（2026-10-11）

1. **GAIA：降为自跑专用**——账号 HF 活跃度不足、不可提交，且 HAL 停更；保留自跑找摩擦用途。
2. **Aider Polyglot：不采纳**——用户裁决「GPT-5 跑 88%，模型能力代差太大，跑了也没用，没办法干净归因」。
   **边界注记**：该 88.0% 系 2026-03 快照，未见 DeepSeek V4.x 在该榜读数；
   而 DeepSeek V4.1 Flash 现为 TB 2.1 公共榜第一（90.6%）、V4 Pro 在 SWE-bench Verified 列第二（96.4%）
   ⇒ 「代差大」前提在本榜缺乏直接证据。若日后重启该线，先补此缺口再判。
3. **τ²-bench：选定先行**——理由＝提交面最开放（带轨迹可获 `verified`）、饱和度 80% 尚有余量、
   且对 harness／基建差异敏感（可干净归因）。
   **专项口径与 S0 勘定见** [`252 批 τ²-bench 评测线立项与指标口径审计`](252_TAU2_BENCH_LINE_SELECTION_AND_PROTOCOL_2026-10-11.md)。

### §9.6 增补来源

- 内账：`docs/audits/215_SCB_COST_BREAKDOWN_2026-10-07.md`；`docs/SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md` §14；
  `docs/SCB_V2_RESET_FORM_FULL_ROUND_REPORT_2026-10-11.md`；`docs/audits/233_0CW_S3_FULL_ROUND_COMPLETE_2026-10-10.md`
- GAIA 提交门槛：<https://huggingface.co/spaces/gaia-benchmark/leaderboard/discussions/50>
- Aider 饱和度：<https://www.codesota.com/news/gpt-5-aider-polyglot-leader>
- τ² 提交：<https://github.com/sierra-research/tau2-bench/blob/main/docs/leaderboard-submission.md>；
  榜：<https://openrouter.ai/benchmarks/tau2-bench-airline>；<https://artificialanalysis.ai/evaluations/tau2-bench>
- τ² 定位与社区驱动说明：<https://sierra.ai/jp/blog/t-bench-leaderboard>
