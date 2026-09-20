# 0ay S4 同三题真机复验（k=1，2026-09-20；载体 0.6.4）

> 用户令（2026-09-20）：「请进行重建和三题重跑吧」——先做双平台载体重建（见
> [`064 重建审计`](064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md)），随后以新载体发起
> **0ay S4 同三题真机复验**（承接 0ax S4）。
> 口径与 r1／r2／r3／S3 逐项一致，**唯一变量＝载体**（0.6.3 `AC3FB7BA…` → **0.6.4
> `1160826E…`**）；本批为**只读取证＋跑批**：零代码改动、零子仓改动、计数不变（未闭合维持 **41**）。

## §0 结论速览

**0ay S4 判据（「无 URL 占比／可引用来源数可机械读并入审计」）——达成。**
三道题各一批、共 **3 批**，每批都能从 journal 的 `source_ledger` 机械读出**收窄可用**
与**合成（无 URL）**两类计数、并由 payload 的声明面对拍：**3/3 批逐值一致**
（`query_summary[0].usable_source_count` ＝ 复算 6／6／6；`synthetic_answer_count` ＝ 复算 3／6／3）。
轮次合计：**收窄可用 18 条、合成（无 URL）12 条、可引用来源（`citation_url_count` 合计）0 条
⇒ 无 URL 占比 12/12 ＝ 100 %**。

**机制面**：3/3 激活 `evidence_threshold_met` 收尾（批墙钟 113.4／113.4／298.7 s，均在 300 s 门内，
最大值占门 99.6 %）；**0aw 口径真机首验**——`host_resource_denied` **0**／`resource_exhausted` **0**／
`reclaim_performed` **0**（0.6.3 轮次为 25 次 deny）；`transport_retry` 三 run 全 **0**（直连形态）；
浏览器死车道仍为快速失败（`browser_not_found` 2／4／5 ＋ `capability_unreachable` 1／1／2，零预算烧蚀）。

**转化面（相对 S3 首次出现两题得分）**：torch-pipeline-parallelism **4/4 通过（reward 1.0）**、
extract-elf **2/2 通过（reward 1.0，公网泄漏路径，见 §6）**、gpt2-codegolf 仍 0（未落 `/app/gpt2.c`）。
**本批不据此归因**：唯一变量是载体，但两题得分路径均含公网参照（§6 已注明）。

**新摩擦（登记级，不立项）**：gpt2 run **无 terminal 事件**（硬杀截断，全卷校验 1 条
`expected exactly one terminal event, found 0`）；torch run 的 `run_invalidated{wallclock}` 落在
**1000.3 s**（预算 900 s 之外 100 s）。二者同属墙钟收尾家族（N2／0au 面），本批只登记。

## §1 轮次身份与执行面

- 作业 `official-verify-timeout3-s4`，起跑 **2026-09-20 18:07:26** / 收工 **18:53:10**
  （**45m44s**，exit=0）；Harbor 公开上传（`--upload --public`）。
- 载体：**0.6.4**（`D:\tb-eval\orz-linux\orz` sha256 `1160826e…`，与 064 重建件逐位一致）；
  适配器 `tb_agents/orz.py` sha256 `fdd161d4…`（与 S3 同）；数据集 pin
  `terminal-bench/terminal-bench-2-1@sha256:7d7bdc1c…`、`deepseek-v4-flash`、
  `-k 1 -n 1 --ak max_wallclock=900` 串行、无时间倍率、`n_retries=0`。
- 起跑前：三题镜像本地在位（`--no-pre-pull`）；容器内金丝雀 **api.deepseek.com 401 ✓／pypi 200 ✓**
  （直连形态，与 r3／S3 同）；代际身份行照旧报 carrier／adapter DRIFT 并显式放行
  （`--allow-identity-drift`；适配器锁定值仍是 0.5.0 代的 `2737cfad…`）。
- 直连形态坐实：三 run 的 `tool_availability_check.retrieval_family` 均为
  `web_search local_segmented=off chain_detail="bing_cn; proxy=off"`——**本地分段车道未启用**
  （0ax S2 待放行），本轮的「带 URL 占比」因此结构性为 0（见 §3 注）。
