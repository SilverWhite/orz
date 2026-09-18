# 检索投量与转化深入分析（TB 2.1 三题验证轮 r1–r3，2026-09-19）

用户令：「请进行进一步分析吧，看看检索部分『检索投量与转化』方面框架中我们还能够做什么优化」。
本文是纯分析件：**零代码、零子仓改动、计数不变**；结论只从 r1／r2／r3 九试次的
journal／trajectory 一手证据推出，涉及设计裁决的部分一律列在 §5／§9 待裁决，不在本文生效。

## §0 结论摘要

1. **检索失败已不是瓶颈**（§10.4／§10.9 已定），瓶颈是**投量的额度轴错配＋转化窗口被排队派发吃掉**。
2. 主 agent 整轮只发出 1–2 次检索派发，真正的调用量发生在子代理内部（**放大 4–23 倍**）；
   每批派发**按 activation 重新起算额度**，run 级无累计上限 ⇒ 单 run 内检索族占用可达 54.6–93.5 % 墙钟。
3. **转化的机制性断点**：同一轮内发出的多个检索派发**串行执行且不给主 agent 中间回合**——
   6 个有派发的试次里，**4 次在首批结果提交后主 agent 拿到 0 个回合**
   （r3 torch 首批提交后剩余 554 s 全部被排队第二批吃掉；r2 extract-elf 297 s、r2 gpt2 228 s 同形）。
4. 框架侧有三处**已设计但空转**的部件：信息充分性判定恒 `indeterminate`、投递族事件在官方口径零出现、
   effort 档位对 web 车道的**地板恒为 extended**（standard 结构上不可达）。
5. 全部候选优化都在**装置／框架侧**，不涉任务镜像、verifier、数据集 pin（沿用 2026-08-29 与 §10.8 裁决）。

## §1 口径与证据

### §1.1 数据来源

- 作业：`official-verify-timeout3`（r1，宿主持不稳代理）／`official-verify-timeout3-r2`（r2，去代理直连）／
  `official-verify-timeout3-r3`（r3，直连＋镜像本地化）；口径见 [`校验轮记录 §10.1`](TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md)。
