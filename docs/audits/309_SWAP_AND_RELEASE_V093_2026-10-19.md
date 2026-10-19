# 309 批：装机位换装 0.9.3＋GitHub Release `v0.9.3` 安装包发行（2026-10-19）

> **用户令**：「请进行换装吧，然后做新版本安装包发布」（281/290＋292 式换装＋发行批；
> 承接 308 重建批——评测载体位已双平台 0.9.3 在役）。**本批零源码、零 `orz/` 子树
> 改动、零跑批**；计数 60 不变。

## §1 装机位换装（`C:\Users\1\AppData\Local\orz\bin`）

| 项 | 结果 |
|---|---|
| 换装前件对账 | 在役三件＝0.9.2 `0e8ee3b7…`／`fc374dca…`／`216e80c8…`，与 289/290 批账面逐位吻合 |
| bak 链预建 | `.0.9.2-bak` ×3（与既有 `.0.9.0-bak`／`.0.9.1-bak` 链及 `orz.exe.bak-0925` 老件并存） |
| 三件换装 | 0.9.3 `orz.exe dabaaf7ea…`（58,533,888 B）／`orz-signer.exe b597cbb0…`／`orz-acaf-provision.exe fd7f92d5…`——与 308 载体位/`target/release` **同源逐位一致（MATCH 3/3）** |
| build-info | `version=0.9.3 os=windows arch=x86_64 profile=release` **rc0**（装机位读数） |
| ACAF provision | **不重 provision**——装机位无 `acaf` 面（266 口径沿 281/290）；signer 已随 308 载体位 provision 核证（同一构建） |

**生效面**：装机位（日常使用＋web 桥）自此 **0.9.3 在役**——0dh 长程进程显式通道
（`is_background`／跨 run 存活／30h／`get_task_output` 增量读出／`kill_task`）与 0di
消息撤回＋中途补充（`_orz/queue_append`／loop-top 投递／`/rewind`）自此在日常使用面
生效；0dh/0di S4 真机判据自此可由日常使用轮承载。

## §2 打包与核证（`D:/tb-eval/rel-309-stage`）

| 项 | 结果 |
|---|---|
| stage 布局 | `win/`＋`lin/` 各 **6 entries**（三件套＋`README.md`＋`SHA256SUMS`＋`carrier-manifest.json`）——281/292 逐式同构 |
| 包内 README | 0.9.3 版派生自 0.9.2 包内档：新增 **0.9.3 概要段与详解节**（0dh D1–D8＋0di＋0dj）＋Linux 解包名 0.9.3＋调优行补 `GROK_BG_MAX_RUNTIME_MS` 并 **10h→30h 兜底改写**＋已知边界补 0dh/0di 两句＋尾部版本信息行 `5fd4c42d`/2026-10-18 构建・2026-10-19 打包 |
| 侧 SHA256SUMS | 4 entries（三件套＋README；`hash *name` ASCII LF＝281 同构）；win README `4abf055c…`＝lin 逐位同件 |
| carrier-manifest | 双侧 `kind=orz-carrier-manifest version=0.9.3` **5 entries** 逐文件对账 OK |
| 资产 | `orz-0.9.3-windows-x86_64.zip` **28,360,494 B `170c23f1…`**／`orz-0.9.3-linux-x86_64.tar.gz` **37,887,053 B `dce90f47…`**／顶层 `SHA256SUMS` 2 entries |
| zip↔win/ | 解包清单 6 entries 与 stage **逐件一致**＋包内 SHA256SUMS 四件逐位 match |
| tar 权限形态 | `-rw-rw-rw-`＝281/292 包同构（README `chmod +x` 步骤承载，非缺陷） |
| 包核证（alpine 3.20） | 解压态 `sha256sum -c SHA256SUMS` **4/4 OK**＋`chmod +x` 后 `--build-info`＝`0.9.3 os=linux` rc0 |
| 包核证（bookworm-slim） | `chmod +x` 后 `--build-info`＝`0.9.3 os=linux` rc0 |
| 活体清单自检 | **未跑**（沿 281/292 口径：0.9.x 无 tty 容器进 TUI；清单正确性由结构性对账承载） |

## §3 发行面

- **tag**：`v0.9.3`（轻量 tag）→ 本批父仓提交；推送 `origin main`＋tag。
- **GitHub Release `v0.9.3`**：**Latest**；三资产＝Windows zip／Linux tar.gz／顶层
  `SHA256SUMS`；正文＝0.9.3 增量说明（0dh／0di／0dj）。
- **README 发布面对齐**：解包名 `orz-0.9.3-linux-x86_64.tar.gz`＋最新 release 链接 `v0.9.3`。
- **回读核证**：发行后 `gh release view` 资产 digest 对账＋回下载逐位比对（282 式），
  见 §4。

## §4 回读核证（发行后）

- `gh release view v0.9.3`：`isDraft=false`／`isPrerelease=false`／**Latest**；三资产
  服务端 digest ↔ 本地逐位一致；tag `v0.9.3` → 本批提交；`targetCommitish=main`。
- 回下载三件与 `rel-309-stage` 本地件逐字节比对一致。

## §5 启动网址远程连接核答（用户问询，随批如实记）

用户问「当前版本的 orz 能用启动网址做远程连接吗」——**不能直接**：

1. **环回硬门**：`orz web` 绑定面有安全门 `security::bind_addr_allowed`（0br S2 设计）
   ＝**只允许 loopback（127.0.0.1／::1）**，`--addr` 指定非环回地址在启动校验即报错；
   默认绑 `127.0.0.1:21487`（`orz-web/src/lib.rs DEFAULT_BIND`）。
2. **设计安全模型＝本机环回**：per-boot 256-bit 令牌（URL fragment 交付前端）＋每
   WS/API 路由令牌校验＋WS 升级 Origin 检查——面向「本机浏览器」形态，非远程暴露形态。
3. **正当远程路径**：SSH 隧道／端口转发（远程机 `127.0.0.1:21487` 转发到本地浏览器）
   ——令牌与 Origin 检查照常工作（浏览器侧 origin 仍为本地回环形态）。
4. **原生远程形态（LAN/公网直连）＝新需求**：需绑面放开＋认证强化（令牌面已备但
   环回门是硬前提），当前版本不支持；如需请立项（本批仅核答不动设计）。

## §6 台账

- 本档；仓库 `README.md` 发布面两处；索引头行 v4.280 → **v4.281**；BACKLOG 指针行/
  计数行＋TODO 计数行＋[`BACKLOG 第二卷 §1.248`](../BACKLOG_AND_PRIORITIES_2.md)。
- 零源码、零 `orz/` 子树改动（manifest 不重生成）；门禁 `valid: true`。
- 计数 60 不变。

## §7 关键词

309 批、装机位换装、0.9.2→0.9.3、MATCH 3/3、bak 链 `.0.9.2-bak`、不重 provision、
GitHub Release `v0.9.3`、rel-309-stage、`170c23f1…` zip、`dce90f47…` tar.gz、
carrier-manifest 5 entries、alpine 4/4 OK、双镜像 build-info 0.9.3、README 发布面对齐、
Latest、环回门核答、SSH 隧道正当路径、计数 60 不变、v4.281。
