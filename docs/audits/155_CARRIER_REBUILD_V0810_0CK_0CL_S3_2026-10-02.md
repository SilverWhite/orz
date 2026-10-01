# 155 批：0.8.10 双平台载体重建进体——0ck/0cl S3 达成（2026-10-02）

> **用户令**：「请进行重建吧」（承接同日 153 批 0ck 落码＋154 批 0cl 落码）。
> **本批**＝0.8.10 代窗口双平台载体重建（沿 117/121/144/147 批形态：源冻结→双平台重建换装→进体
> 字节判据→ACAF 重 provision→身份门换装→冒烟→落账）。
> **源冻结**＝orz **`55d61c47`**（`d5632248` 153/154 批落码＋`41ab8f8e` bump 0.8.9→0.8.10＋
> `55d61c47` 重建判据补笔，见 §2-A）；`cargo metadata --locked` exit 0；未推送。
> **打包顺延**：本批不做 rel-stage 打包（0ck/0cl S4 真机核证未完、无发行压力；147 批打包系
> 0cf/0cg 随批闭合所需在役证据面——偏差如实记，打包随发行批一并）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结 | orz `d5632248`（153/154 落码，2 files +70/−103）→ `41ab8f8e`（bump 0.8.9→0.8.10，Cargo.toml＋Cargo.lock 两行）→ `55d61c47`（判据补笔，见 §2-A）；未推送（分支 ahead 9 commits） |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2`：**初建 exit 假 0 如实记**（rustc 0xc0000409 崩 orz_tui＝148 批先例形态；**退出码被管道 tail 掩蔽**——方法学注记＝后台构建不得经管道取码）→ 重试 **Finished 1m28s exit 0**；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、回滚点 `.0.8.9-bak` 链；`--build-info`＝`0.8.10 os=windows` exit 0；载体清单刷新 0.8.10（generate_carrier_manifest.py，3 entries） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261002-155`；provision exit 0；`binary_sha256=b4b5d87b…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件（`f37556ab…`/`2aa80cb8…`）前后逐位未动（内层正典 again） |
| Linux musl | docker `rust:1.97-slim`＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001，`MSYS_NO_PATHCONV=1`；**首挂中途叫停如实记**＝§2-A 判据拦截后修复重跑）二建 **exit 0**；`/out` 直写换装位（`.0.8.9-bak` 链），产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；Python ELF 解析＝三件 **ET_DYN（PIE）＋PT_INTERP=0（static-pie）**；alpine 3.20/bookworm 双冒烟 `version=0.8.10 os=linux` exit 0 |
| 进体字节判据 | 两平台同形（旧 0.8.9 → 新 0.8.10）：**`RLI_ANNO`（0ck 注解句）0→5**、GUIDE_NOUNS（名词解释与组件关系）0→5、ANCHOR_PARAM（Journal locator）0→5、**`OCF_OLD_GONE`（0cf 英文旧句）5→0**、版本串 0.8.10=1/残留 0.8.9 仅注释（WIN 11→10・LIN 8→7）、保留面零回归（WIN `/dev/null` 3→3・`carrier-write` 1→1；LIN `/dev/null` 46→46・`carrier-write` 1→1）、JOURNAL_LOC 27→37（定位符语法进新描述＋guide） |
| §2-A 判据拦截事件 | 初建 Windows 件字节判据 **`RLI_ANNO`=0**——0cl 瘦身整段替换描述时**静默丢失 0ck 注解句**；字节判据当场拦截（换装前），修复 `55d61c47` 后双平台重build——**0ck S3 判据方法论价值实证** |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `4e35f410…` → **`4ed531e8…`**（注释同步：0.8.10＝源冻结 `55d61c47`；适配器 `6d55c26e…` 未动）；语法解析＋`--help` 加载通过 |
| 冒烟 | Windows `--fake-provider -p hello` 整轮 rc=0（run `RUN-CLI-6abec694`）；Linux alpine 3.20/bookworm `--build-info` 双绿 |
| 台账 | 本档；BACKLOG/TODO（0ck・0cl S3 勾选、指针行）；索引 v4.123 → **v4.124**（头行）；第二卷 §1.107 |

## §2-A 初建判据拦截事件（本批核心事件，如实记）

初建 Windows 件（`fa432e5e…`）字节判据 `RLI_ANNO`=0——**0cl 瘦身以新描述整段替换时，把 153 批
0ck 注解句一并替换丢失**（新 rli 条目只写面描述、无使用指引句）。拦截点＝换装前字节判据；处置＝
源修复 `55d61c47`（rli 条目后补回用户裁决句原文）→ Windows 重试重建（1m28s）→ Linux 首挂中途
叫停重跑——两平台终态件 `RLI_ANNO`=5。**教训双条**：① 整段替换类改动必须携前批新增面清单核对；
② 后台构建退出码不得经管道 `tail` 取（首建 rustc 0xc0000409 崩 orz_tui＝已知形态、重试过，
但管道掩蔽使假 exit 0 未被立即识别，二建才发现）。

## §2 Windows 三件套（终态）

| 文件 | 尺寸 (B)（0.8.9→0.8.10） | SHA256 |
|---|---|---|
| `orz.exe` | 57,053,696（−62,464） | `615e34724734e6a1…` |
| `orz-signer.exe` | 6,740,480（Δ0） | `b4b5d87b18fae785…` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `f23cc362028373a2…` |

（换装基线＝0.8.9 三件 `fadb4f13…`/`fc38e48d…`/`5f31bf0d…` 与 147 批账面逐位吻合。）

## §3 Linux musl 三件套（终态）

| 文件 | SHA256 |
|---|---|
| `orz` | `4ed531e837df68896a8d5c340e1df42a54fa9d63aa96de8bd91705eb7903b310`（身份门新值） |
| `orz-signer` | `63c9856537667d25…` |
| `orz-acaf-provision` | `3668749ffd145bb6…` |

## §6 边界与如实记

1. **未推送未发行**：orz 三提交（`d5632248`/`41ab8f8e`/`55d61c47`）与父仓均在本地；不建 Release；发布面仍停 v0.8.7。
2. **打包顺延**（见档头）——rel-stage 随发行批。
3. **Linux 载体未做 ACAF 重 provision**（沿 093/100/109/122/144/147 同口径）；Windows 在役目录已重 provision。
4. **`--version` 完整性核证通道**：Windows 无 tty 管道落 TUI 挂起（147 §6.5-③ 已知形态，stdin=NUL 仍挂）——完整性以**构造性保证**替代（manifest 由同批换装后文件生成、3 entries rc 0）＋`--build-info` rc 0。
5. **Windows 初建假 exit 0**：rustc 0xc0000409（orz_tui，已知闪退家族）＋管道掩蔽——重试即过；方法学注记见 §2-A。
6. **0ck/0cl 不闭合**：S3 已达成（本批）；S4＝真机核证（消费率读数随 0cj S3/S4 收取），两项维持开放（计数 59 不变）。

## §7 关联与关键词

[`153 批档`](153_RLI_TOOL_ANNOTATION_2026-10-02.md)／[`154 批档`](154_BLACKBOARD_FACE_SLIMMING_2026-10-02.md)／
[`147 批档`](147_CARRIER_REBUILD_V089_0CF_0CG_2026-10-01.md)（同形态先例）／BACKLOG `0ck`/`0cl`／TODO `P1-0ck`/`P1-0cl`。

关键词：155 批、0.8.10 重建、源冻结 `55d61c47`、字节判据拦截 0ck 注解丢失、`4ed531e8…` 身份门、
ACAF 重 provision b4b5d87b、static-pie×3（ELF 解析口径）、alpine/bookworm 双冒烟 0.8.10、
打包顺延、未推送未发行。
