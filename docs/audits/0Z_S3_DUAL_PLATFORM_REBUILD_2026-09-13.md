# 0z S3 双平台重建记录（2026-09-13）

> 入口：BACKLOG **0z** / TODO P0-0z——0z S1（A+D+F）+ S1.1 复核收口 + S2 机制
> 与合约收口 + S2 全面复审返工 + S2R 裁决 15/16 + P1-4 落码全部落在 0.4.3
> 冻结基线之后，本批 S3 = 版本 bump **0.4.3 → 0.5.0**（用户指示：本轮直接打
> 0.5.0 并发布 GitHub Release，跳过 0.4.4 过程版本）+ 双平台三件套重建 +
> 冒烟 + ELF 核验 + 符号核证 + manifest 重算 + **GitHub Release v0.5.0 发布**。
> 基线：orz `187cb9d7`（版本 bump，两文件两行）→ 构建中发现第三处 Linux 断裂，
> 修复提交 orz `1f13e5ec` 后按新 HEAD 双平台重建；父仓库构建前 HEAD
> `623c37d2`。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0z-s3-20260913\`（本地，
> gitignore 覆盖，不入库）；staging：`_windows_high_nist\staging-0z-s3-20260913\`。

## 0. 版本 bump 与源冻结

- bump：orz `187cb9d7`，两文件两行（`crates/orz-bin/Cargo.toml` + `Cargo.lock`）。
- 构建期间发现并修复 **第三处 Linux 构建断裂**（orz `1f13e5ec`）：orz-host
  `lib.rs` 无条件读取 `SpawnObservation.job_handle_dup`（字段
  `#[cfg(windows)]`，S2R `a42fa0c3` 引入；E0609）——P1-4 批只修了
  xai-tty-utils 两处，本处漏网，musl 载体重建实测暴露。修复语义：非 Windows
  平台 `LiveCallJob` 登记 `job_handle = 0`（即既有「0 句柄仍登记进重档集、
  terminate 步警告跳过」语义），关句柄臂 `cfg(windows)` 门控。
- **源冻结基线 = orz `1f13e5ec`**；Windows 侧首轮产物（`187cb9d7` 时点）作废
  重建，发布资产与冻结源一致。
- 0.4.3 载体六件哈希重建前已录
  `D:\tb-eval\orz-0.4.3-carrier-hashes-before-rebuild.txt`，与
  [0v 第二批 S3 审计](0V_S3_BATCH2_DUAL_PLATFORM_REBUILD_2026-09-12.md) 逐条
  吻合（备份口径）。

## 1. Windows 三件套（宿主 release，`1f13e5ec` 基线）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`）。
  首轮 3m48s（`187cb9d7`）+ 修复后增量重建 33.7s，`CARGO_EXIT=0`，编译行
  `Compiling orz-bin v0.5.0`。`orz-host` lib 1 条既有 warning（非本批引入）。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，
  SHA256 逐对 OK）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,742,144 | `a45b60e54b080b655a210f84647c91d56687872d8f7ee48037aedc67625b5c74` |
| orz-signer.exe | 6,742,528 | `d19a1394bf16149ed008904a6b6d865cd5bb899c243f9e0b03818a735f076169` |
| orz-acaf-provision.exe | 6,642,176 | `34a4a91a9a31de27daa3f1920216bb43e73ab6d47f33de019b2860cd2893ee03` |

- 与 0.4.3 差异：`orz.exe` 52,419,072 → **52,742,144 B**（+323,072 B，0z
  S1/S2/S2R/复审返工/P1-4 全量代码增量）。
- 冒烟（`windows-smoke.txt`）：provision usage exit 1；signer manifest 缺失
  fatal exit 1——与上批形态一致。`orz.exe --version` 无该子命令（0.4.x 同），
  输出级冒烟不做 TUI 起手观察（同 0.4.2/0.4.3 边界登记）。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约 + `MSYS_NO_PATHCONV=1`），日志
  `D:\tb-eval\orz-linux\build-20260913-050c.log`，编译行
  `Compiling orz-bin v0.5.0`，`BUILD_EXIT=0`（`-j 1`，约 35m）。
- **操作记录**：首次运行 docker daemon 未启动（Docker Desktop 冷机）——启动
  后重试；第二次 `rustup target add` 在 static.rust-lang.org TLS 握手中断
  （瞬时网络，上批同脚本成功）；第三次成功。首轮成功构建（`187cb9d7` 时点）
  因 Windows 侧断裂修复作废，`050c` 为终版。
- 产物（`D:\tb-eval\orz-linux` 直出；0.4.3 六件哈希以
  `orz-0.4.3-carrier-hashes-before-rebuild.txt` 备查）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 109,982,680 | `393eee34dd0357cab088f289fa1068a39294a13fff48d80d33c50a40684dd623` |
