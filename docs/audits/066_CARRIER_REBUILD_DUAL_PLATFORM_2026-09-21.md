# 0.6.6 双平台载体重建、换装与发布（2026-09-21）

> 用户 2026-09-21 指示：「请先重建，随后做提交与推送吧，再次重建以后版本是不是就到 0.6.6 了，
> 请一并发布双平台包吧」——本批四段：① 未提交树上先做**预检重建**（双平台编译把关，含 0bc §8
> 点名的 Linux `#[cfg(windows)]` 面）② **提交推送**（0bc 批 ＋ 版本 bump）③ **`0.6.6`
> 双平台重建＋换装＋核证** ④ **双平台打包 ＋ GitHub Release `v0.6.6`**。
> **源冻结线**：orz **`6efd192f`**（`feat/fusion-architecture`）＝ 0bc 复合狗粮轮批 `1ecbcaa3`
> （17 文件）＋ bump 两文件两行（`0.6.5 → 0.6.6`）。上一载体：双平台 0.6.5（`e3bf357c` 冻结，
> **未发布**——0.6.5 为内部验证载体）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.5 → **0.6.6**（orz `6efd192f`） |
| 预检重建（未提交树） | Windows 增量 release **269 s** exit 0／Linux Docker **717 s** exit 0——**先验后提交**，0bc §8 点名的 Linux cfg 面无断裂 |
| Windows 三件套 | `cargo clean`（37,286 文件／**26.2 GiB**；D: 17.2 → 41.8 GB）后 **clean 全量 15m02s** exit 0；换装逐件 MATCH；ACAF 重 provision（keystore 未动、manifest↔signer 逐位一致） |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_EXIT=0`（未切代理）；**暖缓存构建 5m53s** exit 0；PT_INTERP=0；bookworm／alpine 双向冒烟 exit 1 形态 |
| 字面量核证 | **0bc 面首次进件**：`ORZ_JOB_CPU_RATE_PERCENT` 0→1、`ORZ_JOB_ACTIVE_PROCESS_LIMIT` 0→1、`commit_notification` 0→2/0→3、`资源软提示` 1→2、`打断式提醒（不锁工具面、动作照常）` 0→1、`NFC/NFD` 0→1；0am 面与既有面逐项保持（±2 见 §6 注） |
| 记账面 | orz 分支 → **`6efd192f`**（推 `cli`）；父仓 pin ＋ `orz_source_manifest.sha256` 重算 **1461** 条（差异恰 **38 行**＝19 文件×2） |
| 发行 | 双平台包 ＋ `SHA256SUMS` 暂存 `D:\tb-eval\rel-066-stage\`，并入 **GitHub Release `v0.6.6`**（见 §8） |

## 1. 前置基线（0.6.5 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256（前 8 位） |
|---|---|---:|---|
| Windows | `orz.exe` | 53,440,512 | `520C2236…` |
| Windows | `orz-signer.exe` | 6,732,800 | `D4227A8E…` |
| Windows | `orz-acaf-provision.exe` | 6,639,616 | `2520EF8F…` |
| Linux | `orz` | 111,886,072 | `C43F4E05…` |
| Linux | `orz-signer` | 1,399,432 | `545DBC6C…` |
| Linux | `orz-acaf-provision` | 1,218,184 | `A8DEA426…` |

与 [`065 审计`](065_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md) 逐位一致；两平台换装前均预留
**`.0.6.5-bak`**（六件）。

## 2. 批序与源冻结

1. **预检重建**（工作树含未提交 0bc 17 文件，**不换装**）：目的＝按用户令「先重建」做**提交前
   把关**——0bc §8 明确点名「Linux 编译面：重点核 `#[cfg(windows)]` 导出面与新增 `Win32_System_IO`
   feature 的非 Windows 分支」；两平台 exit 0 后才提交。
2. **提交与推送**：orz `1ecbcaa3`（0bc 批）→ `6efd192f`（bump）→ 显式 refspec 推 `cli`；
   此为**第一、二次重建之间的分界**（预检树＝0.6.5 标号内容，发布树＝0.6.6）。
3. **发布重建**：Windows `cargo clean` 全量 ＋ Linux 暖缓存；换装、ACAF、字面量核证同批。
4. **打包与发行**：双平台包＋`SHA256SUMS`＋README＋Release 说明 → `v0.6.6`。

> **版本问题的答复**：上一载体（0.6.5）**不会**自动递增——版本号只在 `crates/orz-bin/Cargo.toml`
> 与 `Cargo.lock` 两行显式 bump。本批 bump 后**再次重建即得 0.6.6**（本档即该次重建的读数）。

## 3. 预检重建（未提交树；不换装）

