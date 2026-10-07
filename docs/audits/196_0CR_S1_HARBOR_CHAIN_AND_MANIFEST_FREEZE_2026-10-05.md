# 196 批：0cr S1 勘定与排期——Harbor 上传链＋题集 manifest 冻结＋分日排期（2026-10-05）

> **日期**：2026-10-05；**用户令**：「请开始进行0cr S1吧」。
> **性质**：勘定批——零源码、零跑批、零子仓改动；未提交；计数不变 **60**。
> **范围**＝0cr S1 三件事（BACKLOG 批序）：① Harbor 上传链勘定（org/凭证/上传路径＋与 0ci
> 同管线关系）；② 题集 manifest 冻结（36 题字母序＋参考解缺陷题脚注）；③ 分日排期。
> **结论先行**：三件事全部勘定完毕——Harbor 链全链可运行（dry-run 实证）、manifest 36 题
> ／196 checkpoint 与 158 批口径逐位吻合、排期 6 跑批日＋1 收口日＋2 缓冲日（纯跑批墙钟
> 预估 25–40h，落在立项时 25–45h 预注册带内）。

---

## §1 Harbor 上传链勘定

### 1.1 链路全形（官方 `scb_to_harbor.py` 管线，零改装）

| 环节 | 命令／路径 | 勘定结果 |
|---|---|---|
| 转换 | `cd D:/tb-eval/scbench/scb-problems && uv run scripts/scb_to_harbor.py --org silverwhite --all --validate-with-oracle` | dry-run 实证通过（cfgpipe 单题；plan JSON 正常产出、零写入）；build 冒烟默认开（`docker build` 每题环境镜像）、oracle 验证经 `harbor run --config`（oracle agent 逐步 `strict_pass_rate=1.0` 判据） |
| 产物 | `harbor-tasks/silverwhite/<problem>/`（`task.toml`＋`environment/`＋`tests/`＋`steps/`，org 级另落 `metric.py`） | 转换器源码实读确认（`converter.py:47`） |
| 发布 | `harbor publish harbor-tasks/silverwhite/<题>… --public`（36 路径一次或分批） | sibling harbor 0.23.0 `publish --help` 接口在位核证 |
| 凭证 | `harbor auth status` ⇒ **Logged in as SilverWhite**（API key `sk-harbor-T8IS…`） | rc 0，无需重登；TB 2.1 官方轮（142 批 89 题）同凭证同 Hub |
| overrides | 题库仓 `conversion.toml`（eve_market_tools 内存帽 ulimit＋mocked_http verifier 1800s＋python-services 预设） | dry-run 实证加载（`Using overrides: …conversion.toml`） |

- **org 定案＝`silverwhite`**（本方 Hub 命名空间；官方上游数据集 org=`gabeorlanski` 不可写也不冒用）。
  任务名形态=`silverwhite/<problem>`（dry-run plan 实证 `silverwhite/cfgpipe`）。
- **五道参考解缺陷题与 oracle 验证的关系**（`validation.py` 实读）：oracle 验证要求参考解逐步
  `strict_pass_rate=1.0`，缺陷题（file_merger ck2–3／eve_market_tools ck1–3／eve_industry ck5／
  dynamic_buffer ck4／execution_server ck6）**预期 exit 4（OracleValidationError）**——处置＝转换时
  如实登记为预期失败脚注、任务照常发布（oracle=测试非参考解，158 §1.4 口径；官方KNOWN_ISSUES
  同为在案缺陷）。**跑批零依赖参考解，转换/发布与跑批互不阻塞。**
- **harbor 版本钉（勘定决定）**：转换器经 `resolve_harbor_command` 解析 harbor——优先 sibling
  checkout `../harbor`，否则 `uvx harbor`（浮动 latest）。勘定时 `uvx harbor`=**0.23.0** 而 TB 2.1
  venv 旧件=0.20.0，存在窗口内漂移风险 ⇒ **已建 sibling checkout `D:/tb-eval/scbench/harbor`
  ＝v0.23.0（`1e5c5c6`，浅克隆，仓外可逆）**，复验 dry-run 已切换为
  `uv run --directory …harbor harbor`（确定性解析）；venv 0.20.0 原样不动（TB 2.1／0ci 血统面）。

