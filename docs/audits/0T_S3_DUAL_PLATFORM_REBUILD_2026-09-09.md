# 0t S3 双平台重建记录（2026-09-09）

> 入口：BACKLOG **0t** / TODO P0-0t（审查处理阶段表 **P8**）——S3 = 双平台
> 三件套 + manifest，出口 = 构建冒烟绿。S4 实机复验（P9）待续，判据见
> [`RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md`](../RETRIEVAL_SUBAGENT_DUAL_LANE_DESIGN_2026-09-09.md) §5。
> 基线：orz `92875fd5`（0t S2 收口；版本串 0.3.2，与 0q 同口径**不 bump**——
> S4 实机复验以证据目录 commit/哈希区分，不依赖版本串）；父仓库 HEAD
> `b2dc93e`，构建前两侧工作树净。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0t-s3-20260909\`（本地，
> gitignore `_windows_high_nist/**/evidence-*` 覆盖，不入库）。

## 1. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`，
  增量 1m45s），构建日志 `build-20260909-0t-win.log`，`CARGO_EXIT=0`，
  编译行 `Compiling orz-bin v0.3.2`。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，
  哈希逐对吻合）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 51,982,336 | `72b18e0082114c6f1666cb70f6f238d7e693eb5b91bf7129d32eb24f2478e28d` |
| orz-signer.exe | 6,737,920 | `4cda6ef6113454ad171863ac810b02671914e691c25619e33461ad35105462a1` |
| orz-acaf-provision.exe | 6,642,176 | `67b048e61513f9bb84f60204e27bfb0e148622cac8cff272d206d5f73d46e4c9` |

- staging：`_windows_high_nist\staging-0t-s3-20260909\` + `SHA256SUMS.txt`
  （gitignore `_windows_high_nist/staging-*/` 覆盖，本地留存）。
- 冒烟（`windows-smoke-20260909-0t.txt`）：orz-acaf-provision usage
  exit 1；orz-signer `signer manifest not found` fatal exit 1；orz TUI
  启动就绪（15s 观察后主动终止，非崩溃）——全部预期形态。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun.sh`（
  ORZ-BUILD-MOUNT-001 契约：`D:/CLI:/orz` + 工作目录 `/orz/orz` +
  `/target` 增量缓存 + `/out`），构建日志 `build-20260909-0t-linux.log`，
  `BUILD_EXIT=0`，编译行 `Compiling orz-bin v0.3.2`。
- 网络注记：构建期机场上游 AWS 故障（官方 rust-lang 端点约 1MB/min）；
  用户开启 Clash 全局后 rustup 组件与主构建快速完成，无产物影响。
- 产物（`D:\tb-eval\orz-linux`，2026-09-09 15:19 HKT）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 109,066,368 | `93aa8dbd383c8b60da0314837584949005ee037765ce1c5fe6d919672d5ba36b` |
| orz-signer | 1,396,224 | `83c8644b904d70076ed048f72330aa759d23af24b8c6ee371a4f95eeb3a2b80b` |
| orz-acaf-provision | 1,214,656 | `4a694172cd82c529f2d10beab8c0151c5a54aacfe0a1a969c98a01ba81edbb72` |

- 静态：`file` = ELF 64-bit static-pie（无 PT_INTERP，musl 静态）。
- bookworm 冒烟（本地 `debian:bookworm-slim`，`linux-bookworm-smoke-20260909.txt`）：
  provision usage exit 1 / signer manifest 缺失 fatal exit 1 /
  `orz --version` 无 TTY `tui io error` exit 1——三件加载执行全过。

## 3. 0t 接线符号核证（双平台 orz 二进制字符串命中）

| 标记 | Windows orz.exe | Linux orz |
|---|---|---|
| `browser_control` | 21 | 25 |
| `browser_launch_result` | 3 | 3 |
| `[车道:本地浏览器检索|推荐首选]` | 2 | 2 |
| `[车道:原生检索]` | 2 | 2 |
| `retrieval_enabled` | 5 | 8 |
| `0.3.2` | 25 | 31 |

## 4. 仓库门禁

- `python scripts/check_repository.py` → `valid: true`、`error_count: 0`、
  `orz_source_manifest_files: 1440`；`git diff --check` 净；构建后父仓库与
  orz 子模块工作树均净（构建产物仅落 gitignore 覆盖的 target/证据/staging）。

## 5. 结论与边界

- **S3 闭合**：双平台三件套 + manifest 重建完成，构建冒烟绿（出口达成）。
- S4 实机复验（判据设计 §5）+ 宿主机日常可用性验证（执行代理操作，
  R-D1 TUI 载体起手裁决）待续；`D:\tb-eval\orz-windows` / `orz-linux`
  已刷新为本次产物，供 S4/VM stage sync 使用。
- staging/evidence 为本地件不入 git（既有 gitignore 纪律）；版本 0.3.2
  不 bump（本次为 0t 代码随 0.3.2 基线的重建轮，与 0q 口径一致）。
