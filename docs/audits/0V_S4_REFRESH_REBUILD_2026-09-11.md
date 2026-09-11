# 0v S4 复跑重建记录（2026-09-11）

> 入口：BACKLOG **0v** / TODO P0-0v——S3′ = 0v S4 首轮受阻后的载体重建
> （F1 权限门修复 orz `340fe4a7` 落在 0.4.1 冻结基线之后），出口 = 双平台
> 三件套 + manifest 重建，构建冒烟绿，供 0v S4 复跑取证判据 1/5/6/7。
> 基线：orz `b81c90ac`（本批版本 bump，`orz-bin` 0.4.1 → **0.4.2**；
> 口径同 0.3.1 / 0.3.2 / 0.4.1 过程验证版本）；父仓库构建前 HEAD `6697e13b`，
> 两侧工作树净。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0v-s4-refresh-20260911\`
> （本地，gitignore `_windows_high_nist/**/evidence-*` 覆盖，不入库）。
> **声明**：本次重建只刷新载体目录 `D:\tb-eval\orz-windows` / `orz-linux`；
> 0.4.0 正式发布资产（`orz-0.4.0-{windows-x86_64,linux-x86_64}\` 与两个归档）
> 未被触碰，0w 探针的发布口径 SHA256 锁定值仍然成立。

## 0. 版本 bump（源冻结基线）

- 提交：orz `b81c90ac`，两文件两行（`crates/orz-bin/Cargo.toml` + `Cargo.lock`）。
- 理由：0x S3 冻结的 0.4.1 基线之后落了 0v F1 修复（orz `340fe4a7`，
  宿主侧 `access_kind` 的 `browser_control` 分档映射 + 跨表护栏测试）；
  不 bump 会让「0.4.1」同时指向「修 F1 前」与「修 F1 后」两个不同制品。

## 1. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`，增量
  38.68s），`CARGO_EXIT=0`，日志 `build-win.log`，编译行
  `Compiling orz-bin v0.4.2`。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，哈希逐对
  吻合）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,254,208 | `02dc8cf96b76f76bbc4599b7398bf4e8301120d6faf7305f36fb63e9332034d4` |
| orz-signer.exe | 6,742,528 | `ad1343dc5d8c6473ecc068b64cd98e15bf039bd64ad705b6e81455e236310fb1` |
| orz-acaf-provision.exe | 6,642,176 | `824cc76c3923b85cdeeb0da91cf14ca842d774f30e2d81b4a09cc436fce58cbf` |

- 与上批（0.4.1）差异：`orz.exe` 52,253,696 → **52,254,208 B**（+512 B），
  为 F1 的 `access_kind` 分支与守卫测试以外的主二进制增量；`orz-signer.exe` /
  `orz-acaf-provision.exe` 字节数与上批完全一致。
