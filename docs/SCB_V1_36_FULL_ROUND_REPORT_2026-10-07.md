# SCBench V1 官方轮全轮报告（36 题／196 checkpoint，k=1）——成绩与摩擦综合（2026-10-07）

> 类型：轮次成绩与情况报告（对外披露底稿／对内收尾件；207 批交付物）。
> 范围：2026-10-05 – 2026-10-06 六日跑批（D1–D6，36 题全部 exit 0、零装置性失败零重跑）
> ＋2026-10-07 S3 收口（Harbor 全目转换＋发布＋0bz S4 离线对账）。
> 权威链：勘定与冻结 [`196 批档`](audits/196_0CR_S1_HARBOR_CHAIN_AND_MANIFEST_FREEZE_2026-10-05.md)
> → 跑批六日 [`197`](audits/197_0CR_S2_D1_SIX_QUESTIONS_2026-10-05.md)／[`198`](audits/198_0CR_S2_D2_SIX_QUESTIONS_2026-10-05.md)／
> [`199`](audits/199_0CR_S2_D3_SIX_QUESTIONS_2026-10-05.md)／[`200`](audits/200_0CR_S2_D4_SIX_QUESTIONS_AND_0CT_REGISTRATION_2026-10-06.md)／
> [`201`](audits/201_0CR_S2_D5_SIX_QUESTIONS_2026-10-06.md)／[`202 批档`](audits/202_0CR_S2_D6_SIX_QUESTIONS_ROUND_COMPLETE_2026-10-06.md)
> → 收口 [`205 批档`](audits/205_0CR_S3_HARBOR_CONVERSION_PUBLISH_0BZ_S4_2026-10-07.md)。
> 本档逐题读数**不引二手账面**：全部机械提取自逐题 `evaluation.json`＋journal 去重（RUN 按最大
> 快照去重；提取器 `scan_0cr.py`，工件 `D:/tb-eval/scbench/0cr_official/readings_<题>.txt`）。
> 边界：k=1 筛查轮；**非官方成绩提交**（对外形态＝Harbor 摩擦产物发布，双标签在案）；评测哲学
> ＝题目是找 bug 与不足的工具，分数仅作评判参考（152 批裁决）。

---

## 1. 结论摘要

| 项 | 读数 | 说明 |
|---|---|---|
| **checkpoint solved** | **134/196 = 68.4%** | 每档 Core 全绿记 1；官方判分器（pytest oracle、pass_policy=any） |
| 任务级全档 solved | **11/36 = 30.6%** | Easy 7／Medium 2／Hard 2（逐日表机械复算；更正 202 档「6/1/4」，见 §9） |
| 全坍缩（0 分档起全程未解） | **3/36** | dag_execution 0/3、eve_jump_planner 0/3、eve_market_tools 0/4 |
| 近失族（差 1–2 档） | ≥9 题 | circuit_eval、meshctl、sheeteval、file_merger、xjq、migrate_configs、file_query_tool、mvvault、mocked_http |
| 跑批时长 | 六日墙钟：逐题合计 **≈36.5h**（批口径 ≈41h 含装置开销） | D1–D6 ≈5h00／5h30／4h40／5h42／7h29／8h09 |
| 规模 | 4,593 模型步／6,157 工具调用／186 次压缩 | 全轮合计（机械可复算） |
| 对照：官方榜单 v1.0 | Top＝GPT 5.5/Codex 28.1% Isolated Solve；Opus 4.6/Claude Code 20.9% | **口径不同不可直比**（§6）；同构口径（isolated 满通过占比）下本轮 **30.6%** 方向性略高（§13） |
| 实际成本（DeepSeek 控制台） | **¥157.49≈$22.0**（国庆全低谷价；hit 693.9M／miss 606.3M／output 87.5M） | 论文最便宜 run 的 1/3.8、$/CKPT 1/8.1（§14） |
| 对照：论文 strict pass | Opus 4.6 17%／GPT-5.4 11%／HumanLayer 复测 Opus 5 24% | 同上 |
| 摩擦主线收获 | 0ct 立项＋0cq 家族台账 6 例＋0cs 首样本＋0bz S4 旧第 2 针归零＋4 题新参考解缺陷 | §7／§8 |

三点必须与数字同读：

1. **单一快照**：载体 0.8.14（身份门 `fc990a8a…`）＋deepseek-v4-flash 全程不换；轮内发现的
   摩擦全部**登记不顺延**（0ct/0cq/0cs 等在役摩擦如实计入读数）——分数是「这套快照」的成绩，
   不是框架上限。
2. **k=1 与论文同形**：官方实验设置原文「we select a single run per model」；本仓冻结口径
   （seed 42／pass_policy any／one_shot off／字母序全目）即官方形态。206 批勘误：官方无
   「≥5 试次/题」门槛，152 批该句系评估期成本提醒、非判据。
3. **主线是摩擦不是分数**：全轮零装置性失败、零重跑；写控 9 例 block 逐一验尸定谳（§7.1）；
   压缩面旧「第 2 针」在全语料归零（§7.5）——后者是本轮对框架最有价值的单条机械结论。

---

## 2. 口径与装置

