# 0.6.7 双平台载体重建与换装（2026-09-21 深夜 → 09-22 凌晨）

> 用户令：「做预检的必要性不大，一般情况下不必预检。现在的话请进行重建吧」——本批三段：
> ① **版本 bump**（0.6.6 → 0.6.7）② **双平台重建**（Windows clean 全量／Linux 容器暖缓存）
> ③ **换装＋ACAF 复核＋进件核证**（**不打包、不发行**——打包与 GitHub Release 不在本批范围）。
> **源冻结线**：orz **`6f23bbbf`**（`feat/fusion-architecture`）＝ 0be RLI 在线自适应批
> `0b7b89dd`（六件）＋ bump 两文件两行。上一载体：双平台 0.6.6（`6efd192f` 冻结，已发布
> `v0.6.6`）。**本批免预检**（见 §3）。

## 1. 前置基线（0.6.6 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 53,393,408 | `C2F31EA73C5DFBA9B0AD5666FE3604056B381307A37308404EC641A1D4BA59B3` |
| Windows | `orz-signer.exe` | 6,731,264 | `78308D97C525658A81FAA091C2AEC50AE2103893F4658CE5535411AF421A2831` |
| Windows | `orz-acaf-provision.exe` | 6,639,616 | `DFAEC758DEAD232E507D487DF602E155AFE4E855B0713B35CF41F79CBDEA8742` |
| Linux | `orz` | 111,911,544 | `601ee1db07687d6c72e01ce2c7259d0b745f6dbf72e921ff77cf61cad6d08e63` |
| Linux | `orz-signer` | 1,399,552 | `8b6700c49aceb9841de8114c1a0a4fae357fdc98dd94929ce61be13911fdcf24` |
| Linux | `orz-acaf-provision` | 1,218,280 | `d1c884aa792f689ef67ea35b2f6d6576ea4339328336830b4753b3345588b001` |

换装前逐件复制为备份链 **`.0.6.6-bak`**（Windows `D:\tb-eval\orz-windows\`；
Linux `D:\tb-eval\orz-linux\`）——备份件哈希与上表**逐位一致**（在役件未被外力改动）。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.6 → 0.6.7`
  （orz 提交 **`6f23bbbf`**，推 `cli`）。版本号**只在显式 bump 时递增**，不随重建自动进位。
- 本批源冻结线＝`6f23bbbf`；0be 的六件改动（`0b7b89dd`）首次进入在役载体。

## 3. 预检：本批按用户令免除（口径收窄）

- 用户令「**做预检的必要性不大，一般情况下不必预检**」⇒ 本批**不做**双平台预检，
  bump 后直接 clean 全量重建（上一轮 0be 报告 §8.1 登记的「待用户裁决」项**就此收窄**：
  默认不预检；仅当出现只能在另一平台验证的条件编译面、或用户明确点名时才做）。
- 与 0be §8.1 的事实性结论一致：该项此前**只在 066 档（0bc 批）出现过一次**，
  成因是该轮用户令「请先重建」＋ 0bc §8 点名的 Linux `#[cfg(windows)]` 面。

## 4. Windows 三件套（宿主 release，clean 全量）

- `cargo clean`：移除 **42,452 文件／22.5 GiB**（15.2 s）；D: 余量 **20.0 → 41.0 GB**。
- 构建：`cargo build --release -p orz-bin -j 4`（`PROTOC=D:\tb-eval\.tools\protoc-25.3\bin\protoc.exe`、
  `CARGO_INCREMENTAL=0`）；**exit 0，14m35s（890.6 s 含清理）**，日志 `.tmp-b067-windows-build.log`
  （066＝15m02s、065＝14m40s 同量级；`ORZ-DEV-LINKER-CRASH-001` 未复现）。

| 文件 | 尺寸 (B) | 与 0.6.6 差异（尺寸） | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 53,506,048 | +112,640 B | `A0FA066F434DDCEE3D987194C790C63D2FDC0798BCC681979C12C7C2C7027A9C` |
| `orz-signer.exe` | 6,729,728 | −1,536 B | `C7B1044251622655798BE2FA2C3605FD5A4AECAA4BBB9D0C118BF3410CF28179` |
| `orz-acaf-provision.exe` | 6,638,592 | −1,024 B | `132AFF660AD560DA3B05E0C3545FA9D119691DB71FB25F8F73A1A4D5D676017C` |

