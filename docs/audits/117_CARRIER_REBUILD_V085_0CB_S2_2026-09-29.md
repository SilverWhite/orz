# 117 批：0cb S2 全面审查处理与 0.8.5 载体重建（2026-09-29）

> **用户令**：「请对当前实现的0cb S1/S2部分进行全面审查…」→ 三项裁决「Schema直接升0.3吧／
> 实现过严格的部分要进一步收窄／请对审查出的全部问题进行处理」→「请进行重建吧」。
> **本批**＝0cb 的 S2 审查处理（v2.1）＋S3 双平台载体重建 **0.8.5** 进体。
> **源冻结**＝orz **`15bc1bfb`**（`a8478ff4` S2＋审查处理＋`15bc1bfb` 版本 bump）；**未推送未发行**。
> **计数 57 不变**（`0cb` 维持 `pending`：余 S4＝TB 2.1 整轮 89 题全量重跑读数）。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 全面审查 | 设计/实现/符合性三面＋双子代理（消费面/契约面清扫、触碰面测试复跑）；P1×1＋P2×3＋P3×5；声称读数全部复现（2981/0/6・378・clippy 13/6/99） |
| P1 契约面（用户裁决 schema 升 v0.3） | payload schema 升 **v0.3**（7 现行＋2 legacy 回放专值；v0.2 冻结在盘）；registry 改指；判官/Python 镜像 legacy 豁免（原 block 配对核证、只读非生产集）；legacy fixture＋对拍 legacy 臂 ⇒ 0.8.4- 历史 journal 回放不判红 |
| P2 过严臂收窄 | 规则 2 PhysicalDrive/mkfs 收窄到写侧目标位（读侧 `if=`/镜像文件放行；`/dev/vd`/`/dev/mapper` 登记）；规则 1 补 `ri`＋盘符相对根 `%SystemDrive%` 补全；规则 4 蜂巢目标位判定 |
| P3 | 行数钉补全全封闭表（5＋3 表）；sysctl 写 allow 登记已接受后果；错题解剖引用修正；设计档升 v2.1 |
| 审查处理读数 | orz-tools **2983/0/6**（+2）／orz-assurance **278/0**（对拍含 legacy 臂）／runtime conformance **378/0**／check_repository 除 doctor dirty 预期态零错误／clippy 13/6/99 基线持平／触碰面 fmt 零 diff |
| Windows 重建 | `build_orz.ps1 -Release -Jobs 2` exit 0（**3m38s**）；换装 MATCH 3/3；`--build-info`＝0.8.5；回滚点 `.0.8.4-bak` |
| ACAF 重 provision | exit 0；manifest `binary_sha256=79d9cc3c…` ↔ 换装位逐位一致；keystore 两件（`f37556ab…`/`2aa80cb8…`）逐位未动；旧 manifest 留 `.bak-20260929-117` |
| Linux musl | APT_OK（零代理切换）；直写换装位 MATCH 3/3；static-pie×3＋INTERP=0；alpine/bookworm 双冒烟 0.8.5；墙钟 **34m00s**（orz-tools 基座变更 ⇒ 近全树重编，如实记） |
| 进体判据 | 新规则 id 四值进体（`catastrophic-recursive-delete` 1／`raw-device-write` 2／`boot-firmware-flip` 2／`registry-hive-delete` 1）＋「已越过保底硬边界」文案 +1＋版本串滚动；v1 旧 id（`system-core-write`/`safety-mechanism-flip`）**退出在役二进制**（legacy 回放豁免集在判官 crate 非发行件，如实记） |
| 打包 | `rel-117-stage` 两侧各 5 entries；包内 6/6 MATCH；容器核证 zip 4/4・tar 4/4・包内 build-info＝0.8.5；清单活体两态（干净 0 finding／README+1B 恰 1 条） |
| 台账 | pin `9f12ecc2` → **`15bc1bfb`**；源清单重生成 1506 条 valid；TODO/BACKLOG/索引 v4.85/第二卷 §1.69；门禁除 doctor dirty 预期态零错误 |

## §1 S2 审查处理（v2.1；处置明细）

审查发现与处置的完整记录见设计档 v2.1 §6 与 TODO `P1-0cb` S2 审查处理条目。要点：

- **P1（契约/回放兼容）**：S2 曾把 v0.2 schema 枚举原地改写且判官/镜像无豁免 ⇒ 0.8.4 代际
  历史 journal 回放判红。用户裁决 schema 直接升 v0.3。落地＝新 schema 文件（enum 9 值，
  legacy 两值描述层标注回放专值）＋registry 单一权威改指＋`WRITE_CONTROL_LEGACY_RULE_CATEGORIES`
  （Rust `families.rs` / Python `run_event_journal_validation.py` 同形；配对核证与现行集同表
  chained；错误文案不变）＋fixture `write-control-review.legacy-block.valid.json`（生成器同批）
  ＋对拍 corpus `write_control_review_ok` 追加 legacy 臂（双判官同语料）＋check_repository/
  conformance 约定测试同步（显式 v0.3 覆盖表）。
