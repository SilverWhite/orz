# 0.6.3 双平台载体重建与换装（2026-09-19）

> 用户 2026-09-19 指示：按「提交批 → 去噪补齐 → 载体重建 → 0ar S3」线推进；
> **同日追加：「重建完成后请先不进三道题目的复验」**——本批只做重建、换装与核证，
> **不进入 0ar S3 三道题真机复验**。
> **源冻结线**：orz **`ac17a521`**（`bd253ee7` 提交线＋版本 bump `0.6.2 → 0.6.3`
> 两文件两行）。**0am 影子 RLI／Part A 批不在此线**（仍 stash 挂起，随 O2 裁决）——
> 本批二进制**不含 0am 面**（0.6.1／0.6.2 的「二进制含未提交 0am」边界就此消除）。
> 上一载体：Windows／Linux 均 **0.6.2**（`08ab194c` 冻结）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.2 → **0.6.3**（orz `ac17a521`） |
| Windows 三件套 | `cargo clean`（54,960 文件／39.4 GiB）后 clean 全量重建 **15m44s**、无 error；换装逐件 MATCH；ACAF 重 provision（manifest↔signer 一致） |
| Linux musl 三件套 | Docker `rust:1.97-slim`＋暖 `/target` 缓存，apt 一次通过；静态 rg 自建；构建 exit 0（约 12 分钟）；PT_INTERP=0；bookworm／alpine 双向冒烟 exit 1 形态 |
| 字面量核证 | 0ar S2／去噪／0as 新面 0→非零；既有面保持；**0am 面 0.6.3 全为 0**（0.6.2 有值） |
| 未做（本批边界） | **0ar S3 三道题复验未进入**（用户指示）；未推送、未发 Release、未改计数 |

## 1. 前置基线（0.6.2 在役件）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,989,888 | `D33D904B…` |
| Windows | `orz-signer.exe` | 6,742,528 | `0BF87AB0…` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `30CC538A…` |
| Linux | `orz` | 111,394,672 | `D14D6D9C…` |
| Linux | `orz-signer` | 1,398,880 | `73F5CE71…` |
| Linux | `orz-acaf-provision` | 1,217,640 | `EB186E4F…` |

两平台换装前均已备份 `.0.6.2-bak`，换装前逐件哈希与上表一致。

## 2. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：移除 **54,960 文件／39.4 GiB**（D: 余量回到约 44.9 GB）。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\CLI\orz\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`、清 `ORZ_*`／`GROK_*`）；**exit 0，15m44s**，日志
  `.tmp-063-windows-build.log`。告警 2 项均为既有 dead_code
  （`xai-tty-utils::process_alive`／`orz-host::register_live_call_job`），无 error。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.2 差异 |
|---|---:|---|---|
| `orz.exe` | 53,054,976 | `1A8DECA771457352434939A758FB19E21E0F033CB96E029EA80A6B36810EAEEA` | −934,912 B |
| `orz-signer.exe` | 6,728,192 | `AD05276FF33C7CC6FD0DA614A2B5FD2A15A2F05C73D6153BCB0A69AB769598B5` | −14,336 B |
| `orz-acaf-provision.exe` | 6,633,984 | `4B08534618F86CF05EA3E49B067B1A8977543588C352FD20616B2778D84222AF` | −8,192 B |

- 冒烟（构建位）：provision usage **exit 1**；signer 无 manifest fatal **exit 1**。
- 换装（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**。
- ACAF 重 provision（signer 哈希随 clean 重建变更）：keystore 保留；旧 manifest 留
  `signer-manifest.json.bak-20260919-063`；重 provision 回显
  `binary_sha256=ad05276f…` ↔ 换装位 `orz-signer.exe` 实哈希逐位一致；launch env
  `ORZ_ACAF_KEYSTORE/MANIFEST/BINARY` 已回显。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal **exit 1**（同 052–062 形态）。

## 3. Linux musl 三件套（Docker）