## 5. Linux musl 三件套（Docker，直写换装位）

- 构建：`rust:1.97-slim` ＋ `build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001 契约）——
  挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`（暖缓存）＋
  `-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.6-bak`）；
  **exit 0，676 s（11m16s，含 apt／musl target／静态 ripgrep 装配）**，日志 `.tmp-b067-linux-build.log`。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.6 差异（尺寸） | SHA256 |
|---|---:|---:|---|
| `orz` | 112,053,424 | +141,880 B | `e9f477562bbcd15ea2d933d03ab02aa94b763edf745ad614fa434653e2643708` |
| `orz-signer` | 1,400,944 | +1,392 B | `a866d379a80f510c5cfe227cc5db815ab55d6e734628a536e4b2f20e27ad9d96` |
| `orz-acaf-provision` | 1,219,808 | +1,528 B | `bf613e3ddd87d384121753ebc3fc2a8571c2e24c4cd34ef33a0731dfa2e264bb` |

- ELF 静态核验（`.tmp-b066-elfcheck.py`，逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋
  `e_machine=62`（x86-64）＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–066 同形态）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.6 备份件）

| 字面量 | Win 0.6.6 | Win 0.6.7 | Lin 0.6.6 | Lin 0.6.7 | 判读 |
|---|---:|---:|---:|---:|---|
| `λ̂=` | 0 | **1** | 0 | **1** | 0be① λ̂ 读数列（rli 面）进件 |
| `λ̂`（汇总） | 0 | **11** | 0 | **11** | 0be① 符号面进件 |
| `pred1_prog` | 0 | **10** | 0 | **6** | 0be② 短视锚点 feature 名进件 |
| `pred1_err` | 0 | **10** | 0 | **6** | 0be② 同上（err 通道） |
| `p1(1T̂)` | 0 | **6** | 0 | **6** | 0be② 短视档渲染色标进件 |
| `pred(10T̂)` | 1 | **0** | 1 | **0** | 0be② 写死 horizon 字面量**退役**（改按通道格式化） |
| `会话提示：` | 3 | **6** | 3 | **6** | 0be④ 繁杂度三档提醒文案进件（+3） |
| `越过本会话自校准 q85=` | 0 | **1** | 0 | **1** | 0be④ q85 档阈值渲染进件 |
| `建议适时换新对话` | 0 | **1** | 0 | **1** | 0be④ q95 档换会话建议文案进件 |
| `复杂`（汇总） | 1 | **2** | 1 | **2** | 0be④ 语义字面进件 |
| `q85`（3 字节短字面量） | 0 | 3 | 3 | 7 | 参考项——短字面量有偶然命中，**不作判据** |
| `ORZ_LIF_RLI_SHADOW` | 7 | 7 | 7 | 7 | 0am 影子开关保持 |
| `slow_prog`／`fast_prog` | 12／12 | 12／12 | 8／8 | 8／8 | 0am 模态对保持 |
| `rli.history` | 2 | 2 | 2 | 2 | 0am 域级定位面保持 |
| `ORZ_JOB_CPU_RATE_PERCENT` | 1 | 1 | 1 | 1 | 0bc CPU 去顶保持 |
| `ORZ_JOB_ACTIVE_PROCESS_LIMIT` | 1 | 1 | 1 | 1 | 0bc 进程上限 env 保持 |
| `资源软提示` | 2 | 2 | 2 | 2 | 0bc 软提示文案保持 |
| `utf-8-lossy:` | 1 | 1 | 1 | 1 | 0as 降级读数保持 |
| `[EARLY_DELIVERY]` | 0 | **1** | 1 | 1 | Windows 侧首次物化（**跨批摆动**，065 已判不据以改码） |

