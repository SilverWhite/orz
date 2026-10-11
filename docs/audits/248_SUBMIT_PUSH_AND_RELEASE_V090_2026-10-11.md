# 248 批：提交推送（246–247 累积件）与 0.9.0 发行（2026-10-11）

> **日期**：2026-10-11；**用户令**：「请先进行提交与推送吧，并发行0.9.0」。
> **性质**：零源码——父仓提交推送＋0.9.0 双平台打包发行＋README 发布面对齐；未闭合计数不变 **59**。
> **结论先行**：orz 子树 `316107f6..a942d6f3` → `cli/feat/fusion-architecture` exit 0；
> 父仓 246–247 累积落账件 → `origin/main` exit 0；**GitHub Release `v0.9.0` 已发布**
> （双平台包＋`SHA256SUMS`）；服务端回读与回下载复核随后续补记（沿 224 → 225／244 → 245 先例）。

---

## §1 推送

| 仓 | 范围 | 目标 | 结果 |
|---|---|---|---|
| orz 子模块 | `316107f6..a942d6f3`（2 提交：`af338387` 246 落码＝0cy S2 GSA 会话卫生在役＋摩擦④ permission 护栏收口；`a942d6f3` 247 bump 0.8.16→0.9.0） | `cli/feat/fusion-architecture` | exit 0 |
| 父仓 | 246–247 累积落账件（246／247 批档＋0cy 设计稿＋0DA S4 处理轮报告＋索引／TODO／BACKLOG／BACKLOG 第二卷／身份门脚本） | `origin/main` | exit 0 |

- **推送前状态复核**：`git ls-remote` 显示远端 `refs/heads/feat/fusion-architecture` = `316107f6`、
  `refs/heads/main` = `dce03a03`——与本地基准一致，两仓均为快进。
- 父仓本批前工作树驻留 246–247 未提交件（邻窗完成），随本批一次入仓；`_hdr_new.txt` 已于 247 批删除。

## §2 0.9.0 发行包（`rel-248-stage` 打包）

- **打包**（[`.tmp-b248-package.ps1`](../../.tmp-b248-package.ps1)，沿 244／224／195／183／156／122 形态）：
  两侧各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）；
  包内六件与在役载体 **6/6 MATCH**；manifest `kind=orz-carrier-manifest` `version=0.9.0`。
- **README（包内）**：由 0.8.16 版改写为 **0.9.0 版**（69,845 B／`e21456d2…`）——新增
  0.9.0 概要段与 **0.9.0 详解节**（0cy `.gsa` 会话卫生：收卷时机 D1／扫描口径 D2／足迹清单 D3／
  包 schema v0.3 D4／零契约变化 D5／双 env D6／unarchive 与 delete 升级 D7／共享件归属 D8＋边界），
  版本信息行改 `源码 orz a942d6f3`／构建 2026-10-11。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.9.0-windows-x86_64.zip` | 28,436,427 | `f8716854ffb61762e550b23bbd0408179ab5d6b3c2aac87a2f13a4d13dcb4ef8` |
| `orz-0.9.0-linux-x86_64.tar.gz` | 37,697,953 | `8529e090eb7839f87640a51d0198a18fbcdd2024691fe97e8c475fc095c5ea4d` |
| `SHA256SUMS`（顶层） | 191 | `78ab2e6b9cc8213282fc21d9f09f95daa5526d206b164335462df60f8c86ee14` |

- **在役载体真值（与 247 批账面逐位一致）**：WIN `orz.exe` 58,353,152 `0c07208d…`／
  `orz-signer.exe` 6,740,480 `f7bac89a…`／`orz-acaf-provision.exe` 6,640,128 `93cafd3c…`；
  LIN `orz` 117,188,296 **`1ce40857…`（身份门值）**／`orz-signer` 1,397,512 `7cf5dbe2…`／
  `orz-acaf-provision` 1,215,928 `2ee04153…`。
- **版本核证**：包内 `orz.exe --build-info` = `version=0.9.0 os=windows arch=x86_64
  profile=release`（rc 0）；双载体字节面＝`0.9.0` 在场（WIN 18 处／LIN 6 处）、
  `0.8.16` 与 `0.8.15` 各 0 处。
- **中间载体口径**：0.9.0 为相对上一发行版 0.8.16 的正式增量版；0.8.16 为上一发行版（244 批）。

## §3 发行（GitHub Release v0.9.0）

- 命令＝`gh release create v0.9.0 --target main --title "orz 0.9.0（`.gsa` 会话卫生：新对话新台账 ·
  上一轮整卷进存档 · 双平台包）" --notes-file .tmp-b248-release-notes.md`＋三资产；
  tag `v0.9.0` = 本档所在提交（父仓轻量 tag，沿 122／156／183／195／224／244 形态）；发布后为 **Latest**。
- **服务端回读与完整回下载复核随后续补记**（224 → 225／244 → 245 补记同形）。

## §4 台账与门禁

- README 发布面对齐 0.9.0（Linux 解包文件名 `orz-0.9.0-linux-x86_64.tar.gz`＋最新 release 链接）。
- 索引头行 v4.219 → **v4.220**；BACKLOG 计数行（59 不变）＋本批指针行；BACKLOG 第二卷 §1.194；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 边界与如实记

1. **零源码**：orz 子仓本批无新提交内容（`af338387`／`a942d6f3` 为 246 落码＋247 bump）；
   本批只做提交推送＋打包发行。
2. **246–247 累积件此前未提交**（邻窗完成、工作树驻留），本批一次入仓。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147／155／170／187／194／218／224／243 同口径）。
4. Linux 容器未重跑冒烟——以 247 批 alpine 3.20／bookworm 双冒烟读数＋本批字节面佐证（docker 守护进程本批未启动，如实记）。
5. **0cy S4 真机核证与 0cz／0da 联动轮仍待跑**；**0cw 干净重跑成本门另裁**（不随本批推进）。

## §6 关联与关键词

[`247 批档`](247_CARRIER_REBUILD_V090_0CY_S3_2026-10-11.md)／
[`246 批档`](246_0CY_S2_GSA_SESSION_HYGIENE_2026-10-11.md)／
[`244 批档`](244_SUBMIT_PUSH_AND_RELEASE_V0816_2026-10-11.md)（同形态先例＋补记先例）／
[`224 批档`](224_SUBMIT_PUSH_AND_RELEASE_V0815_2026-10-07.md)／
[`195 批档`](195_SUBMIT_PUSH_AND_RELEASE_V0814_2026-10-04.md)／
[`183 批档`](183_SUBMIT_PUSH_AND_RELEASE_V0812_2026-10-04.md)。

关键词：248 批、提交推送、GitHub Release v0.9.0、rel-248-stage、6/6 MATCH、README 发布面对齐、
0cy、`.gsa` 会话卫生、`1ce40857…` 身份门、计数 59 不变、索引 v4.220。