| 项 | 值 | 证据位置 |
|---|---|---|
| Harness | 官方 slop-code-bench，pin `main=31ceea3`（harness）＋`38d627e`（scb-problems 题库） | 196 批冻结 |
| 题集 | 全目 36 题／196 checkpoint，官方发现序（字母序），难度 Easy 12／Medium 12／Hard 12 | 冻结 manifest（机器副本 `0cr_official/manifest.json`） |
| 官方参数 | seed 42 默认／pass_policy any／one_shot off（源码逐处核证） | 196 批 |
| 模型 | `deepseek/deepseek-v4-flash`（单一快照轮内不换）；**接口名注记（208 批，2026-10-07 用户告知＋官方 pricing 页核证）**：`deepseek-v4-flash` 系 legacy 别名、底层由 DeepSeek-V4.1-Flash 承接（Flash 价位；与 TB2.1 142 批 2026-09-30 注记同源）——现行官方接口名 `deepseek-flash`，改名随 0cv；**底模同代，本报告读数不受接口名影响** | 196 批冻结面 |
| 载体 | orz 0.8.14，身份门 `fc990a8a…`（Windows/Linux 双平台，194 批重建） | 逐题前置门抽查 |
| 执行形态 | k=1；每题独立 `slop-code run`，每题全新 agent 容器跨全部 checkpoint 连续会话，评估按档另起容器；题间全新容器 | 197 批 §4 |
| 读数收取 | `scan_0cr.py` 逐题出 `readings_<题>.txt`（evaluation.json＋journal 去重机械提取） | 各日档 §4 |
| 上传 | `scb_to_harbor.py` 官方管线全目转换 36/36＋`harbor publish --public`（`silverwhite/*`，双标签 `friction-hunting-artifact`/`not-official-scores`） | 205 批 |
| 定性 | k=1 筛查轮；摩擦寻找主线；非官方成绩提交 | 本档头注 |

---

## 3. 收官总账

### 3.1 checkpoint 级（134/196）

| 日 | 题 | ckpt 载荷 | solved | 日内读数 |
|---|---|---|---|---|
| D1（10-05，197 批） | ①–⑥ | 34 | **24/34** | cfgpipe 6/6 完美起跑；dag_execution 0/3 首个坍缩样本 |
| D2（10-05，198 批） | ⑦–⑫ | 27 | **19/27** | env_manager／etl_pipeline／eve_industry（Hard）三题全档 |
| D3（10-05，199 批） | ⑬–⑱ | 26 | **20/26** | eve_route_planner／execution_server／file_backup 三题全档 |
| D4（10-06，200 批） | ⑲–㉔ | 35 | **29/35** | forge／log_query／metric_transform_lang（Hard）三题全档；meshctl Hard 7/8 |
| D5（10-06，201 批） | ㉕–㉚ | 37 | **20/37** | 最重日（4 Hard）零全档；recli 官方轮 3/8（方差系列第 5 点） |
| D6（10-06，202 批） | ㉛–㊱ | 37 | **22/37** | textdrop 6/6；test_translator 2/8 全轮最长墙钟 189m |
| **合计** | **36** | **196** | **134（68.4%）** | 零装置性失败、零重跑、36/36 exit 0 |

### 3.2 难度面（逐日表机械复算）

| 难度 | 题数 | checkpoint | solved | 档均解率 | 任务级全档 |
|---|---|---|---|---|---|
| Easy | 12 | 67 | **59** | 88.1% | **7/12**（cfgpipe、env_manager、etl_pipeline、execution_server、file_backup、forge、textdrop） |
| Medium | 12 | 57 | **34** | 59.6% | **2/12**（eve_route_planner、log_query） |
| Hard | 12 | 72 | **41** | 56.9% | **2/12**（eve_industry、metric_transform_lang） |
| 合计 | 36 | 196 | **134** | 68.4% | 11/36 |

难度梯度方向与基准设计一致（Easy→Hard 衰减）；但 **Hard 档均解率 56.9% 显著高于论文口径的
全员早期坍缩形态**（论文最强 Opus 4.6 strict 17%）——同会话连续性＋压缩管理对「随 checkpoint
推进退化」的抑制作用是本轮最显著的成绩面观察（因果归因见 §6 边界）。

---

## 4. 逐题成绩表（36 题，k=1）

读法：「ckpt」＝checkpoint solved／该题档数；「末档」＝该题最后一个 checkpoint 的 core/full
测试通过数；「墙钟」＝单题起止差；「步/调」＝模型步／工具调用。◆＝参考解缺陷脚注题
（oracle=测试非参考解，照跑照报，见 §8）。

