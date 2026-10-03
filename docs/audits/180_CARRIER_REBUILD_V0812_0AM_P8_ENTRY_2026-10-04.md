# 180 批：0.8.12 双平台载体重建进体——0am P8 面首次进件（2026-10-04）

> **用户令**：「请进行重建吧」（承接 176/177 批 P8-a/P8-b 落码＋178 批审查处置＋179 批 RS-06 收口）。
> **本批**＝0.8.12 代窗口双平台载体重建（沿 117/144/147/155/170 批形态：源冻结→双平台重建换装→进体
> 字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。**0am 余项自此收敛＝真机轮 Verify 读数预注册**（＋0bc 顺序裁决）。
> **源冻结**＝orz **`54717d06`**（`905bc3f5` 179 批 RS-06 收口＋`54717d06` bump 0.8.11→0.8.12；
> `cargo metadata --locked` exit 0、Cargo.toml＋Cargo.lock 恰两行）；未推送。
> **打包顺延**：本批不做 rel-stage 打包、不建 Release（无发行压力；沿 155/170 批口径如实记）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（cargo 5m04s 暖缓存增量）**，退出码独立落盘取（155 §2-A 方学注记）；换装 `D:\tb-eval\orz-windows\` **逐位核对一致**、`.0.8.11-bak` 链；`--build-info`＝`0.8.12 os=windows` exit 0；载体清单刷新（3 entries, 0.8.12） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261004-180`；provision **exit 0**；`binary_sha256=860c9860…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件（`f37556ab…`/`2aa80cb8…`，Sep 12 原件）逐位未动（内层正典 again） |
| signer/provision 哈希变化（如实记） | 与 155/170 的「signer Δ0」不同，本窗两件**哈希变化、尺寸不变**（signer 860c9860…／provision 06b7e693…）——179 批 `RliChannel::restore` sanitize 改动在 orz-assurance，链接进两件所致（合理传播非异常）；provision 重绑后 manifest↔换装位逐位一致 |
| Linux musl | docker `rust:1.97-slim`（镜像在位无重拉）＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001、`MSYS_NO_PATHCONV=1`、/target 缓存沿用）**cargo 32m59s exit 0（-j 1）**；`.0.8.11-bak` 链于容器末段 cp 前**预建**（防直写丢失回滚点）；产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.12 os=linux` exit 0；**Linux 载体未重 provision**（沿 093…155/170 同口径） |
| 进体字节判据（0am P8 面首次进件） | **锚点名**：`u_verify` 2/2（WIN/LIN）、`v_verify` 2/1、`u_ctx`/`v_ctx`/`u_infra`/`v_infra` 各 1/1；**成因段**：`源：` 1/1＋标签环闭集全量在件（验证失败/写控拦截/计划/车道拒绝/门/护栏拒绝/检索启用拒绝/权限拒绝/策略拒绝/其他拒绝/变更成功/间隔失节律/折叠推进/折叠写失败/资源拒绝/限额命中/探针翻转/资源跨档/传输重试＝各 1）；**词表 v2**：`cargo fmt --check`/`pnpm test`/`bun test` 各 1；**179 面**：kill-switch 告警长文案两条各 1（`0/false/no/off disable`＋`keeping the RLI channel family enabled`）；**保留面零回归**：`rli.notice.` 7→7（WIN）/1→1（LIN）、`streak_crossed`/`domain_spike_entry`/`migration_confirmed` 1→1、`coverage_gap` 5→5/6→6、`u_prog` 22→22/32→32、`slow_prog` 3→3、`carrier-write` 1→1；**退役面**：`预算：` 0→0、`rli-shadow-v1` 0→0；`ORZ_LIF_RLI_SHADOW` 2→3（双平台各 +1＝179 告警文案携带 env 名，预期） |
| `rli-shadow-v2` 字节面注记 | 双平台连续字节均 **0**——13B schema 常量在 `to_string()` 与 restore 比较两使用点均被编译器拆分内联（170 §2-A imm64 家族、双平台同形）；**功能在件证据**＝同模块锚点名/标签环全量/词表 v2/179 文案全数在件＋schema 写读同源 const（构造性保证，写↔比较不可能失配）；判据工具 [`check_carrier_literals.py`](../../scripts/check_carrier_literals.py)（本批设立、LIFECYCLE 登记）如实记该形态 |
| 版本串判读 | `0.8.12`＝1（WIN）/2（LIN，＝ahash 0.8.12＋orz 本体）；件内 `0.8.11` 命中 14（WIN）/9（LIN）＝**依赖 regex-syntax 0.8.11 的版本串**（Cargo.lock 在案），非 orz 残留——本体身份以 `--build-info`＝0.8.12 双平台为权威 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `c6a0812c…` → **`b141c1ef…`**（注释同步：0.8.12＝源冻结 `54717d06`＝179 批＋bump；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载 rc 0 |
| 冒烟 | Windows `--fake-provider -p hello` 整轮 rc=0（直取退出码；run `RUN-CLI-6ac13438`）；`dogfood_launch -DryRun` 装配断言全过（carrier v0.8.12 在册识别、sha 40BDDA50…） |