- staging：`_windows_high_nist\staging-0v-s4-refresh-20260911\` + `SHA256SUMS.txt`
  （gitignore `_windows_high_nist/staging-*/` 覆盖，本地留存）。
- 冒烟（`windows-smoke.txt`）：orz-acaf-provision usage exit 1；orz-signer
  `signer manifest not found` fatal exit 1——与上批形态一致。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约：`D:/CLI:/orz` + `-w /orz/orz` + `/target` 增量
  + `/out`），`BUILD_EXIT=0`，编译 **28m43s**（`-j 1`，与上批同口径），日志
  `build-linux.log`，编译行 `Compiling orz-bin v0.4.2`。
- **装置观察（不影响结论，供后续成本估计）**：本批 `/target`
  （`D:\tb-eval\orz-target`）持有上批 0.4.1 的完整产物（`x86_64-unknown-linux-musl/
  release/{orz,orz-signer,orz-acaf-provision}` 与其 `.d` 齐备，5.75 GB），但 cargo
  **仍从 `proc-macro2` 起重编全依赖图**（等价于全量）；领先假设为容器每次
  `--rm` 重建、只挂了 `cargo-config.toml` 而**未挂 CARGO_HOME**，注册表源码/依赖
  指纹每轮重新推导。原因未隔离，本条只作事实登记。
- 产物（`D:\tb-eval\orz-linux`）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 109,387,800 | `ed59ffb5cfc71f375595e867564e18fcd9866ae37211acfdf829bee58ec1aa57` |
| orz-signer | 1,396,232 | `3bed48e0da4f583a74cffac4327a678ff194e170a314fd5e032158e98cd08d7c` |
| orz-acaf-provision | 1,214,816 | `44d0389fea3e45c65d64f3f0289f59e009a8d229204d94674db648899ef4a8b4` |

- 与上批（0.4.1）差异：`orz` 109,383,984 → **109,387,800 B**（+3,816 B）；
  `orz-signer` 1,396,368 → 1,396,232 B（−136 B）；`orz-acaf-provision`
  1,214,832 → 1,214,816 B（−16 B）。后两者不含 F1 改动面，字节级微差来自
  同源码的重链接（与上批非同一 try 的非确定性段），非语义变化。
- 静态核验（`linux-elf-static-check.txt`，宿主侧 ELF 程序头解析器
  `elf-phdr.py`——bookworm-slim 无 `readelf`）：三件均 `e_type=0x3`（ET_DYN）
  + `e_machine=0x3e`（x86-64）+ **PT_INTERP 计数 0**（musl static-pie）。
- bookworm 冒烟（`linux-elf-smoke.txt`）：provision usage exit 1 / signer
  manifest 缺失 fatal exit 1 / `orz --version` 无 TTY `tui io error` exit 1
  ——三件加载执行全过，形态与上批一致。

## 3. 接线符号核证（双平台 orz 二进制字节串命中）

证据：`binary-symbols-both.txt`（扫描器 `scan-binary-symbols.py` 随证据留存）。

| 标记 | Windows orz.exe | Linux orz |
|---|---|---|
| `[车道:本地浏览器检索\|推荐首选]` | 2 | 2 |
| `[车道:原生检索]` | 2 | 2 |
| `browser_launch_result` | 3 | 3 |
| `browser_control` | 75 | 59 |
| `retrieval_enabled` | 5 | 8 |
| `wait_load` | 4 | 7 |
| `setmkt=en-US` | 1 | 1 |
| `low_quality` | 11 | 11 |
| `engine_attempts` | 2 | 2 |
| `[INITIAL_ROUND_INQUIRY v0.1]` | 1 | 1 |
| `[/INITIAL_ROUND_INQUIRY]` | 1 | 1 |
| `开局问询（一次性，非强制模板，不打断动作）` | 1 | 1 |
| `post_tool_batch_gap` | 7 | 7 |
| `template_sha256_initial_round` | 1 | 1 |
| `0.4.2` | 12 | 106 |

边界说明：`initial_round_inquiry`（第 34 族法官名）在两平台 `orz` 二进制内命中
**均为 0**——与 0q 的 `failure_agg_coverage` 同形，家族名不链接进 orz 主二进制，
属执法 CLI 侧资源，**非缺陷**。F1 修复本身是分支逻辑而非字符串，双平台字节串
核证只能确认 0v/0x 的接线面完整；F1 的语义正确性由 orz 源码 + 跨表护栏测试
（`read_only_tools_never_fall_into_the_edit_bucket`）+ S4 复跑的 `permission_decision`
事件面共同取证。

## 4. 仓库门禁

- `python scripts/check_repository.py` → `valid: true`、`error_count: 0`、
  `orz_source_manifest_files: 1441`。
- manifest 重算差异面 = 恰两条（`Cargo.lock`、`crates/orz-bin/Cargo.toml`），
  与 bump 改动面逐条对应；顺序按既定纪律为「先 orz 提交 → 再生成 manifest」。
- 构建产物仅落 gitignore 覆盖的 `target/`、证据/staging 目录；构建后 orz
  子模块工作树净。

## 5. 结论与边界

- **S3′ 闭合**：双平台三件套 + manifest 重建完成，构建冒烟绿（出口达成），
  载体 `D:\tb-eval\orz-windows` / `orz-linux` 已刷新为 0.4.2。
- **0v S4 复跑待放行**（唯一剩余步骤）：判据 1/5/6/7 取证沿用既有执行器
  `scripts/run_0x_0v_s4_live_verify.py`（已含 `D:\tb-eval\browser\chrome-linux`
  → `/opt/chrome-linux` 只读挂载）与 journal 分析器
  `scripts/analyze_0x_0v_s4_journal.py`。
- 未验证：本次只做重建与加载冒烟；0t 双车道 / 0v SERP 的实机行为仍须 S4 复跑
  取证。Windows 侧只做确定性退出码冒烟（未复做 TUI 起手观察）。
