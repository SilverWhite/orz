# 144 批：0ch S3 双平台载体重建 0.8.8 进体（2026-10-01）

> **用户令**：「请进行重建吧」（承接同日 143 批 0ch S1/S2 全面审查通过＋P2×1＋P3×4 处置完毕）。
> **本批**＝0ch 的 S3 双平台载体重建 **0.8.8** 进体（沿 117/121 批形态：源冻结→Windows 重建换装
> ＋ACAF 重 provision→Linux musl docker 直写→进体判据→身份门换装→打包核证→落账）。
> **源冻结**＝orz **`2da7dba0`**（`60b21a37` 0ch S2＋审查处理＋`2da7dba0` 版本 bump 0.8.7→0.8.8）。
> **未推送未发行**（沿 113/116/121 惯例，随推送批一并）。
> **计数 62 不变**（`0ch` 维持 `pending`：余 S4＝`configure-git-webserver`／`caffe-cifar-10`／
> `git-multibranch` 三题重跑复验）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结 | orz `60b21a37`（feat 0ch S2，5 文件 +608/−170）＋`2da7dba0`（bump，Cargo.toml＋Cargo.lock 两行）；`cargo metadata --locked` exit 0；未推送 |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` **exit 0（2m49s）**；唯一警告＝orz-host unused import（既有面）；换装 `D:\tb-eval\orz-windows\` **MATCH 3/3**、回滚点 `.0.8.7-bak` 链；`--build-info`＝`0.8.8 os=windows`（exit 0） |
| ACAF 重 provision | 旧 manifest 留 `signer-manifest.json.bak-20261001-144`；换装位 provision **exit 0**；manifest `binary_sha256=f651f7ba…` ↔ 换装位 `orz-signer.exe` 逐位一致；**keystore 两件（`f37556ab…`/`2aa80cb8…`）前后逐位未动** |
| Linux musl | docker `rust:1.97-slim`＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约 `MSYS_NO_PATHCONV=1`，仓库副本＝tb-eval 副本 diff 一致）**exit 0（约 23 分钟）**；`/out` 直写换装位（旧件预置 `.0.8.7-bak` 链），产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；alpine 3.20 `file`：三件 **static-pie linked**＋**INTERP=0**；alpine/bookworm 双冒烟 `version=0.8.8 os=linux` |
| 进体判据（Windows 件） | 版本串滚动（`0.8.8` 12→13＋1、`0.8.7` 8→7−1）；五规则 id（`catastrophic-recursive-delete` 1／`raw-device-write` 2／`boot-firmware-flip` 2／`registry-hive-delete` 1／`carrier-write` 1）与保底文案「已越过保底硬边界」逐位不变。**如实记**：`DEVICE_SAFE_NODES` 七串与 0cd 退役串（`You are an AI coding agent…`）在 Windows 件两侧均为 0——表唯一消费点 `attach_child_write_guard` 为 `cfg(linux)`，Windows 侧死引用被链接器消除（0cd 常量在 0.8.7 Windows 件即不可达，同理），**0ch 字面量判据的主战场在 Linux 件** |
| 进体判据（Linux 件；Python 逐字节权威核证） | `DEVICE_SAFE_NODES` 全表 7 项可达：`/dev/zero` 0→**1**、`/dev/full` 0→**1**、`/dev/ptmx` 0→**1**、`/dev/urandom` 1→**2**、`/dev/random` 5→**6**、`/dev/null` 45→**46**、`/dev/tty` 1→1（表项与既有串 linker 串合并复用）；`/dev/` 前缀族合计 69→**75**（＋6）✅；版本串滚动；保底文案在位。**方法学如实记**：MSYS `grep -a -o` 对大二进制的计数不可靠（`/dev/zero` 等读 0），以 Python `bytes.count` 定案 |
| 身份门换装 | `run_r0_heavy_official.py` `EXPECTED_CARRIER_SHA256` `3332b38f…` → **`874df6ca…`**（注释行同步：0.8.8＝0ch S3 源冻结 `2da7dba0`；适配器 `6d55c26e…` 未动＝`tb_agents/orz.py` 无改动）；语法解析＋`--help` 加载通过 |
| 打包 | `rel-144-stage` 两侧各 **5 entries**（三件套＋`README.md`＋`SHA256SUMS`；`carrier-manifest.json` 由打包阶段按包内实际内容生成，`kind=orz-carrier-manifest version=0.8.8`）；包内六件与在役载体 **6/6 MATCH**；容器核证（alpine 3.20）zip **4/4**・tar **4/4**・顶层 **2/2**、包内 build-info（tar 侧 in-container）＝**0.8.8**；**清单活体两态**（`--version` 通道、stderr 落被检目录外，沿 121 批勘误）＝解压态干净 **0 finding**／README+1B **恰 1 条**＝`长度不符（清单 45665 ≠ 实际 45666）` ✓ |
| 台账 | 源清单重生成；索引头行 v4.112 → **v4.113**；TODO `P1-0ch` S3 勾选；BACKLOG `0ch` 状态推进（余 S4）；第二卷 §1.96；本档；`check_repository.py` 门禁收口态见 §6 |

## §1 源冻结与提交

| 项 | 值 |
|---|---|
| orz 提交 1 | `60b21a37` `feat(0ch S2 L3 粒度精准化): DEVICE_SAFE_NODES 恰 7＋/ 根 ROOT_MAKE_GRANT 恰 5 位＋装挂三段规则…`（5 文件，+608/−170；含同批审查处理 P3-1/P3-3/P3-4 修复） |
| orz 提交 2 | `2da7dba0` `chore(release): bump version 0.8.7 -> 0.8.8（0ch S3 L3 粒度精准化载体重建源冻结）` |
| 文件 | `crates/orz-bin/Cargo.toml` 0.8.7 → 0.8.8；`Cargo.lock` orz-bin 条目同步 |
| 校验 | `cargo metadata --locked` exit 0 |
| 推送 | **未推送**（orz 分支 ahead：`60b21a37`＋`2da7dba0`） |
| 随行面 | 0cd（`20e4c574` 死代码清退）已在冻结点内——其「源态前移随下次重建进体」承诺本批兑现；0ce 暂缓、0cf/0cg 未实施＝**无代码随行**（同窗落点顺延登记见 §6） |

## §2 Windows 重建与换装

- 构建：`scripts/build_orz.ps1 -Release -Jobs 2` **exit 0（2m49s）**；警告 1 条
  （`orz-host` unused import，既有面）。
- 换装：三件 → `D:\tb-eval\orz-windows\`（源＝`orz/target/release` 同树产物），
  现役件预置 `.0.8.7-bak` 链后覆写（沿 121/122 不清陈旧 bak 链实践）；
  `--build-info`＝`version=0.8.8 os=windows arch=x86_64 profile=release`（exit 0）。

| 文件 | 尺寸 (B)（0.8.7→0.8.8） | SHA256 |
|---|---|---|
| `orz.exe` | 57,117,696（Δ0） | `82c37eaa7eee76443a4718f780f622b969b0e32ecf557366af16c687d98b3583` |
| `orz-signer.exe` | 6,740,480（Δ0） | `f651f7ba762e8d80e509a528d3b7f304f9bd2866256cf6d30d7ef8274d89d6c1` |
| `orz-acaf-provision.exe` | 6,640,128（Δ0） | `3c520cdfdfe7903f15e5446895b9f4bc7bdbcb9a76bd0b4686e491ed725463c3` |

（现役 0.8.7 三件换装前哈希 `104b9bce…`/`ed8066b7…`/`ebf5b3f9…` 与 122 批账面逐位吻合＝
换装基线实证；Linux 侧同（`3332b38f…`/`1defacf8…`/`bc7d5576…`）。）

## §3 ACAF 重 provision（Windows 在役目录）

旧 manifest 留 `signer-manifest.json.bak-20261001-144`；换装位 `orz-acaf-provision.exe`
`<acaf>\keystore <acaf>\signer-manifest.json` **exit 0**；manifest 回显
`binary_sha256=f651f7ba…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件哈希前后逐位
未动（`f37556ab…`／`2aa80cb8…`）——内层 `<acaf>\keystore` 正典 again 成立（113/116/121 批同读数）。