- **P2×3（过严臂收窄，用户裁决）**：见 §0 表。测试同步＝`ri` 负向例、盘符相对根 block/
  子目录放行例（cfg windows）、raw 读侧/镜像文件放行集、reg 数据值放行例、行数钉 6→7・4→5
  与新增 5 表钉。
- **P3×5**：钉补全／sysctl 后果登记（设计档 §2 条 2）／解剖引用修正（BACKLOG「（错题解剖）」
  →设计档头部；TB21 §8 标注窗口总账）／设计档 v2.1／TODO·BACKLOG 同步。

## §2 源冻结与提交

| 项 | 值 |
|---|---|
| orz 提交 1 | `a8478ff4` `feat(0cb S2 写控保底化): 五条灾难 block 规则封闭枚举…`（6 文件，+828/−278） |
| orz 提交 2 | `15bc1bfb` `chore(release): bump version 0.8.4 -> 0.8.5（0cb S2 写控保底化载体重建源冻结）` |
| 文件 | `crates/orz-bin/Cargo.toml` 0.8.4 → 0.8.5；`Cargo.lock` orz-bin 条目同步 |
| 校验 | `cargo metadata --locked` exit 0 |
| 推送 | **未推送**（orz 分支 ahead：`a8478ff4`＋`15bc1bfb`） |

## §3 Windows 重建与换装

- 构建：`scripts/build_orz.ps1 -Release -Jobs 2` **exit 0（3m38s）**；警告 1 条
  （`orz-host` unused import，既有面）。
- 换装：三件 → `D:\tb-eval\orz-windows\`（源＝`orz/target/release` 同树产物），
  现役件预置 `.0.8.4-bak` 链后覆写；陈旧 `.0.8.3-bak`×3 移除；`--build-info`＝
  `version=0.8.5 os=windows arch=x86_64 profile=release`（exit 0）。

| 文件 | SHA256 |
|---|---|
| `orz.exe` | `8884862852fd444b5c5e10d521c306ac3225791b8fd6f1d5ecf111ba9ccb815d` |
| `orz-signer.exe` | `79d9cc3c3ae7e13bda5db04515c68d0c6bdbcaefd7d0047a4b9a64b9b750c190` |
| `orz-acaf-provision.exe` | `53ce64bc554c4ea992766a8f1b26fa1a928daf5a8f10460f9267b847cc83647e` |

## §4 ACAF 重 provision（Windows 在役目录）

旧 manifest 留 `signer-manifest.json.bak-20260929-117`；换装位 `orz-acaf-provision.exe`
`<acaf>\keystore <acaf>\signer-manifest.json` **exit 0**；manifest 回显
`binary_sha256=79d9cc3c…` ↔ 换装位 `orz-signer.exe` 逐位一致；keystore 两件哈希前后逐位
未动（`f37556ab…`／`2aa80cb8…`）——内层 `<acaf>\keystore` 正典 again 成立（113/116 批同读数）。

## §5 Linux musl 三件套（Docker，直写换装位）

- 预检 `APT_OK`（零代理切换）。**构建摩擦（如实记）**：首次启动即败——Git Bash MSYS
  路径改写把 `-w /orz/orz` 改成 `B:/Git/orz/orz`（此前批次的 docker 构建未在 Git Bash
  直跑过或 PATH 序不同）；`MSYS_NO_PATHCONV=1` 后一次通过，登记为环境摩擦非脚本缺陷。
- 构建：`rust:1.97-slim`＋`build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约）**exit 0；
  墙钟 34m00s**（含 apt/rustup/静态 rg；`/target` 缓存虽经 116 批预热，本批 orz-tools
  基座 crate 变更 ⇒ 下游近全树重编，实际耗时接近 116 冷构建口径，如实记）。