| 平台 | 命令形态 | 结果 |
|---|---|---|
| Windows | 增量 release（`PROTOC`／`CARGO_INCREMENTAL=0`／`-j 4`） | **exit 0，269 s**；`orz.exe` 53,393,408 B；日志 `.tmp-b066-pre-windows.log` |
| Linux | Docker `rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（`/target` 暖缓存，输出到**独立** `D:\tb-eval\orz-linux-precheck`） | **exit 0，717 s**；`orz` 111,911,640 B；日志 `.tmp-b066-pre-linux.log`；APT 预检 `.tmp-b066-pre-linux-apt.log`（`APT_EXIT=0`） |

字面量核证（预检件 vs 0.6.5 在役件）：0bc 新面双平台全部 0→非零、既有面保持——**先验后提交**
的机械依据；见 §6 同表。

## 4. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：移除 **37,286 文件／26.2 GiB**（较 065 的 21,812／11.8 GiB 多，主因 0bc 狗粮轮
  的 dev 目标与测试目标在场）；D: 余量 17.2 → **41.8 GB**。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`）；**exit 0，902 s（15m02s）**，日志 `.tmp-b066-windows-build.log`
  （065＝14m40s／064＝15m48s 同量级；`ORZ-DEV-LINKER-CRASH-001` 未复现）。
- 与预检件的关系：`orz.exe` 尺寸逐位相同（53,393,408 B）⇒ 增量与 clean 全量在本次改动面上
  **产物尺寸一致**（内容哈希见下；预检件未入包、不参与发行）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.5 差异（尺寸） |
|---|---:|---|---:|
| `orz.exe` | 53,393,408 | `C2F31EA73C5DFBA9B0AD5666FE3604056B381307A37308404EC641A1D4BA59B3` | −47,104 B |
| `orz-signer.exe` | 6,731,264 | `78308D97C525658A81FAA091C2AEC50AE2103893F4658CE5535411AF421A2831` | −1,536 B |
| `orz-acaf-provision.exe` | 6,639,616 | `DFAEC758DEAD232E507D487DF602E155AFE4E855B0713B35CF41F79CBDEA8742` | ±0 B |

- 冒烟（构建位）：provision usage **exit 1**；signer 无 manifest fatal **exit 1**。

## 5. Linux musl 三件套（Docker）

- 实包预检：`apt-get update && apt-get install -y musl-tools protobuf-compiler ripgrep make`
  → **APT_EXIT=0** ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**。
- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约），
  挂载＝`-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`（暖缓存）＋
  `-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.5-bak`）；
  **exit 0，353 s（5m53s）**，日志 `.tmp-b066-linux-build.log`——暖缓存仅需重编本批改动 crate
  ＋ relink（065 首轮为 12m24s）。
- 构建后 orz 仓 `git status` 为空（容器未在源树留痕）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.5 差异（尺寸） |
|---|---:|---|---:|
| `orz` | 111,911,544 | `601ee1db07687d6c72e01ce2c7259d0b745f6dbf72e921ff77cf61cad6d08e63` | +25,472 B |
| `orz-signer` | 1,399,552 | `8b6700c49aceb9841de8114c1a0a4fae357fdc98dd94929ce61be13911fdcf24` | +120 B |
| `orz-acaf-provision` | 1,218,280 | `d1c884aa792f689ef67ea35b2f6d6576ea4339328336830b4753b3345588b001` | +96 B |

- ELF 静态核验（`python .tmp-b066-elfcheck.py`）：三件均 `e_type=3`（ET_DYN static-pie）＋
  `e_machine=62`（x86-64）＋**PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY／signer manifest 缺失／provision usage——与 052–065 同形态）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.5 备份件）

