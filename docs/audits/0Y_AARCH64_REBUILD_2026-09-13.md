# 0y NP1 aarch64 载体重建记录（0.5.0 同源，2026-09-13）

> 入口：BACKLOG **0y** / TODO P0-0y——0y 的 M1 前置「载体缺口」= aarch64 musl
> 三件套版本落后（现机三件为 2026-09-01 的 **0.2.0** 早期源状态产物）。本批按
> 2026-09-13 用户裁决**与 x86_64 同源重建 0.5.0**（不再沿用文档旧写的
> 「0.4.2 重建」口径，避免造出第二条版本分叉）。
> 源冻结基线：orz **`1f13e5ec`**（与 0z S3 的 x86_64 双平台重建同源；父仓库该批
> 提交为 `659435e8`）。
> 证据目录：`D:\CLI\_windows_high_nist\evidence-0y-arm-20260913\`（本地，gitignore
> 覆盖，不入库）；载体目录：`D:\tb-eval\orz-linux-arm\`（旧 0.2.0 三件就地备份）。

## 0. 路线（沿用 2026-09-01「Linux arm 干跑」定案，非 QEMU 全量编译）

- **x86_64 `rust:1.97-slim` 容器原生 rustc + zig cc 交叉编译 C 依赖 + rust-lld
  链接**：
  - zig **0.14.1**（`D:\tb-eval\zig-linux-014`，挂 `/zig`）承担 cc/cxx/ar/ranlib；
    wrapper 把 rust triple `--target=aarch64-unknown-linux-musl` 翻译为 zig 原生
    `-target aarch64-linux-musl`（zig 不接受 rust triple 格式）。
  - 链接**不用** zig cc：zig 会注入自己的 `crt1.o`，与 rust self-contained crt
    冲突（`duplicate symbol _start`，2026-09-01 实测）→ 用 rustup 自带 rust-lld
    （wrapper 剥离 `-Wl,` 前缀、过滤 `-nostartfiles`/`-nodefaultlibs`、显式
    `-flavor gnu`）。
  - `CARGO_TARGET_DIR=/tmp/orz-target`（容器内）——跨盘 bind mount 会让指纹失效
    导致全量重编（2026-09-01 实测）。
- **打包进 orz-tools 的静态 ripgrep**：不本地 `cargo install`（aarch64-musl 下
  `tikv-jemalloc-sys` C 编译失败，`atomic_memory_order_*` undeclared，2026-09-01
  实测）；由 orz-tools `build.rs` 自动下载官方 ripgrep 15.0.0
  `aarch64-unknown-linux-gnu` 资产（该资产本身为 musl 静态）。脚本保留
  `ORZ_RG_AARCH64_STATIC` 回退口（下载失败时可挂载资产改走
  `GROK_TOOLS_BUNDLE_RG_PATH`；build.rs 对动态链接件有 fail-fast 守卫）。
- **复现入口（新件）**：`scripts/build_orz_aarch64_musl_cross.sh`（已按
  [`scripts/LIFECYCLE.md`](../../scripts/LIFECYCLE.md) 登记为 `active`）。
  调用：
  ```
  docker run --rm \
    -v D:/CLI:/orz \
    -v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml \
    -v D:/tb-eval/zig-linux-014:/zig \
    -v D:/tb-eval/orz-linux-arm:/out \
    -w /orz/orz -e ORZ_BUILD_JOBS=4 \
    rust:1.97-slim bash /orz/scripts/build_orz_aarch64_musl_cross.sh
  ```

## 1. 构建

- 编译行 `Compiling orz-bin v0.5.0`，`Finished release profile [optimized]
  target(s) in 12m 11s`（`ORZ_BUILD_JOBS=4`），`BUILD_EXIT=0`。
- 日志：`D:\tb-eval\orz-linux-arm\build-20260913-arm-050.log`。
- 警告面（非本批引入，与 x86_64 侧同源）：`xai-tty-utils::process_alive` 未使用、
  `orz-config` 未用变量、`orz-assurance` job_object 四警告、`orz-host` 三警告。

## 2. 三件套与哈希（载体 `D:\tb-eval\orz-linux-arm\`）

| 文件 | 大小 (B) | SHA256 | 对 0.2.0 差异 |
|---|---|---|---|
| orz | 73,007,720 | `b78a1c5c0312d299d8180a0963d7c8ed435ccb5093e59fd90274b166d83d49d6` | 70,215,712 → **+2,792,008 B** |
| orz-signer | 1,757,984 | `eafff1cfcbdcc431761fd21aa7165130d0d1011f832d815013b47c8412580b4d` | 1,746,368 → +11,616 B |
| orz-acaf-provision | 1,595,128 | `4797a6d7c682d84eaa7ef020f618d3bf7901203e4e54b30b9f75c9847719d641` | 1,583,872 → +11,256 B |

- 旧件（2026-09-01，0.2.0）就地备份为 `*-0.2.0.bak`（同 x86_64 侧 `.bak` 口径）。
- 证据：`arm-sha256.txt`。

## 3. ELF 形态（`arm-elf-static-check.txt`，`elf-phdr.py`）

| 文件 | e_type | e_machine | PT_INTERP | PT_LOAD |
|---|---|---|---|---|
| orz / orz-signer / orz-acaf-provision | `0x2`（ET_EXEC） | `0xb7`（AArch64） | **0** | 4 |

- 三件均为 **AArch64 静态可执行（无解释器）**，与 2026-09-01 的 0.2.0 arm 产物同形态。
- **平台差异登记**：x86_64 侧为 `ET_DYN` + `PT_INTERP=0`（musl static-pie），
  aarch64 交叉链固定为 `ET_EXEC`；两者都满足「无解释器、可在安卓内核直接执行」
  这一 NP1 前提（`BODY-PRE-01`），差异属链接形态而非能力差异。

## 4. 双向加载冒烟（arm64 容器，Docker Desktop binfmt）

- **bookworm-slim（glibc 环境）**：`arm-smoke-bookworm.txt`——三件形态与 x86_64
  载体一致：`orz` 无 TTY `tui io error` exit 1 / `orz-signer` manifest 缺失 fatal
  exit 1 / `orz-acaf-provision` usage exit 1。
- **alpine 3.20（musl 原生环境）**：`arm-smoke-alpine.txt`——三件形态逐条相同。
- 边界：alpine 侧退出码经管道 `head` 覆盖（显示 0），形态核证以输出文本为准；
  bookworm 侧用 `PIPESTATUS` 捕获得到 exit=1。

## 5. 接线符号核证（`arm-binary-symbols.txt`）

- **0z 七族 + 门/回收/进程树/降级全命中**：`resource_insufficient` 1、
  `host_resource_snapshot` 3、`host_resource_denied` 15、`resource_exhausted` 15、
  `process_tree_reaped` 2、`reclaim_performed` 15、`resource_limit_hit` 14、
  `run_terminated` 2、`process_trees` 8、`DegradedDropped` 4。
- **前批标记保持**：车道标注 2/2、`browser_control` 59、`wait_load` 7、
  `setmkt=en-US` 1、`low_quality` 23、`engine_attempts` 14、
  `[INITIAL_ROUND_INQUIRY v0.1]` 1、`post_tool_batch_gap` 7、`serp-attempts` 15、
  `ordered_engines` 1。
- `0.5.0` 串 2 处（x86_64 侧 16 处）——**非语义差异**：交叉编译产物的字符串合并/
  去重布局不同，版本由 `orz-version` 与 cargo 版本字段承载，功能核证以符号与冒烟为准。

## 6. 边界与不做项

- **零源码改动**：本批只产出载体制品（脚本为新增复现入口，不改 orz 源码）。
- **不入 manifest**：`orz_source_manifest.sha256` 是 orz 源码面台账，载体三件不属其中；
  本审计即该批载体的登记面（哈希见 §2）。
- **未参与发布**：aarch64 三件仅落本地载体供 NP1 支线使用，不进 GitHub Release。
- **待续**：S1 模拟器实机化（Magisk-in-AVD 模块打包/安装/禁用/恢复）+ 本批三件在
  AVD 内的上机冒烟（设计 §14.2 验证载体）；M0 定版（用户侧）。

## 7. 结论

0y「载体缺口（M1 前置）」**闭合**：aarch64 三件套已与 x86_64 0.5.0 同源重建，
具备 `AArch64 + 无解释器 + 双向环境可加载` 三项实证，哈希与证据入档。
