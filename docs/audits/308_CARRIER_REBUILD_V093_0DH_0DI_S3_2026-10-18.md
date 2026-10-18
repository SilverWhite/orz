# 308 批：0.9.3 双平台载体重建进体——0dh＋0di S3 同车（2026-10-18）

> **用户令**：「请进行重建吧」。**本批**＝0dh＋0di S3 同车＝0.9.3 代窗口双平台载体重建
>（沿 247/280/289 批形态：源冻结→双平台重建换装→进体字节判据→ACAF 重 provision→
> 身份门换装→冒烟→落账）。**版本号已随 303 批预 bump**（`45310a09`，0.9.2→0.9.3），
> 本批零新 orz 提交。**源冻结**＝orz **`5fd4c42d`**（305 批处置后；`cargo metadata
> --locked` exit 0 复验；工作树干净）。**S3 达成**（0dh 长程进程显式通道＋0di 消息
> 撤回＋中途补充同车进体）；**两线 S4 真机核证待续**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（8m12s 暖缓存）**；换装 **MATCH 3/3**、`.0.9.2-bak` 链预建（前件与 289 账面 `0e8ee3b7…`/`fc374dca…`/`216e80c8…` 逐位吻合）；`--build-info`＝`0.9.3 os=windows` rc0；清单刷新（v0.9.3，3 entries） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261018-308`；provision **exit 0**（正斜杠沿 218 教训）；`binary_sha256=b597cbb0…`↔换装位 `orz-signer.exe` 逐位一致；keystore 两件 `2aa80cb8…`/`f37556ab…`（`acaf/keystore/`）逐位未动 |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0（38m36s——容器内 `/tmp` rg 静态树每次冷装，orz 依赖缓存暖）**；`MSYS_NO_PATHCONV=1` 一次通过；bak 链宿主预建（前件与 289 账面 `45ea8b60…`/`7845a154…`/`85663300…` 逐位吻合）；**直写换装位**；**static-pie×3**（static-pie linked＋not stripped）；alpine 3.20/bookworm 双冒烟 `0.9.3 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新（v0.9.3，3 entries） |
| 进体字节判据 | **0dh 新面全数进体**（双侧）：`hide_background_input` 4→5／`get_task_output` +1／`kill_task` +1／`session_owned` 0→1／`GROK_BG_MAX_RUNTIME_MS` 0→1／`~30h safety cap` 0→4／`new_bytes` WIN 0→3・LIN 1→4；`is_background` 14→14（Grok 上游字面本在树——S2 注入面由 `hide_background_input` +1 承载，如实记）。**0di 新面全数进体**（双侧）：`_orz/queue_append` 0→1／`_orz/recall_preview` 0→2／`_orz/recall` 0→4／`supplement_delivered` 0→4／`recall_log` 0→6；`SupplementQueue` WIN 0（LIN **0→6**）／`drain_user_supplements` WIN 0（LIN **0→1**）＝WIN 剪除面（288 `journal_line_written` 同形）；`SupplementDelivered` 双侧 0＝类型名内联消解，契约面由 serde 名 `supplement_delivered` ×4 承载（冒烟活体反证通道在体）；版本面 `0.9.3` 进体（WIN 7／LIN 8）、`0.9.2` 残留 WIN 1→0・LIN 2→1（载体版本字面替换，280/289 同形非回归）、`0.9.1` 依赖子串持平；**289 窗保留面逐项持平零回归**（全表见 §2） |
| 冒烟 | WIN scratch（`D:/tb-eval/smoke-308`）两轮 `--fake-provider -p` 均整轮 rc=0（`RUN-CLI-6ad4e7cc`／`6ad4e7d8`）；**活体（R2 在盘 journal＋R1 归档包双证）**：seq2 `tool_availability_check{retrieval_enabled:true}`〔286 缺省开〕＋seq3 `run_started` 双锚（`failure_pipeline:"funnel-v1"`＋`usage_metering:"run-v1"`）＋seq9 `run_usage_metered` 九字段全值恰终态前一条＋哈希链连续（11 events）＋**0cy 收卷链**（`ARC-6ad4e7cc-1` 回执＋`archives/6ad4e7cc.json.gz`〔schema `session-archive-package-v0.3`〕＋源目录删除）＋R1 run 尾零主动归档〔0dd 退役面维持〕；stderr `deepseek-flash` 新名在用〔0cv 面〕 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `45ea8b60…`→**`4d7917c3…`**；py_compile 过＋`--help` rc0 |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（287 在案预存）；LIN 8＝平台门控预存 lint（job_object 非窗桩 4＋permission/session/process_tree 非窗面 3＋orz-config 1）——musl 目标 cfg 面，非本批引入 |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.9.2→0.9.3） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 58,533,888（+138,240） | `dabaaf7ead1dc2a1…` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `b597cbb0033a96ef…`（provision 新值） |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `fd7f92d52802401a…` |
| LIN | `orz` | 117,404,232（+184,576） | **`4d7917c335c10b11…`（身份门新值）** |
| LIN | `orz-signer` | 1,398,000（+256） | `00a16266fad8b19b…` |
| LIN | `orz-acaf-provision` | 1,216,448（+240） | `8fc0b65bc2f8afe4…` |

