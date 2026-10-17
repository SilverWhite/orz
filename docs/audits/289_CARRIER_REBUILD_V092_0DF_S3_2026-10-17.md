# 289 批：0.9.2 双平台载体重建进体——0df S3（2026-10-17）

> **用户令**：「已完成清理，请进行重建吧」（未指名版本，沿 218/243/280 批先例取补丁位
> 递增 0.9.1→0.9.2）。**本批**＝0.9.2 代窗口双平台载体重建（沿 247/280 批形态：源冻结
> →bump→双平台重建换装→进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。
> **0df S3 在本批达成**（287 计量行＋286 检索门缺省开＋288 处置三码同车进体）；**S4 真
> 机读数待续**。**进件面**＝`c678c21b`（289 bump）＋`7966d171`（288 处置）＋`b94f9fb7`
> （286 缺省开）＋`6e107c77`（287 落码）。**源冻结**＝orz **`c678c21b`**（Cargo.toml＋
> Cargo.lock 恰两行、`cargo metadata --locked` exit 0）；未推送。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结＋bump | `c678c21b`（恰两行 0.9.1→0.9.2；locked exit 0）；288 处置提交 `7966d171`（4 文件 +199/−11） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（3m28s 暖缓存）**；换装 **MATCH 3/3**、`.0.9.1-bak` 链预建（前件与 280 账面 `39f6f3fa…` 逐位吻合）；`--build-info`＝`0.9.2 os=windows` rc0；载体清单 3 entries 刷新（v0.9.2） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261017-289`；provision **exit 0**（正斜杠沿 218 教训）；`binary_sha256=fc374dca…`↔换装位 `orz-signer.exe` 逐位一致；keystore `installation-key.json` `2aa80cb8…` 逐位未动 |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0（约 40 分钟冷构建——`orz-target` 缓存随用户磁盘清理被清，镜像亦重拉）**；`MSYS_NO_PATHCONV=1` 一次通过；bak 链宿主预建（前件与 280 账面 `f50b1f17…` 逐位吻合）；**直写换装位**；**static-pie×3**（static-pie linked＋INTERP=0）；alpine 3.20/bookworm 双冒烟 `0.9.2 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新（v0.9.2） |
| 进体字节判据 | **0df 新面全数进体**：`run_usage_metered` 0→2／`usage_metering` 0→5／`run-v1` 0→5（合并常量 `usage_meteringrun-v1` ×5 双侧同值）／`usage_incomplete` 0→1；`effort_tier`／`model_calls` 以 **x64 立即数拆分形**在体（`mov rax,'model_ca'`＋`mov [rax+7],'alls'` 等价对双侧各 ×1——连续字节法盲区，冒烟 journal 九字段全值反证）；286 面 `ORZ_RETRIEVAL_ENABLED` 2→3；版本面 `0.9.2` 进体、`0.9.1` 残留 18→17／26→25（基线含载体版本字面 1 条替换后仅剩依赖子串——280 批 `0.9.0` 同形，非回归）；**280 窗保留面逐项持平零回归**（全表见 §2）；288 面 `journal_line_written` LIN 0→2（未剥离符号面）／WIN 0→0（dead-code 剪除面，如实记）、法官降级文案双侧 0→0（载体不内嵌执法面，执法在 tooling 侧——预判如实记） |
| 冒烟 | WIN scratch（`D:/tb-eval/smoke-289`）两轮 `--fake-provider -p` 均整轮 rc=0；**0df 活体证据（归档 journal 逐事件核证）**：round1 `RUN-CLI-6ad31716`——seq2 `tool_availability_check{retrieval_enabled:true}`〔286 缺省开活体〕＋tools 注册含 `web_search`＋seq3 `run_started` 双锚（`failure_pipeline:"funnel-v1"`＋`usage_metering:"run-v1"`）＋seq9 `run_usage_metered` 九字段全值（`model_calls:1`＋五项诚实 0＋`effort_tier:"unknown"`＋`usage_incomplete:true`）恰终态前一条＋哈希链连续；round2 `RUN-CLI-6ad3171b` 同构（journal 在盘核证同 OK）；**0cy 启动侧扫描活体**：首会话收卷、源目录删除、`archives/6ad31716.json.gz` 在案 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `f50b1f17…`→**`45ea8b60…`**（注释同步重写；adapter 未动；py_compile 过＋`--help` rc0） |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（187 批在案先存，与 247/280 同源） |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.9.1→0.9.2） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 58,395,648（+26,624） | `0e8ee3b73d774879…` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `fc374dcaa3e8f3ce…`（provision 新值） |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `216e80c89600d2a7…` |
| LIN | `orz` | 117,219,656（+27,344） | **`45ea8b604a8bfccb…`（身份门新值）** |
| LIN | `orz-signer` | 1,397,744（+248） | `7845a154bb4c2878…` |
| LIN | `orz-acaf-provision` | 1,216,208（+256） | `85663300817f7977…` |