| 字面量 | Win 0.6.5 | Win 0.6.6 | Lin 0.6.5 | Lin 0.6.6 | 判读 |
|---|---:|---:|---:|---:|---|
| `ORZ_JOB_CPU_RATE_PERCENT` | 0 | **1** | 0 | **1** | 0bc CPU 去顶（可选档）进件 |
| `ORZ_JOB_ACTIVE_PROCESS_LIMIT` | 0 | **1** | 0 | **1** | 0bc 进程上限 env 覆盖进件 |
| `commit_notification` | 0 | **2** | 0 | **3** | 0bc 临限通知 trigger 进件 |
| `commit_notification_bytes` | 0 | **1** | 0 | **1** | 通知读数列进件 |
| `commit_notify=` | 0 | **1** | 0 | **1** | 描述串（通知面）进件 |
| `资源软提示` | 1 | **2** | 1 | **2** | 软提示文案（每 run 一次）进件 |
| `打断式提醒（不锁工具面、动作照常）` | 0 | **1** | 0 | **1** | FR-3 文案进件 |
| `NFC/NFD` | 0 | **1** | 0 | **1** | FR1 组合记号提示进件 |
| `ORZ_LIF_RLI_SHADOW` | 7 | 7 | 7 | 7 | 0am 影面保持 |
| `slow_prog`／`fast_prog` | 7／7 | 12／12 | 8／8 | 8／8 | 0am 模态对保持（Windows ±5 见注） |
| `rli.history` | 2 | 2 | 2 | 2 | 域级定位面保持 |
| `WALLCLOCK_REMAINING_ROUNDS` | 1 | 1 | 1 | 1 | 0am Part A 保持 |
| `context_compress` | 13 | **11** | 5 | 5 | Windows −2 见注 |
| `blackboard_write` | 17 | **15** | 1 | 1 | Windows −2 见注 |
| `evidence_threshold_met` | 7 | 7 | 8 | 8 | 0ar S2 面保持 |
| `dispatch_wallclock_bound` | 7 | 7 | 8 | 8 | 0ar S2-D2 面保持 |
| `subagent_early_delivery` | 7 | 7 | 7 | 7 | 0ar S2-D1 面保持 |
| `retrieval_dispatch_deferred_one_per_round` | 7 | 7 | 7 | 7 | 0ar S2-D3 面保持 |
| `[EARLY_DELIVERY]` | 0 | 0 | 1 | 1 | 跨批摆动（065 已判），本批无变化 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |
| `session archive: ` | 3 | 3 | 1 | 1 | 0ak 面保持 |
| `citation_url_count` | 1 | 1 | 1 | 1 | 0ay S1/S2 面保持 |
| `synthetic_answer_count` | 1 | 1 | 2 | 2 | 0ay 判定面保持 |
| `unattributed_usable_count` | 2 | 2 | 3 | 3 | 0az ① 面保持 |
| `usable_source_count` | 8 | 8 | 9 | 9 | 0ax 面保持 |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 1 | 1 | 1 | 1 | v8 面保持 |
| `context_scale:hard_truncate` | 7 | 7 | 7 | 7 | v8 T1 面保持 |

> **注一（`context_compress`／`blackboard_write` Windows −2）**：直接因果＝**FR-3** 删除的窗口
> 工具面收窄提示行（`窗口内仅 {BLACKBOARD_WRITE_TOOL_NAME} / {CONTEXT_COMPRESS_TOOL_NAME} 可执行…`）
> 一行同时含两个工具名 ⇒ 两个字面量各 −1；另一处为窗口工具面白名单常量退役（同因）。Linux 侧
> 侧原本该串未物化（编译期常量为 1），故不动。
> **注二（`slow_prog`／`fast_prog` Windows 7→12）**：0bc **FR-5** 把工具 schema 的 `name` enum
> 由 `TemporalState::known_feature_names()`＋`RliShadow::known_feature_names()` 单源拼装 ⇒ 字符串
> 在更多调用点物化；Linux 侧为 8（同源不同内联策略）。
> **注三（核证边界）**：字面量核证只回答「面是否进件」，**不构成语义等价证明**；功能级读数
> 由下一轮真机狗粮轮收取（0bc §8 的三项资源面读数）。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.5-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260921-066`。
- ACAF 重 provision：keystore **逐位未动**（`installation-key.dpapi`／`.json` 前后哈希一致）；
  回显 `binary_sha256=78308d97…` ↔ 换装位 `orz-signer.exe` **逐位一致**；launch env 三键回显。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal **exit 1**。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -DryRun` 断言全过（载体哈希回显 `C2F31EA73C5D…`、
  ACAF fail-closed=True、cwd 断言 D:\CLI）。
- Linux：构建脚本直写 `/out`（＝`D:\tb-eval\orz-linux`），在役 0.6.5 已预留 `.0.6.5-bak`；
  Linux 侧 ACAF 由评测容器运行期 provision，不涉 manifest。

## 8. 双平台打包与 GitHub Release

- 暂存 `D:\tb-eval\rel-066-stage\`（本地件不入库）：

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.6-windows-x86_64.zip` | 26,842,565 | `cb2fe91794185802708e8a799a5725fcca1425036de6226849bd21fb8ac785d2` |
| `orz-0.6.6-linux-x86_64.tar.gz` | 35,840,358 | `fc38b6bd83e793913675c861ad71dab628b64279ed6c8025f35b8d18ac4d29df` |

- **包内容**＝三件二进制＋`README.md`（0.6.6 适配：0bc 资源层收口／可失败分配／FR-3／0am 影子面
  更新说明＋两平台快速开始＋完整性校验）＋`SHA256SUMS`（`hash *name`，LF 行尾，包内逐件清单）。
- 包完整性：解包回读逐件与换装载体 **同源同哈希（6/6 MATCH）**；容器内 `sha256sum -c SHA256SUMS`
  **全 OK**（zip 侧 4/4、tar 侧 4/4，`alpine:3.20`）；包内 `./orz` 在无 TTY 容器下为
  `error: tui io error` **exit 1** 形态（与部署侧一致，非包缺陷）。
