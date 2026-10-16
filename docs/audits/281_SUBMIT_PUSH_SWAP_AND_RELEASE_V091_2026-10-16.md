# 281 批：提交推送（275–280 累积件）＋装机位全面换装 0.9.1＋0.9.1 双平台发行（2026-10-16）

> **日期**：2026-10-16；**用户令**：「请先进行提交并推送吧，我考虑接下来进一轮我个人的真机使用，
> 等结束后再看真机运行情况来进行验收」→「请全面换装0.9.1，远端也发布0.9.1吧」。
> **性质**：零源码语义增量——父仓与 orz 子仓累积件推送＋**web 桥装机位全面换装 0.9.1**＋
> **0.9.1 双平台打包发行**＋README 发布面对齐；未闭合计数不变 **61**。
> **结论先行**：orz 子树 `0ba00262..0be4a364` → `cli/feat/fusion-architecture` exit 0；
> 父仓 `490d327f..5d545357` → `origin/main` exit 0（两仓推送前均为纯快进、`ls-remote` 逐位一致）；
> 装机位 `C:\Users\1\AppData\Local\orz\bin` 三件套 0.9.0 → **0.9.1**（旧件留 `.0.9.0-bak` 链）；
> **GitHub Release `v0.9.1` 已发布**（双平台包＋`SHA256SUMS`）；服务端回读与回下载复核随后续补记
> （沿 224 → 225／244 → 245／248 → 249 先例）。

---

## §1 提交推送（275–280 累积件）

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `0ba00262..0be4a364`（4 提交：`4749f6b1` 276 0dd S2 落码；`644f6b01` 278 0de S2 落码；`1ecad57c` 279 审查处置；`0be4a364` 280 bump 0.9.0→0.9.1） | `cli/feat/fusion-architecture` | exit 0 |
| 父仓 | 275–280 累积落账件（六份批档＋两份设计稿＋索引 v4.253＋BACKLOG 两卷＋TODO＋`orz_source_manifest.sha256` 重生成） | `origin/main` | exit 0 |

- **推送前状态复核**：两仓工作树均干净（`--porcelain --untracked-files=all` 空）；`rev-list --count`
  显示父仓 ahead 6／behind 0、orz ahead 4／behind 0——**纯快进**，无分叉、无合并需求。
- **推送后复核**：`git ls-remote origin refs/heads/main refs/heads/feat/fusion-architecture`
  ＝ `5d545357…`／`0be4a364…`，与本地逐位一致；两仓 ahead/behind 归零。
- **账面勘正**：280 批及其前批账面所记「未推送未发行」于本批兑现——BACKLOG 指针行内 280 片段
  就地勘正为「推送与发行随后续批（281）」。**发行面不再停 v0.9.0**。

## §2 装机位全面换装 0.9.1（web 桥装机位）

**装机位**＝`C:\Users\1\AppData\Local\orz\bin`（`Get-Command orz` 解析所得即该位；`Application Data`
目录无 `InstallLocation` 重定向）。沿 **266 批**口径：装机位换装属**用户侧动作、非账本项目**，
仅在本档留痕。

| 阶段 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| 换装前（0.9.0） | `orz.exe` | 58,353,152 | `0c07208d121af341fc05b336e4c4c57e2fe887dd9e4f4220335541f7997d0bb4` |
| 换装前（0.9.0） | `orz-signer.exe` | 6,740,480 | `f7bac89adc9235774ebc0ea160cb9199917b7ad16365f24c6faec560f10a02d5` |
| 换装前（0.9.0） | `orz-acaf-provision.exe` | 6,640,128 | `93cafd3cfdec4e34b38592819292053fb498c93f785bcd161443346c7a01731b` |
| 换装后（0.9.1） | `orz.exe` | 58,369,024 | `39f6f3fac740b2a0b91322d057974c7ab33d6694264b809fd01cb6628fd84578` |
| 换装后（0.9.1） | `orz-signer.exe` | 6,740,480 | `dd55ded5abd2ebbf6e541d4e5955ea9cb6f4246de10016609b5ea11d060c85f4` |
| 换装后（0.9.1） | `orz-acaf-provision.exe` | 6,640,128 | `34563cad863533324d5dc98c8c52888aa540e4fe87e7a1445c1a2d7cefd22acb` |

