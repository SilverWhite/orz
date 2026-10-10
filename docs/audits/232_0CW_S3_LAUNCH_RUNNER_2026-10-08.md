# 232 批：0cw S3 Harbor run 起跑——成本门放行＋起跑器落码＋全目 36 题逐题直跑（2026-10-08）

> **用户令**：「请启动0cw S3吧，每个checkpoint都收结果，早上8点以后就不放新的跑分了，因为9点就要到计费高峰期了，
> 8点前的正常收尾即可」。
> **性质**：实施＋跑批启动批——落码面全在评测侧本地件（`D:/tb-eval/scbench/run_0cw_s3.py`，不入父仓、
> 不入 orz）；零 orz 源码、零父仓源码；计数不变 **56**；未提交、未推送。**S3 状态＝在跑**
> （首窗口 03:23 起；窗口收官读数与全目读数留 S3 收官批／S4 收取批落）。

## §1 本批新约束与形态裁决

| 项 | 内容 |
|---|---|
| 成本门放行 | 226 批估计 $40–80 的成本门由用户放行（「请启动0cw S3吧」）；227 裁决的实时花销监控＋暂停权照常在位 |
| **08:00 停放新题门**（新增，用户令） | 本地 08:00 后**不再放新题**（9:00 计费高峰）；08:00 前已起跑的题**正常收尾**（在途题后续档照跑，含跨峰时段） |
| 每 checkpoint 收结果（用户令） | 逐档结果即时落台账（§3）；harbor 逐档工件（step_result.json／reward.json／轨迹／.gsa 副本）随 job 目录全保留 |
| **起跑器形态＝逐题单 job 串行**（本批细化 227「全目直跑」的承载形态） | harbor 单 job 无题间时间门，**逐题 job 是唯一能同时落实 08:00 门＋中途暂停权＋断点续跑＋每题即时上传的形态**；与 0cr 串行执行同形；Hub 侧每题一公开 job，Model×Harness 聚合面核证留 S4 |

## §2 起跑器（`scbench/run_0cw_s3.py`，评测侧本地件，stdlib-only）

- **题序**：36 题官方字母序（数据集目录实扫 ASCII 排序，运行时校验恰 36 题）：p01 `cfgpipe` …
  p04 `dag_execution` … p29 `recli` … p36 `xjq`。
- **逐题 job**：`0cw-s3-p<i:02d>-<task>`；harbor 0.23.0（`scbench/harbor/.venv` editable）；
  `--agent scb_agents.orz_multistep:OrzMultiStep --model deepseek-flash --ak orz_binary=D:/tb-eval/orz-linux/orz
  --dataset gabeorlanski/slopcodebench@sha256:aec29354… --include-task-name gabeorlanski/<task>
  --n-attempts 1 --env-file D:/tb-eval/.env --upload --public --yes`；重置形态（不传 `continuous`）、
  7200s 档帽零覆盖（零 timeout multiplier）、零重试。命令拼装经 `--print-config` 干跑核证＝
  与 230 §7 预检 config 逐位同形（agent/model/kwargs/dataset pin/单题过滤）。
- **每 checkpoint 收结果**：题毕即解析 `steps/checkpoint_*/agent/step_result.json`（session id／
  stop_reason／rounds／elapsed／timeout_cfg/grace_cfg）＋`verifier/reward.json`（core/strict/isolated/
  erosion/verbosity 八指标）→ 追加 `jobs-0cw/s3_checkpoint_ledger.jsonl` 逐档行；题级行
  （status／rc／wallclock／core_mean／hub URL／题前后余额）→ `s3_problem_ledger.jsonl`。
- **花销监控**：DeepSeek `/user/balance` 轮询（10 分钟粒度 → `s3_balance_log.jsonl`）＋题前后余额
  对账入题级行；**余额守卫 ¥30**＝低于守卫线不放新题（227「实在不行就暂停」的机械护面；
  补足后重入即续）。
- **暂停权**：`jobs-0cw/S3_PAUSE` 文件在位＝当前题收尾后不再放新题（重入续跑）。
- **断点续跑**：已落 `result.json` 终态（completed/errored）的题自动跳过；`unfinished` 残留
  **拒绝覆盖、停机待人工**（防半题误重入）；errored 题照记不重跑（k=1 零重试口径，重跑与否留 S4 裁决）。