- **F1 隔离修复续验**：跑第二／第三试次时，先跑试次整卷移入
  `gsa-volumes\.quarantine\official-verify-timeout3-s4\`（末位试次留在卷根）——与 S3 同形。

## §2 总结果

| 题 | run / session | 结局 | reward | agent 墙钟 | 工具轮（主／子） |
|---|---|---|---|---:|---|
| torch-pipeline-parallelism | `RUN-CLI-6aafb0d0` / `be1c7216` | `run_invalidated{wallclock}` | **1.0（verifier 4/4 通过）** | 1000.3 s | 44／5 |
| gpt2-codegolf | `RUN-CLI-6aafb5cf` / `1a29c97f` | **无 terminal 事件**（硬杀截断） | 0（`/app/gpt2.c` 不存在） | 942.0 s（末事件 941 s） | 9／6 |
| extract-elf | `RUN-CLI-6aafb9cb` / `2da93c6d` | **`run_finished{completed}`** | **1.0（verifier 2/2 通过）** | 299.8 s | 28／6 |

verifier 侧：torch 4 项 46.27 s 内全过、extract 2 项 0.26 s 全过、gpt2 0 项（判分物缺席）。

## §3 0ay S4 判据：无 URL 占比／可引用来源数（逐批机械读）

复算口径＝仓内冻结镜像 `recompute_retrieval_batch_counts`（与生产者 `batch_close::is_synthetic_answer`
同尺、类内独立去重），驱动件 `D:\tb-eval\_0ay_s4_readings.py`（仓外证据工具）：

| 题（批） | 收窄可用 | 合成（无 URL） | 有引用池 | `citation_url_count` 合计 | 声明面（批级／未归因／合成） | schema | 判官 |
|---|---:|---:|---:|---:|---|---:|---:|
| torch-pipeline-parallelism（`…-00-r0-b4bd6e4a`） | 6 | 3 | 0 | **0** | 6／—／3（逐 query 6） | 0 | 0 |
| gpt2-codegolf（`…-00-r0-e7b43cc0`） | 6 | 6 | 0 | **0** | 6／—／6（逐 query 6） | 0 | 0 |
| extract-elf（`…-00-r0-64f4f582`） | 6 | 3 | 0 | **0** | 6／—／3（逐 query 6） | 0 | 0 |
| **合计** | **18** | **12** | **0** | **0** | 3/3 逐值一致 | **0** | **0** |

**判读**：

1. **可机械读达成**：三道题都只发 1 个 query（单 query 激活），`query_summary[0].usable_source_count`
   即批级收窄可用数（`per_query_usable_counts` 的单 query 形态），与 ledger-only 复算**逐值相等**；
   `synthetic_answer_count` 同样逐批相等。**无 URL 占比＝12/12＝100 %、可引用来源数＝0**。
2. **本批是纯合成车道**：12 条 `web_search_result` 条目**全部无引用池**（`citation_url_count` 三卷
   出现 0 次）⇒ 判官 0az ① 的「声明面对拍」**在本轮按构造不触发**（生成代际门＝「有任一条目带
   `citation_url_count`」；纯合成形态与 pre-0ay 生产者不可机械区分，故 F-1 裁决下门保持关闭）。
   本轮读数为 **ledger-only 复算**，声明面数值虽一致但**未获判官交叉校验**——这是 0ay／0az 的
   已知边界，登记在案（不是回归）。
3. **0ax S4 那组建议判据（带 URL ≥90 %、`synthetic_answer_count` 恒 0）不适用于本轮**：
   它们以「启用 ORZ_WEB_SEARCH_LOCAL 本地分段车道」为前提，本轮**未启用**（0ax S2 待放行），
   故 provider 合成车道下 100 % 无 URL 属预期形态。
4. **披露面首次在真机落盘**：gpt2 run 的**子代理对自己收尾块的复述**出现在
   `model_output.text`（事件 103）：「本批可用证据 6 条（阈值 5 已达标并收尾）；原生车道
   web_search×4 + web_fetch×6；浏览器车道不可用（browser_not_found）；grep.app 被安全拦截；
   **2 条无 URL 合成回答（含一份"张量顺序"描述）已按规则排除、不作为结论引用**」——
   0ax 的「无 URL 不计入可用额度」与可见倒数行**在模型面确实生效且可核**
   （S3 的「倒数行无落盘面」观察由此获得一条正面样本；注入块本身仍未落盘，
   后段判据仍属 F10 家族）。

## §4 结构面与机制面读数

| 题 | 激活 | 收尾原因 | 批墙钟 | effort | query | 检索占比 | web_search／web_fetch／浏览器 |
|---|---|---|---|---|---:|---|---|
| torch | `retrieval-external_retrieval-RUN-CLI--00` | `evidence_threshold_met` | 113.4 s | extended | 1 | 6.2 % | 4／6／1 |
| gpt2 | 同上 | `evidence_threshold_met` | 298.7 s | extended | 1 | 12.2 % | 7／6／1 |
| extract | 同上 | `evidence_threshold_met` | 113.4 s | extended | 1 | 21.8 % | 4／6／1+1 |

- 判据面（0ar S2 家族）三题全数生效：**3/3 激活 `evidence_threshold_met`**、
  `dispatch_wallclock_bound`／`subagent_early_delivery` 未出现（无到点批、无提前交付）、
  首批 commit 后主车道回合 36／—／21（gpt2 无可对比口径）、同轮检索调用数 1（未越合并上限）。
- **0aw 面真机读数（本批首次在 0.6.4 上取）**：`host_resource_denied`＝0、`host_resource_snapshot`
  与卷软提示按观测形态在场、`resource_exhausted`＝0、`reclaim_performed`＝0、Job 上限只作上限；
  对照 0.6.3 轮次（S3）的 25 次 deny，本次**零拒绝**——与 0aw 判据「heavy 类拒绝数 → 0」一致。
- **0au 面**：`retrieval_dispatch_wallclock_reserved`＝0（无尾部派发被余量闸拦下）；
  但 torch 的 wallclock 收尾落在 1000.3 s（预算外 100 s）与 gpt2 无 terminal，见 §7。
- 检索车道健康度：`transport_retry`＝0（三 run）、web_search 单次最坏 62.4 s（extract，
  含 4 次调用）、web_fetch 总计 6／6／6 次均完成。

## §5 判官与全卷校验

| 对象 | 读数 | 判读 |
|---|---|---|
| 3 批 payload × 契约 schema（`retrieval-result-event-payload-v0.2`） | **0 错**（3/3） | 0ay S1 契约面在真机产出上成立 |
| 3 批 `result_consistency` 判官（含 0az ① 声明面门） | **0 错**（3/3） | 与 §3-2 的代际门边界一致（门开时无样本） |
| 三 run 全卷校验 `validate_journal_file`（仓内 Python 冻结镜像） | extract **0**／torch **0**／gpt2 **1**（`expected exactly one terminal event, found 0`） | gpt2 为硬杀截断，非 0ay／0az 面缺陷 |
| 0ay S2 恒等式（`citation_url_count == len(candidate_urls) + 该源 prefilter 移除数`） | 本轮无带池条目 ⇒ **无可检样本** | 机制面由 S2 单测＋S3 语料锁定，真机待带池样本 |
| 生成器↔fixtures／仓内读数 | 本批零代码改动 | 沿用 0ay／0az 批读数（runtime 378/0、orz-loop 819/0/3、orz-assurance 246/0） |

## §6 转化面（同载体唯一变量的对照读数）

| 题 | S3（0.6.3） | **S4（0.6.4）** | 判读 |
|---|---|---|---|
| torch-pipeline-parallelism | `run_finished{completed}`、verifier **2/4**、reward 0 | `run_invalidated{wallclock}`、verifier **4/4**、**reward 1.0** | 首次满分；检索占比 19.7 %→6.2 % |
| extract-elf | 3 批达标回送但未落判分物、reward 0 | `run_finished{completed}`、**2/2**、**reward 1.0** | **公网泄漏路径**：journal 内含 `raw.githubusercontent.com/harbor-framework/terminal-bench/main/archive/extract-elf/{tests/test_outputs.py, solution/solve.sh}`（与 r3 的 1.0 同族）⇒ **自主成分打折，不得作为框架能力读数** |
| gpt2-codegolf | 首次出现检索（2 批）但仍未落 `/app/gpt2.c` | 1 批达标回送、仍在写 `merges.txt` 对照（HF gpt2 仓库），**未落判分物** | 形态与 S3 同（检索→继续搜），reward 0 |

**归因纪律**：torch 的 4/4 亦经公网参考（`huggingface/picotron_tutorial` 的 `pipeline_parallel.py`
与 `NVIDIA/Megatron-LM` 源码）——r1–S3 同样可达这些资源而未能交付，故「互联网可达性」不是唯一变量；
但本批**不做因果归因**（载体＋模型行为＋装置三方混杂），只登记形态变化。

**裁判口径裁决（2026-09-20，用户令）**：「extract-elf 依旧是泄露路径的话，那就无论答没答出来我们都自己判 0 即可，
后续跑正式测试了的话，成绩公布时这个问题也显式标注」。⇒ 处置：

1. **自判一律 0**：凡 extract-elf 试次的 journal 出现评测泄漏面（`harbor-framework/terminal-bench`
   归档页的 `tests/test_outputs.py`／`solution/solve.sh` 等可直取参照），**无论 verifier 给分与否**，
   本项目自己的评价面按 **reward 0** 记账（本轮 verifier 1.0 据此作废，不改 platform 读数）。
2. **成绩公布显式标注**：后续正式测试（官方跑批）出分时，须在成绩面**显式标注**该题的评测泄漏问题
   （公网可检索到题目归档与判分物），避免把泄漏路径得分当作能力读数。
3. **不改装置**：官方口径（`task.toml`／镜像／verifier／数据集 pin）不动，**不封网、不改题**；
   本裁决只作用于「本项目自判与公布口径」。
4. 本轮据此复算：**S4 自判面＝torch 1.0／extract 0（verifier 1.0 因泄漏路径作废）／gpt2 0**
   ⇒ 三题自判 reward 合计 **1.0**（platform reward 合计 2.0，两者差异全部来自本裁决）。

## §7 摩擦与观察（全部登记级，不立项）

1. **gpt2 无 terminal 事件（新观察）**：journal 末事件 `tool_completed` ＋ 2 条
   `mechanical_audit_update`（10:46:21），试次于 10:46:36 收工 ⇒ 硬杀截断、无 `run_invalidated`。
   与 S3 的 gpt2（`run_invalidated{wallclock}` 正常收尾）不同形；同族观察 N2（run 尾部派发）＋
   0au 面，**本次不追**（未定位到可归因的代码面）。
2. **torch 墙钟收尾超出预算**：`run_invalidated{wallclock}` 落在 run 起算 1000.3 s（预算 900 s）；
   试次内 4/4 通过（verifier 在其后 46 s 内跑完）⇒ 属收尾时序观察，不影响本轮判分。
3. **纯合成车道下 0ay 判据后半段不可核**：带池样本缺席 ⇒ 声明面对拍与 S2 恒等式在本轮
   无可检样本（§3-2、§5）；要取到样本须启用本地分段车道（0ax S2）或等 provider 车道返回引用。
4. **浏览器死车道**：三 run 共 12 次 `browser_not_found`／`capability_unreachable`，均为快速失败、
   零墙钟烧蚀（N6 家族，官方口径不启用 `eval_browser` ⇒ 维持现状）。
5. **评测泄漏面**（extract-elf）：TB 归档页（含 `tests/test_outputs.py` 与 `solution/solve.sh`）
   在公网可直取 ⇒ F2 家族再复现，本批只作读数打折标注。
   **已裁决（2026-09-20）**：本项目自判面一律记 **0**（不论 verifier 给分），正式公布成绩时显式标注该泄漏问题；
   装置侧维持官方口径不动（见 §6 裁判口径裁决）。

## §8 边界与留待

- **未动**：批阈值 5／机械护栏 10、FP-2、`ORZ_WEB_SEARCH_LOCAL`（仍 off）、官方口径
  （`task.toml`／镜像／verifier／数据集 pin）、0am 影批工作树、未闭合计数（**维持 41**）。
- **0ay 闭合与否待用户裁决**：S4 读数已并入本档（判据达成、边界显式登记）；
  闭合则三方计数 41 → 40，索引条目转 `implemented`。
- **未提交、未推送**：本批父仓改动（pin → `a47e9185`、`orz_source_manifest.sha256`、
  本档与 064 档、索引／BACKLOG／TODO 登记面）留在工作树。

## §9 证据入口

- 轮次日志：`D:\tb-eval\jobs-official\official-verify-timeout3-s4-round.log`（plan 行含载体／适配器双哈希）
- 作业产物：`D:\tb-eval\jobs-official\official-verify-timeout3-s4\<trial>\`（`result.json`／`verifier\reward.txt`／`verifier\test-stdout.txt`）
- 三 run journal：`…\gsa-volumes\official-verify-timeout3-s4\2da93c6d-…\runs\RUN-CLI-6aafb9cb\events.jsonl`（末位）＋
  `…\gsa-volumes\.quarantine\official-verify-timeout3-s4\{be1c7216-…,1a29c97f-…}\runs\{RUN-CLI-6aafb0d0,RUN-CLI-6aafb5cf}\events.jsonl`
- 批归档：各 run `retrieval-results\retrieval-external_retrieval-RUN-CLI--00-r0-{b4bd6e4a,e7b43cc0,64f4f582}.json`
- 复现命令：`python -X utf8 D:\tb-eval\_0ay_s4_readings.py`（读数）／
  `python -X utf8 D:\tb-eval\_s3_analysis.py <vol> <.quarantine\vol>`（结构面）／
  `assurance.run_event_journal_validation.validate_journal_file`（全卷校验）

入口：[`064 载体重建`](064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md) ／
[`0ay 实施报告`](0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md) ／
[`0ar S3 复验`](0AR_S3_THREE_TASK_VERIFY_2026-09-20.md) ／
[`BACKLOG 0ay`](../BACKLOG_AND_PRIORITIES.md)。
关键词：0ay S4、三题真机、无 URL 占比、可引用来源数、收窄可用、合成单列、纯合成车道、
声明面对拍边界、0aw 零拒绝、F1 隔离、公网泄漏路径、硬杀截断、0.6.4。
