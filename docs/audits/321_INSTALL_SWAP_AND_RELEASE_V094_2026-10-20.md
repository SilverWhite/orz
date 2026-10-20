# 321 批：0.9.4 装机位换装＋GitHub Release `v0.9.4` 安装包发行（2026-10-20）

> **用户令**：「请推送0.9.4的安装包吧」（沿 281/290/292/309 式换装＋发行批；承接 320 重建批——
> 评测载体位已双平台 0.9.4 在役）。**本批零源码语义改动、零 `orz/` 子树改动、零跑批**；
> 计数 62 不变。

## §1 装机位换装（`C:\Users\1\AppData\Local\orz\bin`）

| 项 | 结果 |
|---|---|
| 换装前件对账 | `orz-signer.exe`＝0.9.3 `b597cbb0…`、`orz-acaf-provision.exe`＝0.9.3 `fd7f92d5…`（与 308/309 账面一致）；**`orz.exe` 为 45,068,800 B `711465b9…`（mtime 2026-10-19 20:04:45，`--build-info` `version=0.9.3 os=windows … profile=release`）——不是 309 发行的 `dabaaf7e…`（58,533,888 B）**，系 10-19 晚本地构建漂移件（来源未记录，如实记） |
| bak 链预建 | 漂移件留 `orz.exe.pre-0.9.4-bak`；`.0.9.3-bak` 三件齐（`orz.exe` 既有件即真 0.9.3，signer/provision 本批补建） |
| 三件换装 | 0.9.4 `orz.exe f0bb5d92…`（58,703,360 B）／`orz-signer.exe c7f5aa2f…`／`orz-acaf-provision.exe 478fcce7…`——与 320 载体位 **同源逐位一致（MATCH 3/3）** |
| build-info | `version=0.9.4 os=windows arch=x86_64 profile=release` **rc0**（装机位读数） |
| ACAF provision | **不重 provision**——装机位无 `acaf` 面（266 口径沿 281/290/309）；signer 已随 320 载体位 provision 核证（同一构建） |

**生效面**：装机位（日常使用＋web 桥）自此 **0.9.4 在役**——319 的安全保底、维护令牌门、
会话 lease/`session/load` 回放、补充预算与 Web `/rewind` 等修复自此在日常使用面生效。

## §2 打包与核证（`D:/tb-eval/rel-320-stage`）

| 项 | 结果 |
|---|---|
| stage 布局 | `win/`＋`lin/` 各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）——309 同构 |
| 包内 README | 0.9.4 版派生自 0.9.3 包内档（87,011 B，CRLF）：新增 **0.9.4 概要段与更新节**（安全保底／会话生命周期／中途补充与撤回／Web 工作台／构建与门禁）＋Linux 解包名 0.9.4＋已知边界增 0.9.4 残余两句＋尾部版本信息 `5ee818d3`／2026-10-19 构建・2026-10-20 打包 |
| 侧 SHA256SUMS | 4 entries（三件套＋README；`hash *name` ASCII LF＝309 同构）；win README `61b9d332…`＝lin 逐位同件 |
| carrier-manifest | 双侧 `kind=orz-carrier-manifest version=0.9.4` **5 entries** 逐文件对账 OK |
| 资产 | `orz-0.9.4-windows-x86_64.zip` **28,557,864 B `2a9a747e…`**／`orz-0.9.4-linux-x86_64.tar.gz` **37,866,976 B `855bd1ec…`**／顶层 `SHA256SUMS` **191 B**（2 entries） |
| zip↔win/ | 解包清单 6 entries 与 stage 逐件一致＋包内 SHA256SUMS 四件逐位 match（4/4）＋`--build-info` 0.9.4 rc0 |
| tar 权限形态 | `-rw-rw-rw-`＝308/309 包同构（README `chmod +x` 步骤承载，非缺陷） |
| 包核证（alpine 3.20） | 解压态 `sha256sum -c SHA256SUMS` **4/4 OK**＋`chmod +x` 后 `--build-info`＝`0.9.4 os=linux` rc0 |
| 包核证（bookworm-slim） | 同上 **4/4 OK**＋`0.9.4 os=linux` rc0 |
| 活体清单自检 | **未跑**（沿 281/292/309 口径：0.9.x 无 tty 容器进 TUI；清单正确性由结构性对账承载） |

## §3 发行面

- **tag**：`v0.9.4`（轻量 tag）→ 本批父仓提交；推送 `origin main`＋tag。
- **GitHub Release `v0.9.4`**：**Latest**；三资产＝Windows zip／Linux tar.gz／顶层
  `SHA256SUMS`；正文＝0.9.4 增量说明（318/319 修复）。
- **README 发布面对齐**：中文 README 解包名 `orz-0.9.4-linux-x86_64.tar.gz`＋最新 release
  链接 `v0.9.4`；英文 README 同一面对齐（`v0.9.4 face, 2026-10-20`）。
- **回读核证**：发行后 `gh release view` 资产 digest 对账＋回下载逐位比对（282/309 式），
  见 §4。

## §4 回读核证（发行后补记）

（待回读：`gh release view` 状态／tag→提交／三资产 digest↔本地／回下载逐位比对。）

## §5 台账

- 本档；仓库 `README.md` 发布面两处＋`docs/en/README.en.md` 发布面三处。
- 索引头行 v4.291 → **v4.292**；BACKLOG 本批指针行＋计数行（62 不变）；TODO 计数行；
  [`BACKLOG 第二卷 §1.258`](../BACKLOG_AND_PRIORITIES_2.md)。
- 零源码、零 `orz/` 子树改动（manifest 不重生成——父仓 README 不在 `orz_source_manifest`
  覆盖面）；门禁 `valid: true`。
- 计数 62 不变。

## §6 边界与如实记

1. **装机位漂移已在案**：换装前 `orz.exe` 非 309 发行件（45,068,800 B `711465b9…`，10-19
   20:04 本地构建），已留 `orz.exe.pre-0.9.4-bak`；`.0.9.3-bak` 保留真 0.9.3 以便回溯。
   漂移来源未记录，不追改历史档。
2. **S4 未跑**：0di/0dk/0dl 仍 `pending`；本批只证「0.9.4 可安装、可运行、可下载」。
3. **未做安装包内活体清单自检**（无 tty 容器进 TUI）——沿 309 同口径，清单由结构性对账承载。
4. **代理波动**：git/gh 若遇 TLS 瞬断，按 309 先例改直连（`-c http.proxy=`）重试，环境面不入账。

## §7 关键词

321 批、装机位换装、0.9.3→0.9.4、MATCH 3/3、`orz.exe.pre-0.9.4-bak`、装机位漂移 `711465b9…`、
GitHub Release `v0.9.4`、rel-320-stage、`2a9a747e…` zip、`855bd1ec…` tar.gz、
carrier-manifest 5 entries、alpine/bookworm 4/4 OK、双镜像 build-info 0.9.4、README 发布面
对齐、Latest、计数 62 不变、v4.292。