### 1.2 与 0ci（Frontier-Bench）的同管线接入关系

| 面 | 0cr（本线） | 0ci／TB 2.1 血统 |
|---|---|---|
| Hub 账号与凭证 | **同**——SilverWhite API key，`harbor auth status` 一处核验 | 同左 |
| 注册表 | 同一 registry（registry.harborframework.com） | 同左 |
| 上传形态 | **任务/数据集发布**（`scb_to_harbor.py` 转换 → `harbor publish --public`） | **作业结果上传**（`harbor run … --upload --public`，`run_official_2.1.sh` 先例） |
| harbor 版本 | sibling checkout **0.23.0**（本线钉） | venv **0.20.0**（`D:/tb-eval/venv/Scripts/harbor.exe`，不动） |
| 适配器 | harness 原生 `orz` agent（rig 同管线，158 §2） | `tb_agents/orz.py`（TB 系） |

结论：两线**共享凭证与注册表、分属两条上传家族**（publish vs run-upload），互不干扰；
0ci S0 钉仓前无共同 manifest，接入关系仅到「同 Hub 同凭证」层，无进一步耦合。

## §2 冻结面勘定（逐项实核）

| 件 | 值 | 核证方式 |
|---|---|---|
| 题库 pin | scb-problems `38d627e`（=158 批勘定值 `38d627ec…`） | `git log` 实读，工作树净 |
| harness pin | slop-code-bench `31ceea3`（=158 批勘定值 `31ceea3add…`） | `git log` 实读 |
| harness 本地改动（rig 面，随冻结） | 7 文件 +29/−3（deepseek provider env 注入／OrzAgent 注册／Windows workdir as_posix／resolve_host_user 非 posix 臂等）＋未跟踪 `agents/orz/`＋`configs/agents/orz.yaml`＋`configs/models/deepseek-v4-flash.yaml` | `git status`／`git diff` 实读——即 0cj S2 适配器批产物，四跑＋sith 一跑在役形态 |
| 载体 | **0.8.14**，`D:/tb-eval/orz-linux/orz` sha256 `fc990a8a…`（115,593,208 B）＝身份门逐位一致；Windows 件 `--build-info`=0.8.14 | `sha256sum`＋`--build-info` 实测 |
| 模型 | `deepseek/deepseek-v4-flash`（rig aliases 在案），key=`ORZ_DEEPSEEK_API_KEY`（`D:/tb-eval/.env` 1 行在案） | 配置实读 |
| 官方参数 | seed **42**（`cli.py` Option 默认）／pass_policy **any**（`run_config.py` 默认）／one_shot **off**（`OneShotConfig.enabled=False`） | 源码逐处实读——0cr 口径「默认值」三条全部对上 |
| 基镜像 | `slop-code:python3.12` 6.03GB＋`uv:python3.12-trixie-slim` 271MB 在位 | `docker images` 实读 |
| manifest 工件 | `D:/tb-eval/scbench/0cr_official/manifest.json`（机器可读冻结副本；权威=本档 §3 表） | 本批生成 |

## §3 题集 manifest 冻结（官方发现序＝`sorted by name`；k=1）

36 题／**196 checkpoint**（与 158 批 §1.2 核证数逐位一致）；难度 Easy 12／Medium 12／Hard 12。
`◆`＝参考解缺陷题（oracle=测试非参考解，照跑照报加脚注；缺陷 checkpoint 见括号）。