- **换装前件与在役真值互证**：三件与 247／248 批账面 0.9.0 在役值**逐位吻合**（`0c07208d…`／
  `f7bac89a…`／`93cafd3c…`）——即装机位此前确为 0.9.0 发行载体，本次换装前状态可机械复核。
- **换装动作**：旧件就地留备份链 `.0.9.0-bak`（三件各一件，均保留）；9-25 老件
  `orz.exe.bak-0925`（92,742,656 B）原样保留；随后自 `D:\tb-eval\orz-windows\` 直写 0.9.1 三件套。
- **换装后核证**：`orz.exe --build-info` ⇒ `version=0.9.1 os=windows arch=x86_64 profile=release`
  rc 0；三件 SHA256 与来源件**逐位一致**（上表）。
- **未做 ACAF 重 provision**：装机位 `acaf\` 仅存 `installation-key.tmp`（2026-09-14），无
  `signer-manifest.json`——沿 266 批口径只换三件套，不重 provision（**如实记**）。
- **零启动面动作**：本批**未**以装机位载体发起任何会话/run（避免污染用户真机面）；只有只读
  `--build-info` 一次。

## §3 0.9.1 发行包（`rel-281-stage` 打包）

- **打包**（[`.tmp-b281-package.ps1`](../../.tmp-b281-package.ps1)，沿 248／244／224／195／183／156／122
  形态）：两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.9.1`
  （`generated_at`＝2026-10-16T12:52:49Z／12:52:51Z）。
- **包内 README**：由 0.9.0 版改写为 **0.9.1 版**（74,554 B／`543c5800…`）——新增 0.9.1 概要段与
  **0.9.1 详解节**（0dd run 末自动归档退役 D1–D8／N1–N2 钉／边界；0de 四件 F1–F4＋不立两项）；
  **Linux 快速开始解包文件名勘正** `orz-0.8.15-…` → `orz-0.9.1-linux-x86_64.tar.gz`（248 版遗留
  陈旧行，本批顺带勘正）；版本信息行改 `源码 orz 0be4a364`／构建 2026-10-16。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.9.1-windows-x86_64.zip` | 28,441,423 | `9bc5c7d4518f513d5b1d8b6bc933c51d597382490988380bc003195a64984398` |
| `orz-0.9.1-linux-x86_64.tar.gz` | 37,707,315 | `07f67e30a4ac28b1f83dd44f16e4c40de4bad7b5b515cd61d1fcd10c8d0b625a` |
| `SHA256SUMS`（顶层） | 191 | `a883300123ecd9c821ea19b7107cde235794cb81ea10e3c2ca9274d7d6f77173` |

- **在役载体真值（与 280 批账面逐位一致）**：WIN `orz.exe` 58,369,024 `39f6f3fa…`／
  `orz-signer.exe` 6,740,480 `dd55ded5…`／`orz-acaf-provision.exe` 6,640,128 `34563cad…`；
  LIN `orz` 117,192,312 **`f50b1f17…`（身份门值）**／`orz-signer` 1,397,496 `a57a4d29…`／
  `orz-acaf-provision` 1,215,952 `46a7b6d0…`。
- **版本核证（双平台容器内）**：

| 项 | 命令 | 结果 |
|---|---|---|
| 包内 WIN `--build-info` | `_verify_zip\orz.exe --build-info` | `version=0.9.1 os=windows arch=x86_64 profile=release` rc 0 |
| 包内 LIN 校验 | `alpine:3.20` 内 `sha256sum -c SHA256SUMS` | **4/4 OK**（`orz`／`orz-signer`／`orz-acaf-provision`／`README.md`） |
| 包内 LIN 载体形态 | `alpine:3.20` 内 `file orz` | `ELF 64-bit LSB pie executable … static-pie linked` |
| 包内 LIN `--build-info` | `alpine:3.20` 与 `debian:bookworm-slim` 各一次 | 均 `version=0.9.1 os=linux arch=x86_64 profile=release` rc 0 |

- **口径说明**：0.9.1 为相对上一发行版 **0.9.0** 的正式增量版（沿 218／243 批先例取补丁位递增
  的用户令落点）；248 批未跑容器冒烟（docker 未启动），本批 docker 在役故补齐双镜像核证。

## §4 发行（GitHub Release v0.9.1）

- 命令＝`gh release create v0.9.1 --target main --title "orz 0.9.1（会话生命周期用户主权：run 末
  自动归档退役 · 上下文治理摩擦综合四件 · 双平台包）" --notes-file .tmp-b281-release-notes.md`
  ＋三资产；tag `v0.9.1` = 本档所在提交（父仓轻量 tag，沿 122／156／183／195／224／244／248
  形态）；发布后为 **Latest**。