- 换装：旧三件预置 `.0.8.4-bak` 链后直写（陈旧 `.0.8.2-bak`×3 移除），产物与
  `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**；三件均 **static-pie
  linked**＋**INTERP=0**（alpine 3.20 file/binutils）；alpine/bookworm 双冒烟
  `version=0.8.5`（exit 0）。

| 文件 | 尺寸 (B) | 与 0.8.4 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 115,561,208 | +4,952 B | `ecf1d6650dd7da5b20be484f9b25eea09d35ace440e1808e22a91db0155e6007` |
| `orz-signer` | 1,401,488 | +8 B | `9f0b925d889fb9874d25f96ab3b89ba36e6c7f95d822fae497731c7dd9e14cec` |
| `orz-acaf-provision` | 1,220,104 | −16 B | `28d690bc921f8af95b4894f295267c2564557011433a20b7553f3ec0cb44cbcd` |

## §6 进体判据（字面量核证；对照＝`.0.8.4-bak` 三件合流）

| 字节模式 | Windows 0.8.4 → 0.8.5 | Linux 0.8.4 → 0.8.5 | 判定 |
|---|---:|---:|---|
| `catastrophic-recursive-delete` | 0 → **1** | 0 → **1** | 规则 1 进体 ✅ |
| `raw-device-write` | 0 → **2** | 0 → **2** | 规则 2 进体 ✅ |
| `boot-firmware-flip` | 0 → **2** | 0 → **2** | 规则 3 进体 ✅ |
| `registry-hive-delete` | 0 → **1** | 0 → **1** | 规则 4 进体 ✅ |
| `carrier-write` | 1 → 1 | 1 → 1 | 规则 5 语义保持 ✅ |
| `已越过保底硬边界` | 0 → **1** | 0 → **1** | block 文案 ✅ |
| `0.8.5`（版本串） | 1 → 2 | 2 → 3 | 版本字面量 ✅ |
| `0.8.4`（残留） | 8 → 7 | 5 → 4 | 历史字符串滚动，如实记 |
| `system-core-write` | 1 → **0** | 1 → **0** | v1 旧 id 退出在役件 ✅（legacy 回放豁免集在判官 crate `families.rs`，非发行件——如实记） |
| `safety-mechanism-flip` | 2 → **0** | 2 → **0** | 同上 |

## §7 打包与核证（`rel-117-stage`）

- `.tmp-b117-package.ps1`（沿 113/116 形态：清单由打包阶段按包内实际内容生成、tar 显式
  走 `C:\Windows\System32\tar.exe`）；`README.md`＝`.tmp-b117-readme.md`（新增 0.8.5 段：
  写控保底化四节）；两侧各 5 entries；包内六件与在役载体 **6/6 MATCH**。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.5-windows-x86_64.zip` | 28,012,542 | `d1b7e8a5da9262cb22959b748e4f8652b3a6171a4346fef16e5dd0b38603bbb5` |
| `orz-0.8.5-linux-x86_64.tar.gz` | 37,189,968 | `61cb0a68a8f35b9b14ab66e49887dd8ad1521658e79674d53d82fcd0bef774cb` |
| `SHA256SUMS`（顶层） | 191 | 上述两行（LF、`hash *name`） |

- **容器核证**（alpine 3.20 解包 `sha256sum -c`）：zip **4/4 OK**／tar **4/4 OK**；
  包内 `--build-info`（tar 侧 `orz`）读 **`version=0.8.5`**（exit 0）。
- **清单活体两态**（干净重解压树、`./orz rollback list`）：解压态 **0 条**
  carrier-integrity finding；`README.md` 追加 1 字节后**恰 1 条**＝
  `README.md: 长度不符（清单 35136 ≠ 实际 35137）` ✓。

## §8 台账同步

- pin `9f12ecc2` → **`15bc1bfb`**；`orz_source_manifest.sha256` 重生成 **1506 条**
  （`Cargo.toml`／`Cargo.lock` 哈希更新；`--check` valid）。
- TODO：`P1-0cb` S2 审查处理条目新增＋S3 勾选＋首部 P1 汇总行更新。
- BACKLOG：`0cb` 批序/状态行更新（S2 审查处理＋S3 达成；余 S4）。
- 索引：头行 v4.84 → **v4.85**（本批 117 摘要）＋§1 `DESIGN-WRITE-CONTROL-BACKSTOP`
  条目 S2/S3 达成注记。
- 第二卷：§1.69。
- 门禁：`check_repository.py` error_count=1（仅 doctor「orz submodule working tree is
  dirty」预期态——本批已提交 orz，父仓未提交为剩余来源，提交批消除）。

## §9 边界与如实记

1. **未推送未发行**：父仓与 orz 提交均在本地；GitHub Release 不建（沿 113/116 惯例，
   随推送批一并）。
2. **Linux 构建墙钟 34m00s**：`/target` 缓存虽在，orz-tools（基座）变更触发下游
   近全树重编——与本批触碰面（orz-tools/orz-assurance/orz-host）一致，非异常。
3. **MSYS 路径改写**：Git Bash 直跑 docker `-w /orz/orz` 被改写为 Windows 路径致首次
   启动失败；`MSYS_NO_PATHCONV=1` 解决，登记环境摩擦。
4. **v1 旧规则 id 退出在役二进制**：0.8.4 的 `system-core-write`×1/`safety-mechanism-flip`×2
   来自 v1 `exec_policy` 规则表（生产面）；0.8.5 起生产面只有五条 v2 规则 id，legacy 回放
   豁免集位于判官 crate（`families.rs`，非发行件）——回放兼容不受影响。
5. **既有红不动**：orz-loop `user_cancel_closes…`（时序敏感）与 orz-host 两 journal 链
   测试为干净树同红的既有面（S2 审查已归因）；本批触碰面零新增失败。

关键词：0cb S2、写控保底化、schema v0.3、legacy 回放豁免、过严臂收窄、目标位判定、
0.8.5 重建、117 批、未推送未发行、计数 57。
