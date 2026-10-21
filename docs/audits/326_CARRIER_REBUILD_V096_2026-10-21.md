# 326 批：0.9.6 双平台重建＋Windows 装机位换装（0dm S1 v0.2 进体，2026-10-21）

> **用户令**：「请进行提交与推送吧，推送完成后请进行重装与换装」。
> **本批**＝0.9.5→0.9.6 代窗口：源码推送 → Windows release 重建 → 载体位/装机位换装 →
> ACAF 重 provision → Linux musl 重建 → 双平台冒烟 → 身份门/清单 → 落账。
> **源**＝orz `6d98fbab`（325 审查修复批）＋版本 bump `875303bd`（`0.9.5 → 0.9.6`）。
> **已推送** `cli/feat/fusion-architecture`＋`origin/main`；**已发行 GitHub Release `v0.9.6`（Latest）**。

## §0 结论速览

| 面 | 结果 |
|---|---|
| 推送 | orz `6d98fbab` → `cli/feat/fusion-architecture`；父仓 `5b6d9ea8` → `origin/main`（重装前） |
| Windows 重建 | `cargo build -p orz-bin --release -j 2` **exit 0（4m19s）**；唯一警告＝`orz-host/permission.rs` unused imports（预存） |
| Windows 三件 | `orz.exe` **58,786,816 B `f01cf86f…`**／`orz-signer.exe` 6,740,480 `0bf55ac8…`／`orz-acaf-provision.exe` 6,640,128 `1058c308…` |
| 换装 | 装机位 `C:\Users\1\AppData\Local\orz\bin` 三件 MATCH 3/3；载体位 `D:\tb-eval\orz-windows` MATCH 3/3；两侧 `.0.9.5-bak` 已建 |
| ACAF | manifest 备份 `signer-manifest.json.bak-20261021-325`；re-provision **exit 0**；`binary_sha256=0bf55ac8…` ↔ signer 逐位一致 |
| Windows 冒烟 | `RUN-CLI-6ad86f8b` **rc=0**；journal 11 events；`run_finished` 含 `terminal_source=short_text`／`terminal_nudged=false`／`terminal_nudge_count=0`／`self_check_called=false` |
| Linux 重建 | docker `rust:1.97-slim`＋`build_orz_aliyun_trixie.sh` **exit 0（8m14s）** |
| Linux 三件 | `orz` 117,752,808 `7416d397…`／`orz-signer` 1,397,952 `f6321d1b…`／`orz-acaf-provision` 1,216,400 `015b493c…` |
| Linux 静态 | ELF `e_type=3`（PIE）、`machine=62`、`PT_INTERP=0`；alpine/bookworm `--build-info`＝`0.9.6 os=linux` rc0 |
| Linux 冒烟 | alpine `RUN-CLI-6ad8725d` rc0；bookworm `RUN-CLI-6ad87262` rc0；两侧 journal 11 events，终态字段同上 |
| 身份门 | `EXPECTED_CARRIER_SHA256` → **`7416d397…`**；`py_compile` 过、`--help` rc0 |
| 载体清单 | `D:/tb-eval/orz-windows/carrier-manifest.json`、`D:/tb-eval/orz-linux/carrier-manifest.json` 均 `version=0.9.6`、3 entries |
| 发行 | GitHub Release `v0.9.6` **Latest**；zip 28,576,979 `4a11f55f…`／tar.gz 37,871,329 `4bb38820…`／顶层 `SHA256SUMS` 191 B `93495e9a…` |
| 计数 | 63 不变；索引 v4.295→**v4.296** |

## §1 源与推送

- 推送前 orz `6d98fbab`、父仓 `5b6d9ea8`；`cli/feat/fusion-architecture` 与 `origin/main` 均成功。
- 版本 bump：`crates/orz-bin/Cargo.toml` / `Cargo.lock` 0.9.5→0.9.6（仅 orz-bin 包）。
- `cargo metadata --locked --format-version 1 --no-deps` exit 0。

## §2 Windows 重建与换装

