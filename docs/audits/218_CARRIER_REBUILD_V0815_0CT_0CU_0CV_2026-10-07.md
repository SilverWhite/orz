# 218 批：0.8.15 双平台载体重建进体——0ct／0cu／0cv 三件（2026-10-07）

> **用户令**：「请开始进行重建吧」。
> **本批**＝0.8.15 代窗口双平台载体重建（沿 194/187/180 批形态：源冻结→bump→双平台重建换装
> →进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**进件面**＝`93672eca`（204/209/
> 212/214 批 0ct .gsa 读向全开放＋0cu findings 第三分区＋处置＋旁路收口＋备份指引）＋`af6fa9a0`
> （217 批 0cv 接口名对齐）。
> **源冻结**＝orz **`10cfe765`**（`93672eca`＋`af6fa9a0`＋bump 0.8.14→0.8.15；Cargo.toml＋
> Cargo.lock 恰两行、`cargo metadata --locked` exit 0）；未推送。
> **零源码语义增量**；**计数不变（54）**；**未推送、未发行**（发布面停 v0.8.14〔195 批〕）。
> **并行窗交织（如实记）**：219 批（213 询问信修订，隔壁窗）同日先行落账并把 218 让号本批
> （「218 让号邻窗 0.8.15 重建批」）；索引 v4.190 跳号（v4.189→v4.191 邻窗实写），本批落账取
> **v4.192**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结＋bump | `10cfe765`（恰两行 Cargo.toml/Cargo.lock 0.8.14→0.8.15；locked exit 0）；源清单随批再生成 1,486 条（差 6 行＝217 四行未提交面＋bump 两行） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（6m06s 暖缓存）**；换装 **MATCH 3/3**、`.0.8.14-bak` 链预建；`--build-info`＝`0.8.15 os=windows` rc0；载体清单 3 entries |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261007-218`；provision **exit 0**；`binary_sha256=f86158ed…`↔换装位 `orz-signer.exe` 逐位一致；keystore 两件 Sep 12 mtime 未动。**首跑偏航如实记**＝Git Bash 反斜杠参数被 MSYS 剥除 ⇒ drive-relative 名被当相对路径、误生成散落 keystore/manifest 副本（真 keystore 未触）——清理散落件后**正斜杠**重跑成立 |
| Linux musl | docker `rust:1.97-slim`＋trixie 脚本 **exit 0**（`-j 1` 暖 /target）；bak 链宿主预建；**MATCH 3/3**；**static-pie×3**（ET_DYN＋PT_INTERP=0）；alpine 3.20/bookworm 双冒烟 `0.8.15 os=linux` rc0；Linux 不重 provision（沿同口径）；清单刷新 |
| 进体字节判据 | **0ct**：`--target-directory` 0→1（212 族 b 闭表成员）＋`若意图是备份/暂存` 0→1（214 备份指引）双平台；`session_volume_notice` WIN 6→5／LIN 7→6＝notice→denial 转译臂退役恰减一（journal 闭集保留）。**0cu**：`findings` 3→62（WIN）／3→59（LIN）。**0cv**：退役判据连续 `deepseek-v4-flash` **2→0**（双平台）；新名判据＝**len14 邻接折叠 0→1**（连续字面 0 系 LLVM 立即数物化、`movabs(deepseek)` 1→3——非缺失，如实记）。**194 窗保留面零回归**；退役面 `预算：`／`rli-shadow-v1` 0/0；负例 `cursor_agent` 0/0 |
| 冒烟 | WIN `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6ac63765`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.8.15 在册、sha `C9F30C75…`） |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `fc990a8a…`→**`75515440…`**（注释同步；adapter `6d55c26e…` 未动；语法＋`--help` rc0） |
| 构建告警 | WIN 1＝orz-host permission.rs `unused_imports`（187 批在案先存）；LIN 8＝unused 族先存＋rustup 工具链通知＋rg PATH 提示（均触碰面外、非本批引入） |

## §1 双平台三件套（终态）

| 平台 | 文件 | 尺寸 (B)（0.8.14→0.8.15） | SHA256 |
|---|---|---|---|
| WIN | `orz.exe` | 57,135,104（+18,432） | `c9f30c757896…` |
| WIN | `orz-signer.exe` | 6,740,480（Δ0） | `f86158ed6f37…` |
| WIN | `orz-acaf-provision.exe` | 6,640,128（Δ0） | `c887b1a98a9b…` |
| LIN | `orz` | 115,615,912（+22,704） | **`75515440b4ee69cc6bd8a5c2d009150822b243aa6c6fdad7aa0f6731386facb7`（身份门新值）** |
| LIN | `orz-signer` | 1,397,544（+16） | `f1a1d784d81d…` |
| LIN | `orz-acaf-provision` | 1,215,992（Δ0） | `5893eec4029a…` |

（换装基线＝0.8.14 三件 `8dbbd61f…`/`aa1ab041…`/`fdb7a788…`（WIN）与 `fc990a8a…`/`ff2f3ac7…`/`620f91cc…`（LIN），与 195/194 批账面逐位吻合；signer/provision 哈希变化＝共享依赖重编传播（187/194 同形先例）。）

## §2 进体字节判据（方法与全量读数）

**方法**：python 字节直数（`bytes.count`）。**方法注（如实记）**：① grep 对 `--` 前缀模式
（`--target-directory`）吞成选项致假阴性 0，改直数后 1/1；② WIN 构建 log 含 GB 中文横幅被
grep 判二进制静默，`cat -v` 直读恢复（0bs F6 输出编码链同族）。

| 判据 | 基线（0.8.14） | 0.8.15 WIN | 0.8.15 LIN |
|---|---|---|---|
| 0cv 退役：`deepseek-v4-flash` 连续 | 2／2 | **0** | **0** |
| 0cv 新名：`deepseek-flash` 连续 | 0 | 0（折叠形态） | 0（折叠形态） |
| 0cv 新名：len14 邻接折叠 | 0 | **1** | **1** |
| 0cv 旁证：`movabs(deepseek)` | 1 | 3 | 3 |
| 0ct 新增：`--target-directory` | 0 | **1** | **1** |
| 0ct 新增：`若意图是备份/暂存` | 0 | **1** | **1** |
| 0ct 保留：`session_volume_notice` | 6／7 | 5（−1＝转译臂退役） | 6（−1 同形） |
| 0cu 新增：`findings` | 3／3 | **62** | **59** |
| 194 保留：`did you mean "` | 2 | 2 | 2 |
| 194 保留：`run_command_name`／D4 渲染串×2／`前推(` | 1×4 | 1×4 | 1×4（`rli.notice.` 7/1、`coverage_gap` 5/6、`u_prog` 22 同 194 口径） |
| 194 保留：`slow_prog`/`carrier-write`/`ORZ_LIF_RLI_SHADOW`/`streak_crossed`/`domain_spike_entry`/`migration_confirmed` | 各在件 | 全在件 | 全在件 |
| 194 退役：`预算：`／`rli-shadow-v1` | 0 | 0 | 0 |
| 负例：`cursor_agent` | 0 | 0 | 0 |