## §3 起跑读数（首窗口）

- **03:22:55** 起跑器启动；起始余额 **¥243.98**；门自检全过（03:2x 未及 08:00、无 S3_PAUSE、
  余额 ≥ 守卫线）。
- **03:23:00** p01 `cfgpipe` 起跑（job `0cw-s3-p01-cfgpipe`）；03:26 install 自检过
  （orz **0.8.15** os=linux、`continuous=False`、`step_timeout=7200s grace=180s`）；
  **step 1/6 起跑**（cfgpipe 6 档）＝重置形态在证。
- 预算估计（窗口规划用，非裁决）：预检单题 3 档 38.5 min；平均 ≈5.4 档/题 → 单题 ≈40–90 min；
  本窗口（03:2x–08:00 停放新题＋在途收尾）预计新完成 **5–8 题**；余题后续低谷窗重入续跑
  （`cd D:/tb-eval/scbench && python run_0cw_s3.py`，幂等跳过已完题）。

### §3A 首窗口收官补记（2026-10-08 08:59:50 收官；门行为与读数）

- **门行为在证**：08:00 门精确生效——p05 `database_migration` 07:45:02 起跑（峰前最后一题）、
  08:59:50 收官＝在途题正常收尾（尾段跨计费峰，用户已接受）；此后无新题。
- **本窗完成 5 题 / 22 档 / 零 errored 档**（k=1 全 end_turn；逐档 sid 全各异＝重置形态逐档在证；
  7200 帽读数逐档在证、零触发）：

| 题 | 档数 | 墙钟 | core 逐档 → 均值 | 花销（CNY 差分） | hub job |
|---|---|---|---|---|---|
| p01 cfgpipe | 6 | 83.4 min | 1.0×6 → **1.000** | 5.76 | `13e85e28` |
| p02 circuit_eval | 8 | 115.7 min | 1.0×7, 0.882 → **0.985** | 7.76 | `ce3edff8` |
| p04 dag_execution | 3 | 60.5 min | 0.833, 0.600, 0.000 → **0.478** | 4.06 | `7b25f333` |
| p05 database_migration | 5 | 74.8 min | 1.0, 0.667, 1.0, 0.833, 0.333 → **0.767** | 5.49 | `03700277` |

  读数面如实记（k=1 不作结论）：dag_execution 末档坍缩与 0cr/冒烟同题同型＝表现面复现；
  cfgpipe 全程满分、circuit_eval 仅末档微降＝重置形态下工作区跨档持久有效面。
  **p03 见 §3B。**
- **花销**：¥243.98 → **¥220.87**（本窗 **¥23.11** ≈ $3.3；22 档 ≈ ¥1.05/档；线性外推全目
  196 档 ≈ ¥206 ≈ $29 < 当前余额——粗外推仅供充值参考，不作裁决）。
- **起跑器修正（随批）**：`job_state` 判定勘误——harbor 对 env 构建失败 trial 记
  `n_completed=1 ∧ evals.n_trials=0, n_errors=1`，原「n_completed≥1 即 completed」误标；
  修正为「completed 须 evals 实数据 ≥1」，errored 优先识别。

### §3B p03 code_search 构建失败事故与处置（零数据；复跑待下一低谷窗）

- **事故**：06:42–06:44 p03 `code_search` job 「completed」但 **0 档 / 2.0 min / ¥0.04**——
  根因＝**镜像构建期 apt 断网**（`deb.debian.org` Unable to connect，87s 后 exit 100；
  RuntimeError；eval `n_trials=0, n_errors=1`）＝230 §3 设备侧同族（代理链路抖动，
  本窗其余四题构建/复用全正常）。
- **定性**：**零数据装置面事故**，非基准 trial（agent 未起跑、无 workspace 状态遗留）——
  不占 k=1 口径、不计入成绩面；沿 230 §3 先例处置。
- **处置**：① 证据保留＝job 目录改名 `0cw-s3-p03-code_search-erred-buildfail-20261008`（不入
  成绩、不删档）；② 名位让出＝`0cw-s3-p03-code_search` 将在下一窗口由起跑器全新起跑
  （断点续跑幂等）；③ 台账勘误行已追加 `s3_problem_ledger.jsonl`（errored-buildfail）；④
  起跑器 `job_state` 修正随批（§3A 末条）。