（换装基线＝0.9.2 三件 `0e8ee3b7…`/`fc374dca…`/`216e80c8…`（WIN）与 `45ea8b60…`/`7845a154…`/`85663300…`（LIN），与 289 批账面逐位吻合；`.0.9.2-bak` 双侧六件预建在案。）

## §2 进体字节判据（方法与读数）

**方法**：python 字节直数（`bytes.count`；243/247/280/289 批同法），基线＝`.0.9.2-bak`
换装位预建件；脚本 `D:/tb-eval/byte_criteria_308.py`（评测区本地面，不入仓）。

| 判据 | 基线（0.9.2）WIN/LIN | 0.9.3 WIN | 0.9.3 LIN |
|---|---|---|---|
| 0dh：`hide_background_input` | 4／4 | **5** | **5** |
| 0dh：`get_task_output`／`kill_task` | 15・15→31・117 同 | **16／32** | **16／118** |
| 0dh：`session_owned` | 0／0 | **1** | **1** |
| 0dh：`GROK_BG_MAX_RUNTIME_MS` | 0／0 | **1** | **1** |
| 0dh：`~30h safety cap`（教学字面） | 0／0 | **4** | **4** |
| 0dh：`new_bytes` | 0／1 | **3** | **4** |
| 0dh：`is_background`（上游字面在树） | 14／15 同 | 14 | 15 |
| 0di：`_orz/queue_append`／`_orz/recall_preview`／`_orz/recall` | 0 同 | **1／2／4** | **1／2／4** |
| 0di：`supplement_delivered`（serde 名） | 0／0 | **4** | **4** |
| 0di：`recall_log` | 0／0 | **6** | **6** |
| 0di：`SupplementQueue`（类型名） | 0／0 | 0（剪除） | **6** |
| 0di：`drain_user_supplements`（槽名） | 0／0 | 0（剪除） | **1** |
| 0di：`SupplementDelivered`（类型名） | 0／0 | 0（内联消解） | 0（内联消解） |
| 版本面：`0.9.3` | 0／0 | **7** | **8** |
| 版本残留：`0.9.2`（载体版本字面替换） | 1／2 | 0 | 1 |
| 版本残留：`0.9.1`（依赖子串） | 17／25 同 | 17 | 25 |
| 0df 保留：`run_usage_metered`／`usage_metering`／`run-v1`／`usage_incomplete` | 2/5/5/1 同 | 2／5／5／1 | 2／5／5／1 |
| 0df 保留：`effort_tier`／`model_calls`（连续形） | 0 同 | 0（拆分形在体，冒烟反证） | 0（拆分形在体） |
| 286 保留：`ORZ_RETRIEVAL_ENABLED`／`_MODE`／`retrieval_not_enabled` | 3/3/12 同 | 3／3／12 | 3／3／12 |
| 0de 保留：`reminder_only_no_compressible`／`stop_declared` | 7/0 同 | 7／0 | 7／0 |
| 247 保留：`clear_all`／`context_manage`／`keep_recent_rounds`／`handover` | 27/3/13/38（WIN） | 27／3／13／38 | 28／3／13／27（LIN 平台差同基线） |
| 247 保留：清零 marker／一键清理／mode=clear／op=clear | 2/8/23/42（WIN） | 2／8／23／42 | 2／8／23／20（LIN 同基线） |
| 247 保留：`清零`／`清空` | 45/64（WIN） | 45／64 | 48／31（LIN 同基线） |
| 218 保留：`--target-directory`／`若意图是备份/暂存`／`did you mean "`／`前推(` | 1/1/2/1 同 | 1／1／2／1 | 1／1／2／1 |
| 负例：`cursor_agent`／`deepseek-v4-flash` | 0／0 | 0／0 | 0／0 |
| 增长面：`findings` | 96／67 同 | 96 | 69（LIN 同基线） |
| 0cy 保留：`ORZ_GSA_ARCHIVE_ROOT`／`ORZ_GSA_HYGIENE`／`session-archive-package-v0.3`／`gsa hygiene sweep` | 2/1/1/1 同 | 2／1／1／1 | 2／1／1／1 |
| 0dd 保留：`incremental` | 14／25 | 15 | 26（双侧 +1＝0di 面字面增量，非回归） |

