# 0ar S3 同三题真机复验（k=1，2026-09-19 起跑／2026-09-20 收工）

> 用户令（2026-09-19）：「这样的话那就不考虑接 GLM 的 api 了，请直接进 S3 同 3 题的复验吧」
> ——**GLM 接入线就此搁置（未立项、未改码）**，直接放行 **0ar S3**。
> 口径与 r1／r2／r3 逐项一致，**唯一变量＝载体**（0.6.2 `D14D6D9C…` → **0.6.3 `AC3FB7BA…`**）；
> 本批为**只读取证＋跑批**：零代码改动、零子仓改动、计数不变（未闭合维持 **36**）。

## §0 结论速览

**机制面（设计 §8 判据）**：判据 **1／2／3 达成**、判据 **6 部分达成**（合并在场＋逐 query 可核，
但出现逐 query 欠归因形态）；判据 **4／5／7 未触发**（本轮语料未出现同轮 >3 检索、无软硬计数不一致、
模型未使用提前交付）——三者的机制面已由 S2 测试锁定，真机未取到样本。

**转化面（本批第一次出现的形态变化）**：

| 题 | r1–r3 形态（0.6.2） | **S3 形态（0.6.3）** |
|---|---|---|
| torch-pipeline-parallelism | 三试次 AgentTimeout、`/app` 空、判分物从未产出 | **`run_finished{completed}`**（唯一一次非超时收工）＋**判分物产出**＋verifier **2/4 通过**（另 2 项数值不匹配 max diff 0.0137） |
| gpt2-codegolf | r3 零检索亦不交付 | 首次出现检索（2 批）＋首批 commit 后主车道 29 回合，仍未写出 `/app/gpt2.c` |
| extract-elf | r3 靠公网泄漏页复刻得 1.0 | 3 批检索全部达标回送，**未写 `/app/extract.js`**（相对 r3 读数回归，见 §7-N5） |

**结构面判定**：D1（阈值回送）／D3（合并优先）在真机 **7/7 已闭合激活全部生效**；D2（到点交回）
本轮**未被触发**（最长批 232.1 s < 300 s 门）；阈值回送把「首批 commit 后主回合＝0」的旧形态
（r1–r3 4/6 试次）翻转为 **3/3 试次均有主回合**（首批 after-commit 主回合 54／29／17，口径见 §4）。

## §1 轮次身份与执行面

- 作业 `official-verify-timeout3-s3`，起跑 **2026-09-19 23:57:31** / 收工 **2026-09-20 00:52:37**
  （**55m06s**，exit=0）；Harbor 公开上传（`--upload --public`）。
- 载体：**0.6.3**（`D:\tb-eval\orz-linux\orz` sha256 `ac3fb7ba…`，与 063 重建件逐位一致）；
  适配器 `tb_agents/orz.py` sha256 `fdd161d4…`（含 **F1 跨 run 隔离修复**）；
  数据集 pin `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`、`deepseek-v4-flash`、
  `-k 1 -n 1 --ak max_wallclock=900` 串行、无时间倍率、`n_retries=0`。
- 起跑前预检：三题镜像本地在位（`--no-pre-pull`）；容器内金丝雀 **api.deepseek.com 401 ✓／
  pypi 200 ✓**（直连形态，与 r3 同）；代际身份行照旧报载体／适配器 DRIFT 并显式放行。
- 直连形态坐实：三 run 的 `tool_availability_check.retrieval_family` 均为
  `web_search local_segmented=off chain_detail="bing_cn; proxy=off"`；**transport_retry 三 run 全零**。
