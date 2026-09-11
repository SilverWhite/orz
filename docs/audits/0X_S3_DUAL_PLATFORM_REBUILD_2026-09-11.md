# 0x S3 双平台重建记录（2026-09-11）

> 入口：BACKLOG **0x** / TODO P0-0x——S3 = 双平台三件套 + manifest，出口 =
> 构建冒烟绿。S4 实机复验（验证「开局一次、动作中不复发」）待续，判据见
> [`INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md`](../INITIAL_ORIENTATION_DECLARATION_DESIGN_2026-09-11.md) §8。
> 基线：orz `a6f902ef`（本批版本 bump，`orz-bin` 0.4.0 → **0.4.1**；0v S1–S2 与
> 0x S1–S2 均落地于 0.4.0 发版之后，故按 0.3.1/0.3.2 过程验证版本口径升一格冻结
> 源基线）；父仓库构建前 HEAD `98a81e3d`，两侧工作树净。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0x-s3-20260911\`（本地，
> gitignore `_windows_high_nist/**/evidence-*` 覆盖，不入库）。
> **声明**：本次重建只刷新载体目录 `D:\tb-eval\orz-windows` / `orz-linux`；
> 0.4.0 正式发布资产（`orz-0.4.0-{windows-x86_64,linux-x86_64}\` 与两个归档）
> 未被动过，0w 探针的发布口径 SHA256 锁定值仍然成立。

## 0. 版本 bump（源冻结基线）

- 提交：orz `a6f902ef`，两文件两行（`crates/orz-bin/Cargo.toml` + `Cargo.lock`），
  口径同 `d21b883e`（0.3.1）/ `7b00bbc9`（0.3.2）——重建批开始前一次性冻结源。
- 理由：0.4.0 里程碑发版（`a467d0f9`）之后又落了 0v（浏览器车道分类修正 +
  Bing/DDG 反污染与低质量域名加权 + 检索动作引擎链）与 0x（初始轮中立问询）
  两组 S1–S2 代码；不 bump 会让「0.4.0」同时指向两个不同制品。

## 1. Windows 三件套（宿主 release）

- 构建：`cargo build --release -p orz-bin`（PROTOC=`orz\bin\protoc.exe`，增量
  1m38s），`CARGO_EXIT=0`，日志 `build-20260911-0x-win.log`，编译行
  `Compiling orz-bin v0.4.1`。
- 产物（`orz\target\release` → staging → `D:\tb-eval\orz-windows` 同步，哈希逐对
  吻合）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,253,696 | `6fd753850830d614bf5f7393443a57dd180dab23286dfade1c47e95a219c871f` |
| orz-signer.exe | 6,742,528 | `16f4a4568ecd45742121f05f4a6592464e3dfb0ddf9f1fa3ac6cc54c693e72ec` |
| orz-acaf-provision.exe | 6,642,176 | `7f068c8fc65eb2e3684b7610ffef6e70ddd7690bb0102ba9d4b8ede33753d293` |

- staging：`_windows_high_nist\staging-0x-s3-20260911\` + `SHA256SUMS.txt`
  （gitignore `_windows_high_nist/staging-*/` 覆盖，本地留存）。
- 冒烟：orz-acaf-provision usage exit 1；orz-signer `signer manifest not found`
  fatal exit 1；orz TUI 启动就绪（15s 观察后主动终止、非崩溃，stdout 8,240 B）
  ——三者与前批形态一致。

## 2. Linux musl 三件套（Docker）

- 构建：`rust:1.97-slim` + `D:\tb-eval\build_orz_aliyun_trixie.sh`
  （ORZ-BUILD-MOUNT-001 契约：`D:/CLI:/orz` + `-w /orz/orz` + `/target` 增量
  + `/out`），`BUILD_EXIT=0`，编译 41m04s（墙钟 46m26s），日志
  `build-20260911-0x-linux-official.log`，编译行 `Compiling orz-bin v0.4.1`。
- **首轮失败记录**：先按 `build_orz_aliyun.sh`（USTC 镜像变体）跑，两分钟内
  因 `mirrors.ustc.edu.cn` 三个 InRelease 全 `SSL unexpected eof` 而
  `Unable to locate package musl-tools/protobuf-compiler/ripgrep`，
  `BUILD_EXIT=100`（日志 `build-20260911-0x-linux.log`）——与 0.4.0 发布轮
  同一症状，属镜像侧网络，非仓库问题；改官方源变体即通过。
- `/target` 为冷缓存（历史增量目录在磁盘清理轮已删除），故为全量构建。
- 产物（`D:\tb-eval\orz-linux`，2026-09-11 08:30 HKT）：

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz | 109,383,984 | `4b83de75e34b2f289b02162967a47966c89a7ba83ce96c26c661f88c86ae0415` |
| orz-signer | 1,396,368 | `04ea6bb362e200476693267053260e785953c1f5e459fabc4e3555ef5db2bbba` |
| orz-acaf-provision | 1,214,832 | `bc55c52c01a6f2c90723639921057a5ff2bcbb32d78aa7d536d139a1fa85b5c9` |

- 静态：三件均 `ET_DYN` + `machine=x86-64` + **PT_INTERP 计数 0**（musl
  static-pie，无动态解释器）；证据 `linux-elf-static-check-0x-s3.txt`。
- bookworm 冒烟（本地 `debian:bookworm-slim`，`linux-bookworm-smoke-0x-s3.txt`）：
  provision usage exit 1 / signer manifest 缺失 fatal exit 1 / `orz --version`
  无 TTY `tui io error` exit 1——三件加载执行全过。

## 3. 0x 接线符号核证（双平台 orz 二进制字节串命中）

证据：`windows-strings-0x-s3.txt` / `linux-strings-0x-s3.txt`（扫描器
`scan-binary-symbols.py` 随证据留存）。

| 标记 | Windows orz.exe | Linux orz |
|---|---|---|
| `[INITIAL_ROUND_INQUIRY v0.1]` | 1 | 1 |
| `[/INITIAL_ROUND_INQUIRY]` | 1 | 1 |
| `开局问询（一次性，非强制模板，不打断动作）` | 1 | 1 |
| `post_tool_batch_gap` | 7 | 7 |
| `template_sha256_initial_round` | 1 | 1 |
| `retrieval_enabled` | 5 | 8 |
| `browser_control` | 75 | 59 |
| `browser_launch_result` | 3 | 3 |
| `[车道:本地浏览器检索|推荐首选]` | 2 | 2 |
| `[车道:原生检索]` | 2 | 2 |
| `0.4.1` | 463 | 466 |

边界说明：`initial_round_inquiry`（第 34 族法官名）在两平台 `orz` 二进制内命中
**均为 0**——与 0q 的 `failure_agg_coverage` 同形（本次对照实测 0.4.0 发布
`orz.exe` 亦为 0），家族名不链接进 orz 主二进制，属执法 CLI 侧资源，**非缺陷**。

## 4. 仓库门禁

- `python scripts/check_repository.py` → `valid: true`、`error_count: 0`、
  `orz_source_manifest_files: 1441`；`git diff --check` 净。
- manifest 重算差异面 = 恰两条（`Cargo.lock`、`crates/orz-bin/Cargo.toml`），
  与 bump 改动面逐条对应；顺序按既定纪律为「先 orz 提交 → 再生成 manifest」。
- 构建产物仅落 gitignore 覆盖的 `target/`、证据/staging 目录；构建后 orz
  子模块工作树净。

## 5. 结论与边界

- **S3 闭合**：双平台三件套 + manifest 重建完成，构建冒烟绿（出口达成）。
- S4 实机复验待续（可与 0w 后续长任务或其他长会话同场，验证「开局一次、
  动作中不复发」）；已知自然结果：若会话恢复后周期计数已 ≥50，初始轮 fire 后
  的下一个 loop-top 会立刻触发周期问询（符合设计，非缺陷）。
- 版本 0.4.1（过程验证基线，不单独发版）；`D:\tb-eval\orz-windows` /
  `orz-linux` 已刷新为本次产物，供 0x S4、0v S3/S4 与 2.1 全量复跑共用。
- 未验证：0t 双车道 / 0v SERP 的实机行为（本次只做重建与加载冒烟）；
  Windows 侧仅 TUI 起手（无 TTY 交互深度）。