**判读**：0dh S3 字节判据（`is_background` schema 字面＋保留面 +2 名单＋`session_owned`
键＋env 名＋30h 教学字面＋`new_bytes` 契约面）与 0di S3 字节判据（ext 三方法＋事件族
serde 名＋`recall_log`＋`SupplementQueue`/`drain_user_supplements` LIN 未剥离面）**全数
进体**；类型名/槽名 WIN 剪除与 `SupplementDelivered` 内联消解＝连续字节法已知盲区
（288 `journal_line_written` 同形，如实记），契约承载面（serde 名 ×4＋ext 方法字面）
双侧在体；289 窗保留面**逐项持平零回归**；脚本机械旗 `SUSPECT-REGRESSION` 仅由版本
字面替换（0.9.2 WIN 1→0／LIN 2→1）触发＝280/289 批既判非回归形态。

## §3 冒烟与活体核证

- WIN scratch 两轮 rc=0（round1 `RUN-CLI-6ad4e7cc`／round2 `RUN-CLI-6ad4e7d8`）。
- **286 活体**：R2 seq2 `tool_availability_check{probe_scope:retrieval_family,
  retrieval_enabled:true, gate_decision:pass}`（缺省开维持）。
- **0df 活体**：R2 seq3 `run_started` 双锚（`failure_pipeline:"funnel-v1"`＋
  `usage_metering:"run-v1"`）＋seq9 `run_usage_metered` 九字段全值（`model_calls:1`
  与单轮恰等＋fake 无 usage 五项诚实 0＋`usage_incomplete:true`＋
  `effort_tier:"unknown"`）在 `run_finished`（seq10）前恰一条；哈希链连续（11 events）。
- **0cy 活体**：R2 启动侧收卷 R1 会话＝`ARC-6ad4e7cc-1` 回执（`session_archive`
  completed、digest 在案、attempts 1）＋`archives/6ad4e7cc.json.gz`
  （schema `session-archive-package-v0.3`）＋源 run 目录删除；R1 run 尾零主动归档
  〔0dd 退役面维持〕。
- **0cv 面**：stderr `web_search client configured … model:"deepseek-flash"`（新名在用）。
- **0dh/0di 冒烟边界（如实记）**：fake-provider headless 冒烟不产生后台任务与客户端
  ext 调用，两线行为面不可在冒烟活体触发——进体由字节判据承载、行为面由 S4 真机判据
  承载（0dh＝跨 run 过夜训练存活六判据；0di＝R20 增补九判据）。

