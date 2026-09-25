# 0.6.14 双平台载体重建与换装（2026-09-25）

> 用户令：「请进行提交并推送吧，随后再重建一轮，把新的修改应用进 orz 包体」
> （前接 0bm 真机轮 S2 增量落码批父仓 `4b9e0598`／orz `b7dd241e`）。
> 本批三段：① **版本 bump**（0.6.13 → 0.6.14）② **双平台重建** ③ **换装＋字面量核证＋
> ACAF 与装配复核＋载体级 Web 探针＋本地打包**。
> **源冻结线**：orz **`e22c0dba`**（`feat/fusion-architecture`）＝0bm S2 增量 `b7dd241e`
> ＋ bump 两文件两行。**上一载体**：双平台 0.6.13（已发行，`5998d4b1` 冻结）。
> 本批**未发行**（用户令只到「应用进包体」；Release 面待放行）。本批**免预检**（默认口径沿用）。

## 0. 结论速览

| 项 | 结果 |
|---|---|
| 版本 bump | `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` 两文件两行：0.6.13 → **0.6.14**（orz `e22c0dba`，已推 `cli`） |
| 0bm 进件 | 编辑面簇四件（**大小上限 16 MiB＋`ORZ_EDIT_MAX_FILE_BYTES` 逃生**／**行尾逐行保真**／**编辑前回退窗口**`.gsa/rollback/`＋告知行／**写路径收敛去名字特判**）＋0bj②⑤（压缩自选口径／320K 柔和化）**首次进入在役载体** |
| Windows 三件套 | `cargo clean`（35,708 文件／18.9 GiB）后 clean 全量 **28m30s** exit 0；**警告 0／错误 0**；换装逐件 MATCH=True |
| Linux musl 三件套 | Docker `rust:1.97-slim`；实包预检 `APT_OK`（未切代理）；**暖缓存 11m40s** exit 0；三件 ET_DYN／x86-64／PT_INTERP=0；bookworm·alpine 双向冒烟 **6/6 exit 1** |
| 字面量核证 | **新面 7 项全部 0 → ≥1**（双侧逐位同值）；**保持面 43 项零回退**（0.6.13 新面 8／0.6.12 面 9／0.6.11 面 14／更早面 12） |
| 换装与 ACAF | 六件换装（`.0.6.13-bak` 链）＋ACAF 重 provision exit 0（keystore 四值逐位未动、manifest↔signer 逐位一致）＋载体级 Web 探针（令牌／Host 门／boot 面／归档 API 全过、零残留进程） |
| 打包 | 双平台包 ＋ 顶层 `SHA256SUMS` 暂存 `D:\tb-eval\rel-076-stage\`（解包回读 6/6 MATCH、容器内 `sha256sum -c` 全 OK）；**未发布 Release** |
| 摩擦 | ① 探针误跑 `orz --version` 静默进 TUI 并留下 1 个进程（按路径回收，零残留）——属 **0bj① 已立项面**的又一实证，**不另立项**；② 新发现：**载体版本串不落字节面**（§6 方法边界） |

## 1. 前置基线（0.6.13 在役件，换装前实取）

| 平台 | 文件 | 尺寸 (B) | SHA256 |
|---|---|---:|---|
| Windows | `orz.exe` | 56,427,520 | `83BBCE8D2B491AF79C72EC59A5A6D9BA70354D027C5EF8DA3EEB5FB11986A7F4` |
| Windows | `orz-signer.exe` | 6,742,528 | `05F3E83FB158DF30ED757DFD208D79B42FAFCD4CB8EF7F7EA0B4DC457AB3DBCE` |
| Windows | `orz-acaf-provision.exe` | 6,642,176 | `9B0B31F1661F7CA4A0E88E22AE57ABF73A58E78313E5DC714B2568BE312492BD` |
| Linux | `orz` | 114,668,656 | `B9BC348911FC6D0DF717A1076088AC83CF8DBFBEEE5DDDA1EB113CB76028B1DD` |
| Linux | `orz-signer` | 1,401,784 | `50CCC6087B83A5CA83EDA79D97AC90A692A96382ECDC87A2F402A9D735EFA3DE` |
| Linux | `orz-acaf-provision` | 1,220,528 | `FD4E9BECB1A1E6058F0A6161DD46B4BC6190B296860E87B6A4CD78D3D048F7C5` |

六件与 [`075 档 §1`](075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) 记录**逐位一致**
（在役件未被外力改动）。换装前逐件复制为备份链 **`.0.6.13-bak`**（Windows
`D:\tb-eval\orz-windows\`；Linux `D:\tb-eval\orz-linux\`）——六件全部就位。

## 2. 版本 bump 与源冻结

- `crates/orz-bin/Cargo.toml` ＋ `Cargo.lock` **两文件两行**：`0.6.13 → 0.6.14`
  （orz 提交 **`e22c0dba`**；`cargo metadata --locked` exit 0 ⇒ 锁文件自洽）。
- 推送：`b7dd241e..e22c0dba  feat/fusion-architecture -> feat/fusion-architecture`。
- 冻结线内容＝0bm 真机轮 S2 增量批 `b7dd241e`（11 文件，＋925／−191：编辑面公共层
  `util/write_face.rs` 新增 307 行、锚点／经典／hashline 三路径接入行尾逐行保真、
  `console.rs` 去 `search_replace` 名字特判、`context_scale.rs` 压缩自选口径）＋ bump。
- 父仓首段（提交推送批）＝`4b9e0598`：0bs 检索线五道用户令定稿入账＋README 借用标注
  ＋第二卷 §1.17–§1.22；门禁 `valid: true`。

## 3. 预检：本批按默认口径免除

- 用户 2026-09-22 口径：**默认不预检**，仅当出现只能在另一平台验证的条件编译面、或
  用户明确点名时才做。本批为编辑面增量进载体＋打包，**不做**双平台预检。

## 4. Windows 三件套（宿主 release，clean 全量）

- 构建入口＝`scripts/build_orz.ps1 -Release -Clean`（auto 档自检读数 **12 核／物理
  15.8 GiB（余 6.3）／commit 25.8 GiB（余 5.6，使用 78.3%）** ⇒ 判 **`-j 2`**，
  并触发 0bh ⑥ 的**余量软提示**）。
- `cargo clean`：**35,708 文件／18.9 GiB**；构建 **exit 0**，`Finished release` **28m30s**；
  **警告 0 条、错误 0 条**（构建日志全量 28 KB、逐行复核）。

| 文件 | 尺寸 (B) | 与 0.6.13 差异 | SHA256（换装位＝构建产物，MATCH=True） |
|---|---:|---:|---|
| `orz.exe` | 56,440,832 | **+13,312 B** | `B79F20DB60618B5F0CD02F919F36FEF272390F5728686D9DA0FD9CB7BBAD1CCB` |
| `orz-signer.exe` | 6,742,528 | ±0 | `179A2E4048F57E19489D2DD662F8648036A24B03EF838C5E940014A9705871DD` |
| `orz-acaf-provision.exe` | 6,642,176 | ±0 | `332A6EF8C925C19ABF7EA8F5860BFC28612383D03C81935797B7155AC9070CCE` |

> `orz.exe` +13 KB 与编辑面增量的代码量级相称（新增 `write_face.rs` 307 行＋三路径接入）；
> `signer`／`provision` **无源码改动**，尺寸不变、哈希变动属重编译非确定性——与
> 068–075「同码不同哈希」同族，非回退。

## 5. Linux musl 三件套（Docker，直写换装位）

- **实包预检**：`apt-get update -qq && apt-get install -y -qq musl-tools` → **APT_OK**
  ⇒ 按 [`Docker 代理配方`](../../scripts/DOCKER_PROXY_RECIPE.md) §0 **不切代理、不重启**。
- 构建：`rust:1.97-slim` ＋ `bash /orz/scripts/build_orz_aliyun_trixie.sh`（ORZ-BUILD-MOUNT-001
  契约），挂载 `-v D:/CLI:/orz`（父仓原地）＋`-v D:/tb-eval/cargo-config.toml:/root/.cargo/config.toml`
  ＋`-v D:/tb-eval/orz-target:/target`（暖缓存）＋`-v D:/tb-eval/orz-linux:/out`（**直写换装位**，
  换装前已留 `.0.6.13-bak`）；**exit 0**：容器内 `Finished release` **11m40s**
  （日志 `.tmp-b076-linux-build.log`）。
- **容器未在源树留痕**：构建后 orz 仓 `git status` 仅 bump 两文件（无新增未跟踪件）。

| 文件 | 尺寸 (B) | 与 0.6.13 差异 | SHA256 |
|---|---:|---:|---|
| `orz` | 114,693,448 | **+24,792 B** | `e1e0eea75e2f4bb76d6315e4352abfc16573d94c4f1ce99ac9626b7ffea5c9ad` |
| `orz-signer` | 1,401,800 | +16 B | `8da16e7148d2774d97a4cd9a6782c71b029e63c3ec8ceaddd89bfd812462c63a` |
| `orz-acaf-provision` | 1,220,528 | ±0 | `555124f5c80434e455186556b18ad70f02806f6249f9648373defa7809cb650c` |

- ELF 静态核验（逐件，alpine:3.20 `file`＋`readelf`）：三件均 `ET_DYN`（static-pie）＋
  `x86-64` ＋ **PT_INTERP=0**。
- 双向加载冒烟：`debian:bookworm-slim` 与 `alpine:3.20` 两侧、三件均 **exit 1**
  （orz 无 TTY／signer manifest 缺失／provision usage——与 052–075 同形态），**6/6**。
- **警告面（平台条件面基线）**：`orz-config` 1／`orz-assurance` 4／`orz-host` 2
  **与 0.6.13 同数**（三处均为非 Windows 分支的 unused 项，既有基线）；**`orz-web` 0 警告**。

## 6. 字面量核证（双方二进制，字节级出现次数；对照列＝在役 0.6.13 备份件）

脚本 [`.tmp-b076-literals.py`](../../.tmp-b076-literals.py)（可复跑，本批 exit 0）。
**新面（0 → ≥1，双侧同值）**：

| 字面量 | Win 0.6.13 | Win 0.6.14 | Lin 0.6.13 | Lin 0.6.14 | 判读 |
|---|---:|---:|---:|---:|---|
| `ORZ_EDIT_MAX_FILE_BYTES` | 0 | **1** | 0 | **1** | 0bm ④ 大小上限逃生开关**进件** |
| `exceeding the edit-face size limit of` | 0 | **1** | 0 | **1** | 0bm ④ 大小门拒绝文案**进件** |
| `[回退窗口]` | 0 | **2** | 0 | **2** | 0bm ⑦ 回退告知行前缀**进件** |
| `编辑前内容已存` | 0 | **1** | 0 | **1** | 0bm ⑦ 回退已存告知（带指针）**进件** |
| `编辑前内容快照存储失败` | 0 | **1** | 0 | **1** | 0bm ⑦ 快照失败不静默**进件** |
| `.gsa/rollback/` | 0 | **1** | 0 | **1** | 0bm ⑦ 回退快照目录**进件** |
| `是否现在压缩、压缩哪些块由你判断` | 0 | **1** | 0 | **1** | 0bj② 压缩自选口径（320K 柔和化同批）**进件** |

**保持面 43 项零回退**（逐项双侧同值，不回退、不减损）：

- **0.6.13 新面 8 项**（0br Web 工作台）：`/api/archives` 6／`trusted_workspaces` 2（Win）・3（Lin）／
  `archive://` 6／`reconstructed_from_journal` 3／`用法: orz archive <session8>` 1／`归档于 ` 1／
  `转写超上限已截断（` 1／`已信任工作区` 5。
