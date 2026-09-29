# 122 批：提交、推送与 0.8.7 发行（2026-09-30）

> **结论先行**：122 批（0cc S2 审查处理批 v3.1 ＋ 0.8.7 双平台重建进体 ＋ S4 首题翻盘）
> **全部入仓并推送**——orz 子树 `93625868..79a3e8e5`（`feat/fusion-architecture`），父仓
> `8275c598..1f0ad52c`（`main`）；**GitHub Release `v0.8.7` 已发布**（Latest，双平台包＋
> `SHA256SUMS`，三资产服务端 digest 与本地产物逐位一致，完整回下载复核 `identical=True 3/3`）。
> **计数 58 不变**，`0cc` 维持 `pending`（余 S4＝4 题重跑＋未跑面 44 题逐题重跑）。

> **日期**：2026-09-30；**用户令**：「请进行提交并推送吧，0.8.7的包体也推送上去」。
> **形态**：orz 子树零新提交（本批源码提交已就位，仅推送）＋父仓单笔提交＋推送＋打包重建＋发行。

---

## §1 orz 子树推送

| 项 | 值 |
|---|---|
| 本批提交 | `cb88d416`（`feat(0cc S2 审查处理批 v3.1)`：规则 5 宿主态祖先链臂＋动词补齐）／`79a3e8e5`（`chore(release): bump version 0.8.6 -> 0.8.7`） |
| 推送 | `93625868..79a3e8e5 → feat/fusion-architecture`（`cli` remote）**exit 0** |
| 说明 | 远端此前停在 `93625868`（0.8.2 源冻结点）；本推一次带上 0.8.3–0.8.7 的 orz 侧提交 |
| 代码读数 | orz-tools lib **2988/0/6**；clippy 13 基线持平；触碰面 fmt 零 diff |

## §2 父仓提交与推送

| 项 | 值 |
|---|---|
| 提交 | **`1f0ad52c`** |
| 规模 | **9 文件 `+156/−30`** |
| 文件 | [`CLI_PROJECT_INDEX.md`](../../CLI_PROJECT_INDEX.md)／[`README.md`](../../README.md)／[`TODO.md`](../../TODO.md)／[`BACKLOG`](../BACKLOG_AND_PRIORITIES.md)／[`TB21 重跑范围判定档`](../TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)／[`写控保底化设计档 v3.1`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)／`orz`（pin）／`orz_source_manifest.sha256`／`scripts/run_r0_heavy_official.py` |
| 推送 | `8275c598..1f0ad52c → origin/main` **exit 0** |
| 同步态 | `git rev-list --left-right --count origin/main...HEAD` ＝ **`0 0`**；工作树 clean |

## §3 0.8.7 发行包（`rel-122-stage` 重建）

- **README 修正（发行前）**：包内 `README.md` 补 **0.8.6／0.8.7 两节更新说明**；`0.8.7` 改标
  「相对已发布 `0.8.2` 的**正式增量版**（含未单独发行的中间载体 0.8.3–0.8.6）」；`0.8.3–0.8.6`
  各节改标「中间载体，内容已含于 0.8.7」；`0.8.5` 节补「第五类目标集自 0.8.6 起收窄」注；
  版本信息行 `15bc1bfb`／0.8.5 重建 → **`79a3e8e5`／2026-09-30（0.8.7 双平台重建）**。
