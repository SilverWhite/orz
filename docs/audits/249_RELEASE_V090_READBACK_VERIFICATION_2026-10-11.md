# 249 补记：0.9.0 发行回读核证（服务端 digest 逐位一致＋回下载 identical 3/3）（2026-10-11）

> **日期**：2026-10-11；**性质**：**发行回读核证批**（零源码、零跑批、零改动）——
> 248 批 §3「服务端回读与完整回下载复核随后续补记」的兑现；未闭合计数不变 **59**。
> **结论先行**：`v0.9.0` 三资产的**服务端 digest 与本地产物逐位一致**；`gh release download`
> 完整回下载**三件与本地 identical 3/3**（逐字节相等）；release 为 **Latest**，tag 为**轻量 tag**
> ＝父仓 `4b1da9fb`（248 批本档所在提交）。

---

## §1 服务端回读（`gh release view --json`）

| 资产 | 服务端 size | 服务端 digest（sha256） | 本地 | 判定 |
|---|---:|---|---|---|
| `orz-0.9.0-windows-x86_64.zip` | 28,436,427 | `f8716854ffb61762e550b23bbd0408179ab5d6b3c2aac87a2f13a4d13dcb4ef8` | 同 | **逐位一致** |
| `orz-0.9.0-linux-x86_64.tar.gz` | 37,697,953 | `8529e090eb7839f87640a51d0198a18fbcdd2024691fe97e8c475fc095c5ea4d` | 同 | **逐位一致** |
| `SHA256SUMS`（顶层） | 191 | `78ab2e6b9cc8213282fc21d9f09f95daa5526d206b164335462df60f8c86ee14` | 同 | **逐位一致** |

- **发布态**：`isDraft=false`／`isPrerelease=false`；`gh release list --limit 4` 显示 **Latest**；
  `publishedAt`＝2026-10-11T01:25:59Z（＝2026-10-11 HKT 09:25）；`targetCommitish=main`。
- **tag 形态**：`gh api repos/SilverWhite/orz/git/ref/tags/v0.9.0` ⇒ `object.type=commit`、
  `object.sha=4b1da9fbe988b0a9892af7c55fe406c86512938f`＝**轻量 tag**（沿 122／156／183／195／224／244
  形态：tag 落在本批提交上）；`git ls-remote --tags origin v0.9.0` 同值复核。
- **发行正文**：`--notes-file .tmp-b248-release-notes.md`；回读 body 长度 1,897 字符；
  标题＝`orz 0.9.0（.gsa 会话卫生：新对话新台账 · 上一轮整卷进存档 · 双平台包）`。

## §2 完整回下载复核（`identical` 对拍）

- 落点＝`D:\tb-eval\rel-248-verify`（新建目录，非打包暂存区），重下载后与本地逐件对拍：

| 件 | 回下载 size | 回下载 sha256 | 与本地 | 判定 |
|---|---:|---|---|---|
| zip | 28,436,427 | `f8716854…` | 相等 | **identical=True** |
| tar.gz | 37,697,953 | `8529e090…` | 相等 | **identical=True** |
| `SHA256SUMS` | 191 | `78ab2e6b…` | 相等 | **identical=True** |

- ⇒ 远端资产＝本地产物，**无上传截断／替换**（195／225／245 同判据）。
- **通道如实记**：首轮在非仓库目录（`D:\tb-eval\rel-248-verify`）直接调 `gh release download`
  报 `fatal: not a git repository`（gh 需仓库上下文解析 `-R`）——改由父仓目录加 `-R SilverWhite/orz`
  重跑后三件**一次跑完**，未复现 225 批的大文件停摆。

## §3 边界与如实记

1. **零改动**：本批未触碰仓库任何产物与载体；仅新增本档与台账行。
2. **Windows `--build-info` 复核**＝`0.9.0 os=windows` rc0（248 批，取包内件）；Linux 双冒烟
   `0.9.0 os=linux` rc0（247 批）＋248 批字节面（双载体 `0.9.0` 在场 18/6、`0.8.16` 与 `0.8.15`
   各 0 处）；本批不重跑容器（docker 守护进程未启动）。

## §4 台账与门禁

- 索引头行 v4.220 → **v4.221**；BACKLOG 计数行（59 不变）＋本批指针行；BACKLOG 第二卷 §1.195；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 关联与关键词

[`248 批档`](248_SUBMIT_PUSH_AND_RELEASE_V090_2026-10-11.md)（本补记母批）／
[`245 补记`](245_RELEASE_V0816_READBACK_VERIFICATION_2026-10-11.md)（同形态先例）／
[`225 补记`](225_RELEASE_V0815_READBACK_VERIFICATION_2026-10-07.md)。

关键词：249 补记、0.9.0 发行核证、服务端 digest 逐位一致、回下载 identical 3/3、Latest、
轻量 tag `4b1da9fb`、计数 59 不变、索引 v4.221。