## §2 Windows 三件套（终态）

| 文件 | 尺寸 (B)（0.8.11→0.8.12） | SHA256 |
|---|---|---|
| `orz.exe` | 57,101,312（+34,304） | `40bdda5011bfc24c…` |
| `orz-signer.exe` | 6,740,480（Δ0） | `860c9860eb8551f6…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `06b7e693df5fc5d2…` |

（换装基线＝0.8.11 三件 `ff22cbc1…`/`9f1e2889…`/`32d494a5…` 与 170 批账面逐位吻合。）

## §3 Linux musl 三件套（终态）

| 文件 | 尺寸 (B)（0.8.11→0.8.12） | SHA256 |
|---|---|---|
| `orz` | 115,569,504（+53,488） | `b141c1ef5d62812863f036a760d7a7e3a0d22cea54b5f5e1b655d8766433a3d4`（身份门新值） |
| `orz-signer` | 1,397,544（+32） | `a21e1dc58f4c40d7…` |
| `orz-acaf-provision` | 1,216,000（+56） | `8172b2fc9bb354f7…` |

## §6 边界与如实记

1. **未推送未发行**：orz 两提交（`905bc3f5`/`54717d06`）与父仓各批均在本地；不建 Release、不做 rel-stage 打包；发布面仍停 v0.8.7。
2. **Windows 构建为暖缓存增量**（5m04s；176–179 批只触 orz-assurance/orz-loop/orz-bin 面）——非 clean 全量口径，与 170（25m07s）差异如实记；退出码直取未经管道。
3. **Linux 载体未做 ACAF 重 provision**（沿 093/100/109/122/144/147/155/170 同口径）；Windows 在役目录已重 provision。
4. **`.0.8.11-bak` 链（Linux）于容器 cp 步前预建**——`/out` 直写换装位形态下回滚点须先于落盘（本批执行序自检发现并补齐，未造成覆盖）。
5. **冒烟退出码纪律**：首跑经管道取码作废（155 §2-A 教训面），直取重跑 rc=0 方为读数（run `RUN-CLI-6ac13438`；管道首跑 `RUN-CLI-6ac1342e` 同绿但不作判据）。
6. **`rli-shadow-v2` 双平台字节 0**：见 §0 注记——存储形态非功能缺失；后续重建批判据清单继续以「key 串＋键形＋长文案＋标签闭集」为面。
7. **字面量判据工具新设**：`scripts/check_carrier_literals.py`（NEW/RETAINED/GONE 三组；LIFECYCLE `active` 登记随批）；`计划·车道拒绝` 判据串首版笔误（177 批档行文 `·`≠源码 `/`）已随批修正——判据串以源码字面为唯一权威。
8. **0am 线余项**：S3/P8/RS-06 全部进件后，余＝**真机轮 Verify streak 查准/转向相关预注册读数**（178 批登记，随下一真机轮收取）＋θ 衰减地板提案纳入视野；按 2026-09-20 顺序裁决，载体重建后**进 0bc**（RLI 效果观测任务）。

## §7 关联与关键词

[`176 批档`](176_0AM_P8A_STIMULUS_ROUTING_CORE_2026-10-03.md)／[`177 批档`](177_0AM_P8B_BUS_CAUSE_RS06_2026-10-03.md)／
[`178 批档`](178_0AM_REVIEW_DISPOSAL_2026-10-03.md)／[`179 批档`](179_RS06_CLOSURE_2026-10-04.md)／
[`170 批档`](170_CARRIER_REBUILD_V0811_0CP_S3_0CN_S3_2026-10-03.md)（同形态先例）／BACKLOG `0am`／TODO `P1-0am`。

关键词：180 批、0.8.12 重建、源冻结 `54717d06`、0am P8 面首次进件、`b141c1ef…` 身份门、
ACAF 重 provision 860c9860、signer 哈希变化（179 传播）、static-pie×3、alpine/bookworm 双冒烟 0.8.12、
成因段标签环在件、词表 v2 在件、kill-switch 告文案在件、rli-shadow-v2 拆分内联注记、
regex-syntax 版本串判读、打包顺延、未推送未发行、计数 57。