## §4 Linux musl 三件套（Docker，直写换装位）

- 构建前核验：`scripts/build_orz_aliyun_trixie.sh` 与 `D:\tb-eval\` 副本 **diff 一致**。
- 构建：`rust:1.97-slim`＋ORZ-BUILD-MOUNT-001 契约（`MSYS_NO_PATHCONV=1`；父仓挂 `/orz`、
  工作区 `/orz/orz`、`/target` 缓存、`/out`＝换装位直写）**exit 0；约 23 分钟**（`-j 1`；
  orz-tools 基座变更 ⇒ 下游近全树重编，与 117 批口径一致，如实记）。
- 换装：旧三件预置 `.0.8.7-bak` 链后脚本直写，产物与
  `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；alpine 3.20
  `file`：三件均 **static-pie linked**＋**INTERP=0**；alpine/bookworm 双冒烟
  `version=0.8.8 os=linux`（exit 0）。

| 文件 | 尺寸 (B)（0.8.7→0.8.8） | SHA256 |
|---|---|---|
| `orz` | 115,560,976（−544） | `874df6cac1dca4ac0f72de706e54c080a30687fff3cbda0b8cfb9256f0d85fb4` |
| `orz-signer` | 1,401,592（+72） | `8991589eda6117c16d43e376b3a97a6a7883766036a6a1ac63b083870786033d` |
| `orz-acaf-provision` | 1,220,184（+32） | `87c88423d0e3a242448776ecdd9d50e3f3f546f254bde059e67571870178ba80` |