- **服务端回读与完整回下载复核随后续补记**（224 → 225／244 → 245／248 → 249 补记同形）。

## §5 台账与门禁

- README 发布面对齐 0.9.1（Linux 解包文件名 `orz-0.9.1-linux-x86_64.tar.gz`＋最新 release 链接
  `v0.9.1`）。
- 索引头行 v4.253 → **v4.254**；BACKLOG 计数行（61 不变）＋本批指针行（280 片段「未推送未发行」
  就地勘正）；BACKLOG 第二卷 §1.218；TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §6 边界与如实记

1. **零源码语义增量**：orz 指针本批不动（`0be4a364` 为 280 批 bump 批提交）；本批只做推送、
   装机位换装、打包与发行。
2. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170／187／194／
   218／224／243／247／248 同口径）；装机位亦未重 provision（§2）。
3. **装机位换装非账本项目**（沿 266 批）——本档留痕只作可复核记录，不进入未闭合计数。
4. **未推送面清零**：275–280 父仓累积件与 orz 四提交已于本批推送；`_hdr_new.txt` 一类邻窗
   残留件未复现。
5. **0dd／0de S4 真机复验仍待续**：本批不推进该面——用户已预告「接下来进一轮我个人的真机使用」，
   验收随该轮读数进行（0dd＝web 同会话跨 prompt／跨日续用不归档＋手动面回归；0de＝长文写作轮
   收束段无空转＋全角括号注解摘要可解析＋UI 估数非 0）。
6. **发行回读核证**（服务端 digest＋回下载 identical）随后续补记批兑现，本档只记发布动作。

## §7 关联与关键词

[`280 批档`](280_CARRIER_REBUILD_V091_0DD_0DE_S3_2026-10-16.md)（载体重建母批）／
[`248 批档`](248_SUBMIT_PUSH_AND_RELEASE_V090_2026-10-11.md)（同形态先例＋补记先例）／
[`249 补记`](249_RELEASE_V090_READBACK_VERIFICATION_2026-10-11.md)／
[`244 批档`](244_SUBMIT_PUSH_AND_RELEASE_V0816_2026-10-11.md)／
[`224 批档`](224_SUBMIT_PUSH_AND_RELEASE_V0815_2026-10-07.md)／
[`266 批档`](266_0DD_SESSION_LIFECYCLE_USER_SOVEREIGNTY_PROJECT_2026-10-13.md)（装机位留痕先例）。

关键词：281 批、提交推送、装机位全面换装 0.9.1、`0.9.0-bak` 备份链、GitHub Release v0.9.1、
`rel-281-stage`、6/6 MATCH、包内 README 0.9.1 版、Linux 解包名勘正、alpine 3.20／bookworm 双核证、
`f50b1f17…` 身份门、`9bc5c7d4…`／`07f67e30…`／`a8833001…`、计数 61 不变、索引 v4.254。
