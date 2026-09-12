# 官方账面通过题模型代际补注：V4 Flash / V4.1 Flash 划分（2026-09-12）

> 日期：2026-09-12（用户提问触发的事实补注，非新跑批、非实施项；不改任何
> 账面数值）。当日用户复核指出通过题数量口径需核实，同日以盘面
> `result.json` 全量扫描完成口径对账（§2），代际结论不因口径而变（§4）。
> 范围：TB 2.1 官方批次（R1 / R2 波次 / R3 / R4 / r4b，`D:\tb-eval\jobs-official\`）
> 全部 verifier 1.0 结果的**模型代际归属**——哪些通过 run 跑在真 V4 Flash
> 上、哪些跑在 V4.1 Flash（兼容路由）上。
> 口径：本项目全部官方跑批固定使用模型名 `deepseek-v4-flash`（R3/R4 审计
> 明载）；判定依据 = DeepSeek 官方切换公告（一手来源）+ 各通过 run 的
> verifier/journal 事件线时间戳（盘面证据）。
> 结论摘要：**在全部四种口径下，V4.1 Flash 通过题有且只有 1 题
> （path-tracing-reverse，09-10 14:11–14:27 r4b 补跑）；其余全部通过题
> （64–76 题，随口径 65/74/76/77）均为真 V4 Flash**。代际划分对口径
> 选择完全不敏感；口径分歧本身作为记录面发现一并登记（§3）。

## 1. 官方切换事实（一手来源）

1. **官方公告**《DeepSeek V4.1 Flash：更强、更快、更普惠》
   ([api-docs.deepseek.com/zh-cn/news/news260910](https://api-docs.deepseek.com/zh-cn/news/news260910)，
   页面标注 2026/09/10)：
   - "今天，我们正式发布 DeepSeek V4.1 Flash 模型"——V4.1 Flash 于
     2026-09-10 当日上线；
   - "**旧版本模型 V4 Flash 与 V4 Flash Vision Exp 现已下线**，出于兼容
     考虑，模型名 `deepseek-v4-flash`、`deepseek-v4-flash-vision-exp`
     将被**暂时路由到 V4.1 Flash**"（兼容路由无公布截止日）；
   - "V4.1 Flash 的价格于北京时间 2026 年 9 月 10 日 12:00 开始生效"。
2. **通知邮件与报道时点**：[IT之家 2026-09-10 12:10:07 报道](https://www.ithome.com/1/000/692.htm)
   ——DeepSeek 官方当日向 API 用户发送通知邮件，宣布北京时间 2026-09-10
   正式发布 V4.1 Flash 并执行新 Flash 定价。报道时点 12:10 是"切换已完成
   并公告"的**上界锚点**。
3. **与本项目无关的两项官方动态**（不参与判定，仅登记避免混淆）：
   09-08~09-10 的 V4.1 Flash 限时内测走独立入口，不影响 `deepseek-v4-flash`
   调用；`deepseek-v4-pro` 原定 09-14 12:00 路由切换（后续官方取消该计划）
   ——本项目跑批从未使用 pro 名。

**推论**：调用 `deepseek-v4-flash` 的请求，在 2026-09-10 正午（12:00
定价生效 / 12:10 公告已发出）之前由真 V4 Flash 服务，之后由 V4.1 Flash
兼容路由服务。判定只依赖 run 的**事件线时间**，不依赖模型名（名称全程
未变，这正是本补注成立的前提）。

## 2. 通过记录的批次分布（盘面地面真值）

对 `jobs-official` 全部官方批次（TB 4.0 除外）234 份 `result.json` 全量
扫描：**verifier 1.0 共 81 条，涉及 77 个不同题目**。分布：

| 批次族 | 运行窗口（北京时间） | 通过记录 | distinct 通过题 | 代际 |
|---|---|---|---|---|
| R1 官方全量 c1–c9 | 08-25 21:10 ~ 08-27 | 58 条 | **58** | **V4** |
| R2 波次（900s / c1–c4 / s5-1 / s5-2 / 10T / 2×ARCH） | 08-27 16:43 ~ 08-31 11:57 | 16 条 | **13**（逐条见 §3） | **V4** |
| R3 未通过 20 题复跑 | 09-08 20:05 ~ 09-09 07:22 | 5 条 | **5**：count-dataset-tokens / mteb-leaderboard / raman-fitting / tune-mjcf / write-compressor | **V4** |
| R4 未通过 15 题复跑（晨间） | 09-10 00:58 ~ 06:56 | 1 条 | model-extraction-relu-logits（其 08-27 ARCH 结果已存在，对"曾通过"集不新增） | **V4** |
| r4b 补跑 4 题 | 09-10 12:31 ~ 16:19 | 1 条 | **path-tracing-reverse**（journal `RUN-CLI-6aa249f6`：北京 14:11:03 → 14:27:11，verifier 06:27:08Z→06:27:47Z，reward 1.0） | **V4.1** |

R3/R4/r4b 未通过题、切换后非成绩调用（0w 09-11 / 0x S4 09-11 / 0v S4
二批 09-12，reward 全 0）不携带通过记录，不参与划分；R1 之前的 08-08/09
探索性批按项目 08-08 勘误属 pre-beta 工程诊断、不在 89 题官方账面宇宙内，
仅在 §6 附注。

## 3. 口径对账：65 / 74 / 76 / 77 的由来

用户 2026-09-12 复核指出"通过题目数量不止 65"。盘面证实：**65 是
[R4 审计](OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)"官方账面"的窄口径
（R1 58 + R3 5 + R4 1 + r4b 1），未含 R2 波次复验通过题**；而 R3 选样时
用的是"继承成绩口径"。四种口径与盘面记录的对应关系（彼此可完全调和）：

| 口径 | 通过题数 | 构成 |
|---|---|---|
| ① R4 审计"官方账面" | 65 | R1 58 + R3 5 + R4 1 + r4b 1（R2 波次一律不计） |
| ② 08-31 用户裁决"最低口径"登记（`31 题已解 9`，TODO 0j W4-R4 S4） | 74 | ① + R2 登记通过 9（08-29 复验 8：build-pov-ray / caffe-cifar-10 / circuit-fibsqrt / largest-eigenval / mteb-retrieve / pytorch-model-cli / query-optimize / video-processing + 10T rstan-to-pystan 1） |
| ③ R3 选样"继承/曾通过"口径（档案 best-reward，不含 ARCH） | 76 | ② + 2 题"曾通过但未复验"（chess-best-move、torch-pipeline-parallelism，均 08-27 900s 波） |
| ④ 盘面全量 verifier 1.0（含被覆盖结果） | 77 | ③ + 2 条 ARCH 结果（model-extraction-relu-logits 08-27 23:22、protein-assembly 08-28 00:36，见下） |

**ARCH 覆盖关系（④ 与 ③ 的差异来源）**：`official-r2-failures-c1-ARCH-20260828-6cc8586`
（model-extraction 1.0）与 `-c2-ARCH`（protein-assembly 1.0）在 08-28 评测
侧改造（6cc8586）后被 live 批次重跑覆盖（c1 08-28 23:46 → 0.0；c2 08-29
01:16 → 0.0）——两题在后续所有 live 轮（R3/R4/r4b）均未再通过。R3 文档
"档案逐题 best-reward 计 11 题曾通过" = ② 的 9 + ③ 的 2（不含 ARCH），
与盘面 13 = 11 + 2 ARCH 精确吻合。

**记录面发现（2026-09-12 用户裁决：口径冲突保留，不并轨）**：R4 审计
"官方账面 63/89"（= R1 58 + R3 5）的算术**未继承** 08-31 已裁决登记的
R2 期通过 9 题，与 R3 选样"继承成绩口径"（11 题）不一致；BACKLOG 0u
沿用了 63→64→65。用户裁决（2026-09-12）：**目前成绩并不可靠，口径冲突
保留即可，不做 canonical 并轨、不统一回写**——65/74/76/77 各口径并存
作为记录面现状保留，本补注的代际判定覆盖全部口径、不受该裁决影响。

## 4. 代际划分结论（对口径不变）

**在全部四种口径下：V4.1 Flash 通过题 = {path-tracing-reverse}，有且仅有
1 题；其余全部为真 V4 Flash。**

- **V4 Flash**：R1 全部 58 题、R2 波次全部通过题（13 或其子集，视口径）、
  R3 五题、R4 model-extraction-relu-logits（晨间批 05:38–05:46，切换前
  约 6 小时余；其 08-27 ARCH 通过同为 V4）——通过记录时间上界
  09-10 05:46，早于切换点。
- **V4.1 Flash**：path-tracing-reverse（r4b，09-10 14:11–14:27，切换后
  约 2 小时）——该题通过未经检索车道（纯终端 + submit 两阶段），详见
  [R4 审计 §2A](OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md)。

切换后全部其他调用（r4b 其余三题 0.0、0w 09-11、0x S4 09-11、0v S4 二批
09-12）均跑在 V4.1 上且零通过，故 V4.1 对任何口径通过集的贡献恰为 1 题。

## 5. 影响面

1. **账面口径**：无论采纳何种口径，通过集均为**跨两个模型代际的混合
   成绩**，与官方榜单或其他单一模型成绩对照时不得视为同代可比。
2. **0t S4 归因补强**：R4 审计 §5A"检索主导 4 题 1/4 翻案、翻案载体是
   模型路径选择"的表述需追加一层——翻案发生在 **V4.1 代际**（与 R3 的
   V4 代际检索环境失败样本对照时，模型代际是新增混杂变量，与车道修复
   的因果贡献应分开陈述）。
3. **FP-7 归因不变**：model-extraction-relu-logits 的"verifier 网络运气"
   在 R4 晨间批（V4）兑现；其 08-27 ARCH 通过与 08-28 live 复跑失败
   （0.0）同为该 verifier 网络脆弱性的历史样本，代际均为 V4。
4. **后续跑批默认代际**：官方兼容路由无截止日，此后一切
   `deepseek-v4-flash` 调用默认视为 V4.1 Flash（0z S4 及后续批次同此）；
   跨代际成绩对照必须标注代际。若官方公告恢复独立 V4 Flash 或给出
   路由截止日，需另行复核本补注的边界。
5. **口径冲突保留（2026-09-12 用户裁决）**：§3 记录面发现的四种口径
   并存现状予以保留，不做 canonical 并轨、不统一回写 R4 审计 / BACKLOG
   0u（理由：目前成绩并不可靠）。代际补注覆盖全部口径，后续引用通过
   题数时须注明所用口径。

## 6. 残余不确定性

- 官方未公布路由切换的**精确分钟**。可锚定：12:00 新定价生效、
  12:10:07 IT之家已报道通知邮件（切换完成并公告的时点上界）。
- R4 晨间批结束于 06:56、r4b 开始于 12:31，两批与"正午切换"之间各有
  数小时间隔，**两道关键通过题的代际判定对切换时刻在 06:56–12:31 区间内
  的任何取值都不敏感**（分界稳健）。唯一可推翻 V4 归属的假设是"午夜
  00:00 已切换"，与定价/邮件/报道证据链不符，不采纳。
- 本补注依赖"兼容路由自公告时点生效"的官方表述（"现已下线"）；若
  官方内部存在灰度切换窗口，只可能影响 09-10 正午附近的调用，不影响
  上述两批的判定。
- R2 波次的 16 条通过记录（13 distinct）判定代际仅依据批次窗口
  （08-27~08-31），未逐条开卷核对 journal——窗口与切换点间隔 10 天以上，
  无灰度可能，逐条核验无增量信息。
- 附注：08-08/09 探索性批（pre-beta 工程诊断）另有 10 题 best-observed
  通过记录（含 dna-assembly 08-08 16:02 单次 PASS），按项目 08-08 勘误
  不在 89 题官方账面宇宙内，不入本划分；若用户裁决将其纳入"曾通过"
  口径，需另行核对各题与 R1 的重叠（多数已在 R1 58 题内）。

## 7. 证据清单

- 官方公告：https://api-docs.deepseek.com/zh-cn/news/news260910 （2026/09/10）
- 官方更新日志：https://api-docs.deepseek.com/updates/
- IT之家报道（12:10:07）：https://www.ithome.com/1/000/692.htm
- model-extraction-relu-logits（R4，V4）：
  `D:\tb-eval\jobs-official\official-r4-unsolved15-model-extraction-relu-logits\model-extraction-relu-logits__ZgGjN9o\`
  （journal `agent/gsa/runs/RUN-CLI-6aa1d1ea/events.jsonl` 132 事件；
  `result.json` reward 1.0）
- path-tracing-reverse（r4b，V4.1）：
  `D:\tb-eval\jobs-official\official-r4b-unsolved4-path-tracing-reverse\path-tracing-reverse__zmB8FSK\`
  （journal `agent/gsa/runs/RUN-CLI-6aa249f6/events.jsonl` 481 事件；
  `result.json` `verifier_result.rewards.reward = 1.0`）
- R2 波次通过记录：`D:\tb-eval\jobs-official\official-r2-failures-*\`
  各 trial `result.json`（§3 逐条窗口即 verifier `finished_at`）
- 批次文档：[R3 复跑审计](OFFICIAL_R3_UNSOLVED20_RERUN_2026-09-09.md) /
  [R4 复跑审计（含 §2A r4b）](OFFICIAL_R4_UNSOLVED15_RERUN_2026-09-10.md) /
  [R2 10T 复验](OFFICIAL_R2_FAILURES_RECHECK_10T_2026-08-31.md) /
  [0S 细节分析](0S_DETAIL_ANALYSIS_2026-09-09.md) /
  [BACKLOG 0u](../BACKLOG_AND_PRIORITIES.md) / TODO 0j W4-R4 S4 行
