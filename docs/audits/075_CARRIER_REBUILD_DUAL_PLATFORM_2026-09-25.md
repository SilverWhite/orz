# 0.6.13 双平台载体重建、换装与发行（2026-09-25）

> 用户令：「请进行提交推送与重建吧，0.6.13 也作为第一个 UI 版本推上 github」
> （前接 0br Web 形态 UI 落码的提交推送批父仓 `f5b84512`／orz `459f8d85`）。
> 本批四段：① **版本 bump**（0.6.12 → 0.6.13）② **双平台重建** ③ **换装＋字面量核证＋
> ACAF 与装配复核＋载体级 Web 探针** ④ **双平台打包 ＋ GitHub Release `v0.6.13`**。
> **源冻结线**：orz **`5998d4b1`**（`feat/fusion-architecture`）＝0br 落码 `459f8d85`
> ＋ bump 两文件两行。**上一载体**：双平台 0.6.12（已发行，`d69dab47` 冻结）。
> **本次发行的相对面＝已发布 0.6.12**。本批**免预检**（默认口径沿用）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.12 → **0.6.13**（orz `5998d4b1`，已推 `cli`） |
| 0br 进件 | **首个带 UI 的载体**：Web 工作台（桥 `orz-web`＋照搬 `98.css`／`XP.css` 前端）＋ S2 审查处置＋ S3 新增面（会话归档投影面四批）首次进入在役载体 |
| Windows 三件套 | `cargo clean`（18,223 文件／6.9 GiB）后 clean 全量 **29m06s** exit 0；**警告 0／错误 0**；换装逐件 MATCH=True |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_OK`（未切代理）；**暖缓存 6m10s** exit 0；三件 ET_DYN／x86-64／PT_INTERP=0；bookworm·alpine 双向冒烟 **6/6 exit 1** |
| 字面量核证 | **新面 8 项全部 0 → ≥1**（双侧逐位同值）；**保持面 35 项零回退** |
| 换装与 ACAF | 六件换装（`.0.6.12-bak` 链）＋ACAF 重 provision exit 0（keystore 四值逐位未动、manifest↔signer 逐位一致）＋**载体级 Web 探针**（令牌／Host 门／boot 面／归档 API 全过、零残留进程） |
| 发行 | 双平台包 ＋ 顶层 `SHA256SUMS` 暂存 `D:\tb-eval\rel-075-stage\`，并入 **GitHub Release `v0.6.13`**（见 §8） |
| 摩擦 | 前批 UI 走查遗留的两个 `orz` dev server 进程占用 `target\debug\orz.exe` 致 `cargo clean` 拒绝访问（§5.1，仅记录） |

## 1. 前置基线（0.6.12 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 54,896,640 | `3492768882C1EF11ACF231675830681DF080E2B5178573D7B484B92AC1FC19BA` |
| Windows | `orz-signer.exe` | 6,742,528 | `BC6ACB9913012D9CC927B9089B02CC010E25B65011E19F65AE2A668A52E6E9DE` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `E93E8D85DE0EAC245310D930E33C66DAF2509AA51A5E32C7C091A5D3A4AC446A` |
| Linux | `orz` | 112,430,240 | `CC7718442639DBEA4D0C418EE285FF43FDF42E61E31E249FF7B509D03E403E32` |
| Linux | `orz-signer` | 1,401,800 | `2B6B5BA1369865F3FC2B7728A75454678BB60882C7F32A501508A6C94128410F` |
| Linux | `orz-acaf-provision` | 1,220,536 | `EA55BAFB0B22CB85A9A16C730E0D4571A9519AC9EB98556B6FA2ACC63B91C405` |

六件与 [`074 档 §4/§5`](074_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-24.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.12-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——六件全部就位。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.12 → 0.6.13`
  （orz 提交 **`5998d4b1`**；`cargo metadata --locked` exit 0 ⇒ 锁文件自洽）。
