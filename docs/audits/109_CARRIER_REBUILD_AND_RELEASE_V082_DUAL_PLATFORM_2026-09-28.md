# 109 批：0.8.2 载体重建与发行（双平台 · 0.8.1 → **0.8.2**）

> **结论先行**：0by 的 S1（勘定）＋S2（修复）已**随载体进体**——双平台 **0.8.2** 重建完成并换装，
> Windows 在役目录 ACAF 重 provision 后 manifest 绑定新签名器，包内两侧各 5 entries 含载体清单；
> 容器核证全 OK（zip 4/4・tar 4/4・顶层 2/2），包内 `--build-info` 双平台读 `version=0.8.2`，
> **GitHub Release `v0.8.2`** 已发布。**计数 55 不变**，`0by` 维持 `pending`（S3 达成；S4 双形态回归待跑）。

> **日期**：2026-09-28；**用户令**：「请进行提交并推送，随后重建吧，本轮包体发行」后半。
> **进体内容**＝0by S1 签名器 stderr 旁路（`crates/orz-loop/src/acaf.rs`＋`crates/orz-bin/src/main.rs`）
> ＋父仓两启动器 provision 落点修正与装配门（父仓 `a3298152`，非本仓源码）；orz 侧另含版本 bump 两行。
> **源冻结**＝orz HEAD **`93625868`**（pin `da2378f9` → `93625868`）。

---

## §1 版本冻结

| 项 | 值 |
|---|---|
| orz 提交 | `93625868` |
| 主题 | `chore(release): bump version 0.8.1 -> 0.8.2（0by S1/S2 修复载体重建源冻结）` |
| 文件 | `crates/orz-bin/Cargo.toml` 0.8.1 → 0.8.2；`Cargo.lock` orz-bin 条目同步 |
| 校验 | `cargo metadata --locked` **exit 0** |
| 推送 | `da2378f9..93625868 → feat/fusion-architecture`（`cli` remote）**exit 0** |

## §2 Windows 重建与换装

- 构建：[`scripts/build_orz.ps1`](../../scripts/build_orz.ps1) `-Release -Jobs 2` **exit 0（3m06s）**；
  装配清单：12 核／物理 15.8 GiB（余 4.5）／commit 29.4 GiB（余 5.5，使用 81.2%）／`PROTOC` 脚本自装配。
- 警告 **1 条**＝`orz-host` unused import（既有面，非本批引入）。
- 换装：三件 → `D:\tb-eval\orz-windows\`，**MATCH 3/3**（旧三件留 `.0.8.1-bak` 链）；
  `--build-info` 换装位读数 **`version=0.8.2 os=windows arch=x86_64 profile=release`**（exit 0）。

| 文件 | 尺寸 (B) | 与 0.8.1 差异 | SHA256 |
|---|---:|---:|---|
| `orz.exe` | 57,022,976 | +4,096 B | `6661b0d5aee5e49f051842d1dd4e1880e2ba3feb0501b4e03c5f2a4669a73ee3` |
| `orz-signer.exe` | 6,740,480 | 0（哈希变＝内嵌版本号） | `93582c296c995279a0aa763148deda26bf0917dc22081002498bdd5e93a16764` |
| `orz-acaf-provision.exe` | 6,640,128 | 0（同上） | `0b05e2834402dee44348ed6ab3de9dd2aedae331b1c6204f028ffa1758691528` |

## §3 ACAF 重 provision（Windows 在役目录）

- 旧 manifest 留 `signer-manifest.json.bak-20260928-109`；`orz-acaf-provision <keystore> <manifest>`
  **exit 0**；manifest 回显 `binary_sha256=93582c29…` ↔ 换装位 `orz-signer.exe` **逐位一致**。
- keystore 两件哈希前后**逐位未动**（`f37556ab…`／`2aa80cb8…`）——107 批判定的「内层
  `<acaf>\keystore` 为正典」在重建批再次成立（启动器导出根＝provision 落点）。

## §4 Linux musl 三件套（Docker，直写换装位）

- **预检**（DOCKER_PROXY_RECIPE 第 0 步）：`docker run --rm rust:1.97-slim bash -c "apt-get update -qq
  && apt-get install -y -qq musl-tools"` ⇒ **`APT_OK`** ⇒ 跳过代理切换／还原两段。
- **构建**：`rust:1.97-slim` ＋ [`build_orz_aliyun_trixie.sh`](../../scripts/build_orz_aliyun_trixie.sh)
  （ORZ-BUILD-MOUNT-001 契约、`/target` 缓存沿用）**exit 0（cargo 6m27s）**；换装位旧三件预置
  `.0.8.1-bak` 链后直写，产物与 `orz-target/x86_64-unknown-linux-musl/release` **MATCH 3/3**。

| 文件 | 尺寸 (B) | 与 0.8.1 差异 | SHA256 | BuildID |
|---|---:|---:|---|---|
| `orz` | 115,460,160 | +5,056 B | `7920146b20960cdce7ce323f02bca3e3589e701d77e794cce45fda45a68ec6b6` | `192bd6e6` |
| `orz-signer` | 1,401,496 | 0（哈希变＝内嵌版本号） | `77ba031a0a10cc6d0cdfab96b06b7715c3c132ff23df1131f537ec9610570081` | `a408f458` |
| `orz-acaf-provision` | 1,220,112 | −16 B | `d29ee7bd28d0b9322e0a1509c9f05012b2928720c490512a9c68226601dcb913` | `097ce8ac` |