- **待决**：复跑落下一低谷窗（起跑器重入即取）；若构建再失败则升级设备侧排查（Docker 代理链路）。

### §3C 低谷窗判定勘正（官方计费规则核读；用户指认「现在是低谷了」触发；2026-10-08 18:3x）

- **官方规则（`api-docs.deepseek.com/quick_start/pricing` 当日核读；错峰价＝半价）**：
  高峰＝**周一至周五 01:00–04:00 与 06:00–10:00 UTC＝北京时间 09:00–12:00 与 14:00–18:00**
  （中国法定假日整周除外；周末与假日全程低谷）；**低谷＝其余全部时段**——即工作日
  18:00–24:00、00:00–09:00、12:00–14:00 全为低谷。与用户今晨口径互证（「9 点计费高峰」＝
  官方 peak1＝09:00 CST 逐位吻合）；deepseek-flash（V4.1-Flash）错峰价 input-miss $0.15／
  output $0.6 每 1M（半价面）。
- **勘正**：起跑器原「低谷＝00:30–08:00」为旧印象，**错**——晚间 18:00 后即低谷，晚等约 5.5h
  （实际影响＝零：226–231 批与首窗均已在官方低谷内跑）。修正后窗口判定＝**官方高峰 ∪ 晨间
  缓冲（08:00–09:00，用户令保留）不放新题，其余全放**；周末/假日按工作日近似（假日不单列＝
  保守方向：少跑不超付）。`gate_reason` 与等待函数同步改用官方窗口判定；10 例边界自测全过
  （含周五双峰、午间低谷、周末全程、08:30 缓冲）。
- **第二窗口接续（18:37 起）**：p01/p02 识别终态跳过 → **p03 code_search 全新复跑起跑**
  （§3B 处置生效）；余额 ¥220.56；官方低谷连续面＝18:00 → 次日 09:00（用户缓冲 08:00 止），
  本窗可跑时长 ≈13h。**零 orz/父仓源码不变；计数不变 56。**

### §3D 第二窗口收官补记（2026-10-09 08:42:21 收官；夜窗连跑）

- **本窗完成 9 题 / 43 档 / 零 errored 档**（p03 复跑＋p06–p13）；停止原因＝官方高峰/晨间缓冲
  停放新题门（p13 07:15 起跑、08:42 收官＝在途题正常收尾；门行为在证）。
- **逐题**：p03 code_search 复跑 **0.975**（5 档）／p06 datagate **0.964**（7）／
  p07 dynamic_buffer **0.922**（4，〔1.0, 0.8, 0.89, 1.0〕非单调）／
  p08 dynamic_config_service_api **0.908**（4，后段缓降）／p09 env_manager **0.650**
  （5，波动）／p10 etl_pipeline **1.000**（5 全满）／p11 eve_industry **1.000**（6 全满）／
  **p12 eve_jump_planner 1.000（3 全满）＝0cr 全坍缩题〔0/3〕在重置形态下满分——本 A/B 最强对照样本**／
  p13 eve_market_tools **0.575**（4，〔0.5, 0.8, **0**, 1.0〕末档回满非单调；0cr 同题 0/4 全坍缩 →
  0cw 1/4）。**0cr 三全坍缩题已面二**：eve_jump_planner 0/3→3/3、eve_market_tools 0/4→1/4
  （重置形态未坍）；dag_execution 待遇留 p04 已坍（0/3 同形）。
- **花销**：¥220.56 → **¥166.41**（本窗 ¥54.15 ≈ $7.7）。
- **累计（13/36 题，65/196 档＝33%，满分档 45/65）**：均 ≈¥0.85/档；余 23 题 ≈138 档外推 ≈¥118
  < 余额，充足。
- **窗口调度（自动续窗态）**：起跑器重入挂等——下一官方低谷＝**午间 12:00–14:00**（官方规则内
  低谷），随后 14:00–18:00 高峰挂等，**18:00 起连夜窗至 10-10 08:00**；周六（10-10）起全天低谷
  （周末规则）。S3_PAUSE／余额守卫照常在岗。

### §3E 第三/四窗口与两次代理断窗事故补记（2026-10-09 18:00 → 10-10 13:16）