| 文件 | 尺寸 | SHA256 |
|---|---|---|
| `orz.exe` | 58,786,816 | `f01cf86fc6166310e11686c3d3b7c518947e12979f755568e559426d0111bb89` |
| `orz-signer.exe` | 6,740,480 | `0bf55ac844f4afa1cfb9aef365f7099bbeef3b5cd411744ad72a4aed0f52acc3` |
| `orz-acaf-provision.exe` | 6,640,128 | `1058c308d5ddbbecb8ba61baa073ce6e8dd7f386a12df466d586212af2f3179e` |

- 载体位与 `target/release`、装机位三件均 MATCH 3/3；两侧 `.0.9.5-bak` 已建。
- 装机位 `--build-info`：`version=0.9.6 os=windows arch=x86_64 profile=release` rc0。
- ACAF：manifest 备份后 re-provision exit 0；`binary_sha256` 与 signer 逐位一致；keystore 未动。
- `carrier-manifest.json` 刷新为 0.9.6（3 entries）。

## §3 Linux 重建

- 命令同 323 批（`docker run --rm ... rust:1.97-slim bash /build.sh`）。
- 三件均 static-pie、`PT_INTERP=0`；alpine 3.20 与 bookworm-slim `--build-info` 均 `0.9.6 os=linux` rc0。
- `carrier-manifest.json` 刷新为 0.9.6（3 entries）。
- **备份注记**：Linux 载体旧 0.9.5 三件在构建前未单独复制 `.0.9.5-bak`（构建脚本直接覆盖）；需要回退时按 `974d0345` 重建，Windows 侧 `.0.9.5-bak` 完整。

## §4 冒烟

- Windows：`RUN-CLI-6ad86f8b` rc0，11 events；`run_finished` 新四字段落盘；二进制字节含 `反例自查`／`[RUN_END]`／`[TERMINAL_NOTICE]`／`self_check`／`submit_retired`。
- Linux：alpine `RUN-CLI-6ad8725d`、bookworm `RUN-CLI-6ad87262` 均 rc0，11 events；`run_finished` 新四字段落盘。
- `python assurance/run_event_journal_validation.py <win journal>` exit 0（无输出）。

## §5 身份门与清单

- `scripts/run_r0_heavy_official.py`：`EXPECTED_CARRIER_SHA256` `a409e873…`→**`7416d397…`**；注释记 325 批进件；`py_compile` 过＋`--help` rc0。
- 载体清单：Windows/Linux 均 `version=0.9.6`、3 entries。

## §6 边界

1. **已发行**：GitHub Release `v0.9.6`（Latest）；tag 指向发行提交；README 发布面已对齐 0.9.6。
2. **只换 Windows 装机位**；Linux 为载体位，无 Windows 式安装位。
3. **Linux `.0.9.5-bak` 缺失**（见 §3）；Windows 备份完整。
4. 未 `cargo clean`，未删除任何 bak 链或评测数据。
5. S4 三场景真机（短聊/进度文本/显式自查）仍未跑；S2 交付状态面板仍在待办。

## §8 发行

- GitHub Release：`v0.9.6`，Latest；三资产：
  - `orz-0.9.6-windows-x86_64.zip` **28,576,979 B** `4a11f55f…`；
  - `orz-0.9.6-linux-x86_64.tar.gz` **37,871,329 B** `4bb38820…`；
  - `SHA256SUMS` **191 B** `93495e9a…`。
- 包内 6 entries（三件套＋README＋SHA256SUMS＋carrier-manifest）；WIN 侧 4/4 OK、Linux 容器 4/4 OK、双平台 `--build-info` 均 0.9.6。
- README 发布面：解包名／Release 链接／0.9.6 更新节；英文 README 同步 v0.9.6 face。
- 回下载逐位核证与 `gh release view` 服务端 digest 见发行后补记（本档 §8 后续回执）。

## §7 关键词

0.9.6、双平台重建、Windows 换装、ACAF re-provision、Linux musl、static-pie、RUN-CLI-6ad86f8b、7416d397、326。