## §5 进体判据

### Windows 件（MSYS grep 计数；版本串/规则 id 面）

| 字节模式 | 0.8.7 → 0.8.8 | 判定 |
|---|---|---|
| `0.8.8`（版本串） | 12 → **13** | 版本字面量 ✅ |
| `0.8.7`（残留） | 8 → 7 | 历史字符串滚动，如实记 |
| 五规则 id（`catastrophic-recursive-delete` 1／`raw-device-write` 2／`boot-firmware-flip` 2／`registry-hive-delete` 1／`carrier-write` 1） | 不变 | L1/L2 契约零变化 ✅ |
| `已越过保底硬边界` | 1 → 1 | block 文案在位 ✅ |
| `DEVICE_SAFE_NODES` 七串 | 0 → 0 | **cfg(linux) 消费点缺失 ⇒ 死引用消除**，如实记（判据主场在 Linux 件） |
| `You are an AI coding agent`（0cd 退役串） | 0 → 0 | 0.8.7 即不可达（从未进入 Windows 件），如实记 |

### Linux 件（Python `bytes.count` 逐字节权威核证）

| 字节模式 | 0.8.7 → 0.8.8 | 判定 |
|---|---|---|
| `/dev/zero` | 0 → **1** | 表项进体 ✅ |
| `/dev/full` | 0 → **1** | 表项进体 ✅ |
| `/dev/ptmx` | 0 → **1** | 表项进体 ✅ |
| `/dev/urandom` | 1 → **2** | 表项进体 ✅ |
| `/dev/random` | 5 → **6** | 表项进体 ✅ |
| `/dev/null` | 45 → **46** | 表项进体 ✅ |
| `/dev/tty` | 1 → 1 | 表项与既有串 linker 串合并复用 ✅（全表 7 项均可达） |
| `/dev/`（前缀族合计） | 69 → **75**（＋6） | 全表净增印证 ✅ |
| `You are an AI coding agent`（0cd 退役串） | 0 → 0 | 两侧均 0——0cd 常量全周期未达二进制（纯死代码），如实记 |

## §6 边界与如实记

1. **未推送未发行**：父仓与 orz 提交均在本地；GitHub Release 不建（沿 113/116/121 惯例，
   随推送批一并）；发布面仍停 v0.8.7。
2. **Linux 载体未做 ACAF 重 provision**（沿 093/100/109/122 同口径）；Windows 在役目录已重 provision。
3. **Windows 件字面量判据的平台性**：0ch 全部变化位于 L3（Landlock，仅 Linux）；Windows 件
   的进体证据＝版本串滚动＋L1/L2 面零变化＋行为面（L3）不存在——与设计「不动 Windows 语义」一致。
4. **0ce／0cf／0cg 同窗落点顺延**：0ch S3 原记「与 0ce／0cf／0cg 同窗」——本批 0.8.8 窗口
   实际只载 0cd＋0ch（0ce 暂缓不变、0cf/0cg 未实施，三者均无代码随行）；三者实施落点顺延至
   **下一代窗口（0.8.9）**，BACKLOG/TODO 对应节已同步注记。
5. **方法学两则**：① MSYS `grep -a -o` 对大体积二进制的模式计数不可靠（`/dev/zero` 等误读 0），
   本批字面量核证以 Python `bytes.count` 定案；② 清单活体核证须在 Linux 容器内做（Linux 件
   不能于 Windows 宿主直跑），`--version` 通道触发完整性自检、stderr 重定向落被检目录外
   （沿 121 批勘误）——解压态 exit=1 为容器无 tty 的 `tui io error`（两态同现），判据只看
   carrier-integrity finding 有无。
6. **S4 未跑**：`configure-git-webserver`／`caffe-cifar-10`／`git-multibranch` 三题重跑复验
   随下一批（0.8.8 已在役＝重跑线解禁）。

## §7 关联与关键词

[`0ch 设计档 v4.0 §7`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)／
[`117 重建档`](117_CARRIER_REBUILD_V085_0CB_S2_2026-09-29.md)／
[`122 发行档`](122_SUBMIT_PUSH_AND_RELEASE_2026-09-30.md)／
BACKLOG `0ch`／TODO `P1-0ch`。

关键词：144 批、0ch S3、0.8.8 重建、源冻结 `2da7dba0`、DEVICE_SAFE_NODES 进体、
ROOT_MAKE_GRANT、`874df6ca…` 身份门、rel-144-stage、static-pie×3、清单活体两态、
0cd 进体、0.8.9 顺延、未推送未发行、计数 62 不变。