| # | 题 | 难度 | ckpt | 末档 core | 末档 full | 墙钟 | 步/调 |
|---|---|---|---|---|---|---|---|
| 1 | cfgpipe | Easy | **6/6 全档** | 3/3 | 206/216 | 55.6m | 82/139 |
| 2 | circuit_eval | Medium | **7/8** | 15/17 | 564/566 | 80.7m | 176/227 |
| 3 | code_search | Easy | 4/5 | 13/13 | 102/104 | 33.1m | 86/149 |
| 4 | dag_execution | Hard | **0/3 坍缩** | 0/3 | 33/51 | 34.9m | 126/208 |
| 5 | database_migration | Medium | 3/5 | 1/3 | 123/137 | 39.6m | 132/174 |
| 6 | datagate | Easy | 4/7 | 16/16 | 382/405 | 56.0m | 156/214 |
| 7 | dynamic_buffer ◆(ck4) | Hard | 1/4 | 7/20 | 84/172 | 82.6m | 192/188 |
| 8 | dynamic_config_service_api | Medium | 2/4 | 4/6 | 45/81 | 45.6m | 110/218 |
| 9 | env_manager | Easy | **5/5 全档** | 4/4 | 280/304 | 43.3m | 110/161 |
| 10 | etl_pipeline | Easy | **5/5 全档** | 4/4 | 158/164 | 32.8m | 71/113 |
| 11 | eve_industry ◆(ck5) | Hard | **6/6 全档** | 2/2 | 80/80 | 86.2m | 231/266 |
| 12 | eve_jump_planner | Medium | **0/3 坍缩** | 0/1 | 3/31 | 39.6m | 125/144 |
| 13 | eve_market_tools ◆(ck1–3) | Hard | **0/4 坍缩** | 3/8 | 14/75 | 55.3m | 86/102 |
| 14 | eve_route_planner | Medium | **3/3 全档** | 1/1 | 27/41 | 43.3m | 89/88 |
| 15 | execution_server ◆(ck6) | Easy | **6/6 全档** | 14/14 | 70/70 | 44.0m | 81/194 |
| 16 | file_backup | Easy | **4/4 全档** | 1/1 | 62/89 | 34.9m | 99/134 |
| 17 | file_merger ◆(ck2–3) | Medium | 3/4 | 14/19 | 140/147 | 58.5m | 120/196 |
| 18 | file_query_tool | Medium | 4/5 | 5/8 | 75/81 | 43.7m | 128/147 |
| 19 | forge | Easy | **8/8 全档** | 3/3 | 266/295 | 37.3m | 95/150 |
| 20 | l2m | Easy | 3/5 | 4/6 | 40/64 | 42.7m | 87/101 |
| 21 | layered_config_synthesizer | Medium | 1/4 | 7/9 | 83/98 | 53.7m | 92/115 |
| 22 | log_query | Medium | **5/5 全档** | 3/3 | 334/336 | 44.4m | 147/229 |
| 23 | meshctl | Hard | **7/8** | 3/3 | 77/78 | 96.8m | 111/229 |
| 24 | metric_transform_lang | Hard | **5/5 全档** | 1/1 | 69/74 | 67.5m | 189/259 |
| 25 | migrate_configs | Easy | 4/5 | 3/3 | 102/129 | 56.2m | 138/179 |
| 26 | mocked_http | Hard | 6/8 | 3/3 | 168/200 | 106.5m | 240/243 |
| 27 | mvvault | Medium | 4/6 | 5/5 | 40/42 | 53.1m | 113/140 |
| 28 | pwd_manager | Medium | 1/5 | 4/6 | 179/257 | 41.3m | 77/122 |
| 29 | recli | Hard | 3/8 | 5/7 | 153/255 | 113.9m | 150/162 |
| 30 | rejector | Hard | 2/5 | 2/3 | 74/79 | 78.0m | 97/94 |
| 31 | sheeteval | Hard | **6/7** | 1/3 | 127/164 | 121.6m | 110/179 |
| 32 | sith | Hard | 3/6 | 14/21 | 199/228 | 73.8m | 173/185 |
| 33 | test_translator | Hard | 2/8 | 42/50 | 827/2069 | **189.0m**（全轮最长） | 207/277 |
| 34 | textdrop | Easy | **6/6 全档** | 3/3 | 182/183 | 34.1m | 116/196 |
| 35 | trajectory_api | Medium | 1/5 | 2/3 | 127/373 | 40.2m | 91/171 |
| 36 | xjq | Easy | 4/5 | 18/18 | 159/167 | 30.4m | 60/64 |
| — | **合计** | — | **134/196** | — | — | **≈36.5h** | 4,593/6,157 |

---

## 5. 成绩面形态学（分数之外在读什么）

- **全档 11 题**（§3.2 清单）：六题含 late-checkpoint 全绿（forge 8/8、textdrop 6/6、
  eve_industry 6/6、execution_server 6/6、log_query 5/5、metric_transform_lang 5/5）——
  「后期档必然坍缩」的基准设计预期在 11/36 题上被完整证伪。
- **全坍缩 3 题**：dag_execution（8/12→2/5→0/3 单调坍缩＝设计所测退化形态的首个实样本）、
  eve_jump_planner（三档全败、full 停 3/31＝任务本体未起步）、eve_market_tools（◆脚注题本体
  未解）。3/36 的坍缩率远低于论文口径的全员早期坍缩。
- **近失族（差 1–2 档，≥9 题）**：circuit_eval 7/8（末档 2 core 失）、meshctl 7/8（Hard，
  仅 ck4 差 1）、sheeteval 6/7（六连绿后末档失）、file_merger 3/4（◆ck2–3 双满分后末档失）、
  xjq 4/5、migrate_configs 4/5（仅 ck2 单测试失）、file_query_tool 4/5、mvvault 4/6、
  mocked_http 6/8（重依赖题最佳单日成绩）。近失族的失分档多为**末档**或**单档单测试**——
  长程保持力的直接证据面。