- **打包**（[`.tmp-b122-package.ps1`](../../.tmp-b122-package.ps1)，沿 117 形态）：两侧各 **5 entries**
  （三件套＋`README.md`＋`SHA256SUMS`；`carrier-manifest.json` 由打包阶段按包内实际内容生成，
  `kind=orz-carrier-manifest version=0.8.7`）；包内六件与在役载体 **6/6 MATCH**。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.7-windows-x86_64.zip` | 28,013,522 | `28c799422bf4f58375d1764dd74a0cf4c7e55a872dd220fd603a543de8c143bf` |
| `orz-0.8.7-linux-x86_64.tar.gz` | 37,185,803 | `5152b09d1df1fa2fffc24720ec10ac41678f9066e5e083c2863a440161140a58` |
| `SHA256SUMS`（顶层） | 191 | `089f5f0c7f71684858cb39006d1f2ee120ae33608823059d14053bd7c1994110` |

- **容器核证（alpine 3.20）**：zip 解包后 `sha256sum -c` **4/4 OK**、tar **4/4 OK**、
  顶层 `SHA256SUMS` **2/2 OK**；双平台 `--build-info` 均读 `version=0.8.7`。
- **清单活体两态**（触发通道＝`orz --version`；`--build-info` 在完整性自检前早退，沿 121 批勘误）：
  解压态**零告警** ✓；解压态 `README.md` 追加一字节后**恰 1 条** finding＝
  `README.md: 长度不符（清单 41594 ≠ 实际 41595）` ✓。

## §4 发行（含服务端回读）

- **GitHub Release `v0.8.7`** 已发布（Latest）：<https://github.com/SilverWhite/CLI/releases/tag/v0.8.7>；
  `draft=false`／`prerelease=false`／`published_at=2026-09-29T18:09:32Z`；tag `v0.8.7` 指向父仓
  提交 **`1f0ad52c`**（`git ls-remote --tags origin v0.8.7` 读 `1f0ad52c…`，轻量 tag）。
  **如实记**：首跑以 `--target <短 SHA>` 提交被 API 拒（`HTTP 422 target_commitish is invalid`），
  改 `--target main`（＝同值 tip）成立；发布说明＝[`.tmp-b122-release-notes.md`](../../.tmp-b122-release-notes.md)。

| 资产 | asset id | 大小 (B) | 服务端 digest |
|---|---:|---:|---|
| `orz-0.8.7-linux-x86_64.tar.gz` | `598874385` | 37,185,803 | `sha256:5152b09d1df1fa2fffc24720ec10ac41678f9066e5e083c2863a440161140a58` |
| `orz-0.8.7-windows-x86_64.zip` | `598874384` | 28,013,522 | `sha256:28c799422bf4f58375d1764dd74a0cf4c7e55a872dd220fd603a543de8c143bf` |
| `SHA256SUMS` | `598874392` | 191 | `sha256:089f5f0c7f71684858cb39006d1f2ee120ae33608823059d14053bd7c1994110` |

- **完整回下载复核**：`gh release download v0.8.7` 三件落地后与本地包对拍——
  **`identical=True 3/3`**（字节数逐一相等）⇒ 远端资产＝本地产物，无上传截断／替换。

## §5 台账与门禁

| 时点 | 读数 |
|---|---|
| 提交前 | `check_repository.py` ⇒ **`valid: true`**（`error_count: 0`）；源清单 `--check` ⇒ **`valid`** |
| 推送后 | `check_repository.py` ⇒ **`valid: true`**；源清单重生成 **1506 条** |

- 台账：索引头行 v4.90 → **v4.91**（本批＝提交推送＋发行）；BACKLOG 计数行「未提交未推送」→
  「已提交推送＋0.8.7 已发行」，本批记录指针 → 第二卷 §1.74；TODO 计数行同步批号与推送／发行状态；
  第二卷新增 [`§1.74`](../BACKLOG_AND_PRIORITIES_2.md)。
- `README.md` 发布面三处对齐 **v0.8.7**（Linux 解包文件名／最新 release 链接／发布线描述）。

## §6 边界与后续

1. **0cc 余 S4 未跑**：4 题重跑（b3-06 extract-moves-from-video／b1-16 git-multibranch／
   b3-12 torch-pipeline-parallelism／b3-03 winning-avg-corewars）＋未跑面 44 题
   （b3-13…b3-19＝7＋B4×18＋B5×19）逐题独立作业；b1-08 qemu-startup 仍挂起。
2. **提交/推送冻结令解除**：2026-09-28 起的提交／推送冻结由本令解除（本批已推送）。
3. **中间载体不发**：0.8.3–0.8.6 未单独发行，内容一并含于 0.8.7（沿 0.6.x 中间载体口径）。
4. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109 同口径）；Windows 在役目录已重 provision。

## §7 关联与关键词

[`0cc 设计档 v3.1`](../WRITE_CONTROL_BACKSTOP_REVISION_DESIGN_2026-09-29.md)／
[`TB21 重跑范围判定档 §10`](../TB21_V41_RERUN_SCOPE_DETERMINATION_2026-09-29.md)／
[`109 发行档`](109_CARRIER_REBUILD_AND_RELEASE_V082_DUAL_PLATFORM_2026-09-28.md)／
BACKLOG `0cc`／TODO `P1-0cc`。

关键词：122 批、提交推送、父仓 `1f0ad52c`、orz 子树 `79a3e8e5`、GitHub Release v0.8.7、
rel-122-stage、回下载逐位一致、祖先链臂、build-pov-ray 翻盘、计数 58 不变。
