# 0.6.4 双平台载体重建与换装（2026-09-20）

> 用户 2026-09-20 指示：「请进行重建和三题重跑吧」——本批先做**双平台载体重建＋换装＋核证**，
> 随后以新载体发起**三题重跑**（0ay S4 真机，另行登记）。
> **源冻结线**：orz **`a47e9185`**（`fce67ab7`〔0ay S1/S2 ＋ 0az 收口〕＋版本 bump
> `0.6.3 → 0.6.4` 两文件两行）。**0am 影子 RLI／Part A 批不在该线**（仍在主工作树未提交，
> 本批全程未 stash／未改写、`git diff` 逐行与批前一致）；本批二进制**不含 0am 面**。
> 上一载体：Windows／Linux 均 **0.6.3**（`ac17a521` 冻结）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.3 → **0.6.4**（orz `a47e9185`） |
| 构建根隔离 | 全新 git worktree 镜像 `D:\tb-eval\orz-b064\orz`（＋`runtime/` 链接点）⇒ **0am 影批按构造不参与编译**，主工作树未被触碰 |
| Windows 三件套 | `cargo clean`（39,079 文件／29.8 GiB）后 **clean 全量重建 15m48s**、exit 0；换装逐件 MATCH；ACAF 重 provision（keystore 未动、manifest↔signer 一致） |
| Linux musl 三件套 | Docker `rust:1.97-slim` ＋ `/target` 暖缓存；实包预检 `APT_OK`（按配方 §0 **未切换代理**）；构建 exit 0（**13m03s**）；PT_INTERP=0；bookworm／alpine 双向冒烟 exit 1 形态 |
| 字面量核证 | 0ay／0az 新面 0→非零（`citation_url_count` 0→1、`synthetic_answer_count` 0→1／0→2、`unattributed_usable_count` 0→2／0→3）；既有面保持；**0am 三标记双方全 0** |
| 记账面 | orz 分支 `feat/fusion-architecture` → `a47e9185`；父仓 pin `fce67ab7 → a47e9185`＋`orz_source_manifest.sha256` 重算 **1459** 条（差异恰 2 行＝bump 两文件）；门禁 `error_count=1`（唯一＝orz 脏树预期态） |
| 发行 | 双平台包 ＋ `SHA256SUMS` 暂存 `D:\tb-eval\rel-064-stage\`，并入 **GitHub Release `v0.6.4`**（见 §6） |
| 记账 | orz `a47e9185` 推送 `cli` 远端；父仓本批推送 `origin main`；**0ay 闭合 41 → 40**（用户裁决） |

## 1. 前置基线（0.6.3 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,054,976 | `1A8DECA7…EAEEA` |
| Windows | `orz-signer.exe` | 6,728,192 | `AD05276F…98B5` |
| Windows | `orz-acaf-provision.exe` | 6,633,984 | `4B085346…22AF` |
| Linux | `orz` | 111,528,824 | `AC3FB7BA…FD7E` |
| Linux | `orz-signer` | 1,398,272 | `0F965C52…AD2B` |
| Linux | `orz-acaf-provision` | 1,217,056 | `6CFFDE38…38B2` |

与 [`063 审计`](063_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-19.md) 记录逐位一致；两平台换装前均已
备份 `.0.6.3-bak`（六件）。

## 2. 源冻结与构建根隔离（本批与 063 的差别）

- orz 现树含**未提交**的 0am 影批（`lif/*`、`controller.rs`、`prompt.rs`、`blackboard.rs`、
  `acp_server.rs`、`Cargo.*`、`rli*`），且该批改动了 `Cargo.lock` 与工作区 `Cargo.toml`。
  直接就地构建会把 0am 面编进载体（0.6.1／0.6.2 曾如此，063 起已消除）。
- 处置：`git worktree add -b b064-bump D:\tb-eval\orz-b064\orz fce67ab7` ⇒ 在**干净检出**上做
  bump 提交（`a47e9185`）并构建；`D:\tb-eval\orz-b064\runtime` 用目录联结指向 `D:\CLI\runtime`
  满足 `orz-assurance` 的 `../../../runtime` 内嵌契约。**主工作树的 0am 影批全程未被 stash、
  未被改写**；构建结束后父仓侧只做 `reset --mixed a47e9185`（只动 HEAD／索引，不动工作树文件），
  `git status` 复核差异仍为**恰 0am 影批原集合**。

## 3. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：移除 **39,079 文件／29.8 GiB**（D: 余量 17.9 → 47.7 GB）。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`）；**exit 0，15m48s**，日志 `.tmp-b064-windows-build.log`（与 063 的
  15m44s 同量级，`ORZ-DEV-LINKER-CRASH-001` 未复现）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.3 差异 |
|---|---:|---|---|
| `orz.exe` | 53,211,648 | `E168C2C51B552D85C40C0769451AF8CFFFBA5E86E363745A5B4AC5EE537CB154` | +156,672 B |
| `orz-signer.exe` | 6,730,240 | `71032A0DEFE17C8D9638A65B04A8EF98AB0FCBBC27CD8BB0DB1DB7FCAF57ED34` | +2,048 B |
| `orz-acaf-provision.exe` | 6,637,056 | `06E240B8E01182B3C39DB51B11850FDED4A9BB29E2F33F17DB5853B4FF1D5442` | +3,072 B |

- 冒烟（构建位）：provision usage **exit 1**；signer 无 manifest fatal **exit 1**。
- 换装（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；
  旧 manifest 留 `signer-manifest.json.bak-20260920-064`。
- ACAF 重 provision：keystore **保留**（`installation-key.dpapi`／`.json` 前后哈希一致）；
  重 provision 回显 `binary_sha256=71032a0d…` ↔ 换装位 `orz-signer.exe` 实哈希**逐位一致**；
  launch env（`ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST`／`ORZ_ACAF_BINARY`）已回显。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal **exit 1**（同 052–063 形态）。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -DryRun` 断言全过（载体哈希回显 `E168C2C5…`、
  ACAF fail-closed=True）。

## 4. Linux musl 三件套（Docker）

- 实包预检：`docker run --rm rust:1.97-slim bash -c "apt-get update -qq && apt-get install -y -qq
  musl-tools protobuf-compiler ripgrep make"` → **APT_OK** ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)
  §0 **不切换代理、不重启**（当前 `ProxyHTTPMode=system`，与 0.6.3 批一致）。
- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约），
  **exit 0，13m03s**，日志 `.tmp-b064-linux-build.log`。
  **挂载注记**：构建根取自 worktree 镜像（`/orz` ＝ `D:\tb-eval\orz-b064`），其 `runtime` 目录联结
  在容器内**不可解析**（`ls /orz/runtime` 失败）⇒ 追加嵌套绑定 `-v D:/CLI/runtime:/orz/runtime`
  （`RUNTIME_OK` 复核）；其余参数与 063 逐项一致（`/target` 暖缓存沿用、`/out`＝`D:\tb-eval\orz-linux`）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.3 差异 |
|---|---:|---|---|
| `orz` | 111,513,008 | `1160826E7235E6D7B2E4799E58CB0A85C2FF86DB984DB8933777A918DBBD1CB9` | −15,816 B |
| `orz-signer` | 1,397,952 | `13017BA785B0D6B767D8E8A1C9340FB56B810244CDA289E7BD0D4F16892E3194` | −320 B |
| `orz-acaf-provision` | 1,216,744 | `8908F83287F1500F4D3903CB389112DB2ECCA6A7DA9B711FBFF1BA230B6D0B4A` | −312 B |

- ELF 静态核验：三件均 `e_type=3`（ET_DYN static-pie）＋`e_machine=62`（x86-64）＋**PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz TUI 无 TTY／signer manifest 缺失／provision usage——与 052–063 同形态）。
- 换装：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）；在役 0.6.3 已预留 `.0.6.3-bak`。
  Linux 侧 ACAF 由评测容器运行期 provision，不涉 manifest。

## 5. 字面量核证（双方二进制，字节级出现次数）

| 字面量 | Win 0.6.3 | Win 0.6.4 | Lin 0.6.3 | Lin 0.6.4 | 判读 |
|---|---:|---:|---:|---:|---|
| `citation_url_count` | 0 | **1** | 0 | **1** | 0ay S1/S2 进件（判定输入落 ledger） |
| `synthetic_answer_count` | 0 | **1** | 0 | **2** | 0ay 判定面进件 |
| `unattributed_usable_count` | 0 | **2** | 0 | **3** | 0az ① 声明可用面对拍进件 |
| `usable_source_count` | 8 | 8 | 9 | 9 | 0ax 面保持 |
| `context_compress` | 14 | 13 | 5 | 5 | 保持（Windows ±1 见下注） |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 1 | 1 | 1 | 1 | v8 面保持 |
| `context_scale:hard_truncate` | 7 | 7 | 7 | 7 | v8 T1 面保持 |
| `blackboard_write` | 18 | 17 | 1 | 1 | 保持（Windows ±1 见下注） |
| `session archive: ` | 3 | 3 | 1 | 1 | 0ak 面保持 |
| `evidence_threshold_met` | 7 | 7 | 8 | 8 | 0ar S2 面保持 |
| `dispatch_wallclock_bound` | 7 | 7 | 8 | 8 | 0ar S2-D2 面保持 |
| `subagent_early_delivery` | 7 | 7 | 7 | 7 | 0ar S2-D1 面保持 |
| `retrieval_dispatch_deferred_one_per_round` | 7 | 7 | 7 | 7 | 0ar S2-D3 面保持 |
| `[EARLY_DELIVERY]` | 1 | 1 | 1 | 1 | 0ar 面保持 |
| `本批可用证据` | 3 | 3 | 3 | 3 | 可见倒数行保持 |
| `检索批次收尾` | 1 | 1 | 1 | 1 | β 收尾块保持 |
| `未重复检索` | 1 | 1 | 1 | 1 | S3 前去噪保持 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |
| `WALLCLOCK_REMAINING_ROUNDS` | 0 | 0 | 0 | 0 | **0am 面缺席** |
| `rli_shadow` | 0 | 0 | 0 | 0 | 同上 |
| `ORZ_LIF_RLI_SHADOW` | 0 | 0 | 0 | 0 | 同上 |

> **注（Windows 两处 ±1）**：`context_compress`／`blackboard_write` 在 Windows 侧各降 1、Linux 侧
> 逐位不变（5／1），且 `fce67ab7` 的四个改动文件内**不含**这两个字面量（`git show | Select-String`
> 实测无命中）⇒ 判为链接期字面量合并／布局差异，非语义变更；本批不据此改码，留作观察。

## 6. 记账面（父仓 pin 与清单）

- orz 分支推进：`feat/fusion-architecture` → **`a47e9185`**（`reset --mixed`，工作树文件未动）；
  复核 `git status` 差异 ＝ 0am 影批原集合（`Cargo.lock` ＋1 行 `libm`、`Cargo.toml` ＋4 行注释／
  workspace 依赖、`orz-assurance` 侧 6 文件、`orz-loop`／`orz-host` 3 文件、两个未跟踪 `rli` 件）。
- 父仓 pin：`fce67ab7 → a47e9185`（`git add orz` 暂存 gitlink，**未提交**）。
- `orz_source_manifest.sha256`：重算 **1459** 条，差异**恰 2 行**（`Cargo.lock`、`crates/orz-bin/Cargo.toml`）。
- 门禁 `scripts/check_repository.py`：`error_count=1`，唯一＝「orz submodule working tree is dirty」
  （0am 影批预期态；与 0ay／0az 批同形）。

### 6.1 提交与推送（用户令「请提交并推送吧」）

- orz：`a47e9185` → `cli` 远端 `feat/fusion-architecture`（显式 refspec `cli
  feat/fusion-architecture:feat/fusion-architecture`；`origin` 属上游 xai-org、不用于推送）。
- 父仓：本批一笔提交（pin `a47e9185`＋`orz_source_manifest.sha256`＋0ay 闭合三方台账＋
  本档与 0ay S4 档）→ `origin main:main`。

### 6.2 双平台打包与 GitHub Release（用户令「新的安装包也一同推上去进行发布」）

- 双包（暂存 `D:\tb-eval\rel-064-stage\`，本地件不入库）：

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.4-windows-x86_64.zip` | 27,457,145 | `d25e3f653d02482261404cde2b38245250130fc758c2c731a2a79011557fbb9e` |
| `orz-0.6.4-linux-x86_64.tar.gz` | 35,731,429 | `bef8777491340cd683bfadca157fd56e32b7c2b0a13a535cd4ff454ff50fc79f` |

- **包内容**＝三件二进制＋`README.md`（0.6.4 适配：0ar／0ax／0ay／0az／0au／0aw／0at／0av／0as
  更新说明＋两平台快速开始＋完整性校验）＋`SHA256SUMS`（`hash *name` 形态，包内逐件清单）。
- 包完整性：解包回读逐件与换装载体**同源同哈希**（zip 逐件读回＋tar 解包比对，8/8 MATCH）；
  容器内 `sha256sum -c SHA256SUMS` **全 OK**（`orz`／`orz-signer`／`orz-acaf-provision`／`README.md`）；
  包内 `./orz --version` 可执行（无 TTY 的 TUI 报错属预期，exit 0）。
- Release：父仓 tag **`v0.6.4`** ＋ 双资产（双包），说明文本＝源冻结基线、六件二进制哈希表、
  0.6.4 更新说明（相对已发布 0.6.2）与验证摘要（沿 0.6.0／0.6.2 形态；0.6.3 为内部验证载体、未发布）。

## 7. 边界与未做项

1. **提交与推送已按本批用户指示执行**（orz `a47e9185` → `cli`；父仓本批 → `origin main`，见 §6.1）＋
   **GitHub Release `v0.6.4` 已发布**（§6.2）。
2. **计数已按用户裁决变更**：0ay 闭合 **41 → 40**（BACKLOG 计数/总览/开放项＋TODO 路由/勾选＋索引 §8 桶同步）；
   同批登记 **extract-elf 泄漏路径自判 0** 的裁判口径（不论 verifier 给分；正式成绩公布显式标注）。
3. 证据留档（仓根 `.tmp-*` 门禁豁免面）：`.tmp-b064-windows-build.log`／`.tmp-b064-linux-build.log`；
   两平台 `.0.6.3-bak` 备份链保留。构建镜像**已收尾清理**（`git worktree remove --force` ＋
   删除 `runtime` 目录联结后移除 `D:\tb-eval\orz-b064\`；`D:\CLI\runtime` 目标完好核证 True；
   临时分支 `b064-bump` 已删，提交仍在 `feat/fusion-architecture`）。复现路径：重新
   `git -C orz worktree add -b <tmp> <dir> a47e9185` ＋把 `runtime` 挂进构建根即可（D: 余量回升至 46.5 GB）。
4. 载体重跑与读数见 [`0ay S4 三题真机复验`](0AY_S4_THREE_TASK_VERIFY_2026-09-20.md)。
5. **顺带（前批 CI 尾票，本批实取）**：`d327301f` 链接卫生修复批 run `35501542514` **5/5 job 全绿**
   （含唯一长跑项「Rust tests (orz workspace / windows)」）；0ay/0az 提交批 run `35501184318` 的
   Rust job 亦绿、其 4 个 Python job 红系修复前历史读数（不追改）。

## 8. 入口与关键词

入口：[`063 重建先例`](063_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-19.md) ／
[`0ay 实施报告`](0AY_S1_S2_S3_IMPLEMENTATION_2026-09-20.md) ／
[`0az 闭合报告`](0AZ_SYNTHETIC_JUDGEMENT_AUDIT_CLOSURE_2026-09-20.md) ／
[`BACKLOG 0ay`](../BACKLOG_AND_PRIORITIES.md)。
关键词：0.6.4、双平台、载体重建、worktree 构建根隔离、0am 缺席、ACAF 重 provision、
Linux musl static-pie、嵌套 runtime 绑定、字面量核证、pin a47e9185。