- **F1 隔离修复真机首验**：跑第二位试次时，首位试次整卷被移入
  `gsa-volumes\.quarantine\official-verify-timeout3-s3\`（末位试次留在卷根）——与 09-19 冒烟形态一致。

## §2 总结果

| 题 | run / session | 结局 | reward | agent 墙钟 |  工具轮 |
|---|---|---|---|---:|---:|
| torch-pipeline-parallelism | `RUN-CLI-6aaeb160` / `582d0289` | **`run_finished{completed}`**（无 exception） | 0（**2/4 测试通过**） | 815.7 s | 63 |
| gpt2-codegolf | `RUN-CLI-6aaeb62b` / `7f781ed9` | `run_invalidated{wallclock}`＋`AgentTimeoutError` | 0（`/app/gpt2.c` 不存在） | 900.0 s | 41 |
| extract-elf | `RUN-CLI-6aaeba24` / `51180b0a` | `run_invalidated{wallclock}`＋`AgentTimeoutError` | 0（`/app/extract.js` 不存在） | 900.0 s | 26 |

verifier 侧：三题依赖安装均在预算内完成（torch 4 项测试 40.31 s 内跑完、gpt2 0.05 s、extract 0.42 s），
**reward 轴本轮全部可读**（r2 的「无 reward」形态未复现）。

## §3 判据 7 条对照（设计 §8）

| # | 判据 | S3 真机读数 | 判定 |
|---|---|---|---|
| 1 | close reason ∈ 新三值；`dispatch_wallclock_bound` 必带计数＋缺口 | **7/7 已闭合激活全部 `evidence_threshold_met`**；`dispatch_wallclock_bound`／`subagent_early_delivery` 未出现（无到点批、无提前交付） | **达成**（新值真机在场；另两值未取到样本） |
| 2 | 单批墙钟 ≤ `batch_wallclock`＋收尾回合 | 七批墙钟 **157.8／78.2／232.1／57.4／97.6／155.8／155.8 s**（`effort=extended`，门 300 s），最大 232.1 s（占门 77 %） | **达成**（D2 到点臂未触发） |
| 3 | 首批 commit 后主 agent 回合 ≥1 | **54／29／17 回合**（三题全达标；同口径 r3＝0／—／26（gpt2 无 commit）→ 旧形态 4/6 试次为 0） | **达成** |
| 4 | 溢出调用 `cause=retrieval_dispatch_deferred_one_per_round` 且无 `ToolStarted` | 本轮同轮检索调用 **最多 2 个**（上限 3 未越线）⇒ 溢出臂未被触发（`overflow_deferred_calls` 空、无该 cause） | **未触发**（机制面 S2 集成测试锁定） |
| 5 | 软规则与机械护栏计数不一致落 anomaly | 全部 assessment `reason_codes` 仅 `no_mechanical_coverage_requirement`；**零 anomaly 码**（`early_delivery_*` 三码未出现） | **未触发** |
| 6 | 合并激活 `query_summary` 条目数＝合并 query 数且逐 query 可核 | 多 query 激活 4 例（各 2 条）＋单 query 激活 3 例：条目数与实际派发数逐例一致、`usable_source_count` 逐条在场；**但 2 例逐 query 合计小于批级**（见 §7-N1） | **部分达成**（条目数与字段达成；归因准确性有缺口） |
| 7 | 提前交付必带指针；可见倒数行与 `source_counts` 同口径 | 提前交付未使用 ⇒ 指针硬校验无样本；**倒数行不在任何落盘面**（journal／trajectory／会话侧车全扫零命中，见 §7-N3） | **未触发＋不可核** |

## §4 验收读数五项（分析件 §6）与 r3 对照

| 读数 | torch S3（r3） | gpt2 S3（r3） | extract-elf S3（r3） |
|---|---|---|---|
| 主 agent 回合数（窗口法／括号内为 r3） | **65**（1） | **41**（28，零检索） | **27**（32） |
| 首批 commit→首次落盘 | 305.6 s（无落盘） | 119.8 s（无落盘） | **无落盘**（r3：+113.8 s） |
| 落盘时距墙钟剩余 | **400.5 s**（—） | **290.3 s**（—） | —（无落盘） |
| 检索族墙钟占比 | **19.7 %**（44.4 %） | **14.4 %**（0 %） | **21.4 %**（23.6 %） |
| 首批之后新派发数 | 2（=1 次显式续派，另 1 为同轮合并回执） | 1（1） | 3（2）＋1 未闭合尾批 |

「首批之后新派发」本轮**全部落在主代理的新回合**（逐派发回溯到 `model_output`：torch 16:11:01／
gpt2 16:28:17／extract-elf 16:44:03、16:48:07、16:51:26），**不再出现 r3「同轮排队第二个检索、
主代理整轮阻塞」形态**——这正是判据 3 翻正的机制来源。

> 口径注：判据 3 的「首批 commit 后主 agent 回合数」取**主车道工具轮审查事件**
> （`mechanical_audit_update{kind:budget}`，子代理回合不递增该计数；无工具调用的收尾回合不落该事件 ⇒ 取值是下界），
> 三题读数 **54／29／17**（同口径 r3：torch 0／extract-elf 26／gpt2 无 commit）；
> 表中「主 agent 回合数」为**激活窗口法**的全轮计数（含无工具回合），两者互证。

## §5 逐题激活明细（journal 一手）

| 题 | 激活 | close reason | 批墙钟 | effort | query 数 | 逐 query 可用 | 批级可用（宽口径） | `source_counts{total,full,partial,meta}` |
|---|---|---|---:|---|---|---|---|---|
| torch | 00 | `evidence_threshold_met` | 157.8 s | extended | 2 | 1／0 | 5 | 6／0／5／1 |
| torch | 01 | `evidence_threshold_met` | 78.2 s | extended | 2 | 0／0 | 6 | 11／4／2／5 |
| gpt2 | 00 | `evidence_threshold_met` | 232.1 s | extended | 2 | 1／1 | 5 | 9／0／5／4 |
| gpt2 | 01 | `evidence_threshold_met` | 57.4 s | extended | 1 | 5 | 5 | 10／4／1／5 |
| extract | 00 | `evidence_threshold_met` | 97.6 s | extended | 1 | 5 | 5 | 7／2／3／2 |
| extract | 01 | `evidence_threshold_met` | 155.8 s | extended | 1 | 5 | 5 | 13／0／5／8 |
| extract | 02 | `evidence_threshold_met` | 155.8 s | extended | 2 | 0／0 | 6 | 15／5／1／9 |

- **宽口径去重复算与 `usable_source_count` 逐例一致**（脚本按 `content_sha256` 去重重算＝5／6／5／5／5／5／6），
  即阈值／护栏／倒数共用一把尺在真机成立。
- **每批都在 5 或 6 条收尾**——D1 阈值（5）与批级合计口径在真机全程按预期工作；
  护栏 10 未触发（无一批逼近）。
- extract-elf 另有**第 4 批未闭合**：16:51:26 派发，16:52:01 被 run 墙钟截断（journal 有 `tool_running` 1 条，
  无 close record）——见 §7-N2。

## §6 与 r3 基线横比（机制面）

| 维度 | r3（0.6.2） | S3（0.6.3） | 判定 |
|---|---|---|---|
| 首批 commit 后主回合＝0 的试次 | 2/3（torch 0、gpt2 无 commit） | **0/3** | 结构翻转 |
| 同轮多检索形态 | 串行排队（torch 第二批 554 s 同刻启动） | **合并单激活多 query**（5 例） | D3 生效 |
| 单批最长墙钟 | 636 s（r2 extract-elf 单批）／r3 torch 第二批 554 s | **232.1 s** | D2 门以下 |
| close reason | 7/7 `auto_close` | **7/7 `evidence_threshold_met`** | D1 生效 |
| 检索族墙钟占比 | torch 44.4 %／gpt2 0 %／extract 23.6 % | 19.7 %／14.4 %／21.4 % | 下降或持平 |
| 交付物 | 仅 extract-elf 1 件（泄漏路径） | torch 1 件（真实解题路径） | 见 §7-N5 |
| 传输／浏览器／资源门 | transport 0；browser 恒死；deny 14 次 | transport 0；browser 8 次全败；deny **29 次**（14／11／4） | 装置面同族，deny 量级上升 |

## §7 观察与摩擦（登记级；不立项、不动计数）

| # | 观察 | 证据 | 建议去向 |
|---|---|---|---|
| **N1** | **逐 query 可用计数欠归因**：合并批的逐 query 合计可小于批级可用（torch 00 批级 5／逐 1＋0；extract 02 批级 6／逐 0＋0） | §5 表；S2 审查已登记「派生证据多 query 下不归因」为边界 | 若后续要「主代理据逐 query 覆盖度续派」，须先解决归因（留裁决） |
| **N2** | **run 尾部派发**：extract-elf 在距 run 墙钟 35 s 时派发第 4 批，被 run 墙钟直接截断（无 close、`tool_running` 1） | `RUN-CLI-6aaeba24` 16:51:26 派发／16:52:01 `run_invalidated` | 分析件 §5.1 **A2（run 级检索配额＋尾部保留）** 候选再浮现；本批不立项 |
| **N3** | **倒数行／β 注入块无落盘面**：`agent_loop.rs` 把倒数行追加进模型面 `messages`，官方跑批产物（journal／trajectory／会话侧车／编排产物）**全库扫描零命中** ⇒ 判据 7 后段真机不可核 | 三 run 产物全扫 `本批可用证据` 0 命中；机制面由 `batch_close` 单源 helper＋6 单测锁定 | 同 F10 家族（观测面缺口）；是否补落盘面留裁决 |
| **N4** | 资源门 deny 常态化（量级上升） | 三 run 14／11／4＝**29 次**（r3 单轮 14 次、r1 3／r2 12），tier soft | 沿 F4 维持观察 |
| **N5** | **extract-elf 相对 r3 读数回归**：r3 得 1.0 系「公网页含本题 `test_outputs.py`＋内嵌 REF 实现」的泄漏路径（r3 F2 已登记）；本轮 3 批检索均达标回送但未写判分物 | r3 §10.3 泄漏记录 vs 本轮 verifier `test_extract_js_exists` 失败 | **不得归因于 0ar**；官方 1.0 的自主解题成分本就须打折（F2 维持） |
| **N6** | 浏览器车道恒死未变 | 三 run 8 次 `browser_control` 全败（`browser_not_found`→`capability_unreachable`，墙钟 0.0 s） | 沿 F3 用户裁决维持现状 |

## §8 结论与待裁决

1. **0ar S2 三件在真机全部生效**（D1 阈值回送 7/7、D3 合并 4 例多 query 激活、D2 门内 7/7），
   设计 §8 判据 **1／2／3 达成、6 部分达成**；4／5／7 因语料未触发而未取到真机样本（机制面已锁定）。
2. **主线收益成立**：「首批 commit 后主代理无回合」的结构性缺陷（r1–r3 4/6 试次）在 S3 **零复现**，
   且检索墙钟占比降至 14–21 %；torch-pipeline 首次产出判分物并过 2/4 测试。
3. **交付面仍未翻转**（三题 reward 均 0）：torch 差在数值正确性、gpt2／extract 差在判分物未落盘。
   两者均属模型侧习惯面（0.6.2 期 §9.2 裁决不干预的口径未变）。
4. **待裁决（建议主会话不自行决定）**：
   - ① 0ar 是否**闭合**（S3 已按设计收取读数；判据 4/5/7 未触发部分是否接受「机制面锁定」为充分）；
   - ② 观察 **N2（run 尾部派发）** 是否转 A2 立项；
   - ③ 观察 **N3（倒数行落盘面）** 是否补一个可核面（或按 F10 同处置：登记为官方口径下不可测）。

## §9 证据物

- 轮次日志：`D:\tb-eval\jobs-official\official-verify-timeout3-s3-round.log`（plan 行含载体／适配器双哈希）
- 作业产物：`D:\tb-eval\jobs-official\official-verify-timeout3-s3\<trial>\`（result.json／verifier\reward.txt／verifier\test-stdout.txt）
- 三 run journal：`D:\tb-eval\gsa-volumes\.quarantine\official-verify-timeout3-s3\{582d0289,7f781ed9}\runs\`＋
  `D:\tb-eval\gsa-volumes\official-verify-timeout3-s3\51180b0a-…\runs\RUN-CLI-6aaeba24\events.jsonl`（末位试次留在卷根）
- 读数核算脚本（仓外件）：`D:\tb-eval\_s3_analysis.py`（journal 机械重算：激活窗口／宽口径去重／车道归属／判据面）
- 基线：r1–r3 记录见 [`TB21_V41_TIMEOUT3_VERIFY_2026-09-18`](TB21_V41_TIMEOUT3_VERIFY_2026-09-18.md)；
  上游分析件见 [`TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19`](TB21_RETRIEVAL_VOLUME_CONVERSION_DEEP_ANALYSIS_2026-09-19.md)
