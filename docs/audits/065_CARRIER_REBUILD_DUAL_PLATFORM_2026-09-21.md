# 0.6.5 双平台载体重建与换装（2026-09-21）

> 用户 2026-09-21 指示：「请进行提交与推送，并重建吧」——本批先做 RLI 批与 bump 的提交推送，
> 再做**双平台载体重建＋换装＋核证**。同批用户裁决：**RLI 依旧是影子组件**（默认关），
> 下一步裁决待 **0bc 实测**之后。
> **源冻结线**：orz **`e3bf357c`**（0am 模态分离批 `b8789259`〔锚点模态对＋事件级采样＋
> `env_prog` 撤名＋RLI 域级定位面〕＋版本 bump `0.6.4 → 0.6.5` 两文件两行）。
> 上一载体：Windows／Linux 均 **0.6.4**（`a47e9185` 冻结）。本批为 0am 全线的**首次进件载体**
> （0.6.4 按构造不含 0am 面）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.4 → **0.6.5**（orz `e3bf357c`） |
| 构建根 | **原地**（`D:\CLI\orz`）——与 064 的差别见 §2：源树已干净，无需 worktree 镜像隔离；省去双份 `target`（D: 换装前余量 35.2 GB） |
| Windows 三件套 | `cargo clean`（21,812 文件／11.8 GiB）后 **clean 全量重建 14m40s**、exit 0；换装逐件 MATCH；ACAF 重 provision（keystore 未动、manifest↔signer 一致） |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_OK`（按配方 §0 **未切换代理**）；构建 exit 0（**12m24s**）；PT_INTERP=0；bookworm／alpine 双向冒烟 exit 1 形态 |
| 字面量核证 | **0am 全线首次进件**：`ORZ_LIF_RLI_SHADOW` 0→**7/7**、`rli_shadow` 0→2/5、`slow_prog`／`fast_prog` 0→7/8、`rli.history` 0→2/2、`WALLCLOCK_REMAINING_ROUNDS` 0→**1/1**；既有面逐项保持；`[EARLY_DELIVERY]` Windows 1→0（**跨批摆动**，见 §5） |
| 记账面 | orz 分支 `feat/fusion-architecture` → `e3bf357c`；父仓 pin ＋ `orz_source_manifest.sha256` 重算 **1461** 条（差异恰 6 行＝本批 4 文件＋bump 2 文件） |
| 发行 | **未发 Release**（本批用户令只含提交推送与重建；包与 GitHub Release 未做，见 §7） |

## 1. 前置基线（0.6.4 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256（前 8 位） |
|---|---|---:|---|
| Windows | `orz.exe` | 53,211,648 | `E168C2C5…` |
| Windows | `orz-signer.exe` | 6,730,240 | `71032A0D…` |
| Windows | `orz-acaf-provision.exe` | 6,637,056 | `06E240B8…` |
| Linux | `orz` | 111,513,008 | `1160826E…` |
| Linux | `orz-signer` | 1,397,952 | `13017BA7…` |
| Linux | `orz-acaf-provision` | 1,216,744 | `8908F832…` |

与 [`064 审计`](064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md) 记录逐位一致；两平台换装前均
已备份 **`.0.6.4-bak`**（六件，首次出现该后缀）。

## 2. 源冻结与构建根（本轮与 064 的差别）

- 064 需 worktree 镜像（`D:\tb-eval\orz-b064\orz`），因为当时主工作树含**未提交**的 0am 影批，
  就地构建会把影批编进载体。
- **本轮源树干净**：0am 模态分离批已在 `b8789259` 提交、bump 在 `e3bf357c`，`git status` 空。
  故**不再需要镜像隔离**——就地（`D:\CLI\orz`）clean 全量构建，构建内容与冻结线逐字节同源，
  并避免双份 `target` 占用（镜像方案按 064 实测约需 29.8 GiB，本轮仅余 35.2 GB）。
- 副作用如实登记：主工作树的 `target` 由 dev 件转为 release 件（`cargo clean` 移除 11.8 GiB），
  后续测试将重新编译 dev 目标，属预期成本。

## 3. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：移除 **21,812 文件／11.8 GiB**（D: 余量 35.2 → **43.4 GB**）。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`）；**exit 0，14m40s**，日志 `.tmp-b065-windows-build.log`
  （与 063 的 15m44s／064 的 15m48s 同量级，`ORZ-DEV-LINKER-CRASH-001` 未复现）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.4 差异 |