- **走低/塌陷族**：test_translator 2/8（ck5 塌陷后未恢复；题面 2,069 测试、全轮最长墙钟
  189m＝能力边界样本）、recli 3/8（ck6 塌陷与三次历史跑同型＝轨迹中段走偏形态再现）、
  pwd_manager 1/5／rejector 2/5／trajectory_api 1/5／layered_config 1/4／dynamic_buffer 1/4
  （各含单档塌陷）。
- **非单调恢复**：datagate 前三档失分、后三档回稳（4/7）——「塌陷不可逆」不成立的本轮样本；
  mocked_http ck7 失分 ck8 收回同型。
- **方差对照**：recli 官方轮 3/8 为五跑系列第 5 点（2/8→3/8→3/8→8/8→3/8）——k=1 结果面
  双峰的量化注记，一切单题读数不作能力面结论。

---

## 6. 官方对照与口径边界

| 对照线 | 读数 | 可比性 |
|---|---|---|
| scbench.ai 榜单 v1.0（19 模型/agent） | Top GPT 5.5/Codex **28.1%** Isolated Solve；GPT 5.4 25.5%；Opus 4.6/Claude Code 20.9% | **不可直比**：榜单口径（Isolated Solve）与本轮迭代式 checkpoint solved（pass_policy=any）定义未对齐 |
| 论文 strict pass | Opus 4.6 17%／GPT-5.4 11%；HumanLayer 复测 Opus 5 24%（4/17，全部集中早期 checkpoint） | 方向性参照：本轮任务级全档 30.6% 高于全部在榜/论文条目，但 strict/any 未对齐 |
| 本轮 checkpoint 级 | 134/196＝68.4%（any 口径、同会话迭代） | 仅作评判参考、不对榜、不外推 |

三条边界：①**k=1**——recli 五跑 2/8→3/8→3/8→8/8→3/8 的方差系列量化了单跑读数的不确定性；
②**单快照**——0ct（.gsa 读向拦截）/0cq 家族（写控误拦 5 例）等在役摩擦如实计入，修复后重跑
预期只升不降，但本轮不作该断言；③**模型价位的含义**——deepseek-v4-flash 为 flash 价位模型，
读数面高于榜单前沿模型条目的现象**优先归因 harness**（会话连续性＋压缩管理＋黑板），但本轮
无「同模型换 harness」对照实验，不作单因断言。**对外口径**＝Harbor 摩擦产物发布（205 批，
双标签＋README 非官方成绩声明）不变。**成绩提交路径更正（208 批复核）**：官方**无文档化提交流程**
（README／FAQ／docs/evaluation／docs/metrics／contributing 全查无，亦无试次数门槛——206 批
「无公开通道」表述据此收窄为「无文档化流程」）；**事实上的入榜通道＝Harbor 公开 run**（榜单页
按 Model×Harness 聚合公开 run——「Showing best version by % Checkpoints」；题目本体即以
Harbor dataset 分发）；silverwhite 转换任务已发布，后续若以 Harbor runtime 跑 orz 并
`--public` 发布即构成榜单可见条目——属新决策，随用户裁决；Discord 为人工协调通道。

---

## 7. 摩擦台账汇总（主线；全轮零装置性失败、零重跑）

### 7.1 写控（机械写控 9 例 block 逐一验尸）

全轮逐日审查次数（write_control_review 事件口径）：314／970／334／372／509／438；
warn 274 例全留痕不阻断不触模型面（明细多为 `rc=$?`／`*args` 等**散文匹配假阳性**）；block 共 **9 例**：

| # | 日/题 | 定谳 | 家族 |
|---|---|---|---|
| 1 | D1 dag_execution | `//` 路径塌缩→根祖先臂（保守臂语义，真阳性形态；`//` 归一化收窄留观察） | 0cq 家族①散文路径塌缩 |
| 2–3 | D2 dynamic_buffer ×2 | heredoc 代码体内 `format` 词元（`std::string format;`／`"format"` 键）被动词扫描 | 0cq 家族②heredoc 体 |
| 4 | D2 dynamic_config_service_api | 多目标 `rm` 含 `.gsa/rollback` 整命令拒（合法部分一并拒＝保守硬边界设计内代价） | 按设计（写向保留面） |
| 5 | D3 file_merger | echo 单引号散文裸 `/` 进路径候选→根祖先臂 | 0cq 家族③引号散文 |
| 6 | D4 l2m | 写 `.gsa/runs/RUN-*/events.jsonl`（journal 本体） | 按设计（写向保留面） |
| 7 | D4 meshctl | `cp .gsa/rollback/*.bak /tmp/…` **读向复制**被拦 | **升 0ct**（§7.2） |
| 8 | D5 recli | heredoc Python 补丁内 `format` 变量名 | 0cq 家族②heredoc 体（第 3 例） |
| 9 | D6 test_translator | 写 `.gsa/rollback/…` 自建 .bak | 按设计（写向保留面＝0ct 写向保留的在役实证） |

**0cq 家族候选台账累计 6 例**（散文路径塌缩 1＋heredoc 体 3＋引号散文 1＋.gsa 读向 1〔升
0ct〕）；185/186 批已修的「直排散文拆片」形态全轮**零复发**（修复有效性正面实证）；每例模型
均 **1 轮改道**，零题因写控失败。候选修复（heredoc 体豁免扫描／引号串词元豁免／`//` 归一化）
随 0cq 后续批随用户裁决。