- 引擎：Docker Desktop 启动后 server 29.6.2 就绪。
- 构建：`rust:1.97-slim` ＋ `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约；`/target` 暖缓存沿用；apt update+install 一次通过；
  `cargo install ripgrep 15.0.0 --target musl` 静态 rg 后
  `cargo build --release --target x86_64-unknown-linux-musl -p orz-bin -j 1`）；
  **exit 0**（约 12 分钟），日志 `.tmp-063-linux-build.log`。

| 文件 | 尺寸 (B) | SHA256 | 与 0.6.2 差异 |
|---|---:|---|---|
| `orz` | 111,528,824 | `AC3FB7BABFBEC7C3BD8B4116B116DA48C4DA53D7DB5A3434CC8D6CAC86AFFD7E` | +134,152 B |
| `orz-signer` | 1,398,272 | `0F965C5261222D09DB9E2EAE039A6BD27FCE4E33C1B890486DF2ACD02FEAAD2B` | −608 B |
| `orz-acaf-provision` | 1,217,056 | `6CFFDE38E2817F4A03E327FAED32B2097F22A8C5D7B211F633F785E065738B82` | −584 B |

- ELF 静态核验：三件均 `e_type=3`（ET_DYN static-pie）＋`e_machine=62`（x86-64）＋
  **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz TUI 无 TTY／signer manifest 缺失／provision usage——与 052–062 同形态）。
- 换装：构建脚本直写 `/out`（＝ `D:\tb-eval\orz-linux`）；在役 0.6.2 已预留
  `.0.6.2-bak`（哈希与 §1 一致）。Linux 侧 ACAF 由评测容器运行期 provision，不涉 manifest。

## 4. 字面量核证（双方二进制，字节级出现次数）

| 字面量 | Win 0.6.2 | Win 0.6.3 | Lin 0.6.2 | Lin 0.6.3 | 判读 |
|---|---:|---:|---:|---:|---|
| `context_compress` | 14 | 14 | 5 | 5 | 0ap 面保持 |
| `ORZ_MODEL_FACE_SLIDER_TOKENS` | 1 | 1 | 1 | 1 | v8 面保持 |
| `context_scale:hard_truncate` | 7 | 7 | 7 | 7 | v8 T1 面保持 |
| `blackboard_write` | 18 | 18 | 1 | 1 | 0ao 收敛面保持 |
| `session archive: ` | 3 | 3 | 1 | 1 | 0ak 面保持 |
| `evidence_threshold_met` | 0 | **7** | 0 | **8** | 0ar S2 进件 |
| `dispatch_wallclock_bound` | 0 | **7** | 0 | **8** | 0ar S2-D2 进件 |
| `subagent_early_delivery` | 0 | **7** | 0 | **7** | 0ar S2-D1 进件 |
| `retrieval_dispatch_deferred_one_per_round` | 0 | **7** | 0 | **7** | 0ar S2-D3 进件 |
| `[EARLY_DELIVERY]` | 0 | **1** | 0 | **1** | 0ar S2-D1 进件 |
| `本批可用证据` | 0 | **3** | 0 | **3** | 可见倒数行进件 |
| `检索批次收尾` | 0 | **1** | 0 | **1** | β 收尾块进件 |
| `未重复检索` | 0 | **1** | 0 | **1** | S3 前去噪（重复 query 指针）进件 |
| `utf-8-lossy:` | 0 | **1** | 0 | **1** | 0as 降级读数进件 |
| `WALLCLOCK_REMAINING_ROUNDS` | 1 | **0** | 1 | **0** | **0am 面已不在新二进制** |
| `rli_shadow` | 2 | **0** | 4 | **0** | 同上 |
| `ORZ_LIF_RLI_SHADOW` | 1 | **0** | 1 | **0** | 同上 |

**判读**：0ar S2／去噪与 0as 面均按设计进件；既有 0ap／v8／0ao／0ak 面逐项保持；
0am 三标记 0.6.2 有值、0.6.3 全零，坐实「本批二进制＝提交线 `ac17a521`，不含挂起 0am」。

## 5. 边界与未做项

1. **0ar S3 三道题复验未进入**（用户 2026-09-19 指示）——重建完成后保持停在复验门前，
   待进一步指令；BACKLOG／TODO／索引已按「暂缓」登记。
2. **未推送、未发 Release、未改未闭合计数**（本批只做重建、换装与核证）。
3. 证据留档（仓根 `.tmp-*` 门禁豁免面）：`.tmp-063-windows-build.log`／
   `.tmp-063-linux-build.log`；两平台 `.0.6.2-bak` 备份链保留。
4. 本批二进制与源冻结 `ac17a521` 一一对应（0am 不在其中）；后续若提交新的 orz 线
   （如 0am 合回）需再走一次重建方可声称可重现。

## 6. 入口与关键词

入口：[`062 重建先例`](062_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-18.md) ／
[`0AR S2 报告`](0AR_S2_RS_MECHANICAL_0M_S4_OVERNIGHT_2026-09-19.md) ／
[`0AS 实施回执`](0AS_ENCODING_LOSSY_REFINEMENT_2026-09-19.md) ／
[`BACKLOG 0ar`](../BACKLOG_AND_PRIORITIES.md)。
关键词：0.6.3、双平台、载体重建、换装、ACAF 重 provision、Linux musl static-pie、
0ar S2、S3 前去噪、0as、0am 缺席、S3 暂缓。
