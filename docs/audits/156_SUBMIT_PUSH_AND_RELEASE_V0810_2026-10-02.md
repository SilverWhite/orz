# 156 批：提交推送（143–155 累积件）与 0.8.10 发行 ＋ 0cm 对外英文翻译线立项（2026-10-02）

> **结论先行**：143–155 批累积件全部入仓并推送——orz 子树 `20e4c574..55d61c47`
> （`feat/fusion-architecture`），父仓 `1552e4d1..d282046b`（`main`）；**GitHub Release
> `v0.8.10` 已发布**（双平台包＋`SHA256SUMS`，服务端 digest 与本地产物逐位一致，完整回下载
> 复核 `identical`）。同时按用户令**新增 `0cm` 对外英文翻译线**立项登记，**计数 59 → 60**。

> **日期**：2026-10-02；**用户令**：「当前新增0cm 请先进行提交推送与安装包版本发行吧」。
> **形态**：两项父仓提交（① 143–155 材料；② 本批＝0cm 立项＋156 落账）＋orz 子树推送＋
> 打包重建＋发行。

---

## §1 orz 子树推送

| 项 | 值 |
|---|---|
| 本批提交 | `d5632248`（153/154 落码：0ck 注解＋0cl 瘦身）／`41ab8f8e`（`chore(release): bump 0.8.9 -> 0.8.10`）／`55d61c47`（重建判据补笔：0ck 注解句丢失修复） |
| 推送 | `20e4c574..55d61c47 → feat/fusion-architecture`（`cli` remote）**exit 0** |
| 说明 | 远端此前停在 `20e4c574`（0cd 清退点）；本次一次带上 0.8.8–0.8.10 的 orz 侧 **9 个提交** |
| 起点前移 | 父仓子模块 pin `20e4c574` → `55d61c47`；`orz_source_manifest.sha256` 重生成，`--check` = `valid` |

## §2 父仓提交与推送

| 提交 | 内容 | 规模 |
|---|---|---|
| **`d282046b`**（材料批） | 143–155 批落账：0ch 精准化 S1–S4 闭合、0ce/0cf/0cg 与 0au 尾巴退役、评测线裁决与 0ci/0cj 立项、0ck/0cl 落码与 0.8.10 载体重建；含 12 份新批档＋README 发布面对齐＋设计档增补＋源清单 | 24 files `+2211/−599` |
| **本档所在提交**（156 批） | 0cm 立项（charter＋四处台账登记）＋本档 | 见推送批 |

- 推送：`1552e4d1..HEAD → origin/main` **exit 0**；工作树（除 `.tmp-*` 与 `rel-156-stage` 场外件）clean。

## §3 0.8.10 发行包（`rel-156-stage` 重建）

- **打包**（[`.tmp-b156-package.ps1`](../../.tmp-b156-package.ps1)，沿 144／122 形态）：两侧各
  **5 entries**（三件套＋`README.md`＋`SHA256SUMS`；`carrier-manifest.json` 由打包阶段按包内
  实际内容生成，`kind=orz-carrier-manifest` `version=0.8.10`）；包内六件与在役载体 **6/6 MATCH**。
