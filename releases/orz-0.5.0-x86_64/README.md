# orz 0.5.0 发布包

双平台三件套重建 + GitHub Release 发布记录（0z S3；2026-09-13）。正式发布页：
[`github.com/SilverWhite/CLI/releases/tag/v0.5.0`](https://github.com/SilverWhite/CLI/releases/tag/v0.5.0)
（用户指示跳过 0.4.4 过程版本，直接 0.4.3 → 0.5.0；0.4.1/0.4.2/0.4.3 为过程
验证版本，无独立包）。

## 源冻结基线

- orz `187cb9d7`（版本 bump 0.4.3 → 0.5.0，两文件两行）→ 构建实测暴露第三处
  Linux 构建断裂（orz-host lib.rs 无条件读取 Windows-only
  `SpawnObservation.job_handle_dup`；P1-4 批只修 xai-tty-utils 两处、本处漏网）
  → 修复落码 orz `1f13e5ec`（非 Windows 登记 `job_handle=0` 既有语义 + 关句柄
  臂 `cfg(windows)` 门控）。
- **发布资产与 `1f13e5ec` 一致**；Windows 首轮产物（`187cb9d7` 时点）作废重建。
- 0.4.3 载体六件哈希重建前已录
  `D:\tb-eval\orz-0.4.3-carrier-hashes-before-rebuild.txt`（本地 gitignore 面）。

## 资产（GitHub Release v0.5.0）

| 资产 | 大小 (B) | SHA256 |
|---|---|---|
| orz-0.5.0-linux-x86_64.tar.gz | 26,470,950 | `eda2d8b4498421808093b2ab8ee9a3d19262611a80bc0cebb333e3b389cf0e31` |
| orz-0.5.0-windows-x86_64.zip | 26,434,770 | `2bb64b5b7c9a136ec60cffecbb5633b254aa9013a366127b4850135500d86beb` |

双包内容：orz 主程序 + orz-signer + orz-acaf-provision + README。

## 二进制哈希（载体 `D:\tb-eval\orz-windows` / `orz-linux`）

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,742,144 | `a45b60e54b080b655a210f84647c91d56687872d8f7ee48037aedc67625b5c74` |
| orz-signer.exe | 6,742,528 | `d19a1394bf16149ed008904a6b6d865cd5bb899c243f9e0b03818a735f076169` |
| orz-acaf-provision.exe | 6,642,176 | `34a4a91a9a31de27daa3f1920216bb43e73ab6d47f33de019b2860cd2893ee03` |
| orz（musl static-pie） | 109,982,680 | `393eee34dd0357cab088f289fa1068a39294a13fff48d80d33c50a40684dd623` |
| orz-signer | 1,397,360 | `3d5c8155ef2b93e528607ee639f1029e09e14238631bbabe3db1ad939c8cf330` |
| orz-acaf-provision | 1,216,288 | `73fcaeda51eeb3f8883fb182d4129a66fda0333693a4143cefee244b223189d5` |

## 验证摘要

- ELF 静态核验：三件 ET_DYN + x86-64 + **PT_INTERP=0**（static-pie OK）。
- 双向加载冒烟绿：bookworm-slim（glibc）+ alpine 3.20（musl）预期 exit 1 形态
  三件一致。
- 接线符号双平台全命中：0z 七族事件（`host_resource_denied` 15 / 15、
  `resource_exhausted` 16 / 16、`reclaim_performed` 16 / 16、
  `resource_limit_hit` 15 / 15、`process_tree_reaped`、`run_terminated`、
  `host_resource_snapshot`）+ 门/进程树/降级标记（`resource_insufficient`、
  `process_trees`、`DegradedDropped`）+ 前批 0v/0x 标记保持 + `0.5.0`。
- manifest 1446 条 + `check_repository` `valid: true` / `error_count: 0`。
- 重建细节与边界：[`0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13`](../../docs/audits/0Z_S3_DUAL_PLATFORM_REBUILD_2026-09-13.md)。

## 边界

本批只做重建、加载冒烟与字节串核证；资源门/回收/树杀的实机行为仍须 0z S4
复跑取证（判据 1–13，待放行）。