## §4 台账

- 本档：`docs/audits/308_CARRIER_REBUILD_V093_0DH_0DI_S3_2026-10-18.md`。
- BACKLOG：本批指针＋计数行（60 不变）＋P1 总览行 0dh/0di S3 达成注＋0dh/0di 专节
  S3 达成注。
- TODO：计数行＋P1 路由行 0dh/0di 片段＋`P1-0dh`/`P1-0di` S3 勾。
- BACKLOG 第二卷：§1.247（本批）。
- 索引：头行 v4.279 → **v4.280**。
- orz 子仓：**零新提交**（源冻结 `5fd4c42d`＝303 bump `45310a09`＋305 处置，均已随
  307 批推送）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装（py_compile＋`--help` rc0）。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/
  carrier-manifest.json` 刷新 0.9.3（`generate_carrier_manifest.py`，各 3 entries）。
- 字节判据脚本：`D:/tb-eval/byte_criteria_308.py`（评测区本地面，不入仓）。
- 构建日志：`D:/tb-eval/buildlogs/win_093_build.log`／`lin_093_build.log`；冒烟落点
  `D:/tb-eval/smoke-308`。

## §5 边界与如实记

1. **未推送未发行**：本批零新 orz 提交、父仓落账提交在本地；不建 Release、不做
   rel-stage 打包；发布面停 v0.9.2。**装机位（`C:\Users\1\AppData\Local\orz\bin`）
   未换装**——沿 281 式「换装＋发行」另批口径；本批只换评测载体位
   （`D:/tb-eval/orz-{windows,linux}`）。
2. **墙钟非同口径**：WIN 暖缓存 8m12s；LIN 38m36s（容器内 `/tmp` rg 静态树每次冷装
   ＋`-j 1`；orz 依赖缓存 `/target` 暖）。
3. **Docker 引擎本批冷启动**（Docker Desktop 拉起后构建）；三镜像（rust:1.97-slim／
   alpine:3.20／bookworm-slim）本机在位零重拉；磁盘 D: 28G free（预检读数，90% 用量
   如实记）。
4. **S4 待续（两线）**：0dh S4＝跨 run 过夜训练存活至完成（对照 ep96）＋run 尾 sweep
   后存活＋30h 生效路径回放＋完成提醒跨 run 送达＋增量读出真机读数＋跨 run 读出连续
   性；0di S4＝305/R20 增补九判据（补充不打断且下一模型调用边界生效／撤回一致性与
   重发／不回滚面／双端 ext 可用／并发 fail-closed／失败未写回提示／撤回×排队／菜单
   真实用户消息／Unicode 与满队边界）。
5. **`is_background` 计数持平判读**：该字面系 Grok 上游机制既有在树（0dh D1 勘定），
   S2 注入的是 `hide_background_input:false` 逃生阀（4→5 承载）——schema 可见面在体
   由注入面＋`get_task_output`/`kill_task` +1 联合承载，非单字面计数。
6. **告警面**：WIN 1（permission.rs，预存在案）；LIN 8（musl cfg 门控预存 lint：
   job_object×4／permission×1／session×1／process_tree×1／orz-config×1）——非本批引入，
   不入处置。

## §6 关键词

308 批、0.9.3 重建、源冻结 `5fd4c42d`、0dh S3 进体、0di S3 进体、同车、`4d7917c3…`
身份门、ACAF 重 provision `b597cbb0…`、static-pie×3、alpine/bookworm 双冒烟 0.9.3、
`session_owned` 进体、`supplement_delivered` 进体、`_orz/queue_append` 进体、30h 教学
字面进体、保留面零回归、RUN-CLI-6ad4e7cc/6ad4e7d8、未推送未发行、计数 60 不变、
v4.280。