- **0.6.12 新面 9 项**：`[emoji 剥离]` 1／`ORZ_WRITE_KEEP_EMOJI` 1／`Optional line-window anchor edit`
  1／`appears to be UTF-16 encoded` 1／`当前可压区间` 1／`context_scale:block_selection_unrecognized`
  7／`context_scale:mandatory_summary_not_landed` 7／`至少命中 2 个小节` 1／
  `superseded_by_mandatory_window` 7。
- **0.6.11 新面 14 项**：`Call-scope lifecycle` 3／`[编码降级告知]` 3／`目标档：≤` 1／
  `[黑板说明书 guide v1 · digest sha256:` 1／`r<轮>·b<块>·s<seq>` 27／`…(+` 1／`条暂存` 1／
  `[RUN_END` 1／`[/RUN_END]` 1／`await_channel` 5／`lines truncated; narrow the pattern` 1／
  `Text files only: when true, return a section outline` 1／`No section headings found in this ` 1／
  `more headings omitted` 1。
- **更早批次面 12 项**：`RLI提醒:` 1／`持续越线` 1／`域迁移确认` 1／`掩盖缺口` 1／`CoverageGap` 2／
  `lif.domain_migration` 7／`分通道明细 selector=channels` 1／`符号: u=水平 v=速率 pred=闭式前推` 1／
  `本会话负担总值 ` 3／`资源软提示` 2／`utf-8-lossy` 1／`反例` 34。