- 推送：`459f8d85..5998d4b1  feat/fusion-architecture -> feat/fusion-architecture`。
- 冻结线内容＝0br 落码批 `459f8d85`（38 项：新增 crate `orz-web` 31 件＋改动 7 件，
  ＋5,740 行前端与桥面／`orz-host` 归档面 +431 行）＋ bump；**Web 工作台与归档投影面
  首次进入在役载体**（0.6.12 只到 0bi／0bl／0bk／0bn 四批）。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、或用户
  明确点名时才做。本批为 Web 桥／前端资产嵌入＋归档面＋打包发行，**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理 15.8 GiB
  （余 4.5）／commit 25.8 GiB（余 4.4，使用 83%）** ⇒ 判 **`-j 2`**，并触发 0bh ⑥ 的
  **余量软提示**）。
- `cargo clean`：**18,223 文件／6.9 GiB**（0.6.12 批曾清出 63,779 文件／37.0 GiB；本批前
  0br S3 批已清 `target/debug/incremental` 17 GB，故可清量显著小）；构建 **exit 0**，
  `Finished release` **29m06s**；**警告 0 条、错误 0 条**。

| 文件 | 尺寸 (B) | 与 0.6.12 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,427,520 | **+1,530,880 B** | `83BBCE8D2B491AF79C72EC59A5A6D9BA70354D027C5EF8DA3EEB5FB11986A7F4` |
| `orz-signer.exe` | 6,742,528 | ±0 | `05F3E83FB158DF30ED757DFD208D79B42FAFCD4CB8EF7F7EA0B4DC457AB3DBCE` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `9B0B31F1661F7CA4A0E88E22AE57ABF73A58E78313E5DC714B2568BE312492BD` |

> `orz.exe` +1.53 MB 与内嵌 Web 前端资产量级相称（`assets/` 共 31 件含 98.css／XP.css／
> marked／四个字体文件）；`signer`／`provision` **无源码改动**（版本串为唯一差异来源），
> 尺寸不变、哈希变动属重编译非确定性——与 068–074「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` → **APT_OK**
  ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**。
- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001
  契约），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`（暖缓存）
  ＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.12-bak`）；
  **exit 0**：容器内 `Finished release` **6m10s**（日志 `.tmp-b075-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.12 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 114,668,656 | **+2,238,416 B** | `b9bc348911fc6d0df717a1076088ac83cf8dbfbeee5ddda1eb113cb76028b1dd` |
| `orz-signer` | 1,401,784 | −16 B | `50ccc6087b83a5ca83eda79d97ac90a692a96382ecdc87a2f402a9d735efa3de` |
| `orz-acaf-provision` | 1,220,528 | −8 B | `fd4e9becb1a1e6058f0a6161dd46b4bc6190b296860e87b6a4cd78d3d048f7c5` |

- ELF 静态核验（逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋ `e_machine=62`（x86-64）
  ＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–074 同形态），
  **6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.12 同数**（三处均为非 Windows 分支的 unused 项，既有基线）；**`orz-web` 0 警告**。

### 5.1 本批摩擦：遗留 dev server 进程占用构建产物（仅记录）

- **现象**：首轮 `-Release -Clean` 在 `cargo clean` 阶段
  `error: failed to remove file D:\CLI\orz\target\debug\orz.exe … 拒绝访问 (os error 5)`，
  exit 1（清到一半即止，源树无损伤）。
- **根因**：前批 0br S3 浏览器走查拉起的两个 `orz web` dev server（PID 10916／17112，
  路径均 `D:\CLI\orz\target\debug\orz.exe`）仍在运行、持有该文件句柄——S3 档 §4 记「探针进程
  已清理」只覆盖当批探针，走查期间另起的实例未随档清点。
- **处置**：按路径精确识别（`Get-Process | Where-Object { $_.Path -like 'D:\CLI\orz\target\*' }`）
  → 逐个 `Stop-Process` → 复核零残留 → clean 重跑一次通过。
- **口径**：**仅记录，不立项**。建议并入 0bq（进程收口兜底）邻域——真机走查起停的 dev server
  属装置面残留，其占位会静默阻断 clean 重建；后续走查批在收尾时按路径清点一次即可。
- 其余装置面：Docker Desktop 本批正常（未复现 074 的 `HCS_E_CONNECTION_TIMEOUT`）。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.12 备份件）

脚本 [`.tmp-b075-literals.py`](../../.tmp-b075-literals.py)（可复跑，本批 exit 0）。
**新面（0 → ≥1，双侧同值）**：