|---|---:|---|---|
| `orz.exe` | 53,440,512 | `520C2236ED083C8935A97AE97E415980D0F66270C34383D878A09A6B77F38055` | +228,864 B |
| `orz-signer.exe` | 6,732,800 | `D4227A8E2458ECA05D2DF4A7C89BA4AF5EF02B57FC4A8C6356C00563269BB47D` | +2,560 B |
| `orz-acaf-provision.exe` | 6,639,616 | `2520EF8F0454166159340CDA1C2176E0832EF44594B57F7C71D5C75F034A4534` | +2,560 B |

- 冒烟（构建位）：provision usage **exit 1**；signer 无 manifest fatal **exit 1**。
- 换装（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；
  旧 manifest 留 `signer-manifest.json.bak-20260921-065`。
- ACAF 重 provision：keystore **保留**（`installation-key.dpapi` `F37556AB…`／`.json` `2AA80CB8…`
  前后逐位一致）；重 provision 回显 `binary_sha256=d4227a8e…` ↔ 换装位 `orz-signer.exe` 实哈希**逐位一致**；
  launch env（`ORZ_ACAF_KEYSTORE`／`ORZ_ACAF_MANIFEST`／`ORZ_ACAF_BINARY`）已回显。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal **exit 1**。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -DryRun` 断言全过（载体哈希回显 `520C2236ED08…`、
  ACAF fail-closed=True）。

## 4. Linux musl 三件套（Docker）

- 实包预检：`docker run --rm rust:1.97-slim bash -c "apt-get update -qq && apt-get install -y -qq
  musl-tools protobuf-compiler ripgrep make"` → **APT_EXIT=0** ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)
  §0 **不切换代理、不重启**（与 063／064 批一致）。
- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约），
  挂载＝`-v D:/CLI:/orz`（父仓原地，`runtime` 原生可见，**无需 064 的嵌套绑定**）＋
  `-v D:/tb-eval/orz-target:/target`（暖缓存）＋`-v D:/tb-eval/orz-linux:/out`；
  **exit 0，12m24s**，日志 `.tmp-b065-linux-build.log`。
- 构建后 **主工作树 `git status` 为空**（容器未在源树留下任何改动）。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.4 差异 |
|---|---:|---|---|
| `orz` | 111,886,072 | `C43F4E05767B37ECB2E6E87D82358C990EBD96056CCA5605E2EE20EDC5B2F36F` | +373,064 B |
| `orz-signer` | 1,399,432 | `545DBC6C0DC8D0AD40D391597D39950B9AAB2C22DB346CD033BA808186A8B2EA` | +1,480 B |
| `orz-acaf-provision` | 1,218,184 | `A8DEA426E4DF9D97BF84C2627FBD4836E9289D3DD78ABB4BE019E761F5D0E406` | +1,440 B |

- ELF 静态核验：三件均 `e_type=3`（ET_DYN static-pie）＋`e_machine=62`（x86-64）＋**PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY／signer manifest 缺失／provision usage——与 052–064 同形态）。
- 换装：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）；在役 0.6.4 已预留 `.0.6.4-bak`。
  Linux 侧 ACAF 由评测容器运行期 provision，不涉 manifest。

## 5. 字面量核证（双方二进制，字节级出现次数；**对照列为实测**）

