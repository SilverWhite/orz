# 244 批：提交推送（226–243 累积件）与 0.8.16 发行（2026-10-11）

> **日期**：2026-10-11；**用户令**：「请先进行提交并推送吧，新的安装包也推送上去」。
> **性质**：零源码——父仓提交推送＋0.8.16 双平台打包发行＋README 发布面对齐；未闭合计数不变 **59**。
> **结论先行**：orz 子树 `10cfe765..316107f6` → `cli/feat/fusion-architecture` exit 0；父仓
> `101d2226..06b0625d` → `origin/main` exit 0；**GitHub Release `v0.8.16` 已发布**（双平台包＋
> `SHA256SUMS`）；服务端回读与回下载复核随后续补记（沿 224 → 225 补记先例）。

---

## §1 推送

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `10cfe765..316107f6`（2 提交：`39116a81` 238/241/242 落码＋`316107f6` 243 bump 0.8.15→0.8.16） | `cli/feat/fusion-architecture` | exit 0 |
| 父仓 | `101d2226..06b0625d`（226–243 累积落账） | `origin/main` | exit 0 |

- **推送前状态复核**：`git ls-remote` 显示远端 `refs/heads/main` = `101d2226`、
  `refs/heads/feat/fusion-architecture` = `10cfe765`——与本地基准一致，两仓均为快进。
- 父仓本批前工作树驻留 226–243 全部落账件（邻窗完成、未提交），随本批一次入仓
  （提交 `06b0625d`）；`_hdr_new.txt` 为遗留草稿、不入仓。

## §2 0.8.16 发行包（`rel-244-stage` 打包）

- **打包**（[`.tmp-b244-package.ps1`](../../.tmp-b244-package.ps1)，沿 224／195／183／156／122 形态）：
  两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.8.16`。
- **README（包内）**：由 0.8.15 版改写为 **0.8.16 版**（66,413 B／`3fb0ede5…`）——新增
  0.8.16 概要段与 **0.8.16 详解节**（0cz `context_manage` 第十一主工具＋0da 清零态兜底修复与
  清理操作多样化），版本信息行改 `源码 orz 316107f6`／构建 2026-10-11。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.16-windows-x86_64.zip` | 28,133,658 | `06a6df442b27c9bb2e4108fab8c10840537408c3fef38e0a732235fe2d14b377` |
| `orz-0.8.16-linux-x86_64.tar.gz` | 37,298,143 | `11f5e57324467ab319c4235dcdf648f2f9d58fde5362c9611606211a3a3a0586` |
| `SHA256SUMS`（顶层） | 193 | `1b41790c3e1a1a855b83d195ee4bd152f569c7e1bb900476342bd2f40269357c` |

- **在役载体真值（与 243 批账面逐位一致）**：WIN `orz.exe` 57,531,392 `76904567…`／
  `orz-signer.exe` 6,740,480 `1919b56f…`／`orz-acaf-provision.exe` 6,640,128 `446d8cc7…`；
  LIN `orz` 116,019,040 **`db95ed15…`（身份门值）**／`orz-signer` 1,397,464 `d462590f…`／
  `orz-acaf-provision` 1,216,048 `a8c2dd4c…`。
- **版本核证**：Windows `orz.exe --build-info` = `version=0.8.16 os=windows arch=x86_64
  profile=release`（rc 0）；Linux 双冒烟 `0.8.16 os=linux` rc0（243 批 alpine 3.20／bookworm）；
  本批字节面复核＝双载体各恰 1 处 `0.8.16`、0 处 `0.8.15`（docker 守护进程未启动，
  Linux 未重跑容器，如实记）。
- **中间载体口径**：0.8.16 为相对上一发行版 0.8.15 的正式增量版；0.8.15 为上一发行版（224 批）。

## §3 发行（GitHub Release v0.8.16）

- 命令＝`gh release create v0.8.16 --target main --title "orz 0.8.16（模型主控上下文 context_manage ·
  清零态兜底修复与清理操作多样化 · 双平台包）" --notes-file .tmp-b244-release-notes.md`＋三资产；
  tag `v0.8.16` = 本档所在提交（父仓轻量 tag，沿 122／156／183／195／224 形态）；发布后为 **Latest**。
- **服务端回读与完整回下载复核随后续补记**（224 → 225 补记同形）。

## §4 台账与门禁

- README 发布面对齐 0.8.16（Linux 解包文件名 `orz-0.8.16-linux-x86_64.tar.gz`＋最新 release 链接）。
- 索引头行 v4.216 → **v4.217**；BACKLOG 计数行（59 不变）＋本批指针行；BACKLOG 第二卷 §1.190；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 边界与如实记

1. **零源码**：orz 子仓本批无新提交内容（`39116a81`／`316107f6` 为 238／241／242 落码＋243 bump）；
   本批只做提交推送＋打包发行。
2. **226–243 累积件此前未提交**（邻窗完成、工作树驻留），本批一次入仓（`06b0625d`）。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170／187／194／218／224 同口径）；
   Windows 在役目录已重 provision（243 批）。
4. Linux 容器版本核证因 docker 守护进程未启动未能复跑，以 243 批双冒烟读数＋本批字节面佐证。
5. **0cz／0da S4 与 0cw 干净重跑仍待 0cy 卫生前置**（不随本批推进）。

## §6 关联与关键词

[`243 批档`](243_CARRIER_REBUILD_V0816_0CZ_0DA_2026-10-11.md)／
[`224 批档`](224_SUBMIT_PUSH_AND_RELEASE_V0815_2026-10-07.md)（同形态先例＋补记先例）／
[`195 批档`](195_SUBMIT_PUSH_AND_RELEASE_V0814_2026-10-04.md)／
[`183 批档`](183_SUBMIT_PUSH_AND_RELEASE_V0812_2026-10-04.md)／
[`156 批档`](156_SUBMIT_PUSH_AND_RELEASE_V0810_2026-10-02.md)。

关键词：244 批、提交推送、GitHub Release v0.8.16、rel-244-stage、6/6 MATCH、README 发布面对齐、
`context_manage`、0cz、0da、`db95ed15…` 身份门、计数 59 不变、索引 v4.217。
