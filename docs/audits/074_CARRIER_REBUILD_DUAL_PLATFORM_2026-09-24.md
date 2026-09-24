# 0.6.12 双平台载体重建、换装与发布（2026-09-24）

> 用户令：「请进行重建吧，本次重建的双平台包发布上去」（前接 0bi／0bl／0bk／0bn 四批
> 落码的提交推送批 `aeffea2b`）。本批四段：① **版本 bump**（0.6.11 → 0.6.12）
> ② **双平台重建** ③ **换装＋字面量核证＋ACAF 与装配复核** ④ **双平台打包 ＋
> GitHub Release `v0.6.12`**。
> **源冻结线**：orz **`d69dab47`**（`feat/fusion-architecture`）＝四批落码批 `4f28b83a`
> ＋ bump 两文件两行。**上一载体**：双平台 0.6.11（`917fadfb` 冻结，**中间过渡版、未发行**）。
> **本次发行的相对面＝已发布 0.6.6**（0.6.7–0.6.11 全为中间过渡版）。本批**免预检**（默认口径沿用）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.11 → **0.6.12**（orz `d69dab47`，已推 `cli`） |
| 四批落码进件 | **0bi 四件**（BOM 保真／anchor 行窗／反例门收窄／emoji 剥离）＋**0bl 十件**（全仓审查修复）＋**0bk S2**（压缩区间解析）＋**0bn S2**（必定压缩复审补口）**首次进入在役载体** |
| Windows 三件套 | `cargo clean`（63,779 文件／**37.0 GiB**）后 clean 全量 **25m56s** exit 0；**警告 0／错误 0**；换装逐件 MATCH=True |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_OK`（未切代理）；**暖缓存 10m00s** exit 0；三件 ET_DYN／x86-64／PT_INTERP=0；bookworm·alpine 双向冒烟 **6/6 exit 1** |
| 字面量核证 | **新面 9 项全部 0 → ≥1**（双侧逐位同值）；**保持面 26 项零回退**；退役面见 §6 方法边界注 |
| 换装与 ACAF | 六件换装（`.0.6.11-bak` 链）＋ACAF 重 provision exit 0（keystore 四值逐位未动、manifest↔signer 逐位一致）＋`dogfood_launch -DryRun` 断言全过 |
| 发行 | 双平台包 ＋ 顶层 `SHA256SUMS` 暂存 `D:\tb-eval\rel-074-stage\`，并入 **GitHub Release `v0.6.12`**（见 §8） |
| 摩擦 | **Docker Desktop 引擎停摆**（`HCS_E_CONNECTION_TIMEOUT`，本批新增一条，见 §5.1）——与 052／054 的代理摩擦**不同族** |

## 1. 前置基线（0.6.11 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 54,768,640 | `75EAA6E26A2CC37EC8EDADB865B65FE4E49F87652ED2B378DE553DE4FB44B5D7` |
| Windows | `orz-signer.exe` | 6,742,528 | `20F8B0D3581FC3F8842273C73CEEE6C47A26548C635A606B30B1A511FBCEB2BA` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `A802517844A5429A523A7907B3E6BF46D7F2FE41F8B6EFA98CBC4460D1567FD0` |
| Linux | `orz` | 112,300,264 | `7BA2E5CE44509A481B018436001DC6E211E0F891D6579225E1AB4DDD2EC50480` |
| Linux | `orz-signer` | 1,401,880 | `73413F27053D26E60CA9D6C0D0954D9E16589E3A3357F8B89C21B248E0651EAF` |
| Linux | `orz-acaf-provision` | 1,220,568 | `EEF4AE2C50D857675B11C23878396491109AA7104DADDD8D3A72979E135DAC87` |

六件与 [`071 档 §4/§5`](071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.11-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——六件全部就位。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.11 → 0.6.12`
  （orz 提交 **`d69dab47`**；`cargo metadata --locked` exit 0 ⇒ 锁文件自洽）。
- 推送：`4f28b83a..d69dab47  feat/fusion-architecture -> feat/fusion-architecture`。
- 冻结线内容＝四批落码批 `4f28b83a`（27 文件，＋3190/−238）＋ bump；**0bi／0bl／0bk／0bn
  的模型面与工具面首次进入在役载体**（0.6.11 只带 0bh 十三件）。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、或用户
  明确点名时才做。本批为编辑面／压缩面／取消面＋打包发行，**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理 15.8 GiB
  （余 4.3）／commit 29.8 GiB（余 4.1，使用 86.1%）** ⇒ 判 **`-j 2`**，并触发 0bh ⑥ 的
  **余量软提示**）。
- `cargo clean`：**63,779 文件／37.0 GiB**（D: 余量 4.2 GiB → 构建期 34.6 GiB）；构建
  **exit 0**，`Finished release` **25m56s**；**警告 0 条、错误 0 条**。

