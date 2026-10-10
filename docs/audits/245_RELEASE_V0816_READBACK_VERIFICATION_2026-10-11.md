# 245 补记：0.8.16 发行回读核证（服务端 digest 逐位一致＋回下载 identical 3/3）（2026-10-11）

> **日期**：2026-10-11；**性质**：**发行回读核证批**（零源码、零跑批、零改动）——
> 244 批 §3「服务端回读与完整回下载复核随后续补记」的兑现；未闭合计数不变 **59**。
> **结论先行**：`v0.8.16` 三资产的**服务端 digest 与本地产物逐位一致**；`gh release download`
> 完整回下载**三件与本地 identical 3/3**（逐字节相等）；release 为 **Latest**，tag 为**轻量 tag**
> ＝父仓 `8ec44021`（244 批本档所在提交）。

---

## §1 服务端回读（`gh release view --json`）

| 资产 | 服务端 size | 服务端 digest（sha256） | 本地 | 判定 |
|---|---:|---|---|---|
| `orz-0.8.16-windows-x86_64.zip` | 28,133,658 | `06a6df442b27c9bb2e4108fab8c10840537408c3fef38e0a732235fe2d14b377` | 同 | **逐位一致** |
| `orz-0.8.16-linux-x86_64.tar.gz` | 37,298,143 | `11f5e57324467ab319c4235dcdf648f2f9d58fde5362c9611606211a3a3a0586` | 同 | **逐位一致** |
| `SHA256SUMS`（顶层） | 193 | `1b41790c3e1a1a855b83d195ee4bd152f569c7e1bb900476342bd2f40269357c` | 同 | **逐位一致** |

- **发布态**：`isDraft=false`／`isPrerelease=false`；`gh release list --limit 3` 显示 **Latest**；
  `publishedAt`＝2026-10-10T23:16:13Z（＝2026-10-11 HKT）；`targetCommitish=main`。
- **tag 形态**：`gh api /git/ref/tags/v0.8.16` ⇒ `object.type=commit`、`object.sha=8ec44021…`
  ＝**轻量 tag**（沿 122／156／183／195／224 形态：tag 落在本批提交上）。
- **发行正文**：`--notes-file .tmp-b244-release-notes.md`；回读 body 长度 2,091 字符；
  标题＝`orz 0.8.16（模型主控上下文 context_manage · 清零态兜底修复与清理操作多样化 · 双平台包）`。

## §2 完整回下载复核（`identical` 对拍）

- 落点＝`D:\tb-eval\rel-244-readback-<rand>`（新建目录，非打包暂存区），重下载后与本地逐件对拍：

| 件 | 回下载 size | 回下载 sha256 | 与本地 | 判定 |
|---|---:|---|---|---|
| zip | 28,133,658 | `06a6df44…` | 相等 | **identical=True** |
| tar.gz | 37,298,143 | `11f5e573…` | 相等 | **identical=True** |
| `SHA256SUMS` | 193 | `1b41790c…` | 相等 | **identical=True** |

- ⇒ 远端资产＝本地产物，**无上传截断／替换**（195／225 同判据）。
- **通道差异如实记**：本轮 `gh release download`（三件一次下）**一次跑完**，未复现 225 批的
  大文件停摆（本机出口链路现象，非资产完整性维度）。

## §3 边界与如实记

1. **零改动**：本批未触碰仓库任何产物与载体；仅新增本档与台账行。
2. **Windows `--build-info` 复核**＝`0.8.16 os=windows` rc0（244 批）；Linux 双冒烟 `0.8.16
   os=linux` rc0（243 批）＋244 批字节面（双载体各恰 1 处 `0.8.16`、0 处 `0.8.15`）；
   本批不重跑容器（docker 守护进程未启动）。

## §4 台账与门禁

- 索引头行 v4.217 → **v4.218**；BACKLOG 计数行（59 不变）＋本批指针行；BACKLOG 第二卷 §1.191；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 关联与关键词

[`244 批档`](244_SUBMIT_PUSH_AND_RELEASE_V0816_2026-10-11.md)（本补记母批）／
[`225 补记`](225_RELEASE_V0815_READBACK_VERIFICATION_2026-10-07.md)（同形态先例）／
[`195 批档`](195_SUBMIT_PUSH_AND_RELEASE_V0814_2026-10-04.md)。

关键词：245 补记、0.8.16 发行核证、服务端 digest 逐位一致、回下载 identical 3/3、Latest、
轻量 tag `8ec44021`、计数 59 不变、索引 v4.218。
