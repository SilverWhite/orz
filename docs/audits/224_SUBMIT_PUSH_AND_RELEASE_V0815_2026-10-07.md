# 224 批：提交推送（222／223 累积件）与 0.8.15 发行（2026-10-07）

> **日期**：2026-10-07；**用户令**：「请将最新的0.8.15推送上去吧，发布这个版本包」。
> **性质**：零源码——父仓提交推送＋0.8.15 双平台打包发行＋README 发布面对齐；未闭合计数不变 **54**。
> **结论先行**：orz 子树 `10cfe765`（218 bump）远端已在案（本批 `ls-remote` 逐位复核对齐）；父仓
> `origin/main` `2e6f6615` → 本批三提交；**GitHub Release `v0.8.15` 已发布**（双平台包＋
> `SHA256SUMS`）；服务端回读与回下载复核随 **225 批补记**（沿 195→195 补记先例）。

---

## §1 推送

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `10cfe765`（218 bump：0.8.14 → 0.8.15） | `cli/feat/fusion-architecture` | 已在案（本批复核，零待推） |
| 父仓 | `2e6f6615..224`（3 提交：222／223 落账＋224 本批） | `origin/main` | exit 0 |

- **推送前状态复核**：`git ls-remote` 显示远端 `refs/heads/feat/fusion-architecture` ＝ `10cfe765`、
  `refs/heads/main` ＝ `2e6f6615`、`refs/tags/v0.8.14` ＝ `92ff0ad7`——**218 批「未推送」的记账在此
  与现实对齐**（该提交实际已由邻窗推送，本批如实记，不回改 218 档）。
- 父仓本批前工作树带 222／223 落账未提交件（邻窗留档），随本批一并提交推送。

## §2 0.8.15 发行包（`rel-224-stage` 打包）

- **打包**（[`.tmp-b224-package.ps1`](../../.tmp-b224-package.ps1)，沿 195／183／156／122 形态）：
  两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.8.15`。
- **README（发行前改写）**：包内 `README.md` 由 0.8.14 版改写为 **0.8.15 版**（62,435 B／
  `be3786bd…`）——新增 0.8.15 节：**0ct** `.gsa` 读向全开放＋两段门转确认性＋P1 两族旁路收口＋
  会话卷拦截信封备份/暂存指引；**0cu** 黑板模型写入面第三分区 `findings`；**0cv** DeepSeek 接口名
  对齐（legacy `deepseek-v4-flash` → 现行官方 `deepseek-flash`）；并更新 Linux 解包文件名与版本信息行
  （源码 `10cfe765`／构建 2026-10-07）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.15-windows-x86_64.zip` | 28,034,248 | `76a9f86043d45db31dc231ba1eac36dd8162f101733c810b97e0b27016c1013c` |
| `orz-0.8.15-linux-x86_64.tar.gz` | 37,218,091 | `fafb261d54fbbaf4871959bef2b9f38b69f8bd60d47cbefbf39bc4bc6f8d2b5c` |
| `SHA256SUMS`（顶层） | 193 | `012f04982be97f1f38d58bc8fdec9a460c43fec0624a41de12eebaf9f76a5c49` |

- **在役载体真值（与 218 批账面逐位一致）**：WIN `orz.exe` 57,135,104 `c9f30c75…`／
  `orz-signer.exe` 6,740,480 `f86158ed…`／`orz-acaf-provision.exe` 6,640,128 `c887b1a9…`；
  LIN `orz` 115,615,912 **`75515440…`（身份门值）**／`orz-signer` 1,397,544 `f1a1d784…`／
  `orz-acaf-provision` 1,215,992 `5893eec4…`。
- **版本核证**：Windows `orz.exe --build-info` = `version=0.8.15 os=windows arch=x86_64 profile=release`
  （rc 0）；Linux（`alpine:3.20` 容器内）同形 `os=linux`（rc 0）。
- **中间载体口径**：0.8.13／0.8.14 内容一并含于成品线；0.8.13 未单独发行（沿 0.8.11、0.8.3–0.8.9
  先例）；0.8.14 为上一发行版（195 批）。

## §3 发行（GitHub Release v0.8.15）

- 命令＝`gh release create v0.8.15 --target main --title "orz 0.8.15（.gsa 读向全开放 · 拦截信封备份指引 ·
  黑板 findings 分区 · DeepSeek 接口名对齐 · 双平台包）" --notes-file .tmp-b224-release-notes.md`＋三资产；
  tag `v0.8.15` = 本档所在提交（父仓轻量 tag，沿 122／156／183／195 形态）；发布后为 **Latest**。
- **服务端回读与完整回下载复核随 225 批补记**（195 → 补记 `e96cb139` 同形）。

## §4 台账与门禁

- README 发布面对齐 0.8.15（Linux 解包文件名 `orz-0.8.15-linux-x86_64.tar.gz`＋最新 release 链接）。
- 索引头行 v4.196 → **v4.197**；BACKLOG 计数行（54 不变）＋本批指针行（**并修正 219–223 批未滚动的
  失效指针**，原停 218）；BACKLOG 第二卷 §1.170；TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 边界与如实记

1. **零源码**：orz 子仓本批无新提交（`10cfe765` 为 218 bump，远端在案）；本批只做提交推送＋打包发行。
2. **218 批「未推送未发行」记账修正**：218 档写「未推送」，实际远端已有该提交（邻窗推送）——
   本批按事实记，**不回改 218 档**。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170／187／194／218 同口径）；
   Windows 在役目录已重 provision（218 批）。
4. **第二卷 `1.166` 重号**：218 与 220 两节同号（邻窗先写），本批不回改、如实登记。
5. **打包顺延已兑现**：218 批「未推送未发行」由本批补上。

## §6 关联与关键词

[`218 批档`](218_CARRIER_REBUILD_V0815_0CT_0CU_0CV_2026-10-07.md)／
[`223 批档`](223_SCB_INQUIRY_LETTER_SENT_ISSUE40_2026-10-07.md)／
[`195 批档`](195_SUBMIT_PUSH_AND_RELEASE_V0814_2026-10-04.md)（同形态先例＋补记先例）／
[`183 批档`](183_SUBMIT_PUSH_AND_RELEASE_V0812_2026-10-04.md)／
[`156 批档`](156_SUBMIT_PUSH_AND_RELEASE_V0810_2026-10-02.md)／
[`122 批档`](122_SUBMIT_PUSH_AND_RELEASE_2026-09-30.md)。

关键词：224 批、提交推送、GitHub Release v0.8.15、rel-224-stage、6/6 MATCH、README 发布面对齐、
deepseek-flash 首个发行包、findings 第三分区、.gsa 读向全开放、计数 54 不变、索引 v4.197。