| # | 题 | 难度 | ckpt | | # | 题 | 难度 | ckpt |
|--:|---|---|---:|---|--:|---|---|---:|
| 1 | cfgpipe | Easy | 6 | | 19 | forge | Easy | 8 |
| 2 | circuit_eval | Medium | 8 | | 20 | l2m | Easy | 5 |
| 3 | code_search | Easy | 5 | | 21 | layered_config_synthesizer | Medium | 4 |
| 4 | dag_execution | Hard | 3 | | 22 | log_query | Medium | 5 |
| 5 | database_migration | Medium | 5 | | 23 | meshctl | Hard | 8 |
| 6 | datagate | Easy | 7 | | 24 | metric_transform_lang | Hard | 5 |
| 7 | dynamic_buffer ◆(ck4) | Hard | 4 | | 25 | migrate_configs | Easy | 5 |
| 8 | dynamic_config_service_api | Medium | 4 | | 26 | mocked_http | Hard | 8 |
| 9 | env_manager | Easy | 5 | | 27 | mvvault | Medium | 6 |
| 10 | etl_pipeline | Easy | 5 | | 28 | pwd_manager | Medium | 5 |
| 11 | eve_industry ◆(ck5) | Hard | 6 | | 29 | recli | Hard | 8 |
| 12 | eve_jump_planner | Medium | 3 | | 30 | rejector | Hard | 5 |
| 13 | eve_market_tools ◆(ck1–3) | Hard | 4 | | 31 | sheeteval | Hard | 7 |
| 14 | eve_route_planner | Medium | 3 | | 32 | sith | Hard | 6 |
| 15 | execution_server ◆(ck6) | Easy | 6 | | 33 | test_translator | Hard | 8 |
| 16 | file_backup | Easy | 4 | | 34 | textdrop | Easy | 6 |
| 17 | file_merger ◆(ck2–3) | Medium | 4 | | 35 | trajectory_api | Medium | 5 |
| 18 | file_query_tool | Medium | 5 | | 36 | xjq | Easy | 5 |

- 顺序权威＝转换器 `--all` 的 `sorted(available_problems)`（`cli.py:151`）＝本表 Python 字典序，
  与「官方发现序（scb-problems 字母序）」同一定义；**逐题序号即跑批序**。
- recli 已有四跑先例（188 批 8/8 满分为其中之一）、sith 一跑（2/6）——0cr 官方轮**照跑不豁免**
  （全目口径零预期；历史读数只作方差注记，不进记分卡）。

## §4 跑批执行形态勘定（S2 循环定形）

- **发射**（与 188 批逐字同形，rig 目录内）：
  `cd D:/tb-eval/scbench/slop-code-bench && set -a && source D:/tb-eval/.env && set +a && uv run slop-code run --agent orz --model deepseek/deepseek-v4-flash --problem <题>`；
  日志落 `D:/tb-eval/scbench/0cr_official/logs/<题>_run1_0cr.log`。
- **单题单跑语义**：一次 `slop-code run`＝一个全新容器＝该题全部 checkpoint 的单一 ACP 会话
  （官方 SCBench 长程语义，158 §2.1/§2.2）；题间全新容器由 rig 天然保证（每 run 新建随机
  temp 挂载）。checkpoint 客户端预算 3600s（orz.yaml `timeout`，在役形态不动）。
- **每题收一轮**（0cr 口径②）：①官方读数——`outputs/deepseek-v4-flash/orz_just-solve_none_<ts>/`
  逐 checkpoint `evaluation.json`（core／全测试／solved／erosion）；②摩擦台账——journal
  （`.gsa/runs/RUN-*/events.jsonl` 随工件拷出）按 **181 §4b 口径**验尸（写控/权限/infra/压缩/RLI/
  工具错误逐项）；③归档后起下一题。
- **发射前置门（每题）**：D 盘余量（本批勘定时 **15G**——188 时 23G，收窄面如实记；阈值＝
  <10G 即当日清理后再跑）／Docker 在役／载体身份门（`fc990a8a…` 抽核）／key 在案。
- **同轮并收（已登记的两项读数义务）**：**0cs S3 真机核证随 D1 首题（cfgpipe）顺带**（191 批
  排期条款兑现）；**0bz S4 离线对账随轮内 journal 累积分批收取**（192 批离线形态，S3 收口前
  清账）——两项均零额外跑批。
- **失败语义（0cr 口径冻结面照抄）**：装置性失败（harness exit ≠0／装置故障）可重跑该题；
  模型性失败不重跑；轮中修复只登记不顺延进轮（单一快照可比性）。

## §5 分日排期（各步独立放行；日期为默认锚，顺延不追）

**墙钟依据（实测锚）**：recli 8ckpt＝53–125min（三跑）／sith 6ckpt＝44min；均值 ≈6–15 min/ckpt；
36 题均值 5.44 ckpt/题 ⇒ 单题 ≈0.5–1.5h。**纯跑批墙钟预估 25–40h**（立项预注册带 25–45h 内）；
加每题收取与验尸 ≈15–30min，**6 题/日 × 6 日**为常态承载（每日 3–6h 墙钟＋收取间歇）。