- 一手证据：九试次 journal `events.jsonl`（`D:\tb-eval\gsa-volumes\<job>\<session8>\runs\<run>\`）＋
  trajectory（`D:\tb-eval\jobs-official\<job>\<trial>\agent\trajectory.json`）。
- 本文所有读数由 journal 事件重算，不引用既有摘要数字；与 §10.4／§10.9 的口径差异已在各表标注。

### §1.2 切段口径与归属勘误

**派发窗口**定义：`request_header_change{agent_role:"external_retrieval"}` 起，至
`retrieval_close_record` 止；未闭合窗口（被墙钟截断）计到 run 终态。

**勘误（重要）**：journal **不回写「回主车道」的 `request_header_change` 事件**——子代理交回主车道后，
主车道的模型回合与工具调用仍落在最后一条 header 段内。初稿据此按 header 切段，把 extract-elf r3 的
落盘动作误记为子代理动作；以 close 时刻核对后更正：该 run 的 `retrieval_close_record` 在 **17:27:05**，
而 `/app/extract.js` 的 `search_replace` 在 **17:28:59**、`submit` 在 17:29:53 ⇒ **交付物由主 agent 产出**，
与设计分工（子代理检索、主 agent 转化）一致。
⇒ 该缺口的价值在于**审计面**：任何按 header 切段的角色归属统计都会失真，登记于 §5.3。

## §2 投量读数

### §2.1 派发数与子代理内执行数（放大比）

| 轮次／题 | 主 agent 派发（`web_search`） | 子代理内检索执行（`web_search`＋`web_fetch`＋browser） | 放大 |
|---|---|---|---|
| r1 torch-pipeline | 2 | 46 | 23.0× |
| r1 gpt2-codegolf | 2 | 43 | 21.5× |
| r1 extract-elf | 1 | 9 | 9.0× |
| r2 extract-elf | 2 | 38 | 19.0× |
| r2 gpt2-codegolf | 2 | 39 | 19.5× |
| r2 torch-pipeline | 2 | 8 | 4.0× |
| r3 torch-pipeline | 2 | 37 | 18.5× |
| r3 extract-elf | 1 | 13 | 13.0× |
| r3 gpt2-codegolf | 0 | 0 | —（本轮零检索） |

放大本身不是缺陷——子代理的职责就是 query 分解与逐页核验。**问题在额度轴**：放大不设上限，
而承载它的是按 activation 重置的预算。

### §2.2 额度轴：按 activation 重置，无 run 级累计

- 档位额度（`orz/crates/orz-loop/src/retrieval/effort.rs`）：standard ＝ 240 s／30 轮／`max_results` 5／
  8 次 SERP 导航；extended ＝ 600 s／60 轮／8／16；deep ＝ 900 s／90 轮／12／32。
- **每次派发都是全新额度**（`auto_close` 后新激活从 0 起算；代码注释与单测 `second_dispatch_after_auto_close_starts_fresh`
  同口径）。r1 torch 三个 activation 串起来，子代理占用 **935 s／span 93.5 %**。
- FUS-BUDGET 登记的是「子代理 120 轮**按 session 累计**」，实测九试次只用到 29–37 轮 ⇒
  **既有预算轴（轮数／候选数／导航数）与真实瓶颈（墙钟）错配**，没有任何一条现有预算会先于主墙钟触发。

### §2.3 effort 档位地板：web 车道恒 ≥ extended

`classify_retrieval_effort`（同文件 §4.2 计分）对外部车道 `+1`，且 `scope` 缺省（web 工具无 workspace scope 语义）
再 `+1` ⇒ **外部检索最小分 2.0，落 extended**；standard 档对 web 车道**结构上不可达**。
九试次中全部 `retrieval_close_record` 的 `effort` 均为 `extended`，与此一致。
**诚实标注**：各 activation 实际只用 206–554 s，**未触 600 s 顶**，故单独改这项对已死锁案例收益有限；
其价值在压 `max_results`／SERP 导航预算以抑制搜索放大，属边际项。

## §3 转化断点

### §3.1 主样本：r3 torch-pipeline（`RUN-CLI-6aad6aa2`）

| 时刻 | 事件 | 备注 |
|---|---|---|
| 16:45:22 | `run_started` | 首轮模型思考耗 **225 s** |
| 16:49:06 | 主 agent 唯一回合：`run_terminal_cmd`＋**2 次 `web_search`** | 两个派发同轮发出 |
| 16:49:06–16:52:32 | 第一批派发（206 s，5 搜索／8 抓取） | |
| **16:52:32** | **首批结果提交**：22 条来源（6 全文本），结论直指 `train_step_pipeline_afab` 出自 HuggingFace picotron | **答案已在首批** |
| 16:52:32 | 第二批派发**同刻启动**（同轮排队的第二个 `web_search`） | 主 agent 未获得回合 |
| 16:52:32–17:01:46 | 第二批（554 s，14 搜索／8 抓取） | 占主预算 **62 %** |
| 17:01:46 | `run_invalidated{status:"wallclock"}` | 判分物未创建 |

全 run 主 agent 只有 **1 个模型回合**——不是模型不愿落盘，是它**没有回合**。

### §3.2 九试次「首批提交之后」读数

| 轮次／题 | 子代理占用（秒／占比） | 首批 commit | 首批后主 agent 回合 | 首批后剩余预算 | 结局 |
|---|---|---|---|---|---|
| r1 torch | 547 s／54.6 % | t+321 s | 2 | 679 s | wallclock |
| r1 gpt2 | 824 s／84.7 % | t+740 s | 1 | 232 s | 早死（传输） |
| r1 extract-elf | 228 s／64.7 % | t+293 s | 0 | 58 s | `subagent_failed` |
| r2 extract-elf | 636 s／63.6 % | t+703 s | 0 | 297 s | wallclock |
| r2 gpt2 | 576 s／57.6 % | t+772 s | 0 | 228 s | wallclock |
| r2 torch | 144 s／14.4 % | 无（未闭合） | 0 | — | wallclock（环境自举） |
| r3 torch | 760 s／77.2 % | t+431 s | **0** | **554 s** | wallclock |
| r3 extract-elf（成功） | 368 s／90.4 % | t+225 s | **27** | 183 s | `completed`，reward 1.0 |
| r3 gpt2 | 0 | — | 0 | — | wallclock（零检索） |

读数：**首批提交后主 agent 回合数为 0 的占 4/6 有派发试次**；其中 r2 extract-elf、r2 gpt2、
r3 torch 三次的剩余预算（297／228／554 s）**足够落盘**。唯一成功试次是主 agent 拿到 27 个回合的那次。

### §3.3 时间预算分解（r3）

| 题 | 模型侧（含首轮思考） | 检索工具 | 其他工具 | 结局 |
|---|---|---|---|---|
| torch | 484 s（首轮 225 s） | 437 s | ≈1 s | 0 |
| extract-elf | 305 s | 96 s | 5.5 s | 1.0 |
| gpt2 | 677 s（零检索） | 0 | 230 s | 0 |

首轮模型思考在 **4/9 试次**耗 130–225 s（其余 2–4 s），属模型侧固定成本，与检索无因果。
r3 gpt2 形态（零检索、677 s 模型侧仍不落盘）是**模型习惯**，§9.2 已裁决不干预，
本文的检索侧候选对它零收益，**不得用作验收样本**。

## §4 机制面：三处已设计但空转

### §4.1 信息充分性判定（FUS-INFORMATION-SUFFICIENCY 空壳）

六次成功闭合的派发中，`information_sufficiency_assessment` 全部
`status:"indeterminate"` ＋ `reason_codes:["no_mechanical_coverage_requirement"]`，
`retrieval_close_record` 全部 `terminal_reason:"auto_close"` 且 `validated_disposition_id` 恒 `null`。
即：**没有任何机械判据说「这批够了」**，也没有把该事实回传主车道。
设计权威（索引 FUS-INFORMATION-SUFFICIENCY：「主 Agent 必须提交结构化 close 或 continue」）
与实现事实（每派发 auto_close、无 disposition 往返）之间的缺口是既有的，本文只是补上量化读数。

### §4.2 投递族（F10 根因）

- 开关默认关：`ORZ_IMMEDIATE_RESULT_DELIVERY`（`orz-loop/src/immediate_delivery.rs:33`）、
  `ORZ_WEB_SEARCH_LOCAL`（`orz-tools/src/implementations/web_search/local_segmented.rs:53`）。
- 即使打开，`host_exec/tool_run.rs` 里的 M3 写点只做 **journal 记账**
  （`result_delivered{suppressed:true, suppressed_reason:"model_read_directly"}`），
  **不产生「首批结果先返回一次」的工具语义**，故设计 §4.1 表 M3「模型无需等到完成」未落地。
⇒ F10（九 run 投递事件零出现）的根因是**默认关＋只记账**，不是事件丢失。

### §4.3 观测面：检索车道读 `.gsa/session` 被拒且码名不符

r3 三 run 共 3 处 `retrieval_role_write_denied`，命令分别为
`ls -la /app/.gsa/session/web_fetch/ ; wc -c …`（extract-elf 17:25:37）、
`ls … | tail -5; grep …`（torch 16:55:07）、含 `curl -o` 的写命令（torch 16:50:15）。
前两者是**纯读命令**，拒绝码却取自写域门（`agent_loop.rs` `ToolFilter::Retrieval::write_gate`）。
同族历史观测见索引 `OBS-GSA-READ-LANE-ASYMMETRY`（§14.61 两段门裁决，shell 旁路标注为不对称但不追打）；
本条落在**检索车道读主车道会话产物**这一新触发面上，价值是**码名与语义不符会误导归因**。

## §5 优化候选（分层；全部待裁决，本文不生效）

### §5.1 A 级（高杠杆，需 ADR 级裁决）

**A1 同轮检索派发拆轮**。一轮内只放行第一个检索派发，其余以结构化结果（稳定码＋中性事实，
如「本轮只派发 1 个检索，其余 N−1 个未派发」）交回，由模型下一轮决定。
证据：§3.1（首批答案已到，排队第二批吃掉 554 s）、§3.2（4/6 试次首批后 0 回合）。
落点：`agent_loop.rs` 工具循环＋`relay` 路由。收益：同时改善投量与转化。
风险：改变工具可观测语义（「调用未执行」），须登记为设计面变更。

**A2 run 级检索墙钟配额＋尾部保留**。例如检索族累计 ≤ 主墙钟 50–60 %；距墙钟剩余低于阈值时
拒绝新派发，返回稳定码＋已得结果指针，把尾部留给落盘。
证据：§2.2（按 activation 重置、run 级无累计）、§3.2（三次剩余 228–554 s 足够落盘却全部烧在检索）。
落点：派发前判定，可挂既有 `ORZ_RETRIEVAL_*` env 家族。

### §5.2 B 级（机制补全）

**B1 充分性判定落地**：为外部检索定义最小机械覆盖要求（如 ≥1 条 `full_text_observed`＋`relevance:"direct"`），
使 assessment 能给出 `sufficient`，并把该中性事实回传主车道，减少无谓再派发。证据：§4.1。

**B2 投递族可测化**：评测口径显式打开只读开关，或明确「首结果 ≤10 s」判据在 `local_segmented=off` 下不适用。
证据：§4.2／F10。

**B3 子代理内重复性早收口**：扩展既有 `ORZ_RETRIEVAL_EARLY_CLOSE_FAILURES`（现只数失败），
增加收益型判定（连续 N 次抓取无新增 `source_id`／同域重复）。

### §5.3 C 级（effort 与观测面）

**C1 effort 地板纠偏**：外部车道不参与 scope 计分，或按 run 内派发次数递减档位。证据：§2.3（边际收益）。
**C2 回主车道补角色事件**（`header_change{reason:"resume"}` 或等价物）。证据：§1.2 归属勘误。
**C3 拒绝码语义修正**：检索车道读会话产物的拒绝码不应复用写域码。证据：§4.3。

## §6 验收读数建议（不依赖投递族事件）

1. 主 agent 回合数 ≥2（首批返回后必须存在主回合）；
2. 首批 commit → 首次落盘动作的间隔；
3. 交付物落盘时距墙钟剩余 ≥60 s；
4. 检索族墙钟占 run 比例上限；
5. 首批之后新派发数（目标：0，除非模型显式续派）。

上述五项均可从现有 journal 机械重算，不受 `ORZ_IMMEDIATE_RESULT_DELIVERY` 开关影响。

## §7 边界与不受益面

- **不采**「检索失败率」作验收轴：r3 非浏览器 47 次调用已零失败（§10.4）。
- 任务镜像／verifier／数据集 pin **不动**（沿用 2026-08-29、§10.8 用户裁决）；
  不在 orz 做容器内环境提前补强。
- r3 gpt2 形态（零检索亦不交付）属**模型习惯**（§9.2 裁决不干预）：检索侧候选对其零收益，
  不得作为本线优化的验收样本。
- 与既有裁决冲突时以裁决为准：本文只提供读数与候选，不含生效决定。

## §8 证据物

- journal：`D:\tb-eval\gsa-volumes\official-verify-timeout3[-r2|-r3]\<session8>\runs\<run>\events.jsonl`
  （r3 三 run：torch `6aad6aa2`／session `4f65c2ea`、gpt2 `6aad6fad`／`445f1ff2`、extract-elf `6aad7388`／`c1b272d4`）
- 作业产物与 trajectory：`D:\tb-eval\jobs-official\<job>\<trial>\agent\trajectory.json`
- 代码落点：`orz/crates/orz-loop/src/retrieval/effort.rs`（档位与地板）、
  `orz/crates/orz-loop/src/immediate_delivery.rs`（投递开关与写点）、
  `orz/crates/orz-loop/src/host_exec/tool_run.rs`（M3 写点）、
  `orz/crates/orz-loop/src/agent_loop.rs`（写域门、同轮工具循环）
- 同族前序：本目录 [`校验轮记录 §10`](TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md)

## §9 待裁决

- §5.1 A1／A2 是否立项（当前**不动计数**）；
- §5.2 B1–B3 是否随 A 级同批；
- §5.3 C1–C3 是否仅登记；
- 用户 2026-09-19 提出的设计变体（「子代理每取得 5 条可用数据即回报一次结果总结给主代理，
  信息充分性判定上移主代理」）**待评判**——评判结论另批落档，本文不含生效裁决。