- Release：父仓 tag **`v0.6.6`** ＋ 双资产（双包），说明文本＝源冻结基线、六件二进制哈希表、
  0.6.6 更新说明（相对已发布 0.6.4；含内部载体 0.6.5 的面）与验证摘要（沿 0.6.0／0.6.2／0.6.4 形态）。
- 上传面核证：`gh api repos/SilverWhite/CLI/releases/tags/v0.6.6` 回读两资产的 **`digest`
  字段**，与本地包哈希**逐位一致**（zip `sha256:cb2fe917…`／tar.gz `sha256:fc38b6bd…`，
  尺寸 26,842,565／35,840,358 B）⇒ 远端字节＝本地包字节。回下载核验（064 形态的
  `_verify_dl`）本轮因本机网络停摆**未完成**（`gh release download` 两次各创建 0 字节文件后
  阻塞，已终止进程），改以上述 digest 核证为准并如实登记。

## 9. 记账面（pin、清单、提交与推送）

- orz：`1ecbcaa3`（0bc 批，17 文件）→ **`6efd192f`**（bump 0.6.5 → 0.6.6，两文件两行）；
  两笔均已推 `cli` 远端（显式 refspec `cli feat/fusion-architecture:feat/fusion-architecture`）。
- 父仓 pin：`git add orz` 暂存 gitlink → `6efd192f`。
- `orz_source_manifest.sha256`：重算 **1461** 条，差异**恰 38 行**（19 文件×2＝0bc 17 文件＋bump 2 文件）。
- 门禁 `scripts/check_repository.py`：本批提交前 `error_count=1`（唯一＝「orz submodule working tree
  is dirty」＝0bc 未提交态预期）；提交后复跑见 §10。
- 本文档与本批台账（索引 v4.17／BACKLOG／TODO 同步）同批提交，推送 `origin main`；tag `v0.6.6`。

## 10. 边界与未做项

1. **计数不变**（P1 未闭合 **45**）：0bc 属进行中轮次（S3 资源面真机读数未收），本批只做提交、
   重建与发行，**不动三方计数与勾选**。
2. **0bc S3 资源面三项读数仍未收**（CPU 去顶后墙钟／commit 临限通知事件真机首现／进程上限读数）
   ——载体已就绪（本批 0.6.6 在役），待下一轮真机狗粮轮收取（0bd 或用户指定轮次）。
3. **RLI 仍是影子组件**（默认关，需 `ORZ_LIF_RLI_SHADOW=1`）；0bc §6.3 的「事件相关提示」实验
   与 0be 四项改造均**未在本批范围内**。
4. **预检件不入发行**：预检重建（§3）只作提交前把关与字面量对照，产物留在临时目录，不换装、
   不打包。
5. 证据留档（`.tmp-*` 门禁豁免，orz 仓内为 `.tmp-b066-*`）：`.tmp-b066-pre-windows.log`／
   `.tmp-b066-pre-linux.log`／`.tmp-b066-pre-linux-apt.log`／`.tmp-b066-windows-build.log`／
   `.tmp-b066-linux-build.log`／`.tmp-b066-literals.py`／`.tmp-b066-literals-final.py`／
   `.tmp-b066-elfcheck.py`／`.tmp-b066-package.ps1`；两平台 `.0.6.5-bak` 备份链保留。
6. **本批连带的文档勘误（0bc §8「随提交批」项）**：设计档
   [`HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20`](../HOST_RESOURCE_OS_DELEGATION_DESIGN_2026-09-20.md)
   §11-2 的原语名已由 `JOB_OBJECT_LIMIT_JOB_MEMORY_LOW` 修订为 **`…_MEMORY_HIGH`**（真机实证
   HIGH＝「越过上限」方向、不阻断分配；LOW 静默），并加方向注与投递延迟注；0bd 台账的第六件
   据此标注为**已落地**（该轮实际范围＝前五件）。

## 11. 入口与关键词

入口：[`065 重建先例`](065_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md) ／
[`064 重建与发行先例`](064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md) ／
[`0bc 复合狗粮轮总结`](0BC_COMPOSITE_DOGFOOD_2026-09-21.md) ／
[`RLI 前推对拍`](RLI_FORECAST_CONTRAST_2026-09-21.md) ／
[`BACKLOG 0bc`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO P1-0bc`](../../TODO.md)。
关键词：0.6.6、双平台、载体重建、预检重建、先验后提交、clean 全量、暖缓存、ACAF 重 provision、
Linux musl static-pie、字面量核证、FR-3 工具面、可失败分配、GitHub Release v0.6.6、pin 6efd192f。
