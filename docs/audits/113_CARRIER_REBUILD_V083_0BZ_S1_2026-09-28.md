# 113 批：0.8.3 载体重建（双平台 · 0bz S1 指纹件进体）

> **结论先行**：0bz 的 S1（模型面前缀指纹观测件，orz `7a6fe91e`）已**随载体进体**——双平台
> **0.8.3** 重建完成并换装，Windows 在役目录 ACAF 重 provision 后 manifest 绑定新签名器，
> 包内两侧各 5 entries 含载体清单；容器核证全 OK（zip 4/4・tar 4/4・顶层 2/2），包内
> `--build-info` 双平台读 `version=0.8.3`。**未推送、未发行**（用户令「暂不推送」；
> GitHub Release 留待推送批一并）。**计数 56 不变**，`0bz` 维持 `pending`（S2 余项＝真机
> 单轮采集，载体前置已达成）。

> **日期**：2026-09-28；**用户令**：「暂不推送，请进行重建吧」。
> **进体内容**＝0bz S1 指纹件（orz `7a6fe91e`：事件族合约＋生产者＋法官族＋三钉，
> 见 [`111 批档`](111_0BZ_S1_FINGERPRINT_IMPLEMENTATION_2026-09-28.md)）＋版本 bump 两行。
> **源冻结**＝orz HEAD **`778fad39`**（pin `7a6fe91e` → `778fad39`）。

---

## §0 结论速览

| 面 | 结果 |
|---|---|
| 源冻结 | orz `778fad39`（`bump version 0.8.2 -> 0.8.3`，`cargo metadata --locked` exit 0） |
| Windows | `build_orz.ps1 -Release -Jobs 2` **exit 0（4m36s）**；换装 MATCH 3/3；`--build-info`＝`version=0.8.3` |
| ACAF 重 provision | exit 0；manifest `binary_sha256=71371a32…` ↔ 换装位逐位一致；keystore 两件逐位未动 |
| Linux musl | 预检 `APT_OK`（零代理切换）；cargo **8m04s**；直写换装位 MATCH 3/3；static-pie＋INTERP=0；alpine＋bookworm 冒烟 exit 0 |
| 进体判据 | `face_fingerprint` 字面量：win **0 → 2**／linux **0 → 3**；版本字面量同步 |
| 打包 | 两侧各 5 entries；包内 6/6 MATCH；容器核证 zip 4/4・tar 4/4・顶层 2/2；清单活体两态 ✓ |
| 发行 | **无**（未推送未发行；本地重建为 S2 真机采集前置） |
| 台账 | pin `778fad39`；源清单重生成 1506 条；门禁 `valid: true` |

## §1 版本冻结

| 项 | 值 |
|---|---|
| orz 提交 | `778fad39` |
| 主题 | `chore(release): bump version 0.8.2 -> 0.8.3（0bz S1 指纹件载体重建源冻结）` |
| 文件 | `crates/orz-bin/Cargo.toml` 0.8.2 → 0.8.3；`Cargo.lock` orz-bin 条目同步 |
| 校验 | `cargo metadata --locked` **exit 0** |
| 推送 | **未推送**（用户令；orz 分支 ahead 2＝`7a6fe91e`＋`778fad39`） |

## §2 Windows 重建与换装

- 构建：[`scripts/build_orz.ps1`](../../scripts/build_orz.ps1) `-Release -Jobs 2` **exit 0（4m36s）**；
  装配清单：12 核／物理 15.8 GiB（余 4.7）／commit 25.9 GiB（余 4.0，使用 84.6%）／`PROTOC` 脚本自装配。
- 警告 **1 条**＝`orz-host` unused import（`permission.rs` Path/PathBuf，既有面，非本批引入）。
- 换装：三件 → `D:\tb-eval\orz-windows\`（源＝`orz/target/release` 同树产物），现役件预置
  `.0.8.2-bak` 链后覆写；`--build-info` 换装位读数
  **`version=0.8.3 os=windows arch=x86_64 profile=release`**（exit 0）。

| 文件 | 尺寸 (B) | 与 0.8.2 差异 | SHA256 |
|---|---:|---:|---|
| `orz.exe` | 57,110,528 | +87,552 B | `0799d0531c13793842a847dfdb2dfb7b6ea064013e77bb50ab075fe014725646` |
| `orz-signer.exe` | 6,740,480 | 0（哈希变＝内嵌版本号） | `71371a32d37f5a9feb34a95c74435c806cbd2df6ab776b3b92fbf8f1e5f2f51c` |
| `orz-acaf-provision.exe` | 6,640,128 | 0（同上） | `e021fa5c08d411e74a14172ad526880e34594d3442017fd09c928205b43c356c` |

## §3 ACAF 重 provision（Windows 在役目录）

- 旧 manifest 留 `signer-manifest.json.bak-20260928-113`；`orz-acaf-provision <keystore> <manifest>`
  **exit 0**；manifest 回显 `binary_sha256=71371a32…` ↔ 换装位 `orz-signer.exe` **逐位一致**。
- keystore 两件哈希前后**逐位未动**（`f37556ab…`／`2aa80cb8…`）——内层 `<acaf>\keystore` 正典
 again 成立（109 批同读数）。

## §4 Linux musl 三件套（Docker，直写换装位）

- **预检**（DOCKER_PROXY_RECIPE 第 0 步）：`docker run --rm rust:1.97-slim bash -c "apt-get update -qq
  && apt-get install -y -qq musl-tools"` ⇒ **`APT_OK`** ⇒ 跳过代理切换／还原两段。
