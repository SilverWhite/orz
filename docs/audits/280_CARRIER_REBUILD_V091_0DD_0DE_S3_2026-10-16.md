# 280 批：0.9.1 双平台载体重建进体——0dd S3＋0de S3 合并（2026-10-16）

> **用户令**：「请先进行重建吧」（未指名版本，沿 218/243 批先例取补丁位递增 0.9.0→0.9.1）。
> **本批**＝0.9.1 代窗口双平台载体重建（沿 247/243/218 批形态：源冻结→bump→双平台重建换装
> →进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**0dd S3 与 0de S3 合并同一次
> 重建**（278 批档既定合并方案）。**进件面**＝`1ecad57c`（279 处置批）＋`644f6b01`（278 落码
> ＝0de D1/D3/D5/D6/D7）＋`4749f6b1`（276 落码＝0dd run 尾自动归档退役）。
> **源冻结**＝orz **`0be4a364`**（`1ecad57c`＋bump 0.9.0→0.9.1；Cargo.toml＋Cargo.lock 恰两行、
> `cargo metadata --locked` exit 0）；未推送。
> **零源码语义增量**（bump 恰两行）；**计数不变（61）**；**未推送、未发行**（发布面停 v0.9.0）。
> **0dd S3／0de S3 在本批达成**；两线 S4 真机复验待续。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结＋bump | `0be4a364`（恰两行 0.9.0→0.9.1；locked exit 0）；处置批提交 `1ecad57c`（5 文件 +131/−5） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（3m18s 暖缓存）**；换装 **MATCH 3/3**、`.0.9.0-bak` 链预建；`--build-info`＝`0.9.1 os=windows` rc0；载体清单 3 entries 刷新（v0.9.1） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261016-280`；provision **exit 0**（正斜杠沿 218 教训）；`binary_sha256=dd55ded5…`↔换装位 `orz-signer.exe` 逐位一致；keystore `installation-key.json` Sep 21 mtime 未动 |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**（脚本内定 `-j 1` 暖 /target）；`MSYS_NO_PATHCONV=1`（218/243 教训预注）一次通过；bak 链宿主预建；**直写换装位**；**static-pie×3**（ET_DYN＋PT_INTERP=0）；alpine 3.20/bookworm 双冒烟 `0.9.1 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新（v0.9.1） |
| 进体字节判据 | **0de D1 新面 `reminder_only_no_compressible` 0→7（WIN/LIN 同数）**；版本面 `0.9.1` +1／`0.9.0` 残留为依赖子串（WIN 18→17、LIN 6→5，非载体版本面）；`stop_declared` 0→0（内部旗标不进二进制，预判如实记）；**247/0cy/0dd 保留面逐项持平**（全表见 §2）；负例 `cursor_agent`/`deepseek-v4-flash` 0/0；增长面 `findings` WIN 96→96／LIN 67→69（代码面字符串增长非回归，如实记）。D6（web `projection.js`）资产不内嵌载体，无字节面（N5 前端冒烟为该面验证入口，随 279 已绿） |
| 冒烟 | WIN scratch（`D:/tb-eval/smoke-280`）两轮 `--fake-provider -p` 均整轮 rc=0；**第二轮＝0cy 启动侧扫描活体**：首会话 `6ad2152c` 收卷、源目录删除、`archives/6ad2152c.json.gz`（3,078 B）在案、`ARC-6ad2152c-1` 回执保留；两轮 run 尾**零主动归档**（0dd 退役面；短轮冒烟对 0.9.0 不构成区分度——跨里程碑判据由 N1 钉承载，如实记） |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `1ce40857…`→**`f50b1f17…`**（注释同步重写；adapter `6d55c26e…` 未动；语法 py_compile 过＋`--help` rc0） |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（187 批在案先存，与 247 同源） |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.9.0→0.9.1） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 58,369,024（+15,872） | `39f6f3fac740b2a0…` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `dd55ded5abd2ebbf…`（provision 新值） |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `34563cad86353332…` |
| LIN | `orz` | 117,192,312（+4,016） | **`f50b1f17987ff7989003823cfafb1cc85968523e9a0455e1fe17da2dd1ccef1b`（身份门新值）** |
| LIN | `orz-signer` | 1,397,496（−16） | `a57a4d298ab1efe2…` |
| LIN | `orz-acaf-provision` | 1,215,952（+24） | `46a7b6d058a49eee…` |

（换装基线＝0.9.0 三件 `0c07208d…`/`f7bac89a…`/`93cafd3c…`（WIN）与 `1ce40857…`/`7cf5dbe2…`/`2ee04153…`（LIN），与 247 批账面逐位吻合。）

## §2 进体字节判据（方法与全量读数）

**方法**：python 字节直数（`bytes.count`；243/247 批同法），基线＝`.0.9.0-bak` 换装位预建件。

