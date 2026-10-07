# 225 补记：0.8.15 发行回读核证（服务端 digest 逐位一致＋回下载 identical 3/3）（2026-10-07）

> **日期**：2026-10-07；**性质**：**发行回读核证批**（零源码、零跑批、零改动）——
> 224 批 §3「服务端回读与完整回下载复核随 225 补记」的兑现；未闭合计数不变 **54**。
> **结论先行**：`v0.8.15` 三资产的**服务端 digest 与本地产物逐位一致**；`gh release download`
> 完整回下载**三件与本地 identical 3/3**（逐字节相等）；release 为 **Latest**，tag 为**轻量 tag**
> ＝父仓 `9f659c0e`（224 批本档所在提交）。

---

## §1 服务端回读（`gh release view --json`）

| 资产 | 服务端 size | 服务端 digest（sha256） | 本地 | 判定 |
|---|---:|---|---|---|
| `orz-0.8.15-windows-x86_64.zip` | 28,034,248 | `76a9f86043d45db31dc231ba1eac36dd8162f101733c810b97e0b27016c1013c` | 同 | **逐位一致** |
| `orz-0.8.15-linux-x86_64.tar.gz` | 37,218,091 | `fafb261d54fbbaf4871959bef2b9f38b69f8bd60d47cbefbf39bc4bc6f8d2b5c` | 同 | **逐位一致** |
| `SHA256SUMS`（顶层） | 193 | `012f04982be97f1f38d58bc8fdec9a460c43fec0624a41de12eebaf9f76a5c49` | 同 | **逐位一致** |

- **发布态**：`isDraft=false`／`isPrerelease=false`；`gh release list --limit 3` 显示 **Latest**；
  `publishedAt`＝2026-10-07T13:50:10Z；`targetCommitish=main`。
- **tag 形态**：`gh api /git/ref/tags/v0.8.15` ⇒ `object.type=commit`、`object.sha=9f659c0e…`
  ＝**轻量 tag**（沿 122／156／183／195 形态：tag 落在本批提交上）。
- **发行正文**：`--notes-file .tmp-b224-release-notes.md`；回读 body 长度 1,929 字符；
  标题＝`orz 0.8.15（.gsa 读向全开放 · 拦截信封备份指引 · 黑板 findings 分区 · DeepSeek 接口名对齐 · 双平台包）`。

## §2 完整回下载复核（`identical` 对拍）

- 落点＝`D:\tb-eval\rel-224-verify`（新建目录，非打包暂存区），重下载后与本地产物逐件对拍：

| 件 | 回下载 size | 回下载 sha256 | 与本地 | 判定 |
|---|---:|---|---|---|
| zip | 28,034,248 | `76a9f860…` | 相等 | **identical=True** |
| tar.gz | 37,218,091 | `fafb261d…` | 相等 | **identical=True** |
| `SHA256SUMS` | 193 | `012f0498…` | 相等 | **identical=True** |

- ⇒ 远端资产＝本地产物，**无上传截断／替换**（195 批同判据）。

## §3 边界与如实记（本机网络）

1. **首轮回下载停摆**：`gh release download v0.8.15`（三件一次下）在本机到
   `release-assets.githubusercontent.com` 的**大文件传输**上停摆——`SHA256SUMS`（193 B）即时完成，
   两个大件长时间保持 **0 字节**且进度不再推进（进程存活、无读写增长），故中止该轮。
2. **续传路径成立**：改用 `curl -L -C -`（断点续传＋`--speed-limit/--speed-time` 停摆中止重试）
   后两件均**下载完成**，哈希与 §1／§2 逐位一致；过程中一次 range 探测读物
   （`_range_test.bin`）已清理，**未留在核证目录**。
3. **判据口径**：本补记以**服务端 digest（§1）＋回下载逐位（§2）两路互证**为据；网络停摆属
   本机出口链路现象，**不构成资产完整性疑点**（两路独立通道给出同一字节结论）。
4. **零改动**：本批未触碰仓库任何产物与载体；仅新增本档与台账行。

## §4 台账与门禁

- 索引头行 v4.197 → **v4.198**；BACKLOG 计数行（54 不变）＋本批指针行；BACKLOG 第二卷 §1.171；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 关联与关键词

[`224 批档`](224_SUBMIT_PUSH_AND_RELEASE_V0815_2026-10-07.md)（本补记母批）／
[`195 批档`](195_SUBMIT_PUSH_AND_RELEASE_V0814_2026-10-04.md)（同形态先例：回读＋回下载；
其补记为提交 `e96cb139`，无独立档）。

关键词：225 补记、0.8.15 发行核证、服务端 digest 逐位一致、回下载 identical 3/3、Latest、
轻量 tag `9f659c0e`、gh 大文件停摆、curl 续传、计数 54 不变、索引 v4.198。
