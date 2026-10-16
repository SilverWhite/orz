# 282 补记：0.9.1 发行回读核证（服务端 digest 逐位一致＋回下载 identical 3/3）（2026-10-16）

> **日期**：2026-10-16；**性质**：**发行回读核证批**（零源码、零跑批、零改动）——
> [`281 批`](281_SUBMIT_PUSH_SWAP_AND_RELEASE_V091_2026-10-16.md) §4「服务端回读与完整回下载复核
> 随后续补记」的兑现；未闭合计数不变 **61**。
> **结论先行**：`v0.9.1` 三资产的**服务端 digest 与本地产物逐位一致**；`gh release download`
> 完整回下载**三件与本地 identical 3/3**（逐字节相等）；release 为 **Latest**，tag 为**轻量 tag**
> ＝父仓 `584f49dd`（281 批本档所在提交）。

---

## §1 服务端回读（`gh release view --json`）

| 资产 | 服务端 size | 服务端 digest（sha256） | 本地 | 判定 |
|---|---:|---|---|---|
| `orz-0.9.1-windows-x86_64.zip` | 28,441,423 | `9bc5c7d4518f513d5b1d8b6bc933c51d597382490988380bc003195a64984398` | 同 | **逐位一致** |
| `orz-0.9.1-linux-x86_64.tar.gz` | 37,707,315 | `07f67e30a4ac28b1f83dd44f16e4c40de4bad7b5b515cd61d1fcd10c8d0b625a` | 同 | **逐位一致** |
| `SHA256SUMS`（顶层） | 191 | `a883300123ecd9c821ea19b7107cde235794cb81ea10e3c2ca9274d7d6f77173` | 同 | **逐位一致** |

- **发布态**：`isDraft=false`／`isPrerelease=false`；`gh release list --limit 4` 显示 **Latest**；
  `publishedAt`＝2026-10-16T12:58:57Z（＝2026-10-16 HKT 20:58）；`targetCommitish=main`。
- **tag 形态**：`gh api repos/SilverWhite/orz/git/ref/tags/v0.9.1` ⇒ `object.type=commit`、
  `object.sha=584f49dd231b784ab926b827e1b434bed19b0caf`＝**轻量 tag**（沿 122／156／183／195／224／
  244／248 形态：tag 落在本批提交上）；`git ls-remote --tags origin v0.9.1` 同值复核。
- **发行正文**：`--notes-file .tmp-b281-release-notes.md`；回读 body 长度 1,969 字符；
  标题＝`orz 0.9.1（会话生命周期用户主权：run 末自动归档退役 · 上下文治理摩擦综合四件 · 双平台包）`。

## §2 完整回下载复核（`identical` 对拍）

- 落点＝`D:\tb-eval\rel-281-verify`（新建目录，非打包暂存区）；命令＝
  `gh release download v0.9.1 -R SilverWhite/orz -D <dir>`（在父仓目录内执行）；重下载后与本地逐件对拍：

| 件 | 回下载 size | 回下载 sha256 | 与本地 | 判定 |
|---|---:|---|---|---|
| zip | 28,441,423 | `9bc5c7d4…` | 相等 | **identical=True** |
| tar.gz | 37,707,315 | `07f67e30…` | 相等 | **identical=True** |
| `SHA256SUMS` | 191 | `a8833001…` | 相等 | **identical=True** |

- ⇒ 远端资产＝本地产物，**无上传截断／替换**（195／225／245／249 同判据）。
- **通道如实记**：本次在父仓目录内加 `-R SilverWhite/orz` 直跑，**三件一次跑完**（未复现 225 批的
  大文件长时间停摆；249 批首轮「非仓库目录直接调 `gh` 报 `fatal: not a git repository`」情形亦未复现——
  该教训已内化为本批执行口径）。两大件下载耗时以分钟计，判据以「服务端 digest＋回下载」两路互证为据。

## §3 边界与如实记

1. **零改动**：本批未触碰仓库任何产物与载体；仅新增本档与台账行。
2. **双平台版本面复核已在 281 批完成**：包内 WIN `--build-info`＝`0.9.1 os=windows` rc0；
   LIN 于 `alpine:3.20` `sha256sum -c` **4/4 OK**＋`static-pie linked`＋alpine／bookworm 双镜像
   `--build-info`＝`0.9.1 os=linux` rc0；本批**不重跑容器**。
3. **装机位**：281 批已完成 `C:\Users\1\AppData\Local\orz\bin` 三件套 0.9.1 换装并留痕；
   本批不触碰装机位，不发起任何会话/run。
4. **0dd／0de S4 真机复验仍待续**（用户真机轮），本批不推进该面。

## §4 台账与门禁

- 索引头行 v4.254 → **v4.255**；BACKLOG 计数行（61 不变）＋本批指针行；BACKLOG 第二卷 §1.219；
  TODO 计数行。
- 门禁 `scripts/check_repository.py` ⇒ `valid: true`（本批落账后重跑）。

## §5 关联与关键词

[`281 批档`](281_SUBMIT_PUSH_SWAP_AND_RELEASE_V091_2026-10-16.md)（本补记母批）／
[`249 补记`](249_RELEASE_V090_READBACK_VERIFICATION_2026-10-11.md)（同形态先例）／
[`245 补记`](245_RELEASE_V0816_READBACK_VERIFICATION_2026-10-11.md)。

关键词：282 补记、0.9.1 发行核证、服务端 digest 逐位一致、回下载 identical 3/3、Latest、
轻量 tag `584f49dd`、`rel-281-verify`、计数 61 不变、索引 v4.255。