- **第三窗（10-09 18:00 → 10-10 08:28）**：+11 题/61 档零 errored（p16–p26；含五题全满分
  p19/p20/p22/p24/p25）；**p23 meshctl 八档全零＝最强连续性对照样本**（0cr 7/8；已验证真实成绩
  ——8 档各 50–115 轮 end_turn、verifier 正常执行 core 3 收 0 过）；花销 ¥65.83。
- **午间窗（10-10 12:00–14:23）**：+2 题/9 档（p14 0.667〔0cr 3/3 满分→0cw 失末档，第二反向
  样本〕、p15 execution_server 1.000）。
- **周六窗（10-10 09:05 → 13:16）**：p27 mvvault **1.000**（+2）／p28 pwd_manager 0.705／
  p29 recli 0.325（8 档；**hub 上传赶上断窗失败**，待补传 `harbor upload … --public --yes`）。
- **两次代理断窗事故（同一根因）**：本机 clash-verge/mihomo 代理（127.0.0.1:7890，Docker 出网
  与 harbor 认证交换依赖）两次上游失效——第一次 11:41–11:48（端口死），第二次 13:12–13:1x
  （端口活、上游死：经代理 TLS 全 EOF）。**p29–p36 共 15 题次秒败**（rc=1、零 trial 数据、零
  费用）；DeepSeek 余额分文未动（余额监控与 api.deepseek.com 直连可达）。处置＝15 个空 job 目录
  移名保留证据（`-erred-netfail-20261010`/-b）＋台账 15 条勘误行＋**起跑器增出网预检门**
  （`egress_ok`：代理端口＋supabase/hub 双探活，不过则挂等 5 分钟粒度重探、恢复自动放题；
  预检取代盲目放题）＋余额监控改强制直连（`ProxyHandler({})`）。**上游节点恢复待用户侧**
  （clash-verge 切节点/重启）；恢复后起跑器自动续跑 p29 补传＋p30–p36 七题。
- **累计（28/36 题、146/196 档〔74%〕、满分档 106/146）**；余额 ¥74.93（直连读数）。

## §4 边界与登记

- 本批零父仓/orz 源码；`run_0cw_s3.py` 与台账/日志全在评测侧（`D:/tb-eval/scbench/`），不入仓；
  公开面＝每题 harbor job `--upload --public`（预检轮同形）。
- 窗口收官读数（本窗完成题数/档数/花销对账）随收官补记本档 §3；全目读数、0cr 对拍、
  榜单条目核证归 **S4 收取批**。
- 使用面如实记：journal 无 usage 轴（228 定案 null），花销轴唯一实源＝DeepSeek 余额差分
  （215 批控制台实价核证先例）。

## §5 台账

- 本档：`docs/audits/232_0CW_S3_LAUNCH_RUNNER_2026-10-08.md`。
- BACKLOG：本批记录指针＋`0cw` 专节 S3 注记（成本门放行＋08:00 门＋逐题 job 形态）；计数不变（56）。
- TODO：计数行指针＋P1 路由行＋`P1-0cw` S3 勾选行「在跑」注记。
- BACKLOG 第二卷：§1.178。
- 索引：头行 v4.204 → **v4.205**；§6 `0cw` 条目 S3 注记；§8 pending 桶。
- 门禁 `scripts/check_repository.py` ⇒ 落账后重跑。

## §6 关联与关键词

[`230 批档 §7`](230_0CW_S2_ADAPTER_IMPL_SMOKE_2026-10-08.md)（预检与数据集钉值）／
[`227 批档`](227_0CW_S3_SCALE_ADJUDICATION_2026-10-07.md)（规模裁决＝实时花销监控＋暂停权）／
[`228 批档`](228_0CW_S1_OFFICIAL_FORM_ADJUDICATION_2026-10-07.md)（官方形态勘定）／
[`226 批档`](226_0CW_RESET_FORM_HARBOR_ROUND_REGISTRATION_2026-10-07.md)（母批）／
[`215 批档`](215_SCB_COST_BREAKDOWN_2026-10-07.md)（0cr 花销核证先例）。

关键词：232 批、0cw S3 起跑、成本门放行、08:00 停放新题门、计费高峰、每 checkpoint 收结果、
逐题单 job 串行、起跑器 run_0cw_s3.py、余额守卫、S3_PAUSE 暂停权、断点续跑、起始余额 243.98、
cfgpipe step 1/6、计数 56 不变、索引 v4.205。