> **方法边界（本批新发现，第三例同族）**：**载体版本串不落字节面**——`0.6.14` 与 `0.6.13`
> 在四件二进制中的字节计数**全为 0**（含对照件自身）。根因＝显示版本取自
> `orz-version` crate 自身的 `CARGO_PKG_VERSION`（`crates/codegen/orz-version/src/lib.rs:9`），
> 而 `orz-bin` 的 `0.6.x` 是**发行命名口径**（版本 bump 的实际作用＝包名／Release 说明／
> 锁文件自洽），此前批次亦未把版本串列入核证项。本条**非缺陷、非回退**，登记备查。
> 同族前例：074／075 的「`（见上）` 指称计数恒为 1」——字节级扫描是进件核证的**下界**、
> 不能分块隔离，也不能覆盖「未编译进二进制」的元数据。

## 7. 换装、ACAF 与装配复核

- Windows（`D:\tb-eval\orz-windows`）：三件逐件 SHA256 与构建产物 **MATCH=True**；旧件留
  **`.0.6.13-bak`**；旧 manifest 留 `signer-manifest.json.bak-20260925-076`。
- ACAF 重 provision（`orz-acaf-provision.exe acaf\keystore acaf\signer-manifest.json`，**exit 0**）：
  keystore **逐位未动**（外层 `installation-key.dpapi` `2408F596…`／`installation-key.json`
  `C491FC95…`；`keystore/` 内两件 `F37556AB…`／`2AA80CB8…`——**前后四值全一致**）；
  回显 `binary_sha256=179a2e4048f57e19489d2dd662f8648036a24b03ef838c5e940014a9705871dd`
  ↔ 换装位 `orz-signer.exe` **逐位一致**（manifest 169 B 同步更新）。