（换装基线＝0.9.1 三件 `39f6f3fa…`/`dd55ded5…`/`34563cad…`（WIN）与 `f50b1f17…`/`a57a4d29…`/`46a7b6d0…`（LIN），与 280 批账面逐位吻合。）

## §2 进体字节判据（方法与读数）

**方法**：python 字节直数（`bytes.count`；243/247/280 批同法），基线＝`.0.9.1-bak` 换装位
预建件；脚本 `D:/tb-eval/byte_criteria_289.py`（评测区本地面，不入仓）。拆分形核证＝
`H\xb9model_ca`/`H\xb9effort_t` 立即数对计数（双侧各 1）＋合并常量 `usage_meteringrun-v1`
×5（双侧同值）。

| 判据 | 基线（0.9.1）WIN/LIN | 0.9.2 WIN | 0.9.2 LIN |
|---|---|---|---|
| 0df：`run_usage_metered` | 0／0 | **2** | **2** |
| 0df：`usage_metering` | 0／0 | **5** | **5** |
| 0df：`run-v1` | 0／0 | **5** | **5** |
| 0df：`usage_incomplete` | 0／0 | **1** | **1** |
| 0df：`effort_tier`／`model_calls`（连续形） | 0／0 | 0（拆分形在体） | 0（拆分形在体） |
| 288：`journal_line_written` | 0／0 | 0 | **2**（符号未剥离面） |
| 288：法官降级区分文案 | 0／0 | 0 | 0（载体不内嵌执法面） |
| 286：`ORZ_RETRIEVAL_ENABLED` | 2／2 | **3** | **3** |
| 286：`ORZ_RETRIEVAL_MODE`／`retrieval_not_enabled` | 2/12 同 | 3／12 | 3／12 |
| 版本面：`0.9.2` | 0／1 | **1** | **2** |
| 版本残留：`0.9.1`（依赖子串） | 18／26 | 17 | 25 |
| 0de 保留：`reminder_only_no_compressible`／`stop_declared` | 7/0 同 | 7／0 | 7／0 |
| 247 保留：`clear_all`／`context_manage`／`keep_recent_rounds`／`handover` | 27/3/13/38 同 | 27／3／13／38 | 27／3／13／38 |
| 247 保留：清零 marker／一键清理／mode=clear／op=clear | 2/8/23/42 同 | 2／8／23／42 | 2／8／23／42 |
| 247 保留：`清零`／`清空` | 45/64 同 | 45／64 | 45／64 |
| 218 保留：`--target-directory`／`若意图是备份/暂存`／`did you mean "`／`前推(` | 1/1/2/1 同 | 1／1／2／1 | 1／1／2／1 |
| 负例：`cursor_agent`／`deepseek-v4-flash` | 0／0 | 0／0 | 0／0 |
| 增长面：`findings` | 96／67 同 | 96 | 67 |
| 0cy 保留：`ORZ_GSA_ARCHIVE_ROOT`／`ORZ_GSA_HYGIENE`／`session-archive-package-v0.3`／`gsa hygiene sweep` | 2/1/1/1 同 | 2／1／1／1 | 2／1／1／1 |
| 0dd 保留：`incremental` | 14／25 同 | 14 | 25 |

**判读**：0df S3 字节判据（事件族字面＋锚＋payload 键）**全数进体**（`effort_tier`/
`model_calls` 为 x64 立即数拆分存储，连续字节法盲区——冒烟 journal 九字段全值反证在
体）；288 幂等旗为 Rust 字段名（无 serde 面），LIN 未剥离符号面可见 ×2、WIN 剪除，
如实记；载体不内嵌 `usage_metering` 执法面（families 判官在 tooling/测试侧消费）＝预
判成立；280 窗保留面**逐项持平零回归**。

