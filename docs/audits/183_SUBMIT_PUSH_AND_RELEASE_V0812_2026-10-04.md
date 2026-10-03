# 183 批：提交推送（174–182 累积件）与 0.8.12 发行（2026-10-04）

> **日期**：2026-10-04；**用户令**：「请先进行推送吧，最新的双平台安装包也推送上去」。
> **性质**：零源码——父仓/子仓提交推送＋0.8.12 双平台打包发行＋README 发布面对齐；
> 未闭合计数不变 **58**。
> **结论先行**：orz 子树 `905bc3f5..54717d06`（7 提交）与父仓 `b5da5375..85e2b463`
> （11 提交，含 174–182 批）已推送；**GitHub Release `v0.8.12` 已发布**（双平台包＋
> `SHA256SUMS`，服务端 digest 与本地产物逐位一致，完整回下载复核 `identical`）。

---

## §1 推送

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `905bc3f5..54717d06`（7 提交：176/177 P8-a/P8-b·178 审查处置·179 RS-06·180 bump） | `cli/feat/fusion-architecture` | exit 0 |
| 父仓 | `b5da5375..85e2b463`（11 提交，含 174–182 批） | `origin/main` | exit 0 |

- 推送前状态：orz 子模块 `feat/fusion-architecture` 领先 `cli/...` 7 提交；父仓 `main`
  领先 `origin/main` 11 提交（上次推送停在 173 批 `b5da5375`）。两仓工作树均 clean。

## §2 0.8.12 发行包（`rel-183-stage` 打包）

- **打包**（[`.tmp-b183-package.ps1`](../../.tmp-b183-package.ps1)，沿 156／147／122 形态）：
  两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.8.12`。
- **README（发行前改写）**：包内 `README.md` 由 0.8.10 版改写为 **0.8.12 版**——新增
  0.8.12（0am P8 刺激面类型化总线与成因段）与 0.8.11（0cp 动作采样附注直投／0cn
  budget 注入撤除）两节；旧版节照旧保留并标注归属；版本信息行 `54717d06`／2026-10-04
  （0.8.12 双平台重建）；RLI 边界行更新（八通道与提醒附注在役、预测面未落地）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.12-windows-x86_64.zip` | 28,018,976 | `5f2df0f1915da36bd1a785e2e6e8f520d7d548f40bb08e9ee4f9e1757f316ded` |
| `orz-0.8.12-linux-x86_64.tar.gz` | 37,187,937 | `1870cc7ee7751ab84129b1184626b2fd2aa15c9a6c47737d50056c74cf992968` |
| `SHA256SUMS`（顶层） | 193 | `ab6edfb2e187e52be14add57089af03c1ded198bc6db2974fdfe7cf012ec045f` |

- **在役载体真值（进件终态）**：WIN `orz.exe` 57,101,312 `40bdda50…`／`orz-signer.exe`
  `860c9860…`／`orz-acaf-provision.exe` `06b7e693…`；LIN `orz` 115,569,504 `b141c1ef…`
  （身份门值）／`orz-signer` `a21e1dc5…`／`orz-acaf-provision` `8172b2fc…`。
- **版本核证**：Windows `orz.exe --build-info` = `version=0.8.12 os=windows arch=x86_64 profile=release`（rc 0）。

## §3 发行（GitHub Release v0.8.12）

- 命令＝`gh release create v0.8.12 --target main --title … --notes-file .tmp-b183-release-notes.md`
  ＋三资产；tag `v0.8.12` = `5eb93b60`（父仓本档所在提交，轻量 tag，沿 122／156 形态）；
  发布后为 **Latest**。
- **服务端回读**（`gh release view v0.8.12 --json assets`）：三资产 digest 与本地产物**逐位一致**——
  zip `sha256:5f2df0f1…`／tar.gz `sha256:1870cc7e…`／`SHA256SUMS` `sha256:ab6edfb2…`。
- **完整回下载复核**（`gh release download v0.8.12`）：三件落地后与本地产物对拍——
  **`identical=True` 3/3**（字节数逐一相等）⇒ 远端资产＝本地产物，无上传截断／替换。

## §4 台账与门禁

- README 发布面对齐 v0.8.12（解包文件名 `orz-0.8.12-linux-x86_64.tar.gz`＋最新 release 链接）。
- 索引头行 v4.154 → **v4.155**；BACKLOG 本批记录指针＋计数行（58 不变）；TODO 计数行＋`P1-0am` 行；
  BACKLOG 第二卷 §1.137。
- 门禁 `check_repository.py` ⇒ `valid: true`。

## §5 边界

1. **零源码**：orz 子仓本批无新提交；仅推送既有提交＋打包发行。
2. **中间载体不发**：0.8.11 未单独发行，内容一并含于 0.8.12（沿 0.6.x／0.8.3–0.8.6／0.8.8–0.8.9 口径）。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170 同口径）；Windows 在役目录已重 provision。
4. **打包顺延已兑现**：180 批「打包顺延、未推送未发行」由本批补上。
5. 下一步＝0am 预测段设计卷与实现批（待放行）；0cq S1 勘定；之后按 2026-09-20 顺序裁决进 0bc。

## §6 关联与关键词

[`182 批档`](182_0CQ_REGISTRATION_S05_ARRIVAL_NEGATIVE_FORECAST_FORM_NARROWED_2026-10-04.md)／
[`181 批档`](181_RECLI_RUN3_0AM_VERIFY_S0_FORECAST_BACKTEST_2026-10-04.md)／
[`180 批档`](180_CARRIER_REBUILD_V0812_0AM_P8_ENTRY_2026-10-04.md)／
[`156 批档`](156_SUBMIT_PUSH_AND_RELEASE_V0810_2026-10-02.md)（同形态先例）／
[`122 批档`](122_SUBMIT_PUSH_AND_RELEASE_2026-09-30.md)。

关键词：183 批、提交推送、orz 子树 `54717d06`、父仓 `85e2b463`、GitHub Release v0.8.12、
rel-183-stage、回下载逐位一致、README 发布面对齐、中间载体 0.8.11 不发、计数 58 不变。