- **README（发行前改写）**：包内 `README.md` 由 0.8.8 版改写为 **0.8.10 版**——新增 0.8.10 与
  0.8.9 两节更新说明；0.8.8／0.8.9 改标「中间载体，内容已含于 0.8.10」；版本信息行
  `55d61c47`／2026-10-02（0.8.10 双平台重建）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.8.10-windows-x86_64.zip` | 27,994,761 | `7e2c4623ef3631efc5d475d7c98db18dab99031e7f2cb8e983a35337ce6d03e8` |
| `orz-0.8.10-linux-x86_64.tar.gz` | 37,153,264 | `66b0053bc4f3bf03d8c21120ec4e9be523487850640b7dff35badde1d21e859d` |
| `SHA256SUMS`（顶层） | 193 | `ea68a58f0af87c5da7ceeefe2aeafa2ba2c390184c6706dcfcbb9d1c9ffbff9b` |

- **容器核证（alpine 3.20）**：`lin/` `sha256sum -c` **4/4 OK**、`win/` **4/4 OK**；
  Linux 解包态 `--build-info` = `version=0.8.10 os=linux`；Windows `--build-info` =
  `version=0.8.10 os=windows`。
- **清单活体两态**（触发通道＝`orz --version`；容器无 tty 时以 `tui io error` 结束＝144 批已知
  形态，判据只看 carrier-integrity finding）：解压态**零 finding** ✓；`README.md` 追加一字节后
  **恰 1 条** = `README.md: 长度不符（清单 49743 ≠ 实际 49744）` ✓。

## §4 发行（GitHub Release v0.8.10）

- **GitHub Release `v0.8.10`** 已发布（Latest）：<https://github.com/SilverWhite/CLI/releases/tag/v0.8.10>；
  命令＝`gh release create v0.8.10 --target main --title … --notes-file .tmp-b156-release-notes.md`
  ＋三资产；tag `v0.8.10` 指向父仓**本档所在提交**（轻量 tag，沿 122 批形态）。
- **服务端回读**：`gh release view v0.8.10 --json assets` 逐资产 digest 与本地产物对照——
  zip／tar／`SHA256SUMS` **逐位一致**；`gh release download` 回下载后与本地产物比对 **identical**。

## §5 0cm 对外英文翻译线立项登记

- **charter**（范围与质量口径权威）：[`EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md`](../EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md)
  ——为 Reddit 征求建议产出**最小英文门面集**（D1 术语对照表／D2 `README.en.md`／D3 跑分报告英文档／
  D4 写控当前态蒸馏／D5 ACAF 当前态蒸馏），有效翻译量约 1.5 万中文字；质量口径＝对照表注入＋
  表格数字程序化照抄＋英文档头声明（`LLM-assisted translation; the Chinese originals are authoritative.`），
  不依赖人工逐句审查；批序 S1 对照表 → S2 README → S3 设计当前态蒸馏 → S4 报告英译＋发帖包。
- **登记四处**：BACKLOG（P1 名册＋计数行 **59 → 60**＋开放项行＋新增 `0cm` 节）；TODO（计数行＋
  P1 路由行＋新增 `P1-0cm` 勾选节）；索引（头行 v4.125＋`AUTH-EN-TRANSLATION-LINE` 条目＋§8 桶）；
  本卷 §1.108。
- **边界**：不翻译 docs/ 全量、不翻译台账与审计、不动 `orz/README.md`、不承诺逐句等价、不新造英文品牌名。

## §6 台账与门禁

| 时点 | 读数 |
|---|---|
| 提交前 | `check_repository.py` ⇒ **`valid: true`**（`error_count: 0`）；源清单 `--check` ⇒ **`valid`** |
| 推送后 | `check_repository.py` ⇒ **`valid: true`**；索引头行 v4.124 → **v4.125** |

## §7 边界与后续

1. **0ck／0cl 未闭合**：S3（进体）已达成（155 批），**S4 真机核证待续**——RLI 消费率读数随
   `0cj` S3/S4 收取（对照基线 `0bf` S3 的 0/3）；计数不变（两项仍开放）。
2. **中间载体不发**：0.8.8／0.8.9 未单独发行，内容一并含于 0.8.10（沿 0.6.x／0.8.3–0.8.6 口径）。
3. **Linux 载体未做 ACAF 重 provision**（沿 093／100／109／122／144／147 同口径）；Windows 在役目录已重 provision。
4. **打包顺延已兑现**：155 批「打包顺延发行批」由本批补上。
5. 下一步＝`0ci` Frontier-Bench 试点线 S0（钉数据集仓库与版本 pin，顺位第一）／`0cj` SlopCodeBench 挑题；
   `0cm` 按 charter 批序 S1 起。

## §8 关联与关键词

[`155 批档`](155_CARRIER_REBUILD_V0810_0CK_0CL_S3_2026-10-02.md)／
[`122 批档`](122_SUBMIT_PUSH_AND_RELEASE_2026-09-30.md)（同形态先例）／
[`0cm charter`](../EN_TRANSLATION_PROJECT_CHARTER_2026-10-02.md)／BACKLOG `0cm`／TODO `P1-0cm`。

关键词：156 批、提交推送、父仓 `d282046b`、orz 子树 `55d61c47`、GitHub Release v0.8.10、
rel-156-stage、回下载逐位一致、0cm 对外英文翻译线立项、计数 59 → 60、未闭合 60。