## §3 冒烟与活体核证

- WIN scratch 两轮 rc=0（round1 `RUN-CLI-6ad31716`／round2 `RUN-CLI-6ad3171b`）。
- **0df 活体（归档＋在盘 journal 双证）**：两 run 均＝双锚 `run_started`（`failure_pipeline`
  ＋`usage_metering:"run-v1"`）＋`run_usage_metered` 九字段恰一条（seq9）在 `run_finished`
  （seq10）前＋哈希链连续；`model_calls=1` 与单轮恰等；fake 无 usage ⇒ 五项 0＋
  `usage_incomplete:true`（诚实不完整）；`effort_tier:"unknown"`（fake 无配置暴露）。
- **286 活体**：seq2 `tool_availability_check{retrieval_enabled:true}`＋`request_header_change`
  tools 注册含 `web_search`（缺省开）。
- **0cy 活体**：round2 启动侧收卷 round1 会话（源目录删除、`archives/6ad31716.json.gz`
  在案、footprint.runs 含全量 events.jsonl）。

## §4 台账

- 本档：`docs/audits/289_CARRIER_REBUILD_V092_0DF_S3_2026-10-17.md`。
- BACKLOG：0df P1 表行 S3 勾＋指针行（本批＝289、前批链插 288）＋计数行＋开放项段
  S3 达成态＋289 批档链接。
- BACKLOG 第二卷：§1.228（288）／§1.229（289）。
- TODO：计数行（本批＝289）＋P1 路由 0df 片段 S3 勾。
- 索引：头行 v4.261 → **v4.262**。
- orz 子仓：`c678c21b` bump 批提交（**不推送**；`7966d171`/`b94f9fb7`/`6e107c77` 同在本地）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装（注释同步重写）。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json`
  刷新 0.9.2（`generate_carrier_manifest.py`，各 3 entries）。
- 字节判据脚本：`D:/tb-eval/byte_criteria_289.py`（评测区本地面，不入仓）。

## §5 边界与如实记

1. **未推送未发行**：orz 新提交（`7966d171`/`c678c21b`）与父仓各批均在本地；不建
   Release、不做 rel-stage 打包；发布面停 v0.9.1。**装机位（`C:\Users\1\AppData\Local\orz\bin`）
   未换装**——沿 281 式「换装＋发行」另批口径；本批只换评测载体位（`D:/tb-eval/orz-{windows,linux}`）。
2. **LIN 为冷构建**（`/target` 缓存随用户磁盘清理被清；rust:1.97-slim/alpine:3.20/
   bookworm-slim 镜像重拉）：约 40 分钟 `-j 1`；WIN 为暖缓存增量（3m28s）——非同口径
   墙钟，如实记。
3. **冒烟落点＝scratch 目录**（`D:/tb-eval/smoke-289`）：0cy 启动侧收卷只在 scratch 内活体
   触发，不触碰 D:\CLI 工作区会话。
4. **版本号取值**：用户令未指名版本，沿 218/243/280 批补丁位递增先例取 0.9.2。
5. **S4 待续**：0df S4 真机读数＝真机轮 run journal 终态前恰一行＋五项与 provider 账面
   可对＋检索子代理轮折入同行（`model_calls` ≥ 主车道轮数）＋transport 重试低估口径
   以 `transport_retry` 校准（287 §5 判据＋288 §5 降级文案面）。
6. **`journal_line_written` 双侧计数不对称**（WIN 0／LIN 2）：Debug derive 字段名的链接
   剪除差异，非功能面；法官降级文案双侧 0＝载体不内嵌执法面，预判成立。

## §6 关键词

289 批、0.9.2 重建、源冻结 `c678c21b`、0df S3 进体、`45ea8b60…` 身份门、ACAF 重
provision `fc374dca…`、static-pie×3、alpine/bookworm 双冒烟 0.9.2、`run_usage_metered`
进体、`usage_meteringrun-v1` 合并常量、立即数拆分形、286 缺省开活体、journal 九字段
活体、保留面零回归、RUN-CLI-6ad31716/6ad3171b、未推送未发行、计数 62 不变、v4.262。