- **构建**：`rust:1.97-slim` ＋ [`build_orz_aliyun_trixie.sh`](../../scripts/build_orz_aliyun_trixie.sh)
  （ORZ-BUILD-MOUNT-001 契约、`/target` 缓存沿用）**exit 0（cargo 8m04s，墙钟 ≈12 min 含 apt／rg 静态构建）**；
  换装位旧三件预置 `.0.8.2-bak` 链后直写，产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**。

| 文件 | 尺寸 (B) | 与 0.8.2 差异 | SHA256 | BuildID |
|---|---:|---:|---|---|
| `orz` | 115,556,560 | +96,400 B | `46964d0cd0a2d4e72074cfe548df81e8971c2ecf3602680552c12a1c32f8a009` | `350f335c` |
| `orz-signer` | 1,401,472 | −24 B | `4bee4909f9cd9848fe1b98d594f8f6f0d06a9883a61694a1fc46560eebf02ab0` | `b46df244` |
| `orz-acaf-provision` | 1,220,120 | +8 B | `960c7e23865bbc8477ed04d1fece28ed9e9faf248621c2d04003cb46f72b1f43` | `1d3e060a` |

- **ELF 核验**（alpine 3.20 ＋ `file`／`binutils`）：三件均 **`static-pie linked`**＋**`INTERP` 段 = 0**。
- **双向加载冒烟**：alpine 3.20 与 `debian:bookworm-slim` 内 `orz --build-info` **exit 0** 且读数
  **`version=0.8.3 os=linux arch=x86_64 profile=release`**。
- **警告面**：Linux-only cfg 既有族（`orz-config` 1／`orz-assurance` 4／`orz-host` 3，`unused` 族）——
  本批触碰面（`orz-loop` 指纹件）零新增告警。

## §5 进体判据

### §5.1 字面量核证（出现次数法；对照＝`.0.8.2-bak`）

| 字节模式 | Windows 0.8.2 → 0.8.3 | Linux 0.8.2 → 0.8.3 | 判定 |
|---|---:|---:|---|
| `face_fingerprint` | 0 → **2** | 0 → **3** | 0bz S1 指纹件进体 ✅ |
| `0.8.3`（内嵌版本串） | 17 → 18 | 36 → 37 | 版本字面量 ✅ |
| `0.8.2`（残留） | 2 → 1 | 1 → 0 | 历史字符串，如实记 |
| `face-fingerprint`（slug 连字形态） | 0 / 0 | 0 / 0 | slug 由枚举运行时派生、不落字面量，如实记 |

- 其余历史字面量面本批未触，按 109 档读数沿用；本版进体判据＝**上表＋版本字面量＋源冻结一致性**。

## §6 打包与核证（`D:\tb-eval\rel-113-stage\`）