### 7.2 `.gsa` 读向拦截→0ct 立项（本轮最重要单条摩擦）

D4 meshctl 读向 `cp` 被整命令拦，用户定性「**读不是写…….gsa 台账完全开放，跑完 36 题第一个
修复**」→ 0ct 立项（60 → 61）；204 批 S1/S2 已落码（读全开放＋两段门转确认性＋写向保留面
8 条对照钉全过），S3 载体随下一重建批。本轮 36 题读数含此在役摩擦（单一快照纪律）。

### 7.3 0cs 工具名近似提示（S3 真机核证）

全轮 `Tool not found` 事件 **1 起**（D2 dynamic_buffer：幻影名 `run_cmd`→信封建议
`did you mean "run_terminal_cmd"?`→**下一手即纠正、1 轮恢复**），D1/D3–D6 零样本。
与 188 批基线（无建议面 ~1 轮/次试错）形态一致偏优，但 n=1 不作充分性结论（如实记）。

### 7.4 RLI（0.8.14 全机制在役的真机读数）

- **streak fire 94 次**＝91 单通道全带预测段（≤329–343B ≤400B 预算）＋3 双通道无预测段
  （184 批「多 fire 不附预测段」设计钉的首个真机样本 D4 layered_config `stall×3 slow×3`
  双越线一行，设计内形态非缺陷）；fire 密度与题难相关（env_manager 完美轮 0 fire／
  dynamic_buffer 坍缩轮 5 fire）。
- notice 1,323 条（spike 进入／迁移确认／coverage_gap／streak）；域机以 normal↔low_progress
  振荡为主（与 188 批形态同族）。
- **消费面**：0ck 注解＋0bd ⑦阈值推面在役；本轮未设消费率判据（0cj 线已闭合、判据移交记录
  在案），读数留 RLI 转正线（0am，挂起）O2 解挂时取用。

### 7.5 压缩面与 0bz S4（本轮对框架最有价值的机械结论）

全轮 186 次 `context_compressed`（mode＝model_selected/context_scale_window 混合、零机械兜底
强压）。0bz S4 离线对账（205 批，36 题 244 去重 journal 全语料）：

- **+2 事件 137/137 全部恰尾槽分歧**（stable≡count−3）、尾槽前分歧 **0 例**——192 批修码钉
  `d4_rerender_diverges_only_at_its_tail_slot`（D4 机械段移尾）在官方轮全语料 **100% 成立**，
  修复前形态（头部槽全前缀重价、+2 hit 崛至 ~12.8K）**0 例**；
- **+2 hit 中位 115,968 tokens**（min 43K／max 265K）＝历史全程缓存复用、仅尾槽 3 消息设计内
  刷新——长程会话的缓存经济学读数；
- 空跑 compress 全轮 2 次、零整窗分叉；自发塌陷启发式候选 15 例（checkpoint 切换合法头变更
  混杂，非硬判据口径，如实记）。

### 7.6 传输／权限／资源

`transport_retry` 全轮 **6 次**（D2 1／D3 2／D4 3）**均自愈零再发**；`resource_denied` 0；
权限面全程自动放行零拒绝；counterexample_gate 196（每档 1，设计内常量）。

### 7.7 框架引导面（按设计工作的证据）

`&` 后台符→`is_background=true` 指引、工作区门拒绝（`grep`/`read_file outside_workspace`）、
参数形状拒绝（`search_replace` 缺 `old_string` 等）——全部模型侧语义失败或按设计引导，
**框架侧零故障、零干扰摩擦**。

### 7.8 装置事件（跑批窗零发生；S3 转换窗四件全收敛）

跑批六日**零装置性失败、零重跑**（装置重跑条款未触发）。S3 收口窗（转换/发布，非跑批）四件
如实登记：problems 清单 CRLF 行尾致 34 题空跑（write_text Windows 行尾；重跑收敛）、Docker
Desktop 引擎 500（重启恢复 29.6.2）、loop 2 停止误杀 dag_execution oracle 阶段（--force 重跑
rc=0）、forge apt 网络瞬断两轮（第三轮 rc=0）——**均不影响任何跑批读数**。

---

## 8. 转换忠实度脚注（Harbor 全目转换 36/36 的副产品）

官方 `scb_to_harbor.py` 管线（pin `38d627e`）全目转换：34 题全链 rc=0；6 题 oracle 缺陷脚注
（rc=4，**oracle=测试非参考解**——脚注为转换忠实度记录、非 agent 分数）：

| 题 | 性质 | 明细 |
|---|---|---|
| dynamic_buffer／eve_market_tools | 已知（KNOWN_ISSUES） | 预期脚注兑现 |
| **env_manager** | **新发现** | ck3 strict 0.984（参考解差约 2 测试；agent 官方轮 5/5 全绿对照） |
| **file_backup** | **新发现** | ck2 0.84／ck3 0.68（agent 4/4 全绿对照） |
| **mvvault** | **新发现** | ck5 strict 0.9946（差 1 测试；agent 4/6） |
| **test_translator** | **新发现** | 全档 strict 0.69–0.75（参考解挂两三成测试；agent 2/8 与该题难度面互证） |

