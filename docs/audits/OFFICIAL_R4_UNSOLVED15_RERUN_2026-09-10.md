# 官方 R4 未通过 15 题复跑记录（official-r4-unsolved15，2026-09-10）

> 日期：2026-09-10 用户裁决放行（0u），当日 00:58–06:56 执行。
> 定位：R3 未通过 15 题按官方口径再跑一轮 + **0t S4 实机复验载体**（判据见
> [`RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md)
> §5，判据预登记见 BACKLOG 0u / TODO P0-0u）。
> 口径：与 R3 一致——继承成绩、k=1、一题一作业、官方数据集 pin
> `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`、官方墙钟唯一、
> deepseek-v4-flash、eval_browser=true、Docker 容器。
> 环境裁决（用户）：**无代理直连**（机场波动不开 Clash）+ 本地预拉镜像
> （15/15 在位，不依赖 Docker Hub）——即 0t 双车道的目标真实环境。
> 载体：**orz 0.4.0 正式发布三件套**（`D:/tb-eval/orz-linux/orz`，
> 109,066,552 B，SHA256 `0797610e…` =
> [`0.4.0 发布审计`](0.4.0_RELEASE_2026-09-09.md) 锁定值；orz `a467d0f9`
> = 0t S3 `92875fd5` + 版本 bump，双车道代码同一）——双车道代码首次官方
> 口径实机。
> 执行器：[`run_r4_unsolved15_per_task.py`](../../scripts/run_r4_unsolved15_per_task.py)
> （Python argv 直传，规避 PS 5.1 引号破坏教训）；job 前缀
> `official-r4-unsolved15-<task>`；TEMP 重定向 `D:/tb-eval/tmp`
> （用户裁决：本跑批不写 C 盘；Docker 数据盘
> `D:\DockerData\DockerDesktopWSL\disk\docker_data.vhdx` 本在 D 盘）。

## 1. 执行结构

- 00:52 首次启动；00:58 按「不写 C 盘」裁决主动中止重启（TEMP 改 D），
  作废尝试仅 adaptive-rejection-sampler 一个部分 run（无结果，k=1 不计）。
- 15 题串行一题一作业；filter-js-from-html pass-1 无效（见 §3），pass-2 补跑。
- 06:25 DeepSeek 账户余额耗尽（详见 §3），末尾 4 个作业受影响。

## 2. 成绩：官方 reward 面 1/15；有效真实试次 13/15

| 任务 | 官方墙钟 | journal 墙钟 | 真实试次 | reward | 结束方式 |
|---|---|---|---|---|---|
| model-extraction-relu-logits | 900s | 451s | ✓ 完整 | **1.0 通过** | run_finished:completed（两阶段 submit） |
| adaptive-rejection-sampler | 900s | 891s | ✓ 完整 | 0.0 | 自然完成（verifier 不过；单轮 440s 长轮） |
| path-tracing | 1800s | 891s | ✓ 完整 | 0.0 | 自然完成 |
| dna-assembly | 1800s | 1913s | ✓ 完整 | 0.0 | AgentTimeout（journal 面无终止事件） |
| dna-insert | 1800s | 1831s | ✓ 完整 | 0.0 | AgentTimeout |
| extract-elf | 900s | 1011s | ✓ 完整 | 0.0 | AgentTimeout |
| extract-moves-from-video | 1800s | 1841s | ✓ 完整 | 0.0 | AgentTimeout |
| gcode-to-text | 900s | 914s | ✓ 完整 | 0.0 | AgentTimeout |
| gpt2-codegolf | 900s | 806s | ✓ 完整 | 0.0 | AgentTimeout |
| make-doom-for-mips | 900s | 951s | ✓ 完整 | 0.0 | AgentTimeout |
| make-mips-interpreter | 1800s | 1825s | ✓ 完整 | 0.0 | AgentTimeout |
| filter-js-from-html | 1800s | 1808s | pass-1 真实但无效 | —（pass-2 为 verifier-only 0.0） | pass-1 `run_failed`: 流断连（stream idle，0 re-sends）；pass-2 未运行（余额） |
| path-tracing-reverse | 1800s | 1272s | ◑ 中断 | 0.0（verifier-only） | 06:25 余额耗尽中断 `run_failed`，3 次重试秒死 |
| protein-assembly | — | — | ✗ 未运行 | 0.0（verifier-only） | 4 次尝试全部 `Insufficient Balance` 秒死 |
| train-fasttext | 3600s | — | ✗ 未运行 | 0.0（verifier-only） | 4 次尝试全部 `Insufficient Balance` 秒死 |

- **官方账面**：R3 后 63/89 → R4 后 **64/89**（唯一翻案 = model-extraction，
  即 [0S 细节分析](0S_DETAIL_ANALYSIS_2026-09-09.md) FP-7 预测的 verifier
  网络运气题——本轮 `test.sh` 的 uv 下载成功，模型解法首次获评）。
- **机制口径**：12 题完整真实试次 + path-tracing-reverse 中断试次 +
  filter-js pass-1 无效试次；protein-assembly / train-fasttext 的 reward 0.0
  为 verifier-only，**不计入机制对照**。

## 2A. r4b 补跑结果（2026-09-10，余额恢复后同日执行；批次 `official-r4b-unsolved4`）

| 任务 | 真实 run | reward | 结束方式 |
|---|---|---|---|
| path-tracing-reverse | 481 事件 | **1.0 通过** | run_finished:completed（58 轮、journal 16min、**零检索调用**——纯终端 + submit 两阶段，命中 91.63%） |
| protein-assembly | 464 事件 | 0.0 | 官方墙钟耗尽 `AgentTimeoutError`（journal 无终止事件，末条为 `tool_completed`；题目域，同 R3 形态） |
| train-fasttext | 717 事件 | 0.0 | 3600s 官方墙钟耗尽 `AgentTimeoutError`（长命令，同 R3 形态） |
| filter-js-from-html | pass-1 572 事件 reward 缺失（FAIL 判据正确）；pass-2 763 事件 | 0.0 | pass-2 `run_failed`（模型流中断 `stream interrupted after 0 re-sends`）+ 官方 `AgentTimeoutError`（非自然结束） |

- **官方账面：63（R3）→ 64（R4 model-extraction）→ 65/89（r4b path-tracing-reverse）**。
- 口径注记：path-tracing-reverse 本轮通过**未经检索车道**（纯终端路径）——R3
  它死于检索依赖（browser 报废 + web_search 慢通道），本轮模型以本地知识 +
  终端自验解出；「检索主导 4 题」在补跑轮 1/4 翻案，翻案载体是模型路径
  选择而非车道修复。
- 余额事件收口：四题全部真实试次完成，§3 余额影响边界解除。
- **结束方式复核（2026-09-10 现场核对，用户质询触发）**：本条初稿把
  protein-assembly 与 filter-js pass-2 记为「自然结束」，与 `result.json`
  的 `exception_stats` 和 journal 末条不符，已按证据改写为上表；三题
  （protein-assembly / train-fasttext / filter-js pass-2）实际均以官方墙钟
  耗尽 `AgentTimeoutError` 收尾。口径影响：filter-js pass-2 的 0.0 含
  DeepSeek 流中断成分，**不是干净题面试次**，后续归因按「环境受影响」
  标注，不作纯题目域证据；path-tracing-reverse 为该批唯一无异常试次。
- filter-js pass-1 verifier 无 reward（R3 FP-7 verifier 脆弱同族形态），
  结果以 pass-2 为准；执行器硬化判据（journal 真实 run >10 事件）本轮
  生效（余额秒死 stub 均被正确判 FAIL——本轮无 stub 复发，判据为防御性）。

## 3. 余额事件（影响边界）

- `run_failed {"error": "model error: model error: Insufficient Balance
  (code=invalid_request_error)"}`。
- 时间线（UTC，本地 +8）：**06:25 本地** path-tracing-reverse 真实 run 于
  22:25Z 被余额中断（1272s / 1800s）；其后 3 次重试秒死；protein-assembly
  （22:32–22:37Z ×4）、train-fasttext（22:40–22:50Z ×4）、filter-js pass-2
  （22:52–22:56Z ×4）全部秒死，harbor 以 verifier-only reward 0.0 收尾。
- **执行器完成判据盲区登记**：`job_complete`（finished_at + 非空 reward 键）
  无法区分「agent 真实试次」与「余额秒死 + verifier-only reward」——
  R3 `045f91a` 修掉 AgentSetupTimeout 掩蔽后，余额秒死是同族新掩蔽形态。
  后续批次判据应加「journal 真实 run 存在（事件数 >10）」一条。
- 处置：余额充值属用户侧动作；4 个受影响题（path-tracing-reverse 有效
  中断 / protein / train-fasttext / filter-js pass-2）如需机制口径成绩，
  待充值后小批补跑（是否补跑待用户裁决）。

## 4. 0t S4 判据 1–5 判定（13 个真实 run + 全卷清扫）

| 判据 | 结论 | 证据 |
|---|---|---|
| 1 web 族零拒绝 | **过** | 全部 journal 零 `retrieval_mode_requires_framework_fallback` 拒绝；检索任务自由混用双车道（dna-assembly：web_search 15 + web_fetch 9 + browser_read 11 + browser_control 1） |
| 2 静态标注存在性 | **过** | 运行二进制（0.4.0，sha256 `0797610e…`）符号：`[车道:本地浏览器检索…` ×2、`browser_control` ×25、`browser_launch_result` ×3（与 S3 符号表同形）；无动态健康度字段 |
| 3 γ 退役兼容 | **过** | **全部卷（13 真实 run + 全部 stub）零 `retrieval_mode` / `retrieval_mode_transition` / `framework_fallback` 字符串**；`browser_launch_result` 事实事件在案且带真实原因（success：dna-assembly / dna-insert / extract-elf / filter-js；failure：gpt2 / path-tracing-reverse `browser_not_found`，逐字列出搜索过的可执行名）；旧 journal 回放兼容为 S2/S3 既有证据 |
| 4 失败回传形态 | **过** | 真实类别样本：`web_search timed out after 120.0s …（client budget）`、`browser_read refused [browser_read_blocked_scheme]: unsupported URL scheme: chrome-error (only http/https allowed)`、`browser_launch_failed: browser_not_found (ORZ_BROWSER_PATH unset; searched: …)`；全卷清扫零「勿重试」类教学/阻拦文案 |
| 5 通用统计 | **过（附注记）** | 零 `transport_retry`、13 真实 run 零模型面 400（余额 400 属计费环境事件，§3 单列）；命中率（provider 口径 cache_hit/(hit+miss)）：12/13 ≥89.8%，3 个检索重题 <90%（dna-assembly 85.83 / extract-elf 89.82 / gpt2 83.12）——与 R3 dna 82.36 / feal 88.21 同族（web 检索注入摊薄缓存命中），按 0i 先例作成本观察不阻塞 |

- 判据 6（宿主机日常可用性，执行代理操作）另线，不随本批。
- 附带确认：`browser_control` 每检索任务 1 次 `retrieval_role_write_denied`
  （external 车道对黑板写越权被拒）——检索角色写边界机制正确，模型次轮
  自行恢复，非缺陷。

## 5. 与 R3 对照（0s 四分类再归因）

- **检索主导 4 题（0/4 翻案，但死亡形态质变）**：本轮浏览器注入链健康
  （Chromium 155 快照在 4 题注入成功），R3「browser 报废 → web 慢通道
  独木桥」形态消失——双车道行为实证：dna-assembly 双族并用且模型自主
  换道（browser_read 11 次成功侧 + web_search 15 次）；gpt2 / path-reverse
  镜像内无浏览器，`browser_launch_result(failure, browser_not_found)` 后
  模型直接转 web 车道，无任何模式仪式。仍未过的预算杀手变为：无代理下
  web 慢通道自身失败率（dna-assembly web_search 错误墙 1209s、7×120s
  超时 + 4×HTTP 断流）+ 检索子代理 600s 超时 ×2 + chrome-error 重试
  （FP-2 形态按裁决保留正常回传，extract-elf 7 次 163s 错误墙）。
  **结论：双车道机制面按设计工作；无代理环境的 web 慢通道质量是剩余
  瓶颈（用户已裁决节点自理、不作 orz 处置）。**（归因修正见 §5A。）
- **轮次延迟主导 5 题（0/5）**：结构性复现——adaptive 单轮 440s 无事件
  （900s 预算 49%，FP-4 同族）；make-mips 202 次调用终端墙钟仅 18s；
  path-tracing 自然完成但 verifier 不过。与 R3 同判，双车道不作用于此类。
- **长命令主导 2 题**：extract-moves 终端墙钟 951s（TER 4×`tool_running`
  180s 后台化首报正常工作）；train-fasttext 未运行（余额）。
- **verifier·题目域 4 题（1/4）**：model-extraction 通过（FP-7 verifier
  网络运气兑现）；dna-insert（ΔTm 定义分歧）、protein-assembly（未运行）、
  filter-js（pass-1 流断连无效）。

## 5A. 归因修正（2026-09-10 复核，用户质询触发；修正 §5「web 慢通道质量」表述）

调用级证据（dna-assembly 40 次检索调用全序列 + extract-elf 失败目标清单）
不支持「web 慢通道自身质量」的笼统归因，修正为三类分解：

1. **web_search 120s 超时与 46–79s 慢成功 = DeepSeek 服务端搜索后端延迟**
   （Responses API server-side search 在服务侧执行，与本地代理无关；成功
   调用 46–79s、超时打满 120s 客户端预算）。本地网络只作用于响应体流读取。
2. **web_fetch / browser_read 失败呈域名集中性 = 无代理下目标域名本地可达
   问题**：extract-elf 15 次失败目标 = github.com ×7 / huggingface.co ×5 /
   jsdelivr ×2 / duckduckgo ×1（本地不可达或不稳定，10–35s 失败）；可达域
   （neb.com / addgene.org / idtdna / pubmed）web_fetch 1–2s、browser_read
   5–6s 即成功。dna-assembly 同构：wikipedia / web.archive.org /
   protocols.io 失败，NEB/Addgene 成功。
3. **模型换道行为与用户判断一致**：dna-assembly 序列实证——web_search
   超时（seq115）→ browser_read neb.com 5s ok（seq123）→ chrome-error →
   web_fetch 直读 PDF（seq175 1s ok）→ 再回 web_search……双车道全程自适应
   交替；「超时→错误信封（120s 有界返回）→模型转通道」路径成立。后期
   （seq247/249/278）仍重试 web_search 属 FP-2 裁决范围（不抑制、模型自主）。

**dna-assembly 预算死因重述**：1800s ≈ web_search 累计 1209s（7×120s 超时
840s + 慢成功 ~370s）+ 2×检索子代理 600s 窗口到期重委派 + 其余执行；即
DeepSeek 服务端搜索延迟叠加本地不可达域试探耗尽墙钟，双车道机制本身按
设计工作。

**新观察（候选接缝，待用户裁决是否立项）**：`browser_control` navigate 在
external 检索车道被拒 `retrieval_role_write_denied`（dna-assembly seq196，
每检索任务恰好 1 次）——与 0t P1-2b「Phase 1 导航级动作面加入 external
lane」的设计预期相悖，疑为角色写边界对 browser_control 动作分类问题；
登记为 0t 后续候选。

## 6. 机械层正面确认

- 0q 失败管线全程在案：`failure_target` 盖章（url_target/cmd_target/
  anchor_target）+ `failure_agg_absent` 标记（3 次）与事件同源，XOR 无冲突。
- TER 常驻化：extract-moves 4×`tool_running(180s)` 首报、无硬杀误伤。
- 结构化拒绝全部单轮恢复：`Tool not found` 变体（FP-5 形态，每题 ≤4 次、
  含 1 次 `run_command`）、尾随 `&` 拒绝 1 次、`outside_workspace` 读拒绝 3 次。
- 两段门（0p）：make-doom / make-mips 各 1 次 `session_volume_notice`，信封正常。
- 搭车遥测在场（**仅登记，不构成相应条目 S4 闭合**）：`dep_graph` 事件字段
  139 次（P2-11 依赖图）、`ledger_fold_advance` 15 次（P2-12 / P2-14 折叠链）、
  `tool_running` 7 次（TER）；`context_compressed` / `session_archive` 本批为 0
  （无全量压缩触发、任务容器会话未产存档事件）。

## 7. 证据边界

- journal 不含工具返回内容；browser_read「成功」调用无法核验实际内容。
- filter-js pass-1 结果无效（run_failed 无 reward），pass-2 为 verifier-only；
  k=1 口径下 filter-js 本轮无有效机制试次。
- path-tracing-reverse 中断试次（1272s）作参考不作判据基线。
- 命中率为 provider 口径聚合，检索注入摊薄的机制解释沿 0i 先例，未做
  逐请求归因。
- 执行中止重启（00:52→00:58）作废 1 个部分 run，无结果污染（k=1 不计）。

## 8. 后续

- 用户侧：DeepSeek 余额充值（已收口）；4 个余额受影响题补跑（已收口——
  r4b 4/4 真实试次，见 §2A）。
- 0t S4：判据 1–5 本批证据已齐（本审计 + agg4）；判据 6 宿主机日常可用性
  待执行代理另线；闭合入账在 BACKLOG 0t / TODO P0-0t。
- 0v 检索引擎 SERP + browser_control 分类修正：设计定稿待放行
  （[`RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10`](../RETRIEVAL_ENGINE_SERP_SEARCH_ACTION_DESIGN_2026-09-10.md)）。
- 收口：BACKLOG 0u / TODO P0-0u 同步（本审计即落档），计数入账沿各条目
  自身小节（0t S4 闭合时按其小节入账）。

## 9. 账本

- 逐题结果：`D:\tb-eval\jobs-official\official-r4-unsolved15-<task>\result.json`
  （pass-1 无效件：`official-r4-unsolved15-filter-js-from-html-stale-*`）
- gsa 卷：`D:\tb-eval\gsa-volumes\official-r4-unsolved15-<task>\`
- 控制台/摘要：`official-r4-unsolved15-*-console.log`、
  `official-r4-unsolved15-per-task-summary.log`
- 聚合：`D:\tb-eval\analysis\r4\{agg4.py,agg4.json}`（本地件；pin 规则 v3 =
  卷内真实 run（事件数 >10）取最后事件最新者）
- 执行器：[`scripts/run_r4_unsolved15_per_task.py`](../../scripts/run_r4_unsolved15_per_task.py)