| 字面量 | Win 0.6.12 | Win 0.6.13 | Lin 0.6.12 | Lin 0.6.13 | 判读 |
|---|---:|---:|---:|---:|---|
| `/api/archives` | 0 | **6** | 0 | **6** | 0br S3 归档桥路由**进件** |
| `trusted_workspaces` | 0 | **2** | 0 | **3** | 0br 批二 信任清单投影**进件** |
| `archive://` | 0 | **6** | 0 | **6** | 0br S3 归档 URI 方案**进件** |
| `reconstructed_from_journal` | 0 | **3** | 0 | **3** | 0br 批三 journal 重构归档标记**进件** |
| `用法: orz archive <session8>` | 0 | **1** | 0 | **1** | 0br 批二 `orz archive` 子命令**进件** |
| `归档于 ` | 0 | **1** | 0 | **1** | 0br S3 归档事实行**进件** |
| `转写超上限已截断（` | 0 | **1** | 0 | **1** | 0br S3 归档转写截断告知**进件** |
| `已信任工作区` | 0 | **5** | 0 | **5** | 0br 批二 已信任工作区子级**进件** |

**保持面 35 项零回退**（逐项双侧同值，不回退、不减损）：

- 074 批新面 9 项：`[emoji 剥离]` 1／`ORZ_WRITE_KEEP_EMOJI` 1／`Optional line-window anchor
  edit` 1／`appears to be UTF-16 encoded` 1／`当前可压区间` 1／
  `context_scale:block_selection_unrecognized` 7／`context_scale:mandatory_summary_not_landed` 7／
  `至少命中 2 个小节` 1／`superseded_by_mandatory_window` 7。
- 071 批新面 14 项：`Call-scope lifecycle` 3／`[编码降级告知]` 3／`目标档：≤` 1／
  `[黑板说明书 guide v1 · digest sha256:` 1／`r<轮>·b<块>·s<seq>` 27／`…(+` 1／
  `条暂存` 1／`[RUN_END` 1／`[/RUN_END]` 1／`await_channel` 5／
  `lines truncated; narrow the pattern` 1／`Text files only: when true, return a section outline` 1／
  `No section headings found in this ` 1／`more headings omitted` 1。
- 更早批次面 12 项：`RLI提醒:` 1／`持续越线` 1／`域迁移确认` 1／`掩盖缺口` 1／`CoverageGap`
  1→2（**本批 +1**：`orz-web` 的信任/归档面文案同现，属新增命中、非回退）／
  `lif.domain_migration` 7／`分通道明细 selector=channels` 1／面头符号表 1／
  `本会话负担总值 ` 3／`资源软提示` 2／`utf-8-lossy` 1／`反例` 33→34（**本批 +1**，同因）。

> **方法边界（第三例，与 071／074 同族）**：0bn② 的「`（见上）` 指称退役」对象是强制压缩
> 窗口块；H1 阶梯窗口块的**同消息内指称**仍在，故全局字节计数恒为 `1 → 1`。字节级扫描是
> 进件核证的**下界**、不能分块隔离——本条**非缺陷**，登记备查。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.12-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260925-075`。
- ACAF 重 provision（`orz-acaf-provision.exe acaf\keystore acaf\signer-manifest.json`，**exit 0**）：
  keystore **逐位未动**（外层 `installation-key.dpapi` `2408F596…`／`installation-key.json`
  `C491FC95…`；`keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=05f3e83fb158df30ed757dfd208d79b42fafcd4cb8ef7f7ea0b4dc457ab3dbce`
  ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- **载体级 Web 探针（本批新增面；脚本 [`.tmp-b075-win-web-probe.ps1`](../../.tmp-b075-win-web-probe.ps1)）**：
  `orz.exe web --addr 127.0.0.1:21555` 拉起换装位二进制 →
  启动打印 64 位令牌 URL；`/`（静态）**200**（静态资产按设计免令牌）；`/api/boot` 无令牌
  **401**；伪造 Host **401**；带令牌 **200**，boot 键＝`cwd,agent,trusted_workspaces`
  （批二信任投影在役）；`/api/archives` 带令牌 **200** → **19 件**（本机真实归档数）；
  探针后进程零残留（`leftover_orz_procs=0`）。
  ⇒ **首个 UI 载体在真机可起、门面与新增 API 面均在役。**
- Linux：`/out` 直写即换装位（换装前 `.0.6.12-bak` 六件齐备）；ELF／冒烟读数见 §5。

## 8. 双平台打包与 GitHub Release

- 暂存 `D:\tb-eval\rel-075-stage\`（本地件不入库），入口脚本
  [`.tmp-b075-package.ps1`](../../.tmp-b075-package.ps1)（沿 066／074 形态）：

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.13-windows-x86_64.zip` | 27,713,322 | `1230585986a25afcd38f3e955a0990f567c3dee164aa97a5b0027791d156f097` |
| `orz-0.6.13-linux-x86_64.tar.gz` | 36,814,163 | `cbffd6587c86d3e1c066ca441cfb93bf5b9fed015203ba576e4244a415afd279` |