> **注一（`pred(10T̂)` 1→0 非回退）**：0.6.6 的渲染行是写死字面量
> `pred(10T̂)={:.2}`（0am 单一 10·T̂ 口径）；0be 四项② 改为按通道格式化
> `pred({:.0}T̂)={:.2} p1(1T̂)={:.2}`（Err=10／Deny=30／Stall=2／Slow=1），
> 故旧字面量退役、新色标 `p1(1T̂)` 与新 feature 名进件——**同一改动的两面**。
> 格式化占位符本身不构成可核字面量（`format!` 在占位符处切分字符串），故不单列。
> **注二（核证边界）**：字面量核证只回答「面是否进件」，**不构成语义等价证明**；
> 功能级读数由真机狗粮轮收取。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.6-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260922-067`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：
  keystore **逐位未动**（`installation-key.dpapi` `2408F596…`／`installation-key.json`
  `C491FC95…`；`keystore/` 内两件 `F37556AB…`／`2AA80CB8…` 前后一致）；
  回显 `binary_sha256=c7b1044251622655798be2fa2c3605fd5a4aecaa4bbb9d0c118bf3410cf28179`
  ↔ 换装位 `orz-signer.exe` **逐位一致**。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal **exit 1**。
- 宿主装配复核：`scripts/dogfood_launch.ps1 -TaskFile … -DryRun` 断言全过
  （cwd＝`D:\CLI` 断言通过、载体哈希回显 `A0FA066F434D…`、ACAF fail-closed=True、
  `PROTOC`／`GROK_HOME` 就位、flags `--real --allow-write --allow-shell --allow-network`）。
  版本列显示 `unknown` 系 Rust 载体无 VersionInfo 的**既有口径**（2026-09-18 已登记），非本批缺陷。
- **操作瑕疵（如实登记）**：换装后误以 `orz.exe --version` 试探——该旗标不被识别，裸跑进 TUI
  于无 TTY 下**挂起**（PID 9228，CPU 0.64 s），随即终止；`git status` 与 `.gsa/` 运行痕迹
  **均无残留**，无副作用。**口径**：验版本请走启动器回显，勿裸跑载体。
- Linux：构建脚本直写 `/out`（＝`D:\tb-eval\orz-linux`），在役 0.6.6 已预留 `.0.6.6-bak`；
  评测容器运行期自行 provision，不涉 Windows 侧 manifest。

## 8. 本批未做（边界）

1. **不打包、不发行**：本批用户令只有「重建」；上轮 0.6.6 的打包与 GitHub Release 另有明令。
   若需发行 0.6.7，沿用 `.tmp-b066-package.ps1` 形态（改 `$ver`／暂存目录）即可，属下一步、待令。
2. **未跑真机狗粮轮**：本批只完成换装与机械核证；RLI 常开后的实际读数、
   0bc S3 三项资源面读数（CPU 去顶后墙钟／commit 临限通知首现／进程上限读数）、
   0be 繁杂度投递链与侧车落盘的真机覆盖，仍待 0bd／0bf 长会话轮收取。
3. **LIF 退役裁决**：待 RLI 转正后首轮真机读数（0bf ①）。

## 9. 记账面（pin、清单、提交与推送）

- orz：`0b7b89dd`（0be 批）→ **`6f23bbbf`**（bump），均推 `cli`。
- 父仓：本档 ＋ 子模块 pin ＋ `orz_source_manifest.sha256` 重算（条数 **1462 不变**——
  本批无新增／删除文件；差异 **2 行**＝`crates/orz-bin/Cargo.toml` 与 `Cargo.lock` 各一行），
  推 `origin main`；门禁 `valid: true`。

## 10. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b067-windows-build.log`／`.tmp-b067-linux-build.log`／`.tmp-b067-literals.py`／
`.tmp-b067-literals.txt`／`.tmp-b067-lits-extract.py`／`.tmp-orz-commit-msg-bump067.txt`；
两平台 `.0.6.6-bak` 备份链与 `signer-manifest.json.bak-20260922-067` 保留。

## 11. 关联

[`0bc 复合狗粮轮总结`](0BC_COMPOSITE_DOGFOOD_2026-09-21.md) ／
[`0be 报告`](0BE_RLI_ONLINE_ADAPTIVE_2026-09-21.md) ／
[`066 重建与发行`](066_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md) ／
[`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／ [`TODO`](../../TODO.md)。
关键词：0.6.7、双平台、载体重建、免预检口径、clean 全量、暖缓存、ACAF 重 provision、
Linux musl static-pie、字面量核证、λ̂、pred1 锚点、繁杂度、pin 6f23bbbf。
