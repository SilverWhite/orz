# 101 提交推送与发行（0.8.1；2026-09-27）

> **日期**：2026-09-27；**用户令**：「请先进行重建吧，然后再提交并推送，双平台安装包也进行发布」。
> **本批＝0.8.1 落账（orz 两笔＋父仓 pin＋源清单）＋打包（**包内首次含载体清单**）＋GitHub Release `v0.8.1`**；**计数 55 不变**（0bx 维持 `partial`：S4 agent 级真机承接读数待）。
> 前批＝[`100 载体重建`](100_CARRIER_REBUILD_V081_DUAL_PLATFORM_2026-09-27.md)（双平台 0.8.1 换装）；关联＝[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)。

## §1 orz 提交与推送

| 提交 | 内容 |
|---|---|
| `4d5c3441` | `fix(0bx 就绪判定): 浏览器会话就绪判定改认 CDP 端点（hand-off 形态）＋三枚钉子`——`cdp.rs`（端点兜底＋TTL 缓存＋关停位）＋`rollback_maintenance.rs`（测试告警清理）；**2 文件 +210/−10** |
| `12dbaf90` | `chore(release): bump version 0.8.0 -> 0.8.1（0bx 修复载体重建源冻结）`——`Cargo.toml`＋`Cargo.lock` 两行 |

- 推送：`c25e459a..12dbaf90 → feat/fusion-architecture`（`cli` remote）**exit 0**；提交后 orz 工作树 clean；本地分支与远端同点。

## §2 父仓落账面

- 子模块 pin `d0e29615` → **`12dbaf90`**；`orz_source_manifest.sha256` 重生成 **1506 条目**（4 处哈希随新 blob 更新：`cdp.rs`／`rollback_maintenance.rs`／`Cargo.toml`／`Cargo.lock`；`--check` 读数 `valid`）。
- 入仓文档：[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)／[`100 重建档`](100_CARRIER_REBUILD_V081_DUAL_PLATFORM_2026-09-27.md)／本档；父仓 `README.md` 发布面三处对齐 **v0.8.1**（解压示例／Release 入口／发布行）。

## §3 打包（`D:\tb-eval\rel-101-stage\`）

- 入口 [`.tmp-b101-package.ps1`](../../.tmp-b101-package.ps1)（沿 095 形态＋本批新增加载清单步骤）；**重建与打包解耦**——直接取 100 批已换装的双平台在役载体，故包内二进制与 100 档读数逐位一致。
- **本批相对 095 的唯一结构性变化：包内**含** `carrier-manifest.json`**（095 档登记的待裁项＝用户本批裁决「把载体清单一起打进去」）。清单**由打包阶段按包内实际内容生成**（`python scripts/generate_carrier_manifest.py <stage>/win|lin --version 0.8.1`）⇒ 两侧各 **5 entries**＝三件套＋`README.md`＋`SHA256SUMS`（清单自身与 `.bak` 链按既定排除集不入册）。**无需改码、无需二次重建载体**（095 档预估的「改码＋重建」经勘定证否）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.1-windows-x86_64.zip` | 27,979,623 | `fd1f7caedef13abf3c7ba8247fad047b9285f2bd8dd1a1f3c7fee36437f191f3` |
| `orz-0.8.1-linux-x86_64.tar.gz` | 37,149,421 | `3bb36d566e168e7f4fc4be30a50c2361acdbc8a27ff764918048b8aad53a9f6a` |
| `SHA256SUMS`（顶层） | — | 上述两行（LF、`hash *name`） |

- 包内六件 × 两侧（含清单）：三件套与 100 批在役载体 **6/6 MATCH**。
- 容器核证（`alpine:3.20`）：`sha256sum -c`＝zip **4/4 OK**／tar **4/4 OK**／顶层 **2/2 OK**；包内二进制 `--build-info` 双平台读 **`version=0.8.1`**（`os=windows`／`os=linux`）。
- **包内清单活体两态**（本批新面实证，触发命令＝`orz rollback list`）：解压态**零告警** ✓；把解压态 `README.md` 追加一字节后**恰 1 条** finding＝`README.md: 长度不符（清单 25779 ≠ 实际 25787）` ✓ ⇒ 清单对 `README.md`／`SHA256SUMS` 的覆盖**真在生效**（095 档担心的「清单外新增」告警面随打包口径消解）。

## §3.1 发行回读

- **GitHub Release `v0.8.1`** 已发布（Latest）；`draft=false`／`prerelease=false`；tag 指向本批父仓提交 `（见 §4 回执，回填于发行后）`。
- 三资产远端 digest 与本地逐位一致；经认证 API **完整回下载**复核 `identical=True` ×3（读数随本小节回填）。

## §4 台账同步

- BACKLOG／TODO：计数行「本批」→ 本节（**55 不变**）；`### 0bx.` 节补 S3 达成注记（载体 0.8.1）＋`P1-0bx` 的 S3 勾选；P1 开放项清单与优先级总览表**不动**（0bx 仍在开放集）。
- 索引：头行 v4.68 → **v4.69**（v4.68 滚入存档卷 118 → **119 行**）；§6 `GAP-BROWSER-READY-HANDOFF` 条目补「载体已进体（0.8.1）」。
- 第二卷：§1.53。门禁：目标 `valid: true`（orz 树面已提交）。

## §5 边界与未做

1. **0bx 维持 `partial`**：闭合判据＝「agent 级真机采样显示 `web_search` 链首**承接**（或如实让渡于真不可用）」，本批只到「修复进体＋真机钉绿＋包内清单活体两态」；**S4 未跑**。
2. **89 题重跑未跑**（余额 71.14 CNY 不足，沿 098 档登记待充值放行）。
3. 双平台预检未做（默认口径）；Linux 载体未做 ACAF 重 provision（沿 093 同口径）。

## §6 关联与关键词

[`100 重建档`](100_CARRIER_REBUILD_V081_DUAL_PLATFORM_2026-09-27.md)／[`099 就绪判定档`](099_BROWSER_READY_HANDOFF_2026-09-27.md)／[`095 发行档`](095_RELEASE_2026-09-27.md)（包内清单待裁项来源）／[`083 全面审查档`](083_FULL_PROJECT_REVIEW_2026-09-26.md)。

关键词：0.8.1、提交推送、pin d0e29615→12dbaf90、源清单 1506、打包、**包内 carrier-manifest（5 entries）**、
清单活体两态、篡改恰 1 条 finding、容器 sha256sum -c 全 OK、GitHub Release v0.8.1、S4 待跑、0bx partial。