- 冒烟（换装位）：provision usage **exit 1**；signer fatal（manifest 缺失）**exit 1**。
- **载体级 Web 探针（脚本 [`.tmp-b076-win-web-probe.ps1`](../../.tmp-b076-win-web-probe.ps1)）**：
  `orz.exe web --addr 127.0.0.1:21556` 拉起换装位二进制 → 启动打印 64 位令牌 URL；
  `/`（静态）**200**；`/api/boot` 无令牌 **401**；伪造 Host **401**；带令牌 **200**，
  boot 键＝`cwd,agent,trusted_workspaces`；`/api/archives` 带令牌 **200** → **20 件**
  （本机真实归档数，较 075 首读 19 件 +1）；探针后 **`leftover_orz_procs=0`**。
- Linux：`/out` 直写即换装位（换装前 `.0.6.13-bak` 六件齐备）；ELF／冒烟读数见 §5。

### 7.1 本批摩擦（仅记录，不另立项）

- **探针误跑 `orz --version`**：`orz.exe --version` 不进参数解析分支、**静默进入 TUI**，
  在 `Start-Process` 之外的直接调用下留下 1 个进程（PID 13820，路径精确识别 → `Stop-Process`
  → 复核 `leftover=0`，源树与换装位无损伤）。**该面已由 0bj① 立项**
  （「`orz.exe --version` 未打印版本且直入 TUI 挂起〔版本核验改旁路＋『勿裸跑 `orz.exe`』
  入口纪律〕」），本次为**同族又一实证**，随 0bj① 处理，**不新增计数**。
- 版本核验的旁路口径已在本批落地：**版本以打包面（`orz-0.6.14-*` 资产名／包 README／
  manifest）为准**，二进制面改用§6 功能字面量核证（版本串不落字节面，见§6 方法边界）。

## 8. 打包与发行状态