- **ELF 核验**（alpine 3.20 ＋ `file`／`binutils`）：三件均 **`static-pie linked`**＋**`INTERP` 段 = 0**。
- **双向加载冒烟**：alpine 3.20 与 `debian:bookworm-slim` 内 `orz --build-info` **exit 0** 且读数
  **`version=0.8.2 os=linux arch=x86_64 profile=release`**。
- **警告面**：`orz-host` 3 条（Linux-only cfg 既有面：`unused variable` 族）——本批触碰面（`orz-loop::acaf`）
  零新增告警。

## §5 进体判据与打包

### §5.1 字面量核证（出现次数法；对照＝`.0.8.1-bak`）

| 字节模式 | Windows 0.8.1 → 0.8.2 | Linux 0.8.1 → 0.8.2 | 判定 |
|---|---:|---:|---|
| `ORZ_ACAF_SIGNER_STDERR_LOG` | 0 → **3** | 0 → **3** | S1 旁路进体 ✅ |
| `signer stderr log`（失败报错文案） | 0 → **1** | 0 → **1** | fail-loud 文案进体 ✅ |

- 其余历史字面量面（`[carrier-integrity]`／写入管控／`GrokBuild:` 边界等）本批未触，按 100 档读数沿用；
  本版进体判据＝**上表＋版本字面量＋源冻结一致性**（行为面读数见 §7）。

### §5.2 打包（`D:\tb-eval\rel-109-stage\`）

- 入口 [`.tmp-b109-package.ps1`](../../.tmp-b109-package.ps1)（沿 101 形态：清单**由打包阶段按包内实际内容生成**）：
  两侧各 **5 entries**＝三件套＋`README.md`＋`SHA256SUMS`；包内六件与在役载体 **6/6 MATCH**。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.2-windows-x86_64.zip` | 28,658,314 | `679b9934713fecf4c48c2a695f43823d3ff36a1c1e044628aa59f4c3f88d1aa6` |
| `orz-0.8.2-linux-x86_64.tar.gz` | 37,156,074 | `d8cee70e3b0bf69efaea27b6c758ed364a81354493c1e652bda16a4e8233a798` |
| `SHA256SUMS`（顶层） | 186 | 上述两行（LF、`hash *name`） |

## §6 容器核证与清单活体两态

- `alpine:3.20` 内解包后 `sha256sum -c`：zip **4/4 OK**／tar **4/4 OK**／顶层 **2/2 OK**。
- 包内 `--build-info`：Windows `_verify_zip\orz.exe` 与 Linux `_verify_tar/orz` 均读 **`version=0.8.2`**（exit 0）。
- **清单活体两态**（触发命令＝`orz rollback list`，Linux 包内）：解压态**零告警** ✓；解压态 `README.md`
  追加一字节后**恰 1 条** finding＝`README.md: 长度不符（清单 28224 ≠ 实际 28225）` ✓ ⇒ 清单对
  `README.md`／`SHA256SUMS` 的覆盖在生效。

## §7 发行

- **GitHub Release `v0.8.2`**（Latest）：tag 指向本批父仓提交；三资产＝Windows zip／Linux tar.gz／顶层
  `SHA256SUMS`。服务端 digest 与回下载逐位复核读数见后续「发行回读」批。
- 发布说明＝包内 `README.md` 的 0.8.2 段（问题／修复／可观测性／验证四节）。

## §8 台账与门禁

- pin `da2378f9` → **`93625868`**；`orz_source_manifest.sha256` 重生成 **1506 条**（`Cargo.toml`／`Cargo.lock`
  两处哈希更新；`--check` 读 `valid`）。
- 父仓 `README.md` 发布面三处对齐 **v0.8.2**（解包文件名／最新 release 链接／发布线描述）。
- BACKLOG `### 0by.` 补 S3 段＋状态行；TODO `P1-0by` S3 勾选；索引 §6 条目补「载体已进体（0.8.2）」＋头行
  v4.76 → **v4.77**（v4.76 滚入存档卷，152 → **153 行**）；第二卷 §1.61；门禁目标 `valid: true`。

## §9 边界与未做

1. **S4 未跑**：桌面形态零拒绝已有两条单工具绿读数（`RUN-CLI-6ab9513b`／`RUN-CLI-6ab95149`，107 批）；
   **容器形态回归零拒绝未跑** ⇒ `0by` 维持 **`pending`**（计数 55 不变）。
2. **89 题整轮未发起**（余额 71.14 CNY 待充）。
3. 双平台预检未做（默认口径）；Linux 载体未做 ACAF 重 provision（沿 093／100 同口径）。
4. 签名器失败原因在**缺省配置**下仍不进 journal（需显式开启 `ORZ_ACAF_SIGNER_STDERR_LOG`）；
   「缺省亦带首条 stderr」仍为候选，不占计数。

## §10 关联与关键词

[`108 提交推送档`](108_SUBMIT_PUSH_2026-09-28.md)／[`107 S2 修复档`](107_0BY_S2_FIX_2026-09-28.md)／
[`106 S1 勘定档`](106_0BY_S1_DIAGNOSIS_2026-09-28.md)／
[`100 重建档`](100_CARRIER_REBUILD_V081_DUAL_PLATFORM_2026-09-27.md)／
[`101 发行档`](101_SUBMIT_PUSH_AND_RELEASE_2026-09-27.md)。

关键词：0.8.2、载体重建、双平台、0by S3 进体、签名器 stderr 旁路、字面量核证、ACAF 重 provision、
static-pie、INTERP 段 0、carrier-manifest 5 entries、容器 sha256sum -c 全 OK、清单活体两态、
GitHub Release v0.8.2、S4 待跑、0by pending。
