# 195 批：提交推送（184–194 累积件）与 0.8.14 发行（2026-10-04）

> **日期**：2026-10-04；**用户令**：「请进行推送吧，并将最新版本的安装包也发布上去」。
> **性质**：零源码——父仓/子仓提交推送＋0.8.14 双平台打包发行＋README 发布面对齐；
> 未闭合计数不变 **60**。
> **结论先行**：orz 子树 `54717d06..b5cb57ca`（8 提交）与父仓 `75a7d68b..bbf00580`
> （11 提交，含 184–194 批）已推送；**GitHub Release `v0.8.14` 已发布**（双平台包＋
> `SHA256SUMS`）；服务端回读与回下载复核见 §3。

---

## §1 推送

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `54717d06..b5cb57ca`（8 提交：184 落码·185/186 0cq·187 bump·191 0cs S1·192 0bz S3′·193 审查处置·194 bump） | `cli/feat/fusion-architecture` | exit 0 |
| 父仓 | `75a7d68b..bbf00580`（11 提交，含 184–194 批） | `origin/main` | exit 0 |

- 推送前状态：orz 子模块 `feat/fusion-architecture` 领先 `cli/...` 8 提交；父仓 `main` 领先
  `origin/main` 11 提交（上次推送停在 183 补记 `75a7d68b`）。两仓工作树均 clean。

## §2 0.8.14 发行包（`rel-195-stage` 打包）

- **打包**（[`.tmp-b195-package.ps1`](../../.tmp-b195-package.ps1)，沿 183／156／147／122 形态）：
  两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.8.14`。
- **README（发行前改写）**：包内 `README.md` 由 0.8.12 版改写为 **0.8.14 版**——新增
  0.8.14（工具名近似提示 0cs／D4 机械段移尾 0bz S3′）与 0.8.13（写控误拦修复 0cq／RLI 前推
  预告段 0am）两节；旧版节照旧保留并标注归属；版本信息行 `b5cb57ca`／2026-10-04（0.8.14 双
  平台重建）；RLI 边界行更新（前推预告段已落地）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.14-windows-x86_64.zip` | 28,018,899 | `9ea5861126cd7bcf4ae9b0f146536c139eaad7a3f3eeaf52f80c09b0fab7e3f6` |
| `orz-0.8.14-linux-x86_64.tar.gz` | 37,202,636 | `8c10ed329675ae8c62429ae9a8acb06b119e14ab47ddaa60083a6c02ea4e6016` |
| `SHA256SUMS`（顶层） | 193 | `8cd1a4ca97b2b9a7d00652c7f376d664c2e321116a8456647c4ebc29b3db962d` |

- **在役载体真值（进件终态）**：WIN `orz.exe` 57,116,672 `8dbbd61f…`／`orz-signer.exe`
  6,740,480 `aa1ab041…`／`orz-acaf-provision.exe` 6,640,128 `fdb7a788…`；LIN `orz` 115,593,208
  `fc990a8a…`（身份门值）／`orz-signer` 1,397,528 `ff2f3ac7…`／`orz-acaf-provision` 1,215,992
  `620f91cc…`。
- **版本核证**：Windows `orz.exe --build-info` = `version=0.8.14 os=windows arch=x86_64 profile=release`（rc 0）。

## §3 发行（GitHub Release v0.8.14）

- 命令＝`gh release create v0.8.14 --target main --title "orz 0.8.14（工具名近似提示与模型面前缀
  D4 移尾 · 双平台包）" --notes-file .tmp-b195-release-notes.md`＋三资产；tag `v0.8.14` = `92ff0ad7`
  （父仓本档所在提交，轻量 tag，沿 122／156／183 形态）；发布后为 **Latest**。
- **服务端回读**（`gh release view v0.8.14 --json assets`）：三资产 digest 与本地产物**逐位一致**——
  zip `sha256:9ea58611…`／tar.gz `sha256:8c10ed32…`／`SHA256SUMS` `sha256:8cd1a4ca…`。
- **完整回下载复核**（`gh release download v0.8.14`）：三件落地后与本地产物对拍——
  **`identical=True` 3/3**（字节数逐一相等：zip 28,018,899／tar.gz 37,202,636／`SHA256SUMS` 193）
  ⇒ 远端资产＝本地产物，无上传截断／替换。

## §4 台账与门禁

- README 发布面对齐 v0.8.14（解包文件名 `orz-0.8.14-linux-x86_64.tar.gz`＋最新 release 链接）。
- 索引头行 v4.166 → **v4.167**；BACKLOG 本批记录指针＋计数行（60 不变）；TODO 计数行；
  BACKLOG 第二卷本批条目。
- 门禁 `check_repository.py` ⇒ `valid: true`。

## §5 边界

1. **零源码**：orz 子仓本批无新提交；仅推送既有提交＋打包发行。
2. **中间载体不发**：0.8.13 未单独发行，内容一并含于 0.8.14（沿 0.8.11、0.6.x、0.8.3–0.8.6、
   0.8.8／0.8.9 口径）。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170／187／194
   同口径）；Windows 在役目录已重 provision。
4. **打包顺延已兑现**：187／194 批「打包顺延、未推送未发行」由本批补上。

## §6 关联与关键词

[`194 批档`](194_CARRIER_REBUILD_V0814_0CS_S2_0BZ_S3PRIME_2026-10-04.md)／
[`193 批档`](193_REVIEW_DISPOSAL_0BZ_0CS_2026-10-04.md)／
[`183 批档`](183_SUBMIT_PUSH_AND_RELEASE_V0812_2026-10-04.md)（同形态先例）／
[`156 批档`](156_SUBMIT_PUSH_AND_RELEASE_V0810_2026-10-02.md)／
[`122 批档`](122_SUBMIT_PUSH_AND_RELEASE_2026-09-30.md)。

关键词：195 批、提交推送、orz 子树 `b5cb57ca`、父仓 `bbf00580`、GitHub Release v0.8.14、
rel-195-stage、回下载逐位一致、README 发布面对齐、中间载体 0.8.13 不发、计数 60 不变。