| 日 | 日期 | 题（序号连续、字母序不可调换） | ckpt 小计 |
|---|---|---|---:|
| D1 | 2026-10-06 | ①–⑥ cfgpipe · circuit_eval · code_search · dag_execution · database_migration · datagate（首题顺带 0cs S3） | 34 |
| D2 | 2026-10-07 | ⑦–⑫ dynamic_buffer · dynamic_config_service_api · env_manager · etl_pipeline · eve_industry · eve_jump_planner | 27 |
| D3 | 2026-10-08 | ⑬–⑱ eve_market_tools · eve_route_planner · execution_server · file_backup · file_merger · file_query_tool | 26 |
| D4 | 2026-10-09 | ⑲–㉔ forge · l2m · layered_config_synthesizer · log_query · meshctl · metric_transform_lang | 35 |
| D5 | 2026-10-10 | ㉕–㉚ migrate_configs · mocked_http · mvvault · pwd_manager · recli · rejector | 37 |
| D6 | 2026-10-11 | ㉛–㉞㉟㊱ sheeteval · sith · test_translator · textdrop · trajectory_api · xjq | 37 |
| S3 | 2026-10-12 | 收口＝Harbor 转换全目（build 冒烟＋oracle；五缺陷题预期 exit 4 脚注）→ `harbor publish --public` → 官方记分卡＋摩擦汇总＋报告档（142 批报告同形态＋方差注记） | — |
| 缓冲 | 2026-10-13/14 | 装置性失败重跑＋溢出顺延 | — |

- D2/D3 天然偏轻（27/26 ckpt）＝内置溢出吸收位；D4–D6 偏重（35–37）如溢出顺延缓冲日。
- **磁盘纪律**：转换的 build 冒烟会新增 36+ 环境镜像（量级 10–30GB）——转换排在 D7 收口窗、
  跑批六日不并行转换（防 docker 争用与满盘；159 批满盘教训条款兑现）。

## §6 台账

- 本档：`docs/audits/196_0CR_S1_HARBOR_CHAIN_AND_MANIFEST_FREEZE_2026-10-05.md`。
- TODO：`P1-0cr` S1 勾选＋达成注记；头部计数行指针（计数不变 60）；P1 路由行 0cr 状态同步。
- BACKLOG：本批记录指针＋计数行（60 不变）＋P1 总览行＋开放项锚点行＋`0cr` 专节批序 S1 达成注记；第二卷 §1.143。
- 索引：头行 v4.167 → **v4.168**。
- 仓外工件：`0cr_official/manifest.json`＋`logs/` 目录；sibling checkout `D:/tb-eval/scbench/harbor`（v0.23.0 浅克隆）。
- 门禁 `check_repository.py` ⇒ **`valid: true`／error_count 0**（一次回缠：TODO P1 路由行首版 1213>1200 字符帽，本批 0cr 段压缩后复跑全绿——细节留在 P1-0cr 节与批档，路由行只留指针，沿 152 批方法论）。

## §7 边界

1. **零源码**：orz 子仓零改动；父仓仅本档与台账；harness／题库两上游仓零改动（rig 本地改动为
   0cj S2 既成事实，本批只登记冻结不改写）。
2. **未跑批**：S2 首题起跑待用户放行（各步独立放行口径）；本批仅 dry-run（零写入）验证链路。
3. **sibling checkout 为仓外可逆件**：不影响 venv 0.20.0 血统面；若上游 harbor 出 ≥0.24 需重钉时
   重走本节勘定（不静默浮动）。
4. **Harbor 发布时机在 S3**（转换＋验证＋发布一窗）；跑批不依赖发布，两轨解耦。
5. 排期日期为默认锚非承诺；用户逐日放行，顺延不追。

## §8 关键词

196 批、0cr S1、Harbor 上传链勘定、scb_to_harbor、silverwhite、harbor publish、sibling checkout
v0.23.0、uvx 浮动风险、五缺陷题 exit 4 脚注、36 题 196 checkpoint、manifest 冻结、官方参数
seed42／any／one_shot off、载体 0.8.14 身份门 fc990a8a、6+1+2 排期、25–40h、0cs S3 顺带、
0bz S4 同轮收取、计数 60 不变。