- **包内容**＝三件二进制＋`README.md`（0.6.13 更新说明：Web 工作台＝形态与复用／工作台能力／
  安全与边界，并保留 0.6.12 及更早的面＋两平台快速开始＋完整性校验）＋`SHA256SUMS`
  （`hash *name`、LF 行尾）。
- 包完整性：解包回读逐件与换装载体 **同源同哈希（6/6 MATCH）**；容器内
  `sha256sum -c SHA256SUMS` **全 OK**（zip 侧 4/4、tar 侧 4/4，`alpine:3.20`）。
- **tar 权限形态（沿用口径）**：包内条目 mode 为 `-rw-rw-rw-`（Windows bsdtar 打包固有），
  与已发布的 0.6.6／0.6.12 tar **同形**；README 的 Linux 快速开始已含
  `chmod +x orz orz-signer orz-acaf-provision`，非本批引入。
- Release：父仓 tag **`v0.6.13`**（指向本批提交）＋双资产（双包），说明文本＝源冻结基线、
  六件二进制哈希表、**0.6.13＝首个 UI 载体**的更新说明（相对已发布 0.6.12）与验证摘要。
- 上传面核证：`gh api repos/SilverWhite/CLI/releases/tags/v0.6.13` 回读两资产 **`digest` 字段**
  （见 §8.1 补记）。

### 8.1 上传面核证补记（本档提交后回读）

- **创建读数**：`gh release create v0.6.13 --target a3df39ed…` ⇒
  <https://github.com/SilverWhite/CLI/releases/tag/v0.6.13>；
  `gh api repos/SilverWhite/CLI/git/ref/tags/v0.6.13` 回读
  `object.sha = a3df39ede3f606f7ea447c454c21ab4ab0963756` ⇒ **tag 指向本批父仓提交**。
- **资产 digest 回读**（`gh api repos/SilverWhite/CLI/releases/tags/v0.6.13`）：

| 资产 | 远端 size | 远端 digest | 本地 size／SHA256 | 判读 |
|---|---:|---|---|---|
| `orz-0.6.13-windows-x86_64.zip` | 27,713,322 | `sha256:1230585986a25afcd38f3e955a0990f567c3dee164aa97a5b0027791d156f097` | 27,713,322／同值 | **逐位一致** |
| `orz-0.6.13-linux-x86_64.tar.gz` | 36,814,163 | `sha256:cbffd6587c86d3e1c066ca441cfb93bf5b9fed015203ba576e4244a415afd279` | 36,814,163／同值 | **逐位一致** |
| `SHA256SUMS`（顶层） | 193 | `sha256:b94ff2f74c15014828a2fcfe5928ccbefdbe05d60a32cbd67ea02be84ebfe44e` | 193／同值 | **逐位一致** |

- **回下载核验（本批达成，074 未竟项收口）**：三资产经**认证 API 资产面**完整回下载——
  `gh api repos/SilverWhite/CLI/releases/assets/{id} -H "Accept: application/octet-stream"`：
  zip 回读 27,713,322 B（SHA256 与本地**逐位一致**）、tar.gz 回读 36,814,163 B（**同值**）、
  `SHA256SUMS` 回读 193 B（**同值**，其内两行与本地包哈希逐行相符）。
- **074 摩擦根因补记（本条为口径澄清，非新摩擦）**：本仓为**私有仓**（`gh repo view` ⇒
  `isPrivate: true`）⇒ 匿名路径 `https://github.com/…/releases/download/vX/…`
  一律返回 **404**（回读体 `Not Found`，9 B），这解释了 074 §8.1「`gh release download`
  两次各创建 **0 字节**文件后阻塞」的现象族——**属私有面＋本机网络面的组合，非上传失败**；
  资产面核证应以**认证 API 路径**为准（`releases/tags/{tag}` 的 `digest` 字段＋
  `releases/assets/{id}` 下载回读）。

