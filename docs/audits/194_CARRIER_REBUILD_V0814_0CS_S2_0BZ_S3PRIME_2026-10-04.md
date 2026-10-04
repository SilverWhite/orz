# 194 批：0.8.14 双平台载体重建进体——0cs S2 收口＋0bz S3′＋193 处置进件（2026-10-04）

> **用户令**：「请进行重建吧」（承接 193 批审查处置）。
> **本批**＝0.8.14 代窗口双平台载体重建（沿 117/144/147/155/170/180/187 批形态：源冻结→双平台重建
> 换装→进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**进件面**＝191 批 0cs S1 did-you-mean
> ＋192 批 0bz S3′ D4 移尾＋193 批审查处置（注释同步＋marker 变体钉＋活名回显微修）。
> **源冻结**＝orz **`b5cb57ca`**（`2d51d22c` 191 批＋`0b1e04f9` 192 批＋`54667109` 193 批＋`b5cb57ca`
> bump 0.8.13→0.8.14；`cargo metadata --locked` exit 0、Cargo.toml＋Cargo.lock 恰两行）；未推送。
> **打包顺延**：本批不做 rel-stage 打包、不建 Release（无发行压力；沿 155/170/180/187 批口径如实记）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（cargo 5m13s 暖缓存增量）**，退出码落盘直取；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、`.0.8.13-bak` 链预建；`--build-info`＝`0.8.14 os=windows` exit 0；载体清单刷新（3 entries, 0.8.14） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261004-193`；provision **exit 0**；`binary_sha256=aa1ab041…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件逐位未动（内层正典不变） |
| signer/provision 哈希变化（如实记） | 尺寸 signer Δ0／provision Δ0、**哈希均变化**（signer `8aa9aafd…`→`aa1ab041…`、provision `57d01eef…`→`fdb7a788…`）——件内无版本串（0.8.13/0.8.14 均 0 命中），归因＝191–193 批 orz-tools/orz-loop 面链接传播＋共享依赖重编译（187/180 同形先例，合理传播非异常） |
| Linux musl | docker `rust:1.97-slim`（镜像在位无重拉）＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001、`MSYS_NO_PATHCONV=1`、/target 缓存沿用）**cargo 9m28s exit 0（-j 1）**；`.0.8.13-bak` 链于宿主**预建**（180 §6.4 教训面兑现，先于容器 cp 步）；产物与 `/target/…/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.14 os=linux` exit 0；**Linux 载体未重 provision**（沿 093…187 同口径）；载体清单刷新 0.8.13→0.8.14（3 entries） |
| 进体字节判据（双平台） | **NEW（191 0cs）**：`did you mean "` 2/2〔WIN/LIN〕、表键 `run_command_name`/`terminal_exec`/`exec_command` 各 1/1、负例 `cursor_agent` 0/0（不收名不入体）；**0bz 保留面**：`【本 run 自编辑文件】` 1/1、`【最近编辑指纹】` 1/1（D4 移尾只动位置、渲染串全保留）、`[前文上下文已压缩` 5（WIN）/6（LIN）＝压缩/截断两前缀族（双平台去重差已知形态）；**187 窗保留面零回归**：`前推(` 1/1、段内五值闭集各 1/1、注解扩展 1/1、`rli.notice.` 7（WIN）/1（LIN）、`coverage_gap` 5/6、`u_prog` 22/32、`slow_prog` 3/3、`carrier-write` 1/1、`ORZ_LIF_RLI_SHADOW` 3/3、`streak_crossed`/`domain_spike_entry`/`migration_confirmed` 1/1、写控续行 1/1——与 187 批读数逐位同型；**退役面**：`预算：` 0/0、`rli-shadow-v1` 0/0 |
| 版本串判读 | `0.8.14`＝WIN/LIN `--build-info` 双绿（本体身份权威）；orz.exe 内 `0.8.14` 在件 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `0e1c10e7…` → **`fc990a8a…`**（注释同步：0.8.14＝源冻结 `b5cb57ca`＝191＋192＋193＋bump；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载 rc 0 |
| 0cr 冻结面顺延 | BACKLOG/TODO 0cr 节「载体 0.8.13（`0e1c10e7…`）」→「载体 0.8.14（`fc990a8a…`）」（191 批 §5 预告的顺延义务兑现） |
| 冒烟 | Windows `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6ac27394`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.8.14 在册识别、sha `8DBBD61FFEAE…`；题面 BOM 补写在案） |
| 首跑如实记 | Windows 构建首跑经相对路径调用 exit 127（log 仅横幅＝未进 cargo；Git Bash 调用形态问题非源面）——绝对路径＋`-ExecutionPolicy Bypass` 重跑 exit 0 方为读数；退出码纪律（直取落盘）保持 |