- 暂存 `D:\tb-eval\rel-076-stage\`（本地件不入库），入口脚本
  [`.tmp-b076-package.ps1`](../../.tmp-b076-package.ps1)（沿 066／074／075 形态）；
  包 README 由 [`.tmp-b076-readme-package.py`](../../.tmp-b076-readme-package.py)
  从 0.6.13 包 README 增量改写（新增「0.6.14 更新」段，12,372 B）。

| 资产 | 大小 (B) | SHA256 |
|---|---:|---|
| `orz-0.6.14-windows-x86_64.zip` | 28,393,183 | `5dc2802520d88b1ffe2beff31b7f045a148341c25cb11cde43fb925b0e74148d` |
| `orz-0.6.14-linux-x86_64.tar.gz` | 36,830,675 | `bbf0a2d73fb6996ab15a25ff24ff4ba17acb112a19a1b2c9aa411c86c9e46b55` |
| `SHA256SUMS`（顶层） | 193 | `ab73002fcc07e28d51c06d59b1619bf731cf329f13c080c2921d067dd1ab6db2` |

- 包内容＝三件二进制＋`README.md`（0.6.14 更新说明：编辑面簇四件／0.6.13 及更早的面＋
  两平台快速开始＋完整性校验）＋`SHA256SUMS`（`hash *name`、LF 行尾）。
- 包完整性：解包回读逐件与换装载体 **同源同哈希（6/6 MATCH）**；容器内
  `sha256sum -c SHA256SUMS` **全 OK**（zip 侧 4/4、tar 侧 4/4、顶层 2/2，`alpine:3.20`）。
- **未发布 GitHub Release**：用户令止于「把新的修改应用进 orz 包体」——本批只做
  **重建＋换装＋本地打包**；`v0.6.14` tag／Release 与 README 发布面对齐（现仍指已发布
  `v0.6.13`）**待放行**后执行（资产与哈希已冻结，可直接上传）。

## 9. 记账面（pin、清单、索引、计数）

- orz：**`e22c0dba`**（bump；已推 `cli` 远端）。
- 父仓：本档 ＋ `orz_source_manifest.sha256` **重新生成 1,498 条**（差异恰为两行：
  `Cargo.lock` 与 `crates/orz-bin/Cargo.toml`）＋ 子模块 pin 指向 `e22c0dba`
  ＋ TODO／BACKLOG 的 **0bm「进载体 0.6.14」标注** ＋ 第二卷 §1.23 本批流水
  ＋ 索引头行 → **v4.43**（v4.42 头行滚入 `存档/index/CLI_PROJECT_INDEX_FULL_2026-09-24.md`）。
- **计数不变**（**未闭合 55**）：本批为重建与换装，不新增／不闭合开放项。
- 门禁 `python scripts/check_repository.py`：提交后复跑须 `valid: true`（见 §10）。

## 10. 边界与未做项

1. **未发 Release**：见 §8（资产已冻结于本地暂存；发布需一句放行）。
2. **未跑真机狗粮轮**：本批只做到「0bm 增量进件＋字面量可核＋载体级 Web 探针＋换装」；
   0bm 未竟三项（①ADR 分卷／②`run_agent_loop` 拆分／③spawn sink per-dispatch）与
   0bp／0bq／0bs 各行其账。
3. **0bm 增量只做静态与字面量进件核证**：编辑面行为（大小门／行尾保真／回退窗口）
   在 0bm 轮与落码批已由单元／集成测试覆盖（`orz-tools --lib` 2,939 项、`search_replace`
   119 项、`orz-loop` 845 项）；**载体面首次行为复验留给下一轮真机**（S3 真机读数）。
4. **未做双平台预检**（默认口径；§3）。

## 11. 证据留档（`.tmp-*` 门禁豁免）

`.tmp-b076-windows-build.log`／`.tmp-b076-linux-build.log`／`.tmp-b076-literals.py`／
`.tmp-b076-package.ps1`／`.tmp-b076-package.log`／`.tmp-b076-readme-package.py`／
`.tmp-b076-win-web-probe.ps1`／`.tmp-b076-win-web-probe.log`／`.tmp-b076-win-web-probe.out.log`；
两平台 `.0.6.13-bak` 备份链与 `signer-manifest.json.bak-20260925-076` 保留；
发布包暂存 `D:\tb-eval\rel-076-stage\`（含 `_verify_zip`／`_verify_tar` 回读目录）。

## 12. 关联与关键词

[`075 重建档`](075_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-25.md) ／
[`0bm 轮报告`](0BM_EDIT_FACE_IMPLEMENTATION_AND_FRICTION_2026-09-25.md) ／
[`074 重建档`](074_CARRIER_REBUILD_DUAL_PLATFORM_2026-09-24.md) ／
[`0BS 检索线路线调研`](0BS_RETRIEVAL_ROUTE_SURVEY_2026-09-25.md) ／
[`综合稿`](../UI_FORM_CONSOLIDATED_DESIGN_2026-09-24.md) ／
[`ADR-0010 §14.78`](../../adr/ADR-0010-fusion-runtime-and-agent-architecture.md)

关键词：0.6.14、编辑面簇进件、大小上限 16 MiB、`ORZ_EDIT_MAX_FILE_BYTES`、行尾逐行保真、
回退窗口、`.gsa/rollback`、写路径收敛、字面量核证、载体级 Web 探针、ACAF 重 provision、
PT_INTERP=0、版本串不落字节面、0bj① 同族实证、本地打包未发行。
