# orz 0.5.1（0ac S3 载体重建）

Windows x86_64 / Linux x86_64（musl static-pie）双平台发布包。
源冻结基线：orz `dbb42b1d`（`feat/fusion-architecture`，0.5.0 之后 9 提交：
0ac S3①② 落地、`immediate_feedback` 法官面、本地分段检索、`ea777918`
宿主资源事件修复、审记 G1/G2 修复）；版本 bump **0.5.0 → 0.5.1**（两文件
两行）；重建 2026-09-14（run `RUN-CLI-6aa77e19`），发布物与冻结源一致。

## 资产（GitHub Release v0.5.1）

| 资产 | 大小 (B) | SHA256 |
|---|---|---|
| orz-0.5.1-linux-x86_64.tar.gz | 34,987,981 | `49df6cebbce12375cb2969f2bed03dfdfc8f4b02f5b89764f6475fb05ce7cdf0` |
| orz-0.5.1-windows-x86_64.zip | 26,472,460 | `f7e8274c706c9b0cf202ccc126d220e4719097bf98847fe3478c6b368689432f` |

双包内容：orz 主程序 + orz-signer + orz-acaf-provision + README（含两平台
快速开始与 0.5.1 更新说明）。

## 0.5.1 更新要点（相对 0.5.0）

- **本地分段检索（0ac S3）**：`web_search` 增加本地 SERP 分段解析路径
  （`retrieval_path=local_segmented`）——HTML 实体解码（含命名实体）、
  广告位跳过、结构容错；独立/整体两档截止口径 + 开关；解析器字符边界
  panic（G1）已修。
- **检索族探针与工具探针扩面（0ac S3①②）**：`retrieval_family` 探针与
  `tool_probe` 扩面；失败 `cause` 自描述形状（壳码/孤儿目标拒绝）。
- **法官规则镜像（0ac S3-a）**：S2 机器契约 fixture 判读、首个结果截止族
  与零事件路径钉子；Rust 法官与 Python 参考实现镜像同步。
- **宿主资源事件补项**：`host_resource_snapshot` 事实判定表补齐
  （`GAP-ORZ-HOST-RESOURCE-SNAPSHOT-DROP`）。
- **修复与可测性**：`fmt --check` 16 处 → 0、新文件 clippy 2 处 → 0、
  会话面测试环境口径（F-012）；`orz-tools` / `orz-host` 测试目标补
  `cfg` 门（Linux 可测性）。

## 二进制哈希（载体 `D:\tb-eval\orz-windows` / `orz-linux`）

| 文件 | 大小 (B) | SHA256 |
|---|---|---|
| orz.exe | 52,837,376 | `0eff8ef8568ed21d201c45b34e1900afa8a4cd8f7ecbca5234c8d31f45206485` |
| orz-signer.exe | 6,742,528 | `19edf5bd4595b6f839a9a46d344069a235f19e9a3ae153b5f2b4a45116b4fea8` |
| orz-acaf-provision.exe | 6,642,176 | `9fbc26397012b4211269c8b1c062e21ffa4530e706d21fa5d8d2be7cc42ea558` |
| orz（musl static-pie） | 110,099,784 | `149ab44658f36d4e8dc10877e0a3f45cf6054ebc3fcccd58cfedf8c5e73d4b8d` |
| orz-signer | 1,397,552 | `97978a526894f9aa7032b69f9e0776c2b97e2a90fc290faecda66ddf532d949c` |
| orz-acaf-provision | 1,216,344 | `9ee21fcc8ac7d5970e7c3a76eb09a700ca3b7bed69c0901792ba94004ad3d0c7` |

## 验证摘要

- ELF 静态核验：三件 `ET_DYN` + x86-64 + **PT_INTERP=0**（static-pie OK）。
- 双向加载冒烟绿：`debian:bookworm-slim`（glibc）+ `alpine:3.20`（musl），
  预期 exit 1 形态三件一致。
- 接线符号/字面量双平台核证：`local_segmented` 6/54、`retrieval_family`
  1/4、`ORZ_RETRIEVAL_ENGINES` / `ORZ_WEB_SEARCH_LOCAL` 各 2/2、`0.5.1`
  24/17；浏览器/检索/轮次接线标记全命中。
- 包完整性：两包逐成员 sha256 与载体/暂存逐对 `match=True`（×8）；
  发布后下载重哈希与服务端 digest 三方一致。
- manifest 1448 条 + `check_repository` `valid: true` / `error_count: 0`。

## 边界

本批只做重建、加载冒烟与字节串核证；0ac 投递侧（①-b：三事件写点 /
I1–I3 / M1–M3）未落、0ac 维持 open；资源门/回收/树杀实机行为与 0ac
检索/投递实机取证另行放行（S4 / 后续批次）。

重建细节与边界：[`0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14`](../../docs/audits/0AC_S3_DUAL_PLATFORM_REBUILD_2026-09-14.md)。