| 字面量 | Win 0.6.4 | Win 0.6.5 | Lin 0.6.4 | Lin 0.6.5 | 判读 |
|---|---:|---:|---:|---:|---|
| `ORZ_LIF_RLI_SHADOW` | 0 | **7** | 0 | **7** | **RLI 影面进件** |
| `rli_shadow` | 0 | **2** | 0 | **5** | 同上（快照字段面） |
| `slow_prog` | 0 | **7** | 0 | **8** | 模态对进件 |
| `fast_prog` | 0 | **7** | 0 | **8** | 模态对进件 |
| `rli.history` | 0 | **2** | 0 | **2** | 域级定位面进件 |
| `sample_anchor_series` | 0 | 0 | 0 | **1** | Windows 侧未物化（内联；Linux 保留）——登记 |
| `WALLCLOCK_REMAINING_ROUNDS` | 0 | **1** | 0 | **1** | 0am Part A（T̂ 轮次预算）进件 |
| `context_compress` | 13 | 13 | 5 | 5 | 保持 |
| `blackboard_write` | 17 | 17 | 1 | 1 | 保持 |
| `evidence_threshold_met` | 7 | 7 | 8 | 8 | 0ar S2 面保持 |
| `dispatch_wallclock_bound` | 7 | 7 | 8 | 8 | 0ar S2-D2 面保持 |
| `subagent_early_delivery` | 7 | 7 | 7 | 7 | 0ar S2-D1 面保持 |
| `retrieval_dispatch_deferred_one_per_round` | 7 | 7 | 7 | 7 | 0ar S2-D3 面保持 |
| `[EARLY_DELIVERY]` | 1 | **0** | 1 | 1 | **跨批摆动**，见下注 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |
| `session archive: ` | 3 | 3 | 1 | 1 | 0ak 面保持 |
| `citation_url_count` | 1 | 1 | 1 | 1 | 0ay S1/S2 面保持 |
| `synthetic_answer_count` | 1 | 1 | 2 | 2 | 0ay 判定面保持 |
| `unattributed_usable_count` | 2 | 2 | 3 | 3 | 0az ① 面保持 |
| `usable_source_count` | 8 | 8 | 9 | 9 | 0ax 面保持 |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 1 | 1 | 1 | 1 | v8 面保持 |
| `context_scale:hard_truncate` | 7 | 7 | 7 | 7 | v8 T1 面保持 |

> **注一（`[EARLY_DELIVERY]` Windows 摆动）**：实测**历史载体序列**——`0.5.0`–`0.5.4`／`0.6.0`／
> `0.6.1`／`0.6.2` = **0**；`0.6.3`／`0.6.4` = **1**；`0.6.5` = **0**（Linux 恒为 1）。
> 源码面未变（`batch_close.rs` `EARLY_DELIVERY_MARKER`）、符号面不变（`early_delivery` 28→28、
> `subagent_early_delivery` 7→7）、且标记解析有单测覆盖 ⇒ 判为**编译器／链接期的常量物化差异**
> （与本批改动无因果），按 064 批同口径**不据此改码，留作观察**。
> **注二（字面量核证的边界）**：核证只回答「面是否进件」的粗问题，**不构成语义等价证明**；
> 功能级验证留给 0bc 真机实测。

## 6. 记账面（父仓 pin 与清单）

- orz 分支推进：`feat/fusion-architecture` → **`e3bf357c`**（`b8789259` RLI 批 → bump）；
  已推 `cli` 远端（显式 refspec `cli feat/fusion-architecture:feat/fusion-architecture`）。
- 父仓 pin：`git add orz` 暂存 gitlink → `e3bf357c`。
- `orz_source_manifest.sha256`：重算 **1461** 条，差异**恰 6 行**——本批 4 文件
  （`rli.rs`／`lif/mod.rs`／`controller.rs`／`rli_shadow_replay.rs`）＋ bump 2 文件
  （`crates/orz-bin/Cargo.toml`／`Cargo.lock`）。
- 本文档与本批台账（索引／BACKLOG／TODO）同批提交，推送 `origin main`。

## 7. 边界与未做项

1. **未发 GitHub Release**：本批用户令为「提交与推送＋重建」，**不含**打包与发布；若需发布
   （双平台包＋`SHA256SUMS`＋Release 说明），按 064 §6.2 形态另行执行。
2. **RLI 仍是影子组件**（用户裁决）：默认关，需 `ORZ_LIF_RLI_SHADOW=1`；**下一步裁决待 0bc 实测后**。
   本批只保证面进件，不改变启用面。
3. **功能级验证未做**（不在本批范围）：0am 全线（RLI 影面／域级定位面／Part A 轮次预算）
   的真机读数由 **0bc 复合狗粮轮**收取。
4. `sample_anchor_series` 在 Windows 侧未物化（Linux 为 1）——同注一类别，不据以改码。
5. 证据留档（`orz/.gitignore` 已豁免 `.tmp-*`）：`.tmp-b065-windows-build.log`／
   `.tmp-b065-linux-build.log`；两平台 `.0.6.4-bak` 备份链保留；门禁
   `scripts/check_repository.py` 见 §6。

## 8. 入口与关键词

入口：[`064 重建先例`](064_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-20.md) ／
[`0am 模态分离报告`](0AM_RLI_MODE_SPLIT_2026-09-20.md) ／
[`BACKLOG 0am/0bc`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO P1-0am`](../../TODO.md)。
关键词：0.6.5、双平台、载体重建、原地构建根、0am 首次进件、ACAF 重 provision、
Linux musl static-pie、字面量核证、跨批摆动、pin e3bf357c。