- 入口 [`.tmp-b113-package.ps1`](../../.tmp-b113-package.ps1)（沿 109 形态：清单**由打包阶段按包内实际内容生成**；
  tar 显式走 `C:\Windows\System32\tar.exe`——MSYS tar 陷阱见 §7）：
  两侧各 **5 entries**＝三件套＋`README.md`＋`SHA256SUMS`；包内六件与在役载体 **6/6 MATCH**。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.3-windows-x86_64.zip` | 28,008,113 | `83df998ac34c18f852c76d289272fa7fe6ee277aa36648ea6014adb0fbb52802` |
| `orz-0.8.3-linux-x86_64.tar.gz` | 37,177,883 | `1890335ac66573c67666a85bd1d1f29ce1e16b2653baf62962d92c069df2eba4` |
| `SHA256SUMS`（顶层） | 191 | 上述两行（LF、`hash *name`）；自身 `sha256=f9ca1df8f6c3a1c843f877c597b812afcc8397d69ecab99c3388e85f062664b9` |

- 包内 `README.md`＝[`.tmp-b113-readme.md`](../../.tmp-b113-readme.md)：新增 0.8.3 段（指纹观测件四节），
  快速开始 tar 名对齐 0.8.3，版本信息行指 `778fad39`。
- **容器核证**（`alpine:3.20` 内解包后 `sha256sum -c`）：zip **4/4 OK**／tar **4/4 OK**／顶层 **2/2 OK**；
  包内 `--build-info`（tar 侧 `orz`）读 **`version=0.8.3`**（exit 0）。
- **清单活体两态**（触发命令＝`orz rollback list`，Linux 包内、干净解压树）：
  解压态 **0 条** carrier-integrity finding ✓；`README.md` 追加一字节后**恰 1 条**＝
  `README.md: 长度不符（清单 29860 ≠ 实际 29861）` ✓ ⇒ 清单对 `README.md`／`SHA256SUMS` 的覆盖在生效。

## §7 边界与如实记

1. **未推送未发行**（用户令「暂不推送」）：父仓 110/111/112/113 四批、orz 两提交（`7a6fe91e`／`778fad39`）
   均在本地；GitHub Release 不建——tag 需指向父仓提交而父仓提交未推送，发行随推送批一并补。
2. **磁盘事件**：批首 D 盘余 **694 MB**（100%）——清理 `orz/target/debug/incremental`（2.4 GB，可再生，
   111 批同先例）得 3.0 GB；构建全程余量最低 2.2 GB。`release/` 缓存保留（增量重建 4m36s 的前提）。
3. **tar MSYS 陷阱**：PowerShell 内裸 `tar` 本次解析到 Git Bash `/usr/bin/tar`（`D:\` 被当远程主机名
   报 `Cannot connect to D`）——打包脚本改显式 `C:\Windows\System32\tar.exe` 后一次通过；109 批未遇
   （当时 PATH 序不同），如实登记为环境摩擦非脚本缺陷。
4. **清单活体首跑作废**：首次两态核验两处污染——容器 `sh` 的当前目录不在 `PATH`（`orz` 找不到，
   状态判定全空）＋同名 `README.md` 已被追加 1 B（`_verify_tar` 未重建）。`_verify_tar` 重解压后
   以 `./orz` 重跑，§6 读数以重跑为准。
5. **换装 bak 链**：Windows 侧本批新增 `.0.8.2-bak`×3（前链 `.0.8.0/0.8.1-bak` 等保留）；
   Linux 侧新增 `.0.8.2-bak`×3。
6. **既有面不动**：`orz-host` unused import（Windows 1 条／Linux 3 条同族）；`user_cancel_closes`
   flaky 与本批无关。

## §8 台账同步

- pin `7a6fe91e` → **`778fad39`**；`orz_source_manifest.sha256` 重生成 **1506 条**（`Cargo.toml`／
  `Cargo.lock` 两处哈希更新；`--check` 读 `valid`）。
- TODO：`P1-0bz` S2 注记改「载体前置已达成（113 批，0.8.3）；余项＝真机单轮采集（随下一狗粮轮）」＋
  计数行改指本批。
- BACKLOG：`### 0bz.` 进体注记＋计数行/指针改指本批（第二卷 §1.65）。
- 索引：§6 `0bz` 条目＋§8 `pending` 桶行补进体；头行 v4.80 → **v4.81**（v4.80 滚入存档卷）。
- 第二卷：§1.65。
- **门禁**：`python scripts/check_repository.py` ⇒ `valid: true`（error_count 0）。

## §9 关联与关键词

[`110 立项档`](110_CONTEXT_FACE_TRANSIENT_FORK_REGISTRATION_2026-09-28.md)／
[`111 S1 落码档`](111_0BZ_S1_FINGERPRINT_IMPLEMENTATION_2026-09-28.md)／
[`109 重建先例`](109_CARRIER_REBUILD_AND_RELEASE_V082_DUAL_PLATFORM_2026-09-28.md)／
[`DOCKER_PROXY_RECIPE`](../../scripts/DOCKER_PROXY_RECIPE.md)／
[`build_orz.ps1`](../../scripts/build_orz.ps1)／
[`build_orz_aliyun_trixie.sh`](../../scripts/build_orz_aliyun_trixie.sh)／
BACKLOG `0bz`／TODO `P1-0bz`。

关键词：载体重建、0.8.3、0bz S1 进体、双平台、face_fingerprint 字面量、ACAF 重 provision、
static-pie、清单活体两态、MSYS tar 陷阱、未推送未发行、计数 56 不变。
