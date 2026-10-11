# 247 批：0.9.0 双平台载体重建进体——0cy S3（2026-10-11）

> **用户令**：「请进行重建吧，本次版本号为0.9.0」。
> **本批**＝0.9.0 代窗口双平台载体重建（沿 243/218/194 批形态：源冻结→bump→双平台重建换装
> →进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**进件面**＝`af338387`
> （246 批落码＝0cy S2 GSA 会话卫生在役〔启动侧扫描收卷＋包 v0.3 带足迹＋unarchive 按需恢复
> ＋delete 扩面＋双 env 口径〕＋摩擦④ permission 护栏收口〔桥臂＋样本两表同批补登
> `context_manage`，基线红 362/1 → 363/0 转绿〕）。
> **源冻结**＝orz **`a942d6f3`**（`af338387`＋bump 0.8.16→0.9.0；Cargo.toml＋Cargo.lock 恰两行、
> `cargo metadata --locked` exit 0）；未推送。
> **零源码语义增量**；**计数不变（59）**；**未推送、未发行**（发布面停 v0.8.16〔244 批〕）。
> **0cy S3 在本批达成**；0cz S3/0da S3 已随 243 在役不动。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结＋bump | `a942d6f3`（恰两行 0.8.16→0.9.0；locked exit 0）；落码提交 `af338387`（4 文件 +1,724/−68） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（3m21s 暖缓存）**；换装 **MATCH 3/3**、`.0.8.16-bak` 链预建；`--build-info`＝`0.9.0 os=windows` rc0；载体清单 3 entries 刷新 |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261011-247`；provision **exit 0**（正斜杠沿 218 教训）；`binary_sha256=f7bac89a…`↔换装位 `orz-signer.exe` 逐位一致；keystore `installation-key.json` Sep 12 mtime 未动 |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**（脚本内定 `-j 1` 暖 /target）；`MSYS_NO_PATHCONV=1`（218/243 教训预注）一次通过；bak 链宿主预建；**MATCH 3/3（直写换装位）**；**static-pie×3**（ET_DYN＋PT_INTERP=0）；alpine 3.20/bookworm 双冒烟 `0.9.0 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新。**环境如实记**＝docker 守护进程起批时未启动 → Docker Desktop 拉起 t+5s 就绪 |
| 进体字节判据 | **0cy 面全命中（WIN/LIN）**：`ORZ_GSA_ARCHIVE_ROOT` 0→2／0→2；`ORZ_GSA_HYGIENE` 0→1／0→1；`session-archive-package-v0.3` 0→1／0→1；`footprint` 0→9／0→46；扫描汇总行 `gsa hygiene sweep` 0→1／0→1。**schema 过渡如实记**＝`session-archive-package-v0.2` 2→1／2→1（常量升 v0.3；解码侧兼容位保留 1 处）。**243 窗保留面零回归（逐项持平）**：`clear_all` 27/28、`context_manage` 3/3、`keep_recent_rounds` 13/13、`handover` 38/27、清零 marker 前缀 2/2、权限句 `一键清理` 8/8、`mode=clear` 23/23、`op=clear` 42/20、`清零` 45/48、`清空` 64/31、`--target-directory` 1/1、`若意图是备份/暂存` 1/1、`deepseek-v4-flash` 0/0、`did you mean "` 2/2、`前推(` 1/1；负例 `cursor_agent` 0/0；`findings` 92→96／67→67＝0cy 代码面增长非回归（如实记） |
| 冒烟 | WIN `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6acadf82`，scratch 目录 `D:/tb-eval/smoke-247`）；**同目录第二轮＝0cy 启动侧扫描首次活体触发**：`gsa hygiene sweep: swept ["6acadf82"]; shared→Some("6acadf82")`，run 源目录删除、`archives/6acadf82.json.gz` 在案（**v0.3 四键核验**＝schema/archive_keys/conversation/footprint 齐；footprint.runs=1、conversation 侧车在包）、`ARC-6acadf82-1` 回执保留（D3 豁免面活体证）；第二轮 rc=0（`RUN-CLI-6acadf8f`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.9.0、sha `0C07208D…`） |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `db95ed15…`→**`1ce40857…`**（注释同步；adapter `6d55c26e…` 未动；语法＋`--help` rc0） |
| 构建告警 | WIN 1＝orz-host permission.rs L22 `unused_imports`（187 批在案先存） |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.8.16→0.9.0） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 58,353,152（+821,760） | `0c07208d121af341fc05b336e4c4c57e2fe887dd9e4f4220335541f7997d0bb4` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `f7bac89adc9235774ebc0ea160cb9199917b7ad16365f24c6faec560f10a02d5` |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `93cafd3cfdec52e07b84af9e10e5a37181e36343878635d94a0e8797211a68d2` |
| LIN | `orz` | 117,188,296（+1,169,256） | **`1ce40857f67cf0ac8c5a25909d7ea1a082ba0411097b0bf13c71c4f5b1a1a30b`（身份门新值）** |
| LIN | `orz-signer` | 1,397,512（+48） | `7cf5dbe2c3fa15518ba1e3a9151a921ce8b428016ecd7e1df19f3ad1518f84cd` |
| LIN | `orz-acaf-provision` | 1,215,928（−120） | `2ee04153539a7cbd5a5cc06fd6467d634c3703ed9728767ad25c2addc60feca8` |

（换装基线＝0.8.16 三件 `76904567…`/`1919b56f…`/`446d8cc7…`（WIN）与 `db95ed15…`/`d462590f…`/`a8c2dd4c…`（LIN），与 243 批账面逐位吻合。）

## §2 进体字节判据（方法与全量读数）