KNOWN_ISSUES 三题（eve_industry／file_merger／execution_server）在本 pin 未显形（rc=0）；
`38d627e`（fix/harbor-port-issues-30-31）后已修或环境相关。agent 全绿与参考解不过的并存的
四题（env_manager／file_backup／execution_server◆ck6／file_merger◆ck2–3）＝「测试为权威」
条款的累计实证。**新发现 4 题可 upstream 上游仓**（候选，随用户裁决）。

---

## 9. 数据完整性与账目更正（本报告与既有账面的差异）

逐题读数机械可复算（`readings_*.txt`），据此核对既有账面，更正四处（**本档数字优先**）：

1. **202 批档「Easy 6／Medium 1／Hard 4」**：与逐日表难度列不符——机械复算为 **Easy 7／
   Medium 2／Hard 2**（§3.2）。
2. **D3–D5 批档运行计数「任务级全档 9／12／12」**：含「差 1 档」题误计；收官机械清单为
   **11 题**（202 批档 §1 命名清单为准，本档 §3.2/§4 复算一致）。
3. **202 批档 D6 合计行「1567/2984」**：与逐行复算 **1621/3184** 不符（逐行为准）。
4. **「六日 ≈41h」**：为批口径（含发射/监控/收取与日间装置开销）；逐题墙钟合计 **≈36.5h**
   （§4 表），两口径并存如实注记。

---

## 10. 披露要件清单（逐条核对）

| # | 要件 | 状态 |
|---|---|---|
| 1 | k=1 筛查轮定性；非官方成绩提交；对外形态＝Harbor 摩擦产物（双标签） | §头注／§6 |
| 2 | 单一快照：0.8.14 `fc990a8a…`＋deepseek-v4-flash 全程不换；轮内零修复顺延 | §1／§2 |
| 3 | 官方参数与题序：seed 42／any／one_shot off／字母序全目 36 题 | §2 |
| 4 | 逐题读数机械提取、可复算；本档与既有账面差异已更正并注记 | §4／§9 |
| 5 | 脚注题（参考解缺陷）照跑照报、oracle=测试非参考解 | §4◆／§8 |
| 6 | 装置事件全录（跑批窗零发生；转换窗四件收敛） | §7.8 |
| 7 | 在役摩擦计入读数的披露（0ct/0cq/0cs） | §1／§7 |
| 8 | k=1 方差（recli 五跑系列）与口径不可直比声明 | §5／§6 |

---

## 11. 边界与不可外推项

1. k=1 单试次：不含方差信息；recli 五跑双峰（2/8–8/8）直接量化了单题读数的波动幅度。
2. 单快照：读数＝「0.8.14＋deepseek-v4-flash＋在役摩擦（0ct/0cq/0cs）」的组合成绩；修复后
   形态未测，不作「框架上限」推断。
3. 官方榜单/论文读数不可直比（isolated vs iterative／strict vs any 未对齐）；「高于在榜条目」
   仅为方向性观察。
4. harness 归因无对照实验（同模型换 harness 未跑），不作单因断言。
5. 墙钟含容器起止与评估段，非 agent 纯运行时长；API 现金未入账。
6. 0cs 消费/恢复读数 n=1；RLI 消费面本轮未设判据。

---

## 12. 复现入口

| 用途 | 位置 |
|---|---|
| 冻结 manifest（36 题字母序＋196 ckpt） | `D:/tb-eval/scbench/0cr_official/manifest.json`＋[`196 批档 §3`](audits/196_0CR_S1_HARBOR_CHAIN_AND_MANIFEST_FREEZE_2026-10-05.md) |
| 逐题官方读数（权威） | `D:/tb-eval/scbench/0cr_official/readings_<题>.txt`（36 份；`scan_0cr.py` 提取） |
| 官方多维逐档原始记录 | 各 run 目录 `checkpoint_results.jsonl`（strict/core/isolated pass rate＋scb-check 0.1.3 verbosity/erosion）；聚合件 `0cr_official/multidim_aggregate.py`／`multidim_aggregate.json`（210 批） |
| 消耗与账单分项 | DeepSeek 控制台逐日分项（用户提供；215 批记录于 §14：hit／miss／output＋¥157.49 实收） |
| 逐题跑批日志 | `D:/tb-eval/scbench/0cr_official/logs/<题>_run1_0cr.log` |
| journal 与轨迹卷 | 36 题输出树 `outputs/deepseek-v4-flash/orz_just-solve_none_2026100*/`（`.gsa/runs/RUN-*/events.jsonl` 为唯一机器可读留档） |
| 0bz S4 对账工件 | `0cr_official/bz_s4_round_full.txt`（逐压缩行）＋`bz_s4_round_files.txt`（语料清单） |
| Harbor 发布回执 | `0cr_official/publish.log`／`publish_dataset.log`；数据集 `hub.harborframework.com/datasets/silverwhite/slopcodebench-friction` |
| 六日跑批账 | [`197–202 批档`](audits/202_0CR_S2_D6_SIX_QUESTIONS_ROUND_COMPLETE_2026-10-06.md)（逐日 §1 记分卡＋§2 摩擦台账） |

> 本档即 205 批 Harbor 数据集 README 所引「see round report」的指向件（仓内路径
> `docs/SCB_V1_36_FULL_ROUND_REPORT_2026-10-07.md`）。