## 9. 记账面（pin、清单、索引、计数）

- orz：**`5998d4b1`**（bump；已推 `cli` 远端）。
- 父仓：本档 ＋ `orz_source_manifest.sha256` **不变**（1497 条；子模块 pin 在提交批已指向
  `5998d4b1`，本批无子模块内容变化——差异应恰为本档＋README＋索引＋TODO／BACKLOG）
  ＋ README 发布面对齐 **v0.6.13** ＋ TODO／BACKLOG 的 **0br「进载体 0.6.13」标注**
  ＋ 索引 v4.40 → **v4.41**（v4.40 头行滚入 `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`）。
- **计数不变**（**未闭合 54**）：本批为重建与发行，不新增／不闭合开放项。
- 门禁 `python scripts/check_repository.py`：提交后复跑须 `valid: true`（见 §10）。

## 10. 边界与未做项

1. **未跑真机狗粮轮**：本批只做到「0br 进件＋字面量可核＋载体级 Web 探针＋双平台发行」；
   0bm（狗粮长轮）与其同轮的 0bj／0bp／0bq，以及 0bk／0bn 的 **S3 真机读数**、0bi 的
   S3 未读三项（journal anchor 三态／`run_finished` 新字段／投递计数）与 ⑪ 告知行首读，
   均留待下一轮真机。
2. **0br 未竟面**：S3 真机首读（三稿区块逐项对照、IME／刷新／长会话渲染读数）与 S4 收口
   （判据入账、TUI 后补排期）仍未开工；本批只把 Web UI 送进在役载体。
3. **未做双平台预检**（默认口径；§3）。
4. **发行面只发双平台包**：不发行 `releases/` 目录式试用包（历史形态），不动既有 tag。
5. **已知观察项（2026-09-25 用户令并入 0bm）**：`orz web` 启动输出在 GBK 控制台显示乱码（UTF-8 字节被 GBK 控制台误解）——用户令「这个问题也并入 0bm 吧」⇒ **已并入 0bm 本轮任务**（归 0bj ④ 输出编码链一族，见 BACKLOG／TODO 0bm「本轮同载四件」；本批只作证据留档〔探针日志见 §7〕，不在本批修）；本机 `D:` 余量构建后约 20 GB 量级（构建产物与两个缓存目录为主）。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b075-windows-build.log`／`.tmp-b075-linux-build.log`／`.tmp-b075-literals.py`／
`.tmp-b075-package.ps1`／`.tmp-b075-win-web-probe.ps1`／`.tmp-b075-index.py`／
`.tmp-b075-ledger.py`／`.tmp-b075-ledger2.py`／`.tmp-b075-readme.py`／
`.tmp-b075-release-notes.md`／`.tmp-b075-win-web-probe.log`；
**回下载核验副本** `.tmp-b075-dl-SHA256SUMS`／`.tmp-b075-dl-SHA256SUMS-api`／
`.tmp-b075-dl-orz-0.6.13-windows-x86_64.zip`／`.tmp-b075-dl-orz-0.6.13-linux-x86_64.tar.gz`
（认证 API 回读件，§8.1；删除动作被本机策略拦截，**保留作证据**）；
两平台 `.0.6.12-bak` 备份链与 `signer-manifest.json.bak-20260925-075` 保留；
发布包暂存 `D:\tb-eval\rel-075-stage\`（含 `_verify_zip`／`_verify_tar` 回读目录）。

## 12. 关联与关键词

[`074 重建档`](074_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-24.md) ／
[`0br S3 归档投影档`](0BR_S3_ARCHIVE_PROJECTION_2026-09-25.md) ／
[`0br S2 审查处置档`](0BR_S2_REVIEW_HANDLING_2026-09-25.md) ／
[`综合稿`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) ／
[`ADR-0010 §14.78`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.6.13、首个 UI 载体、Web 工作台进件、`orz web`、98.css、XP.css、归档投影面、
trusted_workspaces、字面量核证、载体级 Web 探针、ACAF 重 provision、PT_INTERP=0、
dev server 残留占位、GitHub Release v0.6.13。
