# 187 批：0.8.13 双平台载体重建进体——0cq S3 收口＋184/185/186 三批进件（2026-10-04）

> **用户令**：「请对审查出的全部问题进行处理吧。全部处理完成后请进入重建」（承接 186 批审查处置）。
> **本批**＝0.8.13 代窗口双平台载体重建（沿 117/144/147/155/170/180 批形态：源冻结→双平台重建换装→
> 进体字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**进件面**＝184 批 0am 预测段（单 fire 前推
> 预告段＋行宽两预算重订）＋185 批 0cq S2 写控误拦两族修复（四件）＋186 批审查处置（三件）。
> **源冻结**＝orz **`a7526cc2`**（`f0b7f5ac` 186 批处置＋`a7526cc2` bump 0.8.12→0.8.13；
> `cargo metadata --locked` exit 0、Cargo.toml＋Cargo.lock 恰两行）；未推送。
> **打包顺延**：本批不做 rel-stage 打包、不建 Release（无发行压力；沿 155/170/180 批口径如实记）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（cargo 4m14s 暖缓存增量）**，退出码独立落盘取；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、`.0.8.12-bak` 链；`--build-info`＝`0.8.13 os=windows` exit 0；载体清单刷新（3 entries, 0.8.13） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261004-186`；provision **exit 0**；`binary_sha256=8aa9aafd…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件（Sep 12 原件）逐位未动（内层正典 again） |
| signer/provision 哈希变化（如实记） | 尺寸 signer Δ0／provision Δ0、**哈希均变化**（signer `860c9860…`→`8aa9aafd…`、provision `06b7e693…`→`57d01eef…`）——184 批 `RliChannel::forecast`／`scan_crossing` 在 orz-assurance、链接进两件所致（合理传播非异常；180 同形） |
| Linux musl | docker `rust:1.97-slim`（镜像在位无重拉）＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001、`MSYS_NO_PATHCONV=1`、/target 缓存沿用）**cargo 22m16s exit 0（-j 1）**；`.0.8.12-bak` 链于容器末段 cp 前**预建**（180 §6.4 教训面兑现）；产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.13 os=linux` exit 0；**Linux 载体未重 provision**（沿 093…180 同口径）；Linux 载体清单顺手刷新 0.8.11→0.8.13（180 批未刷新的陈旧面，如实记） |
| 进体字节判据（判据表随批更新 0.8.13 窗） | **NEW（184 预测段）**：`前推(` 1/1〔WIN/LIN〕、段内五值闭集（`段内θ上持续`/`段内回落`/`段内再越线`/`段内不越线`/`段内越线`）各 1/1、注解扩展 `前推=闭式自由演化至` 1/1；**NEW（185/186 写控）**：`--iglob` 1/3、`--exclude-dir` 0/6（LIN 命中来自捆绑 rg 串）——`-wholename`/`--include-dir` 双平台记 0＝短 ASCII 串内联/去重已知形态（工具注记如实记；该面权威证据＝185 批功能钉） **保留面零回归**：`rli.notice.` 7→7（WIN）/1→1（LIN）、`streak_crossed`/`domain_spike_entry`/`migration_confirmed` 1→1、`coverage_gap` 5→5/6→6、`u_prog` 22→22/32→32、`slow_prog` 3→3、`carrier-write` 1→1、`ORZ_LIF_RLI_SHADOW` 3→3/3→3、0.8.12 窗全量（P8 锚点/成因段标签环/词表 v2/179 kill-switch 文案）逐项在件；**恢复面**：`recursive delete targets a fundamental tree root` 1/1（186 批续行修复后逐字节同形）；**退役面**：`预算：` 0→0、`rli-shadow-v1` 0→0 |
| 版本串判读 | `0.8.13`＝1（WIN）/1（LIN，＝orz 本体）；件内 `0.8.12`＝0（WIN）/1（LIN，＝ahash 0.8.12 依赖串）；`0.8.11`＝14（WIN）/9（LIN）＝regex-syntax 0.8.11 版本串（Cargo.lock 在案）——本体身份以 `--build-info`＝0.8.13 双平台为权威 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `b141c1ef…` → **`0e1c10e7…`**（注释同步：0.8.13＝源冻结 `a7526cc2`＝186 批＋bump；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载 rc 0 |
| 冒烟 | Windows `--fake-provider -p hello` 整轮 rc=0（直取退出码；run `RUN-CLI-6ac1a9df`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.8.13 在册识别、sha 232B7517…）；Linux alpine 3.20/bookworm `--build-info` 双绿 |

## §1 Windows 三件套（终态）

| 文件 | 尺寸 (B)（0.8.12→0.8.13） | SHA256 |
|---|---|---|
| `orz.exe` | 57,107,456（+6,144） | `232b75172f26866b…` |
| `orz-signer.exe` | 6,740,480（Δ0） | `8aa9aafde0698096…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `57d01eef643e3dc6…` |

（换装基线＝0.8.12 三件 `40bdda50…`/`860c9860…`/`06b7e693…` 与 180 批账面逐位吻合。）

## §2 Linux musl 三件套（终态）

| 文件 | 尺寸 (B)（0.8.12→0.8.13） | SHA256 |
|---|---|---|
| `orz` | 115,577,992（+8,488） | `0e1c10e7f4e5b5e58b65429df43485949a0dac66150b3afbbb0627cd96e57b7a`（身份门新值） |
| `orz-signer` | 1,397,544（Δ0） | `0e094d535bdfb8d3…` |
| `orz-acaf-provision` | 1,215,984（−16） | `3c404ff1bb06008b…` |

## §3 边界与如实记

1. **未推送未发行**：orz 两提交（`f0b7f5ac`/`a7526cc2`）与父仓各批均在本地；不建 Release、不做 rel-stage 打包；发布面仍停 v0.8.12（183 批）。
2. **Windows 构建为暖缓存增量**（4m14s；184–186 批触 orz-assurance/orz-loop/codegen 面）——非 clean 全量口径，与 170（25m07s）差异如实记；退出码直取未经管道。
3. **构建告警**：WIN 1 条＝`orz-host` permission.rs:22 `unused_imports`（该文件自 0.8.12 冻结前未动——本窗依赖扇出重编译复现的先存源面告警，触碰面外）；LIN 13 条＝先存集合。clippy 口径零新增（186 批核证）。
4. **`-wholename`/`--include-dir` 双平台字节 0**：短 ASCII 串编译期内联/去重已知形态（工具注记；同 170 §2-A imm64 家族）——185 批 `read_pattern_values_are_not_write_targets` 等功能钉为该面权威证据。
5. **Linux 载体清单陈旧面顺手修复**：`D:/tb-eval/orz-linux/carrier-manifest.json` 停留 0.8.11（180 批未刷新）——本批刷新至 0.8.13（3 entries）；Linux 载体仍不做 ACAF 重 provision（沿同口径）。
6. **冒烟退出码纪律**：直取退出码（155 §2-A 教训面）；alpine 冒烟经 `MSYS_NO_PATHCONV=1`（Git Bash 路径转换首跑 127 作废、直取重跑 rc=0 方为读数）。
7. **0cq 余项**：S1/S2/S3 全部落码＋进体后，余＝**123 批「零误拦」读数的真机复核**（归下一真机轮；0cq 项保持开放、计数 58 不变）；预测段真机读数随 0bc 同轮观测顺接（09-20 顺序裁决不变）。

## §4 关联与关键词

[`186 批档`](186_REVIEW_FINDINGS_DISPOSAL_2026-10-04.md)／[`185 批档`](185_0CQ_S1_S2_WRITE_CONTROL_FALSE_BLOCK_FIX_2026-10-04.md)／
[`184 批档`](184_0AM_FORECAST_SEGMENT_DESIGN_AND_IMPL_2026-10-04.md)／[`180 批档`](180_CARRIER_REBUILD_V0812_0AM_P8_ENTRY_2026-10-04.md)（同形态先例）／
[`170 批档`](170_CARRIER_REBUILD_V0811_0CP_S3_0CN_S3_2026-10-03.md)／BACKLOG `0cq`/`0am`／TODO `P1-0cq`/`P1-0am`。

关键词：187 批、0.8.13 重建、源冻结 `a7526cc2`、184 预测段进件、185/186 写控修复进件、
`0e1c10e7…` 身份门、ACAF 重 provision 8aa9aafd、static-pie×3、alpine/bookworm 双冒烟 0.8.13、
段内五值闭集在件、前推=闭式自由演化至在件、bak 链预建、Linux 载体清单陈旧面修复、
短 ASCII 串内联记 0、打包顺延、未推送未发行、计数 58 不变。