---

## 13. 官方多维聚合读数（离线补账；210 批，零重跑）

> 数据源＝官方 harness 原生产物：36 个 run 目录的 `checkpoint_results.jsonl`（逐档记录
> strict/core/isolated pass rate＋scb-check 复合分）——判分器与质量检查器均为官方管线自带、
> 随跑随采，本节为纯离线汇总（`0cr_official/multidim_aggregate.py`，语义对齐官方 `metric.py`
> dataset 级聚合）。§4 的二元 checkpoint solved（any 口径）之外，这是与官方榜单同维度的完整
> 读数；**「跑完即有官方多维」同时说明：多维缺口属于出账口径选择，不构成重跑理由。**

### 13.1 官方指标定义（源码核证，pin `31ceea3`）

- `strict_pass_rate`＝全部测试通过率；`core_pass_rate`＝Core 组通过率；**`isolated_pass_rate`
  ＝当前 checkpoint 自身测试（扣除 Regression）通过率**（`metrics/checkpoint/extractors.py`）。
- 官方 dashboard 二值 checkpoint 通过＝strict 满分 ∨ isolated 满分 ∨ 全部测试通过
  （`dashboard/data.py` `passed_chkpt`）。
- `verbosity`/`erosion`＝`scb-check==0.1.3` 复合分（版本钉死、随档记录；跨版本不可比）。

### 13.2 全轮读数（36 题／196 checkpoint）

| 口径 | 值 | 官方对照 |
|---|---|---|
| **isolated 满通过占比**（isolated_pass_rate≥1.0） | **60/196＝30.6%** | 榜首 GPT 5.5/Codex「Isolated Solve」28.1%——同构口径下方向性略高〔榜单聚合公式未文档化，以此为最合理对齐推定、待官方确认〕 |
| strict 满通过占比（≥1.0） | 44/196＝22.4% | 论文 strict：Opus 4.6 17%／GPT-5.4 11%——同构对照 |
| core 满通过（any 口径＝§3 定档） | 134/196＝68.4% | 本仓既有账面 |
| isolated pass rate 均值（部分得分） | 79.8% | 与二值口径不可直比，两口径并列为避免误读 |
| strict / core pass rate 均值 | 83.0% / 83.4% | 同上 |
| **verbosity 均值**（低好） | **0.289** | 榜首 GPT 5.5 0.269／GPT 5.4 0.193——同量级 |
| **erosion 均值**（低好） | **0.613** | 榜首 0.494／GPT 5.4 0.278——偏高，如实记 |
| verbosity 增长率（逐档转移） | 58.75% | 官方设计的退化信号（论文：~80% 轨迹上升为常态——本轮低于该常态） |
| erosion 增长率 | 65.0% | 同上 |

### 13.3 难度面（pass rate 均值）

| 难度 | checkpoints | core | strict | isolated | verbosity | erosion |
|---|---|---|---|---|---|---|
| Easy | 67 | 96.3% | 90.6% | 91.1% | 0.285 | 0.552 |
| Medium | 57 | 80.2% | 80.9% | 76.1% | 0.259 | 0.622 |
| Hard | 72 | 73.7% | 77.6% | 72.1% | 0.322 | 0.666 |

### 13.4 塌陷信号面（isolated 首→末档个体形态）

- **首档满分后末档走低**（基准设计的「随档退化」清晰可见）：eve_route_planner 1.00→0.38、
  dynamic_buffer 0.93→0.18、dag_execution 0.85→0.00、file_query_tool 1.00→0.60、
  dynamic_config_service_api 1.00→0.56、database_migration 1.00→0.70、l2m 0.98→0.62、
  recli 1.00→0.68、migrate_configs 1.00→0.67；
- **反向（末档不降反升）**：env_manager 0.36→0.88、file_backup 0.59→0.86、mocked_http
  0.83→0.94、etl_pipeline 0.90→1.00、cfgpipe 0.89→1.00——官方口径下非单调恢复同样成立
  （与 §5 datagate 形态互证）。

### 13.5 边界

1. 官方「Isolated Solve」的榜单聚合公式未在文档钉死（本仓 pin 的 metrics 文档无 isolated
   词条）——13.2 对照以「isolated 满通过占比」为最合理对齐推定，**待官方确认**；均值/二值
   两口径全量给出，防口径误读。
2. erosion/verbosity 为 scb-check 0.1.3 规则集产物，跨版本不可比（版本随档记录）。
3. 本节为离线汇总，零新跑批、零模型调用；聚合脚本与产物在仓外 `0cr_official/`（复现入口见
   §12）。

---

## 14. 成本与消耗（DeepSeek 控制台分项对账；215 批，零重跑）

> 数据源＝DeepSeek 官方控制台逐日分项（用户提供，2026-10-07）：**hit 693,859,598／miss
> 606,333,056／output 87,526,542**（console 四列中后两列均为 output 分项），合计
> **1,447,719,196 tokens**；实收 **¥157.49**（run 窗口 10-05～10-07 恰逢国庆假期、全程低谷价；
> 官方 off-peak 价目：hit $0.003／miss $0.15／output $0.60 每 1M）。harness 侧 cost/token
> 字段全零（orz 未回传计量），控制台为唯一权威。

### 14.1 消耗结构