**方法**：python 字节直数（`bytes.count`；243 批同法），基线＝`.0.8.16-bak` 换装位预建件。

| 判据 | 基线（0.8.16）WIN/LIN | 0.9.0 WIN | 0.9.0 LIN |
|---|---|---|---|
| 0cy 新增：`ORZ_GSA_ARCHIVE_ROOT` | 0／0 | **2** | **2** |
| 0cy 新增：`ORZ_GSA_HYGIENE` | 0／0 | **1** | **1** |
| 0cy 新增：`session-archive-package-v0.3` | 0／0 | **1** | **1** |
| 0cy 新增：`footprint` | 0／0 | **9** | **46** |
| 0cy 新增：`gsa hygiene sweep`（stderr 汇总行） | 0／0 | **1** | **1** |
| schema 过渡：`session-archive-package-v0.2` | 2／2 | 1 | 1（兼容位保留；如实记） |
| 243 保留：`clear_all` | 27／28 | 27 | 28 |
| 243 保留：`context_manage` | 3／3 | 3 | 3 |
| 243 保留：`keep_recent_rounds` | 13／13 | 13 | 13 |
| 243 保留：`handover` | 38／27 | 38 | 27 |
| 243 保留：清零 marker 前缀 `` [前文上下文已清零 `` | 2／2 | 2 | 2 |
| 243 保留：权限句 `一键清理` | 8／8 | 8 | 8 |
| 243 保留：`mode=clear` | 23／23 | 23 | 23 |
| 243 保留：`op=clear` | 42／20 | 42 | 20 |
| 243 保留：`清零`／`清空` | 45/64／48/31 | 45／64 | 48／31 |
| 218 保留：`--target-directory`／`若意图是备份/暂存` | 1／1 | 1 | 1 |
| 218 保留：`deepseek-v4-flash`／`did you mean "`／`前推(` | 0/2/1 | 同 | 同 |
| 负例：`cursor_agent` | 0／0 | 0 | 0 |
| 增长面：`findings` | 92／67 | **96** | 67（0cy 代码面字符串增长，非回归） |

**判读**：0cy S3 字节判据（归档根 env／卫生关断 env／包 schema v0.3／足迹成员／启动侧扫描）
**全数入面**；0cz/0da（243 窗）与 0ct/0cu/0cv（218 窗）保留面**逐项持平零回归**。

## §3 台账

- 本档：`docs/audits/247_CARRIER_REBUILD_V090_0CY_S3_2026-10-11.md`。
- BACKLOG：0cy 批序 S1/S2 勾达成〔246〕＋S3 勾达成〔247〕＋指针行（本批＝247、前批链插
  246）＋计数行＋P1 总览片段＋开放项锚点。
- BACKLOG 第二卷：§1.192（246）／§1.193（247）。
- TODO：头行（本批＝247）＋P1 路由 0cy 片段＋P1-0cy S1/S2/S3 勾。
- 索引：头行 v4.218 → **v4.219**＋§6 GSA-SESSION-HYGIENE 条目更新（S1/S2/S3 达成）。
- orz 子仓：`a942d6f3` bump 批提交（**不推送**；落码提交 `af338387` 同在本地）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json`
  刷新 0.9.0。
- 残留件处置：`_hdr_new.txt`（238 批落账索引头草稿、0DA 轮 F3 登记）随本批删除闭合（内容已
  被现役索引头完全覆盖，零信息量）。

## §4 边界与如实记

1. **未推送未发行**：orz 两提交（`af338387`/`a942d6f3`）与父仓各批均在本地；不建 Release、
   不做 rel-stage 打包；发布面停 v0.8.16（244 批）。
2. **构建为暖缓存增量**（WIN 3m21s／LIN docker `-j 1` 暖 /target）——非 clean 全量口径。
3. **冒烟落点＝scratch 目录**（`D:/tb-eval/smoke-247`）：0.9.0 载体已带 0cy 启动侧扫描，
   D:\CLI 工作区级收卷属 S4 真机核证判据面，不在本批冒烟中半触发；scratch 内两轮顺带完成
   0cy 面首次活体观测（收卷/删源/包形/回执豁免四点全过）。
4. **0cy S4 真机核证待续**（harbor 多步重置语义复验＝新会话空板＋活区无前档足迹＋archives
   在案；净室用例 `ORZ_GSA_ARCHIVE_ROOT` 出工作区）；**0cw 干净重跑成本门另裁**；
   **0cz S4／0da S4 狗粮长轮实测待 0cy S4 后联动轮**。
5. **docker 守护进程**起批时未启动（245 批同状态）——Docker Desktop 拉起后 t+5s 就绪，
   无偏航；`MSYS_NO_PATHCONV=1` 预注一次通过（243 首跑偏航教训前移为预置）。
6. Linux 载体不重 provision（沿 093…243 同口径）。
7. **246 批（0cy S1＋S2）补登记**：246 批档/设计稿已随狗粮轮落盘，台账行（BACKLOG/TODO/
   索引）此前未同步（沿 0da 先例「报告即留痕」）——本批落账一并补齐，索引头前批链插入 246。

## §5 关键词

247 批、0.9.0 重建、源冻结 `a942d6f3`、0cy S3 进体、0cy 启动侧扫描活体首触、
`swept ["6acadf82"]`、包 v0.3 四键、`1ce40857…` 身份门、ACAF 重 provision `f7bac89a…`、
static-pie×3、alpine/bookworm 双冒烟 0.9.0、`ORZ_GSA_ARCHIVE_ROOT` 0→2、
`session-archive-package-v0.3` 0→1、保留面零回归、RUN-CLI-6acadf82/6acadf8f、
未推送未发行、计数 59 不变、v4.219。