**0cv 判读**：`MAIN_AGENT_MODEL` 与 web_search 回退两处新名均被 LLVM 折叠为「8 字节立即数
`deepseek`＋紧随 14（`0x0e`）长度常量」的运行期物化（`deepseek-flash` 恰 14 字节）——连续
字面消失属编译期形态、非缺失；退役面连续字面 2→0 为直接证据，两者合读＝改名已进体。

## §3 台账

- 本档：`docs/audits/218_CARRIER_REBUILD_V0815_0CT_0CU_0CV_2026-10-07.md`。
- BACKLOG：0ct 节 S3 勾达成（218 读数）＋0cu 排期 S3 勾达成＋0cv 边界进体达成＋计数行（本批
  ＝218、前批链插 219）＋指针行（前批链插 219）＋优先级总览表三片段＋P1 开放项锚点行三片段。
- BACKLOG 第二卷：§1.166。
- TODO：头行（本批＝218、前批＝219）＋P1 路由行三片段＋P1-0ct S3 勾＋P1-0cu S3 勾＋P1-0cv
  进体注记＋闭合行更新。
- 索引：头行 v4.191 → **v4.192**（本批＝218；前批＝219／217…；**v4.190 跳号如实记**）＋§8
  `pending` 桶三片段（0ct/0cu/0cv S3 进体达成）。
- orz 子仓：`10cfe765` bump 批提交（**不推送**；落码两提交 `93672eca`/`af6fa9a0` 同在本地）。
- 身份门：`scripts/run_r0_heavy_official.py` 载体锁定值换装。

## §4 边界与如实记

1. **未推送未发行**：orz 三提交（`93672eca`/`af6fa9a0`/`10cfe765`）与父仓各批均在本地；不建
   Release、不做 rel-stage 打包；发布面停 v0.8.14（195 批）。
2. **构建为暖缓存增量**（WIN 6m06s／LIN docker `-j 1` 暖 /target）——非 clean 全量口径。
3. **0ct/0cu S4 真机复核待下一真机轮**（0ct＝读向 `cp`/`cat` 放行＋写向仍拦；0cu＝模型自发
   findings 写入形态如实记、不设使用率判据）；0cv 闭合待用户裁决（计数不变 54）。
4. **ACAF provision 首跑偏航**：MSYS 反斜杠剥除 ⇒ 散落件误生成（新 keystore 副本＋manifest、
   字面名 `tb-evalorz-windowsacaf*`）——真 keystore 未触；散落件已清理、正斜杠重跑成立；
   教训与 ORZ-BUILD-MOUNT-001/194 首跑 127 同族（Windows 壳层参数形态），沿批档如实记。
5. **并行窗交织**：219 批先落账并让号 218；两窗台账编辑经逐一断言合并（零互损；中途一次锚点
   片段误写 `0co 于` 已即改回 `0cu 于` 并复核恰一）；索引 v4.190 跳号。
6. Linux 载体不重 provision（沿 093…194 同口径）；manifest 已刷新 0.8.15。

## §5 关键词

218 批、0.8.15 重建、源冻结 `10cfe765`、0ct 进体、0cu 进体、0cv 进体、`75515440…` 身份门、
ACAF 重 provision `f86158ed…`、static-pie×3、alpine/bookworm 双冒烟 0.8.15、
`deepseek-v4-flash` 2→0、len14 折叠 0→1、`findings` 3→62/59、RUN-CLI-6ac63765、MSYS 路径
剥除偏航、v4.190 跳号、218 让号、未推送未发行、计数 54 不变。