| 列 | tokens | 占总量 | 账单占比（off-peak 价） |
|---|---|---|---|
| Input（cache hit） | 693,859,598 | 47.9% | **1.4%**（¥2.08） |
| Input（cache miss） | 606,333,056 | 41.9% | **62.5%**（¥90.95） |
| Output | 87,526,542 | 6.0% | **36.1%**（¥52.52） |
| 合计 | 1,447,719,196 | — | **¥145.55（账面）／¥157.49（实收）** |

效率读数：**315.2K tokens/步**（输入 283.1K＋输出 19.06K——输出侧为 flash 思考链形态）、
**40.2M tokens/题**；cache hit 占输入 **53.4%**。

### 14.2 账单复核与一处文档出入

- 按 off-peak 官方价目（USD 数值同 numerics 的 RMB 计）复算：¥2.08＋¥90.95＋¥52.52＝
  **¥145.55**；实收 ¥157.49＝**×1.082**（+8.2%，计费粒度/折算量级）——**对账成立**。
- 出入登记：官方中文文档页价目（miss ¥1–2／output ¥4–8 每 M）与控制台实收差约 6 倍——
  以控制台实收为准，该文档页待核（控制台账单明细的单价列可一键钉死）。
- 反推实际等效单价 ≈ hit ¥0.0035／miss ¥0.176／output ¥0.71 每 1M。

### 14.3 miss 占比 46.6% 的构成（「命中与未命中同量级」的解释）

miss 不是缓存失效，是三类来源的合计：①**每步新增段的不可约下限**——API 无状态、每个请求
全量重发，长会话逐 step 追加的工具结果与消息天然全部计 miss；②**186 次压缩的 prefix 手术**——
压缩/黑板改写后该段重计 miss（0bz 移尾修复已把改写代价压到「仅尾槽 3 条」，否则更高）；
③**36 次换题冷启动**。hit/miss 同量级是「长会话＋频繁上下文手术」形态的正常读数；若 miss
占比远超一半（如 70%+）才指向缓存机制失效。

### 14.4 成本表现（与论文 Table 1 对照）

| 口径 | 本轮 | 论文对照（Table 1） |
|---|---|---|
| per-run | **¥157.49≈$22.0** | 最低 GPT 5.3 Spark **$84.46**；全场 $84–423 → **1/3.8** |
| $/CKPT（总成本÷196 档） | **$0.112** | 最低 $0.91（GPT 5.3 Spark）→ **1/8.1**；最贵 $4.55 → 1/40 |
| $/isolated 满通过档 | $0.37（60 档） | — |

成本形态一句话：**缓存折价把 53% 的输入变成 1.4% 的账单（4.7 倍有效折价），钱花在 miss 段
与深思考输出**；现金成本为论文全场最低档的 1/8（k=1、flash 价位、单一快照口径）。"$500/run"
之说出自官方 contributing 页、非论文（论文实测 $84–423 与其同量级）。

### 14.5 边界

1. 控制台分项由用户提供、本批照录未核（harness 侧计量为零）；第四列为 output 第二分项
   （两列合计 87.5M）。
2. 汇率 7.15 近似；国庆低谷价适用整个 run 窗口（10-05～10-07 均在假期内）。
3. 官方中文文档页价目与实收的 6 倍出入未定谳（待控制台单价列核对）；$500 仅为 contributing
   页口径，不作达标判据。

---

## 15. 形态勘正注记（0cw S1 随批落；228 批，零重跑）

> 226 批形态分歧核证（官方 runner 逐档 `reset_context=True`＋论文 §2/App C 双原文 vs 本轮连续
> 会话变体）与本批 S1 官方口径勘定的两处可比性勘正，按「dated 档不回改」纪律以本节注记落；
> §1–§14 原文与读数不改。

1. **§5「证伪」表述勘正**：全档 11 题（含六题 late-checkpoint 全绿）是在**连续会话变体**下取得；
   「后期档必然坍缩」是基准对**重置形态**的设计预期（论文 §2：agent 只带工作区、不带前序对话）。
   连续变体的反例不构成对该预期的证伪——形态变量未对齐。§5 各「证伪」字样收窄为「连续会话变体
   观察」；该预期是否成立由 0cw 重置形态轮（同模型 A/B）检验。
2. **§6 对照口径勘正**：本表「不可直比」三行之外，本轮与论文/榜单的全部差值还混杂**会话形态**
   因子（0cr＝连续、论文/榜单＝重置）；边界③「优先归因 harness（会话连续性＋压缩管理＋黑板）」
   收窄为「harness 差（含会话形态）整体」——单因断言待 0cw 读数（0cw＝隔离「会话连续性」单因子
   的同模型对照轮）。134/196、core 相位 94.0→77.7 等读数维持「连续会话变体」标注，不与重置形态
   条目作方向性强弱比较。
3. **§2 口径补注**：本轮「官方口径」指独立 harness 的参数面（k=1／seed 42／pass_policy any/
   one_shot off／字母序）；会话形态为连续变体（158/196 两处登记已由 226 批勘正）。0cw 的口径
   基准转为 Harbor 上榜形态（官方数据集 `gabeorlanski/slopcodebench`＋每 checkpoint 全新会话＋
   7200s/档帽；228 批 §1）。