| 文件 | 尺寸 (B) | 与 0.6.11 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 54,896,640 | +128,000 B | `3492768882C1EF11ACF231675830681DF080E2B5178573D7B484B92AC1FC19BA` |
| `orz-signer.exe` | 6,742,528 | ±0 | `BC6ACB9913012D9CC927B9089B02CC010E25B65011E19F65AE2A668A52E6E9DE` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `E93E8D85DE0EAC245310D930E33C66DAF2509AA51A5E32C7C091A5D3A4AC446A` |

> `orz.exe` +128 KB 与四批进件量级相称（编辑面写入路径＋emoji 剥离表＋压缩面告知块＋
> 取消臂）；`signer`／`provision` **无源码改动**（版本串为唯一差异来源），尺寸不变、
> 哈希变动属重编译非确定性——与 068–071「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` → **APT_OK**
  ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**。
- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001
  契约），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/orz-target:/target`（暖缓存）
  ＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，换装前已留 `.0.6.11-bak`）；
  **exit 0**：容器内 `Finished release` **10m00s**（日志 `.tmp-b074-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 为空。

| 文件 | 尺寸 (B) | 与 0.6.11 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 112,430,240 | +129,976 B | `cc7718442639dbea4d0c418ee285ff43fdf42e61e31e249ff7b509d03e403e32` |
| `orz-signer` | 1,401,800 | −80 B | `2b6b5ba1369865f3fc2b7728a75454678bb60882c7f32a501508a6c94128410f` |
| `orz-acaf-provision` | 1,220,536 | −32 B | `ea55bafb0b22cb85a9a16c730e0d4571a9519ac9eb98556b6fa2acc63b91c405` |

- ELF 静态核验（逐件）：三件均 `e_type=3`（ET_DYN static-pie）＋ `e_machine=62`（x86-64）
  ＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY `tui io error`／signer manifest 缺失／provision usage——与 052–071 同形态），
  **6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.11 同数**（三处均为非 Windows 分支的 unused 项，既有基线）。

### 5.1 本批摩擦：Docker Desktop 引擎停摆（`HCS_E_CONNECTION_TIMEOUT`）

- **现象**：首轮 `docker run` 报 `request returned 500 Internal Server Error … /containers/create`；
  随后 `docker info` / `docker ps` / 裸 `docker run alpine echo` 一律
  `Docker Desktop is unable to start`。`docker desktop status` 长期停在 `starting`。
- **根因定位**：`wsl -d docker-desktop -e /bin/echo` ⇒
  `Wsl/Service/CreateInstance/HCS_E_CONNECTION_TIMEOUT`；**同时**`wsl -d Ubuntu-24.04-D` 探针
  **正常返回** ⇒ **WSL 整体可用、只有 docker-desktop 发行版／HCS 连接面挂死**（非整机故障、
  非本仓改动引起）。
- **处置序列（如实记录，全部失败→外部手段）**：① `docker desktop start`（报 already running）；
  ② `docker desktop stop` ＋ `start`（stop 成功、start 停摆）；③ `Start-Process Docker Desktop.exe`
  重启 App（进程起、引擎不起）；④ `docker desktop restart`（**无法停止**：`processes still
  running … context deadline exceeded`）；⑤ 杀 `com.docker.backend`／`com.docker.build`／
  `docker-desktop` 后重启 App（进程消失、引擎仍不起）；⑥ 最终**由用户重启电脑**恢复。
- **恢复读数**：重启后 `docker version` server **29.6.2**、`docker desktop status = running`、
  `rust:1.97-slim`／`alpine:3.20`／`debian:bookworm-slim` 三镜像在位 ⇒ Linux 构建**一次通过**
  （§5 全部读数即该次）。
- **口径与建议**：本摩擦**与 052／054 的代理切换摩擦不同族**（那两条是 apt 走死上游代理；
  本条是 Docker Desktop 引擎／HCS 面挂死，且**实包预检 `APT_OK` 证明网络通路正常**）。
  按「仅记录」处置：不立项，建议并入 0bq（进程收口兜底）的邻域观察项——重建环境属**装置面**，
  其挂死会静默阻断 Linux 载体重建这一唯一通道。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.11 备份件）

脚本 [`.tmp-b074-literals.py`](../../.tmp-b074-literals.py)（可复跑，本批 exit 0）。
**新面（0 → ≥1，双侧同值）**：

| 字面量 | Win 0.6.11 | Win 0.6.12 | Lin 0.6.11 | Lin 0.6.12 | 判读 |
|---|---:|---:|---:|---:|---|
| `[emoji 剥离]` | 0 | **1** | 0 | **1** | 0bi⑪ 写入面 emoji 剥离告知行**进件** |
| `ORZ_WRITE_KEEP_EMOJI` | 0 | **1** | 0 | **1** | 0bi⑪ 逃逸开关 env **进件** |
| `Optional line-window anchor edit` | 0 | **1** | 0 | **1** | 0bi⑤ anchor 行窗工具描述**进件** |
| `appears to be UTF-16 encoded` | 0 | **1** | 0 | **1** | 0bl③ UTF-16 拒绝文案**进件** |
| `当前可压区间` | 0 | **1** | 0 | **1** | 0bk 区间未识别告知块**进件** |
| `context_scale:block_selection_unrecognized` | 0 | **7** | 0 | **7** | 0bk 审计键**进件** |
| `context_scale:mandatory_summary_not_landed` | 0 | **7** | 0 | **7** | 0bn① 未落地审计键**进件** |
| `至少命中 2 个小节` | 0 | **1** | 0 | **1** | 0bn② 摘要识别门槛句**进件** |
| `superseded_by_mandatory_window` | 0 | **7** | 0 | **7** | 073 R13 窗口取代标签**进件** |

**保持面 26 项零回退**（逐项双侧同值，不回退、不减损）：

- 071 批新面 14 项：`Call-scope lifecycle` 3／`[编码降级告知]` 3／`目标档：≤` 1／
  `[黑板说明书 guide v1 · digest sha256:` 1／`r<轮>·b<块>·s<seq>` 27／`…(+` 1／`条暂存` 1／
  `[RUN_END` 1／`[/RUN_END]` 1／`await_channel` 5／`lines truncated; narrow the pattern` 1／
  `Text files only: when true, return a section outline` 1／`No section headings found in this ` 1／
  `more headings omitted` 1。
- 更早批次面 12 项：`RLI提醒:` 1／`持续越线` 1／`域迁移确认` 1／`掩盖缺口` 1／`CoverageGap` 1／
  `lif.domain_migration` 7／`分通道明细 selector=channels` 1／面头符号表 1／`本会话负担总值 ` 3／
  `资源软提示` 2／`utf-8-lossy` 1／`反例` 33。

> **方法边界（本批第二例，与 071 `[RUN_END]` 同族）**：0bn② 的「`（见上）` 指称退役」对象是
> **强制压缩窗口块**——该块已自嵌摘要模板，钉子 `…self_contained` 断言两级询问块**不含**
> `（见上）`（`context_scale.rs:880`）。但**全形字节级扫描只见全局计数**：`（见上）` 1 → 1，
> 唯一命中来自 H1 阶梯窗口块 `compression_window_block()` 的**同消息内指称**——
> `hard_reminder_block()` 已在同一注入消息里内嵌 `summary_block_guide()`，
> `agent_loop.rs:2375-2383` 两串由同一 `format!` 装配（H1 不单独注入）。
> ⇒ **字节级扫描是进件核证的下界、不能分块隔离**；本条**非缺陷**，属方法边界，登记备查。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.11-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260924-074`。
- ACAF 重 provision（root＝`D:\tb-eval\orz-windows\acaf`，**exit 0**）：keystore **逐位未动**
  （外层 `installation-key.dpapi` `2408F596…`／`installation-key.json` `C491FC95…`；
  `keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=bc6acb9913012d9cc927b9089b02cc010e25b65011e19f65ae2a668a52e6e9de`
  ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- 宿主装配复核（**PS 5.1 真机**）：`powershell -File scripts\dogfood_launch.ps1
  -TaskFile .tmp-0bi-task.txt -DryRun` 断言全过（cwd＝`D:\CLI`、载体哈希回显 `3492768882C1…`、
  ACAF fail-closed=True、`PROTOC`／`GROK_HOME` 就位、`rli=on（缺省常开）`、题面
  `.tmp-0bi-task.txt` 回显 **113 字符・utf-8 BOM・读取＝显式 UTF-8**）。
- Linux：`/out` 直写即换装位（换装前 `.0.6.11-bak` 六件齐备）；ELF／冒烟读数见 §5。

## 8. 双平台打包与 GitHub Release

- 暂存 `D:\tb-eval\rel-074-stage\`（本地件不入库），入口脚本
  [`.tmp-b074-package.ps1`](../../.tmp-b074-package.ps1)（沿 066 形态）：

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.12-windows-x86_64.zip` | 27,815,042 | `1a48f33c32a013c9808528cc3ce6d9f357e6c6009e47f3e9fd8c6dacd4815927` |
| `orz-0.6.12-linux-x86_64.tar.gz` | 36,042,724 | `5bd2af6e695e800445f89dc685e0dadc07370b138a547b1d15013e8db0094b8a` |

- **包内容**＝三件二进制＋`README.md`（0.6.12 更新说明：编辑面保真／工具执行与进程／
  交互与治理面／RLI 现状＋两平台快速开始＋完整性校验）＋`SHA256SUMS`（`hash *name`、LF 行尾）。
- 包完整性：解包回读逐件与换装载体 **同源同哈希（6/6 MATCH）**；容器内
  `sha256sum -c SHA256SUMS` **全 OK**（zip 侧 4/4、tar 侧 4/4，`alpine:3.20`）。
- **tar 权限形态（沿用口径）**：包内条目 mode 为 `-rw-rw-rw-`（Windows bsdtar 打包固有），
  与已发布的 0.6.6 tar **同形**；README 的 Linux 快速开始已含 `chmod +x orz orz-signer
  orz-acaf-provision`，非本批引入。
- Release：父仓 tag **`v0.6.12`**（指向本批提交）＋双资产（双包），说明文本＝源冻结基线、
  六件二进制哈希表、0.6.12 更新说明（相对已发布 0.6.6；含内部载体 0.6.7–0.6.11 的面）与
  验证摘要（沿 0.6.0／0.6.2／0.6.4／0.6.6 形态）。
- 上传面核证：`gh api repos/SilverWhite/CLI/releases/tags/v0.6.12` 回读两资产 **`digest` 字段**，
  与本地包哈希逐位一致（见 §8.1 补记）。

### 8.1 上传面核证补记（本档提交后回读）

（本小节由随后的补记提交填写：资产 digest 回读结果、远端 size 与本地 size 对照。）

## 9. 记账面（pin、清单、索引、计数）

- orz：**`d69dab47`**（bump；已推 `cli` 远端）。
- 父仓：本档 ＋ 子模块 pin 指向 `d69dab47` ＋ `orz_source_manifest.sha256` 重算
  （差异恰 **2 行**＝bump 两文件）＋ TODO／BACKLOG 的 **0bi／0bl／0bk／0bn「进载体 0.6.12」标注**
  ＋ 索引 v4.38 → **v4.39**（v4.38 头行滚入 `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`）
  ＋ `README.md` 发布面对齐 **v0.6.12**。
- **计数不变**（**未闭合 54**）：本批为重建与发行，不新增／不闭合开放项；0bm 狗粮长轮仍未开工
  （本批只把其前置备齐）。
- 门禁 `python scripts/check_repository.py`：提交前唯一红＝orz 脏树（预期态）；提交后须复跑见 §10。

## 10. 边界与未做项

1. **未跑真机狗粮轮**：本批只做到「四批进件＋字面量可核＋宿主装配可核＋双平台发行」；
   0bm（狗粮长轮）与 0bk／0bn 的 **S3 真机读数**、0bi S3 未读三项（journal anchor 三态／
   `run_finished` 新字段／投递计数）与 ⑪ 告知行首读，均留待下一轮真机。
2. **0bn 定位（用户裁决 2026-09-24）**：单独条目、不进 0bm；本批只随载体重建进件，
   **S3 真机与后续批次由用户另定**。
3. **0br（Web 形态 UI）未动**：S1 勘定仍待开工。
4. **未做双平台预检**（默认口径；§3）。
5. **发行面只发双平台包**：不发行 `releases/` 目录式试用包（历史形态），不动既有 tag。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b074-windows-build.log`／`.tmp-b074-linux-build.log`／`.tmp-b074-literals.py`／
`.tmp-b074-package.ps1`／`.tmp-b074-package.ps1` 运行读数（资产表）／`.tmp-orz-commit-msg-bump074.txt`；
两平台 `.0.6.11-bak` 备份链与 `signer-manifest.json.bak-20260924-074` 保留；
发布包暂存 `D:\tb-eval\rel-074-stage\`（含 `_verify_zip`／`_verify_tar` 回读目录）。

## 12. 关联与关键词

[`071 重建档`](071_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-23.md) ／
[`073 复审`](073_UNCOMMITTED_REVIEW_AND_REMEDIATION_2026-09-24.md) ／
[`0bi 轮报告`](0BI_FRICTION_CARRYOVER_2026-09-23.md) ／
[`066 重建与发行档`](066_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-21.md) ／
[`索引`](../../CLI_PROJECT_INDEX.md) ／ [`BACKLOG`](../BACKLOG_AND_PRIORITIES.md) ／
[`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md)。
关键词：0.6.12、双平台、载体重建、换装、发行、GitHub Release、四批进件、0bi、0bl、0bk、0bn、
字面量核证、方法边界、ACAF 重 provision、Linux musl static-pie、Docker Desktop 停摆、
HCS_E_CONNECTION_TIMEOUT、tar 权限形态。