## §1 Windows 三件套（终态）

| 文件 | 尺寸 (B)（0.8.13→0.8.14） | SHA256 |
|---|---|---|
| `orz.exe` | 57,116,672（+9,216） | `8dbbd61ffeaed786f887139d09a4e35ba22ba2e7be60a491ecced4e757f835bd` |
| `orz-signer.exe` | 6,740,480（Δ0） | `aa1ab04115c33be5173c304b1bb39fda1ce111c2629b2bb58bacf7adda0b072f` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `fdb7a788daa8898ebef831cd288452162985d9a1a7201cb5048546113351aad1` |

（换装基线＝0.8.13 三件 `232b7517…`/`8aa9aafd…`/`57d01eef…` 与 187 批账面逐位吻合。）

## §2 Linux musl 三件套（终态）

| 文件 | 尺寸 (B)（0.8.13→0.8.14） | SHA256 |
|---|---|---|
| `orz` | 115,593,208（+15,216） | `fc990a8abe05e65b05a88fbc3c9c2fa17e1dd34f5243e7c78333215fc8029610`（身份门新值） |
| `orz-signer` | 1,397,528（−16） | `ff2f3ac7ae815c16badd592c33c6f888704cfcbeee746d15ad67ff1d53c6b8d8` |
| `orz-acaf-provision` | 1,215,992（+8） | `620f91ccfa9b8cae43451e425f0780def584f1ad6fd622f187ad9d68596c7d4e` |

## §3 边界与如实记

1. **未推送未发行**：orz 两提交（`54667109`/`b5cb57ca`）与父仓各批均在本地；不建 Release、不做 rel-stage 打包；发布面仍停 v0.8.12（183 批）。
2. **构建为暖缓存增量**（WIN 5m13s／LIN 9m28s；/target 缓存跨批沿用）——非 clean 全量口径，与 187（22m16s）差异如实记；LIN 首段 apt/toolchain 走容器缓存。
3. **构建告警**：WIN 1 条＝`orz-host` permission.rs `unused_imports`（187 批在案先存，触碰面外）；LIN 1 条＝`orz-config` 同形先存。clippy 口径零新增（193 批 107=107 核证）。
4. **0cs 负例判读**：`cursor_agent` 双平台 0＝「无近似不附」名不入体的字节面实证；`invoke`/`run_task` 因通用词串风险不做负例扫描（功能面由钉 8 覆盖）。
5. **`[前文上下文已压缩` WIN 5／LIN 6**：压缩 marker 与 T1 截断 marker 两前缀族的链接期去重差（同 170 §2-A 短串内联/去重家族）——非回归（双平台均非零、功能钉在件）。
6. **Linux 载体不重 provision**（沿 093…187 同口径）；manifest 陈旧面本批已同步刷新（0.8.13→0.8.14）。
7. **0bz S4 终验与 0cs S3 真机**：机械判据（+2 针归零、空跑零分叉）与幻影名改道轮数下降均待下一真机轮（0bz 判据④／0cr 首题顺带）——本批只完成载体进体，不预支真机读数。

## §4 关联与关键词

[`193 批档`](193_REVIEW_DISPOSAL_0BZ_0CS_2026-10-04.md)／[`192 批档`](192_0BZ_S3PRIME_D4_TAIL_AND_S4_OFFLINE_READINGS_2026-10-04.md)／
[`191 批档`](191_0CS_S1_TOOL_NAME_SUGGESTION_IMPL_2026-10-04.md)／[`187 批档`](187_CARRIER_REBUILD_V0813_0CQ_S3_2026-10-04.md)（同形态先例）／
[`180 批档`](180_CARRIER_REBUILD_V0812_0AM_P8_ENTRY_2026-10-04.md)／BACKLOG `0bz`/`0cs`/`0cr`／TODO `P1-0bz`/`P2-0cs`/`P1-0cr`。

关键词：194 批、0.8.14 重建、源冻结 `b5cb57ca`、191 did-you-mean 进件、192 D4 移尾进件、193 处置进件、
`fc990a8a…` 身份门、ACAF 重 provision `aa1ab041…`、static-pie×3、alpine/bookworm 双冒烟 0.8.14、
`did you mean "` 在件、cursor_agent 0、D4 渲染串保留、187 窗保留面零回归、退役面 0、
bak 链预建、0cr 冻结面顺延、首跑 127 绝对路径重跑、打包顺延、未推送未发行、计数 60 不变。