| 判据 | 基线（0.9.0）WIN/LIN | 0.9.1 WIN | 0.9.1 LIN |
|---|---|---|---|
| 0de D1 新面：`reminder_only_no_compressible` | 0／0 | **7** | **7** |
| 版本面：`0.9.1` | 17／25 | 18 | 26 |
| 版本残留：`0.9.0`（依赖/常量子串，非载体版本面） | 18／6 | 17 | 5 |
| `stop_declared`（内部旗标，预期 0＝不进二进制） | 0／0 | 0 | 0 |
| 247 保留：`clear_all` | 27／28 | 27 | 28 |
| 247 保留：`context_manage` | 3／3 | 3 | 3 |
| 247 保留：`keep_recent_rounds` | 13／13 | 13 | 13 |
| 247 保留：`handover` | 38／27 | 38 | 27 |
| 247 保留：清零 marker 前缀 `` [前文上下文已清零 `` | 2／2 | 2 | 2 |
| 247 保留：权限句 `一键清理` | 8／8 | 8 | 8 |
| 247 保留：`mode=clear`／`op=clear` | 23/42／23/20 | 23／42 | 23／20 |
| 247 保留：`清零`／`清空` | 45/64／48/31 | 45／64 | 48／31 |
| 218 保留：`--target-directory`／`若意图是备份/暂存` | 1／1 | 1 | 1 |
| 218 保留：`did you mean "`／`前推(` | 2/1／2/1 | 2／1 | 2／1 |
| 负例：`cursor_agent`／`deepseek-v4-flash` | 0／0 | 0 | 0 |
| 增长面：`findings` | 96／67 | 96 | **69**（代码面字符串增长非回归，如实记） |
| 0cy 保留：`ORZ_GSA_ARCHIVE_ROOT`／`ORZ_GSA_HYGIENE` | 2/1／2/1 | 2／1 | 2／1 |
| 0cy 保留：`session-archive-package-v0.3`／`gsa hygiene sweep` | 1/1／1/1 | 1／1 | 1／1 |
| 0dd 保留：`incremental`（无头道原语保留面） | 14／25 | 14 | 25 |

**判读**：0de S3 字节判据（D1 空集审计 form 面）**全数入面**；0dd 为纯退役批无新增字符串面
（`incremental` 无头道原语保留面逐项持平＝「单一源不破」字节级佐证）；243/218/247 窗保留面
**逐项持平零回归**。

## §3 台账

- 本档：`docs/audits/280_CARRIER_REBUILD_V091_0DD_0DE_S3_2026-10-16.md`。
- BACKLOG：0dd/0de 批序 S3 勾达成〔280〕＋指针行（本批＝280、前批链插 279）＋计数行＋P1 总览
  片段＋开放项锚点。
- BACKLOG 第二卷：§1.216（279）／§1.217（280）。
- TODO：头行（本批＝280）＋P1 路由 0dd/0de 片段＋P1-0dd/P1-0de S3 勾。
- 索引：头行 v4.252 → **v4.253**。
- orz 子仓：`0be4a364` bump 批提交（**不推送**；落码/处置提交 `1ecad57c`/`644f6b01`/`4749f6b1`
  同在本地）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装（注释同步重写）。
- 换装位清单：`D:/tb-eval/orz-windows/carrier-manifest.json`／`D:/tb-eval/orz-linux/carrier-manifest.json`
  刷新 0.9.1（`generate_carrier_manifest.py`，各 3 entries）。
- 字节判据脚本：`D:/tb-eval/byte_criteria_280.py`（评测区本地面，不入仓）。

## §4 边界与如实记

1. **未推送未发行**：orz 两新提交（`1ecad57c`/`0be4a364`）与父仓各批均在本地；不建 Release、
   不做 rel-stage 打包；发布面停 v0.9.0。
2. **构建为暖缓存增量**（WIN 3m18s／LIN docker `-j 1` 暖 /target）——非 clean 全量口径。
3. **冒烟落点＝scratch 目录**（`D:/tb-eval/smoke-280`）：0cy 启动侧收卷只在 scratch 内活体触发，
   不触碰 D:\CLI 工作区会话；短轮冒烟对 0dd 退役无区分度（0.9.0 同形），跨里程碑判据由
   N1 钉（276）承载、S4 真机复验补活体。
4. **版本号取值**：用户令未指名版本，沿 218/243 批「请开始进行重建吧」→补丁位递增先例取
   0.9.1；与本批同日若需 0.10.0 可随发行批重择（版本号纯标记，无契约面）。
5. **D6 无字节面**：web `projection.js` 由 orz-web 运行时从盘伺服、不内嵌载体（src 无
   include_str/bytes）——该面验证入口为前端冒烟 N5（279 批已绿），字节判据不覆盖，如实记。
6. **`findings` LIN 67→69**：与 WIN 96→96 不同源的链接期字符串合并差异方向，增长面非回归，
   如实记不深究。
7. **docker 守护进程**本批起批时已在运行（245/247 需手动拉起的状态未复现）；磁盘起批余量
   12–13 GiB（279 处置批清理后），全程未触限。
8. **0dd S4／0de S4 真机复验待续**：0dd＝跨里程碑长答 run 尾零归档活体（N1 场景真机形）；
   0de＝事发形态回放（H1 空集只提醒不开窗＋收束同消息豁免＋括号注解选择）。

## §5 关键词

280 批、0.9.1 重建、源冻结 `0be4a364`、0dd S3＋0de S3 合并进体、
`f50b1f17…` 身份门、ACAF 重 provision `dd55ded5…`、static-pie×3、
alpine/bookworm 双冒烟 0.9.1、`reminder_only_no_compressible` 0→7、
保留面零回归、RUN-CLI-6ad2152c/6ad2153a、未推送未发行、计数 61 不变、v4.253。