| orz-signer | 1,397,360 | `3d5c8155ef2b93e528607ee639f1029e09e14238631bbabe3db1ad939c8cf330` |
| orz-acaf-provision | 1,216,288 | `73fcaeda51eeb3f8883fb182d4129a66fda0333693a4143cefee244b223189d5` |

- 与 0.4.3 差异：`orz` 109,540,832 → **109,982,680 B**（+441,848 B，0z 全量
  代码增量）。
- 静态核验（`linux-elf-static-check.txt`，`elf-phdr.py`）：三件均
  `e_type=0x3`（ET_DYN）+ `e_machine=0x3e`（x86-64）+ **PT_INTERP=0**
  （musl static-pie），`static-pie OK`。
- **双向加载冒烟全绿**（预期 exit 1 形态三件一致）：
  bookworm-slim（glibc）与 alpine:3.20（musl）两侧 provision usage exit 1 /
  signer manifest 缺失 fatal exit 1 / `orz --version` 无 TTY `tui io error`
  exit 1（`linux-elf-smoke-bookworm.txt` / `linux-elf-smoke-alpine.txt`）。
  alpine 侧 signer 退出码经 `sh` 管道取 `$?` 显示 0（无 PIPESTATUS），fatal
  文案与 bookworm 逐字相同，形态判定以文案为准。

## 3. 接线符号核证（双平台 orz 二进制字节串命中）

扫描器 `scan-binary-symbols.py`（本批追加 0z 事件族 11 项标记 + `0.5.0`）。
证据：`binary-symbols-win.txt` / `binary-symbols-linux.txt`。

| 标记 | Windows orz.exe | Linux orz |
|---|---|---|
| `resource_insufficient`（预检门拒绝信封） | 1 | 1 |
| `host_resource_snapshot` | 3 | 3+ |
| `host_resource_denied` | 15 | 15 |
| `resource_exhausted` | 16 | 16 |
| `process_tree_reaped` | 2 | 2 |
| `reclaim_performed` | 16 | 16 |
| `resource_limit_hit` | 15 | 15 |
| `run_terminated` | 2 | 2 |
| `process_trees` | 1 | 8 |
| `DegradedDropped` | 4 | 4 |
| `not_attempted` | 1 | 2 |
| `serp-attempts` | 48 | 15 |
| `[INITIAL_ROUND_INQUIRY v0.1]` | 1 | 1 |
| `low_quality` / `engine_attempts` | 23 / 14 | 23 / 14 |
| `0.5.0` | 16 | 2 |

- **0z 全部接线标记双平台命中**（门/回收/进程树/降级/七族事件）；
  前批（0v/0x）标记保持。
- 0 计数三项均为已登记非缺陷：`initial_round_inquiry` / `ordered_engines`
  （Windows）/ `degraded_complete`——执法 CLI 侧资源或 debuginfo 路径段，
  不链入主二进制，与上批同形。

## 4. 仓库门禁与发布

- manifest 重算 **1446 条**（bump 与断裂修复共 3 文件改动面均在 orz 子模块
  内，manifest 粒度不变）+ `check_repository` **`valid: true` /
  `error_count: 0`**。
- 发布：**GitHub Release v0.5.0**（`SilverWhite/CLI`，用户指示；orz 子模块
  upstream `xai-org/grok-build` 无发布权限，发布面沿用 0.4.0 以来的
  SilverWhite/CLI 惯例）——资产
  `orz-0.5.0-linux-x86_64.tar.gz`（26,470,950 B，
  `eda2d8b4498421808093b2ab8ee9a3d19262611a80bc0cebb333e3b389cf0e31`）+
  `orz-0.5.0-windows-x86_64.zip`（26,434,770 B，
  `2bb64b5b7c9a136ec60cffecbb5633b254aa9013a366127b4850135500d86beb`），
  双包内含三件二进制 + README（包内容清单已核验）；release notes 覆盖
  0z 主体 + 0x/0v/P1-4 + 三处 Linux 断裂修复。
- 发布核验：`gh release view v0.5.0` draft=false、两资产 size 逐字节吻合。

## 5. 结论与边界

- **S3 闭合**：0.5.0 双平台三件套 + manifest + 门禁重建完成，ELF 静态核验 +
  双向加载冒烟绿，接线符号双平台全命中，载体
  `D:\tb-eval\orz-windows` / `orz-linux` 已刷新为 0.5.0，GitHub Release
  v0.5.0 已发布。
- **0z S4 实机复验待放行**（唯一剩余步骤，判据 1–13）：真机长任务复跑（含
  重活路径）+ 满盘注入 + abort 注入 + 编码样本；0v-C 不搭车（已另行闭合）。
  S2 复审登记的 S3/S4 残段（F-BE-4/EV-7：`run_terminated{resource_exhausted}`
  + `resource_limit_hit` producer 读回；F-BE-7 残段更早 run scratch 层；
  F-C-5/6/11）随 S4 取证核验。
- 未验证：本批只做重建与加载冒烟 + 字节串核证；资源门/回收/树杀的实机行为
  仍须 S4 复跑取证。
